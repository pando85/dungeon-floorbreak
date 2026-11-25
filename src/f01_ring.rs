//! RING-A wire and RING-B wire: the two rooms of the Broken Ring as rooms of
//! the late-Floor-1 slice (`docs/07-agents/f01-playtest-integration-plan.md`
//! section 15): room 4a `ring_court` (the crowned breach and the Shutter
//! Court) and room 4b `ring_split` (the split passage and the light beyond
//! stone). They launch only through `--scene f01_late --f01-room ring_court`
//! (or `ring_split`): no scene arm of their own (KR22).
//!
//! Bevy-free. [`launch`] loads a room and its sidecar through the one room
//! parser ([`crate::encounter::load`], which runs the whole validation, the
//! KR3 shutter rule included), composes the kit the way the Boiler does
//! ([`crate::f01_boiler::compose_kit`], with the DP-Q45 lane ceiling slab of
//! `ceiling_px` 228) and builds the fixture and the checkpoint rules from the
//! sidecar. [`combat_for`] builds the combat state the scene, the fixtures and
//! the PLAYTRACE verifier share:
//!
//! * the court's **static closed shutter** (`initial: closed`, KR3 clause 5)
//!   is an env prop of the env step: one block (its closed footprint,
//!   RINGW-AD8 after review F5 of #271; since RING-FU-A a blast's line of
//!   sight is blocked along slice seams too, and the one block is kept) appended to
//!   `scene.world.solids` by the KR6 sync (Timber, so the companion's nav
//!   updates stay incremental), standing for the whole attempt. Collision, the
//!   goblin's `wall_stop` (the CF02-B slam), the Magic Missile and the
//!   companion's `NavDriver::sync` read them unchanged. The moving shutter,
//!   its arm rule and its pins are RING-C wire's;
//! * the **two goblins of each sidecar** are the fixture's goblins, chasing, in
//!   roster order; an entry's `trigger_x` and `delay_ticks` are its FA-Q20 wake
//!   gate ([`crate::f01_boiler::wake_gate`], the one implementation);
//! * the story pin is the sidecar's `f01_pin_post_pile_boss` (KR12); the room
//!   grants nothing and arms no trigger;
//! * the exit is the sidecar's `exit` (`goal: reach_exit`): no exit gate.
//!
//! The rooms' content is pinned per room ([`RingRoom::room_hash`],
//! [`RingRoom::encounter_hash`]): [`launch`] refuses an edited file, a test
//! pins each constant to its file, and an edit is a deliberate re-pin here.
//!
//! Adopted defaults of this step (`RINGW-AD*`, listed in the PR): AD1 both
//! rooms are **listed** (launchable, milestone `greybox`) after `hazards` and
//! before `juicer`, so a floor run walks them; AD2 RING-B wire is grouped into
//! this step (the static gate and the two goblins run from the sidecar as
//! authored, so the launched court is never an open arch the data does not
//! describe); AD3 the roster honours FA-Q20's wake gates; AD4 the court's
//! shutter draws as a labelled greybox stand-in (`env_prop_art::pieces`, KR17)
//! until RING-D and AREA-RINGS-01; AD5 the rooms record under the
//! `f01_room` source with the fixture names `f01_ring_court` and
//! `f01_ring_split`.

use std::path::Path;

use crate::camera_lab::room::EnemyKind;
use crate::carl_prog::CarlProgFlags;
use crate::combat_scene::{CombatState, DemoRules, EnvRuntime, Fixture, Hold};
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::donut_prog::DonutProgFlags;
use crate::encounter::{self, Loaded, PropKind, RosterKind};
use crate::env_kit::{EnvKit, IRect as KitRect};
use crate::goblin::WakeGate;
use crate::input::RawInput;
use crate::scene_enemy::{SlotRoom, spawn_from_roster};

