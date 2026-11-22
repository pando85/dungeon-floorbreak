//! HAZ-B: the **stand-in presentation** of the slime, the Rot Sticker, the acid
//! strip and the warning ring, the pure half
//! (`docs/01-design/enemy-gameplay-slime-rot.md` section 3 and HAZ-Q37, HAZ-Q38).
//! A function of the combat state through **shared references only**, it returns
//! what a presenter draws: which SLIME-01 or ROT-01 frame, where, mirrored or
//! turned, plus a few labelled code-drawn pieces (the acid strip at its current
//! bounds, the ring at the blast radius, the olive drip a spent sticker leaves).
//!
//! **Presentation only.** Nothing here mutates a state, draws a random number or
//! is hashed, replayed or read back by the simulation: a run that calls every
//! function here each tick has the same state hash as one that never does (a
//! test). Every output is a function of the machine states and the tick, so a
//! render that skips frames or a replay seek draws the same picture for the same
//! tick.
//!
//! Art: the SLIME-01 and ROT-01 atlases are the committed candidates
//! (`review_status: prototype_fixture_unapproved`); the timing below is the
//! sheet's playback retime of held frames, not a redraw (K10). The ring, the acid
//! strip and the drip are **code-drawn stand-ins**, labelled as such, shown in
//! every build until approved hazard VFX art exists (HAZ-Q38).
//!
//! Coordinates are world px, y up, like the simulation.

use crate::actor_sprite::{ActorSpec, actor_center};
use crate::camera_lab::room::IRect;
use crate::combat_anim::dynamite::ring_points;
use crate::combat_scene::slime::SlimeEnemy;
use crate::combat_scene::{CombatState, EnvRuntime};
use crate::rot_sticker::{self, Orientation, RotSticker, StickerState};
use crate::slime::{self, SlimePhase};

/// SLIME-01: 16 frames of 80 x 40, pivot (40, 39), authored facing right.
pub const SLIME_ATLAS: &str = "generated/art/slime-01.png";
pub const SLIME_FRAME_SIZE: (u32, u32) = (80, 40);
pub const SLIME_FRAMES: u32 = 16;
pub const SLIME_SPEC: ActorSpec = ActorSpec {
    name: "SLIME",
    width: 80,
    height: 40,
    pivot_x: 40,
    pivot_y: 39,
    shadow_width: 0,
    shadow_pivot_x: 0,
};
/// ROT-01: 15 frames of 32 x 32, pivot (6, 16), authored on the wall at its left
/// (the pleated face toward +x).
pub const ROT_ATLAS: &str = "generated/art/rot-01.png";
pub const ROT_FRAME_SIZE: (u32, u32) = (32, 32);
pub const ROT_FRAMES: u32 = 15;
/// The distance from the pivot (the dome centre) to the frame's centre, along
/// the face's normal.
pub const ROT_PIVOT_TO_CENTRE_PX: f32 = 10.0;

/// SLIME-01 frames (the `anim` tags of `slime-01.json`).
pub const SLIME_IDLE: [u8; 3] = [0, 1, 2];
pub const SLIME_CREEP: [u8; 4] = [3, 4, 5, 6];
pub const SLIME_TELL_RISE: u8 = 7;
pub const SLIME_TELL_HOLD: u8 = 8;
pub const SLIME_LAUNCH: u8 = 9;
pub const SLIME_IMPACT: u8 = 10;
pub const SLIME_RECOVER: u8 = 11;
pub const SLIME_HURT: u8 = 12;
pub const SLIME_DOWN: [u8; 3] = [13, 14, 15];
/// Ticks each idle frame is held (the atlas' 200, 133, 200 ms at 60 Hz).
const SLIME_IDLE_TICKS: [u64; 3] = [12, 8, 12];
/// Ticks each creep frame is held (133 ms).
const CREEP_TICKS: u64 = 8;
/// Ticks each `down` frame is held before the last one stays.
pub const SLIME_DOWN_TICKS: u64 = 6;

