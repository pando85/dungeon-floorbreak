//! Combat animation of the Floor 1 scene (ART-INT-02): which frame of the
//! approved Carl melee atlas and of the two goblin atlases is drawn for a given
//! combat state. Pure Rust, no Bevy types, presentation only: everything here
//! reads `&SceneSim` / `&CombatState` and nothing flows back, so the simulation
//! hash cannot depend on it (a test pins that for every fixture).
//!
//! # Pure frame selection
//!
//! [`carl_pose`] and [`goblin_pose`] are pure functions of a small input struct
//! that [`CombatAnimators`] derives from the finished state of each fixed tick.
//! Almost everything is already a clock of the simulation (the move clock, the
//! goblin's attack tick, `down_ticks`, `hurt_ticks`, the hit-stop counters), so a
//! frozen sim clock holds the frame for free: **during hit-stop the frame is
//! held**. Only two things need a memory, both derived from the same finished
//! states and so deterministic and replayable:
//!
//! * the walk cycle (a goblin walks when its x changed this tick while it is
//!   neutral and not sliding; the cycle advances one frame per
//!   [`WALK_STRIDE_MILLI_PX`] of ground, which paces the 6-frame cycle to the
//!   62 px/s chase speed with the planted foot 5 px back per frame), and
//! * the age of Carl's sprawl after a knockdown (ticks since he touched down).
//!
//! # Data, not constants
//!
//! Frame size, pivot, tags, per-frame tick durations, hold frames and airborne
//! tags come from the manifests (`melee.moves`, `goblin.frames`), the exported
//! atlas JSON (tag ranges) and the committed pixel-grid sources (the opaque
//! extent of every frame). The only art-side numbers kept here are the walk
//! stride (5 px of ground per frame, GOB-01 spec section 5) and the idle
//! footprint of the contact shadow. [`SpriteClips::sim_mismatches`] compares the
//! art timings with the sim windows (`feel.rs`, `combat.rs`) so a disagreement is
//! reported by a test, never silently fixed.
//!
//! # Placement
//!
//! Grounded frames are drawn with their pivot at the physics foot (0 px sink),
//! like the locomotion atlases. **Airborne tags** (the knockback frames) are
//! drawn with the lowest opaque row of the *frame* on the bottom of the sim body
//! ([`SpriteClips::anchor_foot_y`]): the pivot row is 2 to 17 px below those
//! frames' feet, so the plain pivot rule would hover the goblin by up to 9 px
//! over its own body box at launch and touchdown and pop on landing. This is not
//! a ground snap: the bottom follows the body up and down in the air. Carl's
//! knockdown blends from the pivot rule at the launch to this rule by the apex
//! (his grounded hurt frame before it is pivot-anchored, so there is no drop).
//!
//! # Hit-stop contact
//!
//! The goblin's recoil frame throws its head back, so on the hit tick the
//! striking limb of a1, b1 and k3 would hang in the air. While the goblin is
//! frozen by Carl's blow it is drawn up to [`MAX_STRIKE_SHIFT_PX`] pixels toward
//! him so the limb overlaps it by two pixels ([`StrikeGeometry`], from opaque
//! masks built at load), and the shift eases back over [`STRIKE_EASE_TICKS`]
//! moving ticks while the knockback carries it away. Stacked goblins are drawn
//! apart (see [`STACK_SPACING_PX`]). None of this touches the sim.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;

use crate::actor_sprite::{ActorSpec, CARL_SHADOW_FOOTPRINT, DONUT_SHADOW_FOOTPRINT, actor_center};
use crate::combat::{ENEMY_HALF_HEIGHT, MoveId};
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::donut_art;
use crate::feel::LLAMA_TELL_TICKS;
use crate::feel::{
    ATTACK_TICKS, DOWNED_TICKS, HERO_HURT_TICKS, HERO_KNOCKDOWN_VY, IMPACT_END, KNOCK_GRAVITY,
    LETHAL_KNOCK_VY, SMEAR_END, TELL_TICKS,
};
use crate::goblin::{AttackPhase, GoblinMode};
use crate::llama::{LLAMA_HALF_HEIGHT, LlamaMode};
use crate::prototype::HERO_HALF_HEIGHT;
use crate::prototype::TICK_SECONDS;

pub const CARL_MELEE_MANIFEST: &str = "carl_book1_h52_melee.json";
pub const GOBLIN_MANIFESTS: [&str; 2] = ["goblin_a_h36.json", "goblin_b_h36.json"];
/// CP-B2: the ANIM-CARL-03 dynamite throw sheet.
pub const CARL_DYNAMITE_MANIFEST: &str = "carl_book1_h52_dynamite.json";
/// CF02-I: the ART-CARL-CF02, ART-DONUT-CF02 and ART-GOB-CF02 sheets.
pub const CARL_CF02_MANIFEST: &str = "carl_book1_h52_cf02.json";
pub const DONUT_CF02_MANIFEST: &str = "donut_h40_cf02.json";
/// GUILD-VERB-01: the ANIM-INTERACT-01 sheets (Carl's hand, floor and push verbs, Donut's paw and
/// nudge), read by tag and frame index through [`crate::interact::Verb`].
pub const CARL_INTERACT_MANIFEST: &str = "carl_book1_h52_interact.json";
pub const DONUT_INTERACT_MANIFEST: &str = "donut_h40_interact.json";
pub const GOBLIN_CF02_MANIFESTS: [&str; 2] = ["goblin_a_h36_cf02.json", "goblin_b_h36_cf02.json"];
pub const GOBLIN_SLAMMED_MANIFESTS: [&str; 2] =
    ["goblin_a_slammed_h36.json", "goblin_b_slammed_h36.json"];
/// The Bad Llama atlas (LLAMA-01, 96 x 72 frames, pivot (44, 71)).
pub const LLAMA_MANIFEST: &str = "llama-01.json";
/// LLAMA-03: the Bad Llama rush atlas (10 frames, same 96 x 72 frame and pivot as
/// LLAMA-01; `rush_brace`, `rush_run` and `rush_recover`).
pub const LLAMA_RUSH_MANIFEST: &str = "llama_rush_h64.json";
/// ENEMY-GAMEPLAY-ENGINEER: the Goblin Engineer atlas (ENG-01, 64 x 56 frames,
/// pivot (28, 55), 22 frames: ENG-01 frames 0 to 17, then the ENG-02 get-up pair
/// (`getup`, 18 and 19) and the ram-run pair (`attack_ram_run`, 20 and 21)).
pub const ENGINEER_MANIFEST: &str = "eng_01.json";
/// ENG-02: the Engineer's slammed sheet (stomped, wall crumple, flung; the GOB A
/// CF02 cels with the belt, no weapon), the same tags and frame indices as
/// [`GOBLIN_SLAMMED_MANIFESTS`].
pub const ENGINEER_SLAMMED_MANIFEST: &str = "eng_slammed_h36.json";
/// Contact shadow of the llama: the 46 px standing box, pivot column 26 of it.
pub const LLAMA_SHADOW_FOOTPRINT: (i32, i32) = (46, 26);
/// Ground covered per `walk` frame: 4 px (the 40 px/s walk at 6 ticks a frame).
pub const LLAMA_WALK_STRIDE_MILLI_PX: u32 = 4000;

/// Ground covered per goblin walk frame in thousandths of a pixel: the planted
/// foot travels 5 px back per frame (GOB-01 spec section 5), so one 6-frame
/// cycle is 30 px of ground and, at the 62 px/s chase speed, about 29 ticks.
pub const WALK_STRIDE_MILLI_PX: u32 = 5000;
/// Contact shadow footprint of a standing goblin: width and pivot column of its
/// idle canvas (the opaque idle is about 36 px wide, the sole midpoint is the
/// pivot; the shadow is a little narrower than the body like Carl's).
pub const GOBLIN_SHADOW_FOOTPRINT: (i32, i32) = (28, 14);
/// Idle ticks of a goblin are offset by this per goblin index so a group does
/// not breathe in sync.
pub const IDLE_INDEX_OFFSET_TICKS: u64 = 13;
/// Walk phase offset per goblin index (15 px of ground, three frames), so a
/// group does not walk in lockstep.
pub const WALK_INDEX_OFFSET_MILLI_PX: u32 = 15_000;
/// Hit-stop contact: while a goblin is frozen by Carl's blow its recoil frame
/// throws the head back, so it is drawn this many pixels toward Carl (at most)
/// until the striking limb overlaps it by [`STRIKE_TARGET_OVERLAP_PX`] pixels.
pub const MAX_STRIKE_SHIFT_PX: i32 = 8;
pub const STRIKE_TARGET_OVERLAP_PX: usize = 2;
/// The most overlap a strike may show (deeper is drawn back).
pub const STRIKE_MAX_OVERLAP_PX: usize = 6;
/// The shift eases back to 0 over this many moving ticks after the freeze (the
/// knockback hides the return).
pub const STRIKE_EASE_TICKS: u8 = 3;
/// Two upright goblins closer than this are drawn apart (presentation only; the
/// real spacing is CP-02): the later one is nudged away from Carl by the
/// shortfall, at most [`STACK_MAX_NUDGE_PX`], and drawn behind the other.
pub const STACK_SPACING_PX: f32 = 8.0;
pub const STACK_MAX_NUDGE_PX: f32 = 6.0;
/// Atlas variants: goblin index parity picks A (even) or B (odd).
pub const GOBLIN_VARIANTS: usize = 2;

pub fn goblin_variant(index: usize) -> usize {
    index % GOBLIN_VARIANTS
}

// ---------------------------------------------------------------- data ------

#[derive(Deserialize)]
struct TagRange {
    name: String,
    from: usize,
    to: usize,
}

#[derive(Deserialize)]
struct AtlasFrame {
    duration: u32,
}

#[derive(Deserialize)]
struct AtlasMeta {
    #[serde(rename = "frameTags")]
    frame_tags: Vec<TagRange>,
}

#[derive(Deserialize)]
struct AtlasFile {
    frames: Vec<AtlasFrame>,
    meta: AtlasMeta,
}

#[derive(Deserialize)]
struct GridFrame {
    cels: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
struct GridFile {
    canvas: [i32; 2],
    frames: Vec<GridFrame>,
    tags: Vec<TagRange>,
}

/// Opaque extent of one frame in frame pixels (inclusive columns, lowest row).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameBox {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub lowest: i32,
}

/// Opaque pixels of one frame, one `u64` per row (bit `x` is column `x`; the
/// frames are at most 64 px wide): every layer, and each layer on its own.
/// Built once at load, so the contact test never scans an image.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FrameMasks {
    all: Vec<u64>,
    layers: BTreeMap<String, Vec<u64>>,
}

/// Where the tags and durations are read from (same choice as the locomotion
/// atlases: the exported atlas JSON the scene draws, or the grid source).
pub use crate::actor_sprite::FrameSource;

/// One atlas of the combat art with everything frame selection needs.
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteClips {
    pub spec: ActorSpec,
    /// Atlas image path relative to the Bevy asset root.
    pub atlas_asset: String,
    pub frame_count: usize,
    /// Tag name to `(first atlas frame, frame count)`.
    tags: BTreeMap<String, (usize, usize)>,
    /// Ticks each atlas frame is shown; `None` holds until the state changes.
    ticks: Vec<Option<u32>>,
    airborne_tags: BTreeSet<String>,
    boxes: Vec<FrameBox>,
    masks: Vec<FrameMasks>,
}

