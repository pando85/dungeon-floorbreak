//! TROG-A scenario fixtures (`docs/01-design/floor-01-troglodyte-integration.md` section 7.1):
//! the troglodytes and `trog_run` driven by scripted party inputs on the launched room,
//! deterministic, seeded and fixed-tick. Every fixture runs twice and must end on an identical
//! state hash. Bevy-free.
//!
//! The scripts are **closed loops on the state** (the heroes' columns and the troglodytes'
//! phases), never tick counts. Where a plan row names an exact pivot (F01's "Carl at L - 48,
//! facing it", F02's "Donut planted at 432") the fixture writes that one position at setup, once,
//! exactly as the llama sheet's F11 exempts setup writes; every other step is an input. The
//! uncontrolled hero of an isolated fixture is **parked** in the protected spawn (x 80) each tick
//! so no companion or follower can add a second fighter (the evaluator's `isolated` variant).

use crate::carl_prog::CarlProgFlags;
use crate::combat::CombatEvent;
use crate::combat_scene::CombatState;
use crate::combat_scene::trog::TrogEnemy;
use crate::corridor::SceneSim;
use crate::donut_prog::DonutProgFlags;
use crate::f01_late::Loadout;
use crate::f01_trog::{self, TrogRoom, TrogRun};
use crate::input::{RawInput, SlotInputs};
use crate::party::{HeroId, PROVISIONAL_TUNING, PadId, PartyState, PartyTuning};
use crate::prototype::HERO_HALF_HEIGHT;
use crate::rest_fixtures::{BOT, FREE, Party};
use crate::trog::{State, Trog, TrogAct};
use std::path::Path;

/// The fixed-tick budget of a room fixture.
pub const MAX_TICKS: u64 = 6000;
/// The seed of the fixtures' `--cf02-seed`.
pub const SEED: u64 = 0x0F01_7206_5EED;
/// Where an isolated fixture parks the uncontrolled hero (inside `protected_spawn`).
pub const PARK_X: f32 = 80.0;

/// A launched troglodyte room with a party, stepped the way the interactive scene steps it.
pub struct TrogSim {
    pub run: TrogRun,
    pub scene: SceneSim,
    pub combat: CombatState,
    pub party: PartyState,
    /// The uncontrolled hero parked in the protected spawn each tick (`None`: free).
    pub parked: Option<HeroId>,
    pub solo_donut: bool,
}

impl TrogSim {
    /// The run room under the Story flags.
    pub fn new(root: &Path, party: Party, parked: Option<HeroId>) -> Result<Self, String> {
        let (carl, donut) = Loadout::Story.flags();
        Self::with_flags(root, party, parked, carl, donut)
    }

