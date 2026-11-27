use super::*;
use crate::env_kit::IRect;
use crate::f01_act1;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn id_of(room: Act1Room) -> &'static str {
    room.id()
}

fn dressing(room: Act1Room) -> Dressing {
    dressing_of(&root(), id_of(room)).unwrap().unwrap()
}

/// The room as launched, without and with the dressing (the launch's own pushes included).
fn runs(room: Act1Room) -> (f01_act1::Act1Run, f01_act1::Act1Run) {
    (
        f01_act1::launch_with(&root(), room, false).unwrap(),
        f01_act1::launch_with(&root(), room, true).unwrap(),
    )
}

// ------------------------------------------------------------------ ACT1-ART-A1: presentation only

/// ACT1-ART-A1: the dressing changes no collider (the guild slab, the humps and the ledge included),
/// spawn, exit, view or walking line, no blink barrier and no collision cell, and adds no collision
/// tile; the room, the sidecar (statics, locks, triggers, spawns, checkpoint, exit) and the rules are
/// the same value.
#[test]
fn the_dressing_changes_no_collider_spawn_exit_view_or_walk_line() {
    for room in Act1Room::ALL {
        let (plain, dressed) = runs(room);
        let (p, d) = (&plain.kit, &dressed.kit);
        assert_eq!(d.solid_rects, p.solid_rects, "{room:?}");
        assert_eq!(d.spawn, p.spawn);
        assert_eq!(d.exit, p.exit);
        assert_eq!(d.views, p.views);
        assert_eq!(d.base_walk_line_y, p.base_walk_line_y);
        assert_eq!(d.blink_barriers, p.blink_barriers);
        assert_eq!((d.width, d.height), (p.width, p.height));
        assert_eq!(d.collidable_cells, p.collidable_cells);
        assert_eq!(d.actor_z, p.actor_z);
        assert_eq!(d.deck_top_px, p.deck_top_px);
        let collision = |kit: &EnvKit| {
            kit.tiles
                .iter()
                .filter(|t| t.layer == "collision")
                .map(|t| (t.col, t.row, t.rect))
                .collect::<Vec<_>>()
        };
        assert_eq!(collision(d), collision(p));
        assert_ne!(
            format!("{:?}", d.pieces),
            format!("{:?}", p.pieces),
            "{room:?}: the dressing did happen"
        );
        assert_eq!(plain.rules.checkpoints, dressed.rules.checkpoints);
        assert_eq!(plain.loaded.room_hash, dressed.loaded.room_hash);
        assert_eq!(plain.loaded.sidecar_hash, dressed.loaded.sidecar_hash);
        assert_eq!(
            format!("{:?}", plain.loaded.encounter),
            format!("{:?}", dressed.loaded.encounter),
            "{room:?}: statics, locks, triggers, spawns, checkpoint and exit"
        );
    }
}

/// The solids the launch pushes are in both: the guild's slab, the chase's two humps and ledge.
#[test]
fn the_launch_solids_are_in_the_dressed_kit() {
    let has = |kit: &EnvKit, x, y, w, h| {
        kit.solid_rects
            .iter()
            .any(|r| (r.x, r.y, r.w, r.h) == (x, y, w, h))
    };
    let (_, chase) = runs(Act1Room::Chase);
    for (x, y, w, h) in [
        (1168, 564, 64, 16),
        (1456, 564, 64, 16),
        (2064, 564, 96, 16),
    ] {
        assert!(has(&chase.kit, x, y, w, h), "static ({x}, {y})");
    }
    let barricade = chase.barricade().unwrap();
    assert_eq!(
        (barricade.x, barricade.y, barricade.w, barricade.h),
        (2312, 304, 80, 276)
    );
    let (_, guild) = runs(Act1Room::Guild);
    let door = guild.door().unwrap();
    assert_eq!((door.x, door.y, door.w, door.h), (528, 336, 16, 244));
    let slab = guild
        .kit
        .solid_rects
        .iter()
        .filter(|r| r.w == guild.kit.width && r.h == 16)
        .count();
    assert_eq!(slab, 1, "the guild's DP-Q45 ceiling slab");
}

