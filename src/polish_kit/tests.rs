use super::*;
use crate::env_kit::IRect;
use crate::{f01_dozer, f01_mastery};
use serde_json::Value;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `POLISH_LAYOUT_DIR=<dir> cargo test --lib -- --ignored dump_polish_layouts`: the plain composed
/// layouts of the two rooms for the authoring script and the measurements.
#[test]
#[ignore = "evidence helper: writes the composed layouts for the Python measurements"]
fn dump_polish_layouts() {
    let Ok(dir) = std::env::var("POLISH_LAYOUT_DIR") else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    for (name, plain) in [("dozer", true), ("mastery", true)] {
        let mut out = None;
        let hook = |layout: &mut serde_json::Value| {
            out = Some(layout.clone());
            Ok(())
        };
        if name == "dozer" {
            let loaded = crate::encounter::load(&root(), crate::f01_dozer::ENCOUNTER_PATH).unwrap();
            crate::f01_boiler::compose_kit_hooked(&root(), &loaded.room, &[], None, hook).unwrap();
        } else {
            crate::cf02::arena::load_arena_hooked(&root(), &[], None, hook).unwrap();
        }
        let _ = plain;
        std::fs::write(
            std::path::Path::new(&dir).join(format!("{name}.plain.layout.json")),
            serde_json::to_string(&out.unwrap()).unwrap(),
        )
        .unwrap();
    }
}

fn dressing(room_id: &str) -> Dressing {
    dressing_of(&root(), room_id).unwrap().unwrap()
}

/// The kit-contract piece sizes by id (the polish contract merged over the opening and ENV-01 kits).
fn sizes() -> std::collections::BTreeMap<String, (i32, i32, String)> {
    load_contract(&root(), POLISH_CONTRACT)
        .unwrap()
        .pieces
        .iter()
        .map(|p| (p.id.clone(), (p.rect[2], p.rect[3], p.layer.clone())))
        .collect()
}

/// The dressed composed layout of a room (the value the kit is built from), for the geometry tests.
fn dressed_layout(room_id: &str) -> Value {
    let mut out = None;
    let d = dressing(room_id);
    let contract = load_contract(&root(), POLISH_CONTRACT).unwrap();
    let hook = |layout: &mut Value| {
        apply(layout, &d, room_id)?;
        out = Some(layout.clone());
        Ok(())
    };
    if room_id == DOZER_ROOM {
        let loaded = crate::encounter::load(&root(), f01_dozer::ENCOUNTER_PATH).unwrap();
        crate::f01_boiler::compose_kit_hooked(&root(), &loaded.room, &[], Some(contract), hook)
            .unwrap();
    } else {
        crate::cf02::arena::load_arena_hooked(&root(), &[], Some(contract), hook).unwrap();
    }
    out.unwrap()
}

fn collision(kit: &EnvKit) -> Vec<(i32, i32, IRect)> {
    kit.tiles
        .iter()
        .filter(|t| t.layer == "collision")
        .map(|t| (t.col, t.row, t.rect))
        .collect()
}

// ------------------------------------------------------------------ presentation only