    pub fn with_flags(
        root: &Path,
        party: Party,
        parked: Option<HeroId>,
        carl: CarlProgFlags,
        donut: DonutProgFlags,
    ) -> Result<Self, String> {
        let run = f01_trog::launch(root, TrogRoom::Run)?;
        let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x)?;
        let combat = f01_trog::combat_for(&run, &mut scene, carl, donut, Some(SEED))?;
        let uncontrolled = match party {
            Party::Solo { uncontrolled } => uncontrolled,
            Party::TwoPlayer => BOT,
        };
        let mut state = PartyState::new(
            PartyTuning {
                uncontrolled_hero: uncontrolled,
                ..PROVISIONAL_TUNING
            }
            .with_cf02_swap(),
        );
        if party == Party::TwoPlayer {
            state.pad_connected(PadId(1));
            state.pad_start_pressed(PadId(2));
        }
        Ok(Self {
            run,
            scene,
            combat,
            party: state,
            parked,
            solo_donut: false,
        })
    }

    /// One tick: the swap, the party routing, the scene drive and the combat step.
    pub fn step(&mut self, slots: SlotInputs) {
        let hurt = self.combat.hurt_ticks_of(self.party.controlled_hero()) > 0;
        self.party.request_swap(slots[0].swap, hurt);
        let hero = self.party.hero_inputs(slots);
        let drive = self.party.drive(true);
        self.scene.set_drive(drive);
        let restarts = self.combat.restarts;
        self.combat.step_party(&mut self.scene, hero);
        let respawned = self.combat.take_respawned();
        if self.combat.restarts != restarts || respawned {
            self.party.restart();
        }
        if let Some(id) = self.parked {
            let body = self.scene.world.body_mut(id);
            body.x = PARK_X;
            body.vx = 0.0;
        }
    }

    /// One tick with `raw` on P1's slot.
    pub fn step1(&mut self, raw: RawInput) {
        self.step([raw, RawInput::default()]);
    }

    /// Make Donut the controlled hero (solo): the first tick presses the swap.
    pub fn swap_to_donut(&mut self) {
        self.step1(RawInput {
            swap: true,
            ..RawInput::NONE
        });
        self.solo_donut = true;
    }

    pub fn tick(&self) -> u64 {
        self.scene.world.tick
    }

    pub fn x_of(&self, id: HeroId) -> f32 {
        self.scene.world.body(id).x
    }

    pub fn foot_of(&self, id: HeroId) -> f32 {
        self.scene.world.body(id).y - HERO_HALF_HEIGHT
    }

    /// World y of the walking line.
    pub fn foot_of_floor(&self) -> f32 {
        self.scene.world.hero_foot_y()
    }

    pub fn complete(&self) -> bool {
        self.scene.world.complete
    }

    pub fn hash(&self) -> u64 {
        self.combat.state_hash(&self.scene)
    }

    /// A setup write: a hero stands at `x` facing `facing`, at rest.
    pub fn place(&mut self, id: HeroId, x: f32, facing: i8) {
        let body = self.scene.world.body_mut(id);
        body.x = x;
        body.vx = 0.0;
        body.facing = facing;
    }

    /// The troglodyte at roster index `i`.
    pub fn trog(&self, i: usize) -> &Trog {
        self.combat.slot_enemies()[i]
            .as_any()
            .and_then(|a| a.downcast_ref::<TrogEnemy>())
            .expect("a troglodyte")
            .trog()
    }

    pub fn health(&self, id: HeroId) -> u8 {
        self.scene.world.body(id).health
    }

    /// Every troglodyte act, `(tick, enemy index, act)`, in log order.
    pub fn acts(&self) -> Vec<(u64, usize, TrogAct)> {
        self.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Trog { tick, enemy, act } => Some((
                    tick,
                    (enemy - crate::combat_scene::trog::TROG_ID_BASE) as usize,
                    act,
                )),
                _ => None,
            })
            .collect()
    }

    pub fn hurts(&self) -> Vec<(u64, HeroId)> {
        self.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::HeroHurt { tick, hero, .. } => Some((tick, hero)),
                _ => None,
            })
            .collect()
    }

    pub fn defeats(&self) -> usize {
        self.combat
            .events
            .iter()
            .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
            .count()
    }

    pub fn hero_defeats(&self) -> usize {
        self.combat
            .events
            .iter()
            .filter(|e| matches!(e, CombatEvent::HeroDefeated { .. }))
            .count()
    }

    pub fn living(&self) -> Vec<usize> {
        (0..self.combat.slot_enemies().len())
            .filter(|&i| self.combat.slot_enemies()[i].alive())
            .collect()
    }
}

// ----------------------------------------------------------------- inputs

fn press() -> RawInput {
    RawInput {
        attack: true,
        ..RawInput::NONE
    }
}

fn moving(dir: f32) -> RawInput {
    RawInput {
        move_x: dir,
        ..RawInput::NONE
    }
}

fn stomp() -> RawInput {
    RawInput {
        physical: true,
        down: true,
        ..RawInput::NONE
    }
}

/// A read-only view of a launched room: what the scripted pilots read (a pure function of it).
pub struct Probe<'a> {
    pub scene: &'a SceneSim,
    pub statics: &'a [crate::encounter::StaticEntry],
}

impl Probe<'_> {
    pub fn x_of(&self, id: HeroId) -> f32 {
        self.scene.world.body(id).x
    }

    pub fn foot_of(&self, id: HeroId) -> f32 {
        self.scene.world.body(id).y - HERO_HALF_HEIGHT
    }

    /// World y of the walking line.
    pub fn foot_of_floor(&self) -> f32 {
        self.scene.world.hero_foot_y()
    }

    pub fn tick(&self) -> u64 {
        self.scene.world.tick
    }
}

