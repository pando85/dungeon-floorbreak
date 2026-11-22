//! PT-D: read-only aggregate counts of one loaded session (`playtrace stats`).
//!
//! Counts only: ticks, episodes and, per hero, how many ticks each control
//! source drove it and which actions that hero held. For a human-driven hero
//! the action is the routed input (`hero`); for an AI-driven hero it is the
//! effective input the sim applied (`eff`, the AI's own decision). Human and AI
//! counts are kept apart and never merged.

use super::codec::{ActionBits, ControlSource};
use super::reader::LoadedSession;

/// Stick magnitude at or above which a tick counts as `left` or `right`
/// (the same threshold the Python exporter uses to discretize `move_x`).
pub const STICK_THRESHOLD: f32 = 0.5;

/// Action counts of one hero for one class of controller.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionHistogram {
    pub ticks: u64,
    pub left: u64,
    pub neutral: u64,
    pub right: u64,
    pub jump: u64,
    pub attack: u64,
    pub physical: u64,
    pub ability: u64,
    pub down: u64,
    pub swap: u64,
}

impl ActionHistogram {
    fn add(&mut self, action: ActionBits) {
        let raw = action.to_raw();
        self.ticks += 1;
        if raw.move_x <= -STICK_THRESHOLD {
            self.left += 1;
        } else if raw.move_x >= STICK_THRESHOLD {
            self.right += 1;
        } else {
            self.neutral += 1;
        }
        self.jump += u64::from(raw.jump);
        self.attack += u64::from(raw.attack);
        self.physical += u64::from(raw.physical);
        self.ability += u64::from(raw.ability);
        self.down += u64::from(raw.down);
        self.swap += u64::from(raw.swap);
    }
}

/// Per-hero counts: `human` and `ai` histograms plus the ticks nobody drove.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeroStats {
    pub human: ActionHistogram,
    pub ai: ActionHistogram,
    pub undriven_ticks: u64,
    /// Human ticks whose routed input differs from the effective input.
    pub human_intent_differs_effective: u64,
}

/// Aggregate counts of one session.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionStats {
    /// The manifest marks the session synthetic (a fixture, never a human).
    pub synthetic: bool,
    pub ticks: u64,
    pub episodes: u32,
    pub combat_events: u64,
    /// `[carl, donut]`.
    pub heroes: [HeroStats; 2],
}

pub fn session_stats(session: &LoadedSession) -> SessionStats {
    let mut out = SessionStats {
        synthetic: super::synthetic::is_synthetic(&session.manifest),
        ticks: session.ticks.len() as u64,
        episodes: session.ticks.last().map_or(0, |t| t.ep + 1),
        ..SessionStats::default()
    };
    for tick in &session.ticks {
        out.combat_events += tick.ev.len() as u64;
        for hero in 0..2 {
            let stats = &mut out.heroes[hero];
            match tick.src[hero] {
                ControlSource::HumanP1 | ControlSource::HumanP2 => {
                    stats.human.add(tick.hero[hero]);
                    if tick.eff.is_some_and(|eff| eff[hero] != tick.hero[hero]) {
                        stats.human_intent_differs_effective += 1;
                    }
                }
                ControlSource::CompanionAi | ControlSource::ScriptedBot => {
                    if let Some(eff) = tick.eff {
                        stats.ai.add(eff[hero]);
                    }
                }
                ControlSource::None => stats.undriven_ticks += 1,
            }
        }
    }
    out
}

/// FIX-TOOL: no tick of the session carries any raw input in either slot (P1 and P2): the
/// window was never touched (a scripted exit on a live room, a harness run), so the session
/// shows the partner AI and the room, not a person playing. An empty session is not
/// "input-free" (there is nothing to say). `inspect` labels such a `Human` session honestly.
pub fn input_free(session: &LoadedSession) -> bool {
    !session.ticks.is_empty()
        && session
            .ticks
            .iter()
            .all(|tick| tick.slots.iter().all(|slot| *slot == ActionBits::NONE))
}
