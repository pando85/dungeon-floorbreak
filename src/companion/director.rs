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
//! `decision_cadence_ticks` (6) and revalidated every tick; interrupts are
//! evaluated on every tick. One AI-started move per `action_cadence_ticks`
//! (CF02-Q16).
//!
//! **Safety vetoes.** No press while the hero cannot start a move; no Donut
//! cast before the unlock or below the AI mana reserve (the view's
//! `ai_can_cast`); Carl never grabs or throws explosives (`ability` is never
//! set); never pause or swap; no Down (the `RawInput` of this stage has none);
//! no Jump while the hero's own move is running (CA-26, the bail-out is
//! CF02-Q55 and the AI never does it); and a press needs a released button on
//! the tick before, so it is always a clean press edge.

use crate::combat::CARL_JAB;
use crate::companion::profile::{KillPolicy, ProfileSet};
use crate::companion::view::{
    CompanionView, Control, HeroView, StableEntityId, TargetState, TargetView, ThreatView,
};
use crate::donut_moves::{POUNCE_DAMAGE, SWAT_STARTUP};
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
    /// CF02-Q16: an AI move started less than `action_cadence_ticks` ago.
    ActionCadence,
    /// The button was held on the previous tick: a press needs a gap.
    PressEdge,
    /// CA-26: a Jump during the hero's own move is dropped.
    JumpDuringMove,
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

