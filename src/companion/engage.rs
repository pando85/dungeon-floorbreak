//! The leader's engagement predicate (COMPANION-AI-02 CA2-A, spec 4.2).
//!
//! One pure definition of "the leader engages this enemy": a recent hit of his
//! (A12, either side), then the side he faces (A11), then a running move or a
//! knockdown of his within the finisher reach (A08). It was the body of
//! `CompanionDirector::steals_the_finish`; that method keeps its gates (rule
//! on, `factual` policy, human leader, the HP reserve) and calls this
//! predicate, and the autonomy layer's claim set (`autonomy::LeaderClaims`)
//! calls the same predicate without the HP gate. One definition, no copy.
//!
//! Role-neutral: the leader is whichever hero the human controls. The view's
//! `carl_hit` and `carl_hit_age` fields keep their CF02-DUO names (they record
//! the human leader's hits in v1).

use crate::companion::profile::ProfileSet;
use crate::companion::view::{HeroView, TargetState, TargetView};

/// +1 when `dx` is zero or positive, -1 when negative (an enemy exactly on the
/// leader counts as in front: the conservative reading).
fn side(dx: f32) -> i8 {
    if dx < 0.0 { -1 } else { 1 }
}

/// Does `leader` engage `target` this tick?
///
/// - **A12** (checked first, PR 170 review B-1): a target the leader hit within
///   `cf02_duo_a12_recent_hit_ticks` stays his on either side of him and at any
///   distance, so a walk-through, a step back or a turn does not hand it over
///   before the window ends.
/// - **A11**: with `cf02_duo_a11_engage_facing_only`, only a target on the side
///   the leader faces can be engaged.
/// - **A08**: the leader has a move running, or the target is still down or
///   staggered from his last hit, and the target is within
///   `cf02_duo_a08_finisher_reach_px` of him.
///
/// The gates of the kill-steal veto (rule on, policy, a human leader, the HP
/// reserve) are not here: they belong to the caller that applies the veto.
/// Pure and free of any state; a leader who merely stands near an enemy
/// engages nothing.
pub fn leader_engages(set: &ProfileSet, leader: &HeroView, target: &TargetView) -> bool {
    let duo = &set.cf02_duo;
    let recent = target
        .carl_hit_age
        .is_some_and(|age| age <= duo.cf02_duo_a12_recent_hit_ticks);
    if recent {
        return true;
    }
    let dx = target.x - leader.x;
    let in_front = dx == 0.0 || side(dx) == leader.facing;
    if duo.cf02_duo_a11_engage_facing_only && !in_front {
        return false;
    }
    let combo = leader.active_move.is_some();
    let from_his_hit = target.carl_hit
        && matches!(
            target.state,
            TargetState::Staggered { .. } | TargetState::Lying { .. }
        );
    (combo || from_his_hit) && dx.abs() <= f32::from(duo.cf02_duo_a08_finisher_reach_px)
}

