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
use crate::goblin::WakeGate;
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
pub const ENCOUNTER_HASH: u64 = 0xd241_b68d_d308_f2ec;
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

/// PR-C of the Floor 1 fix-all plan: the Boiler's knobs
/// (`assets/data/gameplay/f01_boiler_fixes_v0.json`), embedded, validated and
/// FNV-1a pinned like the other tunings (the pin is scoped to the Boiler room,
/// [`crate::f01_late::pin_applies`]).
pub mod fixes {
    use serde::Deserialize;

    use crate::donut_mana::content_hash;

    pub const TUNING_JSON: &str = include_str!("../assets/data/gameplay/f01_boiler_fixes_v0.json");
    /// The committed FNV-1a 64 content hash of [`TUNING_JSON`].
    pub const TUNING_HASH: u64 = 0x71f0_acee_3e65_dace;
    /// The `data_pins()` key.
    pub const PIN_KEY: &str = "f01_boiler_fixes";

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Fixes {
        pub schema_version: u32,
        /// FA-Q20 (amends BOILE-AD2): the live roster's existing kinds honour
        /// `trigger_x` and `delay_ticks`. Off: every enemy wakes by sight alone, the
        /// pre-PR-C behaviour (and its hashes).
        pub fa_q20_wake_gates: bool,
        /// FA-Q20, adopted `false`: an entry with a `delay_ticks` and no `trigger_x`
        /// keeps the plain sight wake (the sidecar's g2 sets `delay_ticks` 120 only to
        /// stagger its spawn for the roster validator). On: that delay also counts after
        /// the sight wake.
        pub fa_q20_delay_after_sight: bool,
        /// FA-Q34 (room 2 retune): the finish reserve radius in px, absent (`None`) =
        /// off. An enemy with at most one hit point within it of the human leader is his
        /// to finish: the AI partner starts no press against it. Boiler-scoped.
        #[serde(default)]
        pub fa_q34_finish_reserve_px: Option<u16>,
        /// FA-Q35 (room 2 retune): the AI partner starts no press against an enemy that is
        /// asleep or counting its wake delay (B0 "read the machine"). Absent = off.
        #[serde(default)]
        pub fa_q35_hold_fire_until_awake: bool,
        /// FA-Q36 (review of #257): the AI partner does not step into the sight of an enemy
        /// that has not been woken, unless the human leader is already inside it or she is
        /// attacked, so the human opens every fight. Absent = off.
        #[serde(default)]
        pub fa_q36_wake_leash: bool,
        /// FA-Q36: how many px outside the sight the leash holds her (0..16).
        #[serde(default = "default_leash_margin_px")]
        pub fa_q36_leash_margin_px: u8,
        /// FA-Q37: a reserved enemy (FA-Q34) is released, for good this attempt, after this
        /// many hits on any hero. Absent = no release by hits.
        #[serde(default)]
        pub fa_q37_release_on_hits: Option<u8>,
        /// FA-Q37: or after being reserved this many ticks in a row. Absent = no release by
        /// time.
        #[serde(default)]
        pub fa_q37_release_after_ticks: Option<u16>,
    }

    fn default_leash_margin_px() -> u8 {
        4
    }

    impl Fixes {
        pub fn from_json(text: &str) -> Result<Self, String> {
            let fixes: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
            if fixes.schema_version != 1 {
                return Err(format!("schema_version {} is not 1", fixes.schema_version));
            }
            Ok(fixes)
        }

        /// The embedded file, which must equal its pin.
        pub fn pinned() -> Result<Self, String> {
            let actual = content_hash(TUNING_JSON);
            if actual != TUNING_HASH {
                return Err(format!(
                    "f01_boiler_fixes_v0.json hash {actual:#018x} is not the pin {TUNING_HASH:#018x}"
                ));
            }
            Self::from_json(TUNING_JSON)
        }
    }

    /// The pinned knobs: what a launch of the room reads.
    impl Fixes {
        /// The partner knobs of these fixes, as the combat state carries them.
        pub fn partner_knobs(&self) -> crate::companion::view::PartnerKnobs {
            crate::companion::view::PartnerKnobs {
                finish_reserve_px: self.fa_q34_finish_reserve_px.map(f32::from),
                hold_fire_until_awake: self.fa_q35_hold_fire_until_awake,
                wake_leash_margin_px: self
                    .fa_q36_wake_leash
                    .then(|| f32::from(self.fa_q36_leash_margin_px)),
                release_on_hits: self.fa_q37_release_on_hits,
                release_after_ticks: self.fa_q37_release_after_ticks,
            }
        }
    }

