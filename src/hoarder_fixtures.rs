//! HRD-A2: the Hoarder's direct-fight fixtures (`docs/01-design/enemy-gameplay-hoarder.md`
//! section 14, "HRD-A2 (direct fight, `hoarder_fixtures.rs`)").
//!
//! Bevy-free. Each fixture builds the **real room** ([`crate::f01_hoarder`]) and
//! the real combat state (her slot enemy, the story ledger, CF02 rules), drives
//! it with a scripted input or a pilot, **asserts the sheet's expectation** and
//! returns the final state hash. A fixture that fails returns the reason, never
//! a silent pass. They are library fixtures (like `f01_late_fixtures`): the tests
//! below pin each final hash and replay each fixture twice; they are not in the
//! `--party-fixture` registries (no committed replay file, no CI loop).
//!
//! Conventions of the sheet: her pivot is at [`E`] (760), awake in Neutral facing
//! -x, the entry gate closed; Carl at distance `d` stands at x `E - d`. The tick a
//! `CombatEvent::PileBoss { Tell }` is logged is **attack tick 0** (T0); a hero
//! input "on attack tick k" is the input of scene tick T0 + k (the hero's step
//! precedes her step within a tick, so the input of T0 itself is the one given
//! before she chose). A fixture finds T0 with a probe run of the same start (she
//! is deterministic), then replays with the script.
//!
//! The entry gate and the exit door are runtime env solids of the one sync
//! (`combat_scene/env.rs`): the door a sidecar lock that opens on her defeat tick,
//! the gate a slot gate that closes on `Engaged` (`hrd_gate_physical`,
//! `hrd_exit_door_physical`).
//!
//! **Not here (deferred to HRD-A2b, listed in the PR):** `hrd_gate_no_band`,
//! `hrd_disengage_follow_punished`, `hrd_leash_kite_probe` (reporting only),
//! `hrd_direct_clear_companion` (waits for CA2-B2) and `hrd_room_validator` (the
//! validator side is BOIL-A's and HRD-A1's tests).

use std::path::Path;

use crate::combat::CombatEvent;
use crate::combat_scene::pile_boss::{PileBossEnemy, of};
use crate::combat_scene::{CombatState, Fixture};
use crate::corridor::SceneSim;
use crate::f01_hoarder::{self, HoarderRun, slot_room};
use crate::f01_late::Loadout;
use crate::input::RawInput;
use crate::party::{PROVISIONAL_TUNING, PadId, PartyState};
use crate::pile_boss::{
    PILE_BOSS_ID, PILE_BOSS_SOURCE_ID, PileAttack, PileBossTuning, PileEvent, PileMode,
};
use crate::scene_enemy::SlotSet;

/// Her pivot in the sheet's fixtures.
pub const E: f32 = 760.0;

/// Who is in the room and who drives whom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// Carl only: the party is not live, Donut is not a hero (the sheet's "Carl only").
    CarlOnly,
    /// Solo, P1 controls Carl, Donut is the plain bot.
    SoloCarl,
    /// Solo, P1 controls Donut (swapped before tick 1), Carl is the plain bot.
    SoloDonut,
    /// Two players: Carl on P1, Donut on P2.
    TwoPlayer,
}

/// The start of a fixture.
#[derive(Debug, Clone, Copy)]
pub struct Plan {
    pub control: Control,
    pub carl_x: i32,
    pub donut_x: i32,
    /// `Some((x, facing))`: awake in Neutral with the gate closed
    /// ([`PileBossEnemy::awake_at`]); `None`: the sidecar roster's dormant Hoarder.
    pub awake: Option<(f32, i8)>,
    pub shove_cooldown: Option<u32>,
    pub hp: Option<u16>,
    /// The entry gate lever (HRD-Q41) off: "disabled by fixture setup".
    pub gate_off: bool,
    /// Bank A removed ("the settled case").
    pub no_bank: bool,
    /// The loadout's story flags (`story` by default).
    pub loadout: Loadout,
}

impl Plan {
    /// Carl alone at `E - d`, her awake at [`E`] facing -x, the story loadout.
    pub fn carl_at(d: f32) -> Self {
        Self {
            control: Control::CarlOnly,
            carl_x: (E - d).round() as i32,
            donut_x: 80,
            awake: Some((E, -1)),
            shove_cooldown: None,
            hp: None,
            gate_off: false,
            no_bank: false,
            loadout: Loadout::Story,
        }
    }

    pub fn with_control(mut self, control: Control) -> Self {
        self.control = control;
        self
    }

    pub fn donut_at(mut self, x: i32) -> Self {
        self.donut_x = x;
        self
    }

    pub fn dormant(mut self) -> Self {
        self.awake = None;
        self
    }
}

/// The tuning with the entry gate lever off, built once ([`PileBoss`] borrows its
/// tuning for `'static`).
fn gate_off_tuning() -> &'static PileBossTuning {
    static OFF: std::sync::OnceLock<PileBossTuning> = std::sync::OnceLock::new();
    OFF.get_or_init(|| {
        let mut tuning = PileBossTuning::active().clone();
        tuning.hrdq41_gate_enabled = false;
        tuning
    })
}

/// A running fixture: scene, combat state and (for the party modes) the party.
pub struct Rig {
    pub run: HoarderRun,
    pub scene: SceneSim,
    pub combat: CombatState,
    party: Option<PartyState>,
}

impl Rig {
    pub fn new(root: &Path, plan: Plan) -> Result<Self, String> {
        let run = f01_hoarder::launch(root)?;
        let (carl, donut) = plan.loadout.flags();
        let fixture: &'static Fixture = Box::leak(Box::new(Fixture {
            carl_x: plan.carl_x,
            ..*run.fixture
        }));
        let mut scene = SceneSim::new_at(&run.kit, 360, plan.carl_x)?;
        let party = match plan.control {
            Control::CarlOnly => None,
            _ => {
                let mut party = PartyState::new(PROVISIONAL_TUNING);
                if plan.control == Control::TwoPlayer {
                    party.pad_connected(PadId(1));
                    party.pad_start_pressed(PadId(2));
                }
                if plan.control == Control::SoloDonut {
                    party.request_swap(true, false);
                }
                scene.set_drive(party.drive(true));
                scene.world.donut_body.x = plan.donut_x as f32;
                scene.world.sync_donut_for_tests();
                Some(party)
            }
        };
        let slot = match plan.awake {
            None => {
                // The roster spawn, exactly as the scene builds it.
                crate::scene_enemy::spawn_from_roster(
                    &run.loaded.encounter,
                    slot_room(&run, &scene),
                    false,
                )
                .map_err(|e| e.to_string())?
                .slot
            }
            Some((x, facing)) => {
                let tuning = if plan.gate_off {
                    gate_off_tuning()
                } else {
                    PileBossTuning::active()
                };
                let mut boss = PileBossEnemy::from_encounter(
                    &run.loaded.encounter,
                    slot_room(&run, &scene),
                    tuning,
                )
                .awake_at(x, facing);
                if plan.no_bank {
                    boss = boss.without_bank_a();
                }
                if let Some(ticks) = plan.shove_cooldown {
                    boss = boss.with_shove_cooldown(ticks);
                }
                if let Some(hp) = plan.hp {
                    boss = boss.with_hp(hp);
                }
                SlotSet::new(vec![Box::new(boss)], false).map_err(|e| e.to_string())?
            }
        };
        let combat = f01_hoarder::combat_with(fixture, &run, &mut scene, carl, donut, slot)?;
        Ok(Self {
            run,
            scene,
            combat,
            party,
        })
    }

    /// One tick with one raw input per slot.
    pub fn step(&mut self, slots: [RawInput; 2]) {
        match &mut self.party {
            None => self.combat.step(&mut self.scene, slots[0]),
            Some(party) => {
                let hurt = self.combat.hurt_ticks_of(party.controlled_hero()) > 0;
                party.request_swap(slots[0].swap, hurt);
                let inputs = party.hero_inputs(slots);
                self.scene.set_drive(party.drive(true));
                let restarts = self.combat.restarts;
                self.combat.step_party(&mut self.scene, inputs);
                if self.combat.restarts != restarts {
                    party.restart();
                }
            }
        }
    }

    /// The scene tick of the next step.
    pub fn next_tick(&self) -> u64 {
        self.scene.world.tick + 1
    }

    /// Run `ticks` steps with `script(tick)` giving the slots of each.
    pub fn run(&mut self, ticks: u64, script: &dyn Fn(u64) -> [RawInput; 2]) {
        for _ in 0..ticks {
            let tick = self.next_tick();
            self.step(script(tick));
        }
    }

    pub fn boss(&self) -> &PileBossEnemy {
        of(&self.combat).expect("her slot enemy")
    }

    pub fn mode(&self) -> PileMode {
        self.boss().boss().mode()
    }

    /// Her events, in log order.
    pub fn piles(&self) -> Vec<(u64, PileEvent)> {
        self.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::PileBoss { tick, event } => Some((tick, event)),
                _ => None,
            })
            .collect()
    }

    /// `HeroHurt` events: `(tick, source, health_left, hero index)`.
    pub fn hurts(&self) -> Vec<(u64, u32, u8, usize)> {
        self.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::HeroHurt {
                    tick,
                    source,
                    health_left,
                    hero,
                } => Some((tick, source, health_left, hero.index())),
                _ => None,
            })
            .collect()
    }

    pub fn hash(&self) -> u64 {
        let mut hash = self.combat.state_hash(&self.scene);
        if let Some(party) = &self.party {
            let (cooldown, buffer) = party.swap_timers();
            for value in [
                0x5041_5254_5953_5441,
                u64::from(cooldown),
                u64::from(buffer),
            ] {
                for byte in value.to_le_bytes() {
                    hash ^= u64::from(byte);
                    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
        }
        hash
    }

    /// The scene and combat state (for a caller that carries them on).
    pub fn into_parts(self) -> (SceneSim, CombatState) {
        (self.scene, self.combat)
    }

    pub fn carl_x(&self) -> f32 {
        self.scene.world.body(crate::party::HeroId::Carl).x
    }

    pub fn defeats(&self) -> Vec<(u64, u32, Option<u32>)> {
        self.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::EnemyDefeated {
                    tick,
                    target,
                    instigator,
                    ..
                } => Some((tick, target, instigator)),
                _ => None,
            })
            .collect()
    }

    /// Every `CapabilityGranted`: `(tick, hero, capability)`.
    pub fn grants(&self) -> Vec<(u64, u8, &'static str)> {
        self.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::CapabilityGranted {
                    tick,
                    hero,
                    capability,
                } => Some((tick, hero, capability)),
                _ => None,
            })
            .collect()
    }
}

