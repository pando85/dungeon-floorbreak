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
//! * [`Enemy`]: the WORK-021 greybox enemy (health, ground chase, contact
//!   damage). The Floor 1 scene goblin is [`crate::goblin::Goblin`]; both are
//!   [`StrikeTarget`]s of the same [`resolve_strike`].
//! * Hit feedback (CP-01): a [`Reaction`] carries hit-stop, stagger and the
//!   knockback impulse; the numbers live in [`crate::feel`].
//! * [`CombatEvent`]: what happened on which tick, for replay logs.
//!
//! Bombs and explosives are not part of this module; they stay in the
//! WORK-021 greybox only.

use std::fmt;

use crate::feel::{
    HAYMAKER_FEEL, HAYMAKER_STEP_PX, HAYMAKER_STEP_TICKS, JAB_FEEL, KICK_FEEL, MoveFeel,
};
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

/// What a landed hit does to the target besides damage (hit feedback, CP-01).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reaction {
    /// Ticks the target is staggered (no attack, no tell, no walking).
    pub stagger_ticks: u8,
    /// Knockback impulse in px/s: `knock_vx` away from the attacker, `knock_vy`
    /// upwards. Zero for the greybox.
    pub knock_vx: f32,
    pub knock_vy: f32,
    /// Ticks the attacker's move clock and the target freeze on the hit.
    pub hit_stop: u8,
    /// Camera kick on the confirming hit, virtual pixels (presentation only).
    pub camera_kick_px: u8,
    /// A hit that does not defeat the target launches it (the push kick, D-02)
    /// with `knock_vx` / `knock_vy` instead of staggering it.
    pub launch: bool,
}

impl Reaction {
    pub const NONE: Self = Self {
        stagger_ticks: 0,
        knock_vx: 0.0,
        knock_vy: 0.0,
        hit_stop: 0,
        camera_kick_px: 0,
        launch: false,
    };

    pub const fn from_feel(feel: MoveFeel) -> Self {
        Self {
            stagger_ticks: feel.stagger,
            knock_vx: feel.knock_vx,
            knock_vy: feel.knock_vy,
            hit_stop: feel.hit_stop,
            camera_kick_px: feel.camera_kick_px,
            launch: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveId {
    GreyboxStrike,
    Jab,
    Haymaker,
    PushKick,
}

impl MoveId {
    pub fn name(self) -> &'static str {
        match self {
            Self::GreyboxStrike => "greybox_strike",
            Self::Jab => "jab",
            Self::Haymaker => "haymaker",
            Self::PushKick => "push_kick",
        }
    }
}

/// A string: a press of `on` while the running move is at a tick in
/// `press_from..=press_to` starts `into` as soon as the running move has reached
/// `earliest` (the press tick itself when that is later). Resolved through the
/// move table ([`carl_move`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chain {
    pub on: MoveId,
    pub into: MoveId,
    pub press_from: u16,
    pub press_to: u16,
    pub earliest: u16,
}

/// Forward motion of the attacker on move ticks `from_tick..=to_tick`
/// (`px_per_tick` each, the way the attacker faces): applied by the scene,
/// stopped by walls and at body contact with an alive target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootStep {
    pub from_tick: u16,
    pub to_tick: u16,
    pub px_per_tick: f32,
}

/// One attack as data. Tick `0` of the move is the tick the input is accepted;
/// the first active tick (`startup`) is the earliest hit tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveSpec {
    pub id: MoveId,
    pub startup: u16,
    pub active: u16,
    pub recovery: u16,
    pub shape: HitShape,
    pub damage: u8,
    pub reaction: Reaction,
    /// The string this move can cancel into, if any.
    pub chain: Option<Chain>,
    /// A forward step during the move, if any.
    pub root_step: Option<RootStep>,
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
    chain: None,
    root_step: None,
};

/// Carl's jab (`punch_a` of the melee art): startup 3, active 2, recovery 9.
/// The fist box (art reach table, PR #43 after its redesign) is +16..21 px ahead
/// of the pivot at 29..34 px above the feet, written as the half-open box
/// `[16, 22) x [29, 35)`. A second attack press at jab ticks 3..=13 starts the
/// haymaker at tick 7 or later. Reaction: [`JAB_FEEL`].
pub const CARL_JAB: MoveSpec = MoveSpec {
    id: MoveId::Jab,
    startup: 3,
    active: 2,
    recovery: 9,
    shape: HitShape::Box {
        x_min: 16.0,
        x_max: 22.0,
        y_min: 29.0,
        y_max: 35.0,
    },
    damage: 1,
    reaction: Reaction::from_feel(JAB_FEEL),
    chain: Some(Chain {
        on: MoveId::Jab,
        into: MoveId::Haymaker,
        press_from: 3,
        press_to: 13,
        earliest: 7,
    }),
    root_step: None,
};

