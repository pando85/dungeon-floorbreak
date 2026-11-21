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

use crate::autoplay::replay::kit_content_hash;
use crate::cf02_fixtures::find_party_fixture;
use crate::combat_scene::{CombatState, Fixture};
use crate::corridor::SceneSim;
use crate::donut_mana::{DonutSetup, StartState};
use crate::env_kit::EnvKit;
use crate::party::PartyTuning;
use crate::party_scene::PartyFixtureSim;

use super::build_info::{BUILD_COMMIT, SOURCE_FINGERPRINT};
use super::codec::{ACTION_SCHEMA, ACTION_SCHEMA_V1, EVENT_SCHEMA, OBS_SCHEMA_ID};
use super::cursor::{ReplayCursor, ReplayPolicy, Trust};
use super::manifest::{SessionManifest, SessionStatus};
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
    pub(super) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            tick: None,
        }
    }

    pub(super) fn at(code: &'static str, tick: u64, message: impl Into<String>) -> Self {
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
pub(super) struct Reset {
    pub(super) scene: SceneSim,
    pub(super) combat: Option<CombatState>,
    pub(super) tuning: PartyTuning,
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

pub(super) fn rebuild(manifest: &SessionManifest, root: &Path) -> Result<Reset, VerifyFailure> {
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
                // ENEMY-GAMEPLAY-ENGINEER: a session of the Engineer room names its
                // fixture; every earlier session names `cf02_arena*`.
                let fixture = match reset.name.as_str() {
                    "cf02_arena_eng" => &crate::cf02::arena::LIVE_ENG,
                    "cf02_arena_eng_cp2" => &crate::cf02::arena::LIVE_ENG_CP2,
                    _ => crate::cf02::arena::fixture_for_beat(reset.demo_beat as usize),
                };
                (kit, fixture)
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

/// The checks that need no rebuilt scene: status, build and rules. Under
/// [`ReplayPolicy::CheckedCrossBuild`] the source-fingerprint predicate is the
/// one check waived (an empty fingerprint is still rejected); the returned
/// [`Trust`] says which way it went.
pub(super) fn preflight(
    loaded: &LoadedSession,
    policy: ReplayPolicy,
) -> Result<(VerifyScope, &'static str, Trust), VerifyFailure> {
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
    let (build, trust) = if manifest.source_fingerprint == SOURCE_FINGERPRINT {
        (
            if manifest.build_commit == BUILD_COMMIT {
                "identical"
            } else {
                "source_identical_commit_differs"
            },
            Trust::Strict,
        )
    } else if policy == ReplayPolicy::CheckedCrossBuild {
        // The one waived predicate: every other check below and per tick stays on.
        ("cross_build", Trust::CheckedCrossBuild)
    } else {
        return Err(VerifyFailure::new(
            "build_mismatch",
            format!(
                "recorded by source {} ({}), this build is source {SOURCE_FINGERPRINT} ({BUILD_COMMIT})",
                manifest.source_fingerprint, manifest.toolchain
            ),
        ));
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

    Ok((scope, build, trust))
}

/// Content: the kit and tuning hashes of the recording against this checkout.
pub(super) fn check_content(
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

/// Verify a loaded session: preflight, then the tick-by-tick replay. This is
/// the strict verifier: [`ReplayCursor::open`] under [`ReplayPolicy::Strict`]
/// plus a loop over `next_tick` with owned locals.
pub fn verify_loaded(loaded: &LoadedSession, root: &Path) -> Result<VerifyReport, VerifyFailure> {
    let manifest = &loaded.manifest;
    let (mut cursor, mut scene, mut combat, _tuning, _trust) =
        ReplayCursor::open(loaded, root, ReplayPolicy::Strict)?;
    while cursor
        .next_tick(loaded, &mut scene, combat.as_mut())?
        .is_some()
    {}
    let counters = cursor.counters();
    let scope = cursor.scope();
    let mut report = VerifyReport {
        session_id: manifest.session_id.clone(),
        scope,
        ticks: counters.ticks,
        // A tick that the journal marks as being in a later episode than any
        // `episode_begin` was already rejected; every tick had its own check.
        episodes: loaded.ticks.last().map_or(0, |t| t.ep + 1),
        combat_events: counters.combat_events,
        control_events: loaded.control.len(),
        state_hashes: counters.state_hashes,
        event_hashes: counters.event_hashes,
        effective_inputs: counters.effective_inputs,
        observations: counters.observations,
        final_observations: counters.final_observations,
        diagnostics: counters.diagnostics,
        routing: counters.routing,
        labels: counters.labels,
        build: cursor.build(),
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
    if manifest.observation_is_partial {
        report.notes.push(
            "observations not checked: the session has none (observation_is_partial)".to_owned(),
        );
    }
    Ok(report)
}