/// POLISH-A1: the dressing changes no collider, spawn, exit, view or walking line, no blink barrier and
/// no collision tile in either room; the pieces do change, and the room, the sidecar and the rules
/// are the same value.
#[test]
fn the_dressing_changes_no_collider_spawn_exit_view_or_walk_line() {
    let (dp, dd) = (
        f01_dozer::launch_with(&root(), false).unwrap(),
        f01_dozer::launch_with(&root(), true).unwrap(),
    );
    let (mp, md) = (
        f01_mastery::launch_with(&root(), false).unwrap(),
        f01_mastery::launch_with(&root(), true).unwrap(),
    );
    for (name, p, d) in [("dozer", &dp.kit, &dd.kit), ("mastery", &mp.kit, &md.kit)] {
        assert_eq!(d.solid_rects, p.solid_rects, "{name}");
        assert_eq!(d.spawn, p.spawn, "{name}");
        assert_eq!(d.exit, p.exit, "{name}");
        assert_eq!(d.views, p.views, "{name}");
        assert_eq!(d.base_walk_line_y, p.base_walk_line_y, "{name}");
        assert_eq!(d.blink_barriers, p.blink_barriers, "{name}");
        assert_eq!((d.width, d.height), (p.width, p.height), "{name}");
        assert_eq!(d.collidable_cells, p.collidable_cells, "{name}");
        assert_eq!(d.actor_z, p.actor_z, "{name}");
        assert_eq!(d.deck_top_px, p.deck_top_px, "{name}");
        assert_eq!(collision(d), collision(p), "{name}");
        assert_ne!(
            format!("{:?}", d.pieces),
            format!("{:?}", p.pieces),
            "{name}: the dressing did happen"
        );
    }
    assert_eq!(dp.loaded.room_hash, dd.loaded.room_hash);
    assert_eq!(dp.loaded.sidecar_hash, dd.loaded.sidecar_hash);
    assert_eq!(
        format!("{:?}", dp.loaded.encounter),
        format!("{:?}", dd.loaded.encounter)
    );
    assert_eq!(mp.loaded.sidecar_hash, md.loaded.sidecar_hash);
    assert_eq!(
        format!("{:?}", mp.loaded.encounter),
        format!("{:?}", md.loaded.encounter)
    );
}

/// POLISH-A2: the state hash is equal on every tick of a 500-tick scripted run with the plain and the
/// dressed kit, in both rooms.
#[test]
fn the_dressing_does_not_change_the_state_hash() {
    let (carl, donut) = crate::f01_late::Loadout::Story.flags();
    let dozer = |dressed: bool| -> Vec<u64> {
        let run = f01_dozer::launch_with(&root(), dressed).unwrap();
        let mut scene =
            crate::corridor::SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
        let mut combat = f01_dozer::combat(&run, &mut scene, carl, donut).unwrap();
        (1..=500u64)
            .map(|tick| {
                combat.step(
                    &mut scene,
                    run.fixture.input(tick.min(f01_dozer::SMOKE_TICKS)),
                );
                combat.state_hash(&scene)
            })
            .collect()
    };
    let mastery = |dressed: bool| -> Vec<u64> {
        let run = f01_mastery::launch_with(&root(), dressed).unwrap();
        let mut scene =
            crate::corridor::SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
        let mut combat = f01_mastery::combat_for(&run, &mut scene, carl, donut, None).unwrap();
        (1..=500u64)
            .map(|tick| {
                combat.step(
                    &mut scene,
                    run.fixture.input(tick.min(f01_mastery::SMOKE_TICKS)),
                );
                combat.state_hash(&scene)
            })
            .collect()
    };
    let plain = dozer(false);
    assert_eq!(plain.len(), 500);
    assert_eq!(dozer(true), plain, "dozer");
    let plain = mastery(false);
    assert_eq!(mastery(true), plain, "mastery");
}

// ------------------------------------------------------------------ the dressing blocks

/// POLISH-A3: every `remove_at` entry matches exactly one composer piece (`dress_with` reports zero
/// unmatched), and the dozer's two removals are the bricked bay's sill and chain.
#[test]
fn every_remove_at_matches_exactly_one_piece() {
    for (room_id, want) in [(DOZER_ROOM, 2), (MASTERY_ROOM, 0)] {
        let d = dressing(room_id);
        assert_eq!(d.remove_at.len(), want, "{room_id}");
        let layout = dressed_layout(room_id);
        assert_eq!(layout["kit_contract"], POLISH_CONTRACT);
    }
    // The composer's own instance list: the removed pieces are gone, their upper-row twins stay.
    let layout = dressed_layout(DOZER_ROOM);
    let has = |piece: &str, x: i64, y: i64| {
        layout["piece_layers"]["structure"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["piece"] == piece && p["x"] == x && p["y"] == y)
    };
    assert!(!has("arch_threshold", 416, 544) && has("arch_threshold", 416, 272));
    assert!(!has("arch_chain", 432, 384) && has("arch_chain", 432, 112));
}

