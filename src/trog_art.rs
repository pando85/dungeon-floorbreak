//! TROG-A atlas presentation, the pure half (TROG-AD39..AD41): which TROG-01/02/03 frame a
//! troglodyte draws, where, mirrored or not. A function of the domain state through **shared
//! references only**, so nothing here reaches the simulation, the replay or the state hash (a
//! test proves it with a state-hash comparison). The Bevy half is
//! `src/bin/floor01_scene/trog_art.rs`. The atlases are the committed
//! `prototype_fixture_unapproved` candidates (art review pending), exported by
//! `scripts/export_assets.sh`; without the export the presenter keeps the labelled hurtbox
//! rectangles.
//!
//! **Frame map** (TROG-AD39, the manifests' `playback` answers every case but the attack
//! split, which the sim's windows answer): `idle` (frames 0..=2, looped on their ticks) for
//! Dormant, Rest and a standing Approach; `walk` (3..=6) while its x changed in the last 6
//! ticks; the tell is frame 7 for the manifest's onset ticks and frame 8 after; the active
//! window is frame 9 before the hitbox is live and frame 10 while it is; Recover is frame 11
//! (`attack_land` or `attack_recover`); Hurt is 12; Down plays 13, 14 on their ticks and holds
//! 15. The Pygmy's frames are drawn at its **airborne pivot** (TROG-AD25), so the leap needs
//! no other offset. The art faces right: a facing of -1 mirrors it (TROG-AD40: the pivot
//! mirrors with the frame).

use crate::env_prop_art::{FALL_SHADOW, FALL_SHADOW_TELL};
use crate::prototype::{Block, Rect};
use crate::trog::{Species, State, Trog, TrogTuning};

/// The sheet's frames (one row).
pub const FRAMES: u32 = 16;

/// `generated/art/trog_0N.png`, exported by `scripts/export_assets.sh`.
pub const fn atlas(species: Species) -> &'static str {
    match species {
        Species::Pygmy => "generated/art/trog_01.png",
        Species::Basher => "generated/art/trog_02.png",
        Species::Virtuoso => "generated/art/trog_03.png",
    }
}

/// The frame size, from the manifests.
pub const fn frame_size(species: Species) -> (u32, u32) {
    match species {
        Species::Pygmy => (80, 56),
        Species::Basher => (112, 88),
        Species::Virtuoso => (128, 88),
    }
}

/// The pivot inside a frame (x from the left, y from the top), from the manifests.
pub const fn pivot(species: Species) -> (u32, u32) {
    match species {
        Species::Pygmy => (30, 55),
        Species::Basher => (48, 87),
        Species::Virtuoso => (44, 87),
    }
}

/// The ticks each frame is shown (`0`: held), from the manifests' `frames` (a test compares).
pub const fn frame_ticks(species: Species) -> [u16; 16] {
    match species {
        Species::Pygmy => [12, 8, 12, 6, 6, 6, 6, 6, 18, 3, 8, 14, 12, 4, 8, 0],
        Species::Basher => [14, 10, 14, 8, 8, 8, 8, 8, 28, 3, 8, 20, 14, 5, 10, 0],
        Species::Virtuoso => [12, 10, 12, 8, 8, 8, 8, 6, 24, 2, 6, 18, 14, 5, 10, 0],
    }
}

/// The presenter's render-local knowledge of one troglodyte (never read by the simulation).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrogLook {
    /// Its x changed in the last few ticks.
    pub moving: bool,
    /// Ticks since it fell, `None` while it lives.
    pub down_age: Option<u64>,
}

/// One troglodyte's draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrogDraw {
    pub frame: u16,
    pub flip_x: bool,
    /// The sprite's centre in world px (y up).
    pub centre: (f32, f32),
}

/// Walk ticks the presenter keeps `moving` after the last change of x.
pub const MOVE_HOLD_TICKS: u64 = 6;

/// The frame of a looped run of frames `first..=last` at presenter tick `tick`.
fn looped(ticks: &[u16; 16], first: usize, last: usize, tick: u64) -> u16 {
    let total: u64 = ticks[first..=last].iter().map(|t| u64::from(*t)).sum();
    let mut at = tick % total.max(1);
    for (i, t) in ticks.iter().enumerate().take(last + 1).skip(first) {
        if at < u64::from(*t) {
            return i as u16;
        }
        at -= u64::from(*t);
    }
    first as u16
}

/// The atlas frame of a troglodyte in its state.
pub fn frame(t: &TrogTuning, trog: &Trog, tick: u64, look: TrogLook) -> u16 {
    let ticks = frame_ticks(trog.species);
    let sp = t.species(trog.species);
    let a = trog.attack_tick();
    match trog.state {
        State::Dormant | State::Rest => looped(&ticks, 0, 2, tick),
        State::Approach if look.moving => looped(&ticks, 3, 6, tick),
        State::Approach => looped(&ticks, 0, 2, tick),
        State::Tell => {
            if a < ticks[7] {
                7
            } else {
                8
            }
        }
        State::Active => {
            if a < sp.strike_start {
                9
            } else {
                10
            }
        }
        State::Recover => 11,
        State::Hurt => 12,
        State::Down => {
            let age = look.down_age.unwrap_or(u64::MAX);
            if age < u64::from(ticks[13]) {
                13
            } else if age < u64::from(ticks[13]) + u64::from(ticks[14]) {
                14
            } else {
                15
            }
        }
    }
}

