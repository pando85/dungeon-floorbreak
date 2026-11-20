//! AUTOPLAY-01B native CLI: list scenarios, run one bot, evaluate a batch,
//! replay and verify a stored episode, benchmark headless throughput.
//!
//! ```text
//! autoplay list-scenarios
//! autoplay run --scenario ID --policy ID [--seed N] [--out FILE.jsonl]
//! autoplay evaluate [--scenario all|dev|holdout|ID[,ID]] [--policy all|ID[,ID]]
//!                   [--episodes N] [--seed N] [--max-ticks N]
//!                   [--replays none|first|all] --out DIR
//! autoplay replay --episode FILE.jsonl [--verify]
//! autoplay benchmark [--scenario ID] [--episodes N] [--repeat N] [--out FILE.json]
//! ```
//!
//! Every command takes `--assets-root DIR` (default: the working directory
//! when it holds `assets/levels/floor01`, else the source checkout). No
//! Python, model, renderer or audio is involved.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use dungeon_floorbreak::autoplay::bench::{self, BenchOptions};
use dungeon_floorbreak::autoplay::bots::{BOT_IDS, BOT_SETTINGS_VERSION, make_bot};
use dungeon_floorbreak::autoplay::config::AutoplayConfig;
use dungeon_floorbreak::autoplay::env::{AutoplayEnv, CORRIDOR_LAYOUT};
use dungeon_floorbreak::autoplay::eval::{
    EvalOptions, ReplayMode, RunIdentity, evaluate, resolve_policies, resolve_scenarios,
    run_episode, split_name,
};
use dungeon_floorbreak::autoplay::metrics::EpisodeContext;
use dungeon_floorbreak::autoplay::replay::{self, Replay};
use dungeon_floorbreak::autoplay::scenario;

const USAGE: &str = "usage: autoplay <command> [options]
commands:
  list-scenarios
  run        --scenario ID --policy ID [--seed N] [--max-ticks N] [--out FILE.jsonl]
  evaluate   [--scenario all|dev|holdout|ID[,ID]] [--policy all|ID[,ID]] [--episodes N]
             [--seed N] [--max-ticks N] [--replays none|first|all] --out DIR
  replay     --episode FILE.jsonl [--verify]
  benchmark  [--scenario ID] [--episodes N] [--repeat N] [--out FILE.json]
all commands: [--assets-root DIR]";

/// Parsed `--key value` options and bare `--flag`s.
struct Args {
    values: BTreeMap<String, String>,
    flags: Vec<String>,
}

const FLAGS: [&str; 1] = ["verify"];

fn parse_args(raw: &[String]) -> Result<Args, String> {
    let mut values = BTreeMap::new();
    let mut flags = Vec::new();
    let mut iter = raw.iter();
    while let Some(arg) = iter.next() {
        let key = arg
            .strip_prefix("--")
            .ok_or_else(|| format!("unexpected argument {arg}"))?;
        if FLAGS.contains(&key) {
            flags.push(key.to_string());
        } else {
            let value = iter
                .next()
                .ok_or_else(|| format!("--{key} needs a value"))?;
            if values.insert(key.to_string(), value.clone()).is_some() {
                return Err(format!("--{key} given twice"));
            }
        }
    }
    Ok(Args { values, flags })
}

impl Args {
    fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    fn require(&self, key: &str) -> Result<&str, String> {
        self.get(key).ok_or_else(|| format!("--{key} is required"))
    }

    fn number<T: std::str::FromStr>(&self, key: &str, default: T) -> Result<T, String> {
        match self.get(key) {
            None => Ok(default),
            Some(text) => text
                .parse()
                .map_err(|_| format!("--{key} must be a number, got {text}")),
        }
    }

    fn flag(&self, key: &str) -> bool {
        self.flags.iter().any(|f| f == key)
    }

    fn check_known(&self, allowed: &[&str]) -> Result<(), String> {
        for key in self.values.keys().chain(self.flags.iter()) {
            if key != "assets-root" && !allowed.contains(&key.as_str()) {
                return Err(format!("unknown option --{key} for this command"));
            }
        }
        Ok(())
    }

    fn assets_root(&self) -> PathBuf {
        if let Some(root) = self.get("assets-root") {
            return PathBuf::from(root);
        }
        if Path::new(CORRIDOR_LAYOUT).exists() {
            return PathBuf::from(".");
        }
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }
}

/// The commit of the working tree, `GITHUB_SHA`, or `unknown`.
fn current_commit() -> String {
    if let Ok(output) = Command::new("git").args(["rev-parse", "HEAD"]).output() {
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if output.status.success() && text.len() >= 7 {
            return text;
        }
    }
    std::env::var("GITHUB_SHA").unwrap_or_else(|_| "unknown".into())
}

fn list_scenarios() {
    println!("scenarios (v1 roster, Carl-only corridor fights):");
    for entry in scenario::roster() {
        println!(
            "  {:<22} {:<12} enemies {} initial_hash {:016x}",
            entry.id,
            split_name(entry.split),
            entry.fixture.enemies.len(),
            entry.initial_hash
        );
        println!(
            "      layout: {}",
            scenario::layout_fingerprint(entry.fixture)
        );
    }
    println!("policies (settings version {BOT_SETTINGS_VERSION}):");
    for id in BOT_IDS {
        println!("  {id}");
    }
    println!("any other fixture is rejected as unsupported.");
}

