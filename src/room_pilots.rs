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
use crate::party::{HeroId, PartyTuning};
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
/// `walk_to` stops within this many px of its target.
const WALK_DEADZONE: f32 = 4.0;
/// How far inside `LURE_RANGE` the lure walks (px): past the dead zone of `walk_to`.
const LURE_MARGIN: f32 = 6.0;
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

/// PT-GATE-C1: who plays an aligned room pilot (`mastery:<who>:<seed>`,
/// `juicer:<who>:<seed>`): the same four pilots as the library fixtures
/// ([`crate::f01_mastery_fixtures::Who`] and [`crate::f01_late_fixtures::JuicerWho`]),
/// so a scene run and a library run play the same fight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignedWho {
    /// Solo Carl; the other hero is the party's `cf02_assist` partner (mastery) or the
    /// follow-and-jump bot (Juicer).
    CarlSolo,
    /// Carl clears the room, then the swap, then Donut (solo) walks to the exit.
    DonutBySwap,
    /// Mastery only: the swap on tick 90, Donut (solo) fights the room.
    DonutFights,
    /// Carl is the pilot's, Donut is the autonomous `cf02_free` partner.
    Companion,
}

impl AlignedWho {
    pub const fn token(self) -> &'static str {
        match self {
            Self::CarlSolo => "carl_solo",
            Self::DonutBySwap => "donut_by_swap",
            Self::DonutFights => "donut_fights",
            Self::Companion => "companion",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        [
            Self::CarlSolo,
            Self::DonutBySwap,
            Self::DonutFights,
            Self::Companion,
        ]
        .into_iter()
        .find(|who| who.token() == value)
    }

    /// The library fixture's `Who` of the mastery room.
    pub const fn mastery(self) -> crate::f01_mastery_fixtures::Who {
        use crate::f01_mastery_fixtures::Who;
        match self {
            Self::CarlSolo => Who::CarlSolo,
            Self::DonutBySwap => Who::DonutBySwap,
            Self::DonutFights => Who::DonutFights,
            Self::Companion => Who::Companion,
        }
    }
}

/// An aligned room pilot: the library fixture's pilot, party and seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aligned {
    pub who: AlignedWho,
    pub seed: u64,
}

/// A parsed `--room-pilot` value: `PLAN`, `PLAN:2p`, or `mastery|juicer:WHO:SEED`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomPilot {
    pub plan: Plan,
    /// P2 plays Donut with the CF02 pilot (two local players); otherwise the
    /// follower bot drives her, as in every scripted solo run.
    pub two_player: bool,
    /// PT-GATE-C1: the run is the library fixture's (same party, same seed, same
    /// inputs). The plain `mastery` plan is `mastery:carl_solo:0`; the plain
    /// `juicer` plan stays the unaligned PT-GATE-A pilot.
    pub aligned: Option<Aligned>,
}

