//! F01 danger FX: the art of four Floor 1 danger visuals, the pure half
//! (`docs/02-art/floor01-danger-fx-spec.md`). A function of the combat state and
//! the tick through **shared references only**, it returns what a presenter draws:
//!
//! - the **acid pool** of every live patch, cropped to the patch's *current*
//!   bounds (a draining pool shrinks with its volume, so the drawn pool is the
//!   simulation's rectangle at every drain step);
//! - the **blast reach ring** around a warning Rot Sticker (HAZ-Q38, `t..t+23`)
//!   and around Carl's landed, burning stick (its profile's outer radius), with
//!   the hot variant for the last [`RING_HOT_TICKS`];
//! - the **blast wave** of every detonated charge whose outer radius has a drawn
//!   wave (48 px: the Rot Sticker and the Boiler drum; 64 px: the thrown
//!   dynamite), under the VFX-01 core burst, its reach frame ending exactly on
//!   the radius;
//! - the **room 5 entrance gate** (the Juicer's arena): the closed grate over the
//!   gate's collider while the simulation's gate is closed, the raised grate's
//!   rail and teeth under the lintel otherwise.
//!
//! F01-SIM-TICKS (spec `docs/02-art/f01-sim-ticks-spec.md`) adds the gate's
//! **drop**, played once from the simulation's close tick
//! ([`Boss::gate_closed_tick`]), and the **slam dust** ([`Sheet::SlamDust`]) at the
//! gate's impact and at both sides of the Boiler weight's remnant from its landing
//! tick ([`crate::env_prop::FallingMass::landed_tick`]). Both ticks are unhashed
//! copies of the scene tick at a hashed edge, so these one-shots read the
//! simulation exactly as the wave reads `detonate_tick`.
//!
//! AREA-HAZARDS-01B adds the acid **entry splash** sheet ([`Sheet::Splash`], its
//! sprites are made by the stateful cue in [`crate::acid_splash`], not by [`draws`])
//! and recolours the acid to the teal ramp (HZB-AD1) and backs the rings' outer row
//! with the dark tape (HZB-AD2); both are art-only changes (spec
//! `docs/02-art/area-hazards-01b-spec.md`).
//!
//! **Presentation only.** Nothing here mutates a state, draws a random number or
//! is hashed, replayed or read back by the simulation: a run that calls every
//! function here each tick has the same state hash as one that never does (a
//! test). The simulation keeps no "ticks since" for these visuals, so the loops
//! run on the world tick (`tick / ticks_per_frame`) and the one-shot wave reads
//! the charge's own `detonate_tick`, exactly as the VFX-01 burst does.
//!
//! The art matches the simulation or is not drawn: a ring or wave is drawn only
//! for a radius that has its own canvas ([`ring_sheet`], [`wave_sheet`]), the gate
//! only when its collider is the 28 px the canvas covers; anything else keeps its
//! code-drawn stand-in (the ring dots, the gate slab) or, for the wave, the core
//! burst alone. The art is a prototype (`prototype_fixture_unapproved`).
//!
//! Coordinates are world px, y up, like the simulation; every [`FxDraw`] gives
//! the bottom-left corner of what it covers.

use crate::blast::BlastCharge;
use crate::boss::Boss;
use crate::combat_scene::{CombatState, EnvRuntime};
use crate::rot_sticker::{self, StickerState};
use crate::slime::PoolState;

/// The atlases of the danger FX: one per canvas, a single row of equal cells
/// exported by `scripts/export_assets.sh` from `assets/manifests/f01-dfx-*.json`
/// (source grids `assets/source/props/floor01/dfx_*.grid.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sheet {
    Acid,
    RingR48,
    RingR64,
    WaveR48,
    WaveR64,
    Gate,
    /// AREA-HAZARDS-01B: the acid entry splash (5 frames, played once).
    Splash,
    /// F01-SIM-TICKS: the slam dust of a heavy mass hitting the floor (5 frames,
    /// played once).
    SlamDust,
}

