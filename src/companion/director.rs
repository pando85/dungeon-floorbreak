//! The deterministic companion director (CA-01, CA-02, CA-10, CA-24, CA-26).
//!
//! One director drives one uncontrolled hero. Each call takes the tick's
//! [`CompanionView`], the [`ProfileSet`] and its own small state, and returns
//! exactly one [`RawInput`]: the follow layer's movement plus, at most, one
//! legal press. It writes nothing else.
//!
//! **States.** [`Mode::Follow`] (the follow layer only), [`Mode::Interrupt`]
//! (answer a tell aimed at the other hero) and [`Mode::SupportAttack`] (a
//! legal hit on a standing, staggered or lying goblin).
//!
//! **Selection.** Legal utility candidates only; the best utility wins and ties
//! break by [`StableEntityId`] and then [`Action`] order, so no iteration order
//! or float comparison decides anything (utilities are integers).
//!
//! **Cadence (CA-10).** Support attacks are planned every
//! `ca10_decision_cadence_ticks` (6) and revalidated every tick; interrupts are
//! evaluated on every tick. One AI-started move per `cf02_q16_action_cadence_ticks`
//! (CF02-Q16).
//!
//! **Safety vetoes.** No press while the hero cannot start a move; no Donut
//! cast before the unlock or below the AI mana reserve (the view's
//! `ai_can_cast`); Carl never grabs or throws explosives (`ability` is never
//! set); never pause or swap; no Down (the `RawInput` of this stage has none);
//! no Jump while the hero's own move is running (CA-26, the bail-out is
//! CF02-Q55 and the AI never does it); and a press needs a released button on
//! the tick before, so it is always a clean press edge.
//!
//! **Briefs (COMPANION-AI-02, CA2-B1).** [`CompanionDirector::step_briefed`]
//! takes an optional [`GoalBrief`] from the autonomy layer. With no brief
//! (`step` and `step_traced` pass `None`) the director is exactly the one above,
//! byte for byte. With a brief it plans and keeps a support target only inside
//! `allowed` (the filter runs again every tick, so a plan made a tick before a
//! goal change dies on the next tick), presses only while the target is also
//! inside the brief's press band and she already faces it, and writes **no**
//! `move_x` and no turn for a support plan: goal steering owns all movement.
//! An interrupt keeps the director's own approach and outranks every goal, but it
//! is not exempt from the drift check (spec 3.3): the brief carries the range of
//! positions from which an interrupt's worst case stays in view
//! ([`GoalBrief::commit_x`]), and a press outside it is skipped
//! ([`Veto::DriftSkip`], counted for M16 by the autonomy layer).

use crate::combat::{CARL_HAYMAKER, CARL_JAB, ENEMY_HALF_WIDTH, MAGIC_MISSILE, MoveId};
use crate::companion::engage::leader_engages;
use crate::companion::profile::{KillPolicy, ProfileSet};
use crate::companion::view::{
    BOSS_ID, CompanionView, Control, EntityKind, HeroView, StableEntityId, TargetState, TargetView,
    ThreatView,
};
use crate::donut_moves::{POUNCE_DAMAGE, POUNCE_FLIGHT_NOMINAL, POUNCE_STARTUP, SWAT_STARTUP};
use crate::input::RawInput;
use crate::party::HeroId;

/// Top-level director state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Follow,
    Interrupt,
    SupportAttack,
}

/// The one legal move a candidate would start. The derived order is the
/// last tie-break.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    /// Donut's swat (Primary before the unlock).
    Swat,
    /// Donut's Magic Missile (Primary after the unlock).
    Missile,
    /// Donut's pounce (Physical).
    Pounce,
    /// Carl's jab (Primary).
    Jab,
    /// Carl's haymaker: the legal jab chain (a second Primary press inside
    /// the running jab's window). Part of the same AI-started move (CA-36).
    Haymaker,
}

impl Action {
    const fn uses_physical(self) -> bool {
        matches!(self, Self::Pounce)
    }
}

/// Why a press, a jump or the whole input was suppressed this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Veto {
    /// The hero is not companion-driven: the director emits nothing.
    NotCompanionDriven,
    /// The hero is defeated.
    HeroDown,
    /// The hero cannot start a move (hurt, frozen, in a move).
    CannotStartMove,
    /// CF02-Q16: an AI move started less than `cf02_q16_action_cadence_ticks` ago.
    ActionCadence,
    /// The button was held on the previous tick: a press needs a gap.
    PressEdge,
    /// CA-26: a Jump during the hero's own move is dropped.
    JumpDuringMove,
    /// COMPANION-AI-02: an interrupt press skipped because the worst case of its
    /// commitment would leave the view (spec 3.3, [`GoalBrief::commit_x`]).
    DriftSkip,
}

/// What one call decided, for traces and tests (never hashed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    pub mode: Mode,
    pub action: Option<Action>,
    pub target: Option<StableEntityId>,
    /// The press was emitted this tick.
    pub pressed: bool,
    pub veto: Option<Veto>,
    /// A follow-layer jump was dropped (CA-26 or a press tick).
    pub jump_dropped: bool,
}

impl Decision {
    const fn idle(veto: Option<Veto>) -> Self {
        Self {
            mode: Mode::Follow,
            action: None,
            target: None,
            pressed: false,
            veto,
            jump_dropped: false,
        }
    }
}

/// What the autonomy layer lets the director do this tick (spec 5.2, review
/// B-3). The brief is the gate: with it the director has no walk of its own
/// toward a support target.
#[derive(Debug, Clone, PartialEq)]
pub struct GoalBrief {
    /// The only enemies the director may plan or keep a support plan on, in
    /// id order. Empty means interrupts only (Retreat, Escort, Perch, Idle, the
    /// Hunt recoil) or nothing at all (`interrupts` false, Show-off).
    pub allowed: Vec<StableEntityId>,
    /// The press band, centre distance in px: a support press needs the target
    /// inside it (and inside the move's own reach band). Before the unlock the
    /// pounce band, after it the casting band.
    pub press_min_px: f32,
    pub press_max_px: f32,
    /// May the director answer a tell aimed at the leader? False only while a
    /// Show-off hold runs (it ends on such a tell, the next tick).
    pub interrupts: bool,
    /// The partner positions (x range) from which the worst case of an
    /// interrupt's commitment (a swat or a missile, 16 ticks) stays in view, from
    /// the autonomy layer's drift check (spec 3.3). An interrupt press outside it
    /// is skipped with [`Veto::DriftSkip`]; the approach is not. `None` (the
    /// tests' hand-made briefs) means no limit.
    pub commit_x: Option<(f32, f32)>,
}

impl GoalBrief {
    /// Interrupts only: no support target, no press but an interrupt.
    pub fn interrupts_only() -> Self {
        Self {
            allowed: Vec::new(),
            press_min_px: 0.0,
            press_max_px: 0.0,
            interrupts: true,
            commit_x: None,
        }
    }

    /// Nothing at all, not even an interrupt (the Show-off hold).
    pub fn silent() -> Self {
        Self {
            interrupts: false,
            ..Self::interrupts_only()
        }
    }

    /// Hunt or support `target`, pressing inside `[min, max]` px of it.
    pub fn targeting(target: StableEntityId, min: f32, max: f32) -> Self {
        Self {
            allowed: vec![target],
            press_min_px: min,
            press_max_px: max,
            interrupts: true,
            commit_x: None,
        }
    }

    fn allows(&self, id: StableEntityId) -> bool {
        self.allowed.contains(&id)
    }

    fn in_band(&self, distance: f32) -> bool {
        distance >= self.press_min_px && distance <= self.press_max_px
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Plan {
    action: Action,
    target: StableEntityId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Buttons {
    attack: bool,
    physical: bool,
    jump: bool,
}

/// A legal, scored option. `dx` is the signed centre distance to the target.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Candidate {
    action: Action,
    target: StableEntityId,
    utility: i32,
    dx: f32,
    reach_min: f32,
    reach_max: f32,
}

const INTERRUPT_UTILITY: i32 = 2000;
const POUNCE_UTILITY: i32 = 1500;
const STRIKE_UTILITY: i32 = 1000;
/// CF02-DUO offense: the missile beats the pounce on a standing goblin beats
/// the swat; all stay below the Q15 pounce on a goblin that lies.
const DUO_MISSILE_UTILITY: i32 = 1300;
const DUO_POUNCE_UTILITY: i32 = 1200;

impl Candidate {
    /// Higher utility first, then the lower stable id, then the lower action.
    fn beats(&self, other: &Self) -> bool {
        self.utility
            .cmp(&other.utility)
            .reverse()
            .then(self.target.cmp(&other.target))
            .then(self.action.cmp(&other.action))
            .is_lt()
    }
}

fn better(best: &mut Option<Candidate>, candidate: Option<Candidate>) {
    if let Some(c) = candidate
        && best.as_ref().is_none_or(|b| c.beats(b))
    {
        *best = Some(c);
    }
}

fn other(hero: HeroId) -> HeroId {
    match hero {
        HeroId::Carl => HeroId::Donut,
        HeroId::Donut => HeroId::Carl,
    }
}

fn sign(dx: f32) -> i8 {
    if dx < 0.0 { -1 } else { 1 }
}

/// CF02-DUO-A15: the move step toward a flank spot `dx` px away, with a dead
/// zone wider than one run step (3.1 px) so she stops on it instead of
/// shuffling around it.
fn flank_step(dx: f32) -> f32 {
    if dx.abs() <= FLANK_DEADZONE_PX {
        0.0
    } else {
        f32::from(sign(dx))
    }
}

const FLANK_DEADZONE_PX: f32 = 4.0;

/// Integer distance score: whole pixels, rounded down.
fn px(distance: f32) -> i32 {
    distance as i32
}

/// FA-Q29 (CF02-FU-B): a pounce whose predicted landing is `landing_ticks` from
/// now is launched only if it lands at least `slack` ticks before the target's
/// next impact, `impact_in` ticks from now. The plan's sample (a swing in 6 ticks:
/// landing 3 accepted) contradicts its own 4 tick slack, so the edge is exact:
/// landing at most `impact_in - slack`.
pub fn pounce_lands_in_time(landing_ticks: u16, impact_in: u16, slack: u16) -> bool {
    landing_ticks.saturating_add(slack) <= impact_in
}

/// The director of one hero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionDirector {
    hero: HeroId,
    mode: Mode,
    plan: Option<Plan>,
    last_plan_tick: Option<u64>,
    last_action_tick: Option<u64>,
    held: Buttons,
}

impl CompanionDirector {
    pub const fn new(hero: HeroId) -> Self {
        Self {
            hero,
            mode: Mode::Follow,
            plan: None,
            last_plan_tick: None,
            last_action_tick: None,
            held: Buttons {
                attack: false,
                physical: false,
                jump: false,
            },
        }
    }

    pub const fn hero(&self) -> HeroId {
        self.hero
    }

    pub const fn mode(&self) -> Mode {
        self.mode
    }

    /// Forget the plan (a swap, a restart, the hero stops being companion
    /// driven). The action cadence survives: a swap never resets it (Q16).
    pub fn clear_plan(&mut self) {
        self.mode = Mode::Follow;
        self.plan = None;
        self.last_plan_tick = None;
        self.held = Buttons::default();
    }

    /// The input of this tick.
    pub fn step(&mut self, view: &CompanionView, set: &ProfileSet) -> RawInput {
        self.step_traced(view, set).0
    }

    /// [`Self::step`] plus the decision that produced it.
    pub fn step_traced(&mut self, view: &CompanionView, set: &ProfileSet) -> (RawInput, Decision) {
        self.step_briefed(view, set, None)
    }

