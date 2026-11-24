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

/// Melee atlas frames of the damage reactions (ANIM-DONUT-02, manifest
/// `melee.frames` 8 to 12): `h0` flinch, `h1` recoil, `kb` thrown, `d0` sploot
/// impact and `d1` sploot sulk. The get-up frames (13, 14) are not used: no
/// simulation state gets Donut up (the defeat ends the attempt and the retry
/// restarts it), the same as Carl's get-up.
const HURT_FLINCH_FRAME: usize = 8;
const HURT_RECOIL_FRAME: usize = 9;
const KNOCKBACK_FRAME: usize = 10;
const KNOCKDOWN_IMPACT_FRAME: usize = 11;
const KNOCKDOWN_SULK_FRAME: usize = 12;
/// Manifest ticks of `h0` (0 to 4) and of `d0` (0 to 5).
const HURT_FLINCH_TICKS: u32 = 5;
const KNOCKDOWN_IMPACT_TICKS: u32 = 6;
/// The knockback frame's lowest opaque row is this many pixels above the pivot
/// row (row 41 against 43), like the pounce flight frame.
const KNOCKBACK_LIFT_PX: f32 = 2.0;

/// What Donut's damage-reaction frame depends on, from one finished tick (the
/// same inputs as Carl's `CarlInput`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HurtInput {
    /// Ticks of the flinch left (counts down from `hurt_total`).
    pub hurt_ticks: u8,
    /// The flinch length of the blow (`PartyTuning::donut_hurt_ticks`).
    pub hurt_total: u8,
    /// Frozen in the hit-stop of a blow on her.
    pub frozen: bool,
    /// Donut is the defeated hero.
    pub defeated: bool,
    pub grounded: bool,
    /// Ticks since she touched down after the defeat (0 on the first grounded
    /// tick).
    pub landed_age: Option<u32>,
}

/// Donut's damage-reaction frame in the melee atlas, or `None` when no
/// reaction is drawn. Mirrors Carl's `carl_pose`:
///
/// * defeated: the hit-stop shows `h0`, then `kb` while she is in the air, then
///   `d0` (6 ticks) and `d1` (held) from touchdown;
/// * hurt: `h0` for the first 5 ticks of the flinch, then `h1`; the freeze of
///   the blow holds `h0`.
pub fn hurt_frame(input: &HurtInput) -> Option<usize> {
    if input.defeated {
        return Some(if input.frozen {
            HURT_FLINCH_FRAME
        } else if input.grounded {
            if input.landed_age.unwrap_or(0) < KNOCKDOWN_IMPACT_TICKS {
                KNOCKDOWN_IMPACT_FRAME
            } else {
                KNOCKDOWN_SULK_FRAME
            }
        } else {
            KNOCKBACK_FRAME
        });
    }
    if input.hurt_ticks > 0 {
        let age = u32::from(input.hurt_total.saturating_sub(input.hurt_ticks));
        return Some(if age < HURT_FLINCH_TICKS {
            HURT_FLINCH_FRAME
        } else {
            HURT_RECOIL_FRAME
        });
    }
    None
}

