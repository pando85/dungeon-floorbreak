//! TROG-B: `trog_eval`, the STATIONARY-DONUT evaluator of the Test of Strength
//! (`docs/01-design/floor-01-troglodyte-integration.md` section 7.2 and 7.3, the work order
//! `docs/07-agents/floor-01-troglodyte-workorder.md`).
//!
//! It answers one question with numbers: can a Donut who stands still and presses Magic Missile
//! (a hop or a turn in place still counts as standing) defeat **both** troglodytes of the Test of
//! Strength from a natural start? Every run drives the real party simulation
//! ([`crate::trog_fixtures::TrogSim`], the live party step order, the real mana, collisions and
//! the troglodyte machine) with legal per-slot raw inputs, one tick at a time.
//!
//! It reuses `corral_eval`'s machinery: the stationary **policies** ([`policy::Policy`], through
//! the one generalisation that lets an observed target carry its own hurtbox,
//! `LlamaObs::rect`; the llama path is unchanged and the corral report stays byte-identical) and
//! the **reflex variant** ([`crate::corral_eval::Evasion`]).
//!
//! Hard rules, enforced by construction and by tests:
//!
//! * **Setup-only writes.** The only places game state is written are the tick-0 setup: Donut's
//!   column, surface and facing, the per-enemy phase (the roster's `delay_ticks`, the extra wait
//!   after a wake), the parked seat and the reflex profile. After that the only mutation is
//!   [`TrogSim::step`]. The evaluator never writes mana, health or a hero's position later and
//!   never moves a troglodyte.
//! * **No rescue, no teleport.** A stuck hero is reported as a timeout.
//!
//! **Stages.** [`Stage::Strength`] is the gated room (the Virtuoso and the Basher behind the
//! bench); [`Stage::T1`] to [`Stage::T3`] are the run's pockets, reported only (T1's Pygmy is
//! the positive control, TROG-F02).
//!
//! **Gate** ([`report::Gate`], asserted by the `trog_gate_*` tests): the hard invariant (no
//! gated stationary policy, from any natural start, in the decisive variant defeats both
//! troglodytes by a Donut-credited hit), KC19, the positive control, the movement rows (mobile
//! Donut, Carl melee and co-op each clear the room) and the hard system failures.

pub mod policy;
pub mod report;

use std::path::Path;

use crate::combat::{CombatEvent, MoveId};
use crate::combat_scene::trog::TROG_ID_BASE;
use crate::combat_scene::{BOLT_SPAWN_FORWARD, BOLT_SPAWN_UP};
use crate::corral_eval::Evasion;
use crate::corral_eval::policy::{Aim, HeroObs, LlamaObs, Obs, Policy, PolicyId};
use crate::f01_trog::TrogRoom;
use crate::input::{InputFrame, RawInput, SlotInputs};
use crate::party::HeroId;
use crate::prototype::{
    BodyDrive, HERO_HALF_HEIGHT, Hero, Rect, feet_height_above_solid, step_hero_motion,
};
use crate::rest_fixtures::{BOT, Party};
use crate::scene_enemy::SLOT_ID_MIN;
use crate::trog::{State, TROG_SOURCE_ID, TrogAct, TrogTuning};
use crate::trog_fixtures::{self, SimSetup, TrogSim};

/// Bump when a policy, a stage, a start or a metric definition changes.
pub const POLICY_VERSION: u32 = 1;
/// The KC19 clear limit and the isolated attempt horizon: 30 s at 60 Hz.
pub const HORIZON_TICKS: u64 = 1800;
/// The movement rows' clear limit (plan 7.3 row 4).
pub const MOVE_HORIZON_TICKS: u64 = 2400;
/// Donut's biscuit pool at the start of every run (the corral's M0 grants it; plan 6.3).
pub const START_MANA: u8 = 8;

// ---------------------------------------------------------------- stages --

/// A start: Donut's column and the height of her surface above the walking line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Start {
    pub x: i32,
    pub rise: i32,
}

/// The Test of Strength's surfaces above the walking line, from its sidecar (a test ties them to
/// the file so they cannot drift, TROG-AD47): the retreat shelf and the bench, both static solids.
pub const SHELF_RISE_PX: i32 = 48;
pub const BENCH_RISE_PX: i32 = 80;

const fn at(x: i32, rise: i32) -> Start {
    Start { x, rise }
}

