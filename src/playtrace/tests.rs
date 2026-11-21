use std::path::{Path, PathBuf};

use super::fake::FakeParty;
use super::*;

/// A fresh empty directory under the system temp dir, removed on drop.
pub(super) struct TempDir(PathBuf);

impl TempDir {
    pub(super) fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("playtrace-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        Self(dir)
    }
    pub(super) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(super) fn reset() -> ResetDescriptor {
    ResetDescriptor {
        source: "fake".to_owned(),
        name: "fake_party".to_owned(),
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
        kit_hash: 0,
        start_hash: 0xcbf2_9ce4_8422_2325,
    }
}

/// CARL-PROG-01 CP-Q09: a descriptor with the three story flags false
/// serializes exactly as it did before they existed, an older recording without
/// the fields loads as false, and a set flag round-trips.
#[test]
fn the_story_flags_are_written_only_when_set_and_old_descriptors_load_as_false() {
    let text = serde_json::to_string(&reset()).unwrap();
    for key in ["carl_prog", "carl_dynamite", "carl_gauntlet"] {
        assert!(
            !text.contains(key),
            "{key} is not written while false: {text}"
        );
    }
    let back: ResetDescriptor = serde_json::from_str(&text).unwrap();
    assert_eq!(back, reset());
    let mut set = reset();
    set.carl_dynamite = true;
    let text = serde_json::to_string(&set).unwrap();
    assert!(text.contains("\"carl_dynamite\":true"));
    assert!(!text.contains("carl_prog"));
    assert_eq!(serde_json::from_str::<ResetDescriptor>(&text).unwrap(), set);
}

pub(super) fn manifest() -> SessionManifest {
    SessionManifest::new(generate_session_id(), reset())
}

/// Drive `ticks` ticks of the fake party into `writer` with retry control
/// events, like the live adapter does.
pub(super) fn drive(writer: &mut SessionWriter, party: &mut FakeParty, ticks: u64) {
    if party.t == 0 {
        writer
            .begin_episode(EpisodeBoundary {
                episode: 0,
                reason: EpisodeReason::Start,
                stage: 0,
            })
            .unwrap();
    }
    for _ in 0..ticks {
        if party.retry_due() {
            writer.record_control(ControlEvent::Restart).unwrap();
            writer
                .begin_episode(EpisodeBoundary {
                    episode: party.episode + 1,
                    reason: EpisodeReason::Restart,
                    stage: 0,
                })
                .unwrap();
        }
        let tick = party.tick();
        writer.record_tick(&tick).unwrap();
    }
}

pub(super) fn finished_session(base: &Path, seed: u64, ticks: u64) -> (PathBuf, FakeParty) {
    let mut writer = SessionWriter::create(base, manifest()).unwrap();
    let mut party = FakeParty::new(seed, 500);
    drive(&mut writer, &mut party, ticks);
    writer.finish(SessionStatus::Completed).unwrap();
    (writer.dir().to_path_buf(), party)
}

#[test]
fn actions_round_trip_bit_exact() {
    let floats = [
        0.0_f32,
        -0.0,
        1.0,
        -1.0,
        0.18,
        0.123_456_79,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::NAN,
        f32::from_bits(0x7fc0_1234),
        f32::INFINITY,
    ];
    for x in floats {
        for buttons in 0..128_u8 {
            let bits = ActionBits(x.to_bits(), buttons);
            let raw = bits.to_raw();
            assert_eq!(ActionBits::from_raw(raw), bits);
            let json = serde_json::to_string(&bits).unwrap();
            assert_eq!(serde_json::from_str::<ActionBits>(&json).unwrap(), bits);
        }
    }
    // The seven button bits map to the seven RawInput fields and nothing else.
    let all = crate::input::RawInput {
        move_x: 0.5,
        jump: true,
        attack: true,
        physical: true,
        ability: true,
        pause: true,
        swap: true,
        down: true,
    };
    assert_eq!(ActionBits::from_raw(all).1, 0b111_1111);
    // `rawinput-bits/1` records (no down bit) still decode, with Down released.
    let v1 = ActionBits(0x3f80_0000, 0b11_1111).to_raw();
    assert!(!v1.down && v1.swap && v1.jump);
    assert_eq!(super::codec::ACTION_SCHEMA, "rawinput-bits/2");
    assert_eq!(super::codec::ACTION_SCHEMA_V1, "rawinput-bits/1");
    assert_eq!(
        ActionBits::from_raw(crate::input::RawInput::NONE),
        ActionBits::NONE
    );
}

#[test]
fn round_trip_across_chunks_with_retries() {
    let dir = TempDir::new("roundtrip");
    let (session, _) = finished_session(dir.path(), 7, 1300);
    let loaded = read_session(&session, false).unwrap();
    assert_eq!(loaded.ticks.len(), 1300);
    assert_eq!(loaded.chunks.len(), 3);
    assert_eq!(
        loaded.chunks.iter().map(|c| c.ticks).collect::<Vec<_>>(),
        [600, 600, 100]
    );
    assert!(loaded.chunks.iter().all(|c| c.closed));
    assert!(!loaded.recovered && loaded.gaps.is_empty());
    for (index, tick) in loaded.ticks.iter().enumerate() {
        assert_eq!(tick.t, index as u64);
    }
    // The fake retries every 500 ticks: three episodes, reset `et`.
    assert_eq!(loaded.ticks.last().unwrap().ep, 2);
    assert_eq!(loaded.ticks[499].et, 500);
    assert_eq!(loaded.ticks[500].et, 1);
    let manifest = &loaded.manifest;
    assert_eq!(manifest.status, SessionStatus::Completed);
    assert!(manifest.capture_complete);
    assert_eq!(manifest.tick_count, 1300);
    assert_eq!(manifest.last_tick, Some(1299));
    assert_eq!(manifest.chunk_checksums.len(), 3);
    // Control events are ordered and carry the tick boundary.
    let seqs: Vec<u64> = loaded.control.iter().map(|c| c.seq).collect();
    assert!(seqs.windows(2).all(|w| w[1] == w[0] + 1));
    let restarts: Vec<u64> = loaded
        .control
        .iter()
        .filter(|c| c.ev == ControlEvent::Restart)
        .map(|c| c.at_t)
        .collect();
    assert_eq!(restarts, [500, 1000]);
    assert!(session.join("ticks-000002.jsonl").exists());
}

#[test]
fn same_seed_double_run_is_byte_identical() {
    let a = TempDir::new("seed-a");
    let b = TempDir::new("seed-b");
    let (da, _) = finished_session(a.path(), 42, 700);
    let (db, _) = finished_session(b.path(), 42, 700);
    for name in ["ticks-000000.jsonl", "ticks-000001.jsonl", "control.jsonl"] {
        assert_eq!(
            std::fs::read(da.join(name)).unwrap(),
            std::fs::read(db.join(name)).unwrap()
        );
    }
    let (dc, _) = finished_session(a.path(), 43, 700);
    assert_ne!(
        std::fs::read(da.join("ticks-000000.jsonl")).unwrap(),
        std::fs::read(dc.join("ticks-000000.jsonl")).unwrap()
    );
}

#[test]
fn producer_logic_is_identical_with_and_without_a_recorder() {
    let dir = TempDir::new("optional");
    let mut with = FakeParty::new(9, 300);
    let mut without = FakeParty::new(9, 300);
    let mut writer = SessionWriter::create(dir.path(), manifest()).unwrap();
    drive(&mut writer, &mut with, 900);
    let mut plain = Vec::new();
    for _ in 0..900 {
        plain.push(without.tick());
    }
    assert_eq!(with.digest(), without.digest());
    writer.finish(SessionStatus::Completed).unwrap();
    let loaded = read_session(writer.dir(), false).unwrap();
    assert_eq!(loaded.ticks, plain);
}

#[test]
fn two_slots_are_independent_and_edges_are_not_synthesized() {
    let mut party = FakeParty::new(5, 0);
    let ticks: Vec<TickRecord> = (0..400).map(|_| party.tick()).collect();
    assert!(ticks.iter().any(|t| t.slots[0] != t.slots[1]));
    assert!(
        ticks
            .iter()
            .any(|t| t.slots[0].1 & 1 != 0 && t.slots[1].1 & 1 == 0)
    );
    // A button stays exactly as produced: re-encoding never flips a bit.
    for tick in &ticks {
        for slot in tick.slots {
            assert_eq!(ActionBits::from_raw(slot.to_raw()), slot);
        }
        assert_eq!(tick.slots, tick.hero);
    }
}

#[test]
fn writer_rejects_duplicate_skipped_and_reordered_ticks() {
    let dir = TempDir::new("order");
    let mut writer = SessionWriter::create(dir.path(), manifest()).unwrap();
    let mut party = FakeParty::new(1, 0);
    let first = party.tick();
    let second = party.tick();
    let third = party.tick();
    writer.record_tick(&first).unwrap();
    assert!(matches!(
        writer.record_tick(&first),
        Err(PlaytraceError::NonContiguousTick {
            expected: 1,
            found: 0
        })
    ));
    assert!(matches!(
        writer.record_tick(&third),
        Err(PlaytraceError::NonContiguousTick {
            expected: 1,
            found: 2
        })
    ));
    writer.record_tick(&second).unwrap();
    writer.record_tick(&third).unwrap();
    writer.finish(SessionStatus::Completed).unwrap();
    assert!(matches!(
        writer.record_tick(&third),
        Err(PlaytraceError::Finished)
    ));
    assert!(matches!(
        writer.finish(SessionStatus::Completed),
        Err(PlaytraceError::Finished)
    ));
}

#[test]
fn reader_rejects_a_duplicate_tick_in_a_chunk() {
    let dir = TempDir::new("dup");
    let (session, _) = finished_session(dir.path(), 3, 10);
    let path = session.join("ticks-000000.jsonl");
    let text = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    // Repeat tick 3 and re-seal the chunk so only the ordering is wrong.
    let mut fixed: Vec<&str> = lines[..4].to_vec();
    fixed.push(lines[3]);
    fixed.extend(&lines[4..10]);
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut body = String::new();
    for line in &fixed {
        body.push_str(line);
        body.push('\n');
    }
    for byte in body.as_bytes() {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    let trailer = ChunkTrailer {
        end: true,
        chunk: 0,
        ticks: 11,
        first_t: Some(0),
        last_t: Some(9),
        fnv: hash,
    };
    body.push_str(&serde_json::to_string(&trailer).unwrap());
    body.push('\n');
    std::fs::write(&path, body).unwrap();
    assert!(matches!(
        read_session(&session, true),
        Err(PlaytraceError::NonContiguousTick { .. })
    ));
}

#[test]
fn completed_and_abandoned_manifests() {
    let dir = TempDir::new("status");
    let (done, _) = finished_session(dir.path(), 1, 50);
    assert_eq!(
        read_session(&done, false).unwrap().manifest.status,
        SessionStatus::Completed
    );
    // Dropped without `finish`: a clean abandon, not a crash.
    let abandoned_dir = {
        let mut writer = SessionWriter::create(dir.path(), manifest()).unwrap();
        let mut party = FakeParty::new(2, 0);
        drive(&mut writer, &mut party, 30);
        writer.dir().to_path_buf()
    };
    let loaded = read_session(&abandoned_dir, false).unwrap();
    assert_eq!(loaded.manifest.status, SessionStatus::Abandoned);
    assert_eq!(loaded.ticks.len(), 30);
    assert!(!loaded.recovered);
}

#[test]
fn empty_session_is_valid() {
    let dir = TempDir::new("empty");
    let mut writer = SessionWriter::create(dir.path(), manifest()).unwrap();
    writer.finish(SessionStatus::Completed).unwrap();
    let loaded = read_session(writer.dir(), false).unwrap();
    assert!(loaded.ticks.is_empty() && loaded.chunks.is_empty() && loaded.control.is_empty());
    assert_eq!(loaded.manifest.tick_count, 0);
    assert_eq!(loaded.manifest.last_tick, None);
}

#[test]
fn unwritable_directory_is_an_error_not_a_panic() {
    let dir = TempDir::new("unwritable");
    // A regular file where the base directory should be.
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, b"not a directory").unwrap();
    assert!(matches!(
        SessionWriter::create(&blocker, manifest()),
        Err(PlaytraceError::Io(_))
    ));
    assert!(matches!(
        SessionWriter::create(&blocker.join("below"), manifest()),
        Err(PlaytraceError::Io(_))
    ));
    // An existing session id is never reused.
    let m = manifest();
    let id = m.session_id.clone();
    let _first = SessionWriter::create(dir.path(), m).unwrap();
    let again = SessionManifest::new(id, reset());
    assert!(matches!(
        SessionWriter::create(dir.path(), again),
        Err(PlaytraceError::Exists(_))
    ));
}

#[test]
fn full_queue_drops_loudly_and_bounds_memory() {
    let dir = TempDir::new("queue");
    let mut writer = SessionWriter::create(dir.path(), manifest()).unwrap();
    let mut party = FakeParty::new(4, 0);
    writer.fail_writes = u32::MAX;
    for _ in 0..650 {
        let tick = party.tick();
        writer.record_tick(&tick).unwrap();
        // 600 ticks plus at most one trailer.
        assert!(writer.health().queued <= QUEUE_BOUND + 1);
    }
    let health = writer.health();
    assert_eq!(health.dropped_ticks, 50);
    assert!(health.write_errors > 0 && health.incomplete());
    // The disk comes back: the queue drains, later ticks are written.
    writer.fail_writes = 0;
    for _ in 0..20 {
        let tick = party.tick();
        writer.record_tick(&tick).unwrap();
    }
    let manifest = writer.finish(SessionStatus::Completed).unwrap();
    assert_eq!(manifest.status, SessionStatus::Incomplete);
    assert!(!manifest.capture_complete && !manifest.training_eligible);
    assert!(
        manifest
            .quality_flags
            .contains(&"capture_incomplete".to_owned())
    );
    assert!(manifest.dropped_ticks >= 50);
    // The missing ticks are visible, never silent.
    assert!(matches!(
        read_session(writer.dir(), false),
        Err(PlaytraceError::TickGap { .. })
    ));
    let loaded = read_session(writer.dir(), true).unwrap();
    assert_eq!(loaded.gaps.len(), 1);
    assert_eq!(loaded.gaps[0].from, 600);
    assert_eq!(loaded.ticks.len() as u64, manifest.tick_count);
}

#[test]
fn a_transient_write_error_marks_incomplete_but_loses_nothing() {
    let dir = TempDir::new("transient");
    let mut writer = SessionWriter::create(dir.path(), manifest()).unwrap();
    let mut party = FakeParty::new(8, 0);
    for index in 0..100 {
        if index == 40 {
            writer.fail_writes = 3;
        }
        let tick = party.tick();
        writer.record_tick(&tick).unwrap();
    }
    let manifest = writer.finish(SessionStatus::Completed).unwrap();
    assert_eq!(manifest.status, SessionStatus::Incomplete);
    assert_eq!(manifest.dropped_ticks, 0);
    assert!(manifest.write_errors >= 1);
    let loaded = read_session(writer.dir(), false).unwrap();
    assert_eq!(loaded.ticks.len(), 100);
}

#[test]
fn killed_writer_recovers_the_valid_prefix() {
    let dir = TempDir::new("crash");
    let mut writer = SessionWriter::create(dir.path(), manifest()).unwrap();
    let mut party = FakeParty::new(6, 0);
    drive(&mut writer, &mut party, 650);
    let session = writer.dir().to_path_buf();
    // SIGKILL: no finish, no drop handler, and a half-written last line.
    writer.crash_for_test();
    {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(session.join("ticks-000001.jsonl"))
            .unwrap();
        file.write_all(b"{\"t\":650,\"ep\":0,\"et\":6").unwrap();
    }
    let loaded = read_session(&session, false).unwrap();
    assert!(loaded.recovered);
    assert_eq!(loaded.manifest.status, SessionStatus::CrashedRecovered);
    assert_eq!(loaded.manifest.terminal.as_deref(), Some("truncated"));
    assert_eq!(loaded.ticks.len(), 650);
    assert_eq!(loaded.partial_lines_dropped, 1);
    assert_eq!(loaded.chunks.last().map(|c| c.closed), Some(false));
    // Sealing rewrites the directory so it validates as a normal session.
    let sealed = recover_session(&session).unwrap();
    assert_eq!(sealed.status, SessionStatus::CrashedRecovered);
    let again = read_session(&session, false).unwrap();
    assert_eq!(again.ticks, loaded.ticks);
    assert_eq!(again.partial_lines_dropped, 0);
    assert!(again.chunks.iter().all(|c| c.closed));
    assert_eq!(again.manifest.terminal.as_deref(), Some("truncated"));
}

#[test]
fn a_live_session_is_locked_and_cannot_be_recovered_or_mistaken_for_a_crash() {
    let dir = TempDir::new("lock");
    let mut writer = SessionWriter::create(dir.path(), manifest()).unwrap();
    let mut party = FakeParty::new(21, 0);
    drive(&mut writer, &mut party, 50);
    let session = writer.dir().to_path_buf();
    assert!(session_is_live(&session));
    assert!(matches!(
        recover_session(&session),
        Err(PlaytraceError::SessionLive)
    ));
    let loaded = read_session(&session, false).unwrap();
    assert!(loaded.live && loaded.recovered);
    // The refused recovery changed nothing: the writer carries on and the
    // session finishes as a normal one.
    drive(&mut writer, &mut party, 50);
    writer.finish(SessionStatus::Completed).unwrap();
    assert!(!session_is_live(&session));
    let done = read_session(&session, false).unwrap();
    assert_eq!(done.ticks.len(), 100);
    assert!(!done.live && !done.recovered);
    // A crashed writer holds nothing: recovery is allowed.
    let mut crashed = SessionWriter::create(dir.path(), manifest()).unwrap();
    drive(&mut crashed, &mut FakeParty::new(22, 0), 10);
    let path = crashed.dir().to_path_buf();
    crashed.crash_for_test();
    assert!(!session_is_live(&path));
    assert!(recover_session(&path).is_ok());
}

#[test]
fn checksum_corruption_is_rejected() {
    let dir = TempDir::new("corrupt");
    let (session, _) = finished_session(dir.path(), 11, 700);
    let path = session.join("ticks-000000.jsonl");
    let mut bytes = std::fs::read(&path).unwrap();
    // Flip one digit inside a tick line (still valid JSON).
    let at = bytes.windows(5).position(|w| w == b"\"et\":").unwrap() + 5;
    bytes[at] = if bytes[at] == b'9' {
        b'1'
    } else {
        bytes[at] + 1
    };
    std::fs::write(&path, bytes).unwrap();
    assert!(matches!(
        read_session(&session, true),
        Err(PlaytraceError::ChecksumMismatch { chunk: 0 })
    ));
}

#[test]
fn missing_chunk_trailer_and_manifest_mismatch_are_rejected() {
    let dir = TempDir::new("structure");
    let (session, _) = finished_session(dir.path(), 12, 1300);
    // A hole in the chunk numbering.
    let backup = std::fs::read(session.join("ticks-000001.jsonl")).unwrap();
    std::fs::remove_file(session.join("ticks-000001.jsonl")).unwrap();
    assert!(matches!(
        read_session(&session, true),
        Err(PlaytraceError::MissingChunk { index: 1 })
    ));
    std::fs::write(session.join("ticks-000001.jsonl"), &backup).unwrap();
    // A closed chunk that lost its trailer.
    let text = String::from_utf8(backup.clone()).unwrap();
    let without: String = text.lines().take(600).map(|l| format!("{l}\n")).collect();
    std::fs::write(session.join("ticks-000001.jsonl"), without).unwrap();
    assert!(matches!(
        read_session(&session, true),
        Err(PlaytraceError::MissingTrailer { chunk: 1 })
    ));
    std::fs::write(session.join("ticks-000001.jsonl"), &backup).unwrap();
    assert!(read_session(&session, false).is_ok());
    // A manifest that disagrees with the chunks.
    let manifest_path = session.join("manifest.json");
    let text = std::fs::read_to_string(&manifest_path).unwrap();
    std::fs::write(
        &manifest_path,
        text.replace("\"tick_count\": 1300", "\"tick_count\": 1299"),
    )
    .unwrap();
    assert!(matches!(
        read_session(&session, false),
        Err(PlaytraceError::ManifestMismatch(_))
    ));
}

#[test]
fn schema_mismatch_is_strict() {
    let dir = TempDir::new("schema");
    let (session, _) = finished_session(dir.path(), 13, 5);
    let path = session.join("manifest.json");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        text.replace("playtrace-session/1", "playtrace-session/2"),
    )
    .unwrap();
    assert!(matches!(
        read_session(&session, false),
        Err(PlaytraceError::SchemaMismatch { .. })
    ));
    std::fs::remove_file(&path).unwrap();
    assert!(matches!(
        read_session(&session, false),
        Err(PlaytraceError::MissingManifest)
    ));
}

