//! PT-C robustness follow-ups of the PT-B review (items 2, 3, 5 and 6 of its
//! FOLLOW-UP list; item 1, the build identity, is in `build_info`).

use std::path::Path;

use super::fake::FakeParty;
use super::tests::{TempDir, drive, finished_session, manifest};
use super::*;

fn manifest_text(session: &Path) -> String {
    std::fs::read_to_string(session.join("manifest.json")).unwrap()
}

/// Rewrite `control.jsonl` with `edit` and bring the manifest's integrity
/// record back in line with the new file, so only the *semantic* checks
/// (seq contiguity, at_t order) can reject it.
fn rewrite_control(session: &Path, edit: impl FnOnce(Vec<String>) -> Vec<String>) {
    let text = std::fs::read_to_string(session.join("control.jsonl")).unwrap();
    let lines = edit(text.lines().map(str::to_owned).collect());
    let mut out = String::new();
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for line in &lines {
        out.push_str(line);
        out.push('\n');
        for byte in line.bytes().chain(std::iter::once(b'\n')) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    std::fs::write(session.join("control.jsonl"), out).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&manifest_text(session)).unwrap();
    value["control_integrity"] = serde_json::json!({
        "lines": lines.len(),
        "fnv": format!("{hash:016x}"),
    });
    std::fs::write(
        session.join("manifest.json"),
        serde_json::to_string_pretty(&value).unwrap(),
    )
    .unwrap();
}

fn is_integrity_error(result: Result<LoadedSession, PlaytraceError>) -> bool {
    matches!(result, Err(PlaytraceError::StreamIntegrity { .. }))
}

#[test]
fn the_final_manifest_records_the_control_line_count_and_checksum() {
    let dir = TempDir::new("ctl-manifest");
    let (session, _) = finished_session(dir.path(), 21, 1300);
    let loaded = read_session(&session, false).unwrap();
    let integrity = loaded.manifest.control_integrity.expect("recorded");
    assert_eq!(integrity.lines, loaded.control.len() as u64);
    assert!(integrity.lines > 2, "retries produced control events");
    assert_eq!(loaded.manifest.dropped_control, 0);
    // The line count is real: it equals the file.
    let text = std::fs::read_to_string(session.join("control.jsonl")).unwrap();
    assert_eq!(text.lines().count() as u64, integrity.lines);
}

#[test]
fn a_deleted_control_line_is_rejected_even_when_seq_is_hidden() {
    let dir = TempDir::new("ctl-delete");
    let (session, _) = finished_session(dir.path(), 22, 1300);
    let before = std::fs::read_to_string(session.join("control.jsonl")).unwrap();
    // Delete a `restart` line: the manifest still says N lines.
    let kept: String = before
        .lines()
        .filter(|l| !l.contains("\"k\":\"restart\""))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_ne!(kept, before, "the script restarted at least once");
    std::fs::write(session.join("control.jsonl"), &kept).unwrap();
    assert!(is_integrity_error(read_session(&session, false)));
    // The same deletion with a manifest forged to match is still caught by the
    // seq contiguity check.
    rewrite_control(&session, |lines| {
        lines
            .into_iter()
            .filter(|l| !l.contains("\"k\":\"restart\""))
            .collect()
    });
    let error = read_session(&session, false).unwrap_err();
    assert!(
        matches!(&error, PlaytraceError::StreamIntegrity { why, .. } if why.starts_with("seq ")),
        "{error}"
    );
}

#[test]
fn an_edited_control_line_fails_the_checksum() {
    let dir = TempDir::new("ctl-edit");
    let (session, _) = finished_session(dir.path(), 23, 700);
    let path = session.join("control.jsonl");
    let text = std::fs::read_to_string(&path).unwrap();
    // Same length, different content: only the FNV can see it.
    let edited = text.replacen("\"stage\":0", "\"stage\":1", 1);
    assert_ne!(edited, text);
    std::fs::write(&path, edited).unwrap();
    let error = read_session(&session, false).unwrap_err();
    assert!(
        matches!(&error, PlaytraceError::StreamIntegrity { why, .. } if why.contains("checksum")),
        "{error}"
    );
}

