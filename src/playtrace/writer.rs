//! The append-only durable writer: `control.jsonl` plus 600-tick chunks, with
//! a bounded queue so a failing disk never stops (or silently loses) gameplay.

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use super::codec::{
    ChunkTrailer, ControlEvent, ControlRecord, EpisodeBoundary, QUEUE_BOUND, TICKS_PER_CHUNK,
    TickRecord, chunk_file_name,
};
use super::error::PlaytraceError;
use super::manifest::{ChunkChecksum, SessionManifest, SessionStatus};

pub(super) const MANIFEST: &str = "manifest.json";
pub(super) const MANIFEST_TMP: &str = "manifest.json.tmp";
pub(super) const CONTROL: &str = "control.jsonl";

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a-64 folded over `bytes` starting from `hash`.
pub(super) fn fnv_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

pub(super) const FNV_START: u64 = FNV_OFFSET;

/// Counters the HUD and the log read: a non-zero value means the recording is
/// not complete, and the game says so.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WriterHealth {
    pub write_errors: u64,
    pub dropped_ticks: u64,
    pub dropped_control: u64,
    /// Operations waiting for the disk.
    pub queued: usize,
    /// `io::ErrorKind` of the latest failure (no path).
    pub last_error: Option<String>,
}

impl WriterHealth {
    pub fn incomplete(&self) -> bool {
        self.write_errors > 0 || self.dropped_ticks > 0 || self.dropped_control > 0
    }
}

enum Op {
    Control(String),
    Tick { chunk: usize, line: String },
    Close { chunk: usize, trailer: String },
}

pub struct SessionWriter {
    dir: PathBuf,
    manifest: SessionManifest,
    control_file: Option<File>,
    chunk_file: Option<(usize, File)>,
    queue: VecDeque<Op>,
    queued_ticks: usize,
    queued_control: usize,
    seq: u64,
    next_t: u64,
    chunk: usize,
    chunk_ticks: usize,
    chunk_hash: u64,
    chunk_first: Option<u64>,
    chunk_last: Option<u64>,
    checksums: Vec<ChunkChecksum>,
    tick_count: u64,
    last_tick: Option<u64>,
    health: WriterHealth,
    finished: bool,
    /// Test hook: the next N file operations fail with `StorageFull`.
    pub(super) fail_writes: u32,
}

impl SessionWriter {
    /// Create `<base>/<session_id>/`, the control file and the first manifest
    /// (`status: recording`). Any error here means nothing is recorded: the
    /// caller reports it and the game runs on.
    pub fn create(base: &Path, manifest: SessionManifest) -> Result<Self, PlaytraceError> {
        std::fs::create_dir_all(base)?;
        let dir = base.join(&manifest.session_id);
        match std::fs::create_dir(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(PlaytraceError::Exists(dir));
            }
            Err(e) => return Err(e.into()),
        }
        let control_file = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(dir.join(CONTROL))?;
        // The lock lives as long as this handle: `recover` and `delete` refuse
        // a session whose writer is alive (a crash releases it).
        control_file.try_lock().map_err(|e| match e {
            std::fs::TryLockError::WouldBlock => PlaytraceError::SessionLive,
            std::fs::TryLockError::Error(e) => PlaytraceError::from(e),
        })?;
        let mut writer = Self {
            dir,
            manifest,
            control_file: Some(control_file),
            chunk_file: None,
            queue: VecDeque::new(),
            queued_ticks: 0,
            queued_control: 0,
            seq: 0,
            next_t: 0,
            chunk: 0,
            chunk_ticks: 0,
            chunk_hash: FNV_START,
            chunk_first: None,
            chunk_last: None,
            checksums: Vec::new(),
            tick_count: 0,
            last_tick: None,
            health: WriterHealth::default(),
            finished: false,
            fail_writes: 0,
        };
        writer.write_manifest()?;
        Ok(writer)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn session_id(&self) -> &str {
        &self.manifest.session_id
    }

    /// The global tick the next [`Self::record_tick`] must carry.
    pub fn next_t(&self) -> u64 {
        self.next_t
    }

    pub fn health(&self) -> WriterHealth {
        WriterHealth {
            queued: self.queue.len(),
            ..self.health.clone()
        }
    }

    pub fn manifest(&self) -> &SessionManifest {
        &self.manifest
    }

