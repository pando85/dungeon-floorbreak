//! ACT1: Act I at the **start** of the late-Floor-1 run
//! (`docs/01-design/floor-01-act-01-integration.md`, ACT1-PLAN). Three rooms before the
//! corral: the **chase** (`act1_chase`, "The First Chase", ACT1-B, I-01 and I-02, the Murder
//! Dozer opening: DOZER-CORE under its `opening` profile), the **shore** (`act1_shore`,
//! "The Rotten Shore", I-03, Carl's first revenge on two goblins and the rotten beam) and the
//! **guild** (`act1_guild`, "The Guild Door", I-04, GUILD-BEAT-01, Donut's biscuit).
//!
//! The chase (ACT1-B) holds one `dozer` entry that runs room 3's machine with the opening
//! tuning (`dozer::DozerTuning::opening`): a quiet lane, an emergence at the window, two timber
//! sets and two humps, a charge that commits only when every hero can reach the authored pocket
//! (the lit alcove) and the charge reaches the barricade, a barricade lock that only the machine
//! opens (`dozer_contact`) and a wedge at the arch. The humps and the alcove ledge are 16 px
//! `solid` statics of the sidecar pushed into the kit here; the soffit, the timbers, the window
//! and the arch are decor (presentation only). Nobody can hurt the machine.
//!
//! They launch through `--scene f01_late --f01-room <id>` and walk in `--f01-run`: no
//! scene arm of their own, like the Ring and the quiet rooms. Both are **story-less**
//! like the corral: the sidecar's `story_pin` is a label with no table row (checked
//! here to stay so), the story setup is [`SceneKind::Corral`]'s (it holds and arms
//! nothing under any loadout) and the run carries an empty ledger across them
//! ([`crate::f01_run`]'s `carry_for`).
//!
//! Bevy-free. [`launch`] loads the room and its sidecar through the one room parser
//! ([`crate::encounter::load`], the whole validation, `donut_unlocked` lock included),
//! composes the kit like the Boiler ([`crate::f01_boiler::compose_kit`]) with the
//! AREA-ACT1-01 dressing ([`crate::act1_kit`]; [`launch_with`] `false` is the plain
//! **labelled greybox** composition), pushes the guild's lane ceiling slab and builds the fixture and
//! the checkpoint rules from the sidecar. [`combat_for`] builds the combat state the
//! scene, the fixtures and the PLAYTRACE verifier share:
//!
//! * the shore holds the sidecar's two goblins (a brawler and a coward, ACT1-IMP-1) and
//!   the rotten shore's strike column ([`SHORE_DX`], [`crate::shore`]); its exit is the
//!   derived gate of a `defeat_roster` room and opens when both are defeated by any
//!   credited cause. Donut is locked there (the swat, no missile, her AI never casts);
//! * the guild holds no enemy. Donut starts locked with a **guild zone**
//!   ([`GUILD_ZONE_X`]) in front of the guide; her pivot entering it grants the biscuit
//!   (`ManaEvent::UnlockGranted`, `primary_ranged`), and the `guild_door` lock seals the
//!   exit until her mana is unlocked (ACT1-AD13). The door is a pure function of her mana
//!   (`EnvSolidKind::ManaLock`): no flag, no hash term;
//! * under `--f01-loadout all` or a `--donut-light-feet` / `--donut-puddle` flag the biscuit
//!   is held from tick 0 in both rooms (DP-Q06, DP-Q10): the door is open at tick 0 and no
//!   `UnlockGranted` is emitted.
//!
//! The decor tables ([`decor`]) are presentation only (ACT1-AD10, ACT1-AD11): flat
//! labelled rectangles the binary draws for a plain room (a dressed room draws none); never solids, never in the sim, never hashed.
//!
//! Each room and sidecar is pinned ([`SHORE_ROOM_HASH`], [`GUILD_ENCOUNTER_HASH`], ...):
//! [`launch`] refuses an edited file, a test pins each constant to its file, and an edit
//! is a deliberate re-pin here.

