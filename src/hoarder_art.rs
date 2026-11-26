//! FA-Q23 (HRD-P) and HOARD-02: the **presentation** of the Hoarder's room, the pure half
//! (`docs/01-design/enemy-gameplay-hoarder.md` section 4, `docs/02-art/hoarder-hoard-02.md`
//! sections 4 and 7). A function of the combat state, the room's sidecar and the tick through
//! **shared references only**, it returns what a presenter draws:
//!
//! - which HOARD-02 frame of the Hoarder, where, mirrored or not, per the spec's state table
//!   ([`frame_for`]); during the Reveal she is drawn one ramp step darker ([`Shade::Shadow`]);
//! - the room kit's sprites ([`PropDraw`]): the exit door, the entry gate, bank A, the J-B post
//!   and stack, the lair mound (`sealed` back and front layers, then the low `opened` spill) and
//!   the dressing heaps. Each prop has its own function ([`bank_prop`], [`post_prop`], ...) from
//!   its state to its frame, so a PR that wires a prop's later states only changes the state it
//!   passes in;
//! - a few cosmetic dust motes ([`Mote`]) for the Reveal (dust sliding off her, then the burst);
//! - the J-B footprint's floor shadow ([`FloorShadow`]) while the stack warns and falls (HRD-C).
//!
//! **Presentation only.** Nothing here mutates a state, draws a random number or is hashed,
//! replayed or read back by the simulation: a run that calls every function here each tick
//! has the same state hash as one that never does (a test, FA-Q23). Every output is a function
//! of the machine states and the tick, so a render that skips frames or a replay seek draws the
//! same picture for the same tick.
//!
//! Art: HOARD-02 (`assets/manifests/hoard-02.json`) and the `floor01_hoarder_*` room kit
//! (`review_status: prototype_fixture_unapproved`). No stand-in label remains.
//!
//! Coordinates are world px, y up, like the simulation.

use crate::actor_sprite::{ActorSpec, actor_center};
use crate::combat_scene::CombatState;
use crate::combat_scene::pile_boss::of;
use crate::encounter::{Encounter, StaticKind};
use crate::env_prop::{FallingMass, MassState, Slide, SlideState};
use crate::pile_boss::PileMode;

/// HOARD-02: 29 frames of 160 x 128 in one row, pivot (70, 127), authored facing right. Frames 0-15
/// are the HOARD-01 frames unchanged; 16-28 are the shove, lunge, jam, pinned, get-up and emerge
/// poses.
pub const ATLAS: &str = "generated/art/hoard-02.png";
pub const FRAME_SIZE: (u32, u32) = (160, 128);
pub const FRAMES: u32 = 29;
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

