//! The manifest: one descriptor per room (LAB-00 "exact behaviors").
//!
//! For each room: id, layout/config/content hash, seed and variant, allowed
//! hero modes, legal capability profile, encounter and wave ids, hazard and
//! terrain ids, checkpoints, camera constraints, goal and the recording
//! segment boundaries. **There is no difficulty field** (LAB-Q08): every type
//! is `deny_unknown_fields`, so a `difficulty` key is a parse error.

use serde::{Deserialize, Serialize};

use super::ability::{AbilityStatus, classify};
use super::profile::{CapabilityProfile, ProfileSpec};
use super::{LAB_MAX_CONCURRENT, LabError, MANIFEST_SCHEMA, ROOM_IDS, fnv64};

/// The player modes a room may allow (`sandbox` and `stress` are probe tags,
/// see [`crate::combat_lab::ProbeTag`]).
pub const MODE_IDS: [&str; 3] = ["solo_carl", "solo_donut", "coop_local"];
/// The kit views the scene camera reads.
pub const VIEW_IDS: [&str; 2] = ["abc_640x360", "abc_640x400"];
/// What the party separation wall allows at a 640 px view (Q7, LAB-Q11).
pub const SEPARATION_PX: u16 = 448;
/// Segment boundary ids.
pub const SEGMENT_BEGIN: [&str; 1] = ["room_enter"];
///
/// There is no `defeat` end: the simulation restarts itself on a party defeat,
/// which stays inside the episode and is counted in its `deaths` (LAB-00-F).
pub const SEGMENT_END: [&str; 3] = ["clear", "retry", "abandon"];
/// Capabilities a room may declare as wanted but not required (gated work).
pub const OPTIONAL_CAPABILITIES: [(&str, &str); 4] = [
    ("terrain_l1_destructible_cover", "TERRAIN-L1"),
    ("terrain_l1_destructible_bridge", "TERRAIN-L1"),
    ("terrain_l3_explosive_trap", "TERRAIN-L3"),
    ("cf02_f_wave_contract", "CF02-F"),
];

/// One enemy of a roster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemySpec {
    /// `goblin` or `llama` (the roster with gameplay on main).
    pub kind: String,
    /// Centre column; the enemy stands on the walking line.
    pub x: i32,
    /// `chase` or `passive`.
    pub ai: String,
}

