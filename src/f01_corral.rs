//! CORRAL-WIRE: room 0, the Infernal Corral, as a room of the late-Floor-1
//! slice (`docs/07-agents/f01-playtest-integration-plan.md` section 14, the
//! CORRAL-0 sheet `docs/01-design/enemy-gameplay-llama-rush.md` sections 10 and
//! 15). It launches only through `--scene f01_late --f01-room corral` (plan:
//! "CORRAL-WIRE adds no arg"). CORRAL-WIRE kept it **launchable but not listed**
//! (CW-AD1); CORRAL-F (section 14.7) lists it as the first room of the
//! manifest, so `--f01-run` starts here and walks on to the Hoarder.
//!
//! Bevy-free. [`launch`] loads the CORRAL-C room file and its sidecar through the
//! one room parser ([`crate::encounter::load`], the whole validation), composes
//! the kit with the room's authored full-height steps (the entry shelf, the dock
//! bumper and the dock, read from the room file's `parapet` list exactly as the
//! CORRAL-C fixtures and the evaluator read them, KC16) and the DP-Q45 lane
//! ceiling of `ceiling_px` 228, and builds the fixture and the checkpoint rules
//! from the sidecar. [`combat_for`] builds the combat state the scene, the
//! fixtures and the PLAYTRACE verifier share:
//!
//! * **M0**: the sidecar's two goblins, chasing (the biscuit's first bolts);
//! * **M1 and M2**: the sidecar's three llamas, in roster order. Their per-llama
//!   opt-ins (the M1 llama's legacy spit with party targeting; B and A with the
//!   corral rush profile, facing -1, dormant until the CRL-Q24 wake, A on the
//!   dock surface 32 px up) are the CORRAL-C ones of
//!   [`crate::corral_fixtures::room`], applied by the exact fixture name
//!   [`room::WIRED_NAME`] through the existing name hook (KC15), so the room and
//!   the CORRAL-C gate read one definition. [`launch`] refuses a sidecar whose
//!   llamas are not at the columns that definition is authored for;
//! * **the partition** (CORRAL-D, `pen_partition`): the sidecar's one prop, the optional
//!   rush-breakable timber screen at 1392..1408 (sheet section 10). It runs through the env
//!   runtime ([`env_runtime`] builds it from the sidecar like any prop) and is on in every launch
//!   of the room, `--f01-run` included;
//! * **the M3 gate** (CRL-Q26, DV-3): a full-height solid at 2048..2064 that
//!   stands until **both** M2 llamas are defeated and opens on the second defeat
//!   tick (an [`crate::combat_scene::EnvSolidKind::ExitGate`] that waits on the
//!   M2 llamas only, [`crate::combat_scene::EnvRuntime::with_llama_gate`]). The
//!   rush profile's `crlq26_exit_gate` switch removes it;
//! * the story: room 0 holds the Guild biscuit only (read through, Donut's
//!   `DonutSetup::unlocked(8)`) and grants nothing (KC9, KC11). Its setup is the
//!   [`SceneKind::Corral`] one, which no pin row and no trigger names, so it
//!   holds and arms nothing under any loadout. The sidecar's `story_pin` is a
//!   neutral label with no table row (CW-AD3), checked here to stay so;
//! * the exit is the sidecar's `exit` (`goal: reach_exit`): the M3 threshold
//!   beyond the gate is quiet.
//!
//! The room and the sidecar are pinned ([`ROOM_HASH`], [`ENCOUNTER_HASH`]):
//! [`launch`] refuses an edited file, a test pins each constant to its file,
//! and an edit is a deliberate re-pin here.

use std::path::Path;

use crate::camera_lab::room::{EnemyKind, IRect};
use crate::carl_prog::CarlProgFlags;
use crate::combat_scene::{CombatState, DemoRules, EnvRuntime, Fixture, Hold};
use crate::corral_fixtures::room;
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::donut_prog::DonutProgFlags;
use crate::encounter::{self, Loaded, RosterKind};
use crate::env_kit::{EnvKit, IRect as KitRect};
use crate::goblin::WakeGate;
use crate::input::RawInput;
use crate::scene_enemy::{SlotRoom, spawn_from_roster};
use crate::story_unlock::{SceneKind, StorySetup, StoryTable};

