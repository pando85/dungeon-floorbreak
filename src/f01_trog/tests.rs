//! TROG-A room tests: the pinned files, the launch, the slot roster and the replay.

use super::*;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn hash(path: &str) -> u64 {
    crate::donut_mana::content_hash(&std::fs::read_to_string(root().join(path)).expect("reads"))
}

#[test]
fn the_pinned_hashes_are_the_files() {
    for room in TrogRoom::ALL {
        assert_eq!(hash(room.room_path()), room.room_hash(), "{}", room.id());
        assert_eq!(
            hash(room.encounter_path()),
            room.encounter_hash(),
            "{}",
            room.id()
        );
        let run = launch(&root(), room).expect("the room launches");
        assert_eq!(run.loaded.room_hash, room.room_hash());
        assert_eq!(run.loaded.sidecar_hash, room.encounter_hash());
    }
    assert_eq!(
        hash("assets/data/gameplay/enemy_trog_v0.json"),
        crate::trog::TUNING_HASH
    );
    for (key, value) in data_pins() {
        assert_ne!(value, 0, "{key}");
    }
}

#[test]
fn the_room_spawns_its_three_troglodytes_from_the_roster() {
    let run = launch(&root(), TrogRoom::Run).unwrap();
    let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
    let combat = combat_of(&run, &mut scene).unwrap();
    let kinds: Vec<&str> = combat.slot_enemies().iter().map(|e| e.kind()).collect();
    assert_eq!(kinds, ["trog_pygmy", "trog_basher", "trog_virtuoso"]);
    let ids: Vec<u32> = combat
        .slot_enemies()
        .iter()
        .map(|e| e.target_id())
        .collect();
    assert_eq!(
        ids,
        [
            crate::combat_scene::trog::target_id(0),
            crate::combat_scene::trog::target_id(1),
            crate::combat_scene::trog::target_id(2)
        ]
    );
    assert!(!combat.roster_cleared());
    assert!(combat.env().is_some(), "the env step is live");
    let line = entry_line(&run);
    assert!(line.starts_with("F01_TROG_RUN_ENTRY"), "{line}");
    assert!(line.contains("pygmies=1 bashers=1 virtuosos=1"), "{line}");
}

#[test]
fn the_room_replays_to_the_same_hash_twice() {
    let run_once = || {
        let run = launch(&root(), TrogRoom::Run).unwrap();
        let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
        let mut combat = combat_of(&run, &mut scene).unwrap();
        for tick in 1..=500u64 {
            combat.step(&mut scene, run.fixture.input(tick.min(SMOKE_TICKS)));
        }
        combat.state_hash(&scene)
    };
    assert_eq!(run_once(), run_once());
}

#[test]
fn a_roster_kind_other_than_a_troglodyte_is_refused_not_dropped() {
    let run = launch(&root(), TrogRoom::Run).unwrap();
    assert!(trogs_only(&run.loaded.encounter.roster).is_ok());
    let mut roster = run.loaded.encounter.roster.clone();
    roster[0].kind = RosterKind::Slime;
    let error = trogs_only(&roster).expect_err("a slime is refused");
    assert!(error.contains("troglodytes only"), "{error}");
}
