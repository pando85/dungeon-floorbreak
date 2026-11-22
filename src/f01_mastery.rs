//! PT-MASTERY: the closing arena of the late-Floor-1 playtest slice
//! (`f01_mastery`, the room `mastery` of `--scene f01_late`, room 6 of
//! `docs/07-agents/f01-playtest-integration-plan.md`).
//!
//! Bevy-free. The room reuses **only the geometry** of the CF02-F microarena
//! (`assets/levels/floor01/cf02_microarena.room.json`, composed by
//! [`crate::cf02::arena::load_arena`]: floor 580, the left perch, the parapet,
//! the right perch, the rotten shore) and its wave contract (two waves, the
//! checkpoints 128 and 704, the exit that opens when the roster is clear). The
//! roster is not the arena's constants but the sidecar
//! `assets/levels/f01_late/mastery.encounter.json`, loaded and validated by
//! [`crate::encounter::load`] and spawned through ENEMY-SLOT's roster path: the
//! goblins, the Goblin Engineer and the two llamas are the fixture's enemies
//! ([`crate::f01_boiler::existing_enemies`], in roster order), and
//! [`crate::scene_enemy::spawn_from_roster`] builds any slot kind the roster
//! names (none yet, PTM-AD7).
//!
//! Adopted defaults of this step (`PTM-AD*`, listed in the PR):
//!
//! * AD1 the room is a CF02-rules room that keeps the shore (the arena's own
//!   environmental answer): `Fixture::shore_dx` is the arena's;
//! * AD2 the story pin is `f01_late_mastery` (every capability) and the room has
//!   no grant and no trigger: the final-boss rule, so a story run of it emits no
//!   `CapabilityGranted`;
//! * AD3 the exit is the sidecar's `exit` gate: it opens when every roster enemy
//!   is defeated (`goal: defeat_roster`, HAZ-Q39), the CF02-F "the exit opens on
//!   clear" contract, and stays a solid until then;
//! * AD4 the Juicer is not part of the arena (the plan, row 6);
//! * AD5 wave 2 puts the second llama at 820 and keeps the arena's llama home
//!   (CF02-Q29, 1000): the exit gate occupies 1072..1136, so a llama at 1050 would
//!   stand in it;
//! * AD6 the roster validator bounds a sidecar that opts in (`"waves": true`) per
//!   beat and in total ([`crate::encounter::MAX_ROSTER_WAVES`]; every other room
//!   keeps HAZ rule 5's cap of four), and accepts the llama as a roster kind (an
//!   existing kind, K16);
//! * AD7 the slime and the Rot Stickers are not in the sidecar yet: the HAZ
//!   pocket rule (rule 5) cannot hold in a crowd room (open question PTM-Q1).
//!
//! The room's content is pinned ([`ROOM_HASH`], [`ENCOUNTER_HASH`]): [`launch`]
//! refuses an edited file, a test pins each constant to its file, and an edit
//! is a deliberate re-pin here.

use std::path::Path;

use crate::carl_prog::CarlProgFlags;
use crate::cf02::arena;
use crate::combat_scene::{CombatState, DemoRules, EnvRuntime, Fixture, Hold};
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::donut_prog::DonutProgFlags;
use crate::encounter::{self, Loaded};
use crate::env_kit::EnvKit;
use crate::f01_boiler::{existing_enemies, story_setup};
use crate::input::RawInput;
use crate::scene_enemy::{SlotRoom, spawn_from_roster};

/// The `--scene` name (the fixture name of a launched room).
pub const SCENE_NAME: &str = "f01_mastery";
/// The room id of the manifest (`--f01-room mastery`).
pub const ROOM_NAME: &str = "mastery";
/// The sidecar, relative to the repository root.
pub const ENCOUNTER_PATH: &str = "assets/levels/f01_late/mastery.encounter.json";
/// FNV-1a 64 of the room file (the CF02-F microarena's geometry) and of the
/// sidecar. The files live under `assets/levels`, so the hashes are constants.
pub const ROOM_HASH: u64 = 0x4991_e0c7_5078_4c18;
pub const ENCOUNTER_HASH: u64 = 0xd5ef_f132_19c0_020a;
/// The `data_pins()` keys.
pub const ROOM_PIN_KEY: &str = "f01_mastery_room";
pub const ENCOUNTER_PIN_KEY: &str = "f01_mastery_encounter";
/// A fixture seed is only recorded: a live run draws its temperaments from
/// `--cf02-seed`.
const FIXTURE_SEED: u64 = 0x0F01_AA57;

