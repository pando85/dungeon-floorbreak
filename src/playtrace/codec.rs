//! The `playtrace-session/1` record types and their exact JSON encoding.
//!
//! Everything is integers, booleans and strings: no float is printed, so the
//! encoding is byte-stable and bit-exact for analog axes (`f32::to_bits`).

use serde::{Deserialize, Serialize};

use crate::combat::CombatEvent;
use crate::input::RawInput;

/// Session format version (independent of `autoplay-episode/1`).
pub const SESSION_SCHEMA: &str = "playtrace-session/1";
/// Action encoding version ([`ActionBits`]). Version 2 adds the `down` bit
/// (COMBAT-FUN-02, CF02-Q01); every version 1 record decodes unchanged (its
/// `down` bit is never set).
pub const ACTION_SCHEMA: &str = "rawinput-bits/2";
/// The previous action encoding (no `down` bit); still decodable.
pub const ACTION_SCHEMA_V1: &str = "rawinput-bits/1";
/// Event encoding version ([`EventRecord`]: absolute id, variant, Debug text).
pub const EVENT_SCHEMA: &str = "combat-event-debug/1";
/// Observation encoding ([`encode_obs_bits`]): the flattened `float32` bits of
/// one ObservationV2, hex tokens with zero runs.
pub const OBS_ENCODING: &str = "f32bits-rle-hex/1";
/// The manifest's `observation_schema_id` of a session that records
/// ObservationV2 per hero (PT-C).
pub const OBS_SCHEMA_ID: &str = "autoplay-obs/2+f32bits-rle-hex/1";
/// Ticks per `ticks-NNNNNN.jsonl` chunk (PT-Q15).
pub const TICKS_PER_CHUNK: usize = 600;
/// Bound of the pending write queue, in ticks (PT-Q15).
pub const QUEUE_BOUND: usize = 600;

/// File name of tick chunk `index`.
pub fn chunk_file_name(index: usize) -> String {
    format!("ticks-{index:06}.jsonl")
}

/// A `u64` printed as 16 lowercase hex digits (JSON numbers lose precision
/// above 2^53 in many readers).
pub mod hex64 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{value:016x}"))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
        let text = String::deserialize(deserializer)?;
        u64::from_str_radix(&text, 16).map_err(serde::de::Error::custom)
    }
}

/// One [`RawInput`] after the deadzone, bit-exact: `(move_x bits, buttons)`.
/// Button bits: jump 1, attack 2, physical 4, ability 8, pause 16, swap 32, down 64
/// (`rawinput-bits/2`; a `/1` record has bit 64 clear).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionBits(pub u32, pub u8);

impl ActionBits {
    pub const NONE: ActionBits = ActionBits(0, 0);

    pub fn from_raw(raw: RawInput) -> Self {
        let buttons = u8::from(raw.jump)
            | u8::from(raw.attack) << 1
            | u8::from(raw.physical) << 2
            | u8::from(raw.ability) << 3
            | u8::from(raw.pause) << 4
            | u8::from(raw.swap) << 5
            | u8::from(raw.down) << 6;
        ActionBits(raw.move_x.to_bits(), buttons)
    }

    pub fn to_raw(self) -> RawInput {
        RawInput {
            move_x: f32::from_bits(self.0),
            jump: self.1 & 1 != 0,
            attack: self.1 & 2 != 0,
            physical: self.1 & 4 != 0,
            ability: self.1 & 8 != 0,
            pause: self.1 & 16 != 0,
            swap: self.1 & 32 != 0,
            down: self.1 & 64 != 0,
        }
    }
}

impl From<RawInput> for ActionBits {
    fn from(raw: RawInput) -> Self {
        Self::from_raw(raw)
    }
}

/// Who drives a hero this tick. `companion_ai` is reserved for the COMPANION
/// wiring (not on main); the live follower bot is recorded as `scripted_bot`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlSource {
    HumanP1,
    HumanP2,
    CompanionAi,
    ScriptedBot,
    None,
}

/// One authoritative combat event with its absolute id
/// (`dropped_events + index` of the capped log).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventRecord {
    pub i: u64,
    /// The variant name. Build-scoped and diagnostic only (PTI-02): a rename
    /// changes it without a schema bump. The authoritative event identity is
    /// the typed rolling hash `eh`; never parse `k` or `d` in an exporter.
    pub k: String,
    /// The full `Debug` text (exact within one build; a typed schema is future
    /// work, required before PT-D).
    pub d: String,
}

