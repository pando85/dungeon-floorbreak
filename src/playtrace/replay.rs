//! PT-C: the fresh-process verifier (`playtrace verify`).
//!
//! A recorded session is replayed **from its reset descriptor** through
//! `CombatState::step_party` (or `SceneSim::step_party` on the story route),
//! feeding the recorded per-hero routed inputs, the `drive_change` events and
//! the lifecycle events that happen outside the step (`restart`,
//! `demo_respawn`). Nothing is injected for the AI: the follower bot is
//! re-derived by the sim, and its effective inputs are compared with the live
//! `last_effective_inputs` log. At every tick the verifier compares:
//!
//! - the pre-state and post-state hashes, the rolling event hash, every event
//!   and the episode tick;
//! - the effective inputs the sim applied (live log vs re-derived);
//! - the per-hero ObservationV2 `o_t` (rebuilt **before** the step with the one
//!   shared builder) and, at each episode boundary, `o_(t+1)`;
//! - the privileged diagnostics (everything except the rig rectangle, which has
//!   no replay);
//! - the routing of the per-slot provenance inputs through the recorded drive.
//!
//! It rejects a different build (source fingerprint), content (kit and tuning
//! hashes), rules (schemas, CF02 flag) and anything missing or tampered.
//!
//! This is a **simulation** replay. It does not reproduce rendering, audio,
//! cosmetics or the presentation camera rig, and it does not claim to. The
//! party swap-timer hash (`ph`) is not re-derived: `PartyState` pad and swap
//! history is not reconstructible from the journal (PadIds are deliberately not
//! recorded), so it is reported as not checked.

use std::path::Path;

use crate::autoplay::metrics::EventHasher;
use crate::autoplay::replay::kit_content_hash;
use crate::cf02_fixtures::find_party_fixture;
use crate::combat_scene::{CombatState, Fixture};
use crate::corridor::SceneSim;
use crate::donut_mana::{DonutSetup, StartState};
use crate::env_kit::EnvKit;
use crate::input::RawInput;
use crate::party::{HeroId, PartyDrive, PartyState, PartyTuning};
use crate::party_scene::PartyFixtureSim;

use super::build_info::{BUILD_COMMIT, SOURCE_FINGERPRINT};
use super::codec::{
    ACTION_SCHEMA, ACTION_SCHEMA_V1, ActionBits, ControlEvent, ControlRecord, ControlSource,
    EVENT_SCHEMA, EventRecord, OBS_SCHEMA_ID, TickRecord,
};
use super::manifest::{RunKind, SessionManifest, SessionStatus};
use super::obs::{ObsPair, diag_of, observe_pair};
use super::reader::{LoadedSession, read_session};

const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

/// How much of the session the verdict covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyScope {
    /// A finished session: every tick from the reset to the exit.
    Full,
    /// A crash-recovered session: the valid complete prefix only.
    Prefix,
}

impl VerifyScope {
    pub fn name(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Prefix => "prefix",
        }
    }
}

/// What a passing verification checked, and how often.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyReport {
    pub session_id: String,
    pub scope: VerifyScope,
    pub ticks: u64,
    pub episodes: u32,
    pub combat_events: u64,
    pub control_events: usize,
    /// Ticks whose pre and post state hashes matched.
    pub state_hashes: u64,
    pub event_hashes: u64,
    pub effective_inputs: u64,
    /// `o_t` vectors compared (two heroes per tick).
    pub observations: u64,
    /// `o_(t+1)` vectors compared at episode boundaries.
    pub final_observations: u64,
    pub diagnostics: u64,
    pub routing: u64,
    /// Ticks whose control-source labels and stage matched the replay.
    pub labels: u64,
    /// `identical` (same commit) or `source_identical_commit_differs`.
    pub build: &'static str,
    pub notes: Vec<String>,
}

/// Why a session is not verified. `code` is stable (tests and scripts match
/// it); `message` is for people.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyFailure {
    pub code: &'static str,
    pub message: String,
    pub tick: Option<u64>,
}

