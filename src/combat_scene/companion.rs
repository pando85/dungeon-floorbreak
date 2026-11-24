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
use crate::boss::BossThreat;
use crate::combat::{CombatEvent, ENEMY_HALF_HEIGHT, MoveSpec};
use crate::companion::autonomy::{
    AutonomyBrief, AutonomyInput, AutonomyState, BlinkChoice, BlinkInput, BlinkKnobs, DriftCheck,
    FreeZone, ViewRows, ZoneInput, choose_blink,
};
use crate::companion::director::{Action, CompanionDirector, Decision, Mode};
use crate::companion::engage::leader_engages;
use crate::companion::events::{
    CompanionEvent, CompanionStats, EVENT_CAP, RegroupOutcome, ReserveRelease,
};
use crate::companion::nav::{NavConfig, Reason};
use crate::companion::navfollow::{
    Body, NavDriver, NavNote, NavSpots, Regroup, RegroupView, arrive_eps_px, fence_step,
    hazards_from_feed,
};
use crate::companion::profile::{AutonomyRole, KillPolicy, ProfileFile, ProfileSet, ProfileSetId};
use crate::companion::rules;
use crate::companion::steer;
use crate::companion::view::{
    ActiveMove, BossView, CompanionView, Control, EntityKind, FollowInput, HeroView, MovePhase,
    StableEntityId, TargetState, TargetView, ThreatView, ViewBounds,
};
use crate::corridor::SceneSim;
use crate::donut_prog::{DonutProgTuning, SPECIAL_SHORT_BLINK};
use crate::feel::LLAMA_TELL_TICKS;
use crate::follower::Threat;
use crate::goblin::GoblinMode;
use crate::input::RawInput;
use crate::llama::LlamaMode;
use crate::party::HeroId;
use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, HeroMotion, Rect, TICK_SECONDS};

/// The profile file, parsed and hash-pinned once per process (CA-39).
fn profiles() -> &'static ProfileFile {
    #[cfg(test)]
    if let Some(file) = PROFILE_OVERRIDE.with(std::cell::Cell::get) {
        return file;
    }
    static PROFILES: OnceLock<ProfileFile> = OnceLock::new();
    PROFILES.get_or_init(|| {
        ProfileFile::pinned().expect("the embedded companion_ai_v0.json is valid and pinned")
    })
}

#[cfg(test)]
thread_local! {
    /// The knob-off tests (FA-Q "knob off reproduces the old hashes"): an edited
    /// copy of the profile file that this thread's simulations read instead.
    static PROFILE_OVERRIDE: std::cell::Cell<Option<&'static ProfileFile>> =
        const { std::cell::Cell::new(None) };
}

/// Run `run` with the pinned profile file after `edit` (the fix-all knobs off,
/// say) as the only profile this thread reads. The copy is leaked (a test's
/// lifetime) and the override is cleared on exit, a panic included.
#[cfg(test)]
pub(crate) fn with_profile_override<R>(
    edit: impl FnOnce(&mut ProfileFile),
    run: impl FnOnce() -> R,
) -> R {
    struct Clear;
    impl Drop for Clear {
        fn drop(&mut self) {
            PROFILE_OVERRIDE.with(|o| o.set(None));
        }
    }
    let mut file = ProfileFile::pinned().expect("the embedded profile file is pinned");
    edit(&mut file);
    let file: &'static ProfileFile = Box::leak(Box::new(file));
    PROFILE_OVERRIDE.with(|o| o.set(Some(file)));
    let _clear = Clear;
    run()
}

/// Every fix-all knob of every profile set off (tests): the pre-PR-A behaviour, which
/// the knob-off identity tests compare against the hashes pinned before it.
#[cfg(test)]
pub(crate) fn fix_all_off(f: &mut ProfileFile) {
    f.cf02_assist.fix_all = crate::companion::profile::FixAll::default();
    f.companion_v0.fix_all = crate::companion::profile::FixAll::default();
    f.cf02_free.fix_all = crate::companion::profile::FixAll::default();
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
    /// FA-Q26 (FA-Q26-AD1): the enemies Carl engages on the tick about to run, by
    /// the canonical predicate `engage::leader_engages` (A12, A11, A08). Empty
    /// without the knob or without a human Carl.
    engaged: Vec<StableEntityId>,
    /// FA-Q26-AD1: the same set at each hero's last press (the start tick of the move
    /// that press began), so a hit that lands later still sees what Carl was working
    /// when she committed.
    engaged_at_press: [Vec<StableEntityId>; 2],
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
    /// FA-Q37 (Boiler only): the reserve of each enemy (by index): how long it has been
    /// reserved in a row, the hits it landed meanwhile and whether it is released for the
    /// attempt. Empty unless a release knob is on, so every other run is untouched; state of
    /// the run while it has anything to say (hashed then). A restart clears it.
    reserve: Vec<ReserveTrack>,
    /// FA-Q37: how many events (counted since the run began, dropped ones included) the
    /// reserve bookkeeping has read (`None` until its first tick of an attempt). Derived,
    /// never hashed.
    reserve_cursor: Option<usize>,
}

/// FA-Q37: one enemy's reserve bookkeeping.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ReserveTrack {
    /// Ticks reserved in a row.
    run: u16,
    /// Hits on any hero while reserved.
    hits: u8,
    /// Released for the rest of the attempt.
    released: bool,
}

/// A marker word of a reserve track in the companion block ("RESV").
const RESERVE_MARK: u64 = 0x5245_5356_0000_0001;

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
            engaged: Vec::new(),
            engaged_at_press: [Vec::new(), Vec::new()],
            stats: CompanionStats::default(),
            events: Vec::new(),
            last_trace: [None; 2],
            nav: None,
            nav_failed: false,
            autonomy: [AutonomyState::new(), AutonomyState::new()],
            autonomous: [false; 2],
            reserve: Vec::new(),
            reserve_cursor: None,
        }
    }
}