fn run_one(args: &Args) -> Result<(), String> {
    args.check_known(&["scenario", "policy", "seed", "max-ticks", "out"])?;
    let scenario_id = args.require("scenario")?;
    let policy = args.require("policy")?;
    let seed: u64 = args.number("seed", 0)?;
    let root = args.assets_root();
    let entry = scenario::lookup(scenario_id).map_err(|e| e.to_string())?;
    let mut config = AutoplayConfig::new(entry.id);
    config.max_ticks = args.number("max-ticks", config.max_ticks)?;
    let mut env = AutoplayEnv::new(config, &root).map_err(|e| e.to_string())?;
    let mut bot = make_bot(policy).map_err(|e| e.to_string())?;
    let identity = RunIdentity {
        commit: current_commit(),
        kit_hash: replay::kit_content_hash(&root).map_err(|e| e.to_string())?,
    };
    let out = args.get("out");
    let ctx = EpisodeContext {
        episode: 1,
        scenario: entry.id.into(),
        split: split_name(entry.split).into(),
        policy: policy.into(),
        requested_seed: seed,
        replay: out.map(|_| "episode".to_string()),
        action_hash: 0,
        required_enemies: 0,
    };
    let (metrics, recorded) =
        run_episode(&mut env, bot.as_mut(), seed, ctx, &identity).map_err(|e| e.to_string())?;
    if let (Some(path), Some(recorded)) = (out, recorded) {
        std::fs::write(path, recorded.to_jsonl())
            .map_err(|e| format!("cannot write {path}: {e}"))?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&metrics).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn run_evaluate(args: &Args) -> Result<(), String> {
    args.check_known(&[
        "scenario",
        "policy",
        "episodes",
        "seed",
        "max-ticks",
        "replays",
        "out",
    ])?;
    let out = PathBuf::from(args.require("out")?);
    let options = EvalOptions {
        scenarios: resolve_scenarios(args.get("scenario").unwrap_or("all"))?,
        policies: resolve_policies(args.get("policy").unwrap_or("all"))?,
        episodes: args.number("episodes", 20)?,
        base_seed: args.number("seed", 0)?,
        max_ticks: args.number("max-ticks", AutoplayConfig::new("x").max_ticks)?,
        replays: ReplayMode::parse(args.get("replays").unwrap_or("all"))?,
        out,
        assets_root: args.assets_root(),
        commit: current_commit(),
    };
    let summary = evaluate(&options)?;
    print!("{}", summary.text);
    println!("wrote manifest.json, episodes.{{json,csv}}, aggregate.{{json,csv}} and replays/");
    Ok(())
}

fn run_replay(args: &Args) -> Result<bool, String> {
    args.check_known(&["episode", "verify"])?;
    let path = args.require("episode")?;
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    if !args.flag("verify") {
        let replay = Replay::from_jsonl(&text).map_err(|e| e.to_string())?;
        let h = &replay.header;
        println!(
            "{}: scenario {} policy {} seed {} commit {}\n  {} ticks, outcome {}, {} checkpoints (pass --verify to re-run)",
            h.schema,
            h.scenario,
            h.policy,
            h.requested_seed,
            h.commit,
            replay.end.ticks,
            replay.end.outcome,
            replay.checkpoints.len()
        );
        return Ok(true);
    }
    match replay::verify(&text, &args.assets_root(), &current_commit()) {
        Ok(report) => {
            println!(
                "replay verified: {} / {}: {} ticks, outcome {}, {} checkpoints identical\n  final state_hash {} event_hash {}",
                report.scenario,
                report.policy,
                report.ticks,
                report.outcome,
                report.checkpoints,
                report.final_state_hash,
                report.event_hash
            );
            if report.recorded_commit != report.current_commit {
                println!(
                    "  note: recorded on commit {}, verified on {}",
                    report.recorded_commit, report.current_commit
                );
            }
            Ok(true)
        }
        Err(e) => {
            eprintln!("{e}");
            Ok(false)
        }
    }
}

fn run_benchmark(args: &Args) -> Result<(), String> {
    args.check_known(&["scenario", "episodes", "repeat", "out"])?;
    let options = BenchOptions {
        scenario: args.get("scenario").unwrap_or("cp01_single_goblin").into(),
        episodes: args.number("episodes", 200)?,
        repeat: args.number("repeat", 5)?,
        assets_root: args.assets_root(),
    };
    let report = bench::run(&options)?;
    print!("{}", report.to_text());
    if let Some(path) = args.get("out") {
        let mut text = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
        text.push('\n');
        std::fs::write(path, text).map_err(|e| format!("cannot write {path}: {e}"))?;
    }
    Ok(())
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let Some((command, rest)) = raw.split_first() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let args = match parse_args(rest) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("error: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let result = match command.as_str() {
        "list-scenarios" => {
            list_scenarios();
            Ok(true)
        }
        "run" => run_one(&args).map(|()| true),
        "evaluate" => run_evaluate(&args).map(|()| true),
        "replay" => run_replay(&args),
        "benchmark" => run_benchmark(&args).map(|()| true),
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            Ok(true)
        }
        other => Err(format!("unknown command {other}")),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
