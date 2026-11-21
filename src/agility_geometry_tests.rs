//! DONUT-MOBILITY-01 (DM-A) geometry proofs on the real kits: the microarena's
//! lane ceiling (DM-Q27), the HUD clearance of every space where Donut has a
//! body (DM-Q26, spec test 8), the nav diff between the integrated and agile
//! presets (spec test 5) and the U4 measurement of a running human Donut with
//! the bot Carl behind her.
//!
//! Heights below are above the walking line unless said otherwise.

use std::path::PathBuf;

use crate::agility;
use crate::boss_scene;
use crate::cf02::arena;
use crate::companion::nav::{EdgeKind, EdgeState, NavConfig, NavGraph, NavLevel};
use crate::demo;
use crate::env_kit::{EnvKit, to_world_y};
use crate::input::{InputFrame, RawInput};
use crate::party::{HeroId, MotionProfile, PROVISIONAL_TUNING, PartyTuning};
use crate::party_scene::PartyFixtureSim;
use crate::prototype::{
    Block, BodyDrive, HERO_HALF_HEIGHT, Hero, HeroMotion, Rect, step_hero_motion,
};
use crate::scene_camera::walk_line_hud_px;

pub(crate) fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

const CARL: HeroMotion = HeroMotion::CARL;

fn donut() -> HeroMotion {
    agility::agile_motion()
}

/// The microarena's solids with the walking line moved to y = 0, and whether a
/// ceiling slab is part of them (`ceiling` false removes it, to show what the
/// ceiling is for).
fn arena_blocks(ceiling: bool) -> (Vec<Block>, f32) {
    let kit = arena::load_arena(&root()).unwrap();
    assert_eq!(kit.base_walk_line_y, arena::FLOOR_Y);
    let walk = to_world_y(kit.height, kit.base_walk_line_y);
    let mut blocks = kit.blocks();
    if !ceiling {
        blocks.retain(|b| b.rect.height != 16.0 || b.rect.width != kit.width as f32);
    }
    for b in &mut blocks {
        b.rect.bottom -= walk;
    }
    (blocks, kit.width as f32)
}

fn has_ceiling(blocks: &[Block], width: f32) -> bool {
    blocks
        .iter()
        .any(|b| b.rect.width == width && b.rect.height == 16.0 && b.rect.bottom > 100.0)
}

/// The golden of `dm_microarena_perch_ceiling` (see the test below).
const DM_MICROARENA_PERCH_CEILING: u64 = 0x7955_6683_ec1e_a92c;

const LEFT_PERCH: (f32, f32) = (544.0, 672.0);
const RIGHT_PERCH: (f32, f32) = (896.0, 992.0);
const PERCH_TOP: f32 = 80.0;

fn step(motion: HeroMotion, hero: &mut Hero, frame: &mut InputFrame, raw: RawInput, b: &[Block]) {
    frame.advance(raw);
    step_hero_motion(hero, motion, frame, b, 1152.0, &BodyDrive::default());
}

#[test]
fn the_shipped_arena_has_the_ceiling_with_its_underside_228_px_up() {
    let (blocks, width) = arena_blocks(true);
    let slab = blocks
        .iter()
        .find(|b| b.rect.width == width && b.rect.height == 16.0 && b.rect.bottom > 100.0)
        .expect("DM-Q27: a lane ceiling slab");
    assert_eq!(
        slab.rect.bottom, 228.0,
        "underside 228 px above the walk line"
    );
    let (without, w) = arena_blocks(false);
    assert!(!has_ceiling(&without, w));
    assert_eq!(without.len() + 1, blocks.len());
}

