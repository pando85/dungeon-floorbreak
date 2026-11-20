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
    text.replace_range(at..at + 6, "012100");
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
