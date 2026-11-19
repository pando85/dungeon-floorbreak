//! Floor 1 corridor scene simulation (COMP-01): Carl on the kit's walking
//! line, a passive Donut companion and a scroll camera. Deterministic at a
//! fixed 60 Hz tick and independent of Bevy, rendering and lighting.
//!
//! Deliberately absent: enemies, bombs, health, melee. `attack` and `ability`
//! inputs are ignored here; the combat fixtures layer melee and goblins on top
//! in `combat_scene` without changing this simulation. Hero physics is the greybox's `step_hero`.

use std::collections::VecDeque;
use std::str::FromStr;

use crate::combat::HERO_HEALTH;
use crate::env_kit::{EnvKit, IRect, to_world_y};
use crate::input::{InputFrame, RawInput};
use crate::prototype::{Block, HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Hero, Rect, step_hero};
use crate::scroll_camera::{CameraConfig, ScrollCamera};

/// How far behind Carl (path length, virtual px) Donut trails.
pub const DONUT_FOLLOW_PX: f32 = 32.0;
/// Safety cap on stored Donut trail samples (about 8 s of continuous motion
/// beyond the 32 px leash, which needs only a few dozen).
const MAX_TRAIL_SAMPLES: usize = 512;
/// Hard stop for scripted runs so a stuck scenario cannot loop forever.
pub const MAX_SCENARIO_TICKS: u64 = 20_000;

#[derive(Debug, Clone, Copy)]
struct TrailSample {
    x: f32,
    /// Foot line (y-up world height of the feet's bottom edge).
    foot_y: f32,
    /// Cumulative horizontal path length at this sample.
    arc: f32,
    facing: i8,
}

/// Donut's maximum health: the same as Carl's. HUD-HYBRID-A-01 gave her an
/// authoritative value so the HUD reads a real number; nothing damages her yet
/// (HUD-DEBT-01 adds the damage rules), so she stays at full health.
pub const DONUT_HEALTH: u8 = HERO_HEALTH;

/// Donut: a visual companion that replays Carl's foot path. She has no
/// collision, physics, combat or influence on Carl.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Companion {
    pub x: f32,
    pub foot_y: f32,
    pub facing: i8,
    /// Authoritative health, `0..=DONUT_HEALTH`. Full after every (re)start.
    pub health: u8,
}

#[derive(Debug, Clone)]
pub struct CorridorWorld {
    pub hero: Hero,
    pub donut: Companion,
    pub solids: Vec<Block>,
    pub tick: u64,
    pub complete: bool,
    layout_height: i32,
    max_x: f32,
    exit: Option<Rect>,
    spawn: (f32, f32),
    /// Foot line of the base corridor (y-up), the level the bot compares against.
    base_foot_y: f32,
    trail: VecDeque<TrailSample>,
}

impl CorridorWorld {
    pub fn new(kit: &EnvKit) -> Self {
        Self::with_spawn(kit, kit.spawn_centre(HERO_HALF_HEIGHT))
    }

    /// A world whose hero spawns (and restarts) standing on the walking line at
    /// column `x` (scenario fixtures start away from the story spawn). `None`
    /// when the column has no walking line.
    pub fn new_at(kit: &EnvKit, x: i32) -> Option<Self> {
        let line = kit.walking_line_at(x)?;
        let centre_y = to_world_y(kit.height, line) + HERO_HALF_HEIGHT;
        Some(Self::with_spawn(kit, (x as f32, centre_y)))
    }

    fn with_spawn(kit: &EnvKit, spawn: (f32, f32)) -> Self {
        let exit = kit.exit.map(|r| rect_to_world(kit.height, r));
        let mut world = Self {
            hero: Hero::at(spawn.0, spawn.1),
            donut: Companion {
                x: 0.0,
                foot_y: 0.0,
                facing: 1,
                health: DONUT_HEALTH,
            },
            solids: kit.blocks(),
            tick: 0,
            complete: false,
            layout_height: kit.height,
            max_x: kit.width as f32 - HERO_HALF_WIDTH,
            exit,
            spawn,
            base_foot_y: spawn.1 - HERO_HALF_HEIGHT,
            trail: VecDeque::new(),
        };
        world.reset_state();
        world
    }

