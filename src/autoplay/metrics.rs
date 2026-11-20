//! Event-driven episode metrics and their aggregate (AUTOPLAY-01B, AP-Q14).
//!
//! Everything here is derived from the ordered events the env drained per
//! tick, the executed actions and the env's own counters. Attribution follows
//! AP-Q14: a defeat is credited through `EnemyDefeated.instigator`
//! (0 carl, 1 donut, `None` unknown), a `Hit` through the family of its
//! `move_id`, and what the event set cannot say is reported as
//! `not_observable` or `derived`, never guessed.
//!
//! Per-hero data is keyed by hero id (`carl` today), so a later party mode can
//! add the companion without a schema break. Output is plain data: no wall
//! clock, no paths, so two runs with the same arguments are byte-identical.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::combat::{CombatEvent, HERO_HEALTH, MoveId};
use crate::party::HeroId;

use super::action::ActionV1;
use super::diagnostics::move_attribution;
use super::env::{AutoplayEnv, Outcome, StepOutcome};
use super::observation::{ObservationV1, SELF_VX};
use super::reward::Attribution;

/// Version of the per-episode and aggregate metric schema.
pub const METRICS_SCHEMA: &str = "autoplay-metrics/1";

/// A timeout with no hit, defeat or hurt event in this many final ticks is
/// flagged `no_progress` (derived from events, not a game rule).
pub const NO_PROGRESS_WINDOW_TICKS: u64 = 600;

/// Things the event set cannot tell today; reported honestly in every manifest.
pub const NOT_OBSERVABLE: [&str; 4] = [
    "terrain_hazard_damage_to_carl",
    "enemy_body_contact_with_carl",
    "swaps_and_companion_assistance",
    "causes_of_enemy_hurt_by_non_melee_sources",
];

/// Hero ids as they appear in output keys.
pub fn hero_key(hero: HeroId) -> &'static str {
    match hero {
        HeroId::Carl => "carl",
        HeroId::Donut => "donut",
    }
}

fn attribution_key(attribution: Attribution) -> &'static str {
    match attribution {
        Attribution::Carl => "carl",
        Attribution::Donut => "donut",
        Attribution::Unknown => "unknown",
    }
}

/// Per-move counters of one hero.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct MoveStats {
    /// `MoveStarted` events.
    pub attempts: u32,
    /// Distinct move instances with at least one `Hit`.
    pub connected: u32,
    /// `Hit` events (one per target per instance).
    pub hits: u32,
    /// Derived: attempts whose instance never produced a `Hit`
    /// (`derived`, AP-Q14). A move still running at the end counts.
    pub whiffs_derived: u32,
}

/// One hero's metrics, keyed by hero id in [`EpisodeMetrics::heroes`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct HeroMetrics {
    /// `HeroHurt` events for this hero (the lethal hit is logged too).
    pub hurt_events: u32,
    /// Health lost, from `health_left` deltas of the hero's `HeroHurt` events.
    pub damage_received: u32,
    pub defeated: bool,
    pub moves: BTreeMap<String, MoveStats>,
}

/// Per-component action usage of an episode.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ActionUsage {
    pub ticks: u64,
    pub left_ticks: u64,
    pub neutral_ticks: u64,
    pub right_ticks: u64,
    pub jump_ticks: u64,
    pub primary_ticks: u64,
    pub physical_ticks: u64,
    pub primary_presses: u64,
    pub physical_presses: u64,
    pub jump_presses: u64,
    /// Reserved component 4: no Carl behaviour on this build.
    pub special_unavailable_ticks: u64,
    pub special_unavailable_presses: u64,
    /// Reserved component 5: ignored by the sim.
    pub swap_unavailable_ticks: u64,
    pub swap_unavailable_presses: u64,
    /// Ticks with a neutral stick and no button (a deliberate no-op).
    pub idle_ticks: u64,
    /// Share of ticks spent on the single most used action tuple.
    pub modal_action_share: f64,
}

/// Where Carl stood when the episode ended (privileged diagnostics).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TerminalPosition {
    pub x: f32,
    pub y: f32,
}

/// The itemised reward of a whole episode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct RewardTotals {
    pub terminal: f64,
    pub enemy_defeated: f64,
    pub carl_hurt: f64,
    pub tick: f64,
}

/// Who and what was needed to produce an episode.
#[derive(Debug, Clone)]
pub struct EpisodeContext {
    pub episode: u32,
    pub scenario: String,
    pub split: String,
    pub policy: String,
    pub requested_seed: u64,
    /// Replay path relative to the output directory, when one was written.
    pub replay: Option<String>,
    pub action_hash: u64,
    pub required_enemies: usize,
}

