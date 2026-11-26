//! DONUT-PROG-01: the run flags and the tuning of the second hero's Floor 01
//! ability progression (DP-A scaffold, spec sections 3 to 5).
//!
//! Bevy-free. DP-A lands the **complete** knob file
//! (`assets/data/gameplay/donut_prog_v0.json`, every key named by its question
//! ID) with its loader, validation and FNV-1a pin, plus the three debug flags
//! `--donut-prog`, `--donut-light-feet` and `--donut-puddle` as a value
//! ([`DonutProgFlags`]) and the single story-setup entry point both programs
//! share ([`story_setup`]). Nothing here reads a knob in the simulation: the
//! boosted launch (DP-B), the blink (DP-C) and the presentation (DP-D) only
//! read values. The capability ids and the unlock rules are the generic ones of
//! [`crate::story_unlock`]; no engine id names the fan layer.

use serde::Deserialize;

use crate::carl_prog::CarlProgFlags;
use crate::donut_mana::{HUD_MANA_SEGMENTS, content_hash};
use crate::story_unlock::{SceneKind, StorySetup, StoryTable};

/// The tuning file, embedded so the binary and the tests read the same bytes.
pub const TUNING_JSON: &str = include_str!("../assets/data/gameplay/donut_prog_v0.json");

/// The committed FNV-1a 64 content hash of [`TUNING_JSON`]. Editing the file
/// without updating this value fails [`DonutProgTuning::pinned`] and the pin
/// test, so a knob edit never changes a recorded outcome silently.
pub const TUNING_HASH: u64 = 0x2c88_960f_7da3_48e8;

/// The `content_hashes` key of the tuning pin (DP-Q46).
pub const PIN_KEY: &str = "donut_prog_tuning";

/// The schema version [`DonutProgTuning`] accepts.
pub const SCHEMA_VERSION: u32 = 1;

/// The story flag `--donut-prog` turns on (its pins and trigger entries).
pub const STORY_FLAG: &str = "donut_prog";

/// The `jump_reach_boost` capability id (DP-Q07).
pub const JUMP_REACH_BOOST: crate::story_unlock::CapabilityId =
    crate::story_unlock::CapabilityId::new("jump_reach_boost");

/// The `special_short_blink` capability id (DP-Q07).
pub const SPECIAL_SHORT_BLINK: crate::story_unlock::CapabilityId =
    crate::story_unlock::CapabilityId::new("special_short_blink");

/// The three debug flags of the run (DP-Q10). All are refused by the command
/// line in the combat lab scene and do nothing to a scripted fixture.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DonutProgFlags {
    /// `--donut-prog`: the pins and trigger entries of Donut's program.
    pub prog: bool,
    /// `--donut-light-feet`: the jump boost is held from tick 0 (implies the
    /// biscuit).
    pub light_feet: bool,
    /// `--donut-puddle`: the short blink is held from tick 0 (implies the jump
    /// boost and the biscuit).
    pub puddle: bool,
}

impl DonutProgFlags {
    pub const OFF: Self = Self {
        prog: false,
        light_feet: false,
        puddle: false,
    };

    pub const fn any(self) -> bool {
        self.prog || self.light_feet || self.puddle
    }

    /// Whether a start flag holds a capability that requires the biscuit (DP-Q06,
    /// DP-Q10): the command line then also sets `--donut-biscuit`, so the
    /// recorded descriptor and a replay agree. `--donut-prog` alone does not:
    /// the biscuit comes from the story.
    pub const fn implies_biscuit(self) -> bool {
        self.light_feet || self.puddle
    }
}

/// The story setup the run flags of **both** programs give `scene`: one
/// [`StorySetup::for_scene`] call with every enabled flag, so the per-hero grant
/// filter sees the whole run (`--carl-prog` alone never grants Donut's rows of a
/// shared trigger, `--donut-prog` alone never grants Carl's, and both together
/// grant both in list order). With no flag it is [`StorySetup::NONE`].
pub fn story_setup(carl: CarlProgFlags, donut: DonutProgFlags, scene: SceneKind) -> StorySetup {
    story_setup_for(carl, donut, scene, None)
}

/// [`story_setup`] for a late-Floor-1 room (PT-SCENE): the pin rows held are
/// the ones named `pin` in the table (a room pin carries one row per story
/// flag), the triggers armed are those of `scene` (the kind the room runs on).
pub fn story_setup_pin(
    carl: CarlProgFlags,
    donut: DonutProgFlags,
    scene: SceneKind,
    pin: &str,
) -> StorySetup {
    story_setup_for(carl, donut, scene, Some(pin))
}

