//! Reading, validating, listing and recovering sessions.

use std::path::{Path, PathBuf};

use super::codec::{
    ChunkTrailer, ControlRecord, SESSION_SCHEMA, TICKS_PER_CHUNK, TickRecord, chunk_file_name,
};
use super::error::PlaytraceError;
use super::manifest::{ChunkChecksum, SessionManifest, SessionStatus};
use super::writer::{CONTROL, FNV_START, MANIFEST, MANIFEST_TMP, fnv_bytes};

/// One chunk as found on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkInfo {
    pub index: usize,
    pub ticks: usize,
    pub fnv: u64,
    /// The trailer was present and its checksum matched.
    pub closed: bool,
}

/// A run of missing ticks `from..to` (a dropped-tick `incomplete` session).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickGap {
    pub from: u64,
    pub to: u64,
}

#[derive(Debug, Clone)]
pub struct LoadedSession {
    /// With the effective status: a manifest still saying `recording` is a
    /// crashed session and reads as `crashed_recovered`.
    pub manifest: SessionManifest,
    pub control: Vec<ControlRecord>,
    pub ticks: Vec<TickRecord>,
    pub chunks: Vec<ChunkInfo>,
    /// The session did not finish cleanly; the valid complete prefix is kept.
    pub recovered: bool,
    /// Unparseable trailing lines dropped by the recovery.
    pub partial_lines_dropped: usize,
    pub gaps: Vec<TickGap>,
}

fn read_manifest(dir: &Path) -> Result<SessionManifest, PlaytraceError> {
    let text =
        std::fs::read_to_string(dir.join(MANIFEST)).map_err(|_| PlaytraceError::MissingManifest)?;
    let manifest: SessionManifest =
        serde_json::from_str(&text).map_err(|_| PlaytraceError::MissingManifest)?;
    if manifest.schema != SESSION_SCHEMA {
        return Err(PlaytraceError::SchemaMismatch {
            found: manifest.schema,
            expected: SESSION_SCHEMA.to_owned(),
        });
    }
    Ok(manifest)
}

/// Split `text` into complete lines and a possible partial tail (no `\n`).
fn split_lines(text: &str) -> (Vec<&str>, Option<&str>) {
    let mut lines: Vec<&str> = text.split('\n').collect();
    let tail = lines.pop().filter(|t| !t.is_empty());
    (lines, tail)
}

fn read_control(
    dir: &Path,
    lenient: bool,
    dropped: &mut usize,
) -> Result<Vec<ControlRecord>, PlaytraceError> {
    let text = std::fs::read_to_string(dir.join(CONTROL)).unwrap_or_default();
    let (lines, tail) = split_lines(&text);
    let mut out = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        match serde_json::from_str::<ControlRecord>(line) {
            Ok(record) => out.push(record),
            Err(_) if lenient && index + 1 == lines.len() && tail.is_none() => *dropped += 1,
            Err(e) => {
                return Err(PlaytraceError::BadLine {
                    file: CONTROL.to_owned(),
                    line: index + 1,
                    why: e.to_string(),
                });
            }
        }
    }
    if tail.is_some() {
        if !lenient {
            return Err(PlaytraceError::BadLine {
                file: CONTROL.to_owned(),
                line: lines.len() + 1,
                why: "unterminated last line".to_owned(),
            });
        }
        *dropped += 1;
    }
    Ok(out)
}

fn chunk_indices(dir: &Path) -> Result<Vec<usize>, PlaytraceError> {
    let mut found: Vec<usize> = std::fs::read_dir(dir)?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let digits = name.strip_prefix("ticks-")?.strip_suffix(".jsonl")?;
            digits.parse().ok()
        })
        .collect();
    found.sort_unstable();
    for (expected, index) in found.iter().enumerate() {
        if *index != expected {
            return Err(PlaytraceError::MissingChunk { index: expected });
        }
    }
    Ok(found)
}

