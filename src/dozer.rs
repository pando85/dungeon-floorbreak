//! DOZER-CORE: the revenge machine of room 3, pure core
//! (`docs/01-design/enemy-gameplay-dozer.md`, DZ-Q01..DZ-Q25). Bevy-free,
//! deterministic, headless.
//!
//! Engine names are IP-neutral (`dozer`, a tracked pursuit machine, DZ-AD18).
//! This module owns everything that does not need the scene:
//!
//! * [`DozerTuning`]: every `dzq##` knob of
//!   `assets/data/gameplay/enemy_dozer_v0.json`, loaded through a validator whose
//!   errors name the field (the K5 invariant included), FNV-1a pinned
//!   ([`TUNING_HASH`]);
//! * [`DozerProfile`] and its speed band ([`Band`]), the [`release_window`]
//!   helper (the single definition of the stop frame's useful window);
//! * [`Dozer`], the state machine: dormant, approaching, committed chase,
//!   escaped (brake, strain, idle), impact stunned and disabled, with the wheel
//!   circle and its Dozer-owned persistent `Wheel` volume, the push box and its
//!   pinned rule, the nav rows and the solo rescue (`place`);
//! * [`DozerEvent`], the sub-events of `CombatEvent::Dozer`.
//!
//! What it does **not** do: read a scene or touch a body. The slot module
//! (`combat_scene/dozer.rs`) wraps it for the scene.
//!
//! # Coordinates
//!
//! World px, y up. The rig origin `O` is the tread midpoint (world x); the
//! machine always faces right. Every box is O-relative: x forward, y up from
//! the walking line ([`Dozer::floor_y`]). Nav rows are layout px (y down), as
//! the feed expects.
//!
//! # Tick order (one [`Dozer::step`] per scene tick)
//!
//! 1. the state's move (or hold): trigger, rumble, band and speed, brake;
//! 2. the push-out of every hero body that overlaps the push box at the
//!    candidate origin; if any shift would put a body into a solid, the move
//!    is reverted and no hero is shifted this tick (the pinned rule);
//! 3. the solo rescue of a bot (`place`), on the post-shift positions;
//! 4. the wheel test on the post-shift positions, the rescued bots excluded.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::boss::{HeroBlow, HeroView};
use crate::camera_lab::room::IRect;
use crate::donut_mana::content_hash;
use crate::env_hazard::{
    HazardClass, HazardSpec, HazardVolume, HazardWindow, NavPhase, NavRect, Victim,
};
use crate::hero_damage::HeroDamagePolicy;
use crate::party::HeroId;
use crate::prototype::{
    Block, HERO_HALF_HEIGHT, HERO_HALF_WIDTH, HeroMotion, RUN_SPEED, Rect, TICK_SECONDS,
};

/// The Dozer's target id (`SlotEnemy::target_id`): the slot id range, after
/// the slimes (DZ-IMPL-AD: no id was specified).
pub const DOZER_ID: u32 = crate::scene_enemy::SLOT_ID_MIN + 0x200;
/// DZ-Q04: the wheel's `HeroBlow.source` ("DZ").
pub const DOZER_SOURCE_ID: u32 = 0x445A;
/// DZ-Q04: the fixed `hp()` no hit lowers.
pub const DOZER_HP: u16 = 255;
/// The nav feed id family of the Dozer's rows (`0x0400 + row`).
pub const FEED_ID_DOZER: u16 = 0x0400;
/// K5: revenge's cruise ceiling (DZ-Q06).
pub const REVENGE_CRUISE_MAX_PX_S: u16 = 170;
/// The hull top must clear the boosted apex by this many px (section 14).
pub const HULL_APEX_MARGIN_PX: f32 = 16.0;

/// The tuning file, embedded so the binary and the tests read the same bytes.
pub const TUNING_JSON: &str = include_str!("../assets/data/gameplay/enemy_dozer_v0.json");
/// The committed FNV-1a 64 content hash of [`TUNING_JSON`].
pub const TUNING_HASH: u64 = 0x146c_8ef7_91c8_e9e9;
/// The `content_hashes` key of the tuning pin (registered by DR-A).
pub const PIN_KEY: &str = "dozer_tuning";
const SCHEMA_VERSION: u32 = 1;

/// ACT1-B: the opening profile's file (the chase of `act1_chase`), merged over the shared
/// machine values of [`TUNING_JSON`] when a site says `profile: "opening"`. Room 3's
/// [`TUNING_HASH`] does not move: the opening values live only here.
pub const OPENING_TUNING_JSON: &str =
    include_str!("../assets/data/gameplay/enemy_dozer_opening_v0.json");
/// The committed FNV-1a 64 content hash of [`OPENING_TUNING_JSON`].
pub const OPENING_TUNING_HASH: u64 = 0xb813_c4a6_57b1_65a7;
/// The `content_hashes` key of the opening tuning pin.
pub const OPENING_PIN_KEY: &str = "dozer_opening_tuning";
const OPENING_SCHEMA: &str = "dozer-opening-v0";
/// A wheel front within this many px short of a solid counts as touching it, so the f32
/// accumulation of a charge that reaches the barricade exactly on its last tick still
/// counts (ACT1-IMP-10).
const REACH_EPS_PX: f32 = 0.01;

// ------------------------------------------------------------------ tuning --

/// A knob file problem. A load error names the field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DozerError {
    Parse(String),
    Field { field: &'static str, reason: String },
    HashMismatch { expected: u64, actual: u64 },
}

impl core::fmt::Display for DozerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "dozer tuning: {msg}"),
            Self::Field { field, reason } => write!(f, "dozer tuning: field `{field}`: {reason}"),
            Self::HashMismatch { expected, actual } => write!(
                f,
                "dozer tuning hash {actual:#018x} is not the pinned {expected:#018x}"
            ),
        }
    }
}

impl std::error::Error for DozerError {}

fn field(field: &'static str, reason: impl Into<String>) -> DozerError {
    DozerError::Field {
        field,
        reason: reason.into(),
    }
}

/// A point in O-relative px (x forward, y up from the walking line).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PointPx {
    pub x: i32,
    pub y: i32,
}

/// An O-relative box (x forward, y up from the walking line).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpanPx {
    pub x0: i32,
    pub x1: i32,
    pub y0: i32,
    pub y1: i32,
}

/// The room 3 speed profile `revenge` (DZ-Q06..DZ-Q10, DZ-Q25).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevengeData {
    pub dzq06_cruise_px_s: u16,
    pub dzq07_catch_up_px_s: u16,
    pub dzq07_catch_up_gap_px: u16,
    pub dzq08_mercy_px_s: u16,
    pub dzq08_mercy_gap_px: u16,
    pub dzq09_accel_px_s_per_tick: u16,
    /// DZ-Q10: `null` in room 3 (DZ-AD3); the charge is CHASE-01's append.
    pub dzq10_charge: Option<serde_json::Value>,
    /// DZ-Q25: the target while no living hero is ahead of the axle.
    pub dzq25_no_hero_ahead_target: String,
}

/// Every DZ knob of the machine file (section 12).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DozerTuning {
    pub schema_version: u32,
    pub dzq01_wheel_axle: PointPx,
    pub dzq01_wheel_r_px: u16,
    pub dzq02_push_box: SpanPx,
    pub dzq03_impact_collider: SpanPx,
    pub dzq04_wheel_damage: u8,
    pub dzq04_knock_x_px_s: u16,
    pub dzq04_knock_up_px_s: u16,
    pub dzq04_source_id: u32,
    pub dzq04_hp_constant: u16,
    pub dzq05_clang_hit_stop: String,
    pub dzq12_rumble_ticks: u16,
    pub dzq13_emerge_speed: String,
    pub dzq14_brake_ticks: u16,
    pub dzq14_brake_slide_px: u16,
    pub dzq14_strain_ticks: u16,
    pub dzq14_lethal_in_brake: bool,
    pub dzq15_stunned_ticks: u16,
    pub dzq16_nav_lead_px: u16,
    pub dzq16_nav_grid_px: u16,
    pub dzq17_rescue_band_px: u16,
    pub dzq17_rescue_ahead_px: u16,
    pub dzq17_rescue_zone_margin_px: u16,
    pub dzq17_rescue_past_stop_px: u16,
    pub dzq17_rescue_exit_margin_px: u16,
    pub dzq17_lane_spot_stop_margin_px: u16,
    pub dzq17_lane_spot_view_margin_px: u16,
    pub dzq17_rescue_min_spacing_ticks: u16,
    pub dzq23_melee_recovery_ticks: u16,
    pub dzq23_min_release_window_ticks: u16,
    pub dzq23_min_visible_lead_ticks: u16,
    pub dzq24_ca2dz_autonomous_rescues_max: u32,
    pub dzq24_ca2dz_assist_rescues_per_run_max: f32,
    pub dzq24_ca2dz_toward_regroups_max: u32,
    pub dzq24_stuck_ticks_max: u32,
    pub revenge: RevengeData,
    /// ACT1-B: the opening profile's charge block (`dzq10_charge`); `None` in the
    /// shared file (room 3 has no charge, DZ-AD3). Never read from `enemy_dozer_v0.json`.
    #[serde(skip)]
    pub charge: Option<ChargeTuning>,
}

/// ACT1-B: the opening's charge (`dzq10_charge` of `enemy_dozer_opening_v0.json`, DOZ-01
/// section 9). Present only in the opening tuning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChargeTuning {
    /// The leading hero's x that arms the windup (presentation only).
    pub arm_x: i32,
    /// The commit gap: the rearmost body's left edge minus the wheel front, at least.
    pub min_gap_px: i32,
    pub tell_ticks: u16,
    pub speed_px_s: u16,
    pub max_ticks: u16,
    pub brake_ticks: u16,
    pub brake_slide_px: u16,
    pub strain_ticks: u16,
    /// The walk-to-the-pocket margin (ticks) the commit rule requires.
    pub reach_margin_ticks: u16,
    pub charges_per_attempt: u8,
}

/// The shape of `enemy_dozer_opening_v0.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpeningFile {
    schema: String,
    #[allow(dead_code)]
    provenance: String,
    dzq06_cruise_px_s: u16,
    dzq07_catch_up_px_s: u16,
    dzq07_catch_up_gap_px: u16,
    dzq08_mercy_px_s: u16,
    dzq08_mercy_gap_px: u16,
    dzq09_accel_px_s_per_tick: u16,
    dzq25_no_hero_ahead_target: String,
    dzq10_charge: ChargeTuning,
}

impl DozerTuning {
    /// Parse and validate.
    pub fn from_json(text: &str) -> Result<Self, DozerError> {
        let tuning: Self =
            serde_json::from_str(text).map_err(|err| DozerError::Parse(err.to_string()))?;
        tuning.validate()?;
        Ok(tuning)
    }

    /// Parse the embedded file and require the pinned content hash.
    pub fn pinned() -> Result<Self, DozerError> {
        Self::from_json_pinned(TUNING_JSON, TUNING_HASH)
    }

