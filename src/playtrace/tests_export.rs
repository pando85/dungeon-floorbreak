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

#[test]
fn synth_fixture_refuses_the_default_store_and_anything_inside_it() {
    use super::synthetic::refuse_default_store;
    let dir = TempDir::new("pt-d-store");
    let store = dir.0.join("data/dungeon-floorbreak/playtrace");
    // The store itself, a not-yet-created child and a `..` detour are all refused.
    for base in [
        store.clone(),
        store.join("nested/new"),
        store.join("x/../y"),
        dir.0.join("data/dungeon-floorbreak/./playtrace"),
    ] {
        assert!(
            refuse_default_store(&base, &store).is_err(),
            "{} must be refused",
            base.display()
        );
    }
    // A sibling (sharing a name prefix) and an unrelated directory are fine.
    assert!(
        refuse_default_store(&dir.0.join("data/dungeon-floorbreak/playtrace-x"), &store).is_ok()
    );
    assert!(refuse_default_store(&dir.0.join("scratch"), &store).is_ok());
    // A symlink into the store is refused too.
    #[cfg(unix)]
    {
        std::fs::create_dir_all(&store).unwrap();
        let link = dir.0.join("link");
        std::os::unix::fs::symlink(&store, &link).unwrap();
        assert!(refuse_default_store(&link.join("child"), &store).is_err());
    }
}

#[test]
fn stats_label_a_synthetic_session_as_synthetic_not_human() {
    use super::synthetic::is_synthetic;
    let dir = TempDir::new("pt-d-stats-synth");
    let manifest = record_synthetic(&dir.0, &root(), DEFAULT_FIXTURE, 120).expect("recorded");
    let loaded = read_session(&dir.0.join(&manifest.session_id), false).unwrap();
    assert!(session_stats(&loaded).synthetic);
    // Any single marker is enough, so dropping one flag does not relabel it human.
    let mut m = loaded.manifest.clone();
    m.quality_flags.clear();
    assert!(is_synthetic(&m), "mode and feature flag still mark it");
    m.feature_flags.clear();
    assert!(is_synthetic(&m), "the mode still marks it");
    m.mode = "human".to_owned();
    assert!(!is_synthetic(&m));
}
