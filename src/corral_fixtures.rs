//! CORRAL-B wire: the isolated `corral_*` fixtures of the llama rush and of
//! llama party targeting (spec `enemy-gameplay-llama-rush.md` section 15), and
//! the name-replay hooks that make them replayable (plan KC2, KC15, R2).
//!
//! Every fixture runs on the corridor kit under CF02 rules (the name arm of
//! `cf02::fixture_runs_cf02`), party live, with **one** llama at `E = 600`
//! facing -1, awake, cooldowns 0, and the tested hero at `E - d` facing +1. The
//! other hero stands out of the llama's 320 px range. Attack tick 0 is the
//! llama's logged `LlamaRushTell`; an input "on attack tick n" is the scripted
//! tick `T0 + n` (the llama tells on scene tick `T0 = 1`, asserted by a test).
//!
//! The hooks are exact-name lookups (never a prefix), so the CORRAL-EVAL
//! evaluator stages (`corral_eval_*`) and every other fixture are untouched:
//!
//! * [`runs_cf02`] is the `cf02::fixture_runs_cf02` arm (KC2);
//! * [`donut_start_x`] is the `cf02_fixtures::donut_start_x` delegate (KC15);
//! * [`llama_opt_ins`] is applied in `CombatState::reset_attempt` (R2, KC15):
//!   party targeting, each llama's profile, surface, leash, facing and
//!   activation, and the fixture's authored solids, so a retry and a name replay
//!   match a builder-built run;
//! * [`replayable`] is the `PartyFixture::all()` chain entry (KC15).

use crate::boss_scene::Pilot;
use crate::combat::EnemyAi;
use crate::combat_scene::corral::LlamaOptInSet;
use crate::combat_scene::{Fixture, LlamaSpawn};
use crate::donut_fixtures::primary;
use crate::donut_mana::DonutSetup;
use crate::llama::{LlamaOptIn, RushOptIn};
use crate::llama_rush::{self, Leash};
use crate::party::PROVISIONAL_TUNING;
use crate::party_scene::{PartyFixture, SlotHold, jump, run};
use crate::prototype::{Block, Material, Rect};

/// The llama's column in every isolated fixture (spec section 15).
pub const E: i32 = 600;
/// The scene tick the llama starts its tell on: attack tick `n` is `T0 + n`.
pub const T0: u64 = 1;
/// The hero who is far: outside the llama's 320 px range and out of its way.
const FAR: i32 = 240;
/// A tick count that covers a rush (tell 32 + 24 active + recover 40) twice.
const TICKS: u64 = 220;

/// What a fixture's llama is configured with (beyond the corral profile).
#[derive(Debug, Clone, Copy)]
pub struct Cfg {
    /// `false`: the M1 llama (party targeting, no rush profile).
    pub rush: bool,
    /// The llama's authored leash on its pivot x.
    pub leash: (f32, f32),
    /// Starting health (`None`: the legacy 3).
    pub health: Option<u8>,
    /// A wall whose **face** (right edge) is at this x, 20 px thick.
    pub wall_face: Option<f32>,
    /// The llama starts dormant.
    pub dormant: bool,
}

const CFG: Cfg = Cfg {
    rush: true,
    leash: (E as f32 - 200.0, E as f32 + 200.0),
    health: None,
    wall_face: None,
    dormant: false,
};

/// One registered fixture.
pub struct Entry {
    pub fixture: &'static PartyFixture,
    /// Where Donut starts (`None`: her default follow distance).
    pub donut_x: Option<f32>,
    pub cfg: Cfg,
    /// Listed in `PartyFixture::all()`. A fixture whose scenario needs a state
    /// write at tick 0 (an invulnerable hero, a hero out of view) is tests-only.
    pub replay: bool,
}

const fn at(n: u64) -> u64 {
    T0 + n
}

