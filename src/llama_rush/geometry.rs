//! The rush's geometry: the box, the motion, the lanes and the leash clamp
//! (CRL-Q08, Q19, Q32, Q43). Every function is pure and takes the solids it
//! sees; none reads the scene.
//!
//! `engineer::ram_box` takes `&EngineerTuning` and `lane_clear` uses the goblin
//! half-width, so neither is reused (F8). The llama's body is the LLAMA-01 box
//! (46 x 39, centred 3 px behind the pivot, away from the way it faces).

use super::{REMNANT_MAX_PX, RushProfile};
use crate::combat::solid_at;
use crate::feel::{
    GROUND_FRICTION, LLAMA_GLOB_HALF_H, LLAMA_GLOB_HALF_W, LLAMA_GLOB_MOUTH_DX, LLAMA_GLOB_MOUTH_DY,
};
use crate::llama::{LLAMA_BODY_BACK_SHIFT, LLAMA_HALF_HEIGHT, LLAMA_HALF_WIDTH};
use crate::prototype::{Block, Rect, TICK_SECONDS};

/// A corral llama's authored leash interval on its pivot x (CRL-Q28): it
/// replaces `home_x +- LLAMA_LEASH` for this llama only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Leash {
    pub min: f32,
    pub max: f32,
}

impl Leash {
    pub fn contains(self, x: f32) -> bool {
        x >= self.min && x <= self.max
    }
}

/// A solid for a llama: intact and taller than a prop remnant (CRL-Q32).
pub fn llama_solid(block: &Block) -> bool {
    block.intact && block.rect.height > REMNANT_MAX_PX
}

/// The 46 x 39 body of a llama at pivot `x` with its feet at `feet`.
pub fn llama_body(x: f32, feet: f32, facing: i8) -> Rect {
    Rect {
        left: x - LLAMA_HALF_WIDTH - LLAMA_BODY_BACK_SHIFT * f32::from(facing),
        bottom: feet,
        width: LLAMA_HALF_WIDTH * 2.0,
        height: LLAMA_HALF_HEIGHT * 2.0,
    }
}

/// The rush box in the world: `dir` is +1 or -1, `feet` the llama's floor line.
pub fn rush_box(p: &RushProfile, x: f32, dir: i8, feet: f32) -> Rect {
    let b = &p.crlq08_box;
    let (x0, x1) = (f32::from(b.x0), f32::from(b.x1));
    let (left, right) = if dir >= 0 {
        (x + x0, x + x1)
    } else {
        (x - x1, x - x0)
    };
    Rect {
        left,
        bottom: feet + f32::from(b.y0),
        width: right - left,
        height: f32::from(b.y1 - b.y0),
    }
}

/// `wanted` clamped to `[lo, hi]`, with the interval widened to contain `x`,
/// so a clamp never moves a llama that is already outside it (no teleport).
pub fn clamp_step(x: f32, wanted: f32, lo: f32, hi: f32) -> f32 {
    wanted.clamp(lo.min(x), hi.max(x))
}

/// What one active rush tick of motion did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RushMotion {
    pub x: f32,
    /// The box (or the body, for a solid above the box) met an intact solid.
    pub met: bool,
    /// The index in `solids` of the nearest face met.
    pub met_block: Option<usize>,
    /// The step was cut by the leash interval or the layout edge.
    pub leash_stop: bool,
}

/// One active rush tick of motion: the llama moves a whole step forward. When
/// the box front would pass the face of an intact solid, it is clamped so the
/// front rests on the face (never backward) and the tick is `met`; a body
/// meeting a high solid stops it too. Then the step is clamped to the leash and
/// the layout edge (`leash_stop`, CRL-Q43). Remnants of 16 px or less are not
/// solids for a llama (CRL-Q32).
pub fn rush_motion(
    p: &RushProfile,
    x: f32,
    dir: i8,
    feet: f32,
    solids: &[Block],
    leash: Leash,
    max_x: f32,
) -> RushMotion {
    let step = f32::from(dir) * p.px_per_tick();
    let candidate = x + step;
    let boxed = rush_box(p, candidate, dir, feet);
    let mut face: Option<(f32, usize)> = None;
    for (index, block) in solids.iter().enumerate() {
        if !llama_solid(block) || !boxed.overlaps(block.rect) {
            continue;
        }
        let f = if dir >= 0 {
            block.rect.left
        } else {
            block.rect.right()
        };
        let nearer = match face {
            None => true,
            Some((g, _)) if dir >= 0 => f < g,
            Some((g, _)) => f > g,
        };
        if nearer {
            face = Some((f, index));
        }
    }
    let (reached, met, met_block) = if let Some((face, index)) = face {
        let front = f32::from(p.crlq08_box.x1);
        let stop = if dir >= 0 { face - front } else { face + front };
        let (lo, hi) = if dir >= 0 {
            (x, candidate)
        } else {
            (candidate, x)
        };
        (stop.clamp(lo, hi), true, Some(index))
    } else {
        let body = llama_body(candidate, feet, dir);
        match solids
            .iter()
            .position(|b| llama_solid(b) && body.overlaps(b.rect))
        {
            Some(index) => (x, true, Some(index)),
            None => (candidate, false, None),
        }
    };
    // The leash interval and the layout edge, forward only.
    let lo = leash.min.max(LLAMA_HALF_WIDTH);
    let hi = leash.max.min(max_x);
    let limited = clamp_step(x, reached, lo, hi);
    RushMotion {
        x: limited,
        met,
        met_block,
        leash_stop: limited != reached,
    }
}

