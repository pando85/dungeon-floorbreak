//! The companion's own event log and tallies (CA-25, 01A-wire).
//!
//! Separate from [`crate::combat::CombatEvent`]: no new combat variant exists,
//! no combat hash reads these, and they are drained like
//! `CombatState::take_new_events`. A `Press` records an injected press (the
//! one real write path of the companion, CA-24); the decision trace is a debug
//! aid and never hashed.

use crate::companion::director::{Action, Mode};
use crate::companion::view::StableEntityId;
use crate::party::HeroId;

/// One companion fact, in tick order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompanionEvent {
    /// The director injected a press for `hero` this tick. `chain` is a
    /// follow-up inside the running AI-started move (the haymaker, CA-36): it
    /// is not a new assist start.
    Press {
        tick: u64,
        hero: HeroId,
        action: Action,
        target: StableEntityId,
        chain: bool,
    },
    /// The director's mode, action or target changed (a debug trace).
    Decision {
        tick: u64,
        hero: HeroId,
        mode: Mode,
        action: Option<Action>,
        target: Option<StableEntityId>,
    },
}

/// Per-run companion counters, per hero (index 0 Carl, 1 Donut). Never
/// hashed. These are the CA-31 / CF02-Q39 numbers of one fixture: assist
/// starts, interrupts, chain follow-ups and decisions made. The kill-credit,
/// hit-share, glance and `ai_indirect_kills` figures come from the combat
/// event log (see `companion_fixtures::tally`), because only the event log
/// proves what actually landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompanionStats {
    /// Presses that started an AI move (chain follow-ups excluded).
    pub assist_starts: [u32; 2],
    /// Presses that answered a tell aimed at the other hero.
    pub interrupt_starts: [u32; 2],
    /// Chain follow-up presses (the haymaker).
    pub chain_presses: [u32; 2],
    /// Ticks the director chose a non-follow mode.
    pub active_ticks: [u32; 2],
}

/// The cap of the bounded event buffer (oldest dropped first).
pub const EVENT_CAP: usize = 256;