/// HOARD-02 frames (the manifest's tags).
pub const IDLE: [u8; 3] = [0, 1, 2];
pub const WALK: [u8; 4] = [3, 4, 5, 6];
/// The gather (attack tick 0..7) and the rear-up hold (8..33) of the swipe tell.
pub const GATHER: u8 = 7;
pub const REAR_UP: u8 = 8;
pub const SMEAR: u8 = 9;
pub const IMPACT: u8 = 10;
/// The held recovery after a swipe or a whiffed shove. It is no longer the jam hold, Exposed or
/// Get-up (spec section 3).
pub const RECOVER: u8 = 11;
pub const HURT: u8 = 12;
pub const DOWN: [u8; 3] = [13, 14, 15];
/// The shove tell: the settle (an in-between from idle, already wide) then the low brace.
pub const SHOVE_SETTLE: u8 = 16;
pub const SHOVE_BRACE: u8 = 17;
/// The lunge: two heave frames, alternating.
pub const LUNGE: [u8; 2] = [18, 19];
/// The jam: the impact on the obstacle face, then the dazed stagger hold.
pub const JAM_IMPACT: u8 = 20;
pub const JAM_STAGGER: u8 = 21;
/// Pinned under the landed J-B remnant (alive, eyes open: never the `down` frames), alternating.
pub const PINNED: [u8; 2] = [22, 23];
/// Get-up: the push on hands and knees, then the rise.
pub const GET_UP: [u8; 2] = [24, 25];
/// The emerge, in thirds of the Reveal: kneeling in the mound, rising, standing.
pub const EMERGE: [u8; 3] = [26, 27, 28];
/// Ticks each idle frame is held (the manifest's 267, 200, 267 ms at 60 Hz).
const IDLE_TICKS: [u64; 3] = [16, 12, 16];
/// Ticks each walk frame is held (133 ms).
const WALK_TICKS: u64 = 8;
/// The swipe's gather lasts this many ticks of the tell, then the rear-up holds.
const GATHER_TICKS: u32 = 8;
/// The shove tell shows the settle for this many ticks, then the brace (spec section 4).
const SHOVE_SETTLE_TICKS: u32 = 10;
/// Each lunge frame is held this many ticks, starting on the first.
const LUNGE_TICKS: u32 = 6;
/// A jam shows the impact for this many ticks, then the stagger hold.
const JAM_IMPACT_TICKS: u32 = 12;
/// Each pinned frame is held this many ticks.
const PINNED_TICKS: u32 = 20;
/// The get-up shows the push for this many ticks, then the rise.
const GET_UP_PUSH_TICKS: u32 = 12;
/// The first two `down` frames are held this many ticks each (the manifest's 8 and 12); the
/// last stays.
const DOWN_TICKS: [u64; 2] = [8, 12];
/// The first Reveal's length when the sim's value is not known (`hrdq05_reveal_ticks`).
const REVEAL_TICKS_DEFAULT: u32 = 90;
/// R7: the dust burst that marks "she is out" lasts this many ticks from the first tick she can be hit.
pub const OUT_BURST_TICKS: u64 = 4;

