use super::*;
use crate::combat_scene::EnvSolidKind;
use crate::encounter::{ReachHero, route_graphs};
use crate::f01_late::Loadout;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn hash(path: &str) -> u64 {
    crate::donut_mana::content_hash(&std::fs::read_to_string(root().join(path)).expect("reads"))
}

fn shore() -> Act1Run {
    launch(&root(), Act1Room::Shore).expect("the shore launches")
}

fn guild() -> Act1Run {
    launch(&root(), Act1Room::Guild).expect("the guild launches")
}

fn scene_of(run: &Act1Run) -> SceneSim {
    SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap()
}

#[test]
fn the_pinned_hashes_are_the_files() {
    for room in Act1Room::ALL {
        assert_eq!(
            hash(room.room_path()),
            room.room_hash(),
            "{:#018x}",
            hash(room.room_path())
        );
        assert_eq!(
            hash(room.encounter_path()),
            room.encounter_hash(),
            "{:#018x}",
            hash(room.encounter_path())
        );
        let run = launch(&root(), room).expect("the room launches");
        assert_eq!(run.loaded.room_hash, room.room_hash());
        assert_eq!(run.loaded.sidecar_hash, room.encounter_hash());
        assert_eq!(Act1Room::from_id(room.id()), Some(room));
    }
    for (key, value) in data_pins() {
        assert_ne!(value, 0, "{key}");
    }
    let keys: Vec<&str> = data_pins().iter().map(|(k, _)| *k).collect();
    assert_eq!(
        keys,
        [
            "f01_act1_shore_room",
            "f01_act1_shore_encounter",
            "f01_act1_guild_room",
            "f01_act1_guild_encounter"
        ]
    );
    assert_eq!(Act1Room::from_id("shore"), None);
}

