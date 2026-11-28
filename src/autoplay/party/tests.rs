//! 02a-A tests: parity, seat entry, swap stripping, seat isolation, terminal
//! rules, state isolation, roster and config validation (AP-Q22 to AP-Q25,
//! AP-Q29, AP-Q35).

use std::path::PathBuf;

use super::*;
use crate::autoplay::bots::{BOT_IDS, make_bot};
use crate::autoplay::env::Outcome;
use crate::autoplay::{ActionV1, AutoplayConfig, AutoplayEnv, AutoplayError, SimSeed};
use crate::combat::CombatEvent;
use crate::combat_scene::Fixture;
use crate::env_kit::EnvKit;
use crate::input::RawInput;
use crate::party::HeroId;
use crate::party_scene::{PartyFixture, PartyFixtureSim};

use scenario::COMPANION_IDS;

const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";
const FJB: &str = "follow_and_jump_bot";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn env_for(id: &str, seat: HeroId) -> PartyAutoplayEnv {
    PartyAutoplayEnv::new(AutoplayConfig::party_solo(id, seat, FJB), &root()).unwrap()
}

fn reset_env(id: &str, seat: HeroId) -> PartyAutoplayEnv {
    let mut env = env_for(id, seat);
    env.reset(0).unwrap();
    env
}

fn act(values: [u8; 6]) -> ActionV1 {
    ActionV1::new(values).unwrap()
}

/// A deterministic action stream that also presses the reserved indices 4 and
/// 5 now and then.
fn stream(seed: u64, len: usize) -> Vec<ActionV1> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let bits = (state >> 33) as u8;
            act([
                bits % 3,
                (bits >> 2) & 1,
                (bits >> 3) & 1,
                (bits >> 4) & 1,
                (bits >> 5) & 1,
                (bits >> 6) & 1,
            ])
        })
        .collect()
}

fn run_stream(env: &mut PartyAutoplayEnv, actions: &[ActionV1]) {
    for action in actions {
        let _ = env.step(*action); // an ended episode just stops advancing
    }
}

fn run_to_end(env: &mut PartyAutoplayEnv, action: ActionV1) -> PartyStepOutcome {
    loop {
        let step = env.step(action).unwrap();
        if step.outcome.is_some() {
            return step;
        }
    }
}

// ------------------------------------------------------------------ parity --

/// Literal final hashes copied from `party_scene::tests::GOLDENS`
/// (src/party_scene.rs): the seven committed party goldens.
const PARTY_GOLDENS: [(&str, u64); 7] = [
    ("duo_independent_moves", 0x81ca_65ec_029b_6e63),
    ("duo_separation_wall", 0x1f40_a48b_874e_1dc9),
    ("solo_swap_continuity", 0x0a3b_cbef_fb97_ab5b),
    ("donut_hurt_iframes", 0x6b13_a15a_e7fc_186c),
    ("donut_defeat_party", 0xd481_5dd6_9ec4_8fff),
    ("solo_guard_floors_at_one", 0x78d4_244f_d621_6569),
    ("solo_follower_dodge", 0x8f8b_7dcc_2cc5_dc20),
];

/// Literal final hashes copied from `donut_fixtures::tests::GOLDENS`
/// (src/donut_fixtures.rs): the corridor-kit `q10_*` goldens. `q10_pounce_boss`
/// runs on the arena kit and is not part of this corridor-kit parity.
const Q10_GOLDENS: [(&str, u64); 7] = [
    ("q10_swat_interrupt", 0x7151_31d0_374a_16c8),
    ("q10_swat_guard", 0x985a_1da4_907c_4771),
    ("q10_pounce_hit", 0xf7fe_d891_c0e2_f8c1),
    ("q10_pounce_miss", 0x4274_d6eb_4552_8cc1),
    ("q10_dry_press", 0x642f_dcab_7d6b_7c1a),
    ("q10_mana_regen", 0x7861_1a00_785e_f14e),
    ("q10_unlock_persists", 0xfbb2_96d7_ee82_6006),
];

