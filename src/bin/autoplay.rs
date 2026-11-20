//! AUTOPLAY-01B native CLI: list scenarios, run one bot, evaluate a batch,
//! replay and verify a stored episode, benchmark headless throughput.
//!
//! ```text
//! autoplay list-scenarios
//! autoplay run --scenario ID --policy ID [--seed N] [--out FILE.jsonl]
//! autoplay evaluate [--scenario all|dev|holdout|ID[,ID]] [--policy all|ID[,ID]]
//!                   [--episodes N] [--seed N] [--max-ticks N]
//!                   [--replays none|first|all] --out DIR
//! ```
//!
//! `--replays` defaults to `first` (one replay per scenario and policy; `all`
//! writes one file per episode). Use an empty or new `--out` directory: files
//! left by an earlier run are kept and are not listed in the manifest.
//!
//! ```text
//! autoplay replay --episode FILE.jsonl [--verify]
//! autoplay benchmark [--scenario ID] [--episodes N] [--repeat N] [--out FILE.json]
//! ```
//!
//! AUTOPLAY-02a (stages A and B) adds the solo-party env to two commands:
//!
//! ```text
//! autoplay list-scenarios --mode party_solo [--companion-profile ID]
//! autoplay run --mode party_solo --seat carl|donut --scenario ap2a_ID --policy ID
//!              [--companion-profile ID] [--seed N] [--max-ticks N]
//! ```
//!
//! `--mode` defaults to `solo` (the v1 env, unchanged). `evaluate`, `replay`
//! and `benchmark` have no party mode yet: the party metrics, reward and
//! `autoplay-episode/2` replay arrive with 02a-C.
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
use dungeon_floorbreak::autoplay::party::{self, PartyAutoplayEnv};
use dungeon_floorbreak::autoplay::replay::{self, Replay};
use dungeon_floorbreak::autoplay::scenario;

const USAGE: &str = "usage: autoplay <command> [options]
commands:
  list-scenarios [--mode solo|party_solo] [--companion-profile ID]
  run        --scenario ID --policy ID [--seed N] [--max-ticks N] [--out FILE.jsonl]
             [--mode party_solo --seat carl|donut [--companion-profile ID]]
             (party_solo prints a summary; replays /2 arrive with 02a-C)
  evaluate   [--scenario all|dev|holdout|ID[,ID]] [--policy all|ID[,ID]] [--episodes N]
             [--seed N] [--max-ticks N] [--replays none|first|all] --out DIR
             (--replays defaults to first; use an empty or new DIR: files from an
              earlier run are kept and are not listed in the manifest)
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

/// The commit of the source checkout this binary was built from (read with
/// `git -C <CARGO_MANIFEST_DIR>`, so the current directory never matters),
/// `-dirty` appended when tracked files have uncommitted changes. Falls back
/// to `GITHUB_SHA`, then `unknown`.
fn current_commit() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
    };
    if let Some(output) = git(&["rev-parse", "HEAD"]) {
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.len() >= 7 {
            let dirty = git(&["status", "--porcelain", "--untracked-files=no"])
                .is_some_and(|o| !o.stdout.is_empty());
            return if dirty { format!("{text}-dirty") } else { text };
        }
    }
    std::env::var("GITHUB_SHA").unwrap_or_else(|_| "unknown".into())
}

/// `--mode`: `solo` (default) or `party_solo`.
fn party_mode(args: &Args) -> Result<bool, String> {
    match args.get("mode") {
        None | Some("solo") => Ok(false),
        Some("party_solo") => Ok(true),
        Some(other) => Err(format!("unknown --mode {other} (solo, party_solo)")),
    }
}

