//! BOIL-A core: encounter data and gate validation. Pure, Bevy-free, headless.
//!
//! A room file in the existing `floor01-demo-hall-v1` schema
//! ([`crate::camera_lab::room::parse_rooms`], the only room parser) holds the
//! **geometry**. Next to it an `*.encounter.json` sidecar (schema
//! [`SCHEMA`]) holds what the room does not: the story pin, the goal, the
//! protected spawn, exit and checkpoint rectangles, the interactive props, the
//! hazard volumes, the roster, the link graph and the height-gate classes.
//! [`load`] reads both and **rejects the sidecar with every problem it finds**
//! (never the first one only), so an authoring mistake is visible at load time:
//!
//! * unknown prop ids, links that form a cycle, more than
//!   [`MAX_AUTHORED_LINKS`] links (K2), a link without a physical contact;
//! * hazards on the protected spawn, exit or checkpoint, hazards without a
//!   readable telegraph, hazards that stay active too long;
//! * a roster that spawns into a damage volume, on the protected spawn, with a
//!   synchronized spawn, or with more than [`MAX_LIVING_FIRST_PROBE`] enemies;
//! * a room that grants anything (a room never unlocks an ability);
//! * reachability: for Carl, base Donut, boosted Donut and two players
//!   together, in **every destruction state** of the props, the exit lane
//!   stays open, every perch a hero class is meant to reach is reachable and
//!   has a way back, both spawns regroup, and **a gate is not bypassed**: a
//!   surface's effective class is the weakest class whose route graph reaches
//!   it (stacked perches included) and it must not be weaker than the declared
//!   one. The classes (Carl 80 px, base Donut 104 px, boosted Donut 128 px,
//!   their reach, the drop gap, Carl's height)
//!   are **data** ([`HeightClasses`], `assets/data/gameplay/height_classes_v0.json`),
//!   not constants of this module: the jump physics belong to DONUT-MOBILITY-01
//!   and DONUT-PROG-01 and may move;
//! * the **DP-Q45 ceiling rule**: a room with a surface above the rule's free
//!   height (42 px) that the boosted Donut can stand on declares a `ceiling_px`
//!   in its room file (an optional field of the scenario, absent = none) of at
//!   most 228 px. The rule is applied to every room: any Floor 01 room can be
//!   run with the all-capabilities test loadout;
//! * the **K8 nest rule**: a surface 81..=128 px above the walking line that
//!   no enemy threatens is an error unless it is declared as a vantage with a
//!   counter.
//!
//! * **locks**: a solid that stays closed until a named event (an exit that is
//!   solid until the arena is clear). The target must exist, be neither
//!   optional nor missing, the closed solid may not wall a hero off from it, and
//!   the exit is checked with every lock open.
//! * the room file keeps its `props`, `enemies` and `hazards` arrays **empty**.
//!
//! Disclosed limits: the reachability uses the height classes and flat-jump
//! distances of the data file (an optimistic upper bound for a rising jump
//! between perches: `encounter_physics` replays every accepted edge with the real
//! integrator, BOIL-A wire), not the real
//! jump physics, and it does not trust
//! the composer's generated access steps (a scene may clear them, as the
//! microarena does), so it proves the jump-only case. The room parser of this
//! branch does not read `ceiling_px` yet (DM-A adds the field to the loader);
//! [`load`] reads it from the room file itself and this module never edits the
//! parser. It checks that a ceiling is declared and low enough, not that Carl's
//! own arcs stay under it. The room-file snapping (`snap_dy`) is applied by the
//! room parser; sidecar rectangles are in the snapped layout space.
//!
//! Everything is integer math; there is no RNG and no wall-clock time.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;

use crate::camera_lab::room::{EnemyKind, IRect, Room, parse_rooms};
use crate::env_hazard::{HazardClass, HazardSet, HazardSetError, HazardSpec, HazardWindow};
use crate::env_prop::{ShutterConfig, ShutterInitial, ValveConfig};
use crate::hero_damage::HeroDamagePolicy;
use crate::rot_sticker::{Orientation, Track, TrackError};
use crate::slime::AcidPatchSpec;

mod haz;

pub const SCHEMA: &str = "floor01-encounter-v1";
pub const BOILER_ENCOUNTER_PATH: &str = "assets/levels/floor01/f01_boiler.encounter.json";
pub const BOILER_ROOM_PATH: &str = "assets/levels/floor01/f01_boiler.room.json";
/// HAZ-A1: the hazard room (`docs/01-design/enemy-gameplay-slime-rot.md`).
pub const HAZARDS_ENCOUNTER_PATH: &str = "assets/levels/floor01/f01_hazards.encounter.json";
pub const HAZARDS_ROOM_PATH: &str = "assets/levels/floor01/f01_hazards.room.json";
pub const HEIGHT_CLASSES_PATH: &str = "assets/data/gameplay/height_classes_v0.json";
pub const HEIGHT_CLASSES_SCHEMA: &str = "floor01-height-classes-v1";

/// HAZ rule 1: a standing-start jump over an acid patch starts with the hero's
/// front edge 0..=this many px before the patch (the take-off zone).
pub const TAKEOFF_ZONE_PX: i32 = 8;
/// HAZ rule 1: dry floor required on both sides of an acid patch.
pub const MIN_DRY_SIDE_PX: i32 = 48;
/// HAZ rule 3: a sticker's blast sweep leaves a dry, blast-free pocket this
/// wide on each side of its track.
pub const MIN_POCKET_PX: i32 = 64;
/// HAZ rule 3: a sticker's dome stays within the shared camera's band.
pub const STICKER_BAND_PX: i32 = 200;
/// HAZ rule 4: tracks of different stickers (and a track and an authored
/// charge) are more than this far apart (48 + 8).
pub const STICKER_SEPARATION_PX: i32 = 56;
/// HAZ rule 5: caps per pocket.
pub const POCKET_MAX_STICKERS: u8 = 2;
pub const POCKET_MAX_SLIMES: u8 = 1;
pub const POCKET_MAX_GOBLINS: u8 = 1;

/// K2 / #176: at most this many authored prop-to-prop links, acyclic.
pub const MAX_AUTHORED_LINKS: usize = 3;
/// #176 B3: at most this many living enemies in the first probe.
pub const MAX_LIVING_FIRST_PROBE: u8 = 4;
/// PT-MASTERY (PTM-AD6): a sidecar that opts in with `"waves": true` may hold
/// more than `max_living` roster entries in total, as long as every beat (a
/// wave) holds at most `max_living` of them and the whole roster at most this
/// many. Without the opt-in the whole roster is bounded by `max_living`, as
/// before (HAZ rule 5's cap of four stays for every other room).
pub const MAX_ROSTER_WAVES: usize = 8;
/// At most this many props (the validator enumerates their destruction states).
pub const MAX_PROPS: usize = 8;
/// A landed remnant or a solid prop on the lane is at most this high.
pub const MAX_LANE_OBSTACLE_PX: i32 = 48;
/// A hanging weight's lowest point is at least this far above the lane.
pub const MIN_HANG_PX: i32 = 96;
/// Contact margin of a landing link: the footprint grows by this on each side.
pub const LANDING_CONTACT_MARGIN_PX: i32 = 8;
/// Spawns of the same trigger differ by at least this many ticks (1: no two
/// share a tick).
pub const MIN_SPAWN_STAGGER_TICKS: u32 = 1;
/// Enemy body used for the spawn checks.
pub const ENEMY_BODY: (i32, i32) = (24, 52);
/// Story gates a roster entry may name, each with a required fallback kind.
pub const KNOWN_GATES: [&str; 1] = ["cf02_engineer"];

// ------------------------------------------------------------------ classes

/// What a hero can do to get around, for the route checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReachHero {
    Carl,
    DonutBase,
    DonutBoosted,
}

impl ReachHero {
    pub const ALL: [Self; 3] = [Self::Carl, Self::DonutBase, Self::DonutBoosted];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Carl => "carl",
            Self::DonutBase => "donut_base",
            Self::DonutBoosted => "donut_boosted",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|h| h.name() == name)
    }

    /// Carl's spawn for Carl, Donut's for both Donut classes.
    fn spawn_x(self, room: &Room) -> i32 {
        match self {
            Self::Carl => room.spawn_carl,
            _ => room.spawn_donut,
        }
    }
}

/// The class of a surface: the weakest hero class that reaches it.
pub type GateClass = ReachHero;

/// One class of the height-class data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassSpec {
    /// Highest surface (px above the walking line) the class lands on.
    pub max_rise_px: i32,
    /// Widest flat gap (px) a jump of the class bridges.
    pub reach_x_px: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClasses {
    carl: ClassSpec,
    donut_base: ClassSpec,
    donut_boosted: ClassSpec,
}

/// DP-Q45 as data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CeilingRule {
    /// A surface at most this high needs no lane ceiling.
    pub free_surface_px: i32,
    /// The ceiling's underside is at most this far above the walking line.
    pub ceiling_max_px: i32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHeightClasses {
    schema: String,
    #[allow(dead_code)]
    status: String,
    classes: RawClasses,
    donut_base_human_only_px: i32,
    enemy_melee_reach_px: i32,
    drop_gap_px: i32,
    carl_height_px: i32,
    ceiling_rule: CeilingRule,
}

/// The level-authoring height classes, read from data (never copied physics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeightClasses {
    carl: ClassSpec,
    donut_base: ClassSpec,
    donut_boosted: ClassSpec,
    /// Informational: the narrow human-only ledge of base Donut (DP-Q18). It
    /// feeds no class, reach, nest or ceiling rule: the AI never attempts it.
    pub donut_base_human_only_px: i32,
    /// How far above or below its own standing surface a melee enemy's attack
    /// reaches (the goblin swing box, 0..30 px).
    pub enemy_melee_reach_px: i32,
    /// Walking off an edge and dropping bridges at most this horizontal gap.
    pub drop_gap_px: i32,
    /// Carl's collider height: the lane under a perch must clear it.
    pub carl_height_px: i32,
    pub ceiling_rule: CeilingRule,
}

impl HeightClasses {
    pub fn parse(text: &str) -> Result<Self, EncounterError> {
        let bad = |reason: String| EncounterError::ClassesInvalid(reason);
        let raw: RawHeightClasses =
            serde_json::from_str(text).map_err(|e| bad(format!("parse: {e}")))?;
        if raw.schema != HEIGHT_CLASSES_SCHEMA {
            return Err(bad(format!("schema {:?}", raw.schema)));
        }
        let c = Self {
            carl: raw.classes.carl,
            donut_base: raw.classes.donut_base,
            donut_boosted: raw.classes.donut_boosted,
            donut_base_human_only_px: raw.donut_base_human_only_px,
            enemy_melee_reach_px: raw.enemy_melee_reach_px,
            drop_gap_px: raw.drop_gap_px,
            carl_height_px: raw.carl_height_px,
            ceiling_rule: raw.ceiling_rule,
        };
        let specs = [c.carl, c.donut_base, c.donut_boosted];
        if specs
            .iter()
            .any(|s| s.max_rise_px <= 0 || s.reach_x_px <= 0)
        {
            return Err(bad("every class needs a positive rise and reach".into()));
        }
        if specs
            .windows(2)
            .any(|w| w[0].max_rise_px >= w[1].max_rise_px || w[0].reach_x_px > w[1].reach_x_px)
        {
            return Err(bad(
                "rises must grow carl < donut_base < donut_boosted and reaches must not shrink"
                    .into(),
            ));
        }
        if c.donut_base_human_only_px < c.donut_base.max_rise_px || c.enemy_melee_reach_px <= 0 {
            return Err(bad(
                "donut_base_human_only_px must not be under the base class and enemy_melee_reach_px must be positive"
                    .into(),
            ));
        }
        if c.drop_gap_px <= 0 || c.carl_height_px <= 0 {
            return Err(bad("drop_gap_px and carl_height_px must be positive".into()));
        }
        let rule = c.ceiling_rule;
        if rule.free_surface_px < 0 || rule.ceiling_max_px <= rule.free_surface_px {
            return Err(bad("ceiling_rule is inconsistent".into()));
        }
        Ok(c)
    }

    pub fn load(root: &Path) -> Result<Self, EncounterError> {
        let path = root.join(HEIGHT_CLASSES_PATH);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| EncounterError::Io(format!("{}: {e}", path.display())))?;
        Self::parse(&text)
    }

    pub fn spec(&self, hero: ReachHero) -> ClassSpec {
        match hero {
            ReachHero::Carl => self.carl,
            ReachHero::DonutBase => self.donut_base,
            ReachHero::DonutBoosted => self.donut_boosted,
        }
    }

    pub fn max_rise(&self, hero: ReachHero) -> i32 {
        self.spec(hero).max_rise_px
    }

    pub fn reach_x(&self, hero: ReachHero) -> i32 {
        self.spec(hero).reach_x_px
    }

    /// The gate class of a surface `rise` px above the walking line, `None`
    /// above every class (nobody lands on it).
    pub fn gate_class(&self, rise: i32) -> Option<GateClass> {
        ReachHero::ALL
            .into_iter()
            .find(|h| rise <= self.max_rise(*h))
    }
}

/// What the validator needs besides the sidecar: the parsed room, the height
/// classes and the room file's optional `ceiling_px`.
#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    pub room: Room,
    pub classes: HeightClasses,
    pub ceiling_px: Option<i32>,
}

/// What answers a surface that a hero class reaches but Carl does not (K8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VantageCounter {
    /// A lane ceiling bonks the jump (DP-Q45).
    LaneCeiling,
    /// A ranged enemy of the roster can hit the surface from the lane.
    GroundRanged,
    /// A reinforcement wave pressures whoever holds it.
    ReinforcementWave,
}

impl VantageCounter {
    pub const fn name(self) -> &'static str {
        match self {
            Self::LaneCeiling => "lane_ceiling",
            Self::GroundRanged => "ground_ranged",
            Self::ReinforcementWave => "reinforcement_wave",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "lane_ceiling" => Self::LaneCeiling,
            "ground_ranged" => Self::GroundRanged,
            "reinforcement_wave" => Self::ReinforcementWave,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropKind {
    Valve,
    WeightRig,
    Drum,
    /// HAZ-A1: a Rot Sticker on a wall or ceiling track. An env prop that owns
    /// no hazard volume: its blast is an authored `BlastCharge` (HAZ-A2).
    RotSticker,
    /// RING-B core (KR3): a portcullis in an arch under a platform slab. Closed,
    /// it is a cut for every class and never a standable node.
    Shutter,
}

impl PropKind {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "valve" => Self::Valve,
            "weight_rig" => Self::WeightRig,
            "drum" => Self::Drum,
            "rot_sticker" => Self::RotSticker,
            "shutter" => Self::Shutter,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    /// The `from` prop's steam hazard reaches the `to` prop's target.
    Steam,
    /// The `from` weight's landing footprint (plus a contact margin) reaches
    /// the `to` prop's target.
    Landing,
    /// The `from` prop's blast radius reaches the `to` prop's target.
    Blast,
}

impl LinkKind {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "steam" => Self::Steam,
            "landing" => Self::Landing,
            "blast" => Self::Blast,
            _ => return None,
        })
    }
}

/// What completes the room. The format reads the goal as a string, so a later
/// step appends a kind (for example `defeat_roster`) without breaking any
/// existing sidecar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Goal {
    ReachExit,
    /// HAZ-Q39: the exit opens when every roster enemy has an `EnemyDefeated`;
    /// props (a Rot Sticker) never count. HAZ-A2 opens the exit at runtime.
    DefeatRoster,
}

impl Goal {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "reach_exit" => Self::ReachExit,
            "defeat_roster" => Self::DefeatRoster,
            _ => return None,
        })
    }
}

/// The event that opens a [`Lock`]. Appended to by later steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LockEvent {
    /// The roster entry at this index of [`Encounter::roster`] is defeated.
    EnemyDefeated { roster: usize },
}

/// A solid that stays closed until a named event (an exit that is solid until
/// the arena is clear). The hook is [`Lock::solid`]: the scene asks it each
/// tick with whether the event has happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lock {
    pub id: String,
    pub solid: IRect,
    pub opens_on: LockEvent,
}

impl Lock {
    /// The collision rectangle: the solid while closed, nothing once opened.
    pub const fn solid(&self, opened: bool) -> Option<IRect> {
        if opened { None } else { Some(self.solid) }
    }
}

/// What a [`StaticEntry`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticKind {
    /// A solid that stays where it is (HRD-A1: Hoarder bank A until HRD-B turns
    /// it into a prop). It stands on the lane and is at most
    /// [`MAX_LANE_OBSTACLE_PX`] high.
    Solid,
    /// A hit target that is not solid (HRD-A1: the J-B support post).
    HitTarget,
}

impl StaticKind {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "solid" => Self::Solid,
            "hit_target" => Self::HitTarget,
            _ => return None,
        })
    }
}

/// A rectangle of the sidecar that never changes state: the home of an
/// authored obstacle before its prop state machine exists. Layout pixels, y
/// down, standing on the walking line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticEntry {
    pub id: String,
    pub kind: StaticKind,
    pub rect: IRect,
}

// -------------------------------------------------------------- typed data

#[derive(Debug, Clone, PartialEq)]
pub struct Prop {
    pub id: String,
    pub kind: PropKind,
    /// Where a stimulus hits the prop.
    pub target: IRect,
    /// The first playtest may ship without it (the drum, P3).
    pub optional: bool,
    /// The prop blocks the lane while intact (a drum).
    pub solid_intact: bool,
    pub hang_px: Option<i32>,
    /// The low solid left after the prop is used up (the landed weight).
    pub remnant: Option<IRect>,
    pub blast_radius_px: Option<i32>,
    /// A valve's tunables with the timing of its steam hazard.
    pub valve: Option<ValveConfig>,
    /// A rot sticker's track (layout px, y down).
    pub track: Option<StickerLayout>,
    /// A shutter's knobs (RING-B core). `target` is its closed rectangle.
    pub shutter: Option<ShutterConfig>,
}

/// A rot sticker's track as authored in the sidecar (layout px, y down): the
/// surface it clings to, two end points on that surface and a start point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickerLayout {
    pub orientation: Orientation,
    /// The solid the track clings to (layout px). HAZ rule 3 requires real
    /// geometry: a ceiling surface equals a room platform's slab, or its top
    /// reaches the room's `ceiling_px`, in which case it is an authored
    /// **full-height soffit solid with no standable top** that HAZ-A2
    /// materialises (PR #204 review F1/F2). A wall surface is a room step or
    /// platform (no boundary-wall sticker in HAZ-A1).
    pub surface: IRect,
    pub a: (i32, i32),
    pub b: (i32, i32),
    pub start: (i32, i32),
}

impl StickerLayout {
    /// The domain track in world px, y up (heights above the walking line
    /// `floor_y`), as [`crate::rot_sticker`] works.
    pub fn world_track(&self, floor_y: i32) -> Result<Track, TrackError> {
        let up = |p: (i32, i32)| (p.0, floor_y - p.1);
        Track::from_points(self.orientation, up(self.a), up(self.b), up(self.start))
    }
}

/// A pocket of the room (HAZ rule 5): an x range with its own caps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pocket {
    pub id: String,
    pub x0: i32,
    pub x1: i32,
    pub max_stickers: u8,
    pub max_slimes: u8,
    pub max_goblins: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hazard {
    pub name: String,
    /// Index into [`Encounter::props`]; `None` for a volume no prop owns (an
    /// acid patch, a slide, a wheel).
    pub owner: Option<usize>,
    pub spec: HazardSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    pub from: usize,
    pub to: usize,
    pub kind: LinkKind,
}

/// What a roster entry is: a room enemy kind, or the HAZ slime (a slot enemy
/// that the room file's `EnemyKind` does not name).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterKind {
    Room(EnemyKind),
    Slime,
}

impl RosterKind {
    /// The room enemy kind, `None` for a slime.
    pub const fn room(self) -> Option<EnemyKind> {
        match self {
            Self::Room(k) => Some(k),
            Self::Slime => None,
        }
    }

    pub const fn is_slime(self) -> bool {
        matches!(self, Self::Slime)
    }

    /// Whether this kind supplies `CompanionView.boss` (HAZ rule 9). Appended to
    /// by the enemy sheets: today the slime does, and the Hoarder, the single
    /// supplier of her room (HRD sheet section 8).
    pub const fn supplies_boss(self) -> bool {
        matches!(self, Self::Slime | Self::Room(EnemyKind::Hoarder))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterEntry {
    pub id: String,
    pub kind: RosterKind,
    /// Slime only: the authored facing (+1 or -1), `1` for every other kind.
    pub facing: i8,
    /// Slime only: the leash column (its pool's centre or its spawn).
    pub home: i32,
    /// Slime only: the acid patch it guards (an `acid_patches` id).
    pub linked_pool: Option<String>,
    pub gate: Option<String>,
    pub fallback: Option<EnemyKind>,
    pub x: i32,
    /// Standing on this platform (an index of the room's platforms).
    pub perch: Option<usize>,
    pub beat: String,
    pub trigger_x: Option<i32>,
    pub delay_ticks: u32,
    /// Surfaces this enemy threatens besides its own perch.
    pub threatens: Vec<usize>,
    /// Names of the hazards this entry may spawn inside (an acid slime spawns
    /// in its own acid). Every other hazard still rejects it.
    pub allow_in_hazard: Vec<String>,
    /// A lock may not wait on an optional enemy.
    pub optional: bool,
}

/// Reachability of one hero class in one destruction state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReachRow {
    pub hero: ReachHero,
    /// Bit k set: the k-th changeable prop is used up.
    pub state: u16,
    /// The exit is reachable from the hero's spawn.
    pub exit: bool,
    /// The ground lane alone (no perch) leads from the spawn to the exit.
    pub lane: bool,
    /// The two spawns are on the same open stretch of the lane.
    pub regroup: bool,
    /// Per platform `(reachable, can_return)` for this class, whatever the
    /// surface's own height: a class reaches a high perch through a lower one
    /// (stacked routes).
    pub perches: Vec<(bool, bool)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Encounter {
    pub id: String,
    pub room_id: String,
    pub story_pin: String,
    pub goal: Goal,
    pub max_living: u8,
    pub platform_slab_px: i32,
    pub protected_spawn: IRect,
    pub checkpoint: IRect,
    pub exit: IRect,
    pub props: Vec<Prop>,
    pub hazards: Vec<Hazard>,
    pub links: Vec<Link>,
    pub roster: Vec<RosterEntry>,
    /// Gate class per platform, in the room's platform order.
    pub gates: Vec<GateClass>,
    pub vantages: Vec<(usize, VantageCounter)>,
    /// Solids that open on an event (the "solid toggled by event" hook).
    pub locks: Vec<Lock>,
    /// Authored rectangles that never change state (HRD-A1: bank A and the J-B
    /// support post). Empty for a sidecar without the field.
    pub statics: Vec<StaticEntry>,
    /// The weakest class that reaches each platform from its spawn in any
    /// destruction state (the effective gate class).
    pub effective_gates: Vec<GateClass>,
    /// The reach matrix the validation computed.
    pub reach: Vec<ReachRow>,
    /// HAZ-A1: the acid patches (never in `hazards`, so `RosterInHazard` does
    /// not see them).
    pub acid_patches: Vec<AcidPatchSpec>,
    /// HAZ-A1: the pockets with their caps (empty when the sidecar has none).
    pub pockets: Vec<Pocket>,
}

impl Encounter {
    pub fn prop_index(&self, id: &str) -> Option<usize> {
        self.props.iter().position(|p| p.id == id)
    }

    /// Fresh hazard volumes of the encounter, in id order.
    pub fn hazard_set(&self) -> Result<HazardSet, HazardSetError> {
        HazardSet::new(self.hazards.iter().map(|h| h.spec.clone()).collect())
    }

    /// The hazard of `kind` owned by prop `owner`.
    pub fn owned_hazard(&self, owner: usize, class: HazardClass) -> Option<&Hazard> {
        self.hazards
            .iter()
            .find(|h| h.owner == Some(owner) && h.spec.class == class)
    }

    /// The lock solids at this moment: `opened[i]` says whether lock `i`'s
    /// event has happened. The scene's "solid toggled by event" hook.
    pub fn lock_solids(&self, opened: &[bool]) -> Vec<IRect> {
        self.locks
            .iter()
            .enumerate()
            .filter_map(|(i, l)| l.solid(opened.get(i).copied().unwrap_or(false)))
            .collect()
    }

    /// The locks that open when roster entry `index` is defeated.
    pub fn locks_opened_by(&self, index: usize) -> Vec<usize> {
        self.locks
            .iter()
            .enumerate()
            .filter(|(_, l)| l.opens_on == LockEvent::EnemyDefeated { roster: index })
            .map(|(i, _)| i)
            .collect()
    }

    /// The living-enemy count of the roster with every gate closed.
    pub fn roster_size(&self) -> usize {
        self.roster.len()
    }
}

/// A loaded encounter with the file hashes (FNV-1a 64) that pin the data.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub room: Room,
    pub encounter: Encounter,
    pub classes: HeightClasses,
    pub ceiling_px: Option<i32>,
    pub room_hash: u64,
    pub sidecar_hash: u64,
}

/// Which side of a closed shutter a route check starts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutterSide {
    Left,
    Right,
}

