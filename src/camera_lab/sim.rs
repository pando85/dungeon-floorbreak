//! The deterministic lab simulation: two actors (Carl, Donut) with solo
//! switching and 2P input ownership, enemies, tagged destructible props, a moving
//! gate and projectiles in one of the six rooms.
//!
//! It is Bevy-free and ticks at the fixed 60 Hz. It **reuses** the existing
//! domain code instead of building a parallel combat architecture:
//!
//! * [`crate::combat::MoveRunner`], [`crate::combat::resolve_strike`] and
//!   Carl's [`crate::combat::CARL_MOVES`] (jab, haymaker, push kick) with the
//!   [`crate::feel`] numbers;
//! * [`crate::goblin::Goblin`], the Floor 1 scene goblin, for every goblin;
//! * [`crate::input::InputFrame`] edge detection;
//! * the same strike targets trait for goblins, generic foes and props, so a
//!   destructible prop is hit by exactly the same strike code as an enemy.
//!
//! New in the lab (and plainly a probe): the generic foe state machine for the
//! non-goblin enemies, Donut's pre-biscuit swat and pounce as [`MoveSpec`]
//! data, props with hit points and a tagged support-to-weight collapse, the
//! moving gate, the boss charge, the minimal 2-actor input ownership and the
//! solo companion. Nothing here reads the camera, the coverage or any
//! rendering state: the same inputs give the same hash at every coverage.

// Struct updates (`..CARL_JAB`, `..Default::default()`) keep this module compiling
// when other branches add fields to the shared combat structs; on a tree without
// those fields they are "needless".
#![allow(clippy::needless_update)]

use std::fmt;

use super::room::{EnemyKind, IRect, Prop, PropKind, Room};
use crate::combat::{
    Attacker, CARL_BUFFER_TICKS, CARL_JAB, CombatEvent, HERO_INVULNERABLE_TICKS, HitOutcome,
    HitShape, MoveId, MoveRunner, MoveSpec, Phase, Reaction, RootStep, StrikeTarget, carl_move,
    resolve_strike,
};
use crate::feel::{
    HERO_HURT_TICKS, KNOCK_GRAVITY, LETHAL_HIT_STOP_BONUS, MAX_ATTACKERS, TELL_SPACING,
};
use crate::goblin::{Goblin, GoblinMode};
use crate::input::STICK_DEADZONE;
use crate::prototype::{Block, HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Material, Rect, TICK_SECONDS};

pub const CARL: usize = 0;
pub const DONUT: usize = 1;
/// Horizontal room the framing needs around the two actors' centres: both
/// safety regions (96 px a side, `camera_op::SAFETY_PAD_X`) plus Carl's and
/// Donut's half widths. A leash of `cap - LEASH_MARGIN_PX` keeps both regions
/// inside a view of width `cap`.
pub const LEASH_MARGIN_PX: f32 = 220.0;
/// The 2P leash (gameplay, **one platform-independent constant**): 416 px keeps
/// both safety regions inside a 1.0x (640 px) frame, so same-screen co-op needs
/// no widening on any output. It never depends on the output resolution, the
/// camera mode or the coverage: the same inputs give the same authoritative
/// state on a Deck, a 1080p and a 4K machine, and docking or resizing mid-run
/// changes no rule.
pub const DEFAULT_LEASH_PX: f32 = 416.0;
/// The documented alternative (736 px, both regions inside the 1.5x cap). Only
/// an explicit scenario or config choice (`LabSim::leash`, `--leash wide`) may
/// select it, never the output.
pub const WIDE_LEASH_PX: f32 = 736.0;
/// Ticks of a blocked leash before an idle partner starts catching up.
pub const CATCHUP_AFTER_TICKS: u16 = 45;

/// The leash for an output whose automatic camera is capped at `cap_w` px:
/// 736 at 960 (1080p, 4K), 416 at 640 (Steam Deck 1280x800, 1440p, 720p).
pub fn leash_for_cap(cap_w: i32) -> f32 {
    (((cap_w as f32 - LEASH_MARGIN_PX) / 8.0).floor() * 8.0).max(160.0)
}
/// Donut's body (the 40 px cat), her lab movement values and health. All lab
/// tuning proposals, not approved balance.
pub const DONUT_HALF_WIDTH: f32 = 16.0;
pub const DONUT_HALF_HEIGHT: f32 = 14.0;
pub const DONUT_HEALTH: u8 = 3;
const DONUT_RUN: f32 = 205.0;
const DONUT_JUMP: f32 = 600.0;
const CARL_RUN: f32 = 185.0;
const CARL_JUMP: f32 = 440.0;
const GRAVITY: f32 = 1050.0;
const COYOTE: u8 = 6;
const JUMP_BUFFER: u8 = 7;
const DOWN_TICKS: u16 = 90;
const SWITCH_COOLDOWN: u8 = 20;
pub const GATE_PERIOD: u64 = 420;
pub const GATE_OPEN_FROM: u64 = 160;
pub const GATE_WARNING: u64 = 50;
const FOE_DOWN_TICKS: u16 = 40;

// ---------------------------------------------------------------- Donut moves

/// Donut's swat (`swat` of the Donut melee art): a weak feline paw. Pre-biscuit
/// she has no magic; the lab reuses the generic move id.
pub const DONUT_SWAT: MoveSpec = MoveSpec {
    id: MoveId::GreyboxStrike,
    startup: 4,
    active: 3,
    recovery: 10,
    shape: HitShape::Box {
        x_min: 10.0,
        x_max: 26.0,
        y_min: 4.0,
        y_max: 24.0,
    },
    damage: 1,
    reaction: Reaction {
        stagger_ticks: 8,
        knock_vx: 40.0,
        hit_stop: 2,
        ..Reaction::NONE
    },
    chain: None,
    root_step: None,
    ..CARL_JAB
};

/// Donut's pounce (`pounce` of the art): a short forward leap with a paw strike.
pub const DONUT_POUNCE: MoveSpec = MoveSpec {
    id: MoveId::Jab,
    startup: 8,
    active: 5,
    recovery: 18,
    shape: HitShape::Box {
        x_min: 8.0,
        x_max: 36.0,
        y_min: 2.0,
        y_max: 26.0,
    },
    damage: 1,
    reaction: Reaction {
        stagger_ticks: 14,
        knock_vx: 90.0,
        hit_stop: 4,
        ..Reaction::NONE
    },
    chain: None,
    root_step: Some(RootStep {
        from_tick: 1,
        to_tick: 8,
        px_per_tick: 2.0,
    }),
    ..CARL_JAB
};

// ---------------------------------------------------------------------- input

/// One player's logical input for a tick (device adapters live in the binary).
/// Square/X is `primary`, Triangle/Y `physical`, L1/LB `switch`. `special` is
/// Circle/B: story-gated, it does nothing in the lab and is logged as locked.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LabInput {
    pub move_x: f32,
    pub jump: bool,
    pub primary: bool,
    pub physical: bool,
    pub special: bool,
    pub switch: bool,
}

/// Press edges and the dead-zoned axis of one actor's input. The lab keeps its
/// own edge detection (instead of the shared `InputFrame`) so it does not depend
/// on how the shared input struct names its buttons.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Edges {
    pub move_x: f32,
    jump_held: bool,
    primary_held: bool,
    physical_held: bool,
    pub jump_pressed: bool,
    pub primary_pressed: bool,
    pub physical_pressed: bool,
}

