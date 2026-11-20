//! [`PartyAutoplayEnv`]: the `party_solo` env (AP-Q22, AP-Q23, AP-Q29).
//!
//! The sim is a [`PartyFixtureSim`] stepped **only** through `step_slots`, the
//! interactive scene's own order (swap request, hero routing, scene drive,
//! combat step). The env never calls `SceneSim::step_party` or
//! `CombatState::step_party` itself, and never runs into a restart (the
//! episode ends on the defeating step), so `PartyState::restart`, which would
//! return control to Carl, is never reached.

use std::collections::BTreeSet;
use std::path::Path;

use crate::combat::CombatEvent;
use crate::env_kit::EnvKit;
use crate::input::{RawInput, SlotInputs};
use crate::party::HeroId;
use crate::party_scene::{PartyFixture, PartyFixtureSim};

use super::super::action::{ActionV1, UnavailableCounters};
use super::super::config::AutoplayConfig;
use super::super::env::{CORRIDOR_LAYOUT, Outcome, ResetInfo};
use super::super::error::AutoplayError;
use super::super::observation::ObsStats;
use super::observation::{ObservationV2, observe_v2};
use super::scenario::{self, PartyRosterEntry};
use super::{AutoplayMode, PARTY_REWARD_PENDING};

/// Desktop view height, the one the headless fixture path uses.
const VIEW_H: i32 = 360;

/// The result of one `step` (one to `action_repeat` real ticks). Like the v1
/// `StepOutcome` it carries nothing hidden (AP-Q08); the reward (AP-Q32)
/// arrives with 02a-C.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyStepOutcome {
    /// Ticks executed by this call (less than `action_repeat` when the episode
    /// ended mid-bundle).
    pub ticks_executed: u32,
    /// Executed ticks of the episode so far: the env's own per-real-tick
    /// counter, the seat-entry tick included.
    pub tick: u64,
    /// `Some` when the episode is over: terminal or truncated.
    pub outcome: Option<Outcome>,
    /// Defeat and success were both true after the same tick (AP-Q09).
    pub simultaneous_clear: bool,
    /// Which hero's defeat ended the attempt (`Some` only on `Defeat`).
    pub defeated_hero: Option<HeroId>,
}

impl PartyStepOutcome {
    pub fn terminated(&self) -> bool {
        self.outcome.is_some_and(Outcome::terminated)
    }

    pub fn truncated(&self) -> bool {
        self.outcome.is_some_and(Outcome::truncated)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    NeedsReset,
    Running,
    Over(Outcome),
}

/// One solo-party fight on the real party path. Each env owns its sim, kit
/// clone and counters; nothing is shared between envs (AP-Q13).
#[derive(Debug, Clone)]
pub struct PartyAutoplayEnv {
    config: AutoplayConfig,
    seat: HeroId,
    kit: EnvKit,
    fixture: &'static PartyFixture,
    initial_hash: u64,
    sim: PartyFixtureSim,
    phase: Phase,
    requested_seed: u64,
    previous_action: Option<ActionV1>,
    unavailable: UnavailableCounters,
    events: Vec<CombatEvent>,
    /// Events already in the log when `reset` returned (the entry tick's).
    reset_events: usize,
    defeated: BTreeSet<u32>,
    required: Vec<u32>,
    /// The per-real-tick counter (AP-Q31): incremented for every simulated
    /// tick, the entry tick included.
    ticks: u64,
    defeated_hero: Option<HeroId>,
    obs_stats: ObsStats,
    /// Parity probes: `reset` is skipped, no entry and no swap stripping.
    raw_probe: bool,
}

impl PartyAutoplayEnv {
    /// Build an env for a roster scenario. `assets_root` is the directory
    /// holding `assets/levels/floor01/`.
    pub fn new(config: AutoplayConfig, assets_root: &Path) -> Result<Self, AutoplayError> {
        config.validate()?;
        let AutoplayMode::PartySolo {
            seat,
            companion_profile,
        } = config.mode.clone()
        else {
            return Err(AutoplayError::InvalidConfig(
                "mode solo is run by AutoplayEnv, not PartyAutoplayEnv (AP-Q22)".into(),
            ));
        };
        if config.max_ticks < 2 {
            return Err(AutoplayError::InvalidConfig(
                "party_solo max_ticks must be at least 2: the seat entry is env tick 1".into(),
            ));
        }
        let entry = scenario::lookup(&config.scenario_id, &companion_profile)?;
        Self::build(config, seat, assets_root, entry)
    }

