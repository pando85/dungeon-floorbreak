//! DOZER-CORE fixtures: room 3's revenge chase under the K5 matrix, the solo
//! rescue, the escape end, the visible lead, the hull (DOZR-05) and the retry
//! (DOZR-07) (`docs/01-design/enemy-gameplay-dozer.md`, sections 10 and 14).
//!
//! Every run builds the room through [`crate::f01_dozer`] (the machine spawns
//! from the sidecar roster through ENEMY-SLOT) under the story loadout and the
//! sidecar's pin. The heroes are driven by small direct pilots (the room has no
//! enemy to fight); the uncontrolled hero of a solo run is the CF02 companion
//! (`carl_solo`: Donut autonomous, `cf02_free`; `donut_solo`: Carl assisted,
//! `cf02_assist`, after one swap at [`SWAP_TICK`]).
//!
//! # The K5 matrix
//!
//! `dz_k5_{carl_solo,donut_solo,two_player}_{run,stall,close_call,wait}_s0..s8`.
//! Each seed also spreads the timing ([`Vary::of_seed`]): the hold columns,
//! the release gap, a reaction delay and the `donut_solo` swap tick.
//!
//! Hard asserts ([`K5Report::check`]): no hero touched (no hurt, no defeat, no
//! wheel hit), every hero alive and out of the machine's reach when the room
//! completes (DZ-IMPL-AD14: "every hero reaches the exit" is read as the room's
//! completion with every body alive and ahead of the wheel, or the wheel off.
//! The scene freezes when the first hero overlaps the exit, and nothing can
//! touch the trailing hero after that), the machine's speed at most the
//! catch-up 185 at every tick, its cruise target at most 170, and its speed in
//! the cruise band at most 170 once it has been there for
//! [`cruise_settle_ticks`] (DZ-IMPL-AD15: the speed sheds the catch-up surplus
//! at the acceleration, so "≤ 170 at every tick" cannot hold on the first
//! cruise ticks), no bot ever touched by the wheel, no bot centre behind the
//! axle for more than one tick while the wheel is lethal, no hero stuck for
//! more than [`STUCK_LIMIT`] ticks, and each band entered across the matrix.
//! Reported only: ticks per band, rescues per bot, the companion's off-screen
//! regroups during the chase (those toward the machine are the CA2-DZ count of
//! DZ-Q24, [`ca2dz_verdict`]), the rescue-then-regroup cycles (section 7.3)
//! and the nav feed's changes.

use std::path::Path;

use crate::boss_scene::Pilot;
use crate::carl_prog::CarlProgFlags;
use crate::combat::CombatEvent;
use crate::combat_scene::dozer::DozerEnemy;
use crate::companion::events::{CompanionEvent, RegroupOutcome};
use crate::donut_mana::DonutSetup;
use crate::donut_prog::DonutProgFlags;
use crate::dozer::{Band, Dozer, DozerEvent, DozerState, EscapePhase};
use crate::f01_dozer::{self, DozerRun};
use crate::f01_late::Loadout;
use crate::f01_mastery_fixtures::{APART_PX, STUCK_LIMIT, Who as MasteryWho, tuning};
use crate::input::RawInput;
use crate::party::HeroId;
use crate::party_scene::{PartyFixture, PartyFixtureSim};
use crate::prototype::HERO_HALF_WIDTH;
use crate::scene_enemy::{SlotSet, spawn_from_roster};

/// The seeds of the matrix.
pub const SEEDS: std::ops::Range<u64> = 0..9;
/// A run's tick budget.
pub const RUN_TICKS: u64 = 3000;
/// `donut_solo` swaps to Donut on this tick.
pub const SWAP_TICK: u64 = 30;
/// The x the stall and the close call hold at.
pub const HOLD_X: f32 = 480.0;
/// The x the wait holds at (the resting machine's origin).
pub const WAIT_X: f32 = 832.0;
/// How long the stall holds.
pub const STALL_TICKS: u32 = 30;
/// The close call and the wait run once the wheel is this close (body left
/// minus the wheel front).
pub const CLOSE_GAP_PX: f32 = 64.0;

/// Who plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Carl steered, Donut the autonomous companion (`cf02_free`).
    CarlSolo,
    /// Donut steered after one swap, Carl the assisted companion (`cf02_assist`).
    DonutSolo,
    /// Both steered.
    TwoPlayer,
}

impl Mode {
    pub const ALL: [Self; 3] = [Self::CarlSolo, Self::DonutSolo, Self::TwoPlayer];

    pub const fn name(self) -> &'static str {
        match self {
            Self::CarlSolo => "carl_solo",
            Self::DonutSolo => "donut_solo",
            Self::TwoPlayer => "two_player",
        }
    }
}

/// What the steered heroes do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plan {
    /// Run to the exit.
    Run,
    /// Stop [`STALL_TICKS`] at [`HOLD_X`], then run.
    Stall,
    /// Hold at [`HOLD_X`] until the wheel is [`CLOSE_GAP_PX`] away, then run.
    CloseCall,
    /// Hold at [`WAIT_X`] until the wheel is [`CLOSE_GAP_PX`] away, then run.
    Wait,
}

impl Plan {
    pub const ALL: [Self; 4] = [Self::Run, Self::Stall, Self::CloseCall, Self::Wait];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Stall => "stall",
            Self::CloseCall => "close_call",
            Self::Wait => "wait",
        }
    }
}

/// The per-seed timing spread of a K5 cell: the hold column's offset, the
/// release gap, the reaction delay after the release condition and the
/// `donut_solo` swap tick. [`Vary::BASE`] is the scenarios' fixed timing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vary {
    pub hold_dx: f32,
    pub close_gap: f32,
    pub delay: u32,
    pub swap_tick: u64,
}

impl Vary {
    /// The scenarios' timing: [`HOLD_X`], [`WAIT_X`], [`CLOSE_GAP_PX`], no
    /// delay, [`SWAP_TICK`].
    pub const BASE: Self = Self {
        hold_dx: 0.0,
        close_gap: CLOSE_GAP_PX,
        delay: 0,
        swap_tick: SWAP_TICK,
    };

    /// Seed `s` of the matrix: the hold columns move by (s − 4)·16 px, the
    /// release gap is 48 + 6s px, the reaction delay s ticks and the swap tick
    /// 30 + 5s.
    pub fn of_seed(seed: u64) -> Self {
        let s = seed as f32;
        Self {
            hold_dx: (s - 4.0) * 16.0,
            close_gap: 48.0 + 6.0 * s,
            delay: seed as u32,
            swap_tick: SWAP_TICK + 5 * seed,
        }
    }
}

/// A steered hero's plan progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct PlanState {
    held: u32,
    /// The `held` count on which the release condition first held.
    cond_at: Option<u32>,
    released: bool,
}

/// The machine of a run (the slot's only enemy).
pub fn dozer_of(sim: &PartyFixtureSim) -> &Dozer {
    sim.combat
        .slot_enemies()
        .iter()
        .find_map(|e| e.as_any().and_then(|a| a.downcast_ref::<DozerEnemy>()))
        .expect("room 3 holds the machine")
        .dozer()
}

/// The machine's own state hash.
pub fn dozer_hash(dozer: &Dozer) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    dozer.mix_into(&mut |v| {
        h ^= v;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    });
    h
}

/// The party fixture of a mode.
fn party_fixture(run: &DozerRun, mode: Mode) -> &'static PartyFixture {
    let who = match mode {
        Mode::CarlSolo => MasteryWho::Companion,
        Mode::DonutSolo | Mode::TwoPlayer => MasteryWho::CarlSolo,
    };
    Box::leak(Box::new(PartyFixture {
        base: run.fixture,
        two_player: mode == Mode::TwoPlayer,
        tuning: tuning(who),
        script: &[],
        pilot: Pilot::ArenaDirect,
        donut: DonutSetup::unlocked(8),
    }))
}