/// An edited sidecar is refused until it is re-pinned on purpose.
#[test]
fn an_unpinned_edit_is_refused() {
    let dir = std::env::temp_dir().join(format!("act1-edit-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("assets/levels/floor01")).unwrap();
    std::fs::create_dir_all(dir.join("assets/data/gameplay")).unwrap();
    let text = std::fs::read_to_string(root().join(SHORE_ENCOUNTER_PATH)).unwrap();
    let edited = text.replace("\"x\": 528", "\"x\": 532");
    assert_ne!(edited, text);
    std::fs::write(dir.join(SHORE_ENCOUNTER_PATH), edited).unwrap();
    for rel in [SHORE_ROOM_PATH, crate::encounter::HEIGHT_CLASSES_PATH] {
        std::fs::copy(root().join(rel), dir.join(rel)).unwrap();
    }
    let err = launch(&dir, Act1Room::Shore).expect_err("an unpinned edit");
    assert!(err.contains("re-pin src/f01_act1.rs"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// ACT1-T03: the shore's sidecar is valid: two goblins under `defeat_roster`, no lock, no
/// platform, the 976 px room, the exit and the checkpoint where the design puts them.
#[test]
fn act1_shore_sidecar_validates() {
    let run = shore();
    let loaded = &run.loaded;
    assert_eq!(loaded.encounter.id, "F01-ACT1-SHORE-ENC");
    assert_eq!(loaded.encounter.goal, Goal::DefeatRoster);
    assert_eq!(run.kit.width, 976);
    assert_eq!(loaded.encounter.max_living, 2);
    assert!(loaded.encounter.locks.is_empty() && loaded.room.platforms.is_empty());
    assert_eq!(loaded.ceiling_px, None);
    assert_eq!(run.fixture.shore_dx, Some(SHORE_DX));
    assert_eq!(
        run.loaded.room.spawn_carl + SHORE_DX,
        367,
        "local strike column"
    );
    let roster: Vec<(&str, i32, &str)> = loaded
        .encounter
        .roster
        .iter()
        .map(|e| (e.id.as_str(), e.x, e.beat.as_str()))
        .collect();
    assert_eq!(roster, [("gob_a", 528, "I03"), ("gob_b", 568, "I03")]);
    let goblins: Vec<i32> = run.fixture.enemies.iter().map(|e| e.x).collect();
    assert_eq!(goblins, [528, 568]);
    // The exit is the guild portal at local 760, inside the protected exit (x >= 624).
    let exit = loaded.encounter.exit;
    assert_eq!((exit.x, exit.right()), (728, 792));
    assert!(exit.x >= 624);
    // The checkpoint is the post-chase column (route 2640 -> local 32): a respawn goes there.
    let cp = loaded.encounter.checkpoint;
    assert_eq!((cp.x, cp.x + cp.w / 2), (32, 56));
    assert_eq!(run.rules.checkpoints, [56]);
    // No surface above the 42 px free height.
    assert!(
        run.kit.solid_rects.iter().all(|s| s.w >= run.kit.width
            || s.y + s.h >= run.kit.base_walk_line_y - 42
            || s.y <= 0),
        "{:?}",
        run.kit.solid_rects
    );
}

/// ACT1-T03: the guild's sidecar is valid: an empty roster under `reach_exit`, one
/// `donut_unlocked` lock, the 228 px ceiling slab, a 640 px room.
#[test]
fn act1_guild_sidecar_validates() {
    let run = guild();
    let loaded = &run.loaded;
    assert_eq!(loaded.encounter.id, "F01-ACT1-GUILD-ENC");
    assert_eq!(loaded.encounter.goal, Goal::ReachExit);
    assert_eq!(run.kit.width, 640);
    assert!(loaded.encounter.roster.is_empty() && loaded.room.platforms.is_empty());
    assert_eq!(loaded.ceiling_px, Some(228));
    assert_eq!(loaded.encounter.locks.len(), 1);
    assert_eq!(loaded.encounter.locks[0].id, GUILD_DOOR);
    assert_eq!(loaded.encounter.locks[0].opens_on, LockEvent::DonutUnlocked);
    let slab_underside = run.kit.base_walk_line_y - GUILD_CEILING_PX;
    assert!(
        run.kit
            .solid_rects
            .iter()
            .any(|s| s.w >= run.kit.width && s.y + s.h == slab_underside)
    );
    let exit = loaded.encounter.exit;
    assert_eq!((exit.x, exit.right()), (560, 624));
    assert_eq!(
        run.rules.checkpoints,
        [88],
        "the checkpoint is behind the start"
    );
    // The decor guide stands on the floor at the refuge_guild mark (x 380).
    let guide = decor(Act1Room::Guild)[0];
    assert_eq!(guide.label, "guide");
    assert_eq!(guide.rect.x + guide.rect.w / 2, 380);
    assert_eq!(guide.rect.bottom(), 580);
    let shore_decor = decor(Act1Room::Shore)[0];
    assert_eq!(shore_decor.label, "wedged_dozer");
    assert!(shore_decor.rect.bottom() <= 580);
}

/// ACT1-T04: the guild door seals the room: with the lock closed no class reaches the exit (it
/// is a full-height solid from the walking line up to the slab: a cut for every class), with
/// it open every class does. The door reaches the slab, so no jump clears it.
#[test]
fn act1_guild_door_seals_until_the_biscuit() {
    let run = guild();
    let enc = &run.loaded.encounter;
    let graphs = route_graphs(&run.loaded.room, &run.loaded.classes, enc);
    let reaches = |g: &crate::encounter::RouteGraph| -> bool {
        let (Some(spawn), Some(goal)) = (g.spawn, g.goal) else {
            return false;
        };
        let mut seen = vec![false; g.nodes.len()];
        seen[spawn] = true;
        let mut stack = vec![spawn];
        while let Some(i) = stack.pop() {
            for &(a, b) in &g.edges {
                if a == i && !seen[b] {
                    seen[b] = true;
                    stack.push(b);
                }
            }
        }
        seen[goal]
    };
    let (mut open, mut closed) = (0, 0);
    for g in &graphs {
        if g.locks_closed {
            closed += 1;
            assert!(
                !reaches(g),
                "{:?}: the exit is reachable with the door closed",
                g.hero
            );
            assert!(g.solids.contains(&enc.locks[0].solid));
        } else {
            open += 1;
            assert!(
                reaches(g),
                "{:?}: the exit is unreachable with the door open",
                g.hero
            );
        }
    }
    assert_eq!((open, closed), (3, 3), "every class, both ways");
    for hero in [
        ReachHero::Carl,
        ReachHero::DonutBase,
        ReachHero::DonutBoosted,
    ] {
        assert!(graphs.iter().any(|g| g.hero == hero && g.locks_closed));
    }
    let door = enc.locks[0].solid;
    assert_eq!(door.bottom(), run.loaded.room.floor_y, "floor to ...");
    let slab_top = run.kit.base_walk_line_y - GUILD_CEILING_PX - 16;
    assert_eq!(door.y, slab_top, "... the slab: nobody jumps over it");
    // In the running room the door is an env solid, standing while Donut's mana is locked.
    let mut scene = scene_of(&run);
    let combat = combat_of(&run, &mut scene).unwrap();
    let env = combat.env().expect("the env runtime");
    assert_eq!(
        env.solids()
            .iter()
            .filter(|s| matches!(s.kind, EnvSolidKind::ManaLock { .. }))
            .count(),
        1
    );
    assert!(!combat.donut_unlocked());
    assert!(env.solid_intact(0), "the runtime alone defaults to closed");
    assert_eq!(env.mana_doors(), [door]);
}

/// The Act I story pins are labels: no table row names them (ACT1-AD1).
#[test]
fn the_act1_story_pins_are_not_table_rows() {
    let table = StoryTable::active();
    for room in Act1Room::ALL {
        let run = launch(&root(), room).unwrap();
        assert_eq!(run.loaded.encounter.story_pin, room.story_pin());
        assert!(
            !table.has_pin(room.story_pin()),
            "{}: a pin row would be held",
            room.story_pin()
        );
        // A story run of the room holds and arms nothing: no triggers, no stored capability.
        let (carl, donut) = Loadout::Story.flags();
        let mut scene = scene_of(&run);
        let combat = combat_for(&run, &mut scene, carl, donut, None).unwrap();
        assert!(combat.story_snapshot().is_none_or(|s| s.held.is_empty()));
    }
}

/// Both rooms launch under every loadout, in a state the entry line reports (ACT1-AD13).
#[test]
fn the_act1_rooms_launch() {
    let (carl, donut) = Loadout::Story.flags();
    let (all_carl, all_donut) = Loadout::All.flags();
    for room in Act1Room::ALL {
        let run = launch(&root(), room).unwrap();
        for flags in [(carl, donut), (all_carl, all_donut), Default::default()] {
            let mut scene = scene_of(&run);
            let combat = combat_for(&run, &mut scene, flags.0, flags.1, None).unwrap();
            let line = entry_line(&run, &combat);
            assert!(
                line.starts_with(&format!("{}_ENTRY room=F01-ACT1-", room.marker())),
                "{line}"
            );
            assert!(
                line.contains("greybox=true") && line.contains("env=live"),
                "{line}"
            );
            // Locked under the Story setup or no flag; held under the All loadout.
            let held = flags.1.light_feet || flags.1.puddle;
            assert_eq!(combat.donut_unlocked(), held, "{line}");
            if room == Act1Room::Guild {
                let door = if held { "door=open" } else { "door=closed" };
                let biscuit = if held { "biscuit=held" } else { "biscuit=zone" };
                assert!(line.contains(door) && line.contains(biscuit), "{line}");
            } else {
                assert!(line.contains("goblins=2"), "{line}");
            }
        }
    }
}

/// ACT1-AD13: Donut's setup per launch.
#[test]
fn donut_is_locked_in_act1_under_story_and_open_under_all() {
    let (_, story) = Loadout::Story.flags();
    let (_, all) = Loadout::All.flags();
    assert_eq!(donut_setup(Act1Room::Shore, story), DonutSetup::NONE);
    assert_eq!(
        donut_setup(Act1Room::Shore, DonutProgFlags::default()),
        DonutSetup::NONE
    );
    assert_eq!(
        donut_setup(Act1Room::Guild, story).guild_zone,
        Some((336.0, 376.0))
    );
    assert!(!donut_setup(Act1Room::Guild, story).start.unlocked);
    for room in Act1Room::ALL {
        assert_eq!(donut_setup(room, all), DonutSetup::unlocked(8));
        let light = DonutProgFlags {
            light_feet: true,
            ..DonutProgFlags::default()
        };
        assert_eq!(donut_setup(room, light), DonutSetup::unlocked(8));
    }
}

/// ACT1-IMP-1: the shore's goblins are a brawler and a coward on every attempt and seed.
#[test]
fn the_shore_goblins_are_a_brawler_and_a_coward() {
    let run = shore();
    for seed in [1u64, 2, 3, 0xDEAD_BEEF] {
        let mut scene = scene_of(&run);
        let combat = combat_for(
            &run,
            &mut scene,
            CarlProgFlags::default(),
            DonutProgFlags::default(),
            Some(seed),
        )
        .unwrap();
        let temperaments: Vec<Temperament> = (0..2)
            .map(|i| combat.goblin_morale(i).expect("morale").temperament())
            .collect();
        assert_eq!(temperaments, SHORE_TEMPERAMENTS, "seed {seed}");
    }
}

/// The shore smoke: Carl walks right and must not reach the exit (the goblins are alive and the
/// derived exit gate stands until both are defeated).
#[test]
fn the_shore_smoke_does_not_reach_the_exit() {
    let run = shore();
    let mut scene = scene_of(&run);
    let mut combat = combat_of(&run, &mut scene).unwrap();
    for tick in 1..=SMOKE_TICKS {
        combat.step(&mut scene, run.fixture.input(tick));
    }
    assert!(
        !scene.world.complete,
        "the exit stays closed with the goblins alive"
    );
    assert!(!combat.roster_cleared());
}
