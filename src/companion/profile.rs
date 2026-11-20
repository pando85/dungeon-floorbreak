//! `assets/data/gameplay/companion_ai_v0.json`: the companion profiles.
//!
//! Two sets (CA-21): `cf02_assist` is exactly the CF02-Q13..Q17 and Q39 assistant
//! (AI Donut active, AI Carl passive: follow layer only) and `companion_v0`
//! makes both roles active. Knob names that carry a CF02 value are named by
//! their question ID (`cf02_q14_swat_range_px`); `cf02_q16_action_cadence_ticks` is
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
pub const PROFILE_HASH: u64 = 0x5856_cc9c_1062_b17f;

/// The profile set id under the spec's name (CA-21, section 17): the runtime's
/// `UncontrolledHero::Companion(CompanionProfileSet)` names it.
pub type CompanionProfileSet = ProfileSetId;

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
    pub ca21_active: bool,
    /// Centre distance to a target at which the jab can connect (CA-41, a
    /// planning window inside the move boxes widened by `ENEMY_HALF_WIDTH`).
    pub ca41_strike_min_px: u16,
    pub ca41_strike_max_px: u16,
    /// CA-40: a target farther than this (CF02-Q14's 96 px) is not engaged.
    pub ca40_engage_range_px: u16,
    /// A tell aimed at the human hero is interrupted within this distance.
    pub ca40_interrupt_range_px: u16,
}

/// AI Donut's knobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DonutRole {
    pub ca21_active: bool,
    /// Q14: a tell aimed at the controlled hero is answered within this range.
    pub cf02_q14_swat_range_px: u16,
    /// Q14 (Q39): the earliest tell tick an assist swat may start from.
    pub cf02_q14_swat_tell_tick: u16,
    /// Centre distance at which the swat box connects (CA-41, same rule).
    pub ca41_swat_hit_min_px: u16,
    pub ca41_swat_hit_max_px: u16,
    /// Q15: pounce range ahead of Donut, on a lying or staggered goblin.
    pub cf02_q15_pounce_min_px: u16,
    pub cf02_q15_pounce_max_px: u16,
    /// CF02-Q38 (hold): a pounce on a lying goblin starts only while the goblin
    /// still lies for at least this many ticks, about the pounce's startup plus
    /// flight, so it lands before the get-up (a late one would stand the goblin
    /// up, the outcome Q34 and Q38 prevent).
    pub cf02_q38_pounce_lying_min_ticks_left: u16,
    /// Q17: Magic Missile range, after the unlock only.
    pub cf02_q17_missile_min_px: u16,
    pub cf02_q17_missile_max_px: u16,
}

/// One profile set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileSet {
    /// No serde default: a missing field is a parse error that names it.
    pub ca19_kill_policy: KillPolicy,
    /// CA-10: the utility planning cadence.
    pub ca10_decision_cadence_ticks: u16,
    /// CF02-Q16: at most one AI-started move per this many ticks.
    pub cf02_q16_action_cadence_ticks: u16,
    /// CF02-Q55 / CA-26: the AI never bails out (v0 requires `false`).
    pub ca26_ai_bailout: bool,
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
        // CF02-Q38 (section 17(c)): the assistant set never lets the AI defeat.
        if self.cf02_assist.ca19_kill_policy != KillPolicy::Never {
            return bad("cf02_assist.ca19_kill_policy", "CF02-Q38: must be never");
        }
        // CA-21: cf02_assist is Q13's assistant (AI Carl passive); v0 is both.
        if self.cf02_assist.carl.ca21_active {
            return bad(
                "cf02_assist.carl.ca21_active",
                "CF02-Q13: AI Carl stays passive",
            );
        }
        if !self.cf02_assist.donut.ca21_active {
            return bad(
                "cf02_assist.donut.ca21_active",
                "CF02-Q13: AI Donut is the assistant",
            );
        }
        if !self.companion_v0.carl.ca21_active {
            return bad(
                "companion_v0.carl.ca21_active",
                "companion_v0 makes both roles active",
            );
        }
        if !self.companion_v0.donut.ca21_active {
            return bad(
                "companion_v0.donut.ca21_active",
                "companion_v0 makes both roles active",
            );
        }
        Ok(())
    }
}

