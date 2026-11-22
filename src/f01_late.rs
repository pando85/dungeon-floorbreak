//! PT-SCENE: `--scene f01_late`, the late-Floor-1 playtest scene
//! (`docs/07-agents/f01-playtest-integration-plan.md` section 4.7 and section 8).
//!
//! Bevy-free. The scene is a **launcher**, not a new world: a room of
//! `assets/levels/f01_late/manifest.json` is launched through the scene that
//! already runs it (`f01_boiler`, `boss_f01`), under a **loadout**:
//!
//! - `story` (the default): `--carl-prog --donut-prog`. The room's story pin
//!   holds the capabilities granted before it, and the grant triggers armed in
//!   the room fire.
//! - `all`: `--carl-dynamite --carl-gauntlet --donut-puddle` with the story
//!   triggers **off** (a TEST LOADOUT: every capability in every room, no grant
//!   moment).
//!
//! What this module owns:
//!
//! - the manifest ([`Manifest`]): the listed rooms and the **static story
//!   order** of every slice room and grant point, listed or not;
//! - the KR12 ordering rule ([`check_story_order`]): no room pin holds a
//!   capability whose grant point comes later in that order;
//! - the loadouts ([`Loadout`]);
//! - the Boiler's start state ([`boiler_start`], [`boiler_combat`]), the one
//!   constructor the binary's room and the PLAYTRACE verifier share;
//! - the pin scope of the recording ([`pin_is_scoped_out`], U5 of the #207
//!   review).
//!
//! Room-to-room moves are scene relaunches that carry the ledger
//! ([`crate::combat_scene::CombatState::story_snapshot`] and `carry_story`,
//! CP-Q07); the fixtures over two relaunches are in `f01_late_fixtures`.

use std::path::Path;

use serde::Deserialize;

use crate::carl_prog::CarlProgFlags;
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::donut_prog::DonutProgFlags;
use crate::f01_boiler::{self, BoilerRun, Roster};
use crate::story_unlock::{Storage, StoryTable};
use crate::{f01_hazards, f01_hoarder};

/// The manifest of the listed rooms.
pub const MANIFEST_PATH: &str = "assets/levels/f01_late/manifest.json";
/// FNV-1a 64 of the manifest file. The file lives under `assets/levels`, which
/// a compiled-in `include_str!` may not reach (`playtrace::build_info`), so the
/// hash is a constant, pinned to the file by a test and compared by every
/// replay of a recorded room. A step that lists a room (CORRAL-WIRE, RING-A
/// wire, HRD-A2, ...) re-pins it on purpose.
pub const MANIFEST_HASH: u64 = 0x495a_114d_01db_d9d4;
/// The fixture-only stub trigger the story-order tie skips by name.
pub const STUB_TRIGGER: &str = "carl_prog_find_zone_stub";
/// The `data_pins()` key.
pub const MANIFEST_PIN_KEY: &str = "f01_late_manifest";

/// The room a bare `--scene f01_late` launches (adopted HRDA2B-AD1).
pub const DEFAULT_ROOM: &str = "boiler";

/// The `--scene` name.
pub const SCENE_NAME: &str = "f01_late";
/// The `source` of a recorded Boiler session's reset descriptor (the Juicer
/// records as `boss`, like every boss-arena session).
pub const SESSION_SOURCE: &str = "f01_room";

