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
//! 3. [`LiveRecorder::set_rig_view`] (optional diagnostic), then
//!    [`LiveRecorder::begin_tick`] just before the step (pre-state hash, the
//!    per-slot provenance inputs, the routed per-hero replay driver and, with a
//!    combat state, the ObservationV2 `o_t` of both heroes from the sim camera,
//!    **before** the live decision).
//! 4. [`LiveRecorder::end_tick`] just after it (events through the recorder's
//!    own absolute cursor, post-state hashes, the effective inputs the sim
//!    applied, episode boundaries).
//!
//! `o_(t+1)` is not stored per tick (it is the next tick's `o_t`): the
//! recorder keeps the observation of the state after the last step and writes
//! it only into the `episode_end` record of a boundary (restart, respawn,
//! defeat, exit).
//!
//! The recorder never records the presim, captures, perf probes or headless
//! runs: the binary only builds one for a live-input run.

use std::path::Path;

use crate::autoplay::metrics::EventHasher;
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::input::{RawInput, SlotInputs};
use crate::party::{
    HeroControl, HeroId, PartyDrive, PartyMode, PartyState, PartyTuning, PlayerSlot,
    UncontrolledHero,
};

use super::codec::{
    ActionBits, ControlEvent, ControlSource, EpisodeBoundary, EpisodeReason, EventRecord,
    OBS_SCHEMA_ID, TickRecord,
};
use super::error::PlaytraceError;
use super::manifest::{SessionManifest, SessionStatus};
use super::obs::{ObsPair, diag_of, observe_pair};
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
    obs: Option<[String; 2]>,
    diag: Option<super::codec::DiagRecord>,
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
    reported_manifest_warning: bool,
    ticks: u64,
    /// The slot inputs come from a script or autopilot (a forced `--record`
    /// of a scripted run), so they are labelled `scripted_bot`.
    scripted: bool,
    /// ObservationV2 of both heroes for the state after the last completed step
    /// (the fresh state before the first): the `o_(t+1)` a boundary stores.
    after: Option<ObsPair>,
    /// The presentation rig's view rectangle for the next tick (diagnostic).
    rig_view: Option<[i32; 4]>,
    finished: bool,
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

fn source_of(
    party: &PartyState,
    hero: HeroId,
    drive: &PartyDrive,
    scripted: bool,
) -> ControlSource {
    match party.hero_control(hero) {
        // A forced `--record` of a scripted run: the slot inputs come from a
        // script or the autopilot, never from a person (F-3).
        HeroControl::Player(_) if scripted => ControlSource::ScriptedBot,
        HeroControl::Player(PlayerSlot::P1) => ControlSource::HumanP1,
        HeroControl::Player(PlayerSlot::P2) => ControlSource::HumanP2,
        // COMPANION-AI-01: the director's input is its own source.
        HeroControl::Uncontrolled if drive.companion_set(hero).is_some() => {
            ControlSource::CompanionAi
        }
        HeroControl::Uncontrolled if drive.is_bot(hero) => ControlSource::ScriptedBot,
        HeroControl::Uncontrolled => ControlSource::None,
    }
}

/// The `companion` manifest flag: who drives the uncontrolled hero, so a
/// verifier rebuilds the same tuning (the party tuning hash names it too).
pub fn companion_label(hero: UncontrolledHero) -> &'static str {
    match hero {
        UncontrolledHero::Companion(set) => set.name(),
        UncontrolledHero::FollowAndJumpBot => "off",
        UncontrolledHero::TrailFollowNeverAttacks => "trail",
    }
}

/// The data files whose FNV-1a pins a session stamps into `content_hashes` and a
/// replay checks (`source_fingerprint` hashes only `src/`, so a knob edit must
/// fail a replay loudly). Later work appends its pin here and nowhere else.
pub fn data_pins() -> Vec<(&'static str, u64)> {
    vec![
        ("mana_tuning", crate::donut_mana::TUNING_HASH),
        ("cf02_tuning", crate::cf02::TUNING_HASH),
        // CARL-PROG-01 CP-Q46.
        ("story_unlocks", crate::story_unlock::TABLE_HASH),
        ("carl_prog", crate::carl_prog::TUNING_HASH),
        // DONUT-MOBILITY-01.
        ("agility_tuning", crate::agility::TUNING_HASH),
        // DONUT-PROG-01 DP-Q46: stamped on every recording and required by a
        // replay, like `carl_prog`.
        (crate::donut_prog::PIN_KEY, crate::donut_prog::TUNING_HASH),
        // CORRAL-B core (CRL-Q40): the llama rush profile. Stamped now so a
        // recording made once the wire step reads it carries the hash.
        (crate::llama_rush::PIN_KEY, crate::llama_rush::TUNING_HASH),
        // HAZ-A1: the slime/acid and Rot Sticker knob files (HAZ-0 section 16),
        // stamped on every recording and required by a replay.
        (crate::slime::PIN_KEY, crate::slime::TUNING_HASH),
        (crate::rot_sticker::PIN_KEY, crate::rot_sticker::TUNING_HASH),
        // HRD-A1: the Hoarder boss knobs, stamped on every recording and
        // required by a replay like the other tuning pins.
        (crate::pile_boss::PIN_KEY, crate::pile_boss::TUNING_HASH),
        // BOIL-A wire: the Goblin Boiler room and its sidecar (`--scene
        // f01_boiler`). Stamped on every recording like the tunings, so an edit
        // of either file fails a replay that names the pin.
        (
            crate::f01_boiler::ROOM_PIN_KEY,
            crate::f01_boiler::ROOM_HASH,
        ),
        (
            crate::f01_boiler::ENCOUNTER_PIN_KEY,
            crate::f01_boiler::ENCOUNTER_HASH,
        ),
    ]
}

