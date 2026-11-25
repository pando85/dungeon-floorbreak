//! CORRAL-WIRE: scripted route fixtures of room 0, the Infernal Corral (plan
//! section 14; named `corral_*`).
//!
//! Bevy-free and library-only (no committed replay file; the scene's headless
//! smoke and the PLAYTRACE tests cover the launched room). Each run is one
//! launch of the room built the way the scene builds it
//! ([`crate::f01_late::corral_start`], the story loadout: room 0 holds the
//! Guild biscuit only), driven by a **move-right, jump, attack** pilot: the
//! shared room pilot of the late slice
//! ([`crate::f01_mastery_fixtures::pilot_with`], base kit only, no ability; it
//! fights goblins and llamas) plus the walk to the exit once the roster is
//! clear. The room's raised surfaces (the entry shelf, the bumper, the dock)
//! are full-height steps the pilot jumps onto when it is blocked.
//!
//! These fixtures prove the **wiring**: the room launches, M0 to M3 is
//! traversable, the M3 gate holds until both M2 llamas are down and opens on
//! the second defeat, and room 0 grants nothing. They are not the CORRAL-C
//! difficulty gate (that is `corral_auto_*`, unchanged) and not game feel.
//!
//! | who | what plays |
//! | --- | --- |
//! | [`Who::CarlSolo`] | solo Carl (the live party's `cf02_assist` partner follows) |
//! | [`Who::DonutSolo`] | the swap on [`SWAP_TICK`], then Donut (solo) plays the room |
//! | [`Who::TwoPlayer`] | two slots, each hero driven by its own pilot |

use std::path::Path;

use crate::boss_scene::Pilot;
use crate::combat::CombatEvent;
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::f01_corral::CorralRun;
use crate::f01_late::{Loadout, corral_start};
pub use crate::f01_mastery_fixtures::STUCK_LIMIT;
use crate::f01_mastery_fixtures::{pilot_with, tuning as mastery_tuning};
use crate::input::RawInput;
use crate::party::{HeroId, PartyTuning};
use crate::party_scene::{PartyFixture, PartyFixtureSim};
use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Rect};

/// The run length (100 s of game time).
pub const RUN_TICKS: u64 = 6000;
/// The tick on which [`Who::DonutSolo`] swaps to Donut.
pub const SWAP_TICK: u64 = 30;

/// Who plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Who {
    CarlSolo,
    DonutSolo,
    TwoPlayer,
}

impl Who {
    pub const ALL: [Who; 3] = [Who::CarlSolo, Who::DonutSolo, Who::TwoPlayer];

    pub const fn name(self) -> &'static str {
        match self {
            Self::CarlSolo => "carl_solo",
            Self::DonutSolo => "donut_solo",
            Self::TwoPlayer => "two_player",
        }
    }

    fn tuning(self) -> PartyTuning {
        mastery_tuning(crate::f01_mastery_fixtures::Who::CarlSolo)
    }
}

/// The route of a run: the exit's centre column, and the goal column every
/// hero walks to after the clear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Route {
    pub exit_x: f32,
    /// The M3 threshold's right edge (x 2304, the dock's end and the face of
    /// the room's end wall): sheet section 15 requires **both** heroes to reach
    /// it (`corral_route_m0_m3`). A hero reaches it when its box does (body
    /// centre at x 2292, see [`at_goal`]).
    pub goal_x: f32,
}

impl Route {
    pub fn of(run: &CorralRun) -> Self {
        let exit = run.loaded.encounter.exit;
        Self {
            exit_x: (exit.x + exit.w / 2) as f32,
            goal_x: crate::corral_fixtures::room::DOCK.1 as f32,
        }
    }
}

/// Whether a solid stands within 14 px ahead of a body at ledge height.
fn blocked_ahead(scene: &SceneSim, x: f32, y: f32, dir: f32) -> bool {
    let probe = Rect {
        left: x - HERO_HALF_WIDTH + dir * 14.0,
        bottom: y - HERO_HALF_HEIGHT + 4.0,
        width: HERO_HALF_WIDTH * 2.0,
        height: 8.0,
    };
    crate::combat::solid_at(&scene.world.solids, probe)
}

/// Whether a body centred at `x` has reached the route's goal column: its box's
/// right edge is on it (the end wall's face, so the centre stands at x 2292).
pub fn at_goal(route: &Route, x: f32) -> bool {
    x + HERO_HALF_WIDTH >= route.goal_x - 0.5
}

