//! PT-B proofs: the live adapter records every tick once, is observer-only
//! (recording on and off give identical simulation hashes), and what it wrote
//! replays to the same hashes in a fresh process.
//!
//! The `Rig` mirrors the order of operations of `fixed_step` in
//! `floor01_scene` (observe, swap, route, drive, begin, step, end).

use std::path::{Path, PathBuf};

use super::fake::FakeParty;
use super::live::{self, LiveRecorder};
use super::*;
use crate::autoplay::metrics::EventHasher;
use crate::cf02_fixtures::find_party_fixture;
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::env_kit::EnvKit;
use crate::input::{RawInput, SlotInputs};
use crate::party::{HeroId, PadId, PartyDrive, PartyState};
use crate::party_scene::{PARTY_FIXTURES, PartyFixture, PartyFixtureSim, party_hash};

const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

pub(super) fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub(super) fn kit() -> EnvKit {
    EnvKit::load(&root(), LAYOUT).expect("corridor kit")
}

pub(super) struct TempDir(pub(super) PathBuf);

impl TempDir {
    pub(super) fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("playtrace-live-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn manifest_for(fixture: &PartyFixture, scene: &SceneSim, combat: &CombatState) -> SessionManifest {
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
        donut_motion: crate::party::PROVISIONAL_TUNING.donut_motion,
        donut_prog: false,
        donut_light_feet: false,
        donut_puddle: false,
        f01_room: None,
        f01_loadout: None,
        f01_roster: None,
        kit_hash: real_kit_hash(),
        start_hash: combat.state_hash(scene),
    };
    stamped(reset)
}

pub(super) fn real_kit_hash() -> u64 {
    crate::autoplay::replay::kit_content_hash(&root()).expect("kit hash")
}

/// A manifest stamped like the binary's: build identity, ruleset and content
/// hashes of this checkout.
pub(super) fn stamped(reset: ResetDescriptor) -> SessionManifest {
    let mut manifest = SessionManifest::new(generate_session_id(), reset);
    live::stamp_manifest(&mut manifest, real_kit_hash());
    manifest
}

/// A live-path stand-in: the same dispatch as `fixed_step`.
pub(super) struct Rig {
    pub(super) sim: PartyFixtureSim,
    fixture: &'static PartyFixture,
    pub(super) rec: Option<LiveRecorder>,
    /// The per-tick state hash after each step (to compare runs).
    hashes: Vec<u64>,
    steps: u64,
}

impl Rig {
    pub(super) fn new(fixture: &'static PartyFixture, base: Option<&Path>) -> Self {
        let sim = PartyFixtureSim::new(&kit(), 360, fixture).unwrap();
        let rec = base.map(|base| {
            let manifest = manifest_for(fixture, &sim.scene, &sim.combat);
            LiveRecorder::start(
                base,
                manifest,
                &sim.scene,
                Some(&sim.combat),
                &sim.party,
                false,
            )
            .unwrap()
        });
        Self {
            sim,
            fixture,
            rec,
            hashes: Vec::new(),
            steps: 0,
        }
    }

    pub(super) fn step(&mut self, slots: SlotInputs) {
        let sim = &mut self.sim;
        if let Some(rec) = self.rec.as_mut() {
            rec.observe_party(&sim.party);
        }
        if sim.party.is_paused() {
            return;
        }
        let hurt = sim.combat.hurt_ticks_of(sim.party.controlled_hero()) > 0;
        sim.party.request_swap(slots[0].swap, hurt);
        let hero = sim.party.hero_inputs(slots);
        let drive = sim.party.drive(true);
        sim.scene.set_drive(drive);
        if let Some(rec) = self.rec.as_mut() {
            rec.begin_tick(
                &sim.scene,
                Some(&sim.combat),
                &sim.party,
                slots,
                hero,
                drive,
            );
        }
        let restarts = sim.combat.restarts;
        sim.combat.step_party(&mut sim.scene, hero);
        let respawned = sim.combat.take_respawned();
        if sim.combat.restarts != restarts {
            sim.party.restart();
        }
        if let Some(rec) = self.rec.as_mut() {
            rec.end_tick(&sim.scene, Some(&sim.combat), &sim.party, respawned);
        }
        self.steps += 1;
        self.hashes
            .push(party_hash(&sim.combat, &sim.scene, &sim.party));
    }

