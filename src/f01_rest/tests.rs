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

fn camp() -> RestRun {
    launch(&root(), RestRoom::Camp).expect("the camp launches")
}

fn aftermath() -> RestRun {
    launch(&root(), RestRoom::Aftermath).expect("the aftermath launches")
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

/// `jump_reach_boost` requires `primary_ranged`: Donut starts with the biscuit in all three
/// rooms, or the refuge's trigger would be spent without granting and seal the room on every
/// restart (and the aftermath's blink lane would have no Donut mana to spend).
#[test]
fn rest_refuge_starts_with_the_biscuit() {
    let (carl, donut) = Loadout::Story.flags();
    for room in RestRoom::ALL {
        let run = launch(&root(), room).unwrap();
        let mut scene = scene_of(&run);
        for flags in [(carl, donut), Default::default()] {
            let combat = combat_for(&run, &mut scene, flags.0, flags.1, None).unwrap();
            assert!(
                combat.donut_mana_view().is_some(),
                "{}: Donut's mana is unlocked at tick 0",
                room.id()
            );
            assert!(combat.story_holds(crate::story_unlock::PRIMARY_RANGED));
        }
    }
}

/// REST-B and REST-C: both sidecars validate (an empty roster under `reach_exit`, no lock,
/// no gated surface, the room pins of the story table, no ceiling slab and no surface above
/// the 42 px free height), and the camp's crates and the aftermath's bench are decor only
/// (REST-IMP-8: the validator's lane rule rejects a 32 px or 24 px deck).
#[test]
fn rest_camp_and_aftermath_sidecars_validate() {
    for (run, id, pin) in [
        (camp(), "F01-REST-CAMP-ENC", "f01_pin_post_pile_boss"),
        (aftermath(), "F01-REST-AFTERMATH-ENC", "f01_late_mastery"),
    ] {
        let loaded = &run.loaded;
        assert_eq!(loaded.encounter.id, id);
        assert_eq!(loaded.encounter.goal, crate::encounter::Goal::ReachExit);
        assert!(loaded.encounter.roster.is_empty());
        assert_eq!(loaded.encounter.story_pin, pin);
        assert!(StoryTable::active().has_room_pin(pin), "{pin}: a room pin");
        assert!(loaded.encounter.locks.is_empty(), "no lock");
        assert!(run.door().is_none());
        assert_eq!(loaded.ceiling_px, None, "no lane ceiling");
        assert!(loaded.room.platforms.is_empty(), "no surface at all");
        assert!(
            loaded.encounter.effective_gates.is_empty() && loaded.encounter.vantages.is_empty()
        );
        // No slab in the kit: no full-width solid hangs in the lane (the 32 px top wall of
        // the room's frame is the only full-width solid above the floor).
        assert!(
            !run.kit
                .solid_rects
                .iter()
                .any(|s| s.w >= run.kit.width && s.y > 0 && s.y + s.h < run.kit.base_walk_line_y),
            "no ceiling slab"
        );
    }
    // The heavy bag is an inert static on the walking line, mirrored by the decor rectangle
    // (REST-AD12: never solid, never blocking).
    let run = aftermath();
    let statics = &run.loaded.encounter.statics;
    assert_eq!(statics.len(), 1);
    assert_eq!(statics[0].id, "heavy_bag");
    assert_eq!(statics[0].rect.bottom(), run.loaded.room.floor_y);
    let bag = decor(RestRoom::Aftermath)
        .iter()
        .find(|d| d.label == "heavy_bag")
        .expect("the bag decor");
    assert_eq!(bag.rect, statics[0].rect);
    let env = combat_of(&run, &mut scene_of(&run)).unwrap();
    assert!(
        env.env().unwrap().solids().is_empty(),
        "the aftermath has no env solid: the bag never blocks"
    );
}

/// The decor tables of the camp and the aftermath stand on the floor inside their room, with
/// unique labels, and sit where the design puts them (the checkpoint under the central
/// light, the blink lane's two floor marks 112 px apart, chalk over the last 128 px).
#[test]
fn rest_camp_and_aftermath_decor_sits_where_the_design_puts_it() {
    for room in RestRoom::ALL {
        let run = launch(&root(), room).unwrap();
        let mut labels: Vec<&str> = decor(room).iter().map(|d| d.label).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), decor(room).len(), "{}", room.id());
        for d in decor(room) {
            assert!(
                d.rect.x >= 0 && d.rect.right() <= run.kit.width,
                "{} {}",
                room.id(),
                d.label
            );
        }
    }
    let rect = |room, label: &str| {
        decor(room)
            .iter()
            .find(|d| d.label == label)
            .unwrap_or_else(|| panic!("{label}"))
            .rect
    };
    let camp = camp();
    let checkpoint = camp.loaded.encounter.checkpoint;
    let light = rect(RestRoom::Camp, "camp_light");
    assert!(
        light.x >= checkpoint.x && light.right() <= checkpoint.right(),
        "the checkpoint sits under the central light"
    );
    let chalk = rect(RestRoom::Camp, "chalk");
    assert_eq!(chalk.w, 128);
    assert_eq!(
        chalk.right(),
        camp.loaded.encounter.exit.x,
        "the last 128 px"
    );
    let left = rect(RestRoom::Aftermath, "blink_lane_left");
    let right = rect(RestRoom::Aftermath, "blink_lane_right");
    assert_eq!(left.x, BLINK_LANE_X.0);
    assert_eq!(right.right(), BLINK_LANE_X.1);
    assert_eq!(BLINK_LANE_X.1 - BLINK_LANE_X.0, 112, "the 112 px blink");
    for room in [RestRoom::Camp, RestRoom::Aftermath] {
        for d in decor(room) {
            assert!(
                d.rect.bottom() <= 580,
                "{}: {} stands on or above the walking line",
                room.id(),
                d.label
            );
        }
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

/// The camp and the aftermath are not sealed: a flagless walk right reaches the exit under
/// the plain constructor (no flag, no door, no grant), and the same walk twice gives the same
/// hash.
#[test]
fn rest_camp_and_aftermath_flagless_smokes_reach_the_exit() {
    for room in [RestRoom::Camp, RestRoom::Aftermath] {
        let hash = || {
            let run = launch(&root(), room).unwrap();
            let mut scene = scene_of(&run);
            let mut combat = combat_of(&run, &mut scene).unwrap();
            for tick in 1..=SMOKE_TICKS {
                combat.step(&mut scene, run.fixture.input(tick));
            }
            assert!(scene.world.complete, "{}: the exit is reached", room.id());
            assert!(
                !combat
                    .events
                    .iter()
                    .any(|e| matches!(e, crate::combat::CombatEvent::CapabilityGranted { .. })),
                "{}: a quiet room grants nothing",
                room.id()
            );
            combat.state_hash(&scene)
        };
        assert_eq!(hash(), hash(), "{}", room.id());
    }
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

/// The rooms' scenes and ids line up with the manifest's names.
#[test]
fn the_rest_scene_names_match_the_manifest() {
    let manifest = crate::f01_late::Manifest::load(&root()).unwrap();
    for (room, title) in [
        (RestRoom::Refuge, "The Wrong Restaurant"),
        (RestRoom::Camp, "The Survivors' Camp"),
        (RestRoom::Aftermath, "After the Juicer"),
    ] {
        let row = manifest.room(room.id()).unwrap();
        assert_eq!(row.scene.name(), room.scene_name());
        assert_eq!(row.title, title);
        assert_eq!(row.milestone, "greybox");
        assert_eq!(row.scene.rest(), Some(room));
    }
}
