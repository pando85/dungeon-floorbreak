//! BOIL-E (K10) and AREA-GOBLINWORKSHOP-01: the art of the Goblin Boiler's props,
//! the pure half. A function of the encounter's authored props, the **read-only**
//! env runtime and the tick, it returns what a presenter draws:
//!
//! - [`sprites`]: frames of the Goblin Workshop prop kit (`gw_*` atlases, one per
//!   canvas, see [`Sheet`]) for the boiler landmark (P4), the valve in its four
//!   states, the brace, the hanging weight (swaying, falling), the rig and its
//!   chains, the landed remnant, the drum (armed, telegraph, scorch) and the
//!   tells (steam sputter and plume, weight shadow, drum blast ring);
//! - [`pieces`]: the code-drawn rectangles. They are the **footprints** of the
//!   tells (steam warning area and edge, the drum's blast area outline) and the
//!   **placeholder** of every prop whose atlas is missing
//!   ([`Piece::sheet`] names the sheet a piece stands in for, so a presenter draws
//!   the piece only while that sheet is absent).
//!
//! **Presentation only.** Nothing here mutates the runtime (every input is a
//! shared reference), draws a random number or is hashed, replayed or read back by
//! the simulation: a run with the art drawn and one without have the same state
//! hash (a test). Every output is a function of the machine states and the tick, so
//! a render that skips frames or a replay seek draws the same picture for the same
//! tick. The simulation keeps no "ticks since the event" for a prop, so the loops
//! below run on the world tick (`tick / ticks_per_frame`), like the stand-in's
//! flash and sway did; a tag that is not a loop (the rig's run) is chosen by state.
//! The one exception is the scorch's ember fade, which plays once from the drum
//! charge's own `detonate_tick` (read from the scene's charges, like the blast
//! wave of `danger_fx_art`).
//! The explosion of the drum is **not** drawn here: its `BlastCharge` already
//! plays the shared blast presentation (`blast_art`).
//!
//! The kit is a prototype (`prototype_fixture_unapproved`; independent review
//! passed in PR 256, owner review pending). Telegraph pieces and sprites sit on [`Layer::Tell`], above every other
//! piece, so no prop or backdrop hides a warning.
//!
//! Coordinates are layout pixels, y down, like the sidecar. A `RING-D` append adds
//! the shutter and the Rot Sticker arms to [`pieces`].

use crate::blast::BlastCharge;
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

// ------------------------------------------------------------ the prop kit ----

/// The Goblin Workshop atlases: one per canvas size, each a single row of equal
/// cells exported by `scripts/export_assets.sh` from `assets/manifests/f01-gw-*.json`
/// (source grids `assets/source/props/floor01/gw_*.grid.json`; spec
/// `docs/02-art/goblin-workshop-prop-kit.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sheet {
    Boiler,
    Valve,
    Brace,
    Weight,
    Remnant,
    Drum,
    Scorch,
    Rig,
    Chain,
    Sputter,
    Plume,
    Shadow,
    Ring,
}

/// One animation tag of a sheet: its first frame, frame count and ticks per frame
/// (60 Hz). `ticks` is `0` for a single held frame. The test
/// `the_tag_table_matches_the_manifests` compares every row with the manifests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tag {
    pub name: &'static str,
    pub first: usize,
    pub frames: usize,
    pub ticks: u64,
}

const fn tag(name: &'static str, first: usize, frames: usize, ticks: u64) -> Tag {
    Tag {
        name,
        first,
        frames,
        ticks,
    }
}

impl Tag {
    /// The frame of this tag at `tick`: a loop over its frames, or the one held frame.
    pub fn frame_at(self, tick: u64) -> usize {
        if self.frames <= 1 || self.ticks == 0 {
            self.first
        } else {
            self.first + ((tick / self.ticks) % self.frames as u64) as usize
        }
    }

