//! CORRAL-EVAL: the STATIONARY-DONUT evaluator of CORRAL-AUTO-01 on the real
//! party simulation.
//!
//! It answers one question with numbers: does a Donut who stands still and
//! presses Magic Missile trivialise the corral llamas? Every run drives the
//! real [`PartyFixtureSim`] (the live party step order, real mana, collisions,
//! the llama state machine with its rush) with legal per-slot raw inputs, one
//! tick at a time.
//!
//! Hard rules, enforced by construction and by tests:
//! - **Setup-only writes.** The one place the evaluator writes game state is
//!   [`build`], at tick 0: Donut's start column, surface and facing, each
//!   llama's attack phase `(cooldown, rush_cooldown)` and the parked seat.
//!   After that the only mutation is [`PartyFixtureSim::step_slots`]. It never
//!   calls `missile_hits_llama`, never writes mana or health, and never moves a
//!   hero after tick 0.
//! - **Construction path (review L1).** The state is built with
//!   `CombatState::new(..).with_cf02().with_donut(..)` and
//!   `PartyFixtureSim::from_parts`. The corral stages reuse the `corral_m1`
//!   and `corral_m2` room fixtures, whose opt-ins come from the exact-name hook
//!   of [`crate::corral_fixtures`]; no fixture is registered with
//!   `PartyFixture::all()`, so no golden or committed replay changes.
//!
//! **Stages.** CORRAL-C runs the Infernal Corral itself on the room kit
//! (`f01_corral.room.json`): [`Stage::CorralM1`] (the first llama, the
//! positive control) and [`Stage::CorralM2`] (rush llama B on the pen floor,
//! rush llama A on the dock behind the 80 px bumper). The CORRAL-EVAL core
//! baselines ([`Stage::BASELINE`]: today's llamas on the corridor kit, no
//! cover) stay as the "before" half of the before-and-after report.
//!
//! **Variants** (spec section 11): the starts, both facings, the attack phases
//! P0..P2 written at tick 0 as per-llama `(cooldown, rush_cooldown)`, the
//! reflex (`off` / `reflex`, DM-B: the gate is decided on `reflex`) and the
//! seat (the decisive `isolated` one parks Carl; the companion one is
//! reported). The partition variants (CORRAL-D, [`PartitionVariant`]) `absent`
//! (the cut path), `intact` (the shipped room) and `broken` (authored
//! pre-broken, the 16 px remnant standing) run on the M2 stage: the env runtime
//! of the real sidecar supplies the partition, and every start group is judged
//! per variant. Seeds do not vary gameplay, so the report counts **fixtures**,
//! never seeds.
//!
//! **Gate** ([`report::Gate`], asserted by the `corral_auto_*` tests from
//! CORRAL-C on): KC19 on the decisive variant, the CRL-Q35 geometric
//! invariant, the M1 positive control, one mobile Donut, one Carl melee and
//! one co-op victory, and the hard system failures.

mod evasion;
pub mod policy;
pub mod report;

pub use self::evasion::Evasion;

use std::path::Path;
use std::sync::OnceLock;

use crate::boss_scene::Pilot;
use crate::combat::{CombatEvent, EnemyAi, MoveId};
use crate::combat_scene::{BOLT_SPAWN_FORWARD, BOLT_SPAWN_UP, CombatState, Fixture, LlamaSpawn};
use crate::corral_fixtures::room;
use crate::corridor::SceneSim;
use crate::donut_mana::{DonutSetup, ManaEvent};
use crate::env_kit::EnvKit;
use crate::input::{InputFrame, RawInput, SlotInputs};
use crate::llama::LlamaMode;
use crate::llama_rush::{LLAMA_RUSH_SOURCE_ID, RushEnd, RushPhase};
use crate::party::{HeroId, PROVISIONAL_TUNING, PartyTuning, UncontrolledHero};
use crate::party_scene::{PartyFixture, PartyFixtureSim};
use crate::prototype::{
    BodyDrive, HERO_HALF_HEIGHT, Hero, feet_height_above_solid, step_hero_motion,
};

use self::policy::{Aim, HeroInputs, HeroObs, LlamaObs, Obs, Policy, PolicyId, Route};

/// Bump when a policy, a stage or a metric definition changes. 2: CORRAL-C
/// (the room stages, the parked seat, the aimed, turret and jump-cast
/// policies, the `(cooldown, rush_cooldown)` phases, the reflex variant, the
/// CRL-Q35 invariant and the system-failure checks). 3: CORRAL-D (the
/// partition variants and the partition-top and remnant-top starts).
pub const POLICY_VERSION: u32 = 3;
/// The isolated M2 attempt horizon (CORRAL-AUTO section 3): 30 s at 60 Hz.
pub const HORIZON_TICKS: u64 = 1800;
/// The corridor kit the baseline stages run on.
pub const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";
pub const VIEW_H: i32 = 360;
/// Carl's start column on every baseline stage.
pub const CARL_X: i32 = 400;
/// A start closer than this to a llama's column is not a natural one (the
/// llama body half-width is 23 px).
pub const START_CLEARANCE_PX: i32 = 48;
/// Donut's biscuit pool at the start of every run (the corral's M0 grants it).
pub const START_MANA: u8 = 8;

// ---------------------------------------------------------------- stages --

/// A stage. `M1`, `M2` and `M2Pincer` are the CORRAL-EVAL core baselines on
/// the corridor kit; `CorralM1` and `CorralM2` are the Infernal Corral.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Stage {
    /// One llama: the positive control (the first power fantasy).
    M1,
    /// Two llamas side by side ahead of Donut.
    M2,
    /// One llama ahead and one behind: facing and a reveal behind her.
    M2Pincer,
    /// Room 0, M1: the first llama at 848 (no rush, party targeting).
    CorralM1,
    /// Room 0, M2: B on the pen floor and A on the dock (both rush).
    CorralM2,
}

impl Stage {
    pub const BASELINE: [Stage; 3] = [Stage::M1, Stage::M2, Stage::M2Pincer];
    pub const CORRAL: [Stage; 2] = [Stage::CorralM1, Stage::CorralM2];
    pub const ALL: [Stage; 5] = [
        Stage::M1,
        Stage::M2,
        Stage::M2Pincer,
        Stage::CorralM1,
        Stage::CorralM2,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Stage::M1 => "corral_eval_m1",
            Stage::M2 => "corral_eval_m2",
            Stage::M2Pincer => "corral_eval_m2_pincer",
            Stage::CorralM1 => "corral_m1",
            Stage::CorralM2 => "corral_m2",
        }
    }

    pub const fn is_corral(self) -> bool {
        matches!(self, Stage::CorralM1 | Stage::CorralM2)
    }

    /// An M1 stage (one llama, the positive control).
    pub const fn is_m1(self) -> bool {
        matches!(self, Stage::M1 | Stage::CorralM1)
    }

    /// The llama columns.
    pub fn llama_xs(self) -> Vec<i32> {
        self.base().llamas.iter().map(|l| l.x).collect()
    }

    fn base(self) -> &'static Fixture {
        match self {
            Stage::M1 => &M1_BASE,
            Stage::M2 => &M2_BASE,
            Stage::M2Pincer => &PINCER_BASE,
            Stage::CorralM1 => room::m1_base(),
            Stage::CorralM2 => room::m2_base(),
        }
    }
}

const fn stage_fixture(
    name: &'static str,
    summary: &'static str,
    seed: u64,
    llamas: &'static [LlamaSpawn],
) -> Fixture {
    Fixture {
        name,
        summary,
        seed,
        carl_x: CARL_X,
        enemies: &[],
        shore_dx: None,
        boss: None,
        llamas,
        engineers: &[],
        charges: &[],
        script: &[],
        ticks: HORIZON_TICKS,
        room: crate::combat_scene::FixtureRoom::Corridor,
    }
}

const fn llama(x: i32) -> LlamaSpawn {
    LlamaSpawn {
        x,
        ai: EnemyAi::Chase,
    }
}

