//! HAZ-A1: the Rot Sticker's knobs and Bevy-free domain
//! (`docs/01-design/enemy-gameplay-slime-rot.md`, section 10 and HAZ-Q24..Q38).
//!
//! A Rot Sticker is an **environmental prop**, not an enemy: a dormant
//! explosive on one wall or ceiling track that wakes when a hero is near,
//! creeps toward the spot above or beside that hero, and goes off 24 ticks
//! after a hero's body comes within 32 px or anything hits it. It has no
//! health, is never needed to clear a room and is never set off by an enemy
//! walking near it. This module owns everything that needs no scene:
//!
//! * [`RotTuning`]: every `hazq##` knob of
//!   `assets/data/gameplay/enemy_rot_sticker_v0.json`, loaded, validated and
//!   FNV-1a pinned (the slime half is `enemy_slime_v0.json`,
//!   [`crate::slime`]);
//! * the track geometry ([`Track`], [`Orientation`]): the dome centre 6 px off
//!   the surface and the ROT-01 shell hurtbox, rotated with the orientation;
//! * the machine `latched -> scuttling -> warning -> spent`
//!   ([`RotSticker::step`]): wake and sleep, scuttle at 1 px per tick, the
//!   collider trigger circle with line of sight, hit stimuli through BOIL-B
//!   core's [`Stimulus`] intake, rule B (a Magic Missile bolt within the
//!   trigger circle), the committed 24-tick warning, the dynamite chain and the
//!   initiator credit rule;
//! * the Rot blast ([`RotBlast`]): the `BlastProfile` row as data (core band
//!   none, also at `sq == 0`; outer 48; `hero_damage: AllHeroes`), one
//!   evaluation per victim per blast, and the effects on each victim kind;
//! * the presentation frame table ([`warning_frame`], a pure function of the
//!   ticks left to the detonation).
//!
//! Frame: **world px, y up** (`Rect`; the sim's convention). A [`Track`] is
//! given in the same frame; the sidecar's layout y (down) is converted by the
//! caller with the room's walking line. The ROT-01 shell box is surface-local:
//! `x` along the surface, `y` out of it.
//!
//! The detonation itself is an authored `BlastCharge` appended at attempt start
//! (HAZ-A2, the BOIL-D drum precedent); this core mirrors the charge fields
//! (`detonate_tick`, `owner`) so the scene copies them each tick, and reports
//! the detonation tick as an event.
//!
//! No RNG, no wall-clock time. Every switch is read through
//! [`RotTuning::on`], never raw.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::blast::BLAST_CENTRE_DY;
use crate::boss::{HeroBlow, HeroView};
use crate::cf02::BoxPx;
use crate::donut_mana::content_hash;
use crate::env_prop::Stimulus;
use crate::hero_damage::HeroDamagePolicy;
use crate::party::HeroId;
use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Rect};

/// The tuning file, embedded so the binary and the tests read the same bytes.
pub const TUNING_JSON: &str = include_str!("../assets/data/gameplay/enemy_rot_sticker_v0.json");

/// The committed FNV-1a 64 content hash of [`TUNING_JSON`]. Editing the file
/// without updating this value fails [`RotTuning::pinned`] and the pin test.
pub const TUNING_HASH: u64 = 0xe2d9_db27_cbc9_de5a;

/// The key of the pin in a session manifest's `content_hashes`.
pub const PIN_KEY: &str = "rot_sticker_tuning";

/// The `HeroBlow::source` base of a Rot blast (the ASCII "RT"); the sticker
/// index goes in the high half ([`rot_source`]).
pub const ROT_SOURCE_ID: u32 = 0x5254;

/// The source of the blast of sticker `index`.
pub const fn rot_source(index: u8) -> u32 {
    ROT_SOURCE_ID | ((index as u32) << 16)
}

/// A dynamite chain detonates a sticker this many ticks after the blast that
/// reached it (CP-Q34).
pub const CHAIN_DELAY_TICKS: u64 = 8;

/// A track is at most this long (HAZ-Q23).
pub const MAX_TRACK_PX: i32 = 160;

/// Both heroes' colliders are 24 x 52 (DM-Q07): the trigger measures it.
pub const COLLIDER_W: f32 = HERO_HALF_WIDTH * 2.0;
pub const COLLIDER_H: f32 = HERO_HALF_HEIGHT * 2.0;

/// A knob file problem. A load error names the field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RotTuningError {
    Parse(String),
    Field { field: &'static str, reason: String },
    HashMismatch { expected: u64, actual: u64 },
}

impl core::fmt::Display for RotTuningError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "rot sticker tuning: {msg}"),
            Self::Field { field, reason } => {
                write!(f, "rot sticker tuning: field `{field}`: {reason}")
            }
            Self::HashMismatch { expected, actual } => write!(
                f,
                "rot sticker tuning hash {actual:#018x} is not the pinned {expected:#018x}"
            ),
        }
    }
}

impl std::error::Error for RotTuningError {}

/// Which body the proximity trigger measures (HAZ-Q27).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SenseBox {
    /// The 24 x 52 collider of either hero (adopted).
    Collider,
    /// The hero's hurtbox (Donut's is 20 x 24).
    Hurtbox,
}

