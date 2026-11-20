//! PT-B: the live recording adapter core. Bevy-free and observer-only.
//!
//! The `floor01_scene` fixed step calls a handful of hooks on a
//! [`LiveRecorder`]; every hook takes shared references to the simulation and
//! only reads. Nothing here calls `take_new_events()`, mutates the party or the
//! combat state, draws a random number or reads the wall clock into a tick:
//! recording on and off give identical simulation hashes.
//!
//! Call order per fixed step (see `fixed_step` in `floor01_scene`):
//!
//! 1. [`LiveRecorder::observe_party`] at the top (pad lifecycle and the D6
//!    pause happen between steps, even when the step then returns early).
//! 2. [`LiveRecorder::note_restart`] / [`LiveRecorder::note_demo_respawn`]
//!    right after the matching sim call.
//! 3. [`LiveRecorder::begin_tick`] just before the step (pre-state hash, the
//!    per-slot provenance inputs and the routed per-hero replay driver).
//! 4. [`LiveRecorder::end_tick`] just after it (events through the recorder's
//!    own absolute cursor, post-state hashes, episode boundaries).
//!
//! The recorder never records the presim, captures, perf probes or headless
//! runs: the binary only builds one for a live-input run.

use std::path::Path;
use std::process::Command;

use crate::autoplay::metrics::EventHasher;
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::input::{RawInput, SlotInputs};
use crate::party::{HeroControl, HeroId, PartyDrive, PartyMode, PartyState, PlayerSlot};

use super::codec::{
    ActionBits, ControlEvent, ControlSource, EpisodeBoundary, EpisodeReason, EventRecord,
    TickRecord,
};
use super::error::PlaytraceError;
use super::manifest::{SessionManifest, SessionStatus};
use super::writer::{SessionWriter, WriterHealth};

/// What the recorder remembers of the party between hooks, to turn state
/// changes into ordered control events without touching the party.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PartySnap {
    mode: PartyMode,
    controlled: HeroId,
    paused: bool,
    vacated: [bool; 2],
}

impl PartySnap {
    fn of(party: &PartyState) -> Self {
        let vacated = party.vacated_slots();
        Self {
            mode: party.mode(),
            controlled: party.controlled_hero(),
            paused: party.is_paused(),
            vacated: [
                vacated.contains(&PlayerSlot::P1),
                vacated.contains(&PlayerSlot::P2),
            ],
        }
    }
}

/// The inputs and pre-state captured by `begin_tick`, completed by `end_tick`.
struct Pending {
    pre: u64,
    slots: SlotInputs,
    hero: [RawInput; 2],
    src: [ControlSource; 2],
    stage: u32,
}

pub struct LiveRecorder {
    writer: SessionWriter,
    session_id: String,
    episode: u32,
    /// Absolute event cursor: `dropped_events + index` of the capped log.
    cursor: u64,
    events_lost: u64,
    ehash: EventHasher,
    snap: PartySnap,
    drive: Option<PartyDrive>,
    restarts: u32,
    stage: u32,
    pending: Option<Pending>,
    messages: Vec<String>,
    reported_incomplete: bool,
    ticks: u64,
}

fn state_hash(sim: &SceneSim, combat: Option<&CombatState>) -> u64 {
    combat.map_or_else(|| sim.state_hash(), |c| c.state_hash(sim))
}

/// A small hash of the swap timers and the routing (the `party_hash` field).
fn party_hash(party: &PartyState) -> u64 {
    let (cooldown, buffer) = party.swap_timers();
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for value in [
        0x5041_5254_5953_5441_u64,
        u64::from(cooldown),
        u64::from(buffer),
        party.controlled_hero().index() as u64,
        u64::from(party.mode() == PartyMode::TwoPlayer),
    ] {
        hash = crate::autoplay::metrics::fnv_mix(hash, value);
    }
    hash
}

