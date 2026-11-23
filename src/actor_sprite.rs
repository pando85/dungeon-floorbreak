//! Actor sprite placement maths shared with the Character Playground rules:
//! feet anchor to the manifest pivot, the pivot is mirrored when the sprite is
//! flipped, and every position is an integer so no sprite sits on a half pixel.
//!
//! The Floor 1 scene draws the locomotion atlases (ANIM-INT-01): Carl 48 x 56,
//! pivot (25, 55); Donut 72 x 44, pivot (46, 43). Frame size, pivot, tags and
//! durations are read from the manifests and the exported atlas JSON, not from
//! constants (see [`load_actor`]).
//!
//! Foot rule (owner decision, deep floor): the lowest opaque row of a grounded
//! actor sits on the walking line, `FOOT_SINK_PX = 0`. Airborne frames use the
//! same rule: the pivot is the physics foot position, with no ground snapping,
//! so a frame whose feet are drawn above the pivot row (rise, fall, the gallop
//! suspension) floats exactly as far as the art says.
//!
//! Known 1 px asymmetry (accepted): the flipped sprite's pivot column lands one
//! pixel to the left of the unflipped one. The Playground has the same
//! behaviour; half-pixel centres are deliberately not used to hide it. It holds
//! for the wider locomotion frames too (both widths are even).

use std::path::Path;

use crate::anim::{CARL_RUN_PACING, ClipSet, DONUT_RUN_PACING, RunPacing};
pub use crate::env_kit::{ATLAS_ASSET_PREFIX, FOOT_SINK_PX};

/// Frame geometry of one actor atlas. Size and pivot come from the manifest
/// (see [`ActorSpec::from_clips`]), never from constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorSpec {
    pub name: &'static str,
    pub width: i32,
    pub height: i32,
    /// Pivot pixel: x column and the lowest opaque row of grounded frames
    /// (y, from the top).
    pub pivot_x: i32,
    pub pivot_y: i32,
    /// Lighting footprint: the contact shadow keeps the geometry of the
    /// pre-ANIM-INT-01 idle canvas (width, pivot column), so adopting the wider
    /// locomotion atlases does not change the lighting. Not read from data.
    pub shadow_width: i32,
    pub shadow_pivot_x: i32,
}

/// Idle canvas of the approved Carl atlas `carl_book1_h52`: 36 px wide, pivot
/// column 19. Pinned against its manifest by a test.
pub const CARL_SHADOW_FOOTPRINT: (i32, i32) = (36, 19);
/// Idle canvas of the approved Donut atlas `donut_scale_h40`: 46 px wide, pivot
/// column 22. Pinned against its manifest by a test.
pub const DONUT_SHADOW_FOOTPRINT: (i32, i32) = (46, 22);

impl ActorSpec {
    pub fn from_clips(name: &'static str, clips: &ClipSet, footprint: (i32, i32)) -> Self {
        Self {
            name,
            width: clips.frame_size.0,
            height: clips.frame_size.1,
            pivot_x: clips.pivot.0,
            pivot_y: clips.pivot.1,
            shadow_width: footprint.0,
            shadow_pivot_x: footprint.1,
        }
    }
}

/// Where the frame durations and tags are read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameSource {
    /// The exported atlas JSON next to the atlas PNG (`generated/art/`): what
    /// the running scene draws. Needs `scripts/export_assets.sh`.
    ExportedAtlas,
    /// The committed pixel-grid source named by the manifest (`grid_source`):
    /// the same data without an export. For tests and tools.
    GridSource,
}

/// One actor ready to draw: spec, clips, run pacing and its atlas asset name.
#[derive(Debug, Clone)]
pub struct ActorAssets {
    pub spec: ActorSpec,
    pub clips: ClipSet,
    pub pacing: RunPacing,
    /// Atlas image path relative to the Bevy asset root (`generated/art/`).
    pub atlas_asset: String,
    /// The name the HUD draws, from the manifest's `display_name` (the
    /// fan-concept layer; engine code stays IP-neutral).
    pub display_name: String,
}