    /// Mutable manifest fields the caller may refine before finish (flags,
    /// participants, mode). Identity fields should not be changed.
    pub fn manifest_mut(&mut self) -> &mut SessionManifest {
        &mut self.manifest
    }

    /// Append a control event at the current tick boundary (the event happened
    /// before tick [`Self::next_t`] was stepped).
    pub fn record_control(&mut self, ev: ControlEvent) -> Result<(), PlaytraceError> {
        if self.finished {
            return Err(PlaytraceError::Finished);
        }
        let record = ControlRecord {
            seq: self.seq,
            at_t: self.next_t,
            ev,
        };
        self.seq += 1;
        let line = serde_json::to_string(&record).map_err(|e| PlaytraceError::Io(e.to_string()))?;
        if self.queued_control >= QUEUE_BOUND {
            self.health.dropped_control += 1;
        } else {
            self.queued_control += 1;
            self.queue.push_back(Op::Control(line));
        }
        self.flush();
        Ok(())
    }

    pub fn begin_episode(&mut self, boundary: EpisodeBoundary) -> Result<(), PlaytraceError> {
        self.record_control(ControlEvent::EpisodeBegin {
            episode: boundary.episode,
            reason: boundary.reason,
            stage: boundary.stage,
        })
    }

    pub fn end_episode(&mut self, episode: u32, outcome: &str) -> Result<(), PlaytraceError> {
        self.record_control(ControlEvent::EpisodeEnd {
            episode,
            outcome: outcome.to_owned(),
        })
    }

    /// Append one tick. `Err` is a caller bug (a repeated, skipped or
    /// out-of-order tick); a disk problem is absorbed into [`Self::health`].
    pub fn record_tick(&mut self, tick: &TickRecord) -> Result<(), PlaytraceError> {
        if self.finished {
            return Err(PlaytraceError::Finished);
        }
        if tick.t != self.next_t {
            return Err(PlaytraceError::NonContiguousTick {
                expected: self.next_t,
                found: tick.t,
            });
        }
        self.next_t += 1;
        let mut line =
            serde_json::to_string(tick).map_err(|e| PlaytraceError::Io(e.to_string()))?;
        line.push('\n');
        if self.queued_ticks >= QUEUE_BOUND {
            // Backpressure: the tick is lost, loudly (counter, `incomplete`).
            self.health.dropped_ticks += 1;
            self.flush();
            return Ok(());
        }
        self.chunk_hash = fnv_bytes(self.chunk_hash, line.as_bytes());
        self.chunk_first.get_or_insert(tick.t);
        self.chunk_last = Some(tick.t);
        self.chunk_ticks += 1;
        self.tick_count += 1;
        self.last_tick = Some(tick.t);
        self.queued_ticks += 1;
        self.queue.push_back(Op::Tick {
            chunk: self.chunk,
            line,
        });
        let rotated = self.chunk_ticks == TICKS_PER_CHUNK;
        if rotated {
            self.close_chunk();
        }
        self.flush();
        if rotated {
            let _ = self.write_manifest();
        }
        Ok(())
    }

    fn close_chunk(&mut self) {
        let trailer = ChunkTrailer {
            end: true,
            chunk: self.chunk,
            ticks: self.chunk_ticks,
            first_t: self.chunk_first,
            last_t: self.chunk_last,
            fnv: self.chunk_hash,
        };
        self.checksums.push(ChunkChecksum {
            chunk: self.chunk,
            ticks: self.chunk_ticks,
            fnv: self.chunk_hash,
        });
        let mut line = serde_json::to_string(&trailer).unwrap_or_default();
        line.push('\n');
        // A trailer is never dropped: it is one per 600 ticks.
        self.queue.push_back(Op::Close {
            chunk: self.chunk,
            trailer: line,
        });
        self.chunk += 1;
        self.chunk_ticks = 0;
        self.chunk_hash = FNV_START;
        self.chunk_first = None;
        self.chunk_last = None;
    }

    fn fail_injected(&mut self) -> std::io::Result<()> {
        if self.fail_writes > 0 {
            self.fail_writes -= 1;
            return Err(std::io::ErrorKind::StorageFull.into());
        }
        Ok(())
    }

