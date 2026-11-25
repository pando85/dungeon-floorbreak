//! AREA-GYM-01: the gym dressing of the Floor 01 boss arena (room 5). Pure and
//! headless, no Bevy types.
//!
//! The arena is composed from the catacomb kit by [`crate::camera_lab::layout`]
//! exactly as before; this module then applies the `arena_dressing` block of
//! the gym kit contract (`floor01_gym_kit.json`, which extends the opening kit
//! and through it ENV-01): it removes the reused catacomb pieces the gym does
//! not want (`door_timber`, the `guild_portal_closed` exit marker) and appends
//! the gym pieces per layer. Pieces never produce solids (only collidable tiles
//! of the collision layer do), so the dressing is **presentation only**: the
//! tests below prove the solids, spawn, exit, views, walking line and blink
//! barriers are identical with and without it. Spec:
//! `docs/02-art/area-gym-01-spec.md`.

use std::path::Path;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::camera_lab::layout::{BASE_LAYOUT, compose};
use crate::camera_lab::room::Room;
use crate::env_kit::{EnvError, EnvKit, Layout, load_contract};

/// The gym kit contract (extends the opening kit, which extends ENV-01).
pub const GYM_CONTRACT: &str = "assets/source/terrain/floor01/floor01_gym_kit.json";

/// One placement of the dressing (layout px, top-left).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Placement {
    pub layer: String,
    pub piece: String,
    pub x: i32,
    pub y: i32,
}

/// The contract's `arena_dressing` block.
#[derive(Debug, Clone, Deserialize)]
pub struct Dressing {
    pub remove: Vec<String>,
    pub place: Vec<Placement>,
}

fn read_json(root: &Path, rel: &str) -> Result<Value, EnvError> {
    let path = root.join(rel);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| EnvError::Io(format!("{}: {e}", path.display())))?;
    serde_json::from_str(&text).map_err(|e| EnvError::Parse(format!("{}: {e}", path.display())))
}

/// The dressing block of the gym contract.
pub fn dressing(root: &Path) -> Result<Dressing, EnvError> {
    let contract = read_json(root, GYM_CONTRACT)?;
    serde_json::from_value(contract["arena_dressing"].clone())
        .map_err(|e| EnvError::Parse(format!("{GYM_CONTRACT} arena_dressing: {e}")))
}

/// Apply `dressing` to a composed `env-kit-layout-v2` value: drop the removed
/// pieces from every piece layer, then append the placements in order.
pub fn dress(layout: &mut Value, dressing: &Dressing) {
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
    for p in &dressing.place {
        let list = &mut layout["piece_layers"][p.layer.as_str()];
        if !list.is_array() {
            *list = json!([]);
        }
        if let Some(list) = list.as_array_mut() {
            list.push(json!({"piece": p.piece, "x": p.x, "y": p.y}));
        }
    }
    layout["kit_contract"] = json!(GYM_CONTRACT);
}

