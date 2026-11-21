//! COMPANION-AI-01 01A-wire: the combat-scene side of the companion.
//!
//! This child module of `combat_scene` is the only place that reads
//! `CombatState` and `SceneSim` for the companion (CA-23) and the only place
//! that writes anything for it: one full [`RawInput`] per companion-driven hero
//! through `step_party`'s first statement (CA-24). Everything is gated on
//! `UncontrolledHero::Companion` (the drive's `companion_set`): with the
//! companion off, [`CombatState::companion_inputs`] returns its input
//! unchanged, [`CompanionRuntime::mix_hash`] writes nothing and the glance is
//! the identity, so no golden changes (`companion_stub_parity_*`).
//!
//! Layout: [`CompanionRuntime`] holds the two directors (the per-hero
//! decision state), the AI-started move marks (CA-20) and the unhashed
//! counters and event log (CA-25); `companion_inputs` builds the pre-tick
//! [`CompanionView`] (read-only, never hashed, derived from hashed state) and
//! steps the directors; [`Glance`] is the kill-policy rule applied at the
//! strike sites (CA-19, CA-20).

use std::sync::OnceLock;

use super::CombatState;
use crate::combat::{CombatEvent, ENEMY_HALF_HEIGHT, MoveSpec};
use crate::companion::autonomy::{
    AutonomyBrief, AutonomyInput, AutonomyState, DriftCheck, FreeZone, ViewRows, ZoneInput,
};
use crate::companion::director::{Action, CompanionDirector, Decision, Mode};
use crate::companion::events::{CompanionEvent, CompanionStats, EVENT_CAP, RegroupOutcome};
use crate::companion::nav::{NavConfig, Reason};
use crate::companion::navfollow::{
    Body, NavDriver, NavNote, NavSpots, Regroup, RegroupView, arrive_eps_px,
};
use crate::companion::profile::{AutonomyRole, KillPolicy, ProfileFile, ProfileSet, ProfileSetId};
use crate::companion::rules;
use crate::companion::steer;
use crate::companion::view::{
    ActiveMove, CompanionView, Control, EntityKind, FollowInput, HeroView, MovePhase,
    StableEntityId, TargetState, TargetView, ThreatView, ViewBounds,
};
use crate::corridor::SceneSim;
use crate::feel::LLAMA_TELL_TICKS;
use crate::follower::Threat;
use crate::goblin::GoblinMode;
use crate::input::RawInput;
use crate::llama::LlamaMode;
use crate::party::HeroId;
use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, HeroMotion, Rect, TICK_SECONDS};

/// The profile file, parsed and hash-pinned once per process (CA-39).
fn profiles() -> &'static ProfileFile {
    static PROFILES: OnceLock<ProfileFile> = OnceLock::new();
    PROFILES.get_or_init(|| {
        ProfileFile::pinned().expect("the embedded companion_ai_v0.json is valid and pinned")
    })
}

/// Which counter a move instance number belongs to: Carl's melee runner, Donut's
/// swat/pounce state machine and Donut's casts each number their own instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkKind {
    CarlMove,
    DonutMove,
    DonutCast,
}

/// How many AI-started instance marks are kept (oldest dropped first). A move
/// lasts under 60 ticks and the AI starts at most one per 60, so a handful is
/// plenty; the cap keeps the hash bounded.
const MARK_CAP: usize = 8;

/// How many ticks after an injected press a move start still counts as that
/// press's (the move buffer and the cast buffer delay a start by up to 6).
const PRESS_WINDOW: u64 = 8;

/// A marker word the companion block of the state hash starts with ("COMPAN"),
/// written only while a hero is companion-driven.
const HASH_MARK: u64 = 0x434f_4d50_414e_0001;

/// A wait anchor must lie this many px inside the camera (01B-wire).
const NAV_BOUNDS_MARGIN: f32 = 24.0;
/// CF02-DUO-A15: how far from her `flat_span` looks each way.
const FLAT_SPAN_REACH_PX: f32 = 160.0;

/// A marker word of a live nav transit in the companion block ("NAV").
const NAV_MARK: u64 = 0x4e41_5600_0000_0001;

/// A marker word of an autonomy state in the companion block ("AUTO"), written
/// only for a hero that the autonomy layer steers (`cf02_free`).
const AUTO_MARK: u64 = 0x4155_5400_0000_0001;

/// The mode, action and target of a traced decision.
type Trace = (Mode, Option<Action>, Option<StableEntityId>);

#[derive(Debug, Clone)]
pub(crate) struct CompanionRuntime {
    /// One director per hero (Carl 0, Donut 1).
    directors: [CompanionDirector; 2],
    /// Which heroes the companion drove on the last stepped tick.
    driven: [bool; 2],
    /// The profile set of the last stepped tick (`None` while off).
    set: Option<ProfileSetId>,
    /// Move instances that were started by an injected press (CA-20). The flag
    /// survives a swap, so it is state of the run and hashed.
    marks: Vec<(MarkKind, u32)>,
    /// The tick of the director's last injected press per hero that no move
    /// has consumed yet. A move start inside [`PRESS_WINDOW`] ticks of it is the
    /// director's own (CA-20: "started by an injected press"); a move a human
    /// started is never marked, whoever drives the hero afterwards.
    pending_press: [Option<u64>; 2],
    /// The last Carl move instance the strike site saw (a new one is a start).
    last_carl_instance: u32,
    /// The tick about to run (set by `companion_inputs`).
    tick: u64,
    /// Unhashed counters and event log (CA-25, CA-31).
    stats: CompanionStats,
    events: Vec<CompanionEvent>,
    /// The last traced decision per hero, to log only changes.
    last_trace: [Option<Trace>; 2],
    /// 01B-wire: the nav graph of the level and the executor state of both
    /// heroes, built on the first companion-driven tick of an attempt from the
    /// scene's solids (derived state, so it is not hashed; the committed
    /// transits are, while live).
    nav: Option<NavDriver>,
    /// The graph could not be built for this level: the follow-and-jump bot
    /// stays in charge of movement (tried once per attempt).
    nav_failed: bool,
    /// COMPANION-AI-02 CA2-B2: the goal layer's state per hero (hashed only for
    /// a hero it steers).
    autonomy: [AutonomyState; 2],
    /// The heroes the autonomy layer steers: companion-driven, in a set with
    /// `autonomy.enabled` and a role assigned (CA2-Q01, CA2-Q02). Set by
    /// `companion_inputs`; also read by the presentation camera (the partner is
    /// a bound-only subject, CA2-Q25).
    autonomous: [bool; 2],
}

