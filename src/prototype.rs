//! Deterministic 60 Hz, IP-neutral linear combat greybox.
//! Gameplay state is independent of Bevy rendering and can run without a GPU.

use crate::combat::{
    Attacker, CombatEvent, GREYBOX_STRIKE, HERO_HEALTH, MoveRunner, resolve_strike,
};
pub use crate::combat::{ENEMY_HALF_HEIGHT, ENEMY_HALF_WIDTH, Enemy};
use crate::input::InputFrame;

pub const TICK_SECONDS: f32 = 1.0 / 60.0;
pub const STAGE_WIDTH: f32 = 2048.0;
pub const FLOOR_TOP: f32 = -120.0;
pub const HERO_HALF_WIDTH: f32 = 12.0;
pub const HERO_HALF_HEIGHT: f32 = 26.0;

pub const RUN_SPEED: f32 = 185.0;
pub const JUMP_SPEED: f32 = 440.0;
pub const GRAVITY: f32 = 1050.0;
const JUMP_BUFFER_TICKS: u8 = 7;
const COYOTE_TICKS: u8 = 6;
const BOMB_COOLDOWN: u16 = 135;
const BOMB_FUSE: u8 = 30;
const BOMB_RADIUS: f32 = 84.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub left: f32,
    pub bottom: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn right(self) -> f32 {
        self.left + self.width
    }

    pub fn top(self) -> f32 {
        self.bottom + self.height
    }

    pub fn center(self) -> (f32, f32) {
        (
            self.left + self.width * 0.5,
            self.bottom + self.height * 0.5,
        )
    }

    pub fn overlaps(self, other: Self) -> bool {
        self.left < other.right()
            && self.right() > other.left
            && self.bottom < other.top()
            && self.top() > other.bottom
    }

    fn squared_distance_to(self, x: f32, y: f32) -> f32 {
        let dx = (self.left - x).max(0.0).max(x - self.right());
        let dy = (self.bottom - y).max(0.0).max(y - self.top());
        dx * dx + dy * dy
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Material {
    Stone,
    Steel,
    Catwalk,
    Timber,
}

impl Material {
    pub fn destructible(self) -> bool {
        matches!(self, Self::Timber)
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub rect: Rect,
    pub material: Material,
    pub intact: bool,
}

#[derive(Debug, Clone)]
pub struct Hero {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub facing: i8,
    pub grounded: bool,
    pub health: u8,
    pub invulnerable_ticks: u8,
    /// Melee move clock (the WORK-021 strike is a 14-tick move, see
    /// [`crate::combat`]); the Floor 1 scene drives its own moves through it.
    pub melee: MoveRunner,
    pub bomb_cooldown: u16,
    coyote_ticks: u8,
    jump_buffer_ticks: u8,
}

impl Hero {
    fn new() -> Self {
        Self::at(86.0, FLOOR_TOP + HERO_HALF_HEIGHT)
    }

    /// Grounded, full-health hero whose AABB centre is `(x, y)`.
    pub fn at(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            facing: 1,
            grounded: true,
            health: HERO_HEALTH,
            invulnerable_ticks: 0,
            melee: MoveRunner::default(),
            bomb_cooldown: 0,
            coyote_ticks: COYOTE_TICKS,
            jump_buffer_ticks: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bomb {
    pub x: f32,
    pub y: f32,
    pub fuse: u8,
}

#[derive(Debug, Clone)]
pub struct Blast {
    pub x: f32,
    pub y: f32,
    pub remaining: u8,
}

#[derive(Debug, Clone)]
pub struct World {
    pub hero: Hero,
    pub blocks: Vec<Block>,
    pub enemies: Vec<Enemy>,
    pub bombs: Vec<Bomb>,
    pub blasts: Vec<Blast>,
    pub tick: u64,
    pub deaths: u32,
    pub complete: bool,
    /// Destructible blocks broken by the last [`World::step`] (presentation
    /// reads it for the wall-break sound; never used by the simulation).
    pub timbers_broken_last_step: u32,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        let mut blocks = vec![Block {
            rect: Rect {
                left: 0.0,
                bottom: -154.0,
                width: STAGE_WIDTH,
                height: 34.0,
            },
            material: Material::Stone,
            intact: true,
        }];

        // A bomb can remove the first barrier; indestructible masonry later
        // forces a short, forward-moving climb instead of free excavation.
        blocks.extend([
            Block {
                rect: Rect {
                    left: 610.0,
                    bottom: FLOOR_TOP,
                    width: 34.0,
                    height: 56.0,
                },
                material: Material::Timber,
                intact: true,
            },
            Block {
                rect: Rect {
                    left: 1100.0,
                    bottom: -56.0,
                    width: 122.0,
                    height: 16.0,
                },
                material: Material::Catwalk,
                intact: true,
            },
            Block {
                rect: Rect {
                    left: 1220.0,
                    bottom: 10.0,
                    width: 106.0,
                    height: 16.0,
                },
                material: Material::Catwalk,
                intact: true,
            },
            Block {
                rect: Rect {
                    left: 1340.0,
                    bottom: FLOOR_TOP,
                    width: 48.0,
                    height: 180.0,
                },
                material: Material::Steel,
                intact: true,
            },
            Block {
                rect: Rect {
                    left: 1400.0,
                    bottom: -46.0,
                    width: 115.0,
                    height: 16.0,
                },
                material: Material::Catwalk,
                intact: true,
            },
            Block {
                rect: Rect {
                    left: 1610.0,
                    bottom: FLOOR_TOP,
                    width: 34.0,
                    height: 48.0,
                },
                material: Material::Timber,
                intact: true,
            },
        ]);

        // Keep the very first screen safe until the player chooses to advance.
        let enemies = [350.0, 465.0, 765.0, 1010.0, 1540.0, 1790.0]
            .into_iter()
            .map(|x| Enemy::new(x, FLOOR_TOP + ENEMY_HALF_HEIGHT))
            .collect();

        Self {
            hero: Hero::new(),
            blocks,
            enemies,
            bombs: Vec::new(),
            blasts: Vec::new(),
            tick: 0,
            deaths: 0,
            complete: false,
            timbers_broken_last_step: 0,
        }
    }

    pub fn step(&mut self, input: &InputFrame) {
        if self.complete {
            return;
        }
        self.tick += 1;
        self.timbers_broken_last_step = 0;
        self.hero.invulnerable_ticks = self.hero.invulnerable_ticks.saturating_sub(1);
        self.hero.bomb_cooldown = self.hero.bomb_cooldown.saturating_sub(1);
        for blast in &mut self.blasts {
            blast.remaining = blast.remaining.saturating_sub(1);
        }
        self.blasts.retain(|blast| blast.remaining > 0);

        // The WORK-021 strike through the shared combat domain: a 14-tick move
        // that hits on its first tick, with no buffering and no reaction.
        let request = input.attack_pressed.then_some(&GREYBOX_STRIKE);
        if let Some(move_tick) = self.hero.melee.step(request, 0, true) {
            let attacker = Attacker {
                x: self.hero.x,
                y: self.hero.y,
                half_height: HERO_HALF_HEIGHT,
                facing: self.hero.facing,
                player: 0,
            };
            let mut events: Vec<CombatEvent> = Vec::new();
            resolve_strike(
                &mut self.hero.melee,
                &move_tick,
                attacker,
                &mut self.enemies,
                self.tick,
                &mut events,
            );
        }

        if input.ability_pressed && self.hero.bomb_cooldown == 0 {
            self.hero.bomb_cooldown = BOMB_COOLDOWN;
            self.bombs.push(Bomb {
                x: self.hero.x + self.hero.facing as f32 * 26.0,
                y: self.hero.y - 18.0,
                fuse: BOMB_FUSE,
            });
        }

        let mut detonations = Vec::new();
        for bomb in &mut self.bombs {
            bomb.fuse = bomb.fuse.saturating_sub(1);
            if bomb.fuse == 0 {
                detonations.push((bomb.x, bomb.y));
            }
        }
        self.bombs.retain(|bomb| bomb.fuse > 0);
        for (x, y) in detonations {
            self.explode(x, y);
        }

        self.move_hero(input);
        self.move_enemies();

        if self.hero.health == 0 {
            let deaths = self.deaths + 1;
            *self = Self::new();
            self.deaths = deaths;
        } else if self.hero.x > STAGE_WIDTH - 72.0 {
            self.complete = true;
        }
    }

    fn explode(&mut self, x: f32, y: f32) {
        self.blasts.push(Blast { x, y, remaining: 9 });
        for block in &mut self.blocks {
            if block.intact
                && block.material.destructible()
                && block.rect.squared_distance_to(x, y) <= BOMB_RADIUS * BOMB_RADIUS
            {
                block.intact = false;
                self.timbers_broken_last_step += 1;
            }
        }
        for enemy in &mut self.enemies {
            if enemy.health > 0 {
                let dx = enemy.x - x;
                let dy = enemy.y - y;
                if dx * dx + dy * dy <= BOMB_RADIUS * BOMB_RADIUS {
                    enemy.health = 0;
                }
            }
        }
    }

    fn move_hero(&mut self, input: &InputFrame) {
        step_hero(&mut self.hero, input, &self.blocks, STAGE_WIDTH);
    }

    fn move_enemies(&mut self) {
        // Deliberately simple ground chasers (shared `combat::Enemy`). They stop
        // at permanent geometry; they do not try to solve the raised route in
        // the first combat test.
        for enemy in &mut self.enemies {
            if !enemy.alive() {
                continue;
            }
            enemy.step(self.hero.x, &self.blocks);
            enemy.contact(
                self.hero.x,
                self.hero.y,
                &mut self.hero.health,
                &mut self.hero.invulnerable_ticks,
            );
        }
    }

    pub fn intact_timbers(&self) -> usize {
        self.blocks
            .iter()
            .filter(|block| block.intact && block.material.destructible())
            .count()
    }
}

/// One fixed tick of hero movement against static solids: jump buffer, coyote
/// time, horizontal sweep and gravity/landing. `max_x` is the largest centre x.
/// Shared by the combat greybox and the Floor 1 corridor scene.
pub fn step_hero(hero: &mut Hero, input: &InputFrame, blocks: &[Block], max_x: f32) {
    if input.jump_pressed {
        hero.jump_buffer_ticks = JUMP_BUFFER_TICKS;
    } else {
        hero.jump_buffer_ticks = hero.jump_buffer_ticks.saturating_sub(1);
    }
    if hero.grounded {
        hero.coyote_ticks = COYOTE_TICKS;
    } else {
        hero.coyote_ticks = hero.coyote_ticks.saturating_sub(1);
    }
    if hero.jump_buffer_ticks > 0 && hero.coyote_ticks > 0 {
        hero.vy = JUMP_SPEED;
        hero.grounded = false;
        hero.jump_buffer_ticks = 0;
        hero.coyote_ticks = 0;
    }

    hero.vx = input.held.move_x * RUN_SPEED;
    if hero.vx.abs() > 0.1 {
        hero.facing = if hero.vx > 0.0 { 1 } else { -1 };
    }
    let mut next_x = (hero.x + hero.vx * TICK_SECONDS).clamp(HERO_HALF_WIDTH, max_x);
    for block in blocks.iter().filter(|block| block.intact) {
        let candidate = Rect {
            left: next_x - HERO_HALF_WIDTH,
            bottom: hero.y - HERO_HALF_HEIGHT,
            width: HERO_HALF_WIDTH * 2.0,
            height: HERO_HALF_HEIGHT * 2.0,
        };
        if candidate.overlaps(block.rect) {
            if hero.vx > 0.0 {
                next_x = next_x.min(block.rect.left - HERO_HALF_WIDTH);
            } else if hero.vx < 0.0 {
                next_x = next_x.max(block.rect.right() + HERO_HALF_WIDTH);
            }
        }
    }
    hero.x = next_x;

    let previous_bottom = hero.y - HERO_HALF_HEIGHT;
    let previous_top = hero.y + HERO_HALF_HEIGHT;
    hero.vy -= GRAVITY * TICK_SECONDS;
    let mut next_y = hero.y + hero.vy * TICK_SECONDS;
    let mut grounded = false;
    for block in blocks.iter().filter(|block| block.intact) {
        let left = hero.x - HERO_HALF_WIDTH;
        let right = hero.x + HERO_HALF_WIDTH;
        if right <= block.rect.left || left >= block.rect.right() {
            continue;
        }
        let next_bottom = next_y - HERO_HALF_HEIGHT;
        let next_top = next_y + HERO_HALF_HEIGHT;
        if hero.vy <= 0.0
            && previous_bottom >= block.rect.top() - 0.01
            && next_bottom <= block.rect.top()
        {
            next_y = block.rect.top() + HERO_HALF_HEIGHT;
            hero.vy = 0.0;
            grounded = true;
        } else if hero.vy > 0.0
            && previous_top <= block.rect.bottom + 0.01
            && next_top >= block.rect.bottom
        {
            next_y = block.rect.bottom - HERO_HALF_HEIGHT;
            hero.vy = 0.0;
        }
    }
    hero.y = next_y;
    hero.grounded = grounded;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{InputFrame, RawInput};

    const GOLDEN2_HASH: u64 = 0x602a_e125_eed5_c95f;
    const GOLDEN2_GROUNDED_CHANGES: [u64; 6] = [30, 69, 85, 144, 145, 194];
    const GOLDEN_HASH: u64 = 0xd279_b98a_e4f7_a845;
    const GOLDEN_CHECKPOINTS: [(u32, u32); 5] = [
        (1_132_953_596, 3_256_833_366),
        (1_136_994_996, 3_239_706_626),
        (1_131_424_418, 3_239_706_626),
        (1_133_963_946, 3_239_706_626),
        (1_138_005_346, 3_254_495_916),
    ];

    fn press(world: &mut World, raw: RawInput) {
        let mut input = InputFrame::default();
        input.advance(raw);
        world.step(&input);
    }

    fn idle(world: &mut World, frames: usize) {
        for _ in 0..frames {
            press(world, RawInput::default());
        }
    }

    #[test]
    fn bomb_removes_timber_but_preserves_steel_and_stone() {
        let mut world = World::new();
        world.hero.x = 568.0;
        let before = world.intact_timbers();
        press(
            &mut world,
            RawInput {
                ability: true,
                ..Default::default()
            },
        );
        idle(&mut world, 30);
        assert_eq!(world.intact_timbers(), before - 1);
        assert!(
            world
                .blocks
                .iter()
                .any(|b| b.material == Material::Steel && b.intact)
        );
        assert!(
            world
                .blocks
                .iter()
                .any(|b| b.material == Material::Stone && b.intact)
        );
    }

    #[test]
    fn melee_damages_only_targets_in_front_and_respects_cooldown() {
        let mut world = World::new();
        world.enemies[0].x = 112.0;
        world.enemies[1].x = 53.0;
        press(
            &mut world,
            RawInput {
                attack: true,
                ..Default::default()
            },
        );
        assert_eq!(world.enemies[0].health, 1);
        assert_eq!(world.enemies[1].health, 2);
        press(&mut world, RawInput::default());
        press(
            &mut world,
            RawInput {
                attack: true,
                ..Default::default()
            },
        );
        assert_eq!(world.enemies[0].health, 1);
    }

    #[test]
    fn unbreakable_wall_blocks_ground_only_running() {
        let mut world = World::new();
        world.hero.x = 1300.0;
        let mut frame = InputFrame::default();
        for _ in 0..120 {
            frame.advance(RawInput {
                move_x: 1.0,
                ..Default::default()
            });
            world.step(&frame);
        }
        assert!(world.hero.x <= 1340.0 - HERO_HALF_WIDTH + 0.01);
        assert!(
            world
                .blocks
                .iter()
                .any(|b| b.material == Material::Steel && b.intact)
        );
    }

    #[test]
    fn jumping_is_required_to_reach_upper_catwalk() {
        let ground_jump_height = JUMP_SPEED * JUMP_SPEED / (2.0 * GRAVITY);
        assert!(ground_jump_height < 180.0);
        assert!(ground_jump_height > 80.0);
        let mut world = World::new();
        // Launch outside the platform, then enter it from above.
        world.hero.x = 1060.0;
        press(
            &mut world,
            RawInput {
                jump: true,
                ..Default::default()
            },
        );
        idle(&mut world, 19);
        for _ in 0..24 {
            press(
                &mut world,
                RawInput {
                    move_x: 1.0,
                    ..Default::default()
                },
            );
        }
        assert!(world.hero.grounded);
        assert!((world.hero.y - (-40.0 + HERO_HALF_HEIGHT)).abs() < 0.1);
    }

    #[test]
    fn fixed_step_worlds_produce_identical_results() {
        let mut a = World::new();
        let mut b = World::new();
        let mut fa = InputFrame::default();
        let mut fb = InputFrame::default();
        for tick in 0..180 {
            let raw = RawInput {
                move_x: if tick < 100 { 1.0 } else { 0.0 },
                jump: (38..44).contains(&tick),
                attack: (75..78).contains(&tick),
                ability: (115..118).contains(&tick),
                ..Default::default()
            };
            fa.advance(raw);
            fb.advance(raw);
            a.step(&fa);
            b.step(&fb);
        }
        assert_eq!(a.hero.x.to_bits(), b.hero.x.to_bits());
        assert_eq!(a.hero.y.to_bits(), b.hero.y.to_bits());
        assert_eq!(a.intact_timbers(), b.intact_timbers());
        assert_eq!(
            a.enemies.iter().map(|e| e.health).collect::<Vec<_>>(),
            b.enemies.iter().map(|e| e.health).collect::<Vec<_>>()
        );
    }

    /// Scripted 300-tick input exercising run, analogue run, jump buffer,
    /// coyote time, melee, bombs, enemy contact and turning around.
    fn golden_script(tick: u64) -> RawInput {
        let move_x = match tick {
            0..100 => 1.0,
            100..130 => 0.0,
            130..200 => -1.0,
            200..250 => 1.0,
            _ => 0.6,
        };
        RawInput {
            move_x,
            jump: (20..26).contains(&tick)
                || tick == 90
                || (150..153).contains(&tick)
                || (210..216).contains(&tick)
                || tick == 262,
            attack: (60..63).contains(&tick) || (170..173).contains(&tick),
            ability: (95..98).contains(&tick),
            ..Default::default()
        }
    }

    /// Second golden script: wall contact, climbing onto the timber barrier,
    /// a late (coyote) jump after walking off its far edge, a buffered jump
    /// just before landing, and a ceiling-free turn back.
    fn wall_ledge_script(tick: u64) -> RawInput {
        let move_x = match tick {
            0..30 => 1.0,
            30..34 => 0.0,
            34..43 => 1.0,
            43..70 => 0.0,
            70..160 => 1.0,
            _ => -1.0,
        };
        RawInput {
            move_x,
            jump: tick == 30 || tick == 89 || tick == 141,
            ..Default::default()
        }
    }

    fn fnv1a(hash: &mut u64, value: u64) {
        for byte in value.to_le_bytes() {
            *hash ^= u64::from(byte);
            *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    /// Regression lock: one scripted run (running, jumps, melee, a bomb, enemy
    /// contact, turning around) must reproduce bit for bit. It is a coarse net
    /// over the whole `World::step`, not a proof of every rule: the direct
    /// `step_hero` tests below pin coyote time, the jump buffer, wall and
    /// ceiling push-out and landing one by one.
    #[test]
    fn golden_trace_of_the_greybox_hero_is_bit_stable() {
        let mut world = World::new();
        let mut frame = InputFrame::default();
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        let mut checkpoints = Vec::new();
        for tick in 0..300 {
            frame.advance(golden_script(tick));
            world.step(&frame);
            for value in [
                world.hero.x.to_bits(),
                world.hero.y.to_bits(),
                world.hero.vx.to_bits(),
                world.hero.vy.to_bits(),
            ] {
                fnv1a(&mut hash, u64::from(value));
            }
            fnv1a(&mut hash, u64::from(world.hero.grounded));
            fnv1a(&mut hash, world.tick);
            fnv1a(&mut hash, u64::from(world.hero.health));
            if tick % 60 == 59 {
                checkpoints.push((world.hero.x.to_bits(), world.hero.y.to_bits()));
            }
        }
        assert_eq!(hash, GOLDEN_HASH);
        assert_eq!(checkpoints, GOLDEN_CHECKPOINTS);
    }

    // ---- direct `step_hero` tests on synthetic blocks ----------------------
    //
    // These pin the movement rules one at a time (coyote window, jump buffer,
    // wall push-out, ceiling bonk, landing) so a change to any single constant
    // or push-out distance fails a named test instead of hiding inside a hash.

    fn block(left: f32, bottom: f32, width: f32, height: f32) -> Block {
        Block {
            rect: Rect {
                left,
                bottom,
                width,
                height,
            },
            material: Material::Stone,
            intact: true,
        }
    }

    /// Drives `step_hero` with edge-detected input, like the real adapters.
    struct Rig {
        hero: Hero,
        frame: InputFrame,
        solids: Vec<Block>,
    }

    impl Rig {
        fn new(x: f32, y: f32, solids: Vec<Block>) -> Self {
            Self {
                hero: Hero::at(x, y),
                frame: InputFrame::default(),
                solids,
            }
        }

        fn tick(&mut self, move_x: f32, jump: bool) {
            self.frame.advance(RawInput {
                move_x,
                jump,
                ..Default::default()
            });
            step_hero(&mut self.hero, &self.frame, &self.solids, 10_000.0);
        }
    }

    /// Jump pressed on the `k`-th tick after the hero walked off a ledge
    /// (k = 1 is the first airborne tick). Returns whether the jump fired.
    fn ledge_jump_after(k: u32) -> bool {
        // Floor top at y = 0 for x in 0..100; nothing beyond the ledge.
        let mut rig = Rig::new(
            60.0,
            HERO_HALF_HEIGHT,
            vec![block(0.0, -100.0, 100.0, 100.0)],
        );
        while rig.hero.grounded {
            rig.tick(1.0, false);
        }
        // The tick that left the ledge was tick 0; the press is on tick k.
        for _ in 1..k {
            rig.tick(1.0, false);
        }
        rig.tick(1.0, true);
        // A fired jump sets vy = JUMP_SPEED before gravity: still rising.
        rig.hero.vy > 0.0
    }

    #[test]
    fn coyote_time_allows_a_late_jump_for_exactly_the_coyote_window() {
        // Literal numbers on purpose: the window is 6 ticks (0.1 s); a test
        // built from the constant would follow a mutated constant.
        for k in 1..=5 {
            assert!(
                ledge_jump_after(k),
                "jump {k} ticks after the ledge must fire"
            );
        }
        assert!(!ledge_jump_after(6), "the coyote window is over");
        assert!(!ledge_jump_after(9));
    }

    fn flat_rig() -> Rig {
        Rig::new(
            500.0,
            HERO_HALF_HEIGHT,
            vec![block(0.0, -100.0, 1000.0, 100.0)],
        )
    }

    /// Jump from flat ground; the tick (counted from the jump press, which is
    /// tick 0) on which the hero lands.
    fn landing_tick() -> u32 {
        let mut rig = flat_rig();
        rig.tick(0.0, true);
        for t in 1..200 {
            rig.tick(0.0, false);
            if rig.hero.grounded {
                return t;
            }
        }
        panic!("never landed");
    }

    /// Jump, press jump again `early` ticks before the landing tick, and
    /// report whether the buffered press produced a second jump.
    fn buffered_jump_fires(early: u32) -> bool {
        let landing = landing_tick();
        let mut rig = flat_rig();
        rig.tick(0.0, true);
        for t in 1..=landing + 2 {
            let press = t + early == landing;
            rig.tick(0.0, press);
            // The first jump is over by the landing: rising again afterwards
            // can only be the buffered jump.
            if t > landing && rig.hero.vy > 0.0 {
                return true;
            }
        }
        false
    }

    #[test]
    fn jump_buffer_remembers_a_press_made_shortly_before_landing() {
        // The buffer is 7 ticks: a press up to 5 ticks before the landing tick
        // still fires on the first grounded step; 6 or more ticks is forgotten.
        for early in 0..=5 {
            assert!(buffered_jump_fires(early), "press {early} ticks early");
        }
        assert!(!buffered_jump_fires(6), "6 ticks early is forgotten");
        assert!(!buffered_jump_fires(9));
    }

    #[test]
    fn walking_into_a_wall_stops_exactly_one_hero_half_width_away() {
        // Wall on the floor, to the right and then to the left of the hero.
        let floor = block(0.0, -100.0, 1000.0, 100.0);
        let mut right = Rig::new(
            100.0,
            HERO_HALF_HEIGHT,
            vec![floor.clone(), block(200.0, 0.0, 40.0, 100.0)],
        );
        for _ in 0..120 {
            right.tick(1.0, false);
        }
        assert_eq!(right.hero.x, 200.0 - HERO_HALF_WIDTH);
        assert!(right.hero.grounded);
        let mut left = Rig::new(
            400.0,
            HERO_HALF_HEIGHT,
            vec![floor, block(200.0, 0.0, 40.0, 100.0)],
        );
        for _ in 0..120 {
            left.tick(-1.0, false);
        }
        assert_eq!(left.hero.x, 240.0 + HERO_HALF_WIDTH);
    }

    #[test]
    fn jumping_into_a_ceiling_stops_the_head_exactly_at_its_underside() {
        let ceiling_bottom = 80.0;
        let mut rig = Rig::new(
            50.0,
            HERO_HALF_HEIGHT,
            vec![
                block(0.0, -100.0, 100.0, 100.0),
                block(0.0, ceiling_bottom, 100.0, 40.0),
            ],
        );
        rig.tick(0.0, true);
        let mut bonked = false;
        for _ in 0..40 {
            rig.tick(0.0, false);
            if rig.hero.vy == 0.0 && !rig.hero.grounded {
                assert_eq!(rig.hero.y, ceiling_bottom - HERO_HALF_HEIGHT);
                bonked = true;
                break;
            }
        }
        assert!(bonked, "the hero never touched the ceiling");
        // After the bonk gravity takes over and the hero lands again.
        for _ in 0..120 {
            rig.tick(0.0, false);
        }
        assert!(rig.hero.grounded);
        assert_eq!(rig.hero.y, HERO_HALF_HEIGHT);
    }

    #[test]
    fn landing_snaps_the_feet_exactly_onto_the_surface() {
        let mut rig = Rig::new(50.0, 200.0, vec![block(0.0, -100.0, 100.0, 100.0)]);
        for _ in 0..120 {
            rig.tick(0.0, false);
        }
        assert!(rig.hero.grounded);
        assert_eq!(rig.hero.y, HERO_HALF_HEIGHT);
        assert_eq!(rig.hero.vy, 0.0);
    }

    /// Second regression lock, aimed at rules the first script barely touches:
    /// wall contact, climbing onto the timber barrier, a coyote jump 4 ticks
    /// after walking off its far edge, a jump buffered 3 ticks before landing,
    /// and turning back. Same caveat: a coarse net, the direct tests pin rules.
    #[test]
    fn golden_trace_of_wall_ledge_coyote_and_buffer_is_bit_stable() {
        let mut world = World::new();
        world.hero.x = 560.0;
        let mut frame = InputFrame::default();
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        let mut grounded_changes = Vec::new();
        let mut previous = true;
        for tick in 0..200 {
            frame.advance(wall_ledge_script(tick));
            world.step(&frame);
            for value in [
                world.hero.x.to_bits(),
                world.hero.y.to_bits(),
                world.hero.vx.to_bits(),
                world.hero.vy.to_bits(),
            ] {
                fnv1a(&mut hash, u64::from(value));
            }
            fnv1a(&mut hash, u64::from(world.hero.grounded));
            fnv1a(&mut hash, u64::from(world.hero.health));
            if world.hero.grounded != previous {
                grounded_changes.push(tick);
                previous = world.hero.grounded;
            }
        }
        assert_eq!(hash, GOLDEN2_HASH);
        assert_eq!(grounded_changes, GOLDEN2_GROUNDED_CHANGES);
    }

    #[test]
    fn opening_corridor_is_safe_while_the_player_is_idle() {
        let mut world = World::new();
        idle(&mut world, 600);
        assert_eq!(world.hero.health, 4);
        assert_eq!(world.deaths, 0);
        assert_eq!(world.hero.x, 86.0);
    }

    #[test]
    fn death_restarts_the_short_stage() {
        let mut world = World::new();
        world.hero.health = 0;
        press(&mut world, RawInput::default());
        assert_eq!(world.deaths, 1);
        assert_eq!(world.hero.health, 4);
        assert_eq!(world.tick, 0);
    }
}
