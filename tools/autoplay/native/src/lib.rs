//! AUTOPLAY-01C: PyO3 binding over the native `AutoplayEnv` (AP-Q02).
//!
//! The binding is a thin shell. Every rule (ticks, terminal decision, reward,
//! observation) lives in `dungeon_floorbreak::autoplay`; this crate only
//! converts types. Privileged data (state hashes, diagnostics) is reachable
//! through explicit methods and never through `info` (AP-Q08).

mod bridge;

use numpy::{PyArray1, PyArrayMethods};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use dungeon_floorbreak::autoplay::bots::{BOT_IDS, BOT_SETTINGS_VERSION, Bot, make_bot};
use dungeon_floorbreak::autoplay::observation::{
    AVAILABLE_LEN, ENEMY_FEATURES, HAZARD_FEATURES, MAX_ENEMIES, MAX_HAZARDS, SELF_LEN,
};
use dungeon_floorbreak::autoplay::replay::{EpisodeRecording, kit_content_hash};
use dungeon_floorbreak::autoplay::scenario::roster;
use dungeon_floorbreak::autoplay::{
    ACTION_V1_LEN, ACTION_VERSION_V1, ActionV1, AutoplayConfig, AutoplayEnv, AutoplayError, OBS_V1,
    OBSERVATION_VERSION, Outcome, ResetInfo, Split,
};

pyo3::create_exception!(
    _native,
    EpisodeOverError,
    PyRuntimeError,
    "`step` was called before `reset` or after the episode ended."
);
pyo3::create_exception!(
    _native,
    InvalidActionError,
    PyValueError,
    "The action has the wrong length or an out-of-range component."
);

fn to_py_err(err: AutoplayError) -> PyErr {
    let message = err.to_string();
    match err {
        AutoplayError::NotReset | AutoplayError::EpisodeOver(_) => {
            EpisodeOverError::new_err(message)
        }
        AutoplayError::InvalidAction(_) => InvalidActionError::new_err(message),
        AutoplayError::InvalidConfig(_)
        | AutoplayError::Cf02Unsupported
        | AutoplayError::SimSeedUnsupported
        | AutoplayError::UnsupportedScenario(_)
        | AutoplayError::UnsupportedCompanion(_) => PyValueError::new_err(message),
        AutoplayError::Sim(_)
        | AutoplayError::InitialHashMismatch { .. }
        | AutoplayError::SeatEntryFailed(_) => PyRuntimeError::new_err(message),
    }
}

fn build_config(
    scenario_id: &str,
    max_ticks: u64,
    action_repeat: u32,
    reward_id: &str,
) -> AutoplayConfig {
    let mut config = AutoplayConfig::new(scenario_id);
    config.max_ticks = max_ticks;
    config.action_repeat = action_repeat;
    config.reward_id = reward_id.to_string();
    config
}

fn flat<'py, const N: usize, const M: usize>(
    py: Python<'py>,
    rows: &[[f32; M]; N],
) -> PyResult<Bound<'py, PyAny>> {
    let values: Vec<f32> = rows.iter().flatten().copied().collect();
    Ok(PyArray1::from_vec(py, values).reshape([N, M])?.into_any())
}

fn observation_dict<'py>(py: Python<'py>, env: &AutoplayEnv) -> PyResult<Bound<'py, PyDict>> {
    let obs = env.observe();
    let dict = PyDict::new(py);
    dict.set_item("self", PyArray1::from_slice(py, &obs.own))?;
    dict.set_item("enemies", flat(py, &obs.enemies)?)?;
    dict.set_item("enemy_mask", PyArray1::from_slice(py, &obs.enemy_mask))?;
    dict.set_item("hazards", flat(py, &obs.hazards)?)?;
    dict.set_item("hazard_mask", PyArray1::from_slice(py, &obs.hazard_mask))?;
    dict.set_item("available", PyArray1::from_slice(py, &obs.available))?;
    Ok(dict)
}

fn outcome_name(outcome: Option<Outcome>) -> Option<&'static str> {
    outcome.map(Outcome::name)
}

/// One native episode runner. Not thread-safe: use one per Python thread.
#[pyclass(name = "NativeEnv", unsendable)]
struct NativeEnv {
    env: AutoplayEnv,
    requested_seed: u64,
    sim_seed: u64,
    config: AutoplayConfig,
    assets_root: std::path::PathBuf,
    last_reset: Option<ResetInfo>,
    recording: Option<EpisodeRecording>,
}