/// The dressing does not change the state hash: the room's smoke script hashes the same on every
/// tick with the plain kit and the dressed kit, in all three rooms.
#[test]
fn the_dressing_does_not_change_the_state_hash() {
    for room in Act1Room::ALL {
        let hashes = |dressed: bool| -> Vec<u64> {
            let run = f01_act1::launch_with(&root(), room, dressed).unwrap();
            let mut scene =
                crate::corridor::SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
            let (carl, donut) = crate::f01_late::Loadout::Story.flags();
            let mut combat = f01_act1::combat_for(&run, &mut scene, carl, donut, None).unwrap();
            (1..=500u64)
                .map(|tick| {
                    combat.step(
                        &mut scene,
                        run.fixture.input(tick.min(f01_act1::SMOKE_TICKS)),
                    );
                    combat.state_hash(&scene)
                })
                .collect()
        };
        let plain = hashes(false);
        assert_eq!(plain.len(), 500);
        assert_eq!(hashes(true), plain, "{room:?}");
    }
}

// ------------------------------------------------------------------ the dressing block

/// ACT1-ART-A3: every `remove_at` entry matches (`dress_with` reports zero unmatched) and drops exactly
/// one composer piece; an unmatched entry is an error, never skipped.
#[test]
fn every_remove_at_matches_exactly_one_piece() {
    let want = [
        (Act1Room::Chase, 18),
        (Act1Room::Shore, 2),
        (Act1Room::Guild, 2),
    ];
    for (room, n) in want {
        let d = dressing(room);
        assert_eq!(d.remove_at.len(), n, "{room:?}");
        let (plain, dressed) = runs(room);
        for r in &d.remove_at {
            let count = |kit: &EnvKit| {
                kit.pieces
                    .iter()
                    .filter(|p| {
                        p.layer == r.layer && p.piece == r.piece && p.x == r.x && p.y == r.y
                    })
                    .count()
            };
            assert_eq!(count(&plain.kit), 1, "{r:?} in the plain kit");
            assert_eq!(count(&dressed.kit), 0, "{r:?} dropped");
        }
        let contract = load_contract(&root(), ACT1_CONTRACT).unwrap();
        let mut missed = usize::MAX;
        crate::f01_boiler::compose_kit_hooked(
            &root(),
            &plain.loaded.room,
            &[],
            Some(contract),
            |l| {
                missed = dress_with(l, &d, ACT1_CONTRACT);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(missed, 0, "{room:?}");
        let mut bad = dressing(room);
        bad.remove_at[0].x += 1;
        let contract = load_contract(&root(), ACT1_CONTRACT).unwrap();
        let err = crate::f01_boiler::compose_kit_hooked(
            &root(),
            &plain.loaded.room,
            &[],
            Some(contract),
            |l| match dress_with(l, &bad, ACT1_CONTRACT) {
                0 => Ok(()),
                n => Err(format!("{n} unmatched")),
            },
        )
        .unwrap_err();
        assert!(err.contains("unmatched"), "{err}");
    }
}

/// The chase keeps the torches the spec keeps (the triple that stood clear of a set, the ledge's
/// neighbours) and loses the chain and the blue sill of the bricked bay.
#[test]
fn the_chase_keeps_its_other_torches_and_the_open_windows_keep_their_chains() {
    let (_, dressed) = runs(Act1Room::Chase);
    let torches = |x: i32| {
        dressed
            .kit
            .pieces
            .iter()
            .filter(|p| p.piece == "torch_wall" && p.x == x)
            .count()
    };
    for x in [768, 824, 864, 1104, 1160, 1440, 1496, 2112] {
        assert_eq!(torches(x), 0, "the torch at {x} is gone");
    }
    for x in [1200, 1536, 2168, 2208] {
        assert_eq!(torches(x), 1, "the torch at {x} stays");
    }
    let has = |piece: &str, x: i32, y: i32| {
        dressed
            .kit
            .pieces
            .iter()
            .any(|p| p.piece == piece && (p.x, p.y) == (x, y))
    };
    assert!(
        !has("arch_threshold", 576, 544),
        "no blue sill under the bricked bay"
    );
    assert!(!has("arch_chain", 592, 384));
    assert!(has("arch_chain", 256, 384) && has("arch_chain", 928, 384));
}

/// Every placed piece exists in the merged contract with the layer the placement names, lies inside
/// the room, is held by the kit, and nothing is collidable.
#[test]
fn every_placed_piece_exists_and_lies_inside_the_room() {
    let contract = load_contract(&root(), ACT1_CONTRACT).unwrap();
    let want = [
        (Act1Room::Chase, 23, 1),
        (Act1Room::Shore, 0, 0),
        (Act1Room::Guild, 16, 0),
    ];
    for (room, n, fills) in want {
        let d = dressing(room);
        let (_, dressed) = runs(room);
        assert!(d.remove.is_empty() && d.parallax.is_empty());
        assert_eq!(d.backwall_fill.len(), fills, "{room:?}");
        assert_eq!(d.place.len(), n, "{room:?}");
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
                r.x >= 0 && r.y >= 0 && r.right() <= dressed.kit.width && r.bottom() <= 580,
                "{} {r:?} is outside the {} x {} room",
                p.piece,
                dressed.kit.width,
                dressed.kit.height
            );
            assert!(
                dressed
                    .kit
                    .pieces
                    .iter()
                    .any(|d| d.piece == p.piece && d.x == p.x && d.y == p.y),
                "{} not in the kit",
                p.piece
            );
        }
    }
}

