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
//! - **labelled stand-ins** ([`StandIn`]) for the environment pieces DOZ-02
//!   will draw (DZ-AD13): the rear gate (still, shaking in the rumble, burst
//!   once the machine is out) and the drawn 16 px buffer stop, plus a marker
//!   for a damage state; DR-B adds the **stop frame** ([`frame_stand_ins`]):
//!   the hanging frame and its chain, the warning shake, the fall, the floor
//!   shadow of the warning, the landed remnant, the brace cleat on the back
//!   wall (intact, weakened, snapped) and the **rear glance** (the frame strikes
//!   the rear deck and tips off behind the exhaust). Every stand-in's label
//!   says so and the presenter draws a `STAND-IN` tag over each.
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
}

impl Sheet {
    pub const ALL: [Self; 10] = [
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
        }
    }

    /// The manifest's frame count (one row).
    pub const fn frames(self) -> u32 {
        match self {
            Self::Hull | Self::FrontCab | Self::FrontTub | Self::Yoke => 1,
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

/// The environment stand-ins (room px, DZ-Q22): the rear gate (x 0..32, the
/// lane's full height) and the drawn buffer stop (x 1008..1024).
pub const GATE_X: (i32, i32) = (0, 32);
pub const BUFFER_X: (i32, i32) = (1008, 1024);
/// DRA-AD4: the buffer stop's drawn height (not data: it is no solid).
pub const BUFFER_H_PX: i32 = 24;
/// The lane's ceiling above the walking line (`ceiling_px`).
pub const LANE_H_PX: i32 = 228;
/// The gate rattles 1 px each way, a step every this many ticks.
pub const GATE_SHAKE_TICKS: u64 = 2;

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

/// One atlas sprite: its sheet and frame, its bottom-left corner (world px,
/// y up) and its draw order (the rig's z).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteDraw {
    pub sheet: Sheet,
    pub frame: u8,
    pub left: i32,
    pub bottom: i32,
    pub z: u8,
}

impl SpriteDraw {
    /// The sprite's centre (world px, y up).
    pub fn centre(&self) -> (f32, f32) {
        let (w, h) = self.sheet.frame_size();
        (
            self.left as f32 + w as f32 / 2.0,
            self.bottom as f32 + h as f32 / 2.0,
        )
    }
}

/// The tone of a code-drawn stand-in (`[r, g, b, a]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    GateIron,
    GateBar,
    Debris,
    HazardYellow,
    HazardBlack,
    Jam,
    FrameIron,
    FrameChain,
    BraceWood,
    FrameShadow,
}

impl Tone {
    pub const fn rgba(self) -> [u8; 4] {
        match self {
            Self::GateIron => [0x3a, 0x33, 0x2e, 0xff],
            Self::GateBar => [0x5c, 0x52, 0x48, 0xff],
            Self::Debris => [0x4a, 0x40, 0x38, 0xff],
            Self::HazardYellow => [0xd8, 0xa8, 0x30, 0xff],
            Self::HazardBlack => [0x22, 0x1e, 0x1c, 0xff],
            Self::Jam => [0x8a, 0x2a, 0x22, 0xff],
            Self::FrameIron => [0x46, 0x4a, 0x50, 0xff],
            Self::FrameChain => [0x6e, 0x6a, 0x64, 0xff],
            Self::BraceWood => [0x6a, 0x48, 0x2c, 0xff],
            Self::FrameShadow => [0x10, 0x0c, 0x0a, 0x90],
        }
    }
}

/// Where a stand-in is drawn: behind the machine, or in front of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Back,
    Front,
}

/// One code-drawn rectangle of a labelled stand-in (world px, y up).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandIn {
    pub left: i32,
    pub bottom: i32,
    pub w: i32,
    pub h: i32,
    pub tone: Tone,
    pub layer: Layer,
    /// What it stands in for (for the capture and the tests).
    pub label: &'static str,
}

