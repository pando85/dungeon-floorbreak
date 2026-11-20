//! Native episode replay: record, parse and verify (AUTOPLAY-01B, AP-Q19).
//!
//! An episode artifact is JSONL. Line 1 is the header (`schema =
//! "autoplay-episode/1"`, build and scenario identity, versions), then one
//! `tick` line per executed tick with the compact action, a `checkpoint`
//! line every [`CHECKPOINT_EVERY`] ticks and at the end (rolling event hash
//! and `state_hash`), and a last `end` line with the outcome and tick count.
//!
//! Verification rebuilds the env from reset, feeds the recorded actions and
//! compares every checkpoint. It needs no Python, model, renderer or audio.
//! A seed and a scenario id are never a substitute for the action trace, and
//! any schema or version mismatch is rejected explicitly.

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::donut_mana::content_hash;

use super::action::ActionV1;
use super::config::{ACTION_VERSION_V1, AutoplayConfig};
use super::env::{AutoplayEnv, CORRIDOR_LAYOUT, ResetInfo};
use super::error::AutoplayError;
use super::metrics::{EventHasher, fnv_mix, hex};
use super::observation::OBSERVATION_VERSION;
use super::reward::RewardProfile;
use super::scenario;

pub const REPLAY_SCHEMA: &str = "autoplay-episode/1";
/// A checkpoint line every this many executed ticks, and at the end.
pub const CHECKPOINT_EVERY: u64 = 60;

/// Content hash of the Floor 01 corridor layout and the kit contract it names
/// (FNV-1a 64 via the repository's own helper, no new crate).
pub fn kit_content_hash(assets_root: &Path) -> Result<u64, AutoplayError> {
    let read = |path: &Path| {
        std::fs::read_to_string(path)
            .map_err(|e| AutoplayError::Sim(format!("cannot read {}: {e}", path.display())))
    };
    let layout_text = read(&assets_root.join(CORRIDOR_LAYOUT))?;
    let value: serde_json::Value = serde_json::from_str(&layout_text)
        .map_err(|e| AutoplayError::Sim(format!("layout is not valid JSON: {e}")))?;
    let contract = value
        .get("kit_contract")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AutoplayError::Sim("layout names no kit_contract".into()))?;
    let contract_text = read(&assets_root.join(contract))?;
    Ok(fnv_mix(
        content_hash(&layout_text),
        content_hash(&contract_text),
    ))
}