/// Idle input for both slots.
pub const IDLE: [RawInput; 2] = [RawInput::NONE, RawInput::NONE];

/// Carl's input on slot 0.
pub fn carl_input(raw: RawInput) -> [RawInput; 2] {
    [raw, RawInput::NONE]
}

pub fn jump_input() -> RawInput {
    RawInput {
        jump: true,
        ..RawInput::NONE
    }
}

pub fn attack_input() -> RawInput {
    RawInput {
        attack: true,
        ..RawInput::NONE
    }
}

pub fn run_input(move_x: f32) -> RawInput {
    RawInput {
        move_x,
        ..RawInput::NONE
    }
}

// -------------------------------------------------------------- fixtures --

/// What a fixture returns: the final state hash and a short report line (the
/// measured numbers the sheet asks to report).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub hash: u64,
    pub note: String,
}

pub type FixtureFn = fn(&Path) -> Result<Outcome, String>;

/// Fail the fixture with a message.
macro_rules! ensure {
    ($cond:expr, $($arg:tt)*) => {
        if $cond {
        } else {
            return Err(format!($($arg)*));
        }
    };
}

fn done(rig: &Rig, note: impl Into<String>) -> Result<Outcome, String> {
    Ok(Outcome {
        hash: rig.hash(),
        note: note.into(),
    })
}

/// The tick of her first `Tell` of `attack`.
fn first_tell(rig: &Rig, attack: PileAttack) -> Option<u64> {
    rig.piles().iter().find_map(|&(tick, event)| match event {
        PileEvent::Tell { attack: a, .. } if a == attack => Some(tick),
        _ => None,
    })
}

/// T0 of `attack` for a start: a probe run of the same plan, idle, until her
/// first tell of that attack (she is deterministic, so the scripted run tells on
/// the same tick).
fn probe_t0(root: &Path, plan: Plan, attack: PileAttack) -> Result<u64, String> {
    let mut rig = Rig::new(root, plan)?;
    for _ in 0..400 {
        rig.step(IDLE);
        if let Some(t0) = first_tell(&rig, attack) {
            return Ok(t0);
        }
    }
    Err(format!("no {attack:?} tell inside 400 ticks"))
}

fn carl_health(rig: &Rig) -> u8 {
    rig.scene.world.hero.health
}

/// A run of `plan` with Carl's jump pressed on attack tick `k` of her first
/// `attack`, to `T0 + horizon`. Returns whether any blow landed on a hero by then.
fn jump_run(
    root: &Path,
    plan: Plan,
    attack: PileAttack,
    k: u64,
    horizon: u64,
) -> Result<(bool, Rig), String> {
    let t0 = probe_t0(root, plan, attack)?;
    let mut rig = Rig::new(root, plan)?;
    rig.run(t0 + horizon, &|tick| {
        if tick == t0 + k {
            carl_input(jump_input())
        } else {
            IDLE
        }
    });
    let hit = !rig.hurts().is_empty();
    Ok((hit, rig))
}

/// The four-run edge check of a jump table row: hit / clear / clear / hit.
fn edge_runs(
    root: &Path,
    plan: Plan,
    attack: PileAttack,
    ks: [u64; 4],
    horizon: u64,
) -> Result<Outcome, String> {
    let want = [true, false, false, true];
    let mut last = None;
    for (k, want_hit) in ks.into_iter().zip(want) {
        let (hit, rig) = jump_run(root, plan, attack, k, horizon)?;
        ensure!(
            hit == want_hit,
            "jump on attack tick {k}: a blow landed = {hit}, the sheet says {want_hit}"
        );
        last = Some(rig);
    }
    let rig = last.expect("four runs");
    done(&rig, format!("edges {ks:?}"))
}

/// `hrd_swipe_hits_idle_carl`: Carl idle at d 60. Tell 0..33, smear 34..36,
/// impact 37..42, Recover 43..90, then Cooldown; `HeroHurt` 2 on attack tick 37.
pub fn hrd_swipe_hits_idle_carl(root: &Path) -> Result<Outcome, String> {
    let plan = Plan::carl_at(60.0);
    let mut rig = Rig::new(root, plan)?;
    let start_x = rig.carl_x();
    let full = carl_health(&rig);
    let t0 = probe_t0(root, plan, PileAttack::Swipe)?;
    let mut modes = Vec::new();
    for _ in 0..100 {
        rig.step(IDLE);
        modes.push((rig.next_tick() - 1, rig.mode()));
    }
    let at = |a: u64| modes.iter().find(|(t, _)| *t == t0 + a).map(|(_, m)| *m);
    for (a, mode) in [
        (0, PileMode::SwipeTell),
        (33, PileMode::SwipeTell),
        (34, PileMode::SwipeSmear),
        (36, PileMode::SwipeSmear),
        (37, PileMode::SwipeImpact),
        (42, PileMode::SwipeImpact),
        (43, PileMode::SwipeRecover),
        (90, PileMode::SwipeRecover),
        (91, PileMode::Cooldown),
    ] {
        ensure!(
            at(a) == Some(mode),
            "attack tick {a}: {:?}, want {mode:?}",
            at(a)
        );
    }
    let hurts = rig.hurts();
    ensure!(hurts.len() == 1, "one blow, got {hurts:?}");
    let (tick, source, left, hero) = hurts[0];
    ensure!(tick == t0 + 37, "HeroHurt on attack tick {}", tick - t0);
    ensure!(source == PILE_BOSS_SOURCE_ID && hero == 0, "{hurts:?}");
    ensure!(left == full - 2, "2 damage: {full} -> {left}");
    ensure!(
        rig.carl_x() < start_x,
        "knocked away (-x): {start_x} -> {}",
        rig.carl_x()
    );
    done(&rig, format!("T0 {t0}, hurt on {}", tick - t0))
}

/// `hrd_swipe_jump_edges`: Carl at d 60, jump on attack tick 0 / 1 / 30 / 31.
pub fn hrd_swipe_jump_edges(root: &Path) -> Result<Outcome, String> {
    edge_runs(
        root,
        Plan::carl_at(60.0),
        PileAttack::Swipe,
        [0, 1, 30, 31],
        100,
    )
}

/// `hrd_armour_tell`: Carl at d 44 jabs on attack tick 5: `Hurt` 1 lands (HP 29)
/// and the swipe continues: no flinch, the strike still comes (the hit-stop only freezes her a few ticks).
pub fn hrd_armour_tell(root: &Path) -> Result<Outcome, String> {
    let plan = Plan::carl_at(44.0);
    let t0 = probe_t0(root, plan, PileAttack::Swipe)?;
    let mut rig = Rig::new(root, plan)?;
    rig.run(t0 + 60, &|tick| {
        if tick == t0 + 5 {
            carl_input(attack_input())
        } else {
            IDLE
        }
    });
    let piles = rig.piles();
    let hurt = piles
        .iter()
        .find(|(_, e)| {
            matches!(
                e,
                PileEvent::Hurt {
                    damage: 1,
                    hp_left: 29,
                    ..
                }
            )
        })
        .ok_or_else(|| format!("no Hurt 1 (hp 29): {piles:?}"))?;
    ensure!(
        hurt.0 <= t0 + 5 + 12,
        "the jab lands on tick {}",
        hurt.0 - t0
    );
    ensure!(
        !piles.iter().any(|(_, e)| matches!(e, PileEvent::Flinch)),
        "armour: no flinch"
    );
    ensure!(
        piles
            .iter()
            .any(|(t, e)| (t0 + 37..=t0 + 37 + 8).contains(t)
                && matches!(
                    e,
                    PileEvent::Strike {
                        attack: PileAttack::Swipe
                    }
                )),
        "the swipe continues to its strike on attack tick 37 (plus the jab's hit-stop)"
    );
    done(&rig, format!("jab lands on attack tick {}", hurt.0 - t0))
}