    /// Back to the spawn (restart). Solids and layout are unchanged.
    pub fn reset(&mut self) {
        self.reset_state();
    }

    fn reset_state(&mut self) {
        self.hero = Hero::at(self.spawn.0, self.spawn.1);
        self.tick = 0;
        self.complete = false;
        let foot_y = self.hero.y - HERO_HALF_HEIGHT;
        // Donut starts one leash length behind the spawn point.
        self.donut.health = DONUT_HEALTH;
        self.trail.clear();
        self.trail.push_back(TrailSample {
            x: self.hero.x - DONUT_FOLLOW_PX,
            foot_y,
            arc: 0.0,
            facing: 1,
        });
        self.trail.push_back(TrailSample {
            x: self.hero.x,
            foot_y,
            arc: DONUT_FOLLOW_PX,
            facing: 1,
        });
        self.update_donut();
    }

    pub fn layout_height(&self) -> i32 {
        self.layout_height
    }

    /// y-up world height of the hero's foot line.
    pub fn hero_foot_y(&self) -> f32 {
        self.hero.y - HERO_HALF_HEIGHT
    }

    pub fn step(&mut self, input: &InputFrame) {
        if self.complete {
            return;
        }
        self.tick += 1;
        step_hero(&mut self.hero, input, &self.solids, self.max_x);
        self.record_trail();
        self.update_donut();
        if let Some(exit) = self.exit {
            let hero_box = Rect {
                left: self.hero.x - HERO_HALF_WIDTH,
                bottom: self.hero.y - HERO_HALF_HEIGHT,
                width: HERO_HALF_WIDTH * 2.0,
                height: HERO_HALF_HEIGHT * 2.0,
            };
            if overlaps(hero_box, exit) {
                self.complete = true;
            }
        }
    }

    fn record_trail(&mut self) {
        let last = *self.trail.back().expect("trail is never empty");
        let (x, foot_y) = (self.hero.x, self.hero_foot_y());
        if x == last.x && foot_y == last.foot_y {
            return;
        }
        let sample = TrailSample {
            x,
            foot_y,
            arc: last.arc + (x - last.x).abs(),
            facing: self.hero.facing,
        };
        // Vertical-only motion (jumping in place) adds no path length: keep a
        // single sample per arc position instead of growing the trail.
        let len = self.trail.len();
        if len >= 2 && self.trail[len - 2].arc == sample.arc && last.arc == sample.arc {
            *self.trail.back_mut().expect("trail is never empty") = sample;
        } else {
            self.trail.push_back(sample);
        }
        while self.trail.len() > MAX_TRAIL_SAMPLES {
            self.trail.pop_front();
        }
        // Keep a little more history than the leash needs.
        let newest = self.trail.back().map_or(0.0, |s| s.arc);
        while self.trail.len() > 2 && self.trail[1].arc < newest - DONUT_FOLLOW_PX - 8.0 {
            self.trail.pop_front();
        }
    }

    fn update_donut(&mut self) {
        let newest = self.trail.back().map_or(0.0, |s| s.arc);
        let target = newest - DONUT_FOLLOW_PX;
        let index = self.trail.partition_point(|s| s.arc <= target);
        let (x, foot_y, facing) = if index == 0 {
            let first = self.trail[0];
            (first.x, first.foot_y, first.facing)
        } else {
            let a = self.trail[index - 1];
            match self.trail.get(index) {
                Some(b) if b.arc > a.arc => {
                    let t = (target - a.arc) / (b.arc - a.arc);
                    (
                        a.x + (b.x - a.x) * t,
                        a.foot_y + (b.foot_y - a.foot_y) * t,
                        a.facing,
                    )
                }
                _ => (a.x, a.foot_y, a.facing),
            }
        };
        self.donut = Companion {
            x,
            foot_y,
            facing,
            health: self.donut.health,
        };
    }