/// Every metric of one episode.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EpisodeMetrics {
    pub schema: &'static str,
    pub episode: u32,
    pub scenario: String,
    pub split: String,
    pub policy: String,
    pub requested_seed: u64,
    /// `success`, `defeat`, `abandoned` or `timeout`.
    pub outcome: &'static str,
    pub simultaneous_clear: bool,
    pub ticks: u64,
    pub return_total: f64,
    pub reward: RewardTotals,
    pub required_enemies: usize,
    pub enemies_defeated: u32,
    /// Defeat credit by instigator: `carl`, `donut`, `unknown`.
    pub defeats_by_attribution: BTreeMap<String, u32>,
    /// Distinct defeat causes (debug name of the event's cause) with counts.
    pub defeat_causes: BTreeMap<String, u32>,
    pub heroes: BTreeMap<String, HeroMetrics>,
    /// Hits whose move family is not Carl's (donut or unknown), counted once.
    pub hits_other_attribution: u32,
    /// `Hit`s of a launching move that left the target alive (derived).
    pub knockbacks_derived: u32,
    /// `GoblinContact` events (a launched goblin touching another).
    pub goblin_contacts: u32,
    pub other_events: u32,
    pub action_usage: ActionUsage,
    /// Ticks Carl's horizontal speed was zero (observation `vx == 0`).
    pub standing_still_ticks: u64,
    /// Longest run of ticks without a hit, defeat or hurt event.
    pub longest_no_progress_ticks: u64,
    /// Derived flag: a timeout whose last window had no progress event.
    pub no_progress: bool,
    pub terminal_position: TerminalPosition,
    pub obs_clipped: u32,
    /// `kind:count` fingerprint of action families and contact outcomes.
    pub strategy_fingerprint: String,
    pub final_state_hash: String,
    pub event_hash: String,
    /// Hash of the whole action trace plus the final hashes: two episodes
    /// with equal values are the same trajectory.
    pub trajectory_hash: String,
    pub replay: Option<String>,
}

/// FNV-1a over a stream of `u64`, the repository's own event-hash recipe.
pub fn fnv_mix(hash: u64, value: u64) -> u64 {
    let mut hash = hash;
    for byte in value.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

pub const FNV_SEED: u64 = 0xcbf2_9ce4_8422_2325;

/// Rolling hash of the ordered events, folded with `CombatEvent::mix_into`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventHasher {
    hash: u64,
}

impl EventHasher {
    pub fn new() -> Self {
        Self { hash: FNV_SEED }
    }

    pub fn fold(&mut self, events: &[CombatEvent]) {
        let mut hash = self.hash;
        for event in events {
            event.mix_into(&mut |value| hash = fnv_mix(hash, value));
        }
        self.hash = hash;
    }

    pub fn value(&self) -> u64 {
        self.hash
    }
}

impl Default for EventHasher {
    fn default() -> Self {
        Self::new()
    }
}

/// Folds one episode's steps into [`EpisodeMetrics`].
#[derive(Debug, Clone)]
pub struct EpisodeCollector {
    usage: ActionUsage,
    action_counts: BTreeMap<[u8; 6], u64>,
    previous: Option<ActionV1>,
    reward: RewardTotals,
    started: Vec<(MoveId, u32)>,
    hit_instances: BTreeSet<u32>,
    move_hits: BTreeMap<&'static str, u32>,
    defeated: BTreeSet<u32>,
    defeats_by_attribution: BTreeMap<String, u32>,
    defeat_causes: BTreeMap<String, u32>,
    hero_hurt: BTreeMap<&'static str, (u32, u32)>,
    hero_health: BTreeMap<&'static str, u8>,
    hero_defeated: BTreeSet<&'static str>,
    hits_other_attribution: u32,
    knockbacks: u32,
    goblin_contacts: u32,
    other_events: u32,
    standing_still: u64,
    last_progress_tick: u64,
    longest_gap: u64,
    events: EventHasher,
    simultaneous_clear: bool,
}

impl EpisodeCollector {
    pub fn new() -> Self {
        Self {
            usage: ActionUsage::default(),
            action_counts: BTreeMap::new(),
            previous: None,
            reward: RewardTotals::default(),
            started: Vec::new(),
            hit_instances: BTreeSet::new(),
            move_hits: BTreeMap::new(),
            defeated: BTreeSet::new(),
            defeats_by_attribution: BTreeMap::new(),
            defeat_causes: BTreeMap::new(),
            hero_hurt: BTreeMap::new(),
            hero_health: BTreeMap::new(),
            hero_defeated: BTreeSet::new(),
            hits_other_attribution: 0,
            knockbacks: 0,
            goblin_contacts: 0,
            other_events: 0,
            standing_still: 0,
            last_progress_tick: 0,
            longest_gap: 0,
            events: EventHasher::new(),
            simultaneous_clear: false,
        }
    }

    /// Rolling event hash so far.
    pub fn event_hash(&self) -> u64 {
        self.events.value()
    }