/// Every Rot Sticker knob of the HAZ-0 sheet, section 16.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotTuning {
    pub schema_version: u32,
    /// Q02: the data kill switch of the sticker.
    pub hazq02_rot_enabled: bool,
    /// Q24: the shell hurtbox, surface-local (`x` along the surface, `y` out of
    /// it), and the dome offset.
    pub hazq24_hurtbox: BoxPx,
    pub hazq24_dome_offset_px: u8,
    /// Q25: wake distance, sleep distance and the ticks without a hero.
    pub hazq25_wake_px: u16,
    pub hazq25_sleep_px: u16,
    pub hazq25_sleep_ticks: u16,
    /// Q26: scuttle speed (a multiple of 60 px/s) and the re-pick hysteresis.
    pub hazq26_speed_px_s: u16,
    pub hazq26_switch_px: u16,
    /// Q27: the trigger circle and the body it measures.
    pub hazq27_trigger_px: u16,
    pub hazq27_sense_box: SenseBox,
    /// Q28: the committed warning.
    pub hazq28_warning_ticks: u16,
    /// Q29: a hit (melee, bolt, flier) provokes.
    pub hazq29_hit_provokes: bool,
    /// Q30: the Rot blast profile row.
    pub hazq30_core_radius_px: u16,
    pub hazq30_outer_radius_px: u16,
    pub hazq30_damage: u8,
    pub hazq30_launch_x: u16,
    pub hazq30_launch_y: u16,
    pub hazq30_line_of_sight: bool,
    pub hazq30_hero_damage: String,
    pub hazq30_breaks_structures: bool,
    /// Q31: the knock on heroes (the caller signs it).
    pub hazq31_knock_x: u16,
    pub hazq31_knock_y: u16,
    /// Q35: a sticker killed during its warning becomes a dud (not in v1).
    pub hazq35_defuse: bool,
    /// Q38: the radius ring around the dome centre.
    pub hazq38_ring: bool,
    /// Q47: audio switches.
    pub hazq47_provoke_cue: bool,
    pub hazq47_scuttle_loop: bool,
}

fn bad(field: &'static str, reason: &str) -> Result<(), RotTuningError> {
    Err(RotTuningError::Field {
        field,
        reason: reason.to_string(),
    })
}

/// The Rot blast as a profile row (CP-B1's `BlastProfile` adopts this shape).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RotBlastProfile {
    pub core_radius: u16,
    pub outer_radius: u16,
    pub damage: u8,
    pub launch_x: u16,
    pub launch_y: u16,
    pub line_of_sight: bool,
    pub hero_damage: HeroDamagePolicy,
    pub breaks_structures: bool,
    pub hero_knock_x: u16,
    pub hero_knock_y: u16,
}

impl RotTuning {
    /// Parse and validate.
    pub fn from_json(text: &str) -> Result<Self, RotTuningError> {
        let tuning: Self =
            serde_json::from_str(text).map_err(|err| RotTuningError::Parse(err.to_string()))?;
        tuning.validate()?;
        Ok(tuning)
    }

    /// Parse the embedded file and require the pinned content hash.
    pub fn pinned() -> Result<Self, RotTuningError> {
        Self::from_json_pinned(TUNING_JSON, TUNING_HASH)
    }

    /// Parse `text` and require its content hash to equal `expected`.
    pub fn from_json_pinned(text: &str, expected: u64) -> Result<Self, RotTuningError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(RotTuningError::HashMismatch { expected, actual });
        }
        Self::from_json(text)
    }

    /// The one way to read a switch of the file: effective only while the data
    /// kill switch `hazq02_rot_enabled` is on.
    pub fn on(&self, switch: bool) -> bool {
        self.hazq02_rot_enabled && switch
    }

    /// Whether Rot Stickers exist at all (the kill switch).
    pub fn enabled(&self) -> bool {
        self.hazq02_rot_enabled
    }

    /// Scuttle speed in whole px per tick.
    pub fn scuttle_px_per_tick(&self) -> i32 {
        i32::from(self.hazq26_speed_px_s / 60)
    }

    /// The `hero_damage` policy of the blast (validated at load).
    pub fn hero_damage(&self) -> HeroDamagePolicy {
        HeroDamagePolicy::parse(&self.hazq30_hero_damage).unwrap_or(HeroDamagePolicy::AllHeroes)
    }

    /// The Rot blast profile row.
    pub fn blast_profile(&self) -> RotBlastProfile {
        RotBlastProfile {
            core_radius: self.hazq30_core_radius_px,
            outer_radius: self.hazq30_outer_radius_px,
            damage: self.hazq30_damage,
            launch_x: self.hazq30_launch_x,
            launch_y: self.hazq30_launch_y,
            line_of_sight: self.hazq30_line_of_sight,
            hero_damage: self.hero_damage(),
            breaks_structures: self.hazq30_breaks_structures,
            hero_knock_x: self.hazq31_knock_x,
            hero_knock_y: self.hazq31_knock_y,
        }
    }

    pub fn validate(&self) -> Result<(), RotTuningError> {
        let b = &self.hazq24_hurtbox;
        if b.x0 >= b.x1 || b.y0 >= b.y1 {
            return bad("hazq24_hurtbox", "needs x0 < x1 and y0 < y1");
        }
        if self.hazq24_dome_offset_px == 0 {
            return bad("hazq24_dome_offset_px", "must be positive");
        }
        if self.hazq25_wake_px == 0 {
            return bad("hazq25_wake_px", "must be positive");
        }
        if self.hazq25_sleep_px < self.hazq25_wake_px {
            return bad("hazq25_sleep_px", "must not be below hazq25_wake_px");
        }
        if self.hazq25_sleep_ticks == 0 {
            return bad("hazq25_sleep_ticks", "must be positive");
        }
        if self.hazq26_speed_px_s == 0 || !self.hazq26_speed_px_s.is_multiple_of(60) {
            return bad(
                "hazq26_speed_px_s",
                "must be a positive multiple of 60 (whole px per tick)",
            );
        }
        if self.hazq26_switch_px == 0 {
            return bad("hazq26_switch_px", "must be positive");
        }
        if self.hazq27_trigger_px == 0 {
            return bad("hazq27_trigger_px", "must be positive");
        }
        // Owner (#178): the detonation warning is 18..24 fixed ticks.
        if !(18..=24).contains(&self.hazq28_warning_ticks) {
            return bad("hazq28_warning_ticks", "must be 18..=24 (owner range)");
        }
        // Owner (#178): the blast radius is 40..56 px.
        if !(40..=56).contains(&self.hazq30_outer_radius_px) {
            return bad("hazq30_outer_radius_px", "must be 40..=56 (owner range)");
        }
        if self.hazq30_core_radius_px > self.hazq30_outer_radius_px {
            return bad(
                "hazq30_core_radius_px",
                "must not exceed hazq30_outer_radius_px",
            );
        }
        if self.hazq30_damage == 0 {
            return bad("hazq30_damage", "must be positive");
        }
        match HeroDamagePolicy::parse(&self.hazq30_hero_damage) {
            None => return bad("hazq30_hero_damage", "unknown policy name"),
            Some(HeroDamagePolicy::OwnerReduced) => {
                return bad(
                    "hazq30_hero_damage",
                    "a world blast has no owner to reduce the damage for",
                );
            }
            Some(_) => {}
        }
        if self.hazq31_knock_x == 0 {
            return bad("hazq31_knock_x", "must be positive");
        }
        Ok(())
    }
}