fn list_party_scenarios(args: &Args) -> Result<(), String> {
    let companion = args
        .get("companion-profile")
        .unwrap_or(party::scenario::FOLLOW_AND_JUMP_BOT);
    let root = args.assets_root();
    let roster = party::scenario::roster(companion).map_err(|e| e.to_string())?;
    println!("scenarios (02a party_solo roster, companion {companion}, cf02_rules false):");
    for entry in roster {
        println!(
            "  {:<32} {:<12} enemies {} donut {} initial_hash {:016x}",
            entry.id,
            split_name(entry.split),
            entry.fixture.base.enemies.len(),
            party::scenario::donut_setup_label(&entry.fixture.donut),
            entry.initial_hash
        );
        println!(
            "      layout: {}",
            party::scenario::layout_fingerprint(entry.fixture)
        );
        for seat in dungeon_floorbreak::party::HeroId::ALL {
            let config = AutoplayConfig::party_solo(entry.id, seat, companion);
            let mut env = PartyAutoplayEnv::new(config, &root).map_err(|e| e.to_string())?;
            env.reset(0).map_err(|e| e.to_string())?;
            let obs = env.observe();
            println!(
                "      reset observation, seat {:<5}: {} enemies visible, ally visible {}",
                party::seat_name(seat),
                obs.visible_enemies(),
                obs.ally_visible()
            );
        }
    }
    println!(
        "companions: {} (cf02_assist and companion_v0_* need COMPANION 01A-wire)",
        party::scenario::COMPANION_IDS.join(", ")
    );
    println!("policies (settings version {BOT_SETTINGS_VERSION}, they see the v1 projection):");
    for id in BOT_IDS {
        println!("  {id}");
    }
    Ok(())
}

fn run_party_one(args: &Args) -> Result<(), String> {
    args.check_known(&[
        "mode",
        "seat",
        "companion-profile",
        "scenario",
        "policy",
        "seed",
        "max-ticks",
        "out",
    ])?;
    if args.get("out").is_some() {
        return Err(
            "--out is not supported with --mode party_solo yet: autoplay-episode/2 arrives with 02a-C"
                .into(),
        );
    }
    let scenario_id = args.require("scenario")?;
    let policy = args.require("policy")?;
    let seat_text = args.require("seat")?;
    let seat = party::parse_seat(seat_text)
        .ok_or_else(|| format!("unknown --seat {seat_text} (carl, donut)"))?;
    let companion = args
        .get("companion-profile")
        .unwrap_or(party::scenario::FOLLOW_AND_JUMP_BOT);
    let seed: u64 = args.number("seed", 0)?;
    let mut config = AutoplayConfig::party_solo(scenario_id, seat, companion);
    config.max_ticks = args.number("max-ticks", config.max_ticks)?;
    let mut env = PartyAutoplayEnv::new(config, &args.assets_root()).map_err(|e| e.to_string())?;
    let mut bot = make_bot(policy).map_err(|e| e.to_string())?;
    let info = env.reset(seed).map_err(|e| e.to_string())?;
    bot.reset(seed);
    let last = loop {
        let action = bot.act(&env.observe().as_v1());
        let step = env.step(action).map_err(|e| e.to_string())?;
        if step.outcome.is_some() {
            break step;
        }
    };
    let unavailable = env.unavailable_counters();
    let summary = serde_json::json!({
        "mode": "party_solo",
        "scenario": scenario_id,
        "seat": party::seat_name(seat),
        "companion_profile": companion,
        "policy": policy,
        "obs_projection": "v1",
        "requested_seed": seed,
        "outcome": last.outcome.map(|o| o.name()),
        "ticks": last.tick,
        "defeated_hero": last.defeated_hero.map(party::seat_name),
        "simultaneous_clear": last.simultaneous_clear,
        "initial_hash": format!("{:016x}", info.initial_hash),
        "final_state_hash": format!("{:016x}", env.state_hash()),
        "unavailable": {
            "special_presses": unavailable.special_presses,
            "swap_presses": unavailable.swap_presses,
        },
        "obs_clipped": env.obs_stats().clipped,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?
    );
    Ok(())
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
    if party_mode(args)? {
        return run_party_one(args);
    }
    args.check_known(&["mode", "scenario", "policy", "seed", "max-ticks", "out"])?;
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
        replays: ReplayMode::parse(args.get("replays").unwrap_or("first"))?,
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
    if rest.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let args = match parse_args(rest) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("error: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let result = match command.as_str() {
        "list-scenarios" => args
            .check_known(&["mode", "companion-profile"])
            .and_then(|()| {
                if party_mode(&args)? {
                    list_party_scenarios(&args)
                } else if args.get("companion-profile").is_some() {
                    Err("--companion-profile needs --mode party_solo".into())
                } else {
                    list_scenarios();
                    Ok(())
                }
            })
            .map(|()| true),
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
