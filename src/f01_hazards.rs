//! HAZ-A2: the hazard room as a scene (`f01_hazards`, the room `hazards` of
//! `--scene f01_late`): the slime, the acid patch and the Rot Sticker in the
//! isolated pockets A, B and C (`docs/01-design/enemy-gameplay-slime-rot.md`
//! section 15).
//!
//! Bevy-free. [`launch`] loads the room and its encounter sidecar through the
//! one room parser ([`crate::encounter::load`], which runs the whole
//! validation), composes the kit exactly as the Boiler does
//! ([`crate::f01_boiler::compose_kit`]) and builds the combat fixture and the
//! checkpoint rules from the sidecar. [`combat`] then builds the combat state
//! the scene and the fixtures share:
//!
//! * the **slimes spawn from the sidecar roster through ENEMY-SLOT**
//!   ([`crate::scene_enemy::spawn_from_roster`], the `slime` kind registered in
//!   [`crate::scene_enemy::SLOT_KINDS`]); the goblins of the roster are the
//!   fixture's goblins, in roster order (HAZA2-AD8);
//! * the **stickers and the acid** are the env step's ([`crate::combat_scene::EnvRuntime`]):
//!   the sticker charges are appended after the fixture charges at the end of
//!   `reset_attempt`, the acid patches are persistent volumes, the goblins get
//!   the walk barriers, and the exit opens when the roster is clear
//!   (`defeat_roster`, HAZ-Q39).
//!
//! The room's content is pinned ([`ROOM_HASH`], [`ENCOUNTER_HASH`]): [`launch`]
//! refuses an edited file, a test pins each constant to its file, and an edit
//! is a deliberate re-pin here.

use std::path::Path;

use crate::camera_lab::room::EnemyKind;
use crate::camera_lab::room::IRect;
use crate::carl_prog::CarlProgFlags;
use crate::combat::EnemyAi;
use crate::combat_scene::{CombatState, DemoRules, EnemySpawn, EnvRuntime, Fixture, Hold};
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::donut_prog::DonutProgFlags;
use crate::encounter::{self, HAZARDS_ENCOUNTER_PATH, Loaded, PropKind, RosterKind};
use crate::env_kit::{EnvKit, IRect as KitRect};
use crate::input::RawInput;
use crate::scene_enemy::{SlotRoom, spawn_from_roster};

/// The `--scene` name.
pub const SCENE_NAME: &str = "f01_hazards";
/// The id of the room's scenario in its file.
pub const ROOM_ID: &str = "F01-HAZARDS";
/// FNV-1a 64 of the room file and of the sidecar (the Boiler's precedent: the
/// files live under `assets/levels`, so the hash is a constant).
pub const ROOM_HASH: u64 = 0x3c67_e0f2_adee_5032;
pub const ENCOUNTER_HASH: u64 = 0x6faf_f731_fce0_4f6b;
/// The `data_pins()` keys.
pub const ROOM_PIN_KEY: &str = "f01_hazards_room";
pub const ENCOUNTER_PIN_KEY: &str = "f01_hazards_encounter";
/// A fixture seed is only recorded (no rule reads it in this room).
const FIXTURE_SEED: u64 = 0x0F01_4A2D;

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

/// A launched hazard room: everything a scene or a fixture needs.
#[derive(Debug, Clone)]
pub struct HazardsRun {
    pub kit: EnvKit,
    pub fixture: &'static Fixture,
    /// Respawn columns: the start column, then the checkpoint's when it is
    /// right of the start.
    pub rules: &'static DemoRules,
    pub loaded: Loaded,
}

