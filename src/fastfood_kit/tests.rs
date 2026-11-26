use super::*;
use crate::camera_lab::room::IRect as RoomRect;
use crate::env_kit::IRect;
use crate::f01_late::Loadout;
use crate::f01_rest::{self, RestRoom};
use crate::input::RawInput;
use crate::rest_fixtures::{BOT, DonutScript, Party, RestSim};
use serde_json::json;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn overlaps(r: IRect, x0: i32, x1: i32, y0: i32, y1: i32) -> bool {
    r.x < x1 && r.right() > x0 && r.y < y1 && r.bottom() > y0
}

/// The refuge's composition without and with the dressing, before the launch's own pushes.
fn kits() -> (EnvKit, EnvKit) {
    let run = f01_rest::launch_with(&root(), RestRoom::Refuge, false).unwrap();
    let plain = crate::f01_boiler::compose_kit(&root(), &run.loaded.room).unwrap();
    let dressed = compose_fastfood_kit(&root(), &run.loaded.room).unwrap();
    (plain, dressed)
}

fn story_sim(party: Party) -> RestSim {
    let (carl, donut) = Loadout::Story.flags();
    RestSim::new(&root(), 360, carl, donut, party).unwrap()
}

fn swap() -> RawInput {
    RawInput {
        swap: true,
        ..RawInput::NONE
    }
}

// ------------------------------------------------------------------ FF-A1: presentation only

/// Spec section 4 / FF-A1: the dressing changes no collider, spawn, exit, view or walking
/// line, no blink barrier and no collision cell, and adds no tile.
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
    // No collision tile is added, removed or moved: `backwall_fill` only bricks up bay A's
    // opening with non-collidable backwall tiles (FF-AD12), the composer's own brick.
    let layer = |kit: &EnvKit, name: &str| kit.tiles.iter().filter(|t| t.layer == name).count();
    assert_eq!(layer(&dressed, "collision"), layer(&plain, "collision"));
    assert_eq!(dressed.tiles.len() - plain.tiles.len(), {
        layer(&dressed, "backwall") - layer(&plain, "backwall")
    });
    assert!(layer(&dressed, "backwall") > layer(&plain, "backwall"));
    assert_eq!(
        dressed
            .tiles
            .iter()
            .filter(|t| t.layer == "collision")
            .map(|t| (t.col, t.row, t.rect))
            .collect::<Vec<_>>(),
        plain
            .tiles
            .iter()
            .filter(|t| t.layer == "collision")
            .map(|t| (t.col, t.row, t.rect))
            .collect::<Vec<_>>()
    );
    assert_eq!(dressed.actor_z, plain.actor_z);
    assert_eq!(dressed.deck_top_px, plain.deck_top_px);
    assert_eq!(dressed.strips.len(), plain.strips.len());
    // The dressing did happen.
    assert!(dressed.pieces.len() > plain.pieces.len());
    assert!(dressed.pieces.iter().any(|p| p.piece == "ff_light_strip"));
}

/// The launched refuge, dressed and plain: the same solids (the ceiling slab 336..351 and
/// every platform included), spawn, exit, views, walking line, the same lock rect and the
/// same checkpoint columns.
#[test]
fn the_launched_refuge_is_the_same_room_with_and_without_the_dressing() {
    let plain = f01_rest::launch_with(&root(), RestRoom::Refuge, false).unwrap();
    let dressed = f01_rest::launch(&root(), RestRoom::Refuge).unwrap();
    assert_eq!(dressed.kit.solid_rects, plain.kit.solid_rects);
    assert_eq!(dressed.kit.spawn, plain.kit.spawn);
    assert_eq!(dressed.kit.exit, plain.kit.exit);
    assert_eq!(dressed.kit.views, plain.kit.views);
    assert_eq!(dressed.kit.base_walk_line_y, plain.kit.base_walk_line_y);
    assert_eq!(dressed.kit.blink_barriers, plain.kit.blink_barriers);
    assert_eq!(dressed.door(), plain.door());
    assert_eq!(dressed.rules.checkpoints, plain.rules.checkpoints);
    assert_eq!(dressed.loaded.room_hash, plain.loaded.room_hash);
    assert_eq!(dressed.loaded.sidecar_hash, plain.loaded.sidecar_hash);
    let slab = dressed.kit.solid_rects.last().unwrap();
    assert_eq!(
        (slab.x, slab.y, slab.w, slab.h),
        (0, 336, 1600, 16),
        "the lane ceiling slab"
    );
    assert_eq!(
        dressed.door(),
        Some(RoomRect::new(1440, 352, 16, 228)),
        "the staff door's lock rect"
    );
    assert_eq!(dressed.kit.walking_line_at(112), Some(580));
    // The room's own default is the dressed one.
    assert!(dressed.kit.pieces.len() > plain.kit.pieces.len());
}