use std::path::Path;

use crate::camera_lab::room::IRect;
use crate::carl_prog::CarlProgFlags;
use crate::cf02::Temperament;
use crate::combat_scene::{CombatState, DemoRules, EnvRuntime, Fixture, Hold};
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::donut_prog::DonutProgFlags;
use crate::encounter::{self, Goal, Loaded, LockEvent};
use crate::env_kit::{EnvKit, IRect as KitRect};
use crate::input::RawInput;
use crate::scene_enemy::{SlotRoom, spawn_from_roster};
use crate::story_unlock::{SceneKind, StorySetup, StoryTable};

/// The shore's sidecar and room file.
pub const SHORE_ENCOUNTER_PATH: &str = "assets/levels/floor01/f01_act1_shore.encounter.json";
pub const SHORE_ROOM_PATH: &str = "assets/levels/floor01/f01_act1_shore.room.json";
/// The chase's sidecar and room file (ACT1-B).
pub const CHASE_ENCOUNTER_PATH: &str = "assets/levels/floor01/f01_act1_chase.encounter.json";
pub const CHASE_ROOM_PATH: &str = "assets/levels/floor01/f01_act1_chase.room.json";
/// The guild's sidecar and room file.
pub const GUILD_ENCOUNTER_PATH: &str = "assets/levels/floor01/f01_act1_guild.encounter.json";
pub const GUILD_ROOM_PATH: &str = "assets/levels/floor01/f01_act1_guild.room.json";

/// FNV-1a 64 of the room file and of the sidecar (the files live under
/// `assets/levels`, so the hashes are constants, the Boiler's precedent).
pub const SHORE_ROOM_HASH: u64 = 0x0855_765d_aa55_a38c;
pub const SHORE_ENCOUNTER_HASH: u64 = 0x3881_9eb7_e8d5_db56;
pub const GUILD_ROOM_HASH: u64 = 0xf9f5_b1d5_2ed7_4534;
pub const GUILD_ENCOUNTER_HASH: u64 = 0x12d4_80e0_013f_5b38;
pub const CHASE_ROOM_HASH: u64 = 0xc850_ebe7_523f_6be6;
pub const CHASE_ENCOUNTER_HASH: u64 = 0x70e6_a2b4_4658_c40a;
/// The `data_pins()` keys, scoped to their room by [`crate::f01_late::pin_applies`].
pub const SHORE_ROOM_PIN_KEY: &str = "f01_act1_shore_room";
pub const SHORE_ENCOUNTER_PIN_KEY: &str = "f01_act1_shore_encounter";
pub const GUILD_ROOM_PIN_KEY: &str = "f01_act1_guild_room";
pub const GUILD_ENCOUNTER_PIN_KEY: &str = "f01_act1_guild_encounter";
pub const CHASE_ROOM_PIN_KEY: &str = "f01_act1_chase_room";
pub const CHASE_ENCOUNTER_PIN_KEY: &str = "f01_act1_chase_encounter";

/// A fixture seed is only recorded: the rooms have no random draw, but the goblins and
/// the companion draw from `--cf02-seed`.
const FIXTURE_SEED: u64 = 0x0F01_AC71;

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

