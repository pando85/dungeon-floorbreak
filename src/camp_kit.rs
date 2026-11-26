//! AREA-CAMP-01: the dressing of the two Floor 01 quiet rooms `rest_camp` (the survivors'
//! camp, REST-B) and `rest_aftermath` (the quiet after the Juicer, REST-C), and the pure
//! frame selection of the aftermath's heavy bag. Pure and headless, no Bevy types.
//!
//! Each room is composed from the Boiler's catacomb kit exactly as before
//! ([`crate::f01_boiler::compose_kit_hooked`]); this module then applies the room's
//! `room_dressing` block of its contract with the dressing code of the Broken Ring kit
//! ([`crate::ring_kit::dress_with`]): the camp removes four torches and their pools (the
//! hearth's fire is the light there), the aftermath bricks up window B2 and removes its
//! chain, and both place their wall, structure and prop pieces. Pieces never produce
//! solids, so the dressing is **presentation only**: the tests below prove the solids,
//! spawns, checkpoints, exits, views, walking line and the sidecar's inert hit target are
//! identical with and without it.
//!
//! The heavy bag is **not** placed. The scene's presenter draws the frame
//! [`bag_frame`] picks from the 38-tick sway table (the entry swing on the first
//! tick it observes the room, a restart on every rising edge of an overlap with a hero's
//! active hit box). It reads state that already exists and writes nothing back: the sim's
//! `hit_target` stays inert (REST-IMP-9). Spec: `docs/02-art/area-camp-01-spec.md`.

use std::path::Path;

use crate::camera_lab::room::Room;
use crate::env_kit::{EnvKit, load_contract};
use crate::prototype::Rect;
use crate::ring_kit::{Dressing, dress_with, dressing_in};

/// The camp kit contract (extends the Broken Ring kit).
pub const CAMP_CONTRACT: &str = "assets/source/terrain/floor01/floor01_camp_kit.json";
/// The aftermath kit contract (extends the gym kit).
pub const AFTERMATH_CONTRACT: &str = "assets/source/terrain/floor01/floor01_aftermath_kit.json";
/// The room ids of the dressing blocks.
pub const CAMP_ROOM: &str = "rest_camp";
pub const AFTERMATH_ROOM: &str = "rest_aftermath";

/// The dressing block of the camp.
pub fn camp_dressing(root: &Path) -> Result<Dressing, crate::env_kit::EnvError> {
    dressing_in(root, CAMP_CONTRACT, CAMP_ROOM)
}

/// The dressing block of the aftermath.
pub fn aftermath_dressing(root: &Path) -> Result<Dressing, crate::env_kit::EnvError> {
    dressing_in(root, AFTERMATH_CONTRACT, AFTERMATH_ROOM)
}

fn compose(root: &Path, room: &Room, contract_path: &str, room_id: &str) -> Result<EnvKit, String> {
    let contract = load_contract(root, contract_path).map_err(|e| e.to_string())?;
    let dressing = dressing_in(root, contract_path, room_id).map_err(|e| e.to_string())?;
    crate::f01_boiler::compose_kit_hooked(
        root,
        room,
        &[],
        Some(contract),
        |layout| match dress_with(layout, &dressing, contract_path) {
            0 => Ok(()),
            n => Err(format!("{room_id}: {n} remove_at entries match no piece")),
        },
    )
}

/// The env kit of the camp: the room's composition with the camp dressing applied. An
/// unmatched `remove_at` entry is an error.
pub fn compose_camp_kit(root: &Path, room: &Room) -> Result<EnvKit, String> {
    compose(root, room, CAMP_CONTRACT, CAMP_ROOM)
}

/// The env kit of the aftermath: the room's composition with the aftermath dressing
/// applied. An unmatched `remove_at` entry is an error.
pub fn compose_aftermath_kit(root: &Path, room: &Room) -> Result<EnvKit, String> {
    compose(root, room, AFTERMATH_CONTRACT, AFTERMATH_ROOM)
}

// ------------------------------------------------------------------ the heavy bag

/// The bag canvas's top-left (layout px, y down): the chain hooks under the girder at
/// (640, 450); the body covers the sim's hit rect.
pub const BAG_AT: (i32, i32) = (620, 450);
/// The sidecar's inert `hit_target` (layout px, y down): `heavy_bag` {624, 516, 32, 64}.
pub const BAG_RECT: (i32, i32, i32, i32) = (624, 516, 32, 64);
/// The bag's centre column: the push direction is the sign of this minus the attacker's x.
pub const BAG_CENTRE_X: i32 = 640;
/// The atlas of the five bag frames.
pub const BAG_ATLAS: &str = "floor01_aft_bag";
/// The bag's five pieces: `rest`, `r1`, `r2`, `l1`, `l2`.
pub const BAG_PIECES: [&str; 5] = [
    "aft_heavy_bag_rest",
    "aft_heavy_bag_r1",
    "aft_heavy_bag_r2",
    "aft_heavy_bag_l1",
    "aft_heavy_bag_l2",
];

/// The sway table (spec section 7): one frame per presentation tick after a trigger, then
/// `rest`. A left push mirrors `r` and `l`. `scripts/author_camp01.py` `SWAY_TABLE` is the
/// same list (the art unittest compares them).
pub const SWAY_TABLE: [&str; 38] = [
    "r1", "r1", "r1", "r1", //
    "r2", "r2", "r2", "r2", "r2", "r2", //
    "r1", "r1", "r1", "r1", //
    "rest", "rest", "rest", "rest", //
    "l1", "l1", "l1", "l1", "l1", "l1", "l1", "l1", //
    "rest", "rest", "rest", "rest", //
    "r1", "r1", "r1", "r1", //
    "rest", "rest", "rest", "rest",
];

