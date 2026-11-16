//! Deterministic 60 Hz, IP-neutral linear combat greybox.
//! Gameplay state is independent of Bevy rendering and can run without a GPU.

use crate::input::InputFrame;

pub const TICK_SECONDS: f32 = 1.0 / 60.0;
pub const STAGE_WIDTH: f32 = 2048.0;
pub const FLOOR_TOP: f32 = -120.0;
pub const HERO_HALF_WIDTH: f32 = 12.0;
pub const HERO_HALF_HEIGHT: f32 = 26.0;
pub const ENEMY_HALF_WIDTH: f32 = 12.0;
pub const ENEMY_HALF_HEIGHT: f32 = 19.0;

const RUN_SPEED: f32 = 185.0;
const JUMP_SPEED: f32 = 440.0;
const GRAVITY: f32 = 1050.0;
const JUMP_BUFFER_TICKS: u8 = 7;
const COYOTE_TICKS: u8 = 6;
const MELEE_COOLDOWN: u8 = 14;
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

    fn overlaps(self, other: Self) -> bool {
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
    pub melee_cooldown: u8,
    pub bomb_cooldown: u16,
    coyote_ticks: u8,
    jump_buffer_ticks: u8,
}

impl Hero {
    fn new() -> Self {
        Self {
            x: 86.0,
            y: FLOOR_TOP + HERO_HALF_HEIGHT,
            vx: 0.0,
            vy: 0.0,
            facing: 1,
            grounded: true,
            health: 4,
            invulnerable_ticks: 0,
            melee_cooldown: 0,
            bomb_cooldown: 0,
            coyote_ticks: COYOTE_TICKS,
            jump_buffer_ticks: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Enemy {
    pub x: f32,
    pub y: f32,
    pub health: u8,
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
            .map(|x| Enemy {
                x,
                y: FLOOR_TOP + ENEMY_HALF_HEIGHT,
                health: 2,
            })
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
        }
    }

    pub fn step(&mut self, input: &InputFrame) {
        if self.complete {
            return;
        }
        self.tick += 1;
        self.hero.invulnerable_ticks = self.hero.invulnerable_ticks.saturating_sub(1);
        self.hero.melee_cooldown = self.hero.melee_cooldown.saturating_sub(1);
        self.hero.bomb_cooldown = self.hero.bomb_cooldown.saturating_sub(1);
        for blast in &mut self.blasts {
            blast.remaining = blast.remaining.saturating_sub(1);
        }
        self.blasts.retain(|blast| blast.remaining > 0);

        if input.attack_pressed && self.hero.melee_cooldown == 0 {
            self.hero.melee_cooldown = MELEE_COOLDOWN;
            let facing = self.hero.facing as f32;
            for enemy in &mut self.enemies {
                let forward = (enemy.x - self.hero.x) * facing;
                if enemy.health > 0
                    && (-9.0..=54.0).contains(&forward)
                    && (enemy.y - self.hero.y).abs() <= 44.0
                {
                    enemy.health -= 1;
                }
            }
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

    fn solid_at(&self, candidate: Rect) -> bool {
        self.blocks
            .iter()
            .any(|b| b.intact && candidate.overlaps(b.rect))
    }

    fn move_hero(&mut self, input: &InputFrame) {
        if input.jump_pressed {
            self.hero.jump_buffer_ticks = JUMP_BUFFER_TICKS;
        } else {
            self.hero.jump_buffer_ticks = self.hero.jump_buffer_ticks.saturating_sub(1);
        }
        if self.hero.grounded {
            self.hero.coyote_ticks = COYOTE_TICKS;
        } else {
            self.hero.coyote_ticks = self.hero.coyote_ticks.saturating_sub(1);
        }
        if self.hero.jump_buffer_ticks > 0 && self.hero.coyote_ticks > 0 {
            self.hero.vy = JUMP_SPEED;
            self.hero.grounded = false;
            self.hero.jump_buffer_ticks = 0;
            self.hero.coyote_ticks = 0;
        }

        self.hero.vx = input.held.move_x * RUN_SPEED;
        if self.hero.vx.abs() > 0.1 {
            self.hero.facing = if self.hero.vx > 0.0 { 1 } else { -1 };
        }
        let mut next_x =
            (self.hero.x + self.hero.vx * TICK_SECONDS).clamp(HERO_HALF_WIDTH, STAGE_WIDTH);
        for block in self.blocks.iter().filter(|block| block.intact) {
            let candidate = Rect {
                left: next_x - HERO_HALF_WIDTH,
                bottom: self.hero.y - HERO_HALF_HEIGHT,
                width: HERO_HALF_WIDTH * 2.0,
                height: HERO_HALF_HEIGHT * 2.0,
            };
            if candidate.overlaps(block.rect) {
                if self.hero.vx > 0.0 {
                    next_x = next_x.min(block.rect.left - HERO_HALF_WIDTH);
                } else if self.hero.vx < 0.0 {
                    next_x = next_x.max(block.rect.right() + HERO_HALF_WIDTH);
                }
            }
        }
        self.hero.x = next_x;

        let previous_bottom = self.hero.y - HERO_HALF_HEIGHT;
        let previous_top = self.hero.y + HERO_HALF_HEIGHT;
        self.hero.vy -= GRAVITY * TICK_SECONDS;
        let mut next_y = self.hero.y + self.hero.vy * TICK_SECONDS;
        let mut grounded = false;
        for block in self.blocks.iter().filter(|block| block.intact) {
            let left = self.hero.x - HERO_HALF_WIDTH;
            let right = self.hero.x + HERO_HALF_WIDTH;
            if right <= block.rect.left || left >= block.rect.right() {
                continue;
            }
            let next_bottom = next_y - HERO_HALF_HEIGHT;
            let next_top = next_y + HERO_HALF_HEIGHT;
            if self.hero.vy <= 0.0
                && previous_bottom >= block.rect.top() - 0.01
                && next_bottom <= block.rect.top()
            {
                next_y = block.rect.top() + HERO_HALF_HEIGHT;
                self.hero.vy = 0.0;
                grounded = true;
            } else if self.hero.vy > 0.0
                && previous_top <= block.rect.bottom + 0.01
                && next_top >= block.rect.bottom
            {
                next_y = block.rect.bottom - HERO_HALF_HEIGHT;
                self.hero.vy = 0.0;
            }
        }
        self.hero.y = next_y;
        self.hero.grounded = grounded;
    }

    fn move_enemies(&mut self) {
        // Deliberately simple ground chasers. They stop at permanent geometry;
        // they do not try to solve the raised route in the first combat test.
        for index in 0..self.enemies.len() {
            if self.enemies[index].health == 0 {
                continue;
            }
            let dx = self.hero.x - self.enemies[index].x;
            if dx.abs() < 220.0 && dx.abs() > 22.0 {
                let candidate_x = self.enemies[index].x + dx.signum() * 62.0 * TICK_SECONDS;
                let candidate = Rect {
                    left: candidate_x - ENEMY_HALF_WIDTH,
                    bottom: self.enemies[index].y - ENEMY_HALF_HEIGHT,
                    width: ENEMY_HALF_WIDTH * 2.0,
                    height: ENEMY_HALF_HEIGHT * 2.0,
                };
                if !self.solid_at(candidate) {
                    self.enemies[index].x = candidate_x;
                }
            }
            let enemy = &self.enemies[index];
            if self.hero.invulnerable_ticks == 0
                && (enemy.x - self.hero.x).abs() < 23.0
                && (enemy.y - self.hero.y).abs() < 36.0
            {
                self.hero.health -= 1;
                self.hero.invulnerable_ticks = 55;
            }
        }
    }

    pub fn intact_timbers(&self) -> usize {
        self.blocks
            .iter()
            .filter(|block| block.intact && block.material.destructible())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{InputFrame, RawInput};

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
