//! Donut's presentation frame selection (Q10 melee, Magic Missile cast, bolt).
//!
//! Pure functions of the simulation's read-only views. The Floor 1 render
//! binary draws the result; nothing here is read back by the simulation, so no
//! replay hash can depend on it. Frame tables follow
//! `docs/02-art/donut-moveset-q10.md` section 7 and the two manifests
//! `assets/manifests/donut_h40_melee.json` and `donut_h40_missile.json`.

use crate::donut_moves::{MoveKind, MovePhase, MoveView};

/// Melee atlas (ANIM-DONUT-02): 15 frames of 72 x 44.
pub const MELEE_ATLAS: &str = "donut_h40_melee.png";
pub const MELEE_FRAMES: usize = 15;
/// Missile cast atlas (ANIM-DONUT-03): 3 phases x 16 locomotion frames.
pub const MISSILE_ATLAS: &str = "donut_h40_missile.png";
pub const MISSILE_FRAMES: usize = 48;
/// Both Donut atlases share this frame size (and her locomotion pivot).
pub const FRAME_W: u32 = 72;
pub const FRAME_H: u32 = 44;
/// The Magic Missile VFX atlas: 9 frames of 32 x 32.
pub const VFX_ATLAS: &str = "vfx_magic_missile.png";
pub const VFX_FRAMES: usize = 9;
pub const VFX_SIZE: u32 = 32;

/// The dry-press cue lasts this many ticks (presentation only).
pub const DRY_CUE_TICKS: u64 = 8;

/// Melee atlas frame of the pounce crouch (`p0`).
const POUNCE_CROUCH_FRAME: usize = 4;
/// Melee atlas frame of the pounce flight (`p1`).
const POUNCE_FLIGHT_FRAME: usize = 5;
/// Ticks the pin frame (`p2`) is shown from touchdown; `p3` follows
/// (manifest: p2 8 ticks, p3 12 ticks).
const PIN_TICKS: u16 = 8;
/// The flight frame's lowest opaque row is this many pixels above the pivot
/// row (row 41 against 43): the drawn pivot drops by it so the paws hang from
/// the body's bottom edge, like every airborne frame.
const FLIGHT_LIFT_PX: f32 = 2.0;

/// Swat frame by tick: `s0` 0-3, `s1` 4-6, `s2` 7-10, `s3` 11 and later (the
/// 4 / 3 / 4 / 3 table; a chained swat is a new instance that starts at tick 0
/// and therefore restarts at `s0`).
pub fn swat_frame(tick: u16) -> usize {
    match tick {
        0..=3 => 0,
        4..=6 => 1,
        7..=10 => 2,
        _ => 3,
    }
}

/// The melee atlas frame for the running move.
///
/// Pounce: `p0` crouch, `p1` flight (held until touchdown), `p2` pin for 8
/// ticks from touchdown, then `p3`. Where the manifest is ambiguous (the sim's
/// pin phase is only two ticks) the manifest's own tick table wins.
pub fn melee_frame(view: &MoveView) -> usize {
    match view.mv {
        MoveKind::Swat => swat_frame(view.tick),
        MoveKind::Pounce => match (view.phase, view.touchdown) {
            (MovePhase::Crouch, _) => POUNCE_CROUCH_FRAME,
            (_, Some(td)) => {
                if view.tick.saturating_sub(td) < PIN_TICKS {
                    6
                } else {
                    7
                }
            }
            (MovePhase::Pin, None) => 6,
            (MovePhase::Recovery, None) => 7,
            _ => POUNCE_FLIGHT_FRAME,
        },
    }
}

/// DP-Q34 (stand-in): the melee atlas frame of the blink's crouch, the existing
/// pounce crouch frame `p0`, for the first `crouch_ticks` ticks of the windup
/// (`windup_age` is `0` on the press tick, `None` outside a windup). Presentation
/// only; the arrival tick is not a crouch tick (she has moved by then).
pub fn blink_crouch_frame(windup_age: Option<u64>, crouch_ticks: u8) -> Option<usize> {
    windup_age
        .filter(|age| *age < u64::from(crouch_ticks))
        .map(|_| POUNCE_CROUCH_FRAME)
}

/// How far the drawn pivot drops below the body's foot for a melee frame.
pub fn melee_foot_drop(frame: usize) -> f32 {
    if frame == POUNCE_FLIGHT_FRAME {
        FLIGHT_LIFT_PX
    } else {
        0.0
    }
}

/// Cast phase of a cast age: `cast_gather` 0-3, `cast_release` 4-9,
/// `cast_follow` 10-15.
pub fn cast_phase(age: u16) -> usize {
    match age {
        0..=3 => 0,
        4..=9 => 1,
        _ => 2,
    }
}

