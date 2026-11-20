//! `assets/data/gameplay/companion_ai_v0.json`: the companion profiles.
//!
//! Two sets (CA-21): `cf02_assist` is exactly the CF02-Q13..Q17 and Q39 assistant
//! (AI Donut active, AI Carl passive: follow layer only) and `companion_v0`
//! makes both roles active. Knob names that carry a CF02 value are named by
//! their question ID (`cf02_q14_swat_range_px`); `action_cadence_ticks` is
//! CF02-Q16. The file is validated and FNV-1a pinned like
//! `companion_magic_v0.json` ([`crate::donut_mana::content_hash`]); a load error
//! names the field. Canonical mana values (the AI reserve) are not repeated
//! here: the view carries `DonutMana::ai_can_cast`.

use serde::{Deserialize, Serialize};

use crate::donut_mana::content_hash;

/// The tuning file, embedded so the binary and the tests read the same bytes.
pub const PROFILE_JSON: &str = include_str!("../../assets/data/gameplay/companion_ai_v0.json");

/// The committed FNV-1a 64 content hash of [`PROFILE_JSON`]. Editing the file
/// without updating this value fails [`ProfileFile::pinned`] and the pin test.
pub const PROFILE_HASH: u64 = 0x6193_c051_ca3f_97e1;

/// What an AI-started hit that would defeat does (CA-19, CA-20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KillPolicy {
    /// The default through 01D: an AI-started hit with damage >= target HP
    /// deals 0 (see [`crate::companion::rules::glance`]).
    #[default]
    Never,
    /// The hit is unchanged; credit stays factual either way. Supported as
    /// data now, tested as the candidate default in 01E (addendum A2).
    Factual,
}

/// Which profile set drives the companion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileSetId {
    Cf02Assist,
    CompanionV0,
}

impl ProfileSetId {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cf02Assist => "cf02_assist",
            Self::CompanionV0 => "companion_v0",
        }
    }
}

/// AI Carl's knobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarlRole {
    /// `false`: AI Carl only follows (the CF02 Q13 passive Carl).
    pub active: bool,
    /// Centre distance to a target at which the jab can connect.
    pub strike_min_px: u16,
    pub strike_max_px: u16,
    /// CA-40: a target farther than this (CF02-Q14's 96 px) is not engaged.
    pub engage_range_px: u16,
    /// A tell aimed at the human hero is interrupted within this distance.
    pub interrupt_range_px: u16,
}

/// AI Donut's knobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DonutRole {
    pub active: bool,
    /// Q14: a tell aimed at the controlled hero is answered within this range.
    pub cf02_q14_swat_range_px: u16,
    /// Q14 (Q39): the earliest tell tick an assist swat may start from.
    pub cf02_q14_swat_tell_tick: u16,
    /// Centre distance at which the swat box connects.
    pub swat_hit_min_px: u16,
    pub swat_hit_max_px: u16,
    /// Q15: pounce range ahead of Donut, on a lying or staggered goblin.
    pub cf02_q15_pounce_min_px: u16,
    pub cf02_q15_pounce_max_px: u16,
    /// Q17: Magic Missile range, after the unlock only.
    pub cf02_q17_missile_min_px: u16,
    pub cf02_q17_missile_max_px: u16,
    /// Planning bound: the latest tell tick a missile can still land by the
    /// end of the tell (cast release plus flight at the minimum range).
    pub missile_latest_tell_tick: u16,
}

/// One profile set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileSet {
    #[serde(default)]
    pub kill_policy: KillPolicy,
    /// CA-10: the utility planning cadence.
    pub decision_cadence_ticks: u16,
    /// CF02-Q16: at most one AI-started move per this many ticks.
    pub action_cadence_ticks: u16,
    /// CF02-Q55 / CA-26: the AI never bails out (v0 requires `false`).
    pub ai_bailout: bool,
    /// Q39 acceptance floor, read by the fixtures: assist starts.
    pub cf02_q39_min_assist_starts: u8,
    /// Q39 acceptance ceiling: the AI's share of the hits, in percent.
    pub cf02_q39_max_hit_share_percent: u8,
    pub carl: CarlRole,
    pub donut: DonutRole,
}

