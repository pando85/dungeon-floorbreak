//! PTR-R0 proofs: the replay cursor is the verifier's state machine made
//! reusable. It drives the same sessions to the same hashes, reports the
//! lifecycle of every tick, and its checked cross-build policy waives exactly
//! the source-fingerprint predicate and nothing else.
//!
//! Every session comes from the in-crate `live_tests` rigs (synthetic, no
//! human data); a cross-build session is one whose `source_fingerprint` is
//! rewritten after recording, like a recording made by another build.

use std::path::Path;

use super::build_info::SOURCE_FINGERPRINT;
use super::codec::{decode_obs_bits, encode_obs_bits};
use super::live_tests::{DemoRig, Rig, TempDir, demo_script, live_script, root, solo_fixture};
use super::tests_verify::{
    coop_script, recorded_solo, session_of, solo_swap_script, tamper_control, tamper_diag,
    tamper_manifest, tamper_ticks,
};
use super::*;
use crate::input::RawInput;

/// What a full cursor run produced.
struct Run {
    outcomes: Vec<TickOutcome>,
    counters: ReplayCounters,
    trust: Trust,
    loaded: LoadedSession,
    post: u64,
}

fn open_run(session: &Path, policy: ReplayPolicy) -> Result<Run, VerifyFailure> {
    let loaded = read_session(session, true)
        .map_err(|e| VerifyFailure::new("invalid_session", e.to_string()))?;
    let (mut cursor, mut scene, mut combat, _tuning, trust) =
        ReplayCursor::open(&loaded, &root(), policy)?;
    let mut outcomes = Vec::new();
    while let Some(outcome) = cursor.next_tick(&loaded, &mut scene, combat.as_mut())? {
        outcomes.push(outcome);
    }
    assert!(cursor.is_finished());
    let post = state_hash_of(&scene, combat.as_ref());
    Ok(Run {
        outcomes,
        counters: cursor.counters(),
        trust,
        loaded,
        post,
    })
}

fn strict(session: &Path) -> Result<Run, VerifyFailure> {
    open_run(session, ReplayPolicy::Strict)
}

fn checked(session: &Path) -> Result<Run, VerifyFailure> {
    open_run(session, ReplayPolicy::CheckedCrossBuild)
}

/// A recording by another build: the fingerprint is rewritten after the fact.
fn make_cross_build(session: &Path) {
    tamper_manifest(session, |m| {
        m.source_fingerprint = "0123456789abcdef".to_owned();
        m.build_commit = "e".repeat(40);
    });
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
    }
}

/// The cursor's per-tick report agrees with the journal and with the verifier.
fn assert_matches_journal(run: &Run, session: &Path) {
    let loaded = &run.loaded;
    assert_eq!(run.outcomes.len(), loaded.ticks.len());
    let mut next_event = 0;
    for (outcome, tick) in run.outcomes.iter().zip(&loaded.ticks) {
        assert_eq!(outcome.t, tick.t);
        assert_eq!(outcome.ep, tick.ep);
        assert_eq!(outcome.et, tick.et);
        assert_eq!(outcome.src, tick.src);
        assert_eq!(outcome.events.start, next_event, "contiguous event ranges");
        assert_eq!(
            outcome.events.end - outcome.events.start,
            tick.ev.len() as u64
        );
        next_event = outcome.events.end;
    }
    assert_eq!(
        run.post,
        loaded.ticks.last().expect("a session has ticks").post
    );
    let report = verify_dir(session, &root()).expect("the verifier agrees");
    let c = run.counters;
    assert_eq!(report.ticks, c.ticks);
    assert_eq!(report.state_hashes, c.state_hashes);
    assert_eq!(report.event_hashes, c.event_hashes);
    assert_eq!(report.effective_inputs, c.effective_inputs);
    assert_eq!(report.observations, c.observations);
    assert_eq!(report.final_observations, c.final_observations);
    assert_eq!(report.diagnostics, c.diagnostics);
    assert_eq!(report.routing, c.routing);
    assert_eq!(report.labels, c.labels);
    assert_eq!(report.combat_events, c.combat_events);
    assert_eq!(report.combat_events, next_event);
}

// ------------------------------------------------------------- routes