/// The `party_tuning` content hash a manifest records and a verifier compares:
/// the tuning with Donut's motion normalized to the shipped one. The motion is
/// pinned by the descriptor field and the `agility_tuning` hash, and a legacy
/// rebuild of a recording made before DM-A must still match the hash it was
/// recorded with.
pub(super) fn recorded_tuning_hash(tuning: &PartyTuning) -> u64 {
    crate::autoplay::party::party_tuning_hash(&PartyTuning {
        donut_motion: crate::party::PROVISIONAL_TUNING.donut_motion,
        ..*tuning
    })
}

/// Fill what every manifest of this checkout shares: the build identity
/// (embedded at build time), the ruleset and camera ids, the `cf02_rules` flag
/// and the content hashes a verifier compares. The caller adds the run's own
/// flags and identity (mode, participants, lab selection, ...).
pub fn stamp_manifest(manifest: &mut SessionManifest, kit_hash: u64) {
    let identity = super::build_info::build_identity();
    manifest.build_commit = identity.commit.clone();
    manifest.build_dirty = identity.dirty.unwrap_or(false);
    manifest.toolchain = identity.toolchain.clone();
    manifest.source_fingerprint = identity.source_fingerprint.clone();
    manifest.binary_fingerprint = identity.binary_fingerprint.clone();
    if identity.commit_is_not_enough() {
        manifest
            .quality_flags
            .push("dirty_or_unknown_build".to_owned());
    }
    manifest.game_ruleset_id = "floor01-greybox".to_owned();
    manifest.camera_ruleset = "sim-scroll-camera".to_owned();
    manifest.feature_flags.insert(
        "cf02_rules".to_owned(),
        manifest.reset.cf02_rules.to_string(),
    );
    // DONUT-MOBILITY-01: the motion Donut's body stepped with, next to the
    // descriptor field it must agree with.
    manifest.feature_flags.insert(
        "donut_motion".to_owned(),
        manifest.reset.donut_motion.name().to_owned(),
    );
    for (key, value) in std::iter::once(("kit", kit_hash)).chain(data_pins()) {
        manifest
            .content_hashes
            .insert(key.to_owned(), format!("{value:016x}"));
    }
}