    /// Fold one executed `step`. `obs_before` is what the policy saw when it
    /// chose `action` (used for the standing-still counter only).
    /// `new_events` are the events of this step in order, read from the env's
    /// crate-internal episode log (`StepOutcome` carries no raw events).
    pub fn on_step(
        &mut self,
        obs_before: &ObservationV1,
        action: ActionV1,
        step: &StepOutcome,
        new_events: &[CombatEvent],
    ) {
        let ticks = u64::from(step.ticks_executed);
        self.record_action(action, ticks);
        if obs_before.own[SELF_VX] == 0.0 {
            self.standing_still += ticks;
        }
        self.reward.terminal += step.reward.terminal;
        self.reward.enemy_defeated += step.reward.enemy_defeated;
        self.reward.carl_hurt += step.reward.carl_hurt;
        self.reward.tick += step.reward.tick;
        self.events.fold(new_events);
        self.simultaneous_clear |= step.simultaneous_clear;
        let mut progress = false;
        for event in new_events {
            progress |= self.on_event(event);
        }
        if progress {
            self.longest_gap = self
                .longest_gap
                .max(step.tick.saturating_sub(self.last_progress_tick + 1));
            self.last_progress_tick = step.tick;
        }
    }

    fn record_action(&mut self, action: ActionV1, ticks: u64) {
        let u = &mut self.usage;
        u.ticks += ticks;
        match action.horizontal() {
            0 => u.left_ticks += ticks,
            2 => u.right_ticks += ticks,
            _ => u.neutral_ticks += ticks,
        }
        let was = |f: fn(&ActionV1) -> bool| self.previous.as_ref().is_some_and(f);
        if action.jump() {
            u.jump_ticks += ticks;
            u.jump_presses += u64::from(!was(ActionV1::jump));
        }
        if action.primary() {
            u.primary_ticks += ticks;
            u.primary_presses += u64::from(!was(ActionV1::primary));
        }
        if action.physical() {
            u.physical_ticks += ticks;
            u.physical_presses += u64::from(!was(ActionV1::physical));
        }
        if action.special() {
            u.special_unavailable_ticks += ticks;
            u.special_unavailable_presses += u64::from(!was(ActionV1::special));
        }
        if action.swap() {
            u.swap_unavailable_ticks += ticks;
            u.swap_unavailable_presses += u64::from(!was(ActionV1::swap));
        }
        if action.horizontal() == 1 && action.as_array()[1..].iter().all(|v| *v == 0) {
            u.idle_ticks += ticks;
        }
        *self.action_counts.entry(action.as_array()).or_insert(0) += ticks;
        self.previous = Some(action);
    }

    /// Returns whether the event counts as progress (hit, defeat, hurt).
    fn on_event(&mut self, event: &CombatEvent) -> bool {
        match event {
            CombatEvent::MoveStarted {
                move_id, instance, ..
            } => {
                self.started.push((*move_id, *instance));
                false
            }
            CombatEvent::Hit {
                move_id,
                instance,
                health_left,
                ..
            } => {
                self.hit_instances.insert(*instance);
                if move_attribution(*move_id) == Attribution::Carl {
                    *self.move_hits.entry(move_id.name()).or_insert(0) += 1;
                } else {
                    self.hits_other_attribution += 1;
                }
                if *move_id == MoveId::PushKick && *health_left > 0 {
                    self.knockbacks += 1;
                }
                true
            }
            CombatEvent::EnemyDefeated {
                target,
                cause,
                instigator,
                ..
            } => {
                if self.defeated.insert(*target) {
                    let key = attribution_key(Attribution::from_instigator(*instigator));
                    *self.defeats_by_attribution.entry(key.into()).or_insert(0) += 1;
                    *self
                        .defeat_causes
                        .entry(cause_name(&format!("{cause:?}")))
                        .or_insert(0) += 1;
                }
                true
            }
            CombatEvent::HeroHurt {
                hero, health_left, ..
            } => {
                let key = hero_key(*hero);
                let before = *self.hero_health.entry(key).or_insert(HERO_HEALTH);
                let entry = self.hero_hurt.entry(key).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += u32::from(before.saturating_sub(*health_left));
                self.hero_health.insert(key, *health_left);
                true
            }
            CombatEvent::HeroDefeated { hero, .. } => {
                self.hero_defeated.insert(hero_key(*hero));
                true
            }
            CombatEvent::GoblinContact { .. } => {
                self.goblin_contacts += 1;
                false
            }
            _ => {
                self.other_events += 1;
                false
            }
        }
    }