/// Start one run: the room under the story loadout, the seed, and the machine
/// as the roster builds it, edited by `edit` (a fixture's own machine state).
pub fn start_with(
    root: &Path,
    mode: Mode,
    seed: u64,
    edit: impl FnOnce(&mut Dozer),
) -> Result<(PartyFixtureSim, DozerRun), String> {
    start_flags(root, mode, seed, Loadout::Story.flags(), edit)
}

/// [`start_with`] under explicit program flags.
pub fn start_flags(
    root: &Path,
    mode: Mode,
    seed: u64,
    flags: (CarlProgFlags, DonutProgFlags),
    edit: impl FnOnce(&mut Dozer),
) -> Result<(PartyFixtureSim, DozerRun), String> {
    let run = f01_dozer::launch(root)?;
    let mut scene = crate::corridor::SceneSim::new_at(&run.kit, 360, run.fixture.carl_x)?;
    let spawn = spawn_from_roster(
        &run.loaded.encounter,
        f01_dozer::slot_room(&run, &scene),
        false,
    )
    .map_err(|e| e.to_string())?;
    let enemy = spawn
        .slot
        .enemies()
        .first()
        .and_then(|e| e.as_any())
        .and_then(|a| a.downcast_ref::<DozerEnemy>())
        .ok_or("room 3's roster builds the machine")?
        .clone();
    let mut dozer = enemy.dozer().clone();
    edit(&mut dozer);
    let slot =
        SlotSet::new(vec![Box::new(enemy.with_dozer(dozer))], false).map_err(|e| e.to_string())?;
    let (carl, donut) = flags;
    let combat =
        f01_dozer::combat_seeded(run.fixture, &run, &mut scene, carl, donut, slot, Some(seed))?;
    let fixture = party_fixture(&run, mode);
    Ok((PartyFixtureSim::from_parts(scene, combat, fixture), run))
}

/// Start one run with the machine as the roster builds it.
pub fn start(root: &Path, mode: Mode, seed: u64) -> Result<(PartyFixtureSim, DozerRun), String> {
    start_with(root, mode, seed, |_| {})
}

/// The gap of `hero` alone (its body left minus the wheel front), once the
/// machine has emerged.
fn own_gap(sim: &PartyFixtureSim, hero: HeroId) -> Option<f32> {
    let d = dozer_of(sim);
    d.state
        .emerged()
        .then(|| sim.scene.world.body(hero).x - HERO_HALF_WIDTH - d.wheel_front())
}

/// The input of a steered hero under `plan` and the timing `vary`.
fn plan_input(
    plan: Plan,
    vary: Vary,
    sim: &PartyFixtureSim,
    hero: HeroId,
    state: &mut PlanState,
    exit_x: f32,
) -> RawInput {
    let x = sim.scene.world.body(hero).x;
    let run = RawInput {
        move_x: if x < exit_x { 1.0 } else { 0.0 },
        ..RawInput::default()
    };
    let hold_at = match plan {
        Plan::Run => return run,
        Plan::Stall | Plan::CloseCall => HOLD_X,
        Plan::Wait => WAIT_X,
    } + vary.hold_dx;
    if state.released || x < hold_at {
        return run;
    }
    state.held += 1;
    let cond = match plan {
        Plan::Stall => state.held > STALL_TICKS,
        _ => own_gap(sim, hero).is_some_and(|g| g <= vary.close_gap),
    };
    if cond && state.cond_at.is_none() {
        state.cond_at = Some(state.held);
    }
    state.released = state
        .cond_at
        .is_some_and(|at| state.held >= at + vary.delay);
    if state.released {
        run
    } else {
        RawInput::default()
    }
}

/// The report of one K5 run.
#[derive(Debug, Clone)]
pub struct K5Report {
    pub mode: Mode,
    pub plan: Plan,
    pub seed: u64,
    pub complete_tick: Option<u64>,
    pub hurts: usize,
    pub defeats: usize,
    pub wheel_hits: [u32; 2],
    /// Wheel hits on a hero while it was a bot.
    pub bot_wheel_hits: u32,
    pub max_speed: f32,
    pub max_cruise_target: f32,
    /// The highest speed in the cruise band once the machine has been in it
    /// long enough to shed the catch-up surplus ([`cruise_settle_ticks`]).
    pub max_settled_cruise_speed: f32,
    /// The longest run of ticks a bot's centre stayed behind the axle while
    /// the wheel was lethal.
    pub max_behind_axle: u32,
    pub max_stuck: [u32; 2],
    pub band_ticks: [u32; 3],
    pub rescues: [u32; 2],
    pub lane_rescues: u32,
    pub tunnel_rescues: u32,
    pub pinned_holds: u32,
    /// Off-screen regroups of the companion during the chase (placed or
    /// legacy; a denied regroup moves nobody).
    pub chase_regroups: u32,
    /// ... of which count as toward the machine (DZ-Q24): a placed regroup
    /// whose own x lies behind the axle, inside a Dozer row, or between the
    /// wheel and the leader, and every legacy regroup (section 7.5: it places
    /// behind the leader by construction).
    pub toward_regroups: u32,
    /// Off-screen regroups of a hero the machine had already rescued this run
    /// (the rescue-then-regroup cycles of section 7.3).
    pub rescue_regroup_cycles: u32,
    pub hazard_feed_changes: u32,
    /// Every hero alive and out of reach at completion.
    pub out_of_reach: bool,
    pub final_state: &'static str,
    pub hash: u64,
    pub ticks: u64,
}

/// The machine's reach at one instant, for the DZ-Q24 "toward the machine"
/// test of a regroup's placement.
#[derive(Debug, Clone, Copy)]
struct Reach {
    chasing: bool,
    axle_x: f32,
    front: f32,
    row_start: f32,
    row_end: f32,
}

impl Reach {
    fn of(d: &Dozer) -> Self {
        Self {
            chasing: d.state == DozerState::CommittedChase,
            axle_x: d.axle().0,
            front: d.wheel_front(),
            row_start: d.o + d.tuning.dzq02_push_box.x0 as f32,
            row_end: d.tips() + f32::from(d.tuning.dzq16_nav_lead_px),
        }
    }

    /// A placement at `x` lies behind the axle, inside a Dozer row, or
    /// between the wheel and the leader at `leader_x`.
    fn toward(&self, x: f32, leader_x: f32) -> bool {
        x < self.axle_x
            || (x >= self.row_start && x < self.row_end)
            || (x > self.front && x < leader_x)
    }
}

/// The ticks the machine needs, at its acceleration, to bring the catch-up
/// speed down to the cruise maximum of 170 (AD15: from then on, a speed in the
/// cruise band is at most 170).
pub fn cruise_settle_ticks() -> u32 {
    let p = crate::dozer::DozerTuning::active().revenge();
    let surplus = p.catch_up - f32::from(crate::dozer::REVENGE_CRUISE_MAX_PX_S);
    if surplus <= 0.0 {
        0
    } else {
        (surplus / p.accel).ceil() as u32
    }
}

