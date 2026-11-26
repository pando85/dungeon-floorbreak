//! DOZER-CORE: room 3 of the late-Floor-1 slice, the revenge Dozer's chase
//! (`f01_dozer`, `docs/01-design/enemy-gameplay-dozer.md` section 5).
//!
//! Bevy-free. DR-A wires it: `--scene f01_dozer`, the manifest row `dozer` and
//! the PLAYTRACE registration; the drawing is `crate::dozer_art`.
//! [`launch`] loads the room and its encounter sidecar through the one room
//! parser ([`crate::encounter::load`], which runs the whole validation, the
//! Dozer rules of section 9 included), composes the kit exactly as the Boiler
//! does ([`crate::f01_boiler::compose_kit`]), adds the lane ceiling slab and the
//! tunnel lintel (the sidecar's `dozer` block, DZ-Q22) to the solids, and builds
//! the combat fixture and the checkpoint rules. [`combat`] then builds the
//! combat state the fixtures share; **the machine spawns from the sidecar roster
//! through ENEMY-SLOT** (the `dozer` kind registered in
//! [`crate::scene_enemy::SLOT_KINDS`]).
//!
//! The room's pin is the `F01Late` row the sidecar names
//! (`f01_pin_post_pile_boss`), chosen by id; the room arms no trigger. DR-B adds
//! the stop frame: the sidecar's `weight_rig` prop and its `falling_mass`
//! hazard run on the env runtime ([`env_runtime`]). DR-C adds the crew: the
//! roster's two mounted goblins (`rider_a`, `rider_b`, DZ-Q20) are the fixture's
//! enemies 0 and 1, carried aboard the machine from tick 0
//! ([`CombatState::with_crew`]) and landing on the tick it is disabled.
//!
//! The room's content is pinned ([`ROOM_HASH`], [`ENCOUNTER_HASH`]): [`launch`]
//! refuses an edited file, a test pins each constant to its file, and an edit
//! is a deliberate re-pin here.

use std::path::Path;

use crate::carl_prog::CarlProgFlags;
use crate::combat_scene::{CombatState, DemoRules, Fixture, Hold};
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::donut_prog::DonutProgFlags;
use crate::encounter::{self, Loaded};
use crate::env_kit::{EnvKit, IRect as KitRect};
use crate::input::RawInput;
use crate::scene_enemy::{SlotRoom, SlotSet, spawn_from_roster};
use crate::story_unlock::{SceneKind, StorySetup, StoryTable};

/// The `--scene` name of the room's fixtures.
pub const SCENE_NAME: &str = "f01_dozer";
/// The id of the room's scenario in its file.
pub const ROOM_ID: &str = "F01-DOZER";
pub const ROOM_PATH: &str = "assets/levels/floor01/f01_dozer.room.json";
pub const ENCOUNTER_PATH: &str = "assets/levels/floor01/f01_dozer.encounter.json";
/// FNV-1a 64 of the room file and of the sidecar (the files live under
/// `assets/levels`, so the hash is a constant, as in [`crate::f01_hoarder`]).
pub const ROOM_HASH: u64 = 0xaf54_bc91_c52d_6f83;
pub const ENCOUNTER_HASH: u64 = 0x3dd3_2eec_2779_bdda;
/// The data pin keys of the room's files.
pub const ROOM_PIN_KEY: &str = "f01_dozer_room";
pub const ENCOUNTER_PIN_KEY: &str = "f01_dozer_encounter";
/// The pin the sidecar names: after the Hoarder (the dynamite is held).
pub const PIN_POST_PILE_BOSS: &str = "f01_pin_post_pile_boss";
/// A fixture seed is only recorded (no rule reads it in this room).
const FIXTURE_SEED: u64 = 0x0F01_D0E3;

/// DR-A: the smoke script, hold right for this many ticks (a live run ignores
/// it). The walk crosses the trigger, so the smoke runs the rumble and the chase.
pub const SMOKE_TICKS: u64 = 600;
const WALK_RIGHT: &[Hold] = &[Hold {
    from: 0,
    to: SMOKE_TICKS,
    input: RawInput {
        move_x: 1.0,
        ..RawInput::NONE
    },
}];

