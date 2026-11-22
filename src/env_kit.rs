//! Loader for the Floor 1 environment kit contract (`env-kit-v2`) and layout
//! (`env-kit-layout-v2`). Pure and headless: no Bevy types, no image decoding.
//!
//! The data uses top-left, y-down virtual pixels. This module converts to the
//! y-up world used by `prototype::Block` exactly once (`to_world_y`).
//! Collision is derived only from `collidable` tiles of the `collision` layer;
//! lighting, parallax and decal data never reach the solids.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

use crate::lighting::{LightDef, LightingProfile, LightingScene};
use crate::prototype::{Block, Material, Rect};

pub const KIT_FORMAT: &str = "env-kit-v2";
pub const LAYOUT_FORMAT: &str = "env-kit-layout-v2";
/// Where the contract says the generated atlases live. The Bevy asset root is
/// this directory; asset names are paths below it.
pub const ATLAS_ASSET_PREFIX: &str = "generated/art/";
/// Actor contact rule: the lowest opaque row of an actor sits on the walking
/// line with this many pixels of overlap (owner decision: 0).
pub const FOOT_SINK_PX: i32 = 0;

#[derive(Debug)]
pub enum EnvError {
    Io(String),
    Parse(String),
    Invalid(String),
}

impl fmt::Display for EnvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(m) => write!(f, "io: {m}"),
            Self::Parse(m) => write!(f, "parse: {m}"),
            Self::Invalid(m) => write!(f, "invalid: {m}"),
        }
    }
}

impl std::error::Error for EnvError {}

fn invalid<T>(message: impl Into<String>) -> Result<T, EnvError> {
    Err(EnvError::Invalid(message.into()))
}

// ---------------------------------------------------------------- contract --

