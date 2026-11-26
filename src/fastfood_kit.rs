//! AREA-FASTFOOD-01: the fast-food refuge dressing of Floor 01 room 1 `rest_refuge`
//! ("The Wrong Restaurant") and the pure presenter selection of its three state-keyed
//! props (the staff door, the Light on Your Feet box and the service bell). Pure and
//! headless, no Bevy types.
//!
//! The room is composed from the catacomb kit exactly as before
//! ([`crate::f01_boiler::compose_kit_hooked`]); this module then applies the room's
//! `room_dressing` block of the fast-food contract (`floor01_fastfood_kit.json`, which
//! extends the opening kit and through it ENV-01) with the dressing code of the Broken
//! Ring kit ([`crate::ring_kit::dress_with`]): it removes the four dining-room torches,
//! their pools and the grime, bricks up bay A's opening behind the restaurant and places
//! the wall, structure and prop pieces. Pieces never produce solids, so the dressing is
//! **presentation only**: the tests below prove the solids (the lock and the ceiling slab
//! included), spawn, exit, views, walking line and blink barriers are identical with and
//! without it, and the refuge fixtures hash the same on every tick.
//!
//! The door, the box, the sparkle and the bell are **not** placed (their drawn art must
//! equal the lock's 16 x 228 solid, REST-AD6, and follow the run's state, REST-AD16): the
//! contract lists them as state groups and the scene's presenter draws the frame the pure
//! functions here pick ([`door_frame`], [`box_frame`], [`bell_frame`]), reading the
//! lock's state first (gate note G4). Spec: `docs/02-art/area-fastfood-01-spec.md`.

use std::path::Path;

use crate::camera_lab::room::Room;
use crate::combat::CombatEvent;
use crate::env_kit::{EnvError, EnvKit, load_contract};
use crate::ring_kit::{Dressing, dress_with, dressing_in};

/// The fast-food kit contract (extends the opening kit, which extends ENV-01).
pub const FASTFOOD_CONTRACT: &str = "assets/source/terrain/floor01/floor01_fastfood_kit.json";

/// The room id of the dressing block in the fast-food contract.
pub const ROOM_ID: &str = "rest_refuge";

/// The dressing block of the refuge in the fast-food contract.
pub fn dressing(root: &Path) -> Result<Dressing, EnvError> {
    dressing_in(root, FASTFOOD_CONTRACT, ROOM_ID)
}

/// The env kit of the refuge: the room's composition (CF02 steps cleared, platform face
/// rows cleared, the two scroll-camera views) with the dressing applied, built from the
/// fast-food contract. An unmatched `remove_at` entry is an error.
pub fn compose_fastfood_kit(root: &Path, room: &Room) -> Result<EnvKit, String> {
    let contract = load_contract(root, FASTFOOD_CONTRACT).map_err(|e| e.to_string())?;
    let dressing = dressing(root).map_err(|e| e.to_string())?;
    crate::f01_boiler::compose_kit_hooked(
        root,
        room,
        &[],
        Some(contract),
        |layout| match dress_with(layout, &dressing, FASTFOOD_CONTRACT) {
            0 => Ok(()),
            n => Err(format!("{ROOM_ID}: {n} remove_at entries match no piece")),
        },
    )
}

// ------------------------------------------------------------------ presenter geometry

/// The door canvas's top-left (layout px, y down): the lock's solid x 1440..1455 is canvas
/// columns 4..19, row 0 is the slab's underside (y 352), row 227 the walking line (y 579).
pub const DOOR_AT: (i32, i32) = (1436, 352);
/// The box canvas's top-left: bottom row 23 on the walking line, centred on the grant
/// column x 1200..1240.
pub const BOX_AT: (i32, i32) = (1204, 556);
/// The sparkle canvas's top-left: 16 x 16, centred on (1220, 562).
pub const SPARKLE_AT: (i32, i32) = (1212, 554);
/// The bell canvas's top-left: 8 x 6, its bottom row on the ledge's top (y 447).
pub const BELL_AT: (i32, i32) = (1380, 442);

/// The door's rise: ticks 1..12 after the open event, the panel raised by `RISE[k - 1]` px
/// (spec section 6). The 14-frame door atlas holds each crop pre-baked (gate note G5).
pub const RISE: [i32; 12] = [19, 37, 59, 85, 112, 139, 165, 188, 205, 217, 222, 224];
/// The rise and the bell last this many ticks from the open event.
pub const RISE_TICKS: u64 = 12;
/// The bell holds each frame (`ding_a`, `ding_b`, `ding_a`) this many ticks.
pub const BELL_STEP: u64 = 4;
/// The box's glint alternates every this many presentation ticks.
pub const GLINT_TICKS: u64 = 24;
/// The sparkle burst lasts this many ticks from the grant, three frames of [`SPARKLE_STEP`].
pub const SPARKLE_TICKS: u64 = 12;
pub const SPARKLE_STEP: u64 = 4;

