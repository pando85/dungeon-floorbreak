//! AUTOPLAY-01B: the native CLI, exercised as separate processes.

use std::path::PathBuf;
use std::process::{Command, Output};

fn autoplay(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_autoplay"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("the autoplay binary runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("autoplay-cli-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn record(dir: &std::path::Path, scenario: &str, policy: &str) -> PathBuf {
    let path = dir.join("episode.jsonl");
    let out = autoplay(&[
        "run",
        "--scenario",
        scenario,
        "--policy",
        policy,
        "--seed",
        "5",
        "--out",
        path.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "run failed: {}", text(&out.stderr));
    path
}

#[test]
fn list_scenarios_names_the_roster_and_policies() {
    let out = autoplay(&["list-scenarios"]);
    assert!(out.status.success());
    let stdout = text(&out.stdout);
    for needle in [
        "cp01_single_goblin",
        "ap_two_goblins_wide",
        "cp02_two_goblins",
        "ap_single_goblin_far",
        "holdout",
        "random_legal",
        "approach_and_attack",
        "reactive_survivor",
    ] {
        assert!(stdout.contains(needle), "missing {needle}");
    }
}

#[test]
fn a_bot_replay_verifies_identically_in_a_fresh_process() {
    let dir = scratch("verify");
    for (scenario, policy) in [
        ("cp01_single_goblin", "approach_and_attack"),
        ("cp02_two_goblins", "reactive_survivor"),
        ("ap_single_goblin_far", "random_legal"),
    ] {
        let path = record(&dir, scenario, policy);
        let out = autoplay(&["replay", "--episode", path.to_str().unwrap(), "--verify"]);
        assert!(out.status.success(), "{}", text(&out.stderr));
        assert!(text(&out.stdout).contains("replay verified"));
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_corrupted_action_fails_verification_with_a_clear_diff() {
    let dir = scratch("corrupt");
    let path = record(&dir, "cp01_single_goblin", "approach_and_attack");
    let original = std::fs::read_to_string(&path).unwrap();
    // The bot walks right at the start; make tick 2 a jump left.
    let needle = "\"t\":2,\"a\":\"";
    let at = original.find(needle).unwrap() + needle.len();
    let mut corrupted = original.clone();
    corrupted.replace_range(at..at + 6, "010000");
    assert_ne!(original, corrupted);
    std::fs::write(&path, corrupted).unwrap();
    let out = autoplay(&["replay", "--episode", path.to_str().unwrap(), "--verify"]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = text(&out.stderr);
    assert!(stderr.contains("verification FAILED"), "{stderr}");
    assert!(stderr.contains("tick "), "{stderr}");
    assert!(
        stderr.contains("expected") && stderr.contains("re-run produced"),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_incompatible_schema_is_rejected_explicitly() {
    let dir = scratch("schema");
    let path = record(&dir, "cp01_single_goblin", "random_legal");
    let original = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        original.replace("autoplay-episode/1", "autoplay-episode/9"),
    )
    .unwrap();
    let out = autoplay(&["replay", "--episode", path.to_str().unwrap(), "--verify"]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = text(&out.stderr);
    assert!(
        stderr.contains("unsupported schema") && stderr.contains("autoplay-episode/9"),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_unsupported_scenario_fails_clearly() {
    for scenario in ["cp02_kick_contact", "does_not_exist"] {
        let out = autoplay(&["run", "--scenario", scenario, "--policy", "random_legal"]);
        assert_eq!(out.status.code(), Some(1));
        let stderr = text(&out.stderr);
        assert!(stderr.contains("unsupported autoplay scenario"), "{stderr}");
        assert!(stderr.contains("supported:"), "{stderr}");
    }
    let out = autoplay(&["frobnicate"]);
    assert_eq!(out.status.code(), Some(1));
    let out = autoplay(&["evaluate", "--episodes", "x", "--out", "unused"]);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn evaluate_writes_deterministic_json_and_csv_and_names_held_out_cases() {
    let (a, b) = (scratch("eval-a"), scratch("eval-b"));
    let mut summaries = Vec::new();
    for dir in [&a, &b] {
        let out = autoplay(&[
            "evaluate",
            "--episodes",
            "4",
            "--seed",
            "3",
            "--max-ticks",
            "900",
            "--replays",
            "first",
            "--out",
            dir.to_str().unwrap(),
        ]);
        assert!(out.status.success(), "{}", text(&out.stderr));
        summaries.push(text(&out.stdout));
    }
    assert_eq!(summaries[0], summaries[1]);
    let summary = &summaries[0];
    assert!(summary.contains("48 episodes"), "{summary}");
    assert!(summary.contains("held-out scenarios:    cp02_two_goblins, ap_single_goblin_far"));
    for file in [
        "aggregate.json",
        "aggregate.csv",
        "episodes.json",
        "episodes.csv",
        "manifest.json",
    ] {
        assert_eq!(
            std::fs::read(a.join(file)).unwrap(),
            std::fs::read(b.join(file)).unwrap(),
            "{file} is not byte-identical"
        );
    }
    let aggregate: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(a.join("aggregate.json")).unwrap()).unwrap();
    assert_eq!(aggregate["episodes_total"], 48);
    assert_eq!(aggregate["heldout_scenarios"][0], "cp02_two_goblins");
    let group = &aggregate["groups"][0];
    assert!(group["success_ci95_low"].is_number() && group["success_ci95_high"].is_number());
    assert_eq!(group["ci_method"], "wilson-95");
    let _ = (std::fs::remove_dir_all(a), std::fs::remove_dir_all(b));
}

#[test]
fn benchmark_reports_every_mode_with_its_build_profile() {
    let out = autoplay(&["benchmark", "--episodes", "3", "--repeat", "2"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    for mode in [
        "raw_fixture_sim",
        "env_step ",
        "env_step_observe",
        "bot_loop",
        "reset",
    ] {
        assert!(stdout.contains(mode), "missing {mode}: {stdout}");
    }
    assert!(stdout.contains("build:") && stdout.contains("cpu:"));
}