impl TrogSim {
    /// The pilots' read-only view of this room.
    pub fn probe(&self) -> Probe<'_> {
        Probe {
            scene: &self.scene,
            statics: &self.run.loaded.encounter.statics,
        }
    }
}

/// Carl's brawler policy against the troglodyte `t` (a pure function of the state): what a
/// competent player does with the plain moves (TROG-AD22): back out of a committed tell, punish
/// the recovery low (the stomp on the Pygmy's landing, the jab in Rest), step into the Virtuoso's
/// dead zone and jab, never trade in front of a smash.
pub fn carl_vs(p: &Probe, t: &Trog, last_press: &mut u64) -> RawInput {
    let carl = p.x_of(HeroId::Carl);
    let dx = t.x - carl;
    let d = dx.abs();
    let dir = if dx >= 0.0 { 1.0 } else { -1.0 };
    let now = p.tick();
    let idle = !p.scene.world.hero.melee.busy();
    let ready = idle && now.saturating_sub(*last_press) >= 14;
    let mut jab = |input: RawInput| {
        *last_press = now;
        input
    };
    match t.state {
        State::Dormant | State::Down => {
            // Walk toward it until it wakes.
            if d > 60.0 {
                moving(dir)
            } else {
                RawInput::NONE
            }
        }
        State::Tell => match t.species {
            crate::trog::Species::Pygmy => {
                // Back out until a stomp box from L - 44 is still reachable: away from the
                // landing, at least 48 px.
                let l = t.landing_x();
                let away = -dir;
                if (carl - l).abs() < 50.0 || (carl - l) * away < 0.0 {
                    moving(away)
                } else {
                    RawInput::NONE
                }
            }
            crate::trog::Species::Basher => {
                if d < 58.0 {
                    moving(-dir)
                } else {
                    RawInput::NONE
                }
            }
            crate::trog::Species::Virtuoso => {
                // In under the tongue: the dead zone is inside 44 px.
                if d > 36.0 {
                    moving(dir)
                } else {
                    RawInput::NONE
                }
            }
        },
        State::Active => match t.species {
            crate::trog::Species::Pygmy => {
                let l = t.landing_x();
                let away = -dir;
                if (carl - l).abs() < 50.0 && (carl - l) * away >= 0.0 {
                    moving(away)
                } else {
                    RawInput::NONE
                }
            }
            crate::trog::Species::Basher => {
                if d < 58.0 {
                    moving(-dir)
                } else {
                    RawInput::NONE
                }
            }
            crate::trog::Species::Virtuoso => {
                if d > 36.0 {
                    moving(dir)
                } else {
                    RawInput::NONE
                }
            }
        },
        State::Recover => match t.species {
            crate::trog::Species::Pygmy => {
                // The stomp on the landing: close to the box's reach, then press.
                let want = 44.0;
                let l = t.x;
                if d > want + 3.0 {
                    moving(dir)
                } else if ready {
                    jab(stomp())
                } else {
                    let _ = l;
                    RawInput::NONE
                }
            }
            _ => {
                if d > 46.0 {
                    moving(dir)
                } else if ready {
                    jab(press())
                } else {
                    RawInput::NONE
                }
            }
        },
        State::Approach | State::Rest | State::Hurt => {
            let reach = match t.species {
                crate::trog::Species::Pygmy => 36.0,
                crate::trog::Species::Basher => 44.0,
                crate::trog::Species::Virtuoso => 36.0,
            };
            if d > reach + 3.0 {
                moving(dir)
            } else if ready {
                jab(press())
            } else {
                RawInput::NONE
            }
        }
    }
}

/// A hero pushing toward a low step jumps it (the refuge and the bell step are 32 px lane steps).
pub fn with_hop(p: &Probe, hero: HeroId, mut raw: RawInput) -> RawInput {
    let body = p.scene.world.body(hero);
    if raw.move_x == 0.0 || !body.grounded {
        return raw;
    }
    let ahead = body.x + raw.move_x.signum() * 30.0;
    let near = p.statics.iter().any(|st| {
        let (l, r) = (st.rect.x as f32, st.rect.right() as f32);
        let inside = ahead >= l && ahead <= r;
        let on_it = body.x >= l && body.x <= r;
        inside && !on_it
    });
    raw.jump = near;
    raw
}