/// The court's sidecar and room file (RING-A core).
pub const COURT_ENCOUNTER_PATH: &str = "assets/levels/floor01/f01_ring_court.encounter.json";
pub const COURT_ROOM_PATH: &str = "assets/levels/floor01/f01_ring_court.room.json";
/// The split's sidecar and room file (RING-A core).
pub const SPLIT_ENCOUNTER_PATH: &str = "assets/levels/floor01/f01_ring_split.encounter.json";
pub const SPLIT_ROOM_PATH: &str = "assets/levels/floor01/f01_ring_split.room.json";

/// A fixture seed is only recorded: a live run draws its goblins'
/// temperaments from `--cf02-seed`.
const FIXTURE_SEED: u64 = 0x0F01_4A4E;

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

/// One of the two Ring rooms (KR1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RingRoom {
    /// Room 4a, R0 and R1: the crowned breach and the Shutter Court.
    Court,
    /// Room 4b, R2 and R3: the split passage and the light beyond stone.
    Split,
}

impl RingRoom {
    pub const ALL: [RingRoom; 2] = [RingRoom::Court, RingRoom::Split];

    /// The manifest id (`--f01-room`).
    pub const fn id(self) -> &'static str {
        match self {
            Self::Court => "ring_court",
            Self::Split => "ring_split",
        }
    }

    /// The fixture name (a recording's `reset.name`).
    pub const fn scene_name(self) -> &'static str {
        match self {
            Self::Court => "f01_ring_court",
            Self::Split => "f01_ring_split",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.id() == id)
    }

    pub const fn encounter_path(self) -> &'static str {
        match self {
            Self::Court => COURT_ENCOUNTER_PATH,
            Self::Split => SPLIT_ENCOUNTER_PATH,
        }
    }

    pub const fn room_path(self) -> &'static str {
        match self {
            Self::Court => COURT_ROOM_PATH,
            Self::Split => SPLIT_ROOM_PATH,
        }
    }

    /// FNV-1a 64 of the room file (the files live under `assets/levels`, so
    /// the hash is a constant, the Boiler's precedent).
    pub const fn room_hash(self) -> u64 {
        match self {
            Self::Court => COURT_ROOM_HASH,
            Self::Split => SPLIT_ROOM_HASH,
        }
    }

    /// FNV-1a 64 of the sidecar.
    pub const fn encounter_hash(self) -> u64 {
        match self {
            Self::Court => COURT_ENCOUNTER_HASH,
            Self::Split => SPLIT_ENCOUNTER_HASH,
        }
    }

    /// The `data_pins()` key of the room file.
    pub const fn room_pin_key(self) -> &'static str {
        match self {
            Self::Court => COURT_ROOM_PIN_KEY,
            Self::Split => SPLIT_ROOM_PIN_KEY,
        }
    }

    /// The `data_pins()` key of the sidecar.
    pub const fn encounter_pin_key(self) -> &'static str {
        match self {
            Self::Court => COURT_ENCOUNTER_PIN_KEY,
            Self::Split => SPLIT_ENCOUNTER_PIN_KEY,
        }
    }

    /// The marker of the headless smoke and the entry line.
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Court => "F01_RING_COURT",
            Self::Split => "F01_RING_SPLIT",
        }
    }
}

pub const COURT_ROOM_HASH: u64 = 0x4951_683c_b336_eefa;
pub const COURT_ENCOUNTER_HASH: u64 = 0xde73_7bfb_7e64_45a7;
pub const SPLIT_ROOM_HASH: u64 = 0xa634_1e71_16e1_9601;
pub const SPLIT_ENCOUNTER_HASH: u64 = 0x723d_48ff_1bb9_ffe4;
pub const COURT_ROOM_PIN_KEY: &str = "f01_ring_court_room";
pub const COURT_ENCOUNTER_PIN_KEY: &str = "f01_ring_court_encounter";
pub const SPLIT_ROOM_PIN_KEY: &str = "f01_ring_split_room";
pub const SPLIT_ENCOUNTER_PIN_KEY: &str = "f01_ring_split_encounter";

