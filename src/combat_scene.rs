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

use crate::blast::{Arm, BlastCharge, BlastReach};
use crate::boss::{
    BOSS_ENEMY_ID, Boss, BossCtx, BossHit, BossOut, BossSetup, BossState, HeroBlow, HeroView,
};
use crate::cf02::{self, Cf02Tuning};
use crate::combat::{
    Attacker, CARL_BUFFER_TICKS, CombatEvent, DefeatCause, ENEMY_HALF_HEIGHT, ENEMY_HALF_WIDTH,
    EnemyAi, HERO_INVULNERABLE_TICKS, MoveId, MoveRunner, MoveSpec, MoveTick, Phase, RootStep,
    carl_move, hit_box, solid_at, strike_group,
};
use crate::corridor::{PartyStepCtx, SceneSim};
use crate::donut_mana::{DonutSetup, RestartKind};
use crate::env_kit::EnvKit;
use crate::feel::{
    CAMERA_KICK_TICKS, GROUND_FRICTION, HERO_DEFEAT_HIT_STOP, HERO_HURT_HIT_STOP,
    HERO_KNOCKDOWN_VX, HERO_KNOCKDOWN_VY, IMPACT_END, IMPACT_FLASH_TICKS, LETHAL_CAMERA_KICK_BONUS,
    MAX_ATTACKERS, TELL_SPACING,
};
pub use crate::feel::{HERO_HURT_TICKS, RESTART_DELAY_TICKS};
use crate::follower::Threat;
use crate::goblin::{Goblin, GoblinMode};
use crate::input::{InputFrame, RawInput};
use crate::llama::{
    FireView, Glob, GlobFate, LLAMA_HALF_HEIGHT, LLAMA_HALF_WIDTH, Llama, LlamaMode,
};
use crate::party::{EnemyTargetRule, HeroId};
use crate::prototype::{
    Block, BodyDrive, HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Hero, Rect, TICK_SECONDS,
};
use crate::shore::{PLAYER_ONE, Shore};

mod carl_cf02;
mod companion;
mod donut;
use self::carl_cf02::{CarlCf02, CarlCf02Plan};
use self::companion::CompanionRuntime;
pub use donut::{Bolt, Cast, DonutKit};

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

/// One Bad Llama spawn (DEMO-01 PR B): its centre column and behaviour. It
/// stands on Carl's walking line (checked on load). `home` is the spawn column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LlamaSpawn {
    pub x: i32,
    pub ai: EnemyAi,
}

/// DEATH-D: one authored blast charge of a QA fixture. The only producer of a
/// [`BlastCharge`]: there is no route, ability or input that creates one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChargeSpawn {
    /// Column relative to Carl's start column (feet on the walking line).
    pub dx: i32,
    /// The attempt-local tick it detonates on.
    pub tick: u64,
    /// The player credited with the blast (`None`: authored, no credit).
    pub owner: Option<u32>,
    /// What sets it off: `Arm::Tick` (the fixed tick above) for every QA
    /// fixture, or DEMO-01's `Arm::CarlPast` (the tick is then unused).
    pub arm: Arm,
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
            physical: true,
            ..RawInput::NONE
        },
    }
}

/// A single held attack button over `from..to`.
const fn attack(from: u64, to: u64) -> Hold {
    Hold {
        from,
        to,
        input: RawInput {
            attack: true,
            ..RawInput::NONE
        },
    }
}

/// A held attack (`kick`: the push kick) over `from..to` pressed together with
/// a direction, which turns Carl first (`dir` is +1 or -1).
const fn strike_towards(from: u64, to: u64, dir: f32, kick: bool) -> Hold {
    Hold {
        from,
        to,
        input: RawInput {
            move_x: dir,
            attack: !kick,
            physical: kick,
            ..RawInput::NONE
        },
    }
}

/// Carl walks (`dir` is +1 or -1) over `from..to`.
const fn walk(from: u64, to: u64, dir: f32) -> Hold {
    Hold {
        from,
        to,
        input: RawInput {
            move_x: dir,
            ..RawInput::NONE
        },
    }
}

