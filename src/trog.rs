//! TROG-A: the troglodyte family, pure core
//! (`docs/01-design/floor-01-troglodyte-integration.md` section 5, TROG-AD1..AD34).
//! Bevy-free, deterministic, headless.
//!
//! One machine, three profiles (the Pygmy's committed leap, the Basher's overhead smash, the
//! Virtuoso's clipped tongue). Every number lives in
//! `assets/data/gameplay/enemy_trog_v0.json` ([`TrogTuning`]), loaded through a validator whose
//! errors name the field and FNV-1a pinned ([`TUNING_HASH`]); every box and window is checked
//! against the art manifests by a drift test (TROG-T02), and the sim never reads art.
//!
//! This module owns everything that does not need the scene: the tuning, the state machine
//! ([`Trog`]), the commit rule (facing, landing x, smash rectangle and line are fixed on the
//! tell's first tick), the interrupt rules (keyed by [`HitKind`], never by poise), the events
//! ([`TrogAct`]) and the state hash. The slot adapter (`combat_scene/trog.rs`) wraps it for the
//! scene.
//!
//! # Coordinates
//!
//! World px, y up, as every slot enemy. Boxes are pivot-relative: x forward (mirrored by the
//! facing), y up from the feet ([`Trog::foot_y`]).
//!
//! # Attack clock
//!
//! The tick on which a tell starts is attack tick 0. Attack tick `a` is a tell for
//! `a < tell_ticks`, active for `tell_ticks <= a <= strike_end` (the hitbox is live on
//! `strike_start..=strike_end`), a recovery for the next `recover_ticks` and then a rest.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::boss::{HeroBlow, HeroView};
use crate::cf02::BoxPx;
use crate::donut_mana::content_hash;
use crate::party::HeroId;
use crate::prototype::{Block, Rect};
use crate::slime::{signed_away, world_box};

/// The tuning file, embedded so the binary and the tests read the same bytes.
pub const TUNING_JSON: &str = include_str!("../assets/data/gameplay/enemy_trog_v0.json");

/// The committed FNV-1a 64 content hash of [`TUNING_JSON`]. Editing the file without updating
/// this value fails [`TrogTuning::pinned`] and the pin test.
pub const TUNING_HASH: u64 = 0xde76_c204_0e24_ea83;

/// The key of the pin in a session manifest's `content_hashes`.
pub const PIN_KEY: &str = "trog_tuning";

/// TROGB-AD2: a standing solid this low (px) or lower never blocks a troglodyte's walk or leap.
pub const LOW_STEP_PX: f32 = 16.0;

/// The `HeroBlow::source` base of a troglodyte (the ASCII "TR"). The roster index goes in the
/// high half ([`trog_source`]).
pub const TROG_SOURCE_ID: u32 = 0x5452;

/// The source of the troglodyte at roster index `index`.
pub const fn trog_source(index: u8) -> u32 {
    TROG_SOURCE_ID | ((index as u32) << 16)
}

// --------------------------------------------------------------- the data

/// A tuning file error. A field error names the field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrogTuningError {
    Parse(String),
    Field { field: String, reason: String },
    HashMismatch { expected: u64, actual: u64 },
}

impl core::fmt::Display for TrogTuningError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "trog tuning: {msg}"),
            Self::Field { field, reason } => write!(f, "trog tuning: field `{field}`: {reason}"),
            Self::HashMismatch { expected, actual } => write!(
                f,
                "trog tuning hash {actual:#018x} is not the pinned {expected:#018x}"
            ),
        }
    }
}

impl std::error::Error for TrogTuningError {}

/// The three species.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Species {
    Pygmy,
    Basher,
    Virtuoso,
}

impl Species {
    pub const ALL: [Self; 3] = [Self::Pygmy, Self::Basher, Self::Virtuoso];

    /// The sidecar roster kind (`kind`).
    pub const fn kind_name(self) -> &'static str {
        match self {
            Self::Pygmy => "trog_pygmy",
            Self::Basher => "trog_basher",
            Self::Virtuoso => "trog_virtuoso",
        }
    }

    /// The species of a roster kind string.
    pub fn from_kind(kind: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.kind_name() == kind)
    }

    /// The short name (logs, the evidence tables).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pygmy => "pygmy",
            Self::Basher => "basher",
            Self::Virtuoso => "virtuoso",
        }
    }

    const fn code(self) -> u64 {
        match self {
            Self::Pygmy => 1,
            Self::Basher => 2,
            Self::Virtuoso => 3,
        }
    }
}

/// One species' values. Species-specific keys default to 0 for the others.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeciesTuning {
    pub hp: u16,
    /// Walk speed in px per tick.
    pub walk_px_tick: f32,
    /// Half-width of the leash around the spawn x; never left (TROG-AD5).
    pub leash_px: u16,
    /// The tell: attack ticks `0..tell_ticks`.
    pub tell_ticks: u16,
    /// The hitbox is live on `strike_start..=strike_end`; the active window is
    /// `tell_ticks..=strike_end`.
    pub strike_start: u16,
    pub strike_end: u16,
    pub recover_ticks: u16,
    pub rest_ticks: u16,
    pub hurt_ticks: u16,
    /// The attack hitbox, facing +1, pivot-relative.
    pub hitbox: BoxPx,
    /// The knock of a blow in px/s, world frame (y up; the plan's table gives y down).
    pub knock_vx: f32,
    pub knock_vy: f32,
    pub hurt_idle: BoxPx,
    pub hurt_tell: BoxPx,
    pub hurt_active: BoxPx,
    pub hurt_recover: BoxPx,
    pub hurt_hurt: BoxPx,
    // Pygmy
    #[serde(default)]
    pub leap_min_px: u16,
    #[serde(default)]
    pub leap_max_px: u16,
    #[serde(default)]
    pub apex_px: u16,
    #[serde(default)]
    pub trigger_px: u16,
    // Basher
    #[serde(default)]
    pub open_x0: i16,
    #[serde(default)]
    pub open_x1: i16,
    /// TROG-B: the bench shove (read there; validated here).
    #[serde(default)]
    pub bench_shove_px_s: u16,
    #[serde(default)]
    pub bench_min_overlap_px: u16,
    #[serde(default)]
    pub bait_px: u16,
    #[serde(default)]
    pub interrupt_until: u16,
    #[serde(default)]
    pub interrupt_cooldown: u16,
    // Virtuoso
    #[serde(default)]
    pub standoff_px: u16,
}