/// What the presenter knows about her beyond the simulation's state: it is render-local.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HoarderLook {
    /// A hit landed on her a few frames ago: the white flash runs.
    pub flash: bool,
    /// Ticks since the presenter first saw her defeated.
    pub defeated_age: Option<u64>,
    /// Ticks since the presenter first saw her out of the mound (the first tick she is neither
    /// Dormant nor in the Reveal); `None` while she is still hidden.
    pub out_age: Option<u64>,
    /// The Reveal's length in ticks, read from the sim by [`draw`] (0 = the first Reveal's 90).
    pub reveal_ticks: u32,
    /// Ticks since the presenter first saw the J-B stack released (its 24-tick warning); `None`
    /// while it hangs, falls or has landed. Drives the stack's sag ([`stack_warning_offset`]).
    pub released_age: Option<u64>,
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
    out_since: Option<u64>,
    released_since: Option<u64>,
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
            out_age: self.out_since.map(|since| tick.saturating_sub(since)),
            reveal_ticks: 0,
            released_age: self.released_since.map(|since| tick.saturating_sub(since)),
        }
    }

    /// Fold in whether she is still hidden (Dormant or in the Reveal) and return the ticks since
    /// she came out (0 on the first tick she can be hit), for the R7 dust burst. Call after
    /// [`Self::observe`] on the same tick; a restart (a tick that went backwards) already cleared it.
    pub fn observe_out(&mut self, hidden: bool, tick: u64) -> Option<u64> {
        if hidden {
            self.out_since = None;
        } else {
            self.out_since.get_or_insert(tick);
        }
        self.out_since.map(|since| tick.saturating_sub(since))
    }

    /// Fold in whether the J-B stack is released (warning, [`stack_released`]) and return the ticks
    /// since the presenter first saw it so (0 on the release tick), for its sag. Call after
    /// [`Self::observe`] on the same tick; a restart already cleared it.
    pub fn observe_released(&mut self, released: bool, tick: u64) -> Option<u64> {
        if released {
            self.released_since.get_or_insert(tick);
        } else {
            self.released_since = None;
        }
        self.released_since.map(|since| tick.saturating_sub(since))
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

/// The HOARD-02 frame of her in `mode` (spec section 4): the idle and walk loops, the swipe's
/// gather, rear-up, smear and impact, the held recovery, the shove's settle and brace, the lunge's
/// two heave frames, the jam's impact and stagger, the flinch, the pinned loop, the get-up's push
/// and rise, the emerge in thirds of the Reveal, and the three `down` frames once defeated.
/// `mode_tick` is the tick in the mode (1 on the tick it was entered).
pub fn frame_for(mode: PileMode, mode_tick: u32, tick: u64, look: HoarderLook) -> u8 {
    let step = mode_tick.saturating_sub(1);
    match mode {
        PileMode::Dormant => EMERGE[0],
        PileMode::Reveal => {
            let len = if look.reveal_ticks == 0 {
                REVEAL_TICKS_DEFAULT
            } else {
                look.reveal_ticks
            };
            EMERGE[(step.min(len - 1) * 3 / len) as usize]
        }
        PileMode::Neutral | PileMode::Turn | PileMode::Cooldown => {
            loop_frame(&IDLE, &IDLE_TICKS, tick)
        }
        PileMode::Walk => WALK[(tick / WALK_TICKS % WALK.len() as u64) as usize],
        PileMode::SwipeTell if mode_tick <= GATHER_TICKS => GATHER,
        PileMode::SwipeTell => REAR_UP,
        PileMode::SwipeSmear => SMEAR,
        PileMode::SwipeImpact => IMPACT,
        PileMode::SwipeRecover | PileMode::ShoveRecover => RECOVER,
        PileMode::ShoveTell if mode_tick <= SHOVE_SETTLE_TICKS => SHOVE_SETTLE,
        PileMode::ShoveTell => SHOVE_BRACE,
        PileMode::Lunge => LUNGE[(step / LUNGE_TICKS % 2) as usize],
        PileMode::Jammed if mode_tick <= JAM_IMPACT_TICKS => JAM_IMPACT,
        PileMode::Jammed => JAM_STAGGER,
        PileMode::Flinch => HURT,
        PileMode::Exposed => PINNED[(step / PINNED_TICKS % 2) as usize],
        PileMode::GetUp if mode_tick <= GET_UP_PUSH_TICKS => GET_UP[0],
        PileMode::GetUp => GET_UP[1],
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

/// How bright her sprite is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shade {
    /// Full value: she can be hit.
    Full,
    /// R7: about one ramp step darker, the mound's shadow value, while she has no hurtbox (the Reveal).
    Shadow,
}

/// Her sprite to draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoarderDraw {
    /// The sprite's centre (world px).
    pub centre: (f32, f32),
    pub flip_x: bool,
    pub frame: u8,
    pub shade: Shade,
    /// The hit flash is running (never during the Reveal, R7).
    pub flash: bool,
    /// The pivot x she stands at.
    pub x: f32,
}

/// The room kit's props, one atlas each (`assets/manifests/floor01_hoarder_*.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prop {
    Mound,
    Bank,
    Post,
    Stack,
    Door,
    Gate,
    Heap,
}

impl Prop {
    pub const ALL: [Self; 7] = [
        Self::Mound,
        Self::Bank,
        Self::Post,
        Self::Stack,
        Self::Door,
        Self::Gate,
        Self::Heap,
    ];

    pub const fn atlas(self) -> &'static str {
        match self {
            Self::Mound => "generated/art/floor01_hoarder_mound.png",
            Self::Bank => "generated/art/floor01_hoarder_bank_a.png",
            Self::Post => "generated/art/floor01_hoarder_post_b.png",
            Self::Stack => "generated/art/floor01_hoarder_stack_b.png",
            Self::Door => "generated/art/floor01_hoarder_door.png",
            Self::Gate => "generated/art/floor01_hoarder_gate.png",
            Self::Heap => "generated/art/floor01_hoarder_heap.png",
        }
    }

    /// One frame's size in px (the manifest's `frame_size`).
    pub const fn frame_size(self) -> (u32, u32) {
        match self {
            Self::Mound => (176, 72),
            Self::Bank => (80, 48),
            Self::Post => (16, 64),
            Self::Stack => (128, 104),
            Self::Door | Self::Gate => (16, 228),
            Self::Heap => (48, 24),
        }
    }

    /// How many frames the atlas has, in one row.
    pub const fn frames(self) -> u32 {
        match self {
            // HRD-CUES-01: the mound's three `stir_*` back frames (3..5) and the post's `shored` and
            // `give` frames (3, 4) are in the atlases; the presenter does not draw them yet.
            Self::Mound => 6,
            Self::Post => 5,
            Self::Heap => 3,
            Self::Bank => 6,
            Self::Stack => 4,
            Self::Door | Self::Gate => 2,
        }
    }
}