/// The env kit of the arena room: the catacomb composition with (`dressed`) or
/// without the gym dressing. Both load the same merged contract.
/// The undressed kit is built from the gym contract too, not from main's
/// `camera_lab::layout::build_kit` path; its equivalence with main rests on the
/// unchanged boss goldens.
pub fn build_arena_kit(root: &Path, room: &Room, dressed: bool) -> Result<EnvKit, EnvError> {
    let contract = load_contract(root, GYM_CONTRACT)?;
    let base = read_json(root, BASE_LAYOUT)?;
    let (mut value, _plan) = compose(room, &base);
    if dressed {
        dress(&mut value, &dressing(root)?);
    }
    let layout: Layout =
        serde_json::from_value(value).map_err(|e| EnvError::Parse(e.to_string()))?;
    EnvKit::build(&contract, &layout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss_scene;
    use crate::env_kit::IRect;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn rect(kit: &EnvKit, piece: &str) -> Vec<IRect> {
        kit.pieces
            .iter()
            .filter(|p| p.piece == piece)
            .map(|p| IRect {
                x: p.x,
                y: p.y,
                w: p.rect.w,
                h: p.rect.h,
            })
            .collect()
    }

    fn overlaps(a: IRect, b: IRect) -> bool {
        a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
    }

    #[test]
    fn the_dressing_changes_no_collider_spawn_exit_view_or_walk_line() {
        let dressed = boss_scene::load_arena(root()).unwrap();
        let plain = boss_scene::load_arena_undressed(root()).unwrap();
        assert_eq!(dressed.solid_rects, plain.solid_rects);
        assert_eq!(dressed.spawn, plain.spawn);
        assert_eq!(dressed.exit, plain.exit);
        assert_eq!(dressed.views, plain.views);
        assert_eq!(dressed.base_walk_line_y, plain.base_walk_line_y);
        assert_eq!(dressed.blink_barriers, plain.blink_barriers);
        assert_eq!((dressed.width, dressed.height), (plain.width, plain.height));
        assert_eq!(dressed.collidable_cells, plain.collidable_cells);
        assert_eq!(dressed.tiles.len(), plain.tiles.len());
        assert_eq!(dressed.actor_z, plain.actor_z);
        // The dressing did happen: the gym pieces are there, the catacomb door
        // and the guild portal are not.
        assert!(!rect(&dressed, "gym_mirror_bay").is_empty());
        assert!(rect(&dressed, "door_timber").is_empty());
        assert!(rect(&dressed, "guild_portal_closed").is_empty());
        assert!(!rect(&plain, "guild_portal_closed").is_empty());
    }

    /// Spec section 2 / acceptance 4: the crack lands on mirror glass and no
    /// prop covers it (the crack is drawn over props).
    #[test]
    fn the_mirror_crack_zones_lie_on_glass_and_no_prop_covers_them() {
        let kit = boss_scene::load_arena(root()).unwrap();
        let walk = kit.base_walk_line_y;
        assert_eq!(walk, 420);
        // mirror_crack frame 2 content around the pivot (wall x, walk - 40).
        let right = IRect {
            x: 597,
            y: walk - 58,
            w: 25,
            h: 34,
        };
        let left_visible = IRect {
            x: 30,
            y: walk - 58,
            w: 14,
            h: 34,
        };
        // The bay's glass: columns 1..94 and rows 1..69 of the 96 x 74 piece.
        let glass: Vec<IRect> = rect(&kit, "gym_mirror_bay")
            .into_iter()
            .map(|b| IRect {
                x: b.x + 1,
                y: b.y + 1,
                w: 94,
                h: 69,
            })
            .collect();
        for zone in [right, left_visible] {
            assert!(
                glass.iter().any(|g| g.x <= zone.x
                    && g.right() >= zone.right()
                    && g.y <= zone.y
                    && g.bottom() >= zone.bottom()),
                "crack zone {zone:?} is not on mirror glass"
            );
            for p in kit.pieces.iter().filter(|p| p.layer == "prop") {
                let r = IRect {
                    x: p.x,
                    y: p.y,
                    w: p.rect.w,
                    h: p.rect.h,
                };
                if crate::gym_art::piece_state(&p.piece).is_some() {
                    // GYM-AD2: the exit door in the end-wall strip may touch the
                    // largest crack frame (to x 624) by at most 4 px.
                    assert!(r.x >= 620, "{} reaches into the crack zone", p.piece);
                    continue;
                }
                assert!(!overlaps(r, zone), "{} covers the crack zone", p.piece);
            }
        }
    }

    /// Acceptance 5: props stand on the floor back line, nothing crosses the
    /// walking line, the mats leave the floor's back row and lip visible, and the
    /// girder's bottom row is the ceiling collider line.
    #[test]
    fn props_mats_and_girder_respect_the_floor_and_ceiling_lines() {
        let kit = boss_scene::load_arena(root()).unwrap();
        let walk = kit.base_walk_line_y;
        let back = walk - 20;
        // Standing pieces never cross the walking line; the mats lie on the floor
        // surface and keep its back row (400) and the lip row (423) visible.
        for p in kit.pieces.iter().filter(|p| p.piece.starts_with("gym_")) {
            if p.piece.starts_with("gym_mat_") {
                assert!(p.y > back && p.y + p.rect.h < walk + 4, "{}", p.piece);
            } else {
                assert!(
                    p.y + p.rect.h <= walk,
                    "{} crosses the walking line",
                    p.piece
                );
            }
        }
        for id in [
            "gym_reception_desk",
            "gym_turnstile",
            "gym_dumbbell_rack",
            "gym_bench",
            "gym_plate_tree",
            "gym_cable_tower",
            "gym_tyres",
        ] {
            let r = rect(&kit, id);
            assert_eq!(r.len(), 1, "{id}");
            assert_eq!(r[0].bottom(), back, "{id} stands on the floor back line");
        }
        let ceiling = walk - 224;
        for g in rect(&kit, "gym_girder") {
            assert_eq!(
                g.bottom(),
                ceiling,
                "the girder's bottom row is the ceiling line"
            );
        }
        let lintel = rect(&kit, "gym_gate_lintel")[0];
        assert!(lintel.x == 0 && lintel.y < ceiling && lintel.bottom() >= ceiling);
    }

    #[test]
    fn the_gym_lights_stay_inside_the_light_budget() {
        use crate::lighting::{MAX_LIGHTS, ViewRect};
        let kit = boss_scene::load_arena(root()).unwrap();
        let scene = kit.lighting.as_ref().unwrap();
        let view = kit.views["abc_640x400"];
        let (g, dropped) = scene.gather(
            &kit,
            ViewRect {
                left: view.x,
                top: view.y,
                w: view.w,
                h: view.h,
            },
            0,
        );
        assert_eq!(dropped, 0);
        assert!(g.lights.len() <= MAX_LIGHTS);
        assert!(g.lights.len() >= 6, "the three tube fixtures emit");
    }
}
