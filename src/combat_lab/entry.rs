//! The leak guard (LAB-Q01): which entry points may load a lab profile.
//!
//! The profile is selectable **only** through the lab entry point. `--demo`,
//! the story route, plain `--cf02` and the boss scene reject any lab profile
//! argument. The guard is a mechanism, not only intent: [`LabCatalog::launch`]
//! needs an [`EntryPoint`], and every entry but [`EntryPoint::CombatLab`] is
//! refused, so no route can produce a lab run, and therefore no post-biscuit
//! lab state, by accident. The source-scan test of this module additionally
//! keeps the profile id and the module out of the story and demo code.
//!
//! [`LabCatalog::launch`]: super::LabCatalog::launch

use super::LabError;

/// The run entries of `floor01_scene` (and the headless fixtures).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryPoint {
    /// `--scene combat_lab` (LAB-00-wire). The only entry that may load a lab
    /// profile.
    CombatLab,
    /// The story route (no scene flag).
    Story,
    /// `--demo` (DEMO-01).
    Demo,
    /// Plain `--cf02`.
    Cf02,
    /// `--scene boss_f01`.
    Boss,
    /// A named scripted fixture.
    Fixture,
}

impl EntryPoint {
    pub const ALL: [EntryPoint; 6] = [
        Self::CombatLab,
        Self::Story,
        Self::Demo,
        Self::Cf02,
        Self::Boss,
        Self::Fixture,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::CombatLab => "combat_lab",
            Self::Story => "story",
            Self::Demo => "demo",
            Self::Cf02 => "cf02",
            Self::Boss => "boss_f01",
            Self::Fixture => "fixture",
        }
    }

    pub const fn may_load_lab_profile(self) -> bool {
        matches!(self, Self::CombatLab)
    }
}

/// Whether `id` has the shape of a lab profile id (`floor01-postbiscuit-lab-v1`).
pub fn is_lab_profile_id(id: &str) -> bool {
    id.contains("-lab-v")
}

/// For the argument parsers of the other entries: reject a lab profile
/// argument with a descriptive error.
pub fn reject_lab_profile_arg(entry: EntryPoint, arg: &str) -> Result<(), LabError> {
    if is_lab_profile_id(arg) && !entry.may_load_lab_profile() {
        return Err(denied(entry, arg));
    }
    Ok(())
}

pub(super) fn denied(entry: EntryPoint, profile: &str) -> LabError {
    LabError::EntryDenied {
        entry: entry.name(),
        reason: format!(
            "profile {profile:?} is selectable only through the combat_lab entry (LAB-Q01)"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn only_the_lab_entry_may_load_a_profile() {
        for entry in EntryPoint::ALL {
            let r = reject_lab_profile_arg(entry, super::super::PROFILE_ID);
            assert_eq!(r.is_ok(), entry == EntryPoint::CombatLab, "{entry:?}");
        }
        // Ordinary arguments are never touched.
        assert!(reject_lab_profile_arg(EntryPoint::Demo, "--demo-beat").is_ok());
        assert!(reject_lab_profile_arg(EntryPoint::Story, "boss_f01").is_ok());
    }

    fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                rust_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// The story, demo and CF02 code never mention the lab or its profile id:
    /// the only files that may are the lab itself, its fixtures, the module
    /// registration and the (future) live entry and overlay.
    #[test]
    fn the_story_and_demo_code_never_mention_the_lab_profile() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rust_files(&root, &mut files);
        assert!(files.len() > 50);
        let allowed = [
            "combat_lab",
            "combat_lab_fixtures.rs",
            "combat_lab_overlay.rs",
            "lib.rs",
            "floor01_scene.rs",
        ];
        // PLAYTRACE-01 PT-C: the session verifier rebuilds a recorded lab run
        // through the lab's own launch (the entry-point guard still applies).
        let allowed_paths = [Path::new("playtrace").join("replay.rs")];
        for file in files {
            let rel = file.strip_prefix(&root).unwrap();
            let first = rel
                .components()
                .next()
                .unwrap()
                .as_os_str()
                .to_string_lossy();
            let name = file.file_name().unwrap().to_string_lossy();
            if allowed.contains(&first.as_ref())
                || allowed.contains(&name.as_ref())
                || allowed_paths.iter().any(|p| p == rel)
            {
                continue;
            }
            let text = std::fs::read_to_string(&file).unwrap();
            assert!(
                !text.contains("-lab-v1") && !text.contains("combat_lab"),
                "{} mentions the combat lab outside the lab modules",
                rel.display()
            );
        }
    }
}
