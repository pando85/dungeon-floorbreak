//! DEATH-C: the gore preset, the presentation RNG and the goblin death art.
//!
//! Pure, Bevy-free **presentation** code. Nothing here is read by the
//! simulation, stored in simulation state, mixed into a replay hash or carried
//! by an event: the preset only decides which cosmetic pieces are drawn and how
//! the gore layer of the art is shown.
//!
//! * [`GorePreset`] is the 1 to 4 setting. **4 (full) is the default** (owner
//!   decision 2026-11-18, "the preset level must be 4, even we want more
//!   intensity"); 1 to 3 are reduced and accessibility settings.
//! * [`PresentationRng`] is the only randomness the presenter uses. It is
//!   seeded from the defeat event (`attempt`, `tick`, `target`) and never
//!   touches any simulation seed.
//! * [`DeathArt`] reads the GOB-02-DEATH manifests and pixel-grid sources
//!   (poses with anchors, parts with spawn metadata) and composes the sprite
//!   sheets per preset: the `gore` layer hidden at preset 1, recoloured at 2,
//!   shown at 3 and 4 (spec `docs/02-art/gob-02-death-spec.md` section 5).
//!   Nothing is redrawn per preset: it is the same sources and four switches.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

/// The presentation intensity. Never part of simulation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum GorePreset {
    /// 1: no blood sprite; the same poses, silhouettes and feedback.
    Off,
    /// 2: the #79 `reduced`: muted blood, few pieces, no gibs.
    Reduced,
    /// 3: a selectable intermediate (half the drops and gibs, no bone, no wall
    /// spatter, pools to stage 2).
    Medium,
    /// 4: full, the shipping default.
    #[default]
    Full,
}

impl GorePreset {
    pub const ALL: [Self; 4] = [Self::Off, Self::Reduced, Self::Medium, Self::Full];

    pub const fn level(self) -> u8 {
        match self {
            Self::Off => 1,
            Self::Reduced => 2,
            Self::Medium => 3,
            Self::Full => 4,
        }
    }

    pub fn from_level(level: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.level() == level)
    }

    /// `1` to `4`, or `off`, `reduced`, `medium`, `full`.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "off" => Some(Self::Off),
            "reduced" => Some(Self::Reduced),
            "medium" => Some(Self::Medium),
            "full" => Some(Self::Full),
            other => other.parse().ok().and_then(Self::from_level),
        }
    }

    /// Whether the `gore` layer of the art is drawn at all.
    pub const fn shows_gore_layer(self) -> bool {
        !matches!(self, Self::Off)
    }

    /// Whether the gore layer is recoloured to the dark `B` blood (`R` and `P`
    /// become `B`): the reduced setting.
    pub const fn mutes_gore(self) -> bool {
        matches!(self, Self::Reduced)
    }

    /// Quarters of the base count kept for blood drops: 1.0, 0.5, 0.25, 0.
    pub const fn drop_quarters(self) -> u32 {
        match self {
            Self::Full => 4,
            Self::Medium => 2,
            Self::Reduced => 1,
            Self::Off => 0,
        }
    }

    /// Quarters of the base count kept for gibs: 1.0, 0.5, 0 (no gibs at 2), 0.
    pub const fn gib_quarters(self) -> u32 {
        match self {
            Self::Full => 4,
            Self::Medium => 2,
            Self::Reduced | Self::Off => 0,
        }
    }
}

/// `ceil(count * quarters / 4)`: a count scaled by a preset, never rounded to
/// zero while the preset keeps anything.
pub const fn scale_count(count: u32, quarters: u32) -> u32 {
    (count * quarters).div_ceil(4)
}

/// Whether item `k` (0-based) of a stream of items survives a `quarters / 4`
/// scale: exactly `floor(n * quarters / 4)` of any `n` items are kept, spread
/// evenly.
pub const fn keeps_stream_item(k: u32, quarters: u32) -> bool {
    ((k + 1) * quarters) / 4 > (k * quarters) / 4
}

// --------------------------------------------------------- cf02 budget ----

/// CF02-D (COMBAT-FUN-02 Q22, Q36, Q43): the gore budget of the cf02 rules, as
/// pure functions of the preset and the pose or force. Built from the tuning
/// only when `cf02_rules` and `q36_gore_budget` are on, so a run without them
/// never holds one and every legacy count stays as it is. Presentation only.
///
/// * **Q22** (preset 4 only): heavy and catastrophic finishes use
///   `q22_preset4_gib_quarters` (6, i.e. 1.5x of 4) and may separate
///   `q22_preset4_extra_separated_parts` more part where the anatomy allows.
///   Drop quarters, corpse and decal caps and presets 1 to 3 are unchanged.
/// * **Q43**: the slammed goblin poses carry their own base (quarters and
///   separated parts at preset 4) that overrides Q22; presets 1 to 3 scale by
///   the existing ratio (`gib_quarters / 4`, rounded up, never to zero while the
///   preset keeps anything). Routine blunt kills add `q43_routine_spray_count`
///   transient sprays at preset 4, each gone after `q43_routine_spray_ticks`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cf02GoreBudget {
    q22_quarters: u32,
    q22_extra_parts: u32,
    stomped: (u32, u32),
    wall_crumple: (u32, u32),
    flung: (u32, u32),
    spray_count: u32,
    spray_ticks: u64,
}

impl Cf02GoreBudget {
    /// `None` unless `cf02_rules` and the Q36 gore-budget switch are on.
    pub fn from_tuning(t: &crate::cf02::Cf02Tuning) -> Option<Self> {
        if !t.on(t.q36_gore_budget) {
            return None;
        }
        Some(Self {
            q22_quarters: u32::from(t.q22_preset4_gib_quarters),
            q22_extra_parts: u32::from(t.q22_preset4_extra_separated_parts),
            stomped: (
                u32::from(t.q43_stomped_gib_quarters),
                u32::from(t.q43_stomped_separated_parts),
            ),
            wall_crumple: (
                u32::from(t.q43_wall_crumple_gib_quarters),
                u32::from(t.q43_wall_crumple_separated_parts),
            ),
            flung: (
                u32::from(t.q43_flung_gib_quarters),
                u32::from(t.q43_flung_separated_parts),
            ),
            spray_count: u32::from(t.q43_routine_spray_count),
            spray_ticks: u64::from(t.q43_routine_spray_ticks),
        })
    }

