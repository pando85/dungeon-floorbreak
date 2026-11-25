//! ROOM-PORTALS-01: the room exit of the Floor 1 rooms 2, 4, 4a, 4b and 6, the pure
//! half (`docs/02-art/floor01-room-exit-spec.md`). The Boiler, the hazard room, the
//! two Broken Ring rooms and the mastery arena drew their exit with the reused Guild
//! door (`guild_portal_closed`, the composer's exit marker), and the `defeat_roster`
//! exit gate of rooms 4 and 6
//! (`EnvSolidKind::ExitGate`, the sidecar's 64 x 96 exit) was an invisible solid.
//! This module says what the presenter draws in the Guild door's place: a stone
//! doorway around the sidecar's exit with the room 5 gate's drop grate
//!
//! - **down** (`exit_closed`) while the room's exit gate stands: exactly while the
//!   simulation keeps the gate's block intact (every roster enemy not yet defeated,
//!   HAZ-Q39; the predicate the env sync uses, read through shared references);
//! - **raised** (`exit_open`) otherwise: once the roster is clear, and always in a
//!   room without an exit gate (the `reach_exit` goal of the Boiler and the Ring rooms).
//!
//! The composer's ground torch at the room's end stands behind the doorway with only
//! its flame tip above the cap; [`hides_piece`] names it so the presenter does not
//! draw it (it read as a candle on the cap; its light stays).
//!
//! **Presentation only.** Nothing here mutates a state, draws a random number or
//! is hashed, replayed or read back by the simulation (a test steps a cleared run
//! with and without drawing and compares the state hash on every tick). The grate
//! snaps between its two frames on the tick the gate opens: the simulation keeps no
//! opening tick, so a lift animation would need a render-local memory (adopted
//! default EXIT-AD3, the room 5 gate's DFX-AD7).
//!
//! The art matches the simulation or is not drawn: the opening is drawn only for a
//! 64 x 96 exit (the canvas's opening); another size keeps the Guild door.
//! Coordinates are world px, y up; an [`ExitDraw`] gives the bottom-left corner.

use crate::camera_lab::room::IRect;
use crate::combat_scene::{CombatState, EnvSolidKind};

/// The manifest under `assets/manifests/` (without `.json`).
pub const MANIFEST: &str = "f01-dfx-exit";
/// The exported atlas, relative to the repository root.
pub const ATLAS: &str = "generated/art/f01-dfx-exit.png";
/// The kit piece the art replaces in rooms 2, 4 and 6 (the composer's exit marker).
pub const PORTAL_PIECE: &str = "guild_portal_closed";
/// The canvas: a 16 px jamb on each side of the 64 px opening, 40 rows of cap, header,
/// lintel and grate slot over the 96 px opening (the replaced Guild door's height).
pub const EXIT_W: i32 = 96;
pub const EXIT_H: i32 = 136;
pub const JAMB_W: i32 = 16;
pub const OPENING_W: i32 = 64;
pub const OPENING_H: i32 = 96;
/// The raised grate's rail and teeth hang this many rows under the lintel.
pub const RAISED_ROWS: i32 = 16;
/// The composer's wall torch piece (the ground torch at a room's end uses it).
pub const TORCH_PIECE: &str = "torch_wall";

/// The two held frames of the sheet, in atlas order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitFrame {
    /// `exit_closed`: the grate down over the whole opening.
    Closed,
    /// `exit_open`: the grate raised into the lintel, the passage clear.
    Open,
}

impl ExitFrame {
    /// The frame's index in the atlas (one row of equal cells).
    pub const fn index(self) -> usize {
        match self {
            Self::Closed => 0,
            Self::Open => 1,
        }
    }

    /// The manifest's tag of this frame.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Closed => "exit_closed",
            Self::Open => "exit_open",
        }
    }
}

/// The sprite: `frame` of the sheet, its bottom-left at world `(x, y)`, `w` x `h` px.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitDraw {
    pub frame: ExitFrame,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Whether the room's exit gate stands now: the room has a `defeat_roster` exit
/// gate and its roster is not clear. The same predicate as the env sync's
/// (`EnvSolidKind::ExitGate => !cleared`), so the drawn grate is the solid block.
pub fn exit_closed(state: &CombatState) -> bool {
    state.env().is_some_and(|env| {
        env.solids()
            .iter()
            .any(|s| matches!(s.kind, EnvSolidKind::ExitGate))
    }) && !state.roster_cleared()
}

/// The doorway around the sidecar exit `exit` (layout px, y down) of a room whose
/// layout is `layout_h` px tall: the opening on the exit's columns, the canvas's
/// bottom row on the exit's bottom (the walking line). `None` when the exit is not
/// the 64 x 96 opening the canvas is drawn for.
pub fn exit_draw(state: &CombatState, exit: IRect, layout_h: i32) -> Option<ExitDraw> {
    if exit.w != OPENING_W || exit.h != OPENING_H {
        return None;
    }
    let frame = if exit_closed(state) {
        ExitFrame::Closed
    } else {
        ExitFrame::Open
    };
    Some(ExitDraw {
        frame,
        x: exit.x - JAMB_W,
        y: layout_h - (exit.y + exit.h),
        w: EXIT_W,
        h: EXIT_H,
    })
}

/// Whether the kit piece `piece` at `(x, y)` (layout px, y down, top-left) of size
/// `w` x `h` is a torch the doorway around `exit` covers but for its flame tip: its
/// columns lie inside the canvas and its bottom row inside the canvas's rows. Such a
/// piece is not drawn where the doorway is (EXIT-AD7); every other piece is.
pub fn hides_piece(exit: IRect, piece: &str, x: i32, y: i32, w: i32, h: i32) -> bool {
    let (left, bottom) = (exit.x - JAMB_W, exit.y + exit.h);
    piece == TORCH_PIECE
        && x >= left
        && x + w <= left + EXIT_W
        && y + h <= bottom
        && y + h > bottom - EXIT_H
}

#[cfg(test)]
mod tests;
