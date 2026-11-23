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
    /// 1 once revealed; ramps up during the first ticks of the reveal.
    pub alpha: f32,
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
}

/// The tones of the stand-ins, in Floor 01's brick and junk colours.
pub const PALETTE: [[u8; 4]; 8] = [
    [0x3a, 0x38, 0x3c, 0xff], // DoorClosed
    [0x8a, 0x4a, 0x2c, 0xff], // DoorFrame
    [0x5c, 0x58, 0x4e, 0xff], // GateBar
    [0x8a, 0x4a, 0x2c, 0x50], // GateOpen (the open gate's footprint, faint)
    [0x5e, 0x4a, 0x3a, 0xff], // BankBody
    [0x7a, 0x60, 0x48, 0xff], // BankTop
    [0x6a, 0x4a, 0x2c, 0xff], // Post
    [0x4a, 0x3a, 0x30, 0xff], // Mound
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
    Mound,
}

impl TagKind {
    pub const ALL: [Self; 6] = [
        Self::Hoarder,
        Self::Door,
        Self::Gate,
        Self::BankA,
        Self::PostB,
        Self::Mound,
    ];

    pub const fn text(self) -> &'static str {
        match self {
            Self::Hoarder => STAND_IN_LABEL,
            Self::Door => "DOOR",
            Self::Gate => "GATE",
            Self::BankA => "BANK A",
            Self::PostB => "POST B",
            Self::Mound => "MOUND",
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
pub const MAX_PIECES: usize = 24;

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
const MOUND_H_PX: i32 = 22;

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
        let alpha = if mode == PileMode::Reveal {
            (boss.mode_tick() as f32 / REVEAL_FADE_TICKS as f32).clamp(0.0, 1.0)
        } else {
            1.0
        };
        art.hoarder = Some(HoarderDraw {
            centre: actor_center(SPEC, boss.x() + dx, floor_y, flip_x),
            flip_x,
            frame: frame_for(mode, boss.mode_tick(), tick, look),
            alpha,
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
        "lair mound (stand-in)",
    ));
    // Its label shows while she is under it (Dormant) and rising out of it (Reveal); once she
    // stands in front of it the label would sit across her legs.
    if matches!(mode, PileMode::Dormant | PileMode::Reveal) {
        art.tags.push(Tag {
            kind: TagKind::Mound,
            centre_x: home as f32,
            bottom_y: (my + MOUND_H_PX + 2) as f32,
        });
    }

    // The sidecar's statics: bank A (a solid) and the J-B post (a hit target).
    for s in &encounter.statics {
        let r = s.rect;
        let (x, y, w, h) = (r.x, layout_h - r.bottom(), r.w, r.h);
        match s.kind {
            StaticKind::Solid => {
                art.pieces.push(piece(
                    x,
                    y,
                    w,
                    h,
                    Tone::BankBody,
                    Layer::Prop,
                    "bank A (stand-in)",
                ));
                art.pieces.push(piece(
                    x,
                    y + h - 4,
                    w,
                    4,
                    Tone::BankTop,
                    Layer::Prop,
                    "bank A top (stand-in)",
                ));
                art.pieces.push(piece(
                    x + w / 4,
                    y + h / 3,
                    w / 2,
                    3,
                    Tone::BankTop,
                    Layer::Prop,
                    "bank A junk (stand-in)",
                ));
                art.tags.push(Tag {
                    kind: TagKind::BankA,
                    centre_x: (x + w / 2) as f32,
                    bottom_y: (y + h / 2 - 2) as f32,
                });
            }
            StaticKind::HitTarget => {
                art.pieces.push(piece(
                    x,
                    y,
                    w,
                    h,
                    Tone::Post,
                    Layer::Prop,
                    "J-B post (stand-in)",
                ));
                art.tags.push(Tag {
                    kind: TagKind::PostB,
                    centre_x: (x + w / 2) as f32,
                    bottom_y: (y + h + 3) as f32,
                });
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

// ------------------------------------------------------------------- labels --

/// The 3 x 5 glyphs of the labels: five rows of three columns, `#` set.
const GLYPHS: &[(char, [&str; 5])] = &[
    ('A', [".#.", "#.#", "###", "#.#", "#.#"]),
    ('B', ["##.", "#.#", "##.", "#.#", "##."]),
    ('D', ["##.", "#.#", "#.#", "#.#", "##."]),
    ('E', ["###", "#..", "##.", "#..", "###"]),
    ('G', [".##", "#..", "#.#", "#.#", ".##"]),
    ('H', ["#.#", "#.#", "###", "#.#", "#.#"]),
    ('I', ["###", ".#.", ".#.", ".#.", "###"]),
    ('K', ["#.#", "#.#", "##.", "#.#", "#.#"]),
    ('L', ["#..", "#..", "#..", "#..", "###"]),
    ('M', ["#.#", "###", "###", "#.#", "#.#"]),
    ('N', ["##.", "#.#", "#.#", "#.#", "#.#"]),
    ('O', [".#.", "#.#", "#.#", "#.#", ".#."]),
    ('P', ["##.", "#.#", "##.", "#..", "#.."]),
    ('R', ["##.", "#.#", "##.", "#.#", "#.#"]),
    ('S', [".##", "#..", ".#.", "..#", "##."]),
    ('T', ["###", ".#.", ".#.", ".#.", ".#."]),
    ('U', ["#.#", "#.#", "#.#", "#.#", "###"]),
    ('0', ["###", "#.#", "#.#", "#.#", "###"]),
    ('2', ["##.", "..#", ".#.", "#..", "###"]),
    ('-', ["...", "...", "###", "...", "..."]),
    (' ', ["...", "...", "...", "...", "..."]),
];

/// Glyph advance (3 columns and 1 blank) and height of the label font, px.
pub const GLYPH_ADVANCE_PX: u32 = 4;
pub const GLYPH_HEIGHT_PX: u32 = 5;
/// The label ink (amber) and its 1 px drop shadow, so a label reads on any background.
pub const TAG_INK: [u8; 4] = [0xff, 0xd0, 0x70, 0xff];
pub const TAG_SHADOW: [u8; 4] = [0x10, 0x0c, 0x0a, 0xe0];

/// A rasterised label: `w x h` RGBA8, row-major, top row first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextImage {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

/// Rasterise `text` in the 3 x 5 label font with a 1 px shadow (one more column and row).
/// `None` when a character has no glyph.
pub fn text_pixels(text: &str) -> Option<TextImage> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return None;
    }
    let w = chars.len() as u32 * GLYPH_ADVANCE_PX; // advance includes the shadow column
    let h = GLYPH_HEIGHT_PX + 1;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let mut put = |x: u32, y: u32, c: [u8; 4], over: bool| {
        let i = ((y * w + x) * 4) as usize;
        if over || rgba[i + 3] == 0 {
            rgba[i..i + 4].copy_from_slice(&c);
        }
    };
    for (n, ch) in chars.iter().enumerate() {
        let glyph = GLYPHS.iter().find(|(c, _)| c == ch)?.1;
        let ox = n as u32 * GLYPH_ADVANCE_PX;
        for (row, line) in glyph.iter().enumerate() {
            for (col, cell) in line.bytes().enumerate() {
                if cell == b'#' {
                    // The shadow first (it never overwrites ink), then the ink on top.
                    put(ox + col as u32 + 1, row as u32 + 1, TAG_SHADOW, false);
                }
            }
        }
    }
    for (n, ch) in chars.iter().enumerate() {
        let glyph = GLYPHS.iter().find(|(c, _)| c == ch)?.1;
        let ox = n as u32 * GLYPH_ADVANCE_PX;
        for (row, line) in glyph.iter().enumerate() {
            for (col, cell) in line.bytes().enumerate() {
                if cell == b'#' {
                    put(ox + col as u32, row as u32, TAG_INK, true);
                }
            }
        }
    }
    Some(TextImage { w, h, rgba })
}

#[cfg(test)]
mod tests;
