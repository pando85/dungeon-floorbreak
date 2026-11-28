//! BOS-A: the Ball of Swine's (room 18, `bos`) policies, party fixtures and
//! fixture tests F1..F18 (`docs/01-design/floor-01-bos-integration.md` 9.2).
//! Bevy-free. The encounter itself is [`crate::boss`] on the `rolling_boss_f01`
//! data; the arena is [`crate::boss_scene::load_ball_arena`].
//!
//! The policies are pure functions of the finished state (so a replay is
//! bit-identical): [`bait_pilot`] is `carl_bait_punish`, the control of the
//! evaluator (`crate::bos_eval`, row G-0).

use std::path::Path;

use crate::boss::{Boss, BossMode, RackState};
use crate::boss_scene::{BALL_FULL, Pilot, ball_hp, ball_story_setup, boss_base, load_ball_arena};
use crate::combat::CombatEvent;
use crate::combat_scene::{CombatState, Fixture};
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::f01_late::Loadout;
use crate::input::RawInput;
use crate::party::HeroId;
use crate::party::PROVISIONAL_TUNING;
use crate::party_scene::PartyFixture;
use crate::party_scene::PartyFixtureSim;
use crate::story_unlock::LedgerSnapshot;

/// Where the pilot waits inside the left and the right alcove (valid pivots
/// 40..110 and 530..596).
pub const ALCOVE_LEFT_X: f32 = 80.0;
pub const ALCOVE_RIGHT_X: f32 = 560.0;
/// The jab period of the pilot: a first-clear player, not a frame-perfect bot.
const SWING_PERIOD: u64 = 16;
/// Where the pilot waits just outside an alcove (reachable pivots 111..529).
const STEP_OUT_LEFT_X: f32 = 118.0;
const STEP_OUT_RIGHT_X: f32 = 522.0;
/// A hero this far (px, pivot to pivot) from the ball is outside its shove box.
const SHOVE_CLEAR_PX: f32 = 118.0;
/// The pilot walks past the engage line to this column to close the gate.
const ENTER_X: f32 = 150.0;

const fn run(dir: f32) -> RawInput {
    RawInput {
        move_x: dir,
        ..RawInput::NONE
    }
}

const fn toward(from: f32, to: f32) -> f32 {
    if to > from { 1.0 } else { -1.0 }
}

/// BOSB: the pilot fights a Tuskling this close (px, pivot to pivot): the reach of its lunge.
const TUSK_FIGHT_PX: f32 = 44.0;
/// BOSB: the pilot closes to this gap before it jabs.
const TUSK_JAB_PX: f32 = 30.0;

/// The signed x gap (target minus `x`) to the nearest living, non-cowering Tuskling within
/// `within` px, or `None`.
pub fn nearest_tusk_gap(combat: &CombatState, x: f32, within: f32) -> Option<f32> {
    use crate::combat_scene::tuskling::TuskEnemy;
    use crate::tuskling::State;
    combat
        .slot_enemies()
        .iter()
        .filter_map(|e| e.as_any()?.downcast_ref::<TuskEnemy>())
        .filter(|t| !matches!(t.tusk().state, State::Down | State::Cower))
        .map(|t| t.tusk().x - x)
        .filter(|gap| gap.abs() <= within)
        .min_by(|a, b| a.abs().total_cmp(&b.abs()))
}

/// Jab the Tuskling at signed gap `gap` without leaving the spot: turn to it (one step), then a
/// jab every 12 ticks.
fn hold_and_jab(tick: u64, gap: f32, facing: i8) -> RawInput {
    let side = if gap >= 0.0 { 1.0 } else { -1.0 };
    if f32::from(facing) != side {
        return run(side);
    }
    RawInput {
        attack: tick.is_multiple_of(12),
        ..RawInput::NONE
    }
}

/// Jab the Tuskling at signed gap `gap`: close in, then a jab every 12 ticks (BOSB, AD13).
fn fight_tusk(tick: u64, gap: f32) -> RawInput {
    let side = if gap >= 0.0 { 1.0 } else { -1.0 };
    if gap.abs() > TUSK_JAB_PX {
        return run(side);
    }
    RawInput {
        attack: tick.is_multiple_of(12),
        move_x: side,
        ..RawInput::NONE
    }
}

/// `carl_bait_punish` (BOS-AD22, F11): Carl runs into the alcove behind the arch
/// the roll is aimed at, steps out to punish the crash (not the shove: it baits
/// the next roll instead, BOS-AD34), and steps out of the shove's box. A pure function of the state.
pub fn bait_pilot(tick: u64, combat: &CombatState, scene: &SceneSim, boss: &Boss) -> RawInput {
    bait_inner(tick, combat, scene, boss, true)
}

/// [`bait_pilot`] that leaves the Tusklings alone: the Ball's half of `duo_coop` (BOSB-AD13),
/// whose other hero is on them.
pub fn bait_ball_only(tick: u64, combat: &CombatState, scene: &SceneSim, boss: &Boss) -> RawInput {
    bait_inner(tick, combat, scene, boss, false)
}

fn bait_inner(
    tick: u64,
    combat: &CombatState,
    scene: &SceneSim,
    boss: &Boss,
    fight_tusks: bool,
) -> RawInput {
    let hero = &scene.world.hero;
    let x = hero.x;
    let idle = RawInput::default();
    if combat.hero_defeated {
        return idle;
    }
    if boss.released() {
        return RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        };
    }
    if !boss.gate_closed() {
        return RawInput {
            move_x: if x < ENTER_X { 1.0 } else { 0.0 },
            ..RawInput::NONE
        };
    }
    match boss.mode {
        BossMode::ChargeTell | BossMode::Charge => {
            let side = boss.run_dir();
            let (safe, target) = if side < 0 {
                (x <= 104.0, ALCOVE_LEFT_X)
            } else {
                (x >= 536.0, ALCOVE_RIGHT_X)
            };
            if safe {
                // In the alcove the roll cannot reach him: a Tuskling that follows him in is jabbed
                // in place (BOSB), never chased out.
                match nearest_tusk_gap(combat, x, TUSK_FIGHT_PX).filter(|_| fight_tusks) {
                    Some(gap) => hold_and_jab(tick, gap, hero.facing),
                    None => idle,
                }
            } else {
                RawInput {
                    move_x: toward(x, target),
                    ..RawInput::NONE
                }
            }
        }
        BossMode::PoundTell | BossMode::PoundStrike => {
            let away = x - boss.x;
            if away.abs() < SHOVE_CLEAR_PX {
                RawInput {
                    move_x: if away >= 0.0 { 1.0 } else { -1.0 },
                    ..RawInput::NONE
                }
            } else if let Some(gap) =
                nearest_tusk_gap(combat, x, TUSK_FIGHT_PX).filter(|_| fight_tusks)
            {
                hold_and_jab(tick, gap, hero.facing)
            } else {
                idle
            }
        }
        _ => {
            // BOSB: a Tuskling within a lunge's reach is jabbed first (two jabs lay one down, a
            // jab on its tell cancels it); a farther one is ignored.
            if let Some(gap) = nearest_tusk_gap(combat, x, TUSK_FIGHT_PX).filter(|_| fight_tusks) {
                return fight_tusk(tick, gap);
            }
            let Some(body) = boss.hurtbox() else {
                return idle;
            };
            let vulnerable = matches!(
                boss.mode,
                BossMode::Rebound | BossMode::Stun | BossMode::Pinned | BossMode::Flinch
            );
            let gap = if x < body.left {
                body.left - x
            } else if x > body.right() {
                x - body.right()
            } else {
                0.0
            };
            let side = if x < body.left { 1.0 } else { -1.0 };
            if !vulnerable {
                // Bait: stay at the edge of the shove's reach (never nearer
                // than the box) and let the ball come or roll.
                if gap < 56.0 {
                    return RawInput {
                        move_x: -side,
                        ..RawInput::NONE
                    };
                }
                // Out of the alcove once the ball has walked back far enough (BOS-AD34): a
                // hero inside it is unreachable, so no roll could be baited.
                if x <= 110.0 && (boss.x - STEP_OUT_LEFT_X).abs() >= 130.0 {
                    return run(1.0);
                }
                if x >= 530.0 && (boss.x - STEP_OUT_RIGHT_X).abs() >= 130.0 {
                    return run(-1.0);
                }
                return idle;
            }
            if gap > 18.0 {
                return RawInput {
                    move_x: side,
                    ..RawInput::NONE
                };
            }
            if tick.is_multiple_of(SWING_PERIOD) {
                RawInput {
                    attack: true,
                    move_x: side,
                    ..RawInput::NONE
                }
            } else {
                idle
            }
        }
    }
}