// ------------------------------------------------------------------ the sim rects

fn placed_rects(room: Act1Room, prefix: &str) -> Vec<IRect> {
    let contract = load_contract(&root(), ACT1_CONTRACT).unwrap();
    dressing(room)
        .place
        .iter()
        .filter(|p| p.piece.starts_with(prefix))
        .map(|p| {
            let def = contract.pieces.iter().find(|c| c.id == p.piece).unwrap();
            IRect {
                x: p.x,
                y: p.y,
                w: def.rect[2],
                h: def.rect[3],
            }
        })
        .collect()
}

/// The step faces cover exactly the sidecar's three statics, edge to edge (no sliver, no overhang).
#[test]
fn the_step_faces_cover_the_statics_exactly() {
    let faces = placed_rects(Act1Room::Chase, "act1_step_");
    assert_eq!(faces.len(), 5 + 5 + 7);
    let run = f01_act1::launch(&root(), Act1Room::Chase).unwrap();
    let statics: Vec<IRect> = run
        .loaded
        .encounter
        .statics
        .iter()
        .filter(|s| s.kind == crate::encounter::StaticKind::Solid)
        .map(|s| IRect {
            x: s.rect.x,
            y: s.rect.y,
            w: s.rect.w,
            h: s.rect.h,
        })
        .collect();
    assert_eq!(statics.len(), 3);
    for s in &statics {
        let mut row: Vec<&IRect> = faces
            .iter()
            .filter(|f| f.x >= s.x && f.right() <= s.right() && f.y == s.y)
            .collect();
        row.sort_by_key(|f| f.x);
        assert!(!row.is_empty(), "no face on {s:?}");
        let mut x = s.x;
        for f in row {
            assert_eq!((f.x, f.y, f.h), (x, s.y, s.h), "{f:?} on {s:?}");
            x += f.w;
        }
        assert_eq!(x, s.right(), "faces end at the static's right edge {s:?}");
    }
    assert_eq!(
        faces.iter().map(|f| f.w).sum::<i32>(),
        statics.iter().map(|s| s.w).sum::<i32>(),
        "no face lies off a static"
    );
}

