//! The boss' art binding (HUD-DEBT-03). Bevy-free.
//!
//! Every sprite lookup of the boss goes through one resource, [`BossArt`],
//! keyed by the **animation tag** and the **frame index inside the tag**
//! (`("charge_run", 1)`), never by an atlas position. The scene asks
//! [`pose_of`] which `(tag, frame)` the simulation state shows, then
//! [`BossArt::atlas_index`] for the atlas frame; with no boss atlas the lookup
//! answers `None` and the scene draws placeholder rects at the spec sizes
//! ([`PLACEHOLDER_FRAME`]). The art integration step only has to drop the
//! manifest and the exported atlas in place.
//!
//! The manifest is `assets/manifests/brute_boss_f01.json` (`frame_size`,
//! `pivot`, `output_atlas`, `output_metadata`); the exported atlas JSON holds
//! the tag ranges (`meta.frameTags`, name / from / to) like every actor atlas.
//! The expected tags are [`EXPECTED_TAGS`].

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::boss::{Boss, BossMode};

/// The animation tags the binding expects (JUICE-01's seven, then the
/// encounter's six keys of the design, section 8).
pub const EXPECTED_TAGS: [&str; 13] = [
    "idle",
    "walk",
    "attack_tell",
    "attack_pound",
    "attack_recover",
    "hurt",
    "down",
    "charge_tell",
    "charge_run",
    "charge_run_b",
    "crash_stun",
    "phase_roar",
    "pinned",
];

/// Frame size and pivot of the placeholder (and of JUICE-01): 160 x 136 with
/// the feet pivot at (72, 135).
pub const PLACEHOLDER_FRAME: (i32, i32) = (160, 136);
pub const PLACEHOLDER_PIVOT: (i32, i32) = (72, 135);
/// The manifest of the boss atlas.
pub const MANIFEST: &str = "assets/manifests/brute_boss_f01.json";
/// The HUD emblem key (18 x 18).
pub const EMBLEM_KEY: &str = "brute_boss_f01_emblem";

/// Which `(tag, frame inside the tag)` the boss shows.
pub fn pose_of(boss: &Boss) -> (&'static str, usize) {
    let tick = usize::from(boss.mode_tick);
    match boss.mode {
        BossMode::Dormant | BossMode::Neutral => ("idle", 0),
        BossMode::Turn => ("walk", 0),
        BossMode::PoundTell | BossMode::SecondTell => ("attack_tell", tick / 8),
        BossMode::PoundStrike | BossMode::SecondStrike => ("attack_pound", tick / 3),
        BossMode::PoundRecover | BossMode::SecondRecover | BossMode::DoubleStep => {
            ("attack_recover", tick / 8)
        }
        BossMode::ChargeTell => ("charge_tell", 0),
        BossMode::Charge => {
            if (tick / 4).is_multiple_of(2) {
                ("charge_run", 0)
            } else {
                ("charge_run_b", 0)
            }
        }
        BossMode::Rebound | BossMode::Stun => ("crash_stun", 0),
        BossMode::Roar => ("phase_roar", 0),
        BossMode::Flinch | BossMode::Backstep => ("hurt", 0),
        BossMode::Pinned => ("pinned", 0),
        BossMode::Defeated => ("down", tick / 8),
    }
}

#[derive(Deserialize)]
struct ManifestData {
    frame_size: [i32; 2],
    pivot: [i32; 2],
    output_atlas: String,
}

#[derive(Deserialize)]
struct AtlasFile {
    meta: AtlasMeta,
}

#[derive(Deserialize)]
struct AtlasMeta {
    #[serde(rename = "frameTags")]
    frame_tags: Vec<Tag>,
}

#[derive(Deserialize)]
struct Tag {
    name: String,
    from: usize,
    to: usize,
}

/// The boss atlas as the scene draws it.
#[derive(Debug, Clone)]
pub struct BossArt {
    pub frame_size: (i32, i32),
    pub pivot: (i32, i32),
    /// Atlas image path relative to the Bevy asset root.
    pub atlas_asset: String,
    pub frame_count: usize,
    tags: BTreeMap<String, (usize, usize)>,
}