    /// The fixture's own script (the golden path).
    pub(super) fn step_scripted(&mut self) {
        let slots = self.fixture.inputs(
            self.sim.scene.world.tick + 1,
            &self.sim.combat,
            &self.sim.scene,
        );
        self.step(slots);
    }

    /// A manual restart (R / Select held): the same calls as `fixed_step`.
    pub(super) fn restart(&mut self) {
        let sim = &mut self.sim;
        sim.combat.restart(&mut sim.scene);
        sim.party.restart();
        if let Some(rec) = self.rec.as_mut() {
            rec.note_restart(&sim.scene, Some(&sim.combat), &sim.party);
        }
    }

    pub(super) fn finish(&mut self) -> Option<SessionManifest> {
        self.rec.as_mut().and_then(|rec| rec.finish("test"))
    }
}

/// Replay a recorded session from its reset descriptor and the input tape:
/// the per-hero routed inputs, the drive and the lifecycle events. Compares
/// every pre and post hash, the rolling event hash and every event. (The
/// shipped verifier is PT-C; this is the proof that the journal is enough.)
fn replay_session(dir: &Path) -> Result<u64, String> {
    let loaded = read_session(dir, false).map_err(|e| e.to_string())?;
    let reset = &loaded.manifest.reset;
    let (mut scene, mut combat, tuning) = match reset.source.as_str() {
        "party_fixture" => {
            let fixture = find_party_fixture(&reset.name).ok_or("unknown fixture")?;
            let sim = PartyFixtureSim::new(&kit(), reset.view_height, fixture)
                .map_err(|e| e.to_string())?;
            (sim.scene, sim.combat, fixture.tuning)
        }
        "demo" => {
            let (scene, combat, party) = demo_scene(reset.demo_beat as usize);
            (scene, combat, *party.tuning())
        }
        other => return Err(format!("unsupported source {other}")),
    };
    if combat.state_hash(&scene) != reset.start_hash {
        return Err("the fresh reset does not match the recorded start hash".to_owned());
    }
    let mut events = EventHasher::new();
    let mut cursor = 0_u64;
    let mut controls = loaded.control.iter().peekable();
    for tick in &loaded.ticks {
        while let Some(record) = controls.next_if(|c| c.at_t <= tick.t) {
            match &record.ev {
                ControlEvent::Restart => combat.restart(&mut scene),
                // A Select-tap respawn happens outside the step: apply it.
                // The defeat respawn happens inside the step and is not a
                // control event, so it is never applied twice.
                ControlEvent::DemoRespawn => {
                    combat.respawn(&mut scene);
                    let _ = combat.take_respawned();
                }
                ControlEvent::DriveChange {
                    live,
                    two_player,
                    controlled,
                    bots,
                } => {
                    let hero = if *controlled == 0 {
                        HeroId::Carl
                    } else {
                        HeroId::Donut
                    };
                    scene.set_drive(PartyDrive {
                        live: *live,
                        two_player: *two_player,
                        controlled: hero,
                        bots: *bots,
                        tuning,
                    });
                }
                _ => {}
            }
        }
        if combat.state_hash(&scene) != tick.pre {
            return Err(format!("pre-state hash differs at t={}", tick.t));
        }
        let hero = [tick.hero[0].to_raw(), tick.hero[1].to_raw()];
        combat.step_party(&mut scene, hero);
        let _ = combat.take_respawned();
        if combat.state_hash(&scene) != tick.post {
            return Err(format!("post-state hash differs at t={}", tick.t));
        }
        let dropped = combat.dropped_events as u64;
        let fresh = &combat.events[(cursor - dropped) as usize..];
        events.fold(fresh);
        let replayed: Vec<EventRecord> = fresh
            .iter()
            .enumerate()
            .map(|(i, e)| EventRecord::from_combat(cursor + i as u64, e))
            .collect();
        cursor = dropped + combat.events.len() as u64;
        if replayed != tick.ev || events.value() != tick.eh {
            return Err(format!("events differ at t={}", tick.t));
        }
        if scene.world.tick != tick.et {
            return Err(format!("episode tick differs at t={}", tick.t));
        }
    }
    Ok(loaded.ticks.len() as u64)
}