    /// [`Self::step_traced`] under the autonomy layer's optional brief (see the
    /// module docs). `None` is today's code path.
    pub fn step_briefed(
        &mut self,
        view: &CompanionView,
        set: &ProfileSet,
        brief: Option<&GoalBrief>,
    ) -> (RawInput, Decision) {
        debug_assert_eq!(
            view.validate(),
            Ok(()),
            "the view builder owes its guarantees"
        );
        let me = *view.hero(self.hero);
        if me.control != Control::Companion {
            self.clear_plan();
            return (
                RawInput::NONE,
                Decision::idle(Some(Veto::NotCompanionDriven)),
            );
        }
        if me.defeated {
            self.clear_plan();
            return (RawInput::NONE, Decision::idle(Some(Veto::HeroDown)));
        }
        let active = match self.hero {
            HeroId::Carl => set.carl.ca21_active,
            HeroId::Donut => set.donut.ca21_active,
        };

        // Planning (CA-10): the support plan every `ca10_decision_cadence_ticks`.
        let due = self.last_plan_tick.is_none_or(|t| {
            view.tick.saturating_sub(t) >= u64::from(set.ca10_decision_cadence_ticks)
        });
        if due {
            self.last_plan_tick = Some(view.tick);
            self.plan = if active {
                self.best_support(view, set, &me, brief).map(|c| Plan {
                    action: c.action,
                    target: c.target,
                })
            } else {
                None
            };
        }

        // Urgent checks (CA-10): interrupts every tick, the plan revalidated.
        let urgent = if active && brief.is_none_or(|b| b.interrupts) {
            self.best_interrupt(view, set, &me)
        } else {
            None
        };
        let (mode, goal) = match urgent {
            Some(c) => (Mode::Interrupt, Some(c)),
            None => {
                let planned = self
                    .plan
                    .and_then(|p| self.revalidate(view, set, &me, p, brief));
                if planned.is_none() {
                    self.plan = None;
                }
                (
                    if planned.is_some() {
                        Mode::SupportAttack
                    } else {
                        Mode::Follow
                    },
                    planned,
                )
            }
        };
        let (mode, goal) = self.with_chain(view, set, &me, active, mode, goal);
        self.mode = mode;

        let follow = view.follow[self.hero.index()];
        let mut raw = RawInput {
            move_x: follow.move_x,
            ..RawInput::NONE
        };
        let mut jump = follow.jump;
        // CF02-DUO-A15: with nothing to hit and no jump in progress (a dodge,
        // a hop, the nav), she idles on the flank spot instead of trailing.
        // Under a brief the goal layer owns all movement (A15 is superseded).
        if goal.is_none()
            && !jump
            && brief.is_none()
            && let Some(x) = self.flank_spot(view, set, &me)
        {
            raw.move_x = flank_step(x - me.x);
        }
        let mut decision = Decision {
            mode,
            action: goal.map(|c| c.action),
            target: goal.map(|c| c.target),
            pressed: false,
            veto: None,
            jump_dropped: false,
        };
        let mut next = Buttons::default();

        if let Some(c) = goal {
            let distance = c.dx.abs();
            // Under a brief a support plan (not an interrupt) has no walk of its
            // own: goal steering brings her into the band and turns her, and the
            // director presses only inside both bands while she faces the target.
            let held_back = brief.filter(|_| mode == Mode::SupportAttack);
            let in_reach = distance >= c.reach_min
                && distance <= c.reach_max
                && held_back.is_none_or(|b| b.in_band(distance));
            let facing_ok = sign(c.dx) == me.facing;
            if !in_reach {
                // Close in (or back off a target that is too close).
                if held_back.is_none() {
                    raw.move_x = if distance > c.reach_max {
                        sign(c.dx) as f32
                    } else {
                        -(sign(c.dx) as f32)
                    };
                }
            } else if !facing_ok {
                if held_back.is_none() {
                    raw.move_x = sign(c.dx) as f32; // turn to it, press next tick
                }
            } else {
                let chain = c.action == Action::Haymaker;
                decision.veto = if chain {
                    self.chain_veto(&me)
                } else {
                    self.press_veto(view, set, &me, c.action)
                };
                // An interrupt is not exempt from the drift check (spec 3.3): from
                // where the worst case leaves the view, the press is skipped.
                if decision.veto.is_none()
                    && mode == Mode::Interrupt
                    && brief
                        .and_then(|b| b.commit_x)
                        .is_some_and(|(lo, hi)| me.x < lo || me.x > hi)
                {
                    decision.veto = Some(Veto::DriftSkip);
                }
                if decision.veto.is_none() {
                    if c.action.uses_physical() {
                        next.physical = true;
                    } else {
                        next.attack = true;
                    }
                    decision.pressed = true;
                    if !chain {
                        // A chain follow-up is part of the running AI-started
                        // move: it does not restart the Q16 cadence (CA-36).
                        self.last_action_tick = Some(view.tick);
                    }
                }
            }
        }

        // CA-26: no Jump during the hero's own move; a press tick never jumps;
        // a held jump is released first so the next one is an edge.
        if jump {
            let in_move = me.active_move.is_some() && !set.ca26_ai_bailout;
            if in_move || decision.pressed {
                jump = false;
                decision.jump_dropped = true;
                if in_move {
                    decision.veto.get_or_insert(Veto::JumpDuringMove);
                }
            } else if self.held.jump {
                jump = false;
            }
        }
        next.jump = jump;
        self.held = next;
        raw.attack = next.attack;
        raw.physical = next.physical;
        raw.jump = next.jump;
        // `ability`, `pause` and `swap` stay false by construction.
        (raw, decision)
    }

    /// The hero may press `action` now: move legality, the cadence and the
    /// press edge. (The per-action rules, unlock and reserve, are part of the
    /// candidate itself.)
    fn press_veto(
        &self,
        view: &CompanionView,
        set: &ProfileSet,
        me: &HeroView,
        action: Action,
    ) -> Option<Veto> {
        if !me.can_start_move || me.active_move.is_some() || me.hurt_ticks > 0 {
            return Some(Veto::CannotStartMove);
        }
        if let Some(last) = self.last_action_tick
            && view.tick.saturating_sub(last) < u64::from(set.cf02_q16_action_cadence_ticks)
        {
            return Some(Veto::ActionCadence);
        }
        let held = if action.uses_physical() {
            self.held.physical
        } else {
            self.held.attack
        };
        held.then_some(Veto::PressEdge)
    }

    /// CA-36: inside the running AI-started jab's chain window the goal on the
    /// same target becomes the haymaker, if that follow-up is itself legal
    /// (under `never` it must not be lethal: the view's HP is read after the
    /// jab landed, the window starts when the jab's active ticks are over).
    fn with_chain(
        &self,
        view: &CompanionView,
        set: &ProfileSet,
        me: &HeroView,
        active: bool,
        mode: Mode,
        goal: Option<Candidate>,
    ) -> (Mode, Option<Candidate>) {
        let unchanged = (mode, goal);
        let (true, HeroId::Carl, Some(goal)) = (active, self.hero, goal) else {
            return unchanged;
        };
        let (Some(mv), Some(chain)) = (me.active_move, CARL_JAB.chain) else {
            return unchanged;
        };
        if goal.action != Action::Jab || mv.id != MoveId::Jab || !mv.ai_started {
            return unchanged;
        }
        // Sim ordering this depends on: the view is the pre-tick state and
        // `MoveRunner::step` runs `running.tick += 1` before it reads the
        // press (and ends the move once `running.tick >= total`). A press read
        // on this tick is therefore judged at `elapsed + 1`, where `elapsed`
        // is the pre-tick move clock. At `elapsed + 1 == total` the jab is
        // over and the press would start a fresh jab, bypassing the veto and
        // the Q16 cadence, so that tick is outside the window.
        let elapsed = CARL_JAB.total().saturating_sub(mv.ticks_left);
        let judged = elapsed + 1;
        let from = chain.press_from.max(CARL_JAB.startup + CARL_JAB.active);
        if judged < from || judged > chain.press_to || judged >= CARL_JAB.total() {
            return unchanged;
        }
        let Some(target) = view.target(goal.target) else {
            return unchanged;
        };
        match self.support_candidate(view, set, me, target, Action::Haymaker) {
            Some(c) => (
                if mode == Mode::Follow {
                    Mode::SupportAttack
                } else {
                    mode
                },
                Some(c),
            ),
            None => unchanged,
        }
    }

    /// The chain follow-up needs no free hero (it is inside the running move)
    /// and no cadence slot, only a hero that is not hurt and a clean edge.
    fn chain_veto(&self, me: &HeroView) -> Option<Veto> {
        if me.hurt_ticks > 0 {
            return Some(Veto::CannotStartMove);
        }
        self.held.attack.then_some(Veto::PressEdge)
    }

    /// The best interrupt of a tell aimed at the other hero.
    fn best_interrupt(
        &self,
        view: &CompanionView,
        set: &ProfileSet,
        me: &HeroView,
    ) -> Option<Candidate> {
        if !view.view.in_view(me.x) {
            return None; // Q40: never acts outside the scroll-camera view
        }
        let owner = other(self.hero);
        let mut best = None;
        for threat in &view.threats {
            // The Q10 light guard glances further swats only; Carl's hits and
            // the pounce still interrupt it (goblin.rs, the light guard).
            let glanced = threat.guarded && self.hero == HeroId::Donut;
            if threat.aimed_at != Some(owner) || !threat.in_view || glanced {
                continue;
            }
            better(&mut best, self.interrupt_candidate(set, me, threat));
        }
        best
    }

    fn interrupt_candidate(
        &self,
        set: &ProfileSet,
        me: &HeroView,
        threat: &ThreatView,
    ) -> Option<Candidate> {
        let dx = threat.x - me.x;
        let distance = dx.abs();
        let slot = |action, reach_min: u16, reach_max: u16| Candidate {
            action,
            target: threat.id,
            utility: INTERRUPT_UTILITY - px(distance),
            dx,
            reach_min: f32::from(reach_min),
            reach_max: f32::from(reach_max),
        };
        match self.hero {
            HeroId::Donut => {
                let d = &set.donut;
                let mana = me.mana?;
                if !mana.unlocked {
                    // Primary is the swat. Q14 + Q39: from the minimum tell
                    // tick, within range, and the swat must hit inside the tell.
                    //
                    // Sim ordering this depends on: the view is the pre-tick
                    // state, then the heroes step, then the goblins. A press
                    // read on this tick hits on tick T + SWAT_STARTUP, when the
                    // goblin's attack tick is already `tell_tick +
                    // SWAT_STARTUP` (goblins step after Donut). The tell is
                    // attack ticks 0..tell_len; tick `tell_len` is the
                    // committed smear that cannot be interrupted, so the hit
                    // must land strictly before it.
                    let lands = threat.tell_tick + SWAT_STARTUP < threat.tell_len;
                    (threat.tell_tick >= d.cf02_q14_swat_tell_tick
                        && distance <= f32::from(d.cf02_q14_swat_range_px)
                        && lands)
                        .then(|| slot(Action::Swat, d.ca41_swat_hit_min_px, d.ca41_swat_hit_max_px))
                } else {
                    // Q17: never before the unlock (above), only above the AI
                    // reserve, only in the missile band, and only when the bolt
                    // arrives inside the tell.
                    //
                    // Same sim ordering as the swat: the bolt spawns on cast
                    // tick `MAGIC_MISSILE.startup` (T + 4), when the goblin's
                    // attack tick is already `tell_tick + 4`. Its hit box front
                    // is `bolt_lead_px` ahead of Donut on the spawn tick, and
                    // the box is tested on that tick, so the flight is the
                    // number of `speed` px steps still needed to reach the
                    // goblin's body edge, plus the spawn tick itself:
                    // `floor((d - 12 - lead) / speed) + 1` (0 when it already
                    // overlaps). The arrival attack tick must be < `tell_len`.
                    let speed = f32::from(mana.projectile_speed_px_per_tick.max(1));
                    let gap = distance - ENEMY_HALF_WIDTH - f32::from(mana.bolt_lead_px);
                    let flight = if gap < 0.0 {
                        0
                    } else {
                        (gap / speed).floor() as u32 + 1
                    };
                    let arrival =
                        u32::from(threat.tell_tick) + u32::from(MAGIC_MISSILE.startup) + flight;
                    (mana.ai_can_cast
                        && arrival < u32::from(threat.tell_len)
                        && distance >= f32::from(d.cf02_q17_missile_min_px)
                        && distance <= f32::from(d.cf02_q17_missile_max_px))
                    .then(|| {
                        // Already in the band: fire (no approach).
                        slot(
                            Action::Missile,
                            d.cf02_q17_missile_min_px,
                            d.cf02_q17_missile_max_px,
                        )
                    })
                }
            }
            HeroId::Carl => {
                let c = &set.carl;
                // Same sim ordering as the swat (pre-tick view, heroes, then
                // goblins): the jab hits on T + startup, when the goblin's
                // attack tick is `tell_tick + startup`, which must be < the
                // committed smear tick `tell_len`.
                let lands = threat.tell_tick + CARL_JAB.startup < threat.tell_len;
                (distance <= f32::from(c.ca40_interrupt_range_px) && lands)
                    .then(|| slot(Action::Jab, c.ca41_strike_min_px, c.ca41_strike_max_px))
            }
        }
    }

    /// The best non-urgent support (pounce, strike), over the ordered targets.
    fn best_support(
        &self,
        view: &CompanionView,
        set: &ProfileSet,
        me: &HeroView,
        brief: Option<&GoalBrief>,
    ) -> Option<Candidate> {
        if !view.view.in_view(me.x) {
            return None;
        }
        let mut best = None;
        for target in &view.targets {
            if brief.is_some_and(|b| !b.allows(target.id)) {
                continue;
            }
            for action in [Action::Pounce, Action::Jab, Action::Swat, Action::Missile] {
                better(
                    &mut best,
                    self.support_candidate(view, set, me, target, action),
                );
            }
        }
        // FA-Q04: the boss, only when the brief names it (BossPunish): never in
        // `targets`, so a set without the brief never plans on it.
        if brief.is_some_and(|b| b.allows(BOSS_ID))
            && let Some(boss) = view.find_target(BOSS_ID)
        {
            for action in [Action::Pounce, Action::Missile] {
                better(&mut best, self.boss_candidate(set, me, &boss, action));
            }
        }
        best
    }

    fn revalidate(
        &self,
        view: &CompanionView,
        set: &ProfileSet,
        me: &HeroView,
        plan: Plan,
        brief: Option<&GoalBrief>,
    ) -> Option<Candidate> {
        if !view.view.in_view(me.x) {
            return None;
        }
        // The brief's filter runs every tick: a plan whose target left
        // `allowed` dies now, not at the next planning tick.
        if brief.is_some_and(|b| !b.allows(plan.target)) {
            return None;
        }
        let target = view.find_target(plan.target)?; // gone or a new spawn serial
        if target.id == BOSS_ID {
            return self.boss_candidate(set, me, &target, plan.action);
        }
        self.support_candidate(view, set, me, &target, plan.action)
    }

