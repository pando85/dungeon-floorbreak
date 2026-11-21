//! PTR-R0: the shared replay cursor (PLAYTRACE-RENDER-01).
//!
//! [`ReplayCursor`] is the verifier's per-tick state machine (it used to be the
//! private `Replayer` of `replay.rs`), made reusable: it **does not own** the
//! [`SceneSim`] or the [`CombatState`], its methods borrow them, and it does not
//! borrow the session either (it keeps journal positions and its own counters),
//! so a Bevy adapter can hold it in a `Resource` next to the sim resources that
//! stay the single owners of the world.
//!
//! - [`ReplayCursor::open`] runs the preflight for a [`ReplayPolicy`], rebuilds
//!   tick 0 from the reset descriptor, checks the content hashes and the start
//!   hash.
//! - [`ReplayCursor::next_tick`] applies the control records due at the next
//!   tick, runs every pre-check, steps the sim with the recorded per-hero
//!   inputs, runs every post-check and reports a [`TickOutcome`].
//! - `verify_loaded` is `open` plus a loop over `next_tick`: its signature,
//!   report, failure codes and tests are unchanged.
//!
//! # Policies
//!
//! [`ReplayPolicy::Strict`] is today's verifier. [`ReplayPolicy::CheckedCrossBuild`]
//! waives exactly one predicate, `manifest.source_fingerprint ==
//! SOURCE_FINGERPRINT`; an empty fingerprint is still rejected, and a matching
//! fingerprint is reported as [`Trust::Strict`]. A checked replay proves the
//! simulation equal to the recording tick by tick, so it is evidence for
//! viewing, never a `playtrace verify` PASS (ADR-0011: `replay_unverified` with
//! tick-by-tick parity). Nothing here prints a `PLAYTRACE_VERIFY ` line.
//!
//! # Limits
//!
//! The cursor supports exactly what `verify_loaded` supports (`RunKind::Human`
//! and `ScriptedBot`, every reset source `rebuild` handles). The party swap
//! hash `ph` is still not re-derived. Simulation replay only: no rendering,
//! audio or cosmetics.

use std::ops::Range;
use std::path::Path;

use crate::autoplay::metrics::EventHasher;
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::input::RawInput;
use crate::party::{HeroId, PartyDrive, PartyState, PartyTuning};

use super::codec::{
    ActionBits, ControlEvent, ControlRecord, ControlSource, DiagRecord, EventRecord, TickRecord,
};
use super::manifest::RunKind;
use super::obs::{ObsPair, diag_of, observe_pair};
use super::reader::LoadedSession;
use super::replay::{VerifyFailure, VerifyScope, check_content, preflight, rebuild};

/// How strictly a session must match this build to be replayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayPolicy {
    /// Today's verifier: the source fingerprint must match.
    Strict,
    /// Waives only the source-fingerprint predicate (an empty fingerprint is
    /// still rejected). Every other check stays on, tick by tick.
    CheckedCrossBuild,
}

/// What a successfully opened replay can claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trust {
    /// The source fingerprint matches this build (STRICT).
    Strict,
    /// The fingerprint differs and the policy waived it: the replay is
    /// checked tick by tick, but this build is not the recording build.
    CheckedCrossBuild,
}

impl Trust {
    /// The `result=` word of the `PLAYTRACE_REPLAY` line.
    pub fn result_word(self) -> &'static str {
        match self {
            Self::Strict => "STRICT",
            Self::CheckedCrossBuild => "CHECKED",
        }
    }
}

/// Counters of what the cursor checked so far (the numbers of a `VerifyReport`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReplayCounters {
    pub ticks: u64,
    pub combat_events: u64,
    pub state_hashes: u64,
    pub event_hashes: u64,
    pub effective_inputs: u64,
    pub observations: u64,
    pub final_observations: u64,
    pub diagnostics: u64,
    pub routing: u64,
    pub labels: u64,
}