/// A launched room 3: everything a scene or a fixture needs.
#[derive(Debug, Clone)]
pub struct DozerRun {
    pub kit: EnvKit,
    pub fixture: &'static Fixture,
    /// Respawn columns: the start column, then the checkpoint's when it is
    /// right of the start.
    pub rules: &'static DemoRules,
    pub loaded: Loaded,
    /// DR-C: the mount of each fixture enemy (all of them are riders).
    pub crew: Vec<Option<encounter::Mount>>,
}

/// Load the room, compose its kit (with the ceiling slab and the lintel) and
/// build the fixture and rules.
pub fn launch(root: &Path) -> Result<DozerRun, String> {
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
             sidecar {:#018x} vs {ENCOUNTER_HASH:#018x}): re-pin src/f01_dozer.rs on purpose",
            loaded.room_hash, loaded.sidecar_hash
        ));
    }
    if loaded.encounter.room_id != ROOM_ID {
        return Err(format!(
            "{ENCOUNTER_PATH}: the sidecar names room {:?}, not {ROOM_ID}",
            loaded.encounter.room_id
        ));
    }
    if !StoryTable::active().has_room_pin(&loaded.encounter.story_pin) {
        return Err(format!(
            "{ENCOUNTER_PATH}: unknown or non-room story pin {:?}",
            loaded.encounter.story_pin
        ));
    }
    let Some(site) = loaded.encounter.dozer.clone() else {
        return Err(format!("{ENCOUNTER_PATH}: no dozer block"));
    };
    let mut kit = crate::f01_boiler::compose_kit(root, &loaded.room)?;
    if kit.base_walk_line_y != loaded.room.floor_y {
        return Err(format!(
            "the kit's walking line {} is not the room's floor {}",
            kit.base_walk_line_y, loaded.room.floor_y
        ));
    }
    // DP-Q45: the lane ceiling slab of the room (`ceiling_px` above the line).
    if let Some(ceiling) = loaded.ceiling_px {
        kit.solid_rects.push(KitRect {
            x: 0,
            y: kit.base_walk_line_y - ceiling - 16,
            w: kit.width,
            h: 16,
        });
    }
    // DZ-Q22: the tunnel lintel, a solid from the passage's clear height up to
    // the ceiling. It stops nothing but a jump: the machine ignores solids and
    // brakes at `stop_x`.
    let Some(l) = site.lintel else {
        return Err(format!(
            "{ENCOUNTER_PATH}: room 3's dozer block has no lintel"
        ));
    };
    kit.solid_rects.push(KitRect {
        x: l.x,
        y: l.y,
        w: l.w,
        h: l.h,
    });
    // DP-Q35: the room's optional blink barriers (search only, never collision).
    kit.blink_barriers = loaded.room.kit_blink_barriers();
    let start = loaded.room.spawn_carl;
    let checkpoint = loaded.encounter.checkpoint;
    let checkpoint_x = checkpoint.x + checkpoint.w / 2;
    let columns: Vec<i32> = if checkpoint_x > start {
        vec![start, checkpoint_x]
    } else {
        vec![start]
    };
    for x in &columns {
        if kit.walking_line_at(*x) != Some(kit.base_walk_line_y) {
            return Err(format!("column {x} is not on the walking line"));
        }
    }
    // DR-C: the crew. Every existing-kind entry of the roster is a mounted
    // goblin; it stands at its roster column (parked off-room, carried).
    let mut riders: Vec<crate::combat_scene::EnemySpawn> = Vec::new();
    let mut crew = Vec::new();
    for entry in &loaded.encounter.roster {
        if entry.kind == encounter::RosterKind::Dozer {
            continue;
        }
        let Some(mount) = entry.mount else {
            return Err(format!(
                "{ENCOUNTER_PATH}: roster {} is neither the machine nor a rider",
                entry.id
            ));
        };
        riders.push(crate::combat_scene::EnemySpawn {
            x: entry.x,
            ai: crate::combat::EnemyAi::Chase,
        });
        crew.push(Some(mount));
    }
    // One small leak per launch: the combat state borrows its rules for `'static`.
    let rules: &'static DemoRules = Box::leak(Box::new(DemoRules {
        checkpoints: Box::leak(columns.into_boxed_slice()),
    }));
    let fixture: &'static Fixture = Box::leak(Box::new(Fixture {
        name: SCENE_NAME,
        summary: "DOZER-CORE, DR-A, DR-B and DR-C: room 3, the revenge machine's chase, the stop frame and the crew (greybox)",
        seed: FIXTURE_SEED,
        carl_x: start,
        enemies: Box::leak(riders.into_boxed_slice()),
        shore_dx: None,
        boss: None,
        llamas: &[],
        engineers: &[],
        charges: &[],
        script: WALK_RIGHT,
        ticks: SMOKE_TICKS,
        room: crate::combat_scene::FixtureRoom::Corridor,
    }));
    Ok(DozerRun {
        kit,
        fixture,
        rules,
        loaded,
        crew,
    })
}