    /// Parse `text` and require its content hash to equal `expected`.
    pub fn from_json_pinned(text: &str, expected: u64) -> Result<Self, DozerError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(DozerError::HashMismatch { expected, actual });
        }
        Self::from_json(text)
    }

    /// The pinned tuning every simulation reads.
    pub fn active() -> &'static Self {
        static ACTIVE: OnceLock<DozerTuning> = OnceLock::new();
        ACTIVE.get_or_init(|| Self::pinned().expect("the embedded dozer data is pinned and valid"))
    }

    /// ACT1-B: the opening profile's tuning: the shared machine values of the pinned
    /// `enemy_dozer_v0.json` with the speed band and the charge block of the pinned
    /// `enemy_dozer_opening_v0.json` laid over them.
    pub fn opening() -> &'static Self {
        static OPENING: OnceLock<DozerTuning> = OnceLock::new();
        OPENING.get_or_init(|| {
            Self::opening_pinned().expect("the embedded opening data is pinned and valid")
        })
    }

    /// Parse the embedded opening file and require its pinned content hash.
    pub fn opening_pinned() -> Result<Self, DozerError> {
        Self::opening_from_json_pinned(OPENING_TUNING_JSON, OPENING_TUNING_HASH)
    }

    /// Parse `text` as the opening file and require its content hash to equal `expected`.
    pub fn opening_from_json_pinned(text: &str, expected: u64) -> Result<Self, DozerError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(DozerError::HashMismatch { expected, actual });
        }
        Self::opening_from_json(text)
    }

    /// Parse and validate the opening file over the shared tuning.
    pub fn opening_from_json(text: &str) -> Result<Self, DozerError> {
        let file: OpeningFile =
            serde_json::from_str(text).map_err(|err| DozerError::Parse(err.to_string()))?;
        if file.schema != OPENING_SCHEMA {
            return Err(field("schema", format!("must be {OPENING_SCHEMA:?}")));
        }
        let mut merged = Self::active().clone();
        merged.revenge = RevengeData {
            dzq06_cruise_px_s: file.dzq06_cruise_px_s,
            dzq07_catch_up_px_s: file.dzq07_catch_up_px_s,
            dzq07_catch_up_gap_px: file.dzq07_catch_up_gap_px,
            dzq08_mercy_px_s: file.dzq08_mercy_px_s,
            dzq08_mercy_gap_px: file.dzq08_mercy_gap_px,
            dzq09_accel_px_s_per_tick: file.dzq09_accel_px_s_per_tick,
            dzq10_charge: None,
            dzq25_no_hero_ahead_target: file.dzq25_no_hero_ahead_target,
        };
        merged.charge = Some(file.dzq10_charge);
        merged.validate_opening()?;
        Ok(merged)
    }

    /// The opening's rules: every value positive, the band ordered, the K5 ceiling on
    /// the band speeds (the charge speed is exempt, Dozer note section 4.4) and the
    /// charge at least as fast as the cruise.
    pub fn validate_opening(&self) -> Result<(), DozerError> {
        // The shared rules (K5 included) hold for the band speeds; the charge block sits
        // beside them, never in `revenge.dzq10_charge`.
        self.validate()?;
        let Some(c) = self.charge else {
            return Err(field("dzq10_charge", "the opening needs a charge block"));
        };
        let r = &self.revenge;
        for (name, v) in [
            ("dzq06_cruise_px_s", u32::from(r.dzq06_cruise_px_s)),
            ("dzq07_catch_up_px_s", u32::from(r.dzq07_catch_up_px_s)),
            ("dzq07_catch_up_gap_px", u32::from(r.dzq07_catch_up_gap_px)),
            ("dzq08_mercy_px_s", u32::from(r.dzq08_mercy_px_s)),
            ("dzq08_mercy_gap_px", u32::from(r.dzq08_mercy_gap_px)),
            (
                "dzq09_accel_px_s_per_tick",
                u32::from(r.dzq09_accel_px_s_per_tick),
            ),
            ("dzq10_charge.tell_ticks", u32::from(c.tell_ticks)),
            ("dzq10_charge.speed_px_s", u32::from(c.speed_px_s)),
            ("dzq10_charge.max_ticks", u32::from(c.max_ticks)),
            ("dzq10_charge.brake_ticks", u32::from(c.brake_ticks)),
            ("dzq10_charge.brake_slide_px", u32::from(c.brake_slide_px)),
            ("dzq10_charge.strain_ticks", u32::from(c.strain_ticks)),
            (
                "dzq10_charge.reach_margin_ticks",
                u32::from(c.reach_margin_ticks),
            ),
            (
                "dzq10_charge.charges_per_attempt",
                u32::from(c.charges_per_attempt),
            ),
            ("dzq10_charge.arm_x", c.arm_x.max(0) as u32),
            ("dzq10_charge.min_gap_px", c.min_gap_px.max(0) as u32),
        ] {
            if v == 0 {
                return Err(field(name, "must be positive"));
            }
        }
        if c.speed_px_s < r.dzq06_cruise_px_s {
            return Err(field(
                "dzq10_charge.speed_px_s",
                "the charge is at least the cruise",
            ));
        }
        Ok(())
    }

    /// Every rule a file must pass, K5 included (section 4.2).
    pub fn validate(&self) -> Result<(), DozerError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(field("schema_version", format!("must be {SCHEMA_VERSION}")));
        }
        if self.dzq01_wheel_r_px == 0 {
            return Err(field("dzq01_wheel_r_px", "must be positive"));
        }
        for (name, b) in [
            ("dzq02_push_box", self.dzq02_push_box),
            ("dzq03_impact_collider", self.dzq03_impact_collider),
        ] {
            if b.x0 >= b.x1 || b.y0 >= b.y1 || b.y0 < 0 {
                return Err(field(name, "needs x0 < x1, 0 <= y0 < y1"));
            }
        }
        if self.dzq04_source_id <= 0xFF || self.dzq04_source_id != DOZER_SOURCE_ID {
            return Err(field(
                "dzq04_source_id",
                format!("must be the Dozer's own source {DOZER_SOURCE_ID:#x}"),
            ));
        }
        if self.dzq04_hp_constant == 0 {
            return Err(field("dzq04_hp_constant", "must be nonzero"));
        }
        if self.dzq05_clang_hit_stop != "move" {
            return Err(field(
                "dzq05_clang_hit_stop",
                "only \"move\" (the move's own hit-stop) is defined",
            ));
        }
        if self.dzq13_emerge_speed != "target" {
            return Err(field(
                "dzq13_emerge_speed",
                "only \"target\" (the band's target) is defined",
            ));
        }
        if self.dzq14_brake_ticks == 0 {
            return Err(field("dzq14_brake_ticks", "must be at least 1"));
        }
        if self.dzq16_nav_grid_px == 0 {
            return Err(field("dzq16_nav_grid_px", "must be positive"));
        }
        if self.dzq24_ca2dz_assist_rescues_per_run_max.is_nan()
            || self.dzq24_ca2dz_assist_rescues_per_run_max < 0.0
        {
            return Err(field(
                "dzq24_ca2dz_assist_rescues_per_run_max",
                "must be a non-negative number",
            ));
        }
        let r = &self.revenge;
        if r.dzq10_charge.is_some() {
            return Err(field(
                "revenge.dzq10_charge",
                "room 3 has no charge (DZ-AD3): must be null",
            ));
        }
        if r.dzq25_no_hero_ahead_target != "cruise" {
            return Err(field(
                "revenge.dzq25_no_hero_ahead_target",
                "only \"cruise\" is defined",
            ));
        }
        if !(r.dzq08_mercy_px_s <= r.dzq06_cruise_px_s
            && r.dzq06_cruise_px_s <= r.dzq07_catch_up_px_s)
        {
            return Err(field(
                "revenge.dzq06_cruise_px_s",
                "the band needs mercy <= cruise <= catch_up",
            ));
        }
        if r.dzq08_mercy_gap_px >= r.dzq07_catch_up_gap_px {
            return Err(field(
                "revenge.dzq08_mercy_gap_px",
                "must be below dzq07_catch_up_gap_px",
            ));
        }
        if r.dzq09_accel_px_s_per_tick == 0 {
            return Err(field(
                "revenge.dzq09_accel_px_s_per_tick",
                "must be positive",
            ));
        }
        // K5: no charge block, so every speed is at most RUN_SPEED and every
        // hero's run speed; revenge's cruise at most 170.
        let ceiling = k5_speed_ceiling();
        for (name, v) in [
            ("revenge.dzq06_cruise_px_s", r.dzq06_cruise_px_s),
            ("revenge.dzq07_catch_up_px_s", r.dzq07_catch_up_px_s),
            ("revenge.dzq08_mercy_px_s", r.dzq08_mercy_px_s),
        ] {
            if f32::from(v) > ceiling {
                return Err(field(
                    name,
                    format!("K5: {v} px/s is above the slowest hero's run speed {ceiling}"),
                ));
            }
        }
        if r.dzq06_cruise_px_s > REVENGE_CRUISE_MAX_PX_S {
            return Err(field(
                "revenge.dzq06_cruise_px_s",
                format!("K5: revenge cruise must be at most {REVENGE_CRUISE_MAX_PX_S}"),
            ));
        }
        Ok(())
    }

    /// The room 3 profile in simulation units.
    pub fn revenge(&self) -> DozerProfile {
        let r = &self.revenge;
        DozerProfile {
            cruise: f32::from(r.dzq06_cruise_px_s),
            catch_up: f32::from(r.dzq07_catch_up_px_s),
            catch_up_gap: f32::from(r.dzq07_catch_up_gap_px),
            mercy: f32::from(r.dzq08_mercy_px_s),
            mercy_gap: f32::from(r.dzq08_mercy_gap_px),
            accel: f32::from(r.dzq09_accel_px_s_per_tick),
        }
    }

    /// The wheel front, O-relative (axle x + r).
    pub fn wheel_front(&self) -> f32 {
        (self.dzq01_wheel_axle.x + i32::from(self.dzq01_wheel_r_px)) as f32
    }

    /// The spike tips, O-relative (the impact collider's front).
    pub fn tips(&self) -> f32 {
        self.dzq03_impact_collider.x1 as f32
    }
}

/// K5's ceiling: `RUN_SPEED` and every hero's run speed (Carl's legacy motion,
/// Donut's agile motion).
pub fn k5_speed_ceiling() -> f32 {
    let donut = crate::agility::agile_motion().run_speed;
    RUN_SPEED.min(HeroMotion::CARL.run_speed).min(donut)
}

// ------------------------------------------------------------- the profile --

/// The band the target speed is in (section 4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Band {
    CatchUp,
    Cruise,
    Mercy,
}

impl Band {
    pub const ALL: [Self; 3] = [Self::CatchUp, Self::Cruise, Self::Mercy];

    pub const fn name(self) -> &'static str {
        match self {
            Self::CatchUp => "catch_up",
            Self::Cruise => "cruise",
            Self::Mercy => "mercy",
        }
    }

    pub const fn index(self) -> usize {
        self as usize
    }
}

/// A speed profile in px/s (`revenge` in room 3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DozerProfile {
    pub cruise: f32,
    pub catch_up: f32,
    pub catch_up_gap: f32,
    pub mercy: f32,
    pub mercy_gap: f32,
    /// The most the speed moves toward the target per tick.
    pub accel: f32,
}

impl DozerProfile {
    /// The band for `gap` (`None`: no living hero ahead of the axle, DZ-Q25's
    /// cruise). Inclusive at the lower edge: the mercy gap and the catch-up gap
    /// are both cruise.
    pub fn band(&self, gap: Option<f32>) -> Band {
        match gap {
            Some(g) if g > self.catch_up_gap => Band::CatchUp,
            Some(g) if g < self.mercy_gap => Band::Mercy,
            _ => Band::Cruise,
        }
    }

