//! COMPANION-AI-01 modules: 01A-core is the deterministic companion director.
//!
//! One director drives either uncontrolled hero (CA-01). It is Bevy-free and
//! pure: it reads a [`view::CompanionView`] (plain owned data, built later by
//! `combat_scene/companion.rs`, stage 01A-wire), a [`profile::ProfileSet`] and
//! its own small state, and returns exactly one [`crate::input::RawInput`]. It
//! never writes the simulation: the press goes through the same input path a
//! human press uses (CA-24).
//!
//! - [`view`]: the perception contract (`CompanionView`, `StableEntityId`).
//! - [`profile`]: `companion_ai_v0.json` (three profile sets, FNV-1a pinned).
//! - [`director`]: states, legal utility candidates, safety vetoes, cadence.
//! - [`events`]: the separate `CompanionEvent` log and the per-run counters.
//! - [`rules`]: the kill-policy `glance` (CA-19, CA-20).
//! - [`nav`]: the platform graph (01B-core); [`navfollow`]: the nav-driven
//!   follow executor and the camera-safe regroup (01B-wire).
//! - `command_input`: the pure command grammar (01C).
//! - [`engage`] and [`autonomy`]: COMPANION-AI-02, the leader's engagement
//!   predicate, the camera-bounded free zone, the drift check, the leader's
//!   claim set (CA2-A) and the goal selector with its brief for the director
//!   (CA2-B1). Pure and not wired: nothing in a scene reads them until CA2-B2,
//!   and the `autonomy` profile block is off in every shipped set but
//!   `cf02_free`.
//!
//! `docs/03-tech/companion-ai-command-system.md` and
//! `docs/07-agents/companion-ai-01-workorder.md` describe the stages.
//! 01A-wire (`combat_scene/companion.rs`, `corridor.rs`, `party.rs`) wires the
//! director behind `UncontrolledHero::Companion` and the `--companion` flag:
//! with the companion off nothing here runs, hashes or is read, so no golden
//! changes.

pub mod autonomy;
pub mod command_input;
pub mod director;
pub mod engage;
pub mod events;
pub mod nav;
pub mod navfollow;
pub mod profile;
pub mod rules;
pub mod steer;
pub mod view;
