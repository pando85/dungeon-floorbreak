//! PT-D proofs for the Rust side of the exporter: the synthetic fixture really
//! verifies and is labelled synthetic, and `stats` counts agree with the
//! journal. The exporter itself is Python (`tools/playtrace`).

use super::live_tests::{TempDir, root};
use super::synthetic::{DEFAULT_FIXTURE, SYNTHETIC_FLAG, SYNTHETIC_MODE, record_synthetic};
use super::*;

#[test]
fn the_synthetic_fixture_verifies_and_is_labelled_synthetic() {
    let dir = TempDir::new("pt-d-synth");
    let manifest = record_synthetic(&dir.0, &root(), DEFAULT_FIXTURE, 240).expect("recorded");
    assert_eq!(manifest.tick_count, 240);
    assert_eq!(manifest.mode, SYNTHETIC_MODE);
    assert!(manifest.quality_flags.iter().any(|f| f == SYNTHETIC_FLAG));
    assert_eq!(
        manifest.feature_flags.get("synthetic").map(String::as_str),
        Some("true")
    );
    assert!(!manifest.observation_is_partial);
    let session = dir.0.join(&manifest.session_id);
    let report = verify_dir(&session, &root()).expect("the synthetic session verifies");
    assert_eq!(report.ticks, 240);
    assert_eq!(report.scope, VerifyScope::Full);
    // The restart in the middle gives a second episode.
    assert_eq!(report.episodes, 2);
}

#[test]
fn the_synthetic_recorder_rejects_unknown_and_arena_fixtures() {
    let dir = TempDir::new("pt-d-synth-bad");
    assert!(record_synthetic(&dir.0, &root(), "no_such_fixture", 10).is_err());
    assert!(record_synthetic(&dir.0, &root(), "cf02_parapet_cover", 10).is_err());
    assert!(
        list_sessions(&dir.0).is_empty(),
        "a refused run leaves no session"
    );
}

#[test]
fn stats_count_every_tick_once_and_keep_human_and_ai_apart() {
    let dir = TempDir::new("pt-d-stats");
    // The solo fixture: Carl is human first, then the swap makes Donut the human.
    let manifest =
        record_synthetic(&dir.0, &root(), "solo_swap_continuity", 300).expect("recorded");
    let loaded = read_session(&dir.0.join(&manifest.session_id), false).unwrap();
    let stats = session_stats(&loaded);
    assert_eq!(stats.ticks, 300);
    assert_eq!(stats.episodes, 2);
    for hero in &stats.heroes {
        // Every tick of every hero is driven by exactly one class.
        assert_eq!(hero.human.ticks + hero.ai.ticks + hero.undriven_ticks, 300);
        let moves = hero.human.left + hero.human.neutral + hero.human.right;
        assert_eq!(moves, hero.human.ticks);
        let moves = hero.ai.left + hero.ai.neutral + hero.ai.right;
        assert_eq!(moves, hero.ai.ticks);
    }
    // One hero is human on a tick, the other is not (solo play).
    assert_eq!(
        stats.heroes[0].human.ticks + stats.heroes[1].human.ticks,
        300,
        "exactly one human-driven hero per tick in a solo session"
    );
    assert!(stats.heroes[0].human.ticks > 0 && stats.heroes[1].human.ticks > 0);
}
