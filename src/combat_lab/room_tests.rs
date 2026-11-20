//! COMBAT-LAB-01 LAB-01: the headless asserts of the R00 to R02 layout spec
//! (`docs/01-design/combat-lab-01-rooms-r00-r02.md`, section 9.1), run on the
//! real parser, composer, hero integration and combat state. Every number in
//! the tables below is the spec's; a failing row is a design defect to report,
//! not a value to tune here.

use std::path::Path;

use crate::camera_lab::layout::build_kit;
use crate::camera_lab::room::{IRect, Room, TILE, parse_rooms};
use crate::combat::EnemyAi;
use crate::env_kit::{EnvKit, to_world_y};
use crate::feel::{LLAMA_GLOB_MOUTH_DX, LLAMA_GLOB_MOUTH_DY};
use crate::input::{InputFrame, RawInput};
use crate::llama::{Glob, GlobFate};
use crate::party::{HeroId, PROVISIONAL_TUNING};
use crate::prototype::{
    Block, BodyDrive, HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Hero, Rect, step_hero_driven,
};

use super::*;

const FLOOR: i32 = 580;
/// The HUD clearance of the spec (rule 3): a head at most this far above the
/// walking line.
const HEAD_LIMIT: f32 = 228.0;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn parse_file(name: &str) -> Vec<Room> {
    let text = std::fs::read_to_string(root().join(LAB_DIR).join(name)).unwrap();
    parse_rooms(&text).unwrap()
}

/// One entry per scenario of the four room files: the file, the scenario id,
/// the width and the expected generated steps (assert 2).
struct Expect {
    file: &'static str,
    id: &'static str,
    width: i32,
    steps: &'static [(i32, i32, i32, i32)],
    /// `(x, top surface y, w)` of every slab (spec 3.2, 4.2, 4.6, 5.2).
    slabs: &'static [(i32, i32, i32)],
}

const EXPECT: [Expect; 6] = [
    Expect {
        file: "r00_setup_cage.room.json",
        id: "LAB-R00",
        width: 640,
        steps: &[(384, 548, 32, 32)],
        slabs: &[(288, 548, 64), (416, 516, 64), (480, 548, 32)],
    },
    Expect {
        file: "r01_impact_pit.room.json",
        id: "LAB-R01-V1",
        width: 1024,
        steps: &[],
        slabs: &[(16, 548, 96)],
    },
    Expect {
        file: "r01_impact_pit.room.json",
        id: "LAB-R01-V2",
        width: 1024,
        steps: &[],
        slabs: &[(16, 548, 96)],
    },
    Expect {
        file: "r01_impact_pit.room.json",
        id: "LAB-R01-V3",
        width: 1024,
        steps: &[],
        slabs: &[(16, 548, 96)],
    },
    Expect {
        file: "r01h_impact_pit_holdout.room.json",
        id: "LAB-R01H",
        width: 1024,
        steps: &[],
        slabs: &[(912, 548, 96)],
    },
    Expect {
        file: "r02_broken_scaffold.room.json",
        id: "LAB-R02",
        width: 1280,
        steps: &[(704, 548, 32, 32)],
        slabs: &[
            (288, 548, 128),
            (736, 516, 96),
            (832, 548, 32),
            (1088, 548, 64),
        ],
    },
];

fn scenario(e: &Expect) -> Room {
    parse_file(e.file)
        .into_iter()
        .find(|r| r.id == e.id)
        .unwrap_or_else(|| panic!("{} has no scenario {}", e.file, e.id))
}

fn kit_of(room: &Room) -> EnvKit {
    build_kit(root(), room).unwrap().0
}

// ------------------------------------------------------------ asserts 1 to 5

/// Assert 1: every file parses through `parse_rooms`, no snap, render size.
#[test]
fn a1_every_file_parses_with_no_snap_and_the_render_size() {
    for e in &EXPECT {
        let room = scenario(e);
        assert_eq!(room.snap_dy, 0, "{}", e.id);
        assert_eq!(room.render, (e.width, 672), "{}", e.id);
        assert_eq!(room.floor_y, FLOOR, "{}", e.id);
    }
    assert_eq!(parse_file("r01_impact_pit.room.json").len(), 3);
    for name in [
        "r00_setup_cage",
        "r02_broken_scaffold",
        "r01h_impact_pit_holdout",
    ] {
        assert_eq!(parse_file(&format!("{name}.room.json")).len(), 1, "{name}");
    }
}

/// Assert 2 and 3: the generated steps and the derived platform rectangles
/// equal the spec, and the three R01 scenarios have identical geometry.
#[test]
fn a2_a3_the_steps_and_slabs_equal_the_spec_and_r01_geometry_is_shared() {
    for e in &EXPECT {
        let room = scenario(e);
        let steps: Vec<_> = room.steps.iter().map(|s| (s.x, s.y, s.w, s.h)).collect();
        assert_eq!(steps, e.steps, "{}: generated steps", e.id);
        let mut slabs: Vec<_> = room
            .platforms
            .iter()
            .map(|p| (p.rect.x, p.rect.y, p.rect.w))
            .collect();
        slabs.sort_unstable();
        let mut want = e.slabs.to_vec();
        want.sort_unstable();
        assert_eq!(slabs, want, "{}: slabs", e.id);
    }
    let geometry = |e: &Expect| {
        let r = scenario(e);
        (
            r.render,
            r.floor_y,
            r.play_x,
            r.spawn_carl,
            r.spawn_donut,
            r.platforms.clone(),
            r.steps.clone(),
            r.goal_x,
            r.exit,
        )
    };
    let v1 = geometry(&EXPECT[1]);
    assert_eq!(geometry(&EXPECT[2]), v1);
    assert_eq!(geometry(&EXPECT[3]), v1);
    // Derived keys (spec 3.2, 4.2, 4.6, 5.2).
    let r00 = scenario(&EXPECT[0]);
    assert_eq!(r00.play_x, (16, 624));
    assert_eq!(r00.exit, IRect::new(536, 484, 64, 96));
    let r01 = scenario(&EXPECT[1]);
    assert_eq!(r01.play_x, (16, 1008));
    assert_eq!(r01.exit, IRect::new(920, 484, 64, 96));
    let r01h = scenario(&EXPECT[4]);
    assert_eq!(r01h.exit, IRect::new(40, 484, 64, 96));
    let r02 = scenario(&EXPECT[5]);
    assert_eq!(r02.play_x, (16, 1264));
    assert_eq!(r02.exit, IRect::new(1176, 484, 64, 96));
}

