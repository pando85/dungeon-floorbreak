use super::*;
use crate::boss_scene;
use crate::env_kit::EnvKit;

fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `BOS_LAYOUT_DIR=<dir> cargo test --lib -- --ignored dump_bos_layouts`: the composed layout of the
/// arena, plain, for the evidence scripts and the art unittest.
#[test]
#[ignore = "evidence helper: writes the composed layout for the Python measurements"]
fn dump_bos_layouts() {
    let Ok(dir) = std::env::var("BOS_LAYOUT_DIR") else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let text = std::fs::read_to_string(root().join(boss_scene::ARENA_PATH)).unwrap();
    let rooms = crate::camera_lab::room::parse_rooms(&text).unwrap();
    let plain = composed(&root(), &rooms[0]).unwrap();
    std::fs::write(
        std::path::Path::new(&dir).join("bos.plain.layout.json"),
        serde_json::to_string(&plain).unwrap(),
    )
    .unwrap();
}

fn kits() -> (EnvKit, EnvKit) {
    (
        boss_scene::load_ball_arena(&root()).unwrap(),
        boss_scene::load_ball_arena_dressed(&root()).unwrap(),
    )
}

fn count(kit: &EnvKit, piece: &str) -> usize {
    kit.pieces.iter().filter(|p| p.piece == piece).count()
}

/// The dressing is presentation only: the solids, spawn, exit, views, walking line, blink barriers
/// and size are the plain arena's.
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
    assert_eq!(dressed.tiles.len(), plain.tiles.len());
    assert_eq!(dressed.actor_z, plain.actor_z);
    assert_eq!(dressed.layout_id, plain.layout_id);
    // The dressing did happen.
    assert_eq!(count(&dressed, "bos_post_cap"), 3);
    assert_eq!(count(&dressed, "bos_lintel"), 4);
    assert_eq!(count(&plain, "bos_post_cap"), 0);
    // The composer's four torches and the timber door are gone, two torches stand on the lintels.
    assert_eq!(count(&plain, "torch_wall"), 4);
    assert_eq!(count(&dressed, "torch_wall"), 2);
    assert_eq!(count(&dressed, "door_timber"), 0);
    assert_eq!(count(&dressed, "gym_fluoro_accent"), 2);
    // The exit's closed door stays (the open one is the presenter's overlay).
    assert_eq!(count(&dressed, PORTAL_CLOSED_PIECE), 1);
}

/// The light trade: four composer torches (eight lights) out, two torches (four) and two UV tubes
/// (four) in, so the room is as lit as before and the worst 640 px view stays inside the budget.
#[test]
fn the_lights_stay_inside_the_light_budget() {
    use crate::lighting::{MAX_LIGHTS, ViewRect};
    let (plain, dressed) = kits();
    let p = plain.lighting.as_ref().expect("the plain arena is lit");
    let d = dressed.lighting.as_ref().expect("the dressed arena is lit");
    assert_eq!(d.emitters.len() as i32, p.emitters.len() as i32 - 8 + 4 + 4);
    let view = *dressed.views.values().next().unwrap();
    let (g, dropped) = d.gather(
        &dressed,
        ViewRect {
            left: 0,
            top: view.y,
            w: 640,
            h: 360,
        },
        0,
    );
    assert_eq!(dropped, 0);
    assert!(
        g.lights.len() <= MAX_LIGHTS,
        "worst view {}",
        g.lights.len()
    );
    assert!(d.worst_case_lights <= MAX_LIGHTS);
}

/// Torch clearance: no placed `prop` or `structure` piece overlaps a remaining torch (16 x 56), and
/// no flame (the torch's top 16 rows) hangs over an alcove, the weight or within the ball's and a
/// jump's reach (flame bottom at least 150 px over the walking line).
#[test]
fn the_torches_are_clear_of_the_alcoves_the_lane_and_every_piece() {
    let (_, dressed) = kits();
    let walk = dressed.base_walk_line_y;
    let torches: Vec<_> = dressed
        .pieces
        .iter()
        .filter(|p| p.piece == "torch_wall")
        .collect();
    assert_eq!(torches.len(), 2);
    for t in &torches {
        assert!(
            walk - (t.y + 16) >= 150,
            "the flame is above the ball and a jump"
        );
        for (a, b) in [(28, 110), (530, 608)] {
            assert!(!(a..=b).contains(&(t.x + 8)), "no flame over an alcove");
        }
        for p in dressed.pieces.iter().filter(|p| {
            (p.layer == "prop" || p.layer == "structure")
                && p.piece != "torch_wall"
                && p.x < t.x + 16
                && t.x < p.x + p.rect.w
                && p.y < t.y + 56
                && t.y < p.y + p.rect.h
        }) {
            // The lintel's top edge touches the torch's foot, it never overlaps it; a pier behind
            // the torch is the wall it hangs on.
            assert!(
                p.layer == "structure"
                    && ["pier_l", "pier_r", "arch_ring_a"].contains(&p.piece.as_str()),
                "{} overlaps a torch at ({}, {})",
                p.piece,
                t.x,
                t.y
            );
        }
    }
}