    pub fn speed_of(&self, band: Band) -> f32 {
        match band {
            Band::CatchUp => self.catch_up,
            Band::Cruise => self.cruise,
            Band::Mercy => self.mercy,
        }
    }

    /// One tick of the speed toward `target`, by at most `accel`.
    pub fn approach(&self, speed: f32, target: f32) -> f32 {
        if speed < target {
            (speed + self.accel).min(target)
        } else {
            (speed - self.accel).max(target)
        }
    }
}

/// A stop frame as [`release_window`] reads it (the frame lands in DR-B): its
/// footprint and the ticks from the release to the landing evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StopFrame {
    pub footprint_x0: f32,
    pub footprint_x1: f32,
    /// Ticks from the release tick to the landing evaluation (30 + 8 − 1 = 37
    /// in room 3: the landing is evaluated on R + 37).
    pub land_delay_ticks: u32,
    /// The wheel front at emergence (the chase starts there).
    pub emerge_front_x: f32,
}

/// The useful release window of a frame for a hero who waits at `hero_x`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ReleaseWindow {
    /// Useful release ticks (counted from the first chase tick).
    pub ticks: u32,
    pub first_tick: Option<u32>,
    pub last_tick: Option<u32>,
    /// The wheel front on the first and the last useful release tick.
    pub first_front: f32,
    pub last_front: f32,
    /// DR-B: the release ticks whose landing meets the front collider, with or
    /// without the hero's recovery (the set DR-B's sweep compares with the
    /// simulation), and their first and last tick.
    pub hit_ticks: u32,
    pub hit_first_tick: Option<u32>,
    pub hit_last_tick: Option<u32>,
    /// DR-B: the release ticks evaluated (0 .. `evaluated`: every chase tick
    /// whose wheel front is still short of the hero's body), the domain of
    /// DR-B's sweep.
    pub evaluated: u32,
}

/// Section 6, the single definition of the useful release window. The machine
/// chases a hero who stands still with its centre at `hero_x`; a release on
/// chase tick R is **useful** when the landing on R + `land_delay_ticks` meets
/// the front impact collider and the wheel front on R + `recovery_ticks` has
/// not yet reached the hero's body (the hero can still turn and run). The band
/// is integrated tick by tick exactly as [`Dozer::step`] does (the gap of a
/// hero whose centre is ahead of the axle, negative once the front has passed
/// its left edge), and the landing is the env step's swept test (DR-B): the
/// collider of the landing tick joined with the one of the tick before, each
/// rounded to layout px, overlaps the footprint.
pub fn release_window(
    profile: &DozerProfile,
    frame: &StopFrame,
    hero_x: f32,
    recovery_ticks: u32,
) -> ReleaseWindow {
    let hits = release_hits(profile, frame, hero_x);
    let mut out = ReleaseWindow {
        evaluated: hits.len() as u32,
        ..ReleaseWindow::default()
    };
    let fronts = chase_fronts(profile, frame, hero_x, recovery_ticks);
    let hero_left = hero_x - HERO_HALF_WIDTH;
    for (r, &hits) in hits.iter().enumerate() {
        let recovered = fronts[r + recovery_ticks as usize];
        if hits {
            out.hit_first_tick.get_or_insert(r as u32);
            out.hit_last_tick = Some(r as u32);
            out.hit_ticks += 1;
        }
        if hits && recovered < hero_left {
            if out.first_tick.is_none() {
                out.first_tick = Some(r as u32);
                out.first_front = fronts[r];
            }
            out.last_tick = Some(r as u32);
            out.last_front = fronts[r];
            out.ticks += 1;
        }
    }
    out
}

/// The wheel front of every chase tick for a hero who stands still at
/// `hero_x`: every tick until the front reaches the hero's left edge (the
/// release domain), then `land_delay_ticks` and `recovery_ticks` more so
/// every release in the domain has its landing and its recovery.
fn chase_fronts(
    profile: &DozerProfile,
    frame: &StopFrame,
    hero_x: f32,
    recovery_ticks: u32,
) -> Vec<f32> {
    let t = DozerTuning::active();
    let axle_behind = t.wheel_front() - t.dzq01_wheel_axle.x as f32;
    let hero_left = hero_x - HERO_HALF_WIDTH;
    // Dozer::gap: the hero counts while its centre is ahead of the axle. A
    // hero who waits until the wheel front reaches its left edge is run over
    // (defeated): from then on the machine sees no hero and cruises.
    let gap = |front: f32| {
        (front < hero_left && hero_x > front - axle_behind).then_some(hero_left - front)
    };
    let tail = recovery_ticks.max(frame.land_delay_ticks) as usize;
    let mut fronts = Vec::new();
    let mut front = frame.emerge_front_x;
    let mut speed = profile.speed_of(profile.band(gap(front)));
    let mut extra = None;
    for _ in 0..4096 + tail {
        fronts.push(front);
        if extra.is_none() && front >= hero_left {
            extra = Some(tail);
        }
        match extra.as_mut() {
            Some(0) => break,
            Some(n) => *n -= 1,
            None => {}
        }
        let target = profile.speed_of(profile.band(gap(front)));
        speed = profile.approach(speed, target);
        front += speed * TICK_SECONDS;
    }
    fronts
}

/// DR-B: per release tick R of the domain (every chase tick whose wheel front
/// is short of the hero's left edge), whether the landing on R +
/// `land_delay_ticks` meets the front impact collider under the env step's
/// swept test. This is the set DR-B's sweep (`dz_frame_window_sweep`) checks
/// against the simulation tick by tick.
pub fn release_hits(profile: &DozerProfile, frame: &StopFrame, hero_x: f32) -> Vec<bool> {
    let t = DozerTuning::active();
    let wheel_front = t.wheel_front();
    let tips_ahead = t.tips() - wheel_front;
    let tub_behind = wheel_front - t.dzq03_impact_collider.x0 as f32;
    let hero_left = hero_x - HERO_HALF_WIDTH;
    let fronts = chase_fronts(profile, frame, hero_x, 0);
    let land = frame.land_delay_ticks.max(1) as usize;
    let (fx0, fx1) = (frame.footprint_x0.round(), frame.footprint_x1.round());
    fronts
        .iter()
        .take_while(|&&f| f < hero_left)
        .enumerate()
        .map(|(r, _)| {
            // The env step's layout rounding of the victim rect, swept from
            // the tick before the landing (the machine only moves right).
            let (prev, cur) = (fronts[r + land - 1], fronts[r + land]);
            (cur + tips_ahead).round() > fx0 && (prev - tub_behind).round() < fx1
        })
        .collect()
}

// --------------------------------------------------------------- the rules --

/// The fixed room rules of the Dozer note's section 9 (design rules, layout
/// px, not tuning knobs), read by the encounter validator.
pub mod rules {
    /// Rule 1: `stop_x` is at least the gate x + 256 + 96.
    pub const MIN_RUN_PX: i32 = 256 + 96;
    /// Rule 2: a remnant this tall or lower may lie on the lane.
    pub const REMNANT_MAX_PX: i32 = 16;
    /// Rule 2: the frame hang; a taller solid must have its bottom at or above it.
    pub const FRAME_HANG_PX: i32 = 168;
    /// Rule 2: no solid at all from `stop_x` - 256 to the tunnel mouth.
    pub const RUN_OUT_CLEAR_PX: i32 = 256;
    /// Rule 2: the mouth is at least `stop_x` + 32.
    pub const MOUTH_MARGIN_PX: i32 = 32;
    /// Rule 4: the lane ceiling is at least this high.
    pub const MIN_CEILING_PX: i32 = 200;
    /// Rule 4: the escape passage's clear height (DOZ-01 9.1).
    pub const PASSAGE_CLEAR_PX: (i32, i32) = (64, 116);
    /// Rule 3: a frame hangs at least this far above the push box's top.
    pub const HANG_OVER_PUSH_BOX_PX: i32 = 8;
    /// Rule 5: the brace hero spot is the hero's body this far right of the
    /// brace target (the doc's body 820..844 for a target ending at 816).
    pub const BRACE_STAND_OFF_PX: i32 = 4;
}

// ---------------------------------------------------------------- the site --

/// Which speed profile a site runs (ACT1-B): room 3's `revenge` or the chase's `opening`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SiteProfile {
    #[default]
    Revenge,
    Opening,
}

impl SiteProfile {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Revenge => "revenge",
            Self::Opening => "opening",
        }
    }
}

/// Whose x wakes the machine (ACT1-B, ACT1-AD6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TriggerBy {
    /// The lead (maximum x) living hero: room 3's rule.
    #[default]
    Lead,
    /// The rearmost (minimum x) living hero: nobody is left behind the emergence.
    Rearmost,
}

/// An authored safe pocket of an opening room (layout px, y down).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PocketSite {
    pub id: String,
    pub rect: IRect,
    /// The id of the `solid` static the pocket stands on.
    pub ledge: String,
}

/// A timber set the wheel front smashes (ACT1-B): fires `TimberSmashed` once per attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimberSite {
    pub id: String,
    pub x: i32,
}

/// The room's Dozer data (the sidecar's `dozer` block, DZ-Q11, DZ-Q13, DZ-Q14,
/// DZ-Q22; ACT1-B: the opening's profile, pockets, timbers and wedge), in room px.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DozerSite {
    /// DZ-Q11: the hero's x that wakes the machine ([`Self::trigger_by`]).
    pub trigger_x: i32,
    /// DZ-Q11: the machine wakes after this many dormant ticks anyway.
    pub idle_trigger_ticks: u32,
    /// DZ-Q13: the wheel front at emergence (the gate's right edge).
    pub emerge_front_x: i32,
    /// DZ-Q14: the wheel front when the brake ends.
    pub stop_x: i32,
    /// DZ-Q22: the tunnel's lintel solid (layout px, y down). `None` in an opening
    /// room, which has no tunnel (ACT1-B).
    pub lintel: Option<IRect>,
    /// The speed profile (ACT1-B): `Opening` selects the opening tuning.
    pub profile: SiteProfile,
    pub trigger_by: TriggerBy,
    /// The authored safe pockets (opening only; empty in room 3).
    pub pockets: Vec<PocketSite>,
    /// The smashable timber sets, in increasing x (opening only).
    pub timbers: Vec<TimberSite>,
    /// The machine wedges at `stop_x`: after the strain it reports `Wedged`, and its push
    /// box and nav rows go off (opening only).
    pub wedge: bool,
    /// The room's walking line in layout px (y down), set by the loader (the pocket's
    /// ledge height over the line, in the world frame, is `floor_px - ledge_top`).
    pub floor_px: i32,
    /// The `dozer_contact` lock's solid x (layout px): the barricade the front breaks.
    pub barricade_x: Option<i32>,
}

impl DozerSite {
    /// A room 3 (`revenge`) site: no pocket, no timber, no wedge, no barricade.
    pub fn revenge(
        trigger_x: i32,
        idle_trigger_ticks: u32,
        emerge_front_x: i32,
        stop_x: i32,
        lintel: IRect,
    ) -> Self {
        Self {
            trigger_x,
            idle_trigger_ticks,
            emerge_front_x,
            stop_x,
            lintel: Some(lintel),
            profile: SiteProfile::Revenge,
            trigger_by: TriggerBy::Lead,
            pockets: Vec::new(),
            timbers: Vec::new(),
            wedge: false,
            floor_px: 0,
            barricade_x: None,
        }
    }

    /// The tuning this site's profile runs.
    pub fn tuning(&self) -> &'static DozerTuning {
        match self.profile {
            SiteProfile::Revenge => DozerTuning::active(),
            SiteProfile::Opening => DozerTuning::opening(),
        }
    }
}

