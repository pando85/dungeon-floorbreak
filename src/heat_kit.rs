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
//! CORRAL-D's presenter draws them. Spec: `docs/02-art/area-heat-01-spec.md`.

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