    /// FNV-1a hash of every simulated value (stable across machines).
    pub fn state_hash(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        let mut mix = |value: u64| {
            for byte in value.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        mix(self.tick);
        mix(u64::from(self.complete));
        for value in [
            self.hero.x,
            self.hero.y,
            self.hero.vx,
            self.hero.vy,
            self.donut.x,
            self.donut.foot_y,
        ] {
            mix(u64::from(value.to_bits()));
        }
        mix(u64::from(self.hero.grounded));
        mix(self.hero.facing as u64);
        mix(self.donut.facing as u64);
        // Donut's health joins the hash only once it deviates from full, so
        // every hash recorded before she could be damaged stays valid. It never
        // deviates today (no damage source: HUD-DEBT-01).
        // No field may be appended after this conditional (HUD-DEBT-01 should
        // mix the health unconditionally and regenerate the goldens).
        if self.donut.health != DONUT_HEALTH {
            mix(u64::from(self.donut.health));
        }
        hash
    }
}

fn rect_to_world(layout_height: i32, r: IRect) -> Rect {
    Rect {
        left: r.x as f32,
        bottom: to_world_y(layout_height, r.bottom()),
        width: r.w as f32,
        height: r.h as f32,
    }
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.left < b.right() && a.right() > b.left && a.bottom < b.top() && a.top() > b.bottom
}

// ---------------------------------------------------------------- scenario --

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    /// No input at all.
    Idle,
    /// Hold right; jump whenever grounded against a step. Reaches the exit.
    WalkRight,
    /// `WalkRight` for [`TURN_AROUND_TICK`] ticks, then hold left (flip check).
    TurnAround,
    /// Animation review run: stand, run right (`WalkRight` jumps the two low
    /// steps), jump on the open floor while running, keep running (land into
    /// the run), stop, then jump in place and stand through the landing.
    /// Timings are the `RUN_JUMP_*` constants.
    RunJump,
}

/// Tick at which the `TurnAround` scenario reverses direction.
pub const TURN_AROUND_TICK: u64 = 150;

/// `RunJump` timings, in completed ticks (the input applies to the next step).
/// Stand until the run starts.
pub const RUN_JUMP_RUN_TICK: u64 = 30;
/// Jump while running on the open floor (after the two low steps near the
/// spawn, which `WalkRight`'s rule jumps over).
pub const RUN_JUMP_RUNNING_JUMP_TICK: u64 = 170;
/// Release the stick (stop running).
pub const RUN_JUMP_STOP_TICK: u64 = 260;
/// Jump in place after stopping.
pub const RUN_JUMP_STANDING_JUMP_TICK: u64 = 290;
/// Ticks that show the whole `RunJump` scenario including the final landing
/// (the evidence capture runs ticks 142, 145, ... 382).
pub const RUN_JUMP_TOTAL_TICKS: u64 = 382;

impl FromStr for Scenario {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, String> {
        match name {
            "idle" => Ok(Self::Idle),
            "walk_right" => Ok(Self::WalkRight),
            "turn_around" => Ok(Self::TurnAround),
            "run_jump" => Ok(Self::RunJump),
            other => Err(format!(
                "unknown scenario {other:?} (expected idle, walk_right, turn_around or run_jump)"
            )),
        }
    }
}

