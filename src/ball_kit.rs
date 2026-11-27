//! BOS-ART: the dressing of the Ball's room 18 (`bos`, `--scene f01_ball`). Pure and headless, no
//! Bevy types.
//!
//! The arena is composed from the catacomb kit by [`crate::camera_lab::layout`] exactly as before
//! (the locked 640 px boss arena of [`crate::boss_scene`]); this module then applies the `bos`
//! block of the room-dressing contract (`floor01_bos_kit.json`, which extends the gym kit and
//! through it the opening kit and ENV-01) with the dressing code of the Broken Ring kit
//! ([`crate::ring_kit::dress_with`]): the composer's gallery bay is replaced by the two arches
//! over the gameplay arch rects with their hero alcoves, the back wall and the deck are dressed
//! and the chain-weight's ceiling girder is placed. Pieces never produce solids, so the dressing is
//! **presentation only**: the tests prove the solids, spawn, exit, views and walking line are the
//! plain arena's. The chain weight, its chain and the open stairwell are presenter sprites whose
//! frame follows the combat state ([`weight_piece`], [`stairwell_open`]). Spec:
//! `docs/02-art/bos-art-spec.md`.

use std::path::Path;

use crate::camera_lab::layout::{BASE_LAYOUT, compose};
use crate::camera_lab::room::Room;
use crate::env_kit::{EnvError, EnvKit, Layout, load_contract};
use crate::ring_kit::{Dressing, dress_with};

/// The Ball's kit contract (extends the gym kit, which extends the opening kit and ENV-01).
pub const BOS_CONTRACT: &str = "assets/source/terrain/floor01/floor01_bos_kit.json";
/// The room id of the dressing block.
pub const ROOM_ID: &str = "bos";

/// The presenter pieces of the chain weight (ids of the contract).
pub const WEIGHT_PIECES: [&str; 2] = ["bos_weight_hung", "bos_weight_landed"];
pub const CHAIN_PIECE: &str = "bos_chain_v";
pub const CHAIN_PIECE_H: &str = "bos_chain_h";
/// The open exit: the opening kit's open Guild door drawn over the composer's closed one.
pub const PORTAL_OPEN_PIECE: &str = "guild_portal_open";
pub const PORTAL_CLOSED_PIECE: &str = "guild_portal_closed";

/// The weight's piece by state: `landed` once the rack has dropped (it stays down).
pub fn weight_piece(landed: bool) -> &'static str {
    WEIGHT_PIECES[usize::from(landed)]
}

/// Whether the stairwell behind the right wall is shown (the boss released the exit).
pub fn stairwell_open(released: bool) -> bool {
    released
}

fn read_contract(root: &Path) -> Result<serde_json::Value, EnvError> {
    let path = root.join(BOS_CONTRACT);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| EnvError::Io(format!("{}: {e}", path.display())))?;
    serde_json::from_str(&text).map_err(|e| EnvError::Parse(format!("{}: {e}", path.display())))
}

/// The dressing block of the Ball's room.
pub fn dressing(root: &Path) -> Result<Dressing, EnvError> {
    let contract = read_contract(root)?;
    serde_json::from_value(contract["room_dressing"][ROOM_ID].clone())
        .map_err(|e| EnvError::Parse(format!("{BOS_CONTRACT} room_dressing.{ROOM_ID}: {e}")))
}

/// The composed layout of the arena room (plain), before any dressing.
pub fn composed(root: &Path, room: &Room) -> Result<serde_json::Value, EnvError> {
    let path = root.join(BASE_LAYOUT);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| EnvError::Io(format!("{}: {e}", path.display())))?;
    let base: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| EnvError::Parse(format!("{}: {e}", path.display())))?;
    Ok(compose(room, &base).0)
}

/// The env kit of the Ball's arena with the dressing: the plain composition, the `bos` block
/// applied (an unmatched `remove_at` entry is an error) and the Ball's contract loaded.
pub fn build_arena_kit(root: &Path, room: &Room) -> Result<EnvKit, EnvError> {
    let contract = load_contract(root, BOS_CONTRACT)?;
    let mut value = composed(root, room)?;
    let missed = dress_with(&mut value, &dressing(root)?, BOS_CONTRACT);
    if missed != 0 {
        return Err(EnvError::Parse(format!(
            "{BOS_CONTRACT}: {missed} remove_at entries match no piece"
        )));
    }
    let layout: Layout =
        serde_json::from_value(value).map_err(|e| EnvError::Parse(e.to_string()))?;
    EnvKit::build(&contract, &layout)
}

/// The composer's closed exit door (the open overlay is drawn on it): its top-left in the room.
pub const PORTAL_AT: (i32, i32) = (536, 284);

/// The atlas and rect of every presenter piece, read from the merged contract at spawn.
#[derive(Debug, Clone)]
pub struct PresenterRects {
    pub pieces: Vec<(String, String, [i32; 4])>,
    pub portal_open: (String, [i32; 4]),
}

impl PresenterRects {
    /// The `(atlas, rect)` of a weight piece by its id.
    pub fn piece(&self, id: &str) -> Option<(&str, [i32; 4])> {
        self.pieces
            .iter()
            .find(|(piece, _, _)| piece == id)
            .map(|(_, atlas, rect)| (atlas.as_str(), *rect))
    }
}

/// Read every presenter piece's atlas and rect from the Ball's contract.
pub fn presenter_rects(root: &Path) -> Result<PresenterRects, String> {
    let contract = load_contract(root, BOS_CONTRACT).map_err(|e| e.to_string())?;
    let find = |id: &str| -> Result<(String, [i32; 4]), String> {
        contract
            .pieces
            .iter()
            .find(|p| p.id == id)
            .map(|p| (p.atlas.clone(), p.rect))
            .ok_or_else(|| format!("the Ball's contract has no piece {id}"))
    };
    let mut pieces = Vec::new();
    for id in WEIGHT_PIECES
        .into_iter()
        .chain([CHAIN_PIECE, CHAIN_PIECE_H])
    {
        let (atlas, rect) = find(id)?;
        pieces.push((id.to_owned(), atlas, rect));
    }
    Ok(PresenterRects {
        pieces,
        portal_open: find(PORTAL_OPEN_PIECE)?,
    })
}

#[cfg(test)]
mod tests;