fn live_reference(fixture: &'static PartyFixture) -> (u64, Vec<CombatEvent>) {
    let kit = EnvKit::load(&root(), LAYOUT).unwrap();
    let mut sim = PartyFixtureSim::new(&kit, 360, fixture).unwrap();
    sim.run_scripted(fixture.base.ticks);
    assert_eq!(sim.combat.dropped_events, 0, "the log never hit its cap");
    (sim.state_hash(), sim.combat.events.clone())
}

fn raw_path(fixture: &'static PartyFixture) -> (u64, Vec<CombatEvent>) {
    let mut env = PartyAutoplayEnv::new_unchecked(&root(), fixture).unwrap();
    for _ in 0..fixture.base.ticks {
        let sim = env.sim();
        let slots = fixture.inputs(sim.scene.world.tick + 1, &sim.combat, &sim.scene);
        env.step_raw(slots);
    }
    (env.state_hash(), env.episode_events().to_vec())
}

#[test]
fn step_raw_matches_the_party_goldens_and_the_live_path_byte_for_byte() {
    for (name, golden) in PARTY_GOLDENS.into_iter().chain(Q10_GOLDENS) {
        let fixture = PartyFixture::by_name(name).unwrap();
        let (hash, events) = raw_path(fixture);
        let (live_hash, live_events) = live_reference(fixture);
        assert_eq!(hash, live_hash, "{name}: live PartyFixtureSim reference");
        assert_eq!(hash, golden, "{name}: committed golden");
        assert_eq!(events, live_events, "{name}: ordered events");
        let (again, _) = raw_path(fixture);
        assert_eq!(again, hash, "{name}: replays");
    }
}

#[test]
fn the_parity_probe_forwards_both_slots_and_the_swap_verbatim() {
    // solo_swap_continuity swaps twice through slot 0 and relies on it.
    let fixture = PartyFixture::by_name("solo_swap_continuity").unwrap();
    let mut env = PartyAutoplayEnv::new_unchecked(&root(), fixture).unwrap();
    let mut controlled = std::collections::BTreeSet::new();
    for _ in 0..fixture.base.ticks {
        let sim = env.sim();
        let slots = fixture.inputs(sim.scene.world.tick + 1, &sim.combat, &sim.scene);
        env.step_raw(slots);
        controlled.insert(env.sim().party.controlled_hero().index());
    }
    assert_eq!(controlled.len(), 2, "the scripted swap reached the party");
}

// -------------------------------------------------------------- seat entry --

#[test]
fn reset_runs_one_entry_tick_for_both_seats_and_the_seat_is_controlled() {
    for seat in HeroId::ALL {
        for id in scenario::layout_ids() {
            let mut env = env_for(id, seat);
            let info = env.reset(7).unwrap();
            assert_eq!(env.tick_count(), 1, "{id} {seat:?}: entry is env tick 1");
            assert_eq!(env.sim().party.controlled_hero(), seat, "{id} {seat:?}");
            assert_eq!(env.sim().combat.elapsed, 1);
            assert_eq!(info.observation_version, 2);
            assert_eq!(info.requested_seed, 7);
            assert!(!info.seed_affects_sim);
            assert_eq!(info.reward_id, PARTY_REWARD_PENDING);
            assert_eq!(env.outcome(), None, "the roster cannot end on tick 1");
            // The injected entry swap is not a policy action.
            let counters = env.unavailable_counters();
            assert_eq!(counters.swap_presses, 0);
            assert_eq!(counters.swap_held_ticks, 0);
            // The reset observation is post-entry: identity flags show the seat.
            let obs = env.observe();
            assert_eq!(
                obs.own[observation::SELF_IS_CARL],
                f32::from(seat == HeroId::Carl)
            );
            assert_eq!(
                obs.own[observation::SELF_IS_DONUT],
                f32::from(seat == HeroId::Donut)
            );
            assert_eq!(
                obs.ally[observation::SELF_IS_CARL],
                f32::from(seat != HeroId::Carl)
            );
        }
    }
}

#[test]
fn the_first_policy_action_is_applied_on_env_tick_two() {
    for seat in HeroId::ALL {
        let mut env = reset_env("ap2a_single_goblin", seat);
        let step = env.step(ActionV1::NOOP).unwrap();
        assert_eq!(step.tick, 2);
        assert_eq!(env.tick_count(), 2);
    }
}