    /// Parity probe: any `&'static PartyFixture`, no roster validation, no pinned
    /// initial hash, no seat entry and no swap stripping (AP-Q35). The sim is
    /// fresh and running; drive it with [`Self::step_raw`]. Tests only.
    #[cfg(test)]
    pub(crate) fn new_unchecked(
        assets_root: &Path,
        fixture: &'static PartyFixture,
    ) -> Result<Self, AutoplayError> {
        let config = AutoplayConfig::party_solo(fixture.base.name, HeroId::Carl, "probe");
        let kit = EnvKit::load(assets_root, CORRIDOR_LAYOUT)
            .map_err(|e| AutoplayError::Sim(e.to_string()))?;
        let sim = PartyFixtureSim::new(&kit, VIEW_H, fixture).map_err(AutoplayError::Sim)?;
        let initial_hash = sim.state_hash();
        let required = (0..fixture.base.enemies.len() as u32).collect();
        Ok(Self {
            config,
            seat: HeroId::Carl,
            kit,
            fixture,
            initial_hash,
            sim,
            phase: Phase::Running,
            requested_seed: 0,
            previous_action: None,
            unavailable: UnavailableCounters::default(),
            events: Vec::new(),
            reset_events: 0,
            defeated: BTreeSet::new(),
            required,
            ticks: 0,
            defeated_hero: None,
            obs_stats: ObsStats::default(),
            raw_probe: true,
        })
    }

    fn build(
        config: AutoplayConfig,
        seat: HeroId,
        assets_root: &Path,
        entry: PartyRosterEntry,
    ) -> Result<Self, AutoplayError> {
        scenario::validate_fixture(entry.fixture)?;
        let kit = EnvKit::load(assets_root, CORRIDOR_LAYOUT)
            .map_err(|e| AutoplayError::Sim(e.to_string()))?;
        let sim = PartyFixtureSim::new(&kit, VIEW_H, entry.fixture).map_err(AutoplayError::Sim)?;
        let required = (0..entry.fixture.base.enemies.len() as u32).collect();
        Ok(Self {
            config,
            seat,
            kit,
            fixture: entry.fixture,
            initial_hash: entry.initial_hash,
            sim,
            phase: Phase::NeedsReset,
            requested_seed: 0,
            previous_action: None,
            unavailable: UnavailableCounters::default(),
            events: Vec::new(),
            reset_events: 0,
            defeated: BTreeSet::new(),
            required,
            ticks: 0,
            defeated_hero: None,
            obs_stats: ObsStats::default(),
            raw_probe: false,
        })
    }

    /// Start a fresh episode: a brand new `PartyFixtureSim`, the pre-entry hash
    /// check (AP-Q11), the one seat-entry tick for **both** seats (AP-Q23) and
    /// the post-entry observation.
    pub fn reset(&mut self, seed: u64) -> Result<ResetInfo, AutoplayError> {
        let entry = [scenario::entry_slot(self.seat), RawInput::NONE];
        self.reset_with_entry(entry, seed)
    }

