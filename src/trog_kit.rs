//! AREA-TROG-01 stage 1: the dressing of the Floor 01 troglodyte run `trog_run` (TROG-A). Pure and
//! headless, no Bevy types.
//!
//! The room is composed from the Boiler's catacomb kit exactly as before
//! ([`crate::f01_boiler::compose_kit_hooked`]); this module then applies the room's `room_dressing`
//! block of the trog contract with the dressing code of the Broken Ring kit
//! ([`crate::ring_kit::dress_with`]): bays 1 and 4 are bricked up (the slam crack and knuckle
//! marks, the fist-shaped hole), their chains and one doubled torch with its pool are removed, and
//! the wall, floor and prop pieces are placed. Pieces never produce solids, so the dressing is
//! **presentation only**: the DP-Q45 ceiling slab and the sidecar's plain `solid` statics (the low
//! refuge and the bell step) are pushed in by [`lane_solids`], one code path for the dressed and the
//! plain composition, and the tests prove the solids, spawns, checkpoints, exits, views and walking
//! line are identical with and without the dressing.
//!
//! The strength room `trog_strength` (the bench stack and its debris burst) is stage 2 and waits
//! for TROG-B. Spec: `docs/02-art/area-trog-01-spec.md`.

use std::path::Path;

use crate::encounter::{Loaded, StaticKind};
use crate::env_kit::{EnvKit, IRect as KitRect, load_contract};
use crate::f01_trog::TrogRoom;
use crate::ring_kit::{Dressing, dress_with, dressing_in};

/// The troglodyte kit contract (extends the gym kit).
pub const TROG_CONTRACT: &str = "assets/source/terrain/floor01/floor01_trog_kit.json";
/// The room id of the run's dressing block.
pub const RUN_ROOM: &str = "trog_run";

/// The dressing block of the room `room_id` in a parsed contract: `None` when the contract has no
/// block (or a null one) for it, an error when the block is malformed.
pub fn dressing_from(
    contract: &serde_json::Value,
    room_id: &str,
) -> Result<Option<Dressing>, crate::env_kit::EnvError> {
    let block = &contract["room_dressing"][room_id];
    if block.is_null() {
        return Ok(None);
    }
    serde_json::from_value(block.clone())
        .map(Some)
        .map_err(|e| {
            crate::env_kit::EnvError::Parse(format!("{TROG_CONTRACT} room_dressing.{room_id}: {e}"))
        })
}

fn read_contract(root: &Path) -> Result<serde_json::Value, crate::env_kit::EnvError> {
    let path = root.join(TROG_CONTRACT);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| crate::env_kit::EnvError::Io(format!("{}: {e}", path.display())))?;
    serde_json::from_str(&text)
        .map_err(|e| crate::env_kit::EnvError::Parse(format!("{}: {e}", path.display())))
}

/// The dressing block of a troglodyte room (`None`: the room is not dressed yet and launches plain).
pub fn dressing_of(
    root: &Path,
    room: TrogRoom,
) -> Result<Option<Dressing>, crate::env_kit::EnvError> {
    dressing_from(&read_contract(root)?, room.id())
}

/// Whether the room has a dressing block (the binary prints `decor=dressed|plain` from it).
pub fn is_dressed(root: &Path, room: TrogRoom) -> bool {
    dressing_of(root, room).is_ok_and(|d| d.is_some())
}

/// The dressing block of the run.
pub fn run_dressing(root: &Path) -> Result<Dressing, crate::env_kit::EnvError> {
    dressing_in(root, TROG_CONTRACT, RUN_ROOM)
}

/// The room's composition **before** the lane solids: dressed (the trog contract's dressing block,
/// an unmatched `remove_at` entry is an error) or plain (the Boiler's catacomb kit). A room whose
/// contract has no `room_dressing` block composes plain even when `dressed` is asked for: stage 2
/// rooms launch plain until their block lands (TROG-ART-AD19).
pub fn compose(
    root: &Path,
    loaded: &Loaded,
    room: TrogRoom,
    dressed: bool,
) -> Result<EnvKit, String> {
    compose_by_id(root, loaded, room.id(), dressed)
}

/// [`compose`] for a room id.
pub fn compose_by_id(
    root: &Path,
    loaded: &Loaded,
    room_id: &str,
    dressed: bool,
) -> Result<EnvKit, String> {
    let dressing = if dressed {
        dressing_from(&read_contract(root).map_err(|e| e.to_string())?, room_id)
            .map_err(|e| e.to_string())?
    } else {
        None
    };
    let Some(dressing) = dressing else {
        return crate::f01_boiler::compose_kit(root, &loaded.room);
    };
    let contract = load_contract(root, TROG_CONTRACT).map_err(|e| e.to_string())?;
    crate::f01_boiler::compose_kit_hooked(root, &loaded.room, &[], Some(contract), |layout| {
        match dress_with(layout, &dressing, TROG_CONTRACT) {
            0 => Ok(()),
            n => Err(format!("{room_id}: {n} remove_at entries match no piece")),
        }
    })
}

/// The lane solids of a troglodyte room (DP-Q45 / DM-Q27 / KR9): the lane ceiling slab and the
/// sidecar's plain `solid` statics (the low refuge and the bell step) as lane steps. The same code
/// for the dressed and the plain composition.
pub fn lane_solids(kit: &mut EnvKit, loaded: &Loaded, path: &str) -> Result<(), String> {
    let ceiling = loaded
        .ceiling_px
        .ok_or_else(|| format!("{path}: a troglodyte room needs a lane ceiling (DP-Q45)"))?;
    kit.solid_rects.push(KitRect {
        x: 0,
        y: kit.base_walk_line_y - ceiling - 16,
        w: kit.width,
        h: 16,
    });
    for st in loaded
        .encounter
        .statics
        .iter()
        .filter(|st| st.kind == StaticKind::Solid && st.slide.is_none())
    {
        kit.solid_rects.push(KitRect {
            x: st.rect.x,
            y: st.rect.y,
            w: st.rect.w,
            h: st.rect.h,
        });
    }
    Ok(())
}

/// The env kit of a troglodyte room as [`crate::f01_trog::launch`] builds it: the composition (dressed
/// or plain) with the lane solids pushed in. Checks the kit's walking line against the room's floor.
pub fn compose_room_kit(
    root: &Path,
    loaded: &Loaded,
    room: TrogRoom,
    dressed: bool,
) -> Result<EnvKit, String> {
    let mut kit = compose(root, loaded, room, dressed)?;
    if kit.base_walk_line_y != loaded.room.floor_y {
        return Err(format!(
            "the kit's walking line {} is not the room's floor {}",
            kit.base_walk_line_y, loaded.room.floor_y
        ));
    }
    lane_solids(&mut kit, loaded, room.encounter_path())?;
    Ok(kit)
}

#[cfg(test)]
mod tests;
