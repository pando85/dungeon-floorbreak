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
//! - [`profile`]: `companion_ai_v0.json` (two profile sets, FNV-1a pinned).
//! - [`director`]: states, legal utility candidates, safety vetoes, cadence.
//! - [`events`]: the separate `CompanionEvent` log and the per-run counters.
//! - [`rules`]: the kill-policy `glance` (CA-19, CA-20).
//! - `nav`, `command_input`: CF02-00 stubs, filled by 01B-core and 01C.
//!
//! `docs/03-tech/companion-ai-command-system.md` and
//! `docs/07-agents/companion-ai-01-workorder.md` describe the stages.
//! 01A-wire (`combat_scene/companion.rs`, `corridor.rs`, `party.rs`) wires the
//! director behind `UncontrolledHero::Companion` and the `--companion` flag:
//! with the companion off nothing here runs, hashes or is read, so no golden
//! changes.

pub mod command_input;
pub mod director;
pub mod events;
pub mod nav;
pub mod profile;
pub mod rules;
pub mod view;
