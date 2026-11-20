//! COMBAT-LAB-01 LAB-00: the scenario descriptor, the test-only capability
//! profile, the validator and the headless runner of the Floor 01 mid-floor
//! combat lab. Bevy-free.
//!
//! The lab is **not** a new simulation. It runs the authoritative
//! [`crate::combat_scene::CombatState`], [`crate::party::PartyState`] and
//! [`crate::corridor::SceneSim`] path on rooms described by
//! `assets/levels/combat_lab/manifest.json` and the `.room.json` files next to
//! it (the camera-lab room schema, as [`crate::demo`] and
//! [`crate::boss_scene`] load theirs). It has no RNG of its own: the only
//! variety source is the CF02-Q58 effective seed (`Cf02Config::seed`, LAB-Q06).
//!
//! Layers, each testable without a window:
//!
//! * [`ability`]: what a move or ability id means on main (implemented, gated
//!   behind `cf02_rules`, known but unimplemented, unknown);
//! * [`profile`]: the capability profile `floor01-postbiscuit-lab-v1` and its
//!   load-time validation (LAB-Q01);
//! * [`descriptor`]: the manifest types and their structural checks;
//! * [`validate`]: geometry, camera and reachability checks of every room,
//!   reporting explicit `validation_pending` where a check needs an
//!   integration fixture that does not exist yet;
//! * [`entry`]: the leak guard, which entry points may load a lab profile;
//! * [`runner`]: [`LabCatalog::launch`] and [`LabSession`], the room
//!   selection, the player modes and the restart.
//!
//! **Wiring (LAB-00-wire).** The live entry is a thin adapter: parse the
//! `--scene combat_lab` token, build a [`LabConfig`], call
//! [`LabCatalog::launch`] with [`EntryPoint::CombatLab`] and drive the returned
//! [`LabRun`] (`scene`, `combat`, `party` and `kit` are public) with the same
//! per-tick slot inputs as the party scene, or wrap it in a [`LabSession`] for
//! the room selection and restart. No rule lives in the adapter: after every
//! `party.restart()` (a party defeat's retry, the manual restart) the adapter
//! calls [`PlayerMode::seat`], which puts the human back on the mode's hero.

pub mod ability;
pub mod descriptor;
pub mod entry;
pub mod profile;
pub mod runner;
pub mod validate;

#[cfg(test)]
mod tests;

pub use descriptor::{
    EnemySpec, GoalKind, HazardSpec, Manifest, RoomSpec, VariantKind, VariantSpec,
};
pub use entry::{EntryPoint, is_lab_profile_id, reject_lab_profile_arg};
pub use profile::{CapabilityProfile, PROFILE_ID, ProfileSpec};
pub use runner::{
    EpisodeEnd, EpisodeRecord, LabCatalog, LabConfig, LabRun, LabSession, PlayerMode, ProbeTag,
    effective_seed,
};
pub use validate::{Check, CheckStatus, ValidationReport};

use std::fmt;

/// The directory of the lab data, relative to the repository root.
pub const LAB_DIR: &str = "assets/levels/combat_lab";
/// The manifest, relative to the repository root.
pub const MANIFEST_PATH: &str = "assets/levels/combat_lab/manifest.json";
/// The manifest schema id.
pub const MANIFEST_SCHEMA: &str = "combat-lab-manifest-v1";
/// The room ids, in lab order (R06 links the existing boss arena).
pub const ROOM_IDS: [&str; 7] = ["R00", "R01", "R02", "R03", "R04", "R05", "R06"];
/// LAB-Q04: the most enemies concurrently active in a natural run.
pub const LAB_MAX_CONCURRENT: usize = 4;
/// Whether `cf02_rules` is wired into the combat simulation. It is not on main
/// yet (CF02-I): a lab run that asks for it is refused instead of silently
/// replaying the legacy rules under a `cf02_rules` label.
pub const CF02_RULES_INTEGRATED: bool = false;

