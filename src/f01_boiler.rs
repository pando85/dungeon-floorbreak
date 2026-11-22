//! BOIL-A wire: the Goblin Boiler room as a scene (`--scene f01_boiler`).
//!
//! Bevy-free. [`launch`] loads the room and its encounter sidecar through the
//! one room parser ([`crate::encounter::load`], which runs the whole
//! validation), composes the kit the scene runs on (the camera-lab composer, plus the
//! DP-Q45 / DM-Q27 lane ceiling slab that the room declares), and builds the
//! combat fixture and the checkpoint rules from the sidecar: the spawn, the
//! checkpoint and the protected exit.
//!
//! **What runs.** BOIL-B wire and BOIL-C wire make the sidecar's **props and
//! hazards live** ([`env_runtime`], the env step of `combat_scene::env`): the
//! valve and its steam, the hanging weight with its remnant and the drum's
//! solid. The **roster** is [`Roster::Inert`] under `--scene f01_boiler` (the
//! greybox: no enemies, the scripted pilots of `boiler_fixtures` bring their
//! own) and [`Roster::Live`] under `--scene f01_late --f01-room boiler` (BOIL-E,
//! K16): the sidecar's goblin, Engineer and llama entries become the fixture's
//! enemies. Nothing here grants a capability: the sidecar's story
//! pin only says what the room *holds* (K1, K9: the post-Hoarder state), and the
//! hold applies only under the matching program flag (`--carl-prog`,
//! `--donut-prog`), like every pin.
//!
//! The scene is recorded by PLAYTRACE since PT-SCENE: the replay source is
//! `crate::f01_late::boiler_combat`, the one constructor the binary's room and
//! the verifier share. The content is pinned ([`ROOM_PIN_KEY`],
//! [`ENCOUNTER_PIN_KEY`] in `playtrace::live::data_pins`), so an edit of the
//! room or the sidecar fails a replay that names it.

use std::path::Path;

use crate::carl_prog::CarlProgFlags;
use serde_json::Value;

use crate::camera_lab::layout::{BASE_LAYOUT, compose, merged_contract};
use crate::camera_lab::room::EnemyKind;
use crate::camera_lab::room::{Room, TILE};
use crate::combat::EnemyAi;
use crate::combat_scene::{DemoRules, EnemySpawn, Fixture, Hold, LlamaSpawn};
use crate::donut_prog::DonutProgFlags;
use crate::encounter::{self, BOILER_ENCOUNTER_PATH, Encounter, Loaded, RosterKind};
use crate::env_kit::{EnvKit, IRect, Layout, ViewDef};
use crate::input::RawInput;
use crate::scene_enemy::{SlotRoom, spawn_from_roster};
use crate::story_unlock::{SceneKind, StorySetup};

/// The `--scene` name.
pub const SCENE_NAME: &str = "f01_boiler";
/// The id of the room's scenario in its file.
pub const ROOM_ID: &str = "F01-BOILER";

/// FNV-1a 64 of the room file. The files live under `assets/levels`, which a
/// compiled-in `include_str!` may not reach (`playtrace::build_info`), so the
/// hash is a constant: [`launch`] refuses a room whose file differs from it, a
/// test pins it to the file, and every replay that names the pin compares it.
/// Editing the room or the sidecar means re-pinning here, on purpose.
pub const ROOM_HASH: u64 = 0x2b05_73f3_e314_bfd6;
/// FNV-1a 64 of the sidecar (see [`ROOM_HASH`]).
pub const ENCOUNTER_HASH: u64 = 0xf47a_f435_0701_aca7;
/// The `data_pins()` keys.
pub const ROOM_PIN_KEY: &str = "f01_boiler_room";
pub const ENCOUNTER_PIN_KEY: &str = "f01_boiler_encounter";

/// The post-Hoarder story pin the sidecar names (IP-neutral, like
/// `pile_boss_f01_defeated`).
pub const PIN_POST_PILE_BOSS: &str = "f01_pin_post_pile_boss";

