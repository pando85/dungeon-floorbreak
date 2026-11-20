//! The native environment: reset, step and episode status (AP-Q09 to AP-Q11).

use std::collections::BTreeSet;
use std::path::Path;

use crate::combat::CombatEvent;
use crate::combat_scene::{Fixture, FixtureSim};
use crate::env_kit::EnvKit;

use super::action::{ActionV1, UnavailableCounters};
use super::config::AutoplayConfig;
use super::error::AutoplayError;

/// The corridor layout the Floor 01 fixtures run on (relative to the asset root).
const CORRIDOR_LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";
/// Desktop view height, the one the headless fixture path uses.
const VIEW_H: i32 = 360;

/// Scenarios of this part and the `state_hash` of their fresh sim. A reset
/// rebuilds the sim and fails loudly when it does not hash to this value
/// (AP-Q11). The new `ap_*` scenarios arrive with part 2.
const SCENARIOS: [(&str, u64); 2] = [
    ("cp01_single_goblin", 0x707a_cdf0_0ae4_52b5),
    ("cp02_two_goblins", 0xa820_6ad8_287d_62e4),
];

/// How an episode ended (AP-Q09). `Timeout` is the only truncation; the other
/// three are terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Every required hostile was defeated.
    Success,
    /// Carl was defeated (checked on the defeating step).
    Defeat,
    /// Carl reached the corridor exit with required hostiles alive.
    Abandoned,
    /// The tick budget ran out with Carl alive.
    Timeout,
}

impl Outcome {
    pub fn name(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Defeat => "defeat",
            Self::Abandoned => "abandoned",
            Self::Timeout => "timeout",
        }
    }

    pub fn terminated(self) -> bool {
        !self.truncated()
    }

    pub fn truncated(self) -> bool {
        self == Self::Timeout
    }
}

/// What `reset` reports. Non-privileged bookkeeping only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResetInfo {
    pub scenario_id: String,
    pub requested_seed: u64,
    /// The fixture's own constant; no gameplay RNG exists on this build.
    pub sim_seed: u64,
    /// Always `false` today: the reset seed only seeds policy-side randomness.
    pub seed_affects_sim: bool,
    pub cf02_rules: bool,
    pub action_version: u32,
    pub max_ticks: u64,
    pub action_repeat: u32,
    /// `state_hash` of the fresh sim, checked against the pinned value.
    pub initial_hash: u64,
    /// Events already present in the fresh sim (events drained at reset).
    pub event_cursor: usize,
    /// Number of hostiles that must be defeated for success.
    pub required_enemies: usize,
}

/// The result of one `step` (one to `action_repeat` real ticks).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepOutcome {
    /// Ticks executed by this call (less than `action_repeat` when the episode
    /// ended mid-bundle).
    pub ticks_executed: u32,
    /// Executed ticks of the episode so far (`combat.elapsed`).
    pub tick: u64,
    /// `Some` when the episode is over: terminal or truncated.
    pub outcome: Option<Outcome>,
    /// Defeat and success were both true after the same tick (AP-Q09).
    pub simultaneous_clear: bool,
    /// Combat events of every executed tick of this call, in order.
    pub new_events: Vec<CombatEvent>,
}

impl StepOutcome {
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

/// One Carl-only fight on the real fixture simulation. Each env owns its sim,
/// kit clone and counters; nothing is shared between envs (AP-Q13).
#[derive(Debug, Clone)]
pub struct AutoplayEnv {
    config: AutoplayConfig,
    kit: EnvKit,
    fixture: &'static Fixture,
    initial_hash: u64,
    sim: FixtureSim,
    phase: Phase,
    requested_seed: u64,
    previous_action: Option<ActionV1>,
    unavailable: UnavailableCounters,
    events: Vec<CombatEvent>,
    defeated: BTreeSet<u32>,
    required: Vec<u32>,
}

impl AutoplayEnv {
    /// Build an env for a supported scenario. `assets_root` is the directory
    /// holding `assets/levels/floor01/`.
    pub fn new(config: AutoplayConfig, assets_root: &Path) -> Result<Self, AutoplayError> {
        config.validate()?;
        let (fixture, initial_hash) = scenario(&config.scenario_id)?;
        Self::build(config, assets_root, fixture, initial_hash)
    }

    /// Parity probe: any fixture, no pinned initial hash (AP-Q18). Tests only.
    #[cfg(test)]
    pub(crate) fn new_unchecked(
        config: AutoplayConfig,
        assets_root: &Path,
        fixture: &'static Fixture,
    ) -> Result<Self, AutoplayError> {
        config.validate()?;
        let mut env = Self::build(config, assets_root, fixture, 0)?;
        env.initial_hash = env.sim.state_hash();
        Ok(env)
    }