macro_rules! corral_fixture {
    (
        $base:ident, $fix:ident, $name:literal, $summary:literal, $seed:literal,
        carl $carl:expr, llama $llama:expr, ticks $ticks:expr, two $two:expr,
        donut $donut:expr, script $script:expr
    ) => {
        static $base: Fixture = Fixture {
            name: $name,
            summary: $summary,
            seed: $seed,
            carl_x: $carl,
            enemies: &[],
            shore_dx: None,
            boss: None,
            engineers: &[],
            llamas: &[LlamaSpawn {
                x: $llama,
                ai: EnemyAi::Chase,
            }],
            charges: &[],
            script: &[],
            ticks: $ticks,
            room: crate::combat_scene::FixtureRoom::Corridor,
        };
        pub static $fix: PartyFixture = PartyFixture {
            base: &$base,
            two_player: $two,
            tuning: PROVISIONAL_TUNING,
            script: $script,
            pilot: Pilot::None,
            donut: $donut,
        };
    };
}

// ------------------------------------------------------------- rush hits --

corral_fixture!(
    HITS_DONUT_BASE, HITS_IDLE_DONUT, "corral_rush_hits_idle_donut",
    "The corral llama tells, commits and rushes an idle Donut 96 px ahead: contact on active tick 15",
    0xC0DA_0001, carl FAR, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script &[]
);
corral_fixture!(
    HITS_CARL_BASE, HITS_IDLE_CARL, "corral_rush_hits_idle_carl",
    "The corral llama rushes an idle Carl 96 px ahead (Donut far): contact on active tick 15",
    0xC0DA_0002, carl E - 96, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script &[]
);
corral_fixture!(
    POINT_BLANK_BASE, POINT_BLANK, "corral_rush_point_blank",
    "Donut 24 px ahead: the rush box reaches her on the first active tick",
    0xC0DA_0003, carl FAR, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script &[]
);
corral_fixture!(
    TRIGGER_EDGE_BASE, TRIGGER_EDGE, "corral_rush_trigger_edge",
    "Donut exactly 128 px ahead: the trigger edge, contact on active tick 23",
    0xC0DA_0004, carl FAR, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script &[]
);

// --------------------------------------------------------------- evasion --

const JUMP_DONUT_4: &[SlotHold] = &[jump(at(4), at(4) + 1, 1)];
corral_fixture!(
    JUMP_DONUT_BASE, JUMP_CLEARS_DONUT, "corral_rush_jump_clears_donut",
    "Donut jumps on attack tick 4: the rush passes under her and misses",
    0xC0DA_0005, carl FAR, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script JUMP_DONUT_4
);
const JUMP_CARL_12: &[SlotHold] = &[jump(at(12), at(12) + 1, 0)];
corral_fixture!(
    JUMP_CARL_BASE, JUMP_CLEARS_CARL, "corral_rush_jump_clears_carl",
    "Carl jumps on attack tick 12: the rush passes under him and misses",
    0xC0DA_0006, carl E - 96, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script JUMP_CARL_12
);
const RUN_AWAY_44: &[SlotHold] = &[run(at(44), at(44) + 40, 1, -1.0)];
corral_fixture!(
    RUN_AWAY_BASE, RUN_AWAY, "corral_rush_run_away",
    "Donut runs away from attack tick 44: she outruns the end of the rush",
    0xC0DA_0007, carl FAR, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script RUN_AWAY_44
);

// ------------------------------------------------------------- interrupt --

const MISSILE_20: &[SlotHold] = &[primary(at(20), at(20) + 1, 1)];
corral_fixture!(
    MISSILE_BASE, MISSILE_INTERRUPT, "corral_rush_missile_interrupt",
    "Donut casts Magic Missile on attack tick 20: the bolt interrupts the tell, no commit",
    0xC0DA_0008, carl FAR, llama E, ticks TICKS, two true,
    donut DonutSetup::unlocked(8), script MISSILE_20
);
// Spec section 15 says "Primary on 21". The bolt flies 11 ticks, so a press on
// attack tick 21 lands on attack tick 32, and a strike resolves before the
// llama's own step of that tick: it still meets the Tell (the same hero-first
// order the section 6 fairness tables use). The first press that lands inside
// the commit is attack tick 22 (design erratum, reported in the PR).
const MISSILE_22: &[SlotHold] = &[primary(at(22), at(22) + 1, 1)];
corral_fixture!(
    COMMIT_LETHAL_BASE, COMMIT_LETHAL, "corral_rush_commit_lethal",
    "A one-health llama is hit by a bolt (press on attack tick 22) during the commit: it goes Down in place, no contact",
    0xC0DA_0009, carl FAR, llama E, ticks TICKS, two true,
    donut DonutSetup::unlocked(8), script MISSILE_22
);