fn read(root: &Path, relative: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(relative)).map_err(|e| format!("{relative}: {e}"))
}

fn str_field<'a>(value: &'a serde_json::Value, key: &str, what: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("{what}: missing {key}"))
}

/// `[first, last]` inclusive tick range of a manifest frame, or null for a hold.
fn range_ticks(value: &serde_json::Value, what: &str) -> Result<Option<u32>, String> {
    if value.is_null() {
        return Ok(None);
    }
    let pair = value
        .as_array()
        .filter(|a| a.len() == 2)
        .ok_or_else(|| format!("{what}: ticks must be null or [first, last]"))?;
    let (first, last) = (pair[0].as_u64(), pair[1].as_u64());
    match (first, last) {
        (Some(first), Some(last)) if last >= first => Ok(Some((last - first + 1) as u32)),
        _ => Err(format!("{what}: bad tick range {value}")),
    }
}

fn analyse(
    frame: &GridFrame,
    width: i32,
    height: i32,
) -> Result<Option<(FrameBox, FrameMasks)>, String> {
    // The contact masks are one `u64` per row, so only frames up to 64 px wide
    // get them (Carl and the goblins); a wider frame (the 96 px llama) gets its
    // box and an empty mask, and has no strike-contact shift.
    let narrow = width <= 64;
    let mut masks = FrameMasks {
        all: vec![0; height as usize],
        layers: BTreeMap::new(),
    };
    let (mut left, mut right, mut top, mut lowest) = (i32::MAX, -1, i32::MAX, -1);
    for (layer, rows) in &frame.cels {
        if rows.len() as i32 != height || rows.iter().any(|r| r.chars().count() as i32 != width) {
            return Err("cel is not the canvas size".into());
        }
        let mut layer_rows = vec![0u64; height as usize];
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if ch != '.' {
                    if narrow {
                        layer_rows[y] |= 1 << x;
                    }
                    left = left.min(x as i32);
                    right = right.max(x as i32);
                    top = top.min(y as i32);
                    lowest = lowest.max(y as i32);
                }
            }
        }
        if narrow {
            for (all, row) in masks.all.iter_mut().zip(&layer_rows) {
                *all |= row;
            }
            masks.layers.insert(layer.clone(), layer_rows);
        }
    }
    if right < 0 {
        return Ok(None);
    }
    let b = FrameBox {
        left,
        right,
        top,
        lowest,
    };
    Ok(Some((b, masks)))
}

impl SpriteClips {
    /// Load one atlas from its manifest. `name` labels the spec; `kind` says how
    /// the manifest lists per-frame ticks.
    fn load(
        root: &Path,
        name: &'static str,
        manifest_file: &str,
        footprint: (i32, i32),
        source: FrameSource,
        kind: Kind,
    ) -> Result<Self, String> {
        let path = format!("assets/manifests/{manifest_file}");
        let text = read(root, &path)?;
        let manifest: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
        let err = |m: String| format!("{path}: {m}");
        let width = manifest["frame_size"][0].as_i64().unwrap_or(0) as i32;
        let height = manifest["frame_size"][1].as_i64().unwrap_or(0) as i32;
        let pivot = (
            manifest["pivot"][0].as_i64().unwrap_or(-1) as i32,
            manifest["pivot"][1].as_i64().unwrap_or(-1) as i32,
        );
        if width <= 0 || height <= 0 || pivot.0 < 0 || pivot.1 < 0 {
            return Err(err("bad frame_size or pivot".into()));
        }
        // The grid source always supplies the opaque extents (and the tags and
        // frame count when the source is the grid).
        let grid_path = str_field(&manifest, "grid_source", &path)?;
        let grid: GridFile = serde_json::from_str(&read(root, grid_path)?)
            .map_err(|e| format!("{grid_path}: {e}"))?;
        if grid.canvas != [width, height] {
            return Err(err("grid canvas differs from the manifest".into()));
        }
        let mut boxes = Vec::with_capacity(grid.frames.len());
        let mut masks = Vec::with_capacity(grid.frames.len());
        for (index, frame) in grid.frames.iter().enumerate() {
            let (b, m) = analyse(frame, width, height)
                .map_err(|e| format!("{grid_path}: frame {index}: {e}"))?
                .ok_or_else(|| format!("{grid_path}: frame {index} is empty"))?;
            boxes.push(b);
            masks.push(m);
        }
        let (tag_ranges, frame_count) = match source {
            FrameSource::GridSource => (grid.tags, grid.frames.len()),
            FrameSource::ExportedAtlas => {
                let atlas_path = str_field(&manifest, "output_metadata", &path)?;
                let atlas: AtlasFile = serde_json::from_str(&read(root, atlas_path)?)
                    .map_err(|e| format!("{atlas_path}: {e}"))?;
                if atlas.frames.iter().any(|f| f.duration == 0) {
                    return Err(format!("{atlas_path}: zero frame duration"));
                }
                let count = atlas.frames.len();
                (atlas.meta.frame_tags, count)
            }
        };
        if frame_count != boxes.len() {
            return Err(err(format!(
                "{frame_count} frames in the atlas, {} in the grid",
                boxes.len()
            )));
        }
        let mut tags = BTreeMap::new();
        for range in tag_ranges {
            if range.from > range.to || range.to >= frame_count {
                return Err(err(format!("tag {} is outside the atlas", range.name)));
            }
            if tags
                .insert(range.name.clone(), (range.from, range.to - range.from + 1))
                .is_some()
            {
                return Err(err(format!("tag {} appears twice", range.name)));
            }
        }
        let (ticks, airborne_list) = match kind {
            Kind::CarlMelee => carl_ticks(&manifest, &tags, frame_count).map_err(err)?,
            Kind::Goblin => goblin_ticks(&manifest, frame_count).map_err(err)?,
            Kind::Llama => llama_ticks(&manifest, frame_count).map_err(err)?,
            Kind::LlamaRush => llama_rush_ticks(&manifest, frame_count).map_err(err)?,
            Kind::Cf02Carl => cf02_carl_ticks(&manifest, frame_count).map_err(err)?,
            Kind::Cf02Donut => cf02_donut_ticks(&manifest, frame_count).map_err(err)?,
            Kind::Cf02Goblin => cf02_goblin_ticks(&manifest, frame_count).map_err(err)?,
            Kind::Engineer => engineer_ticks(frame_count).map_err(err)?,
            Kind::CarlDynamite => dynamite_ticks(&manifest, frame_count).map_err(err)?,
            // The verb clock (`crate::interact`) picks the frame, so no per-frame ticks.
            Kind::Interact => (vec![None; frame_count], Vec::new()),
        };
        let atlas_asset = str_field(&manifest, "output_atlas", &path)?
            .strip_prefix(crate::actor_sprite::ATLAS_ASSET_PREFIX)
            .ok_or_else(|| err("atlas is not below the asset prefix".into()))?
            .to_owned();
        Ok(Self {
            spec: ActorSpec {
                name,
                width,
                height,
                pivot_x: pivot.0,
                pivot_y: pivot.1,
                shadow_width: footprint.0,
                shadow_pivot_x: footprint.1,
            },
            atlas_asset,
            frame_count,
            tags,
            ticks,
            airborne_tags: airborne_list.into_iter().collect(),
            boxes,
            masks,
        })
    }

    pub fn load_carl_melee(root: &Path, source: FrameSource) -> Result<Self, String> {
        Self::load(
            root,
            "CARL_MELEE",
            CARL_MELEE_MANIFEST,
            CARL_SHADOW_FOOTPRINT,
            source,
            Kind::CarlMelee,
        )
    }

    /// Goblin variant 0 (A) or 1 (B).
    pub fn load_goblin(root: &Path, variant: usize, source: FrameSource) -> Result<Self, String> {
        let file = GOBLIN_MANIFESTS
            .get(variant)
            .ok_or_else(|| format!("no goblin variant {variant}"))?;
        Self::load(
            root,
            "GOBLIN",
            file,
            GOBLIN_SHADOW_FOOTPRINT,
            source,
            Kind::Goblin,
        )
    }

    /// ENEMY-GAMEPLAY-ENGINEER: the Goblin Engineer atlas (ENG-01). Its frames are
    /// chosen by the gameplay state (design section 10), so only the idle and walk
    /// loops carry ticks; every other frame is a hold.
    pub fn load_engineer(root: &Path, source: FrameSource) -> Result<Self, String> {
        Self::load(
            root,
            "ENGINEER",
            ENGINEER_MANIFEST,
            GOBLIN_SHADOW_FOOTPRINT,
            source,
            Kind::Engineer,
        )
    }

    /// ENG-02: the Engineer's slammed sheet (stomped, wall crumple, flung).
    pub fn load_engineer_slammed(root: &Path, source: FrameSource) -> Result<Self, String> {
        Self::load(
            root,
            "ENGINEER_SLAMMED",
            ENGINEER_SLAMMED_MANIFEST,
            GOBLIN_SHADOW_FOOTPRINT,
            source,
            Kind::Cf02Goblin,
        )
    }

    /// The Bad Llama atlas (LLAMA-01).
    pub fn load_llama(root: &Path, source: FrameSource) -> Result<Self, String> {
        Self::load(
            root,
            "LLAMA",
            LLAMA_MANIFEST,
            LLAMA_SHADOW_FOOTPRINT,
            source,
            Kind::Llama,
        )
    }

    /// LLAMA-03: the Bad Llama rush atlas. Its frames are chosen by the rush state
    /// ([`llama_rush_pose`]), so every frame is a hold.
    pub fn load_llama_rush(root: &Path, source: FrameSource) -> Result<Self, String> {
        Self::load(
            root,
            "LLAMA_RUSH",
            LLAMA_RUSH_MANIFEST,
            LLAMA_SHADOW_FOOTPRINT,
            source,
            Kind::LlamaRush,
        )
    }

    /// CF02-I: the Carl CF02 sheet (stomp, air kick, dive, bail-out, flinch,
    /// frustration). Every tag is placed by the pivot rule (the sheet's airborne
    /// frames are already lifted 2 px, like the locomotion jump frames), so the
    /// manifest's airborne tags are not used for anchoring.
    pub fn load_carl_cf02(root: &Path, source: FrameSource) -> Result<Self, String> {
        Self::load(
            root,
            "CARL_CF02",
            CARL_CF02_MANIFEST,
            CARL_SHADOW_FOOTPRINT,
            source,
            Kind::Cf02Carl,
        )
    }

    /// CP-B2: the ANIM-CARL-03 sheet (the standing throw and the moving overlay
    /// frames), pivot-anchored like the locomotion atlas.
    pub fn load_carl_dynamite(root: &Path, source: FrameSource) -> Result<Self, String> {
        Self::load(
            root,
            "CARL_DYNAMITE",
            CARL_DYNAMITE_MANIFEST,
            CARL_SHADOW_FOOTPRINT,
            source,
            Kind::CarlDynamite,
        )
    }

    /// GUILD-VERB-01: Carl's interact sheet (12 frames, three verbs), pivot-anchored like the
    /// locomotion atlas.
    pub fn load_carl_interact(root: &Path, source: FrameSource) -> Result<Self, String> {
        Self::load(
            root,
            "CARL_INTERACT",
            CARL_INTERACT_MANIFEST,
            CARL_SHADOW_FOOTPRINT,
            source,
            Kind::Interact,
        )
    }

