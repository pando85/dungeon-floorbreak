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
//!   stays open, every perch the hero's class can reach is reachable and has a
//!   way back, and both spawns regroup. The classes (Carl 80 px, base Donut
//!   112 px, boosted Donut 128 px, their reach, the drop gap, Carl's height)
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
//! Disclosed limits: the reachability uses the height classes and flat-jump
//! distances of the data file, not the real jump physics, and it does not trust
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
use crate::env_hazard::{HazardClass, HazardSet, HazardSetError, HazardSpec, TeamPolicy};
use crate::env_prop::ValveConfig;

pub const SCHEMA: &str = "floor01-encounter-v1";
pub const BOILER_ENCOUNTER_PATH: &str = "assets/levels/floor01/f01_boiler.encounter.json";
pub const BOILER_ROOM_PATH: &str = "assets/levels/floor01/f01_boiler.room.json";
pub const HEIGHT_CLASSES_PATH: &str = "assets/data/gameplay/height_classes_v0.json";
pub const HEIGHT_CLASSES_SCHEMA: &str = "floor01-height-classes-v1";

/// K2 / #176: at most this many authored prop-to-prop links, acyclic.
pub const MAX_AUTHORED_LINKS: usize = 3;
/// #176 B3: at most this many living enemies in the first probe.
pub const MAX_LIVING_FIRST_PROBE: u8 = 4;
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
}

