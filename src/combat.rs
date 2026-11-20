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

use crate::boss::BossMove;
use crate::donut_mana::{ManaEvent, RejectReason, Unlock};
use crate::donut_moves::GlanceReason;
use crate::feel::{
    HAYMAKER_FEEL, HAYMAKER_STEP_PX, HAYMAKER_STEP_TICKS, JAB_FEEL, KICK_FEEL, MISSILE_FEEL,
    MoveFeel, POUNCE_FEEL, SWAT_FEEL,
};
use crate::party::HeroId;
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
    // Appended after `PushKick` so every Carl hash keeps its value. Donut's
    // moves (WORK-023): their timing lives in `crate::donut_moves`, the specs
    // below only carry the hit data `take_hit` needs.
    Swat,
    Pounce,
    MagicMissile,
}

impl MoveId {
    pub fn name(self) -> &'static str {
        match self {
            Self::GreyboxStrike => "greybox_strike",
            Self::Jab => "jab",
            Self::Haymaker => "haymaker",
            Self::PushKick => "push_kick",
            Self::Swat => "swat",
            Self::Pounce => "pounce",
            Self::MagicMissile => "magic_missile",
        }
    }

    /// The hero index that owns this move: Donut's three moves are hero 1,
    /// every other move is Carl's (hero 0). `CombatEvent::Hit` names no
    /// attacker, so anything that needs the attacker of a hit (the CINE launch
    /// credit) reads it from here instead of defaulting to hero 0.
    pub const fn hero(self) -> u32 {
        match self {
            Self::Swat | Self::Pounce | Self::MagicMissile => crate::donut_moves::DONUT_HERO,
            Self::GreyboxStrike | Self::Jab | Self::Haymaker | Self::PushKick => 0,
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

/// Donut's swat (Q10-02..05): startup 4, active 3, recovery 7; the box
/// `[13, 24) x [5, 11)` at shin height; damage 0. A swat on a tell cancels it
/// like a jab does (stagger 10, token back), see [`SWAT_FEEL`]. A second press
/// at ticks 4..=13 chains a swat at tick 10 (Q10-07). The timing and the box
/// are owned by `crate::donut_moves` (a test pins the two equal).
pub const DONUT_SWAT: MoveSpec = MoveSpec {
    id: MoveId::Swat,
    startup: 4,
    active: 3,
    recovery: 7,
    shape: HitShape::Box {
        x_min: 13.0,
        x_max: 24.0,
        y_min: 5.0,
        y_max: 11.0,
    },
    damage: 0,
    reaction: Reaction::from_feel(SWAT_FEEL),
    chain: Some(Chain {
        on: MoveId::Swat,
        into: MoveId::Swat,
        press_from: 4,
        press_to: 13,
        earliest: 10,
    }),
    root_step: None,
};

/// Donut's pounce (Q10-10, 11, 13, 14): crouch 10, flight, a pin and a recovery
/// of 18. The `MoveSpec` is the flat-floor nominal (10 / 16 / 18); the scene
/// ends the flight on touchdown. Box `[11, 25) x [0, 17)`, damage 1.
pub const DONUT_POUNCE: MoveSpec = MoveSpec {
    id: MoveId::Pounce,
    startup: 10,
    active: 16,
    recovery: 18,
    shape: HitShape::Box {
        x_min: 11.0,
        x_max: 25.0,
        y_min: 0.0,
        y_max: 17.0,
    },
    damage: 1,
    reaction: Reaction::from_feel(POUNCE_FEEL),
    chain: None,
    root_step: None,
};

/// Donut's Magic Missile (mana v0): release on cast tick 4 (the bolt spawns),
/// 16 ticks per cast. The bolt box is 8 x 6; damage 1.
pub const MAGIC_MISSILE: MoveSpec = MoveSpec {
    id: MoveId::MagicMissile,
    startup: 4,
    active: 1,
    recovery: 11,
    shape: HitShape::Box {
        x_min: 0.0,
        x_max: 8.0,
        y_min: 0.0,
        y_max: 6.0,
    },
    damage: 1,
    reaction: Reaction::from_feel(MISSILE_FEEL),
    chain: None,
    root_step: None,
};

/// Donut's move table (the hit data of the swat, the pounce and the missile).
pub const DONUT_MOVES: [&MoveSpec; 3] = [&DONUT_SWAT, &DONUT_POUNCE, &MAGIC_MISSILE];

/// The Donut move with this id (`None` for every Carl move).
pub fn donut_move(id: MoveId) -> Option<&'static MoveSpec> {
    DONUT_MOVES.iter().copied().find(|spec| spec.id == id)
}

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
    pub(crate) fn record_hit(&mut self, target: u32) -> bool {
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
    /// The health the glance rule of [`strike_group`] sees. The default
    /// (`u8::MAX`) means "never lethal"; every target with hit points overrides it.
    fn health_for_glance(&self) -> u8 {
        u8::MAX
    }
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

    fn health_for_glance(&self) -> u8 {
        self.health
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
    let stop = strike_group(
        runner,
        move_tick,
        attacker,
        targets,
        0,
        sim_tick,
        events,
        &|spec, _hp| *spec,
    );
    runner.begin_hit_stop(stop);
}

/// One group of targets of [`resolve_strike`]: the target ids are
/// `first_id + index`, so a scene with several kinds of target (goblins first,
/// then llamas) resolves one move over all of them with stable ids. Returns the
/// hit-stop the attacker must freeze for (the longest of the hits in this
/// group); the caller starts it once, after every group
/// ([`MoveRunner::begin_hit_stop`] keeps the longest). `glance` maps the move
/// and the target's health to the move actually applied (the identity outside
/// the companion's glance rule).
#[allow(clippy::too_many_arguments)]
pub fn strike_group<T: StrikeTarget>(
    runner: &mut MoveRunner,
    move_tick: &MoveTick,
    attacker: Attacker,
    targets: &mut [T],
    first_id: u32,
    sim_tick: u64,
    events: &mut Vec<CombatEvent>,
    glance: &dyn Fn(&MoveSpec, u8) -> MoveSpec,
) -> u8 {
    if move_tick.phase != Phase::Active || move_tick.frozen {
        return 0;
    }
    let spec = move_tick.spec;
    let mut attacker_stop = 0u8;
    for (index, target) in targets.iter_mut().enumerate() {
        if !target.can_be_hit() || !reaches(&spec.shape, &attacker, target) {
            continue;
        }
        let id = first_id + index as u32;
        if !runner.record_hit(id) {
            continue;
        }
        let landed = glance(spec, target.health_for_glance());
        let (outcome, stop) = target.take_hit(&landed, attacker.facing);
        attacker_stop = attacker_stop.max(stop);
        events.push(CombatEvent::Hit {
            tick: sim_tick,
            move_id: spec.id,
            instance: move_tick.instance,
            target: id,
            damage: landed.damage,
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
    attacker_stop
}

// ------------------------------------------------------------------- events --

/// How a target was defeated, with the credit (`player` 0 is Carl's slot).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefeatCause {
    /// A melee hit of the attacker (the CP-01 default).
    Melee,
    /// CP-03: the falling block, credited to the player who broke the shore.
    ShoreBlock { player: u32 },
    /// HUD-DEBT-03: a falling plate rack (the Floor 01 boss arena), credited
    /// to the player who dropped it. A new variant beside `ShoreBlock`, so the
    /// CP-03 hashes and goldens stay byte-identical (no hash migration).
    DropHazard { player: u32, rack: u8 },
    /// DEATH-D: the block's vertical edge came down across an upright
    /// target's neck (head under the stone, torso outside it). The same
    /// landing and the same state change as [`Self::ShoreBlock`]; only the
    /// cause differs, so a shear finish can be selected.
    ShoreBlockEdge { player: u32 },
    /// DEATH-D: a blast charge reached the target. `point_blank` is the
    /// charge centre inside the target's body box (the burst); otherwise the
    /// target was within the blast radius (thrown). `player` is the charge's
    /// owner, if any (a QA fixture charge has none).
    Explosion {
        charge: u32,
        player: Option<u32>,
        point_blank: bool,
    },
}

impl DefeatCause {
    /// The force tier a cause implies when nothing stronger is recorded: a
    /// melee hit is routine, a falling block (or its edge) is catastrophic, a
    /// point blank blast is catastrophic and a blast ring heavy.
    pub const fn default_force(self) -> ForceTier {
        match self {
            Self::Melee => ForceTier::Routine,
            Self::ShoreBlock { .. } | Self::ShoreBlockEdge { .. } | Self::DropHazard { .. } => {
                ForceTier::Catastrophic
            }
            Self::Explosion { point_blank, .. } => {
                if point_blank {
                    ForceTier::Catastrophic
                } else {
                    ForceTier::Heavy
                }
            }
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
            Self::ShoreBlock { .. } | Self::ShoreBlockEdge { .. } | Self::DropHazard { .. } => {
                &[ForceTier::Catastrophic]
            }
            Self::Explosion { point_blank, .. } => {
                if point_blank {
                    &[ForceTier::Catastrophic]
                } else {
                    &[ForceTier::Heavy]
                }
            }
        }
    }

    /// The player credited with a **player-activated drop hazard** defeat: the
    /// falling shore block, its DEATH-D edge shear and the boss arena's plate
    /// rack. `None` for every other cause. Cause-class matchers (CINE-01 F7)
    /// use this one helper rather than naming a variant, so a future drop
    /// hazard gets credit by being added here.
    pub const fn drop_credit(self) -> Option<u32> {
        match self {
            Self::ShoreBlock { player }
            | Self::ShoreBlockEdge { player }
            | Self::DropHazard { player, .. } => Some(player),
            Self::Melee | Self::Explosion { .. } => None,
        }
    }

    /// The instigator a cause implies when nothing else is recorded: Carl's
    /// slot for a melee defeat, nobody for the block (its breaker is part of
    /// the cause) and the dropping player for a rack. The asymmetry is kept
    /// on purpose: this value decides whether `instigator` is mixed into the
    /// replay hash, so changing it for the blocks would be a hash migration.
    /// Use [`Self::drop_credit`] to ask who a drop hazard is credited to.
    pub const fn implied_instigator(self) -> Option<u32> {
        match self {
            Self::Melee => Some(0),
            Self::ShoreBlock { .. } | Self::ShoreBlockEdge { .. } | Self::Explosion { .. } => None,
            Self::DropHazard { player, .. } => Some(player),
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
    /// A goblin's swing hurt a hero. `hero` is Carl unless the party is live
    /// and Donut was hit; Carl's hits hash and print exactly as before.
    HeroHurt {
        tick: u64,
        source: u32,
        health_left: u8,
        hero: HeroId,
    },
    /// A hero reached 0 health: a party defeat (D1), whichever hero it was.
    HeroDefeated {
        tick: u64,
        hero: HeroId,
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
    /// DEMO-01 PR B: a Bad Llama started its tell (attack tick 0). `llama` is
    /// the index among the llamas (its target id is the goblin count plus it).
    LlamaTell {
        tick: u64,
        llama: u32,
    },
    /// A llama released its spit glob (attack tick 36).
    LlamaSpit {
        tick: u64,
        llama: u32,
    },
    /// A hit cancelled a llama's tell.
    LlamaInterrupted {
        tick: u64,
        llama: u32,
    },
    /// DEATH-D: a blast charge detonated (once, on its fixture tick); the
    /// defeats it caused follow as `EnemyDefeated` events.
    Blast {
        tick: u64,
        charge: u32,
    },
    /// DEMO-01: Carl stood up again at checkpoint `checkpoint` (the demo's
    /// respawn: defeated enemies stay defeated). Emitted only in the demo.
    Respawn {
        tick: u64,
        checkpoint: u32,
    },
    /// HUD-DEBT-03: the boss gate closed (every living hero inside).
    BossEngaged {
        tick: u64,
    },
    /// The boss started a tell on `target` (the telegraph flag is up).
    BossTell {
        tick: u64,
        mv: BossMove,
        target: HeroId,
    },
    /// The tell ended and the move became active.
    BossStrike {
        tick: u64,
        mv: BossMove,
    },
    /// The charge hit the wall on side `wall` (-1 left, +1 right).
    BossCrash {
        tick: u64,
        wall: i8,
    },
    /// A threshold roar began: the boss is now in `phase`.
    BossPhase {
        tick: u64,
        phase: u8,
    },
    /// The boss took `damage` from player `source`; `hp_left` remains.
    BossHurt {
        tick: u64,
        source: u32,
        damage: u8,
        hp_left: u8,
    },
    /// A rack was armed when the gate closed.
    RackArmed {
        tick: u64,
        rack: u8,
    },
    /// A melee hit on a rack's strut (`hits` so far).
    RackHit {
        tick: u64,
        rack: u8,
        hits: u8,
    },
    /// A rack started to fall; `player` dropped it.
    RackDropped {
        tick: u64,
        rack: u8,
        player: u32,
    },
    /// A rack landed; `crushed` when the boss was under it.
    RackLanded {
        tick: u64,
        rack: u8,
        crushed: bool,
    },
    /// WORK-023: a Donut swat that did nothing but flash (a glance): no
    /// stagger, no hit-stop, no state change (Q10 section 1).
    DonutGlance {
        tick: u64,
        instance: u32,
        target: u32,
        reason: GlanceReason,
    },
    /// HUD-DEBT-02: Donut's biscuit unlock and mana pool (the capped log
    /// events of the mana v0 proposal, section 5).
    Mana(ManaEvent),
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
                match cause {
                    DefeatCause::Melee | DefeatCause::DropHazard { .. } => {}
                    DefeatCause::ShoreBlock { player } => {
                        mix(0x5348_4f52);
                        mix(u64::from(player));
                    }
                    // DEATH-D tags ("EDGE", "BLST"): new words only for new causes.
                    DefeatCause::ShoreBlockEdge { player } => {
                        mix(0x4544_4745);
                        mix(u64::from(player));
                    }
                    DefeatCause::Explosion {
                        charge,
                        player,
                        point_blank,
                    } => {
                        mix(0x424c_5354);
                        mix(u64::from(charge));
                        mix(player.map_or(0, |p| u64::from(p) + 1));
                        mix(u64::from(point_blank));
                    }
                }
                if let DefeatCause::DropHazard { player, rack } = cause {
                    mix(0x4452_4f50);
                    mix(u64::from(player));
                    mix(u64::from(rack));
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
                hero,
            } => {
                mix(4);
                mix(tick);
                mix(u64::from(source));
                mix(u64::from(health_left));
                // Only Donut's hits carry the hero, so every Carl hash is unchanged.
                if hero != HeroId::Carl {
                    mix(0x4452_4854 + hero.index() as u64);
                }
            }
            Self::HeroDefeated { tick, hero } => {
                mix(5);
                mix(tick);
                if hero != HeroId::Carl {
                    mix(0x4452_4854 + hero.index() as u64);
                }
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
            // Tags 40 and up: the llama events (PR B), apart from the tags
            // other in-flight work orders append after 13.
            Self::LlamaTell { tick, llama } => {
                mix(40);
                mix(tick);
                mix(u64::from(llama));
            }
            Self::LlamaSpit { tick, llama } => {
                mix(41);
                mix(tick);
                mix(u64::from(llama));
            }
            Self::LlamaInterrupted { tick, llama } => {
                mix(42);
                mix(tick);
                mix(u64::from(llama));
            }
            Self::Blast { tick, charge } => {
                mix(14);
                mix(tick);
                mix(u64::from(charge));
            }
            Self::Respawn { tick, checkpoint } => {
                // A tag far from the combat tags: DEMO-01 only.
                mix(0x4445_4d4f);
                mix(tick);
                mix(u64::from(checkpoint));
            }
            // The boss encounter (HUD-DEBT-03): tags in their own range, so no
            // earlier replay hashes any of them.
            Self::BossEngaged { tick } => {
                mix(0x4255_0001);
                mix(tick);
            }
            Self::BossTell { tick, mv, target } => {
                mix(0x4255_0002);
                mix(tick);
                mix(mv as u64);
                mix(target.index() as u64);
            }
            Self::BossStrike { tick, mv } => {
                mix(0x4255_0003);
                mix(tick);
                mix(mv as u64);
            }
            Self::BossCrash { tick, wall } => {
                mix(0x4255_0004);
                mix(tick);
                mix(wall as i64 as u64);
            }
            Self::BossPhase { tick, phase } => {
                mix(0x4255_0005);
                mix(tick);
                mix(u64::from(phase));
            }
            Self::BossHurt {
                tick,
                source,
                damage,
                hp_left,
            } => {
                mix(0x4255_0006);
                mix(tick);
                mix(u64::from(source));
                mix(u64::from(damage));
                mix(u64::from(hp_left));
            }
            Self::RackArmed { tick, rack } => {
                mix(0x4255_0007);
                mix(tick);
                mix(u64::from(rack));
            }
            Self::RackHit { tick, rack, hits } => {
                mix(0x4255_0008);
                mix(tick);
                mix(u64::from(rack));
                mix(u64::from(hits));
            }
            Self::RackDropped { tick, rack, player } => {
                mix(0x4255_0009);
                mix(tick);
                mix(u64::from(rack));
                mix(u64::from(player));
            }
            Self::RackLanded {
                tick,
                rack,
                crushed,
            } => {
                mix(0x4255_000a);
                mix(tick);
                mix(u64::from(rack));
                mix(u64::from(crushed));
            }
            // WORK-023: the Donut moves and the mana events, each with a tag in
            // its own range ("DONG", "DONM"), so no earlier replay hashes any.
            Self::DonutGlance {
                tick,
                instance,
                target,
                reason,
            } => {
                mix(0x444f_4e47_0001);
                mix(tick);
                mix(u64::from(instance));
                mix(u64::from(target));
                mix(reason as u64);
            }
            Self::Mana(event) => match event {
                ManaEvent::UnlockGranted { tick, hero, .. } => {
                    mix(0x444f_4e4d_0001);
                    mix(tick);
                    mix(u64::from(hero));
                }
                ManaEvent::ManaSpent { tick, hero, left } => {
                    mix(0x444f_4e4d_0002);
                    mix(tick);
                    mix(u64::from(hero));
                    mix(u64::from(left));
                }
                ManaEvent::ManaRegained { tick, hero, now } => {
                    mix(0x444f_4e4d_0003);
                    mix(tick);
                    mix(u64::from(hero));
                    mix(u64::from(now));
                }
                ManaEvent::CastRejected { tick, hero, .. } => {
                    mix(0x444f_4e4d_0004);
                    mix(tick);
                    mix(u64::from(hero));
                }
            },
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
                match cause {
                    DefeatCause::Melee | DefeatCause::DropHazard { .. } => {}
                    DefeatCause::ShoreBlock { player } => {
                        write!(f, " cause=shore_block credit_player={player}")?;
                    }
                    DefeatCause::ShoreBlockEdge { player } => {
                        write!(f, " cause=shore_block_edge credit_player={player}")?;
                    }
                    DefeatCause::Explosion {
                        charge,
                        player,
                        point_blank,
                    } => {
                        write!(f, " cause=explosion charge={charge}")?;
                        if let Some(player) = player {
                            write!(f, " credit_player={player}")?;
                        }
                        if point_blank {
                            write!(f, " point_blank")?;
                        }
                    }
                }
                if let DefeatCause::DropHazard { player, rack } = cause {
                    write!(f, " cause=drop_hazard credit_player={player} rack={rack}")?;
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
                hero,
            } => {
                write!(
                    f,
                    "COMBAT_EVENT tick={tick} kind=hero_hurt source={source} health_left={health_left}"
                )?;
                if hero != HeroId::Carl {
                    write!(f, " hero=donut")?;
                }
                Ok(())
            }
            Self::HeroDefeated { tick, hero } => {
                write!(f, "COMBAT_EVENT tick={tick} kind=hero_defeated")?;
                if hero != HeroId::Carl {
                    write!(f, " hero=donut")?;
                }
                Ok(())
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
            Self::LlamaTell { tick, llama } => {
                write!(f, "COMBAT_EVENT tick={tick} kind=llama_tell llama={llama}")
            }
            Self::LlamaSpit { tick, llama } => {
                write!(f, "COMBAT_EVENT tick={tick} kind=llama_spit llama={llama}")
            }
            Self::LlamaInterrupted { tick, llama } => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=llama_interrupted llama={llama}"
            ),
            Self::Blast { tick, charge } => {
                write!(f, "COMBAT_EVENT tick={tick} kind=blast charge={charge}")
            }
            Self::Respawn { tick, checkpoint } => {
                write!(
                    f,
                    "COMBAT_EVENT tick={tick} kind=respawn checkpoint={checkpoint}"
                )
            }
            Self::BossEngaged { tick } => write!(f, "COMBAT_EVENT tick={tick} kind=boss_engaged"),
            Self::BossTell { tick, mv, target } => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=boss_tell move={} target={}",
                mv.name(),
                if target == HeroId::Carl {
                    "carl"
                } else {
                    "donut"
                }
            ),
            Self::BossStrike { tick, mv } => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=boss_strike move={}",
                mv.name()
            ),
            Self::BossCrash { tick, wall } => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=boss_crash wall={}",
                if wall < 0 { "left" } else { "right" }
            ),
            Self::BossPhase { tick, phase } => {
                write!(f, "COMBAT_EVENT tick={tick} kind=boss_phase phase={phase}")
            }
            Self::BossHurt {
                tick,
                source,
                damage,
                hp_left,
            } => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=boss_hurt source={source} damage={damage} hp_left={hp_left}"
            ),
            Self::RackArmed { tick, rack } => {
                write!(f, "COMBAT_EVENT tick={tick} kind=rack_armed rack={rack}")
            }
            Self::RackHit { tick, rack, hits } => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=rack_hit rack={rack} hits={hits}"
            ),
            Self::RackDropped { tick, rack, player } => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=rack_dropped rack={rack} player={player}"
            ),
            Self::RackLanded {
                tick,
                rack,
                crushed,
            } => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=rack_landed rack={rack} crushed={}",
                u8::from(crushed)
            ),
            Self::DonutGlance {
                tick,
                instance,
                target,
                reason,
            } => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=glance instance={instance} target={target} reason={}",
                reason.name()
            ),
            Self::Mana(ManaEvent::UnlockGranted { tick, hero, unlock }) => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=unlock_granted hero={hero} unlock={}",
                match unlock {
                    Unlock::PrimaryRanged => "primary_ranged",
                }
            ),
            Self::Mana(ManaEvent::ManaSpent { tick, hero, left }) => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=mana_spent hero={hero} left={left}"
            ),
            Self::Mana(ManaEvent::ManaRegained { tick, hero, now }) => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=mana_regained hero={hero} now={now}"
            ),
            Self::Mana(ManaEvent::CastRejected { tick, hero, reason }) => write!(
                f,
                "COMBAT_EVENT tick={tick} kind=cast_rejected hero={hero} reason={}",
                match reason {
                    RejectReason::Dry => "dry",
                }
            ),
        }
    }
}