/// The breach, the soffit slices, the niche, the timbers, the barricade and the grant box sit on their
/// sim rects exactly.
#[test]
fn the_pieces_sit_on_their_sim_rects() {
    let decor = |label: &str| {
        f01_act1::decor(Act1Room::Chase)
            .iter()
            .find(|d| d.label == label)
            .unwrap()
            .rect
    };
    let breach = placed_rects(Act1Room::Chase, "act1_gallery_breach")[0];
    let window = decor("window");
    assert_eq!(
        (breach.x, breach.y, breach.w, breach.h),
        (window.x, window.y, window.w, window.h)
    );
    assert_eq!((BREACH_AT.0, BREACH_AT.1), (window.x, window.y));
    let mut soffit = placed_rects(Act1Room::Chase, "act1_soffit_");
    soffit.sort_by_key(|r| r.x);
    let want = decor("soffit");
    assert_eq!(soffit.len(), 4);
    assert_eq!((soffit[0].x, soffit[0].y), (want.x, want.y));
    assert_eq!((SOFFIT_AT.0, SOFFIT_AT.1), (want.x, want.y));
    let mut x = want.x;
    for r in &soffit {
        assert_eq!((r.x, r.y, r.h), (x, want.y, want.h));
        x += r.w;
    }
    assert_eq!(x, want.x + want.w);
    for (i, label) in ["timber_a", "timber_b"].iter().enumerate() {
        let t = decor(label);
        assert_eq!(TIMBER_AT[i], (t.x, t.y), "{label}");
    }
    let rects = presenter_rects(&root()).unwrap();
    for (_, r) in &rects.timber {
        assert_eq!((r[2], r[3]), (80, 104), "the timber piece");
    }
    let t = decor("timber_a");
    assert_eq!((t.w, t.h), (80, 104));
    // The barricade piece covers the lock solid {2312, 304, 80, 276}.
    let run = f01_act1::launch(&root(), Act1Room::Chase).unwrap();
    let lock = run.barricade().unwrap();
    for (_, r) in &rects.barricade {
        assert_eq!((r[2], r[3]), (96, 280));
        assert!(BARRICADE_AT.0 <= lock.x && lock.right() <= BARRICADE_AT.0 + r[2]);
        assert_eq!(BARRICADE_AT.1, lock.y);
        assert!(lock.bottom() <= BARRICADE_AT.1 + r[3]);
    }
    // The niche stands on the alcove ledge (2064..2159, top 564) and the box on the guild floor.
    let niche = placed_rects(Act1Room::Chase, "act1_alcove_niche")[0];
    assert_eq!((niche.x, niche.bottom()), (2064, 564));
    for (_, r) in &rects.boxes {
        assert_eq!((r[2], r[3]), (24, 28));
        assert_eq!(BOX_AT.1 + r[3] - 1, 579, "floor contact row");
        let (z0, z1) = f01_act1::GUILD_ZONE_X;
        assert!(
            BOX_AT.0 >= z0 && BOX_AT.0 + r[2] <= z1,
            "inside the grant zone"
        );
    }
    // The guild portal: the composer's piece is where the overlay goes, and `_open` has its rect.
    let (_, guild) = runs(Act1Room::Guild);
    let portal = guild
        .kit
        .pieces
        .iter()
        .find(|p| p.piece == "guild_portal_closed")
        .unwrap();
    assert_eq!((portal.x, portal.y), PORTAL_AT);
    assert_eq!(rects.portal_open.1[2..], [portal.rect.w, portal.rect.h]);
    // The hearth is on the spawn side: clear of the zone, the guide's column and the door.
    let hearth = placed_rects(Act1Room::Guild, "refuge_hearth")[0];
    assert!(hearth.right() <= 160 && hearth.right() <= f01_act1::GUILD_ZONE_X.0);
}

/// The presenter pieces and their pure selection.
#[test]
fn the_presenter_pieces_are_picked_by_state() {
    assert_eq!(timber_piece(false), "timber_set_intact");
    assert_eq!(timber_piece(true), "timber_set_smashed");
    assert_eq!(barricade_piece(false), "timber_barricade_full_intact");
    assert_eq!(barricade_piece(true), "timber_barricade_full_smashed");
    assert_eq!(box_piece(false), "act1_grant_box_closed");
    assert_eq!(box_piece(true), "act1_grant_box_open");
    assert!(portal_open(true) && !portal_open(false));
    let rects = presenter_rects(&root()).unwrap();
    assert_eq!(rects.timber[0].0, "floor01_opening_props");
    assert_eq!(rects.boxes[0].0, "floor01_act1_keys");
    assert_eq!(rects.portal_open.0, "floor01_guild");
    // The presenter pieces are not placements.
    for room in Act1Room::ALL {
        for p in &dressing(room).place {
            assert!(
                !TIMBER_PIECES
                    .iter()
                    .chain(&BARRICADE_PIECES)
                    .chain(&BOX_PIECES)
                    .chain(&[PORTAL_OPEN_PIECE])
                    .any(|id| *id == p.piece),
                "{} is the presenter's",
                p.piece
            );
        }
    }
}

// ------------------------------------------------------------------ pixels

