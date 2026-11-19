//! The boss' art binding (HUD-DEBT-03, art integration). Bevy-free.
//!
//! The encounter draws from six exported atlases, each a single row of equal
//! cells (the layout `scripts/export_assets.sh` produces):
//!
//! | [`SheetId`] | Manifest | Content |
//! | --- | --- | --- |
//! | `Base` | `juice-01` | the JUICE-01 base set: idle, walk, pound tell / strike / recover, hurt, down |
//! | `Enc` | `boss-f01` | the encounter set: charge tell and run, crash stun, phase roar, pinned, pinned dead |
//! | `Vfx` | `boss-f01-vfx` | pound debris, ripple, crash dust, roar ring, mirror crack, floor dust, boil spatter, blood pool, rack shadow |
//! | `Rack` | `boss-f01-rack` | the plate stack (hang, shudder, fall, landed, landed on the boss) and the ceiling pulley |
//! | `Strut` | `boss-f01-strut` | the rotten strut in three damage states |
//! | `Ledge` | `boss-f01-ledge` | the release plate (raised, pressed) |
//!
//! The binding table is section 8 of `docs/02-art/floor-01-boss-design.md`.
//! [`pose_of`] maps the simulation's [`BossMode`] to a tag and a time inside
//! the tag, [`draws`] lists every sprite of the encounter for one tick (the
//! body, the racks, the pulleys, the strut, the ledge, the floor shadow and
//! every effect). Both read the simulation only: nothing here feeds back into
//! it, so a hash can never change through art.
//!
//! The emblem (`boss-f01-emblem`) is drawn by the HUD adapter, not here.
//!
//! With the base or the encounter atlas missing [`BossArt::load`] answers
//! `None` and the scene draws the placeholder rects at the spec sizes
//! ([`PLACEHOLDER_FRAME`]). A missing optional sheet (effects, rack, strut,
//! ledge) falls back to its own placeholder only.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::boss::{Boss, BossMode, RackState};
use crate::combat::CombatEvent;

/// Frame size and pivot of the placeholder (and of JUICE-01): 160 x 136 with
/// the feet pivot at (72, 135).
pub const PLACEHOLDER_FRAME: (i32, i32) = (160, 136);
pub const PLACEHOLDER_PIVOT: (i32, i32) = (72, 135);
/// The HUD emblem key (18 x 18).
pub const EMBLEM_KEY: &str = "brute_boss_f01_emblem";

/// Height of the rack's underside while it hangs, over the walking line.
pub const HANG_PX: f32 = 189.0;
/// The walk cycle advances one frame per this many pixels walked
/// (JUICE-01: 16 ticks per frame at 72 px/s is 19.2 px).
pub const WALK_STEP_PX: f32 = 19.2;
/// Ticks of the rack's shudder at the start of its fall (the shadow's first
/// third), before the falling frame takes over.
pub const SHUDDER_TICKS: u16 = 12;
/// Ticks the shadow shows each of its three widths.
pub const SHADOW_STEP_TICKS: u16 = 12;
/// Ticks into the defeat at which a felled boss meets the floor (JUICE-01
/// `down`: 6 + 10 ticks), where the floor dust and the boils start.
pub const FLOOR_IMPACT_TICKS: u16 = 16;
/// Height above the walking line of the mirror crack's impact point.
pub const CRACK_HEIGHT_PX: f32 = 40.0;
/// Height of the boil spatter's origin above the floor contact row.
pub const SPATTER_HEIGHT_PX: f32 = 19.0;

/// The atlases of the encounter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SheetId {
    Base,
    Enc,
    Vfx,
    Rack,
    Strut,
    Ledge,
}

impl SheetId {
    pub const ALL: [Self; 6] = [
        Self::Base,
        Self::Enc,
        Self::Vfx,
        Self::Rack,
        Self::Strut,
        Self::Ledge,
    ];

    /// The manifest under `assets/manifests/` (without `.json`).
    pub const fn manifest(self) -> &'static str {
        match self {
            Self::Base => "juice-01",
            Self::Enc => "boss-f01",
            Self::Vfx => "boss-f01-vfx",
            Self::Rack => "boss-f01-rack",
            Self::Strut => "boss-f01-strut",
            Self::Ledge => "boss-f01-ledge",
        }
    }

    /// The animation tags the sheet must provide.
    pub const fn tags(self) -> &'static [&'static str] {
        match self {
            Self::Base => &[
                "idle",
                "walk",
                "attack_tell",
                "attack_pound",
                "attack_recover",
                "hurt",
                "down",
            ],
            Self::Enc => &[
                "charge_tell",
                "charge_run",
                "crash_stun",
                "phase_roar",
                "pinned",
                "pinned_dead",
            ],
            Self::Vfx => &[
                "pound_debris",
                "shockwave_ripple",
                "crash_dust",
                "roar_ring",
                "mirror_crack",
                "floor_dust",
                "boil_spatter",
                "blood_pool",
                "rack_shadow",
            ],
            Self::Rack => &[
                "rack_hang",
                "rack_shudder",
                "rack_fall",
                "rack_landed",
                "rack_landed_boss",
                "rack_pulley",
            ],
            Self::Strut => &["strut_sound", "strut_cracked", "strut_splintered"],
            Self::Ledge => &["plate_raised", "plate_pressed"],
        }
    }

    /// The manifest's object holding the frame list.
    const fn manifest_key(self) -> &'static str {
        match self {
            Self::Base => "juicer",
            _ => "boss",
        }
    }
}