/// One animation tag of a sheet: its first frame, frame count and ticks per
/// frame (60 Hz); `ticks` is `0` for a held frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tag {
    pub name: &'static str,
    pub first: usize,
    pub frames: usize,
    pub ticks: u64,
}

impl Tag {
    /// The frame of this tag at `tick`: a loop over its frames, or the held frame.
    pub fn frame_at(self, tick: u64) -> usize {
        if self.frames <= 1 || self.ticks == 0 {
            self.first
        } else {
            self.first + ((tick / self.ticks) % self.frames as u64) as usize
        }
    }
}

const fn tag(name: &'static str, first: usize, frames: usize, ticks: u64) -> Tag {
    Tag {
        name,
        first,
        frames,
        ticks,
    }
}

pub const ACID_IDLE: Tag = tag("acid_idle", 0, 4, 8);
pub const ACID_DRAIN: Tag = tag("acid_drain", 4, 2, 6);
pub const RING_MARCH: Tag = tag("ring_march", 0, 2, 4);
pub const RING_HOT: Tag = tag("ring_hot", 2, 2, 2);
/// The blast wave plays once: ticks per frame (the manifest's `ticks`).
pub const WAVE_TICKS: [u64; 6] = [2, 2, 2, 6, 5, 5];
/// The wave frame whose bright edge ends exactly on the reach.
pub const WAVE_REACH_FRAME: usize = 3;
pub const BLAST_WAVE: Tag = tag("blast_wave", 0, WAVE_TICKS.len(), 0);
pub const GATE_CLOSED: Tag = tag("gate_closed", 0, 1, 0);
pub const GATE_RAISED: Tag = tag("gate_raised", 1, 1, 0);
/// F01-SIM-TICKS: the gate drop, played once from the close tick, 2 ticks a frame
/// (the grate's teeth `200 - 2 t^2` px above the floor at t = 2, 4, 6, 8).
pub const GATE_DROP: Tag = tag("gate_drop", 2, 4, 2);
/// The age (ticks after the close tick) of the gate's impact: the closed frame and
/// the slam dust start here.
pub const GATE_IMPACT_AGE: u64 = GATE_DROP.frames as u64 * GATE_DROP.ticks;
/// The slam dust plays once: ticks per frame (the manifest's `ticks`), 20 in all.
pub const DUST_TICKS: [u64; 5] = [2, 3, 4, 5, 6];
/// The slam dust frame of the key (`roll`).
pub const DUST_KEY_FRAME: usize = 1;
pub const SLAM_DUST: Tag = tag("slam_dust", 0, DUST_TICKS.len(), 0);
/// The slam dust canvas: 44 x 20, its pivot column 4 on the impact edge (columns
/// 0..3 tuck under the object), its bottom row on the walking line; it rolls toward +x.
pub const DUST_W: i32 = 44;
pub const DUST_H: i32 = 20;
pub const DUST_PIVOT_X: i32 = 4;
/// The acid entry splash plays once: ticks per frame (the manifest's `ticks`), 16 in all.
pub const SPLASH_TICKS: [u64; 5] = [2, 3, 3, 4, 4];
/// The splash frame of the key (`crown`).
pub const SPLASH_KEY_FRAME: usize = 2;
pub const SPLASH: Tag = tag("splash", 0, SPLASH_TICKS.len(), 0);
/// The splash canvas: 32 x 24, pivot (16, 23) on the patch's walking line.
pub const SPLASH_W: i32 = 32;
pub const SPLASH_H: i32 = 24;

/// The ring turns into one solid hot band for this many ticks before the blast.
pub const RING_HOT_TICKS: u64 = 6;
/// The acid canvas: 80 x 10, the 6 px volume in rows 4..9 (fumes above it).
pub const ACID_W: i32 = 80;
pub const ACID_H: i32 = 10;
/// The volume height the acid canvas is drawn for (HAZ-Q18 `hazq18_volume_height_px`).
pub const ACID_VOLUME_H: i32 = 6;
/// The room 5 gate canvas: the closed gate's 28 px collider from the walking line
/// up to the arena ceiling (224 px), the rectangle the stand-in slab covered.
pub const GATE_W: i32 = 28;
pub const GATE_H: i32 = 224;
/// The raised gate's rail and teeth hang this far below the top of the doorway.
pub const GATE_RAISED_ROWS: i32 = 24;