/// The rotten shore's strike column in pixels right of Carl's start column: strike local
/// 367 (route 2975 re-based by -2608) minus Carl's spawn 112 (the `f01_mastery`
/// precedent, [`crate::combat_scene::Fixture::shore_dx`]). The fall footprint is local
/// 392..472.
pub const SHORE_DX: i32 = 255;
/// The goblins' authored temperaments (ACT1-IMP-1): `gob_a` a brawler, `gob_b` a coward,
/// so the second arrives late (I-03 G1). The sidecar format carries a temperament on a
/// mounted entry only, so they are data of this module, pinned by a test.
pub const SHORE_TEMPERAMENTS: [Temperament; 2] = [Temperament::Brawler, Temperament::Coward];
/// Donut's guild zone: her pivot entering this half-open x range grants the biscuit
/// (GUILD-BEAT-01), in front of the guide's mark (x 380).
pub const GUILD_ZONE_X: (i32, i32) = (336, 376);
/// GUILD-VERB-01: the grant strongbox's left and right faces (`act1_kit::BOX_AT` x, 24 px wide),
/// layout px. Interact opens it (GUILDV-AD1); the grant itself stays the zone's (GUILDV-Q1).
pub const GUILD_BOX_FACES: (i32, i32) = (340, 364);
/// GUILD-VERB-01: the guide Mordecai's column (the `guide` decor's centre, the refuge mark).
pub const GUILD_GUIDE_X: i32 = 380;
/// The guild's interact site: the strongbox and the guide.
pub const GUILD_INTERACT: crate::interact::InteractSite = crate::interact::InteractSite {
    box_faces: Some(GUILD_BOX_FACES),
    guide_x: Some(GUILD_GUIDE_X),
};
/// The guild door's sidecar lock id.
pub const GUILD_DOOR: &str = "guild_door";
/// The chase's barricade lock id (`dozer_contact`).
pub const CHASE_BARRICADE: &str = "barricade";
/// The guild's lane ceiling (the DP-Q45 slab, the refuge's rule).
pub const GUILD_CEILING_PX: i32 = 228;
/// Donut's mana at the start under the All loadout.
const UNLOCKED_MANA: u8 = 8;

/// One of the Act I rooms in the listed run (ACT1-AD1), in run order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Act1Room {
    /// ACT1-B, I-01 and I-02: the false calm and the Murder Dozer opening.
    Chase,
    /// I-03: the first two goblins and the rotten shore.
    Shore,
    /// I-04, GUILD-BEAT-01: the guide, the biscuit and the door.
    Guild,
}

impl Act1Room {
    pub const ALL: [Act1Room; 3] = [Act1Room::Chase, Act1Room::Shore, Act1Room::Guild];

    /// The manifest id (`--f01-room`).
    pub const fn id(self) -> &'static str {
        match self {
            Self::Chase => "act1_chase",
            Self::Shore => "act1_shore",
            Self::Guild => "act1_guild",
        }
    }

    /// The fixture name (a recording's `reset.name`).
    pub const fn scene_name(self) -> &'static str {
        match self {
            Self::Chase => "f01_act1_chase",
            Self::Shore => "f01_act1_shore",
            Self::Guild => "f01_act1_guild",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.id() == id)
    }

    pub const fn encounter_path(self) -> &'static str {
        match self {
            Self::Chase => CHASE_ENCOUNTER_PATH,
            Self::Shore => SHORE_ENCOUNTER_PATH,
            Self::Guild => GUILD_ENCOUNTER_PATH,
        }
    }

    pub const fn room_path(self) -> &'static str {
        match self {
            Self::Chase => CHASE_ROOM_PATH,
            Self::Shore => SHORE_ROOM_PATH,
            Self::Guild => GUILD_ROOM_PATH,
        }
    }

    /// FNV-1a 64 of the room file.
    pub const fn room_hash(self) -> u64 {
        match self {
            Self::Chase => CHASE_ROOM_HASH,
            Self::Shore => SHORE_ROOM_HASH,
            Self::Guild => GUILD_ROOM_HASH,
        }
    }

    /// FNV-1a 64 of the sidecar.
    pub const fn encounter_hash(self) -> u64 {
        match self {
            Self::Chase => CHASE_ENCOUNTER_HASH,
            Self::Shore => SHORE_ENCOUNTER_HASH,
            Self::Guild => GUILD_ENCOUNTER_HASH,
        }
    }

    /// The `data_pins()` key of the room file.
    pub const fn room_pin_key(self) -> &'static str {
        match self {
            Self::Chase => CHASE_ROOM_PIN_KEY,
            Self::Shore => SHORE_ROOM_PIN_KEY,
            Self::Guild => GUILD_ROOM_PIN_KEY,
        }
    }

    /// The `data_pins()` key of the sidecar.
    pub const fn encounter_pin_key(self) -> &'static str {
        match self {
            Self::Chase => CHASE_ENCOUNTER_PIN_KEY,
            Self::Shore => SHORE_ENCOUNTER_PIN_KEY,
            Self::Guild => GUILD_ENCOUNTER_PIN_KEY,
        }
    }

    /// The sidecar's story pin: a label with no table row (checked by [`launch`]).
    pub const fn story_pin(self) -> &'static str {
        match self {
            Self::Chase => "f01_pin_act1_chase_entry",
            Self::Shore => "f01_pin_act1_shore_entry",
            Self::Guild => "f01_pin_act1_guild_entry",
        }
    }

    /// The marker of the headless smoke and the entry line.
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Chase => "F01_ACT1_CHASE",
            Self::Shore => "F01_ACT1_SHORE",
            Self::Guild => "F01_ACT1_GUILD",
        }
    }
}