    /// GUILD-VERB-01: Donut's interact sheet (8 frames, two verbs).
    pub fn load_donut_interact(root: &Path, source: FrameSource) -> Result<Self, String> {
        Self::load(
            root,
            "DONUT_INTERACT",
            DONUT_INTERACT_MANIFEST,
            DONUT_SHADOW_FOOTPRINT,
            source,
            Kind::Interact,
        )
    }

    /// CF02-I: Donut's startle and smug sheet.
    pub fn load_donut_cf02(root: &Path, source: FrameSource) -> Result<Self, String> {
        Self::load(
            root,
            "DONUT_CF02",
            DONUT_CF02_MANIFEST,
            DONUT_SHADOW_FOOTPRINT,
            source,
            Kind::Cf02Donut,
        )
    }

    /// CF02-I: the goblin acting sheet (hesitate, panic, taunt) of variant 0 or 1.
    pub fn load_goblin_cf02(
        root: &Path,
        variant: usize,
        source: FrameSource,
    ) -> Result<Self, String> {
        let file = GOBLIN_CF02_MANIFESTS
            .get(variant)
            .ok_or_else(|| format!("no goblin variant {variant}"))?;
        Self::load(
            root,
            "GOBLIN_CF02",
            file,
            GOBLIN_SHADOW_FOOTPRINT,
            source,
            Kind::Cf02Goblin,
        )
    }

    /// CF02-I: the goblin slammed sheet (stomped, wall crumple, flung) of variant
    /// 0 or 1.
    pub fn load_goblin_slammed(
        root: &Path,
        variant: usize,
        source: FrameSource,
    ) -> Result<Self, String> {
        let file = GOBLIN_SLAMMED_MANIFESTS
            .get(variant)
            .ok_or_else(|| format!("no goblin variant {variant}"))?;
        Self::load(
            root,
            "GOBLIN_SLAMMED",
            file,
            GOBLIN_SHADOW_FOOTPRINT,
            source,
            Kind::Cf02Goblin,
        )
    }

    /// First atlas frame and frame count of a tag.
    pub fn tag(&self, name: &str) -> (usize, usize) {
        *self
            .tags
            .get(name)
            .unwrap_or_else(|| panic!("atlas has no {name} tag"))
    }

    pub fn has_tag(&self, name: &str) -> bool {
        self.tags.contains_key(name)
    }

    pub fn is_airborne_tag(&self, name: &str) -> bool {
        self.airborne_tags.contains(name)
    }

    /// Ticks of one atlas frame (`None` = hold).
    pub fn frame_ticks(&self, frame: usize) -> Option<u32> {
        self.ticks[frame]
    }

    /// Total ticks of a tag's frames, `None` when one of them holds.
    pub fn tag_ticks(&self, name: &str) -> Option<u32> {
        let (first, len) = self.tag(name);
        self.ticks[first..first + len].iter().copied().sum()
    }

    /// Atlas frame shown `age` ticks after the tag started: frames play in order
    /// for their ticks, a hold frame stays, and past the end the last frame stays.
    pub fn frame_at(&self, name: &str, age: u32) -> usize {
        let (first, len) = self.tag(name);
        let mut left = age;
        for offset in 0..len {
            match self.ticks[first + offset] {
                None => return first + offset,
                Some(ticks) if left < ticks => return first + offset,
                Some(ticks) => left -= ticks,
            }
        }
        first + len - 1
    }

    /// Frame `index` inside a tag, clamped.
    pub fn frame_in(&self, name: &str, index: usize) -> usize {
        let (first, len) = self.tag(name);
        first + index.min(len - 1)
    }

    pub fn frame_box(&self, frame: usize) -> FrameBox {
        self.boxes[frame]
    }

    /// Foot height to hand to `actor_center` so the frame sits on the sim body:
    /// grounded tags keep the pivot on the physics foot; airborne tags put the
    /// frame's lowest opaque row on the bottom of the sim body.
    pub fn anchor_foot_y(&self, pose: &ArtPose, sim_foot_y: f32) -> f32 {
        self.anchor_foot_y_blend(pose, sim_foot_y, 1.0)
    }

    /// [`Self::anchor_foot_y`] with a blend from the pivot rule (`0.0`) to the
    /// lowest-row rule (`1.0`) for airborne tags. Carl's knockdown starts on the
    /// pivot rule, like the grounded hurt frame before it, and settles onto the
    /// lowest row by the apex ([`knockdown_blend`]).
    pub fn anchor_foot_y_blend(&self, pose: &ArtPose, sim_foot_y: f32, blend: f32) -> f32 {
        if self.is_airborne_tag(pose.tag) {
            let lift = (self.spec.pivot_y - self.boxes[pose.frame].lowest) as f32;
            sim_foot_y - lift * blend.clamp(0.0, 1.0)
        } else {
            sim_foot_y
        }
    }

    /// Contact shadow `(box width, box pivot column)` for a drawn frame: the
    /// idle footprint for upright frames, the opaque extent of a lying frame
    /// (down, corpse) so the shadow follows the body on the floor.
    pub fn shadow_footprint(&self, pose: &ArtPose) -> (i32, i32) {
        if matches!(pose.tag, "down" | "corpse" | "getup" | "knockdown") {
            let b = self.boxes[pose.frame];
            (b.right - b.left + 1, self.spec.pivot_x - b.left)
        } else {
            (self.spec.shadow_width, self.spec.shadow_pivot_x)
        }
    }

    /// Disagreements between the art timings and the sim windows. Empty when the
    /// art fits the sim. Reported, never patched over (the sim is not changed by
    /// the art integration).
    pub fn sim_mismatches(&self, kind_name: &str) -> Vec<String> {
        let out = std::cell::RefCell::new(Vec::new());
        let check = |what: &str, got: Option<u32>, want: u32| {
            if got != Some(want) {
                out.borrow_mut()
                    .push(format!("{kind_name} {what}: art {got:?} ticks, sim {want}"));
            }
        };
        if kind_name == "goblin" {
            check(
                "attack_tell",
                self.tag_ticks("attack_tell"),
                TELL_TICKS.into(),
            );
            check(
                "attack_swing",
                self.tag_ticks("attack_swing"),
                (IMPACT_END - TELL_TICKS).into(),
            );
            check(
                "attack_recover",
                self.tag_ticks("attack_recover"),
                (ATTACK_TICKS - IMPACT_END).into(),
            );
            // The impact frame (the swing's second frame) must be the hit window.
            let (first, _) = self.tag("attack_swing");
            check(
                "smear frame",
                self.frame_ticks(first),
                (SMEAR_END - TELL_TICKS).into(),
            );
            check(
                "impact frame",
                self.frame_ticks(first + 1),
                (IMPACT_END - SMEAR_END).into(),
            );
        } else {
            for (tag, id) in [
                ("punch_a", MoveId::Jab),
                ("punch_b", MoveId::Haymaker),
                ("kick", MoveId::PushKick),
            ] {
                let spec = crate::combat::carl_move(id).expect("move table");
                check(tag, self.tag_ticks(tag), spec.total().into());
                // Frame 0 of each move is the startup, frame `contact` the first
                // active tick (jab a1 at 3, haymaker b1 at 5, kick k3 at 13).
                let contact = self.frame_at(tag, spec.startup.into());
                let (first, _) = self.tag(tag);
                let want = first + strike_of(tag).expect("a move that hits").1;
                if contact != want {
                    out.borrow_mut().push(format!(
                        "carl {tag}: frame {contact} on the hit tick {}, expected the contact frame {want}",
                        spec.startup
                    ));
                }
            }
            check("hurt", self.tag_ticks("hurt"), HERO_HURT_TICKS.into());
        }
        out.into_inner()
    }
}

#[derive(Clone, Copy)]
enum Kind {
    /// GUILD-VERB-01: the interact sheets.
    Interact,
    CarlMelee,
    Goblin,
    Llama,
    /// LLAMA-03: the rush atlas (`rush_poses`).
    LlamaRush,
    /// CF02-I: the ART-CARL-CF02 sheet (`cf02_acting.frames`).
    Cf02Carl,
    /// CF02-I: the ART-DONUT-CF02 sheet (`cf02.moves`).
    Cf02Donut,
    /// CF02-I: the ART-GOB-CF02 acting and slammed sheets (`cf02_goblin.frames`).
    Cf02Goblin,
    /// ENEMY-GAMEPLAY-ENGINEER: the ENG-01 sheet.
    Engineer,
    /// CARL-PROG-01 CP-B2: the ANIM-CARL-03 dynamite sheet
    /// (`dynamite.moves.dynamite_throw.frames`, then the moving frames).
    CarlDynamite,
}

/// LLAMA-03: the rush atlas has [`llama_rush_pose::RUSH_FRAMES`] frames, each held by the rush state (no ticks of
/// its own); the manifest carries the `rush_poses` tick map.
fn llama_rush_ticks(
    manifest: &serde_json::Value,
    frame_count: usize,
) -> Result<(Vec<Option<u32>>, Vec<String>), String> {
    if frame_count != llama_rush_pose::RUSH_FRAMES {
        return Err(format!(
            "the rush atlas has {} frames, the atlas lists {frame_count}",
            llama_rush_pose::RUSH_FRAMES
        ));
    }
    if !manifest["rush_poses"].is_object() {
        return Err("rush_poses missing".into());
    }
    Ok((vec![None; frame_count], Vec::new()))
}

/// Per-frame ticks of the ENG-01 atlas: the idle loop is 38 ticks (frames 0 to 2:
/// 14, 10, 14), the walk loop 36 (frames 3 to 8, 6 ticks each, exactly his 40 px/s contact
/// plan); every other frame is held by the gameplay state, the ENG-02 get-up
/// (frames 18 and 19, by the lying age) and ram-run (frames 20 and 21, by the ram
/// age) included.
fn engineer_ticks(frame_count: usize) -> Result<(Vec<Option<u32>>, Vec<String>), String> {
    if frame_count != ENGINEER_FRAMES {
        return Err(format!(
            "ENG-01 and ENG-02 have {ENGINEER_FRAMES} frames, the atlas lists {frame_count}"
        ));
    }
    let mut ticks: Vec<Option<u32>> = vec![None; frame_count];
    for (frame, t) in [(0, 14), (1, 10), (2, 14)] {
        ticks[frame] = Some(t);
    }
    for slot in &mut ticks[3..=8] {
        *slot = Some(6);
    }
    Ok((ticks, Vec::new()))
}

/// Frames of the Engineer atlas: ENG-01's 18 and ENG-02's four.
pub const ENGINEER_FRAMES: usize = 22;