static M1_BASE: Fixture = stage_fixture(
    "corral_eval_m1",
    "CORRAL-EVAL baseline: one llama ahead (positive control)",
    0xC0AA_0001,
    &[llama(640)],
);
static M2_BASE: Fixture = stage_fixture(
    "corral_eval_m2",
    "CORRAL-EVAL baseline: two llamas ahead, no cover, no rush (M2-like)",
    0xC0AA_0002,
    &[llama(640), llama(720)],
);
static PINCER_BASE: Fixture = stage_fixture(
    "corral_eval_m2_pincer",
    "CORRAL-EVAL baseline: one llama ahead and one behind (facing and reveal)",
    0xC0AA_0003,
    &[llama(240), llama(640)],
);

// ------------------------------------------------------------- partition --

/// The corral's partition variant (spec section 11, CORRAL-D). Only the M2
/// stage has one; the baselines and M1 run [`Self::Absent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum PartitionVariant {
    /// No partition: the cut path (the sidecar's prop is not built).
    #[default]
    Absent,
    /// The shipped room: the 16 x 48 screen stands at tick 0.
    Intact,
    /// Authored pre-broken: the 16 px remnant stands (a tick-0 setup write).
    Broken,
}

impl PartitionVariant {
    pub const ALL: [PartitionVariant; 3] = [
        PartitionVariant::Absent,
        PartitionVariant::Intact,
        PartitionVariant::Broken,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            PartitionVariant::Absent => "absent",
            PartitionVariant::Intact => "intact",
            PartitionVariant::Broken => "broken",
        }
    }

    /// The run id's suffix; empty for `Absent`, so every run id that existed
    /// before CORRAL-D is unchanged.
    const fn id_suffix(self) -> &'static str {
        match self {
            PartitionVariant::Absent => "",
            PartitionVariant::Intact => ".pt-intact",
            PartitionVariant::Broken => ".pt-broken",
        }
    }
}

/// The partition's surfaces as starts: the top of the standing screen (48 px)
/// and the top of its remnant (16 px), both over the screen's 16 px column.
pub const PARTITION_TOP_RISE: i32 = 48;
pub const REMNANT_TOP_RISE: i32 = 16;

/// Whether `(x, rise)` is a start on one of the partition's own tops.
pub fn is_partition_surface(x: i32, rise: i32) -> bool {
    matches!(rise, PARTITION_TOP_RISE | REMNANT_TOP_RISE)
        && (PARTITION_X.0..PARTITION_X.1).contains(&x)
}

/// The partition's columns (the sidecar prop's `target`).
pub const PARTITION_X: (i32, i32) = (1392, 1408);

/// Whether the start `(x, rise)` exists under `variant`: the screen's top only
/// while it stands, the remnant's only once broken, and no floor column inside
/// the screen's footprint while it stands or its remnant lies.
pub fn start_exists(variant: PartitionVariant, x: i32, rise: i32) -> bool {
    let on_column = (PARTITION_X.0 - 10..PARTITION_X.1 + 10).contains(&x);
    match rise {
        PARTITION_TOP_RISE => variant == PartitionVariant::Intact && is_partition_surface(x, rise),
        REMNANT_TOP_RISE => variant == PartitionVariant::Broken && is_partition_surface(x, rise),
        0 => variant == PartitionVariant::Absent || !on_column,
        _ => true,
    }
}

// ----------------------------------------------------------------- seats --

/// Who plays and who stands in. In every Donut seat the tested hero is Donut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Seat {
    /// Solo, swapped to Donut on tick 1; Carl is the follow-and-jump bot that
    /// **never attacks** (the CORRAL-EVAL core decisive variant).
    SoloDonutBot,
    /// Solo, swapped to Donut; Carl is the companion AI of the live scene
    /// (`Cf02Assist`). **Companion-assisted** when it dealt any damage:
    /// reported, never counted as proof (KC19). The corral's `default`
    /// variant: Carl starts at 1040 in M2.
    SoloDonutAssist,
    /// Solo, swapped to Donut; Carl is **parked** (spec section 11, F3 and NB2:
    /// no bot, no companion, an idle seat at 880 in M2 or 300 in M1). The
    /// corral's decisive `isolated` variant.
    SoloDonutParked,
    /// Solo as Carl; Donut is the follow-and-jump bot (never attacks).
    SoloCarl,
    /// Two players: P1 Carl, P2 Donut.
    TwoPlayer,
}

impl Seat {
    pub const fn name(self) -> &'static str {
        match self {
            Seat::SoloDonutBot => "solo_donut_bot",
            Seat::SoloDonutAssist => "solo_donut_assist",
            Seat::SoloDonutParked => "solo_donut_parked",
            Seat::SoloCarl => "solo_carl",
            Seat::TwoPlayer => "two_player",
        }
    }

    /// Donut is the hero a human drives in this seat.
    pub const fn donut_is_player(self) -> bool {
        matches!(
            self,
            Seat::SoloDonutBot | Seat::SoloDonutAssist | Seat::SoloDonutParked | Seat::TwoPlayer
        )
    }

    /// The decisive seat of a stage: the parked Carl in the corral, the
    /// never-attacking bot on the baselines.
    pub const fn decisive(stage: Stage) -> Seat {
        if stage.is_corral() {
            Seat::SoloDonutParked
        } else {
            Seat::SoloDonutBot
        }
    }
}

const BOT_TUNING: PartyTuning = PROVISIONAL_TUNING.with_cf02_swap();
const ASSIST_TUNING: PartyTuning = PartyTuning {
    uncontrolled_hero: UncontrolledHero::Companion(
        crate::companion::profile::ProfileSetId::Cf02Assist,
    ),
    ..PROVISIONAL_TUNING
}
.with_cf02_swap();

/// The party fixture of one (stage, seat, reflex variant): the stage's base
/// with the seat's tuning and the variant's reflex profile.
fn fixture_for(stage: Stage, seat: Seat, evasion: Evasion) -> &'static PartyFixture {
    type Key = (Stage, Seat, Evasion);
    static TABLE: OnceLock<Vec<(Key, PartyFixture)>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let seats = [
            Seat::SoloDonutBot,
            Seat::SoloDonutAssist,
            Seat::SoloDonutParked,
            Seat::SoloCarl,
            Seat::TwoPlayer,
        ];
        let mut out = Vec::new();
        for stage in Stage::ALL {
            for seat in seats {
                for evasion in Evasion::ALL {
                    let mut tuning = if seat == Seat::SoloDonutAssist {
                        ASSIST_TUNING
                    } else {
                        BOT_TUNING
                    };
                    tuning.donut_reflex = evasion.profile();
                    out.push((
                        (stage, seat, evasion),
                        PartyFixture {
                            base: stage.base(),
                            two_player: seat == Seat::TwoPlayer,
                            tuning,
                            script: &[],
                            pilot: Pilot::None,
                            donut: DonutSetup::unlocked(START_MANA),
                        },
                    ));
                }
            }
        }
        out
    });
    &table
        .iter()
        .find(|(k, _)| *k == (stage, seat, evasion))
        .expect("every (stage, seat, evasion) is in the table")
        .1
}

// ------------------------------------------------------------- variants --

/// One llama's attack phase, written at tick 0 (F13, CRL-Q36):
/// `(cooldown, rush_cooldown)`. A llama without a rush profile ignores the
/// second value.
pub type Phase = (u16, u16);

/// CRL-Q36: the M2 phase variants P0..P2 as `[B, A]`.
pub const M2_PHASES: [[Phase; 2]; 3] = [[(0, 0), (45, 75)], [(45, 75), (90, 0)], [(90, 0), (0, 0)]];

/// One run: everything that distinguishes it. The run id is derived from it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunSpec {
    pub stage: Stage,
    pub seat: Seat,
    pub policy: PolicyId,
    pub evasion: Evasion,
    /// The tested hero's start column and the height of its start surface
    /// above the walking line (set at tick 0). In a Donut seat it is Donut's.
    pub donut_x: i32,
    pub donut_rise: i32,
    pub facing: i8,
    /// Each llama's attack phase at tick 0.
    pub phase: [Phase; 2],
    /// `stationary_jump_cast` only: Primary press tick minus Jump press tick.
    pub jump_offset: i8,
    pub horizon: u64,
    /// The corral's partition variant (CORRAL-D); `Absent` off the M2 stage.
    pub partition: PartitionVariant,
}

