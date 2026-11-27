//! CORRAL-AUTO-01 action policies (versioned, [`super::POLICY_VERSION`]).
//!
//! A policy is a pure function of the **observable** state (the previous
//! tick's finished state: positions, health, mana, the cast in progress, the
//! llamas' modes and rush phases) and of the room's static geometry ([`Aim`]:
//! the solids, Donut's own jump arc, the bolt's flight), plus its own press
//! bookkeeping. It returns one raw input per **hero** with true press edges (a
//! press tick, then a release tick before the next legal cast), and the runner
//! routes them to the party's slots. A policy never reads the future, never
//! switches to a stronger policy to turn a loss into a win, and never calls
//! anything that writes game state.
//!
//! CORRAL-C (spec section 11, CRL-Q34, CRL-Q42) adds the three gated static
//! policies `stationary_cast_aimed`, `stationary_turret` and
//! `stationary_jump_cast` (a swept press offset `o`, plus its `aimed` variant):
//! a hop in place and a turn in place still count as standing still.

use crate::input::RawInput;
use crate::llama::LlamaMode;
use crate::llama_rush::RushPhase;
use crate::llama_rush::geometry::llama_body;
use crate::prototype::{Block, Rect};

/// Which policy drives the tested hero(es).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PolicyId {
    /// Neutral inputs: what the llamas do to a hero who plays nothing.
    NoInteraction,
    /// `attack` held for the whole run: one press, not many spells. The "press
    /// continuously" no-op of #181, run and labelled trivial.
    HoldAttackTrivial,
    /// Donut stands still (no move, no jump) and presses a new Magic Missile
    /// whenever a legal cast can begin and mana permits, through the regen.
    StationaryCast,
    /// Same stand, but spends the pool in a closely spaced burst and waits for
    /// a full pool before the next one.
    StationaryCastBurst,
    /// Same stand; presses only when a living llama body meets the bolt's
    /// line within its reach with no solid between (the strongest legal
    /// schedule of a planted Donut).
    StationaryCastAimed,
    /// As aimed, but turns in place (a one-tick stick tap) toward a llama that
    /// is in line behind her.
    StationaryTurret,
    /// Every Primary press is paired with a Jump press (the run's `o`: Primary
    /// press tick minus Jump press tick, -3..52); a new Jump only while
    /// grounded; turn taps as the turret (CRL-Q42).
    StationaryJumpCast,
    /// The jump-cast whose `o` is picked before each press: the smallest one
    /// (a grounded cast first) whose release height puts the bolt on a living
    /// llama in line with no solid between.
    StationaryJumpCastAimed,
    /// Donut moves to keep a firing band, faces the nearest llama, dodges a
    /// rush tell and (in the corral) climbs to the dock. Reads only the
    /// current state.
    MobileCast,
    /// Carl closes with the nearest llama, jumps its rush and jabs it (solo as
    /// Carl).
    CarlMelee,
    /// P1 Carl melee and P2 Donut stationary cast on the same ticks.
    CoopProbe,
}

impl PolicyId {
    pub const ALL: [PolicyId; 11] = [
        PolicyId::NoInteraction,
        PolicyId::HoldAttackTrivial,
        PolicyId::StationaryCast,
        PolicyId::StationaryCastBurst,
        PolicyId::StationaryCastAimed,
        PolicyId::StationaryTurret,
        PolicyId::StationaryJumpCast,
        PolicyId::StationaryJumpCastAimed,
        PolicyId::MobileCast,
        PolicyId::CarlMelee,
        PolicyId::CoopProbe,
    ];