/// ROT-01 frames.
pub const ROT_IDLE: [u8; 3] = [0, 1, 2];
pub const ROT_WALK: [u8; 4] = [3, 4, 5, 6];
const ROT_IDLE_TICKS: [u64; 3] = [14, 12, 14];

/// What the presenter knows about a slime beyond the simulation's state: it is
/// render-local (a position delta, the tick it was first seen defeated).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SlimeLook {
    /// The slime changed x since the last frame: the creep loop plays.
    pub moving: bool,
    /// Ticks since the presenter first saw it defeated.
    pub defeated_age: Option<u64>,
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

/// The SLIME-01 frame of one slime (section 3): the idle and creep loops, the
/// rear-up tell (frame 7 for the half-rise, frame 8 held), the launch, the lunge
/// impact, the recover, the hurt frame while it is flashing and not attacking,
/// and the three `down` frames of its remains once defeated.
pub fn slime_frame(
    phase: SlimePhase,
    hurt: bool,
    defeated: bool,
    look: SlimeLook,
    tick: u64,
) -> u8 {
    if defeated {
        let step = look.defeated_age.map_or(0, |a| a / SLIME_DOWN_TICKS) as usize;
        return SLIME_DOWN[step.min(SLIME_DOWN.len() - 1)];
    }
    match phase {
        SlimePhase::TellRise => SLIME_TELL_RISE,
        SlimePhase::TellHold => SLIME_TELL_HOLD,
        SlimePhase::Launch => SLIME_LAUNCH,
        SlimePhase::Impact => SLIME_IMPACT,
        SlimePhase::Recover => SLIME_RECOVER,
        SlimePhase::Idle | SlimePhase::Cooldown if hurt => SLIME_HURT,
        SlimePhase::Idle | SlimePhase::Cooldown if look.moving => {
            SLIME_CREEP[(tick / CREEP_TICKS % SLIME_CREEP.len() as u64) as usize]
        }
        SlimePhase::Idle | SlimePhase::Cooldown => loop_frame(&SLIME_IDLE, &SLIME_IDLE_TICKS, tick),
    }
}

/// One slime to draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlimeDraw {
    /// The roster index of the slime (the presenter keeps one sprite each).
    pub index: usize,
    /// The sprite's centre (world px).
    pub centre: (f32, f32),
    pub flip_x: bool,
    pub frame: u8,
    /// The slime's hit flash is running.
    pub flash: bool,
    pub defeated: bool,
    /// The x it stands at (the presenter derives `moving` from its change).
    pub x: f32,
}

/// Every slime of the room in roster order. `look(index, x, defeated)` is the
/// presenter's render-local knowledge of that slime.
pub fn slimes(
    combat: &CombatState,
    tick: u64,
    mut look: impl FnMut(usize, f32, bool) -> SlimeLook,
) -> Vec<SlimeDraw> {
    let t = slime::active();
    let mut out = Vec::new();
    for (index, e) in combat.slot_enemies().iter().enumerate() {
        let Some(s) = e
            .as_any()
            .and_then(|a| a.downcast_ref::<SlimeEnemy>())
            .map(SlimeEnemy::slime)
        else {
            continue;
        };
        let defeated = s.is_defeated();
        let look = look(index, s.x, defeated);
        let phase = s.phase(t);
        let frame = slime_frame(
            phase,
            s.hurt_frame_left() > 0,
            defeated,
            look,
            tick + index as u64 * 5,
        );
        let flip_x = s.facing < 0;
        out.push(SlimeDraw {
            index,
            centre: actor_center(SLIME_SPEC, s.x, s.foot_y, flip_x),
            flip_x,
            frame,
            flash: s.flash_left() > 0,
            defeated,
            x: s.x,
        });
    }
    out
}

