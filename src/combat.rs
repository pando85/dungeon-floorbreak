//! One source of truth for melee combat (WORK-021 domain, COMBAT-INT-01).
//!
//! Bevy-free, deterministic at the 60 Hz fixed tick, no wall-clock time and no
//! randomness. The old combat greybox ([`crate::prototype::World`]) and the
//! Floor 1 scene fixtures ([`crate::combat_scene`]) both call this module:
//!
//! * [`MoveSpec`]: a move is data (startup, active, recovery, hit shape,
//!   damage, target reaction). [`GREYBOX_STRIKE`] is the WORK-021 melee
//!   expressed as data; [`CARL_JAB`] and [`CARL_PUSH_KICK`] are the Carl moves
//!   of the Floor 1 scene (art contract: `docs/02-art/carl-melee-anim-02-spec.md`).
//! * [`MoveRunner`]: the per-attacker move clock with press buffering and a
//!   per-move-instance hit list, so a target is hit at most once per move.
//! * [`Enemy`]: health, stagger, a deterministic push impulse, ground chase and
//!   the WORK-021 contact damage rule.
//! * [`CombatEvent`]: what happened on which tick, for replay logs.
//!
//! Bombs and explosives are not part of this module; they stay in the
//! WORK-021 greybox only.

use std::fmt;

use crate::prototype::{Block, Rect, TICK_SECONDS};

pub const ENEMY_HALF_WIDTH: f32 = 12.0;
pub const ENEMY_HALF_HEIGHT: f32 = 19.0;
/// Starting health of a regular goblin: two hits (the WORK-021 baseline).
pub const ENEMY_HEALTH: u8 = 2;
/// Starting health of the hero.
pub const HERO_HEALTH: u8 = 4;
/// Ticks the hero is immune to contact damage after being hurt.
pub const HERO_INVULNERABLE_TICKS: u8 = 55;
/// Chase speed of a ground enemy (px/s) and how far away it notices the hero.
const CHASE_SPEED: f32 = 62.0;
const CHASE_RANGE: f32 = 220.0;
const CHASE_STOP: f32 = 22.0;
/// Contact damage reach (centre to centre) of WORK-021.
const CONTACT_X: f32 = 23.0;
const CONTACT_Y: f32 = 36.0;
/// A push impulse is applied evenly over this many ticks.
const PUSH_TICKS: u8 = 4;

// ------------------------------------------------------------------- moves --

/// Where a move's hit test looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HitShape {
    /// A box in front of the attacker's pivot (feet centre): `x_min..x_max`
    /// pixels ahead (mirrored with the facing), `y_min..y_max` pixels above the
    /// feet. A target is hit when its body box overlaps it.
    Box {
        x_min: f32,
        x_max: f32,
        y_min: f32,
        y_max: f32,
    },
    /// The WORK-021 reach test on centre distances: the target centre is within
    /// `back` behind and `front` ahead of the attacker centre and within
    /// `vertical` of its centre height. A greybox value; kept so the old
    /// prototype stays bit-stable.
    GreyboxReach {
        back: f32,
        front: f32,
        vertical: f32,
    },
}

/// What a landed hit does to the target besides damage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reaction {
    /// Ticks the target is staggered (no chase, no contact damage).
    pub stagger_ticks: u8,
    /// Total push away from the attacker, in pixels, spread over a few ticks.
    pub push_px: f32,
}

impl Reaction {
    pub const NONE: Self = Self {
        stagger_ticks: 0,
        push_px: 0.0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveId {
    GreyboxStrike,
    Jab,
    PushKick,
}

impl MoveId {
    pub fn name(self) -> &'static str {
        match self {
            Self::GreyboxStrike => "greybox_strike",
            Self::Jab => "jab",
            Self::PushKick => "push_kick",
        }
    }
}

/// One attack as data. Tick `0` of the move is the tick the input is accepted;
/// the first active tick (`startup`) is the hit tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveSpec {
    pub id: MoveId,
    pub startup: u16,
    pub active: u16,
    pub recovery: u16,
    pub shape: HitShape,
    pub damage: u8,
    pub reaction: Reaction,
}