    /// The gated static policies of CRL-Q34 except the swept jump-cast.
    pub const GATED_FIXED: [PolicyId; 5] = [
        PolicyId::StationaryCast,
        PolicyId::StationaryCastBurst,
        PolicyId::StationaryCastAimed,
        PolicyId::StationaryTurret,
        PolicyId::StationaryJumpCastAimed,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            PolicyId::NoInteraction => "no_interaction",
            PolicyId::HoldAttackTrivial => "hold_attack_trivial",
            PolicyId::StationaryCast => "stationary_cast",
            PolicyId::StationaryCastBurst => "stationary_cast_burst",
            PolicyId::StationaryCastAimed => "stationary_cast_aimed",
            PolicyId::StationaryTurret => "stationary_turret",
            PolicyId::StationaryJumpCast => "stationary_jump_cast",
            PolicyId::StationaryJumpCastAimed => "stationary_jump_cast_aimed",
            PolicyId::MobileCast => "mobile_cast",
            PolicyId::CarlMelee => "carl_melee",
            PolicyId::CoopProbe => "coop_probe",
        }
    }

    /// The stationary policies the KC19 gate and the CRL-Q35 invariant are
    /// defined on (CRL-Q34: a hop or a turn in place is still stationary).
    pub const fn is_stationary_gate(self) -> bool {
        matches!(
            self,
            PolicyId::StationaryCast
                | PolicyId::StationaryCastBurst
                | PolicyId::StationaryCastAimed
                | PolicyId::StationaryTurret
                | PolicyId::StationaryJumpCast
                | PolicyId::StationaryJumpCastAimed
        )
    }

    /// The policy jumps (its `donut_move_inputs` counts its jump presses).
    pub const fn jumps(self) -> bool {
        matches!(
            self,
            PolicyId::StationaryJumpCast | PolicyId::StationaryJumpCastAimed
        )
    }
}

/// One hero as the policy sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeroObs {
    pub x: f32,
    pub facing: i8,
    pub health: u8,
    pub grounded: bool,
    /// World y of the feet (y up).
    pub feet: f32,
    /// World y of the top of the highest solid under her (her support).
    pub support: f32,
}

/// One llama as the policy sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LlamaObs {
    pub x: f32,
    /// World y of its feet (its surface).
    pub feet: f32,
    pub facing: i8,
    pub health: u8,
    pub mode: LlamaMode,
    pub attack_tick: u16,
    /// The rush phase while a rush runs (a corral llama only).
    pub rush: Option<RushPhase>,
    /// TROG-B: the target's own hurtbox (world px), for a slot enemy that is no llama (a
    /// troglodyte, `trog_eval`). `None` for a llama: its body is the llama geometry.
    pub rect: Option<Rect>,
}

impl LlamaObs {
    pub fn alive(&self) -> bool {
        self.health > 0 && !matches!(self.mode, LlamaMode::Down | LlamaMode::Corpse)
    }
    pub fn telling(&self) -> bool {
        self.mode == LlamaMode::Attack
    }
    pub fn rush_telling(&self) -> bool {
        self.rush == Some(RushPhase::Tell)
    }
    /// A spit tell (an attack that is not a rush).
    pub fn spit_telling(&self) -> bool {
        self.mode == LlamaMode::Attack && self.rush.is_none()
    }
    pub fn body(&self) -> Rect {
        self.rect
            .unwrap_or_else(|| llama_body(self.x, self.feet, self.facing))
    }
}

/// What a policy may read: the finished state of the previous tick.
#[derive(Debug, Clone)]
pub struct Obs {
    /// The tick about to be stepped (1-based, like the scene's `tick`).
    pub tick: u64,
    pub carl: HeroObs,
    pub donut: HeroObs,
    /// `(mana, mana_max)` once the biscuit is unlocked.
    pub mana: Option<(u8, u8)>,
    /// A Magic Missile cast is in progress (its press has been taken).
    pub cast_active: bool,
    /// The hero is in a hurt flinch: it cannot start a cast.
    pub donut_hurt: bool,
    pub llamas: Vec<LlamaObs>,
}

/// One raw input per hero. The runner routes it to the party's slots.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HeroInputs {
    pub carl: RawInput,
    pub donut: RawInput,
}

/// The corral's route for the movement policies: the walking line and the
/// dock bumper they climb (static room geometry, world px).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Route {
    pub floor: f32,
    pub bumper_left: f32,
    pub bumper_right: f32,
    pub bumper_top: f32,
}

/// The static facts every policy may read: the room's solids, the bolt's
/// flight and Donut's own jump in place from her start (the arc a player
/// knows by feel). Built once per run from the setup, never from later state.
#[derive(Debug, Clone, Default)]
pub struct Aim {
    pub solids: Vec<Block>,
    /// Bolt spawn ahead of and above the pivot, step per tick, range.
    pub bolt_forward: f32,
    pub bolt_up: f32,
    pub bolt_step: f32,
    pub bolt_range: f32,
    /// Donut's feet above her start surface after each jump tick of a jump in
    /// place (index 0 = jump tick 1, the press tick), to the landing.
    pub arc: Vec<f32>,
    pub route: Option<Route>,
}

