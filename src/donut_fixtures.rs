//! Donut's Q10 moves and mana as replay fixtures (WORK-023, HUD-DEBT-02).
//!
//! Bevy-free party fixtures in the shape of [`crate::party_scene`]: a combat
//! fixture for the goblins and Carl, the slot scripts of the two players, and
//! Donut's declared biscuit state ([`DonutSetup`]). Each one has a committed
//! replay in `docs/evidence/donut-q10-mana` and a golden hash pinned by a test
//! in this file. They are listed after the boss fixtures in
//! [`PartyFixture::all`], so every earlier fixture, its golden and its replay
//! are untouched.
//!
//! Tick numbering is the one of [`crate::combat_scene`]: a hold over
//! `from..to` covers the steps that produce `tick = from .. to - 1`, and a
//! button held from `from` gives its press edge on `from`.

use crate::boss_scene::{FULL, Pilot, boss_base};
use crate::combat::EnemyAi;
use crate::combat_scene::{EnemySpawn, Fixture, LlamaSpawn};
use crate::donut_mana::{DonutSetup, StartState};
use crate::input::RawInput;
use crate::party::{PROVISIONAL_TUNING, PartyTuning};
use crate::party_scene::{PartyFixture, SlotHold, base, jump, run, swap};

/// A held Primary (Square/X) on `slot`: its press edge is on `from`.
pub(crate) const fn primary(from: u64, to: u64, slot: usize) -> SlotHold {
    SlotHold {
        from,
        to,
        slot,
        input: RawInput {
            attack: true,
            ..RawInput::NONE
        },
    }
}

/// A held Physical (Triangle/Y) on `slot`: its press edge is on `from`.
pub(crate) const fn physical(from: u64, to: u64, slot: usize) -> SlotHold {
    SlotHold {
        from,
        to,
        slot,
        input: RawInput {
            physical: true,
            ..RawInput::NONE
        },
    }
}

const fn chase(x: i32) -> EnemySpawn {
    EnemySpawn {
        x,
        ai: EnemyAi::Chase,
    }
}

/// The Guild placeholder: Donut's pivot inside this half-open x range grants
/// the biscuit (the corridor has no Guild room yet).
pub const GUILD_PLACEHOLDER_ZONE: (f32, f32) = (520.0, 560.0);

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

// ------------------------------------------------------------------ swat --

const SWAT_INTERRUPT_BASE: Fixture = base(
    "q10_swat_interrupt",
    "Two players: a goblin tells at Carl, P2 Donut swats the tell, Carl jabs a tick later and the goblin falls",
    0x0D10_0001,
    &[chase(520)],
    260,
);