/// Line 1 of an episode artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Header {
    #[serde(rename = "type")]
    pub kind: String,
    pub schema: String,
    pub run_id: String,
    /// Git commit of the build that produced the artifact (`unknown` when
    /// the tool could not tell). A different commit is allowed on verify.
    pub commit: String,
    pub scenario: String,
    /// Layout fingerprint of the scenario (not its name, script or ticks).
    pub scenario_fingerprint: String,
    /// 64-bit content hash of the corridor layout and kit contract, hex.
    pub kit_hash: String,
    pub action_version: u32,
    pub obs_version: u32,
    pub reward_id: String,
    pub cf02_rules: bool,
    /// The fixture's own constant, hex (no gameplay RNG exists on this build).
    pub sim_seed: String,
    pub requested_seed: u64,
    pub policy: String,
    pub bot_settings_version: u32,
    pub action_repeat: u32,
    pub max_ticks: u64,
    /// `state_hash` of the fresh sim, hex.
    pub initial_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TickLine {
    #[serde(rename = "type")]
    kind: String,
    t: u64,
    /// Compact action: six digits, one per `ActionV1` component.
    a: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    #[serde(rename = "type")]
    pub kind: String,
    pub t: u64,
    pub event_hash: String,
    pub state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct End {
    #[serde(rename = "type")]
    pub kind: String,
    pub outcome: String,
    pub ticks: u64,
    /// Episode return as raw `f64` bits, hex (exact comparison).
    pub return_bits: String,
}

/// A parsed or freshly recorded episode artifact.
#[derive(Debug, Clone, PartialEq)]
pub struct Replay {
    pub header: Header,
    pub actions: Vec<ActionV1>,
    pub checkpoints: Vec<Checkpoint>,
    pub end: End,
}

fn compact(action: ActionV1) -> String {
    action.as_array().iter().map(|v| v.to_string()).collect()
}

fn parse_compact(text: &str) -> Option<ActionV1> {
    let digits: Vec<u8> = text
        .chars()
        .map(|c| c.to_digit(10).map(|d| d as u8))
        .collect::<Option<_>>()?;
    let array: [u8; 6] = digits.try_into().ok()?;
    ActionV1::new(array).ok()
}

impl Replay {
    /// Serialise to JSONL (one trailing newline).
    pub fn to_jsonl(&self) -> String {
        let mut out = String::new();
        let mut line = |value: &dyn ErasedSerialize| {
            out.push_str(&value.json());
            out.push('\n');
        };
        line(&self.header);
        let mut next = self.checkpoints.iter().peekable();
        for (index, action) in self.actions.iter().enumerate() {
            let t = index as u64 + 1;
            line(&TickLine {
                kind: "tick".into(),
                t,
                a: compact(*action),
            });
            while let Some(checkpoint) = next.next_if(|c| c.t == t) {
                line(checkpoint);
            }
        }
        line(&self.end);
        out
    }

    /// Parse a JSONL artifact. An unknown `schema` is rejected before the
    /// rest of the header is read.
    pub fn from_jsonl(text: &str) -> Result<Self, ReplayError> {
        let mut lines = text
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty());
        let (_, first) = lines
            .next()
            .ok_or_else(|| ReplayError::Parse(1, "the artifact is empty".into()))?;
        let value: serde_json::Value =
            serde_json::from_str(first).map_err(|e| ReplayError::Parse(1, e.to_string()))?;
        let schema = value
            .get("schema")
            .and_then(|v| v.as_str())
            .unwrap_or("<missing>");
        if schema != REPLAY_SCHEMA {
            return Err(ReplayError::UnsupportedSchema(schema.to_string()));
        }
        let header: Header =
            serde_json::from_value(value).map_err(|e| ReplayError::Parse(1, e.to_string()))?;
        let mut actions = Vec::new();
        let mut checkpoints = Vec::new();
        let mut end = None;
        for (index, line) in lines {
            let number = index + 1;
            let bad = |msg: String| ReplayError::Parse(number, msg);
            if end.is_some() {
                return Err(bad("content after the end line".into()));
            }
            let value: serde_json::Value =
                serde_json::from_str(line).map_err(|e| bad(e.to_string()))?;
            match value.get("type").and_then(|v| v.as_str()) {
                Some("tick") => {
                    let tick: TickLine =
                        serde_json::from_value(value).map_err(|e| bad(e.to_string()))?;
                    if tick.t != actions.len() as u64 + 1 {
                        return Err(bad(format!(
                            "tick {} is out of order (expected {})",
                            tick.t,
                            actions.len() + 1
                        )));
                    }
                    let action = parse_compact(&tick.a)
                        .ok_or_else(|| bad(format!("invalid action {:?}", tick.a)))?;
                    actions.push(action);
                }
                Some("checkpoint") => {
                    checkpoints
                        .push(serde_json::from_value(value).map_err(|e| bad(e.to_string()))?);
                }
                Some("end") => {
                    end = Some(serde_json::from_value(value).map_err(|e| bad(e.to_string()))?);
                }
                other => return Err(bad(format!("unknown line type {other:?}"))),
            }
        }
        let end = end.ok_or_else(|| {
            ReplayError::Parse(text.lines().count(), "the artifact has no end line".into())
        })?;
        Ok(Self {
            header,
            actions,
            checkpoints,
            end,
        })
    }
}

