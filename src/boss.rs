//! The Floor 01 boss encounter `brute_boss_f01` (HUD-DEBT-03): the domain.
//!
//! Bevy-free and deterministic at the 60 Hz fixed tick: rule-based, no random
//! draw, no wall-clock time. The design is `docs/02-art/floor-01-boss-design.md`;
//! every number lives in `assets/data/bosses/brute_boss_f01.json` ([`BossData`],
//! validated on load, unknown fields rejected). The engine code never names the
//! fan-concept boss: the data id is `brute_boss_f01`, the display name is data.
//!
//! The scene layer ([`crate::combat_scene`]) owns one [`Boss`] only in a boss
//! fixture or the boss scene, so no earlier route or fixture changes. It builds
//! a [`BossCtx`] (the heroes as plain values) every tick, calls [`Boss::step`]
//! after the heroes' strikes and the scene step, and applies the returned
//! [`BossOut`]: events to log, blows to land on heroes, positions to correct.
//!
//! Geometry: x is world pixels (the arena is one 640 px screen), the boss
//! pivot is the feet centre, boxes are pivot-relative with x forward (mirrored
//! by the facing) and y up from the walking line.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::combat::{CombatEvent, solid_at};
use crate::party::HeroId;
use crate::prototype::{Block, HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Rect};

/// The boss' target id in [`CombatEvent::EnemyDefeated`].
pub const BOSS_ENEMY_ID: u32 = 0x4255;
/// The boss' source id in [`CombatEvent::HeroHurt`].
pub const BOSS_SOURCE_ID: u32 = 0x4255;
/// The rack's source id in [`CombatEvent::HeroHurt`] (the falling stack).
pub const RACK_SOURCE_ID: u32 = 0x5241;
const BUILTIN_JSON: &str = include_str!("../assets/data/bosses/brute_boss_f01.json");
const TICK: f32 = 1.0 / 60.0;

// -------------------------------------------------------------------- data --

/// A forward box: `x` ahead of the pivot, `y` above the walking line.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoxData {
    pub x: [f32; 2],
    pub y: [f32; 2],
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoundData {
    pub tell: u16,
    pub smear: u16,
    pub impact: u16,
    pub recover: u16,
    pub damage: u8,
    pub range_px: [f32; 2],
    #[serde(rename = "box")]
    pub hit_box: BoxData,
    pub launch_px_s: [f32; 2],
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShockwaveData {
    pub speed: f32,
    pub reach: f32,
    pub height: f32,
    pub width: f32,
    pub origin_px: f32,
    pub damage: u8,
    pub from_phase: u8,
    pub hop_px_s: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChargeData {
    pub tell_by_phase: [u16; 3],
    pub speed_px_s: f32,
    #[serde(rename = "box")]
    pub ram_box: BoxData,
    pub damage: u8,
    pub launch: [f32; 2],
    pub min_target_px: f32,
    pub min_room_px: f32,
    pub cooldown_by_phase: [u16; 3],
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrashData {
    pub stun_by_phase: [u16; 3],
    pub rebound_px: f32,
    pub ticks: u16,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoublePoundData {
    pub step_ticks: u16,
    pub step_px_per_tick: f32,
    pub second_tell: u16,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoarData {
    pub ticks: u16,
    pub push_radius: f32,
    pub push_px: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlinchData {
    pub ticks: u16,
    pub backstep_px: f32,
    pub backstep_ticks: u16,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyData {
    pub standing: BoxData,
    pub committed: BoxData,
    pub stun: BoxData,
    pub pinned: BoxData,
    pub push_px_per_tick: f32,
    pub min_side_room_px: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeftRackData {
    pub enabled: bool,
    pub strut: BoxData,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RightRackData {
    pub enabled: bool,
    pub ledge_x: [f32; 2],
    pub ledge_height_px: f32,
    pub dwell_ticks: u16,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RackData {
    pub fall_ticks: u16,
    pub boss_damage: u16,
    pub pin_ticks: u16,
    pub hero_damage: u8,
    pub hero_hop_px_s: f32,
    pub footprint_px: f32,
    pub footprint_centre_offset: f32,
    pub footprint_height: f32,
    pub strut_hits: u8,
    pub left: LeftRackData,
    pub right: RightRackData,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetData {
    pub switch_px: f32,
    pub ai_bias_px: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FairnessData {
    pub min_tell: u16,
    pub min_punish: u16,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArenaData {
    pub ceiling_px: i32,
    pub left_wall_x: f32,
    pub right_wall_x: f32,
    pub centre_x: f32,
    pub boss_spawn_x: f32,
    pub boss_spawn_facing: i8,
    pub walk_margin_px: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateData {
    pub engage_x: f32,
    pub inside_x: f32,
    pub settle_ticks: u16,
    pub regroup_ticks: u16,
    pub regroup_x: f32,
    /// Right edge of the closed gate's collider. 28 keeps a hero standing at
    /// `inside_x` (left edge 28) outside it.
    pub solid_x: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseData {
    pub queue_roar: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HudData {
    pub display_name: String,
    pub emblem: String,
}

/// Every tunable of the encounter (design section 6).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BossData {
    pub schema_version: u32,
    pub id: String,
    pub species: String,
    pub hp_max: u16,
    pub phase_thresholds: [u16; 2],
    pub walk_px_s: f32,
    pub turn_ticks: u16,
    pub poise_damage: u16,
    pub poise_window: u16,
    pub pound: PoundData,
    pub shockwave: ShockwaveData,
    pub charge: ChargeData,
    pub crash: CrashData,
    pub double_pound: DoublePoundData,
    pub roar: RoarData,
    pub flinch: FlinchData,
    pub body: BodyData,
    pub rack: RackData,
    pub target: TargetData,
    pub fairness: FairnessData,
    pub arena: ArenaData,
    pub gate: GateData,
    pub phase: PhaseData,
    pub ai_charge_jump_lead: u16,
    pub exit_slide_delay: u16,
    pub hud: HudData,
}

impl BossData {
    /// Parse and validate a boss data file.
    pub fn parse(json: &str) -> Result<Self, String> {
        let data: Self = serde_json::from_str(json).map_err(|e| format!("boss data: {e}"))?;
        data.validate()?;
        Ok(data)
    }

    /// The fairness floors (no tell and no punish window under the floor) and
    /// the ordering of the thresholds, checked on load.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!("boss data: schema_version {}", self.schema_version));
        }
        let tell = self.fairness.min_tell;
        let punish = self.fairness.min_punish;
        let tells = [
            ("pound.tell", self.pound.tell),
            ("charge.tell_by_phase[0]", self.charge.tell_by_phase[0]),
            ("charge.tell_by_phase[1]", self.charge.tell_by_phase[1]),
            ("charge.tell_by_phase[2]", self.charge.tell_by_phase[2]),
            ("double_pound.second_tell", self.double_pound.second_tell),
        ];
        for (name, value) in tells {
            if value < tell {
                return Err(format!(
                    "boss data: {name} {value} is under the {tell} floor"
                ));
            }
        }
        let punishes = [
            ("pound.recover", self.pound.recover),
            ("crash.stun_by_phase[0]", self.crash.stun_by_phase[0]),
            ("crash.stun_by_phase[1]", self.crash.stun_by_phase[1]),
            ("crash.stun_by_phase[2]", self.crash.stun_by_phase[2]),
        ];
        for (name, value) in punishes {
            if value < punish {
                return Err(format!(
                    "boss data: {name} {value} is under the {punish} floor"
                ));
            }
        }
        let [first, second] = self.phase_thresholds;
        if !(self.hp_max > first && first > second && second > 0) {
            return Err("boss data: phase_thresholds must descend below hp_max".to_owned());
        }
        if self.arena.left_wall_x >= self.arena.right_wall_x {
            return Err("boss data: arena walls are out of order".to_owned());
        }
        Ok(())
    }

    /// The crash pivot at the wall `facing` runs into: the ram front meets the
    /// inner wall face, then the body rebounds (`crash_pivot = wall - facing *
    /// (ram_front + rebound)`).
    pub fn crash_pivot(&self, facing: i8) -> f32 {
        let wall = if facing > 0 {
            self.arena.right_wall_x
        } else {
            self.arena.left_wall_x
        };
        wall - f32::from(facing) * (self.charge.ram_box.x[1] + self.crash.rebound_px)
    }

    /// The centre of the landing footprint of the rack on the `facing` side.
    pub fn footprint_centre(&self, facing: i8) -> f32 {
        self.crash_pivot(facing) + f32::from(facing) * self.rack.footprint_centre_offset
    }
}

static BUILTIN: OnceLock<BossData> = OnceLock::new();
static NO_RACKS: OnceLock<BossData> = OnceLock::new();

/// How a fixture sets the encounter up (`Fixture::boss`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BossSetup {
    /// Starting HP, clamped to the data's `hp_max`.
    pub hp: u16,
    /// The left rack is on (the data default). Off proves a fight winnable
    /// without racks: the strut is not there to hit.
    pub left_rack: bool,
}

impl BossSetup {
    /// A fresh boss for this setup on a floor at `floor_y`.
    pub fn boss(self, floor_y: f32) -> Boss {
        let data = if self.left_rack {
            builtin()
        } else {
            NO_RACKS.get_or_init(|| {
                let mut data = builtin().clone();
                data.rack.left.enabled = false;
                data
            })
        };
        let mut boss = Boss::new(data, floor_y);
        boss.hp = boss.hp.min(self.hp);
        boss
    }
}

/// The committed `brute_boss_f01` data, parsed once. A malformed file is a
/// programming error caught by the unit tests.
pub fn builtin() -> &'static BossData {
    BUILTIN.get_or_init(|| BossData::parse(BUILTIN_JSON).expect("brute_boss_f01.json is valid"))
}

// ------------------------------------------------------------------ states --

/// What a [`CombatEvent::BossTell`] and [`CombatEvent::BossStrike`] name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossMove {
    Pound,
    SecondPound,
    Charge,
    PhaseRoar,
}

impl BossMove {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pound => "pound",
            Self::SecondPound => "second_pound",
            Self::Charge => "charge",
            Self::PhaseRoar => "phase_roar",
        }
    }
}

/// The boss' state machine. Each mode counts `mode_tick` from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossMode {
    /// Waiting at the far side until the entrance gate closes.
    Dormant,
    /// The decision point: walk, turn, or start a move.
    Neutral,
    Turn,
    PoundTell,
    PoundStrike,
    PoundRecover,
    /// Phase 3: the step between the pounds.
    DoubleStep,
    SecondTell,
    SecondStrike,
    SecondRecover,
    ChargeTell,
    Charge,
    /// The 16 px rebound after the wall hit.
    Rebound,
    Stun,
    Roar,
    Flinch,
    Backstep,
    Pinned,
    Defeated,
}

impl BossMode {
    /// The telegraph flag: exactly the tell ticks (never the roar, the strikes
    /// or the rack fall).
    pub const fn is_tell(self) -> bool {
        matches!(self, Self::PoundTell | Self::SecondTell | Self::ChargeTell)
    }

    /// Hits deal damage but never interrupt: tells, strikes, the charge, the
    /// roar. Everything else takes free damage.
    pub const fn armoured(self) -> bool {
        matches!(
            self,
            Self::PoundTell
                | Self::PoundStrike
                | Self::SecondTell
                | Self::SecondStrike
                | Self::ChargeTell
                | Self::Charge
                | Self::Roar
        )
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Dormant => "dormant",
            Self::Neutral => "neutral",
            Self::Turn => "turn",
            Self::PoundTell => "pound_tell",
            Self::PoundStrike => "pound_strike",
            Self::PoundRecover => "pound_recover",
            Self::DoubleStep => "double_step",
            Self::SecondTell => "second_tell",
            Self::SecondStrike => "second_strike",
            Self::SecondRecover => "second_recover",
            Self::ChargeTell => "charge_tell",
            Self::Charge => "charge",
            Self::Rebound => "rebound",
            Self::Stun => "stun",
            Self::Roar => "roar",
            Self::Flinch => "flinch",
            Self::Backstep => "backstep",
            Self::Pinned => "pinned",
            Self::Defeated => "defeated",
        }
    }
}

/// The boss as the HUD reads it (spec 6, `RealHudInputs.boss`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BossState {
    pub hp: u16,
    pub hp_max: u16,
    pub mode: BossMode,
    pub phase: u8,
}

impl BossState {
    /// `ceil(234 * hp / hp_max)`: at least 1 px while alive and 0 at defeat.
    pub fn fill_px(&self) -> u8 {
        if self.hp == 0 || self.hp_max == 0 {
            return 0;
        }
        let hp = u32::from(self.hp.min(self.hp_max));
        (234 * hp).div_ceil(u32::from(self.hp_max)) as u8
    }

    /// The warning icon: exactly the tell ticks.
    pub fn warning(&self) -> bool {
        self.mode.is_tell()
    }
}

/// One hero as the boss sees it this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeroView {
    pub id: HeroId,
    /// Alive and in play (not defeated).
    pub present: bool,
    pub x: f32,
    /// World y of the hero's feet.
    pub foot_y: f32,
    pub grounded: bool,
    /// The hero's hurtbox (Donut's is the forgiving 20x24).
    pub hurt: Rect,
    /// A blow would land now (not invulnerable, not defeated).
    pub vulnerable: bool,
    /// Driven by the follower bot.
    pub bot: bool,
}

/// What the scene tells the boss each tick.
#[derive(Debug, Clone, Copy)]
pub struct BossCtx<'a> {
    pub heroes: [HeroView; 2],
    pub solids: &'a [Block],
    /// Solo: the bot hero counts `ai_bias_px` farther away.
    pub solo: bool,
    /// The scene tick of this step (attempt-local).
    pub tick: u64,
}

/// A blow the boss (or its rack) lands on a hero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeroBlow {
    pub hero: HeroId,
    pub source: u32,
    pub damage: u8,
    /// The x the blow comes from (the knockback goes away from it).
    pub from_x: f32,
    pub knock_vx: f32,
    pub knock_vy: f32,
}

/// What one boss step asks the scene to do.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BossOut {
    pub events: Vec<CombatEvent>,
    pub blows: Vec<HeroBlow>,
    /// Corrected hero x (the body and the roar push).
    pub shifts: Vec<(HeroId, f32)>,
    /// Solo: put this bot hero at `gate.regroup_x` before the gate closes.
    pub regroup: Option<HeroId>,
    /// A rack crush killed the boss: credit this player and rack.
    pub crushed_by: Option<(u32, u8)>,
}

/// One landed hit of a hero on the boss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BossHit {
    pub damage: u16,
    /// Counts toward poise (Donut's swat counts 1 with 0 damage).
    pub poise: u16,
    pub player: u32,
    pub hit_stop: u8,
}