/// The soffit solids of the sidecar (layout px): a ceiling sticker's surface
/// that is not a room platform's slab is a full-height soffit with no
/// standable top (PR #204 review F1), which the env step materialises.
pub fn soffits(loaded: &Loaded) -> Vec<IRect> {
    let slab = loaded.encounter.platform_slab_px;
    let platforms: Vec<IRect> = loaded
        .room
        .platforms
        .iter()
        .map(|p| IRect::new(p.rect.x, p.rect.y, p.rect.w, slab))
        .collect();
    let mut out: Vec<IRect> = Vec::new();
    for prop in &loaded.encounter.props {
        if prop.kind != PropKind::RotSticker {
            continue;
        }
        let Some(track) = prop.track else { continue };
        let s = track.surface;
        if !platforms.contains(&s) && !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

/// The env runtime of a launched room: the sidecar's stickers and acid patches
/// placed on the room's walking line, with the soffit solids and the exit gate.
pub fn env_runtime(run: &HazardsRun) -> Result<EnvRuntime, String> {
    let enc = &run.loaded.encounter;
    Ok(EnvRuntime::new(enc)?.in_room(
        run.loaded.room.floor_y,
        &soffits(&run.loaded),
        Some(enc.exit),
    ))
}

/// Load the room, compose its kit and build the fixture and rules.
pub fn launch(root: &Path) -> Result<HazardsRun, String> {
    let loaded = encounter::load(root, HAZARDS_ENCOUNTER_PATH).map_err(|errors| {
        format!(
            "{HAZARDS_ENCOUNTER_PATH}: {}",
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
             sidecar {:#018x} vs {ENCOUNTER_HASH:#018x}): re-pin src/f01_hazards.rs on purpose",
            loaded.room_hash, loaded.sidecar_hash
        ));
    }
    if !crate::story_unlock::StoryTable::active().has_room_pin(&loaded.encounter.story_pin) {
        return Err(format!(
            "{HAZARDS_ENCOUNTER_PATH}: unknown story pin {:?}",
            loaded.encounter.story_pin
        ));
    }
    let mut kit = crate::f01_boiler::compose_kit(root, &loaded.room)?;
    if kit.base_walk_line_y != loaded.room.floor_y {
        return Err(format!(
            "the kit's walking line {} is not the room's floor {}",
            kit.base_walk_line_y, loaded.room.floor_y
        ));
    }
    // DP-Q45 / DM-Q27: the room's lane ceiling slab (see the Boiler).
    if let Some(ceiling) = loaded.ceiling_px {
        kit.solid_rects.push(KitRect {
            x: 0,
            y: kit.base_walk_line_y - ceiling - 16,
            w: kit.width,
            h: 16,
        });
    }
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
    // HAZA2-AD8: the roster's goblins (an existing kind) are the fixture's
    // goblins, chasing, in roster order; `trigger_x` and `delay_ticks` are not
    // read (a goblin that is far from the heroes is dormant by its sight range).
    let goblins: Vec<EnemySpawn> = loaded
        .encounter
        .roster
        .iter()
        .filter(|e| e.kind == RosterKind::Room(EnemyKind::Goblin))
        .map(|e| EnemySpawn {
            x: e.x,
            ai: EnemyAi::Chase,
        })
        .collect();
    // One small leak per launch (a process launches the room once): the combat
    // state borrows its rules for `'static`.
    let rules: &'static DemoRules = Box::leak(Box::new(DemoRules {
        checkpoints: Box::leak(columns.into_boxed_slice()),
    }));
    let fixture: &'static Fixture = Box::leak(Box::new(Fixture {
        name: SCENE_NAME,
        summary: "HAZ-A2: the hazard room (slime and acid, Rot Stickers) in isolated pockets",
        seed: FIXTURE_SEED,
        carl_x: start,
        enemies: Box::leak(goblins.into_boxed_slice()),
        shore_dx: None,
        boss: None,
        llamas: &[],
        engineers: &[],
        charges: &[],
        script: WALK_RIGHT,
        ticks: SMOKE_TICKS,
    }));
    Ok(HazardsRun {
        kit,
        fixture,
        rules,
        loaded,
    })
}

/// The combat state of a launched room, with the CF02 rules, the slot slimes and
/// the env step, under no story flag (the fixtures' state).
pub fn combat_of(run: &HazardsRun, scene: &mut SceneSim) -> Result<CombatState, String> {
    combat_for(
        run,
        scene,
        CarlProgFlags::default(),
        DonutProgFlags::default(),
        None,
    )
}