impl Default for CompanionRuntime {
    fn default() -> Self {
        Self {
            directors: [
                CompanionDirector::new(HeroId::Carl),
                CompanionDirector::new(HeroId::Donut),
            ],
            driven: [false; 2],
            set: None,
            marks: Vec::new(),
            pending_press: [None; 2],
            last_carl_instance: 0,
            tick: 0,
            stats: CompanionStats::default(),
            events: Vec::new(),
            last_trace: [None; 2],
            nav: None,
            nav_failed: false,
            autonomy: [AutonomyState::new(), AutonomyState::new()],
            autonomous: [false; 2],
        }
    }
}

impl CompanionRuntime {
    /// Attempt restart: the decision state, the marks and the drive flags start
    /// again (the cadence is measured in attempt-local ticks). The run's
    /// counters and event log are kept.
    pub(crate) fn reset(&mut self) {
        let stats = self.stats;
        let events = std::mem::take(&mut self.events);
        *self = Self::default();
        self.stats = stats;
        self.events = events;
    }

    /// Writes nothing while no hero is companion-driven, so every hash is
    /// unchanged; while one is, it writes a marker word first (as the party and
    /// boss blocks do) and then the profile set, both directors and the marks
    /// (CA-25).
    pub(crate) fn mix_hash(&self, mix: &mut impl FnMut(u64)) {
        let Some(set) = self.set.filter(|_| self.driven.iter().any(|d| *d)) else {
            return;
        };
        mix(HASH_MARK);
        mix(set as u64);
        mix(u64::from(self.driven[0]) | (u64::from(self.driven[1]) << 1));
        for director in &self.directors {
            director.hash_into(mix);
        }
        for press in self.pending_press {
            mix(press.map_or(0, |t| t + 1));
        }
        mix(u64::from(self.last_carl_instance));
        mix(self.marks.len() as u64);
        for &(kind, instance) in &self.marks {
            mix(kind as u64);
            mix(u64::from(instance));
        }
        // 01B-wire: a committed nav transit is state of the run; with none live
        // nothing is written, so a run that never leaves its anchor hashes as
        // before.
        if let Some(nav) = &self.nav {
            for (i, hero) in nav.heroes.iter().enumerate() {
                if hero.is_active() {
                    mix(NAV_MARK ^ i as u64);
                    hero.hash_into(mix);
                }
            }
        }
        // CA2-B2: the goal layer's state, only for a hero it steers, so no run
        // under another set writes a word.
        for (i, state) in self.autonomy.iter().enumerate() {
            if self.autonomous[i] {
                mix(AUTO_MARK ^ i as u64);
                state.hash_into(mix);
            }
        }
    }

    /// The kill policy of the running profile set (`never` while off).
    fn policy(&self) -> KillPolicy {
        self.set
            .map_or(KillPolicy::Never, |s| profiles().set(s).ca19_kill_policy)
    }

    /// A move of `hero` has just started (its first tick, `instance` of `kind`).
    /// It is AI-started (CA-20) only if the director injected a press for that
    /// hero inside [`PRESS_WINDOW`] ticks; the press is then consumed. A move the
    /// human started, and a start after the hero stopped being driven, is never
    /// marked.
    pub(crate) fn note_move_start(&mut self, hero: HeroId, kind: MarkKind, instance: u32) {
        let i = hero.index();
        let Some(pressed) = self.pending_press[i] else {
            return;
        };
        if self.tick.saturating_sub(pressed) > PRESS_WINDOW {
            self.pending_press[i] = None;
            return;
        }
        self.pending_press[i] = None;
        if self.marks.contains(&(kind, instance)) {
            return;
        }
        self.marks.push((kind, instance));
        if self.marks.len() > MARK_CAP {
            self.marks.remove(0);
        }
    }

    pub(crate) fn marked(&self, kind: MarkKind, instance: u32) -> bool {
        self.marks.contains(&(kind, instance))
    }

    /// The glance rule for hits dealt by `hero` in move `instance` (CA-20): the
    /// flag is whether the director's own press started that move instance, and
    /// it survives a swap. The strike site calls this on every tick of Carl's
    /// running move, so the first call with a new instance is its start; Donut's
    /// moves are noted where they start (`donut.rs`) and read through
    /// [`Self::glance_for`].
    pub(crate) fn strike_glance(&mut self, hero: HeroId, instance: u32) -> Glance {
        match hero {
            HeroId::Carl => {
                if instance != self.last_carl_instance {
                    self.last_carl_instance = instance;
                    self.note_move_start(HeroId::Carl, MarkKind::CarlMove, instance);
                }
                self.glance_for(MarkKind::CarlMove, instance)
            }
            HeroId::Donut => self.glance_for(MarkKind::DonutMove, instance),
        }
    }

    /// The glance for a move instance of `kind`.
    pub(crate) fn glance_for(&self, kind: MarkKind, instance: u32) -> Glance {
        Glance {
            ai_started: self.marked(kind, instance),
            policy: self.policy(),
        }
    }

    fn record(&mut self, tick: u64, hero: HeroId, decision: &Decision) {
        let i = hero.index();
        if decision.mode != Mode::Follow {
            self.stats.active_ticks[i] += 1;
        }
        if decision.pressed
            && let (Some(action), Some(target)) = (decision.action, decision.target)
        {
            let chain = action == Action::Haymaker;
            if chain {
                self.stats.chain_presses[i] += 1;
            } else {
                self.stats.assist_starts[i] += 1;
                if decision.mode == Mode::Interrupt {
                    self.stats.interrupt_starts[i] += 1;
                }
            }
            self.push_event(CompanionEvent::Press {
                tick,
                hero,
                action,
                target,
                chain,
            });
        }
        let trace = (decision.mode, decision.action, decision.target);
        if self.last_trace[i] != Some(trace) {
            self.last_trace[i] = Some(trace);
            self.push_event(CompanionEvent::Decision {
                tick,
                hero,
                mode: decision.mode,
                action: decision.action,
                target: decision.target,
            });
        }
    }

    fn push_event(&mut self, event: CompanionEvent) {
        self.events.push(event);
        if self.events.len() > EVENT_CAP {
            self.events.remove(0);
        }
    }
}

/// The kill-policy rule at a strike site (CA-19, CA-20), as a plain value the
/// site applies to the spec it is about to land.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Glance {
    /// The move was started by an injected press.
    ai_started: bool,
    policy: KillPolicy,
}

impl Default for Glance {
    /// A human-started hit: the identity.
    fn default() -> Self {
        Self {
            ai_started: false,
            policy: KillPolicy::Never,
        }
    }
}

impl Glance {
    /// The move as it lands on a target with `target_hp` on the hit tick.
    pub(crate) fn apply(&self, spec: &MoveSpec, target_hp: u8) -> MoveSpec {
        glance_hook(spec, target_hp, self.ai_started, self.policy)
    }