    /// The last frame of the tag (where a state-chosen "settled" pose rests).
    pub const fn last(self) -> usize {
        self.first + self.frames - 1
    }

    /// The frame of a tag played once, `age` ticks after its event: each frame for
    /// `ticks`, then the last frame holds.
    pub fn frame_once(self, age: u64) -> usize {
        if self.frames <= 1 || self.ticks == 0 {
            self.first
        } else {
            self.first + ((age / self.ticks) as usize).min(self.frames - 1)
        }
    }
}

pub const BOILER_IDLE: Tag = tag("boiler_idle", 0, 4, 8);
pub const BOILER_STRAIN: Tag = tag("boiler_strain", 4, 2, FLASH_TICKS);
pub const BOILER_DAMAGED: Tag = tag("boiler_damaged", 6, 2, 10);
pub const VALVE_INTACT: Tag = tag("valve_intact", 0, 1, 0);
pub const VALVE_WARNING: Tag = tag("valve_warning", 1, 2, FLASH_TICKS);
pub const VALVE_VENTING: Tag = tag("valve_venting", 3, 3, 4);
pub const VALVE_SPENT: Tag = tag("valve_spent", 6, 3, 10);
pub const BRACE_SUPPORTED: Tag = tag("brace_supported", 0, 1, 0);
pub const BRACE_WEAKENED: Tag = tag("brace_weakened", 1, 2, 6);
pub const BRACE_STUMP: Tag = tag("brace_stump", 3, 1, 0);
pub const WEIGHT_HANG: Tag = tag("weight_hang", 0, 1, 0);
pub const WEIGHT_FALL: Tag = tag("weight_fall", 1, 1, 0);
pub const REMNANT: Tag = tag("remnant", 0, 1, 0);
pub const DRUM_ARMED: Tag = tag("drum_armed", 0, 1, 0);
pub const DRUM_TELEGRAPH: Tag = tag("drum_telegraph", 1, 2, FLASH_TICKS);
pub const SCORCH: Tag = tag("scorch", 0, 3, 10);
pub const RIG: Tag = tag("rig", 0, 1, 0);
pub const RIG_RUN: Tag = tag("rig_run", 1, 2, 6);
pub const CHAIN: Tag = tag("chain", 0, 1, 0);
pub const STEAM_SPUTTER: Tag = tag("steam_sputter", 0, 3, FLASH_TICKS);
pub const STEAM_PLUME: Tag = tag("steam_plume", 0, 4, FLASH_TICKS);
pub const FALL_SHADOW_TELL: Tag = tag("fall_shadow_tell", 0, 2, FLASH_TICKS);
pub const FALL_SHADOW: Tag = tag("fall_shadow", 2, 1, 0);
pub const DRUM_BLAST_TELL: Tag = tag("drum_blast_tell", 0, 2, FLASH_TICKS);

impl Sheet {
    pub const ALL: [Self; 13] = [
        Self::Boiler,
        Self::Valve,
        Self::Brace,
        Self::Weight,
        Self::Remnant,
        Self::Drum,
        Self::Scorch,
        Self::Rig,
        Self::Chain,
        Self::Sputter,
        Self::Plume,
        Self::Shadow,
        Self::Ring,
    ];