/// One authored hazard. Only `rotten_shore` exists (LAB-Q02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HazardSpec {
    pub id: String,
    pub kind: String,
    /// The strike column of the shore (`Fixture::shore_dx` is relative to
    /// Carl's start column).
    pub x: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalKind {
    /// No goal (R00, the selection room).
    None,
    /// A hero reaches the protected exit.
    ReachExit,
    /// Every goblin and llama is defeated.
    ClearEnemies,
    /// The goal belongs to the linked scene (R06).
    Link,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalSpec {
    pub kind: GoalKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraSpec {
    pub views: Vec<String>,
    pub max_separation_px: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SegmentSpec {
    pub id: String,
    pub begin: String,
    pub end: Vec<String>,
}

/// An existing scene a room links instead of copying (R06, LAB-Q09).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkSpec {
    pub scene: String,
    pub arena: String,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariantKind {
    Play,
    HeldoutGeometry,
}

/// A variant of a room (LAB-Q07: R01 and R05 only). It may replace the
/// roster, the seed and (held-out geometry) the room file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariantSpec {
    pub id: String,
    pub kind: VariantKind,
    pub seed: Option<u64>,
    pub room_file: Option<String>,
    pub layout_hash: Option<String>,
    pub enemies: Option<Vec<EnemySpec>>,
    /// Carl's start columns of a variant with its own geometry (the held-out
    /// R01H starts on the east side).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoints: Option<Vec<i32>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomSpec {
    pub id: String,
    pub name: String,
    /// A design hypothesis (LAB-Q05), not a timer.
    pub aim_seconds: Option<[u32; 2]>,
    pub room_file: Option<String>,
    /// FNV-1a 64 of the room file, 16 hex digits.
    pub layout_hash: Option<String>,
    pub link: Option<LinkSpec>,
    pub encounter_id: String,
    pub wave_ids: Vec<String>,
    pub allowed_modes: Vec<String>,
    pub profiles: Vec<String>,
    pub required_moves: Vec<String>,
    /// Carl's start columns; the first is the room start.
    pub checkpoints: Vec<i32>,
    pub enemies: Vec<EnemySpec>,
    pub hazards: Vec<HazardSpec>,
    pub optional_capabilities: Vec<String>,
    pub max_concurrent: usize,
    pub camera: CameraSpec,
    pub goal: GoalSpec,
    pub segment: SegmentSpec,
    /// The fixture seed; the CF02-Q58 effective seed overrides it (LAB-Q06).
    pub seed: u64,
    /// The default length of a headless run, in ticks.
    pub ticks: u64,
    /// Checks that need an integration fixture or an unmerged dependency.
    pub validation_pending: Vec<String>,
    pub variants: Option<Vec<VariantSpec>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub status: String,
    pub profiles: Vec<ProfileSpec>,
    pub rooms: Vec<RoomSpec>,
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, LabError> {
        serde_json::from_str(text).map_err(|e| LabError::Parse(e.to_string()))
    }
}

/// A room with one variant picked: what the validator and the runner use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved<'a> {
    pub room: &'a RoomSpec,
    pub variant: String,
    pub kind: VariantKind,
    pub room_file: &'a str,
    pub layout_hash: u64,
    pub enemies: &'a [EnemySpec],
    pub checkpoints: &'a [i32],
    pub seed: u64,
}

impl Resolved<'_> {
    /// The hash of the descriptor of this room and variant (the roster, the
    /// hazards, the seed and every rule).
    pub fn config_hash(&self) -> u64 {
        let body = serde_json::to_string(&(self.room, &self.variant, self.enemies, self.seed))
            .unwrap_or_default();
        fnv64(body.as_bytes())
    }

    /// Layout and config together: the content hash of the episode.
    pub fn content_hash(&self) -> u64 {
        fnv64(format!("{:016x}{:016x}", self.layout_hash, self.config_hash()).as_bytes())
    }
}

pub fn parse_hash(text: &str) -> Option<u64> {
    (text.len() == 16).then(|| u64::from_str_radix(text, 16).ok())?
}

impl RoomSpec {
    /// The variant ids of this room (the implicit one is `v1`).
    pub fn variant_ids(&self) -> Vec<&str> {
        match &self.variants {
            Some(list) => list.iter().map(|v| v.id.as_str()).collect(),
            None => vec!["v1"],
        }
    }

    /// Pick a variant. A linked room has no variants to resolve.
    pub fn resolve(&self, variant: &str) -> Result<Resolved<'_>, LabError> {
        let at = || format!("room {}", self.id);
        let Some(room_file) = self.room_file.as_deref() else {
            return Err(LabError::Schema {
                at: at(),
                reason: "a linked room has no room file".into(),
            });
        };
        let layout = self
            .layout_hash
            .as_deref()
            .and_then(parse_hash)
            .ok_or_else(|| LabError::Schema {
                at: at(),
                reason: "layout_hash must be 16 hex digits".into(),
            })?;
        let default = |kind| Resolved {
            room: self,
            variant: variant.to_owned(),
            kind,
            room_file,
            layout_hash: layout,
            enemies: &self.enemies,
            checkpoints: &self.checkpoints,
            seed: self.seed,
        };
        let Some(list) = &self.variants else {
            return if variant == "v1" {
                Ok(default(VariantKind::Play))
            } else {
                Err(LabError::Unknown {
                    what: format!("variant of {}", self.id),
                    id: variant.into(),
                })
            };
        };
        let v = list
            .iter()
            .find(|v| v.id == variant)
            .ok_or_else(|| LabError::Unknown {
                what: format!("variant of {}", self.id),
                id: variant.into(),
            })?;
        let mut out = default(v.kind);
        if let Some(file) = &v.room_file {
            out.room_file = file;
            out.layout_hash = v
                .layout_hash
                .as_deref()
                .and_then(parse_hash)
                .ok_or_else(|| LabError::Schema {
                    at: format!("room {} variant {}", self.id, v.id),
                    reason: "layout_hash must be 16 hex digits".into(),
                })?;
        }
        if let Some(enemies) = &v.enemies {
            out.enemies = enemies;
        }
        if let Some(checkpoints) = &v.checkpoints {
            out.checkpoints = checkpoints;
        }
        if let Some(seed) = v.seed {
            out.seed = seed;
        }
        Ok(out)
    }
}