/// The script of the headless smoke: hold right for this many ticks, which
/// carries Carl from the spawn to the exit with room to spare, with one jump
/// over the drum's solid (a 32 px solid on the lane, the validated geometry:
/// every class clears it). A live run ignores it.
pub const SMOKE_TICKS: u64 = 600;
/// The tick the smoke's jump over the drum is pressed: Carl is at x 818, 30 px
/// before the drum's solid at 848 (the drum blocks the lane at 836).
pub const SMOKE_JUMP_TICK: u64 = 229;
const WALK_RIGHT: &[Hold] = &[
    Hold {
        from: 0,
        to: SMOKE_TICKS,
        input: RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        },
    },
    Hold {
        from: SMOKE_JUMP_TICK,
        to: SMOKE_JUMP_TICK + 2,
        input: RawInput {
            jump: true,
            ..RawInput::NONE
        },
    },
];
/// A fixture seed is only recorded (no rule reads it in this room).
const FIXTURE_SEED: u64 = 0x0F01_B011;

/// BOIL-E (K16): how the room's sidecar roster loads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Roster {
    /// No enemies: the BOIL-A/B/C greybox (`--scene f01_boiler`), the room the
    /// scripted pilots and the story-ledger fixtures walk.
    Inert,
    /// The sidecar's goblin, Engineer and llama entries are the fixture's enemies
    /// (`--scene f01_late --f01-room boiler`, the story run).
    Live,
}

impl Roster {
    /// The roster a recorded or launched run uses: the story launcher
    /// (`--scene f01_late`) writes an `f01_loadout` label into its descriptor,
    /// the plain `--scene f01_boiler` does not. The one rule the binary and the
    /// PLAYTRACE verifier share, so a recording rebuilds the world it was made in.
    pub fn for_loadout(label: Option<&str>) -> Self {
        if label.is_some() {
            Self::Live
        } else {
            Self::Inert
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Inert => "inert",
            Self::Live => "live",
        }
    }
}

/// The data gate the sidecar's Engineer entry names (K16): the Engineer is built
/// only while his data kill switch (ENG-G-Q25's `engq25_enabled`) is on, and
/// stands in as the entry's fallback kind otherwise.
pub const ENGINEER_GATE: &str = "cf02_engineer";

/// A launched Boiler room: everything the scene arm needs.
#[derive(Debug, Clone)]
pub struct BoilerRun {
    /// How the roster was loaded.
    pub roster: Roster,
    pub kit: EnvKit,
    pub fixture: &'static Fixture,
    /// Respawn columns: the start column, then the checkpoint's.
    pub rules: &'static DemoRules,
    pub loaded: Loaded,
}

/// The story setup of a run of this room: the debug start flags, then the
/// rows of the pin the sidecar names, each under its own program flag
/// (`--carl-prog`, `--donut-prog`). The pin is a row of the story table
/// (`story_unlocks_v0.json`, PT-SCENE), the same table the other room pins and
/// the KR12 order test read. With no flag it is [`StorySetup::NONE`], so a run
/// without them is unchanged (the flag gating of every program).
pub fn story_setup(
    pin: &str,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
) -> Result<StorySetup, String> {
    if !crate::story_unlock::StoryTable::active().has_room_pin(pin) {
        return Err(format!("unknown or non-room story pin {pin:?}"));
    }
    // The room runs on the CF02 scene kind (it arms no trigger in the table),
    // and its pin is chosen by id.
    Ok(crate::donut_prog::story_setup_pin(
        carl,
        donut,
        SceneKind::CfArena,
        pin,
    ))
}

/// Compose the room's kit the way the CF02 microarena does (CF02-Q45): **no
/// authored access steps** (the composer's staircases would wall the lane, and
/// the validator's route graph does not trust them), each perch cut to its deck
/// tile (a 16 px slab, top 80 px up, the deck tile's solid 12 px: Carl walks
/// under it), the scene's two scroll-camera views and the room's exit marker.
pub(crate) fn compose_kit(root: &Path, room: &Room) -> Result<EnvKit, String> {
    let mut room = room.clone();
    room.steps.clear();
    let base_path = root.join(BASE_LAYOUT);
    let base: Value = serde_json::from_str(
        &std::fs::read_to_string(&base_path)
            .map_err(|e| format!("{}: {e}", base_path.display()))?,
    )
    .map_err(|e| e.to_string())?;
    let (mut layout, _plan) = compose(&room, &base);
    for p in &room.platforms {
        let face_row = ((p.rect.y - 4) / TILE + 1) as usize;
        for col in (p.rect.x / TILE)..(p.rect.right() / TILE) {
            layout["tile_layers"]["collision"][face_row][col as usize] = Value::Null;
        }
    }
    let layout: Layout = serde_json::from_value(layout).map_err(|e| e.to_string())?;
    let contract = merged_contract(root).map_err(|e| e.to_string())?;
    let mut kit = EnvKit::build(&contract, &layout).map_err(|e| e.to_string())?;
    // The walking line 288 px below the top of the 640x360 view (308 px below
    // the top of the 640x400 one), like the corridor and the DEMO hall.
    kit.views.clear();
    for (name, h, below) in [("abc_640x360", 360, 288), ("abc_640x400", 400, 308)] {
        kit.views.insert(
            name.to_owned(),
            ViewDef {
                x: 0,
                y: kit.base_walk_line_y - below,
                w: 640,
                h,
            },
        );
    }
    Ok(kit)
}