#[test]
fn concurrent_independent_recordings_do_not_mix() {
    let dir = TempDir::new("concurrent");
    let mut a = SessionWriter::create(dir.path(), manifest()).unwrap();
    let mut b = SessionWriter::create(dir.path(), manifest()).unwrap();
    assert_ne!(a.session_id(), b.session_id());
    let mut pa = FakeParty::new(100, 0);
    let mut pb = FakeParty::new(200, 0);
    for _ in 0..5 {
        drive(&mut a, &mut pa, 130);
        drive(&mut b, &mut pb, 130);
    }
    a.finish(SessionStatus::Completed).unwrap();
    b.finish(SessionStatus::Completed).unwrap();
    let la = read_session(a.dir(), false).unwrap();
    let lb = read_session(b.dir(), false).unwrap();
    assert_eq!(la.ticks.len(), 650);
    assert_eq!(lb.ticks.len(), 650);
    assert_ne!(la.ticks[0], lb.ticks[0]);
    let listed = list_sessions(dir.path());
    assert_eq!(listed.len(), 2);
}

#[test]
fn manifest_has_no_machine_or_user_data() {
    let dir = TempDir::new("privacy");
    let (session, _) = finished_session(dir.path(), 14, 20);
    let text = std::fs::read_to_string(session.join("manifest.json")).unwrap();
    let base = dir.path().to_string_lossy().into_owned();
    assert!(!text.contains(&base), "no absolute path");
    for key in ["HOME", "USER", "LOGNAME", "HOSTNAME"] {
        if let Ok(value) = std::env::var(key)
            && value.len() > 2
        {
            assert!(!text.contains(&value), "{key} leaked into the manifest");
        }
    }
    assert!(!text.contains("PadId") && !text.contains("serial"));
    let manifest: SessionManifest = serde_json::from_str(&text).unwrap();
    assert_eq!(manifest.participants, Participants::OwnerOnly);
    assert_eq!(manifest.schema, SESSION_SCHEMA);
    assert_eq!(manifest.snapshot_support, "reset_only");
    assert!(manifest.observation_is_partial && !manifest.training_eligible);
    let day = manifest.started_on_utc_day.unwrap();
    assert_eq!(day.len(), 10);
    assert_eq!(&day[4..5], "-");
}