/// The opaque pixels of one atlas rect, from the committed pixel-grid source (no export needed).
fn opaque_rect(
    cache: &mut BTreeMap<String, Vec<Vec<u8>>>,
    atlas: &str,
    rect: [i32; 4],
) -> Vec<Vec<bool>> {
    if !cache.contains_key(atlas) {
        // Follow the contract chain (act-1, opening, ENV-01) for the atlas's grid.
        let mut path = ACT1_CONTRACT.to_owned();
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

/// ACT1-ART-A5: the undressed S2 bay (pier_l 512 .. pier_r 736 + 15, y 352..559) shows background
/// inside the `backwall_fill` rect only, and the dressed layout shows none in the whole bay.
#[test]
fn no_background_shows_in_the_bricked_bay() {
    let (plain, dressed) = runs(Act1Room::Chase);
    let before = coverage(&plain.kit, &ALL_TILES, &ALL_PIECES);
    let after = coverage(&dressed.kit, &ALL_TILES, &ALL_PIECES);
    let d = dressing(Act1Room::Chase);
    let fill = d.backwall_fill[0];
    assert_eq!((fill.x, fill.y, fill.w, fill.h), (584, 352, 152, 208));
    let g = gaps(&before, 584, 736, 352, 560);
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
    let g = gaps(&after, 584, 736, 352, 560);
    assert!(
        g.is_empty(),
        "{} uncovered pixels, first {:?}",
        g.len(),
        &g[..g.len().min(5)]
    );
}

/// The breach's void reaches the floor deck, so the gap under it is closed: no background pixel in
/// the breach's own rect either (its transparent pixels show the bricked wall behind).
#[test]
fn no_background_shows_behind_the_breach() {
    let (_, dressed) = runs(Act1Room::Chase);
    let after = coverage(&dressed.kit, &ALL_TILES, &ALL_PIECES);
    let g = gaps(&after, 696, 888, 352, 560);
    assert!(
        g.is_empty(),
        "{} uncovered pixels behind the breach, first {:?}",
        g.len(),
        &g[..g.len().min(5)]
    );
}

/// ACT1-ART-A4: every dressing placement intersecting y 561..579 draws above the collision layer, and
/// the flat floor pieces (the step faces) are on `structure`; only the guild's furniture (a standing
/// `prop` of the refuge kit, which stands on the walking line) is on `prop`.
#[test]
fn the_floor_pieces_draw_above_the_collision_layer() {
    let contract = load_contract(&root(), ACT1_CONTRACT).unwrap();
    let order = |name: &str| {
        contract
            .layers
            .iter()
            .find(|l| l.name == name)
            .unwrap_or_else(|| panic!("layer {name}"))
            .draw_order
    };
    let mut n = 0;
    for room in Act1Room::ALL {
        for p in &dressing(room).place {
            let def = contract.pieces.iter().find(|c| c.id == p.piece).unwrap();
            if p.y < 580 && p.y + def.rect[3] > 561 {
                assert!(
                    order(&p.layer) > order("collision"),
                    "{} on {} is under the floor deck",
                    p.piece,
                    p.layer
                );
                assert!(
                    p.layer == "structure" || (p.layer == "prop" && p.piece.starts_with("refuge_")),
                    "{} on {}",
                    p.piece,
                    p.layer
                );
                n += 1;
            }
        }
    }
    assert_eq!(
        n,
        17 + 6,
        "the 17 step faces and the guild's hearth, barrel, stool, desk, candle, shelf"
    );
}

/// No `prop` placement's opaque pixel lies inside an opening that is not bricked up, above y 520 (the
/// pieces' real pixels are used, not their rects).
#[test]
fn no_prop_pixel_stands_in_an_open_window_above_y_520() {
    let contract = load_contract(&root(), ACT1_CONTRACT).unwrap();
    let mut cache = BTreeMap::new();
    for room in Act1Room::ALL {
        let (_, dressed) = runs(room);
        // Openings (pier_l + 40 .. pier_l + 191); the chase's bay at pier_l 512 is bricked up.
        let open: Vec<(i32, i32)> = dressed
            .kit
            .pieces
            .iter()
            .filter(|p| {
                p.piece == "pier_l" && p.y == 32 && !(room == Act1Room::Chase && p.x == 512)
            })
            .map(|p| (p.x + 40, p.x + 191))
            .collect();
        for p in dressing(room).place.iter().filter(|p| p.layer == "prop") {
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
    }
}

/// Torch clearance: no new `prop` or `structure` piece has an opaque pixel inside a remaining
/// `torch_wall` rect (16 x 56). A wall decal sits behind the torch.
#[test]
fn no_new_piece_covers_a_remaining_torch() {
    let mut cache = BTreeMap::new();
    for room in Act1Room::ALL {
        let (_, dressed) = runs(room);
        let torches: Vec<_> = dressed
            .kit
            .pieces
            .iter()
            .filter(|p| p.piece == "torch_wall")
            .collect();
        assert!(!torches.is_empty());
        let placed = dressing(room).place;
        for p in dressed.kit.pieces.iter().filter(|p| {
            (p.layer == "prop" || p.layer == "structure")
                && placed
                    .iter()
                    .any(|q| q.piece == p.piece && (q.x, q.y) == (p.x, p.y))
        }) {
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
}

// ------------------------------------------------------------------ ACT1-ART-A6: lights

/// The light trade of each room (spec section 8) and the budget: the worst 640 px view stays at or
/// under `MAX_LIGHTS` and no light is dropped.
#[test]
fn the_lights_stay_inside_the_light_budget() {
    use crate::lighting::{MAX_LIGHTS, ViewRect};
    // (room, removed torches, added emitters): a torch holds two lights; the hearth two, the candle one,
    // the niche one.
    for (room, trade) in [
        (Act1Room::Chase, -15), // -16 (eight torches), +1 (the niche)
        (Act1Room::Shore, -2),
        (Act1Room::Guild, 1), // -2 (one torch), +2 (the hearth) +1 (the candle)
    ] {
        let (plain, dressed) = runs(room);
        let p = plain.kit.lighting.as_ref().expect("the plain room is lit");
        let d = dressed
            .kit
            .lighting
            .as_ref()
            .expect("the dressed room is lit");
        assert_eq!(
            d.emitters.len() as i32,
            p.emitters.len() as i32 + trade,
            "{room:?}"
        );
        let view = *dressed.kit.views.values().next().unwrap();
        let mut worst = 0;
        for left in (0..=dressed.kit.width - 640).step_by(8) {
            let (g, dropped) = d.gather(
                &dressed.kit,
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
        assert!(d.worst_case_lights <= MAX_LIGHTS);
    }
}

// ------------------------------------------------------------------ the block lookups

/// A room id outside the contract composes plain; a malformed block is an error; the three act-1
/// rooms are dressed.
#[test]
fn a_room_without_a_dressing_block_composes_plain() {
    let contract: Value =
        serde_json::from_str(&std::fs::read_to_string(root().join(ACT1_CONTRACT)).unwrap())
            .unwrap();
    assert!(
        dressing_from(&contract, "act1_synthetic")
            .unwrap()
            .is_none()
    );
    assert!(dressing_of(&root(), "corral").unwrap().is_none());
    for room in Act1Room::ALL {
        assert!(dressing_from(&contract, room.id()).unwrap().is_some());
        assert!(is_dressed(&root(), room));
    }
    let mut null_block = contract.clone();
    null_block["room_dressing"]["act1_synthetic"] = Value::Null;
    assert!(
        dressing_from(&null_block, "act1_synthetic")
            .unwrap()
            .is_none()
    );
    let mut bad = contract.clone();
    bad["room_dressing"]["act1_synthetic"] = json!({"place": 3});
    assert!(dressing_from(&bad, "act1_synthetic").is_err());
    let loaded = f01_act1::launch_with(&root(), Act1Room::Shore, false)
        .unwrap()
        .loaded;
    let plain = crate::f01_boiler::compose_kit(&root(), &loaded.room).unwrap();
    let synthetic = compose_by_id(&root(), &loaded, "act1_synthetic", true).unwrap();
    assert_eq!(
        format!("{:?}", synthetic.pieces),
        format!("{:?}", plain.pieces)
    );
}

// ------------------------------------------------------------------ evidence helper

/// `ACT1_LAYOUT_DIR=<dir> cargo test --lib -- --ignored dump_act1_layouts`: the composed layouts of
/// each act-1 room, plain and (where the contract has a block) dressed, for the evidence scripts
/// and the art unittest.
#[test]
#[ignore = "evidence helper: writes the composed layouts for the Python measurements"]
fn dump_act1_layouts() {
    let Ok(dir) = std::env::var("ACT1_LAYOUT_DIR") else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    for room in Act1Room::ALL {
        let loaded = f01_act1::launch_with(&root(), room, false).unwrap().loaded;
        let block = dressing_of(&root(), room.id()).ok().flatten();
        for dressed in [false, true] {
            if dressed && block.is_none() {
                continue;
            }
            let mut out = None;
            let contract = if dressed {
                Some(load_contract(&root(), ACT1_CONTRACT).unwrap())
            } else {
                None
            };
            crate::f01_boiler::compose_kit_hooked(&root(), &loaded.room, &[], contract, |layout| {
                if let (true, Some(d)) = (dressed, &block) {
                    assert_eq!(dress_with(layout, d, ACT1_CONTRACT), 0);
                    layout["kit_contract"] = json!(ACT1_CONTRACT);
                }
                out = Some(layout.clone());
                Ok(())
            })
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
