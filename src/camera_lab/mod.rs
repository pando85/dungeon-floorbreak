//! Floor 01 Camera Scenarios Lab (CAMERA-LAB): an isolated, Bevy-free domain for
//! the six camera probe rooms. See `docs/03-tech/camera-lab.md`.
//!
//! Everything here is a probe, not game design: the geometry, enemy behaviour
//! and tuning are lab hypotheses. The camera (`camera_op`) is presentation only
//! and never enters the simulation hash.

pub mod camera_op;
pub mod layout;
pub mod room;
pub mod sim;
pub mod script;
pub mod session;