impl NativeEnv {
    /// Non-privileged bookkeeping shared by `reset` and `step` (AP-Q08).
    fn base_info<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let info = PyDict::new(py);
        info.set_item("scenario_id", &self.config.scenario_id)?;
        info.set_item("tick", self.env.tick_count())?;
        info.set_item("requested_seed", self.requested_seed)?;
        info.set_item("sim_seed", self.sim_seed)?;
        info.set_item("seed_affects_sim", false)?;
        info.set_item("cf02_rules", self.config.cf02_rules)?;
        info.set_item("action_version", self.config.action_version)?;
        info.set_item("observation_version", OBSERVATION_VERSION)?;
        info.set_item("reward_id", &self.config.reward_id)?;
        info.set_item("max_ticks", self.config.max_ticks)?;
        info.set_item("action_repeat", self.config.action_repeat)?;
        let unavailable = self.env.unavailable_counters();
        let counters = PyDict::new(py);
        counters.set_item("special_held_ticks", unavailable.special_held_ticks)?;
        counters.set_item("special_presses", unavailable.special_presses)?;
        counters.set_item("swap_held_ticks", unavailable.swap_held_ticks)?;
        counters.set_item("swap_presses", unavailable.swap_presses)?;
        info.set_item("unavailable", counters)?;
        Ok(info)
    }
}

#[pymethods]
impl NativeEnv {
    #[new]
    #[pyo3(signature = (scenario_id, assets_root=None, max_ticks=1800, action_repeat=1, reward_id="combat-v1"))]
    fn new(
        scenario_id: &str,
        assets_root: Option<String>,
        max_ticks: u64,
        action_repeat: u32,
        reward_id: &str,
    ) -> PyResult<Self> {
        let config = build_config(scenario_id, max_ticks, action_repeat, reward_id);
        let root = bridge::resolve_assets_root(assets_root);
        let env = AutoplayEnv::new(config.clone(), &root).map_err(to_py_err)?;
        Ok(Self {
            env,
            requested_seed: 0,
            sim_seed: 0,
            config,
            assets_root: root,
            last_reset: None,
            recording: None,
        })
    }