impl Aim {
    /// Every box a bolt released at `feet` from `pivot` tests, in order, until
    /// a solid ends it or its range does (solids tested before targets; the
    /// `combat_scene::Bolt` geometry).
    pub fn bolt_path(&self, pivot: f32, facing: i8, feet: f32) -> Vec<Rect> {
        let mut bolt = crate::combat_scene::Bolt {
            instance: 1,
            x: pivot + f32::from(facing) * self.bolt_forward,
            y: feet + self.bolt_up,
            dir: facing,
            travelled: 0.0,
        };
        let mut path = Vec::new();
        if self.bolt_step <= 0.0 {
            return path;
        }
        loop {
            let rect = bolt.rect();
            if crate::combat::solid_at(&self.solids, rect) {
                return path;
            }
            path.push(rect);
            bolt.x += f32::from(facing) * self.bolt_step;
            bolt.travelled += self.bolt_step;
            if bolt.travelled >= self.bolt_range {
                return path;
            }
        }
    }

    /// A bolt released now from `pivot` at `feet` in `facing` meets a living
    /// llama body (at its current position) with no solid between.
    pub fn bolt_hits(&self, pivot: f32, facing: i8, feet: f32, llamas: &[LlamaObs]) -> bool {
        let bodies: Vec<Rect> = llamas
            .iter()
            .filter(|l| l.alive())
            .map(LlamaObs::body)
            .collect();
        !bodies.is_empty()
            && self
                .bolt_path(pivot, facing, feet)
                .iter()
                .any(|r| bodies.iter().any(|b| r.overlaps(*b)))
    }

    /// The feet height above her start surface at the release of a jump-cast
    /// with offset `o` (release on jump tick `o + 5`), or `None` once landed.
    pub fn release_rise(&self, o: i8) -> Option<f32> {
        let tick = i32::from(o) + 5;
        if tick < 1 {
            return None;
        }
        self.arc.get(tick as usize - 1).copied()
    }
}

/// Magic Missile reach (pivot to pivot, px) the mobile policy aims inside:
/// the pinned 240 px range with a margin for the body half-width.
const SHOOT_BAND: (f32, f32) = (130.0, 225.0);
/// Carl's jab reach the melee policy closes to.
const JAB_REACH: f32 = 30.0;
/// The offsets of the jump-cast sweep (CRL-Q42): -3..52.
pub const JUMP_OFFSETS: core::ops::RangeInclusive<i8> = -3..=52;
/// A llama farther than this is never a target (CRL-Q15).
const TARGET_RANGE: f32 = 320.0;

fn toward(from: f32, to: f32) -> f32 {
    if to >= from { 1.0 } else { -1.0 }
}

fn nearest(from: f32, llamas: &[LlamaObs]) -> Option<&LlamaObs> {
    llamas
        .iter()
        .filter(|l| l.alive())
        .min_by(|a, b| (a.x - from).abs().total_cmp(&(b.x - from).abs()))
}

/// The next llama along the room's route: the living one with the lowest x
/// (B before A in the corral).
fn next_on_route(llamas: &[LlamaObs]) -> Option<&LlamaObs> {
    llamas
        .iter()
        .filter(|l| l.alive())
        .min_by(|a, b| a.x.total_cmp(&b.x))
}

/// One scheduled jump-cast: the tick of its Jump and of its Primary press.
#[derive(Debug, Clone, Copy)]
struct Cycle {
    jump_at: u64,
    press_at: Option<u64>,
}

