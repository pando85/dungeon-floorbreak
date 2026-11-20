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
use crate::combat::EnemyAi;
use crate::combat_scene::{EnemySpawn, Fixture, LlamaSpawn};
use crate::companion::profile::ProfileSetId;
use crate::donut_fixtures::primary;
use crate::donut_mana::{DonutSetup, StartState};
use crate::party::{PROVISIONAL_TUNING, PartyTuning, UncontrolledHero};
use crate::party_scene::{PartyFixture, base, run, swap};

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
    "Solo as Carl, who stands still: a goblin tells at him and the AI Donut runs up and swats the tell (CF02-Q14)",
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
    &[chase(640)],
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
    "Solo as Carl: he jabs a goblin and the AI Donut pounces the staggered goblin, which never takes her last hit; Carl finishes it (CF02-Q15, Q38)",
    0xC0A1_0003,
    &[dummy(430)],
    160,
);

/// CF02-Q15 and Q38: the assist pounce on a staggered goblin, "Donut holds,
/// Carl finishes".
pub const CF02_ASSIST_POUNCE_STAGGERED: PartyFixture = PartyFixture {
    base: &ASSIST_POUNCE_BASE,
    two_player: false,
    tuning: ASSIST,
    script: &[
        primary(5, 6, 0),
        primary(50, 51, 0),
        run(58, 70, 0, 1.0),
        primary(72, 73, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const ASSIST_GLANCE_BASE: Fixture = base(
    "cf02_assist_hp_drops_midmove",
    "Solo as Carl: the AI Donut starts a non-lethal pounce, Carl's jab lowers the goblin to 1 HP before it lands and the pounce glances (damage 0), so Carl takes the kill (CF02-Q38)",
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
    "Solo as Carl, who stands: two goblins tell one after the other and the AI Donut starts at most one move per 60 ticks, never while a move runs (CF02-Q16)",
    0xC0A1_0005,
    &[chase(500), chase(520)],
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
    "Solo as Carl, who stands, biscuit unlocked with 3 charges: the AI Donut casts one Magic Missile at a llama's tell (3 to 2) and never again, the reserve of 2 (CF02-Q17, Q10-A09)",
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

pub const COMPANION_FIXTURES: [&PartyFixture; 12] = [
    &CF02_ASSIST_SWAT_INTERRUPT,
    &CF02_ASSIST_SWAT_TRAILING_GAP,
    &CF02_ASSIST_POUNCE_STAGGERED,
    &CF02_ASSIST_HP_DROPS_MIDMOVE,
    &CF02_ASSIST_CADENCE,
    &CF02_ASSIST_MAGIC_LOCKED,
    &CF02_ASSIST_MISSILE_RESERVE,
    &CF02_ASSIST_CAMERA_PARITY,
    &COMPANION_CARL_INTERRUPTS_FOR_DONUT,
    &COMPANION_DONUT_SUPPORTS_CARL,
    &COMPANION_DUO_NO_AI,
    &COMPANION_SWAP_MID_MOVE,
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
                    if events.iter().any(|e| {
                        matches!(e, CombatEvent::Hit { tick: x, move_id, .. }
                            if *x == tick && is_driven(move_id.hero()))
                    }) {
                        t.ai_interrupts += 1;
                    }
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

    fn sim_of(fixture: &'static PartyFixture) -> PartyFixtureSim {
        PartyFixtureSim::new(&kit(), 360, fixture).unwrap()
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
        let mut sim = PartyFixtureSim::new(&kit(), 360, fixture).unwrap();
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
                "t={t} carl={:.0} donut={:.0} dmove={:?} gob={gob:?} llama={llamas:?} driven={:?}",
                sim.scene.world.body(HeroId::Carl).x,
                sim.scene.world.body(HeroId::Donut).x,
                sim.combat.donut_move_view().map(|v| (v.mv, v.tick)),
                c.companion_driven(),
            );
        }
    }

    /// The committed final `PARTY_STATE` hash of every fixture here (the CI
    /// replays in `docs/evidence/companion-ai` pin the same values).
    const GOLDENS: [(&str, u64); 12] = [
        ("cf02_assist_swat_interrupt", 0x17348ad0b5fdab64),
        ("cf02_assist_swat_trailing_gap", 0x1d15e293cd423414),
        ("cf02_assist_pounce_staggered", 0xf7175a0520f0e191),
        ("cf02_assist_hp_drops_midmove", 0xe78c93737f4ba899),
        ("cf02_assist_cadence", 0x44328606ab0252af),
        ("cf02_assist_magic_locked", 0x2e0450bbda409d12),
        ("cf02_assist_missile_reserve", 0xc65d91e525de91ae),
        ("cf02_assist_camera_parity", 0xa5f8df9d0ba7e654),
        ("companion_carl_interrupts_for_donut", 0x2c0f6cc83f4d2ae2),
        ("companion_donut_supports_carl", 0x23ceb3e6d143be48),
        ("companion_duo_no_ai", 0x67ac6477f6d13dd9),
        ("companion_swap_mid_move", 0x9c3f5fd58d8a64e7),
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
            8,
            "the eight assistant fixtures moved from CF02-C2"
        );
    }

    /// CF02-Q14: the AI Donut swats a tell aimed at the controlled Carl, Carl stays
    /// passive (Q13), nobody dies.
    #[test]
    fn q14_the_assist_swat_breaks_the_tell_and_ai_carl_is_passive() {
        let (sim, events, _) = run_events(&CF02_ASSIST_SWAT_INTERRUPT);
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
        let t = tally_of(&CF02_ASSIST_POUNCE_STAGGERED);
        let (_, events, _) = run_events(&CF02_ASSIST_POUNCE_STAGGERED);
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
        let (_, events, _) = run_events(&CF02_ASSIST_HP_DROPS_MIDMOVE);
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
        let t = tally_of(&CF02_ASSIST_HP_DROPS_MIDMOVE);
        assert_eq!((t.glances, t.ai_kills, t.human_kills), (1, 0, 1));
    }

    /// CF02-Q16: at most one AI-started move per 60 ticks, none while one runs.
    #[test]
    fn q16_one_ai_started_move_per_sixty_ticks() {
        let (_, events, companion) = run_events(&CF02_ASSIST_CADENCE);
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

    /// CF02-Q39 (floor and ceiling) on the two goblin fight, per profile set (CA-32).
    #[test]
    fn q39_the_assist_floor_and_ceiling_hold() {
        for f in [&CF02_ASSIST_CAMERA_PARITY, &COMPANION_DONUT_SUPPORTS_CARL] {
            let t = tally_of(f);
            let set = f.tuning;
            let _ = set;
            assert!(t.assist_starts[1] >= 2, "{}: {t:?}", f.name());
            assert!(t.ai_hit_share_percent() <= 25, "{}: {t:?}", f.name());
            assert_eq!(t.ai_kills, 0);
        }
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
            CompanionEvent::Decision { .. } => true,
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
            if !f.two_player {
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
}
