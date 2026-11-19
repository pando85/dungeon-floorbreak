//! Donut's swat and pounce (Q10, WORK-023).
//!
//! Bevy-free, deterministic at the 60 Hz fixed tick, and built on plain data:
//! the adapter maps the scene into a [`StepInput`] (press edges, grounded flag,
//! the targets as [`TargetView`]s) and maps the [`StepOutput`] back (velocity
//! commands, locks, the hit box, [`MoveEvent`]s). Nothing here reads or writes
//! a goblin, the corridor or an ECS resource. The numbers are the Q10 knobs of
//! `docs/02-art/donut-moveset-q10.md`.
//!
//! **Tick contract.** Tick 0 of a move is the tick the press is accepted. Call
//! [`DonutMoves::step`] once per fixed tick **before** the scene integrates
//! Donut's motion for that tick: the pounce's launch impulse is returned on
//! tick 10 and the integration of the same tick applies it (14 integration
//! steps, touchdown seen on tick 24, the first tick that starts grounded).
//!
//! **Hit-stop split** (implementation note b). The attacker's freeze and the
//! target's freeze are separate numbers ([`HitStopSplit`]). `target` is the
//! value to store in the goblin's `hit_stop` field directly. The legacy rule
//! (`Goblin::take_hit`) stores `stop + 1` because the scene steps goblins after
//! the strike on the same tick: [`HitStopSplit::legacy`]. The swat freezes only
//! Donut (`target = 0`), so a Carl jab on the next tick still connects.
//!
//! **Light guard** (anti-lock). [`LightGuard`] is per target and is set by the
//! adapter when a swat outcome says so. It clears once at least 60 ticks have
//! passed **and** the target's next swing reached Impact. The Impact is
//! *latched* (implementation note a): it is seen once, so the guard is
//! released exactly once and a hit resolves once.
//!
//! **Hash.** [`DonutMoves::hash_into`] and [`hash_guard`] mix nothing while
//! idle or unset, so a scene whose Donut never moves hashes as before.

use crate::feel::HURT_COOLDOWN;
use crate::prototype::Rect;

/// The hero index Donut's kills are credited to (`DefeatCause::Melee`).
pub const DONUT_HERO: u32 = 1;

// ---------------------------------------------------------------- swat data --

/// Q10-02: startup, active, recovery.
pub const SWAT_STARTUP: u16 = 4;
pub const SWAT_ACTIVE: u16 = 3;
pub const SWAT_RECOVERY: u16 = 7;
pub const SWAT_TOTAL: u16 = SWAT_STARTUP + SWAT_ACTIVE + SWAT_RECOVERY;
/// Q10-03: `[13, 24) x [5, 11)`, shin height.
pub const SWAT_BOX: LocalBox = LocalBox {
    x0: 13.0,
    x1: 24.0,
    y0: 5.0,
    y1: 11.0,
};
/// Q10-04. The swat never defeats.
pub const SWAT_DAMAGE: u8 = 0;
/// Q10-05: attacker freeze 2, target freeze 0.
pub const SWAT_HIT_STOP: HitStopSplit = HitStopSplit {
    attacker: 2,
    target: 0,
};
pub const SWAT_STAGGER: u8 = 10;
/// Push in px/s (about 1.5 px in total).
pub const SWAT_PUSH_PX_S: f32 = 60.0;
/// Q10-07: a press on ticks 4..=13 swats again at tick 10.
pub const SWAT_CHAIN_PRESS_FROM: u16 = 4;
pub const SWAT_CHAIN_PRESS_TO: u16 = 13;
pub const SWAT_CHAIN_AT: u16 = 10;
/// Q10-06: the light guard lasts at least this many ticks.
pub const LIGHT_GUARD_MIN_TICKS: u16 = 60;
/// A boss in neutral takes this much poise from a swat; never HP.
pub const SWAT_BOSS_POISE: u8 = 1;

// -------------------------------------------------------------- pounce data --

/// Q10-10, Q10-13: crouch (the tell), nominal flight, pin, recovery.
pub const POUNCE_STARTUP: u16 = 10;
pub const POUNCE_FLIGHT_NOMINAL: u16 = 14;
pub const POUNCE_PIN: u16 = 2;
pub const POUNCE_RECOVERY: u16 = 18;
/// 10 + 14 + 2 + 18, the art's nominal total on a flat floor.
pub const POUNCE_TOTAL_NOMINAL: u16 =
    POUNCE_STARTUP + POUNCE_FLIGHT_NOMINAL + POUNCE_PIN + POUNCE_RECOVERY;
/// Q10-11: `[11, 25) x [0, 17)`, the flight and pin paws.
pub const POUNCE_BOX: LocalBox = LocalBox {
    x0: 11.0,
    x1: 25.0,
    y0: 0.0,
    y1: 17.0,
};
/// Q10-14: damage 1, attacker hit-stop 4, stagger 14, no push. The target
/// freezes like a jab victim ([`HitStopSplit::legacy`]).
pub const POUNCE_DAMAGE: u8 = 1;
pub const POUNCE_HIT_STOP: HitStopSplit = HitStopSplit::legacy(4);
pub const POUNCE_STAGGER: u8 = 14;
/// Body contact: centre distance with a living goblin that stops the pounce.
pub const POUNCE_CONTACT_PX: f32 = 24.0;

/// The tunable Q10 knobs that `PartyTuning` carries (Q10-08, 09, 10, 12, 13,
/// 15). The frame tables above are fixed data like `MoveSpec`s.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DonutMoveTuning {
    /// Q10-08: `vx` cap during a swat, px/s.
    pub swat_vx_cap_px_s: f32,
    /// Q10-09: the first swat tick a jump cancels the recovery.
    pub swat_jump_cancel_from: u16,
    /// Q10-10: launch velocity, px/s (`vx` is mirrored by facing).
    pub pounce_launch_vx: f32,
    pub pounce_launch_vy: f32,
    /// Q10-12: ticks airborne after launch before the lapse.
    pub pounce_air_max_ticks: u16,
    /// Q10-13: input stays locked until touchdown + this many ticks.
    pub pounce_cancel_after_touchdown: u16,
    /// Q10-15: press buffer, ticks.
    pub buffer_ticks: u8,
}

impl DonutMoveTuning {
    pub const SPEC: Self = Self {
        swat_vx_cap_px_s: 60.0,
        swat_jump_cancel_from: 7,
        pounce_launch_vx: 180.0,
        pounce_launch_vy: 130.0,
        pounce_air_max_ticks: 20,
        pounce_cancel_after_touchdown: 8,
        buffer_ticks: 6,
    };
}

impl Default for DonutMoveTuning {
    fn default() -> Self {
        Self::SPEC
    }
}

// ------------------------------------------------------------ plain data in --

/// A pivot-relative half-open box: `x` forward, `y` up from the feet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalBox {
    pub x0: f32,
    pub x1: f32,
    pub y0: f32,
    pub y1: f32,
}

impl LocalBox {
    /// The world rectangle of the box for a pivot (feet) and a facing (+1/-1).
    pub fn world(self, pivot_x: f32, pivot_y: f32, facing: i8) -> Rect {
        let (left, right) = if facing >= 0 {
            (pivot_x + self.x0, pivot_x + self.x1)
        } else {
            (pivot_x - self.x1, pivot_x - self.x0)
        };
        Rect {
            left,
            bottom: pivot_y + self.y0,
            width: right - left,
            height: self.y1 - self.y0,
        }
    }
}

/// Attacker and target freezes on a confirmed hit, as two numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitStopSplit {
    /// Ticks Donut's move clock freezes (the runner's `begin_hit_stop`).
    pub attacker: u8,
    /// The value to store in the target's `hit_stop` (0 = not frozen).
    pub target: u8,
}

impl HitStopSplit {
    /// What `Goblin::take_hit` does today for a move with hit-stop `stop`: the
    /// attacker freezes `stop` ticks, the target `stop + 1` (it is stepped
    /// after the strike on the same tick).
    pub const fn legacy(stop: u8) -> Self {
        Self {
            attacker: stop,
            target: stop + 1,
        }
    }
}

/// What a target is doing, as the adapter maps its goblin or boss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetState {
    Neutral,
    /// In the tell (attack ticks 0..24): the only state a swat interrupts.
    Tell,
    Smear,
    /// Impact (attack ticks 26..30).
    Impact,
    Recovery,
    /// Staggered by a hit.
    Staggered,
    /// A living goblin launched by the kick.
    Launched,
    /// A living goblin lying down after a launch.
    Downed,
    /// Launched by a killing hit: dead, never a target.
    Airborne,
    /// A corpse or a defeated goblin: never a target.
    Defeated,
}

impl TargetState {
    /// Launched, downed or defeated: ends a light guard.
    pub const fn ends_guard(self) -> bool {
        matches!(
            self,
            Self::Launched | Self::Downed | Self::Airborne | Self::Defeated
        )
    }

    const fn living(self) -> bool {
        !matches!(self, Self::Airborne | Self::Defeated)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Goblin,
    /// The Floor 1 boss (#115): the swat only ever glances.
    Boss,
}

/// One target of the tick. `index` is the stable index the lowest-first rule
/// uses; `body` is the world hurtbox; `guarded` is `guard.is_some()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetView {
    pub index: u32,
    pub kind: TargetKind,
    pub body: Rect,
    pub state: TargetState,
    pub guarded: bool,
}

/// How a Primary press is routed (Q10 section 3). Compute it with
/// [`route_primary`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryRoute {
    /// Pre-biscuit with `donut_moves`: the swat.
    Swat,
    /// Post-biscuit with mana: the Magic Missile cast (not this module).
    Cast,
    /// Post-biscuit at 0 mana: the final dry press.
    Dry,
    /// Nothing happens (the attack-less Donut of `donut_moves: false`).
    Off,
}

/// Primary mapping and the 0-mana rule. `empty_press_swat` is the playtest
/// variant, valid only with `donut_moves` (validation rejects it otherwise;
/// here it is ignored without `donut_moves`).
pub const fn route_primary(
    donut_moves: bool,
    unlocked: bool,
    can_cast: bool,
    empty_press_swat: bool,
) -> PrimaryRoute {
    if !unlocked {
        return if donut_moves {
            PrimaryRoute::Swat
        } else {
            PrimaryRoute::Off
        };
    }
    if can_cast {
        PrimaryRoute::Cast
    } else if donut_moves && empty_press_swat {
        PrimaryRoute::Swat
    } else {
        PrimaryRoute::Dry
    }
}