/// CP-B2: ticks of the dynamite sheet: the nine standing frames hold their
/// `duration_ticks`; the 36 moving frames follow the locomotion clock, so they
/// carry the preview value of 4 (the frame is chosen by the throw clock, never by
/// these). No airborne tag.
fn dynamite_ticks(
    manifest: &serde_json::Value,
    frame_count: usize,
) -> Result<(Vec<Option<u32>>, Vec<String>), String> {
    let frames = manifest["dynamite"]["moves"]["dynamite_throw"]["frames"]
        .as_array()
        .ok_or("dynamite.moves.dynamite_throw.frames missing")?;
    if frames.len() != 9 || frame_count != 9 + 6 * 6 {
        return Err(format!(
            "dynamite sheet: {} standing frames and {frame_count} atlas frames (expected 9 and 45)",
            frames.len()
        ));
    }
    let mut ticks = Vec::with_capacity(frame_count);
    for (i, frame) in frames.iter().enumerate() {
        let t = frame["duration_ticks"]
            .as_u64()
            .filter(|t| *t > 0)
            .ok_or_else(|| format!("dynamite frame {i} has no duration_ticks"))?;
        ticks.push(Some(t as u32));
    }
    ticks.resize(frame_count, Some(4));
    Ok((ticks, Vec::new()))
}

/// Per-frame ticks of the melee atlas from `melee.moves`, and its airborne tags.
fn carl_ticks(
    manifest: &serde_json::Value,
    tags: &BTreeMap<String, (usize, usize)>,
    frame_count: usize,
) -> Result<(Vec<Option<u32>>, Vec<String>), String> {
    let melee = &manifest["melee"];
    let mut ticks: Vec<Option<Option<u32>>> = vec![None; frame_count];
    // Tag to (move block, first listed frame): the knockback frame is the first
    // entry of the knockdown move, d0 and d1 follow it.
    for (tag, (move_name, skip)) in [
        ("punch_a", ("punch_a", 0)),
        ("punch_b", ("punch_b", 0)),
        ("kick", ("kick", 0)),
        ("hurt", ("hurt", 0)),
        ("knockback", ("knockdown", 0)),
        ("knockdown", ("knockdown", 1)),
        ("getup", ("getup", 0)),
    ] {
        let &(first, len) = tags
            .get(tag)
            .ok_or_else(|| format!("atlas has no {tag} tag"))?;
        let frames = melee["moves"][move_name]["frames"]
            .as_array()
            .ok_or_else(|| format!("melee.moves.{move_name}.frames missing"))?;
        let listed = &frames[skip.min(frames.len())..];
        if listed.len() < len {
            return Err(format!("melee.moves.{move_name} lists too few frames"));
        }
        for offset in 0..len {
            let what = format!("melee.moves.{move_name} frame {}", skip + offset);
            ticks[first + offset] = Some(range_ticks(&listed[offset]["ticks"], &what)?);
        }
    }
    let ticks = ticks
        .into_iter()
        .enumerate()
        .map(|(i, t)| t.ok_or_else(|| format!("frame {i} has no tick data")))
        .collect::<Result<Vec<_>, _>>()?;
    let airborne = string_list(&melee["airborne_tags"])?;
    Ok((ticks, airborne))
}

/// Per-frame ticks of the goblin atlas from `goblin.frames`.
fn goblin_ticks(
    manifest: &serde_json::Value,
    frame_count: usize,
) -> Result<(Vec<Option<u32>>, Vec<String>), String> {
    let goblin = &manifest["goblin"];
    let frames = goblin["frames"].as_array().ok_or("goblin.frames missing")?;
    if frames.len() != frame_count {
        return Err(format!(
            "goblin.frames lists {} frames, the atlas has {frame_count}",
            frames.len()
        ));
    }
    let mut ticks = Vec::with_capacity(frame_count);
    for (i, frame) in frames.iter().enumerate() {
        if frame["index"].as_u64() != Some(i as u64) {
            return Err(format!("goblin.frames[{i}] has the wrong index"));
        }
        ticks.push(if frame["ticks"].is_null() {
            None
        } else {
            Some(
                frame["ticks"]
                    .as_u64()
                    .filter(|t| *t > 0)
                    .ok_or_else(|| format!("goblin.frames[{i}] has bad ticks"))?
                    as u32,
            )
        });
    }
    Ok((ticks, string_list(&goblin["airborne_tags"])?))
}

/// Per-frame ticks of the llama atlas from `llama.frames` (a `ticks` of null
/// holds the frame). The llama has no airborne tag.
fn llama_ticks(
    manifest: &serde_json::Value,
    frame_count: usize,
) -> Result<(Vec<Option<u32>>, Vec<String>), String> {
    let frames = manifest["llama"]["frames"]
        .as_array()
        .ok_or("llama.frames missing")?;
    if frames.len() != frame_count {
        return Err(format!(
            "llama.frames lists {} frames, the atlas has {frame_count}",
            frames.len()
        ));
    }
    let mut ticks = Vec::with_capacity(frame_count);
    for (i, frame) in frames.iter().enumerate() {
        if frame["index"].as_u64() != Some(i as u64) {
            return Err(format!("llama.frames[{i}] has the wrong index"));
        }
        ticks.push(if frame["ticks"].is_null() {
            None
        } else {
            Some(
                frame["ticks"]
                    .as_u64()
                    .filter(|t| *t > 0)
                    .ok_or_else(|| format!("llama.frames[{i}] has bad ticks"))?
                    as u32,
            )
        });
    }
    Ok((ticks, Vec::new()))
}

/// Ticks of a CF02 sheet from a `frames` list whose entries carry `index` and
/// `ticks` (a number, or null for a hold).
fn listed_frame_ticks(
    frames: &serde_json::Value,
    frame_count: usize,
    what: &str,
) -> Result<Vec<Option<u32>>, String> {
    let frames = frames
        .as_array()
        .ok_or_else(|| format!("{what}.frames missing"))?;
    if frames.len() != frame_count {
        return Err(format!(
            "{what}.frames lists {} frames, the atlas has {frame_count}",
            frames.len()
        ));
    }
    frames
        .iter()
        .enumerate()
        .map(|(i, frame)| {
            if frame["index"].as_u64() != Some(i as u64) {
                return Err(format!("{what}.frames[{i}] has the wrong index"));
            }
            if frame["ticks"].is_null() {
                return Ok(None);
            }
            frame["ticks"]
                .as_u64()
                .filter(|t| *t > 0)
                .map(|t| Some(t as u32))
                .ok_or_else(|| format!("{what}.frames[{i}] has bad ticks"))
        })
        .collect()
}

/// Carl CF02 ticks: the manifest's per-frame ticks, with the last frame of a
/// `hold` tag (the dive plunge, the bail-out scramble) held until the state
/// changes. No airborne tag: the sheet is pivot-anchored.
fn cf02_carl_ticks(
    manifest: &serde_json::Value,
    frame_count: usize,
) -> Result<(Vec<Option<u32>>, Vec<String>), String> {
    let block = &manifest["cf02_acting"];
    let mut ticks = listed_frame_ticks(&block["frames"], frame_count, "cf02_acting")?;
    let tags = manifest["animation_tags"]
        .as_array()
        .ok_or("animation_tags missing")?;
    let frames = block["frames"]
        .as_array()
        .ok_or("cf02_acting.frames missing")?;
    for tag in tags.iter().filter_map(|t| t.as_str()) {
        if block["playback"][tag].as_str() == Some("hold")
            && let Some(last) = frames.iter().rposition(|f| f["tag"].as_str() == Some(tag))
        {
            ticks[last] = None;
        }
    }
    Ok((ticks, Vec::new()))
}

/// Donut CF02 ticks from `cf02.moves` (each frame's tick range) through the
/// `cf02.frames` name to index map.
fn cf02_donut_ticks(
    manifest: &serde_json::Value,
    frame_count: usize,
) -> Result<(Vec<Option<u32>>, Vec<String>), String> {
    let block = &manifest["cf02"];
    let mut ticks: Vec<Option<u32>> = vec![None; frame_count];
    let mut seen = vec![false; frame_count];
    let moves = block["moves"].as_object().ok_or("cf02.moves missing")?;
    for (name, mv) in moves {
        for entry in mv["frames"].as_array().ok_or("cf02.moves frames missing")? {
            let frame_name = entry["name"].as_str().ok_or("cf02.moves frame name")?;
            let index = block["frames"][frame_name]["index"]
                .as_u64()
                .ok_or_else(|| format!("cf02.frames has no {frame_name}"))?
                as usize;
            if index >= frame_count {
                return Err(format!("cf02.frames.{frame_name} is outside the atlas"));
            }
            ticks[index] =
                range_ticks(&entry["ticks"], &format!("cf02.moves.{name}.{frame_name}"))?;
            seen[index] = true;
        }
    }
    if let Some(missing) = seen.iter().position(|s| !s) {
        return Err(format!("frame {missing} has no tick data"));
    }
    Ok((ticks, Vec::new()))
}

/// Goblin CF02 ticks from `cf02_goblin.frames`.
fn cf02_goblin_ticks(
    manifest: &serde_json::Value,
    frame_count: usize,
) -> Result<(Vec<Option<u32>>, Vec<String>), String> {
    let ticks = listed_frame_ticks(
        &manifest["cf02_goblin"]["frames"],
        frame_count,
        "cf02_goblin",
    )?;
    Ok((ticks, Vec::new()))
}

fn string_list(value: &serde_json::Value) -> Result<Vec<String>, String> {
    value
        .as_array()
        .ok_or("airborne_tags missing")?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_owned)
                .ok_or_else(|| "airborne_tags entry is not a string".to_owned())
        })
        .collect()
}

// ---------------------------------------------------------------- poses -----

/// What is drawn: an atlas frame and the tag it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtPose {
    pub tag: &'static str,
    pub frame: usize,
}

impl ArtPose {
    /// `tag:atlas_index`, for logs and state lines.
    pub fn label(&self) -> String {
        format!("{}:{}", self.tag, self.frame)
    }
}

/// The melee tag of a Carl move (`None` for the WORK-021 greybox strike).
pub fn melee_tag(id: MoveId) -> Option<&'static str> {
    match id {
        MoveId::Jab => Some("punch_a"),
        MoveId::Haymaker => Some("punch_b"),
        MoveId::PushKick => Some("kick"),
        // CF02-I: the follow-through plays the approved kick frames without the
        // chamber ([`follow_kick_frame`], Q05). The stomp, air kick and dive have
        // their own ART-CARL-CF02 frames ([`crate::combat_anim::cf02`]), so the
        // melee atlas draws nothing for them and no strike shift is made.
        MoveId::FollowKick => Some("kick"),
        MoveId::Stomp
        | MoveId::AirKick
        | MoveId::DiveStomp
        | MoveId::GreyboxStrike
        | MoveId::Swat
        | MoveId::Pounce
        | MoveId::MagicMissile => None,
    }
}

/// What Carl's melee frame depends on, from one finished tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CarlInput {
    /// The running move and its clock (held while the move is frozen).
    pub melee: Option<(MoveId, u16)>,
    /// Ticks of the hurt flinch left (counts down from `HERO_HURT_TICKS`).
    pub hurt_ticks: u8,
    /// Carl is frozen in the hit-stop of a blow on him.
    pub frozen: bool,
    pub defeated: bool,
    pub grounded: bool,
    /// Carl's vertical speed (px/s, y up): the knockdown anchor blend.
    pub vy: f32,
    /// Ticks since Carl touched down after his knockdown (0 on the first
    /// grounded tick).
    pub landed_age: Option<u32>,
}