#[test]
fn the_entry_events_are_reset_events_not_step_events() {
    let env = reset_env("ap2a_single_goblin", HeroId::Donut);
    assert_eq!(env.reset_event_count(), env.episode_events().len());
    let mut env = env;
    env.step(ActionV1::NOOP).unwrap();
    assert!(env.episode_events().len() >= env.reset_event_count());
}

#[test]
fn a_seat_entry_that_does_not_reach_the_seat_is_a_reset_error() {
    // An idle entry for the Donut seat leaves Carl controlled: never a silent
    // Carl episode.
    let mut env = env_for("ap2a_single_goblin", HeroId::Donut);
    let err = env
        .reset_with_entry([RawInput::NONE, RawInput::NONE], 0)
        .unwrap_err();
    assert!(matches!(err, AutoplayError::SeatEntryFailed(_)), "{err:?}");
    assert_eq!(env.step(ActionV1::NOOP), Err(AutoplayError::NotReset));
    // A good reset afterwards works.
    env.reset(0).unwrap();
    assert_eq!(env.sim().party.controlled_hero(), HeroId::Donut);
}

#[test]
fn reset_twice_gives_the_same_start() {
    for seat in HeroId::ALL {
        let mut env = env_for("ap2a_two_goblins_wide_biscuit", seat);
        env.reset(1).unwrap();
        let (hash, obs, events) = (
            env.state_hash(),
            env.observe().to_bits(),
            env.episode_events().to_vec(),
        );
        run_stream(&mut env, &stream(5, 200));
        env.reset(2).unwrap();
        assert_eq!(env.state_hash(), hash);
        assert_eq!(env.observe().to_bits(), obs);
        assert_eq!(env.episode_events(), events.as_slice());
        assert_eq!(env.tick_count(), 1);
        assert_eq!(env.unavailable_counters(), Default::default());
    }
}

// --------------------------------------------------- swap stripping, seats --

#[test]
fn holding_swap_never_changes_the_controlled_hero() {
    for seat in HeroId::ALL {
        let mut env = reset_env("ap2a_single_goblin_far", seat);
        let held = act([1, 0, 0, 0, 0, 1]);
        for _ in 0..300 {
            if env.step(held).unwrap().outcome.is_some() {
                break;
            }
            assert_eq!(env.sim().party.controlled_hero(), seat);
        }
        let counters = env.unavailable_counters();
        assert_eq!(counters.swap_presses, 1, "one 0 to 1 transition");
        assert!(counters.swap_held_ticks >= 100, "{counters:?}");
        assert_eq!(env.observe().available, [0.0, 0.0]);
    }
}

#[test]
fn pressing_swap_repeatedly_never_changes_the_seat_and_counts_each_press() {
    for seat in HeroId::ALL {
        let mut env = reset_env("ap2a_single_goblin_far", seat);
        let (on, off) = (act([1, 0, 0, 0, 0, 1]), act([1, 0, 0, 0, 0, 0]));
        for i in 0..60 {
            env.step(if i % 2 == 0 { on } else { off }).unwrap();
            assert_eq!(env.sim().party.controlled_hero(), seat);
        }
        assert_eq!(env.unavailable_counters().swap_presses, 30);
    }
}

#[test]
fn the_seat_receives_the_policy_action_and_the_companion_hero_stays_idle() {
    // A shadow sim stepped with the spec's routing (slot 0 = the action with
    // swap stripped, slot 1 idle) must equal the env tick for tick, and on every
    // tick the routed inputs are exactly seat = action, companion = idle.
    let actions = stream(11, 400);
    for seat in HeroId::ALL {
        let mut env = reset_env("ap2a_two_goblins_wide_biscuit", seat);
        let kit = EnvKit::load(&root(), LAYOUT).unwrap();
        let fixture = scenario::lookup("ap2a_two_goblins_wide_biscuit", FJB)
            .unwrap()
            .fixture;
        let mut shadow = PartyFixtureSim::new(&kit, 360, fixture).unwrap();
        let entry = [
            RawInput {
                swap: seat == HeroId::Donut,
                ..RawInput::NONE
            },
            RawInput::NONE,
        ];
        shadow.step_slots(entry);
        assert_eq!(shadow.state_hash(), env.state_hash());
        for action in &actions {
            let expected = RawInput {
                swap: false,
                ..action.to_raw_input()
            };
            let routed = shadow.party.hero_inputs([expected, RawInput::NONE]);
            assert_eq!(routed[seat.index()], expected);
            assert_eq!(routed[seat.other().index()], RawInput::default());
            if env.step(*action).unwrap().outcome.is_some() {
                break;
            }
            shadow.step_slots([expected, RawInput::NONE]);
            assert_eq!(shadow.state_hash(), env.state_hash(), "{seat:?}");
        }
    }
}