impl MoveSpec {
    pub const fn total(&self) -> u16 {
        self.startup + self.active + self.recovery
    }
}

/// WORK-021 melee as data: instant hit on the press tick, 14-tick cooldown,
/// reach -9..54 px on centre distances (a greybox value), no target reaction.
pub const GREYBOX_STRIKE: MoveSpec = MoveSpec {
    id: MoveId::GreyboxStrike,
    startup: 0,
    active: 1,
    recovery: 13,
    shape: HitShape::GreyboxReach {
        back: -9.0,
        front: 54.0,
        vertical: 44.0,
    },
    damage: 1,
    reaction: Reaction::NONE,
};

/// Carl's jab (`punch_a` of the melee art): startup 3, active 2, recovery 9.
/// The fist box is +16..21 px ahead of the pivot at 31..36 px above the feet
/// (art reach table), written as the half-open box `[16, 22) x [31, 37)`.
/// Reaction values are the proposed starting values (stagger 10, push about
/// 4 px), tuned in the hit-feel work.
pub const CARL_JAB: MoveSpec = MoveSpec {
    id: MoveId::Jab,
    startup: 3,
    active: 2,
    recovery: 9,
    shape: HitShape::Box {
        x_min: 16.0,
        x_max: 22.0,
        y_min: 31.0,
        y_max: 37.0,
    },
    damage: 1,
    reaction: Reaction {
        stagger_ticks: 10,
        push_px: 4.0,
    },
};

/// Carl's push kick (`kick` of the melee art): startup 13, active 4,
/// recovery 19. The sole box is +15..21 px ahead at 18..27 px above the feet.
/// Its knockback behaviour (launch, CP-02) is NOT defined yet: it deals the
/// jab's damage and reaction as a placeholder.
pub const CARL_PUSH_KICK: MoveSpec = MoveSpec {
    id: MoveId::PushKick,
    startup: 13,
    active: 4,
    recovery: 19,
    shape: HitShape::Box {
        x_min: 15.0,
        x_max: 22.0,
        y_min: 18.0,
        y_max: 28.0,
    },
    damage: 1,
    reaction: Reaction {
        stagger_ticks: 10,
        push_px: 4.0,
    },
};

/// Carl's move table (the documented move table of the Floor 1 scene).
pub const CARL_MOVES: [&MoveSpec; 2] = [&CARL_JAB, &CARL_PUSH_KICK];

/// How long a press made while a move is still running is remembered, in
/// ticks. A press shorter than this before the move ends starts the next move
/// the tick the current one ends; an earlier press is forgotten. Only press
/// edges count: a held button never produces a second press.
pub const CARL_BUFFER_TICKS: u8 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Startup,
    Active,
    Recovery,
}

/// The move clock of one attacker on one fixed tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveTick {
    pub spec: &'static MoveSpec,
    /// Ticks since the move started (0 on the tick the input was accepted).
    pub tick: u16,
    /// Monotonic move instance number (1 for the first move ever started).
    pub instance: u32,
    pub phase: Phase,
    /// True on the tick the move started.
    pub started: bool,
}