    /// The damage a boss hit lands: 0 when the rule glances it, else `damage`.
    pub(crate) fn lands(&self, damage: u16, target_hp: u16) -> u16 {
        let clamp = |v: u16| u8::try_from(v).unwrap_or(u8::MAX);
        if rules::is_glance(
            clamp(damage),
            clamp(target_hp),
            self.ai_started,
            self.policy,
        ) {
            0
        } else {
            damage
        }
    }
}

/// [`rules::glance`] for the strike sites: returns `*spec` for a human-started
/// hit, under `factual` and for a non-lethal hit.
pub(crate) fn glance_hook(
    spec: &MoveSpec,
    target_hp: u8,
    ai_started: bool,
    policy: KillPolicy,
) -> MoveSpec {
    rules::glance(*spec, target_hp, ai_started, policy)
}

impl CombatState {
    /// Which heroes the companion drove on the last stepped tick (the CINE feed
    /// of CA-29 reads it; `[false; 2]` while the companion is off).
    pub fn companion_driven(&self) -> [bool; 2] {
        self.companion.driven
    }

    /// Whether Carl's move `instance` is marked AI-started (tests).
    #[cfg(test)]
    pub(crate) fn companion_marked_carl(&self, instance: u32) -> bool {
        self.companion.marked(MarkKind::CarlMove, instance)
    }

    /// Whether Donut's swat or pounce `instance` is marked AI-started (tests).
    #[cfg(test)]
    pub(crate) fn companion_marked_donut_move(&self, instance: u32) -> bool {
        self.companion.marked(MarkKind::DonutMove, instance)
    }

    /// The heroes the autonomy layer steers this tick (`[false; 2]` unless the
    /// running set has `autonomy.enabled`) and the bound pad (CA2-Q25,
    /// `ca2_q25_bound_pad_px`): the presentation camera treats such a hero as a
    /// bound-only subject, a region of her camera body plus this pad that it
    /// keeps inside the view but never frames. `([false; 2], 0.0)` for every run
    /// that is not under `cf02_free`, so every existing camera series is
    /// unchanged.
    pub fn camera_bound_only(&self) -> ([bool; 2], f32) {
        let pad = self.companion.set.map_or(0.0, |set| {
            f32::from(profiles().set(set).autonomy.ca2_q25_bound_pad_px)
        });
        (self.companion.autonomous, pad)
    }

    /// The perception contract of the tick about to run, for the metrics
    /// harness (`cf02_free_fixtures::autonomy_metrics`): the same read-only view
    /// the companion's own step builds. `None` while no companion runs.
    #[doc(hidden)]
    pub fn companion_probe_view(&self, scene: &SceneSim) -> Option<CompanionView> {
        let set = self.companion.set?;
        Some(self.companion_view(scene, set))
    }

    /// The autonomy layer's state of `hero` (counters for the M16 report).
    pub fn autonomy_state(&self, hero: HeroId) -> &AutonomyState {
        &self.companion.autonomy[hero.index()]
    }

    /// The run's companion counters (never hashed).
    pub fn companion_stats(&self) -> CompanionStats {
        self.companion.stats
    }

    /// The companion events since the last call (a separate log, CA-25).
    pub fn take_companion_events(&mut self) -> Vec<CompanionEvent> {
        std::mem::take(&mut self.companion.events)
    }

    /// CA-24: replace the input of each companion-driven hero by the director's
    /// one full `RawInput` (`move_x`, `jump`, `attack`, `physical`; the pause,
    /// swap, ability and Down bits stay false). `inputs` is `[Carl, Donut]` as
    /// in `step_party`. The identity while no hero is companion-driven.
    pub(crate) fn companion_inputs(
        &mut self,
        scene: &mut SceneSim,
        inputs: [RawInput; 2],
    ) -> [RawInput; 2] {
        let drive = *scene.drive();
        self.companion.tick = scene.world.tick + 1;
        let sets = [
            drive.companion_set(HeroId::Carl),
            drive.companion_set(HeroId::Donut),
        ];
        self.companion.driven = [sets[0].is_some(), sets[1].is_some()];
        let Some(set_id) = sets[0].or(sets[1]) else {
            // Off (or two players): nothing runs, the plans are forgotten and
            // the scene's legacy regroup rule is in charge again.
            self.companion.set = None;
            self.companion.pending_press = [None; 2];
            self.companion.nav = None;
            self.companion.nav_failed = false;
            self.companion.autonomous = [false; 2];
            scene.set_nav_regroup([false; 2]);
            for director in &mut self.companion.directors {
                director.clear_plan();
            }
            return inputs;
        };
        self.companion.set = Some(set_id);
        let set = profiles().set(set_id);
        // 01B-wire: the nav graph follows the solids, the off-screen regroup is
        // decided before the follow layer is read (it moves the hero), then the
        // view and the nav-driven follow are built from the placed bodies.
        self.companion_nav_sync(scene);
        let owned = self.companion_regroup(scene);
        scene.set_nav_regroup(owned);
        let mut view = self.companion_view(scene, set_id);
        // CA2-B2: the heroes the goal layer steers (never a hero of a set without
        // `autonomy.enabled`, so every other set runs the code below unchanged).
        let roles = [set.carl.autonomy_role, set.donut.autonomy_role];
        for hero in HeroId::ALL {
            let i = hero.index();
            self.companion.autonomous[i] =
                set.autonomy.enabled && self.companion.driven[i] && roles[i] != AutonomyRole::None;
        }
        self.companion_nav_follow(scene, &mut view);
        let mut briefs: [Option<AutonomyBrief>; 2] = [None, None];
        let mut exempt = [false; 2];
        for hero in HeroId::ALL {
            let i = hero.index();
            if self.companion.autonomous[i] {
                (briefs[i], exempt[i]) = self.companion_autonomy(scene, &mut view, set, hero);
            }
        }
        let mut out = inputs;
        for hero in HeroId::ALL {
            let i = hero.index();
            if !self.companion.driven[i] {
                self.companion.directors[i].clear_plan();
                // A press the director made but no move consumed does not
                // outlive its control.
                self.companion.pending_press[i] = None;
                if let Some(nav) = &mut self.companion.nav {
                    nav.heroes[i] = Default::default();
                }
                self.companion.autonomy[i].clear_plan();
                continue;
            }
            let brief = briefs[i].as_ref();
            let (raw, decision) =
                self.companion.directors[i].step_briefed(&view, set, brief.map(|b| &b.director));
            let mut move_x = raw.move_x;
            if brief.is_some() && !exempt[i] {
                // Goal steering and the interrupt approach stay inside `Z`; a dodge
                // and a committed transit are exempt (spec 5.2, review F-9).
                let run = self.run_px_per_tick(scene, hero);
                let zone = Self::zone_of(scene, &view, set, hero);
                move_x = steer::clamp_to_zone(view.hero(hero).x, move_x, &zone, run);
            }
            if brief.is_some() {
                // M16: the director's drift skips (CA2-B1 review: a caller owes this).
                self.companion.autonomy[i].record_decision(&decision);
            }
            // A press tick stands still: a direction held with a press would
            // turn the hero first (the Carl rule), maybe away from its target.
            let move_x = if decision.pressed { 0.0 } else { move_x };
            out[i] = RawInput {
                move_x,
                jump: raw.jump,
                attack: raw.attack,
                physical: raw.physical,
                ..RawInput::NONE
            };
            if decision.pressed {
                self.companion.pending_press[i] = Some(view.tick);
            }
            self.companion.record(view.tick, hero, &decision);
        }
        out
    }