/// Carl's haymaker (`punch_b`, an overhand chop): startup 5, active 3, recovery
/// 14, total 22. Contact box +16..21 px ahead at 27..37 px above the feet
/// (`[16, 22) x [27, 38)`). It hits on its first active tick at which the box
/// overlaps a target. Ends the string. Steps Carl forward 4 px (1 px per tick on
/// move ticks 1..=4). Reaction: [`HAYMAKER_FEEL`].
pub const CARL_HAYMAKER: MoveSpec = MoveSpec {
    id: MoveId::Haymaker,
    startup: 5,
    active: 3,
    recovery: 14,
    shape: HitShape::Box {
        x_min: 16.0,
        x_max: 22.0,
        y_min: 27.0,
        y_max: 38.0,
    },
    damage: 1,
    reaction: Reaction::from_feel(HAYMAKER_FEEL),
    chain: None,
    root_step: Some(RootStep {
        from_tick: 1,
        to_tick: HAYMAKER_STEP_TICKS,
        px_per_tick: HAYMAKER_STEP_PX / HAYMAKER_STEP_TICKS as f32,
    }),
};

/// Carl's push kick (`kick` of the melee art): startup 13, active 4,
/// recovery 19. The sole box is +15..21 px ahead at 18..27 px above the feet.
/// A goblin it does not defeat is launched (D-02, [`KICK_FEEL`]); a launched
/// living goblin can interrupt a second one (CP-02, `combat_scene`).
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
        launch: true,
        ..Reaction::from_feel(KICK_FEEL)
    },
    chain: None,
    root_step: None,
};

/// Carl's move table, the one lookup of the Floor 1 scene (strings and the
/// event log resolve moves through [`carl_move`]).
pub const CARL_MOVES: [&MoveSpec; 3] = [&CARL_JAB, &CARL_HAYMAKER, &CARL_PUSH_KICK];

/// The Carl move with this id (`None` for the WORK-021 greybox strike).
pub fn carl_move(id: MoveId) -> Option<&'static MoveSpec> {
    CARL_MOVES.iter().copied().find(|spec| spec.id == id)
}

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
    /// True while the attacker is frozen by hit-stop: the clock did not move
    /// and the tick resolves no hit.
    pub frozen: bool,
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

