//! AREA-HAZARDS-01B: the **static decals** and the **spent drip** of the hazard
//! rooms, the pure half (`docs/02-art/area-hazards-01b-spec.md` sections 3.2, 3.3 and
//! 4.3). A function of the room's authored data and the combat state through
//! **shared references only**:
//!
//! - the **corrosion** under every acid patch: `dfx_corr_face` tiles along the floor
//!   face (the patch's **authored** `x0..x1`, not the live rectangle, because the
//!   stain outlives the drain), one `dfx_corr_wall` trail on the backwall per patch (its
//!   pivot row on the **backwall base**, above the deck's back edge: HZB-AD17), and
//!   the const entry stains of [`ENTRY_STAINS`] (room 4's `dfx_corr_top` at x 150);
//! - the **Rot marks** on every ceiling track: a `dfx_rot_nest` at the track's `start`
//!   (HZB-AD6: ceilings only, wall nests are deferred) and a `dfx_rot_trail` scuff
//!   every 16 px along `lo..=hi`, skipped within 16 px of the nest;
//! - the **spent drip** of a Rot Sticker ([`drips`]): `drip_run_*` for ages 0..23 from
//!   the detonation tick, then `drip_drop_*` looping; the ceiling uses the `_c` tags,
//!   the walls the `_w` tags (mirrored for `WallRight`).
//!
//! The static decals are dressing, not state (HZB-AD8): [`static_decals`] computes them
//! once from the authored specs and the sticker tracks, with no sim coupling, so they
//! are deterministic and never hashed. The dry rule holds for every corrosion pixel
//! (no lit acid entry) and the Rot marks use ROT-01 entries only (HZB-AD4, HZB-AD5).
//!
//! **Presentation only.** Nothing here mutates a state, draws a random number or is
//! hashed, replayed or read back by the simulation: a run that calls every function
//! here each tick has the state hash of one that never does (a test).
//!
//! Coordinates are world px, y up, like the simulation; every [`Decal`] gives the
//! bottom-left corner of what it covers. A patch's `floor_y` is a layout row (y down)
//! and `layout_h` flips it: the **walking line** of a patch is the world row
//! `layout_h - floor_y`, and "pivot row on the walking line" means the same as for
//! `dfx_acid`: the canvas' bottom row is the first world row **above** that line.

use crate::combat_scene::{CombatState, EnvRuntime};
use crate::rot_sticker::{self, Orientation, RotSticker, StickerState, Track};
use crate::slime::AcidPatchSpec;

/// The atlases of the decals: one per canvas, a single row of equal cells exported by
/// `scripts/export_assets.sh` from `assets/manifests/f01-dfx-*.json` (source grids
/// `assets/source/props/floor01/dfx_*.grid.json`, written by
/// `scripts/author_danger_fx.py` through `scripts/author_hazard_decals.py`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DecalSheet {
    CorrFace,
    CorrTop,
    CorrWall,
    RotNest,
    RotTrail,
    RotDrip,
}

impl DecalSheet {
    pub const ALL: [Self; 6] = [
        Self::CorrFace,
        Self::CorrTop,
        Self::CorrWall,
        Self::RotNest,
        Self::RotTrail,
        Self::RotDrip,
    ];

    /// The manifest under `assets/manifests/` (without `.json`).
    pub const fn manifest(self) -> &'static str {
        match self {
            Self::CorrFace => "f01-dfx-corr-face",
            Self::CorrTop => "f01-dfx-corr-top",
            Self::CorrWall => "f01-dfx-corr-wall",
            Self::RotNest => "f01-dfx-rot-nest",
            Self::RotTrail => "f01-dfx-rot-trail",
            Self::RotDrip => "f01-dfx-rot-drip",
        }
    }

    /// The exported atlas, relative to the repository root.
    pub const fn atlas(self) -> &'static str {
        match self {
            Self::CorrFace => "generated/art/f01-dfx-corr-face.png",
            Self::CorrTop => "generated/art/f01-dfx-corr-top.png",
            Self::CorrWall => "generated/art/f01-dfx-corr-wall.png",
            Self::RotNest => "generated/art/f01-dfx-rot-nest.png",
            Self::RotTrail => "generated/art/f01-dfx-rot-trail.png",
            Self::RotDrip => "generated/art/f01-dfx-rot-drip.png",
        }
    }

    /// One cell of the atlas, in px.
    pub const fn frame_size(self) -> (i32, i32) {
        match self {
            Self::CorrFace => (FACE_W, FACE_H),
            Self::CorrTop => (TOP_W, TOP_H),
            Self::CorrWall => (WALL_W, WALL_H),
            Self::RotNest => (NEST_W, NEST_H),
            Self::RotTrail => (TRAIL_W, TRAIL_H),
            Self::RotDrip => (DRIP_W, DRIP_H),
        }
    }

    /// The number of frames of the atlas.
    pub const fn frames(self) -> usize {
        match self {
            Self::RotDrip => DRIP_FRAMES,
            _ => 1,
        }
    }
}