// --------------------------------------------------------------- the events --

/// Where a rescue put a bot (section 7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RescueSpot {
    Lane,
    Tunnel,
    /// ACT1-B (appended): the centre of the safe pocket, feet on its ledge.
    Pocket,
    /// ACT1-B (appended): on the lane right behind the hull.
    Behind,
}

impl RescueSpot {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Lane => "lane",
            Self::Tunnel => "tunnel",
            Self::Pocket => "pocket",
            Self::Behind => "behind",
        }
    }
}

/// Why a rescue fired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RescueCause {
    /// The body came within the band of the wheel front.
    Near,
    /// The centre is behind the axle.
    BehindAxle,
    /// Off-screen, inside the machine's reach.
    OffScreen,
    /// ACT1-B (appended): the charge's tell began with the bot outside the pocket.
    Charge,
}

impl RescueCause {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Near => "near",
            Self::BehindAxle => "behind_axle",
            Self::OffScreen => "off_screen",
            Self::Charge => "charge",
        }
    }
}

/// The sub-events of `CombatEvent::Dozer`. No floats (positions are rounded
/// px), so the event stays `Copy + Eq`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DozerEvent {
    /// dormant to approaching: the rear gate shakes.
    Rumble { by_trigger: bool },
    /// approaching to committed chase: the wheel front at the gate.
    Emerged { front_x: i32, band: Band },
    /// The chase's target band changed.
    BandChanged { band: Band, front_x: i32 },
    /// The brake began (escaped); the wheel front reaches `stop_x` at its end.
    BrakeStarted { front_x: i32 },
    /// The brake ended: strain, then idle; the wheel is off.
    BrakeEnded { front_x: i32 },
    /// A stop frame landed on the front collider. `by` is the player credited
    /// with the release (DOZR-04: the thrower of a stick, the caster of the
    /// missiles, the hero whose jabs broke the brace).
    Stunned { front_x: i32, by: Option<u32> },
    /// The stun ended.
    Disabled,
    /// A solo bot was put ahead of the machine (`SlotOut.place`).
    AiRescue {
        hero: HeroId,
        x: i32,
        foot_y: i32,
        spot: RescueSpot,
        cause: RescueCause,
    },
    /// A hero hit or a blast met the hull: no state change.
    Clang { player: u32, blast: bool },
    /// The wheel hit a hero (one blow).
    WheelHit { hero: HeroId },
    /// A push-out would have put a hero into a solid: the machine held still.
    PinnedHold { hero: HeroId },
    /// DR-B (DZ-AD7): a stop frame landed on the hull behind the front
    /// collider: a miss, drawn as the frame striking the rear deck and tipping
    /// off behind the exhaust. No state change.
    Glance { front_x: i32 },
    /// ACT1-B (code 13): the leading living hero first reached `arm_x`. Presentation only.
    ChargeArmed,
    /// ACT1-B (code 14): the commit rule held: the tell began (the speed holds).
    ChargeTell,
    /// ACT1-B (code 15): the tell ended: the speed is set to the charge speed at once.
    ChargeCommitted,
    /// ACT1-B (code 16): the charge ended after `by_ticks` charge ticks (at `stop_x` after a
    /// `BarrierImpact`, or at `max_ticks` on the defensive overshoot path).
    ChargeEnded { by_ticks: u16 },
    /// ACT1-B (code 17): the front reached timber set `id` (its index in the site).
    TimberSmashed { id: u8 },
    /// ACT1-B (code 18): `hero` entered a safe pocket for the first time this attempt.
    PocketEntered { hero: HeroId },
    /// ACT1-B (code 19): the front first reached the barricade: the `dozer_contact` lock opens.
    BarrierImpact,
    /// ACT1-B (code 20): the strain at `stop_x` ended: the machine is wedged.
    Wedged,
}

impl DozerEvent {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Rumble { .. } => "rumble",
            Self::Emerged { .. } => "emerged",
            Self::BandChanged { .. } => "band",
            Self::BrakeStarted { .. } => "brake",
            Self::BrakeEnded { .. } => "escaped",
            Self::Stunned { .. } => "impact_stunned",
            Self::Disabled => "disabled",
            Self::AiRescue { .. } => "ai_rescue",
            Self::Clang { .. } => "clang",
            Self::WheelHit { .. } => "wheel_hit",
            Self::PinnedHold { .. } => "pinned_hold",
            Self::Glance { .. } => "glance",
            Self::ChargeArmed => "charge_armed",
            Self::ChargeTell => "charge_tell",
            Self::ChargeCommitted => "charge_committed",
            Self::ChargeEnded { .. } => "charge_ended",
            Self::TimberSmashed { .. } => "timber_smashed",
            Self::PocketEntered { .. } => "pocket_entered",
            Self::BarrierImpact => "barrier_impact",
            Self::Wedged => "wedged",
        }
    }

    const fn code(&self) -> u64 {
        match self {
            Self::Rumble { .. } => 1,
            Self::Emerged { .. } => 2,
            Self::BandChanged { .. } => 3,
            Self::BrakeStarted { .. } => 4,
            Self::BrakeEnded { .. } => 5,
            Self::Stunned { .. } => 6,
            Self::Disabled => 7,
            Self::AiRescue { .. } => 8,
            Self::Clang { .. } => 9,
            Self::WheelHit { .. } => 10,
            Self::PinnedHold { .. } => 11,
            Self::Glance { .. } => 12,
            Self::ChargeArmed => 13,
            Self::ChargeTell => 14,
            Self::ChargeCommitted => 15,
            Self::ChargeEnded { .. } => 16,
            Self::TimberSmashed { .. } => 17,
            Self::PocketEntered { .. } => 18,
            Self::BarrierImpact => 19,
            Self::Wedged => 20,
        }
    }

    /// Fold the event into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.code());
        match *self {
            Self::Rumble { by_trigger } => mix(u64::from(by_trigger)),
            Self::Emerged { front_x, band } | Self::BandChanged { band, front_x } => {
                mix(front_x as u32 as u64);
                mix(band.index() as u64);
            }
            Self::BrakeStarted { front_x }
            | Self::BrakeEnded { front_x }
            | Self::Glance { front_x } => mix(front_x as u32 as u64),
            Self::Stunned { front_x, by } => {
                mix(front_x as u32 as u64);
                mix(by.map_or(0, |p| u64::from(p) + 1));
            }
            Self::Disabled => {}
            Self::AiRescue {
                hero,
                x,
                foot_y,
                spot,
                cause,
            } => {
                mix(hero.index() as u64);
                mix(x as u32 as u64);
                mix(foot_y as u32 as u64);
                mix(spot as u64);
                mix(cause as u64);
            }
            Self::Clang { player, blast } => {
                mix(u64::from(player));
                mix(u64::from(blast));
            }
            Self::WheelHit { hero } | Self::PinnedHold { hero } | Self::PocketEntered { hero } => {
                mix(hero.index() as u64);
            }
            Self::ChargeArmed
            | Self::ChargeTell
            | Self::ChargeCommitted
            | Self::BarrierImpact
            | Self::Wedged => {}
            Self::ChargeEnded { by_ticks } => mix(u64::from(by_ticks)),
            Self::TimberSmashed { id } => mix(u64::from(id)),
        }
    }

    /// The `key=value` fields of the log line (after `event=<name>`).
    pub fn fields(&self) -> String {
        match *self {
            Self::Rumble { by_trigger } => format!("by_trigger={by_trigger}"),
            Self::Emerged { front_x, band } | Self::BandChanged { band, front_x } => {
                format!("front_x={front_x} band={}", band.name())
            }
            Self::BrakeStarted { front_x }
            | Self::BrakeEnded { front_x }
            | Self::Glance { front_x } => format!("front_x={front_x}"),
            Self::Stunned { front_x, by } => match by {
                Some(p) => format!("front_x={front_x} by={p}"),
                None => format!("front_x={front_x} by=none"),
            },
            Self::Disabled => String::new(),
            Self::AiRescue {
                hero,
                x,
                foot_y,
                spot,
                cause,
            } => format!(
                "hero={hero:?} x={x} foot_y={foot_y} spot={} cause={}",
                spot.name(),
                cause.name()
            ),
            Self::Clang { player, blast } => format!("player={player} blast={blast}"),
            Self::WheelHit { hero } | Self::PinnedHold { hero } | Self::PocketEntered { hero } => {
                format!("hero={hero:?}")
            }
            Self::ChargeArmed
            | Self::ChargeTell
            | Self::ChargeCommitted
            | Self::BarrierImpact
            | Self::Wedged => String::new(),
            Self::ChargeEnded { by_ticks } => format!("by_ticks={by_ticks}"),
            Self::TimberSmashed { id } => format!("timber={id}"),
        }
    }
}

// ---------------------------------------------------------- the machine --

/// The escaped sub-phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapePhase {
    /// Brake step `k` of `dzq14_brake_ticks` (0 on the tick the brake began).
    /// The `k = 0` tick is the chase's last move, clamped to `stop_x` minus
    /// the slide; the 24 slide steps `k = 1..=24` follow. With
    /// `dzq14_lethal_in_brake` the wheel is lethal on all 25 ticks
    /// (DZ-IMPL-AD8).
    Brake {
        k: u16,
    },
    Strain {
        ticks_left: u16,
    },
    Idle,
}

/// The machine's state (section 4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DozerState {
    /// Parked off-room; `ticks` counts the dormant ticks.
    Dormant {
        ticks: u32,
    },
    /// The rumble tell.
    Approaching {
        ticks_left: u16,
    },
    CommittedChase,
    Escaped(EscapePhase),
    ImpactStunned {
        ticks_left: u16,
    },
    Disabled,
}

impl DozerState {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Dormant { .. } => "dormant",
            Self::Approaching { .. } => "approaching",
            Self::CommittedChase => "committed_chase",
            Self::Escaped(_) => "escaped",
            Self::ImpactStunned { .. } => "impact_stunned",
            Self::Disabled => "disabled",
        }
    }

    /// The machine is in the room (its hull blocks and is hit).
    pub const fn emerged(self) -> bool {
        !matches!(self, Self::Dormant { .. } | Self::Approaching { .. })
    }

    fn mix_into(self, mix: &mut impl FnMut(u64)) {
        match self {
            Self::Dormant { ticks } => {
                mix(1);
                mix(u64::from(ticks));
            }
            Self::Approaching { ticks_left } => {
                mix(2);
                mix(u64::from(ticks_left));
            }
            Self::CommittedChase => mix(3),
            Self::Escaped(phase) => {
                mix(4);
                match phase {
                    EscapePhase::Brake { k } => {
                        mix(1);
                        mix(u64::from(k));
                    }
                    EscapePhase::Strain { ticks_left } => {
                        mix(2);
                        mix(u64::from(ticks_left));
                    }
                    EscapePhase::Idle => mix(3),
                }
            }
            Self::ImpactStunned { ticks_left } => {
                mix(5);
                mix(u64::from(ticks_left));
            }
            Self::Disabled => mix(6),
        }
    }
}

/// What the scene tells the machine each tick.
#[derive(Debug, Clone, Copy)]
pub struct DozerCtx<'a> {
    pub tick: u64,
    pub heroes: [HeroView; 2],
    /// Every solid of the scene (the pinned rule and the rescue's floor).
    pub solids: &'a [Block],
    /// Solo: a bot may be rescued.
    pub solo: bool,
    /// The sim camera's horizontal view.
    pub view_x: (f32, f32),
}

