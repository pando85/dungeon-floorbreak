//! AREA-RINGS-01: the stone shutter's frame selection (the pure half). The
//! presenter reads only `Shutter::phase()`, `edge_px()`, `lower_edge_y()` and
//! `footprint()` (`src/env_prop.rs`) and picks, per tick, how many bottom rows
//! of `ring_shutter_closed` to draw at the shutter rect and which extras to add
//! (spec `docs/02-art/area-rings-01-spec.md` section 6, RNG-AD5, RNG-AD6).
//!
//! The one counter it needs, the ticks since the phase last changed, is a
//! presentation-side clock ([`PhaseClock`]): it is read from the phase and never
//! fed back to the simulation, so nothing here can reach the replay or the state
//! hash (the `f01_ring` hash test calls the selector on every tick).
//!
//! The Bevy half (`src/bin/floor01_scene/env_prop_art.rs`) crops the atlas
//! sprite to [`ShutterFrame::crop_rows`] and places the extras.

use std::collections::BTreeMap;
use std::path::Path;

use crate::camera_lab::room::IRect;
use crate::env_prop::ShutterPhase;

/// The key piece: the closed slab (32 x 68), cropped by `edge_px` in motion.
pub const SLAB: &str = "ring_shutter_closed";
pub const GRIT_A: &str = "ring_shutter_grit_a";
pub const GRIT_B: &str = "ring_shutter_grit_b";
pub const SHADOW_A: &str = "ring_shutter_shadow_a";
pub const SHADOW_B: &str = "ring_shutter_shadow_b";
pub const SHADOW_C: &str = "ring_shutter_shadow_c";
pub const DUST_A: &str = "ring_shutter_dust_a";
pub const DUST_B: &str = "ring_shutter_dust_b";
pub const DUST_C: &str = "ring_shutter_dust_c";

/// Every piece the presenter draws (the slab and the extras).
pub const PIECES: [&str; 9] = [
    SLAB, GRIT_A, GRIT_B, SHADOW_A, SHADOW_B, SHADOW_C, DUST_A, DUST_B, DUST_C,
];

/// The label of the greybox rectangle that stands in for the slab while the
/// ring atlas is missing (`bash scripts/export_assets.sh` was not run).
pub const STANDIN_LABEL: &str = "shutter (stand-in until the ring atlas loads)";

/// The warning's tick windows of the floor shadow (spec section 6).
pub const SHADOW_B_FROM: u32 = 14;
pub const SHADOW_C_FROM: u32 = 27;
/// Grit alternates between its two pieces every this many ticks.
pub const GRIT_TICKS: u32 = 6;
/// Dust alternates every this many ticks while the slab moves.
pub const DUST_TICKS: u32 = 4;
/// The floor-contact puff lasts this many ticks after entering Closed.
pub const CONTACT_TICKS: u32 = 8;
/// The grit falls from the slot, this far below the shutter's top.
pub const GRIT_DY: i32 = 6;
/// The shadow lies on the floor under the slot: left of the rect by this much,
/// this far above the rect's bottom.
pub const SHADOW_DX: i32 = 4;
pub const SHADOW_UP: i32 = 4;
/// The dust is drawn left of the rect by this much, this far above the edge.
pub const DUST_DX: i32 = 8;
pub const DUST_UP: i32 = 6;

/// One extra piece to draw, at layout px (top-left).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extra {
    pub piece: &'static str,
    pub x: i32,
    pub y: i32,
}

/// What to draw this tick: the key's bottom `crop_rows` rows at the shutter
/// rect's top-left (0: no slab), plus the extras.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShutterFrame {
    pub crop_rows: i32,
    pub extras: Vec<Extra>,
}

fn extra(piece: &'static str, x: i32, y: i32) -> Extra {
    Extra { piece, x, y }
}