/// Run one cell of the matrix.
pub fn run_k5(root: &Path, mode: Mode, plan: Plan, seed: u64) -> Result<K5Report, String> {
    let vary = Vary::of_seed(seed);
    let settle = cruise_settle_ticks();
    let (mut sim, run) = start(root, mode, seed)?;
    let exit = run.loaded.encounter.exit;
    let exit_x = (exit.x + exit.w / 2) as f32;
    let stop_x = run
        .loaded
        .encounter
        .dozer
        .map(|s| s.stop_x as f32)
        .ok_or("no dozer block")?;
    let mut plans = [PlanState::default(); 2];
    let mut swap_pressed = false;
    let mut last_x = [sim.scene.world.hero.x, sim.scene.world.donut_body.x];
    let mut stuck = [0u32; 2];
    let mut max_stuck = [0u32; 2];
    let mut behind = [0u32; 2];
    let mut max_behind_axle = 0u32;
    let mut max_speed = 0.0f32;
    let mut max_cruise_target = 0.0f32;
    let mut max_settled_cruise_speed = 0.0f32;
    let mut cruise_ticks = 0u32;
    let mut bot_wheel_hits = 0u32;
    let mut chase_regroups = 0u32;
    let mut toward_regroups = 0u32;
    let mut rescue_regroup_cycles = 0u32;
    let mut rescued = [false; 2];
    let mut complete_tick = None;
    let mut out_of_reach = false;
    let mut seen_events = 0usize;
    for _ in 0..RUN_TICKS {
        let tick = sim.scene.world.tick + 1;
        let controlled = sim.party.controlled_hero();
        let mut slots = [RawInput::NONE; 2];
        if mode == Mode::TwoPlayer {
            for id in HeroId::ALL {
                slots[id.index()] =
                    plan_input(plan, vary, &sim, id, &mut plans[id.index()], exit_x);
            }
        } else {
            slots[0] = plan_input(
                plan,
                vary,
                &sim,
                controlled,
                &mut plans[controlled.index()],
                exit_x,
            );
            if mode == Mode::DonutSolo
                && tick >= vary.swap_tick
                && !swap_pressed
                && controlled == HeroId::Carl
            {
                slots[0].swap = true;
                swap_pressed = true;
            }
        }
        let walking = [slots[0].move_x != 0.0, slots[1].move_x != 0.0];
        let bots_before = HeroId::ALL.map(|id| sim.scene.drive().is_bot(id));
        // The machine and the leaders as the companion's regroup (which runs
        // before the scene and the slot steps) saw them.
        let reach_before = Reach::of(dozer_of(&sim));
        let x_before = HeroId::ALL.map(|id| sim.scene.world.body(id).x);
        sim.step_slots(slots);
        let d = dozer_of(&sim);
        max_speed = max_speed.max(d.speed);
        if d.band == Some(Band::Cruise) {
            max_cruise_target = max_cruise_target.max(d.profile.cruise);
            cruise_ticks += 1;
            if cruise_ticks > settle {
                max_settled_cruise_speed = max_settled_cruise_speed.max(d.speed);
            }
        } else {
            cruise_ticks = 0;
        }
        let lethal = d.wheel_lethal();
        let (axle_x, _) = d.axle();
        let reach_after = Reach::of(d);
        for e in &sim.combat.events[seen_events..] {
            if let CombatEvent::Dozer {
                event: DozerEvent::WheelHit { hero },
                ..
            } = e
                && bots_before[hero.index()]
            {
                bot_wheel_hits += 1;
            }
        }
        for ev in sim.combat.take_companion_events() {
            let CompanionEvent::Regroup { hero, outcome, .. } = ev else {
                continue;
            };
            if !(reach_before.chasing || reach_after.chasing) {
                continue;
            }
            // Judged by the event's own placement, never by where the body
            // stands after the slot step (a same-tick rescue would mask it).
            let toward = match outcome {
                RegroupOutcome::Placed { x, .. } => {
                    let (x, leader_x) = (x as f32, x_before[hero.other().index()]);
                    reach_before.toward(x, leader_x) || reach_after.toward(x, leader_x)
                }
                RegroupOutcome::Legacy => true,
                RegroupOutcome::Denied(_) => continue,
            };
            chase_regroups += 1;
            toward_regroups += u32::from(toward);
            rescue_regroup_cycles += u32::from(rescued[hero.index()]);
        }
        for e in &sim.combat.events[seen_events..] {
            if let CombatEvent::Dozer {
                event: DozerEvent::AiRescue { hero, .. },
                ..
            } = e
            {
                rescued[hero.index()] = true;
            }
        }
        seen_events = sim.combat.events.len();
        for id in HeroId::ALL {
            let i = id.index();
            let body = sim.scene.world.body(id);
            let x = body.x;
            let bot = sim.scene.drive().is_bot(id);
            behind[i] = if bot && lethal && x < axle_x {
                behind[i] + 1
            } else {
                0
            };
            max_behind_axle = max_behind_axle.max(behind[i]);
            let still = (x - last_x[i]).abs() < 0.1;
            let steered = mode == Mode::TwoPlayer || id == controlled;
            let counts = if steered {
                let w = if mode == Mode::TwoPlayer {
                    walking[i]
                } else {
                    walking[0]
                };
                w && still
            } else {
                let leader = sim.scene.world.body(controlled).x;
                let leader_moved = (leader - last_x[controlled.index()]).abs() >= 0.1;
                still && (x - leader).abs() > APART_PX && leader_moved
            };
            stuck[i] = if counts { stuck[i] + 1 } else { 0 };
            max_stuck[i] = max_stuck[i].max(stuck[i]);
        }
        for id in HeroId::ALL {
            last_x[id.index()] = sim.scene.world.body(id).x;
        }
        if sim.scene.world.complete {
            complete_tick = Some(sim.scene.world.tick);
            let d = dozer_of(&sim);
            out_of_reach = HeroId::ALL.into_iter().all(|id| {
                let body = sim.scene.world.body(id);
                body.health > 0
                    && (!d.wheel_lethal()
                        || body.x - HERO_HALF_WIDTH >= stop_x
                        || body.x - HERO_HALF_WIDTH > d.wheel_front())
            });
            break;
        }
    }
    let count = |f: fn(&CombatEvent) -> bool| sim.combat.events.iter().filter(|e| f(e)).count();
    let hurts = count(|e| matches!(e, CombatEvent::HeroHurt { .. }));
    let defeats = count(|e| matches!(e, CombatEvent::HeroDefeated { .. }));
    let mut wheel_hits = [0u32; 2];
    for e in &sim.combat.events {
        if let CombatEvent::Dozer {
            event: DozerEvent::WheelHit { hero },
            ..
        } = e
        {
            wheel_hits[hero.index()] += 1;
        }
    }
    let d = dozer_of(&sim);
    let s = d.stats;
    Ok(K5Report {
        mode,
        plan,
        seed,
        complete_tick,
        hurts,
        defeats,
        wheel_hits,
        bot_wheel_hits,
        max_speed,
        max_cruise_target,
        max_settled_cruise_speed,
        max_behind_axle,
        max_stuck,
        band_ticks: s.band_ticks,
        rescues: s.rescues,
        lane_rescues: s.lane_rescues,
        tunnel_rescues: s.tunnel_rescues,
        pinned_holds: s.pinned_holds,
        chase_regroups,
        toward_regroups,
        rescue_regroup_cycles,
        hazard_feed_changes: sim.combat.companion_stats().hazard_feed_changes,
        out_of_reach,
        final_state: d.state.name(),
        hash: sim.state_hash(),
        ticks: sim.scene.world.tick,
    })
}

impl K5Report {
    pub fn name(&self) -> String {
        format!(
            "dz_k5_{}_{}_s{}",
            self.mode.name(),
            self.plan.name(),
            self.seed
        )
    }

    /// The hard asserts of one run (the band coverage is the matrix's).
    pub fn check(&self) -> Result<(), String> {
        let ceiling = f32::from(
            crate::dozer::DozerTuning::active()
                .revenge
                .dzq07_catch_up_px_s,
        );
        let fails = [
            (self.complete_tick.is_none(), "the room never completed"),
            (!self.out_of_reach, "a hero was dead or in reach at the end"),
            (self.hurts > 0 || self.defeats > 0, "a hero was touched"),
            (self.wheel_hits != [0, 0], "the wheel hit a hero"),
            (self.bot_wheel_hits > 0, "the wheel hit a bot"),
            (self.max_speed > ceiling.min(185.0), "the speed passed 185"),
            (
                self.max_cruise_target > f32::from(crate::dozer::REVENGE_CRUISE_MAX_PX_S),
                "the cruise target passed 170",
            ),
            (
                self.max_settled_cruise_speed > f32::from(crate::dozer::REVENGE_CRUISE_MAX_PX_S),
                "the speed in the cruise band passed 170 after the settle ticks",
            ),
            (self.max_behind_axle > 1, "a bot stayed behind the axle"),
            (
                self.max_stuck.iter().any(|s| *s > STUCK_LIMIT),
                "a hero was stuck",
            ),
            (self.pinned_holds > 0, "the machine held on a pinned hero"),
        ];
        let failed: Vec<&str> = fails.iter().filter(|(f, _)| *f).map(|(_, m)| *m).collect();
        if failed.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{}: {}: {}",
                self.name(),
                failed.join(", "),
                self.line()
            ))
        }
    }

    /// One evidence line.
    pub fn line(&self) -> String {
        format!(
            "DZ_K5 name={} complete={:?} hurts={} defeats={} wheel_hits={:?} bot_wheel_hits={} \
             max_speed={:.1} settled_cruise={:.1} behind_axle={} max_stuck={:?} \
             band_ticks[catch_up,cruise,mercy]={:?} rescues[carl,donut]={:?} lane={} tunnel={} \
             pinned={} chase_regroups={} toward_regroups={} rescue_regroup_cycles={} \
             feed_changes={} end={} hash={:016x}",
            self.name(),
            self.complete_tick,
            self.hurts,
            self.defeats,
            self.wheel_hits,
            self.bot_wheel_hits,
            self.max_speed,
            self.max_settled_cruise_speed,
            self.max_behind_axle,
            self.max_stuck,
            self.band_ticks,
            self.rescues,
            self.lane_rescues,
            self.tunnel_rescues,
            self.pinned_holds,
            self.chase_regroups,
            self.toward_regroups,
            self.rescue_regroup_cycles,
            self.hazard_feed_changes,
            self.final_state,
            self.hash
        )
    }
}