// ---------------------------------------------------------- terminal rules --

#[test]
fn an_idle_seat_is_defeated_on_the_defeating_step_without_a_restart() {
    for seat in HeroId::ALL {
        let mut env = reset_env("ap2a_two_goblins_wide", seat);
        let last = run_to_end(&mut env, ActionV1::NOOP);
        assert_eq!(last.outcome, Some(Outcome::Defeat), "{seat:?}");
        assert!(last.terminated() && !last.truncated());
        assert_eq!(last.defeated_hero, Some(seat));
        assert!(env.sim().combat.hero_defeated);
        assert_eq!(env.sim().combat.restarts, 0, "never ran into the restart");
        assert_eq!(env.sim().party.controlled_hero(), seat);
        assert_eq!(
            env.step(ActionV1::NOOP),
            Err(AutoplayError::EpisodeOver(Outcome::Defeat))
        );
        assert_eq!(env.outcome(), Some(Outcome::Defeat));
    }
}

#[test]
fn the_companion_is_never_the_defeated_hero_under_the_roster_tuning() {
    // solo_last_hp_guard keeps the uncontrolled hero at 1 HP or more.
    for seat in HeroId::ALL {
        for id in scenario::layout_ids() {
            for seed in 1..4 {
                let mut env = reset_env(id, seat);
                let mut min_companion = u8::MAX;
                for action in stream(seed, 700) {
                    let Ok(step) = env.step(action) else { break };
                    let world = &env.sim().scene.world;
                    min_companion = min_companion.min(world.body(seat.other()).health);
                    if let Some(hero) = step.defeated_hero {
                        assert_eq!(hero, seat, "{id} {seat:?} seed {seed}");
                    }
                    if step.outcome.is_some() {
                        break;
                    }
                }
                assert!(min_companion >= 1, "{id} {seat:?} seed {seed}");
            }
        }
    }
}

#[test]
fn timeout_truncates_at_the_env_tick_counter_entry_tick_included() {
    for seat in HeroId::ALL {
        let mut config = AutoplayConfig::party_solo("ap2a_single_goblin_far", seat, FJB);
        config.max_ticks = 120;
        let mut env = PartyAutoplayEnv::new(config, &root()).unwrap();
        env.reset(0).unwrap();
        let last = run_to_end(&mut env, ActionV1::NOOP);
        assert_eq!((last.outcome, last.tick), (Some(Outcome::Timeout), 120));
        assert!(last.truncated() && !last.terminated());
        assert_eq!(last.ticks_executed, 1);
        assert_eq!(env.sim().combat.elapsed, 120);
    }
}

#[test]
fn max_ticks_two_is_accepted_and_times_out_on_the_first_policy_step() {
    for seat in HeroId::ALL {
        let mut config = AutoplayConfig::party_solo("ap2a_single_goblin_far", seat, FJB);
        config.max_ticks = 2;
        let mut env = PartyAutoplayEnv::new(config, &root()).unwrap();
        env.reset(0).unwrap();
        assert_eq!(env.outcome(), None, "the entry tick alone does not end it");
        let last = env.step(ActionV1::NOOP).unwrap();
        assert_eq!((last.outcome, last.tick), (Some(Outcome::Timeout), 2));
    }
}