    /// Her run speed per tick (the body's own motion; the jump boost does not
    /// change it).
    fn run_px_per_tick(&self, scene: &SceneSim, hero: HeroId) -> f32 {
        Self::motion_of(scene, hero).run_speed * TICK_SECONDS
    }

    /// The body motion `hero` steps with this tick.
    fn motion_of(scene: &SceneSim, hero: HeroId) -> HeroMotion {
        match hero {
            HeroId::Carl => HeroMotion::CARL,
            HeroId::Donut => scene.donut_motion_now(),
        }
    }

    /// The free zone of this tick (spec 3.1) for the partner `hero`.
    fn zone_of(scene: &SceneSim, view: &CompanionView, set: &ProfileSet, hero: HeroId) -> FreeZone {
        let leader = view.hero(hero.other());
        FreeZone::compute(
            &ZoneInput::new(
                leader,
                view.view,
                ViewRows::of_camera(&scene.camera.config()),
            ),
            &set.autonomy,
        )
    }

    /// Whether one of `hero`'s damaging moves connected on the previous tick (the
    /// Show-off trigger, spec 5.1): read from the combat event log, which holds
    /// the events of the tick that just ended.
    fn partner_hit_landed(&self, scene: &SceneSim, hero: HeroId) -> bool {
        let last = scene.world.tick;
        // The newest events only: a tick logs a handful.
        self.events.iter().rev().take(48).any(|e| {
            matches!(e, CombatEvent::Hit { tick, move_id, damage, .. }
                    if *tick == last
                        && *damage > 0
                        && move_id.hero() as usize == hero.index())
        })
    }

    /// CA2-B2: one tick of the goal layer for the partner `hero`: decide (the
    /// brief), then steer (the nav go-to for a route that needs a jump or a drop,
    /// the flat walk otherwise, the dodge on top) and write the result as the
    /// director's follow input. Returns the brief (`None`: no steering this tick,
    /// the director runs without one) and whether this tick's movement is exempt
    /// from the zone clamp (a dodge or a committed transit).
    fn companion_autonomy(
        &mut self,
        scene: &SceneSim,
        view: &mut CompanionView,
        set: &ProfileSet,
        hero: HeroId,
    ) -> (Option<AutonomyBrief>, bool) {
        let i = hero.index();
        let Some(mut nav) = self.companion.nav.take() else {
            return (None, false);
        };
        let a = &set.autonomy;
        // CA2 Revision 3, DP-Q38: no jump or hop edge from a surface above the
        // apex-aware rise limit (her effective apex, the boost included),
        // rechecked every tick against the leader's feet.
        let apex = crate::agility::discrete_apex_px(Self::motion_of(scene, hero).jump_speed);
        nav.set_jump_ceiling(Some(
            Self::nav_body(scene, hero.other()).foot_y + steer::jump_edge_rise_limit_px(apex),
        ));
        let zone = Self::zone_of(scene, view, set, hero);
        let drift = DriftCheck::new(Self::motion_of(scene, hero.other()), a);
        let hit = self.partner_hit_landed(scene, hero);
        let brief = {
            let world = NavSpots::new(&nav, hero, f32::from(a.ca2_q29_ledge_inset_px));
            self.companion.autonomy[i].step(&AutonomyInput {
                view,
                set,
                partner: hero,
                zone: &zone,
                drift: &drift,
                partner_motion: Self::motion_of(scene, hero),
                tuning: &crate::donut_moves::DonutMoveTuning::SPEC,
                world: &world,
                pan_remaining: scene.camera_pan_remaining(),
                partner_hit_landed: hit,
            })
        };
        let Some(brief) = brief else {
            self.companion.nav = Some(nav);
            return (None, false);
        };
        let tick = self.companion.tick;
        if let Some(reason) = brief.changed {
            self.companion.push_event(CompanionEvent::GoalChanged {
                tick,
                hero,
                goal: brief.goal,
                reason,
            });
        }
        let mut exempt = false;
        if Self::follow_free_view(&view.heroes[i], &self.hero_state(hero)) {
            let me = Self::nav_body(scene, hero);
            let goal_body = brief.spot.map_or(me, |s| Body {
                x: s.x,
                foot_y: s.surface_y,
                grounded: true,
            });
            let bounds = Some((
                view.view.left + NAV_BOUNDS_MARGIN,
                view.view.right - NAV_BOUNDS_MARGIN,
            ));
            let (drive, notes) = nav.go_to(hero, me, goal_body, bounds);
            let run = self.run_px_per_tick(scene, hero);
            let mut input = match drive {
                Some(d) => {
                    exempt = nav.heroes[i].is_active();
                    FollowInput {
                        move_x: d.move_x,
                        jump: d.jump,
                    }
                }
                None => FollowInput {
                    move_x: steer::flat_steer(
                        &view.heroes[i],
                        &brief,
                        run,
                        arrive_eps_px(Self::motion_of(scene, hero).run_speed),
                    ),
                    jump: false,
                },
            };
            let active = match hero {
                HeroId::Carl => set.carl.ca21_active,
                HeroId::Donut => set.donut.ca21_active,
            };
            let threats = self.follow_threats(hero, active);
            let dodge = scene.dodge_input(hero, &threats, view.boss.as_ref());
            if dodge.jump || dodge.move_x != 0.0 {
                input = FollowInput {
                    move_x: dodge.move_x,
                    jump: dodge.jump,
                };
                exempt = true;
            }
            view.follow[i] = input;
            self.push_nav_notes(hero, notes);
        } else {
            view.follow[i] = FollowInput::default();
        }
        self.companion.nav = Some(nav);
        (Some(brief), exempt)
    }