/// The CA2-DZ counts of the matrix (DZ-Q24, section 7.5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ca2DzCounts {
    /// Rescues of the autonomous Donut (`carl_solo`).
    pub autonomous_rescues: u32,
    /// Rescues of the assisted Carl (`donut_solo`) and that mode's runs.
    pub assisted_rescues: u32,
    pub assisted_runs: u32,
    /// Companion off-screen regroups toward the machine during the chase.
    pub toward_regroups: u32,
    /// Rescue-then-regroup cycles (section 7.3; reported, not a trigger).
    pub rescue_regroup_cycles: u32,
}

impl Ca2DzCounts {
    pub fn of(reports: &[K5Report]) -> Self {
        let assisted = || reports.iter().filter(|r| r.mode == Mode::DonutSolo);
        Self {
            autonomous_rescues: reports
                .iter()
                .filter(|r| r.mode == Mode::CarlSolo)
                .map(|r| r.rescues[HeroId::Donut.index()])
                .sum(),
            assisted_rescues: assisted().map(|r| r.rescues[HeroId::Carl.index()]).sum(),
            assisted_runs: assisted().count() as u32,
            toward_regroups: reports.iter().map(|r| r.toward_regroups).sum(),
            rescue_regroup_cycles: reports.iter().map(|r| r.rescue_regroup_cycles).sum(),
        }
    }

    /// The assisted Carl's mean rescues per run.
    pub fn assisted_per_run(&self) -> f32 {
        if self.assisted_runs == 0 {
            0.0
        } else {
            self.assisted_rescues as f32 / self.assisted_runs as f32
        }
    }
}

/// DZ-Q24: the CA2-DZ trigger over the matrix's reports. `Some(reason)` when it
/// fires.
pub fn ca2dz_verdict(reports: &[K5Report]) -> Option<String> {
    let t = crate::dozer::DozerTuning::active();
    let c = Ca2DzCounts::of(reports);
    let mut reasons = Vec::new();
    if c.autonomous_rescues > t.dzq24_ca2dz_autonomous_rescues_max {
        reasons.push(format!(
            "{} rescues of the autonomous Donut",
            c.autonomous_rescues
        ));
    }
    let per_run = c.assisted_per_run();
    if per_run > t.dzq24_ca2dz_assist_rescues_per_run_max {
        reasons.push(format!("{per_run:.2} rescues per run of the assisted Carl"));
    }
    if c.toward_regroups > t.dzq24_ca2dz_toward_regroups_max {
        reasons.push(format!("{} regroups toward the machine", c.toward_regroups));
    }
    (!reasons.is_empty()).then(|| reasons.join("; "))
}

/// The escape's end state of a machine (strain or idle).
pub fn escaped_at_rest(dozer: &Dozer) -> bool {
    matches!(
        dozer.state,
        DozerState::Escaped(EscapePhase::Strain { .. } | EscapePhase::Idle)
    )
}

// ------------------------------------------------------------ the scenarios --

/// A fixture's failure: its message.
macro_rules! ensure {
    ($cond:expr, $($arg:tt)*) => {
        let ok: bool = $cond;
        if !ok {
            return Err(format!($($arg)*));
        }
    };
}

/// The machine in the committed chase with its wheel front at `front`, at the
/// cruise speed.
pub fn chase_at(front: f32) -> impl FnOnce(&mut Dozer) {
    move |d| {
        d.o = front - d.tuning.wheel_front();
        d.state = DozerState::CommittedChase;
        d.band = Some(Band::Cruise);
        d.speed = d.profile.cruise;
    }
}

/// The machine at rest after the escape (idle, the wheel front at `stop_x`).
pub fn idle_at_rest() -> impl FnOnce(&mut Dozer) {
    move |d| {
        d.o = d.site.stop_x as f32 - d.tuning.wheel_front();
        d.state = DozerState::Escaped(EscapePhase::Idle);
        d.speed = 0.0;
    }
}

/// The program flags without Donut's jump boost (the `donut` jump case).
pub fn unboosted_flags() -> (CarlProgFlags, DonutProgFlags) {
    (Loadout::Story.flags().0, DonutProgFlags::default())
}

/// A staged scenario: the sim with the heroes put where the fixture wants
/// them, and the machine's sub-events so far.
pub struct Stage {
    pub sim: PartyFixtureSim,
    pub run: DozerRun,
    pub events: Vec<(u64, DozerEvent)>,
    seen: usize,
}

impl Stage {
    /// Build: `mode` (a `donut_solo` stage swaps at once), the flags, the
    /// machine edit, and the two heroes' columns (the camera centred on the
    /// steered hero).
    pub fn new(
        root: &Path,
        mode: Mode,
        flags: (CarlProgFlags, DonutProgFlags),
        edit: impl FnOnce(&mut Dozer),
        carl_x: f32,
        donut_x: f32,
    ) -> Result<Self, String> {
        let (mut sim, run) = start_flags(root, mode, 0, flags, edit)?;
        if mode == Mode::DonutSolo {
            sim.party.request_swap(true, false);
        }
        let lead_x = match sim.party.controlled_hero() {
            HeroId::Carl => carl_x,
            HeroId::Donut => donut_x,
        };
        sim.scene.respawn_at(lead_x);
        sim.scene.world.body_mut(HeroId::Carl).x = carl_x;
        sim.scene.world.body_mut(HeroId::Donut).x = donut_x;
        sim.scene.world.donut.x = donut_x;
        let seen = sim.combat.events.len();
        Ok(Self {
            sim,
            run,
            events: Vec::new(),
            seen,
        })
    }

    pub fn dozer(&self) -> &Dozer {
        dozer_of(&self.sim)
    }

    pub fn controlled(&self) -> HeroId {
        self.sim.party.controlled_hero()
    }

    pub fn bot(&self) -> HeroId {
        self.controlled().other()
    }

    /// The walking line (world y).
    pub fn floor(&self) -> f32 {
        self.dozer().floor_y
    }

    /// Move a body (its feet stay where they are).
    pub fn put(&mut self, hero: HeroId, x: f32) {
        self.sim.scene.world.body_mut(hero).x = x;
        if hero == HeroId::Donut {
            self.sim.scene.world.donut.x = x;
        }
    }

    /// One tick with the steered hero's input; returns this tick's machine
    /// sub-events.
    pub fn step(&mut self, raw: RawInput) -> Vec<DozerEvent> {
        self.step_slots([raw, RawInput::NONE])
    }

    pub fn step_slots(&mut self, slots: [RawInput; 2]) -> Vec<DozerEvent> {
        self.sim.step_slots(slots);
        let mut now = Vec::new();
        for e in &self.sim.combat.events[self.seen..] {
            if let CombatEvent::Dozer { tick, event } = e {
                self.events.push((*tick, *event));
                now.push(*event);
            }
        }
        self.seen = self.sim.combat.events.len();
        now
    }