impl MoveTick {
    /// The hit tick: the first active tick.
    pub fn first_active(&self) -> bool {
        self.phase == Phase::Active && self.tick == self.spec.startup
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Running {
    spec: &'static MoveSpec,
    tick: u16,
    instance: u32,
    /// Target ids already hit by this move instance.
    hit: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Buffered {
    spec: &'static MoveSpec,
    /// Last runner clock value at which the press may still start a move.
    expires: u64,
}

/// Move clock with press buffering and per-instance hit dedup.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MoveRunner {
    clock: u64,
    next_instance: u32,
    current: Option<Running>,
    buffered: Option<Buffered>,
}

impl MoveRunner {
    /// Advance one fixed tick. `request` is a press edge of this tick,
    /// `buffer_ticks` how long a press during a running move is remembered
    /// (`0` = forgotten at once, the WORK-021 behaviour) and `can_act` false
    /// drops the press (the actor is hurt or defeated). Returns the clock of
    /// the move running on this tick, if any.
    pub fn step(
        &mut self,
        request: Option<&'static MoveSpec>,
        buffer_ticks: u8,
        can_act: bool,
    ) -> Option<MoveTick> {
        self.clock += 1;
        let mut started = false;
        if let Some(running) = self.current.as_mut() {
            running.tick += 1;
            if running.tick >= running.spec.total() {
                self.current = None;
            }
        }
        if !can_act {
            self.current = None;
            self.buffered = None;
            return None;
        }
        if let Some(spec) = request {
            if self.current.is_none() {
                self.start(spec);
                started = true;
            } else if buffer_ticks > 0 {
                self.buffered = Some(Buffered {
                    spec,
                    expires: self.clock + u64::from(buffer_ticks),
                });
            }
        }
        if self.current.is_none()
            && let Some(buffered) = self.buffered.take()
            && buffered.expires >= self.clock
        {
            self.start(buffered.spec);
            started = true;
        }
        self.current.as_ref().map(|running| MoveTick {
            spec: running.spec,
            tick: running.tick,
            instance: running.instance,
            phase: phase_of(running.spec, running.tick),
            started,
        })
    }

    fn start(&mut self, spec: &'static MoveSpec) {
        self.next_instance += 1;
        self.buffered = None;
        self.current = Some(Running {
            spec,
            tick: 0,
            instance: self.next_instance,
            hit: Vec::new(),
        });
    }

    /// Interrupt (the attacker was hit): the move and any buffered press end.
    pub fn cancel(&mut self) {
        self.current = None;
        self.buffered = None;
    }

    /// A move is running (the attacker is committed).
    pub fn busy(&self) -> bool {
        self.current.is_some()
    }

    /// The running move's clock without advancing it.
    pub fn current(&self) -> Option<MoveTick> {
        self.current.as_ref().map(|running| MoveTick {
            spec: running.spec,
            tick: running.tick,
            instance: running.instance,
            phase: phase_of(running.spec, running.tick),
            started: running.tick == 0,
        })
    }

    /// True the first time `target` is recorded for the running move instance.
    fn record_hit(&mut self, target: u32) -> bool {
        match self.current.as_mut() {
            Some(running) if !running.hit.contains(&target) => {
                running.hit.push(target);
                true
            }
            _ => false,
        }
    }

    /// Fold the full runner state into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.clock);
        mix(u64::from(self.next_instance));
        match &self.current {
            Some(running) => {
                mix(1 + running.spec.id as u64);
                mix(u64::from(running.tick));
                mix(u64::from(running.instance));
                for target in &running.hit {
                    mix(u64::from(*target));
                }
            }
            None => mix(0),
        }
        match &self.buffered {
            Some(buffered) => {
                mix(1 + buffered.spec.id as u64);
                mix(buffered.expires);
            }
            None => mix(0),
        }
    }
}

fn phase_of(spec: &MoveSpec, tick: u16) -> Phase {
    if tick < spec.startup {
        Phase::Startup
    } else if tick < spec.startup + spec.active {
        Phase::Active
    } else {
        Phase::Recovery
    }
}

// ------------------------------------------------------------------ enemies --

/// How an enemy moves. `Passive` is a stationary test dummy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyAi {
    Chase,
    Passive,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Enemy {
    pub x: f32,
    pub y: f32,
    pub health: u8,
    pub ai: EnemyAi,
    /// +1 / -1: the way the enemy faces (towards the hero while it chases).
    pub facing: i8,
    /// Ticks left of the stagger after a hit; no chase and no contact damage.
    pub stagger_ticks: u8,
    /// Ticks of the push impulse left and its signed per-tick step in pixels.
    pub push_ticks: u8,
    pub push_dx: f32,
    /// Ticks since the enemy was defeated (0 while alive).
    pub defeated_ticks: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitOutcome {
    Hurt { health_left: u8 },
    Defeated,
}

impl Enemy {
    pub fn new(x: f32, y: f32) -> Self {
        Self::with_ai(x, y, EnemyAi::Chase)
    }

    pub fn with_ai(x: f32, y: f32, ai: EnemyAi) -> Self {
        Self {
            x,
            y,
            health: ENEMY_HEALTH,
            ai,
            facing: -1,
            stagger_ticks: 0,
            push_ticks: 0,
            push_dx: 0.0,
            defeated_ticks: 0,
        }
    }

    /// Alive enemies are the only ones that can be hit or deal damage: a
    /// defeated enemy has no hurtbox and no contact damage.
    pub fn alive(&self) -> bool {
        self.health > 0
    }

    pub fn staggered(&self) -> bool {
        self.stagger_ticks > 0
    }

    /// The enemy's body box (24 x 38, centred on `(x, y)`).
    pub fn body(&self) -> Rect {
        body_rect(self.x, self.y)
    }

    /// Apply a landed move: damage, stagger and the push impulse away from the
    /// attacker (`attacker_facing` is +1 / -1).
    pub fn apply_hit(&mut self, spec: &MoveSpec, attacker_facing: i8) -> HitOutcome {
        self.health = self.health.saturating_sub(spec.damage);
        if spec.reaction.stagger_ticks > 0 {
            self.stagger_ticks = spec.reaction.stagger_ticks;
        }
        if spec.reaction.push_px != 0.0 {
            self.push_ticks = PUSH_TICKS;
            self.push_dx =
                f32::from(attacker_facing) * spec.reaction.push_px / f32::from(PUSH_TICKS);
        }
        if self.health == 0 {
            self.stagger_ticks = 0;
            HitOutcome::Defeated
        } else {
            HitOutcome::Hurt {
                health_left: self.health,
            }
        }
    }

    /// One fixed tick of enemy behaviour: finish a push, hold a stagger, or
    /// chase the hero (the WORK-021 ground chaser: closer than 220 px and
    /// farther than 22 px, 62 px/s, stopped by solid geometry).
    pub fn step(&mut self, hero_x: f32, solids: &[Block]) {
        if self.health == 0 {
            self.defeated_ticks = self.defeated_ticks.saturating_add(1);
            self.push(solids);
            return;
        }
        if self.stagger_ticks > 0 {
            self.stagger_ticks -= 1;
            self.push(solids);
            return;
        }
        self.push(solids);
        if self.ai == EnemyAi::Passive {
            return;
        }
        let dx = hero_x - self.x;
        if dx.abs() < CHASE_RANGE && dx.abs() > CHASE_STOP {
            self.facing = if dx > 0.0 { 1 } else { -1 };
            let candidate_x = self.x + dx.signum() * CHASE_SPEED * TICK_SECONDS;
            if !solid_at(solids, body_rect(candidate_x, self.y)) {
                self.x = candidate_x;
            }
        }
    }

    fn push(&mut self, solids: &[Block]) {
        if self.push_ticks == 0 {
            return;
        }
        self.push_ticks -= 1;
        let candidate_x = self.x + self.push_dx;
        if !solid_at(solids, body_rect(candidate_x, self.y)) {
            self.x = candidate_x;
        }
    }

    /// WORK-021 contact damage: an alive, un-staggered enemy touching the hero
    /// (centre distance under 23 px across and 36 px up) hurts a hero who is
    /// not invulnerable. Returns true when the hero was hurt.
    pub fn contact(
        &self,
        hero_x: f32,
        hero_y: f32,
        hero_health: &mut u8,
        hero_invulnerable: &mut u8,
    ) -> bool {
        if self.alive()
            && !self.staggered()
            && *hero_invulnerable == 0
            && (self.x - hero_x).abs() < CONTACT_X
            && (self.y - hero_y).abs() < CONTACT_Y
        {
            *hero_health = hero_health.saturating_sub(1);
            *hero_invulnerable = HERO_INVULNERABLE_TICKS;
            return true;
        }
        false
    }

    /// Fold the full enemy state into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(u64::from(self.x.to_bits()));
        mix(u64::from(self.y.to_bits()));
        mix(u64::from(self.health));
        mix(self.ai as u64);
        mix(self.facing as u64);
        mix(u64::from(self.stagger_ticks));
        mix(u64::from(self.push_ticks));
        mix(u64::from(self.push_dx.to_bits()));
        mix(u64::from(self.defeated_ticks));
    }
}

fn body_rect(x: f32, y: f32) -> Rect {
    Rect {
        left: x - ENEMY_HALF_WIDTH,
        bottom: y - ENEMY_HALF_HEIGHT,
        width: ENEMY_HALF_WIDTH * 2.0,
        height: ENEMY_HALF_HEIGHT * 2.0,
    }
}

fn solid_at(solids: &[Block], candidate: Rect) -> bool {
    solids
        .iter()
        .any(|block| block.intact && candidate.overlaps(block.rect))
}

// ------------------------------------------------------------------ strikes --

/// Where an attacker stands: AABB centre and which way it faces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attacker {
    pub x: f32,
    pub y: f32,
    /// Half the attacker's box height, to find the feet from the centre.
    pub half_height: f32,
    pub facing: i8,
}

