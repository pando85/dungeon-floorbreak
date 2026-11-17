//! Combat fixtures for the Floor 1 scene (COMBAT-INT-01): named, deterministic
//! scenarios that put Carl and goblins on the corridor kit's clean ground and
//! drive them through the shared [`crate::combat`] domain.
//!
//! Bevy-free. A fixture is data: a seed, Carl's start column, goblin spawns and
//! an optional scripted input trace. [`CombatState`] layers combat on a
//! [`SceneSim`] (corridor physics, camera, `step_hero`) without changing it, so
//! the normal story run and its `FLOOR1_SCENE_STATE` hashes are untouched.
//! Fixtures never run in the normal route.
//!
//! Tick numbering: a tick is the 1-based number the scene reports after the
//! step (`SceneSim.world.tick`). Script spans and every event use it, so "press
//! on tick 122" is the step that produces `tick=122`, and the first active tick
//! of a jab pressed then is 125.
//!
//! The only randomness-related state is the fixture `seed`. Nothing draws from
//! it yet (combat is fully deterministic without it); it is part of the state
//! hash so later knockback variance can use it without changing the contract.

use crate::combat::{
    Attacker, CARL_BUFFER_TICKS, CARL_JAB, CARL_PUSH_KICK, CombatEvent, ENEMY_HALF_HEIGHT, Enemy,
    EnemyAi, resolve_strike,
};
use crate::corridor::SceneSim;
use crate::env_kit::EnvKit;
use crate::input::{InputFrame, RawInput};
use crate::prototype::HERO_HALF_HEIGHT;

/// Ticks Carl is stunned after taking a hit (the melee art's `hurt` flinch).
pub const HERO_HURT_TICKS: u8 = 18;
/// Ticks Carl lies defeated before the fixture restarts by itself.
pub const RESTART_DELAY_TICKS: u16 = 45;

/// One goblin spawn: its centre column and behaviour. It stands on the same
/// walking line as Carl (checked on load).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnemySpawn {
    pub x: i32,
    pub ai: EnemyAi,
}

/// Scripted input for the ticks `from..to` (half-open, 1-based ticks).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hold {
    pub from: u64,
    pub to: u64,
    pub input: RawInput,
}

/// A single held attack button over `from..to`.
const fn attack(from: u64, to: u64) -> Hold {
    Hold {
        from,
        to,
        input: RawInput {
            move_x: 0.0,
            jump: false,
            attack: true,
            ability: false,
            pause: false,
        },
    }
}

/// A named, deterministic combat setup.
#[derive(Debug, PartialEq)]
pub struct Fixture {
    pub name: &'static str,
    pub summary: &'static str,
    pub seed: u64,
    /// Carl's start column on the corridor's clean ground.
    pub carl_x: i32,
    pub enemies: &'static [EnemySpawn],
    pub script: &'static [Hold],
    /// Default run length of the headless replay and the CI hash.
    pub ticks: u64,
}

impl Fixture {
    /// The scripted input of `tick`: every hold that covers it, merged.
    pub fn input(&self, tick: u64) -> RawInput {
        let mut raw = RawInput::default();
        for hold in self
            .script
            .iter()
            .filter(|h| (h.from..h.to).contains(&tick))
        {
            raw.attack |= hold.input.attack;
            raw.ability |= hold.input.ability;
            raw.jump |= hold.input.jump;
            if hold.input.move_x != 0.0 {
                raw.move_x = hold.input.move_x;
            }
        }
        raw
    }