/// Carl's combat frame, or `None` when the locomotion animation draws him.
///
/// * defeated: the hit-stop shows the impact frame, then the launched frame
///   while he is in the air, then the impact (d0) and the sprawl (d1) from
///   touchdown (there is no sim state that gets him up: the fixture restarts);
/// * hurt: `hurt_ticks` counts down from 18, the frame follows the 18 ticks of
///   the flinch (h0, h1, h2); the freeze of the blow holds h0;
/// * a running move: frame by move tick (a1, b1, k3 on the hit tick).
pub fn carl_pose(clips: &SpriteClips, input: &CarlInput) -> Option<ArtPose> {
    if input.defeated {
        return Some(if input.frozen {
            ArtPose {
                tag: "hurt",
                frame: clips.frame_at("hurt", 0),
            }
        } else if input.grounded {
            ArtPose {
                tag: "knockdown",
                frame: clips.frame_at("knockdown", input.landed_age.unwrap_or(0)),
            }
        } else {
            ArtPose {
                tag: "knockback",
                frame: clips.frame_at("knockback", 0),
            }
        });
    }
    if input.hurt_ticks > 0 {
        let age = u32::from(HERO_HURT_TICKS.saturating_sub(input.hurt_ticks));
        return Some(ArtPose {
            tag: "hurt",
            frame: clips.frame_at("hurt", age),
        });
    }
    let (id, tick) = input.melee?;
    let tag = melee_tag(id)?;
    if id == MoveId::FollowKick {
        return Some(ArtPose {
            tag,
            frame: follow_kick_frame(clips, tick),
        });
    }
    Some(ArtPose {
        tag,
        frame: clips.frame_at(tag, u32::from(tick)),
    })
}

/// The approved kick frames the follow-through plays, as `(index inside the
/// kick tag, ticks)` (CF02 acting spec section 3, Q05): k0 for the load, k2 for
/// the last two startup ticks (the chamber k1 is skipped, because the confirm
/// only fires on a goblin the jab already staggered), the contact frame k3 on the
/// hit tick, then k4, k5 and k6. The 32 ticks of `FollowKick` (startup 9, active
/// 4, recovery 19) in order; a test pins the total and the contact frame.
pub const FOLLOW_KICK_FRAMES: [(usize, u32); 6] = [(0, 7), (2, 2), (3, 6), (4, 5), (5, 7), (6, 5)];

/// The atlas frame of the kick tag shown on follow-through tick `tick`.
pub fn follow_kick_frame(clips: &SpriteClips, tick: u16) -> usize {
    let (first, _) = clips.tag("kick");
    let mut left = u32::from(tick);
    for (index, ticks) in FOLLOW_KICK_FRAMES {
        if left < ticks {
            return first + index;
        }
        left -= ticks;
    }
    first + FOLLOW_KICK_FRAMES[FOLLOW_KICK_FRAMES.len() - 1].0
}

/// What a goblin's frame depends on, from one finished tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoblinInput {
    pub mode: GoblinMode,
    pub attack_tick: u16,
    /// The goblin's own step is frozen by hit-stop this tick. The sim consumes
    /// the last freeze tick while its counter reads 0, so this is "the counter
    /// is above 0 now or was at the end of the previous tick".
    pub frozen: bool,
    pub down_ticks: u16,
    pub vy: f32,
    /// Ticks since a living launch (the kick) was released.
    pub flight_ticks: u16,
    /// The `knockback_contact` pose is held (CP-02 contact, first ticks).
    pub contact: bool,
    /// Walking this tick (neutral, not sliding, x changed).
    pub walking: bool,
    /// Ground covered since the walk began, in thousandths of a pixel.
    pub walk_milli_px: u32,
    /// Idle clock (scene tick plus the goblin's offset).
    pub idle_tick: u64,
    /// ENEMY-GAMEPLAY-ENGINEER: set only for the Engineer (drawn from the ENG-01
    /// atlas); `None` for every goblin.
    pub engineer: Option<EngineerFrame>,
}

/// What the Engineer's frame depends on besides a goblin's input (design section
/// 10): the phase of his attack, the age of a bonk and the lying times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineerFrame {
    pub phase: Option<crate::engineer::EngineerPhase>,
    /// Bonk ticks spent so far (`None` outside a bonk).
    pub bonk_age: Option<u16>,
    /// The bonk's hurt ticks: the hurt frame shows through them, the daze after.
    pub bonk_hurt_ticks: u16,
    /// Ticks a living Engineer lies before the get-up (CF02-Q04).
    pub lying_ticks: u16,
    /// Ram ticks spent so far (`None` outside the ram): the ram frames cycle by
    /// it ([`RAM_CYCLE`]).
    pub ram_age: Option<u16>,
}

/// ENG-02: the frames of the ram, by ram age at [`RAM_FRAME_TICKS`] ticks a frame:
/// the braced frame 12 (`attack_ram` 1), the passing frame 20 (`attack_ram_run` 0),
/// the opposite contact 21 (`attack_ram_run` 1), and 20 again. The cycle reuses
/// frame 20 for both passings (an accepted cheat flagged for the owner's game-feel
/// review).
pub const RAM_CYCLE: [(&str, usize); 4] = [
    ("attack_ram", 1),
    ("attack_ram_run", 0),
    ("attack_ram_run", 1),
    ("attack_ram_run", 0),
];
/// Ticks each frame of [`RAM_CYCLE`] shows.
pub const RAM_FRAME_TICKS: u16 = 4;
/// ENG-02: ticks of the get-up sit (`getup` 0, atlas frame 18) before the shove (`getup` 1, frame 19): 10 and 12
/// make the 22 ticks of `q04_getup_ticks`.
pub const GETUP_SIT_TICKS: u16 = 10;

/// The Engineer's frame (design section 10): idle and walk loops; rear 9, point
/// 10, launch 11, ram 12 (held), recover 13 (held); a bonk shows the hurt frame 14
/// for its hurt ticks and the recover frame 13 for the daze; hurt, launched and
/// airborne 14; a living lying body 15 then 16 held, the get-up 13; a defeat 15
/// (5 ticks), 16 (10), then 17 held.
///
/// ENG-02: the get-up shows `getup` 0 (the sit, 18) for its first
/// [`GETUP_SIT_TICKS`] ticks and `getup` 1 (the shove, 19) after, and holds the
/// shove if the lying body outlasts the window; the ram cycles
/// [`RAM_CYCLE`] by ram age.
pub fn engineer_pose(clips: &SpriteClips, input: &GoblinInput, e: &EngineerFrame) -> ArtPose {
    use crate::engineer::EngineerPhase as P;
    let one = |tag: &'static str, index: usize| ArtPose {
        tag,
        frame: clips.frame_in(tag, index),
    };
    match input.mode {
        GoblinMode::Corpse => one("down", 2),
        GoblinMode::Down => {
            let t = input.down_ticks;
            one(
                "down",
                if t < 5 {
                    0
                } else if t < 15 {
                    1
                } else {
                    2
                },
            )
        }
        GoblinMode::Airborne | GoblinMode::Launched | GoblinMode::Hurt => one("hurt", 0),
        GoblinMode::Downed => {
            if input.down_ticks < e.lying_ticks {
                one("down", usize::from(input.down_ticks >= 5))
            } else {
                // ENG-02: the get-up, sit then shove; a window that runs past 22
                // ticks holds the shove.
                let age = input.down_ticks - e.lying_ticks;
                one("getup", usize::from(age >= GETUP_SIT_TICKS))
            }
        }
        GoblinMode::Attack => match e.phase {
            Some(P::Rear) => one("attack_tell", 0),
            Some(P::Point) => one("attack_tell", 1),
            Some(P::Launch) => one("attack_ram", 0),
            Some(P::Ram) => {
                let age = e.ram_age.unwrap_or(0) / RAM_FRAME_TICKS;
                let (tag, index) = RAM_CYCLE[usize::from(age) % RAM_CYCLE.len()];
                one(tag, index)
            }
            Some(P::Recover) => one("attack_recover", 0),
            Some(P::Bonk) => {
                if e.bonk_age.is_some_and(|age| age <= e.bonk_hurt_ticks) {
                    one("hurt", 0)
                } else {
                    one("attack_recover", 0)
                }
            }
            None => one("idle", 0),
        },
        GoblinMode::Neutral => {
            if input.walking {
                let (_, len) = clips.tag("walk");
                // 6 ticks a frame at 40 px/s: 4 px of ground per frame.
                let step = (input.walk_milli_px / 4000) as usize;
                one("walk", step % len)
            } else {
                let cycle = u64::from(clips.tag_ticks("idle").expect("idle frames are timed"));
                ArtPose {
                    tag: "idle",
                    frame: clips.frame_at("idle", (input.idle_tick % cycle) as u32),
                }
            }
        }
    }
}

/// Ticks since the launch was released (1 on the first moving tick), recovered
/// from the vertical speed: the lethal launch speed and gravity are constants
/// of `feel.rs`, and the killing hit is the only thing that sets `vy` of an
/// `Airborne` goblin. 0 while the speed is still the launch speed (the freeze).
///
/// **Depends on the `vy` source:** it is only right while every `Airborne`
/// launch uses `LETHAL_KNOCK_VY` and plain gravity. A launch with another speed
/// (the CP-02 kick) must carry its own launch speed (or an age counter) in the
/// state instead of reusing this.
pub fn launch_age(vy: f32) -> u32 {
    let per_tick = KNOCK_GRAVITY * TICK_SECONDS;
    ((LETHAL_KNOCK_VY - vy) / per_tick).round().max(0.0) as u32
}

/// A goblin's frame.
///
/// * neutral: `walk` while it walks (frame by ground covered), else the looping
///   `idle`;
/// * attack: tell, smear, impact and recover by the attack tick (impact frame
///   exactly on the hit window); the clock freezes in hit-stop, so does the frame;
/// * hurt: the recoil frame (16) for the whole hit-stop (at least 4 ticks, the
///   jab's freeze), then the stagger frame (17) held for the rest (follow-up F2);
/// * airborne (the killing hit): recoil (16) through the kill's hit-stop, then
///   the launch frame (18) for 4 ticks from the release, then the airborne frame
///   (19) until touchdown (follow-up F1);
/// * down: the three lying frames by `down_ticks`; corpse: the corpse frame.
pub fn goblin_pose(clips: &SpriteClips, input: &GoblinInput) -> ArtPose {
    if let Some(e) = input.engineer {
        return engineer_pose(clips, input, &e);
    }
    let at = |tag: &'static str, age: u32| ArtPose {
        tag,
        frame: clips.frame_at(tag, age),
    };
    match input.mode {
        GoblinMode::Corpse => at("corpse", 0),
        GoblinMode::Down => at("down", u32::from(input.down_ticks)),
        GoblinMode::Airborne => {
            if input.frozen {
                ArtPose {
                    tag: "hurt",
                    frame: clips.frame_in("hurt", 0),
                }
            } else {
                at("knockback", launch_age(input.vy).saturating_sub(1))
            }
        }
        // A living kick launch (D-02): recoil through the hit-stop, then the
        // launch and airborne frames by flight age; on a goblin-to-goblin
        // contact (D-03, CP-02) the crumpled `knockback_contact` frame.
        GoblinMode::Launched => {
            if input.contact {
                at("knockback_contact", 0)
            } else if input.frozen {
                ArtPose {
                    tag: "hurt",
                    frame: clips.frame_in("hurt", 0),
                }
            } else {
                at("knockback", u32::from(input.flight_ticks).saturating_sub(1))
            }
        }
        // Landed alive: the lying frames, then the get-up frames.
        GoblinMode::Downed => {
            let lying = DOWNED_TICKS;
            if input.down_ticks < lying {
                at("down", u32::from(input.down_ticks))
            } else {
                at("getup", u32::from(input.down_ticks - lying))
            }
        }
        GoblinMode::Hurt => ArtPose {
            tag: "hurt",
            frame: clips.frame_in("hurt", usize::from(!input.frozen)),
        },
        GoblinMode::Attack => {
            let tick = input.attack_tick;
            match crate::goblin::attack_phase(tick) {
                AttackPhase::Tell => at("attack_tell", u32::from(tick)),
                AttackPhase::Smear | AttackPhase::Impact => {
                    at("attack_swing", u32::from(tick - TELL_TICKS))
                }
                AttackPhase::Recover => at("attack_recover", u32::from(tick - IMPACT_END)),
            }
        }
        GoblinMode::Neutral => {
            if input.walking {
                let (_, len) = clips.tag("walk");
                let step = (input.walk_milli_px / WALK_STRIDE_MILLI_PX) as usize;
                ArtPose {
                    tag: "walk",
                    frame: clips.frame_in("walk", step % len),
                }
            } else {
                let cycle = u64::from(clips.tag_ticks("idle").expect("idle frames are timed"));
                at("idle", (input.idle_tick % cycle) as u32)
            }
        }
    }
}

