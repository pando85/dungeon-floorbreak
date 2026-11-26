use super::*;
use crate::env_kit::IRect;
use crate::f01_late::Loadout;
use crate::f01_rest::{self, RestRoom};
use crate::rest_fixtures::{BOT, Party, RestSim};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn contract_path(room: RestRoom) -> &'static str {
    match room {
        RestRoom::Camp => CAMP_CONTRACT,
        _ => AFTERMATH_CONTRACT,
    }
}

fn dressing_of(room: RestRoom) -> Dressing {
    match room {
        RestRoom::Camp => camp_dressing(&root()).unwrap(),
        _ => aftermath_dressing(&root()).unwrap(),
    }
}

const ROOMS: [RestRoom; 2] = [RestRoom::Camp, RestRoom::Aftermath];

/// The room's composition without and with the dressing, before the launch's own pushes.
fn kits(room: RestRoom) -> (EnvKit, EnvKit) {
    let run = f01_rest::launch_with(&root(), room, false).unwrap();
    let plain = crate::f01_boiler::compose_kit(&root(), &run.loaded.room).unwrap();
    let dressed = match room {
        RestRoom::Camp => compose_camp_kit(&root(), &run.loaded.room),
        _ => compose_aftermath_kit(&root(), &run.loaded.room),
    }
    .unwrap();
    (plain, dressed)
}

// ------------------------------------------------------------------ CAMP-A1: presentation only

/// CAMP-A1: the dressing changes no collider, spawn, exit, view or walking line, no blink
/// barrier and no collision cell, and adds no collision tile.
#[test]
fn the_dressing_changes_no_collider_spawn_exit_view_or_walk_line() {
    for room in ROOMS {
        let (plain, dressed) = kits(room);
        assert_eq!(dressed.solid_rects, plain.solid_rects, "{room:?}");
        assert_eq!(dressed.spawn, plain.spawn, "{room:?}");
        assert_eq!(dressed.exit, plain.exit, "{room:?}");
        assert_eq!(dressed.views, plain.views, "{room:?}");
        assert_eq!(dressed.base_walk_line_y, plain.base_walk_line_y, "{room:?}");
        assert_eq!(dressed.blink_barriers, plain.blink_barriers, "{room:?}");
        assert_eq!((dressed.width, dressed.height), (plain.width, plain.height));
        assert_eq!(dressed.collidable_cells, plain.collidable_cells, "{room:?}");
        assert_eq!(dressed.actor_z, plain.actor_z);
        assert_eq!(dressed.deck_top_px, plain.deck_top_px);
        let collision = |kit: &EnvKit| {
            kit.tiles
                .iter()
                .filter(|t| t.layer == "collision")
                .map(|t| (t.col, t.row, t.rect))
                .collect::<Vec<_>>()
        };
        assert_eq!(collision(&dressed), collision(&plain), "{room:?}");
        // The dressing did happen.
        assert!(dressed.pieces.len() != plain.pieces.len(), "{room:?}");
    }
}

/// The launched rooms, dressed and plain: the same solids, spawn, exit, views, walking
/// line, checkpoint columns, the same room and sidecar hashes and the sidecar's inert
/// `hit_target` (the aftermath's heavy bag, REST-IMP-9).
#[test]
fn the_launched_rooms_are_the_same_rooms_with_and_without_the_dressing() {
    for room in ROOMS {
        let plain = f01_rest::launch_with(&root(), room, false).unwrap();
        let dressed = f01_rest::launch(&root(), room).unwrap();
        assert_eq!(dressed.kit.solid_rects, plain.kit.solid_rects, "{room:?}");
        assert_eq!(dressed.kit.spawn, plain.kit.spawn);
        assert_eq!(dressed.kit.exit, plain.kit.exit);
        assert_eq!(dressed.kit.views, plain.kit.views);
        assert_eq!(dressed.kit.base_walk_line_y, plain.kit.base_walk_line_y);
        assert_eq!(dressed.kit.blink_barriers, plain.kit.blink_barriers);
        assert_eq!(dressed.rules.checkpoints, plain.rules.checkpoints);
        assert_eq!(dressed.loaded.room_hash, plain.loaded.room_hash);
        assert_eq!(dressed.loaded.sidecar_hash, plain.loaded.sidecar_hash);
        assert_eq!(
            format!("{:?}", dressed.loaded.encounter.statics),
            format!("{:?}", plain.loaded.encounter.statics),
            "{room:?}: the sidecar's statics"
        );
        assert_ne!(
            format!("{:?}", dressed.kit.pieces),
            format!("{:?}", plain.kit.pieces),
            "{room:?}: the room's own default is the dressed one"
        );
    }
    let bag = f01_rest::launch(&root(), RestRoom::Aftermath).unwrap();
    let statics = &bag.loaded.encounter.statics;
    assert_eq!(statics.len(), 1);
    assert_eq!(statics[0].id, "heavy_bag");
    assert_eq!(
        (
            statics[0].rect.x,
            statics[0].rect.y,
            statics[0].rect.w,
            statics[0].rect.h
        ),
        BAG_RECT,
        "the presenter reads the sidecar's hit target rect"
    );
    assert_eq!(
        statics[0].kind,
        crate::encounter::StaticKind::HitTarget,
        "the bag stays an inert hit target"
    );
}