impl PropKind {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "valve" => Self::Valve,
            "weight_rig" => Self::WeightRig,
            "drum" => Self::Drum,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Goal {
    ReachExit,
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hazard {
    pub name: String,
    /// Index into [`Encounter::props`].
    pub owner: usize,
    pub spec: HazardSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    pub from: usize,
    pub to: usize,
    pub kind: LinkKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterEntry {
    pub id: String,
    pub kind: EnemyKind,
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
    /// Per platform: `Some((reachable, can_return))` if the class reaches its
    /// height, `None` if the surface is above the class.
    pub perches: Vec<Option<(bool, bool)>>,
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
    /// The reach matrix the validation computed.
    pub reach: Vec<ReachRow>,
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
            .find(|h| h.owner == owner && h.spec.class == class)
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
            UnknownGoal(g) => write!(f, "unknown goal {g:?} (only reach_exit)"),
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
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHazard {
    id: u16,
    name: String,
    class: String,
    owner: String,
    rect: RawRect,
    telegraph_rect: Option<RawRect>,
    telegraph_ticks: u16,
    active_ticks: u16,
    max_activations: u8,
    damage: u8,
    push_x: i16,
    policy: String,
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
        .map(|n| n as i32)
}

/// Parse sidecar text against an already parsed context and validate it.
pub fn parse_sidecar(text: &str, ctx: &Context) -> Result<Encounter, Vec<EncounterError>> {
    let raw: RawEncounter =
        serde_json::from_str(text).map_err(|e| vec![EncounterError::Parse(e.to_string())])?;
    validate(&raw, ctx)
}

// --------------------------------------------------------------- validation

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
    let goal = match raw.goal.as_str() {
        "reach_exit" => Goal::ReachExit,
        other => {
            errs.push(E::UnknownGoal(other.into()));
            Goal::ReachExit
        }
    };
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
        } else if room.floor_y - target.y > carl_rise {
            errs.push(E::PropOutOfReach(p.id.clone()));
        }
        let field = |reason: &str| E::PropFields {
            prop: p.id.clone(),
            reason: reason.into(),
        };
        match kind {
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
        }
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
            let blocks = (p.solid_intact && overlaps(target, rect))
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
        let Some(policy) = TeamPolicy::parse(&h.policy) else {
            errs.push(E::HazardPolicy(h.policy.clone()));
            continue;
        };
        let Some(&owner) = prop_pos.get(h.owner.as_str()) else {
            errs.push(E::HazardUnknownOwner {
                hazard: h.name.clone(),
                owner: h.owner.clone(),
            });
            continue;
        };
        let rect = IRect::from(&h.rect);
        let spec = HazardSpec {
            id: h.id,
            class,
            rect,
            telegraph_rect: h.telegraph_rect.as_ref().map_or(rect, IRect::from),
            telegraph_ticks: h.telegraph_ticks,
            active_ticks: h.active_ticks,
            max_activations: h.max_activations,
            damage: h.damage,
            push_x: h.push_x,
            policy,
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
            PropKind::Valve => HazardClass::Steam,
            PropKind::WeightRig => HazardClass::FallingMass,
            PropKind::Drum => HazardClass::Blast,
        };
        let owned = hazards
            .iter()
            .filter(|h| h.owner == pos && h.spec.class == want)
            .count();
        if owned != 1 {
            errs.push(E::HazardOwnerMismatch {
                prop: raw_prop.id.clone(),
                reason: format!("needs exactly one {} hazard, has {owned}", want.name()),
            });
        }
        if kind == PropKind::Valve {
            let steam = hazards
                .iter()
                .find(|h| h.owner == pos && h.spec.class == HazardClass::Steam);
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
                warning_ticks: steam.map_or(d.warning_ticks, |h| h.spec.telegraph_ticks),
                vent_ticks: steam.map_or(d.vent_ticks, |h| h.spec.active_ticks),
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
                    .find(|h| h.owner == pos && h.spec.class == HazardClass::FallingMass),
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
                h.owner == from
                    && h.spec.class == HazardClass::Steam
                    && overlaps(h.spec.rect, target)
            }),
            LinkKind::Landing => hazards.iter().any(|h| {
                h.owner == from
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
    if raw.roster.len() > usize::from(raw.max_living.min(MAX_LIVING_FIRST_PROBE)) {
        errs.push(E::TooManyLiving {
            roster: raw.roster.len(),
            max: raw.max_living.min(MAX_LIVING_FIRST_PROBE),
        });
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
        let allowed = |k: EnemyKind| matches!(k, EnemyKind::Goblin | EnemyKind::Engineer);
        let kind = match EnemyKind::parse(&r.kind).filter(|k| allowed(*k)) {
            Some(k) => k,
            None => {
                errs.push(E::RosterKind {
                    entry: r.id.clone(),
                    kind: r.kind.clone(),
                });
                continue;
            }
        };
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
        let body = IRect::new(
            r.x - ENEMY_BODY.0 / 2,
            top - ENEMY_BODY.1,
            ENEMY_BODY.0,
            ENEMY_BODY.1,
        );
        if body.x < room.play_x.0 || body.right() > room.play_x.1 {
            errs.push(E::RosterOutOfBounds(r.id.clone()));
        }
        if overlaps(body, protected_spawn) {
            errs.push(E::RosterOnProtected(r.id.clone()));
        }
        for h in &hazards {
            if overlaps(body, h.spec.rect) || overlaps(body, h.spec.telegraph_rect) {
                errs.push(E::RosterInHazard {
                    entry: r.id.clone(),
                    hazard: h.name.clone(),
                });
            }
        }
        for s in &r.threatens {
            if *s >= platform_count {
                errs.push(E::GateUnknownSurface(*s));
            }
        }
        roster.push(RosterEntry {
            id: r.id.clone(),
            kind,
            gate: r.gate.clone(),
            fallback,
            x: r.x,
            perch: r.perch,
            beat: r.beat.clone(),
            trigger_x: r.trigger_x,
            delay_ticks: r.delay_ticks,
            threatens: r.threatens.clone(),
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
            Some(c) => vantages.push((v.surface, c)),
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

    // ---- reachability, in every destruction state
    let changeable: Vec<usize> = props
        .iter()
        .enumerate()
        .filter(|(_, p)| p.solid_intact || p.remnant.is_some())
        .map(|(i, _)| i)
        .collect();
    let mut reach: Vec<ReachRow> = Vec::new();
    if props.len() <= MAX_PROPS {
        reach = reach_matrix(room, classes, &props, &changeable);
        for row in &reach {
            let hero = row.hero;
            let state = row.state;
            if !row.lane && (hero == ReachHero::Carl || !row.exit) {
                errs.push(E::LaneCut { hero, state });
            }
            if !row.exit {
                errs.push(E::ExitUnreachable { hero, state });
            }
            if !row.regroup {
                errs.push(E::RegroupBlocked { hero, state });
            }
            for (surface, perch) in row.perches.iter().enumerate() {
                match perch {
                    Some((false, _)) => errs.push(E::PerchUnreachable {
                        hero,
                        state,
                        surface,
                    }),
                    Some((true, false)) => errs.push(E::PerchNoReturn {
                        hero,
                        state,
                        surface,
                    }),
                    _ => {}
                }
            }
        }
    }

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
            reach,
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
}

impl Graph {
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

fn platform_end_free(room: &Room, drop_gap: i32, r: IRect) -> bool {
    r.x - drop_gap >= room.play_x.0 || r.right() + drop_gap <= room.play_x.1
}

/// Floor segments of one hero class (cut by blockers it cannot hop) plus the
/// platforms.
fn nodes_for(room: &Room, c: &HeightClasses, hero: ReachHero, solids: &[IRect]) -> Graph {
    let mut cuts: Vec<(i32, i32)> = solids
        .iter()
        .filter(|b| room.floor_y - b.y > c.max_rise(hero) || b.w > c.reach_x(hero))
        .map(|b| (b.x, b.right()))
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
    for p in &room.platforms {
        nodes.push(Node {
            x0: p.rect.x,
            x1: p.rect.right(),
            top: p.rect.y,
            floor: false,
        });
    }
    Graph { nodes, cuts }
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

/// The reach matrix: every hero class in every destruction state of the
/// changeable props (`2^n` states).
pub fn reach_matrix(
    room: &Room,
    c: &HeightClasses,
    props: &[Prop],
    changeable: &[usize],
) -> Vec<ReachRow> {
    let exit_x = room.goal_x.min(room.exit.right() - 1);
    let mut rows = Vec::new();
    for hero in ReachHero::ALL {
        for state in 0..(1u16 << changeable.len()) {
            let solids = blockers(props, changeable, state);
            let graph = nodes_for(room, c, hero, &solids);
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
                .map(|(i, p)| {
                    let rise = room.floor_y - p.rect.y;
                    if rise > c.max_rise(hero) {
                        return None;
                    }
                    let node = nodes.len() - room.platforms.len() + i;
                    let there =
                        spawn.is_some_and(|s| reachable_from(room, c, hero, &graph, s)[node]);
                    let back = goal.is_some_and(|g| reachable_from(room, c, hero, &graph, node)[g]);
                    Some((there, back))
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
        assert_eq!(e.story_pin, "f01_pin_post_hoarder");
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
    const SIDECAR_HASH: u64 = 13_332_603_978_203_026_341;

    #[test]
    fn the_data_file_pins_the_three_classes() {
        // These numbers are the data file's, not the module's: a value that
        // moves in the jump specs moves in the file and in this one test.
        let c = ctx().classes;
        assert_eq!(c.gate_class(80), Some(ReachHero::Carl));
        assert_eq!(c.gate_class(81), Some(ReachHero::DonutBase));
        assert_eq!(c.gate_class(112), Some(ReachHero::DonutBase));
        assert_eq!(c.gate_class(113), Some(ReachHero::DonutBoosted));
        assert_eq!(c.gate_class(128), Some(ReachHero::DonutBoosted));
        assert_eq!(c.gate_class(129), None);
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
        let moved = text.replace("\"max_rise_px\": 112", "\"max_rise_px\": 120");
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
    fn bad_class_data_is_rejected() {
        let text = std::fs::read_to_string(root().join(HEIGHT_CLASSES_PATH)).unwrap();
        for bad in [
            text.replace("\"max_rise_px\": 112", "\"max_rise_px\": 60"),
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
                assert_eq!(*perch, Some((true, true)), "{row:?}");
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
            assert!(v.spec().policy.hurts(crate::env_hazard::Team::Hero));
        }
        let steam = e
            .owned_hazard(e.prop_index("valve").unwrap(), HazardClass::Steam)
            .unwrap();
        assert_eq!(
            (steam.spec.telegraph_ticks, steam.spec.active_ticks),
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
        v["hazards"][0]["telegraph_ticks"] = json!(6);
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
        let mut v = sidecar();
        v["roster"][0]["kind"] = json!("llama");
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
        let mut v = sidecar();
        v["roster"].as_array_mut().unwrap().push(
            json!({ "id": "g9", "kind": "goblin", "x": 400, "beat": "B3", "delay_ticks": 500 }),
        );
        assert!(has(&errors(&v), |e| matches!(
            e,
            EncounterError::TooManyLiving { roster: 5, max: 4 }
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
        // An enemy threatens it from the lane.
        let v = sidecar_for(&room, &[(0, "donut_base")], |v| {
            v["roster"][0]["threatens"] = json!([0]);
        });
        assert!(parse_sidecar(&v.to_string(), &ctx_for(room.clone())).is_ok());
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
        // Two stacked perches: 80 px, then 128 px beside it. Carl reaches the
        // first only, the boosted Donut both: the matrix says so.
        let room = room_with_platforms(&[(304, 80, 144), (480, 128, 144)]);
        let v = sidecar_for(&room, &[(0, "carl"), (1, "donut_boosted")], |v| {
            v["roster"][0]["perch"] = json!(1);
            v["roster"][0]["x"] = json!(552);
        });
        let enc = parse_sidecar(&v.to_string(), &ctx_for(room.clone()))
            .unwrap_or_else(|e| panic!("{e:?}"));
        let by = |hero, state| {
            enc.reach
                .iter()
                .find(|r| r.hero == hero && r.state == state)
                .unwrap()
        };
        assert_eq!(by(ReachHero::Carl, 0).perches, [Some((true, true)), None]);
        assert_eq!(
            by(ReachHero::DonutBase, 0).perches,
            [Some((true, true)), None]
        );
        assert_eq!(
            by(ReachHero::DonutBoosted, 0).perches,
            [Some((true, true)), Some((true, true))]
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

    #[test]
    fn dist2_is_exact() {
        let r = IRect::new(10, 10, 10, 10);
        assert_eq!(dist2_point_rect(15, 15, r), 0);
        assert_eq!(dist2_point_rect(5, 15, r), 25);
        assert_eq!(dist2_point_rect(25, 25, r), 25 + 25);
    }
}
