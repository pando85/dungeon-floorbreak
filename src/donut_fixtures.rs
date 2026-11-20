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
use crate::combat_scene::{EnemySpawn, Fixture};
use crate::donut_mana::{DonutSetup, StartState};
use crate::input::RawInput;
use crate::party::{PROVISIONAL_TUNING, PartyTuning};
use crate::party_scene::{PartyFixture, SlotHold, base, run, swap};

/// A held Primary (Square/X) on `slot`: its press edge is on `from`.
pub(crate) const fn primary(from: u64, to: u64, slot: usize) -> SlotHold {
    SlotHold {
        from,
        to,
        slot,
        input: RawInput {
            move_x: 0.0,
            jump: false,
            attack: true,
            physical: false,
            ability: false,
            pause: false,
            swap: false,
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
            move_x: 0.0,
            jump: false,
            attack: false,
            physical: true,
            ability: false,
            pause: false,
            swap: false,
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

// ------------------------------------------------------------ the registry --

pub const DONUT_FIXTURES: [&PartyFixture; 8] = [
    &Q10_SWAT_INTERRUPT,
    &Q10_SWAT_GUARD,
    &Q10_POUNCE_HIT,
    &Q10_POUNCE_MISS,
    &Q10_DRY_PRESS,
    &Q10_MANA_REGEN,
    &Q10_UNLOCK_PERSISTS,
    &Q10_POUNCE_BOSS,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss_scene::{BOSS_FIXTURES, load_arena};
    use crate::combat::{CombatEvent, DefeatCause, ForceTier, MoveId};
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
        ("q10_swat_interrupt", 0x4bc1_f1c9_2814_7e99),
        ("q10_swat_guard", 0xc64f_48fa_8f71_2d20),
        ("q10_pounce_hit", 0xf7fe_d891_c0e2_f8c1),
        ("q10_pounce_miss", 0x4274_d6eb_4552_8cc1),
        ("q10_dry_press", 0x672f_2eca_b864_6492),
        ("q10_mana_regen", 0xf7c2_1bf4_376c_ab99),
        ("q10_unlock_persists", 0xd069_6a0b_3952_ce94),
        ("q10_pounce_boss", 0xe2de_0452_dc5d_ede4),
    ];

    #[test]
    fn the_donut_fixtures_match_their_goldens_and_replay_identically() {
        assert_eq!(GOLDENS.len(), DONUT_FIXTURES.len());
        for (name, expected) in GOLDENS {
            let fixture = PartyFixture::by_name(name).unwrap();
            let (a, b) = (replay(fixture), replay(fixture));
            assert_eq!(a.state_hash(), b.state_hash(), "{name} replays");
            assert_eq!(lines(&a), lines(&b), "{name}: the same events twice");
            assert_eq!(a.state_hash(), expected, "{name}: {}", a.state_line());
        }
        let mut hashes: Vec<u64> = GOLDENS.iter().map(|g| g.1).collect();
        hashes.sort_unstable();
        hashes.dedup();
        assert_eq!(hashes.len(), GOLDENS.len(), "every fixture is distinct");
    }

    #[test]
    fn the_committed_replays_are_the_replay() {
        for fixture in DONUT_FIXTURES {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("docs/evidence/donut-q10-mana")
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
        assert_eq!(
            names.len(),
            PARTY_FIXTURES.len() + BOSS_FIXTURES.len() + DONUT_FIXTURES.len()
        );
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len());
        assert!(
            names[names.len() - DONUT_FIXTURES.len()..]
                .iter()
                .all(|n| n.starts_with("q10_"))
        );
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
        assert_eq!(granted, vec![55], "once, on the first tick inside the zone");
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
}