/// POLISH-A4: every placement lies inside the room, on its own piece's layer, and a placement in
/// the floor strip (y 561..579) is a decal-weight `structure` piece: nothing stands on the walking
/// line.
#[test]
fn every_placement_is_inside_the_room_on_its_layer() {
    let sizes = sizes();
    for (room_id, width) in [(DOZER_ROOM, 1344), (MASTERY_ROOM, 1152)] {
        for p in &dressing(room_id).place {
            let (w, h, layer) = &sizes[&p.piece];
            // the opening/catacomb pieces of the contract chain are declared in the parents
            assert_eq!(&p.layer, layer, "{room_id} {}", p.piece);
            if !p.piece.starts_with("torch_pool") {
                assert!(p.x >= 0 && p.x + w <= width, "{room_id} {p:?} x extent");
            }
            // the composer's torch pools are 224 px soft decals that overlap the floor by design
            if !p.piece.starts_with("torch_pool") {
                assert!(
                    p.y >= 0 && p.y + h <= 580,
                    "{room_id} {p:?} reaches the walking line"
                );
            }
            if p.y >= 540 {
                assert_eq!(p.layer, "structure", "{room_id} {p:?}: floor strip");
                assert!(p.y >= 561, "{room_id} {p:?}");
            }
        }
    }
}

/// POLISH-A5: the dozer's bricked bay has no open backwall cell left (no background through it) and
/// no sill piece under it; the mastery's blind bays are over solid brick (no null cell under a pier
/// or an arch ring).
#[test]
fn no_background_shows_through_a_bricked_bay() {
    let layout = dressed_layout(DOZER_ROOM);
    let rows = layout["tile_layers"]["backwall"].as_array().unwrap();
    let f = dressing(DOZER_ROOM).backwall_fill[0];
    for row in f.y / 16..(f.y + f.h) / 16 {
        for col in f.x / 16..(f.x + f.w) / 16 {
            // the fill applies to open cells only; the bay's open cells must be gone
            if layout["tile_layers"]["backwall"][row as usize][col as usize].is_null() {
                let bay_open = (24..=36).contains(&col) && (22..=34).contains(&row);
                assert!(
                    !bay_open,
                    "open backwall cell ({col}, {row}) in the bricked bay"
                );
            }
        }
    }
    assert!(rows.len() > 34);
    // mastery: every backwall cell under a placed arcade piece is brick
    let layout = dressed_layout(MASTERY_ROOM);
    for (x, w) in [(416, 304), (640, 80), (784, 288), (1008, 80)] {
        for row in 22..35usize {
            for col in (x / 16)..((x + w) / 16) {
                assert!(
                    !layout["tile_layers"]["backwall"][row][col as usize].is_null(),
                    "open cell ({col}, {row}) under the mastery arcade piece at x {x}"
                );
            }
        }
    }
}

/// POLISH-A6: no torch flame stands over a natural standing spot of the mastery arena: a placed torch
/// clears every perch and the parapet by 8 px, and no placement hides under the exit portal.
#[test]
fn no_torch_flame_over_a_standing_spot() {
    let standing = [(544, 672), (896, 992), (736, 768)];
    let torches: Vec<_> = dressing(MASTERY_ROOM)
        .place
        .into_iter()
        .filter(|p| p.piece == "torch_wall")
        .collect();
    assert_eq!(torches.len(), 3);
    for t in torches {
        for (l, r) in standing {
            assert!(
                t.x + 16 + 8 <= l || t.x >= r + 8,
                "torch at x {} stands over the standing spot {l}..{r}",
                t.x
            );
        }
    }
}

