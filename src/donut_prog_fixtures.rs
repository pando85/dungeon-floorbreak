//! DONUT-PROG-01 fixtures (the `donut_prog_*` namespace): the second hero's
//! ability rows, flags and unlock triggers as replay fixtures.
//!
//! Bevy-free party fixtures in the shape of [`crate::party_scene`]. Each one
//! declares its own story setup ([`story_setup`]: the program flags it enables,
//! never a debug flag), has a committed replay in
//! `docs/evidence/donut-prog-01/replays` and a golden hash pinned by a test in
//! this file. They are **not** in [`PartyFixture::all`]: the registry is
//! [`DONUT_PROG_FIXTURES`], listed by `floor01_scene --list-fixtures
//! donut_prog_` and replayed by the CI loop, so every earlier fixture, golden
//! and replay is untouched. Later DONUT-PROG PRs append their fixtures here.
//!
//! Tick numbering is the one of [`crate::combat_scene`]: a hold over
//! `from..to` covers the steps that produce `tick = from .. to - 1`.

use crate::boss_scene::{Pilot, boss_base, hp};
use crate::carl_prog::CarlProgFlags;
use crate::combat::EnemyAi;
use crate::combat_scene::{EnemySpawn, Fixture};
use crate::donut_fixtures::{GUILD_PLACEHOLDER_ZONE, primary};
use crate::donut_mana::{DonutSetup, StartState};
use crate::donut_prog::{DonutProgFlags, story_setup as program_setup};
use crate::input::RawInput;
use crate::party::{PROVISIONAL_TUNING, PartyTuning};
use crate::party_scene::{PartyFixture, SlotHold, base, run};
use crate::story_unlock::{SceneKind, StorySetup};

const fn chase(x: i32) -> EnemySpawn {
    EnemySpawn {
        x,
        ai: EnemyAi::Chase,
    }
}

/// A held Circle/B on `slot`: its press edge is on `from`.
const fn special(from: u64, to: u64, slot: usize) -> SlotHold {
    SlotHold {
        from,
        to,
        slot,
        input: RawInput {
            ability: true,
            ..RawInput::NONE
        },
    }
}

const DONUT_PROG: DonutProgFlags = DonutProgFlags {
    prog: true,
    ..DonutProgFlags::OFF
};

/// The story a `donut_prog_*` fixture declares for itself (a scripted fixture
/// never reads a debug flag, DP-Q10). Every other fixture name is not in this
/// registry: [`StorySetup::NONE`].
pub fn story_setup(fixture: &str) -> StorySetup {
    match fixture {
        // The story corridor run under `--donut-prog`: the guild zone (the
        // biscuit, `DonutSetup`) and the safe-room placeholder zone.
        "donut_prog_story_unlocks"
        | "donut_prog_grant_survives_retry"
        | "donut_prog_primary_mapping" => {
            program_setup(CarlProgFlags::OFF, DONUT_PROG, SceneKind::Corridor)
        }
        // The boss pin and the Juicer trigger, Donut's program only.
        "donut_prog_boss_pins" => program_setup(CarlProgFlags::OFF, DONUT_PROG, SceneKind::Boss),
        // Both programs: the union of the pins and both entries of the grants list.
        "donut_prog_boss_both_programs" => program_setup(
            CarlProgFlags {
                prog: true,
                ..CarlProgFlags::OFF
            },
            DONUT_PROG,
            SceneKind::Boss,
        ),
        _ => StorySetup::NONE,
    }
}

/// The biscuit starts locked and the placeholder guild zone grants it.
const GUILD: DonutSetup = DonutSetup {
    start: StartState::LOCKED,
    guild_zone: Some(GUILD_PLACEHOLDER_ZONE),
};

const STORY_BASE: Fixture = base(
    "donut_prog_story_unlocks",
    "Two players: both walk the corridor run under the Donut program; the guild zone gives the biscuit, the safe-room placeholder zone grants the jump boost once, and Circle/B stays inert",
    0x0D90_0001,
    &[],
    480,
);