/// How a tag plays: the manifest's `playback`, kept as the code needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Play {
    /// Loop over the tag's ticks.
    Loop,
    /// Once, then the last frame stays.
    Once,
}

/// A time inside a tag, or an explicit frame inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum At {
    /// Ticks since the tag started.
    Tick(u32, Play),
    /// The n-th frame of the tag (clamped).
    Frame(usize),
}

/// Which `(sheet, tag, time)` the boss' body shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pose {
    pub sheet: SheetId,
    pub tag: &'static str,
    pub at: At,
}

/// What the presentation knows beyond the simulation state: the sim tick
/// (idle loop clock) and whether the boss walked since the last tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Gait {
    pub clock: u64,
    pub walking: bool,
    /// Ticks since the boss was defeated. `Boss::step` stops advancing
    /// `mode_tick` once he is down, so [`draws`] takes the age from the
    /// `EnemyDefeated` event instead ([`defeat_age`]).
    pub defeat_age: Option<u32>,
}

/// Ticks since the boss' `EnemyDefeated` event at `now` (`None` while he has
/// not fallen in this log).
pub fn defeat_age(events: &[CombatEvent], now: u64) -> Option<u32> {
    events.iter().rev().find_map(|event| match *event {
        CombatEvent::EnemyDefeated { tick, target, .. }
            if target == crate::boss::BOSS_ENEMY_ID && tick <= now =>
        {
            Some((now - tick).min(u64::from(u32::MAX)) as u32)
        }
        _ => None,
    })
}

/// Which pose the boss shows. Section 8's table, state by state.
///
/// Deviations (no frame exists for the state, so the nearest is used):
/// `Turn` shows the first walk frame, `DoubleStep` (the step between the two
/// pounds) the pound recovery, `Backstep` the flinch (`hurt`).
pub fn pose_of(boss: &Boss, gait: Gait) -> Pose {
    let tick = u32::from(boss.mode_tick);
    let base = |tag, at| Pose {
        sheet: SheetId::Base,
        tag,
        at,
    };
    let enc = |tag, at| Pose {
        sheet: SheetId::Enc,
        tag,
        at,
    };
    let once = |t| At::Tick(t, Play::Once);
    match boss.mode {
        BossMode::Dormant => base("idle", At::Tick(clock32(gait.clock), Play::Loop)),
        BossMode::Neutral => {
            if gait.walking {
                // The cycle follows the ground covered, not the clock.
                let phase = (boss.x * f32::from(boss.facing) / WALK_STEP_PX).floor() as i64;
                base("walk", At::Frame(phase.rem_euclid(4) as usize))
            } else {
                base("idle", At::Tick(clock32(gait.clock), Play::Loop))
            }
        }
        BossMode::Turn => base("walk", At::Frame(0)),
        BossMode::PoundTell | BossMode::SecondTell => base("attack_tell", once(tick)),
        BossMode::PoundStrike | BossMode::SecondStrike => base("attack_pound", once(tick)),
        BossMode::PoundRecover | BossMode::SecondRecover | BossMode::DoubleStep => {
            base("attack_recover", once(tick))
        }
        BossMode::ChargeTell => enc("charge_tell", once(tick)),
        BossMode::Charge => enc("charge_run", At::Tick(tick, Play::Loop)),
        // The crash is 6 ticks (hit, slump); the stun holds the last frame.
        BossMode::Rebound => enc("crash_stun", once(tick)),
        BossMode::Stun => enc("crash_stun", once(tick + 6)),
        BossMode::Roar => enc("phase_roar", once(tick)),
        BossMode::Flinch | BossMode::Backstep => base("hurt", once(tick)),
        BossMode::Pinned => enc("pinned", once(tick)),
        BossMode::Defeated => {
            if boss.defeated_cause_rack {
                enc("pinned_dead", At::Frame(0))
            } else {
                base("down", once(gait.defeat_age.unwrap_or(tick)))
            }
        }
    }
}

fn clock32(clock: u64) -> u32 {
    (clock % u64::from(u32::MAX)) as u32
}

#[derive(Deserialize)]
struct ManifestData {
    frame_size: [i32; 2],
    pivot: [i32; 2],
    output_atlas: String,
    #[serde(default)]
    boss: Option<FrameList>,
    #[serde(default)]
    juicer: Option<FrameList>,
}

#[derive(Deserialize)]
struct FrameList {
    #[serde(default)]
    frames: Vec<ManifestFrame>,
}

#[derive(Deserialize)]
struct ManifestFrame {
    #[serde(default)]
    index: Option<usize>,
    #[serde(default)]
    atlas_index: Option<usize>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    ticks: Option<u32>,
    #[serde(default)]
    pivot: Option<[i32; 2]>,
}

#[derive(Deserialize)]
struct AtlasFile {
    frames: Vec<AtlasFrame>,
    meta: AtlasMeta,
}

#[derive(Deserialize)]
struct AtlasFrame {
    frame: AtlasRect,
}

#[derive(Deserialize)]
struct AtlasRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

#[derive(Deserialize)]
struct AtlasMeta {
    #[serde(rename = "frameTags")]
    frame_tags: Vec<Tag>,
}

#[derive(Deserialize)]
struct Tag {
    name: String,
    from: usize,
    to: usize,
}

