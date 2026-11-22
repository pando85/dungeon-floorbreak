//! EVID-ROOM: scripted **room pilots**, the input source of the deterministic
//! room-scene capture path (`floor01_scene --scene f01_late --f01-room ROOM
//! --room-pilot PLAN[:2p]`, see `docs/evidence/f01-boiler/README.md`).
//!
//! A pilot is a pure function of the finished simulation state (the hero and
//! the enemies, the env props, the combat log) and the tick: no clock, no RNG,
//! no wall time, no camera, nothing the presentation owns. The same pilot on the
//! same room therefore replays to the same state hash in the headless smoke and
//! in a rendered capture, which is how the evidence shows that the presentation
//! is read-only. A pilot is **not** a player and not an evaluator of fun: it
//! plays a fixed plan so the room can be filmed.
//!
//! Bevy-free. The plans reuse the CF02 pilots (`cf02::arena::autopilot` fights
//! the nearest awake goblin; `donut_pilot` is P2 as a human would play the room)
//! and the Boiler's prop reach columns (`boiler_fixtures::scene_pilots`).

use crate::boiler_fixtures::scene_pilots::{BRACE_X, VALVE_X};
use crate::boss_scene::Pilot;
use crate::cf02::arena::{self, Plan as FightPlan};
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::env_prop::{MassState, ValveState};
use crate::goblin::GoblinMode;
use crate::input::RawInput;
use crate::party::HeroId;
use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Rect};

/// Ticks between the two distinct jabs a prop needs (the scene pilots use 1 and 31).
const JAB_PERIOD: u64 = 30;
/// A goblin this close (px) is fought before anything else.
const FIGHT_RADIUS: f32 = 70.0;
/// Where the chain pilot waits for the goblin it will kick: left of the valve,
/// so the launched body flies right into the valve's box.
const KICK_STAND_X: f32 = 380.0;
/// Where the chain pilot walks to wake the goblins that are still asleep, and
/// how close (px) a goblin must be to count as following him.
const LURE_X: f32 = 560.0;
const LURE_RANGE: f32 = 200.0;
/// Carl backs off while the follower is nearer than this (px).
const KITE_GAP: f32 = 110.0;
/// A kick reaches a goblin whose centre is this far ahead (px).
const KICK_REACH: f32 = 36.0;
/// A launched goblin must start within this many px left of the valve column to
/// reach its box (the BOIL-D launch test kicks it from 30 px away).
const KICK_MAX_GAP: f32 = 60.0;
/// The brace (prop 1) of the Boiler and its trigger weight.
const BRACE_PROP: u16 = 1;

/// What a room pilot does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plan {
    /// Fight everything straight through and walk to the exit.
    Direct,
    /// Fight on the way; when the valve is untouched and nobody is close, stand
    /// at the valve and jab it twice, then wait out the steam.
    Valve,
    /// The Boiler's chain: jab the brace once, walk back left of the valve and
    /// kick the goblin that follows into the valve (steam, brace, landing, drum).
    Chain,
    /// PT-GATE-A: the closing arena (`--f01-room mastery`): the PT-MASTERY pilot
    /// ([`crate::f01_mastery_fixtures::pilot`]) fights the roster with the base
    /// kit and walks to the exit. Only valid in the mastery room.
    Mastery,
    /// PT-GATE-A: the Juicer's room (`--f01-room juicer`): the PT-SCENE-B solo-Carl
    /// pilot ([`crate::boss_scene::pilot_inputs`] with [`Pilot::Carl`], jab phase 0)
    /// fights the boss and walks out. Only valid in the Juicer's room.
    Juicer,
}

impl Plan {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Valve => "valve",
            Self::Chain => "chain",
            Self::Mastery => "mastery",
            Self::Juicer => "juicer",
        }
    }
}

/// A parsed `--room-pilot` value: `PLAN` or `PLAN:2p`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomPilot {
    pub plan: Plan,
    /// P2 plays Donut with the CF02 pilot (two local players); otherwise the
    /// follower bot drives her, as in every scripted solo run.
    pub two_player: bool,
}

impl RoomPilot {
    pub fn parse(value: &str) -> Result<Self, String> {
        let (plan, rest) = value.split_once(':').map_or((value, ""), |(a, b)| (a, b));
        let plan = match plan {
            "direct" => Plan::Direct,
            "valve" => Plan::Valve,
            "chain" => Plan::Chain,
            "mastery" => Plan::Mastery,
            "juicer" => Plan::Juicer,
            other => {
                return Err(format!(
                    "unknown --room-pilot plan {other:?} (expected direct, valve, chain, mastery or juicer, optionally :2p)"
                ));
            }
        };
        let two_player = match rest {
            "" => false,
            "2p" => true,
            other => {
                return Err(format!(
                    "bad --room-pilot suffix {other:?} (the only one is :2p)"
                ));
            }
        };
        Ok(Self { plan, two_player })
    }