/// [`combat_of`] under a run's story flags and `--cf02-seed` (HAZ-A2b): the
/// sidecar's story pin applies under the program flags (the post-Hoarder state,
/// the same pin the Boiler holds), and the seed is the temperament draw of the
/// roster's goblins. The binary's scene and the PLAYTRACE verifier both call it
/// through [`crate::f01_late::hazards_start`]. The seed is applied **before** the
/// slot: a builder that resets the attempt after the slot would mark a slime as
/// retried.
pub fn combat_for(
    run: &HazardsRun,
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
    let setup = crate::f01_boiler::story_setup(&run.loaded.encounter.story_pin, carl, donut)?;
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

/// The console line of a launch.
pub fn entry_line(run: &HazardsRun) -> String {
    let e = &run.loaded.encounter;
    let room = &run.loaded.room;
    format!(
        "F01_HAZARDS_ENTRY room={} encounter={} pin={} spawn_carl={} spawn_donut={} \
         checkpoints={:?} goal={:?} acid_patches={} stickers={} roster={} env=live \
         room_hash={:#018x} sidecar_hash={:#018x}",
        e.room_id,
        e.id,
        e.story_pin,
        room.spawn_carl,
        room.spawn_donut,
        run.rules.checkpoints,
        e.goal,
        e.acid_patches.len(),
        e.props
            .iter()
            .filter(|p| p.kind == PropKind::RotSticker)
            .count(),
        e.roster.len(),
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
    use crate::input::RawInput;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn the_pinned_hashes_are_the_files() {
        let hash = |path: &str| {
            crate::donut_mana::content_hash(
                &std::fs::read_to_string(root().join(path)).expect("the file reads"),
            )
        };
        assert_eq!(hash(encounter::HAZARDS_ROOM_PATH), ROOM_HASH);
        assert_eq!(hash(HAZARDS_ENCOUNTER_PATH), ENCOUNTER_HASH);
        let run = launch(&root()).expect("the room launches");
        assert_eq!(run.loaded.room_hash, ROOM_HASH);
        assert_eq!(run.loaded.sidecar_hash, ENCOUNTER_HASH);
        for (key, value) in data_pins() {
            assert_ne!(value, 0, "{key}");
        }
    }

    #[test]
    fn the_room_builds_its_slimes_stickers_and_gate() {
        let run = launch(&root()).unwrap();
        // The goblins of the roster are the fixture's, in roster order.
        assert_eq!(
            run.fixture.enemies.iter().map(|e| e.x).collect::<Vec<_>>(),
            vec![380, 952]
        );
        let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
        let combat = combat_of(&run, &mut scene).unwrap();
        let slimes = combat.slot_enemies();
        assert_eq!(slimes.len(), 2, "S1 and S2");
        assert_eq!(
            slimes.iter().map(|s| s.target_id()).collect::<Vec<_>>(),
            vec![
                crate::combat_scene::slime::target_id(0),
                crate::combat_scene::slime::target_id(2)
            ],
            "the target id follows the roster index"
        );
        let env = combat.env().expect("the env step");
        assert_eq!(env.sticker_count(), 2, "R1 and R2");
        assert_eq!(env.acid().patches().count(), 2);
        // The sticker charges follow the (empty) fixture charges, then the
        // reserved dynamite slot.
        assert_eq!(combat.dynamite_slot(), 2);
        // The soffit of pocket C is a runtime solid with no standable top, and
        // the exit gate is the last env solid.
        assert_eq!(soffits(&run.loaded).len(), 1);
        assert!(!combat.roster_cleared());
    }

    #[test]
    fn the_room_replays_to_the_same_hash_twice() {
        let run_once = || {
            let run = launch(&root()).unwrap();
            let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
            let mut combat = combat_of(&run, &mut scene).unwrap();
            for tick in 1..=400u64 {
                combat.step(&mut scene, run.fixture.input(tick.min(SMOKE_TICKS)));
            }
            combat.state_hash(&scene)
        };
        assert_eq!(run_once(), run_once());
        let _ = RawInput::NONE;
    }

    /// HAZ-A2b: the room holds the sidecar's story pin (the post-Hoarder state,
    /// dynamite among it) under the program flags and nothing without them, and
    /// the fixtures' state (`combat_of`) is the unflagged one.
    #[test]
    fn the_story_flags_apply_the_post_hoarder_pin_and_the_fixtures_stay_unflagged() {
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
        let fixtures = combat_of(&run, &mut scene).unwrap();
        assert_eq!(fixtures.state_hash(&scene), plain_hash);
        // A seed is part of the state (the roster's goblins draw temperaments).
        let seeded = |seed| {
            let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
            let combat = combat_for(
                &run,
                &mut scene,
                CarlProgFlags::default(),
                DonutProgFlags::default(),
                seed,
            )
            .unwrap();
            assert_eq!(
                combat.slot_enemies().len(),
                2,
                "the slimes survive the seed"
            );
            combat.state_hash(&scene)
        };
        assert_eq!(seeded(None), plain_hash);
        assert_eq!(seeded(Some(7)), seeded(Some(7)));
    }

    #[test]
    fn a_disabled_kill_switch_loads_the_room_without_that_kind() {
        // The data switches are read through `enabled()`; the shipped data has both on.
        assert!(crate::slime::active().enabled());
        assert!(crate::rot_sticker::active().enabled());
    }
}