/// The world-space hit box of a [`HitShape::Box`] move, if the shape is one.
pub fn hit_box(shape: &HitShape, attacker: &Attacker) -> Option<Rect> {
    let HitShape::Box {
        x_min,
        x_max,
        y_min,
        y_max,
    } = *shape
    else {
        return None;
    };
    let (left, right) = if attacker.facing >= 0 {
        (attacker.x + x_min, attacker.x + x_max)
    } else {
        (attacker.x - x_max, attacker.x - x_min)
    };
    let feet = attacker.y - attacker.half_height;
    Some(Rect {
        left,
        bottom: feet + y_min,
        width: right - left,
        height: y_max - y_min,
    })
}

fn reaches(shape: &HitShape, attacker: &Attacker, enemy: &Enemy) -> bool {
    match shape {
        HitShape::GreyboxReach {
            back,
            front,
            vertical,
        } => {
            let forward = (enemy.x - attacker.x) * f32::from(attacker.facing);
            (*back..=*front).contains(&forward) && (enemy.y - attacker.y).abs() <= *vertical
        }
        HitShape::Box { .. } => hit_box(shape, attacker).is_some_and(|b| b.overlaps(enemy.body())),
    }
}

/// Resolve the running move against `enemies` (stable index order) on an
/// active tick. A target is hit at most once per move instance, defeated
/// targets are skipped. Hits are applied and logged to `events`.
pub fn resolve_strike(
    runner: &mut MoveRunner,
    move_tick: &MoveTick,
    attacker: Attacker,
    enemies: &mut [Enemy],
    sim_tick: u64,
    events: &mut Vec<CombatEvent>,
) {
    if move_tick.phase != Phase::Active {
        return;
    }
    let spec = move_tick.spec;
    for (index, enemy) in enemies.iter_mut().enumerate() {
        if !enemy.alive() || !reaches(&spec.shape, &attacker, enemy) {
            continue;
        }
        let target = index as u32;
        if !runner.record_hit(target) {
            continue;
        }
        let outcome = enemy.apply_hit(spec, attacker.facing);
        events.push(CombatEvent::Hit {
            tick: sim_tick,
            move_id: spec.id,
            instance: move_tick.instance,
            target,
            damage: spec.damage,
            health_left: enemy.health,
        });
        if outcome == HitOutcome::Defeated {
            events.push(CombatEvent::EnemyDefeated {
                tick: sim_tick,
                target,
            });
        }
    }
}