impl Edges {
    pub fn advance(&mut self, input: LabInput) {
        let x = if input.move_x.is_finite() {
            input.move_x.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        self.move_x = if x.abs() <= STICK_DEADZONE { 0.0 } else { x };
        self.jump_pressed = input.jump && !self.jump_held;
        self.primary_pressed = input.primary && !self.primary_held;
        self.physical_pressed = input.physical && !self.physical_held;
        self.jump_held = input.jump;
        self.primary_held = input.primary;
        self.physical_held = input.physical;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// One human, a controlled actor and an AI companion; L1 switches.
    Solo,
    /// Two humans: player 1 owns Carl, player 2 owns Donut; L1 does nothing.
    TwoPlayer,
}

// ------------------------------------------------------------------- physics

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body {
    /// Centre, y up.
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub hw: f32,
    pub hh: f32,
    pub facing: i8,
    pub grounded: bool,
    coyote: u8,
    jump_buf: u8,
}

impl Body {
    fn new(x: f32, y: f32, hw: f32, hh: f32) -> Self {
        Self {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            hw,
            hh,
            facing: 1,
            grounded: true,
            coyote: COYOTE,
            jump_buf: 0,
        }
    }
    pub fn rect(&self) -> Rect {
        Rect {
            left: self.x - self.hw,
            bottom: self.y - self.hh,
            width: self.hw * 2.0,
            height: self.hh * 2.0,
        }
    }
    pub fn feet_y(&self) -> f32 {
        self.y - self.hh
    }
}

/// One tick of movement against solid blocks: the same jump buffer, coyote time,
/// horizontal sweep and gravity landing as [`crate::prototype::step_hero`]
/// (a test proves equality for Carl's numbers), with the body size and the run
/// and jump speeds as parameters so Donut can use it.
#[allow(clippy::too_many_arguments)]
pub fn step_body(
    b: &mut Body,
    jump_pressed: bool,
    move_x: f32,
    run: f32,
    jump: f32,
    blocks: &[Block],
    min_x: f32,
    max_x: f32,
) {
    if jump_pressed {
        b.jump_buf = JUMP_BUFFER;
    } else {
        b.jump_buf = b.jump_buf.saturating_sub(1);
    }
    if b.grounded {
        b.coyote = COYOTE;
    } else {
        b.coyote = b.coyote.saturating_sub(1);
    }
    if b.jump_buf > 0 && b.coyote > 0 {
        b.vy = jump;
        b.grounded = false;
        b.jump_buf = 0;
        b.coyote = 0;
    }
    b.vx = move_x * run;
    if b.vx.abs() > 0.1 {
        b.facing = if b.vx > 0.0 { 1 } else { -1 };
    }
    let mut next_x = (b.x + b.vx * TICK_SECONDS).clamp(min_x, max_x);
    for block in blocks.iter().filter(|block| block.intact) {
        let candidate = Rect {
            left: next_x - b.hw,
            bottom: b.y - b.hh,
            width: b.hw * 2.0,
            height: b.hh * 2.0,
        };
        if candidate.overlaps(block.rect) {
            if b.vx > 0.0 {
                next_x = next_x.min(block.rect.left - b.hw);
            } else if b.vx < 0.0 {
                next_x = next_x.max(block.rect.right() + b.hw);
            }
        }
    }
    b.x = next_x;
    let previous_bottom = b.y - b.hh;
    let previous_top = b.y + b.hh;
    b.vy -= GRAVITY * TICK_SECONDS;
    let mut next_y = b.y + b.vy * TICK_SECONDS;
    let mut grounded = false;
    for block in blocks.iter().filter(|block| block.intact) {
        let left = b.x - b.hw;
        let right = b.x + b.hw;
        if right <= block.rect.left || left >= block.rect.right() {
            continue;
        }
        let next_bottom = next_y - b.hh;
        let next_top = next_y + b.hh;
        if b.vy <= 0.0
            && previous_bottom >= block.rect.top() - 0.01
            && next_bottom <= block.rect.top()
        {
            next_y = block.rect.top() + b.hh;
            b.vy = 0.0;
            grounded = true;
        } else if b.vy > 0.0
            && previous_top <= block.rect.bottom + 0.01
            && next_top >= block.rect.bottom
        {
            next_y = block.rect.bottom - b.hh;
            b.vy = 0.0;
        }
    }
    b.y = next_y;
    b.grounded = grounded;
}

// -------------------------------------------------------------------- actors

#[derive(Debug, Clone)]
pub struct Actor {
    pub body: Body,
    pub health: u8,
    pub max_health: u8,
    pub invuln: u8,
    pub hurt_ticks: u8,
    pub hit_stop: u8,
    /// 0 while standing; counts ticks lying down after the last health point.
    pub down_ticks: u16,
    pub downs: u32,
    pub runner: MoveRunner,
    input: Edges,
    pub exit_reached: bool,
    pub heat_cooldown: u8,
}

impl Actor {
    fn new(x: f32, y: f32, hw: f32, hh: f32, health: u8) -> Self {
        Self {
            body: Body::new(x, y, hw, hh),
            health,
            max_health: health,
            invuln: 0,
            hurt_ticks: 0,
            hit_stop: 0,
            down_ticks: 0,
            downs: 0,
            runner: MoveRunner::default(),
            input: Edges::default(),
            exit_reached: false,
            heat_cooldown: 0,
        }
    }
    pub fn down(&self) -> bool {
        self.down_ticks > 0
    }
    pub fn can_act(&self) -> bool {
        !self.down() && self.hurt_ticks == 0 && self.hit_stop == 0
    }
    pub fn current_move(&self) -> Option<crate::combat::MoveTick> {
        self.runner.current()
    }
}

// ---------------------------------------------------------------------- foes

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoeSpec {
    pub kind: EnemyKind,
    pub name: &'static str,
    pub hw: f32,
    pub hh: f32,
    pub hp: u8,
    pub speed: f32,
    pub sight: f32,
    /// Starts the tell at this distance or closer.
    pub trigger: f32,
    pub tell: u16,
    pub active: u16,
    pub recover: u16,
    /// Hit box ahead of the feet: x range, y range.
    pub reach: (f32, f32, f32, f32),
    pub damage: u8,
    /// Forward speed during the active frames (px/s): a lunge, leap or charge.
    pub lunge: f32,
    /// Fires a projectile on the first active tick instead of a melee box.
    pub ranged: bool,
    /// Knockback scale (1 for a small enemy, 0 for a boss).
    pub knock: f32,
    /// Ticks between the end of one attack and the next tell.
    pub cooldown: u8,
}

pub const LLAMA: FoeSpec = FoeSpec {
    kind: EnemyKind::Llama,
    name: "llama",
    hw: 22.0,
    hh: 26.0,
    hp: 2,
    speed: 40.0,
    sight: 520.0,
    trigger: 380.0,
    tell: 42,
    active: 8,
    recover: 40,
    reach: (0.0, 0.0, 0.0, 0.0),
    damage: 1,
    lunge: 0.0,
    ranged: true,
    knock: 1.0,
    cooldown: 70,
};
pub const ENGINEER: FoeSpec = FoeSpec {
    kind: EnemyKind::Engineer,
    name: "engineer",
    hw: 14.0,
    hh: 24.0,
    hp: 3,
    speed: 48.0,
    sight: 300.0,
    trigger: 40.0,
    tell: 30,
    active: 6,
    recover: 28,
    reach: (8.0, 44.0, 0.0, 34.0),
    damage: 1,
    lunge: 90.0,
    ranged: false,
    knock: 0.8,
    cooldown: 50,
};
pub const SCATTERER: FoeSpec = FoeSpec {
    kind: EnemyKind::Scatterer,
    name: "scatterer",
    hw: 12.0,
    hh: 8.0,
    hp: 1,
    speed: 96.0,
    sight: 360.0,
    trigger: 44.0,
    tell: 14,
    active: 8,
    recover: 24,
    reach: (4.0, 30.0, 0.0, 14.0),
    damage: 1,
    lunge: 220.0,
    ranged: false,
    knock: 1.2,
    cooldown: 36,
};
pub const HOARDER: FoeSpec = FoeSpec {
    kind: EnemyKind::Hoarder,
    name: "hoarder",
    hw: 32.0,
    hh: 52.0,
    hp: 4,
    speed: 34.0,
    sight: 420.0,
    trigger: 70.0,
    tell: 40,
    active: 8,
    recover: 36,
    reach: (10.0, 76.0, 0.0, 60.0),
    damage: 1,
    lunge: 0.0,
    ranged: false,
    knock: 0.3,
    cooldown: 50,
};
pub const TROG: FoeSpec = FoeSpec {
    kind: EnemyKind::Trog,
    name: "trog",
    hw: 24.0,
    hh: 34.0,
    hp: 3,
    speed: 52.0,
    sight: 380.0,
    trigger: 64.0,
    tell: 30,
    active: 8,
    recover: 30,
    reach: (8.0, 60.0, 0.0, 56.0),
    damage: 1,
    lunge: 70.0,
    ranged: false,
    knock: 0.6,
    cooldown: 50,
};
pub const JUICER: FoeSpec = FoeSpec {
    kind: EnemyKind::Juicer,
    name: "juicer",
    hw: 34.0,
    hh: 50.0,
    hp: 5,
    speed: 38.0,
    sight: 480.0,
    trigger: 84.0,
    tell: 36,
    active: 9,
    recover: 24,
    reach: (10.0, 96.0, 0.0, 80.0),
    damage: 2,
    lunge: 0.0,
    ranged: false,
    knock: 0.2,
    cooldown: 60,
};
pub const BALL: FoeSpec = FoeSpec {
    kind: EnemyKind::Ball,
    name: "ball",
    hw: 56.0,
    hh: 60.0,
    hp: 6,
    speed: 46.0,
    sight: 1400.0,
    trigger: 1000.0,
    tell: 36,
    active: 78,
    recover: 54,
    reach: (0.0, 0.0, 0.0, 0.0),
    damage: 2,
    lunge: 300.0,
    ranged: false,
    knock: 0.0,
    cooldown: 40,
};

pub fn foe_spec(kind: EnemyKind) -> Option<FoeSpec> {
    Some(match kind {
        EnemyKind::Goblin => return None,
        EnemyKind::Llama => LLAMA,
        EnemyKind::Engineer => ENGINEER,
        EnemyKind::Scatterer => SCATTERER,
        EnemyKind::Hoarder => HOARDER,
        EnemyKind::Trog => TROG,
        EnemyKind::Juicer => JUICER,
        EnemyKind::Ball => BALL,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoeMode {
    Neutral,
    Tell,
    Active,
    Recover,
    Hurt,
    Down,
    Corpse,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Foe {
    pub spec: FoeSpec,
    pub x: f32,
    pub y: f32,
    pub floor_y: f32,
    pub health: u8,
    pub facing: i8,
    pub mode: FoeMode,
    pub mode_tick: u16,
    pub vx: f32,
    pub vy: f32,
    pub hit_stop: u8,
    pub cooldown: u8,
    pub swing_hit: bool,
    /// Charge direction of the boss, fixed when the tell ends.
    pub charge_dir: i8,
    /// Platform x range the foe is bound to (a perch guard), if any.
    pub perch: Option<(f32, f32)>,
    /// Boss: stunned by a gate or wall strike (ticks left, 0 = not).
    pub stun: u16,
}

impl Foe {
    fn new(
        spec: FoeSpec,
        x: f32,
        floor_top: f32,
        hp: Option<u8>,
        perch: Option<(f32, f32)>,
    ) -> Self {
        let y = floor_top + spec.hh;
        Self {
            spec,
            x,
            y,
            floor_y: y,
            health: hp.unwrap_or(spec.hp),
            facing: -1,
            mode: FoeMode::Neutral,
            mode_tick: 0,
            vx: 0.0,
            vy: 0.0,
            hit_stop: 0,
            cooldown: 30,
            swing_hit: false,
            charge_dir: -1,
            perch,
            stun: 0,
        }
    }
    pub fn alive(&self) -> bool {
        self.health > 0
    }
    pub fn body(&self) -> Rect {
        Rect {
            left: self.x - self.spec.hw,
            bottom: self.y - self.spec.hh,
            width: self.spec.hw * 2.0,
            height: self.spec.hh * 2.0,
        }
    }
    /// The melee hit box ahead of the feet, when this foe has one.
    pub fn swing_box(&self) -> Option<Rect> {
        let (x0, x1, y0, y1) = self.spec.reach;
        if x1 <= x0 {
            return None;
        }
        let feet = self.y - self.spec.hh;
        let (l, r) = if self.facing >= 0 {
            (self.x + x0, self.x + x1)
        } else {
            (self.x - x1, self.x - x0)
        };
        Some(Rect {
            left: l,
            bottom: feet + y0,
            width: r - l,
            height: y1 - y0,
        })
    }
    fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(u64::from(self.x.to_bits()));
        mix(u64::from(self.y.to_bits()));
        mix(u64::from(self.health));
        mix(self.mode as u64);
        mix(u64::from(self.mode_tick));
        mix(self.facing as u64);
        mix(u64::from(self.vx.to_bits()));
        mix(u64::from(self.vy.to_bits()));
        mix(u64::from(self.hit_stop));
        mix(u64::from(self.cooldown));
        mix(u64::from(self.swing_hit));
        mix(self.charge_dir as u64);
        mix(u64::from(self.stun));
    }
}

impl StrikeTarget for Foe {
    fn hurtbox_active(&self) -> bool {
        self.alive()
    }
    fn can_be_hit(&self) -> bool {
        self.alive() && self.hit_stop == 0
    }
    fn body(&self) -> Rect {
        Foe::body(self)
    }
    fn centre(&self) -> (f32, f32) {
        (self.x, self.y)
    }
    fn take_hit(&mut self, spec: &MoveSpec, attacker_facing: i8) -> (HitOutcome, u8) {
        let dir = f32::from(attacker_facing);
        self.health = self.health.saturating_sub(spec.damage);
        let mut stop = spec.reaction.hit_stop;
        let outcome = if self.health == 0 {
            self.mode = FoeMode::Down;
            self.mode_tick = 0;
            self.vx = dir * 120.0 * self.spec.knock;
            stop += LETHAL_HIT_STOP_BONUS;
            self.stun = 0;
            HitOutcome::Defeated
        } else {
            // The active frames (a lunge, a charge) cannot be interrupted.
            if self.mode != FoeMode::Active {
                self.mode = FoeMode::Hurt;
                self.mode_tick = 0;
                self.swing_hit = false;
                let stagger = if self.spec.knock == 0.0 {
                    0
                } else {
                    spec.reaction.stagger_ticks
                };
                self.vx = dir * spec.reaction.knock_vx * self.spec.knock;
                if self.spec.knock == 0.0 || stagger == 0 {
                    self.mode = FoeMode::Neutral;
                }
            }
            HitOutcome::Hurt {
                health_left: self.health,
            }
        };
        self.hit_stop = stop + 1;
        (outcome, stop)
    }
}

// --------------------------------------------------------------------- props

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropMode {
    Intact,
    Broken,
    /// A hanging weight dropping after its support broke.
    Falling,
    Landed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropState {
    pub def: Prop,
    /// Collision rect, y up.
    pub rect: Rect,
    pub hp: u8,
    pub mode: PropMode,
    pub vy: f32,
    pub bump_cooldown: u8,
    /// Targets already hurt by this falling weight (foes then actors).
    hit_mask: u32,
}

impl PropState {
    fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(u64::from(self.hp));
        mix(self.mode as u64);
        mix(u64::from(self.rect.bottom.to_bits()));
        mix(u64::from(self.vy.to_bits()));
        mix(u64::from(self.bump_cooldown));
        mix(u64::from(self.hit_mask));
    }
}

impl StrikeTarget for PropState {
    fn hurtbox_active(&self) -> bool {
        self.def.breakable && self.mode == PropMode::Intact && self.hp > 0
    }
    fn body(&self) -> Rect {
        self.rect
    }
    fn centre(&self) -> (f32, f32) {
        self.rect.center()
    }
    fn take_hit(&mut self, spec: &MoveSpec, _facing: i8) -> (HitOutcome, u8) {
        self.hp = self.hp.saturating_sub(spec.damage);
        if self.hp == 0 {
            (HitOutcome::Defeated, 3)
        } else {
            (
                HitOutcome::Hurt {
                    health_left: self.hp,
                },
                2,
            )
        }
    }
}

/// Everything a strike can land on, in one stable-ordered list (enemies first,
/// then props) so [`resolve_strike`] hit ids are unique.
#[derive(Debug, Clone)]
pub enum Hittable {
    Goblin(Goblin),
    Foe(Foe),
    Prop(PropState),
}

impl StrikeTarget for Hittable {
    fn hurtbox_active(&self) -> bool {
        match self {
            Self::Goblin(g) => g.hurtbox_active(),
            Self::Foe(f) => f.hurtbox_active(),
            Self::Prop(p) => p.hurtbox_active(),
        }
    }
    fn can_be_hit(&self) -> bool {
        match self {
            Self::Goblin(g) => g.can_be_hit(),
            Self::Foe(f) => f.can_be_hit(),
            Self::Prop(p) => p.can_be_hit(),
        }
    }
    fn body(&self) -> Rect {
        match self {
            Self::Goblin(g) => StrikeTarget::body(g),
            Self::Foe(f) => StrikeTarget::body(f),
            Self::Prop(p) => p.rect,
        }
    }
    fn centre(&self) -> (f32, f32) {
        match self {
            Self::Goblin(g) => g.centre(),
            Self::Foe(f) => f.centre(),
            Self::Prop(p) => p.centre(),
        }
    }
    fn take_hit(&mut self, spec: &MoveSpec, facing: i8) -> (HitOutcome, u8) {
        match self {
            Self::Goblin(g) => g.take_hit(spec, facing),
            Self::Foe(f) => f.take_hit(spec, facing),
            Self::Prop(p) => p.take_hit(spec, facing),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projectile {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: u16,
}

// -------------------------------------------------------------------- events

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabEvent {
    Combat(CombatEvent),
    MoveStarted {
        tick: u64,
        actor: u8,
        move_name: &'static str,
        instance: u32,
    },
    GoblinTell {
        tick: u64,
        goblin: u32,
    },
    GoblinSwing {
        tick: u64,
        goblin: u32,
    },
    GoblinInterrupted {
        tick: u64,
        goblin: u32,
    },
    Crushed {
        tick: u64,
        target: u32,
    },
    PropHit {
        tick: u64,
        prop: u32,
        hp: u8,
    },
    PropBroken {
        tick: u64,
        prop: u32,
        seed: u32,
    },
    WeightFell {
        tick: u64,
        prop: u32,
    },
    WeightLanded {
        tick: u64,
        prop: u32,
        x: i32,
    },
    ImpactDamage {
        tick: u64,
        victim: u32,
        is_actor: bool,
    },
    ActorHurt {
        tick: u64,
        actor: u8,
        health: u8,
    },
    ActorDown {
        tick: u64,
        actor: u8,
    },
    ActorRevived {
        tick: u64,
        actor: u8,
    },
    Switched {
        tick: u64,
        to: u8,
    },
    SpecialLocked {
        tick: u64,
        actor: u8,
    },
    SpitFired {
        tick: u64,
        foe: u32,
    },
    BossTell {
        tick: u64,
        foe: u32,
    },
    BossCharge {
        tick: u64,
        foe: u32,
        dir: i8,
    },
    BossStunned {
        tick: u64,
        foe: u32,
        by_gate: bool,
    },
    ExitReached {
        tick: u64,
        actor: u8,
    },
}

impl LabEvent {
    fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        match *self {
            Self::Combat(e) => e.mix_into(mix),
            Self::MoveStarted {
                tick,
                actor,
                move_name,
                instance,
            } => {
                mix(120);
                mix(tick);
                mix(u64::from(actor));
                mix(move_name.len() as u64);
                mix(u64::from(instance));
            }
            Self::GoblinTell { tick, goblin } => {
                mix(121);
                mix(tick);
                mix(u64::from(goblin));
            }
            Self::GoblinSwing { tick, goblin } => {
                mix(122);
                mix(tick);
                mix(u64::from(goblin));
            }
            Self::GoblinInterrupted { tick, goblin } => {
                mix(123);
                mix(tick);
                mix(u64::from(goblin));
            }
            Self::Crushed { tick, target } => {
                mix(124);
                mix(tick);
                mix(u64::from(target));
            }
            Self::PropHit { tick, prop, hp } => {
                mix(101);
                mix(tick);
                mix(u64::from(prop));
                mix(u64::from(hp));
            }
            Self::PropBroken { tick, prop, seed } => {
                mix(102);
                mix(tick);
                mix(u64::from(prop));
                mix(u64::from(seed));
            }
            Self::WeightFell { tick, prop } => {
                mix(103);
                mix(tick);
                mix(u64::from(prop));
            }
            Self::WeightLanded { tick, prop, x } => {
                mix(104);
                mix(tick);
                mix(u64::from(prop));
                mix(x as u64);
            }
            Self::ImpactDamage {
                tick,
                victim,
                is_actor,
            } => {
                mix(105);
                mix(tick);
                mix(u64::from(victim));
                mix(u64::from(is_actor));
            }
            Self::ActorHurt {
                tick,
                actor,
                health,
            } => {
                mix(106);
                mix(tick);
                mix(u64::from(actor));
                mix(u64::from(health));
            }
            Self::ActorDown { tick, actor } => {
                mix(107);
                mix(tick);
                mix(u64::from(actor));
            }
            Self::ActorRevived { tick, actor } => {
                mix(108);
                mix(tick);
                mix(u64::from(actor));
            }
            Self::Switched { tick, to } => {
                mix(109);
                mix(tick);
                mix(u64::from(to));
            }
            Self::SpecialLocked { tick, actor } => {
                mix(110);
                mix(tick);
                mix(u64::from(actor));
            }
            Self::SpitFired { tick, foe } => {
                mix(111);
                mix(tick);
                mix(u64::from(foe));
            }
            Self::BossTell { tick, foe } => {
                mix(112);
                mix(tick);
                mix(u64::from(foe));
            }
            Self::BossCharge { tick, foe, dir } => {
                mix(113);
                mix(tick);
                mix(u64::from(foe));
                mix(dir as u64);
            }
            Self::BossStunned { tick, foe, by_gate } => {
                mix(114);
                mix(tick);
                mix(u64::from(foe));
                mix(u64::from(by_gate));
            }
            Self::ExitReached { tick, actor } => {
                mix(115);
                mix(tick);
                mix(u64::from(actor));
            }
        }
    }
}

impl fmt::Display for LabEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Combat(e) => write!(f, "{e}"),
            Self::MoveStarted {
                tick,
                actor,
                move_name,
                instance,
            } => write!(
                f,
                "LAB_EVENT tick={tick} kind=move_started actor={actor} move={move_name} instance={instance}"
            ),
            Self::GoblinTell { tick, goblin } => {
                write!(f, "LAB_EVENT tick={tick} kind=goblin_tell goblin={goblin}")
            }
            Self::GoblinSwing { tick, goblin } => {
                write!(f, "LAB_EVENT tick={tick} kind=goblin_swing goblin={goblin}")
            }
            Self::GoblinInterrupted { tick, goblin } => write!(
                f,
                "LAB_EVENT tick={tick} kind=goblin_interrupted goblin={goblin}"
            ),
            Self::Crushed { tick, target } => {
                write!(f, "LAB_EVENT tick={tick} kind=crushed target={target}")
            }
            Self::PropHit { tick, prop, hp } => {
                write!(f, "LAB_EVENT tick={tick} kind=prop_hit prop={prop} hp={hp}")
            }
            Self::PropBroken { tick, prop, seed } => write!(
                f,
                "LAB_EVENT tick={tick} kind=prop_broken prop={prop} debris_seed={seed}"
            ),
            Self::WeightFell { tick, prop } => {
                write!(f, "LAB_EVENT tick={tick} kind=weight_fell prop={prop}")
            }
            Self::WeightLanded { tick, prop, x } => write!(
                f,
                "LAB_EVENT tick={tick} kind=weight_landed prop={prop} x={x}"
            ),
            Self::ImpactDamage {
                tick,
                victim,
                is_actor,
            } => write!(
                f,
                "LAB_EVENT tick={tick} kind=impact_damage victim={victim} actor={is_actor}"
            ),
            Self::ActorHurt {
                tick,
                actor,
                health,
            } => write!(
                f,
                "LAB_EVENT tick={tick} kind=actor_hurt actor={actor} health={health}"
            ),
            Self::ActorDown { tick, actor } => {
                write!(f, "LAB_EVENT tick={tick} kind=actor_down actor={actor}")
            }
            Self::ActorRevived { tick, actor } => {
                write!(f, "LAB_EVENT tick={tick} kind=actor_revived actor={actor}")
            }
            Self::Switched { tick, to } => write!(f, "LAB_EVENT tick={tick} kind=switched to={to}"),
            Self::SpecialLocked { tick, actor } => {
                write!(f, "LAB_EVENT tick={tick} kind=special_locked actor={actor}")
            }
            Self::SpitFired { tick, foe } => write!(f, "LAB_EVENT tick={tick} kind=spit foe={foe}"),
            Self::BossTell { tick, foe } => {
                write!(f, "LAB_EVENT tick={tick} kind=boss_tell foe={foe}")
            }
            Self::BossCharge { tick, foe, dir } => write!(
                f,
                "LAB_EVENT tick={tick} kind=boss_charge foe={foe} dir={dir}"
            ),
            Self::BossStunned { tick, foe, by_gate } => write!(
                f,
                "LAB_EVENT tick={tick} kind=boss_stunned foe={foe} gate={by_gate}"
            ),
            Self::ExitReached { tick, actor } => {
                write!(f, "LAB_EVENT tick={tick} kind=exit_reached actor={actor}")
            }
        }
    }
}

// ----------------------------------------------------------------------- sim

fn mix(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(0x0000_0100_0000_01b3)
}

/// Gate openness at `tick`: 0.0 closed .. 1.0 open (a 30-tick lift each way).
pub fn gate_open_at(tick: u64) -> f32 {
    let p = tick % GATE_PERIOD;
    let open_to = GATE_PERIOD - 30;
    if p < GATE_OPEN_FROM {
        0.0
    } else if p < GATE_OPEN_FROM + 30 {
        (p - GATE_OPEN_FROM) as f32 / 30.0
    } else if p < open_to {
        1.0
    } else {
        1.0 - (p - open_to) as f32 / 30.0
    }
}

/// True during the last ticks before the gate comes down (the visible warning).
pub fn gate_warning_at(tick: u64) -> bool {
    let p = tick % GATE_PERIOD;
    (GATE_PERIOD - GATE_WARNING..GATE_PERIOD - 30).contains(&p)
}

#[derive(Clone)]
pub struct LabSim {
    pub room: Room,
    pub mode: Mode,
    pub tick: u64,
    pub seed: u64,
    rng: u64,
    /// Room height, to convert layout y (down) to simulation y (up).
    pub height: f32,
    pub actors: [Actor; 2],
    /// Solo: the human-controlled actor. 2P: unused (slot i owns actor i).
    pub active: usize,
    switch_cooldown: u8,
    switch_prev: [bool; 2],
    special_prev: [bool; 2],
    /// 2P leash in px ([`DEFAULT_LEASH_PX`] unless a scenario sets another) and
    /// its catch-up bookkeeping. Part of the authoritative state.
    pub leash: f32,
    leash_hold: u16,
    catchup: Option<usize>,
    prev_bodies: [Body; 2],
    pub targets: Vec<Hittable>,
    pub projectiles: Vec<Projectile>,
    blocks: Vec<Block>,
    /// Block index of every prop (None for a non-solid one) and of the gate.
    prop_block: Vec<Option<usize>>,
    gate_block: Option<usize>,
    gate_index: Option<usize>,
    pub events: Vec<LabEvent>,
    cursor: usize,
    event_hash: u64,
    pub last_tell_start: Option<u64>,
    /// Presentation caches (never read by the simulation, not hashed).
    pub last_hit_tick: u64,
}

fn rect_up(height: f32, r: IRect) -> Rect {
    Rect {
        left: r.x as f32,
        bottom: height - r.bottom() as f32,
        width: r.w as f32,
        height: r.h as f32,
    }
}

impl LabSim {
    pub fn new(room: &Room, mode: Mode, seed: u64) -> Self {
        let height = room.render.1 as f32;
        let floor_top = height - room.floor_y as f32;
        let mut blocks = Vec::new();
        let stone = |rect: Rect| Block {
            rect,
            material: Material::Stone,
            intact: true,
        };
        // Floor slab, both wall columns, platforms and steps: protected solids.
        blocks.push(stone(Rect {
            left: 0.0,
            bottom: 0.0,
            width: room.render.0 as f32,
            height: floor_top,
        }));
        blocks.push(stone(Rect {
            left: 0.0,
            bottom: 0.0,
            width: room.play_x.0 as f32,
            height,
        }));
        blocks.push(stone(Rect {
            left: room.play_x.1 as f32,
            bottom: 0.0,
            width: (room.render.0 - room.play_x.1) as f32,
            height,
        }));
        for p in &room.platforms {
            blocks.push(stone(rect_up(height, p.rect)));
        }
        for s in &room.steps {
            blocks.push(stone(rect_up(height, *s)));
        }
        let mut prop_block = Vec::new();
        let mut gate_block = None;
        let mut gate_index = None;
        let mut states = Vec::new();
        for p in &room.props {
            let rect = rect_up(height, p.rect);
            let solid = p.kind.solid() || p.kind == PropKind::Gate;
            if solid {
                prop_block.push(Some(blocks.len()));
                if p.kind == PropKind::Gate {
                    gate_block = Some(blocks.len());
                    gate_index = Some(p.id);
                }
                blocks.push(stone(rect));
            } else {
                prop_block.push(None);
            }
            states.push(PropState {
                def: p.clone(),
                rect,
                hp: p.hp,
                mode: PropMode::Intact,
                vy: 0.0,
                bump_cooldown: 0,
                hit_mask: 0,
            });
        }
        let mut targets: Vec<Hittable> = Vec::new();
        for e in &room.enemies {
            let (top, perch) = match e.perch {
                Some(i) => {
                    let r = room.platforms[i].rect;
                    (height - r.y as f32, Some((r.x as f32, r.right() as f32)))
                }
                None => (floor_top, None),
            };
            match foe_spec(e.kind) {
                None => {
                    let mut g = Goblin::new(
                        e.x as f32,
                        top + crate::combat::ENEMY_HALF_HEIGHT,
                        crate::combat::EnemyAi::Chase,
                    );
                    if let Some(hp) = e.hp {
                        g.health = hp;
                    }
                    targets.push(Hittable::Goblin(g));
                }
                Some(spec) => {
                    targets.push(Hittable::Foe(Foe::new(spec, e.x as f32, top, e.hp, perch)))
                }
            }
        }
        for s in states {
            targets.push(Hittable::Prop(s));
        }
        let carl_y = floor_top + HERO_HALF_HEIGHT;
        let donut_y = floor_top + DONUT_HALF_HEIGHT;
        let mut sim = Self {
            room: room.clone(),
            mode,
            tick: 0,
            seed,
            rng: seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1,
            height,
            actors: [
                Actor::new(
                    room.spawn_carl as f32,
                    carl_y,
                    HERO_HALF_WIDTH,
                    HERO_HALF_HEIGHT,
                    crate::combat::HERO_HEALTH,
                ),
                Actor::new(
                    room.spawn_donut as f32,
                    donut_y,
                    DONUT_HALF_WIDTH,
                    DONUT_HALF_HEIGHT,
                    DONUT_HEALTH,
                ),
            ],
            active: CARL,
            switch_cooldown: 0,
            switch_prev: [false; 2],
            special_prev: [false; 2],
            leash: DEFAULT_LEASH_PX,
            leash_hold: 0,
            catchup: None,
            prev_bodies: [
                Body::new(
                    room.spawn_carl as f32,
                    carl_y,
                    HERO_HALF_WIDTH,
                    HERO_HALF_HEIGHT,
                ),
                Body::new(
                    room.spawn_donut as f32,
                    donut_y,
                    DONUT_HALF_WIDTH,
                    DONUT_HALF_HEIGHT,
                ),
            ],
            targets,
            projectiles: Vec::new(),
            blocks,
            prop_block,
            gate_block,
            gate_index,
            events: Vec::new(),
            cursor: 0,
            event_hash: 0xcbf2_9ce4_8422_2325,
            last_tell_start: None,
            last_hit_tick: 0,
        };
        sim.actors[DONUT].body.facing = 1;
        sim
    }

    fn rand(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        (x.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }

    pub fn take_new_events(&mut self) -> &[LabEvent] {
        let from = self.cursor.min(self.events.len());
        self.cursor = self.events.len();
        &self.events[from..]
    }

    fn log(&mut self, event: LabEvent) {
        let mut h = self.event_hash;
        event.mix_into(&mut |v| h = mix(h, v));
        self.event_hash = h;
        self.events.push(event);
    }

    /// Top (y up) of the highest intact solid under `x` that is at or below
    /// `feet_y`: where a contact shadow lies (presentation helper).
    pub fn ground_below(&self, x: f32, feet_y: f32) -> f32 {
        self.blocks
            .iter()
            .filter(|b| {
                b.intact && x >= b.rect.left && x < b.rect.right() && b.rect.top() <= feet_y + 1.0
            })
            .map(|b| b.rect.top())
            .fold(self.floor_top(), f32::max)
    }

    /// True when no intact solid lies on the straight line between two points
    /// (y up): a coarse line of sight for the presentation and the camera.
    pub fn line_clear(&self, a: (f32, f32), b: (f32, f32)) -> bool {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let steps = ((dx * dx + dy * dy).sqrt() / 8.0).ceil().max(1.0) as i32;
        (1..steps).all(|i| {
            let t = i as f32 / steps as f32;
            let (x, y) = (a.0 + dx * t, a.1 + dy * t);
            !self.blocks.iter().any(|bl| {
                bl.intact
                    && x >= bl.rect.left
                    && x < bl.rect.right()
                    && y >= bl.rect.bottom
                    && y < bl.rect.top()
            })
        })
    }

    pub fn floor_top(&self) -> f32 {
        self.height - self.room.floor_y as f32
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    pub fn gate_open(&self) -> f32 {
        gate_open_at(self.tick)
    }

    /// Controlled actor in solo mode; in 2P both are human.
    pub fn is_human(&self, actor: usize) -> bool {
        self.mode == Mode::TwoPlayer || actor == self.active
    }

    fn actor_input(&self, actor: usize, inputs: &[LabInput; 2]) -> LabInput {
        match self.mode {
            Mode::TwoPlayer => {
                // A partner left behind by a held leash catches up on its own
                // (walks and hops towards the leader) until its player moves.
                if self.catchup == Some(actor) && inputs[actor] == LabInput::default() {
                    self.companion_input(actor)
                } else {
                    inputs[actor]
                }
            }
            Mode::Solo => {
                if actor == self.active {
                    inputs[0]
                } else {
                    self.companion_input(actor)
                }
            }
        }
    }

    /// Minimal deterministic companion for the inactive actor in solo mode: it
    /// follows the active actor, hops obstacles and swings at an enemy in front
    /// of it. A pure function of the simulation state.
    fn companion_input(&self, actor: usize) -> LabInput {
        let me = &self.actors[actor];
        let lead = &self.actors[1 - actor];
        let mut input = LabInput::default();
        if me.down() {
            return input;
        }
        let side = if lead.body.facing >= 0 { -1.0 } else { 1.0 };
        let want_x = lead.body.x + side * if actor == DONUT { 70.0 } else { 90.0 };
        let dx = want_x - me.body.x;
        if dx.abs() > 28.0 {
            input.move_x = dx.signum();
        }
        // Hop a wall in the way, or follow the leader up.
        let ahead = Rect {
            left: me.body.x + input.move_x * (me.body.hw + 6.0) - 2.0,
            bottom: me.body.y - me.body.hh + 4.0,
            width: 4.0,
            height: 8.0,
        };
        let blocked = input.move_x != 0.0
            && self
                .blocks
                .iter()
                .any(|b| b.intact && ahead.overlaps(b.rect));
        let leader_above = lead.body.feet_y() > me.body.feet_y() + 20.0;
        input.jump = me.body.grounded
            && (blocked || (leader_above && dx.abs() < 120.0 && self.tick.is_multiple_of(40)));
        // Strike a live foe within reach in front, on a slow rhythm.
        let reach = if actor == CARL { 44.0 } else { 34.0 };
        let near = self.targets.iter().any(|t| {
            let (cx, cy) = t.centre();
            let live = match t {
                Hittable::Prop(_) => false,
                _ => t.hurtbox_active(),
            };
            live && (cx - me.body.x) * f32::from(me.body.facing) > 0.0
                && (cx - me.body.x).abs() < reach + 12.0
                && (cy - me.body.y).abs() < 44.0
        });
        if near && self.tick % 9 == (actor as u64 * 4) {
            input.primary = true;
        }
        input
    }

    fn nearest_actor_x(&self, x: f32, y_feet: f32, reach_y: f32) -> Option<(usize, f32)> {
        self.actors
            .iter()
            .enumerate()
            .filter(|(_, a)| !a.down() && (a.body.feet_y() - y_feet).abs() <= reach_y)
            .map(|(i, a)| (i, a.body.x))
            .min_by(|a, b| {
                (a.1 - x)
                    .abs()
                    .total_cmp(&(b.1 - x).abs())
                    .then(a.0.cmp(&b.0))
            })
    }

    /// One fixed tick. `inputs[0]` is player 1 (Carl in 2P, the controlled actor
    /// in solo), `inputs[1]` is player 2 (Donut, 2P only).
    pub fn step(&mut self, inputs: [LabInput; 2]) {
        self.tick += 1;
        let tick = self.tick;
        self.update_gate();
        self.handle_switch(&inputs);
        self.projectiles_before_actors();

        self.prev_bodies = [self.actors[CARL].body, self.actors[DONUT].body];
        // Actors in index order.
        for i in 0..2 {
            let input = self.actor_input(i, &inputs);
            self.step_actor(i, input);
        }
        self.step_targets();
        self.step_props();
        self.step_projectiles();
        self.step_hazards();
        self.step_exit();
        self.step_revive();
        // 2P leash and the shared bound on the two actors.
        if self.mode == Mode::TwoPlayer {
            self.leash();
        }
        let _ = tick;
    }

    fn handle_switch(&mut self, inputs: &[LabInput; 2]) {
        self.switch_cooldown = self.switch_cooldown.saturating_sub(1);
        let pressed = inputs[0].switch && !self.switch_prev[0];
        self.switch_prev[0] = inputs[0].switch;
        if self.mode == Mode::Solo && pressed && self.switch_cooldown == 0 {
            let other = 1 - self.active;
            if !self.actors[other].down() {
                self.active = other;
                self.switch_cooldown = SWITCH_COOLDOWN;
                self.log(LabEvent::Switched {
                    tick: self.tick,
                    to: other as u8,
                });
            }
        }
        for (slot, special) in [(0usize, inputs[0].special), (1, inputs[1].special)] {
            let edge = special && !self.special_prev[slot];
            self.special_prev[slot] = special;
            if edge {
                let actor = if self.mode == Mode::Solo {
                    self.active
                } else {
                    slot
                };
                self.log(LabEvent::SpecialLocked {
                    tick: self.tick,
                    actor: actor as u8,
                });
            }
        }
    }

    fn update_gate(&mut self) {
        let (Some(block), Some(index)) = (self.gate_block, self.gate_index) else {
            return;
        };
        let open = gate_open_at(self.tick);
        // The gate never closes on a body: it waits (a safe escape before closure).
        let rect = match &self.targets[self.room.enemies.len() + index] {
            Hittable::Prop(p) => p.rect,
            _ => return,
        };
        let occupied = self.actors.iter().any(|a| a.body.rect().overlaps(rect))
            || self.targets.iter().any(|t| match t {
                Hittable::Goblin(g) => g.alive() && g.body().overlaps(rect),
                Hittable::Foe(f) => {
                    f.alive() && f.spec.kind != EnemyKind::Ball && f.body().overlaps(rect)
                }
                _ => false,
            });
        let closed = open < 0.5 && !(occupied && !self.blocks[block].intact);
        self.blocks[block].intact = closed;
    }

    fn projectiles_before_actors(&mut self) {}

    // ------------------------------------------------------------------ actors

    fn step_actor(&mut self, i: usize, input: LabInput) {
        let tick = self.tick;
        let (run, jump, half_h, min_x, max_x) = if i == CARL {
            (
                CARL_RUN,
                CARL_JUMP,
                HERO_HALF_HEIGHT,
                self.room.play_x.0 as f32 + HERO_HALF_WIDTH,
                self.room.play_x.1 as f32 - HERO_HALF_WIDTH,
            )
        } else {
            (
                DONUT_RUN,
                DONUT_JUMP,
                DONUT_HALF_HEIGHT,
                self.room.play_x.0 as f32 + DONUT_HALF_WIDTH,
                self.room.play_x.1 as f32 - DONUT_HALF_WIDTH,
            )
        };
        let _ = half_h;
        {
            let a = &mut self.actors[i];
            a.input.advance(input);
            a.invuln = a.invuln.saturating_sub(1);
            a.heat_cooldown = a.heat_cooldown.saturating_sub(1);
            if a.hit_stop > 0 {
                a.hit_stop -= 1;
            } else {
                a.hurt_ticks = a.hurt_ticks.saturating_sub(1);
            }
            if a.down() {
                a.down_ticks += 1;
            }
        }
        if self.actors[i].down() {
            // Lying down: gravity only.
            let blocks = self.blocks.clone();
            let a = &mut self.actors[i];
            step_body(&mut a.body, false, 0.0, run, jump, &blocks, min_x, max_x);
            return;
        }
        let can_act = self.actors[i].can_act();
        let melee_frozen =
            self.actors[i].runner.busy() && self.actors[i].runner.hit_stop_left() > 0;
        let request: Option<&'static MoveSpec> = if i == CARL {
            if self.actors[i].input.primary_pressed {
                carl_move(MoveId::Jab)
            } else if self.actors[i].input.physical_pressed {
                carl_move(MoveId::PushKick)
            } else {
                None
            }
        } else if self.actors[i].input.primary_pressed {
            Some(&DONUT_SWAT)
        } else if self.actors[i].input.physical_pressed {
            Some(&DONUT_POUNCE)
        } else {
            None
        };
        if can_act && request.is_some() && !self.actors[i].runner.busy() && input.move_x != 0.0 {
            self.actors[i].body.facing = if input.move_x > 0.0 { 1 } else { -1 };
        }
        let move_tick = self.actors[i]
            .runner
            .step(request, CARL_BUFFER_TICKS, can_act);
        if let Some(mt) = move_tick {
            if mt.started {
                self.log(LabEvent::MoveStarted {
                    tick,
                    actor: i as u8,
                    move_name: mt.spec.id.name(),
                    instance: mt.instance,
                });
            }
            if !mt.frozen
                && let Some(step) = mt.spec.root_step
                && (step.from_tick..=step.to_tick).contains(&mt.tick)
            {
                self.root_step(i, step);
            }
            let a = &self.actors[i];
            // `..Default::default()`: other branches add fields (the credited
            // player slot); the lab leaves them at their default.
            let attacker = Attacker {
                x: a.body.x,
                y: a.body.y,
                half_height: a.body.hh,
                facing: a.body.facing,
                ..Default::default()
            };
            let mut hits = Vec::new();
            let mut runner = std::mem::take(&mut self.actors[i].runner);
            resolve_strike(
                &mut runner,
                &mt,
                attacker,
                &mut self.targets,
                tick,
                &mut hits,
            );
            self.actors[i].runner = runner;
            for event in hits {
                if let CombatEvent::Hit {
                    target,
                    health_left,
                    ..
                } = event
                {
                    self.last_hit_tick = tick;
                    self.after_hit(target as usize, health_left, attacker);
                }
                self.log(LabEvent::Combat(event));
            }
        }
        // Committed on the ground: the feet stay planted.
        let committed = self.actors[i].runner.busy() && self.actors[i].body.grounded;
        let (move_x, jump_pressed) = if !can_act || committed {
            (0.0, false)
        } else {
            (
                input.move_x.clamp(-1.0, 1.0),
                self.actors[i].input.jump_pressed,
            )
        };
        let move_x = if self.actors[i].input.move_x == 0.0 {
            0.0
        } else {
            move_x
        };
        if self.actors[i].hit_stop > 0 || melee_frozen {
            // Frozen: nothing moves.
            return;
        }
        let blocks = self.blocks.clone();
        let a = &mut self.actors[i];
        step_body(
            &mut a.body,
            jump_pressed,
            move_x,
            run,
            jump,
            &blocks,
            min_x,
            max_x,
        );
    }

    fn root_step(&mut self, i: usize, step: RootStep) {
        let a = &mut self.actors[i];
        if !a.body.grounded {
            return;
        }
        let wanted = a.body.x + f32::from(a.body.facing) * step.px_per_tick;
        let candidate = Rect {
            left: wanted - a.body.hw,
            bottom: a.body.y - a.body.hh,
            width: a.body.hw * 2.0,
            height: a.body.hh * 2.0,
        };
        if !self
            .blocks
            .iter()
            .any(|b| b.intact && candidate.overlaps(b.rect))
        {
            a.body.x = wanted;
        }
    }

    fn after_hit(&mut self, target: usize, health_left: u8, _attacker: Attacker) {
        let tick = self.tick;
        let enemies = self.room.enemies.len();
        if target >= enemies {
            let prop_index = target - enemies;
            if health_left == 0 {
                let seed = self.rand();
                self.log(LabEvent::PropBroken {
                    tick,
                    prop: prop_index as u32,
                    seed,
                });
                self.break_prop(prop_index);
            } else {
                self.log(LabEvent::PropHit {
                    tick,
                    prop: prop_index as u32,
                    hp: health_left,
                });
            }
        }
    }

    fn break_prop(&mut self, prop_index: usize) {
        let enemies = self.room.enemies.len();
        let (kind, link) = match &mut self.targets[enemies + prop_index] {
            Hittable::Prop(p) => {
                p.mode = PropMode::Broken;
                p.hp = 0;
                (p.def.kind, p.def.link)
            }
            _ => return,
        };
        if let Some(b) = self.prop_block[prop_index] {
            self.blocks[b].intact = false;
        }
        if kind == PropKind::Support
            && let Some(weight) = link
            && let Hittable::Prop(w) = &mut self.targets[enemies + weight]
            && w.mode == PropMode::Intact
        {
            w.mode = PropMode::Falling;
            w.vy = 0.0;
            let tick = self.tick;
            self.log(LabEvent::WeightFell {
                tick,
                prop: weight as u32,
            });
        }
    }

    // ----------------------------------------------------------------- targets

    fn step_targets(&mut self) {
        let tick = self.tick;
        let enemies = self.room.enemies.len();
        // Goblin attack tokens: at most MAX_ATTACKERS in a tell or swing.
        let active_tokens = self
            .targets
            .iter()
            .filter(|t| matches!(t, Hittable::Goblin(g) if g.token_held()))
            .count();
        let slots = MAX_ATTACKERS.saturating_sub(active_tokens);
        let mut ranked: Vec<(f32, usize)> = Vec::new();
        for (index, t) in self.targets.iter().enumerate() {
            if let Hittable::Goblin(g) = t
                && g.wants_attack()
                && let Some((_, ax)) = self.nearest_actor_x(g.x, g.y - g.body().height / 2.0, 40.0)
            {
                ranked.push(((g.x - ax).abs(), index));
            }
        }
        ranked.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let approved: Vec<usize> = ranked.iter().take(slots).map(|r| r.1).collect();
        let blocks = self.blocks.clone();
        let x_max = self.room.play_x.1 as f32 - 12.0;
        for index in 0..enemies {
            let may_start = self
                .last_tell_start
                .is_none_or(|s| tick - s >= TELL_SPACING);
            match self.targets[index].clone() {
                Hittable::Goblin(mut g) => {
                    let feet = g.y - crate::combat::ENEMY_HALF_HEIGHT;
                    let target = self.nearest_actor_x(g.x, feet, 40.0);
                    let (hx, alive) = match target {
                        Some((_, x)) => (x, true),
                        None => (g.x, false),
                    };
                    let was_attacking = g.token_held();
                    let step = g.step(
                        hx,
                        alive,
                        &blocks,
                        x_max,
                        approved.contains(&index),
                        may_start,
                    );
                    if let Some(Hittable::Foe(_)) = None::<Hittable> {}
                    // Perch guards stay on their platform.
                    if let Some(range) = self.perch_range(index) {
                        g.x = g.x.clamp(range.0 + 12.0, range.1 - 12.0);
                    }
                    if step.started_tell {
                        self.last_tell_start = Some(tick);
                        self.log(LabEvent::GoblinTell {
                            tick,
                            goblin: index as u32,
                        });
                    }
                    if step.swing_began {
                        self.log(LabEvent::GoblinSwing {
                            tick,
                            goblin: index as u32,
                        });
                    }
                    if was_attacking
                        && g.mode != GoblinMode::Attack
                        && g.attack_tick == 0
                        && g.hit_stop > 0
                    {
                        self.log(LabEvent::GoblinInterrupted {
                            tick,
                            goblin: index as u32,
                        });
                    }
                    let swing = (step.impact).then(|| g.swing_box());
                    let (gx, source) = (g.x, index as u32);
                    let mut swing_hit = g.swing_hit;
                    if let Some(boxr) = swing {
                        for ai in 0..2 {
                            if swing_hit {
                                break;
                            }
                            if self.actor_hittable(ai) && boxr.overlaps(self.actors[ai].body.rect())
                            {
                                swing_hit = true;
                                g.hit_stop = g.hit_stop.max(crate::feel::HERO_HURT_HIT_STOP);
                                let away = if self.actors[ai].body.x >= gx {
                                    1.0
                                } else {
                                    -1.0
                                };
                                self.hurt_actor(ai, 1, away, source);
                            }
                        }
                    }
                    g.swing_hit = swing_hit;
                    self.targets[index] = Hittable::Goblin(g);
                }
                Hittable::Foe(mut f) => {
                    self.step_foe(index, &mut f, &blocks);
                    self.targets[index] = Hittable::Foe(f);
                }
                Hittable::Prop(_) => {}
            }
        }
        // Launched enemies bump destructible props: one damage per flight.
        for index in 0..enemies {
            let (body, speed) = match &self.targets[index] {
                // Any goblin that is flying or staggered (modes added by other
                // branches, such as a kick launch, count too).
                Hittable::Goblin(g)
                    if !matches!(
                        g.mode,
                        GoblinMode::Neutral
                            | GoblinMode::Attack
                            | GoblinMode::Down
                            | GoblinMode::Corpse
                    ) =>
                {
                    (g.body(), g.vx.abs())
                }
                _ => continue,
            };
            if speed < 60.0 {
                continue;
            }
            let probe = Rect {
                left: body.left - 3.0,
                width: body.width + 6.0,
                ..body
            };
            for p in enemies..self.targets.len() {
                let hit = match &mut self.targets[p] {
                    Hittable::Prop(ps)
                        if ps.def.breakable
                            && ps.mode == PropMode::Intact
                            && ps.bump_cooldown == 0
                            && ps.rect.overlaps(probe) =>
                    {
                        ps.hp = ps.hp.saturating_sub(1);
                        ps.bump_cooldown = 30;
                        Some(ps.hp)
                    }
                    _ => None,
                };
                if let Some(hp) = hit {
                    let prop = (p - enemies) as u32;
                    if hp == 0 {
                        let seed = self.rand();
                        self.log(LabEvent::PropBroken { tick, prop, seed });
                        self.break_prop(p - enemies);
                    } else {
                        self.log(LabEvent::PropHit { tick, prop, hp });
                    }
                }
            }
        }
    }

    fn perch_range(&self, index: usize) -> Option<(f32, f32)> {
        let def = self.room.enemies.get(index)?;
        let platform = self.room.platforms.get(def.perch?)?;
        Some((platform.rect.x as f32, platform.rect.right() as f32))
    }

    fn actor_hittable(&self, i: usize) -> bool {
        let a = &self.actors[i];
        !a.down() && a.invuln == 0
    }

    fn hurt_actor(&mut self, i: usize, damage: u8, away: f32, source: u32) {
        let tick = self.tick;
        let a = &mut self.actors[i];
        a.health = a.health.saturating_sub(damage);
        a.invuln = HERO_INVULNERABLE_TICKS;
        a.hurt_ticks = HERO_HURT_TICKS;
        a.hit_stop = a.hit_stop.max(crate::feel::HERO_HURT_HIT_STOP);
        a.runner.cancel();
        a.body.vx = away * 120.0;
        let health = a.health;
        let _ = source;
        self.log(LabEvent::ActorHurt {
            tick,
            actor: i as u8,
            health,
        });
        if health == 0 {
            let a = &mut self.actors[i];
            a.down_ticks = 1;
            a.downs += 1;
            self.log(LabEvent::ActorDown {
                tick,
                actor: i as u8,
            });
            if self.mode == Mode::Solo && self.active == i && !self.actors[1 - i].down() {
                self.active = 1 - i;
                self.log(LabEvent::Switched {
                    tick,
                    to: self.active as u8,
                });
            }
        }
    }

    fn step_revive(&mut self) {
        let tick = self.tick;
        for i in 0..2 {
            if self.actors[i].down_ticks >= DOWN_TICKS {
                let partner = &self.actors[1 - i];
                let px = if partner.down() {
                    self.room.spawn_carl as f32
                } else {
                    partner.body.x
                };
                let a = &mut self.actors[i];
                a.down_ticks = 0;
                a.health = (a.max_health / 2).max(1);
                a.invuln = HERO_INVULNERABLE_TICKS;
                a.body.x = (px - 36.0 * f32::from(partner_side(i))).clamp(
                    self.room.play_x.0 as f32 + 20.0,
                    self.room.play_x.1 as f32 - 20.0,
                );
                a.body.y = self.height - self.room.floor_y as f32 + a.body.hh + 1.0;
                a.body.vy = 0.0;
                self.log(LabEvent::ActorRevived {
                    tick,
                    actor: i as u8,
                });
            }
        }
    }

    /// Generic foe state machine (everything except goblins).
    fn step_foe(&mut self, index: usize, f: &mut Foe, blocks: &[Block]) {
        let tick = self.tick;
        if f.hit_stop > 0 {
            f.hit_stop -= 1;
            return;
        }
        let spec = f.spec;
        let boss = spec.kind == EnemyKind::Ball;
        let target = self.nearest_actor_x(f.x, f.y - spec.hh, if boss { 400.0 } else { 40.0 });
        let x_max = self.room.play_x.1 as f32 - spec.hw;
        let x_min = self.room.play_x.0 as f32 + spec.hw;
        let integrate = |f: &mut Foe, blocks: &[Block]| {
            if f.vx != 0.0 {
                let wanted = (f.x + f.vx * TICK_SECONDS).clamp(x_min, x_max);
                let rect = Rect {
                    left: wanted - spec.hw,
                    bottom: f.y - spec.hh,
                    width: spec.hw * 2.0,
                    height: spec.hh * 2.0,
                };
                let blocked = blocks.iter().any(|b| b.intact && rect.overlaps(b.rect));
                if !blocked {
                    f.x = wanted;
                }
                if blocked || (wanted - (f.x)).abs() > 0.0 && f.x != wanted {
                    f.vx = 0.0;
                }
            }
            if let Some((l, r)) = f.perch {
                f.x = f.x.clamp(l + spec.hw, r - spec.hw);
            }
        };
        match f.mode {
            FoeMode::Corpse => {}
            FoeMode::Down => {
                integrate(f, blocks);
                f.vx *= 0.9;
                f.mode_tick += 1;
                if f.mode_tick >= FOE_DOWN_TICKS {
                    f.mode = FoeMode::Corpse;
                    f.vx = 0.0;
                }
            }
            FoeMode::Hurt => {
                integrate(f, blocks);
                f.vx *= 0.85;
                f.mode_tick += 1;
                let stagger = 12u16;
                if f.mode_tick >= stagger {
                    f.mode = FoeMode::Neutral;
                    f.cooldown = 24;
                }
            }
            FoeMode::Neutral => {
                if f.stun > 0 {
                    f.stun -= 1;
                    return;
                }
                f.cooldown = f.cooldown.saturating_sub(1);
                f.vx = 0.0;
                if let Some((_, tx)) = target {
                    let dx = tx - f.x;
                    let dist = dx.abs();
                    if dist < spec.sight {
                        f.facing = if dx > 0.0 { 1 } else { -1 };
                        let range_ok = if spec.ranged {
                            dist <= spec.trigger && dist >= 120.0
                        } else {
                            dist <= spec.trigger
                        };
                        if f.cooldown == 0 && range_ok && may_attack(self, index, tick) {
                            f.mode = FoeMode::Tell;
                            f.mode_tick = 0;
                            f.swing_hit = false;
                            self.last_tell_start = Some(tick);
                            if boss {
                                self.log(LabEvent::BossTell {
                                    tick,
                                    foe: index as u32,
                                });
                            }
                        } else if dist > spec.trigger * 0.6 || boss {
                            // Close in (the boss paces towards its target).
                            let keep = if spec.ranged {
                                260.0
                            } else {
                                spec.trigger * 0.5
                            };
                            if dist > keep {
                                f.vx = f32::from(f.facing) * spec.speed;
                            } else if spec.ranged && dist < 180.0 {
                                f.vx = -f32::from(f.facing) * spec.speed * 0.6;
                            }
                        }
                    }
                }
                integrate(f, blocks);
            }
            FoeMode::Tell => {
                f.mode_tick += 1;
                if f.mode_tick >= spec.tell {
                    f.mode = FoeMode::Active;
                    f.mode_tick = 0;
                    f.swing_hit = false;
                    if boss {
                        // Commit the direction now: the tell showed it.
                        f.charge_dir = f.facing;
                        self.log(LabEvent::BossCharge {
                            tick,
                            foe: index as u32,
                            dir: f.charge_dir,
                        });
                    }
                }
            }
            FoeMode::Active => {
                f.mode_tick += 1;
                if boss {
                    f.vx = f32::from(f.charge_dir) * spec.lunge;
                    let before = f.x;
                    integrate(f, blocks);
                    let rect = f.body();
                    // Struck structures: a closed gate (stun), a wall, breakable props.
                    let ahead = Rect {
                        left: rect.left + f32::from(f.charge_dir) * 6.0,
                        ..rect
                    };
                    let mut stunned = None;
                    if let Some(gb) = self.gate_block
                        && self.blocks[gb].intact
                        && ahead.overlaps(self.blocks[gb].rect)
                    {
                        stunned = Some(true);
                    } else if (f.x - x_min).abs() < 1.0
                        || (f.x - x_max).abs() < 1.0
                        || (f.x - before).abs() < 0.01 && f.vx == 0.0
                    {
                        stunned = Some(false);
                    }
                    // Smash weak props in the lane.
                    let enemies = self.room.enemies.len();
                    for p in enemies..self.targets.len() {
                        let smash = matches!(&self.targets[p], Hittable::Prop(ps)
                            if ps.def.breakable && ps.mode == PropMode::Intact && ps.rect.overlaps(ahead));
                        if smash {
                            if let Hittable::Prop(ps) = &mut self.targets[p] {
                                ps.hp = 0;
                            }
                            let seed = self.rand();
                            self.log(LabEvent::PropBroken {
                                tick,
                                prop: (p - enemies) as u32,
                                seed,
                            });
                            self.break_prop(p - enemies);
                        }
                    }
                    // Contact damage on actors in the lane.
                    for ai in 0..2 {
                        if self.actor_hittable(ai) && f.body().overlaps(self.actors[ai].body.rect())
                        {
                            let away = if self.actors[ai].body.x >= f.x {
                                1.0
                            } else {
                                -1.0
                            };
                            self.hurt_actor(ai, spec.damage, away, index as u32);
                        }
                    }
                    if let Some(by_gate) = stunned {
                        f.mode = FoeMode::Hurt;
                        f.mode_tick = 0;
                        f.vx = 0.0;
                        f.stun = if by_gate { 150 } else { 60 };
                        f.mode = FoeMode::Recover;
                        f.mode_tick = spec.recover.saturating_sub(f.stun.min(spec.recover));
                        self.log(LabEvent::BossStunned {
                            tick,
                            foe: index as u32,
                            by_gate,
                        });
                        // A stunned boss is open to hits: Hurt-ish recover that
                        // hits interrupt (not Active).
                    } else if f.mode_tick >= spec.active {
                        f.mode = FoeMode::Recover;
                        f.mode_tick = 0;
                        f.vx = 0.0;
                    }
                    return;
                }
                if spec.lunge != 0.0 {
                    f.vx = f32::from(f.facing) * spec.lunge;
                    integrate(f, blocks);
                    f.vx = 0.0;
                }
                if spec.ranged && f.mode_tick == 1 {
                    // Spit: aimed at the target's position when it is fired,
                    // with a seeded jitter.
                    let aim = target.map_or((f.x + f32::from(f.facing) * 200.0, f.y), |(i, _)| {
                        (self.actors[i].body.x, self.actors[i].body.y)
                    });
                    let jitter = (self.rand() % 17) as f32 - 8.0;
                    let (sx, sy) = (
                        f.x + f32::from(f.facing) * (spec.hw + 6.0),
                        f.y + spec.hh * 0.3,
                    );
                    let (dx, dy) = (aim.0 - sx, aim.1 + jitter - sy);
                    let len = (dx * dx + dy * dy).sqrt().max(1.0);
                    let speed = 210.0;
                    self.projectiles.push(Projectile {
                        x: sx,
                        y: sy,
                        vx: dx / len * speed,
                        vy: dy / len * speed,
                        life: 150,
                    });
                    self.log(LabEvent::SpitFired {
                        tick,
                        foe: index as u32,
                    });
                }
                // Melee hit box on the active frames.
                if !spec.ranged
                    && !f.swing_hit
                    && let Some(boxr) = f.swing_box()
                {
                    for ai in 0..2 {
                        if self.actor_hittable(ai) && boxr.overlaps(self.actors[ai].body.rect()) {
                            f.swing_hit = true;
                            let away = if self.actors[ai].body.x >= f.x {
                                1.0
                            } else {
                                -1.0
                            };
                            self.hurt_actor(ai, spec.damage, away, index as u32);
                            break;
                        }
                    }
                }
                if f.mode_tick >= spec.active {
                    f.mode = FoeMode::Recover;
                    f.mode_tick = 0;
                }
            }
            FoeMode::Recover => {
                if f.stun > 0 {
                    // A stunned boss stays down on one knee, hits land.
                    f.stun -= 1;
                    if f.stun == 0 {
                        f.mode = FoeMode::Neutral;
                        f.cooldown = spec.cooldown;
                    }
                    return;
                }
                f.mode_tick += 1;
                if f.mode_tick >= spec.recover {
                    f.mode = FoeMode::Neutral;
                    f.cooldown = spec.cooldown;
                }
            }
        }
    }

    // -------------------------------------------------------------------- props

    fn step_props(&mut self) {
        let tick = self.tick;
        let enemies = self.room.enemies.len();
        let floor_top = self.floor_top();
        for p in enemies..self.targets.len() {
            let mut landed = None;
            if let Hittable::Prop(ps) = &mut self.targets[p] {
                ps.bump_cooldown = ps.bump_cooldown.saturating_sub(1);
                if ps.mode == PropMode::Falling {
                    ps.vy -= KNOCK_GRAVITY * TICK_SECONDS;
                    ps.rect.bottom += ps.vy * TICK_SECONDS;
                    if ps.rect.bottom <= floor_top {
                        ps.rect.bottom = floor_top;
                        ps.vy = 0.0;
                        ps.mode = PropMode::Landed;
                        landed = Some((p - enemies, ps.rect.center().0));
                    }
                }
            }
            // A falling or landing weight hurts what is below it, once each.
            let (falling, rect, mask) = match &self.targets[p] {
                Hittable::Prop(ps) => (
                    matches!(ps.mode, PropMode::Falling) || landed.is_some(),
                    ps.rect,
                    ps.hit_mask,
                ),
                _ => continue,
            };
            if falling {
                let zone = Rect {
                    left: rect.left - 10.0,
                    width: rect.width + 20.0,
                    bottom: rect.bottom - 4.0,
                    height: rect.height + 8.0,
                };
                let mut new_mask = mask;
                for e in 0..enemies {
                    let (alive, body) = match &self.targets[e] {
                        Hittable::Goblin(g) => (g.alive(), g.body()),
                        Hittable::Foe(f) => (f.alive() && f.spec.kind != EnemyKind::Ball, f.body()),
                        _ => continue,
                    };
                    if alive && new_mask & (1 << e) == 0 && zone.overlaps(body) {
                        new_mask |= 1 << e;
                        self.log(LabEvent::ImpactDamage {
                            tick,
                            victim: e as u32,
                            is_actor: false,
                        });
                        let dir = if body.center().0 >= rect.center().0 {
                            1
                        } else {
                            -1
                        };
                        self.crush_enemy(e, dir);
                    }
                }
                for ai in 0..2 {
                    if self.actor_hittable(ai)
                        && new_mask & (1 << (16 + ai)) == 0
                        && zone.overlaps(self.actors[ai].body.rect())
                    {
                        new_mask |= 1 << (16 + ai);
                        self.log(LabEvent::ImpactDamage {
                            tick,
                            victim: ai as u32,
                            is_actor: true,
                        });
                        let away = if self.actors[ai].body.x >= rect.center().0 {
                            1.0
                        } else {
                            -1.0
                        };
                        self.hurt_actor(ai, 1, away, 99);
                    }
                }
                if let Hittable::Prop(ps) = &mut self.targets[p] {
                    ps.hit_mask = new_mask;
                }
            }
            if let Some((prop, cx)) = landed {
                self.log(LabEvent::WeightLanded {
                    tick,
                    prop: prop as u32,
                    x: cx as i32,
                });
            }
        }
    }

    fn crush_enemy(&mut self, e: usize, dir: i8) {
        let tick = self.tick;
        let spec = MoveSpec {
            damage: 2,
            reaction: Reaction {
                stagger_ticks: 20,
                knock_vx: 100.0,
                hit_stop: 6,
                ..Reaction::NONE
            },
            ..DONUT_SWAT
        };
        let (outcome, _) = self.targets[e].take_hit(&spec, dir);
        if outcome == HitOutcome::Defeated {
            self.log(LabEvent::Crushed {
                tick,
                target: e as u32,
            });
        }
    }

    // -------------------------------------------------------------- projectiles

    fn step_projectiles(&mut self) {
        let mut keep = Vec::new();
        let projectiles = std::mem::take(&mut self.projectiles);
        for mut p in projectiles {
            p.x += p.vx * TICK_SECONDS;
            p.y += p.vy * TICK_SECONDS;
            p.life = p.life.saturating_sub(1);
            let r = Rect {
                left: p.x - 3.0,
                bottom: p.y - 3.0,
                width: 6.0,
                height: 6.0,
            };
            let mut dead =
                p.life == 0 || self.blocks.iter().any(|b| b.intact && b.rect.overlaps(r));
            if !dead {
                for ai in 0..2 {
                    if self.actor_hittable(ai) && self.actors[ai].body.rect().overlaps(r) {
                        let away = if p.vx >= 0.0 { 1.0 } else { -1.0 };
                        self.hurt_actor(ai, 1, away, 98);
                        dead = true;
                        break;
                    }
                }
            }
            if !dead {
                keep.push(p);
            }
        }
        self.projectiles = keep;
    }

    fn step_hazards(&mut self) {
        let height = self.height;
        for i in 0..2 {
            for h in self.room.hazards.clone() {
                let r = rect_up(height, h.rect);
                let a = &self.actors[i];
                if self.actor_hittable(i)
                    && a.heat_cooldown == 0
                    && a.body.grounded
                    && a.body.rect().overlaps(Rect {
                        height: r.height + 2.0,
                        ..r
                    })
                {
                    self.actors[i].heat_cooldown = 60;
                    self.hurt_actor(i, 1, 0.0, 97);
                }
            }
        }
    }

    fn step_exit(&mut self) {
        let tick = self.tick;
        let exit = rect_up(self.height, self.room.exit);
        for i in 0..2 {
            if !self.actors[i].exit_reached
                && !self.actors[i].down()
                && self.actors[i].body.rect().overlaps(exit)
            {
                self.actors[i].exit_reached = true;
                self.log(LabEvent::ExitReached {
                    tick,
                    actor: i as u8,
                });
            }
        }
    }

    /// Collision-safe 2P leash. When a tick would leave the actors further apart
    /// than the leash, the actor whose move widened the gap is put back on its
    /// body of the start of the tick (a state that was legal), so nobody is ever
    /// moved through geometry. A partner that stays held for a while catches up
    /// by walking (the companion follower), never by a jump in position.
    fn leash(&mut self) {
        let gap = |s: &Self| (s.actors[CARL].body.x - s.actors[DONUT].body.x).abs();
        if gap(self) > self.leash {
            let prev_gap = (self.prev_bodies[CARL].x - self.prev_bodies[DONUT].x).abs();
            if gap(self) > prev_gap {
                // Put back the actor that moved further away, then the other if
                // the gap is still too wide.
                let widen = |s: &Self, i: usize| {
                    let other = 1 - i;
                    gap(s) - (s.prev_bodies[i].x - s.actors[other].body.x).abs()
                };
                let first = if widen(self, CARL) >= widen(self, DONUT) {
                    CARL
                } else {
                    DONUT
                };
                for i in [first, 1 - first] {
                    if gap(self) > self.leash && gap(self) > prev_gap {
                        let mut b = self.prev_bodies[i];
                        b.vx = 0.0;
                        self.actors[i].body = b;
                    }
                }
                self.leash_hold = self.leash_hold.saturating_add(1);
                if self.leash_hold >= CATCHUP_AFTER_TICKS && self.catchup.is_none() {
                    // The partner that was not moving away is the one to catch up.
                    let trailing = if (self.prev_bodies[first].x - self.actors[first].body.x).abs()
                        < f32::EPSILON
                    {
                        1 - first
                    } else {
                        first
                    };
                    self.catchup = Some(trailing);
                }
                return;
            }
        }
        self.leash_hold = 0;
        if gap(self) < self.leash - 60.0 {
            self.catchup = None;
        }
    }

    // --------------------------------------------------------------------- hash

    /// FNV-1a style hash of every authoritative value. The camera, coverage,
    /// presentation caches and the output resolution are not in it. The 2P leash
    /// is (a scenario setting), and it never derives from the output.
    pub fn state_hash(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325_u64;
        let mut m = |v: u64| h = mix(h, v);
        m(self.tick);
        m(self.rng);
        m(self.active as u64);
        m(u64::from(self.leash.to_bits()));
        m(u64::from(self.leash_hold));
        m(self.catchup.map_or(0, |i| 1 + i as u64));
        m(u64::from(self.switch_cooldown));
        for a in &self.actors {
            m(u64::from(a.body.x.to_bits()));
            m(u64::from(a.body.y.to_bits()));
            m(u64::from(a.body.vx.to_bits()));
            m(u64::from(a.body.vy.to_bits()));
            m(a.body.facing as u64);
            m(u64::from(a.body.grounded));
            m(u64::from(a.health));
            m(u64::from(a.invuln));
            m(u64::from(a.hurt_ticks));
            m(u64::from(a.hit_stop));
            m(u64::from(a.down_ticks));
            m(u64::from(a.downs));
            m(u64::from(a.exit_reached));
            a.runner.mix_into(&mut m);
        }
        for t in &self.targets {
            match t {
                Hittable::Goblin(g) => g.mix_into(&mut m),
                Hittable::Foe(f) => f.mix_into(&mut m),
                Hittable::Prop(p) => p.mix_into(&mut m),
            }
        }
        for p in &self.projectiles {
            m(u64::from(p.x.to_bits()));
            m(u64::from(p.y.to_bits()));
            m(u64::from(p.life));
        }
        m(self.event_hash);
        h
    }

    pub fn state_line(&self) -> String {
        let a = &self.actors;
        format!(
            "LAB_STATE room={} mode={:?} tick={} active={} carl=({:.1},{:.1} hp {}) donut=({:.1},{:.1} hp {}) alive_foes={} props_broken={} events={} hash={:016x}",
            self.room.id,
            self.mode,
            self.tick,
            self.active,
            a[CARL].body.x,
            a[CARL].body.y,
            a[CARL].health,
            a[DONUT].body.x,
            a[DONUT].body.y,
            a[DONUT].health,
            self.alive_foes(),
            self.broken_props(),
            self.events.len(),
            self.state_hash()
        )
    }

    pub fn alive_foes(&self) -> usize {
        self.targets
            .iter()
            .filter(|t| match t {
                Hittable::Goblin(g) => g.alive(),
                Hittable::Foe(f) => f.alive(),
                _ => false,
            })
            .count()
    }

    pub fn broken_props(&self) -> usize {
        self.targets
            .iter()
            .filter(|t| matches!(t, Hittable::Prop(p) if p.mode != PropMode::Intact))
            .count()
    }
}

fn partner_side(i: usize) -> i8 {
    if i == CARL { 1 } else { -1 }
}

/// Foes attack only while fewer than two are in a tell, active or recover phase
/// of a melee attack (a tell spacing like the goblin tokens), ranged foes are
/// not limited.
fn may_attack(sim: &LabSim, index: usize, tick: u64) -> bool {
    let busy = sim
        .targets
        .iter()
        .enumerate()
        .filter(|(i, t)| {
            *i != index
                && match t {
                    Hittable::Foe(f) => matches!(f.mode, FoeMode::Tell | FoeMode::Active),
                    Hittable::Goblin(g) => g.token_held(),
                    _ => false,
                }
        })
        .count();
    let spaced = sim.last_tell_start.is_none_or(|s| tick - s >= TELL_SPACING);
    busy < MAX_ATTACKERS && spaced
}

/// A phase of an attack for presentation: used by the camera threat and the
/// binary's animation selection.
pub fn foe_phase_progress(f: &Foe) -> Option<(Phase, u16)> {
    match f.mode {
        FoeMode::Tell => Some((Phase::Startup, f.spec.tell.saturating_sub(f.mode_tick))),
        FoeMode::Active => Some((Phase::Active, f.spec.active.saturating_sub(f.mode_tick))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_lab::room::load_rooms;
    use crate::camera_lab::script::{run, scripts};
    use crate::input::{InputFrame, RawInput};
    use crate::prototype::{Hero, step_hero};
    use std::path::Path;

    fn rooms() -> Vec<Room> {
        load_rooms(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    fn idle() -> [LabInput; 2] {
        [LabInput::default(); 2]
    }

    #[test]
    fn carl_body_physics_equals_the_shared_hero_step() {
        // A floor and a step; the same scripted inputs through both steppers.
        let floor = Block {
            rect: Rect {
                left: 0.0,
                bottom: 0.0,
                width: 2000.0,
                height: 100.0,
            },
            material: Material::Stone,
            intact: true,
        };
        let step = Block {
            rect: Rect {
                left: 400.0,
                bottom: 100.0,
                width: 48.0,
                height: 40.0,
            },
            material: Material::Stone,
            intact: true,
        };
        let blocks = [floor, step];
        let mut hero = Hero::at(100.0, 100.0 + HERO_HALF_HEIGHT);
        let mut body = Body::new(
            100.0,
            100.0 + HERO_HALF_HEIGHT,
            HERO_HALF_WIDTH,
            HERO_HALF_HEIGHT,
        );
        let mut frame = InputFrame::default();
        for t in 0..900u32 {
            let raw = RawInput {
                move_x: if (t / 120) % 3 == 2 { -1.0 } else { 1.0 },
                jump: t % 97 < 3 || t % 211 < 2,
                ..RawInput::default()
            };
            frame.advance(raw);
            step_hero(&mut hero, &frame, &blocks, 1988.0);
            step_body(
                &mut body,
                frame.jump_pressed,
                frame.held.move_x,
                CARL_RUN,
                CARL_JUMP,
                &blocks,
                HERO_HALF_WIDTH,
                1988.0,
            );
            assert_eq!(
                (hero.x, hero.y, hero.vy, hero.grounded),
                (body.x, body.y, body.vy, body.grounded),
                "tick {t}"
            );
        }
    }

    #[test]
    fn the_jumps_match_the_no_superhuman_rule() {
        // Carl's apex stays below 92 px; Donut's high jump is the larger one.
        let apex = |jump: f32| jump * jump / (2.0 * GRAVITY);
        assert!(apex(CARL_JUMP) < 93.0);
        assert!(apex(DONUT_JUMP) > apex(CARL_JUMP) * 1.8 && apex(DONUT_JUMP) < 190.0);
    }

    #[test]
    fn two_player_input_ownership_is_strict_and_l1_does_not_steal() {
        let room = &rooms()[0];
        let mut sim = LabSim::new(room, Mode::TwoPlayer, 1);
        let (cx, dx) = (sim.actors[CARL].body.x, sim.actors[DONUT].body.x);
        let mut p1 = LabInput {
            move_x: 1.0,
            ..Default::default()
        };
        let p2 = LabInput::default();
        for _ in 0..30 {
            sim.step([p1, p2]);
        }
        assert!(sim.actors[CARL].body.x > cx + 50.0);
        assert_eq!(sim.actors[DONUT].body.x, dx, "player 1 never moves Donut");
        // L1 / LB is not a swap in 2P.
        p1.switch = true;
        sim.step([p1, p2]);
        p1.switch = false;
        sim.step([p1, p2]);
        assert!(
            sim.events
                .iter()
                .all(|e| !matches!(e, LabEvent::Switched { .. }))
        );
        // Player 2 moves Donut only.
        let carl_x = sim.actors[CARL].body.x;
        for _ in 0..30 {
            sim.step([
                LabInput::default(),
                LabInput {
                    move_x: -1.0,
                    ..Default::default()
                },
            ]);
        }
        assert!(sim.actors[DONUT].body.x < dx - 50.0);
        assert!((sim.actors[CARL].body.x - carl_x).abs() < 1.0);
    }

    #[test]
    fn solo_switch_gives_the_controlled_actor_to_donut_and_the_ai_takes_carl() {
        let room = &rooms()[0];
        let mut sim = LabSim::new(room, Mode::Solo, 1);
        assert_eq!(sim.active, CARL);
        let sw = LabInput {
            switch: true,
            ..Default::default()
        };
        sim.step([sw, idle()[1]]);
        assert_eq!(sim.active, DONUT);
        // Held L1 does not switch again, a cooldown protects a mash.
        sim.step([sw, idle()[1]]);
        assert_eq!(sim.active, DONUT);
        let carl0 = sim.actors[CARL].body.x;
        for _ in 0..60 {
            sim.step([
                LabInput {
                    move_x: 1.0,
                    ..Default::default()
                },
                idle()[1],
            ]);
        }
        // Slot 0 now drives Donut; the companion AI keeps Carl near her.
        assert!(sim.actors[DONUT].body.x > room.spawn_donut as f32 + 60.0);
        assert!(
            sim.actors[CARL].body.x > carl0 + 20.0,
            "the companion follows"
        );
    }

    #[test]
    fn story_gated_specials_do_nothing_and_are_logged_locked() {
        let room = &rooms()[0];
        let mut sim = LabSim::new(room, Mode::TwoPlayer, 1);
        let special = LabInput {
            special: true,
            ..Default::default()
        };
        let before = sim.state_hash();
        sim.step([special, special]);
        let kinds: Vec<_> = sim
            .events
            .iter()
            .filter(|e| matches!(e, LabEvent::SpecialLocked { .. }))
            .collect();
        assert_eq!(kinds.len(), 2);
        assert!(sim.projectiles.is_empty());
        assert_ne!(before, sim.state_hash());
        // No move was started by the special button.
        assert!(
            sim.events
                .iter()
                .all(|e| !matches!(e, LabEvent::MoveStarted { .. }))
        );
    }

    #[test]
    fn the_two_player_leash_is_a_fixed_constant_independent_of_the_camera() {
        let room = &rooms()[0];
        let mut sim = LabSim::new(room, Mode::TwoPlayer, 1);
        for _ in 0..900 {
            sim.step([
                LabInput {
                    move_x: 1.0,
                    ..Default::default()
                },
                idle()[1],
            ]);
            assert!((sim.actors[CARL].body.x - sim.actors[DONUT].body.x).abs() <= sim.leash + 1.0);
        }
    }

    #[test]
    fn destruction_never_removes_the_protected_floor_steps_platforms_or_exit() {
        let rooms = rooms();
        for (i, script) in scripts().into_iter().enumerate() {
            let room = &rooms[i];
            let mut sim = LabSim::new(room, script.mode, script.seed);
            let protected = 3 + room.platforms.len() + room.steps.len();
            for _ in 0..script.ticks {
                let inputs = crate::camera_lab::script::script_inputs(&sim, &script);
                sim.step(inputs);
            }
            assert!(
                sim.blocks()[..protected].iter().all(|b| b.intact),
                "{}",
                room.id
            );
        }
        // Even with every prop broken the exit stays reachable (room data test).
    }

    #[test]
    fn room03_support_break_makes_the_weight_fall_land_and_hurt_in_that_order() {
        let rooms = rooms();
        let script = scripts().remove(2);
        let trace = run(&rooms[2], &script);
        let at = |needle: &str| {
            trace
                .events
                .iter()
                .position(|e| e.contains(needle))
                .unwrap_or_else(|| panic!("no {needle}"))
        };
        let broken = at("kind=prop_broken prop=2");
        let fell = at("kind=weight_fell prop=3");
        let landed = at("kind=weight_landed prop=3");
        assert!(broken < fell && fell < landed, "cause to effect order");
        assert!(
            trace
                .events
                .iter()
                .any(|e| e.contains("kind=impact_damage")),
            "the falling weight hurt a goblin"
        );
    }

    #[test]
    fn the_gate_cycle_opens_often_and_never_closes_on_a_body() {
        let open_ticks = (0..GATE_PERIOD).filter(|t| gate_open_at(*t) >= 0.5).count() as u64;
        assert!(
            open_ticks * 100 / GATE_PERIOD >= 40,
            "the gate is open {open_ticks} of {GATE_PERIOD} ticks"
        );
        assert!(
            (0..GATE_PERIOD).any(gate_warning_at),
            "a visible warning before closing"
        );
        let rooms = rooms();
        let room = &rooms[5];
        let mut sim = LabSim::new(room, Mode::TwoPlayer, 1);
        // Put Carl under the gate when it is open and run through a closing.
        let gate = sim
            .room
            .props
            .iter()
            .find(|p| p.kind == PropKind::Gate)
            .unwrap()
            .rect;
        // Wait until the gate is open, then stand in its lane through a closing.
        while gate_open_at(sim.tick + 1) < 0.9 {
            sim.step(idle());
        }
        sim.actors[CARL].body.x = (gate.x + gate.w / 2) as f32;
        for _ in 0..(GATE_PERIOD * 2) {
            // Nothing moves Carl: he stands in the gate lane the whole time.
            sim.step(idle());
            let block = sim.gate_block.unwrap();
            if sim.blocks[block].intact {
                assert!(
                    !sim.actors[CARL]
                        .body
                        .rect()
                        .overlaps(sim.blocks[block].rect),
                    "crushed at tick {}",
                    sim.tick
                );
            }
        }
    }

    #[test]
    fn the_boss_charge_is_telegraphed_then_stuns_on_the_closed_gate() {
        let rooms = rooms();
        let script = scripts().remove(5);
        let trace = run(&rooms[5], &script);
        let tell = trace
            .events
            .iter()
            .position(|e| e.contains("kind=boss_tell"))
            .unwrap();
        let charge = trace
            .events
            .iter()
            .position(|e| e.contains("kind=boss_charge"))
            .unwrap();
        let stun = trace
            .events
            .iter()
            .position(|e| e.contains("kind=boss_stunned foe=0 gate=true"))
            .unwrap();
        assert!(tell < charge && charge < stun);
    }

    #[test]
    fn replay_events_do_not_depend_on_call_order_of_presentation() {
        // Taking events (a presentation read) must not change the hash.
        let rooms = rooms();
        let script = scripts().remove(0);
        let mut a = LabSim::new(&rooms[0], script.mode, script.seed);
        let mut b = LabSim::new(&rooms[0], script.mode, script.seed);
        for _ in 0..600 {
            let ia = crate::camera_lab::script::script_inputs(&a, &script);
            let ib = crate::camera_lab::script::script_inputs(&b, &script);
            a.step(ia);
            b.step(ib);
            let _ = a.take_new_events().len();
        }
        assert_eq!(a.state_hash(), b.state_hash());
    }
}

#[cfg(test)]
mod leash_tests {
    use super::*;
    use crate::camera_lab::camera_op::{AUTO_CAP_W, SAFETY_PAD_X, rest_widths};
    use crate::camera_lab::room::load_rooms;
    use std::path::Path;

    fn room(i: usize) -> Room {
        load_rooms(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap()[i].clone()
    }

    #[test]
    fn the_leash_is_consistent_with_the_camera_cap() {
        // Both safety regions (96 px a side plus the half widths) fit the cap.
        assert_eq!(
            LEASH_MARGIN_PX,
            2.0 * SAFETY_PAD_X + HERO_HALF_WIDTH + DONUT_HALF_WIDTH
        );
        for out_w in [1280u32, 1920, 2560, 3840] {
            let cap = *rest_widths(out_w).last().unwrap();
            let leash = leash_for_cap(cap);
            assert!(
                leash + LEASH_MARGIN_PX <= cap as f32,
                "{out_w}: {leash} in {cap}"
            );
            assert!(cap <= AUTO_CAP_W);
        }
        assert_eq!(leash_for_cap(960), WIDE_LEASH_PX);
        assert_eq!(leash_for_cap(640), DEFAULT_LEASH_PX, "Deck, 1440p and 720p");
        // The default fits a 1.0x frame: the same-screen rule needs no widening.
        const { assert!(DEFAULT_LEASH_PX + LEASH_MARGIN_PX <= 640.0) };
    }

    #[test]
    fn the_leash_holds_without_snapping_through_geometry_and_the_partner_catches_up() {
        for leash in [736.0f32, 416.0] {
            let r = room(0);
            let mut sim = LabSim::new(&r, Mode::TwoPlayer, 1);
            sim.leash = leash;
            sim.actors[DONUT].body.x = 40.0;
            let start_donut = sim.actors[DONUT].body.x;
            let mut max_gap = 0.0f32;
            let mut moved_by_catchup = false;
            for tick in 0..1200 {
                // Player 1 runs right and keeps jumping; player 2 does nothing.
                let p1 = LabInput {
                    move_x: 1.0,
                    jump: tick % 50 < 3,
                    ..Default::default()
                };
                let before = sim.actors[CARL].body.x;
                sim.step([p1, LabInput::default()]);
                let gap = (sim.actors[CARL].body.x - sim.actors[DONUT].body.x).abs();
                max_gap = max_gap.max(gap);
                assert!(gap <= leash + 1.0, "tick {tick}: gap {gap} over {leash}");
                // No actor ever ends a tick inside solid geometry.
                for a in &sim.actors {
                    let body = a.body.rect();
                    let inside = sim.blocks().iter().filter(|b| b.intact).any(|b| {
                        body.left < b.rect.right() - 0.01
                            && body.right() > b.rect.left + 0.01
                            && body.bottom < b.rect.top() - 0.01
                            && body.top() > b.rect.bottom + 0.01
                    });
                    assert!(!inside, "tick {tick}: an actor is inside a solid");
                }
                // No tick moves an actor further than its own speed allows.
                assert!(
                    (sim.actors[CARL].body.x - before).abs() < 4.0,
                    "tick {tick}: a jump in position"
                );
                if sim.actors[DONUT].body.x > start_donut + 100.0 {
                    moved_by_catchup = true;
                }
            }
            assert!(
                max_gap > leash - 40.0,
                "the leash was reached ({max_gap} of {leash})"
            );
            assert!(moved_by_catchup, "the idle partner caught up");
        }
    }
}