impl RunSpec {
    fn policy_label(&self) -> String {
        if self.policy == PolicyId::StationaryJumpCast {
            format!("{}.o{}", self.policy.name(), self.jump_offset)
        } else {
            self.policy.name().to_string()
        }
    }

    pub fn run_id(&self) -> String {
        format!(
            "{}.p{}-{}-{}-{}",
            self.group_id(),
            self.phase[0].0,
            self.phase[0].1,
            self.phase[1].0,
            self.phase[1].1,
        )
    }

    /// The start group: every phase variant of one (stage, seat, policy,
    /// evasion, column, surface, facing). KC19 judges a group, not a single run.
    pub fn group_id(&self) -> String {
        format!(
            "{}.{}.{}.{}.x{}.h{}.f{}{}",
            self.stage.name(),
            self.seat.name(),
            self.policy_label(),
            self.evasion.name(),
            self.donut_x,
            self.donut_rise,
            if self.facing >= 0 { "r" } else { "l" },
            self.partition.id_suffix(),
        )
    }
}

/// Why a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminal {
    /// Every llama defeated.
    Victory,
    /// A hero reached 0 health (the party's defeat, D1).
    HeroDefeat,
    /// The horizon elapsed.
    Timeout,
    /// Reserved: stuck-enemy detection is not measured (no definition of
    /// progress for a llama that legitimately holds); never emitted.
    Softlock,
    /// The start is not a legal one (no surface under the column).
    InvalidSetup,
    /// The variant cannot run (kept for a future pending variant).
    Blocked,
}

impl Terminal {
    pub const fn name(self) -> &'static str {
        match self {
            Terminal::Victory => "victory",
            Terminal::HeroDefeat => "hero_defeat",
            Terminal::Timeout => "timeout",
            Terminal::Softlock => "softlock",
            Terminal::InvalidSetup => "invalid_setup",
            Terminal::Blocked => "blocked",
        }
    }
}

/// The hard system failures of spec section 11, gate item 5, counted per run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystemFailures {
    /// A rush `HeroHurt` with no llama in its committed rush.
    pub rush_hit_before_commit: u32,
    /// A glob released by a llama with a rush running.
    pub glob_from_rush: u32,
    /// A rush or spit tell started with the llama's body outside the view.
    pub offscreen_tell: u32,
    /// A corral llama outside its leash or off its surface, any mode.
    pub left_leash_or_level: u32,
    /// A bolt hit the dock llama while a static Donut stood on the floor.
    pub dock_hit_from_floor: u32,
    /// A corral stage ran without llama party targeting.
    pub carl_only_targeting: u32,
}

impl SystemFailures {
    pub fn total(&self) -> u32 {
        self.rush_hit_before_commit
            + self.glob_from_rush
            + self.offscreen_tell
            + self.left_leash_or_level
            + self.dock_hit_from_floor
            + self.carl_only_targeting
    }

    fn describe(&self) -> String {
        format!(
            "rush_before_commit={} glob_from_rush={} offscreen_tell={} leash_or_level={} dock_hit_from_floor={} carl_only_targeting={}",
            self.rush_hit_before_commit,
            self.glob_from_rush,
            self.offscreen_tell,
            self.left_leash_or_level,
            self.dock_hit_from_floor,
            self.carl_only_targeting
        )
    }
}

/// The metrics of one run (CORRAL-AUTO section 4, what the schema can
/// attribute honestly; anything it cannot is absent, never faked).
#[derive(Debug, Clone, PartialEq)]
pub struct RunRecord {
    pub spec: RunSpec,
    pub run_id: String,
    pub terminal: Terminal,
    pub note: String,
    pub ticks: u64,
    pub donut_hp_start: u8,
    pub donut_hp_final: u8,
    pub carl_hp_start: u8,
    pub carl_hp_final: u8,
    pub mana_start: u8,
    pub mana_final: u8,
    pub mana_min: u8,
    pub mana_spent: u32,
    pub mana_regained: u32,
    /// Press edges the policy issued.
    pub presses: u32,
    /// Jump presses the jump-cast policies issued.
    pub jump_presses: u32,
    /// Bolts actually released (a `ManaSpent` event each), not held presses.
    pub bolts_released: u32,
    pub dry_presses: u32,
    /// Magic Missile hits on a llama.
    pub bolt_hits: u32,
    /// Hits on a llama by Donut's other moves.
    pub donut_melee_hits: u32,
    /// Hits on a llama by a Carl move (jab, haymaker, kick...).
    pub carl_hits: u32,
    pub llamas: usize,
    pub llamas_defeated: usize,
    /// Tick each llama was defeated on (`None`: alive at the end).
    pub kill_ticks: Vec<Option<u64>>,
    /// The last hit on each defeated llama was Donut's (CRL-Q35 `hit_by`).
    pub kill_by_donut: Vec<bool>,
    pub llama_final_hp: Vec<u8>,
    pub tells: u32,
    /// The tick of the first llama tell (the attack-phase variant's effect).
    pub first_tell_tick: Option<u64>,
    pub spits: u32,
    pub tells_interrupted: u32,
    pub rush_tells: u32,
    pub rush_commits: u32,
    pub rush_contacts: u32,
    pub rush_misses: u32,
    pub rush_bonks: u32,
    /// CORRAL-D (diagnostic): what broke the partition this run, if it broke:
    /// the rush (`Shove`) or hero hits (`MeleeHit`). A pre-broken run has none.
    pub partition_break: Option<crate::env_prop::StimulusKind>,
    /// Spit hits on Donut / on Carl (`HeroHurt` events).
    pub donut_hurt_events: u32,
    pub carl_hurt_events: u32,
    /// Rush contacts on Donut / on Carl.
    pub donut_rush_hurts: u32,
    pub carl_rush_hurts: u32,
    /// Donut evaded an attack (`HeroEvaded` events).
    pub evades: u32,
    /// Ticks on which the policy sent Donut a move or jump input.
    pub donut_move_inputs: u32,
    pub donut_displacement_px: f32,
    /// Largest horizontal distance from Donut's start column (CRL-Q42).
    pub donut_max_offset_px: f32,
    /// Diagnostic (review finding 1, CRL-Q35's "hero displaced to another
    /// level"): landings of a policy-driven Donut on a surface of another
    /// height than her last one, with no horizontal input from the policy in
    /// the [`STEER_WINDOW_TICKS`] before (a knock, not a walk).
    pub donut_level_changes: u32,
    /// Spits of the first llama (B on M2) released while Donut stood over the
    /// walking line (MAG-07, CRL-Q11: a floor Donut is rushed, never spat
    /// at). Measured on her support at the spit tick, not on her start
    /// surface. The event names no target and the spit is decided at its
    /// tell, so the count is judged (0) on the parked stationary runs only;
    /// in a seat where a hero moves it is reported.
    pub spits_at_floor_donut: u32,
    pub donut_start: (f32, f32),
    pub donut_final: (f32, f32),
    pub failures: SystemFailures,
    pub state_hash: u64,
    /// Every event's display line, for the double-run equality check.
    pub event_lines: Vec<String>,
}