    /// Gib quarters of a finish of `force`: Q22's at preset 4 for heavy and
    /// catastrophic, the preset's own otherwise.
    pub fn gib_quarters(&self, preset: GorePreset, force: crate::combat::ForceTier) -> u32 {
        if preset == GorePreset::Full && force >= crate::combat::ForceTier::Heavy {
            self.q22_quarters
        } else {
            preset.gib_quarters()
        }
    }

    /// Extra separated parts of a heavy or catastrophic finish (Q22, preset 4).
    pub fn extra_parts(&self, preset: GorePreset, force: crate::combat::ForceTier) -> u32 {
        if preset == GorePreset::Full && force >= crate::combat::ForceTier::Heavy {
            self.q22_extra_parts
        } else {
            0
        }
    }

    fn pose_base(&self, pose: crate::death::SlamPose) -> (u32, u32) {
        use crate::death::SlamPose;
        match pose {
            SlamPose::Stomped => self.stomped,
            SlamPose::WallCrumple => self.wall_crumple,
            SlamPose::Flung => self.flung,
        }
    }

    /// Gib quarters of a slammed pose at `preset`: its Q43 base at preset 4,
    /// scaled by the existing ratio below.
    pub fn slam_quarters(&self, preset: GorePreset, pose: crate::death::SlamPose) -> u32 {
        self.pose_base(pose).0 * preset.gib_quarters() / 4
    }

    /// Separated parts of a slammed pose at `preset` (the manifest's preset
    /// filter still decides which part ids may fly).
    pub fn slam_parts(&self, preset: GorePreset, pose: crate::death::SlamPose) -> u32 {
        scale_count(self.pose_base(pose).1, preset.gib_quarters())
    }

    /// Transient sprays of a routine blunt kill: Q43's count at preset 4 only.
    pub fn routine_sprays(&self, preset: GorePreset) -> u32 {
        if preset == GorePreset::Full {
            self.spray_count
        } else {
            0
        }
    }

    /// Lifetime of a transient spray, in presenter ticks.
    pub fn spray_ticks(&self) -> u64 {
        self.spray_ticks
    }
}

// ------------------------------------------------------------------- rng ----

/// Deterministic cosmetic randomness (splitmix64). It is seeded from the
/// defeat event only; it never reads or advances a simulation seed, and the
/// simulation never reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationRng {
    state: u64,
}

