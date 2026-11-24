//! FA-Q23 (HRD-P): the **stand-in presentation** of the Hoarder's room, the pure half
//! (`docs/01-design/enemy-gameplay-hoarder.md` section 4, HRD-Q34, K10). A function of the
//! combat state, the room's sidecar and the tick through **shared references only**, it
//! returns what a presenter draws:
//!
//! - which HOARD-01 frame of the Hoarder, where, mirrored or not, per the sheet's stand-in
//!   table (the committed HOARD-01 frames are reused as they are: no sprite is drawn or
//!   altered here, K10);
//! - a few code-drawn, labelled greybox pieces: the exit door, the entry gate, bank A, the
//!   J-B support post and the lair mound;
//! - the visible labels: every one of her frames carries the label **"STAND-IN until
//!   HOARD-02"** (the HOARD-01 independent review is still pending), and each greybox piece
//!   carries its own name. Labels are rasterised by [`text_pixels`] (a 3 x 5 pixel font), so
//!   the picture needs no font asset.
//!
//! **Presentation only.** Nothing here mutates a state, draws a random number or is hashed,
//! replayed or read back by the simulation: a run that calls every function here each tick
//! has the same state hash as one that never does (a test, FA-Q23). Every output is a function
//! of the machine states and the tick, so a render that skips frames or a replay seek draws the
//! same picture for the same tick.
//!
//! Art: HOARD-01 is the committed candidate (`review_status: prototype_fixture_unapproved`);
//! the timing below is the sheet's playback retime of held frames, not a redraw. The pieces are
//! **stand-ins until HOARD-02 and the pile props** (K10).
//!
//! Coordinates are world px, y up, like the simulation.

use crate::actor_sprite::{ActorSpec, actor_center};
use crate::combat_scene::CombatState;
use crate::combat_scene::pile_boss::of;
use crate::encounter::{Encounter, StaticKind};
use crate::env_prop::{MassState, SlideState};
use crate::pile_boss::{GATE_HEIGHT_PX, GATE_WIDTH_PX, PileMode};

/// HOARD-01: 16 frames of 160 x 128 in one row, pivot (70, 127), authored facing right.
pub const ATLAS: &str = "generated/art/hoard-01.png";
pub const FRAME_SIZE: (u32, u32) = (160, 128);
pub const FRAMES: u32 = 16;
pub const SPEC: ActorSpec = ActorSpec {
    name: "HOARD",
    width: 160,
    height: 128,
    pivot_x: 70,
    pivot_y: 127,
    shadow_width: 0,
    shadow_pivot_x: 0,
};
/// Her opaque height at idle (the manifest's `opaque_height_idle`).
pub const IDLE_HEIGHT_PX: i32 = 112;

/// Her label sits this far above her feet: clear of the raised hands of the rear-up frame
/// (HOARD-01 frame 8, the tallest, about 121 px).
pub const LABEL_ABOVE_FEET_PX: f32 = 134.0;

/// While she is prone (Exposed, Get-up, defeated) her label sits this far above her feet, just
/// over the prone body (HOARD-01 frame 15 is about 48 px tall).
pub const PRONE_LABEL_ABOVE_FEET_PX: f32 = 60.0;

/// The label every frame of her carries.
pub const STAND_IN_LABEL: &str = "STAND-IN UNTIL HOARD-02";