// ------------------------------------------------------------------- events --

/// Something that happened on a fixed tick, for deterministic replay logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatEvent {
    MoveStarted {
        tick: u64,
        move_id: MoveId,
        instance: u32,
    },
    Hit {
        tick: u64,
        move_id: MoveId,
        instance: u32,
        target: u32,
        damage: u8,
        health_left: u8,
    },
    EnemyDefeated {
        tick: u64,
        target: u32,
    },
    HeroHurt {
        tick: u64,
        source: u32,
        health_left: u8,
    },
    HeroDefeated {
        tick: u64,
    },
    Restart {
        tick: u64,
        deaths: u32,
    },
}

impl CombatEvent {
    /// Stable numeric fingerprint of the event (for the rolling event hash).
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        match *self {
            Self::MoveStarted {
                tick,
                move_id,
                instance,
            } => {
                mix(1);
                mix(tick);
                mix(move_id as u64);
                mix(u64::from(instance));
            }
            Self::Hit {
                tick,
                move_id,
                instance,
                target,
                damage,
                health_left,
            } => {
                mix(2);
                mix(tick);
                mix(move_id as u64);
                mix(u64::from(instance));
                mix(u64::from(target));
                mix(u64::from(damage));
                mix(u64::from(health_left));
            }
            Self::EnemyDefeated { tick, target } => {
                mix(3);
                mix(tick);
                mix(u64::from(target));
            }
            Self::HeroHurt {
                tick,
                source,
                health_left,
            } => {
                mix(4);
                mix(tick);
                mix(u64::from(source));
                mix(u64::from(health_left));
            }
            Self::HeroDefeated { tick } => {
                mix(5);
                mix(tick);
            }
            Self::Restart { tick, deaths } => {
                mix(6);
                mix(tick);
                mix(u64::from(deaths));
            }
        }
    }
}