fn source_of(party: &PartyState, hero: HeroId, drive: &PartyDrive) -> ControlSource {
    match party.hero_control(hero) {
        HeroControl::Player(PlayerSlot::P1) => ControlSource::HumanP1,
        HeroControl::Player(PlayerSlot::P2) => ControlSource::HumanP2,
        HeroControl::Uncontrolled if drive.is_bot(hero) => ControlSource::ScriptedBot,
        HeroControl::Uncontrolled => ControlSource::None,
    }
}

/// The commit of the source checkout this binary was built from, `-dirty`
/// when tracked files changed, `unknown` when git cannot tell. The checkout
/// path is used only to run git and is never stored.
pub fn build_identity() -> (String, bool) {
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(env!("CARGO_MANIFEST_DIR"))
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
    };
    let Some(output) = git(&["rev-parse", "HEAD"]) else {
        return ("unknown".to_owned(), false);
    };
    let commit = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if commit.len() < 7 {
        return ("unknown".to_owned(), false);
    }
    let dirty = git(&["status", "--porcelain", "--untracked-files=no"])
        .is_none_or(|o| !o.stdout.is_empty());
    (commit, dirty)
}

impl LiveRecorder {
    /// Start a session before the first simulated tick: create the directory,
    /// write the manifest and open episode 0. `Err` means nothing is being
    /// recorded; the caller says so out loud and the game runs on.
    pub fn start(
        base: &Path,
        manifest: SessionManifest,
        combat: Option<&CombatState>,
        party: &PartyState,
    ) -> Result<Self, PlaytraceError> {
        let session_id = manifest.session_id.clone();
        let mut writer = SessionWriter::create(base, manifest)?;
        let stage = combat.map_or(0, |c| c.checkpoint as u32);
        writer.begin_episode(EpisodeBoundary {
            episode: 0,
            reason: EpisodeReason::Start,
            stage,
        })?;
        let messages = vec![format!(
            "PLAYTRACE_REC session={session_id} dir={}",
            writer.dir().display()
        )];
        Ok(Self {
            writer,
            session_id,
            episode: 0,
            cursor: 0,
            events_lost: 0,
            ehash: EventHasher::new(),
            snap: PartySnap::of(party),
            drive: None,
            restarts: combat.map_or(0, |c| c.restarts),
            stage,
            pending: None,
            messages,
            reported_incomplete: false,
            ticks: 0,
        })
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Ticks recorded so far.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    pub fn health(&self) -> WriterHealth {
        self.writer.health()
    }

    pub fn manifest(&self) -> &SessionManifest {
        self.writer.manifest()
    }

    /// Lines for stdout (`PLAYTRACE_` prefix), drained by the caller.
    pub fn drain_messages(&mut self) -> Vec<String> {
        std::mem::take(&mut self.messages)
    }

    fn control(&mut self, ev: ControlEvent) {
        if let Err(error) = self.writer.record_control(ev) {
            self.messages.push(format!(
                "PLAYTRACE_ERROR session={} {error}",
                self.session_id
            ));
        }
    }

    fn new_episode(&mut self, reason: EpisodeReason, outcome: &str, stage: u32) {
        let ended = self.episode;
        self.control(ControlEvent::EpisodeEnd {
            episode: ended,
            outcome: outcome.to_owned(),
        });
        self.episode += 1;
        let boundary = EpisodeBoundary {
            episode: self.episode,
            reason,
            stage,
        };
        if let Err(error) = self.writer.begin_episode(boundary) {
            self.messages.push(format!(
                "PLAYTRACE_ERROR session={} {error}",
                self.session_id
            ));
        }
        self.stage = stage;
    }

    /// Turn party state changes since the last hook into ordered control
    /// events (pad vacated or reclaimed, D6 pause begin and end, P2 join or
    /// quit, solo swap). Reads the party only.
    pub fn observe_party(&mut self, party: &PartyState) {
        let now = PartySnap::of(party);
        let before = self.snap;
        if now == before {
            return;
        }
        for slot in 0..2 {
            if now.vacated[slot] != before.vacated[slot] {
                let ev = if now.vacated[slot] {
                    ControlEvent::PadVacated { slot: slot as u8 }
                } else {
                    ControlEvent::PadReclaimed { slot: slot as u8 }
                };
                self.control(ev);
            }
        }
        if now.paused != before.paused {
            self.control(if now.paused {
                ControlEvent::D6PauseBegin
            } else {
                ControlEvent::D6PauseEnd
            });
        }
        if now.mode != before.mode {
            if now.mode == PartyMode::TwoPlayer {
                self.control(ControlEvent::PartyJoin);
                self.messages.push(format!(
                    "PLAYTRACE_REC session={} participants=P1+P2 (P2 joined)",
                    self.session_id
                ));
            } else {
                self.control(ControlEvent::PartyQuit);
            }
        } else if now.mode == PartyMode::Solo && now.controlled != before.controlled {
            self.control(ControlEvent::SoloSwap {
                controlled: now.controlled.index() as u8,
            });
        }
        self.snap = now;
    }

    /// A manual restart (R or Select held) just ran: a new episode begins.
    /// Call after `combat.restart` / `sim.reset` and `party.restart()`.
    pub fn note_restart(&mut self, combat: Option<&CombatState>, party: &PartyState) {
        self.control(ControlEvent::Restart);
        let stage = combat.map_or(0, |c| c.checkpoint as u32);
        self.new_episode(EpisodeReason::Restart, "restart", stage);
        self.restarts = combat.map_or(0, |c| c.restarts);
        self.snap = PartySnap::of(party);
    }

    /// A demo respawn (Select tap) just ran, before the step: the tick counter
    /// keeps running, a new episode begins. Its events are picked up by the
    /// next `end_tick` through the absolute cursor.
    pub fn note_demo_respawn(&mut self, combat: Option<&CombatState>, party: &PartyState) {
        self.control(ControlEvent::DemoRespawn);
        let stage = combat.map_or(0, |c| c.checkpoint as u32);
        self.new_episode(EpisodeReason::DemoRespawn, "demo_respawn", stage);
        self.snap = PartySnap::of(party);
    }

    /// Before the step: capture the pre-state hash, the per-slot inputs
    /// (provenance), the routed per-hero inputs (the replay driver) and the
    /// drive. `hero` is `party.hero_inputs(slots)` as fed to the step.
    pub fn begin_tick(
        &mut self,
        sim: &SceneSim,
        combat: Option<&CombatState>,
        party: &PartyState,
        slots: SlotInputs,
        hero: [RawInput; 2],
        drive: PartyDrive,
    ) {
        self.observe_party(party);
        if self.drive != Some(drive) {
            self.control(ControlEvent::DriveChange {
                live: drive.live,
                two_player: drive.two_player,
                controlled: drive.controlled.index() as u8,
                bots: drive.bots,
            });
            self.drive = Some(drive);
        }
        self.pending = Some(Pending {
            pre: state_hash(sim, combat),
            slots,
            hero,
            src: [
                source_of(party, HeroId::Carl, &drive),
                source_of(party, HeroId::Donut, &drive),
            ],
            stage: self.stage,
        });
    }

    /// After the step: write the tick. `respawned` is the demo respawn that
    /// happened inside the step (`step_combat`'s return value).
    pub fn end_tick(
        &mut self,
        sim: &SceneSim,
        combat: Option<&CombatState>,
        party: &PartyState,
        respawned: bool,
    ) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        let mut events = Vec::new();
        if let Some(combat) = combat {
            let dropped = combat.dropped_events as u64;
            let end = dropped + combat.events.len() as u64;
            if self.cursor < dropped {
                // The cap drained events the recorder had not read yet: loud,
                // never silent (cannot happen at one read per tick).
                self.events_lost += dropped - self.cursor;
                self.cursor = dropped;
            }
            let from = (self.cursor - dropped) as usize;
            let fresh = &combat.events[from.min(combat.events.len())..];
            self.ehash.fold(fresh);
            events = fresh
                .iter()
                .enumerate()
                .map(|(index, event)| EventRecord::from_combat(self.cursor + index as u64, event))
                .collect();
            self.cursor = end;
        }
        let record = TickRecord {
            t: self.writer.next_t(),
            ep: self.episode,
            et: sim.world.tick,
            stage: pending.stage,
            pre: pending.pre,
            slots: [
                ActionBits::from_raw(pending.slots[0]),
                ActionBits::from_raw(pending.slots[1]),
            ],
            hero: [
                ActionBits::from_raw(pending.hero[0]),
                ActionBits::from_raw(pending.hero[1]),
            ],
            src: pending.src,
            ev: events,
            post: state_hash(sim, combat),
            eh: self.ehash.value(),
            ph: party_hash(party),
        };
        if let Err(error) = self.writer.record_tick(&record) {
            self.messages.push(format!(
                "PLAYTRACE_ERROR session={} {error}",
                self.session_id
            ));
        }
        self.ticks += 1;
        if let Some(combat) = combat {
            if combat.restarts != self.restarts {
                // A party defeat restarted the attempt inside the step.
                self.restarts = combat.restarts;
                let stage = combat.checkpoint as u32;
                self.new_episode(EpisodeReason::PartyDefeatRestart, "party_defeat", stage);
                self.snap = PartySnap::of(party);
            } else if respawned {
                let stage = combat.checkpoint as u32;
                self.control(ControlEvent::DemoRespawn);
                self.new_episode(EpisodeReason::DemoRespawn, "demo_respawn", stage);
                self.snap = PartySnap::of(party);
            } else if combat.checkpoint as u32 > self.stage {
                self.stage = combat.checkpoint as u32;
                self.control(ControlEvent::DemoCheckpointAdvance {
                    checkpoint: self.stage,
                });
            }
        }
        self.report_health();
    }