/// HOARD-01 frames (the manifest's tags).
pub const IDLE: [u8; 3] = [0, 1, 2];
pub const WALK: [u8; 4] = [3, 4, 5, 6];
/// The gather (attack tick 0..7) and the rear-up hold (8..33) of the swipe tell.
pub const GATHER: u8 = 7;
pub const REAR_UP: u8 = 8;
pub const SMEAR: u8 = 9;
pub const IMPACT: u8 = 10;
/// The held recovery: after a swipe, a whiffed or jammed shove, and the stand-in of the
/// exposed pose (alive, eyes open; never the `down` frames, the Engineer U1 lesson).
pub const RECOVER: u8 = 11;
pub const HURT: u8 = 12;
pub const DOWN: [u8; 3] = [13, 14, 15];
/// Ticks each idle frame is held (the manifest's 267, 200, 267 ms at 60 Hz).
const IDLE_TICKS: [u64; 3] = [16, 12, 16];
/// Ticks each walk frame is held (133 ms).
const WALK_TICKS: u64 = 8;
/// The swipe's gather lasts this many ticks of the tell, then the rear-up holds.
const GATHER_TICKS: u32 = 8;
/// The shove's tell is drawn this many px back (the wind-up, sheet section 4.2).
pub const SHOVE_WINDUP_PX: f32 = 4.0;
/// A jam shows the hurt frame for this many ticks, then the recovery frame.
const JAM_HURT_TICKS: u32 = 12;
/// The first two `down` frames are held this many ticks each (the manifest's 8 and 12); the
/// last stays.
const DOWN_TICKS: [u64; 2] = [8, 12];
/// She fades in over the first ticks of the reveal (the stand-in of the emerge, HOARD-02).
pub const REVEAL_FADE_TICKS: u32 = 30;
/// While she has no hurtbox (the whole Reveal) she is drawn as a dim silhouette at this
/// alpha, and solid from the first hittable tick (interim, until HOARD-02's emerge art).
pub const REVEAL_DIM_ALPHA: f32 = 0.35;

/// What the presenter knows about her beyond the simulation's state: it is render-local.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HoarderLook {
    /// A hit landed on her a few frames ago: the white flash runs.
    pub flash: bool,
    /// Ticks since the presenter first saw her defeated.
    pub defeated_age: Option<u64>,
}

/// The hit flash runs this many ticks after her hit points drop.
pub const FLASH_TICKS: u64 = 4;

/// What the presenter remembers of her between frames (render-local, never hashed). Kept here, in
/// the pure half, so its rules are tested: the scene tick goes back to 0 on a restart or a retry
/// (`SceneSim` resets it), and a memo that kept an absolute tick would flash her through most of the
/// next attempt, so a tick that goes backwards clears it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HoarderMemo {
    hp: Option<u16>,
    flash_until: u64,
    defeated_since: Option<u64>,
    last_tick: Option<u64>,
}

impl HoarderMemo {
    /// Fold this frame's facts in and return the look to draw with: her hit points, whether she is
    /// alive and the scene tick.
    pub fn observe(&mut self, hp: u16, alive: bool, tick: u64) -> HoarderLook {
        if self.last_tick.is_some_and(|last| tick < last) {
            *self = Self::default();
        }
        self.last_tick = Some(tick);
        if self.hp.is_some_and(|prev| hp < prev) {
            self.flash_until = tick + FLASH_TICKS;
        }
        self.hp = Some(hp);
        if alive {
            self.defeated_since = None;
        } else {
            self.defeated_since.get_or_insert(tick);
        }
        HoarderLook {
            flash: tick < self.flash_until,
            defeated_age: self.defeated_since.map(|since| tick.saturating_sub(since)),
        }
    }
}

fn loop_frame(frames: &[u8], ticks: &[u64], tick: u64) -> u8 {
    let total: u64 = ticks.iter().sum();
    let mut t = tick % total.max(1);
    for (frame, held) in frames.iter().zip(ticks) {
        if t < *held {
            return *frame;
        }
        t -= held;
    }
    frames[0]
}

/// The HOARD-01 frame of her in `mode` (the sheet's stand-in table, section 4): the idle and
/// walk loops, the swipe's gather, rear-up, smear and impact, the held recovery, the shove's
/// tell on the gather frame and its lunge on the smear frame, the jam on the hurt frame and
/// then the recovery, the flinch on the hurt frame, Exposed and Get-up on the recovery frame,
/// and the three `down` frames once defeated. `mode_tick` is the tick in the mode (1 on the
/// tick it was entered).
pub fn frame_for(mode: PileMode, mode_tick: u32, tick: u64, look: HoarderLook) -> u8 {
    match mode {
        PileMode::Dormant => IDLE[0],
        PileMode::Reveal | PileMode::Neutral | PileMode::Turn | PileMode::Cooldown => {
            loop_frame(&IDLE, &IDLE_TICKS, tick)
        }
        PileMode::Walk => WALK[(tick / WALK_TICKS % WALK.len() as u64) as usize],
        PileMode::SwipeTell if mode_tick <= GATHER_TICKS => GATHER,
        PileMode::SwipeTell => REAR_UP,
        PileMode::SwipeSmear => SMEAR,
        PileMode::SwipeImpact => IMPACT,
        PileMode::SwipeRecover | PileMode::ShoveRecover => RECOVER,
        PileMode::ShoveTell => GATHER,
        PileMode::Lunge => SMEAR,
        PileMode::Jammed if mode_tick <= JAM_HURT_TICKS => HURT,
        PileMode::Jammed => RECOVER,
        PileMode::Flinch => HURT,
        PileMode::Exposed | PileMode::GetUp => RECOVER,
        PileMode::Defeated => {
            let age = look.defeated_age.unwrap_or(0);
            if age < DOWN_TICKS[0] {
                DOWN[0]
            } else if age < DOWN_TICKS[0] + DOWN_TICKS[1] {
                DOWN[1]
            } else {
                DOWN[2]
            }
        }
    }
}