    pub fn tick(&self) -> u64 {
        self.sim.scene.world.tick
    }

    /// The rescues so far.
    pub fn rescues(&self) -> Vec<(u64, DozerEvent)> {
        self.events
            .iter()
            .filter(|(_, e)| matches!(e, DozerEvent::AiRescue { .. }))
            .copied()
            .collect()
    }

    pub fn count(&self, f: impl Fn(&DozerEvent) -> bool) -> usize {
        self.events.iter().filter(|(_, e)| f(e)).count()
    }

    /// Hurts and defeats of the whole run.
    pub fn hurts(&self) -> usize {
        self.sim
            .combat
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    CombatEvent::HeroHurt { .. } | CombatEvent::HeroDefeated { .. }
                )
            })
            .count()
    }

    pub fn hash(&self) -> u64 {
        self.sim.state_hash()
    }
}

/// Fold fixture hashes into one.
pub fn fold(hashes: impl IntoIterator<Item = u64>) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for v in hashes {
        h ^= v;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// The steered hero of a solo mode stands still.
const IDLE: RawInput = RawInput::NONE;

/// The solo modes (a rescue needs a bot).
pub const SOLO: [Mode; 2] = [Mode::CarlSolo, Mode::DonutSolo];

/// `(hero, x, foot_y, spot, cause)` of a rescue event.
fn rescue_of(
    e: &DozerEvent,
) -> Option<(
    HeroId,
    i32,
    i32,
    crate::dozer::RescueSpot,
    crate::dozer::RescueCause,
)> {
    match *e {
        DozerEvent::AiRescue {
            hero,
            x,
            foot_y,
            spot,
            cause,
        } => Some((hero, x, foot_y, spot, cause)),
        _ => None,
    }
}

/// The body overlaps a solid.
fn in_solid(stage: &Stage, hero: HeroId) -> bool {
    let b = stage.sim.scene.world.body(hero);
    crate::combat::solid_at(
        &stage.sim.scene.world.solids,
        crate::dozer::body_rect(b.x, b.y - crate::prototype::HERO_HALF_HEIGHT),
    )
}

fn foot(stage: &Stage, hero: HeroId) -> f32 {
    stage.sim.scene.world.body(hero).y - crate::prototype::HERO_HALF_HEIGHT
}

/// `dz_rescue_leader_in_tunnel`: the leader waits in the tunnel, the bot is at
/// the wheel. One rescue, to the tunnel spot (never past the exit margin), and
/// the bot is never touched afterward.
pub fn dz_rescue_leader_in_tunnel(root: &Path) -> Result<u64, String> {
    let mut hashes = Vec::new();
    for mode in SOLO {
        let (lead_x, bot_x) = (1100.0, 722.0);
        let (carl_x, donut_x) = if mode == Mode::CarlSolo {
            (lead_x, bot_x)
        } else {
            (bot_x, lead_x)
        };
        let mut st = Stage::new(
            root,
            mode,
            Loadout::Story.flags(),
            chase_at(700.0),
            carl_x,
            donut_x,
        )?;
        let bot = st.bot();
        let stop_x = st.dozer().site.stop_x as f32;
        let exit_left = st.dozer().exit_left;
        let now = st.step(IDLE);
        let r = now.iter().find_map(rescue_of);
        let Some((hero, x, _, spot, _)) = r else {
            return Err(format!(
                "{}: no rescue on the first tick: {now:?}",
                mode.name()
            ));
        };
        ensure!(hero == bot, "{}: rescued {hero:?}", mode.name());
        ensure!(
            spot == crate::dozer::RescueSpot::Tunnel
                && x as f32 >= stop_x + 48.0
                && x as f32 <= exit_left - 16.0,
            "{}: the spot {x} {spot:?}",
            mode.name()
        );
        ensure!(
            !in_solid(&st, bot),
            "{}: the bot is in a solid",
            mode.name()
        );
        for _ in 0..240 {
            st.step(IDLE);
        }
        ensure!(
            st.rescues().len() == 1,
            "{}: rescues {:?}",
            mode.name(),
            st.rescues()
        );
        ensure!(
            st.hurts() == 0 && st.count(|e| matches!(e, DozerEvent::WheelHit { .. })) == 0,
            "{}: a hero was touched",
            mode.name()
        );
        hashes.push(st.hash());
    }
    Ok(fold(hashes))
}

/// The tick (counted from the stage) of the steered hero's jump apex: the
/// first tick its vertical speed is not positive after the takeoff.
fn apex_ticks(
    root: &Path,
    mode: Mode,
    flags: (CarlProgFlags, DonutProgFlags),
    lead_x: f32,
    bot_x: f32,
) -> Result<u32, String> {
    let (carl_x, donut_x) = if mode == Mode::CarlSolo {
        (lead_x, bot_x)
    } else {
        (bot_x, lead_x)
    };
    let mut st = Stage::new(root, mode, flags, chase_at(300.0), carl_x, donut_x)?;
    let lead = st.controlled();
    let jump = RawInput {
        jump: true,
        ..RawInput::default()
    };
    let mut rose = false;
    for k in 1..=120 {
        st.step(jump);
        let b = st.sim.scene.world.body(lead);
        if b.vy > 0.0 {
            rose = true;
        } else if rose && !b.grounded {
            return Ok(k);
        }
    }
    Err(format!("{}: no apex", mode.name()))
}

/// The jump cases: `(name, mode, flags)` (the steered hero jumps).
pub fn jump_cases() -> [(&'static str, Mode, (CarlProgFlags, DonutProgFlags)); 3] {
    [
        ("carl", Mode::CarlSolo, Loadout::Story.flags()),
        ("donut", Mode::DonutSolo, unboosted_flags()),
        ("donut_boosted", Mode::DonutSolo, Loadout::Story.flags()),
    ]
}

/// `dz_rescue_at_leader_apex`: the rescue fires on the tick the leader is at
/// its jump apex. The bot lands on the floor at its spot (never at the
/// leader's height), overlapping no solid.
pub fn dz_rescue_at_leader_apex(root: &Path) -> Result<u64, String> {
    let mut hashes = Vec::new();
    for (name, mode, flags) in jump_cases() {
        let (lead_x, bot_x) = (600.0, 760.0);
        let k = apex_ticks(root, mode, flags, lead_x, bot_x)?;
        let (carl_x, donut_x) = if mode == Mode::CarlSolo {
            (lead_x, bot_x)
        } else {
            (bot_x, lead_x)
        };
        let mut st = Stage::new(root, mode, flags, chase_at(300.0), carl_x, donut_x)?;
        let (lead, bot) = (st.controlled(), st.bot());
        let jump = RawInput {
            jump: true,
            ..RawInput::default()
        };
        for _ in 1..k {
            st.step(jump);
        }
        ensure!(st.rescues().is_empty(), "{name}: an early rescue");
        let front = st.dozer().wheel_front();
        st.put(bot, front + HERO_HALF_WIDTH + 6.0);
        let now = st.step(jump);
        let l = st.sim.scene.world.body(lead);
        ensure!(
            !l.grounded && l.vy <= 0.0,
            "{name}: the leader is not at its apex"
        );
        let lead_foot = foot(&st, lead);
        let Some((hero, x, foot_y, _, _)) = now.iter().find_map(rescue_of) else {
            return Err(format!("{name}: no rescue on the apex tick {k}: {now:?}"));
        };
        let floor = st.floor();
        ensure!(hero == bot, "{name}: rescued {hero:?}");
        ensure!(
            (foot_y as f32 - floor).abs() < 0.5 && (foot(&st, bot) - floor).abs() < 0.5,
            "{name}: the bot's feet {} (event {foot_y}) are not the floor {floor} (leader's feet {lead_foot})",
            foot(&st, bot)
        );
        ensure!(
            lead_foot > floor + 16.0,
            "{name}: the leader is not airborne ({lead_foot})"
        );
        ensure!(
            (st.sim.scene.world.body(bot).x - x as f32).abs() <= 4.0,
            "{name}: the bot is not at its spot"
        );
        ensure!(!in_solid(&st, bot), "{name}: the bot is in a solid");
        hashes.push(st.hash());
    }
    Ok(fold(hashes))
}

/// `dz_rescue_lane_spot`: the wheel at 300, the leader at 600, the bot at the
/// wheel: the lane spot, 48 px ahead of the leader, in view.
pub fn dz_rescue_lane_spot(root: &Path) -> Result<u64, String> {
    let mut hashes = Vec::new();
    for mode in SOLO {
        let (lead_x, bot_x) = (600.0, 330.0);
        let (carl_x, donut_x) = if mode == Mode::CarlSolo {
            (lead_x, bot_x)
        } else {
            (bot_x, lead_x)
        };
        let mut st = Stage::new(
            root,
            mode,
            Loadout::Story.flags(),
            chase_at(300.0),
            carl_x,
            donut_x,
        )?;
        let bot = st.bot();
        let now = st.step(IDLE);
        let Some((hero, x, _, spot, cause)) = now.iter().find_map(rescue_of) else {
            return Err(format!("{}: no rescue: {now:?}", mode.name()));
        };
        let view = st.sim.scene.view_x();
        ensure!(
            hero == bot
                && spot == crate::dozer::RescueSpot::Lane
                && cause == crate::dozer::RescueCause::Near,
            "{}: {hero:?} {spot:?} {cause:?}",
            mode.name()
        );
        ensure!(x == 648, "{}: the lane spot is {x}, not 648", mode.name());
        ensure!(
            (x as f32) >= view.0 + 24.0 && (x as f32) <= view.1 - 24.0,
            "{}: the lane spot {x} is not in the view {view:?}",
            mode.name()
        );
        ensure!(!in_solid(&st, bot), "{}: in a solid", mode.name());
        hashes.push(st.hash());
    }
    Ok(fold(hashes))
}

/// `dz_rescue_no_loop`: a second rescue within the spacing goes to the tunnel
/// (never back to the lane), and the run then holds at most two rescues per
/// 120 ticks with no hero touched while the leader makes a close call.
pub fn dz_rescue_no_loop(root: &Path) -> Result<u64, String> {
    let mut hashes = Vec::new();
    for mode in SOLO {
        let (lead_x, bot_x) = (600.0, 330.0);
        let (carl_x, donut_x) = if mode == Mode::CarlSolo {
            (lead_x, bot_x)
        } else {
            (bot_x, lead_x)
        };
        let mut st = Stage::new(
            root,
            mode,
            Loadout::Story.flags(),
            chase_at(300.0),
            carl_x,
            donut_x,
        )?;
        let (lead, bot) = (st.controlled(), st.bot());
        let exit = st.run.loaded.encounter.exit;
        let exit_x = (exit.x + exit.w / 2) as f32;
        st.step(IDLE);
        ensure!(st.rescues().len() == 1, "{}: no first rescue", mode.name());
        for _ in 0..9 {
            st.step(IDLE);
        }
        let front = st.dozer().wheel_front();
        st.put(bot, front + HERO_HALF_WIDTH + 6.0);
        let now = st.step(IDLE);
        let Some((_, x, _, spot, _)) = now.iter().find_map(rescue_of) else {
            return Err(format!("{}: no second rescue: {now:?}", mode.name()));
        };
        ensure!(
            spot == crate::dozer::RescueSpot::Tunnel && x >= 1048,
            "{}: the second rescue went to {spot:?} {x}",
            mode.name()
        );
        let mut plan = PlanState::default();
        for _ in 0..600 {
            if st.sim.scene.world.complete {
                break;
            }
            let raw = plan_input(
                Plan::CloseCall,
                Vary::BASE,
                &st.sim,
                lead,
                &mut plan,
                exit_x,
            );
            st.step(raw);
        }
        let ticks: Vec<u64> = st.rescues().iter().map(|(t, _)| *t).collect();
        for (i, t) in ticks.iter().enumerate() {
            let within = ticks[i..].iter().filter(|u| **u < t + 120).count();
            ensure!(
                within <= 2,
                "{}: {within} rescues within 120 ticks of {t}: {ticks:?}",
                mode.name()
            );
        }
        ensure!(st.hurts() == 0, "{}: a hero was touched", mode.name());
        ensure!(st.sim.scene.world.complete, "{}: no exit", mode.name());
        hashes.push(st.hash());
    }
    Ok(fold(hashes))
}

/// `dz_rescue_behind_axle`: a bot behind the axle (between the hull and the
/// axle, or inside the hull and pushed out behind it) is put ahead on the same
/// tick and never touched by the wheel.
pub fn dz_rescue_behind_axle(root: &Path) -> Result<u64, String> {
    let mut hashes = Vec::new();
    for mode in SOLO {
        for offset in [110.0, -40.0] {
            let front = 500.0;
            let o = front - crate::dozer::DozerTuning::active().wheel_front();
            let (lead_x, bot_x) = (800.0, o + offset);
            let (carl_x, donut_x) = if mode == Mode::CarlSolo {
                (lead_x, bot_x)
            } else {
                (bot_x, lead_x)
            };
            let mut st = Stage::new(
                root,
                mode,
                Loadout::Story.flags(),
                chase_at(front),
                carl_x,
                donut_x,
            )?;
            let bot = st.bot();
            let now = st.step(IDLE);
            let Some((hero, x, _, _, cause)) = now.iter().find_map(rescue_of) else {
                return Err(format!("{} {offset}: no rescue: {now:?}", mode.name()));
            };
            ensure!(
                hero == bot && cause == crate::dozer::RescueCause::BehindAxle,
                "{} {offset}: {hero:?} {cause:?}",
                mode.name()
            );
            ensure!(
                x as f32 > st.dozer().tips(),
                "{} {offset}: not ahead ({x})",
                mode.name()
            );
            for _ in 0..60 {
                st.step(IDLE);
            }
            ensure!(
                st.count(|e| matches!(e, DozerEvent::WheelHit { hero } if *hero == bot)) == 0,
                "{} {offset}: the wheel hit the bot",
                mode.name()
            );
            hashes.push(st.hash());
        }
    }
    Ok(fold(hashes))
}

/// `dz_escaped_wheel_off`: the steered hero waits with its left edge at
/// `stop_x`, the bot at the tunnel mouth. The brake ends with the wheel front
/// at `stop_x`; no hero is touched and no bot is rescued.
pub fn dz_escaped_wheel_off(root: &Path) -> Result<u64, String> {
    let mut hashes = Vec::new();
    for mode in SOLO {
        let stop_x = 1000.0;
        let (lead_x, bot_x) = (stop_x + HERO_HALF_WIDTH, 1052.0);
        let (carl_x, donut_x) = if mode == Mode::CarlSolo {
            (lead_x, bot_x)
        } else {
            (bot_x, lead_x)
        };
        let mut st = Stage::new(
            root,
            mode,
            Loadout::Story.flags(),
            chase_at(800.0),
            carl_x,
            donut_x,
        )?;
        let lead = st.controlled();
        for _ in 0..240 {
            st.step(IDLE);
            ensure!(
                st.sim.scene.world.body(lead).x - HERO_HALF_WIDTH >= stop_x - 0.01,
                "{}: the steered hero moved into reach",
                mode.name()
            );
        }
        let d = st.dozer();
        ensure!(
            d.state == DozerState::Escaped(EscapePhase::Idle),
            "{}: the machine is {:?}",
            mode.name(),
            d.state
        );
        ensure!(
            (d.wheel_front() - stop_x).abs() < 0.01,
            "{}: the wheel front rests at {}",
            mode.name(),
            d.wheel_front()
        );
        ensure!(
            st.hurts() == 0 && st.rescues().is_empty(),
            "{}: touched or rescued: {:?}",
            mode.name(),
            st.events
        );
        hashes.push(st.hash());
    }
    Ok(fold(hashes))
}

/// `dz_stop_face_safe`: the machine at rest; the steered hero walks back into
/// the parked wheel and leans on the hull: never hurt, never past the hull.
pub fn dz_stop_face_safe(root: &Path) -> Result<u64, String> {
    let mut hashes = Vec::new();
    for mode in SOLO {
        let (lead_x, bot_x) = (1012.0, 1100.0);
        let (carl_x, donut_x) = if mode == Mode::CarlSolo {
            (lead_x, bot_x)
        } else {
            (bot_x, lead_x)
        };
        let mut st = Stage::new(
            root,
            mode,
            Loadout::Story.flags(),
            idle_at_rest(),
            carl_x,
            donut_x,
        )?;
        let lead = st.controlled();
        let hull_right = st.dozer().push_box().right();
        let back = RawInput {
            move_x: -1.0,
            ..RawInput::default()
        };
        let mut leaned = false;
        for _ in 0..120 {
            st.step(back);
            let x = st.sim.scene.world.body(lead).x;
            ensure!(
                x - HERO_HALF_WIDTH >= hull_right - 0.5,
                "{}: the hero passed into the hull ({x})",
                mode.name()
            );
            leaned |= x - HERO_HALF_WIDTH < hull_right + 1.0;
        }
        ensure!(leaned, "{}: the hero never reached the hull", mode.name());
        ensure!(
            st.hurts() == 0
                && st.count(|e| matches!(e, DozerEvent::WheelHit { .. })) == 0
                && st.dozer().state == DozerState::Escaped(EscapePhase::Idle),
            "{}: touched: {:?}",
            mode.name(),
            st.events
        );
        hashes.push(st.hash());
    }
    Ok(fold(hashes))
}

/// `dz_visible_lead`: Carl stands still at every 16 px from 224 to 1000 (Donut
/// parked ahead wakes the machine); the wheel is in his view at least
/// `dzq23_min_visible_lead_ticks` before it touches him. Returns the fold, the
/// smallest lead and Carl's column there.
pub fn dz_visible_lead(root: &Path) -> Result<(u64, u64, i32), String> {
    let min_lead = u64::from(crate::dozer::DozerTuning::active().dzq23_min_visible_lead_ticks);
    let mut hashes = Vec::new();
    let (mut smallest, mut at) = (u64::MAX, 0);
    for x in (224..=1000).step_by(16) {
        let mut st = Stage::new(
            root,
            Mode::CarlSolo,
            Loadout::Story.flags(),
            |_| {},
            x as f32,
            1150.0,
        )?;
        st.sim.park_uncontrolled();
        let mut visible = None;
        let mut hit = None;
        for _ in 0..900 {
            let now = st.step(IDLE);
            let d = st.dozer();
            let (l, r) = st.sim.scene.view_x();
            let w = d.wheel_box();
            if visible.is_none() && d.state.emerged() && w.right() > l && w.left < r {
                visible = Some(st.tick());
            }
            if now
                .iter()
                .any(|e| matches!(e, DozerEvent::WheelHit { hero: HeroId::Carl }))
            {
                hit = Some(st.tick());
                break;
            }
        }
        let (Some(v), Some(h)) = (visible, hit) else {
            return Err(format!("x {x}: visible {visible:?} hit {hit:?}"));
        };
        ensure!(
            h - v >= min_lead,
            "x {x}: the wheel was in view {} ticks before the contact",
            h - v
        );
        if h - v < smallest {
            (smallest, at) = (h - v, x);
        }
        hashes.push(st.hash());
    }
    Ok((fold(hashes), smallest, at))
}

/// A press on one tick in `period`.
fn press_every(k: u32, period: u32) -> bool {
    k.is_multiple_of(period)
}

/// `dz_hull_clang`: Carl's jab on the resting hull and on the chasing hull's
/// rear clangs, changes nothing in the machine (a twin run without the jab
/// keeps the same machine hash at every tick) and never defeats it.
pub fn dz_hull_clang(root: &Path) -> Result<u64, String> {
    let mut hashes = Vec::new();
    // (machine, Carl's x, facing)
    let cases: [(bool, f32, i8); 2] = [(false, 940.0, -1), (true, 334.0, 1)];
    for (chasing, carl_x, facing) in cases {
        let mut twins = Vec::new();
        for attack in [true, false] {
            // The chasing case: the hull's rear right after the emergence,
            // still at rest (a jab reaches 10 px past Carl's body and the
            // cruising hull would outrun it).
            let edit = move |d: &mut Dozer| {
                if chasing {
                    chase_at(600.0)(d);
                    d.speed = 0.0;
                } else {
                    idle_at_rest()(d);
                }
            };
            let mut st = Stage::new(
                root,
                Mode::CarlSolo,
                Loadout::Story.flags(),
                edit,
                carl_x,
                1150.0,
            )?;
            st.sim.park_uncontrolled();
            st.sim.scene.world.body_mut(HeroId::Carl).facing = facing;
            let mut trace = Vec::new();
            for k in 0..40 {
                let raw = RawInput {
                    attack: attack && k < 12 && press_every(k, 6),
                    ..RawInput::default()
                };
                st.step(raw);
                trace.push(dozer_hash(st.dozer()));
            }
            let clangs = st.count(|e| matches!(e, DozerEvent::Clang { blast: false, .. }));
            let defeated = st
                .sim
                .combat
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::EnemyDefeated { .. }));
            ensure!(!defeated, "chasing={chasing}: an enemy defeat was logged");
            if attack {
                ensure!(clangs >= 1, "chasing={chasing}: no clang");
            }
            let expected = if chasing {
                DozerState::CommittedChase
            } else {
                DozerState::Escaped(EscapePhase::Idle)
            };
            ensure!(
                st.dozer().state == expected,
                "chasing={chasing}: the machine is {:?}",
                st.dozer().state
            );
            twins.push(trace);
            hashes.push(st.hash());
        }
        ensure!(
            twins[0] == twins[1],
            "chasing={chasing}: the jab changed the machine"
        );
    }
    Ok(fold(hashes))
}