impl EventRecord {
    pub fn from_combat(id: u64, event: &CombatEvent) -> Self {
        let text = format!("{event:?}");
        let kind = text.split([' ', '{', '(']).next().unwrap_or("").to_owned();
        Self {
            i: id,
            k: kind,
            d: text,
        }
    }
}

/// One simulated 60 Hz transition: pre-state, the actions fed to the step,
/// the ordered events it produced, post-state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickRecord {
    /// Recorder-owned global tick, contiguous from 0 (never reset).
    pub t: u64,
    /// Episode id (starts at 0; a restart or respawn begins the next).
    pub ep: u32,
    /// `sim.world.tick` after the step (restart resets it, respawn keeps it).
    pub et: u64,
    /// Stage (demo checkpoint index; 0 elsewhere).
    pub stage: u32,
    /// State hash before the step.
    #[serde(with = "hex64")]
    pub pre: u64,
    /// Per-slot raw actions (P1, P2): provenance and consistency check.
    pub slots: [ActionBits; 2],
    /// Routed per-hero actions (Carl, Donut): the replay driver.
    pub hero: [ActionBits; 2],
    /// Who drove each hero (Carl, Donut).
    pub src: [ControlSource; 2],
    /// Events emitted since the previous record, in order.
    pub ev: Vec<EventRecord>,
    /// State hash after the step.
    #[serde(with = "hex64")]
    pub post: u64,
    /// Rolling ordered-event hash (`EventHasher`) after this tick.
    #[serde(with = "hex64")]
    pub eh: u64,
    /// Party swap-timer hash.
    #[serde(with = "hex64")]
    pub ph: u64,
    /// PT-C: the policy observation `o_t` of each hero (Carl, Donut), ObservationV2
    /// computed **before** the live decision from the state `pre` hashes, in
    /// [`encode_obs_bits`] form. `None` without a combat state (the story route)
    /// or in a PT-B session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obs: Option<[String; 2]>,
    /// PT-C: the effective per-hero inputs the sim applied this tick
    /// (`SceneSim::last_effective_inputs`: after the bot decision, the combat
    /// gating and the 2P separation wall). `None` when the drive is not live
    /// (no bot exists then) and in a PT-B session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eff: Option<[ActionBits; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeReason {
    Start,
    Restart,
    PartyDefeatRestart,
    /// A Select-tap respawn (also recorded as a `demo_respawn` control event).
    DemoRespawn,
    /// The demo's own respawn after Carl's defeat, performed inside the step:
    /// the re-derived step does it again, so there is no control event.
    DemoDefeatRespawn,
}

/// The authoritative boundary that starts an episode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpisodeBoundary {
    pub episode: u32,
    pub reason: EpisodeReason,
    pub stage: u32,
}

/// Lifecycle and control events that exist on main today. Future events
/// (pause menu, `config_change`, unlocks, companion commands) are not invented.
///
/// **Replay inputs** (a verifier must apply them before the tick `at_t`):
/// `restart` (a manual restart outside the step), `demo_respawn` (a Select-tap
/// respawn outside the step) and `drive_change`. Everything else is
/// informational: `episode_*`, `solo_swap`, `party_*`, `pad_*`, `d6_pause_*`,
/// `demo_checkpoint_advance`, `exit`, `app_quit`. Restarts and respawns that
/// the step performs itself (party defeat, demo defeat) are never control
/// events, only episode boundaries (`party_defeat_restart`,
/// `demo_defeat_respawn`), so a replay never applies them twice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "k", rename_all = "snake_case")]
pub enum ControlEvent {
    EpisodeBegin {
        episode: u32,
        reason: EpisodeReason,
        stage: u32,
    },
    EpisodeEnd {
        episode: u32,
        outcome: String,
        /// PT-C: `o_(t+1)`, the observation of each hero (Carl, Donut) of the
        /// state after the last tick of the episode, stored only here (terminal,
        /// truncation, segment boundary). For an in-step reset (`party_defeat`,
        /// `demo_defeat_respawn`) the step has already reset the scene, so this
        /// is the first observation of the next episode: the defeat frame is
        /// not observable outside the step (PTI-14).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        final_obs: Option<[String; 2]>,
    },
    Restart,
    DemoRespawn,
    DemoCheckpointAdvance {
        checkpoint: u32,
    },
    SoloSwap {
        controlled: u8,
    },
    PartyJoin,
    PartyQuit,
    PadVacated {
        slot: u8,
    },
    PadReclaimed {
        slot: u8,
    },
    D6PauseBegin,
    D6PauseEnd,
    DriveChange {
        live: bool,
        two_player: bool,
        controlled: u8,
        bots: [bool; 2],
    },
    Frozen {
        on: bool,
    },
    Exit {
        reason: String,
    },
    AppQuit,
}

