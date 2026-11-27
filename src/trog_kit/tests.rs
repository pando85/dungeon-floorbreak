use super::*;
use crate::env_kit::IRect;
use crate::f01_trog;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

const ROOM: TrogRoom = TrogRoom::Run;

fn dressing() -> Dressing {
    run_dressing(&root()).unwrap()
}

/// The room's kit (with its lane solids) without and with the dressing.
fn kits() -> (EnvKit, EnvKit) {
    kits_of(ROOM)
}

fn kits_of(room: TrogRoom) -> (EnvKit, EnvKit) {
    let loaded = f01_trog::launch_with(&root(), room, false).unwrap().loaded;
    (
        compose_room_kit(&root(), &loaded, room, false).unwrap(),
        compose_room_kit(&root(), &loaded, room, true).unwrap(),
    )
}

// ------------------------------------------------------------------ TROG-ART-A1: presentation only

/// TROG-ART-A1: the dressing changes no collider (the ceiling slab and the statics included),
/// spawn, exit, view or walking line, no blink barrier and no collision cell, and adds no
/// collision tile.
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
    assert_eq!(dressed.actor_z, plain.actor_z);
    assert_eq!(dressed.deck_top_px, plain.deck_top_px);
    let collision = |kit: &EnvKit| {
        kit.tiles
            .iter()
            .filter(|t| t.layer == "collision")
            .map(|t| (t.col, t.row, t.rect))
            .collect::<Vec<_>>()
    };
    assert_eq!(collision(&dressed), collision(&plain));
    assert!(
        dressed.pieces.len() != plain.pieces.len(),
        "the dressing did happen"
    );
    // The slab and the two sidecar statics are in both (the shared lane_solids path).
    let slab = |kit: &EnvKit| {
        kit.solid_rects
            .iter()
            .filter(|r| r.w == kit.width && r.h == 16)
            .count()
    };
    assert_eq!((slab(&plain), slab(&dressed)), (1, 1));
    for (x, y) in [(352, 548), (1840, 548)] {
        assert!(
            dressed
                .solid_rects
                .iter()
                .any(|r| (r.x, r.y, r.w, r.h) == (x, y, 64, 32)),
            "the static at ({x}, {y})"
        );
    }
}

/// The launched rooms, dressed and plain: the same solids, spawn, exit, views, walking line and
/// checkpoint columns, the same room and sidecar hashes and the same statics.
#[test]
fn the_launched_rooms_are_the_same_rooms_with_and_without_the_dressing() {
    let plain = f01_trog::launch_with(&root(), ROOM, false).unwrap();
    let dressed = f01_trog::launch(&root(), ROOM).unwrap();
    assert_eq!(dressed.kit.solid_rects, plain.kit.solid_rects);
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
        format!("{:?}", plain.loaded.encounter.statics)
    );
    assert_ne!(
        format!("{:?}", dressed.kit.pieces),
        format!("{:?}", plain.kit.pieces),
        "the room's own default is the dressed one"
    );
}

/// The dressing does not change the state hash: the room's smoke script (walk right under the
/// troglodytes) hashes the same on every tick with the plain kit and the dressed kit.
#[test]
fn the_dressing_does_not_change_the_state_hash() {
    let hashes = |dressed: bool| -> Vec<u64> {
        let run = f01_trog::launch_with(&root(), ROOM, dressed).unwrap();
        let mut scene =
            crate::corridor::SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
        let mut combat = f01_trog::combat_of(&run, &mut scene).unwrap();
        (1..=500u64)
            .map(|tick| {
                combat.step(
                    &mut scene,
                    run.fixture.input(tick.min(f01_trog::SMOKE_TICKS)),
                );
                combat.state_hash(&scene)
            })
            .collect()
    };
    let plain = hashes(false);
    assert_eq!(plain.len(), 500);
    assert_eq!(hashes(true), plain);
}

// ------------------------------------------------------------------ the dressing block