/// The column where Carl jabs the chain-weight post (x 64..80) from inside the
/// left alcove (F10).
pub const POST_X: f32 = 52.0;

/// BOS-ART evidence pilot `bos_weight`: [`bait_pilot`] that first stands at [`POST_X`] in the left
/// alcove and jabs the chain-weight post twice, then waits there (the siege rolls at the alcove and
/// crashes into the arch) and jabs the third time on the crash, so the weight lands on the stunned
/// ball (F10 with a solo Carl). It plays on as [`bait_pilot`] once the rack has dropped. Harness
/// only: no fixture, golden or pin selects it. A pure function of the state.
pub fn weight_pilot(tick: u64, combat: &CombatState, scene: &SceneSim, boss: &Boss) -> RawInput {
    let x = scene.world.hero.x;
    let live = boss.gate_closed() && !boss.released() && !combat.hero_defeated;
    if let (true, RackState::Idle { hits }) = (live, boss.racks[0].state) {
        let jab = RawInput {
            attack: true,
            move_x: 1.0,
            ..RawInput::NONE
        };
        if hits < 2 {
            if (x - POST_X).abs() > 3.0 {
                return run(toward(x, POST_X));
            }
            return if tick.is_multiple_of(20) {
                jab
            } else {
                RawInput::default()
            };
        }
        if boss.mode == BossMode::Rebound {
            return jab;
        }
        return RawInput::default();
    }
    bait_pilot(tick, combat, scene, boss)
}

/// `carl_environment` (BOSB-AD13): Carl baits the crashes and uses the chain weight
/// ([`weight_pilot`]), and jabs a Tuskling that comes within a lunge's reach first. A pure
/// function of the state.
pub fn environment_pilot(
    tick: u64,
    combat: &CombatState,
    scene: &SceneSim,
    boss: &Boss,
) -> RawInput {
    let live = boss.gate_closed() && !boss.released() && !combat.hero_defeated;
    if live && let Some(gap) = nearest_tusk_gap(combat, scene.world.hero.x, TUSK_FIGHT_PX) {
        return hold_and_jab(tick, gap, scene.world.hero.facing);
    }
    weight_pilot(tick, combat, scene, boss)
}

/// The pocket column where the evidence pilot [`kick_pilot`] waits for a Tuskling: just inside the
/// left arch's lane, where the ball (standing at the crash pivot) is a Tuskling's length away.
pub const KICK_POCKET_X: f32 = 125.0;

/// BOSB evidence pilot `bos_kick`: [`bait_pilot`] until the first crash has shed its Tusklings, then
/// Carl stands at [`KICK_POCKET_X`] and kicks the Tuskling that comes within reach, so it flies into
/// the ball (BOSB-AD8). It baits the rolls as usual (the ball's tells win over the kick). Harness
/// only: no fixture, golden or pin selects it. A pure function of the state.
pub fn kick_pilot(tick: u64, combat: &CombatState, scene: &SceneSim, boss: &Boss) -> RawInput {
    let x = scene.world.hero.x;
    let busy = matches!(boss.mode, BossMode::ChargeTell | BossMode::Charge);
    if combat.hero_defeated || boss.released() || busy || combat.tusk_impacts() == 0 {
        return bait_pilot(tick, combat, scene, boss);
    }
    if combat.tusklings_live() == 0 {
        return bait_pilot(tick, combat, scene, boss);
    }
    if (x - KICK_POCKET_X).abs() > 4.0 {
        return run(toward(x, KICK_POCKET_X));
    }
    match nearest_tusk_gap(combat, x, 34.0).filter(|g| *g > 0.0) {
        Some(_) => RawInput {
            physical: true,
            move_x: 1.0,
            ..RawInput::NONE
        },
        None => RawInput::default(),
    }
}

/// The Tuskling hunter of `duo_coop` (BOSB-AD13): Donut turns to the nearest Tuskling within
/// the missile's reach, casts at it, and backs off the alcove side when it gets close. With
/// none, she waits in the left alcove. A pure function of the state.
pub fn donut_tusk_pilot(
    tick: u64,
    combat: &CombatState,
    scene: &SceneSim,
    boss: &Boss,
) -> RawInput {
    let x = scene.world.donut_body.x;
    let facing = scene.world.donut_body.facing;
    if boss.released() {
        return RawInput::default();
    }
    if !boss.gate_closed() {
        // Both seats walk in to close the gate.
        return if x < ENTER_X {
            run(1.0)
        } else {
            RawInput::default()
        };
    }
    let Some(gap) = nearest_tusk_gap(combat, x, DONUT_CAST_PX) else {
        return if x > ALCOVE_LEFT_X + 20.0 {
            run(-1.0)
        } else {
            RawInput::default()
        };
    };
    let side = if gap >= 0.0 { 1.0 } else { -1.0 };
    if gap.abs() < DONUT_BACK_OFF_PX {
        return run(-side);
    }
    if f32::from(facing) != side {
        return run(side);
    }
    RawInput {
        attack: tick.is_multiple_of(2),
        ..RawInput::NONE
    }
}

/// Donut casts at a Tuskling this close (px) and no farther.
const DONUT_CAST_PX: f32 = 260.0;
/// She backs away from one closer than this (px).
const DONUT_BACK_OFF_PX: f32 = 50.0;