impl fmt::Display for CombatEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::MoveStarted {
                tick,
                move_id,
                instance,
            } => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=move_started move={} instance={instance}",
                move_id.name()
            ),
            Self::Hit {
                tick,
                move_id,
                instance,
                target,
                damage,
                health_left,
            } => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=hit move={} instance={instance} target={target} damage={damage} health_left={health_left}",
                move_id.name()
            ),
            Self::EnemyDefeated { tick, target } => {
                write!(
                    f,
                    "COMBAT_EVENT tick={tick} kind=enemy_defeated target={target}"
                )
            }
            Self::HeroHurt {
                tick,
                source,
                health_left,
            } => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=hero_hurt source={source} health_left={health_left}"
            ),
            Self::HeroDefeated { tick } => {
                write!(f, "COMBAT_EVENT tick={tick} kind=hero_defeated")
            }
            Self::Restart { tick, deaths } => {
                write!(f, "COMBAT_EVENT tick={tick} kind=restart deaths={deaths}")
            }
        }
    }
}

// -------------------------------------------------------------------- tests --

#[cfg(test)]
mod tests {
    use super::*;

    fn runner_ticks(
        runner: &mut MoveRunner,
        spec: &'static MoveSpec,
        press_on: &[u64],
        buffer: u8,
        ticks: u64,
    ) -> Vec<(u64, Phase, u16, bool)> {
        let mut log = Vec::new();
        for tick in 1..=ticks {
            let request = press_on.contains(&tick).then_some(spec);
            if let Some(m) = runner.step(request, buffer, true) {
                log.push((tick, m.phase, m.tick, m.started));
            }
        }
        log
    }