/// TROG-ART-A3: every `remove_at` entry matches (`dress_with` reports zero unmatched) and drops
/// exactly one composer piece; an unmatched entry is an error, never skipped.
#[test]
fn every_remove_at_matches_exactly_one_piece() {
    let d = dressing();
    assert_eq!(d.remove_at.len(), 6);
    let (plain, dressed) = kits();
    for r in &d.remove_at {
        let count = |kit: &EnvKit| {
            kit.pieces
                .iter()
                .filter(|p| p.layer == r.layer && p.piece == r.piece && p.x == r.x && p.y == r.y)
                .count()
        };
        assert_eq!(count(&plain), 1, "{r:?} in the plain kit");
        assert_eq!(count(&dressed), 0, "{r:?} dropped");
    }
    let loaded = f01_trog::launch_with(&root(), ROOM, false).unwrap().loaded;
    let contract = load_contract(&root(), TROG_CONTRACT).unwrap();
    let mut missed = usize::MAX;
    crate::f01_boiler::compose_kit_hooked(&root(), &loaded.room, &[], Some(contract), |l| {
        missed = dress_with(l, &d, TROG_CONTRACT);
        Ok(())
    })
    .unwrap();
    assert_eq!(missed, 0);
    let mut bad = dressing();
    bad.remove_at[0].x += 1;
    let contract = load_contract(&root(), TROG_CONTRACT).unwrap();
    let err =
        crate::f01_boiler::compose_kit_hooked(&root(), &loaded.room, &[], Some(contract), |l| {
            match dress_with(l, &bad, TROG_CONTRACT) {
                0 => Ok(()),
                n => Err(format!("{n} unmatched")),
            }
        })
        .unwrap_err();
    assert!(err.contains("unmatched"), "{err}");
    // The doubled torch at x 2240 loses its upper flame and pool; the lower torch and the other
    // torches stay; the two bricked bays lose their chains, the open windows keep theirs.
    let torches = |x: i32| {
        dressed
            .pieces
            .iter()
            .filter(|p| p.piece == "torch_wall" && p.x == x)
            .map(|p| p.y)
            .collect::<Vec<_>>()
    };
    assert_eq!(torches(2240), vec![448], "one torch stays at 2240");
    for x in [208, 1216] {
        assert!(
            !dressed
                .pieces
                .iter()
                .any(|p| p.piece == "arch_threshold" && (p.x, p.y) == (x, 544)),
            "the bricked bay's blue sill at {x} is gone (AD20)"
        );
    }
    for x in [224, 1232] {
        assert!(
            !dressed
                .pieces
                .iter()
                .any(|p| p.piece == "arch_chain" && (p.x, p.y) == (x, 384)),
            "the bricked bay's catenary at {x} is gone"
        );
    }
    for x in [560, 896, 1568, 2064] {
        assert!(
            dressed
                .pieces
                .iter()
                .any(|p| p.piece == "arch_chain" && (p.x, p.y) == (x, 384)),
            "the open window at {x} keeps its chain"
        );
    }
}

