//! CORRAL-C: the room file, its kit, the two room fixtures and the machine
//! item 2 reach sweep on the room's real collision rects.

use super::room::{self, A_LEASH, A_X, B_LEASH, B_X, FLOOR_WORLD_Y};
use crate::env_kit::EnvKit;
use crate::llama_rush::Activation;
use crate::party_scene::PartyFixtureSim;
use crate::prototype::{Block, Rect};
use std::path::PathBuf;
use std::sync::OnceLock;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn kit() -> &'static EnvKit {
    static KIT: OnceLock<EnvKit> = OnceLock::new();
    KIT.get_or_init(|| room::load_kit(&root()).unwrap())
}

/// The kit's solids in world px.
fn solids() -> Vec<Block> {
    kit().blocks()
}

/// The top of the highest intact solid under column `x` below `limit`.
fn top_at(solids: &[Block], x: f32, limit: f32) -> Option<f32> {
    solids
        .iter()
        .filter(|b| b.intact && b.rect.left <= x && b.rect.right() > x && b.rect.top() <= limit)
        .map(|b| b.rect.top())
        .max_by(f32::total_cmp)
}

#[test]
fn the_room_file_authors_the_sheets_steps_and_ceiling() {
    let (room, steps) = room::load_room(&root()).unwrap();
    assert_eq!(room.ceiling_px, Some(228), "CRL-Q31");
    assert!(
        room.platforms.is_empty(),
        "no platform: no gap leaks a bolt"
    );
    let authored: Vec<(i32, i32, i32)> = steps
        .iter()
        .map(|s| (s.x, s.right(), room.floor_y - s.y))
        .collect();
    assert_eq!(authored, vec![room::ENTRY_SHELF, room::BUMPER, room::DOCK]);
    // CRL-Q41: the bumper runs from the floor (a step), 5 tiles of 16 px.
    assert_eq!(steps[1].bottom(), room.floor_y);
    assert_eq!(room::BUMPER.2, 80);
}

/// The composed kit really has the authored heights on the 16 px tile layer:
/// the walking line at `FLOOR_WORLD_Y`, the shelf and the dock 32 px up, the
/// bumper 80 px up, and no other standable surface from the room's left wall
/// to the M3 threshold's right edge.
#[test]
fn the_kit_has_the_floor_the_shelf_the_dock_and_the_80_px_bumper() {
    let solids = solids();
    let ceiling = FLOOR_WORLD_Y + 200.0;
    for x in (24..2300).step_by(4) {
        let xf = x as f32 + 0.5;
        let expected = if (room::BUMPER.0..room::BUMPER.1).contains(&x) {
            80.0
        } else if (room::ENTRY_SHELF.0..room::ENTRY_SHELF.1).contains(&x)
            || (room::DOCK.0..room::DOCK.1).contains(&x)
        {
            32.0
        } else {
            0.0
        };
        assert_eq!(
            top_at(&solids, xf, ceiling),
            Some(FLOOR_WORLD_Y + expected),
            "column {x}"
        );
    }
    // The pinned walking line is the scene's.
    let scene = crate::corridor::SceneSim::new_at(kit(), 360, room::CARL_PARK_M2).unwrap();
    assert_eq!(scene.world.hero_foot_y(), FLOOR_WORLD_Y);
}

/// Machine item 2 (spec section 12) on the room's real collision rects: across
/// every surface pivot on a 1 px grid, both facings and every release height
/// of launches 440, 500 and 547, the llamas a bolt can reach are {B} on the
/// floor, {A} on the dock, and none on the entry shelf or the bumper top. A
/// change to the bumper, the dock, a leash or the bolt geometry fails here.
#[test]
fn corral_b1_reach_sweep_on_the_real_room_rects() {
    use crate::llama_rush::geometry_tests::{self as geo, Reach, sweep};
    // `sweep` uses its own leashes and dock height: they must be the room's.
    assert_eq!(
        (geo::A_LEASH.min, geo::A_LEASH.max),
        (A_LEASH.min, A_LEASH.max)
    );
    assert_eq!(
        (geo::B_LEASH.min, geo::B_LEASH.max),
        (B_LEASH.min, B_LEASH.max)
    );
    assert_eq!(geo::DOCK_FEET, room::DOCK_RISE);
    assert_eq!(geo::DOCK_FEET, room::DOCK.2 as f32);
    // Floor-relative solids (the sweep puts the walking line at 0).
    let solids: Vec<Block> = solids()
        .into_iter()
        .map(|b| Block {
            rect: Rect {
                bottom: b.rect.bottom - FLOOR_WORLD_Y,
                ..b.rect
            },
            ..b
        })
        .collect();
    assert_eq!(
        sweep(&solids, 0.0, 1068..=1844),
        Reach { a: false, b: true },
        "pen floor"
    );
    assert_eq!(
        sweep(&solids, 32.0, 1109..=1227),
        Reach::default(),
        "entry shelf"
    );
    assert_eq!(
        sweep(&solids, 32.0, 1900..=2036),
        Reach { a: true, b: false },
        "dock"
    );
    assert_eq!(
        sweep(&solids, 80.0, 1845..=1899),
        Reach::default(),
        "bumper top"
    );
}