    #[test]
    fn jab_has_the_art_timings_and_hits_on_the_first_active_tick() {
        assert_eq!(
            (CARL_JAB.startup, CARL_JAB.active, CARL_JAB.recovery),
            (3, 2, 9)
        );
        assert_eq!(CARL_JAB.total(), 14);
        assert_eq!(
            (
                CARL_PUSH_KICK.startup,
                CARL_PUSH_KICK.active,
                CARL_PUSH_KICK.recovery
            ),
            (13, 4, 19)
        );
        assert_eq!(CARL_PUSH_KICK.total(), 36);
        let mut runner = MoveRunner::default();
        let log = runner_ticks(&mut runner, &CARL_JAB, &[10], 0, 40);
        // Press on tick 10 = move tick 0; startup 10..12, active 13..14,
        // recovery 15..23, idle from tick 24.
        let phase_at = |t: u64| log.iter().find(|e| e.0 == t).map(|e| e.1);
        assert_eq!(phase_at(10), Some(Phase::Startup));
        assert_eq!(phase_at(12), Some(Phase::Startup));
        assert_eq!(phase_at(13), Some(Phase::Active));
        assert_eq!(phase_at(14), Some(Phase::Active));
        assert_eq!(phase_at(15), Some(Phase::Recovery));
        assert_eq!(phase_at(23), Some(Phase::Recovery));
        assert_eq!(phase_at(24), None);
        let first_active: Vec<u64> = log
            .iter()
            .filter(|e| e.1 == Phase::Active && e.2 == CARL_JAB.startup)
            .map(|e| e.0)
            .collect();
        assert_eq!(first_active, vec![13]);
    }

    #[test]
    fn a_held_button_is_one_press_and_a_late_press_is_buffered_once() {
        // InputFrame turns a held button into a single press edge: one press
        // yields exactly one move instance however long the button is held.
        let mut runner = MoveRunner::default();
        let log = runner_ticks(&mut runner, &CARL_JAB, &[1], CARL_BUFFER_TICKS, 60);
        assert_eq!(log.iter().filter(|e| e.3).count(), 1);
        // A press in the last CARL_BUFFER_TICKS of the move starts the next
        // move the tick the first one ends (tick 15 here: total 14).
        let mut runner = MoveRunner::default();
        let log = runner_ticks(&mut runner, &CARL_JAB, &[1, 10], CARL_BUFFER_TICKS, 60);
        let starts: Vec<u64> = log.iter().filter(|e| e.3).map(|e| e.0).collect();
        assert_eq!(starts, vec![1, 15]);
        // A press earlier than the buffer window is forgotten.
        let mut runner = MoveRunner::default();
        let log = runner_ticks(&mut runner, &CARL_JAB, &[1, 5], CARL_BUFFER_TICKS, 60);
        let starts: Vec<u64> = log.iter().filter(|e| e.3).map(|e| e.0).collect();
        assert_eq!(starts, vec![1]);
    }

    #[test]
    fn greybox_strike_is_the_work_021_cooldown_of_14_ticks() {
        let mut runner = MoveRunner::default();
        let log = runner_ticks(&mut runner, &GREYBOX_STRIKE, &[1, 5, 14, 15], 0, 40);
        let starts: Vec<u64> = log.iter().filter(|e| e.3).map(|e| e.0).collect();
        // Cooldown 14 set on tick 1 is spent on tick 15; the press on tick 14
        // and the one on tick 5 are dropped, the one on tick 15 starts.
        assert_eq!(starts, vec![1, 15]);
        // Hits on the press tick itself (startup 0).
        assert_eq!(log[0], (1, Phase::Active, 0, true));
    }

    fn facing_right_carl() -> Attacker {
        Attacker {
            x: 100.0,
            y: 126.0,
            half_height: 26.0,
            facing: 1,
        }
    }

    fn goblin_at(x: f32) -> Enemy {
        Enemy::with_ai(x, 100.0 + ENEMY_HALF_HEIGHT, EnemyAi::Passive)
    }

    /// Run one jab against goblins, return the events.
    fn jab_against(enemies: &mut [Enemy], attacker: Attacker) -> Vec<CombatEvent> {
        let mut runner = MoveRunner::default();
        let mut events = Vec::new();
        for tick in 1..=CARL_JAB.total() as u64 + 2 {
            let request = (tick == 1).then_some(&CARL_JAB);
            if let Some(m) = runner.step(request, 0, true) {
                resolve_strike(&mut runner, &m, attacker, enemies, tick, &mut events);
            }
        }
        events
    }