/// The draw order, back to front.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    /// Behind the actors, heroes included (the mound's back layer, the dressing heaps).
    Back,
    /// With the lane props (bank A, the post, the stack, the door, the gate).
    Prop,
    /// In front of the actors (the mound's front lip while she is Dormant or in the Reveal).
    Front,
}

/// One sprite of the room kit: a frame of a prop's atlas with its bottom-left corner at
/// `(x, y)` (world px, y up).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropDraw {
    pub prop: Prop,
    pub frame: u8,
    pub x: i32,
    pub y: i32,
    pub layer: Layer,
}

/// A cosmetic dust mote (world px, y up, the colour is the presenter's): dust sliding off her
/// during the Reveal, and the burst when she comes out. A function of the tick only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mote {
    pub x: f32,
    pub y: f32,
}

/// The most sprites and motes [`draw`] can return, with margin: the presenter keeps a pool of each.
pub const MAX_PROPS: usize = 16;
pub const MAX_MOTES: usize = 16;

/// Everything to draw this tick.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HoarderArt {
    /// Her sprite; `None` while she is Dormant (she is under the mound).
    pub hoarder: Option<HoarderDraw>,
    pub props: Vec<PropDraw>,
    pub motes: Vec<Mote>,
    /// HRD-C: the floor shadow of the J-B stack's footprint while it is released or falling.
    pub shadow: Option<FloorShadow>,
}

/// A translucent dark floor band (world px, y up, bottom-left corner): the J-B stack's footprint
/// warning, read from the falling mass' own telegraph rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloorShadow {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// The footprint shadow is at most this tall (a band on the floor, not the whole volume).
pub const FLOOR_SHADOW_PX: i32 = 8;

// ---------------------------------------------------------------- prop states --

/// Mound frames: the `sealed` back and front layers and the `opened` spill.
pub const MOUND_BACK: u8 = 0;
pub const MOUND_FRONT: u8 = 1;
pub const MOUND_OPENED: u8 = 2;
/// The mound sprite is centred on her home column, its bottom row on the floor (pivot 88, 71).
const MOUND_PIVOT_X: i32 = 88;

/// Bank A's states (HRD-B): standing, shuddering, sliding, settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BankState {
    Stable,
    Disturbed,
    Slid,
    Settled,
}

/// Bank A's frame: `stable` 0, `disturbed` 1 and 2 (a 2-frame shudder, the presenter also shifts it
/// 1 px each way), `slid` 3 and 4 (a tumble), `settled` 5 (the 80 x 16 remnant).
pub const fn bank_frame(state: BankState, tick: u64) -> u8 {
    let alt = (tick % 2) as u8;
    match state {
        BankState::Stable => 0,
        BankState::Disturbed => 1 + alt,
        BankState::Slid => 3 + alt,
        BankState::Settled => 5,
    }
}

/// The J-B post's states (HRD-C): intact, cracked by the first qualifying hit, broken on the release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostState {
    Intact,
    Cracked,
    Broken,
}

pub const fn post_frame(state: PostState) -> u8 {
    match state {
        PostState::Intact => 0,
        PostState::Cracked => 1,
        PostState::Broken => 2,
    }
}