#[test]
fn the_m2_fixture_spawns_b_on_the_floor_and_a_on_the_dock_dormant_with_the_profile() {
    let sim = PartyFixtureSim::new(kit(), 360, &room::CORRAL_M2).unwrap();
    let [b, a] = [&sim.combat.llamas[0], &sim.combat.llamas[1]];
    assert_eq!((b.x, b.feet()), (B_X as f32, FLOOR_WORLD_Y));
    assert_eq!(
        (a.x, a.feet()),
        (A_X as f32, FLOOR_WORLD_Y + room::DOCK_RISE)
    );
    for l in [a, b] {
        let c = l.corral.as_ref().expect("the corral profile");
        assert_eq!(c.activation, Activation::Dormant, "CRL-Q24");
        assert_eq!(l.facing, -1);
    }
    assert_eq!(b.corral.as_ref().unwrap().leash, B_LEASH);
    assert_eq!(a.corral.as_ref().unwrap().leash, A_LEASH);
    assert!(sim.combat.llama_party_target_on(), "KC1");
    assert!(crate::cf02::fixture_runs_cf02("corral_m2"), "KC2");
    assert!(crate::cf02::fixture_runs_cf02("corral_m1"));
    // Tests only: replaying them by name needs the corral scene arm (CORRAL-WIRE).
    assert!(crate::party_scene::PartyFixture::by_name("corral_m2").is_none());
    assert!(crate::party_scene::PartyFixture::by_name("corral_m1").is_none());
}

#[test]
fn the_m1_fixture_has_party_targeting_and_no_rush_profile() {
    let sim = PartyFixtureSim::new(kit(), 360, &room::CORRAL_M1).unwrap();
    let l = &sim.combat.llamas[0];
    assert!(l.corral.is_none(), "CRL-Q27: legacy llama");
    assert_eq!(l.x, room::M1_X as f32);
    assert!(sim.combat.llama_party_target_on());
}

/// Without its authored surface the dock llama would stand off Carl's walking
/// line: the spawn check defers it to the opt-in load check (R1), and that
/// check still refuses a surface that is not there.
#[test]
fn the_dock_llamas_surface_is_checked_by_the_opt_in_load_check() {
    let scene = crate::corridor::SceneSim::new_at(kit(), 360, room::CARL_PARK_M2).unwrap();
    let combat = crate::combat_scene::CombatState::new(room::m2_base(), kit(), &scene).unwrap();
    let mut opts = room::llama_opt_ins("corral_m2").unwrap().opt_ins;
    opts[1].feet = Some(FLOOR_WORLD_Y + 48.0);
    let err = combat
        .validate_llama_opt_ins_for_test(&scene.world.solids, &opts)
        .unwrap_err();
    assert!(err.contains("surface"), "{err}");
}

/// MAG-13 (retry): a retry mid-rush restores both corral llamas, their
/// surfaces, leashes, activation and cooldowns from the fixture.
#[test]
fn a_retry_in_m2_restores_both_llamas_from_the_fixture() {
    let fresh = PartyFixtureSim::new(kit(), 360, &room::CORRAL_M2).unwrap();
    let mut sim = PartyFixtureSim::new(kit(), 360, &room::CORRAL_M2).unwrap();
    // Carl walks into B's pen: it wakes, rushes, takes its cooldowns.
    let mut saw_rush = false;
    for t in 0..400 {
        // Right, hopping onto and off the 32 px entry shelf.
        sim.step_slots([
            crate::input::RawInput {
                move_x: 1.0,
                jump: t % 40 == 0,
                ..crate::input::RawInput::NONE
            },
            crate::input::RawInput::NONE,
        ]);
        saw_rush |= sim.combat.llamas[0].rush_state().is_some();
        if sim.scene.world.hero.x > 1300.0 {
            break;
        }
    }
    // Step until a rush is running at the restart tick, whatever the first
    // loop saw.
    let rushing =
        |s: &PartyFixtureSim| s.combat.llamas[0].rush_state().is_some() && s.combat.rush_seq() > 0;
    for _ in 0..600 {
        if rushing(&sim) {
            break;
        }
        sim.step_slots([crate::input::RawInput::NONE; 2]);
    }
    assert!(saw_rush || rushing(&sim), "B rushed Carl");
    assert!(rushing(&sim), "a rush is running at the restart");
    sim.combat.restart(&mut sim.scene);
    let mix = |s: &PartyFixtureSim| {
        let mut h = Vec::new();
        for l in &s.combat.llamas {
            l.mix_into(&mut |v| h.push(v));
        }
        h
    };
    assert_eq!(mix(&sim), mix(&fresh), "the llama block is the fresh one");
    assert_eq!(sim.combat.rush_seq(), 0);
}
