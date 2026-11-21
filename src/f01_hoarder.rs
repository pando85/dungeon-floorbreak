//! HRD-A2: the Hoarder's room as a scene (`f01_hoarder`, the room `hoarder` of
//! `--scene f01_late`).
//!
//! Bevy-free. [`launch`] loads the room and its encounter sidecar through the
//! one room parser ([`crate::encounter::load`], which runs the whole
//! validation), composes the kit exactly as the Boiler does
//! ([`crate::f01_boiler::compose_kit`]), and builds the combat fixture and the
//! checkpoint rules from the sidecar. [`combat`] then builds the combat state
//! the scene and the fixtures share:
//!
//! * the **Hoarder spawns from the sidecar roster through ENEMY-SLOT**
//!   ([`crate::scene_enemy::spawn_from_roster`], the `hoarder` kind registered
//!   in [`crate::scene_enemy::SLOT_KINDS`]); she is the first enemy to load
//!   from a roster. The K16 relaxation for the existing goblin, llama and
//!   Engineer kinds is not here (BOIL-E);
//! * the story setup arms her defeat trigger: the room's scene kind is
//!   [`SceneKind::PileBoss`], which **only arms triggers**, while the room pin
//!   stays the `F01Late` row the sidecar names (`f01_pin_pre_pile_boss`), chosen
//!   by id. The dynamite is granted by the ledger's event trigger
//!   `pile_boss_f01_defeated` (a generic `EnemyDefeated` matcher on her target
//!   id) and by nothing in this file.
//!
//! The room's content is pinned ([`ROOM_HASH`], [`ENCOUNTER_HASH`]): [`launch`]
//! refuses an edited file, a test pins each constant to its file, and an edit
//! is a deliberate re-pin here.

use std::path::Path;

use crate::camera_lab::room::IRect;
use crate::carl_prog::CarlProgFlags;
use crate::combat_scene::EnvRuntime;
use crate::combat_scene::{CombatState, DemoRules, Fixture};
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::donut_prog::DonutProgFlags;
use crate::encounter::{self, Loaded};
use crate::env_kit::{EnvKit, IRect as KitRect};
use crate::pile_boss::{
    ENCOUNTER_PATH, GATE_HEIGHT_PX, GATE_WIDTH_PX, PILE_BOSS_ID, PileBossTuning,
};
use crate::scene_enemy::{SlotRoom, SlotSet, spawn_from_roster};
use crate::story_unlock::{SceneKind, StorySetup, StoryTable};

/// The `--scene` name of the room's fixtures and launch.
pub const SCENE_NAME: &str = "f01_hoarder";
/// The id of the room's scenario in its file.
pub const ROOM_ID: &str = "F01-HOARDER";
/// FNV-1a 64 of the room file and of the sidecar. The files live under
/// `assets/levels`, which a compiled-in `include_str!` may not reach, so the hash
/// is a constant ([`crate::f01_boiler::ROOM_HASH`] is the precedent).
pub const ROOM_HASH: u64 = 0x5b52_cfe9_96a9_0679;
pub const ENCOUNTER_HASH: u64 = 0x05fc_ad32_3772_d2d8;
/// The pin the sidecar names: the room holds Light on Your Feet, not the
/// dynamite (K1, K9); the dynamite is her defeat's grant.
pub const PIN_PRE_PILE_BOSS: &str = "f01_pin_pre_pile_boss";
/// A fixture seed is only recorded (no rule reads it in this room).
const FIXTURE_SEED: u64 = 0x0F01_4D0A;

/// A launched Hoarder room: everything a scene or a fixture needs.
#[derive(Debug, Clone)]
pub struct HoarderRun {
    pub kit: EnvKit,
    pub fixture: &'static Fixture,
    /// Respawn columns: the start column, then the checkpoint's when it is
    /// right of the start.
    pub rules: &'static DemoRules,
    pub loaded: Loaded,
}

/// Load the room, compose its kit and build the fixture and rules.
pub fn launch(root: &Path) -> Result<HoarderRun, String> {
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
             sidecar {:#018x} vs {ENCOUNTER_HASH:#018x}): re-pin src/f01_hoarder.rs on purpose",
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
    // One small leak per launch: the combat state borrows its rules for `'static`.
    let rules: &'static DemoRules = Box::leak(Box::new(DemoRules {
        checkpoints: Box::leak(columns.into_boxed_slice()),
    }));
    let fixture: &'static Fixture = Box::leak(Box::new(Fixture {
        name: SCENE_NAME,
        summary: "HRD-A2: the Hoarder's room, her sidecar roster, gate and door (greybox)",
        seed: FIXTURE_SEED,
        carl_x: start,
        enemies: &[],
        shore_dx: None,
        boss: None,
        llamas: &[],
        engineers: &[],
        charges: &[],
        script: &[],
        ticks: 0,
    }));
    Ok(HoarderRun {
        kit,
        fixture,
        rules,
        loaded,
    })
}

