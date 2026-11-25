//! The reflex variant of the CORRAL-AUTO sweep. This is the one file of the
//! evaluator that names a test-only reflex profile (`ReflexProfile::Off`,
//! the `off` variant is test tuning, plan section 14.4, F6), so the reflex
//! source scan exempts exactly this file and keeps scanning the rest.

use crate::agility::ReflexProfile;

/// DM-Q11 reflex evasion mode of the sweep (F6, #181 section 7 and MAG-16).
/// DM-B is on main: `Off` forces `dm_q11_evasion_mode: off`, `Reflex` runs
/// the shipped reflex (720 ticks, armed at attempt start per FA-Q19).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Evasion {
    Off,
    Reflex,
}

impl Evasion {
    pub const ALL: [Evasion; 2] = [Evasion::Off, Evasion::Reflex];

    pub const fn name(self) -> &'static str {
        match self {
            Evasion::Off => "off",
            Evasion::Reflex => "reflex",
        }
    }

    /// The party tuning's reflex for this variant.
    pub const fn profile(self) -> ReflexProfile {
        match self {
            Evasion::Off => ReflexProfile::Off,
            Evasion::Reflex => ReflexProfile::Shipped,
        }
    }
}