/// An enemy kind of the roster with gameplay on main.
pub fn enemy_status(kind: &str) -> Result<(), &'static str> {
    match kind {
        "goblin" | "llama" => Ok(()),
        "rat" | "bomb_bard" | "troglodyte" | "scatterer" | "rot_sticker" | "slime" => {
            Err("approved art only, no gameplay on main; a LAB-04 ENEMY-GAMEPLAY item (LAB-Q14)")
        }
        "juicer" => Err("the boss is the linked room R06, never a roster entry"),
        _ => Err(""),
    }
}

fn is_known_enemy(kind: &str) -> bool {
    matches!(
        kind,
        "goblin"
            | "llama"
            | "rat"
            | "bomb_bard"
            | "troglodyte"
            | "scatterer"
            | "rot_sticker"
            | "slime"
            | "juicer"
    )
}

/// Structural validation of the whole manifest (no files are read). The
/// geometry checks are [`crate::combat_lab::validate`].
pub fn validate_manifest(manifest: &Manifest) -> Result<Vec<CapabilityProfile>, LabError> {
    let schema = |at: &str, reason: &str| LabError::Schema {
        at: at.into(),
        reason: reason.into(),
    };
    if manifest.schema != MANIFEST_SCHEMA {
        return Err(schema("manifest", "unknown schema id"));
    }
    let mut profiles = Vec::new();
    for spec in &manifest.profiles {
        profiles.push(CapabilityProfile::validate(spec.clone())?);
    }
    if profiles.is_empty() {
        return Err(schema("manifest", "no capability profile"));
    }
    let ids: Vec<&str> = manifest.rooms.iter().map(|r| r.id.as_str()).collect();
    if ids != ROOM_IDS {
        return Err(schema("manifest", "rooms must be R00 to R06 in order"));
    }
    for room in &manifest.rooms {
        validate_room(room, &profiles)?;
    }
    Ok(profiles)
}