    /// 01B-wire: build the nav graph from the scene's solids on the first
    /// companion-driven tick of an attempt, and bring it in line with a
    /// destruction or restoration since.
    fn companion_nav_sync(&mut self, scene: &SceneSim) {
        if self.companion.nav_failed {
            return;
        }
        let solids = &scene.world.solids;
        // DP-B: the physics Donut has now (the boosted launch while held); the
        // graph is rebuilt on the tick it changes and never otherwise.
        let config = NavConfig::for_motion(scene.drive().tuning.donut_motion)
            .with_donut_launch(scene.donut_motion_now().jump_speed);
        let ok = match &mut self.companion.nav {
            Some(nav) => {
                // The apex-aware jump ceiling is the autonomy layer's, set per
                // tick for the partner it steers; every other tick has none.
                nav.set_jump_ceiling(None);
                nav.set_config(config, solids).is_ok() && nav.sync(solids).is_ok()
            }
            None => match NavDriver::new(solids, config) {
                Ok(nav) => {
                    self.companion.nav = Some(nav);
                    true
                }
                Err(_) => false,
            },
        };
        if !ok {
            self.companion.nav = None;
            self.companion.nav_failed = true;
        }
    }

    /// The scene's body of `id` as the nav executor sees it.
    fn nav_body(scene: &SceneSim, id: HeroId) -> Body {
        let body = scene.world.body(id);
        Body {
            x: body.x,
            foot_y: body.y - HERO_HALF_HEIGHT,
            grounded: body.grounded,
        }
    }

    /// 01B-wire: the camera-safe regroup of every companion-driven hero that has
    /// been off-screen long enough (the legacy `regroup_ticks` rule), before the
    /// follow layer is read. Returns the heroes whose regroup the nav owns this
    /// tick (all driven ones, except a hero standing on no known anchor, where
    /// the legacy rule stays the escape hatch).
    fn companion_regroup(&mut self, scene: &mut SceneSim) -> [bool; 2] {
        let mut owned = [false; 2];
        let Some(nav) = self.companion.nav.as_ref() else {
            return owned;
        };
        let left = scene.camera.left() as f32;
        let right = left + scene.camera.config().view_w as f32;
        let mut outcomes = Vec::new();
        for hero in HeroId::ALL {
            let i = hero.index();
            if !self.companion.driven[i] {
                continue;
            }
            owned[i] = true;
            if !scene.regroup_due(hero) {
                continue;
            }
            let leader = hero.other();
            let behind = if scene.world.body(leader).facing >= 0 {
                -1.0
            } else {
                1.0
            };
            let view = RegroupView {
                view_left: left,
                view_right: right,
                behind,
            };
            outcomes.push((
                hero,
                nav.regroup(
                    hero,
                    Self::nav_body(scene, hero),
                    Self::nav_body(scene, leader),
                    &view,
                ),
            ));
        }
        let tick = self.companion.tick;
        for (hero, regroup) in outcomes {
            let i = hero.index();
            let outcome = match regroup {
                Regroup::Place { anchor, x, foot_y } => {
                    if scene.regroup_place(hero, x, foot_y) {
                        if let Some(nav) = &mut self.companion.nav {
                            nav.heroes[i] = Default::default();
                        }
                        self.companion.stats.regroups[i] += 1;
                        RegroupOutcome::Placed {
                            anchor,
                            x: x.round() as i32,
                        }
                    } else {
                        // A body does not fit there: stay, count again.
                        scene.regroup_skip(hero);
                        self.companion.stats.regroups_denied[i] += 1;
                        RegroupOutcome::Denied(Reason::Obstructed { by: anchor.solid })
                    }
                }
                Regroup::Legacy => {
                    owned[i] = false;
                    RegroupOutcome::Legacy
                }
                Regroup::Denied(reason) => {
                    scene.regroup_skip(hero);
                    self.companion.stats.regroups_denied[i] += 1;
                    RegroupOutcome::Denied(reason)
                }
                // Cannot be judged while the leader is in the air: no event,
                // no placement, and the off-screen counter is held at its due
                // value, so the regroup is asked again on the very next tick.
                Regroup::Defer => {
                    scene.regroup_hold(hero);
                    continue;
                }
            };
            self.companion.push_event(CompanionEvent::Regroup {
                tick,
                hero,
                outcome,
            });
        }
        owned
    }

    /// 01B-wire: replace the follow layer of a free companion-driven hero by the
    /// nav-driven one where the nav has something to add (a route that needs a
    /// jump or a drop, or no route at all). The follow-and-jump bot's dodges
    /// stay on top (a goblin tell, a boss pound or charge).
    fn companion_nav_follow(&mut self, scene: &SceneSim, view: &mut CompanionView) {
        let Some(mut nav) = self.companion.nav.take() else {
            return;
        };
        let profile = profiles().set(view.profile_set);
        let bounds = Some((
            view.view.left + NAV_BOUNDS_MARGIN,
            view.view.right - NAV_BOUNDS_MARGIN,
        ));
        let mut notes: Vec<(HeroId, NavNote)> = Vec::new();
        for hero in HeroId::ALL {
            let i = hero.index();
            // CA2-B2: a hero the goal layer steers has its own go-to executor.
            if !self.companion.driven[i]
                || self.companion.autonomous[i]
                || !Self::follow_free_view(&view.heroes[i], &self.hero_state(hero))
            {
                continue;
            }
            let me = Self::nav_body(scene, hero);
            let leader = Self::nav_body(scene, hero.other());
            let (drive, tick_notes) = nav.follow(hero, me, leader, bounds);
            notes.extend(tick_notes.into_iter().map(|n| (hero, n)));
            let Some(drive) = drive else { continue };
            // A route that needs a jump or a drop is the nav's: no flank
            // (CF02-DUO-A15 never overrides a transit).
            if hero == HeroId::Donut {
                view.flank_span = None;
            }
            let active = match hero {
                HeroId::Carl => profile.carl.ca21_active,
                HeroId::Donut => profile.donut.ca21_active,
            };
            let threats = self.follow_threats(hero, active);
            let dodge = scene.dodge_input(hero, &threats, view.boss.as_ref());
            view.follow[i] = if dodge.jump || dodge.move_x != 0.0 {
                FollowInput {
                    move_x: dodge.move_x,
                    jump: dodge.jump,
                }
            } else {
                FollowInput {
                    move_x: drive.move_x,
                    jump: drive.jump,
                }
            };
        }
        self.companion.nav = Some(nav);
        for (hero, note) in notes {
            self.push_nav_notes(hero, [note]);
        }
    }

    /// Count and log the nav executor's notes of `hero` (01B-wire).
    fn push_nav_notes(&mut self, hero: HeroId, notes: impl IntoIterator<Item = NavNote>) {
        let i = hero.index();
        let tick = self.companion.tick;
        for note in notes {
            match note {
                NavNote::Begin { .. } => self.companion.stats.nav_transits[i] += 1,
                NavNote::Arrived { .. } => self.companion.stats.nav_arrivals[i] += 1,
                NavNote::Waiting { .. } => self.companion.stats.nav_waits[i] += 1,
                NavNote::Abandoned | NavNote::Stranded { .. } => {}
            }
            self.companion
                .push_event(CompanionEvent::Nav { tick, hero, note });
        }
    }