/// Spec section 6, row by row. `rect` is the closed rectangle (its bottom is the
/// walking line), `edge_px` the lower edge's descent from its top and
/// `ticks_in_phase` the presentation clock (0 on the first tick of the phase;
/// `u32::MAX` when the phase was never seen to start, which draws no puff).
pub fn shutter_frame(
    rect: IRect,
    phase: ShutterPhase,
    edge_px: i32,
    ticks_in_phase: u32,
) -> ShutterFrame {
    let edge_px = edge_px.clamp(0, rect.h);
    let lower_edge = rect.y + edge_px;
    let dust = |alternate: u32| {
        extra(
            if (ticks_in_phase / alternate).is_multiple_of(2) {
                DUST_A
            } else {
                DUST_B
            },
            rect.x - DUST_DX,
            lower_edge - DUST_UP,
        )
    };
    let shadow = |piece: &'static str| extra(piece, rect.x - SHADOW_DX, rect.bottom() - SHADOW_UP);
    match phase {
        ShutterPhase::Open => ShutterFrame {
            crop_rows: 0,
            extras: Vec::new(),
        },
        ShutterPhase::Warning => ShutterFrame {
            crop_rows: 0,
            extras: vec![
                shadow(if ticks_in_phase < SHADOW_B_FROM {
                    SHADOW_A
                } else if ticks_in_phase < SHADOW_C_FROM {
                    SHADOW_B
                } else {
                    SHADOW_C
                }),
                extra(
                    if (ticks_in_phase / GRIT_TICKS).is_multiple_of(2) {
                        GRIT_A
                    } else {
                        GRIT_B
                    },
                    rect.x,
                    rect.y + GRIT_DY,
                ),
            ],
        },
        ShutterPhase::Closing => ShutterFrame {
            crop_rows: edge_px,
            extras: vec![shadow(SHADOW_C), dust(DUST_TICKS)],
        },
        ShutterPhase::Closed => ShutterFrame {
            crop_rows: edge_px,
            extras: if ticks_in_phase < CONTACT_TICKS {
                vec![extra(DUST_C, rect.x - DUST_DX, lower_edge - DUST_UP)]
            } else {
                Vec::new()
            },
        },
        ShutterPhase::Opening => ShutterFrame {
            crop_rows: edge_px,
            extras: vec![dust(DUST_TICKS)],
        },
    }
}

/// The presentation-side clock: the world tick at which the phase last changed.
/// It is keyed on the world tick (not on rendered frames), so a render that skips
/// frames draws the same picture for the same tick. The first phase it sees
/// counts as already old (`u32::MAX`), so a shutter that was never seen to enter
/// Closed (the static court) draws no contact puff.
#[derive(Debug, Clone, Copy, Default)]
pub struct PhaseClock {
    phase: Option<ShutterPhase>,
    since: u64,
    started_seen: bool,
}

impl PhaseClock {
    /// Observe the phase at world tick `tick` and return the ticks in it so far
    /// (0 on the first tick after a change).
    pub fn observe(&mut self, phase: ShutterPhase, tick: u64) -> u32 {
        match self.phase {
            None => {
                self.since = tick;
                self.started_seen = false;
            }
            Some(p) if p != phase => {
                self.since = tick;
                self.started_seen = true;
            }
            Some(_) => {}
        }
        self.phase = Some(phase);
        if self.started_seen {
            u32::try_from(tick.saturating_sub(self.since)).unwrap_or(u32::MAX - 1)
        } else {
            u32::MAX
        }
    }
}

/// The atlas rect `[x, y, w, h]` of every presenter piece, read from the rings
/// contract.
pub fn atlas_rects(root: &Path) -> Result<BTreeMap<&'static str, [i32; 4]>, String> {
    let contract = crate::env_kit::load_contract(root, crate::ring_kit::RINGS_CONTRACT)
        .map_err(|e| e.to_string())?;
    let mut out = BTreeMap::new();
    for id in PIECES {
        let piece = contract
            .pieces
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("the rings contract has no piece {id}"))?;
        out.insert(id, piece.rect);
    }
    Ok(out)
}