/// Move clock with press buffering, strings, hit-stop and per-instance hit
/// dedup.
///
/// Hit-stop (`hold`) freezes the move clock: while it runs the move tick does
/// not advance and neither does the buffer clock, so a press made during the
/// freeze is judged by the move tick it froze on (a press during a jab's
/// hit-stop is inside the haymaker window) and a buffered press does not
/// expire during the freeze.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MoveRunner {
    clock: u64,
    next_instance: u32,
    current: Option<Running>,
    buffered: Option<Buffered>,
    chained: Option<&'static MoveSpec>,
    hold: u8,
    /// The last [`Self::step`] was a frozen tick (display only, not hashed).
    froze: bool,
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
        self.froze = false;
        if !can_act {
            self.clock += 1;
            self.current = None;
            self.buffered = None;
            self.chained = None;
            self.hold = 0;
            return None;
        }
        if self.hold > 0 && self.current.is_some() {
            // Hit-stop: nothing advances, the press is still remembered.
            self.hold -= 1;
            self.froze = true;
            if let Some(spec) = request {
                self.note_press(spec, buffer_ticks);
            }
            return self.snapshot(false, true);
        }
        self.hold = 0;
        self.clock += 1;
        let mut started = false;
        if let Some(running) = self.current.as_mut() {
            running.tick += 1;
            if running.tick >= running.spec.total() {
                self.current = None;
                self.chained = None;
            }
        }
        if let Some(spec) = request {
            if self.current.is_none() {
                self.start(spec);
                started = true;
            } else {
                self.note_press(spec, buffer_ticks);
            }
        }
        // A string: start the chained move once the running one is at or past
        // its earliest cancel tick.
        if let (Some(running), Some(into)) = (self.current.as_ref(), self.chained)
            && let Some(chain) = running.spec.chain
            && running.tick >= chain.earliest
        {
            self.start(into);
            started = true;
        }
        if self.current.is_none()
            && let Some(buffered) = self.buffered.take()
            && buffered.expires >= self.clock
        {
            self.start(buffered.spec);
            started = true;
        }
        self.snapshot(started, false)
    }

    /// A press while a move runs: inside the running move's string window it
    /// is the chained move; otherwise it is remembered for `buffer_ticks`.
    fn note_press(&mut self, spec: &'static MoveSpec, buffer_ticks: u8) {
        let Some(running) = self.current.as_ref() else {
            return;
        };
        if let Some(chain) = running.spec.chain
            && chain.on == spec.id
            && (chain.press_from..=chain.press_to).contains(&running.tick)
            && let Some(into) = carl_move(chain.into)
        {
            self.chained.get_or_insert(into);
        } else if buffer_ticks > 0 {
            self.buffered = Some(Buffered {
                spec,
                expires: self.clock + u64::from(buffer_ticks),
            });
        }
    }

    fn start(&mut self, spec: &'static MoveSpec) {
        self.next_instance += 1;
        self.buffered = None;
        self.chained = None;
        self.current = Some(Running {
            spec,
            tick: 0,
            instance: self.next_instance,
            hit: Vec::new(),
        });
    }

    fn snapshot(&self, started: bool, frozen: bool) -> Option<MoveTick> {
        self.current.as_ref().map(|running| MoveTick {
            spec: running.spec,
            tick: running.tick,
            instance: running.instance,
            phase: phase_of(running.spec, running.tick),
            started,
            frozen,
        })
    }

    /// Interrupt (the attacker was hit): the move, any buffered press and any
    /// hit-stop end.
    pub fn cancel(&mut self) {
        self.current = None;
        self.buffered = None;
        self.chained = None;
        self.hold = 0;
    }

    /// Freeze the move clock for `ticks` more ticks (a confirmed hit).
    pub fn begin_hit_stop(&mut self, ticks: u8) {
        self.hold = self.hold.max(ticks);
    }

    /// Ticks of hit-stop still to run.
    pub fn hit_stop_left(&self) -> u8 {
        self.hold
    }

    /// A move is running (the attacker is committed).
    pub fn busy(&self) -> bool {
        self.current.is_some()
    }

    /// The running move's clock without advancing it.
    pub fn current(&self) -> Option<MoveTick> {
        self.snapshot(
            self.current.as_ref().is_some_and(|r| r.tick == 0),
            self.froze,
        )
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
        mix(self.chained.map_or(0, |spec| 1 + spec.id as u64));
        mix(u64::from(self.hold));
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

/// The WORK-021 greybox enemy: health, a ground chase and contact damage. The
/// Floor 1 scene goblin (tell, swing, hit-stop, knockback) is
/// [`crate::goblin::Goblin`].
#[derive(Debug, Clone, PartialEq)]
pub struct Enemy {
    pub x: f32,
    pub y: f32,
    pub health: u8,
    pub ai: EnemyAi,
    /// +1 / -1: the way the enemy faces (towards the hero while it chases).
    pub facing: i8,
    /// Ticks left of a stagger; no chase and no contact damage.
    pub stagger_ticks: u8,
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

    /// Apply a landed move: damage and the stagger of its reaction.
    pub fn apply_hit(&mut self, spec: &MoveSpec, _attacker_facing: i8) -> HitOutcome {
        self.health = self.health.saturating_sub(spec.damage);
        if spec.reaction.stagger_ticks > 0 {
            self.stagger_ticks = spec.reaction.stagger_ticks;
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

    /// One fixed tick of enemy behaviour: hold a stagger, or chase the hero
    /// (the WORK-021 ground chaser: closer than 220 px and farther than 22 px,
    /// 62 px/s, stopped by solid geometry).
    pub fn step(&mut self, hero_x: f32, solids: &[Block]) {
        if self.health == 0 {
            self.defeated_ticks = self.defeated_ticks.saturating_add(1);
            return;
        }
        if self.stagger_ticks > 0 {
            self.stagger_ticks -= 1;
            return;
        }
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
}

pub(crate) fn body_rect(x: f32, y: f32) -> Rect {
    Rect {
        left: x - ENEMY_HALF_WIDTH,
        bottom: y - ENEMY_HALF_HEIGHT,
        width: ENEMY_HALF_WIDTH * 2.0,
        height: ENEMY_HALF_HEIGHT * 2.0,
    }
}

pub(crate) fn solid_at(solids: &[Block], candidate: Rect) -> bool {
    solids
        .iter()
        .any(|block| block.intact && candidate.overlaps(block.rect))
}

// ------------------------------------------------------------------ strikes --

/// Where an attacker stands: AABB centre and which way it faces.
///
/// `Default` is a camera-lab hook: callers write `Attacker { x, y, half_height,
/// facing, ..Default::default() }` so they keep compiling when another branch
/// adds a field.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Attacker {
    pub x: f32,
    pub y: f32,
    /// Half the attacker's box height, to find the feet from the centre.
    pub half_height: f32,
    pub facing: i8,
    /// The player slot credited for what this attacker defeats (0 is Carl).
    pub player: u32,
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

/// Something a strike can land on: the greybox [`Enemy`] and the scene
/// [`crate::goblin::Goblin`].
pub trait StrikeTarget {
    /// Has a hurtbox: a defeated target has none.
    fn hurtbox_active(&self) -> bool;
    /// Can take a hit right now: false while the target is frozen in hit-stop
    /// (a target is never hit again inside the freeze it just caused).
    fn can_be_hit(&self) -> bool {
        self.hurtbox_active()
    }
    fn body(&self) -> Rect;
    /// Centre of the body (the WORK-021 reach test measures from it).
    fn centre(&self) -> (f32, f32);
    /// Apply a landed move; returns the outcome and the hit-stop ticks the
    /// attacker must freeze for.
    fn take_hit(&mut self, spec: &MoveSpec, attacker_facing: i8) -> (HitOutcome, u8);
}

impl StrikeTarget for Enemy {
    fn hurtbox_active(&self) -> bool {
        self.alive()
    }

    fn body(&self) -> Rect {
        Enemy::body(self)
    }

    fn centre(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    fn take_hit(&mut self, spec: &MoveSpec, attacker_facing: i8) -> (HitOutcome, u8) {
        (self.apply_hit(spec, attacker_facing), 0)
    }
}

fn reaches(shape: &HitShape, attacker: &Attacker, target: &impl StrikeTarget) -> bool {
    match shape {
        HitShape::GreyboxReach {
            back,
            front,
            vertical,
        } => {
            let (tx, ty) = target.centre();
            let forward = (tx - attacker.x) * f32::from(attacker.facing);
            (*back..=*front).contains(&forward) && (ty - attacker.y).abs() <= *vertical
        }
        HitShape::Box { .. } => hit_box(shape, attacker).is_some_and(|b| b.overlaps(target.body())),
    }
}

/// Resolve the running move against `targets` (stable index order) on an
/// active tick. A target is hit at most once per move instance, on the first
/// active tick at which its body overlaps the hit box; defeated targets and
/// targets frozen in hit-stop are skipped, and a tick frozen by the attacker's
/// own hit-stop resolves nothing. A confirmed hit starts the attacker's
/// hit-stop (the longest of the targets hit this tick). Hits are applied and
/// logged to `events`.
pub fn resolve_strike<T: StrikeTarget>(
    runner: &mut MoveRunner,
    move_tick: &MoveTick,
    attacker: Attacker,
    targets: &mut [T],
    sim_tick: u64,
    events: &mut Vec<CombatEvent>,
) {
    if move_tick.phase != Phase::Active || move_tick.frozen {
        return;
    }
    let spec = move_tick.spec;
    let mut attacker_stop = 0u8;
    for (index, target) in targets.iter_mut().enumerate() {
        if !target.can_be_hit() || !reaches(&spec.shape, &attacker, target) {
            continue;
        }
        let id = index as u32;
        if !runner.record_hit(id) {
            continue;
        }
        let (outcome, stop) = target.take_hit(spec, attacker.facing);
        attacker_stop = attacker_stop.max(stop);
        events.push(CombatEvent::Hit {
            tick: sim_tick,
            move_id: spec.id,
            instance: move_tick.instance,
            target: id,
            damage: spec.damage,
            health_left: match outcome {
                HitOutcome::Hurt { health_left } => health_left,
                HitOutcome::Defeated => 0,
            },
            hit_stop: stop,
        });
        if outcome == HitOutcome::Defeated {
            events.push(CombatEvent::enemy_defeated(
                sim_tick,
                id,
                DefeatCause::Melee,
                Some(attacker.player),
                Some(spec),
            ));
        }
    }
    runner.begin_hit_stop(attacker_stop);
}

// ------------------------------------------------------------------- events --

/// How a target was defeated, with the credit (`player` 0 is Carl's slot).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefeatCause {
    /// A melee hit of the attacker (the CP-01 default).
    Melee,
    /// CP-03: the falling block, credited to the player who broke the shore.
    ShoreBlock { player: u32 },
}

impl DefeatCause {
    /// The force tier a cause implies when nothing stronger is recorded: a
    /// melee hit is routine, a falling block is catastrophic.
    pub const fn default_force(self) -> ForceTier {
        match self {
            Self::Melee => ForceTier::Routine,
            Self::ShoreBlock { .. } => ForceTier::Catastrophic,
        }
    }

    /// Every force tier a cause may legitimately record. `force` is not mixed
    /// into the replay hash because it is derived from hashed data, so a force
    /// outside this list is a bug (checked in [`CombatEvent::mix_into`]); a
    /// cause whose force is not derivable must mix it, as a documented hash
    /// migration. The exhaustive match forces a new cause to decide.
    pub const fn derived_forces(self) -> &'static [ForceTier] {
        match self {
            Self::Melee => &[ForceTier::Routine, ForceTier::Heavy],
            Self::ShoreBlock { .. } => &[ForceTier::Catastrophic],
        }
    }

    /// The instigator a cause implies when nothing else is recorded: Carl's
    /// slot for a melee defeat, nobody for the block (its breaker is part of
    /// the cause).
    pub const fn implied_instigator(self) -> Option<u32> {
        match self {
            Self::Melee => Some(0),
            Self::ShoreBlock { .. } => None,
        }
    }
}

/// The authoritative strength of a defeat (never a gore or intensity setting:
/// the presentation preset is not part of simulation state).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForceTier {
    Routine,
    Heavy,
    Catastrophic,
}

impl ForceTier {
    /// The tier of a lethal melee move: a launching move (the push kick) is
    /// heavy, every other strike routine.
    pub const fn of_move(spec: &MoveSpec) -> Self {
        if spec.reaction.launch {
            Self::Heavy
        } else {
            Self::Routine
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Routine => "routine",
            Self::Heavy => "heavy",
            Self::Catastrophic => "catastrophic",
        }
    }
}

/// Something that happened on a fixed tick, for deterministic replay logs.
///
/// Ticks are **attempt-local**: the scene tick counter starts again at 1 after
/// a restart. A [`CombatEvent::Restart`] carries the tick the finished attempt
/// ended on (the tick that triggered it), so a log reads as: events of attempt
/// 1 up to `Restart{tick: N}`, then the ticks of attempt 2 from 1.
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
        /// Ticks the attacker and the target freeze (0 for the greybox).
        hit_stop: u8,
    },
    /// The one authoritative defeat event, emitted once per alive -> dead
    /// transition. `cause` says how (melee by default; the falling block of
    /// CP-03 credits the player who broke the shore), `force` how hard
    /// ([`ForceTier`]) and `instigator` which player's action is behind it: the
    /// attacker for a melee defeat, and for a block the player who last hit
    /// the target while it lived (a kick that put it under the block), if any.
    /// Presentation data (gore, intensity, finish) never belongs in it.
    EnemyDefeated {
        tick: u64,
        target: u32,
        cause: DefeatCause,
        force: ForceTier,
        instigator: Option<u32>,
    },
    /// A goblin took an attack token and started its tell (attack tick 0).
    GoblinTell {
        tick: u64,
        goblin: u32,
    },
    /// A goblin's swing became active (attack tick 26).
    GoblinSwing {
        tick: u64,
        goblin: u32,
    },
    /// A hit cancelled a goblin's tell; its token went back.
    GoblinInterrupted {
        tick: u64,
        goblin: u32,
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
    /// CP-02: a launched living goblin (`source`) touched another (`target`).
    /// At most one per (`launch`, `target`); no damage.
    GoblinContact {
        tick: u64,
        launch: u32,
        source: u32,
        target: u32,
    },
    /// CP-03: a melee hit advanced the rotten shore (`hits` of 3 so far).
    ShoreHit {
        tick: u64,
        instance: u32,
        hits: u8,
    },
    /// CP-03: the third hit broke the shore; the block will fall.
    ShoreBroken {
        tick: u64,
        player: u32,
    },
    /// CP-03: the block landed; the hazard footprint was evaluated once.
    ShoreLanded {
        tick: u64,
    },
}

impl CombatEvent {
    /// The one constructor of the defeat event: the force tier is derived here
    /// (never passed in), from the lethal move for a melee defeat and from the
    /// cause otherwise, so a force source cannot skip the derivation rule.
    pub fn enemy_defeated(
        tick: u64,
        target: u32,
        cause: DefeatCause,
        instigator: Option<u32>,
        lethal_move: Option<&MoveSpec>,
    ) -> Self {
        let force = match (cause, lethal_move) {
            (DefeatCause::Melee, Some(spec)) => ForceTier::of_move(spec),
            _ => cause.default_force(),
        };
        Self::EnemyDefeated {
            tick,
            target,
            cause,
            force,
            instigator,
        }
    }

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
                hit_stop,
            } => {
                mix(2);
                mix(tick);
                mix(move_id as u64);
                mix(u64::from(instance));
                mix(u64::from(target));
                mix(u64::from(damage));
                mix(u64::from(health_left));
                mix(u64::from(hit_stop));
            }
            Self::EnemyDefeated {
                tick,
                target,
                cause,
                force,
                instigator,
            } => {
                debug_assert!(
                    cause.derived_forces().contains(&force),
                    "force {force:?} is not derivable from {cause:?}: mix it"
                );
                mix(3);
                mix(tick);
                mix(u64::from(target));
                // Mixed only for a non-melee cause: melee replays keep their hash.
                if let DefeatCause::ShoreBlock { player } = cause {
                    mix(0x5348_4f52);
                    mix(u64::from(player));
                }
                // DEATH-B: `force` is not mixed because it is derived from
                // hashed data: a melee defeat's tier is `ForceTier::of_move` of
                // the move id of the `Hit` mixed on the same tick, a block's is
                // constant (a test pins both). `instigator` is mixed only where
                // the cause does not imply it (a block that crushed a target a
                // player had hit), so every earlier replay keeps its hash.
                if instigator != cause.implied_instigator() {
                    mix(0x494e_5354);
                    mix(instigator.map_or(0, |p| u64::from(p) + 1));
                }
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
            Self::GoblinTell { tick, goblin } => {
                mix(7);
                mix(tick);
                mix(u64::from(goblin));
            }
            Self::GoblinSwing { tick, goblin } => {
                mix(8);
                mix(tick);
                mix(u64::from(goblin));
            }
            Self::GoblinInterrupted { tick, goblin } => {
                mix(9);
                mix(tick);
                mix(u64::from(goblin));
            }
            Self::GoblinContact {
                tick,
                launch,
                source,
                target,
            } => {
                mix(10);
                mix(tick);
                mix(u64::from(launch));
                mix(u64::from(source));
                mix(u64::from(target));
            }
            Self::ShoreHit {
                tick,
                instance,
                hits,
            } => {
                mix(11);
                mix(tick);
                mix(u64::from(instance));
                mix(u64::from(hits));
            }
            Self::ShoreBroken { tick, player } => {
                mix(12);
                mix(tick);
                mix(u64::from(player));
            }
            Self::ShoreLanded { tick } => {
                mix(13);
                mix(tick);
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
                hit_stop,
            } => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=hit move={} instance={instance} target={target} damage={damage} health_left={health_left} hit_stop={hit_stop}",
                move_id.name()
            ),
            Self::EnemyDefeated {
                tick,
                target,
                cause,
                force,
                instigator,
            } => {
                write!(
                    f,
                    "COMBAT_EVENT tick={tick} kind=enemy_defeated target={target}"
                )?;
                if let DefeatCause::ShoreBlock { player } = cause {
                    write!(f, " cause=shore_block credit_player={player}")?;
                }
                // Fields that are implied by the cause stay out of the line.
                if force != cause.default_force() {
                    write!(f, " force={}", force.name())?;
                }
                if instigator != cause.implied_instigator()
                    && let Some(player) = instigator
                {
                    write!(f, " instigator={player}")?;
                }
                Ok(())
            }
            Self::GoblinTell { tick, goblin } => {
                write!(
                    f,
                    "COMBAT_EVENT tick={tick} kind=goblin_tell goblin={goblin}"
                )
            }
            Self::GoblinSwing { tick, goblin } => {
                write!(
                    f,
                    "COMBAT_EVENT tick={tick} kind=goblin_swing goblin={goblin}"
                )
            }
            Self::GoblinInterrupted { tick, goblin } => {
                write!(
                    f,
                    "COMBAT_EVENT tick={tick} kind=goblin_interrupted goblin={goblin}"
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
            Self::GoblinContact {
                tick,
                launch,
                source,
                target,
            } => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=goblin_contact launch={launch} source={source} target={target}"
            ),
            Self::ShoreHit {
                tick,
                instance,
                hits,
            } => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=shore_hit instance={instance} hits={hits}"
            ),
            Self::ShoreBroken { tick, player } => {
                write!(
                    f,
                    "COMBAT_EVENT tick={tick} kind=shore_broken player={player}"
                )
            }
            Self::ShoreLanded { tick } => {
                write!(f, "COMBAT_EVENT tick={tick} kind=shore_landed")
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

    fn starts_of(
        spec: &'static MoveSpec,
        press_on: &[u64],
        buffer: u8,
        ticks: u64,
    ) -> Vec<(u64, MoveId)> {
        let mut runner = MoveRunner::default();
        let mut starts = Vec::new();
        for tick in 1..=ticks {
            let request = press_on.contains(&tick).then_some(spec);
            if let Some(m) = runner.step(request, buffer, true)
                && m.started
            {
                starts.push((tick, m.spec.id));
            }
        }
        starts
    }

    #[test]
    fn a_held_button_is_one_press_and_a_late_press_is_buffered_once() {
        // InputFrame turns a held button into a single press edge: one press
        // yields exactly one move instance however long the button is held.
        assert_eq!(
            starts_of(&CARL_HAYMAKER, &[1], CARL_BUFFER_TICKS, 60),
            vec![(1, MoveId::Haymaker)]
        );
        // The haymaker has no string: a press in the last CARL_BUFFER_TICKS of
        // the move starts the next move the tick the first one ends (tick 23:
        // total 22).
        assert_eq!(
            starts_of(&CARL_HAYMAKER, &[1, 19], CARL_BUFFER_TICKS, 60),
            vec![(1, MoveId::Haymaker), (23, MoveId::Haymaker)]
        );
    }

    #[test]
    fn the_buffer_window_boundary_is_exactly_six_ticks_before_the_move_ends() {
        // Move pressed on tick 1 ends on tick 23 (total 22). A press on tick p
        // is remembered until p + 6, so p = 17 still starts the next move on
        // tick 23 and p = 16 is forgotten.
        let at = |p: u64| starts_of(&CARL_HAYMAKER, &[1, p], CARL_BUFFER_TICKS, 60);
        assert_eq!(at(17), vec![(1, MoveId::Haymaker), (23, MoveId::Haymaker)]);
        assert_eq!(at(16), vec![(1, MoveId::Haymaker)]);
        // The last tick of the move (22) is inside the window, the move's own
        // press tick plus the whole length is not needed.
        assert_eq!(at(22), vec![(1, MoveId::Haymaker), (23, MoveId::Haymaker)]);
    }

    #[test]
    fn the_jab_string_window_is_ticks_3_to_13_and_the_haymaker_starts_at_tick_7_or_later() {
        // The jab is pressed on tick 1 (jab tick 0); a second attack press on
        // tick 1 + t is at jab tick t.
        let second = |t: u64| starts_of(&CARL_JAB, &[1, 1 + t], CARL_BUFFER_TICKS, 60);
        // Too early (jab ticks 0..=2): forgotten, only the jab.
        assert_eq!(second(2), vec![(1, MoveId::Jab)]);
        // Window start (jab tick 3): chained, but not before jab tick 7 =
        // tick 8.
        assert_eq!(second(3), vec![(1, MoveId::Jab), (8, MoveId::Haymaker)]);
        assert_eq!(second(6), vec![(1, MoveId::Jab), (8, MoveId::Haymaker)]);
        // From jab tick 7 the haymaker starts on the press tick itself.
        assert_eq!(second(7), vec![(1, MoveId::Jab), (8, MoveId::Haymaker)]);
        assert_eq!(second(9), vec![(1, MoveId::Jab), (10, MoveId::Haymaker)]);
        // Window end (jab tick 13).
        assert_eq!(second(13), vec![(1, MoveId::Jab), (14, MoveId::Haymaker)]);
        // Jab tick 14 does not exist: the jab is over, a press is a new jab.
        assert_eq!(second(14), vec![(1, MoveId::Jab), (15, MoveId::Jab)]);
        // The haymaker ends the string: a press during it is buffered, never a
        // third blow of the string; held buttons never chain.
        let starts = starts_of(&CARL_JAB, &[1, 4, 6], CARL_BUFFER_TICKS, 60);
        assert_eq!(starts, vec![(1, MoveId::Jab), (8, MoveId::Haymaker)]);
    }

    #[test]
    fn hit_stop_freezes_the_move_clock_and_a_press_during_it_still_counts() {
        let mut runner = MoveRunner::default();
        let mut log = Vec::new();
        for tick in 1..=14u64 {
            let request = (tick == 1 || tick == 6).then_some(&CARL_JAB);
            let m = runner.step(request, CARL_BUFFER_TICKS, true);
            if tick == 4 {
                // Jab tick 3 = the hit tick: freeze for 3 ticks.
                runner.begin_hit_stop(3);
            }
            log.push((tick, m.map(|m| (m.tick, m.frozen, m.started, m.spec.id))));
        }
        // Ticks 5, 6 and 7 are frozen on jab tick 3; the clock resumes on 8.
        let tick_of = |t: u64| log[t as usize - 1].1.unwrap();
        assert_eq!(tick_of(4), (3, false, false, MoveId::Jab));
        for frozen in 5..=7 {
            assert_eq!(tick_of(frozen).0, 3);
            assert!(tick_of(frozen).1, "tick {frozen} is frozen");
        }
        assert_eq!(tick_of(8), (4, false, false, MoveId::Jab));
        // The press on tick 6 (during the freeze, jab tick 3) chained: the
        // haymaker starts when the jab reaches tick 7 = tick 11.
        assert_eq!(tick_of(11), (0, false, true, MoveId::Haymaker));
    }

    #[test]
    fn a_frozen_tick_resolves_no_hit_and_a_defeated_target_stays_skipped() {
        let carl = facing_right_carl();
        let mut enemies = [goblin_at(125.0)];
        let mut runner = MoveRunner::default();
        let mut events = Vec::new();
        let mut m = None;
        for tick in 1..=4u64 {
            m = runner.step((tick == 1).then_some(&CARL_JAB), 0, true);
        }
        let mut frozen = m.unwrap();
        frozen.frozen = true;
        resolve_strike(&mut runner, &frozen, carl, &mut enemies, 4, &mut events);
        assert!(events.is_empty(), "a frozen tick hits nothing");
        let live = MoveTick {
            frozen: false,
            ..frozen
        };
        resolve_strike(&mut runner, &live, carl, &mut enemies, 4, &mut events);
        assert_eq!(events.len(), 1);
        // The attacker froze for the move's hit-stop only if the target
        // reported one: the greybox enemy reports none.
        assert_eq!(runner.hit_stop_left(), 0);
    }

    #[test]
    fn a_hit_lands_on_the_first_active_tick_at_which_the_box_overlaps() {
        // The goblin is out of reach on the first active tick (jab tick 3) and
        // steps into the box on the second (tick 4): that is the hit tick.
        let carl = facing_right_carl();
        let mut enemy = [goblin_at(100.0 + 37.0)]; // left edge +25: out of reach
        let mut runner = MoveRunner::default();
        let mut events = Vec::new();
        let mut hit_tick = None;
        for tick in 1..=8u64 {
            if tick == 5 {
                enemy[0].x = 100.0 + 33.0; // left edge +21: inside [16, 22)
            }
            if let Some(m) = runner.step((tick == 1).then_some(&CARL_JAB), 0, true) {
                resolve_strike(&mut runner, &m, carl, &mut enemy, tick, &mut events);
            }
            if hit_tick.is_none() && !events.is_empty() {
                hit_tick = Some(tick);
            }
        }
        // Move tick 3 is tick 4, move tick 4 is tick 5 (the second active tick).
        assert_eq!(hit_tick, Some(5));
        assert_eq!(events.len(), 1);
        // The same for the haymaker: out of reach on its first active tick
        // (move tick 5), in on its second.
        let mut enemy = [goblin_at(100.0 + 37.0)];
        let mut runner = MoveRunner::default();
        let mut events = Vec::new();
        let mut hit_tick = None;
        for tick in 1..=12u64 {
            if tick == 8 {
                enemy[0].x = 100.0 + 33.0;
            }
            if let Some(m) = runner.step((tick == 1).then_some(&CARL_HAYMAKER), 0, true) {
                resolve_strike(&mut runner, &m, carl, &mut enemy, tick, &mut events);
            }
            if hit_tick.is_none() && !events.is_empty() {
                hit_tick = Some(tick);
            }
        }
        // Haymaker tick 5 is tick 6; tick 8 is its third active tick, the
        // first at which the goblin overlaps.
        assert_eq!(hit_tick, Some(8));
    }

    #[test]
    fn the_move_table_is_the_lookup_and_matches_the_art_boxes() {
        for spec in CARL_MOVES {
            assert_eq!(carl_move(spec.id), Some(spec));
        }
        assert_eq!(carl_move(MoveId::GreyboxStrike), None);
        let boxed = |spec: &MoveSpec| match spec.shape {
            HitShape::Box {
                x_min,
                x_max,
                y_min,
                y_max,
            } => (x_min, x_max, y_min, y_max),
            HitShape::GreyboxReach { .. } => unreachable!(),
        };
        // PR #43 reach boxes after the redesign (half-open).
        assert_eq!(boxed(&CARL_JAB), (16.0, 22.0, 29.0, 35.0));
        assert_eq!(boxed(&CARL_HAYMAKER), (16.0, 22.0, 27.0, 38.0));
        assert_eq!(boxed(&CARL_PUSH_KICK), (15.0, 22.0, 18.0, 28.0));
        assert_eq!(
            (
                CARL_HAYMAKER.startup,
                CARL_HAYMAKER.active,
                CARL_HAYMAKER.recovery
            ),
            (5, 3, 14)
        );
        assert_eq!(CARL_HAYMAKER.total(), 22);
        // The haymaker steps Carl 4 px, 1 px per tick on move ticks 1..=4; the
        // other moves keep the feet planted.
        let step = CARL_HAYMAKER.root_step.unwrap();
        assert_eq!(
            (step.from_tick, step.to_tick, step.px_per_tick),
            (1, 4, 1.0)
        );
        assert!(CARL_JAB.root_step.is_none() && CARL_PUSH_KICK.root_step.is_none());
        // Camera kick: heavy blows only.
        assert_eq!(CARL_JAB.reaction.camera_kick_px, 0);
        assert_eq!(CARL_HAYMAKER.reaction.camera_kick_px, 2);
        // Hit-stop values of the Carl melee spec section 4.
        assert_eq!(CARL_JAB.reaction.hit_stop, 3);
        assert_eq!(CARL_HAYMAKER.reaction.hit_stop, 5);
        assert_eq!(CARL_PUSH_KICK.reaction.hit_stop, 8);
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
            player: 0,
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
    fn a_hit_staggers_and_a_staggered_enemy_deals_no_contact_damage() {
        let mut enemy = Enemy::new(200.0, 119.0);
        enemy.apply_hit(&CARL_JAB, 1);
        assert_eq!(enemy.stagger_ticks, 10);
        let mut health = HERO_HEALTH;
        let mut invulnerable = 0;
        // Staggered: standing on the hero does no damage.
        assert!(!enemy.contact(enemy.x, enemy.y, &mut health, &mut invulnerable));
        for _ in 0..10 {
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
