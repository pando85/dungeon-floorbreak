//! The companion's own event log and tallies (CA-25, 01A-wire).
//!
//! Separate from [`crate::combat::CombatEvent`]: no new combat variant exists,
//! no combat hash reads these, and they are drained like
//! `CombatState::take_new_events`. A `Press` records an injected press (the
//! one real write path of the companion, CA-24); the decision trace is a debug
//! aid and never hashed.

use crate::companion::autonomy::{Goal, GoalReason};
use crate::companion::director::{Action, Mode};
use crate::companion::nav::{NavAnchorId, Reason};
use crate::companion::navfollow::NavNote;
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
    /// The nav-driven follow's trace (01B-wire): a transit began or ended, or the
    /// hero waits because no route to the leader exists.
    Nav {
        tick: u64,
        hero: HeroId,
        note: NavNote,
    },
    /// The autonomy layer's goal changed (COMPANION-AI-02, CA2-B1; a trace, never
    /// hashed). Nothing emits it before the CA2-B2 wiring.
    GoalChanged {
        tick: u64,
        hero: HeroId,
        goal: Goal,
        reason: GoalReason,
    },
    /// The hazard rectangles fed to the nav graph changed (COMPANION-AI-02 CA2-C,
    /// K13; a trace, never hashed): `rects` hero-hurting volumes are telegraphed
    /// or active now. Emitted on a change only, not every tick.
    HazardFeed { tick: u64, rects: u8 },
    /// An off-screen regroup of a companion-driven hero (01B-wire).
    Regroup {
        tick: u64,
        hero: HeroId,
        outcome: RegroupOutcome,
    },
    /// FA-Q37 (Boiler only; a trace, never hashed): the enemy `target` stopped being
    /// reserved for the human leader, for good (this attempt), because of `cause`.
    ReserveReleased {
        tick: u64,
        target: StableEntityId,
        cause: ReserveRelease,
    },
}

/// Why a reserved enemy was released (FA-Q37).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReserveRelease {
    /// It landed the configured number of hits on a hero.
    Hit,
    /// It stayed reserved for the configured number of ticks in a row.
    Timeout,
}

/// What the nav-safe regroup did (the debug trace and the camera warning).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegroupOutcome {
    /// Placed on `anchor` at `x` (rounded px), just outside the view.
    Placed { anchor: NavAnchorId, x: i32 },
    /// No safe, reachable, off-screen anchor: the hero stays where it is.
    Denied(Reason),
    /// The hero stood on no known anchor: the legacy placement ran.
    Legacy,
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
    /// Nav transits committed (01B-wire).
    pub nav_transits: [u32; 2],
    /// Nav transits that ended on their destination.
    pub nav_arrivals: [u32; 2],
    /// Times the hero began waiting because no route to the leader exists.
    pub nav_waits: [u32; 2],
    /// Nav regroups that placed the hero.
    pub regroups: [u32; 2],
    /// Nav regroups refused (no safe, reachable, off-screen anchor).
    pub regroups_denied: [u32; 2],
    /// CA2-C: times the hazard rectangles fed to the nav graph changed (each a
    /// graph rebuild).
    pub hazard_feed_changes: u32,
    /// CA2-C: times the graph refused the hazard list (always 0: a defect if not).
    pub hazard_feed_errors: u32,
    /// FA-Q34 (M16, Boiler only): ticks on which the AI's press against an enemy reserved
    /// for the human leader was withheld.
    pub reserve_withheld: u32,
    /// FA-Q37 (M16, Boiler only): reserved enemies released by a hit on a hero.
    pub reserve_released_hit: u32,
    /// FA-Q37 (M16, Boiler only): reserved enemies released by the timeout.
    pub reserve_released_timeout: u32,
}

/// The cap of the bounded event buffer (oldest dropped first).
pub const EVENT_CAP: usize = 256;