#[test]
fn session_ids_are_128_bit_hex_and_unique() {
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..2000 {
        let id = generate_session_id();
        assert_eq!(id.len(), 32);
        assert!(id.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(seen.insert(id));
    }
}

#[test]
fn storage_paths_follow_xdg_with_a_home_fallback() {
    assert_eq!(
        default_record_dir(Some("/data"), Some("/home/x")).unwrap(),
        PathBuf::from("/data/dungeon-floorbreak/playtrace")
    );
    assert_eq!(
        default_record_dir(None, Some("/home/x")).unwrap(),
        PathBuf::from("/home/x/.local/share/dungeon-floorbreak/playtrace")
    );
    // A relative or empty XDG value is ignored (the spec requires absolute).
    assert_eq!(
        default_record_dir(Some("rel"), Some("/home/x")).unwrap(),
        PathBuf::from("/home/x/.local/share/dungeon-floorbreak/playtrace")
    );
    assert_eq!(
        default_record_dir(Some(""), Some("/home/x")).unwrap(),
        PathBuf::from("/home/x/.local/share/dungeon-floorbreak/playtrace")
    );
    assert!(matches!(
        default_record_dir(None, None),
        Err(PlaytraceError::NoDataDir)
    ));
    assert_eq!(WARN_BYTES, 5 * 1024 * 1024 * 1024);
    let dir = TempDir::new("size");
    std::fs::write(dir.path().join("a"), vec![0_u8; 1000]).unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub/b"), vec![0_u8; 234]).unwrap();
    assert_eq!(dir_size_bytes(dir.path()), 1234);
    assert_eq!(dir_size_bytes(&dir.path().join("missing")), 0);
}

#[test]
fn golden_encoded_samples_are_stable() {
    // The exact text of one tick and one control line: a format change must be
    // a deliberate schema bump.
    let tick = TickRecord {
        t: 7,
        ep: 1,
        et: 3,
        stage: 2,
        pre: 0x00ff,
        slots: [ActionBits(0x3f80_0000, 0b101), ActionBits::NONE],
        hero: [ActionBits(0x3f80_0000, 0b101), ActionBits::NONE],
        src: [ControlSource::HumanP1, ControlSource::ScriptedBot],
        ev: vec![EventRecord {
            i: 4,
            k: "Hit".to_owned(),
            d: "Hit { tick: 3 }".to_owned(),
        }],
        post: 0xabcd,
        eh: 1,
        ph: 2,
        obs: None,
        eff: None,
    };
    assert_eq!(
        serde_json::to_string(&tick).unwrap(),
        concat!(
            r#"{"t":7,"ep":1,"et":3,"stage":2,"pre":"00000000000000ff","#,
            r#""slots":[[1065353216,5],[0,0]],"hero":[[1065353216,5],[0,0]],"#,
            r#""src":["human_p1","scripted_bot"],"#,
            r#""ev":[{"i":4,"k":"Hit","d":"Hit { tick: 3 }"}],"#,
            r#""post":"000000000000abcd","eh":"0000000000000001","ph":"0000000000000002"}"#
        )
    );
    let record = ControlRecord {
        seq: 3,
        at_t: 8,
        ev: ControlEvent::DriveChange {
            live: true,
            two_player: false,
            controlled: 1,
            bots: [true, false],
        },
    };
    assert_eq!(
        serde_json::to_string(&record).unwrap(),
        r#"{"seq":3,"at_t":8,"ev":{"k":"drive_change","live":true,"two_player":false,"controlled":1,"bots":[true,false]}}"#
    );
}