/// Assert 4: `build_kit` succeeds and no camera position shows undrawn cells
/// (the whole render bounds is drawn).
#[test]
fn a4_build_kit_succeeds_and_no_view_shows_void() {
    for e in &EXPECT {
        let room = scenario(e);
        let (kit, _contract, plan) = build_kit(root(), &room).unwrap();
        assert_eq!((kit.width, kit.height), room.render, "{}", e.id);
        let cols = (kit.width / TILE) as usize;
        let rows = (kit.height / TILE) as usize;
        let mut drawn = vec![vec![false; cols]; rows];
        for t in &kit.tiles {
            drawn[t.row as usize][t.col as usize] = true;
        }
        let in_opening = |c: usize, r: usize| {
            let (x, y) = (c as i32 * TILE + 8, r as i32 * TILE + 8);
            plan.openings
                .iter()
                .any(|&(ox, oy, ow, oh)| x >= ox && x < ox + ow && y >= oy && y < oy + oh)
        };
        let holes = drawn
            .iter()
            .enumerate()
            .flat_map(|(r, row)| row.iter().enumerate().map(move |(c, ok)| (c, r, *ok)))
            .filter(|&(c, r, ok)| !ok && !in_opening(c, r))
            .count();
        assert_eq!(holes, 0, "{}: undrawn cells", e.id);
    }
}

/// Assert 5: the walking line is the floor (580) at every spawn, enemy column
/// and exit column.
#[test]
fn a5_the_spawns_enemies_and_exits_stand_on_the_floor_line() {
    for e in &EXPECT {
        let room = scenario(e);
        let kit = kit_of(&room);
        assert_eq!(kit.base_walk_line_y, FLOOR, "{}", e.id);
        let mut columns = vec![room.spawn_carl, room.spawn_donut];
        columns.extend(room.enemies.iter().map(|en| en.x));
        columns.extend([room.exit.x + 8, room.exit.x + 56]);
        for x in columns {
            assert_eq!(kit.walking_line_at(x), Some(FLOOR), "{}: column {x}", e.id);
        }
        // The portal art floor stays clear: the whole exit footprint.
        for x in (room.exit.x..room.exit.right()).step_by(8) {
            assert_eq!(kit.walking_line_at(x), Some(FLOOR), "{}: exit {x}", e.id);
        }
    }
}

// ------------------------------------------------------------------ the hero

/// A solo hero on the real `step_hero_driven` against the kit's solids. Carl
/// and Donut share this function and the 24 x 52 collider (spec 1.2); the
/// scene-level check in `a6_donut_steps_like_carl_in_the_scene` runs Donut's
/// body in a live two-player scene.
struct Course {
    solids: Vec<Block>,
    max_x: f32,
    floor_world: f32,
}

/// Where a scripted action ended.
#[derive(Debug, Clone, Copy)]
struct Landing {
    x: f32,
    /// Surface height above the walking line (0, 32 or 64).
    height: i32,
    /// Highest head above the walking line over the whole action.
    head: f32,
    ticks: u32,
}

impl Course {
    fn new(kit: &EnvKit) -> Self {
        Self {
            solids: kit.blocks(),
            max_x: kit.width as f32 - HERO_HALF_WIDTH,
            floor_world: to_world_y(kit.height, FLOOR),
        }
    }

    fn stand(&self, x: f32, height: i32) -> Hero {
        Hero::at(x, self.floor_world + height as f32 + HERO_HALF_HEIGHT)
    }

    /// Whether a hero standing there would start inside a solid (beside a
    /// higher block): such a start is not a position anyone can be in.
    fn embedded(&self, x: f32, height: i32) -> bool {
        let hero = self.stand(x, height);
        let body = Rect {
            left: hero.x - HERO_HALF_WIDTH,
            bottom: hero.y - HERO_HALF_HEIGHT + 0.5,
            width: HERO_HALF_WIDTH * 2.0,
            height: HERO_HALF_HEIGHT * 2.0 - 1.0,
        };
        self.solids
            .iter()
            .any(|b| b.intact && body.overlaps(b.rect))
    }

    fn head(&self, hero: &Hero) -> f32 {
        hero.y + HERO_HALF_HEIGHT - self.floor_world
    }

    fn step(&self, hero: &mut Hero, frame: &mut InputFrame, raw: RawInput) {
        frame.advance(raw);
        step_hero_driven(hero, frame, &self.solids, self.max_x, &BodyDrive::default());
    }

    fn land(&self, hero: &Hero, head: f32, ticks: u32) -> Landing {
        Landing {
            x: hero.x,
            height: (hero.y - HERO_HALF_HEIGHT - self.floor_world).round() as i32,
            head,
            ticks,
        }
    }

    /// Stand at `from.0` on a surface `from.1` high and jump: the jump is
    /// pressed on the first tick, the direction is held for `hold` ticks
    /// (`None` is a full hold until landing).
    fn jump(&self, from: (f32, i32), dir: f32, hold: Option<u32>) -> Landing {
        let mut hero = self.stand(from.0, from.1);
        let mut frame = InputFrame::default();
        let mut head = self.head(&hero);
        for tick in 0..300_u32 {
            let holding = hold.is_none_or(|h| tick < h);
            let raw = RawInput {
                move_x: if holding { dir } else { 0.0 },
                jump: tick == 0,
                ..RawInput::NONE
            };
            self.step(&mut hero, &mut frame, raw);
            head = head.max(self.head(&hero));
            if tick > 2 && hero.grounded {
                return self.land(&hero, head, tick);
            }
        }
        panic!("the jump from {from:?} never landed");
    }