/// The pinned tuning: what a simulation with stickers reads. One shared static.
pub fn active() -> &'static RotTuning {
    static ACTIVE: std::sync::OnceLock<RotTuning> = std::sync::OnceLock::new();
    ACTIVE.get_or_init(|| {
        RotTuning::pinned().expect("the embedded rot sticker data is pinned and valid")
    })
}

// ---------------------------------------------------------------- geometry

/// The surface a sticker clings to (HAZ-Q23): floor stickers and corners are
/// not in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Orientation {
    /// A soffit or slab underside, the face looking down.
    Ceiling,
    /// The wall on the sticker's left, the face looking +x.
    WallLeft,
    /// The wall on the sticker's right, the face looking -x.
    WallRight,
}

impl Orientation {
    pub const ALL: [Self; 3] = [Self::Ceiling, Self::WallLeft, Self::WallRight];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Ceiling => "ceiling",
            Self::WallLeft => "wall_left",
            Self::WallRight => "wall_right",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|o| o.name() == name)
    }

    pub const fn code(self) -> u64 {
        self as u64
    }
}

/// One straight track on a surface, in world px (y up). `fixed` is the surface
/// coordinate (the y of a ceiling's underside, the x of a wall's face) and the
/// sticker moves along `lo..=hi` (x for a ceiling, y for a wall).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Track {
    pub orientation: Orientation,
    pub fixed: i32,
    pub lo: i32,
    pub hi: i32,
    pub start: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackError {
    /// `lo > hi`.
    Reversed,
    /// A track is 0..=160 px long.
    TooLong { len: i32 },
    /// The start point is not on the track.
    StartOffTrack,
    /// The two end points (or the start) are not on one axis-aligned line.
    NotAxisAligned,
}

impl core::fmt::Display for TrackError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Reversed => write!(f, "the track runs backwards"),
            Self::TooLong { len } => write!(f, "the track is {len} px, at most {MAX_TRACK_PX}"),
            Self::StartOffTrack => write!(f, "the start point is not on the track"),
            Self::NotAxisAligned => write!(
                f,
                "the end points and the start point must lie on one line along the surface"
            ),
        }
    }
}

impl std::error::Error for TrackError {}

impl Track {
    pub fn new(
        orientation: Orientation,
        fixed: i32,
        lo: i32,
        hi: i32,
        start: i32,
    ) -> Result<Self, TrackError> {
        if lo > hi {
            return Err(TrackError::Reversed);
        }
        if hi - lo > MAX_TRACK_PX {
            return Err(TrackError::TooLong { len: hi - lo });
        }
        if start < lo || start > hi {
            return Err(TrackError::StartOffTrack);
        }
        Ok(Self {
            orientation,
            fixed,
            lo,
            hi,
            start,
        })
    }