/// One exported atlas: a single row of equal cells, its tags, the ticks each
/// frame is shown and the per-frame pivots.
#[derive(Debug, Clone)]
pub struct BossSheet {
    pub id: SheetId,
    /// Atlas image path relative to the Bevy asset root.
    pub asset: String,
    pub frame_size: (i32, i32),
    pub count: usize,
    tags: BTreeMap<String, (usize, usize)>,
    names: BTreeMap<String, usize>,
    ticks: Vec<Option<u32>>,
    pivots: Vec<(i32, i32)>,
}

impl BossSheet {
    /// Build from the manifest text and the exported atlas JSON. A required
    /// tag the atlas lacks, or an atlas that is not a single row of cells of
    /// the manifest's frame size, is an error naming it.
    pub fn parse(id: SheetId, manifest: &str, atlas: &str) -> Result<Self, String> {
        let name = id.manifest();
        let manifest: ManifestData =
            serde_json::from_str(manifest).map_err(|e| format!("{name} manifest: {e}"))?;
        let atlas: AtlasFile =
            serde_json::from_str(atlas).map_err(|e| format!("{name} atlas json: {e}"))?;
        let (w, h) = (manifest.frame_size[0], manifest.frame_size[1]);
        for (i, f) in atlas.frames.iter().enumerate() {
            let r = &f.frame;
            if (r.x, r.y, r.w, r.h) != (i as i32 * w, 0, w, h) {
                return Err(format!(
                    "{name} atlas frame {i} is not cell {i} of a single row of {w}x{h}"
                ));
            }
        }
        let count = atlas.frames.len();
        let tags: BTreeMap<String, (usize, usize)> = atlas
            .meta
            .frame_tags
            .into_iter()
            .map(|t| (t.name, (t.from, t.to)))
            .collect();
        if let Some(missing) = id.tags().iter().find(|t| !tags.contains_key(**t)) {
            return Err(format!("{name} atlas has no tag {missing:?}"));
        }
        if tags.values().any(|&(from, to)| from > to || to >= count) {
            return Err(format!("{name} atlas has a tag outside its frames"));
        }
        let frames = match id.manifest_key() {
            "juicer" => manifest.juicer,
            _ => manifest.boss,
        }
        .map(|l| l.frames)
        .unwrap_or_default();
        let mut ticks = vec![None; count];
        let mut pivots = vec![(manifest.pivot[0], manifest.pivot[1]); count];
        let mut names = BTreeMap::new();
        for f in frames {
            let Some(i) = f.index.or(f.atlas_index).filter(|&i| i < count) else {
                continue;
            };
            ticks[i] = f.ticks;
            if let Some(p) = f.pivot {
                pivots[i] = (p[0], p[1]);
            }
            if let Some(n) = f.name {
                names.insert(n, i);
            }
        }
        let asset = manifest
            .output_atlas
            .strip_prefix(crate::env_kit::ATLAS_ASSET_PREFIX)
            .ok_or_else(|| format!("{name} manifest: atlas is not below generated/art/"))?
            .to_owned();
        Ok(Self {
            id,
            asset,
            frame_size: (w, h),
            count,
            tags,
            names,
            ticks,
            pivots,
        })
    }

    /// The atlas index of the n-th frame of `tag`, clamped to the tag.
    pub fn tag_frame(&self, tag: &str, n: usize) -> Option<usize> {
        let (from, to) = *self.tags.get(tag)?;
        Some((from + n).min(to))
    }

    /// The atlas index of a frame by its manifest name (`strut_cracked`).
    pub fn named(&self, name: &str) -> Option<usize> {
        self.names.get(name).copied()
    }

    /// The pivot of an atlas frame.
    pub fn pivot(&self, index: usize) -> (i32, i32) {
        self.pivots.get(index).copied().unwrap_or((0, 0))
    }

    /// The atlas index shown `t` ticks into `tag`. A frame without a tick
    /// count (a hold) stays; past the end the last frame stays unless the tag
    /// loops.
    pub fn frame_at(&self, tag: &str, t: u32, play: Play) -> Option<usize> {
        let (from, to) = *self.tags.get(tag)?;
        let total: Option<u32> = self.ticks[from..=to].iter().copied().sum();
        let mut t = match (play, total) {
            (Play::Loop, Some(total)) if total > 0 => t % total,
            _ => t,
        };
        for (i, ticks) in self.ticks.iter().enumerate().take(to + 1).skip(from) {
            match *ticks {
                None => return Some(i),
                Some(d) if t < d => return Some(i),
                Some(d) => t -= d,
            }
        }
        Some(to)
    }

    /// The ticks the tag plays (`None` for a tag that ends in a hold).
    pub fn tag_ticks(&self, tag: &str) -> Option<u32> {
        let (from, to) = *self.tags.get(tag)?;
        self.ticks[from..=to].iter().copied().sum()
    }

    /// The atlas index of a [`Pose`]-style `(tag, at)`.
    pub fn resolve(&self, tag: &str, at: At) -> Option<usize> {
        match at {
            At::Tick(t, play) => self.frame_at(tag, t, play),
            At::Frame(n) => self.tag_frame(tag, n),
        }
    }
}

/// The boss atlases as the scene draws them.
#[derive(Debug, Clone)]
pub struct BossArt {
    sheets: [Option<BossSheet>; 6],
}