    fn report_health(&mut self) {
        let health = self.writer.health();
        if health.incomplete() && !self.reported_incomplete {
            self.reported_incomplete = true;
            self.messages.push(format!(
                "PLAYTRACE_ERROR session={} recording is incomplete (write_errors={} dropped_ticks={} last_error={}); the game keeps running",
                self.session_id,
                health.write_errors,
                health.dropped_ticks,
                health.last_error.as_deref().unwrap_or("none"),
            ));
        }
        if self.events_lost > 0 && !self.reported_incomplete {
            self.reported_incomplete = true;
            self.messages.push(format!(
                "PLAYTRACE_ERROR session={} {} combat events were lost to the log cap",
                self.session_id, self.events_lost
            ));
        }
    }

    /// Close the session (app quit). Records the exit, ends the open episode
    /// and writes the final manifest. Idempotent errors are reported, not
    /// raised: a failing recorder never blocks the quit.
    pub fn finish(&mut self, reason: &str) -> Option<SessionManifest> {
        self.control(ControlEvent::Exit {
            reason: reason.to_owned(),
        });
        self.control(ControlEvent::EpisodeEnd {
            episode: self.episode,
            outcome: "exit".to_owned(),
        });
        self.control(ControlEvent::AppQuit);
        if self.events_lost > 0 {
            let flags = &mut self.writer.manifest_mut().quality_flags;
            flags.push("events_lost_to_cap".to_owned());
        }
        match self.writer.finish(SessionStatus::Completed) {
            Ok(manifest) => {
                self.messages.push(format!(
                    "PLAYTRACE_SAVED session={} ticks={} status={:?} complete={}",
                    self.session_id,
                    manifest.tick_count,
                    manifest.status,
                    manifest.capture_complete
                ));
                Some(manifest)
            }
            Err(error) => {
                self.messages.push(format!(
                    "PLAYTRACE_ERROR session={} could not finish: {error}",
                    self.session_id
                ));
                None
            }
        }
    }
}