fn story_setup_for(
    carl: CarlProgFlags,
    donut: DonutProgFlags,
    scene: SceneKind,
    pin: Option<&str>,
) -> StorySetup {
    if !carl.any() && !donut.any() {
        return StorySetup::NONE;
    }
    let mut flags: Vec<&str> = Vec::new();
    if carl.prog {
        flags.push(crate::carl_prog::STORY_FLAG);
    }
    if donut.prog {
        flags.push(STORY_FLAG);
    }
    let mut starts: Vec<&str> = Vec::new();
    if carl.dynamite {
        starts.push("carl_dynamite");
    }
    if carl.gauntlet {
        starts.push("carl_gauntlet");
    }
    if donut.light_feet {
        starts.push("donut_light_feet");
    }
    if donut.puddle {
        starts.push("donut_puddle");
    }
    match pin {
        Some(pin) => StorySetup::for_pin(StoryTable::active(), &flags, &starts, scene, pin),
        None => StorySetup::for_scene(StoryTable::active(), &flags, &starts, scene),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuningError {
    /// The JSON did not parse or has an unknown or missing field.
    Parse(String),
    /// A field failed validation; `field` names it.
    Field { field: &'static str, reason: String },
    /// The file's content hash is not the pinned one.
    HashMismatch { expected: u64, actual: u64 },
}

impl core::fmt::Display for TuningError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "donut prog tuning: {msg}"),
            Self::Field { field, reason } => {
                write!(f, "donut prog tuning: field `{field}`: {reason}")
            }
            Self::HashMismatch { expected, actual } => write!(
                f,
                "donut prog tuning hash {actual:#018x} is not the pinned {expected:#018x}"
            ),
        }
    }
}

impl std::error::Error for TuningError {}

/// `assets/data/gameplay/donut_prog_v0.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DonutProgTuning {
    pub schema_version: u32,
    pub provenance: String,
    /// DP-Q14: the absolute launch while the boost is held (replaces the base
    /// launch, never multiplied at runtime).
    pub dpq14_boost_launch_px_s: u16,
    /// DP-Q18: the ledge class the boost reaches and the one nobody reaches.
    pub dpq18_ledge_class_px: u16,
    pub dpq18_unreachable_class_px: u16,
    /// DP-Q21: the horizontal stick threshold of the blink direction.
    pub dpq21_direction_deadzone: f32,
    /// DP-Q22: candidates from the maximum down to the minimum in fixed steps.
    pub dpq22_range_max_px: u16,
    pub dpq22_range_min_px: u16,
    pub dpq22_range_step_px: u16,
    /// DP-Q23: the mana cost, in charges.
    pub dpq23_cost_charges: u8,
    /// DP-Q24: ticks from the arrival to the next press.
    pub dpq24_cooldown_ticks: u16,
    /// DP-Q25: windup t0..t5 (arrival on the tick after), recovery after it.
    pub dpq25_windup_ticks: u8,
    pub dpq25_recovery_ticks: u8,
    /// DP-Q27: the view inset and the support probe length.
    pub dpq27_view_inset_px: u16,
    pub dpq27_support_probe_px: u16,
    /// DP-Q33: the equipped socket's label (fan layer, at most 5 characters).
    pub dpq33_socket_label: String,
    /// DP-Q34: the stand-in presentation counts.
    pub dpq34_crouch_ticks: u8,
    pub dpq34_ripple_rings: u8,
    pub dpq34_streak_frames: u8,
    /// DP-Q45: the surface height above which a boosted room needs a ceiling,
    /// and the highest allowed ceiling underside (the boss room's is lower).
    pub dpq45_surface_limit_px: u16,
    pub dpq45_ceiling_px_max: u16,
    pub dpq45_boss_ceiling_px_max: u16,
    /// DP-Q46: the AI blink's hull half width and the human drawn-view grace.
    pub dpq46_ai_hull_px: u16,
    pub dpq46_drawn_view_grace_ticks: u8,
}

impl DonutProgTuning {
    /// Parse and validate.
    pub fn from_json(text: &str) -> Result<Self, TuningError> {
        let tuning: Self =
            serde_json::from_str(text).map_err(|err| TuningError::Parse(err.to_string()))?;
        tuning.validate()?;
        Ok(tuning)
    }

