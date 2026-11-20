//! Geometry, camera and reachability validation of the lab rooms.
//!
//! Every room file is composed with the same composer the demo hall and the
//! boss arena use ([`crate::camera_lab::layout::build_kit`]), then checked:
//! the file hash, the walking line under every spawn, checkpoint and hazard,
//! the two camera views inside the layout, and Carl's and Donut's reachability
//! of the protected exit with the camera-lab mobility numbers
//! ([`crate::camera_lab::room`], the reference approach for LAB-01).
//!
//! What cannot be proven here is reported as an explicit
//! [`CheckStatus::Pending`] (never silently passed): platform reachability
//! needs an integration fixture with the real jump physics, and every
//! dependency that has not merged is named.

use std::path::Path;

use super::descriptor::{GoalKind, Manifest, Resolved, RoomSpec};
use super::{LAB_DIR, LabError, fnv64};
use crate::camera_lab::layout::build_kit;
use crate::camera_lab::room::{CARL_MOBILITY, DONUT_MOBILITY, PropState, Room, parse_rooms};
use crate::env_kit::{EnvKit, ViewDef};
use crate::shore::{FOOTPRINT_WIDTH, FOOTPRINT_X_MIN};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    /// Not checked here; the text says what it needs.
    Pending(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub name: String,
    pub status: CheckStatus,
}

/// What the validator established for one room and variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub room: String,
    pub variant: String,
    pub checks: Vec<Check>,
}

impl ValidationReport {
    pub fn pending(&self) -> Vec<&Check> {
        self.checks
            .iter()
            .filter(|c| matches!(c.status, CheckStatus::Pending(_)))
            .collect()
    }

    fn pass(&mut self, name: &str) {
        self.checks.push(Check {
            name: name.into(),
            status: CheckStatus::Pass,
        });
    }

    fn pending_check(&mut self, name: &str, reason: &str) {
        if self.checks.iter().any(|c| c.name == name) {
            return;
        }
        self.checks.push(Check {
            name: name.into(),
            status: CheckStatus::Pending(reason.into()),
        });
    }
}

/// A composed room: the kit the scene runs on, the camera-lab room it came
/// from (for reachability) and the composer's plan.
pub struct LoadedRoom {
    pub kit: EnvKit,
    pub room: Room,
    pub openings: Vec<(i32, i32, i32, i32)>,
}

/// Read a room file of the lab directory.
pub fn read_room_file(root: &Path, file: &str) -> Result<String, LabError> {
    let path = root.join(LAB_DIR).join(file);
    std::fs::read_to_string(&path).map_err(|e| LabError::Io(format!("{}: {e}", path.display())))
}

/// Pick the scenario of a room file. A file with one scenario is that room; a
/// file with several (R01 holds `LAB-R01-V1` to `LAB-R01-V3`, identical
/// geometry, different placements) is addressed by the variant id: variant
/// `v2` is the scenario whose id ends in `-V2`.
fn pick_scenario(mut rooms: Vec<Room>, variant: &str) -> Result<Room, String> {
    if rooms.len() == 1 {
        return Ok(rooms.remove(0));
    }
    let suffix = format!("-{}", variant.to_uppercase());
    let found: Vec<usize> = rooms
        .iter()
        .enumerate()
        .filter(|(_, r)| r.id.ends_with(&suffix))
        .map(|(i, _)| i)
        .collect();
    match found.as_slice() {
        [index] => Ok(rooms.remove(*index)),
        _ => Err(format!(
            "variant {variant} picks {} of the {} scenarios of the room file",
            found.len(),
            rooms.len()
        )),
    }
}

/// Compose a room file into a kit with the two scroll-camera views of the
/// scene. `goal` decides whether the protected exit stays a trigger;
/// `variant` picks the scenario of a multi-scenario file.
pub fn load_room(
    root: &Path,
    text: &str,
    goal: GoalKind,
    room_id: &str,
    variant: &str,
) -> Result<LoadedRoom, LabError> {
    let geometry = |reason: String| LabError::Geometry {
        room: room_id.into(),
        reason,
    };
    let rooms = parse_rooms(text).map_err(geometry)?;
    let room = pick_scenario(rooms, variant).map_err(geometry)?;
    let (mut kit, _contract, plan) = build_kit(root, &room).map_err(|e| geometry(e.to_string()))?;
    let walk = kit.base_walk_line_y;
    // The same framing as the corridor and the demo hall: the walking line
    // 288 px below the top of the 640x360 view, 308 px below the 640x400 one.
    kit.views.clear();
    for (name, h, below) in [("abc_640x360", 360, 288), ("abc_640x400", 400, 308)] {
        kit.views.insert(
            name.to_owned(),
            ViewDef {
                x: 0,
                y: walk - below,
                w: 640,
                h,
            },
        );
    }
    if goal != GoalKind::ReachExit {
        kit.exit = None;
    }
    Ok(LoadedRoom {
        kit,
        room,
        openings: plan.openings,
    })
}

fn hex(hash: u64) -> String {
    format!("{hash:016x}")
}