#[derive(Debug, Clone, Deserialize)]
pub struct LayerDef {
    pub name: String,
    pub role: String,
    pub kind: String,
    pub draw_order: u32,
    #[serde(default)]
    pub parallax: Option<f64>,
    /// Presentation only: the light group of the layer (`lighting`).
    #[serde(default)]
    pub light_group: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AtlasDef {
    pub image: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TileDef {
    pub id: String,
    pub atlas: String,
    pub rect: [i32; 4],
    pub role: String,
    pub collidable: bool,
    #[serde(default)]
    pub collider: Option<[i32; 4]>,
    #[serde(default)]
    pub walkable_top: Option<bool>,
    #[serde(default)]
    pub walk_line_y: Option<i32>,
    #[serde(default)]
    pub deck_back: Option<bool>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct FrameRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct AnimFrame {
    pub frame: FrameRect,
    pub duration: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Animation {
    pub frames: Vec<AnimFrame>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PieceDef {
    pub id: String,
    pub atlas: String,
    pub rect: [i32; 4],
    pub layer: String,
    #[serde(default)]
    pub tile_x: Option<bool>,
    #[serde(default)]
    pub animation: Option<Animation>,
    /// Presentation only: light emitters on the piece (`lighting`).
    #[serde(default)]
    pub lights: Vec<LightDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Contract {
    pub format: String,
    pub kit_id: String,
    pub tile_size: [i32; 2],
    pub layers: Vec<LayerDef>,
    pub actor_draw_order: u32,
    /// Presentation only: the light group actors are lit as.
    #[serde(default = "default_actor_light_group")]
    pub actor_light_group: String,
    pub atlases: BTreeMap<String, AtlasDef>,
    pub tiles: Vec<TileDef>,
    pub pieces: Vec<PieceDef>,
}

// ------------------------------------------------------------------ layout --

#[derive(Debug, Clone, Deserialize)]
pub struct Marker {
    pub id: String,
    pub kind: String,
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub rect: Option<[i32; 4]>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ParallaxPlacement {
    pub piece: String,
    pub x_origin: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PiecePlacement {
    pub piece: String,
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub frame_offset: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Layout {
    pub format: String,
    pub layout_id: String,
    pub kit_contract: String,
    pub grid_size: [i32; 2],
    pub pixel_size: [i32; 2],
    pub base_walk_line_y: i32,
    pub markers: Vec<Marker>,
    pub views: BTreeMap<String, [i32; 4]>,
    pub parallax_layers: BTreeMap<String, ParallaxPlacement>,
    pub tile_layers: BTreeMap<String, Vec<Vec<Option<String>>>>,
    pub piece_layers: BTreeMap<String, Vec<PiecePlacement>>,
    /// Presentation only: ambient, haze, shadow and bloom (`lighting`).
    #[serde(default)]
    pub lighting: Option<LightingProfile>,
}

fn default_actor_light_group() -> String {
    "actors".to_owned()
}

// ----------------------------------------------------------------- derived --

/// Integer rectangle in layout pixels (top-left origin, y down).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl IRect {
    pub fn right(self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(self) -> i32 {
        self.y + self.h
    }
}

/// One non-empty tile of a tile layer, ready to draw.
#[derive(Debug, Clone)]
pub struct TileDraw {
    pub layer: String,
    pub col: i32,
    pub row: i32,
    pub atlas: String,
    pub rect: IRect,
    pub z: f32,
}

/// One placed piece (decal, structure, prop, foreground); animated when
/// `frames` has more than one entry.
#[derive(Debug, Clone)]
pub struct PieceDraw {
    pub layer: String,
    pub piece: String,
    pub index: usize,
    pub x: i32,
    pub y: i32,
    pub atlas: String,
    pub rect: IRect,
    pub frames: Vec<(IRect, u32)>,
    pub frame_offset: usize,
    pub z: f32,
}

/// A horizontally repeating parallax strip.
#[derive(Debug, Clone)]
pub struct ParallaxStrip {
    pub layer: String,
    pub piece: String,
    pub atlas: String,
    pub rect: IRect,
    pub factor: f64,
    pub x_origin: i32,
    pub y: i32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewDef {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone)]
pub struct EnvKit {
    pub layout_id: String,
    pub width: i32,
    pub height: i32,
    pub tile: i32,
    pub base_walk_line_y: i32,
    pub atlases: BTreeMap<String, String>,
    pub tiles: Vec<TileDraw>,
    pub pieces: Vec<PieceDraw>,
    pub strips: Vec<ParallaxStrip>,
    /// Merged collider rectangles, layout pixels, y down.
    pub solid_rects: Vec<IRect>,
    pub spawn: (i32, i32),
    pub exit: Option<IRect>,
    pub views: BTreeMap<String, ViewDef>,
    pub actor_z: f32,
    pub foreground_z: f32,
    pub collidable_cells: usize,
    /// Presentation-only lighting data (`None` when the layout has no
    /// `lighting` profile). Never read by collision or the simulation.
    pub lighting: Option<LightingScene>,
    /// DONUT-PROG-01 DP-Q27 rule 4b: the room's optional `blink_barriers`
    /// (absent = none), layout pixels, y down. Collision never reads them; only
    /// the blink's destination search does.
    pub blink_barriers: Vec<IRect>,
}

/// Bevy z for a draw-order slot (steps of one, per the contract). Pieces in the
/// same layer add a tiny per-index bias so placement order is the tie-break.
pub fn layer_z(draw_order: u32) -> f32 {
    draw_order as f32
}

const PIECE_Z_STEP: f32 = 0.001;

/// y-up world y of a y-down layout line at `y`.
pub fn to_world_y(layout_height: i32, y: i32) -> f32 {
    (layout_height - y) as f32
}

fn load_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, EnvError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| EnvError::Io(format!("{}: {e}", path.display())))?;
    serde_json::from_str(&text).map_err(|e| EnvError::Parse(format!("{}: {e}", path.display())))
}

/// Longest `extends` chain a contract may have (a cycle guard).
const MAX_EXTENDS_DEPTH: usize = 4;

/// The contract at `path` (relative to `root`) with its `extends` chain merged,
/// as `scripts/env_kit.py resolve_extends` does: an extension adds atlases
/// (names must be new), tiles and pieces to its base and overrides descriptive
/// fields; a contract without `extends` is read as it is.
fn load_contract(root: &Path, path: &str) -> Result<Contract, EnvError> {
    let value = merged_contract_value(root, path, 0)?;
    serde_json::from_value(value).map_err(|e| EnvError::Parse(format!("{path}: {e}")))
}

fn merged_contract_value(
    root: &Path,
    path: &str,
    depth: usize,
) -> Result<serde_json::Value, EnvError> {
    use serde_json::Value;
    let mut ext: Value = load_json(&root.join(path))?;
    let Some(base_ref) = ext
        .get("extends")
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return Ok(ext);
    };
    if depth >= MAX_EXTENDS_DEPTH {
        return invalid(format!("contract {path} extends too deep (cycle?)"));
    }
    let mut merged = merged_contract_value(root, &base_ref, depth + 1)?;
    let (Some(base), Some(ext)) = (merged.as_object_mut(), ext.as_object_mut()) else {
        return invalid(format!("contract {path} is not an object"));
    };
    for (key, value) in std::mem::take(ext) {
        match (key.as_str(), value) {
            ("extends", _) => {}
            ("atlases", Value::Object(add)) => {
                let atlases = base
                    .entry("atlases")
                    .or_insert_with(|| Value::Object(Default::default()));
                let Some(atlases) = atlases.as_object_mut() else {
                    return invalid(format!("base atlases of {path} is not an object"));
                };
                for (name, atlas) in add {
                    if atlases.insert(name.clone(), atlas).is_some() {
                        return invalid(format!("contract {path} redefines base atlas {name}"));
                    }
                }
            }
            ("tiles" | "pieces" | "material_tiles", Value::Array(add)) => {
                let list = base
                    .entry(key.clone())
                    .or_insert_with(|| Value::Array(Vec::new()));
                let Some(list) = list.as_array_mut() else {
                    return invalid(format!("base {key} of {path} is not a list"));
                };
                list.extend(add);
            }
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
    Ok(merged)
}

fn irect(r: [i32; 4]) -> IRect {
    IRect {
        x: r[0],
        y: r[1],
        w: r[2],
        h: r[3],
    }
}

impl EnvKit {
    /// Load a layout (path relative to `root`) and the contract it names.
    pub fn load(root: &Path, layout_path: &str) -> Result<Self, EnvError> {
        let layout: Layout = load_json(&root.join(layout_path))?;
        let contract = load_contract(root, &layout.kit_contract)?;
        Self::build(&contract, &layout)
    }

    pub fn build(contract: &Contract, layout: &Layout) -> Result<Self, EnvError> {
        if contract.format != KIT_FORMAT {
            return invalid(format!(
                "contract format {:?}, expected {KIT_FORMAT} (v1 is rejected, see the kit migration notes)",
                contract.format
            ));
        }
        if layout.format != LAYOUT_FORMAT {
            return invalid(format!(
                "layout format {:?}, expected {LAYOUT_FORMAT}",
                layout.format
            ));
        }
        if contract.tile_size != [16, 16] {
            return invalid("only 16x16 tiles are supported");
        }
        let tile = contract.tile_size[0];
        let [cols, rows] = layout.grid_size;
        let (width, height) = (layout.pixel_size[0], layout.pixel_size[1]);
        if width != cols * tile || height != rows * tile {
            return invalid("pixel_size does not match grid_size * tile_size");
        }

        let mut atlases = BTreeMap::new();
        for (name, atlas) in &contract.atlases {
            let Some(asset) = atlas.image.strip_prefix(ATLAS_ASSET_PREFIX) else {
                return invalid(format!(
                    "atlas {name}: image {:?} is not below {ATLAS_ASSET_PREFIX}",
                    atlas.image
                ));
            };
            atlases.insert(name.clone(), asset.to_owned());
        }

        let layer_def = |name: &str| contract.layers.iter().find(|l| l.name == name);
        let tile_by_id: BTreeMap<&str, &TileDef> =
            contract.tiles.iter().map(|t| (t.id.as_str(), t)).collect();
        let piece_by_id: BTreeMap<&str, &PieceDef> =
            contract.pieces.iter().map(|p| (p.id.as_str(), p)).collect();

        // Tiles per layer (draw data) and solids (collision layer only).
        let mut tiles = Vec::new();
        let mut cells: Vec<IRect> = Vec::new();
        let mut collidable_cells = 0usize;
        for (layer_name, grid) in &layout.tile_layers {
            let Some(layer) = layer_def(layer_name) else {
                return invalid(format!("tile layer {layer_name} is not in the contract"));
            };
            if layer.kind != "tiles" {
                return invalid(format!("layer {layer_name} is not a tile layer"));
            }
            if grid.len() != rows as usize || grid.iter().any(|r| r.len() != cols as usize) {
                return invalid(format!("tile layer {layer_name}: wrong grid size"));
            }
            for (row, line) in grid.iter().enumerate() {
                for (col, cell) in line.iter().enumerate() {
                    let Some(id) = cell else { continue };
                    let Some(def) = tile_by_id.get(id.as_str()) else {
                        return invalid(format!("tile layer {layer_name}: unknown tile {id}"));
                    };
                    if !atlases.contains_key(&def.atlas) {
                        return invalid(format!("tile {id}: unknown atlas {}", def.atlas));
                    }
                    let (col, row) = (col as i32, row as i32);
                    tiles.push(TileDraw {
                        layer: layer_name.clone(),
                        col,
                        row,
                        atlas: def.atlas.clone(),
                        rect: irect(def.rect),
                        z: layer_z(layer.draw_order),
                    });
                    // Only collision-role, collidable tiles produce solids.
                    if layer.role == "collision" && def.collidable {
                        let Some(c) = def.collider else {
                            return invalid(format!("collidable tile {id} has no collider"));
                        };
                        collidable_cells += 1;
                        cells.push(IRect {
                            x: col * tile + c[0],
                            y: row * tile + c[1],
                            w: c[2],
                            h: c[3],
                        });
                    }
                }
            }
        }
        tiles.sort_by_key(|a| (a.z.to_bits(), a.row, a.col));

        // Pieces per layer, in placement order.
        let mut pieces = Vec::new();
        for (layer_name, placements) in &layout.piece_layers {
            let Some(layer) = layer_def(layer_name) else {
                return invalid(format!("piece layer {layer_name} is not in the contract"));
            };
            for (index, placement) in placements.iter().enumerate() {
                let Some(def) = piece_by_id.get(placement.piece.as_str()) else {
                    return invalid(format!("unknown piece {}", placement.piece));
                };
                if def.layer != *layer_name {
                    return invalid(format!(
                        "piece {} belongs to layer {}, placed in {layer_name}",
                        def.id, def.layer
                    ));
                }
                let frames = def
                    .animation
                    .as_ref()
                    .map(|a| {
                        a.frames
                            .iter()
                            .map(|f| {
                                (
                                    IRect {
                                        x: f.frame.x,
                                        y: f.frame.y,
                                        w: f.frame.w,
                                        h: f.frame.h,
                                    },
                                    f.duration,
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                pieces.push(PieceDraw {
                    layer: layer_name.clone(),
                    piece: def.id.clone(),
                    index,
                    x: placement.x,
                    y: placement.y,
                    atlas: def.atlas.clone(),
                    rect: irect(def.rect),
                    frames,
                    frame_offset: placement.frame_offset.unwrap_or(0),
                    z: layer_z(layer.draw_order) + index as f32 * PIECE_Z_STEP,
                });
            }
        }

        let mut strips = Vec::new();
        for (layer_name, placement) in &layout.parallax_layers {
            let Some(layer) = layer_def(layer_name) else {
                return invalid(format!(
                    "parallax layer {layer_name} is not in the contract"
                ));
            };
            let (Some(factor), Some(def)) =
                (layer.parallax, piece_by_id.get(placement.piece.as_str()))
            else {
                return invalid(format!(
                    "parallax layer {layer_name}: missing factor or piece"
                ));
            };
            if def.tile_x != Some(true) || def.rect[2] <= 0 {
                return invalid(format!("parallax piece {} must tile horizontally", def.id));
            }
            strips.push(ParallaxStrip {
                layer: layer_name.clone(),
                piece: def.id.clone(),
                atlas: def.atlas.clone(),
                rect: irect(def.rect),
                factor,
                x_origin: placement.x_origin,
                y: placement.y,
                z: layer_z(layer.draw_order),
            });
        }
        strips.sort_by_key(|s| s.z.to_bits());

        let marker = |kind: &str| layout.markers.iter().find(|m| m.kind == kind);
        let Some(spawn) = marker("spawn") else {
            return invalid("layout has no spawn marker");
        };
        let exit = marker("exit").and_then(|m| m.rect).map(irect);
        let views = layout
            .views
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    ViewDef {
                        x: v[0],
                        y: v[1],
                        w: v[2],
                        h: v[3],
                    },
                )
            })
            .collect();

        let mut kit = Self {
            layout_id: layout.layout_id.clone(),
            width,
            height,
            tile,
            base_walk_line_y: layout.base_walk_line_y,
            atlases,
            tiles,
            pieces,
            strips,
            solid_rects: merge_rects(cells),
            spawn: (spawn.x, spawn.y),
            exit,
            views,
            actor_z: layer_z(contract.actor_draw_order),
            foreground_z: contract
                .layers
                .iter()
                .find(|l| l.name == "foreground")
                .map_or(8.0, |l| layer_z(l.draw_order)),
            collidable_cells,
            lighting: None,
            blink_barriers: Vec::new(),
        };
        // Lighting is derived last and only reads the finished kit (walking
        // lines for floor pools); nothing flows back into collision.
        kit.lighting = LightingScene::build(contract, layout, &kit)
            .map_err(|e| EnvError::Invalid(e.to_string()))?;
        Ok(kit)
    }

    /// Static solids as y-up greybox blocks (the shared hero physics input).
    pub fn blocks(&self) -> Vec<Block> {
        self.solid_rects
            .iter()
            .map(|r| Block {
                rect: Rect {
                    left: r.x as f32,
                    bottom: to_world_y(self.height, r.bottom()),
                    width: r.w as f32,
                    height: r.h as f32,
                },
                material: Material::Stone,
                intact: true,
            })
            .collect()
    }

    /// Standing surface (layout y, y down) of the column at pixel `x`: the top
    /// of the highest solid that starts no more than two tiles (the contract's
    /// `max_step_tiles`) above the base walking line. A column that is solid
    /// from the ceiling down (an end wall) has no walking line.
    pub fn walking_line_at(&self, x: i32) -> Option<i32> {
        let highest_floor = self.base_walk_line_y - 2 * self.tile;
        let covers = |r: &IRect, y: i32| x >= r.x && x < r.right() && y >= r.y && y < r.bottom();
        // Solid right above the highest allowed floor: an end wall, not floor.
        if self
            .solid_rects
            .iter()
            .any(|r| covers(r, highest_floor - 1))
        {
            return None;
        }
        self.solid_rects
            .iter()
            .filter(|r| x >= r.x && x < r.right() && r.y >= highest_floor)
            .map(|r| r.y)
            .min()
    }

    pub fn view(&self, name: &str) -> Option<ViewDef> {
        self.views.get(name).copied()
    }

    pub fn atlas_asset(&self, atlas: &str) -> Option<&str> {
        self.atlases.get(atlas).map(String::as_str)
    }

    /// Spawn point as the hero's AABB centre in y-up world coordinates
    /// (feet on the walking line, `half_height` above it).
    pub fn spawn_centre(&self, half_height: f32) -> (f32, f32) {
        // Marker y is the foot contact row, so the walking line is one lower.
        (
            self.spawn.0 as f32,
            to_world_y(self.height, self.spawn.1 + 1) + half_height,
        )
    }
}

/// Merge per-cell rectangles: horizontal runs first, then vertical stacks with
/// an identical x span. Output order is deterministic (x, width, y).
pub fn merge_rects(mut cells: Vec<IRect>) -> Vec<IRect> {
    cells.sort_by_key(|r| (r.y, r.h, r.x));
    let mut runs: Vec<IRect> = Vec::new();
    for cell in cells {
        if let Some(last) = runs.last_mut()
            && last.y == cell.y
            && last.h == cell.h
            && last.right() == cell.x
        {
            last.w += cell.w;
            continue;
        }
        runs.push(cell);
    }
    runs.sort_by_key(|r| (r.x, r.w, r.y));
    let mut merged: Vec<IRect> = Vec::new();
    for run in runs {
        if let Some(last) = merged.last_mut()
            && last.x == run.x
            && last.w == run.w
            && last.bottom() == run.y
        {
            last.h += run.h;
            continue;
        }
        merged.push(run);
    }
    merged
}

/// Left edge, in layout pixels, of the strip copy `k = 0` for a parallax layer.
///
/// Contract: a strip pixel at strip-local `u` is drawn at screen x
/// `x_origin + k*W + u - camera_x*factor`, snapped to whole virtual pixels. In
/// layout pixels that is `camera_x + x_origin - camera_x*factor`, rounded
/// (ties away from zero).
pub fn parallax_start(x_origin: i32, camera_left: i32, factor: f64) -> i32 {
    let screen = f64::from(x_origin) - f64::from(camera_left) * factor;
    camera_left + screen.round() as i32
}

/// Layout x of every strip copy that touches `[camera_left, camera_left+view_w)`.
pub fn parallax_copies(start: i32, strip_w: i32, camera_left: i32, view_w: i32) -> Vec<i32> {
    // Smallest copy start that is > camera_left - strip_w.
    let first = start + (camera_left - strip_w - start).div_euclid(strip_w) * strip_w + strip_w;
    let mut out = Vec::new();
    let mut x = first;
    while x < camera_left + view_w {
        out.push(x);
        x += strip_w;
    }
    out
}

/// Frame index of an animated piece at a fixed-step `tick` (60 Hz). Frame
/// durations are in milliseconds; `frame_offset` starts the cycle on that
/// frame. Pure integer arithmetic, so it is identical on every machine.
pub fn animation_frame(durations_ms: &[u32], frame_offset: usize, tick: u64) -> usize {
    let n = durations_ms.len();
    if n <= 1 {
        return 0;
    }
    let cycle: u64 = durations_ms.iter().map(|&d| u64::from(d)).sum();
    if cycle == 0 {
        // All-zero durations would divide by zero; show the first frame.
        return 0;
    }
    let start: u64 = durations_ms
        .iter()
        .take(frame_offset % n)
        .map(|&d| u64::from(d))
        .sum();
    let mut t = (tick * 1000 / 60 + start) % cycle;
    for (index, &d) in durations_ms.iter().enumerate() {
        if t < u64::from(d) {
            return index;
        }
        t -= u64::from(d);
    }
    n - 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn files() -> (Contract, Layout) {
        let layout: Layout = load_json(&root().join(LAYOUT)).unwrap();
        let contract: Contract = load_json(&root().join(&layout.kit_contract)).unwrap();
        (contract, layout)
    }

    fn kit() -> EnvKit {
        EnvKit::load(&root(), LAYOUT).unwrap()
    }

    #[test]
    fn sample_layout_and_contract_parse_with_expected_shape() {
        let kit = kit();
        assert_eq!((kit.width, kit.height), (2048, 400));
        assert_eq!(kit.base_walk_line_y, 308);
        assert_eq!(kit.spawn, (72, 307));
        assert_eq!(
            kit.exit,
            Some(IRect {
                x: 1720,
                y: 176,
                w: 128,
                h: 128
            })
        );
        assert_eq!(kit.strips.len(), 2);
        assert_eq!(
            kit.tiles.iter().filter(|t| t.layer == "backwall").count(),
            1674
        );
        assert_eq!(
            kit.tiles.iter().filter(|t| t.layer == "collision").count(),
            1190
        );
        assert!(kit.view("abc_640x360").is_some() && kit.view("abc_640x400").is_some());
        assert!(
            kit.atlases
                .values()
                .all(|a| a.starts_with("floor01_") && a.ends_with(".png"))
        );
    }

    #[test]
    fn v1_formats_are_rejected() {
        let (mut contract, layout) = files();
        contract.format = "env-kit-v1".into();
        assert!(EnvKit::build(&contract, &layout).is_err());
        let (contract, mut layout) = files();
        layout.format = "env-kit-layout-v1".into();
        assert!(EnvKit::build(&contract, &layout).is_err());
    }

    #[test]
    fn only_collidable_collision_tiles_produce_solids() {
        let (contract, layout) = files();
        let kit = EnvKit::build(&contract, &layout).unwrap();
        let by_id: BTreeMap<_, _> = contract.tiles.iter().map(|t| (t.id.as_str(), t)).collect();
        let mut expected_cells = 0;
        let mut expected_area = 0i64;
        for line in &layout.tile_layers["collision"] {
            for id in line.iter().flatten() {
                let def = by_id[id.as_str()];
                if def.collidable {
                    expected_cells += 1;
                    let c = def.collider.unwrap();
                    expected_area += i64::from(c[2]) * i64::from(c[3]);
                } else {
                    // deck_back and any other visual tile has no collider.
                    assert!(def.collider.is_none(), "{id}");
                }
            }
        }
        assert_eq!(kit.collidable_cells, expected_cells);
        let area: i64 = kit
            .solid_rects
            .iter()
            .map(|r| i64::from(r.w) * i64::from(r.h))
            .sum();
        assert_eq!(
            area, expected_area,
            "merging must neither add nor lose area"
        );
        assert!(
            kit.solid_rects.len() < expected_cells / 4,
            "merge is effective"
        );
        // Merged rectangles never overlap.
        for (i, a) in kit.solid_rects.iter().enumerate() {
            for b in &kit.solid_rects[i + 1..] {
                let overlap =
                    a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom();
                assert!(!overlap, "{a:?} overlaps {b:?}");
            }
        }
    }

    #[test]
    fn deck_back_never_collides_and_deck_front_uses_the_walking_line() {
        let (contract, layout) = files();
        let by_id: BTreeMap<_, _> = contract.tiles.iter().map(|t| (t.id.as_str(), t)).collect();
        let kit = EnvKit::build(&contract, &layout).unwrap();
        for def in &contract.tiles {
            if def.deck_back == Some(true) {
                assert!(!def.collidable && def.collider.is_none(), "{}", def.id);
            }
            if def.walkable_top == Some(true) {
                assert_eq!(def.collider, Some([0, 4, 16, 12]), "{}", def.id);
                assert_eq!(def.walk_line_y, Some(4));
            }
        }
        // Base corridor: row 18 is deck_back (non-collidable), row 19 deck_front.
        let row18 = &layout.tile_layers["collision"][18];
        let row19 = &layout.tile_layers["collision"][19];
        let probe_col = 4; // x 64..80: base corridor, left of the first platform
        assert!(by_id[row18[probe_col].as_ref().unwrap().as_str()].deck_back == Some(true));
        assert_eq!(
            by_id[row19[probe_col].as_ref().unwrap().as_str()].walk_line_y,
            Some(4)
        );
        assert_eq!(kit.walking_line_at(72), Some(kit.base_walk_line_y));
        // A1 one-tile platform (x 112..208) and A2 two-tile platform (x 1392..1520).
        assert_eq!(kit.walking_line_at(168), Some(292));
        assert_eq!(kit.walking_line_at(1450), Some(276));
        // The end wall column has no walking line.
        assert_eq!(kit.walking_line_at(8), None);
    }

    #[test]
    fn mutating_backwall_or_pieces_changes_no_collision() {
        let (contract, mut layout) = files();
        let before = EnvKit::build(&contract, &layout).unwrap().solid_rects;
        let backwall = layout.tile_layers.get_mut("backwall").unwrap();
        for row in backwall.iter_mut() {
            for cell in row.iter_mut() {
                *cell = None;
            }
        }
        layout.piece_layers.clear();
        layout.parallax_layers.clear();
        let after = EnvKit::build(&contract, &layout).unwrap().solid_rects;
        assert_eq!(before, after);
    }

    #[test]
    fn z_table_follows_the_contract_draw_order() {
        let (contract, _) = files();
        let order: Vec<(String, u32)> = contract
            .layers
            .iter()
            .map(|l| (l.name.clone(), l.draw_order))
            .collect();
        assert_eq!(
            order,
            [
                ("parallax_far", 0),
                ("parallax_mid", 1),
                ("backwall", 2),
                ("backwall_decal", 3),
                ("collision", 4),
                ("structure", 5),
                ("prop", 6),
                ("foreground", 8),
            ]
            .map(|(n, o)| (n.to_owned(), o))
        );
        assert_eq!(contract.actor_draw_order, 7);
        let kit = kit();
        assert_eq!(kit.actor_z, 7.0);
        for piece in &kit.pieces {
            assert!(piece.z.floor() as u32 <= 8);
        }
        // Per-index bias never crosses into the next layer.
        let max_bias = kit.pieces.iter().map(|p| p.z.fract()).fold(0.0, f32::max);
        assert!(max_bias < 0.5);
    }

    #[test]
    fn parallax_offset_math_matches_the_contract() {
        // Camera at 0: strip starts at x_origin.
        assert_eq!(parallax_start(115, 0, 0.35), 115);
        // Screen x of the strip = x_origin - camera*factor, snapped.
        for camera in [0, 1, 9, 10, 328, 1408] {
            let start = parallax_start(115, camera, 0.35);
            let screen = start - camera;
            assert_eq!(
                screen,
                (115.0 - f64::from(camera) * 0.35).round() as i32,
                "camera {camera}"
            );
        }
        // The far layer moves 0.35 as fast as the camera, mid 0.6.
        let a = parallax_start(0, 100, 0.35) - 100;
        let b = parallax_start(0, 200, 0.35) - 200;
        assert_eq!(a - b, 35);
        // Copies always cover the view with no gap.
        for camera in [0, 3, 327, 328, 1000, 1408] {
            let start = parallax_start(115, camera, 0.35);
            let copies = parallax_copies(start, 440, camera, 640);
            assert!(copies.first().unwrap() <= &camera);
            assert!(copies.last().unwrap() + 440 >= camera + 640);
            for pair in copies.windows(2) {
                assert_eq!(pair[1] - pair[0], 440);
            }
            assert_eq!((copies[0] - start).rem_euclid(440), 0);
        }
    }

    #[test]
    fn all_zero_durations_do_not_divide_by_zero() {
        assert_eq!(animation_frame(&[0, 0, 0], 1, 1234), 0);
        assert_eq!(animation_frame(&[0], 0, 5), 0);
    }

    #[test]
    fn torch_frame_comes_from_the_tick_and_contract_durations() {
        let burn = [120, 140, 100, 130];
        // 60 Hz ticks are 16.666 ms: tick 0 -> frame 0, tick 8 (133 ms) -> frame 1.
        assert_eq!(animation_frame(&burn, 0, 0), 0);
        assert_eq!(animation_frame(&burn, 0, 7), 0);
        assert_eq!(animation_frame(&burn, 0, 8), 1);
        assert_eq!(animation_frame(&burn, 0, 16), 2);
        assert_eq!(animation_frame(&burn, 0, 22), 3);
        // The cycle is 490 ms = 29.4 ticks.
        assert_eq!(animation_frame(&burn, 0, 30), 0);
        // frame_offset starts the cycle on that frame.
        for offset in 0..4 {
            assert_eq!(animation_frame(&burn, offset, 0), offset);
        }
        assert_eq!(animation_frame(&[100], 3, 99), 0);
        // The loaded torch has the contract's four frames.
        let kit = kit();
        let torch = kit
            .pieces
            .iter()
            .find(|p| p.piece == "torch_wall")
            .expect("torch placed");
        let durations: Vec<u32> = torch.frames.iter().map(|f| f.1).collect();
        assert_eq!(durations, burn);
    }

    #[test]
    fn merge_rects_joins_rows_and_stacks() {
        let cells = (0..3)
            .flat_map(|c| {
                (0..2).map(move |r| IRect {
                    x: c * 16,
                    y: r * 16,
                    w: 16,
                    h: 16,
                })
            })
            .collect();
        assert_eq!(
            merge_rects(cells),
            vec![IRect {
                x: 0,
                y: 0,
                w: 48,
                h: 32
            }]
        );
    }
}