/// Everything that can stop a lab descriptor from loading or a run from
/// starting. The messages are descriptive on purpose (a rejected profile says
/// which ability and why).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabError {
    /// A file could not be read.
    Io(String),
    /// JSON that does not parse, or has an unknown or missing field (there is
    /// no `difficulty` field: LAB-Q08).
    Parse(String),
    /// A structural rule of the manifest.
    Schema { at: String, reason: String },
    /// An ability or move id the game has never heard of.
    UnknownAbility { at: String, id: String },
    /// A known ability that does not exist on main (dynamite, the gauntlet
    /// fist, Puddle Jumper, ...).
    UnimplementedAbility {
        at: String,
        id: String,
        reason: String,
    },
    /// An implemented ability the profile does not allow (Donut's swat
    /// post-biscuit, a CF02 move outside `cf02_moves`, ...).
    IllegalAbility {
        at: String,
        id: String,
        reason: String,
    },
    /// An enemy id the game has never heard of.
    UnknownEnemy { at: String, id: String },
    /// A known enemy that has no gameplay yet (LAB-04).
    UnimplementedEnemy {
        at: String,
        id: String,
        reason: String,
    },
    /// Geometry, camera or lane that does not resolve.
    Geometry { room: String, reason: String },
    /// A stored hash does not match the file or descriptor.
    HashMismatch {
        what: String,
        expected: String,
        actual: String,
    },
    /// A room that does not allow this mode, profile, variant or probe.
    NotAllowed { room: String, reason: String },
    /// No such room, variant or profile.
    Unknown { what: String, id: String },
    /// The leak guard: this entry point may not load a lab profile (LAB-Q01).
    EntryDenied { entry: &'static str, reason: String },
    /// `cf02_rules` was requested but is not integrated.
    Cf02NotIntegrated,
    /// The room links an existing scene (R06): the adapter starts that scene.
    Linked {
        room: String,
        scene: String,
        blockers: Vec<String>,
    },
}

impl fmt::Display for LabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(m) => write!(f, "combat lab: {m}"),
            Self::Parse(m) => write!(f, "combat lab manifest does not parse: {m}"),
            Self::Schema { at, reason } => write!(f, "combat lab: {at}: {reason}"),
            Self::UnknownAbility { at, id } => {
                write!(f, "combat lab: {at}: unknown ability or move id {id:?}")
            }
            Self::UnimplementedAbility { at, id, reason } => write!(
                f,
                "combat lab: {at}: ability {id:?} is not implemented on main ({reason})"
            ),
            Self::IllegalAbility { at, id, reason } => {
                write!(
                    f,
                    "combat lab: {at}: ability {id:?} is not legal here ({reason})"
                )
            }
            Self::UnknownEnemy { at, id } => {
                write!(f, "combat lab: {at}: unknown enemy id {id:?}")
            }
            Self::UnimplementedEnemy { at, id, reason } => write!(
                f,
                "combat lab: {at}: enemy {id:?} has no gameplay on main ({reason})"
            ),
            Self::Geometry { room, reason } => {
                write!(
                    f,
                    "combat lab: room {room}: geometry does not resolve: {reason}"
                )
            }
            Self::HashMismatch {
                what,
                expected,
                actual,
            } => write!(
                f,
                "combat lab: {what} hash is {actual}, the manifest says {expected}"
            ),
            Self::NotAllowed { room, reason } => {
                write!(f, "combat lab: room {room}: {reason}")
            }
            Self::Unknown { what, id } => write!(f, "combat lab: unknown {what} {id:?}"),
            Self::EntryDenied { entry, reason } => {
                write!(
                    f,
                    "combat lab: the {entry} entry cannot load a lab profile: {reason}"
                )
            }
            Self::Cf02NotIntegrated => write!(
                f,
                "combat lab: cf02_rules is not integrated into the combat simulation yet \
                 (CF02-I); the lab runs the legacy rules"
            ),
            Self::Linked {
                room,
                scene,
                blockers,
            } => write!(
                f,
                "combat lab: room {room} links the existing scene {scene:?} \
                 (open blockers: {})",
                blockers.join(", ")
            ),
        }
    }
}

impl std::error::Error for LabError {}

/// FNV-1a 64 over bytes, as every pinned hash of the repository.
pub fn fnv64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