/// `enemy_trog_v0.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrogTuning {
    pub schema_version: u32,
    /// Damage of a blow, every species (the heroes have 4 HP).
    pub trogq01_damage: u8,
    /// A troglodyte does not start a tell within this many ticks of another tell start.
    pub trogq02_tell_gap_ticks: u16,
    /// A wake or a tell needs the whole hurtbox inside the view, this many px in.
    pub trogq03_wake_margin_px: u16,
    /// Solo: the bot hero counts this much farther away (TROG-AD27).
    pub trogq04_ai_bias_px: u16,
    /// The other hero must be this much nearer to take the target.
    pub trogq05_target_switch_px: u16,
    pub pygmy: SpeciesTuning,
    pub basher: SpeciesTuning,
    pub virtuoso: SpeciesTuning,
}

fn bad(field: &str, reason: &str) -> Result<(), TrogTuningError> {
    Err(TrogTuningError::Field {
        field: field.to_owned(),
        reason: reason.to_owned(),
    })
}

fn in_range(field: &str, value: u32, lo: u32, hi: u32) -> Result<(), TrogTuningError> {
    if value < lo || value > hi {
        return bad(field, &format!("must be {lo}..={hi}, not {value}"));
    }
    Ok(())
}

fn valid_box(field: &str, b: &BoxPx) -> Result<(), TrogTuningError> {
    if b.x0 >= b.x1 || b.y0 >= b.y1 {
        return bad(field, "needs x0 < x1 and y0 < y1");
    }
    Ok(())
}

impl SpeciesTuning {
    fn validate(&self, name: &str, species: Species) -> Result<(), TrogTuningError> {
        let f = |key: &str| format!("{name}.{key}");
        in_range(&f("hp"), u32::from(self.hp), 1, 8)?;
        if !(self.walk_px_tick > 0.0 && self.walk_px_tick <= 4.0) {
            return bad(&f("walk_px_tick"), "must be in (0, 4]");
        }
        in_range(&f("leash_px"), u32::from(self.leash_px), 16, 512)?;
        in_range(&f("tell_ticks"), u32::from(self.tell_ticks), 1, 120)?;
        if self.strike_start < self.tell_ticks {
            return bad(&f("strike_start"), "must not precede the end of the tell");
        }
        if self.strike_end < self.strike_start || self.strike_end > 240 {
            return bad(
                &f("strike_end"),
                "must be at least strike_start and at most 240",
            );
        }
        in_range(&f("recover_ticks"), u32::from(self.recover_ticks), 1, 120)?;
        in_range(&f("rest_ticks"), u32::from(self.rest_ticks), 1, 240)?;
        in_range(&f("hurt_ticks"), u32::from(self.hurt_ticks), 1, 120)?;
        if !(0.0..=600.0).contains(&self.knock_vx) {
            return bad(&f("knock_vx"), "must be 0..=600");
        }
        if !(0.0..=600.0).contains(&self.knock_vy) {
            return bad(&f("knock_vy"), "must be 0..=600");
        }
        for (key, b) in [
            ("hitbox", &self.hitbox),
            ("hurt_idle", &self.hurt_idle),
            ("hurt_tell", &self.hurt_tell),
            ("hurt_active", &self.hurt_active),
            ("hurt_recover", &self.hurt_recover),
            ("hurt_hurt", &self.hurt_hurt),
        ] {
            valid_box(&f(key), b)?;
        }
        match species {
            Species::Pygmy => {
                in_range(&f("leap_min_px"), u32::from(self.leap_min_px), 1, 512)?;
                if self.leap_max_px < self.leap_min_px || self.leap_max_px > 512 {
                    return bad(&f("leap_max_px"), "must be leap_min_px..=512");
                }
                in_range(&f("apex_px"), u32::from(self.apex_px), 1, 64)?;
                in_range(&f("trigger_px"), u32::from(self.trigger_px), 1, 512)?;
            }
            Species::Basher => {
                if self.open_x0 >= self.open_x1
                    || self.open_x0 < self.hitbox.x0
                    || self.open_x1 > self.hitbox.x1
                {
                    return bad(
                        &f("open_x0"),
                        "the open band must lie inside the hitbox x range",
                    );
                }
                in_range(
                    &f("bench_shove_px_s"),
                    u32::from(self.bench_shove_px_s),
                    1,
                    600,
                )?;
                in_range(
                    &f("bench_min_overlap_px"),
                    u32::from(self.bench_min_overlap_px),
                    1,
                    64,
                )?;
                in_range(&f("bait_px"), u32::from(self.bait_px), 1, 512)?;
                in_range(
                    &f("interrupt_until"),
                    u32::from(self.interrupt_until),
                    1,
                    120,
                )?;
                in_range(
                    &f("interrupt_cooldown"),
                    u32::from(self.interrupt_cooldown),
                    1,
                    600,
                )?;
            }
            Species::Virtuoso => {
                in_range(&f("standoff_px"), u32::from(self.standoff_px), 1, 256)?;
            }
        }
        Ok(())
    }

    /// The airborne ticks of the Pygmy's leap (`tell_ticks..=strike_end`).
    pub fn air_ticks(&self) -> u16 {
        self.strike_end + 1 - self.tell_ticks
    }
}

impl TrogTuning {
    /// Parse and validate.
    pub fn from_json(text: &str) -> Result<Self, TrogTuningError> {
        let tuning: Self =
            serde_json::from_str(text).map_err(|err| TrogTuningError::Parse(err.to_string()))?;
        tuning.validate()?;
        Ok(tuning)
    }

    /// Parse the embedded file and require the pinned content hash.
    pub fn pinned() -> Result<Self, TrogTuningError> {
        Self::from_json_pinned(TUNING_JSON, TUNING_HASH)
    }