/// A stage of the sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Stage {
    /// The Test of Strength (gated): the Virtuoso at 496 and the Basher at 800 behind the bench.
    Strength,
    /// `trog_run` T1, the Pygmy (the positive control, reported).
    T1,
    /// `trog_run` T2, the Basher (reported).
    T2,
    /// `trog_run` T3, the Virtuoso (reported).
    T3,
}

impl Stage {
    pub const ALL: [Stage; 4] = [Stage::Strength, Stage::T1, Stage::T2, Stage::T3];

    pub const fn name(self) -> &'static str {
        match self {
            Stage::Strength => "trog_strength",
            Stage::T1 => "trog_run_t1",
            Stage::T2 => "trog_run_t2",
            Stage::T3 => "trog_run_t3",
        }
    }

    pub const fn room(self) -> TrogRoom {
        match self {
            Stage::Strength => TrogRoom::Strength,
            _ => TrogRoom::Run,
        }
    }

    /// The roster indices whose defeat is the stage's clear.
    pub const fn watch(self) -> &'static [usize] {
        match self {
            Stage::Strength => &[0, 1],
            Stage::T1 => &[0],
            Stage::T2 => &[1],
            Stage::T3 => &[2],
        }
    }

    /// The gate is defined on the Test of Strength; the run's pockets are reported.
    pub const fn gated(self) -> bool {
        matches!(self, Stage::Strength)
    }

    /// The natural starts (plan 7.3): the spawn, the shelf top, the bench top (80 px, TROG-AD46,
    /// TROG-AD47), the far landing (Donut on the floor past the bench) and the exit apron (the
    /// exit gate stands at 944, so the apron start is 928, TROG-AD42); the run's pockets from the plan's reported rows.
    pub fn natural_starts(self) -> Vec<Start> {
        match self {
            Stage::Strength => vec![
                at(80, 0),
                at(320, SHELF_RISE_PX),
                at(656, BENCH_RISE_PX),
                at(720, 0),
                at(928, 0),
            ],
            Stage::T1 => vec![at(432, 0), at(384, 32)],
            Stage::T2 => vec![at(1056, 0), at(1280, 0)],
            Stage::T3 => vec![at(1760, 0), at(1872, 32), at(1952, 64)],
        }
    }

    /// The diagnostic starts (reported, not gated): a 16 px grid over the floor 240..928 plus the
    /// shelf and bench tops (plan 7.3). Starts inside a solid are not legal and are skipped by
    /// the runner.
    pub fn diagnostic_starts(self) -> Vec<Start> {
        if self != Stage::Strength {
            return Vec::new();
        }
        let mut out: Vec<Start> = (240..=928).step_by(16).map(|x| at(x, 0)).collect();
        out.extend((288..=352).step_by(16).map(|x| at(x, SHELF_RISE_PX)));
        out.extend((640..=672).step_by(16).map(|x| at(x, BENCH_RISE_PX)));
        out
    }
}

/// The per-enemy phase variants P0..P2: the extra wait after a wake (the roster's `delay_ticks`,
/// plan 7.2 "written at setup as the first rest counter"), by roster index.
pub const PHASES: [[u32; 3]; 3] = [[0, 0, 0], [30, 60, 15], [60, 0, 45]];

// ----------------------------------------------------------------- seats --

/// Who plays and who stands in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Seat {
    /// Solo as Donut (swapped on tick 1); Carl is **parked** in the protected spawn, where a
    /// troglodyte never tells. The decisive `isolated` variant.
    Isolated,
    /// Solo as Donut with the follow-and-jump bot as Carl (it never attacks): reported.
    Default,
    /// Solo as Carl; Donut parked.
    CarlSolo,
    /// P1 Carl and P2 Donut.
    TwoPlayer,
}

impl Seat {
    pub const fn name(self) -> &'static str {
        match self {
            Seat::Isolated => "isolated",
            Seat::Default => "default",
            Seat::CarlSolo => "carl_solo",
            Seat::TwoPlayer => "two_player",
        }
    }

    const fn donut_solo(self) -> bool {
        matches!(self, Seat::Isolated | Seat::Default)
    }
}

// -------------------------------------------------------------- policies --

/// What drives the tested hero(es).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EvalPolicy {
    /// One of `corral_eval`'s stationary policies (gated when
    /// [`PolicyId::is_stationary_gate`]).
    Stationary(PolicyId),
    /// Donut moves, hops the statics and fights with the caster policy (Carl parked).
    MobileCast,
    /// Carl fights with the brawler policy and hops the statics (Donut parked).
    CarlMelee,
    /// P1 Carl melee and P2 Donut stationary cast on the same ticks.
    CoopProbe,
}