/// The dressing does not change the state hash: the camp and the aftermath fixtures hash the
/// same on every tick with the plain kit and the dressed kit.
#[test]
fn the_dressing_does_not_change_the_state_hash() {
    let (carl, donut) = Loadout::Story.flags();
    for room in ROOMS {
        let run_hashes = |dressed: bool| -> Vec<u64> {
            let run = f01_rest::launch_with(&root(), room, dressed).unwrap();
            let mut sim = RestSim::from_run(
                run,
                360,
                carl,
                donut,
                Party::Solo { uncontrolled: BOT },
                None,
            )
            .unwrap();
            let mut out = Vec::new();
            while sim.tick() < 360 && !sim.complete() {
                sim.step([
                    crate::input::RawInput {
                        move_x: 1.0,
                        ..crate::input::RawInput::NONE
                    },
                    crate::input::RawInput::NONE,
                ]);
                out.push(sim.hash());
            }
            out
        };
        let plain = run_hashes(false);
        assert!(plain.len() >= 300, "{room:?}: {} ticks", plain.len());
        assert_eq!(run_hashes(true), plain, "{room:?}");
    }
}

// ------------------------------------------------------------------ the dressing block

/// CAMP-A3: every `remove_at` entry matches (zero unmatched) and drops exactly one composer
/// piece in both rooms; an unmatched entry is an error, never skipped.
#[test]
fn every_remove_at_matches_exactly_one_piece() {
    for room in ROOMS {
        let d = dressing_of(room);
        let (plain, dressed) = kits(room);
        assert_eq!(
            d.remove_at.len(),
            if room == RestRoom::Camp { 8 } else { 2 },
            "{room:?}"
        );
        for r in &d.remove_at {
            let count = |kit: &EnvKit| {
                kit.pieces
                    .iter()
                    .filter(|p| {
                        p.layer == r.layer && p.piece == r.piece && p.x == r.x && p.y == r.y
                    })
                    .count()
            };
            let re_placed = d
                .place
                .iter()
                .any(|p| p.layer == r.layer && p.piece == r.piece && p.x == r.x && p.y == r.y);
            assert_eq!(count(&plain), 1, "{r:?} in the plain kit");
            assert_eq!(
                count(&dressed),
                usize::from(re_placed),
                "{r:?} dropped (the aftermath re-places the pool after the grooves)"
            );
        }
        // dress_with reports zero unmatched entries.
        let contract = load_contract(&root(), contract_path(room)).unwrap();
        let run = f01_rest::launch_with(&root(), room, false).unwrap();
        let mut missed = usize::MAX;
        crate::f01_boiler::compose_kit_hooked(
            &root(),
            &run.loaded.room,
            &[],
            Some(contract),
            |l| {
                missed = dress_with(l, &d, contract_path(room));
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(missed, 0, "{room:?}");
        // An unmatched entry is an error.
        let mut bad = dressing_of(room);
        bad.remove_at[0].x += 1;
        let contract = load_contract(&root(), contract_path(room)).unwrap();
        let err = crate::f01_boiler::compose_kit_hooked(
            &root(),
            &run.loaded.room,
            &[],
            Some(contract),
            |l| match dress_with(l, &bad, contract_path(room)) {
                0 => Ok(()),
                n => Err(format!("{n} unmatched")),
            },
        )
        .unwrap_err();
        assert!(err.contains("unmatched"), "{err}");
    }
    // The camp's four torches and pools go, the others stay; the aftermath's chain goes.
    let (_, camp) = kits(RestRoom::Camp);
    for (x, y) in [(48, 432), (128, 448), (424, 432), (464, 448), (1376, 448)] {
        assert!(
            camp.pieces
                .iter()
                .any(|p| p.piece == "torch_wall" && (p.x, p.y) == (x, y)),
            "the camp torch at ({x}, {y}) stays"
        );
    }
    let (_, aft) = kits(RestRoom::Aftermath);
    assert!(
        !aft.pieces
            .iter()
            .any(|p| p.piece == "arch_chain" && (p.x, p.y) == (560, 384)),
        "B2's chain is gone"
    );
    assert!(
        aft.pieces
            .iter()
            .any(|p| p.piece == "arch_chain" && (p.x, p.y) == (224, 384)),
        "B1 keeps its chain"
    );
}

/// Every placed piece exists in the merged contract with the layer the placement names, lies
/// inside the room, is held by the kit, and nothing is collidable.
#[test]
fn every_placed_piece_exists_and_lies_inside_the_room() {
    for room in ROOMS {
        let contract = load_contract(&root(), contract_path(room)).unwrap();
        let d = dressing_of(room);
        let (_, dressed) = kits(room);
        assert!(d.remove.is_empty());
        assert!(!d.place.is_empty());
        assert_eq!(
            d.parallax.len(),
            if room == RestRoom::Camp { 2 } else { 0 },
            "{room:?}"
        );
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
            // The composer's own torch pools overhang the room edge; the aftermath re-places one.
            let pool = p.piece.starts_with("torch_pool");
            assert!(
                r.x >= 0 && r.y >= 0 && (pool || r.right() <= dressed.width) && r.bottom() <= 580,
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
}

/// The bag is presenter-drawn, never placed (its five frames are a state group).
#[test]
fn the_bag_is_not_placed_and_the_camp_fire_is_animated_and_lit() {
    let d = aftermath_dressing(&root()).unwrap();
    assert!(
        d.place
            .iter()
            .all(|p| !BAG_PIECES.contains(&p.piece.as_str()))
    );
    let (_, aft) = kits(RestRoom::Aftermath);
    assert!(
        aft.pieces
            .iter()
            .all(|p| !p.piece.starts_with("aft_heavy_bag"))
    );
    let contract = load_contract(&root(), AFTERMATH_CONTRACT).unwrap();
    for id in BAG_PIECES {
        let def = contract.pieces.iter().find(|c| c.id == id).unwrap();
        assert_eq!(def.atlas, BAG_ATLAS);
        assert_eq!((def.rect[2], def.rect[3]), (40, 122));
    }
    let (_, camp) = kits(RestRoom::Camp);
    let fire = camp
        .pieces
        .iter()
        .find(|p| p.piece == "camp_fire")
        .expect("the fire is placed");
    assert_eq!((fire.x, fire.y), (758, 526));
    assert_eq!(
        fire.frames.iter().map(|f| f.1).collect::<Vec<_>>(),
        vec![110, 90, 130, 100]
    );
    let hearth = camp
        .pieces
        .iter()
        .find(|p| p.piece == "camp_hearth")
        .unwrap();
    assert!(
        camp.pieces.iter().position(|p| p.piece == "camp_fire")
            > camp.pieces.iter().position(|p| p.piece == "camp_hearth"),
        "the fire draws over the hearth ({hearth:?})"
    );
}

// ------------------------------------------------------------------ pixels

/// The opaque pixels of one atlas rect, from the committed pixel-grid source (no export
/// needed): `rows[y][x]` is true where the cell is not transparent.
fn opaque_rect(
    cache: &mut BTreeMap<String, Vec<Vec<u8>>>,
    contract: &str,
    atlas: &str,
    rect: [i32; 4],
) -> Vec<Vec<bool>> {
    if !cache.contains_key(atlas) {
        // Follow the contract chain (camp or aftermath, then its bases) for the atlas's grid.
        let mut path = contract.to_owned();
        let grid = loop {
            let v: Value =
                serde_json::from_str(&std::fs::read_to_string(root().join(&path)).unwrap())
                    .unwrap();
            if let Some(g) = v["atlases"][atlas]["grid"].as_str() {
                break g.to_owned();
            }
            path = v["extends"]
                .as_str()
                .unwrap_or_else(|| panic!("no grid for atlas {atlas}"))
                .to_owned();
        };
        let g: Value =
            serde_json::from_str(&std::fs::read_to_string(root().join(grid)).unwrap()).unwrap();
        let layer = g["layers"][0].as_str().unwrap().to_owned();
        let rows = g["frames"][0]["cels"][layer.as_str()]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_str().unwrap().bytes().collect())
            .collect();
        cache.insert(atlas.to_owned(), rows);
    }
    let rows = &cache[atlas];
    (0..rect[3])
        .map(|y| {
            (0..rect[2])
                .map(|x| rows[(rect[1] + y) as usize][(rect[0] + x) as usize] != b'.')
                .collect()
        })
        .collect()
}

/// A per-pixel coverage map of the layout, painted by the chosen tiles and pieces.
fn coverage(
    kit: &EnvKit,
    contract: &str,
    tile_layers: &[&str],
    piece_layers: &[&str],
) -> Vec<Vec<bool>> {
    let mut cache = BTreeMap::new();
    let mut covered = vec![vec![false; kit.width as usize]; 580];
    let mut paint = |x0: i32, y0: i32, opaque: Vec<Vec<bool>>| {
        for (dy, row) in opaque.iter().enumerate() {
            for (dx, &on) in row.iter().enumerate() {
                let (x, y) = (x0 + dx as i32, y0 + dy as i32);
                if on && x >= 0 && y >= 0 && (y as usize) < covered.len() && x < kit.width {
                    covered[y as usize][x as usize] = true;
                }
            }
        }
    };
    for t in kit
        .tiles
        .iter()
        .filter(|t| tile_layers.contains(&t.layer.as_str()))
    {
        let rect = [t.rect.x, t.rect.y, t.rect.w, t.rect.h];
        paint(
            t.col * kit.tile,
            t.row * kit.tile,
            opaque_rect(&mut cache, contract, &t.atlas, rect),
        );
    }
    for p in kit
        .pieces
        .iter()
        .filter(|p| piece_layers.contains(&p.layer.as_str()))
    {
        let rect = [p.rect.x, p.rect.y, p.rect.w, p.rect.h];
        paint(p.x, p.y, opaque_rect(&mut cache, contract, &p.atlas, rect));
    }
    covered
}

fn gaps(covered: &[Vec<bool>], x0: i32, x1: i32, y0: i32, y1: i32) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            if !covered[y as usize][x as usize] {
                out.push((x, y));
            }
        }
    }
    out
}