/// The whole file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileFile {
    pub schema_version: u32,
    pub cf02_assist: ProfileSet,
    pub companion_v0: ProfileSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    /// The JSON did not parse or has an unknown or missing field (serde names
    /// it in the message).
    Parse(String),
    /// A field failed validation; `field` is its dotted path.
    Field { field: String, reason: String },
    /// The file's content hash is not the pinned one.
    HashMismatch { expected: u64, actual: u64 },
}

impl core::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "companion ai profile: {msg}"),
            Self::Field { field, reason } => {
                write!(f, "companion ai profile: field `{field}`: {reason}")
            }
            Self::HashMismatch { expected, actual } => write!(
                f,
                "companion ai profile hash {actual:#018x} is not the pinned {expected:#018x}"
            ),
        }
    }
}

impl std::error::Error for ProfileError {}

fn bad(field: &str, reason: &str) -> Result<(), ProfileError> {
    Err(ProfileError::Field {
        field: field.to_string(),
        reason: reason.to_string(),
    })
}

impl ProfileFile {
    /// Parse and validate.
    pub fn from_json(text: &str) -> Result<Self, ProfileError> {
        let file: Self =
            serde_json::from_str(text).map_err(|err| ProfileError::Parse(err.to_string()))?;
        file.validate()?;
        Ok(file)
    }

    /// Parse the embedded file and require the pinned content hash.
    pub fn pinned() -> Result<Self, ProfileError> {
        Self::from_json_pinned(PROFILE_JSON, PROFILE_HASH)
    }

    /// Parse `text` and require its content hash to equal `expected` (a replay
    /// header or fixture records the hash it ran with).
    pub fn from_json_pinned(text: &str, expected: u64) -> Result<Self, ProfileError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(ProfileError::HashMismatch { expected, actual });
        }
        Self::from_json(text)
    }

    pub fn set(&self, id: ProfileSetId) -> &ProfileSet {
        match id {
            ProfileSetId::Cf02Assist => &self.cf02_assist,
            ProfileSetId::CompanionV0 => &self.companion_v0,
        }
    }

    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.schema_version != 1 {
            return bad("schema_version", "must be 1");
        }
        self.cf02_assist.validate("cf02_assist")?;
        self.companion_v0.validate("companion_v0")?;
        // CA-21: cf02_assist is Q13's assistant (AI Carl passive); v0 is both.
        if self.cf02_assist.carl.active {
            return bad("cf02_assist.carl.active", "CF02-Q13: AI Carl stays passive");
        }
        if !self.cf02_assist.donut.active {
            return bad(
                "cf02_assist.donut.active",
                "CF02-Q13: AI Donut is the assistant",
            );
        }
        if !self.companion_v0.carl.active {
            return bad(
                "companion_v0.carl.active",
                "companion_v0 makes both roles active",
            );
        }
        if !self.companion_v0.donut.active {
            return bad(
                "companion_v0.donut.active",
                "companion_v0 makes both roles active",
            );
        }
        Ok(())
    }
}