/// `hrd_swipe_punish`: Carl at d 44 jumps on attack tick 20 and jabs on the tick
/// after he lands: the jab lands inside her Recover, as free damage with no flinch.
pub fn hrd_swipe_punish(root: &Path) -> Result<Outcome, String> {
    let plan = Plan::carl_at(44.0);
    let t0 = probe_t0(root, plan, PileAttack::Swipe)?;
    let mut rig = Rig::new(root, plan)?;
    let mut airborne = false;
    let mut landed_at = None;
    let mut jab_tick = None;
    for _ in 0..(t0 + 100) {
        let tick = rig.next_tick();
        let input = if tick == t0 + 20 {
            carl_input(jump_input())
        } else if let Some(l) = landed_at
            && tick == l + 1
        {
            jab_tick = Some(tick);
            carl_input(attack_input())
        } else {
            IDLE
        };
        rig.step(input);
        let grounded = rig.scene.world.hero.grounded;
        if !grounded {
            airborne = true;
        } else if airborne && landed_at.is_none() {
            landed_at = Some(rig.next_tick() - 1);
        }
    }
    let jab = jab_tick.ok_or("Carl never landed")?;
    ensure!(
        (t0 + 43..=t0 + 90).contains(&jab),
        "the jab is inside Recover: attack tick {}",
        jab - t0
    );
    let piles = rig.piles();
    ensure!(
        piles
            .iter()
            .any(|(t, e)| *t >= jab && matches!(e, PileEvent::Hurt { .. })),
        "the jab lands: {piles:?}"
    );
    ensure!(
        !piles.iter().any(|(_, e)| matches!(e, PileEvent::Flinch)),
        "free damage: no flinch"
    );
    ensure!(
        rig.hurts().is_empty(),
        "the jump cleared the swipe: {:?}",
        rig.hurts()
    );
    done(&rig, format!("jab on attack tick {}", jab - t0))
}

/// `hrd_shove_hits_idle_carl`: Carl idle at d 128. One blow of 1 on attack tick 62
/// (lunge tick 23).
pub fn hrd_shove_hits_idle_carl(root: &Path) -> Result<Outcome, String> {
    let plan = Plan::carl_at(128.0);
    let t0 = probe_t0(root, plan, PileAttack::Shove)?;
    let mut rig = Rig::new(root, plan)?;
    let full = carl_health(&rig);
    let start_x = rig.carl_x();
    rig.run(t0 + 130, &|_| IDLE);
    let hurts = rig.hurts();
    ensure!(hurts.len() == 1, "one blow, got {hurts:?}");
    let (tick, _, left, _) = hurts[0];
    ensure!(tick == t0 + 62, "blow on attack tick {}", tick - t0);
    ensure!(left == full - 1, "1 damage: {full} -> {left}");
    ensure!(rig.carl_x() < start_x, "thrown toward -x");
    done(&rig, format!("T0 {t0}, hurt on {}", tick - t0))
}

/// `hrd_shove_jump_edges`: Carl at d 128, jump on attack tick 37 / 38 / 55 / 56.
pub fn hrd_shove_jump_edges(root: &Path) -> Result<Outcome, String> {
    edge_runs(
        root,
        Plan::carl_at(128.0),
        PileAttack::Shove,
        [37, 38, 55, 56],
        130,
    )
}

/// `hrd_shove_jump_lands_behind`: Carl at d 128, jump on attack tick 45: no blow
/// from the shove; after the lunge Carl is behind her (+x), and she turns before
/// her next tell.
pub fn hrd_shove_jump_lands_behind(root: &Path) -> Result<Outcome, String> {
    let plan = Plan::carl_at(128.0);
    let t0 = probe_t0(root, plan, PileAttack::Shove)?;
    let mut rig = Rig::new(root, plan)?;
    let mut turned_at = None;
    let mut behind_after_lunge = false;
    for _ in 0..(t0 + 200) {
        let tick = rig.next_tick();
        rig.step(if tick == t0 + 45 {
            carl_input(jump_input())
        } else {
            IDLE
        });
        if tick == t0 + 130 {
            ensure!(
                rig.hurts().is_empty(),
                "no blow from the shove: {:?}",
                rig.hurts()
            );
            behind_after_lunge = rig.carl_x() > rig.boss().boss().x();
        }
        if turned_at.is_none() && rig.mode() == PileMode::Turn {
            turned_at = Some(tick);
        }
    }
    ensure!(behind_after_lunge, "Carl behind her after the lunge");
    let turned = turned_at.ok_or("she never turned")?;
    let next_tell = rig
        .piles()
        .iter()
        .filter(|(t, e)| *t > t0 && matches!(e, PileEvent::Tell { .. }))
        .map(|(t, _)| *t)
        .next();
    ensure!(
        next_tell.is_none_or(|tell| turned < tell),
        "Turn on {turned} before the next tell {next_tell:?}"
    );
    done(&rig, format!("Turn on attack tick {}", turned - t0))
}

/// `hrd_shove_2p_both`: two players, Carl at d 112 and Donut at d 160: each is
/// hit once by one shove.
pub fn hrd_shove_2p_both(root: &Path) -> Result<Outcome, String> {
    let plan = Plan {
        donut_x: (E - 160.0) as i32,
        ..Plan::carl_at(112.0)
    }
    .with_control(Control::TwoPlayer);
    let mut rig = Rig::new(root, plan)?;
    rig.run(160, &|_| IDLE);
    let hurts = rig.hurts();
    let by = |hero: usize| hurts.iter().filter(|h| h.3 == hero).count();
    ensure!(by(0) == 1 && by(1) == 1, "one blow each: {hurts:?}");
    let shoves = rig
        .piles()
        .iter()
        .filter(|(_, e)| {
            matches!(
                e,
                PileEvent::Tell {
                    attack: PileAttack::Shove,
                    ..
                }
            )
        })
        .count();
    ensure!(shoves == 1, "one shove tell, got {shoves}");
    done(&rig, format!("{hurts:?}"))
}

/// `hrd_shove_jams_on_solid`: she at 470, Carl at 330 behind bank A (d 140): the
/// lunge clamps at pivot 464 on lunge tick 3 and she is Jammed for 72 ticks.
pub fn hrd_shove_jams_on_solid(root: &Path) -> Result<Outcome, String> {
    let plan = Plan {
        carl_x: 330,
        awake: Some((470.0, -1)),
        ..Plan::carl_at(140.0)
    };
    let mut rig = Rig::new(root, plan)?;
    let mut jam = None;
    let mut jammed_ticks = 0;
    let mut x_after = Vec::new();
    for _ in 0..200 {
        rig.step(IDLE);
        if rig.mode() == PileMode::Jammed {
            jammed_ticks += 1;
            x_after.push(rig.boss().boss().x());
        }
        if jam.is_none()
            && let Some((tick, _)) = rig
                .piles()
                .iter()
                .find(|(_, e)| matches!(e, PileEvent::Jammed { .. }))
        {
            jam = Some(*tick);
        }
    }
    let jam = jam.ok_or("no Jammed event")?;
    let t0 = first_tell(&rig, PileAttack::Shove).ok_or("no shove tell")?;
    ensure!(
        rig.piles().iter().any(|(_, e)| matches!(
            e,
            PileEvent::Jammed {
                obstacle: crate::pile_boss::ObstacleKind::BankA
            }
        )),
        "jammed on bank A"
    );
    // Tell 0..39, lunge tick 1 is attack tick 40: lunge tick 3 is attack tick 42.
    ensure!(jam == t0 + 42, "jam on attack tick {} (T0 {t0})", jam - t0);
    ensure!(jammed_ticks == 72, "Jammed for {jammed_ticks} ticks");
    ensure!(
        x_after.iter().all(|x| (*x - 464.0).abs() < 0.01),
        "clamped at 464, no motion backward: {:?}",
        x_after.first()
    );
    done(&rig, format!("T0 {t0}, jam on {}", jam - t0))
}

/// `hrd_turn_flank`: Carl behind her at 40 px: `Turn` 20 ticks, then a swipe tell
/// facing him.
pub fn hrd_turn_flank(root: &Path) -> Result<Outcome, String> {
    let plan = Plan {
        carl_x: (E + 40.0) as i32,
        ..Plan::carl_at(60.0)
    };
    let mut rig = Rig::new(root, plan)?;
    let mut turn_ticks = 0;
    let mut facing_at_tell = None;
    for _ in 0..80 {
        rig.step(IDLE);
        if rig.mode() == PileMode::Turn {
            turn_ticks += 1;
        }
        if facing_at_tell.is_none() && first_tell(&rig, PileAttack::Swipe).is_some() {
            facing_at_tell = Some(rig.boss().boss().facing());
        }
    }
    ensure!(turn_ticks == 20, "Turn for {turn_ticks} ticks");
    ensure!(facing_at_tell == Some(1), "tell facing {facing_at_tell:?}");
    done(&rig, "turn 20 then swipe at his side")
}