/// `dz_wheel_strike`: Carl jabs into the arriving wheel (pressing once the
/// tips are 16 px from his body, so the jab's active box meets the collider):
/// it clangs, the chase goes on and the wheel hits him. A twin run without the
/// jab keeps the same machine hash at every tick before the hit (no hit-stop,
/// no speed change on the moving machine).
pub fn dz_wheel_strike(root: &Path) -> Result<u64, String> {
    let mut hashes = Vec::new();
    let mut twins = Vec::new();
    for attack in [true, false] {
        let mut st = Stage::new(
            root,
            Mode::CarlSolo,
            Loadout::Story.flags(),
            chase_at(400.0),
            560.0,
            1150.0,
        )?;
        st.sim.park_uncontrolled();
        st.sim.scene.world.body_mut(HeroId::Carl).facing = -1;
        let mut hit_tick = None;
        // The machine hash after every tick before the hit.
        let mut trace = Vec::new();
        for k in 0..240 {
            let gap = st.sim.scene.world.body(HeroId::Carl).x - HERO_HALF_WIDTH - st.dozer().tips();
            let raw = RawInput {
                attack: attack && gap <= 16.0 && press_every(k, 2),
                ..RawInput::default()
            };
            let now = st.step(raw);
            ensure!(
                matches!(
                    st.dozer().state,
                    DozerState::CommittedChase | DozerState::Escaped(_)
                ),
                "attack={attack}: the machine is {:?}",
                st.dozer().state
            );
            if now
                .iter()
                .any(|e| matches!(e, DozerEvent::WheelHit { hero: HeroId::Carl }))
            {
                hit_tick = Some(st.tick());
                break;
            }
            trace.push(dozer_hash(st.dozer()));
        }
        let Some(hit) = hit_tick else {
            return Err(format!("attack={attack}: the wheel never hit Carl"));
        };
        let clangs_before = st
            .events
            .iter()
            .filter(|(t, e)| *t < hit && matches!(e, DozerEvent::Clang { .. }))
            .count();
        if attack {
            ensure!(clangs_before >= 1, "no clang before the hit");
        } else {
            ensure!(clangs_before == 0, "a clang without a jab");
        }
        twins.push(trace);
        hashes.push(st.hash());
    }
    let n = twins[0].len().min(twins[1].len());
    ensure!(n > 0, "the wheel hit on the first tick");
    ensure!(
        twins[0][..n] == twins[1][..n],
        "the jab changed the moving machine"
    );
    Ok(fold(hashes))
}