impl EvalPolicy {
    pub fn name(self) -> String {
        match self {
            EvalPolicy::Stationary(id) => id.name().to_owned(),
            EvalPolicy::MobileCast => "mobile_cast".to_owned(),
            EvalPolicy::CarlMelee => "carl_melee".to_owned(),
            EvalPolicy::CoopProbe => "coop_probe".to_owned(),
        }
    }

    pub const fn is_stationary_gate(self) -> bool {
        match self {
            EvalPolicy::Stationary(id) => id.is_stationary_gate(),
            _ => false,
        }
    }

    /// The seat a movement policy runs in.
    pub const fn movement_seat(self) -> Option<Seat> {
        match self {
            EvalPolicy::MobileCast => Some(Seat::Isolated),
            EvalPolicy::CarlMelee => Some(Seat::CarlSolo),
            EvalPolicy::CoopProbe => Some(Seat::TwoPlayer),
            EvalPolicy::Stationary(_) => None,
        }
    }
}

// ------------------------------------------------------------------ runs --

/// One run: everything that distinguishes it. The run id is derived from it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunSpec {
    pub stage: Stage,
    pub seat: Seat,
    pub policy: EvalPolicy,
    pub evasion: Evasion,
    pub start: Start,
    pub facing: i8,
    /// The phase variant P0..P2 (an index into [`PHASES`]).
    pub phase: usize,
    /// `stationary_jump_cast` only: Primary press tick minus Jump press tick.
    pub jump_offset: i8,
    pub horizon: u64,
}

impl RunSpec {
    fn policy_label(&self) -> String {
        match self.policy {
            EvalPolicy::Stationary(PolicyId::StationaryJumpCast) => {
                format!("stationary_jump_cast.o{}", self.jump_offset)
            }
            other => other.name(),
        }
    }

    /// The start group: every phase variant of one (stage, seat, policy, evasion, column,
    /// surface, facing). KC19 judges a group, not a single run.
    pub fn group_id(&self) -> String {
        format!(
            "{}.{}.{}.{}.x{}.h{}.f{}",
            self.stage.name(),
            self.seat.name(),
            self.policy_label(),
            self.evasion.name(),
            self.start.x,
            self.start.rise,
            if self.facing >= 0 { "r" } else { "l" },
        )
    }

    pub fn run_id(&self) -> String {
        format!("{}.p{}", self.group_id(), self.phase)
    }
}

/// Why a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminal {
    /// Every watched troglodyte defeated.
    Victory,
    /// A hero reached 0 health (the party's defeat).
    HeroDefeat,
    /// The horizon elapsed.
    Timeout,
    /// The start is not a legal one (inside a solid or no surface under the column).
    InvalidSetup,
}

impl Terminal {
    pub const fn name(self) -> &'static str {
        match self {
            Terminal::Victory => "victory",
            Terminal::HeroDefeat => "hero_defeat",
            Terminal::Timeout => "timeout",
            Terminal::InvalidSetup => "invalid_setup",
        }
    }
}

/// The hard system failures of plan 7.3 row 5, counted per run (the bench rows are gone with the
/// static fallback, TROG-AD43).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystemFailures {
    /// A blow on a hero while its troglodyte was outside its active strike window.
    pub blow_before_window: u32,
    /// A tell against a target standing in the protected spawn.
    pub tell_in_protected: u32,
    /// A proximity wake, or any tell, with the body outside the view the slot step read.
    pub offscreen: u32,
    /// A tell whose target's hurtbox cannot meet the species' band (TROG-AD17).
    pub unreachable_target: u32,
    /// A troglodyte outside its leash.
    pub outside_leash: u32,
}

impl SystemFailures {
    pub fn total(&self) -> u32 {
        self.blow_before_window
            + self.tell_in_protected
            + self.offscreen
            + self.unreachable_target
            + self.outside_leash
    }

    pub fn describe(&self) -> String {
        format!(
            "blow_before_window={} tell_in_protected={} offscreen={} unreachable_target={} outside_leash={}",
            self.blow_before_window,
            self.tell_in_protected,
            self.offscreen,
            self.unreachable_target,
            self.outside_leash
        )
    }
}

