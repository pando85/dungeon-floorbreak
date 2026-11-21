//! The section 10 geometry of the Infernal Corral as a unit test: the B1 reach
//! sweep (no spot where a Donut stays, jumping in place included, can hit both
//! llamas), the dock bumper and the leash extents.
//!
//! The solids are the sheet's authored values, not the room file: CORRAL-C
//! turns this sweep into the machine-item-2 test over the real
//! `f01_corral.room.json` (the 16 px collision tile layer). This test already
//! uses main's real physics: `step_hero_motion` for the jump arc and
//! `combat_scene::Bolt::rect` for the bolt box; the bolt spawn (27 px ahead,
//! 19 px up), its 6 px step and its 240 px range are the constants of
//! `combat_scene/donut.rs`.

use super::tests::profile;
use super::*;
use crate::combat::solid_at;
use crate::combat_scene::Bolt;
use crate::input::InputFrame;
use crate::prototype::{BodyDrive, HERO_HALF_HEIGHT, Hero, HeroMotion, Material, step_hero_motion};

const BOLT_FORWARD: f32 = 27.0;
const BOLT_UP: f32 = 19.0;
const BOLT_STEP: f32 = 6.0;
const BOLT_RANGE: f32 = 240.0;

const BUMPER_LEFT: f32 = 1856.0;
const BUMPER_WIDTH: f32 = 32.0;
const BUMPER_TOP: f32 = 80.0;
const DOCK_FEET: f32 = 32.0;
const A_LEASH: Leash = Leash {
    min: 1920.0,
    max: 2016.0,
};
const B_LEASH: Leash = Leash {
    min: 1248.0,
    max: 1830.0,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Partition {
    Absent,
    Intact,
    Broken,
}

fn block(left: f32, right: f32, bottom: f32, top: f32) -> Block {
    Block {
        rect: Rect {
            left,
            bottom,
            width: right - left,
            height: top - bottom,
        },
        material: Material::Stone,
        intact: true,
    }
}

/// The corral's solids: the pen floor, the entry shelf, the partition (absent,
/// intact 48 tall, or its 16 px remnant), the exit dock and the 80 px bumper.
fn corral(partition: Partition, bumper: bool) -> Vec<Block> {
    let mut solids = vec![
        block(-1000.0, 1856.0, -100.0, 0.0),
        block(1856.0, 2400.0, -100.0, DOCK_FEET),
        block(1120.0, 1216.0, 0.0, 32.0),
    ];
    match partition {
        Partition::Absent => {}
        Partition::Intact => solids.push(block(1392.0, 1408.0, 0.0, 48.0)),
        Partition::Broken => solids.push(block(1392.0, 1408.0, 0.0, 16.0)),
    }
    if bumper {
        solids.push(block(
            BUMPER_LEFT,
            BUMPER_LEFT + BUMPER_WIDTH,
            0.0,
            BUMPER_TOP,
        ));
    }
    solids
}

/// Every feet height at release that a jump in place gives on a surface of top
/// `top` with `launch`: standing, then each airborne tick to the landing.
fn release_heights(solids: &[Block], pivot: f32, top: f32, launch: f32) -> Vec<f32> {
    let mut hero = Hero::at(pivot, top + HERO_HALF_HEIGHT);
    let motion = HeroMotion {
        run_speed: 207.0,
        jump_speed: launch,
    };
    let mut heights = vec![top];
    let mut input = InputFrame {
        jump_pressed: true,
        ..InputFrame::default()
    };
    for _ in 0..120 {
        step_hero_motion(
            &mut hero,
            motion,
            &input,
            solids,
            1.0e9,
            &BodyDrive::default(),
        );
        input.jump_pressed = false;
        if hero.grounded {
            break;
        }
        heights.push(hero.y - HERO_HALF_HEIGHT);
    }
    assert!(hero.grounded, "the jump lands");
    heights
}

/// Every box a bolt released at `feet` from `pivot` tests, in order, until a
/// solid ends it or its range does (solids tested before targets).
fn bolt_path(solids: &[Block], pivot: f32, facing: i8, feet: f32) -> Vec<Rect> {
    let mut bolt = Bolt {
        instance: 1,
        x: pivot + f32::from(facing) * BOLT_FORWARD,
        y: feet + BOLT_UP,
        dir: facing,
        travelled: 0.0,
    };
    let mut path = Vec::new();
    loop {
        let rect = bolt.rect();
        if solid_at(solids, rect) {
            return path;
        }
        path.push(rect);
        bolt.x += f32::from(facing) * BOLT_STEP;
        bolt.travelled += BOLT_STEP;
        if bolt.travelled >= BOLT_RANGE {
            return path;
        }
    }
}

/// The union of every body a llama can take inside its leash, both facings: a
/// superset, so "no overlap" proves no hit from any body position.
fn reach_box(leash: Leash, feet: f32) -> Rect {
    let left = llama_body(leash.min, feet, 1)
        .left
        .min(llama_body(leash.min, feet, -1).left);
    let right = llama_body(leash.max, feet, 1)
        .right()
        .max(llama_body(leash.max, feet, -1).right());
    Rect {
        left,
        bottom: feet,
        width: right - left,
        height: LLAMA_BODY_HEIGHT,
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
struct Reach {
    a: bool,
    b: bool,
}

/// What a Donut staying on `surface` (top, pivot range) can hit with a bolt
/// released at any height of a jump in place, with any launch, both facings.
fn sweep(solids: &[Block], top: f32, pivots: core::ops::RangeInclusive<i32>) -> Reach {
    let a = reach_box(A_LEASH, DOCK_FEET);
    let b = reach_box(B_LEASH, 0.0);
    let mut reach = Reach::default();
    for launch in [440.0_f32, 500.0, 547.0] {
        for pivot in pivots.clone() {
            let pivot = pivot as f32;
            let heights = release_heights(solids, pivot, top, launch);
            for facing in [1_i8, -1] {
                for &feet in &heights {
                    for rect in bolt_path(solids, pivot, facing, feet) {
                        reach.a |= rect.overlaps(a);
                        reach.b |= rect.overlaps(b);
                    }
                }
            }
        }
    }
    reach
}

#[test]
fn the_b1_sweep_the_floor_reaches_only_b_the_dock_only_a_and_every_other_surface_neither() {
    for partition in [Partition::Absent, Partition::Intact, Partition::Broken] {
        let solids = corral(partition, true);
        let label = |s: &str| format!("{s} ({partition:?})");
        // The pen floor (pivots 1068..1844): only B, whatever the jump timing.
        assert_eq!(
            sweep(&solids, 0.0, 1068..=1844),
            Reach { a: false, b: true },
            "{}",
            label("floor")
        );
        // The entry shelf (1109..1227, top 32): neither.
        assert_eq!(
            sweep(&solids, 32.0, 1109..=1227),
            Reach::default(),
            "{}",
            label("entry shelf")
        );
        // The exit dock (1900..2036, top 32): only A.
        assert_eq!(
            sweep(&solids, 32.0, 1900..=2036),
            Reach { a: true, b: false },
            "{}",
            label("dock")
        );
        // The bumper top (1845..1899, top 80): neither.
        assert_eq!(
            sweep(&solids, BUMPER_TOP, 1845..=1899),
            Reach::default(),
            "{}",
            label("bumper top")
        );
        match partition {
            Partition::Intact => assert_eq!(
                sweep(&solids, 48.0, 1381..=1419),
                Reach::default(),
                "partition top"
            ),
            // The 16 px remnant top is a surface a Donut can stand on: it is
            // below B's top, so she can hit B but never A.
            Partition::Broken => assert_eq!(
                sweep(&solids, 16.0, 1381..=1419),
                Reach { a: false, b: true },
                "remnant top"
            ),
            Partition::Absent => {}
        }
    }
}

#[test]
fn without_the_bumper_a_floor_donut_can_hit_the_dock_llama_the_revision_1_leak() {
    // The negative control: the sweep detects the leak the bumper closes. The
    // review's counterexample stands at 1728 and jump-casts toward the dock.
    let solids = corral(Partition::Absent, false);
    let reach = sweep(&solids, 0.0, 1068..=1844);
    assert!(
        reach.a && reach.b,
        "no bumper: the floor reaches both: {reach:?}"
    );
    // With the bumper, the single start of the review (M2-13) is clean.
    let solids = corral(Partition::Absent, true);
    assert_eq!(
        sweep(&solids, 0.0, 1728..=1728),
        Reach { a: false, b: true }
    );
}

#[test]
fn a_72_px_bumper_would_still_stop_the_floor_but_80_is_what_the_tile_layer_builds() {
    // The bumper must be at least the dock llama's top (71). 72 and 80 both
    // close the floor-to-dock path; 72 is not buildable in the 16 px layer.
    let top = DOCK_FEET + LLAMA_BODY_HEIGHT;
    assert_eq!(top, 71.0);
    assert!(BUMPER_TOP >= top);
    assert_eq!(BUMPER_TOP % 16.0, 0.0, "a multiple of the 16 px tile");
    assert_eq!((BUMPER_TOP / 16.0) as u32, 5, "five collision tiles");
    // A bumper lower than the llama's top lets a high enough bolt over it into A.
    let mut low = corral(Partition::Absent, true);
    let bumper = low.len() - 1;
    low[bumper].rect.height = 48.0;
    let reach = sweep(&low, 0.0, 1068..=1844);
    assert!(reach.a, "a 48 px bumper leaks: {reach:?}");
}

#[test]
fn both_llama_bodies_stay_clear_of_the_bumper_and_the_gate_inside_their_leashes() {
    // A: body inside 1894..2042, clear of the bumper (1888) and the M3 gate
    // (2048) in both facings, at every pivot of the leash.
    for x in (1920..=2016).step_by(1) {
        for facing in [1_i8, -1] {
            let body = llama_body(x as f32, DOCK_FEET, facing);
            assert!(body.left >= 1894.0, "A body left {} at {x}", body.left);
            assert!(
                body.right() <= 2042.0,
                "A body right {} at {x}",
                body.right()
            );
            assert!(body.left > BUMPER_LEFT + BUMPER_WIDTH);
            assert!(body.right() < 2048.0);
        }
    }
    // B: at the leash maximum facing -1 its right edge is flush with the bumper
    // face (FN4), never beyond it.
    for x in (1248..=1830).step_by(1) {
        for facing in [1_i8, -1] {
            let body = llama_body(x as f32, 0.0, facing);
            assert!(
                body.right() <= BUMPER_LEFT,
                "B body right {} at {x}",
                body.right()
            );
        }
    }
    assert_eq!(llama_body(1830.0, 0.0, -1).right(), BUMPER_LEFT);
    // A's rush box at the leash minimum ends beyond the bumper's right face.
    let p = profile();
    let boxed = rush_box(&p, A_LEASH.min, -1, DOCK_FEET);
    assert!(boxed.left >= BUMPER_LEFT + BUMPER_WIDTH);
    // The box heights: A's 34..62 and B's 2..30.
    assert_eq!((boxed.bottom, boxed.top()), (34.0, 62.0));
    let b_box = rush_box(&p, 1488.0, -1, 0.0);
    assert_eq!((b_box.bottom, b_box.top()), (2.0, 30.0));
}

#[test]
fn the_rush_boxes_of_the_height_truth_table() {
    let p = profile();
    let hurt = |left: f32, bottom: f32, w: f32, h: f32| Rect {
        left,
        bottom,
        width: w,
        height: h,
    };
    // B's box (2..30) hits a floor Donut and Carl, never a hero on a 32 px
    // shelf, the 48 px partition top or the 80 px bumper top.
    let b_box = rush_box(&p, 1488.0, -1, 0.0);
    let col = b_box.left + 4.0;
    for (bottom, hit) in [
        (0.0, true),
        (29.9, true),
        (30.0, false),
        (32.0, false),
        (48.0, false),
        (80.0, false),
    ] {
        assert_eq!(
            b_box.overlaps(hurt(col, bottom, 20.0, 24.0)),
            hit,
            "Donut at {bottom}"
        );
        assert_eq!(
            b_box.overlaps(hurt(col, bottom, 24.0, 52.0)),
            hit,
            "Carl at {bottom}"
        );
    }
    // A's box (34..62) hits a hero standing on the dock (32), not one on the
    // bumper top (80) or on the floor under it.
    let a_box = rush_box(&p, 1968.0, -1, DOCK_FEET);
    let col = a_box.left + 4.0;
    assert!(a_box.overlaps(hurt(col, 32.0, 20.0, 24.0)));
    assert!(a_box.overlaps(hurt(col, 32.0, 24.0, 52.0)));
    assert!(!a_box.overlaps(hurt(col, 80.0, 24.0, 52.0)));
    assert!(!a_box.overlaps(hurt(col, 0.0, 20.0, 24.0)));
    // A's glob band (74..80) hits Carl on the dock (32..84), not Donut on it
    // (32..56), not Carl on the bumper top (80..132: strict overlap).
    assert!(glob_band_overlaps(DOCK_FEET, hurt(0.0, 32.0, 24.0, 52.0)));
    assert!(!glob_band_overlaps(DOCK_FEET, hurt(0.0, 32.0, 20.0, 24.0)));
    assert!(!glob_band_overlaps(DOCK_FEET, hurt(0.0, 80.0, 24.0, 52.0)));
    // B's glob band (42..48) hits Carl on the floor and Donut on a 32 px shelf,
    // not Donut on the floor (0..24) or on the partition top (48..72).
    assert!(glob_band_overlaps(0.0, hurt(0.0, 0.0, 24.0, 52.0)));
    assert!(glob_band_overlaps(0.0, hurt(0.0, 32.0, 20.0, 24.0)));
    assert!(!glob_band_overlaps(0.0, hurt(0.0, 0.0, 20.0, 24.0)));
    assert!(!glob_band_overlaps(0.0, hurt(0.0, 48.0, 20.0, 24.0)));
}

#[test]
fn a_boosted_donut_caught_in_a_dock_hop_never_lands_on_the_bumper() {
    // A's rush knocks a dock hero toward the bumper; the contact needs her feet
    // below 30 above the dock. Even a boosted Donut (547) caught late in a hop
    // and knocked up by 160 px/s peaks under the 80 px top.
    // Feet at the contact tick below 30 above the dock, then knock_vy 160.
    let mut worst = 0.0_f32;
    for launch in [440.0_f32, 500.0, 547.0] {
        let heights = release_heights(&corral(Partition::Absent, true), 1960.0, DOCK_FEET, launch);
        for &feet in heights.iter().filter(|f| **f < DOCK_FEET + 30.0) {
            // The knock replaces vy with 160 px/s: the further rise is v^2 / 2g.
            let rise = 160.0_f32 * 160.0 / (2.0 * 1050.0);
            worst = worst.max(feet + rise);
        }
    }
    assert!(worst < BUMPER_TOP, "worst peak {worst}");
}