/// Her sprite to draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoarderDraw {
    /// The sprite's centre (world px).
    pub centre: (f32, f32),
    pub flip_x: bool,
    pub frame: u8,
    /// 1 once she is hittable; during the Reveal a dim ramp up to [`REVEAL_DIM_ALPHA`].
    pub alpha: f32,
    /// She has no hurtbox now (the Reveal): the presenter draws a dark silhouette. Read from
    /// the simulation's own hurtbox, so it flips on the first hittable tick.
    pub silhouette: bool,
    /// The hit flash is running.
    pub flash: bool,
    /// The pivot x she stands at (the label follows it).
    pub x: f32,
}

/// The tone of a greybox piece: `[r, g, b, a]` in [`PALETTE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    DoorClosed = 0,
    DoorFrame = 1,
    GateBar = 2,
    GateOpen = 3,
    BankBody = 4,
    BankTop = 5,
    Post = 6,
    Mound = 7,
    /// The floor shadow of the stack's footprint while it warns and falls.
    Shadow = 8,
    /// The crack of a weakened post.
    Crack = 9,
}

/// The tones of the stand-ins, in Floor 01's brick and junk colours.
pub const PALETTE: [[u8; 4]; 10] = [
    [0x3a, 0x38, 0x3c, 0xff], // DoorClosed
    [0x8a, 0x4a, 0x2c, 0xff], // DoorFrame
    [0x5c, 0x58, 0x4e, 0xff], // GateBar
    [0x8a, 0x4a, 0x2c, 0x50], // GateOpen (the open gate's footprint, faint)
    [0x5e, 0x4a, 0x3a, 0xff], // BankBody
    [0x7a, 0x60, 0x48, 0xff], // BankTop
    [0x6a, 0x4a, 0x2c, 0xff], // Post
    [0x24, 0x1e, 0x1c, 0xa0], // Mound (low, dark, translucent dressing)
    [0x00, 0x00, 0x00, 0x70], // Shadow
    [0x1c, 0x14, 0x0e, 0xff], // Crack
];

impl Tone {
    pub const fn rgba(self) -> [u8; 4] {
        PALETTE[self as usize]
    }
}

/// The draw order, back to front.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    /// Behind the actors (the mound).
    Back,
    /// With the lane props (bank A, the post, the door, the gate).
    Prop,
}

/// One rectangle of a stand-in, world px (y up).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub tone: Tone,
    pub layer: Layer,
    /// What it stands in for (a label for the capture and the tests).
    pub label: &'static str,
}

/// What a visible label says. Each is rasterised once by [`text_pixels`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKind {
    Hoarder,
    Door,
    Gate,
    BankA,
    PostB,
    /// HRD-C: the J-B stack the post holds up.
    StackB,
}

impl TagKind {
    pub const ALL: [Self; 6] = [
        Self::Hoarder,
        Self::Door,
        Self::Gate,
        Self::BankA,
        Self::PostB,
        Self::StackB,
    ];

    pub const fn text(self) -> &'static str {
        match self {
            Self::Hoarder => STAND_IN_LABEL,
            Self::Door => "DOOR",
            Self::Gate => "GATE",
            Self::BankA => "BANK A",
            Self::PostB => "POST B",
            Self::StackB => "STACK B",
        }
    }
}