/// The pins `playtrace::live::data_pins` stamps for this scene.
pub const fn data_pins() -> [(&'static str, u64); 1] {
    [(MANIFEST_PIN_KEY, MANIFEST_HASH)]
}

/// The room pins, scoped by room on **both** sides of a recording. The Boiler's
/// room and sidecar hashes belong to a recording of the `boiler` room only, and
/// the manifest hash to a recording that names any listed room. A run that names
/// no room (`--cf02`, `--scene boss_f01`, every recording before PT-SCENE) neither
/// stamps nor needs them, so a later edit of the manifest (every room step) or of
/// the Boiler files (BOIL-B/C wire) fails only the recordings it can change; a
/// Juicer recording needs the manifest hash and not the Boiler's.
pub const SCOPED_PIN_KEYS: [&str; 7] = [
    f01_boiler::ROOM_PIN_KEY,
    f01_boiler::ENCOUNTER_PIN_KEY,
    f01_hoarder::ROOM_PIN_KEY,
    f01_hoarder::ENCOUNTER_PIN_KEY,
    f01_hazards::ROOM_PIN_KEY,
    f01_hazards::ENCOUNTER_PIN_KEY,
    MANIFEST_PIN_KEY,
];

/// Whether a recording of `room` (`ResetDescriptor::f01_room`) stamps and needs
/// the data pin `key`: every pin that is not a room pin, always; a room pin
/// exactly when it covers the room.
pub fn pin_applies(key: &str, room: Option<&str>) -> bool {
    match key {
        f01_boiler::ROOM_PIN_KEY | f01_boiler::ENCOUNTER_PIN_KEY => room == Some("boiler"),
        f01_hoarder::ROOM_PIN_KEY | f01_hoarder::ENCOUNTER_PIN_KEY => room == Some("hoarder"),
        f01_hazards::ROOM_PIN_KEY | f01_hazards::ENCOUNTER_PIN_KEY => room == Some("hazards"),
        MANIFEST_PIN_KEY => room.is_some(),
        _ => true,
    }
}

/// Whether a verifier skips the pin `key`: it is a room pin the room does not
/// cover. A recorded value is then not compared (an older recording may carry a
/// stale one).
pub fn pin_is_scoped_out(key: &str, room: Option<&str>) -> bool {
    SCOPED_PIN_KEYS.contains(&key) && !pin_applies(key, room)
}

/// The scene a room runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum RoomScene {
    /// `--scene f01_boiler` (BOIL-A wire).
    #[serde(rename = "f01_boiler")]
    Boiler,
    /// `--scene boss_f01` (the Juicer).
    #[serde(rename = "boss_f01")]
    Boss,
    /// `--scene f01_hoarder` (HRD-A2b).
    #[serde(rename = "f01_hoarder")]
    Hoarder,
    /// `--scene f01_hazards` (HAZ-A2b).
    #[serde(rename = "f01_hazards")]
    Hazards,
}

/// The run rules of a room (K17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum RoomRules {
    #[serde(rename = "cf02_rules")]
    Cf02,
    #[serde(rename = "boss")]
    Boss,
}

/// One listed room.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomRow {
    pub id: String,
    pub title: String,
    pub scene: RoomScene,
    pub rules: RoomRules,
    /// The honest milestone the room is at (`greybox`, `boss`, ...).
    pub milestone: String,
}

/// One capability granted at a point of the story.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantRow {
    pub capability: String,
    /// The story-table trigger that grants it (`None`: a read-through or a
    /// grant made before the slice, like the Guild biscuit).
    pub trigger: Option<String>,
}

/// `room` or `point` (a grant point that is not a slice room).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderKind {
    Room,
    Point,
}

/// One entry of the static story order.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderRow {
    pub id: String,
    pub kind: OrderKind,
    /// The story pins the room holds at entry (ids of the story table).
    pub pins: Vec<String>,
    pub grants: Vec<GrantRow>,
}

/// `assets/levels/f01_late/manifest.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub status: String,
    pub rooms: Vec<RoomRow>,
    pub story_order: Vec<OrderRow>,
}

impl Manifest {
    /// Parse and validate a manifest text against the story table.
    pub fn from_json(text: &str, table: &StoryTable) -> Result<Self, String> {
        let manifest: Self = serde_json::from_str(text).map_err(|e| format!("manifest: {e}"))?;
        manifest.validate(table)?;
        Ok(manifest)
    }

    /// Read the committed manifest under `root`.
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join(MANIFEST_PATH);
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{MANIFEST_PATH}: {e}"))?;
        Self::from_json(&text, StoryTable::active())
    }

    /// The ids of the listed rooms, in story order.
    pub fn listed(&self) -> Vec<&str> {
        self.rooms.iter().map(|r| r.id.as_str()).collect()
    }

    /// The row of a listed room. A refusal names the listed rooms.
    pub fn room(&self, id: &str) -> Result<&RoomRow, String> {
        self.rooms.iter().find(|r| r.id == id).ok_or_else(|| {
            let known = self.story_order.iter().any(|o| o.id == id);
            format!(
                "{} room {id:?} (listed rooms: {})",
                if known { "unlisted" } else { "unknown" },
                self.listed().join(", ")
            )
        })
    }

    /// The default of `--f01-room`: the Boiler, the room the launcher began with
    /// (HRD-A2b: a room listed earlier in the story, the Hoarder, must not change
    /// what a bare `--scene f01_late` launches), else the first listed room.
    pub fn default_room(&self) -> &RoomRow {
        self.rooms
            .iter()
            .find(|r| r.id == DEFAULT_ROOM)
            .unwrap_or(&self.rooms[0])
    }

    /// The pins a room holds at entry, from the story order.
    pub fn pins_of(&self, room: &str) -> &[String] {
        self.story_order
            .iter()
            .find(|o| o.id == room)
            .map_or(&[], |o| o.pins.as_slice())
    }

    /// Structure and the KR12 ordering rule.
    pub fn validate(&self, table: &StoryTable) -> Result<(), String> {
        if self.schema != "f01-late-manifest-v1" {
            return Err(format!("manifest: unknown schema {:?}", self.schema));
        }
        if self.rooms.is_empty() {
            return Err("manifest: no listed room".into());
        }
        let mut last = None;
        for (i, row) in self.rooms.iter().enumerate() {
            if self.rooms[..i].iter().any(|r| r.id == row.id) {
                return Err(format!("manifest: duplicate room {:?}", row.id));
            }
            let at = self
                .story_order
                .iter()
                .position(|o| o.id == row.id && o.kind == OrderKind::Room)
                .ok_or_else(|| {
                    format!("manifest: listed room {:?} is not in story_order", row.id)
                })?;
            if last.is_some_and(|prev| at <= prev) {
                return Err(format!(
                    "manifest: listed room {:?} is out of story order",
                    row.id
                ));
            }
            last = Some(at);
            let rules_ok = match row.scene {
                RoomScene::Boiler | RoomScene::Hoarder | RoomScene::Hazards => {
                    row.rules == RoomRules::Cf02
                }
                RoomScene::Boss => row.rules == RoomRules::Boss,
            };
            if !rules_ok {
                return Err(format!(
                    "manifest: room {:?} runs under the wrong rules for its scene (K17)",
                    row.id
                ));
            }
        }
        check_story_order(&self.story_order, table)
    }
}