fn read(root: &Path, relative: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(relative)).map_err(|e| format!("{relative}: {e}"))
}

/// Load an actor from its locomotion manifest (`assets/manifests/<file>`).
pub fn load_actor(
    root: &Path,
    name: &'static str,
    manifest_file: &str,
    footprint: (i32, i32),
    pacing: RunPacing,
    source: FrameSource,
) -> Result<ActorAssets, String> {
    let manifest_path = format!("assets/manifests/{manifest_file}");
    let manifest_text = read(root, &manifest_path)?;
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_text).map_err(|e| format!("{manifest_path}: {e}"))?;
    let field = |key: &str| -> Result<&str, String> {
        manifest[key]
            .as_str()
            .ok_or_else(|| format!("{manifest_path}: missing {key}"))
    };
    let clips = match source {
        FrameSource::ExportedAtlas => {
            ClipSet::from_atlas_json(&manifest_text, &read(root, field("output_metadata")?)?)
        }
        FrameSource::GridSource => {
            ClipSet::from_grid_json(&manifest_text, &read(root, field("grid_source")?)?)
        }
    }
    .map_err(|e| format!("{manifest_path}: {e}"))?;
    let atlas_asset = field("output_atlas")?
        .strip_prefix(ATLAS_ASSET_PREFIX)
        .ok_or_else(|| format!("{manifest_path}: atlas is not below {ATLAS_ASSET_PREFIX}"))?
        .to_owned();
    let display_name = field("display_name")?.to_owned();
    Ok(ActorAssets {
        display_name,
        spec: ActorSpec::from_clips(name, &clips, footprint),
        clips,
        pacing,
        atlas_asset,
    })
}

pub const CARL_MANIFEST: &str = "carl_book1_h52_locomotion.json";
pub const DONUT_MANIFEST: &str = "donut_h40_locomotion.json";

pub fn load_carl(root: &Path, source: FrameSource) -> Result<ActorAssets, String> {
    load_actor(
        root,
        "CARL",
        CARL_MANIFEST,
        CARL_SHADOW_FOOTPRINT,
        CARL_RUN_PACING,
        source,
    )
}

pub fn load_donut(root: &Path, source: FrameSource) -> Result<ActorAssets, String> {
    load_actor(
        root,
        "DONUT",
        DONUT_MANIFEST,
        DONUT_SHADOW_FOOTPRINT,
        DONUT_RUN_PACING,
        source,
    )
}

/// Sprite centre in y-up world pixels for a centre-anchored sprite.
///
/// `foot_x` is the pivot x; `ground_y` is the y-up world height of the bottom
/// edge of the actor's feet (the walking line while grounded, the physics foot
/// position while airborne).
pub fn actor_center(spec: ActorSpec, foot_x: f32, ground_y: f32, flip_x: bool) -> (f32, f32) {
    let offset_x = spec.width as f32 / 2.0 - spec.pivot_x as f32;
    let center_x = if flip_x {
        foot_x - offset_x
    } else {
        foot_x + offset_x
    };
    // The pivot row is the lowest opaque row: its bottom edge is the foot line.
    let frame_bottom = ground_y - FOOT_SINK_PX as f32 - (spec.height - 1 - spec.pivot_y) as f32;
    (
        center_x.round(),
        (frame_bottom + spec.height as f32 / 2.0).round(),
    )
}

/// Centre x of the lighting footprint (the pre-locomotion idle canvas) for the
/// contact shadow. Same maths as [`actor_center`]'s x, on the footprint box.
pub fn shadow_center_x(spec: ActorSpec, foot_x: f32, flip_x: bool) -> f32 {
    let offset_x = spec.shadow_width as f32 / 2.0 - spec.shadow_pivot_x as f32;
    let center_x = if flip_x {
        foot_x - offset_x
    } else {
        foot_x + offset_x
    };
    center_x.round()
}