/// Where a troglodyte's sprite stands: the frame's pivot on the troglodyte's pivot (feet plus
/// the Pygmy's lift), mirrored with the facing.
pub fn draw(t: &TrogTuning, trog: &Trog, tick: u64, look: TrogLook) -> TrogDraw {
    let (w, h) = frame_size(trog.species);
    let (px, py) = pivot(trog.species);
    let dir = if trog.facing < 0 { -1.0 } else { 1.0 };
    let cx = trog.x + dir * (w as f32 * 0.5 - px as f32);
    // The pivot row is the bottom pixel row of the feet: the frame's bottom edge is below it.
    let cy = trog.foot_y + trog.lift() + (h as f32 * 0.5) - (h as f32 - 1.0 - py as f32);
    TrogDraw {
        frame: frame(t, trog, tick, look),
        flip_x: trog.facing < 0,
        centre: (cx, cy),
    }
}

/// The sprite's world rectangle (the debug toggle and the tests compare it with the hurtbox).
pub fn sprite_rect(trog: &Trog, d: &TrogDraw) -> Rect {
    let (w, h) = frame_size(trog.species);
    Rect {
        left: d.centre.0 - w as f32 * 0.5,
        bottom: d.centre.1 - h as f32 * 0.5,
        width: w as f32,
        height: h as f32,
    }
}

// ------------------------------------------------------------------ the attack telegraph (TROG-FX-01)

/// TROG-FX-01: the telegraph of a committed attack is the approved Goblin Workshop weight-shadow sheet
/// (`f01-gw-tell-shadow`, AREA-GOBLINWORKSHOP-01: a dark dithered floor patch that flashes through its tell
/// and goes solid when the weight lands) cropped to the **exact committed rectangle** instead of a
/// translucent code-drawn box. The sheet is 64 x 8 with three frames (the tell's two dither phases, the
/// solid patch); its centre columns [`TELL_TILE_X`]..+[`TELL_TILE_W`] are a flat 8 x 8 tile of that
/// pattern (full height, the dither period divides the width), repeated left to right and clipped on the
/// last piece, so the drawn width is exactly the rectangle's and never overstates the threat. Timing is
/// the simulation's: [`Trog::threat`] is `Some` for the tell and the active window, the patch is solid
/// from the first tick the hitbox is live, and the dither flashes on the approved sheet's own clock
/// ([`FALL_SHADOW_TELL`], 4 ticks a phase). A pure function of the domain state: nothing here reaches
/// the simulation.
pub const TELL_SHEET: &str = "generated/art/f01-gw-tell-shadow.png";
/// One frame of the sheet.
pub const TELL_FRAME: (u32, u32) = (64, 8);
/// Frames of the sheet (the tell's two phases, then the solid patch).
pub const TELL_FRAMES: u32 = 3;
/// The tile's first column and width inside a frame.
pub const TELL_TILE_X: u32 = 28;
pub const TELL_TILE_W: u32 = 8;
/// The largest number of tile pieces a telegraph needs (the Pygmy's whole leap path, [`TELL_MAX_PX`]).
pub const TELL_MAX_PIECES: usize = 28;
/// The widest rectangle drawn (`TELL_MAX_PIECES` tiles); a wider threat keeps its first tiles only.
pub const TELL_MAX_PX: f32 = (TELL_MAX_PIECES as u32 * TELL_TILE_W) as f32;
/// The patch's alpha: the Workshop weight shadow's (`env_prop_art::SHADOW_ALPHA`).
pub const TELL_ALPHA: u8 = crate::env_prop_art::SHADOW_ALPHA;

/// One drawn telegraph: the patch's bottom-left corner in world px (y up), its width, and the sheet frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TellDraw {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// The sheet frame: 0 and 1 flash through the tell, 2 is the solid patch of a live hitbox.
    pub frame: u16,
}

/// The sheet frame of a telegraph at presenter `tick`: solid once the hitbox is live (`ticks_to_live`
/// is 0), the flashing dither before.
pub fn tell_frame(ticks_to_live: u16, tick: u64) -> u16 {
    let tag = if ticks_to_live == 0 {
        FALL_SHADOW
    } else {
        FALL_SHADOW_TELL
    };
    tag.frame_at(tick) as u16
}

/// The telegraph of `trog` at `tick`, `None` outside its tell and active window. The patch spans the
/// committed rectangle's x extent exactly. The floor-level attacks (the Pygmy's leap path, the Basher's
/// smash) sit on the floor under the rectangle; the Virtuoso's tongue band is mid-air, so its patch is
/// centred on the band's height (rounded down, never above the band).
pub fn tell_draw(t: &TrogTuning, trog: &Trog, solids: &[Block], tick: u64) -> Option<TellDraw> {
    let (rect, left) = trog.threat(t, solids)?;
    let height = TELL_FRAME.1 as f32;
    let y = match trog.species {
        Species::Pygmy | Species::Basher => trog.foot_y,
        Species::Virtuoso => rect.bottom + ((rect.height - height) * 0.5).floor().max(0.0),
    };
    Some(TellDraw {
        x: rect.left,
        y,
        width: rect.width.min(TELL_MAX_PX),
        height,
        frame: tell_frame(left, tick),
    })
}

/// One tile piece of a telegraph: its source columns inside the sheet frame and its offset from the
/// patch's left edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TellPiece {
    pub src_x: u32,
    pub dx: u32,
    pub width: u32,
}

/// The pieces that cover exactly `width` px (rounded to whole pixels, at least one), 8 px each and the
/// last clipped.
pub fn tell_pieces(width: f32) -> Vec<TellPiece> {
    let total = width.round().max(1.0) as u32;
    (0..total.div_ceil(TELL_TILE_W))
        .map(|k| TellPiece {
            src_x: TELL_TILE_X,
            dx: k * TELL_TILE_W,
            width: (total - k * TELL_TILE_W).min(TELL_TILE_W),
        })
        .collect()
}

#[cfg(test)]
mod tests;