/// A presentation-only rectangle of a room (ACT1-AD10, ACT1-AD11): a flat labelled
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

/// The chase's decor (ACT1-B, ENV-02 option R, presentation only): the S3 soffit (the lowered
/// ceiling, not a solid: no jump reaches it, ACT1-AD8), the two timber sets (smashed by the
/// front, `TimberSmashed`), the window the machine comes out of, the lit `alcove` (drawn
/// **behind** the heroes, the brightest shape in S4: spotting it is the player's whole decision,
/// #179 C4) and the S5 arch the machine wedges in. Their labels are the names the binary draws.
const CHASE_DECOR: [Decor; 6] = [
    d("soffit", 976, 272, 704, 96),
    d("timber_a", 1104, 456, 80, 104),
    d("timber_b", 1424, 456, 80, 104),
    d("window", 696, 342, 192, 218),
    d("alcove", 2072, 460, 80, 104),
    d("arch", 2608, 342, 248, 238),
];

/// The shore's decor: the machine wedged in the arch (route 2664..2856), wheel spinning,
/// not lethal and not in the sim (ACT1-AD10: the chase room resolves it).
const SHORE_DECOR: [Decor; 1] = [d("wedged_dozer", 56, 440, 192, 140)];

/// The guild's decor: the guide at the `refuge_guild` Mordecai mark (x 380), no text
/// (ACT1-AD11). The binary may draw the existing `mordecai_h44` sprite there.
const GUILD_DECOR: [Decor; 1] = [d("guide", 368, 536, 24, 44)];

/// The decor of a room (presentation only).
pub const fn decor(room: Act1Room) -> &'static [Decor] {
    match room {
        Act1Room::Chase => &CHASE_DECOR,
        Act1Room::Shore => &SHORE_DECOR,
        Act1Room::Guild => &GUILD_DECOR,
    }
}

/// A launched Act I room: everything a scene or a fixture needs.
#[derive(Debug, Clone)]
pub struct Act1Run {
    pub room: Act1Room,
    pub kit: EnvKit,
    pub fixture: &'static Fixture,
    /// Respawn columns: the checkpoint's (or the start column when it lies ahead).
    pub rules: &'static DemoRules,
    pub loaded: Loaded,
    /// The goblins' wake gates, in enemy index order (empty when none has one).
    pub wakes: Vec<Option<crate::goblin::WakeGate>>,
    /// The room is composed with the AREA-ACT1-01 dressing (the scene then draws no flat decor
    /// rectangle, timber, barricade or door bar). `false` for the plain greybox composition.
    pub dressed: bool,
}

impl Act1Run {
    /// The guild door's rectangle (layout px, y down), if the room has one.
    pub fn door(&self) -> Option<IRect> {
        self.loaded
            .encounter
            .locks
            .iter()
            .find(|l| l.id == GUILD_DOOR)
            .map(|l| l.solid)
    }
}

impl Act1Run {
    /// The chase's barricade rectangle (layout px, y down), if the room has one (ACT1-B).
    pub fn barricade(&self) -> Option<IRect> {
        self.loaded
            .encounter
            .locks
            .iter()
            .find(|l| l.id == CHASE_BARRICADE)
            .map(|l| l.solid)
    }
}