/// The sidecar (CORRAL-WIRE) and the CORRAL-C room file it names (KC16).
pub const ENCOUNTER_PATH: &str = "assets/levels/floor01/f01_corral.encounter.json";
pub const ROOM_PATH: &str = room::ROOM_PATH;

/// FNV-1a 64 of the room file and of the sidecar (the Boiler's precedent: the
/// files live under `assets/levels`, so the hashes are constants).
pub const ROOM_HASH: u64 = 0x5ccf_c53b_8eee_3702;
pub const ENCOUNTER_HASH: u64 = 0xc642_7d89_07c3_b8a7;
/// The `data_pins()` keys, scoped to the `corral` room.
pub const ROOM_PIN_KEY: &str = "f01_corral_room";
pub const ENCOUNTER_PIN_KEY: &str = "f01_corral_encounter";

/// The manifest id (`--f01-room`).
pub const ROOM_ID: &str = "corral";
/// The fixture name (a recording's `reset.name`).
pub const SCENE_NAME: &str = room::WIRED_NAME;
/// The marker of the headless smoke and the entry line.
pub const MARKER: &str = "F01_CORRAL";

/// The M3 gate (CRL-Q26): its left column and width, full lane height.
pub const M3_GATE_X: i32 = 2048;
pub const M3_GATE_W: i32 = 16;
/// The roster beat of the llamas the M3 gate waits on.
pub const GATE_BEAT: &str = "m2";

/// A fixture seed is only recorded: a live run draws its goblins'
/// temperaments from `--cf02-seed`.
const FIXTURE_SEED: u64 = 0x0F01_C0DC;

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

/// The launched room: everything a scene or a fixture needs.
#[derive(Debug, Clone)]
pub struct CorralRun {
    pub kit: EnvKit,
    pub fixture: &'static Fixture,
    /// Respawn columns: the start column, then the M2 checkpoint's.
    pub rules: &'static DemoRules,
    pub loaded: Loaded,
    /// The room's authored full-height steps (layout px, y down).
    pub steps: Vec<IRect>,
    /// FA-Q20: the wake gate of each goblin, in enemy index order (empty when
    /// no entry has one).
    pub wakes: Vec<Option<WakeGate>>,
    /// The M3 gate's rectangle (layout px, y down) and the llama indices it
    /// waits on; `None` when the `crlq26_exit_gate` switch is off.
    pub m3_gate: Option<(IRect, Vec<usize>)>,
}

/// The M3 gate's rectangle on a room whose walking line is `floor_y` and whose
/// lane ceiling is `ceiling` px above it: from the ceiling to the walking line.
pub fn m3_gate_rect(floor_y: i32, ceiling: i32) -> IRect {
    IRect::new(M3_GATE_X, floor_y - ceiling, M3_GATE_W, ceiling)
}

/// The env runtime of a launched room: the sidecar's partition prop, the goblins' wake gates and,
/// with the switch on, the M3 gate on the M2 llamas.
pub fn env_runtime(run: &CorralRun) -> Result<EnvRuntime, String> {
    let env = EnvRuntime::new(&run.loaded.encounter)?
        .with_wake_gates(run.wakes.clone())
        .in_room(run.loaded.room.floor_y, &[], None);
    Ok(match &run.m3_gate {
        Some((rect, llamas)) => env.with_llama_gate(*rect, llamas.clone()),
        None => env,
    })
}

/// The roster rule of room 0: goblins in M0, then the three llamas at the
/// CORRAL-C columns ([`room::WIRED_LLAMAS`], the M1 llama, B, A); any other
/// kind is refused, never dropped. Returns the llama indices of beat
/// [`GATE_BEAT`] (the M3 gate's).
fn check_roster(roster: &[encounter::RosterEntry]) -> Result<Vec<usize>, String> {
    let mut llamas = Vec::new();
    let mut gate = Vec::new();
    for e in roster {
        match e.kind {
            RosterKind::Room(EnemyKind::Goblin) => {}
            RosterKind::Room(EnemyKind::Llama) => {
                if e.beat == GATE_BEAT {
                    gate.push(llamas.len());
                }
                llamas.push(e.x);
            }
            other => {
                return Err(format!(
                    "roster {}: {other:?} is not a corral kind (goblins and llamas)",
                    e.id
                ));
            }
        }
    }
    if llamas != room::WIRED_LLAMAS {
        return Err(format!(
            "the corral llamas stand at {llamas:?}, not at the columns their opt-ins are \
             authored for {:?} (src/corral_fixtures/room.rs)",
            room::WIRED_LLAMAS
        ));
    }
    if gate != [1, 2] {
        return Err(format!(
            "the M3 gate waits on the llamas {gate:?} of beat {GATE_BEAT:?}, not on B and A"
        ));
    }
    Ok(gate)
}

