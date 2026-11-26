//! AREA-RINGS-01: the Broken Ring dressing of Floor 01 rooms 4a `ring_court`
//! and 4b `ring_split`. Pure and headless, no Bevy types.
//!
//! The room is composed from the catacomb kit exactly as before
//! ([`crate::f01_boiler::compose_kit_hooked`]); this module then applies the
//! room's `room_dressing` block of the rings contract
//! (`floor01_rings_kit.json`, which extends the opening kit and through it
//! ENV-01): it removes listed pieces (by id, or one exact instance by
//! `remove_at`), bricks up listed backwall rects with the composer's own brick
//! tiles (`backwall_fill`), appends the placements per layer and swaps the two
//! parallax strips. Pieces never produce solids (only collidable tiles
//! of the collision layer do), so the dressing is **presentation only**: the
//! tests below prove the solids, spawn, exit, views, walking line, blink
//! barriers and the ceiling slab are identical with and without it, and the
//! `f01_ring` tests prove the state hash is. Spec:
//! `docs/02-art/area-rings-01-spec.md`.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::camera_lab::room::Room;
use crate::env_kit::{EnvError, EnvKit, load_contract};

/// The rings kit contract (extends the opening kit, which extends ENV-01).
pub const RINGS_CONTRACT: &str = "assets/source/terrain/floor01/floor01_rings_kit.json";

/// One placement of the dressing (layout px, top-left).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Placement {
    pub layer: String,
    pub piece: String,
    pub x: i32,
    pub y: i32,
    /// Animation phase of an animated piece (a moved torch keeps its phase).
    #[serde(default)]
    pub frame_offset: Option<usize>,
}

/// A backwall rect (layout px, multiples of the 16 px tile) whose empty cells
/// the dressing fills with the composer's brick (RNG-AD20/AD21: a bricked-up
/// arch opening).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub struct FillRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// One room's `room_dressing` block.
#[derive(Debug, Clone, Deserialize)]
pub struct Dressing {
    pub remove: Vec<String>,
    /// Exact composer instances to drop (layer, piece, x, y); each must match
    /// exactly one piece.
    #[serde(default)]
    pub remove_at: Vec<Placement>,
    /// Backwall rects to brick up.
    #[serde(default)]
    pub backwall_fill: Vec<FillRect>,
    pub place: Vec<Placement>,
    /// Parallax layer name to the piece that replaces its strip.
    pub parallax: BTreeMap<String, String>,
}

fn read_json(root: &Path, rel: &str) -> Result<Value, EnvError> {
    let path = root.join(rel);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| EnvError::Io(format!("{}: {e}", path.display())))?;
    serde_json::from_str(&text).map_err(|e| EnvError::Parse(format!("{}: {e}", path.display())))
}

/// The dressing block of `room_id` (`ring_court`, `ring_split`) in the rings
/// contract.
pub fn dressing(root: &Path, room_id: &str) -> Result<Dressing, EnvError> {
    dressing_in(root, RINGS_CONTRACT, room_id)
}

/// The dressing block of `room_id` in the kit contract at `contract_path`
/// (AREA-HEAT-01 reuses the dressing code with its own contract).
pub fn dressing_in(root: &Path, contract_path: &str, room_id: &str) -> Result<Dressing, EnvError> {
    let contract = read_json(root, contract_path)?;
    serde_json::from_value(contract["room_dressing"][room_id].clone())
        .map_err(|e| EnvError::Parse(format!("{contract_path} room_dressing.{room_id}: {e}")))
}

/// Apply `dressing` to a composed `env-kit-layout-v2` value: drop the removed
/// pieces from every piece layer and the `remove_at` instances from theirs,
/// brick up the `backwall_fill` rects, append the placements in order, then swap
/// the parallax strips. Returns the number of `remove_at` entries that matched
/// no piece (zero for a valid dressing).
pub fn dress(layout: &mut Value, dressing: &Dressing) -> usize {
    dress_with(layout, dressing, RINGS_CONTRACT)
}