    /// The perception contract of the tick about to run (CA-23): built from
    /// `&self` and `&SceneSim` only, derived from hashed state, never written
    /// back and never hashed. `in_view` is the simulation scroll camera, never
    /// the presentation camera (CF02-Q40).
    fn companion_view(&self, scene: &SceneSim, profile_set: ProfileSetId) -> CompanionView {
        let drive = *scene.drive();
        let left = scene.camera.left() as f32;
        let view = ViewBounds {
            left,
            right: left + scene.camera.config().view_w as f32,
        };
        let live = scene.party_live();
        let heroes = [HeroId::Carl, HeroId::Donut].map(|id| self.companion_hero(scene, id));

        // The follow layer's input: the follow-and-jump bot on the same goblin
        // threats `step_party` builds. An ACTIVE companion dodges only the
        // swings aimed at itself: a tell aimed at the other hero is the one it
        // may interrupt, and the bot's dodge jump would take it off the ground
        // (and out of reach of its own swat) before the swat can start. A role
        // that cannot act (the passive Carl of `cf02_assist`, CF02-Q13) keeps the
        // plain bot's dodge of every threat.
        let profile = profiles().set(profile_set);
        let boss = self.boss_threat();
        let mut follow = [FollowInput::default(); 2];
        for id in HeroId::ALL {
            let i = id.index();
            let h = &heroes[i];
            let active = match id {
                HeroId::Carl => profile.carl.ca21_active,
                HeroId::Donut => profile.donut.ca21_active,
            };
            let free = Self::follow_free_view(h, &self.hero_state(id));
            if free {
                let threats = self.follow_threats(id, active);
                let raw = scene.follow_input(id, &threats, boss.as_ref());
                follow[i] = FollowInput {
                    move_x: raw.move_x,
                    jump: raw.jump,
                };
            }
        }

        // CF02-DUO-A15: Donut's flat standing run, only where the flank is on.
        let flank_span = (profile.cf02_duo.cf02_duo_a15_flank
            && matches!(heroes[HeroId::Donut.index()].control, Control::Companion))
        .then(|| Self::flat_span(scene, HeroId::Donut))
        .flatten();

        CompanionView {
            tick: scene.world.tick + 1,
            solo: live && !drive.two_player,
            cf02_rules: self.cf02.is_some(),
            profile_set,
            view,
            heroes,
            follow,
            threats: self.companion_threats(view),
            targets: self.companion_targets(view, scene.world.tick + 1),
            boss,
            flank_span,
        }
    }

    /// CF02-DUO-A15: the x interval around `id` that is flat ground at its own
    /// foot level, inset by the body half width at both ends (so its body never
    /// overhangs an edge), and free of walls over the body height. Sampled
    /// every 4 px out to `FLAT_SPAN_REACH_PX`; `None` when the hero is
    /// airborne or does not stand on a flat surface. Pure on the solids.
    fn flat_span(scene: &SceneSim, id: HeroId) -> Option<(f32, f32)> {
        const STEP_PX: f32 = 4.0;
        const FLAT_TOLERANCE_PX: f32 = 2.0;
        let body = scene.world.body(id);
        if !body.grounded {
            return None;
        }
        let foot_y = body.y - HERO_HALF_HEIGHT;
        let solids = &scene.world.solids;
        let stands = |x: f32| {
            solids.iter().any(|b| {
                b.intact
                    && b.rect.left <= x
                    && x <= b.rect.right()
                    && (b.rect.top() - foot_y).abs() <= FLAT_TOLERANCE_PX
            })
        };
        let free = |x: f32| {
            let probe = Rect {
                left: x - HERO_HALF_WIDTH,
                bottom: foot_y + FLAT_TOLERANCE_PX + 1.0,
                width: 2.0 * HERO_HALF_WIDTH,
                height: 2.0 * HERO_HALF_HEIGHT,
            };
            solids.iter().all(|b| !b.intact || !probe.overlaps(b.rect))
        };
        let ok = |x: f32| {
            stands(x - HERO_HALF_WIDTH) && stands(x + HERO_HALF_WIDTH) && stands(x) && free(x)
        };
        if !ok(body.x) {
            return None;
        }
        let reach = |dir: f32| {
            let mut x = body.x;
            for _ in 0..(FLAT_SPAN_REACH_PX / STEP_PX) as usize {
                let next = x + dir * STEP_PX;
                if !ok(next) {
                    break;
                }
                x = next;
            }
            x
        };
        Some((reach(-1.0), reach(1.0)))
    }

    /// May the follow layer (the bot's input, or the nav-driven one) act for
    /// this hero this tick? Mirrors the bot's `can_move` gate.
    fn follow_free_view(h: &HeroView, state: &super::HeroCombat) -> bool {
        h.control == Control::Companion
            && !h.defeated
            && h.hurt_ticks == 0
            && h.active_move.is_none()
            && state.hit_stop == 0
    }

    /// The goblin tells the follow layer of `id` dodges. An ACTIVE companion
    /// dodges only the swings aimed at itself: a tell aimed at the other hero is
    /// the one it may interrupt, and the bot's dodge jump would take it off the
    /// ground (and out of reach of its own swat) before the swat can start. A
    /// role that cannot act (the passive Carl of `cf02_assist`, CF02-Q13) keeps
    /// the plain bot's dodge of every threat.
    fn follow_threats(&self, id: HeroId, active: bool) -> Vec<Threat> {
        self.enemies
            .iter()
            .enumerate()
            .filter(|(index, g)| g.dodge_threat() && (!active || self.target_of(*index) == id))
            .map(|(_, g)| Threat {
                x: g.x,
                attack_tick: g.attack_tick,
            })
            .collect()
    }

    fn companion_hero(&self, scene: &SceneSim, id: HeroId) -> HeroView {
        let drive = scene.drive();
        let body = scene.world.body(id);
        let state = self.hero_state(id);
        let defeated = self.hero_defeated;
        let frozen = state.hit_stop > 0;
        let (active_move, mana, abilities) = match id {
            HeroId::Carl => (
                body.melee.current().map(|m| ActiveMove {
                    id: m.spec.id,
                    phase: match m.phase {
                        crate::combat::Phase::Startup => MovePhase::Startup,
                        crate::combat::Phase::Active => MovePhase::Active,
                        crate::combat::Phase::Recovery => MovePhase::Recovery,
                    },
                    ticks_left: m.spec.total().saturating_sub(m.tick),
                    ai_started: self.companion.marked(MarkKind::CarlMove, m.instance),
                }),
                None,
                true,
            ),
            HeroId::Donut => {
                let kit = self.donut_companion_state(scene);
                (kit.running, Some(kit.mana), kit.abilities_live)
            }
        };
        let control = if drive.companion_set(id).is_some() {
            Control::Companion
        } else if drive.is_bot(id) {
            Control::Bot
        } else {
            Control::Human(if drive.two_player {
                id.index() as u8
            } else {
                0
            })
        };
        HeroView {
            id,
            x: body.x,
            foot_y: body.y - HERO_HALF_HEIGHT,
            vx: body.vx,
            vy: body.vy,
            facing: body.facing,
            grounded: body.grounded,
            health: body.health,
            hurt_ticks: state.hurt_ticks,
            invulnerable_ticks: body.invulnerable_ticks,
            defeated,
            control,
            active_move,
            // CA-37: no air moves, so a press needs the ground (the director
            // only presses while `can_start_move`).
            can_start_move: abilities
                && !defeated
                && state.hurt_ticks == 0
                && !frozen
                && body.grounded
                && active_move.is_none(),
            mana,
        }
    }