    /// From two end points and a start point `(x, y)` in world px.
    pub fn from_points(
        orientation: Orientation,
        a: (i32, i32),
        b: (i32, i32),
        start: (i32, i32),
    ) -> Result<Self, TrackError> {
        let (fixed, a_s, b_s, start_s) = match orientation {
            Orientation::Ceiling => {
                if a.1 != b.1 || a.1 != start.1 {
                    return Err(TrackError::NotAxisAligned);
                }
                (a.1, a.0, b.0, start.0)
            }
            Orientation::WallLeft | Orientation::WallRight => {
                if a.0 != b.0 || a.0 != start.0 {
                    return Err(TrackError::NotAxisAligned);
                }
                (a.0, a.1, b.1, start.1)
            }
        };
        Self::new(orientation, fixed, a_s.min(b_s), a_s.max(b_s), start_s)
    }

    pub fn length(&self) -> i32 {
        self.hi - self.lo
    }

    /// A length-0 track is a fixed sticker.
    pub fn is_fixed(&self) -> bool {
        self.lo == self.hi
    }

    /// The dome centre at along-coordinate `s`: `offset` px off the surface
    /// along its normal (ROT-01).
    pub fn dome(&self, s: i32, offset: u8) -> (f32, f32) {
        let o = f32::from(offset);
        match self.orientation {
            Orientation::Ceiling => (s as f32, self.fixed as f32 - o),
            Orientation::WallLeft => (self.fixed as f32 + o, s as f32),
            Orientation::WallRight => (self.fixed as f32 - o, s as f32),
        }
    }

    /// The shell hurtbox at `s`: the surface-local box rotated with the
    /// orientation. Exactly the drawn shell.
    pub fn shell(&self, s: i32, b: &BoxPx) -> Rect {
        let (a0, a1) = (i32::from(b.x0), i32::from(b.x1));
        let (o0, o1) = (i32::from(b.y0), i32::from(b.y1));
        match self.orientation {
            Orientation::Ceiling => Rect {
                left: (s + a0) as f32,
                bottom: (self.fixed - o1) as f32,
                width: (a1 - a0) as f32,
                height: (o1 - o0) as f32,
            },
            Orientation::WallLeft => Rect {
                left: (self.fixed + o0) as f32,
                bottom: (s + a0) as f32,
                width: (o1 - o0) as f32,
                height: (a1 - a0) as f32,
            },
            Orientation::WallRight => Rect {
                left: (self.fixed - o1) as f32,
                bottom: (s + a0) as f32,
                width: (o1 - o0) as f32,
                height: (a1 - a0) as f32,
            },
        }
    }

    /// The 32 x 32 art frame around the dome centre at `s` (the in-view check
    /// of the wake and trigger).
    pub fn frame(&self, s: i32, offset: u8) -> Rect {
        let (cx, cy) = self.dome(s, offset);
        Rect {
            left: cx - 16.0,
            bottom: cy - 16.0,
            width: 32.0,
            height: 32.0,
        }
    }

    /// The along-coordinate of a point projected onto the track line.
    pub fn project(&self, x: f32, y: f32) -> f32 {
        match self.orientation {
            Orientation::Ceiling => x,
            Orientation::WallLeft | Orientation::WallRight => y,
        }
    }
}

/// Squared distance from a point to the nearest point of a rectangle.
pub fn dist2_to_rect(rect: Rect, x: f32, y: f32) -> f32 {
    let (nx, ny) = nearest_point(rect, x, y);
    (nx - x) * (nx - x) + (ny - y) * (ny - y)
}

/// The point of `rect` nearest to `(x, y)`.
pub fn nearest_point(rect: Rect, x: f32, y: f32) -> (f32, f32) {
    (
        x.clamp(rect.left, rect.right()),
        y.clamp(rect.bottom, rect.top()),
    )
}

/// CP-Q22: the segment from `a` to `b` is blocked by the **interior** of an
/// intact solid. Touching an edge is not blocked.
pub fn segment_blocked(a: (f32, f32), b: (f32, f32), solids: &[Rect]) -> bool {
    solids.iter().any(|s| segment_enters(a, b, *s))
}

fn segment_enters(a: (f32, f32), b: (f32, f32), r: Rect) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let mut t0 = 0.0_f32;
    let mut t1 = 1.0_f32;
    for (p, d, lo, hi) in [(a.0, dx, r.left, r.right()), (a.1, dy, r.bottom, r.top())] {
        if d == 0.0 {
            if p <= lo || p >= hi {
                return false;
            }
            continue;
        }
        let (ta, tb) = ((lo - p) / d, (hi - p) / d);
        let (tmin, tmax) = if ta < tb { (ta, tb) } else { (tb, ta) };
        t0 = t0.max(tmin);
        t1 = t1.min(tmax);
        if t0 >= t1 {
            return false;
        }
    }
    // A zero-length segment (both d == 0) that is strictly inside blocks.
    true
}

/// Whether `rect` is inside the circle of `radius` around `c` (the nearest
/// point of the box, `<=`) and, with `los`, not shielded by a solid.
pub fn circle_reaches(c: (f32, f32), radius: f32, rect: Rect, los: bool, solids: &[Rect]) -> bool {
    let (nx, ny) = nearest_point(rect, c.0, c.1);
    let d2 = (nx - c.0) * (nx - c.0) + (ny - c.1) * (ny - c.1);
    if d2 > radius * radius {
        return false;
    }
    !(los && segment_blocked(c, (nx, ny), solids))
}

/// A hero's 24 x 52 collider from its feet.
pub fn collider(hero: &HeroView) -> Rect {
    Rect {
        left: hero.x - HERO_HALF_WIDTH,
        bottom: hero.foot_y,
        width: COLLIDER_W,
        height: COLLIDER_H,
    }
}