/// What one step asks the scene to do.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DozerOut {
    pub events: Vec<DozerEvent>,
    pub blows: Vec<HeroBlow>,
    pub shifts: Vec<(HeroId, f32)>,
    /// `(bot, x, foot_y)`: the rescue's placement (`SlotOut.place`).
    pub place: Vec<(HeroId, f32, f32)>,
}

/// Counters a fixture reads (also hashed: they are simulation state).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DozerStats {
    /// Chase ticks per band (`Band::index`).
    pub band_ticks: [u32; 3],
    /// Rescues per hero (`HeroId::index`).
    pub rescues: [u32; 2],
    pub lane_rescues: u32,
    pub tunnel_rescues: u32,
    pub pinned_holds: u32,
    pub wheel_hits: u32,
    /// ACT1-B: rescues to the pocket and to the spot behind the hull (hashed only in a
    /// profile with a charge block).
    pub pocket_rescues: u32,
    pub behind_rescues: u32,
}

/// ACT1-B: the charge sub-machine inside `CommittedChase` (only a profile with a
/// `dzq10_charge` block leaves `Cruise`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChargeStage {
    #[default]
    Cruise,
    /// The speed holds for `tell_ticks`.
    Tell {
        t: u16,
    },
    /// The charge speed, set at once, for at most `max_ticks` (past a `BarrierImpact` until
    /// `stop_x`).
    Charge {
        t: u16,
    },
    /// The defensive path: the brake ticks and the slide after a charge that never met the
    /// barricade.
    Overshoot {
        t: u16,
    },
    Strain {
        t: u16,
    },
}

impl ChargeStage {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cruise => "cruise",
            Self::Tell { .. } => "tell",
            Self::Charge { .. } => "charge",
            Self::Overshoot { .. } => "overshoot",
            Self::Strain { .. } => "strain",
        }
    }

    fn mix_into(self, mix: &mut impl FnMut(u64)) {
        match self {
            Self::Cruise => mix(1),
            Self::Tell { t } => {
                mix(2);
                mix(u64::from(t));
            }
            Self::Charge { t } => {
                mix(3);
                mix(u64::from(t));
            }
            Self::Overshoot { t } => {
                mix(4);
                mix(u64::from(t));
            }
            Self::Strain { t } => {
                mix(5);
                mix(u64::from(t));
            }
        }
    }
}

/// The machine (section 4).
#[derive(Debug, Clone)]
pub struct Dozer {
    pub tuning: &'static DozerTuning,
    pub profile: DozerProfile,
    pub site: DozerSite,
    /// The exit's left edge (the tunnel spot's clamp).
    pub exit_left: f32,
    /// The walking line (world y).
    pub floor_y: f32,
    /// The rig origin (world x).
    pub o: f32,
    pub speed: f32,
    pub state: DozerState,
    /// The chase's current target band (`None` before the chase).
    pub band: Option<Band>,
    /// The tick of each hero's last rescue.
    last_rescue: [Option<u64>; 2],
    pub stats: DozerStats,
    /// The Dozer-owned persistent `Wheel` volume (world px, y up: the volume is
    /// never exported, so its rectangles stay in the machine's convention).
    wheel: HazardVolume,
    /// ACT1-B: the charge sub-state (always `Cruise` without a charge block).
    pub charge: ChargeStage,
    /// ACT1-B: charges started this attempt.
    charges_used: u8,
    /// ACT1-B: `ChargeArmed` fired.
    armed: bool,
    /// ACT1-B (ACT1-AD19): the front reached the barricade. The `dozer_contact` lock is open
    /// exactly while this is set; `slot_respawn` and a restart rebuild the machine, so
    /// every retry path closes it.
    barrier_hit: bool,
    /// ACT1-B: `PocketEntered` fired for the hero.
    pocket_seen: [bool; 2],
    /// ACT1-B: bit `i` set once timber `i` was smashed.
    timbers_smashed: u32,
    /// ACT1-B: `Wedged` fired (the push box and the nav rows are off).
    wedged: bool,
    /// ACT1-B: what the commit rule saw when it last held (a diagnostic for the fixtures'
    /// tables: **not hashed**, never read by the simulation).
    pub commit: Option<CommitRecord>,
}

/// ACT1-B: the state at the tick the commit rule held (the tell's first tick).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommitRecord {
    pub front: f32,
    pub speed: f32,
    /// Per hero (`HeroId::index`): `wheel_ticks - walk_ticks`, the ticks to spare (`None`:
    /// the hero was not present).
    pub margin: [Option<i32>; 2],
    /// The gap of rule 2 (the rearmost body's left edge minus the front).
    pub gap: f32,
}

impl Dozer {
    /// A dormant machine at `site`, parked with its wheel front at the gate.
    pub fn new(site: DozerSite, exit_left: f32, floor_y: f32) -> Self {
        let tuning = DozerTuning::active();
        Self::with_tuning(tuning, site, exit_left, floor_y)
    }

    pub fn with_tuning(
        tuning: &'static DozerTuning,
        site: DozerSite,
        exit_left: f32,
        floor_y: f32,
    ) -> Self {
        let o = site.emerge_front_x as f32 - tuning.wheel_front();
        let wheel = HazardVolume::new(HazardSpec {
            id: FEED_ID_DOZER,
            class: HazardClass::Wheel,
            rect: IRect::new(0, 0, 1, 1),
            telegraph_rect: IRect::new(0, 0, 1, 1),
            window: HazardWindow::Persistent,
            damage: tuning.dzq04_wheel_damage,
            push_x: i16::try_from(tuning.dzq04_knock_x_px_s).unwrap_or(i16::MAX),
            hero_damage: HeroDamagePolicy::AllHeroes,
            hurts_enemies: false,
        })
        .expect("the wheel volume spec is valid");
        Self {
            tuning,
            profile: tuning.revenge(),
            site,
            exit_left,
            floor_y,
            o,
            speed: 0.0,
            state: DozerState::Dormant { ticks: 0 },
            band: None,
            last_rescue: [None; 2],
            stats: DozerStats::default(),
            wheel,
            charge: ChargeStage::Cruise,
            charges_used: 0,
            armed: false,
            barrier_hit: false,
            pocket_seen: [false; 2],
            timbers_smashed: 0,
            wedged: false,
            commit: None,
        }
    }

    /// ACT1-B: the profile has a charge block (the opening).
    pub fn has_charge(&self) -> bool {
        self.tuning.charge.is_some()
    }

    /// ACT1-B (ACT1-AD19): the front reached the barricade this attempt.
    pub fn barrier_hit(&self) -> bool {
        self.barrier_hit
    }

    /// ACT1-B: the machine is wedged at `stop_x` (`wedge` sites).
    pub fn wedged(&self) -> bool {
        self.wedged
    }

    /// ACT1-B: charges started this attempt.
    pub fn charges_used(&self) -> u8 {
        self.charges_used
    }

    /// ACT1-B: whether timber set `id` was smashed this attempt.
    pub fn timber_smashed(&self, id: usize) -> bool {
        id < 32 && self.timbers_smashed & (1 << id) != 0
    }

    /// ACT1-B: `ChargeArmed` fired this attempt.
    pub fn armed_once(&self) -> bool {
        self.armed
    }

    /// ACT1-B: whether `hero` entered a pocket this attempt.
    pub fn pocket_seen(&self, hero: HeroId) -> bool {
        self.pocket_seen[hero.index()]
    }

    /// ACT1-B: the index of the pocket `h` is in: the body's x span lies inside the pocket
    /// rect's x span and the feet are at or above the ledge top, airborne included. The
    /// rect is layout px (y down) and `HeroView` is world px (y up): the ledge top is
    /// `floor_px - rect.bottom()` above the walking line (the conversion
    /// `layout y = layout_h - world y` of `nav_rows_into`, taken from the line).
    pub fn pocket_of(&self, h: &HeroView) -> Option<usize> {
        self.site.pockets.iter().position(|p| {
            let ledge = (self.site.floor_px - p.rect.bottom()) as f32;
            h.x - HERO_HALF_WIDTH >= p.rect.x as f32
                && h.x + HERO_HALF_WIDTH <= p.rect.right() as f32
                && h.foot_y >= self.floor_y + ledge - 0.5
        })
    }

    // ------------------------------------------------------------ geometry --

    pub fn wheel_front(&self) -> f32 {
        self.o + self.tuning.wheel_front()
    }

    pub fn tips(&self) -> f32 {
        self.o + self.tuning.tips()
    }

    pub fn axle(&self) -> (f32, f32) {
        let a = self.tuning.dzq01_wheel_axle;
        (self.o + a.x as f32, self.floor_y + a.y as f32)
    }

    pub fn wheel_r(&self) -> f32 {
        f32::from(self.tuning.dzq01_wheel_r_px)
    }

    fn span_rect(&self, o: f32, s: SpanPx) -> Rect {
        Rect {
            left: o + s.x0 as f32,
            bottom: self.floor_y + s.y0 as f32,
            width: (s.x1 - s.x0) as f32,
            height: (s.y1 - s.y0) as f32,
        }
    }

    /// The push box at origin `o` (world).
    pub fn push_box_at(&self, o: f32) -> Rect {
        self.span_rect(o, self.tuning.dzq02_push_box)
    }

    pub fn push_box(&self) -> Rect {
        self.push_box_at(self.o)
    }

    /// The front impact collider (world).
    pub fn impact_collider(&self) -> Rect {
        self.span_rect(self.o, self.tuning.dzq03_impact_collider)
    }

    /// The wheel's bounding box (world).
    pub fn wheel_box(&self) -> Rect {
        let (cx, cy) = self.axle();
        let r = self.wheel_r();
        Rect {
            left: cx - r,
            bottom: cy - r,
            width: 2.0 * r,
            height: 2.0 * r,
        }
    }

    /// The wheel is lethal now (`committed_chase`, and the brake if the data
    /// says so).
    pub fn wheel_lethal(&self) -> bool {
        match self.state {
            DozerState::CommittedChase => true,
            DozerState::Escaped(EscapePhase::Brake { .. }) => self.tuning.dzq14_lethal_in_brake,
            _ => false,
        }
    }

    /// The circle overlaps `hurt` (strict).
    pub fn wheel_touches(&self, hurt: Rect) -> bool {
        let (cx, cy) = self.axle();
        let r = self.wheel_r();
        hurt.squared_distance_to(cx, cy) < r * r
    }

    // ------------------------------------------------------------ the band --

    /// The gap of section 4.2: the left edge of the rearmost living hero whose
    /// centre is ahead of the axle, minus the wheel front. `None` when no
    /// living hero is ahead.
    pub fn gap(&self, heroes: &[HeroView; 2]) -> Option<f32> {
        let (axle_x, _) = self.axle();
        heroes
            .iter()
            .filter(|h| h.present && h.x > axle_x)
            .map(|h| h.x - HERO_HALF_WIDTH)
            .min_by(f32::total_cmp)
            .map(|left| left - self.wheel_front())
    }

    // ----------------------------------------------------------- nav rows --

    /// The feed rows of this tick (section 7.1), layout px (y down), rounded
    /// outward to the data's grid.
    pub fn nav_rows(&self, layout_h: i32) -> Vec<NavRect> {
        let mut rows = Vec::new();
        self.nav_rows_into(layout_h, &mut rows);
        rows
    }

