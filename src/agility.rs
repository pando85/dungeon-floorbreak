//! DONUT-MOBILITY-01 knob schema (DM-A): Donut's agile motion.
//!
//! Bevy-free data contract for `docs/03-tech/donut-mobility-01.md`.
//! [`AgilityTuning`] mirrors `assets/data/gameplay/hero_agility_v0.json` field
//! for field. The file is embedded, `deny_unknown_fields`, validated and FNV-1a
//! pinned like `cf02_combat_v0.json` ([`crate::cf02`]): editing it without
//! updating [`TUNING_HASH`] fails [`AgilityTuning::pinned`] and the pin test, so
//! a tuning edit never changes a recorded outcome silently.
//!
//! DM-A reads the motion knobs (DM-Q01, Q02, Q08). The evasion knobs (DM-Q11,
//! Q12) are declared here so DM-B does not reshape the file; nothing reads them
//! yet. Knob names carry the spec's question ID (`dm_q01_run_speed`). Questions
//! without a key are listed in [`NO_KNOB`] with the reason.

use crate::donut_mana::content_hash;
use crate::prototype::{GRAVITY, HeroMotion, TICK_SECONDS};
use serde::{Deserialize, Serialize};

/// The tuning file, embedded so the binary and the tests read the same bytes.
pub const TUNING_JSON: &str = include_str!("../assets/data/gameplay/hero_agility_v0.json");

/// The committed FNV-1a 64 content hash of [`TUNING_JSON`].
pub const TUNING_HASH: u64 = 0xad4b_7f26_bea9_8c8f;

/// The reflex rearm range of DM-Q12, in ticks (inclusive).
pub const REARM_RANGE: core::ops::RangeInclusive<u32> = 600..=900;

/// The run speed must stay below the scroll camera's 4 px/tick pan cap
/// (240 px/s), so a controlled Donut never outruns the camera (DM-Q01).
pub const RUN_SPEED_CEILING: f32 = 240.0;

/// Question IDs of the spec (1..=27) that have no key in the file, and why.
/// The completeness test requires every other ID to own a `dm_qNN_` key.
pub const NO_KNOB: [(u8, &str); 22] = [
    (3, "shared GRAVITY, unchanged"),
    (4, "derived: air ticks and flat air distance"),
    (5, "no acceleration or deceleration, as Carl"),
    (6, "coyote 6 and jump buffer 7, unchanged"),
    (7, "not scaled: attacks, hops, knockback, collider, hurtbox"),
    (9, "rule: the 2P wall uses each hero's own run speed"),
    (10, "no motion gate: PartyTuning::donut_motion is Agile"),
    (13, "DM-B: the reflex reset rule"),
    (14, "retired: roll mode"),
    (15, "retired: roll mode"),
    (16, "retired: roll mode"),
    (17, "DM-B: the evadable list"),
    (18, "DM-B: the never-evadable list"),
    (19, "DM-B: the evade outcome"),
    (20, "DM-B: the check order"),
    (21, "DM-C: presentation stand-ins"),
    (22, "DM-B: events and hash rule"),
    (23, "retired: the evasion CLI override"),
    (24, "no share target"),
    (25, "Q4 hurtbox, unchanged"),
    (26, "rule: HUD clearance on the drawn top"),
    (27, "room scenario field ceiling_px, not a hero knob"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgilityTuningError {
    /// The JSON did not parse or has an unknown or missing field.
    Parse(String),
    /// A field failed validation; `field` names it.
    Field { field: &'static str, reason: String },
    /// The file's content hash is not the pinned one.
    HashMismatch { expected: u64, actual: u64 },
}

impl core::fmt::Display for AgilityTuningError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "agility tuning: {msg}"),
            Self::Field { field, reason } => {
                write!(f, "agility tuning: field `{field}`: {reason}")
            }
            Self::HashMismatch { expected, actual } => write!(
                f,
                "agility tuning hash {actual:#018x} is not the pinned {expected:#018x}"
            ),
        }
    }
}

impl std::error::Error for AgilityTuningError {}

/// DM-Q11: the evasion mode. `roll` is retired and not a valid value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvasionMode {
    Reflex,
    Off,
}

/// The tuning file, field for field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgilityTuning {
    pub schema_version: u32,
    /// DM-Q01: Donut's run speed, px/s.
    pub dm_q01_run_speed: f32,
    /// DM-Q02: Donut's jump launch speed, px/s.
    pub dm_q02_jump_speed: f32,
    /// DM-Q08: the follower bot's reach margin for Donut, px.
    pub dm_q08_ai_jump_margin_px: f32,
    /// DM-Q11 (DM-B).
    pub dm_q11_evasion_mode: EvasionMode,
    /// DM-Q12 (DM-B): ticks without a hurt before the reflex arms.
    pub dm_q12_reflex_rearm_ticks: u32,
}