/// Every placed piece exists in the merged contract with the layer the placement names, lies
/// inside the room, is held by the kit, and nothing is collidable.
#[test]
fn every_placed_piece_exists_and_lies_inside_the_room() {
    let contract = load_contract(&root(), TROG_CONTRACT).unwrap();
    let d = dressing();
    let (_, dressed) = kits();
    assert!(d.remove.is_empty() && d.parallax.is_empty());
    assert_eq!(d.backwall_fill.len(), 2);
    assert_eq!(d.place.len(), 15);
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

/// Spec section 9.4: the step faces cover exactly the sidecar statics' rects, edge to edge (no
/// sliver, no overhang).
#[test]
fn the_step_faces_cover_the_statics_exactly() {
    let contract = load_contract(&root(), TROG_CONTRACT).unwrap();
    let loaded = f01_trog::launch_with(&root(), ROOM, false).unwrap().loaded;
    let d = dressing();
    let statics: Vec<_> = loaded
        .encounter
        .statics
        .iter()
        .filter(|s| s.kind == crate::encounter::StaticKind::Solid)
        .collect();
    assert_eq!(statics.len(), 2);
    for st in statics {
        let mut spans: Vec<(i32, i32, i32)> = d
            .place
            .iter()
            .filter(|p| p.piece.starts_with("trog_step_"))
            .filter_map(|p| {
                let def = contract.pieces.iter().find(|c| c.id == p.piece)?;
                let (x1, y1) = (p.x + def.rect[2], p.y + def.rect[3]);
                (p.x >= st.rect.x && x1 <= st.rect.x + st.rect.w && p.y == st.rect.y)
                    .then_some((p.x, x1, y1))
            })
            .collect();
        spans.sort_unstable();
        let mut x = st.rect.x;
        for (x0, x1, y1) in spans {
            assert_eq!(x0, x, "{}: a gap or an overlap at {x}", st.id);
            assert_eq!(y1, 580, "{}: the face ends on the walking line", st.id);
            x = x1;
        }
        assert_eq!(x, st.rect.x + st.rect.w, "{}", st.id);
    }
}

// ------------------------------------------------------------------ pixels

/// The opaque pixels of one atlas rect, from the committed pixel-grid source (no export
/// needed): `rows[y][x]` is true where the cell is not transparent.
fn opaque_rect(
    cache: &mut BTreeMap<String, Vec<Vec<u8>>>,
    atlas: &str,
    rect: [i32; 4],
) -> Vec<Vec<bool>> {
    if !cache.contains_key(atlas) {
        // Follow the contract chain (trog, gym, ... ) for the atlas's grid.
        let mut path = TROG_CONTRACT.to_owned();
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
fn coverage(kit: &EnvKit, tile_layers: &[&str], piece_layers: &[&str]) -> Vec<Vec<bool>> {
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
            opaque_rect(&mut cache, &t.atlas, rect),
        );
    }
    for p in kit
        .pieces
        .iter()
        .filter(|p| piece_layers.contains(&p.layer.as_str()))
    {
        let rect = [p.rect.x, p.rect.y, p.rect.w, p.rect.h];
        paint(p.x, p.y, opaque_rect(&mut cache, &p.atlas, rect));
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
/// Bays 1 and 4: pier_l .. pier_r + 15 (piers at 144, 480 and 1152, 1488).
const BRICKED_BAYS: [(i32, i32); 2] = [(144, 495), (1152, 1503)];

/// TROG-ART-A5 (gate note G5): the span of the **undressed** bays that shows background (the
/// parallax or a null pixel, both pier inner columns included, y 352..559) lies inside the
/// `backwall_fill` rect, and the dressed layout shows no background in the whole fill.
#[test]
fn no_background_shows_in_the_bricked_bays() {
    let (plain, dressed) = kits();
    let before = coverage(&plain, &ALL_TILES, &ALL_PIECES);
    let after = coverage(&dressed, &ALL_TILES, &ALL_PIECES);
    let d = dressing();
    assert_eq!(d.backwall_fill.len(), BRICKED_BAYS.len());
    for (&(b0, b1), fill) in BRICKED_BAYS.iter().zip(&d.backwall_fill) {
        let g = gaps(&before, b0, b1 + 1, 352, 560);
        assert!(
            !g.is_empty(),
            "the undressed bay {b0}..{b1} shows background"
        );
        let (lo, hi) = (
            g.iter().map(|p| p.0).min().unwrap(),
            g.iter().map(|p| p.0).max().unwrap(),
        );
        let (ymin, ymax) = (
            g.iter().map(|p| p.1).min().unwrap(),
            g.iter().map(|p| p.1).max().unwrap(),
        );
        assert!(
            lo >= fill.x && hi < fill.x + fill.w && ymin >= fill.y && ymax < fill.y + fill.h,
            "bay {b0}..{b1}: the undressed background spans x {lo}..{hi} y {ymin}..{ymax}, \
             outside the fill {fill:?}"
        );
        let g = gaps(&after, b0, b1 + 1, 352, 560);
        assert!(
            g.is_empty(),
            "bay {b0}..{b1}: {} uncovered pixels, first {:?}",
            g.len(),
            &g[..g.len().min(5)]
        );
    }
}

/// TROG-ART-A6 (gate note G5): every dressing placement intersecting y 561..579 is on a layer
/// whose draw order is above the collision layer, so it draws above the deck.
#[test]
fn the_floor_decals_draw_above_the_collision_layer() {
    let contract = load_contract(&root(), TROG_CONTRACT).unwrap();
    let order = |name: &str| {
        contract
            .layers
            .iter()
            .find(|l| l.name == name)
            .unwrap_or_else(|| panic!("layer {name}"))
            .draw_order
    };
    let mut n = 0;
    for p in &dressing().place {
        let def = contract.pieces.iter().find(|c| c.id == p.piece).unwrap();
        if p.y < 580 && p.y + def.rect[3] > 561 {
            assert!(
                order(&p.layer) > order("collision"),
                "{} on {} is under the floor deck",
                p.piece,
                p.layer
            );
            n += 1;
        }
    }
    assert_eq!(n, 11, "ten step pieces and the crater");
}

/// TROG-ART-A4 (the window rule): no `prop` placement's opaque pixel lies inside an opening that
/// is not bricked up, above y 520. The pieces' real pixels are used, not their rects.
#[test]
fn no_prop_pixel_stands_in_an_open_window_above_y_520() {
    // Openings (pier_l + 40 .. pier_l + 191); bays 1 and 4 (184..335, 1192..1343) are bricked.
    let open = [(520, 671), (856, 1007), (1528, 1679), (2024, 2175)];
    let contract = load_contract(&root(), TROG_CONTRACT).unwrap();
    let mut cache = BTreeMap::new();
    let mut n = 0;
    for p in dressing().place.iter().filter(|p| p.layer == "prop") {
        let def = contract.pieces.iter().find(|c| c.id == p.piece).unwrap();
        let px = opaque_rect(&mut cache, &def.atlas, def.rect);
        for (dy, row) in px.iter().enumerate() {
            for (dx, &on) in row.iter().enumerate() {
                let (x, y) = (p.x + dx as i32, p.y + dy as i32);
                assert!(
                    !(on && y < 520 && open.iter().any(|&(a, b)| (a..=b).contains(&x))),
                    "{} has an opaque pixel at ({x}, {y}) in an open window",
                    p.piece
                );
            }
        }
        n += 1;
    }
    assert_eq!(n, 1, "the bell is the run's one placed prop");
}

/// Torch clearance: no new `prop` or `structure` piece (`trog_*`) has an opaque pixel inside a
/// remaining `torch_wall` rect. A wall decal sits behind the torch, a prop above it.
#[test]
fn no_new_piece_covers_a_remaining_torch() {
    let (_, dressed) = kits();
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
        .filter(|p| p.piece.starts_with("trog_"))
        .filter(|p| p.layer == "prop" || p.layer == "structure")
    {
        let px = opaque_rect(
            &mut cache,
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
                    "{} covers a torch at ({x}, {y})",
                    p.piece
                );
            }
        }
    }
}

// ------------------------------------------------------------------ TROG-ART-A7: lights

/// The run trades the upper torch at x 2240 (a flame and a pool) for nothing: minus two lights.
/// The worst 640 px view stays inside the budget.
#[test]
fn the_lights_stay_inside_the_light_budget() {
    use crate::lighting::{MAX_LIGHTS, ViewRect};
    let (plain, dressed) = kits();
    let p = plain.lighting.as_ref().expect("the plain room is lit");
    let d = dressed.lighting.as_ref().expect("the dressed room is lit");
    assert_eq!(
        d.emitters.len() + 2,
        p.emitters.len(),
        "minus the flame and the pool"
    );
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
        assert_eq!(dropped, 0, "left {left}");
        worst = worst.max(g.lights.len());
    }
    assert!(worst <= MAX_LIGHTS, "worst view {worst}");
    assert!(d.worst_case_lights <= MAX_LIGHTS);
    assert!(
        d.worst_case_lights <= p.worst_case_lights.max(8),
        "no busier than the greybox's lights"
    );
}