#[test]
fn a_timeout_inside_an_action_repeat_bundle_stops_on_the_limit() {
    let mut config = AutoplayConfig::party_solo("ap2a_single_goblin_far", HeroId::Carl, FJB);
    config.max_ticks = 22;
    config.action_repeat = 4;
    let mut env = PartyAutoplayEnv::new(config, &root()).unwrap();
    env.reset(0).unwrap();
    let last = run_to_end(&mut env, ActionV1::NOOP);
    assert_eq!((last.outcome, last.tick), (Some(Outcome::Timeout), 22));
    assert_eq!(last.ticks_executed, 1, "1 entry + 5 * 4 + 1 = 22");
}

#[test]
fn running_to_the_exit_with_hostiles_alive_is_abandoned_for_both_seats() {
    for seat in HeroId::ALL {
        let mut env = reset_env("ap2a_single_goblin_far", seat);
        let run_jump = act([2, 1, 0, 0, 0, 0]);
        let run = act([2, 0, 0, 0, 0, 0]);
        let last = (0..2400)
            .find_map(|t| {
                let step = env.step(if t % 40 < 5 { run_jump } else { run }).ok()?;
                step.outcome.map(|_| step)
            })
            .expect("running right ends the episode");
        assert_eq!(last.outcome, Some(Outcome::Abandoned), "{seat:?}");
        assert!(env.sim().scene.world.complete);
    }
}

#[test]
fn terminal_precedence_is_defeat_over_success_over_abandoned() {
    // Defeat and a full clear on the same tick: defeat, flagged.
    let mut env = reset_env("ap2a_single_goblin_far", HeroId::Carl);
    env.mark_all_defeated();
    env.sim_mut().combat.hero_defeated = true;
    let step = env.step(ActionV1::NOOP).unwrap();
    assert_eq!(step.outcome, Some(Outcome::Defeat));
    assert!(step.simultaneous_clear);

    // Success and the exit on the same tick: success.
    let mut env = reset_env("ap2a_single_goblin_far", HeroId::Donut);
    env.mark_all_defeated();
    env.sim_mut().scene.world.complete = true;
    let step = env.step(ActionV1::NOOP).unwrap();
    assert_eq!(step.outcome, Some(Outcome::Success));
    assert!(!step.simultaneous_clear);

    // The exit alone: abandoned.
    let mut env = reset_env("ap2a_single_goblin_far", HeroId::Carl);
    env.sim_mut().scene.world.complete = true;
    assert_eq!(
        env.step(ActionV1::NOOP).unwrap().outcome,
        Some(Outcome::Abandoned)
    );
}

#[test]
fn any_instigator_clearing_the_roster_is_a_team_success() {
    // The v1 bots drive the seat; whoever lands the kills, a clear is success.
    let mut successes = 0;
    for seat in HeroId::ALL {
        for id in scenario::layout_ids() {
            let mut env = reset_env(id, seat);
            let mut bot = make_bot("approach_and_attack").unwrap();
            bot.reset(0);
            let last = loop {
                let action = bot.act(&env.observe().as_v1());
                let step = env.step(action).unwrap();
                if step.outcome.is_some() {
                    break step;
                }
            };
            if last.outcome == Some(Outcome::Success) {
                successes += 1;
                let kills = env
                    .episode_events()
                    .iter()
                    .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
                    .count();
                assert_eq!(kills, env.sim().combat.enemies.len());
            }
        }
    }
    assert!(successes > 0, "a bot clears at least one party scenario");
}

// --------------------------------------------------- bots, state isolation --

#[test]
fn the_stage_b_bots_run_unchanged_on_either_seat_and_replay_identically() {
    for seat in HeroId::ALL {
        for bot_id in BOT_IDS {
            let run = || {
                let mut env = reset_env("ap2a_two_goblins_wide_biscuit", seat);
                let mut bot = make_bot(bot_id).unwrap();
                bot.reset(3);
                for _ in 0..900 {
                    let action = bot.act(&env.observe().as_v1());
                    if env.step(action).unwrap().outcome.is_some() {
                        break;
                    }
                }
                (
                    env.state_hash(),
                    env.episode_events().to_vec(),
                    env.tick_count(),
                )
            };
            assert_eq!(run(), run(), "{bot_id} {seat:?}");
        }
    }
}