/// The story setup of a run of this room: the rows of the pin the sidecar names
/// (each under its own program flag). The room arms no trigger (it runs on the
/// `F01Late` label). With no flag it is [`StorySetup::NONE`].
pub fn story_setup(
    pin: &str,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
) -> Result<StorySetup, String> {
    if !StoryTable::active().has_room_pin(pin) {
        return Err(format!("unknown or non-room story pin {pin:?}"));
    }
    Ok(crate::donut_prog::story_setup_pin(
        carl,
        donut,
        SceneKind::F01Late,
        pin,
    ))
}

/// The slot room facts of a launched scene (the same values the combat state
/// hands its slot enemies).
pub fn slot_room(run: &DozerRun, scene: &SceneSim) -> SlotRoom {
    SlotRoom {
        floor_y: scene.world.hero_foot_y(),
        walls: (0.0, run.kit.width as f32),
    }
}

/// The machine through ENEMY-SLOT; the roster's existing kinds must be the
/// crew [`launch`] already put in the fixture.
fn slot_spawn(run: &DozerRun, scene: &SceneSim) -> Result<crate::scene_enemy::RosterSpawn, String> {
    let spawn = spawn_from_roster(&run.loaded.encounter, slot_room(run, scene), false)
        .map_err(|e| e.to_string())?;
    if spawn.existing.len() != run.crew.len()
        || spawn
            .existing
            .iter()
            .any(|&i| run.loaded.encounter.roster[i].mount.is_none())
    {
        return Err("room 3's roster holds an existing kind that is not a rider".to_owned());
    }
    Ok(spawn)
}

/// The room's combat state at tick 0: the fixture, Donut with the biscuit and a
/// full pool, the sidecar's pin under the program flags, the checkpoint rules,
/// the CF02 rules and **the machine, spawned from the sidecar roster through
/// ENEMY-SLOT**.
pub fn combat(
    run: &DozerRun,
    scene: &mut SceneSim,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
) -> Result<CombatState, String> {
    let spawn = slot_spawn(run, scene)?;
    combat_with(run.fixture, run, scene, carl, donut, spawn.slot)
}

/// [`combat`] with an explicit fixture and slot set. The slot is installed
/// **last**: a `with_*` builder that resets the attempt after the slot would
/// mark it as retried.
pub fn combat_with(
    fixture: &'static Fixture,
    run: &DozerRun,
    scene: &mut SceneSim,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
    slot: SlotSet,
) -> Result<CombatState, String> {
    combat_seeded(fixture, run, scene, carl, donut, slot, None)
}

/// [`combat_with`] under a CF02 seed (the companion's draws; the fixtures'
/// matrix). The seed is set before the slot, which is installed last.
pub fn combat_seeded(
    fixture: &'static Fixture,
    run: &DozerRun,
    scene: &mut SceneSim,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
    slot: SlotSet,
    cf02_seed: Option<u64>,
) -> Result<CombatState, String> {
    combat_seeded_kit(
        fixture,
        run,
        scene,
        (carl, donut, DonutSetup::unlocked(8)),
        slot,
        cf02_seed,
    )
}

