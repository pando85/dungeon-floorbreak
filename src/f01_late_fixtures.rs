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
//! | [`hoarder_stand_in`] | the Boiler hall as a stand-in for the Hoarder's lair (HRD-A2 has not landed) | the pre-Hoarder pin and the placeholder zone: exactly one dynamite grant, K1 |
//! | [`boiler_relaunch`] | the Boiler, carried | the carried ledger, no second grant |
//! | [`juicer_relaunch`] | the Juicer, carried | the gauntlet and the blink exactly once at the defeat, and nothing under `all` |

use std::path::Path;

use crate::boss_scene::{Pilot, boss_base, hp, load_arena};
use crate::combat::CombatEvent;
use crate::combat_scene::{CombatState, Fixture};
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::donut_prog;
use crate::f01_late::{Loadout, boiler_combat};
use crate::input::RawInput;
use crate::party::PROVISIONAL_TUNING;
use crate::party_scene::{PartyFixture, PartyFixtureSim};
use crate::story_unlock::{LedgerSnapshot, SceneKind};

/// The pre-Hoarder pin and the placeholder zone trigger the stand-in uses.
pub const PRE_PIN: &str = "f01_pin_pre_pile_boss";
pub const HOARDER_TRIGGER: &str = "pile_boss_f01_defeated";

/// A finished room run: the final state and what it granted.
pub struct RoomRun {
    pub scene: SceneSim,
    pub combat: CombatState,
}

impl RoomRun {
    /// Every `CapabilityGranted` of the run: `(tick, hero, capability)`.
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

/// Room 1 as a stand-in: the Boiler hall under the **pre-Hoarder pin**, with the
/// placeholder Hoarder-defeat zone armed by id (the Hoarder does not exist yet,
/// K1). Carl's walk crosses the zone: one dynamite grant, and the pin held
/// Light on Your Feet from the start (no grant for it here).
pub fn hoarder_stand_in(root: &Path) -> Result<RoomRun, String> {
    let (carl, donut) = Loadout::Story.flags();
    let run = crate::f01_boiler::launch(root)?;
    let scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x)?;
    let setup = donut_prog::story_setup_pin(carl, donut, SceneKind::CfArena, PRE_PIN)
        .with_trigger(HOARDER_TRIGGER);
    let combat = CombatState::new(run.fixture, &run.kit, &scene)?
        .with_donut(DonutSetup::unlocked(8))
        .try_with_story(setup)
        .map_err(|e| e.to_string())?
        .with_demo(run.rules)
        .with_cf02()
        .with_cf02_seed(None);
    Ok(walk(&run, scene, combat))
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
    const GOLDEN_STAND_IN: u64 = 0x7502_9aea_159b_bbdd;
    /// Re-pinned by BOIL-B/C wire: the Boiler now runs its env step and the jump.
    const GOLDEN_BOILER: u64 = 0x6a07_7143_d03b_997e;
    const GOLDEN_JUICER: u64 = 0x60d4_470f_f8d7_4c3b;

    #[test]
    fn the_hoarders_dynamite_is_granted_exactly_once() {
        let run = hoarder_stand_in(&root()).unwrap();
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
        // holds no dynamite before the zone (checked at tick 0).
        let (carl, donut) = Loadout::Story.flags();
        let setup = donut_prog::story_setup_pin(carl, donut, SceneKind::CfArena, PRE_PIN);
        let ids: Vec<&str> = setup.holds().iter().map(|c| c.as_str()).collect();
        assert_eq!(ids, ["jump_reach_boost"]);
    }

    #[test]
    fn the_ledger_survives_two_relaunches_retries_and_respawns() {
        let stand_in = hoarder_stand_in(&root()).unwrap();
        let snapshot = stand_in.snapshot().expect("a ledger");
        // Relaunch 1: the Boiler under the story loadout, carried. Its post pin
        // already holds dynamite, so nothing is granted again, and the receipt of
        // the earlier grant came along.
        let mut boiler = boiler_relaunch(&root(), Loadout::Story, Some(&snapshot)).unwrap();
        assert!(
            boiler.grants().is_empty(),
            "no second grant: {:?}",
            boiler.grants()
        );
        assert_eq!(held(&boiler.combat), held(&stand_in.combat));
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
        assert_eq!(held(&boiler.combat), held(&stand_in.combat));
        boiler.combat.restart(&mut boiler.scene);
        assert_eq!(held(&boiler.combat), held(&stand_in.combat));
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
        let stand_in = hoarder_stand_in(&root()).unwrap();
        let snapshot = stand_in.snapshot().unwrap();
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
                "stand_in",
                Box::new(|| hoarder_stand_in(&root()).unwrap().hash()),
                GOLDEN_STAND_IN,
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
}