/// The env runtime of a launched room: the guild's one `donut_unlocked` lock, or the
/// shore's derived exit gate (a `defeat_roster` room), placed on the walking line.
pub fn env_runtime(run: &Act1Run) -> Result<EnvRuntime, String> {
    Ok(EnvRuntime::new(&run.loaded.encounter)?
        .with_wake_gates(run.wakes.clone())
        .in_room(
            run.loaded.room.floor_y,
            &[],
            Some(run.loaded.encounter.exit),
        ))
}

/// Load an Act I room and its sidecar, compose its kit and build the fixture. The room is
/// dressed with the AREA-ACT1-01 kit ([`crate::act1_kit`]).
pub fn launch(root: &Path, room: Act1Room) -> Result<Act1Run, String> {
    launch_with(root, room, true)
}

/// [`launch`] with the dressing switched: `dressed = false` composes the room with the plain
/// Boiler kit (the ACT1-A greybox composition), for the tests that prove the dressing is
/// presentation only.
pub fn launch_with(root: &Path, room: Act1Room, dressed: bool) -> Result<Act1Run, String> {
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
             sidecar {:#018x} vs {:#018x}): re-pin src/f01_act1.rs on purpose",
            loaded.room_hash,
            room.room_hash(),
            loaded.sidecar_hash,
            room.encounter_hash()
        ));
    }
    // The story pin is a label (the corral's rule): a table row under it would be held.
    if StoryTable::active().has_pin(&loaded.encounter.story_pin) {
        return Err(format!(
            "{path}: the story pin {:?} has rows in the story table; the Act I rooms hold \
             nothing at entry (ACT1-AD1)",
            loaded.encounter.story_pin
        ));
    }
    if loaded.encounter.story_pin != room.story_pin() {
        return Err(format!(
            "{path}: the story pin {:?} is not {:?}",
            loaded.encounter.story_pin,
            room.story_pin()
        ));
    }
    check_shape(room, &loaded)?;
    let mut kit = crate::act1_kit::compose(root, &loaded, room, dressed)?;
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
    // ACT1-B: the plain `solid` statics (the chase's humps and the alcove ledge) are physical
    // lane steps: they join the kit's solids (the sidecar statics carry no collision of their
    // own unless they hold a machine, like the Hoarder's bank).
    for st in loaded
        .encounter
        .statics
        .iter()
        .filter(|st| st.kind == encounter::StaticKind::Solid && st.slide.is_none())
    {
        kit.solid_rects.push(KitRect {
            x: st.rect.x,
            y: st.rect.y,
            w: st.rect.w,
            h: st.rect.h,
        });
    }
    kit.blink_barriers = loaded.room.kit_blink_barriers();
    let start = loaded.room.spawn_carl;
    let checkpoint = loaded.encounter.checkpoint;
    let checkpoint_x = checkpoint.x + checkpoint.w / 2;
    // The checkpoint of an Act I room lies behind the start column (it is the post-chase
    // column of ENV-02, ACT1-AD2): a respawn goes there (ACT1-IMP-2). The columns are
    // ascending, so a checkpoint ahead of the start keeps the usual two.
    let columns: Vec<i32> = if checkpoint_x > start {
        vec![start, checkpoint_x]
    } else {
        vec![checkpoint_x]
    };
    for x in columns.iter().chain([&start]) {
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
    // The chase holds exactly one slot kind, the machine; the shore and the guild hold none.
    let slot_kinds = usize::from(room == Act1Room::Chase);
    if spawn.slot.len() != slot_kinds {
        return Err(format!(
            "{path}: the room holds {} slot enemies, not {slot_kinds}",
            spawn.slot.len()
        ));
    }
    let enemies = crate::f01_boiler::existing_enemies(&loaded.encounter, &spawn.existing, false)?;
    if !enemies.llamas.is_empty() || !enemies.engineers.is_empty() {
        return Err(format!("{path}: an Act I room holds goblins only"));
    }
    let wakes = enemies.wakes();
    // One small leak per launch (a process launches a room once): the combat
    // state borrows its rules for `'static`.
    let rules: &'static DemoRules = Box::leak(Box::new(DemoRules {
        checkpoints: Box::leak(columns.into_boxed_slice()),
    }));
    let fixture: &'static Fixture = Box::leak(Box::new(Fixture {
        name: room.scene_name(),
        summary: match room {
            Act1Room::Chase => {
                "ACT1-B: the first chase (a quiet lane, the machine in the window, the alcove and a \
                 barricade only it can break); greybox, no art yet"
            }
            Act1Room::Shore => {
                "ACT1-A: the rotten shore (two goblins, the rotten beam, Donut locked); greybox, \
                 no art yet"
            }
            Act1Room::Guild => {
                "ACT1-A: the guild door (the guide, Donut's biscuit zone and a door that waits \
                 for her mana); greybox, no art yet"
            }
        },
        seed: FIXTURE_SEED,
        carl_x: start,
        enemies: Box::leak(enemies.goblins.into_boxed_slice()),
        shore_dx: match room {
            Act1Room::Shore => Some(SHORE_DX),
            Act1Room::Chase | Act1Room::Guild => None,
        },
        boss: None,
        llamas: &[],
        engineers: &[],
        charges: &[],
        script: WALK_RIGHT,
        ticks: SMOKE_TICKS,
        room: crate::combat_scene::FixtureRoom::Corridor,
    }));
    Ok(Act1Run {
        room,
        kit,
        fixture,
        rules,
        loaded,
        wakes,
        dressed: dressed && crate::act1_kit::is_dressed(root, room),
    })
}