    pub fn by_name(name: &str) -> Option<&'static Fixture> {
        FIXTURES.iter().copied().find(|f| f.name == name)
    }

    pub fn names() -> String {
        FIXTURES
            .iter()
            .map(|f| f.name)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// CP-01 baseline: Carl and one chasing goblin on clean ground. Two jabs (the
/// second lands the moment the first move ends) defeat it.
pub const CP01_SINGLE_GOBLIN: Fixture = Fixture {
    name: "cp01_single_goblin",
    summary: "Carl and one chasing goblin; two jabs defeat it",
    seed: 0x0C01_0001,
    carl_x: 400,
    enemies: &[EnemySpawn {
        x: 560,
        ai: EnemyAi::Chase,
    }],
    script: &[attack(122, 123), attack(136, 137)],
    ticks: 200,
};

/// CP-01 two sequential hits on a stationary goblin dummy. The first jab is
/// pressed and HELD for 50 ticks (one press edge: one hit, no repeat); the
/// second is a fresh press and defeats it.
pub const CP01_TWO_HITS: Fixture = Fixture {
    name: "cp01_two_hits",
    summary: "stationary goblin; a held jab hits once, a second press defeats it",
    seed: 0x0C01_0002,
    carl_x: 400,
    enemies: &[EnemySpawn {
        x: 428,
        ai: EnemyAi::Passive,
    }],
    script: &[attack(10, 60), attack(70, 71)],
    ticks: 120,
};

/// CP-02 placeholder setup: two chasing goblins close together. The scripted
/// jabs only exercise the shared combat; the knockback-into-goblin behaviour
/// is the next PR.
pub const CP02_TWO_GOBLINS: Fixture = Fixture {
    name: "cp02_two_goblins",
    summary: "two close chasing goblins (setup for the knockback work)",
    seed: 0x0C02_0001,
    carl_x: 400,
    enemies: &[
        EnemySpawn {
            x: 520,
            ai: EnemyAi::Chase,
        },
        EnemySpawn {
            x: 552,
            ai: EnemyAi::Chase,
        },
    ],
    script: &[
        attack(83, 84),
        attack(97, 98),
        attack(114, 115),
        attack(128, 129),
    ],
    ticks: 260,
};

pub const FIXTURES: [&Fixture; 3] = [&CP01_SINGLE_GOBLIN, &CP01_TWO_HITS, &CP02_TWO_GOBLINS];

// ------------------------------------------------------------------ the sim --

/// Combat layered on a [`SceneSim`]: goblins, Carl's hurt/defeat/restart state
/// and the event log. The hero's move clock, health and invulnerability live
/// on the scene's own `Hero` (shared with the greybox).
#[derive(Debug, Clone)]
pub struct CombatState {
    pub fixture: &'static Fixture,
    pub enemies: Vec<Enemy>,
    /// Press-edge detector for the combat buttons (the scene keeps its own for
    /// jump).
    input: InputFrame,
    /// Ticks of hurt flinch left; Carl cannot act or move.
    pub hurt_ticks: u8,
    pub hero_defeated: bool,
    /// Ticks since Carl was defeated.
    pub defeated_ticks: u16,
    pub deaths: u32,
    /// Every event since the process started, restarts included.
    pub events: Vec<CombatEvent>,
    event_hash: u64,
    cursor: usize,
    enemy_foot_y: f32,
}

impl CombatState {
    /// Place the fixture's goblins on Carl's walking line. Fails when a goblin
    /// column is not on the same line (the fixture promises clean ground).
    pub fn new(fixture: &'static Fixture, kit: &EnvKit, scene: &SceneSim) -> Result<Self, String> {
        let line = kit
            .walking_line_at(fixture.carl_x)
            .ok_or_else(|| format!("{}: no walking line under Carl", fixture.name))?;
        for spawn in fixture.enemies {
            if kit.walking_line_at(spawn.x) != Some(line) {
                return Err(format!(
                    "{}: goblin column {} is not on Carl's walking line",
                    fixture.name, spawn.x
                ));
            }
        }
        let mut state = Self {
            fixture,
            enemies: Vec::new(),
            input: InputFrame::default(),
            hurt_ticks: 0,
            hero_defeated: false,
            defeated_ticks: 0,
            deaths: 0,
            events: Vec::new(),
            event_hash: EVENT_HASH_SEED,
            cursor: 0,
            enemy_foot_y: scene.world.hero_foot_y(),
        };
        state.reset_attempt();
        Ok(state)
    }

    fn reset_attempt(&mut self) {
        self.enemies = self
            .fixture
            .enemies
            .iter()
            .map(|spawn| {
                Enemy::with_ai(
                    spawn.x as f32,
                    self.enemy_foot_y + ENEMY_HALF_HEIGHT,
                    spawn.ai,
                )
            })
            .collect();
        self.input = InputFrame::default();
        self.hurt_ticks = 0;
        self.hero_defeated = false;
        self.defeated_ticks = 0;
        self.event_hash = EVENT_HASH_SEED;
    }

    /// Fast restart of the fixture: Carl, the goblins and the tick counter go
    /// back to the start. `deaths` and the event log (which records the
    /// restart) survive.
    pub fn restart(&mut self, scene: &mut SceneSim) {
        scene.reset();
        self.reset_attempt();
        // Logged but not folded into the attempt's event hash, so a replayed
        // attempt after a restart hashes like a fresh one.
        self.events.push(CombatEvent::Restart {
            tick: scene.world.tick,
            deaths: self.deaths,
        });
    }

    fn log(&mut self, event: CombatEvent) {
        let mut hash = self.event_hash;
        event.mix_into(&mut |value| hash = mix(hash, value));
        self.event_hash = hash;
        self.events.push(event);
    }

    /// Events recorded since the last call (for live logging).
    pub fn take_new_events(&mut self) -> &[CombatEvent] {
        let from = self.cursor;
        self.cursor = self.events.len();
        &self.events[from..]
    }

    /// One fixed tick. `raw` is the player's (or the script's) input.
    pub fn step(&mut self, scene: &mut SceneSim, raw: RawInput) {
        let tick = scene.world.tick + 1;
        self.input.advance(raw);
        self.hurt_ticks = self.hurt_ticks.saturating_sub(1);
        scene.world.hero.invulnerable_ticks = scene.world.hero.invulnerable_ticks.saturating_sub(1);

        let can_act = !self.hero_defeated && self.hurt_ticks == 0;
        // Attack is the jab; the secondary action is the push kick (there are
        // no bombs before the Guild). Both only start from a press edge, so a
        // held button never repeats.
        let request = if self.input.attack_pressed {
            Some(&CARL_JAB)
        } else if self.input.ability_pressed {
            Some(&CARL_PUSH_KICK)
        } else {
            None
        };
        // Pressing a direction together with the attack turns Carl first.
        if can_act && request.is_some() && !scene.world.hero.melee.busy() && raw.move_x != 0.0 {
            scene.world.hero.facing = if raw.move_x > 0.0 { 1 } else { -1 };
        }
        let move_tick = scene
            .world
            .hero
            .melee
            .step(request, CARL_BUFFER_TICKS, can_act);
        if let Some(move_tick) = move_tick {
            if move_tick.started {
                self.log(CombatEvent::MoveStarted {
                    tick,
                    move_id: move_tick.spec.id,
                    instance: move_tick.instance,
                });
            }
            let hero = &scene.world.hero;
            let attacker = Attacker {
                x: hero.x,
                y: hero.y,
                half_height: HERO_HALF_HEIGHT,
                facing: hero.facing,
            };
            let mut hits = Vec::new();
            resolve_strike(
                &mut scene.world.hero.melee,
                &move_tick,
                attacker,
                &mut self.enemies,
                tick,
                &mut hits,
            );
            for event in hits {
                self.log(event);
            }
        }

        // Committed on the ground: the feet stay planted and jumping is off.
        let committed = scene.world.hero.melee.busy() && scene.world.hero.grounded;
        let gated = if !can_act {
            RawInput::default()
        } else if committed {
            RawInput {
                move_x: 0.0,
                jump: false,
                ..raw
            }
        } else {
            raw
        };
        scene.step(gated);

        for index in 0..self.enemies.len() {
            let hero = &mut scene.world.hero;
            self.enemies[index].step(hero.x, &scene.world.solids);
            if self.hero_defeated {
                continue;
            }
            let hurt = self.enemies[index].contact(
                hero.x,
                hero.y,
                &mut hero.health,
                &mut hero.invulnerable_ticks,
            );
            if hurt {
                hero.melee.cancel();
                self.hurt_ticks = HERO_HURT_TICKS;
                let health_left = hero.health;
                self.log(CombatEvent::HeroHurt {
                    tick,
                    source: index as u32,
                    health_left,
                });
                if health_left == 0 {
                    self.hero_defeated = true;
                    self.deaths += 1;
                    self.log(CombatEvent::HeroDefeated { tick });
                }
            }
        }

        if self.hero_defeated {
            self.defeated_ticks = self.defeated_ticks.saturating_add(1);
            if self.defeated_ticks >= RESTART_DELAY_TICKS {
                self.restart(scene);
            }
        }
    }

    /// FNV-1a hash of the scene and every combat value; the same inputs give
    /// the same hash on every machine.
    pub fn state_hash(&self, scene: &SceneSim) -> u64 {
        let mut hash = EVENT_HASH_SEED;
        let mut put = |value: u64| hash = mix(hash, value);
        put(scene.state_hash());
        put(self.fixture.seed);
        let hero = &scene.world.hero;
        put(u64::from(hero.health));
        put(u64::from(hero.invulnerable_ticks));
        put(u64::from(self.hurt_ticks));
        put(u64::from(self.hero_defeated));
        put(u64::from(self.defeated_ticks));
        put(u64::from(self.deaths));
        hero.melee.mix_into(&mut put);
        for enemy in &self.enemies {
            enemy.mix_into(&mut put);
        }
        put(self.event_hash);
        hash
    }

    /// The line CI compares between the headless replay, the render run and
    /// the committed expectations.
    pub fn state_line(&self, scene: &SceneSim) -> String {
        let enemies: Vec<String> = self
            .enemies
            .iter()
            .enumerate()
            .map(|(i, e)| format!("e{i}={:.3}/{}", e.x, e.health))
            .collect();
        format!(
            "COMBAT_STATE fixture={} seed={:#x} tick={} hero_hp={} deaths={} enemies={} hash={:016x}",
            self.fixture.name,
            self.fixture.seed,
            scene.world.tick,
            scene.world.hero.health,
            self.deaths,
            enemies.join(","),
            self.state_hash(scene)
        )
    }
}

const EVENT_HASH_SEED: u64 = 0xcbf2_9ce4_8422_2325;

fn mix(mut hash: u64, value: u64) -> u64 {
    for byte in value.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// A fixture's scene and combat state together: what the headless replay and
/// the tests drive. The render binary keeps the two in separate resources.
#[derive(Debug, Clone)]
pub struct FixtureSim {
    pub scene: SceneSim,
    pub combat: CombatState,
}

impl FixtureSim {
    pub fn new(kit: &EnvKit, view_h: i32, fixture: &'static Fixture) -> Result<Self, String> {
        let scene = SceneSim::new_at(kit, view_h, fixture.carl_x)?;
        let combat = CombatState::new(fixture, kit, &scene)?;
        Ok(Self { scene, combat })
    }

    pub fn step(&mut self, raw: RawInput) {
        self.combat.step(&mut self.scene, raw);
    }

    /// One tick of the fixture's own script.
    pub fn step_scripted(&mut self) {
        let raw = self.combat.fixture.input(self.scene.world.tick + 1);
        self.step(raw);
    }

    pub fn run_scripted(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.step_scripted();
        }
    }

    pub fn restart(&mut self) {
        self.combat.restart(&mut self.scene);
    }

    pub fn state_line(&self) -> String {
        self.combat.state_line(&self.scene)
    }

    pub fn state_hash(&self) -> u64 {
        self.combat.state_hash(&self.scene)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{CARL_JAB, CARL_PUSH_KICK, ENEMY_HEALTH, HERO_HEALTH, MoveId};
    use std::path::PathBuf;

    const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

    /// Committed expectations: the final `COMBAT_STATE` hash of each fixture
    /// at its default length. CI compares the headless replay and the real
    /// render run against the same values (`docs/evidence/combat-int-01`).
    const EXPECTED_HASHES: [(&str, u64); 3] = [
        ("cp01_single_goblin", 0xc695_2df1_e99c_7c0a),
        ("cp01_two_hits", 0x382b_caed_f5dd_e492),
        ("cp02_two_goblins", 0xf4e1_1cc7_1da4_38a1),
    ];

    fn kit() -> EnvKit {
        EnvKit::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")), LAYOUT).unwrap()
    }

    fn sim_of(fixture: &'static Fixture) -> FixtureSim {
        FixtureSim::new(&kit(), 360, fixture).unwrap()
    }

    fn events_of(sim: &FixtureSim) -> Vec<CombatEvent> {
        sim.combat.events.clone()
    }

    fn hits(events: &[CombatEvent]) -> Vec<(u64, u32)> {
        events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::Hit { tick, target, .. } => Some((*tick, *target)),
                _ => None,
            })
            .collect()
    }

    fn starts(events: &[CombatEvent]) -> Vec<(u64, MoveId)> {
        events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::MoveStarted { tick, move_id, .. } => Some((*tick, *move_id)),
                _ => None,
            })
            .collect()
    }

    fn idle() -> RawInput {
        RawInput::default()
    }

    fn attack_input() -> RawInput {
        RawInput {
            attack: true,
            ..RawInput::default()
        }
    }

    /// A stationary goblin at column `$x` (Carl at column 400) and no script.
    macro_rules! dummy_fixture {
        ($name:ident, $x:expr) => {
            static $name: Fixture = Fixture {
                name: "test_dummy",
                summary: "test",
                seed: 7,
                carl_x: 400,
                enemies: &[EnemySpawn {
                    x: $x,
                    ai: EnemyAi::Passive,
                }],
                script: &[],
                ticks: 100,
            };
        };
    }
    dummy_fixture!(DUMMY_AHEAD, 428);
    dummy_fixture!(DUMMY_BEHIND, 372);

    #[test]
    fn the_jab_hits_on_the_first_active_tick_not_the_press_tick() {
        let mut sim = sim_of(&DUMMY_AHEAD);
        for _ in 0..9 {
            sim.step(idle());
        }
        sim.step(attack_input()); // tick 10: the press
        let after_press = events_of(&sim);
        assert_eq!(starts(&after_press), vec![(10, MoveId::Jab)]);
        assert!(hits(&after_press).is_empty(), "no damage on the press tick");
        for _ in 0..2 {
            sim.step(idle()); // ticks 11, 12: startup
        }
        assert!(hits(&events_of(&sim)).is_empty());
        assert_eq!(sim.combat.enemies[0].health, ENEMY_HEALTH);
        sim.step(idle()); // tick 13: first active tick
        assert_eq!(hits(&events_of(&sim)), vec![(13, 0)]);
        assert_eq!(sim.combat.enemies[0].health, ENEMY_HEALTH - 1);
        sim.step(idle()); // tick 14: second active tick, no second hit
        assert_eq!(hits(&events_of(&sim)).len(), 1);
    }

    #[test]
    fn recovery_and_cooldown_gate_the_next_jab() {
        // A press while the move runs and earlier than the buffer window is
        // forgotten (tick 12); one inside the last six ticks (tick 20) starts
        // the next jab the tick the first ends (10 + 14 = tick 24).
        let mut sim = sim_of(&DUMMY_AHEAD);
        for tick in 1..=60 {
            let press = matches!(tick, 10 | 12 | 20);
            sim.step(if press { attack_input() } else { idle() });
        }
        let s = starts(&events_of(&sim));
        assert_eq!(s, vec![(10, MoveId::Jab), (24, MoveId::Jab)]);
        assert_eq!(s[1].0 - s[0].0, u64::from(CARL_JAB.total()));
    }

    #[test]
    fn a_held_button_or_a_lingering_overlap_never_repeats_damage() {
        let mut sim = sim_of(&CP01_TWO_HITS);
        sim.run_scripted(60);
        let events = events_of(&sim);
        // Attack is held from tick 10 to 59: one press edge, one move, one hit.
        assert_eq!(starts(&events), vec![(10, MoveId::Jab)]);
        assert_eq!(hits(&events), vec![(13, 0)]);
        assert_eq!(sim.combat.enemies[0].health, ENEMY_HEALTH - 1);
        // A new press is a new move and a new hit.
        sim.run_scripted(60);
        assert_eq!(hits(&events_of(&sim)), vec![(13, 0), (73, 0)]);
    }

    #[test]
    fn only_targets_in_front_are_hit_and_pressing_a_direction_turns_carl() {
        let mut sim = sim_of(&DUMMY_BEHIND);
        for _ in 0..20 {
            sim.step(attack_input());
            sim.step(idle());
        }
        assert!(hits(&events_of(&sim)).is_empty(), "behind Carl: never hit");
        assert_eq!(sim.combat.enemies[0].health, ENEMY_HEALTH);
        // Attack together with left turns Carl first; the goblin is in front.
        let mut sim = sim_of(&DUMMY_BEHIND);
        sim.step(RawInput {
            attack: true,
            move_x: -1.0,
            ..idle()
        });
        assert_eq!(sim.scene.world.hero.facing, -1);
        for _ in 0..3 {
            sim.step(idle());
        }
        assert_eq!(hits(&events_of(&sim)), vec![(4, 0)]);
        // The feet stay planted while the move is committed.
        assert_eq!(sim.scene.world.hero.x, 400.0);
    }

    #[test]
    fn defeat_after_two_hits_and_the_corpse_no_longer_collides_or_takes_hits() {
        let mut sim = sim_of(&CP01_TWO_HITS);
        sim.run_scripted(CP01_TWO_HITS.ticks);
        let enemy = &sim.combat.enemies[0];
        assert_eq!(enemy.health, 0);
        assert!(!enemy.alive());
        assert!(events_of(&sim).iter().any(|e| matches!(
            e,
            CombatEvent::EnemyDefeated {
                tick: 73,
                target: 0
            }
        )));
        // Jab the corpse: no hit. Walk through it: no contact damage.
        let hits_before = hits(&events_of(&sim)).len();
        for _ in 0..3 {
            sim.step(attack_input());
            for _ in 0..16 {
                sim.step(idle());
            }
        }
        for _ in 0..120 {
            sim.step(RawInput {
                move_x: 1.0,
                ..idle()
            });
        }
        assert!(
            sim.scene.world.hero.x > 440.0,
            "Carl walked past the corpse"
        );
        let events = events_of(&sim);
        assert_eq!(hits(&events).len(), hits_before);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, CombatEvent::HeroHurt { .. }))
        );
        assert_eq!(sim.scene.world.hero.health, HERO_HEALTH);
    }

    #[test]
    fn push_kick_uses_the_secondary_action_and_hits_on_its_13th_tick() {
        let mut sim = sim_of(&DUMMY_AHEAD);
        for _ in 0..13 {
            sim.step(RawInput {
                ability: true, // held: still a single press edge
                ..idle()
            });
        }
        assert!(hits(&events_of(&sim)).is_empty());
        sim.step(idle()); // tick 14 = press tick 1 + startup 13
        let events = events_of(&sim);
        assert_eq!(starts(&events), vec![(1, MoveId::PushKick)]);
        assert_eq!(hits(&events), vec![(14, 0)]);
        assert_eq!(CARL_PUSH_KICK.startup, 13);
        assert_eq!(CARL_PUSH_KICK.total(), 36);
    }

    #[test]
    fn carl_flinches_cannot_act_while_hurt_dies_and_the_fixture_restarts() {
        // A chasing goblin on an idle Carl: contact damage every 55 ticks.
        static CHASER: Fixture = Fixture {
            name: "test_chaser",
            summary: "test",
            seed: 9,
            carl_x: 400,
            enemies: &[EnemySpawn {
                x: 440,
                ai: EnemyAi::Chase,
            }],
            script: &[],
            ticks: 400,
        };
        let mut sim = sim_of(&CHASER);
        let mut checked_flinch = false;
        for _ in 0..400 {
            sim.step(idle());
            if !checked_flinch && sim.scene.world.hero.health < HERO_HEALTH {
                checked_flinch = true;
                assert_eq!(sim.combat.hurt_ticks, HERO_HURT_TICKS);
                sim.step(attack_input()); // dropped while hurt
                assert!(starts(&events_of(&sim)).is_empty());
                assert!(!sim.scene.world.hero.melee.busy());
            }
        }
        assert!(checked_flinch);
        let events = events_of(&sim);
        let hurts = events
            .iter()
            .filter(|e| matches!(e, CombatEvent::HeroHurt { .. }))
            .count();
        assert!(hurts >= usize::from(HERO_HEALTH), "four hits defeat Carl");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CombatEvent::HeroDefeated { .. }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CombatEvent::Restart { .. }))
        );
        assert_eq!(sim.combat.deaths, 1);
        // Restarted: Carl is alive again at the fixture start.
        assert!(sim.scene.world.hero.health > 0);
        assert!(!sim.combat.hero_defeated);
        assert!(sim.scene.world.tick < 400);
    }

    #[test]
    fn restart_is_deterministic() {
        let mut sim = sim_of(&CP01_SINGLE_GOBLIN);
        sim.run_scripted(CP01_SINGLE_GOBLIN.ticks);
        let first = sim.state_hash();
        sim.restart();
        sim.run_scripted(CP01_SINGLE_GOBLIN.ticks);
        assert_eq!(sim.state_hash(), first, "same script after a restart");
        let restarts = events_of(&sim)
            .iter()
            .filter(|e| matches!(e, CombatEvent::Restart { .. }))
            .count();
        assert_eq!(restarts, 1);
        // A restart in the middle of the fight also lands on the same state.
        let mut mid = sim_of(&CP01_SINGLE_GOBLIN);
        mid.run_scripted(130);
        mid.restart();
        mid.run_scripted(CP01_SINGLE_GOBLIN.ticks);
        assert_eq!(mid.state_hash(), first);
    }

    #[test]
    fn the_same_input_trace_gives_the_same_event_log_and_hash() {
        let mut hashes = Vec::new();
        for fixture in FIXTURES {
            let mut a = sim_of(fixture);
            let mut b = sim_of(fixture);
            a.run_scripted(fixture.ticks);
            b.run_scripted(fixture.ticks);
            assert_eq!(events_of(&a), events_of(&b), "{}", fixture.name);
            assert_eq!(a.state_hash(), b.state_hash(), "{}", fixture.name);
            assert_eq!(a.state_line(), b.state_line());
            hashes.push(a.state_hash());
        }
        assert!(hashes[0] != hashes[1] && hashes[1] != hashes[2] && hashes[0] != hashes[2]);
    }

    #[test]
    fn fixtures_match_the_committed_expected_hashes() {
        assert_eq!(EXPECTED_HASHES.len(), FIXTURES.len());
        for (name, expected) in EXPECTED_HASHES {
            let fixture = Fixture::by_name(name).unwrap();
            let mut sim = sim_of(fixture);
            sim.run_scripted(fixture.ticks);
            assert_eq!(sim.state_hash(), expected, "{name}: {}", sim.state_line());
        }
    }

    #[test]
    fn cp01_single_goblin_is_defeated_by_two_jabs_without_hurting_carl() {
        let mut sim = sim_of(&CP01_SINGLE_GOBLIN);
        sim.run_scripted(CP01_SINGLE_GOBLIN.ticks);
        let events = events_of(&sim);
        assert_eq!(hits(&events), vec![(125, 0), (139, 0)]);
        assert_eq!(sim.combat.enemies[0].health, 0);
        assert_eq!(sim.scene.world.hero.health, HERO_HEALTH);
        assert_eq!(sim.combat.deaths, 0);
    }

    #[test]
    fn cp02_has_two_goblins_close_together_and_defeats_both() {
        let mut sim = sim_of(&CP02_TWO_GOBLINS);
        assert_eq!(sim.combat.enemies.len(), 2);
        assert!(sim.combat.enemies[1].x - sim.combat.enemies[0].x <= 40.0);
        sim.run_scripted(CP02_TWO_GOBLINS.ticks);
        assert!(sim.combat.enemies.iter().all(|e| !e.alive()));
    }

    #[test]
    fn fixtures_stand_on_clean_ground() {
        let kit = kit();
        for fixture in FIXTURES {
            let line = kit.walking_line_at(fixture.carl_x);
            assert!(line.is_some());
            let far = fixture
                .enemies
                .iter()
                .map(|e| e.x)
                .max()
                .unwrap_or(fixture.carl_x);
            for x in (fixture.carl_x - 60)..(far + 60) {
                assert_eq!(kit.walking_line_at(x), line, "{} column {x}", fixture.name);
            }
        }
    }

    #[test]
    fn the_normal_corridor_hashes_are_unchanged() {
        // Recorded before COMBAT-INT-01 (`FLOOR1_SCENE_STATE` of walk_right).
        for (ticks, hash) in [
            (250, 0x9318_68d6_afa5_6498_u64),
            (613, 0x82b8_f87f_a644_de48),
        ] {
            let mut sim = SceneSim::new(&kit(), 360).unwrap();
            sim.run_scenario(crate::corridor::Scenario::WalkRight, ticks);
            assert_eq!(sim.state_hash(), hash, "walk_right {ticks} ticks");
        }
    }

    #[test]
    fn bombs_stay_out_of_the_scene() {
        // Explosives are WORK-021 only: the scene never touches the greybox world
        // that owns them.
        let needle = ["Bo", "mb"].concat();
        for source in [
            include_str!("combat_scene.rs"),
            include_str!("bin/floor01_scene.rs"),
        ] {
            assert!(!source.contains(&needle));
            assert!(!source.contains(&["prototype", "::World"].concat()));
        }
        assert_eq!(CARL_JAB.id.name(), "jab");
    }
}
