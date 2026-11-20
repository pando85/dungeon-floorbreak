//! Episode runner and batch evaluation (AUTOPLAY-01B).
//!
//! [`run_episode`] drives one bot through one env and returns the metrics
//! and, when asked, the replay. [`evaluate`] runs the pinned roster (or a
//! chosen part of it) with the shipped bots, writes per-episode and aggregate
//! JSON and CSV under an output directory and returns the text summary.
//!
//! The output carries no wall-clock time and no paths of this machine, so two
//! runs with the same arguments on the same commit are byte-identical. A
//! deterministic bot on a fixed fixture replays the same trajectory every
//! episode; the aggregate reports `distinct_trajectories` so those copies are
//! not mistaken for independent evidence.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::action::ActionV1;
use super::bots::{BOT_IDS, BOT_SETTINGS_VERSION, Bot, make_bot};
use super::config::{ACTION_VERSION_V1, AutoplayConfig};
use super::env::AutoplayEnv;
use super::error::AutoplayError;
use super::metrics::{
    AGGREGATE_CSV_HEADER, AggregateMetrics, EPISODE_CSV_HEADER, EpisodeCollector, EpisodeContext,
    EpisodeMetrics, METRICS_SCHEMA, NOT_OBSERVABLE, aggregate_csv_row, episode_csv_row, hex,
};
use super::observation::OBSERVATION_VERSION;
use super::replay::{self, Replay, ReplayRecorder};
use super::reward::DEFAULT_REWARD_ID;
use super::scenario::{self, RosterEntry, Split};

/// Which episodes get a replay file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayMode {
    None,
    /// Only the first episode of every scenario and policy.
    First,
    All,
}

impl ReplayMode {
    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "none" => Ok(Self::None),
            "first" => Ok(Self::First),
            "all" => Ok(Self::All),
            other => Err(format!("--replays must be none, first or all, got {other}")),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::First => "first",
            Self::All => "all",
        }
    }
}

pub fn split_name(split: Split) -> &'static str {
    match split {
        Split::Development => "development",
        Split::Holdout => "holdout",
    }
}

/// Resolve `--scenario`: `all`, `dev`, `holdout` or comma-separated ids.
pub fn resolve_scenarios(spec: &str) -> Result<Vec<RosterEntry>, String> {
    let roster = scenario::roster();
    let by_split = |split: Split| {
        roster
            .iter()
            .copied()
            .filter(|e| e.split == split)
            .collect::<Vec<_>>()
    };
    match spec {
        "all" => Ok(roster.to_vec()),
        "dev" | "development" => Ok(by_split(Split::Development)),
        "holdout" | "held-out" | "heldout" => Ok(by_split(Split::Holdout)),
        list => {
            let mut out: Vec<RosterEntry> = Vec::new();
            for id in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let entry = scenario::lookup(id).map_err(|e| e.to_string())?;
                if !out.iter().any(|o| o.id == entry.id) {
                    out.push(entry);
                }
            }
            if out.is_empty() {
                return Err("--scenario names no scenario".into());
            }
            Ok(out)
        }
    }
}

/// Resolve `--policy`: `all` or comma-separated ids.
pub fn resolve_policies(spec: &str) -> Result<Vec<String>, String> {
    if spec == "all" {
        return Ok(BOT_IDS.iter().map(|s| (*s).to_string()).collect());
    }
    let mut out: Vec<String> = Vec::new();
    for id in spec.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        make_bot(id).map_err(|e| e.to_string())?;
        if !out.iter().any(|o| o == id) {
            out.push(id.to_string());
        }
    }
    if out.is_empty() {
        return Err("--policy names no policy".into());
    }
    Ok(out)
}

/// Static identity of a run: shared by every episode.
#[derive(Debug, Clone)]
pub struct RunIdentity {
    pub commit: String,
    pub kit_hash: u64,
}