    #[test]
    fn jab_box_reaches_21_px_ahead_of_the_pivot_not_behind_and_not_a_greybox_54() {
        let carl = facing_right_carl();
        // Goblin body left edge at pivot + 21 (centre +33): inside the box.
        let mut near = [goblin_at(100.0 + 33.0)];
        assert_eq!(jab_against(&mut near, carl).len(), 1);
        // One pixel farther: the fist (right edge +22) does not reach.
        let mut far = [goblin_at(100.0 + 35.0)];
        assert!(jab_against(&mut far, carl).is_empty());
        // The WORK-021 54 px reach does not apply to the art-aligned jab.
        let mut greybox = [goblin_at(100.0 + 50.0)];
        assert!(jab_against(&mut greybox, carl).is_empty());
        // Behind Carl: never hit, however close.
        let mut behind = [goblin_at(100.0 - 20.0)];
        assert!(jab_against(&mut behind, carl).is_empty());
        // Facing left mirrors the box.
        let left = Attacker { facing: -1, ..carl };
        let mut mirrored = [goblin_at(100.0 - 30.0), goblin_at(100.0 + 30.0)];
        let events = jab_against(&mut mirrored, left);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], CombatEvent::Hit { target: 0, .. }));
    }

    #[test]
    fn a_target_is_hit_once_per_move_instance_even_though_two_ticks_are_active() {
        let carl = facing_right_carl();
        let mut enemies = [goblin_at(125.0)];
        let events = jab_against(&mut enemies, carl);
        assert_eq!(events.len(), 1);
        assert_eq!(enemies[0].health, ENEMY_HEALTH - 1);
    }

    #[test]
    fn defeated_enemies_cannot_be_hit_or_hurt_the_hero() {
        let mut enemy = goblin_at(100.0);
        assert_eq!(
            enemy.apply_hit(&CARL_JAB, 1),
            HitOutcome::Hurt { health_left: 1 }
        );
        assert_eq!(enemy.apply_hit(&CARL_JAB, 1), HitOutcome::Defeated);
        assert!(!enemy.alive());
        let mut health = HERO_HEALTH;
        let mut invulnerable = 0;
        assert!(!enemy.contact(enemy.x, enemy.y, &mut health, &mut invulnerable));
        let carl = Attacker {
            x: enemy.x - 25.0,
            ..facing_right_carl()
        };
        let mut again = [enemy];
        assert!(jab_against(&mut again, carl).is_empty());
    }

    #[test]
    fn a_hit_staggers_pushes_away_and_a_staggered_enemy_deals_no_contact_damage() {
        let mut enemy = Enemy::new(200.0, 119.0);
        enemy.apply_hit(&CARL_JAB, 1);
        assert_eq!(enemy.stagger_ticks, 10);
        let x0 = enemy.x;
        for _ in 0..4 {
            enemy.step(0.0, &[]);
        }
        assert_eq!(enemy.x, x0 + 4.0);
        let mut health = HERO_HEALTH;
        let mut invulnerable = 0;
        // Staggered: standing on the hero does no damage.
        assert!(!enemy.contact(enemy.x, enemy.y, &mut health, &mut invulnerable));
        for _ in 0..6 {
            enemy.step(0.0, &[]);
        }
        assert!(!enemy.staggered());
        assert!(enemy.contact(enemy.x, enemy.y, &mut health, &mut invulnerable));
        assert_eq!(
            (health, invulnerable),
            (HERO_HEALTH - 1, HERO_INVULNERABLE_TICKS)
        );
        // Invulnerable: no second hit.
        assert!(!enemy.contact(enemy.x, enemy.y, &mut health, &mut invulnerable));
    }
}
