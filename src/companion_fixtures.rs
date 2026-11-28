//! COMPANION-AI-01 replay fixtures (01A-wire).
//!
//! The registry has the shape of [`crate::donut_fixtures::DONUT_FIXTURES`] and
//! is chained into [`PartyFixture::all`]. The `companion_*` fixtures use the
//! both-role profile and the `cf02_assist_*` fixtures (moved here from CF02-C2,
//! CA-30) the CF02 assistant; `--list-fixtures companion_` and
//! `--list-fixtures cf02_assist_` and the CI loops over them never need another
//! edit. Each fixture has a committed replay in `docs/evidence/companion-ai`
//! and a golden hash pinned by a test below.
//!
//! Tick numbering is the one of [`crate::combat_scene`]: a hold over
//! `from..to` covers the steps that produce `tick = from .. to - 1`.

use crate::boss_scene::Pilot;
use crate::cf02::arena;
use crate::combat::EnemyAi;
use crate::combat_scene::{EnemySpawn, Fixture, LlamaSpawn};
use crate::companion::profile::ProfileSetId;
use crate::donut_fixtures::primary;
use crate::donut_mana::{DonutSetup, StartState};
use crate::party::{PROVISIONAL_TUNING, PartyTuning, UncontrolledHero};
use crate::party_scene::{PartyFixture, base, jump, run, swap};

const fn chase(x: i32) -> EnemySpawn {
    EnemySpawn {
        x,
        ai: EnemyAi::Chase,
    }
}

/// The CF02 assistant: AI Donut active, AI Carl passive (CA-21).
const ASSIST: PartyTuning = PartyTuning {
    uncontrolled_hero: UncontrolledHero::Companion(ProfileSetId::Cf02Assist),
    ..PROVISIONAL_TUNING
};

/// Both roles active.
const V0: PartyTuning = PartyTuning {
    uncontrolled_hero: UncontrolledHero::Companion(ProfileSetId::CompanionV0),
    ..PROVISIONAL_TUNING
};

/// Hit points of a goblin of an assistant fixture that needs more than the
/// legacy 2 (`None`: the legacy 2), read through
/// [`crate::cf02_fixtures::goblin_health`].
pub fn goblin_health(fixture: &str) -> Option<u8> {
    match fixture {
        "cf02_assist_pounce_staggered" => Some(4),
        "cf02_assist_hp_drops_midmove" => Some(3),
        "cf02_assist_donut_guards_carls_back" => Some(4),
        _ => None,
    }
}

const fn dummy(x: i32) -> EnemySpawn {
    EnemySpawn {
        x,
        ai: EnemyAi::Passive,
    }
}

// ----------------------------------------------------------------- Q14 --

const ASSIST_SWAT_BASE: Fixture = base(
    "cf02_assist_swat_interrupt",
    "Solo as Carl, who stands still: a goblin walks at him and the AI Donut runs up and engages it (CF02-Q14 swat interrupt before CF02-DUO; now she pounces it, its swat-interrupt coverage is `legacy` and the trailing-gap fixture)",
    0xC0A1_0001,
    &[chase(520)],
    200,
);