    /// [`Self::nav_rows`] appended to `out` (the hazard feed's own list: no per-tick allocation).
    pub fn nav_rows_into(&self, layout_h: i32, out: &mut Vec<NavRect>) {
        let t = self.tuning;
        let grid = i32::from(t.dzq16_nav_grid_px);
        let height = t.dzq02_push_box.y1 as f32;
        let row = |x0: f32, x1: f32, index: u16, phase: NavPhase| {
            let gx0 = (x0.floor() as i32).div_euclid(grid) * grid;
            let gx1 = -((-(x1.ceil() as i32)).div_euclid(grid)) * grid;
            let top = layout_h as f32 - (self.floor_y + height);
            let bottom = layout_h as f32 - self.floor_y;
            let gy0 = (top.floor() as i32).div_euclid(grid) * grid;
            let gy1 = -((-(bottom.ceil() as i32)).div_euclid(grid)) * grid;
            NavRect {
                hazard: FEED_ID_DOZER + index,
                class: HazardClass::Wheel,
                rect: IRect::new(gx0, gy0, gx1 - gx0, gy1 - gy0),
                phase,
                ticks_left: None,
                hurts_heroes: true,
                hurts_enemies: false,
            }
        };
        let machine = row(
            self.o + t.dzq02_push_box.x0 as f32,
            self.tips(),
            0,
            NavPhase::Active,
        );
        // ACT1-B: a wedged machine has no nav rows; the rows of an opening room are cut
        // around the pocket (x only), so the planner may stand in it.
        if self.wedged {
            return;
        }
        let first = out.len();
        match self.state {
            DozerState::Dormant { .. } | DozerState::Approaching { .. } => {}
            DozerState::CommittedChase => {
                out.push(machine);
                out.push(row(
                    self.tips(),
                    self.tips() + f32::from(t.dzq16_nav_lead_px),
                    1,
                    NavPhase::Telegraph,
                ));
            }
            DozerState::Escaped(_) | DozerState::ImpactStunned { .. } | DozerState::Disabled => {
                out.push(machine);
            }
        }
        if self.has_charge() && !self.site.pockets.is_empty() {
            let cuts: Vec<(i32, i32)> = self
                .site
                .pockets
                .iter()
                .map(|p| (p.rect.x, p.rect.right()))
                .collect();
            let rows: Vec<NavRect> = out.drain(first..).collect();
            for r in rows {
                let (mut left, right) = (r.rect.x, r.rect.right());
                let mut pieces: Vec<(i32, i32)> = Vec::new();
                for &(c0, c1) in &cuts {
                    if c1 <= left || c0 >= right {
                        continue;
                    }
                    if c0 > left {
                        pieces.push((left, c0));
                    }
                    left = left.max(c1);
                    if left >= right {
                        break;
                    }
                }
                if left < right {
                    pieces.push((left, right));
                }
                for (k, (x0, x1)) in pieces.into_iter().enumerate() {
                    out.push(NavRect {
                        hazard: r.hazard + 2 * k as u16,
                        rect: IRect::new(x0, r.rect.y, x1 - x0, r.rect.h),
                        ..r
                    });
                }
            }
        }
    }

    // ------------------------------------------------------------ the step --

    /// One tick (see the module's tick order).
    pub fn step(&mut self, ctx: &DozerCtx<'_>, out: &mut DozerOut) {
        let before = self.o;
        let moved = self.advance(ctx, out);
        let heroes = self.push_out(ctx, before, moved, out);
        self.track_pockets(&heroes, out);
        let rescued = self.rescue(ctx, &heroes, out);
        self.wheel_test(ctx, &heroes, rescued, out);
    }

    /// ACT1-B: `PocketEntered` the first time each hero is in a pocket this attempt.
    fn track_pockets(&mut self, heroes: &[HeroView; 2], out: &mut DozerOut) {
        if self.site.pockets.is_empty() {
            return;
        }
        for h in heroes.iter().filter(|h| h.present) {
            if self.pocket_of(h).is_some() && !self.pocket_seen[h.id.index()] {
                self.pocket_seen[h.id.index()] = true;
                out.events.push(DozerEvent::PocketEntered { hero: h.id });
            }
        }
    }

    /// ACT1-B: the opening's `CommittedChase` tick (a profile with a charge block): the
    /// cruise with the commit rule, the tell, the charge (at the charge speed at once, an
    /// impact tested before `max_ticks`, on to `stop_x` after a `BarrierImpact`, ACT1-AD18),
    /// the defensive overshoot and strain, the timbers and the barricade. Returns whether
    /// the origin moved.
    fn advance_opening(&mut self, ctx: &DozerCtx<'_>, out: &mut DozerOut) -> bool {
        let t = self.tuning;
        let Some(ch) = t.charge else {
            return false;
        };
        if !self.armed {
            let lead = ctx
                .heroes
                .iter()
                .filter(|h| h.present)
                .map(|h| h.x)
                .max_by(f32::total_cmp);
            if lead.is_some_and(|x| x >= ch.arm_x as f32) {
                self.armed = true;
                out.events.push(DozerEvent::ChargeArmed);
            }
        }
        match self.charge {
            ChargeStage::Cruise => {
                let band = self.profile.band(self.gap(&ctx.heroes));
                if self.band != Some(band) {
                    self.band = Some(band);
                    out.events.push(DozerEvent::BandChanged {
                        band,
                        front_x: self.wheel_front().round() as i32,
                    });
                }
                self.stats.band_ticks[band.index()] += 1;
                self.speed = self
                    .profile
                    .approach(self.speed, self.profile.speed_of(band));
                self.o += self.speed * TICK_SECONDS;
            }
            ChargeStage::Tell { t: n } => {
                // The speed holds (no band update, no acceleration).
                self.o += self.speed * TICK_SECONDS;
                let n = n + 1;
                if n >= ch.tell_ticks {
                    self.speed = f32::from(ch.speed_px_s);
                    self.charge = ChargeStage::Charge { t: 0 };
                    out.events.push(DozerEvent::ChargeCommitted);
                } else {
                    self.charge = ChargeStage::Tell { t: n };
                }
            }
            ChargeStage::Charge { t: n } => {
                self.o += self.speed * TICK_SECONDS;
                self.charge = ChargeStage::Charge { t: n + 1 };
            }
            ChargeStage::Overshoot { t: n } => {
                self.o += f32::from(ch.brake_slide_px) / f32::from(ch.brake_ticks);
                let n = n + 1;
                self.charge = if n >= ch.brake_ticks {
                    ChargeStage::Strain { t: 0 }
                } else {
                    ChargeStage::Overshoot { t: n }
                };
            }
            ChargeStage::Strain { t: n } => {
                let n = n + 1;
                self.charge = if n >= ch.strain_ticks {
                    ChargeStage::Cruise
                } else {
                    ChargeStage::Strain { t: n }
                };
            }
        }
        self.touch_set_pieces(out);
        // The impact is tested before `max_ticks`, so an impact on the last charge tick counts.
        if let ChargeStage::Charge { t: n } = self.charge
            && !self.barrier_hit
            && n >= ch.max_ticks
        {
            out.events.push(DozerEvent::ChargeEnded { by_ticks: n });
            self.speed = 0.0;
            self.charge = ChargeStage::Overshoot { t: 0 };
        }
        let brake_at = (self.site.stop_x - i32::from(t.dzq14_brake_slide_px)) as f32;
        if self.wheel_front() >= brake_at {
            // Room 3's escape, in any sub-state; a charge that ran on past the barricade
            // ends here.
            if let ChargeStage::Charge { t: n } = self.charge {
                out.events.push(DozerEvent::ChargeEnded { by_ticks: n });
            }
            self.charge = ChargeStage::Cruise;
            self.o = brake_at - t.wheel_front();
            self.state = DozerState::Escaped(EscapePhase::Brake { k: 0 });
            out.events.push(DozerEvent::BrakeStarted {
                front_x: self.wheel_front().round() as i32,
            });
        } else if self.charge == ChargeStage::Cruise
            && self.charges_used < ch.charges_per_attempt
            && self.commit_holds(ctx, &ch)
        {
            self.charges_used += 1;
            self.charge = ChargeStage::Tell { t: 0 };
            self.commit = Some(self.commit_record(ctx, &ch));
            out.events.push(DozerEvent::ChargeTell);
        }
        true
    }

    /// ACT1-B: the diagnostic record of a commit (see [`CommitRecord`]).
    fn commit_record(&self, ctx: &DozerCtx<'_>, ch: &ChargeTuning) -> CommitRecord {
        let pocket = self
            .site
            .pockets
            .first()
            .map_or((0.0, 0.0), |p| (p.rect.x as f32, p.rect.right() as f32));
        let front = self.wheel_front();
        let wheel = wheel_ticks(ch, front, self.speed, pocket.0) as i32;
        let margin = ctx.heroes.map(|h| {
            h.present.then(|| {
                let hero = CommitHero {
                    x: h.x,
                    run_px_s: hero_run_px_s(h.id),
                };
                wheel - walk_ticks(hero, pocket) as i32
            })
        });
        let gap = ctx
            .heroes
            .iter()
            .filter(|h| h.present)
            .map(|h| h.x - HERO_HALF_WIDTH - front)
            .fold(f32::INFINITY, f32::min);
        CommitRecord {
            front,
            speed: self.speed,
            margin,
            gap,
        }
    }

    /// ACT1-B: the timbers and the barricade the front has reached (any sub-state).
    fn touch_set_pieces(&mut self, out: &mut DozerOut) {
        let front = self.wheel_front();
        for (i, timber) in self.site.timbers.iter().enumerate().take(32) {
            if self.timbers_smashed & (1 << i) == 0 && front + REACH_EPS_PX >= timber.x as f32 {
                self.timbers_smashed |= 1 << i;
                out.events.push(DozerEvent::TimberSmashed { id: i as u8 });
            }
        }
        if !self.barrier_hit
            && let Some(bx) = self.site.barricade_x
            && front + REACH_EPS_PX >= bx as f32
        {
            self.barrier_hit = true;
            out.events.push(DozerEvent::BarrierImpact);
        }
    }

    /// ACT1-B: the commit rule (integration plan section 5.3) on this tick's state.
    fn commit_holds(&self, ctx: &DozerCtx<'_>, ch: &ChargeTuning) -> bool {
        let Some(pocket) = self.site.pockets.first() else {
            return false;
        };
        let heroes: Vec<CommitHero> = ctx
            .heroes
            .iter()
            .filter(|h| h.present)
            .map(|h| CommitHero {
                x: h.x,
                run_px_s: hero_run_px_s(h.id),
            })
            .collect();
        commit_rule_holds(
            ch,
            (pocket.rect.x as f32, pocket.rect.right() as f32),
            self.site.barricade_x.map(|x| x as f32),
            self.wheel_front(),
            self.speed,
            &heroes,
        )
    }

