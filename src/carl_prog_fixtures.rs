//! CARL-PROG-01 fixtures (the `carl_prog_*` namespace): the story-unlock
//! ledger as replay fixtures.
//!
//! Bevy-free party fixtures in the shape of [`crate::party_scene`]. Each one
//! declares its own story pin ([`story_setup`]: the capabilities it starts with
//! and the triggers armed in it, never a debug flag), has a committed replay in
//! `docs/evidence/carl-prog-01/replays` and a golden hash pinned by a test in
//! this file. They are **not** in [`PartyFixture::all`]: the registry is
//! [`CARL_PROG_FIXTURES`], listed by `floor01_scene --list-fixtures carl_prog_`
//! and replayed by the CI loop, so every earlier fixture, golden and replay is
//! untouched. Later CARL-PROG PRs append their fixtures here.
//!
//! Tick numbering is the one of [`crate::combat_scene`]: a hold over
//! `from..to` covers the steps that produce `tick = from .. to - 1`.

use crate::boss_scene::{Pilot, boss_base, hp};
use crate::carl_prog::CarlProgFlags;
use crate::combat::EnemyAi;
use crate::combat_scene::{EnemySpawn, Fixture};
use crate::donut_mana::DonutSetup;
use crate::party::PROVISIONAL_TUNING;
use crate::party_scene::{PartyFixture, base, run};
use crate::story_unlock::{SceneKind, StorySetup};

/// The placeholder zone of `pile_boss_f01_defeated` (K1, f01 integration plan
/// #182: the thrown charge is granted on the Hoarder's defeat; the Hoarder does
/// not exist yet, HRD-A, so a stub zone stands in): Carl's pivot inside this half-open x range grants the
/// thrown charge. Read from `story_unlocks_v0.json`, repeated here only so the
/// scripts below can be read next to it.
pub const FIND_STUB_ZONE_X: (f32, f32) = (600.0, 640.0);

const fn chase(x: i32) -> EnemySpawn {
    EnemySpawn {
        x,
        ai: EnemyAi::Chase,
    }
}

/// The story a `carl_prog_*` fixture declares for itself (a scripted fixture
/// never reads a debug flag, CP-Q09). Every other fixture name is the legacy
/// game: [`StorySetup::NONE`].
pub fn story_setup(fixture: &str) -> StorySetup {
    match fixture {
        // The placeholder Hoarder-defeat zone is armed by the fixture, by id: no scene arms it.
        "carl_prog_story_event_grants_dynamite" | "carl_prog_grant_survives_respawn" => {
            StorySetup::NONE.with_trigger("pile_boss_f01_defeated")
        }
        // The boss pin and the Juicer trigger, as `--carl-prog` gives the arena.
        "carl_prog_juicer_grants_fist" => CarlProgFlags {
            prog: true,
            ..CarlProgFlags::OFF
        }
        .story_setup(SceneKind::Boss),
        _ => StorySetup::NONE,
    }
}

const FIND_BASE: Fixture = base(
    "carl_prog_story_event_grants_dynamite",
    "Solo: Carl walks into the placeholder Hoarder-defeat zone and the ledger grants the thrown charge once; walking on and back grants nothing more",
    0x0CA1_0001,
    &[],
    240,
);