/// The ROT-01 frame of a sticker in `state` (section 3 and HAZ-Q37). `remaining`
/// is the ticks to the detonation while in Warning. `None` once spent: the body
/// is gone.
pub fn sticker_frame(
    state: StickerState,
    remaining: Option<u64>,
    tick: u64,
    index: usize,
) -> Option<u8> {
    match state {
        StickerState::Latched => Some(loop_frame(
            &ROT_IDLE,
            &ROT_IDLE_TICKS,
            tick + index as u64 * 7,
        )),
        StickerState::Scuttling => {
            Some(ROT_WALK[(tick / CREEP_TICKS % ROT_WALK.len() as u64) as usize])
        }
        StickerState::Warning => Some(rot_sticker::warning_frame(remaining.unwrap_or(0))),
        StickerState::Spent => None,
    }
}

/// Whether the ring at the blast radius is drawn: on `t..t+23` of a warning (every
/// tick with at least one left), not on the detonation tick (HAZ-Q38). A chained
/// sticker shows it for its last 8 ticks.
pub const fn ring_drawn(remaining: Option<u64>) -> bool {
    matches!(remaining, Some(r) if r >= 1)
}

/// One sticker to draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StickerDraw {
    /// The sidecar prop index.
    pub prop: u16,
    /// The sprite's centre (world px).
    pub centre: (f32, f32),
    pub flip_x: bool,
    /// Quarter turns clockwise (the ceiling variant of the wall set).
    pub quarter_turns_cw: u8,
    pub frame: u8,
}

/// The sprite placement of a sticker: the pivot (6, 16) is the dome centre, the
/// frame's centre is [`ROT_PIVOT_TO_CENTRE_PX`] along the face's normal. The set is
/// authored for the wall on the left (face +x); the wall on the right mirrors it,
/// the ceiling turns it a quarter clockwise (face down).
pub fn sticker_placement(orientation: Orientation, dome: (f32, f32)) -> ((f32, f32), bool, u8) {
    let d = ROT_PIVOT_TO_CENTRE_PX;
    match orientation {
        Orientation::WallLeft => ((dome.0 + d, dome.1), false, 0),
        Orientation::WallRight => ((dome.0 - d, dome.1), true, 0),
        Orientation::Ceiling => ((dome.0, dome.1 - d), false, 1),
    }
}

/// Every sticker of the room that has a body, in prop order. The detonation tick
/// itself still draws frame 10: the env step has already spent the sticker by the
/// time the frame is read, so the tick comes from its charge (`detonated` on
/// `tick`).
pub fn stickers(combat: &CombatState, tick: u64) -> Vec<StickerDraw> {
    let Some(env) = combat.env() else {
        return Vec::new();
    };
    let t = rot_sticker::active();
    let mut out = Vec::new();
    for (i, (prop, s, charge)) in env.stickers().enumerate() {
        let blast_now = combat
            .charges
            .get(charge)
            .is_some_and(|c| c.detonated && c.detonate_tick == tick);
        let frame = if s.state() == StickerState::Spent && blast_now {
            Some(rot_sticker::warning_frame(0))
        } else {
            sticker_frame(s.state(), s.remaining(tick), tick, i)
        };
        let Some(frame) = frame else {
            continue;
        };
        let (centre, flip_x, quarter_turns_cw) =
            sticker_placement(s.track().orientation, s.dome(t));
        out.push(StickerDraw {
            prop,
            centre,
            flip_x,
            quarter_turns_cw,
            frame,
        });
    }
    out
}

/// The tones of the code-drawn stand-ins (`[r, g, b, a]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// The acid strip's fill.
    Acid,
    /// Its bright lip.
    AcidLip,
    /// The ring at the blast radius.
    Ring,
    /// The olive drip a spent sticker leaves.
    Drip,
}

impl Tone {
    pub const fn rgba(self) -> [u8; 4] {
        match self {
            Self::Acid => [0x7a, 0xa8, 0x2a, 0xc8],
            Self::AcidLip => [0xc4, 0xee, 0x5a, 0xff],
            Self::Ring => [0xff, 0x6a, 0x2a, 0xe0],
            Self::Drip => [0x5c, 0x6a, 0x22, 0xd0],
        }
    }
}