/// The J-B stack's states (HRD-C): hanging, released (the warning), falling, landed (the remnant).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackState {
    Supported,
    Released,
    Falling,
    Landed,
}

pub const fn stack_frame(state: StackState) -> u8 {
    match state {
        StackState::Supported => 0,
        StackState::Released => 1,
        StackState::Falling => 2,
        StackState::Landed => 3,
    }
}

/// The stack hangs on a footprint `[post.x + 8 - 64, +128) x [96, 160)`; its sprite also carries the
/// 32 px strut down to the post's head, so its bottom-left corner is this far under the footprint.
pub const STACK_FOOTPRINT_BOTTOM_PX: i32 = 96;
pub const STACK_LEG_PX: i32 = 32;
pub const STACK_W_PX: i32 = 128;
/// The `falling` frame's strut is snapped to this length (the post is broken): the body is 23 px
/// over the floor on the last falling tick, so the stub never dips under the floor line.
pub const STACK_FALL_LEG_PX: i32 = 20;
/// While the stack is released (its warning) it sags 1 px every this many ticks...
pub const STACK_SAG_EVERY_TICKS: u64 = 6;
/// ...to at most this many px (spec section 7: "sag 1 px every 6 ticks" over the 24-tick warning).
pub const STACK_SAG_MAX_PX: i32 = 3;

/// The released stack's presenter offset `(dx, dy)` (world px, y up): a 1 px shudder each way that
/// flips every 2 ticks, plus the sag of `released_age` (ticks since the release, 0 when unknown).
/// Cosmetic only: the sim's mass waits on its hanging rectangle until it falls.
pub const fn stack_warning_offset(released_age: u64, tick: u64) -> (i32, i32) {
    let shudder = if (tick / 2).is_multiple_of(2) { 1 } else { -1 };
    let steps = released_age / STACK_SAG_EVERY_TICKS;
    let sag = if steps > STACK_SAG_MAX_PX as u64 {
        STACK_SAG_MAX_PX
    } else {
        steps as i32
    };
    (shudder, -sag)
}

/// The dressing heaps: centre columns against the back wall (spec section 7: never between 416 and 880
/// in front of the playfield, so they are drawn behind everything) and the heap each one is.
pub const HEAPS: [(i32, u8); 3] = [(250, 0), (470, 1), (700, 2)];

/// Layout rect (y down) to world `(x, y, w, h)` (y up).
fn world(r: crate::camera_lab::room::IRect, layout_h: i32) -> (i32, i32, i32, i32) {
    (r.x, layout_h - r.bottom(), r.w, r.h)
}

/// Bank A: a sprite on the sidecar's solid, standing on its rectangle's bottom-left corner.
pub fn bank_prop(
    rect: crate::camera_lab::room::IRect,
    state: BankState,
    layout_h: i32,
    tick: u64,
) -> PropDraw {
    let (x, y, _, _) = world(rect, layout_h);
    PropDraw {
        prop: Prop::Bank,
        frame: bank_frame(state, tick),
        x,
        y,
        layer: Layer::Prop,
    }
}

/// The J-B post: a sprite on the sidecar's hit target, bottom-left on its rectangle.
pub fn post_prop(
    rect: crate::camera_lab::room::IRect,
    state: PostState,
    layout_h: i32,
) -> PropDraw {
    let (x, y, _, _) = world(rect, layout_h);
    PropDraw {
        prop: Prop::Post,
        frame: post_frame(state),
        x,
        y,
        layer: Layer::Prop,
    }
}

/// The J-B stack. `body` is the sim's rectangle of it: the hanging or falling mass (`Supported`,
/// `Released`, `Falling`; the sprite carries the [`STACK_LEG_PX`] strut under it, E1) or the landed
/// remnant (`Landed`, the remnant's rows are the bottom of the frame, drawn in front of the actors).
pub fn stack_prop(
    body: crate::camera_lab::room::IRect,
    state: StackState,
    layout_h: i32,
) -> PropDraw {
    let (x, y, _, _) = world(body, layout_h);
    let landed = state == StackState::Landed;
    PropDraw {
        prop: Prop::Stack,
        frame: stack_frame(state),
        x,
        y: if landed { y } else { y - STACK_LEG_PX },
        layer: if landed { Layer::Front } else { Layer::Prop },
    }
}

