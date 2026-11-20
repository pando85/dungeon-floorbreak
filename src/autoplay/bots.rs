//! Three scripted, deterministic baseline policies (AUTOPLAY-01B, AP-Q08).
//!
//! A [`Bot`] is a pure function of [`ObservationV1`] plus its own state and a
//! local seeded PRNG. It never receives diagnostics, the env, the fixture or
//! any future script input: the trait signature is the structural guarantee,
//! and a test also scans this file's source for privileged identifiers.
//!
//! None of these is a human skill proxy. `random_legal` is a smoke and
//! coverage policy; the other two are intentionally imperfect baselines for
//! comparing game commits on a pinned roster. Their settings are pinned by
//! [`BOT_SETTINGS_VERSION`]; changing any threshold below must bump it.

use super::action::ActionV1;
use super::error::AutoplayError;
use super::observation::{
    ENEMY_DAMAGED, ENEMY_DX, ENEMY_DY, ENEMY_RECOVER, ENEMY_STAGGERED, ENEMY_SWING, ENEMY_TELL,
    ObservationV1, SELF_BUSY, SELF_FACING, SELF_GROUNDED,
};

/// Version of the pinned thresholds and sampling rules of all three bots.
pub const BOT_SETTINGS_VERSION: u32 = 1;

pub const RANDOM_LEGAL: &str = "random_legal";
pub const APPROACH_AND_ATTACK: &str = "approach_and_attack";
pub const REACTIVE_SURVIVOR: &str = "reactive_survivor";

/// Every shipped policy id, in report order.
pub const BOT_IDS: [&str; 3] = [RANDOM_LEGAL, APPROACH_AND_ATTACK, REACTIVE_SURVIVOR];

/// Horizontal scale of the `dx` feature (the sim's 640 px camera frame).
const VIEW_W_PX: f32 = 640.0;

const LEFT: u8 = 0;
const NEUTRAL: u8 = 1;
const RIGHT: u8 = 2;

/// A policy. The only input is the player-like observation.
pub trait Bot {
    fn id(&self) -> &'static str;
    /// Start a new episode. `seed` only matters to policies that sample.
    fn reset(&mut self, seed: u64);
    /// Choose the action for the next tick from the current observation.
    fn act(&mut self, obs: &ObservationV1) -> ActionV1;
}

/// Build a policy by id.
pub fn make_bot(id: &str) -> Result<Box<dyn Bot>, AutoplayError> {
    match id {
        RANDOM_LEGAL => Ok(Box::new(RandomLegal::new())),
        APPROACH_AND_ATTACK => Ok(Box::new(ApproachAndAttack::new())),
        REACTIVE_SURVIVOR => Ok(Box::new(ReactiveSurvivor::new())),
        other => Err(AutoplayError::InvalidConfig(format!(
            "unknown policy {other} (known: {})",
            BOT_IDS.join(", ")
        ))),
    }
}

/// SplitMix64: a tiny deterministic generator (no new crate).
#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n` (`n > 0`); the modulo bias is irrelevant at these sizes.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

/// The nearest perceived enemy's `(dx_px, dy, row)`; slot 0 is the nearest by
/// the observation's own deterministic ordering.
fn nearest(obs: &ObservationV1) -> Option<(f32, &[f32])> {
    if obs.enemy_mask[0] > 0.0 {
        let row = &obs.enemies[0];
        Some((row[ENEMY_DX] * VIEW_W_PX, row.as_slice()))
    } else {
        None
    }
}

fn toward(dx_px: f32) -> u8 {
    if dx_px < 0.0 { LEFT } else { RIGHT }
}

fn away(dx_px: f32) -> u8 {
    if dx_px < 0.0 { RIGHT } else { LEFT }
}

fn facing_enemy(obs: &ObservationV1, dx_px: f32) -> bool {
    (obs.own[SELF_FACING] < 0.0) == (dx_px < 0.0)
}

fn action(horizontal: u8, jump: bool, primary: bool, physical: bool) -> ActionV1 {
    ActionV1::new([
        horizontal,
        u8::from(jump),
        u8::from(primary),
        u8::from(physical),
        0,
        0,
    ])
    .expect("bot actions are always in range")
}