/// The smoke script: hold right for this many ticks (a live run ignores it).
pub const SMOKE_TICKS: u64 = 600;
const WALK_RIGHT: &[Hold] = &[Hold {
    from: 0,
    to: SMOKE_TICKS,
    input: RawInput {
        move_x: 1.0,
        ..RawInput::NONE
    },
}];

/// A launched mastery arena: everything a scene or a fixture needs.
#[derive(Debug, Clone)]
pub struct MasteryRun {
    pub kit: EnvKit,
    pub fixture: &'static Fixture,
    /// Respawn columns: the arena's checkpoints (128 and 704).
    pub rules: &'static DemoRules,
    pub loaded: Loaded,
    /// The roster indices of the slot enemies (none yet), for the tests.
    pub slot_indices: Vec<usize>,
}

/// The env runtime of a launched room: the sidecar's exit gate (and the props it
/// would hold, none yet).
pub fn env_runtime(run: &MasteryRun) -> Result<EnvRuntime, String> {
    let enc = &run.loaded.encounter;
    Ok(EnvRuntime::new(enc)?.in_room(run.loaded.room.floor_y, &[], Some(enc.exit)))
}

/// Load the arena and its sidecar, compose its kit and build the fixture.
pub fn launch(root: &Path) -> Result<MasteryRun, String> {
    let loaded = encounter::load(root, ENCOUNTER_PATH).map_err(|errors| {
        format!(
            "{ENCOUNTER_PATH}: {}",
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        )
    })?;
    if loaded.room_hash != ROOM_HASH || loaded.sidecar_hash != ENCOUNTER_HASH {
        return Err(format!(
            "the room or its sidecar differs from the pinned content (room {:#018x} vs {ROOM_HASH:#018x}, \
             sidecar {:#018x} vs {ENCOUNTER_HASH:#018x}): re-pin src/f01_mastery.rs on purpose",
            loaded.room_hash, loaded.sidecar_hash
        ));
    }
    if !crate::story_unlock::StoryTable::active().has_room_pin(&loaded.encounter.story_pin) {
        return Err(format!(
            "{ENCOUNTER_PATH}: unknown story pin {:?}",
            loaded.encounter.story_pin
        ));
    }
    // The arena's own composer: the geometry is the CF02-F microarena's, with
    // its parapet, perches, lane ceiling and blink barriers.
    let kit = arena::load_arena(root)?;
    let room = &loaded.room;
    if kit.base_walk_line_y != room.floor_y || room.floor_y != arena::FLOOR_Y {
        return Err(format!(
            "the kit's walking line {} is not the room's floor {} (the arena's {})",
            kit.base_walk_line_y,
            room.floor_y,
            arena::FLOOR_Y
        ));
    }
    if room.spawn_carl != arena::CARL_SPAWN_X || room.spawn_donut != arena::DONUT_SPAWN_X {
        return Err("the arena's spawn columns moved: the sidecar's checkpoints are stale".into());
    }
    let checkpoint = loaded.encounter.checkpoint;
    if checkpoint.x + checkpoint.w / 2 != arena::CHECKPOINTS[1] {
        return Err(format!(
            "the sidecar's checkpoint column {} is not the arena's {}",
            checkpoint.x + checkpoint.w / 2,
            arena::CHECKPOINTS[1]
        ));
    }
    for x in arena::CHECKPOINTS {
        if kit.walking_line_at(x) != Some(kit.base_walk_line_y) {
            return Err(format!("column {x} is not on the walking line"));
        }
    }
    let slot_room = SlotRoom {
        floor_y: room.floor_y as f32,
        walls: (0.0, kit.width as f32),
    };
    let spawn =
        spawn_from_roster(&loaded.encounter, slot_room, false).map_err(|e| e.to_string())?;
    let enemies = existing_enemies(
        &loaded.encounter,
        &spawn.existing,
        crate::engineer::active().enabled(),
    )?;
    let slot_indices: Vec<usize> = (0..loaded.encounter.roster.len())
        .filter(|i| !spawn.existing.contains(i))
        .collect();
    let rules: &'static DemoRules = &arena::RULES;
    let fixture: &'static Fixture = Box::leak(Box::new(Fixture {
        name: SCENE_NAME,
        summary: "PT-MASTERY: the closing arena (CF02-F geometry, sidecar roster, full kits)",
        seed: FIXTURE_SEED,
        carl_x: arena::CARL_SPAWN_X,
        enemies: Box::leak(enemies.goblins.into_boxed_slice()),
        shore_dx: Some(arena::SHORE_STRIKE_X - arena::CARL_SPAWN_X),
        boss: None,
        llamas: Box::leak(enemies.llamas.into_boxed_slice()),
        engineers: Box::leak(enemies.engineers.into_boxed_slice()),
        charges: &[],
        script: WALK_RIGHT,
        ticks: SMOKE_TICKS,
    }));
    Ok(MasteryRun {
        kit,
        fixture,
        rules,
        loaded,
        slot_indices,
    })
}