impl Scenario {
    pub fn input(self, world: &CorridorWorld) -> RawInput {
        match self {
            Self::Idle => RawInput::default(),
            Self::RunJump => {
                let tick = world.tick;
                let mut raw = if (RUN_JUMP_RUN_TICK..RUN_JUMP_STOP_TICK).contains(&tick) {
                    // The walk_right rule runs right and jumps the low steps.
                    Self::WalkRight.input(world)
                } else {
                    RawInput::default()
                };
                if tick == RUN_JUMP_RUNNING_JUMP_TICK || tick == RUN_JUMP_STANDING_JUMP_TICK {
                    raw.jump = true;
                }
                raw
            }
            Self::TurnAround if world.tick >= TURN_AROUND_TICK => RawInput {
                move_x: -1.0,
                ..RawInput::default()
            },
            Self::WalkRight | Self::TurnAround => {
                let hero = &world.hero;
                let foot_y = hero.y - HERO_HALF_HEIGHT;
                // A probe just ahead of the hero's leading edge, starting one
                // pixel above the feet: a step or wall in the way means jump.
                let probe = Rect {
                    left: hero.x + HERO_HALF_WIDTH,
                    bottom: foot_y + 1.0,
                    width: 6.0,
                    height: 8.0,
                };
                let blocked = world
                    .solids
                    .iter()
                    .any(|b| b.intact && overlaps(probe, b.rect));
                // Airborne above a raised platform: stop steering so the hero
                // drops onto it instead of flying over a short platform.
                let over_platform = !hero.grounded
                    && world.solids.iter().any(|b| {
                        b.rect.left <= hero.x
                            && hero.x < b.rect.right()
                            && b.rect.top() > world.base_foot_y + 0.5
                            && b.rect.top() <= foot_y + 0.5
                    });
                RawInput {
                    move_x: if over_platform { 0.0 } else { 1.0 },
                    jump: hero.grounded && blocked,
                    ..RawInput::default()
                }
            }
        }
    }
}

/// A corridor world, its camera and the edge-detecting input frame: the unit
/// the render binary and the headless smoke both drive.
#[derive(Debug, Clone)]
pub struct SceneSim {
    pub world: CorridorWorld,
    pub camera: ScrollCamera,
    pub input: InputFrame,
    camera_cfg: CameraConfig,
}

impl SceneSim {
    pub fn new(kit: &EnvKit, view_h: i32) -> Result<Self, String> {
        Self::with_world(kit, view_h, CorridorWorld::new(kit))
    }

    /// [`Self::new`] with the hero starting at column `x` (scenario fixtures).
    pub fn new_at(kit: &EnvKit, view_h: i32, x: i32) -> Result<Self, String> {
        let world = CorridorWorld::new_at(kit, x)
            .ok_or_else(|| format!("no walking line at column {x}"))?;
        Self::with_world(kit, view_h, world)
    }

    fn with_world(kit: &EnvKit, view_h: i32, world: CorridorWorld) -> Result<Self, String> {
        let name = if view_h == 400 {
            "abc_640x400"
        } else {
            "abc_640x360"
        };
        let view = kit
            .view(name)
            .ok_or_else(|| format!("layout has no view {name}"))?;
        let camera_cfg = CameraConfig {
            view_w: view.w,
            view_h: view.h,
            layout_w: kit.width,
            layout_h: kit.height,
            view_top: view.y,
        };
        let camera = ScrollCamera::new(camera_cfg, world.hero.x, world.hero.facing);
        Ok(Self {
            world,
            camera,
            input: InputFrame::default(),
            camera_cfg,
        })
    }

    pub fn reset(&mut self) {
        self.world.reset();
        self.input = InputFrame::default();
        self.camera = ScrollCamera::new(self.camera_cfg, self.world.hero.x, self.world.hero.facing);
    }

    /// A frozen tick (combat hit-stop): the tick counter and the input edge
    /// detector advance, nothing else moves. Hero, Donut's trail and the camera
    /// stand still. Only the combat fixtures call this; the story route never
    /// does, so its hashes are untouched.
    pub fn hold_tick(&mut self, raw: RawInput) {
        self.input.advance(raw);
        self.world.tick += 1;
    }

    /// The legacy single-hero step: Carl's slot gets `raw`, Donut's slot is
    /// idle. Delegates to [`Self::step_party`], so every golden and replay hash
    /// is unchanged.
    pub fn step(&mut self, raw: RawInput) {
        self.step_party([raw, RawInput::default()]);
    }

    /// One fixed tick for the party. `inputs` is indexed by hero, not by pad:
    /// `inputs[0]` drives Carl and `inputs[1]` drives Donut. `crate::party`
    /// maps pads and the solo swap onto this order and clears `swap`. The
    /// uncontrolled hero receives `RawInput::default()`.
    ///
    /// Contract stub (WORK-005/WORK-006): only `inputs[0]` is read until the
    /// sim lane gives Donut physics, so behaviour is identical to the old
    /// `step`.
    pub fn step_party(&mut self, inputs: [RawInput; 2]) {
        let raw = inputs[0];
        self.input.advance(raw);
        self.world.step(&self.input);
        self.camera.step(self.world.hero.x, self.world.hero.facing);
    }

