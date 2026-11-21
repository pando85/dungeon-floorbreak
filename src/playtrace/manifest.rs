use std::collections::BTreeMap;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::codec::{ACTION_SCHEMA, EVENT_SCHEMA, SESSION_SCHEMA, hex64};
use crate::party::MotionProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunKind {
    Human,
    ScriptedBot,
    LearnedPolicy,
    Mixed,
}

/// PT-Q16: who played. `owner_only` is the default; an
/// `includes_other_tester` session is never exported without an explicit
/// per-session `--export-consented`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Participants {
    OwnerOnly,
    IncludesOtherTester,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Recording,
    Completed,
    Abandoned,
    CrashedRecovered,
    Incomplete,
    Corrupt,
}

/// The serde default of [`ResetDescriptor::donut_motion`]: a descriptor without
/// the field was recorded before the agile motion existed.
fn legacy_motion() -> MotionProfile {
    MotionProfile::Legacy
}

fn is_legacy_motion(motion: &MotionProfile) -> bool {
    *motion == MotionProfile::Legacy
}

/// What a fresh process needs to rebuild tick 0 (the `PartyFixtureSim` style:
/// replay is from this descriptor plus the input tape; v1 has no snapshots).
/// No path: only repo-relative names and hashes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResetDescriptor {
    /// `demo`, `fixture`, `boss`, `story` (the corridor).
    pub source: String,
    /// Fixture or scene name (empty for the story corridor).
    pub name: String,
    pub demo: bool,
    pub demo_beat: u32,
    pub cf02_rules: bool,
    pub cf02_seed: Option<u64>,
    pub view_height: i32,
    pub donut_biscuit: bool,
    /// CARL-PROG-01 CP-A (CP-Q09, CP-Q46): the story flags of the run. Absent
    /// (false) in older recordings; written only when true, so a descriptor
    /// with all three false serializes exactly as before.
    #[serde(default, skip_serializing_if = "is_false")]
    pub carl_prog: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub carl_dynamite: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub carl_gauntlet: bool,
    /// DONUT-MOBILITY-01: the motion Donut's body stepped with. **Absent means
    /// `legacy`** (every recording made before DM-A), through an explicit
    /// default function, never `Default`, so a rebuild of an old recording can
    /// not silently become agile. Written only when it is not `legacy`.
    #[serde(default = "legacy_motion", skip_serializing_if = "is_legacy_motion")]
    pub donut_motion: MotionProfile,
    /// DONUT-PROG-01 DP-A (DP-Q10): the second hero's story flags, the same
    /// rule: absent (false) in older recordings and written only when true.
    #[serde(default, skip_serializing_if = "is_false")]
    pub donut_prog: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub donut_light_feet: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub donut_puddle: bool,
    /// PT-SCENE: the late-Floor-1 room of the run (`--scene f01_late
    /// --f01-room <id>`, or `--scene f01_boiler`) and its loadout label
    /// (`story` or `all`). Absent in every older recording and written only
    /// when set, so a descriptor of any earlier run serializes exactly as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub f01_room: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub f01_loadout: Option<String>,
    /// FNV-1a-64 of the layout and kit contract (0 when not computed).
    #[serde(with = "hex64")]
    pub kit_hash: u64,
    /// State hash of the freshly reset scene, before tick 0 and before the
    /// first `drive_change` is applied (PTI-05): for a live-party run it
    /// differs from tick 0's `pre`, which is taken after the drive is set.
    /// Compare it only against a fresh reset with the legacy drive.
    #[serde(with = "hex64")]
    pub start_hash: u64,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Line count and FNV-1a-64 (over every line with its `\n`) of an append-only
/// side stream (`control.jsonl`, `diagnostics.jsonl`) as the writer accepted
/// it. A finished session must read back exactly this (a deleted or edited
/// line is caught even when `seq` still looks contiguous).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamIntegrity {
    pub lines: u64,
    #[serde(with = "hex64")]
    pub fnv: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkChecksum {
    pub chunk: usize,
    pub ticks: usize,
    #[serde(with = "hex64")]
    pub fnv: u64,
}