const CLEAR_SOLO_BASE: Fixture = boss_base(
    "ball01_full_clear_solo",
    "BOS-A F11: solo Carl clears the Ball with the bait-and-punish pilot, the follower bot at his back",
    0x0B05_0101,
    56,
    BALL_FULL,
    7200,
);
/// F11, G-0: the solo clear (Carl pilot, Donut the live follower bot).
pub const BALL01_FULL_CLEAR_SOLO: PartyFixture = PartyFixture {
    base: &CLEAR_SOLO_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::CarlBait,
    donut: DonutSetup::unlocked(8),
};

const ENV_BASE: Fixture = boss_base(
    "ball01_env_clear_solo",
    "BOSB-AD13 carl_environment: solo Carl baits the crashes, uses the chain weight and jabs the Tusklings, the follower bot at his back",
    0x0B05_0111,
    56,
    BALL_FULL,
    7200,
);
/// `carl_environment` (BOSB-AD13): the solo clear that uses the weight (G-0e).
pub const BALL01_ENV_CLEAR_SOLO: PartyFixture = PartyFixture {
    base: &ENV_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::CarlEnvironment,
    donut: DonutSetup::unlocked(8),
};

const COOP_BASE: Fixture = boss_base(
    "ball01_duo_coop",
    "BOSB-AD13 duo_coop: two players, Carl on the Ball (bait and punish), Donut on the Tusklings (missiles)",
    0x0B05_0112,
    56,
    BALL_FULL,
    7200,
);
/// `duo_coop` (BOSB-AD13): one hero on the Tusklings, one on the Ball (G-0d).
pub const BALL01_DUO_COOP: PartyFixture = PartyFixture {
    base: &COOP_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::DuoCoop,
    donut: DonutSetup::unlocked(8),
};

// ---------------------------------------------------------------------- rig --

const DUO_BASE: Fixture = boss_base(
    "ball01_duo",
    "BOS-A: two players in the Ball's arena, scripted by the test or the evaluator",
    0x0B05_0102,
    56,
    BALL_FULL,
    7200,
);
/// Two human seats in the Ball's arena, no autopilot (the evaluator's 2P rows
/// and the fixture tests). Not a replay fixture.
pub const BALL01_DUO: PartyFixture = PartyFixture {
    base: &DUO_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::unlocked(8),
};

const SOLO_BASE: Fixture = boss_base(
    "ball01_solo",
    "BOS-A: solo in the Ball's arena (the other hero is the live follower bot), scripted by the test or the evaluator",
    0x0B05_0103,
    56,
    BALL_FULL,
    7200,
);
/// One human seat, the follower bot as the other hero, no autopilot.
pub const BALL01_SOLO: PartyFixture = PartyFixture {
    base: &SOLO_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::unlocked(8),
};

/// What a run of the Ball's room has produced so far, read from the combat event
/// log (a count per tick, never from the simulation's own state).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tally {
    /// Damage dealt to the boss by heroes and the weight.
    pub boss_damage: u32,
    /// Boss hits on heroes by the boss' own source (ram, shove, siege debris).
    pub hero_hits: u32,
    /// Health lost by the heroes in total, across restarts.
    pub party_loss: u32,
    /// Boss hits on heroes that dealt the siege's 2 damage.
    pub debris_hits: u32,
    /// Crashes by wall (-1 left, +1 right).
    pub crashes: Vec<(u64, i8)>,
    /// The tick of the boss' defeat event.
    pub defeated_at: Option<u64>,
    /// Boss events that must never happen on this data (Roar, Shockwave, DoublePound).
    pub forbidden: u32,
    /// Party defeats (restarts).
    pub restarts: u32,
}

/// A sim of the Ball's room with a running [`Tally`].
pub struct Rig {
    pub sim: PartyFixtureSim,
    pub tally: Tally,
    seen: usize,
    health: [u8; 2],
}

impl Rig {
    /// A fresh room on `fixture`.
    pub fn new(root: &Path, fixture: &'static PartyFixture) -> Result<Self, String> {
        let kit = load_ball_arena(root)?;
        let sim = PartyFixtureSim::new(&kit, 360, fixture)?;
        let health = [
            sim.scene.world.hero.health,
            sim.scene.world.donut_body.health,
        ];
        Ok(Self {
            sim,
            tally: Tally::default(),
            seen: 0,
            health,
        })
    }

    /// Setup only (the evaluator's `--q11`): the boss runs on the Ball's data with
    /// BOS-Q11 forced to `on`. Call before the first step.
    pub fn set_ignore_bot_decoy(&mut self, on: bool) {
        let floor_y = self.boss().floor_y();
        self.sim.combat.boss = Some(Boss::new(crate::boss::ball_with_decoy_rule(on), floor_y));
    }

    pub fn boss(&self) -> &Boss {
        self.sim.combat.boss.as_ref().expect("a boss fixture")
    }

    pub fn boss_mut(&mut self) -> &mut Boss {
        self.sim.combat.boss.as_mut().expect("a boss fixture")
    }

    /// Setup only: put a hero's pivot at `x` (a tick-0 placement or a staged
    /// position of a scripted test).
    pub fn place(&mut self, hero: HeroId, x: f32) {
        let body = self.sim.scene.world.body_mut(hero);
        body.x = x;
        body.vx = 0.0;
        self.sim.scene.world.sync_donut_for_tests();
    }

    pub fn x_of(&self, hero: HeroId) -> f32 {
        self.sim.scene.world.body(hero).x
    }

    pub fn health_of(&self, hero: HeroId) -> u8 {
        self.sim.scene.world.body(hero).health
    }

    /// One tick with explicit slot inputs, then the tally of the new events.
    pub fn step(&mut self, slots: [RawInput; 2]) {
        self.sim.step_slots(slots);
        self.read_events();
    }

    /// One tick of the fixture's own pilot, then the tally.
    pub fn step_scripted(&mut self) {
        self.sim.step_scripted();
        self.read_events();
    }

    fn read_events(&mut self) {
        let combat = &self.sim.combat;
        let total = combat.dropped_events + combat.events.len();
        let fresh = total.saturating_sub(self.seen).min(combat.events.len());
        self.seen = total;
        let events = &combat.events[combat.events.len() - fresh..];
        for event in events {
            match *event {
                CombatEvent::BossHurt { damage, .. } => self.tally.boss_damage += u32::from(damage),
                CombatEvent::HeroHurt {
                    source,
                    health_left,
                    hero,
                    ..
                } => {
                    let prev = self.health[hero.index()];
                    let lost = u32::from(prev.saturating_sub(health_left));
                    self.health[hero.index()] = health_left;
                    self.tally.party_loss += lost;
                    if source == crate::boss::BOSS_SOURCE_ID {
                        self.tally.hero_hits += 1;
                        if lost >= 2 {
                            self.tally.debris_hits += 1;
                        }
                    }
                }
                CombatEvent::BossCrash { tick, wall } => self.tally.crashes.push((tick, wall)),
                CombatEvent::EnemyDefeated { tick, target, .. }
                    if target == crate::boss::BOSS_ENEMY_ID =>
                {
                    self.tally.defeated_at.get_or_insert(tick);
                }
                CombatEvent::BossPhase { .. } => self.tally.forbidden += 1,
                CombatEvent::BossStrike {
                    mv: crate::boss::BossMove::SecondPound | crate::boss::BossMove::PhaseRoar,
                    ..
                } => {
                    self.tally.forbidden += 1;
                }
                CombatEvent::Restart { .. } => {
                    self.tally.restarts += 1;
                    // A restart returns the party to full health.
                    self.health = [
                        self.sim.scene.world.hero.health,
                        self.sim.scene.world.donut_body.health,
                    ];
                }
                _ => {}
            }
        }
        // A heal without an event (a restart's refill) never counts as a loss.
        for hero in [HeroId::Carl, HeroId::Donut] {
            let now = self.health_of(hero);
            if now > self.health[hero.index()] {
                self.health[hero.index()] = now;
            }
        }
    }