/// Read and validate a session directory.
///
/// Strict rules: every closed chunk's checksum and tick count match its
/// trailer; chunks are numbered without a hole; ticks are strictly increasing
/// (no duplicate, no reorder); only the last chunk may lack a trailer, and
/// only in a session that did not finish (crash). Missing ticks (a dropped
/// tick of an `incomplete` session) are an error unless `allow_gaps`.
pub fn read_session(dir: &Path, allow_gaps: bool) -> Result<LoadedSession, PlaytraceError> {
    let mut manifest = read_manifest(dir)?;
    let crashed = manifest.status == SessionStatus::Recording;
    let mut dropped = 0;
    let control = read_control(dir, crashed, &mut dropped)?;
    let indices = chunk_indices(dir)?;
    let mut ticks: Vec<TickRecord> = Vec::new();
    let mut chunks = Vec::new();
    let mut gaps = Vec::new();
    for (position, index) in indices.iter().enumerate() {
        let last = position + 1 == indices.len();
        let name = chunk_file_name(*index);
        let text = std::fs::read_to_string(dir.join(&name)).map_err(PlaytraceError::from)?;
        let (lines, tail) = split_lines(&text);
        let mut hash = FNV_START;
        let mut count = 0_usize;
        let mut trailer: Option<ChunkTrailer> = None;
        for (number, line) in lines.iter().enumerate() {
            if line.starts_with("{\"end\":") {
                let parsed: ChunkTrailer =
                    serde_json::from_str(line).map_err(|e| PlaytraceError::BadLine {
                        file: name.clone(),
                        line: number + 1,
                        why: e.to_string(),
                    })?;
                if number + 1 != lines.len() || tail.is_some() {
                    return Err(PlaytraceError::BadLine {
                        file: name.clone(),
                        line: number + 1,
                        why: "data after the trailer".to_owned(),
                    });
                }
                trailer = Some(parsed);
                break;
            }
            let tick: TickRecord = match serde_json::from_str(line) {
                Ok(tick) => tick,
                Err(_) if crashed && last && number + 1 == lines.len() && tail.is_none() => {
                    dropped += 1;
                    break;
                }
                Err(e) => {
                    return Err(PlaytraceError::BadLine {
                        file: name,
                        line: number + 1,
                        why: e.to_string(),
                    });
                }
            };
            if let Some(previous) = ticks.last() {
                if tick.t <= previous.t {
                    return Err(PlaytraceError::NonContiguousTick {
                        expected: previous.t + 1,
                        found: tick.t,
                    });
                }
                if tick.t != previous.t + 1 {
                    gaps.push(TickGap {
                        from: previous.t + 1,
                        to: tick.t,
                    });
                }
            } else if tick.t != 0 {
                gaps.push(TickGap {
                    from: 0,
                    to: tick.t,
                });
            }
            hash = fnv_bytes(hash, line.as_bytes());
            hash = fnv_bytes(hash, b"\n");
            count += 1;
            ticks.push(tick);
        }
        if tail.is_some() {
            if !(crashed && last) {
                return Err(PlaytraceError::BadLine {
                    file: name,
                    line: lines.len() + 1,
                    why: "unterminated last line".to_owned(),
                });
            }
            dropped += 1;
        }
        match trailer {
            Some(trailer) => {
                if trailer.chunk != *index || trailer.ticks != count || trailer.fnv != hash {
                    return Err(PlaytraceError::ChecksumMismatch { chunk: *index });
                }
                chunks.push(ChunkInfo {
                    index: *index,
                    ticks: count,
                    fnv: hash,
                    closed: true,
                });
            }
            None if last
                && (crashed
                    || matches!(
                        manifest.status,
                        SessionStatus::CrashedRecovered | SessionStatus::Incomplete
                    )) =>
            {
                chunks.push(ChunkInfo {
                    index: *index,
                    ticks: count,
                    fnv: hash,
                    closed: false,
                });
            }
            None => return Err(PlaytraceError::MissingTrailer { chunk: *index }),
        }
    }
    if !gaps.is_empty() && !allow_gaps {
        let gap = gaps[0];
        return Err(PlaytraceError::TickGap {
            from: gap.from,
            to: gap.to,
        });
    }
    if matches!(
        manifest.status,
        SessionStatus::Completed | SessionStatus::Abandoned
    ) {
        check_manifest(&manifest, &ticks, &chunks)?;
    }
    let recovered = crashed || manifest.status == SessionStatus::CrashedRecovered;
    if crashed {
        manifest.status = SessionStatus::CrashedRecovered;
        manifest.terminal = Some("truncated".to_owned());
        manifest.tick_count = ticks.len() as u64;
        manifest.last_tick = ticks.last().map(|t| t.t);
        manifest.capture_complete = false;
    }
    Ok(LoadedSession {
        manifest,
        control,
        ticks,
        chunks,
        recovered,
        partial_lines_dropped: dropped,
        gaps,
    })
}