/// One fixed tick of Donut's input and surroundings.
#[derive(Debug, Clone, Copy)]
pub struct StepInput<'a> {
    /// The caller's tick, stamped on nothing here (events are per-step); kept
    /// so the adapter can label what it logs.
    pub tick: u64,
    /// Press edge of Primary this tick, and how it routes.
    pub primary_pressed: bool,
    pub primary_route: PrimaryRoute,
    /// Press edge of Physical (the pounce). Ignore it by passing `false` when
    /// `donut_moves` is off.
    pub physical_pressed: bool,
    pub jump_pressed: bool,
    /// Any horizontal movement input this tick.
    pub move_pressed: bool,
    /// Donut starts the tick on the floor.
    pub grounded: bool,
    pub pivot_x: f32,
    pub pivot_y: f32,
    /// +1 / -1.
    pub facing: i8,
    /// A wall blocks the forward motion (the scene's collision).
    pub wall_blocked: bool,
    /// Donut was hit this tick (no armour: it cancels either move).
    pub hurt: bool,
    /// False for an AI-controlled Donut: the follower never attacks, so no new
    /// move starts and no press is remembered. A running move finishes.
    pub may_start: bool,
    pub targets: &'a [TargetView],
}

// ----------------------------------------------------------- plain data out --

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    Swat,
    Pounce,
}

/// The move's presentation phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovePhase {
    Startup,
    Active,
    Recovery,
    /// Pounce crouch (the tell).
    Crouch,
    Flight,
    /// Touchdown and the pin; also the recovery after it.
    Pin,
}

/// What a hit does to a target. Applied by the adapter; the numbers are the
/// Q10 data, never read back from the goblin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetEffect {
    pub damage: u8,
    pub stagger_ticks: u8,
    /// Push away from Donut, px/s.
    pub push_px_s: f32,
    pub hit_stop: HitStopSplit,
    /// The interrupted attack returns its token.
    pub return_token: bool,
    /// `HURT_COOLDOWN` to start on the target (the interrupt).
    pub hurt_cooldown: u8,
    /// Set a [`LightGuard`] on the target.
    pub sets_guard: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlanceReason {
    /// In its swing (smear or impact) or its recovery.
    Swing,
    Staggered,
    /// Launched, downed or airborne.
    Down,
    Guarded,
    Boss,
}

impl GlanceReason {
    /// The word the event line prints.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Swing => "swing",
            Self::Staggered => "staggered",
            Self::Down => "down",
            Self::Guarded => "guarded",
            Self::Boss => "boss",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CancelCause {
    Hurt,
    Jump,
    /// A move or a new press after the pounce's commitment ends.
    Move,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoveEvent {
    Started {
        mv: MoveKind,
        instance: u32,
    },
    /// Pounce launch (tick 10).
    Launched {
        instance: u32,
    },
    /// First grounded tick after the launch.
    Touchdown {
        instance: u32,
    },
    /// Still airborne `pounce_air_max_ticks` after launch: box and lock off.
    Lapsed {
        instance: u32,
    },
    Cancelled {
        mv: MoveKind,
        instance: u32,
        cause: CancelCause,
    },
    Ended {
        mv: MoveKind,
        instance: u32,
    },
    /// A swat on a tell: the attack is cancelled, the token returns and the
    /// hurt cooldown starts ([`TargetEffect::return_token`],
    /// [`TargetEffect::hurt_cooldown`]).
    TellInterrupted {
        target: u32,
        effect: TargetEffect,
    },
    /// A swat on a neutral goblin: stagger only.
    Staggered {
        target: u32,
        effect: TargetEffect,
    },
    /// A swat that does nothing but flash for one tick (no stagger, no
    /// hit-stop, no state change). `poise` is 1 only for a boss in neutral.
    Glance {
        target: u32,
        reason: GlanceReason,
        poise: u8,
    },
    PounceHit {
        target: u32,
        effect: TargetEffect,
    },
    /// A Primary press at 0 mana after the biscuit: no lock, no buffer.
    DryPress,
}

/// What the adapter must do this tick.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StepOutput {
    /// Pounce launch velocity (`vx` already signed by facing), px/s.
    pub launch: Option<(f32, f32)>,
    /// Force `vx` (the pounce after a hit, a wall or body contact).
    pub set_vx: Option<f32>,
    /// Cap `|vx|` (the swat).
    pub vx_cap: Option<f32>,
    /// Ignore walking input (the pounce until touchdown + cancel delay).
    pub input_locked: bool,
    pub facing_locked: bool,
    /// Swallow a jump press this tick.
    pub jump_blocked: bool,
    /// The attacker's move clock is frozen by hit-stop: skip Donut's physics
    /// and animation this tick; no hit is resolved.
    pub frozen: bool,
    /// The active hit box in the world, if any (also for `--debug-combat`).
    pub hitbox: Option<Rect>,
    /// A buffered or fresh Cast press is released this tick: start the cast
    /// (the Magic Missile runner owns it).
    pub cast_requested: bool,
    pub events: Vec<MoveEvent>,
}

/// The running move without advancing it, for presentation and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveView {
    pub mv: MoveKind,
    pub tick: u16,
    pub instance: u32,
    pub phase: MovePhase,
    pub launched: bool,
    pub touchdown: Option<u16>,
    pub hit: bool,
}

// ---------------------------------------------------------------- the guard --

/// The light guard of one target (Q10-06). Set when a swat staggers it; a
/// guarded target glances further swats. Carl's hits and the pounce still
/// interrupt it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LightGuard {
    age: u16,
    /// The target's next swing reached Impact. **Latched**: once seen it stays,
    /// so the clear condition is evaluated against a fact, not a window.
    impact_seen: bool,
}

impl LightGuard {
    pub const fn new() -> Self {
        Self {
            age: 0,
            impact_seen: false,
        }
    }

    pub const fn age(&self) -> u16 {
        self.age
    }

    pub const fn impact_seen(&self) -> bool {
        self.impact_seen
    }

    /// Advance one tick with the target's state. Returns true when the guard
    /// is released (the adapter then drops it).
    pub fn observe(&mut self, state: TargetState) -> bool {
        self.age = self.age.saturating_add(1);
        if state.ends_guard() {
            return true;
        }
        if state == TargetState::Impact {
            self.impact_seen = true;
        }
        self.impact_seen && self.age >= LIGHT_GUARD_MIN_TICKS
    }

    pub fn hash_into(&self, mix: &mut impl FnMut(u64)) {
        mix(0x6775_6172_6400_0001); // marker
        mix(u64::from(self.age));
        mix(u64::from(self.impact_seen));
    }
}

/// Mix a target's guard: **nothing** when it is not set.
pub fn hash_guard(guard: &Option<LightGuard>, mix: &mut impl FnMut(u64)) {
    if let Some(guard) = guard {
        guard.hash_into(mix);
    }
}

// --------------------------------------------------------------- the runner --

#[derive(Debug, Clone, PartialEq)]
struct Running {
    mv: MoveKind,
    tick: u16,
    instance: u32,
    /// Targets already resolved by this instance (a hit resolves once).
    hit: Vec<u32>,
    /// A swat press is remembered to chain at [`SWAT_CHAIN_AT`].
    chained: bool,
    launched: bool,
    touchdown: Option<u16>,
    /// The pounce already hit its one target.
    pounce_hit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Request {
    Swat,
    Pounce,
    Cast,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Buffered {
    request: Request,
    /// Last clock value at which the press may still start.
    expires: u64,
}

/// The swat and pounce state machine with press buffering and hit-stop.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DonutMoves {
    tuning: DonutMoveTuning,
    clock: u64,
    next_instance: u32,
    current: Option<Running>,
    buffered: Option<Buffered>,
    hold: u8,
}

impl DonutMoves {
    pub fn new(tuning: DonutMoveTuning) -> Self {
        Self {
            tuning,
            ..Self::default()
        }
    }

    pub fn tuning(&self) -> &DonutMoveTuning {
        &self.tuning
    }

    /// A move is running (Donut is committed).
    pub fn busy(&self) -> bool {
        self.current.is_some()
    }

    pub fn hit_stop_left(&self) -> u8 {
        self.hold
    }

    /// Lengthen the attacker's freeze (a lethal pounce adds the lethal
    /// hit-stop bonus the target also took). Never shortens it.
    pub fn extend_hit_stop(&mut self, ticks: u8) {
        self.hold = self.hold.max(ticks);
    }

    /// A swap clears the buffered press; it never cancels a running move.
    pub fn clear_buffer(&mut self) {
        self.buffered = None;
        if let Some(running) = self.current.as_mut() {
            running.chained = false;
        }
    }

    pub fn view(&self) -> Option<MoveView> {
        let r = self.current.as_ref()?;
        let phase = match r.mv {
            MoveKind::Swat => {
                if r.tick < SWAT_STARTUP {
                    MovePhase::Startup
                } else if r.tick < SWAT_STARTUP + SWAT_ACTIVE {
                    MovePhase::Active
                } else {
                    MovePhase::Recovery
                }
            }
            MoveKind::Pounce => {
                if r.tick < POUNCE_STARTUP {
                    MovePhase::Crouch
                } else if let Some(td) = r.touchdown {
                    if r.tick <= td + 1 {
                        MovePhase::Pin
                    } else {
                        MovePhase::Recovery
                    }
                } else {
                    MovePhase::Flight
                }
            }
        };
        Some(MoveView {
            mv: r.mv,
            tick: r.tick,
            instance: r.instance,
            phase,
            launched: r.launched,
            touchdown: r.touchdown,
            hit: r.pounce_hit || !r.hit.is_empty(),
        })
    }