/// One visible label: its text, its horizontal centre and the world y of its bottom row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tag {
    pub kind: TagKind,
    pub centre_x: f32,
    pub bottom_y: f32,
}

/// The most pieces [`draw`] can return, with margin: the presenter keeps a pool of this many.
pub const MAX_PIECES: usize = 32;

/// Everything to draw this tick.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HoarderArt {
    /// Her sprite; `None` while she is Dormant (she is under the mound).
    pub hoarder: Option<HoarderDraw>,
    pub pieces: Vec<Piece>,
    pub tags: Vec<Tag>,
}

const fn piece(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    tone: Tone,
    layer: Layer,
    label: &'static str,
) -> Piece {
    Piece {
        x,
        y,
        w,
        h,
        tone,
        layer,
        label,
    }
}

/// The lair mound's width and height (a static backdrop, no collision, sheet DV-3), centred
/// on her home column.
const MOUND_W_PX: i32 = 96;
const MOUND_H_PX: i32 = 8;
/// A stand-in bank shudders by this many px each way while disturbed (presentation).
const SHUDDER_PX: i32 = 1;

/// Everything of the Hoarder's room to draw at `tick`, or `None` when her slot is not in the
/// combat state (any other room). `encounter` is the room's sidecar (the door, bank A and the
/// post), `layout_h` the scene's layout height (layout px, y down, to world px, y up).
pub fn draw(
    combat: &CombatState,
    encounter: &Encounter,
    layout_h: i32,
    tick: u64,
    look: HoarderLook,
) -> Option<HoarderArt> {
    let enemy = of(combat)?;
    let boss = enemy.boss();
    let floor_y = boss.setup().floor_y;
    let mut art = HoarderArt::default();
    let facing = boss.facing();
    let flip_x = facing < 0;
    let mode = boss.mode();

    // Her sprite.
    if mode != PileMode::Dormant {
        let dx = if mode == PileMode::ShoveTell {
            -f32::from(facing) * SHOVE_WINDUP_PX
        } else {
            0.0
        };
        // Interim readability (until HOARD-02): with no hurtbox she is a dim silhouette, and
        // solid from the first hittable tick. Driven only by the simulation's own hurtbox.
        let silhouette = boss.hurt_rects().is_empty() && mode == PileMode::Reveal;
        let alpha = if silhouette {
            (boss.mode_tick() as f32 / REVEAL_FADE_TICKS as f32).clamp(0.0, 1.0) * REVEAL_DIM_ALPHA
        } else {
            1.0
        };
        art.hoarder = Some(HoarderDraw {
            centre: actor_center(SPEC, boss.x() + dx, floor_y, flip_x),
            flip_x,
            frame: frame_for(mode, boss.mode_tick(), tick, look),
            alpha,
            silhouette,
            flash: look.flash && mode != PileMode::Defeated,
            x: boss.x(),
        });
        art.tags.push(Tag {
            kind: TagKind::Hoarder,
            centre_x: boss.x(),
            bottom_y: floor_y
                + if matches!(
                    mode,
                    PileMode::Exposed | PileMode::GetUp | PileMode::Defeated
                ) {
                    PRONE_LABEL_ABOVE_FEET_PX
                } else {
                    LABEL_ABOVE_FEET_PX
                },
        });
    }

    // The lair mound (DV-3), at her home column, behind everything.
    let home = i32::from(boss.tuning().hrdq19_home_x);
    let (mx, my) = (home - MOUND_W_PX / 2, floor_y.round() as i32);
    art.pieces.push(piece(
        mx,
        my,
        MOUND_W_PX,
        MOUND_H_PX,
        Tone::Mound,
        Layer::Back,
        "lair mound (low backdrop dressing, no collision)",
    ));
    // The sidecar's statics: bank A (a solid) and the J-B post (a hit target) with the stack it
    // holds. Their machines are the env's (HRD-B, HRD-C); a room without an env draws them
    // standing.
    let env = combat.env();
    let machines = env.map(|e| e.pile_props(encounter)).unwrap_or_default();
    for s in &encounter.statics {
        let index = machines
            .iter()
            .find(|(id, _)| *id == s.id)
            .map(|(_, i)| usize::from(*i));
        match s.kind {
            StaticKind::Solid => {
                let slide = index.and_then(|i| env.and_then(|e| e.slide(i)));
                bank_pieces(&mut art, s.rect, slide, layout_h, tick);
            }
            StaticKind::HitTarget => {
                let mass = index.and_then(|i| env.and_then(|e| e.mass(i)));
                post_pieces(&mut art, s.rect, mass, layout_h, tick);
            }
        }
    }

    // The exit door: the sidecar's lock solid, closed until her defeat tick, then only its
    // frame (the opening).
    let door_open = enemy.door_open();
    for lock in &encounter.locks {
        let r = lock.solid;
        let (x, y, w, h) = (r.x, layout_h - r.bottom(), r.w, r.h);
        if door_open {
            for (ex, ew) in [(x, 2), (x + w - 2, 2)] {
                art.pieces.push(piece(
                    ex,
                    y,
                    ew,
                    h,
                    Tone::DoorFrame,
                    Layer::Prop,
                    "door frame, open (stand-in)",
                ));
            }
        } else {
            art.pieces.push(piece(
                x,
                y,
                w,
                h,
                Tone::DoorClosed,
                Layer::Prop,
                "exit door (stand-in)",
            ));
            art.pieces.push(piece(
                x,
                y,
                2,
                h,
                Tone::DoorFrame,
                Layer::Prop,
                "exit door frame (stand-in)",
            ));
            art.pieces.push(piece(
                x + w - 2,
                y,
                2,
                h,
                Tone::DoorFrame,
                Layer::Prop,
                "exit door frame (stand-in)",
            ));
        }
        art.tags.push(Tag {
            kind: TagKind::Door,
            centre_x: (x + w / 2) as f32,
            bottom_y: (y + h / 2) as f32,
        });
    }

    // The entry gate (HRD-Q41, boss data): bars while closed, its faint footprint while open.
    let t = boss.tuning();
    if t.hrdq41_gate_enabled {
        let gx = i32::from(t.hrdq41_gate_x);
        let (gy, gw, gh) = (
            floor_y.round() as i32,
            GATE_WIDTH_PX as i32,
            GATE_HEIGHT_PX as i32,
        );
        if boss.gate_solid().is_some() {
            art.pieces.push(piece(
                gx,
                gy,
                gw,
                gh,
                Tone::GateBar,
                Layer::Prop,
                "entry gate, closed (stand-in)",
            ));
            for k in 0..4 {
                art.pieces.push(piece(
                    gx + 1 + k * 4,
                    gy,
                    2,
                    gh,
                    Tone::DoorFrame,
                    Layer::Prop,
                    "entry gate bar (stand-in)",
                ));
            }
        } else {
            art.pieces.push(piece(
                gx,
                gy,
                gw,
                gh,
                Tone::GateOpen,
                Layer::Prop,
                "entry gate, open (stand-in)",
            ));
        }
        art.tags.push(Tag {
            kind: TagKind::Gate,
            centre_x: (gx + gw / 2) as f32,
            bottom_y: floor_y + gh as f32 / 2.0,
        });
    }
    debug_assert!(art.pieces.len() <= MAX_PIECES);
    Some(art)
}

