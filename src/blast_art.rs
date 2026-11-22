//! DEATH-D: the frames of the QA charge and its blast, read from the exported
//! atlas JSON next to the atlas PNG (`generated/art/`).
//!
//! Bevy-free and presentation only. The charge is drawn as the ANIM-CARL-03
//! `dynamite_fizzle` loop (`vfx_dynamite`) and the blast as the VFX-01 `burst`
//! tag (`vfx-01`); both assets are `prototype_fixture_unapproved` and are
//! drawn in the `floor01_scene` QA fixtures only, which approves nothing. The
//! frame choice is the pure functions of [`crate::death_present`]
//! ([`charge_frame`](crate::death_present::charge_frame),
//! [`blast_vfx_frame`](crate::death_present::blast_vfx_frame)).

use std::path::Path;

/// Asset name of the dynamite atlas below `generated/art/`.
pub const DYNAMITE_IMAGE: &str = "vfx_dynamite.png";
/// Asset name of the VFX-01 atlas below `generated/art/`.
pub const BURST_IMAGE: &str = "vfx-01.png";

/// Asset name of the shared metal hit-spark atlas below `generated/art/`
/// (CP-Q42: the spark of a piercing punch).
pub const SPARK_IMAGE: &str = "vfx_hit_spark_metal.png";

const DYNAMITE_JSON: &str = "generated/art/vfx_dynamite.json";
const BURST_JSON: &str = "generated/art/vfx-01.json";
const SPARK_JSON: &str = "generated/art/vfx_hit_spark_metal.json";

/// Ticks each frame of the metal hit-spark is shown (the manifest's `ticks`:
/// star, burst, fade at 2 ticks each, 60 Hz).
pub const SPARK_FRAME_TICKS: [u64; 3] = [2, 2, 2];

/// The spark frame of a pierce that happened `age` ticks ago (0 on the tick of
/// the hit): `None` once the last frame is over. Pure, presentation only.
pub fn spark_frame(age: u64) -> Option<usize> {
    let mut left = age;
    for (frame, ticks) in SPARK_FRAME_TICKS.into_iter().enumerate() {
        if left < ticks {
            return Some(frame);
        }
        left -= ticks;
    }
    None
}

/// Pixel rects (x, y, w, h) of every frame of the two atlases, in frame order.
#[derive(Debug, Clone, PartialEq)]
pub struct BlastArt {
    pub dynamite: Vec<[i32; 4]>,
    pub burst: Vec<[i32; 4]>,
    /// The `vfx_hit_spark_metal` frames (16 x 16, pivot 8, 8).
    pub spark: Vec<[i32; 4]>,
}

fn read_rects(root: &Path, path: &str) -> Result<Vec<[i32; 4]>, String> {
    let text = std::fs::read_to_string(root.join(path)).map_err(|e| format!("{path}: {e}"))?;
    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
    let frames = json["frames"]
        .as_array()
        .ok_or_else(|| format!("{path}: no frames array"))?;
    frames
        .iter()
        .map(|f| {
            let rect = &f["frame"];
            let get = |key: &str| {
                rect[key]
                    .as_i64()
                    .map(|v| v as i32)
                    .ok_or_else(|| format!("{path}: frame without {key}"))
            };
            Ok([get("x")?, get("y")?, get("w")?, get("h")?])
        })
        .collect()
}

impl BlastArt {
    /// Read both atlas JSON files below `root` (the repository root).
    pub fn load(root: &Path) -> Result<Self, String> {
        let dynamite = read_rects(root, DYNAMITE_JSON)?;
        let burst = read_rects(root, BURST_JSON)?;
        // The fizzle uses frames 4 and 5, the burst frames 3 to 11.
        if dynamite.len() < 6 || burst.len() < 12 {
            return Err(format!(
                "blast art: {} dynamite and {} burst frames (expected at least 6 and 12)",
                dynamite.len(),
                burst.len()
            ));
        }
        let spark = read_rects(root, SPARK_JSON)?;
        if spark.len() < SPARK_FRAME_TICKS.len() {
            return Err(format!(
                "blast art: {} spark frames (expected at least {})",
                spark.len(),
                SPARK_FRAME_TICKS.len()
            ));
        }
        Ok(Self {
            dynamite,
            burst,
            spark,
        })
    }

    /// The rect of dynamite frame `frame`.
    pub fn dynamite_rect(&self, frame: usize) -> [i32; 4] {
        self.dynamite[frame.min(self.dynamite.len() - 1)]
    }

    /// The rect of the metal hit-spark frame `frame`.
    pub fn spark_rect(&self, frame: usize) -> [i32; 4] {
        self.spark[frame.min(self.spark.len() - 1)]
    }

    /// The rect of VFX-01 frame `frame`.
    pub fn burst_rect(&self, frame: usize) -> [i32; 4] {
        self.burst[frame.min(self.burst.len() - 1)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn the_exported_atlases_hold_the_frames_the_blast_uses_when_present() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        if !root.join(DYNAMITE_JSON).exists() || !root.join(BURST_JSON).exists() {
            eprintln!("skipped: the atlases are exported by scripts/export_assets.sh");
            return;
        }
        let art = BlastArt::load(&root).unwrap();
        assert_eq!(art.dynamite.len(), 6);
        assert_eq!(art.burst.len(), 12);
        assert!(art.dynamite.iter().all(|r| r[2] == 24 && r[3] == 24));
        assert!(art.burst.iter().all(|r| r[2] == 48 && r[3] == 48));
        assert_eq!(art.spark.len(), 3);
        assert!(art.spark.iter().all(|r| r[2] == 16 && r[3] == 16));
    }

    #[test]
    fn the_spark_runs_three_frames_of_two_ticks_and_is_over_on_the_seventh() {
        let frames: Vec<Option<usize>> = (0..8).map(spark_frame).collect();
        assert_eq!(
            frames,
            [
                Some(0),
                Some(0),
                Some(1),
                Some(1),
                Some(2),
                Some(2),
                None,
                None
            ]
        );
    }

    #[test]
    fn a_missing_atlas_is_an_error_not_a_panic() {
        let err = BlastArt::load(Path::new("/nonexistent-root")).unwrap_err();
        assert!(err.contains("vfx_dynamite.json"), "{err}");
    }
}
