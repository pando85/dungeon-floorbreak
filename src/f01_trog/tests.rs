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

/// The T3 platform against the real room file (TROG-AD35): its slab bottom clears the
/// Virtuoso's band top, the tongue from the leash start runs under it, and a Carl standing on it
/// is not a Virtuoso candidate.
#[test]
fn the_t3_platform_stands_over_the_tongue_in_the_real_room() {
    use crate::trog::{Spawn, Species, Trog, TrogTuning};
    let run = launch(&root(), TrogRoom::Run).unwrap();
    let room = &run.loaded.room;
    let platform = room.platforms[0].rect;
    let slab = run.loaded.encounter.platform_slab_px;
    let t = TrogTuning::active();
    let sp = t.species(Species::Virtuoso);
    // The platform is 64 above the floor and its slab bottom 52 above it.
    assert_eq!(room.floor_y - platform.y, 64);
    let slab_bottom = room.floor_y - (platform.y + slab);
    assert_eq!(slab_bottom, 52);
    assert!(
        slab_bottom >= i32::from(sp.hitbox.y1),
        "the band (top 46) passes under the slab"
    );
    // The tongue from the leash start (2016) facing left spans 1939..1960, under 1920..1984.
    let vir = run
        .loaded
        .encounter
        .roster
        .iter()
        .find(|e| e.kind.trog() == Some(Species::Virtuoso))
        .unwrap();
    let leash_min = vir.x - i32::from(sp.leash_px);
    assert_eq!(leash_min, 2016);
    let (a, b) = (
        leash_min - i32::from(sp.hitbox.x1),
        leash_min - i32::from(sp.hitbox.x0),
    );
    assert_eq!((a, b), (1939, 1960));
    assert!(a >= platform.x && b <= platform.right());
    // A Carl on the platform (x 1952, feet 64 up) is not a candidate; on the floor he is.
    let floor = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x)
        .unwrap()
        .world
        .hero_foot_y();
    let trog = Trog::new(
        t,
        &Spawn {
            species: Species::Virtuoso,
            index: 2,
            x: vir.x as f32,
            floor_y: floor,
            trigger_x: vir.trigger_x.map(|x| x as f32),
            delay_ticks: 0,
            protected: (16.0, 224.0),
        },
    );
    let carl_at = |x: f32, foot: f32| crate::boss::HeroView {
        id: crate::party::HeroId::Carl,
        present: true,
        x,
        foot_y: foot,
        grounded: true,
        hurt: crate::prototype::Rect {
            left: x - 12.0,
            bottom: foot,
            width: 24.0,
            height: 52.0,
        },
        vulnerable: true,
        bot: false,
    };
    assert!(!trog.is_candidate(t, &carl_at(1952.0, floor + 64.0)));
    assert!(trog.is_candidate(t, &carl_at(1952.0, floor)));
}