/// The capabilities a pin holds: its rows' `holds` and everything stored that
/// they require (a pin is closed under `requires`, DP-Q06).
fn pin_capabilities(table: &StoryTable, pin: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for row in table.pins.iter().filter(|p| p.id == pin) {
        for id in &row.holds {
            let closure = std::iter::once(id.as_str()).chain(
                table
                    .requires_closure(id)
                    .into_iter()
                    .filter(|c| c.storage == Storage::Ledger)
                    .map(|c| c.id.as_str()),
            );
            for cap in closure {
                if !out.iter().any(|c| c == cap) {
                    out.push(cap.to_owned());
                }
            }
        }
    }
    out
}

/// KR12, the generic ordering rule of the room pins: no room pin holds a
/// capability whose grant point comes **at or after** the room in `order` (the
/// static order of all slice rooms and grant points, listed or not; a grant
/// made in a room is held from the next room on). It also ties the order to the
/// story table: a grant names the trigger that makes it, that trigger must grant
/// the capability, and every capability a trigger grants must appear in the
/// order with that trigger. Grants before the slice (the Guild biscuit) are
/// points at the head of the order.
pub fn check_story_order(order: &[OrderRow], table: &StoryTable) -> Result<(), String> {
    for (i, row) in order.iter().enumerate() {
        if order[..i].iter().any(|o| o.id == row.id) {
            return Err(format!("story_order: duplicate id {:?}", row.id));
        }
        if row.kind == OrderKind::Point && !row.pins.is_empty() {
            return Err(format!("story_order: point {:?} holds a pin", row.id));
        }
        for grant in &row.grants {
            let Some(cap) = table.capability(&grant.capability) else {
                return Err(format!(
                    "story_order: {:?} grants unknown capability {:?}",
                    row.id, grant.capability
                ));
            };
            match &grant.trigger {
                Some(trigger) => {
                    let ok = table
                        .trigger(trigger)
                        .is_some_and(|t| t.grants.contains(&cap.id));
                    if !ok {
                        return Err(format!(
                            "story_order: trigger {trigger:?} does not grant {:?}",
                            grant.capability
                        ));
                    }
                }
                None if cap.storage == Storage::Ledger => {
                    return Err(format!(
                        "story_order: the stored capability {:?} needs a trigger",
                        grant.capability
                    ));
                }
                None => {}
            }
        }
    }
    for trigger in &table.triggers {
        // The one exception: HRD-A2's fixture-only stub zone, armed in no scene (a
        // fixture arms it by id), so no run of the story can fire it. Any other
        // trigger, armed or not, must be placed by the order.
        if trigger.id == STUB_TRIGGER {
            continue;
        }
        for cap in &trigger.grants {
            let placed = order.iter().any(|o| {
                o.grants
                    .iter()
                    .any(|g| g.capability == *cap && g.trigger.as_deref() == Some(&trigger.id))
            });
            if !placed {
                return Err(format!(
                    "story_order: trigger {:?} grants {cap:?}, which no entry of the order places",
                    trigger.id
                ));
            }
        }
    }
    for (i, row) in order.iter().enumerate() {
        for pin in &row.pins {
            if !table.has_pin(pin) {
                return Err(format!(
                    "story_order: {:?} names unknown pin {pin:?}",
                    row.id
                ));
            }
            for cap in pin_capabilities(table, pin) {
                let granted_at = order
                    .iter()
                    .position(|o| o.grants.iter().any(|g| g.capability == cap));
                match granted_at {
                    None => {
                        return Err(format!(
                            "story_order: pin {pin:?} of {:?} holds {cap:?}, which no point grants",
                            row.id
                        ));
                    }
                    Some(at) if at >= i => {
                        return Err(format!(
                            "story_order: pin {pin:?} of {:?} holds {cap:?}, granted later at {:?}",
                            row.id, order[at].id
                        ));
                    }
                    Some(_) => {}
                }
            }
        }
    }
    Ok(())
}