/// Load room 0 and its sidecar, compose its kit and build the fixture. The kit
/// carries the AREA-HEAT-01 dressing ([`crate::heat_kit`], presentation only).
pub fn launch(root: &Path) -> Result<CorralRun, String> {
    launch_with(root, true)
}

/// [`launch`] with the dressing switched on or off. The undressed kit is the
/// plain composition of the greybox room; the tests prove the two differ in
/// presentation only (solids, ceiling slab, views, walking line, state hash).
pub fn launch_with(root: &Path, dressed: bool) -> Result<CorralRun, String> {
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
            "the room or its sidecar differs from the pinned content (room {:#018x} vs \
             {ROOM_HASH:#018x}, sidecar {:#018x} vs {ENCOUNTER_HASH:#018x}): re-pin \
             src/f01_corral.rs on purpose",
            loaded.room_hash, loaded.sidecar_hash,
        ));
    }
    // CW-AD3: the story pin is a label; a table row under it would be held.
    if StoryTable::active().has_pin(&loaded.encounter.story_pin) {
        return Err(format!(
            "{ENCOUNTER_PATH}: the story pin {:?} has rows in the story table; room 0 holds \
             nothing (KC9, KC11)",
            loaded.encounter.story_pin
        ));
    }
    let gate_llamas = check_roster(&loaded.encounter.roster)?;
    let (file_room, steps) = room::load_room(root)?;
    if file_room.floor_y != loaded.room.floor_y || file_room.exit != loaded.room.exit {
        return Err("the corral room parses two ways".into());
    }
    let mut kit = if dressed {
        crate::heat_kit::compose_heat_kit(root, &loaded.room, &steps)?
    } else {
        crate::f01_boiler::compose_kit_with_steps(root, &loaded.room, &steps)?
    };
    if kit.base_walk_line_y != loaded.room.floor_y {
        return Err(format!(
            "the kit's walking line {} is not the room's floor {}",
            kit.base_walk_line_y, loaded.room.floor_y
        ));
    }
    // DP-Q45 / CRL-Q31: the room's lane ceiling slab (see the Boiler).
    let ceiling = loaded
        .ceiling_px
        .ok_or("the corral room has no lane ceiling (CRL-Q31)")?;
    kit.solid_rects.push(KitRect {
        x: 0,
        y: kit.base_walk_line_y - ceiling - 16,
        w: kit.width,
        h: 16,
    });
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
    let slot_room = SlotRoom {
        floor_y: loaded.room.floor_y as f32,
        walls: (0.0, kit.width as f32),
    };
    let spawn =
        spawn_from_roster(&loaded.encounter, slot_room, false).map_err(|e| e.to_string())?;
    if !spawn.slot.is_empty() {
        return Err(format!("{ENCOUNTER_PATH}: room 0 holds no slot kind"));
    }
    let enemies = crate::f01_boiler::existing_enemies(&loaded.encounter, &spawn.existing, false)?;
    let wakes = enemies.wakes();
    let profile = crate::llama_rush::active();
    let m3_gate = profile
        .on(profile.crlq26_exit_gate)
        .then(|| (m3_gate_rect(loaded.room.floor_y, ceiling), gate_llamas));
    // One small leak per launch (a process launches a room once): the combat
    // state borrows its rules for `'static`.
    let rules: &'static DemoRules = Box::leak(Box::new(DemoRules {
        checkpoints: Box::leak(columns.into_boxed_slice()),
    }));
    let fixture: &'static Fixture = Box::leak(Box::new(Fixture {
        name: SCENE_NAME,
        summary: "CORRAL-WIRE: room 0, the Infernal Corral (M0 goblins, the M1 llama, rush llamas B \
                  and A, the M3 gate on the M2 llamas)",
        seed: FIXTURE_SEED,
        carl_x: start,
        enemies: Box::leak(enemies.goblins.into_boxed_slice()),
        shore_dx: None,
        boss: None,
        llamas: Box::leak(enemies.llamas.into_boxed_slice()),
        engineers: &[],
        charges: &[],
        script: WALK_RIGHT,
        ticks: SMOKE_TICKS,
        room: crate::combat_scene::FixtureRoom::Corridor,
    }));
    Ok(CorralRun {
        kit,
        fixture,
        rules,
        loaded,
        steps,
        wakes,
        m3_gate,
    })
}

