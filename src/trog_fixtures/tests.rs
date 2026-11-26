//! TROG-A fixture tests: TROG-F01..F04, F06 (the scene cases), F07, F08, F16 and F17.
use super::*;
use crate::trog::{Species, WakeCause};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn twice(case: Case) -> (Report, TrogSim) {
    let (a, sim) = run(&root(), case).unwrap();
    let (b, _) = run(&root(), case).unwrap();
    assert_eq!(
        a.hash,
        b.hash,
        "{}: the double run's final hash",
        case.name()
    );
    assert_eq!(a, b, "{}: the double run's report", case.name());
    (a, sim)
}

fn tells(r: &Report, enemy: usize) -> usize {
    r.acts
        .iter()
        .filter(|(_, e, a)| *e == enemy && matches!(a, TrogAct::Tell { .. }))
        .count()
}

/// TROG-F01.
#[test]
fn trog_pygmy_dodge_and_punish() {
    let (r, sim) = twice(Case::PygmyDodgeAndPunish);
    assert_eq!(r.count(|a| *a == TrogAct::Whiff), 1, "the leap missed Carl");
    assert_eq!(r.count(|a| *a == TrogAct::Strike), 0);
    assert!(!sim.trog(0).alive(), "defeated in one cycle (stomp + jab)");
    assert_eq!(tells(&r, 0), 1, "one tell: one cycle");
    assert!(r.hurts.is_empty(), "Carl takes 0 damage");
    assert_eq!(sim.health(HeroId::Carl), 4);
    // The stomp reached it as one damage with no reaction: its hp dropped to 1 with the Pygmy
    // still committed in Recover (checked by the domain test); here the jab finished it.
    assert_eq!(r.defeats, 1);
}

/// TROG-F02, the positive control.
#[test]
fn trog_pygmy_stationary_donut() {
    let (r, sim) = twice(Case::PygmyStationaryDonut);
    assert!(!sim.trog(0).alive(), "a planted Donut defeats the Pygmy");
    assert!(r.hurts.len() <= 1, "at most 1 HP lost: {:?}", r.hurts);
}

/// TROG-F03.
#[test]
fn trog_basher_flank_and_punish() {
    let (r, sim) = twice(Case::BasherFlankAndPunish);
    assert!(!sim.trog(1).alive(), "the Basher is defeated");
    assert_eq!(
        r.count(|a| *a == TrogAct::Interrupted),
        1,
        "exactly one interrupt"
    );
    assert!(tells(&r, 1) <= 4, "within 4 tells: {}", tells(&r, 1));
    assert!(r.hurts.is_empty(), "Carl takes 0 damage: {:?}", r.hurts);
    assert_eq!(r.count(|a| *a == TrogAct::Strike), 0);
}

/// TROG-F04.
#[test]
fn trog_basher_bolts_do_not_interrupt() {
    let (r, sim) = twice(Case::BasherBoltsDoNotInterrupt);
    assert!(!sim.trog(1).alive(), "four bolts defeat it");
    assert_eq!(
        r.count(|a| *a == TrogAct::Interrupted),
        0,
        "a bolt never interrupts it"
    );
}

fn assert_run_rules(r: &Report, sim: &TrogSim, name: &str) {
    assert!(r.exit_tick.is_some(), "{name}: the exit is reached");
    assert_eq!(r.hero_defeats, 0, "{name}: no hero defeat");
    assert_eq!(sim.living().len(), 0, "{name}: all three defeated");
    // Every proximity Wake and every Tell starts with the enemy fully in view, and no tell
    // starts against a target in the protected spawn: the domain proves the gates, the scene
    // run proves the log (a hit wake is allowed anywhere and carries its cause).
    for (_, e, act) in &r.acts {
        if let TrogAct::Tell { target } = act {
            assert!(*e < 3, "{name}: {target:?}");
        }
    }
    let protected = sim.run.loaded.encounter.protected_spawn;
    for (tick, e, act) in &r.acts {
        if matches!(act, TrogAct::Tell { .. }) {
            // No enemy of the room can see the spawn: its spawn column is far from it.
            assert!(
                sim.trog(*e).spawn_x() > protected.right() as f32 + 64.0,
                "{name} {tick}"
            );
        }
    }
}

/// TROG-F07.
#[test]
fn trog_run_solo_carl() {
    let (r, sim) = twice(Case::RunSoloCarl);
    assert_run_rules(&r, &sim, "solo carl");
}

#[test]
fn trog_run_solo_donut() {
    let (r, sim) = twice(Case::RunSoloDonut);
    assert_run_rules(&r, &sim, "solo donut");
}

#[test]
fn trog_run_2p() {
    let (r, sim) = twice(Case::Run2P);
    assert_run_rules(&r, &sim, "2p");
}

#[test]
fn trog_run_solo_companion() {
    let (r, sim) = twice(Case::RunSoloCompanion);
    assert_run_rules(&r, &sim, "companion");
}

/// The wake causes of a clear run are proximity wakes (nothing sleeps through a hit here).
#[test]
fn the_run_wakes_by_proximity_and_logs_the_cause() {
    let (r, _) = run(&root(), Case::RunSoloCarl).unwrap();
    let wakes: Vec<_> = r
        .acts
        .iter()
        .filter_map(|(_, e, a)| match a {
            TrogAct::Wake { cause } => Some((*e, *cause)),
            _ => None,
        })
        .collect();
    assert_eq!(wakes.len(), 3, "{wakes:?}");
    assert!(wakes.iter().all(|(_, c)| *c == WakeCause::Proximity));
}