    /// FA-Q04: a cast or a pounce on the boss, the AI Donut's BossPunish. The
    /// same band and facing rules as a goblin's (CF02-DUO-A05, A06, A09, A13); the
    /// pounce needs no lying target (the boss takes free damage while it is
    /// vulnerable, and the goal layer gates the window). A lethal hit is a glance
    /// at the strike site (FA-Q06), so no kill veto is needed under `factual`.
    fn boss_candidate(
        &self,
        set: &ProfileSet,
        me: &HeroView,
        target: &TargetView,
        action: Action,
    ) -> Option<Candidate> {
        if self.hero != HeroId::Donut || !target.in_view || target.hp == 0 {
            return None;
        }
        let duo = &set.cf02_duo;
        let dx = target.x - me.x;
        let distance = dx.abs();
        if (target.foot_y - me.foot_y).abs() > f32::from(duo.cf02_duo_a09_vertical_reach_px)
            || !(duo.cf02_duo_a13_turn_to_target || sign(dx) == me.facing)
        {
            return None;
        }
        let never = set.ca19_kill_policy == KillPolicy::Never;
        match action {
            Action::Missile => {
                let mana = me.mana?;
                (mana.unlocked
                    && mana.ai_can_cast
                    && !(never && target.hp <= MAGIC_MISSILE.damage)
                    && distance >= f32::from(duo.cf02_duo_a05_missile_min_px)
                    && distance <= f32::from(duo.cf02_duo_a05_missile_max_px))
                .then_some(Candidate {
                    action,
                    target: target.id,
                    utility: DUO_MISSILE_UTILITY - px(distance),
                    dx,
                    reach_min: f32::from(duo.cf02_duo_a05_missile_min_px),
                    reach_max: f32::from(duo.cf02_duo_a05_missile_max_px),
                })
            }
            Action::Pounce => (!(never && target.hp <= POUNCE_DAMAGE)
                && distance >= f32::from(duo.cf02_duo_a06_pounce_min_px)
                && distance <= f32::from(duo.cf02_duo_a06_pounce_max_px))
            .then_some(Candidate {
                action,
                target: target.id,
                utility: DUO_POUNCE_UTILITY - px(distance),
                dx,
                reach_min: f32::from(duo.cf02_duo_a06_pounce_min_px),
                reach_max: f32::from(duo.cf02_duo_a06_pounce_max_px),
            }),
            _ => None,
        }
    }

    fn support_candidate(
        &self,
        view: &CompanionView,
        set: &ProfileSet,
        me: &HeroView,
        target: &TargetView,
        action: Action,
    ) -> Option<Candidate> {
        if !target.in_view || target.guarded || !view.view.in_view(target.x) {
            return None;
        }
        let dx = target.x - me.x;
        let distance = dx.abs();
        // A hit the AI must not start: lethal under `never`, or one that would
        // take the last hit on an enemy the human Carl is engaging (A08).
        let kills = |damage: u8| {
            (set.ca19_kill_policy == KillPolicy::Never && target.hp <= damage)
                || self.steals_the_finish(view, set, target, damage)
        };
        match (self.hero, action) {
            (HeroId::Donut, Action::Pounce) => {
                // FA-Q29: no pounce that would land too close to (or after) the
                // target's next swing: hold, and ask again next tick.
                if let Some(t) = view.threats.iter().find(|t| t.id == target.id)
                    && set.fix_all.fa_q29_pounce_timing
                    && !pounce_lands_in_time(
                        POUNCE_STARTUP + POUNCE_FLIGHT_NOMINAL,
                        t.impact_tick.saturating_sub(t.tell_tick),
                        set.fix_all.fa_q29_slack_ticks,
                    )
                {
                    return None;
                }
                let d = &set.donut;
                // A lying goblin must still lie when the pounce lands (Q38).
                let down = match target.state {
                    TargetState::Lying {
                        down_ticks_left, ..
                    } => down_ticks_left >= d.cf02_q38_pounce_lying_min_ticks_left,
                    TargetState::Staggered { .. } => true,
                    _ => false,
                };
                // Q15: ahead of her, 30..80 px, and a pounce that cannot kill
                // (under `never`; CF02-DUO-A01 lets it finish).
                (target.id.kind == EntityKind::Goblin // Q15: only on a goblin
                    && down
                    && target.hp > 0
                    && !kills(POUNCE_DAMAGE)
                    && sign(dx) == me.facing
                    && distance >= f32::from(d.cf02_q15_pounce_min_px)
                    && distance <= f32::from(d.cf02_q15_pounce_max_px))
                .then_some(Candidate {
                    action,
                    target: target.id,
                    utility: POUNCE_UTILITY - px(distance),
                    dx,
                    reach_min: f32::from(d.cf02_q15_pounce_min_px),
                    reach_max: f32::from(d.cf02_q15_pounce_max_px),
                })
                .or_else(|| self.duo_pounce_standing(view, set, me, target, &kills))
            }
            (HeroId::Donut, Action::Missile) if set.cf02_duo.cf02_duo_a04_offense => {
                self.duo_missile(set, me, target, &kills)
            }
            (HeroId::Carl, Action::Jab | Action::Haymaker) => {
                let c = &set.carl;
                let hittable = matches!(
                    target.state,
                    TargetState::Standing | TargetState::Staggered { .. }
                );
                (hittable
                    && target.hp > 0
                    && !kills(if action == Action::Jab {
                        CARL_JAB.damage
                    } else {
                        CARL_HAYMAKER.damage
                    })
                    && distance <= f32::from(c.ca40_engage_range_px))
                .then_some(Candidate {
                    action,
                    target: target.id,
                    utility: STRIKE_UTILITY - px(distance),
                    dx,
                    reach_min: f32::from(c.ca41_strike_min_px),
                    reach_max: f32::from(c.ca41_strike_max_px),
                })
            }
            _ => None,
        }
    }

    /// CF02-DUO-A04/A06: a pounce on a goblin that stands (not in a tell), the
    /// AI Donut's own attack on an enemy nobody has put down yet.
    fn duo_pounce_standing(
        &self,
        view: &CompanionView,
        set: &ProfileSet,
        me: &HeroView,
        target: &TargetView,
        kills: &dyn Fn(u8) -> bool,
    ) -> Option<Candidate> {
        let duo = &set.cf02_duo;
        let dx = target.x - me.x;
        let distance = dx.abs();
        let telling = view.threats.iter().any(|t| t.id == target.id);
        (duo.cf02_duo_a04_offense
            && target.id.kind == EntityKind::Goblin
            && target.state == TargetState::Standing
            && !telling
            && target.hp > 0
            && !kills(POUNCE_DAMAGE)
            && (duo.cf02_duo_a13_turn_to_target || sign(dx) == me.facing)
            && (target.foot_y - me.foot_y).abs() <= f32::from(duo.cf02_duo_a09_vertical_reach_px)
            && distance >= f32::from(duo.cf02_duo_a06_pounce_min_px)
            && distance <= f32::from(duo.cf02_duo_a06_pounce_max_px))
        .then_some(Candidate {
            action: Action::Pounce,
            target: target.id,
            utility: DUO_POUNCE_UTILITY - px(distance),
            dx,
            reach_min: f32::from(duo.cf02_duo_a06_pounce_min_px),
            reach_max: f32::from(duo.cf02_duo_a06_pounce_max_px),
        })
    }

    /// CF02-DUO-A15: where AI Donut should idle (an x) instead of trailing: on
    /// the side the human Carl faces, `cf02_duo_a15_flank_ahead_px` ahead of
    /// him, while a living standing goblin on her level approaches on that side
    /// (further than the spot plus her pounce minimum, within
    /// `cf02_duo_a15_flank_goblin_range_px` of Carl, in view). The spot is
    /// clamped to the flat ground the view reports (no ledge, wall or hole) and
    /// to the simulation camera less `cf02_duo_a15_flank_view_margin_px`; with
    /// no room for it there is no flank. Once a goblin is inside the spot plus
    /// her pounce minimum she falls back to `cf02_duo_a15_fallback_behind_px`
    /// behind Carl. No flank during a boss tell, while
    /// she cannot start a move (hurt, frozen, in a move) or off the ground.
    /// Positioning only: every attack still goes through the A08, A11 and A12
    /// anti-steal rules and the interrupt and dodge priority above it.
    fn flank_spot(&self, view: &CompanionView, set: &ProfileSet, me: &HeroView) -> Option<f32> {
        let duo = &set.cf02_duo;
        let carl = &view.heroes[HeroId::Carl.index()];
        if self.hero != HeroId::Donut
            || !duo.cf02_duo_a15_flank
            || !matches!(carl.control, Control::Human(_))
            || carl.defeated
            || view.boss.is_some()
            || !me.grounded
            || !me.can_start_move
        {
            return None;
        }
        let (span_lo, span_hi) = view.flank_span?;
        let ahead = f32::from(duo.cf02_duo_a15_flank_ahead_px);
        let near = ahead + f32::from(duo.cf02_duo_a06_pounce_min_px);
        let range = f32::from(duo.cf02_duo_a15_flank_goblin_range_px);
        let vertical = f32::from(duo.cf02_duo_a09_vertical_reach_px);
        // Standing goblins on her level, in view, on the side Carl faces: `far`
        // ones are still approaching (flank), `near` ones are inside the spot
        // plus her pounce minimum (fall back).
        let mut far = false;
        let mut arrived = false;
        for t in &view.targets {
            let dx = t.x - carl.x;
            if t.id.kind == EntityKind::Goblin
                && t.hp > 0
                && t.state == TargetState::Standing
                && t.in_view
                && sign(dx) == carl.facing
                && (t.foot_y - me.foot_y).abs() <= vertical
            {
                if dx.abs() < near {
                    arrived = true;
                } else if dx.abs() <= range {
                    far = true;
                }
            }
        }
        let offset = if arrived {
            // It would walk through her and leave her too close to pounce
            // it again: stand where it arrives inside her band, behind Carl.
            -f32::from(duo.cf02_duo_a15_fallback_behind_px)
        } else if far {
            ahead
        } else {
            return None;
        };
        let margin = f32::from(duo.cf02_duo_a15_flank_view_margin_px);
        let lo = span_lo.max(view.view.left + margin);
        let hi = span_hi.min(view.view.right - margin);
        if lo > hi {
            return None;
        }
        Some((carl.x + f32::from(carl.facing) * offset).clamp(lo, hi))
    }

    /// CF02-DUO-A08 (spec section 8, `HumanFinishWindow`): a damaging AI hit
    /// of `damage` on `target` would take the finish of an enemy the
    /// human-controlled Carl is engaging. Carl engages (A11: only on the side
    /// he faces) a target within `cf02_duo_a08_finisher_reach_px` of him while
    /// he has a move running (the jab to haymaker window, whatever its phase)
    /// or while the target is still down or staggered from his last hit
    /// (`carl_hit`), and (A12) a target he hit within
    /// `cf02_duo_a12_recent_hit_ticks`, at any distance and on either side of
    /// him (the walk-up between two jabs, a step back, a walk-through; checked
    /// before A11, PR #170 review B-1); a Carl who just stands near a goblin
    /// engages nothing, so she still protects an idle player. The AI hit is refused while the target's
    /// HP is at most `damage + cf02_duo_a08_finisher_reserve_hp`. Known limit
    /// (A14): a press on the tick she starts is invisible to this pre-tick
    /// view. Only the AI Donut under `factual`; it never changes Carl's
    /// director or a set with the rule off.
    fn steals_the_finish(
        &self,
        view: &CompanionView,
        set: &ProfileSet,
        target: &TargetView,
        damage: u8,
    ) -> bool {
        let duo = &set.cf02_duo;
        // Under `never` no AI hit kills (the `kills` guard), so the window is
        // only needed under `factual` (spec section 8).
        if self.hero != HeroId::Donut
            || !duo.cf02_duo_a08_respect_finisher
            || set.ca19_kill_policy != KillPolicy::Factual
        {
            return false;
        }
        let carl = &view.heroes[HeroId::Carl.index()];
        if !matches!(carl.control, Control::Human(_)) || carl.defeated {
            return false;
        }
        if target.hp > damage.saturating_add(duo.cf02_duo_a08_finisher_reserve_hp) {
            return false;
        }
        // A12, A11 and A08: the engagement predicate shared with the autonomy
        // layer's claim set (COMPANION-AI-02, `engage::leader_engages`).
        leader_engages(set, carl, target)
    }

    /// CF02-DUO-A04/A05: a Magic Missile on a standing or staggered enemy in
    /// the offensive band. Only after the unlock and only with the AI reserve
    /// left after the cast (`ai_can_cast`): never a free cast.
    fn duo_missile(
        &self,
        set: &ProfileSet,
        me: &HeroView,
        target: &TargetView,
        kills: &dyn Fn(u8) -> bool,
    ) -> Option<Candidate> {
        let duo = &set.cf02_duo;
        let mana = me.mana?;
        let dx = target.x - me.x;
        let distance = dx.abs();
        let hittable = matches!(
            target.state,
            TargetState::Standing | TargetState::Staggered { .. }
        );
        (mana.unlocked
            && mana.ai_can_cast
            && hittable
            && target.hp > 0
            && !kills(MAGIC_MISSILE.damage)
            && (duo.cf02_duo_a13_turn_to_target || sign(dx) == me.facing)
            && (target.foot_y - me.foot_y).abs() <= f32::from(duo.cf02_duo_a09_vertical_reach_px)
            && distance >= f32::from(duo.cf02_duo_a05_missile_min_px)
            && distance <= f32::from(duo.cf02_duo_a05_missile_max_px))
        .then_some(Candidate {
            action: Action::Missile,
            target: target.id,
            utility: DUO_MISSILE_UTILITY - px(distance),
            dx,
            reach_min: f32::from(duo.cf02_duo_a05_missile_min_px),
            reach_max: f32::from(duo.cf02_duo_a05_missile_max_px),
        })
    }

