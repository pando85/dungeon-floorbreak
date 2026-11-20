use std::path::PathBuf;

use super::*;
use crate::combat::{CombatEvent, MoveId};
use crate::combat_scene::{
    CP01_SINGLE_GOBLIN, CP02_KICK_CONTACT, CP02_TWO_GOBLINS, Fixture, FixtureSim,
};
use crate::demo;
use crate::input::RawInput;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn env_for(id: &str) -> AutoplayEnv {
    AutoplayEnv::new(AutoplayConfig::new(id), &root()).unwrap()
}

fn probe_env(fixture: &'static Fixture) -> AutoplayEnv {
    AutoplayEnv::new_unchecked(AutoplayConfig::new(fixture.name), &root(), fixture).unwrap()
}

fn plain_sim(fixture: &'static Fixture) -> FixtureSim {
    let kit =
        crate::env_kit::EnvKit::load(&root(), "assets/levels/floor01/corridor_abc_sample.json")
            .unwrap();
    FixtureSim::new(&kit, 360, fixture).unwrap()
}

/// The fixture path's hash and ordered events after `fixture.ticks` ticks.
fn fixture_path(fixture: &'static Fixture) -> (u64, Vec<CombatEvent>) {
    let mut sim = plain_sim(fixture);
    let mut events = Vec::new();
    for _ in 0..fixture.ticks {
        sim.step_scripted();
        events.extend_from_slice(sim.combat.take_new_events());
    }
    (sim.state_hash(), events)
}

/// A scripted `RawInput` as an ActionV1 (the fixtures only use -1/0/+1 sticks).
fn action_from_raw(raw: RawInput) -> ActionV1 {
    let horizontal = if raw.move_x < 0.0 {
        0
    } else if raw.move_x > 0.0 {
        2
    } else {
        1
    };
    ActionV1::new([
        horizontal,
        u8::from(raw.jump),
        u8::from(raw.attack),
        u8::from(raw.physical),
        u8::from(raw.ability),
        u8::from(raw.swap),
    ])
    .unwrap()
}

/// A deterministic pseudo-random action stream (LCG), for isolation tests.
fn action_stream(seed: u64, len: usize) -> Vec<ActionV1> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let bits = (state >> 33) as u8;
            ActionV1::new([
                bits % 3,
                (bits >> 2) & 1,
                (bits >> 3) & 1,
                (bits >> 4) & 1,
                0,
                0,
            ])
            .unwrap()
        })
        .collect()
}

fn count_moves(events: &[CombatEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, CombatEvent::MoveStarted { .. }))
        .count()
}

fn hold(env: &mut AutoplayEnv, action: ActionV1, ticks: usize) -> Vec<CombatEvent> {
    let mut events = Vec::new();
    for _ in 0..ticks {
        events.extend(env.step(action).unwrap().new_events);
    }
    events
}

fn primary() -> ActionV1 {
    ActionV1::new([1, 0, 1, 0, 0, 0]).unwrap()
}

/// Final `state_hash` of each fixture's scripted run, copied from
/// `EXPECTED_HASHES` (private, in the tests of `src/combat_scene.rs`).
const EXPECTED_HASHES: [(&str, u64); 3] = [
    ("cp01_single_goblin", 0xcbb3_07e8_a1a7_420f),
    ("cp02_two_goblins", 0x4f3b_150a_c8a0_c633),
    ("cp02_kick_contact", 0x8bd0_1343_f47e_d9df),
];

const FIXTURES_UNDER_PARITY: [&Fixture; 3] =
    [&CP01_SINGLE_GOBLIN, &CP02_TWO_GOBLINS, &CP02_KICK_CONTACT];