const ALL_TILES: [&str; 3] = ["backwall", "collision", "foreground"];
const ALL_PIECES: [&str; 4] = ["backwall_decal", "structure", "prop", "foreground"];

/// CAMP-A5 (gate note G7): no background (the gallery parallax) shows in the bricked bay: every
/// pixel of x 488..744 (window B2 and both adjacent pier columns), y 352..560 is covered by an
/// opaque backwall tile, collision tile or piece of the dressed aftermath. The undressed room
/// shows parallax somewhere in that span, so the span is the right one.
#[test]
fn no_background_shows_in_the_bricked_bay() {
    let (plain, dressed) = kits(RestRoom::Aftermath);
    let before = coverage(&plain, AFTERMATH_CONTRACT, &ALL_TILES, &ALL_PIECES);
    assert!(
        !gaps(&before, 488, 744, 352, 560).is_empty(),
        "the undressed bay shows the parallax"
    );
    let after = coverage(&dressed, AFTERMATH_CONTRACT, &ALL_TILES, &ALL_PIECES);
    let g = gaps(&after, 488, 744, 352, 560);
    assert!(
        g.is_empty(),
        "{} uncovered pixels, first {:?}",
        g.len(),
        &g[..g.len().min(5)]
    );
}

/// Gate note G1: the girder run meets both pier shadow columns: at y 440..449 every pixel from
/// the left pier to the right pier is covered by a girder or a pier (a structure piece), never
/// by backwall brick or the fill.
#[test]
fn the_girder_run_meets_both_piers() {
    let (_, dressed) = kits(RestRoom::Aftermath);
    let structure = coverage(&dressed, AFTERMATH_CONTRACT, &[], &["structure"]);
    let girders: Vec<&crate::env_kit::PieceDraw> = dressed
        .pieces
        .iter()
        .filter(|p| p.piece == "gym_girder")
        .collect();
    assert_eq!(
        girders.len(),
        6,
        "five visible girders and the one under the fifth"
    );
    let first = girders.iter().map(|g| g.x).min().unwrap();
    assert_eq!(first, 548);
    // The sixth girder is listed before the fifth, so the fifth draws over it.
    let order = |x: i32| girders.iter().position(|g| g.x == x).unwrap();
    assert!(order(684) < order(676));
    // The piece's own top corner at its last column is transparent (the gym room's joint too), so
    // row 440 may show brick at a joint (x = girder x + 31) but nowhere else, and rows 441..449
    // are covered from the left pier to the right pier.
    let joints: Vec<i32> = girders.iter().map(|g| g.x + 31).collect();
    let g: Vec<_> = gaps(&structure, 548, 716, 440, 450)
        .into_iter()
        .filter(|&(x, y)| !(y == 440 && joints.contains(&x)))
        .collect();
    assert!(
        g.is_empty(),
        "backwall shows between the girder run and the right pier: {:?}",
        &g[..g.len().min(5)]
    );
    // The run ends where the right pier's shadow column starts (x 716): no brick sliver.
    for (y, row) in structure.iter().enumerate().take(450).skip(441) {
        assert!(row[548], "row {y}: the left end");
        assert!(row[716], "row {y}: the right pier starts at x 716");
    }
}