    pub fn active() -> &'static Fixes {
        static ACTIVE: std::sync::OnceLock<Fixes> = std::sync::OnceLock::new();
        ACTIVE.get_or_init(|| Fixes::pinned().expect("the embedded Boiler fixes are pinned"))
    }
}

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
    /// The roster a launched run uses: the story launcher (`--scene f01_late`)
    /// carries an `f01_loadout` label, the plain `--scene f01_boiler` does not. The
    /// binary writes the choice into the descriptor's explicit `f01_roster` field
    /// (BOIL-FU) and the verifier reads it back with [`Self::from_descriptor`]; this
    /// rule is also its fallback for a recording that predates the field.
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

    /// The roster of a recorded run's descriptor (BOIL-FU): the explicit
    /// `f01_roster` field when it is there (`live` or `inert`; anything else is an
    /// error), else the legacy label rule of [`Self::for_loadout`] for a recording
    /// made before the field existed.
    pub fn from_descriptor(roster: Option<&str>, loadout: Option<&str>) -> Result<Self, String> {
        match roster {
            Some("live") => Ok(Self::Live),
            Some("inert") => Ok(Self::Inert),
            Some(other) => Err(format!(
                "unknown Boiler roster {other:?} (expected live or inert)"
            )),
            None => Ok(Self::for_loadout(loadout)),
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
    /// FA-Q20: the wake gate of each enemy, in enemy index order (empty for the
    /// inert roster and with the knob off); [`env_runtime`] carries them to the combat.
    pub wakes: Vec<Option<WakeGate>>,
    /// FA-Q34, FA-Q35: the partner knobs of the launch ([`crate::f01_late::boiler_combat`]
    /// puts them on the combat state).
    pub partner: crate::companion::view::PartnerKnobs,
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
    Ok(crate::combat_scene::EnvRuntime::new(&run.loaded.encounter)?
        .with_wake_gates(run.wakes.clone())
        .with_partner_knobs(run.partner))
}

/// The enemies a sidecar roster puts in the fixture (BOIL-E, K16), in roster
/// order within each list: the goblins (with an Engineer's fallback when his
/// data gate is off), the Engineers and the llamas.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RosterEnemies {
    pub goblins: Vec<EnemySpawn>,
    pub engineers: Vec<EnemySpawn>,
    pub llamas: Vec<LlamaSpawn>,
    /// FA-Q20: the wake gate of each goblin and each Engineer, parallel to the two
    /// lists above (`None`: wakes by sight alone, the entry sets no `trigger_x`).
    pub goblin_wakes: Vec<Option<WakeGate>>,
    pub engineer_wakes: Vec<Option<WakeGate>>,
}

impl RosterEnemies {
    /// The gates in enemy index order: the goblins, then the Engineers.
    pub fn wakes(&self) -> Vec<Option<WakeGate>> {
        let all: Vec<Option<WakeGate>> = self
            .goblin_wakes
            .iter()
            .chain(&self.engineer_wakes)
            .copied()
            .collect();
        if all.iter().all(Option::is_none) {
            Vec::new()
        } else {
            all
        }
    }
}

/// FA-Q20: the Boiler has no llama entry, and a llama has no wake gate: an
/// entry that names one is refused, never silently ignored.
pub fn refuse_llama_wake(encounter: &Encounter) -> Result<(), String> {
    match encounter.roster.iter().find(|e| {
        e.kind == RosterKind::Room(EnemyKind::Llama) && (e.trigger_x.is_some() || e.delay_ticks > 0)
    }) {
        Some(e) => Err(format!(
            "roster {}: a llama has no wake gate (trigger_x, delay_ticks)",
            e.id
        )),
        None => Ok(()),
    }
}

/// FA-Q20: the wake gate of a roster entry, or `None` when the entry wakes by
/// sight alone: it names no `trigger_x`, and its `delay_ticks` does not count
/// without one unless `fa_q20_delay_after_sight` is on.
pub fn wake_gate(entry: &encounter::RosterEntry, fixes: &fixes::Fixes) -> Option<WakeGate> {
    if !fixes.fa_q20_wake_gates {
        return None;
    }
    let counts =
        entry.trigger_x.is_some() || (fixes.fa_q20_delay_after_sight && entry.delay_ticks > 0);
    counts.then_some(WakeGate {
        trigger_x: entry.trigger_x,
        delay_ticks: entry.delay_ticks,
        delay_after_sight: fixes.fa_q20_delay_after_sight,
    })
}

