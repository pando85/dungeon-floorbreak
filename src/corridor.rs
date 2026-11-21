//! Floor 1 corridor scene simulation (COMP-01): Carl on the kit's walking
//! line, a passive Donut companion and a scroll camera. Deterministic at a
//! fixed 60 Hz tick and independent of Bevy, rendering and lighting.
//!
//! Deliberately absent: enemies, bombs, health, melee. `attack` and `ability`
//! inputs are ignored here; the combat fixtures layer melee and goblins on top
//! in `combat_scene` without changing this simulation. Hero physics is the greybox's `step_hero`.

use std::collections::VecDeque;
use std::str::FromStr;

use crate::boss::BossThreat;
use crate::combat::{HERO_HEALTH, solid_at};
use crate::env_kit::{EnvKit, IRect, to_world_y};
use crate::follower::{Threat, follower_input_boss};
use crate::input::{Actions, InputFrame, RawInput};
use crate::party::{HeroId, PartyDrive, SwapPanMode, eased_pan_step};
use crate::prototype::{
    Block, BodyDrive, HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Hero, RUN_SPEED, Rect, TICK_SECONDS,
    step_hero_driven,
};
use crate::scroll_camera::{CameraConfig, DEADZONE_PX, MAX_SPEED_PX, ScrollCamera};

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

