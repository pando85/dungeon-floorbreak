//! Stage B tests: bots, metrics, replay and evaluation.

use std::path::PathBuf;

use super::bots::{self, BOT_IDS, Bot, SplitMix64, make_bot};
use super::config::AutoplayConfig;
use super::env::AutoplayEnv;
use super::eval::{
    EvalOptions, ReplayMode, RunIdentity, evaluate, resolve_policies, resolve_scenarios,
    run_episode,
};
use super::metrics::{EpisodeContext, EpisodeMetrics};
use super::observation::{ENEMY_DX, ObservationV1};
use super::replay::{self, Replay, ReplayError};
use super::scenario;
use super::{ActionV1, AutoplayError};
use crate::combat_scene::{CP01_SINGLE_GOBLIN, EnemySpawn, Fixture};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn identity() -> RunIdentity {
    RunIdentity {
        commit: "test-commit".into(),
        kit_hash: replay::kit_content_hash(&root()).unwrap(),
    }
}

fn play(scenario_id: &str, policy: &str, seed: u64) -> (EpisodeMetrics, Replay) {
    let mut env = AutoplayEnv::new(AutoplayConfig::new(scenario_id), &root()).unwrap();
    let mut bot = make_bot(policy).unwrap();
    let ctx = EpisodeContext {
        episode: 1,
        scenario: scenario_id.into(),
        split: "test".into(),
        policy: policy.into(),
        requested_seed: seed,
        replay: Some("episode".into()),
        action_hash: 0,
        required_enemies: 0,
    };
    let (metrics, replay) = run_episode(&mut env, bot.as_mut(), seed, ctx, &identity()).unwrap();
    (metrics, replay.unwrap())
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("autoplay-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

// ---------------------------------------------------------------- bots ---

#[test]
fn bot_trait_takes_the_policy_observation_only() {
    // The signature is the structural guarantee: a bot cannot receive
    // diagnostics, the env, the fixture or a script.
    let _: fn(&mut dyn Bot, &ObservationV1) -> ActionV1 = |bot, obs| bot.act(obs);
    let _: fn(&mut dyn Bot, u64) = |bot, seed| bot.reset(seed);
}

#[test]
fn bot_source_names_no_privileged_inputs() {
    let source = std::fs::read_to_string(root().join("src/autoplay/bots.rs")).unwrap();
    let code: String = source
        .split("#[cfg(test)]")
        .next()
        .unwrap()
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in [
        "Diagnostics",
        "diagnostics",
        "AutoplayEnv",
        "FixtureSim",
        "Fixture",
        "episode_events",
        "state_hash",
        "CombatEvent",
        "StepOutcome",
        "super::env",
        "combat_scene",
        "scenario",
    ] {
        assert!(
            !code.contains(forbidden),
            "bots.rs must not reference {forbidden}"
        );
    }
}

fn synthetic(dx_px: f32, tell: bool) -> ObservationV1 {
    let mut obs = ObservationV1 {
        own: [0.0; super::observation::SELF_LEN],
        enemies: [[0.0; super::observation::ENEMY_FEATURES]; super::observation::MAX_ENEMIES],
        enemy_mask: [0.0; super::observation::MAX_ENEMIES],
        hazards: [[0.0; super::observation::HAZARD_FEATURES]; super::observation::MAX_HAZARDS],
        hazard_mask: [0.0; super::observation::MAX_HAZARDS],
        available: [0.0; 2],
    };
    obs.own[super::observation::SELF_FACING] = 1.0;
    obs.own[super::observation::SELF_GROUNDED] = 1.0;
    obs.enemy_mask[0] = 1.0;
    obs.enemies[0][ENEMY_DX] = dx_px / 640.0;
    obs.enemies[0][super::observation::ENEMY_TELL] = f32::from(u8::from(tell));
    obs
}

#[test]
fn bots_depend_only_on_their_observations_and_seed() {
    let sequence: Vec<ObservationV1> = (0..400)
        .map(|i| synthetic(200.0 - i as f32 * 0.5, i % 40 > 30))
        .collect();
    for id in BOT_IDS {
        let run = |seed: u64| {
            let mut bot = make_bot(id).unwrap();
            bot.reset(seed);
            sequence.iter().map(|o| bot.act(o)).collect::<Vec<_>>()
        };
        assert_eq!(run(7), run(7), "{id} is not deterministic");
    }
    let mut a = make_bot(bots::RANDOM_LEGAL).unwrap();
    let mut b = make_bot(bots::RANDOM_LEGAL).unwrap();
    a.reset(1);
    b.reset(2);
    let obs = synthetic(50.0, false);
    let differs = (0..64).any(|_| a.act(&obs) != b.act(&obs));
    assert!(
        differs,
        "different seeds must give different random streams"
    );
}

#[test]
fn random_legal_never_samples_unavailable_components() {
    let mut bot = make_bot(bots::RANDOM_LEGAL).unwrap();
    bot.reset(3);
    let obs = synthetic(50.0, false);
    for _ in 0..2000 {
        let a = bot.act(&obs).as_array();
        assert_eq!((a[4], a[5]), (0, 0));
    }
    let mut rng = SplitMix64::new(5);
    assert!(rng.below(3) < 3);
}

#[test]
fn bots_only_use_forwardable_actions_and_count_none_as_unavailable() {
    for id in BOT_IDS {
        let (metrics, _) = play("cp01_single_goblin", id, 11);
        assert_eq!(metrics.action_usage.special_unavailable_ticks, 0, "{id}");
        assert_eq!(metrics.action_usage.swap_unavailable_ticks, 0, "{id}");
    }
}

#[test]
fn same_bot_same_seed_same_trajectory_and_seed_changes_random_legal_only() {
    let (a, _) = play("cp01_single_goblin", bots::APPROACH_AND_ATTACK, 1);
    let (b, _) = play("cp01_single_goblin", bots::APPROACH_AND_ATTACK, 99);
    assert_eq!(
        a.trajectory_hash, b.trajectory_hash,
        "seed must not change a scripted bot"
    );
    let (c, _) = play("cp01_single_goblin", bots::RANDOM_LEGAL, 1);
    let (d, _) = play("cp01_single_goblin", bots::RANDOM_LEGAL, 1);
    let (e, _) = play("cp01_single_goblin", bots::RANDOM_LEGAL, 2);
    assert_eq!(c.trajectory_hash, d.trajectory_hash);
    assert_ne!(c.trajectory_hash, e.trajectory_hash);
}

// -------------------------------------------------------------- replay ---

#[test]
fn a_bot_replay_verifies_in_process() {
    for id in BOT_IDS {
        let (metrics, replay) = play("cp02_two_goblins", id, 4);
        let text = replay.to_jsonl();
        let report = replay::verify(&text, &root(), "other-commit").unwrap();
        assert_eq!(report.ticks, metrics.ticks);
        assert_eq!(report.outcome, metrics.outcome);
        assert_eq!(report.final_state_hash, metrics.final_state_hash);
        assert_eq!(report.event_hash, metrics.event_hash);
        assert_eq!(Replay::from_jsonl(&text).unwrap(), replay, "round trip");
    }
}

#[test]
fn a_corrupted_action_fails_with_a_tick_and_hashes() {
    let (_, replay) = play("cp01_single_goblin", bots::APPROACH_AND_ATTACK, 0);
    let mut text = replay.to_jsonl();
    // Flip the action of tick 3 (the first ticks walk right) to a jump.
    let needle = "\"t\":3,\"a\":\"";
    let at = text.find(needle).unwrap() + needle.len();
    text.replace_range(at..at + 6, "010100");
    let err = replay::verify(&text, &root(), "x").unwrap_err();
    let message = err.to_string();
    match err {
        ReplayError::Mismatch(diffs) => {
            let first = &diffs[0];
            assert!(first.tick >= 3, "{message}");
            assert!(first.expected != first.actual);
            assert!(
                message.contains("expected") && message.contains("tick"),
                "{message}"
            );
        }
        other => panic!("expected a mismatch, got {other}"),
    }
}

#[test]
fn a_changed_end_line_fails() {
    let (_, replay) = play("cp01_single_goblin", bots::APPROACH_AND_ATTACK, 0);
    let text = replay.to_jsonl();
    let outcome = replay.end.outcome.clone();
    let wrong = if outcome == "success" {
        "defeat"
    } else {
        "success"
    };
    let text = text.replace(
        &format!("\"outcome\":\"{outcome}\""),
        &format!("\"outcome\":\"{wrong}\""),
    );
    assert!(matches!(
        replay::verify(&text, &root(), "x"),
        Err(ReplayError::Mismatch(_))
    ));
}

#[test]
fn an_incompatible_schema_or_version_is_rejected_explicitly() {
    let (_, replay) = play("cp01_single_goblin", bots::RANDOM_LEGAL, 0);
    let text = replay.to_jsonl();
    let schema = text.replace("autoplay-episode/1", "autoplay-episode/2");
    match replay::verify(&schema, &root(), "x") {
        Err(ReplayError::UnsupportedSchema(s)) => assert_eq!(s, "autoplay-episode/2"),
        other => panic!("expected a schema rejection, got {other:?}"),
    }
    for (needle, swap, field) in [
        (
            "\"action_version\":1",
            "\"action_version\":2",
            "action_version",
        ),
        ("\"obs_version\":1", "\"obs_version\":9", "obs_version"),
        ("\"cf02_rules\":false", "\"cf02_rules\":true", "cf02_rules"),
        (
            "\"action_repeat\":1",
            "\"action_repeat\":4",
            "action_repeat",
        ),
    ] {
        let changed = text.replace(needle, swap);
        match replay::verify(&changed, &root(), "x") {
            Err(ReplayError::VersionMismatch { field: f, .. }) => assert_eq!(f, field),
            other => panic!("expected a {field} rejection, got {other:?}"),
        }
    }
    let kit = text.replace(&replay.header.kit_hash, "0000000000000000");
    assert!(matches!(
        replay::verify(&kit, &root(), "x"),
        Err(ReplayError::VersionMismatch {
            field: "kit_hash",
            ..
        })
    ));
    let fingerprint = text.replace("corridor|", "corridor-v2|");
    assert!(matches!(
        replay::verify(&fingerprint, &root(), "x"),
        Err(ReplayError::VersionMismatch {
            field: "scenario_fingerprint",
            ..
        })
    ));
    assert!(matches!(
        replay::verify("", &root(), "x"),
        Err(ReplayError::Parse(..))
    ));
    assert!(matches!(
        replay::verify("{\"no\":\"schema\"}\n", &root(), "x"),
        Err(ReplayError::UnsupportedSchema(_))
    ));
}

#[test]
fn a_truncated_replay_is_not_verified() {
    let (_, replay) = play("cp01_single_goblin", bots::APPROACH_AND_ATTACK, 0);
    let text = replay.to_jsonl();
    let cut: Vec<&str> = text.lines().collect();
    let without_end = cut[..cut.len() - 1].join("\n");
    assert!(matches!(
        replay::verify(&without_end, &root(), "x"),
        Err(ReplayError::Parse(..))
    ));
}

// Checkpoint schedule (F1): every 60 ticks plus the final tick, nothing else.

fn checkpoint_ticks(text: &str) -> Vec<u64> {
    Replay::from_jsonl(text)
        .unwrap()
        .checkpoints
        .iter()
        .map(|c| c.t)
        .collect()
}

fn schedule_diffs(text: &str) -> Vec<replay::Diff> {
    match replay::verify(text, &root(), "x") {
        Err(ReplayError::Mismatch(diffs)) => diffs,
        other => panic!("expected a schedule mismatch, got {other:?}"),
    }
}

fn long_replay_text() -> String {
    let (_, replay) = play("cp01_single_goblin", bots::APPROACH_AND_ATTACK, 0);
    assert!(replay.end.ticks > 61, "the episode must pass tick 61");
    replay.to_jsonl()
}

#[test]
fn the_checkpoint_schedule_is_every_sixty_ticks_plus_the_final_tick() {
    use super::replay::expected_checkpoint_ticks as expected;
    assert_eq!(expected(0), Vec::<u64>::new());
    assert_eq!(expected(59), vec![59]);
    assert_eq!(expected(60), vec![60]);
    assert_eq!(expected(61), vec![60, 61]);
    assert_eq!(expected(130), vec![60, 120, 130]);
    let text = long_replay_text();
    let replay = Replay::from_jsonl(&text).unwrap();
    assert_eq!(checkpoint_ticks(&text), expected(replay.end.ticks));
}

#[test]
fn a_replay_with_every_checkpoint_deleted_is_rejected() {
    let text = long_replay_text();
    let stripped: Vec<&str> = text
        .lines()
        .filter(|l| !l.contains("\"type\":\"checkpoint\""))
        .collect();
    let text = stripped.join("\n");
    assert!(checkpoint_ticks(&text).is_empty());
    let diffs = schedule_diffs(&text);
    assert!(diffs.iter().any(|d| d.tick == 60), "{diffs:?}");
    assert!(diffs.iter().all(|d| d.what == "checkpoint schedule"));
}

#[test]
fn a_replay_without_its_final_checkpoint_is_rejected() {
    let text = long_replay_text();
    let end = Replay::from_jsonl(&text).unwrap().end.ticks;
    let mut lines: Vec<&str> = text.lines().collect();
    let last = lines
        .iter()
        .rposition(|l| l.contains("\"type\":\"checkpoint\""))
        .unwrap();
    assert!(lines[last].contains(&format!("\"t\":{end},")));
    lines.remove(last);
    let diffs = schedule_diffs(&lines.join("\n"));
    assert_eq!(diffs.len(), 1, "{diffs:?}");
    assert_eq!(diffs[0].tick, end);
}

#[test]
fn an_extra_or_duplicate_checkpoint_line_is_rejected() {
    let text = long_replay_text();
    let lines: Vec<&str> = text.lines().collect();
    let first = lines
        .iter()
        .find(|l| l.contains("\"type\":\"checkpoint\""))
        .unwrap();
    assert!(first.contains("\"t\":60,"));

    // An off-schedule checkpoint at tick 61.
    let off = first.replace("\"t\":60,", "\"t\":61,");
    let tick61 = lines.iter().position(|l| l.contains("\"t\":61,")).unwrap();
    let mut extra = lines.clone();
    extra.insert(tick61 + 1, &off);
    let diffs = schedule_diffs(&extra.join("\n"));
    assert!(
        diffs
            .iter()
            .any(|d| d.tick == 61 && d.actual.contains("extra")),
        "{diffs:?}"
    );

    // The tick-60 checkpoint twice.
    let at = lines.iter().position(|l| l == first).unwrap();
    let mut duplicate = lines.clone();
    duplicate.insert(at + 1, first);
    let diffs = schedule_diffs(&duplicate.join("\n"));
    assert!(
        diffs
            .iter()
            .any(|d| d.tick == 60 && d.actual.contains("duplicate")),
        "{diffs:?}"
    );
}

#[test]
fn an_altered_checkpoint_tick_or_hash_is_rejected() {
    let text = long_replay_text();
    let lines: Vec<&str> = text.lines().collect();
    let first = lines
        .iter()
        .find(|l| l.contains("\"type\":\"checkpoint\""))
        .unwrap();
    // The tick moved from 60 to 61: one missing and one extra tick.
    let moved = text.replace(first, &first.replace("\"t\":60,", "\"t\":61,"));
    let diffs = schedule_diffs(&moved);
    assert!(
        diffs
            .iter()
            .any(|d| d.tick == 60 && d.actual == "none recorded")
    );
    assert!(
        diffs
            .iter()
            .any(|d| d.tick == 61 && d.actual.contains("extra"))
    );

    // A flipped hash digit keeps the schedule and fails on the re-run.
    let value = Replay::from_jsonl(&text).unwrap().checkpoints[0]
        .state_hash
        .clone();
    let flipped = format!(
        "{}{}",
        &value[..15],
        if value.ends_with('0') { '1' } else { '0' }
    );
    let altered = text.replace(first, &first.replace(&value, &flipped));
    let diffs = schedule_diffs(&altered);
    assert_eq!(diffs[0].tick, 60);
    assert_eq!(diffs[0].what, "state_hash (checkpoint)");
}

#[test]
fn a_changed_sim_seed_or_header_type_is_rejected() {
    let (_, replay) = play("cp01_single_goblin", bots::APPROACH_AND_ATTACK, 0);
    let text = replay.to_jsonl();
    let seed = replay.header.sim_seed.clone();
    let other = if seed == "0000000000000001" {
        "0000000000000002"
    } else {
        "0000000000000001"
    };
    let changed = text.replacen(
        &format!("\"sim_seed\":\"{seed}\""),
        &format!("\"sim_seed\":\"{other}\""),
        1,
    );
    match replay::verify(&changed, &root(), "x") {
        Err(ReplayError::VersionMismatch { field, .. }) => assert_eq!(field, "sim_seed"),
        other => panic!("expected a sim_seed rejection, got {other:?}"),
    }
    let not_header = text.replacen("\"type\":\"header\"", "\"type\":\"tick\"", 1);
    assert!(matches!(
        replay::verify(&not_header, &root(), "x"),
        Err(ReplayError::Parse(1, _))
    ));
}

// ----------------------------------------------------- scenarios, errors ---

static SEVENTEEN: [EnemySpawn; 17] = [EnemySpawn {
    x: 560,
    ai: crate::combat::EnemyAi::Chase,
}; 17];
static OVERFLOW_FIXTURE: Fixture = Fixture {
    name: "ap_test_overflow",
    summary: "seventeen goblins",
    seed: 0xfeed,
    script: &[],
    enemies: &SEVENTEEN,
    ..CP01_SINGLE_GOBLIN
};

#[test]
fn a_fixture_overflow_and_unsupported_scenarios_fail_clearly() {
    let err = scenario::validate_fixture(&OVERFLOW_FIXTURE).unwrap_err();
    assert!(
        err.to_string().contains("more than the 16"),
        "unclear overflow error: {err}"
    );
    for id in ["cp02_kick_contact", "cp03_goblin_pileup", "nope"] {
        let err = AutoplayEnv::new(AutoplayConfig::new(id), &root()).unwrap_err();
        assert!(matches!(err, AutoplayError::UnsupportedScenario(_)));
        assert!(err.to_string().contains("supported:"), "{err}");
    }
    assert!(resolve_scenarios("cp02_kick_contact").is_err());
    assert!(resolve_policies("expert_human").is_err());
}

// ------------------------------------------------------------ evaluation ---

fn options(out: PathBuf, replays: ReplayMode) -> EvalOptions {
    EvalOptions {
        scenarios: resolve_scenarios("all").unwrap(),
        policies: resolve_policies("all").unwrap(),
        episodes: 3,
        base_seed: 10,
        max_ticks: 900,
        replays,
        out,
        assets_root: root(),
        commit: "test-commit".into(),
    }
}

#[test]
fn evaluation_is_deterministic_and_names_held_out_cases() {
    let (a, b) = (scratch("det-a"), scratch("det-b"));
    let first = evaluate(&options(a.clone(), ReplayMode::First)).unwrap();
    let second = evaluate(&options(b.clone(), ReplayMode::First)).unwrap();
    assert_eq!(first.text, second.text);
    for file in [
        "aggregate.json",
        "aggregate.csv",
        "episodes.json",
        "episodes.csv",
        "manifest.json",
    ] {
        let x = std::fs::read(a.join(file)).unwrap();
        let y = std::fs::read(b.join(file)).unwrap();
        assert!(!x.is_empty(), "{file} is empty");
        assert_eq!(x, y, "{file} differs between runs");
    }
    // 4 scenarios x 3 policies x 3 episodes.
    assert_eq!(first.episodes_total, 36);
    assert!(first.text.contains("36 episodes"));
    assert!(
        first
            .text
            .contains("held-out scenarios:    cp02_two_goblins, ap_single_goblin_far")
    );
    assert!(
        first
            .text
            .contains("development scenarios: cp01_single_goblin, ap_two_goblins_wide")
    );
    let manifest = std::fs::read_to_string(a.join("manifest.json")).unwrap();
    assert!(manifest.contains("\"heldout_scenarios\""));
    assert!(manifest.contains("ap_single_goblin_far"));
    let csv = std::fs::read_to_string(a.join("aggregate.csv")).unwrap();
    assert_eq!(
        csv.lines().count(),
        1 + 12 + 6,
        "12 groups and 6 split roll-ups"
    );
    assert!(
        a.join("replays/cp01_single_goblin/random_legal/episode_0001.jsonl")
            .exists()
    );
    assert!(
        !a.join("replays/cp01_single_goblin/random_legal/episode_0002.jsonl")
            .exists()
    );
    let _ = (std::fs::remove_dir_all(a), std::fs::remove_dir_all(b));
}

#[test]
fn reward_is_credited_once_per_foe_through_the_evaluate_path() {
    let out = scratch("reward");
    let mut opts = options(out.clone(), ReplayMode::None);
    opts.scenarios = resolve_scenarios("cp02_two_goblins,cp01_single_goblin").unwrap();
    evaluate(&opts).unwrap();
    let episodes: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("episodes.json")).unwrap()).unwrap();
    let mut credited = 0;
    for e in episodes.as_array().unwrap() {
        let defeated = e["enemies_defeated"].as_f64().unwrap();
        let required = e["required_enemies"].as_f64().unwrap();
        assert!(defeated <= required, "an enemy was defeated twice: {e}");
        assert_eq!(e["reward"]["enemy_defeated"].as_f64().unwrap(), defeated);
        let by_attribution: f64 = ["carl", "donut", "unknown"]
            .iter()
            .map(|k| e["defeats_by_attribution"][k].as_f64().unwrap())
            .sum();
        assert_eq!(by_attribution, defeated);
        credited += defeated as u32;
    }
    assert!(credited > 0, "no bot defeated a single goblin");
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn metrics_attribute_with_the_event_fields_only() {
    let (m, _) = play("cp01_single_goblin", bots::APPROACH_AND_ATTACK, 0);
    let carl = &m.heroes["carl"];
    let attempts: u32 = carl.moves.values().map(|s| s.attempts).sum();
    let connected: u32 = carl.moves.values().map(|s| s.connected).sum();
    let whiffs: u32 = carl.moves.values().map(|s| s.whiffs_derived).sum();
    assert_eq!(
        attempts,
        connected + whiffs,
        "every attempt connects or whiffs"
    );
    assert!(attempts > 0, "the bot never attacked");
    assert_eq!(
        m.hits_other_attribution, 0,
        "no donut or unknown moves exist"
    );
    assert_eq!(m.action_usage.special_unavailable_presses, 0);
    assert!(m.action_usage.modal_action_share > 0.0);
}

#[test]
fn csv_rows_match_their_headers() {
    use super::metrics::{
        AGGREGATE_CSV_HEADER, AggregateMetrics, EPISODE_CSV_HEADER, aggregate_csv_row,
        episode_csv_row,
    };
    let (m, _) = play("cp01_single_goblin", bots::APPROACH_AND_ATTACK, 0);
    let width = |s: &str| s.split(',').count();
    assert_eq!(width(&episode_csv_row(&m)), width(EPISODE_CSV_HEADER));
    let a = AggregateMetrics::of("cp01_single_goblin", "development", "x", &[m]);
    assert_eq!(width(&aggregate_csv_row(&a)), width(AGGREGATE_CSV_HEADER));
}

#[test]
fn a_donut_hit_never_marks_a_carl_move_instance_as_connected() {
    use super::metrics::EpisodeCollector;
    use crate::combat::{CombatEvent, MoveId};
    let mut env = AutoplayEnv::new(AutoplayConfig::new("cp01_single_goblin"), &root()).unwrap();
    env.reset(0).unwrap();
    let obs = env.observe();
    let action = ActionV1::new([0; 6]).unwrap();
    let step = env.step(action).unwrap();
    let mut collector = EpisodeCollector::new();
    // Carl's jab is instance 7 and whiffs; Donut's swat hit carries the same
    // instance number from its own counter.
    let events = [
        CombatEvent::MoveStarted {
            tick: 1,
            move_id: MoveId::Jab,
            instance: 7,
        },
        CombatEvent::Hit {
            tick: 2,
            move_id: MoveId::Swat,
            instance: 7,
            target: 0,
            damage: 0,
            health_left: 3,
            hit_stop: 0,
        },
    ];
    collector.on_step(&obs, action, &step, &events);
    let ctx = EpisodeContext {
        episode: 1,
        scenario: "cp01_single_goblin".into(),
        split: "test".into(),
        policy: "x".into(),
        requested_seed: 0,
        replay: None,
        action_hash: 0,
        required_enemies: 0,
    };
    let metrics = collector.finish(&env, ctx);
    let jab = &metrics.heroes["carl"].moves["jab"];
    assert_eq!(jab.attempts, 1);
    assert_eq!(jab.connected, 0, "a Donut hit must not connect Carl's jab");
    assert_eq!(jab.whiffs_derived, 1);
    assert_eq!(metrics.hits_other_attribution, 1);
}

#[test]
fn the_seed_range_line_survives_the_largest_seed() {
    let out = scratch("seed-max");
    let mut opts = options(out.clone(), ReplayMode::None);
    opts.scenarios = resolve_scenarios("cp01_single_goblin").unwrap();
    opts.policies = resolve_policies(bots::APPROACH_AND_ATTACK).unwrap();
    opts.episodes = 2;
    opts.base_seed = u64::MAX;
    let summary = evaluate(&opts).unwrap();
    assert!(
        summary.text.contains("seeds 18446744073709551615..0"),
        "{}",
        summary.text
    );
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn copies_of_one_trajectory_report_one_independent_episode() {
    use super::metrics::AggregateMetrics;
    let (m, _) = play("cp01_single_goblin", bots::APPROACH_AND_ATTACK, 0);
    let a = AggregateMetrics::of("cp01_single_goblin", "development", "x", &[m.clone(), m]);
    assert_eq!(a.episodes, 2);
    assert_eq!(a.independent_episodes, 1);
    assert_eq!(a.independent_episodes, a.distinct_trajectories);
}

#[test]
fn episode_recording_matches_run_episode_replay() {
    // The Python bridge records through `EpisodeRecording`; it must write the
    // same artifact as the native evaluator, byte for byte.
    for (scenario_id, policy) in [
        ("cp01_single_goblin", bots::REACTIVE_SURVIVOR),
        ("cp02_two_goblins", bots::APPROACH_AND_ATTACK),
        ("ap_single_goblin_far", bots::RANDOM_LEGAL),
    ] {
        let (_, expected) = play(scenario_id, policy, 7);
        let mut env = AutoplayEnv::new(AutoplayConfig::new(scenario_id), &root()).unwrap();
        let mut bot = make_bot(policy).unwrap();
        let reset = env.reset(7).unwrap();
        bot.reset(7);
        let mut recording = replay::EpisodeRecording::start(
            &env,
            &reset,
            &identity().commit,
            identity().kit_hash,
            policy,
            "0001",
        )
        .unwrap();
        loop {
            let action = bot.act(&env.observe());
            let step = env.step(action).unwrap();
            recording.on_step(&env, action, &step);
            if step.outcome.is_some() {
                break;
            }
        }
        assert_eq!(recording.finish(&env).to_jsonl(), expected.to_jsonl());
    }
}

#[test]
fn episode_recording_rejects_action_repeat() {
    let mut config = AutoplayConfig::new("cp01_single_goblin");
    config.action_repeat = 4;
    let mut env = AutoplayEnv::new(config, &root()).unwrap();
    let reset = env.reset(0).unwrap();
    let err = replay::EpisodeRecording::start(&env, &reset, "c", 0, "p", "0001").unwrap_err();
    assert!(matches!(err, AutoplayError::InvalidConfig(_)));
}