/// Donut's caster policy: walk to a firing column, face the troglodyte and cast on the cooldown.
pub fn donut_vs(p: &Probe, t: &Trog, last_press: &mut u64) -> RawInput {
    let donut = p.x_of(HeroId::Donut);
    // Standing on the 64 px platform her bolts fly over every troglodyte (a vantage, not a
    // turret): she walks off its right edge to the floor first.
    if p.foot_of(HeroId::Donut) > p.foot_of_floor() + 20.0 {
        return moving(1.0);
    }
    let dx = t.x - donut;
    let dir = if dx >= 0.0 { 1.0 } else { -1.0 };
    let now = p.tick();
    if dx.abs() > 200.0 {
        return moving(dir);
    }
    if now.saturating_sub(*last_press) >= 17 {
        *last_press = now;
        // Face it with the press: a move input of the same tick turns her.
        return RawInput {
            move_x: dir * 0.01,
            attack: true,
            ..RawInput::NONE
        };
    }
    moving(dir * 0.01)
}

// ----------------------------------------------------------------- pilots

/// The evidence pilots of `--trog-pilot` (window captures and headless runs of `trog_run`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pilot {
    /// Carl fights the troglodytes in order with the brawler policy, then walks to the exit.
    Carl,
    /// The first tick swaps to Donut, who casts at them from the floor, then walks to the exit.
    Donut,
}

impl Pilot {
    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "carl" => Ok(Self::Carl),
            "donut" => Ok(Self::Donut),
            other => Err(format!("unknown --trog-pilot {other:?} (carl or donut)")),
        }
    }
}

/// The pilot's input for this tick: a pure function of the finished state (the brawler policies
/// with no memory, so a press repeats whenever Carl's move is idle).
pub fn pilot(
    kind: Pilot,
    combat: &CombatState,
    scene: &SceneSim,
    statics: &[crate::encounter::StaticEntry],
    exit_x: f32,
) -> RawInput {
    let probe = Probe { scene, statics };
    if kind == Pilot::Donut && scene.world.tick == 0 {
        return RawInput {
            swap: true,
            ..RawInput::NONE
        };
    }
    let hero = match kind {
        Pilot::Carl => HeroId::Carl,
        Pilot::Donut => HeroId::Donut,
    };
    let target = combat.slot_enemies().iter().find_map(|e| {
        e.as_any()
            .and_then(|a| a.downcast_ref::<TrogEnemy>())
            .filter(|t| t.trog().alive())
            .map(|t| t.trog().clone())
    });
    let raw = match (&target, kind) {
        (Some(t), Pilot::Carl) => carl_vs(&probe, t, &mut 0),
        (Some(t), Pilot::Donut) => donut_vs(&probe, t, &mut 0),
        (None, _) => moving(if probe.x_of(hero) < exit_x { 1.0 } else { 0.0 }),
    };
    with_hop(&probe, hero, raw)
}

// ---------------------------------------------------------------- reports

/// What a fixture run reports.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub name: &'static str,
    pub ticks: u64,
    pub hash: u64,
    pub exit_tick: Option<u64>,
    pub defeats: usize,
    pub hero_defeats: usize,
    pub hurts: Vec<(u64, HeroId)>,
    pub acts: Vec<(u64, usize, TrogAct)>,
}

impl Report {
    pub fn of(name: &'static str, sim: &TrogSim) -> Self {
        Self {
            name,
            ticks: sim.tick(),
            hash: sim.hash(),
            exit_tick: sim.complete().then(|| sim.tick()),
            defeats: sim.defeats(),
            hero_defeats: sim.hero_defeats(),
            hurts: sim.hurts(),
            acts: sim.acts(),
        }
    }

    pub fn count(&self, f: impl Fn(&TrogAct) -> bool) -> usize {
        self.acts.iter().filter(|(_, _, a)| f(a)).count()
    }