/// The scripted live session: solo play with swaps, a manual restart, P2
/// joining, a pad disconnect (D6 pause) and its reconnect.
pub(super) fn live_script(rig: &mut Rig, ticks: u64) {
    let mut input = FakeParty::new(77, 0);
    rig.sim.party.pad_connected(PadId(1));
    for tick in 0..ticks {
        match tick {
            300 => rig.restart(),
            500 => {
                rig.sim.party.pad_connected(PadId(2));
                rig.sim.party.pad_start_pressed(PadId(2));
            }
            700 => {
                rig.sim.party.pad_disconnected(PadId(2));
                // The sim waits (fixed_step returns early) for a while.
                for _ in 0..30 {
                    rig.step([RawInput::NONE; 2]);
                }
                rig.sim.party.pad_connected(PadId(2));
            }
            _ => {}
        }
        let slots = input.next_slots();
        rig.step(slots);
    }
}

pub(super) fn demo_scene(beat: usize) -> (SceneSim, CombatState, PartyState) {
    let kit = crate::demo::load_hall(&root()).expect("demo hall");
    let fixture = crate::demo::fixture_for_beat(beat);
    let scene = SceneSim::new_at(&kit, 360, fixture.carl_x).unwrap();
    let combat = CombatState::new(fixture, &kit, &scene)
        .unwrap()
        .with_donut(crate::donut_mana::DonutSetup::NONE)
        .with_demo(&crate::demo::DEMO_RULES);
    (scene, combat, PartyState::default())
}

/// The `--demo` rules path: the same dispatch as `fixed_step`, with Select-tap
/// respawns (before the step) and the demo's own defeat respawn (inside it).
pub(super) struct DemoRig {
    scene: SceneSim,
    combat: CombatState,
    party: PartyState,
    pub(super) rec: LiveRecorder,
    hashes: Vec<u64>,
}

impl DemoRig {
    pub(super) fn new(base: &Path, scripted: bool, beat: usize) -> Self {
        Self::with_party(base, scripted, beat, PartyState::default())
    }

    /// The same rig with a chosen party (for example a companion tuning).
    pub(super) fn with_party(base: &Path, scripted: bool, beat: usize, party: PartyState) -> Self {
        let (scene, combat, _) = demo_scene(beat);
        let reset = ResetDescriptor {
            source: "demo".to_owned(),
            name: "demo01".to_owned(),
            demo: true,
            demo_beat: beat as u32,
            cf02_rules: false,
            cf02_seed: None,
            view_height: 360,
            donut_biscuit: false,
            carl_prog: false,
            carl_dynamite: false,
            carl_gauntlet: false,
            donut_motion: crate::party::PROVISIONAL_TUNING.donut_motion,
            donut_prog: false,
            donut_light_feet: false,
            donut_puddle: false,
            f01_room: None,
            f01_loadout: None,
            f01_roster: None,
            kit_hash: real_kit_hash(),
            start_hash: combat.state_hash(&scene),
        };
        let manifest = stamped(reset);
        let rec =
            LiveRecorder::start(base, manifest, &scene, Some(&combat), &party, scripted).unwrap();
        Self {
            scene,
            combat,
            party,
            rec,
            hashes: Vec::new(),
        }
    }