// ------------------------------------------------------------ random_legal --

/// Uniform sampling of every component that the observation says is
/// available. Reserved components (special, swap) stay off while
/// `available[i] == 0`: sampling them would be sampling illegal actions.
#[derive(Debug, Clone)]
pub struct RandomLegal {
    rng: SplitMix64,
}

impl RandomLegal {
    pub fn new() -> Self {
        Self {
            rng: SplitMix64::new(0),
        }
    }
}

impl Default for RandomLegal {
    fn default() -> Self {
        Self::new()
    }
}

impl Bot for RandomLegal {
    fn id(&self) -> &'static str {
        RANDOM_LEGAL
    }

    fn reset(&mut self, seed: u64) {
        self.rng = SplitMix64::new(seed);
    }

    fn act(&mut self, obs: &ObservationV1) -> ActionV1 {
        let horizontal = self.rng.below(3) as u8;
        let jump = self.rng.below(2) as u8;
        let primary = self.rng.below(2) as u8;
        let physical = self.rng.below(2) as u8;
        // Always draw both so the stream does not depend on availability.
        let special = self.rng.below(2) as u8;
        let swap = self.rng.below(2) as u8;
        let special = if obs.available[0] > 0.0 { special } else { 0 };
        let swap = if obs.available[1] > 0.0 { swap } else { 0 };
        ActionV1::new([horizontal, jump, primary, physical, special, swap])
            .expect("sampled components are in range")
    }
}

// ------------------------------------------------------ approach_and_attack --

/// Walk at the nearest visible enemy, face it, pulse the primary button and
/// kick now and then. Jumps rarely and never reads a tell. Imperfect by design.
#[derive(Debug, Clone)]
pub struct ApproachAndAttack {
    tick: u32,
    jump_cooldown: u32,
}

/// Distance (px) at which it stops walking and starts striking.
const AA_STRIKE_PX: f32 = 26.0;
/// It jumps once when the enemy is this far away (px, inclusive range).
const AA_JUMP_MIN_PX: f32 = 90.0;
const AA_JUMP_MAX_PX: f32 = 150.0;
/// Ticks between jumps.
const AA_JUMP_COOLDOWN: u32 = 240;
/// The primary button is pressed on one tick of every this many.
const AA_PULSE_PERIOD: u32 = 6;
/// It kicks instead of jabbing on one strike pulse of every this many.
const AA_KICK_EVERY: u32 = 4;

impl ApproachAndAttack {
    pub fn new() -> Self {
        Self {
            tick: 0,
            jump_cooldown: 0,
        }
    }
}

impl Default for ApproachAndAttack {
    fn default() -> Self {
        Self::new()
    }
}

impl Bot for ApproachAndAttack {
    fn id(&self) -> &'static str {
        APPROACH_AND_ATTACK
    }

    fn reset(&mut self, _seed: u64) {
        *self = Self::new();
    }

    fn act(&mut self, obs: &ObservationV1) -> ActionV1 {
        self.tick += 1;
        self.jump_cooldown = self.jump_cooldown.saturating_sub(1);
        let Some((dx_px, _)) = nearest(obs) else {
            // Nothing in view: keep walking right, the way the corridor runs.
            return action(RIGHT, false, false, false);
        };
        let dist = dx_px.abs();
        if dist > AA_STRIKE_PX {
            let can_jump = self.jump_cooldown == 0
                && obs.own[SELF_GROUNDED] > 0.0
                && (AA_JUMP_MIN_PX..=AA_JUMP_MAX_PX).contains(&dist);
            if can_jump {
                self.jump_cooldown = AA_JUMP_COOLDOWN;
            }
            return action(toward(dx_px), can_jump, false, false);
        }
        if !facing_enemy(obs, dx_px) {
            return action(toward(dx_px), false, false, false);
        }
        let phase = self.tick % AA_PULSE_PERIOD;
        let pulse = phase == 0;
        let kick = pulse && (self.tick / AA_PULSE_PERIOD) % AA_KICK_EVERY == AA_KICK_EVERY - 1;
        action(NEUTRAL, false, pulse && !kick, kick)
    }
}