/// The door's 14 atlas pieces: closed, rising 1..12, open (the order of [`DoorFrame::index`]).
pub const DOOR_PIECES: [&str; 14] = [
    "ff_staff_door_closed",
    "ff_staff_door_rise_01",
    "ff_staff_door_rise_02",
    "ff_staff_door_rise_03",
    "ff_staff_door_rise_04",
    "ff_staff_door_rise_05",
    "ff_staff_door_rise_06",
    "ff_staff_door_rise_07",
    "ff_staff_door_rise_08",
    "ff_staff_door_rise_09",
    "ff_staff_door_rise_10",
    "ff_staff_door_rise_11",
    "ff_staff_door_rise_12",
    "ff_staff_door_open",
];
/// The box's pieces: `full_a`, `full_b`, `empty`.
pub const BOX_PIECES: [&str; 3] = [
    "ff_reward_box_full_a",
    "ff_reward_box_full_b",
    "ff_reward_box_empty",
];
/// The sparkle's three frames.
pub const SPARKLE_PIECES: [&str; 3] = [
    "ff_reward_sparkle_a",
    "ff_reward_sparkle_b",
    "ff_reward_sparkle_c",
];
/// The bell's pieces: `idle`, `ding_a`, `ding_b`.
pub const BELL_PIECES: [&str; 3] = ["ff_bell_idle", "ff_bell_ding_a", "ff_bell_ding_b"];

/// The atlas of the door's frames, and of the box, sparkle and bell.
pub const DOOR_ATLAS: &str = "floor01_ff_door";
pub const PICKUP_ATLAS: &str = "floor01_ff_pickup";

/// The staff door's frame (spec section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorFrame {
    /// The lock is closed: the full-height panel.
    Closed,
    /// The panel rises into the slab: tick `k` (1..=12) after the open event, raised by
    /// `RISE[k - 1]` px.
    Rising(u8),
    /// The lock is open: rails only.
    Open,
}

impl DoorFrame {
    /// The index into [`DOOR_PIECES`].
    pub const fn index(self) -> usize {
        match self {
            Self::Closed => 0,
            Self::Rising(k) => k as usize,
            Self::Open => 13,
        }
    }

    pub const fn piece(self) -> &'static str {
        DOOR_PIECES[self.index()]
    }
}

/// The door frame to draw (gate note G4): **the lock's state is the source of truth**,
/// never a tick difference. A closed lock is `Closed` (every launch and fixture restart).
/// An open lock is `Rising(now - t + 1)` on the 12 ticks from the open event's tick `t`
/// (so tick `t` draws rising frame 1) and `Open` otherwise: a checkpoint respawn with the
/// lock open, a rewound tick counter and an unknown event all draw `Open` at once.
pub fn door_frame(lock_open: bool, open_tick: Option<u64>, now: u64) -> DoorFrame {
    if !lock_open {
        return DoorFrame::Closed;
    }
    match open_tick {
        Some(t) if t <= now && now < t + RISE_TICKS => DoorFrame::Rising((now - t + 1) as u8),
        _ => DoorFrame::Open,
    }
}

/// The Light on Your Feet box's frame (spec section 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxFrame {
    FullA,
    FullB,
    /// The boost is held: the opened, spent box.
    Empty,
    /// The 12 ticks after the grant: sparkle frame `k` (0..=2) drawn **over** the empty
    /// box.
    Sparkle(u8),
}

impl BoxFrame {
    /// The index into [`BOX_PIECES`] of the box frame itself (a sparkle sits over empty).
    pub const fn index(self) -> usize {
        match self {
            Self::FullA => 0,
            Self::FullB => 1,
            Self::Empty | Self::Sparkle(_) => 2,
        }
    }

    /// The index into [`SPARKLE_PIECES`] drawn over the box, if any.
    pub const fn sparkle(self) -> Option<usize> {
        match self {
            Self::Sparkle(k) => Some(k as usize),
            _ => None,
        }
    }
}