    /// CF02-F: a rig for `--cf02` (the microarena, `arena::RULES`, the CF02 rules,
    /// the party the caller built): the descriptor the binary writes for the flag.
    pub(super) fn cf02(base: &Path, party: PartyState) -> Self {
        let kit = crate::cf02::arena::load_arena(&root()).expect("arena");
        let fixture = crate::cf02::arena::fixture_for_beat(0);
        let scene = SceneSim::new_at(&kit, 360, fixture.carl_x).unwrap();
        let combat = CombatState::new(fixture, &kit, &scene)
            .unwrap()
            .with_donut(crate::donut_mana::DonutSetup::NONE)
            .with_demo(&crate::cf02::arena::RULES)
            .with_cf02()
            .with_cf02_seed(None);
        let reset = ResetDescriptor {
            source: "demo".to_owned(),
            name: fixture.name.to_owned(),
            demo: true,
            demo_beat: 0,
            cf02_rules: true,
            cf02_seed: None,
            view_height: 360,
            donut_biscuit: false,
            carl_prog: false,
            carl_dynamite: false,
            carl_gauntlet: false,
            donut_motion: crate::party::PROVISIONAL_TUNING.donut_motion,
            donut_prog: false,
            donut_light_feet: false,
            donut_puddle: false,
            f01_room: None,
            f01_loadout: None,
            f01_roster: None,
            kit_hash: real_kit_hash(),
            start_hash: combat.state_hash(&scene),
        };
        let manifest = stamped(reset);
        let rec =
            LiveRecorder::start(base, manifest, &scene, Some(&combat), &party, false).unwrap();
        Self {
            scene,
            combat,
            party,
            rec,
            hashes: Vec::new(),
        }
    }

    /// A manual restart (R or Select held) of the demo path.
    pub(super) fn restart(&mut self) {
        self.combat.restart(&mut self.scene);
        self.party.restart();
        self.rec
            .note_restart(&self.scene, Some(&self.combat), &self.party);
    }

    pub(super) fn select_tap_respawn(&mut self) {
        self.combat.respawn(&mut self.scene);
        let _ = self.combat.take_respawned();
        self.party.restart();
        self.rec
            .note_demo_respawn(&self.scene, Some(&self.combat), &self.party);
    }

    pub(super) fn step(&mut self, carl: RawInput) {
        let slots = [carl, RawInput::NONE];
        self.rec.observe_party(&self.party);
        let hurt = self.combat.hurt_ticks_of(self.party.controlled_hero()) > 0;
        self.party.request_swap(slots[0].swap, hurt);
        let hero = self.party.hero_inputs(slots);
        let drive = self.party.drive(true);
        self.scene.set_drive(drive);
        self.rec.begin_tick(
            &self.scene,
            Some(&self.combat),
            &self.party,
            slots,
            hero,
            drive,
        );
        let restarts = self.combat.restarts;
        self.combat.step_party(&mut self.scene, hero);
        let respawned = self.combat.take_respawned();
        if self.combat.restarts != restarts || respawned {
            self.party.restart();
        }
        self.rec
            .end_tick(&self.scene, Some(&self.combat), &self.party, respawned);
        self.hashes.push(self.combat.state_hash(&self.scene));
    }
}

pub(super) fn solo_fixture() -> &'static PartyFixture {
    find_party_fixture("solo_swap_continuity").expect("fixture")
}