impl RunRecord {
    pub fn donut_hp_lost(&self) -> u8 {
        self.donut_hp_start.saturating_sub(self.donut_hp_final)
    }
    pub fn carl_hp_lost(&self) -> u8 {
        self.carl_hp_start.saturating_sub(self.carl_hp_final)
    }
    pub fn both_llamas_defeated(&self) -> bool {
        self.llamas > 0 && self.llamas_defeated == self.llamas
    }
    /// Every llama of an M2 stage was defeated by a Donut-credited hit.
    pub fn all_defeated_by_donut(&self) -> bool {
        self.llamas > 0 && self.kill_by_donut.iter().all(|d| *d)
    }
    /// Carl's move dealt damage to a llama while a human played Donut.
    pub fn companion_assisted(&self) -> bool {
        self.spec.seat.donut_is_player() && self.spec.seat != Seat::TwoPlayer && self.carl_hits > 0
    }
    pub fn no_damage_clear(&self) -> bool {
        self.both_llamas_defeated() && self.donut_hp_lost() == 0 && self.carl_hp_lost() == 0
    }
    pub fn no_movement_clear(&self) -> bool {
        self.both_llamas_defeated() && self.donut_displacement_px == 0.0
    }
    /// Donut never moved from her column (an outcome: knockback moves a
    /// stationary Donut once llamas target her; see `donut_move_inputs` for
    /// what the policy itself sent).
    pub fn stationary_position_valid(&self) -> bool {
        self.donut_displacement_px == 0.0
    }
    /// Mana arithmetic closes: start - spent + regained = final, so nothing
    /// but the real step touched the pool (no false shot accounting).
    pub fn mana_accounting_closes(&self) -> bool {
        i64::from(self.mana_start) - i64::from(self.mana_spent) + i64::from(self.mana_regained)
            == i64::from(self.mana_final)
    }
    /// `layer_static_clear` (diagnostic, not gated): a static Donut defeated
    /// at least one llama of an M2 stage by her own hits.
    pub fn layer_static_clear(&self) -> bool {
        self.spec.policy.is_stationary_gate()
            && !self.spec.stage.is_m1()
            && self
                .kill_by_donut
                .iter()
                .zip(&self.kill_ticks)
                .any(|(d, t)| *d && t.is_some())
    }
}

// ------------------------------------------------------------------ build --

/// The kits the stages run on.
#[derive(Debug, Clone)]
pub struct Kits {
    pub corridor: EnvKit,
    pub corral: EnvKit,
    /// The room's sidecar: its one prop is the partition (CORRAL-D).
    pub encounter: crate::encounter::Encounter,
}

impl Kits {
    pub fn of(&self, stage: Stage) -> &EnvKit {
        if stage.is_corral() {
            &self.corral
        } else {
            &self.corridor
        }
    }
}

