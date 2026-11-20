//! What a move or ability id means on main. The lab never invents an ability:
//! a profile or a room may only name ids this table can classify, and only the
//! ones that really exist can run (LAB-Q01: "validated at load time against
//! actual ability implementation").

use crate::combat::MoveId;
use crate::party::HeroId;

/// The classification of one id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbilityStatus {
    /// Implemented in the legacy combat simulation, owned by `hero`.
    Implemented(HeroId),
    /// A CF02 move: it exists only behind `cf02_rules` (not integrated yet).
    Cf02Gated(HeroId),
    /// A named concept that does not exist on main; the text says why.
    Unimplemented(&'static str),
    /// Not a name the game knows.
    Unknown,
}

/// The CF02 moves of Carl (COMBAT-FUN-02 Q-numbers in `crate::cf02`).
pub const CF02_CARL_MOVES: [&str; 4] =
    ["grounded_stomp", "follow_through", "dive_kick", "bail_out"];

/// Concepts the design names but main does not have.
const UNIMPLEMENTED: [(&str, &str); 9] = [
    (
        "dynamite",
        "the dynamite special has its own profile id when it lands",
    ),
    ("gauntlet_fist", "off in the baseline profile"),
    (
        "explosive",
        "waits for a story-earned explosive and TERRAIN-L3",
    ),
    ("puddle_jumper", "not fabricated (Q10 section 3)"),
    ("advanced_donut_spells", "not fabricated (Q10 section 3)"),
    ("grab", "Carl does not grab enemies"),
    ("dodge", "no new dodge button"),
    ("sprint", "no sprint button"),
    ("mongo_shoulder", "no forced shoulder mode, no Mongo"),
];

/// The legacy moves of the combat simulation: the real [`MoveId`] names.
const LEGACY: [MoveId; 6] = [
    MoveId::Jab,
    MoveId::Haymaker,
    MoveId::PushKick,
    MoveId::Swat,
    MoveId::Pounce,
    MoveId::MagicMissile,
];

/// Classify `id`.
pub fn classify(id: &str) -> AbilityStatus {
    if let Some(mv) = LEGACY.iter().find(|m| m.name() == id) {
        return AbilityStatus::Implemented(if mv.hero() == crate::donut_moves::DONUT_HERO {
            HeroId::Donut
        } else {
            HeroId::Carl
        });
    }
    if CF02_CARL_MOVES.contains(&id) {
        return AbilityStatus::Cf02Gated(HeroId::Carl);
    }
    if let Some((_, reason)) = UNIMPLEMENTED.iter().find(|(name, _)| *name == id) {
        return AbilityStatus::Unimplemented(reason);
    }
    AbilityStatus::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_legacy_moves_are_classified_by_their_owner() {
        assert_eq!(classify("jab"), AbilityStatus::Implemented(HeroId::Carl));
        assert_eq!(
            classify("push_kick"),
            AbilityStatus::Implemented(HeroId::Carl)
        );
        assert_eq!(
            classify("pounce"),
            AbilityStatus::Implemented(HeroId::Donut)
        );
        assert_eq!(
            classify("magic_missile"),
            AbilityStatus::Implemented(HeroId::Donut)
        );
        assert_eq!(classify("swat"), AbilityStatus::Implemented(HeroId::Donut));
    }

    #[test]
    fn cf02_moves_are_gated_and_concepts_are_unimplemented() {
        for id in CF02_CARL_MOVES {
            assert_eq!(classify(id), AbilityStatus::Cf02Gated(HeroId::Carl));
        }
        assert!(matches!(
            classify("dynamite"),
            AbilityStatus::Unimplemented(_)
        ));
        assert!(matches!(
            classify("puddle_jumper"),
            AbilityStatus::Unimplemented(_)
        ));
        assert_eq!(classify("fireball"), AbilityStatus::Unknown);
        assert_eq!(classify("greybox_strike"), AbilityStatus::Unknown);
    }
}
