//! DR-A: the **presentation** of room 3's machine, the pure half
//! (`docs/01-design/enemy-gameplay-dozer.md` sections 4.1 and 14, DZ-AD13;
//! `docs/02-art/doz-01-dozer-spec.md`; the rig contract
//! `assets/source/enemies/dozer/dozer_rig_h120.json`). A function of the
//! machine, the heroes' columns and the tick through **shared references
//! only**, it returns what a presenter draws:
//!
//! - the DOZ-01 rig ([`SpriteDraw`]): hull body, tread, the engineer and riders
//!   A and B, the hull front (cab rail and rider tub), the spiked wheel and the
//!   yoke, each part's pivot on its rig anchor, in the rig's z order
//!   ([`rig_sprites`]);
//! - the **wheel and the tread by distance**: the tread shows `belt_N` with
//!   N = floor(distance) mod 9, the wheel one 7.5 degree phase per
//!   `r * pi / 24` px of travel, so a parked machine's wheel and tread hold
//!   still (DV-7) and the brake's 4 px slide turns them a little;
//! - the **rattle** (hull and riders 1 px up for 3 of every 6 ticks in the
//!   chase, 1 of every 3 in the strain, the riders 2 ticks late), the riders'
//!   idle cycle (B 8 ticks after A), their **yell** loop within 120 px of a hero
//!   and through the escape (the jeer), the brake's forward lurch and the
//!   strain's held lever ([`rider_frames`]);
//! - the **smoke**: stack and rear exhaust puffs and the kettle's steam bursts
//!   per state, world space after spawn, rising 0.5 px a tick, at most
//!   [`MAX_PUFFS`] alive ([`puffs`]);
//! - the **DOZ-02 damage states** (`docs/02-art/doz-02-dozer-damage-spec.md`, the
//!   contract `assets/source/enemies/dozer/dozer_damage_h120.json`): the jammed wheel, the
//!   bent yoke and the hull, cab and tub frames of `impact_stunned` (the nose-dip pitch for
//!   6 + 6 ticks, then level) and `disabled`, the riders' seats following the pitch, the
//!   riders' tumbles out of the tub (their GOB-01 poses) and the driver's slump, the steam
//!   leak and the knocked stack's emitters;
//! - the **room pieces** ([`room_pieces`]): the rear gate (closed, shaking in the rumble,
//!   burst once the machine is out) tiled up the lane and the drawn 16 px buffer stop;
//! - the **stop frame** ([`frame_sprites`]): the hanging frame with its pulley and haul
//!   line, the warning shake and floor shadow, the fall, the landed remnant (and its
//!   landing dust), the brace cleat on the back wall (intact, weakened, snapped and
//!   swinging) and the **rear glance** (the frame strikes the rear deck and tips off
//!   behind the exhaust). Every one is an atlas sprite: no code-drawn stand-in remains.
//!
//! - DR-D (INT DOZER-ROOM, audit-art WO 15) adds the room's **effects**
//!   ([`fx_sprites`]): the DOZ-01 metal hit-spark where a non-blast clang meets the
//!   hull, and the ENV-02 `timber_splinter_burst` on the brace cleat at each
//!   counted hit and at the release, and in the rear gate as the machine bursts
//!   out. Effects read this attempt's event log only (a retry never replays an
//!   earlier attempt's effect, nor its rear glance).
//!
//! **No box stand-in for the machine**: every machine pixel comes from the
//! DOZ-01 atlases. The rumble draws no machine (it is still behind the gate).
//!
//! **Presentation only.** Nothing here mutates a state, draws a random number
//! or is hashed, replayed or read back by the simulation: a run that calls
//! [`draw`] every tick has the same state hash as one that never does (a test).
//! Every output is a function of the machine's state and the tick, so a render
//! that skips frames or a replay seek draws the same picture for the same tick.
//!
//! Coordinates are world px, y up, like the simulation; a sprite is placed by
//! its bottom-left corner on whole pixels.

use crate::camera_lab::room::IRect;
use crate::combat::{CombatEvent, PropPhase};
use crate::combat_scene::dozer::DozerEnemy;
use crate::combat_scene::{CombatState, EnvRuntime};
use crate::dozer::{Dozer, DozerEvent, DozerState, EscapePhase};
use crate::encounter::Seat;
use crate::env_prop::MassState;
use crate::prototype::{HERO_HALF_WIDTH, TICK_SECONDS};

/// One DOZ-01 atlas: a single row of frames (`generated/art`, exported by
/// `scripts/export_assets.sh` from `assets/manifests/dozer_*.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sheet {
    Hull,
    Tread,
    Riders,
    FrontCab,
    FrontTub,
    Wheel,
    Yoke,
    Smoke,
    /// DR-D (INT DOZER-ROOM): the shared DOZ-01 metal hit-spark (rig part
    /// `hit_spark`, z 8).
    Spark,
    /// DR-D (INT DOZER-ROOM): the ENV-02 `timber_splinter_burst` piece.
    Splinter,
    /// DOZ-02: the damaged hull (`pitch_0`, `pitch_1`, `disabled`).
    HullDamage,
    FrontCabDamage,
    FrontTubDamage,
    WheelJammed,
    YokeBent,
    /// DOZ-02: the driver slumped (two breathing frames).
    Slump,
    /// DOZ-02: riders A (frames 0..=2) and B (3..=5) thrown out of the tub.
    Tumble,
    /// DOZ-02: the stop frame (`raised`, `pulley`, `warn_0`, `warn_1`, `falling`).
    StopFrame,
    /// DOZ-02: the floor shadow of the warning (four sizes).
    StopShadow,
    /// DOZ-02: the rear glance (the beam at 0, 15 and 35 degrees, its shadow).
    StopGlance,
    /// DOZ-02: the landed remnant (`landed`, `landed_hit`).
    StopRemnant,
    /// DOZ-02: the brace cleat (`intact`, `weakened`, `snapped`, `snapped_swing`).
    StopBrace,
    /// DOZ-02: the rear gate tile (`closed`, `shake_0`, `shake_1`, `burst`).
    Gate,
    /// DOZ-02: the drawn buffer stop.
    Buffer,
    /// DOZ-02: the haul line (`h`, `v`, `idler`).
    Haul,
}

impl Sheet {
    pub const ALL: [Self; 25] = [
        Self::Hull,
        Self::Tread,
        Self::Riders,
        Self::FrontCab,
        Self::FrontTub,
        Self::Wheel,
        Self::Yoke,
        Self::Smoke,
        Self::Spark,
        Self::Splinter,
        Self::HullDamage,
        Self::FrontCabDamage,
        Self::FrontTubDamage,
        Self::WheelJammed,
        Self::YokeBent,
        Self::Slump,
        Self::Tumble,
        Self::StopFrame,
        Self::StopShadow,
        Self::StopGlance,
        Self::StopRemnant,
        Self::StopBrace,
        Self::Gate,
        Self::Buffer,
        Self::Haul,
    ];