/// The combat state of a launched room under a run's story flags and
/// `--cf02-seed`: the CF02 rules, the story pin applied under the program
/// flags, the env step and the slot enemies of the roster. The binary's scene and the PLAYTRACE
/// verifier both call it through [`crate::f01_late::mastery_start`]. The seed is
/// applied **before** the slot, like the hazard room: a builder that resets the
/// attempt after the slot would mark a slot enemy as retried.
pub fn combat_for(
    run: &MasteryRun,
    scene: &mut SceneSim,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
    cf02_seed: Option<u64>,
) -> Result<CombatState, String> {
    let room = SlotRoom {
        floor_y: scene.world.hero_foot_y(),
        walls: (0.0, run.kit.width as f32),
    };
    let spawn = spawn_from_roster(&run.loaded.encounter, room, false).map_err(|e| e.to_string())?;
    let setup = story_setup(&run.loaded.encounter.story_pin, carl, donut)?;
    let mut combat = CombatState::new(run.fixture, &run.kit, scene)?
        .with_donut(DonutSetup::unlocked(8))
        .try_with_story(setup)
        .map_err(|e| e.to_string())?
        .with_demo(run.rules)
        .with_cf02()
        .with_cf02_seed(cf02_seed)
        .with_env(env_runtime(run)?)
        .with_slot_enemies(spawn.slot)
        .map_err(|e| e.to_string())?;
    combat.sync_env(scene);
    Ok(combat)
}

/// [`combat_for`] under no story flag and no seed (the fixtures' state).
pub fn combat_of(run: &MasteryRun, scene: &mut SceneSim) -> Result<CombatState, String> {
    combat_for(
        run,
        scene,
        CarlProgFlags::default(),
        DonutProgFlags::default(),
        None,
    )
}

/// The console line of a launch.
pub fn entry_line(run: &MasteryRun) -> String {
    let e = &run.loaded.encounter;
    let room = &run.loaded.room;
    format!(
        "F01_MASTERY_ENTRY room={} encounter={} pin={} spawn_carl={} spawn_donut={} \
         checkpoints={:?} goal={:?} roster={} goblins={} engineers={} llamas={} slot={} env=live \
         room_hash={:#018x} sidecar_hash={:#018x}",
        e.room_id,
        e.id,
        e.story_pin,
        room.spawn_carl,
        room.spawn_donut,
        run.rules.checkpoints,
        e.goal,
        e.roster.len(),
        run.fixture.enemies.len(),
        run.fixture.engineers.len(),
        run.fixture.llamas.len(),
        run.slot_indices.len(),
        run.loaded.room_hash,
        run.loaded.sidecar_hash,
    )
}