/// What one replayed tick did, for an adapter that presents it.
#[derive(Debug, Clone, PartialEq)]
pub struct TickOutcome {
    /// Global tick of the journal (`TickRecord::t`).
    pub t: u64,
    /// Episode of the tick and the episode tick after the step (`et`).
    pub ep: u32,
    pub et: u64,
    /// The control records applied just before this tick's step, in order.
    pub applied: Vec<ControlEvent>,
    /// A manual `restart` control was applied before the step.
    pub restarted: bool,
    /// The step itself ran a DEMO-01 respawn (`take_respawned`).
    pub respawned_in_step: bool,
    /// The step itself restarted the attempt (a party defeat: `combat.restarts`
    /// changed inside the step).
    pub defeat_restart_in_step: bool,
    /// The party drive in force for this tick.
    pub drive: PartyDrive,
    /// The recorded (and verified) control-source labels.
    pub src: [ControlSource; 2],
    /// Indices of the combat events this tick produced (the cursor of the
    /// event stream, shared with `EventRecord::i`).
    pub events: Range<u64>,
}

impl TickOutcome {
    /// A control record of this tick moved Carl on purpose (a restart or a
    /// DEMO-01 respawn), or the step did: the presentation resets and cuts.
    pub fn lifecycle_reset(&self) -> bool {
        self.restarted
            || self.respawned_in_step
            || self.defeat_restart_in_step
            || self
                .applied
                .iter()
                .any(|c| matches!(c, ControlEvent::DemoRespawn))
    }
}

/// The state hash of the world the replay compares: the combat state's when
/// there is one, the story scene's otherwise (the helper `replay.rs` used).
pub fn state_hash_of(scene: &SceneSim, combat: Option<&CombatState>) -> u64 {
    combat.map_or_else(|| scene.state_hash(), |c| c.state_hash(scene))
}

/// The routing rule of `PartyState::hero_inputs` for the recorded drive: in
/// two-player Carl gets slot 0 and Donut slot 1; in solo the controlled hero
/// gets slot 0 and the other idles; swap bits are stripped. (D6 pause records
/// no tick, so the paused rule never applies to a recorded one.)
fn expected_routing(drive: &PartyDrive, slots: [RawInput; 2]) -> [RawInput; 2] {
    let strip = |raw: RawInput| RawInput { swap: false, ..raw };
    let mut out = [RawInput::default(); 2];
    if drive.two_player {
        out[0] = strip(slots[0]);
        out[1] = strip(slots[1]);
    } else {
        out[drive.controlled.index()] = strip(slots[0]);
    }
    out
}

/// Who drove each hero this tick, derived from the recorded drive and the run
/// kind alone (F-2). Two-player: Carl is `human_p1`, Donut `human_p2`. Solo: the
/// controlled hero is `human_p1`; the other is the companion director
/// (`companion_ai`) when the tuning names one, the follower bot
/// (`scripted_bot`) when it is a bot, and `none` otherwise. A scripted run
/// (autopilot, script) labels every player-driven hero `scripted_bot`, never
/// human.
fn expected_sources(drive: &PartyDrive, scripted: bool) -> [ControlSource; 2] {
    let mut out = [ControlSource::None; 2];
    for hero in [HeroId::Carl, HeroId::Donut] {
        let slot = if drive.two_player {
            Some(if hero == HeroId::Carl {
                ControlSource::HumanP1
            } else {
                ControlSource::HumanP2
            })
        } else if hero == drive.controlled {
            Some(ControlSource::HumanP1)
        } else {
            None
        };
        out[hero.index()] = match slot {
            Some(_) if scripted => ControlSource::ScriptedBot,
            Some(human) => human,
            None if drive.companion_set(hero).is_some() => ControlSource::CompanionAi,
            None if drive.is_bot(hero) => ControlSource::ScriptedBot,
            None => ControlSource::None,
        };
    }
    out
}

fn bits(raw: [RawInput; 2]) -> [ActionBits; 2] {
    [ActionBits::from_raw(raw[0]), ActionBits::from_raw(raw[1])]
}

fn hero_of(index: u8) -> HeroId {
    if index == 0 {
        HeroId::Carl
    } else {
        HeroId::Donut
    }
}

/// The replay state machine of one session. It owns no world: every method
/// that touches the simulation takes it by reference.
#[derive(Debug, Clone)]
pub struct ReplayCursor {
    policy: ReplayPolicy,
    trust: Trust,
    scope: VerifyScope,
    build: &'static str,
    session_id: String,
    total_ticks: usize,
    party: PartyState,
    tuning: PartyTuning,
    drive: Option<PartyDrive>,
    events: EventHasher,
    /// Index of the next combat event of the stream.
    event_cursor: u64,
    /// ObservationV2 of the state after the last completed step (the fresh
    /// state before the first): the `o_(t+1)` a boundary stores.
    after: Option<ObsPair>,
    /// The state before a restart or respawn applied this boundary.
    boundary: Option<ObsPair>,
    episode: u32,
    observe: bool,
    /// The manifest says a script or autopilot supplied the slot inputs.
    scripted: bool,
    /// A crash-recovered prefix or a counted drop may lack a diagnostics line.
    diag_may_be_missing: bool,
    /// Journal positions (indices into the session's streams).
    next_tick: usize,
    next_control: usize,
    next_diag: usize,
    counters: ReplayCounters,
    /// The first failure: the cursor never advances after it.
    failed: bool,
    finished: bool,
}