// ------------------------------------------------------------------- state

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickerState {
    Latched,
    Scuttling,
    Warning,
    Spent,
}

impl StickerState {
    pub const fn code(self) -> u64 {
        self as u64
    }
}

/// What provoked a sticker (the `RotProvoked` cause of HAZ-A2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotCause {
    Proximity,
    Hit,
    Projectile,
    Flier,
    Chain,
}

impl RotCause {
    pub const fn code(self) -> u64 {
        self as u64
    }
}

/// A Magic Missile bolt this tick (rule B).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Missile {
    pub id: u32,
    pub rect: Rect,
    pub owner: Option<u32>,
}

/// What the scene tells a sticker each tick.
#[derive(Debug, Clone, Copy)]
pub struct StickerCtx<'a> {
    pub tick: u64,
    pub heroes: &'a [HeroView],
    /// The player of each hero, by `HeroId::index` (in solo the one human
    /// player, also when the AI-driven hero is the one in range).
    pub players: [Option<u32>; 2],
    /// The sticker's whole 32 x 32 frame is inside the simulation camera view.
    pub in_view: bool,
    /// Intact solids (line of sight).
    pub solids: &'a [Rect],
    /// Stimuli of this tick that reached this prop.
    pub stimuli: &'a [Stimulus],
    pub missiles: &'a [Missile],
}

/// The provoke of one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Provoke {
    pub cause: RotCause,
    pub player: Option<u32>,
}

/// The detonation of one tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Detonation {
    pub tick: u64,
    pub owner: Option<u32>,
    pub dome: (f32, f32),
}

/// What one sticker step reports.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StickerStep {
    pub provoked: Option<Provoke>,
    pub detonated: Option<Detonation>,
    /// Bolts consumed by the provoke (rule B).
    pub consumed: Vec<u32>,
}

/// One Rot Sticker.
#[derive(Debug, Clone, PartialEq)]
pub struct RotSticker {
    track: Track,
    s: i32,
    state: StickerState,
    target: Option<HeroId>,
    away_ticks: u16,
    /// The mirror of the charge's `detonate_tick`: 0 = unlit.
    detonate_tick: u64,
    /// The mirror of the charge's `owner`.
    owner: Option<u32>,
    cause: Option<RotCause>,
    provoked_tick: u64,
    flash_left: u8,
}

impl RotSticker {
    pub fn new(track: Track) -> Self {
        Self {
            s: track.start,
            track,
            state: StickerState::Latched,
            target: None,
            away_ticks: 0,
            detonate_tick: 0,
            owner: None,
            cause: None,
            provoked_tick: 0,
            flash_left: 0,
        }
    }

    pub fn track(&self) -> &Track {
        &self.track
    }

    /// The along-coordinate now.
    pub fn s(&self) -> i32 {
        self.s
    }

    pub fn state(&self) -> StickerState {
        self.state
    }

    /// The charge's `detonate_tick`; 0 while unlit.
    pub fn detonate_tick(&self) -> u64 {
        self.detonate_tick
    }

    /// The charge's `owner` (the initiator's player), `None` until credited.
    pub fn owner(&self) -> Option<u32> {
        self.owner
    }

    pub fn cause(&self) -> Option<RotCause> {
        self.cause
    }

    pub fn target(&self) -> Option<HeroId> {
        self.target
    }

    pub fn flash_left(&self) -> u8 {
        self.flash_left
    }

    pub fn is_spent(&self) -> bool {
        self.state == StickerState::Spent
    }

    pub fn dome(&self, t: &RotTuning) -> (f32, f32) {
        self.track.dome(self.s, t.hazq24_dome_offset_px)
    }

    /// The shell hurtbox; `None` once spent (the body is gone).
    pub fn hurtbox(&self, t: &RotTuning) -> Option<Rect> {
        (!self.is_spent()).then(|| self.track.shell(self.s, &t.hazq24_hurtbox))
    }

    /// The 32 x 32 art frame (for the in-view test).
    pub fn frame(&self, t: &RotTuning) -> Rect {
        self.track.frame(self.s, t.hazq24_dome_offset_px)
    }

    /// The charge position of this tick: `centre()` of a `BlastCharge` at
    /// `(x, foot_y)` is the dome centre.
    pub fn charge_xy(&self, t: &RotTuning) -> (f32, f32) {
        let (x, y) = self.dome(t);
        (x, y - BLAST_CENTRE_DY)
    }

    /// Ticks left to the detonation (`None` unless in Warning).
    pub fn remaining(&self, tick: u64) -> Option<u64> {
        (self.state == StickerState::Warning).then(|| self.detonate_tick.saturating_sub(tick))
    }

    /// The ROT-01 frame of this tick (HAZ-Q37) while in Warning.
    pub fn frame_index(&self, tick: u64) -> Option<u8> {
        self.remaining(tick).map(warning_frame)
    }

    /// The trigger circle's bounding box (a hazard-feed row, awake stickers).
    pub fn trigger_bounds(&self, t: &RotTuning) -> Rect {
        circle_bounds(self.dome(t), f32::from(t.hazq27_trigger_px))
    }

