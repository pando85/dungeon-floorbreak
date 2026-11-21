//! CORRAL-EVAL core: the STATIONARY-DONUT evaluator of CORRAL-AUTO-01 on the
//! real party simulation.
//!
//! It answers one question with numbers: does a Donut who stands still and
//! presses Magic Missile trivialise the corral llamas? Every run drives the
//! real [`PartyFixtureSim`] (the live party step order, real mana, collisions,
//! llama state machine) with legal per-slot raw inputs, one tick at a time.
//!
//! Hard rules, enforced by construction and by tests:
//! - **Setup-only writes.** The one place the evaluator writes game state is
//!   [`build`], at tick 0: Donut's start column and facing, and each llama's
//!   initial tell delay. After that the only mutation is
//!   [`PartyFixtureSim::step_slots`]. It never calls `missile_hits_llama`,
//!   never writes mana or health, and never moves a hero after tick 0.
//! - **Construction path (review L1).** The state is built with
//!   `CombatState::new(..).with_cf02().with_donut(..)` and
//!   `PartyFixtureSim::from_parts`, *not* through the name-based CF02
//!   detection of `PartyFixtureSim::new`. The stage names carry no `cf02_`
//!   prefix and no fixture is registered with `PartyFixture::all()`, so no
//!   shared fixture file, golden or committed replay changes.
//! - **Honest geometry.** The stages run on the corridor kit with today's
//!   llamas (they walk toward Carl and spit; no rush, no party targeting, no
//!   cover). They are *baselines* for CORRAL-C, not the Infernal Corral.
//!
//! Attack-phase variants (F13): `reset_attempt()` rebuilds llamas from the
//! fixture, so a phase cannot be carried by a fixture edit. The evaluator
//! writes `Llama::cooldown` (ticks until the next tell may start) of each llama
//! once, at tick 0, right after construction. Seeds do not vary gameplay, so
//! the report counts **fixtures**, never seeds.
//!
//! Gate status (F13): the assertions that hold against today's llamas are the
//! `corral_auto_*` tests (press edges and mana accounting, setup-only writes,
//! double-run equality, the M1 positive control). The M2 gate
//! ([`report::Gate`], KC19) depends on the rush and on party targeting; it is
//! **report-only** until CORRAL-C. Donut's reflex variant ([`Evasion`]) is a
//! parameter; its `Reflex` arm is `blocked` until DM-B lands (DM-A declares
//! `dm_q11_evasion_mode`, but nothing reads it yet). The committed baseline is
//! therefore labelled "reflex absent".

pub mod policy;
pub mod report;

use std::path::Path;

use crate::boss_scene::Pilot;
use crate::combat::{CombatEvent, EnemyAi, MoveId};
use crate::combat_scene::{CombatState, Fixture, LlamaSpawn};
use crate::corridor::SceneSim;
use crate::donut_mana::{DonutSetup, ManaEvent};
use crate::env_kit::EnvKit;
use crate::input::{RawInput, SlotInputs};
use crate::llama::LlamaMode;
use crate::party::{HeroId, PROVISIONAL_TUNING, PartyTuning, UncontrolledHero};
use crate::party_scene::{PartyFixture, PartyFixtureSim};

use self::policy::{HeroInputs, HeroObs, LlamaObs, Obs, Policy, PolicyId};

/// Bump when a policy, a stage or a metric definition changes.
pub const POLICY_VERSION: u32 = 1;
/// The isolated M2 attempt horizon (CORRAL-AUTO section 3): 30 s at 60 Hz.
pub const HORIZON_TICKS: u64 = 1800;
/// The corridor kit the baseline stages run on.
pub const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";
pub const VIEW_H: i32 = 360;
/// Carl's start column on every stage.
pub const CARL_X: i32 = 400;
/// A start closer than this to a llama's column is not a natural one (the
/// llama body half-width is 23 px).
pub const START_CLEARANCE_PX: i32 = 48;
/// Donut's biscuit pool at the start of every run (the corral's M0 grants it).
pub const START_MANA: u8 = 8;

// ---------------------------------------------------------------- stages --

/// A baseline stage. `M2*` are M2-*like*: two llamas, no cover, no rush.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    /// One llama: the positive control (the first power fantasy).
    M1,
    /// Two llamas side by side ahead of Donut.
    M2,
    /// One llama ahead and one behind: facing and a reveal behind her.
    M2Pincer,
}

impl Stage {
    pub const ALL: [Stage; 3] = [Stage::M1, Stage::M2, Stage::M2Pincer];