/// A policy instance: [`PolicyId`] plus its press bookkeeping.
#[derive(Debug, Clone)]
pub struct Policy {
    id: PolicyId,
    /// The jump-cast offset `o` of this run.
    offset: i8,
    aim: Aim,
    /// The previous tick carried a press: this tick is the release.
    released_due: bool,
    /// Burst phase: refilling the pool before the next burst.
    refilling: bool,
    /// Presses issued (press edges, never held ticks).
    pub presses: u32,
    /// Jump presses issued (the jump-cast policies).
    pub jumps: u32,
    /// The jab of the melee policy alternates press and release.
    jab_pressed: bool,
    /// The previous tick carried a turn tap.
    tapped: bool,
    cycle: Option<Cycle>,
    /// The movement policies' previous tick carried a jump press.
    jump_held: bool,
    /// The movement policies' hero column on the previous tick and the ticks
    /// it has been pushing against a face without moving (a step to hop).
    last_x: [f32; 2],
    blocked: [u8; 2],
}

impl Policy {
    pub fn new(id: PolicyId) -> Self {
        Self::with_aim(id, 0, Aim::default())
    }

    pub fn with_aim(id: PolicyId, offset: i8, aim: Aim) -> Self {
        Self {
            id,
            offset,
            aim,
            released_due: false,
            refilling: false,
            presses: 0,
            jumps: 0,
            jab_pressed: false,
            tapped: false,
            cycle: None,
            jump_held: false,
            last_x: [f32::NAN; 2],
            blocked: [0; 2],
        }
    }

    pub fn id(&self) -> PolicyId {
        self.id
    }

    pub fn act(&mut self, obs: &Obs) -> HeroInputs {
        let donut = match self.id {
            PolicyId::NoInteraction | PolicyId::CarlMelee => RawInput::NONE,
            PolicyId::HoldAttackTrivial => {
                // Counted as one press: the edge is the first tick only.
                if self.presses == 0 {
                    self.presses = 1;
                }
                RawInput {
                    attack: true,
                    ..RawInput::NONE
                }
            }
            PolicyId::StationaryCast | PolicyId::CoopProbe => self.stationary(obs, false),
            PolicyId::StationaryCastBurst => self.stationary(obs, true),
            PolicyId::StationaryCastAimed => self.aimed(obs, false),
            PolicyId::StationaryTurret => self.aimed(obs, true),
            PolicyId::StationaryJumpCast => self.jump_cast(obs, false),
            PolicyId::StationaryJumpCastAimed => self.jump_cast(obs, true),
            PolicyId::MobileCast => {
                if self.aim.route.is_some() {
                    self.mobile_route(obs)
                } else {
                    self.mobile(obs)
                }
            }
        };
        let carl = match self.id {
            PolicyId::CarlMelee | PolicyId::CoopProbe => self.melee(obs),
            _ => RawInput::NONE,
        };
        HeroInputs { carl, donut }
    }

    fn can_cast(obs: &Obs) -> bool {
        obs.mana.is_some_and(|(m, _)| m > 0) && !obs.cast_active && !obs.donut_hurt
    }

    fn press(&mut self) -> RawInput {
        self.released_due = true;
        self.presses += 1;
        RawInput {
            attack: true,
            ..RawInput::NONE
        }
    }

    /// A press edge on the next legal cast: press for one tick, release the
    /// next, then wait for the cast to end (16-tick refire) before pressing
    /// again. Never moves, never jumps.
    fn stationary(&mut self, obs: &Obs, burst: bool) -> RawInput {
        if self.released_due {
            self.released_due = false;
            return RawInput::NONE;
        }
        let Some((mana, max)) = obs.mana else {
            return RawInput::NONE;
        };
        if burst {
            if self.refilling && mana == max {
                self.refilling = false;
            }
            if mana == 0 {
                self.refilling = true;
            }
            if self.refilling {
                return RawInput::NONE;
            }
        } else if mana == 0 {
            return RawInput::NONE;
        }
        if obs.cast_active || obs.donut_hurt {
            return RawInput::NONE;
        }
        self.press()
    }

    /// The turn tap of the turret and the jump-cast: when nothing is in line
    /// ahead and a living llama within 320 px is in line behind (or, for the
    /// jump-cast, simply behind), one tick of stick toward it turns her.
    fn turn_tap(&mut self, obs: &Obs, behind_in_line: bool) -> Option<RawInput> {
        if self.tapped {
            self.tapped = false;
            return None;
        }
        let me = obs.donut;
        if !me.grounded {
            return None;
        }
        let back = -me.facing;
        let wants = obs.llamas.iter().any(|l| {
            l.alive()
                && (l.x - me.x).abs() <= TARGET_RANGE
                && toward(me.x, l.x) == f32::from(back)
                && (!behind_in_line
                    || self
                        .aim
                        .bolt_hits(me.x, back, me.feet, std::slice::from_ref(l)))
        });
        let ahead = self.aim.bolt_hits(me.x, me.facing, me.feet, &obs.llamas);
        if wants && !ahead {
            self.tapped = true;
            return Some(RawInput {
                move_x: f32::from(back),
                ..RawInput::NONE
            });
        }
        None
    }