impl BossArt {
    /// Load every atlas whose manifest and exported atlas exist. `Ok(None)`
    /// means the body has no art yet (the scene draws placeholders); an error
    /// means a sheet is present but wrong.
    pub fn load(root: &Path) -> Result<Option<Self>, String> {
        let mut sheets: [Option<BossSheet>; 6] = [None, None, None, None, None, None];
        for id in SheetId::ALL {
            let manifest_path = root.join(format!("assets/manifests/{}.json", id.manifest()));
            let Ok(manifest) = std::fs::read_to_string(&manifest_path) else {
                continue;
            };
            let meta: serde_json::Value = serde_json::from_str(&manifest)
                .map_err(|e| format!("{} manifest: {e}", id.manifest()))?;
            let Some(meta_path) = meta["output_metadata"].as_str() else {
                return Err(format!("{} manifest: no output_metadata", id.manifest()));
            };
            let Ok(atlas) = std::fs::read_to_string(root.join(meta_path)) else {
                // The manifest is committed before the export has run.
                continue;
            };
            sheets[id as usize] = Some(BossSheet::parse(id, &manifest, &atlas)?);
        }
        let art = Self { sheets };
        Ok(
            (art.sheet(SheetId::Base).is_some() && art.sheet(SheetId::Enc).is_some())
                .then_some(art),
        )
    }

    /// Assemble from sheets (tests).
    pub fn from_sheets(sheets: Vec<BossSheet>) -> Self {
        let mut all: [Option<BossSheet>; 6] = [None, None, None, None, None, None];
        for sheet in sheets {
            let i = sheet.id as usize;
            all[i] = Some(sheet);
        }
        Self { sheets: all }
    }

    pub fn sheet(&self, id: SheetId) -> Option<&BossSheet> {
        self.sheets[id as usize].as_ref()
    }

    /// The sheets present, in [`SheetId`] order.
    pub fn sheets(&self) -> impl Iterator<Item = &BossSheet> {
        self.sheets.iter().flatten()
    }

    /// The atlas frame the boss body shows now.
    pub fn body_frame(&self, boss: &Boss, gait: Gait) -> Option<(SheetId, usize)> {
        let pose = pose_of(boss, gait);
        let sheet = self.sheet(pose.sheet)?;
        Some((pose.sheet, sheet.resolve(pose.tag, pose.at)?))
    }
}

// ------------------------------------------------------------------ drawing

/// Depth band of a sprite (relative to the actors' plane).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    /// Over the floor, under every actor (shadow, pool).
    Floor,
    /// Behind the actors, over the wall (pulley, strut, ledge, crack, ring).
    Back,
    /// The boss.
    Body,
    /// Over the boss, under Carl (the rack, the effects of the body).
    Front,
}

/// One sprite of the encounter this tick: the frame, where its pivot goes
/// (world, y up; `y` is the pivot row's ground line) and the depth band.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Draw {
    pub sheet: SheetId,
    pub frame: usize,
    pub x: f32,
    pub y: f32,
    pub flip: bool,
    pub layer: Layer,
}

/// The inputs of [`draws`] beyond the boss: the event log, the sim tick, the
/// body's [`Gait`] and whether the gore preset is the full one (the boils and
/// the pool of the finish are preset 4 only).
#[derive(Debug, Clone, Copy)]
pub struct FxCtx<'a> {
    pub events: &'a [CombatEvent],
    pub now: u64,
    pub gait: Gait,
    pub gore_full: bool,
}