    /// Advance one fixed tick.
    pub fn step(&mut self, input: &StepInput<'_>) -> StepOutput {
        let mut out = StepOutput::default();

        // A hit on Donut cancels either move: no armour.
        if input.hurt {
            self.clock += 1;
            if let Some(r) = self.current.take() {
                out.events.push(MoveEvent::Cancelled {
                    mv: r.mv,
                    instance: r.instance,
                    cause: CancelCause::Hurt,
                });
            }
            self.buffered = None;
            self.hold = 0;
            return out;
        }

        let requests = self.requests(input, &mut out);

        // Hit-stop: nothing advances, a press is still remembered.
        if self.hold > 0 && self.current.is_some() {
            self.hold -= 1;
            out.frozen = true;
            for request in requests.iter().flatten() {
                self.note_press(*request);
            }
            self.constrain(&mut out);
            return out;
        }
        self.hold = 0;
        self.clock += 1;

        if let Some(r) = self.current.as_mut() {
            r.tick += 1;
        }
        self.advance_end_and_cancels(input, &requests, &mut out);

        if !input.may_start {
            // The follower never attacks: nothing starts, nothing is kept.
            self.buffered = None;
        } else {
            for request in requests.iter().flatten() {
                self.handle_press(*request, input, &mut out);
            }
            self.start_chained(&mut out);
            self.start_buffered(input, &mut out);
        }

        self.run_current(input, &mut out);
        self.constrain(&mut out);
        out
    }

    /// The presses of this tick in a fixed order (Primary, then Physical). A
    /// dry press resolves right here: an event, no lock, no buffer.
    fn requests(&mut self, input: &StepInput<'_>, out: &mut StepOutput) -> [Option<Request>; 2] {
        let mut requests = [None, None];
        if input.primary_pressed {
            match input.primary_route {
                PrimaryRoute::Swat => requests[0] = Some(Request::Swat),
                PrimaryRoute::Cast => requests[0] = Some(Request::Cast),
                PrimaryRoute::Dry => out.events.push(MoveEvent::DryPress),
                PrimaryRoute::Off => {}
            }
        }
        if input.physical_pressed {
            requests[1] = Some(Request::Pounce);
        }
        requests
    }

    /// End of a move by its total, and the jump or move cancels.
    fn advance_end_and_cancels(
        &mut self,
        input: &StepInput<'_>,
        requests: &[Option<Request>; 2],
        out: &mut StepOutput,
    ) {
        let Some(r) = self.current.as_ref() else {
            return;
        };
        let (mv, instance, tick) = (r.mv, r.instance, r.tick);
        let ended = match mv {
            MoveKind::Swat => tick >= SWAT_TOTAL,
            MoveKind::Pounce => r
                .touchdown
                .is_some_and(|td| tick >= td + POUNCE_PIN + POUNCE_RECOVERY),
        };
        if ended {
            self.current = None;
            out.events.push(MoveEvent::Ended { mv, instance });
            return;
        }
        let cause = match mv {
            // A jump on tick 7..=13 cancels the recovery (agility).
            MoveKind::Swat => (input.jump_pressed && tick >= self.tuning.swat_jump_cancel_from)
                .then_some(CancelCause::Jump),
            // After touchdown + 8, a move or a jump cancels the rest.
            MoveKind::Pounce => r
                .touchdown
                .filter(|td| tick >= td + self.tuning.pounce_cancel_after_touchdown)
                .and_then(|_| {
                    if input.jump_pressed {
                        Some(CancelCause::Jump)
                    } else if input.move_pressed || requests.iter().any(Option::is_some) {
                        Some(CancelCause::Move)
                    } else {
                        None
                    }
                }),
        };
        if let Some(cause) = cause {
            self.current = None;
            self.hold = 0;
            out.events.push(MoveEvent::Cancelled {
                mv,
                instance,
                cause,
            });
        }
    }

    /// A press with the move state of this tick: start, chain or buffer.
    fn handle_press(&mut self, request: Request, input: &StepInput<'_>, out: &mut StepOutput) {
        if request == Request::Cast {
            if self.current.is_none() {
                out.cast_requested = true;
            } else {
                self.note_press(request);
            }
            return;
        }
        if self.current.is_none() && input.grounded {
            self.start(request, out);
        } else {
            self.note_press(request);
        }
    }

    /// A press while busy or airborne: inside the swat's string window it is
    /// the chained swat, otherwise it is remembered for `buffer_ticks`.
    fn note_press(&mut self, request: Request) {
        if let Some(r) = self.current.as_mut()
            && r.mv == MoveKind::Swat
            && request == Request::Swat
            && (SWAT_CHAIN_PRESS_FROM..=SWAT_CHAIN_PRESS_TO).contains(&r.tick)
        {
            r.chained = true;
            return;
        }
        if self.tuning.buffer_ticks > 0 {
            self.buffered = Some(Buffered {
                request,
                expires: self.clock + u64::from(self.tuning.buffer_ticks),
            });
        }
    }

    fn start(&mut self, request: Request, out: &mut StepOutput) {
        let mv = match request {
            Request::Swat => MoveKind::Swat,
            Request::Pounce => MoveKind::Pounce,
            Request::Cast => return,
        };
        self.next_instance += 1;
        self.buffered = None;
        self.current = Some(Running {
            mv,
            tick: 0,
            instance: self.next_instance,
            hit: Vec::new(),
            chained: false,
            launched: false,
            touchdown: None,
            pounce_hit: false,
        });
        out.events.push(MoveEvent::Started {
            mv,
            instance: self.next_instance,
        });
    }

    /// The second swat starts at tick 10 (or on the press, if later).
    fn start_chained(&mut self, out: &mut StepOutput) {
        let Some(r) = self.current.as_ref() else {
            return;
        };
        if r.mv == MoveKind::Swat && r.chained && r.tick >= SWAT_CHAIN_AT {
            let instance = r.instance;
            out.events.push(MoveEvent::Ended {
                mv: MoveKind::Swat,
                instance,
            });
            self.current = None;
            self.start(Request::Swat, out);
        }
    }

    fn start_buffered(&mut self, input: &StepInput<'_>, out: &mut StepOutput) {
        if self.current.is_some() {
            return;
        }
        let Some(buffered) = self.buffered else {
            return;
        };
        if buffered.expires < self.clock {
            self.buffered = None;
            return;
        }
        match buffered.request {
            Request::Cast => {
                self.buffered = None;
                out.cast_requested = true;
            }
            _ if input.grounded => {
                self.buffered = None;
                self.start(buffered.request, out);
            }
            _ => {}
        }
    }

    /// The per-tick behaviour of the running move: launch, touchdown, lapse,
    /// the hit box and the hit.
    fn run_current(&mut self, input: &StepInput<'_>, out: &mut StepOutput) {
        let Some(r) = self.current.as_mut() else {
            return;
        };
        match r.mv {
            MoveKind::Swat => {
                let active = (SWAT_STARTUP..SWAT_STARTUP + SWAT_ACTIVE).contains(&r.tick);
                if !active {
                    return;
                }
                let rect = SWAT_BOX.world(input.pivot_x, input.pivot_y, input.facing);
                out.hitbox = Some(rect);
                let mut stagger_hold = false;
                for target in lowest_first(input.targets, rect) {
                    if r.hit.contains(&target.index) {
                        continue;
                    }
                    r.hit.push(target.index);
                    let event = swat_outcome(target);
                    stagger_hold |= matches!(
                        event,
                        MoveEvent::TellInterrupted { .. } | MoveEvent::Staggered { .. }
                    );
                    out.events.push(event);
                }
                if stagger_hold {
                    self.hold = self.hold.max(SWAT_HIT_STOP.attacker);
                }
            }
            MoveKind::Pounce => {
                if r.tick == POUNCE_STARTUP {
                    r.launched = true;
                    out.launch = Some((
                        f32::from(input.facing) * self.tuning.pounce_launch_vx,
                        self.tuning.pounce_launch_vy,
                    ));
                    out.events.push(MoveEvent::Launched {
                        instance: r.instance,
                    });
                }
                if !r.launched {
                    return;
                }
                if r.tick > POUNCE_STARTUP && r.touchdown.is_none() && input.grounded {
                    r.touchdown = Some(r.tick);
                    out.events.push(MoveEvent::Touchdown {
                        instance: r.instance,
                    });
                }
                if r.touchdown.is_none()
                    && r.tick - POUNCE_STARTUP >= self.tuning.pounce_air_max_ticks
                {
                    // Off a ledge: the box and the lock end, she lands with
                    // `land` and no pin.
                    out.events.push(MoveEvent::Lapsed {
                        instance: r.instance,
                    });
                    self.current = None;
                    return;
                }
                // Active from launch to touchdown + 1.
                if r.touchdown.is_some_and(|td| r.tick > td + 1) {
                    return;
                }
                let rect = POUNCE_BOX.world(input.pivot_x, input.pivot_y, input.facing);
                out.hitbox = Some(rect);
                let in_flight = r.touchdown.is_none();
                if in_flight {
                    // Q10-A04: only a goblin up to 24 px ahead of her stops the
                    // pounce; one behind her and the boss never do.
                    let contact = input.targets.iter().any(|t| {
                        let ahead = (centre_x(t.body) - input.pivot_x) * f32::from(input.facing);
                        t.kind == TargetKind::Goblin
                            && t.state.living()
                            && (0.0..=POUNCE_CONTACT_PX).contains(&ahead)
                    });
                    if input.wall_blocked || contact {
                        out.set_vx = Some(0.0);
                    }
                }
                if r.pounce_hit {
                    return;
                }
                if let Some(target) = lowest_first(input.targets, rect).next() {
                    r.pounce_hit = true;
                    out.set_vx = Some(0.0);
                    out.events.push(MoveEvent::PounceHit {
                        target: target.index,
                        effect: TargetEffect {
                            damage: POUNCE_DAMAGE,
                            stagger_ticks: POUNCE_STAGGER,
                            push_px_s: 0.0,
                            hit_stop: POUNCE_HIT_STOP,
                            return_token: false,
                            hurt_cooldown: 0,
                            sets_guard: false,
                        },
                    });
                    self.hold = self.hold.max(POUNCE_HIT_STOP.attacker);
                }
            }
        }
    }