    /// The manifest under `assets/manifests/` (without `.json`); the atlas sits
    /// beside it as `generated/art/<manifest>.png`.
    pub const fn manifest(self) -> &'static str {
        match self {
            Self::Boiler => "f01-gw-boiler",
            Self::Valve => "f01-gw-valve",
            Self::Brace => "f01-gw-brace",
            Self::Weight => "f01-gw-weight",
            Self::Remnant => "f01-gw-remnant",
            Self::Drum => "f01-gw-drum",
            Self::Scorch => "f01-gw-scorch",
            Self::Rig => "f01-gw-rig",
            Self::Chain => "f01-gw-chain",
            Self::Sputter => "f01-gw-tell-sputter",
            Self::Plume => "f01-gw-tell-plume",
            Self::Shadow => "f01-gw-tell-shadow",
            Self::Ring => "f01-gw-tell-ring",
        }
    }

    /// The exported atlas, relative to the repository root.
    pub const fn atlas(self) -> &'static str {
        match self {
            Self::Boiler => "generated/art/f01-gw-boiler.png",
            Self::Valve => "generated/art/f01-gw-valve.png",
            Self::Brace => "generated/art/f01-gw-brace.png",
            Self::Weight => "generated/art/f01-gw-weight.png",
            Self::Remnant => "generated/art/f01-gw-remnant.png",
            Self::Drum => "generated/art/f01-gw-drum.png",
            Self::Scorch => "generated/art/f01-gw-scorch.png",
            Self::Rig => "generated/art/f01-gw-rig.png",
            Self::Chain => "generated/art/f01-gw-chain.png",
            Self::Sputter => "generated/art/f01-gw-tell-sputter.png",
            Self::Plume => "generated/art/f01-gw-tell-plume.png",
            Self::Shadow => "generated/art/f01-gw-tell-shadow.png",
            Self::Ring => "generated/art/f01-gw-tell-ring.png",
        }
    }

    /// One cell of the atlas, in px.
    pub const fn frame_size(self) -> (i32, i32) {
        match self {
            Self::Boiler => (192, 208),
            Self::Valve => (32, 48),
            Self::Brace => (32, 40),
            Self::Weight => (64, 48),
            Self::Remnant => (64, 32),
            Self::Drum => (36, 36),
            Self::Scorch => (48, 8),
            Self::Rig => (112, 24),
            Self::Chain => (4, 8),
            Self::Sputter => (16, 16),
            Self::Plume => (80, 96),
            Self::Shadow => (64, 8),
            Self::Ring => (96, 8),
        }
    }

    /// The tags of the sheet, in frame order.
    pub const fn tags(self) -> &'static [Tag] {
        match self {
            Self::Boiler => &[BOILER_IDLE, BOILER_STRAIN, BOILER_DAMAGED],
            Self::Valve => &[VALVE_INTACT, VALVE_WARNING, VALVE_VENTING, VALVE_SPENT],
            Self::Brace => &[BRACE_SUPPORTED, BRACE_WEAKENED, BRACE_STUMP],
            Self::Weight => &[WEIGHT_HANG, WEIGHT_FALL],
            Self::Remnant => &[REMNANT],
            Self::Drum => &[DRUM_ARMED, DRUM_TELEGRAPH],
            Self::Scorch => &[SCORCH],
            Self::Rig => &[RIG, RIG_RUN],
            Self::Chain => &[CHAIN],
            Self::Sputter => &[STEAM_SPUTTER],
            Self::Plume => &[STEAM_PLUME],
            Self::Shadow => &[FALL_SHADOW_TELL, FALL_SHADOW],
            Self::Ring => &[DRUM_BLAST_TELL],
        }
    }

    /// The number of frames of the atlas.
    pub const fn frames(self) -> usize {
        let tags = self.tags();
        let last = tags[tags.len() - 1];
        last.first + last.frames
    }
}

/// One sprite of the kit: `frame` of `sheet`, its cell placed at `rect` (layout
/// px, the cell's top-left and size), drawn on `layer` at `alpha` (255 opaque).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteDraw {
    pub sheet: Sheet,
    pub frame: usize,
    pub rect: IRect,
    pub layer: Layer,
    pub alpha: u8,
}

/// The most sprites [`sprites`] can return for the Boiler, with margin (the two
/// chains are tiled from 8 px cells): the presenter keeps a pool of this many.
pub const MAX_SPRITES: usize = 48;