    /// Mix the director's own state (to be folded into the scene hash by
    /// 01A-wire while a hero is companion-driven, CA-25). The tick-local
    /// `Decision` is a trace and is never mixed.
    pub fn hash_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.hero.index() as u64);
        mix(self.mode as u64);
        match self.plan {
            None => mix(0),
            Some(p) => {
                mix(1);
                mix(p.action as u64);
                p.target.hash_into(mix);
            }
        }
        for tick in [self.last_plan_tick, self.last_action_tick] {
            match tick {
                None => mix(0),
                Some(t) => {
                    mix(1);
                    mix(t);
                }
            }
        }
        mix(u64::from(self.held.attack)
            | (u64::from(self.held.physical) << 1)
            | (u64::from(self.held.jump) << 2));
    }

    /// FNV-1a 64 over [`Self::hash_into`] (the fold the scene hash uses).
    pub fn state_hash(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        self.hash_into(&mut |value| {
            for byte in value.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        });
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::MoveId;
    use crate::companion::profile::{ProfileFile, ProfileSetId};
    use crate::companion::view::fixtures::{hero, view};
    use crate::companion::view::{ActiveMove, FollowInput, ManaView, MovePhase};

    fn set_of(id: ProfileSetId) -> ProfileSet {
        ProfileFile::pinned().unwrap().set(id).clone()
    }

    fn assist() -> ProfileSet {
        set_of(ProfileSetId::Cf02Assist)
    }

    /// `cf02_assist` as it was before CF02-DUO (never kills, one action per 60
    /// ticks, no offense): isolates the Q14..Q17 rules these tests pin.
    fn restrained() -> ProfileSet {
        let mut set = assist();
        set.ca19_kill_policy = KillPolicy::Never;
        set.cf02_q16_action_cadence_ticks = 60;
        set.cf02_duo.cf02_duo_a04_offense = false;
        set
    }

    fn v0() -> ProfileSet {
        set_of(ProfileSetId::CompanionV0)
    }

    fn threat(index: u32, x: f32, tell_tick: u16, aimed_at: Option<HeroId>) -> ThreatView {
        ThreatView {
            id: StableEntityId::goblin(index),
            x,
            foot_y: 0.0,
            facing: -1,
            tell_tick,
            tell_len: 24,
            impact_tick: 26,
            aimed_at,
            guarded: false,
            in_view: true,
            swing_x: None,
        }
    }

    fn target(index: u32, x: f32, hp: u8, state: TargetState) -> TargetView {
        TargetView {
            id: StableEntityId::goblin(index),
            x,
            foot_y: 0.0,
            hp,
            state,
            guarded: false,
            in_view: true,
            carl_hit: false,
            carl_hit_age: None,
        }
    }

    /// Human Carl at x=100, AI Donut at `donut_x` facing right, pre-biscuit.
    fn donut_ai(donut_x: f32) -> CompanionView {
        view(
            hero(HeroId::Carl, 100.0, Control::Human(0)),
            hero(HeroId::Donut, donut_x, Control::Companion),
        )
    }

    /// Human Donut at x=100, AI Carl at `carl_x` facing right.
    fn carl_ai(carl_x: f32) -> CompanionView {
        view(
            hero(HeroId::Carl, carl_x, Control::Companion),
            hero(HeroId::Donut, 100.0, Control::Human(0)),
        )
    }

    fn unlock(v: &mut CompanionView, mana: u8, ai_can_cast: bool) {
        v.heroes[1].mana = Some(ManaView {
            mana,
            max: 8,
            unlocked: true,
            ai_can_cast,
            projectile_speed_px_per_tick: 6,
            bolt_lead_px: 35,
        });
    }

    fn donut() -> CompanionDirector {
        CompanionDirector::new(HeroId::Donut)
    }

    fn carl() -> CompanionDirector {
        CompanionDirector::new(HeroId::Carl)
    }

    fn only_follow(raw: &RawInput) -> bool {
        !raw.attack && !raw.physical && !raw.ability && !raw.pause && !raw.swap
    }

    // ------------------------------------------------------------ acceptance --

    #[test]
    fn an_ai_donut_swats_a_tell_aimed_at_the_human_carl() {
        let mut v = donut_ai(60.0);
        // Goblin 30 px ahead of her, tell tick 4, aimed at Carl.
        v.threats = vec![threat(0, 90.0, 4, Some(HeroId::Carl))];
        v.targets = vec![target(0, 90.0, 2, TargetState::Standing)];
        let (raw, d) = donut().step_traced(&v, &assist());
        assert!(
            raw.attack && !raw.physical,
            "Primary is the swat before the unlock"
        );
        assert_eq!(d.mode, Mode::Interrupt);
        assert_eq!(d.action, Some(Action::Swat));
        assert_eq!(d.target, Some(StableEntityId::goblin(0)));
        assert!(d.pressed && d.veto.is_none());
        assert!(!raw.ability && !raw.pause && !raw.swap && !raw.jump);
    }

    #[test]
    fn an_ai_carl_attacks_a_standing_goblin() {
        let mut v = carl_ai(60.0);
        v.threats = Vec::new();
        v.targets = vec![target(0, 85.0, 2, TargetState::Standing)];
        let (raw, d) = carl().step_traced(&v, &v0());
        assert!(raw.attack && !raw.physical && !raw.ability);
        assert_eq!(d.mode, Mode::SupportAttack);
        assert_eq!(d.action, Some(Action::Jab));
        assert_eq!(d.target, Some(StableEntityId::goblin(0)));
    }

    #[test]
    fn under_cf02_assist_ai_carl_is_passive() {
        let mut v = carl_ai(60.0);
        v.targets = vec![target(0, 85.0, 2, TargetState::Standing)];
        v.threats = vec![threat(0, 85.0, 6, Some(HeroId::Donut))];
        v.follow[0] = FollowInput {
            move_x: 1.0,
            jump: false,
        };
        let (raw, d) = carl().step_traced(&v, &assist());
        assert_eq!(d.mode, Mode::Follow);
        assert!(only_follow(&raw) && !raw.attack);
        assert_eq!(
            raw.move_x, 1.0,
            "the follow layer's movement passes through"
        );
    }

    // ------------------------------------------------------- donut interrupts --

    #[test]
    fn a_far_tell_is_approached_not_swatted() {
        let mut v = donut_ai(0.0);
        v.threats = vec![threat(0, 90.0, 4, Some(HeroId::Carl))];
        let (raw, d) = donut().step_traced(&v, &assist());
        assert_eq!(d.mode, Mode::Interrupt);
        assert!(!d.pressed && !raw.attack);
        assert_eq!(raw.move_x, 1.0, "she runs toward it");
        // Too close backs off instead of pressing a swat that cannot connect.
        let mut v = donut_ai(89.0);
        v.threats = vec![threat(0, 90.0, 4, Some(HeroId::Carl))];
        let (raw, d) = donut().step_traced(&v, &assist());
        assert!(!d.pressed && raw.move_x == -1.0);
    }

    #[test]
    fn q39_the_swat_starts_from_tell_tick_four() {
        for (tick, expect) in [(0, false), (3, false), (4, true), (19, true), (20, false)] {
            let mut v = donut_ai(60.0);
            v.threats = vec![threat(0, 90.0, tick, Some(HeroId::Carl))];
            let (_, d) = donut().step_traced(&v, &assist());
            assert_eq!(d.pressed, expect, "tell tick {tick}");
        }
    }

    #[test]
    fn q14_the_swat_must_hit_before_the_committed_smear() {
        // Pre-tick view, then heroes, then goblins: a swat pressed at view
        // tell tick `a` hits on tick T + 4, when the goblin's attack tick is
        // `a + 4`. The tell is attack ticks 0..23; tick 24 is the committed
        // smear. a = 20 would hit on 24 (nothing is interrupted), a = 19 on 23.
        let mut v = donut_ai(60.0);
        v.threats = vec![threat(0, 90.0, 20, Some(HeroId::Carl))];
        let (raw, d) = donut().step_traced(&v, &assist());
        assert_eq!(d.mode, Mode::Follow);
        assert!(!raw.attack && !d.pressed, "a = 20 lands on the smear");
        v.threats = vec![threat(0, 90.0, 19, Some(HeroId::Carl))];
        assert!(donut().step_traced(&v, &assist()).1.pressed, "a = 19");
    }

    #[test]
    fn q14_the_carl_jab_must_hit_before_the_committed_smear() {
        // Same ordering: the jab (startup 3) hits on a + 3; a = 21 is the
        // smear (attack tick 24), a = 20 is the last tell tick (23).
        let mut v = carl_ai(60.0);
        v.threats = vec![threat(0, 85.0, 21, Some(HeroId::Donut))];
        let (raw, d) = carl().step_traced(&v, &v0());
        assert!(
            !raw.attack && !d.pressed && d.mode == Mode::Follow,
            "a = 21"
        );
        v.threats = vec![threat(0, 85.0, 20, Some(HeroId::Donut))];
        assert!(carl().step_traced(&v, &v0()).1.pressed, "a = 20");
    }

    #[test]
    fn only_a_tell_aimed_at_the_other_hero_is_interrupted() {
        for aimed in [None, Some(HeroId::Donut)] {
            let mut v = donut_ai(60.0);
            v.threats = vec![threat(0, 90.0, 6, aimed)];
            let (raw, d) = donut().step_traced(&v, &assist());
            assert_eq!(d.mode, Mode::Follow, "{aimed:?}");
            assert!(!raw.attack);
        }
    }

    #[test]
    fn q14_range_q40_view_and_the_light_guard_gate_the_swat() {
        let go = |f: &dyn Fn(&mut CompanionView)| {
            let mut v = donut_ai(60.0);
            v.threats = vec![threat(0, 90.0, 6, Some(HeroId::Carl))];
            f(&mut v);
            donut().step_traced(&v, &assist()).1
        };
        assert!(go(&|_| {}).pressed);
        // Farther than 96 px.
        assert_eq!(go(&|v| v.threats[0].x = 60.0 + 97.0).mode, Mode::Follow);
        // The goblin outside the scroll-camera view.
        assert_eq!(go(&|v| v.threats[0].in_view = false).mode, Mode::Follow);
        // Donut herself outside the view (Q40): fully idle.
        let d = go(&|v| v.view.left = 70.0);
        assert!(d.mode == Mode::Follow && !d.pressed);
        // A guarded goblin would glance the swat.
        assert_eq!(go(&|v| v.threats[0].guarded = true).mode, Mode::Follow);
    }

    #[test]
    fn carl_still_interrupts_a_light_guarded_tell() {
        // goblin.rs: the light guard glances further swats; Carl's hits and
        // the pounce still interrupt it.
        let mut v = carl_ai(60.0);
        let mut guarded = threat(0, 85.0, 6, Some(HeroId::Donut));
        guarded.guarded = true;
        v.threats = vec![guarded];
        let (raw, d) = carl().step_traced(&v, &v0());
        assert!(raw.attack && d.pressed);
        assert_eq!((d.mode, d.action), (Mode::Interrupt, Some(Action::Jab)));
    }

    // ------------------------------------------------------- donut mana vetoes --

    #[test]
    fn no_cast_before_the_unlock() {
        // A tell in the missile band, locked: nothing is pressed at any tick.
        for tick in 0..24 {
            let mut v = donut_ai(0.0);
            v.heroes[1].mana = Some(ManaView {
                mana: 8,
                max: 8,
                unlocked: false,
                ai_can_cast: true, // a lying field must not matter
                projectile_speed_px_per_tick: 6,
                bolt_lead_px: 35,
            });
            v.threats = vec![threat(0, 120.0, tick, Some(HeroId::Carl))];
            let (raw, d) = donut().step_traced(&v, &assist());
            assert!(!raw.attack && !d.pressed, "tell tick {tick}");
            assert_ne!(d.action, Some(Action::Missile));
        }
    }

    #[test]
    fn no_cast_below_the_ai_mana_reserve() {
        let mut v = donut_ai(0.0);
        unlock(&mut v, 2, false); // the view says the reserve would be broken
        // Tell tick 1: the bolt arrives at 1 + 4 + 13 = 18, inside the tell.
        v.threats = vec![threat(0, 120.0, 1, Some(HeroId::Carl))];
        let (raw, d) = donut().step_traced(&v, &assist());
        assert!(!raw.attack && !d.pressed && d.mode == Mode::Follow);
        // Above the reserve she fires the Magic Missile (Primary).
        unlock(&mut v, 3, true);
        let (raw, d) = donut().step_traced(&v, &assist());
        assert!(raw.attack && d.pressed);
        assert_eq!(d.action, Some(Action::Missile));
        assert_eq!(d.mode, Mode::Interrupt);
    }

    #[test]
    fn q17_the_missile_band_and_arrival_time_gate_the_cast() {
        let fire = |x: f32, tick: u16| {
            let mut v = donut_ai(0.0);
            unlock(&mut v, 8, true);
            v.threats = vec![threat(0, x, tick, Some(HeroId::Carl))];
            donut().step_traced(&v, &assist()).1.pressed
        };
        assert!(fire(97.0, 0));
        assert!(!fire(96.0, 0), "too close for a bolt");
        assert!(!fire(161.0, 0), "out of the band");
        // Arrival attack tick = tell tick + release (4) + flight, where the
        // flight is floor((d - 12 - 35) / 6) + 1 (the bolt box front starts 35
        // px ahead of Donut); it must be below the tell length (24).
        // 160 px: flight 19, so a cast at tell tick 0 arrives on 23, tick 1 on 24.
        assert!(
            fire(160.0, 0),
            "160 px at tell tick 0 lands on the last tell tick"
        );
        assert!(!fire(160.0, 1), "160 px at tell tick 1 lands on the smear");
        // 100 px: flight 9, so tell tick 10 arrives on 23, tick 11 on 24.
        assert!(fire(100.0, 10), "100 px at tell tick 10");
        assert!(!fire(100.0, 11), "100 px at tell tick 11");
    }

    #[test]
    fn an_unlocked_donut_never_presses_primary_as_a_swat() {
        let mut v = donut_ai(60.0);
        unlock(&mut v, 8, true);
        v.threats = vec![threat(0, 90.0, 6, Some(HeroId::Carl))]; // 30 px: swat range
        let (raw, d) = donut().step_traced(&v, &assist());
        assert!(
            !raw.attack && !d.pressed,
            "Primary is the cast after the unlock"
        );
    }

    // ----------------------------------------------------------- donut pounce --

    #[test]
    fn q15_donut_pounces_a_lying_goblin_in_range_that_the_pounce_cannot_kill() {
        let pounce = |x: f32, hp: u8, state: TargetState, facing: i8, set: &ProfileSet| {
            let mut v = donut_ai(60.0);
            v.heroes[1].facing = facing;
            v.targets = vec![target(0, x, hp, state)];
            let mut d = donut();
            let (raw, dec) = d.step_traced(&v, set);
            (raw.physical && !raw.attack, dec)
        };
        let lying = TargetState::Lying {
            down_ticks_left: 40,
            hold_used: false,
        };
        // Q38 hold (review F-2): a lying goblin that gets up before the pounce
        // lands (fewer ticks left than startup plus flight) is not pounced.
        let min = restrained().donut.cf02_q38_pounce_lying_min_ticks_left;
        assert_eq!(
            min,
            crate::donut_moves::POUNCE_STARTUP + crate::donut_moves::POUNCE_FLIGHT_NOMINAL
        );
        let lying_for = |down_ticks_left| TargetState::Lying {
            down_ticks_left,
            hold_used: false,
        };
        assert!(pounce(110.0, 2, lying_for(min), 1, &restrained()).0);
        assert!(!pounce(110.0, 2, lying_for(min - 1), 1, &restrained()).0);
        let (pressed, dec) = pounce(110.0, 2, lying, 1, &restrained());
        assert!(pressed);
        assert_eq!(
            (dec.mode, dec.action),
            (Mode::SupportAttack, Some(Action::Pounce))
        );
        assert!(
            pounce(
                110.0,
                2,
                TargetState::Staggered { ticks: 5 },
                1,
                &restrained()
            )
            .0
        );
        // Range 30..80 ahead of her (Q15).
        assert!(pounce(90.0, 2, lying, 1, &restrained()).0, "exactly 30 px");
        assert!(pounce(140.0, 2, lying, 1, &restrained()).0, "exactly 80 px");
        assert!(!pounce(89.0, 2, lying, 1, &restrained()).0);
        assert!(!pounce(141.0, 2, lying, 1, &restrained()).0);
        // Behind her, standing, airborne: no.
        assert!(!pounce(110.0, 2, lying, -1, &restrained()).0);
        assert!(!pounce(110.0, 2, TargetState::Standing, 1, &restrained()).0);
        assert!(!pounce(110.0, 2, TargetState::Airborne, 1, &restrained()).0);
        // Never lethal under `never`; unchanged under `factual`.
        assert!(!pounce(110.0, 1, lying, 1, &restrained()).0);
        let mut factual = restrained();
        factual.ca19_kill_policy = KillPolicy::Factual;
        assert!(pounce(110.0, 1, lying, 1, &factual).0);
    }

    #[test]
    fn q15_the_pounce_is_only_on_a_goblin() {
        let mut v = donut_ai(60.0);
        let mut llama = target(0, 110.0, 2, TargetState::Staggered { ticks: 5 });
        llama.id.kind = EntityKind::Llama;
        v.targets = vec![llama];
        let (raw, d) = donut().step_traced(&v, &restrained());
        assert!(!raw.physical && !d.pressed && d.mode == Mode::Follow);
        v.targets[0].id.kind = EntityKind::Goblin;
        assert!(donut().step(&v, &restrained()).physical);
    }

    // ------------------------------------------------------------ duo offense --

    #[test]
    fn duo_offense_is_data_and_off_in_v0_and_the_restrained_set() {
        let mut v = donut_ai(60.0);
        v.targets = vec![target(0, 80.0, 2, TargetState::Standing)];
        for set in [v0(), restrained()] {
            let (raw, d) = donut().step_traced(&v, &set);
            assert!(only_follow(&raw) && d.mode == Mode::Follow, "{d:?}");
        }
        // CF02-DUO-A07: even the assistant has no offensive swat any more:
        // a standing goblin in swat reach is no reason to press Primary (a 0
        // damage swat would set the guard that blocks her own interrupt).
        let (raw, d) = donut().step_traced(&v, &assist());
        assert!(!raw.attack && d.action != Some(Action::Swat), "{d:?}");
    }

    #[test]
    fn duo_donut_pounces_a_standing_goblin_and_may_finish_it() {
        let mut v = donut_ai(60.0);
        v.targets = vec![target(0, 110.0, 2, TargetState::Standing)];
        let (raw, d) = donut().step_traced(&v, &assist());
        assert!(raw.physical && d.action == Some(Action::Pounce), "{d:?}");
        // Not a goblin that is telling, not one out of the band.
        let mut telling = v.clone();
        telling.threats = vec![threat(0, 110.0, 3, Some(HeroId::Donut))];
        let (_, d) = donut().step_traced(&telling, &assist());
        assert_ne!(d.action, Some(Action::Pounce), "{d:?}");
        v.targets[0].x = 150.0;
        let (_, d) = donut().step_traced(&v, &assist());
        assert_ne!(d.action, Some(Action::Pounce), "{d:?}");
        // Factual: a 1 HP goblin Carl is not engaging is pounced too (an idle
        // Carl engages nothing).
        let mut last = donut_ai(60.0);
        last.targets = vec![target(0, 110.0, 1, TargetState::Standing)];
        assert!(donut().step_traced(&last, &assist()).1.action == Some(Action::Pounce));
    }

    /// CF02-DUO-A11 and A12: Carl engages only what he faces, and an enemy he
    /// hit a moment ago stays his across the walk-up between two jabs.
    #[test]
    fn c11_carl_engages_the_front_and_the_enemy_he_just_hit() {
        let set = assist();
        let window = set.cf02_duo.cf02_duo_a12_recent_hit_ticks;
        let wants = |v: &CompanionView, set: &ProfileSet| {
            matches!(
                donut().step_traced(v, set).1.action,
                Some(Action::Pounce | Action::Missile)
            )
        };
        let jab = Some(ActiveMove {
            id: MoveId::Jab,
            phase: MovePhase::Recovery,
            ticks_left: 4,
            ai_started: false,
        });
        // A11: Carl (x=100) jabs right; a 1 HP goblin at 110 is his, one at 90
        // (behind him, 40 px ahead of Donut at 50) is hers.
        let mut front = donut_ai(50.0);
        front.heroes[0].active_move = jab;
        front.targets = vec![target(0, 110.0, 1, TargetState::Standing)];
        let mut back = front.clone();
        back.targets = vec![target(0, 90.0, 1, TargetState::Standing)];
        // Donut at 50 would have the goblin at 110 out of her band: put her at
        // 40 for the front case so both are in her 30..80 px band.
        front.heroes[1].x = 40.0;
        assert!(!wants(&front, &set), "in front of Carl: his");
        assert!(wants(&back, &set), "behind Carl: fair game for her");
        // Carl turned left: the roles swap.
        front.heroes[0].facing = -1;
        back.heroes[0].facing = -1;
        assert!(wants(&front, &set), "behind a left-facing Carl");
        assert!(!wants(&back, &set), "in front of a left-facing Carl");
        let mut off = set.clone();
        off.cf02_duo.cf02_duo_a11_engage_facing_only = false;
        assert!(!wants(&back, &off), "A11 off: any side counts");
        // A12: a goblin Carl hit `window` ticks ago is still his with Carl idle
        // and far from it, and on either side of him (B-1); one tick later it
        // is hers.
        let mut walk = donut_ai(60.0);
        walk.heroes[0].x = 20.0;
        walk.targets = vec![target(0, 110.0, 1, TargetState::Standing)];
        walk.targets[0].carl_hit = true;
        walk.targets[0].carl_hit_age = Some(window);
        assert!(!wants(&walk, &set), "recently hit by Carl: his");
        walk.targets[0].carl_hit_age = Some(window + 1);
        assert!(wants(&walk, &set), "the window is over");
        walk.targets[0].carl_hit_age = Some(window);
        walk.heroes[0].facing = -1;
        assert!(!wants(&walk, &set), "Carl turned away from it: still his");
        walk.targets[0].carl_hit_age = Some(window + 1);
        assert!(wants(&walk, &set), "turned away and the window is over");
        walk.targets[0].carl_hit_age = Some(window);
        let mut no_window = set.clone();
        no_window.cf02_duo.cf02_duo_a12_recent_hit_ticks = 0;
        walk.heroes[0].facing = 1;
        walk.targets[0].carl_hit_age = Some(1);
        assert!(wants(&walk, &no_window), "window 0: only a move or stagger");
    }

    /// CF02-DUO-A15: she idles ahead of Carl while a goblin approaches on the
    /// side he faces, falls back behind him once it is inside the spot plus
    /// her pounce minimum, and has nothing to flank for a goblin behind him,
    /// a lying one, one beyond the range, out of view or on another level.
    #[test]
    fn duo_a15_flank_spot_ahead_then_fall_back_behind_carl() {
        let set = assist();
        let mv = |v: &CompanionView, set: &ProfileSet| donut().step_traced(v, set).0.move_x;
        // Carl at 100 faces right; spot = 100 + 56. A goblin 200 px ahead of
        // him is outside her 80 px pounce band from the spot and from past it.
        let mut v = donut_ai(60.0);
        v.flank_span = Some((0.0, 640.0));
        v.targets = vec![target(0, 300.0, 2, TargetState::Standing)];
        assert_eq!(mv(&v, &set), 1.0, "walks to the spot");
        v.heroes[1].x = 156.0;
        assert_eq!(mv(&v, &set), 0.0, "stands on it");
        v.heroes[1].x = 159.0;
        assert_eq!(mv(&v, &set), 0.0, "inside the dead zone");
        v.heroes[1].x = 200.0;
        assert_eq!(mv(&v, &set), -1.0, "comes back to it from past it");
        // A goblin inside the spot plus her pounce minimum (86 px): she falls
        // back to 40 px behind Carl, where it arrives inside her band.
        v.targets = vec![target(0, 150.0, 2, TargetState::Standing)];
        v.heroes[1].x = 156.0;
        assert_eq!(mv(&v, &set), -1.0, "falls back");
        v.heroes[1].x = 60.0;
        assert_eq!(mv(&v, &set), 0.0, "and stands there");
        // Nothing to flank.
        let lying = TargetState::Lying {
            down_ticks_left: 40,
            hold_used: false,
        };
        let mut none = donut_ai(60.0);
        none.flank_span = Some((0.0, 640.0));
        for (name, t) in [
            ("behind Carl", target(0, -100.0, 2, TargetState::Standing)),
            ("lying", target(0, 250.0, 2, lying)),
            (
                "beyond the range",
                target(0, 341.0, 2, TargetState::Standing),
            ),
            ("out of view", {
                let mut t = target(0, 250.0, 2, TargetState::Standing);
                t.in_view = false;
                t
            }),
            ("another level", {
                let mut t = target(0, 250.0, 2, TargetState::Standing);
                t.foot_y = 80.0;
                t
            }),
        ] {
            none.targets = vec![t];
            assert_eq!(mv(&none, &set), 0.0, "{name}: plain follow");
        }
        // Facing left: the spot is on his left (100 - 56 = 44).
        let mut left = donut_ai(140.0);
        left.heroes[0].facing = -1;
        left.flank_span = Some((0.0, 640.0));
        left.targets = vec![target(0, -50.0, 2, TargetState::Standing)];
        assert_eq!(mv(&left, &set), -1.0);
    }

    /// CF02-DUO-A15 safety: the spot is clamped to the flat ground the view
    /// reports and the camera less the margin; no flat ground, no flank; the
    /// follow layer's jump, a hurt or busy hero and the data flag all win.
    #[test]
    fn duo_a15_flank_stays_on_flat_ground_inside_the_view_and_yields_to_everything() {
        let set = assist();
        let mv = |v: &CompanionView, set: &ProfileSet| donut().step_traced(v, set).0.move_x;
        let mut on = donut_ai(60.0);
        on.flank_span = Some((0.0, 640.0));
        on.targets = vec![target(0, 250.0, 2, TargetState::Standing)];
        assert_eq!(mv(&on, &set), 1.0);
        // The flat ground ends at 120 (a ledge edge): the spot is 120.
        let mut ledge = on.clone();
        ledge.flank_span = Some((0.0, 120.0));
        ledge.heroes[1].x = 118.0;
        assert_eq!(mv(&ledge, &set), 0.0, "stops at the end of the flat ground");
        ledge.heroes[1].x = 140.0;
        assert_eq!(mv(&ledge, &set), -1.0, "never past it");
        // No flat ground under her (airborne, or the nav is driving): plain.
        let mut nothing = on.clone();
        nothing.flank_span = None;
        assert_eq!(mv(&nothing, &set), 0.0);
        // The camera: the spot 496 clamps to 520 - 48 = 472.
        let mut edge = donut_ai(500.0);
        edge.heroes[0].x = 440.0;
        edge.view.right = 520.0;
        edge.flank_span = Some((0.0, 640.0));
        edge.targets = vec![target(0, 600.0, 2, TargetState::Standing)];
        assert_eq!(mv(&edge, &set), -1.0, "inside the camera margin");
        // A follow-layer jump (a dodge, a hop, the nav) is never overridden.
        let mut jump = on.clone();
        jump.follow[1].jump = true;
        assert_eq!(mv(&jump, &set), 0.0, "the jump layer's movement stands");
        // Hurt or in a move: no positioning; nor in the air.
        let mut busy = on.clone();
        busy.heroes[1].can_start_move = false;
        assert_eq!(mv(&busy, &set), 0.0);
        let mut air = on.clone();
        air.heroes[1].grounded = false;
        assert_eq!(mv(&air, &set), 0.0);
        // The data flag and the other set: off means the follow layer.
        let mut off = set.clone();
        off.cf02_duo.cf02_duo_a15_flank = false;
        assert_eq!(mv(&on, &off), 0.0, "flag off");
        assert_eq!(mv(&on, &v0()), 0.0, "companion_v0");
        // A Carl the AI drives, or one that is down, has nothing to flank.
        let mut ai_carl = on.clone();
        ai_carl.heroes[0].control = Control::Companion;
        assert_eq!(mv(&ai_carl, &set), 0.0);
        let mut down = on.clone();
        down.heroes[0].defeated = true;
        assert_eq!(mv(&down, &set), 0.0);
    }

    /// CF02-DUO-A13: an enemy behind her is chosen too; she turns first (a
    /// move step toward it, no press) and presses once she faces it.
    #[test]
    fn duo_a13_she_turns_to_an_enemy_behind_her_then_pounces() {
        let set = assist();
        // Donut at 140 faces right; the goblin stands 50 px behind her, and
        // Carl (x=100, idle) engages nothing.
        let mut v = donut_ai(140.0);
        v.targets = vec![target(0, 90.0, 2, TargetState::Standing)];
        let (raw, d) = donut().step_traced(&v, &set);
        assert_eq!(d.action, Some(Action::Pounce), "{d:?}");
        assert!(!d.pressed && raw.move_x < 0.0, "she turns first: {raw:?}");
        // Facing it, she presses.
        v.heroes[1].facing = -1;
        let (raw, d) = donut().step_traced(&v, &set);
        assert!(d.pressed && raw.physical, "{d:?}");
        // Off: the first-cut rule, nothing behind her is a candidate.
        let mut off = set.clone();
        off.cf02_duo.cf02_duo_a13_turn_to_target = false;
        v.heroes[1].facing = 1;
        assert_ne!(
            donut().step_traced(&v, &off).1.action,
            Some(Action::Pounce),
            "A13 off"
        );
    }

    /// CF02-DUO-A08 (spec C-11 under `factual`): the AI never takes a hit that
    /// would leave Carl's pending hit as the finisher of the enemy he engages.
    #[test]
    fn c11_the_ai_never_steals_the_finish_of_the_enemy_carl_engages() {
        let set = assist();
        let wants = |v: &CompanionView| {
            matches!(
                donut().step_traced(v, &set).1.action,
                Some(Action::Pounce | Action::Missile)
            )
        };
        let jab_running = Some(ActiveMove {
            id: MoveId::Jab,
            phase: MovePhase::Recovery,
            ticks_left: 4,
            ai_started: false,
        });
        // Carl (x=100) has a jab running 10 px from a 1 or 2 HP goblin: no
        // pounce, standing or staggered. With 3 HP she still attacks.
        for (hp, state, expect) in [
            (1, TargetState::Standing, false),
            (2, TargetState::Standing, false),
            (1, TargetState::Staggered { ticks: 8 }, false),
            (3, TargetState::Standing, true),
        ] {
            let mut v = donut_ai(60.0);
            v.heroes[0].active_move = jab_running;
            v.targets = vec![target(0, 110.0, hp, state)];
            assert_eq!(wants(&v), expect, "combo: hp {hp} {state:?}");
            // The combo is over (or Carl is far): she may finish it.
            v.heroes[0].active_move = None;
            assert!(wants(&v), "idle Carl engages nothing: hp {hp} {state:?}");
            v.heroes[0].active_move = jab_running;
            v.heroes[0].x = 400.0;
            assert!(wants(&v), "Carl out of reach: hp {hp} {state:?}");
        }
        // Still down or staggered from Carl's last hit, with Carl at it.
        for state in [
            TargetState::Staggered { ticks: 8 },
            TargetState::Lying {
                down_ticks_left: 40,
                hold_used: false,
            },
        ] {
            let mut v = donut_ai(60.0);
            v.targets = vec![target(0, 110.0, 1, state)];
            assert!(wants(&v), "nobody hit it yet: {state:?}");
            v.targets[0].carl_hit = true;
            assert!(!wants(&v), "Carl's own hit leaves it to him: {state:?}");
        }
        // A Standing goblin Carl hit long ago is not his window.
        let mut old = donut_ai(60.0);
        old.targets = vec![target(0, 110.0, 1, TargetState::Standing)];
        old.targets[0].carl_hit = true;
        assert!(wants(&old));
        // Missile: same rule, band 64..160 from Donut at 60.
        let mut m = donut_ai(60.0);
        m.heroes[0].x = 180.0;
        m.heroes[0].active_move = jab_running;
        unlock(&mut m, 8, true);
        m.targets = vec![target(0, 190.0, 2, TargetState::Standing)];
        assert!(!wants(&m));
        m.targets[0].hp = 3;
        assert!(wants(&m));
        // The rule is data: off, she finishes (the set of the first cut).
        let mut off = assist();
        off.cf02_duo.cf02_duo_a08_respect_finisher = false;
        let mut v = donut_ai(60.0);
        v.heroes[0].active_move = jab_running;
        v.targets = vec![target(0, 110.0, 1, TargetState::Standing)];
        assert!(donut().step_traced(&v, &off).1.action == Some(Action::Pounce));
    }

    /// CF02-DUO-A09: no offensive pounce or missile at an enemy on another
    /// level (feet height).
    #[test]
    fn duo_offense_needs_the_target_on_her_level() {
        let set = assist();
        let mut v = donut_ai(60.0);
        v.heroes[0].x = 400.0;
        v.targets = vec![target(0, 110.0, 3, TargetState::Standing)];
        assert!(donut().step_traced(&v, &set).1.action == Some(Action::Pounce));
        v.targets[0].foot_y = 80.0;
        let (raw, d) = donut().step_traced(&v, &set);
        assert!(!raw.attack && d.action != Some(Action::Pounce), "{d:?}");
        let mut m = donut_ai(60.0);
        m.heroes[0].x = 400.0;
        unlock(&mut m, 8, true);
        m.targets = vec![target(0, 190.0, 3, TargetState::Standing)];
        assert!(donut().step_traced(&m, &set).1.action == Some(Action::Missile));
        m.targets[0].foot_y = 80.0;
        let (raw, d) = donut().step_traced(&m, &set);
        assert!(!raw.attack && d.action != Some(Action::Missile), "{d:?}");
    }

    #[test]
    fn duo_donut_casts_the_missile_only_within_the_mana_rules() {
        let mut v = donut_ai(60.0);
        v.targets = vec![target(0, 190.0, 2, TargetState::Standing)]; // 130 px
        unlock(&mut v, 8, true);
        let (raw, d) = donut().step_traced(&v, &assist());
        assert!(raw.attack && d.action == Some(Action::Missile), "{d:?}");
        // Below the AI reserve (`ai_can_cast` false): no cast, no free one.
        unlock(&mut v, 2, false);
        let (raw, d) = donut().step_traced(&v, &assist());
        assert!(!raw.attack && d.action != Some(Action::Missile), "{d:?}");
        // Out of the band (200 px): not engaged, no cast.
        unlock(&mut v, 8, true);
        v.targets[0].x = 260.0;
        let (raw, d) = donut().step_traced(&v, &assist());
        assert!(!raw.attack && d.mode == Mode::Follow, "{d:?}");
        // Before the unlock the Primary press would be a swat: never offensive.
        let mut locked = donut_ai(60.0);
        locked.targets = vec![target(0, 80.0, 2, TargetState::Standing)];
        let (_, d) = donut().step_traced(&locked, &assist());
        assert_ne!(d.action, Some(Action::Swat));
    }

    #[test]
    fn duo_offense_keeps_the_action_cadence() {
        let mut dir = donut();
        let set = assist();
        assert_eq!(set.cf02_q16_action_cadence_ticks, 20);
        let mut presses = Vec::new();
        for tick in 100..180u64 {
            let mut v = donut_ai(60.0);
            v.tick = tick;
            v.heroes[0].x = 400.0;
            v.targets = vec![target(0, 110.0, 4, TargetState::Standing)];
            if dir.step(&v, &set).physical {
                presses.push(tick);
            }
        }
        assert!(presses.len() >= 3, "{presses:?}");
        for pair in presses.windows(2) {
            assert!(pair[1] - pair[0] >= 20, "{presses:?}");
        }
    }

    // ------------------------------------------------------------- carl rules --

    #[test]
    fn carl_never_presses_ability_pause_swap_or_physical_in_any_view() {
        // Sweep tells, targets and distances: only follow, jump, move, Primary.
        for dx in [-170, -90, -20, 0, 20, 90, 170] {
            for tick in [0u16, 4, 10, 21, 30] {
                for hp in [1u8, 2] {
                    let mut v = carl_ai(100.0 + dx as f32);
                    v.tick = u64::from(tick) * 7 + 1;
                    v.threats = vec![threat(0, 100.0, tick, Some(HeroId::Donut))];
                    v.targets = vec![target(0, 100.0, hp, TargetState::Standing)];
                    for set in [assist(), v0()] {
                        let raw = carl().step(&v, &set);
                        assert!(
                            !raw.ability && !raw.pause && !raw.swap && !raw.physical,
                            "dx {dx} tick {tick} hp {hp}: {raw:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn carl_interrupts_a_tell_aimed_at_the_human_donut_even_when_the_hit_would_glance() {
        let mut v = carl_ai(60.0);
        v.threats = vec![threat(0, 85.0, 4, Some(HeroId::Donut))];
        v.targets = vec![target(0, 85.0, 1, TargetState::Standing)]; // lethal jab: glances
        let (raw, d) = carl().step_traced(&v, &v0());
        assert!(raw.attack);
        assert_eq!((d.mode, d.action), (Mode::Interrupt, Some(Action::Jab)));
        // The jab's startup (3) must land by the tell's end.
        v.threats = vec![threat(0, 85.0, 22, Some(HeroId::Donut))];
        assert_eq!(carl().step_traced(&v, &v0()).1.mode, Mode::Follow);
    }

    #[test]
    fn carl_closes_in_on_a_far_standing_goblin_and_ignores_one_out_of_engage_range() {
        let mut v = carl_ai(0.0);
        v.targets = vec![target(0, 96.0, 2, TargetState::Standing)];
        let (raw, d) = carl().step_traced(&v, &v0());
        assert!(!raw.attack && raw.move_x == 1.0 && d.mode == Mode::SupportAttack);
        v.targets = vec![target(0, 97.0, 2, TargetState::Standing)];
        assert_eq!(carl().step_traced(&v, &v0()).1.mode, Mode::Follow);
    }

    #[test]
    fn kill_policy_never_keeps_carl_off_a_lethal_standing_target_and_factual_does_not() {
        let mut v = carl_ai(60.0);
        v.targets = vec![target(0, 85.0, 1, TargetState::Standing)];
        let (raw, d) = carl().step_traced(&v, &v0());
        assert!(
            !raw.attack && d.mode == Mode::Follow,
            "a glance would waste the move"
        );
        let mut factual = v0();
        factual.ca19_kill_policy = KillPolicy::Factual;
        let (raw, d) = carl().step_traced(&v, &factual);
        assert!(raw.attack && d.mode == Mode::SupportAttack);
    }

    // ------------------------------------------------------------ jab chain --

    /// AI Carl mid-jab (`elapsed` ticks into it) facing a goblin 25 px ahead.
    fn mid_jab(elapsed: u16, hp: u8, ai_started: bool, tick: u64) -> CompanionView {
        let mut v = carl_ai(60.0);
        v.tick = tick;
        v.heroes[0].can_start_move = false;
        v.heroes[0].active_move = Some(ActiveMove {
            id: MoveId::Jab,
            phase: MovePhase::Recovery,
            ticks_left: CARL_JAB.total() - elapsed,
            ai_started,
        });
        v.targets = vec![target(0, 85.0, hp, TargetState::Standing)];
        v
    }

    #[test]
    fn ca36_ai_carl_chains_the_haymaker_inside_the_jab_window() {
        let mut dir = carl();
        let set = v0();
        // The jab press itself (tick 100).
        let mut v = carl_ai(60.0);
        v.tick = 100;
        v.targets = vec![target(0, 85.0, 3, TargetState::Standing)];
        let (raw, d) = dir.step_traced(&v, &set);
        assert!(raw.attack && d.action == Some(Action::Jab));
        // Sim ordering: the view is pre-tick and the move clock advances before
        // the press is read, so a press at view `elapsed` is judged at
        // `elapsed + 1`. Judged before the jab's active ticks are over (< 5): no.
        for elapsed in [1, 3] {
            let (raw, d) = dir.step_traced(&mid_jab(elapsed, 3, true, 101), &set);
            assert!(!raw.attack && !d.pressed, "elapsed {elapsed}");
        }
        // At elapsed 13 the press is judged at 14 == CARL_JAB.total(): the jab
        // is over, and the press would start a fresh jab that bypasses the
        // veto and the Q16 cadence. It must not be pressed at all.
        let (raw, d) = dir.step_traced(&mid_jab(13, 3, true, 101), &set);
        assert!(
            !raw.attack && !d.pressed,
            "elapsed 13 is outside the window"
        );
        assert_ne!(d.action, Some(Action::Haymaker));
        // Inside the window the follow-up is the haymaker, with no cadence veto.
        for elapsed in [4, 5, 9, 12] {
            let mut d2 = dir.clone();
            let (raw, d) = d2.step_traced(&mid_jab(elapsed, 3, true, 105), &set);
            assert!(raw.attack && !raw.physical, "elapsed {elapsed}");
            assert_eq!((d.action, d.veto), (Some(Action::Haymaker), None));
            assert_eq!(d.mode, Mode::SupportAttack);
        }
        // Not a jab the AI started: no chain.
        let (raw, _) = dir.clone().step_traced(&mid_jab(6, 3, false, 106), &set);
        assert!(!raw.attack);
        // The chain does not count against the one-move-per-60-ticks limit:
        // the cadence still runs from the jab press at tick 100.
        let (raw, _) = dir.step_traced(&mid_jab(6, 3, true, 106), &set);
        assert!(raw.attack);
        let free = |tick| {
            let mut v = carl_ai(60.0);
            v.tick = tick;
            v.targets = vec![target(0, 85.0, 3, TargetState::Standing)];
            v
        };
        let (raw, d) = dir.step_traced(&free(159), &set);
        assert!(!raw.attack && d.veto == Some(Veto::ActionCadence));
        assert!(dir.step_traced(&free(160), &set).0.attack, "100 + 60");
    }

    #[test]
    fn the_chain_follows_the_kill_policy_on_the_hp_after_the_jab() {
        // Under `never` a haymaker that would defeat is not chained.
        let (raw, d) = carl().step_traced(&mid_jab(6, 1, true, 106), &v0());
        assert!(!raw.attack && d.mode == Mode::Follow);
        let mut factual = v0();
        factual.ca19_kill_policy = KillPolicy::Factual;
        let (raw, d) = carl().step_traced(&mid_jab(6, 1, true, 106), &factual);
        assert!(raw.attack && d.action == Some(Action::Haymaker));
        // The passive cf02_assist Carl never chains.
        assert!(!carl().step(&mid_jab(6, 3, true, 106), &assist()).attack);
    }

    // -------------------------------------------------------- authority vetoes --

    #[test]
    fn no_press_while_the_hero_cannot_start_a_move() {
        let aimed = |mutate: &dyn Fn(&mut HeroView)| {
            let mut v = donut_ai(60.0);
            v.threats = vec![threat(0, 90.0, 6, Some(HeroId::Carl))];
            mutate(&mut v.heroes[1]);
            donut().step_traced(&v, &assist())
        };
        let (raw, d) = aimed(&|_| {});
        assert!(raw.attack && d.veto.is_none());
        let blocked: [&dyn Fn(&mut HeroView); 3] =
            [&|h| h.can_start_move = false, &|h| h.hurt_ticks = 5, &|h| {
                h.active_move = Some(ActiveMove {
                    id: MoveId::Swat,
                    phase: MovePhase::Startup,
                    ticks_left: 5,
                    ai_started: true,
                });
            }];
        for mutate in blocked {
            let (raw, d) = aimed(mutate);
            assert!(!raw.attack && !raw.physical && !d.pressed);
            assert_eq!(d.veto, Some(Veto::CannotStartMove));
            assert_eq!(d.mode, Mode::Interrupt, "she still wants it");
        }
    }

    #[test]
    fn nothing_is_emitted_for_a_hero_that_is_not_companion_driven_or_is_down() {
        for control in [Control::Human(0), Control::Human(1), Control::Bot] {
            let mut v = donut_ai(60.0);
            v.heroes[1].control = control;
            v.threats = vec![threat(0, 90.0, 6, Some(HeroId::Carl))];
            v.follow[1] = FollowInput {
                move_x: 1.0,
                jump: true,
            };
            let (raw, d) = donut().step_traced(&v, &assist());
            assert_eq!(raw, RawInput::NONE);
            assert_eq!(d.veto, Some(Veto::NotCompanionDriven));
        }
        let mut v = donut_ai(60.0);
        v.heroes[1].defeated = true;
        let (raw, d) = donut().step_traced(&v, &assist());
        assert_eq!((raw, d.veto), (RawInput::NONE, Some(Veto::HeroDown)));
    }

    #[test]
    fn ca26_no_jump_during_the_heros_own_move() {
        let follow_jump = |active: Option<ActiveMove>, set: &ProfileSet| {
            let mut v = donut_ai(60.0);
            v.heroes[1].active_move = active;
            v.heroes[1].can_start_move = active.is_none();
            v.follow[1] = FollowInput {
                move_x: -1.0,
                jump: true,
            };
            donut().step_traced(&v, set)
        };
        let mv = |phase| {
            Some(ActiveMove {
                id: MoveId::Swat,
                phase,
                ticks_left: 3,
                ai_started: true,
            })
        };
        for phase in [MovePhase::Recovery, MovePhase::Startup, MovePhase::Active] {
            let (raw, d) = follow_jump(mv(phase), &assist());
            assert!(!raw.jump, "{phase:?}");
            assert_eq!(raw.move_x, -1.0, "movement still follows");
            assert!(d.jump_dropped && d.veto == Some(Veto::JumpDuringMove));
        }
        // No move: the dodge jump passes through.
        let (raw, d) = follow_jump(None, &assist());
        assert!(raw.jump && !d.jump_dropped);
        // The bail-out knob (never on in v0 data) would let it through.
        let mut bail = assist();
        bail.ca26_ai_bailout = true;
        assert!(follow_jump(mv(MovePhase::Recovery), &bail).0.jump);
    }

    #[test]
    fn a_press_tick_never_jumps_and_a_held_jump_is_released_first() {
        let mut v = donut_ai(60.0);
        v.threats = vec![threat(0, 90.0, 6, Some(HeroId::Carl))];
        v.follow[1].jump = true;
        let (raw, d) = donut().step_traced(&v, &assist());
        assert!(raw.attack && !raw.jump && d.jump_dropped);
        // Jump on consecutive ticks: edge, release, edge.
        let mut v = donut_ai(60.0);
        v.follow[1].jump = true;
        let mut dir = donut();
        let jumps: Vec<bool> = (0..4)
            .map(|i| {
                v.tick = 100 + i;
                dir.step(&v, &assist()).jump
            })
            .collect();
        assert_eq!(jumps, [true, false, true, false]);
    }

    // ----------------------------------------------------------------- cadence --

    #[test]
    fn q16_at_most_one_ai_move_per_action_cadence() {
        let mut dir = donut();
        let set = restrained();
        let mut presses = Vec::new();
        for tick in 100..240u64 {
            let mut v = donut_ai(60.0);
            v.tick = tick;
            // A goblin keeps starting a fresh tell at tell tick 6, aimed at Carl.
            v.threats = vec![threat(0, 90.0, 6, Some(HeroId::Carl))];
            let (raw, d) = dir.step_traced(&v, &set);
            if raw.attack {
                presses.push(tick);
            }
            if !raw.attack {
                assert_eq!(d.veto, Some(Veto::ActionCadence), "tick {tick}");
            }
        }
        assert_eq!(presses, [100, 160, 220]);
    }

    #[test]
    fn a_press_needs_a_released_button_the_tick_before() {
        let mut set = assist();
        set.cf02_q16_action_cadence_ticks = 1; // isolate the edge rule
        let mut dir = donut();
        let mut seen = Vec::new();
        for tick in 100..104u64 {
            let mut v = donut_ai(60.0);
            v.tick = tick;
            v.threats = vec![threat(0, 90.0, 6, Some(HeroId::Carl))];
            let (raw, d) = dir.step_traced(&v, &set);
            seen.push((raw.attack, d.veto));
        }
        assert_eq!(
            seen,
            [
                (true, None),
                (false, Some(Veto::PressEdge)),
                (true, None),
                (false, Some(Veto::PressEdge)),
            ]
        );
    }

    #[test]
    fn planning_runs_every_six_ticks_but_interrupts_are_immediate() {
        let mut dir = carl();
        let set = v0();
        let mut v = carl_ai(60.0);
        v.tick = 100;
        assert_eq!(dir.step_traced(&v, &set).1.mode, Mode::Follow); // planned: nothing
        // A standing goblin appears: not planned for until tick 106 ...
        for tick in 101..106 {
            v.tick = tick;
            v.targets = vec![target(0, 85.0, 2, TargetState::Standing)];
            assert_eq!(
                dir.step_traced(&v, &set).1.mode,
                Mode::Follow,
                "tick {tick}"
            );
        }
        v.tick = 106;
        assert_eq!(dir.step_traced(&v, &set).1.mode, Mode::SupportAttack);
        // ... while a tell aimed at the human Donut is answered on the next tick.
        let mut dir = carl();
        v.tick = 200;
        v.targets.clear();
        dir.step(&v, &set);
        v.tick = 201;
        v.threats = vec![threat(0, 85.0, 4, Some(HeroId::Donut))];
        assert_eq!(dir.step_traced(&v, &set).1.mode, Mode::Interrupt);
    }

    // --------------------------------------------------------------- selection --

    #[test]
    fn ties_break_by_stable_entity_id() {
        // Two goblins at the same distance (one each side, same HP).
        let mut v = carl_ai(100.0);
        v.heroes[1].x = 300.0;
        v.targets = vec![
            target(1, 130.0, 2, TargetState::Standing),
            target(3, 70.0, 2, TargetState::Standing),
        ];
        let (_, d) = carl().step_traced(&v, &v0());
        assert_eq!(d.target, Some(StableEntityId::goblin(1)));
        // Same for interrupts.
        let mut v = carl_ai(100.0);
        v.threats = vec![
            threat(2, 130.0, 4, Some(HeroId::Donut)),
            threat(5, 70.0, 4, Some(HeroId::Donut)),
        ];
        let (_, d) = carl().step_traced(&v, &v0());
        assert_eq!(d.target, Some(StableEntityId::goblin(2)));
        // A nearer target beats a lower id.
        v.threats[1].x = 90.0;
        let (_, d) = carl().step_traced(&v, &v0());
        assert_eq!(d.target, Some(StableEntityId::goblin(5)));
    }

    #[test]
    fn an_interrupt_beats_a_support_attack() {
        let mut v = carl_ai(60.0);
        v.targets = vec![
            target(0, 80.0, 2, TargetState::Standing),
            target(1, 85.0, 2, TargetState::Standing),
        ];
        v.threats = vec![threat(1, 85.0, 4, Some(HeroId::Donut))];
        let (_, d) = carl().step_traced(&v, &v0());
        assert_eq!(
            (d.mode, d.target),
            (Mode::Interrupt, Some(StableEntityId::goblin(1)))
        );
    }

    #[test]
    fn a_respawn_into_the_same_slot_never_aliases_a_stale_plan() {
        let mut dir = carl();
        let set = v0();
        let mut v = carl_ai(0.0);
        v.tick = 100;
        v.targets = vec![target(0, 90.0, 2, TargetState::Standing)];
        assert_eq!(dir.step_traced(&v, &set).1.mode, Mode::SupportAttack);
        // Next tick: slot 0 holds a new spawn (serial 1). The plan is dropped
        // and, until the next planning tick, nothing is attacked.
        v.tick = 101;
        v.targets[0].id.spawn_serial = 1;
        let (raw, d) = dir.step_traced(&v, &set);
        assert_eq!(d.mode, Mode::Follow);
        assert!(!raw.attack);
        v.tick = 106;
        let d = dir.step_traced(&v, &set).1;
        assert_eq!(d.target.map(|t| t.spawn_serial), Some(1));
    }

    // ------------------------------------------------------------- determinism --

    /// A scripted fight: goblins cycling stand, tell, stagger, lying; the heroes
    /// drift. Pure integer arithmetic, no RNG.
    fn scripted(tick: u64, donut_ai_driven: bool) -> CompanionView {
        let phase = (tick % 120) as u16;
        let drift = ((tick / 3) % 40) as f32;
        // The AI hero stands 25 px behind the goblin; the human 60 px behind it.
        let (carl_c, donut_c, carl_x, donut_x) = if donut_ai_driven {
            (Control::Human(0), Control::Companion, 65.0, 100.0)
        } else {
            (Control::Companion, Control::Human(0), 100.0, 65.0)
        };
        let mut v = view(
            hero(HeroId::Carl, carl_x + drift, carl_c),
            hero(HeroId::Donut, donut_x + drift, donut_c),
        );
        v.tick = tick;
        v.follow[0].move_x = if tick % 17 < 5 { 1.0 } else { 0.0 };
        v.follow[1].jump = tick.is_multiple_of(23);
        let aim = if donut_ai_driven {
            HeroId::Carl
        } else {
            HeroId::Donut
        };
        if phase < 30 {
            v.threats = vec![threat(0, 125.0 + drift, phase, Some(aim))];
        }
        let state = match phase {
            0..40 => TargetState::Standing,
            40..80 => TargetState::Staggered { ticks: 3 },
            _ => TargetState::Lying {
                down_ticks_left: 120 - phase,
                hold_used: false,
            },
        };
        v.targets = vec![
            target(0, 125.0 + drift, 2, state),
            target(
                2,
                165.0 + drift,
                2,
                if phase >= 80 {
                    TargetState::Lying {
                        down_ticks_left: 120 - phase,
                        hold_used: false,
                    }
                } else {
                    TargetState::Standing
                },
            ),
        ];
        v
    }

    fn run_script(donut_ai_driven: bool, set: &ProfileSet) -> (Vec<String>, Vec<u64>) {
        let mut dir = if donut_ai_driven { donut() } else { carl() };
        let mut outputs = Vec::new();
        let mut hashes = Vec::new();
        for tick in 0..1200 {
            let (raw, d) = dir.step_traced(&scripted(tick, donut_ai_driven), set);
            // f32 compared by bit pattern: byte-identical, not just equal.
            outputs.push(format!(
                "{:08x} {} {} {} {} {} {} {:?} {:?}",
                raw.move_x.to_bits(),
                raw.jump,
                raw.attack,
                raw.physical,
                raw.ability,
                raw.pause,
                raw.swap,
                d.mode,
                d.action
            ));
            hashes.push(dir.state_hash());
        }
        (outputs, hashes)
    }

    #[test]
    fn the_same_view_sequence_gives_byte_identical_outputs_and_hashes() {
        for (donut_driven, set) in [(true, assist()), (true, v0()), (false, v0())] {
            let a = run_script(donut_driven, &set);
            let b = run_script(donut_driven, &set);
            assert_eq!(a, b);
            // The script is not trivial: it presses and the hash moves.
            let presses =
                a.0.iter()
                    .filter(|o| o.split(' ').skip(2).take(2).any(|f| f == "true"))
                    .count();
            assert!(presses >= 5, "{presses} presses");
            assert!(a.1.windows(2).any(|w| w[0] != w[1]));
        }
    }

    const PINNED: [u64; 4] = [
        0x252a_56b0_1aac_3e38,
        0x01ea_ed6c_f685_5811,
        0xc5b9_6b4c_7828_d662,
        0x252a_56b0_1aac_3e38,
    ];

    /// FNV-1a over a whole scripted run: every output bit and every state hash.
    fn trace_fingerprint(donut_ai_driven: bool, set: &ProfileSet) -> u64 {
        let (outputs, hashes) = run_script(donut_ai_driven, set);
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        let mut mix = |bytes: &[u8]| {
            for byte in bytes {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        for line in &outputs {
            mix(line.as_bytes());
        }
        for h in &hashes {
            mix(&h.to_le_bytes());
        }
        hash
    }

    /// COMPANION-AI-02 CA2-B1: the director with no brief is today's director,
    /// byte for byte. The fingerprints were taken on the code before
    /// `step_briefed` existed (the CA2-A merge), so a change to the `None` path
    /// moves them.
    #[test]
    fn no_brief_is_the_pre_ca2_director_byte_for_byte() {
        let free = set_of(ProfileSetId::Cf02Free);
        let cases = [(true, assist()), (true, v0()), (false, v0()), (true, free)];
        let got: Vec<u64> = cases
            .iter()
            .map(|(donut_driven, set)| trace_fingerprint(*donut_driven, set))
            .collect();
        assert_eq!(got, PINNED);
    }

    #[test]
    fn a_different_profile_set_gives_a_different_trace_for_carl() {
        let assist_trace = run_script(false, &assist());
        let v0_trace = run_script(false, &v0());
        assert_ne!(assist_trace, v0_trace);
        assert!(
            assist_trace
                .0
                .iter()
                .all(|o| o.split(' ').nth(2) == Some("false")),
            "passive Carl never presses"
        );
    }

    #[test]
    fn the_state_hash_follows_the_state() {
        let mut a = donut();
        let b = donut();
        assert_eq!(a.state_hash(), b.state_hash());
        let mut v = donut_ai(60.0);
        v.threats = vec![threat(0, 90.0, 6, Some(HeroId::Carl))];
        a.step(&v, &assist());
        assert_ne!(a.state_hash(), b.state_hash());
        let mut mixed = Vec::new();
        a.hash_into(&mut |x| mixed.push(x));
        let mut again = Vec::new();
        a.clone().hash_into(&mut |x| again.push(x));
        assert_eq!(mixed, again);
        // The hero is part of the state.
        assert_ne!(carl().state_hash(), donut().state_hash());
        a.clear_plan();
        assert_eq!(a.mode(), Mode::Follow);
        assert_eq!(a.hero(), HeroId::Donut);
    }

    #[test]
    fn follow_layer_movement_passes_through_with_no_work_to_do() {
        let mut v = donut_ai(60.0);
        v.follow[1] = FollowInput {
            move_x: -0.5,
            jump: false,
        };
        let (raw, d) = donut().step_traced(&v, &assist());
        assert_eq!(
            raw,
            RawInput {
                move_x: -0.5,
                ..RawInput::NONE
            }
        );
        assert_eq!(d, Decision::idle(None));
    }

    /// Director cost per call (the PR's Performance Data). Run with
    /// `--nocapture` to read it; the bound is deliberately generous.
    #[test]
    fn director_cost_per_call() {
        use std::time::Instant;
        let set = v0();
        let mut views: Vec<CompanionView> = (0..1200).map(|t| scripted(t, true)).collect();
        let mut dir = donut();
        let laps = 200u64;
        let start = Instant::now();
        let mut sink = 0u64;
        for _ in 0..laps {
            for v in &mut views {
                let raw = dir.step(v, &set);
                sink = sink.wrapping_add(u64::from(raw.attack || raw.physical));
                v.tick += 1200; // ticks keep increasing across laps
            }
        }
        let calls = laps * 1200;
        let per_call = start.elapsed().as_nanos() as f64 / calls as f64;
        println!("companion director: {per_call:.0} ns per call (sink {sink})");
        assert!(per_call < 100_000.0, "{per_call} ns per call");
    }
    /// COMPANION-AI-02 CA2-A: extracting the engagement predicate changed no
    /// `steals_the_finish` result. The method keeps its gates and the HP
    /// reserve and must equal them composed with the old inline predicate, on a
    /// grid of sets, leaders, targets and damages.
    #[test]
    fn steals_the_finish_is_unchanged_by_the_leader_engages_extraction() {
        use crate::companion::engage::reference_inline;
        let sets = [
            set_of(ProfileSetId::Cf02Assist),
            set_of(ProfileSetId::CompanionV0),
            set_of(ProfileSetId::Cf02Free),
        ];
        let mut stolen = 0u32;
        let mut total = 0u32;
        for set in &sets {
            for control in [Control::Human(0), Control::Companion] {
                for defeated in [false, true] {
                    for facing in [-1i8, 1] {
                        for moving in [false, true] {
                            let mut v = donut_ai(300.0);
                            v.heroes[0].control = control;
                            v.heroes[0].defeated = defeated;
                            v.heroes[0].facing = facing;
                            v.heroes[0].active_move = moving.then_some(ActiveMove {
                                id: MoveId::Jab,
                                phase: MovePhase::Active,
                                ticks_left: 6,
                                ai_started: false,
                            });
                            for state in [
                                TargetState::Standing,
                                TargetState::Staggered { ticks: 4 },
                                TargetState::Lying {
                                    down_ticks_left: 30,
                                    hold_used: false,
                                },
                            ] {
                                for hp in [1u8, 2, 3] {
                                    for carl_hit in [false, true] {
                                        for age in [None, Some(5), Some(31), Some(40)] {
                                            for dx in
                                                [-120.0f32, -30.0, 0.0, 30.0, 39.0, 41.0, 120.0]
                                            {
                                                let mut t = target(0, 100.0 + dx, hp, state);
                                                t.carl_hit = carl_hit;
                                                t.carl_hit_age = age;
                                                for damage in [1u8, 2] {
                                                    let duo = &set.cf02_duo;
                                                    let carl = &v.heroes[0];
                                                    let expected = set.ca19_kill_policy
                                                        == KillPolicy::Factual
                                                        && duo.cf02_duo_a08_respect_finisher
                                                        && matches!(
                                                            carl.control,
                                                            Control::Human(_)
                                                        )
                                                        && !carl.defeated
                                                        && t.hp <= damage.saturating_add(
                                                            duo.cf02_duo_a08_finisher_reserve_hp,
                                                        )
                                                        && reference_inline(set, carl, &t);
                                                    let got = donut()
                                                        .steals_the_finish(&v, set, &t, damage);
                                                    assert_eq!(got, expected);
                                                    total += 1;
                                                    stolen += u32::from(got);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(stolen > 100 && stolen < total, "{stolen}/{total}");
        // Carl's director never steals: the method is Donut's only.
        let v = donut_ai(300.0);
        let mut t = target(0, 120.0, 1, TargetState::Standing);
        t.carl_hit_age = Some(3);
        assert!(!carl().steals_the_finish(&v, &assist(), &t, 1));
    }

    // ------------------------------------------- FA-Q04: the boss as a target ----

    fn boss_view_at(x: f32) -> crate::companion::view::BossView {
        use crate::companion::view::{BossModeClass, BossView};
        BossView {
            x,
            foot_y: 0.0,
            facing: -1,
            hp: 30,
            phase: 1,
            hurtbox: None,
            mode_class: BossModeClass::Vulnerable,
            vulnerable_ticks_left: 60,
            target: None,
            charging: false,
            pound_recovery: false,
            inert: true,
            ram_box: (0.0, 0.0),
            committed_box: None,
            ripple_reach_px: 0.0,
        }
    }

    fn boss_brief() -> GoalBrief {
        GoalBrief::targeting(BOSS_ID, 30.0, 160.0)
    }

    #[test]
    fn a_boss_punish_brief_casts_at_the_boss_inside_the_missile_band() {
        let set = set_of(ProfileSetId::Cf02Free);
        let mut v = donut_ai(100.0);
        v.heroes[0].x = 50.0;
        unlock(&mut v, 8, true);
        v.boss_view = Some(boss_view_at(212.0));
        let (raw, d) = donut().step_briefed(&v, &set, Some(&boss_brief()));
        assert_eq!(d.action, Some(Action::Missile), "{d:?}");
        assert_eq!(d.target, Some(BOSS_ID));
        assert!(d.pressed && raw.attack, "{d:?}");
    }

    #[test]
    fn a_boss_punish_brief_pounces_when_dry_and_only_inside_the_pounce_band() {
        let set = set_of(ProfileSetId::Cf02Free);
        let mut v = donut_ai(100.0);
        unlock(&mut v, 0, false);
        v.boss_view = Some(boss_view_at(156.0));
        let (raw, d) = donut().step_briefed(&v, &set, Some(&boss_brief()));
        assert_eq!(d.action, Some(Action::Pounce), "{d:?}");
        assert!(d.pressed && raw.physical);
        // 100 px away is neither the pounce band (30..80) nor a cast (dry).
        v.boss_view = Some(boss_view_at(200.0));
        let (_, d) = donut().step_briefed(&v, &set, Some(&boss_brief()));
        assert_eq!(d.action, None, "{d:?}");
    }

    #[test]
    fn the_boss_is_never_planned_on_without_a_brief_that_names_it() {
        let set = set_of(ProfileSetId::Cf02Free);
        let mut v = donut_ai(100.0);
        unlock(&mut v, 8, true);
        v.boss_view = Some(boss_view_at(212.0));
        // No brief: today's director, which never looks at the boss view.
        let (_, d) = donut().step_briefed(&v, &set, None);
        assert_eq!(d.action, None);
        // A brief for something else: the filter keeps it off the boss.
        let (_, d) = donut().step_briefed(&v, &set, Some(&GoalBrief::interrupts_only()));
        assert_eq!(d.action, None);
    }

    #[test]
    fn a_boss_plan_dies_with_its_target_and_needs_the_view() {
        let set = set_of(ProfileSetId::Cf02Free);
        let mut v = donut_ai(100.0);
        unlock(&mut v, 8, true);
        v.boss_view = Some(boss_view_at(212.0));
        let mut d = donut();
        let (_, first) = d.step_briefed(&v, &set, Some(&boss_brief()));
        assert_eq!(first.target, Some(BOSS_ID));
        // The boss is gone from the view: the plan is dropped at once.
        v.boss_view = None;
        let (_, next) = d.step_briefed(&v, &set, Some(&boss_brief()));
        assert_eq!(next.action, None, "{next:?}");
    }

    // ------------------------------------------------ FA-Q29: pounce timing ----

    #[test]
    fn fa_q29_a_pounce_lands_four_ticks_before_the_swing_or_is_held() {
        // The swing in 6 ticks: landing in 8 is refused, 3 is refused (slack 3),
        // 2 is accepted (slack 4); exact on both sides of the edge.
        assert!(!pounce_lands_in_time(8, 6, 4));
        assert!(!pounce_lands_in_time(3, 6, 4));
        assert!(pounce_lands_in_time(2, 6, 4));
        assert!(pounce_lands_in_time(24, 28, 4));
        assert!(!pounce_lands_in_time(24, 27, 4));
    }

    #[test]
    fn fa_q29_a_pounce_on_a_goblin_with_a_swing_pending_is_held_and_the_knob_lifts_it() {
        // A staggered goblin that is also in a tell with 20 ticks to impact: the
        // pounce (landing in 24) would land after the blow.
        let mut set = set_of(ProfileSetId::Cf02Free);
        let mut v = donut_ai(100.0);
        v.targets = vec![target(1, 150.0, 2, TargetState::Staggered { ticks: 8 })];
        v.threats = vec![threat(1, 150.0, 6, Some(HeroId::Donut))];
        let brief = GoalBrief::targeting(StableEntityId::goblin(1), 30.0, 80.0);
        let (_, d) = donut().step_briefed(&v, &set, Some(&brief));
        assert_eq!(d.action, None, "{d:?}");
        set.fix_all.fa_q29_pounce_timing = false;
        let (_, d) = donut().step_briefed(&v, &set, Some(&brief));
        assert_eq!(d.action, Some(Action::Pounce), "{d:?}");
    }
}