impl ReplayCursor {
    /// Preflight under `policy`, rebuild tick 0 from the reset descriptor,
    /// check content and the start hash. The scene and combat state come back
    /// by value: the caller owns them from now on.
    #[allow(clippy::type_complexity)]
    pub fn open(
        loaded: &LoadedSession,
        root: &Path,
        policy: ReplayPolicy,
    ) -> Result<
        (
            ReplayCursor,
            SceneSim,
            Option<CombatState>,
            PartyTuning,
            Trust,
        ),
        VerifyFailure,
    > {
        let manifest = &loaded.manifest;
        // Build and rules first: a different build is reported as such, never
        // as whatever it happens to break while rebuilding the scene.
        let (scope, build, trust) = preflight(loaded, policy)?;
        let reset = rebuild(manifest, root)?;
        check_content(manifest, root, &reset.tuning)?;
        let start = state_hash_of(&reset.scene, reset.combat.as_ref());
        if start != manifest.reset.start_hash {
            return Err(VerifyFailure::new(
                "reset_mismatch",
                "the fresh reset does not hash to the recorded start hash (PTI-05: this is the legacy-drive hash, taken before the first drive_change)",
            ));
        }
        let scripted = match manifest.run_kind {
            RunKind::Human => false,
            RunKind::ScriptedBot => true,
            RunKind::LearnedPolicy | RunKind::Mixed => {
                return Err(VerifyFailure::new(
                    "labels",
                    "the run kind has no control-source derivation in this verifier",
                ));
            }
        };
        let observe = !manifest.observation_is_partial;
        if observe && reset.combat.is_none() {
            return Err(VerifyFailure::new(
                "rules_mismatch",
                "the session claims full observations on a route with no combat state",
            ));
        }
        let mut cursor = ReplayCursor {
            policy,
            trust,
            scope,
            build,
            session_id: manifest.session_id.clone(),
            total_ticks: loaded.ticks.len(),
            party: PartyState::new(reset.tuning),
            tuning: reset.tuning,
            drive: None,
            events: EventHasher::new(),
            event_cursor: 0,
            after: None,
            boundary: None,
            episode: 0,
            observe,
            scripted,
            diag_may_be_missing: scope == VerifyScope::Prefix || manifest.dropped_diagnostics > 0,
            next_tick: 0,
            next_control: 0,
            next_diag: 0,
            counters: ReplayCounters::default(),
            failed: false,
            finished: false,
        };
        cursor.refresh_after(&reset.scene, reset.combat.as_ref());
        Ok((cursor, reset.scene, reset.combat, reset.tuning, trust))
    }

    pub fn policy(&self) -> ReplayPolicy {
        self.policy
    }

    pub fn trust(&self) -> Trust {
        self.trust
    }

    pub fn scope(&self) -> VerifyScope {
        self.scope
    }

    /// `identical`, `source_identical_commit_differs` or `cross_build`.
    pub fn build(&self) -> &'static str {
        self.build
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Ticks the journal holds.
    pub fn total_ticks(&self) -> u64 {
        self.total_ticks as u64
    }

    /// Ticks replayed (and fully checked) so far.
    pub fn ticks_done(&self) -> u64 {
        self.counters.ticks
    }

    pub fn counters(&self) -> ReplayCounters {
        self.counters
    }

    /// The drive in force (`None` before the first `drive_change`).
    pub fn drive(&self) -> Option<PartyDrive> {
        self.drive
    }

    pub fn tuning(&self) -> &PartyTuning {
        &self.tuning
    }

    /// The episode of the last applied `episode_begin`.
    pub fn episode(&self) -> u32 {
        self.episode
    }

    /// True once `next_tick` has returned `None` (the journal is consumed).
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    fn refresh_after(&mut self, scene: &SceneSim, combat: Option<&CombatState>) {
        self.after = combat.map(|c| observe_pair(scene, c, &self.party));
    }