    /// The blast circle's bounding box (a hazard-feed row, warning stickers).
    pub fn blast_bounds(&self, t: &RotTuning) -> Rect {
        circle_bounds(self.dome(t), f32::from(t.hazq30_outer_radius_px))
    }

    fn provoke(
        &mut self,
        t: &RotTuning,
        tick: u64,
        cause: RotCause,
        player: Option<u32>,
    ) -> Provoke {
        self.state = StickerState::Warning;
        self.detonate_tick = tick + u64::from(t.hazq28_warning_ticks);
        self.provoked_tick = tick;
        self.cause = Some(cause);
        if self.owner.is_none() {
            self.owner = player;
        }
        if matches!(
            cause,
            RotCause::Hit | RotCause::Projectile | RotCause::Flier
        ) {
            // A hit gives the attacker no hit-stop; the sticker tints for 2 ticks.
            self.flash_left = 2;
        }
        Provoke { cause, player }
    }

    /// A dynamite chain reached this sticker at tick `now` (CP-Q34, the
    /// requirement on CP-C): an unlit charge is set to `now + 8`, one due later
    /// is pulled forward to `now + 8`, one due sooner is left alone. The owner
    /// is written only if it is none (the first initiator wins). Returns the
    /// provoke when this chain lit a latched or scuttling sticker.
    pub fn chain(&mut self, now: u64, owner: Option<u32>) -> Option<Provoke> {
        if self.is_spent() {
            return None;
        }
        let due = now + CHAIN_DELAY_TICKS;
        match self.state {
            StickerState::Latched | StickerState::Scuttling => {
                self.state = StickerState::Warning;
                self.detonate_tick = due;
                self.provoked_tick = now;
                self.cause = Some(RotCause::Chain);
                if self.owner.is_none() {
                    self.owner = owner;
                }
                Some(Provoke {
                    cause: RotCause::Chain,
                    player: owner,
                })
            }
            StickerState::Warning => {
                if self.detonate_tick > due {
                    self.detonate_tick = due;
                }
                if self.owner.is_none() {
                    self.owner = owner;
                }
                None
            }
            StickerState::Spent => None,
        }
    }

    /// One fixed tick of the env step (after the charges step): the detonation,
    /// the stimuli, the proximity and missile triggers, then wake, scuttle and
    /// sleep.
    pub fn step(&mut self, t: &RotTuning, ctx: &StickerCtx<'_>) -> StickerStep {
        let mut out = StickerStep::default();
        self.flash_left = self.flash_left.saturating_sub(1);
        match self.state {
            StickerState::Spent => return out,
            StickerState::Warning => {
                if ctx.tick >= self.detonate_tick {
                    out.detonated = Some(Detonation {
                        tick: ctx.tick,
                        owner: self.owner,
                        dome: self.dome(t),
                    });
                    self.state = StickerState::Spent;
                }
                return out;
            }
            StickerState::Latched | StickerState::Scuttling => {}
        }
        if !t.enabled() {
            return out;
        }
        let dome = self.dome(t);

        // 1. Hit stimuli (BOIL-B core's intake): melee, projectile, flier.
        if t.on(t.hazq29_hit_provokes) {
            for st in ctx.stimuli {
                let cause = match st {
                    Stimulus::MeleeHit { weight, .. } if *weight >= 1 => Some(RotCause::Hit),
                    Stimulus::Projectile { weight, .. } if *weight >= 1 => {
                        Some(RotCause::Projectile)
                    }
                    Stimulus::LaunchedBody { .. } => Some(RotCause::Flier),
                    // A blast, a prop contact and a boss shove are ignored: a
                    // dynamite chain reaches the sticker through its charge.
                    _ => None,
                };
                if let Some(cause) = cause {
                    out.provoked = Some(self.provoke(t, ctx.tick, cause, st.initiator()));
                    return out;
                }
            }
        }

        // 2. Proximity: a hero's collider within the trigger circle, with line
        // of sight, the sticker fully in view. Heroes only.
        let trigger = f32::from(t.hazq27_trigger_px);
        if ctx.in_view {
            for hero in ctx.heroes.iter().filter(|h| h.present) {
                let body = match t.hazq27_sense_box {
                    SenseBox::Collider => collider(hero),
                    SenseBox::Hurtbox => hero.hurt,
                };
                if circle_reaches(dome, trigger, body, true, ctx.solids) {
                    let player = ctx.players[hero.id.index()];
                    out.provoked = Some(self.provoke(t, ctx.tick, RotCause::Proximity, player));
                    return out;
                }
            }
        }

        // 3. Rule B: a Magic Missile bolt within the trigger circle, with line
        // of sight, is consumed and credited to its owner.
        if t.on(t.hazq29_hit_provokes) && ctx.in_view {
            for m in ctx.missiles {
                if circle_reaches(dome, trigger, m.rect, true, ctx.solids) {
                    out.consumed.push(m.id);
                    out.provoked = Some(self.provoke(t, ctx.tick, RotCause::Projectile, m.owner));
                    return out;
                }
            }
        }

        self.wake_scuttle_sleep(t, ctx, dome);
        out
    }