/// A held jump button over `from..to` (tests).
#[cfg(test)]
const fn jump(from: u64, to: u64) -> Hold {
    Hold {
        from,
        to,
        input: RawInput {
            jump: true,
            ..RawInput::NONE
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
    /// HUD-DEBT-03: the Floor 01 boss encounter runs on this fixture (the boss
    /// arena kit, `crate::boss`) with this setup. `None` on every earlier
    /// fixture: no boss.
    pub boss: Option<BossSetup>,
    /// DEMO-01 PR B: Bad Llamas. Their target ids follow the goblins'
    /// (`enemies.len() + index`). Empty on every earlier fixture: nothing of a
    /// llama is mixed into the state hash then.
    pub llamas: &'static [LlamaSpawn],
    /// DEATH-D: authored QA blast charges (empty for every routine fixture).
    pub charges: &'static [ChargeSpawn],
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
            raw.down |= hold.input.down;
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

/// Where a retired demo goblin is parked: left of the layout.
pub const RETIRED_X: f32 = -256.0;

/// DEMO-01 rules of a continuous run (see `crate::demo`). `None` on every
/// fixture, so nothing of it is mixed into a fixture hash.
#[derive(Debug, PartialEq, Eq)]
pub struct DemoRules {
    /// Respawn columns, one per beat, ascending: the checkpoint is the start
    /// column of the furthest beat Carl has entered.
    pub checkpoints: &'static [i32],
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
    boss: None,
    llamas: &[],
    charges: &[],
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
    boss: None,
    llamas: &[],
    charges: &[],
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
    boss: None,
    llamas: &[],
    charges: &[],
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
    boss: None,
    llamas: &[],
    charges: &[],
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
    boss: None,
    llamas: &[],
    charges: &[],
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
    boss: None,
    llamas: &[],
    charges: &[],
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
    boss: None,
    llamas: &[],
    charges: &[],
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
    boss: None,
    llamas: &[],
    charges: &[],
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
    boss: None,
    llamas: &[],
    charges: &[],
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
    boss: None,
    llamas: &[],
    charges: &[],
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
    boss: None,
    llamas: &[],
    charges: &[],
    script: &[attack(20, 21), attack(60, 61), attack(100, 101)],
    ticks: 200,
};

/// DEATH-B chain: Carl kicks a stationary goblin (tick 20); the kick also
/// strikes the rotten shore (hit 1) and launches the living goblin, which
/// flies into the hazard footprint and lies there. Two more jabs break the
/// shore; the block lands on the goblin, which is defeated once: cause block,
/// force catastrophic, and the kicker kept as the instigator. No teleport.
pub const CP03_KICK_BLOCK: Fixture = Fixture {
    name: "cp03_kick_block",
    summary: "a kick launches the goblin under the shore; the block crushes it, the kicker kept",
    seed: 0x0C03_0003,
    carl_x: 400,
    enemies: &[EnemySpawn {
        x: 432,
        ai: EnemyAi::Passive,
    }],
    shore_dx: Some(19),
    boss: None,
    llamas: &[],
    charges: &[],
    script: &[kick(20, 21), attack(100, 101), attack(140, 141)],
    ticks: 260,
};

/// DEATH-E1 worst case: ten stationary goblins defeated by mixed causes within
/// about 65 ticks, so the cosmetic pools fill. Three stand just ahead of Carl
/// and die to two jabs (ticks 20 and 80, defeated on 83), which also hit the
/// rotten shore (hits 1 and 2); a third jab (tick 100) breaks it. Carl then
/// turns, jabs three goblins behind him (tick 118) and kills them with a kick
/// (tick 132, lethal on 148), while the block lands on four goblins standing in
/// the footprint (tick 139). Not a gameplay design: a stress scenario for the
/// presenter, with stationary goblins so the timing does not depend on chase
/// behaviour.
pub const CP03_GOBLIN_PILEUP: Fixture = Fixture {
    name: "cp03_goblin_pileup",
    summary: "ten goblins: jab kills, a lethal kick behind Carl and the shore block, all within ~65 ticks",
    seed: 0x0C03_0004,
    carl_x: 400,
    enemies: &[
        EnemySpawn {
            x: 428,
            ai: EnemyAi::Passive,
        },
        EnemySpawn {
            x: 429,
            ai: EnemyAi::Passive,
        },
        EnemySpawn {
            x: 430,
            ai: EnemyAi::Passive,
        },
        EnemySpawn {
            x: 372,
            ai: EnemyAi::Passive,
        },
        EnemySpawn {
            x: 371,
            ai: EnemyAi::Passive,
        },
        EnemySpawn {
            x: 370,
            ai: EnemyAi::Passive,
        },
        EnemySpawn {
            x: 470,
            ai: EnemyAi::Passive,
        },
        EnemySpawn {
            x: 488,
            ai: EnemyAi::Passive,
        },
        EnemySpawn {
            x: 500,
            ai: EnemyAi::Passive,
        },
        EnemySpawn {
            x: 514,
            ai: EnemyAi::Passive,
        },
    ],
    shore_dx: Some(19),
    boss: None,
    llamas: &[],
    charges: &[],
    script: &[
        attack(20, 21),
        attack(80, 81),
        attack(100, 101),
        strike_towards(118, 119, -1.0, false),
        strike_towards(132, 133, -1.0, true),
    ],
    ticks: 260,
};

/// DEMO-01 PR B: Carl kicks a Bad Llama to death. The llama (health 3) starts
/// its tell at once; the first kick lands in the tell and cancels the spit,
/// Carl follows the pushed-back llama and kicks twice more. The lethal blow is
/// a push kick: `force=heavy`, so the presenter plays the LLAMA-02 slam pair
/// (`slam_contact`, `broken_heap`) (slam rule, owner decision 2026-11-19).
pub const CP05_LLAMA_SLAM: Fixture = Fixture {
    name: "cp05_llama_slam",
    summary: "three push kicks defeat the Bad Llama: the lethal kick is heavy (slam finish)",
    seed: 0x0C05_0001,
    carl_x: 400,
    enemies: &[],
    shore_dx: None,
    boss: None,
    llamas: &[LlamaSpawn {
        x: 440,
        ai: EnemyAi::Chase,
    }],
    charges: &[],
    script: &[
        kick(20, 21),
        walk(60, 80, 1.0),
        kick(90, 91),
        walk(125, 140, 1.0),
        kick(150, 151),
    ],
    ticks: 260,
};

/// DEMO-01 PR B: jab, haymaker, jab defeat a Bad Llama. No lethal hit is a
/// kick, so the force is routine and the presenter keeps the LLAMA-01 `down`.
pub const CP05_LLAMA_DOWN: Fixture = Fixture {
    name: "cp05_llama_down",
    summary: "jab, haymaker, jab defeat the Bad Llama: a routine lethal hit (plain down)",
    seed: 0x0C05_0002,
    carl_x: 400,
    enemies: &[],
    shore_dx: None,
    boss: None,
    llamas: &[LlamaSpawn {
        x: 438,
        ai: EnemyAi::Chase,
    }],
    charges: &[],
    script: &[attack(20, 21), attack(24, 25), attack(60, 61)],
    ticks: 200,
};

/// DEMO-01 PR B: Carl idles inside the frame, 160 px from the llama. It tells
/// (36 ticks of glow), spits, and the glob hits him; the second spit comes
/// after the cooldown and after his invulnerability, and hits again.
pub const CP05_LLAMA_SPIT: Fixture = Fixture {
    name: "cp05_llama_spit",
    summary: "Carl idles in view: two spit hits, the second after invulnerability",
    seed: 0x0C05_0003,
    carl_x: 400,
    enemies: &[],
    shore_dx: None,
    boss: None,
    llamas: &[LlamaSpawn {
        x: 560,
        ai: EnemyAi::Chase,
    }],
    charges: &[],
    script: &[],
    ticks: 300,
};

/// DEMO-01 PR B: the llama stands outside the simulation's scroll-camera view
/// (fair fire): it walks toward Carl up to its leash but never starts a tell.
pub const CP05_LLAMA_OFFSCREEN: Fixture = Fixture {
    name: "cp05_llama_offscreen",
    summary: "the llama is outside the scroll view: it walks, never tells, never spits",
    seed: 0x0C05_0004,
    carl_x: 400,
    enemies: &[],
    shore_dx: None,
    boss: None,
    llamas: &[LlamaSpawn {
        x: 1000,
        ai: EnemyAi::Chase,
    }],
    charges: &[],
    script: &[],
    ticks: 400,
};

/// DEMO-01 PR B: three jabs break the rotten shore and the block lands on a
/// stationary llama standing in the footprint (a crush: the stone is drawn
/// over the llama on the first two contact frames, below the players).
pub const CP05_LLAMA_CRUSH: Fixture = Fixture {
    name: "cp05_llama_crush",
    summary: "three jabs break the shore; the block crushes the llama under it",
    seed: 0x0C05_0005,
    carl_x: 400,
    enemies: &[],
    shore_dx: Some(19),
    boss: None,
    llamas: &[LlamaSpawn {
        x: 484,
        ai: EnemyAi::Passive,
    }],
    charges: &[],
    script: &[attack(20, 21), attack(60, 61), attack(100, 101)],
    ticks: 200,
};

/// DEATH-D: `cp03_shore_kill` with the goblin standing on the footprint's
/// right edge, facing the block: its head (4 px ahead of the pivot) is under
/// the stone and its torso (4 px behind) is outside it, so the landing shears
/// the neck. The one authoritative decapitation fixture; the committed log has
/// `cause=shore_block_edge`.
pub const CP03_BLOCK_EDGE_DECAP: Fixture = Fixture {
    name: "cp03_block_edge_decap",
    summary: "three jabs break the shore; the block's edge comes down across the goblin's neck",
    seed: 0x0C03_0004,
    carl_x: 400,
    enemies: &[EnemySpawn {
        x: 524,
        ai: EnemyAi::Passive,
    }],
    shore_dx: Some(19),
    boss: None,
    llamas: &[],
    charges: &[],
    script: &[attack(20, 21), attack(60, 61), attack(100, 101)],
    ticks: 200,
};

/// DEATH-D QA fixture (never a route, not an ability): an authored blast
/// charge at x 520 detonates on tick 60 with three stationary goblins near it.
/// Goblin 0 stands on the charge (point blank: the burst), goblin 1 is 16 px
/// from the blast (thrown dead, intact), goblin 2 is 48 px away and untouched.
pub const QA_BLAST_BREAKUP: Fixture = Fixture {
    name: "qa_blast_breakup",
    summary: "QA charge at x 520: point blank bursts, the neighbour is thrown, the third is untouched",
    seed: 0x0D0D_0001,
    carl_x: 400,
    enemies: &[
        EnemySpawn {
            x: 520,
            ai: EnemyAi::Passive,
        },
        EnemySpawn {
            x: 548,
            ai: EnemyAi::Passive,
        },
        EnemySpawn {
            x: 580,
            ai: EnemyAi::Passive,
        },
    ],
    shore_dx: None,
    boss: None,
    llamas: &[],
    charges: &[ChargeSpawn {
        dx: 120,
        tick: 60,
        owner: None,
        arm: Arm::Tick,
    }],
    script: &[],
    ticks: 240,
};

pub const FIXTURES: [&Fixture; 20] = [
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
    &CP03_KICK_BLOCK,
    &CP03_GOBLIN_PILEUP,
    &CP03_BLOCK_EDGE_DECAP,
    &QA_BLAST_BREAKUP,
    &CP05_LLAMA_SLAM,
    &CP05_LLAMA_DOWN,
    &CP05_LLAMA_SPIT,
    &CP05_LLAMA_OFFSCREEN,
    &CP05_LLAMA_CRUSH,
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

/// One hero's hit-reaction state (Donut's; Carl keeps the original fields of
/// [`CombatState`] so every legacy hash and test is unchanged).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HeroCombat {
    /// Ticks of hurt flinch left; the hero cannot act or move.
    pub hurt_ticks: u8,
    /// Ticks the hero stays frozen after a goblin's blow landed (hit-stop).
    pub hit_stop: u8,
    /// Knockdown drift in px/s after the defeating blow.
    pub vx: f32,
}

/// Combat layered on a [`SceneSim`]: goblins, Carl's hurt/defeat/restart state
/// and the event log. The hero's move clock, health and invulnerability live
/// on the scene's own `Hero` (shared with the greybox).
#[derive(Debug, Clone)]
pub struct CombatState {
    pub fixture: &'static Fixture,
    pub enemies: Vec<Goblin>,
    /// DEMO-01 PR B: the Bad Llamas (target ids follow the goblins') and the
    /// spit globs in flight.
    pub llamas: Vec<Llama>,
    pub globs: Vec<Glob>,
    /// CP-03: the rotten shore and its block (fixtures with `shore_dx`).
    pub shore: Option<Shore>,
    /// HUD-DEBT-03: the Floor 01 boss encounter (boss fixtures only; `None`
    /// everywhere else, so no earlier route, fixture or hash changes).
    pub boss: Option<Boss>,
    /// DEATH-D: the fixture's authored blast charges (QA only).
    pub charges: Vec<BlastCharge>,
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
    /// Donut's hit state while the party is live (idle otherwise). Carl's
    /// state stays in the fields above; see [`Self::hurt_ticks_of`].
    pub donut: HeroCombat,
    /// WORK-023, HUD-DEBT-02: Donut's swat, pounce, Magic Missile and mana. It
    /// runs only while the party is live and `PartyTuning::donut_moves` is on,
    /// and mixes nothing into the hash while she never used an ability.
    donut_kit: DonutKit,
    /// COMPANION-AI-01 runtime state (CF02-00: an empty no-op stub).
    companion: CompanionRuntime,
    /// COMBAT-FUN-02: the pinned tuning with `cf02_rules` on, `Some` only for a run
    /// under `--cf02` and for every `cf02_*` fixture; `None` is the legacy game
    /// and every golden. Constant for the run, so it is not part of the hash.
    cf02: Option<&'static Cf02Tuning>,
    /// CF02-H: Carl's move buffer and airborne state (default, and not hashed,
    /// without `cf02_rules`).
    carl_cf02: CarlCf02,
    /// Which hero's defeat ended the attempt (D1: either one is the party's
    /// defeat). Meaningful while `hero_defeated`.
    pub defeated_hero: HeroId,
    /// Q3: the hero each goblin is chasing (live party only; `None` until its
    /// first target is chosen).
    targets: Vec<Option<HeroId>>,
    /// Ticks since the party was defeated.
    pub defeated_ticks: u16,
    pub deaths: u32,
    /// Restarts so far (a defeat's automatic one and manual ones). Not hashed:
    /// the party layer watches it to return the swap to Carl.
    pub restarts: u32,
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
    /// DEMO-01 rules (`None` for every fixture) and the furthest beat entered.
    demo: Option<&'static DemoRules>,
    pub checkpoint: usize,
    // Presentation cache (not hashed, never read by the sim).
    confirm: Option<Confirm>,
    flash_tick: Vec<Option<u64>>,
    respawned: bool,
}

impl CombatState {
    /// Place the fixture's goblins on Carl's walking line. Fails when a goblin
    /// column is not on the same line (the fixture promises clean ground).
    pub fn new(fixture: &'static Fixture, kit: &EnvKit, scene: &SceneSim) -> Result<Self, String> {
        let line = kit
            .walking_line_at(fixture.carl_x)
            .ok_or_else(|| format!("{}: no walking line under Carl", fixture.name))?;
        for charge in fixture.charges {
            if charge.tick == 0 && charge.arm == Arm::Tick {
                return Err(format!(
                    "{}: a charge on tick 0 never fires (ticks count from 1)",
                    fixture.name
                ));
            }
        }
        for spawn in fixture.enemies {
            if kit.walking_line_at(spawn.x) != Some(line) {
                return Err(format!(
                    "{}: goblin column {} is not on Carl's walking line",
                    fixture.name, spawn.x
                ));
            }
        }
        for spawn in fixture.llamas {
            if kit.walking_line_at(spawn.x) != Some(line) {
                return Err(format!(
                    "{}: llama column {} is not on Carl's walking line",
                    fixture.name, spawn.x
                ));
            }
        }
        let mut state = Self {
            fixture,
            enemies: Vec::new(),
            llamas: Vec::new(),
            globs: Vec::new(),
            shore: None,
            boss: None,
            charges: Vec::new(),
            contacts: Vec::new(),
            input: InputFrame::default(),
            hurt_ticks: 0,
            hero_hit_stop: 0,
            hero_defeated: false,
            hero_vx: 0.0,
            donut: HeroCombat::default(),
            donut_kit: DonutKit::new(DonutSetup::NONE),
            companion: CompanionRuntime::default(),
            cf02: fixture.name.starts_with("cf02_").then(cf02::active),
            carl_cf02: CarlCf02::default(),
            defeated_hero: HeroId::Carl,
            targets: Vec::new(),
            defeated_ticks: 0,
            deaths: 0,
            restarts: 0,
            last_tell_start: None,
            elapsed: 0,
            events: Vec::new(),
            dropped_events: 0,
            event_hash: EVENT_HASH_SEED,
            cursor: 0,
            enemy_foot_y: scene.world.hero_foot_y(),
            x_max: kit.width as f32,
            demo: None,
            checkpoint: 0,
            confirm: None,
            flash_tick: Vec::new(),
            respawned: false,
        };
        state.reset_attempt();
        Ok(state)
    }

    /// DEMO-01: run this state under the demo rules (checkpoint respawn).
    #[must_use]
    pub fn with_demo(mut self, rules: &'static DemoRules) -> Self {
        self.demo = Some(rules);
        self.reset_attempt();
        self
    }

    /// COMBAT-FUN-02: run this state under `cf02_rules` (`--cf02`). The `cf02_*`
    /// fixtures already do (by their name). Every CF02 rule reads the tuning this
    /// sets, so a state without it is the legacy game byte for byte.
    #[must_use]
    pub fn with_cf02(mut self) -> Self {
        self.cf02 = Some(cf02::active());
        self.reset_attempt();
        self
    }

    /// The CF02 tuning this run reads (`None`: `cf02_rules` is off).
    pub fn cf02(&self) -> Option<&'static Cf02Tuning> {
        self.cf02
    }

    /// A goblin of this run at column `x` (it carries the CF02 tuning, if any).
    fn new_goblin(&self, x: f32, ai: EnemyAi) -> Goblin {
        let mut goblin =
            Goblin::new(x, self.enemy_foot_y + ENEMY_HALF_HEIGHT, ai).with_cf02(self.cf02);
        if self.cf02.is_some()
            && let Some(health) = crate::cf02_fixtures::goblin_health(self.fixture.name)
        {
            goblin.health = health;
        }
        goblin
    }

    /// The beat whose checkpoint is the fixture's start column (0 without
    /// demo rules).
    fn start_checkpoint(&self) -> usize {
        self.demo.map_or(0, |demo| {
            demo.checkpoints
                .iter()
                .rposition(|&column| column <= self.fixture.carl_x)
                .unwrap_or(0)
        })
    }

    /// DEMO-01: true once after a respawn (the presentation resets Carl's
    /// animation). Presentation only, never hashed.
    pub fn take_respawned(&mut self) -> bool {
        std::mem::take(&mut self.respawned)
    }

    /// DEMO-01: Carl stands up at the last checkpoint with full health. Defeated
    /// enemies stay defeated (their corpse and gore stay); every living goblin
    /// goes back to its spawn. The
    /// tick counter and the event log keep running. Does nothing without
    /// demo rules.
    pub fn respawn(&mut self, scene: &mut SceneSim) {
        let Some(demo) = self.demo else {
            return;
        };
        let checkpoint = self.checkpoint.min(demo.checkpoints.len() - 1);
        scene.respawn_at(demo.checkpoints[checkpoint] as f32);
        for (index, spawn) in self.fixture.enemies.iter().enumerate() {
            if self.enemies[index].health > 0 {
                self.enemies[index] = self.new_goblin(spawn.x as f32, spawn.ai);
                self.flash_tick[index] = None;
            }
        }
        // A test charge that has not blown goes back to its unlit state with
        // the goblins that huddle on it; one that blew stays blown.
        for (index, spawn) in self.fixture.charges.iter().enumerate() {
            if !self.charges[index].detonated {
                self.charges[index] = self.charge_of(spawn);
            }
        }
        self.carl_cf02 = CarlCf02::default();
        self.input = InputFrame::default();
        self.hurt_ticks = 0;
        self.hero_hit_stop = 0;
        self.hero_defeated = false;
        self.hero_vx = 0.0;
        self.donut = HeroCombat::default();
        self.defeated_hero = HeroId::Carl;
        self.targets = vec![None; self.enemies.len()];
        self.defeated_ticks = 0;
        self.last_tell_start = None;
        self.confirm = None;
        self.restart_donut(RestartKind::CheckpointRestart);
        self.companion.reset();
        self.respawned = true;
        self.log(CombatEvent::Respawn {
            tick: scene.world.tick,
            checkpoint: checkpoint as u32,
        });
    }

    /// A fresh charge of the fixture's spawn (unlit, not detonated).
    fn charge_of(&self, spawn: &ChargeSpawn) -> BlastCharge {
        BlastCharge::new(
            (self.fixture.carl_x + spawn.dx) as f32,
            self.enemy_foot_y,
            spawn.tick,
            spawn.owner,
        )
        .with_arm(spawn.arm)
    }

    fn reset_attempt(&mut self) {
        self.enemies = self
            .fixture
            .enemies
            .iter()
            .map(|spawn| self.new_goblin(spawn.x as f32, spawn.ai))
            .collect();
        self.llamas = self
            .fixture
            .llamas
            .iter()
            .map(|spawn| {
                Llama::new(
                    spawn.x as f32,
                    self.enemy_foot_y + LLAMA_HALF_HEIGHT,
                    spawn.ai,
                )
            })
            .collect();
        self.globs.clear();
        self.shore = self
            .fixture
            .shore_dx
            .map(|dx| Shore::new((self.fixture.carl_x + dx) as f32, self.enemy_foot_y));
        self.boss = self.fixture.boss.map(|setup| setup.boss(self.enemy_foot_y));
        self.charges = self
            .fixture
            .charges
            .iter()
            .map(|spawn| self.charge_of(spawn))
            .collect();
        self.contacts.clear();
        self.carl_cf02 = CarlCf02::default();
        self.input = InputFrame::default();
        self.hurt_ticks = 0;
        self.hero_hit_stop = 0;
        self.hero_defeated = false;
        self.hero_vx = 0.0;
        self.donut = HeroCombat::default();
        self.defeated_hero = HeroId::Carl;
        self.targets = vec![None; self.enemies.len()];
        self.defeated_ticks = 0;
        self.last_tell_start = None;
        self.event_hash = EVENT_HASH_SEED;
        self.confirm = None;
        self.checkpoint = self.start_checkpoint();
        self.flash_tick = vec![None; self.enemies.len() + self.llamas.len()];
        // CF02 QA hook: a fixture goblin that starts launched.
        if self.cf02.is_some()
            && let Some((vx, vy)) = crate::cf02_fixtures::goblin_launch(self.fixture.name)
            && let Some(goblin) = self.enemies.first_mut()
        {
            goblin.mode = GoblinMode::Launched;
            goblin.vx = vx;
            goblin.vy = vy;
            // Instance 0 is no real move instance (they start at 1).
            goblin.launch = Some(0);
        }
        self.retire_goblins_behind_start();
    }

    /// DEMO-01, a run that starts at a later beat: the goblins of the beats
    /// behind it are not part of the run. They are retired (health 0, corpse,
    /// parked outside the layout, never drawn), so they neither chase Carl nor
    /// keep the run from completing. Nothing happens on a fixture or at beat 0.
    fn retire_goblins_behind_start(&mut self) {
        let Some(demo) = self.demo else {
            return;
        };
        let start = demo.checkpoints[self.checkpoint];
        for (goblin, spawn) in self.enemies.iter_mut().zip(self.fixture.enemies) {
            if spawn.x < start {
                goblin.health = 0;
                goblin.mode = GoblinMode::Corpse;
                goblin.ai = EnemyAi::Passive;
                goblin.x = RETIRED_X;
            }
        }
        // The llamas of the beats behind the start are retired the same way.
        for (llama, spawn) in self.llamas.iter_mut().zip(self.fixture.llamas) {
            if spawn.x < start {
                llama.health = 0;
                llama.mode = LlamaMode::Corpse;
                llama.ai = EnemyAi::Passive;
                llama.x = RETIRED_X;
            }
        }
    }

    /// Fast restart of the fixture: Carl, the goblins and the tick counter go
    /// back to the start. `deaths` and the event log survive. The `Restart`
    /// event carries the tick the finished attempt ended on (the triggering
    /// tick); the attempt that follows counts its ticks from 1 again.
    pub fn restart(&mut self, scene: &mut SceneSim) {
        let start = self.donut_start();
        self.restart_with(scene, RestartKind::Fixture(start));
    }

    /// [`Self::restart`] of a given kind: the party's defeat retry keeps
    /// Donut's unlock and refills her mana, a fixture restart restores the
    /// declared state (mana v0 section 3).
    fn restart_with(&mut self, scene: &mut SceneSim, kind: RestartKind) {
        // Logged but not folded into the attempt's event hash, so a replayed
        // attempt after a restart hashes like a fresh one.
        self.push_event(CombatEvent::Restart {
            tick: scene.world.tick,
            deaths: self.deaths,
        });
        scene.reset();
        self.reset_attempt();
        self.restart_donut(kind);
        self.companion.reset();
        self.sync_arena(scene);
        self.restarts += 1;
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

    /// One fixed tick of the single-hero fixtures: `raw` is Carl's (or the
    /// script's) input and Donut's slot is idle. Identical to
    /// [`Self::step_party`] with the legacy drive.
    pub fn step(&mut self, scene: &mut SceneSim, raw: RawInput) {
        self.step_party(scene, [raw, RawInput::default()]);
    }

    /// One fixed tick for the party. `inputs` is indexed by hero (`[Carl,
    /// Donut]`, see [`crate::party::PartyState::hero_inputs`]). With the legacy
    /// scene drive only `inputs[0]` is read and nothing about Donut changes.
    ///
    /// Order within a tick (the same every tick): Carl's move clock and strike
    /// on the goblins (Carl wins a same-tick trade), the scene step (the
    /// heroes' physics, the camera), then every goblin in index order (attack
    /// tokens, tell, swing, and the blow on its target), the contacts, the
    /// shore, then the blast charges in index order. Donut's swat, pounce and
    /// Magic Missile run after Carl's strike and before the scene step (Carl,
    /// Donut, goblins).
    pub fn step_party(&mut self, scene: &mut SceneSim, inputs: [RawInput; 2]) {
        let inputs = self.companion_inputs(scene, inputs);
        let raw = inputs[0];
        let live = scene.party_live();
        let tick = scene.world.tick + 1;
        self.elapsed += 1;
        self.input.advance(raw);
        // Hit-stop freezes a hero's timers: his flinch, his invulnerability and
        // (through the move clock) his move.
        let hero_frozen = self.hero_hit_stop > 0;
        if hero_frozen {
            self.hero_hit_stop -= 1;
        } else {
            self.hurt_ticks = self.hurt_ticks.saturating_sub(1);
            if self.hurt_ticks == 0 && !self.hero_defeated {
                self.hero_vx = 0.0;
            }
            scene.world.hero.invulnerable_ticks =
                scene.world.hero.invulnerable_ticks.saturating_sub(1);
        }
        let donut_frozen = live && self.donut.hit_stop > 0;
        if live {
            if donut_frozen {
                self.donut.hit_stop -= 1;
            } else {
                self.donut.hurt_ticks = self.donut.hurt_ticks.saturating_sub(1);
                if self.donut.hurt_ticks == 0 && !self.hero_defeated {
                    self.donut.vx = 0.0;
                }
                scene.world.donut_body.invulnerable_ticks =
                    scene.world.donut_body.invulnerable_ticks.saturating_sub(1);
            }
        }
        let melee_frozen =
            scene.world.hero.melee.busy() && scene.world.hero.melee.hit_stop_left() > 0;

        let can_act = !self.hero_defeated && self.hurt_ticks == 0 && !hero_frozen;
        // Attack is the jab (a second press inside its window is the
        // haymaker); PhysicalAttack (Triangle/Y) is the push kick. The
        // EquippedSpecial (Circle/B) has no effect before its story unlock
        // (there are no explosives before the Guild). Both only start from a
        // press edge, so a held button never repeats.
        let plan = if self.cf02.is_some() {
            // CF02-H (Q46..Q54): the move buffer, the same-tick rule, the cancel
            // table, the air moves and the bail-out decide the request.
            self.plan_carl_cf02(scene, raw, can_act, hero_frozen || melee_frozen)
        } else {
            CarlCf02Plan::default()
        };
        let request = if self.cf02.is_some() {
            plan.request
        } else {
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
            request
        };
        let buffer_ticks = self
            .cf02
            .map_or(CARL_BUFFER_TICKS, crate::input_buffer::runner_buffer_ticks);
        let move_tick = scene.world.hero.melee.step(request, buffer_ticks, can_act);
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
                player: PLAYER_ONE,
            };
            let attacking: Vec<bool> = self.enemies.iter().map(Goblin::token_held).collect();
            let telling: Vec<bool> = self.llamas.iter().map(Llama::telling).collect();
            let mut hits = Vec::new();
            let glance = self
                .companion
                .strike_glance(HeroId::Carl, move_tick.instance);
            // One move over every kind of target: the goblins (ids from 0),
            // then the llamas (ids after them). The attacker's hit-stop is the
            // longest of the hits, started once.
            let mut stop = strike_group(
                &mut scene.world.hero.melee,
                &move_tick,
                attacker,
                &mut self.enemies,
                0,
                tick,
                &mut hits,
                &|spec, hp| glance.apply(spec, hp),
            );
            if !self.llamas.is_empty() {
                stop = stop.max(strike_group(
                    &mut scene.world.hero.melee,
                    &move_tick,
                    attacker,
                    &mut self.llamas,
                    self.enemies.len() as u32,
                    tick,
                    &mut hits,
                    &|spec, hp| glance.apply(spec, hp),
                ));
            }
            scene.world.hero.melee.begin_hit_stop(stop);
            for event in hits {
                if let CombatEvent::Hit {
                    move_id,
                    instance,
                    target,
                    health_left,
                    ..
                } = event
                {
                    if let Some(llama) = self
                        .llamas
                        .get_mut((target as usize).wrapping_sub(self.enemies.len()))
                    {
                        llama.hit_by = Some(attacker.player);
                    } else {
                        // A kick that launched a living goblin: stamp the move
                        // instance as the launch id (one contact per launch).
                        let goblin = &mut self.enemies[target as usize];
                        if goblin.mode == GoblinMode::Launched && goblin.launch.is_none() {
                            goblin.launch = Some(instance);
                        }
                        goblin.hit_by = Some(attacker.player);
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
            for (index, was_telling) in telling.into_iter().enumerate() {
                if was_telling && self.llamas[index].mode != LlamaMode::Attack {
                    self.log(CombatEvent::LlamaInterrupted {
                        tick,
                        llama: index as u32,
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
            if self.boss.is_some() {
                // Hero move on the boss (the runner is on the scene's hero).
                let mut runner = std::mem::take(&mut scene.world.hero.melee);
                self.strike_boss(&mut runner, &move_tick, attacker, tick, &|spec, hp| {
                    glance.apply(spec, hp)
                });
                scene.world.hero.melee = runner;
            }
        }

        // CF02-H: the dive, the air kick and the bail-out hop drive Carl's body.
        let carl_drive = self.carl_cf02_drive(scene, move_tick);
        // Committed on the ground: the feet stay planted and jumping is off (CF02:
        // also during a landing recovery).
        let committed =
            (scene.world.hero.melee.busy() && scene.world.hero.grounded) || plan.planted;
        let gated = if !can_act {
            RawInput::default()
        } else if committed {
            RawInput {
                move_x: 0.0,
                jump: false,
                ..raw
            }
        } else if plan.strip_jump {
            RawInput { jump: false, ..raw }
        } else {
            raw
        };
        // A boss blow launches a hero (`hero_vx`); in every earlier fixture the
        // drift runs only after the defeating blow, as before.
        if (self.hero_defeated || self.hero_vx != 0.0) && !hero_frozen {
            self.drift_hero(scene);
        }
        if live {
            if (self.hero_defeated || self.donut.vx != 0.0) && !donut_frozen {
                self.drift_donut(scene);
            }
            // Donut's swat, pounce, Magic Missile and mana (Q10, WORK-023) run
            // here, after Carl's strike and before the physics. Without
            // `donut_moves` only her movement and jump are read, and only while
            // she can act.
            let donut_can_act = !self.hero_defeated && self.donut.hurt_ticks == 0 && !donut_frozen;
            let donut_tick = self.step_donut(scene, inputs[1], donut_can_act, tick);
            let threats: Vec<Threat> = self
                .enemies
                .iter()
                .filter(|g| g.mode == GoblinMode::Attack && g.attack_tick < IMPACT_END)
                .map(|g| Threat {
                    x: g.x,
                    attack_tick: g.attack_tick,
                })
                .collect();
            let mut ctx = PartyStepCtx {
                can_move: [can_act && !committed, donut_tick.can_move],
                frozen: [hero_frozen || melee_frozen, donut_frozen],
                threats: &threats,
                boss: self.boss.as_ref().and_then(Boss::threat),
                drives: Default::default(),
            };
            Self::apply_donut_tick(&mut ctx, &donut_tick);
            ctx.drives[0] = carl_drive;
            scene.step_party_ctx([gated, donut_tick.raw], &ctx);
        } else if hero_frozen || melee_frozen {
            // Frozen in hit-stop: only the tick counter advances. Carl, the
            // camera and Donut's trail stand still (no motion to restore).
            scene.hold_tick(gated);
        } else if self.cf02.is_some() {
            let ctx = PartyStepCtx {
                drives: [carl_drive, BodyDrive::default()],
                ..PartyStepCtx::default()
            };
            scene.step_party_ctx([gated, RawInput::default()], &ctx);
        } else {
            scene.step(gated);
        }

        self.step_demo(scene);
        self.step_goblins(scene, tick);
        self.step_llamas(scene, tick);
        self.step_globs(scene, tick);
        self.resolve_contacts(tick);
        self.step_shore(tick);
        self.step_boss(scene, tick);
        self.step_charges(tick, scene.world.hero.x);

        if self.hero_defeated {
            self.defeated_ticks = self.defeated_ticks.saturating_add(1);
            if self.defeated_ticks >= RESTART_DELAY_TICKS {
                if self.demo.is_some() {
                    self.respawn(scene);
                } else {
                    self.restart_with(scene, RestartKind::DefeatRetry);
                }
            }
        }
    }

    /// The boss as the HUD reads it (`None` outside a boss fixture).
    pub fn boss_state(&self) -> Option<BossState> {
        self.boss.as_ref().map(Boss::state)
    }

    /// A hero's melee against the boss: the active box on its hurtbox (one hit
    /// per move instance) and on the rack strut. Carl's move runs first, so he
    /// wins a same-tick trade. The kick never launches the boss.
    fn strike_boss(
        &mut self,
        runner: &mut MoveRunner,
        move_tick: &MoveTick,
        attacker: Attacker,
        tick: u64,
        glance: &dyn Fn(&MoveSpec, u8) -> MoveSpec,
    ) {
        let Some(mut boss) = self.boss.take() else {
            return;
        };
        if move_tick.phase == Phase::Active
            && !move_tick.frozen
            && let Some(boxed) = hit_box(&move_tick.spec.shape, &attacker)
        {
            let mut events = Vec::new();
            boss.strike_strut(
                move_tick.instance,
                boxed,
                attacker.player,
                tick,
                &mut events,
            );
            let reaches = boss.hurtbox().is_some_and(|b| b.overlaps(boxed));
            let landed = glance(move_tick.spec, u8::try_from(boss.hp).unwrap_or(u8::MAX));
            if reaches
                && runner.record_hit(BOSS_ENEMY_ID)
                && let Some(result) = boss.apply_hit(
                    BossHit {
                        damage: u16::from(landed.damage),
                        poise: u16::from(move_tick.spec.damage),
                        player: attacker.player,
                        hit_stop: move_tick.spec.reaction.hit_stop,
                    },
                    &mut events,
                    tick,
                )
            {
                runner.begin_hit_stop(move_tick.spec.reaction.hit_stop);
                let bonus = if result.defeated {
                    LETHAL_CAMERA_KICK_BONUS
                } else {
                    0
                };
                self.confirm = Some(Confirm {
                    tick,
                    kick_px: move_tick.spec.reaction.camera_kick_px + bonus,
                    direction: attacker.facing,
                });
                if result.defeated {
                    events.push(CombatEvent::enemy_defeated(
                        tick,
                        BOSS_ENEMY_ID,
                        DefeatCause::Melee,
                        Some(attacker.player),
                        Some(move_tick.spec),
                    ));
                }
            }
            for event in events {
                self.log(event);
            }
        }
        self.boss = Some(boss);
    }

    /// One boss tick after the heroes and the goblins: build the boss' view of
    /// the heroes, step it, and apply what it asks for (events, blows, pushes).
    fn step_boss(&mut self, scene: &mut SceneSim, tick: u64) {
        let Some(mut boss) = self.boss.take() else {
            return;
        };
        let drive = *scene.drive();
        let live = scene.party_live();
        let heroes = HeroId::ALL.map(|id| {
            let body = scene.world.body(id);
            let present = !self.hero_defeated && (live || id == HeroId::Carl);
            HeroView {
                id,
                present,
                x: body.x,
                foot_y: body.y - HERO_HALF_HEIGHT,
                grounded: body.grounded,
                hurt: Self::hurtbox(scene, id),
                vulnerable: present && body.invulnerable_ticks == 0,
                bot: drive.is_bot(id),
            }
        });
        let out = boss.step(&BossCtx {
            heroes,
            solids: &scene.world.solids,
            solo: live && !drive.two_player,
            tick,
        });
        self.apply_boss_out(scene, &boss, out, tick);
        crate::boss_scene::sync_arena_solids(&mut scene.world.solids, Some(&boss));
        self.boss = Some(boss);
    }

    fn apply_boss_out(&mut self, scene: &mut SceneSim, boss: &Boss, out: BossOut, tick: u64) {
        for event in out.events {
            self.log(event);
        }
        for (id, x) in out.shifts {
            scene.world.body_mut(id).x = x;
        }
        if let Some(id) = out.regroup {
            let regroup_x = boss.data().gate.regroup_x;
            let leader_y = scene.world.body(id.other()).y;
            let body = scene.world.body_mut(id);
            body.x = regroup_x;
            body.y = leader_y;
            body.vx = 0.0;
            body.vy = 0.0;
            body.grounded = false;
        }
        for blow in out.blows {
            self.hurt_hero(scene, blow, tick);
        }
        if let Some((player, rack)) = out.crushed_by {
            self.log(CombatEvent::enemy_defeated(
                tick,
                BOSS_ENEMY_ID,
                DefeatCause::DropHazard { player, rack },
                Some(player),
                None,
            ));
        }
    }

    /// Put the arena's gate and exit wall where the boss says they are (after
    /// a restart the gate is open and the wall back).
    fn sync_arena(&self, scene: &mut SceneSim) {
        if self.fixture.boss.is_some() {
            crate::boss_scene::sync_arena_solids(&mut scene.world.solids, self.boss.as_ref());
        }
    }

    /// A blow of the boss or its rack on `blow.hero`: the same damage path as
    /// a goblin's swing (invulnerability, the solo last-HP guard, the hurt and
    /// knockdown states, party defeat), plus the launch.
    fn hurt_hero(&mut self, scene: &mut SceneSim, blow: HeroBlow, tick: u64) -> bool {
        let id = blow.hero;
        let drive = *scene.drive();
        let tuning = drive.tuning;
        let guarded = drive.is_guarded(id);
        let hero = scene.world.body_mut(id);
        if self.hero_defeated || hero.invulnerable_ticks > 0 {
            return false;
        }
        let (invulnerable, hurt_for) = match id {
            HeroId::Carl => (HERO_INVULNERABLE_TICKS, HERO_HURT_TICKS),
            HeroId::Donut => (tuning.donut_invulnerable_ticks, tuning.donut_hurt_ticks),
        };
        let floor = u8::from(guarded && tuning.solo_last_hp_guard);
        hero.health = hero
            .health
            .saturating_sub(blow.damage)
            .max(floor.min(hero.health));
        hero.invulnerable_ticks = invulnerable;
        hero.melee.cancel();
        hero.facing = if blow.from_x >= hero.x { 1 } else { -1 };
        let away = if hero.x >= blow.from_x { 1.0 } else { -1.0 };
        let health_left = hero.health;
        let mut state = self.hero_state(id);
        state.hurt_ticks = hurt_for;
        state.hit_stop = state.hit_stop.max(HERO_HURT_HIT_STOP);
        self.log(CombatEvent::HeroHurt {
            tick,
            source: blow.source,
            health_left,
            hero: id,
        });
        if id == HeroId::Donut {
            scene.world.donut.health = health_left;
        }
        let hero = scene.world.body_mut(id);
        if health_left == 0 {
            self.hero_defeated = true;
            self.defeated_hero = id;
            self.deaths += 1;
            state.hit_stop = state.hit_stop.max(HERO_DEFEAT_HIT_STOP);
            state.vx = away * HERO_KNOCKDOWN_VX;
            hero.vy = HERO_KNOCKDOWN_VY;
            hero.grounded = false;
            self.log(CombatEvent::HeroDefeated { tick, hero: id });
        } else if blow.knock_vy > 0.0 {
            // The launch: lifted off the ground, drifting away from the blow.
            hero.vy = blow.knock_vy;
            hero.grounded = false;
            state.vx = blow.knock_vx;
        }
        self.set_hero_state(id, state);
        true
    }

    /// DEMO-01 (no-op without demo rules): advance the checkpoint to the
    /// furthest beat Carl has entered.
    fn step_demo(&mut self, scene: &SceneSim) {
        let Some(demo) = self.demo else {
            return;
        };
        let hero_x = scene.world.hero.x;
        if !self.hero_defeated {
            while self.checkpoint + 1 < demo.checkpoints.len()
                && hero_x >= demo.checkpoints[self.checkpoint + 1] as f32
            {
                self.checkpoint += 1;
            }
        }
    }

    /// Q3: choose the hero each goblin chases (live party only). A goblin in an
    /// attack keeps its target through the tell, the swing and the recovery, so
    /// the tell always points at the hero who will be hit. Otherwise it takes
    /// the nearest hero, switching only when the other one is
    /// `target_switch_px` closer; in solo the uncontrolled hero counts
    /// `target_ai_bias_px` farther away. Ties go to Carl.
    fn update_targets(&mut self, scene: &SceneSim) {
        let drive = *scene.drive();
        let tuning = drive.tuning;
        let effective = |goblin_x: f32, hero: HeroId| {
            let bias = if drive.is_guarded(hero) {
                f32::from(tuning.target_ai_bias_px)
            } else {
                0.0
            };
            (goblin_x - scene.world.body(hero).x).abs() + bias
        };
        for (index, goblin) in self.enemies.iter().enumerate() {
            let current = self.targets[index];
            if goblin.mode == GoblinMode::Attack && current.is_some() {
                continue;
            }
            let (carl, donut) = (
                effective(goblin.x, HeroId::Carl),
                effective(goblin.x, HeroId::Donut),
            );
            let nearest = if donut < carl {
                HeroId::Donut
            } else {
                HeroId::Carl
            };
            self.targets[index] = Some(match (tuning.enemy_target, current) {
                (EnemyTargetRule::HysteresisControlledBias, Some(held)) => {
                    let (kept, other) =
                        (effective(goblin.x, held), effective(goblin.x, held.other()));
                    if other + f32::from(tuning.target_switch_px) <= kept {
                        held.other()
                    } else {
                        held
                    }
                }
                _ => nearest,
            });
        }
    }

    /// The hero goblin `index` chases: Carl unless the party is live.
    pub fn target_of(&self, index: usize) -> HeroId {
        self.targets
            .get(index)
            .copied()
            .flatten()
            .unwrap_or(HeroId::Carl)
    }

    /// Every goblin in index order: the attack token, the tell and swing, and
    /// the blow on its target.
    fn step_goblins(&mut self, scene: &mut SceneSim, tick: u64) {
        let live = scene.party_live();
        if live {
            self.update_targets(scene);
        }
        // The x each goblin chases: Carl for every goblin unless live.
        let target_x: Vec<f32> = (0..self.enemies.len())
            .map(|index| {
                if live {
                    scene.world.body(self.target_of(index)).x
                } else {
                    scene.world.hero.x
                }
            })
            .collect();
        // Attack token: at most MAX_ATTACKERS goblins in a tell or a swing; the
        // free slots go to the goblins that could attack, nearest to their
        // target first, ties to the lower index.
        let active = self.enemies.iter().filter(|g| g.token_held()).count();
        let slots = MAX_ATTACKERS.saturating_sub(active);
        let mut ranked: Vec<(f32, usize)> = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, g)| g.wants_attack())
            .map(|(index, g)| ((g.x - target_x[index]).abs(), index))
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
                target_x[index],
                !self.hero_defeated,
                &scene.world.solids,
                self.x_max - ENEMY_HALF_WIDTH,
                approved.contains(&index),
                may_start,
            );
            // WORK-023: one tick of the light guard (nothing while unset).
            self.enemies[index].observe_guard();
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

    /// DEMO-01 PR B: every llama in index order, after the goblins: the walk,
    /// the fair-fire tell and the release of the glob. The fair-fire view is
    /// the simulation's scroll camera (simulation state), never the
    /// presentation view.
    fn step_llamas(&mut self, scene: &SceneSim, tick: u64) {
        if self.llamas.is_empty() {
            return;
        }
        let hero_x = scene.world.hero.x;
        let left = scene.camera.left();
        let view = FireView {
            left: left as f32,
            right: (left + scene.camera.config().view_w) as f32,
        };
        for index in 0..self.llamas.len() {
            let step = self.llamas[index].step(
                hero_x,
                !self.hero_defeated,
                &scene.world.solids,
                self.x_max - LLAMA_HALF_WIDTH,
                view,
            );
            if step.started_tell {
                self.log(CombatEvent::LlamaTell {
                    tick,
                    llama: index as u32,
                });
            }
            if let Some(mut glob) = step.spit {
                glob.owner = index as u32;
                // Point blank: when Carl stands between the llama and its mouth
                // the glob would spawn behind him and never hit. The segment
                // from the llama's centre to the mouth is tested against his
                // body; on a hit the glob starts on his near face.
                let hero = &scene.world.hero;
                let near = hero.x - f32::from(glob.dir) * HERO_HALF_WIDTH;
                let llama_x = self.llamas[index].x;
                let (lo, hi) = (llama_x.min(glob.x), llama_x.max(glob.x));
                let reaches = hero.x + HERO_HALF_WIDTH > lo && hero.x - HERO_HALF_WIDTH < hi;
                let low = hero.y - HERO_HALF_HEIGHT;
                let high = hero.y + HERO_HALF_HEIGHT;
                if reaches && (low..high).contains(&glob.y) {
                    glob.x = near;
                }
                self.globs.push(glob);
                self.log(CombatEvent::LlamaSpit {
                    tick,
                    llama: index as u32,
                });
            }
        }
    }

    /// The spit globs in flight (oldest first): each flies straight, stops on a
    /// solid or at its range, and hurts Carl once through the ordinary hurt
    /// path when it overlaps him while he is not invulnerable. A glob that
    /// meets his invulnerability **splats** on him (it is gone, no damage, no
    /// event; owner decision 2026-11-19). A jump clears it.
    fn step_globs(&mut self, scene: &mut SceneSim, tick: u64) {
        let mut index = 0;
        while index < self.globs.len() {
            let fate = self.globs[index].step(&scene.world.solids);
            let mut done = fate != GlobFate::Flying;
            if !done {
                let glob = self.globs[index];
                // Carl, and Donut too when the live party is on: the first hero
                // the glob overlaps takes it (or splats it, see above).
                let heroes: &[HeroId] = if scene.party_live() {
                    &[HeroId::Carl, HeroId::Donut]
                } else {
                    &[HeroId::Carl]
                };
                for &id in heroes {
                    if self.hero_defeated || !glob.rect().overlaps(Self::hurtbox(scene, id)) {
                        continue;
                    }
                    let drive = *scene.drive();
                    let tuning = drive.tuning;
                    let hero_x = scene.world.body(id).x;
                    // D1 M4: a guarded hero outside the view cannot be hurt.
                    let unseen = drive.is_guarded(id)
                        && tuning.solo_offscreen_immunity
                        && scene.is_offscreen(hero_x, f32::from(tuning.offscreen_margin_px));
                    if scene.world.body(id).invulnerable_ticks == 0 && !unseen {
                        let owner = glob.owner as usize;
                        let source_x = self.llamas.get(owner).map_or(glob.x, |l| l.x);
                        let source = (self.enemies.len() + owner) as u32;
                        self.hurt_hero_by(scene, id, source, source_x, tick);
                    }
                    done = true;
                    break;
                }
            }
            if done {
                self.globs.remove(index);
            } else {
                index += 1;
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
        let shore = shore.clone();
        self.log(CombatEvent::ShoreLanded { tick });
        for index in 0..self.enemies.len() {
            if self.enemies[index].alive() && self.enemies[index].body().overlaps(footprint) {
                // DEATH-D: classify before `crush` changes the goblin. An
                // upright goblin whose head is under the stone and whose torso
                // is outside it is sheared at the neck; every other kill is a
                // crush. The state change is the same either way.
                let shears = self.enemies[index]
                    .neck_probes()
                    .is_some_and(|probes| shore.edge_shears(probes));
                if !self.enemies[index].crush() {
                    continue;
                }
                let cause = if shears {
                    DefeatCause::ShoreBlockEdge { player }
                } else {
                    DefeatCause::ShoreBlock { player }
                };
                self.log(CombatEvent::enemy_defeated(
                    tick,
                    index as u32,
                    cause,
                    self.enemies[index].hit_by,
                    None,
                ));
                if let Some(slot) = self.flash_tick.get_mut(index) {
                    *slot = Some(tick);
                }
            }
        }
        for index in 0..self.llamas.len() {
            if self.llamas[index].alive()
                && self.llamas[index].body().overlaps(footprint)
                && self.llamas[index].crush()
            {
                let target = (self.enemies.len() + index) as u32;
                self.log(CombatEvent::enemy_defeated(
                    tick,
                    target,
                    DefeatCause::ShoreBlock { player },
                    self.llamas[index].hit_by,
                    None,
                ));
                if let Some(slot) = self.flash_tick.get_mut(target as usize) {
                    *slot = Some(tick);
                }
            }
        }
    }

    /// DEATH-D: every blast charge in index order. A charge detonates exactly
    /// once, on its tick: the blast is logged, then evaluated once against the
    /// living goblins in index order (a goblin already defeated this tick, by
    /// the block or by a lower charge, is skipped). The QA charge does not hurt
    /// Carl or Donut, ignores solids and damages no terrain.
    fn step_charges(&mut self, tick: u64, carl_x: f32) {
        for charge_index in 0..self.charges.len() {
            if !self.charges[charge_index].step(tick, carl_x) {
                continue;
            }
            let charge = self.charges[charge_index];
            self.log(CombatEvent::Blast {
                tick,
                charge: charge_index as u32,
            });
            for index in 0..self.enemies.len() {
                if !self.enemies[index].alive() {
                    continue;
                }
                let Some(reach) = charge.reach(self.enemies[index].body()) else {
                    continue;
                };
                let point_blank = reach == BlastReach::PointBlank;
                let dx = self.enemies[index].x - charge.x;
                let away: i8 = if dx > 0.0 {
                    1
                } else if dx < 0.0 {
                    -1
                } else {
                    -self.enemies[index].facing
                };
                if !self.enemies[index].blast(away, point_blank) {
                    continue;
                }
                self.log(CombatEvent::enemy_defeated(
                    tick,
                    index as u32,
                    DefeatCause::Explosion {
                        charge: charge_index as u32,
                        player: charge.owner,
                        point_blank,
                    },
                    self.enemies[index].hit_by,
                    None,
                ));
                if let Some(slot) = self.flash_tick.get_mut(index) {
                    *slot = Some(tick);
                }
            }
        }
    }

    /// An active swing: it hurts a hero only if its box (in front of the
    /// goblin) overlaps the hero's hurtbox, the hero is not invulnerable and
    /// this attack has not hit. With a live party the goblin's target is tried
    /// first, then the other hero; one hit per attack.
    fn swing_on_hero(&mut self, scene: &mut SceneSim, index: usize, tick: u64) {
        if !scene.party_live() {
            self.swing_on(scene, index, HeroId::Carl, tick);
            return;
        }
        let target = self.target_of(index);
        if !self.swing_on(scene, index, target, tick) {
            self.swing_on(scene, index, target.other(), tick);
        }
    }

    /// The hurtbox of `hero`: Carl's 24x52 body, or Donut's own box (Q4),
    /// centred on the pivot and measured from the feet.
    pub fn hurtbox(scene: &SceneSim, hero: HeroId) -> Rect {
        let body = scene.world.body(hero);
        let (width, height) = match hero {
            HeroId::Carl => (HERO_HALF_WIDTH * 2.0, HERO_HALF_HEIGHT * 2.0),
            HeroId::Donut => scene.drive().tuning.donut_hurtbox.size(),
        };
        Rect {
            left: body.x - width * 0.5,
            bottom: body.y - HERO_HALF_HEIGHT,
            width,
            height,
        }
    }

    /// Try goblin `index`'s swing on `id`. Returns true when the blow landed
    /// (whether or not the solo last-HP guard kept the health up).
    fn swing_on(&mut self, scene: &mut SceneSim, index: usize, id: HeroId, tick: u64) -> bool {
        let drive = *scene.drive();
        let tuning = drive.tuning;
        let body = Self::hurtbox(scene, id);
        let hero_x = scene.world.body(id).x;
        let goblin = &self.enemies[index];
        // D1 M4 (solo only): a guarded hero outside the view cannot be hurt.
        let unseen = drive.is_guarded(id)
            && tuning.solo_offscreen_immunity
            && scene.is_offscreen(hero_x, f32::from(tuning.offscreen_margin_px));
        let hero = scene.world.body_mut(id);
        if self.hero_defeated
            || hero.invulnerable_ticks > 0
            || goblin.swing_hit
            || unseen
            || !goblin.swing_box().overlaps(body)
        {
            return false;
        }
        let goblin = &mut self.enemies[index];
        goblin.swing_hit = true;
        goblin.hit_stop = goblin.hit_stop.max(HERO_HURT_HIT_STOP);
        let goblin_x = goblin.x;
        if self.hurt_hero_by(scene, id, index as u32, goblin_x, tick) {
            self.enemies[index].hit_stop = self.enemies[index].hit_stop.max(HERO_DEFEAT_HIT_STOP);
        }
        true
    }

    /// The one way anything hurts a hero (a goblin's swing, a llama's glob):
    /// damage (Donut's own tuning, the solo last-HP guard), the
    /// invulnerability, the flinch and hit-stop, and the knockdown when it was
    /// the last health. `source` is the target id of the attacker (a goblin
    /// index, or the goblin count plus the llama index) and `source_x` its
    /// column (the blow comes from that side). Returns true when the hero was
    /// defeated; the caller freezes an attacker that has a body that freezes.
    fn hurt_hero_by(
        &mut self,
        scene: &mut SceneSim,
        id: HeroId,
        source: u32,
        source_x: f32,
        tick: u64,
    ) -> bool {
        let drive = *scene.drive();
        let tuning = drive.tuning;
        let (damage, invulnerable, hurt_for) = match id {
            HeroId::Carl => (1, HERO_INVULNERABLE_TICKS, HERO_HURT_TICKS),
            HeroId::Donut => (
                tuning.donut_damage_per_hit,
                tuning.donut_invulnerable_ticks,
                tuning.donut_hurt_ticks,
            ),
        };
        let hero = scene.world.body_mut(id);
        // D1 M1 (solo only): a hero the player does not drive takes the hit
        // (flinch, i-frames, HUD) but never drops below 1 HP.
        let floor = u8::from(drive.is_guarded(id) && tuning.solo_last_hp_guard);
        hero.health = hero
            .health
            .saturating_sub(damage)
            .max(floor.min(hero.health));
        hero.invulnerable_ticks = invulnerable;
        hero.melee.cancel();
        // The hurt and knockdown art assume the blow came from the front.
        hero.facing = if source_x >= hero.x { 1 } else { -1 };
        let away = if hero.x >= source_x { 1.0 } else { -1.0 };
        let health_left = hero.health;
        let mut state = self.hero_state(id);
        state.hurt_ticks = hurt_for;
        state.hit_stop = state.hit_stop.max(HERO_HURT_HIT_STOP);
        self.log(CombatEvent::HeroHurt {
            tick,
            source,
            health_left,
            hero: id,
        });
        if id == HeroId::Donut {
            // The HUD reads the companion's mirror at once, not next tick.
            scene.world.donut.health = health_left;
        }
        if health_left == 0 {
            self.hero_defeated = true;
            self.defeated_hero = id;
            self.deaths += 1;
            // The heavier knockdown: longer freeze, then launched away.
            state.hit_stop = state.hit_stop.max(HERO_DEFEAT_HIT_STOP);
            state.vx = away * HERO_KNOCKDOWN_VX;
            let hero = scene.world.body_mut(id);
            hero.vy = HERO_KNOCKDOWN_VY;
            hero.grounded = false;
            self.set_hero_state(id, state);
            self.log(CombatEvent::HeroDefeated { tick, hero: id });
            true
        } else {
            self.set_hero_state(id, state);
            false
        }
    }

    /// `id`'s hit-reaction state (Carl's lives in the original fields).
    fn hero_state(&self, id: HeroId) -> HeroCombat {
        match id {
            HeroId::Carl => HeroCombat {
                hurt_ticks: self.hurt_ticks,
                hit_stop: self.hero_hit_stop,
                vx: self.hero_vx,
            },
            HeroId::Donut => self.donut,
        }
    }

    fn set_hero_state(&mut self, id: HeroId, state: HeroCombat) {
        match id {
            HeroId::Carl => {
                self.hurt_ticks = state.hurt_ticks;
                self.hero_hit_stop = state.hit_stop;
                self.hero_vx = state.vx;
            }
            HeroId::Donut => self.donut = state,
        }
    }

    /// Ticks of hurt flinch `id` has left.
    pub fn hurt_ticks_of(&self, id: HeroId) -> u8 {
        self.hero_state(id).hurt_ticks
    }

    /// Carl is the defeated hero (his knockdown art plays). False when Donut's
    /// defeat ended the attempt.
    pub fn carl_defeated(&self) -> bool {
        self.hero_defeated && self.defeated_hero == HeroId::Carl
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
        }) || self.llamas.iter().filter(|l| l.alive()).any(|l| {
            let after = (l.x - candidate).abs();
            after < HERO_HALF_WIDTH + LLAMA_HALF_WIDTH && after < (l.x - hero.x).abs()
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
        let mut vx = self.hero_vx;
        drift_body(
            &mut scene.world.hero,
            &mut vx,
            &scene.world.solids,
            self.x_max,
        );
        self.hero_vx = vx;
    }

    /// Donut's knockdown drift (the same rule, her own body).
    fn drift_donut(&mut self, scene: &mut SceneSim) {
        let mut vx = self.donut.vx;
        drift_body(
            &mut scene.world.donut_body,
            &mut vx,
            &scene.world.solids,
            self.x_max,
        );
        self.donut.vx = vx;
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
        // CF02-Q01: Down is mixed only while held under `cf02_rules`.
        if self.cf02.is_some() && held.down {
            put(0x444f_574e);
        }
        put(u64::from(held.pause));
        hero.melee.mix_into(&mut put);
        // CF02-H: Carl's buffer and air state, mixed only under `cf02_rules` and
        // only while it is not the default.
        if self.cf02.is_some() {
            self.carl_cf02.mix_into(&mut put);
        }
        for goblin in &self.enemies {
            goblin.mix_into(&mut put);
        }
        // DEMO-01 PR B: mixed only for a fixture with llamas (or a glob in
        // flight), so every earlier fixture keeps its hash.
        if !self.llamas.is_empty() || !self.globs.is_empty() {
            put(0x4c4c_414d);
            for llama in &self.llamas {
                llama.mix_into(&mut put);
            }
            put(self.globs.len() as u64);
            for glob in &self.globs {
                glob.mix_into(&mut put);
            }
        }
        if let Some(shore) = &self.shore {
            shore.mix_into(&mut put);
        }
        // HUD-DEBT-03: the boss encounter, mixed only in a boss fixture.
        if let Some(boss) = &self.boss {
            put(0x4255_4953_4552_0001);
            boss.mix_into(&mut put);
        }
        // DEATH-D: nothing is mixed for a fixture without charges.
        for charge in &self.charges {
            charge.mix_into(&mut put);
        }
        // Live-party combat state (B1): Donut's hit state, each goblin's
        // target and the defeated hero. Mixed only while the party is live,
        // behind a marker word, so every legacy hash is unchanged.
        if scene.party_live() {
            put(0x5041_5254_5943_4d42);
            put(u64::from(self.donut.hurt_ticks));
            put(u64::from(self.donut.hit_stop));
            put(u64::from(self.donut.vx.to_bits()));
            put(self.defeated_hero.index() as u64);
            for target in &self.targets {
                put(target.map_or(0, |hero| hero.index() as u64 + 1));
            }
            // WORK-023: nothing is mixed while Donut never used an ability.
            self.donut_kit.hash_into(&mut put);
        }
        put(self.event_hash);
        // DEMO-01 only: no fixture mixes anything here.
        if self.demo.is_some() {
            put(0x4445_4d4f);
            put(self.checkpoint as u64);
        }
        self.companion.mix_hash(&mut put);
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
        ) + &self.llama_trace()
            + &self.shore_token()
            + &self.boss_token()
    }

    /// ` l<i>=<mode> ...` per llama and the globs in flight, else empty.
    fn llama_trace(&self) -> String {
        let mut out = String::new();
        for (i, l) in self.llamas.iter().enumerate() {
            out += &format!(
                " l{i}={:?} l{i}_t={} l{i}_x={:.2} l{i}_hs={} l{i}_hp={}",
                l.mode, l.attack_tick, l.x, l.hit_stop, l.health
            );
        }
        if !self.llamas.is_empty() {
            out += &format!(" globs={}", self.globs.len());
        }
        out
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

    /// ` llamas=l0=<x>/<hp>,...` while the fixture has llamas, else empty.
    fn llama_state(&self) -> String {
        if self.llamas.is_empty() {
            return String::new();
        }
        let llamas: Vec<String> = self
            .llamas
            .iter()
            .enumerate()
            .map(|(i, l)| format!("l{i}={:.3}/{}", l.x, l.health))
            .collect();
        format!(" llamas={}", llamas.join(","))
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
            + &self.llama_state()
            + &self.demo_token()
            + &self.boss_token()
    }

    /// ` boss=<hp>/<max>/<mode>/p<phase>` in a boss fixture, else empty.
    fn boss_token(&self) -> String {
        self.boss.as_ref().map_or_else(String::new, |boss| {
            format!(
                " boss={}/{}/{}/p{} boss_x={:.2}",
                boss.hp,
                boss.data().hp_max,
                boss.mode.name(),
                boss.phase,
                boss.x
            )
        })
    }

    /// ` demo_cp=<furthest beat entered>` in a demo run, else empty.
    fn demo_token(&self) -> String {
        self.demo
            .map_or_else(String::new, |_| format!(" demo_cp={}", self.checkpoint))
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

/// One tick of a defeated hero's knockdown drift in px/s: stopped by walls and
/// the layout edge, slowed by friction once the hero lies on the floor.
fn drift_body(hero: &mut Hero, vx: &mut f32, solids: &[Block], x_max: f32) {
    if *vx == 0.0 {
        return;
    }
    let wanted = hero.x + *vx * TICK_SECONDS;
    let candidate = wanted.clamp(HERO_HALF_WIDTH, x_max - HERO_HALF_WIDTH);
    let body = Rect {
        left: candidate - HERO_HALF_WIDTH,
        bottom: hero.y - HERO_HALF_HEIGHT,
        width: HERO_HALF_WIDTH * 2.0,
        height: HERO_HALF_HEIGHT * 2.0,
    };
    let blocked = solid_at(solids, body);
    if !blocked {
        hero.x = candidate;
    }
    if blocked || candidate != wanted {
        *vx = 0.0;
    } else if hero.grounded {
        let drag = GROUND_FRICTION * TICK_SECONDS;
        *vx = if vx.abs() <= drag {
            0.0
        } else {
            *vx - drag * vx.signum()
        };
    }
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
        CARL_HAYMAKER, CARL_JAB, CARL_PUSH_KICK, ENEMY_HEALTH, ForceTier, HERO_HEALTH, StrikeTarget,
    };
    use crate::feel::{
        CONTACT_POSE_TICKS, CONTACT_STAGGER, HERO_HURT_HIT_STOP, JAB_FEEL, TELL_SPACING,
    };
    use std::path::PathBuf;

    const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

    /// Committed expectations: the final `COMBAT_STATE` hash of each fixture
    /// at its default length. CI compares the headless replay and the real
    /// render run against the same values (`docs/evidence/cp01-feel`).
    const EXPECTED_HASHES: [(&str, u64); 20] = [
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
        ("cp03_kick_block", 0x73e4_0916_1ce5_434f),
        ("cp03_goblin_pileup", 0x6a82_811d_8f31_c475),
        // DEATH-D: the two new pinned fixtures (the 13 above are unchanged).
        ("cp03_block_edge_decap", 0x13d9_8f79_0282_17e9),
        ("qa_blast_breakup", 0xc3d2_b370_3f49_5284),
        // DEMO-01 PR B: the Bad Llama fixtures (new; nothing above changed).
        ("cp05_llama_slam", 0x73e1_5034_a522_2827),
        ("cp05_llama_down", 0x0229_7301_d16b_2b03),
        ("cp05_llama_spit", 0x5984_ba11_b72b_53dc),
        ("cp05_llama_offscreen", 0x9e65_7acd_442f_059b),
        ("cp05_llama_crush", 0xe151_c497_b2dd_7506),
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
            ..RawInput::NONE
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
                boss: None,
                llamas: &[],
                charges: &[],
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
        boss: None,
        llamas: &[],
        charges: &[],
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
        boss: None,
        llamas: &[],
        charges: &[],
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
        boss: None,
        llamas: &[],
        charges: &[],
        script: &[Hold {
            from: 1,
            to: 40,
            input: RawInput {
                move_x: 1.0,
                ..RawInput::NONE
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
        boss: None,
        llamas: &[],
        charges: &[],
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
        boss: None,
        llamas: &[],
        charges: &[],
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
            ..RawInput::NONE
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
            boss: None,
            llamas: &[],
            charges: &[],
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
                force: ForceTier::Routine,
                instigator: Some(0),
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
                ..RawInput::NONE
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
                ..RawInput::NONE
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
                CombatEvent::HeroDefeated { tick, .. } => Some(*tick),
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
                CombatEvent::HeroDefeated { tick, .. } => Some(*tick),
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

    /// CF02-00 parity: with `cf02_rules` off (the committed data and
    /// `Cf02Config::OFF`) every legacy fixture replays its committed hash, twice,
    /// and the pinned CF02 data changes nothing. The other legacy goldens (the
    /// walk, party, boss, `q10_*`, DEMO-01 and CINE ones) keep their own tests,
    /// which `cargo test` runs unchanged; `ci_floor01_combat.sh headless` also
    /// replays `--cf02` against `--demo`.
    #[test]
    fn cf02_parity_off() {
        use crate::cf02::Cf02Tuning;
        let tuning = Cf02Tuning::pinned().expect("the CF02 data loads");
        assert!(!tuning.cf02_rules && !tuning.llama_targets());
        assert_eq!(EXPECTED_HASHES.len(), 20, "13 legacy, 2 DEATH-D, 5 llama");
        for (name, expected) in EXPECTED_HASHES {
            let fixture = Fixture::by_name(name).unwrap();
            let (mut a, mut b) = (sim_of(fixture), sim_of(fixture));
            a.run_scripted(fixture.ticks);
            b.run_scripted(fixture.ticks);
            assert_eq!(a.state_hash(), expected, "{name}: {}", a.state_line());
            assert_eq!(a.state_hash(), b.state_hash(), "{name} replays");
            assert_eq!(events_of(&a), events_of(&b), "{name} events replay");
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
            ..RawInput::NONE
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
            ..RawInput::NONE
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
                ..RawInput::NONE
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
                ..RawInput::NONE
            });
        }
        assert!(starts(&events_of(&special)).is_empty());
        // Triangle/Y (PhysicalAttack) starts the push kick on a press edge.
        let mut kicked = sim_of(&CP02_KICK_CONTACT);
        for _ in 0..30 {
            kicked.step(RawInput {
                physical: true,
                ..RawInput::NONE
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
        sim.run_scripted(CP02_KICK_CONTACT.ticks);
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
    fn d03_the_contact_is_recorded_against_its_launch_id_and_target() {
        // The launch id is the kick's move instance; the pair is remembered.
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
            boss: None,
            llamas: &[],
            charges: &[],
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
    fn d04_the_block_landing_on_an_already_defeated_goblin_defeats_nobody_twice() {
        // A goblin already defeated (crush() stands in for any earlier death):
        // the block lands on the body and logs no second defeat.
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
            boss: None,
            llamas: &[],
            charges: &[],
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
                    ..RawInput::NONE
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

    fn defeat(cause: DefeatCause, force: ForceTier, instigator: Option<u32>) -> CombatEvent {
        CombatEvent::EnemyDefeated {
            tick: 3,
            target: 1,
            cause,
            force,
            instigator,
        }
    }

    fn hash_of(event: &CombatEvent) -> Vec<u64> {
        let mut words = Vec::new();
        event.mix_into(&mut |w| words.push(w));
        words
    }

    #[test]
    fn the_defeat_event_line_carries_what_the_cause_does_not_imply() {
        let melee = defeat(DefeatCause::Melee, ForceTier::Routine, Some(0));
        assert_eq!(
            melee.to_string(),
            "COMBAT_EVENT tick=3 kind=enemy_defeated target=1"
        );
        let kick = defeat(DefeatCause::Melee, ForceTier::Heavy, Some(0));
        assert!(kick.to_string().ends_with("target=1 force=heavy"));
        let block = defeat(
            DefeatCause::ShoreBlock { player: 0 },
            ForceTier::Catastrophic,
            None,
        );
        assert!(
            block
                .to_string()
                .ends_with("cause=shore_block credit_player=0")
        );
        let chained = defeat(
            DefeatCause::ShoreBlock { player: 0 },
            ForceTier::Catastrophic,
            Some(0),
        );
        assert!(
            chained
                .to_string()
                .ends_with("cause=shore_block credit_player=0 instigator=0")
        );
    }

    #[test]
    fn the_defeat_hash_only_grows_for_an_instigator_the_cause_does_not_imply() {
        // The pre-DEATH-B words: tag, tick, target (+ the block's credit).
        let melee = defeat(DefeatCause::Melee, ForceTier::Routine, Some(0));
        assert_eq!(hash_of(&melee), [3, 3, 1]);
        let kick = defeat(DefeatCause::Melee, ForceTier::Heavy, Some(0));
        assert_eq!(hash_of(&kick), [3, 3, 1], "force is derived, not mixed");
        let block = defeat(
            DefeatCause::ShoreBlock { player: 0 },
            ForceTier::Catastrophic,
            None,
        );
        assert_eq!(hash_of(&block), [3, 3, 1, 0x5348_4f52, 0]);
        let chained = defeat(
            DefeatCause::ShoreBlock { player: 0 },
            ForceTier::Catastrophic,
            Some(0),
        );
        assert_ne!(hash_of(&chained), hash_of(&block));
        // Intensity has no field to differ in: the event has no gore member.
    }

    #[test]
    fn a_second_players_hit_hashes_differently_and_player_one_hashes_as_before() {
        let words = |hit_by: Option<u32>| {
            let mut g = Goblin::new(480.0, 100.0, EnemyAi::Passive);
            g.hit_by = hit_by;
            let mut out = Vec::new();
            g.mix_into(&mut |w| out.push(w));
            out
        };
        assert_eq!(words(None), words(Some(0)), "1P hashes are unchanged");
        assert_ne!(words(Some(1)), words(Some(0)));
        assert_ne!(words(Some(1)), words(Some(2)));
    }

    #[test]
    fn the_defeat_event_constructor_derives_force_and_every_fixture_force_is_allowed() {
        use crate::combat::{CARL_HAYMAKER, CARL_PUSH_KICK};
        let force = |cause, lethal| match CombatEvent::enemy_defeated(1, 0, cause, None, lethal) {
            CombatEvent::EnemyDefeated { force, .. } => force,
            _ => unreachable!(),
        };
        assert_eq!(
            force(DefeatCause::Melee, Some(&CARL_HAYMAKER)),
            ForceTier::Routine
        );
        assert_eq!(
            force(DefeatCause::Melee, Some(&CARL_PUSH_KICK)),
            ForceTier::Heavy
        );
        let block = DefeatCause::ShoreBlock { player: 0 };
        assert_eq!(force(block, None), ForceTier::Catastrophic);
        // Whatever a fixture logs is in the allowed set of its cause.
        for fixture in FIXTURES {
            let mut sim = sim_of(fixture);
            sim.run_scripted(fixture.ticks);
            for event in events_of(&sim) {
                if let CombatEvent::EnemyDefeated { cause, force, .. } = event {
                    assert!(cause.derived_forces().contains(&force), "{}", fixture.name);
                }
            }
        }
    }

    #[test]
    fn cp03_kick_block_is_the_committed_chain_without_a_teleport() {
        let mut sim = sim_of(&CP03_KICK_BLOCK);
        sim.run_scripted(CP03_KICK_BLOCK.ticks);
        let events = events_of(&sim);
        let defeated: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
            .collect();
        assert_eq!(defeated.len(), 1);
        assert!(matches!(
            defeated[0],
            CombatEvent::EnemyDefeated {
                tick: 179,
                target: 0,
                cause: DefeatCause::ShoreBlock { player: 0 },
                force: ForceTier::Catastrophic,
                instigator: Some(0),
            }
        ));
        // The kick landed (one hit, not lethal); the block was the killer.
        assert_eq!(hits(&events), [(33, 0)]);
    }

    #[test]
    fn the_unhashed_force_is_always_derived_from_hashed_data() {
        use crate::combat::carl_move;
        for fixture in FIXTURES {
            let mut sim = sim_of(fixture);
            sim.run_scripted(fixture.ticks);
            let events = events_of(&sim);
            for (i, event) in events.iter().enumerate() {
                let CombatEvent::EnemyDefeated {
                    tick,
                    target,
                    cause,
                    force,
                    ..
                } = *event
                else {
                    continue;
                };
                match cause {
                    DefeatCause::Melee => {
                        let killing = events[..i].iter().rev().find_map(|e| match *e {
                            CombatEvent::Hit {
                                tick: t,
                                target: x,
                                move_id,
                                ..
                            } if t == tick && x == target => Some(move_id),
                            _ => None,
                        });
                        let spec = carl_move(killing.expect("a hit on the same tick")).unwrap();
                        assert_eq!(force, ForceTier::of_move(spec), "{}", fixture.name);
                    }
                    DefeatCause::ShoreBlock { .. }
                    | DefeatCause::ShoreBlockEdge { .. }
                    | DefeatCause::DropHazard { .. } => {
                        assert_eq!(force, ForceTier::Catastrophic, "{}", fixture.name);
                    }
                    DefeatCause::Explosion { point_blank, .. } => {
                        let want = if point_blank {
                            ForceTier::Catastrophic
                        } else {
                            ForceTier::Heavy
                        };
                        assert_eq!(force, want, "{}", fixture.name);
                    }
                }
            }
        }
    }

    /// The first kick of `CP01_KICK` launches the goblin (it lives with 1 of 2
    /// health) before a block lands on it: kick -> crush.
    static KICK_THEN_BLOCK: Fixture = Fixture {
        name: "test_kick_then_block",
        summary: "test",
        seed: 11,
        carl_x: 400,
        enemies: &[EnemySpawn {
            x: 432,
            ai: EnemyAi::Passive,
        }],
        shore_dx: Some(19),
        boss: None,
        llamas: &[],
        charges: &[],
        script: &[
            kick(20, 21),
            attack(120, 121),
            attack(160, 161),
            attack(200, 201),
        ],
        ticks: 320,
    };

    #[test]
    fn d04_a_kick_then_the_block_credits_the_block_and_keeps_the_kicker() {
        let mut sim = sim_of(&KICK_THEN_BLOCK);
        sim.run_scripted(100);
        assert_eq!(sim.combat.enemies[0].hit_by, Some(0));
        assert!(
            sim.combat.enemies[0].alive(),
            "the kick alone does not kill"
        );
        assert!(defeats(&events_of(&sim)).is_empty());
        // Stand the (recovered) goblin in the footprint, as a pushed enemy
        // would be, then let the shore break and the block land.
        sim.combat.enemies[0].x = 484.0;
        sim.run_scripted(KICK_THEN_BLOCK.ticks - 100);
        let events = events_of(&sim);
        let lethal: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
            .collect();
        assert_eq!(lethal.len(), 1, "exactly one defeat event");
        assert_eq!(
            *lethal[0],
            defeat_at(
                lethal[0],
                DefeatCause::ShoreBlock { player: 0 },
                ForceTier::Catastrophic,
                Some(0)
            )
        );
        // The block, not the kick, is the lethal event: no melee defeat.
        assert_eq!(hits(&events).len(), 1);
    }

    fn defeat_at(
        like: &CombatEvent,
        cause: DefeatCause,
        force: ForceTier,
        instigator: Option<u32>,
    ) -> CombatEvent {
        let CombatEvent::EnemyDefeated { tick, target, .. } = *like else {
            panic!("not a defeat");
        };
        CombatEvent::EnemyDefeated {
            tick,
            target,
            cause,
            force,
            instigator,
        }
    }

    #[test]
    fn a_defeat_nobody_touched_has_no_instigator_and_a_kick_kill_is_heavy() {
        let mut sim = sim_of(&CP03_SHORE_KILL);
        sim.run_scripted(CP03_SHORE_KILL.ticks);
        assert!(events_of(&sim).iter().any(|e| matches!(
            e,
            CombatEvent::EnemyDefeated {
                cause: DefeatCause::ShoreBlock { player: 0 },
                force: ForceTier::Catastrophic,
                instigator: None,
                ..
            }
        )));
        // The second kick of CP01_KICK is a lethal melee defeat: heavy force.
        let mut sim = sim_of(&CP01_KICK);
        sim.run_scripted(CP01_KICK.ticks);
        assert!(events_of(&sim).iter().any(|e| matches!(
            e,
            CombatEvent::EnemyDefeated {
                cause: DefeatCause::Melee,
                force: ForceTier::Heavy,
                instigator: Some(0),
                ..
            }
        )));
    }

    #[test]
    fn every_defeat_event_is_once_per_target_in_every_fixture() {
        for fixture in FIXTURES {
            let mut sim = sim_of(fixture);
            sim.run_scripted(fixture.ticks);
            let mut seen = std::collections::BTreeSet::new();
            for (target, _) in defeats(&events_of(&sim)) {
                assert!(
                    seen.insert(target),
                    "{}: target {target} twice",
                    fixture.name
                );
            }
        }
    }

    #[test]
    fn the_selected_finish_follows_the_event_and_the_replay_hash_ignores_it() {
        use crate::death::DeathProfiles;
        let profiles = DeathProfiles::builtin();
        let finish_of = |fixture: &'static Fixture, species: &str| {
            let mut sim = sim_of(fixture);
            sim.run_scripted(fixture.ticks);
            let hash = sim.state_hash();
            let picked: Vec<String> = events_of(&sim)
                .iter()
                .filter_map(|e| profiles.select_for_event(species, e))
                .map(|(_, s)| s.finish.to_string())
                .collect();
            (picked, hash)
        };
        // The same replay: same events, same finish, same hash; and a second
        // species reads the same events without touching the hash.
        let (goblin_a, hash_a) = finish_of(&CP03_SHORE_KILL, "goblin");
        let (goblin_b, hash_b) = finish_of(&CP03_SHORE_KILL, "goblin");
        let (skeleton, hash_c) = finish_of(&CP03_SHORE_KILL, "skeleton_mock");
        assert_eq!(goblin_a, ["goblin_crushed"]);
        assert_eq!(goblin_a, goblin_b);
        assert_eq!(skeleton, ["skeleton_flat_pile"]);
        assert_eq!((hash_a, hash_a), (hash_b, hash_c));
        let (jab_kill, _) = finish_of(&CP01_TWO_HITS, "goblin");
        assert_eq!(jab_kill, ["goblin_knockback_down"]);
        let (slime, _) = finish_of(&CP01_TWO_HITS, "slime_mock");
        assert_eq!(slime, ["slime_puddle"], "no knockback finish: fallback");
        // DEATH-D: the edge decapitates, the blast bursts the core and throws
        // the ring; a species without a head or a burst degrades.
        let (edge, edge_hash) = finish_of(&CP03_BLOCK_EDGE_DECAP, "goblin");
        assert_eq!(edge, ["goblin_decapitated"]);
        assert_eq!(edge_hash, finish_of(&CP03_BLOCK_EDGE_DECAP, "goblin").1);
        let (edge_skeleton, edge_hash_skeleton) =
            finish_of(&CP03_BLOCK_EDGE_DECAP, "skeleton_mock");
        assert_eq!(edge_skeleton, ["skeleton_flat_pile"]);
        assert_eq!(edge_hash, edge_hash_skeleton);
        let (blast, _) = finish_of(&QA_BLAST_BREAKUP, "goblin");
        assert_eq!(blast, ["goblin_exploded", "goblin_knockback_down"]);
        let (blast_slime, _) = finish_of(&QA_BLAST_BREAKUP, "slime_mock");
        assert_eq!(blast_slime, ["slime_puddle", "slime_puddle"]);
    }

    // ------------------------------------------------ DEATH-D: edge and blast --

    /// Both new fixtures; the 13 older ones never log either new cause.
    const DEATH_D_FIXTURES: [&str; 2] = ["cp03_block_edge_decap", "qa_blast_breakup"];

    fn causes_of(events: &[CombatEvent]) -> Vec<(u32, DefeatCause)> {
        defeats(events)
    }

    #[test]
    fn no_routine_fixture_ever_shears_a_neck_or_explodes() {
        for fixture in FIXTURES {
            if DEATH_D_FIXTURES.contains(&fixture.name) {
                continue;
            }
            assert!(fixture.charges.is_empty(), "{}", fixture.name);
            let mut sim = sim_of(fixture);
            sim.run_scripted(fixture.ticks);
            for event in events_of(&sim) {
                assert!(
                    !matches!(event, CombatEvent::Blast { .. }),
                    "{}: {event}",
                    fixture.name
                );
                if let CombatEvent::EnemyDefeated { cause, .. } = event {
                    assert!(
                        !matches!(
                            cause,
                            DefeatCause::ShoreBlockEdge { .. } | DefeatCause::Explosion { .. }
                        ),
                        "{}: {event}",
                        fixture.name
                    );
                }
            }
        }
    }

    #[test]
    fn cp03_block_edge_decap_shears_the_neck_on_the_landing_tick() {
        let mut sim = sim_of(&CP03_BLOCK_EDGE_DECAP);
        sim.run_scripted(CP03_BLOCK_EDGE_DECAP.ticks);
        let events = events_of(&sim);
        assert_eq!(
            causes_of(&events),
            [(0, DefeatCause::ShoreBlockEdge { player: 0 })]
        );
        let lines: Vec<String> = events.iter().map(ToString::to_string).collect();
        let landed = lines
            .iter()
            .position(|l| l == "COMBAT_EVENT tick=139 kind=shore_landed")
            .expect("the block lands on 139");
        assert_eq!(
            lines[landed + 1],
            "COMBAT_EVENT tick=139 kind=enemy_defeated target=0 cause=shore_block_edge credit_player=0"
        );
        // The state change is exactly a crush: Down, then the corpse.
        let goblin = &sim.combat.enemies[0];
        assert!(!goblin.alive());
        assert!(matches!(goblin.mode, GoblinMode::Down | GoblinMode::Corpse));
        assert_eq!(goblin.x, 524.0);
        assert_eq!(sim.scene.world.hero.health, HERO_HEALTH);
        // The finish is the decapitation.
        let picked = crate::death::DeathProfiles::builtin()
            .select_for_event("goblin", &events[events.len() - 1])
            .map(|(_, s)| s.finish);
        assert_eq!(picked, Some("goblin_decapitated"));
    }

    /// `CP03_SHORE_KILL` with the goblin placed on one side of an edge after
    /// the setup (facing `facing`), returning the cause of its defeat.
    fn block_cause_at(x: f32, facing: i8) -> DefeatCause {
        let mut sim = sim_of(&CP03_SHORE_KILL);
        sim.combat.enemies[0].x = x;
        sim.combat.enemies[0].facing = facing;
        sim.run_scripted(CP03_SHORE_KILL.ticks);
        let found = causes_of(&events_of(&sim));
        assert_eq!(found.len(), 1, "x {x} facing {facing}: {found:?}");
        found[0].1
    }

    #[test]
    fn the_block_classifies_the_edges_through_the_real_landing() {
        let edge = DefeatCause::ShoreBlockEdge { player: 0 };
        let crush = DefeatCause::ShoreBlock { player: 0 };
        // The right edge R = 524, facing -1.
        for (x, want) in [
            (519.9, crush),
            (520.0, edge),
            (524.0, edge),
            (527.9, edge),
            (528.0, crush),
        ] {
            assert_eq!(block_cause_at(x, -1), want, "right edge x {x}");
        }
        // The left edge L = 444, facing +1.
        for (x, want) in [(439.9, crush), (440.0, edge), (447.9, edge), (448.0, crush)] {
            assert_eq!(block_cause_at(x, 1), want, "left edge x {x}");
        }
        // Facing away from the stone: a crush at either edge.
        assert_eq!(block_cause_at(524.0, 1), crush);
        assert_eq!(block_cause_at(444.0, -1), crush);
        // Well inside the footprint: a crush.
        assert_eq!(block_cause_at(484.0, -1), crush);
    }

    #[test]
    fn a_goblin_lying_down_under_the_edge_is_crushed_not_sheared() {
        // The kick launches the living goblin under the block; it lies there
        // (Downed) when the block lands: a crush whatever its x.
        let mut sim = sim_of(&CP03_KICK_BLOCK);
        sim.run_scripted(CP03_KICK_BLOCK.ticks);
        assert_eq!(
            causes_of(&events_of(&sim)),
            [(0, DefeatCause::ShoreBlock { player: 0 })]
        );
        // And a goblin that is not upright has no neck probes at all.
        let mut g = Goblin::new(524.0, 100.0, EnemyAi::Passive);
        g.mode = GoblinMode::Downed;
        assert!(g.neck_probes().is_none());
    }

    #[test]
    fn qa_blast_breakup_bursts_the_core_throws_the_ring_and_spares_the_rest() {
        let mut sim = sim_of(&QA_BLAST_BREAKUP);
        sim.run_scripted(QA_BLAST_BREAKUP.ticks);
        let events = events_of(&sim);
        let lines: Vec<String> = events.iter().map(ToString::to_string).collect();
        assert_eq!(
            lines,
            [
                "COMBAT_EVENT tick=60 kind=blast charge=0",
                "COMBAT_EVENT tick=60 kind=enemy_defeated target=0 cause=explosion charge=0 point_blank",
                "COMBAT_EVENT tick=60 kind=enemy_defeated target=1 cause=explosion charge=0",
            ]
        );
        let force = |target: usize| {
            events.iter().find_map(|e| match e {
                CombatEvent::EnemyDefeated {
                    target: t, force, ..
                } if *t as usize == target => Some(*force),
                _ => None,
            })
        };
        assert_eq!(force(0), Some(ForceTier::Catastrophic));
        assert_eq!(force(1), Some(ForceTier::Heavy));
        assert_eq!(force(2), None);
        let goblins = &sim.combat.enemies;
        assert!(!goblins[0].alive() && !goblins[1].alive());
        assert!(goblins[2].alive(), "48 px from the blast: untouched");
        assert_eq!(goblins[2].x, 580.0);
        // The ring goblin was thrown away from the charge (to the right).
        assert!(goblins[1].x > 548.0 + 40.0, "{}", goblins[1].x);
        // The charge fired once and stays fired; Carl is untouched.
        assert!(sim.combat.charges[0].detonated);
        assert_eq!(sim.scene.world.hero.health, HERO_HEALTH);
        assert!(!sim.combat.hero_defeated);
        // The finishes: the core bursts, the ring is a thrown knockback.
        let profiles = crate::death::DeathProfiles::builtin();
        let finishes: Vec<&str> = events
            .iter()
            .filter_map(|e| profiles.select_for_event("goblin", e))
            .map(|(_, s)| s.finish)
            .collect();
        assert_eq!(finishes, ["goblin_exploded", "goblin_knockback_down"]);
    }

    static BLOCK_AND_CHARGE: Fixture = Fixture {
        name: "test_block_and_charge",
        summary: "test",
        seed: 21,
        carl_x: 400,
        enemies: &[EnemySpawn {
            x: 484,
            ai: EnemyAi::Passive,
        }],
        shore_dx: Some(19),
        boss: None,
        llamas: &[],
        charges: &[ChargeSpawn {
            dx: 84,
            tick: 139,
            owner: None,
            arm: Arm::Tick,
        }],
        script: &[attack(20, 21), attack(60, 61), attack(100, 101)],
        ticks: 200,
    };

    #[test]
    fn a_block_and_a_charge_on_one_tick_the_block_wins_and_the_goblin_dies_once() {
        let mut sim = sim_of(&BLOCK_AND_CHARGE);
        sim.run_scripted(BLOCK_AND_CHARGE.ticks);
        let events = events_of(&sim);
        assert_eq!(
            causes_of(&events),
            [(0, DefeatCause::ShoreBlock { player: 0 })],
            "evaluated first: the charge finds a dead goblin"
        );
        assert!(
            events.iter().any(|e| matches!(
                e,
                CombatEvent::Blast {
                    tick: 139,
                    charge: 0
                }
            )),
            "the blast still goes off"
        );
        let landed = events
            .iter()
            .position(|e| matches!(e, CombatEvent::ShoreLanded { .. }))
            .unwrap();
        let blast = events
            .iter()
            .position(|e| matches!(e, CombatEvent::Blast { .. }))
            .unwrap();
        assert!(landed < blast, "shore first, then the charges");
    }

    static TWO_CHARGES: Fixture = Fixture {
        name: "test_two_charges",
        summary: "test",
        seed: 23,
        carl_x: 400,
        enemies: &[
            EnemySpawn {
                x: 520,
                ai: EnemyAi::Passive,
            },
            EnemySpawn {
                x: 560,
                ai: EnemyAi::Passive,
            },
        ],
        shore_dx: None,
        boss: None,
        llamas: &[],
        charges: &[
            ChargeSpawn {
                dx: 120,
                tick: 60,
                owner: None,
                arm: Arm::Tick,
            },
            ChargeSpawn {
                dx: 135,
                tick: 60,
                owner: Some(0),
                arm: Arm::Tick,
            },
        ],
        script: &[],
        ticks: 100,
    };

    #[test]
    fn two_charges_on_one_tick_go_in_index_order_and_each_target_dies_once() {
        let mut sim = sim_of(&TWO_CHARGES);
        sim.run_scripted(TWO_CHARGES.ticks);
        let events = events_of(&sim);
        let blasts: Vec<u32> = events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::Blast { charge, .. } => Some(*charge),
                _ => None,
            })
            .collect();
        assert_eq!(blasts, [0, 1]);
        // Goblin 0 is inside the radius of both charges and dies once, to the
        // lower index; goblin 1 is reached by charge 1 only (13 px: a ring kill
        // credited to its owner).
        assert_eq!(
            causes_of(&events),
            [
                (
                    0,
                    DefeatCause::Explosion {
                        charge: 0,
                        player: None,
                        point_blank: true
                    }
                ),
                (
                    1,
                    DefeatCause::Explosion {
                        charge: 1,
                        player: Some(0),
                        point_blank: false
                    }
                ),
            ]
        );
        let last = events.last().unwrap().to_string();
        assert_eq!(
            last,
            "COMBAT_EVENT tick=60 kind=enemy_defeated target=1 cause=explosion charge=1 credit_player=0"
        );
    }

    static CHARGE_ON_CARL: Fixture = Fixture {
        name: "test_charge_on_carl",
        summary: "test",
        seed: 25,
        carl_x: 400,
        enemies: &[],
        shore_dx: None,
        boss: None,
        llamas: &[],
        charges: &[ChargeSpawn {
            dx: 0,
            tick: 30,
            owner: Some(0),
            arm: Arm::Tick,
        }],
        script: &[],
        ticks: 80,
    };

    #[test]
    fn the_qa_charge_does_not_hurt_carl_or_anyone_it_does_not_reach() {
        let mut sim = sim_of(&CHARGE_ON_CARL);
        sim.run_scripted(CHARGE_ON_CARL.ticks);
        assert!(events_of(&sim).iter().any(|e| matches!(
            e,
            CombatEvent::Blast {
                tick: 30,
                charge: 0
            }
        )));
        assert_eq!(sim.scene.world.hero.health, HERO_HEALTH);
        assert!(!sim.combat.hero_defeated);
        assert_eq!(sim.combat.hurt_ticks, 0);
        assert!(defeats(&events_of(&sim)).is_empty());
    }

    #[test]
    fn a_restart_rearms_the_charge_and_the_second_attempt_replays_the_first() {
        let mut sim = sim_of(&QA_BLAST_BREAKUP);
        sim.run_scripted(100);
        assert!(sim.combat.charges[0].detonated);
        let first: Vec<CombatEvent> = events_of(&sim);
        sim.restart();
        assert!(!sim.combat.charges[0].detonated, "rebuilt from the fixture");
        assert!(sim.combat.enemies.iter().all(Goblin::alive));
        sim.run_scripted(100);
        let all = events_of(&sim);
        assert_eq!(
            all[first.len()],
            CombatEvent::Restart {
                tick: 100,
                deaths: 0
            }
        );
        assert_eq!(all[first.len() + 1..], first[..], "the same blast, again");
    }

    /// A charge and nobody to hit: only the charge itself can move the hash.
    static LONE_CHARGE: Fixture = Fixture {
        name: "test_lone_charge",
        summary: "test",
        seed: 27,
        carl_x: 400,
        enemies: &[],
        shore_dx: None,
        boss: None,
        llamas: &[],
        charges: &[ChargeSpawn {
            dx: 120,
            tick: 30,
            owner: None,
            arm: Arm::Tick,
        }],
        script: &[],
        ticks: 60,
    };

    static LONE_CHARGE_OWNED: Fixture = Fixture {
        name: "test_lone_charge_owned",
        summary: "test",
        seed: 27,
        carl_x: 400,
        enemies: &[],
        shore_dx: None,
        boss: None,
        llamas: &[],
        charges: &[ChargeSpawn {
            dx: 120,
            tick: 30,
            owner: Some(0),
            arm: Arm::Tick,
        }],
        script: &[],
        ticks: 60,
    };

    #[test]
    fn the_charge_state_is_mixed_into_the_hash_and_only_for_fixtures_with_charges() {
        // With nobody to kill, the charge is the only thing that differs.
        let mut a = sim_of(&LONE_CHARGE);
        a.run_scripted(10);
        let with_charge = a.state_hash();
        // Taking the charge away changes the hash: it is mixed.
        let mut b = a.clone();
        b.combat.charges.clear();
        assert_ne!(b.state_hash(), with_charge);
        // Its owner and its detonated flag are part of the mix.
        let mut owned = sim_of(&LONE_CHARGE_OWNED);
        owned.run_scripted(10);
        assert_ne!(owned.state_hash(), with_charge, "the owner is mixed");
        a.run_scripted(30);
        let fired = a.state_hash();
        let mut rearmed = a.clone();
        rearmed.combat.charges[0].detonated = false;
        assert_ne!(rearmed.state_hash(), fired, "the detonated flag is mixed");
        assert!(a.combat.charges[0].detonated);
        // A fixture without charges mixes nothing for them: clearing the
        // (empty) list leaves the hash alone, and the committed hashes pin it.
        let mut plain = sim_of(&CP03_SHORE_KILL);
        plain.run_scripted(50);
        let before = plain.state_hash();
        plain.combat.charges.clear();
        assert_eq!(plain.state_hash(), before);
    }

    #[test]
    fn a_charge_on_tick_zero_is_refused_when_the_fixture_loads() {
        static ZERO: Fixture = Fixture {
            name: "test_zero_charge",
            summary: "test",
            seed: 29,
            carl_x: 400,
            enemies: &[],
            shore_dx: None,
            boss: None,
            llamas: &[],
            charges: &[ChargeSpawn {
                dx: 10,
                tick: 0,
                owner: None,
                arm: Arm::Tick,
            }],
            script: &[],
            ticks: 10,
        };
        let err = FixtureSim::new(&kit(), 360, &ZERO).unwrap_err();
        assert!(err.contains("tick 0"), "{err}");
    }

    #[test]
    fn the_new_fixtures_replay_to_the_same_hash_and_events() {
        for fixture in [&CP03_BLOCK_EDGE_DECAP, &QA_BLAST_BREAKUP] {
            let mut a = sim_of(fixture);
            let mut b = sim_of(fixture);
            a.run_scripted(fixture.ticks);
            b.run_scripted(fixture.ticks);
            assert_eq!(a.state_hash(), b.state_hash(), "{}", fixture.name);
            assert_eq!(events_of(&a), events_of(&b), "{}", fixture.name);
        }
    }

    #[test]
    fn the_committed_logs_of_the_new_fixtures_are_the_replay() {
        for name in DEATH_D_FIXTURES {
            let fixture = Fixture::by_name(name).unwrap();
            let mut sim = sim_of(fixture);
            sim.run_scripted(fixture.ticks);
            let mut lines: Vec<String> = events_of(&sim).iter().map(ToString::to_string).collect();
            lines.push(sim.state_line());
            let committed = std::fs::read_to_string(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join(format!("docs/evidence/cp01-feel/headless_{name}.txt")),
            )
            .unwrap();
            assert_eq!(lines.join("\n") + "\n", committed, "{name}");
        }
    }

    #[test]
    fn the_equipped_special_still_does_nothing_and_no_input_places_a_charge() {
        // No route, ability or input creates a charge: pressing the special on
        // a fixture without charges leaves `charges` empty and logs no blast.
        let mut sim = sim_of(&NO_GOBLINS);
        let special = RawInput {
            ability: true,
            ..RawInput::NONE
        };
        for _ in 0..60 {
            sim.step(special);
        }
        assert!(sim.combat.charges.is_empty());
        assert!(events_of(&sim).is_empty());
    }

    /// Spec 8.2 (HUD-HYBRID-A-01): over every combat fixture and corridor
    /// scenario, the highest screen-space top of any hero or enemy body,
    /// launched goblin (the launch arc) or swing/telegraph box stays at
    /// y >= 60 on the 360 and 400 canvases, so nothing reaches the HUD plates
    /// (bottom at y = 56). Screen y = layout y minus the view top. Cosmetic
    /// death parts and the vertical camera shake are not simulation state and
    /// are not measured. Run with `--nocapture` to see the numbers.
    #[test]
    fn gameplay_stays_below_the_hud_plates() {
        use crate::corridor::{RUN_JUMP_TOTAL_TICKS, Scenario};
        use crate::prototype::HERO_HALF_HEIGHT;
        const CLEARANCE: f32 = 60.0;
        let kit = kit();
        for (view_h, view_name) in [(360, "abc_640x360"), (400, "abc_640x400")] {
            let view_top = kit.view(view_name).unwrap().y as f32;
            let screen_y = |world_top: f32| kit.height as f32 - world_top - view_top;
            let mut overall = (f32::MAX, String::new());
            let mut note = |top: f32, what: &str| {
                if top < overall.0 {
                    overall = (top, what.to_owned());
                }
            };
            for fixture in FIXTURES {
                let mut sim = FixtureSim::new(&kit, view_h, fixture).unwrap();
                let mut min = f32::MAX;
                for _ in 0..fixture.ticks {
                    sim.step_scripted();
                    let hero = &sim.scene.world.hero;
                    min = min.min(screen_y(hero.y + HERO_HALF_HEIGHT));
                    for goblin in &sim.combat.enemies {
                        let body = goblin.body();
                        min = min.min(screen_y(body.bottom + body.height));
                        let swing = goblin.swing_box();
                        if goblin.attack_phase().is_some() {
                            min = min.min(screen_y(swing.bottom + swing.height));
                        }
                    }
                }
                println!(
                    "HUD_CLEARANCE {view_h} fixture {}: min top y = {min:.1}",
                    fixture.name
                );
                note(min, fixture.name);
            }
            for scenario in [Scenario::WalkRight, Scenario::TurnAround, Scenario::RunJump] {
                let mut sim = SceneSim::new(&kit, view_h).unwrap();
                let mut min = f32::MAX;
                sim.run_scenario_observed(scenario, RUN_JUMP_TOTAL_TICKS, |state| {
                    min = min.min(screen_y(state.world.hero.y + HERO_HALF_HEIGHT));
                });
                println!("HUD_CLEARANCE {view_h} scenario {scenario:?}: min top y = {min:.1}");
                note(min, "scenario");
            }
            println!(
                "HUD_CLEARANCE {view_h} MIN = {:.1} ({})",
                overall.0, overall.1
            );
            assert!(
                overall.0 >= CLEARANCE,
                "{view_h}: {} reaches y = {:.1} < {CLEARANCE}",
                overall.1,
                overall.0
            );
        }
    }

    // ------------------------------------------------------------ Bad Llama ----

    fn llama_events(fixture: &'static Fixture) -> (FixtureSim, Vec<CombatEvent>) {
        let mut sim = sim_of(fixture);
        sim.run_scripted(fixture.ticks);
        let events = events_of(&sim);
        (sim, events)
    }

    fn defeat_rows(events: &[CombatEvent]) -> Vec<(u64, u32, DefeatCause, ForceTier)> {
        events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::EnemyDefeated {
                    tick,
                    target,
                    cause,
                    force,
                    ..
                } => Some((tick, target, cause, force)),
                _ => None,
            })
            .collect()
    }

    fn event_ticks(events: &[CombatEvent], pick: fn(&CombatEvent) -> Option<u64>) -> Vec<u64> {
        events.iter().filter_map(pick).collect()
    }

    #[test]
    fn three_kicks_defeat_the_llama_once_and_the_lethal_kick_is_heavy() {
        let (sim, events) = llama_events(&CP05_LLAMA_SLAM);
        assert_eq!(
            defeat_rows(&events),
            [(163, 0, DefeatCause::Melee, ForceTier::Heavy)]
        );
        // Three push kicks hit it (health 3 -> 2 -> 1 -> 0), each once.
        let healths: Vec<u8> = events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::Hit { health_left, .. } => Some(*health_left),
                _ => None,
            })
            .collect();
        assert_eq!(healths, [2, 1, 0]);
        // The first kick lands inside the tell (tick 33, the spit would leave on
        // 37): the tell is cancelled and no glob ever flew.
        assert!(
            events
                .iter()
                .all(|e| !matches!(e, CombatEvent::LlamaSpit { .. }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CombatEvent::LlamaInterrupted { tick: 33, .. }))
        );
        assert_eq!(sim.combat.llamas[0].mode, crate::llama::LlamaMode::Corpse);
        assert_eq!(sim.scene.world.hero.health, HERO_HEALTH);
    }

    #[test]
    fn jab_haymaker_jab_defeat_the_llama_with_a_routine_force() {
        let (_, events) = llama_events(&CP05_LLAMA_DOWN);
        assert_eq!(
            defeat_rows(&events),
            [(63, 0, DefeatCause::Melee, ForceTier::Routine)]
        );
        let moves: Vec<MoveId> = starts(&events).into_iter().map(|(_, m)| m).collect();
        assert_eq!(moves, [MoveId::Jab, MoveId::Haymaker, MoveId::Jab]);
    }

    #[test]
    fn the_llama_spits_after_a_36_tick_tell_and_hurts_carl_twice() {
        let (sim, events) = llama_events(&CP05_LLAMA_SPIT);
        let tells = event_ticks(&events, |e| match e {
            CombatEvent::LlamaTell { tick, .. } => Some(*tick),
            _ => None,
        });
        let spits = event_ticks(&events, |e| match e {
            CombatEvent::LlamaSpit { tick, .. } => Some(*tick),
            _ => None,
        });
        assert_eq!(tells, [1, 151]);
        // The glob leaves 36 ticks after each tell (attack tick 36).
        assert_eq!(spits, [37, 187]);
        let hurt = hurts(&events);
        assert_eq!(hurt, [76, 226], "one hit per glob, 150 ticks apart");
        // The second hit comes after the first one's invulnerability ended.
        assert!(hurt[1] - hurt[0] > u64::from(HERO_INVULNERABLE_TICKS));
        assert_eq!(sim.scene.world.hero.health, HERO_HEALTH - 2);
        // Each glob is gone once it hit (none in flight at the end).
        assert!(sim.combat.globs.is_empty());
    }

    #[test]
    fn a_llama_outside_the_view_walks_but_never_tells_or_spits() {
        let (sim, events) = llama_events(&CP05_LLAMA_OFFSCREEN);
        assert!(events.iter().all(|e| !matches!(
            e,
            CombatEvent::LlamaTell { .. } | CombatEvent::LlamaSpit { .. }
        )));
        assert!(hurts(&events).is_empty());
        // It walked toward Carl up to its 160 px leash and no farther.
        let x = sim.combat.llamas[0].x;
        assert!((x - 840.0).abs() < 1.5, "x {x}");
        // The scroll view stayed on Carl: the llama was out of it the whole run.
        let left = sim.scene.camera.left() as f32;
        assert!(x - 23.0 > left + 640.0);
    }

    #[test]
    fn a_llama_that_comes_into_view_starts_its_tell() {
        // Carl walks right: the scroll view follows him, the llama at 1000
        // enters it (its body must be fully inside) and the tell starts.
        static APPROACH: Fixture = Fixture {
            name: "test_llama_approach",
            summary: "test",
            seed: 21,
            carl_x: 400,
            enemies: &[],
            shore_dx: None,
            boss: None,
            llamas: &[LlamaSpawn {
                x: 1000,
                ai: EnemyAi::Chase,
            }],
            charges: &[],
            script: &[walk(1, 200, 1.0)],
            ticks: 400,
        };
        let mut sim = sim_of(&APPROACH);
        let mut first_tell = None;
        for _ in 0..APPROACH.ticks {
            sim.step_scripted();
            if first_tell.is_none()
                && sim
                    .combat
                    .events
                    .iter()
                    .any(|e| matches!(e, CombatEvent::LlamaTell { .. }))
            {
                first_tell = Some(sim.scene.world.tick);
                let body = sim.combat.llamas[0].body();
                let left = sim.scene.camera.left() as f32;
                assert!(body.left >= left && body.right() <= left + 640.0);
                let dx = sim.scene.world.hero.x - sim.combat.llamas[0].x;
                assert!(dx.abs() <= 320.0);
            }
        }
        assert!(first_tell.is_some(), "it tells once it is in view");
    }

    #[test]
    fn a_jump_clears_the_glob_and_standing_still_does_not() {
        let hurt_when = |press: Option<u64>| {
            let script: &'static [Hold] = match press {
                Some(tick) => Box::leak(Box::new([jump(tick, tick + 1)])),
                None => &[],
            };
            let fixture: &'static Fixture = Box::leak(Box::new(Fixture {
                name: "test_llama_jump",
                summary: "test",
                seed: 23,
                carl_x: 400,
                enemies: &[],
                shore_dx: None,
                boss: None,
                llamas: &[LlamaSpawn {
                    x: 560,
                    ai: EnemyAi::Chase,
                }],
                charges: &[],
                script,
                ticks: 120,
            }));
            let mut sim = sim_of(fixture);
            sim.run_scripted(fixture.ticks);
            !hurts(&events_of(&sim)).is_empty()
        };
        assert!(hurt_when(None), "standing still, the first glob hits");
        // Some jump timings clear it (the glob is 6 px tall at mouth height, 45
        // px up); a jump that is far too early or too late does not.
        let cleared: Vec<u64> = (40..90).filter(|t| !hurt_when(Some(*t))).collect();
        assert!(
            cleared.len() >= 8,
            "a window of at least 8 ticks clears it: {cleared:?}"
        );
        assert!(
            cleared.windows(2).filter(|w| w[1] == w[0] + 1).count() >= 6,
            "the window is contiguous: {cleared:?}"
        );
        assert!(hurt_when(Some(200)), "a jump after the glob does not");
    }

    #[test]
    fn a_glob_splats_on_an_invulnerable_carl_without_damage_or_an_event() {
        let mut sim = sim_of(&CP05_LLAMA_SPIT);
        // The glob leaves on tick 37 and would reach Carl on 76.
        sim.run_scripted(60);
        assert_eq!(sim.combat.globs.len(), 1);
        sim.scene.world.hero.invulnerable_ticks = 55;
        sim.run_scripted(40);
        assert!(sim.combat.globs.is_empty(), "the glob splatted on him");
        assert!(hurts(&events_of(&sim)).is_empty(), "no hurt event");
        assert_eq!(sim.scene.world.hero.health, HERO_HEALTH, "no damage");
    }

    #[test]
    fn a_glob_spawning_behind_a_close_carl_still_hits_him() {
        // Carl 20 px in front of the llama: its mouth (46 px ahead) is behind
        // him, so the glob starts on his near face and hits on the spit tick.
        static POINT_BLANK: Fixture = Fixture {
            name: "test_llama_point_blank",
            summary: "test",
            seed: 27,
            carl_x: 420,
            enemies: &[],
            shore_dx: None,
            boss: None,
            llamas: &[LlamaSpawn {
                x: 440,
                ai: EnemyAi::Chase,
            }],
            charges: &[],
            script: &[],
            ticks: 60,
        };
        let (_, events) = llama_events(&POINT_BLANK);
        assert_eq!(hurts(&events), [37], "hit on the tick the glob leaves");
    }

    #[test]
    fn a_glob_flying_into_a_wall_is_stopped_by_it() {
        let mut sim = sim_of(&CP05_LLAMA_SPIT);
        // A solid between the llama and Carl, at mouth height, stops the glob.
        sim.scene.world.solids.push(crate::prototype::Block {
            rect: Rect {
                left: 470.0,
                bottom: 0.0,
                width: 10.0,
                height: 1000.0,
            },
            material: crate::prototype::Material::Stone,
            intact: true,
        });
        sim.run_scripted(120);
        assert!(hurts(&events_of(&sim)).is_empty(), "the wall took the glob");
        assert!(sim.combat.globs.is_empty());
    }

    #[test]
    fn the_shore_block_crushes_a_llama_under_it_once() {
        let (sim, events) = llama_events(&CP05_LLAMA_CRUSH);
        assert_eq!(
            defeat_rows(&events),
            [(
                139,
                0,
                DefeatCause::ShoreBlock { player: 0 },
                ForceTier::Catastrophic
            )]
        );
        // It lies where the block fell, then it is a corpse.
        assert!(!sim.combat.llamas[0].alive());
        assert_eq!(sim.combat.llamas[0].x, 484.0);
    }

    #[test]
    fn llama_target_ids_follow_the_goblins_and_the_state_hash_sees_the_llama() {
        // One goblin and one llama in reach: the goblin is target 0, the llama
        // target 1; one move can hit both and the events say which.
        static MIXED: Fixture = Fixture {
            name: "test_mixed",
            summary: "test",
            seed: 25,
            carl_x: 400,
            enemies: &[EnemySpawn {
                x: 428,
                ai: EnemyAi::Passive,
            }],
            shore_dx: None,
            boss: None,
            llamas: &[LlamaSpawn {
                x: 440,
                ai: EnemyAi::Passive,
            }],
            charges: &[],
            script: &[attack(10, 11)],
            ticks: 40,
        };
        let (sim, events) = llama_events(&MIXED);
        let mut targets: Vec<u32> = hits(&events).into_iter().map(|(_, t)| t).collect();
        targets.sort_unstable();
        assert_eq!(targets, [0, 1]);
        assert_eq!(sim.combat.enemies[0].health, ENEMY_HEALTH - 1);
        assert_eq!(sim.combat.llamas[0].health, crate::feel::LLAMA_HEALTH - 1);
        // The llama is part of the state hash: a different llama column differs.
        static OTHER: Fixture = Fixture {
            llamas: &[LlamaSpawn {
                x: 441,
                ai: EnemyAi::Passive,
            }],
            ..MIXED
        };
        let mut other = sim_of(&OTHER);
        other.run_scripted(OTHER.ticks);
        assert_ne!(sim.state_hash(), other.state_hash());
    }

    #[test]
    fn a_restart_brings_the_llama_back_and_clears_the_globs() {
        let mut sim = sim_of(&CP05_LLAMA_SPIT);
        sim.run_scripted(60);
        assert!(!sim.combat.globs.is_empty(), "a glob is flying");
        sim.restart();
        assert!(sim.combat.globs.is_empty());
        assert_eq!(sim.combat.llamas[0].x, 560.0);
        assert_eq!(sim.combat.llamas[0].health, crate::feel::LLAMA_HEALTH);
        // The attempt after a restart is the attempt of a fresh run: the same
        // llama and glob state tick for tick.
        let mut fresh = sim_of(&CP05_LLAMA_SPIT);
        sim.run_scripted(CP05_LLAMA_SPIT.ticks);
        fresh.run_scripted(CP05_LLAMA_SPIT.ticks);
        assert_eq!(sim.combat.llamas, fresh.combat.llamas);
        assert_eq!(sim.combat.globs, fresh.combat.globs);
        assert_eq!(sim.scene.world.hero.health, fresh.scene.world.hero.health);
    }

    /// B1: the live-party hash covers each goblin's target, Donut's hit state
    /// and the defeated hero, and the legacy hash ignores all three.
    #[test]
    fn the_party_hash_covers_targets_donut_hit_state_and_the_defeated_hero() {
        use crate::party_scene::{PartyFixtureSim, SOLO_FOLLOWER_DODGE};
        let mut sim = PartyFixtureSim::new(&kit(), 360, &SOLO_FOLLOWER_DODGE).unwrap();
        sim.run_scripted(40);
        assert!(sim.scene.party_live());
        let base = sim.state_hash();
        let mut changed = sim.clone();
        changed.combat.targets[0] = Some(changed.combat.target_of(0).other());
        assert_ne!(
            changed.state_hash(),
            base,
            "a target change alters the hash"
        );
        let mut changed = sim.clone();
        changed.combat.donut.hit_stop += 1;
        assert_ne!(changed.state_hash(), base, "Donut hit-stop alters the hash");
        let mut changed = sim.clone();
        changed.combat.donut.hurt_ticks += 1;
        assert_ne!(changed.state_hash(), base, "Donut flinch alters the hash");
        let mut changed = sim.clone();
        changed.combat.donut.vx += 1.0;
        assert_ne!(changed.state_hash(), base, "Donut drift alters the hash");
        let mut changed = sim.clone();
        changed.combat.defeated_hero = HeroId::Donut;
        assert_ne!(
            changed.state_hash(),
            base,
            "the defeated hero alters the hash"
        );

        // Legacy: the same fields do not reach the hash while the party is not live.
        let mut legacy = sim_of(FIXTURES[0]);
        legacy.step(RawInput::default());
        let before = legacy.state_hash();
        legacy.combat.targets = vec![Some(HeroId::Donut); legacy.combat.targets.len()];
        legacy.combat.donut.hit_stop = 3;
        legacy.combat.defeated_hero = HeroId::Donut;
        assert_eq!(legacy.state_hash(), before);
    }

    /// S5 / Q4: a goblin swing reaches Donut's forgiving box (20 px wide) at 40
    /// px between centres and Carl's 24 px box at 42, on either side, and the
    /// box is centred on the pivot (mirrored with the goblin's facing).
    #[test]
    fn q4_a_swing_reaches_donut_at_40_px_and_carl_at_42_on_both_sides() {
        use crate::party_scene::{PartyFixtureSim, SOLO_FOLLOWER_DODGE};
        let mut sim = PartyFixtureSim::new(&kit(), 360, &SOLO_FOLLOWER_DODGE).unwrap();
        sim.run_scripted(1);
        assert!(sim.scene.party_live());
        assert_eq!(
            sim.scene.drive().tuning.donut_hurtbox,
            crate::party::DonutHurtbox::Forgiving
        );
        let cases = [
            // (hero, distance, reached)
            (HeroId::Donut, 39.0, true),
            (HeroId::Donut, 41.0, false),
            (HeroId::Donut, 45.0, false),
            (HeroId::Donut, 47.0, false),
            (HeroId::Carl, 39.0, true),
            (HeroId::Carl, 41.0, true),
            (HeroId::Carl, 43.0, false),
            (HeroId::Carl, 45.0, false),
            (HeroId::Carl, 47.0, false),
        ];
        for (hero, distance, reached) in cases {
            for side in [1.0_f32, -1.0] {
                let body = sim.scene.world.body(hero);
                let (x, feet) = (body.x, body.y - HERO_HALF_HEIGHT);
                let goblin = &mut sim.combat.enemies[0];
                goblin.x = x + side * distance;
                goblin.y = feet + crate::combat::ENEMY_HALF_HEIGHT;
                // The goblin faces the hero.
                goblin.facing = if side > 0.0 { -1 } else { 1 };
                let hit = goblin
                    .swing_box()
                    .overlaps(CombatState::hurtbox(&sim.scene, hero));
                assert_eq!(hit, reached, "{hero:?} at {distance} px, side {side}");
            }
        }
    }
}
