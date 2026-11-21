//! The physics-exact reachability check of an encounter room (K8, BOIL-A wire).
//!
//! The validator in [`crate::encounter`] reasons over a route graph whose
//! height classes are **data**: an optimistic model (`reach_x_px` is a flat-jump
//! bound, ledge clearance ignores a ceiling). This module takes that graph
//! exactly as the validator built it ([`crate::encounter::route_graphs`]) and
//! replays every move it accepted with the **real hero integrator**
//! ([`crate::prototype::step_hero_motion`]) on the **real solids of the
//! composed kit** (the room's tiles, the DP-Q45 lane ceiling slab) plus the
//! solid props of the destruction state. Three hero classes are replayed with
//! their real motions: Carl, base Donut (the agile preset) and boosted Donut
//! (Light on Your Feet's launch, DP-Q14).
//!
//! What it asserts (see [`PhysicsReport`]):
//! - **Every edge the validator accepts is really traversable**: some takeoff
//!   column, direction and jump tick lands the body standing on the target
//!   surface. An edge with no witness is a validator false positive.
//! - **The exit is reachable** in every class and destruction state: once over
//!   the witnessed edges only, and once by an actual walk along the lane that
//!   hops what is in the way, ending in the protected exit rectangle.
//!
//! Bevy-free and deterministic: a fixed search order, integer-tick inputs, no
//! RNG and no clock. It is a **test and tooling** module: no scene calls it.
//! Later rooms (the Hoarder, the Ring) run the same check on their own kit.

use crate::agility;
use crate::camera_lab::room::IRect as LayoutRect;
use crate::donut_prog::DonutProgTuning;
use crate::encounter::{Loaded, ReachHero, RouteNode, route_graphs};
use crate::env_kit::{EnvKit, IRect, to_world_y};
use crate::input::{InputFrame, RawInput};
use crate::prototype::{
    Block, BodyDrive, HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Hero, HeroMotion, Material, Rect,
    step_hero_motion,
};

/// Ticks one takeoff trial runs before it is given up on.
const TRIAL_TICKS: u32 = 150;
/// Jump ticks tried: the jump is pressed on this tick of the trial (0 is a
/// standing jump, a later tick a running jump from further back), or never.
const JUMP_TICKS: u32 = 28;
/// Takeoff columns are sampled this far apart (px).
const COLUMN_STEP_PX: f32 = 4.0;
/// A walk along the lane gives up after this many ticks.
const WALK_TICKS: u32 = 2400;
/// The walker hops an obstacle that starts this far ahead of its box (px).
const HOP_LOOKAHEAD_PX: f32 = 10.0;

/// The motion of a hero class: Carl's, base Donut's agile preset, boosted
/// Donut's with the DP-Q14 launch (as `llama_rush` and the DP tests build it).
pub fn motion_of(hero: ReachHero) -> HeroMotion {
    match hero {
        ReachHero::Carl => HeroMotion::CARL,
        ReachHero::DonutBase => agility::agile_motion(),
        ReachHero::DonutBoosted => {
            let mut motion = agility::agile_motion();
            motion.jump_speed = f32::from(DonutProgTuning::active().dpq14_boost_launch_px_s);
            motion
        }
    }
}

/// The takeoff that lands: where the body started, which way it ran, and the
/// tick it jumped on (`None`: it walked off).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Witness {
    pub start_x: f32,
    pub dir: f32,
    pub jump_tick: Option<u32>,
    pub ticks: u32,
}

/// One accepted move of the validator's graph and whether the body can make it.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeReport {
    pub hero: ReachHero,
    pub state: u16,
    pub locks_closed: bool,
    pub from: RouteNode,
    pub to: RouteNode,
    /// `None`: no takeoff lands on the target (a validator false positive).
    pub witness: Option<Witness>,
}

/// The exit of one class in one destruction state.
#[derive(Debug, Clone, PartialEq)]
pub struct ExitReport {
    pub hero: ReachHero,
    pub state: u16,
    /// The witnessed edges alone join the spawn's floor to the exit's floor.
    pub by_edges: bool,
    /// The tick an actual walk along the lane enters the exit (`None`: never).
    pub walk_tick: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicsReport {
    pub edges: Vec<EdgeReport>,
    pub exits: Vec<ExitReport>,
}

impl PhysicsReport {
    /// The accepted edges no takeoff lands.
    pub fn untraversable(&self) -> Vec<&EdgeReport> {
        self.edges.iter().filter(|e| e.witness.is_none()).collect()
    }