    pub const fn name(self) -> &'static str {
        match self {
            Stage::M1 => "corral_eval_m1",
            Stage::M2 => "corral_eval_m2",
            Stage::M2Pincer => "corral_eval_m2_pincer",
        }
    }

    /// The llama columns (feet on the walking line).
    pub fn llama_xs(self) -> Vec<i32> {
        self.base().llamas.iter().map(|l| l.x).collect()
    }

    fn base(self) -> &'static Fixture {
        match self {
            Stage::M1 => &M1_BASE,
            Stage::M2 => &M2_BASE,
            Stage::M2Pincer => &PINCER_BASE,
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

// ----------------------------------------------------------------- seats --

/// Who plays and who stands in. In every Donut seat the tested hero is Donut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Seat {
    /// Solo, swapped to Donut on tick 1; Carl is the follow-and-jump bot that
    /// **never attacks** (the decisive variant: Donut alone).
    SoloDonutBot,
    /// Solo, swapped to Donut; Carl is the companion AI of the live scene
    /// (`Cf02Assist`). **Companion-assisted** when it dealt any damage:
    /// reported, never counted as proof (KC19).
    SoloDonutAssist,
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
            Seat::SoloCarl => "solo_carl",
            Seat::TwoPlayer => "two_player",
        }
    }

    /// Donut is the hero a human drives in this seat.
    pub const fn donut_is_player(self) -> bool {
        matches!(
            self,
            Seat::SoloDonutBot | Seat::SoloDonutAssist | Seat::TwoPlayer
        )
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

const fn party(base: &'static Fixture, two_player: bool, tuning: PartyTuning) -> PartyFixture {
    PartyFixture {
        base,
        two_player,
        tuning,
        script: &[],
        pilot: Pilot::None,
        donut: DonutSetup::unlocked(START_MANA),
    }
}

static M1_SOLO: PartyFixture = party(&M1_BASE, false, BOT_TUNING);
static M1_ASSIST: PartyFixture = party(&M1_BASE, false, ASSIST_TUNING);
static M1_2P: PartyFixture = party(&M1_BASE, true, BOT_TUNING);
static M2_SOLO: PartyFixture = party(&M2_BASE, false, BOT_TUNING);
static M2_ASSIST: PartyFixture = party(&M2_BASE, false, ASSIST_TUNING);
static M2_2P: PartyFixture = party(&M2_BASE, true, BOT_TUNING);
static PINCER_SOLO: PartyFixture = party(&PINCER_BASE, false, BOT_TUNING);
static PINCER_ASSIST: PartyFixture = party(&PINCER_BASE, false, ASSIST_TUNING);
static PINCER_2P: PartyFixture = party(&PINCER_BASE, true, BOT_TUNING);

fn fixture_for(stage: Stage, seat: Seat) -> &'static PartyFixture {
    match (stage, seat) {
        (Stage::M1, Seat::SoloDonutBot | Seat::SoloCarl) => &M1_SOLO,
        (Stage::M1, Seat::SoloDonutAssist) => &M1_ASSIST,
        (Stage::M1, Seat::TwoPlayer) => &M1_2P,
        (Stage::M2, Seat::SoloDonutBot | Seat::SoloCarl) => &M2_SOLO,
        (Stage::M2, Seat::SoloDonutAssist) => &M2_ASSIST,
        (Stage::M2, Seat::TwoPlayer) => &M2_2P,
        (Stage::M2Pincer, Seat::SoloDonutBot | Seat::SoloCarl) => &PINCER_SOLO,
        (Stage::M2Pincer, Seat::SoloDonutAssist) => &PINCER_ASSIST,
        (Stage::M2Pincer, Seat::TwoPlayer) => &PINCER_2P,
    }
}

// ------------------------------------------------------------- variants --

/// DM-Q11 reflex evasion mode of the sweep (F6, #181 section 7 and MAG-16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Evasion {
    /// No reflex. **DM-B wiring trap:** once the reflex reads
    /// `dm_q11_evasion_mode` (the shipped default is `reflex`), [`build`] must
    /// force `off` for this variant, or the `off` rows silently gain the
    /// reflex. Add a test that the two variants differ when that lands.
    Off,
    /// Donut's 12 s post-biscuit reflex (DM-B, PR #199). **Pending**: the
    /// reflex reads nothing yet, so a run with this variant is `blocked`.
    Reflex,
}

impl Evasion {
    pub const ALL: [Evasion; 2] = [Evasion::Off, Evasion::Reflex];

