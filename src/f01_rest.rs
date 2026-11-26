//! REST-WIRE: the quiet rooms of the late-Floor-1 run
//! (`docs/01-design/floor-01-quiet-setpieces-and-transitions.md` section G,
//! `docs/07-agents/floor-01-quiet-setpieces-workorder.md`). This first step (REST-A,
//! REST-AD11) lands the **refuge**, room 1 `rest_refuge`, "The Wrong Restaurant":
//! listed right after the corral so the Light on Your Feet grant of the story table
//! (`f01_safe_room_reward_found`, REST-AD3) happens in play before the Hoarder. The
//! camp and the aftermath are the second step's; [`RestRoom`] is ready for them.
//!
//! It launches through `--scene f01_late --f01-room rest_refuge` (and walks in
//! `--f01-run`): no scene arm of its own, like the Ring rooms (KR22).
//!
//! Bevy-free. [`launch`] loads the room and its sidecar through the one room parser
//! ([`crate::encounter::load`], the whole validation: the `hero_on_surface` lock, the
//! DP-Q45 lane ceiling rule, the K8 nest rule, an empty roster under `reach_exit`,
//! REST-AD13), composes the kit like the Boiler ([`crate::f01_boiler::compose_kit`], no
//! dressing: the AREA-FASTFOOD-01 art is a separate later work order, so the room is a
//! **labelled playable greybox**) and builds the fixture and the checkpoint rules from
//! the sidecar. [`combat_for`] builds the combat state the scene, the fixtures and the
//! PLAYTRACE verifier share:
//!
//! * no enemy, no hazard, no prop: the env runtime carries the one **staff door**, a
//!   latched `hero_on_surface` lock (REST-AD5, REST-AD6): a full-height solid at the
//!   exit that stays until Donut stands grounded on the shelf (the 128 px platform 2,
//!   boosted Donut only), then stays open for the attempt;
//! * Donut starts with the biscuit and a full pool (`DonutSetup::unlocked(8)`, like the
//!   corral and the Ring rooms). It is **mandatory**: `jump_reach_boost` requires
//!   `primary_ranged`, and a trigger whose guard fails is spent without granting, which
//!   would seal the refuge on every restart;
//! * the story: the room holds nothing at entry and **arms** the reward trigger
//!   `f01_safe_room_reward_found` through [`SceneKind::RestRefuge`] under the Donut
//!   program flags (the Story loadout; under All the boost is held and the trigger is
//!   spent). The sidecar's `story_pin` is a neutral label with no table row, like the
//!   corral's (checked here to stay so). A **flagless launch arms nothing, so the door
//!   never opens: the room is sealed by design** ([`entry_line`] says `boost=none`);
//! * the exit is the sidecar's `exit` (`goal: reach_exit`), beyond the door.
//!
//! The decor tables ([`decor`]) are presentation only (REST-AD10, REST-AD16): flat
//! labelled rectangles the binary draws; never solids, never in the sim, never hashed.
//!
//! The room and the sidecar are pinned ([`REFUGE_ROOM_HASH`],
//! [`REFUGE_ENCOUNTER_HASH`]): [`launch`] refuses an edited file, a test pins each
//! constant to its file, and an edit is a deliberate re-pin here.

use std::path::Path;

use crate::camera_lab::room::IRect;
use crate::carl_prog::CarlProgFlags;
use crate::combat_scene::{CombatState, DemoRules, EnvRuntime, Fixture, Hold};
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::donut_prog::DonutProgFlags;
use crate::encounter::{self, Loaded};
use crate::env_kit::{EnvKit, IRect as KitRect};
use crate::input::RawInput;
use crate::scene_enemy::{SlotRoom, spawn_from_roster};
use crate::story_unlock::{SceneKind, StorySetup, StoryTable};

/// The refuge's sidecar and room file.
pub const REFUGE_ENCOUNTER_PATH: &str = "assets/levels/floor01/f01_rest_refuge.encounter.json";
pub const REFUGE_ROOM_PATH: &str = "assets/levels/floor01/f01_rest_refuge.room.json";