    fn wake_scuttle_sleep(&mut self, t: &RotTuning, ctx: &StickerCtx<'_>, dome: (f32, f32)) {
        let dist = |h: &HeroView| -> f32 {
            let (cx, cy) = hero_centre(h);
            ((cx - dome.0).powi(2) + (cy - dome.1).powi(2)).sqrt()
        };
        let near = |h: &HeroView, px: u16| -> bool {
            dist2_to_rect(h.hurt, dome.0, dome.1) <= f32::from(px) * f32::from(px)
        };
        match self.state {
            StickerState::Latched => {
                if ctx.in_view
                    && ctx
                        .heroes
                        .iter()
                        .any(|h| h.present && near(h, t.hazq25_wake_px))
                {
                    self.state = StickerState::Scuttling;
                    self.away_ticks = 0;
                    self.target = self.nearest(ctx.heroes, &dist);
                }
            }
            StickerState::Scuttling => {
                // Sleep: no hero within the sleep distance for the sleep ticks.
                if ctx
                    .heroes
                    .iter()
                    .any(|h| h.present && near(h, t.hazq25_sleep_px))
                {
                    self.away_ticks = 0;
                } else {
                    self.away_ticks += 1;
                    if self.away_ticks >= t.hazq25_sleep_ticks {
                        self.state = StickerState::Latched;
                        self.target = None;
                        self.away_ticks = 0;
                        return;
                    }
                }
                self.pick_target(t, ctx, &dist);
                let Some(h) = self
                    .target
                    .and_then(|id| ctx.heroes.iter().find(|h| h.id == id && h.present))
                else {
                    return;
                };
                let (cx, cy) = hero_centre(h);
                let goal =
                    (self.track.project(cx, cy).round() as i32).clamp(self.track.lo, self.track.hi);
                let step = t.scuttle_px_per_tick();
                self.s += (goal - self.s).clamp(-step, step);
            }
            StickerState::Warning | StickerState::Spent => {}
        }
    }

    /// The nearest living hero by `dist`; ties go to Carl.
    fn nearest(&self, heroes: &[HeroView], dist: &dyn Fn(&HeroView) -> f32) -> Option<HeroId> {
        heroes
            .iter()
            .filter(|h| h.present)
            .min_by(|a, b| {
                dist(a)
                    .total_cmp(&dist(b))
                    .then(a.id.index().cmp(&b.id.index()))
            })
            .map(|h| h.id)
    }

    /// Re-pick only when the other hero is `hazq26_switch_px` nearer.
    fn pick_target(
        &mut self,
        t: &RotTuning,
        ctx: &StickerCtx<'_>,
        dist: &dyn Fn(&HeroView) -> f32,
    ) {
        let current = self
            .target
            .and_then(|id| ctx.heroes.iter().find(|h| h.id == id && h.present));
        let best = self.nearest(ctx.heroes, dist);
        match (current, best) {
            (None, b) => self.target = b,
            (Some(cur), Some(b)) if b != cur.id => {
                let other = ctx.heroes.iter().find(|h| h.id == b).unwrap();
                if dist(other) <= dist(cur) - f32::from(t.hazq26_switch_px) {
                    self.target = Some(b);
                }
            }
            _ => {}
        }
    }

    /// A defeat respawn (HAZ-Q49): a spent sticker stays spent; a sticker in
    /// Warning or Scuttling returns to Latched at its start with the charge
    /// written back (`detonate_tick` 0, owner none).
    pub fn defeat_respawn(&mut self) {
        if self.is_spent() {
            return;
        }
        self.restart_state();
    }

    /// A full restart: everything returns to its authored start, spent
    /// included.
    pub fn reset(&mut self) {
        self.restart_state();
    }

    fn restart_state(&mut self) {
        self.s = self.track.start;
        self.state = StickerState::Latched;
        self.target = None;
        self.away_ticks = 0;
        self.detonate_tick = 0;
        self.owner = None;
        self.cause = None;
        self.provoked_tick = 0;
        self.flash_left = 0;
    }

    /// Fold the full state into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.track.orientation.code());
        for v in [
            self.track.fixed,
            self.track.lo,
            self.track.hi,
            self.track.start,
            self.s,
        ] {
            mix(v as u32 as u64);
        }
        mix(self.state.code());
        mix(self.target.map_or(0, |h| h.index() as u64 + 1));
        mix(u64::from(self.away_ticks));
        mix(self.detonate_tick);
        mix(self.owner.map_or(0, |p| u64::from(p) + 1));
        mix(self.cause.map_or(0, |c| c.code() + 1));
        mix(self.provoked_tick);
        mix(u64::from(self.flash_left));
    }
}

fn hero_centre(h: &HeroView) -> (f32, f32) {
    h.hurt.center()
}

fn circle_bounds(c: (f32, f32), r: f32) -> Rect {
    Rect {
        left: c.0 - r,
        bottom: c.1 - r,
        width: 2.0 * r,
        height: 2.0 * r,
    }
}

/// The ROT-01 frame of a sticker in Warning with `remaining` ticks left to the
/// detonation (HAZ-Q37): frame 7 while 19 or more are left, frame 8 (the whole
/// face lit) from 18 down to 2, frame 9 with 1 left and frame 10 on the
/// detonation tick. A warning of `t`: frame 7 on `t..t+5`, 8 on `t+6..t+22`,
/// 9 on `t+23`, 10 on `t+24`; one set off by a chain at `c`: frame 8 on
/// `c..c+6`, 9 on `c+7`, 10 on `c+8`. A playback retime of a held frame, not a
/// redraw (K10).
pub const fn warning_frame(remaining: u64) -> u8 {
    match remaining {
        0 => 10,
        1 => 9,
        2..=18 => 8,
        _ => 7,
    }
}

