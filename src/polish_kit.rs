//! AREA-POLISH-01: the dressing of the Floor 01 mastery arena (`mastery`, room 17) and the Dozer's
//! room shell (`dozer`, room 8). Pure and headless, no Bevy types.
//!
//! Both rooms are composed exactly as before (the mastery arena by the CF02-F microarena composer,
//! [`crate::cf02::arena::load_arena_hooked`]; the Dozer's room by the Boiler's catacomb kit,
//! [`crate::f01_boiler::compose_kit_hooked`]); this module then applies the room's `room_dressing`
//! block of the polish contract with the dressing code of the Broken Ring kit
//! ([`crate::ring_kit::dress_with`]). The contract is **reuse only**: its pieces are the
//! approved ones of the catacomb kit, the corral (heat), the act-1 and the gym kits, declared
//! again by the same definitions (no new atlas, no new pixel). Pieces never produce solids, so the
//! dressing is **presentation only**: the tests prove the solids, spawns, exits, views and walking
//! line are identical with and without the dressing and that the state hash does not move. Spec:
//! `docs/02-art/area-polish-01-spec.md`.

use std::path::Path;

use serde_json::json;

use crate::encounter::Loaded;
use crate::env_kit::{EnvError, EnvKit, load_contract};
use crate::ring_kit::{Dressing, dress_with};

/// The polish kit contract (extends the opening kit, which extends ENV-01).
pub const POLISH_CONTRACT: &str = "assets/source/terrain/floor01/floor01_polish_kit.json";
/// The room ids of the dressing blocks.
pub const MASTERY_ROOM: &str = "mastery";
pub const DOZER_ROOM: &str = "dozer";

/// The dressing block of the room `room_id` in a parsed contract: `None` when the contract has no
/// block (or a null one) for it, an error when the block is malformed.
pub fn dressing_from(
    contract: &serde_json::Value,
    room_id: &str,
) -> Result<Option<Dressing>, EnvError> {
    let block = &contract["room_dressing"][room_id];
    if block.is_null() {
        return Ok(None);
    }
    serde_json::from_value(block.clone())
        .map(Some)
        .map_err(|e| EnvError::Parse(format!("{POLISH_CONTRACT} room_dressing.{room_id}: {e}")))
}

fn read_contract(root: &Path) -> Result<serde_json::Value, EnvError> {
    let path = root.join(POLISH_CONTRACT);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| EnvError::Io(format!("{}: {e}", path.display())))?;
    serde_json::from_str(&text).map_err(|e| EnvError::Parse(format!("{}: {e}", path.display())))
}

/// The dressing block of a room id (`None`: no block; the room composes plain).
pub fn dressing_of(root: &Path, room_id: &str) -> Result<Option<Dressing>, EnvError> {
    dressing_from(&read_contract(root)?, room_id)
}

/// Whether the room has a dressing block.
pub fn is_dressed(root: &Path, room_id: &str) -> bool {
    dressing_of(root, room_id).is_ok_and(|d| d.is_some())
}

/// The dressing hook: apply the block, an unmatched `remove_at` entry is an error.
fn apply(layout: &mut serde_json::Value, dressing: &Dressing, room_id: &str) -> Result<(), String> {
    match dress_with(layout, dressing, POLISH_CONTRACT) {
        0 => {
            layout["kit_contract"] = json!(POLISH_CONTRACT);
            Ok(())
        }
        n => Err(format!("{room_id}: {n} remove_at entries match no piece")),
    }
}

/// The Dozer's room before the launch's own solids: dressed (the polish contract's block) or plain
/// (the Boiler's catacomb kit). A contract without a block composes plain.
pub fn compose_dozer(root: &Path, loaded: &Loaded, dressed: bool) -> Result<EnvKit, String> {
    let dressing = if dressed {
        dressing_from(&read_contract(root).map_err(|e| e.to_string())?, DOZER_ROOM)
            .map_err(|e| e.to_string())?
    } else {
        None
    };
    let Some(dressing) = dressing else {
        return crate::f01_boiler::compose_kit(root, &loaded.room);
    };
    let contract = load_contract(root, POLISH_CONTRACT).map_err(|e| e.to_string())?;
    crate::f01_boiler::compose_kit_hooked(root, &loaded.room, &[], Some(contract), |layout| {
        apply(layout, &dressing, DOZER_ROOM)
    })
}

/// The mastery arena before the launch's own checks: dressed or plain (the CF02-F microarena).
pub fn compose_mastery(root: &Path, dressed: bool) -> Result<EnvKit, String> {
    let dressing = if dressed {
        dressing_from(
            &read_contract(root).map_err(|e| e.to_string())?,
            MASTERY_ROOM,
        )
        .map_err(|e| e.to_string())?
    } else {
        None
    };
    let Some(dressing) = dressing else {
        return crate::cf02::arena::load_arena(root);
    };
    let contract = load_contract(root, POLISH_CONTRACT).map_err(|e| e.to_string())?;
    crate::cf02::arena::load_arena_hooked(root, &[], Some(contract), |layout| {
        apply(layout, &dressing, MASTERY_ROOM)
    })
}

#[cfg(test)]
mod tests;