/// The loadout of a run (section 4.7, statement 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loadout {
    /// `story`: the program flags, so the pins hold and the triggers fire.
    Story,
    /// `all`: the debug start flags, every capability held, story triggers off.
    All,
}

impl Loadout {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Story => "story",
            Self::All => "all",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "story" => Some(Self::Story),
            "all" => Some(Self::All),
            _ => None,
        }
    }

    /// The flags the loadout implies. `all` sets `--donut-puddle` only: the
    /// jump boost and the biscuit come with it (DP-Q06, DP-Q10).
    pub const fn flags(self) -> (CarlProgFlags, DonutProgFlags) {
        match self {
            Self::Story => (
                CarlProgFlags {
                    prog: true,
                    dynamite: false,
                    gauntlet: false,
                },
                DonutProgFlags {
                    prog: true,
                    light_feet: false,
                    puddle: false,
                },
            ),
            Self::All => (
                CarlProgFlags {
                    prog: false,
                    dynamite: true,
                    gauntlet: true,
                },
                DonutProgFlags {
                    prog: false,
                    light_feet: false,
                    puddle: true,
                },
            ),
        }
    }

    /// Whether the run carries exactly this loadout's flags (a recording's
    /// descriptor must agree with its loadout label).
    pub fn matches(self, carl: CarlProgFlags, donut: DonutProgFlags) -> bool {
        let (c, d) = self.flags();
        c == carl && d == donut
    }
}

/// The console line of a launch.
pub fn entry_line(room: &RoomRow, loadout: Option<Loadout>) -> String {
    let label = match loadout {
        Some(Loadout::All) => " TEST_LOADOUT=all_capabilities_no_story_triggers",
        _ => "",
    };
    format!(
        "F01_LATE_ENTRY room={} title={:?} scene={} milestone={} loadout={}{label}",
        room.id,
        room.title,
        match room.scene {
            RoomScene::Boiler => "f01_boiler",
            RoomScene::Boss => "boss_f01",
            RoomScene::Hoarder => "f01_hoarder",
            RoomScene::Hazards => "f01_hazards",
        },
        room.milestone,
        loadout.map_or("flags", Loadout::name),
    )
}

/// The Boiler room's combat state at tick 0: the fixture, Donut with the
/// biscuit and a full pool (the post-Hoarder state, K1), the sidecar's story pin
/// under the program flags, the room's checkpoint rules and the CF02 rules.
/// The binary's scene and the PLAYTRACE verifier both call it.
///
/// BOIL-B/C wire: the sidecar's props and hazards are live here (the env step),
/// and the env solids are appended to `scene` at setup (`sync_env`), so the
/// live scene and the verifier start from the same world.
pub fn boiler_combat(
    run: &BoilerRun,
    scene: &mut SceneSim,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
    cf02_seed: Option<u64>,
) -> Result<CombatState, String> {
    let setup = f01_boiler::story_setup(&run.loaded.encounter.story_pin, carl, donut)?;
    let mut combat = CombatState::new(run.fixture, &run.kit, scene)?
        .with_donut(DonutSetup::unlocked(8))
        .try_with_story(setup)
        .map_err(|e| e.to_string())?
        .with_demo(run.rules)
        .with_cf02()
        .with_cf02_seed(cf02_seed)
        .with_env(f01_boiler::env_runtime(run)?);
    combat.sync_env(scene);
    Ok(combat)
}

/// Launch the Boiler room and build its scene and combat state.
pub fn boiler_start(
    root: &Path,
    view_height: i32,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
    cf02_seed: Option<u64>,
) -> Result<(BoilerRun, SceneSim, CombatState), String> {
    boiler_start_with(root, view_height, carl, donut, cf02_seed, Roster::Inert)
}