/// Load the corridor kit (baselines) and the corral room kit.
pub fn load_kits(root: &Path) -> Result<Kits, String> {
    let loaded = crate::encounter::load(root, crate::f01_corral::ENCOUNTER_PATH).map_err(|e| {
        e.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    Ok(Kits {
        corridor: load_kit(root)?,
        corral: room::load_kit(root)?,
        encounter: loaded.encounter,
    })
}

/// Carl's start column for a spec: the parked or companion seat column, the
/// tested hero's column when he is the one tested.
fn carl_start(spec: &RunSpec) -> i32 {
    match (spec.stage, spec.seat) {
        (_, Seat::SoloCarl) if spec.stage.is_corral() => spec.donut_x,
        (Stage::CorralM1, _) => room::CARL_PARK_M1,
        (Stage::CorralM2, Seat::SoloDonutParked | Seat::SoloDonutBot) => room::CARL_PARK_M2,
        (Stage::CorralM2, _) => room::CARL_ASSIST_M2,
        _ => CARL_X,
    }
}

/// The world y of the top of an intact solid under `x` whose top is `feet`,
/// covering the whole column `x - half .. x + half`.
fn surface_at(sim: &PartyFixtureSim, x: f32, half: f32, feet: f32) -> bool {
    let solids = &sim.scene.world.solids;
    let mut spans: Vec<(f32, f32)> = solids
        .iter()
        .filter(|b| b.intact && (b.rect.top() - feet).abs() < 0.01)
        .map(|b| (b.rect.left, b.rect.right()))
        .collect();
    spans.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut covered = x - half;
    for &(l, r) in &spans {
        if l <= covered {
            covered = covered.max(r);
        }
    }
    // At least one surface under the pivot itself (a zero half-width start is
    // otherwise covered by nothing at all).
    spans.iter().any(|&(l, r)| l <= x && x <= r)
        && covered >= x + half
        && !solids.iter().any(|b| {
            b.intact
                && b.rect.left < x + half
                && b.rect.right() > x - half
                && b.rect.bottom >= feet
                && b.rect.bottom < feet + 2.0 * HERO_HALF_HEIGHT
        })
}

/// Build the sim of one run: the only place game state is written, at tick 0.
///
/// `Err` carries an `invalid_setup` reason (no surface under the column).
pub fn build(kits: &Kits, spec: &RunSpec) -> Result<PartyFixtureSim, String> {
    let kit = kits.of(spec.stage);
    let fixture = fixture_for(spec.stage, spec.seat, spec.evasion);
    let carl_x = carl_start(spec);
    if !spec.stage.is_corral() {
        let line = kit.walking_line_at(CARL_X);
        if spec.donut_rise != 0 || kit.walking_line_at(spec.donut_x) != line || line.is_none() {
            return Err(format!(
                "no clean ground under Donut's column {}",
                spec.donut_x
            ));
        }
    }
    let mut scene = SceneSim::new_at(kit, VIEW_H, carl_x)?;
    // Review L1: the CF02 rules and Donut's biscuit are declared on the state,
    // not detected from the fixture's name.
    let mut combat = CombatState::new(fixture.base, kit, &scene)?
        .with_cf02()
        .with_donut(fixture.donut);
    // CORRAL-D: the partition comes from the room's real sidecar through the env
    // runtime (the same prop, solids and rush contact the launched room has); the
    // `absent` variant builds no env, as before.
    let with_partition =
        spec.stage == Stage::CorralM2 && spec.partition != PartitionVariant::Absent;
    if with_partition {
        let env = crate::combat_scene::EnvRuntime::new(&kits.encounter)?;
        combat = combat.with_env(env);
        combat.sync_env(&mut scene);
    }
    let mut sim = PartyFixtureSim::from_parts(scene, combat, fixture);
    // Setup (tick 0): the `broken` variant is authored pre-broken (F11).
    if with_partition && spec.partition == PartitionVariant::Broken {
        sim.combat.set_partition_broken_for_setup(&mut sim.scene);
    }
    // Setup (tick 0): the parked seat (spec section 11, NB2).
    if spec.seat == Seat::SoloDonutParked {
        sim.park_uncontrolled();
    }
    let floor = sim.scene.world.hero_foot_y();
    let feet = floor + spec.donut_rise as f32;
    if spec.seat == Seat::SoloCarl {
        // Setup (tick 0): Carl is the tested hero; his facing.
        sim.scene.world.hero.facing = spec.facing;
    } else {
        // A start on the partition's own 16 px top is judged on her pivot column
        // alone (the sheet's pivots stand on its edge, her body overhangs it);
        // every other start needs her whole 20 px body on its surface.
        let half = if is_partition_surface(spec.donut_x, spec.donut_rise) {
            0.0
        } else {
            10.0
        };
        if spec.stage.is_corral() && !surface_at(&sim, spec.donut_x as f32, half, feet) {
            return Err(format!(
                "no surface {} px up under Donut's column {}",
                spec.donut_rise, spec.donut_x
            ));
        }
        // Setup (tick 0): Donut's column, surface and facing.
        sim.scene.world.donut_body.x = spec.donut_x as f32;
        sim.scene.world.donut_body.y = feet + HERO_HALF_HEIGHT;
        sim.scene.world.donut_body.facing = spec.facing;
        sim.scene.world.sync_donut_for_tests();
    }
    // Setup (tick 0): the attack-phase variant. `reset_attempt()` rebuilds the
    // llamas from the fixture, so the phase is written once, here.
    for (llama, (cooldown, rush_cooldown)) in sim.combat.llamas.iter_mut().zip(spec.phase) {
        llama.cooldown = cooldown;
        if let Some(corral) = llama.corral.as_mut() {
            corral.rush_cooldown = rush_cooldown;
        }
    }
    Ok(sim)
}

/// The height of the surface under a hero's feet (her feet when grounded).
fn support_of(body: &Hero, solids: &[crate::prototype::Block]) -> f32 {
    let feet = body.y - HERO_HALF_HEIGHT;
    feet - feet_height_above_solid(body, solids).unwrap_or(0.0)
}

/// A level change counts as a knock when the policy sent Donut no horizontal
/// input in this many ticks before she landed (a walk or a steered hop off a
/// ledge carries her momentum for less than this).
pub const STEER_WINDOW_TICKS: u64 = 30;

/// What the policy may see of the finished state.
fn observe(sim: &PartyFixtureSim) -> Obs {
    let solids = &sim.scene.world.solids;
    let hero = |id: HeroId| {
        let body = sim.scene.world.body(id);
        let feet = body.y - HERO_HALF_HEIGHT;
        HeroObs {
            x: body.x,
            facing: body.facing,
            health: body.health,
            grounded: body.grounded,
            feet,
            support: support_of(body, solids),
        }
    };
    Obs {
        tick: sim.scene.world.tick + 1,
        carl: hero(HeroId::Carl),
        donut: hero(HeroId::Donut),
        mana: sim.combat.donut_mana_view(),
        cast_active: sim.combat.donut_cast().is_some(),
        donut_hurt: sim.combat.hurt_ticks_of(HeroId::Donut) > 0,
        llamas: sim
            .combat
            .llamas
            .iter()
            .map(|l| LlamaObs {
                x: l.x,
                feet: l.feet(),
                facing: l.facing,
                health: l.health,
                mode: l.mode,
                attack_tick: l.attack_tick,
                rush: l.rush_state().map(|r| r.phase),
                rect: None,
            })
            .collect(),
    }
}

/// The static facts the policies read (see [`Aim`]): the room's solids, the
/// bolt's flight and the tested Donut's own jump in place from her start,
/// computed on a copy of her body with the real hero step.
fn aim_of(sim: &PartyFixtureSim, stage: Stage) -> Aim {
    let tuning = crate::donut_mana::ManaTuning::pinned(true).expect("pinned mana tuning loads");
    let solids = sim.scene.world.solids.clone();
    let start = sim.scene.world.donut_body.clone();
    let surface = start.y - HERO_HALF_HEIGHT;
    let mut body = Hero::at(start.x, start.y);
    let motion = crate::agility::agile_motion();
    let mut input = InputFrame {
        jump_pressed: true,
        ..InputFrame::default()
    };
    let mut arc = Vec::new();
    for _ in 0..120 {
        step_hero_motion(
            &mut body,
            motion,
            &input,
            &solids,
            1.0e9,
            &BodyDrive::default(),
        );
        input.jump_pressed = false;
        if body.grounded {
            break;
        }
        arc.push(body.y - HERO_HALF_HEIGHT - surface);
    }
    let floor = sim.scene.world.hero_foot_y();
    let route = stage.is_corral().then_some(Route {
        floor,
        bumper_left: room::BUMPER.0 as f32,
        bumper_right: room::BUMPER.1 as f32,
        bumper_top: floor + room::BUMPER.2 as f32,
    });
    Aim {
        solids,
        bolt_forward: BOLT_SPAWN_FORWARD,
        bolt_up: BOLT_SPAWN_UP,
        bolt_step: f32::from(tuning.projectile_speed_px_per_tick),
        bolt_range: f32::from(tuning.projectile_range_px),
        arc,
        route,
    }
}

/// Route per-hero inputs to the party's slots for this seat.
fn route(seat: Seat, tick: u64, inputs: HeroInputs) -> SlotInputs {
    match seat {
        Seat::SoloDonutBot | Seat::SoloDonutAssist | Seat::SoloDonutParked => {
            // Slot 0 drives the controlled hero; the swap press on tick 1 makes
            // that Donut (the real swap path, no state write). A parked Carl
            // gets the neutral input of the uncontrolled seat.
            let mut raw = inputs.donut;
            raw.swap = tick == 1;
            [raw, RawInput::NONE]
        }
        Seat::SoloCarl => [inputs.carl, RawInput::NONE],
        Seat::TwoPlayer => [inputs.carl, inputs.donut],
    }
}

fn llama_defeated(l: &crate::llama::Llama) -> bool {
    l.health == 0 || matches!(l.mode, LlamaMode::Down | LlamaMode::Corpse)
}

const fn donut_move(move_id: MoveId) -> bool {
    matches!(
        move_id,
        MoveId::MagicMissile | MoveId::Swat | MoveId::Pounce
    )
}

/// Run one spec to a terminal reason. Deterministic: the same spec gives the
/// same record, hash and event lines.
pub fn run(kits: &Kits, spec: &RunSpec) -> RunRecord {
    let llamas = spec.stage.llama_xs().len();
    let mut record = RunRecord {
        spec: *spec,
        run_id: spec.run_id(),
        terminal: Terminal::Timeout,
        note: String::new(),
        ticks: 0,
        donut_hp_start: 0,
        donut_hp_final: 0,
        carl_hp_start: 0,
        carl_hp_final: 0,
        mana_start: 0,
        mana_final: 0,
        mana_min: 0,
        mana_spent: 0,
        mana_regained: 0,
        presses: 0,
        jump_presses: 0,
        bolts_released: 0,
        dry_presses: 0,
        bolt_hits: 0,
        donut_melee_hits: 0,
        carl_hits: 0,
        llamas,
        llamas_defeated: 0,
        kill_ticks: vec![None; llamas],
        kill_by_donut: vec![false; llamas],
        llama_final_hp: Vec::new(),
        tells: 0,
        first_tell_tick: None,
        spits: 0,
        tells_interrupted: 0,
        rush_tells: 0,
        rush_commits: 0,
        rush_contacts: 0,
        rush_misses: 0,
        rush_bonks: 0,
        partition_break: None,
        donut_hurt_events: 0,
        carl_hurt_events: 0,
        donut_rush_hurts: 0,
        carl_rush_hurts: 0,
        evades: 0,
        donut_move_inputs: 0,
        donut_displacement_px: 0.0,
        donut_max_offset_px: 0.0,
        donut_level_changes: 0,
        spits_at_floor_donut: 0,
        donut_start: (0.0, 0.0),
        donut_final: (0.0, 0.0),
        failures: SystemFailures::default(),
        state_hash: 0,
        event_lines: Vec::new(),
    };
    let mut sim = match build(kits, spec) {
        Ok(sim) => sim,
        Err(why) => {
            record.terminal = Terminal::InvalidSetup;
            record.note = why;
            return record;
        }
    };
    let mut policy = Policy::with_aim(spec.policy, spec.jump_offset, aim_of(&sim, spec.stage));
    record.donut_hp_start = sim.scene.world.donut_body.health;
    record.carl_hp_start = sim.scene.world.hero.health;
    let (mana0, _) = sim.combat.donut_mana_view().unwrap_or((0, 0));
    record.mana_start = mana0;
    record.mana_min = mana0;
    let start = (sim.scene.world.donut_body.x, sim.scene.world.donut_body.y);
    record.donut_start = start;
    let enemy_count = sim.combat.enemies.len() as u32;
    if spec.stage.is_corral() && !sim.combat.llama_party_target_on() {
        record.failures.carl_only_targeting += 1;
    }
    // Each corral llama's surface and leash at setup (CRL-Q43).
    let homes: Vec<Option<(f32, crate::llama_rush::Leash)>> = sim
        .combat
        .llamas
        .iter()
        .map(|l| l.corral.as_ref().map(|c| (l.feet(), c.leash)))
        .collect();
    // The llama each rush instance belongs to is committed (KC6: one token).
    let mut committed = vec![false; llamas];
    let mut last_hit_donut = vec![false; llamas];
    // Absolute index of the next unread event (the log drops from the front).
    let mut seen = 0usize;
    let mut last_prop_hit: Option<crate::env_prop::StimulusKind> = None;
    let mut displacement = 0.0f32;
    let mut last = start;
    // The walking line (Carl stands on it at setup in every stage).
    let floor_y = sim.scene.world.hero_foot_y();
    let mut last_support = support_of(&sim.scene.world.donut_body, &sim.scene.world.solids);
    let mut last_steer: Option<u64> = None;
    // Donut's support when her latest bolt was released (review nit 11).
    let mut release_support = last_support;

    for _ in 0..spec.horizon {
        let obs = observe(&sim);
        let tick = obs.tick;
        let inputs = policy.act(&obs);
        if inputs.donut.move_x != 0.0 || inputs.donut.jump {
            record.donut_move_inputs += 1;
        }
        if inputs.donut.move_x != 0.0 {
            last_steer = Some(tick);
        }
        sim.step_slots(route(spec.seat, tick, inputs));
        record.ticks = tick;
        let view_left = sim.scene.camera.left() as f32;
        let view_right = view_left + sim.scene.camera.config().view_w as f32;
        let donut_support = support_of(&sim.scene.world.donut_body, &sim.scene.world.solids);
        let donut_on_floor = (donut_support - floor_y).abs() < 0.5;

        let first = seen.saturating_sub(sim.combat.dropped_events);
        for event in &sim.combat.events[first..] {
            record.event_lines.push(event.to_string());
            match *event {
                CombatEvent::Mana(ManaEvent::ManaSpent { .. }) => {
                    record.mana_spent += 1;
                    record.bolts_released += 1;
                    release_support = donut_support;
                }
                CombatEvent::Mana(ManaEvent::ManaRegained { .. }) => record.mana_regained += 1,
                CombatEvent::Mana(ManaEvent::CastRejected { .. }) => record.dry_presses += 1,
                CombatEvent::Hit {
                    move_id, target, ..
                } if target >= enemy_count => {
                    let index = (target - enemy_count) as usize;
                    match move_id {
                        MoveId::MagicMissile => record.bolt_hits += 1,
                        MoveId::Swat | MoveId::Pounce => record.donut_melee_hits += 1,
                        _ => record.carl_hits += 1,
                    }
                    if let Some(slot) = last_hit_donut.get_mut(index) {
                        *slot = donut_move(move_id);
                    }
                    // CRL-Q35 machine check: a static Donut who released
                    // the bolt over the walking line never hits the dock
                    // llama.
                    if move_id == MoveId::MagicMissile
                        && spec.stage == Stage::CorralM2
                        && index == 1
                        && spec.policy.is_stationary_gate()
                        && (release_support - floor_y).abs() < 0.5
                    {
                        record.failures.dock_hit_from_floor += 1;
                    }
                }
                CombatEvent::LlamaTell { tick, llama }
                | CombatEvent::LlamaRushTell { tick, llama } => {
                    record.tells += 1;
                    record.first_tell_tick.get_or_insert(tick);
                    if matches!(event, CombatEvent::LlamaRushTell { .. }) {
                        record.rush_tells += 1;
                    }
                    if let Some(l) = sim.combat.llamas.get(llama as usize) {
                        let body = crate::llama_rush::geometry::llama_body(l.x, l.feet(), l.facing);
                        if body.left < view_left || body.right() > view_right {
                            record.failures.offscreen_tell += 1;
                        }
                    }
                }
                CombatEvent::LlamaSpit { llama, .. } => {
                    record.spits += 1;
                    if llama == 0 && donut_on_floor {
                        record.spits_at_floor_donut += 1;
                    }
                    if sim
                        .combat
                        .llamas
                        .get(llama as usize)
                        .is_some_and(|l| l.rush_state().is_some())
                    {
                        record.failures.glob_from_rush += 1;
                    }
                }
                CombatEvent::LlamaInterrupted { llama, .. } => {
                    record.tells_interrupted += 1;
                    if let Some(c) = committed.get_mut(llama as usize) {
                        *c = false;
                    }
                }
                CombatEvent::LlamaRushCommit { llama, .. } => {
                    record.rush_commits += 1;
                    if let Some(c) = committed.get_mut(llama as usize) {
                        *c = true;
                    }
                }
                CombatEvent::LlamaRushEnd { llama, end, .. } => {
                    match end {
                        RushEnd::Contact(_) => record.rush_contacts += 1,
                        RushEnd::Miss => record.rush_misses += 1,
                        RushEnd::Blocked | RushEnd::Partition => record.rush_bonks += 1,
                    }
                    if let Some(c) = committed.get_mut(llama as usize) {
                        *c = false;
                    }
                }
                CombatEvent::HeroHurt { hero, source, .. } => {
                    match hero {
                        HeroId::Donut => record.donut_hurt_events += 1,
                        HeroId::Carl => record.carl_hurt_events += 1,
                    }
                    if source == LLAMA_RUSH_SOURCE_ID {
                        match hero {
                            HeroId::Donut => record.donut_rush_hurts += 1,
                            HeroId::Carl => record.carl_rush_hurts += 1,
                        }
                        if !committed.iter().any(|c| *c) {
                            record.failures.rush_hit_before_commit += 1;
                        }
                    }
                }
                CombatEvent::HeroEvaded { .. } => record.evades += 1,
                CombatEvent::PropHit { kind, .. } => last_prop_hit = Some(kind),
                CombatEvent::PropPhase {
                    phase: crate::combat::PropPhase::Broken,
                    ..
                } => record.partition_break = last_prop_hit,
                _ => {}
            }
        }
        seen = sim.combat.dropped_events + sim.combat.events.len();
        // A commit that ended with the llama's defeat logs no rush end.
        for (i, l) in sim.combat.llamas.iter().enumerate() {
            if l.rush_state()
                .is_none_or(|r| r.phase != RushPhase::Committed)
            {
                committed[i] = false;
            }
        }

        if let Some((mana, _)) = sim.combat.donut_mana_view() {
            record.mana_min = record.mana_min.min(mana);
        }
        let now = (sim.scene.world.donut_body.x, sim.scene.world.donut_body.y);
        displacement += (now.0 - last.0).abs() + (now.1 - last.1).abs();
        record.donut_max_offset_px = record.donut_max_offset_px.max((now.0 - start.0).abs());
        last = now;
        // Only a policy-driven Donut: her AI steers itself in the Carl seat.
        if spec.seat.donut_is_player() && sim.scene.world.donut_body.grounded {
            if (donut_support - last_support).abs() > 0.5
                && last_steer.is_none_or(|t| tick.saturating_sub(t) > STEER_WINDOW_TICKS)
            {
                record.donut_level_changes += 1;
            }
            last_support = donut_support;
        }
        for (i, llama) in sim.combat.llamas.iter().enumerate() {
            if llama_defeated(llama) && record.kill_ticks[i].is_none() {
                record.kill_ticks[i] = Some(tick);
                record.kill_by_donut[i] = last_hit_donut[i];
            }
            // CRL-Q43: in every mode, Down and Corpse included.
            if let Some(Some((feet, leash))) = homes.get(i)
                && (!leash.contains(llama.x) || (llama.feet() - feet).abs() > 0.01)
            {
                record.failures.left_leash_or_level += 1;
            }
        }
        record.llamas_defeated = record.kill_ticks.iter().flatten().count();

        if sim.combat.hero_defeated {
            record.terminal = Terminal::HeroDefeat;
            break;
        }
        if record.both_llamas_defeated() {
            record.terminal = Terminal::Victory;
            break;
        }
    }

    record.presses = policy.presses;
    record.jump_presses = policy.jumps;
    record.donut_hp_final = sim.scene.world.donut_body.health;
    record.carl_hp_final = sim.scene.world.hero.health;
    record.mana_final = sim.combat.donut_mana_view().map_or(0, |(m, _)| m);
    record.llama_final_hp = sim.combat.llamas.iter().map(|l| l.health).collect();
    record.donut_displacement_px = displacement;
    record.donut_final = last;
    record.state_hash = sim.state_hash();
    if record.failures.total() > 0 {
        record.note = record.failures.describe();
    }
    record
}

/// Load the corridor kit the baseline stages run on.
pub fn load_kit(root: &Path) -> Result<EnvKit, String> {
    EnvKit::load(root, LAYOUT).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------ sweep --

/// One authored start of the corral (spec section 11): an id, the column,
/// the surface height above the walking line and the natural facing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Start {
    pub id: &'static str,
    pub x: i32,
    pub rise: i32,
    pub facing: i8,
}

const fn start(id: &'static str, x: i32, rise: i32, facing: i8) -> Start {
    Start {
        id,
        x,
        rise,
        facing,
    }
}

/// M1-A and M1-B (the positive control).
pub const M1_STARTS: [Start; 2] = [start("M1-A", 608, 0, 1), start("M1-B", 560, 0, 1)];

/// The named M2 starts of spec section 11. M2-06 is the partition's top, so it
/// exists only in the `intact` variant ([`start_exists`]).
pub const M2_STARTS: [Start; 13] = [
    start("M2-01", 1072, 0, 1),
    start("M2-02", 1168, 32, 1),
    start("M2-03", 1204, 32, 1),
    start("M2-04", 1248, 0, 1),
    start("M2-05", 1376, 0, 1),
    start("M2-06", 1400, PARTITION_TOP_RISE, 1),
    start("M2-07", 1424, 0, 1),
    start("M2-08", 1640, 0, -1),
    start("M2-09", 1840, 0, -1),
    start("M2-10", 1904, 32, 1),
    start("M2-11", 2032, 32, -1),
    start("M2-12", 1872, 80, 1),
    start("M2-13", 1728, 0, 1),
];

/// The named M2 start at `(x, rise)`, or `"grid"` for an M2-G column.
pub fn start_name(x: i32, rise: i32) -> &'static str {
    M2_STARTS
        .iter()
        .find(|s| s.x == x && s.rise == rise)
        .map_or("grid", |s| s.id)
}

/// The room surface of a start: `floor`, `shelf` (the 32 px entry shelf),
/// `bumper` (its 80 px top) or `dock` (the 32 px dock behind it).
pub fn surface_name(x: i32, rise: i32) -> &'static str {
    match rise {
        0 => "floor",
        PARTITION_TOP_RISE => "partition",
        REMNANT_TOP_RISE => "remnant",
        r if r == room::BUMPER.2 => "bumper",
        _ if x < room::BUMPER.0 => "shelf",
        _ => "dock",
    }
}