impl AgilityTuning {
    /// Parse and validate.
    pub fn from_json(text: &str) -> Result<Self, AgilityTuningError> {
        let tuning: Self =
            serde_json::from_str(text).map_err(|err| AgilityTuningError::Parse(err.to_string()))?;
        tuning.validate()?;
        Ok(tuning)
    }

    /// Parse the embedded file and require the pinned content hash.
    pub fn pinned() -> Result<Self, AgilityTuningError> {
        Self::from_json_pinned(TUNING_JSON, TUNING_HASH)
    }

    /// Parse `text` and require its content hash to equal `expected`.
    pub fn from_json_pinned(text: &str, expected: u64) -> Result<Self, AgilityTuningError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(AgilityTuningError::HashMismatch { expected, actual });
        }
        Self::from_json(text)
    }

    pub fn validate(&self) -> Result<(), AgilityTuningError> {
        fn bad(field: &'static str, reason: &str) -> Result<(), AgilityTuningError> {
            Err(AgilityTuningError::Field {
                field,
                reason: reason.to_string(),
            })
        }
        if self.schema_version != 1 {
            return bad("schema_version", "must be 1");
        }
        let run = self.dm_q01_run_speed;
        if !run.is_finite() || run <= 0.0 || run >= RUN_SPEED_CEILING {
            return bad(
                "dm_q01_run_speed",
                "must be positive and below 240 px/s (the scroll camera's pan cap)",
            );
        }
        let jump = self.dm_q02_jump_speed;
        if !jump.is_finite() || jump <= 0.0 {
            return bad("dm_q02_jump_speed", "must be positive");
        }
        let margin = self.dm_q08_ai_jump_margin_px;
        if !margin.is_finite() || margin < 0.0 {
            return bad("dm_q08_ai_jump_margin_px", "must not be negative");
        }
        if !REARM_RANGE.contains(&self.dm_q12_reflex_rearm_ticks) {
            return bad(
                "dm_q12_reflex_rearm_ticks",
                "must be within 600..=900 ticks",
            );
        }
        Ok(())
    }

    /// Donut's agile [`HeroMotion`].
    pub fn motion(&self) -> HeroMotion {
        HeroMotion {
            run_speed: self.dm_q01_run_speed,
            jump_speed: self.dm_q02_jump_speed,
        }
    }

    /// DM-Q08: the follower bot's reach for Donut: her discrete apex less the
    /// margin (110.92 px at the adopted values).
    pub fn ai_jump_reach_px(&self) -> f32 {
        discrete_apex_px(self.dm_q02_jump_speed) - self.dm_q08_ai_jump_margin_px
    }
}

/// The embedded, pinned tuning, parsed once. Panics on a drifted or invalid
/// embedded file: that is a build defect the pin test also catches.
pub fn tuning() -> &'static AgilityTuning {
    static TUNING: std::sync::OnceLock<AgilityTuning> = std::sync::OnceLock::new();
    TUNING.get_or_init(|| AgilityTuning::pinned().expect("hero_agility_v0.json is pinned"))
}

/// Donut's agile motion from the embedded file.
pub fn agile_motion() -> HeroMotion {
    tuning().motion()
}

/// The highest point (px above the takeoff) of a standing jump launched at
/// `jump_speed`, from the code's discrete step order in `f32` (`vy = J`, then
/// `vy -= g * dt`, then `y += vy * dt`), not from the continuous `J^2 / 2g`.
pub fn discrete_apex_px(jump_speed: f32) -> f32 {
    discrete_arc(jump_speed).0
}

/// The number of ticks a standing jump launched at `jump_speed` spends in the
/// air on flat ground (the tick of the launch counts).
pub fn air_ticks(jump_speed: f32) -> u32 {
    discrete_arc(jump_speed).1
}

