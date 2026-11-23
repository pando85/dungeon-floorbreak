//! BOIL-E (K10): the code-drawn **stand-in art** of the Goblin Boiler's props,
//! the pure half. A function of the encounter's authored props, the **read-only**
//! env runtime and the tick, it returns the rectangles a presenter draws:
//!
//! - the boiler landmark (P4), a big background body that changes silhouette when
//!   its connected pieces break, with no collision and no damage of its own;
//! - the valve in its four states, the steam telegraph and the steam plume;
//! - the brace and the hanging weight (supported, weakened, released and swaying,
//!   falling, landed as the low rubble remnant) and the footprint shadow;
//! - the drum (armed, telegraph, detonated as a scorch mark) and its area tell.
//!
//! **Presentation only.** Nothing here mutates the runtime (every input is a
//! shared reference), draws a random number or is hashed, replayed or read back by
//! the simulation: a run with the art drawn and one without have the same state
//! hash (a test). Every piece is a function of the machine states and the tick, so
//! a render that skips frames or a replay seek draws the same picture for the same
//! tick. The explosion of the drum is **not** drawn here: its `BlastCharge` already
//! plays the shared blast presentation (`blast_art`), which is the "cue reuse" of
//! BOIL-E; the sounds reuse the existing cue ids (`audio`).
//!
//! **No production art exists yet** (ART-BOIL is a parallel owner-level track,
//! K10), so these are labelled stand-ins in the Floor 01 copper and brick tones:
//! plain rectangles with a [`Piece::label`] each, never a claim of approved art.
//! The sizes and tones are adopted defaults (BOILE-AD3); the owner may retune
//! them after looking at the capture. Telegraph pieces sit on [`Layer::Tell`],
//! above every other piece, so no prop or backdrop hides a warning.
//!
//! Coordinates are layout pixels, y down, like the sidecar. A `RING-D` append adds
//! the shutter and the Rot Sticker arms to [`pieces`].
//!
//! Each stand-in also carries a visible "STAND-IN: <name>" text label
//! ([`labels::tags`]), so a flat rectangle is never read as a broken texture.

pub mod labels;

use crate::camera_lab::room::IRect;
use crate::combat_scene::EnvRuntime;
use crate::encounter::{Encounter, PropKind};
use crate::env_hazard::{HazardClass, NavPhase};
use crate::env_prop::{DrumState, MassState, ValveState};

/// The tones of the stand-ins: `[r, g, b, a]`.
pub const PALETTE: [[u8; 4]; 10] = [
    [0x8a, 0x4a, 0x2c, 0xff], // Copper
    [0x4e, 0x2a, 0x1c, 0xff], // CopperDark
    [0x3a, 0x38, 0x3c, 0xff], // Iron
    [0x6a, 0x66, 0x62, 0xff], // Rubble
    [0xb2, 0x52, 0x2a, 0xff], // Rust (the drum)
    [0xff, 0xb0, 0x30, 0xff], // Warn
    [0x1c, 0x16, 0x14, 0xff], // Scorch
    [0xe8, 0xee, 0xf0, 0x90], // Steam
    [0xe8, 0xee, 0xf0, 0x40], // SteamHint
    [0xff, 0xb0, 0x30, 0x58], // Shadow
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Copper = 0,
    CopperDark = 1,
    Iron = 2,
    Rubble = 3,
    Rust = 4,
    Warn = 5,
    Scorch = 6,
    Steam = 7,
    SteamHint = 8,
    Shadow = 9,
}

impl Tone {
    pub const fn rgba(self) -> [u8; 4] {
        PALETTE[self as usize]
    }
}

/// The draw order, back to front. The presenter maps each layer to a z.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    /// Behind the actors (the boiler landmark).
    Back,
    /// With the lane props.
    Prop,
    /// Warnings and the steam, above everything (never hidden).
    Tell,
}

/// One rectangle of a stand-in, layout px (y down).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub rect: IRect,
    pub tone: Tone,
    pub layer: Layer,
    /// What it stands in for (a label for the capture and the tests).
    pub label: &'static str,
}

/// The most pieces [`pieces`] can return for the Boiler, with margin: the
/// presenter keeps a pool of this many sprites.
pub const MAX_PIECES: usize = 48;

/// The boiler landmark's height above the lane (BOILE-AD3).
pub const BOILER_HEIGHT_PX: i32 = 176;
/// The valve's wheel: its width and height (BOILE-AD3).
pub const WHEEL_W_PX: i32 = 8;
pub const WHEEL_H_PX: i32 = 6;
/// The drum and valve tells flash a tone swap every this many ticks.
pub const FLASH_TICKS: u64 = 4;
/// A released weight sways by one pixel each way, a step every this many ticks.
pub const SWAY_TICKS: u64 = 6;
/// The steam plume's puffs: how many bands and how many ticks one cycle takes.
pub const PUFF_BANDS: i32 = 3;
pub const PUFF_TICKS: u64 = 5;

const fn rect(x: i32, y: i32, w: i32, h: i32) -> IRect {
    IRect { x, y, w, h }
}