/// Object-safe serialisation helper for [`Replay::to_jsonl`].
trait ErasedSerialize {
    fn json(&self) -> String;
}

impl<T: Serialize> ErasedSerialize for T {
    fn json(&self) -> String {
        serde_json::to_string(self).expect("replay lines always serialise")
    }
}

/// Builds a [`Replay`] while an episode runs (action repeat 1 only: every
/// step is one tick, so the trace is the actual per-tick action stream).
#[derive(Debug, Clone)]
pub struct ReplayRecorder {
    header: Header,
    actions: Vec<ActionV1>,
    checkpoints: Vec<Checkpoint>,
}

impl ReplayRecorder {
    pub fn new(header: Header) -> Self {
        Self {
            header,
            actions: Vec::new(),
            checkpoints: Vec::new(),
        }
    }

    /// Whether the tick that just completed needs a checkpoint.
    pub fn wants_checkpoint(tick: u64) -> bool {
        tick > 0 && tick.is_multiple_of(CHECKPOINT_EVERY)
    }

    /// Record the action executed as tick `tick` (1-based).
    pub fn push_action(&mut self, action: ActionV1) {
        self.actions.push(action);
    }

    pub fn push_checkpoint(&mut self, tick: u64, event_hash: u64, state_hash: u64) {
        self.checkpoints.push(Checkpoint {
            kind: "checkpoint".into(),
            t: tick,
            event_hash: hex(event_hash),
            state_hash: hex(state_hash),
        });
    }

    /// Close the episode: adds the final checkpoint (unless the last tick
    /// already has one) and the end line.
    pub fn finish(
        mut self,
        outcome: &str,
        ticks: u64,
        episode_return: f64,
        event_hash: u64,
        state_hash: u64,
    ) -> Replay {
        if self.checkpoints.last().map(|c| c.t) != Some(ticks) {
            self.push_checkpoint(ticks, event_hash, state_hash);
        }
        Replay {
            header: self.header,
            actions: self.actions,
            checkpoints: self.checkpoints,
            end: End {
                kind: "end".into(),
                outcome: outcome.into(),
                ticks,
                return_bits: hex(episode_return.to_bits()),
            },
        }
    }
}

/// Hash of an action trace (for trajectory identity).
pub fn action_trace_hash(actions: &[ActionV1]) -> u64 {
    let mut hash = super::metrics::FNV_SEED;
    for action in actions {
        let mut packed = 0u64;
        for value in action.as_array() {
            packed = packed * 3 + u64::from(value);
        }
        hash = fnv_mix(hash, packed);
    }
    hash
}

/// Build the header for a fresh episode.
pub fn make_header(
    config: &AutoplayConfig,
    reset: &ResetInfo,
    commit: &str,
    kit_hash: u64,
    policy: &str,
    episode_label: &str,
) -> Result<Header, AutoplayError> {
    let (requested_seed, initial_hash, sim_seed) =
        (reset.requested_seed, reset.initial_hash, reset.sim_seed);
    let entry = scenario::lookup(&config.scenario_id)?;
    Ok(Header {
        kind: "header".into(),
        schema: REPLAY_SCHEMA.into(),
        run_id: format!(
            "{}:{}:{}:{}",
            config.scenario_id, policy, requested_seed, episode_label
        ),
        commit: commit.into(),
        scenario: config.scenario_id.clone(),
        scenario_fingerprint: scenario::layout_fingerprint(entry.fixture),
        kit_hash: hex(kit_hash),
        action_version: config.action_version,
        obs_version: OBSERVATION_VERSION,
        reward_id: config.reward_id.clone(),
        cf02_rules: config.cf02_rules,
        sim_seed: hex(sim_seed),
        requested_seed,
        policy: policy.into(),
        bot_settings_version: super::bots::BOT_SETTINGS_VERSION,
        action_repeat: config.action_repeat,
        max_ticks: config.max_ticks,
        initial_hash: hex(initial_hash),
    })
}