#[test]
fn control_at_t_must_not_go_backwards() {
    let dir = TempDir::new("ctl-order");
    let (session, _) = finished_session(dir.path(), 24, 1300);
    rewrite_control(&session, |mut lines| {
        // Lines are in seq order; give a later line an earlier tick.
        let last = lines.len() - 1;
        let victim = &lines[last];
        let mut value: serde_json::Value = serde_json::from_str(victim).unwrap();
        value["at_t"] = serde_json::json!(1);
        lines[last] = serde_json::to_string(&value).unwrap();
        lines
    });
    let error = read_session(&session, false).unwrap_err();
    assert!(
        matches!(&error, PlaytraceError::StreamIntegrity { why, .. } if why.starts_with("at_t")),
        "{error}"
    );
}

#[test]
fn a_seq_gap_is_legal_only_for_an_incomplete_session_that_counted_the_drop() {
    let dir = TempDir::new("ctl-drop");
    let mut writer = SessionWriter::create(dir.path(), manifest()).unwrap();
    let mut party = FakeParty::new(25, 0);
    drive(&mut writer, &mut party, 20);
    // The disk stalls and control events keep arriving: the queue bound drops.
    writer.fail_writes = u32::MAX;
    for _ in 0..(QUEUE_BOUND + 30) {
        writer.record_control(ControlEvent::Restart).unwrap();
    }
    assert!(writer.health().dropped_control >= 30);
    writer.fail_writes = 0;
    drive(&mut writer, &mut party, 5);
    writer.record_control(ControlEvent::Restart).unwrap();
    let manifest = writer.finish(SessionStatus::Completed).unwrap();
    assert_eq!(manifest.status, SessionStatus::Incomplete);
    assert!(manifest.dropped_control >= 30);
    let loaded = read_session(writer.dir(), false).expect("counted drop: the gap is allowed");
    let seqs: Vec<u64> = loaded.control.iter().map(|c| c.seq).collect();
    assert!(seqs.windows(2).any(|w| w[1] > w[0] + 1), "a real gap");

    // The same gap without the counter is a defect.
    let session = writer.dir().to_path_buf();
    let mut value: serde_json::Value = serde_json::from_str(&manifest_text(&session)).unwrap();
    value["dropped_control"] = serde_json::json!(0);
    std::fs::write(
        session.join("manifest.json"),
        serde_json::to_string(&value).unwrap(),
    )
    .unwrap();
    assert!(is_integrity_error(read_session(&session, false)));

    // And a completed session may never have one.
    let mut value: serde_json::Value = serde_json::from_str(&manifest_text(&session)).unwrap();
    value["dropped_control"] = serde_json::json!(30);
    value["status"] = serde_json::json!("completed");
    std::fs::write(
        session.join("manifest.json"),
        serde_json::to_string(&value).unwrap(),
    )
    .unwrap();
    assert!(read_session(&session, false).is_err());
}

#[test]
fn a_pt_b_session_without_the_integrity_record_still_reads() {
    let dir = TempDir::new("ctl-legacy");
    let (session, _) = finished_session(dir.path(), 26, 100);
    let mut value: serde_json::Value = serde_json::from_str(&manifest_text(&session)).unwrap();
    let object = value.as_object_mut().unwrap();
    for key in [
        "control_integrity",
        "diagnostics_integrity",
        "dropped_control",
        "dropped_diagnostics",
        "manifest_write_errors",
        "source_fingerprint",
    ] {
        object.remove(key);
    }
    std::fs::write(
        session.join("manifest.json"),
        serde_json::to_string(&value).unwrap(),
    )
    .unwrap();
    let loaded = read_session(&session, false).unwrap();
    assert!(loaded.manifest.control_integrity.is_none());
    assert!(loaded.manifest.source_fingerprint.is_empty());
}