/// The labels of the stand-ins: each says it is one.
pub const GATE_LABEL: &str = "rear gate (stand-in until DOZ-02)";
pub const GATE_SHAKE_LABEL: &str = "rear gate shaking (stand-in until DOZ-02)";
pub const GATE_BURST_LABEL: &str = "rear gate burst (stand-in until DOZ-02)";
pub const BUFFER_LABEL: &str = "buffer stop (stand-in until DOZ-02)";
pub const DAMAGE_LABEL: &str = "jammed wheel (stand-in until DOZ-02)";
pub const FRAME_LABEL: &str = "stop frame (stand-in until DOZ-02)";
pub const FRAME_WARN_LABEL: &str = "stop frame warning shake (stand-in until DOZ-02)";
pub const FRAME_FALL_LABEL: &str = "stop frame falling (stand-in until DOZ-02)";
pub const FRAME_SHADOW_LABEL: &str = "stop frame floor shadow (stand-in until DOZ-02)";
pub const REMNANT_LABEL: &str = "landed frame remnant (stand-in until DOZ-02)";
pub const BRACE_LABEL: &str = "brace cleat (stand-in until DOZ-02)";
pub const BRACE_WEAK_LABEL: &str = "brace cleat weakened (stand-in until DOZ-02)";
pub const BRACE_SNAPPED_LABEL: &str = "brace cleat snapped (stand-in until DOZ-02)";
pub const GLANCE_LABEL: &str = "rear glance (stand-in until DOZ-02)";

/// The room's stop frame: prop 0 of the sidecar (`f01_dozer::STOP_FRAME_PROP`).
pub const FRAME_PROP: usize = crate::f01_dozer::STOP_FRAME_PROP;
/// The warning shake: 1 px each way, a step every this many ticks (2 frames).
pub const FRAME_SHAKE_TICKS: u64 = 3;
/// The rear glance plays this many ticks from the landing (strike, tip, off).
pub const GLANCE_TICKS: u64 = 18;
/// The floor shadow's drawn height.
pub const SHADOW_H_PX: i32 = 4;
/// The chain's drawn width.
pub const CHAIN_W_PX: i32 = 2;

/// The most stand-in rectangles [`draw`] returns (the presenter's pool).
pub const MAX_STAND_INS: usize = 24;
/// The most `STAND-IN` tags.
pub const MAX_TAGS: usize = 6;
/// The tag's text (the 3 x 5 label font has every glyph).
pub const TAG_TEXT: &str = "STAND-IN";

/// One frame of the room's machine presentation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DozerArt {
    /// Rig parts and smoke, back to front.
    pub sprites: Vec<SpriteDraw>,
    pub stand_ins: Vec<StandIn>,
    /// Bottom-left corners (world px) of the `STAND-IN` tags.
    pub tags: Vec<(i32, i32)>,
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

/// The engineer's, rider A's and rider B's frames.
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
        _ => idle,
    }
}

/// The rig's atlas sprites at origin `o` on the walking line `floor_y`.
/// `seated`: whether riders A and B are aboard (DR-C: a seat whose rider
/// has dismounted, or stayed dead, is not drawn).
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
    let jammed = matches!(
        d.state,
        DozerState::ImpactStunned { .. } | DozerState::Disabled
    );
    Part::ALL
        .into_iter()
        .filter(|part| match part {
            Part::RiderA => seated[0],
            Part::RiderB => seated[1],
            _ => true,
        })
        .map(|part| {
            let sheet = part.sheet();
            let (_, h) = sheet.frame_size();
            let (px, py) = sheet.pivot();
            let (ax, ay) = part.anchor();
            let lift = match part.rattles() {
                Rattles::Hull => hull_up,
                Rattles::Rider => rider_up,
                Rattles::No => 0,
            };
            let frame = match part {
                Part::Tread => tread_frame(distance),
                // A jammed wheel holds its phase (DOZ-02 draws the jammed frame).
                Part::Wheel if jammed => 0,
                Part::Wheel => wheel_frame(distance, d.wheel_r()),
                Part::Engineer => riders[0],
                Part::RiderA => riders[1],
                Part::RiderB => riders[2],
                _ => 0,
            };
            // The pivot pixel's bottom-left corner sits on the anchor.
            SpriteDraw {
                sheet,
                frame,
                left: o + ax - px,
                bottom: floor + ay + lift - (h as i32 - 1 - py),
                z: part.z(),
            }
        })
        .collect()
}