    /// Step 1: the state's move. Returns whether the origin moved (so the
    /// pinned rule may revert it).
    fn advance(&mut self, ctx: &DozerCtx<'_>, out: &mut DozerOut) -> bool {
        let t = self.tuning;
        match self.state {
            DozerState::Dormant { ticks } => {
                let xs = ctx.heroes.iter().filter(|h| h.present).map(|h| h.x);
                // ACT1-AD6: the opening wakes on the rearmost hero.
                let waker = match self.site.trigger_by {
                    TriggerBy::Lead => xs.max_by(f32::total_cmp),
                    TriggerBy::Rearmost => xs.min_by(f32::total_cmp),
                };
                let by_trigger = waker.is_some_and(|x| x >= self.site.trigger_x as f32);
                let ticks = ticks + 1;
                if by_trigger || ticks >= self.site.idle_trigger_ticks {
                    out.events.push(DozerEvent::Rumble { by_trigger });
                    self.state = DozerState::Approaching {
                        ticks_left: t.dzq12_rumble_ticks,
                    };
                } else {
                    self.state = DozerState::Dormant { ticks };
                }
                false
            }
            DozerState::Approaching { ticks_left } => {
                let ticks_left = ticks_left.saturating_sub(1);
                if ticks_left == 0 {
                    // DZ-Q13: the chase starts at the band's target speed.
                    let band = self.profile.band(self.gap(&ctx.heroes));
                    self.band = Some(band);
                    self.speed = self.profile.speed_of(band);
                    self.state = DozerState::CommittedChase;
                    out.events.push(DozerEvent::Emerged {
                        front_x: self.wheel_front().round() as i32,
                        band,
                    });
                } else {
                    self.state = DozerState::Approaching { ticks_left };
                }
                false
            }
            DozerState::CommittedChase if self.has_charge() => self.advance_opening(ctx, out),
            DozerState::CommittedChase => {
                let band = self.profile.band(self.gap(&ctx.heroes));
                if self.band != Some(band) {
                    self.band = Some(band);
                    out.events.push(DozerEvent::BandChanged {
                        band,
                        front_x: self.wheel_front().round() as i32,
                    });
                }
                self.stats.band_ticks[band.index()] += 1;
                self.speed = self
                    .profile
                    .approach(self.speed, self.profile.speed_of(band));
                self.o += self.speed * TICK_SECONDS;
                let brake_at = (self.site.stop_x - i32::from(t.dzq14_brake_slide_px)) as f32;
                if self.wheel_front() >= brake_at {
                    self.o = brake_at - t.wheel_front();
                    self.state = DozerState::Escaped(EscapePhase::Brake { k: 0 });
                    out.events.push(DozerEvent::BrakeStarted {
                        front_x: self.wheel_front().round() as i32,
                    });
                }
                true
            }
            DozerState::Escaped(EscapePhase::Brake { k }) => {
                if k < t.dzq14_brake_ticks {
                    let k = k + 1;
                    self.o = self.brake_base()
                        + f32::from(t.dzq14_brake_slide_px) * f32::from(k)
                            / f32::from(t.dzq14_brake_ticks);
                    self.state = DozerState::Escaped(EscapePhase::Brake { k });
                    true
                } else {
                    self.speed = 0.0;
                    self.state = DozerState::Escaped(EscapePhase::Strain {
                        ticks_left: t.dzq14_strain_ticks,
                    });
                    out.events.push(DozerEvent::BrakeEnded {
                        front_x: self.wheel_front().round() as i32,
                    });
                    false
                }
            }
            DozerState::Escaped(EscapePhase::Strain { ticks_left }) => {
                self.state = DozerState::Escaped(if ticks_left <= 1 {
                    EscapePhase::Idle
                } else {
                    EscapePhase::Strain {
                        ticks_left: ticks_left - 1,
                    }
                });
                // ACT1-B: a wedge site reports the end of the strain (the machine is
                // stuck for good: no push box, no nav rows from here on).
                if ticks_left <= 1 && self.site.wedge && !self.wedged {
                    self.wedged = true;
                    out.events.push(DozerEvent::Wedged);
                }
                false
            }
            DozerState::Escaped(EscapePhase::Idle) | DozerState::Disabled => false,
            DozerState::ImpactStunned { ticks_left } => {
                if ticks_left <= 1 {
                    self.state = DozerState::Disabled;
                    out.events.push(DozerEvent::Disabled);
                } else {
                    self.state = DozerState::ImpactStunned {
                        ticks_left: ticks_left - 1,
                    };
                }
                false
            }
        }
    }

    /// The origin the brake slides from (the wheel front `slide` px short of
    /// `stop_x`).
    fn brake_base(&self) -> f32 {
        let t = self.tuning;
        (self.site.stop_x - i32::from(t.dzq14_brake_slide_px)) as f32 - t.wheel_front()
    }

    /// Step 2: the push-out at the candidate origin, with the pinned rule.
    /// Returns the hero views after the shifts.
    fn push_out(
        &mut self,
        ctx: &DozerCtx<'_>,
        before: f32,
        moved: bool,
        out: &mut DozerOut,
    ) -> [HeroView; 2] {
        let mut heroes = ctx.heroes;
        // ACT1-B: a wedged machine pushes nothing.
        if !self.state.emerged() || self.wedged {
            return heroes;
        }
        let box_ = self.push_box();
        let mut shifts: Vec<(HeroId, f32)> = Vec::new();
        let mut pinned: Option<HeroId> = None;
        for h in heroes.iter().filter(|h| h.present) {
            // ACT1-B: a hero in a pocket is out of the machine's reach.
            if self.pocket_of(h).is_some() {
                continue;
            }
            let body = body_rect(h.x, h.foot_y);
            if !body.overlaps(box_) {
                continue;
            }
            let mid = box_.left + box_.width * 0.5;
            let x = if h.x < mid {
                box_.left - HERO_HALF_WIDTH
            } else {
                box_.right() + HERO_HALF_WIDTH
            };
            if crate::combat::solid_at(ctx.solids, body_rect(x, h.foot_y)) {
                pinned.get_or_insert(h.id);
                continue;
            }
            shifts.push((h.id, x));
        }
        if let Some(hero) = pinned {
            // The pinned rule: stop instead of crushing. No hero is shifted.
            if moved {
                self.o = before;
                self.undo_brake_step();
                self.stats.pinned_holds += 1;
                out.events.push(DozerEvent::PinnedHold { hero });
            }
            return heroes;
        }
        for (id, x) in shifts {
            let h = &mut heroes[id.index()];
            let dx = x - h.x;
            h.x = x;
            h.hurt.left += dx;
            out.shifts.push((id, x));
        }
        heroes
    }

    /// A pinned hold during the brake keeps the brake step where it was.
    fn undo_brake_step(&mut self) {
        if let DozerState::Escaped(EscapePhase::Brake { k }) = self.state
            && k > 0
        {
            self.state = DozerState::Escaped(EscapePhase::Brake { k: k - 1 });
        }
    }

    /// Step 3: the solo rescue of a bot (section 7.3). Returns the rescued
    /// heroes. ACT1-B (ACT1-AD14), only in a profile with a charge block: a bot in a pocket
    /// or fully behind the hull has no cause; a bot centre behind the axle has one only
    /// while its body overlaps the hull (strict); at the tell's first tick a bot outside
    /// the pocket is put in it (`Charge`); before `BarrierImpact` the spots are the
    /// pocket and the lane behind the hull, never the room-3 tunnel spot.
    fn rescue(
        &mut self,
        ctx: &DozerCtx<'_>,
        heroes: &[HeroView; 2],
        out: &mut DozerOut,
    ) -> [bool; 2] {
        let mut rescued = [false; 2];
        if !ctx.solo || !self.wheel_lethal() {
            return rescued;
        }
        let opening = self.has_charge() && !self.site.pockets.is_empty();
        let t = self.tuning;
        let stop_x = self.site.stop_x as f32;
        let front = self.wheel_front();
        let (axle_x, _) = self.axle();
        let zone_front = self.tips() + f32::from(t.dzq16_nav_lead_px);
        let view_margin = f32::from(t.dzq17_lane_spot_view_margin_px);
        let zone_margin = f32::from(t.dzq17_rescue_zone_margin_px);
        for h in heroes.iter().filter(|h| h.present && h.bot) {
            let left = h.x - HERO_HALF_WIDTH;
            // A body at or past stop_x is out of reach at every tick (section 5).
            if left >= stop_x {
                continue;
            }
            let mut spot_cause: Option<(f32, f32, RescueSpot, RescueCause)> = None;
            if opening {
                if self.pocket_of(h).is_some() {
                    continue;
                }
                if matches!(self.charge, ChargeStage::Tell { t: 0 }) {
                    let (x, foot_y) = self.pocket_spot();
                    spot_cause = Some((x, foot_y, RescueSpot::Pocket, RescueCause::Charge));
                } else if h.x < axle_x {
                    let behind = self.o + t.dzq02_push_box.x0 as f32 - zone_margin;
                    if h.x + HERO_HALF_WIDTH <= behind
                        || !body_rect(h.x, h.foot_y).overlaps(self.push_box())
                    {
                        continue;
                    }
                }
            }
            let in_view = h.x >= ctx.view_x.0 && h.x <= ctx.view_x.1;
            let cause = if h.x < axle_x {
                RescueCause::BehindAxle
            } else if left < front + f32::from(t.dzq17_rescue_band_px) {
                RescueCause::Near
            } else if !in_view && h.x >= self.o + t.dzq02_push_box.x0 as f32 && h.x < zone_front {
                RescueCause::OffScreen
            } else if spot_cause.is_some() {
                RescueCause::Charge
            } else {
                continue;
            };
            let (x, foot_y, spot, cause) = if let Some(chosen) = spot_cause {
                chosen
            } else if opening && !self.barrier_hit {
                let (x, foot_y, spot) = self.opening_spot(ctx, view_margin);
                (x, foot_y, spot, cause)
            } else {
                let leader = &heroes[h.id.other().index()];
                let x_lane =
                    (leader.x + f32::from(t.dzq17_rescue_ahead_px)).max(zone_front + zone_margin);
                let recent = self.last_rescue[h.id.index()].is_some_and(|at| {
                    ctx.tick.saturating_sub(at) < u64::from(t.dzq17_rescue_min_spacing_ticks)
                });
                let lane_ok = x_lane <= stop_x - f32::from(t.dzq17_lane_spot_stop_margin_px)
                    && x_lane >= ctx.view_x.0 + view_margin
                    && x_lane <= ctx.view_x.1 - view_margin
                    && !recent;
                let (x, spot) = if lane_ok {
                    (x_lane, RescueSpot::Lane)
                } else {
                    let lo = stop_x + f32::from(t.dzq17_rescue_past_stop_px);
                    let hi = self.exit_left - f32::from(t.dzq17_rescue_exit_margin_px);
                    (x_lane.max(lo).clamp(lo, hi.max(lo)), RescueSpot::Tunnel)
                };
                (x, standing_foot_y(ctx.solids, x, self.floor_y), spot, cause)
            };
            out.place.push((h.id, x, foot_y));
            out.events.push(DozerEvent::AiRescue {
                hero: h.id,
                x: x.round() as i32,
                foot_y: foot_y.round() as i32,
                spot,
                cause,
            });
            self.last_rescue[h.id.index()] = Some(ctx.tick);
            self.stats.rescues[h.id.index()] += 1;
            match spot {
                RescueSpot::Lane => self.stats.lane_rescues += 1,
                RescueSpot::Tunnel => self.stats.tunnel_rescues += 1,
                RescueSpot::Pocket => self.stats.pocket_rescues += 1,
                RescueSpot::Behind => self.stats.behind_rescues += 1,
            }
            rescued[h.id.index()] = true;
        }
        rescued
    }

    /// ACT1-B: the centre of the first pocket, feet on its ledge (world px).
    fn pocket_spot(&self) -> (f32, f32) {
        let p = &self.site.pockets[0];
        let ledge = (self.site.floor_px - p.rect.bottom()) as f32;
        (
            p.rect.x as f32 + p.rect.w as f32 * 0.5,
            self.floor_y + ledge,
        )
    }