// -------------------------------------------------------------------- tests --

#[cfg(test)]
mod tests {
    use super::*;

    fn event_hash(event: &CombatEvent) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        event.mix_into(&mut |value| {
            for byte in value.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        });
        hash
    }

    #[test]
    fn carls_hurt_and_defeat_events_print_and_hash_as_before_and_donuts_differ() {
        let carl = CombatEvent::HeroHurt {
            tick: 9,
            source: 1,
            health_left: 3,
            hero: HeroId::Carl,
        };
        let donut = CombatEvent::HeroHurt {
            tick: 9,
            source: 1,
            health_left: 3,
            hero: HeroId::Donut,
        };
        assert_eq!(
            carl.to_string(),
            "COMBAT_EVENT tick=9 kind=hero_hurt source=1 health_left=3"
        );
        assert_eq!(
            donut.to_string(),
            "COMBAT_EVENT tick=9 kind=hero_hurt source=1 health_left=3 hero=donut"
        );
        assert_ne!(event_hash(&carl), event_hash(&donut));
        let carl = CombatEvent::HeroDefeated {
            tick: 9,
            hero: HeroId::Carl,
        };
        let donut = CombatEvent::HeroDefeated {
            tick: 9,
            hero: HeroId::Donut,
        };
        assert_eq!(carl.to_string(), "COMBAT_EVENT tick=9 kind=hero_defeated");
        assert_eq!(
            donut.to_string(),
            "COMBAT_EVENT tick=9 kind=hero_defeated hero=donut"
        );
        assert_ne!(event_hash(&carl), event_hash(&donut));
    }

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
    fn strike_group_with_the_identity_glance_logs_the_same_events_as_resolve_strike() {
        let play = |glance: &dyn Fn(&MoveSpec, u8) -> MoveSpec| {
            let carl = facing_right_carl();
            let mut enemies = [goblin_at(125.0)];
            let mut runner = MoveRunner::default();
            let mut events = Vec::new();
            for tick in 1..=6u64 {
                if let Some(m) = runner.step((tick == 1).then_some(&CARL_JAB), 0, true) {
                    let stop = strike_group(
                        &mut runner,
                        &m,
                        carl,
                        &mut enemies,
                        0,
                        tick,
                        &mut events,
                        glance,
                    );
                    runner.begin_hit_stop(stop);
                }
            }
            (events, enemies[0].health)
        };
        let reference = {
            let carl = facing_right_carl();
            let mut enemies = [goblin_at(125.0)];
            let mut runner = MoveRunner::default();
            let mut events = Vec::new();
            for tick in 1..=6u64 {
                if let Some(m) = runner.step((tick == 1).then_some(&CARL_JAB), 0, true) {
                    resolve_strike(&mut runner, &m, carl, &mut enemies, tick, &mut events);
                }
            }
            (events, enemies[0].health)
        };
        assert_eq!(play(&|spec, _hp| *spec), reference);
        assert!(!reference.0.is_empty(), "the jab connected");
        // The hook is real: a glance that zeroes the damage reaches the target
        // and the log (the shape CA-20 uses).
        let (events, health) = play(&|spec, _hp| MoveSpec { damage: 0, ..*spec });
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CombatEvent::Hit { damage: 0, .. }))
        );
        assert_eq!(health, ENEMY_HEALTH, "a glance deals no damage");
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

    // ---- DEATH-D: the new causes, the blast event and the pinned words ----

    const EDGE_CAUSE: DefeatCause = DefeatCause::ShoreBlockEdge { player: 0 };
    const BLAST_PB: DefeatCause = DefeatCause::Explosion {
        charge: 2,
        player: Some(1),
        point_blank: true,
    };
    const BLAST_RING: DefeatCause = DefeatCause::Explosion {
        charge: 2,
        player: None,
        point_blank: false,
    };

    fn defeated(cause: DefeatCause, instigator: Option<u32>) -> CombatEvent {
        CombatEvent::enemy_defeated(9, 3, cause, instigator, None)
    }

    fn words(event: &CombatEvent) -> Vec<u64> {
        let mut out = Vec::new();
        event.mix_into(&mut |w| out.push(w));
        out
    }

    #[test]
    fn the_new_causes_derive_force_forces_and_instigator() {
        assert_eq!(EDGE_CAUSE.default_force(), ForceTier::Catastrophic);
        assert_eq!(EDGE_CAUSE.derived_forces(), [ForceTier::Catastrophic]);
        assert_eq!(EDGE_CAUSE.implied_instigator(), None);
        assert_eq!(BLAST_PB.default_force(), ForceTier::Catastrophic);
        assert_eq!(BLAST_PB.derived_forces(), [ForceTier::Catastrophic]);
        assert_eq!(BLAST_RING.default_force(), ForceTier::Heavy);
        assert_eq!(BLAST_RING.derived_forces(), [ForceTier::Heavy]);
        assert_eq!(BLAST_PB.implied_instigator(), None);
        assert_eq!(BLAST_RING.implied_instigator(), None);
        // The one constructor derives every tier (never passed in).
        for (cause, want) in [
            (EDGE_CAUSE, ForceTier::Catastrophic),
            (BLAST_PB, ForceTier::Catastrophic),
            (BLAST_RING, ForceTier::Heavy),
        ] {
            let CombatEvent::EnemyDefeated { force, .. } = defeated(cause, None) else {
                unreachable!();
            };
            assert_eq!(force, want, "{cause:?}");
            assert!(cause.derived_forces().contains(&force));
        }
    }

    #[test]
    fn the_hash_words_of_every_existing_variant_are_pinned_literally() {
        // Proof that DEATH-D changed no earlier word.
        let jab = MoveId::Jab as u64;
        assert_eq!(
            words(&CombatEvent::MoveStarted {
                tick: 5,
                move_id: MoveId::Jab,
                instance: 2
            }),
            [1, 5, jab, 2]
        );
        assert_eq!(
            words(&CombatEvent::Hit {
                tick: 5,
                move_id: MoveId::Jab,
                instance: 2,
                target: 1,
                damage: 1,
                health_left: 0,
                hit_stop: 4
            }),
            [2, 5, jab, 2, 1, 1, 0, 4]
        );
        assert_eq!(words(&defeated(DefeatCause::Melee, Some(0))), [3, 9, 3]);
        assert_eq!(
            words(&defeated(DefeatCause::ShoreBlock { player: 0 }, None)),
            [3, 9, 3, 0x5348_4f52, 0]
        );
        assert_eq!(
            words(&defeated(DefeatCause::ShoreBlock { player: 0 }, Some(0))),
            [3, 9, 3, 0x5348_4f52, 0, 0x494e_5354, 1]
        );
        assert_eq!(
            words(&CombatEvent::HeroHurt {
                tick: 5,
                source: 1,
                health_left: 3,
                hero: HeroId::Carl
            }),
            [4, 5, 1, 3]
        );
        assert_eq!(
            words(&CombatEvent::HeroDefeated {
                tick: 5,
                hero: HeroId::Carl
            }),
            [5, 5]
        );
        assert_eq!(
            words(&CombatEvent::Restart { tick: 5, deaths: 2 }),
            [6, 5, 2]
        );
        assert_eq!(
            words(&CombatEvent::GoblinTell { tick: 5, goblin: 1 }),
            [7, 5, 1]
        );
        assert_eq!(
            words(&CombatEvent::GoblinSwing { tick: 5, goblin: 1 }),
            [8, 5, 1]
        );
        assert_eq!(
            words(&CombatEvent::GoblinInterrupted { tick: 5, goblin: 1 }),
            [9, 5, 1]
        );
        assert_eq!(
            words(&CombatEvent::GoblinContact {
                tick: 5,
                launch: 4,
                source: 1,
                target: 2
            }),
            [10, 5, 4, 1, 2]
        );
        assert_eq!(
            words(&CombatEvent::ShoreHit {
                tick: 5,
                instance: 2,
                hits: 3
            }),
            [11, 5, 2, 3]
        );
        assert_eq!(
            words(&CombatEvent::ShoreBroken { tick: 5, player: 0 }),
            [12, 5, 0]
        );
        assert_eq!(words(&CombatEvent::ShoreLanded { tick: 5 }), [13, 5]);
    }

    #[test]
    fn the_hash_words_of_the_new_causes_and_the_blast_event() {
        assert_eq!(
            words(&defeated(EDGE_CAUSE, None)),
            [3, 9, 3, 0x4544_4745, 0]
        );
        assert_eq!(
            words(&defeated(EDGE_CAUSE, Some(0))),
            [3, 9, 3, 0x4544_4745, 0, 0x494e_5354, 1],
            "a kicked goblin keeps its instigator"
        );
        // charge 2, owner 1 (mixed as 2), point blank.
        assert_eq!(
            words(&defeated(BLAST_PB, None)),
            [3, 9, 3, 0x424c_5354, 2, 2, 1]
        );
        // charge 2, no owner (0), ring.
        assert_eq!(
            words(&defeated(BLAST_RING, None)),
            [3, 9, 3, 0x424c_5354, 2, 0, 0]
        );
        assert_eq!(
            words(&defeated(BLAST_RING, Some(0))),
            [3, 9, 3, 0x424c_5354, 2, 0, 0, 0x494e_5354, 1]
        );
        assert_eq!(
            words(&CombatEvent::Blast {
                tick: 60,
                charge: 1
            }),
            [14, 60, 1]
        );
        // The force is derived, not mixed: point blank and ring differ by the
        // point_blank word only.
        assert_ne!(
            words(&defeated(BLAST_PB, None)),
            words(&defeated(BLAST_RING, None))
        );
    }

    #[test]
    fn the_log_lines_of_the_new_events() {
        let line = |e: CombatEvent| e.to_string();
        assert_eq!(
            line(CombatEvent::EnemyDefeated {
                tick: 139,
                target: 0,
                cause: EDGE_CAUSE,
                force: ForceTier::Catastrophic,
                instigator: None,
            }),
            "COMBAT_EVENT tick=139 kind=enemy_defeated target=0 cause=shore_block_edge credit_player=0"
        );
        assert_eq!(
            line(CombatEvent::Blast {
                tick: 60,
                charge: 0
            }),
            "COMBAT_EVENT tick=60 kind=blast charge=0"
        );
        let pb = DefeatCause::Explosion {
            charge: 0,
            player: None,
            point_blank: true,
        };
        let ring = DefeatCause::Explosion {
            charge: 0,
            player: None,
            point_blank: false,
        };
        assert_eq!(
            line(defeated_at(60, 0, pb, None)),
            "COMBAT_EVENT tick=60 kind=enemy_defeated target=0 cause=explosion charge=0 point_blank"
        );
        assert_eq!(
            line(defeated_at(60, 1, ring, None)),
            "COMBAT_EVENT tick=60 kind=enemy_defeated target=1 cause=explosion charge=0"
        );
        // An owned charge adds the credit before point_blank; a goblin Carl had
        // hit keeps its instigator at the end.
        let owned = DefeatCause::Explosion {
            charge: 1,
            player: Some(0),
            point_blank: true,
        };
        assert_eq!(
            line(defeated_at(60, 2, owned, Some(0))),
            "COMBAT_EVENT tick=60 kind=enemy_defeated target=2 cause=explosion charge=1 credit_player=0 point_blank instigator=0"
        );
    }

    fn defeated_at(
        tick: u64,
        target: u32,
        cause: DefeatCause,
        instigator: Option<u32>,
    ) -> CombatEvent {
        CombatEvent::enemy_defeated(tick, target, cause, instigator, None)
    }
}