/// `dz_blast_on_hull`: Carl's dynamite thrown at the resting machine clangs
/// (`blast`) and changes nothing.
pub fn dz_blast_on_hull(root: &Path) -> Result<u64, String> {
    let mut st = Stage::new(
        root,
        Mode::CarlSolo,
        Loadout::Story.flags(),
        idle_at_rest(),
        1060.0,
        1250.0,
    )?;
    st.sim.park_uncontrolled();
    st.sim.scene.world.body_mut(HeroId::Carl).facing = -1;
    let before = dozer_hash(st.dozer());
    for k in 0..240 {
        let raw = RawInput {
            ability: k == 1,
            ..RawInput::default()
        };
        st.step(raw);
    }
    let blasts = st.count(|e| matches!(e, DozerEvent::Clang { blast: true, .. }));
    ensure!(blasts >= 1, "no blast clang: {:?}", st.events);
    ensure!(
        dozer_hash(st.dozer()) == before,
        "the blast changed the machine"
    );
    ensure!(
        !st.sim
            .combat
            .events
            .iter()
            .any(|e| matches!(e, CombatEvent::EnemyDefeated { .. })),
        "an enemy defeat was logged"
    );
    Ok(st.hash())
}

/// `dz_no_jump_over_<case>`: the steered hero jumps toward the chasing
/// machine at every 4 px of gap from 0 to 260: each jump ends in a wheel hit,
/// the hero is never above the hull's top while over it and never behind it.
/// Returns the fold and the highest the hero's feet rose over the machine's
/// span (the hull's rear to the wheel's tips).
pub fn dz_no_jump_over(
    root: &Path,
    mode: Mode,
    flags: (CarlProgFlags, DonutProgFlags),
) -> Result<(u64, f32), String> {
    let mut hashes = Vec::new();
    let mut highest = 0.0f32;
    let front = 400.0;
    for gap in (0..=260).step_by(4) {
        let x = front + gap as f32 + HERO_HALF_WIDTH;
        let (carl_x, donut_x) = if mode == Mode::CarlSolo {
            (x, 1150.0)
        } else {
            (1150.0, x)
        };
        let mut st = Stage::new(root, mode, flags, chase_at(front), carl_x, donut_x)?;
        st.sim.park_uncontrolled();
        let hero = st.controlled();
        let raw = RawInput {
            move_x: -1.0,
            jump: true,
            ..RawInput::default()
        };
        let mut hit = false;
        for _ in 0..180 {
            let now = st.step(raw);
            let d = st.dozer();
            let hull = d.push_box();
            let b = st.sim.scene.world.body(hero);
            let feet = b.y - crate::prototype::HERO_HALF_HEIGHT;
            if b.x + HERO_HALF_WIDTH > hull.left && b.x - HERO_HALF_WIDTH < d.tips() {
                highest = highest.max(feet - d.floor_y);
            }
            if b.x + HERO_HALF_WIDTH > hull.left && b.x - HERO_HALF_WIDTH < hull.right() {
                ensure!(
                    feet < hull.top(),
                    "gap {gap}: the hero is above the hull ({feet})"
                );
            }
            ensure!(
                b.x >= hull.left,
                "gap {gap}: the hero got behind the machine ({})",
                b.x
            );
            if now
                .iter()
                .any(|e| matches!(e, DozerEvent::WheelHit { hero: h } if *h == hero))
            {
                hit = true;
                break;
            }
        }
        ensure!(hit, "gap {gap}: the jump got past the wheel");
        hashes.push(st.hash());
    }
    Ok((fold(hashes), highest))
}