    /// Enemy attacks in their tell, goblins then llamas (Q18 only), by id.
    fn companion_threats(&self, view: ViewBounds) -> Vec<ThreatView> {
        let mut threats = Vec::new();
        for (index, g) in self.enemies.iter().enumerate() {
            // ENG-G-Q22: a goblin in its tell, or the Engineer in his rear and
            // point (his `tell_len` is the attack tick on which his launch begins).
            let Some((tell_tick, tell_len, impact_tick)) = g.tell_view() else {
                continue;
            };
            threats.push(ThreatView {
                id: StableEntityId::goblin(index as u32),
                x: g.x,
                foot_y: g.y - ENEMY_HALF_HEIGHT,
                facing: g.facing,
                tell_tick,
                tell_len,
                impact_tick,
                aimed_at: Some(self.target_of(index)),
                guarded: g.guard.is_some(),
                in_view: view.in_view(g.x),
            });
        }
        if self.llama_targets_live().is_some() {
            for (index, l) in self.llamas.iter().enumerate() {
                if !l.telling() || !l.alive() {
                    continue;
                }
                threats.push(ThreatView {
                    id: llama_id(index),
                    x: l.x,
                    foot_y: l.y - crate::llama::LLAMA_HALF_HEIGHT,
                    facing: l.facing,
                    tell_tick: l.attack_tick,
                    tell_len: LLAMA_TELL_TICKS,
                    impact_tick: LLAMA_TELL_TICKS,
                    // A llama always spits at Carl (`step_llamas`).
                    aimed_at: Some(HeroId::Carl),
                    guarded: l.guard.is_some(),
                    in_view: view.in_view(l.x),
                });
            }
        }
        threats
    }

    /// Ticks from Carl's last hit on target slot `slot` (goblins, then llamas)
    /// to `now`, saturating (CF02-DUO-A12).
    fn carl_hit_age(&self, slot: usize, now: u64) -> Option<u16> {
        let at = (*self.carl_hit_tick.get(slot)?)?;
        Some(now.saturating_sub(at).min(u64::from(u16::MAX)) as u16)
    }

    /// The living enemies a support move could hit, goblins then llamas (Q18
    /// only), by id.
    fn companion_targets(&self, view: ViewBounds, now: u64) -> Vec<TargetView> {
        let mut targets = Vec::new();
        for (index, g) in self.enemies.iter().enumerate() {
            if !g.alive() {
                continue;
            }
            let state = match g.mode {
                GoblinMode::Neutral | GoblinMode::Attack => TargetState::Standing,
                GoblinMode::Hurt => TargetState::Staggered {
                    ticks: g.stagger_left,
                },
                // Only a goblin that lies under `cf02_rules` is a lying target
                // (Q15, Q38); without them a downed goblin is not one (the
                // pounce would stand it up).
                GoblinMode::Downed if g.lying() => TargetState::Lying {
                    down_ticks_left: g.down_ticks_left().unwrap_or(0),
                    hold_used: g.hold_used,
                },
                GoblinMode::Downed | GoblinMode::Launched | GoblinMode::Airborne => {
                    TargetState::Airborne
                }
                GoblinMode::Down | GoblinMode::Corpse => continue,
            };
            targets.push(TargetView {
                id: StableEntityId::goblin(index as u32),
                x: g.x,
                foot_y: g.y - ENEMY_HALF_HEIGHT,
                hp: g.health,
                state,
                guarded: g.guard.is_some(),
                in_view: view.in_view(g.x),
                carl_hit: g.hit_by == Some(HeroId::Carl.index() as u32),
                carl_hit_age: self.carl_hit_age(index, now),
            });
        }
        if self.llama_targets_live().is_some() {
            for (index, l) in self.llamas.iter().enumerate() {
                if !l.alive() {
                    continue;
                }
                let state = match l.mode {
                    LlamaMode::Neutral | LlamaMode::Attack => TargetState::Standing,
                    LlamaMode::Hurt => TargetState::Staggered {
                        ticks: l.stagger_left,
                    },
                    LlamaMode::Down | LlamaMode::Corpse => continue,
                };
                targets.push(TargetView {
                    id: llama_id(index),
                    x: l.x,
                    foot_y: l.y - crate::llama::LLAMA_HALF_HEIGHT,
                    hp: l.health,
                    state,
                    guarded: l.guard.is_some(),
                    in_view: view.in_view(l.x),
                    carl_hit: l.hit_by == Some(HeroId::Carl.index() as u32),
                    carl_hit_age: self.carl_hit_age(self.enemies.len() + index, now),
                });
            }
        }
        targets
    }
}