/// Integer distance score: whole pixels, rounded down.
fn px(distance: f32) -> i32 {
    distance as i32
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
        debug_assert_eq!(
            view.validate(),
            Ok(()),
            "the view builder owes its guarantees"
        );
        let me = *view.hero(self.hero);
        if me.control != Control::Companion {
            self.clear_plan();
            return (
                RawInput::default(),
                Decision::idle(Some(Veto::NotCompanionDriven)),
            );
        }
        if me.defeated {
            self.clear_plan();
            return (RawInput::default(), Decision::idle(Some(Veto::HeroDown)));
        }
        let active = match self.hero {
            HeroId::Carl => set.carl.active,
            HeroId::Donut => set.donut.active,
        };

        // Planning (CA-10): the support plan every `decision_cadence_ticks`.
        let due = self
            .last_plan_tick
            .is_none_or(|t| view.tick.saturating_sub(t) >= u64::from(set.decision_cadence_ticks));
        if due {
            self.last_plan_tick = Some(view.tick);
            self.plan = if active {
                self.best_support(view, set, &me).map(|c| Plan {
                    action: c.action,
                    target: c.target,
                })
            } else {
                None
            };
        }

        // Urgent checks (CA-10): interrupts every tick, the plan revalidated.
        let urgent = if active {
            self.best_interrupt(view, set, &me)
        } else {
            None
        };
        let (mode, goal) = match urgent {
            Some(c) => (Mode::Interrupt, Some(c)),
            None => {
                let planned = self.plan.and_then(|p| self.revalidate(view, set, &me, p));
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
        self.mode = mode;

        let follow = view.follow[self.hero.index()];
        let mut raw = RawInput {
            move_x: follow.move_x,
            ..RawInput::default()
        };
        let mut jump = follow.jump;
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
            let in_reach = distance >= c.reach_min && distance <= c.reach_max;
            let facing_ok = sign(c.dx) == me.facing;
            if !in_reach {
                // Close in (or back off a target that is too close).
                raw.move_x = if distance > c.reach_max {
                    sign(c.dx) as f32
                } else {
                    -(sign(c.dx) as f32)
                };
            } else if !facing_ok {
                raw.move_x = sign(c.dx) as f32; // turn to it, press next tick
            } else {
                decision.veto = self.press_veto(view, set, &me, c.action);
                if decision.veto.is_none() {
                    if c.action.uses_physical() {
                        next.physical = true;
                    } else {
                        next.attack = true;
                    }
                    decision.pressed = true;
                    self.last_action_tick = Some(view.tick);
                }
            }
        }

        // CA-26: no Jump during the hero's own move; a press tick never jumps;
        // a held jump is released first so the next one is an edge.
        if jump {
            let in_move = me.active_move.is_some() && !set.ai_bailout;
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
            && view.tick.saturating_sub(last) < u64::from(set.action_cadence_ticks)
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
            if threat.aimed_at != Some(owner) || !threat.in_view || threat.guarded {
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
                    // tick, within range, and the hit tick must land by the
                    // last tell tick.
                    let lands = threat.tell_tick + SWAT_STARTUP <= threat.tell_len;
                    (threat.tell_tick >= d.cf02_q14_swat_tell_tick
                        && distance <= f32::from(d.cf02_q14_swat_range_px)
                        && lands)
                        .then(|| slot(Action::Swat, d.swat_hit_min_px, d.swat_hit_max_px))
                } else {
                    // Q17: never before the unlock (above), only above the AI
                    // reserve, only in the missile band, early in the tell.
                    (mana.ai_can_cast
                        && threat.tell_tick <= d.missile_latest_tell_tick
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
                let lands = threat.tell_tick + CARL_JAB.startup <= threat.tell_len;
                (distance <= f32::from(c.interrupt_range_px) && lands)
                    .then(|| slot(Action::Jab, c.strike_min_px, c.strike_max_px))
            }
        }
    }

    /// The best non-urgent support (pounce, strike), over the ordered targets.
    fn best_support(
        &self,
        view: &CompanionView,
        set: &ProfileSet,
        me: &HeroView,
    ) -> Option<Candidate> {
        if !view.view.in_view(me.x) {
            return None;
        }
        let mut best = None;
        for target in &view.targets {
            for action in [Action::Pounce, Action::Jab] {
                better(
                    &mut best,
                    self.support_candidate(view, set, me, target, action),
                );
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
    ) -> Option<Candidate> {
        if !view.view.in_view(me.x) {
            return None;
        }
        let target = view.target(plan.target)?; // gone or a new spawn serial
        self.support_candidate(view, set, me, target, plan.action)
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
        let kills = |damage: u8| set.kill_policy == KillPolicy::Never && target.hp <= damage;
        match (self.hero, action) {
            (HeroId::Donut, Action::Pounce) => {
                let d = &set.donut;
                let down = matches!(
                    target.state,
                    TargetState::Lying { .. } | TargetState::Staggered { .. }
                );
                // Q15: ahead of her, 30..80 px, and a pounce that cannot kill.
                (down
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
            }
            (HeroId::Carl, Action::Jab) => {
                let c = &set.carl;
                let hittable = matches!(
                    target.state,
                    TargetState::Standing | TargetState::Staggered { .. }
                );
                (hittable
                    && target.hp > 0
                    && !kills(CARL_JAB.damage)
                    && distance <= f32::from(c.engage_range_px))
                .then_some(Candidate {
                    action,
                    target: target.id,
                    utility: STRIKE_UTILITY - px(distance),
                    dx,
                    reach_min: f32::from(c.strike_min_px),
                    reach_max: f32::from(c.strike_max_px),
                })
            }
            _ => None,
        }
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
        for (tick, expect) in [(0, false), (3, false), (4, true), (20, true)] {
            let mut v = donut_ai(60.0);
            v.threats = vec![threat(0, 90.0, tick, Some(HeroId::Carl))];
            let (_, d) = donut().step_traced(&v, &assist());
            assert_eq!(d.pressed, expect, "tell tick {tick}");
        }
    }

    #[test]
    fn q14_the_swat_hit_tick_must_land_by_the_last_tell_tick() {
        // Startup 4, tell 24: a press at tell tick 21 hits on 25.
        let mut v = donut_ai(60.0);
        v.threats = vec![threat(0, 90.0, 21, Some(HeroId::Carl))];
        let (raw, d) = donut().step_traced(&v, &assist());
        assert_eq!(d.mode, Mode::Follow);
        assert!(!raw.attack);
        v.threats = vec![threat(0, 90.0, 20, Some(HeroId::Carl))];
        assert!(donut().step_traced(&v, &assist()).1.pressed);
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
        v.threats = vec![threat(0, 120.0, 2, Some(HeroId::Carl))];
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
    fn q17_the_missile_band_and_timing_gate_the_cast() {
        let fire = |x: f32, tick: u16| {
            let mut v = donut_ai(0.0);
            unlock(&mut v, 8, true);
            v.threats = vec![threat(0, x, tick, Some(HeroId::Carl))];
            donut().step_traced(&v, &assist()).1.pressed
        };
        assert!(fire(97.0, 0) && fire(160.0, 5));
        assert!(!fire(96.0, 0), "too close for a bolt");
        assert!(!fire(161.0, 0), "out of the band");
        assert!(!fire(120.0, 6), "too late in the tell to land by its end");
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
        let (pressed, dec) = pounce(110.0, 2, lying, 1, &assist());
        assert!(pressed);
        assert_eq!(
            (dec.mode, dec.action),
            (Mode::SupportAttack, Some(Action::Pounce))
        );
        assert!(pounce(110.0, 2, TargetState::Staggered { ticks: 5 }, 1, &assist()).0);
        // Range 30..80 ahead of her (Q15).
        assert!(pounce(90.0, 2, lying, 1, &assist()).0, "exactly 30 px");
        assert!(pounce(140.0, 2, lying, 1, &assist()).0, "exactly 80 px");
        assert!(!pounce(89.0, 2, lying, 1, &assist()).0);
        assert!(!pounce(141.0, 2, lying, 1, &assist()).0);
        // Behind her, standing, airborne: no.
        assert!(!pounce(110.0, 2, lying, -1, &assist()).0);
        assert!(!pounce(110.0, 2, TargetState::Standing, 1, &assist()).0);
        assert!(!pounce(110.0, 2, TargetState::Airborne, 1, &assist()).0);
        // Never lethal under `never`; unchanged under `factual`.
        assert!(!pounce(110.0, 1, lying, 1, &assist()).0);
        let mut factual = assist();
        factual.kill_policy = KillPolicy::Factual;
        assert!(pounce(110.0, 1, lying, 1, &factual).0);
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
        factual.kill_policy = KillPolicy::Factual;
        let (raw, d) = carl().step_traced(&v, &factual);
        assert!(raw.attack && d.mode == Mode::SupportAttack);
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
            assert_eq!(raw, RawInput::default());
            assert_eq!(d.veto, Some(Veto::NotCompanionDriven));
        }
        let mut v = donut_ai(60.0);
        v.heroes[1].defeated = true;
        let (raw, d) = donut().step_traced(&v, &assist());
        assert_eq!((raw, d.veto), (RawInput::default(), Some(Veto::HeroDown)));
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
        bail.ai_bailout = true;
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
        let set = assist();
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
        set.action_cadence_ticks = 1; // isolate the edge rule
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
                ..RawInput::default()
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
}