/// One puff source: its emitter (world px) at the tick of its birth is the
/// current emitter moved back along the travel by `speed * age`.
struct Source {
    emitter: (i32, i32),
    steam: bool,
}

fn puff(source: &Source, age: u64, back: f32) -> SpriteDraw {
    let (life_frames, first, frame_ticks) = if source.steam {
        (5, SMOKE_STEAM_FIRST, STEAM_FRAME_TICKS)
    } else {
        (6, SMOKE_BLACK_FIRST, BLACK_FRAME_TICKS)
    };
    let frame = first + ((age / frame_ticks) as u8).min(life_frames - 1);
    let (w, h) = Sheet::Smoke.frame_size();
    let x = source.emitter.0 as f32 - back;
    let y = source.emitter.1 as f32 + RISE_PX_PER_TICK * age as f32;
    SpriteDraw {
        sheet: Sheet::Smoke,
        frame,
        left: (x - w as f32 / 2.0).round() as i32,
        bottom: (y - h as f32 / 2.0).round() as i32,
        z: 0,
    }
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
        DozerState::Dormant { .. } | DozerState::Disabled => {}
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
        DozerState::ImpactStunned { .. } => {
            burst(&at(KETTLE, true), tick % 30, &mut out);
            periodic(&at(STACK, false), tick, 8, 0.0, &mut out);
        }
    }
    out.truncate(MAX_PUFFS);
    out
}

/// The rear gate and the buffer stop (labelled stand-ins until DOZ-02), and the
/// damage marker of a jammed machine.
pub fn stand_ins(d: &Dozer, tick: u64) -> (Vec<StandIn>, Vec<(i32, i32)>) {
    let floor = d.floor_y.round() as i32;
    let mut out = Vec::new();
    let mut tags = Vec::new();
    let gate_w = GATE_X.1 - GATE_X.0;
    let piece = |left, bottom, w, h, tone, layer, label| StandIn {
        left,
        bottom,
        w,
        h,
        tone,
        layer,
        label,
    };
    if d.state.emerged() {
        // Burst: the jambs stay, the panel lies in pieces in front of them.
        for x in [GATE_X.0, GATE_X.1 - 4] {
            out.push(piece(
                x,
                floor,
                4,
                LANE_H_PX,
                Tone::GateBar,
                Layer::Back,
                GATE_BURST_LABEL,
            ));
        }
        for (x, w, h) in [(GATE_X.1 + 2, 22, 4), (GATE_X.1 + 30, 12, 3), (6, 18, 5)] {
            out.push(piece(
                x,
                floor,
                w,
                h,
                Tone::Debris,
                Layer::Back,
                GATE_BURST_LABEL,
            ));
        }
    } else {
        let (label, dx) = match d.state {
            DozerState::Approaching { .. } => (
                GATE_SHAKE_LABEL,
                if (tick / GATE_SHAKE_TICKS).is_multiple_of(2) {
                    1
                } else {
                    -1
                },
            ),
            _ => (GATE_LABEL, 0),
        };
        out.push(piece(
            GATE_X.0 + dx,
            floor,
            gate_w,
            LANE_H_PX,
            Tone::GateIron,
            Layer::Back,
            label,
        ));
        for x in [4, 14, 24] {
            out.push(piece(
                GATE_X.0 + x + dx,
                floor,
                4,
                LANE_H_PX,
                Tone::GateBar,
                Layer::Back,
                label,
            ));
        }
    }
    tags.push((GATE_X.0, floor + 100));
    // The buffer stop: hazard paint in 4 px bands, drawn only (no solid).
    let bw = BUFFER_X.1 - BUFFER_X.0;
    out.push(piece(
        BUFFER_X.0,
        floor,
        bw,
        BUFFER_H_PX,
        Tone::HazardBlack,
        Layer::Front,
        BUFFER_LABEL,
    ));
    for band in (0..BUFFER_H_PX).step_by(8) {
        out.push(piece(
            BUFFER_X.0,
            floor + band,
            bw,
            4,
            Tone::HazardYellow,
            Layer::Front,
            BUFFER_LABEL,
        ));
    }
    tags.push((BUFFER_X.0 - 8, floor + BUFFER_H_PX + 4));
    // A jammed or disabled machine (DR-B and DR-C): a marker bar across the wheel.
    if matches!(
        d.state,
        DozerState::ImpactStunned { .. } | DozerState::Disabled
    ) {
        let (cx, cy) = d.axle();
        let r = d.wheel_r() as i32;
        out.push(piece(
            cx.round() as i32 - r,
            cy.round() as i32 + r / 2,
            2 * r,
            4,
            Tone::Jam,
            Layer::Front,
            DAMAGE_LABEL,
        ));
        tags.push((cx.round() as i32 - 16, cy.round() as i32 + r + 4));
    }
    debug_assert!(out.len() <= MAX_STAND_INS && tags.len() <= MAX_TAGS);
    (out, tags)
}