/// The metrics of one run.
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
    /// Press edges and jump presses the policy issued.
    pub presses: u32,
    pub jump_presses: u32,
    /// Magic Missile hits on a troglodyte, and hits by the other moves.
    pub bolt_hits: u32,
    pub donut_melee_hits: u32,
    pub carl_hits: u32,
    /// The watched troglodytes, the tick each was defeated on (`None`: alive at the end) and
    /// whether a player-credited hit defeated it.
    pub kill_ticks: Vec<Option<u64>>,
    pub kill_by_player: Vec<bool>,
    pub final_hp: Vec<u16>,
    pub tells: u32,
    pub strikes: u32,
    pub whiffs: u32,
    pub interrupted: u32,
    pub wakes_proximity: u32,
    pub wakes_hit: u32,
    pub donut_hurt_events: u32,
    pub carl_hurt_events: u32,
    pub evades: u32,
    pub donut_start: (f32, f32),
    pub donut_final: (f32, f32),
    pub donut_displacement_px: f32,
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

    pub fn watched(&self) -> usize {
        self.kill_ticks.len()
    }

    pub fn defeated(&self) -> usize {
        self.kill_ticks.iter().flatten().count()
    }

    /// Every watched troglodyte was defeated.
    pub fn all_defeated(&self) -> bool {
        self.watched() > 0 && self.defeated() == self.watched()
    }

    /// Every watched troglodyte was defeated by a player-credited hit.
    pub fn all_defeated_by_player(&self) -> bool {
        self.all_defeated() && self.kill_by_player.iter().all(|k| *k)
    }

    /// Carl's move dealt damage while a human played Donut (companion-assisted, reported).
    pub fn companion_assisted(&self) -> bool {
        self.spec.seat == Seat::Default && self.carl_hits > 0
    }

    /// The most damage of both heroes.
    pub fn hp_lost(&self) -> u8 {
        self.donut_hp_lost().max(self.carl_hp_lost())
    }
}

// ----------------------------------------------------------------- build --

fn party_of(seat: Seat) -> (Party, Option<HeroId>) {
    match seat {
        Seat::Isolated => (Party::Solo { uncontrolled: BOT }, Some(HeroId::Carl)),
        Seat::Default => (Party::Solo { uncontrolled: BOT }, None),
        Seat::CarlSolo => (Party::Solo { uncontrolled: BOT }, Some(HeroId::Donut)),
        Seat::TwoPlayer => (Party::TwoPlayer, None),
    }
}

/// The world y of the top of an intact solid under the pivot at `feet`, with the hero's whole
/// body box clear of every other solid above it.
fn legal_start(sim: &TrogSim, x: f32, feet: f32) -> bool {
    let solids = &sim.scene.world.solids;
    let supported = solids.iter().any(|b| {
        b.intact && (b.rect.top() - feet).abs() < 0.01 && b.rect.left <= x && x <= b.rect.right()
    });
    let body = Rect {
        left: x - 10.0,
        bottom: feet + 0.5,
        width: 20.0,
        height: 2.0 * HERO_HALF_HEIGHT - 1.0,
    };
    supported && !solids.iter().any(|b| b.intact && b.rect.overlaps(body))
}

/// Build the sim of one run: the only place game state is written, at tick 0.
pub fn build(root: &Path, spec: &RunSpec) -> Result<TrogSim, String> {
    let (party, parked) = party_of(spec.seat);
    let mut setup = SimSetup::story(spec.stage.room(), party, parked);
    let roster = spec.stage.room().roster_len();
    setup.delays = (0..roster).map(|i| PHASES[spec.phase][i % 3]).collect();
    setup.reflex = Some(spec.evasion.profile());
    let mut sim = TrogSim::build(root, &setup)?;
    if spec.seat.donut_solo() {
        // The real swap path (a press on tick 1), no state write.
        sim.swap_to_donut();
    } else if spec.seat == Seat::TwoPlayer {
        // The second player joins on the first tick.
        sim.step([RawInput::NONE, RawInput::NONE]);
    }
    // Setup (tick 0): the tested Donut's column, surface and facing (a two-player Donut too: a
    // Donut left at the spawn would pin the shared screen).
    if spec.seat.donut_solo() || spec.seat == Seat::TwoPlayer {
        let floor = sim.foot_of_floor();
        let feet = floor + spec.start.rise as f32;
        if !legal_start(&sim, spec.start.x as f32, feet) {
            return Err(format!(
                "no legal surface {} px up under Donut's column {}",
                spec.start.rise, spec.start.x
            ));
        }
        let body = sim.scene.world.body_mut(HeroId::Donut);
        body.x = spec.start.x as f32;
        body.y = feet + HERO_HALF_HEIGHT;
        body.vx = 0.0;
        body.vy = 0.0;
        body.facing = spec.facing;
    }
    Ok(sim)
}