/// Every sprite of the encounter this tick, in drawing order. Sheets that are
/// absent contribute nothing (the scene then draws their placeholders).
pub fn draws(art: &BossArt, boss: &Boss, ctx: FxCtx) -> Vec<Draw> {
    let mut out = Vec::new();
    let data = boss.data();
    let floor = boss.floor_y();
    let flip_of = |facing: i8| facing < 0;

    // The age of the defeat comes from the event log: the boss' own clock
    // stops at the defeat.
    let gait = Gait {
        defeat_age: defeat_age(ctx.events, ctx.now).or(ctx.gait.defeat_age),
        ..ctx.gait
    };
    // The body.
    if let Some((sheet, frame)) = art.body_frame(boss, gait) {
        out.push(Draw {
            sheet,
            frame,
            x: boss.x,
            y: floor,
            flip: flip_of(boss.facing),
            layer: Layer::Body,
        });
    }

    // The racks, their pulleys and shadows, the strut and the ledge.
    let boss_pinned = boss.mode == BossMode::Pinned
        || (boss.mode == BossMode::Defeated && boss.defeated_cause_rack);
    let crushed_rack = (0..2)
        .filter(|&i| boss.racks[i].enabled && boss.racks[i].state == RackState::Landed)
        .min_by(|&a, &b| {
            let d = |i: usize| (data.footprint_centre(boss.racks[i].facing) - boss.x).abs();
            d(a).total_cmp(&d(b))
        })
        .filter(|&i| {
            boss_pinned
                && (data.footprint_centre(boss.racks[i].facing) - boss.x).abs()
                    <= data.rack.footprint_px
        });
    if let Some(rack) = art.sheet(SheetId::Rack) {
        for (i, state) in boss.racks.iter().enumerate().filter(|(_, r)| r.enabled) {
            let centre = data.footprint_centre(state.facing);
            let flip = state.facing > 0;
            let mut push = |tag: &str, n: usize, y: f32, layer| {
                if let Some(frame) = rack.tag_frame(tag, n) {
                    out.push(Draw {
                        sheet: SheetId::Rack,
                        frame,
                        x: centre,
                        y,
                        flip,
                        layer,
                    });
                }
            };
            push("rack_pulley", 0, floor + HANG_PX, Layer::Back);
            match state.state {
                RackState::Idle { .. } => push("rack_hang", 0, floor + HANG_PX, Layer::Front),
                RackState::Falling { ticks, .. } => {
                    let t = f32::from(ticks) / f32::from(data.rack.fall_ticks);
                    let y = floor + HANG_PX * (1.0 - t * t);
                    if ticks < SHUDDER_TICKS {
                        push(
                            "rack_shudder",
                            usize::from((ticks / 3) % 2),
                            y,
                            Layer::Front,
                        );
                    } else {
                        push("rack_fall", 0, y, Layer::Front);
                    }
                }
                RackState::Landed => {
                    if crushed_rack == Some(i) {
                        push("rack_landed_boss", 0, floor, Layer::Front);
                    } else {
                        push("rack_landed", 0, floor, Layer::Front);
                    }
                }
            }
        }
    }
    if let Some(vfx) = art.sheet(SheetId::Vfx) {
        for state in boss.racks.iter().filter(|r| r.enabled) {
            if let RackState::Falling { ticks, .. } = state.state {
                let n = usize::from(ticks / SHADOW_STEP_TICKS).min(2);
                if let Some(frame) = vfx.tag_frame("rack_shadow", n) {
                    out.push(Draw {
                        sheet: SheetId::Vfx,
                        frame,
                        x: data.footprint_centre(state.facing),
                        y: floor,
                        flip: false,
                        layer: Layer::Floor,
                    });
                }
            }
        }
    }
    if let Some(strut) = art.sheet(SheetId::Strut)
        && data.rack.left.enabled
        && let RackState::Idle { hits } = boss.racks[0].state
    {
        let name = match hits {
            0 => "strut_sound",
            1 => "strut_cracked",
            _ => "strut_splintered",
        };
        if let Some(frame) = strut.named(name) {
            out.push(Draw {
                sheet: SheetId::Strut,
                frame,
                x: boss.strut_rect().center().0,
                y: floor,
                flip: false,
                layer: Layer::Back,
            });
        }
    }
    if let Some(ledge) = art.sheet(SheetId::Ledge)
        && data.rack.right.enabled
    {
        let pressed = match boss.racks[1].state {
            RackState::Idle { hits } => hits > 0,
            _ => true,
        };
        let name = if pressed {
            "plate_pressed"
        } else {
            "plate_raised"
        };
        if let Some(frame) = ledge.named(name) {
            let right = &data.rack.right;
            out.push(Draw {
                sheet: SheetId::Ledge,
                frame,
                x: (right.ledge_x[0] + right.ledge_x[1]) * 0.5,
                y: floor + right.ledge_height_px,
                flip: false,
                layer: Layer::Back,
            });
        }
    }

    // The effects.
    if let Some(vfx) = art.sheet(SheetId::Vfx) {
        let strike = i64::from(boss.mode_tick) - i64::from(data.pound.smear);
        let impact = match boss.mode {
            BossMode::PoundStrike | BossMode::SecondStrike => Some(strike),
            BossMode::PoundRecover | BossMode::SecondRecover => {
                Some(i64::from(data.pound.impact) + i64::from(boss.mode_tick))
            }
            _ => None,
        };
        if let Some(t) = impact {
            fx(
                &mut out,
                vfx,
                "pound_debris",
                t,
                Play::Once,
                boss.x + f32::from(boss.facing) * data.shockwave.origin_px,
                floor,
                false,
                Layer::Front,
            );
        }
        for (x, dir) in boss.ripples() {
            // One frame per 10 px travelled (4 ticks at 150 px/s).
            let step = (x / 10.0).floor() as i64;
            if let Some(frame) = vfx.tag_frame("shockwave_ripple", step.rem_euclid(4) as usize) {
                out.push(Draw {
                    sheet: SheetId::Vfx,
                    frame,
                    x,
                    y: floor,
                    flip: dir < 0,
                    layer: Layer::Front,
                });
            }
        }
        if boss.mode == BossMode::Roar {
            fx(
                &mut out,
                vfx,
                "roar_ring",
                i64::from(boss.mode_tick),
                Play::Once,
                boss.x,
                floor,
                false,
                Layer::Back,
            );
        }
        let mut cracks = [0usize; 2];
        for event in ctx.events {
            if let CombatEvent::BossCrash { tick, wall } = *event {
                let side = usize::from(wall > 0);
                if tick <= ctx.now {
                    cracks[side] += 1;
                }
                let x = if wall > 0 {
                    data.arena.right_wall_x
                } else {
                    data.arena.left_wall_x
                };
                fx(
                    &mut out,
                    vfx,
                    "crash_dust",
                    ctx.now as i64 - tick as i64,
                    Play::Once,
                    x,
                    floor,
                    wall < 0,
                    Layer::Front,
                );
            }
        }
        for (side, &n) in cracks.iter().enumerate() {
            if n > 0
                && let Some(frame) = vfx.tag_frame("mirror_crack", n.min(3) - 1)
            {
                out.push(Draw {
                    sheet: SheetId::Vfx,
                    frame,
                    x: if side == 1 {
                        data.arena.right_wall_x
                    } else {
                        data.arena.left_wall_x
                    },
                    y: floor + CRACK_HEIGHT_PX,
                    flip: side == 0,
                    layer: Layer::Back,
                });
            }
        }
        if boss.mode == BossMode::Defeated {
            let since = i64::from(gait.defeat_age.unwrap_or(u32::from(boss.mode_tick)));
            let start = if boss.defeated_cause_rack {
                0
            } else {
                i64::from(FLOOR_IMPACT_TICKS)
            };
            let t = since - start;
            if !boss.defeated_cause_rack {
                fx(
                    &mut out,
                    vfx,
                    "floor_dust",
                    t,
                    Play::Once,
                    boss.x,
                    floor,
                    false,
                    Layer::Front,
                );
            }
            if ctx.gore_full {
                fx(
                    &mut out,
                    vfx,
                    "boil_spatter",
                    t,
                    Play::Once,
                    boss.x,
                    floor + SPATTER_HEIGHT_PX,
                    false,
                    Layer::Front,
                );
                fx(
                    &mut out,
                    vfx,
                    "blood_pool",
                    t - 6,
                    Play::Once,
                    boss.x,
                    floor,
                    false,
                    Layer::Floor,
                );
            }
        }
    }
    out
}