// -------------------------------------------------------- reactive_survivor --

/// Advance when nothing is in view, keep a spacing, back off when an enemy winds up a swing close by, and
/// attack only when an enemy is staggered, recovering or has just been hurt.
/// It reads the visible tell flags of the observation and nothing else.
#[derive(Debug, Clone)]
pub struct ReactiveSurvivor {
    tick: u32,
}

/// Preferred distance (px) while waiting for an opening.
const RS_SPACING_PX: f32 = 56.0;
/// Inside this distance (px) a tell or swing makes it back off.
const RS_DANGER_PX: f32 = 60.0;
/// Strike range (px) for a jab on an opening.
const RS_STRIKE_PX: f32 = 26.0;
/// The primary button is pressed on one tick of every this many.
const RS_PULSE_PERIOD: u32 = 4;

impl ReactiveSurvivor {
    pub fn new() -> Self {
        Self { tick: 0 }
    }
}

impl Default for ReactiveSurvivor {
    fn default() -> Self {
        Self::new()
    }
}

impl Bot for ReactiveSurvivor {
    fn id(&self) -> &'static str {
        REACTIVE_SURVIVOR
    }

    fn reset(&mut self, _seed: u64) {
        *self = Self::new();
    }

    fn act(&mut self, obs: &ObservationV1) -> ActionV1 {
        self.tick += 1;
        let Some((dx_px, row)) = nearest(obs) else {
            // Nothing in view: advance, the way the corridor runs. Hostiles
            // off screen do not come to Carl, so waiting would stall.
            return action(RIGHT, false, false, false);
        };
        let dist = dx_px.abs();
        let winding_up = row[ENEMY_TELL] > 0.0 || row[ENEMY_SWING] > 0.0;
        let opening = row[ENEMY_RECOVER] > 0.0
            || row[ENEMY_STAGGERED] > 0.0
            || (row[ENEMY_DAMAGED] > 0.0 && row[ENEMY_DY].abs() < 0.2);
        if winding_up && dist < RS_DANGER_PX {
            // A jab startup is 3 ticks: cancel a tell that is already in
            // reach, back off from anything else.
            let in_reach = dist <= RS_STRIKE_PX && facing_enemy(obs, dx_px);
            if row[ENEMY_TELL] > 0.0 && in_reach && obs.own[SELF_BUSY] == 0.0 {
                return action(
                    NEUTRAL,
                    false,
                    self.tick.is_multiple_of(RS_PULSE_PERIOD),
                    false,
                );
            }
            return action(away(dx_px), false, false, false);
        }
        if opening && dist <= RS_STRIKE_PX {
            if !facing_enemy(obs, dx_px) {
                return action(toward(dx_px), false, false, false);
            }
            return action(
                NEUTRAL,
                false,
                self.tick.is_multiple_of(RS_PULSE_PERIOD),
                false,
            );
        }
        if opening && dist > RS_STRIKE_PX {
            return action(toward(dx_px), false, false, false);
        }
        if dist > RS_SPACING_PX + 8.0 {
            return action(toward(dx_px), false, false, false);
        }
        if dist < RS_SPACING_PX - 8.0 {
            return action(away(dx_px), false, false, false);
        }
        action(NEUTRAL, false, false, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix64_matches_reference_vector() {
        // Reference outputs of SplitMix64 seeded with 0.
        let mut rng = SplitMix64::new(0);
        assert_eq!(rng.next_u64(), 0xe220_a839_7b1d_cdaf);
        assert_eq!(rng.next_u64(), 0x6e78_9e6a_a1b9_65f4);
        assert_eq!(rng.next_u64(), 0x06c4_5d18_8009_454f);
    }

    #[test]
    fn unknown_policy_is_rejected() {
        assert!(make_bot("expert_human").is_err());
        for id in BOT_IDS {
            assert_eq!(make_bot(id).unwrap().id(), id);
        }
    }
}