/// The inline predicate exactly as `steals_the_finish` had it before the
/// extraction (A12, then A11, then A08), kept as the reference the equivalence
/// tests compare against (here and in the director).
#[cfg(test)]
pub(crate) fn reference_inline(set: &ProfileSet, carl: &HeroView, target: &TargetView) -> bool {
    let duo = &set.cf02_duo;
    let recent = target
        .carl_hit_age
        .is_some_and(|age| age <= duo.cf02_duo_a12_recent_hit_ticks);
    if recent {
        return true;
    }
    let dx = target.x - carl.x;
    let sign = |dx: f32| if dx < 0.0 { -1 } else { 1 };
    let in_front = dx == 0.0 || sign(dx) == carl.facing;
    if duo.cf02_duo_a11_engage_facing_only && !in_front {
        return false;
    }
    let combo = carl.active_move.is_some();
    let from_his_hit = target.carl_hit
        && matches!(
            target.state,
            TargetState::Staggered { .. } | TargetState::Lying { .. }
        );
    (combo || from_his_hit) && dx.abs() <= f32::from(duo.cf02_duo_a08_finisher_reach_px)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::MoveId;
    use crate::companion::profile::{KillPolicy, ProfileFile, ProfileSetId};
    use crate::companion::view::{ActiveMove, Control, MovePhase, StableEntityId, fixtures};
    use crate::party::HeroId;

    fn sets() -> Vec<(&'static str, ProfileSet)> {
        let f = ProfileFile::pinned().unwrap();
        [
            ProfileSetId::Cf02Assist,
            ProfileSetId::CompanionV0,
            ProfileSetId::Cf02Free,
        ]
        .into_iter()
        .map(|id| (id.name(), f.set(id).clone()))
        .collect()
    }

    fn states() -> [TargetState; 4] {
        [
            TargetState::Standing,
            TargetState::Staggered { ticks: 5 },
            TargetState::Lying {
                down_ticks_left: 40,
                hold_used: false,
            },
            TargetState::Airborne,
        ]
    }

    /// `leader_engages` equals the old inline predicate on a grid of views: every
    /// set, both facings, a move running or not, every target state, a hit flag
    /// and a hit age at and around the window, distances around the 40 px reach
    /// on both sides of the leader.
    #[test]
    fn leader_engages_equals_the_old_inline_predicate() {
        let mut checked = 0_u32;
        let mut engaged = 0_u32;
        for (name, set) in sets() {
            for facing in [-1_i8, 1] {
                for moving in [false, true] {
                    let mut leader = fixtures::hero(HeroId::Carl, 400.0, Control::Human(0));
                    leader.facing = facing;
                    leader.active_move = moving.then_some(ActiveMove {
                        id: MoveId::Jab,
                        phase: MovePhase::Active,
                        ticks_left: 6,
                        ai_started: false,
                    });
                    for state in states() {
                        for carl_hit in [false, true] {
                            for age in [None, Some(0), Some(30), Some(31), Some(32), Some(200)] {
                                for dx in [-200.0, -41.0, -40.0, -6.0, 0.0, 6.0, 40.0, 41.0, 200.0]
                                {
                                    let target = TargetView {
                                        id: StableEntityId::goblin(0),
                                        x: 400.0 + dx,
                                        foot_y: 0.0,
                                        hp: 2,
                                        state,
                                        guarded: false,
                                        in_view: true,
                                        carl_hit,
                                        carl_hit_age: age,
                                    };
                                    let got = leader_engages(&set, &leader, &target);
                                    assert_eq!(
                                        got,
                                        reference_inline(&set, &leader, &target),
                                        "{name} facing {facing} moving {moving} {state:?} \
                                         hit {carl_hit} age {age:?} dx {dx}"
                                    );
                                    checked += 1;
                                    engaged += u32::from(got);
                                }
                            }
                        }
                    }
                }
            }
        }
        // The grid exercises both outcomes.
        assert!(
            checked > 5000 && engaged > 500 && engaged < checked,
            "{engaged}/{checked}"
        );
    }

    #[test]
    fn the_predicate_is_the_documented_a12_a11_a08_order() {
        let set = ProfileFile::pinned()
            .unwrap()
            .set(ProfileSetId::Cf02Free)
            .clone();
        let mut leader = fixtures::hero(HeroId::Carl, 400.0, Control::Human(0));
        leader.facing = 1;
        let mut t = TargetView {
            id: StableEntityId::goblin(0),
            x: 100.0,
            foot_y: 0.0,
            hp: 2,
            state: TargetState::Standing,
            guarded: false,
            in_view: true,
            carl_hit: false,
            carl_hit_age: Some(31),
        };
        // A12 first: 300 px behind him, hit 31 ticks ago: still his.
        assert!(leader_engages(&set, &leader, &t));
        t.carl_hit_age = Some(32);
        assert!(!leader_engages(&set, &leader, &t));
        // A standing leader who just stands near an enemy engages nothing.
        t.x = 420.0;
        assert!(!leader_engages(&set, &leader, &t));
        // The gates of the veto are not part of it: the policy and the HP
        // reserve live in the caller.
        let mut never = set.clone();
        never.ca19_kill_policy = KillPolicy::Never;
        t.carl_hit_age = Some(3);
        assert!(leader_engages(&never, &leader, &t));
    }
}