#[test]
fn the_cursor_replays_a_party_session_with_restart_p2_join_and_a_d6_pause() {
    let dir = TempDir::new("cur-live");
    let mut rig = Rig::new(solo_fixture(), Some(&dir.0));
    live_script(&mut rig, 1500);
    let session = session_of(&dir, &mut rig);
    let run = strict(&session).expect("replays");
    assert_eq!(run.trust, Trust::Strict);
    assert_matches_journal(&run, &session);
    // The manual restart at tick 300 is reported on exactly one tick, and the
    // pre-step controls of that tick say so.
    let restarted: Vec<&TickOutcome> = run.outcomes.iter().filter(|o| o.restarted).collect();
    assert_eq!(restarted.len(), 1, "one manual restart");
    assert!(restarted[0].lifecycle_reset());
    assert!(restarted[0].applied.contains(&ControlEvent::Restart));
    // Solo until Donut's pad joins, two-player after, with the drive reported.
    assert!(!run.outcomes.first().unwrap().drive.two_player);
    assert!(run.outcomes.last().unwrap().drive.two_player);
    // The pause (D6) is journalled as control events and records no tick: the
    // journal and the replay both have a gap-free tick sequence.
    let paused = run
        .loaded
        .control
        .iter()
        .any(|c| matches!(c.ev, ControlEvent::D6PauseBegin));
    assert!(paused, "the script pauses");
    let ticks: Vec<u64> = run.outcomes.iter().map(|o| o.t).collect();
    assert!(ticks.windows(2).all(|w| w[1] == w[0] + 1));
}

#[test]
fn the_cursor_replays_one_carl_a_solo_swap_ai_donut_and_two_players() {
    let dir = TempDir::new("cur-modes");
    type Script = fn(&mut Rig);
    let cases: [(&str, Script); 3] = [
        ("carl", |rig| solo_swap_script(rig, 600, &[])),
        ("swap", |rig| solo_swap_script(rig, 700, &[40, 400])),
        ("coop", |rig| coop_script(rig, 700)),
    ];
    for (name, script) in cases {
        let base = dir.0.join(name);
        std::fs::create_dir_all(&base).unwrap();
        let mut rig = Rig::new(solo_fixture(), Some(&base));
        script(&mut rig);
        let manifest = rig.finish().expect("finished");
        let session = base.join(manifest.session_id);
        let run = strict(&session).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_matches_journal(&run, &session);
        let drives: Vec<_> = run.outcomes.iter().map(|o| o.drive).collect();
        match name {
            "carl" => {
                assert!(drives.iter().all(|d| !d.two_player
                    && d.controlled == HeroId::Carl
                    && d.bots == [false, true]))
            }
            "swap" => {
                assert!(drives.iter().any(|d| d.controlled == HeroId::Carl));
                assert!(
                    drives
                        .iter()
                        .any(|d| d.controlled == HeroId::Donut && d.bots == [true, false]),
                    "after the swap Donut is the human and Carl the AI"
                );
            }
            _ => assert!(drives.iter().all(|d| d.two_player)),
        }
    }
}

use crate::party::HeroId;

#[test]
fn the_cursor_reports_defeat_restarts_and_demo_respawns() {
    let dir = TempDir::new("cur-demo");
    let mut rig = DemoRig::new(&dir.0, false, 4);
    demo_script(&mut rig, 2400);
    let manifest = rig.rec.finish("test").unwrap();
    let session = dir.0.join(manifest.session_id);
    let run = strict(&session).expect("replays");
    assert_matches_journal(&run, &session);
    // The demo's own defeat respawn happens inside the step; the Select-tap
    // respawn is a control record applied before it.
    assert!(
        run.outcomes.iter().any(|o| o.respawned_in_step),
        "a defeat respawn inside the step"
    );
    assert!(
        run.outcomes
            .iter()
            .any(|o| o.applied.contains(&ControlEvent::DemoRespawn) && o.lifecycle_reset()),
        "a Select-tap respawn control"
    );
    assert!(
        run.outcomes
            .iter()
            .filter(|o| o.lifecycle_reset())
            .all(|o| o.restarted
                || o.respawned_in_step
                || o.defeat_restart_in_step
                || !o.applied.is_empty())
    );
    // Every non-lifecycle tick reports no reset.
    assert!(run.outcomes.iter().any(|o| !o.lifecycle_reset()));
}