/// `manifest.json`. Written as `manifest.json.tmp` and renamed (atomic) at
/// start and at finish. Contains no hostname, user name, home path, pad id or
/// serial; the wall clock is rounded to the UTC day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionManifest {
    pub schema: String,
    pub session_id: String,
    pub run_kind: RunKind,
    pub participants: Participants,
    /// `YYYY-MM-DD` (UTC), diagnostic only.
    pub started_on_utc_day: Option<String>,
    pub build_commit: String,
    pub build_dirty: bool,
    /// `exe-w64/1:<length>:<hash>` of the recording executable (identifies the
    /// file, not the source), or `unknown`.
    pub binary_fingerprint: String,
    /// The `rustc -V` line of the recording build (embedded by `build.rs`).
    pub toolchain: String,
    /// The build-time FNV-1a-64 over the toolchain, `Cargo.toml`, `Cargo.lock`
    /// and `src/` of the recording build. A replay needs the same value: it is
    /// the sim identity, valid for a dirty tree too. Empty in a PT-B session
    /// (which therefore cannot be verified).
    #[serde(default)]
    pub source_fingerprint: String,
    pub os_family: String,
    pub build_profile: String,
    pub game_ruleset_id: String,
    pub feature_flags: BTreeMap<String, String>,
    pub tick_rate_hz: u32,
    pub content_hashes: BTreeMap<String, String>,
    pub reset: ResetDescriptor,
    pub camera_ruleset: String,
    pub rng_algorithm: String,
    pub seed_requested: Option<u64>,
    pub seed_effective: Option<u64>,
    pub action_schema_id: String,
    pub observation_schema_id: String,
    pub event_schema_id: String,
    pub snapshot_schema_id: String,
    pub party_schema_id: String,
    pub recorder_schema_id: String,
    /// `solo_human`, `local_coop`, ... as known at start.
    pub mode: String,
    pub input_adapter: String,
    pub deadzone: String,
    pub status: SessionStatus,
    pub capture_complete: bool,
    pub replay_verified: bool,
    pub training_eligible: bool,
    pub observation_is_partial: bool,
    /// `reset_only` in v1.
    pub snapshot_support: String,
    pub quality_flags: Vec<String>,
    pub tick_count: u64,
    pub last_tick: Option<u64>,
    pub dropped_ticks: u64,
    pub write_errors: u64,
    /// Control events lost to a full queue (a `seq` gap is legal only then, and
    /// only in an `incomplete` session).
    #[serde(default)]
    pub dropped_control: u64,
    /// Diagnostics lines lost to a full queue (does not make the capture
    /// incomplete: the stream is privileged and optional).
    #[serde(default)]
    pub dropped_diagnostics: u64,
    /// Failed intermediate manifest writes at chunk rotation (no data is lost;
    /// counted, PT-C follow-up 5).
    #[serde(default)]
    pub manifest_write_errors: u64,
    /// `control.jsonl` line count and FNV, written at finish (PT-C follow-up 2).
    /// `None` in a PT-B session.
    #[serde(default)]
    pub control_integrity: Option<StreamIntegrity>,
    /// The same for `diagnostics.jsonl` (`None`: no diagnostics stream).
    #[serde(default)]
    pub diagnostics_integrity: Option<StreamIntegrity>,
    /// `truncated` for a recovered crash, `None` otherwise.
    pub terminal: Option<String>,
    pub chunk_checksums: Vec<ChunkChecksum>,
    pub export_lineage: Vec<String>,
}

impl SessionManifest {
    /// A manifest with the v1 defaults. The caller fills what it knows (build
    /// identity, flags, hashes); recorded sessions are `replay_unverified`,
    /// `observation_is_partial` and not `training_eligible` until PT-C.
    pub fn new(session_id: String, reset: ResetDescriptor) -> Self {
        Self {
            schema: SESSION_SCHEMA.to_owned(),
            session_id,
            run_kind: RunKind::Human,
            participants: Participants::OwnerOnly,
            started_on_utc_day: utc_day_now(),
            build_commit: "unknown".to_owned(),
            build_dirty: false,
            binary_fingerprint: String::new(),
            toolchain: String::new(),
            source_fingerprint: String::new(),
            os_family: std::env::consts::OS.to_owned(),
            build_profile: if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
            .to_owned(),
            game_ruleset_id: String::new(),
            feature_flags: BTreeMap::new(),
            tick_rate_hz: 60,
            content_hashes: BTreeMap::new(),
            reset,
            camera_ruleset: String::new(),
            rng_algorithm: "none".to_owned(),
            seed_requested: None,
            seed_effective: None,
            action_schema_id: ACTION_SCHEMA.to_owned(),
            observation_schema_id: "none".to_owned(),
            event_schema_id: EVENT_SCHEMA.to_owned(),
            snapshot_schema_id: "none".to_owned(),
            party_schema_id: "party-routing/1".to_owned(),
            recorder_schema_id: SESSION_SCHEMA.to_owned(),
            mode: String::new(),
            input_adapter: "bevy-keyboard-pad/1".to_owned(),
            deadzone: "post-deadzone RawInput".to_owned(),
            status: SessionStatus::Recording,
            capture_complete: false,
            replay_verified: false,
            training_eligible: false,
            observation_is_partial: true,
            snapshot_support: "reset_only".to_owned(),
            quality_flags: vec!["replay_unverified".to_owned()],
            tick_count: 0,
            last_tick: None,
            dropped_ticks: 0,
            write_errors: 0,
            dropped_control: 0,
            dropped_diagnostics: 0,
            manifest_write_errors: 0,
            control_integrity: None,
            diagnostics_integrity: None,
            terminal: None,
            chunk_checksums: Vec::new(),
            export_lineage: Vec::new(),
        }
    }
}

/// 128 random-looking bits as 32 hex digits, from `RandomState` mixed with
/// the wall clock, the process id and a counter. No new crate, no PII.
pub fn generate_session_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut words = [0_u64; 2];
    for (index, word) in words.iter_mut().enumerate() {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(nanos);
        hasher.write_u32(std::process::id());
        hasher.write_u64(count);
        hasher.write_usize(index);
        *word = hasher.finish();
    }
    format!("{:016x}{:016x}", words[0], words[1])
}

/// UTC date of the wall clock, `YYYY-MM-DD` (civil-from-days).
fn utc_day_now() -> Option<String> {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let days = (secs / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    Some(format!("{year:04}-{month:02}-{day:02}"))
}