impl Sheet {
    pub const ALL: [Self; 8] = [
        Self::Acid,
        Self::RingR48,
        Self::RingR64,
        Self::WaveR48,
        Self::WaveR64,
        Self::Gate,
        Self::Splash,
        Self::SlamDust,
    ];

    /// The manifest under `assets/manifests/` (without `.json`).
    pub const fn manifest(self) -> &'static str {
        match self {
            Self::Acid => "f01-dfx-acid",
            Self::RingR48 => "f01-dfx-ring-r48",
            Self::RingR64 => "f01-dfx-ring-r64",
            Self::WaveR48 => "f01-dfx-wave-r48",
            Self::WaveR64 => "f01-dfx-wave-r64",
            Self::Gate => "f01-dfx-gate",
            Self::Splash => "f01-dfx-acid-splash",
            Self::SlamDust => "f01-dfx-slam-dust",
        }
    }

    /// The exported atlas, relative to the repository root.
    pub const fn atlas(self) -> &'static str {
        match self {
            Self::Acid => "generated/art/f01-dfx-acid.png",
            Self::RingR48 => "generated/art/f01-dfx-ring-r48.png",
            Self::RingR64 => "generated/art/f01-dfx-ring-r64.png",
            Self::WaveR48 => "generated/art/f01-dfx-wave-r48.png",
            Self::WaveR64 => "generated/art/f01-dfx-wave-r64.png",
            Self::Gate => "generated/art/f01-dfx-gate.png",
            Self::Splash => "generated/art/f01-dfx-acid-splash.png",
            Self::SlamDust => "generated/art/f01-dfx-slam-dust.png",
        }
    }

    /// The reach radius a ring or wave canvas is drawn for (`None` for the others).
    pub const fn radius(self) -> Option<i32> {
        match self {
            Self::RingR48 | Self::WaveR48 => Some(48),
            Self::RingR64 | Self::WaveR64 => Some(64),
            Self::Acid | Self::Gate | Self::Splash | Self::SlamDust => None,
        }
    }

    /// One cell of the atlas, in px. A ring or wave canvas is the radius plus a
    /// 3 px margin on each side, odd, so its centre is one pixel.
    pub const fn frame_size(self) -> (i32, i32) {
        match self.radius() {
            Some(r) => (2 * (r + 3) + 1, 2 * (r + 3) + 1),
            None => match self {
                Self::Gate => (GATE_W, GATE_H),
                Self::Splash => (SPLASH_W, SPLASH_H),
                Self::SlamDust => (DUST_W, DUST_H),
                _ => (ACID_W, ACID_H),
            },
        }
    }

    /// The tags of the sheet, in frame order.
    pub const fn tags(self) -> &'static [Tag] {
        match self {
            Self::Acid => &[ACID_IDLE, ACID_DRAIN],
            Self::RingR48 | Self::RingR64 => &[RING_MARCH, RING_HOT],
            Self::WaveR48 | Self::WaveR64 => &[BLAST_WAVE],
            Self::Gate => &[GATE_CLOSED, GATE_RAISED, GATE_DROP],
            Self::Splash => &[SPLASH],
            Self::SlamDust => &[SLAM_DUST],
        }
    }

    /// The number of frames of the atlas.
    pub const fn frames(self) -> usize {
        let tags = self.tags();
        let last = tags[tags.len() - 1];
        last.first + last.frames
    }
}

/// The ring canvas drawn for a reach of `radius` px, if one exists (exact match:
/// the art never stretches; another radius keeps the code-drawn ring dots).
pub fn ring_sheet(radius: f32) -> Option<Sheet> {
    [Sheet::RingR48, Sheet::RingR64]
        .into_iter()
        .find(|s| s.radius().is_some_and(|r| r as f32 == radius))
}