impl VerifyFailure {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            tick: None,
        }
    }

    fn at(code: &'static str, tick: u64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            tick: Some(tick),
        }
    }
}

impl std::fmt::Display for VerifyFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)?;
        if let Some(tick) = self.tick {
            write!(f, " (tick {tick})")?;
        }
        Ok(())
    }
}

/// Read a session directory and verify it.
pub fn verify_dir(dir: &Path, root: &Path) -> Result<VerifyReport, VerifyFailure> {
    let loaded = read_session(dir, true)
        .map_err(|e| VerifyFailure::new("invalid_session", e.to_string()))?;
    verify_loaded(&loaded, root)
}

/// The scene a reset descriptor rebuilds.
struct Reset {
    scene: SceneSim,
    combat: Option<CombatState>,
    tuning: PartyTuning,
}

fn corridor_kit(root: &Path) -> Result<EnvKit, VerifyFailure> {
    EnvKit::load(root, LAYOUT).map_err(|e| VerifyFailure::new("content_mismatch", e.to_string()))
}

/// Donut's setup, by the rule of `donut_setup` in `floor01_scene` (a lab run
/// brings its own).
fn donut_setup(reset: &super::manifest::ResetDescriptor, boss: bool) -> DonutSetup {
    if reset.donut_biscuit || boss {
        return DonutSetup::unlocked(8);
    }
    if reset.demo {
        return DonutSetup::NONE;
    }
    DonutSetup {
        start: StartState::LOCKED,
        guild_zone: Some(crate::donut_fixtures::GUILD_PLACEHOLDER_ZONE),
    }
}

fn combat_for(
    reset: &super::manifest::ResetDescriptor,
    fixture: &'static Fixture,
    kit: &EnvKit,
    scene: &SceneSim,
    boss: bool,
) -> Result<CombatState, VerifyFailure> {
    let mut combat = CombatState::new(fixture, kit, scene)
        .map_err(|e| VerifyFailure::new("content_mismatch", e))?
        .with_donut(donut_setup(reset, boss));
    if reset.cf02_rules && reset.demo {
        // CF02-F: `--cf02` is the microarena on the demo machinery.
        combat = combat.with_demo(&crate::cf02::arena::RULES);
    } else if reset.demo {
        combat = combat.with_demo(&crate::demo::DEMO_RULES);
    }
    if reset.cf02_rules {
        combat = combat.with_cf02().with_cf02_seed(reset.cf02_seed);
    }
    Ok(combat)
}

/// Rebuild tick 0 from the descriptor (the `PartyFixtureSim` style: no binary
/// code is copied; the same lib constructors the binary calls).
/// The party tuning of a live run on a route without a fixture-declared one:
/// the provisional tuning with the recorded `companion` choice (a manifest
/// without the flag is the follow-and-jump bot, as before COMPANION-AI-01).
fn live_tuning(manifest: &SessionManifest) -> Result<PartyTuning, VerifyFailure> {
    use crate::companion::profile::ProfileSetId;
    use crate::party::UncontrolledHero;
    let uncontrolled_hero = match manifest.feature_flags.get("companion").map(String::as_str) {
        None | Some("off") => UncontrolledHero::FollowAndJumpBot,
        Some("cf02_assist") => UncontrolledHero::Companion(ProfileSetId::Cf02Assist),
        Some("companion_v0") => UncontrolledHero::Companion(ProfileSetId::CompanionV0),
        Some(other) => {
            return Err(VerifyFailure::new(
                "rules_mismatch",
                format!("unknown companion flag {other:?}"),
            ));
        }
    };
    let tuning = PartyTuning {
        uncontrolled_hero,
        ..PartyTuning::default()
    };
    // CF02-Q19, Q20: the live `--cf02` party has the 12-tick swap and the eased pan.
    Ok(if manifest.reset.cf02_rules {
        tuning.with_cf02_swap()
    } else {
        tuning
    })
}