    /// Parse `text` and require its content hash to equal `expected`.
    pub fn from_json_pinned(text: &str, expected: u64) -> Result<Self, TrogTuningError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(TrogTuningError::HashMismatch { expected, actual });
        }
        Self::from_json(text)
    }

    /// The pinned tuning: one shared static.
    pub fn active() -> &'static Self {
        static ACTIVE: OnceLock<TrogTuning> = OnceLock::new();
        ACTIVE.get_or_init(|| Self::pinned().expect("the embedded trog data is pinned and valid"))
    }

    fn validate(&self) -> Result<(), TrogTuningError> {
        if self.schema_version != 1 {
            return bad("schema_version", "must be 1");
        }
        in_range("trogq01_damage", u32::from(self.trogq01_damage), 1, 4)?;
        in_range(
            "trogq02_tell_gap_ticks",
            u32::from(self.trogq02_tell_gap_ticks),
            0,
            240,
        )?;
        in_range(
            "trogq03_wake_margin_px",
            u32::from(self.trogq03_wake_margin_px),
            0,
            64,
        )?;
        in_range(
            "trogq04_ai_bias_px",
            u32::from(self.trogq04_ai_bias_px),
            0,
            200,
        )?;
        in_range(
            "trogq05_target_switch_px",
            u32::from(self.trogq05_target_switch_px),
            0,
            200,
        )?;
        self.pygmy.validate("pygmy", Species::Pygmy)?;
        self.basher.validate("basher", Species::Basher)?;
        self.virtuoso.validate("virtuoso", Species::Virtuoso)?;
        Ok(())
    }

    pub fn species(&self, species: Species) -> &SpeciesTuning {
        match species {
            Species::Pygmy => &self.pygmy,
            Species::Basher => &self.basher,
            Species::Virtuoso => &self.virtuoso,
        }
    }
}

// ----------------------------------------------------------------- events

/// What woke a troglodyte (TROG-AD21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeCause {
    /// A hero crossed `trigger_x` with the whole hurtbox in view.
    Proximity,
    /// A hit, blast or contact reached it while it slept.
    Hit,
}

/// The sub-events of `CombatEvent::Trog`. Codes are appended and hashed like every combat event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrogAct {
    /// Code 1. A lethal hit on a sleeper logs no `Wake` (TROG-AD29).
    Wake { cause: WakeCause },
    /// Code 2: the tell started and committed; `target` is the hero's stable id.
    Tell { target: HeroId },
    /// Code 3: the active window began (the leap, the downswing, the lash; TROG-AD28).
    Launch,
    /// Code 4: the Pygmy landed.
    Land,
    /// Code 5: the first blow of the attack landed.
    Strike,
    /// Code 6: the active window ended with no blow landed.
    Whiff,
    /// Code 7: a hit cancelled the tell.
    Interrupted,
    /// Code 8 (TROG-B, TROGB-AD1, appended): a Basher's smash reached a standing bench stack and
    /// shoved it. `bait` is the baiting hero's stable id: the hero that met rule (b) at the tell
    /// start, or the committed target of a smash at the far face. Logged once per smash.
    BenchSmash { bait: HeroId },
}

impl TrogAct {
    pub const fn code(self) -> u64 {
        match self {
            Self::Wake { .. } => 1,
            Self::Tell { .. } => 2,
            Self::Launch => 3,
            Self::Land => 4,
            Self::Strike => 5,
            Self::Whiff => 6,
            Self::Interrupted => 7,
            Self::BenchSmash { .. } => 8,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Wake { .. } => "wake",
            Self::Tell { .. } => "tell",
            Self::Launch => "launch",
            Self::Land => "land",
            Self::Strike => "strike",
            Self::Whiff => "whiff",
            Self::Interrupted => "interrupted",
            Self::BenchSmash { .. } => "bench_smash",
        }
    }

    /// The payload as `key=value` text (empty without one).
    pub fn fields(self) -> String {
        match self {
            Self::Wake { cause } => format!(
                "cause={}",
                match cause {
                    WakeCause::Proximity => "proximity",
                    WakeCause::Hit => "hit",
                }
            ),
            Self::Tell { target } => format!("target={}", target.index()),
            Self::BenchSmash { bait } => format!("bait={}", bait.index()),
            _ => String::new(),
        }
    }

    pub fn mix_into(self, mix: &mut dyn FnMut(u64)) {
        mix(self.code());
        match self {
            Self::Wake { cause } => mix(cause as u64),
            Self::Tell { target } => mix(target.index() as u64),
            Self::BenchSmash { bait } => mix(bait.index() as u64),
            _ => {}
        }
    }
}

// ------------------------------------------------------------------ state

/// The machine's states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Dormant,
    Approach,
    Tell,
    Active,
    Recover,
    Rest,
    Hurt,
    Down,
}

impl State {
    const fn code(self) -> u64 {
        match self {
            Self::Dormant => 1,
            Self::Approach => 2,
            Self::Tell => 3,
            Self::Active => 4,
            Self::Recover => 5,
            Self::Rest => 6,
            Self::Hurt => 7,
            Self::Down => 8,
        }
    }
}

/// What kind of hit landed (the slot's `HitSource` without the move id).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitKind {
    /// Any melee move of Carl.
    Melee,
    /// Donut's swat (a glance, damage 0).
    Swat,
    /// Donut's pounce.
    Pounce,
    /// Donut's Magic Missile.
    Missile,
    /// Carl's dynamite: an interrupt for every species (TROG-AD15).
    Blast,
    /// A launched body: no damage, flinches the Pygmy only.
    Contact,
}

/// What a hit did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitOutcome {
    pub hp_left: u16,
    pub defeated: bool,
}

/// The committed geometry of the current attack (fixed on the tell's first tick).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Commit {
    /// Pygmy: where the leap started and where it lands.
    start_x: f32,
    landing_x: f32,
    /// Basher: the smash rectangle.
    rect: Option<Rect>,
}

/// What the scene tells a troglodyte each tick.
#[derive(Debug, Clone, Copy)]
pub struct TrogCtx<'a> {
    pub tick: u64,
    pub heroes: &'a [HeroView; 2],
    pub solids: &'a [Block],
    /// Solo: the bot hero counts `trogq04_ai_bias_px` farther away.
    pub solo: bool,
    /// The simulation camera's horizontal view `[left, right]`.
    pub view_x: (f32, f32),
    /// The attack token pool has a free token and the tell spacing allows a tell now.
    pub token_free: bool,
    /// The tick of the last tell start of the scene (any enemy).
    pub last_tell_start: Option<u64>,
    /// TROG-B (TROGB-AD1): the standing breakable solid props of the room (the bench stack), as
    /// world rectangles. Empty in a room with none.
    pub faces: &'a [BenchFace],
}

/// A standing breakable prop a Basher's smash may shove (the bench stack, TROG-B).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BenchFace {
    /// The prop's index in the encounter.
    pub prop: u16,
    /// Its world rectangle (y up).
    pub rect: Rect,
}

