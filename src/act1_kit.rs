//! AREA-ACT1-01: the dressing of the three Floor 01 act-1 rooms `act1_chase`, `act1_shore` and
//! `act1_guild` (the first chase, the rotten shore, the guild door). Pure and headless, no Bevy
//! types.
//!
//! The room is composed from the Boiler's catacomb kit exactly as before
//! ([`crate::f01_boiler::compose_kit_hooked`]); this module then applies the room's
//! `room_dressing` block of the act-1 contract with the dressing code of the Broken Ring kit
//! ([`crate::ring_kit::dress_with`]): the chase's S2 bay is bricked up behind the gallery breach,
//! the torches that stood inside a timber set, over the alcove ledge, behind the wedged machine or
//! beside the hearth are removed, and the soffit, the step faces, the alcove, the breach and the
//! guild furniture are placed. Pieces never produce solids, so the dressing is **presentation
//! only**: [`crate::f01_act1::launch_with`] pushes the guild slab and the sidecar's plain `solid`
//! statics (the humps and the alcove ledge) in after the composition, one code path for the
//! dressed and the plain room, and the tests prove the solids, spawns, exits, views and walking
//! line are identical with and without the dressing.
//!
//! The timbers, the barricade, the grant box and the guild portal's open overlay are **not**
//! placed: their frame follows the combat state, so the scene's presenter draws the piece the pure
//! functions here pick ([`timber_piece`], [`barricade_piece`], [`box_piece`]) at the rects of the
//! sim ([`TIMBER_AT`], [`BARRICADE_AT`], [`BOX_AT`], [`PORTAL_AT`]). Spec:
//! `docs/02-art/area-act1-01-spec.md`.

use std::path::Path;

use crate::encounter::Loaded;
use crate::env_kit::{EnvKit, load_contract};
use crate::f01_act1::Act1Room;
use crate::ring_kit::{Dressing, dress_with};

/// The act-1 kit contract (extends the opening kit, which extends ENV-01).
pub const ACT1_CONTRACT: &str = "assets/source/terrain/floor01/floor01_act1_kit.json";

/// The two timber sets' top-left corners (the sim's `timber_a` and `timber_b` decor rects).
pub const TIMBER_AT: [(i32, i32); 2] = [(1104, 456), (1424, 456)];
/// The chase barricade's top-left (the ENV-02 full barricade over the lock solid {2312, 304, 80, 276}).
pub const BARRICADE_AT: (i32, i32) = (2304, 304);
/// The grant box's top-left: 24 x 28, its bottom row on the walking line (y 579), inside the grant
/// zone 336..376.
pub const BOX_AT: (i32, i32) = (340, 552);
/// The composer's guild portal (`guild_portal_closed`) top-left in `act1_guild`; the open overlay
/// is drawn on it.
pub const PORTAL_AT: (i32, i32) = (536, 444);
/// The chase's gallery breach (the sim's window rect) and the soffit rect.
pub const BREACH_AT: (i32, i32) = (696, 342);
pub const SOFFIT_AT: (i32, i32) = (976, 272);

/// The presenter pieces by state (ids of the merged contract).
pub const TIMBER_PIECES: [&str; 2] = ["timber_set_intact", "timber_set_smashed"];
pub const BARRICADE_PIECES: [&str; 2] = [
    "timber_barricade_full_intact",
    "timber_barricade_full_smashed",
];
pub const BOX_PIECES: [&str; 2] = ["act1_grant_box_closed", "act1_grant_box_open"];
pub const PORTAL_OPEN_PIECE: &str = "guild_portal_open";

/// The timber set's piece: `smashed` stays shown once the front has passed.
pub fn timber_piece(smashed: bool) -> &'static str {
    TIMBER_PIECES[usize::from(smashed)]
}

/// The barricade's piece: `smashed` follows `barrier_hit()` (rewind- and respawn-safe), so the
/// break stays shown through the defeat delay and the barricade is intact again once the respawn
/// closes the lock.
pub fn barricade_piece(hit: bool) -> &'static str {
    BARRICADE_PIECES[usize::from(hit)]
}

/// The grant box's piece: `open` once the guild door's lock is open (`donut_unlocked`).
pub fn box_piece(open: bool) -> &'static str {
    BOX_PIECES[usize::from(open)]
}

/// Whether the guild portal's open overlay is shown (the same lock).
pub fn portal_open(open: bool) -> bool {
    open
}

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
            crate::env_kit::EnvError::Parse(format!("{ACT1_CONTRACT} room_dressing.{room_id}: {e}"))
        })
}

fn read_contract(root: &Path) -> Result<serde_json::Value, crate::env_kit::EnvError> {
    let path = root.join(ACT1_CONTRACT);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| crate::env_kit::EnvError::Io(format!("{}: {e}", path.display())))?;
    serde_json::from_str(&text)
        .map_err(|e| crate::env_kit::EnvError::Parse(format!("{}: {e}", path.display())))
}

/// The dressing block of an act-1 room id (`None`: no block, or an unknown id; the room launches
/// plain).
pub fn dressing_of(
    root: &Path,
    room_id: &str,
) -> Result<Option<Dressing>, crate::env_kit::EnvError> {
    dressing_from(&read_contract(root)?, room_id)
}

/// Whether the room has a dressing block (the binary prints `decor=dressed|plain` from it).
pub fn is_dressed(root: &Path, room: Act1Room) -> bool {
    dressing_of(root, room.id()).is_ok_and(|d| d.is_some())
}

/// The room's composition **before** the launch's own solids: dressed (the act-1 contract's
/// dressing block, an unmatched `remove_at` entry is an error) or plain (the Boiler's catacomb
/// kit). A room whose contract has no block composes plain even when `dressed` is asked for.
pub fn compose(
    root: &Path,
    loaded: &Loaded,
    room: Act1Room,
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
    let contract = load_contract(root, ACT1_CONTRACT).map_err(|e| e.to_string())?;
    crate::f01_boiler::compose_kit_hooked(root, &loaded.room, &[], Some(contract), |layout| {
        match dress_with(layout, &dressing, ACT1_CONTRACT) {
            0 => Ok(()),
            n => Err(format!("{room_id}: {n} remove_at entries match no piece")),
        }
    })
}

/// The atlas and rect of every presenter piece, read from the merged contract at spawn.
#[derive(Debug, Clone)]
pub struct PresenterRects {
    pub timber: [(String, [i32; 4]); 2],
    pub barricade: [(String, [i32; 4]); 2],
    pub boxes: [(String, [i32; 4]); 2],
    pub portal_open: (String, [i32; 4]),
}

/// Read every presenter piece's atlas and rect from the act-1 contract.
pub fn presenter_rects(root: &Path) -> Result<PresenterRects, String> {
    let contract = load_contract(root, ACT1_CONTRACT).map_err(|e| e.to_string())?;
    let find = |id: &str| -> Result<(String, [i32; 4]), String> {
        contract
            .pieces
            .iter()
            .find(|p| p.id == id)
            .map(|p| (p.atlas.clone(), p.rect))
            .ok_or_else(|| format!("the act-1 contract has no piece {id}"))
    };
    Ok(PresenterRects {
        timber: [find(TIMBER_PIECES[0])?, find(TIMBER_PIECES[1])?],
        barricade: [find(BARRICADE_PIECES[0])?, find(BARRICADE_PIECES[1])?],
        boxes: [find(BOX_PIECES[0])?, find(BOX_PIECES[1])?],
        portal_open: find(PORTAL_OPEN_PIECE)?,
    })
}

#[cfg(test)]
mod tests;