/// The atlas image the presenter pieces live in.
pub const ATLAS: &str = "generated/art/floor01_ring_shutter.png";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env_prop::{Shutter, ShutterConfig, ShutterInitial};
    use std::path::PathBuf;

    const FLOOR: i32 = 580;
    const ARCH: IRect = IRect {
        x: 1200,
        y: FLOOR - 68,
        w: 32,
        h: 68,
    };

    fn live() -> Shutter {
        let cfg = ShutterConfig {
            arm_x: Some(1008),
            ..ShutterConfig::defaults(ShutterInitial::Open)
        };
        Shutter::new(cfg, ARCH).unwrap()
    }

    fn shadow(piece: &'static str) -> Extra {
        extra(piece, ARCH.x - SHADOW_DX, FLOOR - SHADOW_UP)
    }

    /// Spec acceptance 5: through Open -> Warning -> Closing -> Closed -> Opening,
    /// the piece and crop of every tick equal section 6's table.
    #[test]
    fn a_full_cycle_draws_the_section_six_table_on_every_tick() {
        let mut s = live();
        let mut clock = PhaseClock::default();
        let mut world_tick = 0u64;
        let mut frame = |s: &Shutter, clock: &mut PhaseClock| {
            let t = clock.observe(s.phase(), world_tick);
            world_tick += 1;
            (t, shutter_frame(ARCH, s.phase(), s.edge_px(), t))
        };
        // Open: nothing, the housing and the jambs are placed pieces. (The clock's
        // first phase is "already old", so Open needs no special case.)
        for _ in 0..5 {
            let (_, f) = frame(&s, &mut clock);
            assert_eq!(s.phase(), ShutterPhase::Open);
            assert_eq!(
                f,
                ShutterFrame {
                    crop_rows: 0,
                    extras: vec![]
                }
            );
            s.step(&[]);
        }
        assert!(s.arm());
        // Warning: 40 ticks (tick 0 is the first observed tick of the phase).
        let mut warning = 0;
        while s.phase() == ShutterPhase::Warning {
            let (t, f) = frame(&s, &mut clock);
            assert_eq!(t, warning, "the clock counts from the phase's first tick");
            assert_eq!(f.crop_rows, 0, "no slab in the warning");
            let want_shadow = match t {
                0..=13 => SHADOW_A,
                14..=26 => SHADOW_B,
                _ => SHADOW_C,
            };
            let want_grit = if (t / 6).is_multiple_of(2) {
                GRIT_A
            } else {
                GRIT_B
            };
            assert_eq!(
                f.extras,
                vec![shadow(want_shadow), extra(want_grit, ARCH.x, ARCH.y + 6)],
                "warning tick {t}"
            );
            warning += 1;
            s.step(&[]);
        }
        assert_eq!(warning, 40, "the warning lasts 40 ticks");
        // Closing: the key's bottom edge_px rows, shadow c and alternating dust.
        let mut closing = 0;
        while s.phase() == ShutterPhase::Closing {
            let (t, f) = frame(&s, &mut clock);
            assert_eq!(f.crop_rows, s.edge_px());
            let dust = if (t / 4).is_multiple_of(2) {
                DUST_A
            } else {
                DUST_B
            };
            assert_eq!(
                f.extras,
                vec![
                    shadow(SHADOW_C),
                    extra(dust, ARCH.x - 8, s.lower_edge_y() - 6)
                ],
                "closing tick {t}"
            );
            closing += 1;
            s.step(&[]);
        }
        assert_eq!(closing, 34, "68 px at 2 px per tick");
        // Closed: the full key, the floor-contact puff for 8 ticks, then nothing.
        let mut closed = 0u32;
        while s.phase() == ShutterPhase::Closed {
            let (t, f) = frame(&s, &mut clock);
            assert_eq!(t, closed);
            assert_eq!(f.crop_rows, 68, "the full key");
            let want = if t < 8 {
                vec![extra(DUST_C, ARCH.x - 8, FLOOR - 6)]
            } else {
                vec![]
            };
            assert_eq!(f.extras, want, "closed tick {t}");
            closed += 1;
            s.step(&[]);
        }
        assert_eq!(closed, 150, "the closed dwell");
        // Opening: the bottom edge_px rows (the slab rises), alternating dust.
        let mut opening = 0;
        while s.phase() == ShutterPhase::Opening {
            let (t, f) = frame(&s, &mut clock);
            assert_eq!(f.crop_rows, s.edge_px());
            let dust = if (t / 4).is_multiple_of(2) {
                DUST_A
            } else {
                DUST_B
            };
            assert_eq!(
                f.extras,
                vec![extra(dust, ARCH.x - 8, s.lower_edge_y() - 6)],
                "opening tick {t}"
            );
            opening += 1;
            s.step(&[]);
        }
        assert_eq!(opening, 34);
        // Open again (cycles 1): nothing.
        let (_, f) = frame(&s, &mut clock);
        assert_eq!(s.phase(), ShutterPhase::Open);
        assert_eq!(
            f,
            ShutterFrame {
                crop_rows: 0,
                extras: vec![]
            }
        );
    }

    /// RNG-AD6: a static shutter (the court) is always Closed, was never seen to
    /// enter it, and so draws only the full key: no puff, no Warning, Closing or
    /// Opening extra, ever.
    #[test]
    fn a_static_shutter_draws_the_closed_key_and_never_an_extra() {
        let mut s = Shutter::new(ShutterConfig::defaults(ShutterInitial::Closed), ARCH).unwrap();
        let mut clock = PhaseClock::default();
        for tick in 0..600 {
            let t = clock.observe(s.phase(), tick);
            let f = shutter_frame(ARCH, s.phase(), s.edge_px(), t);
            assert_eq!(
                f,
                ShutterFrame {
                    crop_rows: 68,
                    extras: vec![]
                }
            );
            s.step(&[]);
        }
    }

    /// Every presenter piece is in the rings contract with the size the
    /// selector assumes (the slab 32 x 68, the extras of spec section 5).
    #[test]
    fn every_presenter_piece_is_in_the_contract_with_its_size() {
        let rects = atlas_rects(&PathBuf::from(env!("CARGO_MANIFEST_DIR"))).unwrap();
        assert_eq!(rects.len(), 9);
        let size = |id: &str| (rects[id][2], rects[id][3]);
        assert_eq!(size(SLAB), (32, 68));
        assert_eq!(size(GRIT_A), (32, 28));
        assert_eq!(size(GRIT_B), (32, 28));
        for id in [SHADOW_A, SHADOW_B, SHADOW_C] {
            assert_eq!(size(id), (40, 4), "{id}");
        }
        for id in [DUST_A, DUST_B, DUST_C] {
            assert_eq!(size(id), (48, 10), "{id}");
        }
    }
}