#[test]
fn parity_with_the_fixture_path_tick_by_tick() {
    for fixture in FIXTURES_UNDER_PARITY {
        let mut env = probe_env(fixture);
        env.reset(0).unwrap();
        let mut reference = plain_sim(fixture);
        let mut reference_events = Vec::new();
        for _ in 0..fixture.ticks {
            // The fixture path: the script of the next world tick.
            let raw = fixture.input(reference.scene.world.tick + 1);
            reference.step(raw);
            reference_events.extend_from_slice(reference.combat.take_new_events());
            env.step_raw(raw);
            assert_eq!(env.state_hash(), reference.state_hash(), "{}", fixture.name);
            assert_eq!(env.episode_events(), reference_events, "{}", fixture.name);
        }
        let (hash, events) = fixture_path(fixture);
        assert_eq!(env.state_hash(), hash, "{}", fixture.name);
        assert_eq!(env.episode_events(), events, "{}", fixture.name);
        let expected = EXPECTED_HASHES
            .iter()
            .find(|(name, _)| *name == fixture.name)
            .unwrap()
            .1;
        assert_eq!(env.state_hash(), expected, "{}", fixture.name);
        assert_eq!(env.tick_count(), fixture.ticks);
    }
}

#[test]
fn scripted_actions_through_step_match_the_fixture_path_until_success() {
    for fixture in [&CP01_SINGLE_GOBLIN, &CP02_TWO_GOBLINS] {
        let mut env = env_for(fixture.name);
        env.reset(0).unwrap();
        let (_, reference_events) = fixture_path(fixture);
        let mut last = None;
        for _ in 0..fixture.ticks {
            let raw = fixture.input(env.sim().scene.world.tick + 1);
            let step = env.step(action_from_raw(raw)).unwrap();
            if step.outcome.is_some() {
                last = Some(step);
                break;
            }
        }
        let last = last.expect("the scripted fight ends in success");
        assert_eq!(last.outcome, Some(Outcome::Success), "{}", fixture.name);
        assert!(last.terminated() && !last.truncated());
        assert!(demo::complete(&env.sim().combat), "{}", fixture.name);
        // The env's events are an in-order prefix of the fixture path's.
        assert_eq!(
            env.episode_events(),
            &reference_events[..env.episode_events().len()],
            "{}",
            fixture.name
        );
        assert_eq!(env.tick_count(), last.tick);
    }
}

#[test]
fn pinned_initial_hashes_match_a_fresh_sim() {
    for id in ["cp01_single_goblin", "cp02_two_goblins"] {
        let mut env = env_for(id);
        let info = env.reset(7).unwrap();
        assert_eq!(
            info.initial_hash,
            plain_sim(Fixture::by_name(id).unwrap()).state_hash()
        );
        assert_eq!(info.sim_seed, Fixture::by_name(id).unwrap().seed);
        assert!(!info.seed_affects_sim);
        assert_eq!(info.requested_seed, 7);
        assert!(!info.cf02_rules);
        assert_eq!(info.event_cursor, 0);
    }
}

#[test]
fn reset_twice_is_identical_and_carries_nothing_over() {
    let mut env = env_for("cp01_single_goblin");
    let first = env.reset(1).unwrap();
    let hash = env.state_hash();
    for action in action_stream(11, 120) {
        env.step(action).unwrap();
    }
    assert!(env.tick_count() > 0);
    let second = env.reset(1).unwrap();
    assert_eq!(first, second);
    assert_eq!(env.state_hash(), hash);
    assert_eq!(env.tick_count(), 0);
    assert!(env.episode_events().is_empty());
    assert_eq!(env.outcome(), None);
    assert_eq!(env.unavailable_counters(), UnavailableCounters::default());
    let sim = env.sim();
    assert_eq!(
        (sim.combat.deaths, sim.combat.restarts, sim.combat.elapsed),
        (0, 0, 0)
    );

    // After a finished (defeat) episode too.
    while env.step(ActionV1::NOOP).unwrap().outcome.is_none() {}
    assert_eq!(env.outcome(), Some(Outcome::Defeat));
    let third = env.reset(2).unwrap();
    assert_eq!(third.initial_hash, first.initial_hash);
    assert_eq!(third.event_cursor, 0);
    assert_eq!(env.state_hash(), hash);
    assert_eq!(env.tick_count(), 0);
    assert!(env.episode_events().is_empty());
    assert_eq!(env.sim().combat.deaths, 0);
    assert_eq!(env.outcome(), None);
}