    /// `stationary_cast_aimed` and `stationary_turret`: press only when the
    /// bolt released from where she stands meets a living llama.
    fn aimed(&mut self, obs: &Obs, turret: bool) -> RawInput {
        if self.released_due {
            self.released_due = false;
            return RawInput::NONE;
        }
        if turret && let Some(tap) = self.turn_tap(obs, true) {
            return tap;
        }
        let me = obs.donut;
        if Self::can_cast(obs) && self.aim.bolt_hits(me.x, me.facing, me.feet, &obs.llamas) {
            return self.press();
        }
        RawInput::NONE
    }

    /// `stationary_jump_cast` (fixed `o`) and its `aimed` variant.
    fn jump_cast(&mut self, obs: &Obs, aimed: bool) -> RawInput {
        let me = obs.donut;
        let mut raw = RawInput::NONE;
        let release_tick = std::mem::take(&mut self.released_due);
        if let Some(cycle) = self.cycle {
            if obs.tick == cycle.jump_at {
                raw.jump = true;
                self.jumps += 1;
            }
            if cycle.press_at == Some(obs.tick) && Self::can_cast(obs) {
                raw.attack = true;
                self.released_due = true;
                self.presses += 1;
            }
            let last = cycle.jump_at.max(cycle.press_at.unwrap_or(0));
            // Over once both presses are behind her and she is down again.
            if obs.tick > last && obs.tick > cycle.jump_at + 2 && me.grounded {
                self.cycle = None;
            }
            return raw;
        }
        if release_tick || !me.grounded || !Self::can_cast(obs) {
            return raw;
        }
        if let Some(tap) = self.turn_tap(obs, aimed) {
            return tap;
        }
        let surface = me.feet;
        if aimed {
            // A grounded cast first, then the smallest offset whose release
            // height puts the bolt on a living llama.
            if self.aim.bolt_hits(me.x, me.facing, surface, &obs.llamas) {
                return self.press();
            }
            let pick = JUMP_OFFSETS.into_iter().find(|&o| {
                self.aim.release_rise(o).is_some_and(|rise| {
                    self.aim
                        .bolt_hits(me.x, me.facing, surface + rise, &obs.llamas)
                })
            });
            if let Some(o) = pick {
                self.start_cycle(obs.tick, o);
                return self.jump_cast(obs, aimed);
            }
            return raw;
        }
        self.start_cycle(obs.tick, self.offset);
        self.jump_cast(obs, aimed)
    }

    fn start_cycle(&mut self, tick: u64, o: i8) {
        let o = i64::from(o);
        self.cycle = Some(Cycle {
            jump_at: tick + (-o).max(0) as u64,
            press_at: Some(tick + o.max(0) as u64),
        });
    }

    /// Donut keeps the nearest llama inside the shooting band, faces it, jumps
    /// the glob lane of a telling llama that faces her and casts when legal.
    /// (The corridor baselines; the corral uses [`Self::mobile_route`].)
    fn mobile(&mut self, obs: &Obs) -> RawInput {
        let me = obs.donut;
        let mut raw = RawInput::NONE;
        let Some(target) = nearest(me.x, &obs.llamas) else {
            return raw;
        };
        let dist = (target.x - me.x).abs();
        let dir = toward(me.x, target.x);
        if target.telling() && target.facing as f32 == -dir && dist < 260.0 && me.grounded {
            raw.jump = true;
        }
        if dist > SHOOT_BAND.1 {
            raw.move_x = dir;
        } else if dist < SHOOT_BAND.0 {
            raw.move_x = -dir;
        } else if me.facing as f32 != dir {
            // A one-tick nudge turns her toward the llama (facing follows move).
            raw.move_x = dir;
        }
        if self.released_due {
            self.released_due = false;
        } else if Self::can_cast(obs) && me.facing as f32 == dir && dist <= SHOOT_BAND.1 {
            raw.attack = true;
            self.released_due = true;
            self.presses += 1;
        }
        raw
    }