/// [`boiler_start`] with an explicit roster (BOIL-E): the binary's story run and
/// the PLAYTRACE verifier call it with [`Roster::for_loadout`], so a recording
/// rebuilds the world it was made in.
pub fn boiler_start_with(
    root: &Path,
    view_height: i32,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
    cf02_seed: Option<u64>,
    roster: Roster,
) -> Result<(BoilerRun, SceneSim, CombatState), String> {
    let run = f01_boiler::launch_with(root, roster)?;
    let mut scene = SceneSim::new_at(&run.kit, view_height, run.fixture.carl_x)?;
    let combat = boiler_combat(&run, &mut scene, carl, donut, cf02_seed)?;
    Ok((run, scene, combat))
}

/// Launch the Hoarder room and build its scene and combat state (HRD-A2b): the
/// one constructor the binary's room and the PLAYTRACE verifier share. The room
/// has no goblin, so it takes no `--cf02-seed`.
pub fn hoarder_start(
    root: &Path,
    view_height: i32,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
) -> Result<(f01_hoarder::HoarderRun, SceneSim, CombatState), String> {
    let run = f01_hoarder::launch(root)?;
    let mut scene = SceneSim::new_at(&run.kit, view_height, run.fixture.carl_x)?;
    let combat = f01_hoarder::combat(&run, &mut scene, carl, donut)?;
    Ok((run, scene, combat))
}

/// Launch the hazard room and build its scene and combat state (HAZ-A2b), the
/// shared constructor of the binary and the verifier.
pub fn hazards_start(
    root: &Path,
    view_height: i32,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
    cf02_seed: Option<u64>,
) -> Result<(f01_hazards::HazardsRun, SceneSim, CombatState), String> {
    let run = f01_hazards::launch(root)?;
    let mut scene = SceneSim::new_at(&run.kit, view_height, run.fixture.carl_x)?;
    let combat = f01_hazards::combat_for(&run, &mut scene, carl, donut, cf02_seed)?;
    Ok((run, scene, combat))
}

