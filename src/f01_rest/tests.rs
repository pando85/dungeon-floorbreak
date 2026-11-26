use super::*;
use crate::combat_scene::EnvSolidKind;
use crate::encounter::{ReachHero, route_graphs};
use crate::f01_late::Loadout;
use crate::story_unlock::SPECIAL_THROWN_CHARGE;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn hash(path: &str) -> u64 {
    crate::donut_mana::content_hash(&std::fs::read_to_string(root().join(path)).expect("reads"))
}

fn refuge() -> RestRun {
    launch(&root(), RestRoom::Refuge).expect("the refuge launches")
}

fn scene_of(run: &RestRun) -> SceneSim {
    SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap()
}

#[test]
fn the_pinned_hashes_are_the_files() {
    for room in RestRoom::ALL {
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
        assert_eq!(RestRoom::from_id(room.id()), Some(room));
    }
    for (key, value) in data_pins() {
        assert_ne!(value, 0, "{key}");
    }
    assert_eq!(RestRoom::from_id("refuge"), None);
}

/// An edited sidecar is refused until it is re-pinned on purpose.
#[test]
fn an_unpinned_edit_is_refused() {
    let dir = std::env::temp_dir().join(format!("restw-edit-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("assets/levels/floor01")).unwrap();
    std::fs::create_dir_all(dir.join("assets/data/gameplay")).unwrap();
    let text = std::fs::read_to_string(root().join(REFUGE_ENCOUNTER_PATH)).unwrap();
    let edited = text.replace("\"x\": 1104", "\"x\": 1108");
    assert_ne!(edited, text);
    std::fs::write(dir.join(REFUGE_ENCOUNTER_PATH), edited).unwrap();
    for rel in [REFUGE_ROOM_PATH, crate::encounter::HEIGHT_CLASSES_PATH] {
        std::fs::copy(root().join(rel), dir.join(rel)).unwrap();
    }
    let err = launch(&dir, RestRoom::Refuge).expect_err("an unpinned edit");
    assert!(err.contains("re-pin src/f01_rest.rs"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The sidecar is valid with an empty roster under `reach_exit` (REST-AD13), the shelf is
/// a boosted-Donut surface that no weaker class bypasses, the lane ceiling backs its
/// vantage and the room declares the 228 px slab.
#[test]
fn rest_refuge_sidecar_validates() {
    let run = refuge();
    let loaded = &run.loaded;
    assert_eq!(loaded.encounter.id, "F01-REST-REFUGE-ENC");
    assert_eq!(loaded.encounter.goal, crate::encounter::Goal::ReachExit);
    assert!(loaded.encounter.roster.is_empty());
    assert_eq!(loaded.ceiling_px, Some(228));
    // p2's effective class is `donut_boosted` (no bypass), the decks are Carl's.
    assert_eq!(
        loaded.encounter.effective_gates,
        [ReachHero::Carl, ReachHero::Carl, ReachHero::DonutBoosted]
    );
    assert_eq!(
        loaded.encounter.vantages,
        [(2, crate::encounter::VantageCounter::LaneCeiling)]
    );
    assert_eq!(loaded.encounter.locks.len(), 1);
    assert_eq!(loaded.encounter.locks[0].id, STAFF_DOOR);
    // The slab of the DP-Q45 ceiling is a real solid of the kit.
    let underside = run.kit.base_walk_line_y - 228;
    assert!(
        run.kit
            .solid_rects
            .iter()
            .any(|s| s.w >= run.kit.width && s.y + s.h == underside)
    );
    // The room's story pin is a label with no table row (a pin row must hold something).
    assert!(!StoryTable::active().has_pin(&loaded.encounter.story_pin));
    assert_eq!(loaded.encounter.story_pin, "f01_pin_refuge_entry");
}

/// The physics-exact run (the validator's route graphs replayed with the real hero
/// integrator on the launched kit): every accepted edge lands, boosted Donut lands on the
/// shelf, and base Donut and Carl have no shelf edge.
#[test]
fn rest_refuge_shelf_is_boosted_only() {
    use crate::encounter_physics::check;
    let run = refuge();
    let report = check(&run.kit, &run.loaded).unwrap();
    println!("rest_refuge K8 {}", report.summary());
    assert!(
        report.untraversable().is_empty(),
        "validator false positives: {:#?}",
        report.untraversable()
    );
    let shelf = run.loaded.room.platforms[2].rect;
    let lands = |hero: ReachHero| {
        report
            .edges
            .iter()
            .any(|e| e.hero == hero && e.to.top == shelf.y && !e.to.floor && e.witness.is_some())
    };
    assert!(
        lands(ReachHero::DonutBoosted),
        "boosted Donut lands on the shelf"
    );
    assert!(!lands(ReachHero::DonutBase), "base Donut does not");
    assert!(!lands(ReachHero::Carl), "Carl does not");
    // The decks are Carl's: he lands on the play deck from the floor.
    let deck = run.loaded.room.platforms[0].rect;
    assert!(report.edges.iter().any(|e| {
        e.hero == ReachHero::Carl && e.to.top == deck.y && !e.to.floor && e.witness.is_some()
    }));
}

/// The staff door seals the room: with the lock closed no class reaches the exit (it is a
/// 228 px solid from the walking line to the slab: a cut for every class, never a node);
/// with it open every class does.
#[test]
fn rest_refuge_staff_door_seals_the_room() {
    let run = refuge();
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
    let mut open = 0;
    let mut closed = 0;
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
    // 2P: the two spawns are Carl's and Donut's; both are walled off from the exit while
    // closed (the per-class check above reads each spawn).
    let door = enc.locks[0].solid;
    assert_eq!(door.bottom(), run.loaded.room.floor_y, "floor to ...");
    assert_eq!(
        door.y,
        run.kit.base_walk_line_y - 228,
        "... the slab's underside: nobody jumps over it"
    );
    // In the running room the door is an env solid standing in the world until opened.
    let mut scene = scene_of(&run);
    let combat = combat_of(&run, &mut scene).unwrap();
    let env = combat.env().expect("the env runtime");
    assert_eq!(
        env.solids()
            .iter()
            .filter(|s| matches!(s.kind, EnvSolidKind::SurfaceLock { .. }))
            .count(),
        1
    );
    assert_eq!(env.surface_locks_open(), [false]);
    assert!(env.solid_intact(0), "closed: the door stands");
}

/// REST-AD4: the table's reward zone is the reward box's column, after the checkpoint
/// and left of the shelf, so Donut always holds the boost when she can first reach it.
#[test]
fn rest_refuge_reward_zone_sits_at_the_box() {
    let run = refuge();
    let table = StoryTable::active();
    let trigger = table.trigger("f01_safe_room_reward_found").unwrap();
    let zone = trigger.zone.expect("a zone trigger");
    assert_eq!(zone.hero, 1);
    assert_eq!(
        (zone.min_x as i32, zone.max_x as i32),
        REWARD_ZONE_X,
        "the zone's x range is the box's"
    );
    let box_rect = decor(RestRoom::Refuge)
        .iter()
        .find(|d| d.label == "reward_box")
        .expect("the reward box decor");
    assert_eq!((box_rect.rect.x, box_rect.rect.right()), REWARD_ZONE_X);
    let checkpoint = run.loaded.encounter.checkpoint;
    assert!(
        zone.min_x as i32 > checkpoint.right(),
        "after the checkpoint"
    );
    assert!(
        (zone.max_x as i32) < run.loaded.room.platforms[2].rect.x,
        "left of the shelf"
    );
    // The decor is presentation only: unique labels and never inside the sim.
    let mut labels: Vec<&str> = decor(RestRoom::Refuge).iter().map(|d| d.label).collect();
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(labels.len(), decor(RestRoom::Refuge).len());
}

/// `jump_reach_boost` requires `primary_ranged`: Donut starts with the biscuit, or the
/// trigger would be spent without granting and the room would seal on every restart.
#[test]
fn rest_refuge_starts_with_the_biscuit() {
    let run = refuge();
    let mut scene = scene_of(&run);
    let (carl, donut) = Loadout::Story.flags();
    for flags in [(carl, donut), Default::default()] {
        let combat = combat_for(&run, &mut scene, flags.0, flags.1, None).unwrap();
        assert!(
            combat.donut_mana_view().is_some(),
            "Donut's mana is unlocked at tick 0"
        );
        assert!(combat.story_holds(crate::story_unlock::PRIMARY_RANGED));
    }
}

/// The entry line says what the room holds: `boost=armed` under the Story flags,
/// `boost=held` under the All loadout, `boost=none` flagless (sealed by design).
#[test]
fn rest_refuge_entry_line_reports_the_boost() {
    let run = refuge();
    let line = |carl, donut| {
        let mut scene = scene_of(&run);
        let combat = combat_for(&run, &mut scene, carl, donut, None).unwrap();
        entry_line(&run, &combat)
    };
    let (carl, donut) = Loadout::Story.flags();
    let story = line(carl, donut);
    assert!(
        story.contains("lock=closed") && story.contains("boost=armed"),
        "{story}"
    );
    assert!(
        story.starts_with("F01_REST_REFUGE_ENTRY room=F01-REST-REFUGE "),
        "{story}"
    );
    let (carl, donut) = Loadout::All.flags();
    let all = line(carl, donut);
    assert!(all.contains("boost=held"), "{all}");
    let none = line(CarlProgFlags::default(), DonutProgFlags::default());
    assert!(
        none.contains("lock=closed") && none.contains("boost=none"),
        "{none}"
    );
}

/// Flags decide what the room holds: under the Story flags the reward trigger is armed
/// and nothing else is held; under no flags nothing exists (a sealed room).
#[test]
fn the_story_flags_arm_the_reward_and_hold_nothing_else() {
    let run = refuge();
    let holds = |carl, donut, id| {
        let mut scene = scene_of(&run);
        combat_for(&run, &mut scene, carl, donut, None)
            .unwrap()
            .story_holds(id)
    };
    let (carl, donut) = Loadout::Story.flags();
    assert!(
        !holds(carl, donut, SPECIAL_THROWN_CHARGE),
        "no dynamite yet (K1)"
    );
    assert!(
        !holds(carl, donut, crate::donut_prog::JUMP_REACH_BOOST),
        "armed, not held"
    );
    let (carl, donut) = Loadout::All.flags();
    assert!(holds(carl, donut, crate::donut_prog::JUMP_REACH_BOOST));
}

#[test]
fn the_rest_rooms_launch() {
    for room in RestRoom::ALL {
        let run = launch(&root(), room).unwrap();
        assert_eq!(run.fixture.name, room.scene_name());
        assert!(run.fixture.enemies.is_empty() && run.fixture.llamas.is_empty());
        assert_eq!(
            run.rules.checkpoints.len(),
            2,
            "the start column and the checkpoint"
        );
        assert!(
            entry_line(&run, &{
                let mut scene = scene_of(&run);
                combat_of(&run, &mut scene).unwrap()
            })
            .contains(&format!("{}_ENTRY", room.marker()))
        );
    }
}

/// A flagless walk-right smoke: nothing arms the reward, the door never opens, the exit is
/// not reached. The room is sealed by design (a bug-shaped result that is not one).
#[test]
fn rest_refuge_flagless_smoke_is_sealed() {
    let run = refuge();
    let mut scene = scene_of(&run);
    let mut combat = combat_of(&run, &mut scene).unwrap();
    for tick in 1..=SMOKE_TICKS {
        combat.step(&mut scene, run.fixture.input(tick));
    }
    assert!(!scene.world.complete, "the exit is not reached");
    assert_eq!(
        combat.env().unwrap().surface_locks_open(),
        [false],
        "the door stays closed"
    );
    assert!(scene.world.hero.x < 1440.0, "Carl stops at the door");
    // The same walk twice gives the same hash.
    let again = {
        let mut scene = scene_of(&run);
        let mut combat = combat_of(&run, &mut scene).unwrap();
        for tick in 1..=SMOKE_TICKS {
            combat.step(&mut scene, run.fixture.input(tick));
        }
        combat.state_hash(&scene)
    };
    assert_eq!(again, combat.state_hash(&scene));
}

/// Under the Story flags a scripted crossing (Donut active, the shelf jump) reaches the
/// exit: the box grants, the shelf opens the door, the exit is reached.
#[test]
fn rest_refuge_story_smoke_reaches_the_exit() {
    let o = crate::rest_fixtures::run(&root(), crate::rest_fixtures::Case::SoloDonut).unwrap();
    assert!(o.exit_tick.is_some());
    assert_eq!(o.grants.len(), 1);
    assert_eq!(o.doors.len(), 1);
}

/// The room's scenes and ids line up with the manifest's names.
#[test]
fn the_refuge_scene_names_match_the_manifest() {
    let manifest = crate::f01_late::Manifest::load(&root()).unwrap();
    let row = manifest.room(RestRoom::Refuge.id()).unwrap();
    assert_eq!(row.scene.name(), RestRoom::Refuge.scene_name());
    assert_eq!(row.title, "The Wrong Restaurant");
    assert_eq!(row.milestone, "greybox");
    assert_eq!(row.scene.rest(), Some(RestRoom::Refuge));
}