/// One difference between a stored and a re-run episode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    pub tick: u64,
    pub what: String,
    pub expected: String,
    pub actual: String,
}

impl fmt::Display for Diff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "tick {}: {} expected {} but the re-run produced {}",
            self.tick, self.what, self.expected, self.actual
        )
    }
}

/// Why a replay could not be verified.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplayError {
    /// Line number (1-based) and the problem.
    Parse(usize, String),
    UnsupportedSchema(String),
    /// A recorded identity or version differs from this build.
    VersionMismatch {
        field: &'static str,
        recorded: String,
        current: String,
    },
    Env(AutoplayError),
    /// The re-run diverged from the stored hashes or outcome.
    Mismatch(Vec<Diff>),
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(line, msg) => write!(f, "replay line {line}: {msg}"),
            Self::UnsupportedSchema(schema) => write!(
                f,
                "replay rejected: unsupported schema {schema:?} (this build reads {REPLAY_SCHEMA:?}); no migration exists"
            ),
            Self::VersionMismatch {
                field,
                recorded,
                current,
            } => write!(
                f,
                "replay rejected: {field} was {recorded} when recorded but is {current} in this build"
            ),
            Self::Env(e) => write!(f, "replay cannot run: {e}"),
            Self::Mismatch(diffs) => {
                write!(
                    f,
                    "replay verification FAILED ({} difference(s))",
                    diffs.len()
                )?;
                for diff in diffs {
                    write!(f, "\n  {diff}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ReplayError {}

impl From<AutoplayError> for ReplayError {
    fn from(e: AutoplayError) -> Self {
        Self::Env(e)
    }
}

/// What a successful verification proved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyReport {
    pub scenario: String,
    pub policy: String,
    pub ticks: u64,
    pub checkpoints: usize,
    pub outcome: String,
    pub final_state_hash: String,
    pub event_hash: String,
    pub recorded_commit: String,
    pub current_commit: String,
}

fn check<T: PartialEq + ToString>(
    field: &'static str,
    recorded: T,
    current: T,
) -> Result<(), ReplayError> {
    if recorded == current {
        Ok(())
    } else {
        Err(ReplayError::VersionMismatch {
            field,
            recorded: recorded.to_string(),
            current: current.to_string(),
        })
    }
}

/// Re-run a stored episode from reset and compare it with the artifact.
/// `current_commit` is only reported: verifying an old artifact on a newer
/// commit is the point of the replay, and a real gameplay change shows up as
/// a hash mismatch.
pub fn verify(
    text: &str,
    assets_root: &Path,
    current_commit: &str,
) -> Result<VerifyReport, ReplayError> {
    let replay = Replay::from_jsonl(text)?;
    let h = &replay.header;
    check("action_version", h.action_version, ACTION_VERSION_V1)?;
    check("obs_version", h.obs_version, OBSERVATION_VERSION)?;
    check("cf02_rules", h.cf02_rules, false)?;
    check("action_repeat", h.action_repeat, 1)?;
    RewardProfile::by_id(&h.reward_id).map_err(|_| ReplayError::VersionMismatch {
        field: "reward_id",
        recorded: h.reward_id.clone(),
        current: "an id this build knows".into(),
    })?;
    let entry = scenario::lookup(&h.scenario)?;
    check(
        "scenario_fingerprint",
        h.scenario_fingerprint.clone(),
        scenario::layout_fingerprint(entry.fixture),
    )?;
    check(
        "kit_hash",
        h.kit_hash.clone(),
        hex(kit_content_hash(assets_root)?),
    )?;

    let mut config = AutoplayConfig::new(h.scenario.clone());
    config.max_ticks = h.max_ticks;
    config.reward_id = h.reward_id.clone();
    let mut env = AutoplayEnv::new(config, assets_root)?;
    let reset = env.reset(h.requested_seed)?;
    let mut diffs = Vec::new();
    if hex(reset.initial_hash) != h.initial_hash {
        diffs.push(Diff {
            tick: 0,
            what: "initial state_hash".into(),
            expected: h.initial_hash.clone(),
            actual: hex(reset.initial_hash),
        });
    }

    let mut hasher = EventHasher::new();
    let mut cursor = env.episode_events().len();
    let mut checkpoints = replay.checkpoints.iter().peekable();
    let mut ended_at = None;
    for (index, action) in replay.actions.iter().enumerate() {
        if !diffs.is_empty() {
            break;
        }
        let tick = index as u64 + 1;
        let step = match env.step(*action) {
            Ok(step) => step,
            Err(e) => {
                diffs.push(Diff {
                    tick,
                    what: "step".into(),
                    expected: "a running episode".into(),
                    actual: e.to_string(),
                });
                break;
            }
        };
        hasher.fold(&env.episode_events()[cursor..]);
        cursor = env.episode_events().len();
        while let Some(checkpoint) = checkpoints.next_if(|c| c.t == tick) {
            let state = hex(env.state_hash());
            let events = hex(hasher.value());
            if checkpoint.state_hash != state {
                diffs.push(Diff {
                    tick,
                    what: "state_hash (checkpoint)".into(),
                    expected: checkpoint.state_hash.clone(),
                    actual: state,
                });
            }
            if checkpoint.event_hash != events {
                diffs.push(Diff {
                    tick,
                    what: "event_hash (checkpoint)".into(),
                    expected: checkpoint.event_hash.clone(),
                    actual: events,
                });
            }
        }
        if let Some(outcome) = step.outcome {
            ended_at = Some((tick, outcome));
            if tick < replay.actions.len() as u64 && diffs.is_empty() {
                diffs.push(Diff {
                    tick,
                    what: "episode end".into(),
                    expected: format!("a running episode until tick {}", replay.actions.len()),
                    actual: format!("ended as {} at tick {tick}", outcome.name()),
                });
            }
            break;
        }
    }
    if diffs.is_empty() {
        match ended_at {
            Some((tick, outcome)) => {
                if outcome.name() != replay.end.outcome {
                    diffs.push(Diff {
                        tick,
                        what: "outcome".into(),
                        expected: replay.end.outcome.clone(),
                        actual: outcome.name().into(),
                    });
                }
                if tick != replay.end.ticks {
                    diffs.push(Diff {
                        tick,
                        what: "tick count".into(),
                        expected: replay.end.ticks.to_string(),
                        actual: tick.to_string(),
                    });
                }
                let bits = hex(env.episode_return().to_bits());
                if bits != replay.end.return_bits {
                    diffs.push(Diff {
                        tick,
                        what: "episode return (f64 bits)".into(),
                        expected: replay.end.return_bits.clone(),
                        actual: bits,
                    });
                }
            }
            None => diffs.push(Diff {
                tick: replay.actions.len() as u64,
                what: "episode end".into(),
                expected: format!("{} at tick {}", replay.end.outcome, replay.end.ticks),
                actual: "still running after the last recorded action".into(),
            }),
        }
        if diffs.is_empty() && checkpoints.peek().is_some() {
            diffs.push(Diff {
                tick: checkpoints.peek().map_or(0, |c| c.t),
                what: "checkpoint".into(),
                expected: "a checkpoint within the recorded ticks".into(),
                actual: "none (the checkpoint lies beyond the last tick)".into(),
            });
        }
    }
    if !diffs.is_empty() {
        return Err(ReplayError::Mismatch(diffs));
    }
    Ok(VerifyReport {
        scenario: h.scenario.clone(),
        policy: h.policy.clone(),
        ticks: replay.end.ticks,
        checkpoints: replay.checkpoints.len(),
        outcome: replay.end.outcome.clone(),
        final_state_hash: hex(env.state_hash()),
        event_hash: hex(hasher.value()),
        recorded_commit: h.commit.clone(),
        current_commit: current_commit.into(),
    })
}