    /// The class and state whose exit is not reached both ways.
    pub fn exit_failures(&self) -> Vec<&ExitReport> {
        self.exits
            .iter()
            .filter(|e| !e.by_edges || e.walk_tick.is_none())
            .collect()
    }

    pub fn is_clean(&self) -> bool {
        self.untraversable().is_empty() && self.exit_failures().is_empty()
    }

    /// One line per class: edges replayed and the walk to the exit.
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        for hero in ReachHero::ALL {
            let edges: Vec<&EdgeReport> = self.edges.iter().filter(|e| e.hero == hero).collect();
            let landed = edges.iter().filter(|e| e.witness.is_some()).count();
            let walks: Vec<u32> = self
                .exits
                .iter()
                .filter(|e| e.hero == hero)
                .filter_map(|e| e.walk_tick)
                .collect();
            lines.push(format!(
                "{}: {landed}/{} edges land, exit walk {:?} ticks",
                hero.name(),
                edges.len(),
                (walks.iter().min(), walks.iter().max())
            ));
        }
        lines.join(" | ")
    }
}

/// The solids and bounds of one replay.
struct World {
    blocks: Vec<Block>,
    layout_height: i32,
    max_x: f32,
    motion: HeroMotion,
}

fn block_of(layout_height: i32, r: IRect) -> Block {
    Block {
        rect: Rect {
            left: r.x as f32,
            bottom: to_world_y(layout_height, r.y + r.h),
            width: r.w as f32,
            height: r.h as f32,
        },
        material: Material::Stone,
        intact: true,
    }
}

fn kit_rect(r: LayoutRect) -> IRect {
    IRect {
        x: r.x,
        y: r.y,
        w: r.w,
        h: r.h,
    }
}

impl World {
    fn new(kit: &EnvKit, hero: ReachHero, solids: &[LayoutRect]) -> Self {
        let mut blocks = kit.blocks();
        blocks.extend(solids.iter().map(|s| block_of(kit.height, kit_rect(*s))));
        Self {
            blocks,
            layout_height: kit.height,
            max_x: kit.width as f32 - HERO_HALF_WIDTH,
            motion: motion_of(hero),
        }
    }

    fn feet(&self, layout_y: i32) -> f32 {
        to_world_y(self.layout_height, layout_y)
    }

    /// A body standing at `x` on a surface whose feet are at `feet` overlaps no
    /// solid (a column under a ceiling or inside a prop is no takeoff).
    fn fits(&self, x: f32, feet: f32) -> bool {
        let body = Rect {
            left: x - HERO_HALF_WIDTH,
            bottom: feet + 0.5,
            width: HERO_HALF_WIDTH * 2.0,
            height: HERO_HALF_HEIGHT * 2.0 - 1.0,
        };
        !self
            .blocks
            .iter()
            .any(|b| b.intact && body.overlaps(b.rect))
    }

    /// One trial: stand at `x` on `from_feet`, run `dir`, press jump on
    /// `jump_tick`. The first time the body is grounded again after leaving the
    /// ground, report whether it stands on the target (feet on its top, centre
    /// inside its span).
    fn trial(
        &self,
        x: f32,
        from_feet: f32,
        dir: f32,
        jump_tick: Option<u32>,
        to: &RouteNode,
    ) -> Option<u32> {
        let to_feet = self.feet(to.top);
        let mut hero = Hero::at(x, from_feet + HERO_HALF_HEIGHT);
        let mut frame = InputFrame::default();
        let mut airborne = false;
        for tick in 0..TRIAL_TICKS {
            frame.advance(RawInput {
                move_x: dir,
                jump: jump_tick == Some(tick),
                ..RawInput::NONE
            });
            step_hero_motion(
                &mut hero,
                self.motion,
                &frame,
                &self.blocks,
                self.max_x,
                &BodyDrive::default(),
            );
            airborne |= !hero.grounded;
            let feet = hero.y - HERO_HALF_HEIGHT;
            // A move between two surfaces of one height (a remnant against a
            // drum) is a walk: the body never leaves the ground.
            let walked = !airborne
                && hero.grounded
                && (to_feet - from_feet).abs() < 0.01
                && (feet - to_feet).abs() < 0.01
                && hero.x >= to.x0 as f32
                && hero.x <= to.x1 as f32;
            if walked {
                return Some(tick + 1);
            }
            if airborne && hero.grounded {
                let there = (feet - to_feet).abs() < 0.01
                    && hero.x >= to.x0 as f32
                    && hero.x <= to.x1 as f32;
                return there.then_some(tick + 1);
            }
            if feet < to_feet - 400.0 {
                return None;
            }
        }
        None
    }