// -------------------------------------------------------------- observing --

fn support_of(body: &Hero, solids: &[crate::prototype::Block]) -> f32 {
    let feet = body.y - HERO_HALF_HEIGHT;
    feet - feet_height_above_solid(body, solids).unwrap_or(0.0)
}

/// What the policy may see of the finished state.
fn observe(sim: &TrogSim) -> Obs {
    let solids = &sim.scene.world.solids;
    let t = TrogTuning::active();
    let hero = |id: HeroId| {
        let body = sim.scene.world.body(id);
        HeroObs {
            x: body.x,
            facing: body.facing,
            health: body.health,
            grounded: body.grounded,
            feet: body.y - HERO_HALF_HEIGHT,
            support: support_of(body, solids),
        }
    };
    let trogs = (0..sim.combat.slot_enemies().len()).map(|i| {
        let trog = sim.trog(i);
        LlamaObs {
            x: trog.x,
            feet: trog.foot_y + trog.lift(),
            facing: trog.facing,
            health: u8::try_from(trog.hp).unwrap_or(u8::MAX),
            mode: if trog.alive() {
                crate::llama::LlamaMode::Neutral
            } else {
                crate::llama::LlamaMode::Down
            },
            attack_tick: 0,
            rush: None,
            rect: trog.hurtbox(t),
        }
    });
    Obs {
        tick: sim.scene.world.tick + 1,
        carl: hero(HeroId::Carl),
        donut: hero(HeroId::Donut),
        mana: sim.combat.donut_mana_view(),
        cast_active: sim.combat.donut_cast().is_some(),
        donut_hurt: sim.combat.hurt_ticks_of(HeroId::Donut) > 0,
        llamas: trogs.collect(),
    }
}

/// The static facts the policies read: the room's solids, the bolt's flight and Donut's own jump
/// in place from her start, computed on a copy of her body with the real hero step.
fn aim_of(sim: &TrogSim) -> Aim {
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
    Aim {
        solids,
        bolt_forward: BOLT_SPAWN_FORWARD,
        bolt_up: BOLT_SPAWN_UP,
        bolt_step: f32::from(tuning.projectile_speed_px_per_tick),
        bolt_range: f32::from(tuning.projectile_range_px),
        arc,
        route: None,
    }
}

// ---------------------------------------------------------------- running --

/// Whether `hero` is inside the protected spawn range of the room.
fn in_protected(sim: &TrogSim, x: f32) -> bool {
    let p = sim.run.loaded.encounter.protected_spawn;
    (p.x as f32..=p.right() as f32).contains(&x)
}

