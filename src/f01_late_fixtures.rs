//! PT-SCENE fixtures: the story of `--scene f01_late` as scripted, hashed runs.
//!
//! Bevy-free. Each run is one **room launch** built the way the scene builds
//! it, driven by a script or by the boss pilot, and each ends with a ledger
//! snapshot the next launch carries (CP-Q07). They are library fixtures: the
//! tests below pin their final hashes and replay each twice; they are not in
//! the `--party-fixture` registries (no committed replay files, no CI loop), the
//! scene's own headless smoke covers the launched rooms.
//!
//! | run | room | what it shows |
//! | --- | --- | --- |
//! | [`hoarder_defeat`] | the Hoarder's room (HRD-A2): her defeat by the real slot enemy | the pre-Hoarder pin and the ledger's event trigger `pile_boss_f01_defeated`: exactly one dynamite grant, K1 |
//! | [`boiler_relaunch`] | the Boiler, carried | the carried ledger, no second grant |
//! | [`juicer_relaunch`] | the Juicer, carried | the gauntlet and the blink exactly once at the defeat, and nothing under `all` |
//! | [`juicer_pilot`] | the Juicer's room, played (PT-SCENE-B) | solo Carl, solo Donut by swap and the `cf02_free` companion at seeds 0..8, the stuck limit, the double replay |

use std::path::Path;

use crate::boss_scene::{Pilot, boss_base, hp, load_arena};
use crate::combat::CombatEvent;
use crate::combat_scene::{CombatState, Fixture};
use crate::companion::profile::ProfileSetId;
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::donut_prog;
use crate::f01_late::{Loadout, boiler_combat};
use crate::input::RawInput;
use crate::party::{HeroId, PROVISIONAL_TUNING, PartyTuning, UncontrolledHero};
use crate::party_scene::{PartyFixture, PartyFixtureSim};
use crate::story_unlock::{LedgerSnapshot, SceneKind};

/// The pre-Hoarder pin of the Hoarder's room.
pub const PRE_PIN: &str = "f01_pin_pre_pile_boss";

/// Every `CapabilityGranted` of a combat state: `(tick, hero, capability)`.
fn combat_grants(combat: &CombatState) -> Vec<(u64, u8, &'static str)> {
    combat
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

/// A finished room run: the final state and what it granted.
pub struct RoomRun {
    pub scene: SceneSim,
    pub combat: CombatState,
}

impl RoomRun {
    /// Every `CapabilityGranted` of the run: `(tick, hero, capability)`.
    pub fn grants(&self) -> Vec<(u64, u8, &'static str)> {
        combat_grants(&self.combat)
    }

    /// The ledger to carry into the next launch.
    pub fn snapshot(&self) -> Option<LedgerSnapshot> {
        self.combat.story_snapshot()
    }

    pub fn hash(&self) -> u64 {
        self.combat.state_hash(&self.scene)
    }
}

/// The Boiler hall, walked by its scripted walk to the exit.
fn walk(
    run: &crate::f01_boiler::BoilerRun,
    mut scene: SceneSim,
    mut combat: CombatState,
) -> RoomRun {
    for tick in 1..=crate::f01_boiler::SMOKE_TICKS {
        let raw = run.fixture.input(tick);
        combat.step_party(&mut scene, [raw, RawInput::default()]);
    }
    RoomRun { scene, combat }
}

/// Room 1: the Hoarder's room under the story loadout (the **pre-Hoarder pin**,
/// Light on Your Feet held), her defeat by the real slot enemy at 1 HP: exactly one
/// dynamite grant, from the ledger's event trigger (K1). It replaces the Boiler-hall
/// stand-in that crossed the retired placeholder zone (PT-SCENE review U6).
pub fn hoarder_defeat(root: &Path) -> Result<RoomRun, String> {
    let (scene, combat) = crate::hoarder_fixtures::defeat_run(root, Loadout::Story)?;
    Ok(RoomRun { scene, combat })
}

/// The Boiler, relaunched with the ledger of the room before it.
pub fn boiler_relaunch(
    root: &Path,
    loadout: Loadout,
    carry: Option<&LedgerSnapshot>,
) -> Result<RoomRun, String> {
    let (carl, donut) = loadout.flags();
    let (run, scene, mut combat) = crate::f01_late::boiler_start(root, 360, carl, donut, None)?;
    if let Some(snapshot) = carry {
        combat.carry_story(snapshot).map_err(|e| e.to_string())?;
    }
    Ok(walk(&run, scene, combat))
}

const JUICER_BASE: Fixture = boss_base(
    "f01_late_juicer",
    "PT-SCENE: the Juicer with 4 HP left under a late-Floor-1 loadout; the rack crush defeats it",
    0x0F01_1A7E,
    180,
    hp(4),
    420,
);

/// The scripted Juicer kill of the room fixtures (the pilot plays Carl).
pub const JUICER_PARTY: PartyFixture = PartyFixture {
    base: &JUICER_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::CarlRack,
    donut: DonutSetup::unlocked(8),
};

/// The Juicer room launched through `--scene f01_late` (the boss scene under
/// the loadout's flags), the boss nearly dead, relaunched with `carry`.
pub fn juicer_relaunch(
    root: &Path,
    loadout: Loadout,
    carry: Option<&LedgerSnapshot>,
) -> Result<PartyFixtureSim, String> {
    let (carl, donut) = loadout.flags();
    let kit = load_arena(root)?;
    let scene = SceneSim::new_at(&kit, 360, JUICER_BASE.carl_x)?;
    let mut combat = CombatState::new(&JUICER_BASE, &kit, &scene)?
        .with_donut(DonutSetup::unlocked(8))
        .try_with_story(donut_prog::story_setup(carl, donut, SceneKind::Boss))
        .map_err(|e| e.to_string())?;
    if let Some(snapshot) = carry {
        combat.carry_story(snapshot).map_err(|e| e.to_string())?;
    }
    let mut sim = PartyFixtureSim::from_parts(scene, combat, &JUICER_PARTY);
    sim.run_scripted(JUICER_BASE.ticks);
    Ok(sim)
}

/// A walking hero that does not move for this many ticks is stuck: the gate
/// allows at most 300 (plan section 10, item 5).
pub const STUCK_LIMIT: u32 = 300;
/// The seeds every Juicer pilot covers (plan section 10, item 5).
pub const JUICER_SEEDS: std::ops::Range<u64> = 0..9;
/// The longest a Juicer pilot runs (the scene's 7200 ticks and the walk out).
pub const JUICER_RUN_TICKS: u64 = 9600;
/// The jab string period the base-kit pilot swings on (`boss_scene`); the
/// seed shifts the pilot's phase inside it (PTB-AD2).
pub const SWING_PERIOD: u64 = 12;

/// Who plays the Juicer's room: solo Carl, solo Donut by swap and the
/// autonomous companion (`--companion cf02_free`), gate item 5 (PTB-AD1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JuicerWho {
    /// Solo Carl fights and walks out (the other hero is the live party's
    /// follow-and-jump bot).
    CarlSolo,
    /// Carl clears the room, then the swap, then Donut (solo) walks to the
    /// exit (the PT-MASTERY shape, PTM-AD8; PTB-AD3).
    DonutBySwap,
    /// Carl is the pilot's, Donut is the autonomous `cf02_free` partner (her
    /// stuck run is counted as ticks left behind, as in PT-MASTERY).
    Companion,
}