/// The env runtime of a launched room: the sidecar's props, hazards and
/// solids (BOIL-B wire, BOIL-C wire). `with_env` puts it on the combat state.
pub fn env_runtime(run: &BoilerRun) -> Result<crate::combat_scene::EnvRuntime, String> {
    crate::combat_scene::EnvRuntime::new(&run.loaded.encounter)
}

/// The enemies a sidecar roster puts in the fixture (BOIL-E, K16), in roster
/// order within each list: the goblins (with an Engineer's fallback when his
/// data gate is off), the Engineers and the llamas.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RosterEnemies {
    pub goblins: Vec<EnemySpawn>,
    pub engineers: Vec<EnemySpawn>,
    pub llamas: Vec<LlamaSpawn>,
}

/// Build the fixture's enemies from the sidecar roster. The slot spawn
/// ([`spawn_from_roster`]) returns the roster indices of the **existing kinds**
/// (goblin, Engineer, llama); a slot kind in the Boiler is an error (the room
/// has none). The Engineer entry is the K16 relaxation of ENG-G-Q25 for a
/// sidecar roster: it needs the data kill switch (`engineer_enabled`), and
/// without it the entry's `fallback_kind` stands in. `trigger_x` and
/// `delay_ticks` are read by slot kinds only: an existing kind stands at its
/// column from tick 0 and wakes by sight (BOILE-AD2).
pub fn roster_enemies(
    encounter: &Encounter,
    room: SlotRoom,
    engineer_enabled: bool,
) -> Result<RosterEnemies, String> {
    let spawn = spawn_from_roster(encounter, room, false).map_err(|e| e.to_string())?;
    if !spawn.slot.is_empty() {
        return Err("the Boiler's roster holds a slot kind (the room has none)".to_owned());
    }
    let mut out = RosterEnemies::default();
    for index in spawn.existing {
        let entry = &encounter.roster[index];
        if entry.perch.is_some() {
            return Err(format!(
                "roster {}: an existing kind stands on the walking line, not a perch",
                entry.id
            ));
        }
        let RosterKind::Room(kind) = entry.kind else {
            return Err(format!("roster {}: not a room enemy kind", entry.id));
        };
        let at = EnemySpawn {
            x: entry.x,
            ai: EnemyAi::Chase,
        };
        match kind {
            EnemyKind::Goblin => out.goblins.push(at),
            EnemyKind::Llama => out.llamas.push(LlamaSpawn {
                x: entry.x,
                ai: EnemyAi::Chase,
            }),
            EnemyKind::Engineer => {
                match entry.gate.as_deref() {
                    None | Some(ENGINEER_GATE) => {}
                    Some(other) => {
                        return Err(format!("roster {}: unknown gate {other:?}", entry.id));
                    }
                }
                if engineer_enabled {
                    out.engineers.push(at);
                } else if entry.fallback == Some(EnemyKind::Goblin) {
                    out.goblins.push(at);
                } else {
                    return Err(format!(
                        "roster {}: the Engineer is off and the entry has no goblin fallback",
                        entry.id
                    ));
                }
            }
            other => {
                return Err(format!(
                    "roster {}: {other:?} is not an existing kind",
                    entry.id
                ));
            }
        }
    }
    Ok(out)
}

/// Load the room with its **inert** roster (the greybox): see [`launch_with`].
pub fn launch(root: &Path) -> Result<BoilerRun, String> {
    launch_with(root, Roster::Inert)
}

