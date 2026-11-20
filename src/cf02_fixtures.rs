//! CF02 replay fixtures (CF02-00: an empty registry, plus the listing helper).
//!
//! The registry has the shape of [`crate::donut_fixtures::DONUT_FIXTURES`]
//! (`&PartyFixture`) so a `cf02_*` fixture replays through `--party-fixture`.
//! CF02-A, H, B, G and F append here; Donut and llama fixtures stay in
//! `donut_fixtures.rs` and the COMPANION ones in `companion_fixtures.rs`.
//! `CF02_FIXTURES` is deliberately not chained into `PartyFixture::all()` (so
//! `PartyFixture::by_name` and generic tests over `all()` do not see it);
//! `--party-fixture` and `--list-fixtures` use [`find_party_fixture`] and
//! [`list_fixtures`].

use crate::boss_scene::{FULL, Pilot, boss_base};
use crate::combat::EnemyAi;
use crate::combat_scene::{EnemySpawn, FIXTURES, Fixture};
use crate::companion_fixtures::COMPANION_FIXTURES;
use crate::donut_fixtures::{physical, primary};
use crate::donut_mana::DonutSetup;
use crate::input::RawInput;
use crate::party::PROVISIONAL_TUNING;
use crate::party_scene::{PartyFixture, SlotHold, base, run};

/// A stationary goblin test dummy (it never tells or walks; it still gets hurt,
/// launched, lies down and gets up).
const fn dummy(x: i32) -> EnemySpawn {
    EnemySpawn {
        x,
        ai: EnemyAi::Passive,
    }
}

/// Down + Physical held over `from..to` on `slot`: the stomp (or, in the air, the
/// dive). Its press edge is on `from`.
pub(crate) const fn stomp(from: u64, to: u64, slot: usize) -> SlotHold {
    SlotHold {
        from,
        to,
        slot,
        input: RawInput {
            physical: true,
            down: true,
            ..RawInput::NONE
        },
    }
}

/// Hit points of a CF02 fixture goblin that needs more than the legacy 2 (so a
/// non-lethal hit can land on a lying goblin before the contact knockdown of
/// CF02-B can put one on the floor unhurt). `None`: the legacy 2.
pub fn goblin_health(fixture: &str) -> Option<u8> {
    match fixture {
        "cf02_stomp_no_stand_up" | "cf02_pounce_keeps_down" | "cf02_pounce_hold" => Some(4),
        _ => None,
    }
}

// -------------------------------------------------------------- CF02-A --

/// Every fixture here starts Carl at column 400, facing right. A `dummy` goblin
/// never tells or walks, so each scenario isolates one rule. Ticks are the 1-based
/// ticks of [`crate::combat_scene`]; a hold over `from..to` gives its press edge on
/// `from`.
const KICK_THEN_STOMP_BASE: Fixture = base(
    "cf02_kick_then_stomp",
    "CF02-A: Carl kicks a goblin down, walks to it and stomps it while it lies: three beats (kick, walk, ground finish)",
    0xCF02_0A01,
    &[dummy(430)],
    120,
);

