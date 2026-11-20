//! Headless throughput benchmark (AUTOPLAY-01B).
//!
//! Measures simulated ticks per second of the native loop, never frame
//! rendering. Every mode runs the same scenario for the same number of
//! episodes and `repeat` times; the report keeps the median, minimum and
//! maximum, plus the build profile and the CPU, so a number is never quoted
//! without its conditions. There is no target and no promised multiplier.
//!
//! Modes (each includes the work of the previous one unless noted):
//! - `raw_fixture_sim`: `FixtureSim::step` directly, no env, no observation.
//! - `env_step`: `AutoplayEnv::step`. The env also folds an internal
//!   observation into its diagnostics counters on every step, so this is not
//!   observation-free; `raw_fixture_sim` is the observation-free floor.
//! - `env_step_observe`: `env_step` plus the policy `observe()` every tick.
//! - `bot_loop`: bot decision, step and observe: the evaluation hot path.
//! - `reset`: `AutoplayEnv::reset` calls per second (not ticks).
//!
//! Construction cost is inside the timed region: `raw_fixture_sim` builds a
//! `FixtureSim` for every episode and the env modes call `env.reset` for every
//! episode. The reported median of an even `repeat` is the mean of the two
//! middle values.

use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

use serde::Serialize;

use crate::combat_scene::FixtureSim;
use crate::env_kit::EnvKit;

use super::action::ActionV1;
use super::bots::{APPROACH_AND_ATTACK, Bot, SplitMix64, make_bot};
use super::config::AutoplayConfig;
use super::env::{AutoplayEnv, CORRIDOR_LAYOUT};
use super::scenario;

/// Desktop view height the headless fixture path uses (as in the env).
const VIEW_H: i32 = 360;
/// The real-time tick rate of the game.
pub const REAL_TIME_HZ: f64 = 60.0;