/// The result of [`Boss::apply_hit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitResult {
    pub hp_left: u16,
    pub defeated: bool,
    pub flinched: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Ripple {
    x: f32,
    dir: i8,
    travelled: f32,
    hit: [bool; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RackState {
    /// Intact: `hits` strut hits so far (or plate dwell ticks for the right rack).
    Idle {
        hits: u8,
    },
    /// The stack is falling, `ticks` into its fall; `player` dropped it.
    Falling {
        ticks: u16,
        player: u32,
    },
    Landed,
}

/// One plate rack. `facing` is the wall side it hangs over (-1 left, +1 right).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rack {
    pub enabled: bool,
    pub facing: i8,
    pub state: RackState,
    last_instance: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Gate {
    /// Open. `crossed` is the clock the first hero crossed the engage line.
    Open {
        crossed: Option<u64>,
    },
    Closed,
    /// The boss is defeated: gate and exit are open.
    Released,
}

/// The encounter's whole simulation state. Everything here is hashed
/// ([`Self::mix_into`]) except `charge_started`, a copy of the hashed clock,
/// and `gate_closed_tick`, a copy of the scene tick at the hashed gate close;
/// presentation never feeds back into it.
#[derive(Debug, Clone)]
pub struct Boss {
    data: &'static BossData,
    floor_y: f32,
    clock: u64,
    /// The clock when the last charge tell began (`BossThreat::Charge::started`).
    /// Not mixed into the hash: a copy of the hashed `clock` taken at a hashed
    /// mode change, so it adds no state of its own.
    charge_started: u64,
    /// The combat tick on which the entrance gate closed (F01-SIM-TICKS), for
    /// presentation. Not mixed into the hash: a copy of `BossCtx::tick` taken
    /// at the hashed `Open -> Closed` change.
    gate_closed_tick: Option<u64>,
    pub hp: u16,
    pub phase: u8,
    pub mode: BossMode,
    pub mode_tick: u16,
    pub x: f32,
    pub facing: i8,
    hit_stop: u8,
    pub target: Option<HeroId>,
    poise: Vec<(u64, u16)>,
    cooldown: u16,
    roars_done: u8,
    charge_dir: i8,
    charge_hit: [bool; 2],
    strike_hit: [bool; 2],
    second_pending: bool,
    pivot_from: f32,
    ripples: Vec<Ripple>,
    pub racks: [Rack; 2],
    gate: Gate,
    /// Clocks since each hero was last outside the gate while the other crossed
    /// (solo regroup), and the player credited with the pin.
    pin_credit: u32,
    pub defeated_cause_rack: bool,
}

impl Boss {
    /// A fresh encounter on a floor whose walking line is `floor_y` (world y-up).
    pub fn new(data: &'static BossData, floor_y: f32) -> Self {
        let rack = |enabled, facing| Rack {
            enabled,
            facing,
            state: RackState::Idle { hits: 0 },
            last_instance: 0,
        };
        Self {
            data,
            floor_y,
            clock: 0,
            charge_started: 0,
            gate_closed_tick: None,
            hp: data.hp_max,
            phase: 1,
            mode: BossMode::Dormant,
            mode_tick: 0,
            x: data.arena.boss_spawn_x,
            facing: data.arena.boss_spawn_facing,
            hit_stop: 0,
            target: None,
            poise: Vec::new(),
            cooldown: 0,
            roars_done: 0,
            charge_dir: 0,
            charge_hit: [false; 2],
            strike_hit: [false; 2],
            second_pending: false,
            pivot_from: 0.0,
            ripples: Vec::new(),
            racks: [
                rack(data.rack.left.enabled, -1),
                rack(data.rack.right.enabled, 1),
            ],
            gate: Gate::Open { crossed: None },
            pin_credit: 0,
            defeated_cause_rack: false,
        }
    }

    pub fn data(&self) -> &'static BossData {
        self.data
    }

    pub fn floor_y(&self) -> f32 {
        self.floor_y
    }

    pub fn state(&self) -> BossState {
        BossState {
            hp: self.hp,
            hp_max: self.data.hp_max,
            mode: self.mode,
            phase: self.phase,
        }
    }

    pub fn alive(&self) -> bool {
        self.mode != BossMode::Defeated
    }

    /// The entrance gate is closed (a collider at the left wall).
    pub fn gate_closed(&self) -> bool {
        matches!(self.gate, Gate::Closed)
    }

    /// The combat tick on which the entrance gate closed, once it closed in
    /// [`Self::step`]; it stays set after the release. Presentation keys the
    /// gate drop off it; it is not hashed.
    pub fn gate_closed_tick(&self) -> Option<u64> {
        self.gate_closed_tick
    }

    /// The gate and the exit are open (the boss is defeated).
    pub fn released(&self) -> bool {
        matches!(self.gate, Gate::Released)
    }

    pub fn ripple_count(&self) -> usize {
        self.ripples.len()
    }

    /// Ripples as `(x, direction)` for the presentation.
    pub fn ripples(&self) -> impl Iterator<Item = (f32, i8)> + '_ {
        self.ripples.iter().map(|r| (r.x, r.dir))
    }

    /// Frozen by hit-stop (the boss' clock stands still).
    pub fn frozen(&self) -> bool {
        self.hit_stop > 0
    }

    // ------------------------------------------------------------ geometry --

    fn forward(&self, range: [f32; 2], y: [f32; 2], pivot: f32, facing: i8) -> Rect {
        let (left, right) = if facing >= 0 {
            (pivot + range[0], pivot + range[1])
        } else {
            (pivot - range[1], pivot - range[0])
        };
        Rect {
            left,
            bottom: self.floor_y + y[0],
            width: right - left,
            height: y[1] - y[0],
        }
    }

    fn box_of(&self, b: &BoxData, pivot: f32, facing: i8) -> Rect {
        self.forward(b.x, b.y, pivot, facing)
    }

    fn body_box(&self) -> Option<&BoxData> {
        let body = &self.data.body;
        match self.mode {
            BossMode::Dormant | BossMode::Defeated => None,
            BossMode::Neutral
            | BossMode::Turn
            | BossMode::PoundTell
            | BossMode::SecondTell
            | BossMode::DoubleStep
            | BossMode::Roar
            | BossMode::Flinch
            | BossMode::Backstep => Some(&body.standing),
            BossMode::ChargeTell
            | BossMode::Charge
            | BossMode::PoundStrike
            | BossMode::PoundRecover
            | BossMode::SecondStrike
            | BossMode::SecondRecover => Some(&body.committed),
            BossMode::Rebound | BossMode::Stun => Some(&body.stun),
            BossMode::Pinned => Some(&body.pinned),
        }
    }

    /// The boss hurtbox (`None` before the gate closes and after defeat).
    pub fn hurtbox(&self) -> Option<Rect> {
        self.body_box().map(|b| self.box_of(b, self.x, self.facing))
    }

    /// The solid body: the same box, but not solid while he runs.
    pub fn solid_body(&self) -> Option<Rect> {
        if self.mode == BossMode::Charge {
            None
        } else {
            self.hurtbox()
        }
    }

    /// The ram box while he runs.
    pub fn ram_box(&self) -> Option<Rect> {
        (self.mode == BossMode::Charge)
            .then(|| self.box_of(&self.data.charge.ram_box, self.x, self.charge_dir))
    }

    /// The pound hit box while a pound strike is active.
    pub fn pound_box(&self) -> Option<Rect> {
        self.pound_active()
            .then(|| self.box_of(&self.data.pound.hit_box, self.x, self.facing))
    }

    fn pound_active(&self) -> bool {
        let pound = &self.data.pound;
        matches!(self.mode, BossMode::PoundStrike | BossMode::SecondStrike)
            && self.mode_tick >= pound.smear
            && self.mode_tick < pound.smear + pound.impact
    }

    /// The landing footprint of rack `index` (world rect on the floor).
    pub fn footprint(&self, index: usize) -> Rect {
        let rack = &self.data.rack;
        let centre = self.data.footprint_centre(self.racks[index].facing);
        Rect {
            left: centre - rack.footprint_px * 0.5,
            bottom: self.floor_y,
            width: rack.footprint_px,
            height: rack.footprint_height,
        }
    }

    /// Where rack `index` hangs over: the strut rect (left) or the release
    /// plate (right).
    pub fn strut_rect(&self) -> Rect {
        self.box_of(&self.data.rack.left.strut, 0.0, 1)
    }

    pub fn ledge_rect(&self) -> Rect {
        let right = &self.data.rack.right;
        Rect {
            left: right.ledge_x[0],
            bottom: self.floor_y + right.ledge_height_px - 16.0,
            width: right.ledge_x[1] - right.ledge_x[0],
            height: 16.0,
        }
    }

    /// Phase-indexed value (phase 1..=3).
    fn by_phase<T: Copy>(&self, values: [T; 3]) -> T {
        values[usize::from(self.phase.clamp(1, 3)) - 1]
    }

    /// Ticks until the ram box reaches `hero` (for the follower's dodge), or
    /// `None` when no charge is running toward it.
    pub fn ram_arrival_ticks(&self, hero_x: f32) -> Option<f32> {
        if self.mode != BossMode::Charge {
            return None;
        }
        let dir = f32::from(self.charge_dir);
        let ram = &self.data.charge.ram_box;
        let lead = self.x + dir * ram.x[1];
        let gap = (hero_x - lead) * dir - HERO_HALF_WIDTH;
        // Past the whole box (and the hero's width): the ram has gone by.
        (gap > -(ram.x[1] - ram.x[0] + 2.0 * HERO_HALF_WIDTH))
            .then(|| gap.max(0.0) / (self.data.charge.speed_px_s * TICK))
    }

    /// The charge cooldown is over (a pilot baits the charge while it is).
    pub fn charge_ready(&self) -> bool {
        self.cooldown == 0
    }

    /// The direction of the running (or telegraphed) charge.
    pub fn run_dir(&self) -> i8 {
        if self.mode == BossMode::Charge {
            self.charge_dir
        } else {
            self.facing
        }
    }

    /// Ticks until the next damaging box turns on, in a tell or the smear of a
    /// pound, and until the ram starts in a charge tell (`None` otherwise).
    pub fn ticks_to_active(&self) -> Option<u16> {
        let d = self.data;
        let left = |len: u16| len.saturating_sub(self.mode_tick);
        match self.mode {
            BossMode::PoundTell => Some(left(d.pound.tell) + d.pound.smear),
            BossMode::SecondTell => Some(left(d.double_pound.second_tell) + d.pound.smear),
            BossMode::PoundStrike | BossMode::SecondStrike => {
                Some(d.pound.smear.saturating_sub(self.mode_tick))
            }
            BossMode::ChargeTell => Some(left(self.by_phase(d.charge.tell_by_phase))),
            _ => None,
        }
    }

    /// The follower's view of the boss: where a pound lands, the way a charge runs.
    pub fn threat(&self) -> Option<BossThreat> {
        match self.mode {
            BossMode::PoundTell | BossMode::SecondTell => Some(BossThreat::Pound {
                x: self.x,
                facing: self.facing,
                target: self.target,
            }),
            BossMode::ChargeTell | BossMode::Charge => Some(BossThreat::Charge {
                tell_left: self.ticks_to_active().unwrap_or(0),
                x: self.x,
                dir: if self.mode == BossMode::Charge {
                    self.charge_dir
                } else {
                    self.facing
                },
                running: self.mode == BossMode::Charge,
                front: self.data.charge.ram_box.x[1],
                depth: self.data.charge.ram_box.x[1] - self.data.charge.ram_box.x[0],
                speed: self.data.charge.speed_px_s * TICK,
                lead_ticks: self.data.ai_charge_jump_lead,
                started: self.charge_started,
            }),
            _ => None,
        }
    }

    // --------------------------------------------------------------- hits --

    /// CP-FU-D: the poise counted toward the next flinch right now (the hits inside
    /// the poise window, at the boss clock). A reinforced punch adds twice the poise of
    /// a plain one (CP-Q40, CP-Q42), so the state line shows the doubling. Read only.
    pub fn poise_now(&self) -> u16 {
        let window = u64::from(self.data.poise_window);
        self.poise
            .iter()
            .filter(|(at, _)| self.clock.saturating_sub(*at) < window)
            .map(|(_, p)| *p)
            .sum()
    }

    /// A hero's melee hit. Hits deal damage and hit-stop but never interrupt a
    /// tell, strike, charge or roar. Neutral hits add to poise; four damage in
    /// the window flinches him. Returns `None` when he has no hurtbox.
    pub fn apply_hit(
        &mut self,
        hit: BossHit,
        events: &mut Vec<CombatEvent>,
        tick: u64,
    ) -> Option<HitResult> {
        self.hurtbox()?;
        self.hp = self.hp.saturating_sub(hit.damage);
        // A zero-damage hit (a Donut swat glance) is poise only: no hurt event,
        // so it never sounds like a real hit.
        if hit.damage > 0 {
            events.push(CombatEvent::BossHurt {
                tick,
                source: hit.player,
                damage: hit.damage.min(255) as u8,
                hp_left: self.hp.min(255) as u8,
            });
        }
        self.hit_stop = self.hit_stop.max(hit.hit_stop);
        if self.hp == 0 {
            self.enter(BossMode::Defeated);
            self.release_gate();
            return Some(HitResult {
                hp_left: 0,
                defeated: true,
                flinched: false,
            });
        }
        let mut flinched = false;
        if matches!(self.mode, BossMode::Neutral | BossMode::Turn) {
            let window = u64::from(self.data.poise_window);
            let now = self.clock;
            self.poise
                .retain(|(at, _)| now.saturating_sub(*at) < window);
            self.poise.push((now, hit.poise));
            let total: u16 = self.poise.iter().map(|(_, p)| *p).sum();
            if total >= self.data.poise_damage {
                self.poise.clear();
                self.enter(BossMode::Flinch);
                flinched = true;
            }
        }
        Some(HitResult {
            hp_left: self.hp,
            defeated: false,
            flinched,
        })
    }

    fn release_gate(&mut self) {
        self.gate = Gate::Released;
        self.ripples.clear();
    }

    /// A melee box over the strut of the left rack: one hit per move
    /// instance. The third hit drops the stack.
    pub fn strike_strut(
        &mut self,
        instance: u32,
        melee: Rect,
        player: u32,
        tick: u64,
        events: &mut Vec<CombatEvent>,
    ) {
        self.strike_strut_weighted(instance, melee, player, tick, 1, events);
    }

    /// [`Self::strike_strut`] with the move's hit weight (CP-Q41: the reinforced
    /// haymaker counts 2). The count never exceeds the hits the strut needs;
    /// a weight of 1 is exactly `strike_strut`.
    pub fn strike_strut_weighted(
        &mut self,
        instance: u32,
        melee: Rect,
        player: u32,
        tick: u64,
        weight: u8,
        events: &mut Vec<CombatEvent>,
    ) {
        let needed = self.data.rack.strut_hits;
        let strut = self.strut_rect();
        let rack = &mut self.racks[0];
        let RackState::Idle { hits } = rack.state else {
            return;
        };
        // Racks arm on engage: a strut hit before the gate closes (or after
        // the boss is down) does nothing, so a dropped stack never waits for
        // the fight to start.
        if !matches!(self.gate, Gate::Closed)
            || !rack.enabled
            || rack.last_instance == instance
            || !melee.overlaps(strut)
        {
            return;
        }
        rack.last_instance = instance;
        let hits = hits.saturating_add(weight.max(1)).min(needed);
        events.push(CombatEvent::RackHit {
            tick,
            rack: 0,
            hits,
        });
        if hits >= needed {
            rack.state = RackState::Falling { ticks: 0, player };
            events.push(CombatEvent::RackDropped {
                tick,
                rack: 0,
                player,
            });
        } else {
            rack.state = RackState::Idle { hits };
        }
    }

    /// CP-C (CP-Q35): the box a blast must reach to bring the left rack down:
    /// the strut, while the rack is armed (gate closed), enabled and intact.
    /// `None` otherwise (a blast before the engage, after the boss is down, with
    /// the rack off or already falling does nothing).
    pub fn strut_blast_footprint(&self) -> Option<Rect> {
        let rack = &self.racks[0];
        (matches!(self.gate, Gate::Closed)
            && rack.enabled
            && matches!(rack.state, RackState::Idle { .. }))
        .then(|| self.strut_rect())
    }

    /// CP-C (CP-Q35): a blast drops the left rack outright, whatever its strut
    /// hits so far, credited to `player` (the crush credit of the existing
    /// `DropHazard` rule). Same arming rule as [`Self::strike_strut`]. Returns
    /// true when the stack started to fall.
    pub fn blast_drop_strut(
        &mut self,
        player: u32,
        tick: u64,
        events: &mut Vec<CombatEvent>,
    ) -> bool {
        if self.strut_blast_footprint().is_none() {
            return false;
        }
        self.racks[0].state = RackState::Falling { ticks: 0, player };
        events.push(CombatEvent::RackDropped {
            tick,
            rack: 0,
            player,
        });
        true
    }

    // --------------------------------------------------------------- step --

    fn enter(&mut self, mode: BossMode) {
        self.mode = mode;
        self.mode_tick = 0;
        if matches!(mode, BossMode::PoundTell | BossMode::ChargeTell) {
            self.strike_hit = [false; 2];
        }
        if mode == BossMode::ChargeTell {
            self.charge_started = self.clock;
        }
    }

    /// The hero (if any) the boss may target: present and not out of reach
    /// (standing on, or airborne above, the ledge).
    fn reachable(&self, hero: &HeroView) -> bool {
        if !hero.present {
            return false;
        }
        let right = &self.data.rack.right;
        let on_ledge = self.racks[1].enabled
            && hero.x >= right.ledge_x[0]
            && hero.x <= right.ledge_x[1]
            && hero.foot_y >= self.floor_y + right.ledge_height_px - 1.0;
        !on_ledge
    }

    /// Q3 B: the nearest reachable hero, with hysteresis and the solo bias.
    fn update_target(&mut self, ctx: &BossCtx) {
        let target = &self.data.target;
        let effective = |h: &HeroView| {
            let bias = if ctx.solo && h.bot {
                target.ai_bias_px
            } else {
                0.0
            };
            (h.x - self.x).abs() + bias
        };
        let candidates: Vec<&HeroView> = ctx.heroes.iter().filter(|h| self.reachable(h)).collect();
        let nearest = candidates.iter().copied().min_by(|a, b| {
            effective(a)
                .total_cmp(&effective(b))
                .then(a.id.index().cmp(&b.id.index()))
        });
        let held = self
            .target
            .and_then(|id| candidates.iter().copied().find(|h| h.id == id));
        self.target = match (held, nearest) {
            (Some(kept), Some(best)) if best.id != kept.id => {
                if effective(best) + target.switch_px <= effective(kept) {
                    Some(best.id)
                } else {
                    Some(kept.id)
                }
            }
            (Some(kept), _) => Some(kept.id),
            (None, best) => best.map(|h| h.id),
        };
    }

    /// One fixed tick. Call after the heroes' strikes and the scene step.
    pub fn step(&mut self, ctx: &BossCtx) -> BossOut {
        let mut out = BossOut::default();
        self.clock += 1;
        match self.gate {
            Gate::Open { .. } => {
                self.step_gate(ctx, &mut out);
                return out;
            }
            Gate::Closed | Gate::Released => {}
        }
        self.step_racks(ctx, &mut out);
        self.step_ripples(ctx, &mut out);
        if self.mode == BossMode::Defeated {
            return out;
        }
        if self.hit_stop > 0 {
            self.hit_stop -= 1;
        } else {
            self.cooldown = self.cooldown.saturating_sub(1);
            self.step_mode(ctx, &mut out);
        }
        self.push_heroes(ctx, &mut out);
        out
    }

    fn step_gate(&mut self, ctx: &BossCtx, out: &mut BossOut) {
        let gate = &self.data.gate;
        let Gate::Open { crossed } = self.gate else {
            return;
        };
        let living: Vec<&HeroView> = ctx.heroes.iter().filter(|h| h.present).collect();
        let crossed = crossed.or_else(|| {
            living
                .iter()
                .any(|h| h.x >= gate.engage_x)
                .then_some(self.clock)
        });
        self.gate = Gate::Open { crossed };
        let Some(at) = crossed else {
            return;
        };
        let elapsed = self.clock - at;
        let all_inside = living.iter().all(|h| h.x >= gate.inside_x);
        if elapsed >= u64::from(gate.settle_ticks) && all_inside && !living.is_empty() {
            self.gate = Gate::Closed;
            self.gate_closed_tick = Some(ctx.tick);
            self.enter(BossMode::Neutral);
            out.events.push(CombatEvent::BossEngaged { tick: ctx.tick });
            for (index, rack) in self.racks.iter().enumerate() {
                if rack.enabled {
                    out.events.push(CombatEvent::RackArmed {
                        tick: ctx.tick,
                        rack: index as u8,
                    });
                }
            }
            return;
        }
        // Solo: the follower regroup places the bot hero when it is still
        // outside after `regroup_ticks`. Two players wait indefinitely.
        if ctx.solo
            && elapsed >= u64::from(gate.regroup_ticks)
            && let Some(bot) = living.iter().find(|h| h.bot && h.x < gate.inside_x)
        {
            out.regroup = Some(bot.id);
        }
    }

    fn step_racks(&mut self, ctx: &BossCtx, out: &mut BossOut) {
        // The right rack's release plate: stand on the ledge for the dwell.
        if self.racks[1].enabled
            && let RackState::Idle { hits } = self.racks[1].state
        {
            let right = &self.data.rack.right;
            let (x0, x1) = (right.ledge_x[0], right.ledge_x[1]);
            let top = self.floor_y + right.ledge_height_px;
            let player = ctx
                .heroes
                .iter()
                .find(|h| {
                    h.present
                        && !h.bot
                        && h.grounded
                        && h.x >= x0
                        && h.x <= x1
                        && h.foot_y >= top - 1.0
                })
                .map(|h| h.id.index() as u32);
            match player {
                Some(player) => {
                    let dwell = u16::from(hits) + 1;
                    if dwell >= right.dwell_ticks {
                        self.racks[1].state = RackState::Falling { ticks: 0, player };
                        out.events.push(CombatEvent::RackDropped {
                            tick: ctx.tick,
                            rack: 1,
                            player,
                        });
                    } else {
                        self.racks[1].state = RackState::Idle { hits: dwell as u8 };
                    }
                }
                None => self.racks[1].state = RackState::Idle { hits: 0 },
            }
        }
        for index in 0..2 {
            let RackState::Falling { ticks, player } = self.racks[index].state else {
                continue;
            };
            let ticks = ticks + 1;
            // The drop tick is tick 0: it lands `fall_ticks` ticks after it.
            if ticks <= self.data.rack.fall_ticks {
                self.racks[index].state = RackState::Falling { ticks, player };
                continue;
            }
            self.racks[index].state = RackState::Landed;
            self.land_rack(index, player, ctx, out);
        }
    }

    fn land_rack(&mut self, index: usize, player: u32, ctx: &BossCtx, out: &mut BossOut) {
        let footprint = self.footprint(index);
        let rack = &self.data.rack;
        let crushed = self.hurtbox().is_some_and(|b| b.overlaps(footprint));
        out.events.push(CombatEvent::RackLanded {
            tick: ctx.tick,
            rack: index as u8,
            crushed,
        });
        for hero in ctx.heroes.iter().filter(|h| h.present && h.vulnerable) {
            // The footprint test uses the hero's pivot (design section 2), not
            // the 24 px hurtbox, and feet under the stack's top.
            if hero.x >= footprint.left
                && hero.x < footprint.right()
                && hero.foot_y < footprint.top()
            {
                out.blows.push(HeroBlow {
                    hero: hero.id,
                    source: RACK_SOURCE_ID,
                    damage: rack.hero_damage,
                    from_x: footprint.left + footprint.width * 0.5,
                    knock_vx: 0.0,
                    knock_vy: rack.hero_hop_px_s,
                });
            }
        }
        if !crushed {
            return;
        }
        let damage = rack.boss_damage;
        self.hp = self.hp.saturating_sub(damage);
        out.events.push(CombatEvent::BossHurt {
            tick: ctx.tick,
            source: player,
            damage: damage.min(255) as u8,
            hp_left: self.hp.min(255) as u8,
        });
        if self.hp == 0 {
            self.enter(BossMode::Defeated);
            self.defeated_cause_rack = true;
            self.release_gate();
            out.crushed_by = Some((player, index as u8));
        } else {
            self.pin_credit = player;
            self.enter(BossMode::Pinned);
        }
    }

    fn step_ripples(&mut self, ctx: &BossCtx, out: &mut BossOut) {
        let sw = self.data.shockwave.clone();
        let step = sw.speed * TICK;
        let floor = self.floor_y;
        for ripple in &mut self.ripples {
            ripple.x += f32::from(ripple.dir) * step;
            ripple.travelled += step;
            let rect = Rect {
                left: ripple.x - sw.width * 0.5,
                bottom: floor,
                width: sw.width,
                height: sw.height,
            };
            for hero in ctx.heroes.iter().filter(|h| h.present && h.vulnerable) {
                let i = hero.id.index();
                if !ripple.hit[i] && hero.hurt.overlaps(rect) {
                    ripple.hit[i] = true;
                    out.blows.push(HeroBlow {
                        hero: hero.id,
                        source: BOSS_SOURCE_ID,
                        damage: sw.damage,
                        from_x: ripple.x - f32::from(ripple.dir) * 8.0,
                        knock_vx: 0.0,
                        knock_vy: sw.hop_px_s,
                    });
                }
            }
        }
        self.ripples.retain(|r| r.travelled < sw.reach);
    }

    fn target_view<'a>(&self, ctx: &'a BossCtx) -> Option<&'a HeroView> {
        self.target
            .map(|id| &ctx.heroes[id.index()])
            .filter(|h| h.present)
    }

    fn step_mode(&mut self, ctx: &BossCtx, out: &mut BossOut) {
        // The threshold roar is queued, never shortening what is running.
        self.mode_tick += 1;
        let d = self.data;
        match self.mode {
            BossMode::Dormant | BossMode::Defeated => {}
            BossMode::Neutral => {
                self.mode_tick = 0;
                self.neutral(ctx, out);
            }
            BossMode::Turn => {
                if self.mode_tick >= d.turn_ticks {
                    self.facing = -self.facing;
                    self.enter(BossMode::Neutral);
                }
            }
            BossMode::PoundTell => {
                if self.mode_tick >= d.pound.tell {
                    self.enter(BossMode::PoundStrike);
                    out.events.push(CombatEvent::BossStrike {
                        tick: ctx.tick,
                        mv: BossMove::Pound,
                    });
                }
            }
            BossMode::SecondTell => {
                if self.mode_tick >= d.double_pound.second_tell {
                    self.enter(BossMode::SecondStrike);
                    out.events.push(CombatEvent::BossStrike {
                        tick: ctx.tick,
                        mv: BossMove::SecondPound,
                    });
                }
            }
            BossMode::PoundStrike | BossMode::SecondStrike => self.pound_strike(ctx, out),
            BossMode::PoundRecover => {
                if self.mode_tick >= d.pound.recover {
                    if self.second_pending {
                        self.enter(BossMode::DoubleStep);
                    } else {
                        self.enter(BossMode::Neutral);
                    }
                }
            }
            BossMode::DoubleStep => {
                let step = f32::from(self.facing) * d.double_pound.step_px_per_tick;
                self.x = self.clamp_walk(self.x + step);
                if self.mode_tick >= d.double_pound.step_ticks {
                    self.second_pending = false;
                    self.strike_hit = [false; 2];
                    self.enter(BossMode::SecondTell);
                    self.strike_hit = [false; 2];
                    out.events.push(CombatEvent::BossTell {
                        tick: ctx.tick,
                        mv: BossMove::SecondPound,
                        target: self.target.unwrap_or(HeroId::Carl),
                    });
                }
            }
            BossMode::SecondRecover => {
                if self.mode_tick >= d.pound.recover {
                    self.enter(BossMode::Neutral);
                }
            }
            BossMode::ChargeTell => {
                if self.mode_tick >= self.by_phase(d.charge.tell_by_phase) {
                    self.charge_dir = self.facing;
                    self.charge_hit = [false; 2];
                    self.enter(BossMode::Charge);
                    out.events.push(CombatEvent::BossStrike {
                        tick: ctx.tick,
                        mv: BossMove::Charge,
                    });
                }
            }
            BossMode::Charge => self.charge(ctx, out),
            BossMode::Rebound => {
                let per = d.crash.rebound_px / f32::from(d.crash.ticks.max(1));
                self.x -= f32::from(self.charge_dir) * per;
                if self.mode_tick >= d.crash.ticks {
                    self.x = d.crash_pivot(self.charge_dir);
                    self.enter(BossMode::Stun);
                }
            }
            BossMode::Stun => {
                if self.mode_tick >= self.by_phase(d.crash.stun_by_phase) {
                    self.enter(BossMode::Neutral);
                }
            }
            BossMode::Roar => {
                if self.mode_tick >= d.roar.ticks {
                    self.enter(BossMode::Neutral);
                }
            }
            BossMode::Flinch => {
                if self.mode_tick >= d.flinch.ticks {
                    self.enter(BossMode::Backstep);
                }
            }
            BossMode::Backstep => {
                let per = d.flinch.backstep_px / f32::from(d.flinch.backstep_ticks.max(1));
                self.x = self.clamp_walk(self.x - f32::from(self.facing) * per);
                if self.mode_tick >= d.flinch.backstep_ticks {
                    self.enter(BossMode::Neutral);
                }
            }
            BossMode::Pinned => {
                if self.mode_tick >= d.rack.pin_ticks {
                    self.enter(BossMode::Neutral);
                }
            }
        }
    }

    fn clamp_walk(&self, x: f32) -> f32 {
        let a = &self.data.arena;
        x.clamp(
            a.left_wall_x + a.walk_margin_px,
            a.right_wall_x - a.walk_margin_px,
        )
    }

    fn start_roar(&mut self, ctx: &BossCtx, out: &mut BossOut) {
        self.roars_done += 1;
        self.phase = self.roars_done + 1;
        self.enter(BossMode::Roar);
        out.events.push(CombatEvent::BossPhase {
            tick: ctx.tick,
            phase: self.phase,
        });
        // The push wave at tick 0: heroes within the radius are pushed back.
        let (radius, push) = (self.data.roar.push_radius, self.data.roar.push_px);
        for hero in ctx.heroes.iter().filter(|h| h.present) {
            let gap = hero.x - self.x;
            if gap.abs() > radius {
                continue;
            }
            let dir = if gap >= 0.0 { 1.0 } else { -1.0 };
            let a = &self.data.arena;
            let to = (hero.x + dir * push).clamp(
                a.left_wall_x + HERO_HALF_WIDTH,
                a.right_wall_x - HERO_HALF_WIDTH,
            );
            let probe = hero_rect(to, hero.foot_y);
            if !solid_at(ctx.solids, probe) {
                out.shifts.push((hero.id, to));
            }
        }
    }

    fn neutral(&mut self, ctx: &BossCtx, out: &mut BossOut) {
        let d = self.data;
        self.update_target(ctx);
        if d.phase.queue_roar
            && self.roars_done < 2
            && self.hp <= d.phase_thresholds[usize::from(self.roars_done)]
        {
            self.start_roar(ctx, out);
            return;
        }
        let Some(target) = self.target_view(ctx).copied() else {
            // Nobody reachable: walk to the centre and growl.
            let centre = d.arena.centre_x;
            if (centre - self.x).abs() > 1.0 {
                self.facing = if centre > self.x { 1 } else { -1 };
                self.x += f32::from(self.facing) * d.walk_px_s * TICK;
            }
            return;
        };
        let gap = target.x - self.x;
        let side: i8 = if gap >= 0.0 { 1 } else { -1 };
        if side != self.facing && gap.abs() > 4.0 {
            self.enter(BossMode::Turn);
            return;
        }
        let ahead = gap * f32::from(self.facing);
        if (d.pound.range_px[0]..=d.pound.range_px[1]).contains(&ahead) {
            self.enter(BossMode::PoundTell);
            self.second_pending = self.phase >= 3;
            out.events.push(CombatEvent::BossTell {
                tick: ctx.tick,
                mv: BossMove::Pound,
                target: target.id,
            });
            return;
        }
        let run_room = (d.crash_pivot(self.facing) - self.x) * f32::from(self.facing);
        if self.cooldown == 0
            && gap.abs() >= d.charge.min_target_px
            && run_room >= d.charge.min_room_px
        {
            self.cooldown = self.by_phase(d.charge.cooldown_by_phase);
            self.enter(BossMode::ChargeTell);
            out.events.push(CombatEvent::BossTell {
                tick: ctx.tick,
                mv: BossMove::Charge,
                target: target.id,
            });
            return;
        }
        if ahead > d.pound.range_px[1] {
            self.x = self.clamp_walk(self.x + f32::from(self.facing) * d.walk_px_s * TICK);
        }
    }

    fn pound_strike(&mut self, ctx: &BossCtx, out: &mut BossOut) {
        let d = self.data;
        let second = self.mode == BossMode::SecondStrike;
        if self.pound_active() {
            if self.mode_tick == d.pound.smear && self.phase >= d.shockwave.from_phase {
                let origin = self.x + f32::from(self.facing) * d.shockwave.origin_px;
                for dir in [-1i8, 1] {
                    self.ripples.push(Ripple {
                        x: origin,
                        dir,
                        travelled: 0.0,
                        hit: [false; 2],
                    });
                }
            }
            let area = self.box_of(&d.pound.hit_box, self.x, self.facing);
            let away = |hero_x: f32, boss_x: f32| if hero_x >= boss_x { 1.0 } else { -1.0 };
            let mut landed = Vec::new();
            for hero in ctx.heroes.iter().filter(|h| h.present && h.vulnerable) {
                let i = hero.id.index();
                if !self.strike_hit[i] && hero.hurt.overlaps(area) {
                    self.strike_hit[i] = true;
                    landed.push(HeroBlow {
                        hero: hero.id,
                        source: BOSS_SOURCE_ID,
                        damage: d.pound.damage,
                        from_x: self.x,
                        knock_vx: away(hero.x, self.x) * d.pound.launch_px_s[0],
                        knock_vy: d.pound.launch_px_s[1],
                    });
                }
            }
            out.blows.extend(landed);
        }
        if self.mode_tick >= d.pound.smear + d.pound.impact {
            self.enter(if second {
                BossMode::SecondRecover
            } else {
                BossMode::PoundRecover
            });
        }
    }

    fn charge(&mut self, ctx: &BossCtx, out: &mut BossOut) {
        let d = self.data;
        let dir = self.charge_dir;
        self.x += f32::from(dir) * d.charge.speed_px_s * TICK;
        let wall = if dir > 0 {
            d.arena.right_wall_x
        } else {
            d.arena.left_wall_x
        };
        let front = self.x + f32::from(dir) * d.charge.ram_box.x[1];
        let ram = self.box_of(&d.charge.ram_box, self.x, dir);
        let mut landed = Vec::new();
        for hero in ctx.heroes.iter().filter(|h| h.present && h.vulnerable) {
            let i = hero.id.index();
            if !self.charge_hit[i] && hero.hurt.overlaps(ram) {
                self.charge_hit[i] = true;
                let away = if hero.x >= self.x { 1.0 } else { -1.0 };
                landed.push(HeroBlow {
                    hero: hero.id,
                    source: BOSS_SOURCE_ID,
                    damage: d.charge.damage,
                    from_x: self.x,
                    knock_vx: away * d.charge.launch[0],
                    knock_vy: d.charge.launch[1],
                });
            }
        }
        out.blows.extend(landed);
        if (front - wall) * f32::from(dir) >= 0.0 {
            self.x = wall - f32::from(dir) * d.charge.ram_box.x[1];
            self.pivot_from = self.x;
            self.enter(BossMode::Rebound);
            out.events.push(CombatEvent::BossCrash {
                tick: ctx.tick,
                wall: dir,
            });
        }
    }

    /// After hero physics: no hero stands inside the solid body. A hero whose
    /// box overlaps it, with its feet below the box top, is pushed out to the
    /// nearer edge plus 1 px (at most `push_px_per_tick`), to the other side
    /// when the nearer one has less than `min_side_room_px` before a wall.
    fn push_heroes(&mut self, ctx: &BossCtx, out: &mut BossOut) {
        let Some(body) = self.solid_body() else {
            return;
        };
        let (push, min_room) = (
            self.data.body.push_px_per_tick,
            self.data.body.min_side_room_px,
        );
        let arena = &self.data.arena;
        for hero in ctx.heroes.iter().filter(|h| h.present) {
            let left = hero.x - HERO_HALF_WIDTH;
            let right = hero.x + HERO_HALF_WIDTH;
            if right <= body.left || left >= body.right() || hero.foot_y >= body.top() {
                continue;
            }
            let to_left = body.left - HERO_HALF_WIDTH - 1.0;
            let to_right = body.right() + HERO_HALF_WIDTH + 1.0;
            let room_left = body.left - arena.left_wall_x;
            let room_right = arena.right_wall_x - body.right();
            let nearer_left = (hero.x - to_left).abs() <= (to_right - hero.x).abs();
            let go_left = if nearer_left {
                room_left >= min_room || room_right < min_room
            } else {
                room_right < min_room && room_left >= min_room
            };
            let goal = if go_left { to_left } else { to_right };
            let to = hero.x + (goal - hero.x).clamp(-push, push);
            if !solid_at(ctx.solids, hero_rect(to, hero.foot_y)) {
                out.shifts.push((hero.id, to));
            }
        }
    }

    // -------------------------------------------------------------- hashing --

    /// Fold the whole encounter state into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.clock);
        mix(u64::from(self.hp));
        mix(u64::from(self.phase));
        mix(self.mode as u64);
        mix(u64::from(self.mode_tick));
        mix(u64::from(self.x.to_bits()));
        mix(self.facing as i64 as u64);
        mix(u64::from(self.hit_stop));
        mix(self.target.map_or(0, |t| t.index() as u64 + 1));
        mix(self.poise.len() as u64);
        for (at, amount) in &self.poise {
            mix(*at);
            mix(u64::from(*amount));
        }
        mix(u64::from(self.cooldown));
        mix(u64::from(self.roars_done));
        mix(self.charge_dir as i64 as u64);
        for hit in self.charge_hit.iter().chain(&self.strike_hit) {
            mix(u64::from(*hit));
        }
        mix(u64::from(self.second_pending));
        mix(u64::from(self.pin_credit));
        mix(self.ripples.len() as u64);
        for ripple in &self.ripples {
            mix(u64::from(ripple.x.to_bits()));
            mix(u64::from(ripple.travelled.to_bits()));
            mix(ripple.dir as i64 as u64);
            mix(u64::from(ripple.hit[0]) | u64::from(ripple.hit[1]) << 1);
        }
        for rack in &self.racks {
            mix(u64::from(rack.enabled));
            mix(u64::from(rack.last_instance));
            match rack.state {
                RackState::Idle { hits } => {
                    mix(1);
                    mix(u64::from(hits));
                }
                RackState::Falling { ticks, player } => {
                    mix(2);
                    mix(u64::from(ticks));
                    mix(u64::from(player));
                }
                RackState::Landed => mix(3),
            }
        }
        match self.gate {
            Gate::Open { crossed } => {
                mix(4);
                mix(crossed.map_or(0, |c| c + 1));
            }
            Gate::Closed => mix(5),
            Gate::Released => mix(6),
        }
        mix(u64::from(self.defeated_cause_rack));
    }
}