impl ShutterSide {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

// ------------------------------------------------------------------- errors

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncounterError {
    Io(String),
    Parse(String),
    Schema(String),
    RoomNotFound(String),
    ClassesInvalid(String),
    /// DP-Q45: a surface above the free height that the boosted Donut stands
    /// on, in a room with no `ceiling_px`.
    CeilingMissing {
        surface: String,
        rise: i32,
    },
    /// DP-Q45: the room's `ceiling_px` is over the rule's maximum (or not
    /// positive).
    CeilingTooHigh {
        ceiling_px: i32,
        max: i32,
    },
    BadId(String),
    DuplicateId(String),
    UnknownGoal(String),
    /// A room never grants an ability.
    GrantsNotEmpty,
    TooManyProps(usize),
    UnknownPropKind(String),
    PropFields {
        prop: String,
        reason: String,
    },
    PropOutOfBounds(String),
    PropOutOfReach(String),
    PropOnProtected {
        prop: String,
        zone: &'static str,
    },
    HangTooLow {
        prop: String,
        px: i32,
    },
    ObstacleTooTall {
        prop: String,
        px: i32,
    },
    ValveConfig {
        prop: String,
        reason: String,
    },
    /// PR #200 review NB-2: a hero lifted onto a landed remnant would have its
    /// head in a slab or the lane ceiling.
    RemnantHeadroom {
        prop: String,
        over: String,
        clearance: i32,
        need: i32,
    },
    /// RING-B core: a shutter knob out of its bounds (KR2, KR19).
    ShutterConfig {
        prop: String,
        reason: String,
    },
    /// KR3: a closed shutter's top must equal a platform's slab underside and
    /// its x range must lie inside that platform's.
    ShutterNotUnderPlatform(String),
    /// KR2/KR3: the closed shutter's bottom is the walking line.
    ShutterNotOnFloor(String),
    /// KR3: nothing else may overlap the closed shutter's rectangle.
    ShutterOverlaps {
        prop: String,
        with: String,
    },
    /// KR3: a floor node on each side of the closed shutter.
    ShutterNoFloorNode {
        prop: String,
        hero: ReachHero,
        state: u16,
        side: ShutterSide,
    },
    /// KR3 clause 4: in a state with the shutter closed, the exit is not
    /// reachable from this side's floor node without crossing it.
    ShutterBypass {
        prop: String,
        hero: ReachHero,
        state: u16,
        side: ShutterSide,
    },
    /// A shutter takes no link in v1 (it hurts nobody and has no contact).
    LinkShutter(String),
    HazardClass(String),
    HazardPolicy(String),
    HazardUnknownOwner {
        hazard: String,
        owner: String,
    },
    HazardSpec {
        hazard: String,
        reason: String,
    },
    HazardOutOfBounds(String),
    HazardOnProtected {
        hazard: String,
        zone: &'static str,
    },
    HazardOwnerMismatch {
        prop: String,
        reason: String,
    },
    /// Timed fields on a persistent hazard, or missing on a timed one.
    HazardWindow(String),
    /// F1/K18: a room file of a setpiece keeps its geometry arrays empty.
    RoomEntitiesNotEmpty {
        array: &'static str,
    },
    /// D3: a weaker class reaches a surface through a lower perch, so the
    /// declared gate does not hold.
    GateBypassed {
        surface: usize,
        declared: &'static str,
        by: &'static str,
    },
    /// F7: the `threatens` of a roster entry does not reach the surface.
    ThreatNotReachable {
        entry: String,
        surface: usize,
    },
    /// F7: a vantage counter with nothing behind it in the data.
    VantageUnbacked {
        surface: usize,
        counter: &'static str,
    },
    LockUnknownEvent(String),
    LockUnknownTarget {
        lock: String,
        target: String,
    },
    LockOptionalTarget {
        lock: String,
        target: String,
    },
    LockOnProtected {
        lock: String,
        zone: &'static str,
    },
    LockOutOfBounds(String),
    /// A static entry is unknown, misplaced or too tall.
    StaticInvalid {
        id: String,
        reason: String,
    },
    /// The closed lock walls a hero off from the enemy it waits on.
    LockTargetUnreachable {
        lock: String,
        hero: ReachHero,
        state: u16,
    },
    /// The closed lock walls a hero off from the column that wakes its target:
    /// the target never spawns and the lock never opens.
    LockTriggerUnreachable {
        lock: String,
        hero: ReachHero,
        state: u16,
    },
    TooManyLinks(usize),
    LinkUnknownProp(String),
    LinkSelf(String),
    LinkDuplicate {
        from: String,
        to: String,
    },
    LinkCycle(Vec<String>),
    LinkNoContact {
        from: String,
        to: String,
    },
    UnknownLinkKind(String),
    RosterKind {
        entry: String,
        kind: String,
    },
    UnknownGate {
        entry: String,
        gate: String,
    },
    RosterFallback {
        entry: String,
    },
    RosterOutOfBounds(String),
    RosterOnProtected(String),
    /// `allow_in_hazard` names a hazard that does not exist.
    RosterUnknownHazard {
        entry: String,
        hazard: String,
    },
    RosterInHazard {
        entry: String,
        hazard: String,
    },
    RosterPerch {
        entry: String,
        perch: usize,
    },
    SynchronizedSpawn {
        a: String,
        b: String,
    },
    TooManyLiving {
        roster: usize,
        max: u8,
    },
    MaxLivingOverCap(u8),
    ExitMismatch,
    ExitNotOnLane,
    CheckpointInvalid(String),
    SpawnNotProtected(&'static str),
    GateMissing(usize),
    GateUnknownSurface(usize),
    GateClassName(String),
    GateMismatch {
        surface: usize,
        declared: &'static str,
        actual: &'static str,
    },
    /// A surface above every class: nobody lands on it.
    SurfaceUnreachable {
        surface: usize,
        rise: i32,
    },
    /// The slab leaves less than Carl's height under it.
    PlatformBlocksLane {
        surface: usize,
        clearance: i32,
    },
    /// K8: a 81..=128 px surface that no enemy threatens and nothing counters.
    UnthreatenedNest {
        surface: usize,
        rise: i32,
    },
    VantageUnknownSurface(usize),
    VantageCounter(String),
    SpawnBlocked(ReachHero),
    LaneCut {
        hero: ReachHero,
        state: u16,
    },
    ExitUnreachable {
        hero: ReachHero,
        state: u16,
    },
    RegroupBlocked {
        hero: ReachHero,
        state: u16,
    },
    PerchUnreachable {
        hero: ReachHero,
        state: u16,
        surface: usize,
    },
    PerchNoReturn {
        hero: ReachHero,
        state: u16,
        surface: usize,
    },
    /// HAZ-A1: a failure of one of the hazard room's validator rules
    /// (`docs/01-design/enemy-gameplay-slime-rot.md` section 15). Names the
    /// rule (0 for a schema problem of the HAZ appends) and the ids.
    Haz {
        rule: u8,
        ids: Vec<String>,
        reason: String,
    },
}

impl std::fmt::Display for EncounterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use EncounterError::*;
        match self {
            Io(e) | Parse(e) | Schema(e) | ClassesInvalid(e) => write!(f, "{e}"),
            CeilingMissing { surface, rise } => write!(
                f,
                "{surface} ({rise} px) is above the free height the boosted Donut may stand on without a lane ceiling: declare ceiling_px (DP-Q45)"
            ),
            CeilingTooHigh { ceiling_px, max } => {
                write!(
                    f,
                    "ceiling_px {ceiling_px} must be between 1 and {max} (DP-Q45)"
                )
            }
            RoomNotFound(id) => write!(f, "the room file has no scenario {id}"),
            BadId(id) => write!(f, "id {id:?} must be lowercase ascii letters, digits and _"),
            DuplicateId(id) => write!(f, "duplicate id {id}"),
            UnknownGoal(g) => write!(f, "unknown goal {g:?} (reach_exit or defeat_roster)"),
            GrantsNotEmpty => write!(f, "a room never grants a capability: grants must be empty"),
            TooManyProps(n) => write!(f, "{n} props, at most {MAX_PROPS}"),
            UnknownPropKind(k) => write!(f, "unknown prop kind {k:?}"),
            PropFields { prop, reason } => write!(f, "prop {prop}: {reason}"),
            PropOutOfBounds(p) => write!(f, "prop {p} is outside the play area or under the floor"),
            PropOutOfReach(p) => write!(f, "prop {p} has its target above Carl's height class"),
            PropOnProtected { prop, zone } => {
                write!(f, "prop {prop} overlaps the protected {zone}")
            }
            HangTooLow { prop, px } => write!(
                f,
                "prop {prop} hangs {px} px above the lane, at least {MIN_HANG_PX} px"
            ),
            ObstacleTooTall { prop, px } => write!(
                f,
                "prop {prop} leaves a {px} px solid on the lane, at most {MAX_LANE_OBSTACLE_PX} px"
            ),
            ValveConfig { prop, reason } => write!(f, "prop {prop}: valve config: {reason}"),
            RemnantHeadroom {
                prop,
                over,
                clearance,
                need,
            } => write!(
                f,
                "prop {prop}: only {clearance} px between its remnant and {over}, a hero lifted onto the remnant needs {need}"
            ),
            ShutterConfig { prop, reason } => write!(f, "prop {prop}: shutter config: {reason}"),
            ShutterNotUnderPlatform(p) => write!(
                f,
                "shutter {p} must sit under a platform: its top equals the platform's slab underside and it lies inside the platform's x range (KR3)"
            ),
            ShutterNotOnFloor(p) => {
                write!(f, "shutter {p} must stand on the walking line (KR2, KR3)")
            }
            ShutterOverlaps { prop, with } => {
                write!(f, "shutter {prop} (closed) overlaps {with} (KR3)")
            }
            ShutterNoFloorNode {
                prop,
                hero,
                state,
                side,
            } => write!(
                f,
                "shutter {prop} has no floor node on its {} side for {} in state {state:#b} (KR3)",
                side.name(),
                hero.name()
            ),
            ShutterBypass {
                prop,
                hero,
                state,
                side,
            } => write!(
                f,
                "from the {} side of closed shutter {prop} {} cannot reach the exit in state {state:#b}: no real crossing (KR3)",
                side.name(),
                hero.name()
            ),
            LinkShutter(p) => write!(f, "link names the shutter {p}: a shutter takes no link"),
            HazardClass(c) => write!(f, "unknown hazard class {c:?}"),
            HazardPolicy(p) => write!(f, "unknown team policy {p:?}"),
            HazardUnknownOwner { hazard, owner } => {
                write!(f, "hazard {hazard}: unknown owner prop {owner}")
            }
            HazardSpec { hazard, reason } => write!(f, "hazard {hazard}: {reason}"),
            HazardOutOfBounds(h) => write!(f, "hazard {h} is outside the play area"),
            HazardOnProtected { hazard, zone } => {
                write!(f, "hazard {hazard} overlaps the protected {zone}")
            }
            HazardOwnerMismatch { prop, reason } => write!(f, "prop {prop}: {reason}"),
            HazardWindow(r) => write!(f, "hazard window: {r}"),
            RoomEntitiesNotEmpty { array } => write!(
                f,
                "the room file's {array} array must stay empty: encounter data lives in the sidecar"
            ),
            Haz { rule, ids, reason } => {
                write!(f, "HAZ rule {rule} [{}]: {reason}", ids.join(", "))
            }
            GateBypassed {
                surface,
                declared,
                by,
            } => write!(
                f,
                "platform {surface} is declared {declared} but {by} reaches it through a lower perch: the gate is bypassed"
            ),
            ThreatNotReachable { entry, surface } => write!(
                f,
                "roster {entry} claims to threaten platform {surface} but its attack cannot reach it"
            ),
            VantageUnbacked { surface, counter } => write!(
                f,
                "platform {surface}: the vantage counter {counter} has nothing behind it in the data"
            ),
            LockUnknownEvent(e) => write!(f, "unknown lock event {e:?}"),
            LockUnknownTarget { lock, target } => {
                write!(f, "lock {lock} waits on the unknown roster entry {target}")
            }
            LockOptionalTarget { lock, target } => {
                write!(f, "lock {lock} waits on the optional roster entry {target}")
            }
            LockOnProtected { lock, zone } => {
                write!(f, "lock {lock} overlaps the protected {zone}")
            }
            LockOutOfBounds(l) => write!(f, "lock {l} is outside the play area or off the lane"),
            StaticInvalid { id, reason } => write!(f, "static {id}: {reason}"),
            LockTargetUnreachable { lock, hero, state } => write!(
                f,
                "lock {lock} (closed) walls {} off from its target in state {state:#b}",
                hero.name()
            ),
            LockTriggerUnreachable { lock, hero, state } => write!(
                f,
                "lock {lock} (closed) walls {} off from the trigger of its target in state {state:#b}: softlock",
                hero.name()
            ),
            TooManyLinks(n) => write!(f, "{n} links, at most {MAX_AUTHORED_LINKS}"),
            LinkUnknownProp(p) => write!(f, "link names the unknown prop {p}"),
            LinkSelf(p) => write!(f, "prop {p} links to itself"),
            LinkDuplicate { from, to } => write!(f, "duplicate link {from} -> {to}"),
            LinkCycle(path) => write!(f, "the links form a cycle through {}", path.join(" -> ")),
            LinkNoContact { from, to } => write!(
                f,
                "link {from} -> {to} has no physical contact: nothing of {from} reaches {to}"
            ),
            UnknownLinkKind(k) => write!(f, "unknown link kind {k:?}"),
            RosterKind { entry, kind } => write!(f, "roster {entry}: kind {kind:?} is not allowed"),
            UnknownGate { entry, gate } => write!(f, "roster {entry}: unknown gate {gate:?}"),
            RosterFallback { entry } => write!(
                f,
                "roster {entry}: a gated entry needs a goblin fallback, an ungated one has none"
            ),
            RosterOutOfBounds(e) => write!(f, "roster {e} is outside the play area"),
            RosterOnProtected(e) => write!(f, "roster {e} spawns on the protected spawn"),
            RosterUnknownHazard { entry, hazard } => write!(
                f,
                "roster {entry}: allow_in_hazard names the unknown hazard {hazard}"
            ),
            RosterInHazard { entry, hazard } => {
                write!(f, "roster {entry} spawns inside hazard {hazard}")
            }
            RosterPerch { entry, perch } => {
                write!(f, "roster {entry}: platform {perch} does not exist")
            }
            SynchronizedSpawn { a, b } => {
                write!(f, "roster {a} and {b} spawn on the same trigger and tick")
            }
            TooManyLiving { roster, max } => {
                write!(f, "{roster} roster entries, at most {max} living")
            }
            MaxLivingOverCap(n) => {
                write!(
                    f,
                    "max_living {n}, at most {MAX_LIVING_FIRST_PROBE} in the first probe"
                )
            }
            ExitMismatch => write!(f, "the exit is not the room's protected exit"),
            ExitNotOnLane => write!(f, "the exit does not stand on the walking line"),
            CheckpointInvalid(r) => write!(f, "checkpoint: {r}"),
            SpawnNotProtected(who) => write!(f, "the {who} spawn is outside the protected spawn"),
            GateMissing(s) => write!(f, "platform {s} has no height gate class"),
            GateUnknownSurface(s) => write!(f, "gate names the unknown platform {s}"),
            GateClassName(c) => write!(f, "unknown gate class {c:?}"),
            GateMismatch {
                surface,
                declared,
                actual,
            } => write!(
                f,
                "platform {surface} is declared {declared} but its height is {actual} class"
            ),
            SurfaceUnreachable { surface, rise } => {
                write!(
                    f,
                    "platform {surface} is {rise} px up, above every height class"
                )
            }
            PlatformBlocksLane { surface, clearance } => write!(
                f,
                "platform {surface} leaves {clearance} px under it, less than Carl's height"
            ),
            UnthreatenedNest { surface, rise } => write!(
                f,
                "platform {surface} ({rise} px) is a nest: no enemy threatens it and it is not a declared vantage with a counter"
            ),
            VantageUnknownSurface(s) => write!(f, "vantage names the unknown platform {s}"),
            VantageCounter(c) => write!(f, "unknown vantage counter {c:?}"),
            SpawnBlocked(h) => write!(f, "the {} spawn is inside a blocker", h.name()),
            LaneCut { hero, state } => write!(
                f,
                "the ground lane is cut for {} in state {state:#b}",
                hero.name()
            ),
            ExitUnreachable { hero, state } => write!(
                f,
                "{} cannot reach the exit in state {state:#b}",
                hero.name()
            ),
            RegroupBlocked { hero, state } => write!(
                f,
                "the spawns do not regroup for {} in state {state:#b}",
                hero.name()
            ),
            PerchUnreachable {
                hero,
                state,
                surface,
            } => write!(
                f,
                "{} cannot reach platform {surface} in state {state:#b}",
                hero.name()
            ),
            PerchNoReturn {
                hero,
                state,
                surface,
            } => write!(
                f,
                "{} cannot return from platform {surface} in state {state:#b}",
                hero.name()
            ),
        }
    }
}