    /// Walk off the surface the hero stands on: hold `dir` until `to_x`, then
    /// let go and fall.
    #[allow(dead_code)]
    fn walk_off(&self, from: (f32, i32), dir: f32, to_x: f32) -> Landing {
        let mut hero = self.stand(from.0, from.1);
        let mut frame = InputFrame::default();
        let mut head = self.head(&hero);
        for tick in 0..400_u32 {
            let arrived = (hero.x - to_x) * dir >= 0.0;
            let raw = RawInput {
                move_x: if arrived { 0.0 } else { dir },
                ..RawInput::NONE
            };
            self.step(&mut hero, &mut frame, raw);
            head = head.max(self.head(&hero));
            if arrived && hero.grounded {
                return self.land(&hero, head, tick);
            }
        }
        panic!("the walk from {from:?} never settled");
    }
}

// ------------------------------------------------------------ spec windows

/// One row of the reachability tables 3.4, 4.4, 4.6 and 5.4: from a surface
/// of `from_height`, running `dir`, take off anywhere in `takeoff` with
/// `hold` and land on a surface of one of the `land` heights whose centre is
/// within `x` (the slab span widened by the body half width).
struct Row {
    name: &'static str,
    from_height: i32,
    dir: f32,
    takeoff: (i32, i32),
    /// Hold lengths to try (`FULL` holds the direction until landing).
    holds: &'static [Option<u32>],
    land: &'static [i32],
    x: (i32, i32),
}

const E: f32 = 1.0;
const W: f32 = -1.0;
const FULL: Option<u32> = None;

const R00_ROWS: &[Row] = &[
    Row {
        name: "floor -> P0 (full)",
        from_height: 0,
        dir: E,
        takeoff: (136, 222),
        holds: &[FULL],
        land: &[32],
        x: (276, 364),
    },
    Row {
        name: "floor -> P0 (hold 10)",
        from_height: 0,
        dir: E,
        takeoff: (244, 276),
        holds: &[Some(10)],
        land: &[32],
        x: (276, 364),
    },
    Row {
        name: "P0 -> S0 (hold 10)",
        from_height: 32,
        dir: E,
        takeoff: (340, 362),
        holds: &[Some(10)],
        land: &[32],
        x: (372, 428),
    },
    Row {
        name: "S0 -> P1",
        from_height: 32,
        dir: E,
        takeoff: (386, 404),
        holds: &[Some(6), Some(10), Some(24)],
        land: &[64],
        x: (404, 492),
    },
    Row {
        name: "P1 -> P2 (hold 10)",
        from_height: 64,
        dir: E,
        takeoff: (470, 490),
        holds: &[Some(10)],
        land: &[32],
        x: (468, 524),
    },
    Row {
        name: "floor -> P2 (west, hold 10)",
        from_height: 0,
        dir: W,
        takeoff: (524, 556),
        holds: &[Some(10)],
        land: &[32],
        x: (468, 524),
    },
    Row {
        name: "P2 -> P1 (west)",
        from_height: 32,
        dir: W,
        takeoff: (492, 510),
        holds: &[Some(6), Some(10), Some(24)],
        land: &[64],
        x: (404, 492),
    },
    Row {
        name: "floor -> P1 direct (west, full)",
        from_height: 0,
        dir: W,
        takeoff: (524, 600),
        holds: &[FULL],
        land: &[64],
        x: (404, 492),
    },
    Row {
        name: "P1 -> S0 (west, hold 10)",
        from_height: 64,
        dir: W,
        takeoff: (406, 434),
        holds: &[Some(10)],
        land: &[32],
        x: (372, 428),
    },
];

const R01_ROWS: &[Row] = &[Row {
    name: "floor -> nook (west, full)",
    from_height: 0,
    dir: W,
    takeoff: (124, 176),
    holds: &[FULL],
    land: &[32],
    x: (4, 124),
}];

const R01H_ROWS: &[Row] = &[Row {
    name: "floor -> nook (east, full)",
    from_height: 0,
    dir: E,
    takeoff: (848, 900),
    holds: &[FULL],
    land: &[32],
    x: (900, 1020),
}];

const R02_ROWS: &[Row] = &[
    Row {
        name: "entry -> P0 (full)",
        from_height: 0,
        dir: E,
        takeoff: (138, 276),
        holds: &[FULL],
        land: &[32],
        x: (276, 428),
    },
    Row {
        name: "pocket -> S0 (full)",
        from_height: 0,
        dir: E,
        takeoff: (552, 606),
        holds: &[FULL],
        land: &[32, 64],
        x: (692, 748),
    },
    Row {
        name: "pocket -> S0 (hold 10)",
        from_height: 0,
        dir: E,
        takeoff: (660, 692),
        holds: &[Some(10)],
        land: &[32],
        x: (692, 748),
    },
    Row {
        name: "S0 -> P1",
        from_height: 32,
        dir: E,
        takeoff: (706, 724),
        holds: &[Some(6), Some(10), Some(24)],
        land: &[64],
        x: (724, 844),
    },
    Row {
        name: "pocket -> P1 direct (full)",
        from_height: 0,
        dir: E,
        takeoff: (608, 692),
        holds: &[FULL],
        land: &[32, 64],
        x: (692, 844),
    },
    Row {
        name: "yard -> P3 (full)",
        from_height: 0,
        dir: E,
        takeoff: (938, 1022),
        holds: &[FULL],
        land: &[32],
        x: (1076, 1164),
    },
    Row {
        name: "bay -> P3 (west, hold 10)",
        from_height: 0,
        dir: W,
        takeoff: (1164, 1196),
        holds: &[Some(10)],
        land: &[32],
        x: (1076, 1164),
    },
    Row {
        name: "yard -> P2 (west, hold 10)",
        from_height: 0,
        dir: W,
        takeoff: (876, 908),
        holds: &[Some(10)],
        land: &[32],
        x: (820, 876),
    },
    Row {
        name: "P2 -> P1 (west)",
        from_height: 32,
        dir: W,
        takeoff: (844, 862),
        holds: &[Some(6), Some(10), Some(24)],
        land: &[64],
        x: (724, 844),
    },
    Row {
        name: "yard -> P1 direct (west, full)",
        from_height: 0,
        dir: W,
        takeoff: (876, 960),
        holds: &[FULL],
        land: &[64],
        x: (724, 844),
    },
    Row {
        name: "pocket -> P0 (west, full)",
        from_height: 0,
        dir: W,
        takeoff: (428, 568),
        holds: &[FULL],
        land: &[32],
        x: (276, 428),
    },
];