/// Whether any roster enemy lives.
pub fn enemies_left(combat: &CombatState) -> bool {
    combat.enemies.iter().any(|g| g.alive()) || combat.llamas.iter().any(|l| l.alive())
}

/// The pilot of hero `hero`: the shared late-slice pilot while an enemy lives,
/// then the walk to the exit (jumping a step it is blocked by). A pure function
/// of the state; it never presses `ability`.
pub fn pilot(route: &Route, combat: &CombatState, scene: &SceneSim, hero: HeroId) -> RawInput {
    let body = scene.world.body(hero);
    let mut raw = if enemies_left(combat) {
        pilot_with(combat, scene, hero, false)
    } else {
        let mut raw = RawInput::default();
        // Walk on until the box meets the goal column (never pace back), and
        // never jump at the end wall that stands on it.
        let gap = route.goal_x - (body.x + HERO_HALF_WIDTH);
        if !at_goal(route, body.x) {
            raw.move_x = 1.0;
            if body.grounded && gap > 14.0 && blocked_ahead(scene, body.x, body.y, raw.move_x) {
                raw.jump = true;
            }
        }
        raw
    };
    raw.ability = false;
    raw
}

/// The report of one run.
#[derive(Debug, Clone)]
pub struct Report {
    pub who: Who,
    pub seed: u64,
    /// The tick the last roster enemy was defeated on.
    pub cleared_tick: Option<u64>,
    /// The tick a hero first reached the exit's column after the clear.
    pub exit_tick: Option<u64>,
    /// The tick each hero (Carl, Donut) first stood at or past the goal column
    /// (x 2304) after the clear; only [`run_through`] records it (and ends once
    /// both have).
    pub goal_ticks: [Option<u64>; 2],
    /// The first tick on which the M3 gate stood open, and the tick of the
    /// second M2 llama defeat.
    pub gate_open_tick: Option<u64>,
    pub second_m2_defeat_tick: Option<u64>,
    /// Whether a hero ever stood right of the gate while it was closed.
    pub passed_closed_gate: bool,
    /// The longest stuck run per hero (told to walk, column unchanged).
    pub max_stuck: [u32; 2],
    pub deaths: u32,
    pub capability_grants: usize,
    pub hash: u64,
    pub events: Vec<CombatEvent>,
}

/// Start one run for `who` at seed `seed` (the story loadout).
pub fn start(root: &Path, who: Who, seed: u64) -> Result<(PartyFixtureSim, CorralRun), String> {
    start_with(root, who, seed, false)
}

/// [`start`], or with `through` the same room whose exit trigger is disarmed
/// (the kit's exit rect dropped before the scene is built): the room never
/// completes, so both bodies can walk on past the exit to the M3 threshold's
/// right edge. Nothing else of the room changes (the gate, the roster, the
/// solids and the seed are the scene's).
fn start_with(
    root: &Path,
    who: Who,
    seed: u64,
    through: bool,
) -> Result<(PartyFixtureSim, CorralRun), String> {
    let (carl, donut) = Loadout::Story.flags();
    let (run, scene, combat) = if through {
        let run = crate::f01_corral::launch(root)?;
        let mut kit = run.kit.clone();
        kit.exit = None;
        let mut scene = SceneSim::new_at(&kit, 360, run.fixture.carl_x)?;
        let combat = crate::f01_corral::combat_for(&run, &mut scene, carl, donut, Some(seed))?;
        (run, scene, combat)
    } else {
        corral_start(root, 360, carl, donut, Some(seed))?
    };
    let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
        base: run.fixture,
        two_player: who == Who::TwoPlayer,
        tuning: who.tuning(),
        script: &[],
        pilot: Pilot::ArenaDirect,
        donut: DonutSetup::unlocked(8),
    }));
    Ok((PartyFixtureSim::from_parts(scene, combat, fixture), run))
}

/// Run one pilot for [`RUN_TICKS`] (or until a hero reaches the exit and the
/// room completes).
pub fn run(root: &Path, who: Who, seed: u64) -> Result<Report, String> {
    run_with(root, who, seed, false)
}

/// [`run`] with the exit trigger disarmed ([`start_with`]): the run goes on
/// until **both** heroes have reached the goal column, x 2304 (sheet section
/// 15, `corral_route_m0_m3`; [`at_goal`]), or [`RUN_TICKS`] run out. In the
/// launched room the first hero to touch the exit rect (x 2232..2296)
/// completes it and freezes both bodies, so x 2304 is only reachable here.
/// Solo, the played hero walks to the goal first, then the player swaps once
/// (the follower trails at its follow gap) and walks the partner on.
pub fn run_through(root: &Path, who: Who, seed: u64) -> Result<Report, String> {
    run_with(root, who, seed, true)
}