/// The camp and the aftermath keep `compose_kit` byte for byte (the dressing is the
/// refuge's only).
#[test]
fn the_camp_and_the_aftermath_kits_are_unchanged() {
    for room in [RestRoom::Camp, RestRoom::Aftermath] {
        let run = f01_rest::launch(&root(), room).unwrap();
        let plain = crate::f01_boiler::compose_kit(&root(), &run.loaded.room).unwrap();
        assert_eq!(
            format!("{:?}", run.kit.pieces),
            format!("{:?}", plain.pieces),
            "{room:?}: pieces"
        );
        assert_eq!(
            format!("{:?}", run.kit.strips),
            format!("{:?}", plain.strips),
            "{room:?}: strips"
        );
        assert!(
            run.kit
                .pieces
                .iter()
                .all(|p| !p.piece.starts_with("ff_") && !p.atlas.starts_with("floor01_ff_")),
            "{room:?} holds no fast-food piece"
        );
    }
}

// ------------------------------------------------------------------ the dressing block

/// Every placed piece exists in the merged contract with the layer the placement names,
/// lies inside the room and is held by the kit.
#[test]
fn every_placed_piece_exists_and_lies_inside_the_room() {
    let contract = load_contract(&root(), FASTFOOD_CONTRACT).unwrap();
    let d = dressing(&root()).unwrap();
    let (_, dressed) = kits();
    assert!(d.remove.is_empty() && d.parallax.is_empty());
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

/// Every `remove_at` entry matches exactly one composer piece, which the dressing drops.
#[test]
fn every_remove_at_matches_exactly_one_piece() {
    let d = dressing(&root()).unwrap();
    let (plain, dressed) = kits();
    assert_eq!(
        d.remove_at.len(),
        10,
        "four torches, four pools, the grime and bay B's chain"
    );
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
    // The threshold torches (48, 96) and the vestibule torch (1536) stay.
    for (x, y) in [(48, 432), (96, 448), (1536, 432)] {
        assert!(
            dressed
                .pieces
                .iter()
                .any(|p| p.piece == "torch_wall" && (p.x, p.y) == (x, y)),
            "the torch at ({x}, {y}) stays"
        );
    }
    // Bay A keeps its catenary (the catacomb side); bay B's would hang across the window (review F2).
    assert!(
        dressed
            .pieces
            .iter()
            .any(|p| p.piece == "arch_chain" && (p.x, p.y) == (160, 384)),
        "bay A keeps its chain"
    );
    assert!(
        !dressed
            .pieces
            .iter()
            .any(|p| p.piece == "arch_chain" && (p.x, p.y) == (912, 384)),
        "bay B's chain is gone"
    );
    // An unmatched entry is an error, never silently skipped.
    let mut bad = dressing(&root()).unwrap();
    bad.remove_at[0].x += 1;
    let contract = load_contract(&root(), FASTFOOD_CONTRACT).unwrap();
    let room = f01_rest::launch_with(&root(), RestRoom::Refuge, false)
        .unwrap()
        .loaded
        .room;
    let err = crate::f01_boiler::compose_kit_hooked(&root(), &room, &[], Some(contract), |l| {
        match dress_with(l, &bad, FASTFOOD_CONTRACT) {
            0 => Ok(()),
            n => Err(format!("{n} unmatched")),
        }
    })
    .unwrap_err();
    assert!(err.contains("unmatched"), "{err}");
}

/// The door, the box, the sparkle and the bell are presenter-drawn, never placed; nothing
/// is placed over the lock rect (1440, 352, 16, 228); no piece is collidable.
#[test]
fn the_presenter_pieces_are_not_placed_and_nothing_covers_the_lock() {
    let contract = load_contract(&root(), FASTFOOD_CONTRACT).unwrap();
    let d = dressing(&root()).unwrap();
    let (_, dressed) = kits();
    let presenter: Vec<&str> = DOOR_PIECES
        .iter()
        .chain(&BOX_PIECES)
        .chain(&SPARKLE_PIECES)
        .chain(&BELL_PIECES)
        .copied()
        .collect();
    for p in &d.place {
        assert!(
            !presenter.contains(&p.piece.as_str()),
            "{} is placed",
            p.piece
        );
        let def = contract.pieces.iter().find(|c| c.id == p.piece).unwrap();
        let r = IRect {
            x: p.x,
            y: p.y,
            w: def.rect[2],
            h: def.rect[3],
        };
        assert!(
            !overlaps(r, 1440, 1456, 352, 580),
            "{} {r:?} covers the lock rect",
            p.piece
        );
    }
    assert!(
        dressed
            .pieces
            .iter()
            .all(|p| !presenter.contains(&p.piece.as_str())),
        "no presenter piece in the kit"
    );
    for id in presenter {
        assert!(
            contract.pieces.iter().any(|c| c.id == id),
            "{id} is in the contract (state group)"
        );
    }
}

/// The platform skins lie on their plank tiles' grid rows (platform y - 4) across the
/// platform's width, and the ledge covers the shelf's plank (spec 9.2, FF-AD10).
#[test]
fn the_skins_cover_the_plank_tiles_of_the_three_platforms() {
    let run = f01_rest::launch(&root(), RestRoom::Refuge).unwrap();
    let platforms = &run.loaded.room.platforms;
    assert_eq!(platforms.len(), 3);
    for (i, (id, w)) in [
        ("ff_deck_skin", 96),
        ("ff_slide_top_skin", 128),
        ("ff_pass_ledge", 96),
    ]
    .into_iter()
    .enumerate()
    {
        let plat = platforms[i].rect;
        let piece = run
            .kit
            .pieces
            .iter()
            .find(|p| p.piece == id)
            .unwrap_or_else(|| panic!("{id} is placed"));
        assert_eq!((piece.x, piece.y), (plat.x, plat.y - 4), "{id}");
        assert_eq!(piece.rect.w, w, "{id}");
        assert_eq!(piece.layer, "structure", "{id}");
    }
}

/// The floor decals lie in the deck's back half, y 561..572.
#[test]
fn the_floor_decals_lie_in_the_decks_back_half() {
    let d = dressing(&root()).unwrap();
    let contract = load_contract(&root(), FASTFOOD_CONTRACT).unwrap();
    let mut n = 0;
    for p in d
        .place
        .iter()
        .filter(|p| p.piece.starts_with("ff_floor_checker") || p.piece == "ff_floor_spill")
    {
        let def = contract.pieces.iter().find(|c| c.id == p.piece).unwrap();
        assert!(
            p.y >= 561 && p.y + def.rect[3] <= 573,
            "{} at y {}",
            p.piece,
            p.y
        );
        n += 1;
    }
    assert_eq!(n, 36 + 2);
}

// ------------------------------------------------------------------ review F1 and F5

/// Review F1: the checker floor, its broken edge and the spill draw **above** the collision
/// layer (the floor deck), and the posts' feet draw over them (they are listed first).
#[test]
fn the_floor_decals_draw_above_the_collision_layer() {
    let contract = load_contract(&root(), FASTFOOD_CONTRACT).unwrap();
    let order = |name: &str| {
        contract
            .layers
            .iter()
            .find(|l| l.name == name)
            .unwrap_or_else(|| panic!("layer {name}"))
            .draw_order
    };
    let d = dressing(&root()).unwrap();
    let floor: Vec<_> = d
        .place
        .iter()
        .filter(|p| (561..=572).contains(&p.y))
        .collect();
    assert_eq!(floor.len(), 38, "36 checker tiles, the edge and the spill");
    for p in &floor {
        assert!(
            order(&p.layer) > order("collision"),
            "{} on {} is under the floor deck",
            p.piece,
            p.layer
        );
    }
    let pos = |id: &str| d.place.iter().position(|p| p.piece == id).unwrap();
    let last_floor = d
        .place
        .iter()
        .rposition(|p| p.piece.starts_with("ff_floor_"))
        .unwrap();
    assert!(
        last_floor < pos("ff_play_posts"),
        "the posts' feet draw over the checker"
    );
}

/// The opaque pixels of one atlas rect, from the committed pixel-grid source (no export
/// needed): `rows[y][x]` is true where the cell is not transparent.
fn opaque_rect(
    cache: &mut std::collections::BTreeMap<String, Vec<Vec<u8>>>,
    atlas: &str,
    rect: [i32; 4],
) -> Vec<Vec<bool>> {
    use serde_json::Value;
    if !cache.contains_key(atlas) {
        // Follow the contract chain (fast-food, opening, catacomb) for the atlas's grid.
        let mut path = FASTFOOD_CONTRACT.to_owned();
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

/// Review F5: no background (the gallery parallax) shows in either bay: every pixel of
/// x 264..383 and x 904..1135, y 304..575 is covered by an opaque backwall tile, collision
/// tile or piece of the dressed room (the pier's inner columns are transparent, so the
/// bricked-up opening and the rock fill must reach them).
#[test]
fn no_background_shows_in_either_bay() {
    let (_, dressed) = kits();
    let mut cache = std::collections::BTreeMap::new();
    let mut covered = vec![vec![false; dressed.width as usize]; 580];
    let mut paint = |x0: i32, y0: i32, opaque: Vec<Vec<bool>>| {
        for (dy, row) in opaque.iter().enumerate() {
            for (dx, &on) in row.iter().enumerate() {
                let (x, y) = (x0 + dx as i32, y0 + dy as i32);
                if on && x >= 0 && y >= 0 && (y as usize) < covered.len() && x < dressed.width {
                    covered[y as usize][x as usize] = true;
                }
            }
        }
    };
    for t in &dressed.tiles {
        let rect = [t.rect.x, t.rect.y, t.rect.w, t.rect.h];
        paint(
            t.col * dressed.tile,
            t.row * dressed.tile,
            opaque_rect(&mut cache, &t.atlas, rect),
        );
    }
    for p in &dressed.pieces {
        let rect = [p.rect.x, p.rect.y, p.rect.w, p.rect.h];
        paint(p.x, p.y, opaque_rect(&mut cache, &p.atlas, rect));
    }
    for (name, x0, x1) in [("bay A", 264, 383), ("bay B", 904, 1135)] {
        let mut gaps = Vec::new();
        for (y, row) in covered.iter().enumerate().take(576).skip(304) {
            for x in x0..=x1 {
                if !row[x as usize] {
                    gaps.push((x, y));
                }
            }
        }
        assert!(
            gaps.is_empty(),
            "{name}: {} uncovered pixels, first {:?}",
            gaps.len(),
            &gaps[..gaps.len().min(5)]
        );
    }
}

// ------------------------------------------------------------------ FF-A7: lights

/// The strips replace the four dining-room torches: net minus 8 plus 6 emitters, and the
/// worst 640 px view stays inside the light budget.
#[test]
fn the_refuge_lights_stay_inside_the_light_budget() {
    use crate::lighting::{MAX_LIGHTS, ViewRect};
    let (plain, dressed) = kits();
    let p = plain.lighting.as_ref().expect("the plain room is lit");
    let d = dressed.lighting.as_ref().expect("the dressed room is lit");
    assert_eq!(
        d.emitters.len() + 8,
        p.emitters.len() + 6,
        "minus 8 (four torches with a flame and a pool each), plus 6 strips"
    );
    assert_eq!(
        d.emitters.iter().filter(|e| e.id.contains("tube")).count(),
        6,
        "six strips emit one light each"
    );
    let view = dressed.views["abc_640x360"];
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
        worst <= p.emitters.len().max(8),
        "the dressed room is no busier than the greybox's lights"
    );
}

// ------------------------------------------------------------------ the pure presenter

#[test]
fn the_door_frame_reads_the_lock_state_first() {
    // Closed lock: closed, whatever the event says.
    assert_eq!(door_frame(false, None, 0), DoorFrame::Closed);
    assert_eq!(door_frame(false, Some(370), 375), DoorFrame::Closed);
    // Open at 370: rising 1..12 on 370..381, then open.
    assert_eq!(door_frame(true, Some(370), 369), DoorFrame::Open, "rewound");
    for k in 1..=12u64 {
        assert_eq!(
            door_frame(true, Some(370), 369 + k),
            DoorFrame::Rising(k as u8)
        );
    }
    assert_eq!(door_frame(true, Some(370), 382), DoorFrame::Open);
    assert_eq!(door_frame(true, Some(370), 900), DoorFrame::Open);
    // An open lock with no event (a carried or restored lock) is open at once.
    assert_eq!(door_frame(true, None, 5), DoorFrame::Open);
    // The frames index the 14 atlas pieces.
    assert_eq!(DoorFrame::Closed.piece(), "ff_staff_door_closed");
    assert_eq!(DoorFrame::Rising(1).piece(), "ff_staff_door_rise_01");
    assert_eq!(DoorFrame::Rising(12).piece(), "ff_staff_door_rise_12");
    assert_eq!(DoorFrame::Open.piece(), "ff_staff_door_open");
    // The rise table ends at the open pose (a 224 px raise: a 4-row lip stays).
    assert_eq!(RISE[11], 224);
    assert!(RISE.windows(2).all(|w| w[0] < w[1]), "the panel only rises");
    assert!(
        RISE[1] >= 37 && RISE[2] >= 59,
        "Donut clears on tick 2, Carl on tick 3"
    );
}

#[test]
fn the_box_frame_is_empty_iff_the_boost_is_held() {
    // Not held: full, the glint alternating every 24 presentation ticks.
    assert_eq!(box_frame(false, None, 0, 0), BoxFrame::FullA);
    assert_eq!(box_frame(false, None, 0, 23), BoxFrame::FullA);
    assert_eq!(box_frame(false, None, 0, 24), BoxFrame::FullB);
    assert_eq!(box_frame(false, None, 0, 47), BoxFrame::FullB);
    assert_eq!(box_frame(false, None, 0, 48), BoxFrame::FullA);
    // A grant tick with the boost not held (a rewound state) never draws a sparkle.
    assert_eq!(box_frame(false, Some(326), 326, 0), BoxFrame::FullA);
    // Held: sparkle 0..2 over empty on the 12 ticks from the grant, then empty.
    for now in 326..338u64 {
        let k = ((now - 326) / 4) as u8;
        assert_eq!(box_frame(true, Some(326), now, 7), BoxFrame::Sparkle(k));
    }
    assert_eq!(box_frame(true, Some(326), 338, 7), BoxFrame::Empty);
    assert_eq!(box_frame(true, Some(326), 2000, 7), BoxFrame::Empty);
    // A launch that starts with the boost has no grant: empty from tick 0.
    assert_eq!(box_frame(true, None, 0, 0), BoxFrame::Empty);
    // The sparkle sits over the empty frame.
    assert_eq!(BoxFrame::Sparkle(1).index(), 2);
    assert_eq!(BoxFrame::Sparkle(1).sparkle(), Some(1));
    assert_eq!(BoxFrame::Empty.sparkle(), None);
    assert_eq!(BoxFrame::FullB.index(), 1);
}

#[test]
fn the_bell_rings_once_on_the_open_tick() {
    assert_eq!(bell_frame(false, Some(370), 370), BellFrame::Idle);
    assert_eq!(bell_frame(true, None, 370), BellFrame::Idle);
    assert_eq!(bell_frame(true, Some(370), 369), BellFrame::Idle);
    for now in 370..374 {
        assert_eq!(bell_frame(true, Some(370), now), BellFrame::DingA);
    }
    for now in 374..378 {
        assert_eq!(bell_frame(true, Some(370), now), BellFrame::DingB);
    }
    for now in 378..382 {
        assert_eq!(bell_frame(true, Some(370), now), BellFrame::DingA);
    }
    assert_eq!(bell_frame(true, Some(370), 382), BellFrame::Idle);
}

/// The event helpers: a fixture restart forgets the attempt's earlier events.
#[test]
fn the_event_helpers_follow_the_attempt() {
    let boost = crate::donut_prog::JUMP_REACH_BOOST.as_str();
    let events = vec![
        CombatEvent::CapabilityGranted {
            tick: 326,
            hero: 1,
            capability: boost,
        },
        CombatEvent::SurfaceLockOpened {
            tick: 370,
            lock: 0,
            hero: 1,
        },
    ];
    assert_eq!(last_grant_tick(&events), Some(326));
    assert_eq!(last_open_tick(&events), Some(370));
    let mut after = events.clone();
    after.push(CombatEvent::Restart {
        tick: 414,
        deaths: 0,
    });
    assert_eq!(last_grant_tick(&after), None);
    assert_eq!(last_open_tick(&after), None);
    after.push(CombatEvent::SurfaceLockOpened {
        tick: 380,
        lock: 0,
        hero: 1,
    });
    assert_eq!(last_open_tick(&after), Some(380), "the second crossing");
    assert_eq!(last_grant_tick(&[]), None);
}

// ------------------------------------------------------------------ FF-A3, FF-A4 on the fixtures

/// `rest_refuge_solo_donut`: sample the presenter on every tick of the run. The box is full
/// before tick 326, sparkles on 326..337 and is empty after; the door is closed before tick
/// 370, rising 1..12 on 370..381 and open after; the bell rings on 370..381.
#[test]
fn the_frames_follow_the_solo_donut_fixture() {
    let mut sim = story_sim(Party::Solo { uncontrolled: BOT });
    let e = sim.run.loaded.encounter.exit;
    let exit_x = (e.x + e.w / 2) as f32;
    let mut script = DonutScript::default();
    let at = |sim: &RestSim| -> (DoorFrame, BoxFrame, BellFrame) {
        let held = sim.combat.story_holds(crate::donut_prog::JUMP_REACH_BOOST);
        let open = sim.door_open();
        let (now, events) = (sim.tick(), sim.combat.events.as_slice());
        (
            door_frame(open, last_open_tick(events), now),
            box_frame(held, last_grant_tick(events), now, now),
            bell_frame(open, last_open_tick(events), now),
        )
    };
    // Tick 0: closed and full.
    let (d, b, _) = at(&sim);
    assert_eq!((d, b), (DoorFrame::Closed, BoxFrame::FullA));
    sim.step([swap(), RawInput::NONE]);
    let mut seen = (0, 0, 0);
    while sim.tick() < 420 && !sim.complete() {
        let input = script.decide(
            sim.x_of(crate::party::HeroId::Donut),
            sim.scene.world.body(crate::party::HeroId::Donut).grounded,
            sim.door_open(),
            sim.tick(),
            exit_x,
        );
        sim.step([input, RawInput::NONE]);
        let tick = sim.tick();
        let (door, boxed, bell) = at(&sim);
        match tick {
            ..=325 => assert!(
                matches!(boxed, BoxFrame::FullA | BoxFrame::FullB),
                "tick {tick}: {boxed:?}"
            ),
            326..=337 => {
                assert_eq!(
                    boxed,
                    BoxFrame::Sparkle(((tick - 326) / 4) as u8),
                    "tick {tick}"
                );
                seen.0 += 1;
            }
            _ => assert_eq!(boxed, BoxFrame::Empty, "tick {tick}"),
        }
        match tick {
            ..=369 => assert_eq!(door, DoorFrame::Closed, "tick {tick}"),
            370..=381 => {
                assert_eq!(door, DoorFrame::Rising((tick - 369) as u8), "tick {tick}");
                seen.1 += 1;
                assert_ne!(bell, BellFrame::Idle, "tick {tick}");
                seen.2 += 1;
            }
            _ => {
                assert_eq!(door, DoorFrame::Open, "tick {tick}");
                assert_eq!(bell, BellFrame::Idle, "tick {tick}");
            }
        }
    }
    assert_eq!(seen, (12, 12, 12));
}

/// A fixture restart shows closed and full again; a checkpoint respawn with the lock open
/// shows open at once, and the box stays empty.
#[test]
fn a_restart_closes_the_door_and_a_respawn_keeps_it_open() {
    let mut sim = story_sim(Party::Solo { uncontrolled: BOT });
    let e = sim.run.loaded.encounter.exit;
    let exit_x = (e.x + e.w / 2) as f32;
    let mut script = DonutScript::default();
    sim.step([swap(), RawInput::NONE]);
    while sim.tick() < 400 && !sim.complete() {
        let input = script.decide(
            sim.x_of(crate::party::HeroId::Donut),
            sim.scene.world.body(crate::party::HeroId::Donut).grounded,
            sim.door_open(),
            sim.tick(),
            exit_x,
        );
        sim.step([input, RawInput::NONE]);
    }
    let frames = |sim: &RestSim| {
        let held = sim.combat.story_holds(crate::donut_prog::JUMP_REACH_BOOST);
        let (now, events) = (sim.tick(), sim.combat.events.as_slice());
        (
            door_frame(sim.door_open(), last_open_tick(events), now),
            box_frame(held, last_grant_tick(events), now, 0),
        )
    };
    assert_eq!(frames(&sim), (DoorFrame::Open, BoxFrame::Empty));
    // Checkpoint respawn: the lock stays open and the boost stays held.
    sim.combat.respawn(&mut sim.scene);
    assert!(sim.door_open());
    assert_eq!(frames(&sim), (DoorFrame::Open, BoxFrame::Empty));
    // Fixture restart: the lock closes and the box re-arms.
    sim.combat.restart(&mut sim.scene);
    assert!(!sim.door_open());
    assert_eq!(frames(&sim), (DoorFrame::Closed, BoxFrame::FullA));
}

/// The `All` loadout starts with the boost held: the box is empty from tick 0 and the door
/// still starts closed.
#[test]
fn the_all_loadout_shows_the_empty_box_from_tick_zero() {
    let (carl, donut) = Loadout::All.flags();
    let sim = RestSim::new(&root(), 360, carl, donut, Party::Solo { uncontrolled: BOT }).unwrap();
    let held = sim.combat.story_holds(crate::donut_prog::JUMP_REACH_BOOST);
    assert!(held, "the All loadout holds Light on Your Feet");
    let events = sim.combat.events.as_slice();
    assert_eq!(
        box_frame(held, last_grant_tick(events), sim.tick(), 0),
        BoxFrame::Empty
    );
    assert_eq!(
        door_frame(sim.door_open(), last_open_tick(events), sim.tick()),
        DoorFrame::Closed
    );
}

// ------------------------------------------------------------------ FF-A2: the state hash

/// The refuge fixtures hash the same on every tick with the plain kit and the dressed kit.
#[test]
fn the_dressing_does_not_change_the_state_hash() {
    let (carl, donut) = Loadout::Story.flags();
    let run_hashes = |dressed: bool, swap_first: bool| -> Vec<u64> {
        let run = f01_rest::launch_with(&root(), RestRoom::Refuge, dressed).unwrap();
        let mut sim = RestSim::from_run(
            run,
            360,
            carl,
            donut,
            Party::Solo { uncontrolled: BOT },
            None,
        )
        .unwrap();
        let e = sim.run.loaded.encounter.exit;
        let exit_x = (e.x + e.w / 2) as f32;
        let mut script = DonutScript::default();
        let mut out = Vec::new();
        if swap_first {
            sim.step([swap(), RawInput::NONE]);
            out.push(sim.hash());
        }
        while sim.tick() < 440 && !sim.complete() {
            let input = if swap_first {
                script.decide(
                    sim.x_of(crate::party::HeroId::Donut),
                    sim.scene.world.body(crate::party::HeroId::Donut).grounded,
                    sim.door_open(),
                    sim.tick(),
                    exit_x,
                )
            } else {
                RawInput {
                    move_x: 1.0,
                    ..RawInput::NONE
                }
            };
            sim.step([input, RawInput::NONE]);
            out.push(sim.hash());
        }
        out
    };
    for swap_first in [true, false] {
        let plain = run_hashes(false, swap_first);
        assert!(plain.len() >= 400, "{} ticks", plain.len());
        assert_eq!(
            run_hashes(true, swap_first),
            plain,
            "swap_first {swap_first}"
        );
    }
}

// ------------------------------------------------------------------ the presenter geometry

/// The door canvas sits on the lock: the solid x 1440..1455 is canvas columns 4..19, row 0
/// is the slab's underside and row 227 the walking line's last row.
#[test]
fn the_presenter_art_sits_on_the_lock_and_the_grant_column() {
    let run = f01_rest::launch(&root(), RestRoom::Refuge).unwrap();
    let door = run.door().unwrap();
    assert_eq!(DOOR_AT, (door.x - 4, door.y));
    assert_eq!(DOOR_AT.1 + 228, run.loaded.room.floor_y);
    assert_eq!(BOX_AT.1 + 24, run.loaded.room.floor_y);
    let (lo, hi) = f01_rest::REWARD_ZONE_X;
    assert_eq!(
        BOX_AT.0 + 16,
        (lo + hi) / 2,
        "the box's centre is the grant column's"
    );
    assert_eq!(SPARKLE_AT, (BOX_AT.0 + 8, BOX_AT.1 - 2));
    let rects = presenter_rects(&root()).unwrap();
    for r in rects.door {
        assert_eq!((r[2], r[3]), (24, 228));
    }
    for r in rects.boxes {
        assert_eq!((r[2], r[3]), (32, 24));
    }
    for r in rects.sparkle {
        assert_eq!((r[2], r[3]), (16, 16));
    }
    for r in rects.bell {
        assert_eq!((r[2], r[3]), (8, 6));
    }
    assert_eq!(
        BELL_AT.1 + 6,
        run.loaded.room.platforms[2].rect.y - 4,
        "the bell's bottom row is y 447, on the ledge piece's top row (y 448)"
    );
}

/// `FF_LAYOUT_DIR=<dir> cargo test --lib -- --ignored dump_ff_layouts`: the composed layout
/// of the refuge, plain and dressed, for `scripts/ff01_view.py` and `scripts/make_ff01_sheets.py`.
#[test]
#[ignore = "evidence helper: writes the composed layouts for the Python measurements"]
fn dump_ff_layouts() {
    let Ok(dir) = std::env::var("FF_LAYOUT_DIR") else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let run = f01_rest::launch_with(&root(), RestRoom::Refuge, false).unwrap();
    for dressed in [false, true] {
        let contract = load_contract(&root(), FASTFOOD_CONTRACT).unwrap();
        let d = dressing(&root()).unwrap();
        let mut out = None;
        crate::f01_boiler::compose_kit_hooked(
            &root(),
            &run.loaded.room,
            &[],
            Some(contract),
            |layout| {
                if dressed {
                    assert_eq!(dress_with(layout, &d, FASTFOOD_CONTRACT), 0);
                }
                layout["kit_contract"] = json!(FASTFOOD_CONTRACT);
                out = Some(layout.clone());
                Ok(())
            },
        )
        .unwrap();
        let name = format!(
            "rest_refuge.{}.layout.json",
            if dressed { "dressed" } else { "plain" }
        );
        std::fs::write(
            std::path::Path::new(&dir).join(name),
            serde_json::to_string(&out.unwrap()).unwrap(),
        )
        .unwrap();
    }
}
