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
    Attacker, CARL_BUFFER_TICKS, CombatEvent, DefeatCause, ENEMY_HALF_HEIGHT, ENEMY_HALF_WIDTH,
    EnemyAi, HERO_INVULNERABLE_TICKS, MoveId, Phase, RootStep, carl_move, hit_box, resolve_strike,
    solid_at,
};
use crate::corridor::SceneSim;
use crate::env_kit::EnvKit;
use crate::feel::{
    CAMERA_KICK_TICKS, GROUND_FRICTION, HERO_DEFEAT_HIT_STOP, HERO_HURT_HIT_STOP,
    HERO_KNOCKDOWN_VX, HERO_KNOCKDOWN_VY, IMPACT_FLASH_TICKS, LETHAL_CAMERA_KICK_BONUS,
    MAX_ATTACKERS, TELL_SPACING,
};
pub use crate::feel::{HERO_HURT_TICKS, RESTART_DELAY_TICKS};
use crate::goblin::{Goblin, GoblinMode};
use crate::input::{InputFrame, RawInput};
use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Rect, TICK_SECONDS};
use crate::shore::{PLAYER_ONE, Shore};

/// Centre-to-centre distance at which Carl's body touches a goblin's (the two
/// half widths): the haymaker's step stops here.
const BODY_CONTACT: f32 = HERO_HALF_WIDTH + crate::combat::ENEMY_HALF_WIDTH;

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

/// A single held secondary (push kick) button over `from..to`.
const fn kick(from: u64, to: u64) -> Hold {
    Hold {
        from,
        to,
        input: RawInput {
            move_x: 0.0,
            jump: false,
            attack: false,
            physical: true,
            ability: false,
            pause: false,
        },
    }
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
            physical: false,
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
    /// CP-03: the ENV-02 rotten shore's strike column, in pixels right of
    /// Carl's start column (`None`: no shore). The shore rules are authored
    /// relative to the strike column ([`crate::shore`]).
    pub shore_dx: Option<i32>,
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
            raw.physical |= hold.input.physical;
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

/// CP-01: Carl and one chasing goblin on clean ground. The same inputs as the
/// COMBAT-INT-01 baseline: a jab pressed on 122, the second jab pressed on 136
/// (hit-stop, stagger, knockback, a launched defeat and a corpse now).
pub const CP01_SINGLE_GOBLIN: Fixture = Fixture {
    name: "cp01_single_goblin",
    summary: "Carl and one chasing goblin; two jabs defeat it",
    seed: 0x0C01_0001,
    carl_x: 400,
    enemies: &[EnemySpawn {
        x: 560,
        ai: EnemyAi::Chase,
    }],
    shore_dx: None,
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
    shore_dx: None,
    script: &[attack(10, 60), attack(70, 71)],
    ticks: 140,
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
    shore_dx: None,
    script: &[
        attack(83, 84),
        attack(97, 98),
        attack(164, 165),
        attack(170, 171),
    ],
    ticks: 260,
};

/// CP-01: Carl idles and the goblin walks up, tells, swings and hits him; then
/// it goes around again. Shows the hurt flinch and the invulnerability.
pub const CP01_GOBLIN_ATTACKS: Fixture = Fixture {
    name: "cp01_goblin_attacks",
    summary: "Carl idles; the goblin tells, swings and hits him",
    seed: 0x0C01_0003,
    carl_x: 400,
    enemies: &[EnemySpawn {
        x: 560,
        ai: EnemyAi::Chase,
    }],
    shore_dx: None,
    script: &[],
    ticks: 300,
};

/// CP-01: Carl jabs the goblin during its tell and the swing never happens.
pub const CP01_INTERRUPT: Fixture = Fixture {
    name: "cp01_interrupt",
    summary: "a jab during the tell cancels the swing",
    seed: 0x0C01_0004,
    carl_x: 400,
    enemies: &[EnemySpawn {
        x: 560,
        ai: EnemyAi::Chase,
    }],
    shore_dx: None,
    script: &[attack(137, 138)],
    ticks: 200,
};

/// CP-01: jab, then the haymaker inside the jab's string window, defeats the
/// goblin.
pub const CP01_HAYMAKER_CHAIN: Fixture = Fixture {
    name: "cp01_haymaker_chain",
    summary: "jab then haymaker kills the goblin",
    seed: 0x0C01_0005,
    carl_x: 400,
    enemies: &[EnemySpawn {
        x: 560,
        ai: EnemyAi::Chase,
    }],
    shore_dx: None,
    script: &[attack(137, 138), attack(141, 142)],
    ticks: 230,
};

/// CP-01: Carl idles against two goblins until he is defeated; the fixture
/// restarts fast and the same script plays again, to the same outcome (defeated
/// on tick 400, restart on elapsed 449, defeated again on elapsed 849).
pub const CP01_RETRY: Fixture = Fixture {
    name: "cp01_retry",
    summary: "Carl is defeated, restarts fast and meets the same outcome",
    seed: 0x0C01_0006,
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
    shore_dx: None,
    script: &[],
    ticks: 850,
};

/// ART-INT-02 / D-02: the push kick (PhysicalAttack, Triangle/Y) against one
/// chasing goblin. The first kick lands inside the goblin's tell: it is
/// launched (8-tick hit-stop, flight, lies down, gets up) and walks back, and
/// the second kick, pressed as it arrives, defeats it. Input data only.
/// **Intentional change (PR A):** before the kick launch the first kick only
/// staggered and a second kick on tick 170 defeated it; the committed hashes
/// and traces were regenerated.
pub const CP01_KICK: Fixture = Fixture {
    name: "cp01_kick",
    summary: "a push kick launches the goblin; a second kick defeats it",
    seed: 0x0C01_0007,
    carl_x: 400,
    enemies: &[EnemySpawn {
        x: 560,
        ai: EnemyAi::Chase,
    }],
    shore_dx: None,
    script: &[kick(122, 123), kick(247, 248)],
    ticks: 340,
};

/// D-03 / CP-02: Carl kicks a stationary goblin (Triangle/Y, pressed on tick
/// 20) into a second stationary goblin 40 px behind it. Contact is
/// deterministic, once per launch, no damage; the kicked goblin lands and gets
/// up. Stationary on purpose so the contact does not depend on chase timing.
pub const CP02_KICK_CONTACT: Fixture = Fixture {
    name: "cp02_kick_contact",
    summary: "a kicked living goblin interrupts a second one (zero damage)",
    seed: 0x0C02_0002,
    carl_x: 400,
    enemies: &[
        EnemySpawn {
            x: 432,
            ai: EnemyAi::Passive,
        },
        EnemySpawn {
            x: 472,
            ai: EnemyAi::Passive,
        },
    ],
    shore_dx: None,
    script: &[kick(20, 21)],
    ticks: 140,
};

/// D-04 / CP-03: three separate jabs (ticks 20, 60 and 100) hit the rotten
/// shore's strike zone 19 px ahead of Carl; the block falls 6 ticks after the
/// break and lands on a stationary goblin standing in the hazard footprint,
/// which is defeated once with the credit going to Carl. The fixture runs on
/// the corridor's clean ground, not on the opening route.
pub const CP03_SHORE_KILL: Fixture = Fixture {
    name: "cp03_shore_kill",
    summary: "three jabs break the shore; the block defeats the goblin under it",
    seed: 0x0C03_0001,
    carl_x: 400,
    enemies: &[EnemySpawn {
        x: 484,
        ai: EnemyAi::Passive,
    }],
    shore_dx: Some(19),
    script: &[attack(20, 21), attack(60, 61), attack(100, 101)],
    ticks: 200,
};

/// D-04 negative case: the same three jabs, a goblin standing outside the
/// footprint. The block lands and nothing is defeated.
pub const CP03_SHORE_MISS: Fixture = Fixture {
    name: "cp03_shore_miss",
    summary: "three jabs break the shore; the footprint is empty, nobody dies",
    seed: 0x0C03_0002,
    carl_x: 400,
    enemies: &[EnemySpawn {
        x: 560,
        ai: EnemyAi::Passive,
    }],
    shore_dx: Some(19),
    script: &[attack(20, 21), attack(60, 61), attack(100, 101)],
    ticks: 200,
};

pub const FIXTURES: [&Fixture; 11] = [
    &CP01_SINGLE_GOBLIN,
    &CP01_TWO_HITS,
    &CP02_TWO_GOBLINS,
    &CP01_GOBLIN_ATTACKS,
    &CP01_INTERRUPT,
    &CP01_HAYMAKER_CHAIN,
    &CP01_RETRY,
    &CP01_KICK,
    &CP02_KICK_CONTACT,
    &CP03_SHORE_KILL,
    &CP03_SHORE_MISS,
];

// ------------------------------------------------------------------ the sim --

/// Most events kept in [`CombatState::events`]; the oldest are dropped past it
/// (a long manual session must not grow the log without bound). Headless
/// fixture replays stay far below it.
pub const EVENT_LOG_CAP: usize = 512;

/// Presentation cache: the last confirmed hit, for the camera kick. Written by
/// the sim, never read by it and not part of the state hash.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Confirm {
    tick: u64,
    kick_px: u8,
    /// +1 / -1: the way Carl hit.
    direction: i8,
}