fn rebuild(manifest: &SessionManifest, root: &Path) -> Result<Reset, VerifyFailure> {
    let reset = &manifest.reset;
    let unsupported = |why: String| VerifyFailure::new("unsupported_source", why);
    match reset.source.as_str() {
        "party_fixture" => {
            let fixture = find_party_fixture(&reset.name)
                .ok_or_else(|| unsupported(format!("unknown party fixture {:?}", reset.name)))?;
            // CF02-F: the room fixtures run on the microarena kit.
            let kit = if crate::cf02::arena::fixture_uses_arena(fixture.name()) {
                crate::cf02::arena::load_arena(root).map_err(unsupported)?
            } else {
                corridor_kit(root)?
            };
            let sim = PartyFixtureSim::new(&kit, reset.view_height, fixture)
                .map_err(|e| VerifyFailure::new("content_mismatch", e))?;
            Ok(Reset {
                scene: sim.scene,
                combat: Some(sim.combat),
                tuning: fixture.tuning,
            })
        }
        "story" => {
            let scene = SceneSim::new(&corridor_kit(root)?, reset.view_height)
                .map_err(|e| VerifyFailure::new("content_mismatch", e))?;
            Ok(Reset {
                scene,
                combat: None,
                tuning: live_tuning(manifest)?,
            })
        }
        "fixture" | "boss" | "demo" => {
            let boss = reset.source == "boss";
            let (kit, fixture) = if reset.cf02_rules && (reset.demo || reset.source == "demo") {
                let kit = crate::cf02::arena::load_arena(root).map_err(unsupported)?;
                (
                    kit,
                    crate::cf02::arena::fixture_for_beat(reset.demo_beat as usize),
                )
            } else if reset.demo || reset.source == "demo" {
                let kit = crate::demo::load_hall(root).map_err(unsupported)?;
                (kit, crate::demo::fixture_for_beat(reset.demo_beat as usize))
            } else if boss {
                let kit = crate::boss_scene::load_arena(root).map_err(unsupported)?;
                (kit, crate::boss_scene::BOSS01_SCENE)
            } else {
                let fixture = Fixture::by_name(&reset.name)
                    .ok_or_else(|| unsupported(format!("unknown fixture {:?}", reset.name)))?;
                (corridor_kit(root)?, fixture)
            };
            let scene = SceneSim::new_at(&kit, reset.view_height, fixture.carl_x)
                .map_err(|e| VerifyFailure::new("content_mismatch", e))?;
            let combat = combat_for(reset, fixture, &kit, &scene, boss)?;
            Ok(Reset {
                scene,
                combat: Some(combat),
                tuning: live_tuning(manifest)?,
            })
        }
        "combat_lab" => rebuild_lab(manifest, root),
        other => Err(unsupported(format!("source {other:?} is not replayable"))),
    }
}

fn rebuild_lab(manifest: &SessionManifest, root: &Path) -> Result<Reset, VerifyFailure> {
    use crate::combat_lab::{EntryPoint, LabCatalog, LabConfig, PlayerMode};
    let flag =
        |key: &str| {
            manifest.feature_flags.get(key).cloned().ok_or_else(|| {
                VerifyFailure::new("rules_mismatch", format!("manifest has no {key}"))
            })
        };
    let mode = PlayerMode::parse(&flag("lab_mode")?)
        .ok_or_else(|| VerifyFailure::new("rules_mismatch", "unknown lab_mode"))?;
    let catalog = LabCatalog::load(root)
        .map_err(|e| VerifyFailure::new("content_mismatch", e.to_string()))?;
    let mut config = LabConfig::new(&flag("lab_room")?, mode);
    config.variant = flag("lab_variant")?;
    config.profile = flag("lab_profile")?;
    config.view_h = manifest.reset.view_height;
    config.cf02.rules = manifest.reset.cf02_rules;
    config.cf02.seed = manifest.reset.cf02_seed;
    let run = catalog
        .launch(EntryPoint::CombatLab, &config)
        .map_err(|e| VerifyFailure::new("content_mismatch", e.to_string()))?;
    let tuning = *run.party.tuning();
    Ok(Reset {
        scene: run.scene,
        combat: Some(run.combat),
        tuning,
    })
}