/// FNV-1a 64 of the room file and of the sidecar (the files live under
/// `assets/levels`, so the hashes are constants, the Boiler's precedent).
pub const REFUGE_ROOM_HASH: u64 = 0x9530_9fd0_8a4b_5554;
pub const REFUGE_ENCOUNTER_HASH: u64 = 0xee94_f77c_d985_cf26;
/// The `data_pins()` keys, scoped to the `rest_refuge` room by
/// [`crate::f01_late::pin_applies`].
pub const REFUGE_ROOM_PIN_KEY: &str = "f01_rest_refuge_room";
pub const REFUGE_ENCOUNTER_PIN_KEY: &str = "f01_rest_refuge_encounter";

/// A fixture seed is only recorded: the room has no random draw, but the companion
/// draws its temperament from `--cf02-seed`.
const FIXTURE_SEED: u64 = 0x0F01_E5E0;

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

/// The staff door's sidecar lock id.
pub const STAFF_DOOR: &str = "staff_door";
/// The refuge's reward box column (the story table's zone, REST-AD4): x range.
pub const REWARD_ZONE_X: (i32, i32) = (1200, 1240);

/// One of the quiet rooms (REST-AD1). Only the refuge is listed by this step; the
/// camp and the aftermath join in REST-WIRE's second PR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RestRoom {
    /// Room 1, REST-A: the fast-food refuge with the staff door.
    Refuge,
}

impl RestRoom {
    pub const ALL: [RestRoom; 1] = [RestRoom::Refuge];

    /// The manifest id (`--f01-room`).
    pub const fn id(self) -> &'static str {
        match self {
            Self::Refuge => "rest_refuge",
        }
    }

    /// The fixture name (a recording's `reset.name`).
    pub const fn scene_name(self) -> &'static str {
        match self {
            Self::Refuge => "f01_rest_refuge",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.id() == id)
    }

    pub const fn encounter_path(self) -> &'static str {
        match self {
            Self::Refuge => REFUGE_ENCOUNTER_PATH,
        }
    }

    pub const fn room_path(self) -> &'static str {
        match self {
            Self::Refuge => REFUGE_ROOM_PATH,
        }
    }

    /// FNV-1a 64 of the room file.
    pub const fn room_hash(self) -> u64 {
        match self {
            Self::Refuge => REFUGE_ROOM_HASH,
        }
    }

    /// FNV-1a 64 of the sidecar.
    pub const fn encounter_hash(self) -> u64 {
        match self {
            Self::Refuge => REFUGE_ENCOUNTER_HASH,
        }
    }

    /// The `data_pins()` key of the room file.
    pub const fn room_pin_key(self) -> &'static str {
        match self {
            Self::Refuge => REFUGE_ROOM_PIN_KEY,
        }
    }

    /// The `data_pins()` key of the sidecar.
    pub const fn encounter_pin_key(self) -> &'static str {
        match self {
            Self::Refuge => REFUGE_ENCOUNTER_PIN_KEY,
        }
    }

    /// The marker of the headless smoke and the entry line.
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Refuge => "F01_REST_REFUGE",
        }
    }
}

/// A presentation-only rectangle of a room (REST-AD10, REST-AD16): a flat labelled
/// greybox shape in layout px (y down). Never a solid, never in the sim, never hashed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decor {
    pub label: &'static str,
    pub rect: IRect,
}

const fn d(label: &'static str, x: i32, y: i32, w: i32, h: i32) -> Decor {
    Decor {
        label,
        rect: IRect::new(x, y, w, h),
    }
}

/// The refuge's greybox decor: the counter behind the plane, booths, windows on the
/// back wall and the reward box standing on the grant column (REST-AD16: non-collidable
/// decor; the table's zone covers any height at the box).
const REFUGE_DECOR: [Decor; 8] = [
    d("counter", 320, 516, 448, 64),
    d("booth_a", 800, 540, 96, 40),
    d("booth_b", 928, 540, 96, 40),
    d("window_a", 352, 404, 64, 48),
    d("window_b", 640, 404, 64, 48),
    d("window_c", 928, 404, 64, 48),
    d("shelf_hatch", 1296, 480, 96, 36),
    d(
        "reward_box",
        REWARD_ZONE_X.0,
        548,
        REWARD_ZONE_X.1 - REWARD_ZONE_X.0,
        32,
    ),
];