/// Run one spec to a terminal reason. Deterministic: the same spec gives the same record, hash
/// and event lines.
pub fn run(root: &Path, spec: &RunSpec) -> RunRecord {
    let watch = spec.stage.watch();
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
        presses: 0,
        jump_presses: 0,
        bolt_hits: 0,
        donut_melee_hits: 0,
        carl_hits: 0,
        kill_ticks: vec![None; watch.len()],
        kill_by_player: vec![false; watch.len()],
        final_hp: Vec::new(),
        tells: 0,
        strikes: 0,
        whiffs: 0,
        interrupted: 0,
        wakes_proximity: 0,
        wakes_hit: 0,
        donut_hurt_events: 0,
        carl_hurt_events: 0,
        evades: 0,
        donut_start: (0.0, 0.0),
        donut_final: (0.0, 0.0),
        donut_displacement_px: 0.0,
        failures: SystemFailures::default(),
        state_hash: 0,
        event_lines: Vec::new(),
    };
    let mut sim = match build(root, spec) {
        Ok(sim) => sim,
        Err(why) => {
            record.terminal = Terminal::InvalidSetup;
            record.note = why;
            return record;
        }
    };
    let stationary = match spec.policy {
        EvalPolicy::Stationary(id) => Some(id),
        EvalPolicy::CoopProbe => Some(PolicyId::StationaryCast),
        _ => None,
    };
    let mut policy = stationary.map(|id| Policy::with_aim(id, spec.jump_offset, aim_of(&sim)));
    let mut brain = policy::Brain::default();
    record.donut_hp_start = sim.health(HeroId::Donut);
    record.carl_hp_start = sim.health(HeroId::Carl);
    let t = TrogTuning::active();
    let start_tick = sim.tick();
    record.donut_start = (sim.x_of(HeroId::Donut), sim.foot_of(HeroId::Donut));
    let mut seen = sim.combat.events.len() + sim.combat.dropped_events;
    let mut last_x = record.donut_start.0;

    for _ in 0..spec.horizon {
        let inputs: SlotInputs = match spec.policy {
            EvalPolicy::Stationary(_) => {
                let obs = observe(&sim);
                let mut raw = policy
                    .as_mut()
                    .expect("a stationary policy")
                    .act(&obs)
                    .donut;
                raw.swap = false;
                [raw, RawInput::NONE]
            }
            EvalPolicy::MobileCast => [brain.mobile_cast(&sim), RawInput::NONE],
            EvalPolicy::CarlMelee => [brain.carl_melee(&sim), RawInput::NONE],
            EvalPolicy::CoopProbe => {
                let obs = observe(&sim);
                let donut = policy.as_mut().expect("a policy").act(&obs).donut;
                [brain.carl_melee(&sim), donut]
            }
        };
        sim.step(inputs);
        record.ticks = sim.tick() - start_tick;

        // Per tick: the leash, the events of the tick.
        for i in 0..sim.combat.slot_enemies().len() {
            let trog = sim.trog(i);
            let (l0, l1) = trog.leash();
            if trog.alive() && (trog.x < l0 - 0.01 || trog.x > l1 + 0.01) {
                record.failures.outside_leash += 1;
            }
        }
        let first = seen.saturating_sub(sim.combat.dropped_events);
        let new: Vec<CombatEvent> =
            sim.combat.events[first.min(sim.combat.events.len())..].to_vec();
        seen = sim.combat.events.len() + sim.combat.dropped_events;
        for event in &new {
            record.event_lines.push(event.to_string());
            match *event {
                CombatEvent::Hit {
                    move_id, target, ..
                } if target >= SLOT_ID_MIN => match move_id {
                    MoveId::MagicMissile => record.bolt_hits += 1,
                    MoveId::Swat | MoveId::Pounce => record.donut_melee_hits += 1,
                    _ => record.carl_hits += 1,
                },
                CombatEvent::Trog { enemy, act, .. } => {
                    let _ = enemy;
                    match act {
                        TrogAct::Tell { .. } => record.tells += 1,
                        TrogAct::Strike => record.strikes += 1,
                        TrogAct::Whiff => record.whiffs += 1,
                        TrogAct::Interrupted => record.interrupted += 1,
                        TrogAct::Wake { cause } => match cause {
                            crate::trog::WakeCause::Proximity => record.wakes_proximity += 1,
                            crate::trog::WakeCause::Hit => record.wakes_hit += 1,
                        },
                        _ => {}
                    }
                }
                CombatEvent::EnemyDefeated {
                    tick,
                    target,
                    instigator,
                    ..
                } if target >= TROG_ID_BASE => {
                    let index = (target - TROG_ID_BASE) as usize;
                    if let Some(slot) = watch.iter().position(|w| *w == index) {
                        record.kill_ticks[slot] = Some(tick);
                        record.kill_by_player[slot] = instigator.is_some();
                    }
                }
                CombatEvent::HeroHurt { source, hero, .. } => {
                    match hero {
                        HeroId::Donut => record.donut_hurt_events += 1,
                        HeroId::Carl => record.carl_hurt_events += 1,
                    }
                    if source & 0xFFFF == TROG_SOURCE_ID {
                        let owner = (source >> 16) as usize;
                        let sp = t.species(sim.trog(owner).species);
                        let a = sim.trog(owner).attack_tick();
                        let inside = sim.trog(owner).state == State::Active
                            && (sp.strike_start..=sp.strike_end).contains(&a);
                        if !inside {
                            record.failures.blow_before_window += 1;
                        }
                    }
                }
                CombatEvent::HeroEvaded { .. } => record.evades += 1,
                _ => {}
            }
        }
        let dx = (sim.x_of(HeroId::Donut) - last_x).abs();
        if spec.seat.donut_solo() {
            record.donut_displacement_px += dx;
        }
        last_x = sim.x_of(HeroId::Donut);

        if sim.hero_defeats() > 0 {
            record.terminal = Terminal::HeroDefeat;
            break;
        }
        if watch.iter().all(|w| !sim.trog(*w).alive()) {
            record.terminal = Terminal::Victory;
            break;
        }
    }

    // The tells and wakes the scene saw (the F07 rules, plan 7.3 row 5).
    for fact in &sim.tell_log {
        if in_protected(&sim, fact.target_x) {
            record.failures.tell_in_protected += 1;
        }
        if fact.body.0 < fact.view.0 || fact.body.1 > fact.view.1 {
            record.failures.offscreen += 1;
        }
        if !fact.meets_band {
            record.failures.unreachable_target += 1;
        }
    }
    for fact in &sim.wake_log {
        if fact.body.0 < fact.view.0 || fact.body.1 > fact.view.1 {
            record.failures.offscreen += 1;
        }
    }
    record.donut_hp_final = sim.health(HeroId::Donut);
    record.carl_hp_final = sim.health(HeroId::Carl);
    record.final_hp = watch.iter().map(|w| sim.trog(*w).hp).collect();
    record.donut_final = (sim.x_of(HeroId::Donut), sim.foot_of(HeroId::Donut));
    record.state_hash = sim.hash();
    if let Some(p) = &policy {
        record.presses = p.presses;
        record.jump_presses = p.jumps;
    }
    record
}