/// The pins `playtrace::live::data_pins` stamps.
pub const fn data_pins() -> [(&'static str, u64); 2] {
    [
        (ROOM_PIN_KEY, ROOM_HASH),
        (ENCOUNTER_PIN_KEY, ENCOUNTER_HASH),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::EnemyAi;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn hash(path: &str) -> u64 {
        crate::donut_mana::content_hash(
            &std::fs::read_to_string(root().join(path)).expect("the file reads"),
        )
    }

    #[test]
    fn the_pinned_hashes_are_the_files() {
        assert_eq!(hash(arena::ROOM_PATH), ROOM_HASH);
        assert_eq!(hash(ENCOUNTER_PATH), ENCOUNTER_HASH);
        let run = launch(&root()).expect("the room launches");
        assert_eq!(run.loaded.room_hash, ROOM_HASH);
        assert_eq!(run.loaded.sidecar_hash, ENCOUNTER_HASH);
        for (key, value) in data_pins() {
            assert_ne!(value, 0, "{key}");
        }
    }

    /// An edited sidecar is refused until it is re-pinned on purpose.
    #[test]
    fn an_unpinned_edit_is_refused() {
        let dir = std::env::temp_dir().join(format!("ptm-edit-{}", std::process::id()));
        let level = dir.join("assets/levels/f01_late");
        std::fs::create_dir_all(&level).unwrap();
        std::fs::create_dir_all(dir.join("assets/levels/floor01")).unwrap();
        std::fs::create_dir_all(dir.join("assets/data/gameplay")).unwrap();
        let text = std::fs::read_to_string(root().join(ENCOUNTER_PATH)).unwrap();
        std::fs::write(
            dir.join(ENCOUNTER_PATH),
            text.replace("\"x\": 520", "\"x\": 524"),
        )
        .unwrap();
        for rel in [arena::ROOM_PATH, crate::encounter::HEIGHT_CLASSES_PATH] {
            std::fs::copy(root().join(rel), dir.join(rel)).unwrap();
        }
        let error = launch(&dir).expect_err("an edited sidecar is refused");
        std::fs::remove_dir_all(&dir).ok();
        assert!(error.contains("re-pin src/f01_mastery.rs"), "{error}");
    }

    #[test]
    fn the_room_is_the_microarena_geometry_with_the_cf02_wave_contract() {
        let run = launch(&root()).unwrap();
        // Geometry: the arena's kit, floor, spawns and checkpoints.
        assert_eq!(run.kit.layout_id, arena::LAYOUT_ID);
        assert_eq!(run.kit.base_walk_line_y, arena::FLOOR_Y);
        assert_eq!(run.rules.checkpoints, arena::CHECKPOINTS);
        assert_eq!(run.fixture.carl_x, arena::CARL_SPAWN_X);
        assert_eq!(
            run.fixture.shore_dx,
            Some(arena::SHORE_STRIKE_X - arena::CARL_SPAWN_X),
            "PTM-AD1: the shore"
        );
        // The wave contract: wave 1 is the arena's three brawl-lane columns,
        // wave 2 the yard: a goblin, the Engineer and two llamas, the arena's
        // llama home among them.
        let goblins: Vec<i32> = run.fixture.enemies.iter().map(|e| e.x).collect();
        assert_eq!(goblins, [520, 600, 680, 880]);
        assert_eq!(&goblins[..3], &[520, 600, 680], "CF02-Q29 wave 1");
        assert!(run.fixture.enemies.iter().all(|e| e.ai == EnemyAi::Chase));
        assert_eq!(
            run.fixture
                .engineers
                .iter()
                .map(|e| e.x)
                .collect::<Vec<_>>(),
            [940]
        );
        let llamas: Vec<i32> = run.fixture.llamas.iter().map(|l| l.x).collect();
        assert_eq!(llamas, [820, 1000]);
        assert!(llamas.contains(&arena::LLAMA_HOME_X), "CF02-Q29 llama home");
        // PTM-AD5: nothing stands in the exit gate, and the exit is the arena's
        // goal column.
        let gate = run.loaded.encounter.exit;
        assert_eq!((gate.x, gate.right()), (1072, 1136));
        for x in goblins.iter().copied().chain(llamas.iter().copied()) {
            assert!(x + 24 <= gate.x, "{x} stands in the exit gate");
        }
        // The Juicer is not part of the arena, and the story gives it no grant.
        assert!(run.loaded.encounter.roster.iter().all(|e| !matches!(
            e.kind,
            encounter::RosterKind::Room(crate::camera_lab::room::EnemyKind::Juicer)
        )));
        assert_eq!(run.loaded.encounter.story_pin, "f01_late_mastery");
        assert!(run.loaded.encounter.goal == encounter::Goal::DefeatRoster);
    }

    #[test]
    fn the_gate_holds_the_exit_until_every_roster_enemy_is_defeated() {
        let run = launch(&root()).unwrap();
        let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
        let combat = combat_of(&run, &mut scene).unwrap();
        assert!(!combat.roster_cleared());
        let env = combat.env().expect("the env step holds the exit gate");
        assert_eq!(
            env.solids().len(),
            1,
            "the exit gate is the one runtime solid"
        );
        // Four goblins (one of them the Engineer is counted apart), two llamas.
        assert_eq!(combat.enemies.len(), 5, "four goblins and the Engineer");
        assert_eq!(combat.enemies.iter().filter(|g| g.is_engineer()).count(), 1);
        assert_eq!(combat.llamas.len(), 2);
        assert_eq!(combat.slot_enemies().len(), 0, "PTM-AD7: no slot enemy yet");
    }

    #[test]
    fn the_seed_draws_three_goblin_temperaments() {
        let run = launch(&root()).unwrap();
        let temperaments = |seed: u64| {
            let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
            let combat = combat_for(
                &run,
                &mut scene,
                CarlProgFlags::default(),
                DonutProgFlags::default(),
                Some(seed),
            )
            .unwrap();
            (0..combat.enemies.len())
                .map(|g| format!("{:?}", combat.goblin_morale(g).unwrap().temperament()))
                .collect::<Vec<_>>()
        };
        let mut distinct_seen = 0;
        for seed in 0..9 {
            let drawn = temperaments(seed);
            assert_eq!(drawn, temperaments(seed), "seed {seed} replays");
            let mut kinds = drawn.clone();
            kinds.sort_unstable();
            kinds.dedup();
            if kinds.len() >= 3 {
                distinct_seen += 1;
            }
        }
        assert!(
            distinct_seen > 0,
            "some seed of 0..9 draws all three temperaments"
        );
    }

    #[test]
    fn the_room_replays_to_the_same_hash_twice_and_the_seed_is_part_of_it() {
        let run_once = |seed: Option<u64>| {
            let run = launch(&root()).unwrap();
            let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
            let mut combat = combat_for(
                &run,
                &mut scene,
                CarlProgFlags::default(),
                DonutProgFlags::default(),
                seed,
            )
            .unwrap();
            for tick in 1..=400u64 {
                combat.step(&mut scene, run.fixture.input(tick.min(SMOKE_TICKS)));
            }
            combat.state_hash(&scene)
        };
        assert_eq!(run_once(None), run_once(None));
        assert_eq!(run_once(Some(3)), run_once(Some(3)));
        assert_ne!(run_once(Some(3)), run_once(Some(4)));
    }

    /// The story pin applies under the program flags and nothing without them;
    /// the fixtures' state is the unflagged one.
    #[test]
    fn the_story_flags_apply_the_full_kit_pin() {
        use crate::story_unlock::SPECIAL_THROWN_CHARGE;
        let run = launch(&root()).unwrap();
        let state = |carl: CarlProgFlags, donut: DonutProgFlags| {
            let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
            let combat = combat_for(&run, &mut scene, carl, donut, None).unwrap();
            (
                combat.story_holds(SPECIAL_THROWN_CHARGE),
                combat.state_hash(&scene),
            )
        };
        let (story_carl, story_donut) = crate::f01_late::Loadout::Story.flags();
        let (plain_holds, plain_hash) = state(CarlProgFlags::default(), DonutProgFlags::default());
        let (story_holds, story_hash) = state(story_carl, story_donut);
        assert!(!plain_holds && story_holds, "the pin holds the dynamite");
        assert_ne!(plain_hash, story_hash);
        let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
        assert_eq!(
            combat_of(&run, &mut scene).unwrap().state_hash(&scene),
            plain_hash
        );
    }

    #[test]
    fn the_entry_line_names_the_roster_and_the_pins() {
        let run = launch(&root()).unwrap();
        let line = entry_line(&run);
        for part in [
            "F01_MASTERY_ENTRY",
            "pin=f01_late_mastery",
            "goblins=4 engineers=1 llamas=2 slot=0",
            "env=live",
        ] {
            assert!(line.contains(part), "{line}");
        }
    }
}