#[test]
fn every_tick_is_recorded_exactly_once_with_ordered_control_events() {
    let dir = TempDir::new("once");
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    live_script(&mut rig, 1500);
    let manifest = rig.finish().unwrap();
    assert_eq!(manifest.status, SessionStatus::Completed);
    let session = dir.0.join(&manifest.session_id);
    let loaded = read_session(&session, false).unwrap();
    // 1500 stepped ticks (the 30 paused calls return before the step).
    assert_eq!(rig.steps, 1500);
    assert_eq!(loaded.ticks.len() as u64, rig.steps);
    for (index, tick) in loaded.ticks.iter().enumerate() {
        assert_eq!(tick.t, index as u64);
    }
    let kinds: Vec<&ControlEvent> = loaded.control.iter().map(|c| &c.ev).collect();
    let has = |wanted: &ControlEvent| kinds.contains(&wanted);
    assert!(has(&ControlEvent::Restart));
    assert!(has(&ControlEvent::PartyJoin));
    assert!(has(&ControlEvent::PadVacated { slot: 1 }));
    assert!(has(&ControlEvent::D6PauseBegin));
    assert!(has(&ControlEvent::PadReclaimed { slot: 1 }));
    assert!(has(&ControlEvent::D6PauseEnd));
    assert!(
        kinds
            .iter()
            .any(|k| matches!(k, ControlEvent::SoloSwap { .. }))
    );
    assert!(
        kinds
            .iter()
            .any(|k| matches!(k, ControlEvent::DriveChange { .. }))
    );
    assert!(kinds.iter().any(|k| matches!(k, ControlEvent::AppQuit)));
    // The pause begins before it ends, both at the same tick boundary (no
    // simulated tick between them was recorded).
    let begin = loaded
        .control
        .iter()
        .find(|c| c.ev == ControlEvent::D6PauseBegin)
        .unwrap();
    let end = loaded
        .control
        .iter()
        .find(|c| c.ev == ControlEvent::D6PauseEnd)
        .unwrap();
    assert!(begin.seq < end.seq && begin.at_t == end.at_t);
    // Episodes: start and the manual restart (more after any party defeat).
    assert!(loaded.ticks.last().unwrap().ep >= 1);
    let restart_tick = loaded.ticks.iter().position(|t| t.ep == 1).unwrap();
    assert_eq!(restart_tick, 300);
    // Solo: the second hero is a bot, then P2 joins and both are human.
    assert!(
        loaded
            .ticks
            .iter()
            .any(|t| t.src[1] == ControlSource::ScriptedBot)
    );
    assert!(
        loaded
            .ticks
            .iter()
            .any(|t| t.src[1] == ControlSource::HumanP2)
    );
    // Per-slot provenance is kept next to the routed driver.
    let swap_ticks = loaded
        .ticks
        .iter()
        .filter(|t| t.slots[0].1 & 32 != 0)
        .count();
    assert!(swap_ticks > 0);
    assert!(
        loaded
            .ticks
            .iter()
            .all(|t| t.hero[0].1 & 32 == 0 && t.hero[1].1 & 32 == 0)
    );
}

#[test]
fn recording_on_and_off_give_identical_simulation_hashes() {
    let dir = TempDir::new("onoff");
    let mut on = Rig::new(solo_fixture(), Some(&dir.0));
    let mut off = Rig::new(solo_fixture(), None);
    live_script(&mut on, 1500);
    live_script(&mut off, 1500);
    assert_eq!(on.hashes, off.hashes);
    assert_eq!(on.sim.combat.events, off.sim.combat.events);
    assert_eq!(on.sim.combat.dropped_events, off.sim.combat.dropped_events);
    // The recorder never touched the log cursor the binary owns.
    assert_eq!(
        on.sim.combat.take_new_events().len(),
        off.sim.combat.take_new_events().len()
    );
}

#[test]
fn every_party_fixture_matches_its_golden_path_with_recording_on_and_replays() {
    for fixture in PARTY_FIXTURES {
        let dir = TempDir::new(&format!("golden-{}", fixture.name()));
        let ticks = 400;
        let mut baseline = PartyFixtureSim::new(&kit(), 360, fixture).unwrap();
        baseline.run_scripted(ticks);
        let mut rig = Rig::new(fixture, Some(&dir.0));
        for _ in 0..ticks {
            rig.step_scripted();
        }
        assert_eq!(
            party_hash(&rig.sim.combat, &rig.sim.scene, &rig.sim.party),
            baseline.state_hash(),
            "{}: the recorded dispatch changed the outcome",
            fixture.name()
        );
        let manifest = rig.finish().unwrap();
        let session = dir.0.join(&manifest.session_id);
        assert_eq!(replay_session(&session), Ok(ticks), "{}", fixture.name());
    }
}

#[test]
fn the_live_session_replays_to_the_same_hashes_in_a_fresh_process() {
    let dir = TempDir::new("fresh");
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    live_script(&mut rig, 1500);
    let manifest = rig.finish().unwrap();
    let session = dir.0.join(&manifest.session_id);
    // Same process first (cheap), then a brand-new process.
    assert_eq!(replay_session(&session), Ok(1500));
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "playtrace::live_tests::replay_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("PLAYTRACE_REPLAY_DIR", &session)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "child failed: {stdout}");
    assert!(
        stdout.contains("PLAYTRACE_REPLAY_OK ticks=1500"),
        "{stdout}"
    );
}