    /// Parse the embedded file and require the pinned content hash.
    pub fn pinned() -> Result<Self, TuningError> {
        Self::from_json_pinned(TUNING_JSON, TUNING_HASH)
    }

    /// Parse `text` and require its content hash to equal `expected`.
    pub fn from_json_pinned(text: &str, expected: u64) -> Result<Self, TuningError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(TuningError::HashMismatch { expected, actual });
        }
        Self::from_json(text)
    }

    /// The pinned tuning every simulation reads: one shared static.
    pub fn active() -> &'static Self {
        static ACTIVE: std::sync::OnceLock<DonutProgTuning> = std::sync::OnceLock::new();
        ACTIVE.get_or_init(|| {
            Self::pinned().expect("the embedded Donut progression data is pinned and valid")
        })
    }

    pub fn validate(&self) -> Result<(), TuningError> {
        let bad = |field: &'static str, reason: &str| {
            Err(TuningError::Field {
                field,
                reason: reason.to_owned(),
            })
        };
        if self.schema_version != SCHEMA_VERSION {
            return bad("schema_version", "expected 1");
        }
        // DP-Q14: the base launch is 500 (DM-Q02) and 558 is the gate-class
        // ceiling (a 144 px ledge stays out of reach).
        if !(501..=558).contains(&self.dpq14_boost_launch_px_s) {
            return bad(
                "dpq14_boost_launch_px_s",
                "must be 501..=558 (above the base launch, below the 144 px gate class)",
            );
        }
        if self.dpq18_ledge_class_px >= self.dpq18_unreachable_class_px {
            return bad(
                "dpq18_unreachable_class_px",
                "must be above the reachable ledge class",
            );
        }
        let dz = self.dpq21_direction_deadzone;
        if !dz.is_finite() || dz <= 0.0 || dz > 1.0 {
            return bad("dpq21_direction_deadzone", "must be in (0, 1]");
        }
        if self.dpq22_range_min_px == 0 || self.dpq22_range_min_px >= self.dpq22_range_max_px {
            return bad(
                "dpq22_range_min_px",
                "needs 0 < min range < max range (DP-Q22)",
            );
        }
        if self.dpq22_range_step_px == 0
            || !(self.dpq22_range_max_px - self.dpq22_range_min_px)
                .is_multiple_of(self.dpq22_range_step_px)
        {
            return bad(
                "dpq22_range_step_px",
                "must be positive and step from the maximum exactly onto the minimum",
            );
        }
        // `mana_max` is validated equal to the HUD segment count.
        if self.dpq23_cost_charges == 0 || self.dpq23_cost_charges > HUD_MANA_SEGMENTS {
            return bad("dpq23_cost_charges", "must be in 1..=mana_max (DP-Q23)");
        }
        if self.dpq25_windup_ticks == 0 || self.dpq25_recovery_ticks == 0 {
            return bad("dpq25_windup_ticks", "windup and recovery are positive");
        }
        let timeline =
            u16::from(self.dpq25_windup_ticks) + 1 + u16::from(self.dpq25_recovery_ticks);
        if self.dpq24_cooldown_ticks <= timeline {
            return bad(
                "dpq24_cooldown_ticks",
                "must outlast the windup, the arrival tick and the recovery",
            );
        }
        if self.dpq27_view_inset_px == 0 || self.dpq27_support_probe_px == 0 {
            return bad("dpq27_view_inset_px", "insets and probes are positive");
        }
        let label = self.dpq33_socket_label.chars().count();
        if label == 0 || label > 5 || !self.dpq33_socket_label.is_ascii() {
            return bad("dpq33_socket_label", "1..=5 ascii characters (DP-Q33)");
        }
        if self.dpq34_ripple_rings == 0 || self.dpq34_crouch_ticks == 0 {
            return bad("dpq34_ripple_rings", "the cues have at least one frame");
        }
        if self.dpq45_surface_limit_px == 0
            || self.dpq45_boss_ceiling_px_max > self.dpq45_ceiling_px_max
        {
            return bad(
                "dpq45_boss_ceiling_px_max",
                "the boss room's ceiling is at most the lane ceiling (DP-Q45)",
            );
        }
        if self.dpq46_ai_hull_px == 0 || self.dpq46_drawn_view_grace_ticks == 0 {
            return bad("dpq46_ai_hull_px", "hull and grace are positive (DP-Q46)");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_data_file_is_pinned_and_valid() {
        assert_eq!(content_hash(TUNING_JSON), TUNING_HASH);
        let tuning = DonutProgTuning::pinned().expect("pinned tuning loads");
        assert_eq!(tuning.schema_version, SCHEMA_VERSION);
        assert_eq!(DonutProgTuning::active(), &tuning);
    }

    #[test]
    fn the_adopted_values_are_the_spec_ones() {
        let t = DonutProgTuning::active();
        assert_eq!(t.dpq14_boost_launch_px_s, 547);
        assert_eq!(
            (
                t.dpq22_range_max_px,
                t.dpq22_range_min_px,
                t.dpq22_range_step_px
            ),
            (112, 16, 4)
        );
        assert_eq!((t.dpq23_cost_charges, t.dpq24_cooldown_ticks), (2, 540));
        assert_eq!((t.dpq25_windup_ticks, t.dpq25_recovery_ticks), (6, 6));
        assert_eq!((t.dpq27_view_inset_px, t.dpq27_support_probe_px), (24, 256));
        assert_eq!(
            (t.dpq45_ceiling_px_max, t.dpq45_boss_ceiling_px_max),
            (228, 224)
        );
        assert_eq!(
            (t.dpq46_ai_hull_px, t.dpq46_drawn_view_grace_ticks),
            (63, 12)
        );
        // 25 candidates from 112 down to 16 in steps of 4.
        assert_eq!(
            (t.dpq22_range_max_px - t.dpq22_range_min_px) / t.dpq22_range_step_px + 1,
            25
        );
    }

    #[test]
    fn every_knob_name_starts_with_its_question_id() {
        let value: serde_json::Value = serde_json::from_str(TUNING_JSON).unwrap();
        for key in value.as_object().unwrap().keys() {
            let knob = key != "schema_version" && key != "provenance";
            assert!(
                !knob || key.starts_with("dpq"),
                "{key}: a knob name starts with its question ID"
            );
        }
    }

    /// The data contract is IP-neutral: no key names a franchise item or
    /// character (the label value and the provenance are the fan layer).
    #[test]
    fn no_knob_key_names_a_franchise_item_or_character() {
        let banned = [
            "donut", "biscuit", "puddle", "juicer", "carl", "feet", "jumper",
        ];
        let value: serde_json::Value = serde_json::from_str(TUNING_JSON).unwrap();
        for key in value.as_object().unwrap().keys() {
            for word in banned {
                assert!(!key.contains(word), "knob key {key} says {word}");
            }
        }
    }

    #[test]
    fn a_drifted_file_fails_the_pin_and_a_bad_field_is_named() {
        let drifted = TUNING_JSON.replace(
            "\"dpq24_cooldown_ticks\": 540",
            "\"dpq24_cooldown_ticks\": 541",
        );
        assert_ne!(drifted, TUNING_JSON);
        match DonutProgTuning::from_json_pinned(&drifted, TUNING_HASH) {
            Err(TuningError::HashMismatch { expected, .. }) => assert_eq!(expected, TUNING_HASH),
            other => panic!("a drifted file must fail the pin, got {other:?}"),
        }
        let field = |from: &str, to: &str| {
            let text = TUNING_JSON.replacen(from, to, 1);
            assert_ne!(text, TUNING_JSON, "{from}");
            match DonutProgTuning::from_json(&text) {
                Err(TuningError::Field { field, .. }) => field,
                other => panic!("expected a field error for {to}, got {other:?}"),
            }
        };
        // DP-Q14: 500 and 559 are rejected (the range is 501..=558).
        for bad in ["500", "559"] {
            assert_eq!(
                field(
                    "\"dpq14_boost_launch_px_s\": 547",
                    &format!("\"dpq14_boost_launch_px_s\": {bad}")
                ),
                "dpq14_boost_launch_px_s"
            );
        }
        for good in ["501", "558"] {
            let text = TUNING_JSON.replacen(
                "\"dpq14_boost_launch_px_s\": 547",
                &format!("\"dpq14_boost_launch_px_s\": {good}"),
                1,
            );
            DonutProgTuning::from_json(&text).expect(good);
        }
        // The cost cannot exceed the pool.
        assert_eq!(
            field("\"dpq23_cost_charges\": 2", "\"dpq23_cost_charges\": 9"),
            "dpq23_cost_charges"
        );
        // The minimum range stays under the maximum.
        assert_eq!(
            field("\"dpq22_range_min_px\": 16", "\"dpq22_range_min_px\": 112"),
            "dpq22_range_min_px"
        );
        assert_eq!(
            field("\"dpq22_range_step_px\": 4", "\"dpq22_range_step_px\": 5"),
            "dpq22_range_step_px"
        );
        assert_eq!(
            field(
                "\"dpq33_socket_label\": \"BLINK\"",
                "\"dpq33_socket_label\": \"\""
            ),
            "dpq33_socket_label"
        );
        assert_eq!(
            field(
                "\"dpq24_cooldown_ticks\": 540",
                "\"dpq24_cooldown_ticks\": 13"
            ),
            "dpq24_cooldown_ticks"
        );
        assert!(matches!(
            DonutProgTuning::from_json("{\"schema_version\": 1, \"bogus\": 1}"),
            Err(TuningError::Parse(_))
        ));
    }

    #[test]
    fn no_flag_is_no_story_and_each_flag_maps_to_its_setup() {
        use crate::story_unlock::ReadThrough;
        for scene in [
            SceneKind::Corridor,
            SceneKind::Demo,
            SceneKind::CfArena,
            SceneKind::Boss,
        ] {
            assert!(story_setup(CarlProgFlags::OFF, DonutProgFlags::OFF, scene).is_inert());
        }
        let prog = DonutProgFlags {
            prog: true,
            ..DonutProgFlags::OFF
        };
        for scene in [SceneKind::Demo, SceneKind::CfArena] {
            assert!(
                story_setup(CarlProgFlags::OFF, prog, scene).is_inert(),
                "nothing held and nothing armed on the pre-biscuit rooms"
            );
        }
        let boss = story_setup(CarlProgFlags::OFF, prog, SceneKind::Boss);
        assert_eq!(boss.holds(), [JUMP_REACH_BOOST], "DP-Q09 boss pin");
        assert_eq!(boss.grant_heroes(), Some(&[1u8][..]));
        assert_eq!(boss.trigger_ids(), ["brute_boss_f01_defeated"]);
        // REST-WIRE (REST-AD3, REST-AD18): the corridor arms nothing; the refuge holds
        // nothing and arms the safe-room reward trigger.
        let corridor = story_setup(CarlProgFlags::OFF, prog, SceneKind::Corridor);
        assert!(corridor.is_inert());
        let refuge = story_setup(CarlProgFlags::OFF, prog, SceneKind::RestRefuge);
        assert!(refuge.holds().is_empty());
        assert_eq!(refuge.trigger_ids(), ["f01_safe_room_reward_found"]);
        // Start flags grant the `requires` closure and arm nothing.
        let puddle = DonutProgFlags {
            puddle: true,
            ..DonutProgFlags::OFF
        };
        let held = story_setup(CarlProgFlags::OFF, puddle, SceneKind::CfArena);
        assert_eq!(held.holds().len(), 2, "blink and, required, the boost");
        assert!(held.holds().contains(&JUMP_REACH_BOOST));
        assert!(held.holds().contains(&SPECIAL_SHORT_BLINK));
        assert!(held.trigger_ids().is_empty());
        let light = DonutProgFlags {
            light_feet: true,
            ..DonutProgFlags::OFF
        };
        assert_eq!(
            story_setup(CarlProgFlags::OFF, light, SceneKind::Demo).holds(),
            [JUMP_REACH_BOOST]
        );
        assert!(puddle.implies_biscuit() && light.implies_biscuit() && !prog.implies_biscuit());
        // With the biscuit read through, the setups pass the setup check.
        let view = ReadThrough {
            primary_ranged: true,
        };
        assert!(held.check_read_through(&view).is_ok());
        // Without it the story order fails, naming the start flag.
        let error = held
            .check_read_through(&ReadThrough::default())
            .expect_err("the biscuit is locked");
        assert!(error.to_string().contains("start_flags[donut_puddle]"));
        // Both programs: one setup, the union of the pins and every heroes row.
        let both = story_setup(
            CarlProgFlags {
                prog: true,
                ..CarlProgFlags::OFF
            },
            prog,
            SceneKind::Boss,
        );
        assert_eq!(both.holds().len(), 2, "the dynamite and the boost");
        assert_eq!(both.grant_heroes(), Some(&[0u8, 1][..]));
    }
}
