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

use crate::prototype::Rect;
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

#[cfg(test)]
mod tests;
