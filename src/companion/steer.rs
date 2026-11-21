//! COMPANION-AI-02 CA2-B2: goal steering (spec 5.2), the movement half of the
//! autonomy layer. Pure and Bevy-free.
//!
//! Under a brief the director writes no `move_x` for a support plan (it only
//! presses), so something has to walk her to the spot of the goal, turn her
//! toward the target once she is in band, and keep her inside the free zone `Z`.
//! That is this module. The nav go-to (committed jump and drop transits) is the
//! executor's ([`crate::companion::navfollow::NavDriver::go_to`]); this module
//! is the flat part that the wiring falls back on when the whole route is a
//! walk, plus the clamp that applies to both.
//!
//! Priority of one tick's movement (spec 5.2): the dodge, then the director's
//! approach for an interrupt only, then goal steering. Goal steering and the
//! interrupt approach are clamped to `Z` ([`clamp_to_zone`]); a dodge and a
//! committed nav transit are exempt (they are involuntary or already
//! drift-checked, spec 3.3).

use crate::companion::autonomy::{AutonomyBrief, FreeZone};
use crate::companion::view::HeroView;

/// The stick value of the one tick that turns her toward her target (the dead
/// zone is 0.18, so this moves her under 0.7 px).
pub const TURN_STICK: f32 = 0.2;

/// The vertical allowance of a full jump in the 640 x 360 automatic view
/// (CA2 Revision 3, "apex above the view"): the highest anchor she may start a
/// jump or hop edge from is this much above the leader's feet **minus her
/// effective apex**. About 137 px at DM-A's 114.92 px apex (the review's
/// number) and about 114 px with the boosted launch (DP-Q38, 137 - 23.03).
pub const JUMP_VIEW_ALLOWANCE_PX: f32 = 252.0;

/// The rise limit of jump and hop edges for a partner whose ordinary jump has
/// `apex_px` (an **input**, so the DP-E blink and the boost feed their own).
pub fn jump_edge_rise_limit_px(apex_px: f32) -> f32 {
    JUMP_VIEW_ALLOWANCE_PX - apex_px
}

/// The analog stick toward `target_x`: full stick far away, the exact remaining
/// step near it (the nav executor's `approach`). `run_px_per_tick` is her own
/// run speed.
pub fn approach_stick(x: f32, target_x: f32, run_px_per_tick: f32) -> f32 {
    ((target_x - x) / run_px_per_tick).clamp(-1.0, 1.0)
}

/// Flat goal steering for one tick: walk to the brief's spot; once on it (or
/// when the goal has no spot, she holds), turn toward `face_x` with one tick of
/// stick if she faces away. `arrive_eps_px` is the nav executor's arrive
/// tolerance for her run speed.
pub fn flat_steer(
    me: &HeroView,
    brief: &AutonomyBrief,
    run_px_per_tick: f32,
    arrive_eps_px: f32,
) -> f32 {
    if let Some(spot) = brief.spot
        && (spot.x - me.x).abs() > arrive_eps_px
    {
        return approach_stick(me.x, spot.x, run_px_per_tick);
    }
    face_stick(me, brief.face_x)
}

/// One tick of stick toward `face_x` when she faces the other way (and the
/// target is not on top of her), otherwise 0.
pub fn face_stick(me: &HeroView, face_x: Option<f32>) -> f32 {
    let Some(fx) = face_x else { return 0.0 };
    let dx = fx - me.x;
    if dx.abs() < 1.0 {
        return 0.0;
    }
    let want: i8 = if dx < 0.0 { -1 } else { 1 };
    if want == me.facing {
        0.0
    } else {
        f32::from(want) * TURN_STICK
    }
}

