//! PLAYTRACE-01: durable, observer-only recording of gameplay sessions.
//!
//! This module is Bevy-free and never touches the simulation. PT-A is the
//! codec and the on-disk journal (`playtrace-session/1`); PT-B adds the live
//! adapter ([`live`]) that the `floor01_scene` fixed step calls.
//!
//! # Storage
//!
//! One directory per session, `<base>/<session_id>/`, where `<base>` is
//! `--record-dir DIR` or [`default_record_dir`]
//! (`$XDG_DATA_HOME/dungeon-floorbreak/playtrace`, falling back to
//! `~/.local/share/dungeon-floorbreak/playtrace`):
//!
//! - `manifest.json`: [`SessionManifest`], written atomically
//!   (`manifest.json.tmp` then rename) at start and again at finish.
//! - `control.jsonl`: ordered [`ControlRecord`] lines (episode boundaries,
//!   restarts, swaps, pad and pause lifecycle, drive changes).
//! - `ticks-NNNNNN.jsonl`: [`TICKS_PER_CHUNK`] (600) [`TickRecord`] lines per
//!   chunk, closed by a trailer line with an FNV-1a-64 checksum of the lines
//!   before it. No compression, no new crate.
//!
//! # Flags (live `floor01_scene` runs, PT-B)
//!
//! `--record` forces recording on, `--no-record` turns it off with a visible
//! `PLAYTRACE_DISABLED` line, `--record-dir DIR` overrides the base directory.
//! The default is [`RECORD_DEFAULT_ON`] for runs with a live input source and
//! off for scripted, headless, CI, capture and presim paths. Changing the
//! constant for a distributed build needs a new ADR (ADR-0011).
//!
//! # Guarantees
//!
//! - Actions are stored bit-exact ([`ActionBits`]): analog stick values are the
//!   `f32` bit pattern, buttons a bitmask. Nothing is quantized and no press
//!   edge is synthesized.
//! - A write error never stops gameplay: the writer keeps a bounded queue
//!   ([`QUEUE_BOUND`] ticks), marks the session `incomplete` and counts any
//!   dropped tick ([`WriterHealth`]). Inputs are never dropped silently.
//! - A crashed session keeps its valid complete prefix ([`recover_session`]).
//! - Manifests hold no hostname, user name, home path, pad id or serial.

mod codec;
mod error;
pub mod fake;
pub mod live;
mod manifest;
mod reader;
mod storage;
mod writer;

#[cfg(test)]
mod live_tests;
#[cfg(test)]
mod tests;

pub use codec::{
    ACTION_SCHEMA, ActionBits, ChunkTrailer, ControlEvent, ControlRecord, ControlSource,
    EVENT_SCHEMA, EpisodeBoundary, EpisodeReason, EventRecord, QUEUE_BOUND, SESSION_SCHEMA,
    TICKS_PER_CHUNK, TickRecord, chunk_file_name, hex64,
};
pub use error::PlaytraceError;
pub use manifest::{
    ChunkChecksum, Participants, ResetDescriptor, RunKind, SessionManifest, SessionStatus,
    generate_session_id,
};
pub use reader::{
    ChunkInfo, LoadedSession, TickGap, list_sessions, read_session, recover_session,
    session_is_live,
};
pub use storage::{WARN_BYTES, default_record_dir, dir_size_bytes};
pub use writer::{SessionWriter, WriterHealth};

/// The one constant that holds the default for live-input runs (ADR-0011).
pub const RECORD_DEFAULT_ON: bool = true;
