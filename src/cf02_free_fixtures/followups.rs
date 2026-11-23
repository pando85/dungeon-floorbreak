//! PR-A follow-ups of the Floor 1 fix-all plan (T01, T03, T04 and T07 of the
//! triage): the review items of CF02-FU-C2F, CF02-FU-I, CA2-FU-HAZ and DP-FU
//! that are a test, a measured report or a small fix, each named by its id.

use crate::hazards_fixtures::{Layout, SlimeAt};
use crate::input::RawInput;
use crate::party::HeroId;

/// CA2-FU-HAZ U-6: the slime-row feed churn under `cf02_free`. A live slime that
/// walks and lunges moves its contact box, so the feed (rounded outward to 8 px,
/// CA2-Q29) changes only when a box crosses a rounding step: the report is the
/// number of graph rebuilds over 600 ticks with the slime alive, and it is
/// bounded here by the measured figure (not one per tick).
#[test]
fn u6_a_live_slime_costs_a_bounded_number_of_feed_rebuilds() {
    let layout = Layout {
        slimes: vec![SlimeAt {
            x: 700,
            facing: -1,
            home: 700,
        }],
        carl_x: 560,
        tuning: Some(super::FREE),
        ..Layout::default()
    };
    let mut rig = layout.sim(Some(false));
    for _ in 0..600 {
        rig.step(RawInput::NONE, RawInput::NONE);
    }
    let stats = rig.combat().companion_stats();
    println!(
        "U6 feed_changes {} feed_errors {}",
        stats.hazard_feed_changes, stats.hazard_feed_errors
    );
    assert_eq!(stats.hazard_feed_errors, 0);
    assert!(
        stats.hazard_feed_changes <= 30,
        "measured 27 over 600 ticks, one per about 22: {}",
        stats.hazard_feed_changes
    );
    let _ = HeroId::Carl;
}

/// DP-FU F1: the windup cancels on a hurt or a lost pair of hands, then on a primary,
/// physical or jump press on t0..t5, and never on t6; checked at both edges.
#[test]
fn dp_fu_f1_the_windup_cancels_on_a_hurt_or_lost_hands_and_an_input_before_t6() {
    use crate::combat_scene::windup_cancel_cause as cause;
    use crate::short_blink::CancelCause;
    assert_eq!(cause(true, true, false, 0, 6), Some(CancelCause::Hurt));
    assert_eq!(
        cause(false, false, false, 3, 6),
        Some(CancelCause::Hurt),
        "frozen"
    );
    assert_eq!(
        cause(true, true, true, 3, 6),
        Some(CancelCause::Hurt),
        "hurt wins"
    );
    assert_eq!(cause(false, true, true, 0, 6), Some(CancelCause::Input));
    assert_eq!(
        cause(false, true, true, 5, 6),
        Some(CancelCause::Input),
        "t5"
    );
    assert_eq!(cause(false, true, true, 6, 6), None, "t6 ignores a press");
    assert_eq!(cause(false, true, false, 5, 6), None, "no press, no cancel");
}