/// [`dress`] for the kit contract at `contract_path`: the layout's
/// `kit_contract` names it.
pub fn dress_with(layout: &mut Value, dressing: &Dressing, contract_path: &str) -> usize {
    if let Some(layers) = layout["piece_layers"].as_object_mut() {
        for list in layers.values_mut() {
            if let Some(list) = list.as_array_mut() {
                list.retain(|p| {
                    !p["piece"]
                        .as_str()
                        .is_some_and(|id| dressing.remove.iter().any(|r| r == id))
                });
            }
        }
    }
    let mut missed = 0;
    for r in &dressing.remove_at {
        let hit = layout["piece_layers"][r.layer.as_str()]
            .as_array_mut()
            .and_then(|list| {
                let at = list.iter().position(|p| {
                    p["piece"].as_str() == Some(r.piece.as_str())
                        && p["x"].as_i64() == Some(r.x.into())
                        && p["y"].as_i64() == Some(r.y.into())
                })?;
                Some(list.remove(at))
            });
        if hit.is_none() {
            missed += 1;
        }
    }
    if !dressing.backwall_fill.is_empty() {
        let fr = (layout["base_walk_line_y"].as_i64().unwrap_or(0) as i32 - 4) / 16;
        if let Some(rows) = layout["tile_layers"]["backwall"].as_array_mut() {
            for f in &dressing.backwall_fill {
                for row in f.y / 16..(f.y + f.h) / 16 {
                    let Some(cells) = rows.get_mut(row as usize).and_then(Value::as_array_mut)
                    else {
                        continue;
                    };
                    for col in f.x / 16..(f.x + f.w) / 16 {
                        if let Some(cell) = cells.get_mut(col as usize)
                            && cell.is_null()
                            && let Some(id) =
                                crate::camera_lab::layout::backwall_brick_id(col, row, fr)
                        {
                            *cell = json!(id);
                        }
                    }
                }
            }
        }
    }
    for p in &dressing.place {
        let list = &mut layout["piece_layers"][p.layer.as_str()];
        if !list.is_array() {
            *list = json!([]);
        }
        if let Some(list) = list.as_array_mut() {
            let mut v = json!({"piece": p.piece, "x": p.x, "y": p.y});
            if let Some(f) = p.frame_offset {
                v["frame_offset"] = json!(f);
            }
            list.push(v);
        }
    }
    for (layer, piece) in &dressing.parallax {
        layout["parallax_layers"][layer.as_str()]["piece"] = json!(piece);
    }
    layout["kit_contract"] = json!(contract_path);
    missed
}