fn run_with(root: &Path, who: Who, seed: u64, through: bool) -> Result<Report, String> {
    let (mut sim, corral) = start_with(root, who, seed, through)?;
    let route = Route::of(&corral);
    let gate = corral.m3_gate.clone();
    let mut last_x = [sim.scene.world.hero.x, sim.scene.world.donut_body.x];
    let mut stuck = [0u32; 2];
    let mut max_stuck = [0u32; 2];
    let mut cleared_tick = None;
    let mut exit_tick = None;
    let mut goal_ticks = [None; 2];
    let mut gate_open_tick = None;
    let mut second_m2_defeat_tick = None;
    let mut passed_closed_gate = false;
    let deaths_at_start = sim.combat.deaths;
    let mut swap_pressed = false;
    let mut handover_pressed = false;
    for _ in 0..RUN_TICKS {
        let tick = sim.scene.world.tick + 1;
        let controlled = sim.party.controlled_hero();
        let mut slots = [
            pilot(&route, &sim.combat, &sim.scene, controlled),
            RawInput::NONE,
        ];
        if who == Who::TwoPlayer {
            slots = [
                pilot(&route, &sim.combat, &sim.scene, HeroId::Carl),
                pilot(&route, &sim.combat, &sim.scene, HeroId::Donut),
            ];
        }
        if who == Who::DonutSolo && tick >= SWAP_TICK && !swap_pressed && controlled == HeroId::Carl
        {
            slots[0].swap = true;
            swap_pressed = true;
        }
        // The walk through, solo: once the played hero stands at the goal and
        // its partner does not, the player swaps once and walks the partner on.
        if through
            && who != Who::TwoPlayer
            && !handover_pressed
            && cleared_tick.is_some()
            && goal_ticks[controlled.index()].is_some()
            && goal_ticks.iter().any(Option::is_none)
        {
            slots[0] = RawInput::NONE;
            slots[0].swap = true;
            handover_pressed = true;
        }
        let walking = [slots[0].move_x != 0.0, slots[1].move_x != 0.0];
        sim.step_slots(slots);
        for (i, id) in HeroId::ALL.into_iter().enumerate() {
            let x = sim.scene.world.body(id).x;
            let told = match who {
                Who::TwoPlayer => walking[i],
                _ => id == controlled && walking[0],
            };
            stuck[i] = if told && (x - last_x[i]).abs() < 0.1 {
                stuck[i] + 1
            } else {
                0
            };
            max_stuck[i] = max_stuck[i].max(stuck[i]);
            last_x[i] = x;
            if let Some((rect, _)) = &gate
                && !sim.combat.exit_gate_open()
                && x > rect.right() as f32
            {
                passed_closed_gate = true;
            }
        }
        if second_m2_defeat_tick.is_none()
            && let Some((_, llamas)) = &gate
            && llamas.iter().all(|&i| !sim.combat.llamas[i].alive())
        {
            second_m2_defeat_tick = Some(sim.scene.world.tick);
        }
        if gate_open_tick.is_none() && sim.combat.exit_gate_open() {
            gate_open_tick = Some(sim.scene.world.tick);
        }
        if cleared_tick.is_none() && !enemies_left(&sim.combat) {
            cleared_tick = Some(sim.scene.world.tick);
        }
        if cleared_tick.is_some()
            && exit_tick.is_none()
            && (sim.scene.world.complete
                || HeroId::ALL
                    .into_iter()
                    .any(|id| sim.scene.world.body(id).x >= route.exit_x))
        {
            exit_tick = Some(sim.scene.world.tick);
            if !through {
                break;
            }
        }
        if through && cleared_tick.is_some() {
            for (i, id) in HeroId::ALL.into_iter().enumerate() {
                if goal_ticks[i].is_none() && at_goal(&route, sim.scene.world.body(id).x) {
                    goal_ticks[i] = Some(sim.scene.world.tick);
                }
            }
            if goal_ticks.iter().all(Option::is_some) {
                break;
            }
        }
    }
    let capability_grants = sim
        .combat
        .events
        .iter()
        .filter(|e| matches!(e, CombatEvent::CapabilityGranted { .. }))
        .count();
    Ok(Report {
        who,
        seed,
        cleared_tick,
        exit_tick,
        goal_ticks,
        gate_open_tick,
        second_m2_defeat_tick,
        passed_closed_gate,
        max_stuck,
        deaths: sim.combat.deaths - deaths_at_start,
        capability_grants,
        hash: sim.combat.state_hash(&sim.scene),
        events: sim.combat.events.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// The seeds the route covers (`--cf02-seed`: the goblins' temperaments).
    const SEEDS: std::ops::Range<u64> = 0..3;

    /// `corral_route_m0_m3`: solo Carl, solo Donut and two players each walk
    /// M0 to M3 and reach the exit, never stuck past [`STUCK_LIMIT`], never
    /// right of the closed M3 gate; both M2 llamas wake and attack; and with
    /// the exit disarmed both heroes walk on to x 2304 (sheet section 15).
    #[test]
    fn corral_route_m0_m3_every_pilot_reaches_the_exit() {
        for who in Who::ALL {
            for seed in SEEDS {
                let r = run(&root(), who, seed).unwrap();
                let tag = format!("{} seed {seed}", who.name());
                assert!(r.cleared_tick.is_some(), "{tag}: the roster is cleared");
                assert!(r.exit_tick.is_some(), "{tag}: a hero reaches the exit");
                // Sheet section 15: both heroes reach x 2304 (the companion
                // trailing past the gate is the case a gate leak would hide
                // in). The launched room completes on the first hero's touch of
                // the exit rect, so this half runs with the exit disarmed.
                let t = run_through(&root(), who, seed).unwrap();
                assert!(
                    t.goal_ticks.iter().all(Option::is_some),
                    "{tag}: both heroes reach x 2304: {:?}",
                    t.goal_ticks
                );
                assert!(
                    !t.passed_closed_gate,
                    "{tag}: nobody passes the closed gate on the walk through"
                );
                assert!(
                    t.max_stuck.iter().all(|&s| s <= STUCK_LIMIT),
                    "{tag}: stuck on the walk through {:?}",
                    t.max_stuck
                );
                assert_eq!(t.cleared_tick, r.cleared_tick, "{tag}: same fight");
                assert!(
                    !r.passed_closed_gate,
                    "{tag}: nobody passes the closed gate"
                );
                assert!(
                    r.max_stuck.iter().all(|&s| s <= STUCK_LIMIT),
                    "{tag}: stuck {:?}",
                    r.max_stuck
                );
                // Both M2 llamas wake (CRL-Q24) and attack. The pilots close in
                // fast and the M2 llamas mostly spit at the stand-off before
                // they die, so a rush is not asserted here; the wiring of the
                // rush is pinned by `a_parked_carl_inside_b_trigger_is_rushed_by_b`.
                for m2 in [1, 2] {
                    let engaged = r.events.iter().any(|e| match *e {
                        CombatEvent::LlamaTell { llama, .. }
                        | CombatEvent::LlamaRushTell { llama, .. } => llama == m2,
                        _ => false,
                    });
                    assert!(engaged, "{tag}: M2 llama {m2} wakes and attacks");
                }
            }
        }
    }

    /// `corral_room0_no_grant`: room 0 grants nothing (KC9, KC11) and Donut
    /// enters with the Guild biscuit read through (mana full).
    #[test]
    fn corral_room0_no_grant() {
        for who in Who::ALL {
            let (sim, _) = start(&root(), who, 0).unwrap();
            assert_eq!(sim.combat.donut_mana_view(), Some((8, 8)), "the biscuit");
            let r = run(&root(), who, 0).unwrap();
            assert_eq!(r.capability_grants, 0, "{}", who.name());
        }
    }

    /// `corral_exit_gate_opens`: the M3 gate is closed until the second M2
    /// llama is defeated and open from that tick on.
    #[test]
    fn corral_exit_gate_opens_on_the_second_m2_defeat_tick() {
        for who in Who::ALL {
            let r = run(&root(), who, 0).unwrap();
            assert!(r.second_m2_defeat_tick.is_some(), "{}", who.name());
            assert_eq!(r.gate_open_tick, r.second_m2_defeat_tick, "{}", who.name());
        }
    }

    #[test]
    fn corral_route_is_deterministic() {
        let a = run(&root(), Who::CarlSolo, 1).unwrap();
        let b = run(&root(), Who::CarlSolo, 1).unwrap();
        assert_eq!((a.hash, a.exit_tick), (b.hash, b.exit_tick));
    }
}