    pub const fn name(self) -> &'static str {
        match self {
            Evasion::Off => "off",
            Evasion::Reflex => "reflex",
        }
    }

    /// Why this variant cannot run yet, if it cannot.
    pub const fn pending(self) -> Option<&'static str> {
        match self {
            Evasion::Off => None,
            Evasion::Reflex => Some(
                "DM-B reflex is not on main (PR #199); DM-A declares dm_q11_evasion_mode but nothing reads it; variant is a parameter only",
            ),
        }
    }
}

/// One run: everything that distinguishes it. The run id is derived from it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunSpec {
    pub stage: Stage,
    pub seat: Seat,
    pub policy: PolicyId,
    pub evasion: Evasion,
    /// Donut's start column and facing (set at tick 0).
    pub donut_x: i32,
    pub facing: i8,
    /// Each llama's initial tell delay in ticks (`Llama::cooldown` at tick 0).
    pub phase: [u16; 2],
    pub horizon: u64,
}

impl RunSpec {
    pub fn run_id(&self) -> String {
        format!(
            "{}.{}.{}.{}.x{}.f{}.p{}-{}",
            self.stage.name(),
            self.seat.name(),
            self.policy.name(),
            self.evasion.name(),
            self.donut_x,
            if self.facing >= 0 { "r" } else { "l" },
            self.phase[0],
            self.phase[1],
        )
    }