    /// The text of the evidence line.
    pub fn line(&self) -> String {
        format!(
            "TROG_FIXTURE {} ticks={} exit_tick={} defeats={} hero_defeats={} hurts={} tells={} \
             whiffs={} strikes={} interrupted={} hash={:#018x}",
            self.name,
            self.ticks,
            self.exit_tick.map_or("none".to_owned(), |t| t.to_string()),
            self.defeats,
            self.hero_defeats,
            self.hurts.len(),
            self.count(|a| matches!(a, TrogAct::Tell { .. })),
            self.count(|a| *a == TrogAct::Whiff),
            self.count(|a| *a == TrogAct::Strike),
            self.count(|a| *a == TrogAct::Interrupted),
            self.hash,
        )
    }
}

/// The scripted cases, by their design ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    /// F01: Carl dodges the leap, stomps the landing and jabs the Rest.
    PygmyDodgeAndPunish,
    /// F02: Donut planted at 432 casts: the positive control.
    PygmyStationaryDonut,
    /// F03: Carl flanks the Basher: one interrupt, then a jab per recovery.
    BasherFlankAndPunish,
    /// F04: four bolts defeat the Basher; none interrupts.
    BasherBoltsDoNotInterrupt,
    /// F07: Carl solo (companion parked) clears the run.
    RunSoloCarl,
    /// F07: Donut solo (Carl parked) clears the run.
    RunSoloDonut,
    /// F07: P1 Carl and P2 Donut.
    Run2P,
    /// F07: Carl solo with the default `cf02_free` companion.
    RunSoloCompanion,
}

impl Case {
    pub const ALL: [Case; 8] = [
        Case::PygmyDodgeAndPunish,
        Case::PygmyStationaryDonut,
        Case::BasherFlankAndPunish,
        Case::BasherBoltsDoNotInterrupt,
        Case::RunSoloCarl,
        Case::RunSoloDonut,
        Case::Run2P,
        Case::RunSoloCompanion,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::PygmyDodgeAndPunish => "trog_pygmy_dodge_and_punish",
            Self::PygmyStationaryDonut => "trog_pygmy_stationary_donut",
            Self::BasherFlankAndPunish => "trog_basher_flank_and_punish",
            Self::BasherBoltsDoNotInterrupt => "trog_basher_bolts_do_not_interrupt",
            Self::RunSoloCarl => "trog_run_solo_carl",
            Self::RunSoloDonut => "trog_run_solo_donut",
            Self::Run2P => "trog_run_2p",
            Self::RunSoloCompanion => "trog_run_solo_companion",
        }
    }
}

/// Run a case to its end.
pub fn run(root: &Path, case: Case) -> Result<(Report, TrogSim), String> {
    let mut sim = match case {
        Case::PygmyDodgeAndPunish | Case::BasherFlankAndPunish | Case::RunSoloCarl => {
            TrogSim::new(root, Party::Solo { uncontrolled: BOT }, Some(HeroId::Donut))?
        }
        Case::PygmyStationaryDonut | Case::BasherBoltsDoNotInterrupt | Case::RunSoloDonut => {
            let mut s = TrogSim::new(root, Party::Solo { uncontrolled: BOT }, Some(HeroId::Carl))?;
            s.swap_to_donut();
            s
        }
        Case::Run2P => TrogSim::new(root, Party::TwoPlayer, None)?,
        Case::RunSoloCompanion => TrogSim::new(root, Party::Solo { uncontrolled: FREE }, None)?,
    };
    match case {
        Case::PygmyDodgeAndPunish => pygmy_dodge_and_punish(&mut sim),
        Case::PygmyStationaryDonut => pygmy_stationary_donut(&mut sim),
        Case::BasherFlankAndPunish => basher_flank_and_punish(&mut sim),
        Case::BasherBoltsDoNotInterrupt => basher_bolts(&mut sim),
        Case::RunSoloCarl | Case::RunSoloCompanion => clear_run(&mut sim, HeroId::Carl),
        Case::RunSoloDonut => clear_run(&mut sim, HeroId::Donut),
        Case::Run2P => clear_run_2p(&mut sim),
    }
    Ok((Report::of(case.name(), &sim), sim))
}

fn run_until(sim: &mut TrogSim, max: u64, mut f: impl FnMut(&TrogSim) -> bool) -> bool {
    for _ in 0..max {
        if f(sim) {
            return true;
        }
        sim.step1(RawInput::NONE);
    }
    f(sim)
}