/// The top of the ceiling rig and of the chains below it (spec section 3 and 4).
pub const RIG_TOP_Y: i32 = 352;
pub const CHAIN_TOP_Y: i32 = 376;
/// The chain tiles hang this far in from the brace's and the weight's left edge
/// (brace chain x 758 = 744 + 14, weight chain x 814 = 784 + 30).
pub const BRACE_CHAIN_DX: i32 = 14;
pub const WEIGHT_CHAIN_DX: i32 = 30;
/// The steam sputter leaves the nozzle bore: this far below the lane's top.
pub const SPUTTER_DY: i32 = 28;
/// Alphas of the translucent tells (the stand-in's `Steam` and `Shadow` tones).
pub const STEAM_ALPHA: u8 = 0x90;
/// The weight's shadow sprite is darker than the stand-in tone (0x58): the PR 256
/// review found the tell faint (follow-up 4, adopted default GWP-AD3).
pub const SHADOW_ALPHA: u8 = 0x70;

impl Piece {
    /// The sheet this piece stands in for, if any. A presenter draws such a piece
    /// only while that sheet's atlas is missing (the placeholder); a piece with no
    /// sheet is a footprint (steam warning area and edge, the drum's blast area)
    /// and is always drawn.
    pub fn sheet(&self) -> Option<Sheet> {
        match self.label {
            "boiler landmark (stand-in)"
            | "boiler band"
            | "boiler dome"
            | "boiler damage notch" => Some(Sheet::Boiler),
            "valve body (stand-in)" | "valve wheel" => Some(Sheet::Valve),
            "brace (stand-in)" | "brace crack" | "brace stump" => Some(Sheet::Brace),
            "weight chain" => Some(Sheet::Chain),
            "hanging weight (stand-in)" => Some(Sheet::Weight),
            "landed remnant (stand-in)" | "rubble chunk" => Some(Sheet::Remnant),
            "fuel drum (stand-in)" | "drum band" => Some(Sheet::Drum),
            "drum scorch (stand-in)" => Some(Sheet::Scorch),
            "steam plume" => Some(Sheet::Plume),
            "weight footprint shadow" | "weight footprint edge" | "weight landing" => {
                Some(Sheet::Shadow)
            }
            _ => None,
        }
    }
}

/// A cell of `sheet` standing on `bottom` with its left edge at `x`.
fn cell(sheet: Sheet, x: i32, bottom: i32) -> IRect {
    let (w, h) = sheet.frame_size();
    rect(x, bottom - h, w, h)
}

/// The frame of the boiler body: the damaged loop once a connected piece broke,
/// the strain pair while the valve warns, else the idle loop.
fn boiler_frame(enc: &Encounter, env: &EnvRuntime, tick: u64) -> usize {
    let mut warning = false;
    let mut broken = false;
    for (i, p) in enc.props.iter().enumerate() {
        match p.kind {
            PropKind::Valve => match env.valve_state(i) {
                Some(ValveState::Warning) => warning = true,
                Some(ValveState::Venting | ValveState::Spent) => broken = true,
                _ => {}
            },
            PropKind::WeightRig => broken |= env.mass_state(i) == Some(MassState::Landed),
            _ => {}
        }
    }
    if broken {
        BOILER_DAMAGED.frame_at(tick)
    } else if warning {
        BOILER_STRAIN.frame_at(tick)
    } else {
        BOILER_IDLE.frame_at(tick)
    }
}