fn validate_room(room: &RoomSpec, profiles: &[CapabilityProfile]) -> Result<(), LabError> {
    let at = format!("room {}", room.id);
    let schema = |reason: &str| LabError::Schema {
        at: at.clone(),
        reason: reason.into(),
    };
    if room.link.is_some() == room.room_file.is_some() {
        return Err(schema(
            "a room has either a room_file or a link, never both or neither",
        ));
    }
    if room.room_file.is_some() != room.layout_hash.is_some() {
        return Err(schema("room_file and layout_hash go together"));
    }
    if let Some(hash) = &room.layout_hash
        && parse_hash(hash).is_none()
    {
        return Err(schema("layout_hash must be 16 hex digits"));
    }
    if let Some(file) = &room.room_file {
        check_file_name(&at, file)?;
    }
    if room.link.is_some() && room.goal.kind != GoalKind::Link
        || room.link.is_none() && room.goal.kind == GoalKind::Link
    {
        return Err(schema("goal kind link goes with a link, and only with one"));
    }
    if room.allowed_modes.is_empty() {
        return Err(schema("allowed_modes is empty"));
    }
    for mode in &room.allowed_modes {
        if !MODE_IDS.contains(&mode.as_str()) {
            return Err(LabError::Unknown {
                what: "player mode".into(),
                id: mode.clone(),
            });
        }
    }
    if room.profiles.is_empty() {
        return Err(schema("no legal capability profile"));
    }
    let mut room_profiles = Vec::new();
    for id in &room.profiles {
        let profile = profiles
            .iter()
            .find(|p| p.id() == id)
            .ok_or_else(|| LabError::Unknown {
                what: "capability profile".into(),
                id: id.clone(),
            })?;
        room_profiles.push(profile);
    }
    // Every move a room needs must exist and be legal in every profile it allows.
    for id in &room.required_moves {
        match classify(id) {
            AbilityStatus::Unknown => {
                return Err(LabError::UnknownAbility {
                    at: at.clone(),
                    id: id.clone(),
                });
            }
            AbilityStatus::Unimplemented(reason) => {
                return Err(LabError::UnimplementedAbility {
                    at: at.clone(),
                    id: id.clone(),
                    reason: reason.into(),
                });
            }
            AbilityStatus::Implemented(_) | AbilityStatus::Cf02Gated(_) => {
                for profile in &room_profiles {
                    if !profile.allows_move(id) {
                        return Err(LabError::IllegalAbility {
                            at: at.clone(),
                            id: id.clone(),
                            reason: format!("the profile {} does not allow it", profile.id()),
                        });
                    }
                }
            }
        }
    }
    if room.checkpoints.is_empty() {
        return Err(schema("at least one checkpoint (the room start)"));
    }
    if room.max_concurrent == 0 || room.max_concurrent > LAB_MAX_CONCURRENT {
        return Err(schema("max_concurrent must be 1 to 4 (LAB-Q04)"));
    }
    if room.ticks == 0 && room.link.is_none() {
        return Err(schema("ticks must be positive"));
    }
    if room.camera.max_separation_px != SEPARATION_PX {
        return Err(schema(
            "max_separation_px must be the party wall of 448 px (Q7, LAB-Q11)",
        ));
    }
    if room.camera.views.is_empty()
        || room
            .camera
            .views
            .iter()
            .any(|v| !VIEW_IDS.contains(&v.as_str()))
    {
        return Err(schema(
            "camera views must be abc_640x360 and/or abc_640x400",
        ));
    }
    if room.segment.begin != SEGMENT_BEGIN[0]
        || room.segment.end.is_empty()
        || room
            .segment
            .end
            .iter()
            .any(|e| !SEGMENT_END.contains(&e.as_str()))
    {
        return Err(schema(
            "segment boundaries must use room_enter and clear/retry/abandon",
        ));
    }
    for cap in &room.optional_capabilities {
        if !OPTIONAL_CAPABILITIES.iter().any(|(id, _)| id == cap) {
            return Err(LabError::Unknown {
                what: "optional capability".into(),
                id: cap.clone(),
            });
        }
    }
    for hazard in &room.hazards {
        match hazard.kind.as_str() {
            "rotten_shore" => {}
            "destructible_cover" | "destructible_bridge" | "explosive_trap" => {
                return Err(schema(&format!(
                    "hazard {:?} is gated on TERRAIN-L1/L3 (LAB-Q02): declare it under optional_capabilities",
                    hazard.kind
                )));
            }
            other => {
                return Err(LabError::Unknown {
                    what: "hazard kind".into(),
                    id: other.into(),
                });
            }
        }
    }
    if room.hazards.len() > 1 {
        return Err(schema(
            "one authored shore per room (the fixture has one shore)",
        ));
    }
    validate_roster(&at, &room.enemies, room.max_concurrent, false)?;
    validate_goal_roster(&at, room.goal.kind, &room.enemies)?;
    // Variants (LAB-Q07): R01 and R05 only, three play variants and one held-out.
    match &room.variants {
        None => {}
        Some(list) => {
            if !matches!(room.id.as_str(), "R01" | "R05") {
                return Err(schema("variants exist for R01 and R05 only (LAB-Q07)"));
            }
            let play = list.iter().filter(|v| v.kind == VariantKind::Play).count();
            let heldout = list
                .iter()
                .filter(|v| v.kind == VariantKind::HeldoutGeometry)
                .count();
            if play != 3 || heldout != 1 {
                return Err(schema(
                    "needs exactly 3 play variants and 1 held-out geometry variant",
                ));
            }
            let mut seen: Vec<&str> = Vec::new();
            for v in list {
                if seen.contains(&v.id.as_str()) {
                    return Err(schema("duplicate variant id"));
                }
                seen.push(&v.id);
                let vat = format!("{at} variant {}", v.id);
                if (v.kind == VariantKind::HeldoutGeometry)
                    != (v.room_file.is_some() && v.layout_hash.is_some())
                {
                    return Err(schema(
                        "a held-out variant has its own room_file and layout_hash, a play variant has none",
                    ));
                }
                if let Some(file) = &v.room_file {
                    check_file_name(&vat, file)?;
                }
                if let Some(hash) = &v.layout_hash
                    && parse_hash(hash).is_none()
                {
                    return Err(schema("variant layout_hash must be 16 hex digits"));
                }
                if let Some(enemies) = &v.enemies {
                    validate_roster(&vat, enemies, room.max_concurrent, false)?;
                    validate_goal_roster(&vat, room.goal.kind, enemies)?;
                }
            }
        }
    }
    Ok(())
}