/// Combat layered on a [`SceneSim`]: goblins, Carl's hurt/defeat/restart state
/// and the event log. The hero's move clock, health and invulnerability live
/// on the scene's own `Hero` (shared with the greybox).
#[derive(Debug, Clone)]
pub struct CombatState {
    pub fixture: &'static Fixture,
    pub enemies: Vec<Goblin>,
    /// CP-03: the rotten shore and its block (fixtures with `shore_dx`).
    pub shore: Option<Shore>,
    /// CP-02: (launch id, target) pairs that already had their contact.
    contacts: Vec<(u32, u32)>,
    /// Press-edge detector and held-button state of the combat buttons (the
    /// scene keeps its own for jump). The held state is part of the hash.
    input: InputFrame,
    /// Ticks of hurt flinch left; Carl cannot act or move.
    pub hurt_ticks: u8,
    /// Ticks Carl stays frozen after a goblin's blow landed (hit-stop).
    pub hero_hit_stop: u8,
    pub hero_defeated: bool,
    /// Carl's knockdown drift in px/s (after the defeating blow).
    pub hero_vx: f32,
    /// Ticks since Carl was defeated.
    pub defeated_ticks: u16,
    pub deaths: u32,
    /// The tick the last goblin tell started in this attempt (tell spacing).
    pub last_tell_start: Option<u64>,
    /// Ticks stepped over every attempt, restarts included (attempt-local ticks
    /// start again at 1 after a restart; this does not).
    pub elapsed: u64,
    /// The newest events (at most [`EVENT_LOG_CAP`]), restarts included.
    pub events: Vec<CombatEvent>,
    /// Events dropped from the front of `events` by the cap.
    pub dropped_events: usize,
    event_hash: u64,
    cursor: usize,
    enemy_foot_y: f32,
    x_max: f32,
    // Presentation cache (not hashed, never read by the sim).
    confirm: Option<Confirm>,
    flash_tick: Vec<Option<u64>>,
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
            shore: None,
            contacts: Vec::new(),
            input: InputFrame::default(),
            hurt_ticks: 0,
            hero_hit_stop: 0,
            hero_defeated: false,
            hero_vx: 0.0,
            defeated_ticks: 0,
            deaths: 0,
            last_tell_start: None,
            elapsed: 0,
            events: Vec::new(),
            dropped_events: 0,
            event_hash: EVENT_HASH_SEED,
            cursor: 0,
            enemy_foot_y: scene.world.hero_foot_y(),
            x_max: kit.width as f32,
            confirm: None,
            flash_tick: Vec::new(),
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
                Goblin::new(
                    spawn.x as f32,
                    self.enemy_foot_y + ENEMY_HALF_HEIGHT,
                    spawn.ai,
                )
            })
            .collect();
        self.shore = self
            .fixture
            .shore_dx
            .map(|dx| Shore::new((self.fixture.carl_x + dx) as f32, self.enemy_foot_y));
        self.contacts.clear();
        self.input = InputFrame::default();
        self.hurt_ticks = 0;
        self.hero_hit_stop = 0;
        self.hero_defeated = false;
        self.hero_vx = 0.0;
        self.defeated_ticks = 0;
        self.last_tell_start = None;
        self.event_hash = EVENT_HASH_SEED;
        self.confirm = None;
        self.flash_tick = vec![None; self.enemies.len()];
    }

    /// Fast restart of the fixture: Carl, the goblins and the tick counter go
    /// back to the start. `deaths` and the event log survive. The `Restart`
    /// event carries the tick the finished attempt ended on (the triggering
    /// tick); the attempt that follows counts its ticks from 1 again.
    pub fn restart(&mut self, scene: &mut SceneSim) {
        // Logged but not folded into the attempt's event hash, so a replayed
        // attempt after a restart hashes like a fresh one.
        self.push_event(CombatEvent::Restart {
            tick: scene.world.tick,
            deaths: self.deaths,
        });
        scene.reset();
        self.reset_attempt();
    }

    fn push_event(&mut self, event: CombatEvent) {
        self.events.push(event);
        if self.events.len() > EVENT_LOG_CAP {
            let excess = self.events.len() - EVENT_LOG_CAP;
            self.events.drain(..excess);
            self.cursor = self.cursor.saturating_sub(excess);
            self.dropped_events += excess;
        }
    }

    fn log(&mut self, event: CombatEvent) {
        let mut hash = self.event_hash;
        event.mix_into(&mut |value| hash = mix(hash, value));
        self.event_hash = hash;
        self.push_event(event);
    }

    /// Events recorded since the last call (for live logging).
    pub fn take_new_events(&mut self) -> &[CombatEvent] {
        let from = self.cursor.min(self.events.len());
        self.cursor = self.events.len();
        &self.events[from..]
    }

    /// One fixed tick. `raw` is the player's (or the script's) input.
    ///
    /// Order within a tick (the same every tick): Carl's move clock and strike
    /// on the goblins (Carl wins a same-tick trade), the scene step (Carl's
    /// physics, Donut, the camera), then every goblin in index order (attack
    /// tokens, tell, swing, and the blow on Carl).
    pub fn step(&mut self, scene: &mut SceneSim, raw: RawInput) {
        let tick = scene.world.tick + 1;
        self.elapsed += 1;
        self.input.advance(raw);
        // Hit-stop freezes Carl's timers: his flinch, his invulnerability and
        // (through the move clock) his move.
        let hero_frozen = self.hero_hit_stop > 0;
        if hero_frozen {
            self.hero_hit_stop -= 1;
        } else {
            self.hurt_ticks = self.hurt_ticks.saturating_sub(1);
            scene.world.hero.invulnerable_ticks =
                scene.world.hero.invulnerable_ticks.saturating_sub(1);
        }
        let melee_frozen =
            scene.world.hero.melee.busy() && scene.world.hero.melee.hit_stop_left() > 0;

        let can_act = !self.hero_defeated && self.hurt_ticks == 0 && !hero_frozen;
        // Attack is the jab (a second press inside its window is the
        // haymaker); PhysicalAttack (Triangle/Y) is the push kick. The
        // EquippedSpecial (Circle/B) has no effect before its story unlock
        // (there are no explosives before the Guild). Both only start from a
        // press edge, so a held button never repeats.
        let request = if self.input.attack_pressed {
            carl_move(MoveId::Jab)
        } else if self.input.physical_pressed {
            carl_move(MoveId::PushKick)
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
            if !move_tick.frozen
                && let Some(step) = move_tick.spec.root_step
                && (step.from_tick..=step.to_tick).contains(&move_tick.tick)
            {
                self.root_step(scene, step);
            }
            let hero = &scene.world.hero;
            let attacker = Attacker {
                x: hero.x,
                y: hero.y,
                half_height: HERO_HALF_HEIGHT,
                facing: hero.facing,
            };
            let attacking: Vec<bool> = self.enemies.iter().map(Goblin::token_held).collect();
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
                if let CombatEvent::Hit {
                    move_id,
                    instance,
                    target,
                    health_left,
                    ..
                } = event
                {
                    // A kick that launched a living goblin: stamp the move
                    // instance as the launch id (one contact per launch).
                    let goblin = &mut self.enemies[target as usize];
                    if goblin.mode == GoblinMode::Launched && goblin.launch.is_none() {
                        goblin.launch = Some(instance);
                    }
                    self.note_confirm(tick, move_id, target as usize, health_left == 0, attacker);
                }
                self.log(event);
            }
            for (index, was_attacking) in attacking.into_iter().enumerate() {
                if was_attacking && self.enemies[index].mode != GoblinMode::Attack {
                    self.log(CombatEvent::GoblinInterrupted {
                        tick,
                        goblin: index as u32,
                    });
                }
            }
            // CP-03: an active melee box over the shore's strike zone. Only a
            // live (not frozen) active tick counts, once per move instance.
            if move_tick.phase == Phase::Active
                && !move_tick.frozen
                && let Some(shore) = self.shore.as_mut()
                && let Some(boxed) = hit_box(&move_tick.spec.shape, &attacker)
                && let Some(strike) = shore.strike(move_tick.instance, boxed, PLAYER_ONE)
            {
                self.log(CombatEvent::ShoreHit {
                    tick,
                    instance: move_tick.instance,
                    hits: strike.hits,
                });
                if strike.broke {
                    self.log(CombatEvent::ShoreBroken {
                        tick,
                        player: PLAYER_ONE,
                    });
                }
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
        if self.hero_defeated && !hero_frozen {
            self.drift_hero(scene);
        }
        if hero_frozen || melee_frozen {
            // Frozen in hit-stop: only the tick counter advances. Carl, the
            // camera and Donut's trail stand still (no motion to restore).
            scene.hold_tick(gated);
        } else {
            scene.step(gated);
        }

        self.step_goblins(scene, tick);
        self.resolve_contacts(tick);
        self.step_shore(tick);

        if self.hero_defeated {
            self.defeated_ticks = self.defeated_ticks.saturating_add(1);
            if self.defeated_ticks >= RESTART_DELAY_TICKS {
                self.restart(scene);
            }
        }
    }

    /// Every goblin in index order: the attack token, the tell and swing, and
    /// the blow on Carl.
    fn step_goblins(&mut self, scene: &mut SceneSim, tick: u64) {
        let hero_x = scene.world.hero.x;
        // Attack token: at most MAX_ATTACKERS goblins in a tell or a swing; the
        // free slots go to the goblins that could attack, nearest to Carl
        // first, ties to the lower index.
        let active = self.enemies.iter().filter(|g| g.token_held()).count();
        let slots = MAX_ATTACKERS.saturating_sub(active);
        let mut ranked: Vec<(f32, usize)> = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, g)| g.wants_attack())
            .map(|(index, g)| ((g.x - hero_x).abs(), index))
            .collect();
        ranked.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        // Starvation note: an approved goblin that cannot get within the trigger
        // distance (a wall or a ledge between it and Carl) keeps its slot and
        // starves the goblins behind it. Fixtures stand on clean ground, so this
        // cannot happen yet; revisit with real geometry (the opening scene).
        let approved: Vec<usize> = ranked.iter().take(slots).map(|r| r.1).collect();
        for index in 0..self.enemies.len() {
            let may_start = self
                .last_tell_start
                .is_none_or(|start| tick - start >= TELL_SPACING);
            let step = self.enemies[index].step(
                hero_x,
                !self.hero_defeated,
                &scene.world.solids,
                self.x_max - ENEMY_HALF_WIDTH,
                approved.contains(&index),
                may_start,
            );
            if step.started_tell {
                self.last_tell_start = Some(tick);
                self.log(CombatEvent::GoblinTell {
                    tick,
                    goblin: index as u32,
                });
            }
            if step.swing_began {
                self.log(CombatEvent::GoblinSwing {
                    tick,
                    goblin: index as u32,
                });
            }
            if step.impact {
                self.swing_on_hero(scene, index, tick);
            }
        }
    }

    /// CP-02: every launched living goblin (index order) touching another
    /// living goblin (index order) for the first time in this launch. The flier
    /// shows `knockback_contact` and slows, the other goblin is interrupted and
    /// pushed. **Zero damage.** Walls stop a flier in `Goblin::integrate`, so a
    /// goblin behind a wall is never reached.
    fn resolve_contacts(&mut self, tick: u64) {
        for source in 0..self.enemies.len() {
            let flier = &self.enemies[source];
            let Some(launch) = flier.launch else {
                continue;
            };
            if flier.mode != GoblinMode::Launched || !flier.alive() {
                continue;
            }
            for target in 0..self.enemies.len() {
                if target == source
                    || self.contacts.contains(&(launch, target as u32))
                    || !self.enemies[target].can_be_contacted()
                    || !self.enemies[source]
                        .body()
                        .overlaps(self.enemies[target].body())
                {
                    continue;
                }
                self.contacts.push((launch, target as u32));
                let push: i8 = if self.enemies[target].x >= self.enemies[source].x {
                    1
                } else {
                    -1
                };
                let was_attacking = self.enemies[target].token_held();
                self.enemies[source].contact_as_flier();
                self.enemies[target].contact_as_target(push);
                self.log(CombatEvent::GoblinContact {
                    tick,
                    launch,
                    source: source as u32,
                    target: target as u32,
                });
                if was_attacking {
                    self.log(CombatEvent::GoblinInterrupted {
                        tick,
                        goblin: target as u32,
                    });
                }
                if let Some(slot) = self.flash_tick.get_mut(target) {
                    *slot = Some(tick);
                }
            }
        }
    }

    /// CP-03: one tick of the shore's block. On the landing tick the hazard
    /// footprint is evaluated once: every living goblin whose body overlaps it
    /// is defeated (index order), credited to the player who broke the shore.
    /// Carl takes no hit from the block yet (a tuning decision, disclosed).
    fn step_shore(&mut self, tick: u64) {
        let Some(shore) = self.shore.as_mut() else {
            return;
        };
        if !shore.step().landed {
            return;
        }
        let footprint = shore.footprint();
        let player = shore.breaker.unwrap_or(PLAYER_ONE);
        self.log(CombatEvent::ShoreLanded { tick });
        for index in 0..self.enemies.len() {
            if self.enemies[index].alive()
                && self.enemies[index].body().overlaps(footprint)
                && self.enemies[index].crush()
            {
                self.log(CombatEvent::EnemyDefeated {
                    tick,
                    target: index as u32,
                    cause: DefeatCause::ShoreBlock { player },
                });
                if let Some(slot) = self.flash_tick.get_mut(index) {
                    *slot = Some(tick);
                }
            }
        }
    }

    /// An active swing: it hurts Carl only if its box (in front of the goblin)
    /// overlaps his body, he is not invulnerable and this attack has not hit.
    fn swing_on_hero(&mut self, scene: &mut SceneSim, index: usize, tick: u64) {
        let goblin = &mut self.enemies[index];
        let hero = &mut scene.world.hero;
        let body = Rect {
            left: hero.x - HERO_HALF_WIDTH,
            bottom: hero.y - HERO_HALF_HEIGHT,
            width: HERO_HALF_WIDTH * 2.0,
            height: HERO_HALF_HEIGHT * 2.0,
        };
        if self.hero_defeated
            || hero.invulnerable_ticks > 0
            || goblin.swing_hit
            || !goblin.swing_box().overlaps(body)
        {
            return;
        }
        goblin.swing_hit = true;
        hero.health = hero.health.saturating_sub(1);
        hero.invulnerable_ticks = HERO_INVULNERABLE_TICKS;
        hero.melee.cancel();
        // The hurt and knockdown art assume the blow came from the front.
        hero.facing = if goblin.x >= hero.x { 1 } else { -1 };
        let away = if hero.x >= goblin.x { 1.0 } else { -1.0 };
        let health_left = hero.health;
        self.hurt_ticks = HERO_HURT_TICKS;
        self.hero_hit_stop = self.hero_hit_stop.max(HERO_HURT_HIT_STOP);
        goblin.hit_stop = goblin.hit_stop.max(HERO_HURT_HIT_STOP);
        self.log(CombatEvent::HeroHurt {
            tick,
            source: index as u32,
            health_left,
        });
        if health_left == 0 {
            self.hero_defeated = true;
            self.deaths += 1;
            // The heavier knockdown: longer freeze, then launched away.
            self.hero_hit_stop = self.hero_hit_stop.max(HERO_DEFEAT_HIT_STOP);
            self.enemies[index].hit_stop = self.enemies[index].hit_stop.max(HERO_DEFEAT_HIT_STOP);
            self.hero_vx = away * HERO_KNOCKDOWN_VX;
            let hero = &mut scene.world.hero;
            hero.vy = HERO_KNOCKDOWN_VY;
            hero.grounded = false;
            self.log(CombatEvent::HeroDefeated { tick });
        }
    }

    /// One tick of a move's forward step: stopped by walls, the layout edge and
    /// body contact (centre distance under `BODY_CONTACT` with an alive goblin
    /// the step would approach).
    fn root_step(&self, scene: &mut SceneSim, step: RootStep) {
        let hero = &mut scene.world.hero;
        if !hero.grounded {
            return;
        }
        let wanted = hero.x + f32::from(hero.facing) * step.px_per_tick;
        let candidate = wanted.clamp(HERO_HALF_WIDTH, self.x_max - HERO_HALF_WIDTH);
        let touching = self.enemies.iter().filter(|g| g.alive()).any(|g| {
            let after = (g.x - candidate).abs();
            after < BODY_CONTACT && after < (g.x - hero.x).abs()
        });
        let body = Rect {
            left: candidate - HERO_HALF_WIDTH,
            bottom: hero.y - HERO_HALF_HEIGHT,
            width: HERO_HALF_WIDTH * 2.0,
            height: HERO_HALF_HEIGHT * 2.0,
        };
        if !touching && !solid_at(&scene.world.solids, body) {
            hero.x = candidate;
        }
    }

    /// Carl's knockdown drift after the defeating blow: stopped by walls and
    /// the layout edge, slowed by friction once he lies on the floor.
    fn drift_hero(&mut self, scene: &mut SceneSim) {
        if self.hero_vx == 0.0 {
            return;
        }
        let hero = &mut scene.world.hero;
        let wanted = hero.x + self.hero_vx * TICK_SECONDS;
        let candidate = wanted.clamp(HERO_HALF_WIDTH, self.x_max - HERO_HALF_WIDTH);
        let body = Rect {
            left: candidate - HERO_HALF_WIDTH,
            bottom: hero.y - HERO_HALF_HEIGHT,
            width: HERO_HALF_WIDTH * 2.0,
            height: HERO_HALF_HEIGHT * 2.0,
        };
        let blocked = solid_at(&scene.world.solids, body);
        if !blocked {
            hero.x = candidate;
        }
        if blocked || candidate != wanted {
            self.hero_vx = 0.0;
        } else if hero.grounded {
            let drag = GROUND_FRICTION * TICK_SECONDS;
            self.hero_vx = if self.hero_vx.abs() <= drag {
                0.0
            } else {
                self.hero_vx - drag * self.hero_vx.signum()
            };
        }
    }

    /// Remember a confirmed hit for the presentation (flash and camera kick).
    fn note_confirm(
        &mut self,
        tick: u64,
        move_id: MoveId,
        target: usize,
        lethal: bool,
        attacker: Attacker,
    ) {
        let base = carl_move(move_id).map_or(0, |spec| spec.reaction.camera_kick_px);
        let bonus = if lethal { LETHAL_CAMERA_KICK_BONUS } else { 0 };
        self.confirm = Some(Confirm {
            tick,
            kick_px: base + bonus,
            direction: attacker.facing,
        });
        if let Some(slot) = self.flash_tick.get_mut(target) {
            *slot = Some(tick);
        }
    }

    /// Camera kick in virtual pixels at scene tick `tick`: the signed offset to
    /// add to the view's left edge. It is **opposite the hit direction**, so the
    /// scene jolts the way the goblin flies (a hit to the right moves the view
    /// left, which moves everything on screen to the right) and never against
    /// the knockback. Presentation only: a pure function of the last confirming
    /// hit and the tick; the sim never reads it.
    pub fn camera_kick(&self, tick: u64) -> i32 {
        let Some(confirm) = self.confirm else {
            return 0;
        };
        let age = tick.saturating_sub(confirm.tick);
        if age >= u64::from(CAMERA_KICK_TICKS) {
            return 0;
        }
        let left = (u64::from(CAMERA_KICK_TICKS) - age) as f32;
        let px = (f32::from(confirm.kick_px) * left / f32::from(CAMERA_KICK_TICKS)).round();
        -(px as i32) * i32::from(confirm.direction)
    }

    /// True while goblin `index` shows the impact flash (presentation only).
    pub fn flash(&self, index: usize, tick: u64) -> bool {
        self.flash_tick
            .get(index)
            .copied()
            .flatten()
            .is_some_and(|hit| tick.saturating_sub(hit) < u64::from(IMPACT_FLASH_TICKS))
    }

    /// FNV-1a hash of the scene and every combat value; the same inputs give
    /// the same hash on every machine. Includes the held-button state.
    pub fn state_hash(&self, scene: &SceneSim) -> u64 {
        let mut hash = EVENT_HASH_SEED;
        let mut put = |value: u64| hash = mix(hash, value);
        put(scene.state_hash());
        put(self.fixture.seed);
        let hero = &scene.world.hero;
        put(u64::from(hero.health));
        put(u64::from(hero.invulnerable_ticks));
        put(u64::from(self.hurt_ticks));
        put(u64::from(self.hero_hit_stop));
        put(u64::from(self.hero_defeated));
        put(u64::from(self.hero_vx.to_bits()));
        put(u64::from(self.defeated_ticks));
        put(u64::from(self.deaths));
        put(self.last_tell_start.map_or(0, |tick| tick + 1));
        put(self.elapsed);
        let held = &self.input.held;
        put(u64::from(held.move_x.to_bits()));
        put(u64::from(held.jump));
        put(u64::from(held.attack));
        put(u64::from(held.ability));
        // Mixed only while held, so fixtures that never kick keep their hashes.
        if held.physical {
            put(0x5048_5953);
        }
        put(u64::from(held.pause));
        hero.melee.mix_into(&mut put);
        for goblin in &self.enemies {
            goblin.mix_into(&mut put);
        }
        if let Some(shore) = &self.shore {
            shore.mix_into(&mut put);
        }
        put(self.event_hash);
        hash
    }

    /// One per-tick row for the evidence diagrams (`--combat-trace`), all
    /// `key=value` tokens: Carl's move and tick (`!` while frozen by hit-stop),
    /// position, health, flinch, hit-stop, invulnerability and the camera kick,
    /// then each goblin's mode, attack tick, position, hit-stop and health.
    pub fn trace_line(&self, scene: &SceneSim) -> String {
        let hero = &scene.world.hero;
        let carl = match hero.melee.current() {
            Some(m) => format!(
                "{}:{}{}",
                m.spec.id.name(),
                m.tick,
                if m.frozen { "!" } else { "" }
            ),
            None => "-".to_owned(),
        };
        let goblins: Vec<String> = self
            .enemies
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let phase = g
                    .attack_phase()
                    .map_or_else(String::new, |p| format!("/{p:?}"));
                format!(
                    "g{i}={:?}{phase} g{i}_t={} g{i}_x={:.2} g{i}_y={:.2} g{i}_hs={} g{i}_hp={}",
                    g.mode,
                    g.attack_tick,
                    g.x,
                    g.y - ENEMY_HALF_HEIGHT,
                    g.hit_stop,
                    g.health
                )
            })
            .collect();
        format!(
            "COMBAT_TRACE tick={} elapsed={} carl_move={carl} carl_x={:.2} carl_y={:.2} carl_hp={} hurt={} carl_stop={} invuln={} kick={} cam_left={} {}",
            scene.world.tick,
            self.elapsed,
            hero.x,
            hero.y,
            hero.health,
            self.hurt_ticks,
            self.hero_hit_stop,
            hero.invulnerable_ticks,
            self.camera_kick(scene.world.tick),
            scene.camera.left(),
            goblins.join(" ")
        ) + &self.shore_token()
    }

    /// ` shore=<hits>/<prop>/<block>` while the fixture has a shore, else empty.
    fn shore_token(&self) -> String {
        self.shore.as_ref().map_or_else(String::new, |shore| {
            format!(
                " shore={}/{:?}/{:?} block_bottom={:.2}",
                shore.hits,
                shore.state(),
                shore.block_state(),
                shore.block_bottom
            )
        })
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
            "COMBAT_STATE fixture={} seed={:#x} tick={} elapsed={} hero_hp={} deaths={} enemies={} hash={:016x}",
            self.fixture.name,
            self.fixture.seed,
            scene.world.tick,
            self.elapsed,
            scene.world.hero.health,
            self.deaths,
            enemies.join(","),
            self.state_hash(scene)
        ) + &self.shore_token()
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
    use crate::combat::{
        CARL_HAYMAKER, CARL_JAB, CARL_PUSH_KICK, ENEMY_HEALTH, HERO_HEALTH, StrikeTarget,
    };
    use crate::feel::{
        CONTACT_POSE_TICKS, CONTACT_STAGGER, HERO_HURT_HIT_STOP, JAB_FEEL, TELL_SPACING,
    };
    use std::path::PathBuf;

    const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

    /// Committed expectations: the final `COMBAT_STATE` hash of each fixture
    /// at its default length. CI compares the headless replay and the real
    /// render run against the same values (`docs/evidence/cp01-feel`).
    const EXPECTED_HASHES: [(&str, u64); 11] = [
        ("cp01_single_goblin", 0xcbb3_07e8_a1a7_420f),
        ("cp01_two_hits", 0x119a_431d_ad6c_7d0b),
        ("cp02_two_goblins", 0x4f3b_150a_c8a0_c633),
        ("cp01_goblin_attacks", 0xa8aa_1111_8006_6441),
        ("cp01_interrupt", 0xee81_fa34_f094_91e0),
        ("cp01_haymaker_chain", 0x01a7_fa7a_9770_271d),
        ("cp01_retry", 0xedaa_118c_f0e3_1e94),
        ("cp01_kick", 0x9e13_d646_1914_9b71),
        ("cp02_kick_contact", 0x8bd0_1343_f47e_d9df),
        ("cp03_shore_kill", 0x098e_86cd_e2e6_b065),
        ("cp03_shore_miss", 0x317b_ead5_5c8a_6a73),
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

    fn ticks_of(events: &[CombatEvent], pick: fn(&CombatEvent) -> Option<u64>) -> Vec<u64> {
        events.iter().filter_map(pick).collect()
    }

    fn tells(events: &[CombatEvent]) -> Vec<u64> {
        ticks_of(events, |e| match e {
            CombatEvent::GoblinTell { tick, .. } => Some(*tick),
            _ => None,
        })
    }

    fn swings(events: &[CombatEvent]) -> Vec<u64> {
        ticks_of(events, |e| match e {
            CombatEvent::GoblinSwing { tick, .. } => Some(*tick),
            _ => None,
        })
    }

    fn hurts(events: &[CombatEvent]) -> Vec<u64> {
        ticks_of(events, |e| match e {
            CombatEvent::HeroHurt { tick, .. } => Some(*tick),
            _ => None,
        })
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
                shore_dx: None,
                script: &[],
                ticks: 100,
            };
        };
    }
    dummy_fixture!(DUMMY_AHEAD, 428);
    dummy_fixture!(DUMMY_BEHIND, 372);
    dummy_fixture!(DUMMY_ON_CARL, 400);

    /// One chasing goblin and an idle Carl (the walk-up takes about 127 ticks).
    static CHASER: Fixture = Fixture {
        name: "test_chaser",
        summary: "test",
        seed: 9,
        carl_x: 400,
        enemies: &[EnemySpawn {
            x: 440,
            ai: EnemyAi::Chase,
        }],
        shore_dx: None,
        script: &[],
        ticks: 400,
    };

    /// Three chasing goblins already in range: more than the token allows.
    static THREE_GOBLINS: Fixture = Fixture {
        name: "test_three",
        summary: "test",
        seed: 11,
        carl_x: 400,
        enemies: &[
            EnemySpawn {
                x: 428,
                ai: EnemyAi::Chase,
            },
            EnemySpawn {
                x: 429,
                ai: EnemyAi::Chase,
            },
            EnemySpawn {
                x: 430,
                ai: EnemyAi::Chase,
            },
        ],
        shore_dx: None,
        script: &[],
        ticks: 120,
    };

    /// Carl runs right, past a goblin that is already in its tell.
    static WALK_PAST: Fixture = Fixture {
        name: "test_walk_past",
        summary: "test",
        seed: 13,
        carl_x: 400,
        enemies: &[EnemySpawn {
            x: 430,
            ai: EnemyAi::Chase,
        }],
        shore_dx: None,
        script: &[Hold {
            from: 1,
            to: 40,
            input: RawInput {
                move_x: 1.0,
                jump: false,
                attack: false,
                physical: false,
                ability: false,
                pause: false,
            },
        }],
        ticks: 150,
    };

    /// A jab pressed so that it lands in the goblin's smear (attack tick 25).
    static ARMOURED: Fixture = Fixture {
        name: "test_armoured",
        summary: "test",
        seed: 15,
        carl_x: 400,
        enemies: &[EnemySpawn {
            x: 560,
            ai: EnemyAi::Chase,
        }],
        shore_dx: None,
        script: &[attack(149, 150)],
        ticks: 200,
    };

    /// No goblins: Carl alone (the Donut and combat-neutrality checks).
    static NO_GOBLINS: Fixture = Fixture {
        name: "test_alone",
        summary: "test",
        seed: 17,
        carl_x: 400,
        enemies: &[],
        shore_dx: None,
        script: &[],
        ticks: 100,
    };

    fn max_tokens(sim: &mut FixtureSim, ticks: u64) -> usize {
        let mut most = 0;
        for _ in 0..ticks {
            sim.step_scripted();
            most = most.max(sim.combat.enemies.iter().filter(|g| g.token_held()).count());
        }
        most
    }

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
        // The jab's second active tick comes after the hit-stop and does not
        // hit again.
        for _ in 0..8 {
            sim.step(idle());
        }
        assert_eq!(hits(&events_of(&sim)).len(), 1);
    }

    #[test]
    fn a_press_in_the_string_window_is_the_haymaker_and_an_early_press_is_forgotten() {
        // Presses on 10 (jab), 12 (jab tick 2: forgotten) and 20. The jab hits
        // on 13 and freezes 14..16, so tick 20 is jab tick 7: the haymaker
        // starts on the press tick itself.
        let mut sim = sim_of(&DUMMY_AHEAD);
        for tick in 1..=60 {
            let press = matches!(tick, 10 | 12 | 20);
            sim.step(if press { attack_input() } else { idle() });
        }
        assert_eq!(
            starts(&events_of(&sim)),
            vec![(10, MoveId::Jab), (20, MoveId::Haymaker)]
        );
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
    fn hit_stop_freezes_attacker_and_target_and_shifts_every_timer_by_the_same_ticks() {
        let mut sim = sim_of(&DUMMY_AHEAD);
        for _ in 0..9 {
            sim.step(idle());
        }
        sim.step(attack_input()); // tick 10
        for _ in 0..3 {
            sim.step(idle()); // ticks 11, 12, 13: the hit lands on 13
        }
        assert_eq!(hits(&events_of(&sim)), vec![(13, 0)]);
        let stop = u64::from(JAB_FEEL.hit_stop);
        let jab_tick = |sim: &FixtureSim| sim.scene.world.hero.melee.current().map(|m| m.tick);
        let stagger = |sim: &FixtureSim| sim.combat.enemies[0].stagger_left;
        let at_hit = (jab_tick(&sim), stagger(&sim), sim.scene.world.hero.x);
        assert_eq!(at_hit.0, Some(3));
        for frozen in 1..=stop {
            sim.step(idle());
            assert_eq!(
                jab_tick(&sim),
                at_hit.0,
                "attacker frozen on {}",
                13 + frozen
            );
            assert_eq!(stagger(&sim), at_hit.1, "target frozen on {}", 13 + frozen);
        }
        sim.step(idle()); // tick 17: both resume
        assert_eq!(jab_tick(&sim), Some(4));
        assert_eq!(stagger(&sim), at_hit.1 - 1);
        // The move ends hit_stop ticks later than without the freeze: the
        // last move tick (13) is tick 26, the move is over on tick 27.
        for _ in 18..=26 {
            sim.step(idle());
        }
        assert!(sim.scene.world.hero.melee.busy());
        sim.step(idle());
        assert!(!sim.scene.world.hero.melee.busy());
        assert_eq!(14 + stop, 17);
    }

    #[test]
    fn hit_stop_is_part_of_the_replay_and_two_runs_agree_tick_by_tick() {
        let hash_at = |ticks: u64| {
            let mut sim = sim_of(&CP01_HAYMAKER_CHAIN);
            sim.run_scripted(ticks);
            sim.state_hash()
        };
        for ticks in [139, 141, 142, 144, 150, 153] {
            assert_eq!(hash_at(ticks), hash_at(ticks), "tick {ticks}");
        }
        // And the freeze is visible in the state: different ticks differ.
        assert_ne!(hash_at(141), hash_at(142));
    }

    #[test]
    fn a_hit_in_the_tell_cancels_the_swing_and_a_new_tell_waits_out_the_stagger_and_cooldown() {
        let mut sim = sim_of(&CP01_INTERRUPT);
        sim.run_scripted(CP01_INTERRUPT.ticks);
        let events = events_of(&sim);
        // The 3 px jab push makes the goblin walk back in before its next tell.
        assert_eq!(tells(&events), vec![127, 177]);
        assert_eq!(hits(&events), vec![(140, 0)]);
        assert!(events.contains(&CombatEvent::GoblinInterrupted {
            tick: 140,
            goblin: 0
        }));
        // The cancelled attack's swing (tell 127 + 26 = 153) never happens and
        // Carl is never hurt.
        assert!(swings(&events).is_empty());
        assert!(hurts(&events).is_empty());
        assert_eq!(sim.scene.world.hero.health, HERO_HEALTH);
        // New tell: after the hit-stop (3), the stagger (10) and the 20-tick
        // post-hurt cooldown.
        let earliest = 140 + u64::from(JAB_FEEL.hit_stop) + u64::from(JAB_FEEL.stagger) + 20;
        assert!(
            tells(&events)[1] >= earliest,
            "{} < {earliest}",
            tells(&events)[1]
        );
    }

    #[test]
    fn a_hit_in_the_smear_is_armoured_and_the_frozen_swing_hurts_once_hit_stop_later() {
        let mut sim = sim_of(&ARMOURED);
        sim.run_scripted(ARMOURED.ticks);
        let events = events_of(&sim);
        assert_eq!(tells(&events)[0], 127);
        // Jab pressed 149, hit on 152 = goblin attack tick 25 (smear).
        assert_eq!(hits(&events), vec![(152, 0)]);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, CombatEvent::GoblinInterrupted { .. })),
            "a swing is not interrupted"
        );
        // Without the freeze the swing is active on 153; frozen for the hit
        // tick plus 3 more ticks it opens on 157, and hurts exactly once, not
        // during the freeze.
        let first_hurt = hurts(&events)[0];
        assert_eq!(first_hurt, 152 + u64::from(JAB_FEEL.hit_stop) + 2);
        assert_eq!(swings(&events)[0], first_hurt);
        assert_eq!(hurts(&events).len(), 1, "one blow per swing");
        // The frozen jab's second active tick did not hit the goblin again.
        assert_eq!(sim.combat.enemies[0].health, ENEMY_HEALTH - 1);
    }

    #[test]
    fn at_most_two_goblins_are_in_a_tell_or_swing_and_tells_start_12_ticks_apart() {
        let mut sim = sim_of(&THREE_GOBLINS);
        assert_eq!(max_tokens(&mut sim, THREE_GOBLINS.ticks), 2);
        let starts = tells(&events_of(&sim));
        assert!(starts.len() >= 3, "all three eventually attack: {starts:?}");
        for pair in starts.windows(2) {
            assert!(
                pair[1] - pair[0] >= TELL_SPACING,
                "tells {} and {} too close",
                pair[0],
                pair[1]
            );
        }
        // The nearest goblins get the two tokens: the third waits until the
        // first swing is over (30 ticks after its tell).
        assert_eq!(starts[0], 1);
        assert_eq!(
            starts[1],
            1 + TELL_SPACING,
            "the second tell waits for the spacing"
        );
        assert!(
            starts[2] >= starts[0] + 30,
            "no third token before one returns"
        );
        let who: Vec<u32> = events_of(&sim)
            .iter()
            .filter_map(|e| match e {
                CombatEvent::GoblinTell { goblin, .. } => Some(*goblin),
                _ => None,
            })
            .collect();
        assert_eq!(&who[..2], &[0, 1], "lowest distance first, then lower id");
    }

    #[test]
    fn the_two_goblin_fixtures_respect_the_token_and_the_spacing() {
        for fixture in [&CP02_TWO_GOBLINS, &CP01_RETRY] {
            let mut sim = sim_of(fixture);
            assert!(max_tokens(&mut sim, fixture.ticks) <= 2, "{}", fixture.name);
            let starts = tells(&events_of(&sim));
            // Carl kills cp02's goblins before they attack; only the retry
            // fixture has tells to space.
            assert!(
                !starts.is_empty() || fixture.name == "cp02_two_goblins",
                "{}",
                fixture.name
            );
            // Tick numbers restart with an attempt; compare within attempts.
            for pair in starts.windows(2).filter(|p| p[1] > p[0]) {
                assert!(
                    pair[1] - pair[0] >= TELL_SPACING,
                    "{}: {pair:?}",
                    fixture.name
                );
            }
        }
        // Two goblins in range together: the spacing is the only limit.
        static PAIR: Fixture = Fixture {
            name: "test_pair",
            summary: "test",
            seed: 19,
            carl_x: 400,
            enemies: &[
                EnemySpawn {
                    x: 428,
                    ai: EnemyAi::Chase,
                },
                EnemySpawn {
                    x: 429,
                    ai: EnemyAi::Chase,
                },
            ],
            shore_dx: None,
            script: &[],
            ticks: 60,
        };
        let mut sim = sim_of(&PAIR);
        sim.run_scripted(PAIR.ticks);
        assert_eq!(tells(&events_of(&sim)), vec![1, 1 + TELL_SPACING]);
    }

    #[test]
    fn the_swing_hurts_carl_only_on_its_first_active_tick_in_front_of_the_goblin() {
        let mut sim = sim_of(&CP01_GOBLIN_ATTACKS);
        sim.run_scripted(CP01_GOBLIN_ATTACKS.ticks);
        let events = events_of(&sim);
        let (tells, swings, hurts) = (tells(&events), swings(&events), hurts(&events));
        // Tell 127: the swing opens on attack tick 26 (153) and that is the
        // first overlapping active tick, so Carl is hurt then, never earlier.
        assert_eq!(swings[0], tells[0] + 26);
        assert_eq!(hurts[0], swings[0]);
        // Every blow coincides with a swing opening: no body-contact damage.
        assert!(hurts.iter().all(|t| swings.contains(t)));
        assert_eq!(hurts.len(), 2);
        // Behind the goblin the same swing misses: Carl runs past the goblin
        // while it is in its tell and is not hurt by that swing.
        let mut sim = sim_of(&WALK_PAST);
        sim.run_scripted(70);
        let events = events_of(&sim);
        assert_eq!(swings_in(&events, 70), 1);
        assert!(
            hurts_in(&events, 70).is_empty(),
            "behind the goblin: no hit"
        );
        assert_eq!(sim.scene.world.hero.health, HERO_HEALTH);
    }

    fn swings_in(events: &[CombatEvent], until: u64) -> usize {
        swings(events).iter().filter(|t| **t <= until).count()
    }

    fn hurts_in(events: &[CombatEvent], until: u64) -> Vec<u64> {
        hurts(events).into_iter().filter(|t| *t <= until).collect()
    }

    #[test]
    fn standing_in_a_goblin_never_hurts_only_the_swing_does() {
        let mut sim = sim_of(&DUMMY_ON_CARL);
        for _ in 0..200 {
            sim.step(idle());
        }
        assert_eq!(sim.scene.world.hero.health, HERO_HEALTH);
        assert!(hurts(&events_of(&sim)).is_empty());
        // The contact damage of the greybox is gone: a chasing goblin that
        // walks up while Carl idles hurts him only through its swing.
        let mut sim = sim_of(&CHASER);
        sim.run_scripted(60);
        let events = events_of(&sim);
        assert!(hurts(&events).iter().all(|t| swings(&events).contains(t)));
    }

    #[test]
    fn hurt_invulnerability_blocks_a_second_swing_in_the_window() {
        let mut sim = sim_of(&THREE_GOBLINS);
        sim.run_scripted(100);
        let events = events_of(&sim);
        assert!(swings(&events).len() >= 3, "{:?}", swings(&events));
        assert_eq!(hurts(&events).len(), 1, "later swings land in the i-frames");
        assert_eq!(sim.scene.world.hero.health, HERO_HEALTH - 1);
    }

    #[test]
    fn carl_is_frozen_by_a_blow_then_flinches_for_18_ticks_and_cannot_act() {
        let mut sim = sim_of(&CP01_GOBLIN_ATTACKS);
        sim.run_scripted(152);
        assert!(hurts(&events_of(&sim)).is_empty());
        sim.step_scripted(); // tick 153: the blow
        assert_eq!(hurts(&events_of(&sim)), vec![153]);
        assert_eq!(sim.combat.hurt_ticks, HERO_HURT_TICKS);
        assert_eq!(sim.combat.hero_hit_stop, HERO_HURT_HIT_STOP);
        let x = sim.scene.world.hero.x;
        for _ in 0..HERO_HURT_HIT_STOP {
            // An attack press during the freeze or the flinch is dropped.
            sim.step(attack_input());
            sim.step(idle());
            assert_eq!(sim.scene.world.hero.x, x);
        }
        assert!(starts(&events_of(&sim)).is_empty());
        assert!(!sim.scene.world.hero.melee.busy());
        // The flinch ticks only count down after the freeze: 4 frozen + 4
        // counted ticks.
        assert_eq!(sim.combat.hurt_ticks, HERO_HURT_TICKS - HERO_HURT_HIT_STOP);
    }

    #[test]
    fn defeat_after_two_hits_launches_the_goblin_and_the_corpse_never_collides_or_hurts() {
        let mut sim = sim_of(&CP01_TWO_HITS);
        sim.run_scripted(CP01_TWO_HITS.ticks);
        let enemy = &sim.combat.enemies[0];
        assert_eq!(enemy.health, 0);
        assert!(!enemy.alive() && !enemy.hurtbox_active());
        assert_eq!(enemy.mode, GoblinMode::Corpse);
        assert!(events_of(&sim).iter().any(|e| matches!(
            e,
            CombatEvent::EnemyDefeated {
                tick: 73,
                target: 0,
                cause: DefeatCause::Melee,
            }
        )));
        // The final hit launched it away from Carl, bounded.
        let flown = enemy.x - 428.0;
        assert!((60.0..95.0).contains(&flown), "flew {flown}");
        // Jab the corpse: no hit. Walk through it: no damage, no block.
        let hits_before = hits(&events_of(&sim)).len();
        for _ in 0..3 {
            sim.step(attack_input());
            for _ in 0..16 {
                sim.step(idle());
            }
        }
        let corpse_x = sim.combat.enemies[0].x;
        for _ in 0..140 {
            sim.step(RawInput {
                move_x: 1.0,
                ..idle()
            });
        }
        assert!(
            sim.scene.world.hero.x > corpse_x + 30.0,
            "Carl walked through the corpse"
        );
        let events = events_of(&sim);
        assert_eq!(hits(&events).len(), hits_before);
        assert!(hurts(&events).is_empty());
        assert_eq!(sim.scene.world.hero.health, HERO_HEALTH);
    }

    #[test]
    fn push_kick_uses_the_secondary_action_and_hits_on_its_13th_tick() {
        let mut sim = sim_of(&DUMMY_AHEAD);
        for _ in 0..13 {
            sim.step(RawInput {
                physical: true, // held: still a single press edge
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
        assert!(matches!(events[1], CombatEvent::Hit { hit_stop: 8, .. }));
    }

    #[test]
    fn carl_flinches_cannot_act_while_hurt_dies_and_the_fixture_restarts_fast() {
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
        assert!(
            hurts(&events).len() >= usize::from(HERO_HEALTH),
            "four blows defeat Carl"
        );
        let defeated = events
            .iter()
            .find_map(|e| match e {
                CombatEvent::HeroDefeated { tick } => Some(*tick),
                _ => None,
            })
            .expect("Carl is defeated");
        let restart = events
            .iter()
            .find_map(|e| match e {
                CombatEvent::Restart { tick, .. } => Some(*tick),
                _ => None,
            })
            .expect("the fixture restarts by itself");
        assert_eq!(sim.combat.deaths, 1);
        // Fast retry: under 60 ticks from the defeating blow, and the event
        // carries the tick that triggered the restart (not 0).
        assert_eq!(restart, defeated + u64::from(RESTART_DELAY_TICKS) - 1);
        assert!(restart - defeated < 60);
        // Restarted: Carl is alive again at the fixture start.
        assert!(sim.scene.world.hero.health > 0);
        assert!(!sim.combat.hero_defeated);
        assert!(sim.scene.world.tick < 400);
    }

    #[test]
    fn the_defeating_blow_knocks_carl_down_away_from_the_goblin_then_the_retry_starts() {
        let mut sim = sim_of(&CP01_GOBLIN_ATTACKS);
        sim.scene.world.hero.health = 1;
        sim.run_scripted(153);
        assert!(sim.combat.hero_defeated);
        assert_eq!(sim.combat.hero_hit_stop, crate::feel::HERO_DEFEAT_HIT_STOP);
        let x0 = sim.scene.world.hero.x;
        for _ in 0..30 {
            sim.step_scripted();
        }
        let moved = x0 - sim.scene.world.hero.x;
        assert!((15.0..80.0).contains(&moved), "knocked back {moved}");
        // The retry is inside 60 ticks of the defeating blow.
        for _ in 0..(RESTART_DELAY_TICKS as usize) {
            sim.step_scripted();
        }
        assert!(!sim.combat.hero_defeated);
        assert_eq!(sim.scene.world.hero.health, HERO_HEALTH);
        assert_eq!(sim.scene.world.hero.x, 400.0);
    }

    #[test]
    fn restart_is_deterministic() {
        let mut sim = sim_of(&CP01_SINGLE_GOBLIN);
        sim.run_scripted(CP01_SINGLE_GOBLIN.ticks);
        let first = sim.scene.state_hash();
        let first_enemy = sim.combat.enemies[0].clone();
        sim.restart();
        sim.run_scripted(CP01_SINGLE_GOBLIN.ticks);
        assert_eq!(sim.scene.state_hash(), first, "same script after a restart");
        assert_eq!(sim.combat.enemies[0], first_enemy);
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
        assert_eq!(mid.scene.state_hash(), first);
        assert_eq!(mid.combat.enemies[0], first_enemy);
        // The attempt hash (the event hash restarts too) matches a fresh run
        // apart from the counters that span attempts (deaths, elapsed).
        assert_eq!(mid.combat.event_hash, sim.combat.event_hash);
    }

    #[test]
    fn the_retry_fixture_plays_the_same_attempt_twice_and_restarts_fast() {
        let mut sim = sim_of(&CP01_RETRY);
        sim.run_scripted(CP01_RETRY.ticks);
        let events = events_of(&sim);
        let restart_at = events
            .iter()
            .position(|e| matches!(e, CombatEvent::Restart { .. }))
            .expect("one restart");
        let (first, rest) = events.split_at(restart_at);
        let second = &rest[1..];
        // Attempt-local ticks: the second attempt repeats the first one's
        // events tick for tick, up to the defeat.
        let cut = first
            .iter()
            .position(|e| matches!(e, CombatEvent::HeroDefeated { .. }))
            .unwrap()
            + 1;
        assert_eq!(first[..cut], second[..cut]);
        let CombatEvent::Restart { tick, deaths } = rest[0] else {
            unreachable!()
        };
        let defeated = first
            .iter()
            .find_map(|e| match e {
                CombatEvent::HeroDefeated { tick } => Some(*tick),
                _ => None,
            })
            .unwrap();
        assert_eq!(deaths, 1);
        assert_eq!(tick, defeated + u64::from(RESTART_DELAY_TICKS) - 1);
        assert_eq!(sim.combat.deaths, 2, "defeated again, the same way");
        assert!(sim.combat.elapsed > sim.scene.world.tick);
    }

    #[test]
    fn the_restart_event_logs_the_triggering_tick_not_zero() {
        let mut sim = sim_of(&DUMMY_AHEAD);
        for _ in 0..77 {
            sim.step(idle());
        }
        sim.restart();
        assert_eq!(
            events_of(&sim).last(),
            Some(&CombatEvent::Restart {
                tick: 77,
                deaths: 0
            })
        );
        assert_eq!(
            sim.scene.world.tick, 0,
            "the new attempt counts from 1 again"
        );
        sim.step(idle());
        assert_eq!(sim.scene.world.tick, 1);
        assert_eq!(sim.combat.elapsed, 78, "elapsed spans attempts");
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
        let mut unique = hashes.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), hashes.len(), "every fixture has its own hash");
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
    fn the_held_button_state_is_part_of_the_hash() {
        // Same press edge, same moves, same events: only the held state at the
        // end differs.
        let run = |held: bool| {
            let mut sim = sim_of(&DUMMY_AHEAD);
            sim.step(attack_input());
            for _ in 0..4 {
                sim.step(if held { attack_input() } else { idle() });
            }
            sim
        };
        let (tapped, held) = (run(false), run(true));
        assert_eq!(events_of(&tapped), events_of(&held));
        assert_ne!(tapped.state_hash(), held.state_hash());
    }

    #[test]
    fn the_manual_event_log_is_capped_and_the_live_cursor_survives_it() {
        let mut sim = sim_of(&DUMMY_AHEAD);
        for _ in 0..100 {
            sim.restart();
        }
        assert_eq!(sim.combat.take_new_events().len(), 100);
        for _ in 0..600 {
            sim.restart();
        }
        assert_eq!(sim.combat.events.len(), EVENT_LOG_CAP);
        assert_eq!(sim.combat.dropped_events, 700 - EVENT_LOG_CAP);
        // 600 events were logged since the last read; the cap keeps the
        // newest 512 and the cursor cannot point before them.
        assert_eq!(sim.combat.take_new_events().len(), EVENT_LOG_CAP);
        assert!(sim.combat.take_new_events().is_empty());
        // The newest event is the last restart.
        assert!(matches!(
            sim.combat.events.last(),
            Some(CombatEvent::Restart { .. })
        ));
    }

    #[test]
    fn cp01_single_goblin_is_defeated_by_a_jab_and_a_haymaker_without_hurting_carl() {
        let mut sim = sim_of(&CP01_SINGLE_GOBLIN);
        sim.run_scripted(CP01_SINGLE_GOBLIN.ticks);
        let events = events_of(&sim);
        // The second press, inside the jab's string window, is the haymaker.
        assert_eq!(
            starts(&events),
            vec![(122, MoveId::Jab), (136, MoveId::Haymaker)]
        );
        assert_eq!(hits(&events), vec![(125, 0), (141, 0)]);
        assert_eq!(sim.combat.enemies[0].health, 0);
        assert_eq!(sim.scene.world.hero.health, HERO_HEALTH);
        assert_eq!(sim.combat.deaths, 0);
    }

    #[test]
    fn cp01_haymaker_chain_jab_then_haymaker_kills_inside_the_window() {
        let mut sim = sim_of(&CP01_HAYMAKER_CHAIN);
        sim.run_scripted(CP01_HAYMAKER_CHAIN.ticks);
        let events = events_of(&sim);
        // Jab pressed 137 (jab tick 0), second press 141 (jab tick 3 plus the
        // 3 frozen ticks: inside the window); the haymaker starts at jab tick 7
        // (147) and its first overlapping active tick is its tick 5 (152).
        assert_eq!(
            starts(&events),
            vec![(137, MoveId::Jab), (147, MoveId::Haymaker)]
        );
        assert_eq!(hits(&events), vec![(140, 0), (152, 0)]);
        assert!(events.iter().any(|e| matches!(
            e,
            CombatEvent::Hit {
                move_id: MoveId::Haymaker,
                health_left: 0,
                hit_stop: 8,
                ..
            }
        )));
        assert_eq!(sim.combat.enemies[0].mode, GoblinMode::Corpse);
        assert!(hurts(&events).is_empty());
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
            // A launched goblin flies about 50 px: the floor under that is
            // clean too.
            for x in (fixture.carl_x - 100)..(far + 100) {
                assert_eq!(kit.walking_line_at(x), line, "{} column {x}", fixture.name);
            }
        }
    }

    #[test]
    fn presentation_events_are_derived_from_the_hit_and_never_change_the_hash() {
        let mut sim = sim_of(&CP01_HAYMAKER_CHAIN);
        sim.run_scripted(140);
        // The jab confirmed on 140: flash, but no camera kick (heavy blows only).
        assert_eq!(sim.combat.camera_kick(140), 0);
        assert!(sim.combat.flash(0, 140) && sim.combat.flash(0, 142));
        assert!(!sim.combat.flash(0, 143));
        sim.run_scripted(12);
        // The haymaker killed on 152: kick 2 + 1 (lethal) = 3 virtual px, with
        // the view moved against the hit direction (see `camera_kick`), fading
        // over CAMERA_KICK_TICKS ticks through every level.
        let kicks: Vec<i32> = (152..152 + u64::from(CAMERA_KICK_TICKS) + 1)
            .map(|t| sim.combat.camera_kick(t))
            .collect();
        assert_eq!(kicks, vec![-3, -3, -2, -2, -2, -1, -1, 0, 0]);
        let before = sim.state_hash();
        // Clearing the whole presentation cache cannot move the sim hash.
        sim.combat.confirm = None;
        sim.combat.flash_tick.fill(None);
        assert_eq!(sim.state_hash(), before);
        assert_eq!(sim.combat.camera_kick(152), 0);
        // Facing left flips the sign: the push kick (2 px) toward -x.
        let mut left = sim_of(&DUMMY_BEHIND);
        left.step(RawInput {
            physical: true,
            move_x: -1.0,
            ..idle()
        });
        for _ in 0..13 {
            left.step(idle());
        }
        assert_eq!(hits(&events_of(&left)), vec![(14, 0)]);
        assert_eq!(left.combat.camera_kick(14), 2);
    }

    /// B1: the kick used to move the whole scene towards the attacker, which
    /// reversed the knockback on screen. The goblin's screen x (world x minus
    /// the view's left edge, kick included) must never fall behind where it was
    /// before the hit while it is frozen: it stays or moves along the knockback.
    #[test]
    fn the_camera_kick_never_pulls_the_goblin_back_during_hit_stop() {
        let screen_x = |sim: &FixtureSim, tick: u64| {
            sim.combat.enemies[0].x
                - (sim.scene.camera.left() as f32 + sim.combat.camera_kick(tick) as f32)
        };
        // Carl facing right: the goblin is knocked to +x.
        let mut sim = sim_of(&CP01_HAYMAKER_CHAIN);
        sim.run_scripted(151);
        let before = screen_x(&sim, 151);
        let mut moved = false;
        for tick in 152..=164 {
            sim.step_scripted();
            let now = screen_x(&sim, tick);
            assert!(now >= before - 1e-3, "tick {tick}: {now} behind {before}");
            moved |= now > before;
        }
        assert!(moved, "the kick shows on screen");
        // Carl facing left: the goblin is knocked to -x, the sign flips.
        let mut left = sim_of(&DUMMY_BEHIND);
        left.step(RawInput {
            physical: true,
            move_x: -1.0,
            ..idle()
        });
        for _ in 0..12 {
            left.step(idle());
        }
        let before = screen_x(&left, 13);
        for tick in 14..=30 {
            left.step(idle());
            assert!(screen_x(&left, tick) <= before + 1e-3, "tick {tick}");
        }
    }

    #[test]
    fn the_haymaker_steps_carl_forward_4_px_one_per_tick_and_a_jab_does_not() {
        let mut sim = sim_of(&CP01_HAYMAKER_CHAIN);
        let mut xs = Vec::new();
        for _ in 0..155 {
            sim.step_scripted();
            xs.push((sim.scene.world.tick, sim.scene.world.hero.x));
        }
        let x_at = |t: u64| xs.iter().find(|e| e.0 == t).unwrap().1;
        // The jab (137..) keeps the feet planted; the haymaker starts on 147.
        assert_eq!(x_at(147), 400.0);
        for (tick, x) in [
            (148, 401.0),
            (149, 402.0),
            (150, 403.0),
            (151, 404.0),
            (152, 404.0),
        ] {
            assert_eq!(x_at(tick), x, "tick {tick}");
        }
        assert_eq!(x_at(155), 404.0, "no step after move tick 4");
    }

    #[test]
    fn the_haymaker_step_stops_at_body_contact_and_at_walls() {
        // Body contact: a goblin 27 px away lets Carl close to 24 px, no more.
        let mut sim = sim_of(&DUMMY_AHEAD);
        sim.combat.enemies[0].x = 427.0;
        let step = CARL_HAYMAKER.root_step.unwrap();
        for _ in 0..4 {
            sim.combat.root_step(&mut sim.scene, step);
        }
        assert_eq!(sim.scene.world.hero.x, 403.0);
        // A wall 14 px ahead of Carl's right edge.
        let mut sim = sim_of(&DUMMY_BEHIND);
        sim.scene.world.solids.push(crate::prototype::Block {
            rect: Rect {
                left: 414.0,
                bottom: 0.0,
                width: 20.0,
                height: 400.0,
            },
            material: crate::prototype::Material::Stone,
            intact: true,
        });
        for _ in 0..4 {
            sim.combat.root_step(&mut sim.scene, step);
        }
        assert_eq!(sim.scene.world.hero.x, 402.0);
        // Total free travel is exactly HAYMAKER_STEP_PX.
        assert_eq!(
            step.px_per_tick * f32::from(step.to_tick - step.from_tick + 1),
            crate::feel::HAYMAKER_STEP_PX
        );
    }

    #[test]
    fn a_goblin_launched_at_the_right_layout_edge_stops_with_its_body_inside() {
        let width = kit().width as f32;
        let mut sim = sim_of(&DUMMY_AHEAD);
        let goblin = &mut sim.combat.enemies[0];
        goblin.x = width - 20.0;
        goblin.take_hit(&CARL_JAB, 1);
        goblin.hit_stop = 0;
        goblin.take_hit(&CARL_JAB, 1);
        for _ in 0..120 {
            sim.step(idle());
            let right_edge = sim.combat.enemies[0].x + crate::combat::ENEMY_HALF_WIDTH;
            assert!(
                right_edge <= width + 1e-3,
                "right edge {right_edge} > {width}"
            );
        }
        // Carl's own bound is the same edge.
        assert_eq!(
            sim.combat.enemies[0].x,
            width - crate::combat::ENEMY_HALF_WIDTH
        );
    }

    #[test]
    fn hit_stop_freezes_the_camera_and_donuts_trail_not_just_carl() {
        let mut sim = sim_of(&CP01_HAYMAKER_CHAIN);
        sim.run_scripted(152);
        let snapshot = |sim: &FixtureSim| {
            (
                sim.scene.camera.left(),
                sim.scene.world.donut.x,
                sim.scene.world.hero.x,
            )
        };
        let at_hit = snapshot(&sim);
        for _ in 0..8 {
            sim.step_scripted();
            assert_eq!(snapshot(&sim), at_hit, "tick {}", sim.scene.world.tick);
        }
        assert_eq!(sim.scene.world.tick, 160, "the tick counter still runs");
    }

    #[test]
    fn donut_follows_passively_and_does_not_change_carls_simulation() {
        // No goblins and no combat input: the combat layer must leave the
        // scene exactly as the bare corridor simulation runs it, Donut included.
        let kit = kit();
        let mut with = sim_of(&NO_GOBLINS);
        let mut bare = SceneSim::new_at(&kit, 360, 400).unwrap();
        for tick in 1..=120u32 {
            let raw = RawInput {
                move_x: if tick < 60 { 1.0 } else { -1.0 },
                jump: tick % 40 == 10,
                ..idle()
            };
            with.step(raw);
            bare.step(raw);
            if tick == 30 {
                let world = &with.scene.world;
                assert!((world.donut.x - world.hero.x).abs() > 1.0, "she trails him");
            }
        }
        assert_eq!(with.scene.state_hash(), bare.state_hash());
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
            include_str!("goblin.rs"),
            include_str!("feel.rs"),
            include_str!("bin/floor01_scene.rs"),
        ] {
            assert!(!source.contains(&needle));
            assert!(!source.contains(&["prototype", "::World"].concat()));
        }
        assert_eq!(CARL_JAB.id.name(), "jab");
    }

    // ------------------------------------------------ PR A: D-01 to D-04 --

    fn defeats(events: &[CombatEvent]) -> Vec<(u32, DefeatCause)> {
        events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::EnemyDefeated { target, cause, .. } => Some((*target, *cause)),
                _ => None,
            })
            .collect()
    }

    fn contacts(events: &[CombatEvent]) -> Vec<(u32, u32, u32)> {
        events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::GoblinContact {
                    launch,
                    source,
                    target,
                    ..
                } => Some((*launch, *source, *target)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_equipped_special_does_nothing_and_physical_attack_kicks() {
        // Circle/B (EquippedSpecial) has no effect before its unlock.
        let mut special = sim_of(&CP02_KICK_CONTACT);
        for _ in 0..80 {
            special.step(RawInput {
                ability: true,
                ..idle()
            });
        }
        assert!(starts(&events_of(&special)).is_empty());
        // Triangle/Y (PhysicalAttack) starts the push kick on a press edge.
        let mut kicked = sim_of(&CP02_KICK_CONTACT);
        for _ in 0..30 {
            kicked.step(RawInput {
                physical: true,
                ..idle()
            });
        }
        assert_eq!(starts(&events_of(&kicked)), [(1, MoveId::PushKick)]);
    }

    #[test]
    fn d01_a_haymaker_takes_one_goblin_down_without_a_launch_contact() {
        let mut sim = sim_of(&CP01_HAYMAKER_CHAIN);
        sim.run_scripted(CP01_HAYMAKER_CHAIN.ticks);
        let events = events_of(&sim);
        assert_eq!(defeats(&events).len(), 1);
        assert_eq!(defeats(&events)[0].1, DefeatCause::Melee);
        assert!(contacts(&events).is_empty());
        assert!(sim.combat.shore.is_none());
    }

    #[test]
    fn d02_the_kick_launches_a_living_goblin_which_lands_lies_and_gets_up() {
        let mut sim = sim_of(&CP02_KICK_CONTACT);
        let mut modes = Vec::new();
        for _ in 0..CP02_KICK_CONTACT.ticks {
            sim.step_scripted();
            let mode = sim.combat.enemies[1].mode;
            if modes.last() != Some(&mode) {
                modes.push(mode);
            }
        }
        // The kicked goblin (index 0) stays alive with one hit point lost.
        let kicked = &sim.combat.enemies[0];
        assert_eq!(kicked.health, ENEMY_HEALTH - 1);
        assert!(kicked.alive());
        let mut seen = Vec::new();
        let mut again = sim_of(&CP02_KICK_CONTACT);
        for _ in 0..CP02_KICK_CONTACT.ticks {
            again.step_scripted();
            let mode = again.combat.enemies[0].mode;
            if seen.last() != Some(&mode) {
                seen.push(mode);
            }
        }
        assert_eq!(
            seen,
            [
                GoblinMode::Neutral,
                GoblinMode::Launched,
                GoblinMode::Downed,
                GoblinMode::Neutral
            ]
        );
        // The kick that defeats it is the lethal launch (it flies farther).
        assert_eq!(defeats(&events_of(&sim)), []);
    }

    #[test]
    fn d03_a_kicked_goblin_contacts_the_second_exactly_once_with_zero_damage() {
        let mut sim = sim_of(&CP02_KICK_CONTACT);
        sim.run_scripted(CP02_KICK_CONTACT.ticks);
        let events = events_of(&sim);
        let found = contacts(&events);
        assert_eq!(found.len(), 1, "one contact per launch: {found:?}");
        assert_eq!((found[0].1, found[0].2), (0, 1));
        // Zero collision damage: the second goblin keeps its two hit points,
        // the kicked one lost only the kick's, nobody is defeated.
        assert_eq!(sim.combat.enemies[1].health, ENEMY_HEALTH);
        assert_eq!(sim.combat.enemies[0].health, ENEMY_HEALTH - 1);
        assert!(defeats(&events).is_empty());
        // Exactly one Hit event: the kick. The contact never counts as a hit.
        assert_eq!(hits(&events).len(), 1);
        // The contact comes after the kick's hit and before the landing.
        let hit_tick = hits(&events)[0].0;
        let contact_tick = events
            .iter()
            .find_map(|e| match e {
                CombatEvent::GoblinContact { tick, .. } => Some(*tick),
                _ => None,
            })
            .unwrap();
        assert!(contact_tick > hit_tick, "{contact_tick} after {hit_tick}");
    }

    #[test]
    fn d03_the_contact_poses_the_flier_and_interrupts_the_second_goblin() {
        let mut sim = sim_of(&CP02_KICK_CONTACT);
        let mut contact_tick = None;
        for _ in 0..CP02_KICK_CONTACT.ticks {
            sim.step_scripted();
            if contact_tick.is_none() && !contacts(&sim.combat.events).is_empty() {
                contact_tick = Some(sim.scene.world.tick);
                let flier = &sim.combat.enemies[0];
                let hit = &sim.combat.enemies[1];
                assert_eq!(flier.contact_ticks, CONTACT_POSE_TICKS);
                assert_eq!(flier.mode, GoblinMode::Launched);
                assert_eq!(hit.mode, GoblinMode::Hurt);
                assert_eq!(hit.stagger_left, CONTACT_STAGGER);
                assert!(hit.vx > 0.0, "pushed away from the flier");
            }
        }
        assert!(contact_tick.is_some());
    }

    #[test]
    fn d03_a_goblin_in_its_tell_is_interrupted_and_the_token_goes_back() {
        let mut sim = sim_of(&CP02_KICK_CONTACT);
        // The moment the kicked goblin is launched, the second one starts a
        // tell (it would hold an attack token).
        let mut armed = false;
        for _ in 0..60 {
            sim.step_scripted();
            if !armed && sim.combat.enemies[0].mode == GoblinMode::Launched {
                sim.combat.enemies[1].mode = GoblinMode::Attack;
                sim.combat.enemies[1].attack_tick = 0;
                assert!(sim.combat.enemies[1].token_held());
                armed = true;
            }
        }
        assert!(armed);
        let events = events_of(&sim);
        assert_eq!(contacts(&events).len(), 1);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CombatEvent::GoblinInterrupted { goblin: 1, .. }))
        );
        assert!(!sim.combat.enemies[1].token_held());
        assert_eq!(sim.combat.enemies[1].health, ENEMY_HEALTH);
    }

    #[test]
    fn d03_a_wall_between_the_goblins_stops_the_flight_and_nothing_is_reached() {
        use crate::prototype::{Block, Material};
        let mut sim = sim_of(&CP02_KICK_CONTACT);
        let foot = sim.combat.enemies[0].y - ENEMY_HALF_HEIGHT;
        // A wall right behind the kicked goblin, in front of the second one.
        sim.scene.world.solids.push(Block {
            rect: Rect {
                left: 452.0,
                bottom: foot,
                width: 8.0,
                height: 80.0,
            },
            material: Material::Stone,
            intact: true,
        });
        sim.run_scripted(CP02_KICK_CONTACT.ticks);
        assert!(contacts(&events_of(&sim)).is_empty());
        assert_eq!(sim.combat.enemies[1].mode, GoblinMode::Neutral);
        assert!(sim.combat.enemies[0].x + ENEMY_HALF_WIDTH <= 452.0 + 0.001);
    }

    #[test]
    fn d03_a_second_kick_launch_makes_a_new_contact_once_not_a_repeat() {
        // Two launches (two instances) of the same goblin into the same one
        // are two (launch, target) pairs; within one launch it is only one.
        let mut sim = sim_of(&CP02_KICK_CONTACT);
        sim.run_scripted(CP02_KICK_CONTACT.ticks);
        let first = contacts(&events_of(&sim));
        assert_eq!(first.len(), 1);
        let launch = first[0].0;
        assert!(sim.combat.contacts.contains(&(launch, 1)));
    }

    #[test]
    fn the_cp02_and_cp03_fixtures_replay_to_the_same_hash_and_events() {
        for fixture in [&CP02_KICK_CONTACT, &CP03_SHORE_KILL, &CP03_SHORE_MISS] {
            let mut a = sim_of(fixture);
            let mut b = sim_of(fixture);
            a.run_scripted(fixture.ticks);
            b.run_scripted(fixture.ticks);
            assert_eq!(a.state_hash(), b.state_hash(), "{}", fixture.name);
            assert_eq!(events_of(&a), events_of(&b), "{}", fixture.name);
        }
    }

    #[test]
    fn d04_three_strikes_break_the_shore_and_the_block_defeats_the_goblin_once() {
        let mut sim = sim_of(&CP03_SHORE_KILL);
        sim.run_scripted(CP03_SHORE_KILL.ticks);
        let events = events_of(&sim);
        let shore_hits: Vec<u8> = events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::ShoreHit { hits, .. } => Some(*hits),
                _ => None,
            })
            .collect();
        assert_eq!(shore_hits, [1, 2, 3]);
        let count = |pick: fn(&CombatEvent) -> bool| events.iter().filter(|e| pick(e)).count();
        assert_eq!(count(|e| matches!(e, CombatEvent::ShoreBroken { .. })), 1);
        assert_eq!(count(|e| matches!(e, CombatEvent::ShoreLanded { .. })), 1);
        // Exactly one defeat of goblin 0, credited to the player who broke it.
        assert_eq!(
            defeats(&events),
            [(0, DefeatCause::ShoreBlock { player: 0 })]
        );
        let goblin = &sim.combat.enemies[0];
        assert!(!goblin.alive());
        assert!(matches!(goblin.mode, GoblinMode::Down | GoblinMode::Corpse));
        // The three jabs never hit the goblin (it stands outside their reach).
        assert!(hits(&events).is_empty());
        // The landing comes after the break by the delay plus the fall.
        let at = |pick: fn(&CombatEvent) -> Option<u64>| events.iter().find_map(pick).unwrap();
        let broke = at(|e| match e {
            CombatEvent::ShoreBroken { tick, .. } => Some(*tick),
            _ => None,
        });
        let landed = at(|e| match e {
            CombatEvent::ShoreLanded { tick } => Some(*tick),
            _ => None,
        });
        assert!((36..=42).contains(&(landed - broke)), "{broke} -> {landed}");
        // Carl is untouched (no player damage from the block yet).
        assert_eq!(sim.scene.world.hero.health, HERO_HEALTH);
    }

    #[test]
    fn d04_an_empty_footprint_defeats_nobody() {
        let mut sim = sim_of(&CP03_SHORE_MISS);
        sim.run_scripted(CP03_SHORE_MISS.ticks);
        let events = events_of(&sim);
        assert!(defeats(&events).is_empty(), "no fabricated defeat");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CombatEvent::ShoreLanded { .. }))
        );
        assert!(sim.combat.enemies[0].alive());
    }

    #[test]
    fn d04_an_unbroken_shore_never_drops_and_a_held_button_counts_once() {
        static HELD: Fixture = Fixture {
            name: "test_held_strike",
            summary: "test",
            seed: 9,
            carl_x: 400,
            enemies: &[EnemySpawn {
                x: 484,
                ai: EnemyAi::Passive,
            }],
            shore_dx: Some(19),
            script: &[attack(20, 400)],
            ticks: 300,
        };
        let mut sim = sim_of(&HELD);
        sim.run_scripted(HELD.ticks);
        let shore = sim.combat.shore.as_ref().unwrap();
        // One press edge is one jab is one hit: the held button never repeats.
        assert_eq!(shore.hits, 1);
        assert!(sim.combat.enemies[0].alive());
        assert!(defeats(&events_of(&sim)).is_empty());
    }

    #[test]
    fn d04_goblins_do_not_damage_the_shore_and_a_hit_goblin_still_counts_once() {
        // A goblin killed first by ordinary melee, then the shore: the block
        // lands on a corpse and defeats nobody a second time.
        static ORDER: Fixture = Fixture {
            name: "test_goblin_first",
            summary: "test",
            seed: 10,
            carl_x: 400,
            enemies: &[EnemySpawn {
                x: 484,
                ai: EnemyAi::Passive,
            }],
            shore_dx: Some(19),
            script: &[attack(20, 21), attack(60, 61), attack(100, 101)],
            ticks: 220,
        };
        let mut sim = sim_of(&ORDER);
        // Defeat the goblin directly before the block falls.

        assert!(sim.combat.enemies[0].crush());
        sim.run_scripted(ORDER.ticks);
        let events = events_of(&sim);
        assert!(
            defeats(&events).is_empty(),
            "already defeated: no second defeat"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CombatEvent::ShoreLanded { .. }))
        );
    }

    #[test]
    fn d04_the_exit_stays_clear_after_every_ordering() {
        for fixture in [&CP03_SHORE_KILL, &CP03_SHORE_MISS] {
            let mut sim = sim_of(fixture);
            let solids = sim.scene.world.solids.len();
            sim.run_scripted(fixture.ticks);
            let shore = sim.combat.shore.as_ref().unwrap();
            assert!(shore.landed, "{}", fixture.name);
            let right = shore.footprint().right();
            // The landed block is non-collidable: the solids never changed and
            // Carl walks straight through the footprint and past it.
            assert_eq!(sim.scene.world.solids.len(), solids);
            let start = sim.scene.world.hero.x;
            for _ in 0..200 {
                sim.step(RawInput {
                    move_x: 1.0,
                    ..idle()
                });
            }
            let after = sim.scene.world.hero.x;
            assert!(after > right, "{} stopped at {after}", fixture.name);
            assert!(after > start + 100.0);
        }
        // The shore never touched: the route is the plain corridor.
        let mut sim = sim_of(&CP01_SINGLE_GOBLIN);
        sim.run_scripted(10);
        assert!(sim.combat.shore.is_none());
    }

    #[test]
    fn d04_a_retry_resets_the_shore_the_enemies_and_the_credit() {
        let mut sim = sim_of(&CP03_SHORE_KILL);
        sim.run_scripted(CP03_SHORE_KILL.ticks);
        let shore = sim.combat.shore.clone();
        let enemies = sim.combat.enemies.clone();
        let log_len = sim.combat.events.len();
        sim.restart();
        let fresh = sim.combat.shore.as_ref().unwrap();
        assert_eq!(fresh.hits, 0);
        assert!(!fresh.landed);
        assert!(sim.combat.enemies[0].alive());
        sim.run_scripted(CP03_SHORE_KILL.ticks);
        assert_eq!(sim.combat.shore, shore, "same attempt, same shore");
        assert_eq!(sim.combat.enemies, enemies);
        let all = events_of(&sim);
        let second: Vec<_> = all[log_len + 1..].to_vec();
        let first: Vec<_> = all[..log_len].to_vec();
        assert_eq!(first, second, "the replayed attempt logs the same events");
    }

    #[test]
    fn the_defeat_event_line_carries_the_cause_only_when_it_is_not_melee() {
        let melee = CombatEvent::EnemyDefeated {
            tick: 3,
            target: 1,
            cause: DefeatCause::Melee,
        };
        assert_eq!(
            melee.to_string(),
            "COMBAT_EVENT tick=3 kind=enemy_defeated target=1"
        );
        let block = CombatEvent::EnemyDefeated {
            tick: 3,
            target: 1,
            cause: DefeatCause::ShoreBlock { player: 0 },
        };
        assert!(
            block
                .to_string()
                .ends_with("cause=shore_block credit_player=0")
        );
    }
}