    /// Close the episode against the env's final state.
    pub fn finish(mut self, env: &AutoplayEnv, ctx: EpisodeContext) -> EpisodeMetrics {
        let diag = env.diagnostics();
        let outcome = env.outcome().unwrap_or(Outcome::Timeout);
        let ticks = env.tick_count();
        self.longest_gap = self
            .longest_gap
            .max(ticks.saturating_sub(self.last_progress_tick));
        let unavailable = env.unavailable_counters();
        self.usage.special_unavailable_ticks = unavailable.special_held_ticks;
        self.usage.special_unavailable_presses = unavailable.special_presses;
        self.usage.swap_unavailable_ticks = unavailable.swap_held_ticks;
        self.usage.swap_unavailable_presses = unavailable.swap_presses;
        let modal = self.action_counts.values().copied().max().unwrap_or(0);
        self.usage.modal_action_share = if self.usage.ticks == 0 {
            0.0
        } else {
            modal as f64 / self.usage.ticks as f64
        };

        let mut heroes: BTreeMap<String, HeroMetrics> = BTreeMap::new();
        let carl = heroes.entry("carl".into()).or_default();
        for (move_id, _) in &self.started {
            if move_attribution(*move_id) == Attribution::Carl {
                carl.moves
                    .entry(move_id.name().into())
                    .or_default()
                    .attempts += 1;
            }
        }
        let mut seen_instances = BTreeSet::new();
        for (move_id, instance) in &self.started {
            if move_attribution(*move_id) != Attribution::Carl {
                continue;
            }
            let stats = carl.moves.entry(move_id.name().into()).or_default();
            if self.hit_instances.contains(instance) {
                if seen_instances.insert(*instance) {
                    stats.connected += 1;
                }
            } else {
                stats.whiffs_derived += 1;
            }
        }
        for (name, hits) in &self.move_hits {
            carl.moves.entry((*name).into()).or_default().hits += hits;
        }
        for (key, (events, damage)) in &self.hero_hurt {
            let hero = heroes.entry((*key).into()).or_default();
            hero.hurt_events = *events;
            hero.damage_received = *damage;
        }
        for key in &self.hero_defeated {
            heroes.entry((*key).into()).or_default().defeated = true;
        }

        let no_progress = outcome == Outcome::Timeout
            && ticks.saturating_sub(self.last_progress_tick) >= NO_PROGRESS_WINDOW_TICKS;
        let fingerprint = fingerprint(&self.usage, &heroes, self.defeated.len());
        let state_hash = env.state_hash();
        let event_hash = self.events.value();
        let trajectory_hash = fnv_mix(fnv_mix(ctx.action_hash, state_hash), event_hash);
        let mut defeats_by_attribution = self.defeats_by_attribution;
        for key in ["carl", "donut", "unknown"] {
            defeats_by_attribution.entry(key.into()).or_insert(0);
        }
        EpisodeMetrics {
            schema: METRICS_SCHEMA,
            episode: ctx.episode,
            scenario: ctx.scenario,
            split: ctx.split,
            policy: ctx.policy,
            requested_seed: ctx.requested_seed,
            outcome: outcome.name(),
            simultaneous_clear: self.simultaneous_clear,
            ticks,
            return_total: env.episode_return(),
            reward: self.reward,
            required_enemies: ctx.required_enemies,
            enemies_defeated: self.defeated.len() as u32,
            defeats_by_attribution,
            defeat_causes: self.defeat_causes,
            heroes,
            hits_other_attribution: self.hits_other_attribution,
            knockbacks_derived: self.knockbacks,
            goblin_contacts: self.goblin_contacts,
            other_events: self.other_events,
            action_usage: self.usage,
            standing_still_ticks: self.standing_still,
            longest_no_progress_ticks: self.longest_gap,
            no_progress,
            terminal_position: TerminalPosition {
                x: diag.carl.x,
                y: diag.carl.y,
            },
            obs_clipped: diag.obs.clipped,
            strategy_fingerprint: fingerprint,
            final_state_hash: hex(state_hash),
            event_hash: hex(event_hash),
            trajectory_hash: hex(trajectory_hash),
            replay: ctx.replay,
        }
    }
}

impl Default for EpisodeCollector {
    fn default() -> Self {
        Self::new()
    }
}

fn cause_name(debug: &str) -> String {
    let end = debug.find([' ', '{', '(']).unwrap_or(debug.len());
    debug[..end].to_ascii_lowercase()
}

pub fn hex(value: u64) -> String {
    format!("{value:016x}")
}

/// A coarse, observable strategy descriptor: the share of ticks on each
/// action family in 10% steps, plus contact outcome counts (capped at 9).
fn fingerprint(
    usage: &ActionUsage,
    heroes: &BTreeMap<String, HeroMetrics>,
    defeated: usize,
) -> String {
    let ticks = usage.ticks.max(1) as f64;
    let bucket = |n: u64| ((n as f64 / ticks) * 10.0).round() as u32;
    let carl = heroes.get("carl");
    let hits: u32 = carl.map_or(0, |h| h.moves.values().map(|m| m.connected).sum());
    let hurt = carl.map_or(0, |h| h.hurt_events);
    format!(
        "L{}N{}R{}|J{}P{}K{}|h{}d{}u{}",
        bucket(usage.left_ticks),
        bucket(usage.neutral_ticks),
        bucket(usage.right_ticks),
        bucket(usage.jump_ticks),
        bucket(usage.primary_ticks),
        bucket(usage.physical_ticks),
        hits.min(9),
        defeated.min(9),
        hurt.min(9),
    )
}