/// Missile atlas index: `16 * phase + locomotion frame` (head swap).
pub fn cast_index(age: u16, locomotion_frame: usize) -> usize {
    (cast_phase(age) * 16 + locomotion_frame % 16).min(MISSILE_FRAMES - 1)
}

/// The dry cue: the `cast_gather` head on the locomotion frame for
/// [`DRY_CUE_TICKS`] after a dry press.
pub fn dry_cue_index(tick: u64, dry_tick: Option<u64>, locomotion_frame: usize) -> Option<usize> {
    let dry = dry_tick?;
    (tick >= dry && tick - dry < DRY_CUE_TICKS).then(|| cast_index(0, locomotion_frame))
}

/// VFX frame of a bolt that has travelled `travelled` px at `speed` px per
/// tick: the launch frame (1) on its spawn tick, then the loop 2, 3, 0 at 4
/// ticks each.
pub fn bolt_frame(travelled: f32, speed: f32) -> usize {
    let age = if speed > 0.0 {
        (travelled / speed).round() as u64
    } else {
        0
    };
    if age == 0 {
        return 1;
    }
    [2, 3, 0][(((age - 1) / 4) % 3) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(mv: MoveKind, tick: u16, phase: MovePhase, touchdown: Option<u16>) -> MoveView {
        MoveView {
            mv,
            tick,
            instance: 1,
            phase,
            launched: touchdown.is_some() || phase == MovePhase::Flight,
            touchdown,
            hit: false,
        }
    }

    #[test]
    fn swat_follows_the_4_3_4_3_table() {
        let frames: Vec<usize> = (0..14).map(swat_frame).collect();
        assert_eq!(frames, [0, 0, 0, 0, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3]);
    }

    #[test]
    fn pounce_frames_follow_the_phases() {
        let m = |t, p, td| melee_frame(&view(MoveKind::Pounce, t, p, td));
        assert_eq!(m(0, MovePhase::Crouch, None), 4);
        assert_eq!(m(9, MovePhase::Crouch, None), 4);
        assert_eq!(m(10, MovePhase::Flight, None), 5);
        assert_eq!(m(23, MovePhase::Flight, None), 5);
        assert_eq!(m(24, MovePhase::Pin, Some(24)), 6);
        assert_eq!(m(31, MovePhase::Recovery, Some(24)), 6);
        assert_eq!(m(32, MovePhase::Recovery, Some(24)), 7);
    }

    #[test]
    fn only_the_flight_frame_drops_the_pivot() {
        assert_eq!(melee_foot_drop(5), 2.0);
        assert_eq!(melee_foot_drop(4), 0.0);
        assert_eq!(melee_foot_drop(7), 0.0);
    }

    #[test]
    fn cast_phases_index_the_head_swap_atlas() {
        assert_eq!(cast_index(0, 3), 3);
        assert_eq!(cast_index(3, 3), 3);
        assert_eq!(cast_index(4, 3), 19);
        assert_eq!(cast_index(9, 15), 31);
        assert_eq!(cast_index(10, 0), 32);
        assert_eq!(cast_index(15, 15), 47);
    }

    #[test]
    fn the_dry_cue_lasts_eight_ticks() {
        assert_eq!(dry_cue_index(100, None, 2), None);
        assert_eq!(dry_cue_index(100, Some(100), 2), Some(2));
        assert_eq!(dry_cue_index(107, Some(100), 2), Some(2));
        assert_eq!(dry_cue_index(108, Some(100), 2), None);
        assert_eq!(dry_cue_index(99, Some(100), 2), None);
    }

    #[test]
    fn the_bolt_plays_the_launch_frame_then_the_loop() {
        let f = |age: u32| bolt_frame(age as f32 * 6.0, 6.0);
        assert_eq!(f(0), 1);
        let loop_frames: Vec<usize> = (1..=13).map(f).collect();
        assert_eq!(loop_frames, [2, 2, 2, 2, 3, 3, 3, 3, 0, 0, 0, 0, 2]);
        assert_eq!(bolt_frame(5.0, 0.0), 1);
    }

    /// DP-Q34: the crouch is the pounce crouch frame for the first 6 ticks of the
    /// windup (ages 0..=5) and nothing on the arrival tick (age 6) or outside one.
    #[test]
    fn the_blink_crouch_is_the_pounce_crouch_for_six_ticks() {
        for age in 0..6 {
            assert_eq!(blink_crouch_frame(Some(age), 6), Some(4), "age {age}");
        }
        assert_eq!(blink_crouch_frame(Some(6), 6), None);
        assert_eq!(blink_crouch_frame(None, 6), None);
        assert_eq!(blink_crouch_frame(Some(0), 0), None);
    }
}