/// Build the fixture's enemies from the sidecar roster. The slot spawn
/// ([`spawn_from_roster`]) returns the roster indices of the **existing kinds**
/// (goblin, Engineer, llama); a slot kind in the Boiler is an error (the room
/// has none). The Engineer entry is the K16 relaxation of ENG-G-Q25 for a
/// sidecar roster: it needs the data kill switch (`engineer_enabled`), and
/// without it the entry's `fallback_kind` stands in. An entry's `trigger_x` and
/// `delay_ticks` become its [`WakeGate`] (FA-Q20, which amends BOILE-AD2): the
/// enemy stands at its column from tick 0 and wakes at the earlier of sight and
/// any hero's column reaching `trigger_x`, plus the delay; an entry without a
/// `trigger_x` wakes by sight alone.
pub fn roster_enemies(
    encounter: &Encounter,
    room: SlotRoom,
    engineer_enabled: bool,
) -> Result<RosterEnemies, String> {
    roster_enemies_with(encounter, room, engineer_enabled, fixes::active())
}

/// [`roster_enemies`] under explicit knobs (the knob-off proofs).
pub fn roster_enemies_with(
    encounter: &Encounter,
    room: SlotRoom,
    engineer_enabled: bool,
    knobs: &fixes::Fixes,
) -> Result<RosterEnemies, String> {
    let spawn = spawn_from_roster(encounter, room, false).map_err(|e| e.to_string())?;
    if !spawn.slot.is_empty() {
        return Err("the Boiler's roster holds a slot kind (the room has none)".to_owned());
    }
    existing_enemies_with(encounter, &spawn.existing, engineer_enabled, knobs)
}

/// The existing-kind half of [`roster_enemies`] (PT-MASTERY): the enemies of the
/// roster entries at `indices` (the `existing` list of a [`spawn_from_roster`]
/// result), for a room whose roster also holds slot kinds. Same rules, same
/// order, same errors.
pub fn existing_enemies(
    encounter: &Encounter,
    indices: &[usize],
    engineer_enabled: bool,
) -> Result<RosterEnemies, String> {
    existing_enemies_with(encounter, indices, engineer_enabled, fixes::active())
}