#[test]
fn the_cursor_replays_a_cf02_room_session() {
    use crate::companion::profile::ProfileSetId;
    use crate::party::{PROVISIONAL_TUNING, PartyState, PartyTuning, UncontrolledHero};
    let dir = TempDir::new("cur-cf02");
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
        rig.step(RawInput {
            move_x: if tick % 300 < 240 { 1.0 } else { 0.0 },
            swap: tick == 120 || tick == 180,
            attack: tick % 11 == 0,
            ..RawInput::NONE
        });
    }
    let manifest = rig.rec.finish("test").unwrap();
    let session = dir.0.join(manifest.session_id);
    let run = strict(&session).expect("replays");
    assert_matches_journal(&run, &session);
    assert!(run.outcomes.iter().any(|o| o.restarted));
    assert!(
        run.outcomes
            .iter()
            .any(|o| o.src.contains(&ControlSource::CompanionAi)),
        "the companion drove a hero"
    );
}

#[test]
fn the_cursor_replays_the_story_route_without_combat() {
    use crate::corridor::SceneSim;
    use crate::party::PartyState;
    let dir = TempDir::new("cur-story");
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
        kit_hash: super::live_tests::real_kit_hash(),
        start_hash: scene.state_hash(),
    };
    let mut manifest = super::live_tests::stamped(reset);
    manifest.mode = "solo_human".to_owned();
    let mut rec = live::LiveRecorder::start(&dir.0, manifest, &scene, None, &party, false).unwrap();
    party.pad_connected(crate::party::PadId(1));
    for tick in 0..400u64 {
        if tick == 200 {
            scene.reset();
            party.restart();
            rec.note_restart(&scene, None, &party);
        }
        rec.observe_party(&party);
        let slots = [
            RawInput {
                move_x: 1.0,
                jump: tick % 60 == 5,
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
    let run = strict(&session).expect("the story route replays");
    assert_matches_journal(&run, &session);
    assert_eq!(run.counters.observations, 0);
    assert!(run.outcomes.iter().any(|o| o.restarted), "the scene break");
}

#[test]
fn a_finished_or_failed_cursor_stays_put() {
    let (_dir, session) = recorded_solo("cur-stay", 150);
    let loaded = read_session(&session, true).unwrap();
    let (mut cursor, mut scene, mut combat, _t, _trust) =
        ReplayCursor::open(&loaded, &root(), ReplayPolicy::Strict).unwrap();
    assert_eq!(cursor.total_ticks(), 150);
    for _ in 0..150 {
        assert!(
            cursor
                .next_tick(&loaded, &mut scene, combat.as_mut())
                .unwrap()
                .is_some()
        );
    }
    assert_eq!(cursor.ticks_done(), 150);
    assert!(
        cursor
            .next_tick(&loaded, &mut scene, combat.as_mut())
            .unwrap()
            .is_none()
    );
    assert!(cursor.is_finished());
    assert!(
        cursor
            .next_tick(&loaded, &mut scene, combat.as_mut())
            .unwrap()
            .is_none(),
        "after the end it keeps returning None"
    );
    // A divergence is permanent: the cursor does not advance past a failure.
    tamper_ticks(&session, |ticks| ticks[20].post ^= 1);
    let loaded = read_session(&session, true).unwrap();
    let (mut cursor, mut scene, mut combat, _t, _trust) =
        ReplayCursor::open(&loaded, &root(), ReplayPolicy::Strict).unwrap();
    let failure = loop {
        match cursor.next_tick(&loaded, &mut scene, combat.as_mut()) {
            Ok(Some(_)) => {}
            Ok(None) => panic!("the tampered tick must fail"),
            Err(failure) => break failure,
        }
    };
    assert_eq!((failure.code, failure.tick), ("post_state", Some(20)));
    assert_eq!(cursor.ticks_done(), 20, "ticks before the failure matched");
    assert!(
        cursor
            .next_tick(&loaded, &mut scene, combat.as_mut())
            .unwrap()
            .is_none()
    );
}

// ------------------------------------------------------ cross-build (R0)

#[test]
fn checked_passes_an_untampered_cross_build_session_and_strict_refuses_it() {
    let (_dir, session) = recorded_solo("cur-xbuild", 400);
    let same = strict(&session).expect("same build is strict");
    assert_eq!(same.trust, Trust::Strict);
    // A matching fingerprint under the checked policy is still STRICT: checked
    // mode is never a downgrade.
    let still_strict = checked(&session).expect("checked accepts a matching source");
    assert_eq!(still_strict.trust, Trust::Strict);

    make_cross_build(&session);
    let run = checked(&session).expect("a cross-build session is checked");
    assert_eq!(run.trust, Trust::CheckedCrossBuild);
    assert_eq!(run.trust.result_word(), "CHECKED");
    assert_eq!(run.counters, same.counters, "the same checks, tick by tick");
    assert_eq!(run.post, same.post);
    assert_eq!(
        strict(&session).err().map(|f| f.code),
        Some("build_mismatch")
    );
    // The verifier and its output are strict-only: it refuses the same session.
    let failure = verify_dir(&session, &root()).unwrap_err();
    assert_eq!(failure.code, "build_mismatch");
}

#[test]
fn an_empty_fingerprint_is_rejected_in_both_modes() {
    let (_dir, session) = recorded_solo("cur-empty", 120);
    tamper_manifest(&session, |m| m.source_fingerprint.clear());
    assert_eq!(
        strict(&session).err().map(|f| f.code),
        Some("build_mismatch")
    );
    assert_eq!(
        checked(&session).err().map(|f| f.code),
        Some("build_mismatch")
    );
}

/// One tamper of a baseline session, by the name of the check it must trip.
struct Case {
    name: &'static str,
    code: &'static str,
    tick: Option<u64>,
    apply: fn(&Path),
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "input",
            code: "slot_routing",
            tick: Some(30),
            apply: |s| {
                tamper_ticks(s, |ticks| {
                    // The slot no longer routes to the recorded hero input.
                    ticks[30].slots[0].1 ^= 2;
                });
            },
        },
        Case {
            name: "control",
            code: "",
            tick: None,
            apply: |s| {
                tamper_control(s, |control| {
                    control.retain(|c| c.ev != ControlEvent::Restart);
                    for (index, record) in control.iter_mut().enumerate() {
                        record.seq = index as u64;
                    }
                });
            },
        },
        Case {
            name: "events",
            code: "events",
            tick: None,
            apply: |s| {
                tamper_ticks(s, |ticks| {
                    let at = ticks.iter().position(|t| !t.ev.is_empty()).unwrap();
                    ticks[at].ev[0].d.push('!');
                });
            },
        },
        Case {
            name: "eh",
            code: "event_hash",
            tick: Some(300),
            apply: |s| tamper_ticks(s, |ticks| ticks[300].eh ^= 1),
        },
        Case {
            name: "eff",
            code: "effective_input",
            tick: Some(120),
            apply: |s| {
                tamper_ticks(s, |ticks| {
                    // The AI hero's live effective input (Donut, driven by the sim).
                    ticks[120].eff.as_mut().unwrap()[1].0 = (0.5_f32).to_bits();
                });
            },
        },
        Case {
            name: "labels",
            code: "labels",
            tick: Some(0),
            apply: |s| {
                tamper_ticks(s, |ticks| {
                    for tick in ticks.iter_mut() {
                        tick.src = [ControlSource::HumanP1, ControlSource::HumanP2];
                    }
                });
            },
        },
        Case {
            name: "obs",
            code: "observation",
            tick: Some(77),
            apply: |s| {
                tamper_ticks(s, |ticks| {
                    let obs = ticks[77].obs.as_mut().unwrap();
                    let mut bits = decode_obs_bits(&obs[1]).unwrap();
                    bits[0] ^= 0x0040_0000;
                    obs[1] = encode_obs_bits(&bits);
                });
            },
        },
        Case {
            name: "diag",
            code: "diagnostics",
            tick: Some(90),
            apply: |s| tamper_diag(s, |diag| diag[90].hp = [1, 1]),
        },
        Case {
            name: "pre",
            code: "pre_state",
            tick: Some(200),
            apply: |s| tamper_ticks(s, |ticks| ticks[200].pre ^= 1),
        },
        Case {
            name: "post",
            code: "post_state",
            tick: Some(200),
            apply: |s| tamper_ticks(s, |ticks| ticks[200].post ^= 1),
        },
        Case {
            name: "content",
            code: "content_mismatch",
            tick: None,
            apply: |s| {
                tamper_manifest(s, |m| {
                    m.content_hashes
                        .insert("mana_tuning".to_owned(), "00000000000000aa".to_owned());
                });
            },
        },
        Case {
            name: "rules",
            code: "rules_mismatch",
            tick: None,
            apply: |s| tamper_manifest(s, |m| m.game_ruleset_id = "floor02".to_owned()),
        },
        Case {
            name: "incomplete",
            code: "capture_incomplete",
            tick: None,
            apply: |s| {
                tamper_manifest(s, |m| {
                    m.status = SessionStatus::Incomplete;
                    m.dropped_control = 1;
                });
            },
        },
    ]
}