/// A shove the Basher's smash sends to a bench prop on its impact tick (once per smash).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BenchShove {
    pub prop: u16,
    /// Shove speed at the contact (px/s).
    pub impulse_px_s: u16,
    /// The hero named by `BenchSmash` (the baiting hero).
    pub bait: HeroId,
    /// The player credited: the bait's player, `None` when the bait is the companion bot
    /// (TROG-AD26).
    pub credit: Option<u32>,
}

/// What one step asks the scene to do.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrogStep {
    pub acts: Vec<TrogAct>,
    pub blows: Vec<HeroBlow>,
    /// The enemy began a tell on this tick and took an attack token.
    pub tell_started: bool,
    /// TROG-B: the shove of a smash that met a bench stack on this tick.
    pub shoves: Vec<BenchShove>,
}

/// One troglodyte.
#[derive(Debug, Clone)]
pub struct Trog {
    pub species: Species,
    index: u8,
    pub x: f32,
    pub foot_y: f32,
    /// +1 / -1. A tell fixes it.
    pub facing: i8,
    pub state: State,
    pub hp: u16,
    spawn_x: f32,
    leash: (f32, f32),
    /// Protected spawn x range: no hero in it is a target.
    protected: (f32, f32),
    trigger_x: Option<f32>,
    delay_ticks: u32,
    wake: Option<WakeCause>,
    /// The hero it follows, and the committed target while it attacks.
    target: Option<HeroId>,
    pub(crate) attack_tick: u16,
    /// The scene tick the current tell started (the evade stamp).
    tell_start_tick: u64,
    commit: Commit,
    /// Heroes this attack instance resolved (blow landed or evaded).
    hit_mask: u8,
    struck: bool,
    /// TROG-B: this smash instance already shoved a bench (at most one shove per smash).
    bench_sent: bool,
    rest_left: u16,
    hurt_left: u16,
    /// The Basher's interrupt budget: the tick of the last interrupt or flinch.
    last_react: Option<u64>,
    /// The Pygmy's height above the walking line this tick.
    pub(crate) lift: f32,
    /// The Pygmy's leap met a solid and holds its x until it lands.
    leap_blocked: bool,
}

/// The airborne height of the Pygmy at leap tick `k` of `n` (TROG-AD25): the integer parabola
/// `round(4 * apex * k * (n - k) / n^2)`.
pub fn leap_height(apex_px: u16, k: u16, n: u16) -> f32 {
    let (apex, k, n) = (u32::from(apex_px), u32::from(k), u32::from(n));
    if k >= n {
        return 0.0;
    }
    let num = 4 * apex * k * (n - k);
    let den = n * n;
    ((2 * num + den) / (2 * den)) as f32
}

fn rect_of(b: &BoxPx, x: f32, facing: i8, feet: f32) -> Rect {
    world_box(b, x, facing, feet)
}

/// Where and how a troglodyte spawns (one roster entry).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spawn {
    pub species: Species,
    /// The roster index (the blow source and the target id carry it).
    pub index: u8,
    pub x: f32,
    /// World y of the walking line.
    pub floor_y: f32,
    pub trigger_x: Option<f32>,
    pub delay_ticks: u32,
    /// The protected spawn's x range: no hero in it is a target.
    pub protected: (f32, f32),
}

impl Trog {
    /// A sleeper at the spawn, facing the way the heroes come from.
    pub fn new(t: &TrogTuning, s: &Spawn) -> Self {
        let Spawn {
            species,
            index,
            x,
            floor_y,
            trigger_x,
            delay_ticks,
            protected,
        } = *s;
        let sp = t.species(species);
        let leash = f32::from(sp.leash_px);
        Self {
            species,
            index,
            x,
            foot_y: floor_y,
            facing: -1,
            state: State::Dormant,
            hp: sp.hp,
            spawn_x: x,
            leash: (x - leash, x + leash),
            protected,
            trigger_x,
            delay_ticks,
            wake: None,
            target: None,
            attack_tick: 0,
            tell_start_tick: 0,
            commit: Commit {
                start_x: x,
                landing_x: x,
                rect: None,
            },
            hit_mask: 0,
            struck: false,
            bench_sent: false,
            rest_left: 0,
            hurt_left: 0,
            last_react: None,
            lift: 0.0,
            leap_blocked: false,
        }
    }

    pub fn alive(&self) -> bool {
        self.state != State::Down
    }

    pub const fn index(&self) -> u8 {
        self.index
    }

    pub const fn spawn_x(&self) -> f32 {
        self.spawn_x
    }

    /// The leash `[left, right]` of the pivot.
    pub const fn leash(&self) -> (f32, f32) {
        self.leash
    }

    pub const fn target(&self) -> Option<HeroId> {
        self.target
    }

    pub const fn wake_cause(&self) -> Option<WakeCause> {
        self.wake
    }

    /// The attack tick (0 on the tell's first tick); meaningful in Tell, Active and Recover.
    pub const fn attack_tick(&self) -> u16 {
        self.attack_tick
    }

    pub const fn tell_start_tick(&self) -> u64 {
        self.tell_start_tick
    }

    /// Hurt ticks left (a flinch).
    pub const fn hurt_left(&self) -> u16 {
        self.hurt_left
    }

    /// The Pygmy's committed landing x.
    pub const fn landing_x(&self) -> f32 {
        self.commit.landing_x
    }

    /// The Basher's committed smash rectangle.
    pub const fn smash_rect(&self) -> Option<Rect> {
        self.commit.rect
    }

    /// The attack holds a token from the tell's first tick to the end of the active window.
    pub fn holds_token(&self) -> bool {
        matches!(self.state, State::Tell | State::Active)
    }

    /// Height above the walking line (the Pygmy's leap).
    pub const fn lift(&self) -> f32 {
        self.lift
    }

    // ------------------------------------------------------------ boxes

    /// The hurtbox of the current state; `None` when it has none (down).
    pub fn hurtbox(&self, t: &TrogTuning) -> Option<Rect> {
        let sp = t.species(self.species);
        let b = match self.state {
            State::Down => return None,
            State::Dormant | State::Approach | State::Rest => &sp.hurt_idle,
            State::Tell => &sp.hurt_tell,
            State::Active => &sp.hurt_active,
            State::Recover => &sp.hurt_recover,
            State::Hurt => &sp.hurt_hurt,
        };
        Some(rect_of(b, self.x, self.facing, self.foot_y + self.lift))
    }