/// A room file is a plain file name inside the lab directory.
fn check_file_name(at: &str, file: &str) -> Result<(), LabError> {
    if file.is_empty()
        || file.contains(['/', '\\'])
        || file.contains("..")
        || !file.ends_with(".room.json")
    {
        return Err(LabError::Schema {
            at: at.into(),
            reason: format!("room file {file:?} must be a plain *.room.json name"),
        });
    }
    Ok(())
}

/// Validate a roster. `lift_cap` is the `stress` probe (a tagged density run).
pub fn validate_roster(
    at: &str,
    enemies: &[EnemySpec],
    max_concurrent: usize,
    lift_cap: bool,
) -> Result<(), LabError> {
    for enemy in enemies {
        match enemy_status(&enemy.kind) {
            Ok(()) => {}
            Err(reason) if is_known_enemy(&enemy.kind) => {
                return Err(LabError::UnimplementedEnemy {
                    at: at.into(),
                    id: enemy.kind.clone(),
                    reason: reason.into(),
                });
            }
            Err(_) => {
                return Err(LabError::UnknownEnemy {
                    at: at.into(),
                    id: enemy.kind.clone(),
                });
            }
        }
        if !matches!(enemy.ai.as_str(), "chase" | "passive") {
            return Err(LabError::Schema {
                at: at.into(),
                reason: format!("enemy ai {:?} is not chase or passive", enemy.ai),
            });
        }
    }
    if !lift_cap && enemies.len() > max_concurrent {
        return Err(LabError::Schema {
            at: at.into(),
            reason: format!(
                "{} enemies exceed the cap of {max_concurrent} concurrently active enemies (LAB-Q04); only a tagged stress probe lifts it",
                enemies.len()
            ),
        });
    }
    Ok(())
}

/// A `clear_enemies` goal needs at least one goblin: the clear predicate
/// ([`crate::demo::complete`]) is false for a roster without one, so the room
/// could never be cleared.
pub fn validate_goal_roster(
    at: &str,
    goal: GoalKind,
    enemies: &[EnemySpec],
) -> Result<(), LabError> {
    if goal == GoalKind::ClearEnemies && !enemies.iter().any(|e| e.kind == "goblin") {
        return Err(LabError::Schema {
            at: at.into(),
            reason: "a clear_enemies goal needs at least one goblin in the roster, or the room can never clear".into(),
        });
    }
    Ok(())
}