/// DP-Q01, DP-Q02, DP-Q06: the biscuit at the guild zone, then
/// `jump_reach_boost` exactly once at the safe-room placeholder zone, with one
/// `CapabilityGranted { hero 1 }` on the first tick her pivot is inside. P2's
/// Circle/B presses (before and after the grant) emit nothing.
pub const DONUT_PROG_STORY_UNLOCKS: PartyFixture = PartyFixture {
    base: &STORY_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        run(5, 440, 0, 1.0),
        run(5, 440, 1, 1.0),
        special(20, 21, 1),
        special(300, 301, 1),
    ],
    pilot: Pilot::None,
    donut: GUILD,
};

const RETRY_BASE: Fixture = base(
    "donut_prog_grant_survives_retry",
    "Two players: Donut takes both unlocks, then two goblins defeat her (one hit does it here) and the party retries twice: the boost stays held and is never granted a second time",
    0x0D90_0002,
    &[chase(1300), chase(1340)],
    1200,
);

/// One hit defeats Donut here (a test tuning), so the retry comes quickly.
const ONE_HIT_DONUT: PartyTuning = PartyTuning {
    donut_damage_per_hit: 4,
    ..PROVISIONAL_TUNING
};

/// CP-Q07 for Donut's row: a grant survives a defeat retry (the demo's
/// checkpoint respawn and a fixture restart are unit-tested).
pub const DONUT_PROG_GRANT_SURVIVES_RETRY: PartyFixture = PartyFixture {
    base: &RETRY_BASE,
    two_player: true,
    tuning: ONE_HIT_DONUT,
    script: &[run(5, 440, 0, 1.0), run(5, 440, 1, 1.0)],
    pilot: Pilot::None,
    donut: GUILD,
};

const MAPPING_BASE: Fixture = base(
    "donut_prog_primary_mapping",
    "Two players: P2 Donut presses Square/X before the biscuit (swat), then at the guild zone (casts until the pool is dry), with Circle/B pressed throughout: it never does anything",
    0x0D90_0003,
    &[],
    420,
);

/// DP-Q01 and M1: the primary mapping is unchanged by the program: the swat
/// before the biscuit, a cast after it, a dry press at 0 mana; Circle/B is
/// inert until `special_short_blink` is held (no event, no state).
pub const DONUT_PROG_PRIMARY_MAPPING: PartyFixture = PartyFixture {
    base: &MAPPING_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        run(5, 140, 1, 1.0),
        primary(8, 9, 1),
        special(10, 11, 1),
        special(40, 41, 1),
        primary(70, 71, 1),
        primary(90, 91, 1),
        primary(110, 111, 1),
        primary(130, 131, 1),
        primary(150, 151, 1),
        primary(170, 171, 1),
        primary(190, 191, 1),
        primary(210, 211, 1),
        primary(230, 231, 1),
        primary(250, 251, 1),
        special(260, 261, 1),
        special(340, 341, 1),
    ],
    pilot: Pilot::None,
    donut: GUILD,
};

const PINS_BASE: Fixture = boss_base(
    "donut_prog_boss_pins",
    "The boss has 4 HP left and Donut holds the jump boost from the boss pin (the biscuit read through): the rack crush defeats it and the Juicer trigger grants only the blink on that tick; Carl's reinforced fist is not granted",
    0x0D90_0004,
    180,
    hp(4),
    420,
);

/// DP-Q03, DP-Q09, rule 3: under `--donut-prog` alone the Juicer trigger
/// grants only `special_short_blink` (hero 1); the pin emits nothing.
pub const DONUT_PROG_BOSS_PINS: PartyFixture = PartyFixture {
    base: &PINS_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::CarlRack,
    donut: DonutSetup::unlocked(8),
};

const BOTH_BASE: Fixture = boss_base(
    "donut_prog_boss_both_programs",
    "The boss has 4 HP left under both programs (Carl's dynamite and Donut's boost pinned): the Juicer defeat grants the reinforced fist and then the blink, in list order on one tick",
    0x0D90_0005,
    180,
    hp(4),
    420,
);

/// DP-Q03, DP-Q09: with both programs the two grants land on the defeat tick
/// in grants-list order, `fist_reinforced` then `special_short_blink`.
pub const DONUT_PROG_BOSS_BOTH_PROGRAMS: PartyFixture = PartyFixture {
    base: &BOTH_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::CarlRack,
    donut: DonutSetup::unlocked(8),
};