/// Donut as drawn and read by the HUD. In the legacy single-hero paths she is
/// a visual companion that replays Carl's foot path (no collision, physics or
/// combat). When the party is live ([`PartyDrive::live`]) her position comes
/// from her own hero body ([`CorridorWorld::donut_body`]) and `health` is the
/// authoritative value the combat damages.
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
    /// Donut's own hero body (the party's second hero): used only while
    /// [`Self::donut_live`]. Its `y` is the centre of the shared 24x52
    /// terrain collider; her hurtbox is separate (`PartyTuning::donut_hurtbox`).
    pub donut_body: Hero,
    /// The party is live: `donut` follows `donut_body`, not Carl's trail.
    pub donut_live: bool,
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
            donut_body: Hero::at(spawn.0 - DONUT_FOLLOW_PX, spawn.1),
            donut_live: false,
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
        self.donut_body = Hero::at(self.spawn.0 - DONUT_FOLLOW_PX, self.spawn.1);
        if self.donut_live {
            self.sync_donut_from_body();
        }
    }

    /// DEMO-01 checkpoint respawn: a full-health Carl standing at column `x` on
    /// the spawn walking line, Donut one leash behind him. Unlike
    /// [`Self::reset`] the tick counter and Carl's move-instance counter keep
    /// counting, so move instances (shore hits, launch ids) stay unique over a
    /// whole demo run. Only the demo calls this.
    pub fn respawn_at(&mut self, x: f32) {
        let mut melee = std::mem::take(&mut self.hero.melee);
        melee.cancel();
        self.hero = Hero::at(x, self.spawn.1);
        self.hero.melee = melee;
        self.complete = false;
        let foot_y = self.hero.y - HERO_HALF_HEIGHT;
        self.donut.health = DONUT_HEALTH;
        self.trail.clear();
        self.trail.push_back(TrailSample {
            x: x - DONUT_FOLLOW_PX,
            foot_y,
            arc: 0.0,
            facing: 1,
        });
        self.trail.push_back(TrailSample {
            x,
            foot_y,
            arc: DONUT_FOLLOW_PX,
            facing: 1,
        });
        self.update_donut();
        // A live party: Donut's own body stands up beside Carl, whole again.
        self.donut_body = Hero::at(x - DONUT_FOLLOW_PX, self.spawn.1);
        if self.donut_live {
            self.sync_donut_from_body();
        }
    }

    pub fn layout_height(&self) -> i32 {
        self.layout_height
    }

    /// y-up world height of the hero's foot line.
    pub fn hero_foot_y(&self) -> f32 {
        self.hero.y - HERO_HALF_HEIGHT
    }

    pub fn step(&mut self, input: &InputFrame) {
        self.step_driven(input, &BodyDrive::default());
    }

    /// [`Self::step`] with the overrides of a running combat move on Carl (CF02-H:
    /// the aerial moves and the bail-out hop). The default drive is the legacy step.
    pub fn step_driven(&mut self, input: &InputFrame, drive: &BodyDrive) {
        if self.complete {
            return;
        }
        self.tick += 1;
        step_hero_driven(&mut self.hero, input, &self.solids, self.max_x, drive);
        self.record_trail();
        self.update_donut();
        self.check_exit(false);
    }

    /// The exit is reached by Carl's box (and, with a live party, Donut's).
    fn check_exit(&mut self, include_donut: bool) {
        let Some(exit) = self.exit else {
            return;
        };
        let box_of = |hero: &Hero| Rect {
            left: hero.x - HERO_HALF_WIDTH,
            bottom: hero.y - HERO_HALF_HEIGHT,
            width: HERO_HALF_WIDTH * 2.0,
            height: HERO_HALF_HEIGHT * 2.0,
        };
        if overlaps(box_of(&self.hero), exit)
            || (include_donut && overlaps(box_of(&self.donut_body), exit))
        {
            self.complete = true;
        }
    }

    /// One tick of the live party's bodies: the tick counter advances, every
    /// hero that is not frozen (hit-stop) runs the shared `step_hero`, and
    /// Donut's drawn position follows her body. No trail is recorded.
    pub fn step_bodies(&mut self, frames: [&InputFrame; 2], frozen: [bool; 2]) {
        self.step_bodies_driven(frames, frozen, [BodyDrive::default(); 2]);
    }

    /// [`Self::step_bodies`] with the overrides of a running combat move per
    /// hero (Donut's pounce and swat). Default drives change nothing.
    pub fn step_bodies_driven(
        &mut self,
        frames: [&InputFrame; 2],
        frozen: [bool; 2],
        drives: [BodyDrive; 2],
    ) {
        if self.complete {
            return;
        }
        self.tick += 1;
        if !frozen[0] {
            step_hero_driven(
                &mut self.hero,
                frames[0],
                &self.solids,
                self.max_x,
                &drives[0],
            );
        }
        if !frozen[1] {
            step_hero_driven(
                &mut self.donut_body,
                frames[1],
                &self.solids,
                self.max_x,
                &drives[1],
            );
        }
        self.sync_donut_from_body();
        self.check_exit(true);
    }

    /// Test hook: re-derive Donut's drawn position after a test moved her body.
    #[doc(hidden)]
    pub fn sync_donut_for_tests(&mut self) {
        self.sync_donut_from_body();
    }

    fn sync_donut_from_body(&mut self) {
        // Live: the body is authoritative for health (the combat damages it);
        // the companion mirrors it for the HUD and the hash.
        self.donut = Companion {
            x: self.donut_body.x,
            foot_y: self.donut_body.y - HERO_HALF_HEIGHT,
            facing: self.donut_body.facing,
            health: self.donut_body.health,
        };
    }

    /// The hero body of `id` (Carl's is `hero`, Donut's `donut_body`).
    pub fn body(&self, id: HeroId) -> &Hero {
        match id {
            HeroId::Carl => &self.hero,
            HeroId::Donut => &self.donut_body,
        }
    }

    pub fn body_mut(&mut self, id: HeroId) -> &mut Hero {
        match id {
            HeroId::Carl => &mut self.hero,
            HeroId::Donut => &mut self.donut_body,
        }
    }

    /// y-up world height of `id`'s foot line.
    pub fn foot_y_of(&self, id: HeroId) -> f32 {
        self.body(id).y - HERO_HALF_HEIGHT
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
        // A live party mixes Donut's own body once, behind its own marker, so
        // no legacy hash (the story route, the fixtures) can ever change.
        if self.donut_live {
            mix(0x444f_4e55_5442_4f44);
            for value in [self.donut_body.vx, self.donut_body.vy] {
                mix(u64::from(value.to_bits()));
            }
            mix(u64::from(self.donut_body.grounded));
            mix(u64::from(self.donut.health));
            mix(u64::from(self.donut_body.invulnerable_ticks));
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
                ..RawInput::NONE
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
                    ..RawInput::NONE
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
    /// Carl's input frame (the legacy single hero).
    pub input: InputFrame,
    /// Donut's input frame (live party only).
    pub donut_input: InputFrame,
    camera_cfg: CameraConfig,
    /// What the party layer says this tick ([`PartyDrive::LEGACY`] until set).
    drive: PartyDrive,
    /// Q7: the temporary solo-swap pan cap is still active.
    pan_boost: bool,
    /// CF02-Q20: the eased swap pan's velocity of the previous tick, px (0 outside
    /// an eased pan; mixed into the hash only while non-zero).
    pan_v: i32,
    /// Q9: ticks each hero (`HeroId::index`) has been a bot and off-screen.
    bot_offscreen: [u16; 2],
    /// The layout is no wider than the view (the boss arena): the camera cannot
    /// scroll, so the 2P separation wall and the join pull are off. Derived
    /// from the kit, constant, not hashed.
    arena_locked: bool,
    /// PLAYTRACE-01 PT-C: the inputs the last live party step applied to each
    /// hero (after the bot decision, the combat gating and the 2P separation
    /// wall). A passive log: written in [`Self::step_live`], **never read by
    /// the simulation, never hashed** and not part of any golden.
    last_effective: [RawInput; 2],
    /// COMPANION-AI-01 01B-wire: the heroes whose off-screen regroup the
    /// companion's nav owns this tick (set by `combat_scene/companion.rs` before
    /// each step, `[false; 2]` otherwise). A passive control flag: **never
    /// hashed**, and with the companion off it is never set, so the legacy rule
    /// of [`SceneSim::regroup_bots`] runs unchanged.
    nav_regroup: [bool; 2],
}

/// What the combat layer tells a live party step about each hero.
#[derive(Debug, Clone, Copy)]
pub struct PartyStepCtx<'a> {
    /// The hero may move and jump this tick (false while hurt, defeated, or
    /// planted by a committed move). Human inputs arrive already gated; this
    /// also gates a bot.
    pub can_move: [bool; 2],
    /// The hero is frozen by hit-stop: its body does not step.
    pub frozen: [bool; 2],
    /// Goblin attacks in progress, for the bot's dodge.
    pub threats: &'a [Threat],
    /// HUD-DEBT-03: the boss' current threat, for the bot's dodges.
    pub boss: Option<BossThreat>,
    /// WORK-023: the running combat move's overrides per hero (Donut's pounce
    /// and swat). Default: nothing.
    pub drives: [BodyDrive; 2],
}