/// FA-Q02: the read-only facts the companion's `BossView` is built from, one
/// plain value per tick (see [`Boss::facts`]). Everything is derived from the
/// boss' own hashed state and its data; nothing here feeds back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BossFacts {
    pub mode: BossMode,
    pub phase: u8,
    pub hp: u16,
    pub x: f32,
    pub facing: i8,
    pub target: Option<HeroId>,
    /// The body box of the current mode (`None` before the gate closes and after
    /// the defeat).
    pub hurtbox: Option<Rect>,
    /// Ticks the current mode still has, for the modes that take free damage
    /// (Stun, Pinned, the pound recoveries and the flinch); `None` otherwise.
    pub vulnerable_ticks_left: Option<u16>,
    /// The direction of the running or telegraphed charge.
    pub run_dir: i8,
    /// The ram box's near and far edge ahead of the boss pivot (px).
    pub ram_x: [f32; 2],
    /// The pound's hit box near and far edge ahead of the boss pivot (px).
    pub pound_x: [f32; 2],
    /// How far from the boss pivot a shockwave ripple may reach on the ground
    /// (the origin ahead of the pivot plus the reach plus half the ripple's
    /// width), `0` while the boss has no shockwave yet (phase below
    /// `shockwave.from_phase`).
    pub ripple_reach_px: f32,
    /// The arena walls the charge runs to.
    pub left_wall_x: f32,
    pub right_wall_x: f32,
}