/// Q10: the swat breaks a tell without freezing the goblin, so Carl's jab on
/// the next tick still connects.
pub const Q10_SWAT_INTERRUPT: PartyFixture = PartyFixture {
    base: &SWAT_INTERRUPT_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        // P2 steps up beside Carl so the goblin is inside the swat's reach.
        run(70, 80, 1, 1.0),
        // The goblin's tell runs from tick 89 to 112: the swat hits on 99.
        primary(95, 96, 1),
        // Carl's jab on the next tick connects (the swat froze only Donut),
        // then his haymaker finishes the staggered goblin.
        primary(100, 101, 0),
        primary(106, 107, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const SWAT_GUARD_BASE: Fixture = base(
    "q10_swat_guard",
    "Solo as Donut: the first swat interrupts the tell, the second meets the light guard and glances, so the swing lands",
    0x0D10_0002,
    &[chase(470)],
    300,
);

/// Q10-06: a goblin facing only Donut lands every second attack.
pub const Q10_SWAT_GUARD: PartyFixture = PartyFixture {
    base: &SWAT_GUARD_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[
        swap(3, 4),
        run(5, 15, 0, 1.0),
        primary(45, 46, 0),
        // The second tell (81): the guard is up, the swat glances and the swing lands.
        primary(85, 86, 0),
        // The guard cleared at the swing (107): the third tell (176) is interrupted again.
        primary(180, 181, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// ---------------------------------------------------------------- pounce --

const POUNCE_HIT_BASE: Fixture = base(
    "q10_pounce_hit",
    "Two players: Carl jabs the goblin to 1 HP, P2 Donut pounces from behind him and finishes it (credited to Donut)",
    0x0D10_0003,
    &[chase(520)],
    260,
);

/// Q10: the pounce finishes a goblin Carl weakened; the kill is credited to
/// hero index 1.
pub const Q10_POUNCE_HIT: PartyFixture = PartyFixture {
    base: &POUNCE_HIT_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(95, 96, 0), physical(96, 97, 1)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const POUNCE_MISS_BASE: Fixture = base(
    "q10_pounce_miss",
    "Solo as Donut: a pounce from about 100 px whiffs and lands her in front of the goblin, which tells and hits her",
    0x0D10_0004,
    &[chase(560)],
    220,
);

/// Q10: the pounce's risk is the whiff. A pounce from about 100 px relocates
/// her 42 px and hits nothing; the goblin then closes in and its swing finds
/// her standing where she landed.
pub const Q10_POUNCE_MISS: PartyFixture = PartyFixture {
    base: &POUNCE_MISS_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[swap(3, 4), physical(70, 71, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// ------------------------------------------------------------------ mana --

const POUNCE_BOSS_BASE: Fixture = boss_base(
    "q10_pounce_boss",
    "Two players in the boss arena: P2 Donut's pounce hits the Juicer as an ordinary 1 damage hero hit under his super armour (no interrupt, no stagger)",
    0x0D10_0008,
    380,
    FULL,
    90,
);

/// Q10-A03: the pounce on the boss goes through the boss' hero-hit rule.
pub const Q10_POUNCE_BOSS: PartyFixture = PartyFixture {
    base: &POUNCE_BOSS_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[physical(12, 13, 1)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DRY_PRESS_BASE: Fixture = base(
    "q10_dry_press",
    "Two players, Donut starts with the biscuit and no mana: three dry presses (no cast, no lock), the first charge returns on tick 70 and the next press casts",
    0x0D10_0005,
    &[chase(600)],
    160,
);

/// Mana v0: a press at 0 mana is a dry press, and mashing it never blocks the
/// regen.
pub const Q10_DRY_PRESS: PartyFixture = PartyFixture {
    base: &DRY_PRESS_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        primary(10, 11, 1),
        primary(20, 21, 1),
        primary(30, 31, 1),
        primary(100, 101, 1),
    ],
    pilot: Pilot::None,
    donut: biscuit(0),
};

const MANA_REGEN_BASE: Fixture = base(
    "q10_mana_regen",
    "Two players, Donut starts with the biscuit and 8 charges: eight bolts at the 16 tick cadence, a dry press, then the full refill 350 ticks after the last release",
    0x0D10_0006,
    &[],
    520,
);

/// Mana v0: 8 charges, a release restarts the regen delay, charge k returns
/// at `release + 30 + 40 k`.
pub const Q10_MANA_REGEN: PartyFixture = PartyFixture {
    base: &MANA_REGEN_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        primary(10, 11, 1),
        primary(26, 27, 1),
        primary(42, 43, 1),
        primary(58, 59, 1),
        primary(74, 75, 1),
        primary(90, 91, 1),
        primary(106, 107, 1),
        primary(122, 123, 1),
        // The ninth press meets an empty pool.
        primary(140, 141, 1),
    ],
    pilot: Pilot::None,
    donut: biscuit(8),
};

const UNLOCK_BASE: Fixture = base(
    "q10_unlock_persists",
    "Two players: P2 Donut walks into the placeholder Guild zone and gets the biscuit, casts once, is defeated by one swing; the retry keeps the unlock and refills the pool",
    0x0D10_0007,
    &[chase(640)],
    290,
);

/// One hit defeats Donut here (a test tuning), so the retry comes quickly.
const ONE_HIT_DONUT: PartyTuning = PartyTuning {
    donut_damage_per_hit: 4,
    ..PROVISIONAL_TUNING
};

/// Mana v0 section 1: the unlock fires once at the Guild entry and survives a
/// defeat retry; the pool restarts full.
pub const Q10_UNLOCK_PERSISTS: PartyFixture = PartyFixture {
    base: &UNLOCK_BASE,
    two_player: true,
    tuning: ONE_HIT_DONUT,
    script: &[
        run(5, 60, 1, 1.0),
        // Unlocked (the first tick her pivot is inside the zone): one bolt.
        primary(70, 71, 1),
    ],
    pilot: Pilot::None,
    donut: DonutSetup {
        start: StartState::LOCKED,
        guild_zone: Some(GUILD_PLACEHOLDER_ZONE),
    },
};

// ------------------------------------------------------ CF02-C1 (Q18, Q44) --
//
// Llamas as targets of Donut's moves. Every name starts with `cf02_`, so the
// combat state runs them under `cf02_rules` (Q37: `CombatState::with_donut`);
// each has a committed replay in `docs/evidence/cf02/replays`. They are listed
// after the Q10 fixtures and never change an earlier fixture or golden.

const fn llama(x: i32) -> LlamaSpawn {
    LlamaSpawn {
        x,
        ai: EnemyAi::Chase,
    }
}

/// A llama fixture: Carl at column 400, no goblin, the given llamas.
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

const LLAMA_SWAT_INTERRUPT_BASE: Fixture = llama_base(
    "cf02_llama_swat_interrupt",
    "Two players: a llama tells at Carl, P2 Donut swats the tell (0 damage, the spit is cancelled), the llama is staggered and set under the light guard",
    0x0CF0_2C01,
    &[llama(560)],
    260,
);

/// CF02-Q18: a swat on a llama's tell interrupts it with 0 damage.
pub const CF02_LLAMA_SWAT_INTERRUPT: PartyFixture = PartyFixture {
    base: &LLAMA_SWAT_INTERRUPT_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[run(5, 60, 1, 1.0), primary(150, 151, 1)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const LLAMA_SWAT_GUARD_BASE: Fixture = llama_base(
    "cf02_llama_swat_guard",
    "Two players: the first swat interrupts the tell, the second meets the light guard and glances (the spit flies), after the spit the third interrupts again",
    0x0CF0_2C02,
    &[llama(560)],
    420,
);

/// CF02-Q18 and Q10-06: a guarded llama glances the swat, so Donut alone
/// cannot lock a llama; the guard clears after 60 ticks and the next spit.
pub const CF02_LLAMA_SWAT_GUARD: PartyFixture = PartyFixture {
    base: &LLAMA_SWAT_GUARD_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        run(5, 60, 1, 1.0),
        primary(150, 151, 1),
        primary(205, 206, 1),
        primary(355, 356, 1),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const LLAMA_POUNCE_BASE: Fixture = llama_base(
    "cf02_llama_pounce",
    "Two players: P2 Donut pounces a llama three times (1 damage each, one target, it stops at the body); the third is a plain down credited to Donut",
    0x0CF0_2C03,
    &[llama(560)],
    300,
);

/// CF02-Q18: the pounce deals 1 damage to one llama and stops at its body.
pub const CF02_LLAMA_POUNCE: PartyFixture = PartyFixture {
    base: &LLAMA_POUNCE_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    // DM-A: she runs 207 px/s, so 49 ticks cover the 169 px that 55 ticks of
    // 185 px/s did and she still arrives at the llama's near face.
    script: &[
        run(5, 54, 1, 1.0),
        physical(70, 71, 1),
        physical(140, 141, 1),
        physical(210, 211, 1),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const LLAMA_BOLT_BASE: Fixture = llama_base(
    "cf02_llama_bolt_stops",
    "Two players, Donut has the biscuit: a Magic Missile ends on the first llama (1 damage) instead of passing through to the one behind it",
    0x0CF0_2C04,
    &[llama(520), llama(600)],
    200,
);

/// CF02-Q18: the bolt ends on the llama's body (it no longer flies through).
pub const CF02_LLAMA_BOLT_STOPS: PartyFixture = PartyFixture {
    base: &LLAMA_BOLT_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[run(5, 20, 1, 1.0), primary(40, 41, 1)],
    pilot: Pilot::None,
    donut: biscuit(8),
};

const DONUT_SOLO_LLAMA_BASE: Fixture = llama_base(
    "cf02_donut_solo_llama",
    "Solo as Donut against one llama: three pounces; the replay records the time to kill and the damage taken (Q44)",
    0x0CF0_2C05,
    &[llama(560)],
    420,
);

/// CF02-Q44: Donut alone against a llama. The owner judges the time to kill
/// and the damage taken (design section 8, question 7).
pub const CF02_DONUT_SOLO_LLAMA: PartyFixture = PartyFixture {
    base: &DONUT_SOLO_LLAMA_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[
        swap(3, 4),
        run(5, 60, 0, 1.0),
        physical(70, 71, 0),
        physical(140, 141, 0),
        physical(210, 211, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// ---------------------------------------------- CF02-C2 (Q19, Q20, Q55) --
//
// The solo swap and Donut's bail-out. `cf02_*` names, so the combat state runs
// them under `cf02_rules`; the committed replays live in
// `docs/evidence/cf02/replays`.

const DONUT_BAILOUT_BASE: Fixture = base(
    "cf02_donut_bailout",
    "Two players: P2 Donut swats and jumps in the swat recovery (tick 7): the cat hop leaves the ground at vx 170 away, no swat starts in the air or in the 2-tick landing recovery, the next swat after it does; a pounce recovery bails out the same way",
    0x0CF0_2C06,
    &[],
    200,
);

/// CF02-Q55 (C6): the swat's and the pounce's recovery cancel into the cat hop.
pub const CF02_DONUT_BAILOUT: PartyFixture = PartyFixture {
    base: &DONUT_BAILOUT_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        primary(5, 6, 1),
        jump(12, 13, 1),
        primary(30, 31, 1),
        primary(50, 51, 1),
        physical(90, 91, 1),
        jump(124, 125, 1),
        primary(140, 141, 1),
        primary(168, 169, 1),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const SWAP_PAN_BASE: Fixture = llama_base(
    "cf02_swap_eased_pan",
    "Solo: Carl runs right, the player swaps to Donut (12-tick cooldown, eased pan) and back; a llama at the edge of the view fires only while its whole body is inside the simulation view that the eased pan moves",
    0x0CF0_2C07,
    &[llama(760)],
    260,
);

/// CF02-Q19, Q20: the solo swap under `--cf02` (cooldown 12, the pan eased in
/// and out), with a llama on the fair-fire edge (Q40: the view is the
/// simulation's scroll camera, which the pan moves).
pub const CF02_SWAP_EASED_PAN: PartyFixture = PartyFixture {
    base: &SWAP_PAN_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING.with_cf02_swap(),
    script: &[
        run(10, 40, 0, 1.0),
        swap(50, 51),
        run(52, 60, 0, -1.0),
        swap(58, 59),
        run(70, 130, 0, 1.0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// ------------------------------------------------------------ the registry --

pub const DONUT_FIXTURES: [&PartyFixture; 15] = [
    &Q10_SWAT_INTERRUPT,
    &Q10_SWAT_GUARD,
    &Q10_POUNCE_HIT,
    &Q10_POUNCE_MISS,
    &Q10_DRY_PRESS,
    &Q10_MANA_REGEN,
    &Q10_UNLOCK_PERSISTS,
    &Q10_POUNCE_BOSS,
    &CF02_LLAMA_SWAT_INTERRUPT,
    &CF02_LLAMA_SWAT_GUARD,
    &CF02_LLAMA_POUNCE,
    &CF02_LLAMA_BOLT_STOPS,
    &CF02_DONUT_SOLO_LLAMA,
    &CF02_DONUT_BAILOUT,
    &CF02_SWAP_EASED_PAN,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss_scene::{BOSS_FIXTURES, load_arena};
    use crate::combat::{CombatEvent, DefeatCause, EnemyAi, ForceTier, MoveId};
    use crate::combat_scene::Fixture;
    use crate::combat_scene::LlamaSpawn;
    use crate::donut_mana::ManaEvent;
    use crate::donut_moves::{GlanceReason, MoveKind};
    use crate::env_kit::EnvKit;
    use crate::party::HeroId;
    use crate::party_scene::{PARTY_FIXTURES, PartyFixtureSim};
    use std::path::{Path, PathBuf};

    const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn kit_of(fixture: &PartyFixture) -> EnvKit {
        if fixture.base.boss.is_some() {
            load_arena(&root()).unwrap()
        } else {
            EnvKit::load(&root(), LAYOUT).unwrap()
        }
    }

    fn replay(fixture: &'static PartyFixture) -> PartyFixtureSim {
        let mut sim = PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        sim.run_scripted(fixture.base.ticks);
        sim
    }

    fn named(name: &str) -> PartyFixtureSim {
        replay(PartyFixture::by_name(name).unwrap())
    }

    /// Every event of the run, as the headless replay prints them.
    fn lines(sim: &PartyFixtureSim) -> Vec<String> {
        assert_eq!(sim.combat.dropped_events, 0, "the log never hit its cap");
        let mut out: Vec<String> = sim.combat.events.iter().map(ToString::to_string).collect();
        out.push(sim.state_line_combat());
        out.push(sim.state_line());
        out
    }

    trait CombatLine {
        fn state_line_combat(&self) -> String;
    }
    impl CombatLine for PartyFixtureSim {
        fn state_line_combat(&self) -> String {
            self.combat.state_line(&self.scene)
        }
    }

    fn ticks_of(sim: &PartyFixtureSim, pick: impl Fn(&CombatEvent) -> Option<u64>) -> Vec<u64> {
        sim.combat.events.iter().filter_map(pick).collect()
    }

    const GOLDENS: [(&str, u64); 8] = [
        ("q10_swat_interrupt", 0x7151_31d0_374a_16c8),
        ("q10_swat_guard", 0x67c2_3bf7_826f_3fab),
        ("q10_pounce_hit", 0xf7fe_d891_c0e2_f8c1),
        ("q10_pounce_miss", 0x4274_d6eb_4552_8cc1),
        ("q10_dry_press", 0x141b_f5d9_6f2c_55a8),
        ("q10_mana_regen", 0x7861_1a00_785e_f14e),
        ("q10_unlock_persists", 0x3934_047f_0ef2_189c),
        ("q10_pounce_boss", 0xe2de_0452_dc5d_ede4),
    ];

    /// CF02-C1: the committed final hashes of the llama fixtures.
    const CF02_GOLDENS: [(&str, u64); 7] = [
        ("cf02_llama_swat_interrupt", 0xbd69_2a6b_647a_2a41),
        ("cf02_llama_swat_guard", 0x764d_bd2f_4024_0d1e),
        ("cf02_llama_pounce", 0xb463_4446_27b7_960e),
        ("cf02_llama_bolt_stops", 0x579f_344f_6826_3c67),
        ("cf02_donut_solo_llama", 0xe1ea_e0ff_ae9b_c431),
        ("cf02_donut_bailout", 0x50c5_db02_4da0_ce67),
        ("cf02_swap_eased_pan", 0xcb19_d498_fcbc_aee8),
    ];

    /// The Q10 fixtures (the DEMO-era ones) of the registry.
    fn q10_fixtures() -> impl Iterator<Item = &'static &'static PartyFixture> {
        DONUT_FIXTURES
            .iter()
            .filter(|f| f.name().starts_with("q10_"))
    }

    #[test]
    fn the_donut_fixtures_match_their_goldens_and_replay_identically() {
        assert_eq!(
            GOLDENS.len() + CF02_GOLDENS.len(),
            DONUT_FIXTURES.len(),
            "every registered fixture has a golden"
        );
        for (name, expected) in GOLDENS.into_iter().chain(CF02_GOLDENS) {
            let fixture = PartyFixture::by_name(name).unwrap();
            let (a, b) = (replay(fixture), replay(fixture));
            assert_eq!(a.state_hash(), b.state_hash(), "{name} replays");
            assert_eq!(lines(&a), lines(&b), "{name}: the same events twice");
            assert_eq!(a.state_hash(), expected, "{name}: {}", a.state_line());
        }
        let mut hashes: Vec<u64> = GOLDENS.iter().chain(&CF02_GOLDENS).map(|g| g.1).collect();
        hashes.sort_unstable();
        hashes.dedup();
        assert_eq!(
            hashes.len(),
            GOLDENS.len() + CF02_GOLDENS.len(),
            "every fixture is distinct"
        );
    }

    #[test]
    fn the_committed_replays_are_the_replay() {
        for fixture in DONUT_FIXTURES {
            // The Q10 replays live with the Donut evidence, the CF02 ones in
            // the shared append-only CF02 folder the CI loop reads.
            let dir = if fixture.name().starts_with("cf02_") {
                "docs/evidence/cf02/replays"
            } else {
                "docs/evidence/donut-q10-mana"
            };
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(dir)
                .join(format!("headless_{}.txt", fixture.name()));
            let committed = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let committed: Vec<&str> = committed.lines().collect();
            let replay = lines(&replay(fixture));
            assert_eq!(committed, replay, "{}", fixture.name());
        }
    }

    #[test]
    fn the_names_are_unique_and_listed_after_the_boss_fixtures() {
        let names: Vec<&str> = PartyFixture::all().map(PartyFixture::name).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len());
        let first = PARTY_FIXTURES.len() + BOSS_FIXTURES.len();
        assert!(names.len() >= first + DONUT_FIXTURES.len());
        let donut = &names[first..first + DONUT_FIXTURES.len()];
        assert_eq!(
            donut,
            DONUT_FIXTURES.map(|f| f.name()),
            "the Donut fixtures follow the boss ones, in registry order"
        );
        assert!(
            donut
                .iter()
                .all(|n| n.starts_with("q10_") || n.starts_with("cf02_")),
            "Q10 first, then the CF02 ones: {donut:?}"
        );
        assert_eq!(q10_fixtures().count(), GOLDENS.len());
    }

    // ------------------------------------------------ the hash is gated --

    /// The same fixture with Q10-01 off: the attack-less Donut of #114.
    fn without_donut_moves(fixture: &'static PartyFixture) -> &'static PartyFixture {
        Box::leak(Box::new(PartyFixture {
            base: fixture.base,
            two_player: fixture.two_player,
            tuning: PartyTuning {
                donut_moves: false,
                ..fixture.tuning
            },
            script: fixture.script,
            pilot: fixture.pilot,
            donut: DonutSetup::NONE,
        }))
    }

    /// Every party and boss fixture replays to the same hash and the same
    /// events with Donut's moves on (the default) and off: nothing of the new
    /// state is hashed or logged while her abilities are locked or unused. The
    /// committed goldens of #114 and #120 pin the absolute values elsewhere.
    #[test]
    fn the_legacy_party_and_boss_fixtures_do_not_change_with_donut_moves_on_or_off() {
        for fixture in PARTY_FIXTURES.iter().chain(BOSS_FIXTURES.iter()) {
            let (on, off) = (replay(fixture), replay(without_donut_moves(fixture)));
            assert_eq!(on.state_hash(), off.state_hash(), "{}", fixture.name());
            assert_eq!(on.combat.events, off.combat.events, "{}", fixture.name());
        }
    }

    // -------------------------------------------------------------- swat --

    #[test]
    fn the_swat_cancels_the_tell_and_carls_next_tick_jab_connects() {
        let sim = named("q10_swat_interrupt");
        let events = &sim.combat.events;
        let swat_hit = events
            .iter()
            .find_map(|e| match *e {
                CombatEvent::Hit {
                    tick,
                    move_id: MoveId::Swat,
                    damage,
                    target,
                    ..
                } => Some((tick, damage, target)),
                _ => None,
            })
            .expect("the swat hit");
        assert_eq!((swat_hit.1, swat_hit.2), (0, 0), "damage 0");
        assert!(
            events.contains(&CombatEvent::GoblinInterrupted {
                tick: swat_hit.0,
                goblin: 0
            }),
            "the tell was cancelled on the swat's hit tick"
        );
        // The goblin was not frozen by the swat: Carl's jab (first active tick
        // is press + 3 = 103) landed 4 ticks after it, and the haymaker
        // finished it.
        let jab = ticks_of(&sim, |e| match *e {
            CombatEvent::Hit {
                tick,
                move_id: MoveId::Jab,
                ..
            } => Some(tick),
            _ => None,
        });
        assert_eq!(jab, vec![103]);
        assert_eq!(sim.combat.enemies[0].health, 0);
        assert_eq!(sim.scene.world.hero.health, 4, "nobody was hurt");
    }

    /// A jab whose first active tick is the very next tick after the swat's
    /// hit tick connects: the swat's hit-stop split leaves the goblin unfrozen.
    #[test]
    fn a_carl_jab_on_the_tick_after_the_swat_still_connects() {
        const JAB_NEXT_TICK: PartyFixture = PartyFixture {
            // Jab pressed on 97: first active tick 100, one after the swat hit (99).
            script: &[run(70, 80, 1, 1.0), primary(95, 96, 1), primary(97, 98, 0)],
            ..Q10_SWAT_INTERRUPT
        };
        let sim = replay(&JAB_NEXT_TICK);
        let hits: Vec<(u64, MoveId)> = sim
            .combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Hit { tick, move_id, .. } => Some((tick, move_id)),
                _ => None,
            })
            .collect();
        assert_eq!(hits[..2], [(99, MoveId::Swat), (100, MoveId::Jab)]);
    }

    #[test]
    fn the_light_guard_makes_a_goblin_facing_only_donut_land_every_second_attack() {
        let sim = named("q10_swat_guard");
        let swat_hits = ticks_of(&sim, |e| match *e {
            CombatEvent::Hit {
                tick,
                move_id: MoveId::Swat,
                ..
            } => Some(tick),
            _ => None,
        });
        assert_eq!(swat_hits, vec![49, 184], "the second swat glanced");
        let glances: Vec<(u64, GlanceReason)> = sim
            .combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::DonutGlance { tick, reason, .. } => Some((tick, reason)),
                _ => None,
            })
            .collect();
        assert_eq!(glances, vec![(89, GlanceReason::Guarded)]);
        // The glance changed nothing: the swing of the second tell landed, and
        // the guard (cleared by that swing) let the third swat interrupt.
        let hurt = ticks_of(&sim, |e| match *e {
            CombatEvent::HeroHurt {
                tick,
                hero: HeroId::Donut,
                ..
            } => Some(tick),
            _ => None,
        });
        assert_eq!(hurt, vec![107, 242]);
    }

    // ------------------------------------------------------------ pounce --

    #[test]
    fn the_pounce_finishes_a_goblin_and_credits_donut_as_a_routine_melee_kill() {
        let sim = named("q10_pounce_hit");
        let defeats: Vec<CombatEvent> = sim
            .combat
            .events
            .iter()
            .copied()
            .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
            .collect();
        assert_eq!(
            defeats,
            vec![CombatEvent::EnemyDefeated {
                tick: 116,
                target: 0,
                cause: DefeatCause::Melee,
                force: ForceTier::Routine,
                instigator: Some(1),
            }]
        );
        // Carl's jab took the first HP, the pounce the second.
        let hits: Vec<(MoveId, u8)> = sim
            .combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Hit {
                    move_id, damage, ..
                } => Some((move_id, damage)),
                _ => None,
            })
            .collect();
        assert_eq!(hits, vec![(MoveId::Jab, 1), (MoveId::Pounce, 1)]);
    }

    #[test]
    fn a_whiffed_pounce_hits_nothing_and_lands_42_px_further() {
        let sim = named("q10_pounce_miss");
        assert!(
            !sim.combat
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::Hit { .. })),
            "no hit"
        );
        // 368 + 42 px of flight.
        assert!((sim.scene.world.donut_body.x - 410.0).abs() < 1.0);
        assert!(
            ticks_of(&sim, |e| match *e {
                CombatEvent::HeroHurt {
                    tick,
                    hero: HeroId::Donut,
                    ..
                } => Some(tick),
                _ => None,
            })
            .contains(&144),
            "the goblin's swing found her where she landed"
        );
    }

    #[test]
    fn a_pounce_on_the_boss_is_an_ordinary_one_damage_hero_hit() {
        let sim = named("q10_pounce_boss");
        let hurt: Vec<(u64, u32, u8, u8)> = sim
            .combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::BossHurt {
                    tick,
                    source,
                    damage,
                    hp_left,
                } => Some((tick, source, damage, hp_left)),
                _ => None,
            })
            .collect();
        assert_eq!(hurt, vec![(34, 1, 1, 35)]);
        // Super armour: the pounce never interrupted the pound that followed.
        assert!(
            sim.combat
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::BossTell { .. }))
        );
        assert!(
            !sim.combat
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
        );
    }

    // -------------------------------------------------------------- mana --

    #[test]
    fn a_dry_press_costs_nothing_and_mashing_never_blocks_the_regen() {
        let sim = named("q10_dry_press");
        let rejected = ticks_of(&sim, |e| match *e {
            CombatEvent::Mana(ManaEvent::CastRejected { tick, .. }) => Some(tick),
            _ => None,
        });
        assert_eq!(rejected, vec![10, 20, 30]);
        let regained = ticks_of(&sim, |e| match *e {
            CombatEvent::Mana(ManaEvent::ManaRegained { tick, .. }) => Some(tick),
            _ => None,
        });
        assert_eq!(regained, vec![70], "70 = 30 delay + 40 per charge");
        // The press at 100 cast: spent on cast tick 4.
        assert_eq!(
            ticks_of(&sim, |e| match *e {
                CombatEvent::Mana(ManaEvent::ManaSpent { tick, .. }) => Some(tick),
                _ => None,
            }),
            vec![104]
        );
        // A dry press starts no move and locks nothing.
        assert!(sim.combat.donut_move_view().is_none());
    }

    #[test]
    fn eight_bolts_drain_the_pool_and_the_full_refill_takes_350_ticks() {
        let sim = named("q10_mana_regen");
        let spent = ticks_of(&sim, |e| match *e {
            CombatEvent::Mana(ManaEvent::ManaSpent { tick, .. }) => Some(tick),
            _ => None,
        });
        assert_eq!(spent, vec![14, 30, 46, 62, 78, 94, 110, 126]);
        assert_eq!(
            ticks_of(&sim, |e| match *e {
                CombatEvent::Mana(ManaEvent::CastRejected { tick, .. }) => Some(tick),
                _ => None,
            }),
            vec![140]
        );
        let regained = ticks_of(&sim, |e| match *e {
            CombatEvent::Mana(ManaEvent::ManaRegained { tick, .. }) => Some(tick),
            _ => None,
        });
        // Charge k returns at release + 30 + 40 k.
        let expected: Vec<u64> = (1..=8).map(|k| 126 + 30 + 40 * k).collect();
        assert_eq!(regained, expected);
        assert_eq!(sim.combat.donut_mana_view(), Some((8, 8)));
        assert_eq!(expected[7] - 126, 350);
    }

    #[test]
    fn the_unlock_fires_once_at_the_guild_zone_and_survives_a_defeat_retry() {
        let sim = named("q10_unlock_persists");
        let granted = ticks_of(&sim, |e| match *e {
            CombatEvent::Mana(ManaEvent::UnlockGranted { tick, .. }) => Some(tick),
            _ => None,
        });
        assert_eq!(granted, vec![50], "once, on the first tick inside the zone");
        let restart_at = sim
            .combat
            .events
            .iter()
            .position(|e| matches!(e, CombatEvent::Restart { .. }))
            .expect("Donut was defeated and the party retried");
        // After the retry she casts again at once (no second grant, the pool is
        // full: 8 charges, one spent).
        let after = &sim.combat.events[restart_at..];
        assert!(
            after
                .iter()
                .any(|e| matches!(e, CombatEvent::Mana(ManaEvent::ManaSpent { left: 7, .. })))
        );
        assert!(
            !after
                .iter()
                .any(|e| matches!(e, CombatEvent::Mana(ManaEvent::UnlockGranted { .. })))
        );
        assert!(sim.combat.donut_mana_view().is_some());
    }

    #[test]
    fn a_fixture_restart_restores_the_declared_state_not_the_unlock() {
        let mut sim = named("q10_unlock_persists");
        assert!(sim.combat.donut_mana_view().is_some());
        sim.combat.restart(&mut sim.scene);
        assert_eq!(
            sim.combat.donut_mana_view(),
            None,
            "R restores the fixture's declared start: locked"
        );
        let mut biscuit = named("q10_mana_regen");
        biscuit.combat.restart(&mut biscuit.scene);
        assert_eq!(biscuit.combat.donut_mana_view(), Some((8, 8)));
    }

    /// A replayed attempt after a restart hashes like a fresh run of the same
    /// inputs, also when the first attempt cast missiles (the cast instance
    /// counter restarts with the attempt).
    #[test]
    fn a_retry_after_a_restart_that_casts_hashes_like_a_fresh_run() {
        let fixture = PartyFixture::by_name("q10_mana_regen").unwrap();
        let fresh = replay(fixture);
        let casts = |sim: &PartyFixtureSim| {
            sim.combat
                .events
                .iter()
                .filter(|e| matches!(e, CombatEvent::Mana(ManaEvent::ManaSpent { .. })))
                .count()
        };
        assert!(casts(&fresh) > 0, "the fixture casts missiles");
        let mut retried = replay(fixture);
        retried.combat.restart(&mut retried.scene);
        retried.party.restart();
        retried.run_scripted(fixture.base.ticks);
        // The events of the replayed attempt (move instances included) equal
        // the fresh run's, and so do the hashed Donut and scene state.
        let restart_at = retried
            .combat
            .events
            .iter()
            .position(|e| matches!(e, CombatEvent::Restart { .. }))
            .unwrap();
        assert_eq!(
            retried.combat.events[restart_at + 1..],
            fresh.combat.events[..]
        );
        assert_eq!(retried.scene.state_hash(), fresh.scene.state_hash());
    }

    /// CINE guard: Donut's kills reach the evaluator credited to hero 1, never
    /// to hero 0 (`CombatEvent::Hit` names no attacker, so the credit follows
    /// the instigator of the defeat and the owner of the move).
    #[test]
    fn a_donut_kill_reaches_cine_credited_to_hero_one_never_hero_zero() {
        use crate::cine::{CineEvent, PresentationSettings};
        use crate::cine_adapter::CineAdapter;
        let sim = named("q10_pounce_hit");
        let end = sim
            .combat
            .events
            .iter()
            .position(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
            .expect("the pounce kills a goblin");
        let kill: Vec<CombatEvent> = sim.combat.events[..=end].to_vec();
        let mapped: Vec<CineEvent> = kill.iter().filter_map(CineEvent::from_combat).collect();
        assert!(
            mapped.iter().any(|e| matches!(
                e,
                CineEvent::Defeated {
                    instigator: Some(1),
                    ..
                }
            )),
            "the pounce kill is Donut's: {mapped:?}"
        );
        assert!(!mapped.iter().any(|e| matches!(
            e,
            CineEvent::Defeated {
                instigator: Some(0),
                ..
            }
        )));
        // The same real kill plus a shore chain by `hero` makes a mixed chain
        // only for hero 1: the kill is credited to Donut and to nobody else.
        let chain = |hero: u32| {
            let mut a = CineAdapter::new(PresentationSettings::OFF);
            for e in &kill {
                a.push(e);
            }
            a.push(&CombatEvent::ShoreBroken {
                tick: 120,
                player: hero,
            });
            a.push(&CombatEvent::enemy_defeated(
                150,
                77,
                DefeatCause::ShoreBlock { player: hero },
                Some(hero),
                None,
            ));
            a.flush()
        };
        let donut = chain(1);
        assert_eq!(donut.len(), 1, "{donut:?}");
        assert_eq!(donut[0].moment.credited, 1);
        assert!(donut[0].moment.feats.mixed_chain);
        assert!(
            chain(0).is_empty(),
            "Carl's shore chain does not take the kill Donut pounced"
        );
    }

    #[test]
    fn the_follower_finishes_a_move_after_a_swap_and_starts_none() {
        const SWAP_MID_POUNCE: PartyFixture = PartyFixture {
            base: &SWAT_GUARD_BASE,
            script: &[
                swap(3, 4),
                // Pounce as Donut on tick 30, then swap back to Carl on 36.
                physical(30, 31, 0),
                swap(36, 37),
                // Carl's slot presses; the AI Donut must not act on them.
                physical(45, 46, 0),
            ],
            ..Q10_SWAT_GUARD
        };
        let mut sim =
            PartyFixtureSim::new(&kit_of(&SWAP_MID_POUNCE), 360, &SWAP_MID_POUNCE).unwrap();
        sim.run_scripted(37);
        assert_eq!(sim.party.controlled_hero(), HeroId::Carl);
        let view = sim.combat.donut_move_view().expect("the pounce goes on");
        assert_eq!(view.mv, MoveKind::Pounce);
        sim.run_scripted(120);
        assert!(sim.combat.donut_move_view().is_none(), "it ran to its end");
        let pounces = sim
            .combat
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    CombatEvent::MoveStarted {
                        move_id: MoveId::Pounce,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(pounces, 1, "the AI started no second move");
    }

    // ------------------------------------- CF02-C1 (Q18, Q37, Q44) --

    fn hits(sim: &PartyFixtureSim, mv: MoveId) -> Vec<(u64, u32, u8, u8)> {
        sim.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Hit {
                    tick,
                    move_id,
                    target,
                    damage,
                    health_left,
                    ..
                } if move_id == mv => Some((tick, target, damage, health_left)),
                _ => None,
            })
            .collect()
    }

    fn event_ticks(sim: &PartyFixtureSim, pick: fn(&CombatEvent) -> Option<u64>) -> Vec<u64> {
        ticks_of(sim, pick)
    }

    fn spits(e: &CombatEvent) -> Option<u64> {
        match *e {
            CombatEvent::LlamaSpit { tick, .. } => Some(tick),
            _ => None,
        }
    }

    fn interrupts(e: &CombatEvent) -> Option<u64> {
        match *e {
            CombatEvent::LlamaInterrupted { tick, .. } => Some(tick),
            _ => None,
        }
    }

    /// The same scenario under a name outside `cf02_*`: the Q37 gate is off.
    fn gated_off(fixture: &'static PartyFixture, name: &'static str) -> &'static PartyFixture {
        let base: &'static Fixture = Box::leak(Box::new(Fixture {
            name,
            summary: fixture.base.summary,
            seed: fixture.base.seed,
            carl_x: fixture.base.carl_x,
            enemies: fixture.base.enemies,
            shore_dx: fixture.base.shore_dx,
            boss: None,
            llamas: fixture.base.llamas,
            engineers: fixture.base.engineers,
            charges: fixture.base.charges,
            script: fixture.base.script,
            ticks: fixture.base.ticks,
            room: crate::combat_scene::FixtureRoom::Corridor,
        }));
        Box::leak(Box::new(PartyFixture {
            base,
            two_player: fixture.two_player,
            tuning: fixture.tuning,
            script: fixture.script,
            pilot: fixture.pilot,
            donut: fixture.donut,
        }))
    }

    #[test]
    fn cf02_llama_swat_interrupt_cancels_the_tell_with_zero_damage() {
        let sim = named("cf02_llama_swat_interrupt");
        // The first tell (tick 1) lands on Carl while Donut closes in; the
        // second (tick 151) is swatted on tick 154: no damage, the spit is
        // cancelled, so only the first and the third tell's spit exist.
        assert_eq!(hits(&sim, MoveId::Swat), vec![(154, 0, 0, 3)]);
        assert_eq!(event_ticks(&sim, interrupts), vec![154]);
        assert_eq!(event_ticks(&sim, spits), vec![37, 237]);
        assert_eq!(sim.combat.llamas[0].health, 3, "a swat never hurts it");
        // The swat set the Q10 light guard; the spit of 237 released it.
        let fixture = PartyFixture::by_name("cf02_llama_swat_interrupt").unwrap();
        let mut early = PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        early.run_scripted(200);
        assert!(early.combat.llamas[0].guard.is_some());
        assert!(sim.combat.llamas[0].guard.is_none());
    }

    #[test]
    fn cf02_llama_swat_guard_glances_once_and_never_locks_the_llama() {
        let sim = named("cf02_llama_swat_guard");
        let glances: Vec<(u64, GlanceReason)> = sim
            .combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::DonutGlance { tick, reason, .. } => Some((tick, reason)),
                _ => None,
            })
            .collect();
        assert_eq!(glances, vec![(209, GlanceReason::Guarded)]);
        // The glance changed nothing: the spit of the guarded tell flew, and
        // that spit released the guard, so the third swat interrupted again.
        assert_eq!(event_ticks(&sim, spits), vec![37, 237]);
        assert_eq!(event_ticks(&sim, interrupts), vec![154, 359]);
        assert_eq!(
            hits(&sim, MoveId::Swat),
            vec![(154, 0, 0, 3), (359, 0, 0, 3)]
        );
    }

    #[test]
    fn cf02_llama_pounce_deals_one_damage_per_hit_and_a_plain_down_credited_to_donut() {
        let sim = named("cf02_llama_pounce");
        assert_eq!(
            hits(&sim, MoveId::Pounce),
            vec![(80, 0, 1, 2), (150, 0, 1, 1), (220, 0, 1, 0)]
        );
        let defeats: Vec<CombatEvent> = sim
            .combat
            .events
            .iter()
            .copied()
            .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
            .collect();
        assert_eq!(
            defeats,
            vec![CombatEvent::EnemyDefeated {
                tick: 220,
                target: 0,
                cause: DefeatCause::Melee,
                force: ForceTier::Routine,
                instigator: Some(1),
            }],
            "a Donut kill is a routine melee kill, never the slam"
        );
        assert!(!sim.combat.llamas[0].alive());
    }

    #[test]
    fn the_pounce_stops_at_the_llama_body_and_flies_on_without_q18() {
        let on = named("cf02_llama_pounce");
        let off = replay(gated_off(
            PartyFixture::by_name("cf02_llama_pounce").unwrap(),
            "q10_llama_pounce_gate_off",
        ));
        assert!(
            hits(&off, MoveId::Pounce).is_empty(),
            "no target without Q18"
        );
        assert_eq!(off.combat.llamas[0].health, 3);
        // Without the switch she flies through the llama's column.
        let llama_x = on.combat.llamas[0].x;
        assert!(
            off.scene.world.donut_body.x > on.scene.world.donut_body.x,
            "the pounce on a llama stops at its body: {llama_x}"
        );
        // The stop point: on the hit tick (80) the pounce box overlaps the
        // llama's near face, her horizontal speed is cancelled, and she stays
        // in front of the llama's body.
        let fixture = PartyFixture::by_name("cf02_llama_pounce").unwrap();
        let mut sim = PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        sim.run_scripted(80);
        let donut = sim.scene.world.donut_body.x;
        let body = sim.combat.llamas[0].body();
        let gap = body.left - donut;
        assert!(
            (0.0..=crate::donut_moves::POUNCE_CONTACT_PX).contains(&gap),
            "she stopped in front of the llama's near face (gap {gap} px)"
        );
        assert!(
            off.scene.world.donut_body.x > body.left,
            "without Q18 the same pounce ends past that face"
        );
        sim.run_scripted(4);
        assert_eq!(
            sim.scene.world.donut_body.x, donut,
            "no horizontal travel after the contact"
        );
    }

    #[test]
    fn cf02_llama_bolt_stops_ends_on_the_first_llama_instead_of_flying_through() {
        let sim = named("cf02_llama_bolt_stops");
        assert_eq!(hits(&sim, MoveId::MagicMissile), vec![(52, 0, 1, 2)]);
        assert_eq!(
            sim.combat.llamas[1].health, 3,
            "the llama behind is untouched"
        );
        assert!(
            sim.combat.donut_bolts().is_empty(),
            "the bolt ended on the llama"
        );
        // Without Q18 the same bolt flies through both llamas.
        let off = replay(gated_off(
            PartyFixture::by_name("cf02_llama_bolt_stops").unwrap(),
            "q10_llama_bolt_gate_off",
        ));
        assert!(hits(&off, MoveId::MagicMissile).is_empty());
        assert!(off.combat.llamas.iter().all(|l| l.health == 3));
    }

    /// Q44: Donut alone kills a llama with little risk but slowly. The numbers
    /// the owner judges: three pounces, 150 ticks from the first press to the
    /// kill, and no damage to Donut. The llama only ever aims at Carl (the fair
    /// fire rule reads his column), so its one spit hits him and never her; her
    /// 24 px hurtbox under the glob line is not what this fixture exercises.
    #[test]
    fn cf02_donut_solo_llama_records_the_time_to_kill_and_the_damage_taken() {
        let sim = named("cf02_donut_solo_llama");
        let first_press = 70;
        let kill = hits(&sim, MoveId::Pounce)
            .into_iter()
            .find(|h| h.3 == 0)
            .expect("the third pounce kills")
            .0;
        assert_eq!(
            kill - first_press,
            150,
            "ticks from the first pounce to the kill"
        );
        assert_eq!(hits(&sim, MoveId::Pounce).len(), 3);
        assert_eq!(sim.scene.world.donut.health, 4, "Donut took no damage");
        assert_eq!(
            sim.scene.world.hero.health, 3,
            "the one spit hit Carl, not her"
        );
    }

    /// Q37: outside `cf02_*` (and `--cf02`) llamas are no target: Donut's moves
    /// pass through them and every event is as before.
    #[test]
    fn without_cf02_rules_llamas_are_no_target_of_donuts_moves() {
        for name in [
            "cf02_llama_swat_interrupt",
            "cf02_llama_swat_guard",
            "cf02_llama_pounce",
            "cf02_llama_bolt_stops",
            "cf02_donut_solo_llama",
        ] {
            let off = replay(gated_off(
                PartyFixture::by_name(name).unwrap(),
                "q10_llama_gate_off",
            ));
            assert!(
                !off.combat.events.iter().any(|e| matches!(
                    e,
                    CombatEvent::Hit { .. }
                        | CombatEvent::DonutGlance { .. }
                        | CombatEvent::LlamaInterrupted { .. }
                )),
                "{name}: Donut touched a llama with Q18 off"
            );
            assert!(
                off.combat
                    .llamas
                    .iter()
                    .all(|l| l.health == 3 && l.guard.is_none())
            );
            assert!(off.combat.llama_targets_live().is_none());
        }
        let on = named("cf02_llama_pounce");
        assert!(on.combat.llama_targets_live().is_some());
    }

    /// Q37: the switch is reversible by data. With `cf02_rules` on but
    /// `q18_llama_targets` off the llamas are still no target.
    #[test]
    fn q18_off_inside_cf02_rules_leaves_the_llamas_untargeted() {
        use crate::cf02::Cf02Tuning;
        use crate::llama::LlamaTargets;
        let mut t = Cf02Tuning::pinned().unwrap();
        t.cf02_rules = true;
        assert!(
            t.llama_targets(),
            "on under cf02_rules with the data as shipped"
        );
        assert!(LlamaTargets::from_tuning(&t).is_some());
        t.q18_llama_targets = false;
        assert!(!t.llama_targets());
        assert_eq!(LlamaTargets::from_tuning(&t), None);
    }

    /// The numbers of the Q18 knobs that the llama and the move domain own as
    /// constants stay equal to the data (a knob edit must edit them too).
    #[test]
    fn the_q18_knobs_equal_the_constants_the_llama_uses() {
        use crate::cf02::Cf02Tuning;
        use crate::combat::{DONUT_POUNCE, DONUT_SWAT, MAGIC_MISSILE};
        use crate::donut_moves::LIGHT_GUARD_MIN_TICKS;
        use crate::feel::{LLAMA_HURT_COOLDOWN, LLAMA_HURT_TICKS};
        use crate::llama::LlamaTargets;
        let t = Cf02Tuning::pinned().unwrap();
        assert!(!t.cf02_rules && !t.llama_targets(), "off by default (Q37)");
        assert_eq!(LlamaTargets::from_tuning(&t), None);
        assert_eq!(LlamaTargets::for_rules(false), None);
        assert_eq!(t.q18_swat_interrupt_stagger, LLAMA_HURT_TICKS);
        assert_eq!(u16::from(t.q18_llama_hurt_cooldown), LLAMA_HURT_COOLDOWN);
        assert_eq!(t.q18_guard_min_ticks, LIGHT_GUARD_MIN_TICKS);
        let on = LlamaTargets::for_rules(true).expect("on under cf02_rules");
        assert_eq!(on.swat_damage, DONUT_SWAT.damage);
        assert_eq!(on.pounce_damage, DONUT_POUNCE.damage);
        assert_eq!(on.missile_damage, MAGIC_MISSILE.damage);
    }

    /// Fair fire is unchanged: a llama outside the simulation's view never
    /// tells or spits, with Q18 on or off (Donut never wakes it).
    #[test]
    fn a_llama_outside_the_view_never_acts_under_q18() {
        const OFFSCREEN: Fixture = Fixture {
            engineers: &[],
            llamas: &[LlamaSpawn {
                x: 1000,
                ai: EnemyAi::Chase,
            }],
            ..base("cf02_test_offscreen", "test", 1, &[], 300)
        };
        const SCRIPT: &[SlotHold] = &[primary(10, 11, 1), physical(40, 41, 1)];
        let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            base: &OFFSCREEN,
            two_player: true,
            tuning: PROVISIONAL_TUNING,
            script: SCRIPT,
            pilot: Pilot::None,
            donut: DonutSetup::NONE,
        }));
        let sim = replay(fixture);
        assert!(sim.combat.llama_targets_live().is_some());
        assert!(!sim.combat.events.iter().any(|e| matches!(
            e,
            CombatEvent::LlamaTell { .. } | CombatEvent::LlamaSpit { .. }
        )));
    }

    // ------------------------------------------- CF02-C2 (Q19, Q20, Q55) --

    /// Swap pan trace: the forced-gap setup of the CF02-00 baseline (Donut far
    /// behind, Carl runs 30 ticks, swap, 80 idle ticks) under `tuning`. Returns
    /// the camera's per-tick step in px (negative: toward Donut).
    fn swap_pan_steps(tuning: PartyTuning) -> Vec<i32> {
        let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            base: crate::party_scene::SOLO_SWAP_CONTINUITY.base,
            two_player: false,
            tuning,
            script: &[],
            pilot: Pilot::None,
            donut: DonutSetup::NONE,
        }));
        let mut sim = PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        sim.scene.world.donut_body.x = 100.0;
        sim.scene.world.sync_donut_for_tests();
        let idle = RawInput::NONE;
        for _ in 0..30 {
            sim.step_slots([
                RawInput {
                    move_x: 1.0,
                    ..RawInput::NONE
                },
                idle,
            ]);
        }
        let mut last = sim.scene.camera.left();
        let mut steps = Vec::new();
        // Index 0 is the swap tick itself: the pan starts on it.
        for tick in 0..81 {
            let swap = tick == 0;
            sim.step_slots([
                RawInput {
                    swap,
                    ..RawInput::NONE
                },
                idle,
            ]);
            if swap {
                assert_eq!(sim.party.controlled_hero(), HeroId::Donut);
            }
            let now = sim.scene.camera.left();
            steps.push(now - last);
            last = now;
        }
        steps
    }

    /// CF02-Q20: the pan velocity trace, legacy against eased, on the same gap.
    /// Run with `cargo test --locked cf02_swap_pan_trace -- --nocapture` to print
    /// the table recorded in `docs/evidence/cf02/cf02-c2f/pan_trace.md`.
    #[test]
    fn cf02_swap_pan_trace_eases_in_and_out_and_arrives_where_the_legacy_pan_does() {
        let old = swap_pan_steps(PROVISIONAL_TUNING);
        let new = swap_pan_steps(PROVISIONAL_TUNING.with_cf02_swap());
        for tick in 0..24 {
            println!(
                "CF02_SWAP_PAN tick_from_swap={tick} old_dx={} new_dx={}",
                old[tick], new[tick]
            );
        }
        // Legacy (unchanged): a hard start at the 8 px cap on the swap tick, 19
        // identical steps and a hard stop.
        let mut expected = vec![-8; 19];
        expected.push(-5);
        expected.resize(81, 0);
        assert_eq!(old, expected);
        // Eased: no velocity step on the first tick (2 px), a ramp of 2 px per
        // tick, a peak under the 12 px clamp, a soft arrival, no overshoot.
        let speeds: Vec<i32> = new.iter().map(|d| d.abs()).collect();
        assert!(new.iter().all(|d| *d <= 0), "never overshoots: {new:?}");
        assert_eq!(speeds[0], 2, "no hard start: {speeds:?}");
        let moving: Vec<i32> = speeds.iter().copied().take_while(|v| *v > 0).collect();
        assert!(moving.len() >= 8, "a real ramp: {moving:?}");
        let peak = moving.iter().copied().max().unwrap();
        let peak_at = moving.iter().position(|v| *v == peak).unwrap();
        assert!(
            moving[..=peak_at].windows(2).all(|w| w[1] - w[0] <= 2),
            "the ramp up gains at most 2 px a tick: {moving:?}"
        );
        // CF02-FU-C2F U1: the pan hands over to the ordinary follow at 4 px left, which
        // finishes the last few px in one step of at most 4 px (a hero that keeps
        // running would otherwise never end the pan).
        let (slowing, handover) = moving.split_at(moving.len() - 1);
        assert!(
            slowing[peak_at..].windows(2).all(|w| w[1] <= w[0]),
            "then it only slows: {moving:?}"
        );
        assert!((6..=12).contains(&peak), "peak {peak}");
        assert!(
            slowing.last().is_some_and(|v| *v <= 4) && handover[0] <= 4,
            "a soft arrival: {moving:?}"
        );
        // The same arrival column as the legacy pan, and the pan ends.
        assert_eq!(new.iter().sum::<i32>(), old.iter().sum::<i32>());
        assert!(speeds[moving.len()..].iter().all(|v| *v == 0));
    }

    #[test]
    fn eased_pan_step_ramps_clamps_slows_and_never_overshoots() {
        use crate::party::eased_pan_step;
        // Long distance: 2, 4, ... 12, then it holds at the clamp.
        let mut v = 0;
        let ramp: Vec<i32> = (0..8)
            .map(|_| {
                v = eased_pan_step(v, 448);
                v
            })
            .collect();
        assert_eq!(ramp, [2, 4, 6, 8, 10, 12, 12, 12]);
        // Short distance: the arrival term slows it and it never exceeds what is left.
        assert_eq!(eased_pan_step(10, 7), 2);
        assert_eq!(eased_pan_step(12, 1), 1);
        assert_eq!(eased_pan_step(0, 0), 0);
        // 448 px takes about 45 ticks (legacy: 56).
        let (mut left, mut v, mut ticks) = (448, 0, 0);
        while left > 0 {
            v = eased_pan_step(v, left);
            left -= v;
            ticks += 1;
        }
        assert!((40..=50).contains(&ticks), "{ticks} ticks");
    }

    /// Q19: the cooldown is 12 under `--cf02` (legacy 20): the same press that
    /// the legacy cooldown refuses (its buffer runs out) swaps under the new one.
    #[test]
    fn cf02_swap_cooldown_is_12_and_a_press_the_legacy_one_refuses_swaps() {
        use crate::party::{CF02_SWAP_COOLDOWN_TICKS, PartyState};
        assert_eq!(CF02_SWAP_COOLDOWN_TICKS, 12);
        assert_eq!(PROVISIONAL_TUNING.swap_cooldown_ticks, 20);
        let swaps = |tuning: PartyTuning| {
            let mut party = PartyState::new(tuning);
            let mut at = Vec::new();
            for tick in 1..=80u64 {
                let press = tick == 10 || tick == 18;
                if party.request_swap(press, false) {
                    at.push(tick);
                }
            }
            at
        };
        assert_eq!(swaps(PROVISIONAL_TUNING), [10], "its buffer runs out first");
        assert_eq!(swaps(PROVISIONAL_TUNING.with_cf02_swap()), [10, 22]);
        // The hurt block, the 6-tick buffer, 2P ownership and the rest are unchanged.
        let t = PROVISIONAL_TUNING.with_cf02_swap();
        assert_eq!(
            (t.swap_buffer_ticks, t.swap_blocked_while_hurt),
            (PROVISIONAL_TUNING.swap_buffer_ticks, true)
        );
    }

    /// Q19/Q20 change nothing in two-player: a 2P fixture replays its committed
    /// hash with the `--cf02` swap tuning (nobody swaps, nothing pans).
    #[test]
    fn the_cf02_swap_tuning_changes_nothing_in_two_player() {
        for name in [
            "duo_independent_moves",
            "duo_separation_wall",
            "q10_swat_interrupt",
        ] {
            let fixture = PartyFixture::by_name(name).unwrap();
            assert!(fixture.two_player, "{name}");
            let swapped: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
                base: fixture.base,
                two_player: true,
                tuning: fixture.tuning.with_cf02_swap(),
                script: fixture.script,
                pilot: fixture.pilot,
                donut: fixture.donut,
            }));
            assert_eq!(
                replay(fixture).state_hash(),
                replay(swapped).state_hash(),
                "{name}"
            );
        }
    }

    /// Q20 + Q40: a llama's fair-fire view is the simulation's scroll camera,
    /// which the eased pan moves, so a tell starts only while its whole body is
    /// inside the view of that very tick, in the legacy pan and in the eased one.
    #[test]
    fn a_llama_on_the_view_edge_fires_only_while_inside_the_pan_moved_view() {
        let run = |tuning: PartyTuning| {
            let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
                base: &SWAP_PAN_BASE_EDGE,
                two_player: false,
                tuning,
                script: &[],
                pilot: Pilot::None,
                donut: DonutSetup::NONE,
            }));
            let mut sim = PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
            sim.scene.world.donut_body.x = 100.0;
            sim.scene.world.sync_donut_for_tests();
            let idle = RawInput::NONE;
            let mut tells = Vec::new();
            let mut in_view_at_tell = Vec::new();
            for tick in 0..260u64 {
                let slots = if tick < 20 {
                    [
                        RawInput {
                            move_x: 1.0,
                            ..RawInput::NONE
                        },
                        idle,
                    ]
                } else {
                    [
                        RawInput {
                            swap: tick == 20,
                            ..RawInput::NONE
                        },
                        idle,
                    ]
                };
                let before = sim.combat.events.len();
                sim.step_slots(slots);
                let left = sim.scene.camera.left() as f32;
                for event in &sim.combat.events[before..] {
                    if let CombatEvent::LlamaTell { tick, .. } = event {
                        let body = sim.combat.llamas[0].body();
                        tells.push(*tick);
                        in_view_at_tell.push(body.left >= left && body.right() <= left + 640.0);
                    }
                }
            }
            (tells, in_view_at_tell, sim.scene.camera.left())
        };
        let (_, old_in_view, _) = run(PROVISIONAL_TUNING);
        let (new_tells, new_in_view, _) = run(PROVISIONAL_TUNING.with_cf02_swap());
        assert!(old_in_view.iter().all(|v| *v), "legacy: {old_in_view:?}");
        assert!(!new_tells.is_empty());
        assert!(new_in_view.iter().all(|v| *v), "eased: {new_in_view:?}");
    }

    /// The pan test's llama a few px inside the fair-fire edge of Carl's view.
    const SWAP_PAN_BASE_EDGE: Fixture = Fixture {
        engineers: &[],
        llamas: &[LlamaSpawn {
            x: 740,
            ai: EnemyAi::Chase,
        }],
        ..SWAP_PAN_BASE
    };

    /// The tick (1-based) on which each Donut move started, from the log.
    fn donut_starts(sim: &PartyFixtureSim) -> Vec<(u64, MoveId)> {
        sim.combat
            .events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::MoveStarted { tick, move_id, .. }
                    if matches!(move_id, MoveId::Swat | MoveId::Pounce) =>
                {
                    Some((*tick, *move_id))
                }
                _ => None,
            })
            .collect()
    }

    /// Q55 (C6): the swat's recovery (tick 7, press on 12) and the pounce's
    /// landing recovery (from touchdown + 8) cancel into the cat hop; nothing
    /// attacks in the air or in the 2-tick landing recovery.
    #[test]
    fn cf02_donut_bailout_hops_from_the_swat_and_the_pounce_recovery() {
        let fixture = PartyFixture::by_name("cf02_donut_bailout").unwrap();
        let mut sim = PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        let mut body = Vec::new();
        for _ in 0..fixture.base.ticks {
            sim.step_scripted();
            let d = &sim.scene.world.donut_body;
            body.push((d.x, d.y, d.vy, d.grounded));
        }
        let at = |tick: usize| body[tick - 1];
        // The swat started on 5 and the jump on 12 (swat tick 7): the hop leaves
        // the ground at vy 300 (one gravity step already applied) and vx -170
        // (away from her facing, right) px/s, held until the landing.
        assert!(at(11).3 && !at(12).3 && at(11).2 == 0.0);
        assert!(
            (at(12).2 - (300.0 - 1050.0 / 60.0)).abs() < 0.01,
            "{:?}",
            at(12)
        );
        assert!((at(13).0 - at(12).0 + 170.0 / 60.0).abs() < 0.01);
        // About 33 ticks in the air, then grounded.
        let landed = (13..=60).find(|t| at(*t).3).unwrap();
        assert!((40..=48).contains(&landed), "landed on {landed}");
        let apex = (12..landed).map(|t| at(t).1).fold(f32::MIN, f32::max) - 118.0;
        assert!((40.0..=46.0).contains(&apex), "apex {apex} px");
        // The legacy jump of a swat recovery is gone: 300 px/s, not the normal jump.
        // Swat presses on 30 (airborne) never start; the one on 50 does.
        let starts = donut_starts(&sim);
        assert_eq!(starts[0], (5, MoveId::Swat));
        assert_eq!(
            starts[1],
            (50, MoveId::Swat),
            "no swat in the air: {starts:?}"
        );
        // The pounce (90) bails out from tick touchdown + 8 (press 124): a second hop.
        assert_eq!(starts[2], (90, MoveId::Pounce));
        assert!(!at(125).3 && at(125).2 > 200.0, "{:?}", at(125));
        // A swat pressed on 140 in that hop does not start; 168 after it does.
        assert_eq!(starts[3], (168, MoveId::Swat));
        assert_eq!(starts.len(), 4, "{starts:?}");
    }

    /// Q55: the hop is a human Donut's tool: the AI never bails out, and with
    /// `cf02_rules` off a recovery jump is the legacy jump.
    #[test]
    fn the_bailout_is_cf02_only_and_the_ai_never_bails_out() {
        // The same script outside `cf02_*` (the Q37 gate is off): the legacy jump
        // cancel, no 300 px/s hop.
        let fixture = PartyFixture::by_name("cf02_donut_bailout").unwrap();
        let legacy: &'static Fixture = Box::leak(Box::new(Fixture {
            name: "q10_donut_bailout_legacy",
            ..*fixture.base
        }));
        let legacy_fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            base: legacy,
            two_player: true,
            tuning: fixture.tuning,
            script: fixture.script,
            pilot: fixture.pilot,
            donut: fixture.donut,
        }));
        let mut sim = PartyFixtureSim::new(&kit_of(legacy_fixture), 360, legacy_fixture).unwrap();
        let before = sim.scene.world.donut_body.x;
        for _ in 0..40 {
            sim.step_scripted();
        }
        let moved = (sim.scene.world.donut_body.x - before).abs();
        assert!(moved < 5.0, "the legacy jump has no sideways hop: {moved}");
        // The bot's kit never records a hop: it never gets a Cancelled(Jump) from a
        // press (the AI starts no move without the companion), so the hop is idle.
        let mut bot = PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        for _ in 0..40 {
            bot.step_slots([RawInput::NONE, RawInput::NONE]);
        }
        assert!(bot.combat.donut_hop_idle());
    }
}