/// [`existing_enemies`] under explicit knobs.
pub fn existing_enemies_with(
    encounter: &Encounter,
    indices: &[usize],
    engineer_enabled: bool,
    knobs: &fixes::Fixes,
) -> Result<RosterEnemies, String> {
    let mut out = RosterEnemies::default();
    for &index in indices {
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
        let gate = wake_gate(entry, knobs);
        match kind {
            EnemyKind::Goblin => {
                out.goblins.push(at);
                out.goblin_wakes.push(gate);
            }
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
                    out.engineer_wakes.push(gate);
                } else if entry.fallback == Some(EnemyKind::Goblin) {
                    out.goblins.push(at);
                    out.goblin_wakes.push(gate);
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
    launch_with_fixes(root, roster, fixes::active())
}

/// [`launch_with`] under explicit knobs (the knob-off proofs).
pub fn launch_with_fixes(
    root: &Path,
    roster: Roster,
    knobs: &fixes::Fixes,
) -> Result<BoilerRun, String> {
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
        Roster::Live => {
            refuse_llama_wake(&loaded.encounter)?;
            roster_enemies_with(
                &loaded.encounter,
                SlotRoom {
                    floor_y: loaded.room.floor_y as f32,
                    walls: (0.0, kit.width as f32),
                },
                crate::engineer::active().enabled(),
                knobs,
            )?
        }
    };
    let wakes = enemies.wakes();
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
        room: crate::combat_scene::FixtureRoom::Corridor,
    }));
    Ok(BoilerRun {
        roster,
        kit,
        fixture,
        wakes,
        partner: knobs.partner_knobs(),
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

    /// BOIL-FU (PR-C): the descriptor's explicit `f01_roster` decides; the label rule is
    /// only the fallback of a recording that predates the field.
    #[test]
    fn the_explicit_roster_field_wins_and_the_label_is_only_the_fallback() {
        let from = Roster::from_descriptor;
        assert_eq!(from(Some("live"), None), Ok(Roster::Live));
        assert_eq!(from(Some("inert"), Some("story")), Ok(Roster::Inert));
        assert_eq!(from(Some("live"), Some("all")), Ok(Roster::Live));
        assert_eq!(from(None, Some("story")), Ok(Roster::Live), "legacy");
        assert_eq!(from(None, None), Ok(Roster::Inert), "legacy");
        let err = from(Some("bogus"), Some("story")).unwrap_err();
        assert!(err.contains("unknown Boiler roster"), "{err}");
        for roster in [Roster::Inert, Roster::Live] {
            assert_eq!(
                from(Some(roster.name()), None),
                Ok(roster),
                "names round-trip"
            );
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
        // FA-Q20: an entry's wake column is the earlier of its sight column (its
        // column minus the sight) and its `trigger_x`; the wake delay (counted
        // only for an entry that names a `trigger_x`) orders two entries on the same
        // column. The (column, delay) pairs are strictly increasing in roster order:
        // one enters at a time (g3 and e1 share 560, 15 ticks apart; FA-Q32: g2 wakes
        // at 224, 15 ticks after a hero leaves the protected spawn).
        let wake: Vec<(i32, u32)> = r
            .loaded
            .encounter
            .roster
            .iter()
            .map(|e| {
                let column = e.trigger_x.map_or(e.x - sight, |t| t.min(e.x - sight));
                (column, e.trigger_x.map_or(0, |_| e.delay_ticks))
            })
            .collect();
        assert_eq!(wake, [(124, 0), (224, 15), (560, 0), (560, 15)]);
        assert!(wake.windows(2).all(|w| w[0] < w[1]), "{wake:?}");
        // No more than `max_living` enemies at once (the design's initial probe).
        assert!(columns.len() <= usize::from(r.loaded.encounter.max_living));
    }

    /// BOILE-AD2 in a simulation, not only in columns: with Carl idle at his spawn
    /// nobody wakes, so every enemy holds its column for 300 ticks (with the roster's
    /// wake gates in the run: FA-Q20 keeps the idle room asleep).
    #[test]
    fn nobody_wakes_while_the_heroes_idle_at_the_spawn() {
        let r = live();
        let mut scene = SceneSim::new_at(&r.kit, 360, r.fixture.carl_x).unwrap();
        let mut combat = CombatState::new(r.fixture, &r.kit, &scene)
            .unwrap()
            .with_cf02()
            .with_env(env_runtime(&r).unwrap());
        assert!(combat.enemies.iter().skip(2).all(|g| g.wake_holds()));
        let at: Vec<f32> = combat.enemies.iter().map(|g| g.x).collect();
        for _ in 0..300 {
            combat.step_party(&mut scene, [RawInput::default(), RawInput::default()]);
        }
        let after: Vec<f32> = combat.enemies.iter().map(|g| g.x).collect();
        assert_eq!(at, after);
    }

    /// FA-Q20: a checkpoint restart rebuilds the room, so a woken B3 sleeps again, with
    /// its gate (the wake state is attempt state, the gate is run configuration).
    #[test]
    fn a_restart_puts_b3_back_to_sleep() {
        let r = live();
        let mut scene = SceneSim::new_at(&r.kit, 360, r.fixture.carl_x).unwrap();
        let mut combat = CombatState::new(r.fixture, &r.kit, &scene)
            .unwrap()
            .with_cf02()
            .with_env(env_runtime(&r).unwrap());
        combat.sync_env(&mut scene);
        for g in [0, 1] {
            combat.enemies[g].health = 0;
            combat.enemies[g].mode = crate::goblin::GoblinMode::Corpse;
        }
        let asleep: Vec<bool> = combat.enemies.iter().map(|g| g.wake_holds()).collect();
        assert_eq!(asleep[2..], [true, true]);
        scene.world.hero.x = 600.0;
        for _ in 0..100 {
            combat.step_party(&mut scene, [RawInput::default(), RawInput::default()]);
        }
        assert!(!combat.enemies[2].wake_holds() && !combat.enemies[3].wake_holds());
        combat.restart(&mut scene);
        assert!(combat.enemies[2].wake_holds() && combat.enemies[3].wake_holds());
        assert_eq!(combat.enemies[2].x, 944.0);
    }

    /// The gates of the live sidecar: g2 (`trigger_x` 224, FA-Q32), g3 and e1 (`trigger_x`
    /// 560) are gated; g1 (no trigger) keeps the plain sight wake and carries no gate.
    #[test]
    fn only_the_entries_with_a_trigger_carry_a_wake_gate() {
        let r = live();
        let gate = |trigger_x, delay_ticks| {
            Some(WakeGate {
                trigger_x: Some(trigger_x),
                delay_ticks,
                delay_after_sight: false,
            })
        };
        assert_eq!(r.wakes, [None, gate(224, 15), gate(560, 0), gate(560, 15)]);
        // The inert roster has no enemy, so no gate.
        assert!(run().wakes.is_empty());
    }

    /// FA-Q20 knob `fa_q20_wake_gates` off: no gate anywhere (the old sight-only wake).
    #[test]
    fn with_the_knob_off_no_entry_has_a_gate() {
        let off = fixes::Fixes {
            fa_q20_wake_gates: false,
            ..*fixes::active()
        };
        let r = launch_with_fixes(&root(), Roster::Live, &off).unwrap();
        assert!(r.wakes.is_empty());
    }

    /// FA-Q20, adopted `fa_q20_delay_after_sight: false`: an entry with a delay and no
    /// trigger has no gate; with the knob on its delay counts after the sight wake.
    #[test]
    fn a_delay_without_a_trigger_gates_only_when_the_delay_after_sight_knob_is_on() {
        let r = live();
        // FA-Q32 gave g2 a trigger; the old shape (a delay, no trigger) is g2 without it.
        let mut g2 = r
            .loaded
            .encounter
            .roster
            .iter()
            .find(|e| e.id == "g2")
            .unwrap()
            .clone();
        g2.trigger_x = None;
        g2.delay_ticks = 120;
        let g2 = &g2;
        let base = *fixes::active();
        assert_eq!(wake_gate(g2, &base), None);
        let on = fixes::Fixes {
            fa_q20_delay_after_sight: true,
            ..base
        };
        assert_eq!(
            wake_gate(g2, &on),
            Some(WakeGate {
                trigger_x: None,
                delay_ticks: 120,
                delay_after_sight: true
            })
        );
        let g1 = r
            .loaded
            .encounter
            .roster
            .iter()
            .find(|e| e.id == "g1")
            .unwrap();
        assert_eq!(wake_gate(g1, &on), None, "no delay, no trigger: no gate");
        let off = fixes::Fixes {
            fa_q20_wake_gates: false,
            ..on
        };
        assert_eq!(wake_gate(g2, &off), None, "the master knob wins");
    }

    /// A llama entry with a wake field is refused, never silently ignored.
    #[test]
    fn a_llama_with_a_wake_field_is_refused() {
        let r = run();
        assert!(refuse_llama_wake(&r.loaded.encounter).is_ok());
        for edit in [
            |e: &mut encounter::RosterEntry| e.delay_ticks = 30,
            |e: &mut encounter::RosterEntry| e.trigger_x = Some(500),
        ] {
            let mut enc = r.loaded.encounter.clone();
            let i = enc.roster.iter().position(|e| e.id == "g1").unwrap();
            enc.roster[i].kind = RosterKind::Room(EnemyKind::Llama);
            edit(&mut enc.roster[i]);
            let err = refuse_llama_wake(&enc).unwrap_err();
            assert!(err.contains("a llama has no wake gate"), "{err}");
        }
        let mut plain = r.loaded.encounter.clone();
        let i = plain.roster.iter().position(|e| e.id == "g1").unwrap();
        plain.roster[i].kind = RosterKind::Room(EnemyKind::Llama);
        assert!(
            refuse_llama_wake(&plain).is_ok(),
            "a llama with no wake field"
        );
    }

    /// Design finding (PR-C): the drum's solid (848..880) stands between g3 (944) and the
    /// steam lane, so a woken g3 cannot walk into 688..768 while the drum stands: it stops
    /// at the drum's face. The T27 symptom was a *hero* in the lane when g3 woke.
    #[test]
    fn the_drum_walls_g3_off_the_lane_until_it_blows() {
        let r = live();
        let mut scene = SceneSim::new_at(&r.kit, 360, r.fixture.carl_x).unwrap();
        let mut combat = CombatState::new(r.fixture, &r.kit, &scene)
            .unwrap()
            .with_cf02()
            .with_env(env_runtime(&r).unwrap());
        combat.sync_env(&mut scene);
        for g in [0, 1, 3] {
            combat.enemies[g].health = 0;
            combat.enemies[g].mode = crate::goblin::GoblinMode::Corpse;
        }
        scene.world.hero.x = 600.0;
        for _ in 0..400 {
            combat.step_party(&mut scene, [RawInput::default(), RawInput::default()]);
        }
        let g3 = &combat.enemies[2];
        assert!(!g3.wake_holds(), "woken by the trigger");
        assert!(g3.x > 880.0, "g3 stops at the drum's face, at {}", g3.x);
    }

    /// FA-Q20 (T27): g3 is the third enemy; once Carl reaches the trigger column it
    /// wakes (Carl is far from the lane then). The drum is blown first by a goblin
    /// launched into it (through the runtime, as in play), which opens the way and
    /// leaves g3, out of the blast's reach, at full health; g3 then walks to Carl
    /// through the steam lane (688..768). Carl, back at the valve, sets it off while g3
    /// is on its way and the steam scalds it (1 of its 2 health); without the vent the
    /// same run leaves it untouched. Scripted by position; g1 and e1 are taken out of
    /// the room and g2 (1 health) is the launched body.
    #[test]
    fn boil_b3_vented_in_the_lane() {
        const G3: usize = 2;
        let play = |vent: bool| {
            let r = live();
            let mut scene = SceneSim::new_at(&r.kit, 360, r.fixture.carl_x).unwrap();
            let mut combat = CombatState::new(r.fixture, &r.kit, &scene)
                .unwrap()
                .with_donut(DonutSetup::unlocked(8))
                .with_demo(r.rules)
                .with_cf02()
                .with_env(env_runtime(&r).unwrap());
            combat.sync_env(&mut scene);
            for g in [0, 3] {
                combat.enemies[g].health = 0;
                combat.enemies[g].mode = crate::goblin::GoblinMode::Corpse;
            }
            combat.enemies[1].health = 1;
            let walk = |dir: f32| RawInput {
                move_x: dir,
                ..RawInput::NONE
            };
            // 0: walk right to the trigger; 1: walk left; 2: walk right to the
            // valve's reach (facing it); 3: stand and jab.
            let mut phase = 0;
            let mut jabs: Vec<u64> = Vec::new();
            let (mut woke, mut in_lane) = (None, false);
            for t in 0..800u64 {
                if t == 5 {
                    // A fast rising arc into the drum's face (the `wall_stop` cases):
                    // the drum is set off by the launched body and blows 24 ticks later.
                    let g2 = &mut combat.enemies[1];
                    g2.x = 800.0;
                    g2.mode = crate::goblin::GoblinMode::Launched;
                    g2.vx = 250.0;
                    g2.vy = 150.0;
                    g2.launch = Some(9);
                }
                let x = scene.world.hero.x;
                match phase {
                    0 if x >= 562.0 => phase = 1,
                    1 if x <= 400.0 => phase = 2,
                    2 if x >= 460.0 => phase = 3,
                    _ => {}
                }
                let gx = combat.enemies[G3].x;
                let input = match phase {
                    0 | 2 => walk(1.0),
                    1 => walk(-1.0),
                    _ if vent && gx <= 800.0 && jabs.len() < 2 => {
                        if jabs.last().is_none_or(|j| t >= j + 30) {
                            jabs.push(t);
                            RawInput {
                                attack: true,
                                ..RawInput::NONE
                            }
                        } else {
                            RawInput::NONE
                        }
                    }
                    _ => RawInput::NONE,
                };
                combat.step_party(&mut scene, [input, RawInput::default()]);
                if woke.is_none() && !combat.enemies[G3].wake_holds() {
                    woke = Some(t);
                }
                in_lane |= (688.0..768.0).contains(&combat.enemies[G3].x);
            }
            let drum_blown =
                combat.env().unwrap().drum_state(2) == Some(crate::env_prop::DrumState::Detonated);
            let g3 = &combat.enemies[G3];
            assert!(drum_blown, "the launched body blew the drum");
            assert!(g3.alive(), "g3 survives with 1 of 2 health at most");
            (
                woke,
                in_lane,
                g3.health < crate::combat::ENEMY_HEALTH,
                jabs.len(),
            )
        };
        let (woke, in_lane, hurt, jabs) = play(true);
        assert!(
            woke.is_some_and(|t| t < 200),
            "g3 wakes at the trigger: {woke:?}"
        );
        assert!(in_lane, "and walks the lane");
        assert_eq!(jabs, 2);
        assert!(hurt, "the steam scalded g3 in the lane");
        let (_, in_lane, hurt, _) = play(false);
        assert!(in_lane && !hurt, "without the vent g3 is untouched");
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