    /// Search the takeoffs of the edge `from -> to` in a fixed order.
    fn witness(&self, from: &RouteNode, to: &RouteNode, reach_px: f32) -> Option<Witness> {
        let from_feet = self.feet(from.top);
        // Columns the body can stand on: the source span, narrowed to the
        // neighbourhood of the target (a long floor is not searched end to end).
        let lo = (from.x0 as f32 + 1.0).max(to.x0 as f32 - reach_px - 16.0);
        let hi = (from.x1 as f32 - 1.0).min(to.x1 as f32 + reach_px + 16.0);
        let mut columns = Vec::new();
        let mut x = lo;
        while x <= hi {
            columns.push(x);
            x += COLUMN_STEP_PX;
        }
        let target = (to.x0 + to.x1) as f32 * 0.5;
        columns.retain(|c| self.fits(*c, from_feet));
        // Nearest the target first: the usual takeoff is found early.
        columns.sort_by(|a, b| (a - target).abs().total_cmp(&(b - target).abs()));
        let jumps: Vec<Option<u32>> = std::iter::once(None)
            .chain((0..=JUMP_TICKS).map(Some))
            .collect();
        for &x in &columns {
            let towards = if target >= x { 1.0 } else { -1.0 };
            for dir in [towards, -towards] {
                for &jump in &jumps {
                    if let Some(ticks) = self.trial(x, from_feet, dir, jump, to) {
                        return Some(Witness {
                            start_x: x,
                            dir,
                            jump_tick: jump,
                            ticks,
                        });
                    }
                }
            }
        }
        None
    }

    /// Walk right along the lane from `start_x`, hopping what blocks the way;
    /// the tick the body's box first overlaps `exit`.
    fn walk_to(&self, start_x: f32, floor_y: i32, exit: LayoutRect) -> Option<u32> {
        let exit = block_of(self.layout_height, kit_rect(exit)).rect;
        let mut hero = Hero::at(start_x, self.feet(floor_y) + HERO_HALF_HEIGHT);
        let mut frame = InputFrame::default();
        for tick in 0..WALK_TICKS {
            let feet = hero.y - HERO_HALF_HEIGHT;
            let ahead = Rect {
                left: hero.x + HERO_HALF_WIDTH,
                bottom: feet + 0.5,
                width: HOP_LOOKAHEAD_PX,
                height: HERO_HALF_HEIGHT * 2.0 - 1.0,
            };
            let blocked = hero.grounded
                && self
                    .blocks
                    .iter()
                    .any(|b| b.intact && ahead.overlaps(b.rect));
            frame.advance(RawInput {
                move_x: 1.0,
                jump: blocked,
                ..RawInput::NONE
            });
            step_hero_motion(
                &mut hero,
                self.motion,
                &frame,
                &self.blocks,
                self.max_x,
                &BodyDrive::default(),
            );
            let body = Rect {
                left: hero.x - HERO_HALF_WIDTH,
                bottom: hero.y - HERO_HALF_HEIGHT,
                width: HERO_HALF_WIDTH * 2.0,
                height: HERO_HALF_HEIGHT * 2.0,
            };
            if body.overlaps(exit) {
                return Some(tick + 1);
            }
        }
        None
    }
}

/// Is `goal` reachable from `spawn` over the witnessed edges only?
fn joined(nodes: usize, edges: &[(usize, usize)], spawn: usize, goal: usize) -> bool {
    let mut seen = vec![false; nodes];
    let mut stack = vec![spawn];
    seen[spawn] = true;
    while let Some(i) = stack.pop() {
        for &(a, b) in edges {
            if a == i && !seen[b] {
                seen[b] = true;
                stack.push(b);
            }
        }
    }
    seen[goal]
}