    /// The idle hurtbox at `x` (the walk body and the view rule).
    fn idle_box_at(&self, t: &TrogTuning, x: f32) -> Rect {
        rect_of(
            &t.species(self.species).hurt_idle,
            x,
            self.facing,
            self.foot_y,
        )
    }

    /// The attack hitbox live on this tick, if any: the Pygmy's from its airborne pivot, the
    /// Basher's committed rectangle, the Virtuoso's band clipped at the first standing solid.
    pub fn live_hitbox(&self, t: &TrogTuning, solids: &[Block]) -> Option<Rect> {
        let sp = t.species(self.species);
        if self.state != State::Active
            || self.attack_tick < sp.strike_start
            || self.attack_tick > sp.strike_end
        {
            return None;
        }
        self.hitbox_at(t, solids)
    }

    fn hitbox_at(&self, t: &TrogTuning, solids: &[Block]) -> Option<Rect> {
        let sp = t.species(self.species);
        match self.species {
            Species::Pygmy => Some(rect_of(
                &sp.hitbox,
                self.x,
                self.facing,
                self.foot_y + self.lift,
            )),
            Species::Basher => self.commit.rect,
            Species::Virtuoso => clip_band(
                rect_of(&sp.hitbox, self.x, self.facing, self.foot_y),
                self.x,
                self.facing,
                solids,
            ),
        }
    }

    /// The hitbox of the current attack and the attack ticks until it is live (0 while live),
    /// for the companion's threat feed. `None` outside Tell and Active.
    pub fn threat(&self, t: &TrogTuning, solids: &[Block]) -> Option<(Rect, u16)> {
        let sp = t.species(self.species);
        if !matches!(self.state, State::Tell | State::Active) {
            return None;
        }
        let left = sp.strike_start.saturating_sub(self.attack_tick);
        let rect = match self.species {
            Species::Pygmy => {
                // The x-union of every strike-tick box along the committed arc, from the
                // take-off to the landing (the report covers the whole sweep, not only the
                // landing box), at the apex's height.
                let n = sp.air_ticks();
                let (start, landing) = (self.commit.start_x, self.commit.landing_x);
                let lift = self.foot_y + f32::from(sp.apex_px);
                let mut union: Option<Rect> = None;
                for a in sp.strike_start..=sp.strike_end {
                    let k = a - sp.tell_ticks;
                    let px = start + (landing - start) * f32::from(k) / f32::from(n);
                    let r = rect_of(&sp.hitbox, px, self.facing, lift);
                    union = Some(match union {
                        None => r,
                        Some(u) => {
                            let (left, right) = (u.left.min(r.left), u.right().max(r.right()));
                            Rect {
                                left,
                                bottom: u.bottom,
                                width: right - left,
                                height: u.height,
                            }
                        }
                    });
                }
                union?
            }
            Species::Basher => self.commit.rect?,
            Species::Virtuoso => clip_band(
                rect_of(&sp.hitbox, self.x, self.facing, self.foot_y),
                self.x,
                self.facing,
                solids,
            )?,
        };
        Some((rect, left))
    }

    // ------------------------------------------------------------- hits

    /// A landed hit. `None`: it takes no hit now (down).
    pub fn hit(
        &mut self,
        t: &TrogTuning,
        tick: u64,
        kind: HitKind,
        damage: u16,
        out: &mut TrogStep,
    ) -> Option<HitOutcome> {
        if self.state == State::Down {
            return None;
        }
        let sp = t.species(self.species);
        self.hp = self.hp.saturating_sub(damage);
        if self.hp == 0 {
            // TROG-AD29: a lethal hit logs no Wake, whatever the state was.
            self.go_down();
            return Some(HitOutcome {
                hp_left: 0,
                defeated: true,
            });
        }
        let outcome = HitOutcome {
            hp_left: self.hp,
            defeated: false,
        };
        // The reaction by species, state and hit kind (plan 5.2 table).
        let budget_ok = self
            .last_react
            .is_none_or(|l| tick.saturating_sub(l) >= u64::from(sp.interrupt_cooldown));
        let melee_or_pounce = matches!(kind, HitKind::Melee | HitKind::Pounce);
        let blast = kind == HitKind::Blast;
        match self.state {
            State::Dormant => {
                self.wake = Some(WakeCause::Hit);
                out.acts.push(TrogAct::Wake {
                    cause: WakeCause::Hit,
                });
                self.state = State::Rest;
                self.rest_left = sp.rest_ticks;
            }
            State::Tell => {
                let interrupts = blast
                    || match self.species {
                        Species::Pygmy => true,
                        Species::Basher => {
                            melee_or_pounce && self.attack_tick < sp.interrupt_until && budget_ok
                        }
                        Species::Virtuoso => melee_or_pounce,
                    };
                if interrupts {
                    out.acts.push(TrogAct::Interrupted);
                    self.flinch(sp, tick);
                }
            }
            State::Approach | State::Rest => {
                let flinches = blast
                    || match self.species {
                        Species::Pygmy | Species::Virtuoso => true,
                        Species::Basher => melee_or_pounce && budget_ok,
                    };
                // A contact flinches the Pygmy only.
                let flinches =
                    flinches && (kind != HitKind::Contact || self.species == Species::Pygmy);
                if flinches {
                    self.flinch(sp, tick);
                }
            }
            State::Active | State::Recover | State::Hurt | State::Down => {}
        }
        Some(outcome)
    }

    fn flinch(&mut self, sp: &SpeciesTuning, tick: u64) {
        self.state = State::Hurt;
        self.hurt_left = sp.hurt_ticks;
        self.attack_tick = 0;
        self.lift = 0.0;
        self.last_react = Some(tick);
    }

    fn go_down(&mut self) {
        self.state = State::Down;
        self.attack_tick = 0;
        self.lift = 0.0;
        self.hurt_left = 0;
        self.rest_left = 0;
    }

    // ------------------------------------------------------------- step