    /// Both heroes run right (two seats) until the gate closed, at most `limit`
    /// ticks. Returns whether it closed.
    pub fn walk_in(&mut self, limit: u64) -> bool {
        let go = RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        };
        for _ in 0..limit {
            if self.boss().gate_closed() {
                return true;
            }
            self.step([go, go]);
        }
        self.boss().gate_closed()
    }
}

const RELAUNCH_BASE: Fixture = boss_base(
    "f01_late_bos",
    "BOS-A: the Ball with 2 HP left under a late-Floor-1 loadout (the room-18 tests of the run)",
    0x0F01_1A7F,
    56,
    ball_hp(2),
    2400,
);

/// The Ball's room under the loadout's flags (the mastery pin, the `F01Late`
/// label), the Ball nearly dead, relaunched with `carry`, played by the pilot
/// until it is defeated (the run's room tests).
pub fn ball_relaunch(
    root: &Path,
    loadout: Loadout,
    carry: Option<&LedgerSnapshot>,
) -> Result<PartyFixtureSim, String> {
    static PARTY: PartyFixture = PartyFixture {
        base: &RELAUNCH_BASE,
        two_player: false,
        tuning: PROVISIONAL_TUNING,
        script: &[],
        pilot: Pilot::CarlBait,
        donut: DonutSetup::unlocked(8),
    };
    let (carl, donut) = loadout.flags();
    let kit = load_ball_arena(root)?;
    let scene = SceneSim::new_at(&kit, 360, RELAUNCH_BASE.carl_x)?;
    let mut combat = CombatState::new(&RELAUNCH_BASE, &kit, &scene)?
        .with_donut(DonutSetup::unlocked(8))
        .try_with_story(ball_story_setup(carl, donut))
        .map_err(|e| e.to_string())?;
    if let Some(snapshot) = carry {
        combat.carry_story(snapshot).map_err(|e| e.to_string())?;
    }
    let mut sim = PartyFixtureSim::from_parts(scene, combat, &PARTY);
    for _ in 0..RELAUNCH_BASE.ticks {
        sim.step_scripted();
        if sim.combat.boss.as_ref().is_some_and(Boss::released) {
            break;
        }
    }
    Ok(sim)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss::RackState;
    use std::path::Path;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    const IDLE: RawInput = RawInput::NONE;

    fn run(dir: f32) -> RawInput {
        RawInput {
            move_x: dir,
            ..RawInput::NONE
        }
    }

    fn jab(dir: f32) -> RawInput {
        RawInput {
            attack: true,
            move_x: dir,
            ..RawInput::NONE
        }
    }

    /// A two-seat room with the gate closed: the heroes ran in, then are placed
    /// (setup only), the ball is on `ball_x` facing `facing`, still on its first
    /// Neutral tick. The BOS-A staging: the ball on its BOS-A data (no Tusklings, no crash
    /// self-damage), so the fixture tests F4..F9 and F19 prove the siege and the bait apart from
    /// them.
    fn staged(ball_x: f32, facing: i8, carl: f32, donut: f32) -> Rig {
        staged_on(crate::boss::ball_bos_a(), ball_x, facing, carl, donut)
    }

    /// [`staged`] on the committed data (the BOSB rules).
    fn staged_b(ball_x: f32, facing: i8, carl: f32, donut: f32) -> Rig {
        staged_on(crate::boss::ball(), ball_x, facing, carl, donut)
    }

    fn staged_on(
        data: &'static crate::boss::BossData,
        ball_x: f32,
        facing: i8,
        carl: f32,
        donut: f32,
    ) -> Rig {
        let mut rig = Rig::new(root(), &BALL01_DUO).unwrap();
        let floor_y = rig.boss().floor_y();
        rig.sim.combat.boss = Some(Boss::new(data, floor_y));
        for hero in [HeroId::Carl, HeroId::Donut] {
            rig.place(hero, 150.0);
        }
        assert!(rig.walk_in(200), "the gate closes");
        rig.place(HeroId::Carl, carl);
        rig.place(HeroId::Donut, donut);
        let boss = rig.boss_mut();
        boss.x = ball_x;
        boss.facing = facing;
        rig
    }

    /// The tick-by-tick modes of the boss.
    fn mode_runs(modes: &[BossMode]) -> Vec<(BossMode, usize)> {
        let mut out: Vec<(BossMode, usize)> = Vec::new();
        for &m in modes {
            match out.last_mut() {
                Some((last, n)) if *last == m => *n += 1,
                _ => out.push((m, 1)),
            }
        }
        out
    }

    fn fnv(text: &str) -> u64 {
        text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
        })
    }

    // F1
    #[test]
    fn f1_the_data_validates_and_the_juicer_is_unchanged() {
        let ball = crate::boss::ball();
        assert_eq!(ball.id, "rolling_boss_f01");
        assert_eq!(ball.species, "rolling_brute");
        assert_eq!(ball.hp_max, 24);
        assert!(!ball.phase.queue_roar);
        assert!(ball.hud.emblem.is_empty(), "no Juicer emblem (BOS-AD31)");
        assert_eq!(ball.arena.hero_right_wall(), 608.0);
        assert_eq!(
            ball.siege.as_ref().unwrap().notice,
            crate::hud::LabelId::ItSniffsAtTheArch.text()
        );
        // The Juicer: the new fields are absent, the bytes are the pre-BOS ones.
        let juicer = crate::boss::builtin();
        assert!(juicer.siege.is_none() && juicer.arena.hero_right_wall_x.is_none());
        assert_eq!(juicer.hp_max, 36);
        assert_eq!(juicer.arena.hero_right_wall(), juicer.arena.right_wall_x);
        let text =
            std::fs::read_to_string(root().join("assets/data/bosses/brute_boss_f01.json")).unwrap();
        assert_eq!(
            fnv(&text),
            0xd3fe_e768_9e51_a614,
            "brute_boss_f01.json is byte-identical"
        );
        // A malformed siege (warn not under commit) is refused.
        let bad = std::fs::read_to_string(root().join("assets/data/bosses/ball_boss_f01.json"))
            .unwrap()
            .replace("\"warn_ticks\": 60", "\"warn_ticks\": 130");
        assert!(crate::boss::BossData::parse(&bad).is_err());
    }

    // F2 lives in `f01_run` and `f01_late` (the manifest, `next_room`, `MANIFEST_HASH`).

    // F3, F15: one full solo clear, every tick read.
    struct Clear {
        rig: Rig,
        modes: Vec<BossMode>,
        warn_matches_tell: bool,
        locked: bool,
        phase_1: bool,
        no_ripple: bool,
        no_forbidden_event: bool,
    }

    fn clear() -> Clear {
        let mut rig = Rig::new(root(), &BALL01_FULL_CLEAR_SOLO).unwrap();
        let mut modes = Vec::new();
        let (mut warn_matches_tell, mut locked, mut phase_1, mut no_ripple) =
            (true, true, true, true);
        let mut tell_lock: Option<(i8, Option<HeroId>)> = None;
        for _ in 0..7200u64 {
            rig.step_scripted();
            let boss = rig.boss();
            modes.push(boss.mode);
            warn_matches_tell &= boss.state().warning()
                == (boss.mode == BossMode::ChargeTell || boss.mode.is_tell());
            phase_1 &= boss.phase == 1;
            no_ripple &= boss.ripple_count() == 0;
            if boss.mode == BossMode::ChargeTell {
                let now = (boss.facing, boss.target);
                locked &= *tell_lock.get_or_insert(now) == now;
            } else {
                tell_lock = None;
            }
            if rig.tally.defeated_at.is_some() {
                break;
            }
        }
        let no_forbidden_event = rig.tally.forbidden == 0;
        Clear {
            rig,
            modes,
            warn_matches_tell,
            locked,
            phase_1,
            no_ripple,
            no_forbidden_event,
        }
    }

    #[test]
    fn f3_f11_f15_the_solo_clear() {
        let c = clear();
        // F3: every roll tell is exactly 36 ticks with a locked heading and target.
        let runs = mode_runs(&c.modes);
        let tells: Vec<usize> = runs
            .iter()
            .filter(|(m, _)| *m == BossMode::ChargeTell)
            .map(|(_, n)| *n)
            .collect();
        assert!(!tells.is_empty(), "the clear rolled at least once");
        assert!(tells.iter().all(|&n| n == 36), "{tells:?}");
        assert!(c.locked, "heading and target are locked during a tell");
        assert!(
            c.warn_matches_tell,
            "the warning flag is exactly the tell ticks"
        );
        // F11: defeat in 1200..6000 ticks, Carl and the bot alive (M1).
        let at = c.rig.tally.defeated_at.expect("the ball is defeated");
        println!(
            "BOS-F11 clear_ticks={at} party_loss={} hero_hits={} crashes={}",
            c.rig.tally.party_loss,
            c.rig.tally.hero_hits,
            c.rig.tally.crashes.len()
        );
        assert!((1200..=6000).contains(&at), "F11 clear at tick {at}");
        // The fight's identity (BOS-AD34): the roll is baited again and again, not once.
        assert!(
            c.rig.tally.crashes.len() >= 3,
            "F11: {} crashes",
            c.rig.tally.crashes.len()
        );
        assert!(c.rig.health_of(HeroId::Carl) >= 1 && c.rig.health_of(HeroId::Donut) >= 1);
        // F15: one phase, no roar, shockwave or double pound.
        assert!(c.phase_1 && c.no_ripple && c.no_forbidden_event);
        // The shove is also part of the fight (the Juicer's pound machine).
        assert!(runs.iter().any(|(m, _)| *m == BossMode::PoundTell));
    }

    /// Run `rig` for up to `ticks`, calling `policy` for the two slots; the modes
    /// seen are returned.
    fn drive(
        rig: &mut Rig,
        ticks: u64,
        mut policy: impl FnMut(&Rig) -> [RawInput; 2],
        mut until: impl FnMut(&Rig) -> bool,
    ) -> Vec<BossMode> {
        let mut modes = Vec::new();
        for _ in 0..ticks {
            let slots = policy(rig);
            rig.step(slots);
            modes.push(rig.boss().mode);
            if until(rig) {
                break;
            }
        }
        modes
    }

    /// Carl runs for the alcove of the roll's side from tell tick 0 (the roll
    /// aimed at him), Donut sits in the opposite alcove (unreachable).
    fn carl_runs(rig: &Rig) -> [RawInput; 2] {
        let b = rig.boss();
        let x = rig.x_of(HeroId::Carl);
        let dir = f32::from(b.run_dir());
        let safe = if dir < 0.0 { x <= 104.0 } else { x >= 536.0 };
        let moving = matches!(b.mode, BossMode::ChargeTell | BossMode::Charge) && !safe;
        [if moving { run(dir) } else { IDLE }, IDLE]
    }

    // F4, F6: both sides, 0 damage, the crash, Rebound 8, Stun 66.
    #[test]
    fn f4_f6_a_hero_who_runs_from_the_tell_takes_no_damage_and_the_crash_follows() {
        for (ball_x, facing, donut_x, wall) in [(448.0, -1i8, 580.0, -1i8), (192.0, 1, 60.0, 1)] {
            let mut rig = staged(ball_x, facing, 320.0, donut_x);
            let modes = drive(&mut rig, 400, carl_runs, |r| {
                r.boss().mode == BossMode::Neutral && !r.tally.crashes.is_empty()
            });
            assert_eq!(rig.tally.party_loss, 0, "side {wall}: no damage");
            assert_eq!(rig.tally.crashes.len(), 1);
            assert_eq!(rig.tally.crashes[0].1, wall);
            let runs = mode_runs(&modes);
            let len = |m: BossMode| runs.iter().find(|(x, _)| *x == m).map(|(_, n)| *n).unwrap();
            // The crash tick itself is a Rebound tick: 8 Rebound ticks, 66 Stun ticks.
            assert_eq!(len(BossMode::Rebound), 8);
            assert_eq!(len(BossMode::Stun), 66);
            assert_eq!(len(BossMode::ChargeTell), 36);
            // The margin: Carl was in the alcove before the ram front met the arch.
            let carl = rig.x_of(HeroId::Carl);
            assert!(
                if wall < 0 {
                    carl <= 110.0
                } else {
                    carl >= 530.0
                },
                "{carl}"
            );
        }
    }

    // F5
    #[test]
    fn f5_a_hero_who_stands_in_the_roll_takes_exactly_one_hit_once() {
        let mut rig = staged(448.0, -1, 320.0, 60.0);
        drive(
            &mut rig,
            400,
            |_| [IDLE, IDLE],
            |r| !r.tally.crashes.is_empty(),
        );
        assert_eq!(rig.tally.hero_hits, 1);
        assert_eq!(rig.tally.party_loss, 1);
        assert_eq!(rig.health_of(HeroId::Carl), 3);
    }

    // F17 (and F6's edge): the alcove edge, pivot 110 and 530, through a bait crash.
    #[test]
    fn f17_the_alcove_edge_is_never_touched_by_a_bait_crash() {
        for (ball_x, facing, edge, runner_dir) in
            [(448.0, -1i8, 110.0, -1.0f32), (192.0, 1, 530.0, 1.0)]
        {
            // The runner is the bait in the lane; the other hero stands exactly on the edge.
            let mut rig = staged(ball_x, facing, edge, 320.0);
            let modes = drive(
                &mut rig,
                400,
                |r| {
                    let b = r.boss();
                    let x = r.x_of(HeroId::Donut);
                    let safe = if runner_dir < 0.0 {
                        x <= 104.0
                    } else {
                        x >= 536.0
                    };
                    let moving = matches!(b.mode, BossMode::ChargeTell | BossMode::Charge) && !safe;
                    [IDLE, if moving { run(runner_dir) } else { IDLE }]
                },
                |r| !r.tally.crashes.is_empty() && r.boss().mode == BossMode::Stun,
            );
            assert!(modes.contains(&BossMode::Rebound));
            assert_eq!(rig.tally.party_loss, 0, "edge {edge}");
            assert!(
                (rig.x_of(HeroId::Carl) - edge).abs() < 0.01,
                "the edge hero never moved"
            );
        }
    }

    // F7: leave the alcove at Stun start and jab.
    #[test]
    fn f7_the_stun_gives_at_least_two_punish_hits() {
        let hits = f7_punish_hits(false);
        assert!(hits >= 2, "F7: {hits} hits in the stun");
    }

    // F7 with the BOSB rules (the Tusklings, the crash cost): the stun still gives its hits.
    #[test]
    fn f7b_the_stun_still_gives_at_least_two_punish_hits_with_the_tusklings() {
        let hits = f7_punish_hits(true);
        println!("BOSB-F7 punish_hits={hits}");
        assert!(hits >= 2, "F7b: {hits} hits in the stun");
    }

    fn f7_punish_hits(bosb: bool) -> u32 {
        let mut rig = if bosb {
            staged_b(448.0, -1, 100.0, 320.0)
        } else {
            staged(448.0, -1, 100.0, 320.0)
        };
        let mut stun_from = None;
        let mut carl_hits = 0u32;
        let mut before = 0;
        for _ in 0..500u64 {
            let b = rig.boss();
            let x = rig.x_of(HeroId::Carl);
            let mode = b.mode;
            let donut = {
                let dx = rig.x_of(HeroId::Donut);
                let moving = matches!(mode, BossMode::ChargeTell | BossMode::Charge) && dx > 104.0;
                if moving { run(-1.0) } else { IDLE }
            };
            let carl = if mode == BossMode::Stun {
                stun_from.get_or_insert(rig.sim.scene.world.tick);
                let body = b.hurtbox().unwrap();
                let gap = body.left - x;
                if gap > 18.0 {
                    run(1.0)
                } else if rig.sim.scene.world.tick.is_multiple_of(12) {
                    jab(1.0)
                } else {
                    IDLE
                }
            } else {
                IDLE
            };
            rig.step([carl, donut]);
            if stun_from.is_some() {
                carl_hits += rig.tally.boss_damage - before;
            }
            before = rig.tally.boss_damage;
            if stun_from.is_some() && rig.boss().mode != BossMode::Stun {
                break;
            }
        }
        println!("BOS-F7 bosb={bosb} punish_hits={carl_hits}");
        assert_eq!(rig.tally.party_loss, 0);
        carl_hits
    }

    // The clearing policies (BOSB-AD13) and the numbers of the clear (BOSB-AD3, AD4, AD11, AD12).
    #[test]
    fn the_three_clearing_policies_clear_the_ball_with_tusklings_and_stay_in_band() {
        for (fixture, ids) in [
            (&BALL01_FULL_CLEAR_SOLO, "carl_bait_punish"),
            (&BALL01_ENV_CLEAR_SOLO, "carl_environment"),
            (&BALL01_DUO_COOP, "duo_coop"),
        ] {
            let mut rig = Rig::new(root(), fixture).unwrap();
            for _ in 0..7200u64 {
                rig.step_scripted();
                if rig.tally.defeated_at.is_some() {
                    break;
                }
            }
            let at = rig
                .tally
                .defeated_at
                .unwrap_or_else(|| panic!("{ids} clears"));
            println!(
                "BOSB-CLEAR {ids} defeated_at={at} party_loss={} crashes={} restarts={}",
                rig.tally.party_loss,
                rig.tally.crashes.len(),
                rig.tally.restarts
            );
            assert_eq!(rig.tally.restarts, 0, "{ids}: nobody went down");
            assert!(
                rig.sim.combat.tusklings_spent() <= 5,
                "{ids}: the budget holds"
            );
            let facts = rig
                .sim
                .combat
                .events
                .iter()
                .filter(|e| matches!(e, CombatEvent::LedgerFact { .. }))
                .count();
            assert_eq!(facts, 1, "{ids}: the fact is written once");
            // Every Tuskling of the clear that fell was credited like any enemy.
            assert!(rig.boss().released());
        }
        // F11's recorded band (1200..6000) holds with the crash costing 3: BOSB-AD12 stays 3.
        let mut rig = Rig::new(root(), &BALL01_FULL_CLEAR_SOLO).unwrap();
        while rig.tally.defeated_at.is_none() {
            rig.step_scripted();
            assert!(rig.sim.scene.world.tick < 7000);
        }
        let at = rig.tally.defeated_at.unwrap();
        assert!(
            (1200..=6000).contains(&at),
            "F11 under the BOSB rules: {at}"
        );
    }

    /// Both heroes in the left alcove (Carl 80, Donut 60) from the gate close.
    fn hidden() -> Rig {
        staged(448.0, -1, 80.0, 60.0)
    }

    // F8: the siege.
    #[test]
    fn f8_the_siege_warns_at_60_commits_at_120_and_drops_debris_on_the_alcove_only() {
        let mut rig = hidden();
        let (mut warned, mut committed) = (None, None);
        for _ in 0..400u64 {
            rig.step([IDLE, IDLE]);
            let b = rig.boss();
            if warned.is_none() && b.siege_warning() {
                warned = Some(b.unreachable_ticks());
            }
            if committed.is_none() && b.mode == BossMode::ChargeTell {
                assert!(b.siege_roll(), "the roll is a siege roll");
                committed = Some(b.unreachable_ticks());
            }
            if !rig.tally.crashes.is_empty() {
                break;
            }
        }
        assert_eq!(warned, Some(60));
        assert_eq!(committed, Some(120));
        // Debris: both alcove heroes take 2, the counter and the flag are reset.
        assert_eq!(rig.tally.debris_hits, 2);
        assert_eq!(rig.health_of(HeroId::Carl), 2);
        assert_eq!(rig.health_of(HeroId::Donut), 2);
        assert_eq!(rig.boss().unreachable_ticks(), 0);
        assert!(!rig.boss().siege_roll());
        // Split: Carl left, Donut right: the ball turns to the nearer one (Donut,
        // 112 px from it) and only she takes the debris.
        let mut rig = staged(448.0, -1, 80.0, 560.0);
        drive(
            &mut rig,
            400,
            |_| [IDLE, IDLE],
            |r| !r.tally.crashes.is_empty(),
        );
        assert_eq!(rig.tally.crashes[0].1, 1, "the right arch");
        assert_eq!(rig.tally.debris_hits, 1);
        assert_eq!(rig.health_of(HeroId::Carl), 4);
        assert_eq!(rig.health_of(HeroId::Donut), 2);
    }

    // F8b: a hide directly after a left crash.
    #[test]
    fn f8b_the_siege_still_commits_after_a_crash_and_the_recentre() {
        // Donut baits a roll at the left arch and hides; Carl is already hidden.
        let mut rig = staged(448.0, -1, 80.0, 320.0);
        drive(
            &mut rig,
            500,
            |r| {
                let b = r.boss();
                let dx = r.x_of(HeroId::Donut);
                let moving =
                    matches!(b.mode, BossMode::ChargeTell | BossMode::Charge) && dx > 104.0;
                [IDLE, if moving { run(-1.0) } else { IDLE }]
            },
            |r| r.boss().mode == BossMode::Stun,
        );
        assert_eq!(rig.tally.party_loss, 0, "the bait crash is safe");
        // Everyone hides now. The ball walks to the centre, stops at the warning.
        let mut stopped = None;
        for _ in 0..600u64 {
            rig.step([IDLE, IDLE]);
            let b = rig.boss();
            if stopped.is_none() && b.siege_warning() {
                stopped = Some(b.x);
            }
            if rig.tally.crashes.len() == 2 {
                break;
            }
        }
        let x = stopped.expect("the warning came");
        println!("BOS-F8b warning_x={x}");
        // BOS-AD34: the ball walks back to the centre after the crash (3 px/tick, about 33
        // ticks) and then stops at the warning; before it, the walk stopped at about 276.
        assert!(
            (x - 320.0).abs() <= 2.0,
            "the ball stops at the centre: {x}"
        );
        assert_eq!(rig.tally.crashes.len(), 2, "the siege roll still crashed");
        assert!(rig.tally.debris_hits >= 1);
    }

    // F19 (BOS-AD34, closes BOS-Q12): after a crash the ball walks back to the centre,
    // so a hero in the arch pocket is baited again: the cycle repeats.
    #[test]
    fn f19_the_ball_recentres_after_a_crash_and_the_next_roll_follows() {
        let mut rig = staged(448.0, -1, 320.0, 60.0);
        let (mut stun_over, mut centred, mut stepped, mut second_tell) = (None, None, false, None);
        for tick in 0..900u64 {
            let b = rig.boss();
            let x = rig.x_of(HeroId::Carl);
            if stun_over.is_none() && !rig.tally.crashes.is_empty() && b.mode == BossMode::Neutral {
                stun_over = Some(tick);
            }
            if centred.is_none() && stun_over.is_some() && (b.x - 320.0).abs() < 0.01 {
                centred = Some(tick);
            }
            if stun_over.is_some() && b.mode == BossMode::ChargeTell && second_tell.is_none() {
                second_tell = Some(tick);
            }
            // Carl runs for the alcove from the first tell, then steps out once the ball
            // has walked back far enough, and waits in the pocket.
            let carl = if second_tell.is_none() && stun_over.is_none() {
                carl_runs(&rig)[0]
            } else if stun_over.is_some() && second_tell.is_none() && (stepped || b.x >= 300.0) {
                stepped = true;
                if x < 125.0 { run(1.0) } else { IDLE }
            } else {
                carl_runs(&rig)[0]
            };
            rig.step([carl, IDLE]);
            if rig.tally.crashes.len() == 2 {
                break;
            }
        }
        let (over, at) = (
            stun_over.expect("the first stun ended"),
            centred.expect("it recentred"),
        );
        println!(
            "BOS-F19 stun_over={over} centred_after={} second_tell={second_tell:?}",
            at - over
        );
        assert!(
            at - over <= 40,
            "back at the centre within 40 ticks: {}",
            at - over
        );
        assert!(second_tell.is_some(), "the next roll was baited");
        assert_eq!(rig.tally.crashes.len(), 2, "and it crashed");
        assert_eq!(
            rig.tally.party_loss, 0,
            "the pocket hero ran for the alcove in time"
        );
        // The Juicer's data has no recentre speed: it stays where it stood.
        assert!(crate::boss::builtin().siege.is_none());
    }

    // F9
    #[test]
    fn f9_a_hero_who_steps_out_after_the_warning_cancels_the_siege() {
        let mut rig = hidden();
        let mut stepped = false;
        for _ in 0..300u64 {
            let b = rig.boss();
            let carl = if b.siege_warning() || stepped {
                stepped = true;
                if rig.x_of(HeroId::Carl) < 125.0 {
                    run(1.0)
                } else {
                    IDLE
                }
            } else {
                IDLE
            };
            rig.step([carl, IDLE]);
            if stepped && rig.x_of(HeroId::Carl) > 111.0 {
                assert_eq!(
                    rig.boss().unreachable_ticks(),
                    0,
                    "counter 0 once reachable"
                );
                break;
            }
        }
        assert!(stepped);
        for _ in 0..100u64 {
            rig.step([IDLE, IDLE]);
            assert!(!rig.boss().siege_roll(), "no siege roll");
        }
        assert_eq!(rig.tally.debris_hits, 0);
    }

    // F10: the chain weight.
    #[test]
    fn f10_three_post_hits_drop_the_weight_on_the_stunned_ball() {
        let mut rig = staged(448.0, -1, POST_X, 320.0);
        let (mut hits, mut next_jab, mut dropped_at, mut pinned) = (0u32, 1u64, None, 0usize);
        let mut tick = 0u64;
        for _ in 0..600u64 {
            tick += 1;
            let b = rig.boss();
            // Donut is the bait: she runs for the left alcove once the roll is told.
            let dx = rig.x_of(HeroId::Donut);
            let donut = if matches!(b.mode, BossMode::ChargeTell | BossMode::Charge) && dx > 104.0 {
                run(-1.0)
            } else {
                IDLE
            };
            // Carl jabs the post: twice early, the third on the crash.
            let early = hits < 2 && tick >= next_jab;
            let third = hits == 2 && b.mode == BossMode::Rebound;
            let carl = if early || third { jab(1.0) } else { IDLE };
            if early || third {
                hits += 1;
                next_jab = tick + 20;
            }
            if matches!(b.racks[0].state, RackState::Falling { .. }) && dropped_at.is_none() {
                dropped_at = Some(tick);
            }
            rig.step([carl, donut]);
            if rig.boss().mode == BossMode::Pinned {
                pinned += 1;
            } else if pinned > 0 {
                break;
            }
        }
        println!(
            "BOS-F10 dropped_at={dropped_at:?} pinned={pinned} boss_hp_after={}",
            rig.boss().hp
        );
        assert!(dropped_at.is_some(), "three hits dropped the weight");
        // `pin_ticks` 90 counts the landing tick: 89 Pinned ticks follow it.
        assert!((89..=90).contains(&pinned), "pinned for 90 ticks: {pinned}");
        // F10 runs on the BOS-A staging (no crash self-damage): the weight deals 4. The BOSB
        // crash cost of 3 is asserted by `bosb_tests`.
        assert_eq!(rig.boss().hp, 20, "the weight deals 4");
        assert_eq!(rig.tally.party_loss, 0, "the alcove heroes are untouched");
        assert_eq!(rig.boss().racks[0].state, RackState::Landed, "one shot");
    }

    /// BOS-ART (BOS-ART-AD11): the evidence pilot `bos_weight` hits the post twice, jabs the third
    /// time on the crash and the weight pins the ball, with a solo Carl.
    #[test]
    fn the_weight_pilot_drops_the_weight_on_the_ball_with_a_solo_carl() {
        static PARTY: PartyFixture = PartyFixture {
            base: &CLEAR_SOLO_BASE,
            two_player: false,
            tuning: PROVISIONAL_TUNING,
            script: &[],
            pilot: Pilot::CarlWeight,
            donut: DonutSetup::NONE,
        };
        let mut rig = Rig::new(root(), &PARTY).unwrap();
        let (mut dropped, mut pinned, mut landed_at) = (None, 0u32, None);
        for tick in 1..=500u64 {
            rig.step_scripted();
            let boss = rig.boss();
            if matches!(boss.racks[0].state, RackState::Falling { .. }) && dropped.is_none() {
                dropped = Some(tick);
            }
            if boss.racks[0].state == RackState::Landed && landed_at.is_none() {
                landed_at = Some(tick);
            }
            pinned += u32::from(boss.mode == BossMode::Pinned);
        }
        println!(
            "BOS-ART weight pilot dropped_at={dropped:?} landed_at={landed_at:?} pinned={pinned}"
        );
        assert!(
            dropped.is_some() && landed_at.is_some(),
            "the weight dropped and landed"
        );
        assert!(pinned >= 80, "the weight pinned the ball: {pinned} ticks");
    }

    // F12
    #[test]
    fn f12_a_party_defeat_restarts_the_whole_encounter() {
        let mut rig = hidden();
        rig.sim.scene.world.hero.health = 2;
        rig.sim.scene.world.donut_body.health = 2;
        rig.sim.scene.world.sync_donut_for_tests();
        rig.health = [2, 2];
        rig.boss_mut().hp = 10;
        let mut events = Vec::new();
        let strut = rig.boss().strut_rect();
        rig.boss_mut().strike_strut(1, strut, 0, 1, &mut events);
        assert_eq!(rig.boss().racks[0].state, RackState::Idle { hits: 1 });
        let mut max_counter = 0;
        for _ in 0..900u64 {
            rig.step([IDLE, IDLE]);
            max_counter = max_counter.max(rig.boss().unreachable_ticks());
            if rig.tally.restarts > 0 {
                break;
            }
        }
        assert!(max_counter >= 60, "the siege ran before the defeat");
        assert_eq!(
            rig.tally.restarts, 1,
            "the debris defeated the party and it restarted"
        );
        let b = rig.boss();
        assert_eq!((b.hp, b.mode), (24, BossMode::Dormant));
        assert_eq!(b.racks[0].state, RackState::Idle { hits: 0 });
        assert_eq!((b.unreachable_ticks(), b.siege_roll()), (0, false));
        assert_eq!(
            (rig.health_of(HeroId::Carl), rig.health_of(HeroId::Donut)),
            (4, 4)
        );
        assert!(!b.gate_closed());
    }

    // F13, F14, F16: the exit and the floor end.
    #[test]
    fn f13_f14_the_exit_opens_on_a_real_defeat_and_grants_nothing() {
        let before = {
            let kit = load_ball_arena(root()).unwrap();
            let scene = SceneSim::new_at(&kit, 360, RELAUNCH_BASE.carl_x).unwrap();
            let (carl, donut) = Loadout::Story.flags();
            CombatState::new(&RELAUNCH_BASE, &kit, &scene)
                .unwrap()
                .with_donut(DonutSetup::unlocked(8))
                .try_with_story(ball_story_setup(carl, donut))
                .unwrap()
                .story_snapshot()
        };
        let mut sim = ball_relaunch(root(), Loadout::Story, None).unwrap();
        let boss = sim.combat.boss.as_ref().unwrap();
        assert!(boss.released() && !boss.alive(), "a real defeat");
        let wall = |s: &PartyFixtureSim| s.scene.world.solids.iter().any(|b| b.rect.left == 608.0);
        assert!(!wall(&sim), "the right wall is gone");
        for _ in 0..90 {
            sim.step_scripted();
        }
        for _ in 0..400 {
            if sim.scene.world.complete {
                break;
            }
            sim.step_scripted();
        }
        assert!(
            sim.scene.world.complete,
            "Carl walked out through the opened wall"
        );
        let exit = load_ball_arena(root()).unwrap().exit.unwrap();
        assert_eq!(exit.x, 612, "the exit lies behind the hero wall at 608");
        assert!(sim.scene.world.hero.x + crate::prototype::HERO_HALF_WIDTH > exit.x as f32);
        // F14: no grant, the Juicer's trigger is not armed and not fired.
        assert_eq!(
            sim.combat.story_snapshot(),
            before,
            "no receipt added or dropped"
        );
        assert!(
            !sim.combat
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::CapabilityGranted { .. })),
            "no grant"
        );
        assert_eq!(
            crate::story_unlock::TABLE_HASH,
            0xf41a_f63c_e01b_a84f,
            "TABLE_HASH is unchanged"
        );
    }

    /// The evidence pilot `bos_kick` reaches a natural kick into the ball (BOSB-AD8): a Tuskling
    /// is kicked into it once per launch and the ball is staggered, for 0 HP.
    #[test]
    fn the_kick_pilot_kicks_a_tuskling_into_the_ball_and_the_ball_staggers() {
        let mut rig = Rig::new(root(), &BALL01_FULL_CLEAR_SOLO).unwrap();
        let mut first = None;
        for t in 0..700u64 {
            let boss = rig.boss().clone();
            let carl = kick_pilot(t + 1, &rig.sim.combat, &rig.sim.scene, &boss);
            let hp = boss.hp;
            rig.step([carl, IDLE]);
            for e in &rig.sim.combat.events {
                if let CombatEvent::Tuskling {
                    tick,
                    act: crate::tuskling::TuskAct::KickContact { launch, staggered },
                    ..
                } = e
                    && first.is_none()
                {
                    first = Some((*tick, *launch, *staggered, hp, rig.boss().hp));
                }
            }
            if first.is_some() {
                break;
            }
        }
        let (tick, launch, staggered, hp_before, hp_after) =
            first.expect("a kick reached the ball");
        println!("BOSB-KICK tick={tick} launch={launch} staggered={staggered}");
        assert!(staggered, "the ball was staggered");
        assert_eq!(hp_before, hp_after, "0 HP damage");
    }
}