// ------------------------------------------------------------------- blast

/// What a Rot blast can reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Hero(HeroId),
    Goblin,
    Engineer,
    Llama,
    Slime,
    /// Another Rot Sticker: nothing happens to it.
    Sticker,
}

/// One body the blast may reach. `id` is the enemy instance, or the hero index.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlastTarget {
    pub id: u32,
    pub kind: TargetKind,
    /// The body's hurtbox.
    pub body: Rect,
    /// The pivot x of the body (the knock is signed away from the dome).
    pub x: f32,
}

/// The reaction an enemy shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnemyReaction {
    /// A goblin or Engineer: the CF02 launched body (CP-Q36 analogue).
    Launch { vx: f32, vy: f32 },
    /// A llama: its kick reaction (CP-Q26 analogue).
    Kick,
    /// A slime (gel): damage and nothing else.
    None,
}

/// What one blast does to one victim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlastEffect {
    /// `hurt_hero` with this blow (invulnerability and the solo last-HP guard
    /// apply; non-evadable).
    HeroHurt(HeroBlow),
    /// Damage to an enemy; a lethal one is `Explosion { point_blank: false }`
    /// credited to `credit`.
    EnemyHit {
        id: u32,
        kind: TargetKind,
        damage: u8,
        reaction: EnemyReaction,
        credit: Option<u32>,
    },
}

/// One Rot blast: fired once, one evaluation per victim.
#[derive(Debug, Clone, PartialEq)]
pub struct RotBlast {
    dome: (f32, f32),
    profile: RotBlastProfile,
    owner: Option<u32>,
    source: u32,
    done: BTreeSet<(u8, u32)>,
}

impl RotBlast {
    pub fn new(dome: (f32, f32), profile: RotBlastProfile, owner: Option<u32>, index: u8) -> Self {
        Self {
            dome,
            profile,
            owner,
            source: rot_source(index),
            done: BTreeSet::new(),
        }
    }

    pub fn dome(&self) -> (f32, f32) {
        self.dome
    }

    pub fn owner(&self) -> Option<u32> {
        self.owner
    }

    /// The victims already evaluated by this blast.
    pub fn evaluated(&self) -> usize {
        self.done.len()
    }

    fn key(kind: TargetKind, id: u32) -> (u8, u32) {
        let tag = match kind {
            TargetKind::Hero(_) => 0,
            TargetKind::Goblin => 1,
            TargetKind::Engineer => 2,
            TargetKind::Llama => 3,
            TargetKind::Slime => 4,
            TargetKind::Sticker => 5,
        };
        (tag, id)
    }

    /// Evaluate every victim once. The core band of the profile applies only
    /// with a positive core radius, and never at `sq == 0` with radius 0: the
    /// outer result applies (no point-blank kill). A victim already evaluated
    /// by this blast is skipped (one damage evaluation per victim per blast).
    pub fn evaluate(&mut self, targets: &[BlastTarget], solids: &[Rect]) -> Vec<BlastEffect> {
        let mut out = Vec::new();
        let radius = f32::from(self.profile.outer_radius);
        for target in targets {
            let key = Self::key(target.kind, target.id);
            if self.done.contains(&key) {
                continue;
            }
            if !circle_reaches(
                self.dome,
                radius,
                target.body,
                self.profile.line_of_sight,
                solids,
            ) {
                continue;
            }
            self.done.insert(key);
            match target.kind {
                // Another sticker: no provoke, no damage.
                TargetKind::Sticker => {}
                TargetKind::Hero(hero) => {
                    if self.profile.hero_damage.hurts_any_hero() {
                        out.push(BlastEffect::HeroHurt(HeroBlow {
                            hero,
                            source: self.source,
                            damage: self.profile.damage,
                            from_x: self.dome.0,
                            knock_vx: crate::slime::signed_away(
                                target.x,
                                self.dome.0,
                                f32::from(self.profile.hero_knock_x),
                            ),
                            knock_vy: f32::from(self.profile.hero_knock_y),
                        }));
                    }
                }
                kind => {
                    let reaction = match kind {
                        TargetKind::Goblin | TargetKind::Engineer => EnemyReaction::Launch {
                            vx: crate::slime::signed_away(
                                target.x,
                                self.dome.0,
                                f32::from(self.profile.launch_x),
                            ),
                            vy: f32::from(self.profile.launch_y),
                        },
                        TargetKind::Llama => EnemyReaction::Kick,
                        _ => EnemyReaction::None,
                    };
                    out.push(BlastEffect::EnemyHit {
                        id: target.id,
                        kind,
                        damage: self.profile.damage,
                        reaction,
                        credit: self.owner,
                    });
                }
            }
        }
        out
    }

    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(u64::from(self.dome.0.to_bits()));
        mix(u64::from(self.dome.1.to_bits()));
        mix(self.owner.map_or(0, |p| u64::from(p) + 1));
        mix(self.done.len() as u64);
        for (tag, id) in &self.done {
            mix(u64::from(*tag));
            mix(u64::from(*id));
        }
    }
}

#[cfg(test)]
mod tests;