/// F01. Carl walks in until the Pygmy wakes and commits its leap at his pivot; he is then placed
/// at L - 48 facing the landing, takes no blow, stomps on the first land tick and jabs the Rest.
fn pygmy_dodge_and_punish(sim: &mut TrogSim) {
    sim.place(HeroId::Carl, 560.0, -1);
    assert!(run_until(sim, 400, |s| s.trog(0).state == State::Tell));
    let l = sim.trog(0).landing_x();
    sim.place(HeroId::Carl, l - 48.0, 1);
    // The leap: the Pygmy flies by and lands (the first Recover tick).
    assert!(run_until(sim, 80, |s| s.trog(0).state == State::Recover));
    // The stomp on the first land tick; Carl's move runs out inside the Pygmy's Rest.
    sim.step1(stomp());
    for _ in 0..60 {
        sim.step1(RawInput::NONE);
        if !sim.scene.world.hero.melee.busy() {
            break;
        }
    }
    // Rest: one tick in, then the jab.
    sim.step1(moving(1.0));
    sim.step1(press());
    for _ in 0..40 {
        sim.step1(RawInput::NONE);
    }
}

/// F02. Donut is planted on the floor at 432 facing right and casts every cooldown.
fn pygmy_stationary_donut(sim: &mut TrogSim) {
    sim.place(HeroId::Donut, 432.0, 1);
    let mut last = 0;
    for _ in 0..600 {
        if !sim.trog(0).alive() {
            break;
        }
        let raw = if sim.tick().saturating_sub(last) >= 17 {
            last = sim.tick();
            press()
        } else {
            RawInput::NONE
        };
        sim.step1(raw);
    }
}

/// F03. Carl at 36 px in front of the Basher: he jabs inside the first tell ticks (one interrupt),
/// then backs out of every later smash and lands a jab in each recovery.
fn basher_flank_and_punish(sim: &mut TrogSim) {
    let bx = 1376.0;
    sim.place(HeroId::Carl, bx - 36.0, 1);
    let mut last = 0u64;
    let mut cycle = 0;
    let mut prev_tell = false;
    for _ in 0..MAX_TICKS {
        let t = sim.trog(1).clone();
        if !t.alive() {
            break;
        }
        let tell = t.state == State::Tell;
        if tell && !prev_tell {
            cycle += 1;
        }
        prev_tell = tell;
        let raw = if cycle == 1
            && tell
            && t.attack_tick() == 4
            && sim.tick().saturating_sub(last) >= 14
        {
            last = sim.tick();
            press()
        } else if cycle <= 1 && t.state != State::Dormant {
            // Hold the first cycle's column until the interrupt is spent.
            RawInput::NONE
        } else {
            carl_vs(&sim.probe(), &t, &mut last)
        };
        sim.step1(raw);
    }
}

/// F04. Donut casts from far away: the bolts deal damage and never interrupt.
fn basher_bolts(sim: &mut TrogSim) {
    sim.place(HeroId::Donut, 1376.0 - 200.0, 1);
    let mut last = 0;
    for _ in 0..1500 {
        if !sim.trog(1).alive() {
            break;
        }
        let raw = if sim.tick().saturating_sub(last) >= 17 {
            last = sim.tick();
            press()
        } else {
            RawInput::NONE
        };
        sim.step1(raw);
    }
}

/// What F08 reports.
#[derive(Debug, Clone, PartialEq)]
pub struct RetryFacts {
    /// A hero was defeated in T2 (the party defeat).
    pub party_defeats: usize,
    /// After the checkpoint respawn: the Pygmy is still down, the Basher stands at its spawn,
    /// whole and asleep again.
    pub pygmy_down: bool,
    pub basher_state: State,
    pub basher_hp: u16,
    pub basher_x: f32,
    /// After a fixture restart every troglodyte is rebuilt.
    pub restart_all_alive: bool,
    pub restart_states: Vec<State>,
    pub hash: u64,
}