/// `(apex, air ticks)` of a flat-ground jump.
fn discrete_arc(jump_speed: f32) -> (f32, u32) {
    let (mut y, mut vy, mut apex, mut ticks) = (0.0_f32, jump_speed, 0.0_f32, 0_u32);
    loop {
        vy -= GRAVITY * TICK_SECONDS;
        y += vy * TICK_SECONDS;
        ticks += 1;
        if y <= 0.0 {
            return (apex, ticks);
        }
        apex = apex.max(y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prototype::JUMP_SPEED;

    fn tuning() -> AgilityTuning {
        AgilityTuning::pinned().expect("the embedded file is pinned")
    }

    #[test]
    fn tuning_hash_is_pinned() {
        assert_eq!(content_hash(TUNING_JSON), TUNING_HASH);
        assert!(AgilityTuning::pinned().is_ok());
    }

    #[test]
    fn a_drifted_file_is_a_clear_hash_failure() {
        let drifted = TUNING_JSON.replace("207.0", "208.0");
        assert_ne!(drifted, TUNING_JSON);
        assert!(matches!(
            AgilityTuning::from_json_pinned(&drifted, TUNING_HASH),
            Err(AgilityTuningError::HashMismatch { .. })
        ));
        assert!(AgilityTuning::from_json(&drifted).is_ok());
    }

    #[test]
    fn the_adopted_values_are_the_spec_values() {
        let t = tuning();
        assert_eq!(t.dm_q01_run_speed, 207.0);
        assert_eq!(t.dm_q02_jump_speed, 500.0);
        assert_eq!(t.dm_q08_ai_jump_margin_px, 4.0);
        assert_eq!(t.dm_q11_evasion_mode, EvasionMode::Reflex);
        assert_eq!(t.dm_q12_reflex_rearm_ticks, 720);
    }

    #[test]
    fn the_discrete_arc_matches_the_spec_numbers() {
        // Spec section 2: apex 114.92 px, 57 air ticks (Carl: 88.54, 50).
        let (apex, ticks) = discrete_arc(500.0);
        assert!((apex - 114.92).abs() < 0.05, "{apex}");
        assert_eq!(ticks, 57);
        let (apex, ticks) = discrete_arc(JUMP_SPEED);
        assert!((apex - 88.54).abs() < 0.05, "{apex}");
        assert_eq!(ticks, 50);
        // DM-Q08: the bot's reach is 110.92, not the continuous 115.05.
        assert!((tuning().ai_jump_reach_px() - 110.92).abs() < 0.05);
    }

    #[test]
    fn the_schema_round_trips_without_losing_or_inventing_a_field() {
        let t = tuning();
        let written = serde_json::to_string_pretty(&t).unwrap();
        assert_eq!(AgilityTuning::from_json(&written).unwrap(), t);
        let mut value: serde_json::Value = serde_json::from_str(TUNING_JSON).unwrap();
        value["extra"] = serde_json::json!(1);
        assert!(AgilityTuning::from_json(&value.to_string()).is_err());
    }

    fn field_of(text: &str) -> &'static str {
        match AgilityTuning::from_json(text) {
            Err(AgilityTuningError::Field { field, .. }) => field,
            other => panic!("expected a field error, got {other:?}"),
        }
    }

    #[test]
    fn validation_names_the_field() {
        let with = |from: &str, to: &str| TUNING_JSON.replace(from, to);
        assert_eq!(
            field_of(&with(
                "\"dm_q01_run_speed\": 207.0",
                "\"dm_q01_run_speed\": 240.0"
            )),
            "dm_q01_run_speed"
        );
        assert_eq!(
            field_of(&with(
                "\"dm_q01_run_speed\": 207.0",
                "\"dm_q01_run_speed\": 0.0"
            )),
            "dm_q01_run_speed"
        );
        assert_eq!(
            field_of(&with(
                "\"dm_q02_jump_speed\": 500.0",
                "\"dm_q02_jump_speed\": -1.0"
            )),
            "dm_q02_jump_speed"
        );
        assert_eq!(
            field_of(&with(
                "\"dm_q08_ai_jump_margin_px\": 4.0",
                "\"dm_q08_ai_jump_margin_px\": -1.0"
            )),
            "dm_q08_ai_jump_margin_px"
        );
        for bad in ["599", "901"] {
            assert_eq!(
                field_of(&with(
                    "\"dm_q12_reflex_rearm_ticks\": 720",
                    &format!("\"dm_q12_reflex_rearm_ticks\": {bad}")
                )),
                "dm_q12_reflex_rearm_ticks"
            );
        }
        for ok in ["600", "900"] {
            assert!(
                AgilityTuning::from_json(&with(
                    "\"dm_q12_reflex_rearm_ticks\": 720",
                    &format!("\"dm_q12_reflex_rearm_ticks\": {ok}")
                ))
                .is_ok()
            );
        }
        // `roll` is retired and not a valid mode.
        assert!(matches!(
            AgilityTuning::from_json(&with("\"reflex\"", "\"roll\"")),
            Err(AgilityTuningError::Parse(_))
        ));
        assert_eq!(
            field_of(&with("\"schema_version\": 1", "\"schema_version\": 2")),
            "schema_version"
        );
    }

    #[test]
    fn every_question_has_a_key_or_a_reason() {
        let keys: Vec<String> = serde_json::from_str::<serde_json::Value>(TUNING_JSON)
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        for id in 1..=27_u8 {
            let owned = keys.iter().any(|k| k.starts_with(&format!("dm_q{id:02}_")));
            let listed = NO_KNOB.iter().any(|(n, _)| *n == id);
            assert!(owned ^ listed, "DM-Q{id:02}: key={owned} no-knob={listed}");
        }
    }

    #[test]
    fn the_run_speed_stays_below_the_camera_cap() {
        let t = tuning();
        assert!(t.dm_q01_run_speed < RUN_SPEED_CEILING);
        assert!(t.dm_q01_run_speed * TICK_SECONDS < 4.0);
    }
}