/// CAMP-A6 (review F1 of the refuge, gate note G7): every placement intersecting y 561..572 is on
/// a layer whose draw order is above the collision layer, so it draws above the deck.
#[test]
fn the_floor_decals_draw_above_the_collision_layer() {
    for room in ROOMS {
        let contract = load_contract(&root(), contract_path(room)).unwrap();
        let order = |name: &str| {
            contract
                .layers
                .iter()
                .find(|l| l.name == name)
                .unwrap_or_else(|| panic!("layer {name}"))
                .draw_order
        };
        let mut n = 0;
        // The torch pool is the composer's own light decal on the back wall (re-placed after the
        // grooves); it is a wall wash, not a floor decal.
        for p in dressing_of(room)
            .place
            .iter()
            .filter(|p| !p.piece.starts_with("torch_pool"))
        {
            let def = contract.pieces.iter().find(|c| c.id == p.piece).unwrap();
            let (y0, y1) = (p.y, p.y + def.rect[3]);
            if y0 < 573 && y1 > 561 {
                assert!(
                    order(&p.layer) > order("collision"),
                    "{} on {} is under the floor deck",
                    p.piece,
                    p.layer
                );
                n += 1;
            }
        }
        assert!(
            n >= if room == RestRoom::Camp { 1 } else { 2 },
            "{room:?}: the floor chalk (and the spill and the lane chalk)"
        );
    }
}