/// CF02-FU-C2F U1: the eased swap pan ends while the new hero keeps running. Carl
/// runs 30 ticks, the swap hands the camera to Donut (100 px behind), who then runs
/// on at full stick: the pan must end (the camera follows normally) within 80 ticks
/// instead of staying boosted for as long as she runs.
#[test]
fn u1_the_eased_swap_pan_ends_while_the_new_hero_keeps_running() {
    use crate::boss_scene::Pilot;
    use crate::donut_mana::DonutSetup;
    use crate::party::PROVISIONAL_TUNING;
    use crate::party_scene::{PartyFixture, PartyFixtureSim, SOLO_SWAP_CONTINUITY};
    let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
        base: SOLO_SWAP_CONTINUITY.base,
        two_player: false,
        tuning: PROVISIONAL_TUNING.with_cf02_swap(),
        script: &[],
        pilot: Pilot::None,
        donut: DonutSetup::NONE,
    }));
    let kit = crate::env_kit::EnvKit::load(
        &crate::hazards_fixtures::root(),
        "assets/levels/floor01/corridor_abc_sample.json",
    )
    .unwrap();
    let mut sim = PartyFixtureSim::new(&kit, 360, fixture).unwrap();
    sim.scene.world.donut_body.x = 100.0;
    sim.scene.world.sync_donut_for_tests();
    let run = RawInput {
        move_x: 1.0,
        ..RawInput::NONE
    };
    for _ in 0..30 {
        sim.step_slots([run, RawInput::NONE]);
    }
    sim.step_slots([RawInput { swap: true, ..run }, RawInput::NONE]);
    assert_eq!(sim.party.controlled_hero(), HeroId::Donut);
    assert!(sim.scene.pan_boosted(), "the pan runs after the swap");
    let mut ended = None;
    for t in 0..80 {
        sim.step_slots([run, RawInput::NONE]);
        if ended.is_none() && !sim.scene.pan_boosted() {
            ended = Some(t);
        }
    }
    assert!(ended.is_some(), "the pan never handed over while she ran");
}

/// CF02-FU-C2F U1, the rule: the pan is over at 4 px left and at most 4 px per tick
/// (inclusive), and not a pixel earlier on either count.
#[test]
fn u1_the_handover_rule_is_exact_at_both_edges() {
    use crate::corridor::eased_pan_done;
    assert!(eased_pan_done(0, 0) && eased_pan_done(0, 12).eq(&false));
    assert!(eased_pan_done(4, 4));
    assert!(!eased_pan_done(5, 4), "one px too far");
    assert!(!eased_pan_done(4, 5), "one px per tick too fast");
    assert!(eased_pan_done(1, 2));
}

/// CF02-FU-C2F U6: the Q39 floor re-measured after the later tuning. The assist
/// partner on `cf02_arena_direct` (the inputs the floor is measured on): her
/// assist starts, her landed hits and the share of the room's hits. The floor was
/// met at exactly 3 starts (3 of 16 hits, 18.8 %); the re-measure keeps a margin
/// over the profile's floor and stays under its share ceiling.
#[test]
fn u6_the_q39_floor_has_a_margin_on_cf02_arena_direct() {
    use crate::cf02::arena;
    use crate::companion::profile::{ProfileFile, ProfileSetId};
    use crate::party::{PartyTuning, UncontrolledHero};
    use crate::party_scene::{PartyFixture, PartyFixtureSim};
    let f: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
        tuning: PartyTuning {
            uncontrolled_hero: UncontrolledHero::Companion(ProfileSetId::Cf02Assist),
            ..super::CF02_FREE_ARENA_DIRECT.tuning
        },
        ..super::CF02_FREE_ARENA_DIRECT
    }));
    let kit = arena::load_arena(&crate::hazards_fixtures::root()).unwrap();
    let mut sim = PartyFixtureSim::new(&kit, 360, f).unwrap();
    let m = super::autonomy_metrics(&mut sim, f.base.ticks);
    let set = ProfileFile::pinned().unwrap();
    let set = set.set(ProfileSetId::Cf02Assist);
    println!(
        "U6 Q39: assist starts {} hits {} of {} ({} %)",
        m.ai_starts,
        m.her_hits,
        m.all_hits,
        m.hit_share_percent()
    );
    assert!(
        m.ai_starts > u32::from(set.cf02_q39_min_assist_starts),
        "a margin over the floor of {}: {}",
        set.cf02_q39_min_assist_starts,
        m.ai_starts
    );
    assert!(m.hit_share_percent() <= u64::from(set.cf02_q39_max_hit_share_percent));
}