/// CF02-Q14: AI Donut interrupts a tell aimed at the controlled hero.
pub const CF02_ASSIST_SWAT_INTERRUPT: PartyFixture = PartyFixture {
    base: &ASSIST_SWAT_BASE,
    two_player: false,
    tuning: ASSIST,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const ASSIST_GAP_BASE: Fixture = base(
    "cf02_assist_swat_trailing_gap",
    "Solo as Carl, who runs at a goblin: Donut trails him and starts a swat only if it can still land inside the tell (CF02-Q14, Q39)",
    0xC0A1_0002,
    &[chase(660)],
    240,
);

/// CF02-Q14: a swat starts only when its hit tick lands inside the tell.
pub const CF02_ASSIST_SWAT_TRAILING_GAP: PartyFixture = PartyFixture {
    base: &ASSIST_GAP_BASE,
    two_player: false,
    tuning: ASSIST,
    script: &[run(10, 110, 0, 1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const ASSIST_POUNCE_BASE: Fixture = base(
    "cf02_assist_pounce_staggered",
    "Solo as Carl: he jabs a 4 HP goblin the AI Donut pounces, then jabs it again and finishes it; the AI never takes the last hit of the enemy Carl is engaging (CF02-DUO-A08, spec C-11), the pre-DUO never-kill scenario runs as a regression test under `companion_v0`",
    0xC0A1_0003,
    &[dummy(430)],
    160,
);

/// CF02-Q15, Q38 and CF02-DUO-A08: a non-lethal assist pounce, Carl finishes.
/// (The pounce starts on a staggered goblin but lands after the stagger: a
/// pounce that lands on a staggered goblin is a design follow-up.) Carl's second
/// jab leaves the goblin at 1 HP while its combo window runs, and the AI Donut
/// holds back (the first cut had her pounce start in the same tick and take
/// the kill); Carl's third jab is the finisher.
pub const CF02_ASSIST_POUNCE_STAGGERED: PartyFixture = PartyFixture {
    base: &ASSIST_POUNCE_BASE,
    two_player: false,
    tuning: ASSIST,
    script: &[
        primary(5, 6, 0),
        primary(46, 47, 0),
        run(58, 70, 0, 1.0),
        primary(72, 73, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const ASSIST_GLANCE_BASE: Fixture = base(
    "cf02_assist_hp_drops_midmove",
    "Solo as Carl: the AI Donut starts a pounce, Carl's jab lowers the 3 HP goblin before it lands; under `factual` (CF02-DUO-A01) the pounce is a real hit, the glance (CF02-Q38) is regression-tested under `companion_v0`",
    0xC0A1_0004,
    &[dummy(430)],
    160,
);

/// CF02-Q38 (AI part): the lethality re-check on the hit tick.
pub const CF02_ASSIST_HP_DROPS_MIDMOVE: PartyFixture = PartyFixture {
    base: &ASSIST_GLANCE_BASE,
    two_player: false,
    tuning: ASSIST,
    script: &[
        primary(5, 6, 0),
        primary(26, 27, 0),
        run(40, 52, 0, 1.0),
        primary(54, 55, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const ASSIST_CADENCE_BASE: Fixture = base(
    "cf02_assist_cadence",
    "Solo as Carl, who stands: two goblins tell one after the other (a goblin at 440 and one at 500, asserted) and the AI Donut starts at most one move per 20 ticks (CF02-DUO-A02, was 60 under CF02-Q16), never while a move runs",
    0xC0A1_0005,
    &[chase(440), chase(500)],
    320,
);

/// CF02-Q16: one AI-started move per 60 ticks.
pub const CF02_ASSIST_CADENCE: PartyFixture = PartyFixture {
    base: &ASSIST_CADENCE_BASE,
    two_player: false,
    tuning: ASSIST,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const fn llama_base(
    name: &'static str,
    summary: &'static str,
    seed: u64,
    llamas: &'static [LlamaSpawn],
    ticks: u64,
) -> Fixture {
    Fixture {
        llamas,
        ..base(name, summary, seed, &[], ticks)
    }
}

const fn llama(x: i32) -> LlamaSpawn {
    LlamaSpawn {
        x,
        ai: EnemyAi::Chase,
    }
}

/// A fixture that starts with the biscuit and `mana` charges.
const fn biscuit(mana: u8) -> DonutSetup {
    DonutSetup {
        start: StartState {
            unlocked: true,
            mana,
        },
        guild_zone: None,
    }
}

const ASSIST_LOCKED_BASE: Fixture = llama_base(
    "cf02_assist_magic_locked",
    "Solo as Carl, who stands: a llama tells at him from range with the biscuit still locked, so the AI Donut never casts (CF02-Q17)",
    0xC0A1_0006,
    &[llama(510)],
    260,
);

/// CF02-Q17: no magic before the unlock.
pub const CF02_ASSIST_MAGIC_LOCKED: PartyFixture = PartyFixture {
    base: &ASSIST_LOCKED_BASE,
    two_player: false,
    tuning: ASSIST,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const ASSIST_RESERVE_BASE: Fixture = llama_base(
    "cf02_assist_missile_reserve",
    "Solo as Carl, who stands, biscuit unlocked with 3 charges: the AI Donut casts a Magic Missile at a llama's tell (3 to 2) and casts again only after the pool regenerates to 3, always keeping the reserve of 2 (CF02-Q17, Q10-A09)",
    0xC0A1_0007,
    &[llama(510)],
    420,
);

/// CF02-Q17: the AI keeps the mana reserve.
pub const CF02_ASSIST_MISSILE_RESERVE: PartyFixture = PartyFixture {
    base: &ASSIST_RESERVE_BASE,
    two_player: false,
    tuning: ASSIST,
    script: &[],
    pilot: Pilot::None,
    donut: biscuit(3),
};

const ASSIST_CAMERA_BASE: Fixture = base(
    "cf02_assist_camera_parity",
    "Solo as Carl in a two goblin fight: the AI Donut's decisions depend on the simulation view only, so `--camera auto` and `--camera scroll` replay it identically (CF02-Q40)",
    0xC0A1_0008,
    &[chase(520), chase(560)],
    360,
);

/// CF02-Q40: "in view" is the simulation scroll camera, never the presentation
/// camera; CI replays this fixture under both `--camera` modes.
pub const CF02_ASSIST_CAMERA_PARITY: PartyFixture = PartyFixture {
    base: &ASSIST_CAMERA_BASE,
    two_player: false,
    tuning: ASSIST,
    script: &[
        primary(95, 96, 0),
        primary(112, 113, 0),
        run(130, 150, 0, 1.0),
        primary(160, 161, 0),
        primary(176, 177, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const ASSIST_DEFEATS_BASE: Fixture = base(
    "cf02_assist_donut_defeats_goblin",
    "Solo as Carl, who stands and never presses a button: the AI Donut engages the goblin on her own, pounces it, and her own pounce defeats it (CF02-DUO-A01, A02, A04, A06; she finishes it because Carl engages nothing, A08)",
    0xC0A1_0009,
    &[chase(520)],
    420,
);

/// CF02-DUO: owner playtest feedback 2026-11-21 ("I expected Donut attacking").
/// No human input at all: the goblin is defeated by AI Donut, before the biscuit.
pub const CF02_ASSIST_DONUT_DEFEATS_GOBLIN: PartyFixture = PartyFixture {
    base: &ASSIST_DEFEATS_BASE,
    two_player: false,
    tuning: ASSIST,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const ASSIST_MISSILE_DEFEATS_BASE: Fixture = base(
    "cf02_assist_donut_missiles_goblin",
    "Solo as Carl, who stands and never presses a button, biscuit unlocked with a full pool: the AI Donut casts Magic Missiles at the goblin from range and defeats it, keeping the reserve of 2 (CF02-DUO-A01, A04, A05)",
    0xC0A1_000A,
    &[chase(560)],
    420,
);

/// CF02-DUO: the offensive missile, within the mana rules.
pub const CF02_ASSIST_DONUT_MISSILES_GOBLIN: PartyFixture = PartyFixture {
    base: &ASSIST_MISSILE_DEFEATS_BASE,
    two_player: false,
    tuning: ASSIST,
    script: &[],
    pilot: Pilot::None,
    donut: biscuit(8),
};

const ASSIST_BEHIND_BASE: Fixture = base(
    "cf02_assist_donut_guards_carls_back",
    "Solo as Carl, who works a 4 HP goblin ahead of him while a second 4 HP goblin walks up from behind: the AI Donut turns to the one at his back, pounces it and defeats it, and never takes the last hit of the goblin Carl is hitting (CF02-DUO-A08, A11, A12, A13)",
    0xC0A1_000B,
    &[dummy(430), chase(330)],
    400,
);

/// CF02-DUO round 2: Donut attacks what is behind Carl before the biscuit,
/// without stealing the goblin he is working. Carl's three hits land on 8, 34
/// and 60; his later jabs whiff (the goblin is out of reach), and after tick
/// 80 he stands still, so the dummy is hers once the 31 tick window is over.
pub const CF02_ASSIST_DONUT_GUARDS_CARLS_BACK: PartyFixture = PartyFixture {
    base: &ASSIST_BEHIND_BASE,
    two_player: false,
    tuning: ASSIST,
    script: &[
        primary(5, 6, 0),
        run(18, 27, 0, 1.0),
        primary(31, 32, 0),
        run(44, 53, 0, 1.0),
        primary(57, 58, 0),
        run(70, 76, 0, 1.0),
        primary(80, 81, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const ASSIST_FLANK_BASE: Fixture = base(
    "cf02_assist_donut_flanks_two_goblins",
    "Solo as Carl, who walks right into two goblins that walk at him: the AI Donut meets them on a flank spot ahead of him and pounces before they reach him, and Carl defeats them; she never takes a last hit of a goblin he is working (CF02-DUO-A15 with A08, A11, A12)",
    0xC0A1_000C,
    &[chase(660), chase(700)],
    200,
);

/// CF02-DUO round 3 (A15): Carl walks 120 px, pauses while the goblins close in
/// (she gets ahead of him and pounces the first, tick 67), steps up and jabs it
/// (113), and does the same for the second (her pounce 116, his jab 136).
pub const CF02_ASSIST_DONUT_FLANKS_TWO_GOBLINS: PartyFixture = PartyFixture {
    base: &ASSIST_FLANK_BASE,
    two_player: false,
    tuning: ASSIST,
    script: &[
        run(5, 45, 0, 1.0),
        run(95, 108, 0, 1.0),
        primary(110, 111, 0),
        run(130, 134, 0, 1.0),
        primary(136, 137, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// ---------------------------------------------------------- companion_v0 --

const V0_CARL_BASE: Fixture = base(
    "companion_carl_interrupts_for_donut",
    "Solo as Donut, who runs up to a goblin: the AI Carl (both roles active) jabs the tell aimed at her; he never grabs, throws or kicks (CA-37)",
    0xC0A2_0001,
    &[chase(560)],
    260,
);

/// 01A both-role: AI Carl interrupts a tell aimed at the controlled Donut.
pub const COMPANION_CARL_INTERRUPTS_FOR_DONUT: PartyFixture = PartyFixture {
    base: &V0_CARL_BASE,
    two_player: false,
    tuning: V0,
    script: &[swap(3, 4), run(5, 60, 0, 1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const V0_DONUT_BASE: Fixture = base(
    "companion_donut_supports_carl",
    "Solo as Carl in a two goblin fight with the both-role profile: the AI Donut swats and pounces, the AI side never lands a kill (CA-19)",
    0xC0A2_0002,
    &[chase(520), chase(560)],
    360,
);

/// 01A both-role: AI Donut supports the controlled Carl.
pub const COMPANION_DONUT_SUPPORTS_CARL: PartyFixture = PartyFixture {
    base: &V0_DONUT_BASE,
    two_player: false,
    tuning: V0,
    script: &[
        primary(95, 96, 0),
        primary(112, 113, 0),
        run(130, 150, 0, 1.0),
        primary(160, 161, 0),
        primary(176, 177, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const V0_DUO_BASE: Fixture = base(
    "companion_duo_no_ai",
    "Two players with the both-role profile selected: no hero is uncontrolled, so the companion never runs and the replay equals the bot tuning's (CA-22)",
    0xC0A2_0003,
    &[chase(520)],
    200,
);

/// 2P: the companion never runs and hashes nothing.
pub const COMPANION_DUO_NO_AI: PartyFixture = PartyFixture {
    base: &V0_DUO_BASE,
    two_player: true,
    tuning: V0,
    script: &[
        run(70, 80, 1, 1.0),
        primary(95, 96, 1),
        primary(100, 101, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const V0_SWAP_BASE: Fixture = base(
    "companion_swap_mid_move",
    "Solo: the AI Donut starts a swat and the player swaps to her mid-move; the swat finishes, the AI Carl takes over and the player is never forced (CF02-Q16)",
    0xC0A2_0004,
    &[chase(520)],
    240,
);

/// A swap never cancels a move; the other hero becomes the companion.
pub const COMPANION_SWAP_MID_MOVE: PartyFixture = PartyFixture {
    base: &V0_SWAP_BASE,
    two_player: false,
    tuning: V0,
    script: &[swap(106, 107), run(110, 140, 0, 1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// ------------------------------------------------------ 01B-wire (nav) --
//
// Room fixtures in the CF02 microarena (named `companion_arena_*`, which the
// arena predicate `cf02::arena::fixture_uses_arena` maps to the arena kit and
// the CF02 rules): no enemies, the one thing under test is the companion
// reaching the leader's tier by the nav graph. Columns are the arena's Q29
// layout: the left perch slab is 544..672 with its top 80 px up and an underside
// 68 px up, so a hero can walk under it.

/// The arena room with nothing in it but the two heroes.
const fn nav_room(name: &'static str, summary: &'static str, seed: u64, ticks: u64) -> Fixture {
    Fixture {
        carl_x: 470,
        enemies: &[],
        llamas: &[],
        shore_dx: None,
        ..arena::fixture(name, summary, seed, ticks)
    }
}

const NAV_DONUT_BASE: Fixture = nav_room(
    "companion_arena_nav_perch_donut",
    "01B-wire, solo as Carl: Carl jumps onto the left perch slab and walks to its right half; the AI Donut, on the floor, is routed by the nav graph to the slab's takeoff x, stops on it, jumps and lands on the slab beside him",
    0xC0A3_0001,
    260,
);

/// 01B-wire: AI Donut reaches a higher tier by the nav graph.
pub const COMPANION_ARENA_NAV_PERCH_DONUT: PartyFixture = PartyFixture {
    base: &NAV_DONUT_BASE,
    two_player: false,
    tuning: V0,
    script: &[run(3, 52, 0, 1.0), jump(8, 9, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const NAV_CARL_BASE: Fixture = nav_room(
    "companion_arena_nav_perch_carl",
    "01B-wire, solo as Donut: the player swaps to Donut, who jumps onto the left perch slab and walks to its right half; the AI Carl is routed by the nav graph up onto the slab",
    0xC0A3_0002,
    260,
);

/// 01B-wire: AI Carl reaches a higher tier by the nav graph.
pub const COMPANION_ARENA_NAV_PERCH_CARL: PartyFixture = PartyFixture {
    base: &NAV_CARL_BASE,
    two_player: false,
    tuning: V0,
    script: &[swap(3, 4), run(6, 56, 0, 1.0), jump(11, 12, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

pub const COMPANION_FIXTURES: [&PartyFixture; 18] = [
    &CF02_ASSIST_SWAT_INTERRUPT,
    &CF02_ASSIST_SWAT_TRAILING_GAP,
    &CF02_ASSIST_POUNCE_STAGGERED,
    &CF02_ASSIST_HP_DROPS_MIDMOVE,
    &CF02_ASSIST_CADENCE,
    &CF02_ASSIST_MAGIC_LOCKED,
    &CF02_ASSIST_MISSILE_RESERVE,
    &CF02_ASSIST_CAMERA_PARITY,
    &CF02_ASSIST_DONUT_DEFEATS_GOBLIN,
    &CF02_ASSIST_DONUT_MISSILES_GOBLIN,
    &CF02_ASSIST_DONUT_GUARDS_CARLS_BACK,
    &CF02_ASSIST_DONUT_FLANKS_TWO_GOBLINS,
    &COMPANION_CARL_INTERRUPTS_FOR_DONUT,
    &COMPANION_DONUT_SUPPORTS_CARL,
    &COMPANION_DUO_NO_AI,
    &COMPANION_SWAP_MID_MOVE,
    &COMPANION_ARENA_NAV_PERCH_DONUT,
    &COMPANION_ARENA_NAV_PERCH_CARL,
];

// ---------------------------------------------------------------- tally --

/// Per-run companion numbers, attributed by AP-Q14 (the hero-index `instigator`
/// of an event, `Hit::move_id.hero()` for a hit) and by who the companion drove
/// on the tick of the event (CA-31, CF02-Q39, CA-32). Computed from the combat
/// event log, because only the log proves what landed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    /// AI-started moves (presses; the chain follow-up is not one), per hero.
    pub assist_starts: [u32; 2],
    /// Of those, the ones that answered a tell aimed at the other hero.
    pub interrupt_starts: [u32; 2],
    /// Hits that landed (damage above 0) from a companion-driven hero.
    pub ai_hits: u32,
    /// Hits that landed from a human-driven hero.
    pub human_hits: u32,
    /// Hits the kill policy turned into a glance (damage 0 of a damaging move).
    pub glances: u32,
    /// Defeats whose killing hit came from a companion-driven hero. Always 0
    /// under `ca19_kill_policy = never`.
    pub ai_kills: u32,
    /// Defeats credited to a companion-driven hero without its own killing hit
    /// (contact, wall slam, shore, hazard; CA-20, not suppressed).
    pub ai_indirect_kills: u32,
    /// Defeats credited to a human-driven hero.
    pub human_kills: u32,
    /// Enemy tells broken by a hit from a companion-driven hero.
    pub ai_interrupts: u32,
}

impl Tally {
    /// The AI's share of the landed hits, in percent (0 with no hit).
    pub fn ai_hit_share_percent(&self) -> u32 {
        (self.ai_hits * 100)
            .checked_div(self.ai_hits + self.human_hits)
            .unwrap_or(0)
    }
}

/// Run `fixture`'s script for its whole length and tally the companion's
/// numbers. Deterministic; reads only the combat log and the runtime counters.
pub fn tally(sim: &mut crate::party_scene::PartyFixtureSim, ticks: u64) -> Tally {
    use crate::combat::{CombatEvent, MoveId};
    let mut t = Tally::default();
    for _ in 0..ticks {
        sim.step_scripted();
        let driven = sim.combat.companion_driven();
        let events: Vec<CombatEvent> = sim.combat.take_new_events().to_vec();
        let is_driven = |hero: u32| driven.get(hero as usize).copied().unwrap_or(false);
        for event in &events {
            match *event {
                CombatEvent::Hit {
                    move_id, damage, ..
                } => {
                    let hero = move_id.hero();
                    if damage > 0 {
                        if is_driven(hero) {
                            t.ai_hits += 1;
                        } else {
                            t.human_hits += 1;
                        }
                    } else if is_driven(hero)
                        && !matches!(move_id, MoveId::Swat | MoveId::GreyboxStrike)
                    {
                        t.glances += 1;
                    }
                }
                CombatEvent::EnemyDefeated {
                    tick,
                    target,
                    instigator: Some(hero),
                    ..
                } => {
                    let direct = events.iter().any(|e| {
                        matches!(e, CombatEvent::Hit { tick: x, target: y, health_left: 0, move_id, .. }
                            if *x == tick && *y == target && move_id.hero() == hero)
                    });
                    match (is_driven(hero), direct) {
                        (true, true) => t.ai_kills += 1,
                        (true, false) => t.ai_indirect_kills += 1,
                        (false, _) => t.human_kills += 1,
                    }
                }
                CombatEvent::GoblinInterrupted { tick, .. }
                | CombatEvent::LlamaInterrupted { tick, .. } => {
                    // A tell broken on a tick a companion-driven hero hit.
                    let by_driven = events.iter().any(|e| {
                        matches!(e, CombatEvent::Hit { tick: x, move_id, .. }
                            if *x == tick && is_driven(move_id.hero()))
                    });
                    t.ai_interrupts += u32::from(by_driven);
                }
                _ => {}
            }
        }
    }
    let stats = sim.combat.companion_stats();
    t.assist_starts = stats.assist_starts;
    t.interrupt_starts = stats.interrupt_starts;
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{CombatEvent, MoveId};
    use crate::companion::events::CompanionEvent;
    use crate::donut_mana::ManaEvent;
    use crate::env_kit::EnvKit;
    use crate::goblin::GoblinMode;
    use crate::party::HeroId;
    use crate::party_scene::PartyFixtureSim;
    use std::path::PathBuf;

    fn kit() -> EnvKit {
        EnvKit::load(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            "assets/levels/floor01/corridor_abc_sample.json",
        )
        .unwrap()
    }

    fn arena_kit() -> EnvKit {
        arena::load_arena(&PathBuf::from(env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    fn sim_of(fixture: &'static PartyFixture) -> PartyFixtureSim {
        let kit = if fixture.base.room == crate::combat_scene::FixtureRoom::Arena {
            arena_kit()
        } else {
            kit()
        };
        PartyFixtureSim::new(&kit, 360, fixture).unwrap()
    }

    /// The same scenario under another tuning (leaked: fixtures are static).
    fn with_tuning(fixture: &PartyFixture, tuning: PartyTuning) -> &'static PartyFixture {
        Box::leak(Box::new(PartyFixture {
            base: fixture.base,
            two_player: fixture.two_player,
            tuning,
            script: fixture.script,
            pilot: fixture.pilot,
            donut: fixture.donut,
        }))
    }

    /// The restrained rules these Q14..Q38 scenarios were written for (never a
    /// kill, one move per 60 ticks, no offense): the `companion_v0` Donut
    /// has exactly the old `cf02_assist` Donut numbers, and AI Carl is the
    /// human's hero in these solo-as-Carl runs, so the replay is the old
    /// one. CF02-DUO changed `cf02_assist` itself; its tests are the `duo_*`
    /// ones.
    fn legacy(fixture: &PartyFixture) -> &'static PartyFixture {
        with_tuning(fixture, V0)
    }

    /// Run the whole fixture; the events and the companion events.
    fn run_events(
        fixture: &'static PartyFixture,
    ) -> (PartyFixtureSim, Vec<CombatEvent>, Vec<CompanionEvent>) {
        let mut sim = sim_of(fixture);
        let mut events = Vec::new();
        let mut companion = Vec::new();
        for _ in 0..fixture.base.ticks {
            sim.step_scripted();
            events.extend(sim.combat.take_new_events().iter().copied());
            companion.extend(sim.combat.take_companion_events());
        }
        (sim, events, companion)
    }

    fn tally_of(fixture: &'static PartyFixture) -> Tally {
        let mut sim = sim_of(fixture);
        tally(&mut sim, fixture.base.ticks)
    }

    fn moves_started(events: &[CombatEvent], id: MoveId) -> Vec<u64> {
        events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::MoveStarted { tick, move_id, .. } if move_id == id => Some(tick),
                _ => None,
            })
            .collect()
    }

    /// `DUMP=name cargo test dump_fixture -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn dump_fixture() {
        let name = std::env::var("DUMP").unwrap();
        let from: u64 = std::env::var("FROM")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let to: u64 = std::env::var("TO")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(400);
        let fixture = crate::cf02_fixtures::find_party_fixture(&name).unwrap();
        let mut sim = sim_of(fixture);
        for _ in 0..to {
            sim.step_scripted();
            let t = sim.scene.world.tick;
            for e in sim.combat.take_companion_events() {
                if t >= from {
                    println!("  event {e:?}");
                }
            }
            if t < from {
                continue;
            }
            let c = &sim.combat;
            let gob: Vec<String> = c
                .enemies
                .iter()
                .map(|g| format!("{:.0}/{:?}/{}/hp{}", g.x, g.mode, g.attack_tick, g.health))
                .collect();
            let llamas: Vec<String> = c
                .llamas
                .iter()
                .map(|g| format!("{:.0}/{:?}/{}/hp{}", g.x, g.mode, g.attack_tick, g.health))
                .collect();
            println!(
                "t={t} carl={:.1},{:.1} donut={:.1},{:.1} dmove={:?} gob={gob:?} llama={llamas:?} driven={:?}",
                sim.scene.world.body(HeroId::Carl).x,
                sim.scene.world.body(HeroId::Carl).y,
                sim.scene.world.body(HeroId::Donut).x,
                sim.scene.world.body(HeroId::Donut).y,
                sim.combat.donut_move_view().map(|v| (v.mv, v.tick)),
                c.companion_driven(),
            );
        }
    }

    /// The committed final `PARTY_STATE` hash of every fixture here (the CI
    /// replays in `docs/evidence/companion-ai` pin the same values).
    const GOLDENS: [(&str, u64); 18] = [
        ("cf02_assist_swat_interrupt", 0xccbc3098bd4935e9),
        ("cf02_assist_swat_trailing_gap", 0x04841f57c5c450c5),
        ("cf02_assist_pounce_staggered", 0x13fe359d33b2ee65),
        ("cf02_assist_hp_drops_midmove", 0xeeef90433185c908),
        ("cf02_assist_cadence", 0xb6639df01f0b99a9),
        ("cf02_assist_magic_locked", 0x3fa0c4ae6aaecf92),
        ("cf02_assist_missile_reserve", 0x69ededc0d38826d4),
        ("cf02_assist_camera_parity", 0x994a61b9dcbec360),
        ("cf02_assist_donut_defeats_goblin", 0xb7c51b95ae363569),
        ("cf02_assist_donut_missiles_goblin", 0x39cf5ae3b07e4392),
        ("cf02_assist_donut_guards_carls_back", 0x31dba7f55a0bc61a),
        ("cf02_assist_donut_flanks_two_goblins", 0xe567e5c22d30484b),
        ("companion_carl_interrupts_for_donut", 0x932624152d49df54),
        ("companion_donut_supports_carl", 0x76a8b25087a909a9),
        ("companion_duo_no_ai", 0x7bbaea9a82c73afe),
        ("companion_swap_mid_move", 0xec01b32eeb394f8f),
        ("companion_arena_nav_perch_donut", 0x92d45ee04a2c1aea),
        ("companion_arena_nav_perch_carl", 0xd16f2d3826906853),
    ];

    #[test]
    #[ignore]
    fn print_goldens() {
        for f in COMPANION_FIXTURES {
            let mut sim = sim_of(f);
            sim.run_scripted(f.base.ticks);
            println!("(\"{}\", {:#018x}),", f.name(), sim.state_hash());
        }
    }

    #[test]
    fn every_fixture_replays_identically_twice_and_matches_its_golden() {
        assert_eq!(GOLDENS.len(), COMPANION_FIXTURES.len());
        for (f, (name, golden)) in COMPANION_FIXTURES.iter().zip(GOLDENS) {
            assert_eq!(f.name(), name);
            let hash = |_: ()| {
                let mut sim = sim_of(f);
                sim.run_scripted(f.base.ticks);
                (sim.state_hash(), sim.state_line())
            };
            let (a, line) = hash(());
            assert_eq!(a, hash(()).0, "{name} is not deterministic");
            assert_eq!(a, golden, "{name}: {line}");
        }
    }

    #[test]
    fn the_names_carry_their_registry_prefix() {
        for f in COMPANION_FIXTURES {
            let n = f.name();
            assert!(
                n.starts_with("companion_") || n.starts_with("cf02_assist_"),
                "{n}"
            );
        }
        assert_eq!(
            COMPANION_FIXTURES
                .iter()
                .filter(|f| f.name().starts_with("cf02_assist_"))
                .count(),
            12,
            "the eight assistant fixtures moved from CF02-C2 plus the four CF02-DUO ones"
        );
    }

    /// CF02-Q14: the AI Donut swats a tell aimed at the controlled Carl, Carl stays
    /// passive (Q13), nobody dies.
    #[test]
    fn q14_the_assist_swat_breaks_the_tell_and_ai_carl_is_passive() {
        let (sim, events, _) = run_events(legacy(&CF02_ASSIST_SWAT_INTERRUPT));
        let swats = moves_started(&events, MoveId::Swat);
        assert!(!swats.is_empty(), "{events:?}");
        let tell = events
            .iter()
            .find_map(|e| match e {
                CombatEvent::GoblinTell { tick, .. } => Some(*tick),
                _ => None,
            })
            .unwrap();
        assert!(swats[0] >= tell + 4, "not before tell tick 4 (Q14, Q39)");
        assert!(events.iter().any(
            |e| matches!(e, CombatEvent::GoblinInterrupted { tick, .. } if *tick == swats[0] + 4)
        ));
        assert!(moves_started(&events, MoveId::Jab).is_empty());
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
        );
        assert_eq!(sim.combat.companion_driven(), [false, true]);
    }

    /// CF02-Q14: a swat starts only when it lands inside the tell, and an AI swat that
    /// started always interrupted something (it never starts late).
    #[test]
    fn q14_a_swat_never_starts_too_late_to_land() {
        let (_, events, _) = run_events(&CF02_ASSIST_SWAT_TRAILING_GAP);
        let swats = moves_started(&events, MoveId::Swat);
        assert!(!swats.is_empty());
        for start in swats {
            let tell = events
                .iter()
                .filter_map(|e| match e {
                    CombatEvent::GoblinTell { tick, .. } if *tick <= start => Some(*tick),
                    _ => None,
                })
                .max()
                .unwrap();
            assert!(
                start - tell + 4 < 24,
                "swat {start} after tell {tell} lands late"
            );
            assert!(
                events.iter().any(|e| matches!(e, CombatEvent::GoblinInterrupted { tick, .. } if *tick == start + 4)),
                "the swat at {start} did not interrupt"
            );
        }
    }

    /// CF02-Q15 and Q38: the AI pounces a staggered goblin without a killing blow and
    /// Carl takes the kill.
    #[test]
    fn q15_the_assist_pounce_leaves_the_kill_to_carl() {
        let t = tally_of(legacy(&CF02_ASSIST_POUNCE_STAGGERED));
        let (_, events, _) = run_events(legacy(&CF02_ASSIST_POUNCE_STAGGERED));
        assert_eq!(moves_started(&events, MoveId::Pounce).len(), 1);
        assert!(events.iter().any(|e| matches!(
            e,
            CombatEvent::Hit {
                move_id: MoveId::Pounce,
                damage: 1,
                health_left: 2,
                ..
            }
        )));
        assert_eq!((t.ai_kills, t.ai_indirect_kills, t.human_kills), (0, 0, 1));
        assert!(events.iter().any(|e| matches!(
            e,
            CombatEvent::EnemyDefeated {
                instigator: Some(0),
                ..
            }
        )));
    }

    /// CF02-Q38 (AI part): the lethality re-check runs on the hit tick, on the HP the goblin has by then.
    #[test]
    fn q38_a_pounce_whose_target_drops_to_one_hp_mid_move_glances() {
        let (_, events, _) = run_events(legacy(&CF02_ASSIST_HP_DROPS_MIDMOVE));
        let glance = events.iter().find_map(|e| match *e {
            CombatEvent::Hit {
                move_id: MoveId::Pounce,
                damage,
                health_left,
                ..
            } => Some((damage, health_left)),
            _ => None,
        });
        assert_eq!(
            glance,
            Some((0, 1)),
            "damage 0, the goblin keeps its last HP"
        );
        let t = tally_of(legacy(&CF02_ASSIST_HP_DROPS_MIDMOVE));
        assert_eq!((t.glances, t.ai_kills, t.human_kills), (1, 0, 1));
    }

    /// CF02-DUO-A07 (PR #165 review B-1): the offensive swat is gone, so the AI
    /// Donut's swats are only interrupts of a tell, each one lands (no glance:
    /// a glance would be a 0 damage swat at a goblin in its swing or at a
    /// llama) and sets no guard on a goblin that is not telling. Her interrupt
    /// swat still breaks a tell aimed at Carl in the two reference fights
    /// (1 each since the A15 flank: she now meets the goblins with a pounce
    /// first, 2 each before it), none of them blocked by a self-set guard.
    #[test]
    fn duo_a07_no_offensive_swat_and_the_interrupt_swat_still_breaks_tells() {
        for (f, interrupts) in [(&CF02_ASSIST_CADENCE, 1), (&CF02_ASSIST_CAMERA_PARITY, 1)] {
            let (_, events, _) = run_events(f);
            assert!(
                !events
                    .iter()
                    .any(|e| matches!(e, CombatEvent::DonutGlance { .. })),
                "{}: an AI swat glanced",
                f.name()
            );
            let swat_hit_ticks: Vec<u64> = events
                .iter()
                .filter_map(|e| match *e {
                    CombatEvent::Hit {
                        tick,
                        move_id: MoveId::Swat,
                        ..
                    } => Some(tick),
                    _ => None,
                })
                .collect();
            let swat_interrupts = events
                .iter()
                .filter(|e| {
                    matches!(e, CombatEvent::GoblinInterrupted { tick, .. }
                        if swat_hit_ticks.contains(tick))
                })
                .count();
            assert_eq!(swat_interrupts, interrupts, "{}: {events:?}", f.name());
            // Both goblins tell in the run, so the one interrupt swat breaks a
            // tell while another goblin is alive (a guard against the fixture
            // drifting back to a run where she kills goblin 0 before it tells).
            let both = f.name() == "cf02_assist_cadence";
            for goblin in (0..f.base.enemies.len() as u32).filter(|_| both) {
                assert!(
                    events.iter().any(
                        |e| matches!(e, CombatEvent::GoblinTell { goblin: g, .. } if *g == goblin)
                    ),
                    "{}: goblin {goblin} never tells",
                    f.name()
                );
            }
            // Every swat hit interrupts a tell: no swat landed on a goblin that
            // was not telling (that would set the guard against her own interrupt).
            for tick in &swat_hit_ticks {
                assert!(
                    events.iter().any(|e| matches!(
                        e,
                        CombatEvent::GoblinInterrupted { tick: t, .. } if t == tick
                    )),
                    "{}: a swat at {tick} broke no tell",
                    f.name()
                );
            }
        }
    }

    /// CF02-DUO-A08 (PR #165 review B-2, spec C-11): in the staggered-pounce
    /// room Carl's second jab leaves the goblin at 1 HP and the AI Donut does
    /// not take the last hit: the kill is Carl's.
    #[test]
    fn c11_the_ai_leaves_the_last_hit_to_carl_inside_his_combo() {
        let t = tally_of(&CF02_ASSIST_POUNCE_STAGGERED);
        let (_, events, _) = run_events(&CF02_ASSIST_POUNCE_STAGGERED);
        assert_eq!((t.ai_kills, t.human_kills), (0, 1), "{t:?}");
        assert!(
            !events.iter().any(|e| matches!(
                e,
                CombatEvent::EnemyDefeated {
                    instigator: Some(1),
                    ..
                }
            )),
            "{events:?}"
        );
        // No Donut hit lands after Carl's hit that left the goblin at 1 HP.
        let carl_left_one = events.iter().find_map(|e| match *e {
            CombatEvent::Hit {
                tick,
                move_id: MoveId::Jab,
                health_left: 1,
                ..
            } => Some(tick),
            _ => None,
        });
        let at = carl_left_one.expect("Carl's jab leaves the goblin at 1 HP");
        assert!(
            !events.iter().any(|e| matches!(
                e,
                CombatEvent::Hit { tick, move_id: MoveId::Pounce, .. } if *tick >= at
            )),
            "the AI hit the goblin at 1 HP"
        );
    }

    /// CF02-Q16: at most one AI-started move per 60 ticks, none while one runs.
    #[test]
    fn q16_one_ai_started_move_per_sixty_ticks() {
        let (_, events, companion) = run_events(legacy(&CF02_ASSIST_CADENCE));
        let presses: Vec<u64> = companion
            .iter()
            .filter_map(|e| match e {
                CompanionEvent::Press {
                    tick, chain: false, ..
                } => Some(*tick),
                _ => None,
            })
            .collect();
        assert!(presses.len() >= 2, "{presses:?}");
        for pair in presses.windows(2) {
            assert!(pair[1] - pair[0] >= 60, "{presses:?}");
        }
        // The first tell's swat leaves the second goblin's tell, 12 ticks later,
        // inside the cadence: it is not interrupted.
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CombatEvent::GoblinSwing { goblin: 1, .. }))
        );
    }

    /// CF02-DUO-A01, A04, A06: with no human input at all AI Donut engages the
    /// goblin, lands hits and her own hit defeats it.
    #[test]
    fn duo_ai_donut_defeats_a_goblin_on_her_own() {
        let f = &CF02_ASSIST_DONUT_DEFEATS_GOBLIN;
        let t = tally_of(f);
        let (_, events, _) = run_events(f);
        assert_eq!(t.human_hits, 0, "{t:?}");
        assert!(t.ai_hits >= 2, "{t:?}");
        assert_eq!((t.ai_kills, t.glances, t.human_kills), (1, 0, 0), "{t:?}");
        assert_eq!(t.ai_hit_share_percent(), 100);
        assert!(events.iter().any(|e| matches!(
            e,
            CombatEvent::Hit {
                move_id: MoveId::Pounce,
                health_left: 0,
                ..
            }
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            CombatEvent::EnemyDefeated {
                instigator: Some(1),
                ..
            }
        )));
        assert!(!moves_started(&events, MoveId::Pounce).is_empty());
    }

    /// CF02-DUO-A04, A05: the offensive missile is a real cast under the mana
    /// rules (the reserve of 2 is kept) and it defeats the goblin.
    #[test]
    fn duo_ai_donut_missiles_a_goblin_within_the_mana_rules() {
        let f = &CF02_ASSIST_DONUT_MISSILES_GOBLIN;
        let t = tally_of(f);
        let (_, events, _) = run_events(f);
        let casts = moves_started(&events, MoveId::MagicMissile);
        assert!(casts.len() >= 2, "{casts:?}");
        for pair in casts.windows(2) {
            assert!(
                pair[1] - pair[0] >= 20,
                "the action cadence holds: {casts:?}"
            );
        }
        let mut spent = 0;
        for e in &events {
            if let CombatEvent::Mana(ManaEvent::ManaSpent { left, .. }) = e {
                spent += 1;
                assert!(*left >= 2, "the reserve is kept: {e:?}");
            }
        }
        assert_eq!(spent, casts.len(), "every cast paid its charge");
        assert_eq!(
            (t.ai_kills, t.human_hits, t.human_kills),
            (1, 0, 0),
            "{t:?}"
        );
        assert!(events.iter().any(|e| matches!(
            e,
            CombatEvent::EnemyDefeated {
                instigator: Some(1),
                ..
            }
        )));
    }

    /// CF02-Q17: before the biscuit the AI never casts and spends nothing.
    #[test]
    fn q17_no_magic_before_the_biscuit() {
        let (_, events, _) = run_events(&CF02_ASSIST_MAGIC_LOCKED);
        assert!(moves_started(&events, MoveId::MagicMissile).is_empty());
        assert!(!events.iter().any(|e| matches!(e, CombatEvent::Mana(_))));
        assert_eq!(tally_of(&CF02_ASSIST_MAGIC_LOCKED).assist_starts, [0, 0]);
    }

    /// CF02-Q17 and Q10-A09: the AI casts only with 2 charges left after the cast.
    #[test]
    fn q17_the_ai_keeps_the_mana_reserve() {
        let (_, events, _) = run_events(&CF02_ASSIST_MISSILE_RESERVE);
        let casts = moves_started(&events, MoveId::MagicMissile);
        assert!(casts.len() >= 2, "{casts:?}");
        for e in &events {
            if let CombatEvent::Mana(ManaEvent::ManaSpent { left, .. }) = e {
                assert!(*left >= 2, "the reserve is kept: {e:?}");
            }
        }
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CombatEvent::LlamaInterrupted { .. }))
        );
    }

    /// CF02-Q39 (CA-32): the ceiling on the AI's hit share holds on the two
    /// goblin fights, with a smoke floor of two assist starts. The real floor
    /// (`cf02_q39_min_assist_starts`, 3 for `companion_v0`, 2 for `cf02_assist` since CF02-DUO-A10) is measured on the CF02-F
    /// `cf02_arena_direct` run. `cf02_assist` now carries the CF02-DUO-A03
    /// ceiling (50 %) and its measurement is `cf02_arena_direct`; here the
    /// camera-parity fight runs under the restrained rules (`legacy`, same
    /// Donut numbers as the old `cf02_assist`, ceiling 25 %) next to the v0 one.
    #[test]
    fn q39_the_ai_hit_share_ceiling_holds_on_the_two_goblin_fights() {
        let profiles = crate::companion::profile::ProfileFile::pinned().unwrap();
        for (f, set) in [
            (
                legacy(&CF02_ASSIST_CAMERA_PARITY),
                ProfileSetId::CompanionV0,
            ),
            (&COMPANION_DONUT_SUPPORTS_CARL, ProfileSetId::CompanionV0),
        ] {
            let t = tally_of(f);
            let p = profiles.set(set);
            assert!(t.assist_starts[1] >= 2, "{}: {t:?}", f.name());
            assert!(
                t.ai_hit_share_percent() <= u32::from(p.cf02_q39_max_hit_share_percent),
                "{}: {t:?}",
                f.name()
            );
            assert_eq!(t.ai_kills, 0);
        }
    }

    const REG_SCRIPT: &[crate::party_scene::SlotHold] =
        &[primary(10, 11, 0), primary(30, 31, 0), swap(31, 32)];
    const REG_CAST_SCRIPT: &[crate::party_scene::SlotHold] = &[
        swap(3, 4),
        primary(10, 11, 0),
        primary(40, 41, 0),
        swap(44, 45),
    ];

    /// A one-goblin fixture for the swap regressions (CA-20, review B-1).
    const REG_BASE: Fixture = base(
        "companion_regression_swap",
        "test only",
        0xC0A2_00FF,
        &[dummy(430)],
        90,
    );

    /// CA-20 / review B-1: a move the human started is never AI-started, even if
    /// the player swaps away mid-move. The human's lethal jab still kills.
    #[test]
    fn a_human_jab_then_a_swap_mid_move_still_kills() {
        // `V0` only: under `cf02_assist` (CF02-DUO) the AI Donut pounces the dummy
        // before the human jab, so the scenario would no longer be the human's.
        let tuning = V0;
        {
            let f: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
                base: &REG_BASE,
                two_player: false,
                tuning,
                // Jab hits a 2 HP dummy twice: the second hit (tick 33) is lethal
                // and the swap lands on tick 31, in the jab's startup.
                script: REG_SCRIPT,
                pilot: Pilot::None,
                donut: DonutSetup::NONE,
            }));
            let (_, events, _) = run_events(f);
            let lethal = events.iter().find_map(|e| match *e {
                CombatEvent::Hit {
                    tick: 33,
                    move_id: MoveId::Jab,
                    damage,
                    health_left,
                    ..
                } => Some((damage, health_left)),
                _ => None,
            });
            assert_eq!(lethal, Some((1, 0)), "{events:?}");
            assert!(events.iter().any(|e| matches!(
                e,
                CombatEvent::EnemyDefeated {
                    instigator: Some(0),
                    ..
                }
            )));
        }
    }

    /// The same for Donut's cast: a bolt the human cast, then a swap mid-flight,
    /// still kills and is credited to Donut.
    #[test]
    fn a_human_cast_then_a_swap_mid_flight_still_kills() {
        const BASE: Fixture = base(
            "companion_regression_swap_cast",
            "test only",
            0xC0A2_00FE,
            &[dummy(560)],
            100,
        );
        let f: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            base: &BASE,
            two_player: false,
            tuning: ASSIST,
            // Donut (human after the swap on tick 3) casts twice; the second
            // bolt is lethal and the player swaps back while it is in the air.
            script: REG_CAST_SCRIPT,
            pilot: Pilot::None,
            donut: biscuit(8),
        }));
        let (_, events, _) = run_events(f);
        let bolts: Vec<(u8, u8)> = events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Hit {
                    move_id: MoveId::MagicMissile,
                    damage,
                    health_left,
                    ..
                } => Some((damage, health_left)),
                _ => None,
            })
            .collect();
        assert_eq!(bolts, vec![(1, 1), (1, 0)], "{events:?}");
        assert!(events.iter().any(|e| matches!(
            e,
            CombatEvent::EnemyDefeated {
                instigator: Some(1),
                ..
            }
        )));
    }

    /// The glance applies only to the director's own press: the marks are empty
    /// while the human plays, and the passive Carl of `cf02_assist` never marks.
    #[test]
    fn only_the_directors_press_marks_a_move_as_ai_started() {
        let mut sim = sim_of(&CF02_ASSIST_POUNCE_STAGGERED);
        sim.run_scripted(CF02_ASSIST_POUNCE_STAGGERED.base.ticks);
        // Carl's human jabs (instances 1..3) are never marked; Donut's pounce is.
        for instance in 1..=3 {
            assert!(
                !sim.combat.companion_marked_carl(instance),
                "jab {instance}"
            );
        }
        assert!(sim.combat.companion_marked_donut_move(1), "the AI pounce");
    }

    /// 01A both roles: AI Carl jabs the tell aimed at the controlled Donut, has no other
    /// move (no grab, explosive, Down move or air move), and never lands a kill.
    #[test]
    fn ca37_ai_carl_only_jabs_and_never_kills() {
        let (_, events, companion) = run_events(&COMPANION_CARL_INTERRUPTS_FOR_DONUT);
        for e in &events {
            if let CombatEvent::MoveStarted { move_id, .. } = e {
                assert!(matches!(move_id, MoveId::Jab | MoveId::Haymaker), "{e:?}");
            }
        }
        assert!(!moves_started(&events, MoveId::Jab).is_empty());
        let t = tally_of(&COMPANION_CARL_INTERRUPTS_FOR_DONUT);
        assert_eq!((t.ai_kills, t.ai_indirect_kills), (0, 0));
        assert!(
            t.glances >= 1,
            "{t:?}: the lethal interrupt glances (CA-20)"
        );
        assert!(companion.iter().all(|e| match e {
            CompanionEvent::Press { hero, .. } => *hero == HeroId::Carl,
            CompanionEvent::Decision { .. }
            | CompanionEvent::Nav { .. }
            | CompanionEvent::GoalChanged { .. }
            | CompanionEvent::HazardFeed { .. }
            | CompanionEvent::Regroup { .. }
            | CompanionEvent::ReserveReleased { .. } => true,
        }));
    }

    /// A swap never cancels a move (the AI finishes it) and the swapped-away hero
    /// becomes the companion.
    #[test]
    fn a_swap_mid_move_lets_the_ai_move_finish_and_hands_the_role_over() {
        let mut sim = sim_of(&COMPANION_SWAP_MID_MOVE);
        let mut swat_seen = false;
        for _ in 0..120 {
            sim.step_scripted();
            let tick = sim.scene.world.tick;
            if tick == 105 {
                assert_eq!(sim.combat.companion_driven(), [false, true]);
            }
            if tick == 106 {
                assert_eq!(sim.combat.companion_driven(), [true, false]);
                assert!(
                    sim.combat.donut_move_view().is_some(),
                    "the swat survives the swap"
                );
                swat_seen = true;
            }
        }
        assert!(swat_seen);
    }

    /// CA-22: two players never run the companion, and the replay equals the bot
    /// tuning's byte for byte (the hash is mixed only while a hero is driven).
    #[test]
    fn two_players_never_run_the_companion() {
        let (sim, events, companion) = run_events(&COMPANION_DUO_NO_AI);
        assert!(companion.is_empty());
        assert_eq!(sim.combat.companion_driven(), [false, false]);
        let bot = with_tuning(&COMPANION_DUO_NO_AI, PROVISIONAL_TUNING);
        let (bot_sim, bot_events, _) = run_events(bot);
        assert_eq!(events, bot_events);
        assert_eq!(sim.state_hash(), bot_sim.state_hash());
    }

    /// CA-22 rollback: the same scenarios with the follow-and-jump bot never start
    /// an AI move, run no director and tally zero.
    #[test]
    fn the_rollback_bot_never_attacks_and_hashes_nothing_of_the_companion() {
        for f in COMPANION_FIXTURES {
            let bot = with_tuning(f, PROVISIONAL_TUNING);
            let (sim, events, companion) = run_events(bot);
            assert!(companion.is_empty(), "{}", f.name());
            assert_eq!(sim.combat.companion_driven(), [false, false]);
            if !f.two_player {
                // The scripted hero may still act; the AI hero never does.
                let ai = sim.party.controlled_hero().other();
                let ai_hero = ai.index() as u32;
                assert!(
                    !events.iter().any(|e| matches!(e, CombatEvent::MoveStarted { move_id, .. } if move_id.hero() == ai_hero)),
                    "{}",
                    f.name()
                );
            }
            let mut again = sim_of(bot);
            again.run_scripted(bot.base.ticks);
            assert_eq!(again.state_hash(), sim.state_hash());
        }
    }

    /// The companion changes the replay (it is not a no-op) and the table of
    /// numbers the PR reports: bot against companion on the same fixtures.
    #[test]
    fn metrics_bot_versus_companion() {
        println!(
            "fixture | mode | assist starts C/D | AI hits | human hits | glances | AI kills | AI indirect | human kills | AI interrupts"
        );
        for f in COMPANION_FIXTURES {
            let companion = tally_of(f);
            let bot = tally_of(with_tuning(f, PROVISIONAL_TUNING));
            for (mode, t) in [("bot", bot), ("companion", companion)] {
                println!(
                    "{} | {} | {}/{} | {} | {} | {} | {} | {} | {} | {}",
                    f.name(),
                    mode,
                    t.assist_starts[0],
                    t.assist_starts[1],
                    t.ai_hits,
                    t.human_hits,
                    t.glances,
                    t.ai_kills,
                    t.ai_indirect_kills,
                    t.human_kills,
                    t.ai_interrupts
                );
            }
            assert_eq!(bot.assist_starts, [0, 0], "{}", f.name());
            assert_eq!(bot.ai_hits + bot.glances, 0, "{}", f.name());
            // CA-19 holds for a set whose policy is `never`; `cf02_assist` is
            // `factual` since CF02-DUO-A01 (its kills are the point).
            let never = match f.tuning.uncontrolled_hero {
                UncontrolledHero::Companion(id) => {
                    crate::companion::profile::ProfileFile::pinned()
                        .unwrap()
                        .set(id)
                        .ca19_kill_policy
                        == crate::companion::profile::KillPolicy::Never
                }
                _ => true,
            };
            if !f.two_player && never {
                assert_eq!(companion.ai_kills, 0, "{}: never kills (CA-19)", f.name());
            }
        }
    }

    /// CF02-Q38 hold, through the AI: a lying goblin 30..80 px ahead is pounced and
    /// the pounce (non-lethal) holds its down timer once.
    #[test]
    fn q15_and_q38_the_assist_pounce_on_a_lying_goblin_holds_it() {
        let mut sim = sim_of(&CF02_ASSIST_POUNCE_STAGGERED);
        let donut_x = sim.scene.world.body(HeroId::Donut).x;
        {
            let g = &mut sim.combat.enemies[0];
            g.x = donut_x + 45.0;
            g.mode = GoblinMode::Downed;
            g.down_ticks = 0;
            g.facing = -1;
        }
        let mut held = false;
        for _ in 0..40 {
            sim.step_slots([crate::input::RawInput::NONE; 2]);
            held |= sim.combat.enemies[0].hold_used;
        }
        assert!(held, "the pounce held the lying goblin");
        assert!(sim.combat.enemies[0].health >= 1, "and never killed it");
    }

    // ------------------------------------------------------ 01B-wire (nav) --

    use crate::companion::events::RegroupOutcome;
    use crate::companion::nav::EdgeKind;
    use crate::companion::navfollow::NavNote;
    use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH};

    /// The foot y of the hero `id` in a sim.
    fn foot(sim: &PartyFixtureSim, id: HeroId) -> f32 {
        sim.scene.world.body(id).y - HERO_HALF_HEIGHT
    }

    /// 01B acceptance: a companion-driven hero reaches the leader's tier through
    /// the nav graph: one committed jump transit onto the 80 px perch slab, one
    /// arrival, and it ends standing on the slab (feet 80 px above the floor).
    fn assert_climbs(fixture: &'static PartyFixture, companion: HeroId) {
        let floor = foot(&sim_of(fixture), companion);
        let (sim, _, events) = run_events(fixture);
        let x = sim.scene.world.body(companion).x;
        assert!(
            (foot(&sim, companion) - floor - 80.0).abs() < 0.01 && (544.0..=672.0).contains(&x),
            "{}: {companion:?} stands at x {x}, feet {}",
            fixture.name(),
            foot(&sim, companion) - floor
        );
        let nav: Vec<_> = events
            .iter()
            .filter_map(|e| match *e {
                CompanionEvent::Nav { hero, note, .. } if hero == companion => Some(note),
                _ => None,
            })
            .collect();
        assert!(
            matches!(
                nav.first(),
                Some(NavNote::Begin {
                    kind: EdgeKind::Jump,
                    ..
                })
            ),
            "{nav:?}"
        );
        assert!(
            matches!(nav.last(), Some(NavNote::Arrived { .. })),
            "{nav:?}"
        );
        let stats = sim.combat.companion_stats();
        assert_eq!(
            (
                stats.nav_transits[companion.index()],
                stats.nav_arrivals[companion.index()]
            ),
            (1, 1)
        );
        // The companion never presses anything but movement and jump here.
        assert_eq!(stats.assist_starts, [0, 0]);
    }

    #[test]
    fn nav_ai_donut_reaches_the_perch_slab() {
        assert_climbs(&COMPANION_ARENA_NAV_PERCH_DONUT, HeroId::Donut);
    }

    #[test]
    fn nav_ai_carl_reaches_the_perch_slab_when_donut_is_controlled() {
        assert_climbs(&COMPANION_ARENA_NAV_PERCH_CARL, HeroId::Carl);
    }

    /// The same two scenarios under the follow-and-jump bot (the rollback, CA-22):
    /// the legacy follower, which the nav never touches. AI Donut under the bot
    /// stays under the slab; the bot happens to climb for Carl (his jump just
    /// reaches), which is logged for comparison.
    #[test]
    fn the_rollback_bot_is_unchanged_by_the_nav() {
        for (f, who) in [
            (&COMPANION_ARENA_NAV_PERCH_DONUT, HeroId::Donut),
            (&COMPANION_ARENA_NAV_PERCH_CARL, HeroId::Carl),
        ] {
            let bot = with_tuning(f, PROVISIONAL_TUNING);
            let (sim, _, events) = run_events(bot);
            assert!(
                events.iter().all(|e| !matches!(
                    e,
                    CompanionEvent::Nav { .. } | CompanionEvent::Regroup { .. }
                )),
                "{}: the bot logs no nav events",
                f.name()
            );
            let floor = foot(&sim_of(bot), who);
            if who == HeroId::Donut {
                // The legacy follower walks under the slab (its underside is 68 px
                // up) and bumps its head there: it never climbs. This is the case
                // the nav graph exists for.
                assert!(foot(&sim, who) - floor < 10.0, "{}", f.name());
            }
            eprintln!(
                "NAV_BASELINE {} bot {who:?}: feet {} above the floor, x {}",
                f.name(),
                foot(&sim, who) - floor,
                sim.scene.world.body(who).x
            );
        }
    }

    /// 01B-wire, the camera-safe regroup in the live scene: the leader runs far
    /// ahead, the AI Donut is left off-screen and, after `regroup_ticks`, is
    /// placed on the floor anchor just outside the view behind Carl (on the
    /// floor, not on the leader's foot line, not inside the camera), once.
    #[test]
    fn nav_regroup_places_an_offscreen_companion_just_outside_the_view() {
        let f = &COMPANION_ARENA_NAV_PERCH_DONUT;
        let mut sim = sim_of(f);
        let floor = foot(&sim, HeroId::Donut);
        sim.scene.world.hero.x = 1000.0;
        sim.scene.world.donut_body.x = 40.0;
        let mut events = Vec::new();
        let mut left_at_regroup = None;
        for _ in 0..260 {
            let before = sim.scene.camera.left();
            sim.step_slots([crate::input::RawInput::NONE; 2]);
            for e in sim.combat.take_companion_events() {
                if let CompanionEvent::Regroup { outcome, .. } = e {
                    left_at_regroup = Some((before, sim.scene.world.body(HeroId::Donut).x));
                    events.push(outcome);
                }
            }
        }
        let placed: Vec<_> = events
            .iter()
            .filter(|o| matches!(o, RegroupOutcome::Placed { .. }))
            .collect();
        assert_eq!(placed.len(), 1, "{events:?}");
        let (left, x) = left_at_regroup.unwrap();
        assert!(
            (x - (left as f32 - HERO_HALF_WIDTH - 2.0)).abs() < 4.0,
            "placed {x} against a view starting at {left}"
        );
        assert_eq!(foot(&sim, HeroId::Donut), floor, "on the floor, at rest");
        assert_eq!(sim.combat.companion_stats().regroups, [0, 1]);
    }

    /// 01B-wire, N-2: a regroup deferred because the leader is in the air is
    /// asked again on the very next tick, so the hero is placed on the first
    /// tick the leader stands, not `regroup_ticks` later.
    #[test]
    fn a_deferred_regroup_is_placed_on_the_first_tick_the_leader_stands() {
        let f = &COMPANION_ARENA_NAV_PERCH_DONUT;
        let mut sim = sim_of(f);
        let carl_floor = foot(&sim, HeroId::Carl);
        sim.scene.world.hero.x = 1000.0;
        sim.scene.world.donut_body.x = 40.0;
        let mut regroups = Vec::new();
        let mut release = None;
        let mut first_stand = None;
        for tick in 0..300 {
            // Carl is held a little above the floor for 130 ticks (past the 120
            // regroup_ticks, while the AI Donut is still off-screen),
            // then falls and lands. DM-A: her 207 px/s run would carry her into
            // the view before Carl lands (she walks in at x about 484 by tick
            // 137), so the setup holds her at x 40 too: the case under test is
            // a regroup that is due while the leader is in the air.
            if tick < 130 {
                let h = &mut sim.scene.world.hero;
                h.y = carl_floor + 24.0;
                h.vy = 0.0;
                h.grounded = false;
                sim.scene.world.donut_body.x = 40.0;
            } else if release.is_none() {
                release = Some(tick);
            }
            let standing_before = sim.scene.world.hero.grounded;
            if release.is_some() && standing_before && first_stand.is_none() {
                first_stand = Some(tick);
            }
            sim.step_slots([crate::input::RawInput::NONE; 2]);
            for e in sim.combat.take_companion_events() {
                if let CompanionEvent::Regroup { outcome, .. } = e {
                    regroups.push((tick, outcome));
                }
            }
        }
        let first_stand = first_stand.expect("Carl landed");
        assert_eq!(regroups.len(), 1, "{regroups:?}");
        assert!(
            matches!(regroups[0].1, RegroupOutcome::Placed { .. }),
            "{regroups:?}"
        );
        assert_eq!(
            regroups[0].0, first_stand,
            "placed on the first tick Carl stands: {regroups:?}"
        );
    }

    // ------------------------------------------------ CF02-DUO-A11, A12 --

    use crate::party_scene::SlotHold;

    /// The `cf02_assist_pounce_staggered` room (a 4 HP dummy at 430, Carl at 400)
    /// with another Carl script (leaked: fixtures are static).
    fn pounce_room(script: &'static [SlotHold]) -> &'static PartyFixture {
        Box::leak(Box::new(PartyFixture {
            script,
            ..CF02_ASSIST_POUNCE_STAGGERED
        }))
    }

    /// `(tick, what)` of every move start, hit and defeat of a run.
    fn timeline(events: &[CombatEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::MoveStarted { tick, move_id, .. } => {
                    Some(format!("{tick} start {move_id:?}"))
                }
                CombatEvent::Hit {
                    tick,
                    move_id,
                    target,
                    damage,
                    health_left,
                    ..
                } => Some(format!(
                    "{tick} hit {move_id:?} g{target} dmg{damage} left{health_left}"
                )),
                CombatEvent::EnemyDefeated {
                    tick, instigator, ..
                } => Some(format!("{tick} defeated by {instigator:?}")),
                _ => None,
            })
            .collect()
    }

    /// `PROBE=walkup|tick50 cargo test probe_timeline -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn probe_timeline() {
        let which = std::env::var("PROBE").unwrap_or_default();
        let f = match which.as_str() {
            "walkup" => pounce_room(WALKUP_SCRIPT),
            "retreat" => pounce_room(RETREAT_SCRIPT),
            "literal" => pounce_room(LITERAL_SCRIPT),
            "tick50" => pounce_room(TICK50_SCRIPT),
            name => crate::cf02_fixtures::find_party_fixture(name).unwrap(),
        };
        let (_, events, _) = run_events(f);
        for line in timeline(&events) {
            println!("{line}");
        }
    }

    /// The first cut of `cf02_assist_pounce_staggered`: Carl's second jab on
    /// tick 50, the tick her pounce starts.
    const TICK50_SCRIPT: &[SlotHold] = &[
        primary(5, 6, 0),
        primary(50, 51, 0),
        run(58, 70, 0, 1.0),
        primary(72, 73, 0),
    ];

    /// The PR #165 round-2 probe (R2-1): the second jab at 46, a walk-up to
    /// 70, a pause, the third jab only at 80 (the review's script ran to 78,
    /// which walks Carl through the goblin and past it).
    const WALKUP_SCRIPT: &[SlotHold] = &[
        primary(5, 6, 0),
        primary(46, 47, 0),
        run(58, 70, 0, 1.0),
        primary(80, 81, 0),
    ];

    /// The PR #170 review B-1 probe: the jab at 46 leaves the goblin at 1 HP
    /// (tick 49), Carl steps back at 64 (facing flips, the goblin is now
    /// behind him) and jabs again at 76.
    const RETREAT_SCRIPT: &[SlotHold] = &[
        primary(5, 6, 0),
        primary(46, 47, 0),
        run(64, 69, 0, -1.0),
        run(70, 73, 0, 1.0),
        primary(76, 77, 0),
    ];

    /// The review's literal walk-through: the same hit at 49, then Carl walks
    /// through the goblin (58 to 78) and presses at 80.
    const LITERAL_SCRIPT: &[SlotHold] = &[
        primary(5, 6, 0),
        primary(46, 47, 0),
        run(58, 78, 0, 1.0),
        primary(80, 81, 0),
    ];

    /// What AI Donut did in a run, read from the combat log (CF02-DUO-A11):
    /// her move starts, landed damaging hits, kills and kill steals. C-11 steal
    /// metric, two counts. `steals` is the first-cut one: a killing hit of hers
    /// within `STEAL_WINDOW` (30) ticks of Carl's own hit on the same enemy; it
    /// is a lower bound, because a pounce lands about 10 to 20 ticks after it
    /// starts and the walk-through case can kill 40 ticks after his hit
    /// (PR #170 review F-2). `start_steals` is the exact A12 contract: a killing
    /// hit of a move she *started* within `cf02_duo_a12_recent_hit_ticks` of
    /// Carl's last hit on that enemy. A kill from a pounce started after the
    /// window is not a steal by design (he is no longer working it).
    #[derive(Debug, Default, PartialEq, Eq)]
    struct Offence {
        starts: Vec<(u64, MoveId)>,
        hits: u32,
        kills: u32,
        steals: u32,
        start_steals: u32,
        carl_hits: u32,
        carl_kills: u32,
    }

    const STEAL_WINDOW: u64 = 30;

    fn offence(events: &[CombatEvent]) -> Offence {
        let a12 = u64::from(
            crate::companion::profile::ProfileFile::pinned()
                .unwrap()
                .set(ProfileSetId::Cf02Assist)
                .cf02_duo
                .cf02_duo_a12_recent_hit_ticks,
        );
        let mut o = Offence::default();
        let carl_hit_ticks: Vec<(u64, u32)> = events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Hit {
                    tick,
                    move_id,
                    target,
                    damage,
                    ..
                } if move_id.hero() == 0 && damage > 0 => Some((tick, target)),
                _ => None,
            })
            .collect();
        o.carl_hits = carl_hit_ticks.len() as u32;
        let start_of = |instance: u32| {
            events.iter().find_map(|e| match *e {
                CombatEvent::MoveStarted {
                    tick,
                    move_id,
                    instance: i,
                } if i == instance && move_id.hero() == 1 => Some(tick),
                _ => None,
            })
        };
        for e in events {
            match *e {
                CombatEvent::MoveStarted { tick, move_id, .. } if move_id.hero() == 1 => {
                    o.starts.push((tick, move_id));
                }
                CombatEvent::Hit {
                    tick,
                    move_id,
                    target,
                    damage,
                    health_left,
                    instance,
                    ..
                } if move_id.hero() == 1 && damage > 0 => {
                    o.hits += 1;
                    if health_left == 0 {
                        o.kills += 1;
                        let carl_hit = |until: u64| {
                            carl_hit_ticks
                                .iter()
                                .any(|&(t, id)| id == target && t <= until && until - t <= a12)
                        };
                        o.steals +=
                            u32::from(carl_hit_ticks.iter().any(|&(t, id)| {
                                id == target && t <= tick && tick - t <= STEAL_WINDOW
                            }));
                        o.start_steals += u32::from(start_of(instance).is_some_and(carl_hit));
                    }
                }
                CombatEvent::EnemyDefeated {
                    instigator: Some(0),
                    ..
                } => o.carl_kills += 1,
                _ => {}
            }
        }
        o
    }

    fn arena_events(name: &str) -> Vec<CombatEvent> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let kit = crate::cf02::arena::load_arena(&root).unwrap();
        let f = crate::cf02_fixtures::find_party_fixture(name).unwrap();
        let mut sim = PartyFixtureSim::new(&kit, 360, f).unwrap();
        sim.run_scripted(f.base.ticks);
        sim.combat.events.to_vec()
    }

    /// Donut's move starts within `from..=to`.
    fn donut_starts_between(o: &Offence, from: u64, to: u64) -> Vec<(u64, MoveId)> {
        o.starts
            .iter()
            .copied()
            .filter(|&(t, _)| (from..=to).contains(&t))
            .collect()
    }

    fn defeats(events: &[CombatEvent]) -> Vec<(u64, u32, Option<u32>)> {
        events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::EnemyDefeated {
                    tick,
                    target,
                    instigator,
                    ..
                } => Some((tick, target, instigator)),
                _ => None,
            })
            .collect()
    }

    /// R2-1 (PR #165) pinned: Carl hits the goblin at 49, walks up and jabs at
    /// 80 (hit 83). A12 keeps it his across the walk-up: she starts nothing
    /// from his hit to the end of the window and his jab is the kill.
    #[test]
    fn c11_a_goblin_carl_hit_stays_his_across_a_walk_up() {
        let (_, events, _) = run_events(pounce_room(WALKUP_SCRIPT));
        let o = offence(&events);
        assert_eq!(donut_starts_between(&o, 49, 80), vec![], "{o:?}");
        assert!(
            defeats(&events).contains(&(83, 0, Some(0))),
            "Carl's jab at 83 is the kill: {:?}",
            timeline(&events)
        );
        assert_eq!((o.kills, o.steals, o.start_steals), (0, 0, 0), "{o:?}");
    }

    /// PR #170 review B-1: A12 is checked before A11, so the goblin Carl hit
    /// stays his when he steps back (facing flips, it is now behind him) or
    /// walks through it. The probe: hit at 49 leaves 1 HP, Carl steps back at
    /// 64 and jabs at 76. Before the fix her pounce started at 67 and killed it
    /// at 78 (steals: 1); now she starts nothing until the window is over.
    #[test]
    fn c11_b1_a_goblin_carl_hit_stays_his_when_he_steps_back_or_walks_through() {
        for (name, script) in [("retreat", RETREAT_SCRIPT), ("literal", LITERAL_SCRIPT)] {
            let (_, events, _) = run_events(pounce_room(script));
            let o = offence(&events);
            assert_eq!(donut_starts_between(&o, 49, 80), vec![], "{name}: {o:?}");
            assert_eq!(
                (o.steals, o.start_steals),
                (0, 0),
                "{name}: {:?}",
                timeline(&events)
            );
        }
        // The retreat: nobody finishes it inside the window.
        let (_, events, _) = run_events(pounce_room(RETREAT_SCRIPT));
        assert!(
            !defeats(&events).iter().any(|d| d.0 <= 80),
            "{:?}",
            timeline(&events)
        );
    }

    /// A14, the documented known limit: a press on the tick her pounce starts
    /// is invisible to the pre-tick view, so when Carl's second jab starts on
    /// the very tick she starts hers (the first cut of
    /// `cf02_assist_pounce_staggered`, press at 50), both land and hers takes
    /// the last HP (tick 61). The first-cut count flags it (`steals` 1); the
    /// start-based count does not, as Carl's earlier hit (8) was 42 ticks
    /// before her start. The fix is not the director's: it cannot read an input
    /// that has not happened.
    #[test]
    fn c11_known_limit_a_same_tick_press_still_lets_her_take_the_last_hit() {
        let (_, events, _) = run_events(pounce_room(TICK50_SCRIPT));
        let o = offence(&events);
        let t = timeline(&events);
        assert!(t.contains(&"50 start Jab".to_string()), "{t:?}");
        assert!(t.contains(&"50 start Pounce".to_string()), "{t:?}");
        assert!(defeats(&events).contains(&(61, 0, Some(1))), "{t:?}");
        assert_eq!((o.kills, o.steals), (1, 1), "{o:?}");
    }

    /// A13 and the review's item 4 on `cf02_assist_donut_guards_carls_back`:
    /// Carl works the 4 HP goblin ahead of him while a second one walks up
    /// behind. She turns to the one at his back and defeats it first (g1, tick
    /// 205), later takes the other once Carl's window is over (g0, 262), and
    /// never steals: the exact outcome is pinned.
    #[test]
    fn duo_a13_ai_donut_defeats_the_goblin_that_comes_at_carl_from_behind() {
        let (_, events, _) = run_events(&CF02_ASSIST_DONUT_GUARDS_CARLS_BACK);
        let o = offence(&events);
        let pounces = |ticks: &[u64]| -> Vec<(u64, MoveId)> {
            ticks.iter().map(|&t| (t, MoveId::Pounce)).collect()
        };
        assert_eq!(o.starts, pounces(&[2, 97, 146, 195, 247]), "{o:?}");
        assert_eq!(
            (o.hits, o.kills, o.steals, o.start_steals),
            (5, 2, 0, 0),
            "{o:?}"
        );
        assert_eq!((o.carl_hits, o.carl_kills), (3, 0), "{o:?}");
        let d = defeats(&events);
        assert_eq!(
            d.first().map(|&(_, target, by)| (target, by)),
            Some((1, Some(1))),
            "the goblin from behind is the first defeat: {d:?}"
        );
        assert_eq!(d.len(), 2, "{d:?}");
        assert!(d.iter().all(|&(_, _, by)| by == Some(1)), "{d:?}");
    }

    /// The scripted arena run measures her offence and never a steal: the
    /// floor (at least 3 starts) holds on both counts, and neither the first-cut
    /// nor the start-based steal count sees one. Measured on the head: three
    /// starts (one pounce that missed, two interrupt swats), no damaging hit.
    #[test]
    fn duo_the_arena_run_measures_her_offence_and_no_steal() {
        for name in ["cf02_arena_direct", "cf02_arena_eng"] {
            let events = arena_events(name);
            let o = offence(&events);
            assert!(o.starts.len() >= 3, "{name}: {o:?}");
            assert_eq!((o.steals, o.start_steals), (0, 0), "{name}: {o:?}");
            assert!(o.carl_kills >= 5, "Carl clears the room: {name}: {o:?}");
        }
    }

    /// CF02-DUO-A15: Carl pauses while two goblins walk at him; she gets ahead
    /// of him and lands the first damaging hit on both before they reach him,
    /// and Carl finishes both (jabs at 110 and 136): no kill of hers, no steal.
    #[test]
    fn duo_a15_she_meets_the_approaching_goblins_and_carl_finishes_them() {
        let (_, events, _) = run_events(&CF02_ASSIST_DONUT_FLANKS_TWO_GOBLINS);
        let o = offence(&events);
        // Her first pounce starts before Carl's first press (110).
        assert_eq!(
            o.starts,
            vec![(61, MoveId::Pounce), (110, MoveId::Pounce)],
            "{o:?}"
        );
        assert_eq!(
            (o.hits, o.kills, o.steals, o.start_steals),
            (2, 0, 0, 0),
            "{o:?}"
        );
        assert_eq!((o.carl_hits, o.carl_kills), (2, 2), "{o:?}");
        // For each goblin the first damaging hit is hers.
        let first_hitter = |goblin: u32| {
            events.iter().find_map(|e| match *e {
                CombatEvent::Hit {
                    move_id,
                    target,
                    damage,
                    ..
                } if target == goblin && damage > 0 => Some(move_id.hero()),
                _ => None,
            })
        };
        assert_eq!(first_hitter(0), Some(1), "{:?}", timeline(&events));
        assert_eq!(first_hitter(1), Some(1), "{:?}", timeline(&events));
        assert!(
            defeats(&events).iter().all(|&(_, _, by)| by == Some(0)),
            "Carl takes both finishes: {:?}",
            defeats(&events)
        );
    }

    /// The same room under the other profile set (A15 absent: off in
    /// `companion_v0`, with the offence off too): she does not flank, so she
    /// lands no first hit on either goblin and starts nothing but an interrupt
    /// swat.
    #[test]
    fn duo_a15_is_off_in_companion_v0() {
        let (_, events, _) = run_events(legacy(&CF02_ASSIST_DONUT_FLANKS_TWO_GOBLINS));
        let o = offence(&events);
        assert!(
            o.hits == 0 && o.starts.iter().all(|s| s.1 == MoveId::Swat),
            "{o:?}"
        );
    }

    /// `PROBE=<fixture> cargo test print_offence -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn print_offence() {
        let name = std::env::var("PROBE").unwrap();
        let events = if name.starts_with("cf02_arena") {
            arena_events(&name)
        } else {
            run_events(crate::cf02_fixtures::find_party_fixture(&name).unwrap()).1
        };
        println!("{:?}", offence(&events));
    }
}