// ------------------------------------------------------------------ sweep --

/// What a sweep covers.
#[derive(Debug, Clone, PartialEq)]
pub struct SweepConfig {
    /// Run the gated Test of Strength.
    pub strength: bool,
    /// Run the run's pockets (reported).
    pub pockets: bool,
    /// Run the 16 px diagnostic grid (reported).
    pub grid: bool,
    /// Sweep the `stationary_jump_cast` offsets (-3..=52) in the decisive variant.
    pub jump_sweep: bool,
    /// Run every gated spec twice and compare the hash and the event lines.
    pub double_run: bool,
}

impl SweepConfig {
    pub const fn full() -> Self {
        Self {
            strength: true,
            pockets: true,
            grid: true,
            jump_sweep: true,
            double_run: true,
        }
    }

    /// The gate alone: the natural starts of the Test of Strength, the aimed jump-cast, no grid,
    /// no pockets.
    pub const fn quick() -> Self {
        Self {
            strength: true,
            pockets: false,
            grid: false,
            jump_sweep: false,
            double_run: true,
        }
    }
}

/// The stationary policies of the sweep: the gated fixed ones.
fn fixed_policies() -> Vec<EvalPolicy> {
    PolicyId::GATED_FIXED
        .iter()
        .map(|id| EvalPolicy::Stationary(*id))
        .collect()
}

fn spec(
    stage: Stage,
    seat: Seat,
    policy: EvalPolicy,
    evasion: Evasion,
    start: Start,
    facing: i8,
    phase: usize,
) -> RunSpec {
    RunSpec {
        stage,
        seat,
        policy,
        evasion,
        start,
        facing,
        phase,
        jump_offset: 0,
        horizon: HORIZON_TICKS,
    }
}