fn state_hash(scene: &SceneSim, combat: Option<&CombatState>) -> u64 {
    combat.map_or_else(|| scene.state_hash(), |c| c.state_hash(scene))
}

fn hero_of(index: u8) -> HeroId {
    if index == 0 {
        HeroId::Carl
    } else {
        HeroId::Donut
    }
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

/// The replay state machine.
struct Replayer<'a> {
    scene: SceneSim,
    combat: Option<CombatState>,
    party: PartyState,
    tuning: PartyTuning,
    drive: Option<PartyDrive>,
    events: EventHasher,
    cursor: u64,
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
    report: &'a mut VerifyReport,
}

impl Replayer<'_> {
    fn refresh_after(&mut self) {
        self.after = self
            .combat
            .as_ref()
            .map(|c| observe_pair(&self.scene, c, &self.party));
    }

    /// Apply one control record (the verifier's half of the lifecycle).
    fn control(&mut self, record: &ControlRecord) -> Result<(), VerifyFailure> {
        match &record.ev {
            ControlEvent::Restart => {
                self.boundary = self.after.clone();
                match self.combat.as_mut() {
                    Some(combat) => combat.restart(&mut self.scene),
                    None => self.scene.reset(),
                }
                self.refresh_after();
            }
            ControlEvent::DemoRespawn => {
                self.boundary = self.after.clone();
                if let Some(combat) = self.combat.as_mut() {
                    combat.respawn(&mut self.scene);
                    let _ = combat.take_respawned();
                }
                self.refresh_after();
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
                self.scene.set_drive(drive);
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
                        self.report.final_observations += 2;
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

    /// One recorded tick: check, step, compare.
    fn tick(
        &mut self,
        tick: &TickRecord,
        diag: Option<&super::codec::DiagRecord>,
    ) -> Result<(), VerifyFailure> {
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
        if state_hash(&self.scene, self.combat.as_ref()) != tick.pre {
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
        self.report.routing += 1;

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
        let stage = self.combat.as_ref().map_or(0, |c| c.checkpoint as u32);
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
        self.report.labels += 1;

        // o_t, before the live decision, from the same state the hash covers.
        if let Some(combat) = self.combat.as_ref()
            && self.observe
        {
            let pair = observe_pair(&self.scene, combat, &self.party);
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
            self.report.observations += 2;
            if let Some(diag) = diag {
                let mut rebuilt = diag_of(t, &self.scene, combat, diag.rig, stats);
                rebuilt.rig = diag.rig;
                if rebuilt != *diag {
                    return Err(VerifyFailure::at(
                        "diagnostics",
                        t,
                        "the privileged diagnostics differ from the replay",
                    ));
                }
                self.report.diagnostics += 1;
            } else if !self.diag_may_be_missing {
                return Err(VerifyFailure::at(
                    "diagnostics",
                    t,
                    "diagnostics line missing",
                ));
            }
        }

        match self.combat.as_mut() {
            Some(combat) => {
                combat.step_party(&mut self.scene, hero);
                let _ = combat.take_respawned();
            }
            None => self.scene.step_party(hero),
        }

        // The effective inputs the sim applied: live log against re-derived.
        match (tick.eff, self.scene.drive().live) {
            (Some(recorded), true) => {
                if recorded != bits(self.scene.last_effective_inputs()) {
                    return Err(VerifyFailure::at(
                        "effective_input",
                        t,
                        "the live effective inputs differ from the re-derived AI inputs",
                    ));
                }
                self.report.effective_inputs += 1;
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

        if state_hash(&self.scene, self.combat.as_ref()) != tick.post {
            return Err(VerifyFailure::at(
                "post_state",
                t,
                "post-state hash differs",
            ));
        }
        self.report.state_hashes += 1;
        let replayed: Vec<EventRecord> = match self.combat.as_ref() {
            Some(combat) => {
                let dropped = combat.dropped_events as u64;
                let from = (self.cursor.saturating_sub(dropped) as usize).min(combat.events.len());
                let fresh = &combat.events[from..];
                self.events.fold(fresh);
                let out = fresh
                    .iter()
                    .enumerate()
                    .map(|(i, e)| EventRecord::from_combat(self.cursor + i as u64, e))
                    .collect();
                self.cursor = dropped + combat.events.len() as u64;
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
        self.report.event_hashes += 1;
        self.report.combat_events += replayed.len() as u64;
        if self.scene.world.tick != tick.et {
            return Err(VerifyFailure::at(
                "episode_tick",
                t,
                "the episode tick differs",
            ));
        }
        self.refresh_after();
        Ok(())
    }
}

/// Compare one recorded content hash with the one this build computes.
fn check_hash(manifest: &SessionManifest, key: &str, current: u64) -> Result<(), VerifyFailure> {
    match manifest.content_hashes.get(key) {
        Some(recorded) if *recorded == format!("{current:016x}") => Ok(()),
        Some(recorded) => Err(VerifyFailure::new(
            "content_mismatch",
            format!("{key} hash {recorded} was recorded, this build has {current:016x}"),
        )),
        None => Err(VerifyFailure::new(
            "content_mismatch",
            format!("the manifest records no {key} hash"),
        )),
    }
}

/// The checks that need no rebuilt scene: status, build and rules.
fn preflight(loaded: &LoadedSession) -> Result<(VerifyScope, &'static str), VerifyFailure> {
    let manifest = &loaded.manifest;
    let scope = match manifest.status {
        SessionStatus::Completed | SessionStatus::Abandoned => VerifyScope::Full,
        SessionStatus::CrashedRecovered => VerifyScope::Prefix,
        other => {
            return Err(VerifyFailure::new(
                "capture_incomplete",
                format!("a {other:?} session is not a complete record"),
            ));
        }
    };
    if !loaded.gaps.is_empty()
        || manifest.dropped_ticks > 0
        || manifest.dropped_control > 0
        || manifest.write_errors > 0
        || manifest
            .quality_flags
            .iter()
            .any(|f| f == "events_lost_to_cap")
    {
        return Err(VerifyFailure::new(
            "capture_incomplete",
            "ticks, control events or combat events are missing from the record",
        ));
    }

    // Build.
    if manifest.source_fingerprint.is_empty() {
        return Err(VerifyFailure::new(
            "build_mismatch",
            "the session carries no source fingerprint (recorded before PT-C): its build cannot be identified",
        ));
    }
    if manifest.source_fingerprint != SOURCE_FINGERPRINT {
        return Err(VerifyFailure::new(
            "build_mismatch",
            format!(
                "recorded by source {} ({}), this build is source {SOURCE_FINGERPRINT} ({BUILD_COMMIT})",
                manifest.source_fingerprint, manifest.toolchain
            ),
        ));
    }
    let build = if manifest.build_commit == BUILD_COMMIT {
        "identical"
    } else {
        "source_identical_commit_differs"
    };

    // Rules.
    let rules = |what: &str, found: &str, expected: &str| {
        if found == expected {
            Ok(())
        } else {
            Err(VerifyFailure::new(
                "rules_mismatch",
                format!("{what} is {found:?}, this build uses {expected:?}"),
            ))
        }
    };
    if manifest.action_schema_id != ACTION_SCHEMA && manifest.action_schema_id != ACTION_SCHEMA_V1 {
        rules("action schema", &manifest.action_schema_id, ACTION_SCHEMA)?;
    }
    rules("event schema", &manifest.event_schema_id, EVENT_SCHEMA)?;
    rules("snapshot support", &manifest.snapshot_support, "reset_only")?;
    rules("game ruleset", &manifest.game_ruleset_id, "floor01-greybox")?;
    if !manifest.observation_is_partial {
        rules(
            "observation schema",
            &manifest.observation_schema_id,
            OBS_SCHEMA_ID,
        )?;
    }
    let cf02 = manifest.feature_flags.get("cf02_rules").map(String::as_str);
    if cf02
        != Some(if manifest.reset.cf02_rules {
            "true"
        } else {
            "false"
        })
    {
        return Err(VerifyFailure::new(
            "rules_mismatch",
            "the cf02_rules flag disagrees with the reset descriptor",
        ));
    }

    Ok((scope, build))
}

/// Content: the kit and tuning hashes of the recording against this checkout.
fn check_content(
    manifest: &SessionManifest,
    root: &Path,
    tuning: &PartyTuning,
) -> Result<(), VerifyFailure> {
    let kit = kit_content_hash(root)
        .map_err(|e| VerifyFailure::new("content_mismatch", e.to_string()))?;
    if manifest.reset.kit_hash != kit {
        return Err(VerifyFailure::new(
            "content_mismatch",
            format!(
                "kit hash {:016x} was recorded, this checkout has {kit:016x}",
                manifest.reset.kit_hash
            ),
        ));
    }
    check_hash(manifest, "kit", kit)?;
    check_hash(manifest, "mana_tuning", crate::donut_mana::TUNING_HASH)?;
    check_hash(manifest, "cf02_tuning", crate::cf02::TUNING_HASH)?;
    check_hash(
        manifest,
        "party_tuning",
        crate::autoplay::party::party_tuning_hash(tuning),
    )
}

/// Verify a loaded session: preflight, then the tick-by-tick replay.
pub fn verify_loaded(loaded: &LoadedSession, root: &Path) -> Result<VerifyReport, VerifyFailure> {
    let manifest = &loaded.manifest;
    // Build and rules first: a different build is reported as such, never as
    // whatever it happens to break while rebuilding the scene.
    let (scope, build) = preflight(loaded)?;
    let reset = rebuild(manifest, root)?;
    check_content(manifest, root, &reset.tuning)?;
    let start = state_hash(&reset.scene, reset.combat.as_ref());
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

    let mut report = VerifyReport {
        session_id: manifest.session_id.clone(),
        scope,
        ticks: 0,
        episodes: 0,
        combat_events: 0,
        control_events: loaded.control.len(),
        state_hashes: 0,
        event_hashes: 0,
        effective_inputs: 0,
        observations: 0,
        final_observations: 0,
        diagnostics: 0,
        routing: 0,
        labels: 0,
        build,
        notes: vec![
            "party_hash (ph) not re-derived: PartyState history is not in the journal".to_owned(),
            "not verified: the per-slot swap bits, ph and the informational control events (solo_swap, party_join, exit, app_quit, episode structure); the control-source labels and the stage are checked per tick".to_owned(),
            "rig view rectangle not replayed: presentation only".to_owned(),
            "simulation replay only: rendering, audio and cosmetics are not reproduced".to_owned(),
        ],
    };
    if scope == VerifyScope::Prefix {
        report
            .notes
            .push("prefix only: a crash-recovered session".to_owned());
    }
    if !observe {
        report.notes.push(
            "observations not checked: the session has none (observation_is_partial)".to_owned(),
        );
    }
    let party = PartyState::new(reset.tuning);
    let mut replay = Replayer {
        scene: reset.scene,
        combat: reset.combat,
        party,
        tuning: reset.tuning,
        drive: None,
        events: EventHasher::new(),
        cursor: 0,
        after: None,
        boundary: None,
        episode: 0,
        observe,
        scripted,
        diag_may_be_missing: scope == VerifyScope::Prefix || manifest.dropped_diagnostics > 0,
        report: &mut report,
    };
    replay.refresh_after();
    let mut diags = loaded.diag.iter().peekable();
    let mut controls = loaded.control.iter().peekable();
    for tick in &loaded.ticks {
        while let Some(record) = controls.next_if(|c| c.at_t <= tick.t) {
            replay.control(record)?;
        }
        while diags.next_if(|d| d.t < tick.t).is_some() {}
        let diag = diags.next_if(|d| d.t == tick.t);
        replay.tick(tick, diag)?;
        replay.report.ticks += 1;
    }
    for record in controls {
        replay.control(record)?;
    }
    let episodes = loaded.ticks.last().map_or(0, |t| t.ep + 1);
    report.episodes = episodes;
    // A tick that the journal marks as being in a later episode than any
    // `episode_begin` was already rejected; every tick had its own check.
    Ok(report)
}