/// The blast wave canvas drawn for a reach of `radius` px, if one exists.
pub fn wave_sheet(radius: f32) -> Option<Sheet> {
    [Sheet::WaveR48, Sheet::WaveR64]
        .into_iter()
        .find(|s| s.radius().is_some_and(|r| r as f32 == radius))
}

/// Whether the acid art draws with the active tuning: only for the volume height
/// its canvas is drawn for (another height keeps the code-drawn strip).
pub fn acid_drawn() -> bool {
    i32::from(crate::slime::active().hazq18_volume_height_px) == ACID_VOLUME_H
}

/// The ring a warning Rot Sticker shows with the active tuning (its outer radius).
pub fn rot_ring_sheet() -> Option<Sheet> {
    ring_sheet(f32::from(rot_sticker::active().hazq30_outer_radius_px))
}

/// The draw order. The presenter maps each layer to a z.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    /// The raised gate under the lintel: behind the actors.
    Back,
    /// The acid on the floor: with the hazard pieces, under the slime and the actors.
    Floor,
    /// F01-SIM-TICKS: the slam dust, over the props (the remnant, the drum) and the
    /// acid, under every actor.
    Dust,
    /// The acid entry splash: above every actor sprite, below the blast layer and the
    /// rings (AREA-HAZARDS-01B, spec section 4.2).
    Splash,
    /// The blast wave: the blast layer, under the VFX-01 burst and every actor.
    Blast,
    /// The closed gate: the boss encounter's z (the stand-in slab's).
    Gate,
    /// The reach rings: warnings, above everything (never hidden).
    Tell,
}

/// One sprite: `frame` of `sheet`, the cell's columns `src_x..src_x + w` and rows
/// `src_y..src_y + h` (the whole cell unless cropped), mirrored about its own
/// vertical axis when `flip_x` (the crop is taken from the unmirrored cell), its
/// bottom-left at world `(x, y)`, `w` x `h` px.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FxDraw {
    pub sheet: Sheet,
    pub frame: usize,
    pub src_x: i32,
    pub src_y: i32,
    pub flip_x: bool,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub layer: Layer,
}

/// The most draws [`draws`] returns in one tick: the presenter's pool size.
pub const MAX_DRAWS: usize = 40;

/// The whole cell of a ring or wave sheet centred on the world pixel `(cx, cy)`
/// (rounded): the canvas' centre pixel covers `[cx, cx + 1) x [cy, cy + 1)`, the
/// pixel the code-drawn ring dots are centred on.
pub fn centred(sheet: Sheet, frame: usize, centre: (f32, f32), layer: Layer) -> FxDraw {
    let (w, h) = sheet.frame_size();
    let (c, r) = ((w - 1) / 2, (h - 1) / 2);
    FxDraw {
        sheet,
        frame,
        src_x: 0,
        src_y: 0,
        flip_x: false,
        x: centre.0.round() as i32 - c,
        y: centre.1.round() as i32 - r,
        w,
        h,
        layer,
    }
}

/// The ring frame `remaining` ticks before a blast: the marching dashes, then the
/// hot band for the last [`RING_HOT_TICKS`].
pub fn ring_frame(remaining: u64, tick: u64) -> usize {
    if remaining <= RING_HOT_TICKS {
        RING_HOT.frame_at(tick)
    } else {
        RING_MARCH.frame_at(tick)
    }
}

/// The wave frame of a blast that happened `age` ticks ago (0 on the detonation
/// tick); `None` once the wave is over.
pub fn wave_frame(age: u64) -> Option<usize> {
    let mut left = age;
    for (frame, ticks) in WAVE_TICKS.into_iter().enumerate() {
        if left < ticks {
            return Some(frame);
        }
        left -= ticks;
    }
    None
}

/// The splash frame `age` ticks after the entry (0 on the event tick); `None` once
/// its 16 ticks are over.
pub fn splash_frame(age: u64) -> Option<usize> {
    let mut left = age;
    for (frame, ticks) in SPLASH_TICKS.into_iter().enumerate() {
        if left < ticks {
            return Some(frame);
        }
        left -= ticks;
    }
    None
}