    /// Start a fresh episode. Returns `(observation, info)`.
    fn reset<'py>(
        &mut self,
        py: Python<'py>,
        seed: u64,
    ) -> PyResult<(Bound<'py, PyDict>, Bound<'py, PyDict>)> {
        let reset = self.env.reset(seed).map_err(to_py_err)?;
        self.requested_seed = reset.requested_seed;
        self.sim_seed = reset.sim_seed;
        self.last_reset = Some(reset);
        self.recording = None;
        let info = self.base_info(py)?;
        Ok((observation_dict(py, &self.env)?, info))
    }

    /// Advance `action_repeat` ticks. Returns the Gymnasium five-tuple.
    #[allow(clippy::type_complexity)]
    fn step<'py>(
        &mut self,
        py: Python<'py>,
        action: Vec<i64>,
    ) -> PyResult<(Bound<'py, PyDict>, f64, bool, bool, Bound<'py, PyDict>)> {
        let action = ActionV1::from_slice(&action).map_err(to_py_err)?;
        let outcome = self.env.step(action).map_err(to_py_err)?;
        if let Some(recording) = self.recording.as_mut() {
            recording.on_step(&self.env, action, &outcome);
        }
        let info = self.base_info(py)?;
        info.set_item("ticks_executed", outcome.ticks_executed)?;
        info.set_item("outcome", outcome_name(outcome.outcome))?;
        info.set_item("terminal_reason", outcome_name(outcome.outcome))?;
        info.set_item("simultaneous_clear", outcome.simultaneous_clear)?;
        let reward = &outcome.reward;
        let breakdown = PyDict::new(py);
        breakdown.set_item("terminal", reward.terminal)?;
        breakdown.set_item("enemy_defeated", reward.enemy_defeated)?;
        breakdown.set_item("carl_hurt", reward.carl_hurt)?;
        breakdown.set_item("tick", reward.tick)?;
        breakdown.set_item("total", reward.total())?;
        breakdown.set_item("hurt_events", reward.hurt_events)?;
        breakdown.set_item("defeats", reward.defeats.len())?;
        info.set_item("reward_breakdown", breakdown)?;
        Ok((
            observation_dict(py, &self.env)?,
            reward.total(),
            outcome.terminated(),
            outcome.truncated(),
            info,
        ))
    }

    /// The current observation. Pure: never advances the simulation.
    fn observe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        observation_dict(py, &self.env)
    }

    /// Privileged: authoritative state hash (tests and parity only).
    fn state_hash(&self) -> u64 {
        self.env.state_hash()
    }

    /// Privileged: a few diagnostics for QA, never a policy input.
    fn diagnostics<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = self.env.diagnostics();
        let out = PyDict::new(py);
        out.set_item("tick", d.tick)?;
        out.set_item("state_hash", d.state_hash)?;
        out.set_item("carl_health", d.carl.health)?;
        out.set_item("defeated", d.defeated)?;
        out.set_item("hero_hurt_events", d.event_counts.hero_hurt)?;
        out.set_item("hits_carl", d.event_counts.hits_carl)?;
        out.set_item("derived_whiffs", d.event_counts.derived_whiffs)?;
        out.set_item("reward_total", d.reward_total)?;
        out.set_item("obs_clipped", d.obs.clipped)?;
        out.set_item("obs_unknown_modes", d.obs.unknown_modes)?;
        out.set_item("obs_overflow", d.obs.overflow)?;
        Ok(out)
    }

    /// Executed ticks of the current episode.
    fn tick_count(&self) -> u64 {
        self.env.tick_count()
    }

    /// Record the episode as a stage-B `autoplay-episode/1` artifact. Call
    /// right after `reset` (before the first `step`); needs `action_repeat=1`.
    /// The replay writer is the native one: this only forwards the steps.
    fn start_recording(&mut self, policy: &str, episode_label: &str, commit: &str) -> PyResult<()> {
        let reset = self.last_reset.as_ref().ok_or_else(|| {
            EpisodeOverError::new_err("start_recording needs a reset episode first")
        })?;
        if self.env.tick_count() != 0 {
            return Err(PyRuntimeError::new_err(
                "start_recording must be called right after reset, before the first step",
            ));
        }
        let kit_hash = kit_content_hash(&self.assets_root).map_err(to_py_err)?;
        self.recording = Some(
            EpisodeRecording::start(&self.env, reset, commit, kit_hash, policy, episode_label)
                .map_err(to_py_err)?,
        );
        Ok(())
    }

    /// The JSONL text of the finished recorded episode.
    fn finish_recording(&mut self) -> PyResult<String> {
        if self.env.outcome().is_none() {
            return Err(PyRuntimeError::new_err(
                "finish_recording needs a terminated or truncated episode",
            ));
        }
        let recording = self
            .recording
            .take()
            .ok_or_else(|| PyRuntimeError::new_err("no recording in progress"))?;
        Ok(recording.finish(&self.env).to_jsonl())
    }
}

/// A scripted Rust bot (stage B). The binding only forwards the observation
/// of a `NativeEnv` to the bot and returns its action; no bot logic lives in
/// Python.
#[pyclass(name = "NativeBot", unsendable)]
struct NativeBot {
    bot: Box<dyn Bot>,
}

#[pymethods]
impl NativeBot {
    #[new]
    fn new(policy_id: &str) -> PyResult<Self> {
        Ok(Self {
            bot: make_bot(policy_id).map_err(to_py_err)?,
        })
    }

    #[getter]
    fn policy_id(&self) -> &'static str {
        self.bot.id()
    }

    fn reset(&mut self, seed: u64) {
        self.bot.reset(seed);
    }

    /// The bot's action for the env's current observation.
    fn act(&mut self, env: PyRef<'_, NativeEnv>) -> Vec<i64> {
        let action = self.bot.act(&env.env.observe());
        action.as_array().iter().map(|&v| i64::from(v)).collect()
    }
}

/// The shipped scripted policy ids, in report order.
#[pyfunction]
fn bot_ids() -> Vec<&'static str> {
    BOT_IDS.to_vec()
}