    /// The value that parses back to this pilot.
    pub fn label(self) -> String {
        if self.two_player {
            format!("{}:2p", self.plan.name())
        } else {
            self.plan.name().to_owned()
        }
    }

    /// The two slot inputs of the next tick. `exit_x` is the column Carl walks
    /// to once no enemy is left (the room's protected exit).
    pub fn inputs(self, combat: &CombatState, sim: &SceneSim, exit_x: f32) -> [RawInput; 2] {
        let carl = match self.plan {
            Plan::Direct => fight_or_walk(combat, sim, exit_x),
            Plan::Valve => valve(combat, sim, exit_x),
            Plan::Chain => chain(combat, sim, exit_x),
            Plan::Mastery => crate::f01_mastery_fixtures::pilot(combat, sim, HeroId::Carl),
            Plan::Juicer => crate::boss_scene::pilot_inputs(
                Pilot::Carl,
                [RawInput::NONE; 2],
                sim.world.tick + 1,
                combat,
                sim,
            )[0],
        };
        let donut = if self.two_player {
            arena::donut_pilot(combat, sim)
        } else {
            RawInput::NONE
        };
        [carl, donut]
    }
}

fn alive_any(combat: &CombatState) -> bool {
    combat.enemies.iter().any(|g| g.alive()) || combat.llamas.iter().any(|l| l.alive())
}

/// A solid that rises above the feet a step ahead (the same probe as the CF02
/// pilot's, so a drum or a remnant is hopped).
fn blocked_ahead(sim: &SceneSim, dir: f32) -> bool {
    let hero = &sim.world.hero;
    let probe = Rect {
        left: hero.x - HERO_HALF_WIDTH + dir * 14.0,
        bottom: hero.y - HERO_HALF_HEIGHT + 4.0,
        width: HERO_HALF_WIDTH * 2.0,
        height: 8.0,
    };
    crate::combat::solid_at(&sim.world.solids, probe)
}

fn walk_to(sim: &SceneSim, x: f32) -> RawInput {
    let hero = &sim.world.hero;
    let dx = x - hero.x;
    let dir = if dx.abs() < 4.0 { 0.0 } else { dx.signum() };
    RawInput {
        move_x: dir,
        jump: hero.grounded && dir != 0.0 && blocked_ahead(sim, dir),
        ..RawInput::NONE
    }
}

/// A goblin whose body overlaps Carl's column is out of reach of his own jab
/// (the jab starts 16 px ahead): Carl steps back from it instead of swinging at
/// nothing (the CF02 pilot does not, because its room never lets a goblin land
/// on him after a hop over a prop).
const TOO_CLOSE: f32 = 10.0;

/// The CF02 fight, with the step back from a goblin that stands on Carl.
fn fight(combat: &CombatState, sim: &SceneSim) -> RawInput {
    let hero = &sim.world.hero;
    if !hero.grounded {
        // Over a goblin after a hop (the drum cuts the room): dive on it
        // (Down + Physical in the air, CF02-Q01).
        let below = combat
            .enemies
            .iter()
            .filter(|g| g.alive() && !matches!(g.mode, GoblinMode::Down | GoblinMode::Corpse))
            .filter(|g| (g.x - hero.x).abs() < 40.0 && hero.y - g.y > 16.0)
            .min_by(|a, b| (a.x - hero.x).abs().total_cmp(&(b.x - hero.x).abs()));
        if let Some(g) = below {
            let dx = g.x - hero.x;
            return RawInput {
                move_x: if dx.abs() < 6.0 { 0.0 } else { dx.signum() },
                physical: true,
                down: true,
                ..RawInput::NONE
            };
        }
    }
    if hero.grounded
        && let Some(g) = combat
            .enemies
            .iter()
            .filter(|g| g.alive() && !matches!(g.mode, GoblinMode::Down | GoblinMode::Corpse))
            .filter(|g| (g.x - hero.x).abs() < 40.0 && hero.y - g.y > 24.0)
            .min_by(|a, b| (a.x - hero.x).abs().total_cmp(&(b.x - hero.x).abs()))
    {
        // Standing on a prop above a goblin: hop and dive (the jab would pass over it).
        return RawInput {
            move_x: (g.x - hero.x).signum() * 0.5,
            jump: true,
            ..RawInput::NONE
        };
    }
    let on_top = combat
        .enemies
        .iter()
        .filter(|g| {
            g.alive()
                && !matches!(g.mode, GoblinMode::Down | GoblinMode::Corpse)
                && (g.x - hero.x).abs() < TOO_CLOSE
                && (g.y - hero.y).abs() < 30.0
        })
        .min_by(|a, b| (a.x - hero.x).abs().total_cmp(&(b.x - hero.x).abs()));
    if let Some(g) = on_top
        && hero.grounded
    {
        let away = if g.x >= hero.x { -1.0 } else { 1.0 };
        return RawInput {
            move_x: away,
            ..RawInput::NONE
        };
    }
    arena::autopilot(combat, sim, FightPlan::Direct)
}