/// Layout rect (y down) to world `(x, y, w, h)` (y up).
fn world(r: crate::camera_lab::room::IRect, layout_h: i32) -> (i32, i32, i32, i32) {
    (r.x, layout_h - r.bottom(), r.w, r.h)
}

/// A junk slab: body, top bar, and a junk strip (the stand-in look of bank A and the stack).
fn slab(art: &mut HoarderArt, (x, y, w, h): (i32, i32, i32, i32), what: &'static str) {
    art.pieces
        .push(piece(x, y, w, h, Tone::BankBody, Layer::Prop, what));
    art.pieces.push(piece(
        x,
        y + h - 4.min(h),
        w,
        4.min(h),
        Tone::BankTop,
        Layer::Prop,
        what,
    ));
}

/// Bank A by its slide state: a solid slab you can stand on while it stands (a one-pixel
/// shudder while disturbed), the moving mass while it slides, the low remnant after.
fn bank_pieces(
    art: &mut HoarderArt,
    rect: crate::camera_lab::room::IRect,
    slide: Option<&crate::env_prop::Slide>,
    layout_h: i32,
    tick: u64,
) {
    let (x, y, w, h) = world(rect, layout_h);
    let state = slide.map_or(SlideState::Stable, crate::env_prop::Slide::state);
    match state {
        SlideState::Stable | SlideState::Disturbed => {
            let dx = if state == SlideState::Disturbed {
                if tick.is_multiple_of(2) {
                    SHUDDER_PX
                } else {
                    -SHUDDER_PX
                }
            } else {
                0
            };
            slab(art, (x + dx, y, w, h), "bank A (stand-in)");
            art.pieces.push(piece(
                x + dx + w / 4,
                y + h / 3,
                w / 2,
                3,
                Tone::BankTop,
                Layer::Prop,
                "bank A junk (stand-in)",
            ));
            art.tags.push(Tag {
                kind: TagKind::BankA,
                centre_x: (x + dx + w / 2) as f32,
                bottom_y: (y + h / 2 - 2) as f32,
            });
        }
        SlideState::Slid => {
            if let Some(m) = slide.and_then(crate::env_prop::Slide::mass_rect) {
                let (mx, my, mw, mh) = world(m, layout_h);
                art.pieces.push(piece(
                    mx,
                    my,
                    mw,
                    mh,
                    Tone::BankBody,
                    Layer::Prop,
                    "bank A, sliding (stand-in)",
                ));
            }
        }
        SlideState::Settled => {
            if let Some(r) = slide.and_then(crate::env_prop::Slide::remnant) {
                slab(art, world(r, layout_h), "bank A remnant (stand-in)");
            }
        }
    }
}