/// The binding ObservationV1 schema as plain Python data (AP-Q06).
#[pyfunction]
fn observation_schema<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
    let schema = PyDict::new(py);
    schema.set_item("version", OBS_V1.version)?;
    schema.set_item("self_names", OBS_V1.self_names.to_vec())?;
    schema.set_item("self_bounds", OBS_V1.self_bounds.to_vec())?;
    schema.set_item("enemy_names", OBS_V1.enemy_names.to_vec())?;
    schema.set_item("enemy_bounds", OBS_V1.enemy_bounds.to_vec())?;
    schema.set_item("hazard_names", OBS_V1.hazard_names.to_vec())?;
    schema.set_item("hazard_bounds", OBS_V1.hazard_bounds.to_vec())?;
    schema.set_item("available_names", OBS_V1.available_names.to_vec())?;
    schema.set_item("self_len", SELF_LEN)?;
    schema.set_item("max_enemies", MAX_ENEMIES)?;
    schema.set_item("enemy_features", ENEMY_FEATURES)?;
    schema.set_item("max_hazards", MAX_HAZARDS)?;
    schema.set_item("hazard_features", HAZARD_FEATURES)?;
    schema.set_item("available_len", AVAILABLE_LEN)?;
    schema.set_item("action_len", ACTION_V1_LEN)?;
    schema.set_item("action_version", ACTION_VERSION_V1)?;
    Ok(schema)
}

/// The v1 scenario roster as `(id, "development" | "holdout")` pairs.
#[pyfunction]
fn scenario_roster<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
    let entries: Vec<(&str, &str)> = roster()
        .iter()
        .map(|e| {
            (
                e.id,
                match e.split {
                    Split::Development => "development",
                    Split::Holdout => "holdout",
                },
            )
        })
        .collect();
    PyList::new(py, entries)
}

/// Native reference run for the parity test: the `state_hash` after each step
/// of a plain `AutoplayEnv`, with no Python adapter in between.
#[pyfunction]
#[pyo3(signature = (scenario_id, actions, seed=0, assets_root=None, max_ticks=1800, action_repeat=1))]
fn reference_hashes(
    scenario_id: &str,
    actions: Vec<Vec<i64>>,
    seed: u64,
    assets_root: Option<String>,
    max_ticks: u64,
    action_repeat: u32,
) -> PyResult<Vec<u64>> {
    let config = build_config(scenario_id, max_ticks, action_repeat, "combat-v1");
    let mut parsed = Vec::with_capacity(actions.len());
    for action in &actions {
        let checked =
            dungeon_floorbreak::autoplay::ActionV1::from_slice(action).map_err(to_py_err)?;
        parsed.push(checked.as_array());
    }
    let root = bridge::resolve_assets_root(assets_root);
    bridge::reference_hashes(config, &root, seed, &parsed).map_err(to_py_err)
}

/// Pure-Rust loop for the benchmark: `(steps, seconds)` with observation reads.
#[pyfunction]
#[pyo3(signature = (scenario_id, actions, steps, assets_root=None))]
fn bench_native(
    scenario_id: &str,
    actions: Vec<Vec<i64>>,
    steps: usize,
    assets_root: Option<String>,
) -> PyResult<(usize, f64)> {
    let mut parsed = Vec::with_capacity(actions.len());
    for action in &actions {
        let checked =
            dungeon_floorbreak::autoplay::ActionV1::from_slice(action).map_err(to_py_err)?;
        parsed.push(checked.as_array());
    }
    let config = build_config(scenario_id, 1800, 1, "combat-v1");
    let root = bridge::resolve_assets_root(assets_root);
    bridge::bench_native(config, &root, &parsed, steps).map_err(to_py_err)
}

/// The default asset root baked in at build time (a checkout path).
#[pyfunction]
fn default_assets_root() -> String {
    bridge::default_assets_root().display().to_string()
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<NativeEnv>()?;
    m.add_class::<NativeBot>()?;
    m.add_function(wrap_pyfunction!(bot_ids, m)?)?;
    m.add("BOT_SETTINGS_VERSION", BOT_SETTINGS_VERSION)?;
    m.add_function(wrap_pyfunction!(observation_schema, m)?)?;
    m.add_function(wrap_pyfunction!(scenario_roster, m)?)?;
    m.add_function(wrap_pyfunction!(reference_hashes, m)?)?;
    m.add_function(wrap_pyfunction!(bench_native, m)?)?;
    m.add_function(wrap_pyfunction!(default_assets_root, m)?)?;
    m.add("EpisodeOverError", m.py().get_type::<EpisodeOverError>())?;
    m.add(
        "InvalidActionError",
        m.py().get_type::<InvalidActionError>(),
    )?;
    m.add("ASSETS_ROOT_ENV", bridge::ASSETS_ROOT_ENV)?;
    Ok(())
}