/// The shape rules of each room that the generic validator cannot see (the zone, the door
/// and the roster of the module's own data).
fn check_shape(room: Act1Room, loaded: &Loaded) -> Result<(), String> {
    let e = &loaded.encounter;
    match room {
        Act1Room::Chase => {
            if e.goal != Goal::ReachExit
                || e.roster.len() != 1
                || loaded.ceiling_px.is_some()
                || e.locks.len() != 1
                || e.locks[0].id != CHASE_BARRICADE
                || e.locks[0].opens_on != LockEvent::DozerContact
            {
                return Err(
                    "the chase is one dozer under reach_exit, no ceiling and the \
                            barricade lock"
                        .into(),
                );
            }
            let Some(site) = &e.dozer else {
                return Err("the chase needs its dozer block".into());
            };
            if site.profile != crate::dozer::SiteProfile::Opening
                || site.trigger_by != crate::dozer::TriggerBy::Rearmost
                || site.pockets.len() != 1
                || site.timbers.len() != 2
                || !site.wedge
            {
                return Err("the chase's dozer block is the opening profile with the \
                            rearmost trigger, one pocket, two timbers and the wedge"
                    .into());
            }
        }
        Act1Room::Shore => {
            if e.goal != Goal::DefeatRoster || !e.locks.is_empty() || e.roster.len() != 2 {
                return Err("the shore is two goblins under defeat_roster and no lock".into());
            }
            if loaded.room.spawn_carl + SHORE_DX != 367 {
                return Err(format!(
                    "the shore's strike column {} is not local 367",
                    loaded.room.spawn_carl + SHORE_DX
                ));
            }
        }
        Act1Room::Guild => {
            if e.goal != Goal::ReachExit || !e.roster.is_empty() || loaded.ceiling_px.is_none() {
                return Err("the guild is an empty roster under reach_exit with a ceiling".into());
            }
            let [door] = e.locks.as_slice() else {
                return Err("the guild has exactly one lock, the door".into());
            };
            if door.id != GUILD_DOOR || door.opens_on != LockEvent::DonutUnlocked {
                return Err("the guild's lock is the donut_unlocked door".into());
            }
            // The zone lies after the spawn and the checkpoint and before the door, so
            // Donut always walks it before she can reach the door.
            if GUILD_ZONE_X.0 <= e.checkpoint.right()
                || GUILD_ZONE_X.0 <= loaded.room.spawn_donut
                || GUILD_ZONE_X.1 > door.solid.x
            {
                return Err("the guild zone must lie after the spawn and before the door".into());
            }
        }
    }
    Ok(())
}