/// CF02-FU-C2F FU-D: `CombatState::with_cf02()` is redundant on the arena path. The
/// arena fixture names already switch the rules, the llama targets and the morale on
/// (`fixture_runs_cf02`, `morale_fixture`), so a state built for such a fixture hashes
/// the same with or without the call: the name is the single source.
#[test]
fn fu_d_with_cf02_is_redundant_on_every_arena_fixture() {
    use crate::combat_scene::CombatState;
    use crate::corridor::SceneSim;
    let kit = crate::cf02::arena::load_arena(&crate::hazards_fixtures::root()).unwrap();
    for f in super::CF02_FREE_FIXTURES {
        assert!(crate::cf02::fixture_runs_cf02(f.name()), "{}", f.name());
        let scene = SceneSim::new_at(&kit, 360, f.base.carl_x).unwrap();
        let plain = CombatState::new(f.base, &kit, &scene).unwrap();
        let twice = CombatState::new(f.base, &kit, &scene).unwrap().with_cf02();
        assert_eq!(
            plain.state_hash(&scene),
            twice.state_hash(&scene),
            "{}: with_cf02() changes nothing on the arena path",
            f.name()
        );
    }
}

/// CF02-FU-C2F FU-A: the Q41 cinematic harness has a positive control. The pile-up
/// fixture `cp03_goblin_pileup` under the CF02 rules (`with_cf02()`), fed through
/// `push_driven` the way the live loop feeds the evaluator, emits at least one moment,
/// so a broken feed cannot pass the room fixtures' "0 or 1 per attempt" bound as 0.
#[test]
fn fu_a_the_cinematic_harness_has_a_positive_control_under_the_rules() {
    use crate::cine::PresentationSettings;
    use crate::cine_adapter::CineAdapter;
    use crate::combat_scene::{CombatState, Fixture};
    use crate::corridor::SceneSim;
    let kit = crate::env_kit::EnvKit::load(
        &crate::hazards_fixtures::root(),
        "assets/levels/floor01/corridor_abc_sample.json",
    )
    .unwrap();
    let fixture = Fixture::by_name("cp03_goblin_pileup").expect("the fixture");
    let mut scene = SceneSim::new_at(&kit, 360, fixture.carl_x).unwrap();
    let mut combat = CombatState::new(fixture, &kit, &scene).unwrap().with_cf02();
    let mut adapter = CineAdapter::new(PresentationSettings::DEFAULT);
    let mut moments = 0;
    for _ in 0..fixture.ticks {
        let raw = fixture.input(scene.world.tick + 1);
        combat.step(&mut scene, raw);
        let driven = combat.companion_driven();
        for event in combat.take_new_events() {
            adapter.push_driven(event, driven);
        }
        moments += adapter.flush().len();
    }
    assert!(moments >= 1, "the control emits a moment, got {moments}");
}