    /// The exported atlas (the manifest's `output_atlas`).
    pub const fn path(self) -> &'static str {
        match self {
            Self::Hull => "generated/art/dozer_hull_h120.png",
            Self::Tread => "generated/art/dozer_tread_h120.png",
            Self::Riders => "generated/art/dozer_riders_h36.png",
            Self::FrontCab => "generated/art/dozer_front_cab_h120.png",
            Self::FrontTub => "generated/art/dozer_front_tub_h120.png",
            Self::Wheel => "generated/art/dozer_wheel_h120.png",
            Self::Yoke => "generated/art/dozer_yoke_h120.png",
            Self::Smoke => "generated/art/dozer_smoke.png",
            Self::Spark => "generated/art/vfx_hit_spark_metal.png",
            Self::Splinter => "generated/art/floor01_splinter_burst.png",
            Self::HullDamage => "generated/art/dozer_hull_damage_h120.png",
            Self::FrontCabDamage => "generated/art/dozer_front_cab_damage_h120.png",
            Self::FrontTubDamage => "generated/art/dozer_front_tub_damage_h120.png",
            Self::WheelJammed => "generated/art/dozer_wheel_jammed_h120.png",
            Self::YokeBent => "generated/art/dozer_yoke_damage_h120.png",
            Self::Slump => "generated/art/dozer_engineer_slump_h120.png",
            Self::Tumble => "generated/art/dozer_rider_tumble_h120.png",
            Self::StopFrame => "generated/art/prop_stop_frame.png",
            Self::StopShadow => "generated/art/prop_stop_frame_shadow.png",
            Self::StopGlance => "generated/art/prop_stop_frame_glance.png",
            Self::StopRemnant => "generated/art/prop_stop_frame_remnant.png",
            Self::StopBrace => "generated/art/prop_stop_frame_brace.png",
            Self::Gate => "generated/art/env_dozer_gate.png",
            Self::Buffer => "generated/art/env_dozer_buffer.png",
            Self::Haul => "generated/art/env_dozer_haul.png",
        }
    }

    /// The manifest's `frame_size`.
    pub const fn frame_size(self) -> (u32, u32) {
        match self {
            Self::Hull => (175, 160),
            Self::Tread => (151, 34),
            Self::Riders => (64, 56),
            Self::FrontCab => (60, 21),
            Self::FrontTub => (65, 82),
            Self::Wheel => (99, 99),
            Self::Yoke => (64, 44),
            Self::Smoke => (24, 24),
            Self::Spark => (16, 16),
            Self::Splinter => (48, 48),
            Self::HullDamage => (175, 160),
            Self::FrontCabDamage => (60, 23),
            Self::FrontTubDamage => (65, 86),
            Self::WheelJammed => (99, 99),
            Self::YokeBent => (64, 44),
            Self::Slump | Self::Tumble => (64, 56),
            Self::StopFrame => (96, 60),
            Self::StopShadow => (80, 4),
            Self::StopGlance => (88, 88),
            Self::StopRemnant => (80, 16),
            Self::StopBrace => (16, 64),
            Self::Gate => (32, 32),
            Self::Buffer => (16, 24),
            Self::Haul => (12, 12),
        }
    }

    /// The manifest's frame count (one row).
    pub const fn frames(self) -> u32 {
        match self {
            Self::Hull
            | Self::FrontCab
            | Self::FrontTub
            | Self::Yoke
            | Self::WheelJammed
            | Self::YokeBent
            | Self::Buffer => 1,
            Self::HullDamage | Self::FrontCabDamage | Self::FrontTubDamage | Self::Haul => 3,
            Self::Slump | Self::StopRemnant => 2,
            Self::StopShadow | Self::StopGlance | Self::StopBrace | Self::Gate => 4,
            Self::StopFrame => 5,
            Self::Tumble => 6,
            Self::Tread => 9,
            Self::Riders => 19,
            Self::Wheel => 5,
            Self::Smoke => 11,
            Self::Spark => 3,
            Self::Splinter => 4,
        }
    }

    /// The manifest's pivot (image px, y down).
    pub const fn pivot(self) -> (i32, i32) {
        match self {
            Self::Hull => (87, 159),
            Self::Tread => (75, 33),
            Self::Riders => (28, 55),
            Self::FrontCab => (0, 20),
            Self::FrontTub => (0, 81),
            Self::Wheel => (49, 49),
            Self::Yoke => (54, 30),
            Self::Smoke => (12, 12),
            Self::Spark => (8, 8),
            Self::Splinter => (24, 24),
            Self::HullDamage => (87, 159),
            Self::FrontCabDamage => (0, 22),
            Self::FrontTubDamage => (0, 81),
            Self::WheelJammed => (49, 49),
            Self::YokeBent => (54, 30),
            Self::Slump => (28, 55),
            Self::Tumble => (32, 55),
            Self::StopFrame => (8, 59),
            Self::StopShadow => (0, 3),
            Self::Buffer => (0, 23),
            Self::StopRemnant => (0, 15),
            Self::Haul => (0, 11),
            Self::StopGlance => (44, 44),
            Self::StopBrace => (0, 63),
            Self::Gate => (0, 31),
        }
    }
}

/// A rig part (the rig's `parts`, without the smoke and the hit-spark).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Hull,
    Tread,
    Engineer,
    RiderA,
    RiderB,
    FrontCab,
    FrontTub,
    Wheel,
    Yoke,
}

impl Part {
    /// The rig's draw order (its `z`, back to front).
    pub const ALL: [Self; 9] = [
        Self::Hull,
        Self::Tread,
        Self::Engineer,
        Self::RiderA,
        Self::RiderB,
        Self::FrontCab,
        Self::FrontTub,
        Self::Wheel,
        Self::Yoke,
    ];

    pub const fn sheet(self) -> Sheet {
        match self {
            Self::Hull => Sheet::Hull,
            Self::Tread => Sheet::Tread,
            Self::Engineer | Self::RiderA | Self::RiderB => Sheet::Riders,
            Self::FrontCab => Sheet::FrontCab,
            Self::FrontTub => Sheet::FrontTub,
            Self::Wheel => Sheet::Wheel,
            Self::Yoke => Sheet::Yoke,
        }
    }

    /// The DOZ-02 replacement of this part in `impact_stunned` and `disabled` (the
    /// contract's `replaces`; the tread, the engineer and the riders keep theirs).
    pub const fn damage_sheet(self) -> Option<Sheet> {
        match self {
            Self::Hull => Some(Sheet::HullDamage),
            Self::FrontCab => Some(Sheet::FrontCabDamage),
            Self::FrontTub => Some(Sheet::FrontTubDamage),
            Self::Wheel => Some(Sheet::WheelJammed),
            Self::Yoke => Some(Sheet::YokeBent),
            Self::Tread | Self::Engineer | Self::RiderA | Self::RiderB => None,
        }
    }

    /// The rig anchor (px from the origin O, y up).
    pub const fn anchor(self) -> (i32, i32) {
        match self {
            Self::Hull | Self::Tread => (0, 0),
            Self::Engineer => (-48, 98),
            Self::RiderA => (38, 96),
            Self::RiderB => (66, 96),
            Self::FrontCab => (-75, 85),
            Self::FrontTub => (19, 23),
            Self::Wheel | Self::Yoke => (124, 49),
        }
    }

    /// The rig's `z` (smoke is 0).
    pub const fn z(self) -> u8 {
        match self {
            Self::Hull => 1,
            Self::Tread => 2,
            Self::Engineer | Self::RiderA => 3,
            Self::RiderB => 4,
            Self::FrontCab | Self::FrontTub => 5,
            Self::Wheel => 6,
            Self::Yoke => 7,
        }
    }