/// Play one episode of `bot` on `env` from a fresh reset.
///
/// `ctx.replay` decides whether a replay is recorded: `Some(path)` records
/// and returns it (the path is only stored in the metrics, the caller writes
/// the file).
pub fn run_episode(
    env: &mut AutoplayEnv,
    bot: &mut dyn Bot,
    seed: u64,
    mut ctx: EpisodeContext,
    identity: &RunIdentity,
) -> Result<(EpisodeMetrics, Option<Replay>), AutoplayError> {
    if env.config().action_repeat != 1 {
        return Err(AutoplayError::InvalidConfig(
            "episode recording needs action_repeat = 1 (one step is one tick)".into(),
        ));
    }
    let reset = env.reset(seed)?;
    bot.reset(seed);
    let mut recorder = match ctx.replay {
        Some(_) => Some(ReplayRecorder::new(replay::make_header(
            env.config(),
            &reset,
            &identity.commit,
            identity.kit_hash,
            bot.id(),
            &format!("{:04}", ctx.episode),
        )?)),
        None => None,
    };
    let mut collector = EpisodeCollector::new();
    let mut trace: Vec<ActionV1> = Vec::new();
    let mut cursor = env.episode_events().len();
    let mut obs = env.observe();
    loop {
        let action = bot.act(&obs);
        let step = env.step(action)?;
        let events = env.episode_events();
        collector.on_step(&obs, action, &step, &events[cursor..]);
        cursor = events.len();
        trace.push(action);
        if let Some(rec) = recorder.as_mut() {
            rec.push_action(action);
            if ReplayRecorder::wants_checkpoint(step.tick) {
                rec.push_checkpoint(step.tick, collector.event_hash(), env.state_hash());
            }
        }
        if step.outcome.is_some() {
            break;
        }
        obs = env.observe();
    }
    ctx.required_enemies = reset.required_enemies;
    ctx.action_hash = replay::action_trace_hash(&trace);
    let metrics = collector.finish(env, ctx);
    let replay = recorder.map(|rec| {
        let outcome = env.outcome().map_or("timeout", |o| o.name());
        rec.finish(
            outcome,
            env.tick_count(),
            env.episode_return(),
            u64::from_str_radix(&metrics.event_hash, 16).unwrap_or(0),
            env.state_hash(),
        )
    });
    Ok((metrics, replay))
}

/// Settings of one `evaluate` run.
#[derive(Debug, Clone)]
pub struct EvalOptions {
    pub scenarios: Vec<RosterEntry>,
    pub policies: Vec<String>,
    pub episodes: u32,
    pub base_seed: u64,
    pub max_ticks: u64,
    pub replays: ReplayMode,
    pub out: PathBuf,
    pub assets_root: PathBuf,
    pub commit: String,
}

#[derive(Serialize)]
struct RosterRow {
    id: &'static str,
    split: &'static str,
    layout_fingerprint: String,
    initial_hash: String,
}

#[derive(Serialize)]
struct Manifest {
    schema: &'static str,
    metrics_schema: &'static str,
    commit: String,
    kit_hash: String,
    action_version: u32,
    observation_version: u32,
    reward_id: &'static str,
    cf02_rules: bool,
    seed_affects_sim: bool,
    bot_settings_version: u32,
    policies: Vec<String>,
    scenarios_evaluated: Vec<&'static str>,
    development_scenarios: Vec<&'static str>,
    heldout_scenarios: Vec<&'static str>,
    episodes_per_group: u32,
    episodes_total: u64,
    base_seed: u64,
    max_ticks: u64,
    replays: &'static str,
    roster: Vec<RosterRow>,
    not_observable: [&'static str; 4],
    notes: Vec<&'static str>,
}

#[derive(Serialize)]
struct AggregateFile<'a> {
    schema: &'static str,
    episodes_total: u64,
    development_scenarios: Vec<&'static str>,
    heldout_scenarios: Vec<&'static str>,
    groups: &'a [AggregateMetrics],
}

/// The text summary and the written files of an evaluation.
#[derive(Debug, Clone)]
pub struct EvalSummary {
    pub text: String,
    pub episodes_total: u64,
    pub aggregates: Vec<AggregateMetrics>,
}