    fn build(
        config: AutoplayConfig,
        assets_root: &Path,
        fixture: &'static Fixture,
        initial_hash: u64,
    ) -> Result<Self, AutoplayError> {
        let kit = EnvKit::load(assets_root, CORRIDOR_LAYOUT)
            .map_err(|e| AutoplayError::Sim(e.to_string()))?;
        let sim = FixtureSim::new(&kit, VIEW_H, fixture).map_err(AutoplayError::Sim)?;
        let required = (0..fixture.enemies.len() as u32).collect();
        Ok(Self {
            config,
            kit,
            fixture,
            initial_hash,
            sim,
            phase: Phase::NeedsReset,
            requested_seed: 0,
            previous_action: None,
            unavailable: UnavailableCounters::default(),
            events: Vec::new(),
            defeated: BTreeSet::new(),
            required,
        })
    }

    /// Start a fresh episode: a brand new `FixtureSim` (never `restart`, which
    /// keeps deaths, elapsed and events), cleared counters and edge state.
    pub fn reset(&mut self, seed: u64) -> Result<ResetInfo, AutoplayError> {
        let mut sim =
            FixtureSim::new(&self.kit, VIEW_H, self.fixture).map_err(AutoplayError::Sim)?;
        let actual = sim.state_hash();
        if actual != self.initial_hash {
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
        Ok(ResetInfo {
            scenario_id: self.config.scenario_id.clone(),
            requested_seed: seed,
            sim_seed: self.fixture.seed,
            seed_affects_sim: false,
            cf02_rules: self.config.cf02_rules,
            action_version: self.config.action_version,
            max_ticks: self.config.max_ticks,
            action_repeat: self.config.action_repeat,
            initial_hash: actual,
            event_cursor: drained,
            required_enemies: self.required.len(),
        })
    }

    /// Advance `action_repeat` real ticks (stopping at the end of the episode).
    /// The action is held across the bundle: one held button, not new presses.
    pub fn step(&mut self, action: ActionV1) -> Result<StepOutcome, AutoplayError> {
        match self.phase {
            Phase::NeedsReset => return Err(AutoplayError::NotReset),
            Phase::Over(outcome) => return Err(AutoplayError::EpisodeOver(outcome)),
            Phase::Running => {}
        }
        let first_new = self.events.len();
        let raw = action.to_raw_input();
        let mut ticks_executed = 0;
        let mut simultaneous_clear = false;
        let mut outcome = None;
        for _ in 0..self.config.action_repeat {
            self.unavailable.record(self.previous_action, action);
            self.previous_action = Some(action);
            self.tick(raw);
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
        }
        Ok(StepOutcome {
            ticks_executed,
            tick: self.sim.combat.elapsed,
            outcome,
            simultaneous_clear,
            new_events: self.events[first_new..].to_vec(),
        })
    }

    /// One tick with an explicit input and no terminal checks: replays a
    /// fixture's own script (AP-Q18). Not part of the policy API.
    #[cfg(test)]
    pub(crate) fn step_raw(&mut self, raw: crate::input::RawInput) {
        assert!(self.phase != Phase::NeedsReset, "step_raw before reset");
        self.tick(raw);
    }

    /// Step the real sim once and drain the tick's events into the episode log.
    fn tick(&mut self, raw: crate::input::RawInput) {
        self.sim.step(raw);
        let first_new = self.events.len();
        self.events
            .extend_from_slice(self.sim.combat.take_new_events());
        for event in &self.events[first_new..] {
            if let CombatEvent::EnemyDefeated { target, .. } = event {
                self.defeated.insert(*target);
            }
        }
    }

    /// Terminal decision after a tick: defeat > success > abandoned, then the
    /// time limit (AP-Q09). The second value is `simultaneous_clear`.
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
        } else if self.sim.combat.elapsed >= self.config.max_ticks {
            Some(Outcome::Timeout)
        } else {
            None
        };
        (outcome, defeat && success)
    }

    /// The wrapped sim, read-only (tests).
    #[cfg(test)]
    pub(crate) fn sim(&self) -> &FixtureSim {
        &self.sim
    }

    pub fn config(&self) -> &AutoplayConfig {
        &self.config
    }

    /// Executed ticks of the current episode (`combat.elapsed`, never `world.tick`).
    pub fn tick_count(&self) -> u64 {
        self.sim.combat.elapsed
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

    /// Privileged: authoritative state hash (diagnostics, never a policy input).
    pub fn state_hash(&self) -> u64 {
        self.sim.state_hash()
    }

    /// Privileged: every combat event of this episode in order.
    pub fn episode_events(&self) -> &[CombatEvent] {
        &self.events
    }
}

/// Resolve a v1 scenario id to its fixture and pinned initial hash.
fn scenario(id: &str) -> Result<(&'static Fixture, u64), AutoplayError> {
    let Some(&(_, hash)) = SCENARIOS.iter().find(|(name, _)| *name == id) else {
        let supported: Vec<&str> = SCENARIOS.iter().map(|(name, _)| *name).collect();
        return Err(AutoplayError::UnsupportedScenario(format!(
            "{id} (supported: {})",
            supported.join(", ")
        )));
    };
    let fixture = Fixture::by_name(id)
        .ok_or_else(|| AutoplayError::UnsupportedScenario(format!("{id} (no such fixture)")))?;
    Ok((fixture, hash))
}