/// A launched Ring room: everything a scene or a fixture needs.
#[derive(Debug, Clone)]
pub struct RingRun {
    pub room: RingRoom,
    pub kit: EnvKit,
    pub fixture: &'static Fixture,
    /// Respawn columns: the start column, then the checkpoint's when it is
    /// right of the start.
    pub rules: &'static DemoRules,
    pub loaded: Loaded,
    /// FA-Q20: the wake gate of each goblin, in enemy index order (empty when
    /// no entry has one).
    pub wakes: Vec<Option<WakeGate>>,
}

impl RingRun {
    /// The closed shutter's rectangle (layout px, y down), if the room has one.
    pub fn shutter(&self) -> Option<crate::camera_lab::room::IRect> {
        self.loaded
            .encounter
            .props
            .iter()
            .find(|p| p.kind == PropKind::Shutter)
            .map(|p| p.target)
    }
}

/// The env runtime of a launched room: the sidecar's props (the court's static
/// shutter) with the roster's wake gates, placed on the room's walking line.
pub fn env_runtime(run: &RingRun) -> Result<EnvRuntime, String> {
    Ok(EnvRuntime::new(&run.loaded.encounter)?
        .with_wake_gates(run.wakes.clone())
        .in_room(run.loaded.room.floor_y, &[], None))
}

/// The roster is goblins only (the existing kind, KR20: RING-11's data swap is
/// later); any other kind is refused, never dropped.
fn goblins_only(roster: &[encounter::RosterEntry]) -> Result<(), String> {
    match roster
        .iter()
        .find(|e| e.kind != RosterKind::Room(EnemyKind::Goblin))
    {
        Some(e) => Err(format!(
            "roster {}: {:?} is not a Ring room kind (goblins only until RING-11)",
            e.id, e.kind
        )),
        None => Ok(()),
    }
}

/// Load a Ring room and its sidecar, compose its kit and build the fixture.
pub fn launch(root: &Path, room: RingRoom) -> Result<RingRun, String> {
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
             sidecar {:#018x} vs {:#018x}): re-pin src/f01_ring.rs on purpose",
            loaded.room_hash,
            room.room_hash(),
            loaded.sidecar_hash,
            room.encounter_hash()
        ));
    }
    if !crate::story_unlock::StoryTable::active().has_room_pin(&loaded.encounter.story_pin) {
        return Err(format!(
            "{path}: unknown story pin {:?}",
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
    goblins_only(&loaded.encounter.roster)?;
    let slot_room = SlotRoom {
        floor_y: loaded.room.floor_y as f32,
        walls: (0.0, kit.width as f32),
    };
    let spawn =
        spawn_from_roster(&loaded.encounter, slot_room, false).map_err(|e| e.to_string())?;
    if !spawn.slot.is_empty() {
        return Err(format!("{path}: a Ring room holds no slot kind"));
    }
    let enemies = crate::f01_boiler::existing_enemies(&loaded.encounter, &spawn.existing, false)?;
    let wakes = enemies.wakes();
    // One small leak per launch (a process launches a room once): the combat
    // state borrows its rules for `'static`.
    let rules: &'static DemoRules = Box::leak(Box::new(DemoRules {
        checkpoints: Box::leak(columns.into_boxed_slice()),
    }));
    let fixture: &'static Fixture = Box::leak(Box::new(Fixture {
        name: room.scene_name(),
        summary: match room {
            RingRoom::Court => {
                "RING-A/B wire: the Shutter Court (static closed shutter, the walkway, two goblins)"
            }
            RingRoom::Split => {
                "RING-A/B wire: the split passage (two routes that rejoin, the view ledge, two goblins)"
            }
        },
        seed: FIXTURE_SEED,
        carl_x: start,
        enemies: Box::leak(enemies.goblins.into_boxed_slice()),
        shore_dx: None,
        boss: None,
        llamas: &[],
        engineers: &[],
        charges: &[],
        script: WALK_RIGHT,
        ticks: SMOKE_TICKS,
        room: crate::combat_scene::FixtureRoom::Corridor,
    }));
    Ok(RingRun {
        room,
        kit,
        fixture,
        rules,
        loaded,
        wakes,
    })
}