/// The decor of a room (presentation only).
pub const fn decor(room: RestRoom) -> &'static [Decor] {
    match room {
        RestRoom::Refuge => &REFUGE_DECOR,
    }
}

/// A launched quiet room: everything a scene or a fixture needs.
#[derive(Debug, Clone)]
pub struct RestRun {
    pub room: RestRoom,
    pub kit: EnvKit,
    pub fixture: &'static Fixture,
    /// Respawn columns: the start column, then the checkpoint's.
    pub rules: &'static DemoRules,
    pub loaded: Loaded,
}

impl RestRun {
    /// The staff door's rectangle (layout px, y down), if the room has one.
    pub fn door(&self) -> Option<IRect> {
        self.loaded
            .encounter
            .locks
            .iter()
            .find(|l| l.id == STAFF_DOOR)
            .map(|l| l.solid)
    }
}

/// The env runtime of a launched room: the sidecar's one latched lock, placed on the
/// room's walking line.
pub fn env_runtime(run: &RestRun) -> Result<EnvRuntime, String> {
    Ok(EnvRuntime::new(&run.loaded.encounter)?.in_room(run.loaded.room.floor_y, &[], None))
}

/// Load a quiet room and its sidecar, compose its kit and build the fixture. No
/// dressing exists yet (AREA-FASTFOOD-01 is a separate work order), so the kit is the
/// plain greybox composition of the Boiler.
pub fn launch(root: &Path, room: RestRoom) -> Result<RestRun, String> {
    let path = room.encounter_path();
    let loaded = encounter::load(root, path).map_err(|errors| {
        format!(
            "{path}: {}",
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        )
    })?;
    if loaded.room_hash != room.room_hash() || loaded.sidecar_hash != room.encounter_hash() {
        return Err(format!(
            "the room or its sidecar differs from the pinned content (room {:#018x} vs {:#018x}, \
             sidecar {:#018x} vs {:#018x}): re-pin src/f01_rest.rs on purpose",
            loaded.room_hash,
            room.room_hash(),
            loaded.sidecar_hash,
            room.encounter_hash()
        ));
    }
    // The refuge's pin is a label (like the corral's): a table row under it would be held.
    match room {
        RestRoom::Refuge => {
            if StoryTable::active().has_pin(&loaded.encounter.story_pin) {
                return Err(format!(
                    "{path}: the story pin {:?} has rows in the story table; the refuge holds \
                     nothing at entry (REST-AD3)",
                    loaded.encounter.story_pin
                ));
            }
        }
    }
    if !loaded.encounter.roster.is_empty() {
        return Err(format!(
            "{path}: a quiet room has an empty roster (REST-AD13)"
        ));
    }
    let mut kit = crate::f01_boiler::compose_kit(root, &loaded.room)?;
    if kit.base_walk_line_y != loaded.room.floor_y {
        return Err(format!(
            "the kit's walking line {} is not the room's floor {}",
            kit.base_walk_line_y, loaded.room.floor_y
        ));
    }
    // DP-Q45 / DM-Q27 / KR9: the room's lane ceiling slab (see the Boiler).
    if let Some(ceiling) = loaded.ceiling_px {
        kit.solid_rects.push(KitRect {
            x: 0,
            y: kit.base_walk_line_y - ceiling - 16,
            w: kit.width,
            h: 16,
        });
    }
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
    if !spawn.slot.is_empty() || !spawn.existing.is_empty() {
        return Err(format!("{path}: a quiet room holds no enemy"));
    }
    // One small leak per launch (a process launches a room once): the combat
    // state borrows its rules for `'static`.
    let rules: &'static DemoRules = Box::leak(Box::new(DemoRules {
        checkpoints: Box::leak(columns.into_boxed_slice()),
    }));
    let fixture: &'static Fixture = Box::leak(Box::new(Fixture {
        name: room.scene_name(),
        summary: match room {
            RestRoom::Refuge => {
                "REST-WIRE: the refuge (a quiet dining room, the reward box, the staff shelf and \
                 the staff door); greybox, no art yet"
            }
        },
        seed: FIXTURE_SEED,
        carl_x: start,
        enemies: &[],
        shore_dx: None,
        boss: None,
        llamas: &[],
        engineers: &[],
        charges: &[],
        script: WALK_RIGHT,
        ticks: SMOKE_TICKS,
        room: crate::combat_scene::FixtureRoom::Corridor,
    }));
    Ok(RestRun {
        room,
        kit,
        fixture,
        rules,
        loaded,
    })
}