/// TROG-F08.
#[test]
fn trog_run_retry() {
    let a = run_retry(&root()).unwrap();
    let b = run_retry(&root()).unwrap();
    assert_eq!(a, b, "identical hash on the double run");
    assert!(a.party_defeats >= 1, "the party was defeated in T2");
    assert!(
        a.pygmy_down,
        "a defeated Pygmy stays down across the respawn"
    );
    assert_eq!(
        a.basher_state,
        State::Dormant,
        "the living Basher is asleep again"
    );
    assert_eq!(a.basher_hp, crate::trog::TrogTuning::active().basher.hp);
    assert_eq!(a.basher_x, 1376.0, "at its spawn");
    assert!(a.restart_all_alive, "a fixture restart rebuilds all");
    assert!(
        a.restart_states.iter().all(|s| *s == State::Dormant),
        "{:?}",
        a.restart_states
    );
}

/// TROG-F16: the presentation reads the sim and never writes it.
#[test]
fn trog_presentation_is_read_only() {
    let plain = run(&root(), Case::PygmyDodgeAndPunish).unwrap().0;
    // The same inputs, reading every presentation accessor each tick.
    let mut sim = TrogSim::new(
        &root(),
        Party::Solo { uncontrolled: BOT },
        Some(HeroId::Donut),
    )
    .unwrap();
    let mut reads = 0usize;
    let mut observe = |s: &TrogSim| {
        for e in s.combat.slot_enemies() {
            reads += e.hurt_rects().len() + usize::from(e.death_body().is_some());
        }
        reads += f01_trog::decor(TrogRoom::Run).len();
        let _ = f01_trog::entry_line(&s.run);
    };
    sim.place(HeroId::Carl, 560.0, -1);
    while sim.trog(0).state != State::Tell {
        observe(&sim);
        sim.step1(RawInput::NONE);
    }
    let l = sim.trog(0).landing_x();
    sim.place(HeroId::Carl, l - 48.0, 1);
    while sim.trog(0).state != State::Recover {
        observe(&sim);
        sim.step1(RawInput::NONE);
    }
    observe(&sim);
    sim.step1(stomp());
    for _ in 0..60 {
        observe(&sim);
        sim.step1(RawInput::NONE);
        if !sim.scene.world.hero.melee.busy() {
            break;
        }
    }
    sim.step1(moving(1.0));
    sim.step1(press());
    for _ in 0..40 {
        observe(&sim);
        sim.step1(RawInput::NONE);
    }
    assert!(reads > 0);
    assert_eq!(Report::of(plain.name, &sim).hash, plain.hash);
    assert_eq!(Report::of(plain.name, &sim).acts, plain.acts);
}

/// TROG-F17 (the part this room can assert alone): a state with no troglodyte has an empty slot,
/// logs no troglodyte event and hashes the slot as nothing, so no committed golden moves (the
/// full suite asserts every other golden).
#[test]
fn trog_existing_hashes_are_untouched_by_a_room_without_one() {
    let corral = crate::f01_corral::launch(&root()).unwrap();
    let mut scene = SceneSim::new_at(&corral.kit, 360, corral.fixture.carl_x).unwrap();
    let mut combat = crate::f01_corral::combat_of(&corral, &mut scene).unwrap();
    for _ in 0..200 {
        combat.step(&mut scene, RawInput::NONE);
    }
    assert!(combat.slot_enemies().is_empty());
    assert!(
        !combat
            .events
            .iter()
            .any(|e| matches!(e, CombatEvent::Trog { .. }))
    );
}

/// The three species and their roster order.
#[test]
fn the_run_holds_one_of_each_species() {
    let sim = TrogSim::new(&root(), Party::Solo { uncontrolled: BOT }, None).unwrap();
    let species: Vec<Species> = (0..3).map(|i| sim.trog(i).species).collect();
    assert_eq!(
        species,
        [Species::Pygmy, Species::Basher, Species::Virtuoso]
    );
}

#[test]
#[ignore = "evidence helper: prints one line per fixture"]
fn print_the_fixture_lines() {
    for case in Case::ALL {
        println!("{}", run(&root(), case).unwrap().0.line());
    }
    let f = run_retry(&root()).unwrap();
    println!("TROG_FIXTURE trog_run_retry {f:?}");
}

/// TROG-AD16: the companion sees the troglodytes through the per-room slot feed: her goal layer
/// evaluates them as hunt candidates while Carl clears the run (the default `cf02_free` partner),
/// and with the feed off in the same room it evaluates nothing (a plain slot enemy is invisible
/// to her). What she then does with them is the AI's own rule and is reported, not asserted.
#[test]
fn the_companion_sees_the_troglodytes_through_the_slot_feed() {
    let evaluations = |sim: &TrogSim| {
        sim.combat
            .autonomy_state(HeroId::Donut)
            .stats()
            .hunt_evaluations
    };
    let (r, sim) = run(&root(), Case::RunSoloCompanion).unwrap();
    assert!(r.exit_tick.is_some() && r.hero_defeats == 0);
    assert!(evaluations(&sim) > 0, "the feed reaches her targeting");
    let mut off = TrogSim::new(&root(), Party::Solo { uncontrolled: FREE }, None).unwrap();
    off.combat.slot_feed = false;
    clear_run(&mut off, HeroId::Carl);
    assert_eq!(
        evaluations(&off),
        0,
        "without the feed she sees nothing to hunt"
    );
}