/// The window rule (CAMP-A4): no `prop` placement's opaque pixel lies inside an opening above
/// y 520. The pieces' real pixels are used, not their rects.
#[test]
fn no_prop_pixel_stands_in_an_opening_above_y_520() {
    let camp = [(184, 335), (520, 671), (856, 1007), (1192, 1343)];
    let aft = [(216, 367), (888, 1039)];
    for (room, openings) in [(RestRoom::Camp, &camp[..]), (RestRoom::Aftermath, &aft[..])] {
        let contract = load_contract(&root(), contract_path(room)).unwrap();
        let mut cache = BTreeMap::new();
        for p in dressing_of(room).place.iter().filter(|p| p.layer == "prop") {
            let def = contract.pieces.iter().find(|c| c.id == p.piece).unwrap();
            let px = opaque_rect(&mut cache, contract_path(room), &def.atlas, def.rect);
            for (dy, row) in px.iter().enumerate() {
                for (dx, &on) in row.iter().enumerate() {
                    let (x, y) = (p.x + dx as i32, p.y + dy as i32);
                    if on && y < 520 && openings.iter().any(|&(a, b)| (a..=b).contains(&x)) {
                        // The reused lean-to and marker are the Ring's own (spec 5: R); they
                        // never stand in an opening.
                        panic!(
                            "{room:?}: {} has an opaque pixel at ({x}, {y}) in an opening",
                            p.piece
                        );
                    }
                }
            }
        }
    }
}

/// Torch clearance: no new `prop` or `structure` piece (`camp_*`, `aft_*`) has an opaque pixel
/// inside a remaining `torch_wall` rect. A wall decal (the grooves, the motifs) sits behind the
/// torch, which is a prop drawn above it.
#[test]
fn no_new_piece_covers_a_remaining_torch() {
    for room in ROOMS {
        let (_, dressed) = kits(room);
        let contract = load_contract(&root(), contract_path(room)).unwrap();
        let mut cache = BTreeMap::new();
        let torches: Vec<_> = dressed
            .pieces
            .iter()
            .filter(|p| p.piece == "torch_wall")
            .collect();
        assert!(!torches.is_empty());
        for p in dressed
            .pieces
            .iter()
            .filter(|p| p.piece.starts_with("camp_") || p.piece.starts_with("aft_"))
            .filter(|p| p.layer == "prop" || p.layer == "structure")
        {
            let px = opaque_rect(
                &mut cache,
                contract_path(room),
                &p.atlas,
                [p.rect.x, p.rect.y, p.rect.w, p.rect.h],
            );
            for (dy, row) in px.iter().enumerate() {
                for (dx, &on) in row.iter().enumerate() {
                    let (x, y) = (p.x + dx as i32, p.y + dy as i32);
                    assert!(
                        !on || !torches
                            .iter()
                            .any(|t| x >= t.x && x < t.x + 16 && y >= t.y && y < t.y + 56),
                        "{room:?}: {} covers a torch at ({x}, {y})",
                        p.piece
                    );
                }
            }
        }
        let _ = &contract;
    }
}

// ------------------------------------------------------------------ CAMP-A7: lights