// ------------------------------------------------------------ geometry --

const JUMP_DONUT_20: &[SlotHold] = &[jump(at(20), at(20) + 1, 1)];
corral_fixture!(
    BONK_BASE, BONK_WALL, "corral_rush_bonk_wall",
    "A wall face 84 px ahead: the rush box meets it on active tick 15 and the llama bonks",
    0xC0DA_000A, carl FAR, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script JUMP_DONUT_20
);
corral_fixture!(
    LEASH_BASE, LEASH_STOP, "corral_rush_leash_stop",
    "The leash bound is 40 px ahead: the rush stops at the bound, a miss, then Recover",
    0xC0DA_000B, carl FAR, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script JUMP_DONUT_20
);
const JUMP_RUN_20: &[SlotHold] = &[
    jump(at(20), at(20) + 1, 1),
    run(at(21), at(21) + 50, 1, 1.0),
];
corral_fixture!(
    NO_TRACKING_BASE, NO_TRACKING, "corral_rush_no_tracking",
    "Donut jumps over the llama and lands behind it: the committed rush never turns",
    0xC0DA_000C, carl FAR, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script JUMP_RUN_20
);
corral_fixture!(
    ONE_HIT_BASE, TWO_PLAYER_ONE_HIT, "corral_rush_2p_one_hit",
    "Carl 72 px and Donut 96 px ahead in the lane: the rush hits Carl only, one HeroHurt",
    0xC0DA_000D, carl E - 72, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script &[]
);
corral_fixture!(
    INVULN_BASE, INVULNERABLE_PASS, "corral_rush_invulnerable_pass",
    "Carl is invulnerable 60 px ahead and Donut 96 px: the box passes Carl and hits Donut",
    0xC0DA_000E, carl E - 60, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script &[]
);
corral_fixture!(
    OFFSCREEN_BASE, OFFSCREEN_NO_TELL, "corral_rush_offscreen_no_tell",
    "The lane end is outside the view: no tell, the llama walks or holds",
    0xC0DA_000F, carl FAR, llama E + 100, ticks TICKS, two true,
    donut DonutSetup::NONE, script &[]
);

// ----------------------------------------------------- targeting and spit --

corral_fixture!(
    SPIT_RULE_BASE, SPIT_HEIGHT_RULE, "corral_spit_height_rule",
    "Donut stands on the floor 160 px ahead: she is never spat at, the llama walks to 112 px and rushes",
    0xC0DA_0010, carl FAR, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script &[]
);
corral_fixture!(
    SPIT_CARL_BASE, SPIT_CARL_FLOOR, "corral_spit_carl_floor",
    "Carl stands on the floor 160 px ahead: the llama spits at him",
    0xC0DA_0011, carl E - 160, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script &[]
);
const HOP: &[SlotHold] = &[
    jump(2, 40, 1),
    jump(41, 80, 1),
    jump(81, 120, 1),
    jump(121, 160, 1),
];
corral_fixture!(
    HOP_BASE, SPIT_HOP_FLOOR, "corral_spit_hop_classified_floor",
    "Donut hops in place 160 px ahead: she is never spat at and stays a rush target",
    0xC0DA_0012, carl FAR, llama E, ticks 300, two true,
    donut DonutSetup::NONE, script HOP
);
corral_fixture!(
    M1_BASE, M1_NEVER_RUSHES, "corral_m1_llama_never_rushes",
    "The M1 llama (party targeting, no rush profile) with Donut 96 px ahead for 600 ticks: no rush tell",
    0xC0DA_0013, carl FAR, llama E, ticks 600, two true,
    donut DonutSetup::NONE, script &[]
);
corral_fixture!(
    SOLO_BASE, PARTY_TARGET_SOLO_DONUT, "corral_party_target_solo_donut",
    "Solo, Donut controlled, Carl 32 px nearer: the llama targets Donut (the solo bias of 40 against the switch of 24)",
    0xC0DA_0014, carl E - 64, llama E, ticks 60, two false,
    donut DonutSetup::NONE, script &[crate::party_scene::swap(1, 2)]
);

