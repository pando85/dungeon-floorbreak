//! TROG-A: the troglodyte rooms of the late-Floor-1 slice
//! (`docs/01-design/floor-01-troglodyte-integration.md` sections 4 and 6, the work order
//! `docs/07-agents/floor-01-troglodyte-workorder.md`). They launch through
//! `--scene f01_late --f01-room <id>` and are listed in the floor run between the survivors'
//! camp and the Juicer. TROG-A landed the first room, [`TrogRoom::Run`] (`trog_run`, #191's
//! T0 to T3: the Pygmy, the Basher and the Virtuoso one at a time); TROG-B adds
//! [`TrogRoom::Strength`] (`trog_strength`, #191's T4, the Test of Strength: the Virtuoso, the
//! Basher behind an 80 px static bench: the static fallback of plan 5.4 at Carl's class limit).
//!
//! Bevy-free. [`launch`] loads the room and its encounter sidecar through the one room parser
//! ([`crate::encounter::load`], the whole validation, the TROG-V rules included), composes the
//! kit exactly as the Boiler does ([`crate::f01_boiler::compose_kit`]) with the DP-Q45 lane
//! ceiling, the sidecar's plain `solid` statics (the low refuge, the bell step and the retreat
//! shelf) pushed in as lane steps (the ACT1-B form), and builds the fixture and the checkpoint
//! rules from the sidecar. [`combat_for`] then builds the combat state the scene, the fixtures and the PLAYTRACE
//! verifier share:
//!
//! * the **troglodytes spawn from the sidecar roster through ENEMY-SLOT**
//!   ([`crate::scene_enemy::spawn_from_roster`], the three `trog_*` kinds registered in
//!   [`crate::scene_enemy::SLOT_KINDS`]); the room has no goblin;
//! * the exit opens when the roster is clear (`defeat_roster`, the env step's derived gate);
//! * the Test of Strength's **bench** is the plan's declared static fallback (TROG-AD12,
//!   TROG-AD43) at 80 px (TROG-AD46): a sidecar static of kind `solid`, never a partition prop,
//!   because the validator refuses a solid *prop* above 48 px; a static may reach 80 px
//!   (`MAX_STATIC_SOLID_PX`);
//! * the story: the room is a story room on the camp's pin (`f01_pin_post_pile_boss`), so it
//!   carries the ledger unchanged and grants nothing (TROG-AD1); Donut holds the biscuit with a
//!   full pool (`DonutSetup::unlocked(8)`, plan 6.3);
//! * the companion sees the troglodytes through the per-room slot feed (TROG-AD16).
//!
//! The decor ([`decor`]) is presentation only: flat labelled rectangles, never a solid, never in
//! the sim, never hashed. AREA-TROG-01 dresses the room ([`crate::trog_kit`]); the table stays as
//! data and the binary no longer draws it for a dressed room.
//!
//! Each room and sidecar is pinned ([`RUN_ROOM_HASH`], [`RUN_ENCOUNTER_HASH`]): [`launch`]
//! refuses an edited file, a test pins each constant to its file, and an edit is a deliberate
//! re-pin here.

use std::path::Path;

use crate::camera_lab::room::IRect;
use crate::carl_prog::CarlProgFlags;
use crate::combat_scene::{CombatState, DemoRules, EnemySpawn, EnvRuntime, Fixture, Hold};
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::donut_prog::DonutProgFlags;
use crate::encounter::{self, Loaded, RosterKind};
use crate::env_kit::EnvKit;
use crate::input::RawInput;
use crate::scene_enemy::{SlotRoom, spawn_from_roster};

/// The run room's sidecar and room file.
pub const RUN_ENCOUNTER_PATH: &str = "assets/levels/floor01/f01_trog_run.encounter.json";
pub const RUN_ROOM_PATH: &str = "assets/levels/floor01/f01_trog_run.room.json";
/// The Test of Strength's sidecar and room file (TROG-B).
pub const STRENGTH_ENCOUNTER_PATH: &str = "assets/levels/floor01/f01_trog_strength.encounter.json";
pub const STRENGTH_ROOM_PATH: &str = "assets/levels/floor01/f01_trog_strength.room.json";