/// What a llama's frame depends on, from one finished tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LlamaInput {
    pub mode: LlamaMode,
    pub attack_tick: u16,
    /// CORRAL-B wire: the attack is the corral rush. A rush never shows the
    /// throat glow or the spit swing (idle frames; the stand-in is CORRAL-E's,
    /// the art LLAMA-03's).
    pub rush: bool,
    pub down_ticks: u16,
    /// Walking this tick (neutral, x changed).
    pub walking: bool,
    /// Ground covered since the walk began, in thousandths of a pixel.
    pub walk_milli_px: u32,
    /// Idle clock (scene tick plus the llama's offset).
    pub idle_tick: u64,
}

/// A llama's frame (LLAMA-01 tags): `idle` (looping), `walk` (by ground
/// covered), `attack_tell` through the 36 tick tell (its last frame, the glow
/// peak, holds past the art's 30 ticks), `attack_swing` from the release (the
/// first tick of frame 9) with its recover frame held to the end of the attack,
/// `hurt`, and the three `down` frames by `down_ticks` (the last held; a corpse
/// keeps it). The slam and crush poses are the presenter's, not this function's.
pub fn llama_pose(clips: &SpriteClips, input: &LlamaInput) -> ArtPose {
    let at = |tag: &'static str, age: u32| ArtPose {
        tag,
        frame: clips.frame_at(tag, age),
    };
    match input.mode {
        LlamaMode::Corpse => at("down", u32::MAX / 2),
        LlamaMode::Down => at("down", u32::from(input.down_ticks)),
        LlamaMode::Hurt => ArtPose {
            tag: "hurt",
            frame: clips.frame_in("hurt", 0),
        },
        LlamaMode::Attack if input.rush => {
            let cycle = u64::from(clips.tag_ticks("idle").expect("idle frames are timed"));
            at("idle", (input.idle_tick % cycle) as u32)
        }
        LlamaMode::Attack => {
            let tick = input.attack_tick;
            if tick < LLAMA_TELL_TICKS {
                at("attack_tell", u32::from(tick))
            } else {
                at("attack_swing", u32::from(tick - LLAMA_TELL_TICKS))
            }
        }
        LlamaMode::Neutral => {
            if input.walking {
                let (_, len) = clips.tag("walk");
                let step = (input.walk_milli_px / LLAMA_WALK_STRIDE_MILLI_PX) as usize;
                ArtPose {
                    tag: "walk",
                    frame: clips.frame_in("walk", step % len),
                }
            } else {
                let cycle = u64::from(clips.tag_ticks("idle").expect("idle frames are timed"));
                at("idle", (input.idle_tick % cycle) as u32)
            }
        }
    }
}

// -------------------------------------------------------- strike contact -----

/// The striking limb of a Carl move and the index, inside the tag, of its
/// contact frame (a1, b1, k3): the layers of the melee grid that hold the arm
/// or the leg that lands the blow.
pub fn strike_of(tag: &str) -> Option<(&'static [&'static str], usize)> {
    match tag {
        "punch_a" => Some((&["far_arm"], 1)),
        "punch_b" => Some((&["near_arm"], 1)),
        "kick" => Some((&["shoes"], 3)),
        _ => None,
    }
}

/// Carl is on a contact frame of a move that hits.
pub fn is_contact_pose(clips: &SpriteClips, pose: &ArtPose) -> bool {
    strike_of(pose.tag).is_some_and(|(_, index)| pose.frame == clips.tag(pose.tag).0 + index)
}

/// Left edge of a drawn sprite in world pixels (`actor_center` places it).
fn placed_left(spec: ActorSpec, foot_x: f32, flip: bool) -> i32 {
    actor_center(spec, foot_x, 0.0, flip).0 as i32 - spec.width / 2
}

/// Carl's contact frame against a frozen goblin, as they are placed.
pub struct StrikeGeometry<'a> {
    pub carl: &'a SpriteClips,
    pub carl_pose: ArtPose,
    pub carl_x: f32,
    pub carl_flip: bool,
    pub carl_foot_y: f32,
    pub goblin: &'a SpriteClips,
    pub goblin_pose: ArtPose,
    pub goblin_x: f32,
    pub goblin_flip: bool,
    pub goblin_foot_y: f32,
}

impl StrikeGeometry<'_> {
    /// Pixels where the striking limb and the goblin are both opaque, with the
    /// goblin drawn `dx` pixels to the right of its sim position.
    pub fn overlap_px(&self, dx: i32) -> usize {
        let Some((layers, _)) = strike_of(self.carl_pose.tag) else {
            return 0;
        };
        let (cw, gw) = (self.carl.spec.width, self.goblin.spec.width);
        let carl_left = placed_left(self.carl.spec, self.carl_x, self.carl_flip);
        let goblin_left = placed_left(self.goblin.spec, self.goblin_x, self.goblin_flip) + dx;
        // Frame rows are counted from the top, both pivot rows are the feet.
        let dy = (self.carl_foot_y - self.goblin_foot_y).round() as i32;
        let carl = &self.carl.masks[self.carl_pose.frame];
        let goblin = &self.goblin.masks[self.goblin_pose.frame].all;
        let mut count = 0;
        for row in 0..self.carl.spec.height {
            let limb = layers
                .iter()
                .filter_map(|l| carl.layers.get(*l))
                .fold(0u64, |acc, m| acc | m[row as usize]);
            let goblin_row = row - dy;
            if limb == 0 || !(0..self.goblin.spec.height).contains(&goblin_row) {
                continue;
            }
            for x in (0..cw).filter(|x| limb >> x & 1 == 1) {
                let world = carl_left + if self.carl_flip { cw - 1 - x } else { x };
                let column = world - goblin_left;
                let column = if self.goblin_flip {
                    gw - 1 - column
                } else {
                    column
                };
                if (0..gw).contains(&column) && goblin[goblin_row as usize] >> column & 1 == 1 {
                    count += 1;
                }
            }
        }
        count
    }

    /// Signed pixels to draw the goblin toward Carl: the smallest shift in
    /// `0..=MAX_STRIKE_SHIFT_PX` that overlaps the striking limb by
    /// [`STRIKE_TARGET_OVERLAP_PX`] pixels (the maximum when none does). When
    /// the limb already sinks deeper than [`STRIKE_MAX_OVERLAP_PX`] (the kick's
    /// sole at the reach limit) the goblin is drawn back instead, the smallest
    /// shift that brings it down to that.
    pub fn shift_px(&self) -> i32 {
        // The goblin stands in front of Carl: to his left when he is mirrored.
        let toward = if self.carl_flip { 1 } else { -1 };
        if self.overlap_px(0) > STRIKE_MAX_OVERLAP_PX {
            return (1..=MAX_STRIKE_SHIFT_PX)
                .find(|s| self.overlap_px(-toward * s) <= STRIKE_MAX_OVERLAP_PX)
                .map_or(0, |s| -toward * s);
        }
        (0..=MAX_STRIKE_SHIFT_PX)
            .find(|s| self.overlap_px(toward * s) >= STRIKE_TARGET_OVERLAP_PX)
            .unwrap_or(MAX_STRIKE_SHIFT_PX)
            * toward
    }
}

/// Carl's knockdown anchor blend from his vertical speed: 0 at the launch
/// (pivot rule, like the hurt frame before it), 1 from the apex on (lowest row
/// on the sim body, so the sprawl lands without a pop).
pub fn knockdown_blend(vy: f32) -> f32 {
    ((HERO_KNOCKDOWN_VY - vy) / HERO_KNOCKDOWN_VY).clamp(0.0, 1.0)
}

// ------------------------------------------------------------- tracker -----

#[derive(Debug, Clone, Copy)]
struct GoblinTrack {
    prev_x: f32,
    /// Hit-stop counter at the end of the previous tick.
    prev_hit_stop: u8,
    walk_milli_px: u32,
    input: GoblinInput,
    /// Drawn shift toward Carl while he strikes the frozen goblin, the value it
    /// was set to, the ticks left to ease it back and the overlap it gives.
    strike_dx: i32,
    strike_set: i32,
    ease_left: u8,
    overlap: Option<usize>,
    /// Drawn nudge that keeps stacked goblins apart.
    stack_dx: i32,
}

#[derive(Debug, Clone, Copy)]
struct LlamaTrack {
    prev_x: f32,
    walk_milli_px: u32,
    input: LlamaInput,
}

/// Steps the presentation state of Carl's melee and every goblin once per fixed
/// tick, from the finished simulation state. Nothing flows back to the sim.
#[derive(Debug, Clone)]
pub struct CombatAnimators {
    pub carl: SpriteClips,
    /// Goblin atlases by variant (A, B).
    pub goblins: [SpriteClips; GOBLIN_VARIANTS],
    carl_input: CarlInput,
    /// Carl's hit-stop counter at the end of the previous tick.
    prev_hero_stop: u8,
    /// Donut's damage-reaction input (idle outside a live party) and her
    /// hit-stop counter at the end of the previous tick.
    donut_hurt: donut_art::HurtInput,
    prev_donut_stop: u8,
    tracks: Vec<GoblinTrack>,
    /// The Bad Llama atlas and per-llama tracks (a fixture with llamas only).
    llama: Option<SpriteClips>,
    llama_tracks: Vec<LlamaTrack>,
    /// ENEMY-GAMEPLAY-ENGINEER: the ENG-01 atlas (`None`: an Engineer, if any, is
    /// drawn with a goblin atlas).
    engineer: Option<SpriteClips>,
    /// CF02-I: the final CF02 frames and the acting overlays (`None` outside a
    /// `cf02_rules` run, so every legacy state line is unchanged).
    acting: Option<cf02::ActingAnimators>,
    /// CP-B2: the ANIM-CARL-03 sheet and the throw frame of the finished tick
    /// (`None` everywhere there is no throw, so every legacy state line is
    /// unchanged).
    dynamite: Option<SpriteClips>,
    throw_pose: Option<ArtPose>,
}