/// The env kit of a Ring room: the room's composition (CF02 steps cleared,
/// platform face rows cleared, the two scroll-camera views) with the room's
/// dressing applied, built from the rings contract.
pub fn compose_ring_kit(root: &Path, room: &Room, room_id: &str) -> Result<EnvKit, String> {
    let contract = load_contract(root, RINGS_CONTRACT).map_err(|e| e.to_string())?;
    let dressing = dressing(root, room_id).map_err(|e| e.to_string())?;
    crate::f01_boiler::compose_kit_hooked(root, room, &[], Some(contract), |layout| {
        match dress(layout, &dressing) {
            0 => Ok(()),
            n => Err(format!("{room_id}: {n} remove_at entries match no piece")),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encounter;
    use crate::env_kit::IRect;
    use crate::f01_ring::RingRoom;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn room_of(room: RingRoom) -> Room {
        encounter::load(root(), room.encounter_path())
            .expect("the room loads")
            .room
    }

    fn rect_of(p: &crate::env_kit::PieceDraw) -> IRect {
        IRect {
            x: p.x,
            y: p.y,
            w: p.rect.w,
            h: p.rect.h,
        }
    }

    fn kits(room: RingRoom) -> (EnvKit, EnvKit) {
        let loaded = room_of(room);
        let plain = crate::f01_boiler::compose_kit(root(), &loaded).unwrap();
        let dressed = compose_ring_kit(root(), &loaded, room.id()).unwrap();
        (plain, dressed)
    }

    /// Spec section 9 / acceptance 4: the dressing changes no collider, spawn,
    /// exit, view or walking line, and no blink barrier or collision cell.
    #[test]
    fn the_dressing_changes_no_collider_spawn_exit_view_or_walk_line() {
        for room in RingRoom::ALL {
            let (plain, dressed) = kits(room);
            assert_eq!(dressed.solid_rects, plain.solid_rects, "{room:?}");
            assert_eq!(dressed.spawn, plain.spawn, "{room:?}");
            assert_eq!(dressed.exit, plain.exit, "{room:?}");
            assert_eq!(dressed.views, plain.views, "{room:?}");
            assert_eq!(dressed.base_walk_line_y, plain.base_walk_line_y, "{room:?}");
            assert_eq!(dressed.blink_barriers, plain.blink_barriers, "{room:?}");
            assert_eq!((dressed.width, dressed.height), (plain.width, plain.height));
            assert_eq!(dressed.collidable_cells, plain.collidable_cells, "{room:?}");
            // Only the backwall fill adds tiles (brick, never collidable).
            let d = dressing(root(), room.id()).unwrap();
            let filled: usize = d
                .backwall_fill
                .iter()
                .map(|f| ((f.w / 16) * (f.h / 16)) as usize)
                .sum();
            assert!(dressed.tiles.len() >= plain.tiles.len(), "{room:?}");
            assert!(
                dressed.tiles.len() <= plain.tiles.len() + filled,
                "{room:?}"
            );
            assert_eq!(dressed.actor_z, plain.actor_z, "{room:?}");
            assert_eq!(dressed.deck_top_px, plain.deck_top_px, "{room:?}");
            // The dressing did happen.
            assert!(dressed.pieces.len() > plain.pieces.len(), "{room:?}");
            assert!(dressed.pieces.iter().any(|p| p.piece == "ring_soffit"));
        }
    }

    /// The ceiling slab `launch` pushes is untouched by the dressing (the
    /// dressed kit and the plain kit take the same push).
    #[test]
    fn the_launched_room_keeps_its_ceiling_slab_and_kit_state() {
        use crate::f01_ring::launch_with;
        for room in RingRoom::ALL {
            let plain = launch_with(root(), room, false).unwrap();
            let dressed = launch_with(root(), room, true).unwrap();
            assert_eq!(dressed.kit.solid_rects, plain.kit.solid_rects, "{room:?}");
            let slab = dressed.kit.solid_rects.last().unwrap();
            assert_eq!(
                (slab.x, slab.y, slab.w, slab.h),
                (
                    0,
                    dressed.kit.base_walk_line_y - 228 - 16,
                    dressed.kit.width,
                    16
                ),
                "{room:?}: the lane ceiling slab"
            );
            assert_eq!(dressed.kit.blink_barriers, plain.kit.blink_barriers);
            assert_eq!(dressed.kit.views, plain.kit.views);
            assert_eq!(dressed.kit.spawn, plain.kit.spawn);
            assert_eq!(dressed.kit.exit, plain.kit.exit);
        }
    }

    /// Every placed piece exists in the merged contract with the layer the
    /// placement names, and its rect lies inside the room.
    #[test]
    fn every_placed_piece_exists_and_lies_inside_the_room() {
        let contract = load_contract(root(), RINGS_CONTRACT).unwrap();
        for room in RingRoom::ALL {
            let d = dressing(root(), room.id()).unwrap();
            let (_, dressed) = kits(room);
            assert!(!d.place.is_empty());
            for p in &d.place {
                let def = contract
                    .pieces
                    .iter()
                    .find(|c| c.id == p.piece)
                    .unwrap_or_else(|| panic!("{room:?}: unknown piece {}", p.piece));
                assert_eq!(def.layer, p.layer, "{room:?}: {}", p.piece);
                let r = IRect {
                    x: p.x,
                    y: p.y,
                    w: def.rect[2],
                    h: def.rect[3],
                };
                assert!(
                    r.x >= 0
                        && r.y >= 0
                        && r.right() <= dressed.width
                        && r.bottom() <= dressed.height,
                    "{room:?}: {} {r:?} is outside the {} x {} room",
                    p.piece,
                    dressed.width,
                    dressed.height
                );
            }
            for (layer, piece) in &d.parallax {
                assert!(contract.pieces.iter().any(|c| &c.id == piece), "{piece}");
                assert!(layer.starts_with("parallax_"));
            }
            // The kit holds every placement (a placed piece is drawn).
            for p in &d.place {
                assert!(
                    dressed
                        .pieces
                        .iter()
                        .any(|d| d.piece == p.piece && d.x == p.x && d.y == p.y),
                    "{room:?}: {} not in the kit",
                    p.piece
                );
            }
        }
    }

    /// RNG-AD20/AD21: every `remove_at` entry names exactly one composer piece,
    /// and the bricked-up rects leave no empty backwall cell: the full ring and
    /// the 4a entry door have brick behind them, no gallery.
    #[test]
    fn removals_match_one_piece_and_the_fills_leave_no_hole() {
        for room in RingRoom::ALL {
            let d = dressing(root(), room.id()).unwrap();
            let mut plain = None;
            crate::f01_boiler::compose_kit_hooked(root(), &room_of(room), &[], None, |layout| {
                plain = Some(layout.clone());
                Ok(())
            })
            .unwrap();
            let plain = plain.unwrap();
            for r in &d.remove_at {
                let n = plain["piece_layers"][r.layer.as_str()]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|p| {
                        p["piece"].as_str() == Some(r.piece.as_str())
                            && p["x"].as_i64() == Some(r.x.into())
                            && p["y"].as_i64() == Some(r.y.into())
                    })
                    .count();
                assert_eq!(n, 1, "{room:?}: remove_at {r:?}");
            }
            let (_, dressed) = kits(room);
            for f in &d.backwall_fill {
                assert_eq!((f.x % 16, f.y % 16, f.w % 16, f.h % 16), (0, 0, 0, 0));
                for row in f.y / 16..(f.y + f.h) / 16 {
                    for col in f.x / 16..(f.x + f.w) / 16 {
                        assert!(
                            dressed
                                .tiles
                                .iter()
                                .any(|t| t.layer == "backwall" && t.col == col && t.row == row),
                            "{room:?}: backwall hole at ({col}, {row}) inside {f:?}"
                        );
                    }
                }
            }
        }
        // The full ring's interior and the 4a entry door lie inside a fill.
        let court = dressing(root(), RingRoom::Court.id()).unwrap();
        let inside = |x: i32, y: i32, w: i32, h: i32| {
            court
                .backwall_fill
                .iter()
                .any(|f| x >= f.x && y >= f.y && x + w <= f.x + f.w && y + h <= f.y + f.h)
        };
        let ring = court
            .place
            .iter()
            .find(|p| p.piece == "ring_relief_landmark")
            .unwrap();
        assert!(
            inside(ring.x + 32, ring.y + 32, 144, 144),
            "the ring's inside is brick"
        );
        assert!(
            inside(208, 496, 64, 64),
            "the entry door has brick behind it"
        );
    }

    /// RNG-AD4 / KR9: the soffit tiles cover the room's full width at y 336..352,
    /// exactly over the ceiling slab the launch pushes.
    #[test]
    fn the_soffit_tiles_cover_the_full_width_over_the_ceiling_slab() {
        for room in RingRoom::ALL {
            let run = crate::f01_ring::launch(root(), room).unwrap();
            let mut tiles: Vec<IRect> = run
                .kit
                .pieces
                .iter()
                .filter(|p| p.piece == "ring_soffit")
                .map(rect_of)
                .collect();
            tiles.sort_by_key(|t| t.x);
            assert!(!tiles.is_empty());
            let mut at = 0;
            for t in &tiles {
                assert_eq!((t.x, t.y, t.w, t.h), (at, 336, 32, 16), "{room:?}");
                at += 32;
            }
            assert_eq!(
                at, run.kit.width,
                "{room:?}: the soffit spans the whole width"
            );
            let slab = run.kit.solid_rects.last().unwrap();
            assert_eq!(
                (slab.y, slab.bottom()),
                (336, 352),
                "{room:?}: the slab it hides"
            );
        }
    }

    /// The ledge fronts lie exactly on the platform rects: 32 px tiles and two
    /// 8 px end caps per platform.
    #[test]
    fn ledge_fronts_lie_on_the_platform_rects() {
        for room in RingRoom::ALL {
            let loaded = room_of(room);
            let run = crate::f01_ring::launch(root(), room).unwrap();
            for p in &loaded.platforms {
                let r = p.rect;
                let on = |id: &str| -> Vec<IRect> {
                    run.kit
                        .pieces
                        .iter()
                        .filter(|d| d.piece == id && d.y == r.y && d.x >= r.x && d.x < r.right())
                        .map(rect_of)
                        .collect()
                };
                let mid = on("ring_ledge_mid");
                assert_eq!(mid.len() as i32 * 32, r.w, "{room:?} {r:?}");
                assert!(mid.iter().all(|m| m.x >= r.x && m.right() <= r.right()));
                let l = on("ring_ledge_end_l");
                let rr = on("ring_ledge_end_r");
                assert_eq!((l.len(), rr.len()), (1, 1), "{room:?} {r:?}");
                assert_eq!(l[0].x, r.x);
                assert_eq!(rr[0].right(), r.right());
            }
        }
    }

    /// AREA-RINGS-01 evidence helper (ignored, writes only under
    /// `RINGS_LAYOUT_DIR`): the composed layout of each room, plain and dressed,
    /// for `scripts/measure_rings01.py` (the Python lit renderer of the kit).
    ///
    /// `RINGS_LAYOUT_DIR=<dir> cargo test --lib -- --ignored dump_ring_layouts`
    #[test]
    #[ignore = "evidence helper: writes the composed layouts for the Python measurements"]
    fn dump_ring_layouts() {
        let Ok(dir) = std::env::var("RINGS_LAYOUT_DIR") else {
            return;
        };
        std::fs::create_dir_all(&dir).unwrap();
        for room in RingRoom::ALL {
            for dressed in [false, true] {
                let contract = load_contract(root(), RINGS_CONTRACT).unwrap();
                let d = dressing(root(), room.id()).unwrap();
                let mut out = None;
                crate::f01_boiler::compose_kit_hooked(
                    root(),
                    &room_of(room),
                    &[],
                    Some(contract),
                    |layout| {
                        if dressed {
                            assert_eq!(dress(layout, &d), 0);
                        }
                        layout["kit_contract"] = json!(RINGS_CONTRACT);
                        out = Some(layout.clone());
                        Ok(())
                    },
                )
                .unwrap();
                let name = format!(
                    "{}.{}.layout.json",
                    room.id(),
                    if dressed { "dressed" } else { "plain" }
                );
                std::fs::write(
                    std::path::Path::new(&dir).join(name),
                    serde_json::to_string(&out.unwrap()).unwrap(),
                )
                .unwrap();
            }
        }
    }

    /// C3 evidence helper (ignored, writes only to `RING_DRESSING_DUMP`): every Ring
    /// room's composed dressing (piece id, position, z, frame) and strips, plain
    /// and dressed, one line each, so two builds can be compared with `diff`.
    ///
    /// `RING_DRESSING_DUMP=<file> cargo test --lib -- --ignored dump_ring_dressing`
    #[test]
    #[ignore = "evidence helper: writes the composed ring dressing for a byte comparison"]
    fn dump_ring_dressing() {
        let Ok(file) = std::env::var("RING_DRESSING_DUMP") else {
            return;
        };
        let mut out = String::new();
        for room in RingRoom::ALL {
            let (plain, dressed) = kits(room);
            for (kind, kit) in [("plain", &plain), ("dressed", &dressed)] {
                for p in &kit.pieces {
                    out += &format!(
                        "{} {kind} piece {} {} x={} y={} z={:?} frame_offset={} rect={:?} frames={:?}\n",
                        room.id(),
                        p.layer,
                        p.piece,
                        p.x,
                        p.y,
                        p.z,
                        p.frame_offset,
                        p.rect,
                        p.frames
                    );
                }
                for s in &kit.strips {
                    out += &format!(
                        "{} {kind} strip {} {} x_origin={} y={} z={:?} rect={:?}\n",
                        room.id(),
                        s.layer,
                        s.piece,
                        s.x_origin,
                        s.y,
                        s.z,
                        s.rect
                    );
                }
                out += &format!(
                    "{} {kind} tiles={} solids={:?}\n",
                    room.id(),
                    kit.tiles.len(),
                    kit.solid_rects
                );
            }
        }
        std::fs::write(file, out).unwrap();
    }

    /// No new light emitter. The torches keep their count (RNG-AD20 moves two,
    /// it adds or drops none), the parallax strips light every camera stop like
    /// the opening kit's (the rings strips carry its torch offsets), and the only
    /// room emitters that go are the gallery lights of the suppressed bay's arch
    /// ring: that opening is bricked up.
    #[test]
    fn the_light_emitter_count_is_unchanged() {
        use crate::lighting::ViewRect;
        let contract = load_contract(root(), RINGS_CONTRACT).unwrap();
        for room in RingRoom::ALL {
            let (plain, dressed) = kits(room);
            let d = dressing(root(), room.id()).unwrap();
            let removed_lights: usize = d
                .remove_at
                .iter()
                .filter(|r| r.piece != "torch_wall")
                .filter_map(|r| contract.pieces.iter().find(|c| c.id == r.piece))
                .map(|c| c.lights.len())
                .sum();
            let count = |kit: &EnvKit| {
                let scene = kit.lighting.as_ref().expect("the room is lit");
                let room_emitters = scene.emitters.iter().filter(|e| e.strip.is_none());
                let torches = room_emitters
                    .clone()
                    .filter(|e| e.id.starts_with("torch_wall/"))
                    .count();
                let others = room_emitters.count() - torches;
                let view = kit.views["abc_640x360"];
                let mut strips = Vec::new();
                let mut x = 0;
                while x + view.w <= kit.width {
                    let (g, dropped) = scene.gather(
                        kit,
                        ViewRect {
                            left: x,
                            top: view.y,
                            w: view.w,
                            h: view.h,
                        },
                        0,
                    );
                    assert_eq!(dropped, 0);
                    strips.push((g.far_end, g.mid_end));
                    x += 128;
                }
                (torches, others, strips)
            };
            let (pt, po, ps) = count(&plain);
            let (dt, d_o, ds) = count(&dressed);
            assert_eq!(dt, pt, "{room:?}: torch emitters");
            assert_eq!(d_o + removed_lights, po, "{room:?}: other room emitters");
            assert_eq!(ds, ps, "{room:?}: parallax emitters per camera stop");
            // And the parallax strips were swapped to the ring pieces.
            let strips: Vec<&str> = dressed.strips.iter().map(|s| s.piece.as_str()).collect();
            assert!(
                strips.contains(&match room {
                    RingRoom::Court => "ring_tiers_far",
                    RingRoom::Split => "ring_camp_far",
                }),
                "{room:?}: {strips:?}"
            );
            assert!(strips.contains(&"ring_tiers_mid"), "{room:?}: {strips:?}");
            assert!(!strips.contains(&"gallery_far") && !strips.contains(&"gallery_mid"));
        }
    }
}