/// `dfx_corr_face`: 32 x 10, row 0 is the first solid row under the pool's bottom row.
pub const FACE_W: i32 = 32;
pub const FACE_H: i32 = 10;
/// `dfx_corr_top`: 32 x 6, the bottom row on the walking line.
pub const TOP_W: i32 = 32;
pub const TOP_H: i32 = 6;
/// `dfx_corr_wall`: 12 x 24, the bottom row on the backwall base (HZB-AD17).
pub const WALL_W: i32 = 12;
pub const WALL_H: i32 = 24;
/// `dfx_rot_nest`: 32 x 12, row 0 on the ceiling's underside line.
pub const NEST_W: i32 = 32;
pub const NEST_H: i32 = 12;
/// `dfx_rot_trail`: 16 x 3, row 0 on the ceiling's underside line.
pub const TRAIL_W: i32 = 16;
pub const TRAIL_H: i32 = 3;
/// `dfx_rot_drip`: 8 x 16, ceiling pivot (4, 0), wall pivot (0, 0).
pub const DRIP_W: i32 = 8;
pub const DRIP_H: i32 = 16;

/// The face tiles are cropped to the patch's authored ends plus this margin, px.
pub const FACE_MARGIN_PX: i32 = 4;
/// The wall trail sits this far in from the patch's authored left end, px.
pub const WALL_INSET_PX: i32 = 6;
/// A scuff every this many px along a ceiling track.
pub const TRAIL_STEP_PX: i32 = 16;
/// No scuff within this many px of the nest.
pub const TRAIL_NEST_GAP_PX: i32 = 16;
/// No floor-top stain within this many px of an acid patch's authored ends.
pub const TOP_PATCH_GAP_PX: i32 = 24;

/// The const entry stains: `(fixture name, x of the stain centre)`, drawn as one
/// `dfx_corr_top` with its bottom row on the walking line of the room's first patch.
/// Room 4 (`f01_hazards`) gets the "teach the look" stain of the room layout note
/// (enemy gameplay: slime and Rot, section 15), well before its first pool.
pub const ENTRY_STAINS: &[(&str, i32)] = &[("f01_hazards", 150)];

/// The drip: `drip_run_*` plays on ages `0..DRIP_RUN_TICKS`, then `drip_drop_*` loops.
pub const DRIP_RUN_FRAMES: usize = 4;
pub const DRIP_RUN_TICK: u64 = 6;
pub const DRIP_RUN_TICKS: u64 = DRIP_RUN_FRAMES as u64 * DRIP_RUN_TICK;
pub const DRIP_DROP_FRAMES: usize = 3;
pub const DRIP_DROP_TICK: u64 = 20;
/// The atlas frames of the drip: `drip_run_c`, `drip_drop_c`, `drip_run_w`, `drip_drop_w`.
pub const DRIP_FRAMES: usize = 2 * (DRIP_RUN_FRAMES + DRIP_DROP_FRAMES);
const DRIP_RUN_C: usize = 0;
const DRIP_DROP_C: usize = DRIP_RUN_C + DRIP_RUN_FRAMES;
const DRIP_RUN_W: usize = DRIP_DROP_C + DRIP_DROP_FRAMES;
const DRIP_DROP_W: usize = DRIP_RUN_W + DRIP_RUN_FRAMES;

