//! The shared policy for what a world or enemy source does to **heroes**.
//! Bevy-free data.
//!
//! One vocabulary for every source that can hurt a hero (integration plan K3):
//! the hero-thrown dynamite profile keeps no friendly fire, a world hazard
//! (steam, a falling weight, a drum, a Rot blast) hurts every hero by its own
//! data, and a source that never hurts a hero says so. The initiator of a
//! hazard only ever decides **credit**, never who is hurt.
//!
//! Whether enemies are hurt is a separate flag of each source (an acid patch
//! may spare slimes: per-kind data of the enemy sheets), not part of this
//! enum.
//!
//! BOIL-B core defines the enum (it lands before CP-B1); CP-B1's
//! `BlastProfile` adopts it.

/// What a source does to heroes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeroDamagePolicy {
    /// Only the owner hero is hurt, and reduced; every other hero is spared (no
    /// friendly fire, CP-Q24): the hero-thrown dynamite profile. The owner is
    /// the hero whose player index is `BlastCharge.owner`. Never valid for a
    /// world hazard, which has no owner.
    OwnerReduced,
    /// Every hero is hurt in full, whoever caused the source.
    AllHeroes,
    /// No hero is hurt.
    NoHeroes,
}

impl HeroDamagePolicy {
    pub const ALL: [Self; 3] = [Self::OwnerReduced, Self::AllHeroes, Self::NoHeroes];

    pub const fn name(self) -> &'static str {
        match self {
            Self::OwnerReduced => "owner_reduced",
            Self::AllHeroes => "all_heroes",
            Self::NoHeroes => "no_heroes",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name() == name)
    }

    /// Whether any hero can be hurt (the owner may be reduced, not spared).
    pub const fn hurts_any_hero(self) -> bool {
        !matches!(self, Self::NoHeroes)
    }

    /// Stable code for state hashes.
    pub const fn code(self) -> u64 {
        self as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_and_unknown_names_are_refused() {
        for p in HeroDamagePolicy::ALL {
            assert_eq!(HeroDamagePolicy::parse(p.name()), Some(p));
        }
        assert_eq!(HeroDamagePolicy::parse("friendly_fire"), None);
        assert_eq!(HeroDamagePolicy::parse(""), None);
    }

    #[test]
    fn only_no_heroes_spares_every_hero() {
        assert!(HeroDamagePolicy::OwnerReduced.hurts_any_hero());
        assert!(HeroDamagePolicy::AllHeroes.hurts_any_hero());
        assert!(!HeroDamagePolicy::NoHeroes.hurts_any_hero());
    }

    #[test]
    fn the_codes_are_distinct() {
        let codes: std::collections::BTreeSet<u64> =
            HeroDamagePolicy::ALL.iter().map(|p| p.code()).collect();
        assert_eq!(codes.len(), 3);
    }
}