/// The camp trades four torches (a flame and a pool each) for the fire's two lights; the
/// aftermath changes no light. The worst 640 px view of each stays inside the budget.
#[test]
fn the_lights_stay_inside_the_light_budget() {
    use crate::lighting::{MAX_LIGHTS, ViewRect};
    for room in ROOMS {
        let (plain, dressed) = kits(room);
        let p = plain.lighting.as_ref().expect("the plain room is lit");
        let d = dressed.lighting.as_ref().expect("the dressed room is lit");
        match room {
            RestRoom::Camp => {
                assert_eq!(
                    d.emitters.len() + 8,
                    p.emitters.len() + 2,
                    "minus 8 (four torches), plus the fire's flame and floor pool"
                );
                assert_eq!(
                    d.emitters.iter().filter(|e| e.id.contains("flame")).count(),
                    p.emitters.iter().filter(|e| e.id.contains("flame")).count() - 4 + 1,
                );
            }
            _ => assert_eq!(d.emitters.len(), p.emitters.len(), "no light change"),
        }
        let view = *dressed.views.values().next().unwrap();
        let mut worst = 0;
        for left in (0..=dressed.width - 640).step_by(8) {
            let (g, dropped) = d.gather(
                &dressed,
                ViewRect {
                    left,
                    top: view.y,
                    w: 640,
                    h: 360,
                },
                0,
            );
            assert_eq!(dropped, 0, "{room:?} left {left}");
            worst = worst.max(g.lights.len());
        }
        assert!(worst <= MAX_LIGHTS, "{room:?}: worst view {worst}");
        assert!(d.worst_case_lights <= MAX_LIGHTS, "{room:?}");
        assert!(
            d.worst_case_lights <= p.worst_case_lights.max(8),
            "{room:?}: no busier than the greybox's lights"
        );
    }
}

// ------------------------------------------------------------------ CAMP-A8: the bag

fn run(
    state: &mut BagSway,
    ticks: std::ops::Range<u64>,
    overlap: bool,
    x: i32,
) -> Vec<&'static str> {
    ticks.map(|t| bag_frame(state, t, overlap, x, 1)).collect()
}

#[test]
fn the_sway_table_has_38_ticks_and_ends_at_rest() {
    assert_eq!(SWAY_TABLE.len(), 38);
    assert_eq!(&SWAY_TABLE[..4], ["r1"; 4]);
    assert_eq!(&SWAY_TABLE[4..10], ["r2"; 6]);
    assert_eq!(&SWAY_TABLE[18..26], ["l1"; 8]);
    assert_eq!(SWAY_TABLE[37], "rest");
    assert!(
        SWAY_TABLE
            .iter()
            .all(|f| BAG_PIECES.iter().any(|p| p.ends_with(&format!("_{f}"))))
    );
}

/// CAMP-A8 1+2: the entry swing plays the table exactly once, from the first observed tick (not
/// from tick 0), then `rest` with no trigger.
#[test]
fn the_entry_swing_plays_the_table_once_then_rests() {
    let mut s = BagSway::default();
    let frames = run(&mut s, 5..80, false, 640);
    assert_eq!(
        &frames[..38],
        SWAY_TABLE.as_slice(),
        "from the first observed tick"
    );
    assert!(frames[38..].iter().all(|f| *f == "rest"));
    // From tick 0 too.
    let mut s = BagSway::default();
    let frames = run(&mut s, 0..60, false, 640);
    assert_eq!(&frames[..38], SWAY_TABLE.as_slice());
    assert!(frames[38..].iter().all(|f| *f == "rest"));
}

/// CAMP-A8 3: a rising edge restarts the table; 4: a held overlap does not retrigger.
#[test]
fn a_rising_edge_restarts_the_table_and_a_held_overlap_does_not() {
    let mut s = BagSway::default();
    run(&mut s, 0..60, false, 600);
    assert_eq!(bag_frame(&mut s, 60, false, 600, 1), "rest");
    assert_eq!(
        bag_frame(&mut s, 61, true, 600, 1),
        "r1",
        "rising edge, a push from the left"
    );
    let rest: Vec<_> = (62..100)
        .map(|t| bag_frame(&mut s, t, true, 600, 1))
        .collect();
    assert_eq!(
        &rest[..37],
        &SWAY_TABLE[1..],
        "the held overlap plays on, no retrigger"
    );
    assert!(rest[37..].iter().all(|f| *f == "rest"));
    // Release and hit again: restarts at once, even mid-swing.
    assert_eq!(bag_frame(&mut s, 100, false, 600, 1), "rest");
    assert_eq!(bag_frame(&mut s, 101, true, 600, 1), "r1");
    assert_eq!(bag_frame(&mut s, 105, true, 600, 1), "r2");
    assert_eq!(bag_frame(&mut s, 106, false, 600, 1), "r2");
    assert_eq!(
        bag_frame(&mut s, 107, true, 600, 1),
        "r1",
        "restarted mid-swing"
    );
}