/// The hanging stack's rectangle over a post when the room has no machine for it: the footprint is
/// centred on the post's column, its bottom [`STACK_FOOTPRINT_BOTTOM_PX`] over the floor (layout y down).
fn hanging_over(post: crate::camera_lab::room::IRect) -> crate::camera_lab::room::IRect {
    crate::camera_lab::room::IRect {
        x: post.x + post.w / 2 - STACK_W_PX / 2,
        y: post.bottom() - STACK_FOOTPRINT_BOTTOM_PX - 64,
        w: STACK_W_PX,
        h: 64,
    }
}

/// The mound's sprites: the `sealed` back layer behind everything and the front lip in front of the
/// actors while she is hidden (Dormant, Reveal); the low `opened` spill behind everything once she is out.
fn mound_props(home: i32, floor_y: i32, hidden: bool) -> Vec<PropDraw> {
    let at = |frame, layer| PropDraw {
        prop: Prop::Mound,
        frame,
        x: home - MOUND_PIVOT_X,
        y: floor_y,
        layer,
    };
    if hidden {
        vec![at(MOUND_BACK, Layer::Back), at(MOUND_FRONT, Layer::Front)]
    } else {
        vec![at(MOUND_OPENED, Layer::Back)]
    }
}

/// The door: a sprite on the sidecar's lock solid, `closed` until her defeat tick, then `open`.
fn door_prop(rect: crate::camera_lab::room::IRect, open: bool, layout_h: i32) -> PropDraw {
    let (x, y, _, _) = world(rect, layout_h);
    PropDraw {
        prop: Prop::Door,
        frame: u8::from(open),
        x,
        y,
        layer: Layer::Prop,
    }
}

/// The entry gate: `closed` while the sim's gate stands, `open` otherwise.
fn gate_prop(x: i32, floor_y: i32, closed: bool) -> PropDraw {
    PropDraw {
        prop: Prop::Gate,
        frame: u8::from(closed),
        x,
        y: floor_y,
        layer: Layer::Prop,
    }
}

fn heap_props(floor_y: i32) -> impl Iterator<Item = PropDraw> {
    HEAPS.into_iter().map(move |(cx, frame)| PropDraw {
        prop: Prop::Heap,
        frame,
        x: cx - Prop::Heap.frame_size().0 as i32 / 2,
        y: floor_y,
        layer: Layer::Back,
    })
}

// ----------------------------------------------------------------- reveal fx --

/// R7 dust: while she is in the Reveal, motes slide down off her shoulders; on the first ticks she can
/// be hit a short burst marks "she is out". Pure functions of the tick (cosmetic, no random numbers).
fn reveal_motes(
    mode: PileMode,
    mode_tick: u32,
    x: f32,
    floor_y: f32,
    out_age: Option<u64>,
) -> Vec<Mote> {
    let mut motes = Vec::new();
    if mode == PileMode::Reveal {
        for k in 0..8u32 {
            let phase = (mode_tick + k * 7) % 28;
            let side = if k % 2 == 0 { -1.0 } else { 1.0 };
            motes.push(Mote {
                x: x + side * (6.0 + (k * 5 % 22) as f32),
                y: floor_y + 52.0 - phase as f32 * 1.4,
            });
        }
    } else if let Some(age) = out_age.filter(|a| *a < OUT_BURST_TICKS) {
        for k in 0..12u32 {
            let ang = k as f32 * std::f32::consts::TAU / 12.0;
            let r = 14.0 + age as f32 * 10.0;
            motes.push(Mote {
                x: x + ang.cos() * r * 1.6,
                y: floor_y + 34.0 + ang.sin() * r,
            });
        }
    }
    motes
}