/// DR-B: the stop frame and its brace (labelled stand-ins until DOZ-02), read
/// from the env runtime's falling mass, and the rear glance for
/// [`GLANCE_TICKS`] after a [`DozerEvent::Glance`] (`glance`: its tick and
/// wheel front). Layout rects (y down) map to world px through the footprint,
/// which rests on the machine's walking line.
pub fn frame_stand_ins(
    d: &Dozer,
    env: &EnvRuntime,
    glance: Option<(u64, i32)>,
    tick: u64,
    out: &mut Vec<StandIn>,
    tags: &mut Vec<(i32, i32)>,
) {
    let Some(mass) = env.mass(FRAME_PROP) else {
        return;
    };
    let floor = d.floor_y.round() as i32;
    let footprint = mass.config().footprint;
    // World bottom of a layout rect.
    let bottom = |r: IRect| floor + footprint.bottom() - r.bottom();
    let state = mass.state();
    let piece = |r: IRect, dx: i32, tone, layer, label| StandIn {
        left: r.x + dx,
        bottom: bottom(r),
        w: r.w,
        h: r.h,
        tone,
        layer,
        label,
    };
    // The brace cleat on the back wall (the prop's hit target).
    if let Some(b) = env.prop_target(FRAME_PROP) {
        match state {
            MassState::Supported => {
                out.push(piece(b, 0, Tone::BraceWood, Layer::Back, BRACE_LABEL));
            }
            MassState::Weakened => {
                out.push(piece(b, 0, Tone::BraceWood, Layer::Back, BRACE_WEAK_LABEL));
                let crack = IRect::new(b.x, b.y + b.h / 2 - 1, b.w, 3);
                out.push(piece(crack, 0, Tone::Jam, Layer::Back, BRACE_WEAK_LABEL));
            }
            _ => {
                // Snapped: the stubs stay, the middle is gone.
                let top = IRect::new(b.x, b.y, b.w, b.h / 4);
                let foot = b.h / 3;
                let low = IRect::new(b.x, b.bottom() - foot, b.w, foot);
                out.push(piece(
                    top,
                    0,
                    Tone::BraceWood,
                    Layer::Back,
                    BRACE_SNAPPED_LABEL,
                ));
                out.push(piece(
                    low,
                    0,
                    Tone::BraceWood,
                    Layer::Back,
                    BRACE_SNAPPED_LABEL,
                ));
            }
        }
        tags.push((b.x - 8, bottom(b) + b.h + 4));
    }
    // The frame: hanging (with its chain to the lane ceiling), shaking in the
    // warning, falling; then the landed remnant.
    let (label, dx) = match state {
        MassState::Supported | MassState::Weakened => (FRAME_LABEL, 0),
        MassState::Released => (
            FRAME_WARN_LABEL,
            if (tick / FRAME_SHAKE_TICKS).is_multiple_of(2) {
                1
            } else {
                -1
            },
        ),
        MassState::Falling | MassState::Landed => (FRAME_FALL_LABEL, 0),
    };
    if let Some(m) = mass.mass_rect() {
        if state != MassState::Falling {
            let ceiling = floor + LANE_H_PX;
            let top = bottom(m) + m.h;
            out.push(StandIn {
                left: m.x + m.w / 2 - CHAIN_W_PX / 2 + dx,
                bottom: top,
                w: CHAIN_W_PX,
                h: (ceiling - top).max(0),
                tone: Tone::FrameChain,
                layer: Layer::Front,
                label,
            });
        }
        out.push(piece(m, dx, Tone::FrameIron, Layer::Front, label));
        tags.push((m.x, bottom(m) + m.h + 4));
    }
    if let Some(shadow) = mass.telegraph_rect() {
        let s = IRect::new(
            shadow.x,
            shadow.bottom() - SHADOW_H_PX,
            shadow.w,
            SHADOW_H_PX,
        );
        out.push(piece(
            s,
            0,
            Tone::FrameShadow,
            Layer::Front,
            FRAME_SHADOW_LABEL,
        ));
    }
    if let Some(r) = mass.remnant() {
        out.push(piece(r, 0, Tone::FrameIron, Layer::Back, REMNANT_LABEL));
        if glance.is_none_or(|(at, _)| tick >= at + GLANCE_TICKS) {
            tags.push((r.x, bottom(r) + r.h + 4));
        }
    }
    // The rear glance: the frame strikes the rear deck (flat), tips over its
    // back edge (on end), and drops off behind the exhaust onto its remnant.
    if let Some((at, front_x)) = glance.filter(|(at, _)| (*at..*at + GLANCE_TICKS).contains(&tick))
    {
        let k = (tick - at) as i32;
        let n = GLANCE_TICKS as i32;
        let hull = d.push_box_at(front_x as f32 - d.tuning.wheel_front());
        let (deck_x, deck_y) = (hull.left.round() as i32, hull.top().round() as i32);
        let w = footprint.w;
        let (left, bot, gw, gh) = match k * 3 / n {
            0 => (deck_x + 8, deck_y, w, 8),
            1 => (deck_x - 8, deck_y - 16, 16, w / 2),
            _ => {
                let rest = footprint.x;
                let j = k - 2 * n / 3;
                let span = n - 2 * n / 3;
                (
                    deck_x - 16 + (rest - deck_x + 16) * j / span,
                    deck_y - 16 - (deck_y - 16 - floor) * j * j / (span * span),
                    w,
                    8,
                )
            }
        };
        out.push(StandIn {
            left,
            bottom: bot,
            w: gw,
            h: gh,
            tone: Tone::FrameIron,
            layer: Layer::Front,
            label: GLANCE_LABEL,
        });
        tags.push((left, bot + gh + 4));
    }
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
fn fx_at(sheet: Sheet, frame: u64, (x, y): (i32, i32)) -> SpriteDraw {
    let (_, h) = sheet.frame_size();
    let (px, py) = sheet.pivot();
    SpriteDraw {
        sheet,
        frame: frame as u8,
        left: x - px,
        bottom: y - (h as i32 - 1 - py),
        z: FX_Z,
    }
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
    let (mut stand_ins, mut tags) = stand_ins(d, tick);
    if let Some(env) = combat.env() {
        frame_stand_ins(d, env, last_glance(combat), tick, &mut stand_ins, &mut tags);
    }
    debug_assert!(stand_ins.len() <= MAX_STAND_INS && tags.len() <= MAX_TAGS);
    Some(DozerArt {
        sprites,
        stand_ins,
        tags,
        state: d.state.name(),
    })
}

#[cfg(test)]
mod tests;