/// The story setup of room 0: the [`SceneKind::Corral`] one, which holds and
/// arms nothing (no pin row, no trigger names the scene), under any flags.
pub fn story_setup(carl: CarlProgFlags, donut: DonutProgFlags) -> StorySetup {
    crate::donut_prog::story_setup(carl, donut, SceneKind::Corral)
}

/// The combat state of a launched room under a run's story flags and
/// `--cf02-seed`: the CF02 rules, Donut with the biscuit and a full pool, the
/// room's (empty) story setup, the checkpoint rules and the env step with the
/// M3 gate. The binary's scene and the PLAYTRACE verifier both call it through
/// [`crate::f01_late::corral_start`].
pub fn combat_for(
    run: &CorralRun,
    scene: &mut SceneSim,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
    cf02_seed: Option<u64>,
) -> Result<CombatState, String> {
    let mut combat = CombatState::new(run.fixture, &run.kit, scene)?
        .with_donut(DonutSetup::unlocked(8))
        .try_with_story(story_setup(carl, donut))
        .map_err(|e| e.to_string())?
        .with_demo(run.rules)
        .with_cf02()
        .with_cf02_seed(cf02_seed)
        .with_env(env_runtime(run)?);
    combat.sync_env(scene);
    Ok(combat)
}

/// [`combat_for`] under no story flag and no seed (the fixtures' state).
pub fn combat_of(run: &CorralRun, scene: &mut SceneSim) -> Result<CombatState, String> {
    combat_for(
        run,
        scene,
        CarlProgFlags::default(),
        DonutProgFlags::default(),
        None,
    )
}

/// The console line of a launch.
pub fn entry_line(run: &CorralRun) -> String {
    let e = &run.loaded.encounter;
    let room = &run.loaded.room;
    format!(
        "{MARKER}_ENTRY room={} encounter={} pin={} spawn_carl={} spawn_donut={} \
         checkpoints={:?} goal={:?} roster={} goblins={} llamas={} m3_gate={} listed=true \
         partition={} env=live room_hash={:#018x} sidecar_hash={:#018x}",
        e.room_id,
        e.id,
        e.story_pin,
        room.spawn_carl,
        room.spawn_donut,
        run.rules.checkpoints,
        e.goal,
        e.roster.len(),
        run.fixture.enemies.len(),
        run.fixture.llamas.len(),
        run.m3_gate.as_ref().map_or("off".to_owned(), |(r, l)| {
            let llamas: Vec<String> = l.iter().map(|i| i.to_string()).collect();
            format!("{}..{}_on_llamas_{}", r.x, r.right(), llamas.join("+"))
        }),
        // CORRAL-D: the optional rush-breakable partition (`off` for a sidecar without it).
        run.loaded
            .encounter
            .props
            .iter()
            .find(|p| p.kind == crate::encounter::PropKind::Partition)
            .map_or("off".to_owned(), |p| format!(
                "{}..{}_{}px",
                p.target.x,
                p.target.right(),
                p.target.h
            )),
        run.loaded.room_hash,
        run.loaded.sidecar_hash,
    )
}

/// The pins `playtrace::live::data_pins` stamps (scoped to the `corral` room by
/// [`crate::f01_late::pin_applies`]).
pub const fn data_pins() -> [(&'static str, u64); 2] {
    [
        (ROOM_PIN_KEY, ROOM_HASH),
        (ENCOUNTER_PIN_KEY, ENCOUNTER_HASH),
    ]
}

#[cfg(test)]
mod tests;