/// The slam dust frame `age` ticks after the impact (0 on the impact tick); `None`
/// once its 20 ticks are over.
pub fn dust_frame(age: u64) -> Option<usize> {
    let mut left = age;
    for (frame, ticks) in DUST_TICKS.into_iter().enumerate() {
        if left < ticks {
            return Some(frame);
        }
        left -= ticks;
    }
    None
}

/// The gate frame `age` ticks after the close tick: a drop frame for the first
/// [`GATE_IMPACT_AGE`] ticks, then the closed grate.
pub fn gate_closed_frame(age: u64) -> usize {
    if age < GATE_IMPACT_AGE {
        GATE_DROP.first + (age / GATE_DROP.ticks) as usize
    } else {
        GATE_CLOSED.first
    }
}

/// One slam dust plume `age` ticks after an impact whose edge is the world column
/// `edge` on the walking line `floor` (world y up), rolling toward `toward` (+1 or
/// -1, mirrored for -1 so the pivot column stays on `edge`).
pub fn slam_dust(edge: i32, floor: i32, toward: i8, age: u64) -> Option<FxDraw> {
    let frame = dust_frame(age)?;
    let flip_x = toward < 0;
    let pivot = if flip_x {
        DUST_W - 1 - DUST_PIVOT_X
    } else {
        DUST_PIVOT_X
    };
    Some(FxDraw {
        sheet: Sheet::SlamDust,
        frame,
        src_x: 0,
        src_y: 0,
        flip_x,
        x: edge - pivot,
        y: floor,
        w: DUST_W,
        h: DUST_H,
        layer: Layer::Dust,
    })
}

/// SMALL-02 (ST-OQ1, SMALL02-AD2): a landing plume stops at the nearest other prop that stands
/// in its way. `edge` is the plume's pivot column (the remnant's edge), `toward` its direction,
/// `floor_row` the remnant's bottom in layout rows (y down). Returns the screen columns of the
/// plume cell that stay visible as `(first, last_exclusive)`, or `None` when the prop stands
/// flush and nothing of the plume is left. A prop is in the way when it overlaps the plume's
/// 20 row band and begins at or past the edge, within the cell's reach.
fn plume_limit(
    env: &EnvRuntime,
    skip: usize,
    edge: i32,
    toward: i8,
    floor_row: i32,
) -> Option<(i32, i32)> {
    let reach = DUST_W - 1 - DUST_PIVOT_X;
    let (mut first, mut last) = if toward > 0 {
        (edge - DUST_PIVOT_X, edge + reach + 1)
    } else {
        (edge - reach, edge + DUST_PIVOT_X + 1)
    };
    for j in 0..env.props() {
        if j == skip {
            continue;
        }
        let Some(t) = env.prop_target(j) else {
            continue;
        };
        if t.bottom() <= floor_row - DUST_H || t.y >= floor_row {
            continue;
        }
        if toward > 0 && t.x >= edge && t.x < last {
            last = t.x;
        } else if toward < 0 && t.right() <= edge + 1 && t.right() > first {
            first = t.right();
        }
    }
    let visible_from = if toward > 0 { edge } else { first };
    let visible_to = if toward > 0 { last } else { edge + 1 };
    (visible_to > visible_from).then_some((first, last))
}