#[test]
fn held_attack_makes_one_press_edge_until_release_and_press() {
    let mut env = env_for("cp01_single_goblin");
    env.reset(0).unwrap();
    let held = hold(&mut env, primary(), 30);
    assert_eq!(count_moves(&held), 1, "a held button is one press");
    let released = hold(&mut env, ActionV1::NOOP, 30);
    assert_eq!(count_moves(&released), 0);
    let again = hold(&mut env, primary(), 30);
    assert_eq!(count_moves(&again), 1, "release then press is a new edge");
    let jabs = env
        .episode_events()
        .iter()
        .filter(|e| {
            matches!(
                e,
                CombatEvent::MoveStarted {
                    move_id: MoveId::Jab,
                    ..
                }
            )
        })
        .count();
    assert_eq!(jabs, 2);
}

#[test]
fn action_repeat_holds_one_button_and_matches_single_steps() {
    let stream = action_stream(5, 100);
    let mut single = env_for("cp02_two_goblins");
    single.reset(0).unwrap();
    let mut repeat_config = AutoplayConfig::new("cp02_two_goblins");
    repeat_config.action_repeat = 4;
    let mut repeated = AutoplayEnv::new(repeat_config, &root()).unwrap();
    repeated.reset(0).unwrap();
    for action in stream.iter().take(25) {
        let step = repeated.step(*action).unwrap();
        assert_eq!(step.ticks_executed, 4);
        for _ in 0..4 {
            single.step(*action).unwrap();
        }
    }
    assert_eq!(repeated.state_hash(), single.state_hash());
    assert_eq!(repeated.episode_events(), single.episode_events());
    assert_eq!(repeated.tick_count(), 100);
}

#[test]
fn action_repeat_stops_mid_bundle_on_defeat() {
    let mut config = AutoplayConfig::new("cp01_single_goblin");
    config.action_repeat = 4;
    let mut env = AutoplayEnv::new(config, &root()).unwrap();
    env.reset(0).unwrap();
    let last = loop {
        let step = env.step(ActionV1::NOOP).unwrap();
        if step.outcome.is_some() {
            break step;
        }
    };
    assert_eq!(last.outcome, Some(Outcome::Defeat));
    assert_eq!(last.tick, 438);
    assert_eq!(last.ticks_executed, 2, "438 = 109 * 4 + 2");
    assert_eq!(env.tick_count(), 438);
}

#[test]
fn defeat_terminates_on_the_defeating_tick_without_a_restart() {
    let mut env = env_for("cp01_single_goblin");
    env.reset(0).unwrap();
    let last = loop {
        let step = env.step(ActionV1::NOOP).unwrap();
        if step.outcome.is_some() {
            break step;
        }
        assert!(!env.sim().combat.hero_defeated);
    };
    assert_eq!(last.outcome, Some(Outcome::Defeat));
    assert!(last.terminated() && !last.truncated());
    assert!(!last.simultaneous_clear);
    let defeat_tick = last
        .new_events
        .iter()
        .find_map(|e| match e {
            CombatEvent::HeroDefeated { tick, .. } => Some(*tick),
            _ => None,
        })
        .expect("the defeating step carries HeroDefeated");
    assert_eq!(defeat_tick, last.tick);
    let sim = env.sim();
    assert_eq!((sim.combat.deaths, sim.combat.restarts), (1, 0));
    assert!(
        !env.episode_events()
            .iter()
            .any(|e| matches!(e, CombatEvent::Restart { .. }))
    );

    // The fixture itself would restart 49 ticks later: the env never gets there.
    let mut reference = plain_sim(&CP01_SINGLE_GOBLIN);
    while !reference.combat.hero_defeated {
        reference.step(RawInput::default());
    }
    for _ in 0..49 {
        reference.step(RawInput::default());
    }
    assert_eq!(reference.combat.restarts, 1);

    assert_eq!(
        env.step(ActionV1::NOOP),
        Err(AutoplayError::EpisodeOver(Outcome::Defeat))
    );
    assert_eq!(env.outcome(), Some(Outcome::Defeat));
}