corral_fixture!(
    NOT_EVADABLE_BASE, NOT_EVADABLE, "corral_rush_not_evadable",
    "Donut with the biscuit (the reflex is armed only by the tests): the rush hits her and is never evaded",
    0xC0DA_001A, carl FAR, llama E, ticks TICKS, two true,
    donut DonutSetup::unlocked(8), script &[]
);

// ------------------------------------------------------------ KC12 pierce --

// The haymaker (a chained second Primary) with the reinforced fist: the jab on
// attack tick 32 is a damage-only hit on the committed rush, the haymaker
// pressed on 37 hits on attack tick 45, one tick before the box would reach
// Carl, and pierces the commit (CP-Q40, plan KC12).
const PIERCE_STRING: &[SlotHold] = &[
    primary(at(32), at(32) + 1, 0),
    primary(at(37), at(37) + 1, 0),
];
corral_fixture!(
    PIERCE_BASE, PIERCE_HAYMAKER, "corral_rush_pierce_haymaker",
    "Carl with the reinforced fist: the haymaker lands on attack tick 45 and pierces the committed rush, so the box never reaches him",
    0xC0DA_0018, carl E - 96, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script PIERCE_STRING
);
corral_fixture!(
    PIERCE_CONTROL_BASE, PIERCE_CONTROL, "corral_rush_pierce_control",
    "The control of corral_rush_pierce_haymaker without the reinforced fist: the same string only damages the committed llama and the rush hits Carl",
    0xC0DA_0019, carl E - 96, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script PIERCE_STRING
);

// ------------------------------------------------------- leash, reset, wake --

const KICK: &[SlotHold] = &[
    run(1, 4, 0, -1.0),
    crate::donut_fixtures::physical(10, 11, 0),
    primary(60, 61, 0),
    primary(80, 81, 0),
];
corral_fixture!(
    KICK_BASE, KICK_AT_LEASH_EDGE, "corral_kick_at_leash_edge",
    "A llama at its leash minimum facing +1: Carl kicks it toward the bound, then a lethal hit: x never leaves the leash",
    0xC0DA_0015, carl E + 30, llama E, ticks 200, two true,
    donut DonutSetup::NONE, script KICK
);
corral_fixture!(
    RETRY_BASE, RETRY_RESET, "corral_rush_retry_reset",
    "A retry during a commit restores the rush state, rush_seq and the cooldowns from the fixture",
    0xC0DA_0016, carl FAR, llama E, ticks TICKS, two true,
    donut DonutSetup::NONE, script &[]
);
const WAKE_RUN: &[SlotHold] = &[run(2, 80, 1, 1.0)];
corral_fixture!(
    WAKE_BASE, WAKE_DORMANT, "corral_wake_dormant",
    "A dormant llama holds until a living hero is within 320 px, then wakes and rushes",
    0xC0DA_0017, carl FAR, llama E, ticks 300, two true,
    donut DonutSetup::NONE, script WAKE_RUN
);

const fn entry(fixture: &'static PartyFixture, donut_x: i32, cfg: Cfg, replay: bool) -> Entry {
    Entry {
        fixture,
        donut_x: Some(donut_x as f32),
        cfg,
        replay,
    }
}

const FAR_X: i32 = FAR;