/// The story setup of an Act I room: [`SceneKind::Corral`]'s, which no pin row and no
/// trigger names, so it holds and arms nothing under any flags (the rooms are story-less
/// like the corral, ACT1-AD1).
pub fn story_setup(carl: CarlProgFlags, donut: DonutProgFlags) -> StorySetup {
    crate::donut_prog::story_setup(carl, donut, SceneKind::Corral)
}

/// Donut's setup in `room` under a run's flags (ACT1-AD13): locked under the Story setup or
/// no flag (the guild with its zone), the biscuit held from tick 0 under the All loadout
/// or any `--donut-light-feet` / `--donut-puddle` flag (DP-Q06, DP-Q10).
pub fn donut_setup(room: Act1Room, donut: DonutProgFlags) -> DonutSetup {
    if donut.light_feet || donut.puddle {
        return DonutSetup::unlocked(UNLOCKED_MANA);
    }
    match room {
        Act1Room::Chase | Act1Room::Shore => DonutSetup::NONE,
        Act1Room::Guild => DonutSetup {
            guild_zone: Some((GUILD_ZONE_X.0 as f32, GUILD_ZONE_X.1 as f32)),
            ..DonutSetup::NONE
        },
    }
}

/// The combat state of a launched room under a run's story flags and `--cf02-seed`: the CF02
/// rules, Donut's setup ([`donut_setup`]), the room's (empty) story setup, the checkpoint
/// rules, the shore's goblins with their authored temperaments and the env step with the
/// shore's exit gate or the guild's door. The binary's scene and the PLAYTRACE verifier both
/// call it through [`crate::f01_late::act1_start`].
pub fn combat_for(
    run: &Act1Run,
    scene: &mut SceneSim,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
    cf02_seed: Option<u64>,
) -> Result<CombatState, String> {
    let mut combat = CombatState::new(run.fixture, &run.kit, scene)?
        .with_donut(donut_setup(run.room, donut))
        .try_with_story(story_setup(carl, donut))
        .map_err(|e| e.to_string())?
        .with_demo(run.rules)
        .with_cf02()
        .with_cf02_seed(cf02_seed);
    if run.room == Act1Room::Shore {
        combat = combat.with_goblin_temperaments(&SHORE_TEMPERAMENTS);
    }
    let mut combat = combat.with_env(env_runtime(run)?);
    if run.room == Act1Room::Guild {
        // GUILD-VERB-01: the Interact verb's targets (the strongbox and the guide). It mixes
        // nothing into the hash until a press is accepted.
        combat = combat.with_interact(GUILD_INTERACT);
    }
    if run.room == Act1Room::Chase {
        // ACT1-B: the machine spawns from the sidecar roster through ENEMY-SLOT, installed
        // **last** (a `with_*` builder after the slot would mark the attempt as retried).
        combat = combat
            .with_slot_enemies(slot_set(run, scene)?)
            .map_err(|e| e.to_string())?;
    }
    combat.sync_env(scene);
    Ok(combat)
}

/// The chase's machine through ENEMY-SLOT (the sidecar's `dozer` entry), for the combat
/// state and the fixtures' restart.
pub fn slot_set(run: &Act1Run, scene: &SceneSim) -> Result<crate::scene_enemy::SlotSet, String> {
    let room = SlotRoom {
        floor_y: scene.world.hero_foot_y(),
        walls: (0.0, run.kit.width as f32),
    };
    spawn_from_roster(&run.loaded.encounter, room, false)
        .map(|spawn| spawn.slot)
        .map_err(|e| e.to_string())
}

/// [`combat_for`] under no story flag and no seed (the fixtures' state).
pub fn combat_of(run: &Act1Run, scene: &mut SceneSim) -> Result<CombatState, String> {
    combat_for(
        run,
        scene,
        CarlProgFlags::default(),
        DonutProgFlags::default(),
        None,
    )
}