// ------------------------------------------------------------------ stage 2: `trog_strength`

const STRENGTH: TrogRoom = TrogRoom::Strength;
const STRENGTH_ID: &str = "trog_strength";

fn strength_dressing() -> Dressing {
    dressing_of(&root(), STRENGTH)
        .unwrap()
        .expect("trog_strength has a block")
}

/// TROG-ART-A1 for the strength room: the dressing changes no collider (the slab and the shelf and
/// bench statics included), spawn, exit, view, walking line, blink barrier or collision cell, and the
/// launched rooms, their hashes and statics are the same.
#[test]
fn strength_the_dressing_changes_no_collider_spawn_exit_view_or_walk_line() {
    let (plain, dressed) = kits_of(STRENGTH);
    assert_eq!(dressed.solid_rects, plain.solid_rects);
    assert_eq!(dressed.spawn, plain.spawn);
    assert_eq!(dressed.exit, plain.exit);
    assert_eq!(dressed.views, plain.views);
    assert_eq!(dressed.base_walk_line_y, plain.base_walk_line_y);
    assert_eq!(dressed.blink_barriers, plain.blink_barriers);
    assert_eq!((dressed.width, dressed.height), (plain.width, plain.height));
    assert_eq!(dressed.collidable_cells, plain.collidable_cells);
    assert_eq!(dressed.actor_z, plain.actor_z);
    assert_eq!(dressed.deck_top_px, plain.deck_top_px);
    assert!(
        dressed.pieces.len() != plain.pieces.len(),
        "the dressing did happen"
    );
    for (x, y, w, h) in [(288, 532, 64, 48), (640, 500, 32, 80)] {
        assert!(
            dressed
                .solid_rects
                .iter()
                .any(|r| (r.x, r.y, r.w, r.h) == (x, y, w, h)),
            "the static at ({x}, {y})"
        );
    }
    let a = f01_trog::launch_with(&root(), STRENGTH, false).unwrap();
    let b = f01_trog::launch(&root(), STRENGTH).unwrap();
    assert_eq!(b.rules.checkpoints, a.rules.checkpoints);
    assert_eq!(
        (b.loaded.room_hash, b.loaded.sidecar_hash),
        (a.loaded.room_hash, a.loaded.sidecar_hash)
    );
    assert_eq!(
        format!("{:?}", b.loaded.encounter.statics),
        format!("{:?}", a.loaded.encounter.statics)
    );
    assert_ne!(format!("{:?}", b.kit.pieces), format!("{:?}", a.kit.pieces));
    assert!(is_dressed(&root(), STRENGTH));
}