impl JuicerWho {
    pub const ALL: [JuicerWho; 3] = [
        JuicerWho::CarlSolo,
        JuicerWho::DonutBySwap,
        JuicerWho::Companion,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::CarlSolo => "juicer_carl_solo",
            Self::DonutBySwap => "juicer_donut_by_swap",
            Self::Companion => "juicer_companion_cf02_free",
        }
    }
}

/// The live Juicer room: the scene's own fixture (full HP, racks on, Carl at
/// the entrance), the live party (K17: boss rules, no `cf02_rules`).
pub const JUICER_ROOM_PARTY: PartyFixture = PartyFixture {
    base: crate::boss_scene::BOSS01_SCENE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::Carl,
    donut: DonutSetup::unlocked(8),
};

/// The live Juicer room under `--companion cf02_free` (PTB-AD6): the same
/// party as [`JUICER_ROOM_PARTY`] with the autonomous partner. The room is not
/// a `cf02_rules` room (K17), so the 12-tick swap cooldown is not applied.
pub const JUICER_COMPANION_PARTY: PartyFixture = PartyFixture {
    tuning: PartyTuning {
        uncontrolled_hero: UncontrolledHero::Companion(ProfileSetId::Cf02Free),
        ..PROVISIONAL_TUNING
    },
    ..JUICER_ROOM_PARTY
};

/// The companion is counted left behind only while this far from Carl (the
/// PT-MASTERY value).
pub const APART_PX: f32 = 96.0;

/// The stuck rule of the gate: a hero told to walk whose column does not move
/// (less than 0.1 px) counts one tick; any other tick resets the run.
#[derive(Debug, Clone, Copy)]
struct StuckRun {
    last_x: f32,
    run: u32,
    max: u32,
}

impl StuckRun {
    fn new(x: f32) -> Self {
        Self {
            last_x: x,
            run: 0,
            max: 0,
        }
    }

    fn tick(&mut self, walking: bool, x: f32) {
        let still = (x - self.last_x).abs() < 0.1;
        self.run = if walking && still { self.run + 1 } else { 0 };
        self.max = self.max.max(self.run);
        self.last_x = x;
    }
}