/// The combat state of a launched room under a run's story flags and
/// `--cf02-seed`: the CF02 rules, the sidecar's story pin under the program
/// flags (the post-Hoarder state, KR12), the env step with the static shutter
/// and the goblins' wake gates. The binary's scene and the PLAYTRACE verifier
/// both call it through [`crate::f01_late::ring_start`].
pub fn combat_for(
    run: &RingRun,
    scene: &mut SceneSim,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
    cf02_seed: Option<u64>,
) -> Result<CombatState, String> {
    let setup = crate::f01_boiler::story_setup(&run.loaded.encounter.story_pin, carl, donut)?;
    let mut combat = CombatState::new(run.fixture, &run.kit, scene)?
        .with_donut(DonutSetup::unlocked(8))
        .try_with_story(setup)
        .map_err(|e| e.to_string())?
        .with_demo(run.rules)
        .with_cf02()
        .with_cf02_seed(cf02_seed)
        .with_env(env_runtime(run)?);
    combat.sync_env(scene);
    Ok(combat)
}

/// [`combat_for`] under no story flag and no seed (the fixtures' state).
pub fn combat_of(run: &RingRun, scene: &mut SceneSim) -> Result<CombatState, String> {
    combat_for(
        run,
        scene,
        CarlProgFlags::default(),
        DonutProgFlags::default(),
        None,
    )
}

/// The console line of a launch.
pub fn entry_line(run: &RingRun) -> String {
    let e = &run.loaded.encounter;
    let room = &run.loaded.room;
    format!(
        "{}_ENTRY room={} encounter={} pin={} spawn_carl={} spawn_donut={} \
         checkpoints={:?} goal={:?} roster={} goblins={} shutter={} env=live \
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
        run.fixture.enemies.len(),
        run.shutter().map_or("none", |_| "static_closed"),
        run.loaded.room_hash,
        run.loaded.sidecar_hash,
    )
}