/// CP-Q01, CP-Q11: a zone trigger grants a capability exactly once, with one
/// `CapabilityGranted` on the first tick Carl's pivot is inside.
pub const CARL_PROG_STORY_EVENT_GRANTS_DYNAMITE: PartyFixture = PartyFixture {
    base: &FIND_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[run(5, 110, 0, 1.0), run(130, 200, 0, -1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const RESPAWN_BASE: Fixture = base(
    "carl_prog_grant_survives_respawn",
    "Solo: Carl takes the placeholder zone grant, two goblins defeat him and the party retries: the grant stays and is not granted twice",
    0x0CA1_0002,
    &[chase(700), chase(740)],
    1500,
);

/// CP-Q07: a grant survives a defeat retry (the demo's checkpoint respawn and a
/// solo swap are unit-tested); a fixture restart removes it.
pub const CARL_PROG_GRANT_SURVIVES_RESPAWN: PartyFixture = PartyFixture {
    base: &RESPAWN_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[run(5, 110, 0, 1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const JUICER_BASE: Fixture = boss_base(
    "carl_prog_juicer_grants_fist",
    "The boss has 4 HP left and Carl holds the thrown charge from the pin: the rack crush defeats it and the Juicer trigger grants the reinforced fist on that tick",
    0x0CA1_0003,
    180,
    hp(4),
    420,
);

/// CP-Q02: the boss defeat is the event; exactly one `CapabilityGranted`
/// (`fist_reinforced`) on the defeat tick, and none for the pin.
pub const CARL_PROG_JUICER_GRANTS_FIST: PartyFixture = PartyFixture {
    base: &JUICER_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::CarlRack,
    donut: DonutSetup::NONE,
};

/// Every `carl_prog_*` replay fixture, in committed order.
pub const CARL_PROG_FIXTURES: [&PartyFixture; 3] = [
    &CARL_PROG_STORY_EVENT_GRANTS_DYNAMITE,
    &CARL_PROG_GRANT_SURVIVES_RESPAWN,
    &CARL_PROG_JUICER_GRANTS_FIST,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss::BOSS_ENEMY_ID;
    use crate::boss_scene::{BOSS_FIXTURES, load_arena};
    use crate::cf02_fixtures::{find_party_fixture, list_fixtures};
    use crate::combat::CombatEvent;
    use crate::combat_scene::{CombatState, FixtureSim};
    use crate::corridor::SceneSim;
    use crate::demo::{DEMO_RULES, autopilot, fixture_for_beat, load_hall};
    use crate::env_kit::EnvKit;
    use crate::party_scene::PartyFixtureSim;
    use crate::story_unlock::{
        CapabilityId, FIST_REINFORCED, PRIMARY_RANGED, SPECIAL_THROWN_CHARGE, StoryTable,
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

    /// The committed final hashes of the `carl_prog_*` fixtures.
    const GOLDENS: [(&str, u64); 3] = [
        (
            "carl_prog_story_event_grants_dynamite",
            0x7594_ba64_4fd2_dbaf,
        ),
        ("carl_prog_grant_survives_respawn", 0x8eeb_9036_87de_c95d),
        ("carl_prog_juicer_grants_fist", 0xddd6_81ab_67f5_26ac),
    ];

    #[test]
    fn the_registry_is_listed_and_every_fixture_matches_its_golden_and_replays_identically() {
        assert_eq!(GOLDENS.len(), CARL_PROG_FIXTURES.len());
        let listed = list_fixtures("carl_prog_");
        for (fixture, (name, expected)) in CARL_PROG_FIXTURES.iter().zip(GOLDENS) {
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
            CARL_PROG_FIXTURES.len(),
            "nothing else in the namespace"
        );
        let mut hashes: Vec<u64> = GOLDENS.iter().map(|g| g.1).collect();
        hashes.sort_unstable();
        hashes.dedup();
        assert_eq!(hashes.len(), GOLDENS.len(), "every fixture is distinct");
    }

    #[test]
    fn the_committed_replays_are_the_replay() {
        for fixture in CARL_PROG_FIXTURES {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("docs/evidence/carl-prog-01/replays")
                .join(format!("headless_{}.txt", fixture.name()));
            let committed = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let committed: Vec<&str> = committed.lines().collect();
            assert_eq!(committed, lines(&replay(fixture)), "{}", fixture.name());
        }
    }

    #[test]
    fn no_other_registry_holds_a_carl_prog_name_and_the_boss_fixtures_stay_unpinned() {
        for name in crate::cf02_fixtures::list_fixtures("") {
            if !name.starts_with("carl_prog_") {
                assert!(
                    story_setup(name).is_inert(),
                    "{name} is a legacy fixture: no pin, no trigger"
                );
            }
        }
        for fixture in BOSS_FIXTURES {
            assert!(story_setup(fixture.name()).is_inert());
        }
    }

    /// CP-Q01 and CP-Q11 (simulation and events only; the notice and the socket
    /// are HUD assertions of CP-B2): the stub zone grants once.
    #[test]
    fn carl_prog_story_event_grants_dynamite() {
        let sim = named("carl_prog_story_event_grants_dynamite");
        let granted = grants(&sim);
        assert_eq!(
            granted.len(),
            1,
            "exactly one CapabilityGranted: {granted:?}"
        );
        let (tick, hero, capability) = granted[0];
        assert_eq!((hero, capability), (0, "special_thrown_charge"));
        // The first tick Carl's pivot is inside the half-open zone.
        let mut probe = PartyFixtureSim::new(
            &kit_of(&CARL_PROG_STORY_EVENT_GRANTS_DYNAMITE),
            360,
            &CARL_PROG_STORY_EVENT_GRANTS_DYNAMITE,
        )
        .unwrap();
        let mut first_inside = None;
        for _ in 0..CARL_PROG_STORY_EVENT_GRANTS_DYNAMITE.base.ticks {
            probe.step_scripted();
            let x = probe.scene.world.hero.x;
            if first_inside.is_none() && (FIND_STUB_ZONE_X.0..FIND_STUB_ZONE_X.1).contains(&x) {
                first_inside = Some(probe.scene.world.tick);
            }
        }
        assert_eq!(first_inside, Some(tick), "granted on the first tick inside");
        assert!(sim.combat.story_holds(SPECIAL_THROWN_CHARGE));
        assert!(!sim.combat.story_holds(FIST_REINFORCED));
        // The data row carries the notice text (13 characters at most).
        let notice = StoryTable::active()
            .notice("special_thrown_charge")
            .unwrap();
        assert_eq!(notice.text, "GOBLIN BOOM");
        // The receipt names the trigger that granted it.
        let receipts = sim.combat.story_ledger().unwrap().receipts();
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].source, "pile_boss_f01_defeated");
    }

    /// CP-Q07: a solo swap and a defeat retry keep the grant, no second grant is
    /// logged, and a fixture restart restores the declared (empty) start.
    #[test]
    fn carl_prog_grant_survives_respawn() {
        let mut sim = named("carl_prog_grant_survives_respawn");
        let granted = grants(&sim);
        assert_eq!(
            granted.len(),
            1,
            "one grant, after the retry too: {granted:?}"
        );
        let restart_at = sim
            .combat
            .events
            .iter()
            .position(|e| matches!(e, CombatEvent::Restart { .. }))
            .expect("the goblin defeated Carl and the party retried");
        let grant_at = sim
            .combat
            .events
            .iter()
            .position(|e| matches!(e, CombatEvent::CapabilityGranted { .. }))
            .unwrap();
        assert!(grant_at < restart_at, "the grant came before the defeat");
        assert!(sim.combat.deaths >= 1);
        assert!(
            sim.combat.story_holds(SPECIAL_THROWN_CHARGE),
            "the retry keeps the grant"
        );
        // Receipt ticks are attempt-local and keep the first attempt's tick.
        let receipts = sim.combat.story_ledger().unwrap().receipts();
        assert_eq!((receipts.len(), receipts[0].tick), (1, granted[0].0));
        // A fixture restart (R / Select) restores the declared start: nothing.
        sim.combat.restart(&mut sim.scene);
        assert!(!sim.combat.story_holds(SPECIAL_THROWN_CHARGE));
        assert!(sim.combat.story_ledger().unwrap().is_empty());
        // The zone is armed again: walking in grants once more.
        sim.party.restart();
        sim.run_scripted(120);
        assert!(sim.combat.story_holds(SPECIAL_THROWN_CHARGE));
    }

    /// CP-Q07: a solo swap neither grants nor removes anything and the ledger
    /// hash does not move with it.
    #[test]
    fn a_solo_swap_leaves_the_ledger_alone() {
        let mut sim = PartyFixtureSim::new(
            &kit_of(&CARL_PROG_STORY_EVENT_GRANTS_DYNAMITE),
            360,
            &CARL_PROG_STORY_EVENT_GRANTS_DYNAMITE,
        )
        .unwrap();
        sim.run_scripted(100);
        assert!(sim.combat.story_holds(SPECIAL_THROWN_CHARGE));
        let mix = |sim: &PartyFixtureSim| {
            let mut out = Vec::new();
            sim.combat
                .story_ledger()
                .unwrap()
                .hash_into(&mut |v| out.push(v));
            out
        };
        let before = mix(&sim);
        let mut slots = [crate::input::RawInput::default(); 2];
        slots[0].swap = true;
        sim.step_slots(slots);
        sim.run_scripted(0);
        for _ in 0..30 {
            sim.step_slots([crate::input::RawInput::default(); 2]);
        }
        assert_eq!(sim.party.controlled_hero(), crate::party::HeroId::Donut);
        assert!(sim.combat.story_holds(SPECIAL_THROWN_CHARGE));
        assert_eq!(mix(&sim), before);
        assert_eq!(grants(&sim).len(), 1);
    }

    /// CP-Q07 on the checkpoint respawn of the demo hall: a grant survives it, a
    /// fixture restart removes it.
    #[test]
    fn a_checkpoint_respawn_keeps_a_grant_and_a_fixture_restart_removes_it() {
        let kit = load_hall(&root()).unwrap();
        let fixture = fixture_for_beat(1);
        let scene = SceneSim::new_at(&kit, 360, fixture.carl_x).unwrap();
        let combat = CombatState::new(fixture, &kit, &scene)
            .unwrap()
            .with_demo(&DEMO_RULES)
            .with_story(StorySetup::NONE.with_hold("fist_reinforced"));
        let mut run = FixtureSim { scene, combat };
        assert!(
            run.combat.story_holds(FIST_REINFORCED),
            "a pin is held from tick 0"
        );
        run.combat.respawn(&mut run.scene);
        assert!(
            run.combat.story_holds(FIST_REINFORCED),
            "the respawn keeps it"
        );
        assert!(
            run.combat
                .events
                .iter()
                .all(|e| !matches!(e, CombatEvent::CapabilityGranted { .. })),
            "a pin is a declared start, never a grant"
        );
        // A grant earned in play survives the respawn; the fixture restart
        // drops it and keeps the pin.
        let mut earned = CombatState::new(fixture, &kit, &run.scene)
            .unwrap()
            .with_demo(&DEMO_RULES)
            .with_story(StorySetup::NONE.with_trigger("pile_boss_f01_defeated"));
        let mut scene = run.scene.clone();
        scene.world.hero.x = 610.0;
        earned.step(&mut scene, crate::input::RawInput::default());
        assert!(earned.story_holds(SPECIAL_THROWN_CHARGE));
        earned.respawn(&mut scene);
        assert!(
            earned.story_holds(SPECIAL_THROWN_CHARGE),
            "the respawn keeps a grant"
        );
        earned.restart(&mut scene);
        assert!(
            !earned.story_holds(SPECIAL_THROWN_CHARGE),
            "R restores the empty start"
        );
        run.combat.restart(&mut run.scene);
        assert!(
            run.combat.story_holds(FIST_REINFORCED),
            "R restores the pin"
        );
    }

    /// CP-Q02: under the boss pin the boss defeat emits exactly one
    /// `CapabilityGranted { fist_reinforced }` on the defeat tick.
    #[test]
    fn carl_prog_juicer_grants_fist() {
        let sim = named("carl_prog_juicer_grants_fist");
        let granted = grants(&sim);
        assert_eq!(granted.len(), 1, "{granted:?}");
        let defeat = sim
            .combat
            .events
            .iter()
            .find_map(|e| match *e {
                CombatEvent::EnemyDefeated { tick, target, .. } if target == BOSS_ENEMY_ID => {
                    Some(tick)
                }
                _ => None,
            })
            .expect("the boss fell");
        assert_eq!(
            granted[0],
            (defeat, 0, "fist_reinforced"),
            "on the defeat tick"
        );
        // The pin holds the thrown charge from tick 0 and emitted nothing.
        assert!(sim.combat.story_holds(SPECIAL_THROWN_CHARGE));
        assert!(sim.combat.story_holds(FIST_REINFORCED));
        let receipts = sim.combat.story_ledger().unwrap().receipts();
        assert_eq!(receipts.len(), 1, "a pin has no receipt");
        assert_eq!(receipts[0].source, "brute_boss_f01_defeated");
    }

    /// CP-Q45: the biscuit is a read-through. With the ledger live around it
    /// (triggers armed, nothing held), `--scene boss_f01`'s biscuit and a `q10_*`
    /// run hash and log exactly as without, `holds` follows `DonutMana`, and the
    /// ledger never stores, hashes, receipts or emits it.
    #[test]
    fn carl_prog_biscuit_readthrough_hash() {
        let armed = || {
            // The Juicer trigger and the placeholder zone armed, no pin: the
            // ledger exists and is empty. The fixtures below neither defeat the
            // boss nor walk Carl into the zone.
            StorySetup::NONE
                .with_trigger("brute_boss_f01_defeated")
                .with_trigger("pile_boss_f01_defeated")
        };
        for name in [
            "q10_unlock_persists",
            "q10_mana_regen",
            "boss01_pound_punish",
            "boss01_charge_jump",
        ] {
            let fixture = find_party_fixture(name).unwrap();
            let kit = kit_of(fixture);
            let run = |story: Option<StorySetup>| {
                let mut combat = CombatState::new(
                    fixture.base,
                    &kit,
                    &SceneSim::new_at(&kit, 360, fixture.base.carl_x).unwrap(),
                )
                .unwrap()
                .with_donut(fixture.donut);
                if let Some(story) = story {
                    combat = combat.with_story(story);
                }
                let scene = SceneSim::new_at(&kit, 360, fixture.base.carl_x).unwrap();
                let mut sim = PartyFixtureSim::from_parts(scene, combat, fixture);
                sim.run_scripted(fixture.base.ticks);
                sim
            };
            let plain = run(None);
            let with_ledger = run(Some(armed()));
            assert_eq!(
                plain.state_hash(),
                with_ledger.state_hash(),
                "{name}: same hash"
            );
            assert_eq!(lines(&plain), lines(&with_ledger), "{name}: same log");
            assert!(
                with_ledger
                    .combat
                    .events
                    .iter()
                    .all(|e| !matches!(e, CombatEvent::CapabilityGranted { .. })),
                "{name}: the biscuit is never a CapabilityGranted"
            );
            let ledger = with_ledger.combat.story_ledger().expect("a live ledger");
            let unlocked = with_ledger.combat.donut_mana_view().is_some();
            assert_eq!(
                with_ledger.combat.story_holds(PRIMARY_RANGED),
                unlocked,
                "{name}: the read-through is DonutMana::unlocked() and nothing else"
            );
            assert!(
                ledger.is_empty() && ledger.receipts().is_empty(),
                "{name}: never stored"
            );
            // The ledger would mix nothing even if asked.
            let mut mixed = false;
            ledger.hash_into(&mut |_| mixed = true);
            assert!(!mixed, "{name}: nothing hashed");
        }
        // The Q10 unlock really flips the read-through (the fixture unlocks the
        // biscuit at the guild zone).
        let sim = named("q10_unlock_persists");
        assert!(sim.combat.donut_mana_view().is_some());
        assert!(
            !sim.combat
                .story_holds(CapabilityId::new("special_thrown_charge"))
        );
    }

    /// `--carl-prog` on the demo hall and the CF02 microarena reproduces the
    /// unflagged run exactly: no pin, no trigger is reachable there. The opening
    /// route corridor runs no combat state at all.
    #[test]
    fn carl_prog_flag_parity() {
        let flags = CarlProgFlags {
            prog: true,
            ..CarlProgFlags::OFF
        };
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
        let flagged = demo(Some(flags.story_setup(SceneKind::Demo)));
        assert_eq!(plain.state_hash(), flagged.state_hash());
        assert_eq!(plain.combat.events, flagged.combat.events);
        assert!(flagged.combat.story_ledger().is_none());
        // CF02 microarena, the direct plan.
        let arena = crate::cf02::arena::load_arena(&root()).unwrap();
        let cf02 = |story: Option<StorySetup>| {
            let fixture = crate::cf02::arena::fixture_for_run(0, false, crate::engineer::active());
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
        let flagged = cf02(Some(flags.story_setup(SceneKind::CfArena)));
        assert_eq!(plain.state_hash(), flagged.state_hash());
        assert_eq!(plain.combat.events, flagged.combat.events);
        // The debug flags are different: they hold a capability from tick 0 and
        // so change the hash (the opt-in is visible, never silent).
        let dynamite = CarlProgFlags {
            dynamite: true,
            ..CarlProgFlags::OFF
        };
        let held = demo(Some(dynamite.story_setup(SceneKind::Demo)));
        assert_ne!(held.state_hash(), plain_demo_hash());
        assert!(held.combat.story_holds(SPECIAL_THROWN_CHARGE));
        assert!(
            held.combat
                .events
                .iter()
                .all(|e| !matches!(e, CombatEvent::CapabilityGranted { .. })),
            "a start flag is a declared start state, not a grant"
        );

        fn plain_demo_hash() -> u64 {
            let hall = load_hall(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
            let fixture = fixture_for_beat(0);
            let scene = SceneSim::new_at(&hall, 360, fixture.carl_x).unwrap();
            let combat = CombatState::new(fixture, &hall, &scene)
                .unwrap()
                .with_demo(&DEMO_RULES);
            let mut run = FixtureSim { scene, combat };
            for _ in 0..1500 {
                let raw = autopilot(&run.combat, &run.scene);
                run.step(raw);
            }
            run.state_hash()
        }
    }
}