// ---------------------------------------------------------------- raw data

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl From<&RawRect> for IRect {
    fn from(r: &RawRect) -> Self {
        IRect::new(r.x, r.y, r.w, r.h)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawValve {
    trigger_weight: Option<u8>,
    launch_min_impulse_px_s: Option<u16>,
    launch_weight: Option<u8>,
    blast_weight: Option<u8>,
    contact_weight: Option<u8>,
    projectile_weight: Option<u8>,
    shove_weight: Option<u8>,
    shove_min_impulse_px_s: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawShutter {
    initial: String,
    arm_x: Option<i32>,
    arm_trail_px: Option<i32>,
    warning_ticks: Option<u16>,
    speed_px_per_tick: Option<u8>,
    slice_px: Option<u8>,
    cycles: Option<u8>,
    pin_ticks: Option<u16>,
    max_closed_ticks: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProp {
    id: String,
    kind: String,
    target: RawRect,
    #[serde(default)]
    optional: bool,
    #[serde(default)]
    solid_intact: bool,
    hang_px: Option<i32>,
    remnant: Option<RawRect>,
    blast_radius_px: Option<i32>,
    valve: Option<RawValve>,
    track: Option<RawTrack>,
    shutter: Option<RawShutter>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPoint {
    x: i32,
    y: i32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTrack {
    orientation: String,
    surface: RawRect,
    a: RawPoint,
    b: RawPoint,
    start: RawPoint,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAcidPatch {
    id: u16,
    x0: i32,
    x1: i32,
    floor_y: i32,
    linked_slime: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPocket {
    id: String,
    x0: i32,
    x1: i32,
    max_stickers: u8,
    max_slimes: u8,
    max_goblins: u8,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHazard {
    id: u16,
    name: String,
    class: String,
    owner: Option<String>,
    rect: RawRect,
    telegraph_rect: Option<RawRect>,
    #[serde(default)]
    persistent: bool,
    telegraph_ticks: Option<u16>,
    active_ticks: Option<u16>,
    max_activations: Option<u8>,
    damage: u8,
    push_x: i16,
    hero_damage: String,
    hurts_enemies: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLink {
    from: String,
    to: String,
    kind: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRoster {
    id: String,
    kind: String,
    gate: Option<String>,
    fallback_kind: Option<String>,
    x: i32,
    perch: Option<usize>,
    beat: String,
    trigger_x: Option<i32>,
    #[serde(default)]
    delay_ticks: u32,
    #[serde(default)]
    threatens: Vec<usize>,
    #[serde(default)]
    allow_in_hazard: Vec<String>,
    #[serde(default)]
    optional: bool,
    facing: Option<i8>,
    home: Option<i32>,
    linked_pool: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOpensOn {
    event: String,
    target: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLock {
    id: String,
    solid: RawRect,
    opens_on: RawOpensOn,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStatic {
    id: String,
    kind: String,
    rect: RawRect,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGate {
    surface: usize,
    class: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVantage {
    surface: usize,
    counter: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEncounter {
    schema: String,
    #[allow(dead_code)]
    status: String,
    id: String,
    room_file: String,
    room_id: String,
    story_pin: String,
    goal: String,
    grants: Vec<String>,
    max_living: u8,
    locks: Vec<RawLock>,
    platform_slab_px: i32,
    protected_spawn: RawRect,
    checkpoint: RawRect,
    exit: RawRect,
    props: Vec<RawProp>,
    hazards: Vec<RawHazard>,
    links: Vec<RawLink>,
    roster: Vec<RawRoster>,
    gates: Vec<RawGate>,
    vantages: Vec<RawVantage>,
    #[serde(default)]
    acid_patches: Vec<RawAcidPatch>,
    #[serde(default)]
    pockets: Vec<RawPocket>,
    /// HRD-A1, optional: absent means none (every earlier sidecar).
    #[serde(default)]
    statics: Vec<RawStatic>,
    /// PT-MASTERY, optional: absent means false (every earlier sidecar). A wave
    /// room bounds its roster per beat ([`MAX_ROSTER_WAVES`]).
    #[serde(default)]
    waves: bool,
}

// ------------------------------------------------------------------ loading

/// FNV-1a 64 of a file's bytes (a local copy: the story-side code must not
/// reference the combat lab modules).
fn fnv64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn is_id(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Read a sidecar (a path relative to `root`) and the room file it names (next
/// to it), parse both and validate. Every problem is returned.
pub fn load(root: &Path, sidecar: &str) -> Result<Loaded, Vec<EncounterError>> {
    let sidecar_path = root.join(sidecar);
    let text = std::fs::read_to_string(&sidecar_path).map_err(|e| {
        vec![EncounterError::Io(format!(
            "{}: {e}",
            sidecar_path.display()
        ))]
    })?;
    let raw: RawEncounter = serde_json::from_str(&text)
        .map_err(|e| vec![EncounterError::Parse(format!("{sidecar}: {e}"))])?;
    let room_path = sidecar_path.parent().unwrap_or(root).join(&raw.room_file);
    let room_text = std::fs::read_to_string(&room_path)
        .map_err(|e| vec![EncounterError::Io(format!("{}: {e}", room_path.display()))])?;
    let rooms = parse_rooms(&room_text).map_err(|e| vec![EncounterError::Parse(e)])?;
    let room = rooms
        .into_iter()
        .find(|r| r.id == raw.room_id)
        .ok_or_else(|| vec![EncounterError::RoomNotFound(raw.room_id.clone())])?;
    let ceiling_px = room_ceiling_px(&room_text, &raw.room_id);
    let classes = HeightClasses::load(root).map_err(|e| vec![e])?;
    let ctx = Context {
        room,
        classes,
        ceiling_px,
    };
    let encounter = validate(&raw, &ctx)?;
    Ok(Loaded {
        room: ctx.room,
        encounter,
        classes,
        ceiling_px,
        room_hash: fnv64(room_text.as_bytes()),
        sidecar_hash: fnv64(text.as_bytes()),
    })
}

/// The optional `ceiling_px` of scenario `room_id` (DM-Q27: absent = none).
/// Read from the room text because the room parser of this branch predates
/// the field.
pub fn room_ceiling_px(room_text: &str, room_id: &str) -> Option<i32> {
    let root: serde_json::Value = serde_json::from_str(room_text).ok()?;
    root["scenarios"]
        .as_array()?
        .iter()
        .find(|s| s["id"].as_str() == Some(room_id))?["ceiling_px"]
        .as_i64()
        .and_then(|n| i32::try_from(n).ok())
}

/// Parse sidecar text against an already parsed context and validate it.
pub fn parse_sidecar(text: &str, ctx: &Context) -> Result<Encounter, Vec<EncounterError>> {
    let raw: RawEncounter =
        serde_json::from_str(text).map_err(|e| vec![EncounterError::Parse(e.to_string())])?;
    validate(&raw, ctx)
}

// --------------------------------------------------------------- validation

/// Kinds whose attack travels (a spit line), so they threaten a surface from
/// the lane. Appended to by the enemy sheets.
fn kind_is_ranged(kind: EnemyKind) -> bool {
    matches!(kind, EnemyKind::Llama)
}

fn overlaps(a: IRect, b: IRect) -> bool {
    a.overlaps(b)
}

fn inflate(r: IRect, by: i32) -> IRect {
    IRect::new(r.x - by, r.y - by, r.w + 2 * by, r.h + 2 * by)
}

/// Squared distance from a point to a rectangle (0 inside).
fn dist2_point_rect(px: i32, py: i32, r: IRect) -> i64 {
    let dx = (r.x - px).max(px - r.right()).max(0) as i64;
    let dy = (r.y - py).max(py - r.bottom()).max(0) as i64;
    dx * dx + dy * dy
}

fn within_play(room: &Room, r: IRect) -> bool {
    r.w > 0
        && r.h > 0
        && r.x >= room.play_x.0
        && r.right() <= room.play_x.1
        && r.bottom() <= room.floor_y
        && r.y >= 0
}

fn validate(raw: &RawEncounter, ctx: &Context) -> Result<Encounter, Vec<EncounterError>> {
    use EncounterError as E;
    let room = &ctx.room;
    let classes = &ctx.classes;
    let carl_rise = classes.max_rise(ReachHero::Carl);
    let mut errs: Vec<E> = Vec::new();

    if raw.schema != SCHEMA {
        errs.push(E::Schema(format!(
            "schema {:?}, expected {SCHEMA:?}",
            raw.schema
        )));
    }
    for id in [&raw.id, &raw.story_pin] {
        if !is_id(&id.to_ascii_lowercase().replace('-', "_")) {
            errs.push(E::BadId(id.clone()));
        }
    }
    if raw.room_id != room.id {
        errs.push(E::RoomNotFound(raw.room_id.clone()));
    }
    let goal = Goal::parse(&raw.goal).unwrap_or_else(|| {
        errs.push(E::UnknownGoal(raw.goal.clone()));
        Goal::ReachExit
    });
    // K18: the room file keeps its geometry arrays empty.
    for (array, len) in [
        ("props", room.props.len()),
        ("enemies", room.enemies.len()),
        ("hazards", room.hazards.len()),
    ] {
        if len > 0 {
            errs.push(E::RoomEntitiesNotEmpty { array });
        }
    }
    if !raw.grants.is_empty() {
        errs.push(E::GrantsNotEmpty);
    }
    if raw.max_living > MAX_LIVING_FIRST_PROBE {
        errs.push(E::MaxLivingOverCap(raw.max_living));
    }

    // ---- protected zones
    let protected_spawn = IRect::from(&raw.protected_spawn);
    let checkpoint = IRect::from(&raw.checkpoint);
    let exit = IRect::from(&raw.exit);
    if exit != room.exit {
        errs.push(E::ExitMismatch);
    }
    if exit.bottom() != room.floor_y {
        errs.push(E::ExitNotOnLane);
    }
    for (who, x) in [("carl", room.spawn_carl), ("donut", room.spawn_donut)] {
        if x < protected_spawn.x || x >= protected_spawn.right() {
            errs.push(E::SpawnNotProtected(who));
        }
    }
    if !within_play(room, protected_spawn) || protected_spawn.bottom() != room.floor_y {
        errs.push(E::SpawnNotProtected("protected rectangle"));
    }
    if !within_play(room, checkpoint) || checkpoint.bottom() != room.floor_y {
        errs.push(E::CheckpointInvalid(
            "must stand on the walking line inside the play area".into(),
        ));
    }

    // ---- props
    if raw.props.len() > MAX_PROPS {
        errs.push(E::TooManyProps(raw.props.len()));
    }
    let mut props: Vec<Prop> = Vec::new();
    let mut seen_ids: BTreeSet<&str> = BTreeSet::new();
    for p in &raw.props {
        if !is_id(&p.id) {
            errs.push(E::BadId(p.id.clone()));
        }
        if !seen_ids.insert(p.id.as_str()) {
            errs.push(E::DuplicateId(p.id.clone()));
        }
        let Some(kind) = PropKind::parse(&p.kind) else {
            errs.push(E::UnknownPropKind(p.kind.clone()));
            continue;
        };
        let target = IRect::from(&p.target);
        let remnant = p.remnant.as_ref().map(IRect::from);
        if !within_play(room, target) {
            errs.push(E::PropOutOfBounds(p.id.clone()));
        } else if kind != PropKind::RotSticker && room.floor_y - target.y > carl_rise {
            // A sticker is provoked at range (a trigger circle, a bolt): it is
            // never a target Carl must reach.
            errs.push(E::PropOutOfReach(p.id.clone()));
        }
        let field = |reason: &str| E::PropFields {
            prop: p.id.clone(),
            reason: reason.into(),
        };
        if kind != PropKind::Shutter && p.shutter.is_some() {
            errs.push(field("only a shutter has a shutter object"));
        }
        match kind {
            PropKind::Shutter => {
                if p.hang_px.is_some()
                    || remnant.is_some()
                    || p.blast_radius_px.is_some()
                    || p.valve.is_some()
                {
                    errs.push(field(
                        "a shutter has no hang_px, remnant, blast_radius_px or valve",
                    ));
                }
                if p.solid_intact {
                    errs.push(field(
                        "a shutter has no solid_intact: its solidity is its state (KR3)",
                    ));
                }
                if p.shutter.is_none() {
                    errs.push(field("a shutter needs a shutter object"));
                }
            }
            PropKind::Valve => {
                if p.hang_px.is_some() || remnant.is_some() || p.blast_radius_px.is_some() {
                    errs.push(field("a valve has no hang_px, remnant or blast_radius_px"));
                }
            }
            PropKind::WeightRig => {
                if p.valve.is_some() || p.blast_radius_px.is_some() {
                    errs.push(field("a weight_rig has no valve or blast_radius_px"));
                }
                match p.hang_px {
                    Some(px) if px < MIN_HANG_PX => errs.push(E::HangTooLow {
                        prop: p.id.clone(),
                        px,
                    }),
                    Some(_) => {}
                    None => errs.push(field("a weight_rig needs hang_px")),
                }
                match remnant {
                    Some(r) => {
                        if !within_play(room, r) || r.bottom() != room.floor_y {
                            errs.push(E::PropOutOfBounds(p.id.clone()));
                        }
                        if r.h > MAX_LANE_OBSTACLE_PX {
                            errs.push(E::ObstacleTooTall {
                                prop: p.id.clone(),
                                px: r.h,
                            });
                        }
                    }
                    None => errs.push(field("a weight_rig needs a low remnant")),
                }
            }
            PropKind::Drum => {
                if p.valve.is_some() || p.hang_px.is_some() {
                    errs.push(field("a drum has no valve or hang_px"));
                }
                match p.blast_radius_px {
                    Some(r) if r > 0 => {}
                    _ => errs.push(field("a drum needs a positive blast_radius_px")),
                }
            }
            PropKind::RotSticker => {
                if p.valve.is_some()
                    || p.hang_px.is_some()
                    || remnant.is_some()
                    || p.blast_radius_px.is_some()
                    || p.solid_intact
                {
                    errs.push(field(
                        "a rot_sticker has no valve, hang_px, remnant, blast_radius_px or solid_intact",
                    ));
                }
                if p.track.is_none() {
                    errs.push(field("a rot_sticker needs a track"));
                }
            }
        }
        if kind != PropKind::RotSticker && p.track.is_some() {
            errs.push(field("only a rot_sticker has a track"));
        }
        let track = p
            .track
            .as_ref()
            .and_then(|t| match Orientation::parse(&t.orientation) {
                Some(orientation) => Some(StickerLayout {
                    orientation,
                    surface: IRect::from(&t.surface),
                    a: (t.a.x, t.a.y),
                    b: (t.b.x, t.b.y),
                    start: (t.start.x, t.start.y),
                }),
                None => {
                    errs.push(E::Haz {
                        rule: 3,
                        ids: vec![p.id.clone()],
                        reason: format!("unknown track orientation {:?}", t.orientation),
                    });
                    None
                }
            });
        if p.solid_intact && target.h > MAX_LANE_OBSTACLE_PX {
            errs.push(E::ObstacleTooTall {
                prop: p.id.clone(),
                px: target.h,
            });
        }
        for (zone, rect) in [
            ("spawn", protected_spawn),
            ("exit", exit),
            ("checkpoint", checkpoint),
        ] {
            // A closed shutter may never seal the spawn, the checkpoint or the
            // exit (#189: the gate cannot seal checkpoint or exit).
            let blocks = (p.solid_intact && overlaps(target, rect))
                || (kind == PropKind::Shutter && overlaps(target, rect))
                || remnant.is_some_and(|r| overlaps(r, rect));
            // A hit target on the protected spawn is also wrong: the first
            // seconds of the room stay free of the machine.
            if blocks || (zone == "spawn" && overlaps(target, rect)) {
                errs.push(E::PropOnProtected {
                    prop: p.id.clone(),
                    zone,
                });
            }
        }
        props.push(Prop {
            id: p.id.clone(),
            kind,
            target,
            optional: p.optional,
            solid_intact: p.solid_intact,
            hang_px: p.hang_px,
            remnant,
            blast_radius_px: p.blast_radius_px,
            valve: None,
            track,
            shutter: None,
        });
    }
    let prop_pos: BTreeMap<String, usize> = props
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.clone(), i))
        .collect();

    // ---- hazards
    let mut hazards: Vec<Hazard> = Vec::new();
    let mut hazard_ids: BTreeSet<u16> = BTreeSet::new();
    for h in &raw.hazards {
        if !hazard_ids.insert(h.id) {
            errs.push(E::DuplicateId(format!("hazard {}", h.id)));
        }
        let Some(class) = HazardClass::parse(&h.class) else {
            errs.push(E::HazardClass(h.class.clone()));
            continue;
        };
        let Some(hero_damage) = HeroDamagePolicy::parse(&h.hero_damage) else {
            errs.push(E::HazardPolicy(h.hero_damage.clone()));
            continue;
        };
        // A hazard may have no prop owner (acid, a slide, a wheel).
        let owner = match &h.owner {
            None => None,
            Some(name) => match prop_pos.get(name.as_str()) {
                Some(&i) => Some(i),
                None => {
                    errs.push(E::HazardUnknownOwner {
                        hazard: h.name.clone(),
                        owner: name.clone(),
                    });
                    continue;
                }
            },
        };
        let window = match (
            h.persistent,
            h.telegraph_ticks,
            h.active_ticks,
            h.max_activations,
        ) {
            (true, None, None, None) => HazardWindow::Persistent,
            (false, Some(telegraph_ticks), Some(active_ticks), Some(max_activations)) => {
                HazardWindow::Timed {
                    telegraph_ticks,
                    active_ticks,
                    max_activations,
                }
            }
            (true, ..) => {
                errs.push(E::HazardWindow(format!(
                    "{}: a persistent hazard has no telegraph_ticks, active_ticks or max_activations",
                    h.name
                )));
                continue;
            }
            _ => {
                errs.push(E::HazardWindow(format!(
                    "{}: a timed hazard needs telegraph_ticks, active_ticks and max_activations",
                    h.name
                )));
                continue;
            }
        };
        let rect = IRect::from(&h.rect);
        let spec = HazardSpec {
            id: h.id,
            class,
            rect,
            telegraph_rect: h.telegraph_rect.as_ref().map_or(rect, IRect::from),
            window,
            damage: h.damage,
            push_x: h.push_x,
            hero_damage,
            hurts_enemies: h.hurts_enemies,
        };
        if let Err(e) = spec.validate() {
            errs.push(E::HazardSpec {
                hazard: h.name.clone(),
                reason: e.to_string(),
            });
        }
        if !within_play(room, spec.rect) || !within_play(room, spec.telegraph_rect) {
            errs.push(E::HazardOutOfBounds(h.name.clone()));
        }
        for (zone, protect) in [
            ("spawn", protected_spawn),
            ("exit", exit),
            ("checkpoint", checkpoint),
        ] {
            if overlaps(spec.rect, protect) || overlaps(spec.telegraph_rect, protect) {
                errs.push(E::HazardOnProtected {
                    hazard: h.name.clone(),
                    zone,
                });
            }
        }
        hazards.push(Hazard {
            name: h.name.clone(),
            owner,
            spec,
        });
    }

    // ---- per-kind ownership of hazards, valve config
    for (i, raw_prop) in raw.props.iter().enumerate() {
        let Some(pos) = props.iter().position(|p| p.id == raw_prop.id) else {
            continue;
        };
        let _ = i;
        let kind = props[pos].kind;
        let want = match kind {
            PropKind::Valve => Some(HazardClass::Steam),
            PropKind::WeightRig => Some(HazardClass::FallingMass),
            PropKind::Drum => Some(HazardClass::Blast),
            // A shutter is never a hazard volume: it hurts nobody (KR5).
            PropKind::Shutter => None,
            PropKind::RotSticker => {
                // HAZ never authors a damage-dealing volume for a sticker: its
                // damage comes only from its BlastCharge (no double blast).
                if hazards.iter().any(|h| h.owner == Some(pos)) {
                    errs.push(E::HazardOwnerMismatch {
                        prop: raw_prop.id.clone(),
                        reason: "a rot_sticker owns no hazard volume (its blast is a BlastCharge)"
                            .into(),
                    });
                }
                continue;
            }
        };
        if let Some(want) = want {
            let owned = hazards
                .iter()
                .filter(|h| h.owner == Some(pos) && h.spec.class == want)
                .count();
            if owned != 1 {
                errs.push(E::HazardOwnerMismatch {
                    prop: raw_prop.id.clone(),
                    reason: format!("needs exactly one {} hazard, has {owned}", want.name()),
                });
            }
        } else if hazards.iter().any(|h| h.owner == Some(pos)) {
            errs.push(E::HazardOwnerMismatch {
                prop: raw_prop.id.clone(),
                reason: "a shutter owns no hazard".into(),
            });
        }
        if kind == PropKind::Shutter
            && let Some(rs) = raw_prop.shutter.as_ref()
        {
            let bad = |reason: String| E::ShutterConfig {
                prop: raw_prop.id.clone(),
                reason,
            };
            match ShutterInitial::parse(&rs.initial) {
                None => errs.push(bad(format!(
                    "initial {:?} must be open or closed",
                    rs.initial
                ))),
                Some(initial) => {
                    let mut cfg = ShutterConfig::defaults(initial);
                    cfg.arm_x = rs.arm_x;
                    if let Some(v) = rs.arm_trail_px {
                        cfg.arm_trail_px = v;
                    }
                    if let Some(v) = rs.warning_ticks {
                        cfg.warning_ticks = v;
                    }
                    if let Some(v) = rs.speed_px_per_tick {
                        cfg.speed_px_per_tick = v;
                    }
                    if let Some(v) = rs.slice_px {
                        cfg.slice_px = v;
                    }
                    if let Some(v) = rs.cycles {
                        cfg.cycles = v;
                    }
                    if let Some(v) = rs.pin_ticks {
                        cfg.pin_ticks = v;
                    }
                    if let Some(v) = rs.max_closed_ticks {
                        cfg.max_closed_ticks = v;
                    }
                    match cfg.validate(props[pos].target) {
                        Ok(()) => props[pos].shutter = Some(cfg),
                        Err(reason) => errs.push(bad(reason)),
                    }
                }
            }
        }
        if kind == PropKind::Valve {
            let steam = hazards
                .iter()
                .find(|h| h.owner == Some(pos) && h.spec.class == HazardClass::Steam);
            if steam.is_some_and(|h| h.spec.is_persistent()) {
                errs.push(E::HazardOwnerMismatch {
                    prop: raw_prop.id.clone(),
                    reason: "a valve's steam must be a timed hazard".into(),
                });
            }
            let d = ValveConfig::DEFAULT;
            let rv = raw_prop.valve.as_ref();
            let cfg = ValveConfig {
                trigger_weight: rv
                    .and_then(|v| v.trigger_weight)
                    .unwrap_or(d.trigger_weight),
                launch_min_impulse_px_s: rv
                    .and_then(|v| v.launch_min_impulse_px_s)
                    .unwrap_or(d.launch_min_impulse_px_s),
                launch_weight: rv.and_then(|v| v.launch_weight).unwrap_or(d.launch_weight),
                blast_weight: rv.and_then(|v| v.blast_weight).unwrap_or(d.blast_weight),
                contact_weight: rv
                    .and_then(|v| v.contact_weight)
                    .unwrap_or(d.contact_weight),
                projectile_weight: rv
                    .and_then(|v| v.projectile_weight)
                    .unwrap_or(d.projectile_weight),
                shove_weight: rv.and_then(|v| v.shove_weight).unwrap_or(d.shove_weight),
                shove_min_impulse_px_s: rv
                    .and_then(|v| v.shove_min_impulse_px_s)
                    .unwrap_or(d.shove_min_impulse_px_s),
                // The timing is the steam hazard's: one source of truth.
                warning_ticks: steam.map_or(d.warning_ticks, |h| h.spec.telegraph_ticks()),
                vent_ticks: steam.map_or(d.vent_ticks, |h| h.spec.active_ticks()),
            };
            match cfg.validate() {
                Ok(()) => props[pos].valve = Some(cfg),
                Err(reason) => errs.push(E::ValveConfig {
                    prop: raw_prop.id.clone(),
                    reason,
                }),
            }
        }
        // The landed remnant stays inside the footprint of its own weight.
        if kind == PropKind::WeightRig
            && let (Some(rem), Some(fall)) = (
                props[pos].remnant,
                hazards
                    .iter()
                    .find(|h| h.owner == Some(pos) && h.spec.class == HazardClass::FallingMass),
            )
            && (rem.x < fall.spec.rect.x || rem.right() > fall.spec.rect.right())
        {
            errs.push(E::HazardOwnerMismatch {
                prop: raw_prop.id.clone(),
                reason: "the remnant must lie inside the landing footprint".into(),
            });
        }
    }

    // ---- links
    if raw.links.len() > MAX_AUTHORED_LINKS {
        errs.push(E::TooManyLinks(raw.links.len()));
    }
    let mut links: Vec<Link> = Vec::new();
    for l in &raw.links {
        let Some(kind) = LinkKind::parse(&l.kind) else {
            errs.push(E::UnknownLinkKind(l.kind.clone()));
            continue;
        };
        let (Some(&from), Some(&to)) = (prop_pos.get(l.from.as_str()), prop_pos.get(l.to.as_str()))
        else {
            for id in [&l.from, &l.to] {
                if !prop_pos.contains_key(id.as_str()) {
                    errs.push(E::LinkUnknownProp(id.clone()));
                }
            }
            continue;
        };
        if from == to {
            errs.push(E::LinkSelf(l.from.clone()));
            continue;
        }
        if let Some(&i) = [from, to]
            .iter()
            .find(|&&i| props[i].kind == PropKind::Shutter)
        {
            errs.push(E::LinkShutter(props[i].id.clone()));
            continue;
        }
        if links.iter().any(|k| k.from == from && k.to == to) {
            errs.push(E::LinkDuplicate {
                from: l.from.clone(),
                to: l.to.clone(),
            });
            continue;
        }
        let target = props[to].target;
        let contact = match kind {
            LinkKind::Steam => hazards.iter().any(|h| {
                h.owner == Some(from)
                    && h.spec.class == HazardClass::Steam
                    && overlaps(h.spec.rect, target)
            }),
            LinkKind::Landing => hazards.iter().any(|h| {
                h.owner == Some(from)
                    && h.spec.class == HazardClass::FallingMass
                    && overlaps(inflate(h.spec.rect, LANDING_CONTACT_MARGIN_PX), target)
            }),
            LinkKind::Blast => props[from].blast_radius_px.is_some_and(|r| {
                let (cx, cy) = (
                    props[from].target.x + props[from].target.w / 2,
                    props[from].target.y + props[from].target.h / 2,
                );
                dist2_point_rect(cx, cy, target) <= i64::from(r) * i64::from(r)
            }),
        };
        if !contact {
            errs.push(E::LinkNoContact {
                from: l.from.clone(),
                to: l.to.clone(),
            });
        }
        links.push(Link { from, to, kind });
    }
    if let Some(cycle) = find_cycle(props.len(), &links) {
        errs.push(E::LinkCycle(
            cycle.into_iter().map(|i| props[i].id.clone()).collect(),
        ));
    }

    // ---- roster
    let living_cap = usize::from(raw.max_living.min(MAX_LIVING_FIRST_PROBE));
    if raw.roster.len() > living_cap && !raw.waves {
        errs.push(E::TooManyLiving {
            roster: raw.roster.len(),
            max: raw.max_living.min(MAX_LIVING_FIRST_PROBE),
        });
    } else if raw.roster.len() > living_cap {
        // PTM-AD6: a wave room's roster is bounded per beat, and in total.
        let mut beats: Vec<(&str, usize)> = Vec::new();
        for r in &raw.roster {
            match beats.iter_mut().find(|(b, _)| *b == r.beat.as_str()) {
                Some((_, n)) => *n += 1,
                None => beats.push((r.beat.as_str(), 1)),
            }
        }
        let widest = beats.iter().map(|(_, n)| *n).max().unwrap_or(0);
        if widest > living_cap {
            errs.push(E::TooManyLiving {
                roster: widest,
                max: raw.max_living.min(MAX_LIVING_FIRST_PROBE),
            });
        } else if raw.roster.len() > MAX_ROSTER_WAVES {
            errs.push(E::TooManyLiving {
                roster: raw.roster.len(),
                max: MAX_ROSTER_WAVES as u8,
            });
        }
    }
    let platform_count = room.platforms.len();
    let mut roster: Vec<RosterEntry> = Vec::new();
    for r in &raw.roster {
        if !is_id(&r.id) {
            errs.push(E::BadId(r.id.clone()));
        }
        if !seen_ids.insert(r.id.as_str()) {
            errs.push(E::DuplicateId(r.id.clone()));
        }
        let allowed = |k: EnemyKind| {
            matches!(
                k,
                EnemyKind::Goblin | EnemyKind::Engineer | EnemyKind::Llama | EnemyKind::Hoarder
            )
        };
        let kind = if r.kind == "slime" {
            RosterKind::Slime
        } else {
            match EnemyKind::parse(&r.kind).filter(|k| allowed(*k)) {
                Some(k) => RosterKind::Room(k),
                None => {
                    errs.push(E::RosterKind {
                        entry: r.id.clone(),
                        kind: r.kind.clone(),
                    });
                    continue;
                }
            }
        };
        let haz_roster = |reason: &str| E::Haz {
            rule: 0,
            ids: vec![r.id.clone()],
            reason: reason.into(),
        };
        let facing = match (kind, r.facing) {
            (RosterKind::Slime, Some(f)) if f == 1 || f == -1 => f,
            (RosterKind::Slime, _) => {
                errs.push(haz_roster("a slime needs a facing of 1 or -1"));
                1
            }
            (RosterKind::Room(_), None) => 1,
            (RosterKind::Room(_), Some(_)) => {
                errs.push(haz_roster("only a slime has a facing"));
                1
            }
        };
        if !kind.is_slime() && (r.home.is_some() || r.linked_pool.is_some()) {
            errs.push(haz_roster("only a slime has a home or a linked_pool"));
        }
        if kind.is_slime() && (r.gate.is_some() || r.fallback_kind.is_some() || r.perch.is_some()) {
            errs.push(haz_roster(
                "a slime has no gate, fallback_kind or perch (it lives on the floor lane)",
            ));
        }
        let fallback = match r.fallback_kind.as_deref() {
            Some(name) => match EnemyKind::parse(name) {
                Some(EnemyKind::Goblin) => Some(EnemyKind::Goblin),
                _ => {
                    errs.push(E::RosterFallback {
                        entry: r.id.clone(),
                    });
                    None
                }
            },
            None => None,
        };
        if let Some(gate) = &r.gate
            && !KNOWN_GATES.contains(&gate.as_str())
        {
            errs.push(E::UnknownGate {
                entry: r.id.clone(),
                gate: gate.clone(),
            });
        }
        if r.gate.is_some() != r.fallback_kind.is_some() {
            errs.push(E::RosterFallback {
                entry: r.id.clone(),
            });
        }
        // Where the body stands.
        let top = match r.perch {
            Some(p) if p >= platform_count => {
                errs.push(E::RosterPerch {
                    entry: r.id.clone(),
                    perch: p,
                });
                room.floor_y
            }
            Some(p) => room.platforms[p].rect.y,
            None => room.floor_y,
        };
        let body = if kind.is_slime() {
            // The slime's idle hurtbox, mirrored by its facing.
            let b = &crate::slime::active().hazq05_hurtbox_idle;
            let (x0, x1) = (i32::from(b.x0), i32::from(b.x1));
            let left = if facing > 0 { r.x + x0 } else { r.x - x1 };
            IRect::new(left, top - i32::from(b.y1), x1 - x0, i32::from(b.y1 - b.y0))
        } else {
            IRect::new(
                r.x - ENEMY_BODY.0 / 2,
                top - ENEMY_BODY.1,
                ENEMY_BODY.0,
                ENEMY_BODY.1,
            )
        };
        if body.x < room.play_x.0 || body.right() > room.play_x.1 {
            errs.push(E::RosterOutOfBounds(r.id.clone()));
        }
        if overlaps(body, protected_spawn) {
            errs.push(E::RosterOnProtected(r.id.clone()));
        }
        for name in &r.allow_in_hazard {
            if !hazards.iter().any(|h| &h.name == name) {
                errs.push(E::RosterUnknownHazard {
                    entry: r.id.clone(),
                    hazard: name.clone(),
                });
            }
        }
        for h in &hazards {
            let exempt = r.allow_in_hazard.contains(&h.name);
            if !exempt && (overlaps(body, h.spec.rect) || overlaps(body, h.spec.telegraph_rect)) {
                errs.push(E::RosterInHazard {
                    entry: r.id.clone(),
                    hazard: h.name.clone(),
                });
            }
        }
        // F7: an enemy threatens a surface it stands on, or one its attack
        // reaches: a ranged kind, or a melee swing within its reach of the
        // surface it stands on (vertical reach only; the horizontal fit is a
        // scene check).
        for s in &r.threatens {
            if *s >= platform_count {
                errs.push(E::GateUnknownSurface(*s));
                continue;
            }
            let surface_top = room.platforms[*s].rect.y;
            let reaches = kind.room().is_some_and(kind_is_ranged)
                || r.perch == Some(*s)
                || (top - surface_top).abs() <= classes.enemy_melee_reach_px;
            if !reaches {
                errs.push(E::ThreatNotReachable {
                    entry: r.id.clone(),
                    surface: *s,
                });
            }
        }
        roster.push(RosterEntry {
            id: r.id.clone(),
            kind,
            facing,
            home: r.home.unwrap_or(r.x),
            linked_pool: r.linked_pool.clone(),
            gate: r.gate.clone(),
            fallback,
            x: r.x,
            perch: r.perch,
            beat: r.beat.clone(),
            trigger_x: r.trigger_x,
            delay_ticks: r.delay_ticks,
            threatens: r.threatens.clone(),
            allow_in_hazard: r.allow_in_hazard.clone(),
            optional: r.optional,
        });
    }
    for (i, a) in roster.iter().enumerate() {
        for b in &roster[i + 1..] {
            let same = a.trigger_x == b.trigger_x
                && a.delay_ticks.abs_diff(b.delay_ticks) < MIN_SPAWN_STAGGER_TICKS;
            if same {
                errs.push(E::SynchronizedSpawn {
                    a: a.id.clone(),
                    b: b.id.clone(),
                });
            }
        }
    }

    // ---- checkpoint: before the action, never inside a lethal chain
    if let Some(first) = props.iter().map(|p| p.target.x).min()
        && checkpoint.right() > first
    {
        errs.push(E::CheckpointInvalid(
            "must lie left of every interactive prop".into(),
        ));
    }
    for h in &hazards {
        if overlaps(checkpoint, h.spec.rect) || overlaps(checkpoint, h.spec.telegraph_rect) {
            errs.push(E::CheckpointInvalid(format!("overlaps hazard {}", h.name)));
        }
    }

    // ---- height gates and the K8 nest rule
    let mut gates: Vec<Option<GateClass>> = vec![None; platform_count];
    for g in &raw.gates {
        let Some(declared) = ReachHero::parse(&g.class) else {
            errs.push(E::GateClassName(g.class.clone()));
            continue;
        };
        if g.surface >= platform_count {
            errs.push(E::GateUnknownSurface(g.surface));
            continue;
        }
        if gates[g.surface].replace(declared).is_some() {
            errs.push(E::DuplicateId(format!("gate for platform {}", g.surface)));
        }
    }
    let mut vantages: Vec<(usize, VantageCounter)> = Vec::new();
    for v in &raw.vantages {
        if v.surface >= platform_count {
            errs.push(E::VantageUnknownSurface(v.surface));
            continue;
        }
        match VantageCounter::parse(&v.counter) {
            Some(c) => {
                // F7: a counter is backed by the data, not only declared.
                let backed = match c {
                    VantageCounter::LaneCeiling => ctx.ceiling_px.is_some(),
                    VantageCounter::GroundRanged => roster
                        .iter()
                        .any(|e| e.kind.room().is_some_and(kind_is_ranged)),
                    VantageCounter::ReinforcementWave => {
                        roster.iter().any(|e| e.trigger_x.is_some())
                    }
                };
                if !backed {
                    errs.push(E::VantageUnbacked {
                        surface: v.surface,
                        counter: c.name(),
                    });
                }
                vantages.push((v.surface, c));
            }
            None => errs.push(E::VantageCounter(v.counter.clone())),
        }
    }
    let mut final_gates: Vec<GateClass> = Vec::new();
    for (i, platform) in room.platforms.iter().enumerate() {
        let rise = room.floor_y - platform.rect.y;
        let clearance = rise - raw.platform_slab_px;
        if clearance < classes.carl_height_px {
            errs.push(E::PlatformBlocksLane {
                surface: i,
                clearance,
            });
        }
        let Some(actual) = classes.gate_class(rise) else {
            errs.push(E::SurfaceUnreachable { surface: i, rise });
            final_gates.push(ReachHero::DonutBoosted);
            continue;
        };
        match gates[i] {
            None => errs.push(E::GateMissing(i)),
            Some(declared) if declared != actual => errs.push(E::GateMismatch {
                surface: i,
                declared: declared.name(),
                actual: actual.name(),
            }),
            Some(_) => {}
        }
        final_gates.push(actual);
        // K8: 81..=128 px must be threatened or a declared vantage.
        if actual != ReachHero::Carl {
            let threatened = roster
                .iter()
                .any(|e| e.perch == Some(i) || e.threatens.contains(&i));
            let vantage = vantages.iter().any(|(s, _)| *s == i);
            if !threatened && !vantage {
                errs.push(E::UnthreatenedNest { surface: i, rise });
            }
        }
    }

    // ---- DP-Q45: the boosted Donut under a lane ceiling
    let rule = classes.ceiling_rule;
    let boosted = classes.max_rise(ReachHero::DonutBoosted);
    let mut tall: Vec<(String, i32)> = Vec::new();
    for (i, platform) in room.platforms.iter().enumerate() {
        let rise = room.floor_y - platform.rect.y;
        if rise > rule.free_surface_px && rise <= boosted {
            tall.push((format!("platform {i}"), rise));
        }
    }
    for p in &props {
        let solids = p
            .remnant
            .into_iter()
            .chain(p.solid_intact.then_some(p.target));
        for r in solids {
            let rise = room.floor_y - r.y;
            if rise > rule.free_surface_px && rise <= boosted {
                tall.push((format!("prop {}", p.id), rise));
            }
        }
    }
    for st in raw.statics.iter().filter(|st| st.kind == "solid") {
        let rise = room.floor_y - IRect::from(&st.rect).y;
        if rise > rule.free_surface_px && rise <= boosted {
            tall.push((format!("static {}", st.id), rise));
        }
    }
    match ctx.ceiling_px {
        Some(c) if c <= 0 || c > rule.ceiling_max_px => errs.push(E::CeilingTooHigh {
            ceiling_px: c,
            max: rule.ceiling_max_px,
        }),
        Some(_) => {}
        None => {
            if let Some((surface, rise)) = tall.into_iter().next() {
                errs.push(E::CeilingMissing { surface, rise });
            }
        }
    }

    // ---- locks: a solid that stays closed until an event
    let mut locks: Vec<Lock> = Vec::new();
    for l in &raw.locks {
        if !is_id(&l.id) {
            errs.push(E::BadId(l.id.clone()));
        }
        if !seen_ids.insert(l.id.as_str()) {
            errs.push(E::DuplicateId(l.id.clone()));
        }
        if l.opens_on.event != "enemy_defeated" {
            errs.push(E::LockUnknownEvent(l.opens_on.event.clone()));
            continue;
        }
        let Some(target) = roster.iter().position(|e| e.id == l.opens_on.target) else {
            errs.push(E::LockUnknownTarget {
                lock: l.id.clone(),
                target: l.opens_on.target.clone(),
            });
            continue;
        };
        if roster[target].optional {
            errs.push(E::LockOptionalTarget {
                lock: l.id.clone(),
                target: l.opens_on.target.clone(),
            });
        }
        let solid = IRect::from(&l.solid);
        if !within_play(room, solid) || solid.bottom() != room.floor_y {
            errs.push(E::LockOutOfBounds(l.id.clone()));
        }
        for (zone, protect) in [("spawn", protected_spawn), ("checkpoint", checkpoint)] {
            if overlaps(solid, protect) {
                errs.push(E::LockOnProtected {
                    lock: l.id.clone(),
                    zone,
                });
            }
        }
        locks.push(Lock {
            id: l.id.clone(),
            solid,
            opens_on: LockEvent::EnemyDefeated { roster: target },
        });
    }

    // ---- statics: rectangles that never change state
    let mut statics: Vec<StaticEntry> = Vec::new();
    for st in &raw.statics {
        if !is_id(&st.id) {
            errs.push(E::BadId(st.id.clone()));
        }
        if !seen_ids.insert(st.id.as_str()) {
            errs.push(E::DuplicateId(st.id.clone()));
        }
        let bad = |reason: &str| E::StaticInvalid {
            id: st.id.clone(),
            reason: reason.into(),
        };
        let Some(kind) = StaticKind::parse(&st.kind) else {
            errs.push(bad("unknown kind (solid or hit_target)"));
            continue;
        };
        let rect = IRect::from(&st.rect);
        if !within_play(room, rect) || rect.bottom() != room.floor_y {
            errs.push(bad("must stand on the walking line inside the play area"));
        }
        if kind == StaticKind::Solid && rect.h > MAX_LANE_OBSTACLE_PX {
            errs.push(bad(&format!(
                "a solid is at most {MAX_LANE_OBSTACLE_PX} px high"
            )));
        }
        for (zone, protect) in [
            ("spawn", protected_spawn),
            ("exit", exit),
            ("checkpoint", checkpoint),
        ] {
            if overlaps(rect, protect) {
                errs.push(bad(&format!("overlaps the protected {zone}")));
            }
        }
        statics.push(StaticEntry {
            id: st.id.clone(),
            kind,
            rect,
        });
    }

    // ---- PR #200 review NB-2: the remnant-lift headroom. A hero standing in a
    // landed remnant is lifted onto its top, so the column above every remnant
    // must stay clear for the tallest body: the underside of every platform
    // slab over the remnant (and the room's lane ceiling) is at least Carl's
    // height above the remnant's top.
    for p in props.iter().filter(|p| p.kind == PropKind::WeightRig) {
        let Some(rem) = p.remnant else { continue };
        let need = classes.carl_height_px;
        let mut over: Vec<(String, i32)> = Vec::new();
        for (i, pl) in room.platforms.iter().enumerate() {
            let underside = pl.rect.y + raw.platform_slab_px;
            if pl.rect.x < rem.right() && pl.rect.right() > rem.x && underside <= rem.y {
                over.push((format!("platform {i}"), rem.y - underside));
            }
        }
        if let Some(ceiling) = ctx.ceiling_px {
            over.push((
                "the lane ceiling".to_owned(),
                rem.y - (room.floor_y - ceiling),
            ));
        }
        for (what, clearance) in over {
            if clearance < need {
                errs.push(E::RemnantHeadroom {
                    prop: p.id.clone(),
                    over: what,
                    clearance,
                    need,
                });
            }
        }
    }

    // ---- KR3: the closed shutter's own geometry
    let slab_px = raw.platform_slab_px;
    for (pi, p) in props
        .iter()
        .enumerate()
        .filter(|(_, p)| p.kind == PropKind::Shutter)
    {
        let t = p.target;
        let mut withs: Vec<String> = Vec::new();
        if t.bottom() != room.floor_y {
            errs.push(E::ShutterNotOnFloor(p.id.clone()));
        }
        // Sits under a platform slab: bounds the height now that the lane
        // obstacle limit is waived for it.
        let under = room.platforms.iter().any(|pl| {
            t.y == pl.rect.y + slab_px && t.x >= pl.rect.x && t.right() <= pl.rect.right()
        });
        if !under {
            errs.push(E::ShutterNotUnderPlatform(p.id.clone()));
        }
        for (i, pl) in room.platforms.iter().enumerate() {
            let slab = IRect::new(pl.rect.x, pl.rect.y, pl.rect.w, slab_px);
            if overlaps(t, slab) {
                withs.push(format!("platform {i}"));
            }
        }
        for other in props
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != pi)
            .map(|(_, o)| o)
        {
            if overlaps(t, other.target) {
                withs.push(format!("the target of prop {}", other.id));
            }
            if other.remnant.is_some_and(|r| overlaps(t, r)) {
                withs.push(format!("the remnant of prop {}", other.id));
            }
        }
        for l in &locks {
            if overlaps(t, l.solid) {
                withs.push(format!("the solid of lock {}", l.id));
            }
        }
        for st in &statics {
            if overlaps(t, st.rect) {
                withs.push(format!("the static {}", st.id));
            }
        }
        errs.extend(withs.into_iter().map(|with| E::ShutterOverlaps {
            prop: p.id.clone(),
            with,
        }));
    }

    // ---- reachability, in every destruction state (every lock open)
    let changeable = changeable_props(&props);
    // Static solids never change state: they are in every state's solids.
    let static_solids = static_solid_rects(&statics);
    let mut reach: Vec<ReachRow> = Vec::new();
    let mut effective: Vec<GateClass> = final_gates.clone();
    if props.len() <= MAX_PROPS {
        reach = reach_matrix(room, classes, &props, &changeable, &static_solids);
        for row in &reach {
            let hero = row.hero;
            let state = row.state;
            if !row.lane && (hero == ReachHero::Carl || !row.exit) {
                // KR3 clause 3: waived only where the closed shutter causes
                // the cut, i.e. the same state with every shutter open keeps
                // the lane. Any other cut still fails.
                let shutter_mask = shutter_state_mask(&props, &changeable);
                let waived = state & shutter_mask != 0
                    && reach
                        .iter()
                        .any(|r| r.hero == hero && r.state == state & !shutter_mask && r.lane);
                if !waived {
                    errs.push(E::LaneCut { hero, state });
                }
            }
            if !row.exit {
                errs.push(E::ExitUnreachable { hero, state });
            }
            if !row.regroup {
                errs.push(E::RegroupBlocked { hero, state });
            }
            for (surface, &(there, back)) in row.perches.iter().enumerate() {
                // A class at or above the surface's gate class must reach it
                // and get back, in every state.
                if hero >= final_gates[surface] {
                    if !there {
                        errs.push(E::PerchUnreachable {
                            hero,
                            state,
                            surface,
                        });
                    } else if !back {
                        errs.push(E::PerchNoReturn {
                            hero,
                            state,
                            surface,
                        });
                    }
                }
            }
        }
        // D3: the effective class of a surface is the weakest class whose
        // graph reaches it in any state; a gate is bypassed when that is
        // weaker than the declared (height) class.
        for (surface, gate) in final_gates.iter().enumerate() {
            let weakest = reach
                .iter()
                .filter(|r| r.perches[surface].0)
                .map(|r| r.hero)
                .min();
            if let Some(by) = weakest {
                effective[surface] = by;
                if by < *gate {
                    errs.push(E::GateBypassed {
                        surface,
                        declared: gate.name(),
                        by: by.name(),
                    });
                }
            }
        }
        // Locks: closed, no hero may be walled off from the enemy it waits on.
        if !locks.is_empty() {
            let checks: Vec<LockCheck> = locks
                .iter()
                .map(|l| {
                    let LockEvent::EnemyDefeated { roster: r } = l.opens_on;
                    LockCheck {
                        solid: l.solid,
                        perch: roster[r].perch,
                        x: roster[r].x,
                        trigger_x: roster[r].trigger_x,
                    }
                })
                .collect();
            for (lock, hero, state, why) in
                locked_unreachable(room, classes, &props, &changeable, &static_solids, &checks)
            {
                let lock = locks[lock].id.clone();
                errs.push(match why {
                    LockFailure::Target => E::LockTargetUnreachable { lock, hero, state },
                    LockFailure::Trigger => E::LockTriggerUnreachable { lock, hero, state },
                });
            }
        }
        // KR3 clause 4: the per-side bypass proof of every closed shutter.
        for (prop, hero, state, side, why) in
            shutter_side_failures(room, classes, &props, &changeable, &static_solids)
        {
            let prop = props[prop].id.clone();
            errs.push(match why {
                SideFailure::NoFloorNode => E::ShutterNoFloorNode {
                    prop,
                    hero,
                    state,
                    side,
                },
                SideFailure::NoCrossing => E::ShutterBypass {
                    prop,
                    hero,
                    state,
                    side,
                },
            });
        }
    }

    // ---- HAZ-A1: the hazard room's rules 1 to 10 (a no-op without HAZ data)
    let (acid_patches, pockets) = haz::check(
        &haz::HazInput {
            raw,
            ctx,
            goal,
            props: &props,
            roster: &roster,
            protected: [
                ("spawn", protected_spawn),
                ("checkpoint", checkpoint),
                ("exit", exit),
            ],
        },
        &mut errs,
    );

    if errs.is_empty() {
        Ok(Encounter {
            id: raw.id.clone(),
            room_id: raw.room_id.clone(),
            story_pin: raw.story_pin.clone(),
            goal,
            max_living: raw.max_living,
            platform_slab_px: raw.platform_slab_px,
            protected_spawn,
            checkpoint,
            exit,
            props,
            hazards,
            links,
            roster,
            gates: final_gates,
            vantages,
            locks,
            statics,
            effective_gates: effective,
            reach,
            acid_patches,
            pockets,
        })
    } else {
        Err(errs)
    }
}

/// A cycle of the link graph as a path, if one exists.
fn find_cycle(n: usize, links: &[Link]) -> Option<Vec<usize>> {
    // 0 = new, 1 = on the stack, 2 = done.
    fn visit(u: usize, links: &[Link], color: &mut [u8], path: &mut Vec<usize>) -> bool {
        color[u] = 1;
        path.push(u);
        for l in links.iter().filter(|l| l.from == u) {
            if color[l.to] == 1 {
                let start = path.iter().position(|&p| p == l.to).unwrap_or(0);
                path.drain(..start);
                path.push(l.to);
                return true;
            }
            if color[l.to] == 0 && visit(l.to, links, color, path) {
                return true;
            }
        }
        path.pop();
        color[u] = 2;
        false
    }
    let mut color = vec![0u8; n];
    for start in 0..n {
        if color[start] == 0 {
            let mut path = Vec::new();
            if visit(start, links, &mut color, &mut path) {
                return Some(path);
            }
        }
    }
    None
}

// ------------------------------------------------------------- reachability

#[derive(Debug, Clone, Copy)]
struct Node {
    x0: i32,
    x1: i32,
    top: i32,
    floor: bool,
}

fn gap(a: &Node, b: &Node) -> i32 {
    (b.x0 - a.x1).max(a.x0 - b.x1).max(0)
}

/// The nodes of one hero class in one state, and the solids it cannot hop:
/// no move crosses one of them.
struct Graph {
    nodes: Vec<Node>,
    cuts: Vec<(i32, i32)>,
    /// Index of the first platform node (platforms come right after the floor
    /// segments, in the room's order; standable solids come last).
    platform_base: usize,
}

impl Graph {
    /// The node of platform `i` of the room.
    fn platform(&self, i: usize) -> usize {
        self.platform_base + i
    }

    /// A solid it cannot hop stands between the two nodes.
    fn walled(&self, a: &Node, b: &Node) -> bool {
        let (g0, g1) = if b.x0 >= a.x1 {
            (a.x1, b.x0)
        } else if a.x0 >= b.x1 {
            (b.x1, a.x0)
        } else {
            return false;
        };
        self.cuts.iter().any(|&(c0, c1)| c0 < g1 && c1 > g0)
    }
}

/// The solid rectangles on the lane in one destruction state: an intact solid
/// prop blocks with its target, a used-up one with its remnant (if any).
fn blockers(props: &[Prop], changeable: &[usize], state: u16) -> Vec<IRect> {
    let mut out = Vec::new();
    for (bit, &i) in changeable.iter().enumerate() {
        let used = state & (1 << bit) != 0;
        let p = &props[i];
        if used {
            out.extend(p.remnant);
        } else if p.solid_intact {
            out.push(p.target);
        }
    }
    out
}

/// Bit mask of the changeable-prop bits that are shutters (set = closed).
fn shutter_state_mask(props: &[Prop], changeable: &[usize]) -> u16 {
    changeable
        .iter()
        .enumerate()
        .filter(|(_, i)| props[**i].kind == PropKind::Shutter)
        .fold(0u16, |m, (bit, _)| m | (1 << bit))
}

/// The closed shutters of one state (KR3): rectangles that cut the floor for
/// **every** class whatever their height and are **never** a standable node.
/// Kept apart from [`blockers`], whose solids a short class can stand on or
/// hop. A set bit means closed (the open state is bit 0, the superset check of
/// any held partial state).
fn shutter_cuts(props: &[Prop], changeable: &[usize], state: u16) -> Vec<IRect> {
    changeable
        .iter()
        .enumerate()
        .filter(|(bit, i)| props[**i].kind == PropKind::Shutter && state & (1 << bit) != 0)
        .map(|(_, &i)| props[i].target)
        .collect()
}

/// The props whose state the reach matrix enumerates (`2^n` states): the ones
/// with a solid or a remnant, and every shutter. Shared by the validation and
/// [`route_graphs`].
fn changeable_props(props: &[Prop]) -> Vec<usize> {
    props
        .iter()
        .enumerate()
        .filter(|(_, p)| p.solid_intact || p.remnant.is_some() || p.kind == PropKind::Shutter)
        .map(|(i, _)| i)
        .collect()
}

/// The sidecar's static solids (they are in every state), in file order.
fn static_solid_rects(statics: &[StaticEntry]) -> Vec<IRect> {
    statics
        .iter()
        .filter(|st| st.kind == StaticKind::Solid)
        .map(|st| st.rect)
        .collect()
}

/// The solids of one destruction state: the changeable props' blockers plus
/// the sidecar's static solids (in every state). The single builder shared by
/// the reach matrix, the closed-lock check and [`route_graphs`], so the exported
/// graphs cannot drift from the validated ones.
fn state_solids(props: &[Prop], changeable: &[usize], state: u16, statics: &[IRect]) -> Vec<IRect> {
    let mut solids = blockers(props, changeable, state);
    solids.extend_from_slice(statics);
    solids
}

fn platform_end_free(room: &Room, drop_gap: i32, r: IRect) -> bool {
    r.x - drop_gap >= room.play_x.0 || r.right() + drop_gap <= room.play_x.1
}

/// Floor segments of one hero class (cut by blockers it cannot hop), the
/// platforms, and every solid it can stand on (an intact drum, a landed
/// remnant, a closed lock) as a surface of its own: a lane solid is a step, not
/// only a wall.
///
/// `cut_only` are the closed shutters (KR3): each is a cut for every class and
/// never becomes a surface node, so the graph cannot step over one.
fn nodes_for(
    room: &Room,
    c: &HeightClasses,
    hero: ReachHero,
    solids: &[IRect],
    cut_only: &[IRect],
) -> Graph {
    let mut cuts: Vec<(i32, i32)> = solids
        .iter()
        .filter(|b| room.floor_y - b.y > c.max_rise(hero) || b.w > c.reach_x(hero))
        .map(|b| (b.x, b.right()))
        .chain(cut_only.iter().map(|b| (b.x, b.right())))
        .collect();
    cuts.sort_unstable();
    let mut nodes = Vec::new();
    let mut x = room.play_x.0;
    for &(c0, c1) in &cuts {
        if c0 > x {
            nodes.push(Node {
                x0: x,
                x1: c0,
                top: room.floor_y,
                floor: true,
            });
        }
        x = x.max(c1);
    }
    if x < room.play_x.1 {
        nodes.push(Node {
            x0: x,
            x1: room.play_x.1,
            top: room.floor_y,
            floor: true,
        });
    }
    let platform_base = nodes.len();
    for p in &room.platforms {
        nodes.push(Node {
            x0: p.rect.x,
            x1: p.rect.right(),
            top: p.rect.y,
            floor: false,
        });
    }
    for b in solids {
        if room.floor_y - b.y <= c.max_rise(hero) {
            nodes.push(Node {
                x0: b.x,
                x1: b.right(),
                top: b.y,
                floor: false,
            });
        }
    }
    Graph {
        nodes,
        cuts,
        platform_base,
    }
}

fn can_move(
    room: &Room,
    c: &HeightClasses,
    hero: ReachHero,
    g: &Graph,
    a: &Node,
    b: &Node,
) -> bool {
    if g.walled(a, b) {
        return false;
    }
    match (a.floor, b.floor) {
        (true, true) => false,
        (true, false) => {
            let rise = a.top - b.top;
            rise > 0
                && rise <= c.max_rise(hero)
                && gap(a, b) <= c.reach_x(hero)
                && platform_end_free(room, c.drop_gap_px, IRect::new(b.x0, b.top, b.x1 - b.x0, 1))
        }
        (false, true) => gap(a, b) <= c.drop_gap_px,
        (false, false) => {
            if b.top < a.top {
                a.top - b.top <= c.max_rise(hero) && gap(a, b) <= c.reach_x(hero)
            } else {
                gap(a, b) <= c.drop_gap_px
            }
        }
    }
}

fn reachable_from(
    room: &Room,
    c: &HeightClasses,
    hero: ReachHero,
    g: &Graph,
    start: usize,
) -> Vec<bool> {
    let nodes = &g.nodes;
    let mut seen = vec![false; nodes.len()];
    let mut stack = vec![start];
    seen[start] = true;
    while let Some(i) = stack.pop() {
        for (j, b) in nodes.iter().enumerate() {
            if !seen[j] && can_move(room, c, hero, g, &nodes[i], b) {
                seen[j] = true;
                stack.push(j);
            }
        }
    }
    seen
}

fn floor_node_at(nodes: &[Node], x: i32) -> Option<usize> {
    nodes.iter().position(|n| n.floor && x >= n.x0 && x < n.x1)
}

/// A lock for the closed-lock reachability check: its closed solid, where its
/// target enemy stands (`perch`, `x`) and the column that wakes it
/// (`trigger_x`).
struct LockCheck {
    solid: IRect,
    perch: Option<usize>,
    x: i32,
    trigger_x: Option<i32>,
}

/// Why a closed lock fails for a class and state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LockFailure {
    /// The target's standing place cannot be reached.
    Target,
    /// The column that wakes the target cannot be reached: the target never
    /// spawns and the lock never opens (a softlock).
    Trigger,
}

/// Locks closed: for every class and state, the target of every lock and the
/// trigger column that wakes it must be reachable from the hero's spawn with
/// all the solids in place. Returns the failing `(lock, hero, state, why)`.
fn locked_unreachable(
    room: &Room,
    c: &HeightClasses,
    props: &[Prop],
    changeable: &[usize],
    statics: &[IRect],
    locks: &[LockCheck],
) -> Vec<(usize, ReachHero, u16, LockFailure)> {
    let mut bad = Vec::new();
    for hero in ReachHero::ALL {
        for state in 0..(1u16 << changeable.len()) {
            let mut solids = state_solids(props, changeable, state, statics);
            solids.extend(locks.iter().map(|l| l.solid));
            let cut_only = shutter_cuts(props, changeable, state);
            let graph = nodes_for(room, c, hero, &solids, &cut_only);
            let Some(start) = floor_node_at(&graph.nodes, hero.spawn_x(room)) else {
                continue;
            };
            let seen = reachable_from(room, c, hero, &graph, start);
            let reached = |node: Option<usize>| node.is_some_and(|n| seen[n]);
            for (i, l) in locks.iter().enumerate() {
                let target = match l.perch {
                    Some(p) => Some(graph.platform(p)),
                    None => floor_node_at(&graph.nodes, l.x),
                };
                if !reached(target) {
                    bad.push((i, hero, state, LockFailure::Target));
                }
                if let Some(tx) = l.trigger_x
                    && !reached(floor_node_at(&graph.nodes, tx))
                {
                    bad.push((i, hero, state, LockFailure::Trigger));
                }
            }
        }
    }
    bad
}

/// Why the per-side proof of a closed shutter fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SideFailure {
    /// No floor node touches this side of the shutter.
    NoFloorNode,
    /// The exit is not reachable from that floor node: no real crossing.
    NoCrossing,
}

/// KR3 clause 4, the per-side bypass proof ("2P regroup"): in every state with
/// a shutter closed, for every class, the exit is reachable from the floor node
/// on **each** side of that shutter, on a graph with every closed shutter as a
/// cut and no shutter-top node, so it passes only through a real crossing such
/// as the walkway. A missing floor node on a side fails too. Returns the failing
/// `(prop, hero, state, side, why)`.
fn shutter_side_failures(
    room: &Room,
    c: &HeightClasses,
    props: &[Prop],
    changeable: &[usize],
    statics: &[IRect],
) -> Vec<(usize, ReachHero, u16, ShutterSide, SideFailure)> {
    let exit_x = room.goal_x.min(room.exit.right() - 1);
    let mask = shutter_state_mask(props, changeable);
    let mut bad = Vec::new();
    for hero in ReachHero::ALL {
        for state in 0..(1u16 << changeable.len()) {
            if state & mask == 0 {
                continue;
            }
            let solids = state_solids(props, changeable, state, statics);
            let cut_only = shutter_cuts(props, changeable, state);
            let graph = nodes_for(room, c, hero, &solids, &cut_only);
            let goal = floor_node_at(&graph.nodes, exit_x);
            for (bit, &prop) in changeable.iter().enumerate() {
                if props[prop].kind != PropKind::Shutter || state & (1 << bit) == 0 {
                    continue;
                }
                let t = props[prop].target;
                for (side, x) in [
                    (ShutterSide::Left, t.x - 1),
                    (ShutterSide::Right, t.right()),
                ] {
                    let Some(node) = floor_node_at(&graph.nodes, x) else {
                        bad.push((prop, hero, state, side, SideFailure::NoFloorNode));
                        continue;
                    };
                    let crosses =
                        goal.is_some_and(|g| reachable_from(room, c, hero, &graph, node)[g]);
                    if !crosses {
                        bad.push((prop, hero, state, side, SideFailure::NoCrossing));
                    }
                }
            }
        }
    }
    bad
}

/// The reach matrix: every hero class in every destruction state of the
/// changeable props (`2^n` states). `statics` are the solids of the sidecar's
/// static entries: they are in every state.
pub fn reach_matrix(
    room: &Room,
    c: &HeightClasses,
    props: &[Prop],
    changeable: &[usize],
    statics: &[IRect],
) -> Vec<ReachRow> {
    let exit_x = room.goal_x.min(room.exit.right() - 1);
    let mut rows = Vec::new();
    for hero in ReachHero::ALL {
        for state in 0..(1u16 << changeable.len()) {
            let solids = state_solids(props, changeable, state, statics);
            let cut_only = shutter_cuts(props, changeable, state);
            let graph = nodes_for(room, c, hero, &solids, &cut_only);
            let nodes = &graph.nodes;
            let spawn = floor_node_at(nodes, hero.spawn_x(room));
            let other_spawn = match hero {
                ReachHero::Carl => room.spawn_donut,
                _ => room.spawn_carl,
            };
            let other = floor_node_at(nodes, other_spawn);
            let goal = floor_node_at(nodes, exit_x);
            let exit = match (spawn, goal) {
                (Some(s), Some(g)) => reachable_from(room, c, hero, &graph, s)[g],
                _ => false,
            };
            let lane = matches!((spawn, goal), (Some(s), Some(g)) if s == g);
            let regroup = matches!((spawn, other), (Some(s), Some(o)) if s == o);
            let perches = room
                .platforms
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let node = graph.platform(i);
                    let there =
                        spawn.is_some_and(|s| reachable_from(room, c, hero, &graph, s)[node]);
                    // A way back only counts for a perch the class can reach.
                    let back = there
                        && goal.is_some_and(|g| reachable_from(room, c, hero, &graph, node)[g]);
                    (there, back)
                })
                .collect();
            rows.push(ReachRow {
                hero,
                state,
                exit,
                lane,
                regroup,
                perches,
            });
        }
    }
    rows
}

// ------------------------------------------------------------ route export

/// One surface of a route graph (layout px, y down), exported for the
/// physics-exact check of BOIL-A wire (K8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteNode {
    pub x0: i32,
    pub x1: i32,
    /// Layout y of the standing surface.
    pub top: i32,
    /// A stretch of the walking line (a platform or a standable solid is not).
    pub floor: bool,
}

/// The route graph of one hero class in one destruction state, exactly as the
/// validator built it: `edges` are the moves `can_move` accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteGraph {
    pub hero: ReachHero,
    /// Bit k set: the k-th changeable prop is used up.
    pub state: u16,
    /// Every lock closed (its solid in the graph). The reach matrix itself
    /// opens every lock, so an exit is only required with `false`.
    pub locks_closed: bool,
    /// The solids of this state: intact solid props, remnants, the sidecar's
    /// static solids (every state), closed locks.
    pub solids: Vec<IRect>,
    /// The closed shutters of this state (KR3): cuts of the floor for every
    /// class, never a standable node. A real wall for the physics check.
    pub cuts: Vec<IRect>,
    pub nodes: Vec<RouteNode>,
    /// Accepted moves, as `(from, to)` node indices.
    pub edges: Vec<(usize, usize)>,
    /// The floor node of the hero's spawn.
    pub spawn: Option<usize>,
    /// The floor node of the exit column.
    pub goal: Option<usize>,
}

/// The route graphs the validation reasoned over: every class in every state
/// of the changeable props with the locks open, and again with them closed when
/// the room has locks. The physics-exact check replays each accepted edge.
pub fn route_graphs(room: &Room, c: &HeightClasses, enc: &Encounter) -> Vec<RouteGraph> {
    let changeable = changeable_props(&enc.props);
    let static_solids = static_solid_rects(&enc.statics);
    let exit_x = room.goal_x.min(room.exit.right() - 1);
    let lock_variants: &[bool] = if enc.locks.is_empty() {
        &[false]
    } else {
        &[false, true]
    };
    let mut out = Vec::new();
    for &locks_closed in lock_variants {
        for hero in ReachHero::ALL {
            for state in 0..(1u16 << changeable.len()) {
                let mut solids = state_solids(&enc.props, &changeable, state, &static_solids);
                if locks_closed {
                    solids.extend(enc.locks.iter().map(|l| l.solid));
                }
                let cuts = shutter_cuts(&enc.props, &changeable, state);
                let graph = nodes_for(room, c, hero, &solids, &cuts);
                let mut edges = Vec::new();
                for (i, a) in graph.nodes.iter().enumerate() {
                    for (j, b) in graph.nodes.iter().enumerate() {
                        if i != j && can_move(room, c, hero, &graph, a, b) {
                            edges.push((i, j));
                        }
                    }
                }
                out.push(RouteGraph {
                    hero,
                    state,
                    locks_closed,
                    solids,
                    cuts,
                    spawn: floor_node_at(&graph.nodes, hero.spawn_x(room)),
                    goal: floor_node_at(&graph.nodes, exit_x),
                    nodes: graph
                        .nodes
                        .iter()
                        .map(|n| RouteNode {
                            x0: n.x0,
                            x1: n.x1,
                            top: n.top,
                            floor: n.floor,
                        })
                        .collect(),
                    edges,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn loaded() -> Loaded {
        load(root(), BOILER_ENCOUNTER_PATH).unwrap_or_else(|e| {
            panic!(
                "{}",
                e.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })
    }

    fn sidecar() -> Value {
        let text = std::fs::read_to_string(root().join(BOILER_ENCOUNTER_PATH)).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    fn ctx() -> Context {
        let l = loaded();
        Context {
            room: l.room,
            classes: l.classes,
            ceiling_px: l.ceiling_px,
        }
    }

    fn room() -> Room {
        ctx().room
    }

    /// The boiler's classes and ceiling around another room.
    fn ctx_for(room: Room) -> Context {
        Context { room, ..ctx() }
    }

    fn check(value: &Value) -> Result<Encounter, Vec<EncounterError>> {
        parse_sidecar(&value.to_string(), &ctx())
    }

    fn errors(value: &Value) -> Vec<EncounterError> {
        check(value).expect_err("the edited sidecar must be rejected")
    }

    fn has(errs: &[EncounterError], pred: impl Fn(&EncounterError) -> bool) -> bool {
        errs.iter().any(pred)
    }

    #[test]
    fn the_boiler_room_and_sidecar_load_and_validate() {
        let l = loaded();
        assert_eq!(l.room.id, "F01-BOILER");
        assert_eq!(l.room.render, (1152, 672));
        assert_eq!(l.room.floor_y, 580);
        assert_eq!(
            l.room.snap_dy, 0,
            "the authored y values are already on the grid"
        );
        let e = &l.encounter;
        assert_eq!(e.story_pin, "f01_pin_post_pile_boss");
        assert_eq!(e.props.len(), 3);
        assert_eq!(e.hazards.len(), 3);
        assert_eq!(e.links.len(), 2);
        assert_eq!(e.roster_size(), 4);
        assert_eq!(e.gates, [ReachHero::Carl, ReachHero::Carl]);
        assert_eq!(e.exit, l.room.exit);
        assert!(
            e.props
                .iter()
                .any(|p| p.optional && p.kind == PropKind::Drum)
        );
    }

    #[test]
    fn the_room_composes_through_the_existing_kit() {
        use crate::camera_lab::layout::build_kit;
        let l = loaded();
        let (kit, _, plan) = build_kit(root(), &l.room).expect("composes");
        assert_eq!((kit.width, kit.height), l.room.render);
        assert!(plan.tiers >= 2);
    }

    #[test]
    fn the_room_file_is_the_existing_schema_and_english() {
        let text = std::fs::read_to_string(root().join(BOILER_ROOM_PATH)).unwrap();
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["schema"], "floor01-demo-hall-v1");
        let r = room();
        for t in [&r.name, &r.subtitle, &r.purpose, &r.expected, &r.coop] {
            assert!(t.is_ascii(), "{t:?}");
        }
        // The room holds geometry only.
        assert!(r.props.is_empty() && r.enemies.is_empty() && r.hazards.is_empty());
    }

    #[test]
    fn the_file_hashes_are_pinned() {
        let l = loaded();
        let room_text = std::fs::read_to_string(root().join(BOILER_ROOM_PATH)).unwrap();
        assert_eq!(l.room_hash, fnv64(room_text.as_bytes()));
        assert_eq!(l.room_hash, ROOM_HASH, "update with the room file");
        assert_eq!(l.sidecar_hash, SIDECAR_HASH, "update with the sidecar");
    }

    // FNV-1a 64 of the two files: a data edit must update them on purpose.
    const ROOM_HASH: u64 = 3_100_011_409_838_489_558;
    const SIDECAR_HASH: u64 = 17_616_661_401_001_110_695;

    #[test]
    fn the_data_file_pins_the_three_classes() {
        // These numbers are the data file's, not the module's: a value that
        // moves in the jump specs moves in the file and in this one test.
        let c = ctx().classes;
        assert_eq!(c.gate_class(80), Some(ReachHero::Carl));
        assert_eq!(c.gate_class(81), Some(ReachHero::DonutBase));
        // D1: base Donut is the 104 px class; 112 px is informational only.
        assert_eq!(c.gate_class(96), Some(ReachHero::DonutBase));
        assert_eq!(c.gate_class(104), Some(ReachHero::DonutBase));
        assert_eq!(c.gate_class(105), Some(ReachHero::DonutBoosted));
        assert_eq!(c.gate_class(112), Some(ReachHero::DonutBoosted));
        assert_eq!(c.gate_class(128), Some(ReachHero::DonutBoosted));
        assert_eq!(c.gate_class(129), None);
        assert_eq!(c.donut_base_human_only_px, 112);
        assert_eq!(c.enemy_melee_reach_px, 30);
        assert_eq!(
            (
                c.ceiling_rule.free_surface_px,
                c.ceiling_rule.ceiling_max_px
            ),
            (42, 228)
        );
    }

    #[test]
    fn the_classes_come_from_data_so_a_moved_value_moves_the_verdict() {
        // A base Donut that lands on 128 px (a hypothetical DM-A change) turns
        // the boosted class into the base class for a 128 px gate.
        let text = std::fs::read_to_string(root().join(HEIGHT_CLASSES_PATH)).unwrap();
        let moved = text
            .replace("\"max_rise_px\": 104", "\"max_rise_px\": 120")
            .replace(
                "\"donut_base_human_only_px\": 112",
                "\"donut_base_human_only_px\": 124",
            );
        let c = HeightClasses::parse(&moved).unwrap();
        assert_eq!(c.gate_class(120), Some(ReachHero::DonutBase));
        assert_eq!(ctx().classes.gate_class(120), Some(ReachHero::DonutBoosted));
        // The same room validates under the moved data: the classes are read.
        let e = parse_sidecar(
            &sidecar().to_string(),
            &Context {
                classes: c,
                ..ctx()
            },
        );
        assert!(e.is_ok(), "{e:?}");
    }

    #[test]
    fn the_informational_human_only_ledge_feeds_no_rule() {
        // Moving 112 changes nothing about classes or verdicts (D1).
        let text = std::fs::read_to_string(root().join(HEIGHT_CLASSES_PATH)).unwrap();
        let moved = text.replace(
            "\"donut_base_human_only_px\": 112",
            "\"donut_base_human_only_px\": 118",
        );
        let c = HeightClasses::parse(&moved).unwrap();
        for rise in [80, 96, 104, 105, 112, 128] {
            assert_eq!(c.gate_class(rise), ctx().classes.gate_class(rise));
        }
        // But it may not sit under the base class.
        let bad = text.replace(
            "\"donut_base_human_only_px\": 112",
            "\"donut_base_human_only_px\": 100",
        );
        assert!(matches!(
            HeightClasses::parse(&bad),
            Err(EncounterError::ClassesInvalid(_))
        ));
    }

    #[test]
    fn bad_class_data_is_rejected() {
        let text = std::fs::read_to_string(root().join(HEIGHT_CLASSES_PATH)).unwrap();
        for bad in [
            text.replace("\"max_rise_px\": 104", "\"max_rise_px\": 60"),
            text.replace(
                "floor01-height-classes-v1\",",
                "floor01-height-classes-v0\",",
            ),
            text.replace("\"drop_gap_px\": 48", "\"drop_gap_px\": 0"),
            text.replace("\"ceiling_max_px\": 228", "\"ceiling_max_px\": 10"),
            text.replace(
                "\"carl_height_px\": 52,",
                "\"carl_height_px\": 52, \"extra\": 1,",
            ),
        ] {
            assert!(
                matches!(
                    HeightClasses::parse(&bad),
                    Err(EncounterError::ClassesInvalid(_))
                ),
                "{bad}"
            );
        }
    }

    #[test]
    fn dp_q45_the_boiler_declares_a_lane_ceiling_under_the_limit() {
        // Its 80 px perches are above 42 px and the boosted Donut stands on them.
        let l = loaded();
        assert_eq!(l.ceiling_px, Some(228));
        // Without the ceiling the same room is rejected.
        let mut c = ctx();
        c.ceiling_px = None;
        let errs = parse_sidecar(&sidecar().to_string(), &c).unwrap_err();
        assert!(
            has(&errs, |e| matches!(
                e,
                EncounterError::CeilingMissing { rise: 80, .. }
            )),
            "{errs:?}"
        );
        // Over the maximum, or not positive: rejected.
        for bad in [229, 300, 0, -4] {
            c.ceiling_px = Some(bad);
            let errs = parse_sidecar(&sidecar().to_string(), &c).unwrap_err();
            assert!(
                has(&errs, |e| matches!(
                    e,
                    EncounterError::CeilingTooHigh { .. }
                )),
                "{bad}"
            );
        }
        // 224 (the boss room's value) passes.
        c.ceiling_px = Some(224);
        assert!(parse_sidecar(&sidecar().to_string(), &c).is_ok());
    }

    #[test]
    fn dp_q45_a_room_with_only_low_surfaces_needs_no_ceiling() {
        // A 32 px perch: under the free height, so no ceiling is needed (the
        // lane check still wants 52 px under the slab, so use a thin rise).
        let low = room_with_platforms(&[]);
        let mut c = ctx_for(low);
        c.ceiling_px = None;
        let v = sidecar_for(&c.room, &[], |_| {});
        let r = parse_sidecar(&v.to_string(), &c);
        assert!(r.is_ok(), "{r:?}");
        // A solid prop above 42 px counts too: a 48 px drum.
        let mut v = sidecar_for(&c.room, &[], |v| {
            v["props"][2]["target"] = json!({ "x": 848, "y": 532, "w": 32, "h": 48 });
        });
        v["props"][2]["target"]["y"] = json!(532);
        let errs = parse_sidecar(&v.to_string(), &c).unwrap_err();
        assert!(
            has(&errs, |e| matches!(
                e,
                EncounterError::CeilingMissing { rise: 48, .. }
            )),
            "{errs:?}"
        );
    }

    #[test]
    fn the_room_file_arrays_stay_empty_and_ceiling_px_is_read_from_the_file() {
        let text = std::fs::read_to_string(root().join(BOILER_ROOM_PATH)).unwrap();
        assert_eq!(room_ceiling_px(&text, "F01-BOILER"), Some(228));
        assert_eq!(room_ceiling_px(&text, "OTHER"), None);
        let v: Value = serde_json::from_str(&text).unwrap();
        let s = &v["scenarios"][0];
        for key in ["props", "enemies", "hazards"] {
            assert_eq!(s[key].as_array().map(Vec::len), Some(0), "{key}");
        }
    }

    #[test]
    fn the_reach_matrix_covers_every_class_and_state_and_all_pass() {
        let e = loaded().encounter;
        // Two changeable props (the weight's remnant and the solid drum): 4 states.
        assert_eq!(e.reach.len(), 3 * 4);
        for row in &e.reach {
            assert!(row.exit && row.lane && row.regroup, "{row:?}");
            for perch in &row.perches {
                assert_eq!(*perch, (true, true), "{row:?}");
            }
        }
    }

    #[test]
    fn hazards_are_data_and_never_evadable() {
        let e = loaded().encounter;
        let set = e.hazard_set().unwrap();
        assert_eq!(set.volumes().len(), 3);
        for v in set.volumes() {
            assert!(!v.evadable());
            assert!(v.spec().hurts(crate::env_hazard::Team::Hero));
            assert_eq!(
                v.spec().hero_damage,
                crate::hero_damage::HeroDamagePolicy::AllHeroes
            );
        }
        let steam = e
            .owned_hazard(e.prop_index("valve").unwrap(), HazardClass::Steam)
            .unwrap();
        assert_eq!(
            (steam.spec.telegraph_ticks(), steam.spec.active_ticks()),
            (24, 72)
        );
        let valve = e.props[e.prop_index("valve").unwrap()].valve.unwrap();
        assert_eq!((valve.warning_ticks, valve.vent_ticks), (24, 72));
    }

    // ---- rejections

    #[test]
    fn unknown_fields_and_a_wrong_schema_are_rejected() {
        let mut v = sidecar();
        v["surprise"] = json!(1);
        assert!(has(&errors(&v), |e| matches!(e, EncounterError::Parse(_))));
        let mut v = sidecar();
        v["schema"] = json!("floor01-encounter-v0");
        assert!(has(&errors(&v), |e| matches!(e, EncounterError::Schema(_))));
        let mut v = sidecar();
        v["room_id"] = json!("NOPE");
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::RoomNotFound(_)
        )));
    }

    #[test]
    fn a_room_that_grants_anything_is_rejected() {
        let mut v = sidecar();
        v["grants"] = json!(["dynamite"]);
        assert!(has(&errors(&v), |e| *e == EncounterError::GrantsNotEmpty));
    }

    #[test]
    fn unknown_prop_ids_in_links_and_hazards_are_rejected() {
        let mut v = sidecar();
        v["links"][0]["to"] = json!("ghost");
        assert!(has(
            &errors(&v),
            |e| matches!(e, EncounterError::LinkUnknownProp(p) if p == "ghost")
        ));
        let mut v = sidecar();
        v["hazards"][0]["owner"] = json!("ghost");
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::HazardUnknownOwner { .. }
        )));
    }

    #[test]
    fn a_cycle_a_self_link_a_duplicate_and_a_fourth_link_are_rejected() {
        // Cycle: the drum's blast reaches the weight's brace (distance 80 from
        // the drum centre, radius widened for the test).
        let mut v = sidecar();
        v["props"][2]["blast_radius_px"] = json!(200);
        v["links"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "from": "drum", "to": "weight", "kind": "blast" }));
        let errs = errors(&v);
        assert!(
            has(
                &errs,
                |e| matches!(e, EncounterError::LinkCycle(p) if p.first() == p.last() && p.len() >= 3)
            ),
            "{errs:?}"
        );

        let mut v = sidecar();
        v["links"] = json!([{ "from": "valve", "to": "valve", "kind": "steam" }]);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::LinkSelf(_)
        )));

        let mut v = sidecar();
        v["links"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "from": "valve", "to": "weight", "kind": "steam" }));
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::LinkDuplicate { .. }
        )));

        // Four links: over the K2 bound, whatever else they are.
        let mut v = sidecar();
        v["props"][2]["blast_radius_px"] = json!(400);
        let extra = [
            json!({ "from": "drum", "to": "valve", "kind": "blast" }),
            json!({ "from": "drum", "to": "weight", "kind": "blast" }),
        ];
        v["links"].as_array_mut().unwrap().extend(extra);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::TooManyLinks(4)
        )));
    }

    #[test]
    fn a_link_needs_a_physical_contact() {
        // Move the brace out of the steam lane.
        let mut v = sidecar();
        v["props"][1]["target"]["x"] = json!(720 + 80);
        let errs = errors(&v);
        assert!(
            has(
                &errs,
                |e| matches!(e, EncounterError::LinkNoContact { from, to } if from == "valve" && to == "weight")
            ),
            "{errs:?}"
        );
        // Move the drum away from the landing footprint.
        let mut v = sidecar();
        v["props"][2]["target"]["x"] = json!(960);
        assert!(has(
            &errors(&v),
            |e| matches!(e, EncounterError::LinkNoContact { from, .. } if from == "weight")
        ));
    }

    #[test]
    fn hazards_cannot_sit_on_the_protected_spawn_exit_or_checkpoint() {
        for (x, zone) in [(100, "spawn"), (1090, "exit"), (170, "spawn")] {
            let mut v = sidecar();
            v["hazards"][0]["rect"] = json!({ "x": x, "y": 484, "w": 40, "h": 96 });
            let errs = errors(&v);
            assert!(
                has(
                    &errs,
                    |e| matches!(e, EncounterError::HazardOnProtected { zone: z, .. } if *z == zone)
                ),
                "{x}: {errs:?}"
            );
        }
        // The telegraph rectangle counts too.
        let mut v = sidecar();
        v["hazards"][1]["telegraph_rect"] = json!({ "x": 1080, "y": 572, "w": 16, "h": 8 });
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::HazardOnProtected { zone: "exit", .. }
        )));
    }

    #[test]
    fn a_hero_hazard_needs_a_readable_telegraph_and_a_short_window() {
        let mut v = sidecar();
        v["hazards"][0]["telegraph_ticks"] = json!(23);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::HazardSpec { .. }
        )));
        let mut v = sidecar();
        v["hazards"][0]["active_ticks"] = json!(400);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::HazardSpec { .. }
        )));
    }

    #[test]
    fn the_roster_cannot_spawn_into_damage_or_on_the_protected_spawn() {
        let mut v = sidecar();
        v["roster"][1]["x"] = json!(720); // inside the steam lane
        assert!(has(
            &errors(&v),
            |e| matches!(e, EncounterError::RosterInHazard { entry, .. } if entry == "g2")
        ));
        let mut v = sidecar();
        v["roster"][0]["x"] = json!(100);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::RosterOnProtected(_)
        )));
    }

    #[test]
    fn the_roster_is_bounded_staggered_and_limited_to_the_allowed_kinds() {
        let mut v = sidecar();
        v["roster"][1]["delay_ticks"] = json!(0);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::SynchronizedSpawn { .. }
        )));
        // PT-MASTERY: the llama is a roster kind (an existing kind, K16); a kind
        // no step has registered still is not.
        let mut v = sidecar();
        v["roster"][0]["kind"] = json!("llama");
        assert!(
            check(&v)
                .err()
                .is_none_or(|errs| !has(&errs, |e| matches!(e, EncounterError::RosterKind { .. }))),
            "a llama is an allowed roster kind"
        );
        let mut v = sidecar();
        v["roster"][0]["kind"] = json!("juicer");
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::RosterKind { .. }
        )));
        let mut v = sidecar();
        v["roster"][0]["kind"] = json!("bard");
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::RosterKind { .. }
        )));
        let mut v = sidecar();
        v["max_living"] = json!(5);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::MaxLivingOverCap(5)
        )));
        // A roster of five in one or two beats is over the cap without the wave
        // opt-in.
        let mut v = sidecar();
        v["roster"].as_array_mut().unwrap().push(
            json!({ "id": "g9", "kind": "goblin", "x": 400, "beat": "B3", "delay_ticks": 500 }),
        );
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::TooManyLiving { roster: 5, max: 4 }
        )));
        // PTM-AD6: with `"waves": true` a roster is bounded per beat (at most
        // `max_living` each) and in total (at most `MAX_ROSTER_WAVES`).
        let extra = |n: u32, beat: &str| json!({ "id": format!("g{}", 10 + n), "kind": "goblin", "x": 400, "beat": beat, "delay_ticks": 500 + n });
        let two_waves = || {
            let mut v = sidecar();
            v["waves"] = json!(true);
            for r in v["roster"].as_array_mut().unwrap() {
                r["beat"] = json!("W1");
            }
            for n in 0..4 {
                v["roster"].as_array_mut().unwrap().push(extra(n, "W2"));
            }
            v
        };
        let too_many = |v: &serde_json::Value| {
            check(v).err().is_some_and(|errs| {
                has(&errs, |e| matches!(e, EncounterError::TooManyLiving { .. }))
            })
        };
        assert!(!too_many(&two_waves()), "{:?}", check(&two_waves()).err());
        // The same roster without the opt-in is over the cap.
        let mut v = two_waves();
        v["waves"] = json!(false);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::TooManyLiving { roster: 8, max: 4 }
        )));
        // A fifth entry in one wave, and a ninth entry in all, are refused.
        let mut v = two_waves();
        v["roster"].as_array_mut().unwrap().push(extra(8, "W1"));
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::TooManyLiving { roster: 5, max: 4 }
        )));
        let mut v = two_waves();
        v["roster"].as_array_mut().unwrap().push(extra(9, "W3"));
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::TooManyLiving { roster: 9, max: 8 }
        )));
        // A gated entry needs the goblin fallback.
        let mut v = sidecar();
        v["roster"][3]["fallback_kind"] = json!(null);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::RosterFallback { .. }
        )));
        let mut v = sidecar();
        v["roster"][3]["gate"] = json!("cf02_bard");
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::UnknownGate { .. }
        )));
    }

    #[test]
    fn the_exit_and_the_checkpoint_follow_the_room_and_the_action() {
        let mut v = sidecar();
        v["exit"]["x"] = json!(1000);
        assert!(has(&errors(&v), |e| *e == EncounterError::ExitMismatch));
        let mut v = sidecar();
        v["checkpoint"]["x"] = json!(440);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::CheckpointInvalid(_)
        )));
    }

    #[test]
    fn a_remnant_or_hanging_weight_cannot_wall_or_bonk_the_lane() {
        let mut v = sidecar();
        v["props"][1]["remnant"]["h"] = json!(96);
        v["props"][1]["remnant"]["y"] = json!(484);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::ObstacleTooTall { .. }
        )));
        let mut v = sidecar();
        v["props"][1]["hang_px"] = json!(60);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::HangTooLow { .. }
        )));
        // A remnant on the exit seals nothing only if it is not there.
        let mut v = sidecar();
        v["props"][1]["remnant"] = json!({ "x": 1072, "y": 548, "w": 32, "h": 32 });
        let errs = errors(&v);
        assert!(
            has(&errs, |e| matches!(
                e,
                EncounterError::PropOnProtected { zone: "exit", .. }
            )),
            "{errs:?}"
        );
    }

    /// PR #200 review NB-2: a remnant under a platform slab (or too near the lane
    /// ceiling) would lift a hero's head into it. The shipped Boiler is clear.
    #[test]
    fn a_remnant_needs_headroom_for_the_lifted_hero_under_slabs_and_the_ceiling() {
        let l = loaded();
        let need = l.classes.carl_height_px;
        // The shipped weight: no platform over it, the ceiling far above.
        assert!(l.ceiling_px.is_some_and(|c| 580 - c <= 548 - need));
        // Under the left perch (slab underside 512): 548 - 512 = 36 < 52.
        let mut v = sidecar();
        v["props"][1]["remnant"] = json!({ "x": 320, "y": 548, "w": 64, "h": 32 });
        v["hazards"][1]["rect"] = json!({ "x": 320, "y": 484, "w": 64, "h": 96 });
        let errs = errors(&v);
        assert!(
            errs.iter().any(|e| matches!(
                e,
                EncounterError::RemnantHeadroom { over, clearance: 36, need: 52, .. }
                    if over == "platform 0"
            )),
            "{errs:?}"
        );
        // A remnant whose x range misses the slab (perch 0 ends at x 448) is clear.
        let mut v = sidecar();
        v["props"][1]["remnant"] = json!({ "x": 448, "y": 548, "w": 32, "h": 32 });
        v["hazards"][1]["rect"] = json!({ "x": 448, "y": 484, "w": 64, "h": 96 });
        let errs = check(&v).err().unwrap_or_default();
        assert!(
            !errs
                .iter()
                .any(|e| matches!(e, EncounterError::RemnantHeadroom { .. })),
            "{errs:?}"
        );
        // The lane ceiling counts too: at 60 px (underside y 520) only 28 px
        // are left above the remnant top (548).
        let mut c = ctx();
        c.ceiling_px = Some(60);
        let errs = parse_sidecar(&sidecar().to_string(), &c).unwrap_err();
        assert!(
            errs.iter().any(|e| matches!(
                e,
                EncounterError::RemnantHeadroom { over, clearance: 28, .. }
                    if over == "the lane ceiling"
            )),
            "{errs:?}"
        );
    }

    #[test]
    fn a_wall_on_the_lane_is_a_cut_in_every_class() {
        // Make the drum a 96 px solid in the middle of the lane: above Carl's
        // class it cuts the lane (also reported as an obstacle).
        let mut v = sidecar();
        v["props"][2]["target"] = json!({ "x": 848, "y": 484, "w": 32, "h": 96 });
        let errs = errors(&v);
        assert!(
            has(&errs, |e| matches!(
                e,
                EncounterError::LaneCut {
                    hero: ReachHero::Carl,
                    ..
                }
            )),
            "{errs:?}"
        );
        assert!(has(&errs, |e| matches!(
            e,
            EncounterError::ExitUnreachable {
                hero: ReachHero::Carl,
                ..
            }
        )));
        assert!(has(&errs, |e| matches!(
            e,
            EncounterError::PropOutOfReach(_)
        )));
    }

    #[test]
    fn the_gate_classes_must_match_the_heights() {
        let mut v = sidecar();
        v["gates"][0]["class"] = json!("donut_boosted");
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::GateMismatch { surface: 0, .. }
        )));
        let mut v = sidecar();
        v["gates"].as_array_mut().unwrap().pop();
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::GateMissing(1)
        )));
        let mut v = sidecar();
        v["gates"][0]["class"] = json!("superhero");
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::GateClassName(_)
        )));
        let mut v = sidecar();
        v["gates"][0]["surface"] = json!(9);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::GateUnknownSurface(9)
        )));
    }

    // ---- K8 and the height classes, on rooms built in the test

    fn room_with_platforms(platforms: &[(i32, i32, i32)]) -> Room {
        let list: Vec<Value> = platforms
            .iter()
            .map(|(x, rise, w)| json!({ "x": x, "y": 580 - rise, "w": w, "h": 16 }))
            .collect();
        let mut room_json: Value =
            serde_json::from_str(&std::fs::read_to_string(root().join(BOILER_ROOM_PATH)).unwrap())
                .unwrap();
        room_json["scenarios"][0]["platforms"] = Value::Array(list);
        parse_rooms(&room_json.to_string()).unwrap().remove(0)
    }

    fn sidecar_for(room: &Room, gates: &[(usize, &str)], extra: impl Fn(&mut Value)) -> Value {
        let mut v = sidecar();
        v["gates"] = Value::Array(
            gates
                .iter()
                .map(|(s, c)| json!({ "surface": s, "class": c }))
                .collect(),
        );
        extra(&mut v);
        let _ = room;
        v
    }

    #[test]
    fn k8_a_nest_nobody_threatens_is_an_error() {
        // A 96 px perch: Donut base reaches it, Carl does not, no enemy there.
        let room = room_with_platforms(&[(304, 96, 144)]);
        let v = sidecar_for(&room, &[(0, "donut_base")], |_| {});
        let errs = parse_sidecar(&v.to_string(), &ctx_for(room.clone())).unwrap_err();
        assert!(
            has(&errs, |e| matches!(
                e,
                EncounterError::UnthreatenedNest {
                    surface: 0,
                    rise: 96
                }
            )),
            "{errs:?}"
        );
        // 128 px likewise.
        let room = room_with_platforms(&[(304, 128, 144)]);
        let v = sidecar_for(&room, &[(0, "donut_boosted")], |_| {});
        let errs = parse_sidecar(&v.to_string(), &ctx_for(room.clone())).unwrap_err();
        assert!(has(&errs, |e| matches!(
            e,
            EncounterError::UnthreatenedNest {
                surface: 0,
                rise: 128
            }
        )));
    }

    #[test]
    fn k8_a_threatened_or_declared_nest_passes() {
        let room = room_with_platforms(&[(304, 96, 144)]);
        // An enemy stands on it.
        let v = sidecar_for(&room, &[(0, "donut_base")], |v| {
            v["roster"][0]["perch"] = json!(0);
            v["roster"][0]["x"] = json!(376);
        });
        let enc = parse_sidecar(&v.to_string(), &ctx_for(room.clone()))
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(enc.gates, [ReachHero::DonutBase]);
        // F7: a floor goblin cannot claim to threaten a 96 px perch: its swing
        // does not reach it.
        let v = sidecar_for(&room, &[(0, "donut_base")], |v| {
            v["roster"][0]["threatens"] = json!([0]);
        });
        let errs = parse_sidecar(&v.to_string(), &ctx_for(room.clone())).unwrap_err();
        assert!(has(&errs, |e| matches!(
            e,
            EncounterError::ThreatNotReachable { surface: 0, .. }
        )));
        // A declared vantage with a counter.
        let v = sidecar_for(&room, &[(0, "donut_base")], |v| {
            v["vantages"] = json!([{ "surface": 0, "counter": "lane_ceiling" }]);
        });
        assert!(parse_sidecar(&v.to_string(), &ctx_for(room.clone())).is_ok());
        // A counter must be one of the known ones.
        let v = sidecar_for(&room, &[(0, "donut_base")], |v| {
            v["vantages"] = json!([{ "surface": 0, "counter": "vibes" }]);
        });
        let errs = parse_sidecar(&v.to_string(), &ctx_for(room.clone())).unwrap_err();
        assert!(has(&errs, |e| matches!(
            e,
            EncounterError::VantageCounter(_)
        )));
    }

    #[test]
    fn carl_class_surfaces_are_not_nests() {
        // 80 px: Carl's class, no threat needed (the boiler's own perches).
        let room = room_with_platforms(&[(304, 80, 144), (928, 64, 128)]);
        let v = sidecar_for(&room, &[(0, "carl"), (1, "carl")], |_| {});
        assert!(parse_sidecar(&v.to_string(), &ctx_for(room.clone())).is_ok());
    }

    #[test]
    fn a_surface_above_every_class_is_an_error_and_so_is_a_lane_blocking_slab() {
        let room = room_with_platforms(&[(304, 144, 144)]);
        let v = sidecar_for(&room, &[(0, "donut_boosted")], |_| {});
        let errs = parse_sidecar(&v.to_string(), &ctx_for(room.clone())).unwrap_err();
        assert!(has(&errs, |e| matches!(
            e,
            EncounterError::SurfaceUnreachable {
                surface: 0,
                rise: 144
            }
        )));
        // A 48 px rise leaves 36 px under a 12 px slab: Carl does not fit.
        let room = room_with_platforms(&[(304, 48, 144)]);
        let v = sidecar_for(&room, &[(0, "carl")], |_| {});
        let errs = parse_sidecar(&v.to_string(), &ctx_for(room.clone())).unwrap_err();
        assert!(has(&errs, |e| matches!(
            e,
            EncounterError::PlatformBlocksLane {
                surface: 0,
                clearance: 36
            }
        )));
    }

    #[test]
    fn a_perch_flush_against_a_wall_has_no_way_back() {
        // A 80 px perch that fills the room from wall to wall: Carl cannot
        // climb on or drop off (no free end).
        let room = room_with_platforms(&[(16, 80, 1120)]);
        let v = sidecar_for(&room, &[(0, "carl")], |_| {});
        let errs = parse_sidecar(&v.to_string(), &ctx_for(room.clone())).unwrap_err();
        assert!(
            has(&errs, |e| matches!(
                e,
                EncounterError::PerchUnreachable {
                    hero: ReachHero::Carl,
                    ..
                }
            )),
            "{errs:?}"
        );
    }

    #[test]
    fn every_error_is_reported_not_only_the_first() {
        let mut v = sidecar();
        v["grants"] = json!(["dynamite"]);
        v["max_living"] = json!(9);
        v["links"][0]["to"] = json!("ghost");
        let errs = errors(&v);
        assert!(errs.len() >= 3, "{errs:?}");
        for e in &errs {
            assert!(!e.to_string().is_empty());
        }
    }

    #[test]
    fn the_donut_classes_reach_what_their_apex_allows() {
        // Two perches: 80 px, then 128 px with a gap of 208 px (more than the
        // reach of base Donut through the lower perch, and than the boosted
        // Donut's reach through it, who reaches the high perch from the floor).
        // Carl reaches the first only: the matrix says so, and the 128 px gate
        // holds.
        let room = room_with_platforms(&[(304, 80, 144), (48, 128, 48)]);
        let v = sidecar_for(&room, &[(0, "carl"), (1, "donut_boosted")], |v| {
            v["roster"][0]["perch"] = json!(1);
            v["roster"][0]["x"] = json!(72);
        });
        let enc = parse_sidecar(&v.to_string(), &ctx_for(room.clone()))
            .unwrap_or_else(|e| panic!("{e:?}"));
        let by = |hero, state| {
            enc.reach
                .iter()
                .find(|r| r.hero == hero && r.state == state)
                .unwrap()
        };
        assert_eq!(
            by(ReachHero::Carl, 0).perches,
            [(true, true), (false, false)]
        );
        assert_eq!(
            by(ReachHero::DonutBase, 0).perches,
            [(true, true), (false, false)]
        );
        assert_eq!(
            by(ReachHero::DonutBoosted, 0).perches,
            [(true, true), (true, true)]
        );
        assert_eq!(
            enc.effective_gates,
            [ReachHero::Carl, ReachHero::DonutBoosted]
        );
    }

    #[test]
    fn the_state_enumeration_includes_the_used_up_props() {
        let e = loaded().encounter;
        let rows: Vec<u16> = e
            .reach
            .iter()
            .filter(|r| r.hero == ReachHero::Carl)
            .map(|r| r.state)
            .collect();
        assert_eq!(rows, [0, 1, 2, 3]);
    }

    // ---- review round 1

    #[test]
    fn d1_a_112_px_surface_is_not_a_base_donut_gate() {
        // Declared base Donut at 112 px: it is the boosted class (112 is the
        // narrow human-only ledge, never an AI route).
        let room = room_with_platforms(&[(304, 112, 144)]);
        let v = sidecar_for(&room, &[(0, "donut_base")], |v| {
            v["vantages"] = json!([{ "surface": 0, "counter": "lane_ceiling" }]);
        });
        let errs = parse_sidecar(&v.to_string(), &ctx_for(room.clone())).unwrap_err();
        assert!(
            has(&errs, |e| matches!(
                e,
                EncounterError::GateMismatch {
                    surface: 0,
                    declared: "donut_base",
                    actual: "donut_boosted"
                }
            )),
            "{errs:?}"
        );
        // A 96 px perch is the base class and passes with a vantage (the room
        // parser snaps perches to whole 16 px tiles, so 104 px is a rack-ledge
        // value, not a tile height).
        let room = room_with_platforms(&[(304, 96, 144)]);
        let v = sidecar_for(&room, &[(0, "donut_base")], |v| {
            v["vantages"] = json!([{ "surface": 0, "counter": "lane_ceiling" }]);
        });
        let r = parse_sidecar(&v.to_string(), &ctx_for(room.clone()));
        assert!(r.is_ok(), "{r:?}");
    }

    #[test]
    fn d2_the_boiler_valve_threshold_is_the_160_that_lets_a_kicked_goblin_in() {
        let e = loaded().encounter;
        let valve = e.props[e.prop_index("valve").unwrap()].valve.unwrap();
        assert_eq!(valve.launch_min_impulse_px_s, 160);
        assert!(
            f32::from(valve.launch_min_impulse_px_s) <= crate::feel::KICK_LAUNCH_VX,
            "the living kick must open the sidecar's valve"
        );
        // The Hoarder's 180 px/s lunge counts by default.
        assert!(valve.shove_min_impulse_px_s <= 180);
    }

    #[test]
    fn d3_a_weaker_class_reaching_a_boost_gate_through_a_lower_perch_is_a_bypass() {
        // An 80 px perch and a 128 px perch only 32 px beside it: Carl walks
        // up the first and jumps to the second (48 px more).
        let room = room_with_platforms(&[(304, 80, 144), (480, 128, 144)]);
        let v = sidecar_for(&room, &[(0, "carl"), (1, "donut_boosted")], |v| {
            v["vantages"] = json!([{ "surface": 1, "counter": "lane_ceiling" }]);
        });
        let errs = parse_sidecar(&v.to_string(), &ctx_for(room.clone())).unwrap_err();
        assert!(
            has(&errs, |e| matches!(
                e,
                EncounterError::GateBypassed {
                    surface: 1,
                    declared: "donut_boosted",
                    by: "carl"
                }
            )),
            "{errs:?}"
        );
        // A gap of 160 px (over Carl's reach) still lets base Donut bypass it.
        let room = room_with_platforms(&[(304, 80, 144), (608, 128, 144)]);
        let v = sidecar_for(&room, &[(0, "carl"), (1, "donut_boosted")], |v| {
            v["vantages"] = json!([{ "surface": 1, "counter": "lane_ceiling" }]);
        });
        let errs = parse_sidecar(&v.to_string(), &ctx_for(room.clone())).unwrap_err();
        assert!(
            has(&errs, |e| matches!(
                e,
                EncounterError::GateBypassed {
                    surface: 1,
                    by: "donut_base",
                    ..
                }
            )),
            "{errs:?}"
        );
        // With a gap of 208 px the gate holds (the perch is on the far side of
        // the spawn, away from every lane solid): only the boosted class
        // reaches it, straight from the floor.
        let room = room_with_platforms(&[(304, 80, 144), (48, 128, 48)]);
        let v = sidecar_for(&room, &[(0, "carl"), (1, "donut_boosted")], |v| {
            v["vantages"] = json!([{ "surface": 1, "counter": "lane_ceiling" }]);
        });
        let enc = parse_sidecar(&v.to_string(), &ctx_for(room.clone()))
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(
            enc.effective_gates,
            [ReachHero::Carl, ReachHero::DonutBoosted]
        );
    }

    #[test]
    fn f1_a_room_file_with_props_enemies_or_hazards_is_rejected() {
        let mut room_json: Value =
            serde_json::from_str(&std::fs::read_to_string(root().join(BOILER_ROOM_PATH)).unwrap())
                .unwrap();
        let scenario = &mut room_json["scenarios"][0];
        scenario["props"] =
            json!([{ "kind": "crate", "x": 600, "y": 548, "w": 32, "h": 32, "breakable": true }]);
        scenario["enemies"] = json!([{ "kind": "goblin", "x": 300 }]);
        scenario["hazards"] = json!([{ "x": 700, "w": 64, "h": 32, "kind": "heat" }]);
        let room = parse_rooms(&room_json.to_string()).unwrap().remove(0);
        let errs = parse_sidecar(&sidecar().to_string(), &ctx_for(room)).unwrap_err();
        for array in ["props", "enemies", "hazards"] {
            assert!(
                has(&errs, |e| *e
                    == EncounterError::RoomEntitiesNotEmpty { array }),
                "{array}: {errs:?}"
            );
        }
        // The boiler's own room is clean.
        assert!(check(&sidecar()).is_ok());
    }

    fn with_lock(v: &mut Value, solid: Value, target: &str) {
        v["locks"] = json!([{
            "id": "exit_door",
            "solid": solid,
            "opens_on": { "event": "enemy_defeated", "target": target }
        }]);
    }

    #[test]
    fn f2_a_lock_is_a_solid_until_its_enemy_is_defeated() {
        let mut v = sidecar();
        with_lock(
            &mut v,
            json!({ "x": 1040, "y": 484, "w": 16, "h": 96 }),
            "e1",
        );
        let enc = check(&v).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(enc.locks.len(), 1);
        let e1 = enc.roster.iter().position(|r| r.id == "e1").unwrap();
        assert_eq!(
            enc.locks[0].opens_on,
            LockEvent::EnemyDefeated { roster: e1 }
        );
        // The hook: closed it is a solid, opened it is nothing.
        let solid = IRect::new(1040, 484, 16, 96);
        assert_eq!(enc.lock_solids(&[false]), [solid]);
        assert_eq!(enc.lock_solids(&[]), [solid], "closed until told otherwise");
        assert!(enc.lock_solids(&[true]).is_empty());
        assert_eq!(enc.locks[0].solid(true), None);
        assert_eq!(enc.locks_opened_by(e1), [0]);
        assert!(enc.locks_opened_by(0).is_empty());
        // The exit is still reachable in every state with the lock open: the
        // matrix is computed without it.
        assert!(enc.reach.iter().all(|r| r.exit));
    }

    #[test]
    fn f2_bad_locks_are_rejected() {
        let solid = json!({ "x": 1040, "y": 484, "w": 16, "h": 96 });
        // Unknown target and unknown event.
        let mut v = sidecar();
        with_lock(&mut v, solid.clone(), "ghost");
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::LockUnknownTarget { .. }
        )));
        let mut v = sidecar();
        with_lock(&mut v, solid.clone(), "g1");
        v["locks"][0]["opens_on"]["event"] = json!("timer");
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::LockUnknownEvent(_)
        )));
        // An optional enemy cannot hold a door shut.
        let mut v = sidecar();
        with_lock(&mut v, solid.clone(), "g3");
        v["roster"][2]["optional"] = json!(true);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::LockOptionalTarget { .. }
        )));
        // On the protected spawn or the checkpoint, or off the lane.
        for (r, zone) in [
            (json!({ "x": 100, "y": 484, "w": 16, "h": 96 }), "spawn"),
            (json!({ "x": 190, "y": 484, "w": 16, "h": 96 }), "spawn"),
        ] {
            let mut v = sidecar();
            with_lock(&mut v, r, "g1");
            assert!(has(
                &errors(&v),
                |e| matches!(e, EncounterError::LockOnProtected { zone: z, .. } if *z == zone)
            ));
        }
        let mut v = sidecar();
        with_lock(
            &mut v,
            json!({ "x": 1040, "y": 400, "w": 16, "h": 96 }),
            "g1",
        );
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::LockOutOfBounds(_)
        )));
        // A closed wall between the spawn and the enemy it waits on.
        let mut v = sidecar();
        with_lock(
            &mut v,
            json!({ "x": 280, "y": 484, "w": 16, "h": 96 }),
            "g1",
        );
        let errs = errors(&v);
        assert!(
            has(&errs, |e| matches!(
                e,
                EncounterError::LockTargetUnreachable {
                    hero: ReachHero::Carl,
                    ..
                }
            )),
            "{errs:?}"
        );
        // A low closed solid (hoppable) walls nobody.
        let mut v = sidecar();
        with_lock(
            &mut v,
            json!({ "x": 280, "y": 548, "w": 16, "h": 32 }),
            "g1",
        );
        assert!(check(&v).is_ok());
        // Duplicate lock ids.
        let mut v = sidecar();
        with_lock(&mut v, solid.clone(), "g1");
        let again = v["locks"][0].clone();
        v["locks"].as_array_mut().unwrap().push(again);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::DuplicateId(_)
        )));
    }

    #[test]
    fn f2_a_lock_may_wait_on_a_gated_entry_because_its_fallback_is_in_the_roster() {
        let mut v = sidecar();
        with_lock(
            &mut v,
            json!({ "x": 1040, "y": 484, "w": 16, "h": 96 }),
            "e1",
        );
        assert_eq!(v["roster"][3]["gate"], "cf02_engineer");
        assert!(check(&v).is_ok());
    }

    #[test]
    fn f5_f13_hazards_may_be_persistent_and_may_have_no_prop_owner() {
        let acid = json!({
            "id": 4, "name": "acid", "class": "acid",
            "rect": { "x": 560, "y": 556, "w": 48, "h": 24 },
            "persistent": true,
            "damage": 1, "push_x": 0,
            "hero_damage": "all_heroes", "hurts_enemies": false
        });
        let mut v = sidecar();
        v["hazards"].as_array_mut().unwrap().push(acid.clone());
        let enc = check(&v).unwrap_or_else(|e| panic!("{e:?}"));
        let patch = enc.hazards.iter().find(|h| h.name == "acid").unwrap();
        assert_eq!(patch.owner, None);
        assert!(patch.spec.is_persistent());
        // It builds a volume that is on from the first tick.
        let set = enc.hazard_set().unwrap();
        assert_eq!(
            set.get(4).unwrap().phase(),
            crate::env_hazard::Phase::Active
        );
        // Timed fields on a persistent hazard, and a timed one without them,
        // are rejected.
        let mut bad = acid.clone();
        bad["telegraph_ticks"] = json!(24);
        let mut v = sidecar();
        v["hazards"].as_array_mut().unwrap().push(bad);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::HazardWindow(_)
        )));
        let mut v = sidecar();
        v["hazards"][0]
            .as_object_mut()
            .unwrap()
            .remove("active_ticks");
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::HazardWindow(_)
        )));
        // owner_reduced is not a world-hazard policy; the shared names parse.
        let mut v = sidecar();
        v["hazards"][0]["hero_damage"] = json!("owner_reduced");
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::HazardSpec { .. }
        )));
        let mut v = sidecar();
        v["hazards"][0]["hero_damage"] = json!("friendly_fire");
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::HazardPolicy(_)
        )));
        // The 24-tick floor applies to timed hero hazards only.
        let mut v = sidecar();
        v["hazards"][0]["telegraph_ticks"] = json!(23);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::HazardSpec { .. }
        )));
    }

    #[test]
    fn a_valve_steam_must_be_timed() {
        let mut v = sidecar();
        let steam = v["hazards"][0].as_object_mut().unwrap();
        for k in ["telegraph_ticks", "active_ticks", "max_activations"] {
            steam.remove(k);
        }
        steam.insert("persistent".into(), json!(true));
        let errs = errors(&v);
        assert!(has(&errs, |e| matches!(
            e,
            EncounterError::HazardOwnerMismatch { .. }
        )));
    }

    #[test]
    fn the_goal_is_an_open_enum_and_the_story_pin_is_neutral() {
        assert_eq!(Goal::parse("reach_exit"), Some(Goal::ReachExit));
        // HAZ-A1 appended the goal `defeat_roster` (HAZ-Q39).
        assert_eq!(Goal::parse("defeat_roster"), Some(Goal::DefeatRoster));
        assert_eq!(Goal::parse("defeat_party"), None);
        let mut v = sidecar();
        v["goal"] = json!("defeat_party");
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::UnknownGoal(_)
        )));
        assert_eq!(loaded().encounter.story_pin, "f01_pin_post_pile_boss");
    }

    #[test]
    fn d_a_roster_entry_may_opt_out_of_the_in_hazard_check() {
        let mut v = sidecar();
        v["roster"][1]["x"] = json!(720);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::RosterInHazard { .. }
        )));
        v["roster"][1]["allow_in_hazard"] = json!(["steam"]);
        assert!(check(&v).is_ok());
        assert_eq!(
            check(&v).unwrap().roster[1].allow_in_hazard,
            ["steam".to_string()]
        );
    }

    #[test]
    fn the_in_hazard_opt_out_exempts_only_the_named_hazard() {
        // x 830 stands inside two volumes: the weight's footprint and the drum
        // blast. Naming one leaves the other rejecting; naming both passes.
        let mut v = sidecar();
        v["roster"][1]["x"] = json!(830);
        let hazards_of = |errs: Vec<EncounterError>| -> Vec<String> {
            errs.into_iter()
                .filter_map(|e| match e {
                    EncounterError::RosterInHazard { hazard, .. } => Some(hazard),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(hazards_of(errors(&v)), ["weight_fall", "drum_blast"]);
        v["roster"][1]["allow_in_hazard"] = json!(["weight_fall"]);
        assert_eq!(hazards_of(errors(&v)), ["drum_blast"]);
        v["roster"][1]["allow_in_hazard"] = json!(["weight_fall", "drum_blast"]);
        assert!(check(&v).is_ok());
        // A name that is no hazard is an error, not a silent wildcard.
        v["roster"][1]["allow_in_hazard"] = json!(["weight_fall", "drum_blast", "acid"]);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::RosterUnknownHazard { hazard, .. } if hazard == "acid"
        )));
    }

    #[test]
    fn r1_a_lane_solid_you_can_stand_on_is_a_step_of_the_route_graph() {
        // A 112 px perch 16 px right of the intact 32 px drum (848..880),
        // declared a boosted-Donut gate: Carl stands on the drum (32 px) and
        // jumps the other 80, so the gate is bypassed in the all-intact state.
        let room = room_with_platforms(&[(896, 112, 144)]);
        let v = sidecar_for(&room, &[(0, "donut_boosted")], |v| {
            v["vantages"] = json!([{ "surface": 0, "counter": "lane_ceiling" }]);
        });
        let errs = parse_sidecar(&v.to_string(), &ctx_for(room.clone())).unwrap_err();
        assert!(
            has(&errs, |e| matches!(
                e,
                EncounterError::GateBypassed {
                    surface: 0,
                    declared: "donut_boosted",
                    by: "carl"
                }
            )),
            "{errs:?}"
        );
        // The weight's landed remnant (784..848, 32 px) is a step too: a 112 px
        // perch right behind it is reachable by Carl in the landed state.
        let room = room_with_platforms(&[(640, 112, 144)]);
        let v = sidecar_for(&room, &[(0, "donut_boosted")], |v| {
            v["vantages"] = json!([{ "surface": 0, "counter": "lane_ceiling" }]);
        });
        let errs = parse_sidecar(&v.to_string(), &ctx_for(room)).unwrap_err();
        assert!(has(&errs, |e| matches!(
            e,
            EncounterError::GateBypassed {
                surface: 0,
                by: "carl",
                ..
            }
        )));
        // Far from every lane solid the gate holds: a small perch at the far
        // right end (208 px from the drum, beyond every reach through it).
        let room = room_with_platforms(&[(1088, 112, 32)]);
        let v = sidecar_for(&room, &[(0, "donut_boosted")], |v| {
            v["vantages"] = json!([{ "surface": 0, "counter": "lane_ceiling" }]);
        });
        let enc = parse_sidecar(&v.to_string(), &ctx_for(room)).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(enc.effective_gates, [ReachHero::DonutBoosted]);
    }

    #[test]
    fn r2_a_lock_cannot_wait_on_an_enemy_whose_trigger_sits_behind_it() {
        // g3 wakes when a hero passes x 1000; a 96 px lock at x 600 waits on g3.
        // Closed, no hero can reach x 1000, so g3 never spawns: a softlock.
        let mut v = sidecar();
        v["roster"][2]["x"] = json!(400);
        v["roster"][2]["trigger_x"] = json!(1000);
        with_lock(
            &mut v,
            json!({ "x": 600, "y": 484, "w": 16, "h": 96 }),
            "g3",
        );
        let errs = errors(&v);
        assert!(
            has(&errs, |e| matches!(
                e,
                EncounterError::LockTriggerUnreachable {
                    hero: ReachHero::Carl,
                    ..
                }
            )),
            "{errs:?}"
        );
        // The target itself is on the near side: only the trigger is the fault.
        assert!(!has(&errs, |e| matches!(
            e,
            EncounterError::LockTargetUnreachable { .. }
        )));
        // With the trigger in front of the lock, the same door is fine.
        let mut v = sidecar();
        v["roster"][2]["x"] = json!(400);
        v["roster"][2]["trigger_x"] = json!(480);
        with_lock(
            &mut v,
            json!({ "x": 600, "y": 484, "w": 16, "h": 96 }),
            "g3",
        );
        assert!(check(&v).is_ok(), "{:?}", check(&v));
    }

    #[test]
    fn f7_vantage_counters_need_data_behind_them() {
        let room = room_with_platforms(&[(304, 96, 144)]);
        let with = |counter: &str| {
            sidecar_for(&room, &[(0, "donut_base")], |v| {
                v["vantages"] = json!([{ "surface": 0, "counter": counter }]);
            })
        };
        // A lane ceiling needs a declared ceiling_px.
        let mut c = ctx_for(room.clone());
        c.ceiling_px = None;
        let errs = parse_sidecar(&with("lane_ceiling").to_string(), &c).unwrap_err();
        assert!(has(&errs, |e| matches!(
            e,
            EncounterError::VantageUnbacked {
                counter: "lane_ceiling",
                ..
            }
        )));
        // A ground-ranged counter needs a ranged kind in the roster (none).
        let errs =
            parse_sidecar(&with("ground_ranged").to_string(), &ctx_for(room.clone())).unwrap_err();
        assert!(has(&errs, |e| matches!(
            e,
            EncounterError::VantageUnbacked {
                counter: "ground_ranged",
                ..
            }
        )));
        // A reinforcement wave needs a triggered roster entry: the boiler has
        // two.
        assert!(
            parse_sidecar(
                &with("reinforcement_wave").to_string(),
                &ctx_for(room.clone())
            )
            .is_ok()
        );
        let mut v = with("reinforcement_wave");
        for r in v["roster"].as_array_mut().unwrap() {
            r.as_object_mut().unwrap().remove("trigger_x");
        }
        v["roster"][3]["delay_ticks"] = json!(300);
        let errs = parse_sidecar(&v.to_string(), &ctx_for(room.clone())).unwrap_err();
        assert!(has(&errs, |e| matches!(
            e,
            EncounterError::VantageUnbacked {
                counter: "reinforcement_wave",
                ..
            }
        )));
    }

    #[test]
    fn f7_an_enemy_on_a_lower_perch_can_threaten_a_surface_within_its_reach() {
        // Perch A at 80 px with a goblin on it; perch B at 96 px, 160 px away
        // on the far side of the spawn (beyond Carl's reach, so the 96 px gate
        // holds for Carl). The goblin's swing is within 30 px of B's height: it
        // threatens B.
        let room = room_with_platforms(&[(304, 80, 144), (48, 96, 96)]);
        let v = sidecar_for(&room, &[(0, "carl"), (1, "donut_base")], |v| {
            v["roster"][0]["perch"] = json!(0);
            v["roster"][0]["x"] = json!(376);
            v["roster"][0]["threatens"] = json!([1]);
        });
        let enc = parse_sidecar(&v.to_string(), &ctx_for(room.clone()))
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(enc.roster[0].threatens, [1]);
        // The same claim from the floor is refused.
        let v = sidecar_for(&room, &[(0, "carl"), (1, "donut_base")], |v| {
            v["roster"][0]["threatens"] = json!([1]);
        });
        let errs = parse_sidecar(&v.to_string(), &ctx_for(room)).unwrap_err();
        assert!(has(&errs, |e| matches!(
            e,
            EncounterError::ThreatNotReachable { surface: 1, .. }
        )));
    }

    #[test]
    fn the_ceiling_is_read_as_a_checked_integer() {
        let t = r#"{"scenarios":[{"id":"A","ceiling_px":99999999999}]}"#;
        assert_eq!(room_ceiling_px(t, "A"), None, "out of range is no ceiling");
        let t = r#"{"scenarios":[{"id":"A","ceiling_px":228}]}"#;
        assert_eq!(room_ceiling_px(t, "A"), Some(228));
    }

    /// B1: the exported route graphs are exactly the validated ones, on every
    /// committed sidecar: every static solid is in every state, and the exit
    /// reachability over the exported edges equals the validator's reach row.
    #[test]
    fn the_route_graphs_are_the_validated_graphs_on_every_sidecar() {
        for sidecar in [
            BOILER_ENCOUNTER_PATH,
            HAZARDS_ENCOUNTER_PATH,
            crate::pile_boss::ENCOUNTER_PATH,
            crate::f01_mastery::ENCOUNTER_PATH,
            crate::ring_fixtures::COURT_ENCOUNTER_PATH,
            crate::ring_fixtures::SPLIT_ENCOUNTER_PATH,
        ] {
            let l = load(root(), sidecar).unwrap_or_else(|e| panic!("{sidecar}: {e:?}"));
            let statics: Vec<IRect> = l
                .encounter
                .statics
                .iter()
                .filter(|st| st.kind == StaticKind::Solid)
                .map(|st| st.rect)
                .collect();
            let graphs = route_graphs(&l.room, &l.classes, &l.encounter);
            assert!(!graphs.is_empty(), "{sidecar}");
            let mut open_checked = 0;
            for g in &graphs {
                for st in &statics {
                    assert!(
                        g.solids.contains(st),
                        "{sidecar}: {:?} state {} lacks a static solid",
                        g.hero,
                        g.state
                    );
                    // The static solid is a node of the graph, as the validator saw it.
                    assert!(
                        g.nodes.iter().any(|n| !n.floor && n.top == st.y),
                        "{sidecar}: no node at the top of a static solid"
                    );
                }
                if g.locks_closed {
                    for lock in &l.encounter.locks {
                        assert!(g.solids.contains(&lock.solid), "{sidecar}: lock missing");
                    }
                    continue;
                }
                let row = l
                    .encounter
                    .reach
                    .iter()
                    .find(|r| r.hero == g.hero && r.state == g.state)
                    .unwrap_or_else(|| panic!("{sidecar}: no reach row"));
                let mut seen = vec![false; g.nodes.len()];
                if let (Some(s), Some(goal)) = (g.spawn, g.goal) {
                    let mut stack = vec![s];
                    seen[s] = true;
                    while let Some(i) = stack.pop() {
                        for &(a, b) in &g.edges {
                            if a == i && !seen[b] {
                                seen[b] = true;
                                stack.push(b);
                            }
                        }
                    }
                    assert_eq!(
                        seen[goal], row.exit,
                        "{sidecar}: {:?} state {} exit != validator",
                        g.hero, g.state
                    );
                } else {
                    assert!(!row.exit, "{sidecar}: no spawn/goal node but exit reached");
                }
                // `exit` is true in every state of every valid sidecar, so it
                // cannot pin the shutter cuts; `lane` and `regroup` can (a
                // closed shutter cuts the floor, the KR3 waiver).
                let other_x = match g.hero {
                    ReachHero::Carl => l.room.spawn_donut,
                    _ => l.room.spawn_carl,
                };
                let other = g
                    .nodes
                    .iter()
                    .position(|n| n.floor && other_x >= n.x0 && other_x < n.x1);
                assert_eq!(
                    row.lane,
                    matches!((g.spawn, g.goal), (Some(s), Some(t)) if s == t),
                    "{sidecar}: {:?} state {} lane != validator",
                    g.hero,
                    g.state
                );
                assert_eq!(
                    row.regroup,
                    matches!((g.spawn, other), (Some(s), Some(o)) if s == o),
                    "{sidecar}: {:?} state {} regroup != validator",
                    g.hero,
                    g.state
                );
                open_checked += 1;
            }
            assert_eq!(open_checked, l.encounter.reach.len(), "{sidecar}");
        }
    }

    /// B1 negative control: the Hoarder's bank is a static solid, and a graph
    /// built without it (the pre-fix behaviour) would lack its node.
    #[test]
    fn the_hoarder_bank_is_a_node_of_every_route_graph() {
        let l = load(root(), crate::pile_boss::ENCOUNTER_PATH).unwrap();
        let bank = l
            .encounter
            .statics
            .iter()
            .find(|st| st.id == "bank_a")
            .expect("bank_a");
        assert_eq!(bank.kind, StaticKind::Solid);
        for g in route_graphs(&l.room, &l.classes, &l.encounter) {
            assert!(g.solids.contains(&bank.rect), "{:?}", g.hero);
            assert!(
                g.nodes
                    .iter()
                    .any(|n| !n.floor && n.top == bank.rect.y && n.x0 == bank.rect.x),
                "{:?} state {}: no node on the bank",
                g.hero,
                g.state
            );
        }
    }

    #[test]
    fn dist2_is_exact() {
        let r = IRect::new(10, 10, 10, 10);
        assert_eq!(dist2_point_rect(15, 15, r), 0);
        assert_eq!(dist2_point_rect(5, 15, r), 25);
        assert_eq!(dist2_point_rect(25, 25, r), 25 + 25);
    }
}