/// FNV-1a 64 of the room file and of the sidecar (the Boiler's precedent: the files live under
/// `assets/levels`, so the hashes are constants).
pub const RUN_ROOM_HASH: u64 = 0xe881_73c2_842f_8bc1;
pub const RUN_ENCOUNTER_HASH: u64 = 0x0fc1_829d_0b5c_935c;
/// The Test of Strength's pins (TROG-B).
pub const STRENGTH_ROOM_HASH: u64 = 0xc6c1_6be0_cb87_1aad;
pub const STRENGTH_ENCOUNTER_HASH: u64 = 0x037f_5984_8a5d_5a90;
/// The `data_pins()` keys, scoped to their room by [`crate::f01_late::pin_applies`].
pub const RUN_ROOM_PIN_KEY: &str = "f01_trog_run_room";
pub const RUN_ENCOUNTER_PIN_KEY: &str = "f01_trog_run_encounter";
pub const STRENGTH_ROOM_PIN_KEY: &str = "f01_trog_strength_room";
pub const STRENGTH_ENCOUNTER_PIN_KEY: &str = "f01_trog_strength_encounter";

/// A fixture seed is only recorded (no rule of the room reads it; the companion draws from
/// `--cf02-seed`).
const FIXTURE_SEED: u64 = 0x0F01_7206;

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

/// The sidecar's story pin: the camp's (the ledger crosses the room unchanged, TROG-AD1).
pub const STORY_PIN: &str = "f01_pin_post_pile_boss";

/// One of the troglodyte rooms in the listed run, in run order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrogRoom {
    /// #191's T0 to T3: the Pygmy, the Basher and the Virtuoso, one at a time.
    Run,
    /// #191's T4, the Test of Strength: the Virtuoso, an 80 px bench and the Basher (TROG-B).
    Strength,
}

impl TrogRoom {
    pub const ALL: [TrogRoom; 2] = [TrogRoom::Run, TrogRoom::Strength];

    /// The manifest id (`--f01-room`).
    pub const fn id(self) -> &'static str {
        match self {
            Self::Run => "trog_run",
            Self::Strength => "trog_strength",
        }
    }

    /// How many troglodytes the room's roster holds.
    pub const fn roster_len(self) -> usize {
        match self {
            Self::Run => 3,
            Self::Strength => 2,
        }
    }

    /// The fixture name (a recording's `reset.name`).
    pub const fn scene_name(self) -> &'static str {
        match self {
            Self::Run => "f01_trog_run",
            Self::Strength => "f01_trog_strength",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.id() == id)
    }

    pub const fn encounter_path(self) -> &'static str {
        match self {
            Self::Run => RUN_ENCOUNTER_PATH,
            Self::Strength => STRENGTH_ENCOUNTER_PATH,
        }
    }

    pub const fn room_path(self) -> &'static str {
        match self {
            Self::Run => RUN_ROOM_PATH,
            Self::Strength => STRENGTH_ROOM_PATH,
        }
    }

    /// FNV-1a 64 of the room file.
    pub const fn room_hash(self) -> u64 {
        match self {
            Self::Run => RUN_ROOM_HASH,
            Self::Strength => STRENGTH_ROOM_HASH,
        }
    }

    /// FNV-1a 64 of the sidecar.
    pub const fn encounter_hash(self) -> u64 {
        match self {
            Self::Run => RUN_ENCOUNTER_HASH,
            Self::Strength => STRENGTH_ENCOUNTER_HASH,
        }
    }

    /// The `data_pins()` key of the room file.
    pub const fn room_pin_key(self) -> &'static str {
        match self {
            Self::Run => RUN_ROOM_PIN_KEY,
            Self::Strength => STRENGTH_ROOM_PIN_KEY,
        }
    }

    /// The `data_pins()` key of the sidecar.
    pub const fn encounter_pin_key(self) -> &'static str {
        match self {
            Self::Run => RUN_ENCOUNTER_PIN_KEY,
            Self::Strength => STRENGTH_ENCOUNTER_PIN_KEY,
        }
    }

    /// The marker of the headless smoke and the entry line.
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Run => "F01_TROG_RUN",
            Self::Strength => "F01_TROG_STRENGTH",
        }
    }
}