    /// Apply one control record (the lifecycle half of the replay).
    fn control(
        &mut self,
        record: &ControlRecord,
        scene: &mut SceneSim,
        combat: &mut Option<&mut CombatState>,
    ) -> Result<(), VerifyFailure> {
        match &record.ev {
            ControlEvent::Restart => {
                self.boundary = self.after.clone();
                match combat.as_deref_mut() {
                    Some(combat) => combat.restart(scene),
                    None => scene.reset(),
                }
                self.refresh_after(scene, combat.as_deref());
            }
            ControlEvent::DemoRespawn => {
                self.boundary = self.after.clone();
                if let Some(combat) = combat.as_deref_mut() {
                    combat.respawn(scene);
                    let _ = combat.take_respawned();
                }
                self.refresh_after(scene, combat.as_deref());
            }
            ControlEvent::DriveChange {
                live,
                two_player,
                controlled,
                bots,
            } => {
                let drive = PartyDrive {
                    live: *live,
                    two_player: *two_player,
                    controlled: hero_of(*controlled),
                    bots: *bots,
                    tuning: self.tuning,
                };
                scene.set_drive(drive);
                self.drive = Some(drive);
            }
            ControlEvent::EpisodeBegin { episode, .. } => self.episode = *episode,
            ControlEvent::EpisodeEnd { final_obs, .. } => {
                let expected = self.boundary.take().or_else(|| self.after.clone());
                match (&expected, final_obs) {
                    (Some(expected), Some(recorded)) => {
                        if expected.encode() != *recorded {
                            return Err(VerifyFailure::at(
                                "final_observation",
                                record.at_t,
                                "o_(t+1) at an episode boundary differs from the replay",
                            ));
                        }
                        self.counters.final_observations += 2;
                    }
                    (Some(_), None) if self.observe => {
                        return Err(VerifyFailure::at(
                            "final_observation",
                            record.at_t,
                            "an episode boundary has no o_(t+1)",
                        ));
                    }
                    (None, Some(_)) => {
                        return Err(VerifyFailure::at(
                            "final_observation",
                            record.at_t,
                            "o_(t+1) recorded without a combat state",
                        ));
                    }
                    _ => {}
                }
            }
            // Informational: the pad, pause, swap, checkpoint and exit events
            // have no effect the step does not re-derive.
            _ => {}
        }
        Ok(())
    }