impl BossArt {
    /// Build from the manifest text and the exported atlas JSON. A tag in
    /// [`EXPECTED_TAGS`] that the atlas lacks is an error naming it.
    pub fn parse(manifest: &str, atlas: &str) -> Result<Self, String> {
        let manifest: ManifestData =
            serde_json::from_str(manifest).map_err(|e| format!("boss manifest: {e}"))?;
        let atlas: AtlasFile =
            serde_json::from_str(atlas).map_err(|e| format!("boss atlas json: {e}"))?;
        let tags: BTreeMap<String, (usize, usize)> = atlas
            .meta
            .frame_tags
            .into_iter()
            .map(|t| (t.name, (t.from, t.to)))
            .collect();
        if let Some(missing) = EXPECTED_TAGS.iter().find(|t| !tags.contains_key(**t)) {
            return Err(format!("boss atlas has no tag {missing:?}"));
        }
        let frame_count = tags.values().map(|(_, to)| to + 1).max().unwrap_or(0);
        let atlas_asset = manifest
            .output_atlas
            .strip_prefix(crate::env_kit::ATLAS_ASSET_PREFIX)
            .ok_or("boss manifest: atlas is not below generated/art/")?
            .to_owned();
        Ok(Self {
            frame_size: (manifest.frame_size[0], manifest.frame_size[1]),
            pivot: (manifest.pivot[0], manifest.pivot[1]),
            atlas_asset,
            frame_count,
            tags,
        })
    }

    /// Load the boss art when its manifest and exported atlas exist. `Ok(None)`
    /// means no art yet (the scene draws placeholders); an error means the art
    /// is present but wrong.
    pub fn load(root: &Path) -> Result<Option<Self>, String> {
        let manifest_path = root.join(MANIFEST);
        let Ok(manifest) = std::fs::read_to_string(&manifest_path) else {
            return Ok(None);
        };
        let meta: serde_json::Value =
            serde_json::from_str(&manifest).map_err(|e| format!("boss manifest: {e}"))?;
        let Some(meta_path) = meta["output_metadata"].as_str() else {
            return Err("boss manifest: no output_metadata".into());
        };
        let Ok(atlas) = std::fs::read_to_string(root.join(meta_path)) else {
            // The manifest is committed before the export has run.
            return Ok(None);
        };
        Self::parse(&manifest, &atlas).map(Some)
    }

    /// The atlas frame of `(tag, frame inside the tag)`, the frame index
    /// clamped to the tag (a tag shorter than the asked frame holds its last).
    pub fn atlas_index(&self, tag: &str, frame: usize) -> Option<usize> {
        let (from, to) = *self.tags.get(tag)?;
        Some((from + frame).min(to))
    }

    /// The atlas frame the boss shows now.
    pub fn frame_of(&self, boss: &Boss) -> Option<usize> {
        let (tag, frame) = pose_of(boss);
        self.atlas_index(tag, frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss::builtin;

    fn fake() -> BossArt {
        let tags: Vec<String> = EXPECTED_TAGS
            .iter()
            .enumerate()
            .map(|(i, t)| format!(r#"{{"name":"{t}","from":{},"to":{}}}"#, i * 2, i * 2 + 1))
            .collect();
        let atlas = format!(r#"{{"meta":{{"frameTags":[{}]}}}}"#, tags.join(","));
        let manifest = r#"{"frame_size":[160,136],"pivot":[72,135],
            "output_atlas":"generated/art/brute_boss_f01.png",
            "output_metadata":"generated/art/brute_boss_f01.json"}"#;
        BossArt::parse(manifest, &atlas).unwrap()
    }

    #[test]
    fn the_binding_resolves_every_mode_to_an_expected_tag() {
        let art = fake();
        let mut boss = Boss::new(builtin(), 0.0);
        for mode in [
            BossMode::Dormant,
            BossMode::Neutral,
            BossMode::Turn,
            BossMode::PoundTell,
            BossMode::PoundStrike,
            BossMode::PoundRecover,
            BossMode::DoubleStep,
            BossMode::SecondTell,
            BossMode::SecondStrike,
            BossMode::SecondRecover,
            BossMode::ChargeTell,
            BossMode::Charge,
            BossMode::Rebound,
            BossMode::Stun,
            BossMode::Roar,
            BossMode::Flinch,
            BossMode::Backstep,
            BossMode::Pinned,
            BossMode::Defeated,
        ] {
            boss.mode = mode;
            let (tag, _) = pose_of(&boss);
            assert!(EXPECTED_TAGS.contains(&tag), "{mode:?} -> {tag}");
            assert!(art.frame_of(&boss).is_some(), "{mode:?}");
        }
    }

    #[test]
    fn a_frame_past_the_end_of_a_tag_holds_the_last() {
        let art = fake();
        assert_eq!(art.atlas_index("idle", 0), Some(0));
        assert_eq!(art.atlas_index("idle", 9), Some(1));
        assert_eq!(art.atlas_index("nope", 0), None);
    }

    #[test]
    fn a_missing_tag_is_an_error_and_missing_art_is_not() {
        let manifest = r#"{"frame_size":[160,136],"pivot":[72,135],
            "output_atlas":"generated/art/b.png","output_metadata":"generated/art/b.json"}"#;
        let atlas = r#"{"meta":{"frameTags":[{"name":"idle","from":0,"to":1}]}}"#;
        assert!(BossArt::parse(manifest, atlas).is_err());
        let dir = std::env::temp_dir().join("floorbreak_no_boss_art");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(BossArt::load(&dir).unwrap().is_none());
    }
}