/// A llama's stable id (serial 0: no respawn in 01A fixtures, CA-28).
fn llama_id(index: usize) -> StableEntityId {
    StableEntityId {
        kind: EntityKind::Llama,
        index: index as u32,
        spawn_serial: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{CARL_HAYMAKER, CARL_JAB, CARL_PUSH_KICK};
    use crate::combat_scene::{FIXTURES, FixtureSim};
    use crate::env_kit::EnvKit;
    use std::path::PathBuf;

    #[test]
    fn companion_stub_parity_mix_hash_writes_nothing() {
        let mut runtime = CompanionRuntime::default();
        let mut calls = 0u32;
        runtime.mix_hash(&mut |_| calls += 1);
        runtime.reset();
        runtime.mix_hash(&mut |_| calls += 1);
        assert_eq!(calls, 0, "the stub mixes nothing into any hash");
    }

    #[test]
    fn companion_stub_parity_glance_hook_is_identity() {
        // A human-started hit is the identity under every policy and HP; so is
        // the default glance and the runtime's glance while the companion is off.
        for spec in [CARL_JAB, CARL_HAYMAKER, CARL_PUSH_KICK] {
            for hp in [1, u8::MAX] {
                for policy in [KillPolicy::Never, KillPolicy::Factual] {
                    assert_eq!(glance_hook(&spec, hp, false, policy), spec);
                }
                // An AI-started hit is the identity when it is not lethal, and
                // under `factual`.
                assert_eq!(glance_hook(&spec, hp, true, KillPolicy::Factual), spec);
                assert_eq!(Glance::default().apply(&spec, hp), spec);
            }
        }
        let mut runtime = CompanionRuntime::default();
        let glance = runtime.strike_glance(HeroId::Carl, 7);
        assert_eq!(glance.apply(&CARL_JAB, 1), CARL_JAB);
        assert!(
            !runtime.marked(MarkKind::CarlMove, 7),
            "off: nothing marked"
        );
        // A non-lethal AI-started hit is unchanged under `never` (review F-3).
        for spec in [CARL_JAB, CARL_HAYMAKER, CARL_PUSH_KICK] {
            assert_eq!(glance_hook(&spec, u8::MAX, true, KillPolicy::Never), spec);
        }
        // The one change the wiring makes: under `never`, an AI-started lethal
        // hit lands as a glance (damage 0, same reaction).
        let landed = glance_hook(&CARL_JAB, 1, true, KillPolicy::Never);
        assert_eq!(landed.damage, 0);
        assert_eq!(
            MoveSpec {
                damage: CARL_JAB.damage,
                ..landed
            },
            CARL_JAB
        );
    }

    #[test]
    fn companion_stub_parity_inputs_pass_through() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let kit = EnvKit::load(&root, "assets/levels/floor01/corridor_abc_sample.json").unwrap();
        let mut sim = FixtureSim::new(&kit, 360, FIXTURES[0]).unwrap();
        let samples = [
            [RawInput::NONE, RawInput::NONE],
            [
                RawInput {
                    move_x: 0.6,
                    attack: true,
                    ..RawInput::NONE
                },
                RawInput {
                    physical: true,
                    jump: true,
                    swap: true,
                    ..RawInput::NONE
                },
            ],
        ];
        for inputs in samples {
            assert_eq!(sim.combat.companion_inputs(&mut sim.scene, inputs), inputs);
        }
        // Restarting calls the stub reset and leaves the state hash alone.
        let before = sim.combat.state_hash(&sim.scene);
        sim.combat.companion.reset();
        assert_eq!(sim.combat.state_hash(&sim.scene), before);
    }

    fn kit() -> EnvKit {
        EnvKit::load(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            "assets/levels/floor01/corridor_abc_sample.json",
        )
        .unwrap()
    }

    /// CF02-Q40: the AI starts nothing while its target is outside the simulation
    /// view, and starts it once the target is inside (the camera mode never enters).
    #[test]
    fn q40_nothing_starts_outside_the_simulation_view() {
        let mut sim = crate::party_scene::PartyFixtureSim::new(
            &kit(),
            360,
            &crate::companion_fixtures::CF02_ASSIST_SWAT_INTERRUPT,
        )
        .unwrap();
        sim.step_slots([crate::input::RawInput::NONE; 2]);
        let right = (sim.scene.camera.left() + sim.scene.camera.config().view_w) as f32;
        // The view bounds the AI reads are exactly the scene's own off-screen rule.
        for x in [right - 1.0, right, right + 0.5, 0.0, right + 100.0] {
            assert_eq!(
                x <= right && x >= sim.scene.camera.left() as f32,
                !sim.scene.is_offscreen(x, 0.0)
            );
        }
        sim.scene.world.hero.x = right - 20.0;
        sim.scene.world.donut_body.x = right - 50.0;
        sim.combat.targets[0] = Some(HeroId::Carl);
        let place = |sim: &mut crate::party_scene::PartyFixtureSim, x: f32| {
            let g = &mut sim.combat.enemies[0];
            g.x = x;
            g.mode = crate::goblin::GoblinMode::Attack;
            g.attack_tick = 8;
            g.facing = -1;
        };
        let mut pressed_out = false;
        for _ in 0..8 {
            place(&mut sim, right + 3.0);
            let inputs = sim
                .combat
                .companion_inputs(&mut sim.scene, [crate::input::RawInput::NONE; 2]);
            pressed_out |= inputs[1].attack;
            sim.combat.take_companion_events();
        }
        assert!(!pressed_out, "outside the view: no swat");
        let mut pressed_in = false;
        for _ in 0..12 {
            place(&mut sim, right - 28.0);
            let inputs = sim
                .combat
                .companion_inputs(&mut sim.scene, [crate::input::RawInput::NONE; 2]);
            pressed_in |= inputs[1].attack;
        }
        assert!(pressed_in, "inside the view: the swat starts");
    }

    /// CF02-DUO-A15: `flat_span` is the flat ground the flank may use. On the
    /// arena floor it is a run around her; on the left perch slab (x 544 to
    /// 672) it ends a body half width inside both slab edges, so the flank never
    /// walks her off a ledge, and it is `None` in the air.
    #[test]
    fn a15_flat_span_ends_a_body_half_width_inside_a_ledge() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let arena_kit = crate::cf02::arena::load_arena(&root).unwrap();
        let fixture = crate::cf02_fixtures::find_party_fixture("cf02_arena_direct").unwrap();
        let mut sim = crate::party_scene::PartyFixtureSim::new(&arena_kit, 360, fixture).unwrap();
        for _ in 0..4 {
            sim.step_slots([RawInput::NONE; 2]);
        }
        let donut = HeroId::Donut;
        let body = sim.scene.world.body(donut);
        assert!(body.grounded);
        let floor_y = body.y - HERO_HALF_HEIGHT;
        let x = body.x;
        let (lo, hi) = CombatState::flat_span(&sim.scene, donut).expect("on the floor");
        assert!(lo <= x && x <= hi && hi - lo >= 40.0, "{lo} {x} {hi}");
        // On the left perch slab, 80 px up.
        assert!(sim.scene.regroup_place(donut, 600.0, floor_y + 80.0));
        assert_eq!(
            CombatState::flat_span(&sim.scene, donut),
            None,
            "airborne until it lands"
        );
        for _ in 0..30 {
            sim.step_slots([RawInput::NONE; 2]);
            if sim.scene.world.body(donut).grounded {
                break;
            }
        }
        let body = sim.scene.world.body(donut);
        assert!(body.grounded, "{body:?}");
        assert!(
            (body.y - HERO_HALF_HEIGHT - (floor_y + 80.0)).abs() < 2.0,
            "stands on the slab: {body:?}"
        );
        let (lo, hi) = CombatState::flat_span(&sim.scene, donut).expect("on the slab");
        let half = crate::prototype::HERO_HALF_WIDTH;
        let (left, right) = (
            crate::cf02::arena::LEFT_PERCH_X.0 as f32,
            crate::cf02::arena::LEFT_PERCH_X.1 as f32,
        );
        assert!(
            lo >= left + half - 4.0 && hi <= right - half + 4.0,
            "{lo} {hi}"
        );
        assert!(hi - lo >= 80.0, "most of the slab: {lo} {hi}");
    }
}