impl ProfileSet {
    fn validate(&self, set: &str) -> Result<(), ProfileError> {
        let f = |name: &str| format!("{set}.{name}");
        if self.ca10_decision_cadence_ticks == 0 {
            return bad(&f("ca10_decision_cadence_ticks"), "must be positive");
        }
        if self.cf02_q16_action_cadence_ticks == 0 {
            return bad(&f("cf02_q16_action_cadence_ticks"), "must be positive");
        }
        if self.ca26_ai_bailout {
            return bad(
                &f("ca26_ai_bailout"),
                "CF02-Q55: the AI never bails out in v0",
            );
        }
        if self.cf02_q39_min_assist_starts == 0 {
            return bad(&f("cf02_q39_min_assist_starts"), "must be positive");
        }
        if self.cf02_q39_max_hit_share_percent == 0 || self.cf02_q39_max_hit_share_percent > 100 {
            return bad(&f("cf02_q39_max_hit_share_percent"), "must be in 1..=100");
        }
        let carl = &self.carl;
        if carl.ca41_strike_min_px >= carl.ca41_strike_max_px {
            return bad(
                &f("carl.ca41_strike_min_px"),
                "must be below ca41_strike_max_px",
            );
        }
        if carl.ca40_engage_range_px == 0 {
            return bad(&f("carl.ca40_engage_range_px"), "must be positive");
        }
        if carl.ca40_interrupt_range_px == 0 {
            return bad(&f("carl.ca40_interrupt_range_px"), "must be positive");
        }
        let d = &self.donut;
        if d.cf02_q14_swat_range_px == 0 {
            return bad(&f("donut.cf02_q14_swat_range_px"), "must be positive");
        }
        if d.ca41_swat_hit_min_px >= d.ca41_swat_hit_max_px {
            return bad(
                &f("donut.ca41_swat_hit_min_px"),
                "must be below ca41_swat_hit_max_px",
            );
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
            "\"cf02_q16_action_cadence_ticks\": 60",
            "\"cf02_q16_action_cadence_ticks\": 61",
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
        assert_eq!(s.ca19_kill_policy, KillPolicy::Never);
        assert_eq!(
            (
                s.ca10_decision_cadence_ticks,
                s.cf02_q16_action_cadence_ticks
            ),
            (6, 60)
        );
        assert!(!s.ca26_ai_bailout);
        assert!(!s.carl.ca21_active, "Q13: AI Carl is passive");
        assert_eq!(
            (s.carl.ca40_engage_range_px, s.carl.ca40_interrupt_range_px),
            (96, 96)
        );
        let d = &s.donut;
        assert!(d.ca21_active);
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
        assert!(v0.carl.ca21_active && v0.donut.ca21_active);
        assert_eq!(v0.ca19_kill_policy, KillPolicy::Never);
        assert_eq!(
            (
                v0.ca10_decision_cadence_ticks,
                v0.cf02_q16_action_cadence_ticks
            ),
            (6, 60)
        );
        assert!(!v0.ca26_ai_bailout);
        assert_eq!(v0.donut, f.cf02_assist.donut, "Donut's numbers are shared");
        assert_eq!(ProfileSetId::Cf02Assist.name(), "cf02_assist");
        assert_eq!(ProfileSetId::CompanionV0.name(), "companion_v0");
    }

    #[test]
    fn kill_policy_is_required_cf02_assist_stays_never_and_factual_is_data() {
        assert_eq!(KillPolicy::default(), KillPolicy::Never);
        // companion_v0 may carry `factual` (tested as the candidate default in 01E).
        let first = PROFILE_JSON.find("\"never\"").unwrap();
        let second = first + 1 + PROFILE_JSON[first + 1..].find("\"never\"").unwrap();
        let mut factual = PROFILE_JSON.to_string();
        factual.replace_range(second..second + "\"never\"".len(), "\"factual\"");
        let f = ProfileFile::from_json(&factual).unwrap();
        assert_eq!(f.companion_v0.ca19_kill_policy, KillPolicy::Factual);
        assert_eq!(f.cf02_assist.ca19_kill_policy, KillPolicy::Never);
        // cf02_assist with `factual` is a field error (CF02-Q38).
        let text = PROFILE_JSON.replacen("\"never\"", "\"factual\"", 1);
        assert_eq!(field_of(&text), "cf02_assist.ca19_kill_policy");
        // A missing field is a parse error that names it.
        let missing = PROFILE_JSON.replacen("\"ca19_kill_policy\": \"never\",", "", 1);
        match ProfileFile::from_json(&missing) {
            Err(ProfileError::Parse(msg)) => assert!(msg.contains("ca19_kill_policy"), "{msg}"),
            other => panic!("expected a parse error, got {other:?}"),
        }
    }

    #[test]
    fn the_planning_hit_windows_stay_inside_the_move_boxes() {
        use crate::combat::{CARL_HAYMAKER, CARL_JAB, ENEMY_HALF_WIDTH, HitShape};
        use crate::donut_moves::SWAT_BOX;
        let f = file();
        let inside = |lo: u16, hi: u16, box_min: f32, box_max: f32, what: &str| {
            assert!(
                f32::from(lo) >= box_min - ENEMY_HALF_WIDTH
                    && f32::from(hi) <= box_max + ENEMY_HALF_WIDTH,
                "{what}: {lo}..{hi} is outside [{}, {}]",
                box_min - ENEMY_HALF_WIDTH,
                box_max + ENEMY_HALF_WIDTH
            );
        };
        for set in [&f.cf02_assist, &f.companion_v0] {
            for spec in [CARL_JAB, CARL_HAYMAKER] {
                let HitShape::Box { x_min, x_max, .. } = spec.shape else {
                    panic!("{:?} is a box move", spec.id);
                };
                inside(
                    set.carl.ca41_strike_min_px,
                    set.carl.ca41_strike_max_px,
                    x_min,
                    x_max,
                    "carl strike",
                );
            }
            inside(
                set.donut.ca41_swat_hit_min_px,
                set.donut.ca41_swat_hit_max_px,
                SWAT_BOX.x0,
                SWAT_BOX.x1,
                "donut swat",
            );
        }
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
                "\"ca10_decision_cadence_ticks\": 6",
                "\"ca10_decision_cadence_ticks\": 0"
            )),
            "cf02_assist.ca10_decision_cadence_ticks"
        );
        assert_eq!(
            field_of(&swap(
                "\"ca26_ai_bailout\": false",
                "\"ca26_ai_bailout\": true"
            )),
            "cf02_assist.ca26_ai_bailout"
        );
        assert_eq!(
            field_of(&swap(
                "\"cf02_q15_pounce_min_px\": 30",
                "\"cf02_q15_pounce_min_px\": 80"
            )),
            "cf02_assist.donut.cf02_q15_pounce_min_px"
        );
        assert_eq!(
            field_of(&swap("\"ca21_active\": false", "\"ca21_active\": true")),
            "cf02_assist.carl.ca21_active"
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
        let unknown = PROFILE_JSON.replacen("\"ca26_ai_bailout\"", "\"ai_bailout_typo\"", 1);
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