/// FA-Q38 (i-frame cue): the ticks one phase of the flicker lasts.
pub const INVULNERABILITY_FLICKER_TICKS: u8 = 4;
/// FA-Q38: the alpha of the dim phase of the flicker.
pub const INVULNERABILITY_DIM_ALPHA: f32 = 0.5;

/// FA-Q38 (i-frame cue): the alpha a hero's sprite is drawn with, from the
/// ticks of invulnerability the simulation reports. Both heroes share it: it
/// alternates 0.5 and 1.0 every 4 ticks while invulnerable (dim while
/// `invulnerable_ticks / 4` is even), is always 1.0 outside the invulnerability,
/// and is off while the hero is `downed` (the defeat freeze and the downed
/// hold). Presentation only; it is never read back by the simulation.
pub fn invulnerability_alpha(invulnerable_ticks: u8, downed: bool) -> f32 {
    if downed
        || invulnerable_ticks == 0
        || !(invulnerable_ticks / INVULNERABILITY_FLICKER_TICKS).is_multiple_of(2)
    {
        1.0
    } else {
        INVULNERABILITY_DIM_ALPHA
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn specs() -> [ActorSpec; 2] {
        [
            load_carl(root(), FrameSource::GridSource).unwrap().spec,
            load_donut(root(), FrameSource::GridSource).unwrap().spec,
        ]
    }

    /// The Character Playground's `actor_transform` x rule, copied verbatim.
    fn playground_center_x(spec: ActorSpec, x: f32, flip_x: bool) -> f32 {
        let offset_x = spec.width as f32 / 2.0 - spec.pivot_x as f32;
        let center_x = if flip_x { x - offset_x } else { x + offset_x };
        center_x.round()
    }

    #[test]
    fn x_matches_the_playground_for_both_facings() {
        for spec in specs() {
            for flip in [false, true] {
                for x in [72.0, 300.4, 1784.0] {
                    assert_eq!(
                        actor_center(spec, x, 92.0, flip).0,
                        playground_center_x(spec, x, flip),
                        "{} flip={flip} x={x}",
                        spec.name
                    );
                }
            }
        }
    }

    #[test]
    fn pivot_foot_stays_put_up_to_the_documented_one_pixel_flip_offset() {
        // Holds for the wider locomotion frames too (48 and 72 px, both even).
        for spec in specs() {
            let foot = |flip: bool| {
                let cx = actor_center(spec, 500.0, 92.0, flip).0;
                // Left edge of the pivot pixel column in world x.
                let left = cx - spec.width as f32 / 2.0;
                if flip {
                    left + (spec.width - 1 - spec.pivot_x) as f32
                } else {
                    left + spec.pivot_x as f32
                }
            };
            assert_eq!(foot(false), 500.0, "{}", spec.name);
            assert_eq!(foot(true), 499.0, "{}", spec.name);
        }
    }

    #[test]
    fn lowest_opaque_row_sits_on_the_walking_line_with_zero_sink() {
        assert_eq!(FOOT_SINK_PX, 0);
        for spec in specs() {
            for ground in [92.0, 76.0, 60.0] {
                let (_, cy) = actor_center(spec, 100.0, ground, false);
                let frame_top = cy + spec.height as f32 / 2.0;
                let pivot_row_bottom = frame_top - (spec.pivot_y + 1) as f32;
                assert_eq!(pivot_row_bottom, ground, "{} on {ground}", spec.name);
                assert_eq!(cy.fract(), 0.0);
            }
        }
    }

    #[test]
    fn airborne_placement_follows_the_physics_foot_without_snapping() {
        // The same maths for any foot height: 20 px higher is 20 px higher.
        for spec in specs() {
            let low = actor_center(spec, 100.0, 92.0, false).1;
            let high = actor_center(spec, 100.0, 112.0, false).1;
            assert_eq!(high - low, 20.0, "{}", spec.name);
        }
    }

    #[test]
    fn locomotion_atlas_geometry_comes_from_the_manifests() {
        let [carl, donut] = specs();
        assert_eq!(
            (carl.width, carl.height, carl.pivot_x, carl.pivot_y),
            (48, 56, 25, 55)
        );
        assert_eq!(
            (donut.width, donut.height, donut.pivot_x, donut.pivot_y),
            (72, 44, 46, 43)
        );
        // Even canvases keep centre-anchored sprites on whole pixels.
        assert!(
            [carl, donut]
                .iter()
                .all(|s| s.width % 2 == 0 && s.height % 2 == 0)
        );
        let assets = load_carl(root(), FrameSource::GridSource).unwrap();
        assert_eq!(assets.atlas_asset, "carl_book1_h52_locomotion.png");
        let assets = load_donut(root(), FrameSource::GridSource).unwrap();
        assert_eq!(assets.atlas_asset, "donut_h40_locomotion.png");
    }

    #[test]
    fn shadow_footprint_is_the_old_idle_canvas_of_the_approved_atlases() {
        for (file, footprint) in [
            ("carl_book1_h52.json", CARL_SHADOW_FOOTPRINT),
            ("donut_scale_h40.json", DONUT_SHADOW_FOOTPRINT),
        ] {
            let text = read(root(), &format!("assets/manifests/{file}")).unwrap();
            let manifest: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(manifest["frame_size"][0], footprint.0, "{file}");
            assert_eq!(manifest["pivot"][0], footprint.1, "{file}");
        }
    }

    #[test]
    fn contact_shadow_x_equals_the_old_idle_sprite_centre() {
        // The idle frames are pixel-identical padded copies, so the old
        // centre-anchored idle canvas centre is the shadow anchor in every
        // frame, for both facings.
        for spec in specs() {
            for flip in [false, true] {
                let old_offset = spec.shadow_width as f32 / 2.0 - spec.shadow_pivot_x as f32;
                let expected = if flip {
                    300.0 - old_offset
                } else {
                    300.0 + old_offset
                };
                assert_eq!(shadow_center_x(spec, 300.0, flip), expected.round());
            }
        }
    }

    #[test]
    fn carl_idle_centre_is_unchanged_by_the_symmetric_padding() {
        // 6 px of padding each side: the sprite centre stays where it was, so
        // the pixel-identical idle frame lands on the same screen pixels.
        let [carl, _] = specs();
        let old_offset = CARL_SHADOW_FOOTPRINT.0 as f32 / 2.0 - CARL_SHADOW_FOOTPRINT.1 as f32;
        assert_eq!(actor_center(carl, 300.0, 92.0, false).0, 300.0 + old_offset);
    }

    #[test]
    fn the_flicker_alternates_every_four_ticks_while_invulnerable() {
        // 55 ticks left (the hit) counting down to 1: dim while ticks / 4 is even.
        let alphas: Vec<f32> = (1..=8).map(|t| invulnerability_alpha(t, false)).collect();
        assert_eq!(alphas, [0.5, 0.5, 0.5, 1.0, 1.0, 1.0, 1.0, 0.5]);
        // Both edges of a phase: 3 is dim, 4 is bright, 7 is bright, 8 is dim.
        assert_eq!(invulnerability_alpha(3, false), 0.5);
        assert_eq!(invulnerability_alpha(4, false), 1.0);
        assert_eq!(invulnerability_alpha(7, false), 1.0);
        assert_eq!(invulnerability_alpha(8, false), 0.5);
        // Only the two alphas ever occur, over the whole 55 tick window.
        for t in 1..=55 {
            let a = invulnerability_alpha(t, false);
            assert!(a == 0.5 || a == 1.0, "tick {t}");
        }
    }

    #[test]
    fn there_is_no_cue_outside_the_invulnerability_or_while_downed() {
        assert_eq!(invulnerability_alpha(0, false), 1.0);
        for t in 0..=255u8 {
            assert_eq!(invulnerability_alpha(t, true), 1.0, "downed, {t} left");
        }
    }
}