/// The pins `playtrace::live::data_pins` stamps (both rooms; each is scoped to
/// its own room by [`crate::f01_late::pin_applies`]).
pub const fn data_pins() -> [(&'static str, u64); 4] {
    [
        (COURT_ROOM_PIN_KEY, COURT_ROOM_HASH),
        (COURT_ENCOUNTER_PIN_KEY, COURT_ENCOUNTER_HASH),
        (SPLIT_ROOM_PIN_KEY, SPLIT_ROOM_HASH),
        (SPLIT_ENCOUNTER_PIN_KEY, SPLIT_ENCOUNTER_HASH),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_scene::EnvSolidKind;
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
        for room in RingRoom::ALL {
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
            assert_eq!(RingRoom::from_id(room.id()), Some(room));
        }
        for (key, value) in data_pins() {
            assert_ne!(value, 0, "{key}");
        }
        assert_eq!(RingRoom::from_id("ring"), None);
    }

    /// An edited sidecar is refused until it is re-pinned on purpose.
    #[test]
    fn an_unpinned_edit_is_refused() {
        let dir = std::env::temp_dir().join(format!("ringw-edit-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("assets/levels/floor01")).unwrap();
        std::fs::create_dir_all(dir.join("assets/data/gameplay")).unwrap();
        let text = std::fs::read_to_string(root().join(SPLIT_ENCOUNTER_PATH)).unwrap();
        let edited = text.replace("\"x\": 560", "\"x\": 564");
        assert_ne!(edited, text);
        std::fs::write(dir.join(SPLIT_ENCOUNTER_PATH), edited).unwrap();
        for rel in [SPLIT_ROOM_PATH, crate::encounter::HEIGHT_CLASSES_PATH] {
            std::fs::copy(root().join(rel), dir.join(rel)).unwrap();
        }
        let err = launch(&dir, RingRoom::Split).expect_err("an unpinned edit");
        assert!(err.contains("re-pin src/f01_ring.rs"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn each_room_builds_its_goblins_shutter_and_pin() {
        for room in RingRoom::ALL {
            let run = launch(&root(), room).unwrap();
            assert_eq!(run.fixture.name, room.scene_name());
            assert_eq!(run.fixture.enemies.len(), 2, "{room:?}: two goblins");
            assert_eq!(run.loaded.encounter.story_pin, "f01_pin_post_pile_boss");
            // FA-Q20: the entries that name a `trigger_x` wake by it.
            let gated: Vec<bool> = run
                .loaded
                .encounter
                .roster
                .iter()
                .map(|e| e.trigger_x.is_some())
                .collect();
            assert_eq!(
                run.wakes.iter().map(Option::is_some).collect::<Vec<_>>(),
                gated
            );
            let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
            let combat = combat_of(&run, &mut scene).unwrap();
            let env = combat.env().expect("the env step");
            let stacks = env
                .solids()
                .iter()
                .filter(|s| s.kind == EnvSolidKind::ShutterStack)
                .count();
            match room {
                RingRoom::Court => {
                    assert_eq!(stacks, 1, "the static stack is one block");
                    assert!(run.shutter().is_some());
                    // It is in the world, standing, as Timber (KR6).
                    let range = env.solid_range();
                    assert_eq!(range.len(), 1);
                    for i in range {
                        let block = &scene.world.solids[i];
                        assert!(block.intact);
                        assert_eq!(block.material, crate::prototype::Material::Timber);
                    }
                }
                RingRoom::Split => {
                    assert_eq!(stacks, 0);
                    assert!(run.shutter().is_none());
                }
            }
            assert!(entry_line(&run).contains(&format!("{}_ENTRY", room.marker())));
        }
    }

    #[test]
    fn a_room_replays_to_the_same_hash_twice_and_the_seed_matters() {
        for room in RingRoom::ALL {
            let run_once = |seed: Option<u64>| {
                let run = launch(&root(), room).unwrap();
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
            assert_eq!(run_once(None), run_once(None), "{room:?}");
            assert_eq!(run_once(Some(3)), run_once(Some(3)), "{room:?}");
            // The seed is the goblins' temperament draw: part of the state.
            assert_ne!(run_once(None), run_once(Some(3)), "{room:?}");
        }
    }

    /// KR12: the room holds the post-Hoarder pin (dynamite among it) under the
    /// program flags and nothing without them.
    #[test]
    fn the_story_flags_apply_the_post_hoarder_pin() {
        use crate::story_unlock::SPECIAL_THROWN_CHARGE;
        let run = launch(&root(), RingRoom::Court).unwrap();
        let holds = |carl: CarlProgFlags, donut: DonutProgFlags| {
            let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
            combat_for(&run, &mut scene, carl, donut, None)
                .unwrap()
                .story_holds(SPECIAL_THROWN_CHARGE)
        };
        let (carl, donut) = crate::f01_late::Loadout::Story.flags();
        assert!(holds(carl, donut));
        assert!(!holds(CarlProgFlags::default(), DonutProgFlags::default()));
    }

    #[test]
    fn a_non_goblin_roster_kind_is_refused_not_dropped() {
        let run = launch(&root(), RingRoom::Split).unwrap();
        let mut roster = run.loaded.encounter.roster.clone();
        assert!(goblins_only(&roster).is_ok());
        roster[1].kind = RosterKind::Room(EnemyKind::Llama);
        let err = goblins_only(&roster).unwrap_err();
        assert!(
            err.contains("goblins only") && err.contains(&roster[1].id),
            "{err}"
        );
    }
}