impl RoomPilot {
    pub fn parse(value: &str) -> Result<Self, String> {
        let (plan, rest) = value.split_once(':').map_or((value, ""), |(a, b)| (a, b));
        if let Some((who, seed)) = rest.split_once(':')
            && matches!(plan, "mastery" | "juicer")
        {
            // PT-GATE-C1: `mastery:<who>:<seed>` and `juicer:<who>:<seed>`.
            let juicer = plan == "juicer";
            let who = AlignedWho::parse(who)
                .filter(|who| !(juicer && *who == AlignedWho::DonutFights))
                .ok_or_else(|| {
                    format!(
                        "bad --room-pilot who {who:?} (expected carl_solo, donut_by_swap, {}companion)",
                        if juicer { "" } else { "donut_fights, " }
                    )
                })?;
            let seed = seed
                .parse::<u64>()
                .map_err(|_| format!("bad --room-pilot seed {seed:?} (an integer)"))?;
            return Ok(Self {
                plan: if juicer { Plan::Juicer } else { Plan::Mastery },
                two_player: false,
                aligned: Some(Aligned { who, seed }),
            });
        }
        let plan = match plan {
            "direct" => Plan::Direct,
            "valve" => Plan::Valve,
            "chain" => Plan::Chain,
            "mastery" => Plan::Mastery,
            "juicer" => Plan::Juicer,
            other => {
                return Err(format!(
                    "unknown --room-pilot plan {other:?} (expected direct, valve, chain, mastery or juicer, optionally :2p, or mastery|juicer:WHO:SEED)"
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
        // The plain mastery plan is the library's `carl_solo` at seed 0 (PT-GATE-C1).
        let aligned = (plan == Plan::Mastery && !two_player).then_some(Aligned {
            who: AlignedWho::CarlSolo,
            seed: 0,
        });
        Ok(Self {
            plan,
            two_player,
            aligned,
        })
    }

    /// The value that parses back to this pilot.
    pub fn label(self) -> String {
        if let Some(a) = self.aligned
            && !(self.plan == Plan::Mastery && a.who == AlignedWho::CarlSolo && a.seed == 0)
        {
            format!("{}:{}:{}", self.plan.name(), a.who.token(), a.seed)
        } else if self.two_player {
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

/// PT-GATE-C1: the party tuning of an aligned run, the library fixture's own
/// ([`crate::f01_mastery_fixtures::tuning`]; the Juicer's room party, or its
/// `cf02_free` partner under [`AlignedWho::Companion`]).
pub fn aligned_tuning(plan: Plan, who: AlignedWho) -> PartyTuning {
    use crate::f01_late_fixtures::{JUICER_COMPANION_PARTY, JUICER_ROOM_PARTY};
    match plan {
        Plan::Juicer => {
            if who == AlignedWho::Companion {
                JUICER_COMPANION_PARTY.tuning
            } else {
                JUICER_ROOM_PARTY.tuning
            }
        }
        _ => crate::f01_mastery_fixtures::tuning(who.mastery()),
    }
}

impl RoomPilot {
    /// The party tuning of an aligned run, or `None` for an unaligned pilot.
    pub fn tuning(self) -> Option<PartyTuning> {
        self.aligned.map(|a| aligned_tuning(self.plan, a.who))
    }

    /// The slot inputs of an aligned run at the next tick: exactly the library
    /// fixture's (`f01_mastery_fixtures::run_for`, `f01_late_fixtures::juicer_run`),
    /// for the hero `controlled` by the party. `swap_sent` is the run's "the swap
    /// was pressed once" flag. Panics on an unaligned pilot.
    pub fn aligned_inputs(
        self,
        combat: &CombatState,
        sim: &SceneSim,
        controlled: HeroId,
        swap_sent: &mut bool,
    ) -> [RawInput; 2] {
        use crate::f01_mastery_fixtures as mastery;
        let Aligned { who, seed } = self.aligned.expect("an aligned room pilot");
        let tick = sim.world.tick + 1;
        let mut input = if self.plan == Plan::Juicer {
            if controlled == HeroId::Carl {
                crate::boss_scene::pilot_inputs(
                    Pilot::Carl,
                    [RawInput::NONE; 2],
                    tick + seed % crate::f01_late_fixtures::SWING_PERIOD,
                    combat,
                    sim,
                )[0]
            } else {
                // Donut (swapped in after the defeat) walks to the exit.
                RawInput {
                    move_x: 1.0,
                    ..RawInput::NONE
                }
            }
        } else {
            match who {
                AlignedWho::CarlSolo | AlignedWho::Companion => {
                    mastery::pilot(combat, sim, HeroId::Carl)
                }
                AlignedWho::DonutBySwap | AlignedWho::DonutFights => {
                    mastery::pilot(combat, sim, controlled)
                }
            }
        };
        let due = if self.plan == Plan::Juicer {
            who == AlignedWho::DonutBySwap && combat.boss.as_ref().is_some_and(|b| b.released())
        } else {
            match who {
                AlignedWho::DonutFights => tick >= mastery::SWAP_TICK,
                AlignedWho::DonutBySwap => combat.roster_cleared(),
                _ => false,
            }
        };
        if due && !*swap_sent && controlled == HeroId::Carl {
            input.swap = true;
            *swap_sent = true;
        }
        [input, RawInput::NONE]
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
    let dir = if dx.abs() < WALK_DEADZONE {
        0.0
    } else {
        dx.signum()
    };
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
            // A live party with an AI hero (not two players): the partner takes part.
            let ai_partner = sim.drive().live && !sim.drive().two_player;
            // The nearest live goblin ahead that no follower rule has claimed yet.
            let sleeper_ahead = combat
                .enemies
                .iter()
                .filter(|g| g.alive() && g.ai == crate::combat::EnemyAi::Chase && g.x > hero.x)
                .map(|g| g.x)
                .min_by(f32::total_cmp);
            match follower {
                Some(gap) if gap < KITE_GAP && hero.x > KICK_STAND_X => walk_to(sim, KICK_STAND_X),
                // With an AI partner on the field a goblin may target her and never come:
                // close in until it is a kite gap away. (An unaccompanied run, and the
                // two-player run, are unchanged: their committed pilot hashes pin them.)
                Some(gap) if ai_partner && gap > KITE_GAP + 10.0 => walk_to(sim, hero.x + 20.0),
                Some(_) => RawInput::NONE,
                // PTG fix (the chain wake step overshoot): `walk_to` stops within 4 px of its
                // target, so the old `hero.x < LURE_X` arm parked Carl on x 556..560 and held
                // him there forever. This arm ends where `walk_to` does; Carl then walks on
                // until a sleeper is within `LURE_RANGE` (a goblin wakes on a hero within 220
                // px, so a trigger column of the room data is not what wakes it).
                None if hero.x < LURE_X - WALK_DEADZONE => walk_to(sim, LURE_X),
                None => match sleeper_ahead {
                    Some(gx) if ai_partner && hero.x < gx - LURE_RANGE + LURE_MARGIN => {
                        walk_to(sim, gx - LURE_RANGE + 2.0 * LURE_MARGIN)
                    }
                    _ => RawInput::NONE,
                },
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
            aligned: None,
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

    /// PTG fix (the chain wake step overshoot): with a live party the chain plan used to
    /// park Carl on x 556..560, short of the room data's 560 wake trigger, and never
    /// reached the exit. On the live Boiler roster at seeds 0..=8, with the `cf02_assist`
    /// partner and with the autonomous one, the plan reaches the exit within 3000 ticks and
    /// Carl is never told to walk and stays still for more than 300 ticks.
    #[test]
    fn the_chain_plan_with_a_party_reaches_the_exit_on_the_live_boiler_at_every_seed() {
        use crate::f01_mastery_fixtures::{Who, tuning};
        use crate::party_scene::{PartyFixture, PartyFixtureSim};
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (carl, donut) = Loadout::Story.flags();
        let pilot = RoomPilot {
            plan: Plan::Chain,
            two_player: false,
            aligned: None,
        };
        // The live room's partners, as the `boiler_fixtures` live pilots run them:
        // the `cf02_assist` partner and the autonomous `cf02_free` one.
        for tuning in [tuning(Who::CarlSolo), tuning(Who::Companion)] {
            for seed in 0..=8u64 {
                let (run, scene, combat) =
                    boiler_start_with(&root, 360, carl, donut, Some(seed), Roster::Live).unwrap();
                let exit = run.loaded.encounter.exit;
                let exit_x = (exit.x + exit.w / 2) as f32;
                let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
                    base: run.fixture,
                    two_player: false,
                    tuning,
                    script: &[],
                    pilot: crate::boss_scene::Pilot::ArenaDirect,
                    donut: crate::donut_mana::DonutSetup::unlocked(8),
                }));
                let mut sim = PartyFixtureSim::from_parts(scene, combat, fixture);
                let (mut last_x, mut still, mut max_still) = (sim.scene.world.hero.x, 0u32, 0u32);
                for _ in 0..3000 {
                    if sim.scene.world.complete {
                        break;
                    }
                    let slots = pilot.inputs(&sim.combat, &sim.scene, exit_x);
                    sim.step_slots(slots);
                    let x = sim.scene.world.hero.x;
                    let walking = slots[0].move_x != 0.0;
                    still = if walking && (x - last_x).abs() < 0.1 {
                        still + 1
                    } else {
                        0
                    };
                    max_still = max_still.max(still);
                    last_x = x;
                }
                assert!(
                    sim.scene.world.complete,
                    "seed {seed} {tuning:?}: no exit in 3000 ticks (Carl at x {})",
                    sim.scene.world.hero.x
                );
                assert!(
                    max_still <= 300,
                    "seed {seed}: Carl stuck for {max_still} ticks"
                );
            }
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
            aligned: None,
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
                aligned: None,
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

    /// PT-GATE-C1: an aligned room pilot plays the library fixture's fight: for
    /// every (who, seed) of the Juicer the room pilot's final state hash equals
    /// the PT-SCENE-B fixture's (27 runs), and for every (who, seed) of the
    /// closing arena it equals the PT-TUNE fixture's (36 runs). The room pilot
    /// reads the party's controlled hero and its own swap flag, nothing else.
    #[test]
    fn an_aligned_room_pilot_equals_the_library_fixture_for_every_who_and_seed() {
        use crate::f01_late_fixtures::{
            JUICER_COMPANION_PARTY, JUICER_ROOM_PARTY, JUICER_SEEDS, JuicerWho, juicer_pilot,
            juicer_room_for,
        };
        use crate::f01_mastery_fixtures as mastery;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut equal = 0;
        for (who, library_who) in [
            (AlignedWho::CarlSolo, JuicerWho::CarlSolo),
            (AlignedWho::DonutBySwap, JuicerWho::DonutBySwap),
            (AlignedWho::Companion, JuicerWho::Companion),
        ] {
            for seed in JUICER_SEEDS {
                let report = juicer_pilot(&root, library_who, seed).unwrap();
                let pilot = RoomPilot::parse(&format!("juicer:{}:{seed}", who.token())).unwrap();
                assert_eq!(pilot.label(), format!("juicer:{}:{seed}", who.token()));
                let party = if who == AlignedWho::Companion {
                    &JUICER_COMPANION_PARTY
                } else {
                    &JUICER_ROOM_PARTY
                };
                assert_eq!(pilot.tuning(), Some(party.tuning));
                let mut sim = juicer_room_for(&root, party).unwrap();
                let mut sent = false;
                for _ in 0..report.ticks {
                    let controlled = sim.party.controlled_hero();
                    let slots =
                        pilot.aligned_inputs(&sim.combat, &sim.scene, controlled, &mut sent);
                    sim.step_slots(slots);
                }
                assert_eq!(sim.state_hash(), report.hash, "juicer {who:?} seed {seed}");
                equal += 1;
            }
        }
        for who in [
            AlignedWho::CarlSolo,
            AlignedWho::DonutBySwap,
            AlignedWho::DonutFights,
            AlignedWho::Companion,
        ] {
            for seed in mastery::SEEDS {
                let report = mastery::run(&root, who.mastery(), seed).unwrap();
                let pilot = RoomPilot::parse(&format!("mastery:{}:{seed}", who.token())).unwrap();
                assert_eq!(pilot.tuning(), Some(mastery::tuning(who.mastery())));
                let (mut sim, _) = mastery::start(&root, who.mastery(), seed).unwrap();
                let mut sent = false;
                for _ in 0..report.ticks {
                    let controlled = sim.party.controlled_hero();
                    let slots =
                        pilot.aligned_inputs(&sim.combat, &sim.scene, controlled, &mut sent);
                    sim.step_slots(slots);
                }
                assert_eq!(sim.state_hash(), report.hash, "mastery {who:?} seed {seed}");
                equal += 1;
            }
        }
        assert_eq!(equal, 63, "27 Juicer runs and 36 arena runs");
    }

    #[test]
    fn the_plain_mastery_plan_is_carl_solo_at_seed_zero() {
        let plain = RoomPilot::parse("mastery").unwrap();
        let named = RoomPilot::parse("mastery:carl_solo:0").unwrap();
        assert_eq!(plain, named);
        assert_eq!(plain.label(), "mastery");
        assert_eq!(
            plain.aligned,
            Some(Aligned {
                who: AlignedWho::CarlSolo,
                seed: 0
            })
        );
        // The plain Juicer plan stays the unaligned PT-GATE-A pilot; the rest of the
        // plans are unaligned too.
        for text in ["juicer", "direct", "valve", "chain:2p"] {
            assert_eq!(RoomPilot::parse(text).unwrap().aligned, None, "{text}");
        }
        // A two-player mastery plan is not the library's solo pilot (the `:2p` refusal stays).
        assert!(RoomPilot::parse("mastery:2p").unwrap().two_player);
        assert_eq!(RoomPilot::parse("mastery:2p").unwrap().aligned, None);
        for bad in [
            "mastery:carl_solo",
            "mastery:carl_solo:x",
            "mastery:nobody:1",
            "juicer:donut_fights:1",
            "mastery:carl_solo:1:2p",
            "direct:carl_solo:1",
        ] {
            assert!(RoomPilot::parse(bad).is_err(), "{bad}");
        }
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