/// Her sprite for this tick, or `None` while she is Dormant.
fn hoarder_sprite(
    boss: &crate::pile_boss::PileBoss,
    tick: u64,
    look: HoarderLook,
) -> Option<HoarderDraw> {
    let mode = boss.mode();
    if mode == PileMode::Dormant {
        return None;
    }
    let facing = boss.facing();
    let flip_x = facing < 0;
    let shadow = mode == PileMode::Reveal;
    Some(HoarderDraw {
        centre: actor_center(SPEC, boss.x(), boss.setup().floor_y, flip_x),
        flip_x,
        frame: frame_for(mode, boss.mode_tick(), tick, look),
        shade: if shadow { Shade::Shadow } else { Shade::Full },
        // R7: no hit flash on her while she has no hurtbox.
        flash: look.flash && !matches!(mode, PileMode::Defeated | PileMode::Reveal),
        x: boss.x(),
    })
}

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
    let floor_y = boss.setup().floor_y.round() as i32;
    let mode = boss.mode();
    let look = HoarderLook {
        reveal_ticks: boss.reveal_ticks(),
        ..look
    };
    let hidden = matches!(mode, PileMode::Dormant | PileMode::Reveal);
    let mut art = HoarderArt {
        hoarder: hoarder_sprite(boss, tick, look),
        ..HoarderArt::default()
    };
    art.motes = reveal_motes(
        mode,
        boss.mode_tick(),
        boss.x(),
        boss.setup().floor_y,
        look.out_age,
    );

    // The room's dressing and the lair mound, behind everything.
    art.props.extend(heap_props(floor_y));
    art.props.extend(mound_props(
        i32::from(boss.tuning().hrdq19_home_x),
        floor_y,
        hidden,
    ));

    // The sidecar's statics: bank A (a solid) and the J-B post (a hit target) with the stack it
    // holds, each by its machine's state (HRD-B, HRD-C).
    let (statics, shadow) =
        static_props(encounter, combat.env(), layout_h, tick, look.released_age);
    art.props.extend(statics);
    art.shadow = shadow;

    // The exit door: the sidecar's lock solid, closed until her defeat tick, then open.
    let door_open = enemy.door_open();
    for lock in &encounter.locks {
        art.props.push(door_prop(lock.solid, door_open, layout_h));
    }

    // The entry gate (HRD-Q41, boss data): bars while closed.
    let t = boss.tuning();
    if t.hrdq41_gate_enabled {
        art.props.push(gate_prop(
            i32::from(t.hrdq41_gate_x),
            floor_y,
            boss.gate_solid().is_some(),
        ));
    }
    debug_assert!(art.props.len() <= MAX_PROPS && art.motes.len() <= MAX_MOTES);
    Some(art)
}

/// Bank A by its slide machine (HRD-B): standing (with the 2-frame shudder while disturbed) on its
/// solid, the tumbling mass on the sim's moving rectangle, the remnant on the sim's remnant. A room
/// without the machine draws it standing.
fn bank_from(
    rect: crate::camera_lab::room::IRect,
    slide: Option<&Slide>,
    layout_h: i32,
    tick: u64,
) -> PropDraw {
    let state = slide.map_or(SlideState::Stable, Slide::state);
    let (state, at) = match state {
        SlideState::Stable => (BankState::Stable, Some(rect)),
        SlideState::Disturbed => (BankState::Disturbed, Some(rect)),
        SlideState::Slid => (BankState::Slid, slide.and_then(Slide::mass_rect)),
        SlideState::Settled => (BankState::Settled, slide.and_then(Slide::remnant)),
    };
    bank_prop(at.unwrap_or(rect), state, layout_h, tick)
}