impl CombatAnimators {
    pub fn new(
        carl: SpriteClips,
        goblins: [SpriteClips; GOBLIN_VARIANTS],
        combat: &CombatState,
    ) -> Self {
        let mut this = Self {
            carl,
            goblins,
            carl_input: CarlInput::default(),
            prev_hero_stop: 0,
            donut_hurt: donut_art::HurtInput::default(),
            prev_donut_stop: 0,
            tracks: Vec::new(),
            llama: None,
            llama_tracks: Vec::new(),
            engineer: None,
            acting: None,
            dynamite: None,
            throw_pose: None,
        };
        this.reset(combat);
        this
    }

    /// CP-B2: draw Carl's dynamite throw from the ANIM-CARL-03 sheet `clips`.
    pub fn with_dynamite(mut self, clips: SpriteClips) -> Self {
        self.dynamite = Some(clips);
        self
    }

    /// The ANIM-CARL-03 sheet, if this run draws the throw.
    pub fn dynamite_clips(&self) -> Option<&SpriteClips> {
        self.dynamite.as_ref()
    }

    /// CP-B2: advance the throw frame by one tick; call right after
    /// [`Self::step`] of the same tick. `run_frame` is the frame inside the
    /// locomotion `run` clip while Carl's locomotion tag is `run`, else `None`.
    /// The throw never shows over a melee, hurt or defeat frame (the simulation
    /// cancels the throw on a hit; this keeps the layer order true in a replay
    /// view as well).
    pub fn step_throw(&mut self, scene: &SceneSim, combat: &CombatState, run_frame: Option<usize>) {
        self.throw_pose = None;
        let Some(clips) = self.dynamite.as_ref() else {
            return;
        };
        if self.carl_pose().is_some() || combat.carl_defeated() {
            return;
        }
        if let Some(clock) = combat.thrower().throw_clock(scene.world.tick) {
            self.throw_pose = dynamite::throw_pose(clips, clock, run_frame);
        }
    }

    /// Carl's throw frame in the dynamite atlas, or `None` (the melee atlas or the
    /// locomotion animation draws him).
    pub fn carl_throw_pose(&self) -> Option<ArtPose> {
        self.throw_pose
    }

    /// Draw the Goblin Engineer of the fixture from `clips` (ENG-01).
    pub fn with_engineer(mut self, clips: SpriteClips, combat: &CombatState) -> Self {
        self.engineer = Some(clips);
        self.reset(combat);
        self
    }

    /// The ENG-01 atlas, if loaded.
    pub fn engineer_clips(&self) -> Option<&SpriteClips> {
        self.engineer.as_ref()
    }

    /// The Engineer's frame inputs for goblin `g` (`None` for a goblin, or while no
    /// ENG-01 atlas is loaded).
    fn engineer_frame(
        &self,
        g: &crate::goblin::Goblin,
        combat: &CombatState,
    ) -> Option<EngineerFrame> {
        let tuning = g
            .eng_tuning
            .filter(|_| g.is_engineer() && self.engineer.is_some())?;
        Some(EngineerFrame {
            phase: g.engineer_phase(),
            bonk_age: g.bonk_age(),
            bonk_hurt_ticks: tuning.engq13_hurt_ticks,
            lying_ticks: combat.cf02().map_or(DOWNED_TICKS, |t| t.q04_downed_ticks),
            ram_age: g.ram_age(),
        })
    }

    /// CF02-I: draw the final CF02 frames and the Q69 to Q72 overlays of a
    /// `cf02_rules` run (`tuning` is `combat.cf02()`). Presentation only.
    pub fn with_acting(
        mut self,
        art: cf02::ActingArt,
        tuning: &'static crate::cf02::Cf02Tuning,
        combat: &CombatState,
    ) -> Self {
        self.acting = Some(cf02::ActingAnimators::new(art, tuning, combat));
        self
    }

    /// The CF02 acting presentation, if this run has it.
    pub fn acting(&self) -> Option<&cf02::ActingAnimators> {
        self.acting.as_ref()
    }

    /// Advance the CF02 acting presentation by one tick; call right after
    /// [`Self::step`] of the same tick, with the locomotion tags the two heroes
    /// animate with.
    pub fn step_acting(
        &mut self,
        scene: &SceneSim,
        combat: &CombatState,
        carl_loco: crate::anim::Tag,
        donut_loco: crate::anim::Tag,
    ) {
        let carl_legacy = self.carl_pose().is_some();
        let carl_input = self.carl_input;
        if let Some(acting) = self.acting.as_mut() {
            acting.step(
                scene,
                combat,
                &carl_input,
                carl_legacy,
                carl_loco,
                donut_loco,
            );
        }
    }

    /// Carl's CF02 frame (stomp, air kick, dive, bail-out, heavy landing,
    /// flinch, frustration) in the CF02 Carl atlas, or `None`.
    pub fn carl_cf02_pose(&self) -> Option<ArtPose> {
        self.acting.as_ref()?.carl_pose()
    }

    /// Donut's startle or smug frame in the CF02 Donut atlas, or `None`.
    pub fn donut_cf02_pose(&self) -> Option<ArtPose> {
        self.acting.as_ref()?.donut_pose()
    }

    /// Goblin `index`'s acting frame (hesitate, panic, taunt) in its variant's
    /// CF02 atlas, or `None`.
    pub fn goblin_cf02_pose(&self, index: usize) -> Option<cf02::GoblinActPose> {
        self.acting.as_ref()?.goblin_pose(index)
    }

    /// Goblin `index`'s slammed finish frame in its variant's slammed atlas when
    /// the death presenter chose `slam` for it.
    pub fn goblin_slammed_pose(
        &self,
        index: usize,
        slam: Option<crate::death::SlamPose>,
    ) -> Option<ArtPose> {
        self.acting
            .as_ref()?
            .slammed_pose(index, goblin_variant(index), slam)
    }

    /// ENG-02: Engineer `index`'s slammed finish frame in the Engineer's own slammed
    /// atlas when the death presenter chose `slam` for it.
    pub fn engineer_slammed_pose(
        &self,
        index: usize,
        slam: Option<crate::death::SlamPose>,
    ) -> Option<ArtPose> {
        self.acting.as_ref()?.engineer_slammed_pose(index, slam)
    }

    /// Draw the fixture's Bad Llamas from `clips` (LLAMA-01).
    pub fn with_llama(mut self, clips: SpriteClips, combat: &CombatState) -> Self {
        self.llama = Some(clips);
        self.reset(combat);
        self
    }

    /// Back to the start of an attempt (the goblins were re-created).
    pub fn reset(&mut self, combat: &CombatState) {
        if let Some(acting) = self.acting.as_mut() {
            acting.reset(combat);
        }
        self.carl_input = CarlInput::default();
        self.throw_pose = None;
        self.prev_hero_stop = 0;
        self.donut_hurt = donut_art::HurtInput::default();
        self.prev_donut_stop = 0;
        self.tracks = combat
            .enemies
            .iter()
            .map(|g| GoblinTrack {
                prev_x: g.x,
                prev_hit_stop: 0,
                strike_dx: 0,
                strike_set: 0,
                ease_left: 0,
                overlap: None,
                stack_dx: 0,
                walk_milli_px: 0,
                input: GoblinInput {
                    mode: g.mode,
                    attack_tick: g.attack_tick,
                    frozen: false,
                    down_ticks: g.down_ticks,
                    vy: g.vy,
                    flight_ticks: g.flight_ticks,
                    contact: g.contact_ticks > 0,
                    walking: false,
                    walk_milli_px: 0,
                    idle_tick: 0,
                    engineer: None,
                },
            })
            .collect();
        for (index, track) in self.tracks.iter_mut().enumerate() {
            let g = &combat.enemies[index];
            track.input.engineer =
                g.eng_tuning
                    .filter(|_| g.is_engineer())
                    .map(|t| EngineerFrame {
                        phase: g.engineer_phase(),
                        bonk_age: g.bonk_age(),
                        bonk_hurt_ticks: t.engq13_hurt_ticks,
                        lying_ticks: combat.cf02().map_or(DOWNED_TICKS, |c| c.q04_downed_ticks),
                        ram_age: g.ram_age(),
                    });
        }
        self.llama_tracks = combat
            .llamas
            .iter()
            .map(|l| LlamaTrack {
                prev_x: l.x,
                walk_milli_px: 0,
                input: LlamaInput {
                    mode: l.mode,
                    attack_tick: l.attack_tick,
                    rush: l.attack_kind() == crate::llama::AttackKind::Rush,
                    down_ticks: l.down_ticks,
                    walking: false,
                    walk_milli_px: 0,
                    idle_tick: 0,
                },
            })
            .collect();
    }