impl ProfileSet {
    fn validate(&self, set: &str) -> Result<(), ProfileError> {
        let f = |name: &str| format!("{set}.{name}");
        if self.decision_cadence_ticks == 0 {
            return bad(&f("decision_cadence_ticks"), "must be positive");
        }
        if self.action_cadence_ticks == 0 {
            return bad(&f("action_cadence_ticks"), "must be positive");
        }
        if self.ai_bailout {
            return bad(&f("ai_bailout"), "CF02-Q55: the AI never bails out in v0");
        }
        if self.cf02_q39_min_assist_starts == 0 {
            return bad(&f("cf02_q39_min_assist_starts"), "must be positive");
        }
        if self.cf02_q39_max_hit_share_percent == 0 || self.cf02_q39_max_hit_share_percent > 100 {
            return bad(&f("cf02_q39_max_hit_share_percent"), "must be in 1..=100");
        }
        let carl = &self.carl;
        if carl.strike_min_px >= carl.strike_max_px {
            return bad(&f("carl.strike_min_px"), "must be below strike_max_px");
        }
        if carl.engage_range_px == 0 {
            return bad(&f("carl.engage_range_px"), "must be positive");
        }
        if carl.interrupt_range_px == 0 {
            return bad(&f("carl.interrupt_range_px"), "must be positive");
        }
        let d = &self.donut;
        if d.cf02_q14_swat_range_px == 0 {
            return bad(&f("donut.cf02_q14_swat_range_px"), "must be positive");
        }
        if d.swat_hit_min_px >= d.swat_hit_max_px {
            return bad(&f("donut.swat_hit_min_px"), "must be below swat_hit_max_px");
        }
        if d.cf02_q15_pounce_min_px >= d.cf02_q15_pounce_max_px {
            return bad(
                &f("donut.cf02_q15_pounce_min_px"),
                "must be below cf02_q15_pounce_max_px",
            );
        }
        if d.cf02_q17_missile_min_px >= d.cf02_q17_missile_max_px {
            return bad(
                &f("donut.cf02_q17_missile_min_px"),
                "must be below cf02_q17_missile_max_px",
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> ProfileFile {
        ProfileFile::pinned().expect("the embedded profile file is valid and pinned")
    }

    #[test]
    fn profile_hash_is_pinned() {
        // Fails when companion_ai_v0.json changes without updating PROFILE_HASH.
        assert_eq!(content_hash(PROFILE_JSON), PROFILE_HASH);
        assert!(ProfileFile::pinned().is_ok());
    }

    #[test]
    fn a_drifted_file_is_a_clear_hash_failure() {
        let drifted = PROFILE_JSON.replacen(
            "\"action_cadence_ticks\": 60",
            "\"action_cadence_ticks\": 61",
            1,
        );
        assert_ne!(drifted, PROFILE_JSON);
        match ProfileFile::from_json_pinned(&drifted, PROFILE_HASH) {
            Err(ProfileError::HashMismatch { expected, actual }) => {
                assert_eq!(expected, PROFILE_HASH);
                assert_ne!(actual, expected);
            }
            other => panic!("expected a hash mismatch, got {other:?}"),
        }
        // Unpinned it still parses: the pin is what catches the edit.
        assert!(ProfileFile::from_json(&drifted).is_ok());
    }

    #[test]
    fn the_file_round_trips() {
        let a = file();
        let text = serde_json::to_string(&a).unwrap();
        let b = ProfileFile::from_json(&text).unwrap();
        assert_eq!(a, b);
        assert_eq!(serde_json::to_string(&b).unwrap(), text);
    }

    #[test]
    fn cf02_assist_carries_the_adopted_q13_to_q17_and_q39_values() {
        let f = file();
        let s = &f.cf02_assist;
        assert_eq!(s.kill_policy, KillPolicy::Never);
        assert_eq!((s.decision_cadence_ticks, s.action_cadence_ticks), (6, 60));
        assert!(!s.ai_bailout);
        assert!(!s.carl.active, "Q13: AI Carl is passive");
        assert_eq!(
            (s.carl.engage_range_px, s.carl.interrupt_range_px),
            (96, 96)
        );
        let d = &s.donut;
        assert!(d.active);
        assert_eq!(d.cf02_q14_swat_range_px, 96);
        assert_eq!(d.cf02_q14_swat_tell_tick, 4);
        assert_eq!(
            (d.cf02_q15_pounce_min_px, d.cf02_q15_pounce_max_px),
            (30, 80)
        );
        assert_eq!(
            (d.cf02_q17_missile_min_px, d.cf02_q17_missile_max_px),
            (97, 160)
        );
        assert_eq!(
            (
                s.cf02_q39_min_assist_starts,
                s.cf02_q39_max_hit_share_percent
            ),
            (3, 25)
        );
    }

    #[test]
    fn companion_v0_makes_both_roles_active_with_the_same_cadences() {
        let f = file();
        let v0 = f.set(ProfileSetId::CompanionV0);
        assert!(v0.carl.active && v0.donut.active);
        assert_eq!(v0.kill_policy, KillPolicy::Never);
        assert_eq!(
            (v0.decision_cadence_ticks, v0.action_cadence_ticks),
            (6, 60)
        );
        assert!(!v0.ai_bailout);
        assert_eq!(v0.donut, f.cf02_assist.donut, "Donut's numbers are shared");
        assert_eq!(ProfileSetId::Cf02Assist.name(), "cf02_assist");
        assert_eq!(ProfileSetId::CompanionV0.name(), "companion_v0");
    }

    #[test]
    fn kill_policy_defaults_to_never_and_factual_is_data() {
        assert_eq!(KillPolicy::default(), KillPolicy::Never);
        let text = PROFILE_JSON.replace("\"kill_policy\": \"never\",", "");
        let f = ProfileFile::from_json(&text).unwrap();
        assert_eq!(f.cf02_assist.kill_policy, KillPolicy::Never);
        let text = PROFILE_JSON.replacen("\"never\"", "\"factual\"", 1);
        let f = ProfileFile::from_json(&text).unwrap();
        assert_eq!(f.cf02_assist.kill_policy, KillPolicy::Factual);
        assert_eq!(f.companion_v0.kill_policy, KillPolicy::Never);
    }

    fn field_of(text: &str) -> String {
        match ProfileFile::from_json(text) {
            Err(ProfileError::Field { field, .. }) => field,
            other => panic!("expected a field error, got {other:?}"),
        }
    }

    #[test]
    fn a_load_error_names_the_field() {
        let swap = |from: &str, to: &str| PROFILE_JSON.replacen(from, to, 1);
        assert_eq!(
            field_of(&swap(
                "\"decision_cadence_ticks\": 6",
                "\"decision_cadence_ticks\": 0"
            )),
            "cf02_assist.decision_cadence_ticks"
        );
        assert_eq!(
            field_of(&swap("\"ai_bailout\": false", "\"ai_bailout\": true")),
            "cf02_assist.ai_bailout"
        );
        assert_eq!(
            field_of(&swap(
                "\"cf02_q15_pounce_min_px\": 30",
                "\"cf02_q15_pounce_min_px\": 80"
            )),
            "cf02_assist.donut.cf02_q15_pounce_min_px"
        );
        assert_eq!(
            field_of(&swap("\"active\": false", "\"active\": true")),
            "cf02_assist.carl.active"
        );
        assert_eq!(
            field_of(&swap("\"schema_version\": 1", "\"schema_version\": 2")),
            "schema_version"
        );
        // A missing or unknown field is named by the parser.
        let missing = PROFILE_JSON.replacen("\"cf02_q39_min_assist_starts\": 3,", "", 1);
        match ProfileFile::from_json(&missing) {
            Err(ProfileError::Parse(msg)) => {
                assert!(msg.contains("cf02_q39_min_assist_starts"), "{msg}")
            }
            other => panic!("expected a parse error, got {other:?}"),
        }
        let unknown = PROFILE_JSON.replacen("\"ai_bailout\"", "\"ai_bailout_typo\"", 1);
        assert!(matches!(
            ProfileFile::from_json(&unknown),
            Err(ProfileError::Parse(_))
        ));
        assert!(
            ProfileError::Field {
                field: "x.y".into(),
                reason: "r".into()
            }
            .to_string()
            .contains("`x.y`")
        );
    }
}