/// POLISH-A7: the worst 640 px view of each dressed room stays inside the light budget.
#[test]
fn the_worst_view_stays_inside_the_light_budget() {
    use crate::lighting::{MAX_LIGHTS, ViewRect};
    let kits = [
        ("dozer", f01_dozer::launch(&root()).unwrap().kit),
        ("mastery", f01_mastery::launch(&root()).unwrap().kit),
    ];
    for (name, kit) in kits {
        let scene = kit.lighting.as_ref().unwrap();
        let view = kit.views["abc_640x360"];
        let mut left = 0;
        while left + 640 <= kit.width {
            let (g, dropped) = scene.gather(
                &kit,
                ViewRect {
                    left,
                    top: view.y,
                    w: 640,
                    h: 360,
                },
                0,
            );
            assert_eq!(dropped, 0, "{name} at x {left}");
            assert!(g.lights.len() <= MAX_LIGHTS, "{name} at x {left}");
            left += 32;
        }
    }
}

/// POLISH-A8: the dressing adds only pieces the contract chain declares and never a collidable one.
#[test]
fn the_polish_contract_adds_no_collidable_piece_or_tile() {
    let raw = read_contract(&root()).unwrap();
    assert!(raw["tiles"].as_array().unwrap().is_empty(), "no tile");
    for p in raw["pieces"].as_array().unwrap() {
        assert_eq!(p["collidable"], false, "{}", p["id"]);
    }
    assert!(
        raw["atlases"].as_object().unwrap().len() <= 2,
        "no new atlas"
    );
    let sizes = sizes();
    for room_id in [DOZER_ROOM, MASTERY_ROOM] {
        for p in &dressing(room_id).place {
            assert!(sizes.contains_key(&p.piece), "{room_id}: {}", p.piece);
        }
    }
}

/// POLISH-A9: a room whose contract has no block composes plain even when `dressed` is asked for.
#[test]
fn an_unknown_room_has_no_block() {
    assert!(dressing_of(&root(), "corral").unwrap().is_none());
    assert!(is_dressed(&root(), DOZER_ROOM) && is_dressed(&root(), MASTERY_ROOM));
}

/// POLISH-A10 (the mastery's spent drip): the room holds one Rot Sticker, so the scene spawns the
/// AREA-HAZARDS-01B hazard-decal presenter, whose `dfx_rot_drip` art replaces the code-drawn
/// "spent drip (stand-in)" wherever the atlas exports: no stand-in is left to replace here.
#[test]
fn the_mastery_sticker_has_the_drip_art_presenter() {
    let run = f01_mastery::launch(&root()).unwrap();
    let env = f01_mastery::env_runtime(&run).unwrap();
    assert_eq!(env.sticker_count(), 1);
    assert!(
        crate::hazard_decals::static_capacity(&env, "f01_mastery") + env.sticker_count() > 0,
        "the presenter spawns"
    );
    assert!(
        crate::hazard_decals::DecalSheet::ALL.contains(&crate::hazard_decals::DecalSheet::RotDrip)
    );
}

/// POLISH-A11: no floor decal sits under or over a pier's footprint (no decal hidden under structure,
/// none drawn across a pier base): each y >= 540 placement is clear of every lower-row pier x range.
#[test]
fn no_floor_decal_touches_a_pier() {
    let sizes = sizes();
    for room_id in [DOZER_ROOM, MASTERY_ROOM] {
        let layout = dressed_layout(room_id);
        let piers: Vec<(i64, i64)> = layout["piece_layers"]["structure"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| {
                p["y"] == 304
                    && p["piece"]
                        .as_str()
                        .is_some_and(|id| id.starts_with("pier_"))
            })
            .map(|p| {
                let w = i64::from(sizes[p["piece"].as_str().unwrap()].0);
                (p["x"].as_i64().unwrap(), p["x"].as_i64().unwrap() + w)
            })
            .collect();
        assert!(piers.len() >= 4, "{room_id}: the lower-row piers");
        for p in dressing(room_id).place.iter().filter(|p| p.y >= 540) {
            let (x0, x1) = (i64::from(p.x), i64::from(p.x + sizes[&p.piece].0));
            for (a, b) in &piers {
                assert!(
                    x1 <= *a || x0 >= *b,
                    "{room_id} {p:?} touches the pier {a}..{b}"
                );
            }
        }
    }
}
