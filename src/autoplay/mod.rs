//! AUTOPLAY-01 native environment core (stage A, part 1).
//!
//! A Bevy-free wrapper around the real Floor 01 fixture simulation
//! ([`crate::combat_scene::FixtureSim`]). It owns no gameplay rules: it turns an
//! [`ActionV1`] into the game's own [`crate::input::RawInput`], steps one real
//! 60 Hz tick, drains the ordered combat events of that tick and decides the
//! episode status (AP-Q09 and AP-Q10 of
//! `docs/03-tech/autoplay-01-automated-gameplay-evaluation.md`).
//!
//! Part 1 has no observation, reward or `ap_*` scenarios; those arrive as
//! separate modules ([`StepOutcome`] already carries the terminal reason and the
//! new events they will consume).

pub mod action;
pub mod config;
pub mod env;
pub mod error;

pub use action::{ACTION_V1_LEN, ActionV1, UnavailableCounters};
pub use config::{
    ACTION_VERSION_V1, AutoplayConfig, DEFAULT_MAX_TICKS, MAX_ACTION_REPEAT, SimSeed,
};
pub use env::{AutoplayEnv, Outcome, ResetInfo, StepOutcome};
pub use error::AutoplayError;

#[cfg(test)]
mod tests;