#[test]
fn a_mid_line_short_write_is_truncated_back_and_the_retry_is_clean() {
    let dir = TempDir::new("short-write");
    let mut writer = SessionWriter::create(dir.path(), manifest()).unwrap();
    let mut party = FakeParty::new(27, 0);
    drive(&mut writer, &mut party, 40);
    // The disk fills in the middle of a tick line: 17 bytes reach the file.
    writer.short_write = Some(17);
    let tick = party.tick();
    writer.record_tick(&tick).unwrap();
    assert_eq!(writer.health().write_errors, 1);
    assert!(writer.health().queued >= 1, "the tick waits for a retry");
    // And in the middle of a control line.
    writer.record_control(ControlEvent::Restart).unwrap();
    writer.short_write = Some(9);
    writer.record_control(ControlEvent::Restart).unwrap();
    assert!(writer.health().write_errors >= 2);
    // The disk is back: the retries append whole lines.
    drive(&mut writer, &mut party, 30);
    let manifest = writer.finish(SessionStatus::Completed).unwrap();
    assert_eq!(
        manifest.status,
        SessionStatus::Incomplete,
        "the error is on the record"
    );
    assert_eq!(manifest.dropped_ticks, 0);
    // Strict read: no partial line anywhere in the middle of any stream.
    let loaded = read_session(writer.dir(), false).expect("no BadLine, no gap");
    assert_eq!(loaded.ticks.len(), 71);
    for (index, tick) in loaded.ticks.iter().enumerate() {
        assert_eq!(tick.t, index as u64);
    }
    let control = std::fs::read_to_string(writer.dir().join("control.jsonl")).unwrap();
    assert!(
        control
            .lines()
            .all(|l| serde_json::from_str::<ControlRecord>(l).is_ok())
    );
}

#[test]
fn a_failed_rotation_manifest_write_is_counted_not_swallowed() {
    let dir = TempDir::new("rotation");
    let mut writer = SessionWriter::create(dir.path(), manifest()).unwrap();
    let mut party = FakeParty::new(28, 0);
    writer.fail_manifest_writes = 1;
    drive(&mut writer, &mut party, 650);
    let health = writer.health();
    assert_eq!(health.manifest_write_errors, 1);
    assert!(
        !health.incomplete(),
        "the chunk and its trailer were written: no data is lost"
    );
    assert_eq!(health.last_error.as_deref(), Some("StorageFull"));
    let manifest = writer.finish(SessionStatus::Completed).unwrap();
    assert_eq!(manifest.status, SessionStatus::Completed);
    assert_eq!(manifest.manifest_write_errors, 1);
    assert!(manifest.capture_complete);
    assert_eq!(read_session(writer.dir(), false).unwrap().ticks.len(), 650);
}

#[test]
fn diagnostics_lines_are_batched_checked_and_never_make_a_capture_incomplete() {
    let dir = TempDir::new("diag");
    let mut writer = SessionWriter::create(dir.path(), manifest()).unwrap();
    let mut party = FakeParty::new(29, 0);
    let diag = |t: u64| DiagRecord {
        t,
        rig: Some([0, 0, 640, 360]),
        cam: [0, 0, 640, 360],
        hp: [5, 5],
        mana: None,
        hurt: [0, 0],
        enemies: 2,
        enemies_hidden: 1,
        obs_clipped: 0,
        obs_unknown_modes: 0,
        obs_overflow: 0,
    };
    for _ in 0..130 {
        let tick = party.tick();
        let t = tick.t;
        writer.record_tick(&tick).unwrap();
        writer.record_diag(&diag(t)).unwrap();
    }
    // Two full batches are on disk; the tail waits in memory.
    let on_disk = std::fs::read_to_string(writer.dir().join("diagnostics.jsonl")).unwrap();
    assert_eq!(on_disk.lines().count(), 120);
    let manifest = writer.finish(SessionStatus::Completed).unwrap();
    assert_eq!(manifest.diagnostics_integrity.unwrap().lines, 130);
    let loaded = read_session(writer.dir(), false).unwrap();
    assert_eq!(loaded.diag.len(), 130);
    assert_eq!(loaded.diag[129].t, 129);
    // A tampered diagnostics line is caught.
    let path = writer.dir().join("diagnostics.jsonl");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replacen("\"hp\":[5,5]", "\"hp\":[4,5]", 1)).unwrap();
    assert!(is_integrity_error(read_session(writer.dir(), false)));
}