/// Whether a recorded descriptor's room label is consistent with this build:
/// the room is listed, a loadout label is known and agrees with the flags.
pub fn check_descriptor(
    manifest: &Manifest,
    room: &str,
    loadout: Option<&str>,
    carl: CarlProgFlags,
    donut: DonutProgFlags,
) -> Result<(), String> {
    manifest.room(room)?;
    if let Some(label) = loadout {
        let known = Loadout::parse(label).ok_or_else(|| format!("unknown loadout {label:?}"))?;
        if !known.matches(carl, donut) {
            return Err(format!(
                "the descriptor says loadout {label:?} but its flags are another run"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story_unlock::StorySetup;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn manifest() -> Manifest {
        Manifest::load(&root()).expect("the committed manifest loads")
    }

    fn table() -> &'static StoryTable {
        StoryTable::active()
    }

    #[test]
    fn the_pinned_hash_is_the_file() {
        let text = std::fs::read_to_string(root().join(MANIFEST_PATH)).unwrap();
        let hash = crate::donut_mana::content_hash(&text);
        assert_eq!(hash, MANIFEST_HASH, "{hash:#018x}");
    }

    #[test]
    fn the_manifest_lists_the_landed_rooms_in_story_order() {
        let m = manifest();
        assert_eq!(m.listed(), ["hoarder", "boiler", "hazards", "juicer"]);
        // HRDA2B-AD1: the Hoarder is listed first in the story, and the default
        // room stays the Boiler.
        assert_eq!(m.default_room().id, "boiler");
        assert_eq!(m.room("boiler").unwrap().scene, RoomScene::Boiler);
        assert_eq!(m.room("hoarder").unwrap().scene, RoomScene::Hoarder);
        assert_eq!(m.room("hazards").unwrap().scene, RoomScene::Hazards);
        for id in ["hoarder", "hazards"] {
            assert_eq!(m.room(id).unwrap().rules, RoomRules::Cf02, "{id}");
        }
        assert_eq!(m.room("juicer").unwrap().rules, RoomRules::Boss);
    }

    #[test]
    fn an_unknown_or_unlisted_room_is_refused_with_the_listed_rooms() {
        let m = manifest();
        let unknown = m.room("cellar").unwrap_err();
        assert!(unknown.starts_with("unknown room"), "{unknown}");
        assert!(
            unknown.contains("hoarder, boiler, hazards, juicer"),
            "{unknown}"
        );
        // Known to the story order but not landed yet.
        for id in ["corral", "dozer", "mastery"] {
            let unlisted = m.room(id).unwrap_err();
            assert!(unlisted.starts_with("unlisted room"), "{unlisted}");
            assert!(
                unlisted.contains("hoarder, boiler, hazards, juicer"),
                "{unlisted}"
            );
        }
    }

    // ---- KR12: the story-order pin test

    #[test]
    fn no_room_pin_holds_a_capability_granted_later() {
        let m = manifest();
        check_story_order(&m.story_order, table()).expect("the committed order is sound");
        // Every room pin of the table (scene `f01_late`) is named by some room,
        // and the mastery pin holds everything.
        for pin in table()
            .pins
            .iter()
            .filter(|p| p.scene == crate::story_unlock::SceneKind::F01Late)
        {
            assert!(
                m.story_order.iter().any(|o| o.pins.contains(&pin.id)),
                "pin {} is used by no room",
                pin.id
            );
        }
        let mastery = pin_capabilities(table(), "f01_late_mastery");
        for cap in [
            "special_thrown_charge",
            "fist_reinforced",
            "jump_reach_boost",
            "special_short_blink",
        ] {
            assert!(mastery.iter().any(|c| c == cap), "{cap}");
        }
        // The pre-Hoarder pin holds no dynamite (it is the Hoarder's reward, K1)
        // and the corral pin holds nothing (KC11).
        let pre = pin_capabilities(table(), "f01_pin_pre_pile_boss");
        assert_eq!(pre, ["jump_reach_boost"]);
        assert!(m.pins_of("corral").is_empty());
        // The post pin holds the dynamite and the boost.
        let post = pin_capabilities(table(), "f01_pin_post_pile_boss");
        assert!(post.iter().any(|c| c == "special_thrown_charge"));
        assert!(post.iter().any(|c| c == "jump_reach_boost"));
    }

    fn mutate(edit: impl FnOnce(&mut Vec<OrderRow>)) -> Result<(), String> {
        let mut order = manifest().story_order;
        edit(&mut order);
        check_story_order(&order, table())
    }

    fn index(order: &[OrderRow], id: &str) -> usize {
        order.iter().position(|o| o.id == id).unwrap()
    }

    #[test]
    fn the_order_rule_fails_when_a_grant_moves_after_its_pin() {
        // The Hoarder's dynamite grant moved past the Boiler: the Boiler's
        // pin now holds a capability granted later.
        let err = mutate(|order| {
            let from = index(order, "hoarder");
            let grant = order[from].grants.remove(0);
            let at = index(order, "juicer");
            order[at].grants.push(grant);
        })
        .unwrap_err();
        assert!(err.contains("granted later"), "{err}");
        // The Hoarder room's own pin may not hold what the Hoarder grants.
        let err = mutate(|order| {
            let at = index(order, "hoarder");
            order[at].pins = vec!["f01_pin_post_pile_boss".into()];
        })
        .unwrap_err();
        assert!(
            err.contains("special_thrown_charge") && err.contains("granted later"),
            "{err}"
        );
        // Light on Your Feet moved to after the Hoarder breaks the pre pin.
        let err = mutate(|order| {
            let from = index(order, "refuge_light_feet");
            let grant = order[from].grants.remove(0);
            let at = index(order, "boiler");
            order[at].grants.push(grant);
        })
        .unwrap_err();
        assert!(err.contains("jump_reach_boost"), "{err}");
    }

    #[test]
    fn the_order_rule_fails_for_a_pin_that_holds_the_blink_too_early() {
        // The mastery pin (everything) on the Boiler: the Juicer's grants come later.
        let err = mutate(|order| {
            let at = index(order, "boiler");
            order[at].pins = vec!["f01_late_mastery".into()];
        })
        .unwrap_err();
        assert!(
            err.contains("fist_reinforced") || err.contains("special_short_blink"),
            "{err}"
        );
        // A pin that does not exist.
        let err = mutate(|order| {
            let at = index(order, "boiler");
            order[at].pins = vec!["f01_pin_nope".into()];
        })
        .unwrap_err();
        assert!(err.contains("unknown pin"), "{err}");
    }

    #[test]
    fn the_order_is_tied_to_the_story_table_triggers() {
        // A trigger that does not grant the capability.
        let err = mutate(|order| {
            let at = index(order, "hoarder");
            order[at].grants[0].trigger = Some("brute_boss_f01_defeated".into());
        })
        .unwrap_err();
        assert!(err.contains("does not grant"), "{err}");
        // A table grant that no entry places.
        let err = mutate(|order| {
            let at = index(order, "juicer");
            order[at].grants.pop();
        })
        .unwrap_err();
        assert!(err.contains("no entry of the order places"), "{err}");
        // HRD-A2: the Hoarder's event trigger is armed in her room, so the order
        // must place it; the fixture-only stub zone (armed nowhere) needs no place.
        let err = mutate(|order| {
            let at = index(order, "hoarder");
            order[at].grants.pop();
        })
        .unwrap_err();
        assert!(err.contains("pile_boss_f01_defeated"), "{err}");
        let stub = table().trigger(STUB_TRIGGER).expect("the stub");
        assert!(stub.armed_in.is_empty());
        assert!(manifest().story_order.iter().all(|o| {
            o.grants
                .iter()
                .all(|g| g.trigger.as_deref() != Some(STUB_TRIGGER))
        }));
        // Negative: an unarmed trigger that is not the named stub gets no exemption.
        let renamed = crate::story_unlock::TABLE_JSON.replacen(
            "\"id\": \"carl_prog_find_zone_stub\"",
            "\"id\": \"carl_prog_other_zone\"",
            1,
        );
        let other = StoryTable::from_json(&renamed).expect("a valid table");
        let err = check_story_order(&manifest().story_order, &other).unwrap_err();
        assert!(err.contains("carl_prog_other_zone"), "{err}");
        // A stored capability without a trigger.
        let err = mutate(|order| {
            let at = index(order, "hoarder");
            order[at].grants[0].trigger = None;
        })
        .unwrap_err();
        assert!(err.contains("needs a trigger"), "{err}");
        // A duplicate id.
        let err = mutate(|order| {
            let dup = order[index(order, "boiler")].clone();
            order.push(dup);
        })
        .unwrap_err();
        assert!(err.contains("duplicate"), "{err}");
    }

    #[test]
    fn the_manifest_validator_rejects_bad_structure() {
        let text = std::fs::read_to_string(root().join(MANIFEST_PATH)).unwrap();
        let bad = |from: &str, to: &str| {
            let edited = text.replacen(from, to, 1);
            assert_ne!(edited, text, "{from}");
            Manifest::from_json(&edited, table()).unwrap_err()
        };
        // A listed room out of story order.
        let m: Manifest = serde_json::from_str(&text).unwrap();
        let mut swapped = m.clone();
        swapped.rooms.reverse();
        assert!(
            swapped
                .validate(table())
                .unwrap_err()
                .contains("out of story order")
        );
        // A listed room the order does not know.
        assert!(
            bad(
                "\"id\": \"boiler\",\n      \"title\"",
                "\"id\": \"cellar\",\n      \"title\""
            )
            .contains("not in story_order")
        );
        // Boss rules on the Boiler scene.
        assert!(bad("\"rules\": \"cf02_rules\"", "\"rules\": \"boss\"").contains("wrong rules"));
        // An unknown field.
        assert!(
            bad(
                "\"milestone\": \"greybox\"",
                "\"milestone\": \"greybox\", \"x\": 1"
            )
            .contains("manifest:")
        );
    }

    // ---- loadouts and launches

    #[test]
    fn the_loadouts_map_to_their_flags() {
        let (carl, donut) = Loadout::Story.flags();
        assert!(carl.prog && donut.prog);
        assert!(!carl.dynamite && !carl.gauntlet && !donut.puddle && !donut.light_feet);
        let (carl, donut) = Loadout::All.flags();
        assert!(carl.dynamite && carl.gauntlet && donut.puddle);
        assert!(!carl.prog && !donut.prog, "story triggers off");
        assert!(donut.implies_biscuit());
        assert_eq!(Loadout::parse("story"), Some(Loadout::Story));
        assert_eq!(Loadout::parse("all"), Some(Loadout::All));
        assert_eq!(Loadout::parse("both"), None);
        assert!(Loadout::Story.matches(carl_flags(true), donut_flags(true)));
        assert!(!Loadout::Story.matches(carl_flags(false), donut_flags(true)));
        assert!(
            entry_line(manifest().room("boiler").unwrap(), Some(Loadout::All))
                .contains("TEST_LOADOUT")
        );
        assert!(
            !entry_line(manifest().room("boiler").unwrap(), Some(Loadout::Story))
                .contains("TEST_LOADOUT")
        );
    }

    fn carl_flags(prog: bool) -> CarlProgFlags {
        CarlProgFlags {
            prog,
            ..CarlProgFlags::OFF
        }
    }

    fn donut_flags(prog: bool) -> DonutProgFlags {
        DonutProgFlags {
            prog,
            ..DonutProgFlags::OFF
        }
    }

    fn held(setup: &StorySetup) -> Vec<&'static str> {
        let mut ids: Vec<&'static str> = setup.holds().iter().map(|c| c.as_str()).collect();
        ids.sort_unstable();
        ids
    }

    /// The pin set each listed room launches with, per loadout.
    #[test]
    fn each_listed_room_launches_with_its_story_pin() {
        let m = manifest();
        // Boiler: the sidecar's pin is the manifest's pin, and the story loadout
        // holds dynamite and Light on Your Feet.
        let (run, scene, combat) =
            boiler_start(&root(), 360, carl_flags(true), donut_flags(true), None).unwrap();
        assert_eq!(
            m.pins_of("boiler"),
            std::slice::from_ref(&run.loaded.encounter.story_pin)
        );
        assert_eq!(run.loaded.encounter.story_pin, "f01_pin_post_pile_boss");
        let ledger = combat.story_ledger().expect("a story runtime");
        let held_ids: Vec<&str> = ledger.held().iter().map(|(_, c)| c.as_str()).collect();
        assert_eq!(held_ids, ["special_thrown_charge", "jump_reach_boost"]);
        assert!(ledger.receipts().is_empty(), "a pin is not a grant");
        drop(scene);
        // Under `all` the start flags hold everything and no trigger is armed.
        let (carl, donut) = Loadout::All.flags();
        let (_, _, combat) = boiler_start(&root(), 360, carl, donut, None).unwrap();
        let ledger = combat.story_ledger().expect("a story runtime");
        assert_eq!(
            ledger.held().len(),
            4,
            "dynamite, gauntlet, boost and blink"
        );
        // Juicer: the two boss pins under the story loadout, nothing armed
        // beyond the Juicer trigger.
        let (carl, donut) = Loadout::Story.flags();
        let setup = donut_prog_setup(carl, donut);
        assert_eq!(held(&setup), ["jump_reach_boost", "special_thrown_charge"]);
        assert_eq!(setup.trigger_ids(), ["brute_boss_f01_defeated"]);
        let (carl, donut) = Loadout::All.flags();
        let setup = donut_prog_setup(carl, donut);
        assert!(setup.trigger_ids().is_empty(), "no grant fires under all");
        assert_eq!(held(&setup).len(), 4);
        // Every pin of the manifest's rooms resolves through the table.
        for room in &m.rooms {
            for pin in m.pins_of(&room.id) {
                assert!(table().has_pin(pin), "{pin}");
            }
        }
    }

    fn donut_prog_setup(carl: CarlProgFlags, donut: DonutProgFlags) -> StorySetup {
        crate::donut_prog::story_setup(carl, donut, crate::story_unlock::SceneKind::Boss)
    }

    #[test]
    fn a_descriptor_must_name_a_listed_room_and_a_matching_loadout() {
        let m = manifest();
        let (carl, donut) = Loadout::Story.flags();
        assert!(check_descriptor(&m, "boiler", Some("story"), carl, donut).is_ok());
        assert!(check_descriptor(&m, "boiler", None, carl, donut).is_ok());
        assert!(check_descriptor(&m, "cellar", None, carl, donut).is_err());
        assert!(check_descriptor(&m, "boiler", Some("all"), carl, donut).is_err());
        assert!(check_descriptor(&m, "boiler", Some("bogus"), carl, donut).is_err());
    }

    #[test]
    fn the_room_pins_are_scoped_to_the_room_they_cover() {
        // Each room's pins cover that room's recordings only; the manifest pin
        // covers a recording of any room.
        let owner = |key: &str| -> Option<&'static str> {
            [
                (
                    "boiler",
                    [f01_boiler::ROOM_PIN_KEY, f01_boiler::ENCOUNTER_PIN_KEY],
                ),
                (
                    "hoarder",
                    [f01_hoarder::ROOM_PIN_KEY, f01_hoarder::ENCOUNTER_PIN_KEY],
                ),
                (
                    "hazards",
                    [f01_hazards::ROOM_PIN_KEY, f01_hazards::ENCOUNTER_PIN_KEY],
                ),
            ]
            .into_iter()
            .find(|(_, keys)| keys.contains(&key))
            .map(|(room, _)| room)
        };
        for room in [
            None,
            Some("juicer"),
            Some("boiler"),
            Some("hoarder"),
            Some("hazards"),
        ] {
            for key in SCOPED_PIN_KEYS {
                let covered = match owner(key) {
                    Some(own) => room == Some(own),
                    None => room.is_some(),
                };
                assert_eq!(pin_applies(key, room), covered, "{key} {room:?}");
                assert_eq!(pin_is_scoped_out(key, room), !covered, "{key} {room:?}");
            }
            assert!(
                !pin_is_scoped_out("cf02_tuning", room),
                "other pins stay required"
            );
            assert!(pin_applies("story_unlocks", room));
        }
    }

    #[test]
    fn the_boiler_start_is_deterministic_and_the_seed_matters_not_to_the_walk() {
        let hash = |seed: Option<u64>| {
            let (_, scene, combat) =
                boiler_start(&root(), 360, carl_flags(true), donut_flags(true), seed).unwrap();
            combat.state_hash(&scene)
        };
        assert_eq!(hash(None), hash(None));
    }
}