/// The M2-G grid (spec section 11, step 16): the floor, the shelf, the bumper
/// top and the dock, each column wholly on its surface (Donut's 20 px wide
/// collider: no floor column inside the entry shelf, the bumper's middle
/// column only), plus the partition's own tops: the screen's 48 px top (the
/// `intact` variant) and its remnant's 16 px top (the `broken` variant), at the
/// screen's two edge columns ([`start_exists`] keeps each to its variant).
pub fn m2_grid() -> Vec<(i32, i32)> {
    let (shelf_l, shelf_r, _) = room::ENTRY_SHELF;
    let mut out = Vec::new();
    for x in (1072..=1840).step_by(16) {
        if x + 10 <= shelf_l || x - 10 >= shelf_r {
            out.push((x, 0));
        }
    }
    for x in (1136..=1200).step_by(16) {
        out.push((x, 32));
    }
    out.push(((room::BUMPER.0 + room::BUMPER.1) / 2, 80));
    for x in (1904..=2032).step_by(16) {
        out.push((x, 32));
    }
    for rise in [PARTITION_TOP_RISE, REMNANT_TOP_RISE] {
        for x in (PARTITION_X.0..=PARTITION_X.1).step_by(16) {
            out.push((x, rise));
        }
    }
    out
}

/// The sweep grid: legal starts crossed with the variants.
#[derive(Debug, Clone)]
pub struct SweepConfig {
    /// The baseline stages to run (CORRAL-EVAL core, the "before").
    pub baseline: Vec<Stage>,
    /// Baseline Donut columns and their tell delays (pairwise).
    pub baseline_xs: Vec<i32>,
    pub baseline_phases: Vec<u16>,
    /// Run the corral stages (CORRAL-C, the "after").
    pub corral: bool,
    /// Add the M2-G grid to the named starts.
    pub grid: bool,
    /// The corral's partition variants run on the M2 stage (CORRAL-D).
    pub partitions: Vec<PartitionVariant>,
    /// The jump-cast offsets run on the named starts (the grid runs `aimed`,
    /// 0 and 45 only).
    pub jump_offsets: Vec<i8>,
    pub facings: Vec<i8>,
    pub evasions: Vec<Evasion>,
    /// Also run the reported seats and policies (companion variant, the
    /// trivial hold, no interaction) on the corral starts.
    pub reported: bool,
    pub horizon: u64,
}