impl Default for PartyStepCtx<'_> {
    fn default() -> Self {
        Self {
            can_move: [true, true],
            frozen: [false, false],
            threats: &[],
            boss: None,
            drives: [BodyDrive::default(); 2],
        }
    }
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
            donut_input: InputFrame::default(),
            camera_cfg,
            drive: PartyDrive::LEGACY,
            pan_boost: false,
            pan_v: 0,
            bot_offscreen: [0, 0],
            arena_locked: kit.width <= view.w,
            last_effective: [RawInput::NONE; 2],
            nav_regroup: [false; 2],
        })
    }

    pub fn reset(&mut self) {
        self.world.reset();
        self.input = InputFrame::default();
        self.donut_input = InputFrame::default();
        self.pan_boost = false;
        self.pan_v = 0;
        self.bot_offscreen = [0, 0];
        self.camera = ScrollCamera::new(self.camera_cfg, self.world.hero.x, self.world.hero.facing);
    }

    /// DEMO-01: [`CorridorWorld::respawn_at`] plus a fresh input edge detector
    /// and a camera centred on the new column.
    pub fn respawn_at(&mut self, x: f32) {
        self.world.respawn_at(x);
        self.input = InputFrame::default();
        self.donut_input = InputFrame::default();
        self.pan_boost = false;
        self.pan_v = 0;
        self.bot_offscreen = [0, 0];
        self.camera = ScrollCamera::new(self.camera_cfg, x, self.world.hero.facing);
    }

    /// The party layer's view of this tick (`crate::party::PartyState::drive`).
    /// The first live drive hands Donut's drawn position to her own body; a
    /// change of the controlled hero in solo starts the swap pan (Q7). The
    /// legacy drive never goes live again once it was (a fixture is either one
    /// or the other).
    pub fn set_drive(&mut self, drive: PartyDrive) {
        if drive.live && !self.world.donut_live {
            self.world.donut_live = true;
            self.world.donut_body = Hero::at(
                self.world.donut.x,
                self.world.donut.foot_y + HERO_HALF_HEIGHT,
            );
            self.world.donut_body.facing = self.world.donut.facing;
            self.world.donut_body.health = self.world.donut.health;
        }
        if drive.live && !drive.two_player && drive.controlled != self.drive.controlled {
            self.pan_boost = self.drive.live;
            self.pan_v = 0;
        }
        let joins = drive.live && drive.two_player && !self.drive.two_player;
        self.drive = drive;
        if joins && !self.arena_locked {
            self.pull_second_player_into_view();
        }
    }

    /// S4: when a second player joins while the bot is farther away than the
    /// separation wall allows (it can wait at a gap for up to `regroup_ticks`),
    /// place Donut beside Carl at once, on the same free-spot rule as the bot
    /// regroup, so the first 2P view has both heroes on screen. A join inside
    /// the limit leaves her where she is.
    fn pull_second_player_into_view(&mut self) {
        let max = self.drive.tuning.max_separation_px(self.camera_cfg.view_w) as f32;
        let (carl_x, carl_y, carl_facing) = {
            let carl = &self.world.hero;
            (carl.x, carl.y, carl.facing)
        };
        if (self.world.donut_body.x - carl_x).abs() <= max {
            return;
        }
        let behind = if carl_facing >= 0 { -1.0 } else { 1.0 };
        let beside = carl_x + behind * f32::from(self.drive.tuning.follow_distance_px);
        let x = self
            .free_spot_x(beside, carl_y)
            .or_else(|| {
                self.free_spot_x(
                    carl_x - behind * f32::from(self.drive.tuning.follow_distance_px),
                    carl_y,
                )
            })
            .unwrap_or(carl_x);
        let body = &mut self.world.donut_body;
        body.x = x;
        body.y = carl_y;
        body.vx = 0.0;
        body.vy = 0.0;
        body.grounded = false;
        self.world.sync_donut_from_body();
        self.bot_offscreen = [0, 0];
    }

    /// `x` clamped to the level when a hero body centred at height `y` fits
    /// there without touching a solid.
    fn free_spot_x(&self, x: f32, y: f32) -> Option<f32> {
        let x = x.clamp(HERO_HALF_WIDTH, self.world.max_x);
        let body = Rect {
            left: x - HERO_HALF_WIDTH,
            bottom: y - HERO_HALF_HEIGHT,
            width: HERO_HALF_WIDTH * 2.0,
            height: HERO_HALF_HEIGHT * 2.0,
        };
        (!solid_at(&self.world.solids, body)).then_some(x)
    }

    pub fn drive(&self) -> &PartyDrive {
        &self.drive
    }

    /// PLAYTRACE-01 PT-C: the effective `[Carl, Donut]` inputs of the last
    /// **live party** step (`drive.live`): what each body actually received
    /// after the follower bot decided, the combat layer gated the human input
    /// and the 2P separation wall clamped it. `[RawInput::NONE; 2]` before the
    /// first live step; a non-live (legacy) step leaves it untouched, the
    /// recorder reads it only while the drive is live. Passive and unhashed.
    pub fn last_effective_inputs(&self) -> [RawInput; 2] {
        self.last_effective
    }

    /// True while the solo-swap pan still runs at the raised speed cap.
    pub fn pan_boosted(&self) -> bool {
        self.pan_boost
    }

    /// True while the party is live (Donut is a real hero).
    pub fn party_live(&self) -> bool {
        self.drive.live
    }

    /// Q7 / D1 M4: `x` is more than `margin` pixels outside the view.
    pub fn is_offscreen(&self, x: f32, margin: f32) -> bool {
        let left = self.camera.left() as f32;
        let right = left + self.camera_cfg.view_w as f32;
        x < left - margin || x > right + margin
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
    /// maps pads and the solo swap onto this order and clears `swap`. A hero the
    /// drive marks as a bot ignores its slot. With the legacy drive only
    /// `inputs[0]` is read and the behaviour is the old single-hero step.
    pub fn step_party(&mut self, inputs: [RawInput; 2]) {
        self.step_party_ctx(inputs, &PartyStepCtx::default());
    }

    /// [`Self::step_party`] with the combat layer's per-hero context.
    pub fn step_party_ctx(&mut self, inputs: [RawInput; 2], ctx: &PartyStepCtx) {
        if !self.drive.live {
            let raw = inputs[0];
            self.input.advance(raw);
            self.world.step_driven(&self.input, &ctx.drives[0]);
            self.camera.step(self.world.hero.x, self.world.hero.facing);
            return;
        }
        self.step_live(inputs, ctx);
    }

    fn step_live(&mut self, humans: [RawInput; 2], ctx: &PartyStepCtx) {
        let drive = self.drive;
        self.regroup_bots();
        self.nav_regroup = [false; 2];
        // Bots decide from the state before this tick; humans arrive gated.
        let mut raws = humans;
        for id in HeroId::ALL {
            let i = id.index();
            if !drive.is_bot(id) {
                continue;
            }
            raws[i] = if !ctx.can_move[i] {
                RawInput::default()
            } else if drive.companion_set(id).is_some() {
                // COMPANION-AI-01 (CA-24): the director's input arrives in
                // `humans[i]`, already gated like a human's, and it started
                // from the follow layer (`follow_input`, nav-driven where a
                // route needs a jump, 01B-wire). The follow layer is decided
                // before Carl's strike and the goblins' state of this tick; the
                // bot's is decided here, after them. What differed in 01A (a
                // regroup teleport) is now done before the layer is read
                // (`regroup_due`, `regroup_place`); what remains is a threat
                // that Carl's same-tick strike interrupts: the companion may
                // dodge it for that one tick, the bot does not.
                humans[i]
            } else {
                self.follow_input(id, ctx.threats, ctx.boss.as_ref())
            };
        }
        // A single locked screen (the boss arena) lets the heroes stand the
        // whole floor apart: the Q7 448 px leader clamp is suspended.
        if drive.two_player && !self.arena_locked {
            self.apply_separation_wall(&mut raws);
        }
        self.last_effective = raws;
        self.input.advance(raws[0]);
        self.donut_input.advance(raws[1]);
        self.world
            .step_bodies_driven([&self.input, &self.donut_input], ctx.frozen, ctx.drives);
        self.step_camera();
    }

    /// The follow-and-jump bot's input for `id` on the current (pre-tick)
    /// state: the one function `step_live` calls for a plain bot and the
    /// COMPANION view builder calls for `CompanionView.follow` (CA-24), so the
    /// companion's movement is the bot's by construction. The caller gates it
    /// with `can_move`.
    pub(crate) fn follow_input(
        &self,
        id: HeroId,
        threats: &[Threat],
        boss: Option<&BossThreat>,
    ) -> RawInput {
        let frame = if id == HeroId::Carl {
            &self.input
        } else {
            &self.donut_input
        };
        follower_input_boss(
            (id, self.world.body(id)),
            frame,
            self.world.body(id.other()),
            &self.world.solids,
            threats,
            boss,
            &self.drive.tuning,
        )
    }

    /// The dodge and boss-dodge reactions of the follow-and-jump bot for `id`
    /// with nothing to follow: the bot is asked with itself as the leader, so
    /// no steering, obstacle hop or ledge hop is left, only the M3 jump over a
    /// goblin tell and the boss pound and charge dodges. The nav-driven follow
    /// (01B-wire) lets these win over its own movement.
    pub(crate) fn dodge_input(
        &self,
        id: HeroId,
        threats: &[Threat],
        boss: Option<&BossThreat>,
    ) -> RawInput {
        let frame = if id == HeroId::Carl {
            &self.input
        } else {
            &self.donut_input
        };
        let me = self.world.body(id);
        follower_input_boss(
            (id, me),
            frame,
            me,
            &self.world.solids,
            threats,
            boss,
            &self.drive.tuning,
        )
    }

    /// 01B-wire: would [`Self::regroup_bots`] place bot `id` on this tick (it has
    /// been off-screen for `regroup_ticks - 1` ticks and still is)? The
    /// companion asks before the tick so it can place the hero (or refuse)
    /// before it reads the follow layer.
    pub(crate) fn regroup_due(&self, id: HeroId) -> bool {
        let body = self.world.body(id);
        self.drive.is_bot(id)
            && self.is_offscreen(body.x, HERO_HALF_WIDTH)
            && !self.world.complete
            && self.bot_offscreen[id.index()].saturating_add(1) >= self.drive.tuning.regroup_ticks
    }

    /// 01B-wire: the regroup of `id` was handled elsewhere without a placement
    /// (refused): start counting again.
    pub(crate) fn regroup_skip(&mut self, id: HeroId) {
        self.bot_offscreen[id.index()] = 0;
    }

    /// 01B-wire: place `id` standing on a surface (`x`, `foot_y`), at rest, when
    /// a body fits there. Returns whether it was placed.
    pub(crate) fn regroup_place(&mut self, id: HeroId, x: f32, foot_y: f32) -> bool {
        let y = foot_y + HERO_HALF_HEIGHT;
        let Some(x) = self.free_spot_x(x, y) else {
            return false;
        };
        let body = self.world.body_mut(id);
        body.x = x;
        body.y = y;
        body.vx = 0.0;
        body.vy = 0.0;
        body.grounded = false;
        self.bot_offscreen[id.index()] = 0;
        true
    }

    /// 01B-wire: which heroes' regroup the companion's nav owns this tick.
    pub(crate) fn set_nav_regroup(&mut self, owned: [bool; 2]) {
        self.nav_regroup = owned;
    }

    /// Q9: a bot that has been off-screen for `regroup_ticks` is placed just
    /// outside the view on the side behind the leader (so it walks back in),
    /// on the leader's foot line, or beside the leader when that spot is solid.
    fn regroup_bots(&mut self) {
        let drive = self.drive;
        for id in HeroId::ALL {
            let i = id.index();
            let body = self.world.body(id);
            let outside = drive.is_bot(id)
                && self.is_offscreen(body.x, HERO_HALF_WIDTH)
                && !self.world.complete;
            if !outside {
                self.bot_offscreen[i] = 0;
                continue;
            }
            self.bot_offscreen[i] = self.bot_offscreen[i].saturating_add(1);
            if self.bot_offscreen[i] < drive.tuning.regroup_ticks {
                continue;
            }
            self.bot_offscreen[i] = 0;
            if self.nav_regroup[i] {
                // 01B-wire: the companion's nav owns this hero's regroup (it
                // places it only where it could get on its own, or says why
                // not); the legacy placement on the leader's foot line, which
                // could cross a gate, does not run.
                continue;
            }
            let leader = self.world.body(id.other());
            let (leader_x, leader_y, leader_facing) = (leader.x, leader.y, leader.facing);
            let behind = if leader_facing >= 0 { -1.0 } else { 1.0 };
            let view_left = self.camera.left() as f32;
            let view_right = view_left + self.camera_cfg.view_w as f32;
            let edge = if behind < 0.0 {
                view_left - HERO_HALF_WIDTH - 2.0
            } else {
                view_right + HERO_HALF_WIDTH + 2.0
            };
            let beside = leader_x + behind * f32::from(drive.tuning.follow_distance_px);
            let x = self
                .free_spot_x(edge, leader_y)
                .or_else(|| self.free_spot_x(beside, leader_y))
                .unwrap_or(leader_x);
            let body = self.world.body_mut(id);
            body.x = x;
            body.y = leader_y;
            body.vx = 0.0;
            body.vy = 0.0;
            body.grounded = false;
        }
    }

    /// Q7 option A (two-player): the heroes may not drift farther apart than
    /// `view_w - 2 * camera_edge_margin_px`. A hero whose move this tick would
    /// take it past the limit, farther from the other, stands still instead:
    /// an invisible wall at the frame edge. A hero already beyond the limit
    /// (a swap, a join) may only move closer.
    fn apply_separation_wall(&self, raws: &mut [RawInput; 2]) {
        let max = self.drive.tuning.max_separation_px(self.camera_cfg.view_w) as f32;
        let step = |raw: &RawInput| Actions::from(*raw).move_x * RUN_SPEED * TICK_SECONDS;
        let mut next = [self.world.hero.x, self.world.donut_body.x];
        let before = next;
        for i in 0..2 {
            let planned = before[i] + step(&raws[i]);
            let other = next[1 - i];
            let (now, then) = ((before[i] - other).abs(), (planned - other).abs());
            if then > max && then > now {
                raws[i].move_x = 0.0;
                next[i] = before[i];
            } else {
                next[i] = planned;
            }
        }
    }

    /// Q7: solo follows the controlled hero (lead and deadzone as before); two
    /// players follow the midpoint (no look-ahead). A solo swap raises the speed
    /// cap to `swap_pan_px_per_tick` until the pan has caught up.
    fn step_camera(&mut self) {
        let world = &self.world;
        let (x, facing) = if self.drive.two_player {
            ((world.hero.x + world.donut_body.x) * 0.5, 0)
        } else {
            let target = world.body(self.drive.controlled);
            (target.x, target.facing)
        };
        // Two players: no deadzone, so the midpoint stays centred and the
        // leader at the wall keeps the full `camera_edge_margin_px` (Q7).
        let deadzone = if self.drive.two_player {
            0
        } else {
            DEADZONE_PX
        };
        if self.pan_boost && self.drive.tuning.swap_pan_mode == SwapPanMode::Eased {
            // CF02-Q20: eased in and out. A zero-cap step measures the distance
            // left without moving; the velocity ramps from the previous tick's.
            let remaining = self.camera.step_capped_deadzone(x, facing, 0, deadzone);
            let v = eased_pan_step(self.pan_v, remaining);
            self.pan_v = v;
            let left = self.camera.step_capped_deadzone(x, facing, v, deadzone);
            if left == 0 {
                self.pan_boost = false;
                self.pan_v = 0;
            }
            return;
        }
        let cap = if self.pan_boost {
            i32::from(self.drive.tuning.swap_pan_px_per_tick)
        } else {
            MAX_SPEED_PX
        };
        let remaining = self.camera.step_capped_deadzone(x, facing, cap, deadzone);
        if remaining <= MAX_SPEED_PX {
            self.pan_boost = false;
        }
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
        // Live-party state, mixed only once the party is live so every legacy
        // hash is unchanged.
        if self.world.donut_live {
            for value in [
                0x5041_5254_5953_494d,
                u64::from(self.donut_input.held.move_x.to_bits()),
                u64::from(self.donut_input.held.jump),
                u64::from(self.pan_boost),
                u64::from(self.bot_offscreen[0]),
                u64::from(self.bot_offscreen[1]),
                self.drive.controlled.index() as u64,
                u64::from(self.drive.two_player),
            ] {
                hash ^= value;
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
            // CF02-Q20: only while an eased pan runs, so legacy hashes hold.
            if self.pan_v != 0 {
                hash ^= 0x4346_3032_5041_4e56 ^ (self.pan_v as u64);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
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
    use crate::prototype::step_hero;
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
                ..RawInput::NONE
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

    /// PLAYTRACE-01 PT-C: the effective-input accessor is a passive log. A live
    /// party run with a bot Donut hashes exactly the same whatever the log
    /// holds (it is never read by the sim), and the log carries the bot's own
    /// decision, not the idle slot the caller passed.
    #[test]
    fn last_effective_inputs_is_a_passive_unhashed_log() {
        let drive = PartyDrive {
            live: true,
            two_player: false,
            controlled: HeroId::Carl,
            bots: [false, true],
            tuning: PartyDrive::LEGACY.tuning,
        };
        let mut plain = sim(360);
        let mut poked = sim(360);
        plain.set_drive(drive);
        poked.set_drive(drive);
        assert_eq!(plain.last_effective_inputs(), [RawInput::NONE; 2]);
        let mut bot_moved = false;
        for tick in 0..240u32 {
            let human = RawInput {
                move_x: 1.0,
                jump: tick % 50 == 7,
                ..RawInput::NONE
            };
            let junk = RawInput {
                move_x: -0.37,
                attack: true,
                swap: true,
                ..RawInput::NONE
            };
            // Whatever the log held before the step changes nothing.
            poked.last_effective = [junk, junk];
            plain.step_party([human, RawInput::default()]);
            poked.step_party([human, RawInput::default()]);
            assert_eq!(plain.state_hash(), poked.state_hash(), "tick {tick}");
            assert_eq!(plain.world.state_hash(), poked.world.state_hash());
            let eff = plain.last_effective_inputs();
            assert_eq!(eff, poked.last_effective_inputs());
            // The human's slot arrives unchanged; Donut's is the bot's decision
            // (the idle slot the caller passed is ignored for a bot).
            assert_eq!(eff[0], human);
            bot_moved |= eff[1].move_x != 0.0;
        }
        assert!(bot_moved, "the follower bot decided to move at some tick");
        // The legacy single-hero step never writes it.
        let mut legacy = sim(360);
        legacy.step(RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        });
        assert_eq!(legacy.last_effective_inputs(), [RawInput::NONE; 2]);
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
                ..RawInput::NONE
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
                ..RawInput::NONE
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
            ..RawInput::NONE
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
                ..RawInput::NONE
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
                ..RawInput::NONE
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