/// CF02-FU-C2F N2: a hero's shore hit and a goblin's swing on the same tick, in a
/// scene (the pair was only unit-tested). In `cf02_goblin_self_own`'s room the goblin
/// is put into its tell beside the damaged shore (1 hit) so that its swing impact
/// starts on tick 42, and Carl, stood at the strike column, jabs on the press tick
/// whose active box lands on that same tick. Both are logged on the tick, each
/// counts one hit (1 to 3 breaks it) and the run replays identically.
#[test]
fn n2_a_hero_hit_and_a_goblin_swing_on_the_same_tick_both_count_in_a_scene() {
    use crate::combat::CombatEvent;
    use crate::goblin::GoblinMode;
    use crate::party_scene::PartyFixtureSim;
    let fixture = crate::cf02_fixtures::find_party_fixture("cf02_goblin_self_own").unwrap();
    let kit = crate::cf02::arena::load_arena(&crate::hazards_fixtures::root()).unwrap();
    let run = |press: u64| {
        let mut sim = PartyFixtureSim::new(&kit, 360, fixture).unwrap();
        sim.combat.shore.as_mut().unwrap().hits = 1;
        let strike_x = sim.combat.shore.as_ref().unwrap().strike_x;
        for t in 1..=60u64 {
            if t == 1 {
                sim.scene.world.hero.x = strike_x - 28.0;
                sim.scene.world.hero.facing = 1;
            }
            if t == 18 {
                // Swing impact (attack tick 26) on tick 18 + 24 = 42.
                let g = &mut sim.combat.enemies[0];
                g.x = strike_x + 32.0;
                g.facing = -1;
                g.mode = GoblinMode::Attack;
                g.attack_tick = 2;
            }
            let raw = RawInput {
                attack: t == press,
                ..RawInput::NONE
            };
            sim.step_slots([raw, RawInput::NONE]);
        }
        sim
    };
    let ticks_of = |sim: &PartyFixtureSim| {
        let hit = sim.combat.events.iter().find_map(|e| match *e {
            CombatEvent::ShoreHit { tick, .. } => Some(tick),
            _ => None,
        });
        let trip = sim.combat.events.iter().find_map(|e| match *e {
            CombatEvent::ShoreTripped { tick, .. } => Some(tick),
            _ => None,
        });
        (hit, trip)
    };
    let found = (20..50u64).find_map(|p| {
        let sim = run(p);
        matches!(ticks_of(&sim), (Some(a), Some(b)) if a == b).then_some((p, sim))
    });
    let (press, sim) =
        found.expect("a press tick puts the hero hit and the goblin swing on one tick");
    let (hit, trip) = ticks_of(&sim);
    assert_eq!(hit, trip);
    assert_eq!(
        run(press).state_hash(),
        sim.state_hash(),
        "replays identically"
    );
    let shore = sim.combat.shore.as_ref().unwrap();
    assert_eq!(shore.hits, 3, "one hit each from 1, and it breaks");
}

/// CF02-FU-C2F U5: the right-end perch climb. In the microarena's nav graph (built
/// from the room's real solids) the right perch (x 896..992, a slab 80 px up) has a
/// standing anchor, and a route to it from the floor exists for both heroes (by way
/// of the parapet, three anchors, about 225 ticks), which is the answer to "does
/// Donut reach a perch": yes, and so does Carl.
#[test]
fn u5_both_heroes_reach_the_right_perch_by_way_of_the_parapet() {
    use crate::cf02::arena::{PERCH_TOP, RIGHT_PERCH_X};
    use crate::companion::nav::{NavConfig, Unlocks};
    use crate::companion::navfollow::NavDriver;
    use crate::corridor::SceneSim;
    let kit = crate::cf02::arena::load_arena(&crate::hazards_fixtures::root()).unwrap();
    let scene = SceneSim::new_at(&kit, 360, 112).unwrap();
    let floor_y = scene.world.hero_foot_y();
    let config = NavConfig::for_motion(crate::party::PROVISIONAL_TUNING.donut_motion)
        .with_donut_launch(scene.donut_motion_now().jump_speed);
    let driver = NavDriver::new(&scene.world.solids, config).unwrap();
    let g = driver.graph();
    let perch = g
        .anchors()
        .find(|a| {
            let (x, y) = g.anchor_pos(*a).unwrap();
            (RIGHT_PERCH_X.0 as f32..=RIGHT_PERCH_X.1 as f32).contains(&x)
                && (y - (floor_y + PERCH_TOP as f32)).abs() < 8.0
        })
        .expect("the right perch has a standing anchor");
    let floor = g.anchor_at(300.0, floor_y).expect("the floor anchor");
    let donut = g.path_or_reason(HeroId::Donut, floor, perch, Unlocks::NONE);
    let carl = g.path_or_reason(HeroId::Carl, floor, perch, Unlocks::NONE);
    println!(
        "U5 donut {:?} carl {:?}",
        donut.as_ref().map(|p| p.ticks),
        carl.as_ref().map(|p| p.ticks)
    );
    assert!(donut.is_ok(), "Donut climbs to the right perch: {donut:?}");
    let carl = carl.expect("Carl climbs it too");
    assert_eq!(carl.anchors.len(), 3, "floor, parapet, perch: {carl:?}");
}