#[test]
fn every_tamper_case_fails_checked_mode_with_the_same_code_and_tick_as_strict() {
    let (dir, baseline) = recorded_solo("cur-cases", 400);
    for case in cases() {
        let same = dir.0.join(format!("{}-same", case.name));
        let cross = dir.0.join(format!("{}-cross", case.name));
        copy_dir(&baseline, &same);
        copy_dir(&baseline, &cross);
        (case.apply)(&same);
        (case.apply)(&cross);
        make_cross_build(&cross);

        let reference = strict(&same)
            .err()
            .unwrap_or_else(|| panic!("{}: the tamper was not noticed", case.name));
        if !case.code.is_empty() {
            assert_eq!(reference.code, case.code, "{}: {reference}", case.name);
        }
        if case.tick.is_some() {
            assert_eq!(reference.tick, case.tick, "{}: {reference}", case.name);
        }
        // Checked mode on the same-source copy (STRICT trust) fails identically.
        let on_same = checked(&same).err().expect(case.name);
        assert_eq!(
            (on_same.code, on_same.tick),
            (reference.code, reference.tick),
            "{}",
            case.name
        );
        // And with the fingerprint rewritten (the cross-build route) it fails
        // with the same code at the same tick: no tamper hides behind the
        // waived predicate.
        let on_cross = checked(&cross).err().expect(case.name);
        assert_eq!(
            (on_cross.code, on_cross.tick),
            (reference.code, reference.tick),
            "{}: {on_cross}",
            case.name
        );
        // Strict still refuses the cross-build copy: `build_mismatch`, except
        // for a record that is incomplete before any build question arises.
        let strict_cross = strict(&cross).err().expect(case.name);
        let expected = if case.name == "incomplete" {
            "capture_incomplete"
        } else {
            "build_mismatch"
        };
        assert_eq!(strict_cross.code, expected, "{}", case.name);
    }
}