    fn apply(&mut self, op: &Op) -> std::io::Result<()> {
        self.fail_injected()?;
        match op {
            Op::Control(line) => {
                if self.control_file.is_none() {
                    self.control_file = Some(
                        OpenOptions::new()
                            .append(true)
                            .open(self.dir.join(CONTROL))?,
                    );
                }
                let file = self.control_file.as_mut().expect("opened above");
                file.write_all(line.as_bytes())?;
                file.write_all(b"\n")
            }
            Op::Tick { chunk, line } => self.chunk_handle(*chunk)?.write_all(line.as_bytes()),
            Op::Close { chunk, trailer } => {
                let file = self.chunk_handle(*chunk)?;
                file.write_all(trailer.as_bytes())?;
                // Durability against power loss is not on the game thread: the
                // data is already in the page cache (it survives a killed
                // process); a detached thread flushes it to the device. The
                // hitch it could cause on slow storage is measured in PT-E.
                if let Ok(clone) = file.try_clone() {
                    std::thread::spawn(move || {
                        let _ = clone.sync_all();
                    });
                }
                self.chunk_file = None;
                Ok(())
            }
        }
    }

    fn chunk_handle(&mut self, chunk: usize) -> std::io::Result<&mut File> {
        if self
            .chunk_file
            .as_ref()
            .is_none_or(|(index, _)| *index != chunk)
        {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.dir.join(chunk_file_name(chunk)))?;
            self.chunk_file = Some((chunk, file));
        }
        Ok(&mut self.chunk_file.as_mut().expect("opened above").1)
    }

    /// Try to write everything queued; stop at the first failure and keep the
    /// rest (retried on the next record).
    fn flush(&mut self) {
        while let Some(op) = self.queue.pop_front() {
            match self.apply(&op) {
                Ok(()) => match op {
                    Op::Tick { .. } => self.queued_ticks -= 1,
                    Op::Control(_) => self.queued_control -= 1,
                    Op::Close { .. } => {}
                },
                Err(error) => {
                    self.health.write_errors += 1;
                    self.health.last_error = Some(format!("{:?}", error.kind()));
                    self.queue.push_front(op);
                    return;
                }
            }
        }
    }

    /// Fold the counters into the manifest and write it atomically.
    fn write_manifest(&mut self) -> Result<(), PlaytraceError> {
        let manifest = &mut self.manifest;
        manifest.tick_count = self.tick_count;
        manifest.last_tick = self.last_tick;
        manifest.dropped_ticks = self.health.dropped_ticks;
        manifest.write_errors = self.health.write_errors;
        manifest.chunk_checksums = self.checksums.clone();
        if self.health.incomplete() {
            manifest.capture_complete = false;
            manifest.training_eligible = false;
            if !manifest
                .quality_flags
                .iter()
                .any(|f| f == "capture_incomplete")
            {
                manifest.quality_flags.push("capture_incomplete".to_owned());
            }
        }
        let text = serde_json::to_string_pretty(&self.manifest)
            .map_err(|e| PlaytraceError::Io(e.to_string()))?;
        let tmp = self.dir.join(MANIFEST_TMP);
        self.fail_injected()?;
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, self.dir.join(MANIFEST))?;
        Ok(())
    }

    /// Close the open chunk, flush and write the final manifest. The status
    /// becomes `incomplete` when anything failed or was dropped, whatever was
    /// asked for. Returns the final manifest.
    pub fn finish(&mut self, status: SessionStatus) -> Result<SessionManifest, PlaytraceError> {
        if self.finished {
            return Err(PlaytraceError::Finished);
        }
        if self.chunk_ticks > 0 {
            self.close_chunk();
        }
        self.flush();
        self.finished = true;
        let failed = self.health.incomplete() || !self.queue.is_empty();
        self.manifest.status = if failed {
            SessionStatus::Incomplete
        } else {
            status
        };
        self.manifest.capture_complete = !failed;
        self.write_manifest()?;
        // Release the session lock: the recording is no longer live.
        self.control_file = None;
        let manifest = self.manifest.clone();
        Ok(manifest)
    }

    /// Test hook: die like a killed process (nothing is finished, the lock is
    /// released, buffers are not flushed).
    #[cfg(test)]
    pub(super) fn crash_for_test(mut self) {
        self.control_file = None;
        std::mem::forget(self);
    }
}

impl Drop for SessionWriter {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.finish(SessionStatus::Abandoned);
        }
    }
}