    /// One scene tick. `evade` asks Donut's reflex whether this attack instance is evaded for
    /// `hero` (it is only called for a hero a blow would land on).
    pub fn step(
        &mut self,
        t: &TrogTuning,
        ctx: &TrogCtx<'_>,
        evade: &mut impl FnMut(HeroId) -> bool,
        out: &mut TrogStep,
    ) {
        match self.state {
            State::Down => {}
            State::Dormant => self.step_dormant(t, ctx, out),
            State::Approach => self.step_approach(t, ctx, out),
            State::Rest => {
                self.rest_left = self.rest_left.saturating_sub(1);
                if self.rest_left == 0 {
                    self.state = State::Approach;
                }
            }
            State::Hurt => {
                self.hurt_left = self.hurt_left.saturating_sub(1);
                if self.hurt_left == 0 {
                    self.state = State::Rest;
                    self.rest_left = t.species(self.species).rest_ticks;
                }
            }
            State::Tell | State::Active | State::Recover => self.step_attack(t, ctx, evade, out),
        }
    }

    fn in_view(&self, t: &TrogTuning, ctx: &TrogCtx<'_>) -> bool {
        let body = self.idle_box_at(t, self.x);
        let margin = f32::from(t.trogq03_wake_margin_px);
        body.left >= ctx.view_x.0 + margin && body.right() <= ctx.view_x.1 - margin
    }

    fn step_dormant(&mut self, t: &TrogTuning, ctx: &TrogCtx<'_>, out: &mut TrogStep) {
        let crossed = self
            .trigger_x
            .is_some_and(|tx| ctx.heroes.iter().any(|h| h.present && h.x >= tx));
        if crossed && self.in_view(t, ctx) {
            self.wake = Some(WakeCause::Proximity);
            out.acts.push(TrogAct::Wake {
                cause: WakeCause::Proximity,
            });
            if self.delay_ticks > 0 {
                self.state = State::Rest;
                self.rest_left = u16::try_from(self.delay_ticks).unwrap_or(u16::MAX);
            } else {
                self.state = State::Approach;
            }
        }
    }

    fn is_protected(&self, h: &HeroView) -> bool {
        h.x >= self.protected.0 && h.x <= self.protected.1
    }

    /// Whether `h` can be struck at all: its hurtbox can meet the attack band in y (TROG-AD17)
    /// and lies inside the leash's reach in x (TROG-AD31).
    pub fn is_candidate(&self, t: &TrogTuning, h: &HeroView) -> bool {
        let sp = t.species(self.species);
        let (lo, hi) = (
            self.foot_y + f32::from(sp.hitbox.y0),
            self.foot_y + f32::from(sp.hitbox.y1),
        );
        let band_y = h.hurt.bottom < hi && h.hurt.top() > lo;
        let reach = f32::from(sp.hitbox.x0.abs().max(sp.hitbox.x1.abs()));
        let reach_x = h.hurt.right() > self.leash.0 - reach && h.hurt.left < self.leash.1 + reach;
        band_y && reach_x
    }

    fn choose_target(&mut self, t: &TrogTuning, ctx: &TrogCtx<'_>) {
        let alive: Vec<&HeroView> = ctx
            .heroes
            .iter()
            .filter(|h| h.present && !self.is_protected(h))
            .collect();
        let cands: Vec<&HeroView> = alive
            .iter()
            .copied()
            .filter(|h| self.is_candidate(t, h))
            .collect();
        // TROG-AD30: with no candidate it follows the nearest hero outside the protected spawn
        // and never tells at it; with no such hero it holds.
        let pool = if cands.is_empty() { alive } else { cands };
        let bias = f32::from(t.trogq04_ai_bias_px);
        let effective =
            |h: &HeroView| (h.x - self.x).abs() + if ctx.solo && h.bot { bias } else { 0.0 };
        // Ties go to Donut (the smaller hurtbox).
        let nearest = pool.iter().copied().min_by(|a, b| {
            effective(a)
                .total_cmp(&effective(b))
                .then(b.id.index().cmp(&a.id.index()))
        });
        let held = self
            .target
            .and_then(|id| pool.iter().copied().find(|h| h.id == id));
        let switch = f32::from(t.trogq05_target_switch_px);
        self.target = match (held, nearest) {
            (Some(kept), Some(best)) if best.id != kept.id => {
                if effective(best) + switch <= effective(kept) {
                    Some(best.id)
                } else {
                    Some(kept.id)
                }
            }
            (Some(kept), _) => Some(kept.id),
            (None, best) => best.map(|h| h.id),
        };
    }

    fn gap_ok(&self, t: &TrogTuning, ctx: &TrogCtx<'_>) -> bool {
        ctx.last_tell_start
            .is_none_or(|s| ctx.tick.saturating_sub(s) >= u64::from(t.trogq02_tell_gap_ticks))
    }

    fn step_approach(&mut self, t: &TrogTuning, ctx: &TrogCtx<'_>, out: &mut TrogStep) {
        self.choose_target(t, ctx);
        let Some(id) = self.target else {
            return;
        };
        let hero = ctx.heroes[id.index()];
        let dx = hero.x - self.x;
        if dx > 0.0 {
            self.facing = 1;
        } else if dx < 0.0 {
            self.facing = -1;
        }
        if self.species == Species::Basher
            && self.in_view(t, ctx)
            && ctx.token_free
            && self.gap_ok(t, ctx)
            && let Some(bait) = self.bench_bait(t, ctx)
        {
            // Rule (b), the bench smash: the tell is the ordinary smash, aimed at the bench.
            self.facing = if bait.x > self.x { 1 } else { -1 };
            self.begin_tell(t, ctx, &bait, out);
            return;
        }
        if self.is_candidate(t, &hero)
            && self.in_view(t, ctx)
            && ctx.token_free
            && self.gap_ok(t, ctx)
            && self.eligible(t, ctx, &hero)
        {
            self.begin_tell(t, ctx, &hero, out);
            return;
        }
        self.walk(t, ctx, &hero);
    }

