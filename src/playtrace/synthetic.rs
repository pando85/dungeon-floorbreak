//! PT-D: a **synthetic** recorded session for the offline exporter's tests.
//!
//! It drives a named party fixture with the fixture's own script (no live
//! input, no human) through the real [`LiveRecorder`], so the result is a
//! session that really passes `playtrace verify` in a fresh process. Because
//! the recorded run is labelled `human` (the slot labels the exporter reads),
//! the manifest carries the quality flag [`SYNTHETIC_FLAG`] and the mode
//! `synthetic_fixture`: the exporter rejects it unless `--allow-synthetic` is
//! given and marks every dataset built from it synthetic. It is **never** a
//! human demonstration and must never be reported as one.
//!
//! The order of operations mirrors `fixed_step` in `floor01_scene` (observe,
//! swap, route, drive, begin, step, end), like the test rig in `live_tests`.
//! One restart at the middle of the run gives a second episode and an
//! `episode_end` with its `o_(t+1)`.

use std::path::Path;

use crate::cf02_fixtures::find_party_fixture;
use crate::env_kit::EnvKit;
use crate::party_scene::PartyFixtureSim;

use super::live::{self, LiveRecorder};
use super::{ResetDescriptor, SessionManifest, generate_session_id};

/// The quality flag that marks a synthetic session.
pub const SYNTHETIC_FLAG: &str = "synthetic_fixture";
/// The manifest `mode` of a synthetic session.
pub const SYNTHETIC_MODE: &str = "synthetic_fixture";
/// The default fixture: two players, so both heroes carry human-labelled actions.
pub const DEFAULT_FIXTURE: &str = "duo_independent_moves";

const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

/// Whether a manifest marks a synthetic session. Any of the three markers is
/// enough, so removing one flag does not turn a fixture into a "human" session.
pub fn is_synthetic(manifest: &SessionManifest) -> bool {
    manifest.mode == SYNTHETIC_MODE
        || manifest.quality_flags.iter().any(|f| f == SYNTHETIC_FLAG)
        || manifest.feature_flags.get("synthetic").map(String::as_str) == Some("true")
}

/// The canonical form of `path` even when parts of it do not exist yet, like
/// `realpath` that tolerates missing components. A relative path is resolved
/// against the working directory, then walked component by component: an
/// existing component is canonicalized (symlinks followed), a missing one is
/// kept as is, and `..` removes the last resolved component. So `missing/..`,
/// `.` and a relative path from inside the store all resolve to where a write
/// would actually land.
fn resolve_path(path: &Path) -> std::path::PathBuf {
    use std::path::Component;
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().map_or_else(|_| path.to_path_buf(), |cwd| cwd.join(path))
    };
    let mut resolved = std::path::PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::Prefix(_) | Component::RootDir => resolved.push(part.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            Component::Normal(name) => {
                resolved.push(name);
                if let Ok(real) = resolved.canonicalize() {
                    resolved = real;
                }
            }
        }
    }
    resolved
}

/// Refuse an output directory that is the default recording store or inside it:
/// a synthetic session must never sit among real recordings.
pub fn refuse_default_store(base: &Path, default_store: &Path) -> Result<(), String> {
    if resolve_path(base).starts_with(resolve_path(default_store)) {
        return Err(
            "synth-fixture refuses to write into the default recording store: pass --dir outside it"
                .to_owned(),
        );
    }
    Ok(())
}

/// Record `ticks` fixed steps of `fixture_name` (a non-arena party fixture)
/// into a new session under `base`, with one restart at `ticks / 2`.
pub fn record_synthetic(
    base: &Path,
    root: &Path,
    fixture_name: &str,
    ticks: u64,
) -> Result<SessionManifest, String> {
    let fixture = find_party_fixture(fixture_name)
        .ok_or_else(|| format!("unknown party fixture {fixture_name:?}"))?;
    record_synthetic_fixture(base, root, fixture, ticks)
}

/// [`record_synthetic`] for a fixture the caller holds (a test can record a
/// fixture with an edited tuning, for example the test-only legacy motion).
pub fn record_synthetic_fixture(
    base: &Path,
    root: &Path,
    fixture: &'static crate::party_scene::PartyFixture,
    ticks: u64,
) -> Result<SessionManifest, String> {
    if crate::cf02::arena::fixture_uses_arena(fixture.name()) {
        return Err("arena fixtures are not supported by the synthetic recorder".to_owned());
    }
    let kit = EnvKit::load(root, LAYOUT).map_err(|e| e.to_string())?;
    let kit_hash = crate::autoplay::replay::kit_content_hash(root).map_err(|e| e.to_string())?;
    let mut sim = PartyFixtureSim::new(&kit, 360, fixture)?;
    let reset = ResetDescriptor {
        source: "party_fixture".to_owned(),
        name: fixture.name().to_owned(),
        demo: false,
        demo_beat: 0,
        cf02_rules: false,
        cf02_seed: None,
        view_height: 360,
        donut_biscuit: false,
        carl_prog: false,
        carl_dynamite: false,
        carl_gauntlet: false,
        donut_motion: fixture.tuning.donut_motion,
        donut_prog: false,
        donut_light_feet: false,
        donut_puddle: false,
        kit_hash,
        start_hash: sim.combat.state_hash(&sim.scene),
    };
    let mut manifest = SessionManifest::new(generate_session_id(), reset);
    live::stamp_manifest(&mut manifest, kit_hash);
    manifest.mode = SYNTHETIC_MODE.to_owned();
    manifest.quality_flags.push(SYNTHETIC_FLAG.to_owned());
    manifest
        .feature_flags
        .insert("synthetic".to_owned(), "true".to_owned());
    let mut rec = LiveRecorder::start(
        base,
        manifest,
        &sim.scene,
        Some(&sim.combat),
        &sim.party,
        false,
    )
    .map_err(|e| e.to_string())?;
    for step in 0..ticks {
        if ticks > 1 && step == ticks / 2 {
            sim.combat.restart(&mut sim.scene);
            sim.party.restart();
            rec.note_restart(&sim.scene, Some(&sim.combat), &sim.party);
        }
        rec.observe_party(&sim.party);
        if sim.party.is_paused() {
            continue;
        }
        let slots = fixture.inputs(sim.scene.world.tick + 1, &sim.combat, &sim.scene);
        let hurt = sim.combat.hurt_ticks_of(sim.party.controlled_hero()) > 0;
        sim.party.request_swap(slots[0].swap, hurt);
        let hero = sim.party.hero_inputs(slots);
        let drive = sim.party.drive(true);
        sim.scene.set_drive(drive);
        rec.begin_tick(
            &sim.scene,
            Some(&sim.combat),
            &sim.party,
            slots,
            hero,
            drive,
        );
        let restarts = sim.combat.restarts;
        sim.combat.step_party(&mut sim.scene, hero);
        let respawned = sim.combat.take_respawned();
        if sim.combat.restarts != restarts {
            sim.party.restart();
        }
        rec.end_tick(&sim.scene, Some(&sim.combat), &sim.party, respawned);
    }
    rec.finish("synthetic_fixture")
        .ok_or_else(|| "the synthetic session could not be finished".to_owned())
}