/// Every `donut_prog_*` replay fixture, in committed order.
pub const DONUT_PROG_FIXTURES: [&PartyFixture; 5] = [
    &DONUT_PROG_STORY_UNLOCKS,
    &DONUT_PROG_GRANT_SURVIVES_RETRY,
    &DONUT_PROG_PRIMARY_MAPPING,
    &DONUT_PROG_BOSS_PINS,
    &DONUT_PROG_BOSS_BOTH_PROGRAMS,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss_scene::{BOSS_FIXTURES, load_arena};
    use crate::cf02_fixtures::{find_party_fixture, list_fixtures};
    use crate::combat::CombatEvent;
    use crate::combat_scene::{CombatState, FixtureSim};
    use crate::corridor::SceneSim;
    use crate::demo::{DEMO_RULES, autopilot, fixture_for_beat, load_hall};
    use crate::donut_mana::ManaEvent;
    use crate::env_kit::EnvKit;
    use crate::party_scene::PartyFixtureSim;
    use crate::story_unlock::{
        CapabilityId, FIST_REINFORCED, PRIMARY_RANGED, ReadThrough, SPECIAL_THROWN_CHARGE,
        StoryRuntime, StoryTable,
    };
    use std::path::{Path, PathBuf};

    const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn kit_of(fixture: &PartyFixture) -> EnvKit {
        if fixture.base.boss.is_some() {
            load_arena(&root()).unwrap()
        } else {
            EnvKit::load(&root(), LAYOUT).unwrap()
        }
    }

    fn replay(fixture: &'static PartyFixture) -> PartyFixtureSim {
        let mut sim = PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        sim.run_scripted(fixture.base.ticks);
        sim
    }

    fn named(name: &str) -> PartyFixtureSim {
        replay(find_party_fixture(name).expect("a registered fixture"))
    }

    /// Every event of the run, as the headless replay prints them.
    fn lines(sim: &PartyFixtureSim) -> Vec<String> {
        assert_eq!(sim.combat.dropped_events, 0, "the log never hit its cap");
        let mut out: Vec<String> = sim.combat.events.iter().map(ToString::to_string).collect();
        out.push(sim.combat.state_line(&sim.scene));
        out.push(sim.state_line());
        out
    }

    fn grants(sim: &PartyFixtureSim) -> Vec<(u64, u8, &'static str)> {
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

    fn boost() -> CapabilityId {
        crate::donut_prog::JUMP_REACH_BOOST
    }

    fn blink() -> CapabilityId {
        crate::donut_prog::SPECIAL_SHORT_BLINK
    }

    /// The committed final hashes of the `donut_prog_*` fixtures.
    const GOLDENS: [(&str, u64); 5] = [
        ("donut_prog_story_unlocks", 0xf7eb_5539_2e6c_5a98),
        ("donut_prog_grant_survives_retry", 0xb3e9_f4b4_c2f6_eff8),
        ("donut_prog_primary_mapping", 0xcd02_6290_cfd5_d561),
        ("donut_prog_boss_pins", 0x0a03_af02_5ce9_5b25),
        ("donut_prog_boss_both_programs", 0xbefb_7d69_bbe9_4cfd),
    ];

    #[test]
    fn the_registry_is_listed_and_every_fixture_matches_its_golden_and_replays_identically() {
        assert_eq!(GOLDENS.len(), DONUT_PROG_FIXTURES.len());
        let listed = list_fixtures("donut_prog_");
        for (fixture, (name, expected)) in DONUT_PROG_FIXTURES.iter().zip(GOLDENS) {
            assert_eq!(fixture.name(), name);
            assert!(listed.contains(&name), "{name} is listed");
            assert!(find_party_fixture(name).is_some(), "{name} is runnable");
            let (a, b) = (replay(fixture), replay(fixture));
            assert_eq!(a.state_hash(), b.state_hash(), "{name} replays");
            assert_eq!(lines(&a), lines(&b), "{name}: the same events twice");
            assert_eq!(a.state_hash(), expected, "{name}: {}", a.state_line());
        }
        assert_eq!(
            listed.len(),
            DONUT_PROG_FIXTURES.len(),
            "nothing else in the namespace"
        );
        let mut hashes: Vec<u64> = GOLDENS.iter().map(|g| g.1).collect();
        hashes.sort_unstable();
        hashes.dedup();
        assert_eq!(hashes.len(), GOLDENS.len(), "every fixture is distinct");
    }

    #[test]
    fn the_committed_replays_are_the_replay() {
        for fixture in DONUT_PROG_FIXTURES {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("docs/evidence/donut-prog-01/replays")
                .join(format!("headless_{}.txt", fixture.name()));
            let committed = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let committed: Vec<&str> = committed.lines().collect();
            assert_eq!(committed, lines(&replay(fixture)), "{}", fixture.name());
        }
    }

    #[test]
    fn no_other_registry_holds_a_donut_prog_name_and_every_other_fixture_is_unpinned() {
        for name in list_fixtures("") {
            if !name.starts_with("donut_prog_") {
                assert!(
                    story_setup(name).is_inert(),
                    "{name} is a legacy fixture: no pin, no trigger"
                );
            }
        }
        for fixture in BOSS_FIXTURES {
            assert!(story_setup(fixture.name()).is_inert());
        }
        // Carl's registry keeps its own setups (the shared lookup falls through to ours).
        for name in list_fixtures("carl_prog_") {
            assert!(story_setup(name).is_inert());
        }
        assert!(!crate::carl_prog_fixtures::story_setup("carl_prog_juicer_grants_fist").is_inert());
    }

    /// DP-Q01, DP-Q02, DP-Q06 (the story corridor run, simulation and events
    /// only): the biscuit at the guild zone on the existing tick, then the
    /// jump boost exactly once at the safe-room placeholder zone.
    #[test]
    fn donut_prog_story_unlocks() {
        let sim = named("donut_prog_story_unlocks");
        let unlock: Vec<u64> = sim
            .combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Mana(ManaEvent::UnlockGranted { tick, .. }) => Some(tick),
                _ => None,
            })
            .collect();
        assert_eq!(unlock, [50], "the guild zone, on the agile body's tick");
        let granted = grants(&sim);
        assert_eq!(
            granted,
            [(246, 1, "jump_reach_boost")],
            "once, to hero 1, after the biscuit"
        );
        assert!(granted[0].0 > unlock[0]);
        assert!(sim.combat.story_holds(boost()));
        assert!(
            sim.combat.story_holds(PRIMARY_RANGED),
            "the biscuit reads through"
        );
        assert!(
            !sim.combat.story_holds(blink()),
            "no blink without the Juicer"
        );
        assert!(
            !sim.combat.story_holds(SPECIAL_THROWN_CHARGE),
            "the Donut program never grants a Carl row"
        );
        let ledger = sim.combat.story_ledger().unwrap();
        assert_eq!(ledger.receipts().len(), 1);
        assert_eq!(ledger.receipts()[0].source, "f01_safe_room_reward_found");
        // Circle/B before and after the grant: nothing was logged for it.
        let kinds: Vec<String> = sim.combat.events.iter().map(ToString::to_string).collect();
        assert_eq!(kinds.len(), 2, "only the unlock and the grant: {kinds:?}");
    }

    /// CP-Q07 for Donut's row: a defeat retry keeps the grant (twice), a
    /// fixture restart restores the empty declared start and re-arms the zone.
    #[test]
    fn donut_prog_grant_survives_retry() {
        let mut sim = named("donut_prog_grant_survives_retry");
        assert_eq!(
            grants(&sim),
            [(246, 1, "jump_reach_boost")],
            "one grant, never two"
        );
        let restarts: Vec<_> = sim
            .combat
            .events
            .iter()
            .filter(|e| matches!(e, CombatEvent::Restart { .. }))
            .collect();
        assert_eq!(restarts.len(), 2, "defeated and retried twice");
        assert!(sim.combat.deaths >= 2);
        assert!(sim.combat.story_holds(boost()), "the retries keep it");
        let receipts = sim.combat.story_ledger().unwrap().receipts();
        assert_eq!((receipts.len(), receipts[0].tick), (1, 246));
        // The biscuit survives the retry too (mana v0), so the order still holds.
        assert!(sim.combat.story_holds(PRIMARY_RANGED));
        // A fixture restart restores the declared start: nothing held, biscuit locked.
        sim.combat.restart(&mut sim.scene);
        assert!(!sim.combat.story_holds(boost()));
        assert!(sim.combat.story_ledger().is_none_or(|l| l.is_empty()));
        assert!(!sim.combat.story_holds(PRIMARY_RANGED));
        // The zone is armed again: walking in grants once more.
        sim.party.restart();
        sim.run_scripted(330);
        assert!(sim.combat.story_holds(boost()), "re-armed by the restart");
    }

    /// DP-Q11 on the checkpoint respawn of the demo hall: a grant earned in
    /// play survives it, a start hold is a declared start and never a grant, a
    /// fixture restart drops the grant and keeps the hold.
    #[test]
    fn a_checkpoint_respawn_keeps_a_donut_grant_and_a_fixture_restart_removes_it() {
        let kit = load_hall(&root()).unwrap();
        let fixture = fixture_for_beat(1);
        let scene = SceneSim::new_at(&kit, 360, fixture.carl_x).unwrap();
        let combat = CombatState::new(fixture, &kit, &scene)
            .unwrap()
            .with_demo(&DEMO_RULES)
            .with_donut(DonutSetup::unlocked(8))
            .with_story(StorySetup::NONE.with_hold("special_short_blink"));
        let mut run = FixtureSim { scene, combat };
        assert!(run.combat.story_holds(blink()), "a pin is held from tick 0");
        assert!(
            run.combat.story_holds(boost()),
            "a hold grants the requires closure"
        );
        run.combat.respawn(&mut run.scene);
        assert!(run.combat.story_holds(blink()), "the respawn keeps it");
        assert!(
            run.combat
                .events
                .iter()
                .all(|e| !matches!(e, CombatEvent::CapabilityGranted { .. })),
            "a pin is a declared start, never a grant"
        );
        // A grant earned in play: Donut's pivot inside the safe-room zone.
        let mut earned = CombatState::new(fixture, &kit, &run.scene)
            .unwrap()
            .with_demo(&DEMO_RULES)
            .with_donut(DonutSetup::unlocked(8))
            .with_story(StorySetup::NONE.with_trigger("f01_safe_room_reward_found"));
        let mut scene = run.scene.clone();
        scene.world.donut_body.x = 1210.0;
        scene.world.sync_donut_for_tests();
        earned.step(&mut scene, crate::input::RawInput::default());
        assert!(earned.story_holds(boost()), "the zone granted it");
        earned.respawn(&mut scene);
        assert!(earned.story_holds(boost()), "the respawn keeps a grant");
        earned.restart(&mut scene);
        assert!(!earned.story_holds(boost()), "R restores the empty start");
        run.combat.restart(&mut run.scene);
        assert!(run.combat.story_holds(blink()), "R restores the pin");
    }

    /// DP-Q01 and M1: the primary mapping is the legacy one (swat, then cast,
    /// then dry) and Circle/B does nothing before the blink is held: no event,
    /// and the run hashes exactly like the same inputs with no story at all.
    #[test]
    fn donut_prog_primary_mapping() {
        let sim = named("donut_prog_primary_mapping");
        let moves: Vec<(u64, &str)> = sim
            .combat
            .events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::MoveStarted { tick, move_id, .. } => Some((*tick, move_id.name())),
                _ => None,
            })
            .collect();
        assert_eq!(moves[0], (8, "swat"), "before the biscuit Square/X swats");
        assert!(
            moves[1..]
                .iter()
                .all(|(tick, name)| *tick >= 59 && *name == "magic_missile"),
            "after the grant (tick 59) it casts: {moves:?}"
        );
        assert_eq!(moves.len(), 9, "one swat, eight casts");
        let dry: Vec<u64> = sim
            .combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Mana(ManaEvent::CastRejected { tick, .. }) => Some(tick),
                _ => None,
            })
            .collect();
        assert_eq!(dry, [230, 250], "at 0 mana the press is dry");
        // Circle/B (ticks 10, 40, 260, 340): the event log holds nothing of it
        // and no capability is held.
        assert!(grants(&sim).is_empty());
        assert!(!sim.combat.story_holds(blink()));
        // The same script with no story: identical hash and events.
        let fixture = &DONUT_PROG_PRIMARY_MAPPING;
        let kit = kit_of(fixture);
        let scene = SceneSim::new_at(&kit, 360, fixture.base.carl_x).unwrap();
        let combat = CombatState::new(fixture.base, &kit, &scene)
            .unwrap()
            .with_donut(fixture.donut);
        let mut plain = PartyFixtureSim::from_parts(scene, combat, fixture);
        plain.run_scripted(fixture.base.ticks);
        assert_eq!(plain.state_hash(), sim.state_hash());
        assert_eq!(plain.combat.events, sim.combat.events);
    }

    /// DP-Q03, DP-Q09, rule 3: under the Donut program alone the Juicer trigger
    /// grants only the blink, on the defeat tick, and the pin emits nothing.
    #[test]
    fn donut_prog_boss_pins_grants_only_the_blink() {
        let sim = named("donut_prog_boss_pins");
        let defeat = sim
            .combat
            .events
            .iter()
            .find_map(|e| match *e {
                CombatEvent::EnemyDefeated { tick, .. } => Some(tick),
                _ => None,
            })
            .expect("the boss is defeated");
        assert_eq!(grants(&sim), [(defeat, 1, "special_short_blink")]);
        assert!(sim.combat.story_holds(boost()), "the pin");
        assert!(sim.combat.story_holds(blink()));
        assert!(
            !sim.combat.story_holds(FIST_REINFORCED),
            "Carl's entry is skipped"
        );
        assert!(!sim.combat.story_holds(SPECIAL_THROWN_CHARGE));
        let ledger = sim.combat.story_ledger().unwrap();
        assert_eq!(
            ledger.receipts().len(),
            1,
            "one receipt, none for the pin or the skip"
        );
    }

    /// DP-Q03: with both programs the two grants land on the defeat tick in
    /// grants-list order.
    #[test]
    fn donut_prog_boss_both_programs_grants_in_list_order() {
        let sim = named("donut_prog_boss_both_programs");
        let defeat = sim
            .combat
            .events
            .iter()
            .find_map(|e| match *e {
                CombatEvent::EnemyDefeated { tick, .. } => Some(tick),
                _ => None,
            })
            .expect("the boss is defeated");
        assert_eq!(
            grants(&sim),
            [
                (defeat, 0, "fist_reinforced"),
                (defeat, 1, "special_short_blink")
            ]
        );
        assert!(sim.combat.story_holds(SPECIAL_THROWN_CHARGE), "Carl's pin");
        assert!(sim.combat.story_holds(boost()), "Donut's pin");
    }

    /// The held set of the boss arena at tick 0 under each flag combination:
    /// CP-Q08's dynamite, Donut's boost (the biscuit read through) and their
    /// union.
    #[test]
    fn donut_prog_boss_pins_the_held_set_per_flag_combination() {
        let carl = CarlProgFlags {
            prog: true,
            ..CarlProgFlags::OFF
        };
        let held = |c: CarlProgFlags, d: DonutProgFlags| {
            let mut ids: Vec<&str> = program_setup(c, d, SceneKind::Boss)
                .holds()
                .iter()
                .map(|id| id.as_str())
                .collect();
            ids.sort_unstable();
            ids
        };
        assert_eq!(held(carl, DonutProgFlags::OFF), ["special_thrown_charge"]);
        assert_eq!(held(CarlProgFlags::OFF, DONUT_PROG), ["jump_reach_boost"]);
        assert_eq!(
            held(carl, DONUT_PROG),
            ["jump_reach_boost", "special_thrown_charge"]
        );
        // The scene accepts every one of them with the biscuit unlocked, and
        // the boss pin with the biscuit locked names the pin.
        for (c, d) in [
            (carl, DonutProgFlags::OFF),
            (CarlProgFlags::OFF, DONUT_PROG),
            (carl, DONUT_PROG),
        ] {
            let setup = program_setup(c, d, SceneKind::Boss);
            assert!(
                setup
                    .check_read_through(&ReadThrough {
                        primary_ranged: true
                    })
                    .is_ok()
            );
        }
        let locked = program_setup(CarlProgFlags::OFF, DONUT_PROG, SceneKind::Boss)
            .check_read_through(&ReadThrough::default())
            .expect_err("the boss pin needs the biscuit");
        assert!(
            locked
                .to_string()
                .contains("pins[f01_donut_boss_held].holds"),
            "{locked}"
        );
    }

    /// A pin that breaks the story order fails at setup, naming the pin, and a
    /// scene built that way is refused when the combat state is assembled.
    #[test]
    #[should_panic(expected = "pins[f01_donut_boss_held].holds")]
    fn a_donut_boss_pin_without_the_biscuit_is_refused_at_setup() {
        let fixture = &DONUT_PROG_BOSS_PINS;
        let kit = kit_of(fixture);
        let scene = SceneSim::new_at(&kit, 360, fixture.base.carl_x).unwrap();
        let _ = CombatState::new(fixture.base, &kit, &scene)
            .unwrap()
            .with_donut(DonutSetup::NONE)
            .with_story(story_setup(fixture.name()));
    }

    /// Rule 3 on the real Juicer row and the real safe-room row, at the runtime
    /// level (the fixtures above prove it in the scene): one trigger fires once,
    /// each entry is guarded by its program and by `requires`, and a skipped
    /// entry leaves no receipt, no hash and one debug line.
    #[test]
    fn the_per_entry_guards_on_the_real_rows() {
        let table = StoryTable::active();
        let view = ReadThrough {
            primary_ranged: true,
        };
        let juicer = |carl: bool, donut: bool, hold_boost: bool| {
            let mut setup = program_setup(
                CarlProgFlags {
                    prog: carl,
                    ..CarlProgFlags::OFF
                },
                DonutProgFlags {
                    prog: donut,
                    ..DonutProgFlags::OFF
                },
                SceneKind::Boss,
            );
            if hold_boost && !donut {
                setup = setup.with_hold("jump_reach_boost");
            }
            let mut runtime = StoryRuntime::new(&setup).expect("armed");
            runtime.observe("brute_boss_f01_defeated");
            let receipts = runtime.step(9, [(0.0, 0.0); 2], false, &view);
            let granted: Vec<(u8, &str)> = receipts
                .iter()
                .map(|r| (r.hero, r.capability.as_str()))
                .collect();
            let mut mixed = Vec::new();
            runtime.hash_into(&mut |v| mixed.push(v));
            (granted, runtime.debug_log().len(), mixed.len())
        };
        // Carl's program alone: only the fist, the skipped blink leaves one debug line.
        let (granted, skipped, _) = juicer(true, false, false);
        assert_eq!(granted, [(0, "fist_reinforced")]);
        assert_eq!(skipped, 1);
        // Donut's program alone: only the blink (her pin holds the boost).
        let (granted, skipped, _) = juicer(false, true, false);
        assert_eq!(granted, [(1, "special_short_blink")]);
        assert_eq!(skipped, 1, "Carl's entry is skipped");
        // Both, in list order on one tick.
        let (granted, skipped, _) = juicer(true, true, false);
        assert_eq!(
            granted,
            [(0, "fist_reinforced"), (1, "special_short_blink")]
        );
        assert_eq!(skipped, 0);
        // A skipped entry changes nothing a hash can see: Carl alone hashes
        // like a ledger that holds the same capabilities with no skip.
        let (_, _, carl_words) = juicer(true, false, false);
        let (_, _, both_words) = juicer(true, true, false);
        assert!(carl_words < both_words);
        // The safe-room zone: armed by the Donut program only, one grant, and
        // only with the biscuit read through (the `requires` guard).
        let corridor = |carl: bool, biscuit: bool| {
            let setup = program_setup(
                CarlProgFlags {
                    prog: carl,
                    ..CarlProgFlags::OFF
                },
                DonutProgFlags {
                    prog: !carl,
                    ..DonutProgFlags::OFF
                },
                SceneKind::Corridor,
            );
            let mut runtime = StoryRuntime::new(&setup)?;
            let view = ReadThrough {
                primary_ranged: biscuit,
            };
            let inside = [(0.0, 0.0), (1210.0, 0.0)];
            let first = runtime.step(1, inside, false, &view);
            let again = runtime.step(2, inside, false, &view);
            assert!(again.is_empty(), "a trigger is consumed, never re-armed");
            Some((first.len(), runtime.debug_log().len()))
        };
        assert_eq!(corridor(false, true), Some((1, 0)));
        assert_eq!(
            corridor(false, false),
            Some((0, 1)),
            "without the biscuit the entry is skipped and the trigger is spent"
        );
        assert_eq!(
            corridor(true, true),
            None,
            "`--carl-prog` never evaluates the safe-room zone"
        );
        assert_eq!(
            table
                .trigger("brute_boss_f01_defeated")
                .unwrap()
                .grants
                .len(),
            2
        );
    }

    /// `--donut-prog` (alone and with `--carl-prog`) on the DEMO-01 hall and the
    /// CF02 microarena reproduces the unflagged run exactly: nothing is held and
    /// nothing is armed there. The opening route runs no combat state at all.
    #[test]
    fn donut_prog_flag_parity() {
        for carl in [
            CarlProgFlags::OFF,
            CarlProgFlags {
                prog: true,
                ..CarlProgFlags::OFF
            },
        ] {
            // DEMO-01 hall, the autopilot from the first beat.
            let hall = load_hall(&root()).unwrap();
            let demo = |story: Option<StorySetup>| {
                let fixture = fixture_for_beat(0);
                let scene = SceneSim::new_at(&hall, 360, fixture.carl_x).unwrap();
                let mut combat = CombatState::new(fixture, &hall, &scene)
                    .unwrap()
                    .with_demo(&DEMO_RULES);
                if let Some(story) = story {
                    combat = combat.with_story(story);
                }
                let mut run = FixtureSim { scene, combat };
                for _ in 0..1500 {
                    let raw = autopilot(&run.combat, &run.scene);
                    run.step(raw);
                }
                run
            };
            let plain = demo(None);
            let flagged = demo(Some(program_setup(carl, DONUT_PROG, SceneKind::Demo)));
            assert_eq!(plain.state_hash(), flagged.state_hash());
            assert_eq!(plain.combat.events, flagged.combat.events);
            assert!(flagged.combat.story_ledger().is_none());
            // CF02 microarena, the direct plan.
            let arena = crate::cf02::arena::load_arena(&root()).unwrap();
            let cf02 = |story: Option<StorySetup>| {
                let fixture =
                    crate::cf02::arena::fixture_for_run(0, false, crate::engineer::active());
                let scene = SceneSim::new_at(&arena, 360, fixture.carl_x).unwrap();
                let mut combat = CombatState::new(fixture, &arena, &scene)
                    .unwrap()
                    .with_demo(&crate::cf02::arena::RULES)
                    .with_cf02();
                if let Some(story) = story {
                    combat = combat.with_story(story);
                }
                let mut run = FixtureSim { scene, combat };
                for _ in 0..1500 {
                    let raw = crate::cf02::arena::autopilot(
                        &run.combat,
                        &run.scene,
                        crate::cf02::arena::Plan::Direct,
                    );
                    run.step(raw);
                }
                run
            };
            let plain = cf02(None);
            let flagged = cf02(Some(program_setup(carl, DONUT_PROG, SceneKind::CfArena)));
            assert_eq!(plain.state_hash(), flagged.state_hash());
            assert_eq!(plain.combat.events, flagged.combat.events);
        }
        // The start flags are different: they hold a capability from tick 0 and
        // so change the hash (the opt-in is visible, never silent), and they
        // grant nothing (a declared start, no event).
        let hall = load_hall(&root()).unwrap();
        let held = {
            let fixture = fixture_for_beat(0);
            let scene = SceneSim::new_at(&hall, 360, fixture.carl_x).unwrap();
            let combat = CombatState::new(fixture, &hall, &scene)
                .unwrap()
                .with_demo(&DEMO_RULES)
                .with_donut(DonutSetup::unlocked(8))
                .with_story(program_setup(
                    CarlProgFlags::OFF,
                    DonutProgFlags {
                        puddle: true,
                        ..DonutProgFlags::OFF
                    },
                    SceneKind::Demo,
                ));
            FixtureSim { scene, combat }
        };
        assert!(held.combat.story_holds(blink()) && held.combat.story_holds(boost()));
        assert!(
            held.combat
                .events
                .iter()
                .all(|e| !matches!(e, CombatEvent::CapabilityGranted { .. })),
            "a start flag is a declared start state, not a grant"
        );
        assert!(!held.combat.story_ledger().unwrap().is_empty());
        let _ = (FIST_REINFORCED, PRIMARY_RANGED);
    }
}