/// The draw order of the decals; the presenter maps each layer to a z.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecalLayer {
    /// The backwall trail: just over the backwall decals, on the backwall base.
    Wall,
    /// The floor stains: over the floor solid, under the acid.
    Floor,
    /// The ceiling nest and its scuffs: over the ceiling solid, under the Rot shell.
    Nest,
    /// The spent drip: over the nest, under the Rot shell (`sticker_z - 0.001`).
    Drip,
}

/// One sprite: `frame` of `sheet`, the cell's columns `src_x..src_x + w` (the whole
/// cell unless cropped, taken from the unmirrored cell), mirrored when `flip_x`, its
/// bottom-left at world `(x, y)`, `w` x `h` px.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decal {
    pub sheet: DecalSheet,
    pub frame: usize,
    pub src_x: i32,
    pub flip_x: bool,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub layer: DecalLayer,
}

impl Decal {
    /// A whole cell with its bottom-left at `(x, y)`.
    fn whole(
        sheet: DecalSheet,
        frame: usize,
        flip_x: bool,
        x: i32,
        y: i32,
        layer: DecalLayer,
    ) -> Self {
        let (w, h) = sheet.frame_size();
        Self {
            sheet,
            frame,
            src_x: 0,
            flip_x,
            x,
            y,
            w,
            h,
            layer,
        }
    }
}

/// The tiles of a cell `width` wide that cover `x0..x1`, centred, alternately mirrored,
/// each cropped to `[x0 - margin, x1 + margin]`: `(tile x, mirrored, src_x, shown w)`.
fn tiles(x0: i32, x1: i32, width: i32, margin: i32) -> Vec<(i32, bool, i32, i32)> {
    let span = x1 - x0;
    let n = (span + width - 1) / width;
    let start = (x0 + x1) / 2 - n * width / 2;
    let (lo, hi) = (x0 - margin, x1 + margin);
    (0..n)
        .filter_map(|k| {
            let at = start + k * width;
            let mirrored = k % 2 == 1;
            let (left, right) = (at.max(lo), (at + width).min(hi));
            let shown = right - left;
            if shown <= 0 {
                return None;
            }
            // The displayed columns `left - at .. right - at`; a mirrored cell shows them
            // from the other end of the source.
            let src_x = if mirrored {
                width - (right - at)
            } else {
                left - at
            };
            Some((left, mirrored, src_x, shown))
        })
        .collect()
}

/// The decals of one acid patch: its corrosion face tiles and its wall trail.
fn patch_decals(spec: &AcidPatchSpec, layout_h: i32, deck_top_px: i32, out: &mut Vec<Decal>) {
    let line = layout_h - spec.floor_y;
    for (x, mirrored, src_x, w) in tiles(spec.x0, spec.x1, FACE_W, FACE_MARGIN_PX) {
        out.push(Decal {
            sheet: DecalSheet::CorrFace,
            frame: 0,
            src_x,
            flip_x: mirrored,
            x,
            // Row 0 is the first solid row under the pool's bottom row.
            y: line - FACE_H,
            w,
            h: FACE_H,
            layer: DecalLayer::Floor,
        });
    }
    // HZB-AD17: the trail lies on the backwall, never on the deck's top surface. Its pivot
    // row is the layout row `floor_y - deck_top_px - 1` (the last backwall row above the
    // deck's back edge), so the canvas' bottom-left sits `deck_top_px` above the walking line.
    out.push(Decal::whole(
        DecalSheet::CorrWall,
        0,
        false,
        spec.x0 + WALL_INSET_PX,
        line + deck_top_px,
        DecalLayer::Wall,
    ));
}

/// The decals of one ceiling track: the nest at its start and the scuffs along it.
fn track_decals(track: &Track, out: &mut Vec<Decal>) {
    if track.orientation != Orientation::Ceiling {
        // HZB-AD6: wall nests are deferred; a wall track gets no nest.
        return;
    }
    let under = track.fixed;
    out.push(Decal::whole(
        DecalSheet::RotNest,
        0,
        false,
        track.start - NEST_W / 2,
        under - NEST_H,
        DecalLayer::Nest,
    ));
    let mut k = 0;
    let mut at = track.lo;
    while at <= track.hi {
        if (at - track.start).abs() > TRAIL_NEST_GAP_PX {
            out.push(Decal::whole(
                DecalSheet::RotTrail,
                0,
                k % 2 == 1,
                at - TRAIL_W / 2,
                under - TRAIL_H,
                DecalLayer::Nest,
            ));
        }
        k += 1;
        at += TRAIL_STEP_PX;
    }
}