impl Boss {
    /// The boss' facts for the companion's `BossView` (FA-Q02).
    pub fn facts(&self) -> BossFacts {
        let d = self.data;
        let left = |len: u16| Some(len.saturating_sub(self.mode_tick));
        let vulnerable_ticks_left = match self.mode {
            BossMode::Stun => left(self.by_phase(d.crash.stun_by_phase)),
            BossMode::Pinned => left(d.rack.pin_ticks),
            BossMode::PoundRecover | BossMode::SecondRecover => left(d.pound.recover),
            BossMode::Flinch => left(d.flinch.ticks),
            _ => None,
        };
        BossFacts {
            mode: self.mode,
            phase: self.phase,
            hp: self.hp,
            x: self.x,
            facing: self.facing,
            target: self.target,
            hurtbox: self.hurtbox(),
            vulnerable_ticks_left,
            run_dir: self.run_dir(),
            ram_x: d.charge.ram_box.x,
            pound_x: d.pound.hit_box.x,
            ripple_reach_px: if self.phase >= d.shockwave.from_phase {
                d.shockwave.origin_px + d.shockwave.reach + d.shockwave.width * 0.5
            } else {
                0.0
            },
            left_wall_x: d.arena.left_wall_x,
            right_wall_x: d.arena.right_wall_x,
        }
    }
}