// ---------------------------------------------------------------- aggregate --

/// Wilson 95% score interval for `successes` of `n` (z = 1.96).
pub fn wilson_95(successes: u64, n: u64) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let z = 1.96_f64;
    let n = n as f64;
    let p = successes as f64 / n;
    let z2 = z * z;
    let denom = 1.0 + z2 / n;
    let centre = (p + z2 / (2.0 * n)) / denom;
    let margin = z * ((p * (1.0 - p) / n) + z2 / (4.0 * n * n)).sqrt() / denom;
    ((centre - margin).max(0.0), (centre + margin).min(1.0))
}

/// Nearest-rank percentile of a sorted slice (`pct` in 0..=100).
pub fn percentile(sorted: &[f64], pct: u32) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = ((f64::from(pct) / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

/// Distribution summary of one numeric metric.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Distribution {
    pub min: f64,
    pub p25: f64,
    pub median: f64,
    pub p75: f64,
    pub p90: f64,
    pub max: f64,
    pub mean: f64,
}

impl Distribution {
    pub fn of(values: &[f64]) -> Self {
        let mut sorted = values.to_vec();
        sorted.sort_by(f64::total_cmp);
        let mean = if sorted.is_empty() {
            0.0
        } else {
            sorted.iter().sum::<f64>() / sorted.len() as f64
        };
        Self {
            min: sorted.first().copied().unwrap_or(0.0),
            p25: percentile(&sorted, 25),
            median: percentile(&sorted, 50),
            p75: percentile(&sorted, 75),
            p90: percentile(&sorted, 90),
            max: sorted.last().copied().unwrap_or(0.0),
            mean,
        }
    }
}

/// Aggregate per-move usage of one hero.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct MoveAggregate {
    pub attempts: u64,
    pub connected: u64,
    pub hits: u64,
    pub whiffs_derived: u64,
    /// `connected / attempts`, 0 when never attempted.
    pub connect_rate: f64,
}

/// Aggregate of one group of episodes (one scenario and policy, or a total).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AggregateMetrics {
    pub schema: &'static str,
    pub scenario: String,
    pub split: String,
    pub policy: String,
    pub episodes: u64,
    pub requested_seeds: Vec<u64>,
    pub successes: u64,
    pub success_rate: f64,
    pub success_ci95_low: f64,
    pub success_ci95_high: f64,
    pub ci_method: &'static str,
    /// Terminal reason counts: `success`, `defeat`, `abandoned`, `timeout`.
    pub outcomes: BTreeMap<String, u64>,
    pub simultaneous_clears: u64,
    pub ticks: Distribution,
    pub damage_received: Distribution,
    pub return_total: Distribution,
    pub enemies_defeated: Distribution,
    pub defeats_by_attribution: BTreeMap<String, u64>,
    pub defeat_causes: BTreeMap<String, u64>,
    pub hero_moves: BTreeMap<String, BTreeMap<String, MoveAggregate>>,
    pub knockbacks_derived: u64,
    pub goblin_contacts: u64,
    pub hero_deaths: BTreeMap<String, u64>,
    pub action_usage: ActionUsage,
    pub mean_modal_action_share: f64,
    pub standing_still_ticks_mean: f64,
    pub no_progress_episodes: u64,
    pub unique_fingerprints: u64,
    /// Shannon entropy of the fingerprint distribution, in bits. Neither
    /// this nor the count proves creativity or fun.
    pub fingerprint_entropy_bits: f64,
    /// Episodes with a distinct trajectory hash. For a deterministic policy
    /// on a fixed fixture this is 1: the episodes are copies, not
    /// independent evidence.
    pub distinct_trajectories: u64,
}