/// The story setup of a run of this room: the debug start flags, the rows of
/// the pin the sidecar names (each under its own program flag), and the
/// triggers armed in [`SceneKind::PileBoss`]. With no flag it is
/// [`StorySetup::NONE`]. Under the `all` loadout (the debug start flags, no
/// program flag) no trigger is armed, so no grant fires.
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
        SceneKind::PileBoss,
        pin,
    ))
}

/// The slot room facts of a launched scene (the same values the combat state
/// hands its slot enemies).
pub fn slot_room(run: &HoarderRun, scene: &SceneSim) -> SlotRoom {
    SlotRoom {
        floor_y: scene.world.hero_foot_y(),
        walls: (0.0, run.kit.width as f32),
    }
}

/// The room's runtime env: the sidecar's lock (the exit door, closed until her
/// defeat) and her entry gate (HRD-Q41, boss data), as the runtime env solids
/// the one sync (`combat_scene/env.rs`) keeps in the scene's solids. The room has
/// no props and no hazards.
pub fn env_runtime(run: &HoarderRun) -> Result<EnvRuntime, String> {
    let tuning = PileBossTuning::active();
    let gate = IRect {
        x: i32::from(tuning.hrdq41_gate_x),
        y: run.loaded.room.floor_y - GATE_HEIGHT_PX as i32,
        w: GATE_WIDTH_PX as i32,
        h: GATE_HEIGHT_PX as i32,
    };
    Ok(EnvRuntime::new(&run.loaded.encounter)?.with_slot_gate(PILE_BOSS_ID, gate))
}

/// The room's combat state at tick 0: the fixture, Donut with the biscuit and a
/// full pool (the post-corral state), the sidecar's pin and the armed trigger
/// under the program flags, the checkpoint rules, the CF02 rules, the env solids
/// (door and gate, appended to `scene` at setup so the live scene and a verifier
/// start from the same world) and **her, spawned from the sidecar roster through
/// ENEMY-SLOT**.
pub fn combat(
    run: &HoarderRun,
    scene: &mut SceneSim,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
) -> Result<CombatState, String> {
    let spawn = spawn_from_roster(&run.loaded.encounter, slot_room(run, scene), false)
        .map_err(|e| e.to_string())?;
    if !spawn.existing.is_empty() {
        return Err("the Hoarder's roster holds an existing kind (BOIL-E loads those)".to_owned());
    }
    combat_with(run.fixture, run, scene, carl, donut, spawn.slot)
}

/// [`combat`] with an explicit fixture (Carl's start column) and slot set (the
/// fixtures' awake Hoarder). The env is installed before the slot and the slot
/// **last**: a `with_*` builder that resets the attempt after the slot would mark
/// her as retried.
pub fn combat_with(
    fixture: &'static Fixture,
    run: &HoarderRun,
    scene: &mut SceneSim,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
    slot: SlotSet,
) -> Result<CombatState, String> {
    let setup = story_setup(&run.loaded.encounter.story_pin, carl, donut)?;
    let mut combat = CombatState::new(fixture, &run.kit, scene)?
        .with_donut(DonutSetup::unlocked(8))
        .try_with_story(setup)
        .map_err(|e| e.to_string())?
        .with_demo(run.rules)
        .with_cf02()
        .with_env(env_runtime(run)?)
        .with_slot_enemies(slot)
        .map_err(|e| e.to_string())?;
    combat.sync_env(scene);
    Ok(combat)
}

/// Launch the room and build its scene and combat state.
pub fn start(
    root: &Path,
    view_height: i32,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
) -> Result<(HoarderRun, SceneSim, CombatState), String> {
    let run = launch(root)?;
    let mut scene = SceneSim::new_at(&run.kit, view_height, run.fixture.carl_x)?;
    let state = combat(&run, &mut scene, carl, donut)?;
    Ok((run, scene, state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pile_boss::ROOM_PATH;
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
}