/// The J-B post and its stack by the falling mass (HRD-C): the post `intact`, `cracked` once weakened
/// and `broken` from the release on; the stack hanging, warning, falling on the sim's mass rectangle,
/// then the landed remnant. A room without the machine draws them standing.
fn post_and_stack_from(
    post: crate::camera_lab::room::IRect,
    mass: Option<&FallingMass>,
    layout_h: i32,
    tick: u64,
    released_age: Option<u64>,
) -> [PropDraw; 2] {
    let state = mass.map_or(MassState::Supported, FallingMass::state);
    let post_state = match state {
        MassState::Supported => PostState::Intact,
        MassState::Weakened => PostState::Cracked,
        MassState::Released | MassState::Falling | MassState::Landed => PostState::Broken,
    };
    let (stack_state, body) = match state {
        MassState::Supported | MassState::Weakened => {
            (StackState::Supported, mass.and_then(FallingMass::mass_rect))
        }
        MassState::Released => (StackState::Released, mass.and_then(FallingMass::mass_rect)),
        MassState::Falling => (StackState::Falling, mass.and_then(FallingMass::mass_rect)),
        MassState::Landed => (StackState::Landed, mass.and_then(FallingMass::remnant)),
    };
    let mut stack = stack_prop(
        body.unwrap_or_else(|| hanging_over(post)),
        stack_state,
        layout_h,
    );
    match (stack_state, mass) {
        // The warning: the shudder and the sag (presentation; the sim's mass has not moved yet).
        (StackState::Released, _) => {
            let (dx, dy) = stack_warning_offset(released_age.unwrap_or(0), tick);
            stack.x += dx;
            stack.y += dy;
        }
        // The fall starts from the sagged pose: the body is drawn no higher than the full sag, so it
        // never jumps back up on the first falling ticks (the sim's drop is 1 px after one tick).
        (StackState::Falling, Some(m)) => {
            let hanging = world(m.config().hanging, layout_h).1 - STACK_LEG_PX;
            stack.y = stack.y.min(hanging - STACK_SAG_MAX_PX);
        }
        _ => {}
    }
    [post_prop(post, post_state, layout_h), stack]
}

/// Whether the room's J-B stack is released (its warning) this tick: the fact the presenter's memo
/// folds into [`HoarderLook::released_age`] ([`HoarderMemo::observe_released`]).
pub fn stack_released(combat: &CombatState, encounter: &Encounter) -> bool {
    let Some(env) = combat.env() else {
        return false;
    };
    env.pile_props(encounter).iter().any(|(id, i)| {
        encounter
            .statics
            .iter()
            .any(|s| s.id == *id && s.kind == StaticKind::HitTarget)
            && env
                .mass(usize::from(*i))
                .is_some_and(|m| m.state() == MassState::Released)
    })
}

/// The sidecar's statics by their machines' states (`env` is `None` in a room without them): bank A
/// and the J-B post with its stack, plus the J-B footprint's floor shadow while the stack warns and
/// falls.
fn static_props(
    encounter: &Encounter,
    env: Option<&crate::combat_scene::EnvRuntime>,
    layout_h: i32,
    tick: u64,
    released_age: Option<u64>,
) -> (Vec<PropDraw>, Option<FloorShadow>) {
    let machines = env.map(|e| e.pile_props(encounter)).unwrap_or_default();
    let mut out = Vec::new();
    let mut shadow = None;
    for s in &encounter.statics {
        let index = machines
            .iter()
            .find(|(id, _)| *id == s.id)
            .map(|(_, i)| usize::from(*i));
        match s.kind {
            StaticKind::Solid => {
                let slide = index.and_then(|i| env.and_then(|e| e.slide(i)));
                out.push(bank_from(s.rect, slide, layout_h, tick));
            }
            StaticKind::HitTarget => {
                let mass = index.and_then(|i| env.and_then(|e| e.mass(i)));
                out.extend(post_and_stack_from(
                    s.rect,
                    mass,
                    layout_h,
                    tick,
                    released_age,
                ));
                if let Some(f) = mass.and_then(FallingMass::telegraph_rect) {
                    let (x, y, w, h) = world(f, layout_h);
                    shadow = Some(FloorShadow {
                        x,
                        y,
                        w,
                        h: h.min(FLOOR_SHADOW_PX),
                    });
                }
            }
        }
    }
    (out, shadow)
}

#[cfg(test)]
mod tests;