#[test]
fn interleaved_and_threaded_envs_equal_sequential_runs() {
    let streams = [stream(1, 500), stream(2, 500)];
    let seats = [HeroId::Carl, HeroId::Donut];
    let alone = |i: usize| {
        let mut env = reset_env("ap2a_two_goblins_wide_biscuit", seats[i]);
        run_stream(&mut env, &streams[i]);
        (
            env.state_hash(),
            env.episode_events().to_vec(),
            env.tick_count(),
        )
    };
    let (a, b) = (alone(0), alone(1));

    let mut env_a = reset_env("ap2a_two_goblins_wide_biscuit", seats[0]);
    let mut env_b = reset_env("ap2a_two_goblins_wide_biscuit", seats[1]);
    for (&input_a, &input_b) in streams[0].iter().zip(&streams[1]).take(500) {
        let _ = env_a.step(input_a);
        let _ = env_b.step(input_b);
    }
    assert_eq!(
        (
            env_a.state_hash(),
            env_a.episode_events().to_vec(),
            env_a.tick_count()
        ),
        a
    );
    assert_eq!(
        (
            env_b.state_hash(),
            env_b.episode_events().to_vec(),
            env_b.tick_count()
        ),
        b
    );

    let handles: Vec<_> = (0..2)
        .map(|i| {
            let streams = streams.clone();
            std::thread::spawn(move || {
                let mut env = reset_env("ap2a_two_goblins_wide_biscuit", seats[i]);
                run_stream(&mut env, &streams[i]);
                (
                    env.state_hash(),
                    env.episode_events().to_vec(),
                    env.tick_count(),
                )
            })
        })
        .collect();
    let threaded: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(threaded[0], a);
    assert_eq!(threaded[1], b);
}

// ------------------------------------------------------ roster and config --

#[test]
fn roster_layouts_names_and_fixtures_are_valid_and_unique() {
    let entries = scenario::roster(FJB).unwrap();
    assert_eq!(entries.len(), 6);
    let mut prints: Vec<String> = entries
        .iter()
        .map(|e| scenario::layout_fingerprint(e.fixture))
        .collect();
    prints.sort();
    prints.dedup();
    assert_eq!(prints.len(), entries.len(), "fingerprints are unique");
    let mut seeds: Vec<u64> = entries.iter().map(|e| e.fixture.base.seed).collect();
    seeds.sort_unstable();
    seeds.dedup();
    assert_eq!(seeds.len(), entries.len(), "distinct seeds");
    for entry in entries {
        let f = entry.fixture;
        assert!(f.base.name.starts_with("ap2a_"));
        assert!(!scenario::collides_with_game_fixtures(f.base.name));
        assert!(PartyFixture::by_name(f.base.name).is_none());
        assert!(Fixture::by_name(f.base.name).is_none());
        assert!(!f.two_player && f.script.is_empty());
        assert_eq!(f.base.ticks, 600);
        assert_eq!(entry.id, f.base.name);
        scenario::validate_fixture(f).unwrap();
        assert!(
            crate::combat_scene::FIXTURES
                .iter()
                .all(|g| g.seed != f.base.seed),
            "{} seed collides with a game fixture",
            f.base.name
        );
    }
    let dev: Vec<_> = entries
        .iter()
        .filter(|e| e.split == crate::autoplay::Split::Development)
        .map(|e| e.id)
        .collect();
    assert_eq!(
        dev,
        [
            "ap2a_single_goblin",
            "ap2a_two_goblins_wide",
            "ap2a_two_goblins_wide_biscuit"
        ]
    );
}

#[test]
fn the_biscuit_layouts_unlock_donuts_magic_and_the_others_do_not() {
    for entry in scenario::roster(FJB).unwrap() {
        let env = reset_env(entry.id, HeroId::Donut);
        let obs = env.observe();
        let biscuit = entry.id.contains("biscuit");
        assert_eq!(
            obs.own[observation::SELF_MAGIC_UNLOCKED],
            f32::from(biscuit),
            "{}",
            entry.id
        );
        assert_eq!(
            obs.own[observation::SELF_CAN_CAST],
            f32::from(biscuit),
            "{}",
            entry.id
        );
        assert_eq!(
            obs.own[observation::SELF_MANA],
            f32::from(biscuit),
            "{}",
            entry.id
        );
    }
}