impl LiveRecorder {
    /// Start a session before the first simulated tick: create the directory,
    /// write the manifest and open episode 0. `Err` means nothing is being
    /// recorded; the caller says so out loud and the game runs on. `scripted`
    /// labels the slot inputs `scripted_bot` instead of `human_p1`/`human_p2`.
    pub fn start(
        base: &Path,
        mut manifest: SessionManifest,
        sim: &SceneSim,
        combat: Option<&CombatState>,
        party: &PartyState,
        scripted: bool,
    ) -> Result<Self, PlaytraceError> {
        let session_id = manifest.session_id.clone();
        // PT-C: ObservationV2 per hero needs the combat state (the story route
        // has none), so only a combat session is a full observation record.
        manifest.content_hashes.insert(
            "party_tuning".to_owned(),
            format!("{:016x}", recorded_tuning_hash(party.tuning())),
        );
        manifest.feature_flags.insert(
            "companion".to_owned(),
            companion_label(party.tuning().uncontrolled_hero).to_owned(),
        );
        if combat.is_some() {
            manifest.observation_is_partial = false;
            manifest.observation_schema_id = OBS_SCHEMA_ID.to_owned();
        } else {
            manifest
                .quality_flags
                .push("no_observation_without_combat".to_owned());
        }
        let after = combat.map(|c| observe_pair(sim, c, party));
        let mut writer = SessionWriter::create(base, manifest)?;
        let stage = combat.map_or(0, |c| c.checkpoint as u32);
        writer.begin_episode(EpisodeBoundary {
            episode: 0,
            reason: EpisodeReason::Start,
            stage,
        })?;
        // No path: the caller adds a data-dir-relative location (no home path
        // in console logs that get pasted into evidence).
        let messages = vec![format!("PLAYTRACE_REC session={session_id}")];
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
            reported_manifest_warning: false,
            ticks: 0,
            scripted,
            after,
            rig_view: None,
            finished: false,
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
        let final_obs = self.after.as_ref().map(ObsPair::encode);
        self.control(ControlEvent::EpisodeEnd {
            episode: ended,
            outcome: outcome.to_owned(),
            final_obs,
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
    pub fn note_restart(
        &mut self,
        sim: &SceneSim,
        combat: Option<&CombatState>,
        party: &PartyState,
    ) {
        self.control(ControlEvent::Restart);
        let stage = combat.map_or(0, |c| c.checkpoint as u32);
        self.new_episode(EpisodeReason::Restart, "restart", stage);
        self.restarts = combat.map_or(0, |c| c.restarts);
        self.snap = PartySnap::of(party);
        // The boundary stored the state before the reset; from here on the
        // "after" observation is the fresh state's.
        self.after = combat.map(|c| observe_pair(sim, c, party));
    }

    /// A demo respawn (Select tap) just ran, before the step: the tick counter
    /// keeps running, a new episode begins. Its events are picked up by the
    /// next `end_tick` through the absolute cursor.
    pub fn note_demo_respawn(
        &mut self,
        sim: &SceneSim,
        combat: Option<&CombatState>,
        party: &PartyState,
    ) {
        self.control(ControlEvent::DemoRespawn);
        let stage = combat.map_or(0, |c| c.checkpoint as u32);
        self.new_episode(EpisodeReason::DemoRespawn, "demo_respawn", stage);
        self.snap = PartySnap::of(party);
        self.after = combat.map(|c| observe_pair(sim, c, party));
    }

    /// The presentation rig's view rectangle `[left, top, w, h]` for the next
    /// tick (called before [`Self::begin_tick`]). Diagnostic only: the
    /// observation never reads it.
    pub fn set_rig_view(&mut self, view: Option<[i32; 4]>) {
        self.rig_view = view;
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
        let (obs, diag) = match combat {
            Some(combat) => {
                let pair = observe_pair(sim, combat, party);
                let diag = diag_of(
                    self.writer.next_t(),
                    sim,
                    combat,
                    self.rig_view.take(),
                    pair.stats,
                );
                (Some(pair.encode()), Some(diag))
            }
            None => (None, None),
        };
        self.rig_view = None;
        self.pending = Some(Pending {
            obs,
            diag,
            pre: state_hash(sim, combat),
            slots,
            hero,
            src: [
                source_of(party, HeroId::Carl, &drive, self.scripted),
                source_of(party, HeroId::Donut, &drive, self.scripted),
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
            obs: pending.obs,
            // Only a live party has an AI hero (and an effective-input log).
            eff: sim.drive().live.then(|| {
                let eff = sim.last_effective_inputs();
                [ActionBits::from_raw(eff[0]), ActionBits::from_raw(eff[1])]
            }),
        };
        if let Err(error) = self.writer.record_tick(&record) {
            self.messages.push(format!(
                "PLAYTRACE_ERROR session={} {error}",
                self.session_id
            ));
        }
        if let Some(diag) = pending.diag.as_ref()
            && let Err(error) = self.writer.record_diag(diag)
        {
            self.messages.push(format!(
                "PLAYTRACE_ERROR session={} {error}",
                self.session_id
            ));
        }
        // The state after this step is `o_(t+1)`: kept for a boundary only.
        self.after = combat.map(|c| observe_pair(sim, c, party));
        self.ticks += 1;
        if let Some(combat) = combat {
            if combat.restarts != self.restarts {
                // A party defeat restarted the attempt inside the step.
                self.restarts = combat.restarts;
                let stage = combat.checkpoint as u32;
                self.new_episode(EpisodeReason::PartyDefeatRestart, "party_defeat", stage);
                self.snap = PartySnap::of(party);
            } else if respawned {
                // The step respawned Carl itself (demo defeat): an episode
                // boundary only. A control event would make a replay respawn
                // a second time.
                let stage = combat.checkpoint as u32;
                self.new_episode(
                    EpisodeReason::DemoDefeatRespawn,
                    "demo_defeat_respawn",
                    stage,
                );
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
        if health.manifest_write_errors > 0 && !self.reported_manifest_warning {
            self.reported_manifest_warning = true;
            self.messages.push(format!(
                "PLAYTRACE_WARN session={} {} intermediate manifest write(s) failed; no data was lost, the final manifest is rewritten at exit",
                self.session_id, health.manifest_write_errors
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
    /// (with its `o_(t+1)`: the truncation observation) and writes the final
    /// manifest. Idempotent: only the first call does anything. Errors are
    /// reported, not raised: a failing recorder never blocks the quit.
    pub fn finish(&mut self, reason: &str) -> Option<SessionManifest> {
        // A second call (the lab relaunch finishes the session and the app exit
        // finishes it again) is a silent no-op, never a PLAYTRACE_ERROR.
        if self.finished {
            return None;
        }
        self.finished = true;
        self.control(ControlEvent::Exit {
            reason: reason.to_owned(),
        });
        self.control(ControlEvent::EpisodeEnd {
            episode: self.episode,
            outcome: "exit".to_owned(),
            final_obs: self.after.as_ref().map(ObsPair::encode),
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
