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
}

/// Section 6, the single definition of the useful release window. The machine
/// chases a hero who stands still with its centre at `hero_x`; a release on
/// chase tick R is **useful** when the landing on R + `land_delay_ticks` meets
/// the front impact collider (the tips past the footprint's left edge and the
/// tub front short of its right edge) and the wheel front on R +
/// `recovery_ticks` has not yet reached the hero's body (the hero can still turn
/// and run). The band is integrated tick by tick exactly as [`Dozer::step`]
/// does.
pub fn release_window(
    profile: &DozerProfile,
    frame: &StopFrame,
    hero_x: f32,
    recovery_ticks: u32,
) -> ReleaseWindow {
    let t = DozerTuning::active();
    let wheel_front = t.wheel_front();
    let tips_ahead = t.tips() - wheel_front;
    let tub_behind = wheel_front - t.dzq03_impact_collider.x0 as f32;
    let hero_left = hero_x - HERO_HALF_WIDTH;
    // The fronts of every chase tick until the wheel reaches the hero.
    let mut fronts = Vec::new();
    let mut front = frame.emerge_front_x;
    let gap = |front: f32| (hero_left > front).then_some(hero_left - front);
    let mut speed = profile.speed_of(profile.band(gap(front)));
    let limit = recovery_ticks.max(frame.land_delay_ticks) as usize + 4096;
    for _ in 0..limit {
        fronts.push(front);
        if front >= hero_left {
            break;
        }
        let target = profile.speed_of(profile.band(gap(front)));
        speed = profile.approach(speed, target);
        front += speed * TICK_SECONDS;
    }
    let mut out = ReleaseWindow::default();
    for r in 0..fronts.len() {
        let (Some(&landing), Some(&recovered)) = (
            fronts.get(r + frame.land_delay_ticks as usize),
            fronts.get(r + recovery_ticks as usize),
        ) else {
            break;
        };
        let hits =
            landing + tips_ahead > frame.footprint_x0 && landing - tub_behind < frame.footprint_x1;
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
}

// ---------------------------------------------------------------- the site --

/// The room's Dozer data (the sidecar's `dozer` block, DZ-Q11, DZ-Q13, DZ-Q14,
/// DZ-Q22), in room px.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DozerSite {
    /// DZ-Q11: the lead hero's x that wakes the machine.
    pub trigger_x: i32,
    /// DZ-Q11: the machine wakes after this many dormant ticks anyway.
    pub idle_trigger_ticks: u32,
    /// DZ-Q13: the wheel front at emergence (the gate's right edge).
    pub emerge_front_x: i32,
    /// DZ-Q14: the wheel front when the brake ends.
    pub stop_x: i32,
    /// DZ-Q22: the tunnel's lintel solid (layout px, y down).
    pub lintel: IRect,
}

// --------------------------------------------------------------- the events --

/// Where a rescue put a bot (section 7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RescueSpot {
    Lane,
    Tunnel,
}

impl RescueSpot {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Lane => "lane",
            Self::Tunnel => "tunnel",
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
}

impl RescueCause {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Near => "near",
            Self::BehindAxle => "behind_axle",
            Self::OffScreen => "off_screen",
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
    /// A stop frame landed on the front collider.
    Stunned { front_x: i32 },
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
            | Self::Stunned { front_x } => mix(front_x as u32 as u64),
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
            Self::WheelHit { hero } | Self::PinnedHold { hero } => mix(hero.index() as u64),
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
            | Self::Stunned { front_x } => format!("front_x={front_x}"),
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
            Self::WheelHit { hero } | Self::PinnedHold { hero } => format!("hero={hero:?}"),
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
        }
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
    }

    // ------------------------------------------------------------ the step --

    /// One tick (see the module's tick order).
    pub fn step(&mut self, ctx: &DozerCtx<'_>, out: &mut DozerOut) {
        let before = self.o;
        let moved = self.advance(ctx, out);
        let heroes = self.push_out(ctx, before, moved, out);
        let rescued = self.rescue(ctx, &heroes, out);
        self.wheel_test(ctx, &heroes, rescued, out);
    }

    /// Step 1: the state's move. Returns whether the origin moved (so the
    /// pinned rule may revert it).
    fn advance(&mut self, ctx: &DozerCtx<'_>, out: &mut DozerOut) -> bool {
        let t = self.tuning;
        match self.state {
            DozerState::Dormant { ticks } => {
                let lead = ctx
                    .heroes
                    .iter()
                    .filter(|h| h.present)
                    .map(|h| h.x)
                    .max_by(f32::total_cmp);
                let by_trigger = lead.is_some_and(|x| x >= self.site.trigger_x as f32);
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
        if !self.state.emerged() {
            return heroes;
        }
        let box_ = self.push_box();
        let mut shifts: Vec<(HeroId, f32)> = Vec::new();
        let mut pinned: Option<HeroId> = None;
        for h in heroes.iter().filter(|h| h.present) {
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
    /// heroes.
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
        let t = self.tuning;
        let stop_x = self.site.stop_x as f32;
        let front = self.wheel_front();
        let (axle_x, _) = self.axle();
        let zone_front = self.tips() + f32::from(t.dzq16_nav_lead_px);
        for h in heroes.iter().filter(|h| h.present && h.bot) {
            let left = h.x - HERO_HALF_WIDTH;
            // A body at or past stop_x is out of reach at every tick (section 5).
            if left >= stop_x {
                continue;
            }
            let in_view = h.x >= ctx.view_x.0 && h.x <= ctx.view_x.1;
            let cause = if h.x < axle_x {
                RescueCause::BehindAxle
            } else if left < front + f32::from(t.dzq17_rescue_band_px) {
                RescueCause::Near
            } else if !in_view && h.x >= self.o + t.dzq02_push_box.x0 as f32 && h.x < zone_front {
                RescueCause::OffScreen
            } else {
                continue;
            };
            let leader = &heroes[h.id.other().index()];
            let x_lane = (leader.x + f32::from(t.dzq17_rescue_ahead_px))
                .max(zone_front + f32::from(t.dzq17_rescue_zone_margin_px));
            let view_margin = f32::from(t.dzq17_lane_spot_view_margin_px);
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
            let foot_y = standing_foot_y(ctx.solids, x, self.floor_y);
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
            }
            rescued[h.id.index()] = true;
        }
        rescued
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
            if !h.present || rescued[h.id.index()] {
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
    /// once, only from the chase. `true` when it took effect.
    pub fn apply_falling_mass(&mut self, out: &mut DozerOut) -> bool {
        if self.state != DozerState::CommittedChase {
            return false;
        }
        self.speed = 0.0;
        self.state = DozerState::ImpactStunned {
            ticks_left: self.tuning.dzq15_stunned_ticks,
        };
        out.events.push(DozerEvent::Stunned {
            front_x: self.wheel_front().round() as i32,
        });
        true
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
    }
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
mod tests;