/// [`combat_seeded`] with Donut's kit given (DR-B: `dz_frame_missile` starts
/// one run without the biscuit, the Magic Missile's capability).
pub fn combat_seeded_kit(
    fixture: &'static Fixture,
    run: &DozerRun,
    scene: &mut SceneSim,
    (carl, donut, kit): (CarlProgFlags, DonutProgFlags, DonutSetup),
    slot: SlotSet,
    cf02_seed: Option<u64>,
) -> Result<CombatState, String> {
    let setup = story_setup(&run.loaded.encounter.story_pin, carl, donut)?;
    let mut combat = CombatState::new(fixture, &run.kit, scene)?
        .with_donut(kit)
        .try_with_story(setup)
        .map_err(|e| e.to_string())?
        .with_demo(run.rules)
        .with_cf02();
    if cf02_seed.is_some() {
        combat = combat.with_cf02_seed(cf02_seed);
    }
    // DR-B: the stop frame is live (the env runtime before the slot, which a
    // builder after it would mark as retried). DR-C: the crew rides from tick 0.
    let mut combat = combat
        .with_crew(run.crew.clone())
        .with_env(env_runtime(run)?)
        .with_slot_enemies(slot)
        .map_err(|e| e.to_string())?;
    combat.sync_env(scene);
    Ok(combat)
}

/// DR-B: the env runtime of a launched room 3, the sidecar's stop frame (a
/// `weight_rig` prop and its `falling_mass` hazard, DZ-Q18, DZ-Q19) and its
/// remnant solid once it has landed. The remnant is out of the partner's nav
/// until the landing (section 6: no remnant before the fall; #287 review F1).
pub fn env_runtime(run: &DozerRun) -> Result<crate::combat_scene::EnvRuntime, String> {
    Ok(crate::combat_scene::EnvRuntime::new(&run.loaded.encounter)?.with_remnants_on_landing())
}

/// DR-B: the index of the stop frame among the sidecar's props.
pub const STOP_FRAME_PROP: usize = 0;

/// DR-B: the live stop frame as [`crate::dozer::release_window`] reads it: the
/// footprint and the landing delay (warning + fall − 1) of the env's falling
/// mass, and the machine's emergence front. `None` without an env or a machine.
pub fn stop_frame_of(combat: &CombatState) -> Option<crate::dozer::StopFrame> {
    let d = crate::dozer_art::dozer_of(combat)?;
    let cfg = combat.env()?.mass(STOP_FRAME_PROP)?.config();
    Some(crate::dozer::StopFrame {
        footprint_x0: cfg.footprint.x as f32,
        footprint_x1: cfg.footprint.right() as f32,
        land_delay_ticks: (u32::from(cfg.warning_ticks) + u32::from(cfg.fall_ticks))
            .saturating_sub(1),
        emerge_front_x: d.site.emerge_front_x as f32,
    })
}

/// The room's combat state for a scene or a verifier (DR-A): [`combat`] under
/// a CF02 seed (the companion's draws). The binary and the PLAYTRACE verifier
/// both reach it through [`crate::f01_late::dozer_start`].
pub fn combat_for(
    run: &DozerRun,
    scene: &mut SceneSim,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
    cf02_seed: Option<u64>,
) -> Result<CombatState, String> {
    let spawn = slot_spawn(run, scene)?;
    combat_seeded(run.fixture, run, scene, carl, donut, spawn.slot, cf02_seed)
}

/// Launch the room and build its scene and combat state.
pub fn start(
    root: &Path,
    view_height: i32,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
) -> Result<(DozerRun, SceneSim, CombatState), String> {
    let run = launch(root)?;
    let mut scene = SceneSim::new_at(&run.kit, view_height, run.fixture.carl_x)?;
    let state = combat(&run, &mut scene, carl, donut)?;
    Ok((run, scene, state))
}