/// How far the drawn pivot drops below the body's foot for the
/// knockback frame: it blends from the pivot rule at the launch (`blend` 0) to
/// the lowest row on the body from the apex on (1), like Carl's knockdown, so the
/// thrown frame never hovers and the sploot lands without a pop.
pub fn knockback_foot_drop(frame: usize, blend: f32) -> f32 {
    if frame == KNOCKBACK_FRAME {
        KNOCKBACK_LIFT_PX * blend.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Contact-shadow footprint `(box width, box pivot column)` of the two lying
/// frames (`d0`, `d1`): their opaque extent in the 72 x 44 frame, columns 12 to
/// 68 with the pivot at column 46 (from the grid source; pinned by a test).
const KNOCKDOWN_SHADOW_FOOTPRINT: (i32, i32) = (57, 34);

/// The contact shadow of a damage-reaction frame: the lying frames (`d0`,
/// `d1`) cast the shadow of the body on the floor, like Carl's sprawl
/// (`SpriteClips::shadow_footprint`); every other frame keeps her idle
/// footprint (`None`). Presentation only.
pub fn hurt_shadow_footprint(frame: usize) -> Option<(i32, i32)> {
    matches!(frame, KNOCKDOWN_IMPACT_FRAME | KNOCKDOWN_SULK_FRAME)
        .then_some(KNOCKDOWN_SHADOW_FOOTPRINT)
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

    fn hurt(hurt_ticks: u8) -> HurtInput {
        HurtInput {
            hurt_ticks,
            hurt_total: 18,
            ..HurtInput::default()
        }
    }

    #[test]
    fn the_flinch_plays_h0_for_five_ticks_then_h1_and_ends_with_the_flinch() {
        // `hurt_ticks` counts down from 18: age 0 is 18 left.
        let frames: Vec<Option<usize>> = (0..=18u8)
            .rev()
            .map(|left| hurt_frame(&hurt(left)))
            .collect();
        assert_eq!(frames[0], Some(8), "age 0");
        assert_eq!(frames[4], Some(8), "age 4, the last h0 tick");
        assert_eq!(frames[5], Some(9), "age 5, the first h1 tick");
        assert_eq!(frames[17], Some(9), "age 17, the last flinch tick");
        assert_eq!(frames[18], None, "no flinch left: her locomotion");
        // A longer hurt tuning keeps the same age table.
        let long = HurtInput {
            hurt_ticks: 20,
            hurt_total: 20,
            ..HurtInput::default()
        };
        assert_eq!(hurt_frame(&long), Some(8));
    }

    #[test]
    fn the_defeat_shows_h0_frozen_then_thrown_then_d0_six_ticks_then_d1() {
        let down = |frozen, grounded, landed_age| HurtInput {
            hurt_ticks: 18,
            hurt_total: 18,
            frozen,
            defeated: true,
            grounded,
            landed_age,
        };
        assert_eq!(hurt_frame(&down(true, false, None)), Some(8));
        assert_eq!(hurt_frame(&down(false, false, None)), Some(10));
        assert_eq!(hurt_frame(&down(false, true, Some(0))), Some(11));
        assert_eq!(hurt_frame(&down(false, true, Some(5))), Some(11));
        assert_eq!(hurt_frame(&down(false, true, Some(6))), Some(12));
        assert_eq!(hurt_frame(&down(false, true, Some(500))), Some(12));
        // Defeated with the flinch long over: still lying, not her locomotion.
        let lying = HurtInput {
            hurt_ticks: 0,
            ..down(false, true, Some(40))
        };
        assert_eq!(hurt_frame(&lying), Some(12));
    }

    #[test]
    fn nothing_is_drawn_when_she_is_not_hurt() {
        assert_eq!(hurt_frame(&HurtInput::default()), None);
        assert_eq!(hurt_frame(&hurt(0)), None);
    }

    #[test]
    fn only_the_thrown_frame_drops_the_pivot_and_by_the_blend() {
        assert_eq!(knockback_foot_drop(10, 0.0), 0.0);
        assert_eq!(knockback_foot_drop(10, 0.5), 1.0);
        assert_eq!(knockback_foot_drop(10, 1.0), 2.0);
        assert_eq!(knockback_foot_drop(10, 3.0), 2.0);
        for frame in [8, 9, 11, 12, 5] {
            assert_eq!(knockback_foot_drop(frame, 1.0), 0.0, "frame {frame}");
        }
    }

    /// F01-FU-B10 (#259): the lying frames' shadow is their opaque extent in the
    /// grid source (columns 12 to 68, pivot column 46), like Carl's sprawl; the
    /// flinch, recoil and thrown frames keep the idle footprint.
    #[test]
    fn the_lying_frames_cast_the_shadow_of_their_opaque_extent() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let read = |rel: &str| -> serde_json::Value {
            serde_json::from_str(&std::fs::read_to_string(root.join(rel)).unwrap()).unwrap()
        };
        let manifest = read("assets/manifests/donut_h40_melee.json");
        let grid = read(manifest["grid_source"].as_str().unwrap());
        let pivot_x = manifest["pivot"][0].as_i64().unwrap() as i32;
        let extent = |index: usize| {
            let (mut left, mut right) = (i32::MAX, -1);
            for rows in grid["frames"][index]["cels"].as_object().unwrap().values() {
                for row in rows.as_array().unwrap() {
                    for (x, ch) in row.as_str().unwrap().chars().enumerate() {
                        if ch != '.' {
                            left = left.min(x as i32);
                            right = right.max(x as i32);
                        }
                    }
                }
            }
            (right - left + 1, pivot_x - left)
        };
        for frame in [KNOCKDOWN_IMPACT_FRAME, KNOCKDOWN_SULK_FRAME] {
            assert_eq!(hurt_shadow_footprint(frame), Some(extent(frame)), "{frame}");
        }
        for frame in [HURT_FLINCH_FRAME, HURT_RECOIL_FRAME, KNOCKBACK_FRAME, 0, 5] {
            assert_eq!(hurt_shadow_footprint(frame), None, "{frame}");
        }
        // Wider than the idle footprint: the shadow grows when she lies down.
        assert!(KNOCKDOWN_SHADOW_FOOTPRINT.0 > crate::actor_sprite::DONUT_SHADOW_FOOTPRINT.0);
    }

    /// The frame table is the manifest's: indices, tags, tick windows and the
    /// airborne tag of the thrown frame.
    #[test]
    fn the_reaction_frames_match_the_melee_manifest() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/manifests/donut_h40_melee.json");
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let melee = &json["melee"];
        let frame = |index: usize| &melee["frames"][index];
        for (index, name, tag, ticks) in [
            (HURT_FLINCH_FRAME, "h0 flinch", "hurt", Some(5)),
            (HURT_RECOIL_FRAME, "h1 recoil", "hurt", Some(11)),
            (KNOCKBACK_FRAME, "kb thrown", "knockback", None),
            (
                KNOCKDOWN_IMPACT_FRAME,
                "d0 sploot impact",
                "knockdown",
                Some(6),
            ),
            (
                KNOCKDOWN_SULK_FRAME,
                "d1 sploot sulk",
                "knockdown",
                Some(30),
            ),
        ] {
            assert_eq!(frame(index)["index"], index, "{name}");
            assert_eq!(frame(index)["name"], name);
            assert_eq!(frame(index)["tag"], tag);
            assert_eq!(frame(index)["ticks"].as_u64(), ticks, "{name}");
        }
        assert_eq!(u64::from(HURT_FLINCH_TICKS), 5);
        assert_eq!(u64::from(KNOCKDOWN_IMPACT_TICKS), 6);
        let airborne: Vec<&str> = melee["airborne_tags"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t.as_str())
            .collect();
        assert!(airborne.contains(&"knockback"));
        assert_eq!(melee["moves"]["hurt"]["total_ticks"], 16);
        assert_eq!(melee["moves"]["knockdown"]["total_ticks"], 36);
        assert_eq!(MELEE_FRAMES, 15);
    }
}