    /// `reset` with an explicit entry-tick input. Crate-private so a test can
    /// show a failed entry is a reset error (AP-Q23).
    pub(crate) fn reset_with_entry(
        &mut self,
        entry: SlotInputs,
        seed: u64,
    ) -> Result<ResetInfo, AutoplayError> {
        let mut sim =
            PartyFixtureSim::new(&self.kit, VIEW_H, self.fixture).map_err(AutoplayError::Sim)?;
        let actual = sim.state_hash();
        if !self.raw_probe && actual != self.initial_hash {
            return Err(AutoplayError::InitialHashMismatch {
                expected: self.initial_hash,
                actual,
            });
        }
        let drained = sim.combat.take_new_events().len();
        self.sim = sim;
        self.phase = Phase::Running;
        self.requested_seed = seed;
        self.previous_action = None;
        self.unavailable = UnavailableCounters::default();
        self.events.clear();
        self.defeated.clear();
        self.ticks = 0;
        self.defeated_hero = None;
        self.obs_stats = ObsStats::default();

        // Seat entry: slot 0 presses swap for Donut (the game's own
        // `request_swap` runs it and `hero_inputs`, after it, already routes
        // the tick to Donut); Carl's tick is idle. Slot 1 is always idle. The
        // entry is not a policy action: nothing counts it.
        self.tick(entry);
        if self.sim.party.controlled_hero() != self.seat {
            self.phase = Phase::NeedsReset;
            return Err(AutoplayError::SeatEntryFailed(format!(
                "controlled hero is {:?} after the entry tick, expected {:?}",
                self.sim.party.controlled_hero(),
                self.seat
            )));
        }
        if self.evaluate().0.is_some() {
            self.phase = Phase::NeedsReset;
            return Err(AutoplayError::SeatEntryFailed(
                "the episode ended on the entry tick".into(),
            ));
        }
        self.reset_events = self.events.len();
        self.record_observation();
        Ok(ResetInfo {
            scenario_id: self.config.scenario_id.clone(),
            requested_seed: seed,
            sim_seed: self.fixture.base.seed,
            seed_affects_sim: false,
            cf02_rules: self.config.cf02_rules,
            action_version: self.config.action_version,
            max_ticks: self.config.max_ticks,
            action_repeat: self.config.action_repeat,
            initial_hash: actual,
            event_cursor: drained,
            required_enemies: self.required.len(),
            reward_id: PARTY_REWARD_PENDING.to_string(),
            observation_version: self.config.obs_version,
        })
    }

    /// Advance `action_repeat` real ticks (stopping at the end of the episode).
    /// The action is held across the bundle: one held button, not new presses.
    /// Index 5 (`swap`) is reserved and stripped: counted as an unavailable
    /// attempt, never forwarded (forwarding would change the seat, AP-Q23).
    pub fn step(&mut self, action: ActionV1) -> Result<PartyStepOutcome, AutoplayError> {
        match self.phase {
            Phase::NeedsReset => return Err(AutoplayError::NotReset),
            Phase::Over(outcome) => return Err(AutoplayError::EpisodeOver(outcome)),
            Phase::Running => {}
        }
        let raw = RawInput {
            swap: false,
            ..action.to_raw_input()
        };
        let mut ticks_executed = 0;
        let mut simultaneous_clear = false;
        let mut outcome = None;
        for _ in 0..self.config.action_repeat {
            self.unavailable.record(self.previous_action, action);
            self.previous_action = Some(action);
            self.tick([raw, RawInput::NONE]);
            ticks_executed += 1;
            let (decided, both) = self.evaluate();
            simultaneous_clear = both;
            if decided.is_some() {
                outcome = decided;
                break;
            }
        }
        if let Some(outcome) = outcome {
            self.phase = Phase::Over(outcome);
            if outcome == Outcome::Defeat {
                self.defeated_hero = Some(self.sim.combat.defeated_hero);
            }
        }
        self.record_observation();
        Ok(PartyStepOutcome {
            ticks_executed,
            tick: self.ticks,
            outcome,
            simultaneous_clear,
            defeated_hero: self.defeated_hero,
        })
    }

    /// One tick with explicit slot inputs forwarded verbatim (swap and slot 1
    /// included) and no terminal checks: parity with the party goldens
    /// (AP-Q35). Not part of the policy API.
    #[cfg(test)]
    pub(crate) fn step_raw(&mut self, slots: SlotInputs) {
        assert!(self.phase != Phase::NeedsReset, "step_raw before reset");
        self.tick(slots);
    }