impl AggregateMetrics {
    pub fn of(scenario: &str, split: &str, policy: &str, eps: &[EpisodeMetrics]) -> Self {
        let n = eps.len() as u64;
        let successes = eps.iter().filter(|e| e.outcome == "success").count() as u64;
        let (low, high) = wilson_95(successes, n);
        let mut outcomes: BTreeMap<String, u64> = BTreeMap::new();
        for name in ["success", "defeat", "abandoned", "timeout"] {
            outcomes.insert(name.into(), 0);
        }
        let mut defeats_by_attribution: BTreeMap<String, u64> = BTreeMap::new();
        for key in ["carl", "donut", "unknown"] {
            defeats_by_attribution.insert(key.into(), 0);
        }
        let mut defeat_causes: BTreeMap<String, u64> = BTreeMap::new();
        let mut hero_moves: BTreeMap<String, BTreeMap<String, MoveAggregate>> = BTreeMap::new();
        let mut hero_deaths: BTreeMap<String, u64> = BTreeMap::new();
        let mut usage = ActionUsage::default();
        let mut fingerprints: BTreeMap<&str, u64> = BTreeMap::new();
        let mut trajectories: BTreeSet<&str> = BTreeSet::new();
        let mut seeds: BTreeSet<u64> = BTreeSet::new();
        let (mut knockbacks, mut contacts, mut clears, mut no_progress) = (0, 0, 0, 0);
        let mut still = 0.0;
        for e in eps {
            *outcomes.entry(e.outcome.into()).or_insert(0) += 1;
            for (k, v) in &e.defeats_by_attribution {
                *defeats_by_attribution.entry(k.clone()).or_insert(0) += u64::from(*v);
            }
            for (k, v) in &e.defeat_causes {
                *defeat_causes.entry(k.clone()).or_insert(0) += u64::from(*v);
            }
            for (hero, h) in &e.heroes {
                let deaths = hero_deaths.entry(hero.clone()).or_insert(0);
                *deaths += u64::from(h.defeated);
                let moves = hero_moves.entry(hero.clone()).or_default();
                for (name, m) in &h.moves {
                    let a = moves.entry(name.clone()).or_default();
                    a.attempts += u64::from(m.attempts);
                    a.connected += u64::from(m.connected);
                    a.hits += u64::from(m.hits);
                    a.whiffs_derived += u64::from(m.whiffs_derived);
                }
            }
            add_usage(&mut usage, &e.action_usage);
            *fingerprints.entry(&e.strategy_fingerprint).or_insert(0) += 1;
            trajectories.insert(&e.trajectory_hash);
            seeds.insert(e.requested_seed);
            knockbacks += u64::from(e.knockbacks_derived);
            contacts += u64::from(e.goblin_contacts);
            clears += u64::from(e.simultaneous_clear);
            no_progress += u64::from(e.no_progress);
            still += e.standing_still_ticks as f64;
        }
        for moves in hero_moves.values_mut() {
            for m in moves.values_mut() {
                m.connect_rate = ratio(m.connected, m.attempts);
            }
        }
        let modal_mean = mean(eps.iter().map(|e| e.action_usage.modal_action_share));
        usage.modal_action_share = modal_mean;
        let entropy: f64 = fingerprints
            .values()
            .map(|c| {
                let p = *c as f64 / n.max(1) as f64;
                -p * p.log2()
            })
            .sum();
        let carl_damage = |e: &EpisodeMetrics| {
            e.heroes
                .get("carl")
                .map_or(0.0, |h| f64::from(h.damage_received))
        };
        Self {
            schema: METRICS_SCHEMA,
            scenario: scenario.into(),
            split: split.into(),
            policy: policy.into(),
            episodes: n,
            requested_seeds: seeds.into_iter().collect(),
            successes,
            success_rate: ratio(successes, n),
            success_ci95_low: low,
            success_ci95_high: high,
            ci_method: "wilson-95",
            outcomes,
            simultaneous_clears: clears,
            ticks: Distribution::of(&eps.iter().map(|e| e.ticks as f64).collect::<Vec<_>>()),
            damage_received: Distribution::of(&eps.iter().map(carl_damage).collect::<Vec<_>>()),
            return_total: Distribution::of(&eps.iter().map(|e| e.return_total).collect::<Vec<_>>()),
            enemies_defeated: Distribution::of(
                &eps.iter()
                    .map(|e| f64::from(e.enemies_defeated))
                    .collect::<Vec<_>>(),
            ),
            defeats_by_attribution,
            defeat_causes,
            hero_moves,
            knockbacks_derived: knockbacks,
            goblin_contacts: contacts,
            hero_deaths,
            action_usage: usage,
            mean_modal_action_share: modal_mean,
            standing_still_ticks_mean: if n == 0 { 0.0 } else { still / n as f64 },
            no_progress_episodes: no_progress,
            unique_fingerprints: fingerprints.len() as u64,
            fingerprint_entropy_bits: entropy,
            distinct_trajectories: trajectories.len() as u64,
        }
    }
}

fn ratio(a: u64, b: u64) -> f64 {
    if b == 0 { 0.0 } else { a as f64 / b as f64 }
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, n) = values.fold((0.0, 0u64), |(s, n), v| (s + v, n + 1));
    if n == 0 { 0.0 } else { sum / n as f64 }
}

fn add_usage(total: &mut ActionUsage, one: &ActionUsage) {
    total.ticks += one.ticks;
    total.left_ticks += one.left_ticks;
    total.neutral_ticks += one.neutral_ticks;
    total.right_ticks += one.right_ticks;
    total.jump_ticks += one.jump_ticks;
    total.primary_ticks += one.primary_ticks;
    total.physical_ticks += one.physical_ticks;
    total.primary_presses += one.primary_presses;
    total.physical_presses += one.physical_presses;
    total.jump_presses += one.jump_presses;
    total.special_unavailable_ticks += one.special_unavailable_ticks;
    total.special_unavailable_presses += one.special_unavailable_presses;
    total.swap_unavailable_ticks += one.swap_unavailable_ticks;
    total.swap_unavailable_presses += one.swap_unavailable_presses;
    total.idle_ticks += one.idle_ticks;
}