    /// Pre-simulate `ticks` fixed ticks of a scripted scenario.
    pub fn run_scenario(&mut self, scenario: Scenario, ticks: u64) {
        self.run_scenario_observed(scenario, ticks, |_| {});
    }

    /// [`Self::run_scenario`], calling `observe` with the finished state after
    /// every tick (the presentation layer steps its animation there; the
    /// observer only reads).
    pub fn run_scenario_observed(
        &mut self,
        scenario: Scenario,
        ticks: u64,
        mut observe: impl FnMut(&SceneSim),
    ) {
        for _ in 0..ticks.min(MAX_SCENARIO_TICKS) {
            let raw = scenario.input(&self.world);
            self.step(raw);
            observe(self);
        }
    }

    pub fn state_hash(&self) -> u64 {
        let mut hash = self.world.state_hash();
        for value in [self.camera.left(), self.camera.top()] {
            hash ^= u64::from(value as u32);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }

    /// The line CI compares between the headless smoke and the render run.
    pub fn state_line(&self) -> String {
        format!(
            "FLOOR1_SCENE_STATE tick={} hero={:.3},{:.3} cam={},{} hash={:016x}",
            self.world.tick,
            self.world.hero.x,
            self.world.hero.y,
            self.camera.left(),
            self.camera.top(),
            self.state_hash()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

    fn kit() -> EnvKit {
        EnvKit::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")), LAYOUT).unwrap()
    }

    fn sim(view_h: i32) -> SceneSim {
        SceneSim::new(&kit(), view_h).unwrap()
    }

    /// Layout-space (y down) walking line under the hero's feet.
    fn feet_line(world: &CorridorWorld) -> f32 {
        world.layout_height() as f32 - world.hero_foot_y()
    }

    #[test]
    fn step_party_with_an_idle_second_slot_matches_the_legacy_step() {
        let mut legacy = sim(360);
        let mut party = sim(360);
        for tick in 0..120u32 {
            let raw = RawInput {
                move_x: 1.0,
                jump: tick % 40 == 5,
                ..RawInput::default()
            };
            legacy.step(raw);
            party.step_party([raw, RawInput::default()]);
            assert_eq!(legacy.state_hash(), party.state_hash());
        }
    }

    #[test]
    fn hero_spawns_on_the_walking_line_with_the_marker_contract() {
        let kit = kit();
        let world = CorridorWorld::new(&kit);
        assert_eq!(world.hero.x, 72.0);
        // Marker y 307 is the foot row, so the feet end at 308 = walking line.
        assert_eq!(feet_line(&world), 308.0);
        assert_eq!(kit.walking_line_at(72), Some(308));
        assert!(world.hero.grounded);
        assert_eq!(world.donut.x, 40.0);
        assert_eq!(world.donut.foot_y, world.hero_foot_y());
    }

    #[test]
    fn idle_for_600_ticks_stays_grounded_on_the_walking_line() {
        let mut sim = sim(360);
        let start_hash = sim.world.state_hash();
        sim.run_scenario(Scenario::Idle, 600);
        assert_eq!(sim.world.tick, 600);
        assert!(sim.world.hero.grounded);
        assert_eq!(sim.world.hero.x, 72.0);
        assert_eq!(feet_line(&sim.world), 308.0);
        assert_eq!(sim.world.hero.vy, 0.0);
        assert_ne!(
            sim.world.state_hash(),
            start_hash,
            "tick is part of the hash"
        );
        assert_eq!(sim.world.donut.x, 40.0);
    }

    #[test]
    fn attack_and_ability_inputs_do_nothing() {
        let mut a = sim(360);
        let mut b = sim(360);
        for tick in 0..120 {
            let base = Scenario::WalkRight.input(&a.world);
            a.step(base);
            b.step(RawInput {
                attack: tick % 3 == 0,
                ability: tick % 5 == 0,
                pause: tick % 7 == 0,
                ..base
            });
        }
        assert_eq!(a.world.hero.x.to_bits(), b.world.hero.x.to_bits());
        assert_eq!(a.world.state_hash(), b.world.state_hash());
    }

    #[test]
    fn holding_right_with_the_step_bot_reaches_the_exit() {
        let mut sim = sim(360);
        let mut max_height = 0.0_f32;
        let mut grounded_heights = Vec::new();
        let mut ticks = 0;
        while !sim.world.complete && ticks < 2400 {
            let raw = Scenario::WalkRight.input(&sim.world);
            sim.step(raw);
            ticks += 1;
            max_height = max_height.max(sim.world.hero_foot_y());
            if sim.world.hero.grounded {
                let line = feet_line(&sim.world);
                if grounded_heights.last() != Some(&line) {
                    grounded_heights.push(line);
                }
            }
        }
        assert!(sim.world.complete, "exit not reached in {ticks} ticks");
        assert!(sim.world.hero.x >= 1700.0, "x = {}", sim.world.hero.x);
        // Visited the base line (308) and raised platforms (292 and 276).
        assert!(grounded_heights.contains(&308.0));
        assert!(grounded_heights.contains(&292.0), "{grounded_heights:?}");
        assert!(grounded_heights.contains(&276.0), "{grounded_heights:?}");
        assert!(max_height > 92.0 + 16.0, "the hero jumped");
        // Every grounded height is a walking line of the layout.
        assert!(
            grounded_heights
                .iter()
                .all(|h| [308.0, 292.0, 276.0].contains(h)),
            "{grounded_heights:?}"
        );
    }

    #[test]
    fn no_tunnelling_walls_floors_or_bounds_during_a_full_run() {
        let kit = kit();
        let mut sim = SceneSim::new(&kit, 360).unwrap();
        for _ in 0..2400 {
            if sim.world.complete {
                break;
            }
            let raw = Scenario::WalkRight.input(&sim.world);
            sim.step(raw);
            let hero = &sim.world.hero;
            let body = Rect {
                left: hero.x - HERO_HALF_WIDTH,
                bottom: hero.y - HERO_HALF_HEIGHT,
                width: HERO_HALF_WIDTH * 2.0,
                height: HERO_HALF_HEIGHT * 2.0,
            };
            for block in &sim.world.solids {
                assert!(
                    !overlaps(body, block.rect),
                    "tick {}: hero {body:?} inside {:?}",
                    sim.world.tick,
                    block.rect
                );
            }
            assert!(hero.x >= HERO_HALF_WIDTH && hero.x <= 2048.0 - HERO_HALF_WIDTH);
            assert!(hero.y - HERO_HALF_HEIGHT >= 0.0);
        }
    }

    #[test]
    fn bounds_clamp_at_both_corridor_ends() {
        let mut sim = sim(360);
        for _ in 0..400 {
            sim.step(RawInput {
                move_x: -1.0,
                ..Default::default()
            });
        }
        // The left end wall (one tile) stops the hero.
        assert!(sim.world.hero.x >= 16.0 + HERO_HALF_WIDTH - 0.01);
        // The exit is reachable but the right layout bound still holds.
        let mut world = CorridorWorld::new(&kit());
        world.hero.x = 2030.0;
        world.complete = false;
        world.exit = None;
        let mut frame = InputFrame::default();
        for _ in 0..60 {
            frame.advance(RawInput {
                move_x: 1.0,
                ..Default::default()
            });
            world.step(&frame);
        }
        assert!(world.hero.x <= 2048.0 - HERO_HALF_WIDTH);
    }

    #[test]
    fn jumping_straight_up_lands_back_on_the_walking_line() {
        let mut sim = sim(360);
        sim.step(RawInput {
            jump: true,
            ..Default::default()
        });
        assert!(!sim.world.hero.grounded);
        for _ in 0..90 {
            sim.step(RawInput::default());
        }
        assert!(sim.world.hero.grounded);
        assert_eq!(feet_line(&sim.world), 308.0);
    }

    #[test]
    fn donut_has_authoritative_full_health_that_never_moves_and_resets() {
        let mut sim = sim(360);
        assert_eq!(sim.world.donut.health, DONUT_HEALTH);
        assert_eq!(DONUT_HEALTH, HERO_HEALTH);
        sim.run_scenario(Scenario::WalkRight, 300);
        assert_eq!(sim.world.donut.health, DONUT_HEALTH);
        // Full health leaves the hash exactly as it was before the field
        // existed; a damaged Donut would change it (future-proofing).
        let full = sim.world.state_hash();
        sim.world.donut.health = DONUT_HEALTH - 1;
        assert_ne!(sim.world.state_hash(), full);
        sim.world.donut.health = DONUT_HEALTH;
        assert_eq!(sim.world.state_hash(), full);
        sim.world.donut.health = 1;
        sim.reset();
        assert_eq!(sim.world.donut.health, DONUT_HEALTH);
    }

    #[test]
    fn replay_state_hash_is_golden_and_repeatable() {
        let run = |ticks| {
            let mut sim = sim(360);
            sim.run_scenario(Scenario::WalkRight, ticks);
            (sim.state_hash(), sim.state_line())
        };
        assert_eq!(run(400), run(400));
        assert_ne!(run(400).0, run(401).0);
        // Golden: update only with an intentional gameplay/physics change.
        let (hash, line) = run(400);
        eprintln!("{line}");
        assert_eq!(hash, GOLDEN_WALK_RIGHT_400, "{line}");
    }

    const GOLDEN_WALK_RIGHT_400: u64 = 0xee3b_82b8_3cd2_0207;

    #[test]
    fn deck_and_desktop_simulations_are_identical() {
        // Only the camera framing differs between 640x360 and 640x400.
        let mut a = sim(360);
        let mut b = sim(400);
        a.run_scenario(Scenario::WalkRight, 700);
        b.run_scenario(Scenario::WalkRight, 700);
        assert_eq!(a.world.state_hash(), b.world.state_hash());
        assert_eq!(a.camera.left(), b.camera.left());
        assert_eq!((a.camera.top(), b.camera.top()), (20, 0));
    }

    #[test]
    fn donut_trails_32_px_behind_on_the_same_walking_line() {
        let mut sim = sim(360);
        for _ in 0..90 {
            sim.step(RawInput {
                move_x: 1.0,
                ..Default::default()
            });
        }
        let gap = sim.world.hero.x - sim.world.donut.x;
        assert!((gap - DONUT_FOLLOW_PX).abs() < 0.01, "gap {gap}");
        assert_eq!(sim.world.donut.foot_y, sim.world.hero_foot_y());
        assert_eq!(sim.world.donut.facing, 1);
        // Carl stops: Donut stops too, 32 px behind, and does not walk through him.
        for _ in 0..120 {
            sim.step(RawInput::default());
        }
        let gap = sim.world.hero.x - sim.world.donut.x;
        assert!((gap - DONUT_FOLLOW_PX).abs() < 0.01, "gap {gap}");
    }

    #[test]
    fn donut_replays_carls_path_over_a_platform_and_is_deterministic() {
        let run = || {
            let mut sim = sim(360);
            let mut lines = Vec::new();
            for _ in 0..900 {
                let raw = Scenario::WalkRight.input(&sim.world);
                sim.step(raw);
                lines.push((
                    sim.world.donut.x.to_bits(),
                    sim.world.donut.foot_y.to_bits(),
                ));
            }
            (sim, lines)
        };
        let (a, la) = run();
        let (b, lb) = run();
        assert_eq!(la, lb);
        assert_eq!(a.world.state_hash(), b.world.state_hash());
        // After a long run Donut is on a valid walking line.
        let line = a.world.layout_height() as f32 - a.world.donut.foot_y;
        assert!(
            [308.0, 292.0, 276.0].contains(&line) || a.world.donut.foot_y > 92.0,
            "{line}"
        );
    }

    #[test]
    fn donut_trail_stays_bounded_when_carl_jumps_in_place_or_pushes_a_wall() {
        let mut sim = sim(360);
        for tick in 0..5000 {
            // Tap jump every 70 ticks, never moving horizontally.
            sim.step(RawInput {
                jump: tick % 70 < 2,
                ..Default::default()
            });
            assert!(
                sim.world.trail.len() <= 4,
                "trail grew to {}",
                sim.world.trail.len()
            );
        }
        // Walking the whole corridor and back also stays far below the cap.
        let longest = sim_with_longest_trail();
        assert!(longest <= 64, "longest trail {longest}");
    }

    fn sim_with_longest_trail() -> usize {
        let mut sim = sim(360);
        let mut longest = 0;
        for _ in 0..1000 {
            if sim.world.complete {
                break;
            }
            let raw = Scenario::WalkRight.input(&sim.world);
            sim.step(raw);
            longest = longest.max(sim.world.trail.len());
        }
        longest
    }

    #[test]
    fn donut_has_no_influence_on_carl() {
        // Carl's trajectory is identical to the bare greybox hero stepping
        // against the same solids, whatever Donut does.
        let kit = kit();
        let mut sim = SceneSim::new(&kit, 360).unwrap();
        let spawn = kit.spawn_centre(HERO_HALF_HEIGHT);
        let mut bare = Hero::at(spawn.0, spawn.1);
        let solids = kit.blocks();
        let mut frame = InputFrame::default();
        while !sim.world.complete {
            let raw = Scenario::WalkRight.input(&sim.world);
            sim.step(raw);
            frame.advance(raw);
            step_hero(
                &mut bare,
                &frame,
                &solids,
                kit.width as f32 - HERO_HALF_WIDTH,
            );
            assert_eq!(sim.world.hero.x.to_bits(), bare.x.to_bits());
            assert_eq!(sim.world.hero.y.to_bits(), bare.y.to_bits());
        }
    }

    #[test]
    fn camera_reaches_the_b_and_c_segments_during_the_scripted_run() {
        let mut sim = sim(360);
        let mut lefts = Vec::new();
        for _ in 0..1000 {
            let raw = Scenario::WalkRight.input(&sim.world);
            sim.step(raw);
            lefts.push(sim.camera.left());
        }
        // B1 is x 512..816 and C1 816..1280: the view must enter and pass both.
        assert!(lefts.iter().any(|&l| (512..816).contains(&(l + 320))));
        assert!(lefts.iter().any(|&l| (816..1280).contains(&(l + 320))));
        assert!(lefts.windows(2).all(|w| (w[1] - w[0]).abs() <= 4));
    }

    #[test]
    fn restart_returns_to_the_initial_state() {
        let mut sim = sim(360);
        let initial = sim.state_hash();
        sim.run_scenario(Scenario::WalkRight, 400);
        assert_ne!(sim.state_hash(), initial);
        sim.reset();
        assert_eq!(sim.state_hash(), initial);
    }

    #[test]
    fn turning_around_flips_both_actors_without_moving_carls_feet_sideways() {
        let mut sim = sim(360);
        sim.run_scenario(Scenario::TurnAround, TURN_AROUND_TICK);
        assert_eq!(sim.world.hero.facing, 1);
        let turn_x = sim.world.hero.x;
        sim.run_scenario(Scenario::TurnAround, 6);
        assert_eq!(sim.world.hero.facing, -1);
        assert!(sim.world.hero.x < turn_x);
        // Donut keeps walking along the recorded path, still facing right,
        // until she reaches the turn; she never influences Carl.
        assert_eq!(sim.world.donut.facing, 1);
        sim.run_scenario(Scenario::TurnAround, 120);
        assert_eq!(sim.world.donut.facing, -1);
        assert_eq!(feet_line(&sim.world), 308.0);
        assert_eq!(sim.world.donut.foot_y, sim.world.hero_foot_y());
    }

    #[test]
    fn scenario_names_parse() {
        assert_eq!("walk_right".parse::<Scenario>(), Ok(Scenario::WalkRight));
        assert_eq!("idle".parse::<Scenario>(), Ok(Scenario::Idle));
        assert_eq!("turn_around".parse::<Scenario>(), Ok(Scenario::TurnAround));
        assert!("fly".parse::<Scenario>().is_err());
    }
}
