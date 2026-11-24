//! The append-only durable writer: `control.jsonl` plus 600-tick chunks, with
//! a bounded queue so a failing disk never stops (or silently loses) gameplay.

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use super::codec::{
    ChunkTrailer, ControlEvent, ControlRecord, DiagRecord, EpisodeBoundary, QUEUE_BOUND,
    TICKS_PER_CHUNK, TickRecord, chunk_file_name,
};
use super::error::PlaytraceError;
use super::manifest::{ChunkChecksum, SessionManifest, SessionStatus, StreamIntegrity};

pub(super) const MANIFEST: &str = "manifest.json";
pub(super) const MANIFEST_TMP: &str = "manifest.json.tmp";
pub(super) const CONTROL: &str = "control.jsonl";
pub(super) const DIAGNOSTICS: &str = "diagnostics.jsonl";
/// Diagnostics lines are batched (one write per batch) so the privileged stream
/// costs one syscall per second, not one per tick.
pub(super) const DIAG_BATCH: usize = 60;

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
    /// Diagnostics lines lost to a full queue (optional stream: does not make
    /// the capture incomplete).
    pub dropped_diagnostics: u64,
    /// Failed intermediate manifest writes at chunk rotation: no data is lost,
    /// but the disk is misbehaving (PT-C follow-up 5).
    pub manifest_write_errors: u64,
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

/// A queued write. Every `String` carries its own trailing `\n`.
enum Op {
    Control(String),
    Tick { chunk: usize, line: String },
    Close { chunk: usize, trailer: String },
    Diag(String),
}

/// An append-only file and the length of the bytes known to be written: a
/// failed or short write is truncated back to `len`, so a retry never leaves a
/// partial line in the middle of a stream.
struct Out {
    file: File,
    len: u64,
}

impl Out {
    fn open(path: PathBuf) -> std::io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let len = file.metadata()?.len();
        Ok(Self { file, len })
    }

    /// Append `bytes` whole or not at all. `short` is the test hook: write that
    /// many bytes, then fail like a full disk.
    fn append(&mut self, bytes: &[u8], short: &mut Option<usize>) -> std::io::Result<()> {
        let result = match short.take() {
            Some(n) => self
                .file
                .write_all(&bytes[..n.min(bytes.len())])
                .and_then(|()| Err(std::io::ErrorKind::StorageFull.into())),
            None => self.file.write_all(bytes),
        };
        match result {
            Ok(()) => {
                self.len += bytes.len() as u64;
                Ok(())
            }
            Err(error) => {
                // Drop whatever part of the line reached the file. If even
                // that fails the next write is a retry of a corrupt stream;
                // the error is counted either way.
                let _ = self.file.set_len(self.len);
                Err(error)
            }
        }
    }
}