/// Load the room, compose its kit and build the fixture and rules. With
/// [`Roster::Live`] the fixture carries the sidecar's enemies (BOIL-E).
pub fn launch_with(root: &Path, roster: Roster) -> Result<BoilerRun, String> {
    let loaded = encounter::load(root, BOILER_ENCOUNTER_PATH).map_err(|errors| {
        format!(
            "{BOILER_ENCOUNTER_PATH}: {}",
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
             sidecar {:#018x} vs {ENCOUNTER_HASH:#018x}): re-pin src/f01_boiler.rs on purpose",
            loaded.room_hash, loaded.sidecar_hash
        ));
    }
    if !crate::story_unlock::StoryTable::active().has_room_pin(&loaded.encounter.story_pin) {
        return Err(format!(
            "{BOILER_ENCOUNTER_PATH}: unknown story pin {:?}",
            loaded.encounter.story_pin
        ));
    }
    let mut kit = compose_kit(root, &loaded.room)?;
    if kit.base_walk_line_y != loaded.room.floor_y {
        return Err(format!(
            "the kit's walking line {} is not the room's floor {}",
            kit.base_walk_line_y, loaded.room.floor_y
        ));
    }
    // DP-Q45 / DM-Q27: the room's lane ceiling, the boss room's pattern: a
    // collision-only slab whose underside is `ceiling_px` above the walking
    // line, so boosted Donut's jump from a perch stays under the HUD band.
    if let Some(ceiling) = loaded.ceiling_px {
        kit.solid_rects.push(IRect {
            x: 0,
            y: kit.base_walk_line_y - ceiling - 16,
            w: kit.width,
            h: 16,
        });
    }
    // DP-Q35: the room's optional blink barriers (search only, never collision).
    kit.blink_barriers = loaded.room.kit_blink_barriers();
    // The checkpoint's column is the middle of its rectangle; the start column
    // is Carl's spawn (a checkpoint at or left of it adds none).
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
    // One small leak per launch (a process launches the room once): the combat
    // state borrows its rules for `'static`.
    let rules: &'static DemoRules = Box::leak(Box::new(DemoRules {
        checkpoints: Box::leak(columns.into_boxed_slice()),
    }));
    let enemies = match roster {
        Roster::Inert => RosterEnemies::default(),
        Roster::Live => roster_enemies(
            &loaded.encounter,
            SlotRoom {
                floor_y: loaded.room.floor_y as f32,
                walls: (0.0, kit.width as f32),
            },
            crate::engineer::active().enabled(),
        )?,
    };
    let fixture: &'static Fixture = Box::leak(Box::new(Fixture {
        name: SCENE_NAME,
        summary: match roster {
            Roster::Inert => {
                "BOIL-A/B/C wire: the Goblin Boiler hall with live props (valve, steam, weight, drum solid)"
            }
            Roster::Live => {
                "BOIL-E: the Goblin Boiler with live props and the sidecar's goblins, Engineer and llamas"
            }
        },
        seed: FIXTURE_SEED,
        carl_x: start,
        enemies: Box::leak(enemies.goblins.into_boxed_slice()),
        shore_dx: None,
        boss: None,
        llamas: Box::leak(enemies.llamas.into_boxed_slice()),
        engineers: Box::leak(enemies.engineers.into_boxed_slice()),
        charges: &[],
        script: WALK_RIGHT,
        ticks: SMOKE_TICKS,
    }));
    Ok(BoilerRun {
        roster,
        kit,
        fixture,
        rules,
        loaded,
    })
}