/// `hrd_defeat_grants_dynamite`: the Hoarder at 1 HP, one jab: one `EnemyDefeated
/// { target: PILE_BOSS_ID, instigator }`, one `CapabilityGranted
/// { special_thrown_charge }` on the same tick (the ledger's event trigger is the
/// only grant path), and the door opens on that tick. Under the `all` loadout the
/// same defeat grants nothing. Before the defeat nothing is held.
pub fn hrd_defeat_grants_dynamite(root: &Path) -> Result<Outcome, String> {
    let plan = Plan {
        hp: Some(1),
        ..Plan::carl_at(44.0)
    };
    let mut rig = Rig::new(root, plan)?;
    let range = rig.combat.env().ok_or("the room has an env")?.solid_range();
    ensure!(range.len() == 2, "door and gate are env solids: {range:?}");
    let (door, gate) = (range.start, range.start + 1);
    ensure!(
        rig.scene.world.solids[door].intact,
        "the door is closed at start"
    );
    ensure!(
        rig.scene.world.solids[gate].intact,
        "the gate is closed (awake)"
    );
    ensure!(
        !rig.combat
            .story_holds(crate::story_unlock::SPECIAL_THROWN_CHARGE),
        "no dynamite before the defeat"
    );
    let mut defeat_tick = None;
    for _ in 0..30 {
        let tick = rig.next_tick();
        rig.step(carl_input(attack_input()));
        if defeat_tick.is_none() && !rig.defeats().is_empty() {
            defeat_tick = Some(tick);
            ensure!(
                !rig.scene.world.solids[door].intact,
                "door open on the defeat tick"
            );
            ensure!(
                !rig.scene.world.solids[gate].intact,
                "gate open on the defeat tick"
            );
        }
    }
    let tick = defeat_tick.ok_or("no defeat inside 30 ticks")?;
    let defeats = rig.defeats();
    ensure!(defeats == [(tick, PILE_BOSS_ID, Some(0))], "{defeats:?}");
    let grants = rig.grants();
    ensure!(
        grants == [(tick, 0, "special_thrown_charge")],
        "exactly one grant on the defeat tick: {grants:?}"
    );
    ensure!(
        rig.combat
            .story_holds(crate::story_unlock::SPECIAL_THROWN_CHARGE),
        "dynamite held"
    );
    let receipts = rig.combat.story_ledger().ok_or("a ledger")?.receipts();
    ensure!(
        receipts.len() == 1 && receipts[0].source == "pile_boss_f01_defeated",
        "the receipt names the event trigger: {receipts:?}"
    );
    // Under `all`: the same defeat grants nothing (no story triggers).
    let mut all = Rig::new(
        root,
        Plan {
            loadout: Loadout::All,
            ..plan
        },
    )?;
    all.run(30, &|_| carl_input(attack_input()));
    ensure!(all.defeats().len() == 1, "defeated under `all` too");
    ensure!(
        all.grants().is_empty(),
        "no grant under `all`: {:?}",
        all.grants()
    );
    done(&rig, format!("defeated and granted on tick {tick}"))
}

/// `hrd_not_evadable`: Donut with the biscuit and the reflex armed stands at d 60:
/// the swipe is a `HeroHurt` and never a `HeroEvaded` (her blows have no evade
/// source, DM-Q18, K6).
pub fn hrd_not_evadable(root: &Path) -> Result<Outcome, String> {
    let plan = Plan {
        carl_x: 80,
        donut_x: (E - 60.0) as i32,
        gate_off: true,
        ..Plan::carl_at(60.0)
    }
    .with_control(Control::SoloDonut);
    let mut rig = Rig::new(root, plan)?;
    rig.combat.arm_reflex_for_test();
    rig.run(60, &|_| IDLE);
    let hurts = rig.hurts();
    ensure!(
        hurts.iter().any(|h| h.3 == 1 && h.1 == PILE_BOSS_SOURCE_ID),
        "Donut is hurt by her swipe: {hurts:?}"
    );
    let evaded = rig
        .combat
        .events
        .iter()
        .any(|e| matches!(e, CombatEvent::HeroEvaded { .. }));
    ensure!(!evaded, "no HeroEvaded");
    done(&rig, format!("{hurts:?}"))
}

/// Carl walks right (`move_x` 1) until `until`, then stands.
fn walk_then_idle(until: u64) -> impl Fn(u64) -> [RawInput; 2] {
    move |tick| {
        if tick < until {
            carl_input(run_input(1.0))
        } else {
            IDLE
        }
    }
}

fn engaged_tick(rig: &Rig) -> Option<u64> {
    rig.piles()
        .iter()
        .find(|(_, e)| matches!(e, PileEvent::Engaged))
        .map(|(t, _)| *t)
}

/// The scene's solid blocks of the room's env: `(door, gate)` indexes.
fn env_blocks(rig: &Rig) -> Result<(usize, usize), String> {
    let range = rig.combat.env().ok_or("the room has an env")?.solid_range();
    ensure!(range.len() == 2, "door and gate: {range:?}");
    Ok((range.start, range.start + 1))
}

/// `hrd_reveal_no_hits`: dormant, gate open; Carl walks right from x 428. The gate
/// closes and `Engaged` fires 8 ticks after Carl reaches x 432; no `Hurt` and no
/// tell for the 90-tick reveal; then Neutral.
pub fn hrd_reveal_no_hits(root: &Path) -> Result<Outcome, String> {
    let plan = Plan {
        carl_x: 428,
        ..Plan::carl_at(60.0)
    }
    .dormant();
    let mut rig = Rig::new(root, plan)?;
    let (_, gate) = env_blocks(&rig)?;
    ensure!(!rig.scene.world.solids[gate].intact, "the gate starts open");
    let mut crossed = None;
    let mut reveal_ticks = 0;
    let mut neutral_at = None;
    let walk = walk_then_idle(40);
    for _ in 0..260 {
        let tick = rig.next_tick();
        // A jab on every tick once Carl stands: she has no hurtbox in the reveal.
        let input = if tick >= 40 {
            carl_input(attack_input())
        } else {
            walk(tick)
        };
        rig.step(input);
        if crossed.is_none() && rig.carl_x() >= 432.0 {
            crossed = Some(tick);
        }
        if rig.mode() == PileMode::Reveal {
            reveal_ticks += 1;
        }
        if neutral_at.is_none() && engaged_tick(&rig).is_some() && rig.mode() != PileMode::Reveal {
            neutral_at = Some(tick);
        }
    }
    let crossed = crossed.ok_or("Carl never crossed x 432")?;
    let engaged = engaged_tick(&rig).ok_or("no Engaged")?;
    ensure!(
        engaged == crossed + 8,
        "Engaged on {engaged}, crossed {crossed}"
    );
    ensure!(reveal_ticks == 90, "reveal lasted {reveal_ticks} ticks");
    ensure!(
        neutral_at == Some(engaged + 90),
        "out of the reveal on {neutral_at:?}"
    );
    let early: Vec<_> = rig
        .piles()
        .into_iter()
        .filter(|(t, e)| {
            *t < engaged + 90 && matches!(e, PileEvent::Hurt { .. } | PileEvent::Tell { .. })
        })
        .collect();
    ensure!(
        early.is_empty(),
        "no Hurt and no tell in the reveal: {early:?}"
    );
    ensure!(
        rig.scene.world.solids[gate].intact,
        "the gate stands once engaged"
    );
    done(&rig, format!("crossed {crossed}, engaged {engaged}"))
}

/// `hrd_gate_2p_waits`: two players, Carl walks past 432 and Donut stays at x 150:
/// the gate stays open and she stays Dormant; when Donut walks to x 236 the gate
/// closes at once and `Engaged` fires.
pub fn hrd_gate_2p_waits(root: &Path) -> Result<Outcome, String> {
    let plan = Plan {
        carl_x: 428,
        donut_x: 150,
        ..Plan::carl_at(60.0)
    }
    .dormant()
    .with_control(Control::TwoPlayer);
    let mut rig = Rig::new(root, plan)?;
    let (_, gate) = env_blocks(&rig)?;
    let mut arrived = None;
    let donut_x = |rig: &Rig| rig.scene.world.body(crate::party::HeroId::Donut).x;
    for _ in 0..300 {
        let tick = rig.next_tick();
        let donut = if tick > 200 {
            run_input(1.0)
        } else {
            RawInput::NONE
        };
        rig.step([walk_then_idle(40)(tick)[0], donut]);
        if tick <= 200 {
            ensure!(
                engaged_tick(&rig).is_none()
                    && !rig.scene.world.solids[gate].intact
                    && rig.mode() == PileMode::Dormant,
                "she waits for Donut (tick {tick})"
            );
        }
        if arrived.is_none() && donut_x(&rig) >= 236.0 {
            arrived = Some(tick);
        }
    }
    let arrived = arrived.ok_or("Donut never reached x 236")?;
    let engaged = engaged_tick(&rig).ok_or("no Engaged")?;
    ensure!(
        engaged >= arrived && engaged <= arrived + 1,
        "Engaged on {engaged}, Donut inside on {arrived}"
    );
    ensure!(rig.scene.world.solids[gate].intact, "the gate closed");
    done(&rig, format!("inside {arrived}, engaged {engaged}"))
}

/// `hrd_gate_solo_regroup`: solo Carl, the plain bot held at x 150 by the fixture
/// (re-placed every tick): after 120 ticks past Carl's crossing the bot is placed
/// at x 248, the gate closes, `Engaged`.
pub fn hrd_gate_solo_regroup(root: &Path) -> Result<Outcome, String> {
    let plan = Plan {
        carl_x: 428,
        donut_x: 150,
        ..Plan::carl_at(60.0)
    }
    .dormant()
    .with_control(Control::SoloCarl);
    let mut rig = Rig::new(root, plan)?;
    let walk = walk_then_idle(40);
    let mut crossed = None;
    let mut placed = None;
    for _ in 0..260 {
        let tick = rig.next_tick();
        if placed.is_none() {
            let donut = rig.scene.world.body_mut(crate::party::HeroId::Donut);
            donut.x = 150.0;
            donut.vx = 0.0;
        }
        rig.step(walk(tick));
        if crossed.is_none() && rig.carl_x() >= 432.0 {
            crossed = Some(tick);
        }
        let x = rig.scene.world.body(crate::party::HeroId::Donut).x;
        if placed.is_none() && x >= 236.0 {
            placed = Some((tick, x));
        }
    }
    let crossed = crossed.ok_or("Carl never crossed")?;
    let (placed_at, x) = placed.ok_or("the bot was never placed")?;
    ensure!(
        placed_at == crossed + 120,
        "placed on {placed_at}, crossed {crossed}"
    );
    ensure!((x - 248.0).abs() < 0.01, "placed at x {x}");
    let engaged = engaged_tick(&rig).ok_or("no Engaged")?;
    ensure!(
        engaged == placed_at + 1,
        "Engaged on {engaged}, bot placed on {placed_at}"
    );
    done(
        &rig,
        format!("crossed {crossed}, placed {placed_at}, engaged {engaged}"),
    )
}