/// A 2 px outline of `r` (four edge pieces), for an area tell.
fn outline(r: IRect, tone: Tone, label: &'static str, out: &mut Vec<Piece>) {
    let t = 2.min(r.w / 2).min(r.h / 2).max(1);
    for edge in [
        rect(r.x, r.y, r.w, t),
        rect(r.x, r.bottom() - t, r.w, t),
        rect(r.x, r.y + t, t, r.h - 2 * t),
        rect(r.right() - t, r.y + t, t, r.h - 2 * t),
    ] {
        if edge.w > 0 && edge.h > 0 {
            out.push(Piece {
                rect: edge,
                tone,
                layer: Layer::Tell,
                label,
            });
        }
    }
}

/// The boiler landmark, between the valve and the steam outlet, standing on the
/// valve's base line. `None` when the room has no valve or no steam hazard.
fn landmark(enc: &Encounter) -> Option<IRect> {
    let (valve_prop, _) = enc
        .props
        .iter()
        .enumerate()
        .find(|(_, p)| p.kind == PropKind::Valve)?;
    let valve = enc.props[valve_prop].target;
    let outlet = enc
        .hazards
        .iter()
        .find(|h| h.spec.class == HazardClass::Steam)?
        .spec
        .rect
        .x;
    let x = valve.right();
    (outlet > x + 16).then(|| {
        rect(
            x,
            valve.bottom() - BOILER_HEIGHT_PX,
            outlet - x,
            BOILER_HEIGHT_PX,
        )
    })
}