/// What one Juicer pilot run saw.
#[derive(Debug, Clone)]
pub struct JuicerReport {
    pub who: JuicerWho,
    pub seed: u64,
    /// The tick of the Juicer's defeat (the exit opens).
    pub defeated_tick: Option<u64>,
    /// The tick the scene's exit was reached (`world.complete`) after the defeat.
    pub exit_tick: Option<u64>,
    /// The longest stuck run per hero: a hero told to walk whose column does
    /// not change, in consecutive ticks.
    pub max_stuck: [u32; 2],
    pub deaths: u32,
    pub hero_hurts: usize,
    pub hash: u64,
    pub ticks: u64,
    pub grants: Vec<(u64, u8, &'static str)>,
    /// Carl's column after every tick of the run.
    pub carl_trace: Vec<f32>,
    /// The width of Donut's column range over the run (px).
    pub donut_x_span: f32,
}

impl JuicerReport {
    /// The one line a table prints.
    pub fn line(&self) -> String {
        format!(
            "{} seed={} defeated={:?} exit={:?} stuck={:?} deaths={} hurts={} hash={:#018x}",
            self.who.name(),
            self.seed,
            self.defeated_tick,
            self.exit_tick,
            self.max_stuck,
            self.deaths,
            self.hero_hurts,
            self.hash
        )
    }
}

/// The Juicer room launched the way `--scene f01_late` launches it (the boss
/// scene under the story loadout, [`SceneKind::Boss`]), at full HP.
pub fn juicer_room(root: &Path) -> Result<PartyFixtureSim, String> {
    juicer_room_for(root, &JUICER_ROOM_PARTY)
}

/// [`juicer_room`] under a given party ([`JUICER_ROOM_PARTY`] or
/// [`JUICER_COMPANION_PARTY`]).
pub fn juicer_room_for(
    root: &Path,
    party: &'static PartyFixture,
) -> Result<PartyFixtureSim, String> {
    let (carl, donut) = Loadout::Story.flags();
    let kit = load_arena(root)?;
    let base = party.base;
    let scene = SceneSim::new_at(&kit, 360, base.carl_x)?;
    let combat = CombatState::new(base, &kit, &scene)?
        .with_donut(DonutSetup::unlocked(8))
        .try_with_story(donut_prog::story_setup(carl, donut, SceneKind::Boss))
        .map_err(|e| e.to_string())?;
    Ok(PartyFixtureSim::from_parts(scene, combat, party))
}

/// Play the Juicer's room for `who` at `seed`. The room reads no seed (it has
/// no roster and the boss draws nothing), so the seed shifts the base-kit
/// pilot's jab phase: nine seeds are nine different fights (PTB-AD2).
pub fn juicer_pilot(root: &Path, who: JuicerWho, seed: u64) -> Result<JuicerReport, String> {
    juicer_run(root, who, seed).map(|(report, _)| report)
}

/// [`juicer_pilot`] and the finished sim (at the exit tick), for a test that
/// reads the end state without re-driving the room.
pub fn juicer_run(
    root: &Path,
    who: JuicerWho,
    seed: u64,
) -> Result<(JuicerReport, PartyFixtureSim), String> {
    let mut sim = juicer_room_for(
        root,
        match who {
            JuicerWho::Companion => &JUICER_COMPANION_PARTY,
            _ => &JUICER_ROOM_PARTY,
        },
    )?;
    let mut carl_x = sim.scene.world.hero.x;
    let mut carl_trace = Vec::new();
    let mut donut_span = (sim.scene.world.donut_body.x, sim.scene.world.donut_body.x);
    let phase = seed % SWING_PERIOD;
    let mut stuck = [
        StuckRun::new(sim.scene.world.hero.x),
        StuckRun::new(sim.scene.world.donut_body.x),
    ];
    let mut defeated_tick = None;
    let mut exit_tick = None;
    let mut deaths = 0u32;
    let mut last_restarts = sim.combat.restarts;
    let mut swap_pressed = false;
    for _ in 0..JUICER_RUN_TICKS {
        let tick = sim.scene.world.tick + 1;
        let controlled = sim.party.controlled_hero();
        let released = sim.combat.boss.as_ref().is_some_and(|b| b.released());
        let mut input = if controlled == HeroId::Carl {
            crate::boss_scene::pilot_inputs(
                Pilot::Carl,
                [RawInput::NONE; 2],
                tick + phase,
                &sim.combat,
                &sim.scene,
            )[0]
        } else {
            // Donut (swapped in after the defeat) walks to the exit.
            RawInput {
                move_x: 1.0,
                ..RawInput::NONE
            }
        };
        if who == JuicerWho::DonutBySwap && released && !swap_pressed && controlled == HeroId::Carl
        {
            input.swap = true;
            swap_pressed = true;
        }
        let mut walking = [false; 2];
        let at = HeroId::ALL.iter().position(|h| *h == controlled);
        walking[at.unwrap_or(0)] = input.move_x != 0.0;
        sim.step_slots([input, RawInput::NONE]);
        let new_carl_x = sim.scene.world.hero.x;
        let carl_moved = (new_carl_x - carl_x).abs() >= 0.1;
        carl_x = new_carl_x;
        carl_trace.push(new_carl_x);
        let donut_x = sim.scene.world.donut_body.x;
        donut_span = (donut_span.0.min(donut_x), donut_span.1.max(donut_x));
        for (i, id) in HeroId::ALL.into_iter().enumerate() {
            let x = sim.scene.world.body(id).x;
            if who == JuicerWho::Companion && id == HeroId::Donut {
                // Left behind: she stands apart from Carl while he walks on
                // (her own walking is not the pilot's, as in PT-MASTERY).
                stuck[i].tick(carl_moved && (x - carl_x).abs() > APART_PX, x);
            } else {
                stuck[i].tick(walking[i], x);
            }
        }
        if sim.combat.restarts != last_restarts {
            deaths += 1;
            last_restarts = sim.combat.restarts;
        }
        let now_released = sim.combat.boss.as_ref().is_some_and(|b| b.released());
        if defeated_tick.is_none() && now_released {
            defeated_tick = Some(sim.scene.world.tick);
        }
        // The scene's own exit rule: a hero's box overlaps the exit rect, which
        // lies behind the right wall and opens with the defeat.
        if defeated_tick.is_some() && exit_tick.is_none() && sim.scene.world.complete {
            exit_tick = Some(sim.scene.world.tick);
        }
        if exit_tick.is_some() {
            break;
        }
    }
    let hero_hurts = sim
        .combat
        .events
        .iter()
        .filter(|e| matches!(e, CombatEvent::HeroHurt { .. }))
        .count();
    let report = JuicerReport {
        who,
        seed,
        defeated_tick,
        exit_tick,
        max_stuck: [stuck[0].max, stuck[1].max],
        deaths,
        hero_hurts,
        hash: sim.state_hash(),
        ticks: sim.scene.world.tick,
        grants: combat_grants(&sim.combat),
        carl_trace,
        donut_x_span: donut_span.1 - donut_span.0,
    };
    Ok((report, sim))
}

/// The Boiler's own combat constructor, for a test that drives it by hand.
pub fn boiler_state(
    root: &Path,
    loadout: Loadout,
) -> Result<(crate::f01_boiler::BoilerRun, SceneSim, CombatState), String> {
    let (carl, donut) = loadout.flags();
    let (run, scene, combat) = crate::f01_late::boiler_start(root, 360, carl, donut, None)?;
    // Same state the verifier rebuilds.
    debug_assert_eq!(
        {
            // The env solids are already in `scene`: the sync is idempotent.
            let mut again = scene.clone();
            boiler_combat(&run, &mut again, carl, donut, None)?.state_hash(&again)
        },
        combat.state_hash(&scene)
    );
    Ok((run, scene, combat))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prototype::HERO_HALF_WIDTH;
    use crate::story_unlock::{FIST_REINFORCED, SPECIAL_THROWN_CHARGE};
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn held(combat: &CombatState) -> Vec<(u8, &'static str)> {
        combat
            .story_ledger()
            .expect("a story runtime")
            .held()
            .iter()
            .map(|(hero, id)| (*hero, id.as_str()))
            .collect()
    }

    fn defeat_tick(sim: &PartyFixtureSim) -> u64 {
        sim.combat
            .events
            .iter()
            .find_map(|e| match *e {
                CombatEvent::EnemyDefeated { tick, .. } => Some(tick),
                _ => None,
            })
            .expect("the Juicer is defeated")
    }

    /// The committed final hashes (the story loadout run of each room).
    /// Re-pinned by BOIL-B/C wire: the shared Boiler walk jumps the drum at tick 229.
    const GOLDEN_HOARDER: u64 = 0x3206_266a_728e_229f;
    /// Re-pinned by BOIL-B/C wire: the Boiler now runs its env step and the jump.
    /// Re-pinned by BOIL-D: the drum's blast charge joins the Boiler's state hash
    /// (the walk touches no prop, so nothing else moved).
    const GOLDEN_BOILER: u64 = 0xd76e_2a86_49e8_48be;
    const GOLDEN_JUICER: u64 = 0x60d4_470f_f8d7_4c3b;

    #[test]
    fn the_hoarders_dynamite_is_granted_exactly_once() {
        let run = hoarder_defeat(&root()).unwrap();
        let grants = run.grants();
        assert_eq!(grants.len(), 1, "one grant moment: {grants:?}");
        assert_eq!((grants[0].1, grants[0].2), (0, "special_thrown_charge"));
        // The pin held Light on Your Feet from the start; the grant added dynamite.
        assert_eq!(
            held(&run.combat),
            [(0, "special_thrown_charge"), (1, "jump_reach_boost")]
        );
        assert!(run.combat.story_holds(SPECIAL_THROWN_CHARGE));
        // Without the pin the grant would not be the only change: the pre pin
        // holds no dynamite before her defeat (checked at tick 0).
        let (carl, donut) = Loadout::Story.flags();
        let setup = donut_prog::story_setup_pin(carl, donut, SceneKind::CfArena, PRE_PIN);
        let ids: Vec<&str> = setup.holds().iter().map(|c| c.as_str()).collect();
        assert_eq!(ids, ["jump_reach_boost"]);
    }

    #[test]
    fn the_ledger_survives_two_relaunches_retries_and_respawns() {
        let hoarder = hoarder_defeat(&root()).unwrap();
        let snapshot = hoarder.snapshot().expect("a ledger");
        // Relaunch 1: the Boiler under the story loadout, carried. Its post pin
        // already holds dynamite, so nothing is granted again, and the receipt of
        // the earlier grant came along.
        let mut boiler = boiler_relaunch(&root(), Loadout::Story, Some(&snapshot)).unwrap();
        assert!(
            boiler.grants().is_empty(),
            "no second grant: {:?}",
            boiler.grants()
        );
        assert_eq!(held(&boiler.combat), held(&hoarder.combat));
        let carried = boiler.snapshot().unwrap();
        assert_eq!(
            carried.receipts.len(),
            1,
            "the Hoarder's receipt came along"
        );
        // A checkpoint respawn keeps the ledger; so does a fixture restart (the
        // carried ledger is the relaunched room's declared start state).
        boiler.combat.respawn(&mut boiler.scene);
        let _ = boiler.combat.take_respawned();
        assert_eq!(held(&boiler.combat), held(&hoarder.combat));
        boiler.combat.restart(&mut boiler.scene);
        assert_eq!(held(&boiler.combat), held(&hoarder.combat));
        // Relaunch 2: the Juicer, carried from the Boiler: the defeat grants the
        // gauntlet and the blink once each, in list order, on one tick.
        let mut juicer = juicer_relaunch(&root(), Loadout::Story, Some(&carried)).unwrap();
        let defeat = defeat_tick(&juicer);
        assert_eq!(
            grants_of(&juicer),
            [
                (defeat, 0, "fist_reinforced"),
                (defeat, 1, "special_short_blink")
            ]
        );
        assert!(juicer.combat.story_holds(FIST_REINFORCED));
        assert!(
            juicer.combat.story_holds(SPECIAL_THROWN_CHARGE),
            "dynamite carried"
        );
        // A defeat retry keeps the grants and never grants them again.
        let before = held(&juicer.combat);
        juicer.combat.respawn(&mut juicer.scene);
        assert_eq!(held(&juicer.combat), before);
    }

    fn grants_of(sim: &PartyFixtureSim) -> Vec<(u64, u8, &'static str)> {
        sim.combat
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

    #[test]
    fn a_carry_into_a_room_without_the_story_flags_is_refused() {
        let hoarder = hoarder_defeat(&root()).unwrap();
        let snapshot = hoarder.snapshot().unwrap();
        let run = crate::f01_boiler::launch(&root()).unwrap();
        let scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
        let mut plain = CombatState::new(run.fixture, &run.kit, &scene).unwrap();
        assert!(
            plain.carry_story(&snapshot).is_err(),
            "no runtime to hold it"
        );
        assert!(plain.story_snapshot().is_none());
        // An empty snapshot is always fine.
        let empty = LedgerSnapshot {
            version: crate::story_unlock::SNAPSHOT_VERSION,
            held: vec![],
            receipts: vec![],
        };
        assert!(plain.carry_story(&empty).is_ok());
    }

    #[test]
    fn a_snapshot_the_table_does_not_know_is_refused() {
        let run = crate::f01_boiler::launch(&root()).unwrap();
        let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
        let (carl, donut) = Loadout::Story.flags();
        let mut combat = boiler_combat(&run, &mut scene, carl, donut, None).unwrap();
        let bad = LedgerSnapshot {
            version: crate::story_unlock::SNAPSHOT_VERSION,
            held: vec![crate::story_unlock::SnapshotHeld {
                hero: 0,
                capability: "no_such_row".into(),
            }],
            receipts: vec![],
        };
        assert!(combat.carry_story(&bad).is_err());
    }

    #[test]
    fn the_juicer_grants_exactly_the_gauntlet_and_the_blink_under_the_story_loadout() {
        let sim = juicer_relaunch(&root(), Loadout::Story, None).unwrap();
        let defeat = defeat_tick(&sim);
        assert_eq!(
            grants_of(&sim),
            [
                (defeat, 0, "fist_reinforced"),
                (defeat, 1, "special_short_blink")
            ]
        );
    }

    // ---- PT-TUNE section 4: the Juicer is a full-kit fight only under
    // `--f01-loadout all` or after its own defeat, never in the story. No code
    // changed: these tests pin the rule that already holds.

    /// The Juicer room as launched (nothing stepped), under `loadout`.
    fn juicer_launch(loadout: Loadout, carry: Option<&LedgerSnapshot>) -> (SceneSim, CombatState) {
        let (carl, donut) = loadout.flags();
        let kit = load_arena(&root()).unwrap();
        let scene = SceneSim::new_at(&kit, 360, JUICER_BASE.carl_x).unwrap();
        let mut combat = CombatState::new(&JUICER_BASE, &kit, &scene)
            .unwrap()
            .with_donut(DonutSetup::unlocked(8))
            .try_with_story(donut_prog::story_setup(carl, donut, SceneKind::Boss))
            .unwrap();
        if let Some(snapshot) = carry {
            combat.carry_story(snapshot).unwrap();
        }
        (scene, combat)
    }

    fn late_kit(combat: &CombatState) -> (bool, bool) {
        (
            combat.story_holds(FIST_REINFORCED),
            combat.story_holds(crate::story_unlock::CapabilityId::new(
                "special_short_blink",
            )),
        )
    }

    /// Pin 1: a story Juicer start with no carried ledger holds neither late
    /// grant on tick 0 (the dynamite and the boost are its pin).
    #[test]
    fn a_story_juicer_start_with_no_carried_ledger_holds_neither_late_grant() {
        let (_scene, combat) = juicer_launch(Loadout::Story, None);
        assert_eq!(late_kit(&combat), (false, false));
        assert!(
            combat.story_holds(SPECIAL_THROWN_CHARGE),
            "the pin's dynamite"
        );
        assert!(combat.events.is_empty(), "tick 0: no grant has fired");
    }

    /// Pin 2: after a hero defeat and a fixture restart the boss is present
    /// again and neither grant is held (a restart restores the declared pin).
    #[test]
    fn a_fixture_restart_after_the_defeat_brings_the_boss_back_without_the_grants() {
        let mut sim = juicer_relaunch(&root(), Loadout::Story, None).unwrap();
        assert!(!sim.combat.boss.as_ref().unwrap().alive(), "defeated");
        assert_eq!(late_kit(&sim.combat), (true, true), "granted at the defeat");
        sim.combat.restart(&mut sim.scene);
        assert!(
            sim.combat.boss.as_ref().unwrap().alive(),
            "the boss is back"
        );
        assert_eq!(late_kit(&sim.combat), (false, false));
    }

    /// Pin 3: the only story state that holds the full kit is the carried
    /// post-defeat ledger, and in that state the Juicer is already defeated (its
    /// defeat is the grant), so no story run fights a full-kit Juicer.
    #[test]
    fn the_only_full_kit_story_state_is_the_post_defeat_ledger() {
        let defeated = juicer_relaunch(&root(), Loadout::Story, None).unwrap();
        let ledger = defeated.combat.story_snapshot().expect("a ledger");
        assert_eq!(late_kit(&defeated.combat), (true, true));
        assert!(!defeated.combat.boss.as_ref().unwrap().alive());
        // Carrying that ledger into a fresh Juicer launch gives the full kit...
        let (_scene, carried) = juicer_launch(Loadout::Story, Some(&ledger));
        assert_eq!(late_kit(&carried), (true, true));
        // ...but a launch without it never does, and no launch under the story
        // loadout holds the late grants before the defeat event.
        let (_scene, fresh) = juicer_launch(Loadout::Story, None);
        assert_eq!(late_kit(&fresh), (false, false));
        // The all loadout is the test loadout: held from tick 0, story triggers off.
        let (_scene, all) = juicer_launch(Loadout::All, None);
        assert_eq!(late_kit(&all), (true, true));
    }

    #[test]
    fn no_grant_fires_under_the_all_loadout() {
        let sim = juicer_relaunch(&root(), Loadout::All, None).unwrap();
        let _ = defeat_tick(&sim);
        assert!(grants_of(&sim).is_empty(), "{:?}", grants_of(&sim));
        let boiler = boiler_relaunch(&root(), Loadout::All, None).unwrap();
        assert!(boiler.grants().is_empty());
        // Everything is held from tick 0.
        assert_eq!(held(&boiler.combat).len(), 4);
    }

    #[test]
    fn the_runs_have_committed_hashes_and_replay_twice() {
        type Run = Box<dyn Fn() -> u64>;
        let runs: [(&str, Run, u64); 3] = [
            (
                "hoarder",
                Box::new(|| hoarder_defeat(&root()).unwrap().hash()),
                GOLDEN_HOARDER,
            ),
            (
                "boiler",
                Box::new(|| {
                    boiler_relaunch(&root(), Loadout::Story, None)
                        .unwrap()
                        .hash()
                }),
                GOLDEN_BOILER,
            ),
            (
                "juicer",
                Box::new(|| {
                    juicer_relaunch(&root(), Loadout::Story, None)
                        .unwrap()
                        .state_hash()
                }),
                GOLDEN_JUICER,
            ),
        ];
        let mut wrong = Vec::new();
        for (name, run, golden) in runs {
            let first = run();
            assert_eq!(first, run(), "{name} replays identically");
            if first != golden {
                wrong.push(format!("{name}: {first:#018x}"));
            }
        }
        assert!(wrong.is_empty(), "{wrong:?}");
    }

    #[test]
    fn the_boiler_state_helper_matches_the_verifier_constructor() {
        let (_, scene, combat) = boiler_state(&root(), Loadout::Story).unwrap();
        assert_eq!(combat.state_hash(&scene), {
            let (carl, donut) = Loadout::Story.flags();
            let (_, scene, combat) =
                crate::f01_late::boiler_start(&root(), 360, carl, donut, None).unwrap();
            combat.state_hash(&scene)
        });
    }

    /// The final state hash of every Juicer pilot run, `[who][seed]`, in
    /// [`JuicerWho::ALL`] and [`JUICER_SEEDS`] order (PT-SCENE-B, new pins). A
    /// change to the room, the boss, the pilot or the story pin is a
    /// deliberate re-pin.
    const GOLDEN_JUICER_PILOTS: [[u64; 9]; 3] = [
        [
            0xbb42_7d22_87b2_7d30,
            0x7a95_75a6_324b_77aa,
            0xd622_b320_2149_dfb5,
            0x370b_5b09_b01b_815a,
            0x8766_22f1_173d_6cb2,
            0x42af_2a74_9ed9_418c,
            0x4c86_2daf_b3b2_f434,
            0x2d9a_a564_6fee_8444,
            0x9290_e851_2559_467a,
        ],
        [
            0xc3bc_a497_bccc_674f,
            0xa238_d8d6_131a_7d59,
            0xdd32_5a2d_8c8f_7f55,
            0x78df_5f9a_581a_df89,
            0x0170_69a3_9fb8_496e,
            0x441d_1a4e_edf8_c702,
            0xb154_9621_d1f2_f52d,
            0x2519_4eb7_7492_d5bd,
            0x0797_9362_3de8_7c42,
        ],
        [
            0xc3fa_1e52_5b9a_c89f,
            0xddc2_cb43_beab_800c,
            0x0b36_9bec_87fa_d147,
            0xf96f_5c4e_bb37_faba,
            0xc6ec_844c_3f87_6372,
            0xccff_08ab_1e64_e2ac,
            0x51ad_2163_d8d3_47e7,
            0xb98d_c029_31e8_14bc,
            0x6864_b594_d7db_dbe6,
        ],
    ];

    fn juicer_reports() -> Vec<JuicerReport> {
        let mut all = Vec::new();
        for who in JuicerWho::ALL {
            for seed in JUICER_SEEDS {
                all.push(juicer_pilot(&root(), who, seed).unwrap());
            }
        }
        all
    }

    #[test]
    fn every_juicer_pilot_replays_twice_and_no_hero_is_stuck_for_more_than_300_ticks() {
        for who in JuicerWho::ALL {
            for seed in JUICER_SEEDS {
                let a = juicer_pilot(&root(), who, seed).unwrap();
                let b = juicer_pilot(&root(), who, seed).unwrap();
                assert_eq!(a.hash, b.hash, "{}", a.line());
                assert_eq!(a.line(), b.line(), "replays identically");
                assert!(
                    a.max_stuck.iter().all(|s| *s <= STUCK_LIMIT),
                    "stuck: {}",
                    a.line()
                );
            }
        }
    }

    #[test]
    fn the_juicer_falls_and_the_exit_is_reached_for_every_pilot_at_every_seed() {
        for r in juicer_reports() {
            let defeated = r.defeated_tick.unwrap_or_else(|| panic!("{}", r.line()));
            let exit = r.exit_tick.unwrap_or_else(|| panic!("{}", r.line()));
            assert!(
                exit > defeated,
                "the exit opens with the defeat: {}",
                r.line()
            );
            assert!(defeated <= JUICER_ROOM_PARTY.base.ticks, "{}", r.line());
            assert_eq!(r.deaths, 0, "one attempt: {}", r.line());
            // The story loadout: the gauntlet and the blink, once each, at the
            // defeat tick, and nothing else.
            assert_eq!(
                r.grants,
                [
                    (defeated, 0, "fist_reinforced"),
                    (defeated, 1, "special_short_blink")
                ],
                "{}",
                r.line()
            );
        }
    }

    #[test]
    fn the_swapped_donut_is_the_hero_who_reaches_the_exit() {
        let exit_x = load_arena(&root()).unwrap().exit.unwrap().x as f32;
        for seed in JUICER_SEEDS {
            let (r, sim) = juicer_run(&root(), JuicerWho::DonutBySwap, seed).unwrap();
            assert!(r.exit_tick.expect("exit reached") > r.defeated_tick.unwrap());
            // Donut is the walker after the swap: her stuck run is the bounded one.
            assert!(r.max_stuck[1] <= STUCK_LIMIT, "{}", r.line());
            // The run stops on the exit tick: read who stands in it.
            assert!(sim.scene.world.complete);
            assert_eq!(sim.party.controlled_hero(), HeroId::Donut);
            assert!(sim.scene.world.donut_body.x + HERO_HALF_WIDTH > exit_x);
        }
    }

    /// The boundary of the companion runs (PTB-AD7): under a boss `cf02_free`
    /// hunts nothing (CA2-Q35, until COMPANION-AI-01E), so Donut escorts and
    /// idles while Carl's fight is tick-identical to solo Carl's. When 01E gives
    /// her boss behaviour, this test is the named, deliberate change.
    #[test]
    fn the_companion_escorts_but_takes_no_part_in_the_juicer_fight() {
        for seed in JUICER_SEEDS {
            let (solo, _) = juicer_run(&root(), JuicerWho::CarlSolo, seed).unwrap();
            let (comp, sim) = juicer_run(&root(), JuicerWho::Companion, seed).unwrap();
            assert_eq!(comp.defeated_tick, solo.defeated_tick, "seed {seed}");
            assert_eq!(comp.exit_tick, solo.exit_tick, "seed {seed}");
            assert_eq!(
                comp.carl_trace, solo.carl_trace,
                "Carl's run is solo Carl's"
            );
            // She is present and decides and walks...
            let donut = sim.combat.autonomy_state(HeroId::Donut).stats();
            assert!(donut.decisions > 0, "seed {seed}: her autonomy runs");
            assert!(comp.donut_x_span > 100.0, "seed {seed}: she walks");
            assert_ne!(comp.hash, solo.hash, "her block is part of the state");
            // ...but starts no move, lands no hit and hunts nothing.
            let stats = sim.combat.companion_stats();
            assert_eq!(stats.assist_starts[1], 0, "seed {seed}");
            assert_eq!(stats.interrupt_starts[1], 0, "seed {seed}");
            assert_eq!(stats.chain_presses[1], 0, "seed {seed}");
            assert_eq!(donut.hunt_evaluations, 0, "seed {seed}");
            assert!(
                !sim.combat
                    .events
                    .iter()
                    .any(|e| matches!(e, CombatEvent::DonutGlance { .. })),
                "seed {seed}: no Donut hit on the boss"
            );
        }
    }

    #[test]
    fn the_seed_shifts_the_fight_and_no_two_seeds_share_a_hash() {
        for who in JuicerWho::ALL {
            let mut hashes: Vec<u64> = JUICER_SEEDS
                .map(|seed| juicer_pilot(&root(), who, seed).unwrap().hash)
                .collect();
            hashes.sort_unstable();
            hashes.dedup();
            assert_eq!(hashes.len(), 9, "{}", who.name());
        }
    }

    #[test]
    fn the_juicer_pilot_hashes_are_pinned() {
        let mut wrong = Vec::new();
        for (w, who) in JuicerWho::ALL.into_iter().enumerate() {
            for (s, seed) in JUICER_SEEDS.enumerate() {
                let hash = juicer_pilot(&root(), who, seed).unwrap().hash;
                if hash != GOLDEN_JUICER_PILOTS[w][s] {
                    wrong.push(format!("{} seed {seed}: {hash:#018x}", who.name()));
                }
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    #[test]
    fn the_stuck_rule_counts_a_walker_that_does_not_move_and_resets_on_progress() {
        let mut run = StuckRun::new(10.0);
        for _ in 0..301 {
            run.tick(true, 10.0);
        }
        assert_eq!(run.max, 301, "a walker against a wall exceeds the limit");
        run.tick(true, 11.0);
        assert_eq!(run.run, 0, "any progress resets the run");
        for _ in 0..500 {
            run.tick(false, 11.0);
        }
        assert_eq!(run.max, 301, "standing still is not walking");
        // The closed right wall really does stop a hero pressing right.
        let mut sim = juicer_room(&root()).unwrap();
        let mut run = StuckRun::new(sim.scene.world.hero.x);
        for _ in 0..1500 {
            let right = RawInput {
                move_x: 1.0,
                ..RawInput::NONE
            };
            sim.step_slots([right, RawInput::NONE]);
            run.tick(true, sim.scene.world.hero.x);
        }
        assert!(
            run.max > STUCK_LIMIT,
            "a walker held at the wall or boss is counted"
        );
    }
}