/// The box frame to draw (gate note G4): `empty` **iff** the boost is held. Not held: the
/// full box, its glint alternating every [`GLINT_TICKS`] presentation ticks
/// (`t_present`). Held: the sparkle over the empty box on the 12 ticks from the grant
/// tick `g`, then `Empty`. A launch with the boost already held (the `All` loadout) has
/// no grant tick and shows `Empty` from the first tick; a fixture restart drops the boost
/// and shows `Full` again.
pub fn box_frame(held: bool, grant_tick: Option<u64>, now: u64, t_present: u64) -> BoxFrame {
    if !held {
        return if (t_present / GLINT_TICKS).is_multiple_of(2) {
            BoxFrame::FullA
        } else {
            BoxFrame::FullB
        };
    }
    match grant_tick {
        Some(g) if g <= now && now < g + SPARKLE_TICKS => {
            BoxFrame::Sparkle(((now - g) / SPARKLE_STEP) as u8)
        }
        _ => BoxFrame::Empty,
    }
}

/// The service bell's frame (spec section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BellFrame {
    Idle,
    DingA,
    DingB,
}

impl BellFrame {
    /// The index into [`BELL_PIECES`].
    pub const fn index(self) -> usize {
        match self {
            Self::Idle => 0,
            Self::DingA => 1,
            Self::DingB => 2,
        }
    }
}

/// The bell frame to draw: from the door's open tick `t` (same guard as [`door_frame`]),
/// `DingA` on `t..t+3`, `DingB` on `t+4..t+7`, `DingA` on `t+8..t+11`, otherwise `Idle`.
pub fn bell_frame(lock_open: bool, open_tick: Option<u64>, now: u64) -> BellFrame {
    match open_tick {
        Some(t) if lock_open && t <= now && now < t + RISE_TICKS => {
            if ((now - t) / BELL_STEP).is_multiple_of(2) {
                BellFrame::DingA
            } else {
                BellFrame::DingB
            }
        }
        _ => BellFrame::Idle,
    }
}

/// The tick of the staff door's latest opening in this attempt: the last
/// [`CombatEvent::SurfaceLockOpened`] after the last [`CombatEvent::Restart`] (a fixture
/// restart closes the lock again, so an earlier attempt's event is not this attempt's).
pub fn last_open_tick(events: &[CombatEvent]) -> Option<u64> {
    let mut at = None;
    for event in events {
        match *event {
            CombatEvent::Restart { .. } => at = None,
            CombatEvent::SurfaceLockOpened { tick, .. } => at = Some(tick),
            _ => {}
        }
    }
    at
}

/// The tick Donut was granted Light on Your Feet in this attempt: the last
/// [`CombatEvent::CapabilityGranted`] of `jump_reach_boost` after the last restart.
pub fn last_grant_tick(events: &[CombatEvent]) -> Option<u64> {
    let mut at = None;
    for event in events {
        match *event {
            CombatEvent::Restart { .. } => at = None,
            CombatEvent::CapabilityGranted {
                tick, capability, ..
            } if capability == crate::donut_prog::JUMP_REACH_BOOST.as_str() => at = Some(tick),
            _ => {}
        }
    }
    at
}

/// The atlas rectangle `[x, y, w, h]` of each presenter frame, from the contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenterRects {
    pub door: [[i32; 4]; 14],
    pub boxes: [[i32; 4]; 3],
    pub sparkle: [[i32; 4]; 3],
    pub bell: [[i32; 4]; 3],
}

/// Read every presenter frame's atlas rect from the fast-food contract.
pub fn presenter_rects(root: &Path) -> Result<PresenterRects, String> {
    let contract = load_contract(root, FASTFOOD_CONTRACT).map_err(|e| e.to_string())?;
    let rect = |id: &str, atlas: &str| -> Result<[i32; 4], String> {
        let piece = contract
            .pieces
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("the fast-food contract has no piece {id}"))?;
        if piece.atlas != atlas {
            return Err(format!("{id} is in {}, not {atlas}", piece.atlas));
        }
        Ok(piece.rect)
    };
    let mut out = PresenterRects {
        door: [[0; 4]; 14],
        boxes: [[0; 4]; 3],
        sparkle: [[0; 4]; 3],
        bell: [[0; 4]; 3],
    };
    for (slot, id) in out.door.iter_mut().zip(DOOR_PIECES) {
        *slot = rect(id, DOOR_ATLAS)?;
    }
    for (slot, id) in out.boxes.iter_mut().zip(BOX_PIECES) {
        *slot = rect(id, PICKUP_ATLAS)?;
    }
    for (slot, id) in out.sparkle.iter_mut().zip(SPARKLE_PIECES) {
        *slot = rect(id, PICKUP_ATLAS)?;
    }
    for (slot, id) in out.bell.iter_mut().zip(BELL_PIECES) {
        *slot = rect(id, PICKUP_ATLAS)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