fn check_manifest(
    manifest: &SessionManifest,
    ticks: &[TickRecord],
    chunks: &[ChunkInfo],
) -> Result<(), PlaytraceError> {
    let mismatch = |why: &str| Err(PlaytraceError::ManifestMismatch(why.to_owned()));
    if manifest.tick_count != ticks.len() as u64 {
        return mismatch("tick_count");
    }
    if manifest.last_tick != ticks.last().map(|t| t.t) {
        return mismatch("last_tick");
    }
    let found: Vec<ChunkChecksum> = chunks
        .iter()
        .filter(|c| c.closed)
        .map(|c| ChunkChecksum {
            chunk: c.index,
            ticks: c.ticks,
            fnv: c.fnv,
        })
        .collect();
    if manifest.chunk_checksums != found {
        return mismatch("chunk_checksums");
    }
    debug_assert!(chunks.iter().all(|c| c.ticks <= TICKS_PER_CHUNK));
    Ok(())
}

/// Seal a crashed session in place: truncate the partial tail line, close the
/// last chunk with a trailer over its complete lines and rewrite the manifest
/// as `crashed_recovered` with a `truncated` terminal. Nothing is invented:
/// only the valid complete prefix remains. A finished session is left alone.
pub fn recover_session(dir: &Path) -> Result<SessionManifest, PlaytraceError> {
    let raw = read_manifest(dir)?;
    if raw.status != SessionStatus::Recording {
        return Ok(raw);
    }
    let loaded = read_session(dir, true)?;
    // Rewrite the control file and the last chunk to their valid prefixes.
    let mut control_text = String::new();
    for record in &loaded.control {
        control_text.push_str(
            &serde_json::to_string(record).map_err(|e| PlaytraceError::Io(e.to_string()))?,
        );
        control_text.push('\n');
    }
    std::fs::write(dir.join(CONTROL), control_text)?;
    let mut manifest = loaded.manifest;
    let mut checksums: Vec<ChunkChecksum> = Vec::new();
    for chunk in &loaded.chunks {
        let (first, last) = chunk_range(&loaded.ticks, &loaded.chunks, chunk.index);
        if !chunk.closed {
            let mut text = String::new();
            for tick in &loaded.ticks[first..last] {
                text.push_str(
                    &serde_json::to_string(tick).map_err(|e| PlaytraceError::Io(e.to_string()))?,
                );
                text.push('\n');
            }
            let trailer = ChunkTrailer {
                end: true,
                chunk: chunk.index,
                ticks: chunk.ticks,
                first_t: loaded.ticks.get(first).map(|t| t.t),
                last_t: last
                    .checked_sub(1)
                    .and_then(|i| loaded.ticks.get(i))
                    .map(|t| t.t),
                fnv: chunk.fnv,
            };
            text.push_str(
                &serde_json::to_string(&trailer).map_err(|e| PlaytraceError::Io(e.to_string()))?,
            );
            text.push('\n');
            std::fs::write(dir.join(chunk_file_name(chunk.index)), text)?;
        }
        checksums.push(ChunkChecksum {
            chunk: chunk.index,
            ticks: chunk.ticks,
            fnv: chunk.fnv,
        });
    }
    manifest.chunk_checksums = checksums;
    manifest.status = SessionStatus::CrashedRecovered;
    manifest.terminal = Some("truncated".to_owned());
    manifest.capture_complete = false;
    manifest.training_eligible = false;
    if !manifest
        .quality_flags
        .iter()
        .any(|f| f == "crashed_recovered")
    {
        manifest.quality_flags.push("crashed_recovered".to_owned());
    }
    let text =
        serde_json::to_string_pretty(&manifest).map_err(|e| PlaytraceError::Io(e.to_string()))?;
    let tmp = dir.join(MANIFEST_TMP);
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, dir.join(MANIFEST))?;
    Ok(manifest)
}

/// Index range of chunk `index` within the flat tick list.
fn chunk_range(_ticks: &[TickRecord], chunks: &[ChunkInfo], index: usize) -> (usize, usize) {
    let first: usize = chunks
        .iter()
        .take_while(|c| c.index < index)
        .map(|c| c.ticks)
        .sum();
    let last = first
        + chunks
            .iter()
            .find(|c| c.index == index)
            .map_or(0, |c| c.ticks);
    (first, last)
}

/// Every session below `base` that has a readable manifest, oldest id first.
/// Does not read the chunks (cheap: used by `list`).
pub fn list_sessions(base: &Path) -> Vec<(PathBuf, SessionManifest)> {
    let Ok(entries) = std::fs::read_dir(base) else {
        return Vec::new();
    };
    let mut out: Vec<(PathBuf, SessionManifest)> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let manifest = read_manifest(&path).ok()?;
            Some((path, manifest))
        })
        .collect();
    out.sort_by(|a, b| a.1.session_id.cmp(&b.1.session_id));
    out
}