    /// The locks and caps of the running move.
    fn constrain(&self, out: &mut StepOutput) {
        let Some(r) = self.current.as_ref() else {
            return;
        };
        out.facing_locked = true;
        match r.mv {
            MoveKind::Swat => {
                out.vx_cap = Some(self.tuning.swat_vx_cap_px_s);
                out.jump_blocked = r.tick < self.tuning.swat_jump_cancel_from;
            }
            MoveKind::Pounce => {
                // Locked until touchdown + 8; the whole flight before it.
                let locked = r
                    .touchdown
                    .is_none_or(|td| r.tick < td + self.tuning.pounce_cancel_after_touchdown);
                out.input_locked = locked;
                out.jump_blocked = locked;
            }
        }
    }

    /// Fold into the state hash. **Nothing is mixed while idle** (no move, no
    /// buffered press, no hit-stop), so a scene whose Donut never moves hashes
    /// as before. The absolute clock is never mixed.
    pub fn hash_into(&self, mix: &mut impl FnMut(u64)) {
        if self.current.is_none() && self.buffered.is_none() && self.hold == 0 {
            return;
        }
        mix(0x646f_6e75_745f_6d76); // "donut_mv" marker
        match &self.current {
            Some(r) => {
                mix(1 + r.mv as u64);
                mix(u64::from(r.tick));
                mix(u64::from(r.instance));
                mix(u64::from(r.chained));
                mix(u64::from(r.launched));
                mix(r.touchdown.map_or(0, |t| 1 + u64::from(t)));
                mix(u64::from(r.pounce_hit));
                for target in &r.hit {
                    mix(u64::from(*target));
                }
            }
            None => mix(0),
        }
        match self.buffered {
            Some(b) => {
                mix(1 + b.request as u64);
                mix(b.expires.saturating_sub(self.clock));
            }
            None => mix(0),
        }
        mix(u64::from(self.hold));
    }
}

fn centre_x(body: Rect) -> f32 {
    body.left + body.width * 0.5
}

/// The living targets whose body overlaps `rect`, lowest index first (a corpse
/// or a goblin killed in the air is never a target).
fn lowest_first(targets: &[TargetView], rect: Rect) -> impl Iterator<Item = &TargetView> {
    let mut hits: Vec<&TargetView> = targets
        .iter()
        .filter(|t| t.state.living() && rect.overlaps(t.body))
        .collect();
    hits.sort_by_key(|t| t.index);
    hits.into_iter()
}

/// The swat's outcome on one target (Q10 section 1).
fn swat_outcome(target: &TargetView) -> MoveEvent {
    let glance = |reason, poise| MoveEvent::Glance {
        target: target.index,
        reason,
        poise,
    };
    if target.kind == TargetKind::Boss {
        // Always a glance: 0 HP, no interrupt, no hit-stop; poise in neutral.
        let poise = if target.state == TargetState::Neutral {
            SWAT_BOSS_POISE
        } else {
            0
        };
        return glance(GlanceReason::Boss, poise);
    }
    match target.state {
        TargetState::Launched | TargetState::Downed | TargetState::Airborne => {
            glance(GlanceReason::Down, 0)
        }
        TargetState::Smear | TargetState::Impact | TargetState::Recovery => {
            glance(GlanceReason::Swing, 0)
        }
        TargetState::Staggered => glance(GlanceReason::Staggered, 0),
        _ if target.guarded => glance(GlanceReason::Guarded, 0),
        TargetState::Tell => MoveEvent::TellInterrupted {
            target: target.index,
            effect: swat_effect(true),
        },
        TargetState::Neutral => MoveEvent::Staggered {
            target: target.index,
            effect: swat_effect(false),
        },
        // Dead targets are filtered out before this point.
        TargetState::Defeated => glance(GlanceReason::Down, 0),
    }
}

fn swat_effect(interrupt: bool) -> TargetEffect {
    TargetEffect {
        damage: SWAT_DAMAGE,
        stagger_ticks: SWAT_STAGGER,
        push_px_s: SWAT_PUSH_PX_S,
        hit_stop: SWAT_HIT_STOP,
        return_token: interrupt,
        hurt_cooldown: if interrupt { HURT_COOLDOWN } else { 0 },
        sets_guard: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLOOR: f32 = 0.0;
    const GRAVITY: f32 = 1050.0;
    const DT: f32 = 1.0 / 60.0;

    fn goblin_at(index: u32, centre_distance: f32, state: TargetState) -> TargetView {
        // Donut's pivot is x = 0 and she faces +x; goblin body 24 x 38.
        TargetView {
            index,
            kind: TargetKind::Goblin,
            body: Rect {
                left: centre_distance - 12.0,
                bottom: FLOOR,
                width: 24.0,
                height: 38.0,
            },
            state,
            guarded: false,
        }
    }

    fn base<'a>(targets: &'a [TargetView]) -> StepInput<'a> {
        StepInput {
            tick: 0,
            primary_pressed: false,
            primary_route: PrimaryRoute::Swat,
            physical_pressed: false,
            jump_pressed: false,
            move_pressed: false,
            grounded: true,
            pivot_x: 0.0,
            pivot_y: FLOOR,
            facing: 1,
            wall_blocked: false,
            hurt: false,
            may_start: true,
            targets,
        }
    }