/// The sprites of the props at `tick`, back to front. Read-only; the same inputs
/// always give the same list. `charges` are the scene's blast charges (only a
/// detonated drum's `detonate_tick` is read, for the scorch's ember fade).
pub fn sprites(
    enc: &Encounter,
    env: &EnvRuntime,
    charges: &[BlastCharge],
    tick: u64,
) -> Vec<SpriteDraw> {
    let mut out: Vec<SpriteDraw> = Vec::with_capacity(MAX_SPRITES);
    let mut add = |sheet: Sheet, frame: usize, rect: IRect, layer: Layer, alpha: u8| {
        out.push(SpriteDraw {
            sheet,
            frame,
            rect,
            layer,
            alpha,
        });
    };
    if let Some(body) = landmark(enc) {
        add(
            Sheet::Boiler,
            boiler_frame(enc, env, tick),
            cell(Sheet::Boiler, body.x, body.bottom()),
            Layer::Back,
            255,
        );
    }
    for (i, prop) in enc.props.iter().enumerate() {
        let t = prop.target;
        match prop.kind {
            PropKind::Valve => {
                let frame = match env.valve_state(i) {
                    Some(ValveState::Intact) | None => VALVE_INTACT.frame_at(tick),
                    Some(ValveState::Warning) => VALVE_WARNING.frame_at(tick),
                    Some(ValveState::Venting) => VALVE_VENTING.frame_at(tick),
                    Some(ValveState::Spent) => VALVE_SPENT.frame_at(tick),
                };
                let (w, _) = Sheet::Valve.frame_size();
                add(
                    Sheet::Valve,
                    frame,
                    cell(Sheet::Valve, t.x + (t.w - w) / 2, t.bottom()),
                    Layer::Prop,
                    255,
                );
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
                // The ceiling rig and the chains hang from it (Back layer).
                let rig_frame = match state {
                    MassState::Supported | MassState::Weakened => RIG.first,
                    MassState::Released => RIG_RUN.frame_at(tick),
                    MassState::Falling | MassState::Landed => RIG_RUN.last(),
                };
                let (rig_w, rig_h) = Sheet::Rig.frame_size();
                add(
                    Sheet::Rig,
                    rig_frame,
                    rect(t.x, RIG_TOP_Y, rig_w, rig_h),
                    Layer::Back,
                    255,
                );
                let (chain_w, chain_h) = Sheet::Chain.frame_size();
                if matches!(state, MassState::Supported | MassState::Weakened) {
                    let mut y = CHAIN_TOP_Y;
                    while y < t.y {
                        add(
                            Sheet::Chain,
                            CHAIN.first,
                            rect(t.x + BRACE_CHAIN_DX, y, chain_w, chain_h),
                            Layer::Back,
                            255,
                        );
                        y += chain_h;
                    }
                }
                // The brace: whole, creaking, or its stump.
                let brace = match state {
                    MassState::Supported => BRACE_SUPPORTED.frame_at(tick),
                    MassState::Weakened => BRACE_WEAKENED.frame_at(tick),
                    _ => BRACE_STUMP.frame_at(tick),
                };
                let (brace_w, _) = Sheet::Brace.frame_size();
                add(
                    Sheet::Brace,
                    brace,
                    cell(Sheet::Brace, t.x + (t.w - brace_w) / 2, t.bottom()),
                    Layer::Prop,
                    255,
                );
                if let Some(body) = mass.mass_rect() {
                    // The chain runs up from the weight while it still hangs.
                    if state != MassState::Falling {
                        let mut y = CHAIN_TOP_Y;
                        while y < body.y {
                            add(
                                Sheet::Chain,
                                CHAIN.first,
                                rect(body.x + WEIGHT_CHAIN_DX + sway, y, chain_w, chain_h),
                                Layer::Back,
                                255,
                            );
                            y += chain_h;
                        }
                    }
                    let (weight_w, _) = Sheet::Weight.frame_size();
                    let frame = if state == MassState::Falling {
                        WEIGHT_FALL.frame_at(tick)
                    } else {
                        WEIGHT_HANG.frame_at(tick)
                    };
                    add(
                        Sheet::Weight,
                        frame,
                        cell(
                            Sheet::Weight,
                            body.x + (body.w - weight_w) / 2 + sway,
                            body.bottom(),
                        ),
                        Layer::Prop,
                        255,
                    );
                }
                if let Some(remnant) = mass.remnant() {
                    let (remnant_w, _) = Sheet::Remnant.frame_size();
                    add(
                        Sheet::Remnant,
                        REMNANT.frame_at(tick),
                        cell(
                            Sheet::Remnant,
                            remnant.x + (remnant.w - remnant_w) / 2,
                            remnant.bottom(),
                        ),
                        Layer::Prop,
                        255,
                    );
                }
            }
            PropKind::Drum => {
                match env.drum_state(i) {
                    Some(DrumState::Detonated) => {
                        // The drum is gone: its scorch mark, 4 px wider than the drum each
                        // side. Its embers fade from the charge's detonation tick; with no
                        // known tick the cold last frame holds.
                        let age = u16::try_from(i)
                            .ok()
                            .and_then(|prop| env.drum_charge(prop))
                            .and_then(|c| charges.get(c))
                            .filter(|c| c.detonated)
                            .and_then(|c| tick.checked_sub(c.detonate_tick));
                        add(
                            Sheet::Scorch,
                            age.map_or(SCORCH.last(), |a| SCORCH.frame_once(a)),
                            cell(Sheet::Scorch, t.x - 4, t.bottom()),
                            Layer::Prop,
                            255,
                        );
                    }
                    other => {
                        let frame = if other == Some(DrumState::Telegraph) {
                            DRUM_TELEGRAPH.frame_at(tick)
                        } else {
                            DRUM_ARMED.frame_at(tick)
                        };
                        let (drum_w, _) = Sheet::Drum.frame_size();
                        add(
                            Sheet::Drum,
                            frame,
                            cell(Sheet::Drum, t.x + (t.w - drum_w) / 2, t.bottom()),
                            Layer::Prop,
                            255,
                        );
                    }
                }
                if let Some(area) = env.drum_telegraph_rect(i) {
                    let (ring_w, _) = Sheet::Ring.frame_size();
                    add(
                        Sheet::Ring,
                        DRUM_BLAST_TELL.frame_at(tick),
                        cell(Sheet::Ring, area.x + (area.w - ring_w) / 2, area.bottom()),
                        Layer::Tell,
                        255,
                    );
                }
            }
            PropKind::RotSticker | PropKind::Shutter => {}
        }
    }
    // The hazard tells and the steam, from the same feed the companion reads.
    for nav in env.nav_feed() {
        match (nav.class, nav.phase) {
            (HazardClass::Steam, NavPhase::Telegraph) => {
                add(
                    Sheet::Sputter,
                    STEAM_SPUTTER.frame_at(tick),
                    rect(
                        nav.rect.x,
                        nav.rect.y + SPUTTER_DY,
                        Sheet::Sputter.frame_size().0,
                        Sheet::Sputter.frame_size().1,
                    ),
                    Layer::Tell,
                    255,
                );
            }
            (HazardClass::Steam, NavPhase::Active) => {
                add(
                    Sheet::Plume,
                    STEAM_PLUME.frame_at(tick),
                    rect(
                        nav.rect.x,
                        nav.rect.y,
                        Sheet::Plume.frame_size().0,
                        Sheet::Plume.frame_size().1,
                    ),
                    Layer::Tell,
                    STEAM_ALPHA,
                );
            }
            (HazardClass::FallingMass, phase @ (NavPhase::Telegraph | NavPhase::Active)) => {
                let (shadow_w, _) = Sheet::Shadow.frame_size();
                let frame = if phase == NavPhase::Telegraph {
                    FALL_SHADOW_TELL.frame_at(tick)
                } else {
                    FALL_SHADOW.frame_at(tick)
                };
                add(
                    Sheet::Shadow,
                    frame,
                    cell(
                        Sheet::Shadow,
                        nav.rect.x + (nav.rect.w - shadow_w) / 2,
                        nav.rect.bottom(),
                    ),
                    Layer::Tell,
                    SHADOW_ALPHA,
                );
            }
            _ => {}
        }
    }
    out.sort_by_key(|p| p.layer);
    out
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