/// The CF02 fight while anything lives, the walk to the exit afterwards.
fn fight_or_walk(combat: &CombatState, sim: &SceneSim, exit_x: f32) -> RawInput {
    if alive_any(combat) {
        fight(combat, sim)
    } else {
        walk_to(sim, exit_x)
    }
}

/// An awake goblin within [`FIGHT_RADIUS`] of Carl.
fn threatened(combat: &CombatState, sim: &SceneSim) -> bool {
    let x = sim.world.hero.x;
    combat.enemies.iter().any(|g| {
        g.alive()
            && !matches!(g.mode, GoblinMode::Down | GoblinMode::Corpse)
            && g.ai == crate::combat::EnemyAi::Chase
            && (g.x - x).abs() < FIGHT_RADIUS
    })
}

/// Walk to a prop's reach column and arrive facing right: `None` once Carl
/// stands on it (within one stride, from the left). From the right of it he
/// first steps back left of it, so the jab that follows faces the prop.
fn approach(sim: &SceneSim, column: f32) -> Option<RawInput> {
    let x = sim.world.hero.x;
    if x > column + 4.0 {
        Some(walk_to(sim, column - 24.0))
    } else if x < column - 4.0 {
        Some(walk_to(sim, column))
    } else {
        None
    }
}

fn jab(tick: u64) -> RawInput {
    RawInput {
        attack: tick % JAB_PERIOD == 1,
        ..RawInput::NONE
    }
}

fn valve(combat: &CombatState, sim: &SceneSim, exit_x: f32) -> RawInput {
    let tick = sim.world.tick + 1;
    let Some(env) = combat.env() else {
        return fight_or_walk(combat, sim, exit_x);
    };
    if threatened(combat, sim) {
        return fight(combat, sim);
    }
    match env.valve_state(0) {
        Some(ValveState::Intact) => approach(sim, VALVE_X).unwrap_or_else(|| jab(tick)),
        // The warning and the vent: stand and let the steam run.
        Some(ValveState::Warning | ValveState::Venting) => RawInput::NONE,
        _ => fight_or_walk(combat, sim, exit_x),
    }
}

/// A goblin the kick can still launch into the valve: awake, ahead of Carl,
/// inside the kick's reach, and close enough to the valve to reach its box.
fn kick_target(combat: &CombatState, sim: &SceneSim) -> bool {
    let hero = &sim.world.hero;
    combat.enemies.iter().any(|g| {
        g.alive()
            && g.ai == crate::combat::EnemyAi::Chase
            && !matches!(g.mode, GoblinMode::Down | GoblinMode::Corpse)
            && g.x > hero.x
            && g.x - hero.x <= KICK_REACH
            && g.x >= VALVE_X - KICK_MAX_GAP
            && g.x < VALVE_X
    })
}

/// Ticks after which the chain pilot gives up waiting for a goblin to kick and
/// jabs the valve itself (the fallback of the Valve plan).
const KICK_GIVE_UP_TICK: u64 = 900;

