//! DONUT-MOBILITY-01 (DM-A) proofs for the playtrace identity of Donut's
//! motion: a new recording carries the motion flag and the agility hash, a
//! recording made before DM-A (no descriptor field) rebuilds and replays as
//! legacy, and the descriptor's absent field is an explicit legacy default.

use super::live_tests::{TempDir, root};
use super::synthetic::record_synthetic_fixture;
use super::tests_verify::tamper_manifest;
use super::*;
use crate::party::{MotionProfile, PartyTuning};
use crate::party_scene::PartyFixture;

/// `duo_independent_moves` with the given Donut motion: Donut runs and jumps,
/// so the two motions leave different bodies.
fn duo(motion: MotionProfile) -> &'static PartyFixture {
    let base = crate::cf02_fixtures::find_party_fixture("duo_independent_moves").unwrap();
    Box::leak(Box::new(PartyFixture {
        tuning: PartyTuning {
            donut_motion: motion,
            ..base.tuning
        },
        ..*base
    }))
}

#[test]
fn an_absent_descriptor_field_is_legacy_and_legacy_is_never_written() {
    let mut manifest = SessionManifest::new(generate_session_id(), tests::reset());
    manifest.reset.donut_motion = MotionProfile::Agile;
    let json = serde_json::to_string(&manifest).unwrap();
    assert!(json.contains("\"donut_motion\":\"agile\""), "{json}");
    let back: SessionManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(back.reset.donut_motion, MotionProfile::Agile);
    // Legacy is skipped on write, and a descriptor without the field reads as
    // legacy through the explicit default (never `Default`).
    manifest.reset.donut_motion = MotionProfile::Legacy;
    let json = serde_json::to_string(&manifest).unwrap();
    assert!(!json.contains("donut_motion"), "{json}");
    let back: SessionManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(back.reset.donut_motion, MotionProfile::Legacy);
}

#[test]
fn a_new_recording_carries_the_motion_flag_and_the_agility_hash_and_verifies() {
    let dir = TempDir::new("dm-agile");
    let manifest = record_synthetic_fixture(&dir.0, &root(), duo(MotionProfile::Agile), 160)
        .expect("recorded");
    assert_eq!(manifest.reset.donut_motion, MotionProfile::Agile);
    assert_eq!(
        manifest
            .feature_flags
            .get("donut_motion")
            .map(String::as_str),
        Some("agile")
    );
    assert_eq!(
        manifest.content_hashes.get("agility_tuning"),
        Some(&format!("{:016x}", crate::agility::TUNING_HASH))
    );
    let session = dir.0.join(&manifest.session_id);
    verify_dir(&session, &root()).expect("an agile recording verifies on its own build");
    // A drifted agility hash is a content mismatch, not a silent drift.
    tamper_manifest(&session, |m| {
        m.content_hashes
            .insert("agility_tuning".to_owned(), "0".repeat(16));
    });
    let failure = verify_dir(&session, &root()).expect_err("a drifted hash is refused");
    assert_eq!(failure.code, "content_mismatch");
}

#[test]
fn a_recording_made_before_dm_a_rebuilds_and_replays_as_legacy() {
    let dir = TempDir::new("dm-legacy");
    let manifest = record_synthetic_fixture(&dir.0, &root(), duo(MotionProfile::Legacy), 160)
        .expect("recorded");
    let session = dir.0.join(&manifest.session_id);
    // Make it look like a pre-DM-A manifest: no motion flag, no agility hash.
    // (The descriptor field is already absent: legacy is never written.)
    tamper_manifest(&session, |m| {
        m.feature_flags.remove("donut_motion");
        m.content_hashes.remove("agility_tuning");
    });
    let text = std::fs::read_to_string(session.join("manifest.json")).unwrap();
    assert!(!text.contains("donut_motion"), "{text}");
    let report = verify_dir(&session, &root()).expect("a pre-DM-A recording verifies as legacy");
    assert_eq!(report.ticks, 160);
}

#[test]
fn a_motion_that_disagrees_with_the_recording_diverges() {
    // An agile recording whose descriptor is rewritten to legacy replays a
    // different body, so the strict verifier refuses it.
    let dir = TempDir::new("dm-mismatch");
    let manifest = record_synthetic_fixture(&dir.0, &root(), duo(MotionProfile::Agile), 160)
        .expect("recorded");
    let session = dir.0.join(&manifest.session_id);
    tamper_manifest(&session, |m| {
        m.reset.donut_motion = MotionProfile::Legacy;
        m.feature_flags
            .insert("donut_motion".to_owned(), "legacy".to_owned());
    });
    assert!(verify_dir(&session, &root()).is_err());
}