/// Count the ticks her reveal lasts in a run of `ticks` (Carl walks right).
fn reveal_length(rig: &mut Rig, ticks: u64) -> u64 {
    let walk = walk_then_idle(40);
    let mut n = 0;
    for _ in 0..ticks {
        let tick = rig.next_tick();
        rig.step(walk(tick));
        if rig.mode() == PileMode::Reveal {
            n += 1;
        }
    }
    n
}

fn restart(rig: &mut Rig) {
    rig.combat.restart(&mut rig.scene);
}

/// `hrd_retry_short_reveal`: the first reveal is 90 ticks, the reveal after a
/// retry 30 (HRD-Q05), with the gate open again.
pub fn hrd_retry_short_reveal(root: &Path) -> Result<Outcome, String> {
    let plan = Plan {
        carl_x: 428,
        ..Plan::carl_at(60.0)
    }
    .dormant();
    let mut rig = Rig::new(root, plan)?;
    let first = reveal_length(&mut rig, 200);
    ensure!(first == 90, "first reveal {first}");
    restart(&mut rig);
    let (_, gate) = env_blocks(&rig)?;
    ensure!(
        !rig.scene.world.solids[gate].intact,
        "the gate is open after the restart"
    );
    ensure!(rig.mode() == PileMode::Dormant, "fresh state");
    let retry = reveal_length(&mut rig, 200);
    ensure!(retry == 30, "retry reveal {retry}");
    done(&rig, format!("reveal {first} then {retry}"))
}

/// `hrd_restart_mid_reveal`: a restart on reveal tick 45 gives a fresh state, the
/// retry reveal is 30 ticks, and the state hash equals that of any other fresh
/// retry.
pub fn hrd_restart_mid_reveal(root: &Path) -> Result<Outcome, String> {
    let plan = Plan {
        carl_x: 428,
        ..Plan::carl_at(60.0)
    }
    .dormant();
    let mut a = Rig::new(root, plan)?;
    let walk = walk_then_idle(40);
    loop {
        let tick = a.next_tick();
        ensure!(tick < 400, "she never reached reveal tick 45");
        a.step(walk(tick));
        if a.mode() == PileMode::Reveal && a.boss().boss().mode_tick() == 45 {
            break;
        }
    }
    let elapsed = a.next_tick() - 1;
    restart(&mut a);
    // A fresh retry with the same elapsed count: Carl idles on x 428 (short of the
    // engage line), so she never wakes before the restart.
    let mut b = Rig::new(root, plan)?;
    b.run(elapsed, &|_| IDLE);
    restart(&mut b);
    ensure!(
        a.hash() == b.hash(),
        "a mid-reveal retry equals a fresh retry (elapsed {elapsed})"
    );
    let retry = reveal_length(&mut a, 200);
    ensure!(retry == 30, "retry reveal {retry}");
    done(&a, format!("retry reveal {retry}"))
}

/// `hrd_leash_return`: gate disabled and bank A removed (the settled case); Carl
/// stands at x 100: she stops at the leash edge 290, after 90 unreachable ticks
/// `Disengage`, she walks to 608 and holds; HP unchanged, no blow.
pub fn hrd_leash_return(root: &Path) -> Result<Outcome, String> {
    // HRDA2-AD3: she starts at x 340, not at E: from 760 the leash edge is 470 px
    // away (627 ticks at 45 px/s) and her 240-tick return (HRD-Q19) walks her home
    // before she gets there, so the sheet's E cannot show the edge.
    let plan = Plan {
        carl_x: 100,
        awake: Some((340.0, -1)),
        gate_off: true,
        no_bank: true,
        ..Plan::carl_at(60.0)
    };
    let mut rig = Rig::new(root, plan)?;
    let mut min_x = f32::MAX;
    let mut at_edge = 0;
    for _ in 0..900 {
        rig.step(IDLE);
        let x = rig.boss().boss().x();
        min_x = min_x.min(x);
        if (x - 290.0).abs() < 0.01 {
            at_edge += 1;
        }
    }
    let disengages = rig
        .piles()
        .iter()
        .filter(|(_, e)| matches!(e, PileEvent::Disengage))
        .count();
    ensure!(
        (min_x - 290.0).abs() < 0.01,
        "she stops at the leash edge: {min_x}"
    );
    ensure!(disengages == 1, "one Disengage, got {disengages}");
    ensure!(at_edge >= 90, "{at_edge} ticks at the edge");
    ensure!(
        (rig.boss().boss().x() - 608.0).abs() < 0.01 && rig.boss().boss().home_hold(),
        "home and holding: x {}",
        rig.boss().boss().x()
    );
    ensure!(rig.boss().boss().hp() == 30, "HP unchanged");
    ensure!(rig.hurts().is_empty(), "no blow");
    done(&rig, format!("{at_edge} ticks at the edge"))
}

/// `hrd_ai_steps_out_of_swipe`: P1 Carl idle at d 140; the plain bot Donut at d 50
/// is her target (50 + the 40 px bias is still nearer than 140): the follower steps
/// out on the Pound anchor and Donut takes no swipe.
pub fn hrd_ai_steps_out_of_swipe(root: &Path) -> Result<Outcome, String> {
    let plan = Plan {
        donut_x: (E - 50.0) as i32,
        gate_off: true,
        ..Plan::carl_at(140.0)
    }
    .with_control(Control::SoloCarl);
    let mut rig = Rig::new(root, plan)?;
    rig.run(120, &|_| IDLE);
    ensure!(
        first_tell(&rig, PileAttack::Swipe).is_some(),
        "she swipes at the bot"
    );
    let donut_hurts: Vec<_> = rig.hurts().into_iter().filter(|h| h.3 == 1).collect();
    ensure!(donut_hurts.is_empty(), "no swipe on Donut: {donut_hurts:?}");
    done(&rig, "the follower stepped out")
}

/// `hrd_ai_steps_out_untargeted`: P1 at d 44 is her target; the plain bot at d 84
/// is not, but the Pound carries `target: None`, so it steps out as well and takes
/// **no blow**. Both ways: P1 Carl with the bot Donut, and P1 Donut with the bot
/// Carl. The anchor is 13 px (HRD-Q33, reviewer ruling on #215): the sheet's 9
/// ignored the follower's 3.45 px follow pull-back, and the bot stood 0.1 px inside
/// the swipe box on the impact tick.
pub fn hrd_ai_steps_out_untargeted(root: &Path) -> Result<Outcome, String> {
    use crate::party::HeroId;
    let mut hash = 0u64;
    let mut notes = Vec::new();
    for (control, bot) in [
        (Control::SoloCarl, HeroId::Donut),
        (Control::SoloDonut, HeroId::Carl),
    ] {
        let (bot_x, p1_x) = (E - 84.0, E - 44.0);
        let plan = Plan {
            carl_x: if bot == HeroId::Carl { bot_x } else { p1_x } as i32,
            donut_x: if bot == HeroId::Donut { bot_x } else { p1_x } as i32,
            gate_off: true,
            ..Plan::carl_at(44.0)
        }
        .with_control(control);
        let mut rig = Rig::new(root, plan)?;
        let mut min_x = f32::MAX;
        for _ in 0..120 {
            rig.step(IDLE);
            min_x = min_x.min(rig.scene.world.body(bot).x);
        }
        ensure!(
            min_x < bot_x - 8.0,
            "{bot:?} bot steps out of the tell (min x {min_x})"
        );
        let p1 = 1 - bot.index();
        ensure!(
            rig.hurts().iter().any(|h| h.3 == p1),
            "the swipe still hits P1: {:?}",
            rig.hurts()
        );
        let bot_blows: Vec<_> = rig
            .hurts()
            .into_iter()
            .filter(|h| h.3 == bot.index())
            .collect();
        ensure!(
            bot_blows.is_empty(),
            "no blow on the {bot:?} bot: {bot_blows:?}"
        );
        notes.push(format!("{bot:?} bot min x {min_x:.1}"));
        hash = hash.rotate_left(9) ^ rig.hash();
    }
    Ok(Outcome {
        hash,
        note: notes.join("; "),
    })
}

/// The entry gate is a physical solid while it is closed: Carl outside (x 190)
/// walking right stops at the gate's face (x 208 less his half width), and the
/// same walk with the lever off (`hrdq41_gate_enabled` false) passes it.
pub fn hrd_gate_physical(root: &Path) -> Result<Outcome, String> {
    let stand = Plan {
        carl_x: 190,
        ..Plan::carl_at(60.0)
    };
    let mut closed = Rig::new(root, stand)?;
    closed.run(40, &|_| carl_input(run_input(1.0)));
    ensure!(
        closed.carl_x() <= 196.5,
        "the closed gate stops Carl at x {}",
        closed.carl_x()
    );
    let mut off = Rig::new(
        root,
        Plan {
            gate_off: true,
            ..stand
        },
    )?;
    off.run(40, &|_| carl_input(run_input(1.0)));
    ensure!(
        off.carl_x() > 210.0,
        "the lever off lets Carl by: {}",
        off.carl_x()
    );
    done(&closed, format!("stopped at {:.1}", closed.carl_x()))
}