    /// The rattle moves the hull and the riders; the tread, the wheel and the
    /// yoke stay on the floor.
    const fn rattles(self) -> Rattles {
        match self {
            Self::Hull | Self::FrontCab | Self::FrontTub => Rattles::Hull,
            Self::Engineer | Self::RiderA | Self::RiderB => Rattles::Rider,
            Self::Tread | Self::Wheel | Self::Yoke => Rattles::No,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rattles {
    Hull,
    Rider,
    No,
}

/// The `dozer_riders_h36` frames (the manifest's tags).
pub mod rider {
    pub const ENGINEER_POINT: u8 = 0;
    pub const ENGINEER_HOLD: u8 = 2;
    pub const ENGINEER_IDLE_1: u8 = 3;
    pub const ENGINEER_LEVER_PULL: u8 = 4;
    pub const ENGINEER_LURCH_FORWARD: u8 = 6;
    pub const A_HOLD: u8 = 7;
    pub const A_IDLE_1: u8 = 8;
    pub const A_BRANDISH: u8 = 9;
    pub const A_YELL_1: u8 = 10;
    pub const A_LURCH_FORWARD: u8 = 12;
    pub const B_HOLD: u8 = 13;
    pub const B_IDLE_1: u8 = 14;
    pub const B_BRANDISH: u8 = 15;
    pub const B_YELL_1: u8 = 16;
    pub const B_LURCH_FORWARD: u8 = 18;
}

/// The `dozer_smoke` frames: black 0..=5, steam 6..=10.
pub const SMOKE_BLACK_FIRST: u8 = 0;
pub const SMOKE_STEAM_FIRST: u8 = 6;

/// Rig timing (`timing` of the rig file, tick_hz 60).
pub const IDLE_TICKS: u64 = 16;
pub const RIDER_B_IDLE_OFFSET: u64 = 8;
pub const YELL_TICKS: u64 = 6;
/// The riders yell when a hero's body is within this many px of the wheel
/// front (DOZ-01 section 5, "when Carl is within 120 px").
pub const YELL_RANGE_PX: f32 = 120.0;
pub const LURCH_FORWARD_TICKS: u16 = 10;
pub const RIDER_LAG_TICKS: u64 = 2;
/// The cruise rattle and the strain rattle: `(period, up)`.
pub const CRUISE_RATTLE: (u64, u64) = (6, 3);
pub const STRAIN_RATTLE: (u64, u64) = (3, 1);
/// Smoke: black puffs live 24 ticks (6 frames of 4), steam 15 (5 frames of 3).
pub const BLACK_LIFE: u64 = 24;
pub const BLACK_FRAME_TICKS: u64 = 4;
pub const STEAM_LIFE: u64 = 15;
pub const STEAM_FRAME_TICKS: u64 = 3;
pub const RISE_PX_PER_TICK: f32 = 0.5;
pub const MAX_PUFFS: usize = 8;
/// A steam burst: 3 puffs, one every 6 ticks.
pub const STEAM_BURST: u64 = 3;
pub const STEAM_EVERY: u64 = 6;
/// The strain repeats its burst every 30 ticks.
pub const STRAIN_STEAM_REPEAT: u16 = 30;
/// DRA-AD3: a parked machine (`escaped`, idle) puffs from its stack every 24
/// ticks (the rig has no idle row; half the cruise rate).
pub const IDLE_STACK_EVERY: u64 = 24;
/// The rumble: puffs over the gate every 12 ticks, and one kettle burst.
pub const RUMBLE_PUFF_EVERY: u64 = 12;
/// Wheel phases (7.5 degrees each) and tread belts.
pub const WHEEL_PHASES: u8 = 4;
pub const TREAD_BELTS: u8 = 9;

/// Emitters (rig px from O, y up).
pub const STACK: (i32, i32) = (15, 156);
pub const REAR_EXHAUST: (i32, i32) = (-82, 124);
pub const KETTLE: (i32, i32) = (5, 129);

/// The room pieces (room px, DZ-Q22): the rear gate (x 0..32, the lane's full
/// height) and the drawn buffer stop (x 1008..1024).
pub const GATE_X: (i32, i32) = (0, 32);
pub const BUFFER_X: (i32, i32) = (1008, 1024);
/// DRA-AD4: the buffer stop's drawn height (not data: it is no solid).
pub const BUFFER_H_PX: i32 = 24;
/// The lane's ceiling above the walking line (`ceiling_px`).
pub const LANE_H_PX: i32 = 228;
/// DOZ-02: the gate shakes `shake_0` / `shake_1` alternately, this many ticks each.
pub const GATE_SHAKE_TICKS: u64 = 3;
/// The gate tile's side: eight of them stack to the lane ceiling (the last is cropped).
pub const GATE_TILE_PX: i32 = 32;

/// DR-D (INT DOZER-ROOM): the hit-spark plays star, burst, fade, 2 ticks each
/// (the rig's `timing`).
pub const SPARK_FRAME_TICKS: u64 = 2;
/// The `timber_splinter_burst`: 4 frames of 67 ms, 4 ticks each at 60 Hz.
pub const SPLINTER_FRAME_TICKS: u64 = 4;
/// DRD-AD1: the spark's height above the walking line (a hero's chest, where
/// a jab meets the hull).
pub const SPARK_CHEST_PX: i32 = 24;
/// DRD-AD2: the gate bursts as the machine comes out: two splinter bursts on
/// the gate's centre line at these heights, the second
/// [`GATE_BURST_STAGGER`] ticks after the first.
pub const GATE_BURST_Y: [i32; 2] = [40, 88];
pub const GATE_BURST_STAGGER: u64 = 4;
/// The rig's `z` of the effects (the `hit_spark` row): over every part.
pub const FX_Z: u8 = 8;
/// The most effect sprites [`draw`] returns (the presenter's pool).
pub const MAX_FX: usize = 4;

/// The draw layers of the room pieces (the rig's `z` is 0..=7, the effects 8):
/// behind every actor, and in front of every actor.
pub const Z_BACK: u8 = 100;
pub const Z_FRONT: u8 = 101;
/// DOZ-02: a rider in the air or on the floor after the tumble draws over every rig part (the
/// wheel and the yoke, z 7).
pub const Z_TUMBLE: u8 = 9;

/// One atlas sprite: its sheet and frame, its bottom-left corner (world px,
/// y up) and its draw order (the rig's z, or [`Z_BACK`] / [`Z_FRONT`] for the room pieces).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteDraw {
    pub sheet: Sheet,
    pub frame: u8,
    pub left: i32,
    pub bottom: i32,
    pub z: u8,
    /// Opacity, 0..=255 (the floor shadow of the warning fades in; everything else is 255).
    pub alpha: u8,
    /// Rows cut from the top of the frame (the gate's last tile ends at the lane ceiling).
    pub crop_top: u8,
}

impl SpriteDraw {
    /// An opaque, uncropped sprite whose bottom-left corner is `(left, bottom)`.
    pub const fn new(sheet: Sheet, frame: u8, left: i32, bottom: i32, z: u8) -> Self {
        Self {
            sheet,
            frame,
            left,
            bottom,
            z,
            alpha: 255,
            crop_top: 0,
        }
    }

    /// A sprite with its pivot pixel's bottom-left corner on `(x, y)` (world px, y up).
    pub fn at_pivot(sheet: Sheet, frame: u8, (x, y): (i32, i32), z: u8) -> Self {
        let (_, h) = sheet.frame_size();
        let (px, py) = sheet.pivot();
        Self::new(sheet, frame, x - px, y - (h as i32 - 1 - py), z)
    }

    #[must_use]
    pub const fn with_alpha(mut self, alpha: u8) -> Self {
        self.alpha = alpha;
        self
    }

    /// The sprite's visible size: the frame less the cropped rows.
    pub fn visible(&self) -> (u32, u32) {
        let (w, h) = self.sheet.frame_size();
        (w, h - u32::from(self.crop_top))
    }

    /// The sprite's visible centre (world px, y up).
    pub fn centre(&self) -> (f32, f32) {
        let (w, h) = self.visible();
        (
            self.left as f32 + w as f32 / 2.0,
            self.bottom as f32 + h as f32 / 2.0,
        )
    }
}

/// The room's stop frame: prop 0 of the sidecar (`f01_dozer::STOP_FRAME_PROP`).
pub const FRAME_PROP: usize = crate::f01_dozer::STOP_FRAME_PROP;
/// The warning shake: `warn_0` / `warn_1`, a step every this many ticks.
pub const FRAME_SHAKE_TICKS: u64 = 3;
/// The rear glance plays these ticks from the landing: `glance_0` 4, `glance_1` 4, `glance_2` 6
/// (the contract's `stop_frame.glance`); the remnant then drops in behind the machine.
pub const GLANCE_TICKS: u64 = 14;
pub const GLANCE_FRAME_TICKS: [u64; 3] = [4, 4, 6];
/// The glance's offsets (x forward, y up) from the strike position per frame, and the strike
/// position's inset from the rear of the push box (the beam's left end).
pub const GLANCE_OFFSETS: [(i32, i32); 3] = [(0, 0), (-6, 0), (-16, -8)];
pub const GLANCE_INSET_PX: i32 = 8;
/// The rear deck (the cab roof) is this far under the push box's top (the boiler dome, 120 px).
pub const GLANCE_DECK_DROP_PX: i32 = 12;
/// The glance atlas' beam centre sits 9 px above the beam's bottom row (the `glance_0` key is
/// drawn 2 px low): the pivot's height above the deck.
pub const GLANCE_PIVOT_ABOVE_DECK: i32 = 9;
/// The glance shadow's last row sits on the deck; the pivot is 43 px above it.
pub const GLANCE_SHADOW_PIVOT_ABOVE_DECK: i32 = 43;
pub const GLANCE_SHADOW_ALPHA: u8 = 115;

/// The warning's floor shadow: four sizes over the warning, alpha 0.3 to 0.6.
pub const SHADOW_ALPHA: (u8, u8) = (77, 153);
/// The brace chain swings (`snapped` / `snapped_swing`) 8 ticks each for 48 ticks after the release.
pub const BRACE_SWING_TICKS: u64 = 8;
pub const BRACE_SWING_FOR: u64 = 48;
/// Landing dust: two steam puffs, 6 ticks apart, at each beam end, 3 frames of 3 ticks.
pub const DUST_EVERY: u64 = 6;
pub const DUST_LIFE: u64 = 9;
/// The haul line (the pulley frame's chain row 4, the idler over the brace, `env_dozer_haul`): the first
/// run tile's bottom is this far above the hanging mass' bottom, it starts this far right of the hanging mass'
/// left edge, and the brace's chain column is at this x offset on its sprite.
pub const HAUL_BOTTOM_ABOVE_MASS: i32 = 49;
pub const HAUL_START_DX: i32 = 88;
pub const BRACE_CHAIN_COL: i32 = 8;
pub const HAUL_RUN_PX: i32 = 8;

/// The impact schedule (the contract's `timing.impact_stunned.hull`): `pitch_0` for 6 ticks, `pitch_1`
/// for 6, then `disabled`.
pub const PITCH_TICKS: [u64; 2] = [6, 6];
/// The riders' seat offsets (y up) of `pitch_0`, `pitch_1` and `disabled`: engineer, rider A, rider B
/// (the contract's `pitch.rider_seat_dy`).
pub const SEAT_DY: [[i32; 3]; 3] = [[1, -2, -3], [0, -1, -2], [0, 0, 0]];
/// The riders lurch for the first ticks of the stun, then they tumble out (rider B 4 ticks after A)
/// and the driver slumps. The tumble's airborne ticks, the arc's height, and the floor stay-put.
pub const LURCH_TICKS: u64 = 12;
pub const TUMBLE_START: [u64; 2] = [12, 16];
pub const TUMBLE_AIR_TICKS: u64 = 24;
pub const TUMBLE_ARC_UP_PX: f32 = 12.0;
pub const TUMBLE_POSE_TICKS: u64 = 6;
/// The rider's landing column relative to the wheel front (the roster's `dzq20_land_dx`): A, B.
pub const TUMBLE_LAND_DX: [i32; 2] = [32, 72];
pub const SLUMP_FRAME_TICKS: u64 = 30;
/// The impact's hit-spark: on the tub lip's crumple, rig px from O (y up).
pub const IMPACT_SPARK: (i32, i32) = (66, 106);
/// The damage emitters (rig px from O, y up): the boiler crack's steam leak, the knocked stack's mouth.
pub const STEAM_LEAK: (i32, i32) = (6, 91);
pub const STACK_CROOKED: (i32, i32) = (8, 144);
/// `disabled`: a steam puff every 40 ticks and a black wisp (`black_3..black_5`) every 60.
pub const DISABLED_STEAM_EVERY: u64 = 40;
pub const DISABLED_WISP_EVERY: u64 = 60;
/// The impact's steam: a puff every 6 ticks over the stun.
pub const LEAK_EVERY: u64 = 6;
/// The wisp: the last three black frames, 4 ticks each.
pub const WISP_FIRST: u8 = 3;
pub const WISP_LIFE: u64 = 12;

/// The most atlas sprites of the room pieces and the stop frame [`draw`] returns (the presenter's pool).
pub const MAX_PIECES: usize = 56;
/// The most atlas sprites [`draw`] returns in all.
pub const MAX_SPRITES: usize = Part::ALL.len() + MAX_PUFFS + MAX_FX + MAX_PIECES;

/// One frame of the room's machine presentation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DozerArt {
    /// Rig parts, smoke, effects and room pieces: back to front by `z`, then by position.
    pub sprites: Vec<SpriteDraw>,
    /// The machine's state name (for the capture logs).
    pub state: &'static str,
}

/// The room's machine (its only slot enemy), if any.
pub fn dozer_of(combat: &CombatState) -> Option<&Dozer> {
    combat
        .slot_enemies()
        .iter()
        .find_map(|e| e.as_any().and_then(|a| a.downcast_ref::<DozerEnemy>()))
        .map(DozerEnemy::dozer)
}

/// The machine's origin at emergence (wheel front at the gate's right edge).
fn emerge_o(d: &Dozer) -> f32 {
    d.site.emerge_front_x as f32 - d.tuning.wheel_front()
}

/// The distance travelled since emergence (px, never negative).
pub fn travelled(d: &Dozer) -> f32 {
    (d.o - emerge_o(d)).max(0.0)
}

/// The tread frame: `belt_N`, N = floor(distance) mod 9.
pub fn tread_frame(distance: f32) -> u8 {
    (distance.floor() as i64).rem_euclid(i64::from(TREAD_BELTS)) as u8
}

/// The wheel frame: one 7.5 degree phase per `r * pi / 24` px rolled (the
/// smear is the opening chase's charge only).
pub fn wheel_frame(distance: f32, wheel_r: f32) -> u8 {
    let arc = wheel_r * std::f32::consts::PI / 24.0;
    ((distance / arc).floor() as i64).rem_euclid(i64::from(WHEEL_PHASES)) as u8
}

/// `true` on the ticks a rattle with `(period, up)` lifts by 1 px.
fn lifted(tick: u64, (period, up): (u64, u64)) -> bool {
    tick % period < up
}

/// The rattle of the machine's state: `None` while it does not rattle.
fn rattle_of(state: DozerState) -> Option<(u64, u64)> {
    match state {
        DozerState::CommittedChase => Some(CRUISE_RATTLE),
        DozerState::Escaped(EscapePhase::Strain { .. }) => Some(STRAIN_RATTLE),
        _ => None,
    }
}

/// The two frames of a loop of `ticks` each.
fn alternate(tick: u64, ticks: u64, a: u8, b: u8) -> u8 {
    if (tick / ticks).is_multiple_of(2) {
        a
    } else {
        b
    }
}

/// The ticks since the impact of a stunned machine (0 on the first tick of the stun), the whole stun for
/// a `disabled` one, `None` otherwise: the clock of the DOZ-02 damage states.
pub fn stun_age(d: &Dozer) -> Option<u64> {
    let total = d.tuning.dzq15_stunned_ticks;
    match d.state {
        DozerState::ImpactStunned { ticks_left } => {
            Some(u64::from(total.saturating_sub(ticks_left)))
        }
        DozerState::Disabled => Some(u64::from(total)),
        _ => None,
    }
}

/// The damaged hull, cab and tub frame of a stun age: `pitch_0`, `pitch_1`, then `disabled`.
pub fn pitch_frame(age: u64) -> u8 {
    if age < PITCH_TICKS[0] {
        0
    } else if age < PITCH_TICKS[0] + PITCH_TICKS[1] {
        1
    } else {
        2
    }
}

/// The engineer's, rider A's and rider B's frames of the `riders` atlas (a tumbling rider and the slumped
/// driver draw from their own atlases instead; see [`rig_sprites`]).
pub fn rider_frames(state: DozerState, tick: u64, hero_near: bool) -> [u8; 3] {
    use rider::*;
    let idle = [
        alternate(tick, IDLE_TICKS, ENGINEER_HOLD, ENGINEER_IDLE_1),
        alternate(tick, IDLE_TICKS, A_HOLD, A_IDLE_1),
        alternate(tick + RIDER_B_IDLE_OFFSET, IDLE_TICKS, B_HOLD, B_IDLE_1),
    ];
    let yell = [
        alternate(tick, YELL_TICKS, A_BRANDISH, A_YELL_1),
        alternate(tick, YELL_TICKS, B_BRANDISH, B_YELL_1),
    ];
    match state {
        DozerState::CommittedChase if hero_near => [ENGINEER_POINT, yell[0], yell[1]],
        DozerState::Escaped(EscapePhase::Brake { k }) if k < LURCH_FORWARD_TICKS => {
            [ENGINEER_LURCH_FORWARD, A_LURCH_FORWARD, B_LURCH_FORWARD]
        }
        // The design's escape: the riders jeer (yell loop); the engineer holds the lever
        // through the strain.
        DozerState::Escaped(EscapePhase::Strain { .. }) => [ENGINEER_LEVER_PULL, yell[0], yell[1]],
        DozerState::Escaped(_) => [idle[0], yell[0], yell[1]],
        // DOZ-02: the struck machine throws its crew forward (the lurch keys, on the pitched
        // seats) until they tumble out.
        DozerState::ImpactStunned { .. } | DozerState::Disabled => {
            [ENGINEER_LURCH_FORWARD, A_LURCH_FORWARD, B_LURCH_FORWARD]
        }
        _ => idle,
    }
}

/// A thrown rider (0: A, 1: B) at `age` ticks into the stun, once the tumble has started: three poses
/// of the `tumble` atlas on a ballistic arc from the seat to the landing the simulation gives the
/// dismounted goblin (the wheel front plus the roster's `dzq20_land_dx`), up [`TUMBLE_ARC_UP_PX`]
/// at the top of the throw and lying on the floor from the end of [`TUMBLE_AIR_TICKS`] until the
/// goblin takes over on the disable tick (DZ2-Q1).
pub fn tumble_sprite(d: &Dozer, rider: usize, age: u64, seat: (i32, i32)) -> Option<SpriteDraw> {
    let t = age.checked_sub(TUMBLE_START[rider])?;
    let floor = d.floor_y.round();
    let (x0, y0) = (seat.0 as f32, seat.1 as f32);
    let x1 = d.wheel_front() + TUMBLE_LAND_DX[rider] as f32;
    let p = t.min(TUMBLE_AIR_TICKS) as f32 / TUMBLE_AIR_TICKS as f32;
    let x = x0 + (x1 - x0) * p;
    let y =
        floor + (y0 - floor) * (1.0 - p * p) + TUMBLE_ARC_UP_PX * (std::f32::consts::PI * p).sin();
    let pose = if t < TUMBLE_POSE_TICKS {
        0
    } else if t < TUMBLE_AIR_TICKS {
        1
    } else {
        2
    };
    let frame = pose + 3 * rider as u8;
    Some(SpriteDraw::at_pivot(
        Sheet::Tumble,
        frame,
        (x.round() as i32, y.round() as i32),
        Z_TUMBLE,
    ))
}

/// The rig's atlas sprites at origin `o` on the walking line `floor_y`.
/// `seated`: whether riders A and B are aboard (DR-C: a seat whose rider
/// has dismounted, or stayed dead, is not drawn). A struck machine (DOZ-02) draws the damaged hull,
/// cab, tub, wheel and yoke; the pitch moves the seats; the riders tumble out and the driver slumps.
pub fn rig_sprites(d: &Dozer, tick: u64, hero_near: bool, seated: [bool; 2]) -> Vec<SpriteDraw> {
    let o = d.o.round() as i32;
    let floor = d.floor_y.round() as i32;
    let distance = travelled(d);
    let rattle = rattle_of(d.state);
    let hull_up = i32::from(rattle.is_some_and(|r| lifted(tick, r)));
    let rider_up = i32::from(
        rattle.is_some_and(|r| tick >= RIDER_LAG_TICKS && lifted(tick - RIDER_LAG_TICKS, r)),
    );
    let riders = rider_frames(d.state, tick, hero_near);
    let age = stun_age(d);
    let pitch = age.map_or(0, pitch_frame);
    let mut out = Vec::with_capacity(Part::ALL.len());
    for part in Part::ALL {
        let seat_index = match part {
            Part::Engineer => Some(0),
            Part::RiderA => Some(1),
            Part::RiderB => Some(2),
            _ => None,
        };
        if matches!(part, Part::RiderA | Part::RiderB) && !seated[seat_index.unwrap_or(1) - 1] {
            continue;
        }
        let (ax, ay) = part.anchor();
        let seat_dy = match (age, seat_index) {
            (Some(_), Some(i)) => SEAT_DY[usize::from(pitch)][i],
            _ => 0,
        };
        if let (Some(a), Some(i @ 1..=2)) = (age, seat_index)
            && let Some(s) = tumble_sprite(d, i - 1, a, (o + ax, floor + ay + seat_dy))
        {
            out.push(s);
            continue;
        }
        let damage = age.and_then(|_| part.damage_sheet());
        let (sheet, frame) = match (age, part, damage) {
            (Some(a), Part::Engineer, _) if a >= LURCH_TICKS => {
                (Sheet::Slump, alternate(tick, SLUMP_FRAME_TICKS, 0, 1))
            }
            (_, Part::Hull | Part::FrontCab | Part::FrontTub, Some(sheet)) => (sheet, pitch),
            // A jammed wheel and a bent yoke hold still (DV-7).
            (_, _, Some(sheet)) => (sheet, 0),
            _ => (
                part.sheet(),
                match part {
                    Part::Tread => tread_frame(distance),
                    Part::Wheel => wheel_frame(distance, d.wheel_r()),
                    Part::Engineer => riders[0],
                    Part::RiderA => riders[1],
                    Part::RiderB => riders[2],
                    _ => 0,
                },
            ),
        };
        let lift = match part.rattles() {
            Rattles::Hull => hull_up,
            Rattles::Rider => rider_up,
            Rattles::No => 0,
        };
        // The pivot pixel's bottom-left corner sits on the anchor.
        out.push(SpriteDraw::at_pivot(
            sheet,
            frame,
            (o + ax, floor + ay + lift + seat_dy),
            part.z(),
        ));
    }
    out
}

/// One puff source: its emitter (world px) at the tick of its birth is the
/// current emitter moved back along the travel by `speed * age`.
struct Source {
    emitter: (i32, i32),
    steam: bool,
}

/// A smoke sprite `age` ticks old: `frame` of the smoke atlas, centred on the emitter moved back by
/// `back` and risen [`RISE_PX_PER_TICK`] a tick.
fn smoke_at(frame: u8, emitter: (i32, i32), age: u64, back: f32, z: u8) -> SpriteDraw {
    let (w, h) = Sheet::Smoke.frame_size();
    let x = emitter.0 as f32 - back;
    let y = emitter.1 as f32 + RISE_PX_PER_TICK * age as f32;
    SpriteDraw::new(
        Sheet::Smoke,
        frame,
        (x - w as f32 / 2.0).round() as i32,
        (y - h as f32 / 2.0).round() as i32,
        z,
    )
}

fn puff(source: &Source, age: u64, back: f32) -> SpriteDraw {
    let (life_frames, first, frame_ticks) = if source.steam {
        (5, SMOKE_STEAM_FIRST, STEAM_FRAME_TICKS)
    } else {
        (6, SMOKE_BLACK_FIRST, BLACK_FRAME_TICKS)
    };
    let frame = first + ((age / frame_ticks) as u8).min(life_frames - 1);
    smoke_at(frame, source.emitter, age, back, 0)
}

/// The thin steam of a disabled machine: `steam_0..steam_2` only (the contract's `disabled` rate).
fn thin_steam(emitter: (i32, i32), age: u64) -> SpriteDraw {
    let frame = SMOKE_STEAM_FIRST + ((age / STEAM_FRAME_TICKS) as u8).min(2);
    smoke_at(frame, emitter, age, 0.0, 0)
}

/// The stack's wisp of a disabled machine: `black_3..black_5` only.
fn wisp(emitter: (i32, i32), age: u64) -> SpriteDraw {
    let frame = SMOKE_BLACK_FIRST + WISP_FIRST + ((age / BLACK_FRAME_TICKS) as u8).min(2);
    smoke_at(frame, emitter, age, 0.0, 0)
}

/// The puffs alive now of an emitter that fires every `every` ticks of the
/// world clock.
fn periodic(source: &Source, tick: u64, every: u64, speed_tick: f32, out: &mut Vec<SpriteDraw>) {
    let life = if source.steam { STEAM_LIFE } else { BLACK_LIFE };
    let mut age = tick % every;
    while age < life && age <= tick {
        out.push(puff(source, age, speed_tick * age as f32));
        age += every;
    }
}

/// The steam puffs alive now of a burst that began `since` ticks ago (3 puffs,
/// one every 6 ticks).
fn burst(source: &Source, since: u64, out: &mut Vec<SpriteDraw>) {
    for n in 0..STEAM_BURST {
        let born = n * STEAM_EVERY;
        if since >= born && since - born < STEAM_LIFE {
            out.push(puff(source, since - born, 0.0));
        }
    }
}

/// The smoke and steam of the machine's state, at most [`MAX_PUFFS`].
pub fn puffs(d: &Dozer, tick: u64) -> Vec<SpriteDraw> {
    let o = d.o.round() as i32;
    let floor = d.floor_y.round() as i32;
    let at = |(x, y): (i32, i32), steam| Source {
        emitter: (o + x, floor + y),
        steam,
    };
    let speed_tick = (d.speed * TICK_SECONDS).max(0.0);
    let mut out = Vec::new();
    match d.state {
        DozerState::Dormant { .. } => {}
        DozerState::Approaching { ticks_left } => {
            // Behind the gate: puffs rise over it, and one kettle burst.
            let rumble = u64::from(d.tuning.dzq12_rumble_ticks.saturating_sub(ticks_left));
            let gate = |y: i32, steam| Source {
                emitter: ((GATE_X.0 + GATE_X.1) / 2, floor + y),
                steam,
            };
            burst(&gate(KETTLE.1, true), rumble, &mut out);
            periodic(
                &gate(STACK.1, false),
                rumble,
                RUMBLE_PUFF_EVERY,
                0.0,
                &mut out,
            );
        }
        DozerState::CommittedChase => {
            periodic(&at(STACK, false), tick, 12, speed_tick, &mut out);
            periodic(&at(REAR_EXHAUST, false), tick, 20, speed_tick, &mut out);
        }
        DozerState::Escaped(EscapePhase::Brake { k }) => {
            burst(&at(KETTLE, true), u64::from(k), &mut out);
            periodic(&at(STACK, false), tick, 8, 0.0, &mut out);
            periodic(&at(REAR_EXHAUST, false), tick, 20, 0.0, &mut out);
        }
        DozerState::Escaped(EscapePhase::Strain { ticks_left }) => {
            let age = d.tuning.dzq14_strain_ticks.saturating_sub(ticks_left);
            burst(
                &at(KETTLE, true),
                u64::from(age % STRAIN_STEAM_REPEAT),
                &mut out,
            );
            periodic(&at(STACK, false), tick, 8, 0.0, &mut out);
            periodic(&at(REAR_EXHAUST, false), tick, 8, 0.0, &mut out);
        }
        DozerState::Escaped(EscapePhase::Idle) => {
            periodic(&at(STACK, false), tick, IDLE_STACK_EVERY, 0.0, &mut out);
        }
        // DOZ-02: the boiler crack leaks a steam puff every 6 ticks over the stun and the knocked
        // stack coughs one black puff on the first tick.
        DozerState::ImpactStunned { .. } => {
            let age = stun_age(d).unwrap_or(0);
            let leak = at(STEAM_LEAK, true);
            for n in 0..d.tuning.dzq15_stunned_ticks as u64 / LEAK_EVERY {
                let born = n * LEAK_EVERY;
                if age >= born && age - born < STEAM_LIFE {
                    out.push(puff(&leak, age - born, 0.0));
                }
            }
            if age < BLACK_LIFE {
                out.push(puff(&at(STACK_CROOKED, false), age, 0.0));
            }
        }
        // DOZ-02: a disabled machine is dead, not lethal: one thin puff of steam every 40 ticks and
        // one black wisp every 60.
        DozerState::Disabled => {
            let (steam, stack) = (at(STEAM_LEAK, true), at(STACK_CROOKED, false));
            let age = tick % DISABLED_STEAM_EVERY;
            if age < STEAM_LIFE.min(3 * STEAM_FRAME_TICKS) {
                out.push(thin_steam(steam.emitter, age));
            }
            let age = tick % DISABLED_WISP_EVERY;
            if age < WISP_LIFE {
                out.push(wisp(stack.emitter, age));
            }
        }
    }
    out.truncate(MAX_PUFFS);
    out
}

/// The rear gate and the buffer stop (DOZ-02 atlases): the gate is `GATE_TILE_PX` tiles stacked from the
/// walking line to the lane ceiling (the last one cropped), closed until the rumble, shaking through it
/// and burst from the emergence on; the buffer stop is drawn only (DZ-AD4).
pub fn room_pieces(d: &Dozer, tick: u64, out: &mut Vec<SpriteDraw>) {
    let floor = d.floor_y.round() as i32;
    let frame = match d.state {
        DozerState::Approaching { .. } => 1 + ((tick / GATE_SHAKE_TICKS) % 2) as u8,
        state if state.emerged() => 3,
        _ => 0,
    };
    let mut bottom = floor;
    while bottom < floor + LANE_H_PX {
        let mut tile = SpriteDraw::new(Sheet::Gate, frame, GATE_X.0, bottom, Z_BACK);
        let over = bottom + GATE_TILE_PX - (floor + LANE_H_PX);
        if over > 0 {
            tile.crop_top = over as u8;
        }
        out.push(tile);
        bottom += GATE_TILE_PX;
    }
    out.push(SpriteDraw::at_pivot(
        Sheet::Buffer,
        0,
        (BUFFER_X.0, floor),
        Z_FRONT,
    ));
}

/// The tick the stop frame was released in this attempt (the brace's release event), if the log still
/// holds it.
fn release_tick(combat: &CombatState) -> Option<u64> {
    attempt_events(combat).find_map(|e| match *e {
        CombatEvent::PropPhase {
            tick,
            prop,
            phase: PropPhase::Released,
            ..
        } if usize::from(prop) == FRAME_PROP => Some(tick),
        _ => None,
    })
}

/// The brace's frame: `intact`, `weakened`, then `snapped` from the release, swinging to
/// `snapped_swing` and back every [`BRACE_SWING_TICKS`] for the first [`BRACE_SWING_FOR`] ticks.
fn brace_frame(state: MassState, release: Option<u64>, tick: u64) -> u8 {
    match state {
        MassState::Supported => 0,
        MassState::Weakened => 1,
        _ => {
            let since = release.map_or(BRACE_SWING_FOR, |r| tick.saturating_sub(r));
            if since < BRACE_SWING_FOR && (since / BRACE_SWING_TICKS) % 2 == 1 {
                3
            } else {
                2
            }
        }
    }
}

/// The haul line (DZ2-AD6): the chain from the pulley's stub along the ceiling to the idler over the
/// brace, then down to the brace's horn. `hang` and `target` are layout rects mapped to world px by
/// `bottom`.
fn haul_sprites(
    hang: IRect,
    hang_bottom: i32,
    target: IRect,
    target_bottom: i32,
    out: &mut Vec<SpriteDraw>,
) {
    let row_bottom = hang_bottom + HAUL_BOTTOM_ABOVE_MASS;
    let chain_x = target.x + BRACE_CHAIN_COL;
    let idler_left = chain_x - 6;
    let mut x = hang.x + HAUL_START_DX;
    while x < idler_left {
        out.push(SpriteDraw::new(Sheet::Haul, 0, x, row_bottom, Z_FRONT));
        x += HAUL_RUN_PX;
    }
    out.push(SpriteDraw::new(
        Sheet::Haul,
        2,
        idler_left,
        row_bottom,
        Z_FRONT,
    ));
    // The vertical run: the content of a tile is its top 8 rows, five columns in from the left.
    let brace_top = target_bottom + Sheet::StopBrace.frame_size().1 as i32;
    let mut top = row_bottom;
    while top > brace_top {
        out.push(SpriteDraw::new(
            Sheet::Haul,
            1,
            chain_x - 5,
            top - Sheet::Haul.frame_size().1 as i32,
            Z_FRONT,
        ));
        top -= HAUL_RUN_PX;
    }
}

/// The stop frame and its brace (DOZ-02 atlases), read from the env runtime's falling mass: the hanging
/// frame with its pulley and the haul line, the warning shake and its floor shadow, the fall, the landed
/// remnant with its landing dust, the brace cleat on the back wall (intact, weakened, snapped and
/// swinging) and the rear glance for [`GLANCE_TICKS`] after a [`DozerEvent::Glance`] (`glance`: its tick
/// and wheel front). Layout rects (y down) map to world px through the footprint, which rests on the
/// machine's walking line.
pub fn frame_sprites(
    d: &Dozer,
    combat: &CombatState,
    env: &EnvRuntime,
    glance: Option<(u64, i32)>,
    tick: u64,
    out: &mut Vec<SpriteDraw>,
) {
    let Some(mass) = env.mass(FRAME_PROP) else {
        return;
    };
    let floor = d.floor_y.round() as i32;
    let cfg = mass.config();
    let footprint = cfg.footprint;
    // World bottom of a layout rect.
    let bottom = |r: IRect| floor + footprint.bottom() - r.bottom();
    let state = mass.state();
    let release = release_tick(combat);
    let glancing = glance.filter(|(at, _)| (*at..*at + GLANCE_TICKS).contains(&tick));
    let hit = matches!(
        d.state,
        DozerState::ImpactStunned { .. } | DozerState::Disabled
    );
    // The brace cleat on the back wall (the prop's hit target) and the haul line to it.
    if let Some(b) = env.prop_target(FRAME_PROP) {
        out.push(SpriteDraw::at_pivot(
            Sheet::StopBrace,
            brace_frame(state, release, tick),
            (b.x, bottom(b)),
            Z_BACK,
        ));
        haul_sprites(cfg.hanging, bottom(cfg.hanging), b, bottom(b), out);
    }
    // The floor shadow of the warning, growing over it (the last size from the fall on).
    if mass.telegraph_rect().is_some() {
        let warning = u64::from(cfg.warning_ticks).max(1);
        let age = match (state, release) {
            (MassState::Released, Some(r)) => tick.saturating_sub(r).min(warning),
            _ => warning,
        };
        let size = ((age * 4) / warning).min(3) as u8;
        let (lo, hi) = (u64::from(SHADOW_ALPHA.0), u64::from(SHADOW_ALPHA.1));
        let alpha = lo + (hi - lo) * age / warning;
        out.push(
            SpriteDraw::at_pivot(Sheet::StopShadow, size, (footprint.x, floor), Z_FRONT)
                .with_alpha(alpha as u8),
        );
    }
    // The pulley stays at the raised anchor in every state; the frame hangs from it, shakes in the
    // warning, falls; then the landed remnant.
    out.push(SpriteDraw::at_pivot(
        Sheet::StopFrame,
        1,
        (cfg.hanging.x, bottom(cfg.hanging)),
        Z_FRONT,
    ));
    if let Some(m) = mass.mass_rect() {
        let frame = match state {
            MassState::Supported | MassState::Weakened => 0,
            MassState::Released => 2 + ((tick / FRAME_SHAKE_TICKS) % 2) as u8,
            MassState::Falling | MassState::Landed => 4,
        };
        out.push(SpriteDraw::at_pivot(
            Sheet::StopFrame,
            frame,
            (m.x, bottom(m)),
            Z_FRONT,
        ));
    }
    if let Some(r) = mass.remnant() {
        if glancing.is_none() {
            out.push(SpriteDraw::at_pivot(
                Sheet::StopRemnant,
                u8::from(hit),
                (r.x, bottom(r)),
                Z_BACK,
            ));
        }
        // The landing dust at both beam ends: two steam puffs each, 6 ticks apart.
        let landed = glance
            .map(|(at, _)| at + GLANCE_TICKS)
            .or_else(|| mass.landed_tick());
        if let Some(lt) = landed.filter(|lt| tick >= *lt) {
            for x in [r.x + 2, r.x + r.w - 2] {
                for n in 0..2 {
                    let Some(age) = (tick - lt).checked_sub(n * DUST_EVERY) else {
                        continue;
                    };
                    if age < DUST_LIFE {
                        let frame = SMOKE_STEAM_FIRST + (age / STEAM_FRAME_TICKS) as u8;
                        out.push(smoke_at(frame, (x, floor + 6), 0, 0.0, Z_FRONT));
                    }
                }
            }
        }
    }
    // The rear glance: the beam strikes the rear deck, tips off behind the exhaust (three frames
    // turned 0, 15 and 35 degrees) and the remnant drops in behind the machine.
    if let Some((at, front_x)) = glancing {
        let k = tick - at;
        let index = GLANCE_FRAME_TICKS
            .iter()
            .scan(0, |end, t| {
                *end += t;
                Some(*end)
            })
            .position(|end| k < end)
            .unwrap_or(2);
        let hull = d.push_box_at(front_x as f32 - d.tuning.wheel_front());
        let (deck_x, deck_y) = (
            hull.left.round() as i32,
            hull.top().round() as i32 - GLANCE_DECK_DROP_PX,
        );
        let strike_x = deck_x + GLANCE_INSET_PX + cfg.footprint.w / 2;
        let (dx, dy) = GLANCE_OFFSETS[index];
        if index < 2 {
            out.push(
                SpriteDraw::at_pivot(
                    Sheet::StopGlance,
                    3,
                    (strike_x, deck_y + GLANCE_SHADOW_PIVOT_ABOVE_DECK),
                    Z_FRONT,
                )
                .with_alpha(GLANCE_SHADOW_ALPHA),
            );
        }
        out.push(SpriteDraw::at_pivot(
            Sheet::StopGlance,
            index as u8,
            (strike_x + dx, deck_y + GLANCE_PIVOT_ABOVE_DECK + dy),
            Z_FRONT,
        ));
    }
}

/// Where the rear glance's beam meets the deck (world px): the hit-spark's point, 16 px in from the beam's
/// left end.
fn glance_contact(d: &Dozer, front_x: i32) -> (i32, i32) {
    let hull = d.push_box_at(front_x as f32 - d.tuning.wheel_front());
    (
        hull.left.round() as i32 + GLANCE_INSET_PX + 16,
        hull.top().round() as i32 - GLANCE_DECK_DROP_PX + 2,
    )
}

/// The events of the current attempt, newest first. Ticks are attempt-local
/// (a defeat retry counts from 1 again), so the scan stops at the latest
/// `Restart`: an effect of an earlier attempt is never drawn again when the
/// new attempt reaches the same tick.
fn attempt_events(combat: &CombatState) -> impl Iterator<Item = &CombatEvent> {
    combat
        .events
        .iter()
        .rev()
        .take_while(|e| !matches!(e, CombatEvent::Restart { .. }))
}

/// The tick and wheel front of the latest rear glance of this attempt, if
/// any (the log is a bounded ring; the glance plays for [`GLANCE_TICKS`]).
pub fn last_glance(combat: &CombatState) -> Option<(u64, i32)> {
    attempt_events(combat).find_map(|e| match *e {
        CombatEvent::Dozer {
            tick,
            event: DozerEvent::Glance { front_x },
        } => Some((tick, front_x)),
        _ => None,
    })
}

/// An effect sprite with its pivot pixel on `(x, y)` (world px, y up).
fn fx_at(sheet: Sheet, frame: u64, at: (i32, i32)) -> SpriteDraw {
    SpriteDraw::at_pivot(sheet, frame as u8, at, FX_Z)
}

/// The frame of an effect `age` ticks old, while it plays.
fn fx_frame(sheet: Sheet, frame_ticks: u64, age: u64) -> Option<u64> {
    let frame = age / frame_ticks;
    (frame < u64::from(sheet.frames())).then_some(frame)
}

/// The world centre of the stop frame's brace cleat (the prop's hit target):
/// the splinter burst's anchor, "the centre of the break".
pub fn brace_centre(d: &Dozer, env: &EnvRuntime) -> Option<(i32, i32)> {
    let mass = env.mass(FRAME_PROP)?;
    let b = env.prop_target(FRAME_PROP)?;
    let floor = d.floor_y.round() as i32;
    let bottom = floor + mass.config().footprint.bottom() - b.bottom();
    Some((b.x + b.w / 2, bottom + b.h / 2))
}

/// DRD-AD1: where a non-blast clang (a melee hit or a Magic Missile) sparks:
/// on the hull face nearest the hero nearest the hull (the rear face of the push box for a hit from behind, the
/// wheel front for one from ahead), at chest height. `Clang` carries the
/// player slot, not the hero, and one pad may drive either hero.
pub fn spark_point(d: &Dozer, heroes_x: [f32; 2]) -> (i32, i32) {
    let left = d.push_box().left;
    let right = d.wheel_front();
    let gap = |x: f32| (left - x).max(x - right).max(0.0);
    let x = heroes_x
        .into_iter()
        .min_by(|a, b| gap(*a).total_cmp(&gap(*b)))
        .unwrap_or(left)
        .clamp(left, right);
    (x.round() as i32, d.floor_y.round() as i32 + SPARK_CHEST_PX)
}

/// DR-D (INT DOZER-ROOM, audit-art WO 15): the room's effects, newest first,
/// at most [`MAX_FX`]:
/// - the DOZ-01 **hit-spark** for each non-blast clang on the hull (a blast's
///   clang has its own blast art), for 3 frames of 2 ticks;
/// - the ENV-02 **splinter burst** on the brace cleat for each counted hit on
///   the stop frame's brace and for its release (one burst a tick);
/// - two splinter bursts in the **rear gate** as the machine emerges.
///
/// A function of this attempt's event log, the machine and the tick only.
pub fn fx_sprites(
    combat: &CombatState,
    d: &Dozer,
    heroes_x: [f32; 2],
    tick: u64,
) -> Vec<SpriteDraw> {
    let floor = d.floor_y.round() as i32;
    let brace = combat.env().and_then(|env| brace_centre(d, env));
    let gate_x = (GATE_X.0 + GATE_X.1) / 2;
    let mut out = Vec::new();
    let mut brace_tick = None;
    for e in attempt_events(combat) {
        if out.len() >= MAX_FX {
            break;
        }
        match *e {
            CombatEvent::Dozer {
                tick: at,
                event: DozerEvent::Clang { blast: false, .. },
            } if at <= tick => {
                if let Some(frame) = fx_frame(Sheet::Spark, SPARK_FRAME_TICKS, tick - at) {
                    out.push(fx_at(Sheet::Spark, frame, spark_point(d, heroes_x)));
                }
            }
            CombatEvent::PropHit { tick: at, prop, .. }
            | CombatEvent::PropPhase {
                tick: at,
                prop,
                phase: PropPhase::Released,
                ..
            } if usize::from(prop) == FRAME_PROP && at <= tick && brace_tick != Some(at) => {
                brace_tick = Some(at);
                let (Some(anchor), Some(frame)) = (
                    brace,
                    fx_frame(Sheet::Splinter, SPLINTER_FRAME_TICKS, tick - at),
                ) else {
                    continue;
                };
                out.push(fx_at(Sheet::Splinter, frame, anchor));
            }
            // DOZ-02: the stop frame's blow sparks on the tub lip's crumple (tick 0 of the stun).
            CombatEvent::Dozer {
                tick: at,
                event: DozerEvent::Stunned { .. },
            } if at <= tick => {
                if let Some(frame) = fx_frame(Sheet::Spark, SPARK_FRAME_TICKS, tick - at) {
                    let (x, y) = IMPACT_SPARK;
                    out.push(fx_at(
                        Sheet::Spark,
                        frame,
                        (d.o.round() as i32 + x, floor + y),
                    ));
                }
            }
            // DOZ-02: the beam's strike on the rear deck sparks where it meets the iron.
            CombatEvent::Dozer {
                tick: at,
                event: DozerEvent::Glance { front_x },
            } if at <= tick => {
                if let Some(frame) = fx_frame(Sheet::Spark, SPARK_FRAME_TICKS, tick - at) {
                    out.push(fx_at(Sheet::Spark, frame, glance_contact(d, front_x)));
                }
            }
            CombatEvent::Dozer {
                tick: at,
                event: DozerEvent::Emerged { .. },
            } if at <= tick => {
                for (n, y) in GATE_BURST_Y.into_iter().enumerate() {
                    let Some(age) = (tick - at).checked_sub(n as u64 * GATE_BURST_STAGGER) else {
                        continue;
                    };
                    if let Some(frame) = fx_frame(Sheet::Splinter, SPLINTER_FRAME_TICKS, age) {
                        out.push(fx_at(Sheet::Splinter, frame, (gate_x, floor + y)));
                    }
                }
            }
            _ => {}
        }
    }
    out.truncate(MAX_FX);
    out
}

/// A hero is within [`YELL_RANGE_PX`] of the wheel front (its body's left
/// edge, heroes ahead of the axle only).
pub fn hero_near(d: &Dozer, heroes_x: [f32; 2]) -> bool {
    let (axle_x, _) = d.axle();
    heroes_x
        .iter()
        .filter(|x| **x > axle_x)
        .any(|x| x - HERO_HALF_WIDTH - d.wheel_front() <= YELL_RANGE_PX)
}

/// Everything the presenter draws this tick. `heroes_x` are Carl's and
/// Donut's body columns. `None` outside room 3 (no machine).
pub fn draw(combat: &CombatState, heroes_x: [f32; 2], tick: u64) -> Option<DozerArt> {
    let d = dozer_of(combat)?;
    let mut sprites = puffs(d, tick);
    if d.state.emerged() {
        let seated = [Seat::RiderA, Seat::RiderB].map(|seat| combat.seat_taken(seat));
        sprites.extend(rig_sprites(d, tick, hero_near(d, heroes_x), seated));
    }
    sprites.extend(fx_sprites(combat, d, heroes_x, tick));
    let pieces = sprites.len();
    room_pieces(d, tick, &mut sprites);
    if let Some(env) = combat.env() {
        frame_sprites(d, combat, env, last_glance(combat), tick, &mut sprites);
    }
    debug_assert!(
        sprites.len() - pieces <= MAX_PIECES && sprites.len() <= MAX_SPRITES,
        "{} sprites",
        sprites.len()
    );
    Some(DozerArt {
        sprites,
        state: d.state.name(),
    })
}

#[cfg(test)]
mod tests;