/// The measured outcome of one spec window: the takeoffs (every 2 px) that
/// land where the row says, as the first and last passing x, and the ones that
/// fail inside the spec window.
#[derive(Debug)]
struct Measured {
    room: &'static str,
    row: &'static str,
    spec: (i32, i32),
    /// Smallest and largest takeoff that passes every hold of the row.
    passing: Option<(i32, i32)>,
    failing: Vec<i32>,
    max_head: f32,
}

fn measure_rows(e: &Expect, rows: &'static [Row]) -> Vec<Measured> {
    let course = Course::new(&kit_of(&scenario(e)));
    rows.iter()
        .map(|row| {
            let mut passing: Option<(i32, i32)> = None;
            let mut failing = Vec::new();
            let mut max_head = 0.0_f32;
            for x in (row.takeoff.0..=row.takeoff.1).step_by(2) {
                let mut ok = true;
                for hold in row.holds {
                    let l = course.jump((x as f32, row.from_height), row.dir, *hold);
                    max_head = max_head.max(l.head);
                    ok &= row.land.contains(&l.height)
                        && (row.x.0 as f32..=row.x.1 as f32).contains(&l.x);
                }
                if ok {
                    passing = Some(passing.map_or((x, x), |(a, _)| (a, x)));
                } else {
                    failing.push(x);
                }
            }
            Measured {
                room: e.id,
                row: row.name,
                spec: row.takeoff,
                passing,
                failing,
                max_head,
            }
        })
        .collect()
}

/// How far a spec window edge may sit from the measured one before the row is
/// a defect and not rounding: the stand-in of the spec stepped x in whole
/// pixels and the run is 3.08 px per tick.
const EDGE_SLACK_PX: i32 = 8;

/// Assert 6 (windows): every takeoff window of the spec tables lands where the
/// table says on the real `step_hero_driven`, head under 228 px. The failing
/// takeoffs of a window may only sit within [`EDGE_SLACK_PX`] of its two ends
/// (the spec windows were derived from a stand-in); the full measured table is
/// printed (`--nocapture`) and recorded in the PR.
#[test]
fn a6_every_spec_takeoff_window_lands_where_the_table_says() {
    let mut all = Vec::new();
    all.extend(measure_rows(&EXPECT[0], R00_ROWS));
    all.extend(measure_rows(&EXPECT[1], R01_ROWS));
    all.extend(measure_rows(&EXPECT[4], R01H_ROWS));
    all.extend(measure_rows(&EXPECT[5], R02_ROWS));
    for m in &all {
        eprintln!(
            "WINDOW {} | {} | spec {:?} | measured {:?} | failing {:?} | head {:.1}",
            m.room, m.row, m.spec, m.passing, m.failing, m.max_head
        );
        assert!(
            m.max_head <= HEAD_LIMIT,
            "{} {}: head {}",
            m.room,
            m.row,
            m.max_head
        );
        let (lo, hi) = m.spec;
        let (pl, ph) = m
            .passing
            .unwrap_or_else(|| panic!("{} {}: no takeoff lands", m.room, m.row));
        assert!(
            pl - lo <= EDGE_SLACK_PX && hi - ph <= EDGE_SLACK_PX,
            "{} {}: the spec window {:?} is more than {EDGE_SLACK_PX} px wider than the measured {:?}",
            m.room,
            m.row,
            m.spec,
            m.passing
        );
        // Everything between the first and last passing takeoff passes.
        assert!(
            m.failing.iter().all(|x| *x < pl || *x > ph),
            "{} {}: a hole inside the passing window: {:?}",
            m.room,
            m.row,
            m.failing
        );
    }
}

// ------------------------------------------------------- the surface graph

/// A standing surface: a contiguous run of walking-line columns of one height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Surface {
    height: i32,
    x0: i32,
    x1: i32,
}

/// The standing surfaces of a room. `walking_line_at` only knows lines up to
/// 32 px above the floor, so a rise-64 slab is added from the room data; the
/// sealed hollow under it is no surface (the hollow rule of spec section 2).
fn surfaces_of(kit: &EnvKit, room: &Room) -> Vec<Surface> {
    let mut out: Vec<Surface> = Vec::new();
    for x in TILE..kit.width - TILE {
        let high = room
            .platforms
            .iter()
            .find(|p| FLOOR - p.rect.y > 32 && (p.rect.x..p.rect.right()).contains(&x));
        let height = match high {
            Some(p) => FLOOR - p.rect.y,
            None => match kit.walking_line_at(x) {
                Some(line) => FLOOR - line,
                None => continue,
            },
        };
        match out.last_mut() {
            Some(s) if s.height == height && s.x1 == x - 1 => s.x1 = x,
            _ => out.push(Surface {
                height,
                x0: x,
                x1: x,
            }),
        }
    }
    out
}