    /// Replay the next recorded tick: apply the control records due at it, run
    /// the pre-checks, step with the recorded per-hero inputs, run the
    /// post-checks. `Ok(None)` after the last tick, once the trailing controls
    /// are applied. After an error the cursor stays failed: it returns `None`.
    ///
    /// `loaded` must be the session the cursor was opened on.
    pub fn next_tick(
        &mut self,
        loaded: &LoadedSession,
        scene: &mut SceneSim,
        combat: Option<&mut CombatState>,
    ) -> Result<Option<TickOutcome>, VerifyFailure> {
        if self.failed || self.finished {
            return Ok(None);
        }
        debug_assert_eq!(loaded.manifest.session_id, self.session_id);
        let mut combat = combat;
        let result = self.advance(loaded, scene, &mut combat);
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn advance(
        &mut self,
        loaded: &LoadedSession,
        scene: &mut SceneSim,
        combat: &mut Option<&mut CombatState>,
    ) -> Result<Option<TickOutcome>, VerifyFailure> {
        let Some(tick) = loaded.ticks.get(self.next_tick) else {
            while let Some(record) = loaded.control.get(self.next_control) {
                self.next_control += 1;
                self.control(record, scene, combat)?;
            }
            self.finished = true;
            return Ok(None);
        };
        let mut applied = Vec::new();
        let mut restarted = false;
        while let Some(record) = loaded
            .control
            .get(self.next_control)
            .filter(|c| c.at_t <= tick.t)
        {
            self.next_control += 1;
            restarted |= record.ev == ControlEvent::Restart;
            applied.push(record.ev.clone());
            self.control(record, scene, combat)?;
        }
        while loaded
            .diag
            .get(self.next_diag)
            .is_some_and(|d| d.t < tick.t)
        {
            self.next_diag += 1;
        }
        let diag = loaded.diag.get(self.next_diag).filter(|d| d.t == tick.t);
        if diag.is_some() {
            self.next_diag += 1;
        }
        let (drive, events, respawned_in_step, defeat_restart_in_step) =
            self.tick(tick, diag, scene, combat)?;
        self.next_tick += 1;
        self.counters.ticks += 1;
        Ok(Some(TickOutcome {
            t: tick.t,
            ep: tick.ep,
            et: tick.et,
            applied,
            restarted,
            respawned_in_step,
            defeat_restart_in_step,
            drive,
            src: tick.src,
            events,
        }))
    }

    /// One recorded tick: check, step, compare. Returns the drive, the event
    /// index range and the two in-step lifecycle flags.
    fn tick(
        &mut self,
        tick: &TickRecord,
        diag: Option<&DiagRecord>,
        scene: &mut SceneSim,
        combat: &mut Option<&mut CombatState>,
    ) -> Result<(PartyDrive, Range<u64>, bool, bool), VerifyFailure> {
        let t = tick.t;
        let drive = self
            .drive
            .ok_or_else(|| VerifyFailure::at("control", t, "a tick before any drive_change"))?;
        if tick.ep != self.episode {
            return Err(VerifyFailure::at(
                "episode",
                t,
                format!(
                    "tick is in episode {} but the journal is in {}",
                    tick.ep, self.episode
                ),
            ));
        }
        if state_hash_of(scene, combat.as_deref()) != tick.pre {
            return Err(VerifyFailure::at("pre_state", t, "pre-state hash differs"));
        }
        // The provenance inputs must route to the replay driver through the
        // recorded drive.
        let slots = [tick.slots[0].to_raw(), tick.slots[1].to_raw()];
        let hero = [tick.hero[0].to_raw(), tick.hero[1].to_raw()];
        if bits(expected_routing(&drive, slots)) != tick.hero {
            return Err(VerifyFailure::at(
                "slot_routing",
                t,
                "the routed per-hero inputs do not follow from the per-slot inputs and the drive",
            ));
        }
        self.counters.routing += 1;

        // F-2: the control-source labels and the stage are derived data too.
        // A human/AI relabel or a forged stage must not pass as a replay.
        if tick.src != expected_sources(&drive, self.scripted) {
            return Err(VerifyFailure::at(
                "labels",
                t,
                format!(
                    "the control sources {:?} do not follow from the drive (expected {:?})",
                    tick.src,
                    expected_sources(&drive, self.scripted)
                ),
            ));
        }
        let stage = combat.as_deref().map_or(0, |c| c.checkpoint as u32);
        if tick.stage != stage {
            return Err(VerifyFailure::at(
                "labels",
                t,
                format!(
                    "the recorded stage {} differs from the replayed checkpoint {stage}",
                    tick.stage
                ),
            ));
        }
        self.counters.labels += 1;

        // o_t, before the live decision, from the same state the hash covers.
        if let Some(combat) = combat.as_deref()
            && self.observe
        {
            let pair = observe_pair(scene, combat, &self.party);
            let stats = pair.stats;
            let Some(recorded) = tick.obs.as_ref() else {
                return Err(VerifyFailure::at("observation", t, "o_t is missing"));
            };
            if pair.encode() != *recorded {
                return Err(VerifyFailure::at(
                    "observation",
                    t,
                    "o_t differs from the observation rebuilt before the step",
                ));
            }
            self.counters.observations += 2;
            if let Some(diag) = diag {
                let mut rebuilt = diag_of(t, scene, combat, diag.rig, stats);
                rebuilt.rig = diag.rig;
                if rebuilt != *diag {
                    return Err(VerifyFailure::at(
                        "diagnostics",
                        t,
                        "the privileged diagnostics differ from the replay",
                    ));
                }
                self.counters.diagnostics += 1;
            } else if !self.diag_may_be_missing {
                return Err(VerifyFailure::at(
                    "diagnostics",
                    t,
                    "diagnostics line missing",
                ));
            }
        }

        let mut respawned_in_step = false;
        let mut defeat_restart_in_step = false;
        match combat.as_deref_mut() {
            Some(combat) => {
                let restarts = combat.restarts;
                combat.step_party(scene, hero);
                respawned_in_step = combat.take_respawned();
                defeat_restart_in_step = combat.restarts != restarts;
            }
            None => scene.step_party(hero),
        }

        // The effective inputs the sim applied: live log against re-derived.
        match (tick.eff, scene.drive().live) {
            (Some(recorded), true) => {
                if recorded != bits(scene.last_effective_inputs()) {
                    return Err(VerifyFailure::at(
                        "effective_input",
                        t,
                        "the live effective inputs differ from the re-derived AI inputs",
                    ));
                }
                self.counters.effective_inputs += 1;
            }
            (None, false) => {}
            (Some(_), false) => {
                return Err(VerifyFailure::at(
                    "effective_input",
                    t,
                    "effective inputs recorded for a drive that is not live",
                ));
            }
            (None, true) => {
                return Err(VerifyFailure::at(
                    "effective_input",
                    t,
                    "a live party tick has no effective-input log",
                ));
            }
        }

        if state_hash_of(scene, combat.as_deref()) != tick.post {
            return Err(VerifyFailure::at(
                "post_state",
                t,
                "post-state hash differs",
            ));
        }
        self.counters.state_hashes += 1;
        let first_event = self.event_cursor;
        let replayed: Vec<EventRecord> = match combat.as_deref() {
            Some(combat) => {
                let dropped = combat.dropped_events as u64;
                let from =
                    (self.event_cursor.saturating_sub(dropped) as usize).min(combat.events.len());
                let fresh = &combat.events[from..];
                self.events.fold(fresh);
                let out = fresh
                    .iter()
                    .enumerate()
                    .map(|(i, e)| EventRecord::from_combat(self.event_cursor + i as u64, e))
                    .collect();
                self.event_cursor = dropped + combat.events.len() as u64;
                out
            }
            None => Vec::new(),
        };
        if replayed != tick.ev {
            return Err(VerifyFailure::at(
                "events",
                t,
                "the ordered combat events differ",
            ));
        }
        if self.events.value() != tick.eh {
            return Err(VerifyFailure::at(
                "event_hash",
                t,
                "the rolling event hash differs",
            ));
        }
        self.counters.event_hashes += 1;
        self.counters.combat_events += replayed.len() as u64;
        if scene.world.tick != tick.et {
            return Err(VerifyFailure::at(
                "episode_tick",
                t,
                "the episode tick differs",
            ));
        }
        self.refresh_after(scene, combat.as_deref());
        Ok((
            drive,
            first_event..self.event_cursor,
            respawned_in_step,
            defeat_restart_in_step,
        ))
    }
}

/// How a replay ended, for the one `PLAYTRACE_REPLAY` result line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayResult {
    Strict,
    Checked,
    Diverged { code: String, tick: Option<u64> },
    Unsupported { code: String },
}