/// RING-B core: the shutter rule (KR3) and its knobs on the Ring court.
#[cfg(test)]
mod shutter_tests {
    use super::*;
    use crate::ring_fixtures::{COURT_ENCOUNTER_PATH, COURT_ROOM_PATH};
    use serde_json::{Value, json};

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn loaded() -> Loaded {
        load(root(), COURT_ENCOUNTER_PATH).unwrap_or_else(|e| {
            panic!(
                "{}",
                e.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })
    }

    fn sidecar() -> Value {
        let text = std::fs::read_to_string(root().join(COURT_ENCOUNTER_PATH)).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    fn ctx() -> Context {
        let l = loaded();
        Context {
            room: l.room,
            classes: l.classes,
            ceiling_px: l.ceiling_px,
        }
    }

    fn result(v: &Value) -> Result<Encounter, Vec<EncounterError>> {
        parse_sidecar(&v.to_string(), &ctx())
    }

    fn errors(v: &Value) -> Vec<EncounterError> {
        result(v).expect_err("the edited sidecar must be rejected")
    }

    fn has(errs: &[EncounterError], pred: impl Fn(&EncounterError) -> bool) -> bool {
        errs.iter().any(pred)
    }

    fn shutter_config_error(errs: &[EncounterError]) -> Option<String> {
        errs.iter().find_map(|e| match e {
            EncounterError::ShutterConfig { reason, .. } => Some(reason.clone()),
            _ => None,
        })
    }

    /// The court with the shutter's knobs replaced.
    fn with_knobs(knobs: Value) -> Value {
        let mut v = sidecar();
        v["props"][0]["shutter"] = knobs;
        v
    }

    /// The boiler's drum (and its blast hazard) moved onto the court's left
    /// stretch: a solid prop, optionally widened so it cuts the lane for Carl.
    fn with_drum(v: &mut Value, w: i32) {
        v["props"].as_array_mut().unwrap().push(json!({
            "id": "drum",
            "kind": "drum",
            "target": { "x": 640, "y": 580 - 48, "w": w, "h": 48 },
            "solid_intact": true,
            "blast_radius_px": 48
        }));
        v["hazards"].as_array_mut().unwrap().push(json!({
            "id": 3,
            "name": "drum_blast",
            "class": "blast",
            "owner": "drum",
            "rect": { "x": 624, "y": 500, "w": 96, "h": 80 },
            "telegraph_ticks": 24,
            "active_ticks": 1,
            "max_activations": 1,
            "damage": 1,
            "push_x": 0,
            "hero_damage": "all_heroes",
            "hurts_enemies": true
        }));
    }

    #[test]
    fn the_court_loads_with_its_static_closed_shutter() {
        let l = loaded();
        assert_eq!(l.room.id, "F01-RING-COURT");
        let e = &l.encounter;
        assert_eq!(e.story_pin, "f01_pin_post_pile_boss");
        let p = &e.props[0];
        assert_eq!(p.kind, PropKind::Shutter);
        assert!(!p.solid_intact && p.remnant.is_none() && p.valve.is_none());
        let cfg = p.shutter.unwrap();
        assert!(cfg.is_static());
        assert_eq!(cfg.initial, ShutterInitial::Closed);
        // One changeable prop: two states (open, closed) times three classes.
        assert_eq!(e.reach.len(), 3 * 2);
        assert!(e.hazards.is_empty() && e.links.is_empty());
        let text = std::fs::read_to_string(root().join(COURT_ROOM_PATH)).unwrap();
        assert_eq!(room_ceiling_px(&text, "F01-RING-COURT"), Some(228));
    }

    #[test]
    fn a_moving_shutter_with_the_adopted_defaults_validates_the_whole_court() {
        // initial open, cycles 1 (the default), arm 192 px before the shutter.
        let v = with_knobs(json!({ "initial": "open", "arm_x": 1008 }));
        let e = result(&v).expect("a moving shutter validates");
        let cfg = e.props[0].shutter.unwrap();
        assert!(!cfg.is_static());
        assert_eq!(
            (
                cfg.warning_ticks,
                cfg.speed_px_per_tick,
                cfg.slice_px,
                cfg.cycles,
                cfg.pin_ticks,
                cfg.max_closed_ticks,
                cfg.arm_trail_px
            ),
            (40, 2, 4, 1, 36, 150, 320)
        );
        assert_eq!(cfg.blocked_window_ticks(68), 218, "KR19");
        // Both states are still proven: the open lane, the closed crossing.
        assert_eq!(e.reach.len(), 6);
    }

    #[test]
    fn a_shutter_object_is_required_and_belongs_to_a_shutter_only() {
        let mut v = sidecar();
        v["props"][0].as_object_mut().unwrap().remove("shutter");
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::PropFields { reason, .. } if reason.contains("needs a shutter object")
        )));
        let mut v = sidecar();
        with_drum(&mut v, 32);
        v["props"][1]["shutter"] = json!({ "initial": "closed" });
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::PropFields { prop, reason }
                if prop == "drum" && reason.contains("only a shutter")
        )));
        // A shutter is not a solid prop, a hanging weight or a valve.
        for (key, value) in [
            ("solid_intact", json!(true)),
            ("hang_px", json!(128)),
            ("blast_radius_px", json!(48)),
        ] {
            let mut v = sidecar();
            v["props"][0][key] = value;
            assert!(
                has(
                    &errors(&v),
                    |e| matches!(e, EncounterError::PropFields { prop, .. } if prop == "shutter")
                ),
                "{key}"
            );
        }
        let mut v = sidecar();
        v["props"][0]["surprise"] = json!(1);
        assert!(has(&errors(&v), |e| matches!(e, EncounterError::Parse(_))));
        let mut v = sidecar();
        v["props"][0]["shutter"]["surprise"] = json!(1);
        assert!(has(&errors(&v), |e| matches!(e, EncounterError::Parse(_))));
    }

    #[test]
    fn every_knob_is_bounded_on_both_sides() {
        // (knobs, must the sidecar validate?, text the error names)
        let moving = |extra: Value| {
            let mut k = json!({ "initial": "open", "arm_x": 1008 });
            for (key, value) in extra.as_object().unwrap() {
                k[key] = value.clone();
            }
            k
        };
        let cases: Vec<(Value, Option<&str>)> = vec![
            // speed: 1..=2, and a slower shutter closes too slowly (68 px).
            (
                moving(json!({ "speed_px_per_tick": 3 })),
                Some("speed_px_per_tick"),
            ),
            (
                moving(json!({ "speed_px_per_tick": 0 })),
                Some("speed_px_per_tick"),
            ),
            (
                moving(json!({ "speed_px_per_tick": 1 })),
                Some("closes in 68 ticks"),
            ),
            // warning: 36..=48.
            (
                moving(json!({ "warning_ticks": 35 })),
                Some("warning_ticks"),
            ),
            (
                moving(json!({ "warning_ticks": 49 })),
                Some("warning_ticks"),
            ),
            (moving(json!({ "warning_ticks": 36 })), None),
            (moving(json!({ "warning_ticks": 48 })), None),
            // slices divide 68.
            (moving(json!({ "slice_px": 0 })), Some("slice_px")),
            (moving(json!({ "slice_px": 3 })), Some("slice_px")),
            (moving(json!({ "slice_px": 17 })), None),
            // pin ticks fit a u8 stagger.
            (moving(json!({ "pin_ticks": 0 })), Some("pin_ticks")),
            (moving(json!({ "pin_ticks": 256 })), Some("pin_ticks")),
            (moving(json!({ "pin_ticks": 255 })), None),
            (moving(json!({ "arm_trail_px": 0 })), Some("arm_trail_px")),
            // arm_x within 256 px of the shutter [1200, 1232).
            (moving(json!({ "arm_x": 943 })), Some("arm_x")),
            (moving(json!({ "arm_x": 944 })), None),
            (moving(json!({ "arm_x": 1488 })), None),
            (moving(json!({ "arm_x": 1489 })), Some("arm_x")),
            // dwell: 1..=150.
            (
                moving(json!({ "max_closed_ticks": 0 })),
                Some("max_closed_ticks"),
            ),
            (
                moving(json!({ "max_closed_ticks": 151 })),
                Some("max_closed_ticks"),
            ),
            (moving(json!({ "max_closed_ticks": 1 })), None),
            // a moving shutter needs its arming position and an open start.
            (json!({ "initial": "open" }), Some("needs arm_x")),
            (
                json!({ "initial": "closed", "cycles": 1, "arm_x": 1008 }),
                Some("starts open"),
            ),
            (moving(json!({ "cycles": 0 })), Some("starts closed")),
            (json!({ "initial": "ajar" }), Some("open or closed")),
        ];
        for (knobs, expect) in cases {
            let r = result(&with_knobs(knobs.clone()));
            match expect {
                None => assert!(r.is_ok(), "{knobs}: {r:?}"),
                Some(text) => {
                    let errs = r.expect_err(&knobs.to_string());
                    let reason = shutter_config_error(&errs)
                        .unwrap_or_else(|| panic!("{knobs}: no shutter config error in {errs:?}"));
                    assert!(reason.contains(text), "{knobs}: {reason}");
                }
            }
        }
        // A static gate needs only its start: the moving bounds do not apply.
        assert!(result(&with_knobs(json!({ "initial": "closed" }))).is_ok());
        assert!(
            result(&with_knobs(
                json!({ "initial": "closed", "max_closed_ticks": 400 })
            ))
            .is_ok(),
            "max_closed_ticks is for the moving machine only"
        );
    }

    #[test]
    fn the_kr19_window_rule_has_its_own_check() {
        use crate::env_prop::shutter_window_ok;
        // At the authored numbers and at the extremes of every other bound.
        assert!(shutter_window_ok(34, 150));
        assert!(
            shutter_window_ok(36, 150),
            "222 is the most the bounds allow"
        );
        // The edge of the rule itself: 2 x 36 + 227 = 299 passes, 228 does not.
        assert!(shutter_window_ok(36, 227));
        assert!(!shutter_window_ok(36, 228));
        assert!(!shutter_window_ok(36, 400));
        assert!(!shutter_window_ok(150, 1));
    }

    #[test]
    fn a_shutter_must_stand_on_the_walking_line() {
        let mut v = sidecar();
        v["props"][0]["target"] = json!({ "x": 1200, "y": 508, "w": 32, "h": 68 });
        let errs = errors(&v);
        assert!(has(&errs, |e| matches!(
            e,
            EncounterError::ShutterNotOnFloor(p) if p == "shutter"
        )));
    }

    #[test]
    fn a_shutter_must_sit_under_a_platform_slab_inside_its_x_range() {
        // Top one px below the slab underside (512): not flush.
        let mut v = sidecar();
        v["props"][0]["target"] = json!({ "x": 1200, "y": 513, "w": 32, "h": 67 });
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::ShutterNotUnderPlatform(_)
        )));
        // The walkway spans [1120, 1312): 1296 + 32 = 1328 sticks out.
        let mut v = sidecar();
        v["props"][0]["target"] = json!({ "x": 1296, "y": 512, "w": 32, "h": 68 });
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::ShutterNotUnderPlatform(_)
        )));
        // Flush against both ends of the walkway is inside its range.
        for x in [1120, 1280] {
            let mut v = sidecar();
            v["props"][0]["target"] = json!({ "x": x, "y": 512, "w": 32, "h": 68 });
            assert!(result(&v).is_ok(), "x {x}: {:?}", result(&v));
        }
    }

    #[test]
    fn a_shutter_with_no_platform_over_it_cuts_every_class_and_is_never_a_step() {
        // Only the shelf is left: the shutter has no slab over it (structure
        // error), and, because a closed shutter is a cut and never a standable
        // node, even a 68 px solid inside every class's rise walls off the exit
        // for Carl, base Donut and boosted Donut alike.
        let l = loaded();
        let mut room = l.room.clone();
        room.platforms = vec![room.platforms[2].clone()];
        let mut v = sidecar();
        v["gates"] = json!([{ "surface": 0, "class": "donut_boosted" }]);
        v["vantages"] = json!([{ "surface": 0, "counter": "lane_ceiling" }]);
        let errs = parse_sidecar(&v.to_string(), &Context { room, ..ctx() }).unwrap_err();
        assert!(has(&errs, |e| matches!(
            e,
            EncounterError::ShutterNotUnderPlatform(_)
        )));
        for hero in ReachHero::ALL {
            assert!(
                has(&errs, |e| *e
                    == EncounterError::ExitUnreachable { hero, state: 1 }),
                "{hero:?}: {errs:?}"
            );
            assert!(
                has(&errs, |e| matches!(
                    e,
                    EncounterError::ShutterBypass { hero: h, side: ShutterSide::Left, .. } if *h == hero
                )),
                "{hero:?}: {errs:?}"
            );
        }
    }

    #[test]
    fn nothing_may_overlap_the_closed_shutter() {
        // A platform slab: the shutter reaches up into the walkway.
        let mut v = sidecar();
        v["props"][0]["target"] = json!({ "x": 1200, "y": 506, "w": 32, "h": 74 });
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::ShutterOverlaps { with, .. } if with == "platform 1"
        )));
        // Another prop's target (a second shutter).
        let mut v = sidecar();
        v["props"].as_array_mut().unwrap().push(json!({
            "id": "shutter_b",
            "kind": "shutter",
            "target": { "x": 1216, "y": 512, "w": 32, "h": 68 },
            "shutter": { "initial": "closed" }
        }));
        let errs = errors(&v);
        assert!(has(&errs, |e| matches!(
            e,
            EncounterError::ShutterOverlaps { prop, with }
                if prop == "shutter" && with.contains("shutter_b")
        )));
        // A remnant: a landed weight left on the floor under the arch.
        let mut v = sidecar();
        v["props"].as_array_mut().unwrap().push(json!({
            "id": "weight",
            "kind": "weight_rig",
            "target": { "x": 1100, "y": 540, "w": 32, "h": 40 },
            "hang_px": 128,
            "remnant": { "x": 1208, "y": 564, "w": 48, "h": 16 }
        }));
        v["hazards"].as_array_mut().unwrap().push(json!({
            "id": 2,
            "name": "weight_fall",
            "class": "falling_mass",
            "owner": "weight",
            "rect": { "x": 1200, "y": 484, "w": 64, "h": 96 },
            "telegraph_rect": { "x": 1200, "y": 572, "w": 64, "h": 8 },
            "telegraph_ticks": 24,
            "active_ticks": 1,
            "max_activations": 1,
            "damage": 1,
            "push_x": 0,
            "hero_damage": "all_heroes",
            "hurts_enemies": true
        }));
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::ShutterOverlaps { with, .. } if with.starts_with("the remnant of prop weight")
        )));
        // A lock solid.
        let mut v = sidecar();
        v["locks"] = json!([{
            "id": "door",
            "solid": { "x": 1216, "y": 484, "w": 16, "h": 96 },
            "opens_on": { "event": "enemy_defeated", "target": "g1" }
        }]);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::ShutterOverlaps { with, .. } if with == "the solid of lock door"
        )));
        // Touching is not overlapping: a lock flush against the shutter's edge.
        let mut v = sidecar();
        v["locks"] = json!([{
            "id": "door",
            "solid": { "x": 1232, "y": 484, "w": 16, "h": 96 },
            "opens_on": { "event": "enemy_defeated", "target": "g1" }
        }]);
        assert!(!has(&errors_or_empty(&v), |e| matches!(
            e,
            EncounterError::ShutterOverlaps { .. }
        )));
    }

    fn errors_or_empty(v: &Value) -> Vec<EncounterError> {
        result(v).err().unwrap_or_default()
    }

    #[test]
    fn a_floor_node_must_touch_each_side_of_the_closed_shutter() {
        // A 160 px drum flush against the left side: while it stands it is
        // wider than Carl's reach, so it cuts his floor and no floor node
        // touches the shutter's left side (state 0b01: closed, drum intact).
        let mut v = sidecar();
        with_drum(&mut v, 160);
        v["props"][1]["target"] = json!({ "x": 1040, "y": 580 - 48, "w": 160, "h": 48 });
        v["hazards"][0]["rect"] = json!({ "x": 1000, "y": 500, "w": 96, "h": 80 });
        let errs = errors(&v);
        assert!(
            has(&errs, |e| matches!(
                e,
                EncounterError::ShutterNoFloorNode {
                    hero: ReachHero::Carl,
                    state: 0b01,
                    side: ShutterSide::Left,
                    ..
                }
            )),
            "{errs:?}"
        );
        // Base Donut hops the drum, so her left floor node is still there.
        assert!(!has(&errs, |e| matches!(
            e,
            EncounterError::ShutterNoFloorNode {
                hero: ReachHero::DonutBase,
                ..
            }
        )));
        // The court itself has both nodes.
        assert!(result(&sidecar()).is_ok());
    }

    #[test]
    fn the_shutter_rule_composes_with_the_static_solids_of_the_sidecar() {
        // HRD-A1 / HAZ-A1 statics: a 160 px solid (wider than Carl's reach)
        // flush against the shutter's left side takes the left floor node of the
        // closed state away for Carl, so the per-side proof sees it.
        let mut v = sidecar();
        v["statics"] = json!([{
            "id": "wall",
            "kind": "solid",
            "rect": { "x": 1040, "y": 580 - 48, "w": 160, "h": 48 }
        }]);
        let errs = errors(&v);
        assert!(
            has(&errs, |e| matches!(
                e,
                EncounterError::ShutterNoFloorNode {
                    hero: ReachHero::Carl,
                    state: 0b1,
                    side: ShutterSide::Left,
                    ..
                }
            )),
            "{errs:?}"
        );
        // A static that overlaps the closed shutter is refused like any solid.
        let mut v = sidecar();
        v["statics"] = json!([{
            "id": "wall",
            "kind": "solid",
            "rect": { "x": 1216, "y": 580 - 32, "w": 32, "h": 32 }
        }]);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::ShutterOverlaps { with, .. } if with == "the static wall"
        )));
        // A low static far from the arch and the shelf changes nothing.
        let mut v = sidecar();
        v["statics"] = json!([{
            "id": "wall",
            "kind": "solid",
            "rect": { "x": 1900, "y": 580 - 32, "w": 32, "h": 32 }
        }]);
        assert!(result(&v).is_ok(), "{:?}", result(&v));
    }

    #[test]
    fn a_shutter_owns_no_hazard_and_takes_no_link() {
        let mut v = sidecar();
        v["hazards"].as_array_mut().unwrap().push(json!({
            "id": 1,
            "name": "scrape",
            "class": "blast",
            "owner": "shutter",
            "rect": { "x": 1200, "y": 512, "w": 32, "h": 68 },
            "telegraph_ticks": 24,
            "active_ticks": 1,
            "max_activations": 1,
            "damage": 1,
            "push_x": 0,
            "hero_damage": "all_heroes",
            "hurts_enemies": true
        }));
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::HazardOwnerMismatch { prop, reason }
                if prop == "shutter" && reason.contains("owns no hazard")
        )));
        // A link into the shutter (the drum's blast reaches it) is refused.
        let mut v = sidecar();
        with_drum(&mut v, 32);
        v["props"][1]["target"] = json!({ "x": 1168, "y": 580 - 48, "w": 32, "h": 48 });
        v["hazards"][0]["rect"] = json!({ "x": 1152, "y": 500, "w": 96, "h": 80 });
        v["links"] = json!([{ "from": "drum", "to": "shutter", "kind": "blast" }]);
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::LinkShutter(p) if p == "shutter"
        )));
    }

    #[test]
    fn a_closed_shutter_may_not_seal_the_spawn_the_checkpoint_or_the_exit() {
        // The sidecar's checkpoint moved onto the arch.
        let mut v = sidecar();
        v["checkpoint"] = json!({ "x": 1190, "y": 484, "w": 48, "h": 96 });
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::PropOnProtected {
                zone: "checkpoint",
                ..
            }
        )));
    }

    #[test]
    fn the_lane_cut_waiver_covers_the_closed_shutter_and_nothing_else() {
        // The court's closed state cuts the lane for every class, and that is
        // not an error: the same state with the shutter open keeps the lane.
        let l = loaded();
        assert!(l.encounter.reach.iter().any(|r| !r.lane && r.state == 1));
        // A 160 px drum (wider than Carl's reach) cuts the lane on its own in
        // every state: the waiver does not hide it, in the open state or the
        // closed one.
        let mut v = sidecar();
        with_drum(&mut v, 160);
        let errs = errors(&v);
        // Changeable props: the shutter is bit 0, the drum bit 1 (set = used
        // up / closed): states 0b00 (open) and 0b01 (closed) keep the drum.
        for state in [0b00, 0b01] {
            assert!(
                has(&errs, |e| *e
                    == EncounterError::LaneCut {
                        hero: ReachHero::Carl,
                        state
                    }),
                "state {state:#b}: {errs:?}"
            );
        }
        // With the drum used up (0b10) and the shutter open, nothing is cut;
        // with it closed (0b11) the shutter alone cuts it: waived.
        for state in [0b10, 0b11] {
            assert!(
                !has(&errs, |e| *e
                    == EncounterError::LaneCut {
                        hero: ReachHero::Carl,
                        state
                    }),
                "state {state:#b}: {errs:?}"
            );
        }
    }

    #[test]
    fn the_closed_shutter_is_part_of_every_validator_graph() {
        // A lock closed behind the shutter: the locked-target check runs the
        // same graphs, so it must see the closed shutter as a cut and the
        // walkway as the crossing (no error here), while the target is still
        // reachable. Without the crossing the same check fails.
        let mut v = sidecar();
        v["locks"] = json!([{
            "id": "door",
            "solid": { "x": 1960, "y": 484, "w": 8, "h": 96 },
            "opens_on": { "event": "enemy_defeated", "target": "g1" }
        }]);
        assert!(result(&v).is_ok(), "{:?}", result(&v));
        // Strict Carl (reliable rise 72) with the step removed: the closed
        // shutter walls him off from g1 on the far side: the locked-target
        // graph sees the shutter cut too.
        let l = loaded();
        let text = std::fs::read_to_string(root().join(HEIGHT_CLASSES_PATH)).unwrap();
        let strict = HeightClasses::parse(&text.replace(
            "\"carl\": { \"max_rise_px\": 80,",
            "\"carl\": { \"max_rise_px\": 72,",
        ))
        .unwrap();
        let mut room = l.room.clone();
        room.platforms.remove(0);
        v["gates"] = json!([
            { "surface": 0, "class": "donut_base" },
            { "surface": 1, "class": "donut_boosted" }
        ]);
        v["vantages"] = json!([{ "surface": 1, "counter": "lane_ceiling" }]);
        let errs = parse_sidecar(
            &v.to_string(),
            &Context {
                room,
                classes: strict,
                ..ctx()
            },
        )
        .unwrap_err();
        assert!(
            has(&errs, |e| matches!(
                e,
                EncounterError::LockTargetUnreachable {
                    hero: ReachHero::Carl,
                    state: 1,
                    ..
                }
            )),
            "{errs:?}"
        );
    }
}