    /// A movement policy that pushes against a face (the entry shelf, the dock
    /// edge) without moving for 3 ticks hops it. `slot`: 0 Carl, 1 Donut.
    fn unstick(&mut self, slot: usize, me: HeroObs, raw: &mut RawInput) {
        let still = (me.x - self.last_x[slot]).abs() < 0.5;
        self.last_x[slot] = me.x;
        if raw.move_x != 0.0 && me.grounded && still {
            self.blocked[slot] = self.blocked[slot].saturating_add(1);
        } else {
            self.blocked[slot] = 0;
        }
        if self.blocked[slot] >= 3 {
            raw.jump = true;
            self.blocked[slot] = 0;
        }
    }

    /// The climb of the movement policies: walk to a take-off column short of
    /// the bumper, jump while running at it, land on its top, step off onto
    /// the dock. `takeoff` is the hero's take-off band.
    fn climb(&mut self, me: HeroObs, route: Route, takeoff: (f32, f32), land: f32) -> RawInput {
        let mut raw = RawInput::NONE;
        let on_bumper = me.support >= route.bumper_top - 1.0;
        if on_bumper || me.x > route.bumper_right {
            raw.move_x = 1.0;
        } else if me.grounded && me.support <= route.floor + 1.0 {
            if me.x < takeoff.0 {
                raw.move_x = 1.0;
            } else if me.x > takeoff.1 {
                raw.move_x = -1.0;
            } else {
                raw.move_x = 1.0;
                if !self.jump_held {
                    raw.jump = true;
                }
            }
        } else if !me.grounded && me.x < land {
            raw.move_x = 1.0;
        }
        self.jump_held = raw.jump;
        raw
    }

    /// The corral's mobile Donut: the llamas in route order; a llama on a
    /// higher surface is reached by the bumper climb; on her own surface she
    /// keeps a firing band, interrupts or jumps a rush tell and casts only when
    /// the bolt meets a llama.
    fn mobile_route(&mut self, obs: &Obs) -> RawInput {
        let me = obs.donut;
        let Some(route) = self.aim.route else {
            return RawInput::NONE;
        };
        let Some(target) = next_on_route(&obs.llamas).copied() else {
            return RawInput::NONE;
        };
        let can = Self::can_cast(obs) && !self.released_due;
        if self.released_due {
            self.released_due = false;
        }
        let ahead = self.aim.bolt_hits(me.x, me.facing, me.feet, &obs.llamas);
        if target.feet > me.support + 8.0 {
            let mut raw = self.climb(
                me,
                route,
                (route.bumper_left - 100.0, route.bumper_left - 60.0),
                route.bumper_right + 16.0,
            );
            if can && ahead {
                raw.attack = true;
                self.released_due = true;
                self.presses += 1;
            }
            return raw;
        }
        let mut raw = RawInput::NONE;
        let dist = (target.x - me.x).abs();
        let dir = toward(me.x, target.x);
        let faces_me = f32::from(target.facing) == -dir;
        if me.support > target.feet + 8.0 {
            // Above it (the bumper top): step down toward it.
            raw.move_x = dir;
        } else if target.rush_telling()
            && faces_me
            && dist < 150.0
            && me.grounded
            && !(can && ahead)
        {
            // Too late to interrupt: jump the rush (DM motion window 4..47).
            if (8..=28).contains(&target.attack_tick) && !self.jump_held {
                raw.jump = true;
            }
        } else if dist > SHOOT_BAND.1 {
            raw.move_x = dir;
        } else if dist < 40.0 && me.grounded && target.rush.is_none() {
            raw.move_x = -dir;
        } else if f32::from(me.facing) != dir && !ahead {
            raw.move_x = dir;
        }
        if !raw.jump {
            self.unstick(1, me, &mut raw);
        }
        self.jump_held = raw.jump;
        if can && ahead {
            raw.attack = true;
            self.released_due = true;
            self.presses += 1;
        }
        raw
    }

