//! BOS-ART: the art binding of the Ball of Swine's room 18. Pure and headless, no Bevy types.
//!
//! The body is the SWINE-01 atlas (`generated/art/swine-01.png`, 18 frames of 176 x 144, pivot
//! (88, 143), exported from the approved `swine-01.ase`). [`body_frame`] maps the simulation's
//! [`BossMode`] to one atlas frame; the roll is six **native** frames and is never rotated or
//! scaled (BOS-CUT7, `docs/02-art/swine-01-spec.md` section 3.1). [`weight_draws`] lists the chain
//! weight's sprites (the weight, the vertical chain it pays out, the tie to the post) at the
//! rack's sim rect. Everything here reads the simulation through shared references and returns
//! values: nothing feeds back, so a hash can never change through art. The map, the adopted
//! defaults BOS-ART-AD* and the open questions are in `docs/02-art/bos-art-spec.md`.

use crate::boss::{Boss, BossMode, RackState};

/// The atlas and its metadata, relative to the repository root.
pub const ATLAS: &str = "generated/art/swine-01.png";
pub const METADATA: &str = "generated/art/swine-01.json";
/// Frame size and pivot (the lowest opaque row of every grounded frame).
pub const FRAME: (i32, i32) = (176, 144);
pub const PIVOT: (i32, i32) = (88, 143);
/// The number of atlas frames.
pub const FRAMES: usize = 18;

/// Atlas frame indices (`swine-01.pixelart.json` tags, spec section 4).
pub mod frame {
    /// `idle`: heave in and out, 16 + 12 + 16 ticks.
    pub const IDLE: [(usize, u32); 3] = [(0, 16), (1, 12), (2, 16)];
    /// `roll`: six native frames, 4 ticks each, one turn per 24 ticks.
    pub const ROLL: [usize; 6] = [3, 4, 5, 6, 7, 8];
    pub const ROLL_TICKS: u32 = 4;
    /// `attack_tell`: the rock back (8 ticks), then the coil hold.
    pub const TELL_ROCK: usize = 9;
    pub const TELL_COIL: usize = 10;
    pub const TELL_ROCK_TICKS: u32 = 8;
    /// `attack_charge`: the launch smear (3), the impact (6, the flat front face), the rebound.
    pub const LAUNCH: usize = 11;
    pub const LAUNCH_TICKS: u32 = 3;
    pub const IMPACT: usize = 12;
    pub const IMPACT_TICKS: u32 = 6;
    pub const REBOUND: usize = 13;
    /// `hurt`: one recoil frame.
    pub const HURT: usize = 14;
    /// `down`: crack (8), slump (12), heap (hold).
    pub const CRACK: usize = 15;
    pub const SLUMP: usize = 16;
    pub const HEAP: usize = 17;
    pub const CRACK_TICKS: u32 = 8;
    pub const SLUMP_TICKS: u32 = 12;
}

/// Ticks per frame of the walk: the roll frames at half the roll rate (BOS-ART-AD2).
pub const WALK_TICKS: u32 = 8;

fn idle_frame(clock: u64) -> usize {
    let total: u32 = frame::IDLE.iter().map(|&(_, t)| t).sum();
    let mut at = (clock % u64::from(total)) as u32;
    for &(index, ticks) in &frame::IDLE {
        if at < ticks {
            return index;
        }
        at -= ticks;
    }
    frame::IDLE[0].0
}

fn roll_frame(tick: u32, per_frame: u32) -> usize {
    frame::ROLL[(tick / per_frame) as usize % frame::ROLL.len()]
}

/// The atlas frame of the body. `clock` is the sim tick (the idle loop), `walking` is whether the
/// ball moved since the last tick, `defeat_age` the ticks since the `EnemyDefeated` event
/// (`None` while the mode's own tick still counts).
pub fn body_frame(boss: &Boss, clock: u64, walking: bool, defeat_age: Option<u32>) -> usize {
    let tick = u32::from(boss.mode_tick);
    match boss.mode {
        BossMode::Dormant => idle_frame(clock),
        BossMode::Neutral if walking => roll_frame(clock as u32, WALK_TICKS),
        BossMode::Neutral => idle_frame(clock),
        BossMode::Turn => frame::IDLE[0].0,
        BossMode::PoundTell | BossMode::SecondTell | BossMode::ChargeTell => {
            if tick < frame::TELL_ROCK_TICKS {
                frame::TELL_ROCK
            } else {
                frame::TELL_COIL
            }
        }
        // The shove: the sim's 3-tick smear and 6-tick impact are the sheet's own (spec section 4).
        BossMode::PoundStrike | BossMode::SecondStrike => {
            if tick < frame::LAUNCH_TICKS {
                frame::LAUNCH
            } else {
                frame::IMPACT
            }
        }
        BossMode::PoundRecover | BossMode::SecondRecover | BossMode::DoubleStep => frame::REBOUND,
        BossMode::Charge => {
            if tick < frame::LAUNCH_TICKS {
                frame::LAUNCH
            } else {
                roll_frame(tick - frame::LAUNCH_TICKS, frame::ROLL_TICKS)
            }
        }
        // The crash: the flat face for the hit, then the open-mouthed rebound; the stun holds it.
        BossMode::Rebound => {
            if tick < frame::IMPACT_TICKS {
                frame::IMPACT
            } else {
                frame::REBOUND
            }
        }
        BossMode::Stun => frame::REBOUND,
        BossMode::Roar => frame::TELL_COIL,
        BossMode::Flinch | BossMode::Backstep => frame::HURT,
        // Under the weight: the slump for the first ticks, then the squashed heap.
        BossMode::Pinned => {
            if tick < frame::CRACK_TICKS {
                frame::SLUMP
            } else {
                frame::HEAP
            }
        }
        BossMode::Defeated => {
            if boss.defeated_cause_rack {
                frame::HEAP
            } else {
                let age = defeat_age.unwrap_or(tick);
                if age < frame::CRACK_TICKS {
                    frame::CRACK
                } else if age < frame::CRACK_TICKS + frame::SLUMP_TICKS {
                    frame::SLUMP
                } else {
                    frame::HEAP
                }
            }
        }
    }
}