    /// Rule (b) of the bench smash (plan 5.4, TROGB-AD1): the Basher's walk toward its target is
    /// blocked within 4 px by a standing bench face, and a living hero stands on the room floor
    /// on the face's far side (away from the Basher) within `bait_px` of the pivot. Returns the
    /// baiting hero: the nearest to the pivot, ties to Donut. The committed smash rectangle must
    /// overlap the face by `bench_min_overlap_px` (the truth rule, so the tell is never a lie).
    fn bench_bait(&self, t: &TrogTuning, ctx: &TrogCtx<'_>) -> Option<HeroView> {
        let sp = t.species(self.species);
        let dir: f32 = if self.facing >= 0 { 1.0 } else { -1.0 };
        let body = self.idle_box_at(t, self.x);
        let smash = rect_of(&sp.hitbox, self.x, self.facing, self.foot_y);
        let face = ctx.faces.iter().find(|f| {
            let y_ok = f.rect.bottom < body.top() && f.rect.top() > body.bottom;
            let gap = if dir < 0.0 {
                body.left - f.rect.right()
            } else {
                f.rect.left - body.right()
            };
            let overlap = smash.right().min(f.rect.right()) - smash.left.max(f.rect.left);
            y_ok && (-0.5..=4.0).contains(&gap)
                && overlap >= f32::from(sp.bench_min_overlap_px)
                && smash.bottom < f.rect.top()
                && smash.top() > f.rect.bottom
        })?;
        let far = |h: &HeroView| {
            h.present
                && (h.foot_y - self.foot_y).abs() <= 2.0
                && if dir < 0.0 {
                    h.x < face.rect.left
                } else {
                    h.x > face.rect.right()
                }
                && (h.x - self.x).abs() <= f32::from(sp.bait_px)
        };
        ctx.heroes
            .iter()
            .filter(|h| far(h))
            .min_by(|a, b| {
                (a.x - self.x)
                    .abs()
                    .total_cmp(&(b.x - self.x).abs())
                    .then(b.id.index().cmp(&a.id.index()))
            })
            .copied()
    }

    /// The truth rule: the committed geometry can overlap the target's current hurtbox, and
    /// the species' range holds.
    fn eligible(&self, t: &TrogTuning, ctx: &TrogCtx<'_>, h: &HeroView) -> bool {
        let sp = t.species(self.species);
        match self.species {
            Species::Pygmy => {
                if (h.x - self.x).abs() > f32::from(sp.trigger_px) {
                    return false;
                }
                let landing = self.pygmy_landing(sp, h.x);
                let n = sp.air_ticks();
                (sp.strike_start..=sp.strike_end).any(|a| {
                    let k = a - sp.tell_ticks;
                    let px = self.x + (landing - self.x) * f32::from(k) / f32::from(n);
                    let lift = leap_height(sp.apex_px, k, n);
                    rect_of(&sp.hitbox, px, self.facing, self.foot_y + lift).overlaps(h.hurt)
                })
            }
            Species::Basher => {
                let open = BoxPx {
                    x0: sp.open_x0,
                    x1: sp.open_x1,
                    y0: sp.hitbox.y0,
                    y1: sp.hitbox.y1,
                };
                rect_of(&open, self.x, self.facing, self.foot_y).overlaps(h.hurt)
            }
            Species::Virtuoso => clip_band(
                rect_of(&sp.hitbox, self.x, self.facing, self.foot_y),
                self.x,
                self.facing,
                ctx.solids,
            )
            .is_some_and(|band| band.overlaps(h.hurt)),
        }
    }

    fn pygmy_landing(&self, sp: &SpeciesTuning, target_x: f32) -> f32 {
        let d = (target_x - self.x)
            .abs()
            .clamp(f32::from(sp.leap_min_px), f32::from(sp.leap_max_px));
        (self.x + f32::from(self.facing) * d).clamp(self.leash.0, self.leash.1)
    }

    fn begin_tell(&mut self, t: &TrogTuning, ctx: &TrogCtx<'_>, h: &HeroView, out: &mut TrogStep) {
        let sp = t.species(self.species);
        self.state = State::Tell;
        self.attack_tick = 0;
        self.tell_start_tick = ctx.tick;
        self.hit_mask = 0;
        self.struck = false;
        self.bench_sent = false;
        self.leap_blocked = false;
        self.lift = 0.0;
        self.commit = Commit {
            start_x: self.x,
            landing_x: self.x,
            rect: None,
        };
        match self.species {
            Species::Pygmy => self.commit.landing_x = self.pygmy_landing(sp, h.x),
            Species::Basher => {
                self.commit.rect = Some(rect_of(&sp.hitbox, self.x, self.facing, self.foot_y));
            }
            Species::Virtuoso => {}
        }
        self.target = Some(h.id);
        out.acts.push(TrogAct::Tell { target: h.id });
        out.tell_started = true;
    }

    /// Whether a standing solid blocks the walk body at `next` while moving `dir`. A solid at most
    /// [`LOW_STEP_PX`] high (the broken bench's remnant, TROGB-AD2) is stepped over: it blocks no
    /// walk, as V2 already treats such a solid as no obstacle in a leash band.
    fn walk_blocked(&self, t: &TrogTuning, solids: &[Block], next: f32, dir: f32) -> bool {
        let body = self.idle_box_at(t, next);
        solids.iter().any(|s| {
            s.intact && s.rect.height > LOW_STEP_PX && body.overlaps(s.rect) && {
                let centre = s.rect.left + s.rect.width * 0.5;
                (centre - self.x) * dir > 0.0
            }
        })
    }

    fn walk(&mut self, t: &TrogTuning, ctx: &TrogCtx<'_>, h: &HeroView) {
        let sp = t.species(self.species);
        let dx = h.x - self.x;
        let dist = dx.abs();
        let toward = if dx > 0.0 { 1.0 } else { -1.0 };
        // The Pygmy only closes in; the others hold a distance, backing off when a hero is
        // inside it (TROG-AD32).
        let hold = match self.species {
            Species::Pygmy => None,
            Species::Basher => Some(f32::from(sp.open_x0 + sp.open_x1) * 0.5),
            Species::Virtuoso => Some(f32::from(sp.standoff_px)),
        };
        let dir = match hold {
            None if dist > 0.5 => toward,
            None => 0.0,
            Some(d) if dist > d + 0.5 => toward,
            Some(d) if dist < d - 0.5 => -toward,
            Some(_) => 0.0,
        };
        if dir == 0.0 {
            return;
        }
        let next = (self.x + dir * sp.walk_px_tick).clamp(self.leash.0, self.leash.1);
        if next == self.x || self.walk_blocked(t, ctx.solids, next, dir) {
            return;
        }
        self.x = next;
    }