/// The pieces of the props at `tick`, back to front within each prop. Read-only.
pub fn pieces(enc: &Encounter, env: &EnvRuntime, tick: u64) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::with_capacity(MAX_PIECES);
    let flash = (tick / FLASH_TICKS).is_multiple_of(2);
    let broken = |env: &EnvRuntime| {
        enc.props.iter().enumerate().any(|(i, p)| match p.kind {
            PropKind::Valve => matches!(
                env.valve_state(i),
                Some(ValveState::Venting | ValveState::Spent)
            ),
            PropKind::WeightRig => env.mass_state(i) == Some(MassState::Landed),
            _ => false,
        })
    };
    // P4: the landmark. Its silhouette changes once a connected piece broke.
    if let Some(body) = landmark(enc) {
        let dark = broken(env);
        out.push(Piece {
            rect: body,
            tone: if dark { Tone::CopperDark } else { Tone::Copper },
            layer: Layer::Back,
            label: "boiler landmark (stand-in)",
        });
        // The band across it and the dome on top (the silhouette).
        out.push(Piece {
            rect: rect(body.x, body.y + body.h / 2 - 4, body.w, 8),
            tone: Tone::Iron,
            layer: Layer::Back,
            label: "boiler band",
        });
        let dome = rect(body.x + body.w / 4, body.y - 12, body.w / 2, 12);
        out.push(Piece {
            rect: dome,
            tone: if dark { Tone::CopperDark } else { Tone::Copper },
            layer: Layer::Back,
            label: "boiler dome",
        });
        if dark {
            // The damaged-state notch: a dark bite out of the dome's side.
            out.push(Piece {
                rect: rect(dome.right() - 14, dome.y, 14, 8),
                tone: Tone::Scorch,
                layer: Layer::Back,
                label: "boiler damage notch",
            });
        }
    }
    for (i, prop) in enc.props.iter().enumerate() {
        let t = prop.target;
        match prop.kind {
            PropKind::Valve => {
                let state = env.valve_state(i);
                let tone = match state {
                    Some(ValveState::Intact) | None => Tone::Copper,
                    Some(ValveState::Warning) => {
                        if flash {
                            Tone::Warn
                        } else {
                            Tone::Copper
                        }
                    }
                    Some(ValveState::Venting) => Tone::CopperDark,
                    Some(ValveState::Spent) => Tone::Scorch,
                };
                out.push(Piece {
                    rect: t,
                    tone,
                    layer: Layer::Prop,
                    label: "valve body (stand-in)",
                });
                // The wheel is gone once the pipe ruptured.
                if matches!(state, Some(ValveState::Intact | ValveState::Warning) | None) {
                    out.push(Piece {
                        rect: rect(
                            t.x + t.w / 2 - WHEEL_W_PX / 2,
                            t.y - WHEEL_H_PX,
                            WHEEL_W_PX,
                            WHEEL_H_PX,
                        ),
                        tone: Tone::Iron,
                        layer: Layer::Prop,
                        label: "valve wheel",
                    });
                }
            }
            PropKind::WeightRig => {
                let Some(mass) = env.mass(i) else { continue };
                let state = mass.state();
                let sway = match state {
                    MassState::Released => match (tick / SWAY_TICKS) % 4 {
                        1 => 1,
                        3 => -1,
                        _ => 0,
                    },
                    _ => 0,
                };
                // The brace: whole, cracked (a warn band), or its stump.
                let brace_tone = if state == MassState::Weakened {
                    Tone::Rust
                } else {
                    Tone::Copper
                };
                if matches!(state, MassState::Supported | MassState::Weakened) {
                    out.push(Piece {
                        rect: t,
                        tone: brace_tone,
                        layer: Layer::Prop,
                        label: "brace (stand-in)",
                    });
                    if state == MassState::Weakened {
                        out.push(Piece {
                            rect: rect(t.x, t.y + t.h / 2 - 1, t.w, 2),
                            tone: Tone::Warn,
                            layer: Layer::Prop,
                            label: "brace crack",
                        });
                    }
                } else {
                    // The stump of the broken brace.
                    out.push(Piece {
                        rect: rect(t.x, t.y + t.h / 2, t.w, t.h - t.h / 2),
                        tone: Tone::CopperDark,
                        layer: Layer::Prop,
                        label: "brace stump",
                    });
                }
                if let Some(body) = mass.mass_rect() {
                    // The chain runs up from the weight while it still hangs.
                    if state != MassState::Falling {
                        let chain_x = body.x + body.w / 2 - 1 + sway;
                        out.push(Piece {
                            rect: rect(chain_x, body.y - 48, 2, 48),
                            tone: Tone::Iron,
                            layer: Layer::Prop,
                            label: "weight chain",
                        });
                    }
                    out.push(Piece {
                        rect: rect(body.x + sway, body.y, body.w, body.h),
                        tone: Tone::Iron,
                        layer: Layer::Prop,
                        label: "hanging weight (stand-in)",
                    });
                }
                if let Some(remnant) = mass.remnant() {
                    out.push(Piece {
                        rect: remnant,
                        tone: Tone::Rubble,
                        layer: Layer::Prop,
                        label: "landed remnant (stand-in)",
                    });
                    out.push(Piece {
                        rect: rect(remnant.x + 6, remnant.y - 4, remnant.w / 3, 4),
                        tone: Tone::Iron,
                        layer: Layer::Prop,
                        label: "rubble chunk",
                    });
                }
            }
            PropKind::Drum => {
                let state = env.drum_state(i);
                match state {
                    Some(DrumState::Detonated) => {
                        // The drum is gone: a flat scorch mark on the lane.
                        out.push(Piece {
                            rect: rect(t.x - 4, t.bottom() - 3, t.w + 8, 3),
                            tone: Tone::Scorch,
                            layer: Layer::Prop,
                            label: "drum scorch (stand-in)",
                        });
                    }
                    other => {
                        let tone = if other == Some(DrumState::Telegraph) && flash {
                            Tone::Warn
                        } else {
                            Tone::Rust
                        };
                        out.push(Piece {
                            rect: t,
                            tone,
                            layer: Layer::Prop,
                            label: "fuel drum (stand-in)",
                        });
                        out.push(Piece {
                            rect: rect(t.x, t.y + t.h / 2 - 1, t.w, 2),
                            tone: Tone::Iron,
                            layer: Layer::Prop,
                            label: "drum band",
                        });
                    }
                }
                if let Some(area) = env.drum_telegraph_rect(i) {
                    outline(area, Tone::Warn, "drum blast tell", &mut out);
                }
            }
            // RING-D appends the shutter and the Rot Sticker here.
            PropKind::RotSticker | PropKind::Shutter => {}
        }
    }
    // The hazard tells and the steam, from the same feed the companion reads.
    for nav in env.nav_feed() {
        match (nav.class, nav.phase) {
            (HazardClass::Steam, NavPhase::Telegraph) => {
                out.push(Piece {
                    rect: nav.rect,
                    tone: Tone::SteamHint,
                    layer: Layer::Tell,
                    label: "steam warning area",
                });
                outline(nav.rect, Tone::Warn, "steam warning edge", &mut out);
            }
            (HazardClass::Steam, NavPhase::Active) => {
                // A rolling plume: bands of steam shifting up with the tick.
                let band_h = (nav.rect.h / PUFF_BANDS).max(1);
                let shift = ((tick / PUFF_TICKS) % PUFF_BANDS as u64) as i32;
                for b in 0..PUFF_BANDS {
                    let y = nav.rect.y + ((b + shift) % PUFF_BANDS) * band_h;
                    out.push(Piece {
                        rect: rect(nav.rect.x + (b % 2) * 6, y, nav.rect.w - 6, band_h - 2),
                        tone: Tone::Steam,
                        layer: Layer::Tell,
                        label: "steam plume",
                    });
                }
            }
            (HazardClass::FallingMass, NavPhase::Telegraph) => {
                out.push(Piece {
                    rect: nav.rect,
                    tone: Tone::Shadow,
                    layer: Layer::Tell,
                    label: "weight footprint shadow",
                });
                outline(nav.rect, Tone::Warn, "weight footprint edge", &mut out);
            }
            (HazardClass::FallingMass, NavPhase::Active) => {
                out.push(Piece {
                    rect: nav.rect,
                    tone: Tone::Shadow,
                    layer: Layer::Tell,
                    label: "weight landing",
                });
            }
            // The drum's area is drawn from the drum itself; the other classes
            // have no stand-in in this room.
            _ => {}
        }
    }
    out.sort_by_key(|p| p.layer);
    out
}

#[cfg(test)]
mod tests;