fn write_file(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    fs::write(path, content).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn json<T: Serialize + ?Sized>(value: &T) -> Result<String, String> {
    let mut text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    text.push('\n');
    Ok(text)
}

/// Run the whole evaluation and write its files.
pub fn evaluate(options: &EvalOptions) -> Result<EvalSummary, String> {
    if options.episodes == 0 {
        return Err("--episodes must be at least 1".into());
    }
    let kit_hash = replay::kit_content_hash(&options.assets_root).map_err(|e| e.to_string())?;
    let identity = RunIdentity {
        commit: options.commit.clone(),
        kit_hash,
    };
    let mut episodes: Vec<EpisodeMetrics> = Vec::new();
    let mut aggregates: Vec<AggregateMetrics> = Vec::new();
    for entry in &options.scenarios {
        let mut config = AutoplayConfig::new(entry.id);
        config.max_ticks = options.max_ticks;
        let mut env = AutoplayEnv::new(config, &options.assets_root).map_err(|e| e.to_string())?;
        for policy in &options.policies {
            let mut bot = make_bot(policy).map_err(|e| e.to_string())?;
            let mut group: Vec<EpisodeMetrics> = Vec::new();
            for index in 0..options.episodes {
                let number = index + 1;
                let seed = options.base_seed.wrapping_add(u64::from(index));
                let wants_replay = match options.replays {
                    ReplayMode::None => false,
                    ReplayMode::First => index == 0,
                    ReplayMode::All => true,
                };
                let replay_path = wants_replay
                    .then(|| format!("replays/{}/{}/episode_{number:04}.jsonl", entry.id, policy));
                let ctx = EpisodeContext {
                    episode: number,
                    scenario: entry.id.into(),
                    split: split_name(entry.split).into(),
                    policy: policy.clone(),
                    requested_seed: seed,
                    replay: replay_path.clone(),
                    action_hash: 0,
                    required_enemies: 0,
                };
                let (metrics, replay) =
                    run_episode(&mut env, bot.as_mut(), seed, ctx, &identity)
                        .map_err(|e| format!("{} / {policy} episode {number}: {e}", entry.id))?;
                if let (Some(path), Some(replay)) = (replay_path, replay) {
                    write_file(&options.out.join(path), &replay.to_jsonl())?;
                }
                group.push(metrics);
            }
            aggregates.push(AggregateMetrics::of(
                entry.id,
                split_name(entry.split),
                policy,
                &group,
            ));
            episodes.extend(group);
        }
    }
    // Per policy roll-ups by split, so development and hold-out stay apart.
    for policy in &options.policies {
        for split in [Split::Development, Split::Holdout] {
            let rows: Vec<EpisodeMetrics> = episodes
                .iter()
                .filter(|e| e.policy == *policy && e.split == split_name(split))
                .cloned()
                .collect();
            if !rows.is_empty() {
                aggregates.push(AggregateMetrics::of(
                    &format!("@{}", split_name(split)),
                    split_name(split),
                    policy,
                    &rows,
                ));
            }
        }
    }
    let ids = |split: Split| -> Vec<&'static str> {
        options
            .scenarios
            .iter()
            .filter(|e| e.split == split)
            .map(|e| e.id)
            .collect()
    };
    let episodes_total = episodes.len() as u64;
    let manifest = Manifest {
        schema: "autoplay-eval-manifest/1",
        metrics_schema: METRICS_SCHEMA,
        commit: options.commit.clone(),
        kit_hash: hex(kit_hash),
        action_version: ACTION_VERSION_V1,
        observation_version: OBSERVATION_VERSION,
        reward_id: DEFAULT_REWARD_ID,
        cf02_rules: false,
        seed_affects_sim: false,
        bot_settings_version: BOT_SETTINGS_VERSION,
        policies: options.policies.clone(),
        scenarios_evaluated: options.scenarios.iter().map(|e| e.id).collect(),
        development_scenarios: ids(Split::Development),
        heldout_scenarios: ids(Split::Holdout),
        episodes_per_group: options.episodes,
        episodes_total,
        base_seed: options.base_seed,
        max_ticks: options.max_ticks,
        replays: options.replays.name(),
        roster: scenario::roster()
            .iter()
            .map(|e| RosterRow {
                id: e.id,
                split: split_name(e.split),
                layout_fingerprint: scenario::layout_fingerprint(e.fixture),
                initial_hash: hex(e.initial_hash),
            })
            .collect(),
        not_observable: NOT_OBSERVABLE,
        notes: vec![
            "The reset seed does not change the simulation (no gameplay RNG on this build); it seeds policy-side randomness only.",
            "A deterministic policy repeats the same trajectory every episode: see distinct_trajectories before reading a success interval as evidence.",
            "Outcomes describe scripted baselines, not human skill, difficulty or fun.",
        ],
    };
    write_file(&options.out.join("manifest.json"), &json(&manifest)?)?;
    write_file(&options.out.join("episodes.json"), &json(&episodes)?)?;
    let aggregate_file = AggregateFile {
        schema: METRICS_SCHEMA,
        episodes_total,
        development_scenarios: ids(Split::Development),
        heldout_scenarios: ids(Split::Holdout),
        groups: &aggregates,
    };
    write_file(&options.out.join("aggregate.json"), &json(&aggregate_file)?)?;
    let mut csv = String::from(EPISODE_CSV_HEADER);
    csv.push('\n');
    for e in &episodes {
        csv.push_str(&episode_csv_row(e));
        csv.push('\n');
    }
    write_file(&options.out.join("episodes.csv"), &csv)?;
    let mut csv = String::from(AGGREGATE_CSV_HEADER);
    csv.push('\n');
    for a in &aggregates {
        csv.push_str(&aggregate_csv_row(a));
        csv.push('\n');
    }
    write_file(&options.out.join("aggregate.csv"), &csv)?;

    let text = summary_text(options, episodes_total, &aggregates);
    Ok(EvalSummary {
        text,
        episodes_total,
        aggregates,
    })
}