/// The landing dust of every falling mass that landed (the Boiler weight): one plume
/// from each side of its remnant, from the landing tick on. `layout_h` flips the
/// room's layout rows (y down) into world rows (y up). A plume is cropped at a prop
/// that stands in its way and dropped when the prop stands flush against the remnant
/// (the Boiler's drum on the right, ST-OQ1).
pub fn landing_dust(env: &EnvRuntime, layout_h: i32, tick: u64) -> Vec<FxDraw> {
    let mut out = Vec::new();
    for i in 0..env.props() {
        let Some(mass) = env.mass(i) else {
            continue;
        };
        let (Some(landed), Some(remnant)) = (mass.landed_tick(), mass.remnant()) else {
            continue;
        };
        let Some(age) = tick.checked_sub(landed) else {
            continue;
        };
        let floor = layout_h - remnant.bottom();
        for (edge, toward) in [(remnant.x + remnant.w, 1i8), (remnant.x - 1, -1i8)] {
            let Some(mut draw) = slam_dust(edge, floor, toward, age) else {
                continue;
            };
            let Some((first, last)) = plume_limit(env, i, edge, toward, remnant.bottom()) else {
                continue;
            };
            // Crop to the visible screen columns `first..last` of the cell; a mirrored
            // sprite takes its crop from the unmirrored cell.
            let (a, b) = (first - draw.x, last - draw.x);
            if (a, b) != (0, DUST_W) {
                draw.src_x = if draw.flip_x { DUST_W - b } else { a };
                draw.w = b - a;
                draw.x = first;
            }
            out.push(draw);
        }
    }
    out
}

/// The acid sprites of one pool whose current layout rectangle is `x..x + w`
/// with its bottom on world row `bottom`: the left part of the canvas from its
/// left cap, the right part from its right cap (and, past 80 px, middle columns),
/// so both meniscus ends sit exactly on the volume's ends.
fn acid_strip(x: i32, w: i32, bottom: i32, frame: usize, out: &mut Vec<FxDraw>) {
    if w <= 0 {
        return;
    }
    let half = ACID_W / 2;
    let left = ((w + 1) / 2).min(half);
    let right = (w / 2).min(half);
    let mut push = |src_x: i32, at: i32, cols: i32| {
        out.push(FxDraw {
            sheet: Sheet::Acid,
            frame,
            src_x,
            src_y: 0,
            flip_x: false,
            x: at,
            y: bottom,
            w: cols,
            h: ACID_H,
            layer: Layer::Floor,
        });
    };
    push(0, x, left);
    let mut at = x + left;
    let mut middle = w - left - right;
    while middle > 0 {
        let cols = middle.min(half);
        push(half / 2, at, cols);
        at += cols;
        middle -= cols;
    }
    push(ACID_W - right, at, right);
}

/// Every live acid pool of the room at its current bounds. `layout_h` flips the
/// room's layout rows (y down) into world rows (y up).
pub fn acid(env: &EnvRuntime, layout_h: i32, tick: u64) -> Vec<FxDraw> {
    let mut out = Vec::new();
    for patch in env.acid().patches() {
        let Some(rect) = patch.rect().filter(|r| r.h == ACID_VOLUME_H) else {
            continue;
        };
        let frame = match patch.pool_state() {
            PoolState::Full => ACID_IDLE.frame_at(tick + u64::from(patch.id()) * 5),
            PoolState::Draining => ACID_DRAIN.frame_at(tick),
            PoolState::Removed => continue,
        };
        acid_strip(rect.x, rect.w, layout_h - rect.bottom(), frame, &mut out);
    }
    out
}

/// The reach ring of every warning Rot Sticker, on `t..t+23` (HAZ-Q38: every tick
/// with at least one left, not on the detonation tick), when its radius has a ring.
pub fn rot_rings(env: &EnvRuntime, tick: u64) -> Vec<FxDraw> {
    let t = rot_sticker::active();
    let Some(sheet) = ring_sheet(f32::from(t.hazq30_outer_radius_px)) else {
        return Vec::new();
    };
    if !t.on(t.hazq38_ring) {
        return Vec::new();
    }
    env.stickers()
        .filter(|(_, s, _)| s.state() == StickerState::Warning)
        .filter_map(|(_, s, _)| {
            let remaining = s.remaining(tick).filter(|r| *r >= 1)?;
            Some(centred(
                sheet,
                ring_frame(remaining, tick),
                s.dome(t),
                Layer::Tell,
            ))
        })
        .collect()
}