/// The story setup of a quiet room under a run's story flags. The refuge is the
/// [`SceneKind::RestRefuge`] one: it holds nothing and arms the reward trigger under
/// the Donut program (REST-AD3), the corral's pattern.
pub fn story_setup(room: RestRoom, carl: CarlProgFlags, donut: DonutProgFlags) -> StorySetup {
    match room {
        RestRoom::Refuge => crate::donut_prog::story_setup(carl, donut, SceneKind::RestRefuge),
    }
}

/// The combat state of a launched room under a run's story flags and `--cf02-seed`:
/// the CF02 rules, Donut with the biscuit and a full pool (mandatory, see the module
/// docs), the room's story setup, the checkpoint rules and the env step with the staff
/// door. The binary's scene and the PLAYTRACE verifier both call it through
/// [`crate::f01_late::rest_start`].
pub fn combat_for(
    run: &RestRun,
    scene: &mut SceneSim,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
    cf02_seed: Option<u64>,
) -> Result<CombatState, String> {
    let mut combat = CombatState::new(run.fixture, &run.kit, scene)?
        .with_donut(DonutSetup::unlocked(8))
        .try_with_story(story_setup(run.room, carl, donut))
        .map_err(|e| e.to_string())?
        .with_demo(run.rules)
        .with_cf02()
        .with_cf02_seed(cf02_seed)
        .with_env(env_runtime(run)?);
    combat.sync_env(scene);
    Ok(combat)
}

/// [`combat_for`] under no story flag and no seed (the fixtures' state): the refuge is
/// **sealed** then, since nothing arms the reward trigger.
pub fn combat_of(run: &RestRun, scene: &mut SceneSim) -> Result<CombatState, String> {
    combat_for(
        run,
        scene,
        CarlProgFlags::default(),
        DonutProgFlags::default(),
        None,
    )
}

/// The console line of a launch. `lock` is `closed|open|none` (the staff door);
/// `boost` is `held` when the ledger holds Donut's `jump_reach_boost`, `armed` when
/// the reward trigger is armed and unfired, and `none` when neither (a flagless
/// launch: the door can never open, the room is sealed by design).
pub fn entry_line(run: &RestRun, combat: &CombatState) -> String {
    let e = &run.loaded.encounter;
    let room = &run.loaded.room;
    let lock = match combat.env().map(EnvRuntime::surface_locks_open).as_deref() {
        Some([open, ..]) => {
            if *open {
                "open"
            } else {
                "closed"
            }
        }
        _ => "none",
    };
    let boost = if combat.story_holds(crate::donut_prog::JUMP_REACH_BOOST) {
        "held"
    } else if combat.story_zone_armed(1) {
        "armed"
    } else {
        "none"
    };
    format!(
        "{}_ENTRY room={} encounter={} pin={} spawn_carl={} spawn_donut={} \
         checkpoints={:?} goal={:?} roster={} lock={lock} boost={boost} env=live greybox=true \
         room_hash={:#018x} sidecar_hash={:#018x}",
        run.room.marker(),
        e.room_id,
        e.id,
        e.story_pin,
        room.spawn_carl,
        room.spawn_donut,
        run.rules.checkpoints,
        e.goal,
        e.roster.len(),
        run.loaded.room_hash,
        run.loaded.sidecar_hash,
    )
}

/// The pins `playtrace::live::data_pins` stamps (scoped to their room by
/// [`crate::f01_late::pin_applies`]).
pub const fn data_pins() -> [(&'static str, u64); 2] {
    [
        (REFUGE_ROOM_PIN_KEY, REFUGE_ROOM_HASH),
        (REFUGE_ENCOUNTER_PIN_KEY, REFUGE_ENCOUNTER_HASH),
    ]
}

#[cfg(test)]
mod tests;