fn summary_text(options: &EvalOptions, total: u64, aggregates: &[AggregateMetrics]) -> String {
    let names = |split: Split| {
        let ids: Vec<&str> = options
            .scenarios
            .iter()
            .filter(|e| e.split == split)
            .map(|e| e.id)
            .collect();
        if ids.is_empty() {
            "(none)".to_string()
        } else {
            ids.join(", ")
        }
    };
    let mut text = String::new();
    let _ = writeln!(
        text,
        "AUTOPLAY-01B evaluation: {total} episodes ({} per scenario and policy)",
        options.episodes
    );
    let _ = writeln!(
        text,
        "  development scenarios: {}",
        names(Split::Development)
    );
    let _ = writeln!(text, "  held-out scenarios:    {}", names(Split::Holdout));
    let _ = writeln!(text, "  policies: {}", options.policies.join(", "));
    let _ = writeln!(
        text,
        "  seeds {}..{} (policy-side only; the simulation has no RNG)",
        options.base_seed,
        options.base_seed + u64::from(options.episodes) - 1
    );
    let _ = writeln!(
        text,
        "{:<22} {:<12} {:<20} {:>4} {:>7} {:>16} {:>6} {:>5} {:>6}",
        "scenario", "split", "policy", "n", "success", "wilson95", "ticks", "dmg", "distinct"
    );
    for a in aggregates {
        let _ = writeln!(
            text,
            "{:<22} {:<12} {:<20} {:>4} {:>6.1}% [{:>5.1}%,{:>5.1}%] {:>6.0} {:>5.1} {:>6}",
            a.scenario,
            a.split,
            a.policy,
            a.episodes,
            a.success_rate * 100.0,
            a.success_ci95_low * 100.0,
            a.success_ci95_high * 100.0,
            a.ticks.median,
            a.damage_received.mean,
            a.distinct_trajectories
        );
    }
    let _ = writeln!(
        text,
        "distinct = distinct trajectories; 1 means the episodes are copies of one deterministic run."
    );
    text
}