#[derive(Debug, Clone)]
pub struct BenchOptions {
    pub scenario: String,
    pub episodes: u32,
    pub repeat: u32,
    pub assets_root: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModeResult {
    pub mode: &'static str,
    /// Ticks (or resets for `reset`) executed per repetition.
    pub work_per_repeat: u64,
    pub per_second_median: f64,
    pub per_second_min: f64,
    pub per_second_max: f64,
    /// Median ticks per second divided by 60 (not meaningful for `reset`).
    pub real_time_factor_median: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchReport {
    pub schema: &'static str,
    pub build_profile: &'static str,
    pub cpu_model: String,
    pub os: &'static str,
    pub arch: &'static str,
    pub scenario: String,
    pub episodes: u32,
    pub repeat: u32,
    pub modes: Vec<ModeResult>,
    pub notes: Vec<&'static str>,
}

fn cpu_model() -> String {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|text| {
            text.lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "unknown".into())
}

/// A fixed legal action stream: the same input for every mode.
fn driver_actions(len: usize) -> Vec<ActionV1> {
    let mut rng = SplitMix64::new(0x00be_9c4a);
    (0..len)
        .map(|_| {
            ActionV1::new([
                rng.below(3) as u8,
                rng.below(2) as u8,
                rng.below(2) as u8,
                rng.below(2) as u8,
                0,
                0,
            ])
            .expect("driver actions are in range")
        })
        .collect()
}

fn summarise(mode: &'static str, work: u64, rates: &mut [f64], per_tick: bool) -> ModeResult {
    rates.sort_by(f64::total_cmp);
    // Even repeat counts average the two middle values.
    let mid = rates.len() / 2;
    let median = if rates.len().is_multiple_of(2) {
        (rates[mid - 1] + rates[mid]) / 2.0
    } else {
        rates[mid]
    };
    ModeResult {
        mode,
        work_per_repeat: work,
        per_second_median: median,
        per_second_min: rates[0],
        per_second_max: rates[rates.len() - 1],
        real_time_factor_median: if per_tick { median / REAL_TIME_HZ } else { 0.0 },
    }
}

/// Run every mode and return the report.
pub fn run(options: &BenchOptions) -> Result<BenchReport, String> {
    if options.episodes == 0 || options.repeat == 0 {
        return Err("--episodes and --repeat must be at least 1".into());
    }
    let entry = scenario::lookup(&options.scenario).map_err(|e| e.to_string())?;
    let config = AutoplayConfig::new(entry.id);
    let max_ticks = config.max_ticks as usize;
    let mut env = AutoplayEnv::new(config, &options.assets_root).map_err(|e| e.to_string())?;
    let kit = EnvKit::load(&options.assets_root, CORRIDOR_LAYOUT).map_err(|e| e.to_string())?;
    let actions = driver_actions(max_ticks);
    let err = |e: super::error::AutoplayError| e.to_string();

    // Pass 0 records the ticks every episode runs, so every mode does the
    // same work.
    let mut episode_ticks: Vec<u64> = Vec::new();
    for e in 0..options.episodes {
        env.reset(u64::from(e)).map_err(err)?;
        for action in &actions {
            if env.step(*action).map_err(err)?.outcome.is_some() {
                break;
            }
        }
        episode_ticks.push(env.tick_count());
    }
    let total_ticks: u64 = episode_ticks.iter().sum();

    let mut modes = Vec::new();

    let mut rates = Vec::new();
    for _ in 0..options.repeat {
        let start = Instant::now();
        let mut hash = 0;
        for ticks in &episode_ticks {
            let mut sim = FixtureSim::new(&kit, VIEW_H, entry.fixture)?;
            for action in actions.iter().take(*ticks as usize) {
                sim.step(action.to_raw_input());
            }
            hash ^= sim.state_hash();
        }
        black_box(hash);
        rates.push(total_ticks as f64 / start.elapsed().as_secs_f64());
    }
    modes.push(summarise("raw_fixture_sim", total_ticks, &mut rates, true));

    let mut rates = Vec::new();
    for _ in 0..options.repeat {
        let start = Instant::now();
        for e in 0..options.episodes {
            env.reset(u64::from(e)).map_err(err)?;
            for action in &actions {
                if env.step(*action).map_err(err)?.outcome.is_some() {
                    break;
                }
            }
        }
        black_box(env.state_hash());
        rates.push(total_ticks as f64 / start.elapsed().as_secs_f64());
    }
    modes.push(summarise("env_step", total_ticks, &mut rates, true));

    let mut rates = Vec::new();
    for _ in 0..options.repeat {
        let start = Instant::now();
        let mut sink = 0.0f32;
        for e in 0..options.episodes {
            env.reset(u64::from(e)).map_err(err)?;
            for action in &actions {
                let done = env.step(*action).map_err(err)?.outcome.is_some();
                sink += env.observe().own[0];
                if done {
                    break;
                }
            }
        }
        black_box(sink);
        rates.push(total_ticks as f64 / start.elapsed().as_secs_f64());
    }
    modes.push(summarise("env_step_observe", total_ticks, &mut rates, true));

    // The bot loop plays its own episodes, so it counts its own ticks.
    let mut bot: Box<dyn Bot> = make_bot(APPROACH_AND_ATTACK).map_err(err)?;
    let mut rates = Vec::new();
    let mut bot_ticks = 0;
    for _ in 0..options.repeat {
        let start = Instant::now();
        let mut ticks = 0;
        for e in 0..options.episodes {
            env.reset(u64::from(e)).map_err(err)?;
            bot.reset(u64::from(e));
            let mut obs = env.observe();
            loop {
                let step = env.step(bot.act(&obs)).map_err(err)?;
                if step.outcome.is_some() {
                    break;
                }
                obs = env.observe();
            }
            ticks += env.tick_count();
        }
        bot_ticks = ticks;
        rates.push(ticks as f64 / start.elapsed().as_secs_f64());
    }
    modes.push(summarise("bot_loop", bot_ticks, &mut rates, true));

    let resets = u64::from(options.episodes).max(100);
    let mut rates = Vec::new();
    for _ in 0..options.repeat {
        let start = Instant::now();
        for e in 0..resets {
            env.reset(e).map_err(err)?;
        }
        rates.push(resets as f64 / start.elapsed().as_secs_f64());
    }
    modes.push(summarise("reset", resets, &mut rates, false));

    Ok(BenchReport {
        schema: "autoplay-bench/1",
        build_profile: if cfg!(debug_assertions) {
            "debug (debug_assertions on: not a throughput number)"
        } else {
            "release (debug_assertions off)"
        },
        cpu_model: cpu_model(),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        scenario: entry.id.to_string(),
        episodes: options.episodes,
        repeat: options.repeat,
        modes,
        notes: vec![
            "Simulated ticks per second of the native loop on one thread; frame rendering is not measured.",
            "env_step includes the env's internal per-step observation bookkeeping; raw_fixture_sim is the observation-free floor.",
            "Single process, single thread; wall-clock numbers vary with machine load.",
            "Per-episode setup is timed: raw_fixture_sim includes FixtureSim::new and the env modes include env.reset. An even repeat reports the mean of the two middle values as the median.",
            "Tool versions (rustc, Bevy) are not embedded in this report; they are recorded in the PR body and Cargo.lock.",
        ],
    })
}

impl BenchReport {
    pub fn to_text(&self) -> String {
        let mut text = format!(
            "AUTOPLAY-01B benchmark: scenario {}, {} episodes x {} repeats\n  build: {}\n  cpu: {} ({} {})\n",
            self.scenario,
            self.episodes,
            self.repeat,
            self.build_profile,
            self.cpu_model,
            self.os,
            self.arch
        );
        for m in &self.modes {
            let unit = if m.mode == "reset" {
                "resets/s"
            } else {
                "ticks/s"
            };
            text.push_str(&format!(
                "  {:<18} median {:>12.0} {unit} (min {:.0}, max {:.0}, work {} per repeat){}\n",
                m.mode,
                m.per_second_median,
                m.per_second_min,
                m.per_second_max,
                m.work_per_repeat,
                if m.mode == "reset" {
                    String::new()
                } else {
                    format!(", {:.0}x real time", m.real_time_factor_median)
                }
            ));
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::summarise;

    #[test]
    fn the_median_of_an_even_repeat_averages_the_middle_values() {
        let mut even = [4.0, 1.0, 3.0, 2.0];
        assert_eq!(summarise("m", 1, &mut even, true).per_second_median, 2.5);
        let mut odd = [5.0, 1.0, 3.0];
        assert_eq!(summarise("m", 1, &mut odd, true).per_second_median, 3.0);
    }
}