#[test]
fn timeout_while_alive_is_truncated_not_defeated() {
    let mut config = AutoplayConfig::new("cp01_single_goblin");
    config.max_ticks = 30;
    let mut env = AutoplayEnv::new(config, &root()).unwrap();
    env.reset(0).unwrap();
    for _ in 0..29 {
        assert_eq!(env.step(ActionV1::NOOP).unwrap().outcome, None);
    }
    let last = env.step(ActionV1::NOOP).unwrap();
    assert_eq!(last.outcome, Some(Outcome::Timeout));
    assert!(last.truncated() && !last.terminated());
    assert_eq!(last.tick, 30);
    assert!(!env.sim().combat.hero_defeated);
    assert_eq!(
        env.step(ActionV1::NOOP),
        Err(AutoplayError::EpisodeOver(Outcome::Timeout))
    );
}

#[test]
fn reaching_the_exit_with_hostiles_alive_is_abandoned() {
    let mut env = env_for("cp01_single_goblin");
    env.reset(0).unwrap();
    let run_jump = ActionV1::new([2, 1, 0, 0, 0, 0]).unwrap();
    let run = ActionV1::new([2, 0, 0, 0, 0, 0]).unwrap();
    let last = (0..1800)
        .find_map(|t| {
            let step = env.step(if t % 40 < 5 { run_jump } else { run }).unwrap();
            step.outcome.map(|_| step)
        })
        .expect("running right reaches the exit");
    assert_eq!(last.outcome, Some(Outcome::Abandoned));
    assert!(last.terminated());
    assert!(env.sim().scene.world.complete);
    assert!(!demo::complete(&env.sim().combat));
}

#[test]
fn step_before_reset_and_after_terminal_are_errors() {
    let mut env = env_for("cp01_single_goblin");
    assert_eq!(env.step(ActionV1::NOOP), Err(AutoplayError::NotReset));
    env.reset(0).unwrap();
    env.step(ActionV1::NOOP).unwrap();
}

#[test]
fn two_interleaved_envs_equal_two_isolated_envs() {
    let stream_a = action_stream(1, 500);
    let stream_b = action_stream(2, 500);
    let run_alone = |id: &str, stream: &[ActionV1]| {
        let mut env = env_for(id);
        env.reset(0).unwrap();
        for action in stream {
            let _ = env.step(*action); // an ended episode just stops advancing
        }
        (
            env.state_hash(),
            env.episode_events().to_vec(),
            env.tick_count(),
        )
    };
    let alone_a = run_alone("cp01_single_goblin", &stream_a);
    let alone_b = run_alone("cp02_two_goblins", &stream_b);

    let mut a = env_for("cp01_single_goblin");
    let mut b = env_for("cp02_two_goblins");
    a.reset(0).unwrap();
    b.reset(0).unwrap();
    for (x, y) in stream_a.iter().zip(&stream_b) {
        let _ = a.step(*x);
        let _ = b.step(*y);
    }
    assert_eq!(
        (a.state_hash(), a.episode_events().to_vec(), a.tick_count()),
        alone_a
    );
    assert_eq!(
        (b.state_hash(), b.episode_events().to_vec(), b.tick_count()),
        alone_b
    );
}

#[test]
fn reserved_actions_are_forwarded_and_counted_not_rewritten() {
    let mut env = env_for("cp01_single_goblin");
    env.reset(0).unwrap();
    let special = ActionV1::new([1, 0, 0, 0, 1, 0]).unwrap();
    let swap = ActionV1::new([1, 0, 0, 0, 0, 1]).unwrap();
    // special held 3 ticks, released, special 1 tick; swap held 2 ticks.
    for action in [
        special,
        special,
        special,
        ActionV1::NOOP,
        special,
        swap,
        swap,
    ] {
        env.step(action).unwrap();
    }
    assert_eq!(
        env.unavailable_counters(),
        UnavailableCounters {
            special_held_ticks: 4,
            special_presses: 2,
            swap_held_ticks: 2,
            swap_presses: 1,
        }
    );
    // The reserved buttons are the sim's real no-op: same state as idling.
    let mut idle_env = env_for("cp01_single_goblin");
    idle_env.reset(0).unwrap();
    for _ in 0..7 {
        idle_env.step(ActionV1::NOOP).unwrap();
    }
    assert_eq!(env.state_hash(), idle_env.state_hash());
}