/// The bag's rectangle in world space (y up) for a layout of `layout_height` px.
pub fn bag_rect_world(layout_height: i32) -> Rect {
    let (x, y, w, h) = BAG_RECT;
    Rect {
        left: x as f32,
        bottom: (layout_height - (y + h)) as f32,
        width: w as f32,
        height: h as f32,
    }
}

/// The index into [`BAG_PIECES`] of a frame name.
pub fn bag_piece_index(frame: &str) -> usize {
    match frame {
        "r1" => 1,
        "r2" => 2,
        "l1" => 3,
        "l2" => 4,
        _ => 0,
    }
}

fn mirrored(frame: &'static str) -> &'static str {
    match frame {
        "r1" => "l1",
        "r2" => "l2",
        "l1" => "r1",
        "l2" => "r2",
        other => other,
    }
}

/// The bag presenter's own memory (presentation state, like any cue timer): never hashed,
/// never written back to the sim.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BagSway {
    /// The last observed tick; `None` until the presenter first observes the room.
    last: Option<u64>,
    /// The tick the running swing started on.
    trigger: Option<u64>,
    /// +1 swings right first, -1 mirrors it.
    dir: i32,
    /// The overlap observed on the last tick (the edge detector).
    prev_overlap: bool,
    /// Carl's and Donut's own overlap flags, for [`BagSway::observe`].
    prev_carl: bool,
    prev_donut: bool,
}

impl BagSway {
    /// The frame name of the running swing at `tick` (`rest` when none runs).
    fn frame_at(&self, tick: u64) -> &'static str {
        let Some(t0) = self.trigger else {
            return "rest";
        };
        let Some(idx) = tick.checked_sub(t0) else {
            return "rest";
        };
        match SWAY_TABLE.get(idx as usize) {
            Some(&f) if self.dir < 0 => mirrored(f),
            Some(&f) => f,
            None => "rest",
        }
    }

    /// Both hero hit boxes at once (gate note G6.4): `overlaps` is Carl's OR Donut's; the
    /// push direction uses Carl's x when his overlap rose this tick, else Donut's. `carl`
    /// and `donut` are `Some((x, facing))` while that hero's active hit box overlaps the
    /// bag.
    pub fn observe(
        &mut self,
        tick: u64,
        carl: Option<(i32, i32)>,
        donut: Option<(i32, i32)>,
    ) -> &'static str {
        let carl_rose = carl.is_some() && !self.prev_carl;
        let donut_rose = donut.is_some() && !self.prev_donut;
        let attacker = if carl_rose {
            carl
        } else if donut_rose {
            donut
        } else {
            carl.or(donut)
        };
        let (x, facing) = attacker.unwrap_or((BAG_CENTRE_X, 1));
        let same_tick = self.last == Some(tick);
        let frame = bag_frame(self, tick, attacker.is_some(), x, facing);
        if !same_tick {
            self.prev_carl = carl.is_some();
            self.prev_donut = donut.is_some();
        }
        frame
    }
}

/// The bag's frame at presentation tick `tick` (gate note G6):
///
/// 1. **Entry:** the entry swing starts on the first tick the presenter observes while
///    `state` is empty (not at "tick 0": a launch at a later tick still swings once).
/// 2. **Idempotent per tick:** the same `tick` returns the same frame and does not
///    re-evaluate the edge. When ticks are skipped the table index is
///    `tick - trigger_tick`, and the edge compares with the last observed overlap.
/// 3. **Rewind:** `tick` below the last observed tick (a fixture seek) resets to `rest`
///    with no running swing and takes the current overlap as already held; the entry swing
///    is never replayed (a retry keeps the tick, so it keeps a running swing).
/// 4. **Retrigger** only on the rising edge of `overlaps`; a held overlap does nothing.
///    The direction is the sign of `640 - attacker_x`; on a tie (`attacker_x == 640`) the
///    attacker's `facing` decides. A left push mirrors `r` and `l`.
pub fn bag_frame(
    state: &mut BagSway,
    tick: u64,
    overlaps: bool,
    attacker_x: i32,
    facing: i32,
) -> &'static str {
    match state.last {
        None => {
            state.trigger = Some(tick);
            state.dir = 1;
            state.prev_overlap = overlaps;
        }
        Some(last) if tick < last => {
            *state = BagSway {
                last: Some(tick),
                prev_overlap: overlaps,
                dir: 1,
                ..BagSway::default()
            };
            return "rest";
        }
        Some(last) if tick == last => return state.frame_at(tick),
        Some(_) => {
            if overlaps && !state.prev_overlap {
                state.trigger = Some(tick);
                state.dir = match (BAG_CENTRE_X - attacker_x).signum() {
                    0 if facing < 0 => -1,
                    0 => 1,
                    s => s,
                };
            }
            state.prev_overlap = overlaps;
        }
    }
    state.last = Some(tick);
    state.frame_at(tick)
}

/// The atlas rectangle `[x, y, w, h]` of each bag frame (the order of [`BAG_PIECES`]), from the
/// aftermath contract.
pub fn bag_rects(root: &Path) -> Result<[[i32; 4]; 5], String> {
    let contract = load_contract(root, AFTERMATH_CONTRACT).map_err(|e| e.to_string())?;
    let mut out = [[0; 4]; 5];
    for (slot, id) in out.iter_mut().zip(BAG_PIECES) {
        let piece = contract
            .pieces
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("the aftermath contract has no piece {id}"))?;
        if piece.atlas != BAG_ATLAS {
            return Err(format!("{id} is in {}, not {BAG_ATLAS}", piece.atlas));
        }
        *slot = piece.rect;
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