    /// ACT1-B: the opening's rescue spot before `BarrierImpact` (ACT1-AD14): the pocket while
    /// its left edge is ahead of the front by at least the rescue band plus a body width,
    /// otherwise the lane right behind the hull. A spot outside the view margin gives way to
    /// the other; with both outside the pocket is used (it is authored on the route). Both
    /// skip the spacing check, and the tunnel spot is never used here.
    fn opening_spot(&self, ctx: &DozerCtx<'_>, view_margin: f32) -> (f32, f32, RescueSpot) {
        let t = self.tuning;
        let (pocket_x, pocket_foot) = self.pocket_spot();
        let pocket_left = self.site.pockets[0].rect.x as f32;
        let behind_x = self.o + t.dzq02_push_box.x0 as f32
            - f32::from(t.dzq17_rescue_zone_margin_px)
            - HERO_HALF_WIDTH;
        let behind_foot = standing_foot_y(ctx.solids, behind_x, self.floor_y);
        let band = f32::from(t.dzq17_rescue_band_px) + 2.0 * HERO_HALF_WIDTH;
        let pocket = (pocket_x, pocket_foot, RescueSpot::Pocket);
        let behind = (behind_x, behind_foot, RescueSpot::Behind);
        let (first, second) = if pocket_left - self.wheel_front() >= band {
            (pocket, behind)
        } else {
            (behind, pocket)
        };
        let in_margin = |x: f32| x >= ctx.view_x.0 + view_margin && x <= ctx.view_x.1 - view_margin;
        if in_margin(first.0) {
            first
        } else if in_margin(second.0) {
            second
        } else {
            pocket
        }
    }

    /// Step 4: the wheel on every hero in the sim view (section 4.3).
    fn wheel_test(
        &mut self,
        ctx: &DozerCtx<'_>,
        heroes: &[HeroView; 2],
        rescued: [bool; 2],
        out: &mut DozerOut,
    ) {
        if !self.wheel_lethal() {
            return;
        }
        let t = self.tuning;
        let bbox = self.wheel_box();
        self.wheel.rearm();
        self.wheel.set_rect(world_irect(bbox));
        let (axle_x, _) = self.axle();
        for h in heroes.iter() {
            if !h.present || rescued[h.id.index()] || self.pocket_of(h).is_some() {
                continue;
            }
            // DZ-AD16: never an off-screen kill.
            if h.x < ctx.view_x.0 || h.x > ctx.view_x.1 {
                continue;
            }
            if !self.wheel_touches(h.hurt) {
                continue;
            }
            let body = world_irect(h.hurt);
            if self
                .wheel
                .contact(Victim::hero(h.id.index() as u32), body)
                .is_none()
            {
                continue;
            }
            self.stats.wheel_hits += 1;
            out.events.push(DozerEvent::WheelHit { hero: h.id });
            out.blows.push(HeroBlow {
                hero: h.id,
                source: t.dzq04_source_id,
                damage: t.dzq04_wheel_damage,
                from_x: axle_x,
                knock_vx: f32::from(t.dzq04_knock_x_px_s),
                knock_vy: f32::from(t.dzq04_knock_up_px_s),
            });
        }
    }

    /// A stop frame's landing on the front collider (`FallingMass`): stunned
    /// once, only from the chase, credited to `by` (the release's initiator).
    /// `true` when it took effect.
    pub fn apply_falling_mass(&mut self, by: Option<u32>, out: &mut DozerOut) -> bool {
        if self.state != DozerState::CommittedChase {
            return false;
        }
        self.speed = 0.0;
        self.state = DozerState::ImpactStunned {
            ticks_left: self.tuning.dzq15_stunned_ticks,
        };
        out.events.push(DozerEvent::Stunned {
            front_x: self.wheel_front().round() as i32,
            by,
        });
        true
    }

    /// DR-B (DZ-AD7): a stop frame landed with `footprint` (world, y up) and
    /// missed the front collider. While the machine moves (the chase or the
    /// brake) a footprint that meets its hull is a rear-hull landing: the
    /// drawn glance. A landing anywhere else (ahead of the machine, an early
    /// miss) is nothing.
    pub fn glance(&self, footprint: Rect) -> Option<DozerEvent> {
        let moving = matches!(
            self.state,
            DozerState::CommittedChase | DozerState::Escaped(EscapePhase::Brake { .. })
        );
        (moving && self.push_box().overlaps(footprint)).then(|| DozerEvent::Glance {
            front_x: self.wheel_front().round() as i32,
        })
    }

    /// The tick of `hero`'s last rescue.
    pub fn last_rescue(&self, hero: HeroId) -> Option<u64> {
        self.last_rescue[hero.index()]
    }

    /// Fold the whole state into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(u64::from(self.o.to_bits()));
        mix(u64::from(self.speed.to_bits()));
        self.state.mix_into(mix);
        mix(self.band.map_or(0, |b| b.index() as u64 + 1));
        for r in self.last_rescue {
            mix(r.map_or(0, |t| t + 1));
        }
        let s = self.stats;
        for v in s.band_ticks {
            mix(u64::from(v));
        }
        for v in s.rescues {
            mix(u64::from(v));
        }
        mix(u64::from(s.lane_rescues));
        mix(u64::from(s.tunnel_rescues));
        mix(u64::from(s.pinned_holds));
        mix(u64::from(s.wheel_hits));
        self.wheel.mix_into(mix);
        // ACT1-B: only a profile with a charge block mixes its extra state, so room 3's
        // hashes are byte-identical.
        if self.has_charge() {
            self.charge.mix_into(mix);
            mix(u64::from(self.charges_used));
            mix(u64::from(self.armed));
            mix(u64::from(self.barrier_hit));
            mix(u64::from(self.pocket_seen[0]) | u64::from(self.pocket_seen[1]) << 1);
            mix(u64::from(self.timbers_smashed));
            mix(u64::from(self.wedged));
            mix(u64::from(s.pocket_rescues));
            mix(u64::from(s.behind_rescues));
        }
    }
}

/// ACT1-B: one living hero as the commit rule reads it: the body centre x and its own run
/// speed (`HeroMotion`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommitHero {
    pub x: f32,
    pub run_px_s: f32,
}

/// A hero's run speed in px/s (Carl's legacy motion, Donut's agile motion).
pub fn hero_run_px_s(id: HeroId) -> f32 {
    match id {
        HeroId::Carl => HeroMotion::CARL.run_speed,
        HeroId::Donut => crate::agility::agile_motion().run_speed,
    }
}

/// ACT1-B: the ticks a hero at `x` needs to walk until its whole body lies inside the
/// pocket's x span `(x0, x1)` (0 when it already does), at its own run speed. A hero past
/// the pocket walks back.
pub fn walk_ticks(hero: CommitHero, pocket: (f32, f32)) -> u32 {
    let (lo, hi) = (pocket.0 + HERO_HALF_WIDTH, pocket.1 - HERO_HALF_WIDTH);
    let dist = (lo - hero.x).max(hero.x - hi).max(0.0);
    (dist / (hero.run_px_s * TICK_SECONDS)).ceil() as u32
}

/// ACT1-B: the ticks the wheel front needs to reach the pocket's left edge: the tell's
/// ticks at `v_now`, then the charge speed from the first charge tick (set at once, so the
/// count is exact). Integrated tick by tick exactly as the machine moves.
pub fn wheel_ticks(ch: &ChargeTuning, front: f32, v_now: f32, pocket_left: f32) -> u32 {
    let mut f = front;
    for k in 1..=u32::from(ch.tell_ticks) {
        f += v_now * TICK_SECONDS;
        if f >= pocket_left {
            return k;
        }
    }
    for k in 1..=u32::from(ch.max_ticks) {
        f += f32::from(ch.speed_px_s) * TICK_SECONDS;
        if f >= pocket_left {
            return u32::from(ch.tell_ticks) + k;
        }
    }
    u32::from(ch.tell_ticks) + u32::from(ch.max_ticks) + 1
}

/// ACT1-B: the commit rule (integration plan section 5.3), a pure function. All four hold:
///
/// 1. every living hero's x is at least `arm_x`;
/// 2. the rearmost living body's left edge minus the wheel front is at least `min_gap_px`
///    (not [`Dozer::gap`], which counts only heroes ahead of the axle: a hero behind the
///    front gives a negative value and fails);
/// 3. every living hero reaches the pocket first: `walk_ticks + reach_margin_ticks <=
///    wheel_ticks`;
/// 4. the charge reaches the barricade: `front + (tell_ticks * v_now + speed * max_ticks) /
///    60 >= barricade_x` (vacuous without a barricade).
///
/// With no living hero the rule does not hold.
pub fn commit_rule_holds(
    ch: &ChargeTuning,
    pocket: (f32, f32),
    barricade_x: Option<f32>,
    front: f32,
    v_now: f32,
    heroes: &[CommitHero],
) -> bool {
    if heroes.is_empty() {
        return false;
    }
    if heroes.iter().any(|h| h.x < ch.arm_x as f32) {
        return false;
    }
    let min_left = heroes
        .iter()
        .map(|h| h.x - HERO_HALF_WIDTH)
        .fold(f32::INFINITY, f32::min);
    if min_left - front < ch.min_gap_px as f32 {
        return false;
    }
    let wheel = wheel_ticks(ch, front, v_now, pocket.0);
    if heroes
        .iter()
        .any(|h| walk_ticks(*h, pocket) + u32::from(ch.reach_margin_ticks) > wheel)
    {
        return false;
    }
    if let Some(bx) = barricade_x {
        let reach = (f32::from(ch.tell_ticks) * v_now
            + f32::from(ch.speed_px_s) * f32::from(ch.max_ticks))
            / 60.0;
        if front + reach + REACH_EPS_PX < bx {
            return false;
        }
    }
    true
}

/// A hero's body (x ± 12, foot .. foot + 52), world.
pub fn body_rect(x: f32, foot_y: f32) -> Rect {
    Rect {
        left: x - HERO_HALF_WIDTH,
        bottom: foot_y,
        width: 2.0 * HERO_HALF_WIDTH,
        height: 2.0 * HERO_HALF_HEIGHT,
    }
}

/// The feet of a body put at `x` on the floor: the top of the highest intact
/// standing solid (its bottom at or below the walking line) under the body
/// span, else the walking line. A lintel or a ceiling is never a floor.
pub fn standing_foot_y(solids: &[Block], x: f32, floor_y: f32) -> f32 {
    let (l, r) = (x - HERO_HALF_WIDTH, x + HERO_HALF_WIDTH);
    solids
        .iter()
        .filter(|b| b.intact && b.rect.bottom <= floor_y + 0.5)
        .filter(|b| b.rect.left < r && b.rect.right() > l)
        .map(|b| b.rect.top())
        .fold(None, |acc: Option<f32>, top| {
            Some(acc.map_or(top, |a| a.max(top)))
        })
        .unwrap_or(floor_y)
}

/// A world rectangle as whole px (rounded outward), still y up.
fn world_irect(r: Rect) -> IRect {
    let (x0, x1) = (r.left.floor() as i32, r.right().ceil() as i32);
    let (y0, y1) = (r.bottom.floor() as i32, r.top().ceil() as i32);
    IRect::new(x0, y0, (x1 - x0).max(1), (y1 - y0).max(1))
}

#[cfg(test)]
mod opening_tests;
#[cfg(test)]
mod tests;