impl SweepConfig {
    /// The sweep of the `corral_eval` binary: every start, variant and policy.
    pub fn full() -> Self {
        Self {
            baseline: Stage::BASELINE.to_vec(),
            baseline_xs: (240..=560).step_by(40).collect(),
            baseline_phases: vec![0, 45, 89],
            corral: true,
            grid: true,
            partitions: PartitionVariant::ALL.to_vec(),
            jump_offsets: policy::JUMP_OFFSETS.collect(),
            facings: vec![1, -1],
            evasions: Evasion::ALL.to_vec(),
            reported: true,
            horizon: HORIZON_TICKS,
        }
    }

    /// The asserted gate sweep of the tests: the whole corral half of
    /// [`Self::full`] (every named start with every jump offset -3..=52, the
    /// M2-G grid, both facings, P0..P2, both reflex variants, the decisive and
    /// the reported seats and policies, the M1 control and the movement and
    /// co-op runs). Only the corridor baselines (report-only) are left out.
    pub fn gate() -> Self {
        Self {
            baseline: Vec::new(),
            baseline_xs: Vec::new(),
            baseline_phases: Vec::new(),
            ..Self::full()
        }
    }

    /// A coarse baseline sweep for the unit tests (debug builds).
    pub fn quick() -> Self {
        Self {
            baseline: Stage::BASELINE.to_vec(),
            baseline_xs: vec![300, 420, 540],
            baseline_phases: vec![0, 89],
            corral: false,
            grid: false,
            partitions: vec![PartitionVariant::Absent],
            jump_offsets: Vec::new(),
            facings: vec![1, -1],
            evasions: vec![Evasion::Off],
            reported: false,
            horizon: HORIZON_TICKS,
        }
    }

    /// The stages this sweep runs.
    pub fn stages(&self) -> Vec<Stage> {
        let mut out = self.baseline.clone();
        if self.corral {
            out.extend(Stage::CORRAL);
        }
        out
    }

    fn baseline_specs(&self, out: &mut Vec<RunSpec>) {
        for &stage in &self.baseline {
            let llamas = stage.llama_xs().len();
            let phases: Vec<[Phase; 2]> = if llamas == 1 {
                self.baseline_phases
                    .iter()
                    .map(|&p| [(p, 0), (0, 0)])
                    .collect()
            } else {
                self.baseline_phases
                    .iter()
                    .flat_map(|&a| self.baseline_phases.iter().map(move |&b| [(a, 0), (b, 0)]))
                    .collect()
            };
            for &evasion in &self.evasions {
                for &x in &self.baseline_xs {
                    // Not a natural start: Donut's column on top of a llama.
                    if stage
                        .llama_xs()
                        .iter()
                        .any(|&lx| (lx - x).abs() < START_CLEARANCE_PX)
                    {
                        continue;
                    }
                    for &facing in &self.facings {
                        for &phase in &phases {
                            let mut push = |seat, policy| {
                                out.push(RunSpec {
                                    stage,
                                    seat,
                                    policy,
                                    evasion,
                                    donut_x: x,
                                    donut_rise: 0,
                                    facing,
                                    phase,
                                    jump_offset: 0,
                                    horizon: self.horizon,
                                    partition: PartitionVariant::Absent,
                                });
                            };
                            for policy in [
                                PolicyId::StationaryCast,
                                PolicyId::StationaryCastBurst,
                                PolicyId::HoldAttackTrivial,
                                PolicyId::NoInteraction,
                                PolicyId::MobileCast,
                            ] {
                                push(Seat::SoloDonutBot, policy);
                            }
                            for policy in [PolicyId::StationaryCast, PolicyId::StationaryCastBurst]
                            {
                                push(Seat::SoloDonutAssist, policy);
                            }
                            // Carl solo does not depend on Donut's column.
                            if x == self.baseline_xs[0] && facing == self.facings[0] {
                                push(Seat::SoloCarl, PolicyId::CarlMelee);
                            }
                            push(Seat::TwoPlayer, PolicyId::CoopProbe);
                        }
                    }
                }
            }
        }
    }