/// Every `corral_*` fixture of the wire stage.
pub static REGISTRY: [Entry; 26] = [
    entry(&HITS_IDLE_DONUT, E - 96, CFG, true),
    entry(&HITS_IDLE_CARL, FAR_X, CFG, true),
    entry(&POINT_BLANK, E - 24, CFG, true),
    entry(&TRIGGER_EDGE, E - 128, CFG, true),
    entry(&JUMP_CLEARS_DONUT, E - 96, CFG, true),
    entry(&JUMP_CLEARS_CARL, FAR_X, CFG, true),
    entry(&RUN_AWAY, E - 96, CFG, true),
    entry(&MISSILE_INTERRUPT, E - 96, CFG, true),
    entry(
        &COMMIT_LETHAL,
        E - 96,
        Cfg {
            health: Some(1),
            ..CFG
        },
        true,
    ),
    entry(
        &BONK_WALL,
        E - 64,
        Cfg {
            wall_face: Some(E as f32 - 84.0),
            ..CFG
        },
        true,
    ),
    entry(
        &LEASH_STOP,
        E - 96,
        Cfg {
            leash: (E as f32 - 40.0, E as f32 + 200.0),
            ..CFG
        },
        true,
    ),
    entry(&NO_TRACKING, E - 64, CFG, true),
    entry(&TWO_PLAYER_ONE_HIT, E - 96, CFG, true),
    entry(&INVULNERABLE_PASS, E - 96, CFG, false),
    entry(&OFFSCREEN_NO_TELL, E + 100 - 96, CFG, false),
    entry(&SPIT_HEIGHT_RULE, E - 160, CFG, true),
    entry(&SPIT_CARL_FLOOR, FAR_X, CFG, true),
    entry(&SPIT_HOP_FLOOR, E - 160, CFG, true),
    entry(&M1_NEVER_RUSHES, E - 96, Cfg { rush: false, ..CFG }, true),
    entry(&PARTY_TARGET_SOLO_DONUT, E - 96, CFG, true),
    entry(
        &KICK_AT_LEASH_EDGE,
        FAR_X,
        Cfg {
            leash: (E as f32, E as f32 + 200.0),
            ..CFG
        },
        true,
    ),
    entry(&RETRY_RESET, E - 96, CFG, true),
    entry(
        &WAKE_DORMANT,
        FAR,
        Cfg {
            dormant: true,
            ..CFG
        },
        true,
    ),
    entry(&PIERCE_HAYMAKER, FAR_X, CFG, true),
    entry(&PIERCE_CONTROL, FAR_X, CFG, true),
    entry(&NOT_EVADABLE, E - 96, CFG, true),
];

/// The `PartyFixture::all()` chain entry: every replayable fixture of the
/// registry, in registry order.
pub fn replayable() -> impl Iterator<Item = &'static PartyFixture> {
    REGISTRY.iter().filter(|e| e.replay).map(|e| e.fixture)
}

fn entry_of(name: &str) -> Option<&'static Entry> {
    REGISTRY.iter().find(|e| e.fixture.name() == name)
}

/// KC2: the exact fixture names that run under `cf02_rules` (the isolated
/// fixtures and the CORRAL-C room fixtures).
pub fn runs_cf02(name: &str) -> bool {
    entry_of(name).is_some() || room::is_room_fixture(name)
}

/// KC15: where Donut starts in a corral fixture.
pub fn donut_start_x(name: &str) -> Option<f32> {
    entry_of(name).and_then(|e| e.donut_x)
}

/// R2, KC15: the party targeting and per-llama opt-ins of a corral fixture, and
/// its authored solids. `None` for every other fixture, so nothing else changes.
pub fn llama_opt_ins(name: &str) -> Option<LlamaOptInSet> {
    if room::is_room_fixture(name) {
        return room::llama_opt_ins(name);
    }
    let entry = entry_of(name)?;
    let cfg = entry.cfg;
    let rush = cfg.rush.then(|| RushOptIn {
        profile: llama_rush::active(),
        leash: Leash {
            min: cfg.leash.0,
            max: cfg.leash.1,
        },
    });
    let solids = cfg
        .wall_face
        .map(|face| {
            vec![Block {
                rect: Rect {
                    left: face - 20.0,
                    bottom: 0.0,
                    width: 20.0,
                    height: 400.0,
                },
                material: Material::Stone,
                intact: true,
            }]
        })
        .unwrap_or_default();
    Some(LlamaOptInSet {
        party_target: true,
        opt_ins: vec![LlamaOptIn {
            index: 0,
            feet: None,
            facing: if name == "corral_kick_at_leash_edge" {
                1
            } else {
                -1
            },
            dormant: cfg.dormant,
            health: cfg.health,
            rush,
        }],
        solids,
    })
}

pub mod room;

#[cfg(test)]
mod room_tests;
#[cfg(test)]
mod tests;