#[test]
fn initial_hashes_are_pinned_and_a_fresh_sim_matches_them() {
    let kit = EnvKit::load(&root(), LAYOUT).unwrap();
    for entry in scenario::roster(FJB).unwrap() {
        assert_ne!(entry.initial_hash, 0);
        let sim = PartyFixtureSim::new(&kit, 360, entry.fixture).unwrap();
        assert_eq!(sim.state_hash(), entry.initial_hash, "{}", entry.id);
        let mut env = env_for(entry.id, HeroId::Carl);
        assert_eq!(env.reset(0).unwrap().initial_hash, entry.initial_hash);
        assert_eq!(env.initial_hash(), entry.initial_hash);
    }
}

#[test]
fn the_tuning_encoding_hash_is_pinned_and_sensitive_to_every_field() {
    use crate::party::{PROVISIONAL_TUNING, UncontrolledHero};
    let tuning = scenario::roster(FJB).unwrap()[0].fixture.tuning;
    assert_eq!(party_tuning_hash(&tuning), 0x6593_418b_436d_c999);
    assert_eq!(
        party_tuning_hash(&PROVISIONAL_TUNING),
        party_tuning_hash(&tuning)
    );
    let variants = [
        crate::party::PartyTuning {
            donut_damage_per_hit: 2,
            ..tuning
        },
        crate::party::PartyTuning {
            target_switch_px: 25,
            ..tuning
        },
        crate::party::PartyTuning {
            swap_cooldown_ticks: 21,
            ..tuning
        },
        crate::party::PartyTuning {
            regroup_ticks: 121,
            ..tuning
        },
        crate::party::PartyTuning {
            solo_last_hp_guard: false,
            ..tuning
        },
        crate::party::PartyTuning {
            donut_moves: false,
            ..tuning
        },
        crate::party::PartyTuning {
            uncontrolled_hero: UncontrolledHero::TrailFollowNeverAttacks,
            ..tuning
        },
    ];
    let mut hashes = vec![party_tuning_hash(&tuning)];
    hashes.extend(variants.iter().map(party_tuning_hash));
    hashes.sort_unstable();
    hashes.dedup();
    assert_eq!(hashes.len(), 8, "every changed field changes the hash");
}

#[test]
fn unsupported_scenarios_companions_and_configs_are_rejected_clearly() {
    // Any other scenario id: UnsupportedScenario (the v1 roster included).
    for id in [
        "cp01_single_goblin",
        "ap_two_goblins_wide",
        "solo_swap_continuity",
        "nope",
    ] {
        let err = PartyAutoplayEnv::new(AutoplayConfig::party_solo(id, HeroId::Carl, FJB), &root())
            .unwrap_err();
        assert!(
            matches!(err, AutoplayError::UnsupportedScenario(_)),
            "{id}: {err:?}"
        );
    }
    // Companions that need 01A-wire, and unknown ones.
    for id in [
        "cf02_assist",
        "companion_v0_never",
        "companion_v0_factual",
        "x",
    ] {
        let err = PartyAutoplayEnv::new(
            AutoplayConfig::party_solo("ap2a_single_goblin", HeroId::Carl, id),
            &root(),
        )
        .unwrap_err();
        assert!(
            matches!(err, AutoplayError::UnsupportedCompanion(_)),
            "{id}: {err:?}"
        );
    }
    assert_eq!(COMPANION_IDS, [FJB]);

    let party = |edit: &dyn Fn(&mut AutoplayConfig)| {
        let mut config = AutoplayConfig::party_solo("ap2a_single_goblin", HeroId::Carl, FJB);
        edit(&mut config);
        PartyAutoplayEnv::new(config, &root()).unwrap_err()
    };
    assert_eq!(
        party(&|c| c.cf02_rules = true),
        AutoplayError::Cf02Unsupported
    );
    assert_eq!(
        party(&|c| c.sim_seed = SimSeed::Fixed(3)),
        AutoplayError::SimSeedUnsupported
    );
    assert_eq!(
        party(&|c| c.sim_seed = SimSeed::FromReset),
        AutoplayError::SimSeedUnsupported
    );
    for edit in [
        (&|c: &mut AutoplayConfig| c.obs_version = 1) as &dyn Fn(&mut AutoplayConfig),
        &|c| c.action_version = 2,
        &|c| c.max_ticks = 1,
        &|c| c.max_ticks = 0,
        &|c| c.action_repeat = 0,
        &|c| c.reward_id = "combat-v1".into(),
    ] {
        assert!(matches!(party(edit), AutoplayError::InvalidConfig(_)));
    }
}