/// A presentation-only rectangle of a room: a flat labelled greybox shape in layout px (y down).
/// Never a solid, never in the sim, never hashed.
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

/// The run room's greybox decor (plan 4.1; data only since AREA-TROG-01 dressed the room): the
/// knuckle marks and the far "slam" beat of T0 (no audio path, TROG-Q7), the fist-shaped hole
/// behind T2's Basher and the bell of T3.
const RUN_DECOR: [Decor; 5] = [
    d("knuckle_marks", 120, 520, 96, 40),
    d("slam_mark", 264, 480, 40, 100),
    d("impact_crater", 520, 556, 72, 24),
    d("fist_hole", 1344, 500, 64, 80),
    d("bell", 1744, 420, 40, 64),
];

/// The Test of Strength's greybox decor (plan 4.2): the retreat shelf and the 80 px static bench
/// (the sidecar's two solids) and the stone-to-tile threshold strip. The window draws it flat only
/// while the room is plain (no AREA-TROG-01 dressing block yet, TROG-ART-AD19), so the solids are
/// visible; a dressed room draws none.
const STRENGTH_DECOR: [Decor; 3] = [
    d("shelf", 288, 532, 64, 48),
    d("bench", 640, 500, 32, 80),
    d("threshold", 560, 572, 160, 8),
];

/// The decor of a room (presentation only).
pub const fn decor(room: TrogRoom) -> &'static [Decor] {
    match room {
        TrogRoom::Run => &RUN_DECOR,
        TrogRoom::Strength => &STRENGTH_DECOR,
    }
}

/// A launched troglodyte room: everything a scene or a fixture needs.
#[derive(Debug, Clone)]
pub struct TrogRun {
    pub room: TrogRoom,
    pub kit: EnvKit,
    pub fixture: &'static Fixture,
    /// Respawn columns: the start column (the checkpoint lies behind it).
    pub rules: &'static DemoRules,
    pub loaded: Loaded,
}

/// The env runtime of a launched room: the derived exit gate of a `defeat_roster` room, placed on
/// the walking line (the room has no prop, hazard or sticker).
pub fn env_runtime(run: &TrogRun) -> Result<EnvRuntime, String> {
    let enc = &run.loaded.encounter;
    Ok(EnvRuntime::new(enc)?.in_room(run.loaded.room.floor_y, &[], Some(enc.exit)))
}

/// A room with a troglodyte holds no other kind: any other roster kind is refused, never dropped
/// (the Hoarder room's and the hazard room's rule).
fn trogs_only(roster: &[encounter::RosterEntry]) -> Result<(), String> {
    match roster.iter().find(|e| e.kind.trog().is_none()) {
        Some(e) => Err(format!(
            "roster {}: {:?} is not a troglodyte-room kind (troglodytes only)",
            e.id, e.kind
        )),
        None => Ok(()),
    }
}

/// Load a troglodyte room, compose its kit and build the fixture and rules. The room is dressed
/// with the AREA-TROG-01 kit ([`crate::trog_kit`]).
pub fn launch(root: &Path, room: TrogRoom) -> Result<TrogRun, String> {
    launch_with(root, room, true)
}