impl ReplayResult {
    pub fn of_trust(trust: Trust) -> Self {
        match trust {
            Trust::Strict => Self::Strict,
            Trust::CheckedCrossBuild => Self::Checked,
        }
    }
}

/// The single result line of the viewer (PTR-Q14). It always starts with
/// `PLAYTRACE_REPLAY ` and is never a `PLAYTRACE_VERIFY ` line, so no consumer
/// of `playtrace verify` output can mistake a replay for a verification.
/// `post` is the state hash after the last tick the replay matched.
pub fn replay_result_line(
    session: &str,
    result: &ReplayResult,
    ticks: u64,
    post: Option<u64>,
) -> String {
    let (word, tail) = match result {
        ReplayResult::Strict => ("STRICT", String::new()),
        ReplayResult::Checked => ("CHECKED", String::new()),
        ReplayResult::Diverged { code, tick } => (
            "DIVERGED",
            match tick {
                Some(tick) => format!(" code={code} tick={tick}"),
                None => format!(" code={code}"),
            },
        ),
        ReplayResult::Unsupported { code } => ("UNSUPPORTED", format!(" code={code}")),
    };
    let post = post.map_or_else(|| "none".to_owned(), |p| format!("{p:016x}"));
    format!("PLAYTRACE_REPLAY session={session} result={word} ticks={ticks} post={post}{tail}")
}

/// The session directory a user-given id names: an existing session directory
/// (a path with a manifest), or a session id or unique prefix of one inside
/// `base` (the same rule as `playtrace list` and `playtrace verify`).
pub fn resolve_session(base: &Path, id: &str) -> Result<std::path::PathBuf, String> {
    let direct = Path::new(id);
    if direct.join("manifest.json").is_file() {
        return Ok(direct.to_path_buf());
    }
    let matches: Vec<std::path::PathBuf> = super::reader::list_sessions(base)
        .into_iter()
        .filter(|(_, m)| m.session_id.starts_with(id))
        .map(|(path, _)| path)
        .collect();
    match matches.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(format!("no session matches {id:?}")),
        _ => Err(format!(
            "{id:?} matches {} sessions: use more digits",
            matches.len()
        )),
    }
}