/// A one-shot or looping effect `t` ticks old (nothing before it starts or
/// after a one-shot has played).
#[allow(clippy::too_many_arguments)]
fn fx(
    out: &mut Vec<Draw>,
    vfx: &BossSheet,
    tag: &str,
    t: i64,
    play: Play,
    x: f32,
    y: f32,
    flip: bool,
    layer: Layer,
) {
    if t < 0 {
        return;
    }
    let t = t as u32;
    if play == Play::Once && vfx.tag_ticks(tag).is_some_and(|total| t >= total) {
        return;
    }
    if let Some(frame) = vfx.frame_at(tag, t, play) {
        out.push(Draw {
            sheet: SheetId::Vfx,
            frame,
            x,
            y,
            flip,
            layer,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss::builtin;

    /// A sheet with `names.len()` cells, each tag one cell per frame name.
    fn fake_sheet(id: SheetId) -> BossSheet {
        let tags = id.tags();
        let frames: Vec<String> = tags
            .iter()
            .enumerate()
            .map(|(i, _)| format!(r#"{{"frame":{{"x":{},"y":0,"w":10,"h":10}}}}"#, i * 10))
            .collect();
        let tag_json: Vec<String> = tags
            .iter()
            .enumerate()
            .map(|(i, t)| format!(r#"{{"name":"{t}","from":{i},"to":{i}}}"#))
            .collect();
        let atlas = format!(
            r#"{{"frames":[{}],"meta":{{"frameTags":[{}]}}}}"#,
            frames.join(","),
            tag_json.join(",")
        );
        let named: Vec<String> = tags
            .iter()
            .enumerate()
            .map(|(i, t)| format!(r#"{{"index":{i},"name":"{t}","ticks":null,"pivot":[5,9]}}"#))
            .collect();
        let manifest = format!(
            r#"{{"frame_size":[10,10],"pivot":[5,9],"output_atlas":"generated/art/x.png",
            "output_metadata":"generated/art/x.json","{}":{{"frames":[{}]}}}}"#,
            id.manifest_key(),
            named.join(",")
        );
        BossSheet::parse(id, &manifest, &atlas).unwrap()
    }

    fn fake() -> BossArt {
        BossArt::from_sheets(SheetId::ALL.into_iter().map(fake_sheet).collect())
    }

    #[test]
    fn the_binding_resolves_every_mode_to_a_frame() {
        let art = fake();
        let mut boss = Boss::new(builtin(), 0.0);
        for mode in [
            BossMode::Dormant,
            BossMode::Neutral,
            BossMode::Turn,
            BossMode::PoundTell,
            BossMode::PoundStrike,
            BossMode::PoundRecover,
            BossMode::DoubleStep,
            BossMode::SecondTell,
            BossMode::SecondStrike,
            BossMode::SecondRecover,
            BossMode::ChargeTell,
            BossMode::Charge,
            BossMode::Rebound,
            BossMode::Stun,
            BossMode::Roar,
            BossMode::Flinch,
            BossMode::Backstep,
            BossMode::Pinned,
            BossMode::Defeated,
        ] {
            boss.mode = mode;
            for walking in [false, true] {
                let gait = Gait {
                    clock: 7,
                    walking,
                    ..Gait::default()
                };
                let pose = pose_of(&boss, gait);
                assert!(
                    art.sheet(pose.sheet).unwrap().tags.contains_key(pose.tag),
                    "{mode:?} -> {}",
                    pose.tag
                );
                assert!(art.body_frame(&boss, gait).is_some(), "{mode:?}");
            }
        }
    }

    #[test]
    fn a_rack_defeat_shows_the_dead_pin_and_a_melee_defeat_the_felled_body() {
        let mut boss = Boss::new(builtin(), 0.0);
        boss.mode = BossMode::Defeated;
        assert_eq!(pose_of(&boss, Gait::default()).tag, "down");
        boss.defeated_cause_rack = true;
        let pose = pose_of(&boss, Gait::default());
        assert_eq!((pose.sheet, pose.tag), (SheetId::Enc, "pinned_dead"));
    }

    #[test]
    fn the_walk_cycle_follows_the_ground_covered() {
        let mut boss = Boss::new(builtin(), 0.0);
        boss.mode = BossMode::Neutral;
        let gait = Gait {
            clock: 0,
            walking: true,
            ..Gait::default()
        };
        let mut frames = Vec::new();
        for step in 0..5 {
            boss.x = 100.0 + WALK_STEP_PX * step as f32 + 1.0;
            boss.facing = 1;
            let At::Frame(n) = pose_of(&boss, gait).at else {
                panic!("walk is a frame");
            };
            frames.push(n);
        }
        assert_eq!(frames[0..4].iter().copied().max(), Some(3));
        assert_eq!(frames[0], frames[4], "four frames per cycle");
        assert_ne!(frames[0], frames[1]);
    }

    #[test]
    fn a_frame_past_the_end_of_a_tag_holds_the_last() {
        let sheet = fake_sheet(SheetId::Base);
        assert_eq!(sheet.tag_frame("idle", 0), Some(0));
        assert_eq!(sheet.tag_frame("idle", 9), Some(0));
        assert_eq!(sheet.tag_frame("nope", 0), None);
    }

    #[test]
    fn the_tick_clock_walks_the_frames_of_a_tag() {
        // idle 14, 10, 14 ticks (loop); tell 8 then hold.
        let atlas = r#"{"frames":[{"frame":{"x":0,"y":0,"w":10,"h":10}},
            {"frame":{"x":10,"y":0,"w":10,"h":10}},{"frame":{"x":20,"y":0,"w":10,"h":10}}],
            "meta":{"frameTags":[{"name":"idle","from":0,"to":2}]}}"#;
        let manifest = r#"{"frame_size":[10,10],"pivot":[5,9],
            "output_atlas":"generated/art/x.png","output_metadata":"generated/art/x.json",
            "juicer":{"frames":[{"index":0,"ticks":14},{"index":1,"ticks":10},{"index":2,"ticks":14}]}}"#;
        // Only the idle tag is present, so parse as a custom sheet.
        let sheet = {
            let m: ManifestData = serde_json::from_str(manifest).unwrap();
            assert_eq!(m.frame_size, [10, 10]);
            let a: AtlasFile = serde_json::from_str(atlas).unwrap();
            BossSheet {
                id: SheetId::Base,
                asset: "x.png".into(),
                frame_size: (10, 10),
                count: a.frames.len(),
                tags: BTreeMap::from([("idle".to_owned(), (0, 2))]),
                names: BTreeMap::new(),
                ticks: vec![Some(14), Some(10), Some(14)],
                pivots: vec![(5, 9); 3],
            }
        };
        let at = |t| sheet.frame_at("idle", t, Play::Loop).unwrap();
        assert_eq!(
            [at(0), at(13), at(14), at(23), at(24), at(37), at(38)],
            [0, 0, 1, 1, 2, 2, 0]
        );
        let once = |t| sheet.frame_at("idle", t, Play::Once).unwrap();
        assert_eq!([once(37), once(38), once(999)], [2, 2, 2]);
    }

    #[test]
    fn a_missing_tag_is_an_error_and_missing_art_is_not() {
        let manifest = r#"{"frame_size":[10,10],"pivot":[5,9],
            "output_atlas":"generated/art/b.png","output_metadata":"generated/art/b.json"}"#;
        let atlas = r#"{"frames":[{"frame":{"x":0,"y":0,"w":10,"h":10}}],
            "meta":{"frameTags":[{"name":"idle","from":0,"to":0}]}}"#;
        assert!(BossSheet::parse(SheetId::Base, manifest, atlas).is_err());
        let dir = std::env::temp_dir().join("floorbreak_no_boss_art");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(BossArt::load(&dir).unwrap().is_none());
    }

    #[test]
    fn an_atlas_that_is_not_a_single_row_of_cells_is_an_error() {
        let manifest = r#"{"frame_size":[10,10],"pivot":[5,9],
            "output_atlas":"generated/art/b.png","output_metadata":"generated/art/b.json"}"#;
        let atlas = r#"{"frames":[{"frame":{"x":0,"y":10,"w":10,"h":10}}],
            "meta":{"frameTags":[]}}"#;
        let error = BossSheet::parse(SheetId::Ledge, manifest, atlas).unwrap_err();
        assert!(error.contains("single row"), "{error}");
    }

    #[test]
    fn the_scene_draws_the_body_and_the_rack_set_with_a_stand_in_art() {
        let art = fake();
        let mut boss = Boss::new(builtin(), 0.0);
        boss.mode = BossMode::Neutral;
        let ctx = FxCtx {
            events: &[],
            now: 10,
            gait: Gait::default(),
            gore_full: true,
        };
        let list = draws(&art, &boss, ctx);
        let on = |sheet| list.iter().filter(|d| d.sheet == sheet).count();
        assert_eq!(on(SheetId::Base), 1, "the body");
        // One enabled rack: a pulley, the hung stack; the strut; no ledge.
        assert_eq!(on(SheetId::Rack), 2);
        assert_eq!(on(SheetId::Strut), 1);
        assert_eq!(on(SheetId::Ledge), 0, "the right rack is off");
    }

    /// With the atlases exported (CI runs the export, then this test with
    /// `REQUIRE_EXPORTED_ATLAS=1`, which fails instead of skipping), every
    /// sheet parses and every simulation state, effect and named frame the
    /// binding table (design section 8) uses resolves.
    #[test]
    fn the_exported_boss_atlases_bind_every_state_when_present() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let required = std::env::var_os("REQUIRE_EXPORTED_ATLAS").is_some();
        let art = BossArt::load(&root).unwrap();
        let Some(art) = art else {
            assert!(!required, "the boss atlases are not exported");
            return;
        };
        if SheetId::ALL.iter().any(|id| art.sheet(*id).is_none()) {
            assert!(!required, "a boss sheet is not exported");
            return;
        }
        let mut boss = Boss::new(builtin(), 0.0);
        for mode in [
            BossMode::Dormant,
            BossMode::Neutral,
            BossMode::Turn,
            BossMode::PoundTell,
            BossMode::PoundStrike,
            BossMode::PoundRecover,
            BossMode::DoubleStep,
            BossMode::SecondTell,
            BossMode::SecondStrike,
            BossMode::SecondRecover,
            BossMode::ChargeTell,
            BossMode::Charge,
            BossMode::Rebound,
            BossMode::Stun,
            BossMode::Roar,
            BossMode::Flinch,
            BossMode::Backstep,
            BossMode::Pinned,
            BossMode::Defeated,
        ] {
            boss.mode = mode;
            for tick in [0u16, 1, 5, 9, 20, 40, 100, 400] {
                boss.mode_tick = tick;
                for walking in [false, true] {
                    let gait = Gait {
                        clock: 99,
                        walking,
                        ..Gait::default()
                    };
                    assert!(art.body_frame(&boss, gait).is_some(), "{mode:?} @{tick}");
                }
            }
        }
        // The sheets' named frames the binding addresses.
        let strut = art.sheet(SheetId::Strut).unwrap();
        for name in ["strut_sound", "strut_cracked", "strut_splintered"] {
            assert!(strut.named(name).is_some(), "{name}");
        }
        let ledge = art.sheet(SheetId::Ledge).unwrap();
        for name in ["plate_raised", "plate_pressed"] {
            assert!(ledge.named(name).is_some(), "{name}");
        }
        // Effect lengths the timings assume (design section 3 and the
        // manifests): the shadow's three widths, the crash dust and the roar
        // ring, a pound debris at least as long as the pound's impact.
        let vfx = art.sheet(SheetId::Vfx).unwrap();
        assert_eq!(vfx.tag_ticks("rack_shadow"), Some(36));
        assert_eq!(vfx.tag_ticks("roar_ring"), Some(12));
        assert!(vfx.tag_ticks("pound_debris").unwrap() >= 9);
        // The pound's tell, strike and recovery use JUICE-01's own timing.
        let base = art.sheet(SheetId::Base).unwrap();
        assert_eq!(base.tag_ticks("attack_tell"), Some(36));
        assert_eq!(base.tag_ticks("attack_pound"), Some(9));
        // The state a scene draws on a plain tick lists the body and the rack set.
        let list = draws(
            &art,
            &Boss::new(builtin(), 0.0),
            FxCtx {
                events: &[],
                now: 0,
                gait: Gait::default(),
                gore_full: true,
            },
        );
        assert!(list.iter().any(|d| d.layer == Layer::Body));
        assert!(list.iter().any(|d| d.sheet == SheetId::Rack));
    }

    /// F1 of the PR #120 review: `Boss::step` stops advancing `mode_tick`
    /// after the defeat, so the fall, the floor dust and the boils take their
    /// age from the `EnemyDefeated` event.
    #[test]
    fn the_defeat_plays_from_the_event_age_not_the_frozen_mode_tick() {
        use crate::combat::{DefeatCause, ForceTier};
        let art = fake();
        let mut boss = Boss::new(builtin(), 0.0);
        boss.mode = BossMode::Defeated;
        boss.mode_tick = 0;
        let died = CombatEvent::EnemyDefeated {
            tick: 100,
            target: crate::boss::BOSS_ENEMY_ID,
            cause: DefeatCause::Melee,
            force: ForceTier::Heavy,
            instigator: Some(0),
        };
        assert_eq!(defeat_age(&[], 130), None);
        assert_eq!(defeat_age(std::slice::from_ref(&died), 130), Some(30));
        let ctx = |now| FxCtx {
            events: std::slice::from_ref(&died),
            now,
            gait: Gait::default(),
            gore_full: true,
        };
        let vfx = art.sheet(SheetId::Vfx).unwrap();
        let count = |list: &[Draw], tag: &str| {
            let frame = vfx.tag_frame(tag, 0).unwrap();
            list.iter()
                .filter(|d| d.sheet == SheetId::Vfx && d.frame == frame)
                .count()
        };
        // Before the floor impact (tick 16): the body falls, no dust yet.
        let early = draws(&art, &boss, ctx(110));
        assert_eq!(count(&early, "floor_dust"), 0);
        assert_eq!(count(&early, "boil_spatter"), 0);
        // After it: dust and spatter play.
        let late = draws(&art, &boss, ctx(100 + u64::from(FLOOR_IMPACT_TICKS) + 2));
        assert_eq!(count(&late, "floor_dust"), 1);
        assert_eq!(count(&late, "boil_spatter"), 1);
        // And the body pose follows the age.
        let gait = Gait {
            defeat_age: Some(30),
            ..Gait::default()
        };
        assert_eq!(pose_of(&boss, gait).at, At::Tick(30, Play::Once));
        // A rack kill shows its spatter from the defeat on.
        boss.defeated_cause_rack = true;
        let rack_kill = draws(&art, &boss, ctx(102));
        assert_eq!(count(&rack_kill, "boil_spatter"), 1);
        assert_eq!(
            count(&rack_kill, "floor_dust"),
            0,
            "no floor dust under the stack"
        );
    }
}