/// The fresh-process half of the test above: a no-op unless the parent sets
/// the directory.
#[test]
fn replay_child() {
    let Ok(dir) = std::env::var("PLAYTRACE_REPLAY_DIR") else {
        return;
    };
    match replay_session(Path::new(&dir)) {
        Ok(ticks) => println!("PLAYTRACE_REPLAY_OK ticks={ticks}"),
        Err(why) => panic!("replay mismatch: {why}"),
    }
}

#[test]
fn a_tampered_input_fails_the_replay() {
    let dir = TempDir::new("tamper");
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    live_script(&mut rig, 400);
    let manifest = rig.finish().unwrap();
    let session = dir.0.join(&manifest.session_id);
    assert!(replay_session(&session).is_ok());
    // Flip one recorded jump bit and re-seal the chunk so only the replay can
    // notice: tick 5 is moving, so a different input changes the state.
    let path = session.join("ticks-000000.jsonl");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let mut tick: TickRecord = serde_json::from_str(&lines[5]).unwrap();
    tick.hero[0].1 ^= 1;
    lines[5] = serde_json::to_string(&tick).unwrap();
    let trailer_at = lines
        .iter()
        .position(|l| l.starts_with("{\"end\""))
        .unwrap();
    let mut body = String::new();
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for line in &lines[..trailer_at] {
        body.push_str(line);
        body.push('\n');
    }
    for byte in body.as_bytes() {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    let trailer = ChunkTrailer {
        end: true,
        chunk: 0,
        ticks: trailer_at,
        first_t: Some(0),
        last_t: Some(trailer_at as u64 - 1),
        fnv: hash,
    };
    body.push_str(&serde_json::to_string(&trailer).unwrap());
    body.push('\n');
    std::fs::write(&path, body).unwrap();
    let mut manifest_json: SessionManifest =
        serde_json::from_str(&std::fs::read_to_string(session.join("manifest.json")).unwrap())
            .unwrap();
    manifest_json.chunk_checksums[0].fnv = hash;
    std::fs::write(
        session.join("manifest.json"),
        serde_json::to_string(&manifest_json).unwrap(),
    )
    .unwrap();
    assert!(replay_session(&session).is_err());
}

#[test]
fn events_use_an_absolute_cursor_across_the_log_cap() {
    let dir = TempDir::new("cursor");
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    let mut input = FakeParty::new(5, 0);
    // A restart every other tick logs one event each: well past the 512 cap.
    for tick in 0..1500_u64 {
        if tick % 2 == 0 {
            rig.restart();
        }
        let slots = input.next_slots();
        rig.step(slots);
    }
    let combat = &rig.sim.combat;
    assert!(combat.dropped_events > 0, "the fixture must exceed the cap");
    let total = (combat.dropped_events + combat.events.len()) as u64;
    let manifest = rig.finish().unwrap();
    let loaded = read_session(&dir.0.join(&manifest.session_id), false).unwrap();
    let ids: Vec<u64> = loaded
        .ticks
        .iter()
        .flat_map(|t| t.ev.iter().map(|e| e.i))
        .collect();
    assert_eq!(
        ids,
        (0..total).collect::<Vec<u64>>(),
        "every event once, in order"
    );
    assert!(
        !manifest
            .quality_flags
            .contains(&"events_lost_to_cap".to_owned())
    );
    // The recorder's cursor is its own: the binary's `take_new_events` still
    // sees everything the cap kept.
    assert_eq!(
        rig.sim.combat.take_new_events().len(),
        rig.sim.combat.events.len()
    );
}

#[test]
fn events_lost_to_the_cap_are_reported_not_hidden() {
    let dir = TempDir::new("lost");
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    let slots = FakeParty::new(1, 0).next_slots();
    rig.step(slots);
    // 600 restarts between two ticks: the cap drops the oldest 88 unread.
    for _ in 0..600 {
        let sim = &mut rig.sim;
        sim.combat.restart(&mut sim.scene);
    }
    rig.step(slots);
    let messages = rig.rec.as_mut().unwrap().drain_messages().join("\n");
    assert!(
        messages.contains("PLAYTRACE_ERROR") && messages.contains("lost to the log cap"),
        "{messages}"
    );
    let manifest = rig.finish().unwrap();
    assert!(
        manifest
            .quality_flags
            .contains(&"events_lost_to_cap".to_owned())
    );
    let loaded = read_session(&dir.0.join(&manifest.session_id), false).unwrap();
    let ids: Vec<u64> = loaded
        .ticks
        .iter()
        .flat_map(|t| t.ev.iter().map(|e| e.i))
        .collect();
    // The oldest unread events are gone from the log: fewer ids than events
    // were logged, and the surviving ones are still ordered.
    assert!(!ids.is_empty() && ids.len() < 600 + 2);
    assert!(ids.windows(2).all(|w| w[1] > w[0]));
}

#[test]
fn one_record_per_logical_step_whatever_the_frame_rate() {
    let dir = TempDir::new("frames");
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    // Render frames are irrelevant: the recorder is only called from the fixed
    // step. A hook pair without a step records nothing, a second `begin_tick`
    // (a duplicated call) replaces the pending one instead of doubling it.
    let mut input = FakeParty::new(3, 0);
    for _ in 0..50 {
        let slots = input.next_slots();
        rig.step(slots);
    }
    {
        let sim = &rig.sim;
        let rec = rig.rec.as_mut().unwrap();
        rec.end_tick(&sim.scene, Some(&sim.combat), &sim.party, false);
        assert_eq!(rec.ticks(), 50);
        let hero = sim.party.hero_inputs([RawInput::NONE; 2]);
        let drive = sim.party.drive(true);
        rec.begin_tick(
            &sim.scene,
            Some(&sim.combat),
            &sim.party,
            [RawInput::NONE; 2],
            hero,
            drive,
        );
        rec.begin_tick(
            &sim.scene,
            Some(&sim.combat),
            &sim.party,
            [RawInput::NONE; 2],
            hero,
            drive,
        );
        rec.end_tick(&sim.scene, Some(&sim.combat), &sim.party, false);
        rec.end_tick(&sim.scene, Some(&sim.combat), &sim.party, false);
        assert_eq!(rec.ticks(), 51);
    }
}

#[test]
fn an_unwritable_directory_does_not_stop_the_game_and_is_visible() {
    let dir = TempDir::new("blocked");
    let blocker = dir.0.join("file");
    std::fs::write(&blocker, b"x").unwrap();
    let sim = PartyFixtureSim::new(&kit(), 360, solo_fixture()).unwrap();
    let manifest = manifest_for(solo_fixture(), &sim.scene, &sim.combat);
    assert!(
        LiveRecorder::start(
            &blocker,
            manifest,
            &sim.scene,
            Some(&sim.combat),
            &sim.party,
            false
        )
        .is_err()
    );
}

/// Idle Carl at demo beat 4: he stands still until the goblins defeat him and the demo respawns him by
/// itself; a Select tap respawns him once by hand in between.
pub(super) fn demo_script(rig: &mut DemoRig, ticks: u64) {
    for tick in 0..ticks {
        if tick == 900 {
            rig.select_tap_respawn();
        }
        rig.step(RawInput::NONE);
    }
}

#[test]
fn demo_rules_replay_covers_defeat_respawn_and_select_tap_respawn() {
    let dir = TempDir::new("demo");
    let mut rig = DemoRig::new(&dir.0, false, 4);
    demo_script(&mut rig, 2400);
    let manifest = rig.rec.finish("test").unwrap();
    let session = dir.0.join(&manifest.session_id);
    let loaded = read_session(&session, false).unwrap();
    let begins: Vec<EpisodeReason> = loaded
        .control
        .iter()
        .filter_map(|c| match c.ev {
            ControlEvent::EpisodeBegin { reason, .. } => Some(reason),
            _ => None,
        })
        .collect();
    // Both paths happened and are distinguishable in the journal.
    assert!(
        begins.contains(&EpisodeReason::DemoRespawn),
        "select tap: {begins:?}"
    );
    assert!(
        begins.contains(&EpisodeReason::DemoDefeatRespawn),
        "the idle hero must be defeated and respawned by the demo: {begins:?}"
    );
    // Only the Select tap is a replay-input control event.
    let respawn_events = loaded
        .control
        .iter()
        .filter(|c| c.ev == ControlEvent::DemoRespawn)
        .count();
    assert_eq!(respawn_events, 1);
    assert_eq!(replay_session(&session), Ok(2400));
}

#[test]
fn a_forced_record_of_a_scripted_run_never_labels_inputs_human() {
    let dir = TempDir::new("scripted");
    let mut scripted = DemoRig::new(&dir.0, true, 0);
    let mut human = DemoRig::new(&dir.0, false, 0);
    for _ in 0..20 {
        scripted.step(RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        });
        human.step(RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        });
    }
    let s = scripted.rec.finish("test").unwrap();
    let h = human.rec.finish("test").unwrap();
    let scripted_ticks = read_session(&dir.0.join(&s.session_id), false)
        .unwrap()
        .ticks;
    let human_ticks = read_session(&dir.0.join(&h.session_id), false)
        .unwrap()
        .ticks;
    assert!(scripted_ticks.iter().all(|t| {
        t.src
            .iter()
            .all(|s| !matches!(s, ControlSource::HumanP1 | ControlSource::HumanP2))
    }));
    assert!(
        scripted_ticks
            .iter()
            .all(|t| t.src[0] == ControlSource::ScriptedBot)
    );
    assert!(
        human_ticks
            .iter()
            .all(|t| t.src[0] == ControlSource::HumanP1)
    );
}