/// The follower's view of the boss (`Boss::threat`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BossThreat {
    /// A pound tell: the strike lands in front of `x`.
    Pound {
        x: f32,
        facing: i8,
        target: Option<HeroId>,
    },
    /// A charge tell or run in direction `dir`: `front` is the near edge of the
    /// ram box, `speed` px per tick while running. `tell_left` is the ticks the
    /// tell still has before the run starts (0 while running).
    Charge {
        tell_left: u16,
        x: f32,
        dir: i8,
        running: bool,
        front: f32,
        /// Depth of the ram box (its far edge minus its near edge).
        depth: f32,
        speed: f32,
        /// `ai_charge_jump_lead`: jump when the ram reaches the bot within this.
        lead_ticks: u16,
        /// The boss clock on the first tick of this charge's tell: one value per
        /// charge, so chained charges are told apart (FA-Q09 keys the AI's
        /// per-charge dodge latch by it).
        started: u64,
    },
}

/// The hero's body for a collision probe: half a pixel inside top and bottom,
/// so a hero standing on the floor is not "inside" it.
fn hero_rect(x: f32, foot_y: f32) -> Rect {
    Rect {
        left: x - HERO_HALF_WIDTH,
        bottom: foot_y + 0.5,
        width: HERO_HALF_WIDTH * 2.0,
        height: HERO_HALF_HEIGHT * 2.0 - 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLOOR: f32 = 100.0;

    fn data() -> &'static BossData {
        builtin()
    }

    fn view(id: HeroId, x: f32) -> HeroView {
        HeroView {
            id,
            present: true,
            x,
            foot_y: FLOOR,
            grounded: true,
            hurt: Rect {
                left: x - HERO_HALF_WIDTH,
                bottom: FLOOR,
                width: HERO_HALF_WIDTH * 2.0,
                height: HERO_HALF_HEIGHT * 2.0,
            },
            vulnerable: true,
            bot: false,
        }
    }

    fn ctx(carl: f32, donut: f32) -> BossCtx<'static> {
        BossCtx {
            heroes: [view(HeroId::Carl, carl), view(HeroId::Donut, donut)],
            solids: &[],
            solo: false,
            tick: 1,
        }
    }

    /// A boss already engaged at `x` facing `facing`.
    fn engaged(x: f32, facing: i8) -> Boss {
        let mut boss = Boss::new(data(), FLOOR);
        boss.gate = Gate::Closed;
        boss.mode = BossMode::Neutral;
        boss.x = x;
        boss.facing = facing;
        boss
    }

    fn run(boss: &mut Boss, c: &BossCtx, ticks: u32) -> Vec<BossOut> {
        (0..ticks).map(|_| boss.step(c)).collect()
    }

    fn hit(boss: &mut Boss, damage: u16) -> Option<HitResult> {
        let mut events = Vec::new();
        boss.apply_hit(
            BossHit {
                damage,
                poise: damage,
                player: 0,
                hit_stop: 0,
            },
            &mut events,
            1,
        )
    }

    #[test]
    fn the_committed_data_is_valid_and_matches_the_spec_table() {
        let d = data();
        assert_eq!(d.id, "brute_boss_f01");
        assert_eq!(d.hp_max, 36);
        assert_eq!(d.phase_thresholds, [24, 12]);
        assert_eq!(
            (d.pound.tell, d.pound.smear, d.pound.impact, d.pound.recover),
            (36, 3, 6, 24)
        );
        assert_eq!(d.charge.tell_by_phase, [40, 36, 30]);
        assert_eq!(d.crash.stun_by_phase, [90, 75, 60]);
        assert!(d.rack.left.enabled && !d.rack.right.enabled);
        assert_eq!(d.hud.display_name, "THE JUICER");
        assert_eq!(d.crash_pivot(-1), 130.0);
        assert_eq!(d.crash_pivot(1), 510.0);
    }

    #[test]
    fn data_validation_rejects_short_tells_short_punishes_bad_thresholds_and_unknown_fields() {
        let base = BUILTIN_JSON;
        assert!(BossData::parse(base).is_ok());
        let short_tell = base.replace("\"tell\": 36", "\"tell\": 20");
        assert!(
            BossData::parse(&short_tell)
                .unwrap_err()
                .contains("pound.tell")
        );
        let short_punish = base.replace("\"recover\": 24", "\"recover\": 12");
        assert!(
            BossData::parse(&short_punish)
                .unwrap_err()
                .contains("pound.recover")
        );
        let short_stun = base.replace("[90, 75, 60]", "[90, 75, 10]");
        assert!(BossData::parse(&short_stun).unwrap_err().contains("stun"));
        let bad = base.replace(
            "\"phase_thresholds\": [24, 12]",
            "\"phase_thresholds\": [12, 24]",
        );
        assert!(BossData::parse(&bad).unwrap_err().contains("thresholds"));
        let unknown = base.replace("\"hp_max\": 36,", "\"hp_max\": 36, \"surprise\": 1,");
        assert!(BossData::parse(&unknown).is_err());
    }

    #[test]
    fn every_tell_and_punish_window_meets_the_fairness_floors() {
        let d = data();
        let floor = d.fairness.min_tell;
        let tells = d
            .charge
            .tell_by_phase
            .iter()
            .copied()
            .chain([d.pound.tell, d.double_pound.second_tell]);
        for tell in tells {
            assert!(tell >= floor, "tell {tell}");
        }
        let punish = d.fairness.min_punish;
        for window in d
            .crash
            .stun_by_phase
            .iter()
            .copied()
            .chain([d.pound.recover])
        {
            assert!(window >= punish, "punish {window}");
        }
    }

    #[test]
    fn the_gate_waits_for_every_living_hero_and_closes_once() {
        let mut boss = Boss::new(data(), FLOOR);
        // Carl crosses the engage line; Donut lags at x 20: the gate waits.
        let lag = ctx(200.0, 20.0);
        let outs = run(&mut boss, &lag, 300);
        assert!(!boss.gate_closed());
        assert!(outs.iter().all(|o| o.events.is_empty()));
        assert_eq!(boss.mode, BossMode::Dormant);
        // Donut walks in: it closes after the settle window, with one event.
        let both = ctx(200.0, 60.0);
        let outs = run(&mut boss, &both, 4);
        let engaged: usize = outs
            .iter()
            .flat_map(|o| &o.events)
            .filter(|e| matches!(e, CombatEvent::BossEngaged { .. }))
            .count();
        assert_eq!(engaged, 1);
        assert!(boss.gate_closed());
        assert_ne!(boss.mode, BossMode::Dormant);
        // A defeated hero does not hold the gate.
        let mut boss = Boss::new(data(), FLOOR);
        let mut dead = ctx(200.0, 20.0);
        dead.heroes[1].present = false;
        run(&mut boss, &dead, 20);
        assert!(boss.gate_closed());
    }

    /// F01-SIM-TICKS: the close records the scene tick of the close step,
    /// once, deterministically, and outside the state hash.
    #[test]
    fn the_gate_records_its_close_tick_once_unhashed() {
        let run = || {
            let mut boss = Boss::new(data(), FLOOR);
            let mut seen = Vec::new();
            for tick in 500..540u64 {
                let c = BossCtx {
                    tick,
                    ..ctx(120.0, 120.0)
                };
                let out = boss.step(&c);
                let engaged = out
                    .events
                    .iter()
                    .any(|e| matches!(e, CombatEvent::BossEngaged { .. }));
                if engaged {
                    assert_eq!(boss.gate_closed_tick(), Some(tick), "the close step");
                }
                seen.push(boss.gate_closed_tick());
            }
            (boss, seen)
        };
        let (boss, seen) = run();
        let at = seen.iter().position(Option::is_some).expect("closes");
        let tick = seen[at].expect("set");
        assert!(boss.gate_closed());
        assert!(seen[at..].iter().all(|t| *t == Some(tick)), "set once");
        assert!(seen[..at].iter().all(Option::is_none));
        assert_eq!(run().1, seen, "deterministic");
        let mut copy = boss.clone();
        copy.gate_closed_tick = None;
        assert_eq!(hash_of(&copy), hash_of(&boss), "not hashed");
        assert_eq!(Boss::new(data(), FLOOR).gate_closed_tick(), None);
    }

    #[test]
    fn the_gate_waits_eight_ticks_after_the_first_crossing() {
        let mut boss = Boss::new(data(), FLOOR);
        let c = ctx(120.0, 120.0);
        let outs = run(&mut boss, &c, 12);
        let at = outs
            .iter()
            .position(|o| !o.events.is_empty())
            .expect("closes");
        assert_eq!(at + 1, 1 + usize::from(data().gate.settle_ticks));
    }

    #[test]
    fn solo_regroup_is_asked_for_after_120_ticks_and_two_players_wait_forever() {
        let mut solo = Boss::new(data(), FLOOR);
        let mut c = ctx(200.0, 10.0);
        c.solo = true;
        c.heroes[1].bot = true;
        let outs = run(&mut solo, &c, 130);
        assert!(outs.iter().any(|o| o.regroup == Some(HeroId::Donut)));
        assert!(
            outs.iter().take(100).all(|o| o.regroup.is_none()),
            "not before 120 ticks"
        );
        let mut two = Boss::new(data(), FLOOR);
        let c = ctx(200.0, 10.0);
        let outs = run(&mut two, &c, 600);
        assert!(outs.iter().all(|o| o.regroup.is_none()));
        assert!(!two.gate_closed());
    }

    #[test]
    fn a_pound_has_a_36_tick_tell_then_hits_on_ticks_39_to_44_and_recovers_24() {
        let mut boss = engaged(300.0, 1);
        // Carl 60 px ahead: inside the pound range.
        let c = ctx(360.0, 100.0);
        let mut tell_events = Vec::new();
        let mut modes = Vec::new();
        let mut blows = Vec::new();
        for t in 0..69 {
            let out = boss.step(&c);
            if out.events.iter().any(|e| {
                matches!(
                    e,
                    CombatEvent::BossTell {
                        mv: BossMove::Pound,
                        ..
                    }
                )
            }) {
                tell_events.push(t);
            }
            if !out.blows.is_empty() {
                blows.push(t);
            }
            modes.push(boss.mode);
        }
        assert_eq!(tell_events, vec![0], "the tell event is on the first tick");
        // 36 tell ticks (the tell-start tick included), then smear 3, impact 6.
        let tells = modes.iter().filter(|m| **m == BossMode::PoundTell).count();
        assert_eq!(tells, 36);
        assert_eq!(blows.len(), 1, "one blow per strike");
        assert_eq!(blows[0], 36 + 3, "first impact tick is attack tick 39");
        let recover = modes
            .iter()
            .filter(|m| **m == BossMode::PoundRecover)
            .count();
        assert_eq!(recover, 24);
    }

    #[test]
    fn the_pound_does_two_damage_and_a_hero_above_the_box_is_not_hit() {
        let mut boss = engaged(300.0, 1);
        let mut c = ctx(360.0, 100.0);
        let outs = run(&mut boss, &c, 60);
        let blow = outs.iter().flat_map(|o| &o.blows).next().unwrap();
        assert_eq!(
            (blow.hero, blow.damage, blow.source),
            (HeroId::Carl, 2, BOSS_SOURCE_ID)
        );
        assert_eq!((blow.knock_vx, blow.knock_vy), (160.0, 200.0));
        // A hero jumping 30 px up clears the 22 px box.
        let mut boss = engaged(300.0, 1);
        c.heroes[0].hurt.bottom = FLOOR + 30.0;
        let outs = run(&mut boss, &c, 60);
        assert!(outs.iter().all(|o| o.blows.is_empty()));
    }

    #[test]
    fn the_warning_flag_is_exactly_the_tell_ticks() {
        let mut boss = engaged(300.0, 1);
        let c = ctx(360.0, 100.0);
        let mut flags = 0;
        for _ in 0..69 {
            boss.step(&c);
            let state = boss.state();
            assert_eq!(state.warning(), state.mode.is_tell());
            flags += u32::from(state.warning());
        }
        assert_eq!(flags, 36, "one pound cycle is 36 + 9 + 24 ticks");
        assert!(!BossMode::Roar.is_tell() && !BossMode::PoundStrike.is_tell());
        assert!(!BossMode::Pinned.is_tell());
    }

    #[test]
    fn the_charge_runs_to_the_wall_crashes_on_both_sides_and_stuns_by_phase() {
        for (facing, wall, pivot) in [(-1i8, "left", 130.0f32), (1, "right", 510.0)] {
            let mut boss = engaged(if facing < 0 { 400.0 } else { 240.0 }, facing);
            // Carl far away on the charge side, high up so nothing is hit.
            let target = if facing < 0 { 60.0 } else { 600.0 };
            let mut c = ctx(target, 330.0);
            c.heroes[0].hurt.bottom = FLOOR + 90.0;
            c.heroes[1].present = false;
            let mut crash = None;
            for t in 0..400 {
                let out = boss.step(&c);
                if out
                    .events
                    .iter()
                    .any(|e| matches!(e, CombatEvent::BossCrash { wall: w, .. } if *w == facing))
                {
                    crash = Some(t);
                }
            }
            assert!(crash.is_some(), "{wall} wall crash");
            let _ = pivot;
        }
        // Stunned at the crash pivot for the phase's stun.
        let mut boss = engaged(400.0, -1);
        let mut c = ctx(60.0, 330.0);
        c.heroes[0].hurt.bottom = FLOOR + 90.0;
        c.heroes[1].present = false;
        let mut stun = 0;
        for _ in 0..400 {
            boss.step(&c);
            if boss.mode == BossMode::Stun {
                stun += 1;
                assert_eq!(boss.x, 130.0);
            }
        }
        assert!(stun >= 90, "stun {stun}");
    }

    #[test]
    fn the_ram_box_hits_a_standing_donut_and_a_jumper_clears_it() {
        let d = data();
        let mut boss = engaged(400.0, -1);
        // Donut's 20x24 box standing in the path, Carl far behind.
        let mut c = ctx(600.0, 250.0);
        c.heroes[1].hurt = Rect {
            left: 240.0,
            bottom: FLOOR,
            width: 20.0,
            height: 24.0,
        };
        let outs = run(&mut boss, &c, 120);
        let hits: Vec<_> = outs.iter().flat_map(|o| &o.blows).collect();
        assert_eq!(hits.len(), 1, "once per charge");
        assert_eq!(hits[0].hero, HeroId::Donut);
        assert_eq!(hits[0].damage, d.charge.damage);
        // The same hero 80 px up is above the 72 px ram box.
        let mut boss = engaged(400.0, -1);
        c.heroes[1].hurt.bottom = FLOOR + 80.0;
        let outs = run(&mut boss, &c, 120);
        assert!(outs.iter().all(|o| o.blows.is_empty()));
    }

    #[test]
    fn the_body_is_not_solid_while_he_runs() {
        let mut boss = engaged(400.0, -1);
        let mut c = ctx(60.0, 330.0);
        c.heroes[0].hurt.bottom = FLOOR + 90.0;
        c.heroes[1].present = false;
        let mut ran = 0;
        for _ in 0..200 {
            boss.step(&c);
            if boss.mode == BossMode::Charge {
                ran += 1;
                assert!(boss.solid_body().is_none());
                assert!(boss.hurtbox().is_some());
            }
        }
        assert!(ran > 10);
    }

    #[test]
    fn a_hero_inside_the_body_is_pushed_out_the_nearer_way_and_never_into_a_wall() {
        let mut boss = engaged(300.0, 1);
        // Carl overlapping the body from the front by 10 px.
        let mut c = ctx(300.0 + 44.0 + 2.0, 560.0);
        let out = boss.step(&c);
        let (hero, x) = out.shifts[0];
        assert_eq!(hero, HeroId::Carl);
        assert!(x > c.heroes[0].x - 0.01, "pushed forward (nearer edge)");
        assert!(x - c.heroes[0].x <= data().body.push_px_per_tick + 1e-3);
        // A hero overlapping near the left wall with < 28 px of room goes right.
        let mut boss = engaged(76.0, 1);
        c = ctx(76.0 - 20.0, 560.0);
        let out = boss.step(&c);
        let (_, x) = out
            .shifts
            .iter()
            .copied()
            .find(|(h, _)| *h == HeroId::Carl)
            .unwrap();
        assert!(
            x > c.heroes[0].x,
            "the room on the wall side is under 28 px"
        );
    }

    #[test]
    fn a_hero_standing_on_top_of_the_body_is_not_pushed() {
        let mut boss = engaged(300.0, 1);
        let mut c = ctx(310.0, 560.0);
        c.heroes[0].foot_y = FLOOR + 101.0;
        let out = boss.step(&c);
        assert!(out.shifts.is_empty());
    }

    #[test]
    fn targeting_picks_the_nearest_with_hysteresis_and_the_ai_bias() {
        let mut boss = engaged(300.0, 1);
        boss.update_target(&ctx(420.0, 560.0));
        assert_eq!(boss.target, Some(HeroId::Carl));
        // Donut closer by 20 px: inside the 24 px hysteresis, no switch.
        boss.update_target(&ctx(420.0, 400.0));
        assert_eq!(boss.target, Some(HeroId::Carl));
        // Closer by 30 px: switches.
        boss.update_target(&ctx(420.0, 390.0));
        assert_eq!(boss.target, Some(HeroId::Donut));
        // Solo: the bot counts 40 px farther.
        let mut boss = engaged(300.0, 1);
        let mut c = ctx(420.0, 400.0);
        c.solo = true;
        c.heroes[1].bot = true;
        boss.update_target(&c);
        assert_eq!(boss.target, Some(HeroId::Carl));
    }

    #[test]
    fn the_target_is_locked_through_an_attack() {
        let mut boss = engaged(300.0, 1);
        // Carl is the target and in pound range; Donut steps much closer
        // mid-tell: the lock holds until the recovery ends.
        run(&mut boss, &ctx(360.0, 560.0), 2);
        assert_eq!(boss.target, Some(HeroId::Carl));
        for _ in 0..68 {
            boss.step(&ctx(560.0, 330.0));
            if boss.mode != BossMode::Neutral {
                assert_eq!(boss.target, Some(HeroId::Carl));
            }
        }
    }

    #[test]
    fn a_hero_on_the_ledge_is_ignored_but_a_jumper_over_the_floor_is_not() {
        let mut data = data().clone();
        data.rack.right.enabled = true;
        let data: &'static BossData = Box::leak(Box::new(data));
        let mut boss = Boss::new(data, FLOOR);
        boss.gate = Gate::Closed;
        boss.mode = BossMode::Neutral;
        boss.x = 300.0;
        let mut c = ctx(450.0, 360.0);
        c.heroes[0].foot_y = FLOOR + 104.0;
        boss.update_target(&c);
        assert_eq!(
            boss.target,
            Some(HeroId::Donut),
            "Carl on the ledge is skipped"
        );
        // Airborne over the floor, not over the ledge: still reachable.
        let mut c = ctx(200.0, 560.0);
        c.heroes[0].foot_y = FLOOR + 60.0;
        c.heroes[0].grounded = false;
        let mut boss = Boss::new(data, FLOOR);
        boss.gate = Gate::Closed;
        boss.mode = BossMode::Neutral;
        boss.x = 300.0;
        boss.update_target(&c);
        assert_eq!(boss.target, Some(HeroId::Carl));
        // Nobody reachable: he walks to the centre.
        let mut c = ctx(450.0, 460.0);
        c.heroes[0].foot_y = FLOOR + 104.0;
        c.heroes[1].foot_y = FLOOR + 104.0;
        let mut boss = Boss::new(data, FLOOR);
        boss.gate = Gate::Closed;
        boss.mode = BossMode::Neutral;
        boss.x = 500.0;
        run(&mut boss, &c, 30);
        assert!(boss.x < 500.0);
    }

    #[test]
    fn poise_flinches_on_the_fourth_neutral_damage_only() {
        let mut boss = engaged(300.0, 1);
        for _ in 0..3 {
            assert!(!hit(&mut boss, 1).unwrap().flinched);
        }
        assert_eq!(boss.mode, BossMode::Neutral);
        assert!(hit(&mut boss, 1).unwrap().flinched);
        assert_eq!(boss.mode, BossMode::Flinch);
        // Free damage in a flinch never adds poise.
        for _ in 0..6 {
            assert!(!hit(&mut boss, 1).unwrap().flinched);
        }
        // Old poise leaves the 120-tick window.
        let mut boss = engaged(300.0, 1);
        for _ in 0..3 {
            hit(&mut boss, 1);
        }
        run(&mut boss, &ctx(600.0, 580.0), 125);
        assert!(!hit(&mut boss, 1).unwrap().flinched);
    }

    /// CP-Q40: a reinforced punch adds twice the poise (jab + haymaker = 4 of the
    /// 4 the flinch needs); the plain pair adds 2 and flinches nothing. The armour
    /// itself is never pierced: in a tell the same four poise change nothing.
    #[test]
    fn a_reinforced_combo_flinches_a_neutral_boss_and_never_an_armoured_one() {
        use crate::carl_prog::CarlProgTuning;
        use crate::combat::{CARL_HAYMAKER, CARL_JAB};
        let t = CarlProgTuning::active();
        let punch = |boss: &mut Boss, poise: u16| {
            let mut events = Vec::new();
            boss.apply_hit(
                BossHit {
                    damage: 1,
                    poise,
                    player: 0,
                    hit_stop: 0,
                },
                &mut events,
                1,
            )
            .unwrap()
        };
        let mut plain = engaged(300.0, 1);
        assert!(!punch(&mut plain, u16::from(CARL_JAB.damage)).flinched);
        assert!(!punch(&mut plain, u16::from(CARL_HAYMAKER.damage)).flinched);
        assert_eq!(plain.mode, BossMode::Neutral, "a plain pair adds 2 of 4");
        // CP-FU-D: the state line's poise number shows the doubling.
        assert_eq!(plain.poise_now(), 2, "a plain pair counts 1 + 1");
        let mut boss = engaged(300.0, 1);
        assert!(!punch(&mut boss, t.boss_poise(&CARL_JAB)).flinched);
        assert_eq!(boss.poise_now(), 2, "a reinforced jab counts 2");
        assert!(punch(&mut boss, t.boss_poise(&CARL_HAYMAKER)).flinched);
        assert_eq!(boss.mode, BossMode::Flinch, "one combo is four poise");
        assert_eq!(boss.poise_now(), 0, "the flinch spends it");
        let mut tell = engaged(300.0, 1);
        tell.enter(BossMode::PoundTell);
        for _ in 0..3 {
            punch(&mut tell, t.boss_poise(&CARL_HAYMAKER));
        }
        assert_eq!(tell.mode, BossMode::PoundTell, "the armour holds");
    }

    /// CP-Q41: a weighted strut hit counts its weight (the reinforced haymaker 2),
    /// a jab and a haymaker drop a fresh rack in one combo, and the count never
    /// passes the hits the strut needs.
    #[test]
    fn a_weighted_strut_hit_counts_its_weight() {
        let fist = Rect {
            left: 194.0,
            bottom: FLOOR + 29.0,
            width: 6.0,
            height: 6.0,
        };
        let needed = data().rack.strut_hits;
        let mut boss = Boss::new(data(), FLOOR);
        boss.gate = Gate::Closed;
        let mut events = Vec::new();
        boss.strike_strut_weighted(1, fist, 0, 5, 1, &mut events);
        assert_eq!(boss.racks[0].state, RackState::Idle { hits: 1 });
        boss.strike_strut_weighted(2, fist, 0, 6, 2, &mut events);
        assert!(
            matches!(boss.racks[0].state, RackState::Falling { .. }),
            "{needed} hits"
        );
        let mut over = Boss::new(data(), FLOOR);
        over.gate = Gate::Closed;
        let mut events = Vec::new();
        over.strike_strut_weighted(1, fist, 0, 5, 9, &mut events);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CombatEvent::RackHit { hits, .. } if *hits == needed))
        );
        // A weight of 1 is exactly strike_strut.
        let (mut a, mut b) = (Boss::new(data(), FLOOR), Boss::new(data(), FLOOR));
        (a.gate, b.gate) = (Gate::Closed, Gate::Closed);
        let (mut ea, mut eb) = (Vec::new(), Vec::new());
        a.strike_strut(1, fist, 0, 5, &mut ea);
        b.strike_strut_weighted(1, fist, 0, 5, 1, &mut eb);
        assert_eq!((a.racks[0].state, ea), (b.racks[0].state, eb));
    }

    #[test]
    fn super_armour_never_interrupts_and_free_windows_take_free_damage() {
        for mode in [
            BossMode::PoundTell,
            BossMode::PoundStrike,
            BossMode::ChargeTell,
            BossMode::Charge,
            BossMode::Roar,
        ] {
            let mut boss = engaged(300.0, 1);
            boss.enter(mode);
            for _ in 0..8 {
                hit(&mut boss, 1);
            }
            assert_eq!(boss.mode, mode, "{mode:?} is armoured");
            assert!(mode.armoured());
        }
        for mode in [BossMode::PoundRecover, BossMode::Stun, BossMode::Pinned] {
            assert!(!mode.armoured());
            let mut boss = engaged(300.0, 1);
            boss.enter(mode);
            let before = boss.hp;
            hit(&mut boss, 1);
            assert_eq!(before - boss.hp, 1);
            assert_eq!(boss.mode, mode, "no poise flinch in {mode:?}");
        }
    }

    #[test]
    fn each_threshold_gives_exactly_one_roar_and_the_phase_advances() {
        let mut boss = engaged(300.0, 1);
        boss.hp = 24;
        let c = ctx(600.0, 580.0);
        let outs = run(&mut boss, &c, 200);
        let phases: Vec<u8> = outs
            .iter()
            .flat_map(|o| &o.events)
            .filter_map(|e| match e {
                CombatEvent::BossPhase { phase, .. } => Some(*phase),
                _ => None,
            })
            .collect();
        assert_eq!(phases, vec![2]);
        boss.hp = 12;
        let outs = run(&mut boss, &c, 200);
        let phases: Vec<u8> = outs
            .iter()
            .flat_map(|o| &o.events)
            .filter_map(|e| match e {
                CombatEvent::BossPhase { phase, .. } => Some(*phase),
                _ => None,
            })
            .collect();
        assert_eq!(phases, vec![3]);
        assert_eq!(boss.phase, 3);
    }

    #[test]
    fn the_roar_pushes_heroes_within_72_px_back_48_and_is_queued_not_cutting() {
        let mut boss = engaged(300.0, 1);
        boss.hp = 24;
        let out = boss.step(&ctx(350.0, 560.0));
        let (hero, x) = out.shifts[0];
        assert_eq!(hero, HeroId::Carl);
        assert_eq!(x, 398.0);
        assert_eq!(boss.mode, BossMode::Roar);
        // Queued behind a stun: it never shortens it.
        let mut boss = engaged(300.0, 1);
        boss.enter(BossMode::Stun);
        boss.hp = 20;
        let c = ctx(600.0, 580.0);
        let mut stun = 0;
        let mut stun_over = false;
        let mut roar_at = None;
        for t in 0..300 {
            boss.step(&c);
            if boss.mode == BossMode::Stun && !stun_over {
                stun += 1;
            } else if stun > 0 {
                stun_over = true;
            }
            if boss.mode == BossMode::Roar && roar_at.is_none() {
                roar_at = Some(t);
            }
        }
        // The step that ends the stun is its 90th tick.
        assert_eq!(stun + 1, 90);
        assert!(roar_at.unwrap() >= 90);
    }

    #[test]
    fn phase_two_pounds_add_two_ripples_and_phase_three_doubles_the_pound() {
        let mut boss = engaged(300.0, 1);
        boss.phase = 2;
        boss.roars_done = 1;
        let c = ctx(360.0, 560.0);
        let mut max_ripples = 0;
        for _ in 0..60 {
            boss.step(&c);
            max_ripples = max_ripples.max(boss.ripple_count());
        }
        assert_eq!(max_ripples, 2);
        let mut boss = engaged(300.0, 1);
        boss.phase = 3;
        boss.roars_done = 2;
        let mut strikes = Vec::new();
        for _ in 0..200 {
            let out = boss.step(&c);
            for e in &out.events {
                if let CombatEvent::BossStrike { mv, .. } = e {
                    strikes.push(*mv);
                }
            }
        }
        assert_eq!(&strikes[..2], &[BossMove::Pound, BossMove::SecondPound]);
    }

    #[test]
    fn a_ripple_hurts_for_one_and_a_jumper_clears_it() {
        let mut boss = engaged(300.0, 1);
        boss.phase = 2;
        boss.roars_done = 1;
        // Donut stands 60 px behind the boss' pound origin on the ripple path.
        let mut c = ctx(356.0, 240.0);
        c.heroes[0].hurt.bottom = FLOOR + 60.0;
        let outs = run(&mut boss, &c, 120);
        let ripple_hits: Vec<_> = outs
            .iter()
            .flat_map(|o| &o.blows)
            .filter(|b| b.damage == 1)
            .collect();
        assert_eq!(ripple_hits.len(), 1);
        assert_eq!(ripple_hits[0].hero, HeroId::Donut);
    }

    #[test]
    fn cp_c_a_blast_drops_the_rack_outright_only_while_it_is_armed_and_intact() {
        // Armed (gate closed), enabled, intact: the strut is a blast footprint.
        let mut boss = engaged(130.0, -1);
        assert_eq!(boss.strut_blast_footprint(), Some(boss.strut_rect()));
        let mut events = Vec::new();
        assert!(boss.blast_drop_strut(1, 9, &mut events));
        assert_eq!(
            events,
            vec![CombatEvent::RackDropped {
                tick: 9,
                rack: 0,
                player: 1
            }]
        );
        assert!(matches!(
            boss.racks[0].state,
            RackState::Falling { player: 1, .. }
        ));
        // Falling already: nothing more, and no second event.
        assert_eq!(boss.strut_blast_footprint(), None);
        assert!(!boss.blast_drop_strut(1, 10, &mut events));
        assert_eq!(events.len(), 1);
        // A partly hit rack is dropped outright too (hits so far do not matter).
        let mut hit_once = engaged(130.0, -1);
        hit_once.racks[0].state = RackState::Idle { hits: 1 };
        assert!(hit_once.blast_drop_strut(0, 5, &mut Vec::new()));
        // Not armed before the engage, and not after the boss is down.
        let mut open = Boss::new(data(), FLOOR);
        assert_eq!(open.strut_blast_footprint(), None);
        assert!(!open.blast_drop_strut(0, 5, &mut Vec::new()));
        let mut released = engaged(130.0, -1);
        released.gate = Gate::Released;
        assert_eq!(released.strut_blast_footprint(), None);
        // The rack off (the no-rack plan): the strut is not there.
        let mut off = engaged(130.0, -1);
        off.racks[0].enabled = false;
        assert_eq!(off.strut_blast_footprint(), None);
    }

    #[test]
    fn the_left_rack_falls_after_three_separate_hits_and_crushes_a_stunned_boss() {
        let mut boss = engaged(130.0, -1);
        boss.enter(BossMode::Stun);
        let fist = Rect {
            left: 194.0,
            bottom: FLOOR + 29.0,
            width: 6.0,
            height: 6.0,
        };
        let mut events = Vec::new();
        for instance in [1, 1, 2, 3] {
            boss.strike_strut(instance, fist, 0, 5, &mut events);
        }
        let hits = events
            .iter()
            .filter(|e| matches!(e, CombatEvent::RackHit { .. }))
            .count();
        assert_eq!(hits, 3, "the same move instance counts once");
        assert!(events.iter().any(|e| matches!(
            e,
            CombatEvent::RackDropped {
                rack: 0,
                player: 0,
                ..
            }
        )));
        let c = ctx(300.0, 560.0);
        let mut landed = None;
        let hp = boss.hp;
        for t in 1..=40 {
            let out = boss.step(&c);
            if out.events.iter().any(|e| {
                matches!(
                    e,
                    CombatEvent::RackLanded {
                        rack: 0,
                        crushed: true,
                        ..
                    }
                )
            }) {
                landed = Some(t);
            }
        }
        assert_eq!(landed, Some(37), "36 ticks after the drop tick");
        assert_eq!(hp - boss.hp, 4);
        assert_eq!(boss.mode, BossMode::Pinned);
    }

    #[test]
    fn the_rack_misses_a_boss_outside_the_footprint_but_still_crushes_heroes_in_it() {
        let mut boss = engaged(400.0, -1);
        boss.racks[0].state = RackState::Falling {
            ticks: data().rack.fall_ticks,
            player: 0,
        };
        let mut c = ctx(123.0, 560.0);
        c.heroes[0].hurt.left = 111.0;
        let out = boss.step(&c);
        assert!(
            out.events
                .iter()
                .any(|e| matches!(e, CombatEvent::RackLanded { crushed: false, .. }))
        );
        assert_eq!(boss.hp, data().hp_max);
        assert_eq!(out.blows.len(), 1);
        assert_eq!(out.blows[0].damage, data().rack.hero_damage);
    }

    #[test]
    fn the_footprint_is_centred_on_the_crash_body_and_covers_it_on_both_walls() {
        let d = data();
        let mut boss = engaged(130.0, -1);
        for (facing, centre) in [(-1i8, 123.0f32), (1, 517.0)] {
            assert_eq!(d.footprint_centre(facing), centre);
            let mut stunned = engaged(d.crash_pivot(facing), facing);
            stunned.enter(BossMode::Stun);
            let body = stunned.hurtbox().unwrap();
            let index = usize::from(facing > 0);
            let foot = stunned.footprint(index);
            assert!(foot.overlaps(body));
            assert!(foot.left <= body.left + 0.01 || foot.left <= body.left + 20.0);
            assert_eq!(foot.width, 80.0);
        }
        boss.enter(BossMode::Neutral);
    }

    #[test]
    fn the_right_rack_is_disabled_by_default_and_drops_after_the_dwell_when_enabled() {
        let boss = Boss::new(data(), FLOOR);
        assert!(!boss.racks[1].enabled);
        let mut on = data().clone();
        on.rack.right.enabled = true;
        let on: &'static BossData = Box::leak(Box::new(on));
        let mut boss = Boss::new(on, FLOOR);
        boss.gate = Gate::Closed;
        boss.mode = BossMode::Neutral;
        boss.x = 200.0;
        let mut c = ctx(560.0, 450.0);
        c.heroes[1].foot_y = FLOOR + 104.0;
        c.heroes[0].hurt.bottom = FLOOR + 90.0;
        let mut dropped = None;
        for t in 1..=20 {
            let out = boss.step(&c);
            if out
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::RackDropped { rack: 1, .. }))
            {
                dropped = Some(t);
            }
        }
        assert_eq!(dropped, Some(12));
    }

    #[test]
    fn a_lethal_rack_credits_the_dropper_and_opens_the_gate() {
        let mut boss = engaged(130.0, -1);
        boss.enter(BossMode::Stun);
        boss.hp = 3;
        boss.racks[0].state = RackState::Falling {
            ticks: data().rack.fall_ticks,
            player: 0,
        };
        let out = boss.step(&ctx(300.0, 560.0));
        assert_eq!(out.crushed_by, Some((0, 0)));
        assert_eq!(boss.mode, BossMode::Defeated);
        assert!(boss.released());
        assert!(boss.hurtbox().is_none() && boss.solid_body().is_none());
    }

    #[test]
    fn fill_px_maps_the_hp_to_234_and_never_hides_a_living_boss() {
        let state = |hp| BossState {
            hp,
            hp_max: 36,
            mode: BossMode::Neutral,
            phase: 1,
        };
        assert_eq!(state(36).fill_px(), 234);
        assert_eq!(state(18).fill_px(), 117);
        assert!(state(1).fill_px() >= 7, "{}", state(1).fill_px());
        assert_eq!(state(0).fill_px(), 0);
    }

    #[test]
    fn a_zero_damage_glance_adds_poise_but_emits_no_hurt_event() {
        let mut boss = engaged(300.0, 1);
        let mut events = Vec::new();
        let result = boss.apply_hit(
            BossHit {
                damage: 0,
                poise: 1,
                player: 1,
                hit_stop: 0,
            },
            &mut events,
            1,
        );
        assert_eq!(result.map(|r| r.hp_left), Some(boss.hp));
        assert!(events.is_empty(), "no BossHurt for 0 damage: {events:?}");
        // A real hit still emits it.
        boss.apply_hit(
            BossHit {
                damage: 1,
                poise: 1,
                player: 1,
                hit_stop: 0,
            },
            &mut events,
            2,
        );
        assert!(matches!(
            events[..],
            [CombatEvent::BossHurt { damage: 1, .. }]
        ));
    }

    #[test]
    fn hit_stop_freezes_the_boss_clock_not_the_racks() {
        let mut boss = engaged(300.0, 1);
        let mut events = Vec::new();
        boss.apply_hit(
            BossHit {
                damage: 1,
                poise: 1,
                player: 0,
                hit_stop: 4,
            },
            &mut events,
            1,
        );
        let c = ctx(420.0, 580.0);
        let x = boss.x;
        run(&mut boss, &c, 4);
        assert_eq!(boss.x, x);
        run(&mut boss, &c, 4);
        assert_ne!(boss.x, x);
    }

    #[test]
    fn ram_arrival_ticks_counts_down_toward_a_hero_in_the_path() {
        let mut boss = engaged(400.0, -1);
        boss.charge_dir = -1;
        boss.enter(BossMode::Charge);
        let far = boss.ram_arrival_ticks(100.0).unwrap();
        boss.x -= 50.0;
        let near = boss.ram_arrival_ticks(100.0).unwrap();
        assert!(near < far);
        assert!(boss.ram_arrival_ticks(560.0).is_none(), "behind him");
    }

    #[test]
    fn the_boss_is_deterministic_and_hashes_every_field() {
        let run_once = || {
            let mut boss = engaged(300.0, 1);
            let c = ctx(360.0, 560.0);
            let mut hash = 0u64;
            for _ in 0..200 {
                boss.step(&c);
                boss.mix_into(&mut |v| hash = hash.rotate_left(5) ^ v);
            }
            hash
        };
        assert_eq!(run_once(), run_once());
        let a = engaged(300.0, 1);
        let mut b = engaged(300.0, 1);
        b.hp -= 1;
        let (mut ha, mut hb) = (0u64, 0u64);
        a.mix_into(&mut |v| ha = ha.rotate_left(5) ^ v);
        b.mix_into(&mut |v| hb = hb.rotate_left(5) ^ v);
        assert_ne!(ha, hb);
    }

    #[test]
    fn the_rack_crush_tests_the_hero_pivot_not_the_hurtbox() {
        // The footprint is x 83..163: Carl baiting at 165 is outside it (his
        // hurtbox reaches 153), at 160 he is inside.
        for (x, hurt) in [
            (165.0f32, false),
            (160.0, true),
            (163.0, false),
            (83.0, true),
        ] {
            let mut boss = engaged(400.0, -1);
            boss.racks[0].state = RackState::Falling {
                ticks: data().rack.fall_ticks,
                player: 0,
            };
            let out = boss.step(&ctx(x, 560.0));
            assert_eq!(!out.blows.is_empty(), hurt, "hero pivot at {x}");
        }
        // Feet above the stack's top are not under it either.
        let mut boss = engaged(400.0, -1);
        boss.racks[0].state = RackState::Falling {
            ticks: data().rack.fall_ticks,
            player: 0,
        };
        let mut c = ctx(120.0, 560.0);
        c.heroes[0].foot_y = FLOOR + 40.0;
        assert!(boss.step(&c).blows.is_empty());
    }

    #[test]
    fn strut_hits_before_the_gate_closes_do_nothing() {
        let fist = Rect {
            left: 194.0,
            bottom: FLOOR + 29.0,
            width: 6.0,
            height: 6.0,
        };
        let mut boss = Boss::new(data(), FLOOR);
        assert!(!boss.gate_closed());
        let mut events = Vec::new();
        for instance in 1..=4 {
            boss.strike_strut(instance, fist, 0, 5, &mut events);
        }
        assert!(events.is_empty(), "no hit logged: {events:?}");
        assert_eq!(boss.racks[0].state, RackState::Idle { hits: 0 });
        boss.gate = Gate::Closed;
        boss.strike_strut(5, fist, 0, 6, &mut events);
        assert_eq!(boss.racks[0].state, RackState::Idle { hits: 1 });
    }

    #[test]
    fn the_charge_threat_carries_the_rest_of_the_tell() {
        let mut boss = engaged(300.0, 1);
        boss.enter(BossMode::ChargeTell);
        boss.mode_tick = 10;
        let Some(BossThreat::Charge {
            tell_left, running, ..
        }) = boss.threat()
        else {
            panic!("a charge tell is a charge threat");
        };
        assert!(!running);
        assert_eq!(tell_left, data().charge.tell_by_phase[0] - 10);
        boss.enter(BossMode::Charge);
        let Some(BossThreat::Charge {
            tell_left, running, ..
        }) = boss.threat()
        else {
            panic!("a charge run is a charge threat");
        };
        assert!(running);
        assert_eq!(tell_left, 0);
    }

    /// FA-Q09 (F01-FU-B10): a charge is identified by the clock of its tell's
    /// first tick, kept through the run; a chained charge gets a new one, and the
    /// key is not part of the hash (a copy of the hashed clock).
    #[test]
    fn a_charge_threat_carries_the_clock_its_tell_began() {
        let started_of = |boss: &Boss| match boss.threat() {
            Some(BossThreat::Charge { started, .. }) => started,
            other => panic!("{other:?}"),
        };
        let mut boss = engaged(300.0, 1);
        boss.clock = 40;
        boss.enter(BossMode::ChargeTell);
        assert_eq!(started_of(&boss), 40);
        boss.clock = 55;
        boss.enter(BossMode::Charge);
        assert_eq!(started_of(&boss), 40, "the run is the same charge");
        boss.clock = 70;
        boss.enter(BossMode::ChargeTell);
        assert_eq!(started_of(&boss), 70, "a chained charge is a new one");
        let mut copy = boss.clone();
        copy.charge_started = 40;
        assert_eq!(hash_of(&copy), hash_of(&boss), "not hashed");
    }

    fn hash_of(boss: &Boss) -> u64 {
        let mut h = 0u64;
        boss.mix_into(&mut |v| h = h.rotate_left(5) ^ v.wrapping_mul(0x9e37_79b9_7f4a_7c15));
        h
    }

    #[test]
    fn every_field_of_the_boss_reaches_the_hash() {
        let base = || {
            let mut boss = engaged(300.0, 1);
            boss.ripples.push(Ripple {
                x: 340.0,
                dir: 1,
                travelled: 10.0,
                hit: [false; 2],
            });
            boss.poise.push((5, 1));
            boss
        };
        let reference = hash_of(&base());
        type Mutation = (&'static str, fn(&mut Boss));
        let mutations: &[Mutation] = &[
            ("clock", |b| b.clock += 1),
            ("hp", |b| b.hp -= 1),
            ("phase", |b| b.phase += 1),
            ("mode", |b| b.mode = BossMode::Stun),
            ("mode_tick", |b| b.mode_tick += 1),
            ("x", |b| b.x += 1.0),
            ("facing", |b| b.facing = -1),
            ("hit_stop", |b| b.hit_stop = 3),
            ("target", |b| b.target = Some(HeroId::Donut)),
            ("poise at", |b| b.poise[0].0 += 1),
            ("poise amount", |b| b.poise[0].1 += 1),
            ("poise length", |b| b.poise.push((6, 1))),
            ("cooldown", |b| b.cooldown += 1),
            ("roars_done", |b| b.roars_done += 1),
            ("charge_dir", |b| b.charge_dir = -1),
            ("charge_hit", |b| b.charge_hit[1] = true),
            ("strike_hit", |b| b.strike_hit[0] = true),
            ("second_pending", |b| b.second_pending = true),
            ("pin_credit", |b| b.pin_credit = 1),
            ("ripple x", |b| b.ripples[0].x += 1.0),
            ("ripple dir", |b| b.ripples[0].dir = -1),
            ("ripple travelled", |b| b.ripples[0].travelled += 1.0),
            ("ripple hit", |b| b.ripples[0].hit[0] = true),
            ("ripple length", |b| b.ripples.clear()),
            ("rack enabled", |b| b.racks[1].enabled = true),
            ("rack instance", |b| b.racks[0].last_instance = 9),
            ("rack hits", |b| {
                b.racks[0].state = RackState::Idle { hits: 1 }
            }),
            ("rack falling", |b| {
                b.racks[0].state = RackState::Falling {
                    ticks: 2,
                    player: 0,
                }
            }),
            ("rack landed", |b| b.racks[0].state = RackState::Landed),
            ("gate", |b| b.gate = Gate::Released),
            ("cause", |b| b.defeated_cause_rack = true),
        ];
        for (name, mutate) in mutations {
            let mut boss = base();
            mutate(&mut boss);
            assert_ne!(hash_of(&boss), reference, "{name} is not hashed");
        }
    }
}
