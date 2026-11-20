//! AUTOPLAY-01 native environment (stage A).
//!
//! A Bevy-free wrapper around the real Floor 01 fixture simulation
//! ([`crate::combat_scene::FixtureSim`]). It owns no gameplay rules: it turns an
//! [`ActionV1`] into the game's own [`crate::input::RawInput`], steps one real
//! 60 Hz tick, drains the ordered combat events of that tick and decides the
//! episode status (AP-Q09 and AP-Q10 of
//! `docs/03-tech/autoplay-01-automated-gameplay-evaluation.md`).
//!
//! Part 2 adds [`ObservationV1`] with the perception mask, the separate
//! privileged [`Diagnostics`], [`RewardProfile`] data with once-per-enemy
//! credit, and the two `ap_*` roster scenarios.

pub mod action;
pub mod bench;
pub mod bots;
pub mod config;
pub mod diagnostics;
pub mod env;
pub mod error;
pub mod eval;
pub mod metrics;
pub mod observation;
pub mod replay;
pub mod reward;
pub mod scenario;

pub use action::{ACTION_V1_LEN, ActionV1, UnavailableCounters};
pub use config::{
    ACTION_VERSION_V1, AutoplayConfig, DEFAULT_MAX_TICKS, MAX_ACTION_REPEAT, SimSeed,
};
pub use diagnostics::{CarlDiagnostics, Diagnostics, EnemyDiagnostics, EventCounts};
pub use env::{AutoplayEnv, Outcome, ResetInfo, StepOutcome};
pub use error::AutoplayError;
pub use observation::{OBS_V1, OBSERVATION_VERSION, ObsSchema, ObsStats, ObservationV1};
pub use reward::{
    Attribution, DEFAULT_REWARD_ID, DefeatCredit, RewardBreakdown, RewardProfile, RewardTracker,
};
pub use scenario::{RosterEntry, Split};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_eval;
#[cfg(test)]
mod tests_part2;