/// The state hash is equal on every tick of the room's smoke script, dressed against plain.
#[test]
fn strength_the_dressing_does_not_change_the_state_hash() {
    let hashes = |dressed: bool| -> Vec<u64> {
        let run = f01_trog::launch_with(&root(), STRENGTH, dressed).unwrap();
        let mut scene =
            crate::corridor::SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
        let mut combat = f01_trog::combat_of(&run, &mut scene).unwrap();
        (1..=500u64)
            .map(|tick| {
                combat.step(
                    &mut scene,
                    run.fixture.input(tick.min(f01_trog::SMOKE_TICKS)),
                );
                combat.state_hash(&scene)
            })
            .collect()
    };
    let plain = hashes(false);
    assert_eq!(plain.len(), 500);
    assert_eq!(hashes(true), plain);
}

/// TROG-ART-A3 for the strength room: eight `remove_at`, each dropping exactly one composer piece
/// (bay 3's chain and blue sill, the two end torches with their pools, the torch above the bench with
/// its pool, gate note G1); `dress_with` reports zero unmatched.
#[test]
fn strength_every_remove_at_matches_exactly_one_piece() {
    let d = strength_dressing();
    assert_eq!(d.remove_at.len(), 8);
    let (plain, dressed) = kits_of(STRENGTH);
    for r in &d.remove_at {
        let count = |kit: &EnvKit| {
            kit.pieces
                .iter()
                .filter(|p| p.layer == r.layer && p.piece == r.piece && p.x == r.x && p.y == r.y)
                .count()
        };
        assert_eq!(count(&plain), 1, "{r:?} in the plain kit");
        assert_eq!(count(&dressed), 0, "{r:?} dropped");
    }
    let loaded = f01_trog::launch_with(&root(), STRENGTH, false)
        .unwrap()
        .loaded;
    let contract = load_contract(&root(), TROG_CONTRACT).unwrap();
    let mut missed = usize::MAX;
    crate::f01_boiler::compose_kit_hooked(&root(), &loaded.room, &[], Some(contract), |l| {
        missed = dress_with(l, &d, TROG_CONTRACT);
        Ok(())
    })
    .unwrap();
    assert_eq!(missed, 0);
    // The open bays keep their sills and chains; no blue sill stays in the bricked bay (AD-S3).
    for x in [80, 416] {
        assert!(
            dressed
                .pieces
                .iter()
                .any(|p| p.piece == "arch_threshold" && (p.x, p.y) == (x, 544))
        );
        assert!(
            dressed
                .pieces
                .iter()
                .any(|p| p.piece == "arch_chain" && (p.x, p.y) == (x + 16, 384))
        );
    }
    assert!(
        !dressed
            .pieces
            .iter()
            .any(|p| p.piece == "arch_threshold" && (p.x, p.y) == (752, 544))
    );
    // G1: no remaining torch flame column stands over the bench.
    for t in dressed.pieces.iter().filter(|p| p.piece == "torch_wall") {
        assert!(
            t.x + 16 <= 640 || t.x >= 672,
            "a torch at ({}, {}) stands over the bench",
            t.x,
            t.y
        );
    }
}