/// The surface a landing belongs to: the one of its height nearest to the
/// landing column, at most one body half width away.
fn surface_at(surfaces: &[Surface], landing: &Landing) -> Option<usize> {
    surfaces
        .iter()
        .enumerate()
        .filter(|(_, s)| s.height == landing.height)
        .map(|(i, s)| {
            let x = landing.x;
            let d = if x < s.x0 as f32 {
                s.x0 as f32 - x
            } else if x > s.x1 as f32 {
                x - s.x1 as f32
            } else {
                0.0
            };
            (i, d)
        })
        .filter(|(_, d)| *d <= HERO_HALF_WIDTH)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

impl Course {
    /// [`Self::walk_off`] that reports a walk that never settles (a wall in
    /// the way) as `None`.
    fn try_walk_off(&self, from: (f32, i32), dir: f32, to_x: f32) -> Option<Landing> {
        let mut hero = self.stand(from.0, from.1);
        let mut frame = InputFrame::default();
        let mut head = self.head(&hero);
        for tick in 0..150_u32 {
            let arrived = (hero.x - to_x) * dir >= 0.0;
            let raw = RawInput {
                move_x: if arrived { 0.0 } else { dir },
                ..RawInput::NONE
            };
            self.step(&mut hero, &mut frame, raw);
            head = head.max(self.head(&hero));
            if arrived && hero.grounded {
                return Some(self.land(&hero, head, tick));
            }
        }
        None
    }
}

/// Every edge of the surface graph reachable with the Floor 01 kit: a walk-off
/// over either edge and a jump from every 4th column with a hold of 6, 10, 16,
/// 24 ticks or a full hold. Returns the adjacency lists and the highest head.
fn graph(kit: &EnvKit, room: &Room) -> (Vec<Surface>, Vec<Vec<usize>>, f32) {
    let course = Course::new(kit);
    let surfaces = surfaces_of(kit, room);
    let mut edges = vec![Vec::new(); surfaces.len()];
    let mut head = 0.0_f32;
    let mut add = |from: usize, landing: Landing, edges: &mut Vec<Vec<usize>>| {
        head = head.max(landing.head);
        if let Some(to) = surface_at(&surfaces, &landing)
            && to != from
            && !edges[from].contains(&to)
        {
            edges[from].push(to);
        }
    };
    for (i, s) in surfaces.iter().enumerate() {
        // A hero stands at least a half width inside the 16 px end walls.
        let (x0, x1) = (
            s.x0.max(TILE + HERO_HALF_WIDTH as i32),
            s.x1.min(kit.width - TILE - HERO_HALF_WIDTH as i32),
        );
        for dir in [E, W] {
            let edge = if dir > 0.0 { x1 } else { x0 } as f32;
            if !course.embedded(edge, s.height)
                && let Some(l) = course.try_walk_off((edge, s.height), dir, edge + dir * 16.0)
            {
                add(i, l, &mut edges);
            }
            for x in (x0..=x1).step_by(4) {
                if course.embedded(x as f32, s.height) {
                    continue;
                }
                for hold in [Some(6), Some(10), Some(16), Some(24), FULL] {
                    let l = course.jump((x as f32, s.height), dir, hold);
                    add(i, l, &mut edges);
                }
            }
        }
    }
    (surfaces, edges, head)
}

fn reach(edges: &[Vec<usize>], from: usize) -> Vec<bool> {
    let mut seen = vec![false; edges.len()];
    let mut stack = vec![from];
    seen[from] = true;
    while let Some(n) = stack.pop() {
        for &m in &edges[n] {
            if !seen[m] {
                seen[m] = true;
                stack.push(m);
            }
        }
    }
    seen
}

/// Asserts 6 and 7: from the spawn the heroes visit every surface and the
/// exit, and every surface returns to the spawn and the exit (the surface
/// graph is strongly connected), the head never above 228 px. Carl and Donut
/// share the integration, see `a6_donut_steps_like_carl_in_the_scene`.
#[test]
fn a6_a7_every_surface_is_visited_from_the_spawn_and_nothing_softlocks() {
    for e in &EXPECT {
        let room = scenario(e);
        let kit = kit_of(&room);
        let (surfaces, edges, head) = graph(&kit, &room);
        assert!(head <= HEAD_LIMIT, "{}: head {head}", e.id);
        let spawn = surfaces
            .iter()
            .position(|s| s.height == 0 && (s.x0..=s.x1).contains(&room.spawn_carl))
            .unwrap();
        let exit = surfaces
            .iter()
            .position(|s| s.height == 0 && (s.x0..=s.x1).contains(&(room.exit.x + 32)))
            .unwrap();
        let forward = reach(&edges, spawn);
        assert!(
            forward.iter().all(|r| *r),
            "{}: surfaces not reached from the spawn: {:?}",
            e.id,
            surfaces
                .iter()
                .zip(&forward)
                .filter(|(_, r)| !**r)
                .map(|(s, _)| *s)
                .collect::<Vec<_>>()
        );
        for (i, s) in surfaces.iter().enumerate() {
            let back = reach(&edges, i);
            assert!(back[spawn], "{}: {s:?} cannot return to the spawn", e.id);
            assert!(back[exit], "{}: {s:?} cannot reach the exit", e.id);
        }
        // The surface counts of the spec: floor segments plus slabs and steps.
        let want = match e.id {
            "LAB-R00" => 7,
            "LAB-R02" => 9,
            _ => 2,
        };
        assert_eq!(surfaces.len(), want, "{}: {surfaces:?}", e.id);
    }
}

// --------------------------------------------------------- the live run

fn catalog() -> LabCatalog {
    LabCatalog::load(root()).unwrap()
}

fn launch(room: &str, variant: &str, mode: PlayerMode) -> LabRun {
    let mut cfg = LabConfig::new(room, mode);
    cfg.variant = variant.to_owned();
    catalog().launch(EntryPoint::CombatLab, &cfg).unwrap()
}

/// A room whose only enemy is one passive goblin parked on the entry floor
/// (a sandbox probe; a clear goal needs a goblin): isolates the heroes and the
/// bot from the fight.
fn launch_empty(room: &str, mode: PlayerMode) -> LabRun {
    let mut cfg = LabConfig::new(room, mode);
    cfg.probe = ProbeTag::Sandbox;
    cfg.enemies_override = match room {
        "R00" => Some(Vec::new()),
        _ => Some(vec![EnemySpec {
            kind: "goblin".into(),
            x: 136,
            ai: "passive".into(),
        }]),
    };
    catalog().launch(EntryPoint::CombatLab, &cfg).unwrap()
}

const NONE: [RawInput; 2] = [RawInput::NONE, RawInput::NONE];

/// Every lab play scenario: room, variant, the file entry of [`EXPECT`].
const PLAY: [(&str, &str, usize); 5] = [
    ("R00", "v1", 0),
    ("R01", "v1", 1),
    ("R01", "v2", 2),
    ("R01", "v3", 3),
    ("R02", "v1", 5),
];

/// Assert 6 (Donut): her body steps through the same `step_hero_driven` in the
/// live scene. The R00 hop from the floor onto the Hurdle, scripted into
/// either slot of a two-player run, ends on the same spot as the standalone
/// hero.
#[test]
fn a6_donut_steps_like_carl_in_the_scene() {
    let course = Course::new(&kit_of(&scenario(&EXPECT[0])));
    let landing = course.jump((200.0, 0), E, FULL);
    for who in [HeroId::Carl, HeroId::Donut] {
        let mut run = launch("R00", "v1", PlayerMode::CoopLocal);
        let other = who.other();
        *run.scene.world.body_mut(who) = course.stand(200.0, 0);
        *run.scene.world.body_mut(other) = course.stand(120.0, 0);
        for tick in 0..=landing.ticks {
            let mut slots = NONE;
            slots[who.index()] = RawInput {
                move_x: 1.0,
                jump: tick == 0,
                ..RawInput::NONE
            };
            run.step_slots(slots);
        }
        let body = run.scene.world.body(who);
        assert_eq!(
            (body.x, body.grounded),
            (landing.x, true),
            "{who:?} ends where the standalone hero does"
        );
        assert_eq!(
            (body.y - HERO_HALF_HEIGHT - course.floor_world).round() as i32,
            landing.height
        );
    }
}

// ------------------------------------------------------------------ assert 8

/// Assert 8: the roster equals section 7 and the JSON, every enemy is `Chase`
/// facing west at load, on the floor, at most 4 active.
#[test]
fn a8_the_rosters_equal_section_7_and_the_json() {
    let want: [(&str, &str, &[i32], &[i32]); 5] = [
        ("R00", "v1", &[], &[]),
        ("R01", "v1", &[560, 588, 880], &[]),
        ("R01", "v2", &[432, 832, 860], &[]),
        ("R01", "v3", &[512, 640, 768], &[]),
        ("R02", "v1", &[560, 592, 928], &[1024]),
    ];
    for ((room, variant, goblins, llamas), (_, _, index)) in want.iter().zip(PLAY) {
        let run = launch(room, variant, PlayerMode::SoloCarl);
        let json = scenario(&EXPECT[index]);
        let json_g: Vec<i32> = json
            .enemies
            .iter()
            .filter(|e| e.kind == crate::camera_lab::room::EnemyKind::Goblin)
            .map(|e| e.x)
            .collect();
        let json_l: Vec<i32> = json
            .enemies
            .iter()
            .filter(|e| e.kind == crate::camera_lab::room::EnemyKind::Llama)
            .map(|e| e.x)
            .collect();
        assert_eq!(&json_g, goblins, "{room} {variant}: JSON goblins");
        assert_eq!(&json_l, llamas, "{room} {variant}: JSON llamas");
        // The Rust fixture mirrors the JSON list (LAB-R-Q13).
        let fixture_g: Vec<i32> = run.info.fixture.enemies.iter().map(|e| e.x).collect();
        let fixture_l: Vec<i32> = run.info.fixture.llamas.iter().map(|e| e.x).collect();
        assert_eq!(
            (&fixture_g, &fixture_l),
            (&json_g, &json_l),
            "{room} {variant}"
        );
        assert!(
            run.info
                .fixture
                .enemies
                .iter()
                .all(|e| e.ai == EnemyAi::Chase)
        );
        assert!(
            run.info
                .fixture
                .llamas
                .iter()
                .all(|e| e.ai == EnemyAi::Chase)
        );
        // The state at load.
        let floor_y = to_world_y(run.kit.height, FLOOR) + 19.5;
        for g in &run.combat.enemies {
            assert_eq!((g.ai, g.facing), (EnemyAi::Chase, -1), "{room} {variant}");
            assert!(g.alive());
        }
        for l in &run.combat.llamas {
            assert_eq!((l.ai, l.facing), (EnemyAi::Chase, -1));
            assert_eq!(l.y, floor_y, "the llama stands on the floor");
        }
        let total = run.combat.enemies.len() + run.combat.llamas.len();
        assert_eq!(total, goblins.len() + llamas.len());
        assert!(
            total <= LAB_MAX_CONCURRENT,
            "{room} {variant}: {total} active"
        );
        // No room starts in a fight (spec rule 7): the nearest enemy is more
        // than the goblin's 220 px sight away from Carl.
        let carl = run.scene.world.hero.x;
        let nearest = run
            .combat
            .enemies
            .iter()
            .map(|g| g.x)
            .chain(run.combat.llamas.iter().map(|l| l.x))
            .map(|x| (x - carl).abs())
            .fold(f32::MAX, f32::min);
        assert!(total == 0 || nearest > 220.0, "{room} {variant}: {nearest}");
    }
}

// ------------------------------------------------------------------ assert 9

/// Assert 9: with Carl parked on each surface in turn for 3000 ticks (a death
/// respawns him at the checkpoint and the run continues) every enemy stays
/// inside its range: R02 section 5.2, R01 112..1008.
#[test]
fn a9_the_enemies_stay_inside_their_confinement_ranges() {
    // (room, variant, spec index, [(enemy index, is llama, x range)]). The
    // spec's llama range (887..1065) is the body span 23 px from the walls; the
    // llama's body is shifted `LLAMA_BODY_BACK_SHIFT` (3 px) behind its
    // column, so its column ranges 3 px further (a finding for the spec).
    type Ranges = &'static [(usize, bool, i32, i32)];
    let ranges: [(&str, &str, usize, Ranges); 4] = [
        (
            "R01",
            "v1",
            1,
            &[
                (0, false, 112, 1008),
                (1, false, 112, 1008),
                (2, false, 112, 1008),
            ],
        ),
        (
            "R01",
            "v2",
            2,
            &[
                (0, false, 112, 1008),
                (1, false, 112, 1008),
                (2, false, 112, 1008),
            ],
        ),
        (
            "R01",
            "v3",
            3,
            &[
                (0, false, 112, 1008),
                (1, false, 112, 1008),
                (2, false, 112, 1008),
            ],
        ),
        (
            "R02",
            "v1",
            5,
            &[
                (0, false, 428, 692),
                (1, false, 428, 692),
                (2, false, 876, 1076),
                (0, true, 887 - 3, 1065 + 3),
            ],
        ),
    ];
    for (room, variant, index, ranges) in ranges {
        let json = scenario(&EXPECT[index]);
        let kit = kit_of(&json);
        let course = Course::new(&kit);
        let surfaces = surfaces_of(&kit, &json);
        for s in &surfaces {
            let x = ((s.x0 + s.x1) / 2) as f32;
            if course.embedded(x, s.height) {
                continue;
            }
            let mut run = launch(room, variant, PlayerMode::SoloCarl);
            *run.scene.world.body_mut(HeroId::Carl) = course.stand(x, s.height);
            let mut seen = vec![(f32::MAX, f32::MIN); ranges.len()];
            for _ in 0..3000 {
                run.step_slots(NONE);
                for (slot, (i, llama, _, _)) in ranges.iter().enumerate() {
                    let at = if *llama {
                        run.combat.llamas[*i].x
                    } else {
                        run.combat.enemies[*i].x
                    };
                    seen[slot] = (seen[slot].0.min(at), seen[slot].1.max(at));
                }
            }
            for (slot, (i, llama, lo, hi)) in ranges.iter().enumerate() {
                assert!(
                    seen[slot].0 >= *lo as f32 && seen[slot].1 <= *hi as f32,
                    "{room} {variant}: enemy {i} (llama {llama}) left {lo}..{hi}: {:?} with Carl on {s:?}",
                    seen[slot]
                );
            }
        }
    }
}

// ----------------------------------------------------------------- assert 10

#[derive(Debug, PartialEq, Eq)]
enum Shot {
    Hit,
    Blocked,
    Spent,
}

/// Fire a scripted glob from a llama standing at `llama_x` towards `target_x`
/// and report what it did to a hurtbox standing `height` above the floor.
fn shoot(run: &LabRun, llama_x: f32, target_x: f32, height: i32, who: HeroId) -> Shot {
    let floor = to_world_y(run.kit.height, FLOOR);
    let dir: i8 = if target_x < llama_x { -1 } else { 1 };
    let mut glob = Glob {
        x: llama_x + f32::from(dir) * LLAMA_GLOB_MOUTH_DX,
        y: floor + LLAMA_GLOB_MOUTH_DY,
        dir,
        travelled: 0.0,
        owner: 0,
    };
    let (w, h) = match who {
        HeroId::Carl => (HERO_HALF_WIDTH * 2.0, HERO_HALF_HEIGHT * 2.0),
        HeroId::Donut => PROVISIONAL_TUNING.donut_hurtbox.size(),
    };
    let hurt = Rect {
        left: target_x - w * 0.5,
        bottom: floor + height as f32,
        width: w,
        height: h,
    };
    loop {
        match glob.step(&run.scene.world.solids) {
            GlobFate::Blocked => return Shot::Blocked,
            GlobFate::Spent => return Shot::Spent,
            GlobFate::Flying => {}
        }
        if glob.rect().overlaps(hurt) {
            return Shot::Hit;
        }
    }
}

/// Assert 10: the spit matrix of section 5.5 with scripted globs from the
/// whole yard (the llama's 887..1065 range).
#[test]
fn a10_the_spit_matrix_matches_section_5_5() {
    let run = launch("R02", "v1", PlayerMode::SoloCarl);
    let llamas = [887.0, 1024.0, 1065.0];
    let carl = HeroId::Carl;
    let donut = HeroId::Donut;
    for lx in llamas {
        // West of x 832 the glob is blocked by P1's east face: the entry, the
        // drop shelf and the pocket floor and ledges are safe from spit.
        for (x, h) in [(200.0, 0), (350.0, 32), (600.0, 0), (720.0, 32)] {
            assert_eq!(
                shoot(&run, lx, x, h, carl),
                Shot::Blocked,
                "llama {lx} Carl {x},{h}"
            );
        }
        // P1 is the refuge: the band passes below his feet and P1 stops it.
        assert_eq!(
            shoot(&run, lx, 780.0, 64, carl),
            Shot::Blocked,
            "llama {lx} P1"
        );
        // Everything east of 832 is exposed: the yard and bay floor, P2, P3.
        for (x, h) in [
            (930.0, 0),
            (1000.0, 0),
            (1200.0, 0),
            (848.0, 32),
            (1120.0, 32),
        ] {
            if (x - lx).abs() < 60.0 {
                continue; // inside the mouth offset: not a shot
            }
            if x < 860.0 && lx < 900.0 {
                continue;
            }
            assert_eq!(
                shoot(&run, lx, x, h, carl),
                Shot::Hit,
                "llama {lx} Carl {x},{h}"
            );
        }
        // Donut on a floor is never hit (her 24 px hurtbox is under the band);
        // on a 32 px ledge she is.
        for x in [930.0, 1000.0, 1200.0] {
            assert_ne!(
                shoot(&run, lx, x, 0, donut),
                Shot::Hit,
                "llama {lx} Donut floor {x}"
            );
        }
        assert_eq!(shoot(&run, lx, 1120.0, 32, donut), Shot::Hit, "Donut on P3");
    }
}

// ----------------------------------------------------------------- assert 11

/// Assert 11: the party wall engages at 448 px in a 2P run, and both heroes
/// can stand in every exit rect together (and one alone does not count).
#[test]
fn a11_the_coop_wall_engages_at_448_and_both_heroes_fit_the_exit() {
    let mut run = launch_empty("R01", PlayerMode::CoopLocal);
    let donut_x = run.scene.world.body(HeroId::Donut).x;
    let mut widest = 0.0_f32;
    for _ in 0..600 {
        run.step_slots([
            RawInput {
                move_x: 1.0,
                ..RawInput::NONE
            },
            RawInput::NONE,
        ]);
        let gap = run.scene.world.body(HeroId::Carl).x - run.scene.world.body(HeroId::Donut).x;
        widest = widest.max(gap);
    }
    assert!(
        (440.0..=448.5).contains(&widest),
        "the wall holds the pair at 448 px, saw {widest} (Donut at {donut_x})"
    );
    for (room, variant, index) in PLAY {
        let json = scenario(&EXPECT[index]);
        let course = Course::new(&kit_of(&json));
        let exit = crate::env_kit::IRect {
            x: json.exit.x,
            y: json.exit.y,
            w: json.exit.w,
            h: json.exit.h,
        };
        let mut duo = launch_empty(room, PlayerMode::CoopLocal);
        let _ = variant;
        let (a, b) = ((exit.x + 16) as f32, (exit.x + 48) as f32);
        assert!(!course.embedded(a, 0) && !course.embedded(b, 0), "{room}");
        *duo.scene.world.body_mut(HeroId::Carl) = course.stand(a, 0);
        *duo.scene.world.body_mut(HeroId::Donut) = course.stand(b, 0);
        assert!(
            flow::heroes_in_exit(&duo.scene, &duo.party, exit),
            "{room}: both inside"
        );
        *duo.scene.world.body_mut(HeroId::Donut) = course.stand(exit.x as f32 - 60.0, 0);
        assert!(
            !flow::heroes_in_exit(&duo.scene, &duo.party, exit),
            "{room}: one hero alone does not fire a two-player exit"
        );
        // Solo: the controlled hero alone fires it.
        let mut solo = launch_empty(room, PlayerMode::SoloCarl);
        *solo.scene.world.body_mut(HeroId::Carl) = course.stand(a, 0);
        assert!(
            flow::heroes_in_exit(&solo.scene, &solo.party, exit),
            "{room}: solo"
        );
    }
}

// ----------------------------------------------------------------- assert 12

fn walk_policy(run: &LabRun, goal_x: f32) -> RawInput {
    autopilot::walk(&run.scene, HeroId::Carl, goal_x)
}

/// Assert 12: with Carl walking the main path of R02 the follower trails him.
/// Regroup events (the bot placed at the view edge: one tick moves it more
/// than 40 px) are measured and reported, not required to be zero.
#[test]
fn a12_the_follower_trails_the_main_path_and_regroups_are_measured() {
    for (room, goal) in [("R00", 560.0), ("R01", 950.0), ("R02", 1200.0)] {
        let mut run = launch_empty(room, PlayerMode::SoloCarl);
        let mut regroups = 0;
        let mut last = run.scene.world.body(HeroId::Donut).x;
        let mut reached = None;
        let mut widest = 0.0_f32;
        for tick in 0..4000_u32 {
            let raw = walk_policy(&run, goal);
            run.step_slots([raw, RawInput::NONE]);
            let d = run.scene.world.body(HeroId::Donut).x;
            if (d - last).abs() > 40.0 {
                regroups += 1;
            }
            last = d;
            widest = widest.max((run.scene.world.hero.x - d).abs());
            if reached.is_none() && (run.scene.world.hero.x - goal).abs() < 6.0 {
                reached = Some(tick);
            }
        }
        eprintln!(
            "BOT {room}: Carl at the exit column after {reached:?} ticks, {regroups} regroup events, widest gap {widest:.0}"
        );
        assert!(
            reached.is_some(),
            "{room}: the walking policy reaches the exit column"
        );
        assert!(
            widest <= 448.5 + 64.0,
            "{room}: the follower never falls away: {widest}"
        );
    }
}

// ----------------------------------------------------------------- assert 13

fn autopilot(run: &LabRun, exit_x: f32) -> RawInput {
    autopilot::clear_and_exit(&run.combat, &run.scene, exit_x)
}

/// Assert 13: the legacy autopilot clears every play room. Soft targets: about
/// 2400 ticks for R01 and 3600 for R02; fail only above twice the target. The
/// actual values are printed.
#[test]
fn a13_the_autopilot_clears_r01_and_r02_within_twice_the_budget() {
    for (room, variant, index, target) in [
        ("R01", "v1", 1, 2400_u64),
        ("R01", "v2", 2, 2400),
        ("R01", "v3", 3, 2400),
        ("R02", "v1", 5, 3600),
    ] {
        let exit_x = scenario(&EXPECT[index]).exit.x as f32 + 32.0;
        let mut run = launch(room, variant, PlayerMode::SoloCarl);
        let mut cleared = None;
        for _ in 0..2 * target {
            let raw = autopilot(&run, exit_x);
            run.step_slots([raw, RawInput::NONE]);
            if run.goal_met() {
                cleared = Some(run.combat.elapsed);
                break;
            }
        }
        eprintln!(
            "CLEAR {room} {variant}: {cleared:?} ticks (target {target}), deaths {}",
            run.combat.deaths
        );
        assert!(
            cleared.is_some(),
            "{room} {variant}: not cleared in {} ticks",
            2 * target
        );
    }
}

// ------------------------------------------------------------ pinned goldens

/// The state hash after `ticks` ticks of the autopilot in solo Carl.
fn golden(room: &str, variant: &str, index: usize, ticks: u64) -> (u64, u64) {
    let exit_x = scenario(&EXPECT[index]).exit.x as f32 + 32.0;
    let mut run = launch(room, variant, PlayerMode::SoloCarl);
    let at_launch = run.state_hash();
    for _ in 0..ticks {
        let raw = autopilot(&run, exit_x);
        run.step_slots([raw, RawInput::NONE]);
    }
    (at_launch, run.state_hash())
}

/// LAB-01 goldens (review of LAB-00, item 12): the R01 variants and R02 are
/// pinned at launch and after 600 ticks of the autopilot, so geometry,
/// roster, checkpoint-rule or seed drift across commits is caught. A change
/// here is deliberate: re-pin together with the layout hash of the manifest.
const GOLDENS: [(&str, &str, usize, u64, u64); 4] = [
    ("R01", "v1", 1, 0xb1e3_b87b_242e_a449, 0xa323_d8f9_a618_736b),
    ("R01", "v2", 2, 0x89c5_0b50_5d36_870f, 0x3432_4460_4da5_e3a1),
    ("R01", "v3", 3, 0xe65b_f514_1400_dda0, 0x8939_379d_9e26_dfda),
    ("R02", "v1", 5, 0x8b02_473f_ed85_f766, 0x6e25_2adf_5dbb_0e06),
];

#[test]
fn the_lab_goldens_for_r01_and_r02_are_pinned() {
    let measured: Vec<(u64, u64)> = GOLDENS
        .iter()
        .map(|&(room, variant, index, _, _)| golden(room, variant, index, 600))
        .collect();
    for (m, g) in measured.iter().zip(GOLDENS) {
        eprintln!("GOLDEN {} {}: {:#018x} {:#018x}", g.0, g.1, m.0, m.1);
    }
    for (m, (room, variant, _, launch_hash, after)) in measured.into_iter().zip(GOLDENS) {
        assert_eq!(m, (launch_hash, after), "{room} {variant}");
    }
}