#[test]
fn a_tampered_ai_hero_effective_input_fails_a_checked_replay_at_its_tick() {
    // The companion profile is not in the content hashes (design 3-B): only
    // the per-tick effective-input comparison can notice an AI that decided
    // differently from the recorded one.
    let (_dir, session) = recorded_solo("cur-ai-eff", 400);
    make_cross_build(&session);
    tamper_ticks(&session, |ticks| {
        let eff = ticks[150].eff.as_mut().expect("a live party tick");
        // Flip one bit of the AI hero's (Donut's) recorded effective input.
        eff[1].1 ^= 1;
    });
    let failure = checked(&session)
        .err()
        .expect("the AI divergence is caught");
    assert_eq!(failure.code, "effective_input");
    assert_eq!(failure.tick, Some(150));
}

#[test]
fn the_replay_result_line_is_never_a_verify_line() {
    let results = [
        ReplayResult::Strict,
        ReplayResult::Checked,
        ReplayResult::Diverged {
            code: "post_state".to_owned(),
            tick: Some(42),
        },
        ReplayResult::Unsupported {
            code: "build_mismatch".to_owned(),
        },
    ];
    for result in &results {
        let line = replay_result_line("20260101-000000-ab12", result, 7, Some(0xdead_beef));
        assert!(line.starts_with("PLAYTRACE_REPLAY session="), "{line}");
        assert!(!line.starts_with("PLAYTRACE_VERIFY"), "{line}");
        assert!(!line.contains("PLAYTRACE_VERIFY"), "{line}");
        assert!(!line.contains("scope=full"), "{line}");
        assert!(line.contains("ticks=7 post=00000000deadbeef"), "{line}");
    }
    let line = replay_result_line("s", &results[2], 41, Some(1));
    assert!(line.ends_with("code=post_state tick=42"), "{line}");
    assert_eq!(
        replay_result_line("s", &ReplayResult::of_trust(Trust::Strict), 3, None),
        "PLAYTRACE_REPLAY session=s result=STRICT ticks=3 post=none"
    );
    // The checked policy and the strict fingerprint are one constant apart.
    assert_ne!(SOURCE_FINGERPRINT, "0123456789abcdef");
}