/// The pins a live run would stamp (the room, the sidecar and the machine's
/// tuning), registered in `playtrace::live::data_pins` and scoped to the
/// `dozer` room by [`crate::f01_late::pin_applies`] (DR-A).
pub const fn data_pins() -> [(&'static str, u64); 3] {
    [
        (ROOM_PIN_KEY, ROOM_HASH),
        (ENCOUNTER_PIN_KEY, ENCOUNTER_HASH),
        (crate::dozer::PIN_KEY, crate::dozer::TUNING_HASH),
    ]
}

/// The console line of a launch: identity, pin, spawn columns, the machine's
/// site and hashes.
pub fn entry_line(run: &DozerRun) -> String {
    let e = &run.loaded.encounter;
    let room = &run.loaded.room;
    let site = e
        .dozer
        .as_ref()
        .map(|s| (s.trigger_x, s.emerge_front_x, s.stop_x));
    format!(
        "F01_DOZER_ENTRY room={} encounter={} pin={} spawn_carl={} spawn_donut={} \
         checkpoints={:?} goal={:?} roster={} dozer={:?} room_hash={:#018x} sidecar_hash={:#018x}",
        e.room_id,
        e.id,
        e.story_pin,
        room.spawn_carl,
        room.spawn_donut,
        run.rules.checkpoints,
        e.goal,
        e.roster.len(),
        site,
        run.loaded.room_hash,
        run.loaded.sidecar_hash,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn hash_of(path: &str) -> u64 {
        crate::donut_mana::content_hash(&std::fs::read_to_string(root().join(path)).unwrap())
    }

    #[test]
    fn the_pinned_hashes_are_the_files() {
        let room = hash_of(ROOM_PATH);
        let sidecar = hash_of(ENCOUNTER_PATH);
        assert_eq!(room, ROOM_HASH, "{room:#018x}");
        assert_eq!(sidecar, ENCOUNTER_HASH, "{sidecar:#018x}");
    }

    #[test]
    fn the_room_launches_with_the_machine_parked() {
        let (run, scene, combat) = start(
            &root(),
            360,
            CarlProgFlags::default(),
            DonutProgFlags::default(),
        )
        .unwrap();
        assert_eq!(run.loaded.encounter.story_pin, PIN_POST_PILE_BOSS);
        let line = entry_line(&run);
        assert!(
            line.starts_with("F01_DOZER_ENTRY room=F01-DOZER ") && line.contains("roster=3"),
            "{line}"
        );
        for (key, value) in data_pins() {
            assert_ne!(value, 0, "{key}");
            // DR-A: registered for a live recording and scoped to room 3 only.
            assert!(
                crate::playtrace::live::data_pins().contains(&(key, value)),
                "{key}"
            );
            assert!(crate::f01_late::pin_applies(key, Some("dozer")), "{key}");
            for other in [None, Some("boiler"), Some("hazards"), Some("juicer")] {
                assert!(
                    crate::f01_late::pin_is_scoped_out(key, other),
                    "{key} {other:?}"
                );
            }
        }
        assert_eq!(combat.slot_enemies().len(), 1);
        // DR-C: the two riders are enemies 0 and 1, carried from tick 0.
        assert_eq!(combat.enemies.len(), 2);
        assert!(combat.is_carried(0) && combat.is_carried(1));
        assert!(combat.enemies.iter().all(|g| !g.alive()));
        assert_eq!(combat.slot_enemies()[0].kind(), "dozer");
        assert!(combat.slot_enemies()[0].hurt_rects().is_empty());
        assert_eq!(scene.world.tick, 0);
        // The lintel is a solid of the scene: its world bottom is 112 px over
        // the walking line.
        let foot = scene.world.hero_foot_y();
        assert!(
            scene
                .world
                .solids
                .iter()
                .any(|b| b.rect.left == 1040.0 && (b.rect.bottom - foot - 112.0).abs() < 0.5),
            "the lintel solid"
        );
    }
}