/// The console line of a launch: identity, pin and hashes, and the fact that
/// the props and hazards are live, and whether the roster is inert or live.
pub fn entry_line(run: &BoilerRun) -> String {
    let e = &run.loaded.encounter;
    let room = &run.loaded.room;
    format!(
        "F01_BOILER_ENTRY room={} encounter={} pin={} spawn_carl={} spawn_donut={} \
         checkpoints={:?} exit={},{},{},{} ceiling_px={} props={} hazards={} roster={} env=live roster_inert={} roster_mode={} \
         room_hash={:#018x} sidecar_hash={:#018x}",
        e.room_id,
        e.id,
        e.story_pin,
        room.spawn_carl,
        room.spawn_donut,
        run.rules.checkpoints,
        e.exit.x,
        e.exit.y,
        e.exit.w,
        e.exit.h,
        run.loaded.ceiling_px.unwrap_or(0),
        e.props.len(),
        e.hazards.len(),
        e.roster.len(),
        run.roster == Roster::Inert,
        run.roster.name(),
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
    use crate::combat_scene::{CombatState, FixtureSim};
    use crate::corridor::SceneSim;
    use crate::donut_mana::DonutSetup;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn run() -> BoilerRun {
        launch(&root()).expect("the Boiler room launches")
    }

    #[test]
    fn the_pinned_hashes_are_the_files() {
        let hash = |path: &str| {
            crate::donut_mana::content_hash(
                &std::fs::read_to_string(root().join(path)).expect("the file reads"),
            )
        };
        assert_eq!(
            hash(crate::encounter::BOILER_ROOM_PATH),
            ROOM_HASH,
            "{:#018x}",
            hash(crate::encounter::BOILER_ROOM_PATH)
        );
        assert_eq!(
            hash(crate::encounter::BOILER_ENCOUNTER_PATH),
            ENCOUNTER_HASH,
            "{:#018x}",
            hash(crate::encounter::BOILER_ENCOUNTER_PATH)
        );
        let r = run();
        assert_eq!(
            r.loaded.room_hash, ROOM_HASH,
            "the loader read the pinned room"
        );
        assert_eq!(r.loaded.sidecar_hash, ENCOUNTER_HASH);
        for (key, hash) in data_pins() {
            assert_ne!(hash, 0, "{key}");
        }
    }

    #[test]
    fn the_room_loads_with_its_spawn_checkpoint_exit_and_ceiling() {
        let r = run();
        let room = &r.loaded.room;
        let e = &r.loaded.encounter;
        assert_eq!(r.fixture.name, SCENE_NAME);
        assert_eq!(r.fixture.carl_x, room.spawn_carl);
        assert_eq!(r.rules.checkpoints, &[112, 200]);
        assert_eq!(e.story_pin, PIN_POST_PILE_BOSS);
        // The kit carries the room's exit and the lane ceiling slab.
        let exit = r.kit.exit.expect("the protected exit");
        assert_eq!(
            (exit.x, exit.y, exit.w, exit.h),
            (e.exit.x, e.exit.y, e.exit.w, e.exit.h)
        );
        let slab = r
            .kit
            .solid_rects
            .iter()
            .find(|s| s.w == r.kit.width && s.h == 16 && s.y < r.kit.base_walk_line_y - 100)
            .expect("DP-Q45: the lane ceiling slab");
        assert_eq!(
            r.kit.base_walk_line_y - (slab.y + slab.h),
            r.loaded.ceiling_px.unwrap(),
            "its underside is ceiling_px above the walking line"
        );
        // Both spawns stand on the walking line.
        for x in [room.spawn_carl, room.spawn_donut] {
            assert_eq!(r.kit.walking_line_at(x), Some(r.kit.base_walk_line_y));
        }
        // The two perches are in the kit as solids at the room's heights.
        for p in &room.platforms {
            assert!(
                r.kit
                    .solid_rects
                    .iter()
                    .any(|s| s.x <= p.rect.x && s.right() >= p.rect.right() && s.y == p.rect.y),
                "platform {:?} is a kit solid",
                p.rect
            );
        }
    }

    #[test]
    fn the_roster_stays_inert_and_the_props_build() {
        let r = run();
        let sim = FixtureSim::new(&r.kit, 360, r.fixture).expect("the fixture builds");
        let env = env_runtime(&r).expect("the sidecar's props build");
        assert_eq!(env.props(), 3, "valve, weight, drum");
        assert_eq!(env.solids().len(), 2, "the remnant and the drum");
        assert!(sim.combat.enemies.is_empty(), "no goblins yet (BOIL-E)");
        assert!(sim.combat.llamas.is_empty());
        assert!(r.fixture.enemies.is_empty() && r.fixture.charges.is_empty());
        // The sidecar's entities are validated but not put in the world.
        assert_eq!(r.loaded.encounter.props.len(), 3);
        assert_eq!(r.loaded.encounter.hazards.len(), 3);
        assert!(!r.loaded.encounter.roster.is_empty());
    }

    // ---- BOIL-E: the live roster (K16)

    fn live() -> BoilerRun {
        launch_with(&root(), Roster::Live).expect("the live Boiler launches")
    }

    fn slot_room(r: &BoilerRun) -> SlotRoom {
        SlotRoom {
            floor_y: r.loaded.room.floor_y as f32,
            walls: (0.0, r.kit.width as f32),
        }
    }

    #[test]
    fn the_loadout_label_picks_the_roster() {
        assert_eq!(Roster::for_loadout(None), Roster::Inert);
        assert_eq!(Roster::for_loadout(Some("story")), Roster::Live);
        assert_eq!(Roster::for_loadout(Some("all")), Roster::Live);
        assert_eq!(run().roster, Roster::Inert, "launch() is the greybox");
    }

    /// BOILE-AD1: the sidecar's roster is the fixture's enemies, in roster order,
    /// the goblins first and the Engineer after them (his target id follows).
    #[test]
    fn a_live_roster_builds_the_sidecar_goblins_and_the_engineer_as_b3() {
        let r = live();
        let xs = |s: &[EnemySpawn]| s.iter().map(|e| e.x).collect::<Vec<_>>();
        assert_eq!(xs(r.fixture.enemies), [344, 640, 944], "g1, g2, g3");
        assert_eq!(xs(r.fixture.engineers), [1000], "e1 is the B3 Engineer");
        assert!(r.fixture.llamas.is_empty());
        assert!(
            r.fixture
                .enemies
                .iter()
                .chain(r.fixture.engineers)
                .all(|e| e.ai == EnemyAi::Chase)
        );
        // Under the CF02 rules (the room runs them) four enemies stand at their
        // columns and exactly the last one is the Engineer.
        let scene = SceneSim::new_at(&r.kit, 360, r.fixture.carl_x).unwrap();
        let combat = CombatState::new(r.fixture, &r.kit, &scene)
            .unwrap()
            .with_cf02();
        assert_eq!(combat.enemies.len(), 4);
        let engineers: Vec<usize> = (0..4)
            .filter(|i| combat.enemies[*i].is_engineer())
            .collect();
        assert_eq!(engineers, [3], "the Engineer follows the goblins");
        assert!(combat.engineer_on());
        // The wire state is the one the PLAYTRACE verifier rebuilds.
        assert!(r.fixture.ticks == SMOKE_TICKS && r.fixture.charges.is_empty());
    }

    /// The staging: nobody is awake at the spawn (the nearest enemy is beyond the
    /// goblin sight of 220 px from both spawns), so the room opens on a readable
    /// vista (B0) and the enemies wake in order as Carl advances (BOILE-AD2).
    #[test]
    fn the_enemies_are_staged_beyond_sight_of_the_spawns_and_in_order() {
        let r = live();
        let sight = crate::feel::GOBLIN_SIGHT as i32;
        let columns: Vec<i32> = r
            .fixture
            .enemies
            .iter()
            .chain(r.fixture.engineers)
            .map(|e| e.x)
            .collect();
        let near = columns.iter().copied().min().unwrap();
        for spawn in [r.loaded.room.spawn_carl, r.loaded.room.spawn_donut] {
            assert!(near - spawn >= sight, "{near} is inside sight of {spawn}");
        }
        let mut sorted = columns.clone();
        sorted.sort_unstable();
        // The wake columns (the enemy's column minus the sight) are strictly
        // increasing in roster order: one enters sight at a time.
        let wake: Vec<i32> = r
            .loaded
            .encounter
            .roster
            .iter()
            .map(|e| e.x - sight)
            .collect();
        assert!(wake.windows(2).all(|w| w[0] < w[1]), "{wake:?}");
        // No more than `max_living` enemies at once (the design's initial probe).
        assert!(columns.len() <= usize::from(r.loaded.encounter.max_living));
    }

    /// BOILE-AD2 in a simulation, not only in columns: with Carl idle at his spawn
    /// nobody wakes, so every enemy holds its column for 300 ticks.
    #[test]
    fn nobody_wakes_while_the_heroes_idle_at_the_spawn() {
        let r = live();
        let mut scene = SceneSim::new_at(&r.kit, 360, r.fixture.carl_x).unwrap();
        let mut combat = CombatState::new(r.fixture, &r.kit, &scene)
            .unwrap()
            .with_cf02();
        let at: Vec<f32> = combat.enemies.iter().map(|g| g.x).collect();
        for _ in 0..300 {
            combat.step_party(&mut scene, [RawInput::default(), RawInput::default()]);
        }
        let after: Vec<f32> = combat.enemies.iter().map(|g| g.x).collect();
        assert_eq!(at, after);
    }

    /// ENG-G-Q25 kill switch: with the Engineer off, his entry stands in as its
    /// goblin fallback and no Engineer exists.
    #[test]
    fn the_engineer_entry_falls_back_to_a_goblin_when_his_data_gate_is_off() {
        let r = run();
        let room = slot_room(&r);
        let on = roster_enemies(&r.loaded.encounter, room, true).unwrap();
        assert_eq!((on.goblins.len(), on.engineers.len()), (3, 1));
        let off = roster_enemies(&r.loaded.encounter, room, false).unwrap();
        assert_eq!((off.goblins.len(), off.engineers.len()), (4, 0));
        assert_eq!(
            off.goblins.last().map(|g| g.x),
            Some(1000),
            "e1 as a goblin"
        );
    }

    #[test]
    fn a_bad_roster_entry_is_an_error_never_silently_dropped() {
        let r = run();
        let room = slot_room(&r);
        let engineer = |e: &Encounter| {
            e.roster
                .iter()
                .position(|x| x.id == "e1")
                .expect("the sidecar's Engineer")
        };
        let mut unknown_gate = r.loaded.encounter.clone();
        let i = engineer(&unknown_gate);
        unknown_gate.roster[i].gate = Some("nope".to_owned());
        let err = roster_enemies(&unknown_gate, room, true).unwrap_err();
        assert!(err.contains("unknown gate"), "{err}");
        let mut no_fallback = r.loaded.encounter.clone();
        no_fallback.roster[i].fallback = None;
        let err = roster_enemies(&no_fallback, room, false).unwrap_err();
        assert!(err.contains("no goblin fallback"), "{err}");
        assert!(roster_enemies(&no_fallback, room, true).is_ok(), "gate on");
        let mut perched = r.loaded.encounter.clone();
        perched.roster[0].perch = Some(0);
        let err = roster_enemies(&perched, room, true).unwrap_err();
        assert!(err.contains("perch"), "{err}");
        let mut llama = r.loaded.encounter.clone();
        llama.roster[0].kind = RosterKind::Room(EnemyKind::Llama);
        let got = roster_enemies(&llama, room, true).unwrap();
        assert_eq!(got.llamas.len(), 1, "a llama entry builds a llama");
        let mut slime = r.loaded.encounter.clone();
        slime.roster[0].kind = RosterKind::Slime;
        assert!(roster_enemies(&slime, room, true).is_err(), "a slot kind");
    }

    /// K16: the relaxation is the sidecar's alone. The same live fixture on a run
    /// without the CF02 rules builds no Engineer (`engineer_on` needs them), and
    /// the inert greybox has no enemy at all.
    #[test]
    fn the_engineer_needs_the_cf02_rules_and_the_inert_room_has_none() {
        let r = live();
        let scene = SceneSim::new_at(&r.kit, 360, r.fixture.carl_x).unwrap();
        let plain = CombatState::new(r.fixture, &r.kit, &scene).unwrap();
        assert!(!plain.engineer_on());
        assert!(plain.enemies.iter().all(|g| !g.is_engineer()));
        let inert = run();
        let scene = SceneSim::new_at(&inert.kit, 360, inert.fixture.carl_x).unwrap();
        let combat = CombatState::new(inert.fixture, &inert.kit, &scene)
            .unwrap()
            .with_cf02();
        assert!(combat.enemies.is_empty() && combat.llamas.is_empty());
    }

    #[test]
    fn a_live_run_is_deterministic_and_differs_from_the_greybox() {
        let hash = |r: &BoilerRun| {
            let mut sim = FixtureSim::new(&r.kit, 360, r.fixture).unwrap();
            sim.combat = sim
                .combat
                .clone()
                .with_demo(r.rules)
                .with_cf02()
                .with_env(env_runtime(r).unwrap());
            sim.combat.sync_env(&mut sim.scene);
            sim.run_scripted(SMOKE_TICKS);
            sim.state_hash()
        };
        assert_eq!(hash(&live()), hash(&live()));
        assert_ne!(hash(&live()), hash(&run()));
    }

    /// The headless smoke in the library: the scripted walk carries Carl from
    /// the spawn through the checkpoint to the protected exit on the real scene
    /// and combat state.
    #[test]
    fn the_scripted_walk_reaches_the_exit_through_the_checkpoint() {
        let r = run();
        let mut scene = SceneSim::new_at(&r.kit, 360, r.fixture.carl_x).unwrap();
        let mut combat = CombatState::new(r.fixture, &r.kit, &scene)
            .unwrap()
            .with_donut(DonutSetup::unlocked(8))
            .with_demo(r.rules)
            .with_cf02()
            .with_env(env_runtime(&r).unwrap());
        combat.sync_env(&mut scene);
        let mut checkpoint_seen = 0;
        let mut reached = None;
        for tick in 1..=SMOKE_TICKS {
            let raw = r.fixture.input(tick);
            combat.step_party(&mut scene, [raw, RawInput::default()]);
            checkpoint_seen = checkpoint_seen.max(combat.checkpoint);
            if scene.world.complete && reached.is_none() {
                reached = Some(tick);
            }
        }
        assert_eq!(checkpoint_seen, 1, "the checkpoint advanced once");
        let tick = reached.expect("the exit is reached inside the smoke window");
        assert!(
            (200..450).contains(&tick),
            "about 950 px at 185 px/s: reached at tick {tick}"
        );
    }

    #[test]
    fn the_smoke_is_deterministic() {
        let r = run();
        let hash = || {
            let mut sim = FixtureSim::new(&r.kit, 360, r.fixture).unwrap();
            sim.combat = sim
                .combat
                .clone()
                .with_demo(r.rules)
                .with_cf02()
                .with_env(env_runtime(&r).unwrap());
            sim.combat.sync_env(&mut sim.scene);
            sim.run_scripted(SMOKE_TICKS);
            sim.state_hash()
        };
        assert_eq!(hash(), hash());
    }

    // ---- the story pin

    #[test]
    fn the_pin_holds_only_under_its_program_flag() {
        let off = story_setup(PIN_POST_PILE_BOSS, CarlProgFlags::OFF, DonutProgFlags::OFF).unwrap();
        assert!(off.is_inert(), "no flag: nothing held");
        let carl = CarlProgFlags {
            prog: true,
            ..CarlProgFlags::OFF
        };
        let donut = DonutProgFlags {
            prog: true,
            ..DonutProgFlags::OFF
        };
        let only_carl = story_setup(PIN_POST_PILE_BOSS, carl, DonutProgFlags::OFF).unwrap();
        assert!(!only_carl.is_inert());
        let only_donut = story_setup(PIN_POST_PILE_BOSS, CarlProgFlags::OFF, donut).unwrap();
        assert!(!only_donut.is_inert());
        let both = story_setup(PIN_POST_PILE_BOSS, carl, donut).unwrap();
        assert!(!both.is_inert());
    }

    #[test]
    fn an_unknown_pin_is_rejected() {
        let err = story_setup("f01_pin_nope", CarlProgFlags::OFF, DonutProgFlags::OFF)
            .expect_err("unknown pin");
        assert!(err.contains("f01_pin_nope"), "{err}");
        assert!(!crate::story_unlock::StoryTable::active().has_room_pin("f01_pin_nope"));
    }

    /// A sidecar names a room pin: the boss arena's pins are refused (they would
    /// give Carl's half only), at the launch check and in `story_setup`.
    #[test]
    fn a_scene_pin_is_not_a_sidecar_pin() {
        for pin in ["f01_dynamite_held", "f01_donut_boss_held"] {
            assert!(crate::story_unlock::StoryTable::active().has_pin(pin));
            let err = story_setup(pin, CarlProgFlags::OFF, DonutProgFlags::OFF)
                .expect_err("a scene pin is not a room pin");
            assert!(err.contains(pin), "{err}");
        }
        for pin in [
            "f01_pin_pre_pile_boss",
            "f01_pin_post_pile_boss",
            "f01_late_mastery",
        ] {
            assert!(
                story_setup(pin, CarlProgFlags::OFF, DonutProgFlags::OFF).is_ok(),
                "{pin}"
            );
        }
    }

    #[test]
    fn the_pin_never_grants_a_trigger_or_arms_one() {
        // The room grants nothing (the validator rejects `grants`), and the pin
        // arms no trigger: a run holds the post-Hoarder state and nothing more.
        let carl = CarlProgFlags {
            prog: true,
            ..CarlProgFlags::OFF
        };
        let donut = DonutProgFlags {
            prog: true,
            ..DonutProgFlags::OFF
        };
        let setup = story_setup(PIN_POST_PILE_BOSS, carl, donut).unwrap();
        let scene = SceneSim::new_at(&run().kit, 360, 112).unwrap();
        let combat = CombatState::new(run().fixture, &run().kit, &scene)
            .unwrap()
            .with_donut(DonutSetup::unlocked(8))
            .try_with_story(setup)
            .expect("the pin is a legal story state (the biscuit is held)");
        assert!(combat.events.is_empty(), "no grant event at start");
    }
}
