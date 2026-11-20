//! Errors of the native autoplay environment.

use std::fmt;

use super::env::Outcome;

#[derive(Debug, Clone, PartialEq)]
pub enum AutoplayError {
    /// The config is inconsistent or asks for something not available yet.
    InvalidConfig(String),
    /// `cf02_rules = true` is rejected until COMBAT-FUN-02 exposes the switch.
    Cf02Unsupported,
    /// A non-default `sim_seed` is rejected until a seed can reach the sim.
    SimSeedUnsupported,
    /// The scenario id is not part of the v1 roster.
    UnsupportedScenario(String),
    /// The action has the wrong length or an out-of-range value (never clamped).
    InvalidAction(String),
    /// `step` was called before the first `reset`.
    NotReset,
    /// `step` was called after the episode ended; call `reset` first.
    EpisodeOver(Outcome),
    /// The kit or the fixture failed to load or validate.
    Sim(String),
    /// A fresh sim did not hash to the pinned initial value.
    InitialHashMismatch { expected: u64, actual: u64 },
    /// `party_solo` companion id that this build has no backing variant for
    /// (AP-Q24).
    UnsupportedCompanion(String),
    /// The one-tick seat entry did not leave the seat hero controlled
    /// (AP-Q23). A reset error: never a silent Carl episode.
    SeatEntryFailed(String),
}

impl fmt::Display for AutoplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "invalid autoplay config: {msg}"),
            Self::Cf02Unsupported => write!(
                f,
                "cf02_rules = true is not supported yet (COMBAT-FUN-02 has not landed); use false"
            ),
            Self::SimSeedUnsupported => write!(
                f,
                "only sim_seed = fixture is supported: there is no gameplay RNG on this build"
            ),
            Self::UnsupportedScenario(id) => {
                write!(f, "unsupported autoplay scenario {id}")
            }
            Self::InvalidAction(msg) => write!(f, "invalid action: {msg}"),
            Self::NotReset => write!(f, "step called before reset"),
            Self::EpisodeOver(outcome) => write!(
                f,
                "episode is over ({}); call reset before step",
                outcome.name()
            ),
            Self::Sim(msg) => write!(f, "simulation setup failed: {msg}"),
            Self::InitialHashMismatch { expected, actual } => write!(
                f,
                "fresh sim hash {actual:016x} differs from the pinned initial hash {expected:016x}"
            ),
            Self::UnsupportedCompanion(msg) => {
                write!(f, "unsupported companion profile {msg}")
            }
            Self::SeatEntryFailed(msg) => write!(f, "party seat entry failed: {msg}"),
        }
    }
}

impl std::error::Error for AutoplayError {}