    /// Step the real sim once and drain the tick's events into the episode log.
    fn tick(&mut self, slots: SlotInputs) {
        self.sim.step_slots(slots);
        self.ticks += 1;
        let first_new = self.events.len();
        self.events
            .extend_from_slice(self.sim.combat.take_new_events());
        for event in &self.events[first_new..] {
            if let CombatEvent::EnemyDefeated { target, .. } = event {
                self.defeated.insert(*target);
            }
        }
    }

    /// Terminal decision after a tick (AP-Q29): defeat > success > abandoned,
    /// then the time limit. The second value is `simultaneous_clear`.
    fn evaluate(&self) -> (Option<Outcome>, bool) {
        let defeat = self.sim.combat.hero_defeated;
        let success = self.required.iter().all(|id| self.defeated.contains(id));
        let abandoned = self.sim.scene.world.complete;
        let outcome = if defeat {
            Some(Outcome::Defeat)
        } else if success {
            Some(Outcome::Success)
        } else if abandoned {
            Some(Outcome::Abandoned)
        } else if self.ticks >= self.config.max_ticks {
            Some(Outcome::Timeout)
        } else {
            None
        };
        (outcome, defeat && success)
    }

    /// The policy input for the current state: the seat's ObservationV2. Pure.
    pub fn observe(&self) -> ObservationV2 {
        observe_v2(
            &self.sim.scene,
            &self.sim.combat,
            &self.sim.party,
            self.seat,
        )
        .0
    }

    /// Fold the current observation's counters into the episode totals. Called
    /// once per `reset` and per `step`, never by `observe` itself.
    fn record_observation(&mut self) {
        let (_, stats) = observe_v2(
            &self.sim.scene,
            &self.sim.combat,
            &self.sim.party,
            self.seat,
        );
        self.obs_stats.add(stats);
    }

    pub fn config(&self) -> &AutoplayConfig {
        &self.config
    }

    /// The policy's hero for the whole episode.
    pub fn seat(&self) -> HeroId {
        self.seat
    }

    /// Executed ticks of the episode: the env's own per-real-tick counter,
    /// the entry tick included (never `combat.elapsed` or `world.tick`).
    pub fn tick_count(&self) -> u64 {
        self.ticks
    }

    /// The episode outcome once it is over.
    pub fn outcome(&self) -> Option<Outcome> {
        match self.phase {
            Phase::Over(outcome) => Some(outcome),
            _ => None,
        }
    }

    pub fn unavailable_counters(&self) -> UnavailableCounters {
        self.unavailable
    }

    /// Privileged: the party state hash (diagnostics, never a policy input).
    pub fn state_hash(&self) -> u64 {
        self.sim.state_hash()
    }

    /// Privileged: observation-side counters (clamps, unknown modes, overflow).
    pub fn obs_stats(&self) -> ObsStats {
        self.obs_stats
    }

    /// Privileged: the pre-entry initial hash this env pins (AP-Q23).
    pub fn initial_hash(&self) -> u64 {
        self.initial_hash
    }

    /// Privileged, crate-internal: every combat event of this episode in
    /// order, the entry tick's included (they are the first
    /// [`Self::reset_event_count`] entries).
    pub fn episode_events(&self) -> &[CombatEvent] {
        &self.events
    }

    /// Events drained as reset events (the entry tick's): folded into metrics,
    /// never into a step reward.
    pub fn reset_event_count(&self) -> usize {
        self.reset_events
    }

    /// The wrapped sim, read-only (tests).
    #[cfg(test)]
    pub(crate) fn sim(&self) -> &PartyFixtureSim {
        &self.sim
    }

    /// The wrapped sim, mutable (tests that build leakage pairs).
    #[cfg(test)]
    pub(crate) fn sim_mut(&mut self) -> &mut PartyFixtureSim {
        &mut self.sim
    }
}

#[cfg(test)]
impl PartyAutoplayEnv {
    /// Mark every required hostile defeated (terminal-precedence tests).
    pub(crate) fn mark_all_defeated(&mut self) {
        self.defeated.extend(self.required.iter().copied());
    }
}