/// The depth band a sprite is drawn in (the scene maps it to a z).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    /// Behind the actors, over the wall (the hung weight, the chains).
    Back,
    /// Over the boss' body, under Carl (the falling and landed weight).
    Front,
}

/// The pieces of the chain weight's atlas (ids of the kit contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Piece {
    WeightHung,
    WeightLanded,
    ChainV,
    ChainH,
}

impl Piece {
    pub const fn id(self) -> &'static str {
        match self {
            Self::WeightHung => "bos_weight_hung",
            Self::WeightLanded => "bos_weight_landed",
            Self::ChainV => "bos_chain_v",
            Self::ChainH => "bos_chain_h",
        }
    }
    /// The piece's size in px.
    pub const fn size(self) -> (f32, f32) {
        match self {
            Self::WeightHung | Self::WeightLanded => (80.0, 36.0),
            Self::ChainV => (8.0, 16.0),
            Self::ChainH => (16.0, 8.0),
        }
    }
}

/// One sprite of the weight: `left` is the world x of its left edge, `bottom` the height of its
/// bottom edge over the walking line, `rows` how many rows of the piece are drawn from its top
/// (a cropped chain tile), `band` the depth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeightDraw {
    pub piece: Piece,
    pub left: f32,
    pub bottom: f32,
    pub rows: f32,
    pub band: Band,
}

/// Height of the ceiling girder's underside over the walking line (the sim's `ceiling_px`).
pub const CEILING_PX: f32 = 224.0;
/// Height of the hung weight's bottom over the walking line (the sim's rack hang).
pub const HANG_PX: f32 = crate::boss_art::HANG_PX;
/// The weight rests this high on the pinned ball's heap (BOS-ART-AD5).
pub const PINNED_REST_PX: f32 = 40.0;
/// Chain tiles of the tie to the post.
pub const TIE_TILES: usize = 5;
/// Height of the tie over the walking line (the post's top is 196).
pub const TIE_BOTTOM_PX: f32 = 197.0;
/// The shackle's column in the weight piece (the chain's x offset).
pub const SHACKLE_X: f32 = 36.0;

/// The chain weight's sprites for this tick (empty when the room has no left rack).
pub fn weight_draws(boss: &Boss) -> Vec<WeightDraw> {
    let rack = &boss.racks[0];
    if !rack.enabled {
        return Vec::new();
    }
    let data = boss.data();
    let left = data.footprint_centre(rack.facing) - data.rack.footprint_px * 0.5;
    let mut out = Vec::new();
    let mut weight = |piece, bottom, band| {
        out.push(WeightDraw {
            piece,
            left,
            bottom,
            rows: piece.size().1,
            band,
        });
    };
    match rack.state {
        RackState::Idle { .. } => {
            weight(Piece::WeightHung, HANG_PX, Band::Back);
            for k in 1..=TIE_TILES {
                out.push(WeightDraw {
                    piece: Piece::ChainH,
                    left: left - 16.0 * k as f32,
                    bottom: TIE_BOTTOM_PX,
                    rows: 8.0,
                    band: Band::Back,
                });
            }
        }
        RackState::Falling { ticks, .. } => {
            let t = f32::from(ticks) / f32::from(data.rack.fall_ticks);
            let bottom = HANG_PX * (1.0 - t * t);
            weight(Piece::WeightHung, bottom, Band::Front);
            // The chain pays out from the girder down to the shackle.
            let top = bottom + Piece::WeightHung.size().1;
            let mut k = 0.0;
            while CEILING_PX - 16.0 * k > top {
                let tile_top = CEILING_PX - 16.0 * k;
                out.push(WeightDraw {
                    piece: Piece::ChainV,
                    left: left + SHACKLE_X,
                    bottom: (tile_top - 16.0).max(top),
                    rows: (tile_top - top).min(16.0),
                    band: Band::Back,
                });
                k += 1.0;
            }
        }
        RackState::Landed => {
            let rest = if boss.mode == BossMode::Pinned {
                PINNED_REST_PX
            } else {
                0.0
            };
            weight(Piece::WeightLanded, rest, Band::Front);
        }
    }
    out
}

/// Which greybox stand-ins of BOS-A (the placeholder rects of the body, the eye, the rack, the
/// strut, the shove's ripple, the arch outlines and the label) may still be drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Greybox {
    Body,
    Eye,
    Rack,
    Strut,
    Ripple,
    Arch,
    Label,
}

/// Whether the stand-in is drawn: never once the room is dressed (the atlas and the kit are
/// loaded). The strut and the ripple reuse the Juicer's exported art (BOS-ART-AD7); a dressed
/// room whose reused sheet is missing (`reused_art` false, never in a full export) keeps that
/// one rect only.
pub fn greybox_shown(piece: Greybox, dressed: bool, reused_art: bool) -> bool {
    match piece {
        Greybox::Strut | Greybox::Ripple => !dressed || !reused_art,
        _ => !dressed,
    }
}

#[cfg(test)]
mod tests;