    /// Call once after every simulation step of the fixture.
    pub fn step(&mut self, scene: &SceneSim, combat: &CombatState) {
        let hero = &scene.world.hero;
        let landed_age = if combat.carl_defeated() && hero.grounded {
            Some(self.carl_input.landed_age.map_or(0, |a| a + 1))
        } else {
            None
        };
        self.carl_input = CarlInput {
            melee: hero.melee.current().map(|m| (m.spec.id, m.tick)),
            hurt_ticks: combat.hurt_ticks,
            frozen: combat.hero_hit_stop > 0 || self.prev_hero_stop > 0,
            defeated: combat.carl_defeated(),
            grounded: hero.grounded,
            vy: hero.vy,
            landed_age,
        };
        self.prev_hero_stop = combat.hero_hit_stop;
        // Donut's reaction frames (a live party only: otherwise she is never hurt).
        if scene.party_live() {
            let donut = &scene.world.donut_body;
            let defeated = combat.hero_downed(crate::party::HeroId::Donut);
            let landed_age = if defeated && donut.grounded {
                Some(self.donut_hurt.landed_age.map_or(0, |a| a + 1))
            } else {
                None
            };
            self.donut_hurt = donut_art::HurtInput {
                hurt_ticks: combat.donut.hurt_ticks,
                hurt_total: scene.drive().tuning.donut_hurt_ticks,
                frozen: combat.donut.hit_stop > 0 || self.prev_donut_stop > 0,
                defeated,
                grounded: donut.grounded,
                landed_age,
            };
            self.prev_donut_stop = combat.donut.hit_stop;
        } else {
            self.donut_hurt = donut_art::HurtInput::default();
            self.prev_donut_stop = 0;
        }
        let carl_pose = carl_pose(&self.carl, &self.carl_input);
        let carl_foot_y = hero.y - HERO_HALF_HEIGHT;
        let tick = scene.world.tick;
        for index in 0..self.tracks.len() {
            let goblin = &combat.enemies[index];
            let engineer = self.engineer_frame(goblin, combat);
            let clips = match (engineer, self.engineer.as_ref()) {
                (Some(_), Some(clips)) => clips,
                _ => &self.goblins[goblin_variant(index)],
            };
            let track = &mut self.tracks[index];
            let moved = (goblin.x - track.prev_x).abs();
            track.prev_x = goblin.x;
            let prev_hit_stop = std::mem::replace(&mut track.prev_hit_stop, goblin.hit_stop);
            let walking = goblin.mode == GoblinMode::Neutral && goblin.vx == 0.0 && moved > 1e-3;
            if walking {
                track.walk_milli_px = track
                    .walk_milli_px
                    .saturating_add((moved * 1000.0).round() as u32);
            } else {
                track.walk_milli_px = 0;
            }
            track.input = GoblinInput {
                mode: goblin.mode,
                attack_tick: goblin.attack_tick,
                frozen: goblin.hit_stop > 0 || prev_hit_stop > 0,
                down_ticks: goblin.down_ticks,
                vy: goblin.vy,
                flight_ticks: goblin.flight_ticks,
                contact: goblin.contact_ticks > 0,
                walking,
                walk_milli_px: track.walk_milli_px + index as u32 * WALK_INDEX_OFFSET_MILLI_PX,
                idle_tick: tick + index as u64 * IDLE_INDEX_OFFSET_TICKS,
                engineer,
            };
            // Hit-stop contact: Carl's striking limb on a goblin frozen by it.
            let striking = track.input.frozen
                && matches!(
                    goblin.mode,
                    GoblinMode::Hurt | GoblinMode::Airborne | GoblinMode::Launched
                )
                && carl_pose.is_some_and(|p| is_contact_pose(&self.carl, &p));
            if let (true, Some(carl_pose)) = (striking, carl_pose) {
                let geometry = StrikeGeometry {
                    carl: &self.carl,
                    carl_pose,
                    carl_x: hero.x,
                    carl_flip: hero.facing < 0,
                    carl_foot_y,
                    goblin: clips,
                    goblin_pose: goblin_pose(clips, &track.input),
                    goblin_x: goblin.x,
                    goblin_flip: goblin.facing < 0,
                    goblin_foot_y: goblin.y - ENEMY_HALF_HEIGHT,
                };
                track.strike_set = geometry.shift_px();
                track.strike_dx = track.strike_set;
                track.ease_left = STRIKE_EASE_TICKS;
                track.overlap = Some(geometry.overlap_px(track.strike_dx));
            } else {
                track.overlap = None;
                if !track.input.frozen && track.ease_left > 0 {
                    track.ease_left -= 1;
                    track.strike_dx = (track.strike_set as f32 * f32::from(track.ease_left)
                        / f32::from(STRIKE_EASE_TICKS))
                    .round() as i32;
                } else {
                    track.ease_left = 0;
                    track.strike_dx = 0;
                }
            }
        }
        for (index, llama) in combat.llamas.iter().enumerate() {
            let track = &mut self.llama_tracks[index];
            let moved = (llama.x - track.prev_x).abs();
            track.prev_x = llama.x;
            let walking = llama.mode == LlamaMode::Neutral && llama.vx == 0.0 && moved > 1e-3;
            if walking {
                track.walk_milli_px = track
                    .walk_milli_px
                    .saturating_add((moved * 1000.0).round() as u32);
            } else {
                track.walk_milli_px = 0;
            }
            track.input = LlamaInput {
                mode: llama.mode,
                attack_tick: llama.attack_tick,
                rush: llama.attack_kind() == crate::llama::AttackKind::Rush,
                down_ticks: llama.down_ticks,
                walking,
                walk_milli_px: track.walk_milli_px,
                idle_tick: tick + index as u64 * IDLE_INDEX_OFFSET_TICKS,
            };
        }
        // Stacked upright goblins: the later one is drawn away from Carl.
        let upright = |g: &crate::goblin::Goblin| {
            g.alive()
                && matches!(
                    g.mode,
                    GoblinMode::Neutral | GoblinMode::Attack | GoblinMode::Hurt
                )
        };
        for index in 0..self.tracks.len() {
            let goblin = &combat.enemies[index];
            let mut nudge = 0.0f32;
            if upright(goblin) {
                for earlier in combat.enemies[..index].iter().filter(|g| upright(g)) {
                    let gap = (goblin.x - earlier.x).abs();
                    nudge = nudge.max((STACK_SPACING_PX - gap).clamp(0.0, STACK_MAX_NUDGE_PX));
                }
            }
            let away = if goblin.x >= hero.x { 1 } else { -1 };
            self.tracks[index].stack_dx = nudge.round() as i32 * away;
        }
    }

    /// Donut's damage-reaction frame in the melee atlas (hurt, thrown, knocked
    /// down), or `None` while no reaction is drawn.
    pub fn donut_hurt_frame(&self) -> Option<usize> {
        donut_art::hurt_frame(&self.donut_hurt)
    }

    pub fn carl_input(&self) -> &CarlInput {
        &self.carl_input
    }

    /// Carl's melee/hurt/knockdown frame, or `None` while the locomotion
    /// animation draws him.
    pub fn carl_pose(&self) -> Option<ArtPose> {
        carl_pose(&self.carl, &self.carl_input)
    }

    /// Pixels to the right of its sim position the goblin is drawn: toward Carl
    /// while he strikes it in hit-stop (eased back after), and away from Carl
    /// when it would stand on another goblin.
    pub fn goblin_draw_dx(&self, index: usize) -> i32 {
        self.tracks[index].strike_dx + self.tracks[index].stack_dx
    }

    /// The part of [`Self::goblin_draw_dx`] that keeps stacked goblins apart.
    pub fn goblin_stack_dx(&self, index: usize) -> i32 {
        self.tracks[index].stack_dx
    }

    /// Striking-limb and goblin pixels that overlap this tick (`None` when no
    /// goblin is in hit-stop contact with Carl's blow).
    pub fn goblin_strike_overlap(&self, index: usize) -> Option<usize> {
        self.tracks[index].overlap
    }

    /// Foot height to draw Carl's combat pose at: the airborne knockdown frame
    /// blends from the pivot rule at the launch to the lowest row by the apex.
    pub fn carl_anchor_foot_y(&self, sim_foot_y: f32) -> f32 {
        match self.carl_pose() {
            Some(pose) => self.carl.anchor_foot_y_blend(
                &pose,
                sim_foot_y,
                knockdown_blend(self.carl_input.vy),
            ),
            None => sim_foot_y,
        }
    }

    pub fn goblin_count(&self) -> usize {
        self.tracks.len()
    }

    pub fn llama_count(&self) -> usize {
        self.llama_tracks.len()
    }

    pub fn llama_clips(&self) -> Option<&SpriteClips> {
        self.llama.as_ref()
    }

    /// The frame drawn for llama `index` (`None` without the llama atlas).
    pub fn llama_pose(&self, index: usize) -> Option<ArtPose> {
        Some(llama_pose(
            self.llama.as_ref()?,
            &self.llama_tracks.get(index)?.input,
        ))
    }

    /// Foot height of a llama's sim body (its floor line), y-up.
    pub fn llama_foot_y(llama: &crate::llama::Llama) -> f32 {
        llama.y - LLAMA_HALF_HEIGHT
    }

    pub fn goblin_input(&self, index: usize) -> &GoblinInput {
        &self.tracks[index].input
    }

    pub fn goblin_clips(&self, index: usize) -> &SpriteClips {
        match (self.tracks[index].input.engineer, self.engineer.as_ref()) {
            (Some(_), Some(clips)) => clips,
            _ => &self.goblins[goblin_variant(index)],
        }
    }

    /// The atlas label of goblin `index` in the state lines: `A`, `B`, or `E` for
    /// the Engineer.
    fn atlas_label(&self, index: usize) -> &'static str {
        if self.tracks[index].input.engineer.is_some() {
            "E"
        } else {
            ["A", "B"][goblin_variant(index)]
        }
    }

    pub fn goblin_pose(&self, index: usize) -> ArtPose {
        goblin_pose(self.goblin_clips(index), &self.tracks[index].input)
    }

    /// Carl is mid-move in his active window (he is drawn above his target).
    pub fn carl_active(scene: &SceneSim) -> bool {
        scene
            .world
            .hero
            .melee
            .current()
            .is_some_and(|m| m.phase == crate::combat::Phase::Active)
    }

    /// The line CI compares between the headless replay and the render run.
    pub fn state_line(&self, tick: u64) -> String {
        let carl = self
            .carl_pose()
            .map_or_else(|| "-".to_owned(), |p| p.label());
        let goblins: Vec<String> = (0..self.tracks.len())
            .map(|i| {
                format!(
                    "g{i}={}/{}",
                    self.atlas_label(i),
                    self.goblin_pose(i).label()
                )
            })
            .collect();
        let llamas: String = (0..self.llama_tracks.len())
            .filter_map(|i| self.llama_pose(i).map(|p| format!(" l{i}={}", p.label())))
            .collect();
        let acting = self
            .acting
            .as_ref()
            .map_or_else(String::new, cf02::ActingAnimators::state_suffix);
        let throw = self
            .throw_pose
            .map_or_else(String::new, |p| format!(" throw={}", p.label()));
        format!(
            "COMBAT_ANIM_STATE tick={tick} carl={carl} {}{llamas}{acting}{throw}",
            goblins.join(" ")
        )
    }

    /// One per-tick row of the committed anim trace: the sim values the frame
    /// depends on, then what is drawn.
    pub fn trace_line(&self, scene: &SceneSim, combat: &CombatState) -> String {
        let hero = &scene.world.hero;
        let carl_move = match hero.melee.current() {
            Some(m) => format!(
                "{}:{}{}",
                m.spec.id.name(),
                m.tick,
                if m.frozen { "!" } else { "" }
            ),
            None => "-".to_owned(),
        };
        let goblins: Vec<String> = combat
            .enemies
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let phase = g
                    .attack_phase()
                    .map_or_else(String::new, |p| format!("/{p:?}"));
                let overlap = self
                    .goblin_strike_overlap(i)
                    .map_or_else(|| "-".to_owned(), |n| n.to_string());
                format!(
                    "g{i}={:?}{phase} g{i}_t={} g{i}_hs={} g{i}_y={:.2} g{i}_dx={} g{i}_ovl={overlap} g{i}_draw={}/{}",
                    g.mode,
                    g.attack_tick,
                    g.hit_stop,
                    g.y - ENEMY_HALF_HEIGHT,
                    self.goblin_draw_dx(i),
                    self.atlas_label(i),
                    self.goblin_pose(i).label()
                )
            })
            .collect();
        let carl = self
            .carl_pose()
            .map_or_else(|| "-".to_owned(), |p| p.label());
        format!(
            "COMBAT_ANIM_TRACE tick={} carl_move={carl_move} hurt={} carl_stop={} carl_grounded={} carl_draw={carl} {}",
            scene.world.tick,
            combat.hurt_ticks,
            combat.hero_hit_stop,
            u8::from(hero.grounded),
            goblins.join(" ")
        )
    }
}

/// Whether Carl is drawn above goblin `goblin_swinging`'s sprite: always, except
/// that a goblin in its smear or impact is drawn over Carl (the club lands on
/// him) when Carl is not in his own active window. During Carl's active frames
/// he is above his target (the melee spec's integration rule: his fist overlaps
/// the goblin's head by up to 6 px).
pub fn carl_above_goblin(carl_active: bool, goblin_swinging: bool) -> bool {
    carl_active || !goblin_swinging
}

pub mod cf02;
pub mod dynamite;
/// LLAMA-03: which rush-atlas frame a rushing llama draws.
pub mod llama_rush_pose;
/// CORRAL-E (CRL-Q38): the code-drawn rush stand-in.
pub mod rush_stand_in;

#[cfg(test)]
mod tests;