/// One code-drawn rectangle (world px, y up, bottom-left origin).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub tone: Tone,
    /// What it stands in for (a label for the capture and the tests).
    pub label: &'static str,
}

/// The ring's dots: a thin ring at the blast radius drawn as this many 3 px dots.
pub const RING_DOTS: usize = 40;
const DOT_PX: i32 = 3;
/// The drip a spent sticker leaves: 2 px wide, 8 px long.
const DRIP_PX: (i32, i32) = (2, 8);

/// The most pieces [`pieces`] can return for `env`: the presenter's pool size.
pub fn max_pieces(env: &EnvRuntime) -> usize {
    env.acid().patches().count() * 2 + env.sticker_count() * (RING_DOTS + 1)
}

fn world_rect(r: IRect, layout_h: i32) -> (i32, i32, i32, i32) {
    (r.x, layout_h - r.bottom(), r.w, r.h)
}

/// The code-drawn pieces: the acid strip of each live patch at its **current**
/// bounds (a draining pool shrinks with its volume, honest bounds) with a
/// 1 px lip, the ring around each warning sticker's dome, and the drip of each
/// spent sticker. `layout_h` flips the room's layout rows into world rows.
pub fn pieces(env: &EnvRuntime, layout_h: i32, tick: u64) -> Vec<Piece> {
    let mut out = Vec::with_capacity(max_pieces(env));
    for patch in env.acid().patches() {
        let Some(rect) = patch.rect() else {
            continue;
        };
        let (x, y, w, h) = world_rect(rect, layout_h);
        out.push(Piece {
            x,
            y,
            w,
            h,
            tone: Tone::Acid,
            label: "acid strip (stand-in)",
        });
        out.push(Piece {
            x,
            y: y + h - 1,
            w,
            h: 1,
            tone: Tone::AcidLip,
            label: "acid lip (stand-in)",
        });
    }
    let t = rot_sticker::active();
    for (_, s, _) in env.stickers() {
        match s.state() {
            StickerState::Warning if t.on(t.hazq38_ring) && ring_drawn(s.remaining(tick)) => {
                ring(s, t, &mut out);
            }
            StickerState::Spent => out.push(drip(s, t)),
            _ => {}
        }
    }
    out
}

fn ring(s: &RotSticker, t: &rot_sticker::RotTuning, out: &mut Vec<Piece>) {
    let (cx, cy) = s.dome(t);
    let radius = f32::from(t.hazq30_outer_radius_px);
    for (dx, dy) in ring_points(radius, RING_DOTS) {
        out.push(Piece {
            x: cx.round() as i32 + dx - DOT_PX / 2,
            y: cy.round() as i32 + dy - DOT_PX / 2,
            w: DOT_PX,
            h: DOT_PX,
            tone: Tone::Ring,
            label: "rot blast ring (stand-in)",
        });
    }
}

/// The cosmetic drip on the surface a spent sticker leaves (never collidable).
fn drip(s: &RotSticker, t: &rot_sticker::RotTuning) -> Piece {
    let (cx, cy) = s.dome(t);
    let (w, h) = DRIP_PX;
    let (x, y, w, h) = match s.track().orientation {
        // On the ceiling: the drip runs down from the surface.
        Orientation::Ceiling => (
            cx.round() as i32 - w / 2,
            cy.round() as i32 + i32::from(t.hazq24_dome_offset_px) - h,
            w,
            h,
        ),
        // On a wall: it runs down the face from the dome's height.
        Orientation::WallLeft | Orientation::WallRight => {
            let face = if s.track().orientation == Orientation::WallLeft {
                cx.round() as i32 - i32::from(t.hazq24_dome_offset_px)
            } else {
                cx.round() as i32 + i32::from(t.hazq24_dome_offset_px) - w
            };
            (face, cy.round() as i32 - h, w, h)
        }
    };
    Piece {
        x,
        y,
        w,
        h,
        tone: Tone::Drip,
        label: "spent drip (stand-in)",
    }
}

#[cfg(test)]
mod tests;
