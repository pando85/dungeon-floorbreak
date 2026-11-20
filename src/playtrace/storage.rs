use std::path::{Path, PathBuf};

use super::error::PlaytraceError;

/// A visible warning is shown when the playtrace directory exceeds this
/// (PT-Q15). Nothing is ever deleted automatically.
pub const WARN_BYTES: u64 = 5 * 1024 * 1024 * 1024;

/// `$XDG_DATA_HOME/dungeon-floorbreak/playtrace`, else
/// `$HOME/.local/share/dungeon-floorbreak/playtrace`. The environment values
/// are parameters so tests do not depend on the process environment.
pub fn default_record_dir(
    xdg_data_home: Option<&str>,
    home: Option<&str>,
) -> Result<PathBuf, PlaytraceError> {
    let base = match (
        xdg_data_home.filter(|v| !v.is_empty()),
        home.filter(|v| !v.is_empty()),
    ) {
        (Some(xdg), _) if Path::new(xdg).is_absolute() => PathBuf::from(xdg),
        (_, Some(home)) => Path::new(home).join(".local/share"),
        _ => return Err(PlaytraceError::NoDataDir),
    };
    Ok(base.join("dungeon-floorbreak").join("playtrace"))
}

/// Total size of every file below `dir` (0 when it does not exist).
pub fn dir_size_bytes(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.path().symlink_metadata() {
            Ok(meta) if meta.is_dir() => dir_size_bytes(&entry.path()),
            Ok(meta) => meta.len(),
            Err(_) => 0,
        })
        .sum()
}