/// The exit door: solid while she lives (Carl at the door cannot pass), open from
/// her defeat tick (Carl walks to the exit and completes the room).
pub fn hrd_exit_door_physical(root: &Path) -> Result<Outcome, String> {
    // Closed: Carl walks right into the door.
    let mut closed = Rig::new(
        root,
        Plan {
            carl_x: 880,
            ..Plan::carl_at(60.0)
        }
        .dormant(),
    )?;
    closed.run(60, &|_| carl_input(run_input(1.0)));
    ensure!(
        closed.carl_x() < 928.0 && !closed.scene.world.complete,
        "the closed door stops Carl at x {}",
        closed.carl_x()
    );
    // Open: defeat her at 1 HP, then walk through to the exit.
    let mut rig = Rig::new(
        root,
        Plan {
            hp: Some(1),
            ..Plan::carl_at(44.0)
        },
    )?;
    rig.run(8, &|_| carl_input(attack_input()));
    ensure!(!rig.defeats().is_empty(), "defeated");
    let mut complete_at = None;
    for _ in 0..200 {
        let tick = rig.next_tick();
        rig.step(carl_input(run_input(1.0)));
        if complete_at.is_none() && rig.scene.world.complete {
            complete_at = Some(tick);
        }
    }
    let complete = complete_at.ok_or("Carl never reached the exit")?;
    done(
        &rig,
        format!(
            "closed at x {}, exit reached on {complete}",
            closed.carl_x()
        ),
    )
}

/// Donut fires one Magic Missile every 16 ticks (the refire cadence) from tick 1:
/// the press edge is the input of each such tick.
fn missile_cadence(tick: u64) -> [RawInput; 2] {
    if tick % 16 == 1 {
        carl_input(attack_input())
    } else {
        IDLE
    }
}

/// `hrd_poise_flinch`: P1 Donut alone-ish (the solo bot Carl waits far behind) at d
/// 150, her shove cooldown preset to 180: four Magic Missiles at the refire cadence
/// from tick 1 land while she walks in; `Flinch` on the 4th hit and not again
/// within the 60 ticks of immunity.
pub fn hrd_poise_flinch(root: &Path) -> Result<Outcome, String> {
    let plan = Plan {
        carl_x: 80,
        donut_x: (E - 150.0) as i32,
        shove_cooldown: Some(180),
        gate_off: true,
        ..Plan::carl_at(150.0)
    }
    .with_control(Control::SoloDonut);
    let mut rig = Rig::new(root, plan)?;
    rig.run(140, &missile_cadence);
    let piles = rig.piles();
    let hurts: Vec<u64> = piles
        .iter()
        .filter_map(|(t, e)| matches!(e, PileEvent::Hurt { .. }).then_some(*t))
        .collect();
    let flinches: Vec<u64> = piles
        .iter()
        .filter_map(|(t, e)| matches!(e, PileEvent::Flinch).then_some(*t))
        .collect();
    ensure!(hurts.len() >= 7, "missiles land: {hurts:?}");
    ensure!(
        flinches == [hurts[3]],
        "Flinch exactly on the 4th hit ({}): {flinches:?}",
        hurts[3]
    );
    ensure!(
        hurts[4..]
            .iter()
            .take_while(|t| **t < flinches[0] + 60)
            .count()
            >= 3,
        "further hits inside the 60 ticks add no poise"
    );
    done(&rig, format!("hits {hurts:?}, flinch {flinches:?}"))
}

/// `hrd_disengage_trigger_band`: gate disabled and bank A removed; P1 Donut at
/// pivot 200 (88 px swipe trigger: she stops at the leash edge 290, 90 px away):
/// `Disengage` after 90 ticks at the edge, though her hurtbox still touches the
/// swipe box. She starts at x 340 (HRDA2-AD3).
pub fn hrd_disengage_trigger_band(root: &Path) -> Result<Outcome, String> {
    let plan = Plan {
        carl_x: 80,
        donut_x: 200,
        awake: Some((340.0, -1)),
        gate_off: true,
        no_bank: true,
        ..Plan::carl_at(60.0)
    }
    .with_control(Control::SoloDonut);
    let mut rig = Rig::new(root, plan)?;
    let mut edge_at = None;
    for _ in 0..300 {
        rig.step(IDLE);
        if edge_at.is_none() && (rig.boss().boss().x() - 290.0).abs() < 0.01 {
            edge_at = Some(rig.next_tick() - 1);
        }
    }
    let edge = edge_at.ok_or("she never reached the leash edge")?;
    let disengage = rig
        .piles()
        .iter()
        .find(|(_, e)| matches!(e, PileEvent::Disengage))
        .map(|(t, _)| *t)
        .ok_or("no Disengage")?;
    ensure!(
        first_tell(&rig, PileAttack::Swipe).is_none(),
        "no swipe from the leash edge at 90 px"
    );
    ensure!(
        (89..=92).contains(&(disengage - edge)),
        "Disengage {} ticks after the edge",
        disengage - edge
    );
    done(&rig, format!("edge {edge}, disengage {disengage}"))
}

/// `hrd_no_safe_snipe`: gate disabled and bank A removed; P1 Donut at x 150 fires
/// whenever the cadence allows: she holds at 290, walks home after 90 ticks (hits
/// never cancel the walk) and holds at 608; no blow on a hero. Starts at x 340
/// (HRDA2-AD3).
pub fn hrd_no_safe_snipe(root: &Path) -> Result<Outcome, String> {
    let plan = Plan {
        carl_x: 80,
        donut_x: 150,
        awake: Some((340.0, -1)),
        gate_off: true,
        no_bank: true,
        ..Plan::carl_at(60.0)
    }
    .with_control(Control::SoloDonut);
    let mut rig = Rig::new(root, plan)?;
    rig.run(900, &missile_cadence);
    let disengages = rig
        .piles()
        .iter()
        .filter(|(_, e)| matches!(e, PileEvent::Disengage))
        .count();
    ensure!(disengages == 1, "one Disengage, got {disengages}");
    ensure!(
        (rig.boss().boss().x() - 608.0).abs() < 0.01 && rig.boss().boss().home_hold(),
        "home and holding: x {}",
        rig.boss().boss().x()
    );
    ensure!(
        rig.piles()
            .iter()
            .any(|(_, e)| matches!(e, PileEvent::Hurt { .. })),
        "the missiles did land while she held the edge"
    );
    ensure!(
        rig.hurts().is_empty(),
        "no blow on a hero: {:?}",
        rig.hurts()
    );
    done(&rig, format!("hp {}", rig.boss().boss().hp()))
}

// ---------------------------------------------------------------- pilots --

/// Carl's scripted pilot (the base kit): wait in front of her at about 56 px, jump
/// the swipe on its first ticks, jump the shove by the sheet's reach table, and
/// punish every free window (Recover, Jammed, Flinch, Exposed, Get-up) with jabs.
/// `seed` varies the reaction delay (0..=2 ticks), the hover distance and the
/// start column, so eight seeds are eight different fights.
#[derive(Debug, Clone)]
pub struct CarlPilot {
    seed: u64,
    jumped: bool,
    jab_toggle: bool,
}

impl CarlPilot {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            jumped: false,
            jab_toggle: false,
        }
    }

    fn delay(&self) -> u32 {
        (self.seed % 3) as u32
    }

    fn hover(&self) -> f32 {
        52.0 + (self.seed % 4) as f32 * 3.0
    }

    /// The input of the next tick from the state after the previous one.
    pub fn input(&mut self, rig: &Rig) -> RawInput {
        let boss = rig.boss().boss();
        let carl = rig.carl_x();
        let grounded = rig.scene.world.hero.grounded;
        let dx = boss.x() - carl;
        let adx = dx.abs();
        let mode = boss.mode();
        let mt = boss.mode_tick();
        if grounded {
            // A jump is spent once Carl is back on the ground.
            self.jumped = false;
        }
        let toward = |gap: f32| {
            if gap > 0.0 { 1.0 } else { -1.0 }
        };
        let mut raw = RawInput::NONE;
        match mode {
            PileMode::SwipeTell | PileMode::SwipeSmear | PileMode::SwipeImpact => {
                if grounded && !self.jumped && adx < 96.0 && mt >= 2 + self.delay() && mt <= 28 {
                    raw.jump = true;
                    self.jumped = true;
                }
            }
            PileMode::ShoveTell | PileMode::Lunge => {
                if grounded && !self.jumped && (90.0..180.0).contains(&adx) {
                    // The middle of the sheet's clear window for this distance
                    // (96: 28..45, 128: 38..55, 160: 47..66, 176: 47..71).
                    let k = 36.0 + ((adx - 96.0) * 20.0 / 64.0).clamp(0.0, 25.0);
                    if mt >= k as u32 + self.delay() {
                        raw.jump = true;
                        self.jumped = true;
                    }
                }
            }
            PileMode::SwipeRecover
            | PileMode::ShoveRecover
            | PileMode::Jammed
            | PileMode::Flinch
            | PileMode::Exposed
            | PileMode::GetUp => {
                if grounded {
                    if adx > 44.0 {
                        raw.move_x = toward(dx);
                    } else {
                        self.jab_toggle = !self.jab_toggle;
                        raw.attack = self.jab_toggle;
                    }
                }
            }
            _ => {
                // Neutral, Walk, Turn, Cooldown: hold the hover distance.
                if grounded {
                    let hover = self.hover();
                    if adx > hover + 4.0 {
                        raw.move_x = toward(dx);
                    } else if adx < hover - 4.0 {
                        raw.move_x = -toward(dx);
                    }
                }
            }
        }
        raw
    }
}