/// The reach in px of a class's data (the takeoff search window).
fn reach_px(loaded: &Loaded, hero: ReachHero) -> f32 {
    loaded.classes.reach_x(hero) as f32
}

/// Replay every edge the validator accepted for `loaded` on `kit`, and walk
/// the lane to the exit. `kit` must be the scene's kit (its lane ceiling
/// included): a room that declares `ceiling_px` whose kit has no slab is an
/// error, never a pass.
pub fn check(kit: &EnvKit, loaded: &Loaded) -> Result<PhysicsReport, String> {
    let room = &loaded.room;
    if let Some(ceiling) = loaded.ceiling_px {
        let underside = kit.base_walk_line_y - ceiling;
        let has_slab = kit
            .solid_rects
            .iter()
            .any(|s| s.w >= kit.width && s.y + s.h == underside);
        if !has_slab {
            return Err(format!(
                "the room declares ceiling_px {ceiling} but the kit has no slab with its \
                 underside {ceiling} px above the walking line"
            ));
        }
    }
    let mut report = PhysicsReport {
        edges: Vec::new(),
        exits: Vec::new(),
    };
    for graph in route_graphs(room, &loaded.classes, &loaded.encounter) {
        let world = World::new(kit, graph.hero, &graph.solids);
        let reach = reach_px(loaded, graph.hero);
        let mut witnessed = Vec::new();
        for &(a, b) in &graph.edges {
            let (from, to) = (graph.nodes[a], graph.nodes[b]);
            let witness = world.witness(&from, &to, reach);
            if witness.is_some() {
                witnessed.push((a, b));
            }
            report.edges.push(EdgeReport {
                hero: graph.hero,
                state: graph.state,
                locks_closed: graph.locks_closed,
                from,
                to,
                witness,
            });
        }
        if graph.locks_closed {
            continue;
        }
        let spawn_x = match graph.hero {
            ReachHero::Carl => room.spawn_carl,
            _ => room.spawn_donut,
        };
        let by_edges = matches!(
            (graph.spawn, graph.goal),
            (Some(s), Some(g)) if joined(graph.nodes.len(), &witnessed, s, g)
        );
        report.exits.push(ExitReport {
            hero: graph.hero,
            state: graph.state,
            by_edges,
            walk_tick: world.walk_to(spawn_x as f32, room.floor_y, room.exit),
        });
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::f01_boiler::launch;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn node(x0: i32, x1: i32, top: i32, floor: bool) -> RouteNode {
        RouteNode { x0, x1, top, floor }
    }

    #[test]
    fn the_motions_are_the_shipped_ones() {
        let (carl, base, boosted) = (
            motion_of(ReachHero::Carl),
            motion_of(ReachHero::DonutBase),
            motion_of(ReachHero::DonutBoosted),
        );
        assert_eq!(carl, HeroMotion::CARL);
        assert_eq!(base, agility::agile_motion());
        assert_eq!(base.run_speed, boosted.run_speed);
        assert!(
            base.jump_speed < boosted.jump_speed,
            "the boost launches higher"
        );
        assert_eq!(
            boosted.jump_speed,
            f32::from(DonutProgTuning::active().dpq14_boost_launch_px_s)
        );
    }

    /// K8: the shipped Boiler room. Every class and every destruction state:
    /// each edge the validator accepted lands, and the exit is reached over the
    /// witnessed edges and by an actual walk.
    #[test]
    fn every_accepted_edge_of_the_boiler_is_traversable_and_the_exit_is_reached() {
        let run = launch(&root()).unwrap();
        let report = check(&run.kit, &run.loaded).unwrap();
        println!("K8 {}", report.summary());
        let bad = report.untraversable();
        assert!(bad.is_empty(), "validator false positives: {bad:#?}");
        let exits = report.exit_failures();
        assert!(exits.is_empty(), "exit not reached: {exits:#?}");
        assert!(report.is_clean());
        // 3 classes x 4 destruction states (the weight's remnant, the drum).
        assert_eq!(report.exits.len(), 12);
        for hero in ReachHero::ALL {
            let edges: Vec<_> = report.edges.iter().filter(|e| e.hero == hero).collect();
            assert!(
                edges.iter().any(|e| !e.from.floor && e.to.floor),
                "{hero:?}: a drop edge was replayed"
            );
            assert!(
                edges.iter().any(|e| e.from.floor && !e.to.floor),
                "{hero:?}: a climb edge was replayed"
            );
        }
    }

    /// The negative control of the check itself: a ceiling too low for the head
    /// on the perch is **accepted by the validator** (it knows classes, not
    /// ceilings) and refused by the integrator.
    #[test]
    fn a_low_ceiling_over_the_perch_is_caught_by_the_integrator() {
        let run = launch(&root()).unwrap();
        let mut kit = run.kit.clone();
        // Underside 130 px above the walking line: a body standing on the 80 px
        // perch (head at 132) cannot stand under it.
        kit.solid_rects.push(IRect {
            x: 0,
            y: kit.base_walk_line_y - 130 - 16,
            w: kit.width,
            h: 16,
        });
        let report = check(&kit, &run.loaded).unwrap();
        let bad = report.untraversable();
        assert!(!bad.is_empty(), "the low ceiling closes the perch");
        assert!(
            bad.iter()
                .any(|e| e.hero == ReachHero::Carl && e.from.floor && !e.to.floor),
            "Carl cannot climb to the perch any more: {bad:#?}"
        );
        // The lane under the perches is still walkable: the exit is reached.
        assert!(report.exits.iter().all(|e| e.walk_tick.is_some()));
    }

    /// A forged edge no body can make has no witness; a real one has.
    #[test]
    fn a_forged_edge_has_no_witness_and_a_real_one_does() {
        let run = launch(&root()).unwrap();
        let world = World::new(&run.kit, ReachHero::Carl, &[]);
        let room = &run.loaded.room;
        let ground = node(room.play_x.0, room.play_x.1, room.floor_y, true);
        // The room's first perch is 80 px up, Carl's class.
        let p = room.platforms[0].rect;
        let perch = node(p.x, p.right(), p.y, false);
        assert!(
            world.witness(&ground, &perch, 146.0).is_some(),
            "Carl climbs the real perch"
        );
        // The same span claimed 130 px up: no surface there, and out of reach.
        let phantom = node(p.x, p.right(), room.floor_y - 130, false);
        assert!(world.witness(&ground, &phantom, 146.0).is_none());
        // A real surface beyond Carl's jump: a 130 px slab added to the world.
        let high = LayoutRect::new(600, room.floor_y - 130, 144, 16);
        let high_world = World::new(&run.kit, ReachHero::Carl, &[high]);
        let high_node = node(600, 744, room.floor_y - 130, false);
        assert!(high_world.witness(&ground, &high_node, 146.0).is_none());
    }

    /// A missing ceiling slab is an error, not a pass.
    #[test]
    fn a_kit_without_the_declared_ceiling_is_an_error() {
        let run = launch(&root()).unwrap();
        let mut kit = run.kit.clone();
        let before = kit.solid_rects.len();
        let underside = kit.base_walk_line_y - run.loaded.ceiling_px.unwrap();
        kit.solid_rects.retain(|s| s.y + s.h != underside);
        assert_eq!(
            kit.solid_rects.len() + 1,
            before,
            "the ceiling slab removed"
        );
        let err = check(&kit, &run.loaded).expect_err("no ceiling slab");
        assert!(err.contains("ceiling_px"), "{err}");
    }

    #[test]
    fn the_exit_check_sees_a_walled_lane() {
        // A wall across the lane (a solid the walker cannot hop) is never reached.
        let run = launch(&root()).unwrap();
        let floor = run.loaded.room.floor_y;
        let wall = LayoutRect::new(600, floor - 200, 16, 200);
        let world = World::new(&run.kit, ReachHero::Carl, &[wall]);
        assert!(
            world
                .walk_to(
                    run.loaded.room.spawn_carl as f32,
                    floor,
                    run.loaded.room.exit
                )
                .is_none()
        );
        let open = World::new(&run.kit, ReachHero::Carl, &[]);
        assert!(
            open.walk_to(
                run.loaded.room.spawn_carl as f32,
                floor,
                run.loaded.room.exit
            )
            .is_some()
        );
    }
}