/// Every spec of a sweep, in a fixed order.
pub fn specs(config: &SweepConfig) -> Vec<RunSpec> {
    let mut out = Vec::new();
    let mut stages: Vec<Stage> = Vec::new();
    if config.strength {
        stages.push(Stage::Strength);
    }
    if config.pockets {
        stages.extend([Stage::T1, Stage::T2, Stage::T3]);
    }
    for stage in stages {
        for start in stage.natural_starts() {
            for facing in [1i8, -1] {
                for evasion in Evasion::ALL {
                    for seat in [Seat::Isolated, Seat::Default] {
                        for policy in fixed_policies() {
                            for phase in 0..3 {
                                out.push(spec(stage, seat, policy, evasion, start, facing, phase));
                            }
                        }
                        // The swept jump-cast: the decisive variant only.
                        if config.jump_sweep && seat == Seat::Isolated && evasion == Evasion::Reflex
                        {
                            for o in crate::corral_eval::policy::JUMP_OFFSETS {
                                for phase in 0..3 {
                                    let mut s = spec(
                                        stage,
                                        seat,
                                        EvalPolicy::Stationary(PolicyId::StationaryJumpCast),
                                        evasion,
                                        start,
                                        facing,
                                        phase,
                                    );
                                    s.jump_offset = o;
                                    out.push(s);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // The movement rows (plan 7.3 row 4): the spawn, no placement, the shipped reflex.
    if config.strength {
        for policy in [
            EvalPolicy::MobileCast,
            EvalPolicy::CarlMelee,
            EvalPolicy::CoopProbe,
        ] {
            for phase in 0..3 {
                let seat = policy.movement_seat().expect("a movement policy");
                // The co-op Donut stands at 400 (a setup write): left at the spawn she would pin
                // the shared screen and Carl could not scroll on.
                let start = if seat == Seat::TwoPlayer {
                    at(400, 0)
                } else {
                    at(80, 0)
                };
                let mut s = spec(
                    Stage::Strength,
                    seat,
                    policy,
                    Evasion::Reflex,
                    start,
                    1,
                    phase,
                );
                s.horizon = MOVE_HORIZON_TICKS;
                out.push(s);
            }
        }
    }
    // The diagnostic grid (reported): the aimed policies, the decisive variant.
    if config.grid {
        let naturals = Stage::Strength.natural_starts();
        for start in Stage::Strength.diagnostic_starts() {
            // A natural start is already a gated group: the grid adds only the others.
            if naturals.contains(&start) {
                continue;
            }
            for facing in [1i8, -1] {
                for id in [
                    PolicyId::StationaryCastAimed,
                    PolicyId::StationaryTurret,
                    PolicyId::StationaryJumpCastAimed,
                ] {
                    for phase in 0..3 {
                        out.push(spec(
                            Stage::Strength,
                            Seat::Isolated,
                            EvalPolicy::Stationary(id),
                            Evasion::Reflex,
                            start,
                            facing,
                            phase,
                        ));
                    }
                }
            }
        }
    }
    out
}

/// The worker threads of a sweep: `TROG_EVAL_THREADS`, else up to 8.
fn threads() -> usize {
    std::env::var("TROG_EVAL_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get().min(8))
                .unwrap_or(1)
        })
}

/// A run plus the double-run verdict (`None`: not repeated).
#[derive(Debug, Clone, PartialEq)]
pub struct Checked {
    pub record: RunRecord,
    /// The second run's hash and event lines equal the first's.
    pub identical: Option<bool>,
}

fn run_checked(root: &Path, spec: &RunSpec, double: bool) -> Checked {
    let record = run(root, spec);
    let identical = (double && record.terminal != Terminal::InvalidSetup).then(|| {
        let again = run(root, spec);
        again.state_hash == record.state_hash && again.event_lines == record.event_lines
    });
    Checked { record, identical }
}

/// Run `specs` on worker threads (worker `k` takes specs `k`, `k + n`, ...); the records keep the
/// spec order.
pub fn run_all(root: &Path, specs: &[RunSpec], double_gated: bool) -> Vec<Checked> {
    let n = threads().min(specs.len()).max(1);
    let mut slots: Vec<Option<Checked>> = vec![None; specs.len()];
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..n)
            .map(|k| {
                scope.spawn(move || {
                    (k..specs.len())
                        .step_by(n)
                        .map(|i| {
                            let s = &specs[i];
                            let gated = s.stage.gated()
                                && (s.policy.is_stationary_gate()
                                    || s.policy.movement_seat().is_some());
                            (i, run_checked(root, s, double_gated && gated))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for handle in handles {
            for (i, checked) in handle.join().expect("a sweep worker") {
                slots[i] = Some(checked);
            }
        }
    });
    slots
        .into_iter()
        .map(|c| c.expect("every spec ran"))
        .collect()
}

/// Run a whole sweep and judge it.
pub fn evaluate(root: &Path, config: &SweepConfig) -> report::Evaluation {
    let specs = specs(config);
    let checked = run_all(root, &specs, config.double_run);
    report::Evaluation::judge(config, checked)
}

/// Write `summary.json` and `report.txt` of a judged sweep into `out`.
pub fn write_to(evaluation: &report::Evaluation, out: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let write = |name: &str, text: &str| {
        std::fs::write(out.join(name), text).map_err(|e| format!("{name}: {e}"))
    };
    write("summary.json", &evaluation.json())?;
    write("report.txt", &evaluation.text())
}

/// Whether a fixture the evaluator shares with `trog_fixtures` agrees: the positive control is
/// TROG-F02 itself.
pub fn positive_control_fixture(root: &Path) -> Result<bool, String> {
    let (_, sim) = trog_fixtures::run(root, trog_fixtures::Case::PygmyStationaryDonut)?;
    Ok(!sim.trog(0).alive() && sim.health(HeroId::Donut) >= 3)
}

#[cfg(test)]
mod tests;