/// The arch posts and lintels stand on the gameplay rects (walls 128 and 512, posts 16 px at wall
/// -32 and +16, lintel top 112 px over the floor) and the girder's underside is the sim ceiling.
#[test]
fn the_arches_stand_on_the_gameplay_rects_and_the_girder_is_at_the_ceiling() {
    let (_, dressed) = kits();
    let walk = dressed.base_walk_line_y;
    let data = crate::boss::ball();
    for (wall, left_post, right_post) in [
        (data.arena.left_wall_x as i32, true, true),
        (data.arena.right_wall_x as i32, true, false),
    ] {
        for (post, wanted) in [(wall - 32, left_post), (wall + 16, right_post)] {
            if !wanted {
                continue;
            }
            // A shaft covers the 16 px gameplay post.
            assert!(
                dressed.pieces.iter().any(|p| p.piece == "bos_post_shaft"
                    && p.x <= post
                    && p.x + p.rect.w >= post + 16
                    && p.y + p.rect.h == walk - 16),
                "a shaft covers the post at {post}"
            );
        }
        // The lintels span the 64 px arch over the posts' caps.
        let lintels: Vec<_> = dressed
            .pieces
            .iter()
            .filter(|p| p.piece == "bos_lintel")
            .collect();
        assert!(
            lintels
                .iter()
                .any(|p| p.x <= wall - 32 && p.y + p.rect.h == walk - 112)
        );
        assert!(
            lintels
                .iter()
                .any(|p| p.x + p.rect.w >= wall + 32 && p.y + p.rect.h == walk - 112)
        );
    }
    let girders: Vec<_> = dressed
        .pieces
        .iter()
        .filter(|p| p.piece == "gym_girder")
        .collect();
    assert!(
        girders.iter().all(|p| p.y + p.rect.h == walk - 224),
        "the girder's underside is the ceiling"
    );
    // It carries the weight's chain: x 169 + 36 on the rack's footprint.
    let boss = crate::boss::Boss::new(data, 0.0);
    let chain_x = (boss.footprint(0).left + crate::ball_art::SHACKLE_X) as i32;
    assert!(
        girders
            .iter()
            .any(|p| p.x <= chain_x && chain_x < p.x + p.rect.w)
    );
}

/// The presenter pieces the scene reads are in the contract with the sizes `ball_art` draws, and
/// the open door's overlay sits on the composer's closed one.
#[test]
fn the_presenter_pieces_match_the_geometry_the_art_module_draws() {
    use crate::ball_art::Piece;
    let rects = presenter_rects(&root()).unwrap();
    for piece in [
        Piece::WeightHung,
        Piece::WeightLanded,
        Piece::ChainV,
        Piece::ChainH,
    ] {
        let (atlas, rect) = rects
            .piece(piece.id())
            .unwrap_or_else(|| panic!("{}", piece.id()));
        assert_eq!(atlas, "floor01_bos_props");
        assert_eq!(
            (rect[2] as f32, rect[3] as f32),
            piece.size(),
            "{}",
            piece.id()
        );
    }
    assert_eq!(rects.portal_open.1[2..], [112, 136]);
    let (_, dressed) = kits();
    let closed = dressed
        .pieces
        .iter()
        .find(|p| p.piece == PORTAL_CLOSED_PIECE)
        .unwrap();
    assert_eq!((closed.x, closed.y), PORTAL_AT);
    assert_eq!(weight_piece(false), "bos_weight_hung");
    assert_eq!(weight_piece(true), "bos_weight_landed");
    assert!(stairwell_open(true) && !stairwell_open(false));
}

#[test]
fn the_dressed_arena_loads() {
    let kit = boss_scene::load_ball_arena_dressed(&root()).unwrap();
    assert!(kit.pieces.iter().any(|p| p.piece == "bos_post_cap"));
}