/// F08: a party defeat in T2, the checkpoint respawn, then a fixture restart.
pub fn run_retry(root: &Path) -> Result<RetryFacts, String> {
    let mut sim = TrogSim::new(root, Party::Solo { uncontrolled: BOT }, Some(HeroId::Donut))?;
    sim.place(HeroId::Carl, 560.0, -1);
    let mut last = 0u64;
    // T1: Carl defeats the Pygmy.
    for _ in 0..MAX_TICKS {
        if !sim.trog(0).alive() {
            break;
        }
        let t = sim.trog(0).clone();
        let raw = carl_vs(&sim.probe(), &t, &mut last);
        let raw = with_hop(&sim.probe(), HeroId::Carl, raw);
        sim.step1(raw);
    }
    if sim.trog(0).alive() {
        return Err("the Pygmy was not defeated".into());
    }
    // T2: Carl stands in front of the Basher and takes every smash.
    let bx = sim.trog(1).spawn_x();
    sim.place(HeroId::Carl, bx - 40.0, 1);
    for _ in 0..MAX_TICKS {
        if sim.hero_defeats() > 0 {
            break;
        }
        // A blow knocks him back: he walks in again and takes the next smash.
        let gap = (bx - 40.0) - sim.x_of(HeroId::Carl);
        sim.step1(if gap > 6.0 {
            moving(1.0)
        } else {
            RawInput::NONE
        });
    }
    let party_defeats = sim.hero_defeats();
    for _ in 0..30 {
        sim.step1(RawInput::NONE);
    }
    sim.combat.respawn(&mut sim.scene);
    sim.party.restart();
    sim.step1(RawInput::NONE);
    let pygmy_down = !sim.trog(0).alive();
    let (basher_state, basher_hp, basher_x) = {
        let b = sim.trog(1);
        (b.state, b.hp, b.x)
    };
    // The fixture restart rebuilds everything.
    sim.combat.restart(&mut sim.scene);
    sim.party.restart();
    sim.step1(RawInput::NONE);
    let restart_states: Vec<State> = (0..3).map(|i| sim.trog(i).state).collect();
    Ok(RetryFacts {
        party_defeats,
        pygmy_down,
        basher_state,
        basher_hp,
        basher_x,
        restart_all_alive: sim.living().len() == 3,
        restart_states,
        hash: sim.hash(),
    })
}

fn exit_x(sim: &TrogSim) -> f32 {
    sim.run.loaded.encounter.exit.x as f32 + 32.0
}

/// One hero clears the run: fight the nearest living troglodyte, then walk to the exit.
fn clear_run(sim: &mut TrogSim, hero: HeroId) {
    if hero == HeroId::Donut {
        // Carl is parked and Donut was swapped in by the case.
    }
    let (mut last_c, mut last_d) = (0u64, 0u64);
    for _ in 0..MAX_TICKS {
        if sim.complete() {
            break;
        }
        let living = sim.living();
        let raw = match living.first().copied() {
            Some(i) => {
                let t = sim.trog(i).clone();
                match hero {
                    HeroId::Carl => carl_vs(&sim.probe(), &t, &mut last_c),
                    HeroId::Donut => donut_vs(&sim.probe(), &t, &mut last_d),
                }
            }
            None => {
                let x = sim.x_of(hero);
                moving(if x < exit_x(sim) { 1.0 } else { 0.0 })
            }
        };
        let raw = with_hop(&sim.probe(), hero, raw);
        sim.step1(raw);
    }
}

/// P1 Carl and P2 Donut each fight the nearest living troglodyte, then walk on.
fn clear_run_2p(sim: &mut TrogSim) {
    let (mut last_c, mut last_d) = (0u64, 0u64);
    for _ in 0..MAX_TICKS {
        if sim.complete() {
            break;
        }
        let (c, d) = match sim.living().first().copied() {
            Some(i) => {
                let t = sim.trog(i).clone();
                (
                    carl_vs(&sim.probe(), &t, &mut last_c),
                    donut_vs(&sim.probe(), &t, &mut last_d),
                )
            }
            None => (
                moving(if sim.x_of(HeroId::Carl) < exit_x(sim) {
                    1.0
                } else {
                    0.0
                }),
                moving(if sim.x_of(HeroId::Donut) < exit_x(sim) {
                    1.0
                } else {
                    0.0
                }),
            ),
        };
        let (c, d) = (
            with_hop(&sim.probe(), HeroId::Carl, c),
            with_hop(&sim.probe(), HeroId::Donut, d),
        );
        sim.step([c, d]);
    }
}

#[cfg(test)]
mod tests;