/// CAMP-A8 5: a push from the right mirrors the frames; a tie follows the attacker's facing.
#[test]
fn a_left_push_mirrors_the_frames() {
    let mut s = BagSway::default();
    run(&mut s, 0..60, false, 700);
    let frames: Vec<_> = (60..98)
        .map(|t| bag_frame(&mut s, t, t == 60, 700, -1))
        .collect();
    let mirrored: Vec<_> = SWAY_TABLE
        .iter()
        .map(|f| match *f {
            "r1" => "l1",
            "r2" => "l2",
            "l1" => "r1",
            other => other,
        })
        .collect();
    assert_eq!(frames, mirrored);
    // A tie (the attacker at the bag's centre column) follows the facing.
    for (facing, first) in [(1, "r1"), (-1, "l1")] {
        let mut s = BagSway::default();
        run(&mut s, 0..60, false, 640);
        assert_eq!(bag_frame(&mut s, 60, true, 640, facing), first);
    }
}

/// CAMP-A8 6 + gate note G6.3: a backwards tick resets to `rest` with no running swing, a held
/// overlap does not fire after it, and the entry swing is never replayed.
#[test]
fn a_backwards_tick_resets_to_rest() {
    let mut s = BagSway::default();
    run(&mut s, 100..110, false, 640);
    assert_eq!(
        bag_frame(&mut s, 109, false, 640, 1),
        "r2",
        "inside the entry swing"
    );
    assert_eq!(bag_frame(&mut s, 20, false, 640, 1), "rest", "rewound");
    assert!(run(&mut s, 21..80, false, 640).iter().all(|f| *f == "rest"));
    // A held overlap across a rewind does not fire.
    let mut s = BagSway::default();
    run(&mut s, 100..160, false, 600);
    assert_eq!(bag_frame(&mut s, 50, true, 600, 1), "rest");
    assert_eq!(bag_frame(&mut s, 51, true, 600, 1), "rest");
    // ...and its release and a new edge fire normally.
    assert_eq!(bag_frame(&mut s, 52, false, 600, 1), "rest");
    assert_eq!(bag_frame(&mut s, 53, true, 600, 1), "r1");
}

/// Gate note G6.2: the same tick returns the same frame and does not re-evaluate the edge; skipped
/// ticks index the table by `tick - trigger`.
#[test]
fn the_selection_is_idempotent_per_tick_and_survives_skipped_ticks() {
    let mut s = BagSway::default();
    run(&mut s, 0..60, false, 600);
    assert_eq!(bag_frame(&mut s, 60, true, 600, 1), "r1");
    for _ in 0..3 {
        assert_eq!(bag_frame(&mut s, 60, true, 600, 1), "r1");
        assert_eq!(
            bag_frame(&mut s, 60, false, 600, 1),
            "r1",
            "no edge re-evaluation"
        );
    }
    // The overlap flag of the repeated tick did not count: the held overlap on 61 is no edge.
    assert_eq!(bag_frame(&mut s, 61, true, 600, 1), "r1");
    // Skipped ticks: from 61 to 70 the index is 70 - 60 = 10.
    assert_eq!(bag_frame(&mut s, 70, true, 600, 1), SWAY_TABLE[10]);
    assert_eq!(bag_frame(&mut s, 200, true, 600, 1), "rest");
}

/// Gate note G6.4: Carl's and Donut's overlaps are ORed; the direction uses Carl's x when his
/// overlap rose this tick, otherwise Donut's.
#[test]
fn two_attackers_or_together_and_carl_leads_the_direction() {
    let mut s = BagSway::default();
    for t in 0..60 {
        s.observe(t, None, None);
    }
    // Donut alone, from the right: a left push.
    assert_eq!(s.observe(60, None, Some((700, -1))), "l1");
    // Carl joins from the left while Donut still holds: the combined overlap does not rise, so
    // there is no retrigger and the left swing plays on.
    assert_eq!(s.observe(61, Some((600, 1)), Some((700, -1))), "l1");
    assert_eq!(s.observe(62, Some((600, 1)), Some((700, -1))), "l1");
    // Carl leads the direction on a shared rising edge (Donut to the right of the bag, Carl to
    // the left): a right push.
    let mut s = BagSway::default();
    for t in 0..60 {
        s.observe(t, None, None);
    }
    assert_eq!(
        s.observe(60, Some((600, 1)), Some((700, -1))),
        "r1",
        "Carl leads on a shared edge"
    );
    assert_eq!(
        s.observe(60, Some((600, 1)), Some((700, -1))),
        "r1",
        "idempotent"
    );
    // Donut leads when only her overlap rose.
    let mut s = BagSway::default();
    for t in 0..60 {
        s.observe(t, Some((600, 1)), None);
    }
    s.observe(60, None, None);
    assert_eq!(s.observe(61, None, Some((700, -1))), "l1");
}