/// [`launch`] with the dressing switched: `dressed = false` composes the room with the plain
/// Boiler kit (the TROG-A greybox composition), for the tests that prove the dressing is
/// presentation only.
pub fn launch_with(root: &Path, room: TrogRoom, dressed: bool) -> Result<TrogRun, String> {
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
             sidecar {:#018x} vs {:#018x}): re-pin src/f01_trog.rs on purpose",
            loaded.room_hash,
            room.room_hash(),
            loaded.sidecar_hash,
            room.encounter_hash()
        ));
    }
    if loaded.encounter.story_pin != STORY_PIN
        || !crate::story_unlock::StoryTable::active().has_room_pin(&loaded.encounter.story_pin)
    {
        return Err(format!(
            "{path}: the story pin {:?} is not the camp's {STORY_PIN:?}",
            loaded.encounter.story_pin
        ));
    }
    trogs_only(&loaded.encounter.roster)?;
    let mut kit = crate::trog_kit::compose_room_kit(root, &loaded, room, dressed)?;
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
    // One small leak per launch (a process launches a room once): the combat state borrows its
    // rules for `'static`.
    let rules: &'static DemoRules = Box::leak(Box::new(DemoRules {
        checkpoints: Box::leak(columns.into_boxed_slice()),
    }));
    let no_goblins: Vec<EnemySpawn> = Vec::new();
    let fixture: &'static Fixture = Box::leak(Box::new(Fixture {
        name: room.scene_name(),
        summary: match room {
            TrogRoom::Run => {
                "TROG-A: the troglodyte run (the Pygmy, the Basher and the Virtuoso, one at a \
                 time); greybox, no art review yet"
            }
            TrogRoom::Strength => {
                "TROG-B: the Test of Strength (the Virtuoso, an 80 px static bench and the \
                 Basher); greybox, no art review yet"
            }
        },
        seed: FIXTURE_SEED,
        carl_x: start,
        enemies: Box::leak(no_goblins.into_boxed_slice()),
        shore_dx: None,
        boss: None,
        llamas: &[],
        engineers: &[],
        charges: &[],
        script: WALK_RIGHT,
        ticks: SMOKE_TICKS,
        room: crate::combat_scene::FixtureRoom::Corridor,
    }));
    Ok(TrogRun {
        room,
        kit,
        fixture,
        rules,
        loaded,
    })
}

/// The combat state of a launched room under a run's story flags and `--cf02-seed`: the CF02
/// rules, Donut with the biscuit and a full pool, the sidecar's story pin under the program
/// flags (the camp's), the checkpoint rules, the env step with the exit gate, the slot
/// troglodytes and the companion's slot feed. The binary's scene and the PLAYTRACE verifier both
/// call it through [`crate::f01_late::trog_start`].
pub fn combat_for(
    run: &TrogRun,
    scene: &mut SceneSim,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
    cf02_seed: Option<u64>,
) -> Result<CombatState, String> {
    let slot_room = SlotRoom {
        floor_y: scene.world.hero_foot_y(),
        walls: (0.0, run.kit.width as f32),
    };
    let spawn =
        spawn_from_roster(&run.loaded.encounter, slot_room, false).map_err(|e| e.to_string())?;
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
        .map_err(|e| e.to_string())?
        .with_slot_feed();
    combat.sync_env(scene);
    Ok(combat)
}

/// [`combat_for`] under no story flag and no seed (the fixtures' state).
pub fn combat_of(run: &TrogRun, scene: &mut SceneSim) -> Result<CombatState, String> {
    combat_for(
        run,
        scene,
        CarlProgFlags::default(),
        DonutProgFlags::default(),
        None,
    )
}

/// The console line of a launch.
pub fn entry_line(run: &TrogRun) -> String {
    let e = &run.loaded.encounter;
    let room = &run.loaded.room;
    let kinds = |species: crate::trog::Species| {
        e.roster
            .iter()
            .filter(|r| r.kind == RosterKind::Trog(species))
            .count()
    };
    format!(
        "{}_ENTRY room={} encounter={} pin={} spawn_carl={} spawn_donut={} checkpoints={:?} \
         goal={:?} roster={} pygmies={} bashers={} virtuosos={} statics={} listed=true env=live \
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
        kinds(crate::trog::Species::Pygmy),
        kinds(crate::trog::Species::Basher),
        kinds(crate::trog::Species::Virtuoso),
        e.statics.len(),
        run.loaded.room_hash,
        run.loaded.sidecar_hash,
    )
}

/// The pins `playtrace::live::data_pins` stamps for this room (scoped to the room by
/// [`crate::f01_late::pin_applies`]), and the tuning pin (scoped to every trog room).
pub fn data_pins() -> [(&'static str, u64); 5] {
    [
        (RUN_ROOM_PIN_KEY, RUN_ROOM_HASH),
        (RUN_ENCOUNTER_PIN_KEY, RUN_ENCOUNTER_HASH),
        (STRENGTH_ROOM_PIN_KEY, STRENGTH_ROOM_HASH),
        (STRENGTH_ENCOUNTER_PIN_KEY, STRENGTH_ENCOUNTER_HASH),
        (crate::trog::PIN_KEY, crate::trog::TUNING_HASH),
    ]
}

#[cfg(test)]
mod tests;