/// Q01, Q02, Q04: the kick lands at 18, the goblin lies from about 43 for 40 ticks
/// and the stomp (pressed 66, active 75) finishes it in place as a heavy kill.
pub const CF02_KICK_THEN_STOMP: PartyFixture = PartyFixture {
    base: &KICK_THEN_STOMP_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[physical(5, 6, 0), run(48, 66, 0, 1.0), stomp(66, 67, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const STOMP_WHIFF_BASE: Fixture = base(
    "cf02_stomp_whiff",
    "CF02-A: a stomp with no goblin in reach plays out its 26 ticks, steps Carl forward 6 px and hits nothing",
    0xCF02_0A02,
    &[dummy(520)],
    60,
);

/// Q02: startup 9, active 3, recovery 14; the 6 px root step.
pub const CF02_STOMP_WHIFF: PartyFixture = PartyFixture {
    base: &STOMP_WHIFF_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[stomp(5, 6, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const STOMP_STANDING_BASE: Fixture = base(
    "cf02_stomp_standing",
    "CF02-A: a stomp on a standing goblin is an ordinary 1-damage hit: stagger 10, no knock, no knockdown",
    0xCF02_0A03,
    &[dummy(430)],
    60,
);

/// Q02: standing target, stagger 10, knock 0.
pub const CF02_STOMP_STANDING: PartyFixture = PartyFixture {
    base: &STOMP_STANDING_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[stomp(5, 6, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const STOMP_NO_STAND_UP_BASE: Fixture = base(
    "cf02_stomp_no_stand_up",
    "CF02-A: a 4 HP goblin kicked down and stomped (non-lethal) stays lying: the stomp deals its damage and never stands it up",
    0xCF02_0A04,
    &[dummy(430)],
    130,
);

/// Q34: a non-lethal stomp on a lying goblin keeps it down, its timer unchanged
/// (a tough goblin: see [`goblin_health`]).
pub const CF02_STOMP_NO_STAND_UP: PartyFixture = PartyFixture {
    base: &STOMP_NO_STAND_UP_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[physical(5, 6, 0), run(48, 66, 0, 1.0), stomp(66, 67, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const CONFIRM_KICK_BASE: Fixture = base(
    "cf02_confirm_kick",
    "CF02-A: a jab that connects, then Physical in its window: the follow-through kick starts on jab tick 9 (startup 9, not 13)",
    0xCF02_0A05,
    &[dummy(430)],
    70,
);

/// Q05: the hit-confirm branch.
pub const CF02_CONFIRM_KICK: PartyFixture = PartyFixture {
    base: &CONFIRM_KICK_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), physical(10, 11, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const WHIFF_NO_CONFIRM_BASE: Fixture = base(
    "cf02_whiff_no_confirm",
    "CF02-A: a jab that whiffs gives no follow-through: the Physical press is buffered and starts the plain kick when the jab ends",
    0xCF02_0A06,
    &[dummy(470)],
    70,
);

/// Q05: a whiffed jab is not a hit-confirm.
pub const CF02_WHIFF_NO_CONFIRM: PartyFixture = PartyFixture {
    base: &WHIFF_NO_CONFIRM_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), physical(15, 16, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DOWN_PHYSICAL_BEATS_CONFIRM_BASE: Fixture = base(
    "cf02_down_physical_beats_confirm",
    "CF02-A: Down + Physical inside the confirm window of a connecting jab is the stomp (buffered past the jab), never the follow-through",
    0xCF02_0A07,
    &[dummy(430)],
    70,
);

/// Q35: Down + Physical is always the stomp.
pub const CF02_DOWN_PHYSICAL_BEATS_CONFIRM: PartyFixture = PartyFixture {
    base: &DOWN_PHYSICAL_BEATS_CONFIRM_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), stomp(17, 18, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const JAB_OVER_LYING_BASE: Fixture = base(
    "cf02_jab_over_lying",
    "CF02-A: Carl's jab and haymaker pass over a lying goblin (lying hurtbox 36 x 20); it stays down and gets up on schedule",
    0xCF02_0A08,
    &[dummy(430)],
    130,
);

/// Q03: the lying hurtbox.
pub const CF02_JAB_OVER_LYING: PartyFixture = PartyFixture {
    base: &JAB_OVER_LYING_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[
        physical(5, 6, 0),
        run(48, 66, 0, 1.0),
        primary(68, 69, 0),
        primary(75, 76, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const LETHAL_KICK_FAR_BASE: Fixture = base(
    "cf02_lethal_kick_far",
    "CF02-A: a kick that kills throws the body 240 / 220 px/s, about 125 px, far past a jab kill's 75 px",
    0xCF02_0A09,
    &[dummy(430)],
    100,
);

/// Q06: the lethal kick knock.
pub const CF02_LETHAL_KICK_FAR: PartyFixture = PartyFixture {
    base: &LETHAL_KICK_FAR_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), physical(30, 31, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const POUNCE_KEEPS_DOWN_BASE: Fixture = base(
    "cf02_pounce_keeps_down",
    "CF02-A, two players: Carl kicks a 4 HP goblin down and P2 Donut's non-lethal pounce on it keeps it lying (it never stands up)",
    0xCF02_0A0A,
    &[dummy(430)],
    110,
);

/// Q34, Q38: a non-lethal pounce on a lying goblin keeps it down.
pub const CF02_POUNCE_KEEPS_DOWN: PartyFixture = PartyFixture {
    base: &POUNCE_KEEPS_DOWN_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        physical(5, 6, 0),
        run(48, 68, 0, 1.0),
        run(30, 56, 1, 1.0),
        physical(56, 57, 1),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const POUNCE_HOLD_BASE: Fixture = base(
    "cf02_pounce_hold",
    "CF02-A, two players: Donut's pounce holds the lying goblin's down timer for 20 ticks, once per down: a second pounce does not hold it again",
    0xCF02_0A0B,
    &[dummy(430)],
    130,
);

/// Q38: the hold, once per down instance.
pub const CF02_POUNCE_HOLD: PartyFixture = PartyFixture {
    base: &POUNCE_HOLD_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        physical(5, 6, 0),
        run(48, 68, 0, 1.0),
        run(30, 56, 1, 1.0),
        physical(56, 57, 1),
        physical(92, 93, 1),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const STOMP_BOSS_BASE: Fixture = boss_base(
    "cf02_stomp_boss",
    "CF02-A: Carl's stomp on the Juicer is an ordinary 1-damage hero hit under his super armour (no interrupt, no knockdown)",
    0xCF02_0A0C,
    380,
    FULL,
    70,
);

/// Q08: the boss keeps its armour and poise rules.
pub const CF02_STOMP_BOSS: PartyFixture = PartyFixture {
    base: &STOMP_BOSS_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[run(1, 20, 0, 1.0), stomp(20, 21, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const SAME_TICK_2P_BASE: Fixture = base(
    "cf02_2p_same_tick",
    "CF02-A, two players: both press Down + Physical on one tick; Carl's is the stomp and Donut's is her ordinary pounce (she ignores Down)",
    0xCF02_0A0D,
    &[dummy(425), dummy(447)],
    50,
);

/// Q35: Donut ignores Down.
pub const CF02_2P_SAME_TICK: PartyFixture = PartyFixture {
    base: &SAME_TICK_2P_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[stomp(10, 11, 0), stomp(10, 11, 1)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

pub const CF02_FIXTURES: [&PartyFixture; 13] = [
    &CF02_KICK_THEN_STOMP,
    &CF02_STOMP_WHIFF,
    &CF02_STOMP_STANDING,
    &CF02_STOMP_NO_STAND_UP,
    &CF02_CONFIRM_KICK,
    &CF02_WHIFF_NO_CONFIRM,
    &CF02_DOWN_PHYSICAL_BEATS_CONFIRM,
    &CF02_JAB_OVER_LYING,
    &CF02_LETHAL_KICK_FAR,
    &CF02_POUNCE_KEEPS_DOWN,
    &CF02_POUNCE_HOLD,
    &CF02_STOMP_BOSS,
    &CF02_2P_SAME_TICK,
];

/// Looks a party fixture up in `PartyFixture::all()` and then in the CF02
/// registry (which is not chained into `all()`).
pub fn find_party_fixture(name: &str) -> Option<&'static PartyFixture> {
    PartyFixture::by_name(name).or_else(|| CF02_FIXTURES.iter().copied().find(|f| f.name() == name))
}

/// Every fixture name starting with `prefix`, over every registry: the legacy
/// combat fixtures, every party fixture (party, boss, Donut and COMPANION) and
/// the CF02 registry, each name once, in registry order. An unmatched prefix
/// gives an empty list; an empty prefix lists everything.
pub fn list_fixtures(prefix: &str) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    let all = FIXTURES
        .iter()
        .map(|f| f.name)
        .chain(PartyFixture::all().map(PartyFixture::name))
        .chain(CF02_FIXTURES.iter().map(|f| f.name()))
        .chain(COMPANION_FIXTURES.iter().map(|f| f.name()));
    for name in all {
        if name.starts_with(prefix) && !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registries_list_only_their_own_names() {
        assert!(COMPANION_FIXTURES.is_empty());
        assert!(list_fixtures("companion_").is_empty());
        assert!(list_fixtures("no_such_prefix_").is_empty());
        // The CF02-C1 llama fixtures live in `donut_fixtures.rs`, which
        // `PartyFixture::all()` chains; the listing shows them first, then this
        // registry in order.
        let listed = list_fixtures("cf02_");
        let own: Vec<_> = listed
            .iter()
            .copied()
            .filter(|n| !(n.starts_with("cf02_llama_") || *n == "cf02_donut_solo_llama"))
            .collect();
        let expected: Vec<_> = CF02_FIXTURES.iter().map(|f| f.name()).collect();
        assert_eq!(own, expected);
        for name in &listed {
            assert!(find_party_fixture(name).is_some());
        }
    }

    /// The CI loops select fixtures by prefix: a name outside its registry's
    /// prefix would never run (a silent skip).
    #[test]
    fn registry_names_carry_the_prefix_the_ci_loops_select() {
        for f in CF02_FIXTURES {
            let name = f.name();
            assert!(name.starts_with("cf02_"), "{name}");
            assert!(
                !name.starts_with("cf02_assist_"),
                "{name} belongs to COMPANION"
            );
        }
        for f in COMPANION_FIXTURES {
            let name = f.name();
            assert!(
                name.starts_with("companion_") || name.starts_with("cf02_assist_"),
                "{name}"
            );
        }
    }

    #[test]
    fn listing_covers_every_existing_registry_once() {
        let all = list_fixtures("");
        let mut unique = all.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), all.len(), "each name is listed once");
        for name in [
            "cp01_kick",
            "cp05_llama_crush",
            "solo_swap_continuity",
            "boss01_full_clear_2p",
            "q10_pounce_boss",
        ] {
            assert!(all.contains(&name), "{name}");
        }
        assert_eq!(list_fixtures("q10_").len(), 8);
        assert_eq!(list_fixtures("cp05_llama").len(), 5);
        assert!(find_party_fixture("q10_pounce_hit").is_some());
        assert!(find_party_fixture("cf02_nope").is_none());
    }

    /// CF02-00 baseline for CF02-Q20: the legacy solo-swap pan, measured on the
    /// same setup as the party test `q7_a_swap_pan_may_exceed_...` (Donut far
    /// behind, Carl runs 30 ticks, swap, 80 idle ticks). Run with
    /// `cargo test --locked cf02_baseline_swap_pan_trace -- --nocapture` to print
    /// the table recorded in `docs/evidence/cf02/audit/camera_trace.md`.
    #[test]
    fn cf02_baseline_swap_pan_trace() {
        use crate::env_kit::EnvKit;
        use crate::input::RawInput;
        use crate::party::HeroId;
        use crate::party_scene::{PartyFixtureSim, SOLO_SWAP_CONTINUITY};
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let kit = EnvKit::load(&root, "assets/levels/floor01/corridor_abc_sample.json").unwrap();
        let mut sim = PartyFixtureSim::new(&kit, 360, &SOLO_SWAP_CONTINUITY).unwrap();
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
        let distance_before = (sim.scene.world.hero.x - sim.scene.world.donut_body.x).round();
        sim.step_slots([
            RawInput {
                swap: true,
                ..RawInput::NONE
            },
            idle,
        ]);
        assert_eq!(sim.party.controlled_hero(), HeroId::Donut);
        let mut last = sim.scene.camera.left();
        let mut steps = Vec::new();
        for n in 1..=80 {
            sim.step_slots([idle, idle]);
            let now = sim.scene.camera.left();
            steps.push(now - last);
            if n <= 21 {
                println!(
                    "CF02_BASELINE_SWAP_PAN tick_after_swap={n} left={now} dx={}",
                    now - last
                );
            }
            last = now;
        }
        println!("CF02_BASELINE_SWAP_PAN hero_gap_px={distance_before}");
        // Legacy pan (the baseline CF02-Q20 replaces): a hard start at the full
        // 8 px/tick cap on the first tick, 18 identical steps (the last one
        // shorter on arrival), then a hard stop.
        let mut expected = vec![-8; 18];
        expected.push(-5);
        expected.resize(80, 0);
        assert_eq!(steps, expected);
        assert!(!sim.scene.pan_boosted());
    }
    // ------------------------------------------------------------ CF02-A --

    use crate::combat::{CombatEvent, ForceTier, MoveId};
    use crate::goblin::GoblinMode;

    fn kit_of(fixture: &PartyFixture) -> crate::env_kit::EnvKit {
        use crate::env_kit::EnvKit;
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        if fixture.base.boss.is_some() {
            crate::boss_scene::load_arena(&root).unwrap()
        } else {
            EnvKit::load(&root, "assets/levels/floor01/corridor_abc_sample.json").unwrap()
        }
    }

    fn replay_for(name: &str, ticks: u64) -> crate::party_scene::PartyFixtureSim {
        let fixture = find_party_fixture(name).unwrap();
        let mut sim =
            crate::party_scene::PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        sim.run_scripted(ticks);
        sim
    }

    fn replay(name: &str) -> crate::party_scene::PartyFixtureSim {
        replay_for(name, find_party_fixture(name).unwrap().base.ticks)
    }

    /// `(tick, move, damage, health_left)` of every `Hit`.
    fn hits(sim: &crate::party_scene::PartyFixtureSim) -> Vec<(u64, MoveId, u8, u8)> {
        sim.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Hit {
                    tick,
                    move_id,
                    damage,
                    health_left,
                    ..
                } => Some((tick, move_id, damage, health_left)),
                _ => None,
            })
            .collect()
    }

    fn starts(sim: &crate::party_scene::PartyFixtureSim) -> Vec<(u64, MoveId)> {
        sim.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::MoveStarted { tick, move_id, .. } => Some((tick, move_id)),
                _ => None,
            })
            .collect()
    }

    /// The goldens: one FNV state hash per fixture, replayed twice. A change is a
    /// deliberate re-baseline of that fixture only.
    const GOLDENS: [(&str, u64); 13] = [
        ("cf02_kick_then_stomp", 0xeade1505d6805381),
        ("cf02_stomp_whiff", 0xfc5181f6b763f50b),
        ("cf02_stomp_standing", 0x46b56f9470306d5e),
        ("cf02_stomp_no_stand_up", 0x789ca7bb0835fbaa),
        ("cf02_confirm_kick", 0x838403b03715e566),
        ("cf02_whiff_no_confirm", 0xab76e42fd4550ea2),
        ("cf02_down_physical_beats_confirm", 0x439c2ea3a5c7f26a),
        ("cf02_jab_over_lying", 0x778f03a3fa2222fb),
        ("cf02_lethal_kick_far", 0x13f5c26a5a91056c),
        ("cf02_pounce_keeps_down", 0x633bd81017697c67),
        ("cf02_pounce_hold", 0xaa75cd4ff812e46b),
        ("cf02_stomp_boss", 0xc191ddce479e568b),
        ("cf02_2p_same_tick", 0xda1a168a6bc17a3e),
    ];

    #[test]
    #[ignore = "prints the goldens"]
    fn print_goldens() {
        for fixture in CF02_FIXTURES {
            println!(
                "GOLDEN {} {:#018x}",
                fixture.name(),
                replay(fixture.name()).state_hash()
            );
        }
    }

    #[test]
    fn every_cf02_fixture_replays_identically_and_matches_its_golden() {
        assert_eq!(GOLDENS.len(), CF02_FIXTURES.len());
        for (name, golden) in GOLDENS {
            let (a, b) = (replay(name), replay(name));
            assert_eq!(a.combat.events, b.combat.events, "{name} events replay");
            assert_eq!(a.state_hash(), b.state_hash(), "{name} replays");
            assert_eq!(a.state_hash(), golden, "{name}: {}", a.state_line());
        }
    }

    #[test]
    fn the_cf02_fixtures_run_under_cf02_rules() {
        for fixture in CF02_FIXTURES {
            let sim = replay_for(fixture.name(), 1);
            assert!(
                sim.combat.cf02().is_some_and(|t| t.cf02_rules),
                "{}",
                fixture.name()
            );
        }
        // A legacy fixture of the same engine does not.
        let sim = replay_for("q10_pounce_hit", 1);
        assert!(sim.combat.cf02().is_none());
    }

    #[test]
    fn q01_q02_q04_a_kick_then_a_stomp_finishes_the_lying_goblin_in_place() {
        let sim = replay("cf02_kick_then_stomp");
        assert_eq!(
            hits(&sim),
            vec![(18, MoveId::PushKick, 1, 1), (75, MoveId::Stomp, 1, 0)]
        );
        // The stomp pressed on 66 starts on 66 (startup 9, hit on 75) and lands
        // inside the 40-tick lying window that began about tick 43.
        assert!(starts(&sim).contains(&(66, MoveId::Stomp)));
        let goblin = &sim.combat.enemies[0];
        assert!(matches!(goblin.mode, GoblinMode::Down | GoblinMode::Corpse));
        assert!(!goblin.alive());
        assert!(
            sim.combat.events.iter().any(|e| matches!(
                e,
                CombatEvent::EnemyDefeated {
                    tick: 75,
                    force: ForceTier::Heavy,
                    ..
                }
            )),
            "a stomp finish is a heavy defeat"
        );
        // The body stays where it fell (lethal knock 0 / 0).
        assert!((goblin.x - 491.7).abs() < 1.0, "stayed at {}", goblin.x);
    }

    #[test]
    fn q02_a_stomp_with_nothing_in_reach_runs_26_ticks_steps_6_px_and_hits_nothing() {
        let sim = replay("cf02_stomp_whiff");
        assert_eq!(starts(&sim), vec![(5, MoveId::Stomp)]);
        assert!(hits(&sim).is_empty());
        assert!(
            (sim.scene.world.hero.x - 406.0).abs() < 0.01,
            "root step 6 px, got {}",
            sim.scene.world.hero.x
        );
        assert!(
            !sim.scene.world.hero.melee.busy(),
            "the 26-tick move is over by tick 60"
        );
    }

    #[test]
    fn q02_a_stomp_on_a_standing_goblin_staggers_it_without_knock_or_knockdown() {
        let sim = replay_for("cf02_stomp_standing", 16);
        assert_eq!(hits(&sim), vec![(14, MoveId::Stomp, 1, 1)]);
        let goblin = &sim.combat.enemies[0];
        assert_eq!(goblin.mode, GoblinMode::Hurt, "stagger, not Downed");
        assert_eq!(goblin.x, 430.0, "knock 0");
        assert_eq!(
            goblin.stagger_left, 10,
            "stagger 10 (frozen by the hit-stop)"
        );
    }

    #[test]
    fn q34_a_non_lethal_stomp_keeps_a_lying_goblin_down_and_never_stands_it_up() {
        let sim = replay_for("cf02_stomp_no_stand_up", 74);
        let before = &sim.combat.enemies[0];
        assert_eq!(before.mode, GoblinMode::Downed);
        let lying_at = before.down_ticks;
        let mut sim = sim;
        sim.step_scripted();
        assert_eq!(
            hits(&sim),
            vec![(18, MoveId::PushKick, 1, 3), (75, MoveId::Stomp, 1, 2)]
        );
        // Damaged, still lying, the timer not reset (it only waits out the
        // 7-tick freeze, like every hit-stop).
        let goblin = &sim.combat.enemies[0];
        assert_eq!((goblin.mode, goblin.health), (GoblinMode::Downed, 2));
        assert!(goblin.down_ticks >= lying_at, "the down timer is not reset");
        for _ in 0..100 {
            sim.step_scripted();
            let g = &sim.combat.enemies[0];
            assert_ne!(g.mode, GoblinMode::Hurt, "the stomp never stands it up");
        }
    }

    #[test]
    fn q05_a_jab_that_connects_then_physical_is_the_follow_through_on_jab_tick_9() {
        let sim = replay("cf02_confirm_kick");
        // The jab hit-stop (3) delays jab tick 9 from tick 14 to 17.
        assert_eq!(
            starts(&sim),
            vec![(5, MoveId::Jab), (17, MoveId::FollowKick)]
        );
        // Startup 9 (a plain push kick is 13): the first active tick is 26.
        assert_eq!(
            hits(&sim),
            vec![(8, MoveId::Jab, 1, 1), (26, MoveId::FollowKick, 1, 0)]
        );
        assert!(
            sim.combat.events.iter().any(|e| matches!(
                e,
                CombatEvent::EnemyDefeated {
                    force: ForceTier::Heavy,
                    ..
                }
            )),
            "the follow-through is a launching kick: a heavy defeat"
        );
    }

    #[test]
    fn q05_a_whiffed_jab_gives_no_follow_through_the_kick_is_buffered() {
        let sim = replay("cf02_whiff_no_confirm");
        assert_eq!(
            starts(&sim),
            vec![(5, MoveId::Jab), (19, MoveId::PushKick)],
            "the plain kick starts when the jab ends (tick 19)"
        );
        assert!(hits(&sim).is_empty());
    }

    #[test]
    fn q35_down_physical_in_the_confirm_window_is_the_stomp_never_the_follow_through() {
        let sim = replay("cf02_down_physical_beats_confirm");
        assert_eq!(
            starts(&sim),
            vec![(5, MoveId::Jab), (22, MoveId::Stomp)],
            "buffered past the jab like any press"
        );
        assert!(!starts(&sim).iter().any(|&(_, m)| m == MoveId::FollowKick));
        assert_eq!(hits(&sim).last(), Some(&(31, MoveId::Stomp, 1, 0)));
    }

    #[test]
    fn q03_the_jab_and_haymaker_pass_over_a_lying_goblin() {
        let sim = replay("cf02_jab_over_lying");
        // Only the kick ever hit it: the jab (68) and the haymaker (75) were
        // thrown while it lay.
        assert_eq!(hits(&sim), vec![(18, MoveId::PushKick, 1, 1)]);
        assert!(starts(&sim).contains(&(68, MoveId::Jab)));
        assert!(starts(&sim).contains(&(75, MoveId::Haymaker)));
        let goblin = &sim.combat.enemies[0];
        assert_eq!(goblin.health, 1);
        assert_eq!(goblin.mode, GoblinMode::Neutral, "it got up on schedule");
    }

    #[test]
    fn q03_the_lying_hurtbox_is_36_by_20_only_while_lying() {
        use crate::combat::StrikeTarget;
        let mut sim = replay_for("cf02_jab_over_lying", 60);
        let goblin = &sim.combat.enemies[0];
        assert!(goblin.lying());
        let body = StrikeTarget::body(goblin);
        assert_eq!((body.width, body.height), (36.0, 20.0));
        assert_eq!(body.bottom, goblin.y - 19.0);
        // Standing (or getting up): the 24 x 38 body again.
        for _ in 0..60 {
            sim.step_scripted();
        }
        let goblin = &sim.combat.enemies[0];
        assert!(!goblin.lying());
        let body = StrikeTarget::body(goblin);
        assert_eq!((body.width, body.height), (24.0, 38.0));
    }

    #[test]
    fn q04_a_kicked_goblin_lies_40_ticks_then_22_getting_up() {
        let mut sim = replay_for("cf02_jab_over_lying", 20);
        let mut landed = None;
        let mut up = None;
        for tick in 21..=130u64 {
            sim.step_scripted();
            let g = &sim.combat.enemies[0];
            if landed.is_none() && g.mode == GoblinMode::Downed {
                landed = Some(tick);
            }
            if landed.is_some() && up.is_none() && g.mode == GoblinMode::Neutral {
                up = Some(tick);
            }
        }
        let (landed, up) = (landed.unwrap(), up.unwrap());
        assert_eq!(up - landed, 40 + 22, "DOWNED_TICKS 40 + GETUP_TICKS 22");
    }

    #[test]
    fn q06_a_kick_that_kills_throws_the_body_about_125_px() {
        let sim = replay("cf02_lethal_kick_far");
        let goblin = &sim.combat.enemies[0];
        assert!(!goblin.alive());
        let flown = goblin.x - 433.0;
        assert!(flown > 110.0, "lethal kick flew {flown} px");
        // The same goblin killed by a jab flies the legacy 160 / 210 (about 75 px).
        assert!(hits(&sim).contains(&(43, MoveId::PushKick, 1, 0)));
    }

    #[test]
    fn q34_q38_a_non_lethal_pounce_keeps_a_lying_goblin_down() {
        let mut sim = replay_for("cf02_pounce_keeps_down", 68);
        assert_eq!(sim.combat.enemies[0].mode, GoblinMode::Downed);
        for _ in 0..20 {
            sim.step_scripted();
        }
        assert!(
            hits(&sim).contains(&(69, MoveId::Pounce, 1, 2)),
            "{:?}",
            hits(&sim)
        );
        let g = &sim.combat.enemies[0];
        assert_eq!(
            (g.mode, g.health),
            (GoblinMode::Downed, 2),
            "never stood up"
        );
    }

    #[test]
    fn q38_the_pounce_hold_pauses_the_down_timer_20_ticks_once_per_down() {
        let mut sim = replay_for("cf02_pounce_hold", 69);
        let g = &sim.combat.enemies[0];
        // Hit on 69, then the hold: the timer waits 20 ticks.
        assert_eq!(
            (g.mode, g.hold_left, g.hold_used),
            (GoblinMode::Downed, 20, true)
        );
        let frozen_at = g.down_ticks;
        let mut waited = 0;
        while sim.combat.enemies[0].hold_left > 0 {
            sim.step_scripted();
            if sim.combat.enemies[0].hold_left > 0 {
                assert_eq!(sim.combat.enemies[0].down_ticks, frozen_at, "timer paused");
                waited += 1;
            }
        }
        assert!(
            waited >= 17,
            "the hold lasted {waited} ticks (the first 3 were the freeze)"
        );
        // The second pounce (hit on 102) lands inside the same down: no new hold.
        for _ in 0..40 {
            sim.step_scripted();
        }
        assert!(
            hits(&sim)
                .iter()
                .any(|h| h.0 == 102 && h.1 == MoveId::Pounce)
        );
        let g = &sim.combat.enemies[0];
        assert_eq!(g.hold_left, 0);
        assert!(g.hold_used, "this down already used its hold");
        assert_eq!(g.health, 1);
    }

    #[test]
    fn q08_a_stomp_on_the_boss_is_an_ordinary_one_damage_hit_without_an_interrupt() {
        let sim = replay("cf02_stomp_boss");
        let hurt: Vec<_> = sim
            .combat
            .events
            .iter()
            .filter(|e| matches!(e, CombatEvent::BossHurt { .. }))
            .collect();
        assert_eq!(hurt.len(), 1, "{hurt:?}");
        let boss = sim.combat.boss_state().unwrap();
        assert_eq!(boss.hp, boss.hp_max - 1, "1 damage");
        assert!(
            sim.combat
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::BossStrike { .. })),
            "super armour: the tell the stomp landed in still became its strike"
        );
    }

    #[test]
    fn q35_donut_ignores_down_her_physical_is_the_pounce_on_the_same_tick() {
        let sim = replay("cf02_2p_same_tick");
        let s = starts(&sim);
        assert!(s.contains(&(10, MoveId::Stomp)), "{s:?}");
        assert!(s.contains(&(10, MoveId::Pounce)), "{s:?}");
        // Carl's stomp hits first (Carl, then Donut, then the goblins).
        let h = hits(&sim);
        let stomp = h.iter().position(|e| e.1 == MoveId::Stomp).unwrap();
        let pounce = h.iter().position(|e| e.1 == MoveId::Pounce).unwrap();
        assert!(stomp < pounce);
    }

    #[test]
    fn cf02_a_moves_resolve_by_id_and_carry_the_data_file_values() {
        use crate::combat::{carl_move, cf02_moves};
        let m = cf02_moves();
        assert_eq!(carl_move(MoveId::Stomp), Some(&m.stomp));
        assert_eq!(carl_move(MoveId::FollowKick), Some(&m.follow_kick));
        assert_eq!(
            (
                m.stomp.startup,
                m.stomp.active,
                m.stomp.recovery,
                m.stomp.damage
            ),
            (9, 3, 14, 1)
        );
        assert_eq!(m.stomp.total(), 26);
        assert_eq!(
            (
                m.follow_kick.startup,
                m.follow_kick.active,
                m.follow_kick.recovery
            ),
            (9, 4, 19)
        );
        assert!(
            m.follow_kick.reaction.launch,
            "the same reaction as the push kick"
        );
        assert_eq!(ForceTier::of_move(&m.stomp), ForceTier::Heavy);
        assert_eq!(ForceTier::of_move(&m.follow_kick), ForceTier::Heavy);
    }
}