    fn corral_specs(&self, out: &mut Vec<RunSpec>) {
        let spec = |stage, seat, policy, evasion, x, rise, facing, phase, o, partition| RunSpec {
            stage,
            seat,
            policy,
            evasion,
            donut_x: x,
            donut_rise: rise,
            facing,
            phase,
            jump_offset: o,
            horizon: self.horizon,
            partition,
        };
        // M1: the positive control, both M1 starts, the stationary policies.
        for &evasion in &self.evasions {
            for s in M1_STARTS {
                for &facing in &self.facings {
                    for policy in [
                        PolicyId::StationaryCast,
                        PolicyId::StationaryCastAimed,
                        PolicyId::StationaryCastBurst,
                    ] {
                        out.push(spec(
                            Stage::CorralM1,
                            Seat::SoloDonutParked,
                            policy,
                            evasion,
                            s.x,
                            s.rise,
                            facing,
                            [(0, 0), (0, 0)],
                            0,
                            PartitionVariant::Absent,
                        ));
                    }
                }
            }
        }
        // M2: the named starts (every jump offset) and the grid (aimed, 0, 45).
        let mut starts: Vec<(i32, i32, bool)> =
            M2_STARTS.iter().map(|s| (s.x, s.rise, true)).collect();
        if self.grid {
            for (x, rise) in m2_grid() {
                if !starts.iter().any(|&(sx, sr, _)| sx == x && sr == rise) {
                    starts.push((x, rise, false));
                }
            }
        }
        for &variant in &self.partitions {
            for &evasion in &self.evasions {
                for &(x, rise, named) in &starts {
                    if !start_exists(variant, x, rise) {
                        continue;
                    }
                    for &facing in &self.facings {
                        for phase in M2_PHASES {
                            let push = |out: &mut Vec<RunSpec>, seat, policy, o| {
                                out.push(spec(
                                    Stage::CorralM2,
                                    seat,
                                    policy,
                                    evasion,
                                    x,
                                    rise,
                                    facing,
                                    phase,
                                    o,
                                    variant,
                                ));
                            };
                            for policy in PolicyId::GATED_FIXED {
                                push(out, Seat::SoloDonutParked, policy, 0);
                            }
                            let offsets: Vec<i8> = if named {
                                self.jump_offsets.clone()
                            } else {
                                vec![0, 45]
                            };
                            for o in offsets {
                                push(out, Seat::SoloDonutParked, PolicyId::StationaryJumpCast, o);
                            }
                            if self.reported {
                                for policy in
                                    [PolicyId::StationaryCast, PolicyId::StationaryCastBurst]
                                {
                                    push(out, Seat::SoloDonutAssist, policy, 0);
                                }
                                for policy in [PolicyId::HoldAttackTrivial, PolicyId::NoInteraction]
                                {
                                    push(out, Seat::SoloDonutParked, policy, 0);
                                }
                            }
                            if named {
                                // Movement success (spec section 11, gate item 4).
                                push(out, Seat::SoloDonutParked, PolicyId::MobileCast, 0);
                                push(out, Seat::TwoPlayer, PolicyId::CoopProbe, 0);
                                // Solo Carl starts at the M2 entry (his column is
                                // the start; Donut follows as the bot).
                                if x == M2_STARTS[0].x && rise == 0 {
                                    push(out, Seat::SoloCarl, PolicyId::CarlMelee, 0);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Every spec of the sweep.
    pub fn specs(&self) -> Vec<RunSpec> {
        let mut out = Vec::new();
        self.baseline_specs(&mut out);
        if self.corral {
            self.corral_specs(&mut out);
        }
        out
    }
}

/// Today's llamas aim every walk, facing and shot at Carl (`step_llamas`
/// passes `scene.world.hero.x`). The probe puts Donut on the far side of the
/// llama and nearer to it than Carl, runs 90 neutral ticks in two-player and
/// reports whether the llama still faces Carl. On the corral stages party
/// targeting is checked on every run (a hard system failure when absent).
pub fn carl_only_targeting(kits: &Kits) -> Option<bool> {
    let spec = RunSpec {
        stage: Stage::M1,
        seat: Seat::TwoPlayer,
        policy: PolicyId::NoInteraction,
        evasion: Evasion::Off,
        // The llama stands at 640, Carl at 400 (left): Donut at 800 is nearer.
        donut_x: 800,
        donut_rise: 0,
        facing: -1,
        phase: [(0, 0), (0, 0)],
        jump_offset: 0,
        horizon: 90,
        partition: PartitionVariant::Absent,
    };
    let mut sim = build(kits, &spec).ok()?;
    for _ in 0..spec.horizon {
        sim.step_slots([RawInput::NONE, RawInput::NONE]);
    }
    let llama = sim.combat.llamas.first()?;
    let donut_nearer =
        (sim.scene.world.donut_body.x - llama.x).abs() < (sim.scene.world.hero.x - llama.x).abs();
    let faces_donut = (sim.scene.world.donut_body.x > llama.x) == (llama.facing > 0);
    // Inconclusive when Donut is not nearer (a llama that walked toward Carl,
    // or party targeting that moved it): never read as "targeting absent".
    donut_nearer.then_some(!faces_donut)
}

/// Run a sweep and write `summary.json`, `runs.csv` and `report.md` into
/// `out`. Returns the summary text.
pub fn evaluate_to(root: &Path, out: &Path, config: &SweepConfig) -> Result<String, String> {
    let kits = load_kits(root)?;
    let records = sweep(&kits, config);
    let targeting = carl_only_targeting(&kits);
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let summary = report::summary_json(&records, config, targeting);
    let text = serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?;
    let write = |name: &str, body: &str| {
        std::fs::write(out.join(name), body).map_err(|e| format!("{name}: {e}"))
    };
    write("summary.json", &text)?;
    write("runs.csv", &report::runs_csv(&records))?;
    write("report.md", &report::report_md(&records, config, targeting))?;
    Ok(text)
}

/// The worker threads of a sweep: `CORRAL_EVAL_THREADS`, else up to 8.
fn threads() -> usize {
    std::env::var("CORRAL_EVAL_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
                .min(8)
        })
        .max(1)
}

/// Run every spec of a sweep. Runs are independent and deterministic, so they
/// run on worker threads; the records keep the spec order.
pub fn sweep(kits: &Kits, config: &SweepConfig) -> Vec<RunRecord> {
    run_all(kits, &config.specs())
}

/// Run `specs` on worker threads (worker `k` takes specs `k`, `k + n`, ...),
/// returned in spec order, each record without its event lines (only the
/// in-process double-run check needs those, through [`run`]).
pub fn run_all(kits: &Kits, specs: &[RunSpec]) -> Vec<RunRecord> {
    let n = threads().min(specs.len()).max(1);
    let mut slots: Vec<Option<RunRecord>> = vec![None; specs.len()];
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..n)
            .map(|k| {
                scope.spawn(move || {
                    specs
                        .iter()
                        .enumerate()
                        .skip(k)
                        .step_by(n)
                        .map(|(i, spec)| {
                            let mut r = run(kits, spec);
                            r.event_lines = Vec::new();
                            (i, r)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for h in handles {
            for (i, r) in h.join().expect("a sweep worker panicked") {
                slots[i] = Some(r);
            }
        }
    });
    slots.into_iter().flatten().collect()
}

#[cfg(test)]
mod partition_fixtures;
#[cfg(test)]
mod tests;