/// The reach ring of Carl's stick once it has landed and burns: its profile's
/// outer radius around the blast centre, until the detonation tick.
pub fn dynamite_ring(state: &CombatState, tick: u64) -> Option<FxDraw> {
    let stick = state.thrower().stick().filter(|s| s.landed)?;
    let charge = state
        .charges
        .get(state.dynamite_slot())
        .filter(|c| !c.detonated)?;
    let sheet = ring_sheet(charge.profile.outer_radius)?;
    let remaining = stick.detonate_tick.checked_sub(tick).filter(|r| *r >= 1)?;
    Some(centred(
        sheet,
        ring_frame(remaining, tick),
        charge.centre(),
        Layer::Tell,
    ))
}

/// The blast wave of one charge at `tick`, if it detonated, its wave is still
/// playing and its outer radius has a wave canvas.
pub fn blast_wave(charge: &BlastCharge, tick: u64) -> Option<FxDraw> {
    if !charge.detonated {
        return None;
    }
    let sheet = wave_sheet(charge.profile.outer_radius)?;
    let frame = wave_frame(tick.checked_sub(charge.detonate_tick)?)?;
    Some(centred(sheet, frame, charge.centre(), Layer::Blast))
}

/// Whether the gate art draws for this encounter: only when its collider is the
/// 28 px the canvas covers (otherwise the stand-in slab draws).
pub fn gate_drawn(boss: &Boss) -> bool {
    boss.data().gate.solid_x == GATE_W as f32
}

/// The room 5 gate at `tick`: closed over `{0, floor, 28, 224}` (the gate collider
/// up to the arena ceiling) while the simulation's gate is closed, after the drop
/// frames of its first [`GATE_IMPACT_AGE`] ticks from the close tick; raised (rail
/// and teeth under the lintel) before and after. `None` when the collider is not
/// the 28 px the canvas covers.
pub fn gate(boss: &Boss, tick: u64) -> Option<FxDraw> {
    if !gate_drawn(boss) {
        return None;
    }
    let floor = boss.floor_y().round() as i32;
    let (frame, layer) = if boss.gate_closed() {
        let age = boss
            .gate_closed_tick()
            .and_then(|t| tick.checked_sub(t))
            .unwrap_or(GATE_IMPACT_AGE);
        (gate_closed_frame(age), Layer::Gate)
    } else {
        (GATE_RAISED.first, Layer::Back)
    };
    Some(FxDraw {
        sheet: Sheet::Gate,
        frame,
        src_x: 0,
        src_y: 0,
        flip_x: false,
        x: 0,
        y: floor,
        w: GATE_W,
        h: GATE_H,
        layer,
    })
}

/// The slam dust of the gate's impact: one plume from the gate's arena face into
/// the arena, [`GATE_IMPACT_AGE`] ticks after the close tick.
pub fn gate_dust(boss: &Boss, tick: u64) -> Option<FxDraw> {
    if !gate_drawn(boss) || !boss.gate_closed() {
        return None;
    }
    let age = tick
        .checked_sub(boss.gate_closed_tick()?)?
        .checked_sub(GATE_IMPACT_AGE)?;
    slam_dust(GATE_W, boss.floor_y().round() as i32, 1, age)
}

/// Everything this module draws for the combat state at `tick`, back to front by
/// layer, at most [`MAX_DRAWS`].
pub fn draws(state: &CombatState, layout_h: i32, tick: u64) -> Vec<FxDraw> {
    let mut out = Vec::new();
    if let Some(env) = state.env() {
        out.extend(acid(env, layout_h, tick));
        out.extend(rot_rings(env, tick));
        out.extend(landing_dust(env, layout_h, tick));
    }
    out.extend(state.charges.iter().filter_map(|c| blast_wave(c, tick)));
    out.extend(dynamite_ring(state, tick));
    if let Some(boss) = state.boss.as_ref() {
        out.extend(gate(boss, tick));
        out.extend(gate_dust(boss, tick));
    }
    out.sort_by_key(|d| d.layer);
    out.truncate(MAX_DRAWS);
    out
}

#[cfg(test)]
mod tests;
