//! The kill-policy rule (CA-19, CA-20): `glance`.
//!
//! Pure data in, data out. The wiring (01A-wire) calls it at Carl's strike site
//! and at Donut's swat, pounce and bolt hit sites, with the target's HP read on
//! the **hit tick** (so a human lowering the HP mid-move is honoured).

use crate::combat::MoveSpec;
use crate::companion::profile::KillPolicy;

/// Whether an AI-started hit of `damage` on a target with `target_hp` is a
/// glance under `kill_policy`.
pub const fn is_glance(
    damage: u8,
    target_hp: u8,
    ai_started: bool,
    kill_policy: KillPolicy,
) -> bool {
    matches!(kill_policy, KillPolicy::Never) && ai_started && damage > 0 && damage >= target_hp
}

/// The move as it lands. Under `never`, an AI-started hit whose damage is at
/// least the target's HP deals 0 and keeps the move's non-lethal reaction
/// (stagger, knockback, hit-stop, launch): the target is hit but not defeated.
/// Under `factual`, for a human-started hit and for a non-lethal hit the spec
/// is returned unchanged. Indirect kills (contact, wall slam, shore, hazard) are
/// not suppressed (CA-20).
pub fn glance(
    spec: MoveSpec,
    target_hp: u8,
    ai_started: bool,
    kill_policy: KillPolicy,
) -> MoveSpec {
    if is_glance(spec.damage, target_hp, ai_started, kill_policy) {
        MoveSpec { damage: 0, ..spec }
    } else {
        spec
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{
        CARL_HAYMAKER, CARL_JAB, CARL_PUSH_KICK, DONUT_POUNCE, DONUT_SWAT, ENEMY_HEALTH,
    };

    #[test]
    fn never_glances_an_ai_started_lethal_hit_and_keeps_the_reaction() {
        for spec in [CARL_JAB, CARL_HAYMAKER, CARL_PUSH_KICK, DONUT_POUNCE] {
            let lethal = glance(spec, spec.damage, true, KillPolicy::Never);
            assert_eq!(lethal.damage, 0, "{:?}", spec.id);
            assert_eq!(
                MoveSpec {
                    damage: spec.damage,
                    ..lethal
                },
                spec,
                "only the damage changes: the non-lethal reaction is kept"
            );
            assert!(lethal.reaction.stagger_ticks == spec.reaction.stagger_ticks);
            assert!(is_glance(spec.damage, 1, true, KillPolicy::Never));
        }
    }

    #[test]
    fn never_leaves_a_non_lethal_ai_hit_alone() {
        let hp = CARL_JAB.damage + 1;
        assert_eq!(glance(CARL_JAB, hp, true, KillPolicy::Never), CARL_JAB);
        assert_eq!(
            glance(CARL_JAB, ENEMY_HEALTH, true, KillPolicy::Never),
            CARL_JAB
        );
    }

    #[test]
    fn never_leaves_a_human_started_hit_alone() {
        assert_eq!(glance(CARL_JAB, 1, false, KillPolicy::Never), CARL_JAB);
        assert_eq!(
            glance(DONUT_POUNCE, 1, false, KillPolicy::Never),
            DONUT_POUNCE
        );
    }

    #[test]
    fn factual_never_changes_the_spec() {
        for spec in [
            CARL_JAB,
            CARL_HAYMAKER,
            CARL_PUSH_KICK,
            DONUT_POUNCE,
            DONUT_SWAT,
        ] {
            for hp in 0..=3 {
                for ai in [false, true] {
                    assert_eq!(glance(spec, hp, ai, KillPolicy::Factual), spec);
                }
            }
        }
    }

    #[test]
    fn a_damage_zero_move_is_never_changed() {
        assert_eq!(DONUT_SWAT.damage, 0);
        assert_eq!(glance(DONUT_SWAT, 1, true, KillPolicy::Never), DONUT_SWAT);
        assert!(!is_glance(0, 0, true, KillPolicy::Never));
    }

    #[test]
    fn a_hit_on_a_target_that_is_already_at_zero_deals_nothing() {
        assert_eq!(glance(CARL_JAB, 0, true, KillPolicy::Never).damage, 0);
    }
}