/// Every placed piece exists in the merged contract with the layer the placement names, lies inside
/// the room and is held by the kit.
#[test]
fn strength_every_placed_piece_exists_and_lies_inside_the_room() {
    let contract = load_contract(&root(), TROG_CONTRACT).unwrap();
    let d = strength_dressing();
    let (_, dressed) = kits_of(STRENGTH);
    assert!(d.remove.is_empty() && d.parallax.is_empty());
    assert_eq!(d.backwall_fill.len(), 1);
    assert_eq!(d.place.len(), 20);
    for p in &d.place {
        let def = contract
            .pieces
            .iter()
            .find(|c| c.id == p.piece)
            .unwrap_or_else(|| panic!("unknown piece {}", p.piece));
        assert_eq!(def.layer, p.layer, "{}", p.piece);
        assert!(
            p.x >= 0
                && p.y >= 0
                && p.x + def.rect[2] <= dressed.width
                && p.y + def.rect[3] <= dressed.height,
            "{} is outside the room",
            p.piece
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

/// The shelf faces cover the shelf static edge to edge, and the placed bench key's drawn bounds
/// equal the sidecar's bench static exactly (TROG-AD46).
#[test]
fn strength_the_shelf_and_the_bench_are_drawn_on_their_statics() {
    let contract = load_contract(&root(), TROG_CONTRACT).unwrap();
    let loaded = f01_trog::launch_with(&root(), STRENGTH, false)
        .unwrap()
        .loaded;
    let d = strength_dressing();
    let rect_of = |id: &str| {
        let s = loaded
            .encounter
            .statics
            .iter()
            .find(|s| s.id == id)
            .unwrap();
        (s.rect.x, s.rect.y, s.rect.w, s.rect.h)
    };
    let shelf = rect_of("shelf");
    let mut spans: Vec<(i32, i32, i32)> = d
        .place
        .iter()
        .filter(|p| p.piece.starts_with("trog_shelf48_"))
        .map(|p| {
            let def = contract.pieces.iter().find(|c| c.id == p.piece).unwrap();
            assert_eq!(p.y, shelf.1, "{}", p.piece);
            assert_eq!(
                p.y + def.rect[3],
                580,
                "{}: the face ends on the walking line",
                p.piece
            );
            (p.x, p.x + def.rect[2], p.y)
        })
        .collect();
    spans.sort_unstable();
    let mut x = shelf.0;
    for (x0, x1, _) in spans {
        assert_eq!(x0, x, "a gap or an overlap at {x}");
        x = x1;
    }
    assert_eq!(x, shelf.0 + shelf.2);
    // The bench: one placed `trog_bench_intact`, its opaque bounding box is the static's rect.
    let bench = rect_of("bench");
    let placed: Vec<_> = d
        .place
        .iter()
        .filter(|p| p.piece.starts_with("trog_bench"))
        .collect();
    assert_eq!(placed.len(), 1, "one static bench key, no partition frames");
    assert_eq!(placed[0].piece, "trog_bench_intact");
    let def = contract
        .pieces
        .iter()
        .find(|c| c.id == "trog_bench_intact")
        .unwrap();
    let mut cache = BTreeMap::new();
    let px = opaque_rect(&mut cache, &def.atlas, def.rect);
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for (dy, row) in px.iter().enumerate() {
        for (dx, &on) in row.iter().enumerate() {
            if on {
                x0 = x0.min(placed[0].x + dx as i32);
                x1 = x1.max(placed[0].x + dx as i32);
                y0 = y0.min(placed[0].y + dy as i32);
                y1 = y1.max(placed[0].y + dy as i32);
            }
        }
    }
    assert_eq!(
        (x0, y0, x1 - x0 + 1, y1 - y0 + 1),
        bench,
        "the drawn bounds are the bench static"
    );
    // The weakened, broken and debris keys are reserved (TROG-Q14), never placed or packed.
    let text = std::fs::read_to_string(root().join(TROG_CONTRACT)).unwrap();
    for reserved in [
        "trog_bench_weakened",
        "trog_bench_broken",
        "trog_bench_debris",
        "state_groups",
    ] {
        assert!(
            !text.contains(reserved),
            "{reserved} is not in the runtime contract"
        );
    }
}

/// TROG-ART-A5 for bay 3: the undressed bay's background span lies inside the `backwall_fill`
/// rect, and the dressed layout shows no background anywhere in the fill.
#[test]
fn strength_no_background_shows_in_the_bricked_bay() {
    let (plain, dressed) = kits_of(STRENGTH);
    let before = coverage(&plain, &ALL_TILES, &ALL_PIECES);
    let after = coverage(&dressed, &ALL_TILES, &ALL_PIECES);
    let fill = &strength_dressing().backwall_fill[0];
    let g = gaps(&before, 688, 1000, 352, 560);
    assert!(!g.is_empty(), "the undressed bay shows background");
    let (lo, hi) = (
        g.iter().map(|p| p.0).min().unwrap(),
        g.iter().map(|p| p.0).max().unwrap(),
    );
    let (ymin, ymax) = (
        g.iter().map(|p| p.1).min().unwrap(),
        g.iter().map(|p| p.1).max().unwrap(),
    );
    assert!(
        lo >= fill.x && hi < fill.x + fill.w && ymin >= fill.y && ymax < fill.y + fill.h,
        "the undressed background spans x {lo}..{hi} y {ymin}..{ymax}, outside the fill {fill:?}"
    );
    let g = gaps(&after, fill.x, fill.x + fill.w, 352, 560);
    assert!(
        g.is_empty(),
        "{} uncovered pixels, first {:?}",
        g.len(),
        &g[..g.len().min(5)]
    );
}

/// TROG-ART-A6 for the strength room: every placement touching y 561..579 is on a layer above the
/// collision layer (shelf face, mats, threshold and the bench).
#[test]
fn strength_the_floor_pieces_draw_above_the_collision_layer() {
    let contract = load_contract(&root(), TROG_CONTRACT).unwrap();
    let order = |name: &str| {
        contract
            .layers
            .iter()
            .find(|l| l.name == name)
            .unwrap()
            .draw_order
    };
    let mut n = 0;
    for p in &strength_dressing().place {
        let def = contract.pieces.iter().find(|c| c.id == p.piece).unwrap();
        if p.y < 580 && p.y + def.rect[3] > 561 {
            assert!(
                order(&p.layer) > order("collision"),
                "{} is under the floor deck",
                p.piece
            );
            n += 1;
        }
    }
    // the five shelf pieces, the three mats, the threshold and the bench (the rack and the plate tree end at y 559)
    assert_eq!(n, 5 + 3 + 1 + 1);
}

/// TROG-ART-A4 for the strength room: no `prop` pixel in an open window (56..207, 392..543) above
/// y 520, and no new `trog_*` prop or structure pixel inside a remaining torch rect.
#[test]
fn strength_no_prop_in_an_open_window_and_no_covered_torch() {
    let open = [(56, 207), (392, 543)];
    let contract = load_contract(&root(), TROG_CONTRACT).unwrap();
    let mut cache = BTreeMap::new();
    for p in strength_dressing()
        .place
        .iter()
        .filter(|p| p.layer == "prop")
    {
        let def = contract.pieces.iter().find(|c| c.id == p.piece).unwrap();
        let px = opaque_rect(&mut cache, &def.atlas, def.rect);
        for (dy, row) in px.iter().enumerate() {
            for (dx, &on) in row.iter().enumerate() {
                let (x, y) = (p.x + dx as i32, p.y + dy as i32);
                assert!(
                    !(on && y < 520 && open.iter().any(|&(a, b)| (a..=b).contains(&x))),
                    "{} has an opaque pixel at ({x}, {y}) in an open window",
                    p.piece
                );
            }
        }
    }
    let (_, dressed) = kits_of(STRENGTH);
    let torches: Vec<_> = dressed
        .pieces
        .iter()
        .filter(|p| p.piece == "torch_wall")
        .collect();
    assert!(!torches.is_empty());
    for p in dressed
        .pieces
        .iter()
        .filter(|p| p.piece.starts_with("trog_") || p.piece.starts_with("gym_"))
        .filter(|p| p.layer == "prop" || p.layer == "structure")
    {
        let px = opaque_rect(
            &mut cache,
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
                    "{} covers a torch at ({x}, {y})",
                    p.piece
                );
            }
        }
    }
}

/// TROG-ART-A7 for the strength room: minus three torches (a flame and a pool each), plus the cold
/// tube's two lights; the worst 640 px view stays within `MAX_LIGHTS`.
#[test]
fn strength_the_lights_stay_inside_the_light_budget() {
    use crate::lighting::{MAX_LIGHTS, ViewRect};
    let (plain, dressed) = kits_of(STRENGTH);
    let p = plain.lighting.as_ref().expect("the plain room is lit");
    let d = dressed.lighting.as_ref().expect("the dressed room is lit");
    let tubes = d.emitters.iter().filter(|e| e.id.contains("tube")).count();
    assert_eq!(tubes, 2, "the cold tube emits two lights");
    assert_eq!(
        d.emitters.len() + 6,
        p.emitters.len() + tubes,
        "minus 6 (three torches), plus the tube"
    );
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
        assert_eq!(dropped, 0, "left {left}");
        worst = worst.max(g.lights.len());
    }
    assert!(worst <= MAX_LIGHTS, "worst view {worst}");
    assert!(d.worst_case_lights <= MAX_LIGHTS);
}