// ---------------------------------------------------------------------- csv --

/// Column names of the per-episode CSV.
pub const EPISODE_CSV_HEADER: &str = "episode,scenario,split,policy,requested_seed,outcome,\
simultaneous_clear,ticks,return_total,required_enemies,enemies_defeated,defeats_carl,\
defeats_donut,defeats_unknown,carl_hurt_events,carl_damage_received,carl_defeated,\
carl_jab_attempts,carl_jab_connected,carl_haymaker_attempts,carl_haymaker_connected,\
carl_push_kick_attempts,carl_push_kick_connected,knockbacks_derived,goblin_contacts,\
primary_presses,physical_presses,jump_presses,special_unavailable_ticks,\
swap_unavailable_ticks,standing_still_ticks,longest_no_progress_ticks,no_progress,\
modal_action_share,terminal_x,terminal_y,strategy_fingerprint,final_state_hash,\
event_hash,trajectory_hash,replay";

fn move_pair(e: &EpisodeMetrics, name: &str) -> (u32, u32) {
    e.heroes
        .get("carl")
        .and_then(|h| h.moves.get(name))
        .map_or((0, 0), |m| (m.attempts, m.connected))
}

/// One CSV row of an episode (no trailing newline).
pub fn episode_csv_row(e: &EpisodeMetrics) -> String {
    let carl = e.heroes.get("carl");
    let (jab, hay, kick) = (
        move_pair(e, "jab"),
        move_pair(e, "haymaker"),
        move_pair(e, "push_kick"),
    );
    let d = |k: &str| e.defeats_by_attribution.get(k).copied().unwrap_or(0);
    let fields: Vec<String> = vec![
        e.episode.to_string(),
        e.scenario.clone(),
        e.split.clone(),
        e.policy.clone(),
        e.requested_seed.to_string(),
        e.outcome.into(),
        e.simultaneous_clear.to_string(),
        e.ticks.to_string(),
        format!("{:.6}", e.return_total),
        e.required_enemies.to_string(),
        e.enemies_defeated.to_string(),
        d("carl").to_string(),
        d("donut").to_string(),
        d("unknown").to_string(),
        carl.map_or(0, |h| h.hurt_events).to_string(),
        carl.map_or(0, |h| h.damage_received).to_string(),
        carl.is_some_and(|h| h.defeated).to_string(),
        jab.0.to_string(),
        jab.1.to_string(),
        hay.0.to_string(),
        hay.1.to_string(),
        kick.0.to_string(),
        kick.1.to_string(),
        e.knockbacks_derived.to_string(),
        e.goblin_contacts.to_string(),
        e.action_usage.primary_presses.to_string(),
        e.action_usage.physical_presses.to_string(),
        e.action_usage.jump_presses.to_string(),
        e.action_usage.special_unavailable_ticks.to_string(),
        e.action_usage.swap_unavailable_ticks.to_string(),
        e.standing_still_ticks.to_string(),
        e.longest_no_progress_ticks.to_string(),
        e.no_progress.to_string(),
        format!("{:.6}", e.action_usage.modal_action_share),
        format!("{:.2}", e.terminal_position.x),
        format!("{:.2}", e.terminal_position.y),
        e.strategy_fingerprint.clone(),
        e.final_state_hash.clone(),
        e.event_hash.clone(),
        e.trajectory_hash.clone(),
        e.replay.clone().unwrap_or_default(),
    ];
    fields.join(",")
}

/// Column names of the aggregate CSV.
pub const AGGREGATE_CSV_HEADER: &str = "scenario,split,policy,episodes,successes,success_rate,\
success_ci95_low,success_ci95_high,outcome_success,outcome_defeat,outcome_abandoned,\
outcome_timeout,simultaneous_clears,ticks_median,ticks_p90,damage_median,damage_p90,\
return_mean,enemies_defeated_mean,defeats_carl,defeats_donut,defeats_unknown,\
carl_jab_connect_rate,carl_haymaker_connect_rate,carl_push_kick_connect_rate,\
knockbacks_derived,goblin_contacts,carl_deaths,primary_presses,physical_presses,\
jump_presses,special_unavailable_ticks,swap_unavailable_ticks,mean_modal_action_share,\
standing_still_ticks_mean,no_progress_episodes,unique_fingerprints,\
fingerprint_entropy_bits,distinct_trajectories";