    fn step_attack(
        &mut self,
        t: &TrogTuning,
        ctx: &TrogCtx<'_>,
        evade: &mut impl FnMut(HeroId) -> bool,
        out: &mut TrogStep,
    ) {
        let sp = t.species(self.species);
        self.attack_tick += 1;
        let a = self.attack_tick;
        if a == sp.tell_ticks {
            self.state = State::Active;
            out.acts.push(TrogAct::Launch);
        }
        // The Pygmy's leap: constant horizontal speed, the integer parabola, stepped against
        // solids (it stops at the face and holds its x until it lands).
        if self.species == Species::Pygmy && a >= sp.tell_ticks && a <= sp.strike_end + 1 {
            let n = sp.air_ticks();
            let k = a - sp.tell_ticks;
            self.lift = leap_height(sp.apex_px, k, n);
            if !self.leap_blocked {
                let next = self.commit.start_x
                    + (self.commit.landing_x - self.commit.start_x) * f32::from(k) / f32::from(n);
                let dir = if next >= self.x { 1.0 } else { -1.0 };
                if next != self.x && self.walk_blocked(t, ctx.solids, next, dir) {
                    self.leap_blocked = true;
                } else {
                    self.x = next;
                }
            }
        }
        if a > sp.strike_end {
            if self.state != State::Recover {
                self.state = State::Recover;
                self.lift = 0.0;
                if self.species == Species::Pygmy {
                    out.acts.push(TrogAct::Land);
                }
                if !self.struck {
                    out.acts.push(TrogAct::Whiff);
                }
            }
            if a > sp.strike_end + sp.recover_ticks {
                self.state = State::Rest;
                self.rest_left = sp.rest_ticks;
                self.attack_tick = 0;
            }
            return;
        }
        if a >= sp.strike_start {
            self.strike(t, ctx, evade, out);
        }
    }

    fn strike(
        &mut self,
        t: &TrogTuning,
        ctx: &TrogCtx<'_>,
        evade: &mut impl FnMut(HeroId) -> bool,
        out: &mut TrogStep,
    ) {
        let sp = t.species(self.species);
        let Some(hitbox) = self.hitbox_at(t, ctx.solids) else {
            return;
        };
        self.bench_shove(t, ctx, &hitbox, out);
        for h in ctx.heroes.iter() {
            let bit = 1u8 << h.id.index();
            if !h.present || !h.vulnerable || self.hit_mask & bit != 0 || !hitbox.overlaps(h.hurt) {
                continue;
            }
            self.hit_mask |= bit;
            if h.id == HeroId::Donut && evade(h.id) {
                continue;
            }
            if !self.struck {
                self.struck = true;
                out.acts.push(TrogAct::Strike);
            }
            out.blows.push(HeroBlow {
                hero: h.id,
                source: trog_source(self.index),
                damage: t.trogq01_damage,
                from_x: self.x,
                knock_vx: signed_away(h.x, self.x, sp.knock_vx),
                knock_vy: sp.knock_vy,
            });
        }
    }

    /// TROG-B (plan 5.4): the smash's rectangle overlapping a standing bench face by
    /// `bench_min_overlap_px` on an impact tick shoves it, once per smash instance; the
    /// `BenchSmash` event names the baiting hero (the committed target).
    fn bench_shove(
        &mut self,
        t: &TrogTuning,
        ctx: &TrogCtx<'_>,
        hitbox: &Rect,
        out: &mut TrogStep,
    ) {
        if self.species != Species::Basher || self.bench_sent {
            return;
        }
        let sp = t.species(self.species);
        let min = f32::from(sp.bench_min_overlap_px);
        let Some(face) = ctx.faces.iter().find(|f| {
            hitbox.bottom < f.rect.top()
                && hitbox.top() > f.rect.bottom
                && hitbox.right().min(f.rect.right()) - hitbox.left.max(f.rect.left) >= min
        }) else {
            return;
        };
        let Some(bait) = self.target else {
            return;
        };
        self.bench_sent = true;
        out.shoves.push(BenchShove {
            prop: face.prop,
            impulse_px_s: sp.bench_shove_px_s,
            bait,
            credit: (!ctx.heroes[bait.index()].bot).then(|| bait.index() as u32),
        });
        out.acts.push(TrogAct::BenchSmash { bait });
    }

    // ------------------------------------------------------------- hash

    /// Fold the whole state into a state hash (everything the simulation reads).
    pub fn mix_into(&self, mix: &mut dyn FnMut(u64)) {
        mix(0x5452_4f47_0001); // "TROG"
        mix(self.species.code());
        mix(u64::from(self.index));
        mix(self.state.code());
        mix(u64::from(self.hp));
        mix(self.facing as i64 as u64);
        mix(u64::from(self.x.to_bits()));
        mix(u64::from(self.foot_y.to_bits()));
        mix(u64::from(self.lift.to_bits()));
        mix(u64::from(self.leash.0.to_bits()));
        mix(u64::from(self.leash.1.to_bits()));
        mix(self.wake.map_or(0, |c| c as u64 + 1));
        mix(self.target.map_or(0, |h| h.index() as u64 + 1));
        mix(u64::from(self.attack_tick));
        mix(u64::from(self.rest_left));
        mix(u64::from(self.hurt_left));
        mix(self.tell_start_tick);
        mix(u64::from(self.commit.start_x.to_bits()));
        mix(u64::from(self.commit.landing_x.to_bits()));
        match self.commit.rect {
            Some(r) => {
                mix(1);
                for v in [r.left, r.bottom, r.width, r.height] {
                    mix(u64::from(v.to_bits()));
                }
            }
            None => mix(0),
        }
        mix(u64::from(self.hit_mask));
        mix(u64::from(self.struck));
        mix(u64::from(self.leap_blocked));
        mix(self.last_react.map_or(0, |l| l + 1));
        // TROG-B: appended only once a smash shoved a bench, so a room without one hashes as
        // before.
        if self.bench_sent {
            mix(0x4245_4e43);
        }
    }
}

/// The band `band` of an attack from `pivot_x` facing `facing`, clipped at the first standing
/// solid ahead of the pivot that meets the band's height; `None` when nothing is left.
pub fn clip_band(band: Rect, pivot_x: f32, facing: i8, solids: &[Block]) -> Option<Rect> {
    let mut left = band.left;
    let mut right = band.right();
    for s in solids.iter().filter(|s| s.intact) {
        let r = s.rect;
        if !(r.bottom < band.top() && r.top() > band.bottom) {
            continue;
        }
        if facing >= 0 && r.right() > pivot_x {
            right = right.min(r.left.max(pivot_x));
        } else if facing < 0 && r.left < pivot_x {
            left = left.max(r.right().min(pivot_x));
        }
    }
    (right > left).then_some(Rect {
        left,
        bottom: band.bottom,
        width: right - left,
        height: band.height,
    })
}

#[cfg(test)]
mod tests;