/// `dz_retry_dormant`: a fixture restart in the middle of the chase, and the
/// party's defeat respawn after the wheel's hit, both bring the machine back
/// dormant at the gate with its pristine state.
pub fn dz_retry_dormant(root: &Path) -> Result<u64, String> {
    let (sim, _) = start(root, Mode::CarlSolo, 0)?;
    let pristine = dozer_hash(dozer_of(&sim));
    let mut hashes = Vec::new();
    // The restart.
    let mut st = Stage::new(
        root,
        Mode::CarlSolo,
        Loadout::Story.flags(),
        |_| {},
        192.0,
        160.0,
    )?;
    let run = RawInput {
        move_x: 1.0,
        ..RawInput::default()
    };
    for _ in 0..200 {
        st.step(run);
    }
    ensure!(
        st.dozer().state == DozerState::CommittedChase,
        "not chasing at 200: {:?}",
        st.dozer().state
    );
    st.sim.combat.restart(&mut st.sim.scene);
    st.sim.party.restart();
    ensure!(
        dozer_hash(st.dozer()) == pristine,
        "the restart left the machine {:?} at {}",
        st.dozer().state,
        st.dozer().o
    );
    hashes.push(st.hash());
    // The defeat respawn: Carl waits at 400 and the wheel hits him.
    let mut st = Stage::new(
        root,
        Mode::CarlSolo,
        Loadout::Story.flags(),
        |_| {},
        400.0,
        160.0,
    )?;
    st.sim.park_uncontrolled();
    let deaths = st.sim.combat.deaths;
    let mut dormant_again = false;
    for _ in 0..900 {
        st.step(IDLE);
        if st.sim.combat.deaths > deaths && matches!(st.dozer().state, DozerState::Dormant { .. }) {
            let d = st.dozer();
            ensure!(
                d.stats == crate::dozer::DozerStats::default()
                    && (d.wheel_front() - d.site.emerge_front_x as f32).abs() < 0.01,
                "the respawn left the machine at {} with {:?}",
                d.wheel_front(),
                d.stats
            );
            dormant_again = true;
            break;
        }
    }
    ensure!(dormant_again, "no defeat respawn to a dormant machine");
    hashes.push(st.hash());
    Ok(fold(hashes))
}

/// The machine's hash and the heroes' columns of each tick of a `run` plan.
fn chase_trace(st: &mut Stage, ticks: u32, exit_x: f32) -> Vec<(u64, u32, u32)> {
    let mut plan = PlanState::default();
    let lead = st.controlled();
    let mut out = Vec::new();
    for _ in 0..ticks {
        let raw = plan_input(Plan::Run, Vary::BASE, &st.sim, lead, &mut plan, exit_x);
        st.step(raw);
        out.push((
            dozer_hash(st.dozer()),
            st.sim.scene.world.hero.x.to_bits(),
            st.sim.scene.world.donut_body.x.to_bits(),
        ));
    }
    out
}

/// `dz_retry_chase`: after a restart in the middle of the chase, the next
/// attempt's machine (and the heroes) replay a fresh attempt tick for tick.
pub fn dz_retry_chase(root: &Path) -> Result<u64, String> {
    let ticks = 400;
    let mut fresh = Stage::new(
        root,
        Mode::TwoPlayer,
        Loadout::Story.flags(),
        |_| {},
        192.0,
        160.0,
    )?;
    let exit = fresh.run.loaded.encounter.exit;
    let exit_x = (exit.x + exit.w / 2) as f32;
    let a = chase_trace(&mut fresh, ticks, exit_x);
    let mut retried = Stage::new(
        root,
        Mode::TwoPlayer,
        Loadout::Story.flags(),
        |_| {},
        192.0,
        160.0,
    )?;
    let _ = chase_trace(&mut retried, 200, exit_x);
    retried.sim.combat.restart(&mut retried.sim.scene);
    retried.sim.party.restart();
    let b = chase_trace(&mut retried, ticks, exit_x);
    if let Some(i) = a.iter().zip(&b).position(|(x, y)| x != y) {
        return Err(format!(
            "the retry differs from a fresh attempt at tick {}",
            i + 1
        ));
    }
    Ok(fold([fresh.hash(), retried.hash()]))
}

#[cfg(test)]
mod tests;