    /// The start group: every phase variant of one (stage, seat, policy,
    /// evasion, column, facing). KC19 judges a group, not a single run.
    pub fn group_id(&self) -> String {
        format!(
            "{}.{}.{}.{}.x{}.f{}",
            self.stage.name(),
            self.seat.name(),
            self.policy.name(),
            self.evasion.name(),
            self.donut_x,
            if self.facing >= 0 { "r" } else { "l" },
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
    /// Reserved: stuck-enemy detection needs the rush's definition of
    /// progress; core never emits it.
    Softlock,
    /// The start is not a legal one (no ground under the column).
    InvalidSetup,
    /// The variant cannot run yet (see [`Evasion::pending`]).
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
    pub llama_final_hp: Vec<u8>,
    pub tells: u32,
    /// The tick of the first llama tell (the attack-phase variant's effect).
    pub first_tell_tick: Option<u64>,
    pub spits: u32,
    pub tells_interrupted: u32,
    /// Spit hits on Donut / on Carl (`HeroHurt` events).
    pub donut_hurt_events: u32,
    pub carl_hurt_events: u32,
    /// Ticks on which the policy sent Donut a move or jump input.
    pub donut_move_inputs: u32,
    pub donut_displacement_px: f32,
    pub donut_start: (f32, f32),
    pub donut_final: (f32, f32),
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
    /// Donut never moved from her column (an outcome: knockback would move a
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
}

// ------------------------------------------------------------------ build --

/// Build the sim of one run: the only place game state is written, at tick 0.
///
/// `Err` carries an `invalid_setup` reason (no ground under Donut's column).
pub fn build(kit: &EnvKit, spec: &RunSpec) -> Result<PartyFixtureSim, String> {
    let fixture = fixture_for(spec.stage, spec.seat);
    let line = kit.walking_line_at(CARL_X);
    if kit.walking_line_at(spec.donut_x) != line || line.is_none() {
        return Err(format!(
            "no clean ground under Donut's column {}",
            spec.donut_x
        ));
    }
    let scene = SceneSim::new_at(kit, VIEW_H, fixture.base.carl_x)?;
    // Review L1: the CF02 rules and Donut's biscuit are declared on the state,
    // not detected from the fixture's name.
    let combat = CombatState::new(fixture.base, kit, &scene)?
        .with_cf02()
        .with_donut(fixture.donut);
    let mut sim = PartyFixtureSim::from_parts(scene, combat, fixture);
    // Setup (tick 0): Donut's column and facing.
    sim.scene.world.donut_body.x = spec.donut_x as f32;
    sim.scene.world.donut_body.facing = spec.facing;
    sim.scene.world.sync_donut_for_tests();
    // DM-B wiring point: force `dm_q11_evasion_mode` to `off` here for
    // `Evasion::Off` once the reflex reads it (see [`Evasion::Off`]).
    // Setup (tick 0): the attack-phase variant. `reset_attempt()` rebuilds the
    // llamas from the fixture, so the delay is written once, here.
    for (llama, delay) in sim.combat.llamas.iter_mut().zip(spec.phase) {
        llama.cooldown = delay;
    }
    Ok(sim)
}

/// What the policy may see of the finished state.
fn observe(sim: &PartyFixtureSim) -> Obs {
    let hero = |id: HeroId| {
        let body = sim.scene.world.body(id);
        HeroObs {
            x: body.x,
            facing: body.facing,
            health: body.health,
            grounded: body.grounded,
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
                facing: l.facing,
                health: l.health,
                mode: l.mode,
                attack_tick: l.attack_tick,
            })
            .collect(),
    }
}

/// Route per-hero inputs to the party's slots for this seat.
fn route(seat: Seat, tick: u64, inputs: HeroInputs) -> SlotInputs {
    match seat {
        Seat::SoloDonutBot | Seat::SoloDonutAssist => {
            // Slot 0 drives the controlled hero; the swap press on tick 1 makes
            // that Donut (the real swap path, no state write).
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

/// Run one spec to a terminal reason. Deterministic: the same spec gives the
/// same record, hash and event lines.
pub fn run(kit: &EnvKit, spec: &RunSpec) -> RunRecord {
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
        bolts_released: 0,
        dry_presses: 0,
        bolt_hits: 0,
        donut_melee_hits: 0,
        carl_hits: 0,
        llamas: spec.stage.llama_xs().len(),
        llamas_defeated: 0,
        kill_ticks: vec![None; spec.stage.llama_xs().len()],
        llama_final_hp: Vec::new(),
        tells: 0,
        first_tell_tick: None,
        spits: 0,
        tells_interrupted: 0,
        donut_hurt_events: 0,
        carl_hurt_events: 0,
        donut_move_inputs: 0,
        donut_displacement_px: 0.0,
        donut_start: (0.0, 0.0),
        donut_final: (0.0, 0.0),
        state_hash: 0,
        event_lines: Vec::new(),
    };
    if let Some(why) = spec.evasion.pending() {
        record.terminal = Terminal::Blocked;
        record.note = why.to_string();
        return record;
    }
    let mut sim = match build(kit, spec) {
        Ok(sim) => sim,
        Err(why) => {
            record.terminal = Terminal::InvalidSetup;
            record.note = why;
            return record;
        }
    };
    let mut policy = Policy::new(spec.policy);
    record.donut_hp_start = sim.scene.world.donut_body.health;
    record.carl_hp_start = sim.scene.world.hero.health;
    let (mana0, _) = sim.combat.donut_mana_view().unwrap_or((0, 0));
    record.mana_start = mana0;
    record.mana_min = mana0;
    let start = (sim.scene.world.donut_body.x, sim.scene.world.donut_body.y);
    record.donut_start = start;
    let enemy_count = sim.combat.enemies.len() as u32;
    // Absolute index of the next unread event (the log drops from the front).
    let mut seen = 0usize;
    let mut displacement = 0.0f32;
    let mut last = start;

    for _ in 0..spec.horizon {
        let obs = observe(&sim);
        let tick = obs.tick;
        let inputs = policy.act(&obs);
        if inputs.donut.move_x != 0.0 || inputs.donut.jump {
            record.donut_move_inputs += 1;
        }
        sim.step_slots(route(spec.seat, tick, inputs));
        record.ticks = tick;

        let first = seen.saturating_sub(sim.combat.dropped_events);
        for event in &sim.combat.events[first..] {
            record.event_lines.push(event.to_string());
            match event {
                CombatEvent::Mana(ManaEvent::ManaSpent { .. }) => {
                    record.mana_spent += 1;
                    record.bolts_released += 1;
                }
                CombatEvent::Mana(ManaEvent::ManaRegained { .. }) => record.mana_regained += 1,
                CombatEvent::Mana(ManaEvent::CastRejected { .. }) => record.dry_presses += 1,
                CombatEvent::Hit {
                    move_id, target, ..
                } if *target >= enemy_count => match move_id {
                    MoveId::MagicMissile => record.bolt_hits += 1,
                    MoveId::Swat | MoveId::Pounce => record.donut_melee_hits += 1,
                    _ => record.carl_hits += 1,
                },
                CombatEvent::LlamaTell { tick, .. } => {
                    record.tells += 1;
                    record.first_tell_tick.get_or_insert(*tick);
                }
                CombatEvent::LlamaSpit { .. } => record.spits += 1,
                CombatEvent::LlamaInterrupted { .. } => record.tells_interrupted += 1,
                CombatEvent::HeroHurt { hero, .. } => match hero {
                    HeroId::Donut => record.donut_hurt_events += 1,
                    HeroId::Carl => record.carl_hurt_events += 1,
                },
                _ => {}
            }
        }
        seen = sim.combat.dropped_events + sim.combat.events.len();

        if let Some((mana, _)) = sim.combat.donut_mana_view() {
            record.mana_min = record.mana_min.min(mana);
        }
        let now = (sim.scene.world.donut_body.x, sim.scene.world.donut_body.y);
        displacement += (now.0 - last.0).abs() + (now.1 - last.1).abs();
        last = now;
        for (i, llama) in sim.combat.llamas.iter().enumerate() {
            if llama_defeated(llama) && record.kill_ticks[i].is_none() {
                record.kill_ticks[i] = Some(tick);
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
    record.donut_hp_final = sim.scene.world.donut_body.health;
    record.carl_hp_final = sim.scene.world.hero.health;
    record.mana_final = sim.combat.donut_mana_view().map_or(0, |(m, _)| m);
    record.llama_final_hp = sim.combat.llamas.iter().map(|l| l.health).collect();
    record.donut_displacement_px = displacement;
    record.donut_final = last;
    record.state_hash = sim.state_hash();
    record
}

/// Load the corridor kit the stages run on.
pub fn load_kit(root: &Path) -> Result<EnvKit, String> {
    EnvKit::load(root, LAYOUT).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------ sweep --

/// The sweep grid: legal starts crossed with the initial attack phases.
#[derive(Debug, Clone)]
pub struct SweepConfig {
    pub stages: Vec<Stage>,
    /// Donut start columns on clean ground.
    pub xs: Vec<i32>,
    pub facings: Vec<i8>,
    /// Initial tell delays tried for each llama (crossed pairwise).
    pub phases: Vec<u16>,
    pub evasions: Vec<Evasion>,
    pub horizon: u64,
}

impl SweepConfig {
    /// The sweep of the `corral_eval` binary.
    pub fn full() -> Self {
        Self {
            stages: Stage::ALL.to_vec(),
            xs: (240..=560).step_by(40).collect(),
            facings: vec![1, -1],
            phases: vec![0, 45, 89],
            evasions: Evasion::ALL.to_vec(),
            horizon: HORIZON_TICKS,
        }
    }

    /// A coarse sweep for the unit tests (debug builds).
    pub fn quick() -> Self {
        Self {
            stages: Stage::ALL.to_vec(),
            xs: vec![300, 420, 540],
            facings: vec![1, -1],
            phases: vec![0, 89],
            evasions: vec![Evasion::Off],
            horizon: HORIZON_TICKS,
        }
    }

    /// Every spec of the sweep: Donut seats with the two stationary policies,
    /// the trivial hold, no-interaction and the mobile policy; Carl melee and
    /// the co-op probe on their own seats.
    pub fn specs(&self) -> Vec<RunSpec> {
        let mut out = Vec::new();
        for &stage in &self.stages {
            let llamas = stage.llama_xs().len();
            let phases: Vec<[u16; 2]> = if llamas == 1 {
                self.phases.iter().map(|&p| [p, 0]).collect()
            } else {
                self.phases
                    .iter()
                    .flat_map(|&a| self.phases.iter().map(move |&b| [a, b]))
                    .collect()
            };
            for &evasion in &self.evasions {
                for &x in &self.xs {
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
                                    facing,
                                    phase,
                                    horizon: self.horizon,
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
                            if x == self.xs[0] && facing == self.facings[0] {
                                push(Seat::SoloCarl, PolicyId::CarlMelee);
                            }
                            push(Seat::TwoPlayer, PolicyId::CoopProbe);
                        }
                    }
                }
            }
        }
        out
    }
}

/// Today's llamas aim every walk, facing and shot at Carl (`step_llamas`
/// passes `scene.world.hero.x`). The probe puts Donut on the far side of the
/// llama and nearer to it than Carl, runs 90 neutral ticks in two-player and
/// reports whether the llama still faces Carl. Measured, not asserted: a
/// llama that attacks only Carl is a hard system failure once CORRAL-B lands.
pub fn carl_only_targeting(kit: &EnvKit) -> Option<bool> {
    let spec = RunSpec {
        stage: Stage::M1,
        seat: Seat::TwoPlayer,
        policy: PolicyId::NoInteraction,
        evasion: Evasion::Off,
        // The llama stands at 640, Carl at 400 (left): Donut at 800 is nearer.
        donut_x: 800,
        facing: -1,
        phase: [0, 0],
        horizon: 90,
    };
    let mut sim = build(kit, &spec).ok()?;
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
    let kit = load_kit(root)?;
    let records = sweep(&kit, config);
    let targeting = carl_only_targeting(&kit);
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

/// Run every spec of a sweep.
pub fn sweep(kit: &EnvKit, config: &SweepConfig) -> Vec<RunRecord> {
    config.specs().iter().map(|spec| run(kit, spec)).collect()
}

#[cfg(test)]
mod tests;