/// CRL-Q19: the rush lane is clear when no intact solid, other than the
/// breakable partition (`partition`, an index into `solids`), lies between the
/// llama's body front and the target's column at the box's heights. Remnants of
/// 16 px or less do not count (CRL-Q32).
pub fn rush_lane_clear(
    p: &RushProfile,
    x: f32,
    feet: f32,
    dir: i8,
    target_x: f32,
    solids: &[Block],
    partition: Option<usize>,
) -> bool {
    let front = x + f32::from(dir) * (LLAMA_HALF_WIDTH - LLAMA_BODY_BACK_SHIFT);
    let (lo, hi) = if dir >= 0 {
        (front, target_x)
    } else {
        (target_x, front)
    };
    if hi <= lo {
        return true;
    }
    let lane = Rect {
        left: lo,
        bottom: feet + f32::from(p.crlq08_box.y0),
        width: hi - lo,
        height: f32::from(p.crlq08_box.y1 - p.crlq08_box.y0),
    };
    !solids
        .iter()
        .enumerate()
        .any(|(i, b)| Some(i) != partition && llama_solid(b) && lane.overlaps(b.rect))
}

/// The glob's height band `[feet + 42, feet + 48)` (mouth 45 up, half height 3)
/// against a hurtbox, strictly.
pub fn glob_band_overlaps(feet: f32, hurtbox: Rect) -> bool {
    let band = Rect {
        left: hurtbox.left,
        bottom: feet + LLAMA_GLOB_MOUTH_DY - LLAMA_GLOB_HALF_H,
        width: hurtbox.width,
        height: LLAMA_GLOB_HALF_H * 2.0,
    };
    band.overlaps(hurtbox)
}

/// CRL-Q11: the glob path is clear when no intact solid lies in the band
/// between the mouth (46 px ahead of the pivot) and the target's column. Every
/// intact solid stops a glob, remnants included, though a remnant never reaches
/// the band.
pub fn glob_lane_clear(x: f32, facing: i8, feet: f32, target_x: f32, solids: &[Block]) -> bool {
    let mouth = x + f32::from(facing) * LLAMA_GLOB_MOUTH_DX;
    let (lo, hi) = if facing >= 0 {
        (mouth - LLAMA_GLOB_HALF_W, target_x)
    } else {
        (target_x, mouth + LLAMA_GLOB_HALF_W)
    };
    if hi <= lo {
        return true;
    }
    let lane = Rect {
        left: lo,
        bottom: feet + LLAMA_GLOB_MOUTH_DY - LLAMA_GLOB_HALF_H,
        width: hi - lo,
        height: LLAMA_GLOB_HALF_H * 2.0,
    };
    !solid_at(solids, lane)
}

/// One walking step toward `dir` at the profile's speed (`Action::Walk`): it
/// moves only while the candidate stays inside the leash and clear of a solid
/// (the legacy rule), and never backward.
pub fn walk_step(
    p: &RushProfile,
    x: f32,
    dir: i8,
    feet: f32,
    solids: &[Block],
    leash: Leash,
) -> f32 {
    let candidate = x + f32::from(dir) * p.walk_px_per_tick();
    let body = llama_body(candidate, feet, dir);
    let blocked = solids
        .iter()
        .any(|b| llama_solid(b) && body.overlaps(b.rect));
    if leash.contains(candidate) && !blocked {
        candidate
    } else {
        x
    }
}

/// CRL-Q43: `Llama::slide` of a corral-profile llama: the knockback, kick push
/// and death slide are also clamped to the authored leash interval, in every
/// mode (`Down` and `Corpse` included). When a clamp stops the llama, `vx`
/// becomes 0. Returns `(x, vx)`.
pub fn corral_slide(
    x: f32,
    vx: f32,
    feet: f32,
    facing: i8,
    solids: &[Block],
    max_x: f32,
    leash: Leash,
) -> (f32, f32) {
    if vx == 0.0 {
        return (x, vx);
    }
    let wanted = x + vx * TICK_SECONDS;
    let edge = wanted.clamp(LLAMA_HALF_WIDTH, max_x);
    let candidate = clamp_step(x, edge, leash.min, leash.max);
    let body = llama_body(candidate, feet, facing);
    let blocked = solids
        .iter()
        .any(|b| llama_solid(b) && body.overlaps(b.rect));
    let new_x = if blocked { x } else { candidate };
    if blocked || candidate != wanted {
        return (new_x, 0.0);
    }
    let drag = GROUND_FRICTION * TICK_SECONDS;
    let vx = if vx.abs() <= drag {
        0.0
    } else {
        vx - drag * vx.signum()
    };
    (new_x, vx)
}

/// Reach maths of the design (section 6), for tests and tools: the first active
/// tick on which the box overlaps a standing hero of half-width `h` at pivot
/// distance `d`, or `None` when the rush never reaches it.
pub fn contact_tick(p: &RushProfile, d: f32, h: f32) -> Option<u16> {
    let step = p.px_per_tick();
    let front = f32::from(p.crlq08_box.x1);
    (1..=p.crlq07_active_ticks).find(|&k| step * f32::from(k) + front > d - h)
}

/// The bonk tick for a solid face `face` px ahead of the start pivot, or `None`
/// when the rush runs out first.
pub fn bonk_tick(p: &RushProfile, face: f32) -> Option<u16> {
    let step = p.px_per_tick();
    let front = f32::from(p.crlq08_box.x1);
    (1..=p.crlq07_active_ticks).find(|&k| step * f32::from(k) + front > face)
}