/// Validate one room and variant: the hash, the geometry, the camera and the
/// reachability. Errors are rejections; unprovable checks are `Pending`.
pub fn validate_resolved(
    root: &Path,
    resolved: &Resolved<'_>,
) -> Result<ValidationReport, LabError> {
    let room = resolved.room;
    let id = room.id.as_str();
    let geometry = |reason: String| LabError::Geometry {
        room: format!("{id} ({})", resolved.variant),
        reason,
    };
    let mut report = ValidationReport {
        room: room.id.clone(),
        variant: resolved.variant.clone(),
        checks: Vec::new(),
    };
    let text = read_room_file(root, resolved.room_file)?;
    let actual = fnv64(text.as_bytes());
    if actual != resolved.layout_hash {
        return Err(LabError::HashMismatch {
            what: format!("room file {}", resolved.room_file),
            expected: hex(resolved.layout_hash),
            actual: hex(actual),
        });
    }
    report.pass("layout_hash");
    let loaded = load_room(root, &text, room.goal.kind, id, &resolved.variant)?;
    let kit = &loaded.kit;
    report.pass("composes_through_the_existing_kit");

    // Camera: both declared views inside the layout, no void.
    for name in &room.camera.views {
        let view = kit
            .view(name)
            .ok_or_else(|| geometry(format!("the kit has no view {name}")))?;
        if view.y < 0 || view.y + view.h > kit.height || view.w > kit.width {
            return Err(geometry(format!(
                "view {name} ({}x{} at y {}) does not fit the {}x{} layout",
                view.w, view.h, view.y, kit.width, kit.height
            )));
        }
    }
    report.pass("camera_views_inside_the_layout");

    // Lanes: Carl's start, the checkpoints and every enemy on the walking line.
    let walk = kit.base_walk_line_y;
    let lane = |x: i32, what: &str| -> Result<(), LabError> {
        if kit.walking_line_at(x) == Some(walk) {
            Ok(())
        } else {
            Err(geometry(format!(
                "{what} at column {x} is not on the walking line"
            )))
        }
    };
    lane(loaded.room.spawn_carl, "Carl's spawn")?;
    for (i, x) in resolved.checkpoints.iter().enumerate() {
        lane(*x, &format!("checkpoint {i}"))?;
    }
    for (i, e) in resolved.enemies.iter().enumerate() {
        lane(e.x, &format!("{} {i}", e.kind))?;
    }
    report.pass("spawns_on_the_walking_line");

    // The authored shore: a clean flat stretch, and masonry behind it.
    for hazard in &room.hazards {
        let from = hazard.x - 40;
        let to = hazard.x + (FOOTPRINT_X_MIN + FOOTPRINT_WIDTH) as i32 + 40;
        if from < 0 || to >= kit.width {
            return Err(geometry(format!(
                "hazard {} sits outside the layout",
                hazard.id
            )));
        }
        for x in (from..=to).step_by(4) {
            lane(x, &format!("the shore stretch of {}", hazard.id))?;
        }
        let (a, b) = (hazard.x - 8, to - 40);
        if let Some((ox, _, ow, _)) = loaded
            .openings
            .iter()
            .find(|(ox, _, ow, _)| ox + ow > a && *ox < b)
        {
            return Err(geometry(format!(
                "an arch opening ({ox}..{}) sits behind the shore {}",
                ox + ow,
                hazard.id
            )));
        }
        report.pass("shore_footprint_is_flat_masonry");
    }

    // Reachability of the protected exit, intact and fully broken, for both.
    let from = *resolved
        .checkpoints
        .first()
        .ok_or_else(|| geometry("a variant needs at least one checkpoint".into()))?;
    for (who, mobility) in [("Carl", CARL_MOBILITY), ("Donut", DONUT_MOBILITY)] {
        for state in [PropState::AllIntact, PropState::AllBroken] {
            if !loaded.room.exit_reachable(mobility, state, from) {
                return Err(geometry(format!(
                    "{who} cannot reach the exit from column {from} ({state:?})"
                )));
            }
        }
    }
    if !loaded.room.platforms_dropable() {
        return Err(geometry("a platform is walled in".into()));
    }
    report.pass("both_heroes_reach_the_exit");

    // Honest pendings.
    if !loaded.room.platforms.is_empty() {
        report.pending_check(
            "platform_reachability_fixture",
            "the exit check uses the camera-lab mobility numbers; platform use needs an integration fixture on the real jump physics",
        );
    }
    for item in &room.validation_pending {
        report.pending_check(item, "declared pending in the manifest");
    }
    Ok(report)
}

/// Validate the linked room R06: the existing boss arena and fixture.
pub fn validate_link(root: &Path, room: &RoomSpec) -> Result<ValidationReport, LabError> {
    let geometry = |reason: String| LabError::Geometry {
        room: room.id.clone(),
        reason,
    };
    let link = room
        .link
        .as_ref()
        .ok_or_else(|| geometry("no link".into()))?;
    if link.scene != "boss_f01" || link.arena != crate::boss_scene::ARENA_PATH {
        return Err(geometry(
            "the only linked scene is boss_f01 on its existing arena (LAB-Q09)".into(),
        ));
    }
    crate::boss_scene::load_arena(root).map_err(geometry)?;
    let mut report = ValidationReport {
        room: room.id.clone(),
        variant: "v1".into(),
        checks: Vec::new(),
    };
    report.pass("linked_arena_loads");
    for blocker in &link.blockers {
        report.pending_check(
            &format!("blocker:{blocker}"),
            "an open #120 follow-up blocks the lab from running this room",
        );
    }
    for item in &room.validation_pending {
        report.pending_check(item, "declared pending in the manifest");
    }
    Ok(report)
}

/// Validate every room and variant of a (structurally valid) manifest.
pub fn validate_all(root: &Path, manifest: &Manifest) -> Result<Vec<ValidationReport>, LabError> {
    let mut reports = Vec::new();
    for room in &manifest.rooms {
        if room.link.is_some() {
            reports.push(validate_link(root, room)?);
            continue;
        }
        for variant in room.variant_ids() {
            let resolved = room.resolve(variant)?;
            reports.push(validate_resolved(root, &resolved)?);
        }
    }
    Ok(reports)
}
