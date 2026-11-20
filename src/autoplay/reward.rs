//! RewardV1 (AP-Q14, AP-Q15): learning incentives, separate from the
//! evaluation metrics and from the authoritative simulator.
//!
//! The numbers are data (`assets/data/autoplay/reward_profiles_v1.json`,
//! embedded at compile time). Credit comes only from gameplay events: one
//! `EnemyDefeated` per enemy id (a repeated event, a replay or an action-repeat
//! bundle never pays twice) and `HeroHurt` of Carl. Cosmetic events (Donut
//! glances, mana, presentation) give nothing. The profile is not linked to any
//! HUD or audience scoring.

use std::collections::BTreeSet;

use serde::Deserialize;

use crate::combat::CombatEvent;
use crate::party::HeroId;

use super::env::Outcome;
use super::error::AutoplayError;

/// `reward_id` used when none is configured.
pub const DEFAULT_REWARD_ID: &str = "combat-v1";

const PROFILES_JSON: &str = include_str!("../../assets/data/autoplay/reward_profiles_v1.json");

/// One named reward profile: every number of the reward as data.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RewardProfile {
    pub id: String,
    pub success: f64,
    pub defeat: f64,
    pub abandoned: f64,
    pub timeout: f64,
    /// Per distinct required enemy defeated (once per id).
    pub enemy_defeated: f64,
    /// Per `HeroHurt` of Carl (the lethal hit counts too).
    pub carl_hurt: f64,
    /// Per executed tick.
    pub tick: f64,
}

#[derive(Debug, Deserialize)]
struct ProfileFile {
    version: u32,
    profiles: Vec<RewardProfile>,
}

impl RewardProfile {
    /// All profiles shipped with RewardV1.
    pub fn all() -> Vec<RewardProfile> {
        let file: ProfileFile = serde_json::from_str(PROFILES_JSON)
            .expect("reward_profiles_v1.json is embedded and valid");
        assert_eq!(file.version, 1, "reward profile file version");
        file.profiles
    }

    pub fn by_id(id: &str) -> Result<RewardProfile, AutoplayError> {
        let profiles = Self::all();
        let known: Vec<String> = profiles.iter().map(|p| p.id.clone()).collect();
        profiles.into_iter().find(|p| p.id == id).ok_or_else(|| {
            AutoplayError::InvalidConfig(format!(
                "unknown reward_id {id} (known: {})",
                known.join(", ")
            ))
        })
    }

    fn terminal(&self, outcome: Outcome) -> f64 {
        match outcome {
            Outcome::Success => self.success,
            Outcome::Defeat => self.defeat,
            Outcome::Abandoned => self.abandoned,
            Outcome::Timeout => self.timeout,
        }
    }
}

/// Who a defeat is credited to, from the event's own `instigator` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attribution {
    Carl,
    Donut,
    /// No instigator, or a player id this build does not know.
    Unknown,
}

impl Attribution {
    pub fn from_instigator(instigator: Option<u32>) -> Self {
        match instigator {
            Some(0) => Self::Carl,
            Some(1) => Self::Donut,
            _ => Self::Unknown,
        }
    }
}

/// One credited defeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefeatCredit {
    pub target: u32,
    pub attribution: Attribution,
}

/// The reward of one `step`, itemised (exposed in `info`, AP-Q08).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RewardBreakdown {
    pub terminal: f64,
    pub enemy_defeated: f64,
    pub carl_hurt: f64,
    pub tick: f64,
    pub defeats: Vec<DefeatCredit>,
    pub hurt_events: u32,
}

impl RewardBreakdown {
    /// Sum of the parts, in a fixed order (deterministic float result).
    pub fn total(&self) -> f64 {
        self.terminal + self.enemy_defeated + self.carl_hurt + self.tick
    }
}

/// Per-episode reward state: which enemies were already credited.
#[derive(Debug, Clone)]
pub struct RewardTracker {
    profile: RewardProfile,
    required: BTreeSet<u32>,
    credited: BTreeSet<u32>,
    total: f64,
}

impl RewardTracker {
    pub fn new(profile: RewardProfile, required: &[u32]) -> Self {
        Self {
            profile,
            required: required.iter().copied().collect(),
            credited: BTreeSet::new(),
            total: 0.0,
        }
    }

    pub fn profile(&self) -> &RewardProfile {
        &self.profile
    }

    /// Return reward for the events of one step (`events` are the ordered
    /// events of every executed tick), `ticks` executed ticks and the
    /// episode outcome if it ended on this step.
    pub fn step(
        &mut self,
        events: &[CombatEvent],
        ticks: u32,
        outcome: Option<Outcome>,
    ) -> RewardBreakdown {
        let mut breakdown = RewardBreakdown::default();
        for event in events {
            match event {
                CombatEvent::EnemyDefeated {
                    target, instigator, ..
                } if self.required.contains(target) && self.credited.insert(*target) => {
                    breakdown.defeats.push(DefeatCredit {
                        target: *target,
                        attribution: Attribution::from_instigator(*instigator),
                    });
                }
                CombatEvent::HeroHurt {
                    hero: HeroId::Carl, ..
                } => breakdown.hurt_events += 1,
                // Everything else (hits, tells, cosmetic and future events)
                // is not reward.
                _ => {}
            }
        }
        breakdown.enemy_defeated = self.profile.enemy_defeated * breakdown.defeats.len() as f64;
        breakdown.carl_hurt = self.profile.carl_hurt * f64::from(breakdown.hurt_events);
        breakdown.tick = self.profile.tick * f64::from(ticks);
        if let Some(outcome) = outcome {
            breakdown.terminal = self.profile.terminal(outcome);
        }
        self.total += breakdown.total();
        breakdown
    }

    /// Return of the episode so far.
    pub fn total(&self) -> f64 {
        self.total
    }

    /// Enemy ids already credited.
    pub fn credited(&self) -> &BTreeSet<u32> {
        &self.credited
    }
}