/// The console line of a launch. The guild reports `door=closed|open` and
/// `biscuit=held|zone|none` (`held` when Donut's mana is unlocked, `zone` when she is locked
/// and the guild zone is set, `none` otherwise); the shore reports its goblin count and the
/// strike column.
pub fn entry_line(run: &Act1Run, combat: &CombatState) -> String {
    let e = &run.loaded.encounter;
    let room = &run.loaded.room;
    let biscuit = if combat.donut_unlocked() {
        "held"
    } else if matches!(run.room, Act1Room::Guild) {
        "zone"
    } else {
        "none"
    };
    let extra = match run.room {
        Act1Room::Chase => {
            let state = crate::dozer_art::dozer_of(combat).map_or("none", |d| d.state.name());
            let site = e.dozer.as_ref();
            format!(
                " dozer={state} pocket={} timbers={} trigger_x={} arm_x={} barricade=closed",
                site.map_or(0, |s| s.pockets.len()),
                site.map_or(0, |s| s.timbers.len()),
                site.map_or(0, |s| s.trigger_x),
                crate::dozer::DozerTuning::opening()
                    .charge
                    .map_or(0, |c| c.arm_x),
            )
        }
        Act1Room::Shore => format!(
            " goblins={} shore_strike_x={} biscuit={biscuit}",
            run.fixture.enemies.len(),
            room.spawn_carl + SHORE_DX,
        ),
        Act1Room::Guild => {
            let door = if combat.donut_unlocked() {
                "open"
            } else {
                "closed"
            };
            format!(
                " door={door} biscuit={biscuit} zone={}..{}",
                GUILD_ZONE_X.0, GUILD_ZONE_X.1
            )
        }
    };
    format!(
        "{}_ENTRY room={} encounter={} pin={} spawn_carl={} spawn_donut={} \
         checkpoints={:?} goal={:?} roster={}{extra} env=live greybox=true \
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

/// ACT1-B: whether the chase's alcove hint is live at `tick`: the party respawned at the
/// checkpoint (the latest `Respawn` event of the log) less than
/// [`crate::hud::ALCOVE_HINT_TICKS`] ticks ago. A respawn follows a hero defeat, so a fresh run
/// (no respawn) never shows it. Presentation only: the scene feeds it to the HUD, nothing in
/// the simulation reads it.
pub fn alcove_hint_live(combat: &CombatState, tick: u64) -> bool {
    // Only the chase (the opening profile's machine) has an alcove.
    crate::dozer_art::dozer_of(combat).is_some_and(|d| d.has_charge())
        && combat
            .events
            .iter()
            .rev()
            .find_map(|e| match *e {
                crate::combat::CombatEvent::Respawn { tick, .. } => Some(tick),
                _ => None,
            })
            .is_some_and(|at| tick.saturating_sub(at) < crate::hud::ALCOVE_HINT_TICKS)
}

/// The pins `playtrace::live::data_pins` stamps (scoped to their room by
/// [`crate::f01_late::pin_applies`]).
pub const fn data_pins() -> [(&'static str, u64); 7] {
    [
        (CHASE_ROOM_PIN_KEY, CHASE_ROOM_HASH),
        (CHASE_ENCOUNTER_PIN_KEY, CHASE_ENCOUNTER_HASH),
        (
            crate::dozer::OPENING_PIN_KEY,
            crate::dozer::OPENING_TUNING_HASH,
        ),
        (SHORE_ROOM_PIN_KEY, SHORE_ROOM_HASH),
        (SHORE_ENCOUNTER_PIN_KEY, SHORE_ENCOUNTER_HASH),
        (GUILD_ROOM_PIN_KEY, GUILD_ROOM_HASH),
        (GUILD_ENCOUNTER_PIN_KEY, GUILD_ENCOUNTER_HASH),
    ]
}

#[cfg(test)]
mod tests;
