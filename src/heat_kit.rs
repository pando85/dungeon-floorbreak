//! AREA-HEAT-01: the scorched corral dressing of Floor 01 room 0 `corral`.
//! Pure and headless, no Bevy types.
//!
//! The room is composed from the catacomb kit with its own authored steps
//! exactly as before ([`crate::f01_boiler::compose_kit_hooked`]); this module
//! then applies the room's `room_dressing` block of the heat contract
//! (`floor01_heat_kit.json`, which extends the opening kit and through it
//! ENV-01) with the dressing code of the Broken Ring kit
//! ([`crate::ring_kit::dress_with`]): scorch and ash decals, spit traces, the
//! step fronts, the dock bumper, the pen rails and the timber debris.
//! Pieces never produce solids, so the dressing is **presentation only**: the
//! tests below prove the solids, spawn, exit, views, walking line, blink
//! barriers and the ceiling slab are identical with and without it, and the
//! state hash is equal on every tick. The goblin partition is **not** placed
//! here (`HEAT-AD1`): its three frames are in the contract as a state group and
//! CORRAL-D's presenter draws the frame of the prop's state
//! ([`partition_piece`], [`partition_origin`]; the Bevy half is the scene's
//! `corral_art`). Spec: `docs/02-art/area-heat-01-spec.md`.

use std::path::Path;

use crate::camera_lab::room::{IRect as RoomRect, Room};
use crate::env_kit::{EnvError, EnvKit, load_contract};
use crate::ring_kit::{Dressing, dress_with, dressing_in};

/// The heat kit contract (extends the opening kit, which extends ENV-01).
pub const HEAT_CONTRACT: &str = "assets/source/terrain/floor01/floor01_heat_kit.json";

/// The room id of the dressing block in the heat contract.
pub const ROOM_ID: &str = "corral";

/// The dressing block of room 0 in the heat contract.
pub fn dressing(root: &Path) -> Result<Dressing, EnvError> {
    dressing_in(root, HEAT_CONTRACT, ROOM_ID)
}

/// CORRAL-D (`HEAT-AD1`): the three state frames of the goblin partition, in
/// [`PartitionState`] order: intact, weakened (after one or two counted hero
/// hits) and broken (the 16 px remnant). One 40 x 48 canvas each in the atlas
/// `floor01_heat_partition`.
pub const PARTITION_PIECES: [&str; 3] = [
    "heat_partition_intact",
    "heat_partition_weakened",
    "heat_partition_broken",
];

/// The atlas image of the partition frames.
pub const PARTITION_ATLAS: &str = "generated/art/floor01_heat_partition.png";

/// `CD-AD1`: the canvas margin left of the solid core. The spec (section 6)
/// draws the 16 x 48 solid in canvas columns 12..27, so the canvas's top-left is
/// the prop's collider top-left minus this: (1392 - 12, 532) = (1380, 532).
pub const PARTITION_MARGIN_X: i32 = 12;
/// The canvas, and the solid core inside it (columns of the canvas).
pub const PARTITION_CANVAS: (i32, i32) = (40, 48);
pub const PARTITION_CORE: (i32, i32) = (12, 28);

/// The frame piece of a partition state (presentation only).
pub const fn partition_piece(state: crate::env_prop::PartitionState) -> &'static str {
    PARTITION_PIECES[state as usize]
}

/// The canvas's top-left (layout px, y down) for the partition whose standing
/// solid is `target`: the solid core of the art sits exactly on the collider.
pub const fn partition_origin(target: RoomRect) -> (i32, i32) {
    (target.x - PARTITION_MARGIN_X, target.y)
}

/// The atlas rectangle `[x, y, w, h]` of each state frame, from the heat
/// contract (`PARTITION_PIECES` order).
pub fn partition_atlas_rects(root: &Path) -> Result<[[i32; 4]; 3], String> {
    let contract = load_contract(root, HEAT_CONTRACT).map_err(|e| e.to_string())?;
    let mut out = [[0; 4]; 3];
    for (slot, id) in out.iter_mut().zip(PARTITION_PIECES) {
        let piece = contract
            .pieces
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("the heat contract has no piece {id}"))?;
        *slot = piece.rect;
    }
    Ok(out)
}