pub struct SessionWriter {
    dir: PathBuf,
    manifest: SessionManifest,
    /// Held open for the session lifetime: it carries the live lock.
    control_lock: Option<File>,
    control_out: Option<Out>,
    diag_out: Option<Out>,
    chunk_file: Option<(usize, Out)>,
    queue: VecDeque<Op>,
    queued_ticks: usize,
    queued_control: usize,
    queued_diag: usize,
    /// `control.jsonl` as accepted: line count and FNV of the lines.
    control_lines: u64,
    control_hash: u64,
    /// `diagnostics.jsonl` as accepted.
    diag_lines: u64,
    diag_hash: u64,
    /// Diagnostics lines waiting for a full batch.
    diag_batch: String,
    diag_batch_lines: usize,
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
    /// Test hook: the next append writes this many bytes, then fails.
    pub(super) short_write: Option<usize>,
    /// Test hook: the next N intermediate manifest writes fail.
    pub(super) fail_manifest_writes: u32,
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
            control_lock: Some(control_file),
            control_out: None,
            diag_out: None,
            chunk_file: None,
            queue: VecDeque::new(),
            queued_ticks: 0,
            queued_control: 0,
            queued_diag: 0,
            control_lines: 0,
            control_hash: FNV_START,
            diag_lines: 0,
            diag_hash: FNV_START,
            diag_batch: String::new(),
            diag_batch_lines: 0,
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
            short_write: None,
            fail_manifest_writes: 0,
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
        let mut line =
            serde_json::to_string(&record).map_err(|e| PlaytraceError::Io(e.to_string()))?;
        line.push('\n');
        if self.queued_control >= QUEUE_BOUND {
            // The `seq` gap is the evidence; only an `incomplete` session may
            // have one (the reader checks).
            self.health.dropped_control += 1;
        } else {
            self.control_lines += 1;
            self.control_hash = fnv_bytes(self.control_hash, line.as_bytes());
            self.queued_control += 1;
            self.queue.push_back(Op::Control(line));
        }
        self.flush();
        Ok(())
    }

    /// Append one privileged diagnostics line (batched; see [`DIAG_BATCH`]).
    /// Never an error for the caller: a full queue drops the line and counts
    /// it, and the stream is optional for completeness.
    pub fn record_diag(&mut self, diag: &DiagRecord) -> Result<(), PlaytraceError> {
        if self.finished {
            return Err(PlaytraceError::Finished);
        }
        let mut line =
            serde_json::to_string(diag).map_err(|e| PlaytraceError::Io(e.to_string()))?;
        line.push('\n');
        if self.queued_diag + self.diag_batch_lines >= QUEUE_BOUND {
            self.health.dropped_diagnostics += 1;
            return Ok(());
        }
        self.diag_lines += 1;
        self.diag_hash = fnv_bytes(self.diag_hash, line.as_bytes());
        self.diag_batch.push_str(&line);
        self.diag_batch_lines += 1;
        if self.diag_batch_lines >= DIAG_BATCH {
            self.queue_diag_batch();
            self.flush();
        }
        Ok(())
    }

    fn queue_diag_batch(&mut self) {
        if self.diag_batch_lines == 0 {
            return;
        }
        self.queued_diag += self.diag_batch_lines;
        self.diag_batch_lines = 0;
        let batch = std::mem::take(&mut self.diag_batch);
        self.queue.push_back(Op::Diag(batch));
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
            final_obs: None,
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
            self.queue_diag_batch();
        }
        self.flush();
        if rotated {
            self.write_rotation_manifest();
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
        let mut short = self.short_write.take();
        let result = match op {
            Op::Control(line) => {
                if self.control_out.is_none() {
                    self.control_out = Some(Out::open(self.dir.join(CONTROL))?);
                }
                let out = self.control_out.as_mut().expect("opened above");
                out.append(line.as_bytes(), &mut short)
            }
            Op::Diag(batch) => {
                if self.diag_out.is_none() {
                    self.diag_out = Some(Out::open(self.dir.join(DIAGNOSTICS))?);
                }
                let out = self.diag_out.as_mut().expect("opened above");
                out.append(batch.as_bytes(), &mut short)
            }
            Op::Tick { chunk, line } => self
                .chunk_handle(*chunk)?
                .append(line.as_bytes(), &mut short),
            Op::Close { chunk, trailer } => {
                let out = self.chunk_handle(*chunk)?;
                out.append(trailer.as_bytes(), &mut short)?;
                // Durability against power loss is not on the game thread: the
                // data is already in the page cache (it survives a killed
                // process); a detached thread flushes it to the device. The
                // hitch it could cause on slow storage is measured in PT-E.
                if let Ok(clone) = out.file.try_clone() {
                    std::thread::spawn(move || {
                        let _ = clone.sync_all();
                    });
                }
                self.chunk_file = None;
                Ok(())
            }
        };
        // A hook that was not consumed (the op wrote nothing) stays armed.
        if short.is_some() {
            self.short_write = short;
        }
        result
    }

    fn chunk_handle(&mut self, chunk: usize) -> std::io::Result<&mut Out> {
        if self
            .chunk_file
            .as_ref()
            .is_none_or(|(index, _)| *index != chunk)
        {
            let out = Out::open(self.dir.join(chunk_file_name(chunk)))?;
            self.chunk_file = Some((chunk, out));
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
                    Op::Diag(batch) => {
                        self.queued_diag -= batch.bytes().filter(|b| *b == b'\n').count();
                    }
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

    /// The intermediate manifest at a chunk rotation. A failure loses no data
    /// (the chunk is already queued and the final manifest is rewritten at
    /// finish), but it is counted, never swallowed.
    fn write_rotation_manifest(&mut self) {
        if self.fail_manifest_writes > 0 {
            self.fail_manifest_writes -= 1;
            self.note_manifest_failure(std::io::ErrorKind::StorageFull);
            return;
        }
        if self.write_manifest().is_err() {
            self.note_manifest_failure(std::io::ErrorKind::Other);
        }
    }

    fn note_manifest_failure(&mut self, kind: std::io::ErrorKind) {
        self.health.manifest_write_errors += 1;
        self.health.last_error = Some(format!("{kind:?}"));
    }

    /// Fold the counters into the manifest and write it atomically.
    fn write_manifest(&mut self) -> Result<(), PlaytraceError> {
        let manifest = &mut self.manifest;
        manifest.tick_count = self.tick_count;
        manifest.last_tick = self.last_tick;
        manifest.dropped_ticks = self.health.dropped_ticks;
        manifest.write_errors = self.health.write_errors;
        manifest.dropped_control = self.health.dropped_control;
        manifest.dropped_diagnostics = self.health.dropped_diagnostics;
        manifest.manifest_write_errors = self.health.manifest_write_errors;
        manifest.control_integrity = Some(StreamIntegrity {
            lines: self.control_lines,
            fnv: self.control_hash,
        });
        manifest.diagnostics_integrity = (self.diag_lines > 0).then_some(StreamIntegrity {
            lines: self.diag_lines,
            fnv: self.diag_hash,
        });
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
        self.queue_diag_batch();
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
        self.release_lock();
        self.control_out = None;
        self.diag_out = None;
        let manifest = self.manifest.clone();
        Ok(manifest)
    }

    /// Drop the session lock, unlocking it first. `flock` belongs to the open
    /// file description, not to this handle: a child process forked by another
    /// thread (a test spawning a subprocess, or the game launching a tool)
    /// holds a duplicate of every descriptor until its `exec`, so closing ours
    /// alone would leave the session looking live for that window
    /// (CF02-FU-TEST: `recover_session` returned `SessionLive` once in CI).
    /// An explicit unlock releases the description's lock for every duplicate.
    fn release_lock(&mut self) {
        if let Some(file) = self.control_lock.take() {
            let _ = file.unlock();
        }
    }

    /// Test hook: die like a killed process (nothing is finished, the lock is
    /// released, buffers are not flushed).
    #[cfg(test)]
    pub(super) fn crash_for_test(mut self) {
        self.release_lock();
        std::mem::forget(self);
    }

    /// Test hook: a duplicate of the lock handle, as a child forked before its
    /// `exec` would hold it.
    #[cfg(test)]
    pub(super) fn lock_duplicate_for_test(&self) -> File {
        self.control_lock
            .as_ref()
            .expect("a live writer holds the lock")
            .try_clone()
            .expect("dup the lock handle")
    }
}

impl Drop for SessionWriter {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.finish(SessionStatus::Abandoned);
        }
    }
}