#[test]
fn the_bag_rect_is_the_sidecars_hit_target_in_world_space() {
    let r = bag_rect_world(600);
    assert_eq!((r.left, r.width, r.height), (624.0, 32.0, 64.0));
    assert_eq!(r.bottom, 20.0, "layout y 516..580 in a 600 px layout");
    let hero = Rect {
        left: 600.0,
        bottom: 25.0,
        width: 30.0,
        height: 20.0,
    };
    assert!(r.overlaps(hero));
    assert!(!r.overlaps(Rect {
        left: 700.0,
        ..hero
    }));
    assert_eq!(bag_piece_index("l2"), 4);
    assert_eq!(bag_piece_index("rest"), 0);
}

/// `CAMP_LAYOUT_DIR=<dir> cargo test --lib -- --ignored dump_camp_layouts`: the composed layouts
/// of both rooms, plain and dressed, for `scripts/make_camp01_sheets.py` and the art unittest.
#[test]
#[ignore = "evidence helper: writes the composed layouts for the Python measurements"]
fn dump_camp_layouts() {
    let Ok(dir) = std::env::var("CAMP_LAYOUT_DIR") else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    for (room, id) in [
        (RestRoom::Camp, CAMP_ROOM),
        (RestRoom::Aftermath, AFTERMATH_ROOM),
    ] {
        let run = f01_rest::launch_with(&root(), room, false).unwrap();
        for dressed in [false, true] {
            let contract = load_contract(&root(), contract_path(room)).unwrap();
            let d = dressing_of(room);
            let mut out = None;
            crate::f01_boiler::compose_kit_hooked(
                &root(),
                &run.loaded.room,
                &[],
                Some(contract),
                |layout| {
                    if dressed {
                        assert_eq!(dress_with(layout, &d, contract_path(room)), 0);
                    }
                    layout["kit_contract"] = json!(contract_path(room));
                    out = Some(layout.clone());
                    Ok(())
                },
            )
            .unwrap();
            let name = format!(
                "{id}.{}.layout.json",
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

/// The `punch` evidence pilot (aftermath only) stands Carl at the bag and lands hits: his active
/// melee hit box overlaps the bag's rect with a rising edge at least three times in 300 ticks, and
/// the presenter selection then leaves `rest` after the entry swing. The bag stays inert.
#[test]
fn the_punch_pilot_hits_the_bag_and_the_presenter_swings() {
    use crate::combat::{Attacker, Phase, hit_box};
    use crate::prototype::HERO_HALF_HEIGHT;
    use crate::rest_fixtures::{Pilot, pilot};
    assert!(Pilot::Punch.fits(RestRoom::Aftermath) && !Pilot::Punch.fits(RestRoom::Camp));
    assert_eq!(Pilot::parse("punch"), Ok(Pilot::Punch));
    let (carl, donut) = Loadout::Story.flags();
    let run = f01_rest::launch(&root(), RestRoom::Aftermath).unwrap();
    let height = run.kit.height;
    let mut sim = RestSim::from_run(
        run,
        360,
        carl,
        donut,
        Party::Solo { uncontrolled: BOT },
        None,
    )
    .unwrap();
    let bag = bag_rect_world(height);
    let mut sway = BagSway::default();
    let (mut edges, mut prev, mut swung_after_entry) = (0, false, false);
    while sim.tick() < 300 {
        let input = pilot(Pilot::Punch, &sim.combat, &sim.scene, 1100.0);
        sim.step([input, crate::input::RawInput::NONE]);
        let hero = &sim.scene.world.hero;
        let attacker = Attacker {
            x: hero.x,
            y: hero.y,
            half_height: HERO_HALF_HEIGHT,
            facing: hero.facing,
            player: 0,
        };
        let hit = hero
            .melee
            .current()
            .filter(|m| m.phase == Phase::Active)
            .and_then(|m| hit_box(&m.spec.shape, &attacker))
            .filter(|r| r.overlaps(bag));
        edges += usize::from(hit.is_some() && !prev);
        prev = hit.is_some();
        let frame = sway.observe(
            sim.scene.world.tick,
            hit.map(|_| (hero.x.round() as i32, i32::from(hero.facing))),
            None,
        );
        if sim.tick() > 60 && frame != "rest" {
            swung_after_entry = true;
        }
    }
    assert!(edges >= 3, "{edges} rising edges");
    assert!(swung_after_entry, "the presenter swings on a hit");
}
