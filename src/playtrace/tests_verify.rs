//! PT-C proofs: the verifier replays a recorded session from its reset
//! descriptor in a fresh process, the per-hero observations are built before
//! the decision from the sim camera, and every kind of tampering, drift or
//! mismatch is rejected with its own stable code.
//!
//! The rigs mirror the order of operations of `fixed_step` in `floor01_scene`
//! (see `live_tests`).

use std::path::{Path, PathBuf};

use super::build_info::SOURCE_FINGERPRINT;
use super::codec::{DiagRecord, decode_obs_bits};
use super::live_tests::{
    DemoRig, Rig, TempDir, demo_script, live_script, real_kit_hash, root, solo_fixture,
};
use super::obs::{diag_of, observe_pair};
use super::*;
use crate::autoplay::party::SELF_V2_LEN;
use crate::input::RawInput;
use crate::party::{HeroId, PadId};

fn verify(session: &Path) -> Result<VerifyReport, VerifyFailure> {
    verify_dir(session, &root())
}

pub(super) fn session_of(dir: &TempDir, rig: &mut Rig) -> PathBuf {
    let manifest = rig.finish().expect("finished");
    dir.0.join(manifest.session_id)
}

fn fnv(lines: &[String]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for line in lines {
        for byte in line.bytes().chain(std::iter::once(b'\n')) {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

/// Rewrite the tick chunks from `edit`'s result and re-seal them (trailers,
/// manifest checksums), so only the replay can notice the edit.
pub(super) fn tamper_ticks(session: &Path, edit: impl FnOnce(&mut Vec<TickRecord>)) {
    let mut loaded = read_session(session, false).unwrap();
    edit(&mut loaded.ticks);
    for chunk in 0..loaded.chunks.len() {
        let _ = std::fs::remove_file(session.join(chunk_file_name(chunk)));
    }
    let mut checksums = Vec::new();
    for (index, chunk) in loaded.ticks.chunks(TICKS_PER_CHUNK).enumerate() {
        let lines: Vec<String> = chunk
            .iter()
            .map(|t| serde_json::to_string(t).unwrap())
            .collect();
        let hash = fnv(&lines);
        let trailer = ChunkTrailer {
            end: true,
            chunk: index,
            ticks: chunk.len(),
            first_t: chunk.first().map(|t| t.t),
            last_t: chunk.last().map(|t| t.t),
            fnv: hash,
        };
        let mut body: String = lines.iter().map(|l| format!("{l}\n")).collect();
        body.push_str(&serde_json::to_string(&trailer).unwrap());
        body.push('\n');
        std::fs::write(session.join(chunk_file_name(index)), body).unwrap();
        checksums.push(ChunkChecksum {
            chunk: index,
            ticks: chunk.len(),
            fnv: hash,
        });
    }
    tamper_manifest(session, |m| {
        m.chunk_checksums = checksums;
    });
}

pub(super) fn tamper_manifest(session: &Path, edit: impl FnOnce(&mut SessionManifest)) {
    let path = session.join("manifest.json");
    let mut manifest: SessionManifest =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    edit(&mut manifest);
    std::fs::write(&path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
}

/// Rewrite `control.jsonl` and bring the manifest's record in line.
pub(super) fn tamper_control(session: &Path, edit: impl FnOnce(&mut Vec<ControlRecord>)) {
    let mut loaded = read_session(session, false).unwrap();
    edit(&mut loaded.control);
    let lines: Vec<String> = loaded
        .control
        .iter()
        .map(|c| serde_json::to_string(c).unwrap())
        .collect();
    let body: String = lines.iter().map(|l| format!("{l}\n")).collect();
    std::fs::write(session.join("control.jsonl"), body).unwrap();
    tamper_manifest(session, |m| {
        m.control_integrity = Some(StreamIntegrity {
            lines: lines.len() as u64,
            fnv: fnv(&lines),
        });
    });
}

pub(super) fn tamper_diag(session: &Path, edit: impl FnOnce(&mut Vec<DiagRecord>)) {
    let mut loaded = read_session(session, false).unwrap();
    edit(&mut loaded.diag);
    let lines: Vec<String> = loaded
        .diag
        .iter()
        .map(|c| serde_json::to_string(c).unwrap())
        .collect();
    let body: String = lines.iter().map(|l| format!("{l}\n")).collect();
    std::fs::write(session.join("diagnostics.jsonl"), body).unwrap();
    tamper_manifest(session, |m| {
        m.diagnostics_integrity = Some(StreamIntegrity {
            lines: lines.len() as u64,
            fnv: fnv(&lines),
        });
    });
}

fn expect_failure(session: &Path, code: &str) -> VerifyFailure {
    match verify(session) {
        Ok(report) => panic!("expected {code}, but the session verified: {report:?}"),
        Err(failure) => {
            assert_eq!(failure.code, code, "{failure}");
            failure
        }
    }
}

/// The solo swap script: Carl for a while, a swap to Donut, then back.
pub(super) fn solo_swap_script(rig: &mut Rig, ticks: u64, swap_at: &[u64]) {
    rig.sim.party.pad_connected(PadId(1));
    for tick in 0..ticks {
        let swap = swap_at.contains(&tick);
        rig.step([
            RawInput {
                move_x: if (tick / 90) % 2 == 0 { 1.0 } else { -0.5 },
                jump: tick % 70 == 11,
                attack: tick % 45 == 3,
                swap,
                ..RawInput::NONE
            },
            RawInput::NONE,
        ]);
    }
}

pub(super) fn coop_script(rig: &mut Rig, ticks: u64) {
    rig.sim.party.pad_connected(PadId(1));
    rig.sim.party.pad_connected(PadId(2));
    rig.sim.party.pad_start_pressed(PadId(2));
    for tick in 0..ticks {
        rig.step([
            RawInput {
                move_x: 1.0,
                attack: tick % 40 == 5,
                ..RawInput::NONE
            },
            RawInput {
                move_x: if tick % 160 < 80 { 0.75 } else { -1.0 },
                jump: tick % 55 == 9,
                physical: tick % 61 == 4,
                ..RawInput::NONE
            },
        ]);
    }
}

fn own_block(encoded: &str) -> Vec<u32> {
    decode_obs_bits(encoded).unwrap()[..SELF_V2_LEN].to_vec()
}

fn is_carl_seat(own: &[u32]) -> bool {
    f32::from_bits(own[crate::autoplay::party::observation::SELF_IS_CARL]) == 1.0
}

#[test]
fn a_live_session_verifies_from_the_reset_descriptor_in_a_fresh_process() {
    let dir = TempDir::new("vfy-fresh");
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    live_script(&mut rig, 1500);
    let session = session_of(&dir, &mut rig);
    let report = verify(&session).expect("verifies");
    assert_eq!(report.ticks, 1500);
    assert_eq!(report.scope, VerifyScope::Full);
    assert_eq!(report.state_hashes, 1500);
    assert_eq!(report.event_hashes, 1500);
    assert_eq!(report.observations, 3000, "two heroes per tick");
    assert_eq!(report.diagnostics, 1500);
    assert_eq!(report.routing, 1500);
    assert_eq!(report.effective_inputs, 1500, "the drive is live all along");
    assert!(
        report.final_observations >= 4,
        "the restart and the exit store o_(t+1): {report:?}"
    );
    assert!(report.episodes >= 2);
    assert_eq!(report.build, "identical");

    // A brand-new process (the test binary again) verifies it too.
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "playtrace::tests_verify::verify_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("PLAYTRACE_VERIFY_DIR", &session)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "child failed: {stdout}");
    assert!(
        stdout.contains("PLAYTRACE_VERIFY_OK ticks=1500"),
        "{stdout}"
    );
}

/// The fresh-process half of the test above: a no-op unless the parent sets
/// the directory.
#[test]
fn verify_child() {
    let Ok(dir) = std::env::var("PLAYTRACE_VERIFY_DIR") else {
        return;
    };
    match verify(Path::new(&dir)) {
        Ok(report) => println!("PLAYTRACE_VERIFY_OK ticks={}", report.ticks),
        Err(failure) => panic!("verification failed: {failure}"),
    }
}

#[test]
fn one_player_carl_with_an_ai_donut_observes_and_replays() {
    let dir = TempDir::new("vfy-carl");
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    solo_swap_script(&mut rig, 600, &[]);
    let session = session_of(&dir, &mut rig);
    let loaded = read_session(&session, false).unwrap();
    assert!(
        loaded
            .ticks
            .iter()
            .all(|t| { t.src == [ControlSource::HumanP1, ControlSource::ScriptedBot] })
    );
    // Donut is the AI: her effective input is the bot's decision, while her
    // routed (replay driver) input is idle.
    assert!(loaded.ticks.iter().all(|t| t.hero[1] == ActionBits::NONE));
    assert!(
        loaded
            .ticks
            .iter()
            .any(|t| t.eff.unwrap()[1] != ActionBits::NONE),
        "the follower bot moved"
    );
    // Both seats are recorded every tick, and the seat is in the vector.
    for tick in &loaded.ticks {
        let obs = tick.obs.as_ref().expect("o_t of both heroes");
        assert!(is_carl_seat(&own_block(&obs[0])));
        assert!(!is_carl_seat(&own_block(&obs[1])));
    }
    verify(&session).expect("verifies");
}

#[test]
fn one_player_donut_with_an_ai_carl_observes_and_replays() {
    let dir = TempDir::new("vfy-donut");
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    solo_swap_script(&mut rig, 700, &[40]);
    let session = session_of(&dir, &mut rig);
    let loaded = read_session(&session, false).unwrap();
    let after: Vec<&TickRecord> = loaded.ticks.iter().filter(|t| t.t > 60).collect();
    assert!(
        after
            .iter()
            .all(|t| t.src == [ControlSource::ScriptedBot, ControlSource::HumanP1]),
        "after the swap Donut is the human and Carl the bot"
    );
    // The human's input now drives Donut; Carl's routed input is idle and his
    // effective input is the bot's.
    assert!(after.iter().all(|t| t.hero[0] == ActionBits::NONE));
    assert!(after.iter().any(|t| t.hero[1] != ActionBits::NONE));
    assert!(after.iter().any(|t| t.eff.unwrap()[0] != ActionBits::NONE));
    let report = verify(&session).expect("verifies");
    assert_eq!(report.effective_inputs, 700);
}

#[test]
fn two_players_both_slots_are_recorded_and_replayed() {
    let dir = TempDir::new("vfy-2p");
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    coop_script(&mut rig, 700);
    let session = session_of(&dir, &mut rig);
    let loaded = read_session(&session, false).unwrap();
    assert!(
        loaded
            .ticks
            .iter()
            .all(|t| t.src == [ControlSource::HumanP1, ControlSource::HumanP2])
    );
    assert!(loaded.ticks.iter().any(|t| t.hero[0] != t.hero[1]));
    // Two independent slots reach their own hero.
    assert!(loaded.ticks.iter().all(|t| t.hero == t.slots));
    verify(&session).expect("verifies");
}

#[test]
fn observation_and_action_are_aligned_before_the_tick() {
    let dir = TempDir::new("vfy-align");
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    // Collect the observation the policy would have had right before each
    // decision, straight from the live state, as an independent witness.
    rig.sim.party.pad_connected(PadId(1));
    let mut witness = Vec::new();
    for tick in 0..300u64 {
        witness.push(observe_pair(&rig.sim.scene, &rig.sim.combat, &rig.sim.party).encode());
        rig.step([
            RawInput {
                move_x: 1.0,
                attack: tick % 30 == 2,
                jump: tick % 50 == 9,
                ..RawInput::NONE
            },
            RawInput::NONE,
        ]);
    }
    let session = session_of(&dir, &mut rig);
    let loaded = read_session(&session, false).unwrap();
    for (tick, expected) in loaded.ticks.iter().zip(&witness) {
        assert_eq!(tick.obs.as_ref(), Some(expected), "o_t of tick {}", tick.t);
    }
    // o_t never contains the effect of a_t: changing the action at tick 5
    // leaves o_5 alone and breaks the replay at 5, not before.
    tamper_ticks(&session, |ticks| {
        ticks[5].hero[0].1 ^= 1;
        ticks[5].slots[0].1 ^= 1;
    });
    let failure = verify(&session).unwrap_err();
    assert_eq!(failure.tick, Some(5), "{failure}");
    assert!(
        ["effective_input", "post_state"].contains(&failure.code),
        "{failure}"
    );
}

#[test]
fn recording_is_deterministic_world_events_and_observations() {
    let run = |name: &str| {
        let dir = TempDir::new(name);
        let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
        live_script(&mut rig, 900);
        let session = session_of(&dir, &mut rig);
        let loaded = read_session(&session, false).unwrap();
        let ticks: Vec<String> = loaded
            .ticks
            .iter()
            .map(|t| serde_json::to_string(t).unwrap())
            .collect();
        let control: Vec<String> = loaded
            .control
            .iter()
            .map(|c| serde_json::to_string(c).unwrap())
            .collect();
        let diag: Vec<String> = loaded
            .diag
            .iter()
            .map(|d| serde_json::to_string(d).unwrap())
            .collect();
        (ticks, control, diag)
    };
    let a = run("vfy-det-a");
    let b = run("vfy-det-b");
    assert_eq!(a.0, b.0, "tick records (hashes, events, obs, effective)");
    assert_eq!(a.1, b.1, "control journal");
    assert_eq!(a.2, b.2, "diagnostics");
}

#[test]
fn the_story_route_without_combat_replays_without_observations() {
    use crate::corridor::SceneSim;
    use crate::party::PartyState;
    let dir = TempDir::new("vfy-story");
    let kit = super::live_tests::kit();
    let mut scene = SceneSim::new(&kit, 360).unwrap();
    let mut party = PartyState::default();
    let reset = ResetDescriptor {
        source: "story".to_owned(),
        name: String::new(),
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
        start_hash: scene.state_hash(),
    };
    let mut manifest = super::live_tests::stamped(reset);
    manifest.mode = "solo_human".to_owned();
    let mut rec = live::LiveRecorder::start(&dir.0, manifest, &scene, None, &party, false).unwrap();
    assert!(
        rec.manifest().observation_is_partial,
        "no combat state, no observation"
    );
    party.pad_connected(PadId(1));
    for tick in 0..500u64 {
        if tick == 250 {
            scene.reset();
            party.restart();
            rec.note_restart(&scene, None, &party);
        }
        rec.observe_party(&party);
        let slots = [
            RawInput {
                move_x: 1.0,
                jump: tick % 60 == 5,
                swap: tick == 100,
                ..RawInput::NONE
            },
            RawInput::NONE,
        ];
        party.request_swap(slots[0].swap, false);
        let hero = party.hero_inputs(slots);
        let drive = party.drive(true);
        scene.set_drive(drive);
        rec.begin_tick(&scene, None, &party, slots, hero, drive);
        scene.step_party(hero);
        rec.end_tick(&scene, None, &party, false);
    }
    let manifest = rec.finish("test").unwrap();
    let session = dir.0.join(manifest.session_id);
    let loaded = read_session(&session, false).unwrap();
    assert!(
        loaded
            .ticks
            .iter()
            .all(|t| t.obs.is_none() && t.eff.is_some())
    );
    assert!(loaded.diag.is_empty());
    let report = verify(&session).expect("the story route replays");
    assert_eq!(report.ticks, 500);
    assert_eq!(report.observations, 0);
    assert_eq!(report.effective_inputs, 500);
    assert!(
        report
            .notes
            .iter()
            .any(|n| n.contains("observations not checked"))
    );
    // ...and a tampered story input is rejected.
    tamper_ticks(&session, |ticks| {
        ticks[40].hero[0] = ActionBits::NONE;
        ticks[40].slots[0] = ActionBits::NONE;
    });
    let failure = verify(&session).unwrap_err();
    assert_eq!(failure.tick.map(|t| t >= 40), Some(true), "{failure}");
}

#[test]
fn the_demo_route_checks_o_t1_at_respawn_boundaries() {
    let dir = TempDir::new("vfy-demo");
    let mut rig = DemoRig::new(&dir.0, false, 4);
    demo_script(&mut rig, 2400);
    let manifest = rig.rec.finish("test").unwrap();
    let session = dir.0.join(manifest.session_id);
    let report = verify(&session).expect("verifies");
    assert!(report.final_observations >= 6, "{report:?}");
    assert!(report.episodes >= 3);
    // A forged terminal observation is caught at its boundary.
    tamper_control(&session, |control| {
        let victim = control
            .iter_mut()
            .find(|c| {
                matches!(&c.ev, ControlEvent::EpisodeEnd { outcome, final_obs: Some(_), .. }
                    if outcome == "demo_respawn")
            })
            .expect("a select-tap boundary");
        if let ControlEvent::EpisodeEnd { final_obs, .. } = &mut victim.ev {
            let obs = final_obs.as_mut().unwrap();
            obs[0] = encode_obs_bits(&[0x3f80_0000]);
        }
    });
    expect_failure(&session, "final_observation");
}

// ---------------------------------------------------------------- negative

pub(super) fn recorded_solo(name: &str, ticks: u64) -> (TempDir, PathBuf) {
    let dir = TempDir::new(name);
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    live_script(&mut rig, ticks);
    let session = session_of(&dir, &mut rig);
    (dir, session)
}

#[test]
fn a_tampered_input_is_rejected_with_its_tick() {
    let (_dir, session) = recorded_solo("vfy-neg-input", 400);
    verify(&session).expect("baseline verifies");
    tamper_ticks(&session, |ticks| {
        for tick in ticks.iter_mut().skip(30).take(1) {
            tick.hero[0].1 ^= 2;
            tick.slots[0].1 ^= 2;
        }
    });
    let failure = verify(&session).unwrap_err();
    assert_eq!(
        failure.tick.map(|t| (30..=31).contains(&t)),
        Some(true),
        "{failure}"
    );
}

#[test]
fn a_deleted_restart_control_line_is_rejected() {
    let (_dir, session) = recorded_solo("vfy-neg-restart", 700);
    // On disk: the control integrity record and `seq` catch it.
    let path = session.join("control.jsonl");
    let original = std::fs::read_to_string(&path).unwrap();
    let kept: String = original
        .lines()
        .filter(|l| !l.contains("\"k\":\"restart\""))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_ne!(kept, original);
    std::fs::write(&path, kept).unwrap();
    expect_failure(&session, "invalid_session");
    std::fs::write(&path, &original).unwrap();
    // A forger who repairs `seq` and the manifest is caught by the replay: the
    // scene never restarted, so the state hash differs at the next tick.
    tamper_control(&session, |control| {
        control.retain(|c| c.ev != ControlEvent::Restart);
        for (index, record) in control.iter_mut().enumerate() {
            record.seq = index as u64;
        }
    });
    let failure = verify(&session).unwrap_err();
    assert!(
        ["pre_state", "final_observation", "episode"].contains(&failure.code),
        "{failure}"
    );
}

#[test]
fn a_wrong_build_is_rejected_and_an_identical_source_is_accepted() {
    let (_dir, session) = recorded_solo("vfy-neg-build", 120);
    // A different source tree.
    tamper_manifest(&session, |m| {
        m.source_fingerprint = "0123456789abcdef".to_owned()
    });
    let failure = expect_failure(&session, "build_mismatch");
    assert!(failure.message.contains("0123456789abcdef"));
    // A session with no fingerprint (a PT-B session) cannot be identified.
    tamper_manifest(&session, |m| m.source_fingerprint.clear());
    expect_failure(&session, "build_mismatch");
    // The same source under another commit (a docs-only commit in between) is
    // fine, and says so.
    tamper_manifest(&session, |m| {
        m.source_fingerprint = SOURCE_FINGERPRINT.to_owned();
        m.build_commit = "f".repeat(40);
        m.build_dirty = true;
    });
    let report = verify(&session).expect("same source, other commit");
    assert_eq!(report.build, "source_identical_commit_differs");
}

#[test]
fn content_and_rules_mismatches_are_rejected() {
    let (_dir, session) = recorded_solo("vfy-neg-content", 120);
    let baseline = std::fs::read_to_string(session.join("manifest.json")).unwrap();
    let restore = || std::fs::write(session.join("manifest.json"), &baseline).unwrap();
    // CARL-PROG-01 CP-Q46: the stamped manifest carries the two data pins and a
    // mismatch of either fails the replay naming it.
    let stamped = std::fs::read_to_string(session.join("manifest.json")).unwrap();
    for key in [
        "story_unlocks",
        "carl_prog",
        "donut_prog_tuning",
        "llama_rush_tuning",
        crate::slime::PIN_KEY,
        crate::rot_sticker::PIN_KEY,
    ] {
        assert!(stamped.contains(&format!("\"{key}\"")), "{key} is stamped");
    }
    for key in [
        "kit",
        "mana_tuning",
        "cf02_tuning",
        "story_unlocks",
        "carl_prog",
        "donut_prog_tuning",
        "llama_rush_tuning",
        crate::slime::PIN_KEY,
        crate::rot_sticker::PIN_KEY,
        "party_tuning",
    ] {
        tamper_manifest(&session, |m| {
            m.content_hashes
                .insert(key.to_owned(), "00000000000000aa".to_owned());
        });
        let failure = expect_failure(&session, "content_mismatch");
        assert!(failure.message.contains(key), "{failure}");
        restore();
    }
    for key in [
        "mana_tuning",
        "story_unlocks",
        "carl_prog",
        "donut_prog_tuning",
        "llama_rush_tuning",
        crate::slime::PIN_KEY,
        crate::rot_sticker::PIN_KEY,
    ] {
        tamper_manifest(&session, |m| {
            m.content_hashes.remove(key);
        });
        expect_failure(&session, "content_mismatch");
        restore();
    }
    // DONUT-PROG-01 DP-Q46: the Donut tuning pin is required like the others,
    // and a descriptor whose Donut start flag breaks the story order (no
    // biscuit) is a failure, never a panic.
    for flag in 0..2 {
        tamper_manifest(&session, |m| {
            // A start flag is read by the DEMO-01 hall (a plain fixture ignores it).
            m.reset.source = "demo".to_owned();
            m.reset.demo = true;
            m.reset.donut_biscuit = false;
            if flag == 0 {
                m.reset.donut_light_feet = true;
            } else {
                m.reset.donut_puddle = true;
            }
        });
        let failure = expect_failure(&session, "rules_mismatch");
        assert!(failure.message.contains("start_flags["), "{failure}");
        restore();
    }
    tamper_manifest(&session, |m| m.reset.kit_hash ^= 1);
    expect_failure(&session, "content_mismatch");
    restore();
    tamper_manifest(&session, |m| m.game_ruleset_id = "floor02".to_owned());
    expect_failure(&session, "rules_mismatch");
    restore();
    tamper_manifest(&session, |m| {
        m.event_schema_id = "combat-event/9".to_owned()
    });
    expect_failure(&session, "rules_mismatch");
    restore();
    tamper_manifest(&session, |m| m.observation_schema_id = "obs/9".to_owned());
    expect_failure(&session, "rules_mismatch");
    restore();
    tamper_manifest(&session, |m| {
        m.feature_flags
            .insert("cf02_rules".to_owned(), "true".to_owned());
    });
    expect_failure(&session, "rules_mismatch");
    restore();
    // A different scene: the fresh reset no longer hashes to the start hash.
    tamper_manifest(&session, |m| m.reset.start_hash ^= 1);
    expect_failure(&session, "reset_mismatch");
    restore();
    tamper_manifest(&session, |m| m.reset.name = "no_such_fixture".to_owned());
    expect_failure(&session, "unsupported_source");
    restore();
    tamper_manifest(&session, |m| m.reset.source = "future_scene".to_owned());
    expect_failure(&session, "unsupported_source");
    restore();
    verify(&session).expect("restored session verifies");
}

#[test]
fn every_recorded_observation_effective_input_and_diagnostic_is_checked() {
    let (_dir, session) = recorded_solo("vfy-neg-fields", 400);
    let baseline = read_session(&session, false).unwrap();

    // o_t of one hero.
    tamper_ticks(&session, |ticks| {
        let obs = ticks[77].obs.as_mut().unwrap();
        let mut bits = decode_obs_bits(&obs[1]).unwrap();
        bits[0] ^= 0x0040_0000;
        obs[1] = encode_obs_bits(&bits);
    });
    let failure = expect_failure(&session, "observation");
    assert_eq!(failure.tick, Some(77));
    tamper_ticks(&session, |ticks| *ticks = baseline.ticks.clone());

    // o_t removed.
    tamper_ticks(&session, |ticks| ticks[10].obs = None);
    expect_failure(&session, "observation");
    tamper_ticks(&session, |ticks| *ticks = baseline.ticks.clone());

    // The live effective-input log.
    tamper_ticks(&session, |ticks| {
        let eff = ticks[120].eff.as_mut().unwrap();
        eff[1].0 = (0.5_f32).to_bits();
    });
    let failure = expect_failure(&session, "effective_input");
    assert_eq!(failure.tick, Some(120));
    tamper_ticks(&session, |ticks| *ticks = baseline.ticks.clone());
    tamper_ticks(&session, |ticks| ticks[3].eff = None);
    expect_failure(&session, "effective_input");
    tamper_ticks(&session, |ticks| *ticks = baseline.ticks.clone());

    // The per-slot provenance must route to the replay driver.
    tamper_ticks(&session, |ticks| ticks[50].slots[0].1 ^= 1);
    expect_failure(&session, "slot_routing");
    tamper_ticks(&session, |ticks| *ticks = baseline.ticks.clone());

    // Hashes and events.
    tamper_ticks(&session, |ticks| ticks[200].post ^= 1);
    expect_failure(&session, "post_state");
    tamper_ticks(&session, |ticks| *ticks = baseline.ticks.clone());
    tamper_ticks(&session, |ticks| ticks[200].pre ^= 1);
    expect_failure(&session, "pre_state");
    tamper_ticks(&session, |ticks| *ticks = baseline.ticks.clone());
    tamper_ticks(&session, |ticks| ticks[300].eh ^= 1);
    expect_failure(&session, "event_hash");
    tamper_ticks(&session, |ticks| *ticks = baseline.ticks.clone());
    tamper_ticks(&session, |ticks| ticks[300].et += 1);
    expect_failure(&session, "episode_tick");
    tamper_ticks(&session, |ticks| *ticks = baseline.ticks.clone());
    tamper_ticks(&session, |ticks| ticks[310].ep += 5);
    expect_failure(&session, "episode");
    tamper_ticks(&session, |ticks| *ticks = baseline.ticks.clone());
    let with_events = baseline
        .ticks
        .iter()
        .position(|t| !t.ev.is_empty())
        .expect("the script produces combat events");
    tamper_ticks(&session, |ticks| {
        ticks[with_events].ev[0].d.push('!');
    });
    expect_failure(&session, "events");
    tamper_ticks(&session, |ticks| *ticks = baseline.ticks.clone());

    // The privileged diagnostics (but not the rig rectangle).
    tamper_diag(&session, |diag| diag[90].hp = [1, 1]);
    let failure = expect_failure(&session, "diagnostics");
    assert_eq!(failure.tick, Some(90));
    tamper_diag(&session, |diag| *diag = baseline.diag.clone());
    tamper_diag(&session, |diag| {
        diag[90].rig = Some([-5000, 7, 1, 1]);
    });
    verify(&session).expect("the rig rectangle is a diagnostic only: it is not replayed");
    tamper_diag(&session, |diag| {
        diag.remove(5);
    });
    expect_failure(&session, "diagnostics");
    tamper_diag(&session, |diag| *diag = baseline.diag.clone());
    verify(&session).expect("restored");
}

/// F-2: the control-source labels and the stage are checked per tick. Every
/// probe re-seals the chunks, so only the replay can notice.
#[test]
fn a_relabelled_control_source_or_stage_is_rejected() {
    let dir = TempDir::new("vfy-neg-labels");
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    solo_swap_script(&mut rig, 600, &[40]);
    let session = session_of(&dir, &mut rig);
    let baseline = read_session(&session, false).unwrap();
    let report = verify(&session).expect("baseline verifies");
    assert_eq!(report.labels, 600);
    let restore = |session: &Path| tamper_ticks(session, |t| *t = baseline.ticks.clone());

    // AI Donut relabelled as the second human on every tick.
    tamper_ticks(&session, |ticks| {
        for tick in ticks.iter_mut() {
            tick.src = [ControlSource::HumanP1, ControlSource::HumanP2];
        }
    });
    let failure = expect_failure(&session, "labels");
    assert_eq!(failure.tick, Some(0));
    restore(&session);

    // Human and bot swapped on every tick.
    tamper_ticks(&session, |ticks| {
        for tick in ticks.iter_mut() {
            tick.src.swap(0, 1);
        }
    });
    expect_failure(&session, "labels");
    restore(&session);

    // A single tick is enough, and the failure names it.
    tamper_ticks(&session, |ticks| ticks[321].src[1] = ControlSource::None);
    assert_eq!(expect_failure(&session, "labels").tick, Some(321));
    restore(&session);

    // The stage (with `ph` changed too) must follow the replayed checkpoint.
    tamper_ticks(&session, |ticks| {
        for tick in ticks.iter_mut() {
            tick.stage += 3;
            tick.ph ^= 1;
        }
    });
    expect_failure(&session, "labels");
    restore(&session);

    // The run kind decides what a player-driven hero is labelled.
    tamper_manifest(&session, |m| m.run_kind = RunKind::ScriptedBot);
    expect_failure(&session, "labels");
    tamper_manifest(&session, |m| m.run_kind = RunKind::LearnedPolicy);
    expect_failure(&session, "labels");
    tamper_manifest(&session, |m| m.run_kind = RunKind::Human);
    verify(&session).expect("restored");
}

/// F-2 for a scripted run: a forced `--record` of an autopilot run labels the
/// slot inputs `scripted_bot`, and claiming it was a human run is rejected.
#[test]
fn a_scripted_run_cannot_be_relabelled_as_human() {
    let dir = TempDir::new("vfy-neg-scripted");
    let mut rig = DemoRig::new(&dir.0, true, 4);
    demo_script(&mut rig, 900);
    let manifest = rig.rec.finish("test").unwrap();
    let session = dir.0.join(manifest.session_id);
    // The recorder labels from the flag it was given; the manifest of this
    // test rig says `human`, so the honest session is the scripted one.
    tamper_manifest(&session, |m| m.run_kind = RunKind::ScriptedBot);
    verify(&session).expect("a scripted run verifies as scripted");
    tamper_ticks(&session, |ticks| {
        for tick in ticks.iter_mut() {
            tick.src[0] = ControlSource::HumanP1;
        }
    });
    expect_failure(&session, "labels");
}

/// COMPANION-AI-01 on a live run: the companion-driven hero is labelled
/// `companion_ai`, its injected input is the recorded effective input, and the
/// verifier rebuilds the same tuning from the manifest.
#[test]
fn a_companion_driven_hero_is_labelled_recorded_and_replayed() {
    use crate::companion::profile::ProfileSetId;
    use crate::party::{PROVISIONAL_TUNING, PartyState, PartyTuning, UncontrolledHero};
    let dir = TempDir::new("vfy-companion");
    let party = PartyState::new(PartyTuning {
        uncontrolled_hero: UncontrolledHero::Companion(ProfileSetId::Cf02Assist),
        ..PROVISIONAL_TUNING
    });
    let mut rig = DemoRig::with_party(&dir.0, false, 4, party);
    // Carl walks, so the companion has a leader to follow.
    for tick in 0..1500_u64 {
        rig.step(RawInput {
            move_x: if tick % 400 < 200 { 1.0 } else { 0.0 },
            ..RawInput::NONE
        });
    }
    let manifest = rig.rec.finish("test").unwrap();
    assert_eq!(
        manifest.feature_flags.get("companion").map(String::as_str),
        Some("cf02_assist")
    );
    let session = dir.0.join(manifest.session_id);
    let loaded = read_session(&session, false).unwrap();
    assert!(
        loaded
            .ticks
            .iter()
            .all(|t| t.src == [ControlSource::HumanP1, ControlSource::CompanionAi])
    );
    assert!(
        loaded
            .ticks
            .iter()
            .any(|t| t.eff.unwrap()[1] != ActionBits::NONE),
        "the companion's input is the recorded effective input"
    );
    let report = verify(&session).expect("a companion run verifies");
    assert_eq!(report.effective_inputs, 1500);
    // The companion is part of the identity: another choice is a content
    // mismatch, and calling the companion a plain bot is a label mismatch.
    tamper_manifest(&session, |m| {
        m.feature_flags.insert("companion".into(), "off".into());
    });
    expect_failure(&session, "content_mismatch");
    tamper_manifest(&session, |m| {
        m.feature_flags
            .insert("companion".into(), "cf02_assist".into());
    });
    tamper_ticks(&session, |ticks| {
        for tick in ticks.iter_mut() {
            tick.src[1] = ControlSource::ScriptedBot;
        }
    });
    expect_failure(&session, "labels");
}

/// CF02-F: a live `--cf02` session (the microarena, its checkpoint rules, the
/// `cf02_assist` companion and the 12-tick swap / eased pan) replays from its reset
/// descriptor in a fresh process; the same recording is rejected when it claims the
/// CF02 rules were off (the verifier would rebuild another room).
#[test]
fn a_cf02_room_session_verifies_from_its_reset_descriptor() {
    use crate::companion::profile::ProfileSetId;
    use crate::party::{PROVISIONAL_TUNING, PartyState, PartyTuning, UncontrolledHero};
    let dir = TempDir::new("vfy-cf02-room");
    let party = PartyState::new(
        PartyTuning {
            uncontrolled_hero: UncontrolledHero::Companion(ProfileSetId::Cf02Assist),
            ..PROVISIONAL_TUNING
        }
        .with_cf02_swap(),
    );
    let mut rig = DemoRig::cf02(&dir.0, party);
    for tick in 0..700_u64 {
        match tick {
            420 => rig.restart(),
            560 => rig.select_tap_respawn(),
            _ => {}
        }
        // Carl walks right (into the first wave); one solo swap and one swap back.
        rig.step(RawInput {
            move_x: if tick % 300 < 240 { 1.0 } else { 0.0 },
            swap: tick == 120 || tick == 180,
            attack: tick % 11 == 0,
            ..RawInput::NONE
        });
    }
    let manifest = rig.rec.finish("test").unwrap();
    let session = dir.0.join(manifest.session_id);
    let report = verify(&session).expect("a --cf02 room session verifies");
    assert_eq!(report.effective_inputs, 700);
    // The recording is bound to the CF02 rules: without them it is another game.
    tamper_manifest(&session, |m| m.reset.cf02_rules = false);
    assert!(
        verify(&session).is_err(),
        "rules off: another room, another run"
    );
}

#[test]
fn an_incomplete_capture_is_not_verified() {
    let (_dir, session) = recorded_solo("vfy-neg-incomplete", 200);
    tamper_manifest(&session, |m| {
        m.status = SessionStatus::Incomplete;
        m.dropped_control = 1;
    });
    // `incomplete` with a counted drop reads, but is not a complete record.
    expect_failure(&session, "capture_incomplete");
}

#[test]
fn a_crash_recovered_session_verifies_its_prefix_only() {
    let dir = TempDir::new("vfy-crash");
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    live_script(&mut rig, 900);
    // Copy the live session directory while the recorder is still alive: the
    // copy is what a kill -9 leaves behind (no trailer on the open chunk, no
    // final manifest, the open batch of diagnostics unwritten).
    let live_dir = std::fs::read_dir(&dir.0)
        .unwrap()
        .flatten()
        .next()
        .unwrap()
        .path();
    let copy = dir.0.join("crashed-copy");
    std::fs::create_dir_all(&copy).unwrap();
    for entry in std::fs::read_dir(&live_dir).unwrap().flatten() {
        std::fs::copy(entry.path(), copy.join(entry.file_name())).unwrap();
    }
    let report = verify(&copy).expect("the valid prefix verifies");
    assert_eq!(report.scope, VerifyScope::Prefix);
    assert!(report.ticks >= 600, "{report:?}");
    assert!(report.notes.iter().any(|n| n.contains("prefix only")));
    drop(rig);
}

#[test]
fn a_pt_b_session_without_observations_or_identity_cannot_be_verified() {
    let (_dir, session) = recorded_solo("vfy-neg-ptb", 100);
    tamper_manifest(&session, |m| {
        m.source_fingerprint.clear();
        m.observation_is_partial = true;
        m.observation_schema_id = "none".to_owned();
    });
    expect_failure(&session, "build_mismatch");
}

// ------------------------------------------------------------- visibility

fn enemy_scene() -> Rig {
    // The first party fixture with goblins: its first goblin is the probe.
    let fixture = crate::party_scene::PARTY_FIXTURES
        .into_iter()
        .find(|f| !Rig::new(f, None).sim.combat.enemies.is_empty())
        .expect("a party fixture with goblins");
    Rig::new(fixture, None)
}

#[test]
fn hidden_enemy_state_never_reaches_the_observation_but_is_in_the_diagnostics() {
    let mut a = enemy_scene();
    let mut b = enemy_scene();
    let view_right = (a.sim.scene.camera.left() + a.sim.scene.camera.config().view_w) as f32;
    // Both put the goblin well outside the sim camera, to the right.
    a.sim.combat.enemies[0].x = view_right + 700.0;
    b.sim.combat.enemies[0].x = view_right + 900.0;
    b.sim.combat.enemies[0].health = b.sim.combat.enemies[0].health.saturating_sub(1).max(1);
    let obs_a = observe_pair(&a.sim.scene, &a.sim.combat, &a.sim.party);
    let obs_b = observe_pair(&b.sim.scene, &b.sim.combat, &b.sim.party);
    assert_eq!(
        obs_a.bits, obs_b.bits,
        "an unseen enemy's position and health leave both observations unchanged"
    );
    let diag_a = diag_of(0, &a.sim.scene, &a.sim.combat, None, obs_a.stats);
    assert!(diag_a.enemies_hidden >= 1, "the diagnostics know it exists");
    // Seen, the same goblin changes the observation.
    b.sim.combat.enemies[0].x = a.sim.scene.world.hero.x + 60.0;
    let seen = observe_pair(&b.sim.scene, &b.sim.combat, &b.sim.party);
    assert_ne!(seen.bits[0], obs_a.bits[0]);
    assert_ne!(seen.bits[1], obs_a.bits[1]);
    let visible = |bits: &[u32]| {
        // enemy_mask starts after own (22), ally (24) and ally_mask (1) and 16*14 rows.
        let mask = SELF_V2_LEN + 24 + 1 + 16 * 14;
        bits[mask..mask + 16].iter().filter(|v| **v != 0).count()
    };
    assert!(visible(&seen.bits[0]) > visible(&obs_a.bits[0]));
}

#[test]
fn the_rig_view_is_a_diagnostic_and_never_changes_the_observation() {
    let record = |name: &str, rig_view: Option<[i32; 4]>| {
        let dir = TempDir::new(name);
        let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
        rig.sim.party.pad_connected(PadId(1));
        for tick in 0..240u64 {
            rig.rec.as_mut().unwrap().set_rig_view(rig_view);
            rig.step([
                RawInput {
                    move_x: 1.0,
                    attack: tick % 33 == 1,
                    ..RawInput::NONE
                },
                RawInput::NONE,
            ]);
        }
        let session = session_of(&dir, &mut rig);
        (dir, read_session(&session, false).unwrap())
    };
    let (_d1, plain) = record("vfy-rig-a", Some([0, 0, 640, 360]));
    let (_d2, wild) = record("vfy-rig-b", Some([1234, 55, 300, 200]));
    let (_d3, none) = record("vfy-rig-c", None);
    for ((a, b), c) in plain.ticks.iter().zip(&wild.ticks).zip(&none.ticks) {
        assert_eq!(a, b, "the tick record (o_t included) ignores the rig");
        assert_eq!(a, c);
    }
    assert_eq!(plain.diag[0].rig, Some([0, 0, 640, 360]));
    assert_eq!(wild.diag[0].rig, Some([1234, 55, 300, 200]));
    assert_eq!(none.diag[0].rig, None);
    // The sim camera, which the observation does use, is recorded next to it.
    assert_eq!(plain.diag[0].cam, wild.diag[0].cam);
    assert_eq!(plain.diag[0].cam[2], 640);
    for (a, b) in plain.diag.iter().zip(&wild.diag) {
        assert_eq!(
            DiagRecord {
                rig: None,
                ..a.clone()
            },
            DiagRecord {
                rig: None,
                ..b.clone()
            }
        );
    }
}

#[test]
fn o_t_plus_one_is_stored_only_at_boundaries() {
    let (_dir, session) = recorded_solo("vfy-boundary", 1500);
    let loaded = read_session(&session, false).unwrap();
    let ends: Vec<&ControlRecord> = loaded
        .control
        .iter()
        .filter(|c| matches!(c.ev, ControlEvent::EpisodeEnd { .. }))
        .collect();
    assert!(ends.len() >= 2);
    for end in &ends {
        let ControlEvent::EpisodeEnd { final_obs, .. } = &end.ev else {
            unreachable!()
        };
        let obs = final_obs.as_ref().expect("every boundary stores o_(t+1)");
        assert_eq!(
            decode_obs_bits(&obs[0]).unwrap().len(),
            decode_obs_bits(&obs[1]).unwrap().len()
        );
    }
    // Per tick there is exactly o_t, never a second observation.
    let json = std::fs::read_to_string(session.join("ticks-000000.jsonl")).unwrap();
    assert!(!json.contains("next_obs") && !json.contains("final_obs"));
    // The last boundary is the exit (truncation): its o_(t+1) is the state
    // after the last recorded tick, the same as the replay's.
    let last = ends.last().unwrap();
    assert_eq!(last.at_t, loaded.ticks.len() as u64);
    let size_bytes: u64 = std::fs::read_dir(&session)
        .unwrap()
        .flatten()
        .map(|e| e.metadata().map_or(0, |m| m.len()))
        .sum();
    // Not an assertion on speed (PT-E): a sanity bound so a regression to
    // dense float text is noticed.
    assert!(
        size_bytes / loaded.ticks.len() as u64 <= 4096,
        "{} bytes per tick",
        size_bytes / loaded.ticks.len() as u64
    );
    let _ = HeroId::Carl;
}