impl CompanionRuntime {
    /// Attempt restart: the decision state, the marks and the drive flags start
    /// again (the cadence is measured in attempt-local ticks). The run's
    /// counters (the autonomy layer's M16 counters included) and event log are
    /// kept.
    pub(crate) fn reset(&mut self) {
        let stats = self.stats;
        let events = std::mem::take(&mut self.events);
        let autonomy_stats = [self.autonomy[0].stats(), self.autonomy[1].stats()];
        *self = Self::default();
        self.stats = stats;
        self.events = events;
        for (state, kept) in self.autonomy.iter_mut().zip(autonomy_stats) {
            state.carry_stats(kept);
        }
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
        // FA-Q26: the engaged sets are state of the run while they are set (empty
        // without the knob, so nothing is written then).
        for set in std::iter::once(&self.engaged).chain(&self.engaged_at_press) {
            if !set.is_empty() {
                mix(HASH_MARK ^ set.len() as u64);
                for id in set {
                    id.hash_into(mix);
                }
            }
        }
        // FA-Q37: the reserve bookkeeping, only while it has something to say.
        for (i, t) in self.reserve.iter().enumerate() {
            if t.run > 0 || t.hits > 0 || t.released {
                mix(RESERVE_MARK ^ i as u64);
                mix(u64::from(t.run));
                mix(u64::from(t.hits) | (u64::from(t.released) << 8));
            }
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

    /// FA-Q26 (FA-Q26-AD1): the enemy `id` is Carl's: the canonical engage predicate
    /// (`engage::leader_engages`: A12, then A11, then A08) held for it on this tick or
    /// at the start tick of her move, or he hit it within the A12 window as of this
    /// very tick (`hit_age`, which sees a hit of his earlier in the same tick). A
    /// standing Carl who never hit it, or who faces away under A11, claims nothing.
    /// Always `false` without the knob (or without a companion set).
    pub(crate) fn leader_claims(
        &self,
        hero: HeroId,
        id: StableEntityId,
        hit_age: Option<u16>,
    ) -> bool {
        let Some(set) = self.set.map(|s| profiles().set(s)) else {
            return false;
        };
        set.fix_all.fa_q26_steal_glance
            && (hit_age.is_some_and(|a| a <= set.cf02_duo.cf02_duo_a12_recent_hit_ticks)
                || self.engaged.contains(&id)
                || self.engaged_at_press[hero.index()].contains(&id))
    }

    /// The glance for a move instance of `kind`.
    pub(crate) fn glance_for(&self, kind: MarkKind, instance: u32) -> Glance {
        Glance {
            ai_started: self.marked(kind, instance),
            policy: self.policy(),
            boss_kill_is_carls: self
                .set
                .is_some_and(|s| profiles().set(s).fix_all.fa_q02_boss_play),
            target_claimed: false,
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
    /// FA-Q06: a lethal AI hit on the boss is a glance whatever the policy says
    /// (`fa_q02_boss_play`): the boss' kill is Carl's, so a companion-credited
    /// defeat never triggers a cinematic finisher.
    boss_kill_is_carls: bool,
    /// FA-Q26: the target is the leader's (he hit it within the A12 window, or it
    /// is within his A08 reach): a lethal AI hit on it is a glance.
    target_claimed: bool,
}

impl Default for Glance {
    /// A human-started hit: the identity.
    fn default() -> Self {
        Self {
            ai_started: false,
            policy: KillPolicy::Never,
            boss_kill_is_carls: false,
            target_claimed: false,
        }
    }
}

impl Glance {
    /// The move as it lands on a target with `target_hp` on the hit tick.
    pub(crate) fn apply(&self, spec: &MoveSpec, target_hp: u8) -> MoveSpec {
        // FA-Q26: a lethal AI hit on an enemy the leader is working is a glance.
        if self.target_claimed && self.ai_started && spec.damage > 0 && spec.damage >= target_hp {
            return MoveSpec { damage: 0, ..*spec };
        }
        glance_hook(spec, target_hp, self.ai_started, self.policy)
    }

    /// FA-Q26: the same glance for a target `claimed` by the leader.
    pub(crate) fn with_claim(mut self, claimed: bool) -> Self {
        self.target_claimed = claimed;
        self
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

impl Glance {
    /// The damage a hit on **the Juicer** lands (FA-Q06): [`Self::lands`], and a
    /// lethal AI hit is a glance whatever the kill policy says (the boss' kill is
    /// Carl's). Only the boss' hit site calls it: slot enemies keep [`Self::lands`].
    pub(crate) fn boss_lands(&self, damage: u16, target_hp: u16) -> u16 {
        if self.boss_kill_is_carls && self.ai_started && damage > 0 && damage >= target_hp {
            return 0;
        }
        self.lands(damage, target_hp)
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

/// DP-Q46 (DP-E): the tuning with the search's farthest candidate cut to the
/// last grid distance (`max - k * step`) that still reaches the hull edge from
/// `x` in direction `dir`, so "the farthest valid candidate inside the hull"
/// is the same pure search on the same candidate grid. `None` when even the
/// nearest candidate is outside the hull.
pub(super) fn hull_capped_tuning(
    t: &DonutProgTuning,
    x: f32,
    dir: i8,
    (lo, hi): (f32, f32),
) -> Option<DonutProgTuning> {
    let reach = if dir > 0 { hi - x } else { x - lo };
    if reach < f32::from(t.dpq22_range_min_px) {
        return None;
    }
    let (max, min, step) = (
        i32::from(t.dpq22_range_max_px),
        i32::from(t.dpq22_range_min_px),
        i32::from(t.dpq22_range_step_px).max(1),
    );
    let reach = reach.floor() as i32;
    let cap = if reach >= max {
        max
    } else {
        max - (max - reach + step - 1) / step * step
    };
    if cap < min {
        return None;
    }
    let mut capped = t.clone();
    capped.dpq22_range_max_px = cap as u16;
    Some(capped)
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

    /// Tests only: aim goblin `index`'s next blow at `hero` and put it in the
    /// swing at `attack_tick` of its tell, at `x` facing `facing` (the DP-E
    /// fixtures stage a committed attack at a chosen distance and tick).
    #[doc(hidden)]
    pub fn test_stage_goblin_swing(
        &mut self,
        index: usize,
        hero: HeroId,
        x: f32,
        facing: i8,
        attack_tick: u16,
    ) {
        if self.targets.len() <= index {
            self.targets.resize(index + 1, None);
        }
        self.targets[index] = Some(hero);
        let g = &mut self.enemies[index];
        g.x = x;
        g.facing = facing;
        g.mode = GoblinMode::Attack;
        g.attack_tick = attack_tick;
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
            self.companion.engaged.clear();
            self.companion.engaged_at_press = [Vec::new(), Vec::new()];
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
        // CA2-C (K13): the hero-hurting hazards of this tick, for the sets whose
        // partner the goal layer steers.
        self.companion_hazard_feed(
            scene,
            set.autonomy.enabled || set.fix_all.fa_q15_assist_rot_avoidance,
        );
        let owned = self.companion_regroup(scene);
        scene.set_nav_regroup(owned);
        let mut view = self.companion_view(scene, set_id);
        // FA-Q37: a reserved enemy that hurt a hero or was reserved too long is released.
        self.reserve_release(&mut view);
        // FA-Q26-AD1: whom Carl engages on this tick, by the canonical predicate.
        self.companion.engaged = engaged_set(set, &view);
        // FA-Q34: a lethal hit on a reserved enemy (she answers its tell) is a glance.
        self.companion.engaged.extend(view.reserved.iter().copied());
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
        let mut dodge_jumps = [false; 2];
        for hero in HeroId::ALL {
            let i = hero.index();
            if self.companion.autonomous[i] {
                (briefs[i], exempt[i], dodge_jumps[i]) =
                    self.companion_autonomy(scene, &mut view, set, hero);
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
            if brief.is_some()
                && let Some(nav) = &self.companion.nav
            {
                // CA2-C: the last guard of a ground step the graph did not plan
                // (an interrupt's approach, a dodge): never into a hazard.
                let run = self.run_px_per_tick(scene, hero);
                move_x = fence_step(nav.hazards(), Self::nav_body(scene, hero), move_x, run);
            }
            if brief.is_none()
                && set.fix_all.fa_q15_assist_rot_avoidance
                && let Some(nav) = &self.companion.nav
            {
                // FA-Q15: the assistant's follow steps never walk into a hazard
                // (a sticker's blast or trigger span, acid): she holds at the fence.
                let run = self.run_px_per_tick(scene, hero);
                move_x = fence_step(nav.hazards(), Self::nav_body(scene, hero), move_x, run);
            }
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
            // DONUT-PROG-01 DP-E: the escape-only blink. The press replaces the
            // director's input of this tick (the same Circle/B press path a human
            // uses, DP-Q31) and any press while the windup runs is withheld: a
            // primary, physical or jump press edge in the windup cancels it
            // (DP-Q25). Both paths need `blink_enabled`, so every other set is
            // untouched.
            if let Some(brief) = brief
                && let Some(choice) =
                    self.companion_blink(scene, &view, set, brief, hero, dodge_jumps[i])
            {
                self.companion.autonomy[i].note_blink(self.companion.tick, choice.reason);
                if choice.boss_charge && set.fix_all.fa_q09_blink_charge_window {
                    // FA-Q09: the dodge jump stays silent for this charge.
                    self.companion.autonomy[i].latch_charge_blink();
                }
                out[i] = RawInput {
                    move_x: f32::from(choice.dir),
                    ability: true,
                    ..RawInput::NONE
                };
                // DP-FU: the director's press is withheld on a blink tick, so it is
                // not counted (stats only; nothing hashed).
                let withheld = Decision {
                    pressed: false,
                    ..decision
                };
                self.companion.record(view.tick, hero, &withheld);
                continue;
            }
            if brief.is_some() && self.companion_blink_windup(set, hero) {
                out[i] = RawInput::NONE;
                let withheld = Decision {
                    pressed: false,
                    ..decision
                };
                self.companion.record(view.tick, hero, &withheld);
                continue;
            }
            // A press tick stands still: a direction held with a press would
            // turn the hero first (the Carl rule), maybe away from its target.
            let (mut raw, mut decision) = (raw, decision);
            if decision.pressed
                && set.fix_all.fa_q16_prop_safe_fire
                && self.press_crosses_prop(scene, &view, hero, &decision)
            {
                // FA-Q16: the press would hit a prop that is no goal of hers.
                decision.pressed = false;
                raw.attack = false;
                raw.physical = false;
            }
            if decision.pressed
                && reserve_withholds(&view.reserved, &view.threats, hero, decision.target)
            {
                // FA-Q34: the finish is the human leader's: no press (and no 0-damage
                // glance, which would stun-lock the target on the 16-tick refire).
                decision.pressed = false;
                raw.attack = false;
                raw.physical = false;
                self.companion.stats.reserve_withheld += 1;
            }
            let move_x = if decision.pressed { 0.0 } else { move_x };
            // FA-Q36: she does not walk into the sight of a sleeper before the human.
            let move_x = self.leash_move(scene, &view, hero, move_x);
            out[i] = RawInput {
                move_x,
                jump: raw.jump,
                attack: raw.attack,
                physical: raw.physical,
                ..RawInput::NONE
            };
            if decision.pressed {
                self.companion.pending_press[i] = Some(view.tick);
                self.companion.engaged_at_press[i].clone_from(&self.companion.engaged);
            }
            self.companion.record(view.tick, hero, &decision);
        }
        out
    }

    /// DP-E (DP-Q39): the blink the goal layer presses for `hero` this tick, if
    /// any. Only Donut's skirmisher table (`blink_enabled`, `cf02_free` only), only
    /// while she holds `special_short_blink` and the press would be accepted now:
    /// nothing running, no hit-stop, the cooldown over and the mana paid for (the
    /// reserve may be spent, DP-Q40). The prediction is the scene's own search,
    /// read-only, so hazards, `blink_barriers`, walls, zones and the camera are
    /// the human blink's.
    fn companion_blink(
        &self,
        scene: &SceneSim,
        view: &CompanionView,
        set: &ProfileSet,
        brief: &AutonomyBrief,
        hero: HeroId,
        dodge_jumps: bool,
    ) -> Option<BlinkChoice> {
        let table = &set.autonomy.roles.skirmisher;
        if hero != HeroId::Donut || !table.blink_enabled || !self.story_holds(SPECIAL_SHORT_BLINK) {
            return None;
        }
        let t = DonutProgTuning::active();
        let tick = self.companion.tick;
        let kit = &self.donut_kit;
        if kit.blink.windup().is_some()
            || kit.blink.recovering(tick)
            || kit.blink.cooling(tick)
            || self.hero_state(hero).hit_stop > 0
            || kit.mana.mana() < t.dpq23_cost_charges
        {
            return None;
        }
        let (x, y) = {
            let body = scene.world.body(hero);
            (body.x, body.y)
        };
        let windup = u16::from(t.dpq25_windup_ticks);
        let hull = self.ai_blink_hull(scene, x);
        choose_blink(
            &BlinkInput {
                view,
                partner: hero,
                goal: brief.goal,
                zone: &Self::zone_of(scene, view, set, hero),
                drift: &DriftCheck::new(Self::motion_of(scene, hero.other()), &set.autonomy),
                knobs: BlinkKnobs {
                    threat_px: f32::from(table.blink_threat_px),
                    min_gain_px: f32::from(table.blink_min_gain_px),
                    hull_px: f32::from(t.dpq46_ai_hull_px),
                    windup_ticks: windup,
                    commit_ticks: windup + u16::from(t.dpq25_recovery_ticks),
                    // FA-Q09, FA-Q10: the boss-charge window and the reach gate.
                    charge_window_extra: set
                        .fix_all
                        .fa_q09_blink_charge_window
                        .then_some(set.fix_all.fa_q09_window_extra_ticks),
                    reach_margin_px: set
                        .fix_all
                        .fa_q10_blink_reach_gate
                        .then_some(f32::from(set.fix_all.fa_q10_reach_margin_px)),
                },
                dodge_jumps,
            },
            |dir| self.blink_search_in(scene, x, y, dir, Some(hull)),
        )
    }

    /// DP-Q46: the hull an AI blink from `x` is held to, `[min(x, carl.x - H),
    /// max(x, carl.x + H)]` with `H` = `dpq46_ai_hull_px`. Reads sim state only.
    fn ai_blink_hull(&self, scene: &SceneSim, x: f32) -> (f32, f32) {
        let h = f32::from(DonutProgTuning::active().dpq46_ai_hull_px);
        let carl = scene.world.body(HeroId::Carl).x;
        (x.min(carl - h), x.max(carl + h))
    }

    /// The hull of the blink running now, only if the goal layer pressed it
    /// (its press tick is the windup's t0), else `None`: a human's blink, and a
    /// swap or a restart that dropped the layer's record, are unconstrained.
    pub(super) fn ai_blink_hull_if_started(&self, scene: &SceneSim, x: f32) -> Option<(f32, f32)> {
        let (start, _) = self.donut_kit.blink.windup()?;
        (self.companion.autonomy[HeroId::Donut.index()].last_blink() == Some(start))
            .then(|| self.ai_blink_hull(scene, x))
    }

    /// DP-E: her blink windup is running under an AI that may blink, so the
    /// director's presses are withheld this tick (a press would cancel it).
    fn companion_blink_windup(&self, set: &ProfileSet, hero: HeroId) -> bool {
        hero == HeroId::Donut
            && set.autonomy.roles.skirmisher.blink_enabled
            && self.donut_kit.blink.windup().is_some()
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
    /// from the zone clamp (a dodge or a committed transit), and whether the follow
    /// layer's jump dodge fires this tick (DP-E reads it).
    fn companion_autonomy(
        &mut self,
        scene: &SceneSim,
        view: &mut CompanionView,
        set: &ProfileSet,
        hero: HeroId,
    ) -> (Option<AutonomyBrief>, bool, bool) {
        let i = hero.index();
        let Some(mut nav) = self.companion.nav.take() else {
            return (None, false, false);
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
            let world = NavSpots::new(&nav, hero, f32::from(a.ca2_q29_ledge_inset_px))
                .with_transit_slack(a.ca2_q36_transit_slack_ticks);
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
            return (None, false, false);
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
        // DP-E: the follow layer's jump dodge fires this tick (set below).
        let mut dodge_jumps = false;
        if Self::follow_free_view(&view.heroes[i], &self.hero_state(hero)) {
            let me = Self::nav_body(scene, hero);
            let bounds = Some((
                view.view.left + NAV_BOUNDS_MARGIN,
                view.view.right - NAV_BOUNDS_MARGIN,
            ));
            // CA2-C: standing in a hazard (a telegraph or a burst began on her),
            // getting out comes before any goal: the goal is the nearest safe
            // ground, and the exit is involuntary, so it may leave `Z`.
            let evacuation = nav.evacuation(hero, me, bounds);
            exempt = evacuation.is_some();
            let goal_body = evacuation.unwrap_or_else(|| {
                brief.spot.map_or(me, |s| Body {
                    x: s.x,
                    foot_y: s.surface_y,
                    grounded: true,
                })
            });
            let (drive, notes) = nav.go_to(hero, me, goal_body, bounds);
            let run = self.run_px_per_tick(scene, hero);
            let eps = arrive_eps_px(Self::motion_of(scene, hero).run_speed);
            let mut input = match (drive, evacuation) {
                (Some(d), _) => {
                    exempt |= nav.heroes[i].is_active();
                    FollowInput {
                        move_x: d.move_x,
                        jump: d.jump,
                    }
                }
                (None, Some(safe)) => FollowInput {
                    move_x: if (safe.x - me.x).abs() <= eps {
                        0.0
                    } else {
                        steer::approach_stick(me.x, safe.x, run)
                    },
                    jump: false,
                },
                (None, None) => FollowInput {
                    move_x: steer::flat_steer(&view.heroes[i], &brief, run, eps),
                    jump: false,
                },
            };
            let active = match hero {
                HeroId::Carl => set.carl.ca21_active,
                HeroId::Donut => set.donut.ca21_active,
            };
            let threats = self.follow_threats(hero, active);
            // FA-Q09: a blink pressed for the boss' charge silences the follow
            // layer's boss dodge for that charge (the latch is released with it).
            let mut latched = self.companion.autonomy[i].charge_latched();
            if latched && !matches!(view.boss, Some(BossThreat::Charge { .. })) {
                self.companion.autonomy[i].release_charge_latch();
                latched = false;
            }
            let dodge_boss = view.boss.filter(|_| !latched);
            let dodge = scene.dodge_input(hero, &threats, dodge_boss.as_ref());
            if dodge.jump || dodge.move_x != 0.0 {
                input = FollowInput {
                    move_x: dodge.move_x,
                    jump: dodge.jump,
                };
                exempt = true;
                dodge_jumps = dodge.jump;
            } // FA-Q05 (recorded deviation): hop a shockwave ripple that is coming at
            // her on the ground (the follow layer has no ripple dodge).
            if set.fix_all.fa_q02_boss_play
                && !dodge_jumps
                && dodge.move_x == 0.0
                && view.heroes[i].grounded
                && self.ripple_coming(scene, hero, f32::from(set.fix_all.fa_q05_ripple_hop_px))
            {
                input.jump = true;
                dodge_jumps = true;
            }
            view.follow[i] = input;
            self.push_nav_notes(hero, notes);
        } else {
            view.follow[i] = FollowInput::default();
        }
        self.companion.nav = Some(nav);
        (Some(brief), exempt, dodge_jumps)
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

    /// COMPANION-AI-02 CA2-C (K13): feed the live nav graph the hazard
    /// rectangles that can hurt a hero this tick, so the partner's routes, spots
    /// and holds avoid them. Producers: the Boiler's env step and the acid
    /// patches ([`super::EnvRuntime::nav_feed`]) and, since HAZ-B, the slimes' contact
    /// boxes and the stickers' trigger and blast boxes
    /// ([`super::CombatState::hazard_feed`], one stable order); a later hazard wire
    /// (the wheel) adds its own `NavRect`s there and nothing in the companion
    /// changes. Only a set whose
    /// partner the goal layer steers is fed (`autonomy.enabled`), so every other
    /// set and every room without hazards builds and hashes as before; a feed
    /// equal to the last one is a no-op.
    fn companion_hazard_feed(&mut self, scene: &SceneSim, enabled: bool) {
        if self.companion.nav.is_none() {
            return;
        }
        let hazards = if enabled {
            let feed = self.hazard_feed(scene.world.layout_height(), scene.world.tick);
            hazards_from_feed(&feed, scene.world.layout_height())
        } else {
            Vec::new()
        };
        let rects = hazards.len();
        let tick = self.companion.tick;
        let Some(nav) = &mut self.companion.nav else {
            return;
        };
        match nav.set_hazards(hazards, &scene.world.solids) {
            Ok(true) => {
                self.companion.stats.hazard_feed_changes += 1;
                self.companion.push_event(CompanionEvent::HazardFeed {
                    tick,
                    rects: u8::try_from(rects).unwrap_or(u8::MAX),
                });
            }
            Ok(false) => {}
            Err(error) => {
                // The graph refused the list (a duplicate id; the feed makes them
                // unique, so this is a defect). Loud in debug and test builds; in
                // release the graph keeps the last list and the failure is counted.
                self.companion.stats.hazard_feed_errors += 1;
                debug_assert!(false, "the hazard feed was refused: {error:?}");
            }
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

        let mut targets = self.companion_targets(view, scene.world.tick + 1);
        let (reserved, asleep) = self.partner_knob_lists(&heroes, &mut targets);
        CompanionView {
            tick: scene.world.tick + 1,
            solo: live && !drive.two_player,
            cf02_rules: self.cf02.is_some(),
            profile_set,
            view,
            heroes,
            follow,
            threats: self.companion_threats(view),
            targets,
            reserved,
            asleep,
            boss,
            boss_view: profile
                .fix_all
                .fa_q02_boss_play
                .then(|| self.companion_boss_view())
                .flatten(),
            flank_span,
        }
    }

    /// FA-Q34 and FA-Q35 (the room's partner knobs, both off elsewhere): the ids reserved
    /// for the human leader's finish (living, at most one hit point, within the radius of
    /// him) and the ids still asleep or counting their wake delay. The sleepers are taken
    /// out of `targets`, so no goal or press can pick one; the reserved stay (an interrupt
    /// may still answer a tell aimed at her). Both lists are empty with the knobs off, a
    /// leader who is not a living human, or no enemy in the case.
    fn partner_knob_lists(
        &self,
        heroes: &[HeroView; 2],
        targets: &mut Vec<TargetView>,
    ) -> (Vec<StableEntityId>, Vec<StableEntityId>) {
        let knobs = self.partner_knobs;
        let mut asleep = Vec::new();
        if knobs.hold_fire_until_awake {
            targets.retain(|t| {
                let sleeping = t.id.kind == EntityKind::Goblin
                    && self
                        .enemies
                        .get(t.id.index as usize)
                        .is_some_and(|g| !g.woken());
                if sleeping {
                    asleep.push(t.id);
                }
                !sleeping
            });
        }
        let mut reserved = Vec::new();
        if let Some(radius) = knobs.finish_reserve_px
            && let Some(leader) = heroes
                .iter()
                .find(|h| matches!(h.control, Control::Human(_)) && !h.defeated)
        {
            reserved = reserved_ids(targets, leader.x, radius);
        }
        (reserved, asleep)
    }

    /// FA-Q37: the reserve bookkeeping of this tick (see [`ReserveTrack`]). Reads the hurts
    /// the combat log gained since the last tick, counts the ticks each candidate has been
    /// reserved in a row, and releases an enemy that landed `release_on_hits` hits or was
    /// reserved `release_after_ticks` ticks: it leaves `view.reserved` and stays out for the
    /// attempt. Each release is a [`CompanionEvent::ReserveReleased`] and a stat. A no-op
    /// without a release knob.
    fn reserve_release(&mut self, view: &mut CompanionView) {
        let (hits_cap, after) = (
            self.partner_knobs.release_on_hits,
            self.partner_knobs.release_after_ticks,
        );
        if hits_cap.is_none() && after.is_none() {
            return;
        }
        let n = self.enemies.len();
        self.companion.reserve.resize(n, ReserveTrack::default());
        // The cursor counts events since the run began (the log drops its oldest entries at
        // `EVENT_LOG_CAP`, so a position in the retained log would go stale): the events
        // not yet read start at its distance from the first retained one.
        let total = self.dropped_events + self.events.len();
        let from = self
            .companion
            .reserve_cursor
            .unwrap_or(total)
            .saturating_sub(self.dropped_events)
            .min(self.events.len());
        let mut new_hits = vec![0u8; n];
        for e in &self.events[from..] {
            if let CombatEvent::HeroHurt { source, .. } = e
                && let Some(h) = new_hits.get_mut(*source as usize)
            {
                *h = h.saturating_add(1);
            }
        }
        self.companion.reserve_cursor = Some(total);
        for (i, hits) in new_hits.into_iter().enumerate() {
            let id = StableEntityId::goblin(i as u32);
            let candidate = view.reserved.binary_search(&id).is_ok();
            let track = &mut self.companion.reserve[i];
            if track.released {
                view.reserved.retain(|r| *r != id);
                continue;
            }
            if !candidate {
                track.run = 0;
                track.hits = 0;
                continue;
            }
            track.run = track.run.saturating_add(1);
            track.hits = track.hits.saturating_add(hits);
            let cause = if hits_cap.is_some_and(|h| track.hits >= h) {
                Some(ReserveRelease::Hit)
            } else if after.is_some_and(|a| track.run >= a) {
                Some(ReserveRelease::Timeout)
            } else {
                None
            };
            if let Some(cause) = cause {
                track.released = true;
                view.reserved.retain(|r| *r != id);
                match cause {
                    ReserveRelease::Hit => self.companion.stats.reserve_released_hit += 1,
                    ReserveRelease::Timeout => self.companion.stats.reserve_released_timeout += 1,
                }
                for hero in HeroId::ALL {
                    if self.companion.driven[hero.index()] {
                        self.companion.autonomy[hero.index()].clear_plan();
                    }
                }
                self.companion.push_event(CompanionEvent::ReserveReleased {
                    tick: view.tick,
                    target: id,
                    cause,
                });
            }
        }
    }

    /// FA-Q36: `move_x` of the AI hero `hero`, or 0 when it would take her deeper into the
    /// sight (plus the leash margin) of an enemy that has not been woken. Not clamped: while
    /// the human leader is inside that sight, while she is hurt or an attack tell is aimed
    /// at her, or when the step leads out of the zone. Without the knob it is `move_x`.
    fn leash_move(&self, scene: &SceneSim, view: &CompanionView, hero: HeroId, move_x: f32) -> f32 {
        let Some(margin) = self.partner_knobs.wake_leash_margin_px else {
            return move_x;
        };
        if move_x == 0.0 {
            return move_x;
        }
        let me = view.hero(hero);
        if me.hurt_ticks > 0 || view.threats.iter().any(|t| t.aimed_at == Some(hero)) {
            return move_x;
        }
        let sleepers: Vec<(f32, f32)> = self
            .enemies
            .iter()
            .filter(|g| g.alive() && !g.woken())
            .map(|g| (g.x, g.sight_px()))
            .collect();
        let next = me.x + move_x.signum() * self.run_px_per_tick(scene, hero);
        if leash_blocks(me.x, next, view.hero(hero.other()).x, &sleepers, margin) {
            0.0
        } else {
            move_x
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
                // FA-Q10: a goblin's swing box (the Engineer's is a ram, unknown).
                swing_x: (g.kind == crate::goblin::GoblinKind::Common).then(|| {
                    let b = g.swing_box();
                    (b.left, b.right())
                }),
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
                    swing_x: None,
                });
            }
        }
        threats
    }

    /// Ticks from Carl's last hit on target slot `slot` (goblins, then llamas)
    /// to `now`, saturating (CF02-DUO-A12).
    pub(super) fn carl_hit_age(&self, slot: usize, now: u64) -> Option<u16> {
        let at = (*self.carl_hit_tick.get(slot)?)?;
        Some(now.saturating_sub(at).min(u64::from(u16::MAX)) as u16)
    }

    /// FA-Q16: the hitbox path of the press `decision` (a swat's box ahead of her, a
    /// pounce's hop to its target, a missile's line from its spawn to its target or
    /// its range) crosses a live prop hit target at her height.
    fn press_crosses_prop(
        &self,
        scene: &SceneSim,
        view: &CompanionView,
        hero: HeroId,
        decision: &Decision,
    ) -> bool {
        let rects = self.prop_target_rects(scene.world.layout_height());
        if rects.is_empty() {
            return false;
        }
        let me = view.hero(hero);
        let dir = f32::from(me.facing);
        let target_x = decision
            .target
            .and_then(|id| view.find_target(id))
            .map(|t| t.x);
        let (a, b) = match decision.action {
            Some(Action::Swat) => (me.x, me.x + dir * 30.0),
            Some(Action::Pounce) => (me.x, target_x.unwrap_or(me.x + dir * 80.0)),
            Some(Action::Missile) => (me.x + dir * 27.0, target_x.unwrap_or(me.x + dir * 160.0)),
            _ => return false,
        };
        path_crosses(&rects, (a.min(b), a.max(b)), me.foot_y)
    }

    /// FA-Q05: a shockwave ripple of the boss is within `lead` px of `hero` and
    /// moving toward her (it has not passed her yet).
    fn ripple_coming(&self, scene: &SceneSim, hero: HeroId, lead: f32) -> bool {
        let x = scene.world.body(hero).x;
        self.boss.as_ref().is_some_and(|b| {
            b.ripples().any(|(rx, dir)| {
                let ahead = (x - rx) * f32::from(dir);
                ahead > 0.0 && ahead <= lead
            })
        })
    }

    /// FA-Q02: the boss as the goal layer sees it, whenever one is alive in the
    /// room (not only during a tell).
    fn companion_boss_view(&self) -> Option<BossView> {
        let boss = self.boss.as_ref().filter(|b| b.alive())?;
        Some(BossView::from_facts(&boss.facts(), boss.floor_y()))
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

/// FA-Q36: whether the step of the AI hero from `x` to `next` is held back by the wake
/// leash. `sleepers` are the `(x, sight)` of the enemies not yet woken. A sleeper holds her
/// when the step ends inside its sight plus `margin` and nearer to it than where she stands
/// (a step out of the zone, or along its edge away from it, is free), and the human leader
/// at `leader_x` is not already inside its sight.
pub(crate) fn leash_blocks(
    x: f32,
    next: f32,
    leader_x: f32,
    sleepers: &[(f32, f32)],
    margin: f32,
) -> bool {
    sleepers.iter().any(|&(ex, sight)| {
        (leader_x - ex).abs() >= sight
            && (next - ex).abs() < sight + margin
            && (next - ex).abs() < (x - ex).abs()
    })
}

/// FA-Q34: the living enemies with at most one hit point left within `radius` px of the human
/// leader at `leader_x`, in id order (`targets` is): his to finish.
pub(crate) fn reserved_ids(
    targets: &[TargetView],
    leader_x: f32,
    radius: f32,
) -> Vec<StableEntityId> {
    targets
        .iter()
        .filter(|t| t.hp > 0 && t.hp <= 1 && (t.x - leader_x).abs() <= radius)
        .map(|t| t.id)
        .collect()
}

/// FA-Q34: whether the AI's press of `hero` on `target` is withheld: the target is reserved
/// for the human leader and no attack tell of it is aimed at `hero` (then the press may go
/// and the lethal hit becomes a glance, because the reserved ids are in the engaged set).
pub(crate) fn reserve_withholds(
    reserved: &[StableEntityId],
    threats: &[ThreatView],
    hero: HeroId,
    target: Option<StableEntityId>,
) -> bool {
    let Some(id) = target else {
        return false;
    };
    reserved.binary_search(&id).is_ok()
        && !threats
            .iter()
            .any(|t| t.id == id && t.aimed_at == Some(hero))
}

/// FA-Q16: whether the x interval `span` at her feet `foot_y` overlaps any of the
/// prop target `rects` (her body band: from just under her feet to her height).
pub(crate) fn path_crosses(rects: &[Rect], span: (f32, f32), foot_y: f32) -> bool {
    rects.iter().any(|r| {
        r.right() > span.0
            && r.left < span.1
            && r.top() > foot_y - 8.0
            && r.bottom < foot_y + 2.0 * HERO_HALF_HEIGHT
    })
}

/// FA-Q26-AD1: the enemies a human Carl engages in `view`, by the canonical predicate
/// (`engage::leader_engages`). Empty without the knob, without a human Carl, or with
/// him defeated.
pub(crate) fn engaged_set(set: &ProfileSet, view: &CompanionView) -> Vec<StableEntityId> {
    let leader = view.hero(HeroId::Carl);
    if !set.fix_all.fa_q26_steal_glance
        || !matches!(leader.control, Control::Human(_))
        || leader.defeated
    {
        return Vec::new();
    }
    view.targets
        .iter()
        .filter(|t| t.hp > 0 && leader_engages(set, leader, t))
        .map(|t| t.id)
        .collect()
}

/// A llama's stable id (serial 0: no respawn in 01A fixtures, CA-28).
pub(super) fn llama_id(index: usize) -> StableEntityId {
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

    fn tgt(index: u32, x: f32, hp: u8) -> TargetView {
        TargetView {
            id: StableEntityId::goblin(index),
            x,
            foot_y: 0.0,
            hp,
            state: TargetState::Standing,
            guarded: false,
            in_view: true,
            carl_hit: false,
            carl_hit_age: None,
        }
    }

    fn tell(index: u32, aimed_at: Option<HeroId>) -> ThreatView {
        ThreatView {
            id: StableEntityId::goblin(index),
            x: 0.0,
            foot_y: 0.0,
            facing: 1,
            tell_tick: 4,
            tell_len: 24,
            impact_tick: 26,
            aimed_at,
            guarded: false,
            in_view: true,
            swing_x: None,
        }
    }

    /// FA-Q34: reserved means one hit point or less (and alive) within the radius of the
    /// leader, at the radius itself and one pixel beyond it, on both sides of him.
    #[test]
    fn fa_q34_reserved_is_one_hp_within_the_radius_of_the_leader() {
        let targets = [
            tgt(0, 400.0 + 160.0, 1), // at the radius, 1 hp: reserved
            tgt(1, 400.0 + 161.0, 1), // one px beyond: not
            tgt(2, 400.0 - 160.0, 1), // the other side, at the radius: reserved
            tgt(3, 400.0 - 161.0, 1), // the other side, beyond: not
            tgt(4, 450.0, 2),         // near, but 2 hp: not
            tgt(5, 450.0, 0),         // near, but dead: not
        ];
        let ids = reserved_ids(&targets, 400.0, 160.0);
        assert_eq!(ids, [StableEntityId::goblin(0), StableEntityId::goblin(2)]);
        // Beyond the radius the enemy is not reserved: she may kill it (owner 0.1(1)).
        assert!(reserved_ids(&targets, 1000.0, 160.0).is_empty());
    }

    /// FA-Q37 (re-review of #257): the reserve's hit counting survives the combat log dropping
    /// its oldest events. The log is overflowed before the first tick (it stays at its cap, so
    /// a cursor kept as a position in it would never see another event); a goblin with one hit
    /// point then reaches the idle human, hurts him, and is released by that hit (not by the
    /// 150 tick timeout).
    #[test]
    fn fa_q37_the_hit_release_survives_the_event_log_overflowing() {
        use crate::boiler_fixtures::scene_pilots::{Who, live_start_with};
        use crate::combat::EnemyAi;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (mut sim, _) =
            live_start_with(&root, Who::Companion, 0, crate::f01_boiler::fixes::active()).unwrap();
        for i in 0..(2 * crate::combat_scene::EVENT_LOG_CAP) {
            sim.combat.push_event(CombatEvent::GoblinTell {
                tick: i as u64,
                goblin: 3,
            });
        }
        assert_eq!(sim.combat.events.len(), crate::combat_scene::EVENT_LOG_CAP);
        let carl_x = sim.scene.world.hero.x;
        {
            let g = &mut sim.combat.enemies[0];
            g.x = carl_x + 60.0;
            g.health = 1;
            g.ai = EnemyAi::Chase;
        }
        for _ in 0..140 {
            sim.step_slots([RawInput::NONE; 2]);
        }
        let stats = sim.combat.companion_stats();
        assert!(
            sim.combat.dropped_events > 0,
            "the log really dropped events"
        );
        assert!(
            stats.reserve_released_hit >= 1,
            "released by the hit on the full log: {stats:?}"
        );
        assert_eq!(stats.reserve_released_timeout, 0, "{stats:?}");
    }

    /// FA-Q36: the leash holds her at the sight (220) plus the margin (4) of a sleeper: a step
    /// that ends at 224 px or more is free, one that ends at 223 is held, a step out of the
    /// zone is free, and so is anything while the human leader is already inside the sight.
    #[test]
    fn fa_q36_the_leash_holds_her_at_the_sight_plus_the_margin() {
        let sleepers = [(500.0, 220.0)];
        // Standing at 270 (230 px away), a step of 6 px lands at 276 (224 px away): free.
        assert!(!leash_blocks(270.0, 276.0, 100.0, &sleepers, 4.0));
        // A step to 277 (223 px away) enters the zone: held.
        assert!(leash_blocks(271.0, 277.0, 100.0, &sleepers, 4.0));
        // The same on the other side of the sleeper.
        assert!(!leash_blocks(730.0, 724.0, 100.0, &sleepers, 4.0));
        assert!(leash_blocks(729.0, 723.0, 100.0, &sleepers, 4.0));
        // Inside the zone already (a knockback): a step out is free, one deeper is held.
        assert!(!leash_blocks(300.0, 294.0, 100.0, &sleepers, 4.0));
        assert!(leash_blocks(300.0, 306.0, 100.0, &sleepers, 4.0));
        // The human leader inside the sight (219 px): she may go.
        assert!(!leash_blocks(271.0, 277.0, 281.0, &sleepers, 4.0));
        assert!(leash_blocks(271.0, 277.0, 280.0, &sleepers, 4.0));
        // No sleeper, no leash; margin 0 holds at the sight itself.
        assert!(!leash_blocks(271.0, 277.0, 100.0, &[], 4.0));
        assert!(!leash_blocks(270.0, 280.0, 100.0, &sleepers, 0.0));
        assert!(leash_blocks(270.0, 281.0, 100.0, &sleepers, 0.0));
    }

    /// FA-Q34: a press on a reserved enemy is withheld, except while an attack tell of it is
    /// aimed at the pressing hero (then the interrupt may go: a glance, not a kill).
    #[test]
    fn fa_q34_a_reserved_press_is_withheld_unless_its_tell_is_aimed_at_her() {
        let reserved = [StableEntityId::goblin(1)];
        let g1 = Some(StableEntityId::goblin(1));
        let g0 = Some(StableEntityId::goblin(0));
        // No tell: withheld. Another target: not.
        assert!(reserve_withholds(&reserved, &[], HeroId::Donut, g1));
        assert!(!reserve_withholds(&reserved, &[], HeroId::Donut, g0));
        assert!(!reserve_withholds(&reserved, &[], HeroId::Donut, None));
        // A tell aimed at the leader (the human): still withheld.
        let at_leader = [tell(1, Some(HeroId::Carl))];
        assert!(reserve_withholds(&reserved, &at_leader, HeroId::Donut, g1));
        // A tell aimed at her: the press may go.
        let at_her = [tell(1, Some(HeroId::Donut))];
        assert!(!reserve_withholds(&reserved, &at_her, HeroId::Donut, g1));
        // Another enemy's tell aimed at her does not release this one.
        let other = [tell(0, Some(HeroId::Donut))];
        assert!(reserve_withholds(&reserved, &other, HeroId::Donut, g1));
    }

    /// FA-Q34: the reserved ids join the leader's claims (never Hunt's, never a steal).
    #[test]
    fn fa_q34_the_reserved_join_the_leader_claims() {
        use crate::companion::autonomy::LeaderClaims;
        use crate::companion::view::fixtures;
        let set = profiles().set(ProfileSetId::Cf02Free).clone();
        let leader = fixtures::hero(HeroId::Carl, 400.0, Control::Human(0));
        let donut = fixtures::hero(HeroId::Donut, 300.0, Control::Companion);
        let mut view = fixtures::view(leader, donut);
        // A standing enemy 150 px behind a standing leader: no engage, no front claim.
        view.targets = vec![tgt(0, 250.0, 1)];
        assert!(!LeaderClaims::compute(&view, &set, &leader).contains(StableEntityId::goblin(0)));
        view.reserved = vec![StableEntityId::goblin(0)];
        assert!(LeaderClaims::compute(&view, &set, &leader).contains(StableEntityId::goblin(0)));
    }

    /// FA-Q35 on the live Boiler: with the knob on, a goblin that is asleep or counting its
    /// wake delay is no target (and is listed as asleep); once its delay is over it is one.
    /// With the knobs off (every other room) the lists are empty and every enemy is a target.
    #[test]
    fn fa_q35_a_sleeper_is_no_target_until_its_wake_delay_is_over() {
        use crate::companion::view::{PartnerKnobs, fixtures};
        use crate::f01_late::{Loadout, boiler_start_with};
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (carl, donut) = Loadout::Story.flags();
        let (_run, mut scene, combat) = boiler_start_with(
            &root,
            360,
            carl,
            donut,
            None,
            crate::f01_boiler::Roster::Live,
        )
        .unwrap();
        let heroes = [
            fixtures::hero(HeroId::Carl, 115.0, Control::Human(0)),
            fixtures::hero(HeroId::Donut, 80.0, Control::Companion),
        ];
        let view = ViewBounds {
            left: 0.0,
            right: 1152.0,
        };
        let ids = |t: &[TargetView]| t.iter().map(|t| t.id.index).collect::<Vec<_>>();
        let off = combat.clone().with_partner_knobs(PartnerKnobs::default());
        let mut on = combat.with_partner_knobs(PartnerKnobs {
            finish_reserve_px: None,
            hold_fire_until_awake: true,
            ..PartnerKnobs::default()
        });
        let mut t = on.companion_targets(view, 1);
        let (reserved, asleep) = on.partner_knob_lists(&heroes, &mut t);
        assert!(reserved.is_empty(), "no reserve radius: nothing reserved");
        // FA-Q35 widened (review of #257): "awake" is a persistent flag (a hero was once in
        // sight, it was touched, or its gate released and its delay elapsed). At the spawn no
        // hero is within 220 px of g1 (344), so nobody is awake, g1 included.
        assert!(t.is_empty(), "nobody is awake at the spawn");
        assert_eq!(
            asleep.iter().map(|i| i.index).collect::<Vec<_>>(),
            [0, 1, 2, 3]
        );
        let mut t = off.companion_targets(view, 1);
        let (reserved, asleep) = off.partner_knob_lists(&heroes, &mut t);
        assert!(reserved.is_empty() && asleep.is_empty());
        assert_eq!(ids(&t), [0, 1, 2, 3], "knobs off: every enemy is a target");
        // A hero steps into g1's sight (x 124 and on): it wakes, and stays awake.
        scene.world.hero.x = 130.0;
        on.step_party(&mut scene, [RawInput::NONE; 2]);
        let mut t = on.companion_targets(view, 1);
        let (_, asleep) = on.partner_knob_lists(&heroes, &mut t);
        assert_eq!(ids(&t), [0], "g1 is awake and a target");
        assert_eq!(
            asleep.iter().map(|i| i.index).collect::<Vec<_>>(),
            [1, 2, 3]
        );
        scene.world.hero.x = 100.0;
        on.step_party(&mut scene, [RawInput::NONE; 2]);
        let mut t = on.companion_targets(view, 1);
        on.partner_knob_lists(&heroes, &mut t);
        assert_eq!(ids(&t), [0], "persistent: g1 stays awake out of sight");
        // A hero leaves the protected spawn (x 224): g2's wake delay (15 ticks) runs, then
        // it is awake and a target; g3 and e1 sleep on (their trigger is 560).
        scene.world.hero.x = 230.0;
        let mut first_target = None;
        for tick in 0..40u32 {
            on.step_party(&mut scene, [RawInput::NONE; 2]);
            let mut t = on.companion_targets(view, 1);
            let (_, asleep) = on.partner_knob_lists(&heroes, &mut t);
            let g2_listed = asleep.contains(&StableEntityId::goblin(1));
            let g2_target = t.iter().any(|x| x.id == StableEntityId::goblin(1));
            assert_ne!(g2_listed, g2_target, "tick {tick}: asleep xor target");
            assert!(asleep.contains(&StableEntityId::goblin(2)), "g3 sleeps on");
            if g2_target && first_target.is_none() {
                first_target = Some(tick);
            }
            assert!(first_target.is_none() || g2_target, "awake stays awake");
        }
        let at = first_target.expect("g2 wakes after its delay");
        assert!((14..=17).contains(&at), "g2 is a target on tick {at}");
    }

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

    /// FA-Q06: the boss' kill is Carl's. A lethal AI hit on the boss lands as a
    /// glance whatever the kill policy says (the boss site only), a non-lethal
    /// one and a human-started one are unchanged, and the rule is the knob's.
    #[test]
    fn fa_q06_a_lethal_companion_hit_on_the_boss_is_a_glance_whatever_the_policy() {
        let mut runtime = CompanionRuntime {
            set: Some(ProfileSetId::Cf02Free),
            ..CompanionRuntime::default()
        };
        runtime.marks.push((MarkKind::DonutMove, 5));
        runtime.marks.push((MarkKind::DonutCast, 6));
        // `cf02_free` is `factual`: the policy alone never glances.
        assert_eq!(runtime.glance_for(MarkKind::DonutMove, 5).lands(1, 1), 1);
        for (kind, instance) in [(MarkKind::DonutMove, 5), (MarkKind::DonutCast, 6)] {
            let g = runtime.glance_for(kind, instance);
            // Lethal (damage at least the boss' HP) is a glance, exactly at the edge
            // and beyond it, and with an HP above a byte.
            assert_eq!(g.boss_lands(1, 1), 0, "damage = hp");
            assert_eq!(g.boss_lands(2, 1), 0, "damage > hp");
            assert_eq!(g.boss_lands(1, 300), 1, "far from lethal");
            assert_eq!(g.boss_lands(3, 300), 3);
            assert_eq!(g.boss_lands(1, 2), 1, "one above the edge lands");
            // The slot enemies keep the policy-only rule.
            assert_eq!(g.lands(1, 1), 1);
            // A zero-damage hit stays zero (poise only).
            assert_eq!(g.boss_lands(0, 1), 0);
        }
        // A move the human started is never a glance.
        assert_eq!(
            runtime.glance_for(MarkKind::DonutMove, 99).boss_lands(1, 1),
            1
        );
        // The knob off (a set without boss play): the rule is the old one.
        with_profile_override(
            |f| f.cf02_free.fix_all = crate::companion::profile::FixAll::default(),
            || {
                let g = runtime.glance_for(MarkKind::DonutMove, 5);
                assert_eq!(g.boss_lands(1, 1), 1, "knob off: not a glance");
            },
        );
        // A runtime with no set (the companion off) is the identity.
        let off = CompanionRuntime::default();
        assert_eq!(off.glance_for(MarkKind::DonutMove, 5).boss_lands(1, 1), 1);
    }

    /// FA-Q26-AD1: the claim is Carl's real engage check, not distance. A lethal AI hit
    /// on an enemy he engages (A12 window at 31/32 ticks, A08 reach at 40/40.5 px with
    /// a move running, A11 facing) is a glance; an idle Carl beside an enemy he never
    /// hit claims nothing; the knob off claims nothing.
    #[test]
    fn fa_q26_a_lethal_ai_hit_on_an_enemy_the_leader_engages_is_a_glance() {
        use crate::combat::MoveId;
        use crate::companion::view::{ActiveMove, MovePhase, fixtures};
        let set = profiles().set(ProfileSetId::Cf02Free);
        let goblin = StableEntityId::goblin(0);
        let target = |dx: f32, hit_age: Option<u16>| TargetView {
            id: goblin,
            x: 400.0 + dx,
            foot_y: 0.0,
            hp: 2,
            state: TargetState::Standing,
            guarded: false,
            in_view: true,
            carl_hit: hit_age.is_some(),
            carl_hit_age: hit_age,
        };
        let view_with = |facing: i8, moving: bool, t: TargetView| {
            let mut leader = fixtures::hero(HeroId::Carl, 400.0, Control::Human(0));
            leader.facing = facing;
            leader.active_move = moving.then_some(ActiveMove {
                id: MoveId::Jab,
                phase: MovePhase::Active,
                ticks_left: 6,
                ai_started: false,
            });
            let mut view = fixtures::view(
                leader,
                fixtures::hero(HeroId::Donut, 300.0, Control::Companion),
            );
            view.targets = vec![t];
            view
        };
        let claims = |view: &CompanionView, hit_age: Option<u16>| {
            let runtime = CompanionRuntime {
                set: Some(ProfileSetId::Cf02Free),
                engaged: engaged_set(set, view),
                ..CompanionRuntime::default()
            };
            runtime.leader_claims(HeroId::Donut, goblin, hit_age)
        };
        // Idle Carl, a goblin 30 px away he never hit: not claimed (the old bare
        // distance rule claimed it, and her lethal pounce on it became a glance).
        let idle = view_with(1, false, target(30.0, None));
        assert!(!claims(&idle, None), "idle Carl never engaged it");
        // Carl jabbing toward the goblin at 40 px: claimed; at 40.5 px: not.
        assert!(claims(&view_with(1, true, target(40.0, None)), None));
        assert!(!claims(&view_with(1, true, target(40.5, None)), None));
        // Jabbing the other way under A11: not claimed ...
        assert!(!claims(&view_with(-1, true, target(40.0, None)), None));
        // ... unless he hit it within the A12 window (31 ticks inclusive).
        assert!(claims(
            &view_with(-1, true, target(-300.0, Some(31))),
            Some(31)
        ));
        assert!(!claims(
            &view_with(-1, false, target(-300.0, Some(32))),
            Some(32)
        ));
        // The live hit age sees a hit of his earlier in this same tick.
        assert!(claims(&idle, Some(0)));
        // At the start tick: the set taken when her move started still claims it after
        // Carl's move has ended.
        let mut runtime = CompanionRuntime {
            set: Some(ProfileSetId::Cf02Free),
            engaged: Vec::new(),
            engaged_at_press: [
                Vec::new(),
                engaged_set(set, &view_with(1, true, target(40.0, None))),
            ],
            ..CompanionRuntime::default()
        };
        assert!(runtime.leader_claims(HeroId::Donut, goblin, None));
        runtime.engaged_at_press = [Vec::new(), Vec::new()];
        assert!(!runtime.leader_claims(HeroId::Donut, goblin, None));
        use crate::combat::DONUT_POUNCE;
        let lethal = |g: Glance, hp: u8| g.apply(&DONUT_POUNCE, hp).damage;
        runtime.marks.push((MarkKind::DonutMove, 5));
        let g = runtime.glance_for(MarkKind::DonutMove, 5);
        assert_eq!(lethal(g.with_claim(true), DONUT_POUNCE.damage), 0, "glance");
        assert_eq!(
            lethal(g.with_claim(true), DONUT_POUNCE.damage + 1),
            DONUT_POUNCE.damage
        );
        assert_eq!(
            lethal(g.with_claim(false), DONUT_POUNCE.damage),
            DONUT_POUNCE.damage
        );
        let human = runtime.glance_for(MarkKind::DonutMove, 99).with_claim(true);
        assert_eq!(lethal(human, DONUT_POUNCE.damage), DONUT_POUNCE.damage);
        // The knob off claims nothing (and computes no set).
        with_profile_override(
            |f| f.cf02_free.fix_all = crate::companion::profile::FixAll::default(),
            || {
                let off = profiles().set(ProfileSetId::Cf02Free);
                let view = view_with(1, true, target(10.0, Some(0)));
                assert!(engaged_set(off, &view).is_empty(), "knob off");
                assert!(!claims(&view, Some(0)), "knob off");
            },
        );
        assert!(
            !CompanionRuntime::default().leader_claims(HeroId::Donut, goblin, Some(0)),
            "off"
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

    /// DP-Q46 (DP-E): the hull cuts the search's farthest candidate to the last
    /// grid distance (112, 108, ... 16) that reaches the hull edge, so "the
    /// farthest valid candidate inside the hull" is the same pure search.
    #[test]
    fn the_hull_cap_keeps_the_candidate_grid() {
        let t = DonutProgTuning::active();
        let cap = |x: f32, dir: i8, hull: (f32, f32)| {
            hull_capped_tuning(t, x, dir, hull).map(|c| c.dpq22_range_max_px)
        };
        // Reach beyond the range: unchanged.
        assert_eq!(cap(440.0, -1, (300.0, 503.0)), Some(112));
        assert_eq!(cap(440.0, -1, (328.0, 503.0)), Some(112), "exactly 112");
        // 103 px of reach: 112, 108, 104, 100: the farthest grid point <= 103.
        assert_eq!(cap(440.0, -1, (337.0, 503.0)), Some(100));
        assert_eq!(cap(440.0, -1, (336.0, 503.0)), Some(104), "104 reaches 336");
        assert_eq!(cap(440.0, 1, (377.0, 543.0)), Some(100), "103 to the right");
        // The nearest candidate (16) is the floor.
        assert_eq!(cap(440.0, -1, (424.0, 503.0)), Some(16));
        assert_eq!(cap(440.0, -1, (424.5, 503.0)), None, "15.5 px: none");
        assert_eq!(cap(440.0, 1, (400.0, 440.0)), None, "no reach at all");
    }
}