    fn press_swat<'a>(targets: &'a [TargetView]) -> StepInput<'a> {
        StepInput {
            primary_pressed: true,
            ..base(targets)
        }
    }

    fn press_pounce<'a>(targets: &'a [TargetView]) -> StepInput<'a> {
        StepInput {
            physical_pressed: true,
            ..base(targets)
        }
    }

    /// A tiny Donut body: semi-implicit integration after each step, the way
    /// the scene does it (the launch impulse of tick 10 is applied by the
    /// integration of tick 10).
    struct Body {
        x: f32,
        y: f32,
        vx: f32,
        vy: f32,
        grounded: bool,
    }

    impl Body {
        fn new() -> Self {
            Self {
                x: 0.0,
                y: 0.0,
                vx: 0.0,
                vy: 0.0,
                grounded: true,
            }
        }

        fn apply(&mut self, out: &StepOutput) {
            if let Some((vx, vy)) = out.launch {
                self.vx = vx;
                self.vy = vy;
                self.grounded = false;
            }
            if let Some(vx) = out.set_vx {
                self.vx = vx;
            }
        }

        fn integrate(&mut self) {
            if self.grounded {
                return;
            }
            self.vy -= GRAVITY * DT;
            self.x += self.vx * DT;
            self.y += self.vy * DT;
            if self.y <= 0.0 && self.vy <= 0.0 {
                self.y = 0.0;
                self.vy = 0.0;
                self.grounded = true;
            }
        }
    }

    /// Run a pounce pressed on tick 0 with `targets` fixed; returns per-tick
    /// outputs and the body. `until` is the number of ticks to run.
    fn run_pounce(targets: &[TargetView], until: u16) -> (Vec<StepOutput>, Body, DonutMoves) {
        run_pounce_facing(targets, until, 1)
    }

    fn run_pounce_facing(
        targets: &[TargetView],
        until: u16,
        facing: i8,
    ) -> (Vec<StepOutput>, Body, DonutMoves) {
        let mut m = DonutMoves::default();
        let mut body = Body::new();
        let mut outs = Vec::new();
        for t in 0..until {
            let input = StepInput {
                physical_pressed: t == 0,
                grounded: body.grounded,
                pivot_x: body.x,
                pivot_y: body.y,
                facing,
                ..base(targets)
            };
            let out = m.step(&input);
            body.apply(&out);
            if !out.frozen {
                body.integrate();
            }
            outs.push(out);
        }
        (outs, body, m)
    }

    // ------------------------------------------------------------ the tables --

    #[test]
    fn frame_tables_match_the_spec() {
        assert_eq!(
            (SWAT_STARTUP, SWAT_ACTIVE, SWAT_RECOVERY, SWAT_TOTAL),
            (4, 3, 7, 14)
        );
        assert_eq!(SWAT_DAMAGE, 0);
        assert_eq!(
            (
                POUNCE_STARTUP,
                POUNCE_FLIGHT_NOMINAL,
                POUNCE_PIN,
                POUNCE_RECOVERY
            ),
            (10, 14, 2, 18)
        );
        assert_eq!(POUNCE_TOTAL_NOMINAL, 44);
        assert_eq!(POUNCE_DAMAGE, 1);
        assert_eq!(
            SWAT_HIT_STOP,
            HitStopSplit {
                attacker: 2,
                target: 0
            }
        );
        assert_eq!(
            POUNCE_HIT_STOP,
            HitStopSplit {
                attacker: 4,
                target: 5
            }
        );
        assert_eq!(
            HitStopSplit::legacy(2),
            HitStopSplit {
                attacker: 2,
                target: 3
            }
        );
        assert_eq!((SWAT_STAGGER, POUNCE_STAGGER), (10, 14));
        let t = DonutMoveTuning::default();
        assert_eq!(t.swat_vx_cap_px_s, 60.0);
        assert_eq!(t.swat_jump_cancel_from, 7);
        assert_eq!((t.pounce_launch_vx, t.pounce_launch_vy), (180.0, 130.0));
        assert_eq!(t.pounce_air_max_ticks, 20);
        assert_eq!(t.pounce_cancel_after_touchdown, 8);
        assert_eq!(t.buffer_ticks, 6);
    }

    #[test]
    fn boxes_mirror_with_facing() {
        let r = SWAT_BOX.world(100.0, 0.0, 1);
        assert_eq!(
            (r.left, r.right(), r.bottom, r.top()),
            (113.0, 124.0, 5.0, 11.0)
        );
        let r = SWAT_BOX.world(100.0, 0.0, -1);
        assert_eq!((r.left, r.right()), (76.0, 87.0));
        let r = POUNCE_BOX.world(0.0, 0.0, 1);
        assert_eq!((r.left, r.right(), r.top()), (11.0, 25.0, 17.0));
    }

    // ------------------------------------------------------------------ swat --

    /// Run a swat pressed on tick 0 against fixed targets; per-tick outputs.
    fn run_swat(targets: &[TargetView], ticks: u16) -> Vec<StepOutput> {
        let mut m = DonutMoves::default();
        (0..ticks)
            .map(|t| {
                let input = if t == 0 {
                    press_swat(targets)
                } else {
                    base(targets)
                };
                m.step(&input)
            })
            .collect()
    }

    #[test]
    fn swat_hits_only_on_ticks_4_to_6_and_hit_tick_is_4() {
        // A recovering goblin glances: no hit-stop, so call index == move tick.
        let target = [goblin_at(0, 20.0, TargetState::Recovery)];
        let outs = run_swat(&target, 16);
        for (t, out) in outs.iter().enumerate() {
            let hit = out.events.iter().any(|e| {
                matches!(
                    e,
                    MoveEvent::Staggered { .. }
                        | MoveEvent::TellInterrupted { .. }
                        | MoveEvent::Glance { .. }
                )
            });
            assert_eq!(hit, t == 4, "tick {t}");
            assert_eq!(
                out.hitbox.is_some(),
                (4..=6).contains(&t),
                "box on tick {t}"
            );
        }
    }

    #[test]
    fn swat_reaches_a_goblin_centre_at_35_and_misses_at_36() {
        for (d, expect) in [(35.0, true), (35.9, true), (36.0, false)] {
            let t = [goblin_at(0, d, TargetState::Neutral)];
            let hit = run_swat(&t, 8).iter().any(|o| {
                o.events
                    .iter()
                    .any(|e| matches!(e, MoveEvent::Staggered { .. }))
            });
            assert_eq!(hit, expect, "centre distance {d}");
        }
        // Shin height: a goblin whose body is entirely above y = 11 is missed.
        let mut high = goblin_at(0, 20.0, TargetState::Neutral);
        high.body.bottom = 11.0;
        assert!(!run_swat(&[high], 8).iter().any(|o| {
            o.events
                .iter()
                .any(|e| matches!(e, MoveEvent::Staggered { .. }))
        }));
    }

    #[test]
    fn the_goblin_swing_reaches_donut_from_39_and_not_40() {
        // The swat never out-ranges what it interrupts: pin the goblin side
        // with the real swing box against Donut's 20 x 24 hurtbox.
        use crate::combat::EnemyAi;
        use crate::goblin::Goblin;
        let hurtbox = |x: f32| Rect {
            left: x - 10.0,
            bottom: 0.0,
            width: 20.0,
            height: 24.0,
        };
        let mut g = Goblin::new(100.0, 19.0, EnemyAi::Chase);
        g.facing = -1; // faces Donut on its left
        let reach = |donut_x: f32| g.swing_box().overlaps(hurtbox(donut_x));
        assert!(reach(100.0 - 39.0));
        assert!(!reach(100.0 - 40.0));
    }

    #[test]
    fn swat_interrupts_a_tell_and_staggers_a_neutral_goblin_and_never_defeats() {
        let tell = [goblin_at(3, 25.0, TargetState::Tell)];
        let out = &run_swat(&tell, 8)[4];
        let MoveEvent::TellInterrupted { target, effect } = out.events[0] else {
            panic!("expected an interrupt, got {:?}", out.events);
        };
        assert_eq!(target, 3);
        assert_eq!(effect.damage, 0);
        assert_eq!(effect.stagger_ticks, 10);
        assert!(effect.return_token);
        assert_eq!(effect.hurt_cooldown, 20);
        assert!(effect.sets_guard);
        assert_eq!(effect.push_px_s, 60.0);
        assert_eq!(effect.hit_stop, SWAT_HIT_STOP);

        let neutral = [goblin_at(3, 25.0, TargetState::Neutral)];
        let out = &run_swat(&neutral, 8)[4];
        let MoveEvent::Staggered { effect, .. } = out.events[0] else {
            panic!("expected a stagger, got {:?}", out.events);
        };
        assert_eq!(effect.damage, 0, "the swat never defeats");
        assert!(!effect.return_token);
        assert_eq!(effect.hurt_cooldown, 0);
        assert!(effect.sets_guard);
    }

    #[test]
    fn swat_glances_on_swing_recovery_staggered_down_and_guarded() {
        let cases = [
            (TargetState::Smear, false, GlanceReason::Swing),
            (TargetState::Impact, false, GlanceReason::Swing),
            (TargetState::Recovery, false, GlanceReason::Swing),
            (TargetState::Staggered, false, GlanceReason::Staggered),
            (TargetState::Launched, false, GlanceReason::Down),
            (TargetState::Downed, false, GlanceReason::Down),
            (TargetState::Tell, true, GlanceReason::Guarded),
            (TargetState::Neutral, true, GlanceReason::Guarded),
        ];
        for (state, guarded, reason) in cases {
            let mut t = goblin_at(0, 25.0, state);
            t.guarded = guarded;
            let outs = run_swat(&[t], 8);
            assert_eq!(
                outs[4].events,
                vec![MoveEvent::Glance {
                    target: 0,
                    reason,
                    poise: 0
                }],
                "{state:?} guarded {guarded}"
            );
            // A glance has no hit-stop, no state change.
            assert!(!outs[5].frozen, "{state:?}");
        }
    }

    #[test]
    fn swat_does_nothing_to_the_dead() {
        for state in [TargetState::Airborne, TargetState::Defeated] {
            let outs = run_swat(&[goblin_at(0, 25.0, state)], 8);
            assert!(outs.iter().all(|o| {
                !o.events
                    .iter()
                    .any(|e| matches!(e, MoveEvent::Glance { .. } | MoveEvent::Staggered { .. }))
            }));
        }
    }

    #[test]
    fn swat_freezes_only_donut_for_two_ticks() {
        let t = [goblin_at(0, 25.0, TargetState::Neutral)];
        let outs = run_swat(&t, 12);
        // Hit on tick 4: the attacker's clock holds for the next two calls.
        assert!(!outs[4].frozen);
        assert!(outs[5].frozen && outs[6].frozen);
        assert!(!outs[7].frozen);
        assert!(outs[5].hitbox.is_none(), "a frozen tick resolves no hit");
        // The split the adapter applies: the goblin is not frozen.
        let MoveEvent::Staggered { effect, .. } = outs[4].events[0] else {
            panic!()
        };
        assert_eq!(effect.hit_stop.attacker, 2);
        assert_eq!(effect.hit_stop.target, 0);
    }

    #[test]
    fn a_swat_resolves_each_target_once_per_instance() {
        // The box is active on ticks 4, 5, 6; the goblin is hit on 4 only.
        let t = [goblin_at(0, 25.0, TargetState::Neutral)];
        let outs = run_swat(&t, 14);
        let hits = outs
            .iter()
            .flat_map(|o| o.events.iter())
            .filter(|e| matches!(e, MoveEvent::Staggered { .. }))
            .count();
        assert_eq!(hits, 1);
    }

    #[test]
    fn swat_movement_cap_facing_and_jump_rules() {
        let t: [TargetView; 0] = [];
        let mut m = DonutMoves::default();
        m.step(&press_swat(&t));
        for tick in 1..14u16 {
            let out = m.step(&StepInput {
                jump_pressed: false,
                ..base(&t)
            });
            assert_eq!(out.vx_cap, Some(60.0), "tick {tick}");
            assert!(out.facing_locked);
            assert!(!out.input_locked, "a swat caps vx, it does not zero it");
            assert_eq!(out.jump_blocked, tick < 7, "tick {tick}");
        }
    }

    #[test]
    fn a_jump_cancels_the_recovery_from_tick_7_and_not_on_tick_6() {
        let t: [TargetView; 0] = [];
        let run = |jump_at: u16| {
            let mut m = DonutMoves::default();
            m.step(&press_swat(&t));
            for tick in 1..=jump_at {
                let input = StepInput {
                    jump_pressed: tick == jump_at,
                    ..base(&t)
                };
                let out = m.step(&input);
                if tick == jump_at {
                    return (m.busy(), out.events);
                }
            }
            unreachable!()
        };
        let (busy, ev) = run(6);
        assert!(busy, "a jump on tick 6 is swallowed");
        assert!(!ev.iter().any(|e| matches!(e, MoveEvent::Cancelled { .. })));
        for tick in [7, 10, 13] {
            let (busy, ev) = run(tick);
            assert!(!busy, "jump on tick {tick} cancels");
            assert!(ev.iter().any(|e| matches!(
                e,
                MoveEvent::Cancelled {
                    cause: CancelCause::Jump,
                    ..
                }
            )));
        }
    }

    #[test]
    fn the_second_swat_fires_at_tick_10_for_a_press_in_4_to_13() {
        let t: [TargetView; 0] = [];
        for press in [4u16, 7, 10, 13] {
            let mut m = DonutMoves::default();
            m.step(&press_swat(&t));
            let mut restart_tick = None;
            for tick in 1..30u16 {
                let input = if tick == press {
                    press_swat(&t)
                } else {
                    base(&t)
                };
                let out = m.step(&input);
                if out.events.iter().any(|e| {
                    matches!(
                        e,
                        MoveEvent::Started {
                            mv: MoveKind::Swat,
                            instance: 2
                        }
                    )
                }) {
                    restart_tick = Some(tick);
                    break;
                }
            }
            assert_eq!(restart_tick, Some(press.max(10)), "press on tick {press}");
            assert_eq!(m.view().unwrap().tick, 0, "the second swat starts at s0");
        }
        // A press on tick 3 is outside the chain window: it is buffered, and
        // it expires (3 + 6) before the move ends on tick 14.
        let mut m = DonutMoves::default();
        m.step(&press_swat(&t));
        let mut second = false;
        for tick in 1..30u16 {
            let input = if tick == 3 { press_swat(&t) } else { base(&t) };
            let out = m.step(&input);
            second |= out
                .events
                .iter()
                .any(|e| matches!(e, MoveEvent::Started { instance: 2, .. }));
        }
        assert!(!second);
    }

    #[test]
    fn a_swat_ends_on_tick_14_after_a_total_of_14_ticks() {
        let t: [TargetView; 0] = [];
        let mut m = DonutMoves::default();
        m.step(&press_swat(&t));
        for tick in 1..=13 {
            let out = m.step(&base(&t));
            assert!(m.busy(), "tick {tick}");
            assert!(
                !out.events
                    .iter()
                    .any(|e| matches!(e, MoveEvent::Ended { .. }))
            );
        }
        let out = m.step(&base(&t));
        assert!(!m.busy());
        assert_eq!(
            out.events,
            vec![MoveEvent::Ended {
                mv: MoveKind::Swat,
                instance: 1
            }]
        );
    }

    #[test]
    fn a_swat_on_the_boss_always_glances_and_adds_poise_in_neutral_only() {
        for (state, poise) in [
            (TargetState::Neutral, 1),
            (TargetState::Tell, 0),
            (TargetState::Impact, 0),
            (TargetState::Staggered, 0),
        ] {
            let mut boss = goblin_at(0, 25.0, state);
            boss.kind = TargetKind::Boss;
            let outs = run_swat(&[boss], 12);
            assert_eq!(
                outs[4].events,
                vec![MoveEvent::Glance {
                    target: 0,
                    reason: GlanceReason::Boss,
                    poise
                }],
                "{state:?}"
            );
            assert!(outs.iter().all(|o| !o.frozen), "no hit-stop on the boss");
        }
    }

    // ----------------------------------------------------------------- guard --

    #[test]
    fn the_guard_needs_both_60_ticks_and_the_next_impact() {
        // Impact early (age 40): still guarded until 60.
        let mut g = LightGuard::new();
        let mut released_at = None;
        for age in 1..=80u16 {
            let state = if (40..44).contains(&age) {
                TargetState::Impact
            } else {
                TargetState::Neutral
            };
            if g.observe(state) {
                released_at = Some(age);
                break;
            }
        }
        assert_eq!(released_at, Some(60));

        // 60 ticks pass with no Impact: still guarded, however late the re-tell.
        let mut g = LightGuard::new();
        for _ in 0..200 {
            assert!(!g.observe(TargetState::Neutral));
        }
        assert!(!g.impact_seen());
        // The first Impact then releases it at once.
        assert!(g.observe(TargetState::Impact));

        // Impact at age 70 (after the 60-tick floor): released on that tick.
        let mut g = LightGuard::new();
        for _ in 1..70 {
            assert!(!g.observe(TargetState::Recovery));
        }
        assert!(g.observe(TargetState::Impact));
    }

    #[test]
    fn the_impact_is_latched_so_a_short_impact_still_counts() {
        let mut g = LightGuard::new();
        assert!(!g.observe(TargetState::Impact));
        assert!(g.impact_seen());
        for _ in 0..30 {
            g.observe(TargetState::Recovery);
        }
        assert!(
            g.impact_seen(),
            "the latch holds once the Impact has passed"
        );
        // age is 31 now; it releases on tick 60 without a second Impact.
        let mut released = None;
        for age in 32..=70 {
            if g.observe(TargetState::Neutral) {
                released = Some(age);
                break;
            }
        }
        assert_eq!(released, Some(60));
    }

    #[test]
    fn the_guard_clears_when_the_target_is_launched_downed_or_defeated() {
        for state in [
            TargetState::Launched,
            TargetState::Downed,
            TargetState::Airborne,
            TargetState::Defeated,
        ] {
            let mut g = LightGuard::new();
            assert!(g.observe(state), "{state:?}");
        }
        // A tell, smear, recovery or stagger does not.
        for state in [
            TargetState::Tell,
            TargetState::Smear,
            TargetState::Recovery,
            TargetState::Staggered,
        ] {
            assert!(!LightGuard::new().observe(state), "{state:?}");
        }
    }

    #[test]
    fn a_goblin_facing_only_donut_lands_every_second_attack() {
        // Scripted goblin: staggered by a swat -> guarded -> re-tells ->
        // swung (Impact) -> 60+ ticks -> the guard is gone and the next tell
        // is swat-interruptible. A re-tell delayed by anything stays guarded.
        let mut guard = Some(LightGuard::new());
        let mut tell_swat_outcomes = Vec::new();
        // 30 ticks neutral (walk-in, spacing), tell, swing, recovery, neutral.
        let script: Vec<TargetState> = std::iter::repeat_n(TargetState::Staggered, 10)
            .chain(std::iter::repeat_n(TargetState::Neutral, 22))
            .chain(std::iter::repeat_n(TargetState::Tell, 24))
            .chain(std::iter::repeat_n(TargetState::Smear, 2))
            .chain(std::iter::repeat_n(TargetState::Impact, 4))
            .chain(std::iter::repeat_n(TargetState::Recovery, 20))
            .chain(std::iter::repeat_n(TargetState::Neutral, 10))
            .collect();
        for state in script {
            if let Some(g) = guard.as_mut()
                && g.observe(state)
            {
                guard = None;
            }
            if state == TargetState::Tell {
                let mut t = goblin_at(0, 25.0, state);
                t.guarded = guard.is_some();
                tell_swat_outcomes.push(matches!(
                    swat_outcome(&t),
                    MoveEvent::Glance {
                        reason: GlanceReason::Guarded,
                        ..
                    }
                ));
            }
        }
        assert!(
            tell_swat_outcomes.iter().all(|glanced| *glanced),
            "the second tell glances"
        );
        assert!(guard.is_none(), "released after 60 ticks and the Impact");
        // Next tell: interruptible again.
        let t = goblin_at(0, 25.0, TargetState::Tell);
        assert!(matches!(
            swat_outcome(&t),
            MoveEvent::TellInterrupted { .. }
        ));
    }

    #[test]
    fn guard_hash_mixes_nothing_when_unset() {
        let mut none = Vec::new();
        hash_guard(&None, &mut |v| none.push(v));
        assert!(none.is_empty());
        let mut some = Vec::new();
        hash_guard(&Some(LightGuard::new()), &mut |v| some.push(v));
        assert!(!some.is_empty());
        let mut later = LightGuard::new();
        later.observe(TargetState::Impact);
        let mut after = Vec::new();
        later.hash_into(&mut |v| after.push(v));
        assert_ne!(some, after);
    }

    // ---------------------------------------------------------------- pounce --

    #[test]
    fn pounce_launches_on_10_touches_down_on_24_and_ends_on_44() {
        let none: [TargetView; 0] = [];
        let (outs, body, m) = run_pounce(&none, 60);
        let at =
            |pred: &dyn Fn(&MoveEvent) -> bool| outs.iter().position(|o| o.events.iter().any(pred));
        assert_eq!(at(&|e| matches!(e, MoveEvent::Launched { .. })), Some(10));
        assert_eq!(at(&|e| matches!(e, MoveEvent::Touchdown { .. })), Some(24));
        assert_eq!(at(&|e| matches!(e, MoveEvent::Ended { .. })), Some(44));
        assert!(!m.busy());
        // Launch impulse, signed by facing.
        assert_eq!(outs[10].launch, Some((180.0, 130.0)));
        assert!(
            outs.iter()
                .enumerate()
                .all(|(t, o)| (t == 10) == o.launch.is_some())
        );
        // Travel: 14 integration steps at 3 px each.
        assert!((body.x - 42.0).abs() <= 1.0, "travel {}", body.x);
    }

    #[test]
    fn pounce_apex_is_at_most_7_px() {
        let none: [TargetView; 0] = [];
        let mut m = DonutMoves::default();
        let mut body = Body::new();
        let mut apex = 0.0f32;
        for t in 0..40 {
            let input = StepInput {
                physical_pressed: t == 0,
                grounded: body.grounded,
                pivot_x: body.x,
                pivot_y: body.y,
                ..base(&none)
            };
            let out = m.step(&input);
            body.apply(&out);
            body.integrate();
            apex = apex.max(body.y);
        }
        assert!(apex <= 7.0 + 0.01 && apex > 6.0, "apex {apex}");
    }

    #[test]
    fn pounce_active_window_and_box_are_launch_to_touchdown_plus_1() {
        let none: [TargetView; 0] = [];
        let (outs, _, _) = run_pounce(&none, 50);
        for (t, out) in outs.iter().enumerate() {
            assert_eq!(out.hitbox.is_some(), (10..=25).contains(&t), "tick {t}");
        }
    }

    #[test]
    fn pounce_locks_input_and_facing_until_touchdown_plus_8() {
        let none: [TargetView; 0] = [];
        let (outs, _, _) = run_pounce(&none, 50);
        for (t, out) in outs.iter().enumerate().take(44) {
            let locked = t < 32; // touchdown 24 + 8
            assert_eq!(out.input_locked, locked, "tick {t}");
            assert_eq!(out.jump_blocked, locked, "tick {t}");
            assert!(out.facing_locked, "tick {t}");
        }
    }

    #[test]
    fn pounce_reaches_a_goblin_centre_under_37_px() {
        for (d, expect) in [(36.0, true), (36.9, true), (37.0, false)] {
            let t = [goblin_at(0, d, TargetState::Neutral)];
            let mut m = DonutMoves::default();
            // Query the box at launch from the pivot (no travel).
            m.step(&press_pounce(&t));
            let mut hit = false;
            for tick in 1..=10 {
                let out = m.step(&StepInput {
                    grounded: tick < 10,
                    ..base(&t)
                });
                hit |= out
                    .events
                    .iter()
                    .any(|e| matches!(e, MoveEvent::PounceHit { .. }));
            }
            assert_eq!(hit, expect, "centre distance {d}");
        }
    }

    #[test]
    fn a_pounce_hits_one_target_lowest_index_first_and_drops_vx_to_zero() {
        // Two goblins in reach; the lower index is the farther one.
        let targets = [
            goblin_at(5, 34.0, TargetState::Neutral),
            goblin_at(2, 36.0, TargetState::Neutral),
        ];
        // Fly the real arc until the first hit (contact stops her beforehand
        // only inside 24 px).
        let (outs, _, _) = run_pounce(&targets, 40);
        let hits: Vec<_> = outs
            .iter()
            .flat_map(|o| o.events.iter())
            .filter_map(|e| match e {
                MoveEvent::PounceHit { target, effect } => Some((*target, *effect)),
                _ => None,
            })
            .collect();
        assert_eq!(hits.len(), 1, "one target per instance");
        let (target, effect) = hits[0];
        assert_eq!(target, 2, "lowest index first");
        assert_eq!(effect.damage, 1);
        assert_eq!(effect.stagger_ticks, 14);
        assert_eq!(effect.push_px_s, 0.0);
        assert_eq!(
            effect.hit_stop,
            HitStopSplit {
                attacker: 4,
                target: 5
            }
        );
        assert!(!effect.sets_guard);
        let tick = outs
            .iter()
            .position(|o| {
                o.events
                    .iter()
                    .any(|e| matches!(e, MoveEvent::PounceHit { .. }))
            })
            .unwrap();
        assert_eq!(outs[tick].set_vx, Some(0.0), "vx goes to 0 on a hit");
        // Donut freezes 4 ticks.
        assert!(outs[tick + 1..tick + 5].iter().all(|o| o.frozen));
        assert!(!outs[tick + 5].frozen);
    }

    #[test]
    fn a_pounce_never_hits_the_dead() {
        let targets = [
            goblin_at(0, 30.0, TargetState::Defeated),
            goblin_at(1, 30.0, TargetState::Airborne),
        ];
        let (outs, _, _) = run_pounce(&targets, 50);
        assert!(!outs.iter().any(|o| {
            o.events
                .iter()
                .any(|e| matches!(e, MoveEvent::PounceHit { .. }))
        }));
    }

    #[test]
    fn a_pounce_stops_at_a_wall_and_at_24_px_contact() {
        let none: [TargetView; 0] = [];
        // Wall.
        let mut m = DonutMoves::default();
        let mut body = Body::new();
        let mut stopped_at = None;
        for t in 0..30u16 {
            let wall = body.x >= 12.0;
            let input = StepInput {
                physical_pressed: t == 0,
                grounded: body.grounded,
                pivot_x: body.x,
                pivot_y: body.y,
                wall_blocked: wall,
                ..base(&none)
            };
            let out = m.step(&input);
            body.apply(&out);
            body.integrate();
            if out.set_vx == Some(0.0) && stopped_at.is_none() {
                stopped_at = Some(body.x);
            }
        }
        assert!(stopped_at.is_some());
        assert!(body.x < 16.0, "she stops at the wall: {}", body.x);

        // Body contact: a living goblin within 24 px of centre. Use a goblin
        // out of the box's vertical reach so only the contact rule applies:
        // place it so its centre is 24 px ahead but above the box top.
        let mut g = goblin_at(0, 24.0, TargetState::Neutral);
        g.body.bottom = 30.0;
        let targets = [g];
        let (outs, body, _) = run_pounce(&targets, 40);
        assert!(outs[10].set_vx == Some(0.0), "contact at launch stops vx");
        assert!(body.x < 1.0, "never passes through or over: {}", body.x);
        // A dead goblin is no contact.
        g.state = TargetState::Defeated;
        let (_, body, _) = run_pounce(&[g], 40);
        assert!(body.x > 40.0);
    }

    /// A goblin whose centre is `centre_distance` from x = 0 but whose body
    /// sits above the pounce box, so only the contact rule can see it.
    fn goblin_above_box(centre_distance: f32, kind: TargetKind) -> TargetView {
        let mut g = goblin_at(0, centre_distance, TargetState::Neutral);
        g.body.bottom = 30.0;
        g.kind = kind;
        g
    }

    fn pounce_hits(outs: &[StepOutput]) -> Vec<(u32, TargetEffect)> {
        outs.iter()
            .flat_map(|o| o.events.iter())
            .filter_map(|e| match e {
                MoveEvent::PounceHit { target, effect } => Some((*target, *effect)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn q10_a04_a_goblin_behind_does_not_stop_the_pounce() {
        let (_, free, _) = run_pounce(&[], 60);
        assert!(free.x > 40.0 && free.x < 44.0, "free travel: {}", free.x);
        // Facing right, goblin 10 px behind.
        let (outs, body, _) = run_pounce(&[goblin_above_box(-10.0, TargetKind::Goblin)], 60);
        assert!(outs.iter().all(|o| o.set_vx != Some(0.0)));
        assert!((body.x - free.x).abs() < 1e-3, "travel {}", body.x);
        // Facing left, goblin 10 px behind (at +10).
        let (outs, body, _) =
            run_pounce_facing(&[goblin_above_box(10.0, TargetKind::Goblin)], 60, -1);
        assert!(outs.iter().all(|o| o.set_vx != Some(0.0)));
        assert!((body.x + free.x).abs() < 1e-3, "travel {}", body.x);
    }

    #[test]
    fn q10_a04_a_goblin_at_zero_and_ahead_still_stops_the_pounce() {
        for (d, facing) in [(0.0, 1), (10.0, 1), (-10.0, -1), (24.0, 1)] {
            let (outs, body, _) =
                run_pounce_facing(&[goblin_above_box(d, TargetKind::Goblin)], 40, facing);
            assert_eq!(outs[10].set_vx, Some(0.0), "goblin at {d}, facing {facing}");
            assert!(body.x.abs() < 1.0, "goblin at {d}: {}", body.x);
        }
    }

    #[test]
    fn q10_a04_the_boss_is_never_a_contact_target() {
        let (_, free, _) = run_pounce(&[], 60);
        let (outs, body, _) = run_pounce(&[goblin_above_box(20.0, TargetKind::Boss)], 60);
        assert!(outs.iter().all(|o| o.set_vx != Some(0.0)));
        assert!((body.x - free.x).abs() < 1e-3, "travel {}", body.x);
    }

    #[test]
    fn q10_a03_a_pounce_hits_launched_and_downed_goblins() {
        for state in [TargetState::Launched, TargetState::Downed] {
            let targets = [goblin_at(3, 30.0, state)];
            let (outs, _, _) = run_pounce(&targets, 50);
            let hits = pounce_hits(&outs);
            assert_eq!(hits.len(), 1, "{state:?}");
            assert_eq!(hits[0].0, 3);
            assert_eq!(hits[0].1.damage, 1);
            assert_eq!(hits[0].1.hit_stop.attacker, 4);
        }
    }

    #[test]
    fn q10_a03_a_pounce_hits_the_boss_with_the_same_effect() {
        let mut boss = goblin_at(7, 30.0, TargetState::Neutral);
        boss.kind = TargetKind::Boss;
        let (outs, _, _) = run_pounce(&[boss], 50);
        let hits = pounce_hits(&outs);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, 7);
        assert_eq!(hits[0].1.damage, 1);
        assert_eq!(hits[0].1.hit_stop.attacker, 4);
        // The boss adapter (`combat_scene::donut`) applies this through the
        // boss hero-hit rule (#115) and ignores `stagger_ticks`.
    }

    #[test]
    fn a_pounce_over_a_ledge_lapses_after_20_air_ticks() {
        let none: [TargetView; 0] = [];
        let mut m = DonutMoves::default();
        let mut lapsed_at = None;
        for t in 0..60u16 {
            // The floor vanished: Donut never lands.
            let input = StepInput {
                physical_pressed: t == 0,
                grounded: t < 11,
                ..base(&none)
            };
            let out = m.step(&input);
            if out
                .events
                .iter()
                .any(|e| matches!(e, MoveEvent::Lapsed { .. }))
                && lapsed_at.is_none()
            {
                lapsed_at = Some(t);
                assert!(out.hitbox.is_none(), "the box turns off");
                assert!(!out.input_locked, "the lock ends");
                assert!(!m.busy());
            }
            assert!(
                !out.events
                    .iter()
                    .any(|e| matches!(e, MoveEvent::Touchdown { .. })),
                "no touchdown, no pin"
            );
        }
        assert_eq!(lapsed_at, Some(30), "launch tick 10 + 20 air ticks");
    }

    #[test]
    fn a_pounce_never_starts_airborne() {
        let none: [TargetView; 0] = [];
        let mut m = DonutMoves::default();
        let out = m.step(&StepInput {
            physical_pressed: true,
            grounded: false,
            ..base(&none)
        });
        assert!(!m.busy());
        assert!(out.events.is_empty());
        // The press is buffered and starts on landing within the window.
        let out = m.step(&StepInput {
            grounded: true,
            ..base(&none)
        });
        assert!(m.busy());
        assert!(out.events.iter().any(|e| matches!(
            e,
            MoveEvent::Started {
                mv: MoveKind::Pounce,
                ..
            }
        )));
    }

    #[test]
    fn a_pounce_cancels_after_touchdown_plus_8_by_move_or_jump_and_not_before() {
        let none: [TargetView; 0] = [];
        let cancel_at = |tick_press: u16, jump: bool| {
            let mut m = DonutMoves::default();
            let mut body = Body::new();
            for t in 0..44u16 {
                let input = StepInput {
                    physical_pressed: t == 0,
                    jump_pressed: jump && t == tick_press,
                    move_pressed: !jump && t == tick_press,
                    grounded: body.grounded,
                    pivot_x: body.x,
                    pivot_y: body.y,
                    ..base(&none)
                };
                let out = m.step(&input);
                body.apply(&out);
                body.integrate();
                if out
                    .events
                    .iter()
                    .any(|e| matches!(e, MoveEvent::Cancelled { .. }))
                {
                    return Some(t);
                }
            }
            None
        };
        for jump in [true, false] {
            assert_eq!(
                cancel_at(31, jump),
                None,
                "tick 31 is still locked (jump {jump})"
            );
            assert_eq!(
                cancel_at(32, jump),
                Some(32),
                "touchdown 24 + 8 (jump {jump})"
            );
            assert_eq!(cancel_at(40, jump), Some(40));
        }
        // Never mid-flight.
        assert_eq!(cancel_at(15, true), None);
    }

    #[test]
    fn a_whiff_exposes_her_for_32_ticks_longer_than_a_goblin_tell_to_impact() {
        let none: [TargetView; 0] = [];
        let (outs, _, _) = run_pounce(&none, 44);
        let locked = outs.iter().filter(|o| o.input_locked).count();
        assert_eq!(locked, 32);
        assert!(locked > 26, "a goblin's tell to impact is 26 ticks");
    }

    // --------------------------------------------------------------- shared --

    #[test]
    fn a_hurt_cancels_either_move_and_the_buffer() {
        let none: [TargetView; 0] = [];
        for (press, mv) in [(true, MoveKind::Swat), (false, MoveKind::Pounce)] {
            let mut m = DonutMoves::default();
            m.step(&StepInput {
                primary_pressed: press,
                physical_pressed: !press,
                ..base(&none)
            });
            m.step(&StepInput {
                primary_pressed: true,
                ..base(&none)
            }); // buffered / chained
            let out = m.step(&StepInput {
                hurt: true,
                ..base(&none)
            });
            assert!(!m.busy());
            assert_eq!(m.hit_stop_left(), 0);
            assert_eq!(
                out.events,
                vec![MoveEvent::Cancelled {
                    mv,
                    instance: 1,
                    cause: CancelCause::Hurt
                }]
            );
            assert!(!out.input_locked && !out.facing_locked && out.vx_cap.is_none());
            // Nothing starts later from the dropped buffer.
            for _ in 0..10 {
                let out = m.step(&base(&none));
                assert!(out.events.is_empty());
            }
        }
    }

    #[test]
    fn a_hurt_in_hit_stop_ends_the_freeze() {
        let t = [goblin_at(0, 25.0, TargetState::Neutral)];
        let mut m = DonutMoves::default();
        for tick in 0..5 {
            m.step(&if tick == 0 { press_swat(&t) } else { base(&t) });
        }
        assert_eq!(m.hit_stop_left(), 2);
        m.step(&StepInput {
            hurt: true,
            ..base(&t)
        });
        assert_eq!(m.hit_stop_left(), 0);
    }

    #[test]
    fn the_buffer_holds_a_press_for_6_ticks_and_drops_it_on_tick_7() {
        let none: [TargetView; 0] = [];
        // Airborne at the press: the press starts on landing only while the
        // buffer holds. Press on tick 0, land on tick N.
        for (land, expect_start) in [(6u16, true), (7, false)] {
            let mut m = DonutMoves::default();
            m.step(&StepInput {
                primary_pressed: true,
                grounded: false,
                ..base(&none)
            });
            let mut started = false;
            for tick in 1..=10u16 {
                let out = m.step(&StepInput {
                    grounded: tick >= land,
                    ..base(&none)
                });
                started |= out
                    .events
                    .iter()
                    .any(|e| matches!(e, MoveEvent::Started { .. }));
            }
            assert_eq!(started, expect_start, "landing on tick {land}");
        }
    }

    #[test]
    fn a_swap_clears_the_buffer_and_the_ai_finishes_a_running_move_and_starts_none() {
        let none: [TargetView; 0] = [];
        let mut m = DonutMoves::default();
        m.step(&StepInput {
            physical_pressed: true,
            grounded: false,
            ..base(&none)
        });
        m.clear_buffer();
        let out = m.step(&base(&none));
        assert!(
            !m.busy() && out.events.is_empty(),
            "a swap drops the buffered press"
        );

        // The AI (may_start false after the press): a running pounce finishes,
        // a later press is neither started nor remembered.
        let mut m = DonutMoves::default();
        let mut body = Body::new();
        let mut ended = false;
        for t in 0..60u16 {
            let input = StepInput {
                physical_pressed: t == 0,
                primary_pressed: t == 20,
                may_start: t == 0,
                grounded: body.grounded,
                pivot_x: body.x,
                pivot_y: body.y,
                ..base(&none)
            };
            let out = m.step(&input);
            body.apply(&out);
            body.integrate();
            ended |= out.events.iter().any(|e| {
                matches!(
                    e,
                    MoveEvent::Ended {
                        mv: MoveKind::Pounce,
                        ..
                    }
                )
            });
            assert!(
                !out.events
                    .iter()
                    .any(|e| matches!(e, MoveEvent::Started { instance: 2, .. })),
                "the follower starts no move (tick {t})"
            );
        }
        assert!(ended, "the running move finished on its own clock");
    }

    #[test]
    fn a_press_while_busy_is_buffered_and_starts_when_the_move_ends() {
        let none: [TargetView; 0] = [];
        let mut m = DonutMoves::default();
        m.step(&press_swat(&none));
        // A pounce press on tick 9 starts at tick 14 when the swat ends
        // (9 + 6 = 15 >= 14).
        let mut started = None;
        for tick in 1..30u16 {
            let input = if tick == 9 {
                press_pounce(&none)
            } else {
                base(&none)
            };
            let out = m.step(&input);
            if out.events.iter().any(|e| {
                matches!(
                    e,
                    MoveEvent::Started {
                        mv: MoveKind::Pounce,
                        ..
                    }
                )
            }) {
                started = Some(tick);
            }
        }
        assert_eq!(started, Some(14));
    }

    // ------------------------------------------------- mapping and dry press --

    #[test]
    fn primary_routes_by_biscuit_mana_and_the_empty_press_rule() {
        use PrimaryRoute::*;
        // route_primary(donut_moves, unlocked, can_cast, empty_press_swat)
        assert_eq!(
            route_primary(true, false, false, false),
            Swat,
            "pre-biscuit swat"
        );
        assert_eq!(
            route_primary(false, false, false, false),
            Off,
            "attack-less Donut"
        );
        assert_eq!(
            route_primary(true, true, true, false),
            Cast,
            "post-biscuit with mana"
        );
        assert_eq!(
            route_primary(true, true, false, false),
            Dry,
            "0 mana: dry and final"
        );
        assert_eq!(
            route_primary(true, true, false, true),
            Swat,
            "playtest variant"
        );
        assert_eq!(
            route_primary(false, true, false, true),
            Dry,
            "swat needs donut_moves"
        );
        assert_eq!(route_primary(false, true, true, false), Cast);
    }

    #[test]
    fn a_dry_press_is_an_event_only_with_no_lock_no_buffer_and_no_swat() {
        let t = [goblin_at(0, 20.0, TargetState::Neutral)];
        let mut m = DonutMoves::default();
        let dry = StepInput {
            primary_pressed: true,
            primary_route: PrimaryRoute::Dry,
            ..base(&t)
        };
        let out = m.step(&dry);
        assert_eq!(out.events, vec![MoveEvent::DryPress]);
        assert!(!m.busy());
        assert!(
            !out.input_locked && !out.facing_locked && !out.jump_blocked && out.vx_cap.is_none()
        );
        assert!(out.hitbox.is_none() && !out.cast_requested && !out.frozen);
        let mut hash = Vec::new();
        m.hash_into(&mut |v| hash.push(v));
        assert!(hash.is_empty(), "nothing buffered");
        // Mash it: every press is a final dry press, never a swat, and nothing
        // fires later when mana returns.
        for _ in 0..20 {
            let out = m.step(&dry);
            assert_eq!(out.events, vec![MoveEvent::DryPress]);
        }
        let out = m.step(&StepInput {
            primary_route: PrimaryRoute::Cast,
            ..base(&t)
        });
        assert!(
            out.events.is_empty() && !out.cast_requested,
            "no surprise cast from a dropped press"
        );
        // A dry press while a pounce runs is still just an event.
        let mut m = DonutMoves::default();
        m.step(&press_pounce(&t));
        let out = m.step(&dry);
        assert_eq!(out.events, vec![MoveEvent::DryPress]);
        assert!(m.view().is_some());
    }

    #[test]
    fn a_cast_press_is_forwarded_and_a_busy_cast_press_is_buffered_6_ticks() {
        let none: [TargetView; 0] = [];
        let cast = |pressed: bool| StepInput {
            primary_pressed: pressed,
            primary_route: PrimaryRoute::Cast,
            ..base(&none)
        };
        let mut m = DonutMoves::default();
        assert!(m.step(&cast(true)).cast_requested);
        assert!(!m.busy(), "the cast is not a swat or pounce move");
        // During a pounce (busy) the press is remembered.
        let mut m = DonutMoves::default();
        m.step(&press_pounce(&none));
        m.step(&cast(true));
        let mut released = None;
        for tick in 2..60u16 {
            if m.step(&cast(false)).cast_requested {
                released = Some(tick);
            }
        }
        assert_eq!(released, None, "6 ticks is shorter than a pounce: dropped");
    }

    // --------------------------------------------------- determinism and hash --

    #[test]
    fn idle_moves_hash_nothing_and_a_running_move_hashes_its_state() {
        let none: [TargetView; 0] = [];
        let mut m = DonutMoves::default();
        for _ in 0..100 {
            m.step(&base(&none));
        }
        let mut idle = Vec::new();
        m.hash_into(&mut |v| idle.push(v));
        assert!(idle.is_empty(), "the clock is never mixed");
        m.step(&press_swat(&none));
        let mut a = Vec::new();
        m.hash_into(&mut |v| a.push(v));
        m.step(&base(&none));
        let mut b = Vec::new();
        m.hash_into(&mut |v| b.push(v));
        assert!(!a.is_empty());
        assert_ne!(a, b);
    }

    #[test]
    fn identical_inputs_give_identical_outputs_and_hashes() {
        let play = || {
            let targets = [
                goblin_at(1, 30.0, TargetState::Tell),
                goblin_at(0, 34.0, TargetState::Neutral),
            ];
            let mut m = DonutMoves::default();
            let mut body = Body::new();
            let mut log = Vec::new();
            let mut hash = 0xcbf2_9ce4_8422_2325_u64;
            for t in 0..120u16 {
                let input = StepInput {
                    primary_pressed: t % 37 == 0,
                    physical_pressed: t % 53 == 5,
                    jump_pressed: t % 29 == 0,
                    move_pressed: t % 41 == 0,
                    grounded: body.grounded,
                    pivot_x: body.x,
                    pivot_y: body.y,
                    ..base(&targets)
                };
                let out = m.step(&input);
                body.apply(&out);
                if !out.frozen {
                    body.integrate();
                }
                m.hash_into(&mut |v| {
                    hash ^= v;
                    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
                });
                log.push((out.events.clone(), out.frozen, out.launch, out.set_vx));
            }
            (log, hash)
        };
        assert_eq!(play(), play());
    }

    #[test]
    fn a_facing_left_pounce_launches_leftwards() {
        let none: [TargetView; 0] = [];
        let mut m = DonutMoves::default();
        m.step(&StepInput {
            physical_pressed: true,
            facing: -1,
            ..base(&none)
        });
        let mut launch = None;
        for _ in 1..=10 {
            let out = m.step(&StepInput {
                facing: -1,
                ..base(&none)
            });
            launch = launch.or(out.launch);
        }
        assert_eq!(launch, Some((-180.0, 130.0)));
    }
}