/// The composed room id of the strength block is the room's id.
#[test]
fn strength_block_is_keyed_by_the_room_id() {
    assert_eq!(STRENGTH.id(), STRENGTH_ID);
}

// ------------------------------------------------------------------ evidence helper

/// `TROG_LAYOUT_DIR=<dir> cargo test --lib -- --ignored dump_trog_layouts`: the composed layouts
/// of each troglodyte room, plain and (where the contract has a block) dressed, for
/// `scripts/make_trogarea01_sheets.py` and the art unittest.
#[test]
#[ignore = "evidence helper: writes the composed layouts for the Python measurements"]
fn dump_trog_layouts() {
    let Ok(dir) = std::env::var("TROG_LAYOUT_DIR") else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    for room in TrogRoom::ALL {
        let loaded = f01_trog::launch_with(&root(), room, false).unwrap().loaded;
        let block = dressing_of(&root(), room).unwrap();
        for dressed in [false, true] {
            if dressed && block.is_none() {
                continue;
            }
            let contract = load_contract(&root(), TROG_CONTRACT).unwrap();
            let mut out = None;
            crate::f01_boiler::compose_kit_hooked(
                &root(),
                &loaded.room,
                &[],
                Some(contract),
                |layout| {
                    if let (true, Some(d)) = (dressed, &block) {
                        assert_eq!(dress_with(layout, d, TROG_CONTRACT), 0);
                    }
                    layout["kit_contract"] = json!(TROG_CONTRACT);
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

/// TROG-ART-AD19: a room whose contract has no `room_dressing` block launches plain (stage 2 rooms
/// until their block lands); a malformed block is still an error and `trog_run` still dresses.
#[test]
fn a_room_without_a_dressing_block_composes_plain() {
    let contract: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root().join(TROG_CONTRACT)).unwrap())
            .unwrap();
    assert!(
        dressing_from(&contract, "trog_synthetic")
            .unwrap()
            .is_none()
    );
    assert!(dressing_from(&contract, RUN_ROOM).unwrap().is_some());
    let mut null_block = contract.clone();
    null_block["room_dressing"]["trog_synthetic"] = serde_json::Value::Null;
    assert!(
        dressing_from(&null_block, "trog_synthetic")
            .unwrap()
            .is_none()
    );
    let mut bad = contract.clone();
    bad["room_dressing"]["trog_synthetic"] = json!({"place": 3});
    assert!(dressing_from(&bad, "trog_synthetic").is_err());

    let loaded = f01_trog::launch_with(&root(), ROOM, false).unwrap().loaded;
    let plain = compose_by_id(&root(), &loaded, "trog_synthetic", true).unwrap();
    let plain_ref = compose_by_id(&root(), &loaded, RUN_ROOM, false).unwrap();
    assert_eq!(
        format!("{:?}", plain.pieces),
        format!("{:?}", plain_ref.pieces)
    );
    let dressed = compose_by_id(&root(), &loaded, RUN_ROOM, true).unwrap();
    assert_ne!(
        format!("{:?}", dressed.pieces),
        format!("{:?}", plain.pieces)
    );
    assert!(is_dressed(&root(), ROOM));
}