/// The Boiler's full chain, in the order of the `scene_pilots` "both" plan
/// reversed (the valve first, then the brace): wait left of the valve, kick the
/// goblin that comes through into the valve (steam, the brace only weakens),
/// then walk to the brace and jab it: the weight falls, lands on the drum and
/// the drum goes off.
fn chain(combat: &CombatState, sim: &SceneSim, exit_x: f32) -> RawInput {
    let tick = sim.world.tick + 1;
    let Some(env) = combat.env() else {
        return fight_or_walk(combat, sim, exit_x);
    };
    let hero = &sim.world.hero;
    match env.valve_state(0) {
        Some(ValveState::Intact) => {
            if tick > KICK_GIVE_UP_TICK {
                return valve(combat, sim, exit_x);
            }
            if kick_target(combat, sim) {
                return RawInput {
                    move_x: 1.0,
                    physical: tick.is_multiple_of(4),
                    ..RawInput::NONE
                };
            }
            if threatened(combat, sim) {
                return fight(combat, sim);
            }
            // Lure and kite: walk right to wake the sleepers, then back off left
            // of the valve while the nearest one follows, and kick it there.
            let follower = combat
                .enemies
                .iter()
                .filter(|g| {
                    g.alive()
                        && g.ai == crate::combat::EnemyAi::Chase
                        && g.x > hero.x
                        && g.x - hero.x < LURE_RANGE
                })
                .map(|g| g.x - hero.x)
                .min_by(f32::total_cmp);
            match follower {
                Some(gap) if gap < KITE_GAP && hero.x > KICK_STAND_X => walk_to(sim, KICK_STAND_X),
                Some(_) => RawInput::NONE,
                None if hero.x < LURE_X => walk_to(sim, LURE_X),
                None => RawInput::NONE,
            }
        }
        // The steam runs: whoever comes is fought, nothing else.
        Some(ValveState::Warning | ValveState::Venting) => {
            if threatened(combat, sim) {
                fight(combat, sim)
            } else {
                RawInput::NONE
            }
        }
        _ => match env.mass_state(BRACE_PROP as usize) {
            Some(MassState::Supported | MassState::Weakened) => {
                if threatened(combat, sim) {
                    fight(combat, sim)
                } else {
                    approach(sim, BRACE_X).unwrap_or_else(|| jab(tick))
                }
            }
            // Released and falling: stand clear of the footprint and watch.
            Some(MassState::Released | MassState::Falling) => RawInput::NONE,
            _ => fight_or_walk(combat, sim, exit_x),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::combat::{CombatEvent, DefeatCause, PropPhase};
    use crate::f01_boiler::Roster;
    use crate::f01_late::{Loadout, boiler_start_with};
    use std::path::PathBuf;

    /// The Boiler under the story loadout with its live roster, played by `plan`
    /// until the exit or 1500 ticks: the scene, the combat state and the exit column.
    fn play(plan: Plan) -> (SceneSim, CombatState) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (carl, donut) = Loadout::Story.flags();
        let (run, mut scene, mut combat) =
            boiler_start_with(&root, 360, carl, donut, None, Roster::Live).unwrap();
        let exit = run.loaded.encounter.exit;
        let exit_x = (exit.x + exit.w / 2) as f32;
        let pilot = RoomPilot {
            plan,
            two_player: false,
        };
        for _ in 0..1500 {
            if scene.world.complete {
                break;
            }
            let inputs = pilot.inputs(&combat, &scene, exit_x);
            combat.step_party(&mut scene, inputs);
        }
        (scene, combat)
    }

    fn phase_ticks(combat: &CombatState, prop: u16, phase: PropPhase) -> Vec<u64> {
        combat
            .events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::PropPhase {
                    tick,
                    prop: p,
                    phase: ph,
                    ..
                } if *p == prop && *ph == phase => Some(*tick),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn every_plan_clears_the_live_boiler_without_a_defeat_and_replays_identically() {
        for plan in [Plan::Direct, Plan::Valve, Plan::Chain] {
            let (scene, combat) = play(plan);
            assert!(scene.world.complete, "{plan:?} reaches the exit");
            assert_eq!(combat.deaths, 0, "{plan:?} is never defeated");
            assert!(
                combat.enemies.iter().all(|g| !g.alive()),
                "{plan:?} leaves nobody alive behind it"
            );
            let (again, combat_again) = play(plan);
            assert_eq!(
                combat.state_hash(&scene),
                combat_again.state_hash(&again),
                "{plan:?} replays to the same hash"
            );
        }
    }

    #[test]
    fn the_valve_plan_spends_the_valve_and_the_direct_plan_touches_no_prop() {
        let (_, valve) = play(Plan::Valve);
        assert_eq!(phase_ticks(&valve, 0, PropPhase::Spent).len(), 1);
        let (_, direct) = play(Plan::Direct);
        assert!(
            !direct.events.iter().any(|e| matches!(
                e,
                CombatEvent::PropHit { .. } | CombatEvent::PropPhase { .. }
            )),
            "the direct plan walks past every prop"
        );
    }

    /// The whole chain in order, each link once: a launched goblin sets the valve
    /// off, the steam reaches the brace, a jab lets it go, the weight lands on the
    /// drum and the drum's blast defeats a goblin, credited to Carl.
    #[test]
    fn the_chain_plan_runs_launch_steam_brace_landing_drum_in_order() {
        let (_, chain) = play(Plan::Chain);
        let launched = chain.events.iter().any(|e| {
            matches!(e, CombatEvent::PropHit { prop: 0, kind, .. }
                if format!("{kind:?}") == "LaunchedBody")
        });
        assert!(launched, "a launched goblin, not a jab, sets the valve off");
        let at = |prop, phase| {
            let ticks = phase_ticks(&chain, prop, phase);
            assert_eq!(ticks.len(), 1, "{prop} {phase:?} once: {ticks:?}");
            ticks[0]
        };
        let order = [
            at(0, PropPhase::Warning),
            at(0, PropPhase::Venting),
            at(1, PropPhase::Released),
            at(1, PropPhase::Falling),
            at(1, PropPhase::Landed),
            at(2, PropPhase::Detonated),
        ];
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
        let blast_kills = chain
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    CombatEvent::EnemyDefeated {
                        cause: DefeatCause::Explosion { .. },
                        ..
                    }
                )
            })
            .count();
        assert_eq!(blast_kills, 1, "the drum defeats exactly one goblin");
    }

    /// PT-GATE-A: the juicer plan is the PT-SCENE-B solo-Carl pilot (seed 0): the
    /// same inputs, so the same final state as `juicer_pilot`, the gauntlet and the
    /// blink granted once each at the defeat.
    #[test]
    fn the_juicer_plan_is_the_pt_scene_b_solo_carl_pilot() {
        use crate::f01_late_fixtures::{JuicerWho, juicer_pilot, juicer_room};
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let report = juicer_pilot(&root, JuicerWho::CarlSolo, 0).unwrap();
        let pilot = RoomPilot {
            plan: Plan::Juicer,
            two_player: false,
        };
        let mut sim = juicer_room(&root).unwrap();
        for _ in 0..report.ticks {
            let inputs = pilot.inputs(&sim.combat, &sim.scene, 0.0);
            sim.step_slots(inputs);
        }
        assert_eq!(sim.state_hash(), report.hash);
        assert!(report.defeated_tick.is_some() && report.exit_tick.is_some());
        assert_eq!(
            report.grants.len(),
            2,
            "the gauntlet and the blink: {:?}",
            report.grants
        );
    }

    /// PT-GATE-A: the mastery plan is the PT-MASTERY Carl pilot, so a scene run by
    /// it clears the roster, reaches the exit and replays to the same hash.
    #[test]
    fn the_mastery_plan_is_the_pt_mastery_carl_pilot_and_replays_identically() {
        use crate::f01_late::mastery_start;
        use crate::f01_mastery_fixtures::{EXIT_X, RUN_TICKS};
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let play = || {
            let (carl, donut) = Loadout::Story.flags();
            let (_run, mut scene, mut combat) =
                mastery_start(&root, 360, carl, donut, Some(0)).unwrap();
            let pilot = RoomPilot {
                plan: Plan::Mastery,
                two_player: false,
            };
            for _ in 0..RUN_TICKS {
                if combat.roster_cleared() && scene.world.hero.x >= EXIT_X {
                    break;
                }
                let inputs = pilot.inputs(&combat, &scene, 0.0);
                combat.step_party(&mut scene, inputs);
            }
            (
                combat.roster_cleared(),
                scene.world.hero.x,
                combat.state_hash(&scene),
            )
        };
        let (cleared, x, hash) = play();
        assert!(cleared && x >= EXIT_X, "cleared={cleared} x={x}");
        assert_eq!(play().2, hash);
    }

    #[test]
    fn the_flag_parses_and_round_trips() {
        for (text, plan, two) in [
            ("direct", Plan::Direct, false),
            ("valve:2p", Plan::Valve, true),
            ("chain", Plan::Chain, false),
            ("mastery", Plan::Mastery, false),
            ("juicer", Plan::Juicer, false),
        ] {
            let pilot = RoomPilot::parse(text).unwrap();
            assert_eq!((pilot.plan, pilot.two_player), (plan, two));
            assert_eq!(pilot.label(), text);
        }
        assert!(RoomPilot::parse("sprint").is_err());
        assert!(RoomPilot::parse("direct:3p").is_err());
        assert!(RoomPilot::parse("").is_err());
    }
}