#[test]
fn the_two_envs_reject_each_others_modes() {
    let err = AutoplayEnv::new(
        AutoplayConfig::party_solo("ap2a_single_goblin", HeroId::Carl, FJB),
        &root(),
    )
    .unwrap_err();
    match err {
        AutoplayError::InvalidConfig(msg) => assert!(msg.contains("PartyAutoplayEnv"), "{msg}"),
        other => panic!("{other:?}"),
    }
    let err =
        PartyAutoplayEnv::new(AutoplayConfig::new("cp01_single_goblin"), &root()).unwrap_err();
    match err {
        AutoplayError::InvalidConfig(msg) => assert!(msg.contains("AutoplayEnv"), "{msg}"),
        other => panic!("{other:?}"),
    }
    // A solo config with a party obs_version or the party reward is invalid.
    let mut solo = AutoplayConfig::new("cp01_single_goblin");
    solo.obs_version = 2;
    assert!(matches!(
        solo.validate(),
        Err(AutoplayError::InvalidConfig(_))
    ));
    let mut solo = AutoplayConfig::new("cp01_single_goblin");
    solo.reward_id = PARTY_REWARD_PENDING.into();
    assert!(matches!(
        solo.validate(),
        Err(AutoplayError::InvalidConfig(_))
    ));
}

#[test]
fn the_solo_config_defaults_are_unchanged() {
    let solo = AutoplayConfig::new("cp01_single_goblin");
    assert_eq!(solo.mode, AutoplayMode::Solo);
    assert_eq!(solo.obs_version, 1);
    assert_eq!(solo.mode.name(), "solo");
    solo.validate().unwrap();
    let party = AutoplayConfig::party_solo("ap2a_single_goblin", HeroId::Donut, FJB);
    assert_eq!(party.mode.seat(), Some(HeroId::Donut));
    assert_eq!(party.mode.name(), "party_solo");
    assert_eq!(party.obs_version, 2);
    party.validate().unwrap();
    assert_eq!(seat_name(HeroId::Carl), "carl");
    assert_eq!(parse_seat("donut"), Some(HeroId::Donut));
    assert_eq!(parse_seat("x"), None);
    assert_eq!(
        HeroId::Carl.index(),
        0,
        "the seat value is the AP-Q14 instigator id"
    );
    assert_eq!(HeroId::Donut.index(), 1);
}

// ------------------------------------------------------------------ source --

fn party_sources() -> Vec<(PathBuf, String)> {
    let dir = root().join("src/autoplay/party");
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .map(|p| {
            let text = std::fs::read_to_string(&p).unwrap();
            (p, text)
        })
        .collect()
}

#[test]
fn party_sources_never_use_bevy_and_the_env_never_steps_combat_itself() {
    let needle = ["be", "vy"].concat();
    for (path, text) in party_sources() {
        if path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("tests"))
        {
            continue;
        }
        assert!(!text.contains(&format!("{needle}::")), "{path:?}");
        assert!(!text.contains(&format!("use {needle}")), "{path:?}");
        if path.ends_with("env.rs") {
            // AP-Q22: only step_slots drives the sim.
            for banned in ["step_party(", "party.restart(", "FixtureSim::restart"] {
                let code: String = text
                    .lines()
                    .filter(|l| !l.trim_start().starts_with("//"))
                    .collect::<Vec<_>>()
                    .join("\n");
                assert!(!code.contains(banned), "env.rs must not call {banned}");
            }
        }
    }
}