#[test]
fn action_maps_to_the_games_raw_input() {
    let raw = ActionV1::new([0, 1, 1, 1, 1, 1]).unwrap().to_raw_input();
    assert_eq!(
        raw,
        RawInput {
            move_x: -1.0,
            jump: true,
            attack: true,
            physical: true,
            ability: true,
            pause: false,
            swap: true,
        }
    );
    assert_eq!(
        ActionV1::new([2, 0, 0, 0, 0, 0])
            .unwrap()
            .to_raw_input()
            .move_x,
        1.0
    );
    assert_eq!(ActionV1::NOOP.to_raw_input(), RawInput::default());
}

#[test]
fn invalid_actions_are_errors_never_clamped() {
    assert!(ActionV1::new([3, 0, 0, 0, 0, 0]).is_err());
    assert!(ActionV1::new([1, 2, 0, 0, 0, 0]).is_err());
    assert!(ActionV1::from_slice(&[1, 0, 0, 0, 0]).is_err());
    assert!(ActionV1::from_slice(&[1, 0, 0, 0, 0, 0, 0]).is_err());
    assert!(ActionV1::from_slice(&[-1, 0, 0, 0, 0, 0]).is_err());
    assert!(ActionV1::from_slice(&[1, 0, 0, 0, 0, 256]).is_err());
    assert_eq!(
        ActionV1::from_slice(&[1, 0, 0, 0, 0, 0]),
        Ok(ActionV1::NOOP)
    );
}

#[test]
fn unsupported_configs_are_rejected_with_clear_errors() {
    let new = |config: AutoplayConfig| AutoplayEnv::new(config, &root()).unwrap_err();
    let mut cf02 = AutoplayConfig::new("cp01_single_goblin");
    cf02.cf02_rules = true;
    assert_eq!(new(cf02), AutoplayError::Cf02Unsupported);
    for seed in [SimSeed::Fixed(3), SimSeed::FromReset] {
        let mut config = AutoplayConfig::new("cp01_single_goblin");
        config.sim_seed = seed;
        assert_eq!(new(config), AutoplayError::SimSeedUnsupported);
    }
    let mut v2 = AutoplayConfig::new("cp01_single_goblin");
    v2.action_version = 2;
    assert!(matches!(new(v2), AutoplayError::InvalidConfig(_)));
    for repeat in [0, MAX_ACTION_REPEAT + 1] {
        let mut config = AutoplayConfig::new("cp01_single_goblin");
        config.action_repeat = repeat;
        assert!(matches!(new(config), AutoplayError::InvalidConfig(_)));
    }
    let mut no_budget = AutoplayConfig::new("cp01_single_goblin");
    no_budget.max_ticks = 0;
    assert!(matches!(new(no_budget), AutoplayError::InvalidConfig(_)));
    // Existing fixtures outside the v1 roster, and unknown ids.
    for id in [
        "cp02_kick_contact",
        "cp01_goblin_attacks",
        "cp05_llama_slam",
        "nope",
    ] {
        assert!(
            matches!(
                new(AutoplayConfig::new(id)),
                AutoplayError::UnsupportedScenario(_)
            ),
            "{id}"
        );
    }
    let defaults = AutoplayConfig::new("cp01_single_goblin");
    assert_eq!(defaults.max_ticks, DEFAULT_MAX_TICKS);
    assert_eq!(defaults.max_ticks, 1800);
    assert_eq!(defaults.action_repeat, 1);
    assert_eq!(defaults.sim_seed, SimSeed::Fixture);
    assert!(!defaults.cf02_rules);
}

#[test]
fn autoplay_sources_never_use_bevy() {
    let needle = ["be", "vy"].concat();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/autoplay");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") && !path.ends_with("tests.rs") {
            let text = std::fs::read_to_string(&path).unwrap();
            assert!(!text.contains(&format!("{needle}::")), "{path:?}");
            assert!(!text.contains(&format!("use {needle}")), "{path:?}");
        }
    }
}