/// PT-C follow-up 6: a second finish (the lab relaunch finishes the session and
/// the app exit finishes it again) does nothing and says nothing.
#[test]
fn finish_is_idempotent_and_the_second_call_is_silent() {
    let dir = TempDir::new("finish-twice");
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    live_script(&mut rig, 120);
    let rec = rig.rec.as_mut().unwrap();
    let first = rec.finish("lab_relaunch").expect("the first call finishes");
    assert_eq!(first.status, SessionStatus::Completed);
    let first_lines = rec.drain_messages();
    assert!(
        first_lines.iter().any(|l| l.starts_with("PLAYTRACE_SAVED")),
        "{first_lines:?}"
    );
    assert!(rec.finish("app_quit").is_none());
    assert!(rec.finish("app_quit").is_none());
    assert!(
        rec.drain_messages().is_empty(),
        "no PLAYTRACE_ERROR from a second exit"
    );
    // The session on disk is the first call's: one exit, one app_quit.
    let loaded = read_session(&dir.0.join(&first.session_id), false).unwrap();
    let exits = loaded
        .control
        .iter()
        .filter(|c| matches!(c.ev, ControlEvent::Exit { .. }))
        .count();
    assert_eq!(exits, 1);
    assert_eq!(loaded.manifest.status, SessionStatus::Completed);
}

/// A rotation manifest failure inside a live session is warned about once, and
/// the session still verifies.
#[test]
fn a_failed_rotation_manifest_is_reported_by_the_live_recorder() {
    let dir = TempDir::new("rot-warn");
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    // 650 ticks cross one chunk boundary; the writer is private to the
    // recorder, so make the manifest path unwritable instead: a directory
    // squatting on `manifest.json.tmp`.
    let session = std::fs::read_dir(&dir.0)
        .unwrap()
        .flatten()
        .next()
        .unwrap()
        .path();
    std::fs::create_dir(session.join("manifest.json.tmp")).unwrap();
    live_script(&mut rig, 650);
    let rec = rig.rec.as_mut().unwrap();
    assert_eq!(rec.health().manifest_write_errors, 1);
    assert!(!rec.health().incomplete());
    let lines = rec.drain_messages();
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.starts_with("PLAYTRACE_WARN"))
            .count(),
        1,
        "{lines:?}"
    );
    std::fs::remove_dir(session.join("manifest.json.tmp")).unwrap();
    let manifest = rig.finish().unwrap();
    assert_eq!(manifest.status, SessionStatus::Completed);
    assert_eq!(manifest.manifest_write_errors, 1);
}