/// Donut's scripted pilot (the biscuit, the base jump): fire a Magic Missile on every
/// refire tick, jump the swipe in its first ticks and the shove by the base-Donut
/// reach table (19..46 at d 96, 30..57 at d 128, 39..68 at d 160), never retreat
/// (she faces the way she walks, so a retreat would aim the missile away). `seed`
/// varies the reaction delay.
#[derive(Debug, Clone)]
pub struct DonutPilot {
    seed: u64,
    jumped: bool,
}

impl DonutPilot {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            jumped: false,
        }
    }

    pub fn input(&mut self, rig: &Rig) -> RawInput {
        let boss = rig.boss().boss();
        let body = rig.scene.world.body(crate::party::HeroId::Donut);
        let (grounded, x) = (body.grounded, body.x);
        let adx = (boss.x() - x).abs();
        let (mode, mt) = (boss.mode(), boss.mode_tick());
        let delay = (self.seed % 3) as u32;
        if grounded {
            self.jumped = false;
        }
        let mut raw = RawInput::NONE;
        match mode {
            PileMode::SwipeTell | PileMode::SwipeSmear | PileMode::SwipeImpact => {
                if grounded && !self.jumped && adx < 96.0 && mt >= 2 + delay && mt <= 28 {
                    raw.jump = true;
                    self.jumped = true;
                }
            }
            PileMode::ShoveTell | PileMode::Lunge => {
                let k = 32.0 + ((adx - 96.0) * 21.0 / 64.0).clamp(0.0, 22.0);
                if grounded
                    && !self.jumped
                    && (90.0..180.0).contains(&adx)
                    && mt >= k as u32 + delay
                {
                    raw.jump = true;
                    self.jumped = true;
                }
            }
            _ => {}
        }
        if !raw.jump && grounded && adx < 235.0 && (rig.next_tick() % 16 == 1) {
            raw.attack = true;
        }
        raw
    }
}

/// The pre-fight walk: from the spawn to the lair, jumping bank A on the way.
fn approach_input_for(rig: &Rig, x: f32, grounded: bool) -> RawInput {
    let _ = rig;
    let mut raw = run_input(1.0);
    // Bank A is [352, 416) and 48 px high: an ordinary jump from just before it.
    if grounded && (326.0..340.0).contains(&x) {
        raw.jump = true;
    }
    raw
}

/// A pilot run of the whole room: walk in from the spawn, fight, walk out. Returns
/// the rig and `(defeat tick, exit tick, longest stuck stretch, hero defeats)`.
fn clear_run(
    root: &Path,
    control: Control,
    seed: u64,
    limit: u64,
) -> Result<(Rig, u64, u64, u64, u32), String> {
    use crate::party::HeroId;
    let plan = Plan {
        carl_x: 112,
        donut_x: 80,
        gate_off: false,
        ..Plan::carl_at(60.0)
    }
    .dormant()
    .with_control(control);
    let mut rig = Rig::new(root, plan)?;
    let hero = if control == Control::SoloDonut {
        HeroId::Donut
    } else {
        HeroId::Carl
    };
    let (mut carl_pilot, mut donut_pilot) = (CarlPilot::new(seed), DonutPilot::new(seed));
    let (mut defeat, mut exit) = (None, None);
    let mut stuck = 0u64;
    let mut longest = 0u64;
    let pos = |rig: &Rig| rig.scene.world.body(hero).x;
    let mut last = (pos(&rig), 30u16);
    for _ in 0..limit {
        let engaged = engaged_tick(&rig).is_some();
        let alive = rig.boss().boss().alive();
        let body = rig.scene.world.body(hero);
        let (x, grounded) = (body.x, body.grounded);
        let input = if !alive {
            run_input(1.0)
        } else if !engaged {
            // Walk in; stop short of her body while she wakes (Donut stops well
            // short of her: she fights from range).
            let stop = if hero == HeroId::Donut {
                470.0 + (seed % 4) as f32 * 9.0
            } else {
                500.0
            };
            if x >= stop {
                RawInput::NONE
            } else {
                approach_input_for(&rig, x, grounded)
            }
        } else if hero == HeroId::Donut {
            donut_pilot.input(&rig)
        } else {
            carl_pilot.input(&rig)
        };
        rig.step(carl_input(input));
        if defeat.is_none() && !rig.defeats().is_empty() {
            defeat = Some(rig.next_tick() - 1);
        }
        if exit.is_none() && rig.scene.world.complete {
            exit = Some(rig.next_tick() - 1);
            break;
        }
        let now = (pos(&rig), rig.boss().boss().hp());
        if (now.0 - last.0).abs() < 0.01 && now.1 == last.1 && rig.boss().boss().alive() {
            stuck += 1;
            longest = longest.max(stuck);
        } else {
            stuck = 0;
        }
        last = now;
    }
    let deaths = rig.combat.deaths;
    Ok((
        rig,
        defeat.ok_or("she was never defeated")?,
        exit.ok_or("the hero never reached the exit")?,
        longest,
        deaths,
    ))
}

fn direct_clear(root: &Path, control: Control) -> Result<Outcome, String> {
    let mut hash = 0u64;
    let mut notes = Vec::new();
    for seed in 0..8u64 {
        let (rig, defeat, exit, stuck, deaths) = clear_run(root, control, seed, 6000)?;
        ensure!(deaths == 0, "seed {seed}: {deaths} hero defeats");
        ensure!(stuck <= 300, "seed {seed}: stuck {stuck} ticks");
        ensure!(
            rig.grants().len() == 1,
            "seed {seed}: one grant, got {:?}",
            rig.grants()
        );
        notes.push(format!("s{seed}: defeat {defeat}, exit {exit}"));
        hash = hash.rotate_left(7) ^ rig.hash();
    }
    Ok(Outcome {
        hash,
        note: notes.join("; "),
    })
}

/// `hrd_direct_clear_solo`: the scripted solo Carl pilot, seeds 0..8: she is
/// defeated and Carl reaches the exit in every seed, with no hero defeat and no
/// stuck stretch over 300 ticks; the clear ticks are reported.
pub fn hrd_direct_clear_solo(root: &Path) -> Result<Outcome, String> {
    direct_clear(root, Control::SoloCarl)
}

/// `hrd_direct_clear_donut`: the same with the scripted solo Donut pilot (the
/// swap puts Donut under P1; Carl is the plain bot), seeds 0..8.
pub fn hrd_direct_clear_donut(root: &Path) -> Result<Outcome, String> {
    direct_clear(root, Control::SoloDonut)
}

/// The Hoarder's room run to her defeat under `loadout`: she starts at 1 HP and
/// Carl jabs her down. The room fixture of `f01_late_fixtures` (the story over
/// relaunches carries its ledger); it replaces the Boiler-hall stand-in that
/// crossed the retired placeholder zone.
pub fn defeat_run(root: &Path, loadout: Loadout) -> Result<(SceneSim, CombatState), String> {
    let mut rig = Rig::new(
        root,
        Plan {
            hp: Some(1),
            loadout,
            ..Plan::carl_at(44.0)
        },
    )?;
    rig.run(30, &|_| carl_input(attack_input()));
    ensure!(!rig.defeats().is_empty(), "the Hoarder fell");
    Ok(rig.into_parts())
}