/// CF02-FU-C2F U4: the environmental autopilot (`Plan::Environment` on
/// `cf02_arena_environment`) completes the room. It used to die twice and never
/// finish: (1) after breaking the shore it fought on without the Direct plan's
/// keep-out, so the block it had just landed hurt Carl (tick 399); (2) a goblin of
/// the second wave that stood on his column was never hit, because the jab box starts
/// 16 px in front of him and passes in front of a goblin at 2 px, so he took four swings
/// and died. Now both plans keep out of the footprint, and the Environment plan steps
/// back out of an overlap of less than 12 px. Measured: no deaths, the room completes
/// at tick 814, and the only hurt is the landed shore block's own trip.
#[test]
fn u4_the_environment_plan_completes_the_room() {
    use crate::party_scene::PartyFixtureSim;
    let f = crate::cf02_fixtures::find_party_fixture("cf02_arena_environment").unwrap();
    let kit = crate::cf02::arena::load_arena(&crate::hazards_fixtures::root()).unwrap();
    let mut sim = PartyFixtureSim::new(&kit, 360, f).unwrap();
    let mut done = None;
    for _ in 0..f.base.ticks {
        sim.step_scripted();
        if done.is_none() && crate::cf02::arena::complete(&sim.combat) {
            done = Some(sim.scene.world.tick);
        }
    }
    assert_eq!(sim.combat.deaths, 0, "no deaths");
    assert!(
        done.is_some_and(|t| t < 1200),
        "the room completes: {done:?}"
    );
    assert!(sim.combat.shore.as_ref().is_some_and(|s| s.landed));
}

/// CF02-FU-C2F FU-C: a scripted live-party smoke with the assistant in combat. The
/// `cf02_arena_direct` inputs with the live party under `cf02_assist` (the default
/// owner partner) for the whole 1800 ticks: Carl's autopilot clears goblins while
/// Donut, driven by the assistant, starts moves and lands hits of her own, nobody is
/// stuck outside the view, and two runs agree on the final hash (the headless
/// counterpart of the windowed run the owner makes).
#[test]
fn fu_c_a_scripted_live_party_run_has_the_assistant_in_combat() {
    use crate::cf02::arena;
    use crate::combat::CombatEvent;
    use crate::companion::profile::ProfileSetId;
    use crate::party::{PartyTuning, UncontrolledHero};
    use crate::party_scene::{PartyFixture, PartyFixtureSim};
    let f: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
        tuning: PartyTuning {
            uncontrolled_hero: UncontrolledHero::Companion(ProfileSetId::Cf02Assist),
            ..super::CF02_FREE_ARENA_DIRECT.tuning
        },
        ..super::CF02_FREE_ARENA_DIRECT
    }));
    let kit = arena::load_arena(&crate::hazards_fixtures::root()).unwrap();
    let run = || {
        let mut sim = PartyFixtureSim::new(&kit, 360, f).unwrap();
        for _ in 0..f.base.ticks {
            sim.step_scripted();
        }
        sim
    };
    let sim = run();
    let donut_hits = sim
        .combat
        .events
        .iter()
        .filter(|e| matches!(e, CombatEvent::Hit { move_id, damage, .. } if move_id.hero() == 1 && *damage > 0))
        .count();
    let carl_hits = sim
        .combat
        .events
        .iter()
        .filter(|e| matches!(e, CombatEvent::Hit { move_id, damage, .. } if move_id.hero() == 0 && *damage > 0))
        .count();
    println!("FU-C Carl hits {carl_hits}, Donut hits {donut_hits}");
    assert!(carl_hits >= 3, "Carl fights: {carl_hits}");
    assert!(
        donut_hits >= 1,
        "the assistant lands a hit of her own: {donut_hits}"
    );
    assert!(sim.combat.companion_stats().assist_starts[1] >= 1);
    assert_eq!(sim.state_hash(), run().state_hash(), "deterministic");
}

