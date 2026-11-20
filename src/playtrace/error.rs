use std::fmt;
use std::path::PathBuf;

/// Everything the recorder, the reader and the recovery can fail with. The
/// messages carry no machine-specific path: the caller prints them as given.
#[derive(Debug)]
pub enum PlaytraceError {
    /// The session directory or one of its files cannot be created or written.
    Io(String),
    /// No data directory could be derived (no `XDG_DATA_HOME`, no `HOME`).
    NoDataDir,
    /// A schema or version string differs from the one this build reads.
    SchemaMismatch { found: String, expected: String },
    /// `manifest.json` is missing or unreadable.
    MissingManifest,
    /// A line does not parse (not the last, recoverable one).
    BadLine {
        file: String,
        line: usize,
        why: String,
    },
    /// A chunk's checksum or tick count does not match its trailer.
    ChecksumMismatch { chunk: usize },
    /// A closed chunk (not the last) has no trailer.
    MissingTrailer { chunk: usize },
    /// Chunk files are not numbered 0..n without a hole.
    MissingChunk { index: usize },
    /// A tick is repeated or arrives out of order.
    NonContiguousTick { expected: u64, found: u64 },
    /// Ticks are missing (the session is `incomplete`) and the caller did not
    /// allow gaps.
    TickGap { from: u64, to: u64 },
    /// The manifest of a finished session disagrees with the chunks.
    ManifestMismatch(String),
    /// The writer was already finished.
    Finished,
    /// The session directory already exists.
    Exists(PathBuf),
}

impl fmt::Display for PlaytraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(why) => write!(f, "playtrace i/o error: {why}"),
            Self::NoDataDir => write!(f, "no data directory (set XDG_DATA_HOME or HOME)"),
            Self::SchemaMismatch { found, expected } => {
                write!(f, "schema mismatch: found {found:?}, expected {expected:?}")
            }
            Self::MissingManifest => write!(f, "manifest.json is missing or unreadable"),
            Self::BadLine { file, line, why } => write!(f, "{file} line {line}: {why}"),
            Self::ChecksumMismatch { chunk } => write!(f, "chunk {chunk}: checksum mismatch"),
            Self::MissingTrailer { chunk } => write!(f, "chunk {chunk}: missing trailer"),
            Self::MissingChunk { index } => write!(f, "chunk {index} is missing"),
            Self::NonContiguousTick { expected, found } => {
                write!(f, "tick {found} where {expected} was expected")
            }
            Self::TickGap { from, to } => write!(f, "ticks {from}..{to} are missing"),
            Self::ManifestMismatch(why) => write!(f, "manifest disagrees with chunks: {why}"),
            Self::Finished => write!(f, "session writer already finished"),
            Self::Exists(_) => write!(f, "session directory already exists"),
        }
    }
}

impl std::error::Error for PlaytraceError {}

impl From<std::io::Error> for PlaytraceError {
    fn from(error: std::io::Error) -> Self {
        // `ErrorKind` only: the OS message may carry a path.
        Self::Io(format!("{:?}", error.kind()))
    }
}