/// The env kit of room 0: the room's composition with its own authored steps
/// and the two scroll-camera views, with the corral dressing applied, built
/// from the heat contract. An unmatched `remove_at` entry is an error.
pub fn compose_heat_kit(root: &Path, room: &Room, steps: &[RoomRect]) -> Result<EnvKit, String> {
    let contract = load_contract(root, HEAT_CONTRACT).map_err(|e| e.to_string())?;
    let dressing = dressing(root).map_err(|e| e.to_string())?;
    crate::f01_boiler::compose_kit_hooked(root, room, steps, Some(contract), |layout| {
        match dress_with(layout, &dressing, HEAT_CONTRACT) {
            0 => Ok(()),
            n => Err(format!("{ROOM_ID}: {n} remove_at entries match no piece")),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env_kit::IRect;
    use serde_json::json;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn rect_of(p: &crate::env_kit::PieceDraw) -> IRect {
        IRect {
            x: p.x,
            y: p.y,
            w: p.rect.w,
            h: p.rect.h,
        }
    }

    fn overlaps(r: IRect, x0: i32, x1: i32, y0: i32, y1: i32) -> bool {
        r.x < x1 && r.right() > x0 && r.y < y1 && r.bottom() > y0
    }

    fn kits() -> (EnvKit, EnvKit) {
        let run = crate::f01_corral::launch_with(root(), false).unwrap();
        let plain = crate::f01_boiler::compose_kit_with_steps(root(), &run.loaded.room, &run.steps)
            .unwrap();
        let dressed = compose_heat_kit(root(), &run.loaded.room, &run.steps).unwrap();
        (plain, dressed)
    }

    /// Spec section 4 / A6: the dressing changes no collider, spawn, exit,
    /// view or walking line, and no blink barrier or collision cell.
    #[test]
    fn the_dressing_changes_no_collider_spawn_exit_view_or_walk_line() {
        let (plain, dressed) = kits();
        assert_eq!(dressed.solid_rects, plain.solid_rects);
        assert_eq!(dressed.spawn, plain.spawn);
        assert_eq!(dressed.exit, plain.exit);
        assert_eq!(dressed.views, plain.views);
        assert_eq!(dressed.base_walk_line_y, plain.base_walk_line_y);
        assert_eq!(dressed.blink_barriers, plain.blink_barriers);
        assert_eq!((dressed.width, dressed.height), (plain.width, plain.height));
        assert_eq!(dressed.collidable_cells, plain.collidable_cells);
        assert_eq!(dressed.tiles.len(), plain.tiles.len(), "no tile is added");
        assert_eq!(dressed.actor_z, plain.actor_z);
        assert_eq!(dressed.deck_top_px, plain.deck_top_px);
        assert_eq!(dressed.strips.len(), plain.strips.len());
        // The dressing did happen.
        assert!(dressed.pieces.len() > plain.pieces.len());
        assert!(
            dressed
                .pieces
                .iter()
                .any(|p| p.piece == "heat_scorch_wall_a")
        );
    }

    /// The launched room keeps its ceiling slab, its steps and its blink
    /// barriers: the dressed and the plain launch take the same pushes.
    #[test]
    fn the_launched_room_keeps_its_ceiling_slab_steps_and_kit_state() {
        let plain = crate::f01_corral::launch_with(root(), false).unwrap();
        let dressed = crate::f01_corral::launch_with(root(), true).unwrap();
        assert_eq!(dressed.kit.solid_rects, plain.kit.solid_rects);
        assert_eq!(dressed.steps, plain.steps);
        let slab = dressed.kit.solid_rects.last().unwrap();
        assert_eq!(
            (slab.x, slab.y, slab.w, slab.h),
            (
                0,
                dressed.kit.base_walk_line_y - 228 - 16,
                dressed.kit.width,
                16
            ),
            "the lane ceiling slab"
        );
        assert_eq!(dressed.kit.blink_barriers, plain.kit.blink_barriers);
        assert_eq!(dressed.kit.views, plain.kit.views);
        assert_eq!(dressed.kit.spawn, plain.kit.spawn);
        assert_eq!(dressed.kit.exit, plain.kit.exit);
        assert_eq!(dressed.kit.walking_line_at(112), Some(580));
        // The default launch is the dressed one (visible in a bare f01 run).
        let default = crate::f01_corral::launch(root()).unwrap();
        assert_eq!(default.kit.pieces.len(), dressed.kit.pieces.len());
    }

    /// Every placed piece exists in the merged contract with the layer the
    /// placement names, lies inside the room and is held by the kit.
    #[test]
    fn every_placed_piece_exists_and_lies_inside_the_room() {
        let contract = load_contract(root(), HEAT_CONTRACT).unwrap();
        let d = dressing(root()).unwrap();
        let (_, dressed) = kits();
        assert!(d.remove.is_empty() && d.remove_at.is_empty() && d.parallax.is_empty());
        assert!(d.backwall_fill.is_empty());
        assert!(!d.place.is_empty());
        for p in &d.place {
            let def = contract
                .pieces
                .iter()
                .find(|c| c.id == p.piece)
                .unwrap_or_else(|| panic!("unknown piece {}", p.piece));
            assert_eq!(def.layer, p.layer, "{}", p.piece);
            let r = IRect {
                x: p.x,
                y: p.y,
                w: def.rect[2],
                h: def.rect[3],
            };
            assert!(
                r.x >= 0 && r.y >= 0 && r.right() <= dressed.width && r.bottom() <= 580,
                "{} {r:?} is outside the {} x {} room",
                p.piece,
                dressed.width,
                dressed.height
            );
            assert!(
                dressed
                    .pieces
                    .iter()
                    .any(|d| d.piece == p.piece && d.x == p.x && d.y == p.y),
                "{} not in the kit",
                p.piece
            );
        }
    }

    /// Spec section 5: nothing near the partition, no floor decal on a step,
    /// no wall decal over a bay opening, every wall decal on the world bond,
    /// and the partition is not placed (`HEAT-AD1`).
    #[test]
    fn the_placements_keep_clear_of_the_partition_the_steps_and_the_bay_openings() {
        let contract = load_contract(root(), HEAT_CONTRACT).unwrap();
        let d = dressing(root()).unwrap();
        let size = |id: &str| {
            let def = contract.pieces.iter().find(|c| c.id == id).unwrap();
            (def.rect[2], def.rect[3])
        };
        let bays = [
            (112, 320),
            (448, 656),
            (784, 992),
            (1248, 1456),
            (1584, 1792),
        ];
        for p in &d.place {
            let (w, h) = size(&p.piece);
            let r = IRect {
                x: p.x,
                y: p.y,
                w,
                h,
            };
            assert!(
                !overlaps(r, 1372, 1428, i32::MIN, i32::MAX),
                "{} {r:?} is in the partition's clear zone",
                p.piece
            );
            assert!(!p.piece.starts_with("heat_partition"), "{}", p.piece);
            let floor_decal = p.layer == "structure"
                && (p.piece.contains("scorch_floor")
                    || p.piece.contains("ash_floor")
                    || p.piece.contains("trace_spit"));
            if floor_decal {
                assert!(
                    !overlaps(r, 1120, 1216, i32::MIN, i32::MAX)
                        && !overlaps(r, 1856, 2304, i32::MIN, i32::MAX),
                    "{} {r:?} lies on a step",
                    p.piece
                );
                assert!(
                    r.y >= 561 && r.bottom() <= 573,
                    "{} {r:?} leaves the deck's back half",
                    p.piece
                );
            }
            if p.layer == "backwall_decal" {
                for (x0, x1) in bays {
                    assert!(
                        !overlaps(r, x0, x1, 352, 576),
                        "{} {r:?} is over the bay opening {x0}..{x1}",
                        p.piece
                    );
                }
                assert_eq!(
                    (p.x % 32, p.y % 16),
                    (0, 0),
                    "{}: a wall decal sits on the world bond",
                    p.piece
                );
            }
            if p.layer == "prop" {
                assert_eq!(r.bottom(), 560, "{}: props stand on the back line", p.piece);
            }
        }
        let placed: std::collections::BTreeSet<&str> =
            d.place.iter().map(|p| p.piece.as_str()).collect();
        assert!(placed.contains("heat_scorch_wall_a"));
        assert!(placed.contains("heat_bumper"));
    }

    /// The step fronts lie exactly on the room's steps: the entry shelf and
    /// the dock in 16 px tiles (the shelf with end caps), the bumper once.
    #[test]
    fn the_step_fronts_lie_on_the_room_steps() {
        let run = crate::f01_corral::launch(root()).unwrap();
        let on = |id: &str, x0: i32, x1: i32, y: i32| -> Vec<IRect> {
            let mut v: Vec<IRect> = run
                .kit
                .pieces
                .iter()
                .filter(|d| d.piece == id && d.y == y && d.x >= x0 && d.x < x1)
                .map(rect_of)
                .collect();
            v.sort_by_key(|r| r.x);
            v
        };
        let mut covered = 0;
        for s in &run.steps {
            // The step rects are the walkable tops: 548 for the shelf and the dock, 500 for the bumper.
            let y = s.y;
            if s.w == 32 && s.h == 80 {
                let b = on("heat_bumper", s.x, s.right(), y);
                assert_eq!(b.len(), 1, "{s:?}");
                assert_eq!((b[0].x, b[0].w, b[0].h), (s.x, 32, 80));
                covered += 1;
                continue;
            }
            let mut tiles: Vec<IRect> = ["heat_shelf_mid_a", "heat_shelf_mid_b"]
                .iter()
                .flat_map(|id| on(id, s.x, s.right(), y))
                .chain(on("heat_shelf_end_l", s.x, s.right(), y))
                .chain(on("heat_shelf_end_r", s.x, s.right(), y))
                .collect();
            tiles.sort_by_key(|r| r.x);
            let mut at = s.x;
            for t in &tiles {
                assert_eq!(t.x, at, "{s:?}: a gap or overlap in the front");
                assert_eq!(t.h, 32);
                at = t.right();
            }
            assert_eq!(at, s.right(), "{s:?}: the front spans the step");
            covered += 1;
        }
        assert_eq!(covered, 3, "the entry shelf, the bumper and the dock");
    }

    /// The state hash is equal on every tick with the plain kit and the
    /// dressed kit (the dressing is presentation only).
    #[test]
    fn the_dressing_does_not_change_the_state_hash() {
        use crate::corridor::SceneSim;
        let hashes = |dressed: bool| -> Vec<u64> {
            let run = crate::f01_corral::launch_with(root(), dressed).unwrap();
            let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
            let mut combat = crate::f01_corral::combat_of(&run, &mut scene).unwrap();
            let mut out = Vec::new();
            for tick in 1..=crate::f01_corral::SMOKE_TICKS {
                combat.step(&mut scene, run.fixture.input(tick));
                out.push(combat.state_hash(&scene));
            }
            out
        };
        let plain = hashes(false);
        assert_eq!(plain.len() as u64, crate::f01_corral::SMOKE_TICKS);
        assert_eq!(hashes(true), plain, "the dressed kit");
    }

    /// No new light emitter: the room's emitters (the torches) are the same,
    /// in the same order and places, and no parallax strip changes.
    #[test]
    fn the_light_emitter_count_is_unchanged() {
        let (plain, dressed) = kits();
        let emitters = |kit: &EnvKit| {
            let scene = kit.lighting.as_ref().expect("the room is lit");
            scene
                .emitters
                .iter()
                .map(|e| format!("{} {:?} {:?}", e.id, (e.x, e.y), e.strip))
                .collect::<Vec<_>>()
        };
        let (p, d) = (emitters(&plain), emitters(&dressed));
        assert!(!p.is_empty());
        assert_eq!(d, p);
        let strips = |kit: &EnvKit| {
            kit.strips
                .iter()
                .map(|s| (s.layer.clone(), s.piece.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(strips(&dressed), strips(&plain));
    }

    /// Evidence helper (ignored, writes only under `HEAT_LAYOUT_DIR`): the
    /// composed layout of room 0, plain and dressed, for
    /// `scripts/measure_heat01.py` (the Python lit renderer of the kit).
    ///
    /// The partition's source grid: the three 40 x 48 frames side by side.
    fn partition_rows() -> Vec<Vec<char>> {
        let grid: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                root().join("assets/source/props/floor01/heat_partition.grid.json"),
            )
            .unwrap(),
        )
        .unwrap();
        grid["frames"][0]["cels"]["main"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_str().unwrap().chars().collect())
            .collect()
    }

    /// `CD-AD1`: the art's solid core sits on the collider. The sidecar's partition
    /// solid is x 1392..1408; the canvas is 40 px wide with the core in columns
    /// 12..27 (spec section 6), so the canvas's top-left is the collider's minus
    /// the 12 px margin: (1380, 532), where `HEAT-AD1` says the frames swap. The
    /// margin (rope tails, straw, debris, the sill) is visual only.
    #[test]
    fn the_partition_art_core_sits_exactly_on_the_collider() {
        let run = crate::f01_corral::launch(root()).unwrap();
        let prop = run
            .loaded
            .encounter
            .props
            .iter()
            .find(|p| p.kind == crate::encounter::PropKind::Partition)
            .expect("the corral sidecar has its partition");
        let target = prop.target;
        assert_eq!((target.x, target.w, target.h), (1392, 16, 48));
        assert_eq!(target.bottom(), run.loaded.room.floor_y);
        assert_eq!(partition_origin(target), (1380, 532));
        let rows = partition_rows();
        assert_eq!(rows.len(), PARTITION_CANVAS.1 as usize);
        let (core_l, core_r) = (PARTITION_CORE.0 as usize, PARTITION_CORE.1 as usize);
        assert_eq!(core_r - core_l, target.w as usize, "16 core columns");
        let opaque = |state: usize, row: usize, col: usize| {
            rows[row][state * PARTITION_CANVAS.0 as usize + col] != '.'
        };
        for state in 0..2 {
            // The standing screen: its cap row is the core exactly, and every row
            // of the 48 px has core pixels (no gap in the drawn screen).
            let cap: Vec<usize> = (0..PARTITION_CANVAS.0 as usize)
                .filter(|&c| opaque(state, 0, c))
                .collect();
            assert_eq!(
                (cap.first().copied(), cap.last().copied()),
                (Some(core_l), Some(core_r - 1)),
                "state {state}: the cap is the collider's 16 columns"
            );
            for row in 0..48 {
                assert!(
                    (core_l..core_r).any(|c| opaque(state, row, c)),
                    "state {state} row {row}: core pixels"
                );
            }
        }
        // The broken frame: the remnant's flat top is canvas row 32, i.e. y 564 =
        // the sidecar remnant's top, across the whole core (HEAT-AD3).
        let remnant = prop.remnant.expect("a partition has its remnant");
        assert_eq!(remnant.y, partition_origin(target).1 + 32, "HEAT-AD3");
        assert!(
            (core_l..core_r).all(|c| opaque(2, 32, c)),
            "the flat top spans the core"
        );
        for row in 0..29 {
            assert!(
                (0..PARTITION_CANVAS.0 as usize).all(|c| !opaque(2, row, c)),
                "broken row {row}: nothing above the splinters"
            );
        }
    }

    /// The three state frames come from the contract in state order and the
    /// presenter's frame is a pure function of the prop's state.
    #[test]
    fn the_partition_frames_follow_the_prop_state() {
        use crate::env_prop::PartitionState::{Broken, Intact, Weakened};
        assert_eq!(partition_piece(Intact), "heat_partition_intact");
        assert_eq!(partition_piece(Weakened), "heat_partition_weakened");
        assert_eq!(partition_piece(Broken), "heat_partition_broken");
        let rects = partition_atlas_rects(root()).unwrap();
        assert_eq!(
            rects,
            [[0, 0, 40, 48], [40, 0, 40, 48], [80, 0, 40, 48]],
            "one 40 x 48 canvas per state, side by side"
        );
    }

    /// The presenter adds nothing to the simulation: the partition is not a kit
    /// piece of the room, and the dressed and plain launch keep the same prop
    /// state, solids and hash (the dressing never touched it, `HEAT-AD1`).
    #[test]
    fn the_dressing_still_does_not_place_the_partition() {
        let (_, dressed) = kits();
        assert!(
            dressed
                .pieces
                .iter()
                .all(|p| !p.piece.starts_with("heat_partition"))
        );
    }

    /// `HEAT_LAYOUT_DIR=<dir> cargo test --lib -- --ignored dump_heat_layouts`
    #[test]
    #[ignore = "evidence helper: writes the composed layouts for the Python measurements"]
    fn dump_heat_layouts() {
        let Ok(dir) = std::env::var("HEAT_LAYOUT_DIR") else {
            return;
        };
        std::fs::create_dir_all(&dir).unwrap();
        let run = crate::f01_corral::launch_with(root(), false).unwrap();
        for dressed in [false, true] {
            let contract = load_contract(root(), HEAT_CONTRACT).unwrap();
            let d = dressing(root()).unwrap();
            let mut out = None;
            crate::f01_boiler::compose_kit_hooked(
                root(),
                &run.loaded.room,
                &run.steps,
                Some(contract),
                |layout| {
                    if dressed {
                        assert_eq!(dress_with(layout, &d, HEAT_CONTRACT), 0);
                    }
                    layout["kit_contract"] = json!(HEAT_CONTRACT);
                    out = Some(layout.clone());
                    Ok(())
                },
            )
            .unwrap();
            let name = format!(
                "corral.{}.layout.json",
                if dressed { "dressed" } else { "plain" }
            );
            std::fs::write(
                Path::new(&dir).join(name),
                serde_json::to_string(&out.unwrap()).unwrap(),
            )
            .unwrap();
        }
    }
}
