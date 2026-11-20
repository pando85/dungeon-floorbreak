//! PT-B proofs: the live adapter records every tick once, is observer-only
//! (recording on and off give identical simulation hashes), and what it wrote
//! replays to the same hashes in a fresh process.
//!
//! The `Rig` mirrors the order of operations of `fixed_step` in
//! `floor01_scene` (observe, swap, route, drive, begin, step, end).

use std::path::{Path, PathBuf};

use super::fake::FakeParty;
use super::live::LiveRecorder;
use super::*;
use crate::autoplay::metrics::EventHasher;
use crate::cf02_fixtures::find_party_fixture;
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::env_kit::EnvKit;
use crate::input::{RawInput, SlotInputs};
use crate::party::{HeroId, PadId, PartyDrive};
use crate::party_scene::{PARTY_FIXTURES, PartyFixture, PartyFixtureSim, party_hash};

const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn kit() -> EnvKit {
    EnvKit::load(&root(), LAYOUT).expect("corridor kit")
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
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
        kit_hash: 0,
        start_hash: combat.state_hash(scene),
    };
    SessionManifest::new(generate_session_id(), reset)
}

/// A live-path stand-in: the same dispatch as `fixed_step`.
struct Rig {
    sim: PartyFixtureSim,
    fixture: &'static PartyFixture,
    rec: Option<LiveRecorder>,
    /// The per-tick state hash after each step (to compare runs).
    hashes: Vec<u64>,
    steps: u64,
}

impl Rig {
    fn new(fixture: &'static PartyFixture, base: Option<&Path>) -> Self {
        let sim = PartyFixtureSim::new(&kit(), 360, fixture).unwrap();
        let rec = base.map(|base| {
            let manifest = manifest_for(fixture, &sim.scene, &sim.combat);
            LiveRecorder::start(base, manifest, Some(&sim.combat), &sim.party).unwrap()
        });
        Self {
            sim,
            fixture,
            rec,
            hashes: Vec::new(),
            steps: 0,
        }
    }

    fn step(&mut self, slots: SlotInputs) {
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
    fn step_scripted(&mut self) {
        let slots = self.fixture.inputs(
            self.sim.scene.world.tick + 1,
            &self.sim.combat,
            &self.sim.scene,
        );
        self.step(slots);
    }

    /// A manual restart (R / Select held): the same calls as `fixed_step`.
    fn restart(&mut self) {
        let sim = &mut self.sim;
        sim.combat.restart(&mut sim.scene);
        sim.party.restart();
        if let Some(rec) = self.rec.as_mut() {
            rec.note_restart(Some(&sim.combat), &sim.party);
        }
    }

    fn finish(&mut self) -> Option<SessionManifest> {
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
    assert_eq!(reset.source, "party_fixture");
    let fixture = find_party_fixture(&reset.name).ok_or("unknown fixture")?;
    let mut sim =
        PartyFixtureSim::new(&kit(), reset.view_height, fixture).map_err(|e| e.to_string())?;
    if sim.combat.state_hash(&sim.scene) != reset.start_hash {
        return Err("the fresh reset does not match the recorded start hash".to_owned());
    }
    let mut events = EventHasher::new();
    let mut cursor = 0_u64;
    let mut controls = loaded.control.iter().peekable();
    for tick in &loaded.ticks {
        while let Some(record) = controls.next_if(|c| c.at_t <= tick.t) {
            match &record.ev {
                ControlEvent::Restart => sim.combat.restart(&mut sim.scene),
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
                    sim.scene.set_drive(PartyDrive {
                        live: *live,
                        two_player: *two_player,
                        controlled: hero,
                        bots: *bots,
                        tuning: fixture.tuning,
                    });
                }
                _ => {}
            }
        }
        if sim.combat.state_hash(&sim.scene) != tick.pre {
            return Err(format!("pre-state hash differs at t={}", tick.t));
        }
        let hero = [tick.hero[0].to_raw(), tick.hero[1].to_raw()];
        sim.combat.step_party(&mut sim.scene, hero);
        let _ = sim.combat.take_respawned();
        if sim.combat.state_hash(&sim.scene) != tick.post {
            return Err(format!("post-state hash differs at t={}", tick.t));
        }
        let dropped = sim.combat.dropped_events as u64;
        let fresh = &sim.combat.events[(cursor - dropped) as usize..];
        events.fold(fresh);
        let replayed: Vec<EventRecord> = fresh
            .iter()
            .enumerate()
            .map(|(i, e)| EventRecord::from_combat(cursor + i as u64, e))
            .collect();
        cursor = dropped + sim.combat.events.len() as u64;
        if replayed != tick.ev || events.value() != tick.eh {
            return Err(format!("events differ at t={}", tick.t));
        }
        if sim.scene.world.tick != tick.et {
            return Err(format!("episode tick differs at t={}", tick.t));
        }
    }
    Ok(loaded.ticks.len() as u64)
}

/// The scripted live session: solo play with swaps, a manual restart, P2
/// joining, a pad disconnect (D6 pause) and its reconnect.
fn live_script(rig: &mut Rig, ticks: u64) {
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

fn solo_fixture() -> &'static PartyFixture {
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
    assert!(LiveRecorder::start(&blocker, manifest, Some(&sim.combat), &sim.party).is_err());
}