/// One `control.jsonl` line. `at_t` is the global tick of the next recorded
/// tick: the event happened before that tick was stepped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlRecord {
    pub seq: u64,
    pub at_t: u64,
    pub ev: ControlEvent,
}

/// The last line of a closed chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkTrailer {
    pub end: bool,
    pub chunk: usize,
    pub ticks: usize,
    pub first_t: Option<u64>,
    pub last_t: Option<u64>,
    /// FNV-1a-64 of the chunk's tick lines (each with its `\n`).
    #[serde(with = "hex64")]
    pub fnv: u64,
}

/// Encode flattened `f32` bits compactly and exactly: comma separated tokens,
/// a non-zero value as lowercase hex without leading zeros, a run of `N >= 1`
/// zeros as `zN`. Most of an observation (masks, empty enemy rows) is zero.
pub fn encode_obs_bits(bits: &[u32]) -> String {
    let mut out = String::with_capacity(bits.len() * 3);
    let mut zeros = 0_usize;
    let mut first = true;
    let flush_zeros = |out: &mut String, zeros: &mut usize, first: &mut bool| {
        if *zeros > 0 {
            if !*first {
                out.push(',');
            }
            out.push('z');
            out.push_str(&zeros.to_string());
            *zeros = 0;
            *first = false;
        }
    };
    for &value in bits {
        if value == 0 {
            zeros += 1;
            continue;
        }
        flush_zeros(&mut out, &mut zeros, &mut first);
        if !first {
            out.push(',');
        }
        out.push_str(&format!("{value:x}"));
        first = false;
    }
    flush_zeros(&mut out, &mut zeros, &mut first);
    out
}

/// The inverse of [`encode_obs_bits`]; strict (any malformed token is an error).
pub fn decode_obs_bits(text: &str) -> Result<Vec<u32>, String> {
    let mut out = Vec::new();
    if text.is_empty() {
        return Ok(out);
    }
    for token in text.split(',') {
        if let Some(run) = token.strip_prefix('z') {
            let count: usize = run
                .parse()
                .ok()
                .filter(|n| *n >= 1 && !run.starts_with('0'))
                .ok_or_else(|| format!("bad zero run {token:?}"))?;
            out.extend(std::iter::repeat_n(0_u32, count));
        } else {
            if token.is_empty() || token.starts_with('0') {
                return Err(format!("bad value token {token:?}"));
            }
            out.push(
                u32::from_str_radix(token, 16).map_err(|_| format!("bad value token {token:?}"))?,
            );
        }
    }
    Ok(out)
}

/// The privileged diagnostics of one tick (`diagnostics.jsonl`): what the
/// policy must never see. Never an input of a learner; the exporter reads
/// `obs` and the inputs only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagRecord {
    /// Recorder-owned global tick this line belongs to.
    pub t: u64,
    /// The live presentation rig's view rectangle `[left, top, w, h]` before
    /// the tick (a diagnostic only: the observation uses the sim camera).
    pub rig: Option<[i32; 4]>,
    /// The sim `ScrollCamera` view `[left, top, w, h]` the observation used.
    pub cam: [i32; 4],
    /// Exact health `[carl, donut]` (the observation holds a fraction).
    pub hp: [i32; 2],
    /// Donut's `(mana, max)` while the biscuit is unlocked.
    pub mana: Option<[u32; 2]>,
    /// Hurt ticks `[carl, donut]`.
    pub hurt: [u32; 2],
    /// Living enemies (goblins and llamas), on and off screen.
    pub enemies: u32,
    /// Living enemies the sim camera does not show (hidden from the policy).
    pub enemies_hidden: u32,
    /// Observation clamping, unknown-mode and overflow counters while building
    /// the two `o_t` of this tick.
    pub obs_clipped: u32,
    pub obs_unknown_modes: u32,
    pub obs_overflow: u32,
}