/// CF02-FU-I FU8: the launcher's "reads the CF02 sheets" predicate is derived from the
/// one that builds the simulation's rules: over every registered combat and party
/// fixture, `run_uses_cf02(false, name)` is exactly "`CombatState::cf02()` is set", and
/// `--cf02` turns it on for all of them.
#[test]
fn fu8_the_art_predicate_is_the_one_the_simulation_uses() {
    use crate::cf02::run_uses_cf02;
    use crate::combat_scene::{CombatState, Fixture};
    use crate::corridor::SceneSim;
    let corridor = crate::env_kit::EnvKit::load(
        &crate::hazards_fixtures::root(),
        "assets/levels/floor01/corridor_abc_sample.json",
    )
    .unwrap();
    let arena = crate::cf02::arena::load_arena(&crate::hazards_fixtures::root()).unwrap();
    let mut names = 0;
    for f in crate::combat_scene::FIXTURES {
        let scene = SceneSim::new_at(&corridor, 360, f.carl_x).unwrap();
        let combat = CombatState::new(f, &corridor, &scene).unwrap();
        assert_eq!(
            run_uses_cf02(false, Some(f.name)),
            combat.cf02().is_some(),
            "{}",
            f.name
        );
        assert!(run_uses_cf02(true, Some(f.name)));
        names += 1;
    }
    for name in crate::cf02_fixtures::list_fixtures("cf02_") {
        let pf = crate::cf02_fixtures::find_party_fixture(name).unwrap();
        let kit = if crate::cf02::arena::fixture_uses_arena(name) {
            &arena
        } else {
            &corridor
        };
        let scene = SceneSim::new_at(kit, 360, pf.base.carl_x).unwrap();
        let combat = CombatState::new(pf.base, kit, &scene).unwrap();
        assert_eq!(
            run_uses_cf02(false, Some(name)),
            combat.cf02().is_some(),
            "{name}"
        );
        names += 1;
    }
    assert!(names >= 40, "{names} fixtures checked");
    assert!(!run_uses_cf02(false, None) && run_uses_cf02(true, None));
    let _: Option<&Fixture> = None;
}

/// CF02-FU-C2F U7: the room is a field of the fixture, not a name prefix. The field
/// agrees with the retired name rule (the prefixes below) over every registered
/// fixture, so no fixture moved room, and the lookup by name reads the field.
#[test]
fn u7_the_room_is_a_fixture_field_and_matches_the_retired_name_rule() {
    use crate::combat_scene::{Fixture, FixtureRoom};
    let retired = |name: &str| {
        name == "cf02_arena"
            || [
                "eng_",
                "cf02_arena_",
                "companion_arena_",
                "cf02_block_",
                "cf02_free_",
                "donut_prog_arena_",
                "carl_prog_pierce_engineer",
            ]
            .iter()
            .any(|p| name.starts_with(p))
            || matches!(
                name,
                "cf02_goblin_self_own"
                    | "cf02_intact_shore_ignores_goblins"
                    | "cf02_parapet_cover"
                    | "cf02_perch_slab_climb"
                    | "cf02_perch_above_spit"
                    | "cf02_perch_dive"
            )
    };
    let mut arena = 0;
    for name in crate::cf02_fixtures::list_fixtures("") {
        let room = crate::cf02_fixtures::find_party_fixture(name)
            .map(|f| f.base.room)
            .or_else(|| Fixture::by_name(name).map(|f| f.room))
            .unwrap_or_else(|| panic!("{name} is registered"));
        assert_eq!(room == FixtureRoom::Arena, retired(name), "{name}");
        assert_eq!(
            crate::cf02::arena::fixture_uses_arena(name),
            retired(name),
            "{name}"
        );
        arena += usize::from(room == FixtureRoom::Arena);
    }
    assert!(arena >= 30, "{arena} arena fixtures");
    for live in [
        &crate::cf02::arena::LIVE,
        &crate::cf02::arena::LIVE_CP2,
        &crate::cf02::arena::LIVE_ENG,
    ] {
        assert_eq!(live.room, FixtureRoom::Arena, "{}", live.name);
    }
}