    /// Carl walks to the next llama, jumps its rush and its spit, climbs to
    /// the dock and jabs (one press, one release).
    fn melee(&mut self, obs: &Obs) -> RawInput {
        let me = obs.carl;
        let mut raw = RawInput::NONE;
        let pick = if self.aim.route.is_some() {
            next_on_route(&obs.llamas)
        } else {
            nearest(me.x, &obs.llamas)
        };
        let Some(target) = pick.copied() else {
            return raw;
        };
        if let Some(route) = self.aim.route
            && target.feet > me.support + 8.0
        {
            // Carl lands on the 80 px bumper top (his class limit), then steps
            // down onto the dock.
            return self.climb(
                me,
                route,
                (route.bumper_left - 98.0, route.bumper_left - 70.0),
                route.bumper_left + 14.0,
            );
        }
        let dist = (target.x - me.x).abs();
        let dir = toward(me.x, target.x);
        let faces_me = f32::from(target.facing) == -dir;
        if target.rush_telling() && faces_me && dist <= 150.0 {
            // Jump the rush on attack tick 14: inside Carl's window at every
            // start distance (spec section 6).
            if target.attack_tick == 14 && me.grounded {
                raw.jump = true;
            }
            self.jab_pressed = false;
            self.jump_held = raw.jump;
            return raw;
        }
        if target.rush == Some(RushPhase::Committed) && faces_me && dist <= 60.0 {
            if me.grounded && !self.jump_held {
                raw.jump = true;
            }
            self.jump_held = raw.jump;
            return raw;
        }
        if target.spit_telling() && faces_me && target.attack_tick >= 26 && me.grounded {
            raw.jump = true;
        }
        self.jump_held = raw.jump;
        // Close in, and turn toward the llama when already in reach.
        if dist > JAB_REACH || me.facing as f32 != dir {
            raw.move_x = dir;
        }
        if self.aim.route.is_some() && !raw.jump {
            self.unstick(0, me, &mut raw);
        }
        if self.jab_pressed {
            self.jab_pressed = false;
        } else if dist <= JAB_REACH + 30.0 && me.facing as f32 == dir {
            self.jab_pressed = true;
            raw.attack = true;
        }
        raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(mana: u8, cast_active: bool) -> Obs {
        let hero = HeroObs {
            x: 300.0,
            facing: 1,
            health: 4,
            grounded: true,
            feet: 0.0,
            support: 0.0,
        };
        Obs {
            tick: 1,
            carl: hero,
            donut: hero,
            mana: Some((mana, 8)),
            cast_active,
            donut_hurt: false,
            llamas: vec![],
        }
    }

    #[test]
    fn a_stationary_press_is_one_tick_and_releases_before_the_next() {
        let mut p = Policy::new(PolicyId::StationaryCast);
        let a = p.act(&obs(8, false));
        assert!(a.donut.attack);
        // The release tick carries no attack even though a cast could begin.
        let b = p.act(&obs(8, false));
        assert!(!b.donut.attack);
        let c = p.act(&obs(8, false));
        assert!(c.donut.attack, "the next legal cast is a new press edge");
        assert_eq!(p.presses, 2);
        assert_eq!(a.donut.move_x, 0.0);
        assert!(!a.donut.jump);
    }

    #[test]
    fn no_press_while_a_cast_runs_or_the_pool_is_dry() {
        let mut p = Policy::new(PolicyId::StationaryCast);
        assert!(!p.act(&obs(8, true)).donut.attack);
        assert!(!p.act(&obs(0, false)).donut.attack);
        assert_eq!(p.presses, 0);
    }

    #[test]
    fn the_burst_policy_spends_the_pool_then_waits_for_a_full_one() {
        let mut p = Policy::new(PolicyId::StationaryCastBurst);
        assert!(p.act(&obs(8, false)).donut.attack);
        let _ = p.act(&obs(7, false)); // release tick
        assert!(p.act(&obs(1, false)).donut.attack);
        let _ = p.act(&obs(0, false)); // release
        // Empty: refilling. It does not cast on a partial pool.
        for mana in 0..8 {
            assert!(!p.act(&obs(mana, false)).donut.attack, "mana {mana}");
        }
        assert!(p.act(&obs(8, false)).donut.attack, "full pool: next burst");
    }

    #[test]
    fn the_trivial_hold_counts_one_press() {
        let mut p = Policy::new(PolicyId::HoldAttackTrivial);
        for _ in 0..120 {
            assert!(p.act(&obs(8, false)).donut.attack);
        }
        assert_eq!(p.presses, 1);
    }

    fn aim() -> Aim {
        Aim {
            solids: Vec::new(),
            bolt_forward: 27.0,
            bolt_up: 19.0,
            bolt_step: 6.0,
            bolt_range: 240.0,
            arc: (1..=50).map(|k| (k * (50 - k)) as f32 * 0.2).collect(),
            route: None,
        }
    }

    fn llama_at(x: f32, feet: f32) -> LlamaObs {
        LlamaObs {
            x,
            feet,
            facing: -1,
            health: 3,
            mode: LlamaMode::Neutral,
            attack_tick: 0,
            rush: None,
            rect: None,
        }
    }

    #[test]
    fn the_aimed_policy_presses_only_with_a_llama_in_the_bolts_line() {
        let mut p = Policy::with_aim(PolicyId::StationaryCastAimed, 0, aim());
        let mut o = obs(8, false);
        assert!(!p.act(&o).donut.attack, "no llama: no press");
        o.llamas = vec![llama_at(700.0, 0.0)];
        assert!(!p.act(&o).donut.attack, "400 px away: out of reach");
        o.llamas = vec![llama_at(500.0, 64.0)];
        assert!(!p.act(&o).donut.attack, "a level up: the bolt passes under");
        o.llamas = vec![llama_at(500.0, 0.0)];
        assert!(p.act(&o).donut.attack, "in line and in reach");
        assert!(!p.act(&o).donut.attack, "the release tick");
    }

    /// TROG-B: a target that carries its own hurtbox (a troglodyte, `trog_eval`) is aimed at
    /// by that box; a llama (`rect: None`) keeps the llama geometry.
    #[test]
    fn an_observed_targets_own_rect_replaces_the_llama_body() {
        let a = aim();
        let mut tall = llama_at(500.0, 0.0);
        tall.rect = Some(Rect {
            left: 480.0,
            bottom: 60.0,
            width: 40.0,
            height: 10.0,
        });
        // The llama body at feet 0 is in the bolt's line; the own rect is 60 px up, above it.
        assert!(a.bolt_hits(300.0, 1, 0.0, &[llama_at(500.0, 0.0)]));
        assert!(!a.bolt_hits(300.0, 1, 0.0, &[tall]));
        // Released from 50 px up, the bolt meets that rect.
        assert!(a.bolt_hits(300.0, 1, 50.0, &[tall]));
    }

    #[test]
    fn the_turret_turns_in_place_toward_a_llama_in_line_behind() {
        let mut p = Policy::with_aim(PolicyId::StationaryTurret, 0, aim());
        let mut o = obs(8, false);
        o.llamas = vec![llama_at(150.0, 0.0)];
        let tap = p.act(&o).donut;
        assert_eq!(tap.move_x, -1.0, "one tick of stick turns her");
        assert!(!tap.attack && !tap.jump);
        o.donut.facing = -1;
        let next = p.act(&o).donut;
        assert_eq!(next.move_x, 0.0, "one tick only");
        assert!(next.attack, "then it fires");
    }

    #[test]
    fn a_jump_cast_pairs_every_press_with_a_jump_at_offset_o() {
        for o in [-3_i8, 0, 7] {
            let mut p = Policy::with_aim(PolicyId::StationaryJumpCast, o, aim());
            let mut ob = obs(8, false);
            let mut jump_tick = None;
            let mut press_tick = None;
            for t in 1..=20 {
                ob.tick = t;
                let raw = p.act(&ob).donut;
                assert_eq!(raw.move_x, 0.0);
                if raw.jump {
                    jump_tick.get_or_insert(t);
                    ob.donut.grounded = false;
                }
                if raw.attack {
                    press_tick.get_or_insert(t);
                }
            }
            let (j, a) = (jump_tick.unwrap() as i64, press_tick.unwrap() as i64);
            assert_eq!(a - j, i64::from(o), "o {o}");
            assert_eq!((p.jumps, p.presses), (1, 1), "o {o}: one pair");
        }
    }
}
