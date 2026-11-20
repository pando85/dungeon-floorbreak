//! Privileged diagnostics (AP-Q08, AP-Q14).
//!
//! Everything a policy must never see: the full event log, true enemy health,
//! cooldowns, hashes and observation-side counters. It is reached only through
//! [`super::AutoplayEnv::diagnostics`] (and later the episode artifact), never
//! through per-step `info`, and is never concatenated to an observation.

use crate::combat::{CombatEvent, MoveId};
use crate::combat_scene::FixtureSim;

use super::observation::ObsStats;
use super::reward::Attribution;

/// Which side owns a move, from its `MoveId` family (AP-Q14).
pub fn move_attribution(move_id: MoveId) -> Attribution {
    match move_id {
        MoveId::Jab | MoveId::Haymaker | MoveId::PushKick => Attribution::Carl,
        MoveId::Swat | MoveId::Pounce | MoveId::MagicMissile => Attribution::Donut,
        _ => Attribution::Unknown,
    }
}

/// Counts of the episode's events by kind and attribution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EventCounts {
    pub moves_started: u32,
    pub hits_carl: u32,
    pub hits_donut: u32,
    pub hits_unknown: u32,
    pub enemies_defeated: u32,
    pub hero_hurt: u32,
    /// Every other event, including variants a later ruleset appends.
    pub other_events: u32,
    /// Carl moves started whose instance never produced a `Hit`. Derived,
    /// not an event of the game (AP-Q14); a move still running counts.
    pub derived_whiffs: u32,
}

impl EventCounts {
    pub fn from_events(events: &[CombatEvent]) -> Self {
        let mut counts = Self::default();
        let mut started = Vec::new();
        let mut hit_instances = Vec::new();
        for event in events {
            match event {
                CombatEvent::MoveStarted {
                    move_id, instance, ..
                } => {
                    counts.moves_started += 1;
                    if move_attribution(*move_id) == Attribution::Carl {
                        started.push(*instance);
                    }
                }
                CombatEvent::Hit {
                    move_id, instance, ..
                } => {
                    match move_attribution(*move_id) {
                        Attribution::Carl => counts.hits_carl += 1,
                        Attribution::Donut => counts.hits_donut += 1,
                        Attribution::Unknown => counts.hits_unknown += 1,
                    }
                    // Donut keeps its own instance counter: only Carl's hits
                    // can cancel a Carl whiff.
                    if move_attribution(*move_id) == Attribution::Carl {
                        hit_instances.push(*instance);
                    }
                }
                CombatEvent::EnemyDefeated { .. } => counts.enemies_defeated += 1,
                CombatEvent::HeroHurt { .. } => counts.hero_hurt += 1,
                _ => counts.other_events += 1,
            }
        }
        counts.derived_whiffs = started
            .iter()
            .filter(|instance| !hit_instances.contains(instance))
            .count() as u32;
        counts
    }
}

/// Carl's true internal state.
#[derive(Debug, Clone, PartialEq)]
pub struct CarlDiagnostics {
    pub health: u8,
    pub hurt_ticks: u8,
    pub invulnerable_ticks: u8,
    pub hero_hit_stop: u8,
    pub x: f32,
    pub y: f32,
}

/// One enemy's true internal state, including what the policy never sees.
#[derive(Debug, Clone, PartialEq)]
pub struct EnemyDiagnostics {
    /// The game's target id (goblins first, then llamas).
    pub id: u32,
    pub kind: &'static str,
    pub health: u8,
    pub mode: String,
    pub attack_tick: u16,
    pub attack_cooldown: u16,
    pub hurt_cooldown: u16,
    pub x: f32,
    pub y: f32,
}

/// The privileged record of an episode so far.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostics {
    pub tick: u64,
    pub state_hash: u64,
    pub initial_hash: u64,
    pub events: Vec<CombatEvent>,
    pub event_counts: EventCounts,
    pub carl: CarlDiagnostics,
    pub enemies: Vec<EnemyDiagnostics>,
    /// Enemy ids defeated so far.
    pub defeated: Vec<u32>,
    /// Enemy ids already credited by the reward.
    pub credited: Vec<u32>,
    pub reward_total: f64,
    /// Cumulative observation-side counters (`obs_clipped`, unknown modes,
    /// overflow), accumulated once per executed `step` and `reset`.
    pub obs: ObsStats,
}

pub(crate) fn carl_diagnostics(sim: &FixtureSim) -> CarlDiagnostics {
    let hero = &sim.scene.world.hero;
    CarlDiagnostics {
        health: hero.health,
        hurt_ticks: sim.combat.hurt_ticks,
        invulnerable_ticks: hero.invulnerable_ticks,
        hero_hit_stop: sim.combat.hero_hit_stop,
        x: hero.x,
        y: hero.y,
    }
}

pub(crate) fn enemy_diagnostics(sim: &FixtureSim) -> Vec<EnemyDiagnostics> {
    let goblins = sim.combat.enemies.len() as u32;
    let mut out: Vec<EnemyDiagnostics> = sim
        .combat
        .enemies
        .iter()
        .enumerate()
        .map(|(i, g)| EnemyDiagnostics {
            id: i as u32,
            kind: "goblin",
            health: g.health,
            mode: format!("{:?}", g.mode),
            attack_tick: g.attack_tick,
            attack_cooldown: u16::from(g.attack_cooldown),
            hurt_cooldown: u16::from(g.hurt_cooldown),
            x: g.x,
            y: g.y,
        })
        .collect();
    out.extend(
        sim.combat
            .llamas
            .iter()
            .enumerate()
            .map(|(i, l)| EnemyDiagnostics {
                id: goblins + i as u32,
                kind: "llama",
                health: l.health,
                mode: format!("{:?}", l.mode),
                attack_tick: l.attack_tick,
                attack_cooldown: l.cooldown,
                hurt_cooldown: 0,
                x: l.x,
                y: l.y,
            }),
    );
    out
}
