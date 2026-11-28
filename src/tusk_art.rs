//! BOSB: the Tuskling's atlas presentation, the pure half (BOSB-AD1, AD14). Which TUSK-01 frame a
//! Tuskling draws, where, mirrored or not. A function of the domain state through **shared
//! references only**, so nothing here reaches the simulation, the replay or the state hash. The
//! Bevy half is `src/bin/floor01_scene/tusk_art.rs`.
//!
//! The atlases are the committed `prototype_fixture_unapproved` candidates `tusk_01` (the tux) and
//! `tusk_01_gown`, 16 frames of 56 x 40 with the pivot at (22, 39), exported by
//! `scripts/export_assets.sh` like the Ball's (BOS-ART). The sprite is **native**: never scaled
//! and never rotated (BOSB-AD1). The costume alternates by spawn index, even tux and odd gown.
//!
//! **Frame map** (`docs/02-art/tusk-01-spec.md` section 4): `idle` 0..=2 for a standing Tuskling,
//! `scramble` 3..=6 while it moved on the last step; the tell is frame 7 for the onset ticks and
//! frame 8 for the hold; the lunge is frame 9, the strike frame 10, the recovery frame 11; a
//! flinch and a kick in flight are frame 12; `down` plays 13, 14 on their ticks and holds 15. A
//! cowering Tuskling (the Ball is down) holds frame 12, eyes shut and ears flat.

use crate::boss::TusklingData;
use crate::tuskling::{State, Tuskling};

/// The sheet's frames (one row).
pub const FRAMES: u32 = 16;
/// The frame size and the pivot inside it (x from the left, y from the top).
pub const FRAME: (u32, u32) = (56, 40);
pub const PIVOT: (u32, u32) = (22, 39);

/// The atlas of the costume of spawn index `index` (even tux, odd gown).
pub const fn atlas(index: u8) -> &'static str {
    if index.is_multiple_of(2) {
        "generated/art/tusk_01.png"
    } else {
        "generated/art/tusk_01_gown.png"
    }
}

/// The ticks the idle frames show (`tusk-01.pixelart.json`: 18, 12, 14).
pub const IDLE: [(u8, u32); 3] = [(0, 18), (1, 12), (2, 14)];
/// Ticks per scramble frame.
pub const SCRAMBLE_TICKS: u32 = 5;
/// The `down` frames: the sag (13) and the roll (14), then the hold (15).
pub const DOWN_TICKS: [u32; 2] = [8, 10];

fn idle_frame(clock: u64) -> u8 {
    let total: u32 = IDLE.iter().map(|&(_, t)| t).sum();
    let mut at = (clock % u64::from(total)) as u32;
    for &(frame, ticks) in &IDLE {
        if at < ticks {
            return frame;
        }
        at -= ticks;
    }
    IDLE[0].0
}

/// The atlas frame of a Tuskling. `down_age` is the ticks since its defeat (`None` while alive).
pub fn frame(t: &Tuskling, down_age: Option<u32>) -> u8 {
    let d: &TusklingData = t.data();
    let tick = u32::from(t.state_tick());
    match t.state {
        State::Chase => {
            if t.moving() {
                3 + ((t.age as u32 / SCRAMBLE_TICKS) % 4) as u8
            } else {
                idle_frame(t.age)
            }
        }
        State::Tell => {
            if tick <= u32::from(d.tell_onset_ticks) {
                7
            } else {
                8
            }
        }
        State::Lunge => 9,
        State::Strike => 10,
        State::Recover => 11,
        State::Hurt | State::Kicked | State::Cower => 12,
        State::Down => match down_age {
            Some(age) if age < DOWN_TICKS[0] => 13,
            Some(age) if age < DOWN_TICKS[0] + DOWN_TICKS[1] => 14,
            _ => 15,
        },
    }
}

/// Where the frame draws: the pivot's world position (x, floor y) and whether the sprite is
/// mirrored (the art faces right).
pub fn placement(t: &Tuskling) -> ((f32, f32), bool) {
    ((t.x, t.foot_y), t.facing < 0)
}