/// The J-B post and its stack by the falling mass' state: the post (cracked once weakened) and
/// the hanging stack while supported, the shaking stack and the floor shadow while released, the
/// dropping stack, then the low remnant.
fn post_pieces(
    art: &mut HoarderArt,
    rect: crate::camera_lab::room::IRect,
    mass: Option<&crate::env_prop::FallingMass>,
    layout_h: i32,
    tick: u64,
) {
    let (x, y, w, h) = world(rect, layout_h);
    let state = mass.map_or(MassState::Supported, crate::env_prop::FallingMass::state);
    if matches!(state, MassState::Supported | MassState::Weakened) {
        art.pieces.push(piece(
            x,
            y,
            w,
            h,
            Tone::Post,
            Layer::Prop,
            "J-B post (stand-in)",
        ));
        if state == MassState::Weakened {
            art.pieces.push(piece(
                x + w / 2 - 1,
                y + h / 3,
                2,
                h / 3,
                Tone::Crack,
                Layer::Prop,
                "J-B post crack (stand-in)",
            ));
        }
        art.tags.push(Tag {
            kind: TagKind::PostB,
            centre_x: (x + w / 2) as f32,
            bottom_y: (y + h + 3) as f32,
        });
    }
    let Some(mass) = mass else { return };
    if let Some(f) = mass.telegraph_rect() {
        let (fx, fy, fw, fh) = world(f, layout_h);
        art.pieces.push(piece(
            fx,
            fy,
            fw,
            fh.min(8),
            Tone::Shadow,
            Layer::Back,
            "J-B footprint shadow (stand-in)",
        ));
    }
    match mass.mass_rect() {
        Some(m) => {
            let dx = if state == MassState::Released {
                if tick.is_multiple_of(2) {
                    SHUDDER_PX
                } else {
                    -SHUDDER_PX
                }
            } else {
                0
            };
            let (mx, my, mw, mh) = world(m, layout_h);
            slab(art, (mx + dx, my, mw, mh), "J-B stack (stand-in)");
            art.tags.push(Tag {
                kind: TagKind::StackB,
                centre_x: (mx + dx + mw / 2) as f32,
                bottom_y: (my + mh / 2 - 2) as f32,
            });
        }
        None => {
            if let Some(r) = mass.remnant() {
                slab(art, world(r, layout_h), "J-B stack remnant (stand-in)");
            }
        }
    }
}

// ------------------------------------------------------------------- labels --

// The font is shared with the Boiler's labels (`crate::label_font`).
pub use crate::label_font::{
    GLYPH_ADVANCE_PX, GLYPH_HEIGHT_PX, TAG_INK, TAG_SHADOW, TextImage, text_pixels,
};

#[cfg(test)]
mod tests;