/// One CSV row of an aggregate (no trailing newline).
pub fn aggregate_csv_row(a: &AggregateMetrics) -> String {
    let rate = |name: &str| {
        a.hero_moves
            .get("carl")
            .and_then(|m| m.get(name))
            .map_or(0.0, |m| m.connect_rate)
    };
    let o = |k: &str| a.outcomes.get(k).copied().unwrap_or(0);
    let d = |k: &str| a.defeats_by_attribution.get(k).copied().unwrap_or(0);
    let fields: Vec<String> = vec![
        a.scenario.clone(),
        a.split.clone(),
        a.policy.clone(),
        a.episodes.to_string(),
        a.successes.to_string(),
        format!("{:.6}", a.success_rate),
        format!("{:.6}", a.success_ci95_low),
        format!("{:.6}", a.success_ci95_high),
        o("success").to_string(),
        o("defeat").to_string(),
        o("abandoned").to_string(),
        o("timeout").to_string(),
        a.simultaneous_clears.to_string(),
        format!("{:.1}", a.ticks.median),
        format!("{:.1}", a.ticks.p90),
        format!("{:.1}", a.damage_received.median),
        format!("{:.1}", a.damage_received.p90),
        format!("{:.6}", a.return_total.mean),
        format!("{:.6}", a.enemies_defeated.mean),
        d("carl").to_string(),
        d("donut").to_string(),
        d("unknown").to_string(),
        format!("{:.6}", rate("jab")),
        format!("{:.6}", rate("haymaker")),
        format!("{:.6}", rate("push_kick")),
        a.knockbacks_derived.to_string(),
        a.goblin_contacts.to_string(),
        a.hero_deaths.get("carl").copied().unwrap_or(0).to_string(),
        a.action_usage.primary_presses.to_string(),
        a.action_usage.physical_presses.to_string(),
        a.action_usage.jump_presses.to_string(),
        a.action_usage.special_unavailable_ticks.to_string(),
        a.action_usage.swap_unavailable_ticks.to_string(),
        format!("{:.6}", a.mean_modal_action_share),
        format!("{:.3}", a.standing_still_ticks_mean),
        a.no_progress_episodes.to_string(),
        a.unique_fingerprints.to_string(),
        format!("{:.6}", a.fingerprint_entropy_bits),
        a.distinct_trajectories.to_string(),
    ];
    fields.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wilson_interval_matches_known_values() {
        let (low, high) = wilson_95(0, 0);
        assert_eq!((low, high), (0.0, 1.0));
        let (low, high) = wilson_95(5, 10);
        assert!((low - 0.2366).abs() < 1e-3, "{low}");
        assert!((high - 0.7634).abs() < 1e-3, "{high}");
        let (low, high) = wilson_95(10, 10);
        assert!((low - 0.7225).abs() < 1e-3, "{low}");
        assert!((high - 1.0).abs() < 1e-9, "{high}");
        let (low, _) = wilson_95(0, 10);
        assert!(low.abs() < 1e-9);
    }

    #[test]
    fn percentile_is_nearest_rank() {
        let v = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        assert_eq!(percentile(&v, 50), 5.0);
        assert_eq!(percentile(&v, 90), 9.0);
        assert_eq!(percentile(&v, 100), 10.0);
        assert_eq!(percentile(&[], 50), 0.0);
    }

    #[test]
    fn cause_names_drop_payloads() {
        assert_eq!(cause_name("Melee"), "melee");
        assert_eq!(cause_name("ShoreBlock { player: 0 }"), "shoreblock");
    }

    #[test]
    fn defeats_are_attributed_by_instigator_once_per_id() {
        use crate::combat::{DefeatCause, ForceTier};
        let mut collector = EpisodeCollector::new();
        let defeat = |target, instigator| CombatEvent::EnemyDefeated {
            tick: 1,
            target,
            cause: DefeatCause::Melee,
            force: ForceTier::Routine,
            instigator,
        };
        for event in [
            defeat(0, Some(0)),
            defeat(0, Some(0)),
            defeat(1, None),
            defeat(2, Some(1)),
            defeat(3, Some(7)),
        ] {
            collector.on_event(&event);
        }
        let by = &collector.defeats_by_attribution;
        assert_eq!(by["carl"], 1);
        assert_eq!(by["unknown"], 2);
        assert_eq!(by["donut"], 1);
        assert_eq!(collector.defeated.len(), 4);
        assert_eq!(collector.defeat_causes["melee"], 4);
    }

    #[test]
    fn unknown_events_go_to_other_events_and_hurt_tracks_damage() {
        let mut collector = EpisodeCollector::new();
        assert!(!collector.on_event(&CombatEvent::GoblinTell { tick: 1, goblin: 0 }));
        assert_eq!(collector.other_events, 1);
        for left in [3, 1] {
            assert!(collector.on_event(&CombatEvent::HeroHurt {
                tick: 2,
                source: 0,
                health_left: left,
                hero: HeroId::Carl,
            }));
        }
        assert_eq!(collector.hero_hurt["carl"], (2, 3));
    }

    #[test]
    fn csv_headers_match_row_widths() {
        let cols = EPISODE_CSV_HEADER.split(',').count();
        assert_eq!(cols, 42);
        let cols = AGGREGATE_CSV_HEADER.split(',').count();
        assert_eq!(cols, 40);
    }
}