/// Keep a voluntary step inside `Z`: a step that would cross a zone edge is cut
/// to end on it, and a step outward from outside the zone is refused (she may
/// only walk back in). Returns the stick to use.
pub fn clamp_to_zone(me_x: f32, move_x: f32, zone: &FreeZone, run_px_per_tick: f32) -> f32 {
    if move_x == 0.0 {
        return 0.0;
    }
    let step = move_x * run_px_per_tick;
    let next = me_x + step;
    if move_x > 0.0 && next > zone.right {
        return if me_x < zone.right {
            ((zone.right - me_x) / run_px_per_tick).max(0.0)
        } else {
            0.0
        };
    }
    if move_x < 0.0 && next < zone.left {
        return if me_x > zone.left {
            ((zone.left - me_x) / run_px_per_tick).min(0.0)
        } else {
            0.0
        };
    }
    move_x
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::companion::autonomy::{Goal, Spot, SpotPurpose};
    use crate::companion::view::{Control, HeroView};
    use crate::input::STICK_DEADZONE;
    use crate::party::HeroId;

    fn hero(x: f32, facing: i8) -> HeroView {
        HeroView {
            id: HeroId::Donut,
            x,
            foot_y: 0.0,
            vx: 0.0,
            vy: 0.0,
            facing,
            grounded: true,
            health: 4,
            hurt_ticks: 0,
            invulnerable_ticks: 0,
            defeated: false,
            control: Control::Companion,
            active_move: None,
            can_start_move: true,
            mana: None,
        }
    }

    fn brief(spot_x: Option<f32>, face_x: Option<f32>) -> AutonomyBrief {
        AutonomyBrief {
            goal: Goal::Hunt,
            changed: None,
            target: None,
            spot: spot_x.map(|x| Spot {
                x,
                surface_y: 0.0,
                key: 0,
                leader_ref_x: 0.0,
                purpose: SpotPurpose::Attack,
            }),
            face_x,
            recoil: false,
            in_zone: true,
            director: crate::companion::director::GoalBrief::interrupts_only(),
        }
    }

    fn zone(left: f32, right: f32) -> FreeZone {
        FreeZone {
            leader_x: (left + right) / 2.0,
            left,
            right,
            reach_left: 0.0,
            reach_right: 0.0,
            safe_left: 0.0,
            safe_right: 0.0,
            surface_min_y: -1000.0,
            surface_max_y: 1000.0,
        }
    }

    const RUN: f32 = 3.45;

    /// CA2 Revision 3, DP-Q38: the rise limit is an input-derived number: about
    /// 137 px at DM-A's apex and about 114 px with the boosted launch (the apex
    /// grows by 23.03 px, so the limit shrinks by the same).
    #[test]
    fn the_jump_edge_rise_limit_follows_the_apex() {
        let base = crate::agility::discrete_apex_px(crate::agility::agile_motion().jump_speed);
        assert!((base - 114.92).abs() < 0.05, "{base}");
        let limit = jump_edge_rise_limit_px(base);
        assert!((limit - 137.0).abs() < 0.2, "{limit}");
        let boosted = crate::agility::discrete_apex_px(547.0);
        assert!((boosted - 137.95).abs() < 0.05, "{boosted}");
        let boosted_limit = jump_edge_rise_limit_px(boosted);
        assert!((boosted_limit - 114.0).abs() < 0.2, "{boosted_limit}");
        assert!((limit - boosted_limit - 23.03).abs() < 0.1);
    }

    #[test]
    fn she_walks_to_the_spot_then_faces_the_target_with_one_tick_of_stick() {
        let b = brief(Some(300.0), Some(400.0));
        // Far: full stick toward the spot, whatever she faces.
        assert_eq!(flat_steer(&hero(100.0, -1), &b, RUN, 0.67), 1.0);
        assert_eq!(flat_steer(&hero(500.0, 1), &b, RUN, 0.67), -1.0);
        // Near: the exact remaining step (the nav executor's arrive rule).
        let near = flat_steer(&hero(298.0, 1), &b, RUN, 0.67);
        assert!((near - 2.0 / RUN).abs() < 1e-6, "{near}");
        // On the spot, facing away: one tick of turning stick, above the dead zone.
        let turn = flat_steer(&hero(300.0, -1), &b, RUN, 0.67);
        assert_eq!(turn, TURN_STICK);
        assert!(turn > STICK_DEADZONE);
        // Facing it already: stand still.
        assert_eq!(flat_steer(&hero(300.0, 1), &b, RUN, 0.67), 0.0);
        // Target on the left, facing right: turn left.
        let b = brief(Some(300.0), Some(200.0));
        assert_eq!(flat_steer(&hero(300.0, 1), &b, RUN, 0.67), -TURN_STICK);
    }

    #[test]
    fn a_goal_without_a_spot_holds_and_only_turns() {
        let b = brief(None, Some(100.0));
        assert_eq!(flat_steer(&hero(300.0, 1), &b, RUN, 0.67), -TURN_STICK);
        assert_eq!(flat_steer(&hero(300.0, -1), &b, RUN, 0.67), 0.0);
        let b = brief(None, None);
        assert_eq!(flat_steer(&hero(300.0, 1), &b, RUN, 0.67), 0.0);
    }

    #[test]
    fn the_zone_clamp_cuts_a_step_at_each_edge_and_refuses_outward_steps_from_outside() {
        let z = zone(100.0, 400.0);
        // Inside, far from the edges: unchanged.
        assert_eq!(clamp_to_zone(250.0, 1.0, &z, RUN), 1.0);
        assert_eq!(clamp_to_zone(250.0, -0.5, &z, RUN), -0.5);
        // The last step to the right edge is cut to end on it.
        let s = clamp_to_zone(398.0, 1.0, &z, RUN);
        assert!((s * RUN - 2.0).abs() < 1e-5, "{s}");
        let s = clamp_to_zone(102.0, -1.0, &z, RUN);
        assert!((s * RUN + 2.0).abs() < 1e-5, "{s}");
        // Exactly on the edge: outward refused, inward kept.
        assert_eq!(clamp_to_zone(400.0, 1.0, &z, RUN), 0.0);
        assert_eq!(clamp_to_zone(400.0, -1.0, &z, RUN), -1.0);
        assert_eq!(clamp_to_zone(100.0, -1.0, &z, RUN), 0.0);
        assert_eq!(clamp_to_zone(100.0, 1.0, &z, RUN), 1.0);
        // Involuntarily outside: outward refused, back in allowed.
        assert_eq!(clamp_to_zone(450.0, 1.0, &z, RUN), 0.0);
        assert_eq!(clamp_to_zone(450.0, -1.0, &z, RUN), -1.0);
        assert_eq!(clamp_to_zone(50.0, -1.0, &z, RUN), 0.0);
        assert_eq!(clamp_to_zone(50.0, 1.0, &z, RUN), 1.0);
        // No stick, no step.
        assert_eq!(clamp_to_zone(450.0, 0.0, &z, RUN), 0.0);
    }
}