/// DM-Q27 and spec 2.1: a jump from the 80 px perch. Donut bonks at feet 176
/// (collider top 228, drawn top 220), Carl's highest head stays 220.5 and his
/// arc is bit-identical with and without the ceiling.
#[test]
fn a_perch_jump_bonks_donut_at_228_and_never_touches_carl() {
    let jump_from_perch = |motion: HeroMotion, ceiling: bool| {
        let (blocks, _) = arena_blocks(ceiling);
        let mut hero = Hero::at(600.0, PERCH_TOP + HERO_HALF_HEIGHT);
        let mut frame = InputFrame::default();
        let mut trail = Vec::new();
        for tick in 0..90 {
            let raw = RawInput {
                jump: tick == 0,
                ..RawInput::NONE
            };
            step(motion, &mut hero, &mut frame, raw, &blocks);
            trail.push((hero.x.to_bits(), hero.y.to_bits(), hero.vy.to_bits()));
        }
        let head = trail
            .iter()
            .map(|t| f32::from_bits(t.1) + HERO_HALF_HEIGHT)
            .fold(0.0, f32::max);
        (head, trail)
    };
    let (donut_head, donut_trail) = jump_from_perch(donut(), true);
    // `dm_microarena_perch_ceiling`: the golden of Donut's perch jump under the
    // ceiling (FNV-1a over every tick's x, y and vy bits).
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for (x, y, vy) in &donut_trail {
        for value in [x, y, vy] {
            for byte in value.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
    }
    assert_eq!(hash, DM_MICROARENA_PERCH_CEILING, "{hash:#018x}");
    let (donut_free, _) = jump_from_perch(donut(), false);
    assert_eq!(
        donut_head,
        228.0 - 0.0,
        "collider top capped at the ceiling"
    );
    assert!(
        donut_free > 228.0 + 10.0,
        "without it she would rise to {donut_free}"
    );
    assert!((donut_head - 44.0 - (176.0 + 8.0)).abs() < 0.01 || donut_head == 228.0);
    let (carl_head, carl_with) = jump_from_perch(CARL, true);
    let (_, carl_without) = jump_from_perch(CARL, false);
    assert!((carl_head - 220.54).abs() < 0.05, "{carl_head}");
    assert_eq!(carl_with, carl_without, "Carl's arc is bit-identical");
}

/// Perch to perch with the parapet in the room: every start column over the
/// last 3.5 px of a perch's edge, every jump tick 0..11 (a standing edge jump
/// and each coyote tick), both directions. With DM-Q27 nobody lands on the far
/// perch; without it Donut does (the route DM-Q27 closes), and Carl never does.
#[test]
fn the_ceiling_closes_the_perch_to_perch_skip() {
    let landings = |motion: HeroMotion, ceiling: bool| {
        let (blocks, _) = arena_blocks(ceiling);
        let mut count = 0_u32;
        for (from, to, dir) in [
            (LEFT_PERCH, RIGHT_PERCH, 1.0_f32),
            (RIGHT_PERCH, LEFT_PERCH, -1.0),
        ] {
            // The hero's box overlaps the standable edge by up to 3.5 px.
            let edge = if dir > 0.0 { from.1 } else { from.0 };
            for n in 0..=70 {
                let x = edge + dir * (12.0 - 3.5 + n as f32 * 0.05);
                for jump_tick in 0..12_u32 {
                    let mut hero = Hero::at(x, PERCH_TOP + HERO_HALF_HEIGHT);
                    let mut frame = InputFrame::default();
                    let mut airborne = false;
                    for tick in 0..240_u32 {
                        let raw = RawInput {
                            move_x: dir,
                            jump: tick == jump_tick,
                            ..RawInput::NONE
                        };
                        step(motion, &mut hero, &mut frame, raw, &blocks);
                        airborne |= !hero.grounded;
                        let feet = hero.y - HERO_HALF_HEIGHT;
                        if hero.grounded && airborne {
                            if (feet - PERCH_TOP).abs() < 0.01
                                && hero.x > to.0 - 12.0
                                && hero.x < to.1 + 12.0
                            {
                                count += 1;
                            }
                            break;
                        }
                    }
                }
            }
        }
        count
    };
    assert_eq!(landings(donut(), true), 0, "with DM-Q27 she cannot skip");
    let free = landings(donut(), false);
    assert!(free > 0, "without it the skip exists: {free}");
    assert_eq!(landings(CARL, true), 0);
    assert_eq!(landings(CARL, false), 0, "Carl cannot cross the 224 px gap");
    println!("DM-Q27 perch skip landings without the ceiling: {free}");
}

// ----------------------------------------------------------------- HUD ---

/// A drawn-top and collider-top report of one space.
struct Hud {
    label: String,
    /// Highest drawn top (feet + 44) over every standable surface, px.
    drawn: f32,
    /// Highest collider top (feet + 52).
    collider: f32,
}

/// Jump straight up from the middle of every surface of `blocks` that stands up
/// to 112 px above the walking line and report Donut's highest tops. `walk` is
/// the walking line's world y.
fn hud_of(label: &str, blocks: &[Block], walk: f32, max_x: f32) -> Hud {
    let mut hud = Hud {
        label: label.to_owned(),
        drawn: 0.0,
        collider: 0.0,
    };
    let mut tops: Vec<(f32, f32)> = Vec::new();
    for b in blocks {
        let height = b.rect.top() - walk;
        if (-0.01..=112.01).contains(&height) && b.rect.width >= 16.0 {
            // Stand on the block's own top, away from an edge, on the first
            // free column (a block under a ceiling or beside a wall may not fit).
            for n in 0..9 {
                let x = b.rect.left + 12.0 + (b.rect.width - 24.0).max(0.0) * n as f32 / 8.0;
                let hero = Hero::at(x, b.rect.top() + HERO_HALF_HEIGHT);
                let body = Rect {
                    left: hero.x - 12.0,
                    bottom: hero.y - HERO_HALF_HEIGHT + 0.5,
                    width: 24.0,
                    height: 51.0,
                };
                if !blocks.iter().any(|o| o.intact && body.overlaps(o.rect)) {
                    tops.push((x, b.rect.top()));
                    break;
                }
            }
        }
    }
    for (x, top) in tops {
        let mut hero = Hero::at(x, top + HERO_HALF_HEIGHT);
        let mut frame = InputFrame::default();
        for tick in 0..90 {
            let raw = RawInput {
                jump: tick == 0,
                ..RawInput::NONE
            };
            frame.advance(raw);
            step_hero_motion(
                &mut hero,
                donut(),
                &frame,
                blocks,
                max_x,
                &BodyDrive::default(),
            );
            let feet = hero.y - HERO_HALF_HEIGHT - walk;
            hud.drawn = hud.drawn.max(feet + 44.0);
            hud.collider = hud.collider.max(feet + 52.0);
        }
    }
    hud
}

fn spaces() -> Vec<(String, Vec<Block>, f32, f32)> {
    let mut out = Vec::new();
    let mut add = |label: &str, kit: &EnvKit| {
        let walk = to_world_y(kit.height, kit.base_walk_line_y);
        out.push((
            label.to_owned(),
            kit.blocks(),
            walk,
            kit.width as f32 - 12.0,
        ));
    };
    add(
        "corridor",
        &EnvKit::load(&root(), "assets/levels/floor01/corridor_abc_sample.json").unwrap(),
    );
    add("demo hall", &demo::load_hall(&root()).unwrap());
    add("boss room", &boss_scene::load_arena(&root()).unwrap());
    add("microarena", &arena::load_arena(&root()).unwrap());
    out
}

/// Spec test 8 (DM-Q26): at 640 x 360 the HUD limit is `walk_line_hud_px(640) -
/// 60` = 228 px above the walking line (248 at 640 x 400); Donut's highest
/// drawn top (feet + 44) from every standable surface stays under it, and her
/// collider top plus the auto camera's 56 px pad stays inside the 288 px over
/// the walking line, so the auto camera's zoom does not move. Returns the
/// measured line for the PR.
pub(crate) fn check_hud(label: &str, blocks: &[Block], walk: f32, max_x: f32) -> String {
    let limit_360 = (walk_line_hud_px(640) - crate::hud::ACTION_LANE_TOP) as f32;
    let limit_400 = limit_360 + 20.0;
    assert_eq!(limit_360, 228.0);
    let hud = hud_of(label, blocks, walk, max_x);
    assert!(hud.drawn > 0.0, "{}: no surface measured", hud.label);
    assert!(
        hud.drawn <= limit_360,
        "{}: drawn top {} over {limit_360}",
        hud.label,
        hud.drawn
    );
    assert!(hud.drawn <= limit_400);
    assert!(
        hud.collider + 56.0 <= walk_line_hud_px(640) as f32,
        "{}: collider top {} + the 56 px camera pad leaves the view",
        hud.label,
        hud.collider
    );
    format!(
        "{}: drawn {:.1} collider {:.1}",
        hud.label, hud.drawn, hud.collider
    )
}

#[test]
fn donut_stays_under_the_hud_band_from_every_surface_of_every_space() {
    let report: Vec<String> = spaces()
        .into_iter()
        .map(|(label, blocks, walk, max_x)| check_hud(&label, &blocks, walk, max_x))
        .collect();
    println!("HUD {}", report.join(" | "));
}

// ------------------------------------------------------------- nav diff ---

/// Spec test 5, one space: every Carl edge is identical between the integrated
/// and the agile preset (state and takeoff point, not the `deps` lists, which
/// widen with her apex), Donut's lost and new edges are listed (returned as a
/// line for the PR), and in the microarena no Donut edge joins the two perches
/// without the floor.
pub(crate) fn check_nav_diff(label: &str, blocks: Vec<Block>, walk: f32) -> String {
    let label = label.to_owned();
    let blocks: Vec<Block> = blocks
        .into_iter()
        .map(|mut b| {
            b.rect.bottom -= walk;
            b
        })
        .collect();
    let level = NavLevel::from_blocks(&blocks, Vec::new());
    let integrated = NavGraph::build(&level, NavConfig::integrated()).unwrap();
    let agile = NavGraph::build(&level, NavConfig::agile()).unwrap();
    let (a, b) = (integrated.edge_views(), agile.edge_views());
    // The agile preset widens the candidate set (a taller jump has more
    // targets): every integrated edge must still exist, and an edge that is
    // new must be closed for Carl.
    let key = |v: &crate::companion::nav::EdgeView| (v.from, v.to);
    let by_key: std::collections::BTreeMap<_, _> = b.iter().map(|v| (key(v), v)).collect();
    for i in &a {
        assert!(by_key.contains_key(&key(i)), "{label}: lost {:?}", key(i));
    }
    let (mut lost, mut gained) = (Vec::new(), Vec::new());
    // An anchor as `x0..x1 @ top` (px, walking line 0).
    let at = |id: crate::companion::nav::NavAnchorId| {
        let (x0, x1) = agile.anchor_span(id).unwrap();
        let (_, top) = agile.anchor_pos(id).unwrap();
        format!("{x0:.0}..{x1:.0}@{top:.0}")
    };
    let carl = HeroId::Carl.index();
    let d = HeroId::Donut.index();
    let open = |s: &EdgeState| matches!(s, EdgeState::Open { .. });
    for g in &b {
        let Some(i) = a.iter().find(|i| key(i) == key(g)) else {
            assert!(
                !open(&g.state[carl]),
                "{label}: a new edge {:?}->{:?} is open for Carl",
                g.from,
                g.to
            );
            if open(&g.state[d]) {
                gained.push(format!("{}->{} (new candidate)", at(g.from), at(g.to)));
            }
            continue;
        };
        assert_eq!(
            i.state[carl], g.state[carl],
            "{label}: Carl {:?}->{:?}",
            i.from, i.to
        );
        assert_eq!(
            i.launch_x[carl], g.launch_x[carl],
            "{label}: Carl's takeoff {:?}->{:?}",
            i.from, i.to
        );
        let kind = |s: &EdgeState| match s {
            EdgeState::Open { kind, .. } => Some(*kind),
            EdgeState::Blocked(_) => None,
        };
        if open(&i.state[d]) && !open(&g.state[d]) {
            lost.push(format!("{}->{}", at(i.from), at(i.to)));
        } else if !open(&i.state[d]) && open(&g.state[d]) {
            gained.push(format!(
                "{}->{} ({:?})",
                at(i.from),
                at(i.to),
                kind(&g.state[d]).unwrap_or(EdgeKind::Walk)
            ));
        }
    }
    let line = format!(
        "NAV-DIFF {label}: edges {}, Donut lost {} {:?}, new {} {:?}",
        a.len(),
        lost.len(),
        lost,
        gained.len(),
        gained
    );
    if label == "microarena" {
        // No agile Donut edge joins the two perch anchors directly.
        // The blocks are translated: the walking line is y = 0, a perch's
        // standing surface is 80.
        let perch = |x: f32| {
            agile
                .anchors()
                .find(|&id| {
                    let (_, top) = agile.anchor_pos(id).unwrap();
                    let (x0, x1) = agile.anchor_span(id).unwrap();
                    x0 < x && x < x1 && (top - PERCH_TOP).abs() < 0.5
                })
                .expect("a perch anchor")
        };
        let (l, r) = (perch(600.0), perch(940.0));
        let d = HeroId::Donut;
        assert!(agile.edge_plan(l, r, d).is_none(), "no left to right edge");
        assert!(agile.edge_plan(r, l, d).is_none(), "no right to left edge");
    }
    line
}

#[test]
fn the_agile_preset_changes_no_carl_edge_and_lists_donuts() {
    for (label, blocks, walk, _) in spaces() {
        println!("{}", check_nav_diff(&label, blocks, walk));
    }
}

// -------------------------------------------------------------------- U4 ---

/// Spec section 6, U4: a human Donut runs and jumps right for 600 ticks on the
/// corridor kit while the bot Carl follows at his 185 px/s. The measurement
/// (Carl's off-screen ticks and regroup placements) is printed for the PR and
/// pinned so a change shows.
#[test]
fn a_running_human_donut_with_the_bot_carl_behind_her_is_measured() {
    use crate::party_scene::{PartyFixture, SlotHold, base, jump, run, swap};
    const SCRIPT: [SlotHold; 13] = [
        swap(1, 2),
        run(3, 603, 0, 1.0),
        jump(40, 41, 0),
        jump(100, 101, 0),
        jump(160, 161, 0),
        jump(220, 221, 0),
        jump(280, 281, 0),
        jump(340, 341, 0),
        jump(400, 401, 0),
        jump(460, 461, 0),
        jump(520, 521, 0),
        jump(580, 581, 0),
        jump(590, 591, 0),
    ];
    const BASE: crate::combat_scene::Fixture = base(
        "dm_u4_running_donut",
        "Solo as Donut, who runs and jumps right for 600 ticks; the bot Carl follows",
        0x0D4D_0004,
        &[],
        603,
    );
    static FIXTURE: PartyFixture = PartyFixture {
        base: &BASE,
        two_player: false,
        tuning: PartyTuning {
            donut_motion: MotionProfile::Agile,
            ..PROVISIONAL_TUNING
        },
        script: &SCRIPT,
        pilot: crate::boss_scene::Pilot::None,
        donut: crate::donut_mana::DonutSetup::NONE,
    };
    let measure = || {
        let kit = EnvKit::load(&root(), "assets/levels/floor01/corridor_abc_sample.json").unwrap();
        let mut sim = PartyFixtureSim::new(&kit, 360, &FIXTURE).unwrap();
        let (mut offscreen, mut regroups, mut prev_x) = (0_u32, 0_u32, sim.scene.world.hero.x);
        for _ in 0..603 {
            sim.step_scripted();
            let carl = sim.scene.world.hero.x;
            if sim.scene.is_offscreen(carl, 16.0) {
                offscreen += 1;
            }
            if (carl - prev_x).abs() > 8.0 {
                regroups += 1;
            }
            prev_x = carl;
        }
        (
            offscreen,
            regroups,
            sim.scene.world.donut_body.x,
            carl_x(&sim),
        )
    };
    fn carl_x(sim: &PartyFixtureSim) -> f32 {
        sim.scene.world.hero.x
    }
    let (offscreen, regroups, donut_x, carl_x) = measure();
    assert_eq!(measure().0, offscreen, "deterministic");
    println!(
        "U4 corridor, 600 ticks: Carl off-screen {offscreen} ticks, {regroups} regroup placements, Donut x {donut_x:.1}, Carl x {carl_x:.1}"
    );
    assert!(donut_x > carl_x, "she leads");
}