/// Every static decal of a hazard room: the corrosion of each authored acid patch,
/// the room's const entry stains (`fixture` is the combat fixture's name) and the Rot
/// marks of each ceiling track. Computed from the authored specs and tracks, never
/// from the live state, so a drained patch keeps its stain (HZB-AD8). `layout_h` flips
/// the room's layout rows (y down) into world rows (y up); `deck_top_px` is the deck's
/// visible top surface above the walking line ([`crate::env_kit::EnvKit::deck_top_px`],
/// derived from the kit's deep floor).
pub fn static_decals(
    env: &EnvRuntime,
    layout_h: i32,
    deck_top_px: i32,
    fixture: &str,
) -> Vec<Decal> {
    let mut out = Vec::new();
    let mut first_line = None;
    for patch in env.acid().patches() {
        patch_decals(patch.spec(), layout_h, deck_top_px, &mut out);
        first_line.get_or_insert(layout_h - patch.spec().floor_y);
    }
    if let Some(line) = first_line {
        for &(name, x) in ENTRY_STAINS {
            if name == fixture {
                out.push(Decal::whole(
                    DecalSheet::CorrTop,
                    0,
                    false,
                    x - TOP_W / 2,
                    line,
                    DecalLayer::Floor,
                ));
            }
        }
    }
    for (_, sticker, _) in env.stickers() {
        track_decals(sticker.track(), &mut out);
    }
    out
}

/// How many sprites the static decals of `env` need (the presenter's pool size); a
/// function of the authored data only.
pub fn static_capacity(env: &EnvRuntime, fixture: &str) -> usize {
    static_decals(env, 0, 0, fixture).len()
}

/// The drip frame of a spent sticker `age` ticks after its detonation: `drip_run_*` on
/// ages `0..24` (4 frames of 6 ticks), then `drip_drop_*` looping (3 frames of 20).
/// `wall` picks the `_w` tags.
pub fn drip_frame(age: u64, wall: bool) -> usize {
    let (run, drop) = if wall {
        (DRIP_RUN_W, DRIP_DROP_W)
    } else {
        (DRIP_RUN_C, DRIP_DROP_C)
    };
    if age < DRIP_RUN_TICKS {
        run + (age / DRIP_RUN_TICK) as usize
    } else {
        let loop_age = age - DRIP_RUN_TICKS;
        drop + ((loop_age / DRIP_DROP_TICK) % DRIP_DROP_FRAMES as u64) as usize
    }
}

/// The drip of one spent sticker at `tick`: the ceiling drip hangs from the underside
/// at the dome's column; a wall drip runs down the face from the dome's height (the
/// wall sheet is authored for `WallLeft` and mirrored for `WallRight`).
fn drip_of(s: &RotSticker, tick: u64) -> Option<Decal> {
    if s.state() != StickerState::Spent {
        return None;
    }
    let age = tick.checked_sub(s.detonate_tick())?;
    let t = rot_sticker::active();
    let (cx, cy) = s.dome(t);
    let track = s.track();
    let (x, flip_x, wall) = match track.orientation {
        Orientation::Ceiling => (cx.round() as i32 - DRIP_W / 2, false, false),
        Orientation::WallLeft => (track.fixed, false, true),
        Orientation::WallRight => (track.fixed - DRIP_W, true, true),
    };
    let top = match track.orientation {
        Orientation::Ceiling => track.fixed,
        _ => cy.round() as i32,
    };
    Some(Decal::whole(
        DecalSheet::RotDrip,
        drip_frame(age, wall),
        flip_x,
        x,
        top - DRIP_H,
        DecalLayer::Drip,
    ))
}

/// The drip of every spent sticker of the room at `tick`, in prop order. A sticker that
/// is not spent has none; a spent one has one from its detonation tick on.
pub fn drips(combat: &CombatState, tick: u64) -> Vec<Decal> {
    let Some(env) = combat.env() else {
        return Vec::new();
    };
    env.stickers()
        .filter_map(|(_, s, _)| drip_of(s, tick))
        .collect()
}

#[cfg(test)]
mod tests;