/// Every fixture, in order.
pub const FIXTURES: &[(&str, FixtureFn)] = &[
    ("hrd_swipe_hits_idle_carl", hrd_swipe_hits_idle_carl),
    ("hrd_swipe_jump_edges", hrd_swipe_jump_edges),
    ("hrd_swipe_punish", hrd_swipe_punish),
    ("hrd_armour_tell", hrd_armour_tell),
    ("hrd_shove_hits_idle_carl", hrd_shove_hits_idle_carl),
    ("hrd_shove_jump_edges", hrd_shove_jump_edges),
    ("hrd_shove_jump_lands_behind", hrd_shove_jump_lands_behind),
    ("hrd_shove_2p_both", hrd_shove_2p_both),
    ("hrd_shove_jams_on_solid", hrd_shove_jams_on_solid),
    ("hrd_turn_flank", hrd_turn_flank),
    ("hrd_defeat_grants_dynamite", hrd_defeat_grants_dynamite),
    ("hrd_not_evadable", hrd_not_evadable),
    ("hrd_reveal_no_hits", hrd_reveal_no_hits),
    ("hrd_gate_2p_waits", hrd_gate_2p_waits),
    ("hrd_gate_solo_regroup", hrd_gate_solo_regroup),
    ("hrd_retry_short_reveal", hrd_retry_short_reveal),
    ("hrd_restart_mid_reveal", hrd_restart_mid_reveal),
    ("hrd_leash_return", hrd_leash_return),
    ("hrd_ai_steps_out_of_swipe", hrd_ai_steps_out_of_swipe),
    ("hrd_ai_steps_out_untargeted", hrd_ai_steps_out_untargeted),
    ("hrd_exit_door_physical", hrd_exit_door_physical),
    ("hrd_gate_physical", hrd_gate_physical),
    ("hrd_poise_flinch", hrd_poise_flinch),
    ("hrd_disengage_trigger_band", hrd_disengage_trigger_band),
    ("hrd_no_safe_snipe", hrd_no_safe_snipe),
    ("hrd_direct_clear_solo", hrd_direct_clear_solo),
    ("hrd_direct_clear_donut", hrd_direct_clear_donut),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encounter::RosterKind;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// The committed final hash of each fixture (a deliberate change re-pins here).
    const GOLDENS: &[(&str, u64)] = &[
        ("hrd_swipe_hits_idle_carl", 0x50ba_b59f_4c54_fd89),
        ("hrd_swipe_jump_edges", 0x2cc6_0f06_d9b1_9790),
        ("hrd_swipe_punish", 0x53d2_786c_97de_addd),
        ("hrd_armour_tell", 0x6672_dcbe_b8ab_8cb7),
        ("hrd_shove_hits_idle_carl", 0x143d_05ee_5fb9_0bf9),
        ("hrd_shove_jump_edges", 0x7afe_603b_77d6_b696),
        ("hrd_shove_jump_lands_behind", 0xeb83_fcbe_474d_086e),
        ("hrd_shove_2p_both", 0xc59f_d826_63fa_76da),
        ("hrd_shove_jams_on_solid", 0x4da4_3218_3dcd_f353),
        ("hrd_turn_flank", 0xf0ed_1f48_7864_7178),
        ("hrd_defeat_grants_dynamite", 0x3206_266a_728e_229f),
        ("hrd_not_evadable", 0x9b88_f640_aa3b_e2cf),
        ("hrd_reveal_no_hits", 0x182c_ed6f_68e1_1f61),
        ("hrd_gate_2p_waits", 0x3c7b_389d_6bee_56ba),
        ("hrd_gate_solo_regroup", 0x81a5_11f1_8321_44ca),
        ("hrd_retry_short_reveal", 0x78c3_48ed_4a39_8b74),
        ("hrd_restart_mid_reveal", 0xd8f5_13a1_c3f9_f43d),
        ("hrd_leash_return", 0xf1cf_09bf_157d_538f),
        ("hrd_ai_steps_out_of_swipe", 0x001f_99e2_e557_9e54),
        ("hrd_ai_steps_out_untargeted", 0x0cc7_df5f_3447_0966),
        ("hrd_exit_door_physical", 0x8334_e001_0139_8372),
        ("hrd_gate_physical", 0x23ae_425d_f63b_f99b),
        ("hrd_poise_flinch", 0x8f13_b63b_b3c9_810e),
        ("hrd_disengage_trigger_band", 0xd6e1_3bd0_5f46_4736),
        ("hrd_no_safe_snipe", 0x3427_3333_a3ee_c759),
        ("hrd_direct_clear_solo", 0x2eb7_4857_c5db_8632),
        ("hrd_direct_clear_donut", 0xd4ee_3852_1b48_7abb),
    ];

    #[test]
    fn every_fixture_passes_replays_identically_and_has_its_committed_hash() {
        let mut wrong = Vec::new();
        let mut failed = Vec::new();
        for (name, run) in FIXTURES {
            let first = match run(&root()) {
                Ok(outcome) => outcome,
                Err(reason) => {
                    failed.push(format!("{name}: {reason}"));
                    continue;
                }
            };
            let second = run(&root()).expect("a passing fixture passes again");
            assert_eq!(first, second, "{name} replays identically");
            match GOLDENS.iter().find(|(n, _)| n == name) {
                Some((_, golden)) if *golden == first.hash => {}
                other => wrong.push(format!(
                    "{name}: {:#018x} (committed {other:?})",
                    first.hash
                )),
            }
        }
        assert!(failed.is_empty(), "fixtures that failed: {failed:#?}");
        assert!(wrong.is_empty(), "hashes to (re)pin: {wrong:#?}");
        assert_eq!(GOLDENS.len(), FIXTURES.len(), "one golden per fixture");
    }

    /// The sheet's names: every fixture the brief lists is present.
    #[test]
    fn the_required_fixtures_exist() {
        // The sheet's HRD-A2 table, minus the disclosed deferrals (the module docs).
        for name in [
            "hrd_reveal_no_hits",
            "hrd_gate_2p_waits",
            "hrd_gate_solo_regroup",
            "hrd_retry_short_reveal",
            "hrd_restart_mid_reveal",
            "hrd_swipe_hits_idle_carl",
            "hrd_swipe_jump_edges",
            "hrd_swipe_punish",
            "hrd_armour_tell",
            "hrd_poise_flinch",
            "hrd_shove_hits_idle_carl",
            "hrd_shove_jump_edges",
            "hrd_shove_jump_lands_behind",
            "hrd_shove_2p_both",
            "hrd_shove_jams_on_solid",
            "hrd_turn_flank",
            "hrd_leash_return",
            "hrd_disengage_trigger_band",
            "hrd_no_safe_snipe",
            "hrd_ai_steps_out_of_swipe",
            "hrd_ai_steps_out_untargeted",
            "hrd_not_evadable",
            "hrd_defeat_grants_dynamite",
            "hrd_direct_clear_solo",
            "hrd_direct_clear_donut",
        ] {
            assert!(FIXTURES.iter().any(|(n, _)| *n == name), "{name}");
        }
    }

    /// `solid_intact` on her door and gate (a room with no props) answers `false`
    /// without indexing props: the slot enemy decides them.
    #[test]
    fn solid_intact_on_the_door_and_gate_of_a_propless_room_does_not_panic() {
        let rig = Rig::new(&root(), Plan::carl_at(60.0)).unwrap();
        let env = rig.combat.env().unwrap();
        assert_eq!(env.props(), 0);
        for i in 0..env.solids().len() {
            assert!(!env.solid_intact(i));
        }
        assert!(!env.solid_intact(99));
    }

    /// The door and the gate are runtime env solids of one sync: the door a lock
    /// solid, the gate a slot gate; the room has no props.
    #[test]
    fn the_door_is_a_lock_solid_and_the_gate_a_slot_gate_of_the_one_env_sync() {
        use crate::combat_scene::EnvSolidKind;
        let rig = Rig::new(&root(), Plan::carl_at(60.0)).unwrap();
        let env = rig.combat.env().expect("the room runs the env sync");
        assert_eq!(env.props(), 0);
        let kinds: Vec<_> = env.solids().iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            [
                EnvSolidKind::Lock {
                    target: PILE_BOSS_ID
                },
                EnvSolidKind::Gate {
                    target: PILE_BOSS_ID
                }
            ]
        );
        let door = env.solids()[0].rect;
        assert_eq!(
            (door.x, door.w, door.h),
            (928, 16, 228),
            "the sidecar's lock solid"
        );
        // Nothing was appended twice, and a repeat sync is idempotent.
        let before = rig.scene.world.solids.len();
        let mut rig = rig;
        rig.combat.sync_env(&mut rig.scene);
        assert_eq!(rig.scene.world.solids.len(), before);
    }

    /// Negative: a lock that waits on a roster kind with no slot target id is an
    /// error, never an inert lock.
    #[test]
    fn a_lock_on_a_kind_without_a_target_id_is_refused() {
        let run = crate::f01_hoarder::launch(&root()).unwrap();
        let mut encounter = run.loaded.encounter.clone();
        encounter.roster[0].kind = RosterKind::Room(crate::camera_lab::room::EnemyKind::Goblin);
        let err = crate::combat_scene::EnvRuntime::new(&encounter).unwrap_err();
        assert!(err.contains("no slot target id"), "{err}");
    }

    /// The door stands while she lives and opens on exactly her defeat tick; a
    /// checkpoint respawn after the defeat keeps it open (the defeated stay
    /// defeated), a fixture restart closes it again.
    #[test]
    fn the_door_follows_her_through_defeat_respawn_and_restart() {
        let mut rig = Rig::new(
            &root(),
            Plan {
                hp: Some(1),
                ..Plan::carl_at(44.0)
            },
        )
        .unwrap();
        let (door, _) = env_blocks(&rig).unwrap();
        rig.run(2, &|_| IDLE);
        assert!(
            rig.scene.world.solids[door].intact,
            "closed before the defeat"
        );
        rig.run(10, &|_| carl_input(attack_input()));
        assert!(
            !rig.scene.world.solids[door].intact,
            "open after the defeat"
        );
        rig.combat.respawn(&mut rig.scene);
        assert!(
            !rig.scene.world.solids[door].intact,
            "a respawn keeps it open"
        );
        rig.combat.restart(&mut rig.scene);
        assert!(
            rig.scene.world.solids[door].intact,
            "a restart closes it again"
        );
    }

    /// The generic matcher: another target's defeat (a goblin index, the Juicer's
    /// id) never fires her trigger, and the Juicer's matcher ignores her.
    #[test]
    fn only_her_defeat_fires_her_trigger() {
        let mut rig = Rig::new(&root(), Plan::carl_at(44.0)).unwrap();
        for other in [0, 7, crate::boss::BOSS_ENEMY_ID] {
            rig.combat.test_observe_defeat(other, 5);
        }
        rig.run(2, &|_| IDLE);
        assert!(rig.grants().is_empty(), "{:?}", rig.grants());
        rig.combat.test_observe_defeat(PILE_BOSS_ID, 9);
        rig.run(1, &|_| IDLE);
        assert_eq!(rig.grants().len(), 1, "{:?}", rig.grants());
    }
}
