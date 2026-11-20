//! Versioned environment configuration (AP-Q05, AP-Q11, AP-Q12, AP-Q16).

use super::error::AutoplayError;
use super::party::{AutoplayMode, PARTY_REWARD_PENDING};
use super::reward::DEFAULT_REWARD_ID;
use crate::party::HeroId;

/// ActionV1: `MultiDiscrete([3, 2, 2, 2, 2, 2])` (AP-Q03).
pub const ACTION_VERSION_V1: u32 = 1;
/// 30 s at 60 Hz for every v1 scenario (AP-Q16).
pub const DEFAULT_MAX_TICKS: u64 = 1800;
/// Largest allowed `action_repeat` (AP-Q16).
pub const MAX_ACTION_REPEAT: u32 = 8;

/// Where the simulation seed comes from (AP-Q11). On this build no gameplay RNG
/// exists, so only [`SimSeed::Fixture`] is accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SimSeed {
    /// The fixture's own constant (the default).
    #[default]
    Fixture,
    /// A fixed value (rejected until CF02 lands).
    Fixed(u64),
    /// Taken from `reset(seed)` (rejected until CF02 lands).
    FromReset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoplayConfig {
    pub scenario_id: String,
    /// COMBAT-FUN-02 ruleset switch (AP-Q12). Must be `false` today.
    pub cf02_rules: bool,
    pub sim_seed: SimSeed,
    /// Truncation budget in executed ticks (keyed on `combat.elapsed`).
    pub max_ticks: u64,
    /// Ticks per `step` (1..=[`MAX_ACTION_REPEAT`]); buttons stay held across them.
    pub action_repeat: u32,
    pub action_version: u32,
    /// Reward profile id (`sparse-v1` or `combat-v1`).
    pub reward_id: String,
    /// `solo` (the v1 Carl-only env, the default) or `party_solo` (AP-Q22).
    pub mode: AutoplayMode,
    /// Observation schema version, derived from `mode`: 1 for `solo`, 2 for
    /// `party_solo` (AP-Q22). A value that disagrees with the mode is rejected.
    pub obs_version: u32,
}

impl AutoplayConfig {
    pub fn new(scenario_id: impl Into<String>) -> Self {
        Self {
            scenario_id: scenario_id.into(),
            cf02_rules: false,
            sim_seed: SimSeed::Fixture,
            max_ticks: DEFAULT_MAX_TICKS,
            action_repeat: 1,
            action_version: ACTION_VERSION_V1,
            reward_id: DEFAULT_REWARD_ID.to_string(),
            mode: AutoplayMode::Solo,
            obs_version: 1,
        }
    }

    /// A `party_solo` config (AP-Q22): the policy drives `seat`, the game's
    /// own AI drives the other hero. The party reward profile arrives with
    /// 02a-C, so the reward id is the pending marker until then.
    pub fn party_solo(
        scenario_id: impl Into<String>,
        seat: HeroId,
        companion_profile: impl Into<String>,
    ) -> Self {
        Self {
            reward_id: PARTY_REWARD_PENDING.to_string(),
            mode: AutoplayMode::PartySolo {
                seat,
                companion_profile: companion_profile.into(),
            },
            obs_version: 2,
            ..Self::new(scenario_id)
        }
    }

    /// Reject everything this build cannot honour, with a clear error.
    pub fn validate(&self) -> Result<(), AutoplayError> {
        if self.cf02_rules {
            return Err(AutoplayError::Cf02Unsupported);
        }
        if self.sim_seed != SimSeed::Fixture {
            return Err(AutoplayError::SimSeedUnsupported);
        }
        if self.action_version != ACTION_VERSION_V1 {
            return Err(AutoplayError::InvalidConfig(format!(
                "action_version {} is not supported (only {ACTION_VERSION_V1})",
                self.action_version
            )));
        }
        let expected_obs = self.mode.obs_version();
        if self.obs_version != expected_obs {
            return Err(AutoplayError::InvalidConfig(format!(
                "obs_version {} does not match mode {} (expected {expected_obs})",
                self.obs_version,
                self.mode.name()
            )));
        }
        let pending = self.reward_id == PARTY_REWARD_PENDING;
        if self.mode.is_party() != pending {
            return Err(AutoplayError::InvalidConfig(format!(
                "reward_id {} does not fit mode {}: party_solo needs a party reward \
                 profile (pending until 02a-C) and solo needs a v1 profile (AP-Q22)",
                self.reward_id,
                self.mode.name()
            )));
        }
        if self.max_ticks == 0 {
            return Err(AutoplayError::InvalidConfig(
                "max_ticks must be at least 1".into(),
            ));
        }
        if !(1..=MAX_ACTION_REPEAT).contains(&self.action_repeat) {
            return Err(AutoplayError::InvalidConfig(format!(
                "action_repeat {} is outside 1..={MAX_ACTION_REPEAT}",
                self.action_repeat
            )));
        }
        Ok(())
    }
}