impl PresentationRng {
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The seed of one defeat: `hash(attempt, tick, target)` (the proposal's
    /// `hash(run_seed, event_id)` where the event id is the unique
    /// (attempt, tick, target) triple).
    pub fn for_event(attempt: u32, tick: u64, target: u32) -> Self {
        let mut mixed = 0x9E37_79B9_7F4A_7C15u64;
        for word in [u64::from(attempt), tick, u64::from(target)] {
            mixed ^= word.wrapping_add(0x9E37_79B9_7F4A_7C15);
            mixed = mixed.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            mixed ^= mixed >> 31;
        }
        Self::new(mixed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Uniform in `[lo, hi]` (`lo <= hi`).
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }

    /// Uniform integer in `lo..=hi`.
    pub fn int(&mut self, lo: u32, hi: u32) -> u32 {
        lo + (self.next_u64() % u64::from(hi - lo + 1)) as u32
    }

    pub fn coin(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ------------------------------------------------------------- art data -----

/// The class of a part (spec section 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartClass {
    Wound,
    Clean,
    Gib,
    Blood,
    /// LLAMA-02: the ruptured spit gland and its lava (drops, decals): the
    /// species signature, shown at every preset (`molten` layer).
    Molten,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mass {
    Light,
    Medium,
    Heavy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpinClass {
    None,
    Slow,
    Fast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Settle {
    Roll,
    Flop,
    Clatter,
    Flutter,
    Bounce,
    Static,
}

/// How long a settled piece stays: until the room resets (or the pool recycles
/// it, oldest first), about 6 s then a 30 tick fade, or about 2 s then 12.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ttl {
    Keep,
    Long,
    Short,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PartMeta {
    pub id: String,
    pub frame: usize,
    pub anchors: BTreeMap<String, [i32; 2]>,
    pub frame0_origin: Option<[i32; 2]>,
    pub class: PartClass,
    pub presets: Vec<u8>,
    pub gore_layer: bool,
    pub mass: Mass,
    pub spin: SpinClass,
    pub settle: Settle,
    pub bounce: f32,
    #[serde(default)]
    pub swap_on_first_contact: Option<String>,
    #[serde(default)]
    pub stain: Option<String>,
    #[serde(default)]
    pub trail: Option<String>,
    #[serde(default)]
    pub on_contact: Option<String>,
    pub ttl: Ttl,
    #[serde(default)]
    pub floor_decal: bool,
    /// The other half of a two-cell decal (`decal_pool_3` and `_r`).
    #[serde(default)]
    pub pair: Option<String>,
    /// Where the right cell's origin sits relative to the left cell's.
    #[serde(default)]
    pub pair_offset: Option<[i32; 2]>,
}

impl PartMeta {
    /// Whether the preset spawns this part (the manifest class filter).
    pub fn allowed_at(&self, preset: GorePreset) -> bool {
        self.presets.contains(&preset.level())
    }

    pub fn anchor(&self, name: &str) -> Option<(i32, i32)> {
        self.anchors.get(name).map(|a| (a[0], a[1]))
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PoseMeta {
    pub index: usize,
    pub tag: String,
    pub ticks: Option<u32>,
    pub anchors: BTreeMap<String, [i32; 2]>,
    /// LLAMA-02 gate ruling: `"over"` draws the falling block directly above
    /// this crushed enemy (below every player actor) on this frame; absent
    /// keeps the goblin order (the block behind). Read from the manifest, never
    /// decided by species code.
    #[serde(default)]
    pub hazard_draw: Option<String>,
}

impl PoseMeta {
    pub fn anchor(&self, name: &str) -> Option<(i32, i32)> {
        self.anchors.get(name).map(|a| (a[0], a[1]))
    }

    /// Whether the falling block is drawn over this pose (`hazard_draw: "over"`).
    pub fn hazard_over(&self) -> bool {
        self.hazard_draw.as_deref() == Some("over")
    }
}

#[derive(Deserialize)]
struct PartsSection {
    cell: [i32; 2],
    parts: Vec<PartMeta>,
}

#[derive(Deserialize)]
struct PosesSection {
    frames: Vec<PoseMeta>,
}

#[derive(Deserialize)]
struct Manifest<T> {
    grid_source: String,
    frame_size: [i32; 2],
    pivot: [i32; 2],
    #[serde(alias = "death_parts", alias = "death_poses")]
    section: T,
}

#[derive(Debug, Deserialize)]
struct PaletteEntry {
    key: String,
    rgba: String,
}

#[derive(Debug, Deserialize)]
struct GridFrame {
    cels: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct GridFile {
    canvas: [i32; 2],
    layers: Vec<String>,
    palette: Vec<PaletteEntry>,
    frames: Vec<GridFrame>,
}

/// Opaque extent of one cell in cell pixels (`right` and `bottom` exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bbox {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Bbox {
    pub fn width(self) -> i32 {
        self.right - self.left
    }

    pub fn height(self) -> i32 {
        self.bottom - self.top
    }
}

/// A row strip of RGBA8 frames, composed for one preset.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub cell_w: i32,
    pub cell_h: i32,
    pub frames: usize,
    /// `cell_w * frames` by `cell_h`, row-major RGBA8.
    pub rgba: Vec<u8>,
    /// Opaque extent per frame (`None` for an empty frame).
    pub bbox: Vec<Option<Bbox>>,
    /// Convex hull of the opaque pixel corners per frame, as offsets from the
    /// cell centre (x right, y up): what the floor clamp and the resting
    /// orientation use, so a thin diagonal piece lies on its real pixels.
    pub hull: Vec<Vec<(f32, f32)>>,
    /// Opaque pixels per frame.
    pub opaque: Vec<u32>,
}

impl Sheet {
    pub fn width(&self) -> usize {
        self.cell_w as usize * self.frames
    }

    pub fn pixel(&self, frame: usize, x: i32, y: i32) -> [u8; 4] {
        let at = (y as usize * self.width() + frame * self.cell_w as usize + x as usize) * 4;
        [
            self.rgba[at],
            self.rgba[at + 1],
            self.rgba[at + 2],
            self.rgba[at + 3],
        ]
    }

    /// Every colour used by opaque pixels of `frame`.
    pub fn colours(&self, frame: usize) -> Vec<[u8; 4]> {
        let mut out = Vec::new();
        for y in 0..self.cell_h {
            for x in 0..self.cell_w {
                let p = self.pixel(frame, x, y);
                if p[3] != 0 && !out.contains(&p) {
                    out.push(p);
                }
            }
        }
        out
    }
}

/// The composed sheets of one preset.
#[derive(Debug, Clone, PartialEq)]
pub struct PresetSheets {
    pub preset: GorePreset,
    pub poses: Sheet,
    pub parts: Sheet,
}

fn parse_rgba(text: &str) -> Result<[u8; 4], String> {
    let hex = text.trim_start_matches('#');
    if hex.len() != 8 {
        return Err(format!("bad colour {text:?}"));
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| format!("{e}"));
    Ok([byte(0)?, byte(2)?, byte(4)?, byte(6)?])
}

/// Convex hull of integer points (Andrew's monotone chain), counter-clockwise,
/// without repeated points.
fn convex_hull(mut points: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    points.sort_unstable();
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let cross = |o: (i32, i32), a: (i32, i32), b: (i32, i32)| {
        (a.0 - o.0) as i64 * (b.1 - o.1) as i64 - (a.1 - o.1) as i64 * (b.0 - o.0) as i64
    };
    let mut hull: Vec<(i32, i32)> = Vec::new();
    for pass in 0..2 {
        let start = hull.len();
        let iter: Box<dyn Iterator<Item = &(i32, i32)>> = if pass == 0 {
            Box::new(points.iter())
        } else {
            Box::new(points.iter().rev())
        };
        for &p in iter {
            while hull.len() >= start + 2
                && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0
            {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
    }
    hull
}

/// The gore layer is the last layer of the art (spec section 2: `body`,
/// `weapon`, `gore`, bottom to top).
const GORE_LAYER: &str = "gore";

impl GridFile {
    fn compose(&self, preset: GorePreset) -> Result<Sheet, String> {
        let colours: BTreeMap<char, [u8; 4]> = self
            .palette
            .iter()
            .map(|p| {
                let key = p.key.chars().next().ok_or("empty palette key")?;
                Ok((key, parse_rgba(&p.rgba)?))
            })
            .collect::<Result<_, String>>()?;
        let (w, h) = (self.canvas[0], self.canvas[1]);
        let frames = self.frames.len();
        let width = w as usize * frames;
        let mut rgba = vec![0u8; width * h as usize * 4];
        let mut bbox = Vec::with_capacity(frames);
        let mut hull = Vec::with_capacity(frames);
        let mut opaque = Vec::with_capacity(frames);
        for (index, frame) in self.frames.iter().enumerate() {
            let mut count = 0;
            let mut extent: Option<Bbox> = None;
            let mut corners: Vec<(i32, i32)> = Vec::new();
            for layer in &self.layers {
                if layer == GORE_LAYER && !preset.shows_gore_layer() {
                    continue;
                }
                let Some(rows) = frame.cels.get(layer) else {
                    continue;
                };
                for (y, row) in rows.iter().enumerate() {
                    for (x, key) in row.chars().enumerate() {
                        if key == '.' {
                            continue;
                        }
                        // Reduced: the wet and fresh blood keys become the
                        // dark clotted key, on the gore layer only.
                        let key = if layer == GORE_LAYER
                            && preset.mutes_gore()
                            && matches!(key, 'R' | 'P')
                        {
                            'B'
                        } else {
                            key
                        };
                        let colour = colours
                            .get(&key)
                            .ok_or_else(|| format!("frame {index}: unknown palette key {key:?}"))?;
                        let at = (y * width + index * w as usize + x) * 4;
                        rgba[at..at + 4].copy_from_slice(colour);
                    }
                }
            }
            for y in 0..h {
                for x in 0..w {
                    let at = (y as usize * width + index * w as usize + x as usize) * 4;
                    if rgba[at + 3] != 0 {
                        count += 1;
                        corners.extend([(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)]);
                        extent = Some(match extent {
                            None => Bbox {
                                left: x,
                                top: y,
                                right: x + 1,
                                bottom: y + 1,
                            },
                            Some(b) => Bbox {
                                left: b.left.min(x),
                                top: b.top.min(y),
                                right: b.right.max(x + 1),
                                bottom: b.bottom.max(y + 1),
                            },
                        });
                    }
                }
            }
            bbox.push(extent);
            hull.push(
                convex_hull(corners)
                    .into_iter()
                    .map(|(x, y)| (x as f32 - w as f32 / 2.0, h as f32 / 2.0 - y as f32))
                    .collect(),
            );
            opaque.push(count);
        }
        Ok(Sheet {
            cell_w: w,
            cell_h: h,
            frames,
            rgba,
            bbox,
            hull,
            opaque,
        })
    }
}

/// The GOB-02-DEATH art data: pose anchors, part spawn metadata and the two
/// pixel-grid sources the sheets are composed from.
#[derive(Debug)]
pub struct DeathArt {
    /// 0 = goblin variant A, 1 = variant B.
    pub variant: usize,
    pub pose_frame_size: (i32, i32),
    pub pose_pivot: (i32, i32),
    pub poses: Vec<PoseMeta>,
    pub part_cell: (i32, i32),
    pub parts: Vec<PartMeta>,
    poses_grid: GridFile,
    parts_grid: GridFile,
}

/// Goblin variants that have death art (A and B, index parity as the GOB-01
/// atlases in `combat_anim::goblin_variant`).
pub const DEATH_VARIANTS: usize = 2;
/// The poses and parts manifests of each variant (A, B). B shares A's contract
/// (frames, pivots, anchors, part ids) and adds an `accessory` layer.
pub const POSES_MANIFESTS: [&str; DEATH_VARIANTS] = [
    "assets/manifests/goblin_a_death_h36.json",
    "assets/manifests/goblin_b_death_h36.json",
];
pub const PARTS_MANIFESTS: [&str; DEATH_VARIANTS] = [
    "assets/manifests/goblin_a_death_parts.json",
    "assets/manifests/goblin_b_death_parts.json",
];
pub const POSES_MANIFEST: &str = POSES_MANIFESTS[0];
pub const PARTS_MANIFEST: &str = PARTS_MANIFESTS[0];
/// LLAMA-02-DEATH (DEMO-01 PR B): the Bad Llama's poses and parts, in the same
/// manifest schema and read by the same code path. Its art `variant` index is
/// [`LLAMA_VARIANT`], after the goblin variants; it adds a `molten` layer that
/// is shown at every preset (`always_shown_layers`).
pub const LLAMA_POSES_MANIFEST: &str = "assets/manifests/llama_death_h64.json";
pub const LLAMA_PARTS_MANIFEST: &str = "assets/manifests/llama_death_parts.json";
pub const LLAMA_VARIANT: usize = DEATH_VARIANTS;

impl DeathArt {
    /// Read both manifests and the grid sources they name, below `root` (the
    /// repository root).
    /// The variant A art (the GOB-02-DEATH set).
    pub fn load(root: &Path) -> Result<Self, String> {
        Self::load_variant(root, 0)
    }

    /// The death art of goblin `variant` (0 = A, 1 = B).
    pub fn load_variant(root: &Path, variant: usize) -> Result<Self, String> {
        let (Some(poses_path), Some(parts_path)) =
            (POSES_MANIFESTS.get(variant), PARTS_MANIFESTS.get(variant))
        else {
            return Err(format!("no goblin death variant {variant}"));
        };
        Self::load_manifests(root, variant, poses_path, parts_path)
    }

    /// The Bad Llama's death art (LLAMA-02-DEATH): poses with the `hazard_draw`
    /// flag, parts with the `molten` class, composed by the same code as the
    /// goblins' (the `molten` layer is never hidden by the preset).
    pub fn load_llama(root: &Path) -> Result<Self, String> {
        Self::load_manifests(
            root,
            LLAMA_VARIANT,
            LLAMA_POSES_MANIFEST,
            LLAMA_PARTS_MANIFEST,
        )
    }

    fn load_manifests(
        root: &Path,
        variant: usize,
        poses_path: &str,
        parts_path: &str,
    ) -> Result<Self, String> {
        let read = |path: &str| {
            std::fs::read_to_string(root.join(path)).map_err(|e| format!("{path}: {e}"))
        };
        let poses: Manifest<PosesSection> =
            serde_json::from_str(&read(poses_path)?).map_err(|e| format!("{poses_path}: {e}"))?;
        let parts: Manifest<PartsSection> =
            serde_json::from_str(&read(parts_path)?).map_err(|e| format!("{parts_path}: {e}"))?;
        let grid = |path: &str| -> Result<GridFile, String> {
            serde_json::from_str(&read(path)?).map_err(|e| format!("{path}: {e}"))
        };
        let art = Self {
            variant,
            pose_frame_size: (poses.frame_size[0], poses.frame_size[1]),
            pose_pivot: (poses.pivot[0], poses.pivot[1]),
            poses: poses.section.frames,
            part_cell: (parts.section.cell[0], parts.section.cell[1]),
            parts: parts.section.parts,
            poses_grid: grid(&poses.grid_source)?,
            parts_grid: grid(&parts.grid_source)?,
        };
        art.validate()?;
        Ok(art)
    }

    fn validate(&self) -> Result<(), String> {
        if self.poses_grid.canvas != [self.pose_frame_size.0, self.pose_frame_size.1]
            || self.poses_grid.frames.len() != self.poses.len()
        {
            return Err("poses manifest and grid disagree".into());
        }
        if self.parts_grid.canvas != [self.part_cell.0, self.part_cell.1]
            || self.parts_grid.frames.len() != self.parts.len()
        {
            return Err("parts manifest and grid disagree".into());
        }
        for (index, part) in self.parts.iter().enumerate() {
            if part.frame != index {
                return Err(format!("part {} is not at its frame index", part.id));
            }
        }
        for (index, pose) in self.poses.iter().enumerate() {
            if pose.index != index {
                return Err(format!("pose {} is not at its frame index", pose.tag));
            }
        }
        Ok(())
    }

    /// Whether `other` has the same contract as this art: frame and cell sizes,
    /// pivots, pose tags and anchors, part ids, indices, anchors, classes and
    /// preset filters. The presenter reads the metadata of variant A for every
    /// variant, so a variant that differs would be drawn wrongly.
    pub fn same_contract(&self, other: &Self) -> Result<(), String> {
        if self.pose_frame_size != other.pose_frame_size
            || self.pose_pivot != other.pose_pivot
            || self.part_cell != other.part_cell
        {
            return Err("frame size, pivot or part cell differs".into());
        }
        if self.poses != other.poses {
            return Err("pose tags, ticks or anchors differ".into());
        }
        if self.parts != other.parts {
            return Err("part ids, anchors, classes or preset filters differ".into());
        }
        Ok(())
    }

    pub fn part_index(&self, id: &str) -> Option<usize> {
        self.parts.iter().position(|p| p.id == id)
    }

    /// The pose frame index by position in the manifest (`crush_contact` has
    /// two frames: 0 and 1).
    pub fn pose(&self, index: usize) -> &PoseMeta {
        &self.poses[index]
    }

    /// Compose both sheets for `preset`.
    pub fn compose(&self, preset: GorePreset) -> Result<PresetSheets, String> {
        Ok(PresetSheets {
            preset,
            poses: self.poses_grid.compose(preset)?,
            parts: self.parts_grid.compose(preset)?,
        })
    }
}

// -------------------------------------------------------------- geometry ----

/// World position (y-up px) of the centre of anchor pixel `(ax, ay)` of pose
/// frame pixels, for a pose standing with its pivot's left edge at `foot_x` and
/// the bottom edge of its pivot row on `ground_y`. A flipped pose mirrors
/// around `foot_x` (the runtime rule of the art spec, anchors included).
pub fn pose_anchor_world(
    pivot: (i32, i32),
    anchor: (i32, i32),
    foot_x: f32,
    ground_y: f32,
    flip: bool,
) -> (f32, f32) {
    let dx = anchor.0 as f32 + 0.5 - pivot.0 as f32;
    let x = if flip { foot_x - dx } else { foot_x + dx };
    let y = ground_y + (pivot.1 - anchor.1) as f32 + 0.5;
    (x, y)
}

/// Centre of a part sprite (cell pivot, the part's centre of mass) that puts
/// the centre of anchor pixel `anchor` on the world point `at`.
pub fn part_center_for_anchor(
    cell: (i32, i32),
    anchor: (i32, i32),
    at: (f32, f32),
    flip: bool,
) -> (f32, f32) {
    let ox = anchor.0 as f32 + 0.5 - cell.0 as f32 / 2.0;
    let oy = -(anchor.1 as f32 + 0.5 - cell.1 as f32 / 2.0);
    let ox = if flip { -ox } else { ox };
    (at.0 - ox, at.1 - oy)
}

/// Lowest point of a part's opaque pixels (its `hull`), as an offset below the
/// sprite centre (never negative), when the sprite is flipped as given and
/// rotated by `angle_deg` (counter-clockwise, about its centre). This is what
/// the floor clamp uses.
pub fn lowest_below_centre(hull: &[(f32, f32)], flip: bool, angle_deg: f32) -> f32 {
    let (sin, cos) = angle_deg.to_radians().sin_cos();
    hull.iter()
        .map(|&(x, y)| {
            let x = if flip { -x } else { x };
            x * sin + y * cos
        })
        .fold(f32::INFINITY, f32::min)
        .min(0.0)
        .abs()
}

/// Height of the rotated hull (top to bottom, px).
fn rotated_height(hull: &[(f32, f32)], flip: bool, angle_deg: f32) -> f32 {
    let (sin, cos) = angle_deg.to_radians().sin_cos();
    let ys = hull.iter().map(|&(x, y)| {
        let x = if flip { -x } else { x };
        x * sin + y * cos
    });
    let (lo, hi) = ys.fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), v| {
        (lo.min(v), hi.max(v))
    });
    hi - lo
}

/// The angle a piece comes to rest at: the eighth turn (45 degrees) nearest to
/// `angle_deg` among those that lie it lowest. A thin diagonal piece (the
/// club) lies flat instead of leaning on a quarter turn; a flat one keeps
/// its side.
pub fn rest_angle(hull: &[(f32, f32)], flip: bool, angle_deg: f32) -> f32 {
    let base = (angle_deg / 45.0).round() * 45.0;
    let candidates: Vec<f32> = (-4..=4).map(|k| base + k as f32 * 45.0).collect();
    let lowest = candidates
        .iter()
        .map(|a| rotated_height(hull, flip, *a))
        .fold(f32::INFINITY, f32::min);
    candidates
        .into_iter()
        .filter(|a| rotated_height(hull, flip, *a) <= lowest + 0.3)
        .min_by(|a, b| (a - angle_deg).abs().total_cmp(&(b - angle_deg).abs()))
        .unwrap_or(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    pub(crate) fn art() -> DeathArt {
        DeathArt::load(root()).expect("goblin death art")
    }

    #[test]
    fn the_default_preset_is_full_and_parsing_is_strict() {
        assert_eq!(GorePreset::default(), GorePreset::Full);
        assert_eq!(GorePreset::default().level(), 4);
        for (text, preset) in [
            ("1", GorePreset::Off),
            ("2", GorePreset::Reduced),
            ("3", GorePreset::Medium),
            ("4", GorePreset::Full),
            ("off", GorePreset::Off),
            ("full", GorePreset::Full),
        ] {
            assert_eq!(GorePreset::parse(text), Some(preset), "{text}");
        }
        for bad in ["0", "5", "", "max", "-1"] {
            assert_eq!(GorePreset::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn scaling_keeps_the_documented_fractions() {
        // 1.0, 0.5, 0.25, 0 of a base count, rounded up while anything is kept.
        for (preset, n, want) in [
            (GorePreset::Full, 18, 18),
            (GorePreset::Medium, 18, 9),
            (GorePreset::Reduced, 18, 5),
            (GorePreset::Off, 18, 0),
            (GorePreset::Medium, 1, 1),
            (GorePreset::Reduced, 1, 1),
        ] {
            assert_eq!(
                scale_count(n, preset.drop_quarters()),
                want,
                "{preset:?} {n}"
            );
        }
        // No gibs at 2 or 1.
        assert_eq!(scale_count(3, GorePreset::Reduced.gib_quarters()), 0);
        assert_eq!(scale_count(3, GorePreset::Off.gib_quarters()), 0);
        // A stream keeps exactly floor(n * q / 4) of n items.
        for q in 0..=4u32 {
            for n in 0..=40u32 {
                let kept = (0..n).filter(|k| keeps_stream_item(*k, q)).count() as u32;
                assert_eq!(kept, n * q / 4, "q={q} n={n}");
            }
        }
    }

    #[test]
    fn the_presentation_rng_is_deterministic_and_event_seeded() {
        let draw = |mut rng: PresentationRng| (0..8).map(|_| rng.next_u64()).collect::<Vec<_>>();
        assert_eq!(
            draw(PresentationRng::for_event(0, 139, 0)),
            draw(PresentationRng::for_event(0, 139, 0))
        );
        for other in [
            PresentationRng::for_event(1, 139, 0),
            PresentationRng::for_event(0, 140, 0),
            PresentationRng::for_event(0, 139, 1),
        ] {
            assert_ne!(draw(PresentationRng::for_event(0, 139, 0)), draw(other));
        }
        let mut rng = PresentationRng::new(7);
        for _ in 0..1000 {
            let u = rng.unit();
            assert!((0.0..1.0).contains(&u));
            assert!((3..=5).contains(&rng.int(3, 5)));
            assert!((-2.0..=2.0).contains(&rng.range(-2.0, 2.0)));
        }
    }

    #[test]
    fn the_art_loads_and_names_every_part_the_presenter_uses() {
        let art = art();
        assert_eq!(art.pose_frame_size, (64, 56));
        assert_eq!(art.pose_pivot, (28, 55));
        assert_eq!(art.poses.len(), 6);
        assert_eq!(art.parts.len(), 26);
        for id in [
            "head",
            "head_dead",
            "club",
            "gib_flesh_a",
            "gib_flesh_b",
            "gib_bone",
            "gib_rag",
            "drop_blood",
            "decal_splat_s",
            "decal_splat_m",
            "decal_pool_1",
            "decal_pool_2",
            "decal_pool_3",
            "decal_pool_3_r",
            "decal_wall_spatter",
            "decal_wall_spatter_b",
            "decal_smear",
        ] {
            assert!(art.part_index(id).is_some(), "{id}");
        }
        for tag in [
            "hit_origin",
            "hand_near",
            "mouth",
            "squeeze_back",
            "squeeze_front",
        ] {
            assert!(art.pose(0).anchor(tag).is_some(), "{tag}");
        }
        assert!(art.pose(2).anchor("pool_origin").is_some());
        for tag in ["neck", "jet_base"] {
            assert!(art.pose(3).anchor(tag).is_some(), "{tag}");
        }
        assert!(art.pose(5).anchor("pool_origin").is_some());
    }

    #[test]
    fn preset_one_composes_without_a_single_gore_layer_pixel() {
        // Preset 1 is exactly the body and weapon layers: compare against a
        // composition that never reads the gore layer, for every frame.
        let art = art();
        let off = art.compose(GorePreset::Off).unwrap();
        for (grid, ours) in [(&art.poses_grid, &off.poses), (&art.parts_grid, &off.parts)] {
            let stripped = GridFile {
                canvas: grid.canvas,
                layers: grid.layers.clone(),
                palette: grid
                    .palette
                    .iter()
                    .map(|p| PaletteEntry {
                        key: p.key.clone(),
                        rgba: p.rgba.clone(),
                    })
                    .collect(),
                frames: grid
                    .frames
                    .iter()
                    .map(|frame| {
                        let mut cels = frame.cels.clone();
                        cels.remove(GORE_LAYER);
                        GridFrame { cels }
                    })
                    .collect(),
            };
            // The gore layer is absent here, so even the full preset shows
            // only body and weapon.
            let sheet = stripped.compose(GorePreset::Full).unwrap();
            assert_eq!(
                sheet.rgba, ours.rgba,
                "preset 1 differs from body+weapon only"
            );
        }
        // The blood-only keys never appear at preset 1 (they live on gore only).
        let blood = parse_rgba("#6e1a22ff").unwrap();
        let wet = parse_rgba("#e2434bff").unwrap();
        for sheet in [&off.poses, &off.parts] {
            for frame in 0..sheet.frames {
                let colours = sheet.colours(frame);
                assert!(!colours.contains(&blood), "frame {frame} has blood_dark");
                assert!(!colours.contains(&wet), "frame {frame} has blood_wet");
            }
        }
        // And the gore-only parts are empty at preset 1.
        for part in art.parts.iter().filter(|p| p.class == PartClass::Blood) {
            assert_eq!(off.parts.opaque[part.frame], 0, "{} has pixels", part.id);
            assert!(!part.allowed_at(GorePreset::Off), "{}", part.id);
        }
    }

    #[test]
    fn variant_b_loads_with_the_contract_of_variant_a_and_composes_at_every_preset() {
        let a = art();
        let b = DeathArt::load_variant(root(), 1).expect("variant B death art");
        assert_eq!(b.variant, 1);
        assert_eq!(a.variant, 0);
        a.same_contract(&b).expect("one contract");
        assert_eq!((b.poses.len(), b.parts.len()), (6, 26));
        assert!(DeathArt::load_variant(root(), DEATH_VARIANTS).is_err());
        let full = b.compose(GorePreset::Full).unwrap();
        let three = b.compose(GorePreset::Medium).unwrap();
        assert_eq!(
            (&full.poses, &full.parts),
            (&three.poses, &three.parts),
            "3 and 4 share"
        );
        let head = b.part_index("head").unwrap();
        for preset in GorePreset::ALL {
            // The accessory layer is no gore: the head is drawn at every preset.
            let sheets = b.compose(preset).unwrap();
            assert!(sheets.parts.opaque[head] > 0, "{preset:?}");
        }
        // Preset 1 of B is B without its gore layer, pixel for pixel: the
        // accessory (the bandana's red, N2 of the art review, shares a palette
        // key with the blood) stays, no gore-layer pixel does.
        let off = b.compose(GorePreset::Off).unwrap();
        for (grid, ours) in [(&b.poses_grid, &off.poses), (&b.parts_grid, &off.parts)] {
            let stripped = GridFile {
                canvas: grid.canvas,
                layers: grid.layers.clone(),
                palette: grid
                    .palette
                    .iter()
                    .map(|p| PaletteEntry {
                        key: p.key.clone(),
                        rgba: p.rgba.clone(),
                    })
                    .collect(),
                frames: grid
                    .frames
                    .iter()
                    .map(|frame| {
                        let mut cels = frame.cels.clone();
                        cels.remove(GORE_LAYER);
                        GridFrame { cels }
                    })
                    .collect(),
            };
            let sheet = stripped.compose(GorePreset::Full).unwrap();
            assert_eq!(sheet.rgba, ours.rgba, "preset 1 differs from B minus gore");
        }
    }

    #[test]
    fn preset_two_mutes_the_gore_layer_and_preset_three_keeps_it() {
        let art = art();
        let full = art.compose(GorePreset::Full).unwrap();
        let three = art.compose(GorePreset::Medium).unwrap();
        let two = art.compose(GorePreset::Reduced).unwrap();
        assert_eq!(full.poses, three.poses, "3 and 4 share the sheets");
        assert_eq!(full.parts, three.parts);
        let fresh = parse_rgba("#b8322eff").unwrap();
        let wet = parse_rgba("#e2434bff").unwrap();
        // The crush key at preset 2 has no wet highlight and far less fresh red
        // than at 4 (the GOB-01 tongue keeps its `R`).
        let at = |sheet: &PresetSheets, colour: [u8; 4]| {
            let sheet = &sheet.poses;
            (0..sheet.cell_h)
                .flat_map(|y| (0..sheet.cell_w).map(move |x| (x, y)))
                .filter(|(x, y)| sheet.pixel(0, *x, *y) == colour)
                .count()
        };
        assert!(at(&full, wet) >= 3);
        assert_eq!(at(&two, wet), 0);
        assert!(at(&two, fresh) < at(&full, fresh));
        // Same silhouette at every preset: the gore layer only recolours or
        // adds, it never moves a body pixel (opaque count is monotone).
        for frame in 0..full.poses.frames {
            assert!(full.poses.opaque[frame] >= two.poses.opaque[frame]);
        }
    }

    #[test]
    fn part_filters_follow_the_spec_presets() {
        let art = art();
        let allowed = |preset: GorePreset| -> Vec<&str> {
            art.parts
                .iter()
                .filter(|p| p.allowed_at(preset))
                .map(|p| p.id.as_str())
                .collect()
        };
        // Preset 1: clean parts and the head only.
        assert_eq!(
            allowed(GorePreset::Off),
            ["head", "club", "head_dead", "gib_rag"]
        );
        // gib_bone, the wall spatters, pool 3 and the smear are preset 4 only.
        for id in [
            "gib_bone",
            "decal_wall_spatter",
            "decal_wall_spatter_b",
            "decal_pool_3",
            "decal_pool_3_r",
            "decal_smear",
        ] {
            for preset in [GorePreset::Off, GorePreset::Reduced, GorePreset::Medium] {
                assert!(!allowed(preset).contains(&id), "{id} at {preset:?}");
            }
            assert!(allowed(GorePreset::Full).contains(&id), "{id}");
        }
        // Preset 2 draws no gibs and no big decals; drops and small splats yes.
        let two = allowed(GorePreset::Reduced);
        for id in [
            "gib_flesh_a",
            "gib_flesh_b",
            "gib_bone",
            "decal_splat_m",
            "decal_pool_1",
        ] {
            assert!(!two.contains(&id), "{id}");
        }
        for id in ["drop_blood", "decal_splat_s"] {
            assert!(two.contains(&id), "{id}");
        }
    }

    #[test]
    fn anchor_geometry_round_trips_and_mirrors() {
        // The pose pivot pixel's centre is half a pixel right of foot_x, and a
        // flipped pose mirrors around foot_x.
        let (x, y) = pose_anchor_world((28, 55), (28, 55), 100.0, 40.0, false);
        assert_eq!((x, y), (100.5, 40.5));
        let (fx, fy) = pose_anchor_world((28, 55), (28, 55), 100.0, 40.0, true);
        assert_eq!((fx, fy), (99.5, 40.5));
        let (x, _) = pose_anchor_world((28, 55), (49, 43), 100.0, 40.0, false);
        let (fx, _) = pose_anchor_world((28, 55), (49, 43), 100.0, 40.0, true);
        assert_eq!(x - 100.0, 100.0 - fx);
        // A part anchored at the cell centre pixel sits half a pixel off centre.
        let c = part_center_for_anchor((32, 32), (16, 16), (10.5, 20.5), false);
        assert_eq!(c, (10.0, 21.0));
        // Upright, unflipped: the lowest point is the bottom of the box.
        let hull = [(-6.0, 4.0), (4.0, 4.0), (4.0, -4.0), (-6.0, -4.0)];
        assert!((lowest_below_centre(&hull, false, 0.0) - 4.0).abs() < 1e-4);
        // Turned a quarter, the box stands on its side: half the width down.
        assert!((lowest_below_centre(&hull, false, 90.0) - 6.0).abs() < 1e-3);
        // Flipping mirrors left and right but not up and down.
        assert!(
            (lowest_below_centre(&hull, true, 0.0) - lowest_below_centre(&hull, false, 0.0)).abs()
                < 1e-4
        );
        // A diagonal stick lies flat at an eighth turn, not leaning on a
        // quarter turn; a flat piece keeps its side.
        let stick = [(-6.0, -6.0), (-5.0, -6.0), (6.0, 5.0), (5.0, 6.0)];
        let flat = rest_angle(&stick, false, 80.0);
        assert!(
            (flat - 135.0).abs() < 1e-3 || (flat - 45.0).abs() < 1e-3,
            "{flat}"
        );
        assert!(rotated_height(&stick, false, flat) < 3.0);
        assert_eq!(rest_angle(&hull, false, 10.0), 0.0);
        assert_eq!(rest_angle(&hull, false, 170.0), 180.0);
    }

    // ------------------------------------------------- cf02 budget (Q22/Q43) ----

    fn budget() -> Cf02GoreBudget {
        Cf02GoreBudget::from_tuning(crate::cf02::active()).expect("active budget")
    }

    #[test]
    fn the_cf02_budget_needs_the_rules_and_its_switch() {
        let off = crate::cf02::Cf02Tuning::pinned().unwrap();
        assert!(Cf02GoreBudget::from_tuning(&off).is_none(), "rules off");
        let mut no_switch = crate::cf02::active().clone();
        no_switch.q36_gore_budget = false;
        assert!(Cf02GoreBudget::from_tuning(&no_switch).is_none());
    }

    #[test]
    fn q43_slammed_poses_scale_by_the_existing_preset_ratio() {
        use crate::death::SlamPose::{Flung, Stomped, WallCrumple};
        let b = budget();
        // Preset 4: the adopted bases (6, 4, 6 quarters; 0, 1, 1 parts).
        let at4 = |pose| {
            (
                b.slam_quarters(GorePreset::Full, pose),
                b.slam_parts(GorePreset::Full, pose),
            )
        };
        assert_eq!(at4(Stomped), (6, 0));
        assert_eq!(at4(WallCrumple), (4, 1));
        assert_eq!(at4(Flung), (6, 1));
        // Preset 3 halves the quarters (the preset's own 2 of 4); preset 1 and 2 draw none.
        assert_eq!(b.slam_quarters(GorePreset::Medium, Stomped), 3);
        assert_eq!(b.slam_quarters(GorePreset::Medium, WallCrumple), 2);
        assert_eq!(b.slam_parts(GorePreset::Medium, WallCrumple), 1);
        for preset in [GorePreset::Reduced, GorePreset::Off] {
            for pose in [Stomped, WallCrumple, Flung] {
                assert_eq!(b.slam_quarters(preset, pose), 0);
                assert_eq!(b.slam_parts(preset, pose), 0);
            }
        }
    }

    #[test]
    fn q22_only_preset_four_heavy_and_catastrophic_get_more() {
        use crate::combat::ForceTier::{Catastrophic, Heavy, Routine};
        let b = budget();
        for force in [Heavy, Catastrophic] {
            assert_eq!(b.gib_quarters(GorePreset::Full, force), 6);
            assert_eq!(b.extra_parts(GorePreset::Full, force), 1);
        }
        assert_eq!(b.gib_quarters(GorePreset::Full, Routine), 4);
        assert_eq!(b.extra_parts(GorePreset::Full, Routine), 0);
        for preset in [GorePreset::Medium, GorePreset::Reduced, GorePreset::Off] {
            for force in [Routine, Heavy, Catastrophic] {
                assert_eq!(b.gib_quarters(preset, force), preset.gib_quarters());
                assert_eq!(b.extra_parts(preset, force), 0);
            }
        }
        // Q43: one transient spray at preset 4 only, gone after 24 ticks.
        assert_eq!(b.routine_sprays(GorePreset::Full), 1);
        for preset in [GorePreset::Medium, GorePreset::Reduced, GorePreset::Off] {
            assert_eq!(b.routine_sprays(preset), 0);
        }
        assert_eq!(b.spray_ticks(), 24);
    }
}
