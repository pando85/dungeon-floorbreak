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
    // Every Tell starts with the enemy's whole body inside the view the slot step read, and
    // never against a target standing in the protected spawn (the scene run proves the log; a
    // hit wake is allowed anywhere and carries its cause).
    let protected = sim.run.loaded.encounter.protected_spawn;
    let (p0, p1) = (protected.x as f32, protected.right() as f32);
    assert_eq!(
        r.tells.len(),
        r.count(|a| matches!(a, TrogAct::Tell { .. }))
    );
    assert!(!r.tells.is_empty(), "{name}: the run has tells");
    for t in &r.tells {
        assert!(
            t.body.0 >= t.view.0 && t.body.1 <= t.view.1,
            "{name}: tell {t:?} starts with the body outside the view"
        );
        assert!(
            !(p0..=p1).contains(&t.target_x),
            "{name}: tell {t:?} names a target in the protected spawn"
        );
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
    let f = run_retry_strength(&root()).unwrap();
    println!("TROG_FIXTURE trog_strength_retry {f:?}");
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

// ---------------------------------------------------------------- TROG-B

/// TROG-F09: Carl solo clears the Test of Strength with the bench standing: the Virtuoso falls
/// left of the Basher's trigger, Carl climbs the 80 px static bench and defeats the Basher on
/// the far side.
#[test]
fn trog_strength_direct() {
    let (r, sim) = twice(Case::StrengthDirect);
    assert_eq!(sim.room, TrogRoom::Strength);
    assert!(r.exit_tick.is_some(), "the exit is reached");
    assert_eq!(r.hero_defeats, 0);
    assert_eq!(sim.living().len(), 0, "both defeated");
    // The bench is a static solid: it stands, whole, at the end.
    let bench = sim
        .scene
        .world
        .solids
        .iter()
        .find(|b| (b.rect.left, b.rect.width) == (640.0, 32.0) && b.intact);
    assert!(bench.is_some(), "the bench still stands");
    // The Basher slept until the Virtuoso was down (its proximity wake came later). The first
    // defeat is asserted to be the Virtuoso's (target id of roster index 0).
    let defeats: Vec<u64> = sim
        .combat
        .events
        .iter()
        .filter_map(|e| match e {
            CombatEvent::EnemyDefeated { tick, target, .. } => Some((*target, *tick)),
            _ => None,
        })
        .map(|(target, t)| {
            assert!(
                target == crate::combat_scene::trog::target_id(0)
                    || target == crate::combat_scene::trog::target_id(1)
            );
            t
        })
        .collect();
    let first_target = sim
        .combat
        .events
        .iter()
        .find_map(|e| match e {
            CombatEvent::EnemyDefeated { target, .. } => Some(*target),
            _ => None,
        })
        .expect("a defeat");
    assert_eq!(
        first_target,
        crate::combat_scene::trog::target_id(0),
        "the Virtuoso falls first"
    );
    let basher_wake = r
        .acts
        .iter()
        .find_map(|(t, e, a)| (*e == 1 && matches!(a, TrogAct::Wake { .. })).then_some(*t))
        .expect("the Basher woke");
    assert!(
        defeats[0] < basher_wake,
        "the Virtuoso (down at {}) fell before the Basher woke ({basher_wake})",
        defeats[0]
    );
    assert!(
        r.acts.iter().all(|(_, _, a)| !matches!(a, TrogAct::Strike)),
        "{:?}",
        r.acts
    );
}

/// TROG-F13, the tell-gap half: with both troglodytes eligible at once (one hero each), both tell
/// and tells start at least 24 ticks apart. The "no hero under two live hitboxes" half is
/// **structural** here and is asserted geometrically by
/// `trog_strength_live_hitboxes_cannot_overlap_one_hero` (the two reaches never meet), so the
/// run's `double_live` counter is a sanity check, not the proof.
#[test]
fn trog_strength_tell_gap() {
    let (r, sim) = twice(Case::StrengthTellGap);
    let tells: Vec<(u64, usize)> = r
        .acts
        .iter()
        .filter_map(|(t, e, a)| matches!(a, TrogAct::Tell { .. }).then_some((*t, *e)))
        .collect();
    assert!(
        tells.iter().any(|(_, e)| *e == 0) && tells.iter().any(|(_, e)| *e == 1),
        "both troglodytes told: {:?}",
        r.acts
    );
    for pair in tells.windows(2) {
        assert!(
            pair[1].0 - pair[0].0 >= 24,
            "tells {:?} start less than 24 ticks apart",
            pair
        );
    }
    assert_eq!(
        sim.double_live, 0,
        "sanity: no hero under two live hitboxes in the run"
    );
}

/// TROG-F13, the overlap half (review R3): no hero can stand under both troglodytes' live hitboxes
/// because their reaches never meet. The Virtuoso's band, clipped by the standing 80 px bench and
/// 48 px shelf at every pivot of its leash and both facings, never reaches past the bench's left
/// face (x 640), and the Basher's committed smash rectangle never starts left of its leash
/// start 704 minus its hitbox's far edge (x 663, over the bench's own column, where only a hero on top of it could stand). A hero on top of the bench (feet 80) is above
/// both bands (the Virtuoso's 36..46 and the Basher's 0..14 from the floor).
#[test]
fn trog_strength_live_hitboxes_cannot_overlap_one_hero() {
    use crate::prototype::{Block, Material, Rect};
    use crate::trog::{Species, TrogTuning, clip_band};
    let sim = TrogSim::for_room(
        &root(),
        TrogRoom::Strength,
        Party::Solo { uncontrolled: BOT },
        None,
    )
    .unwrap();
    let floor = sim.foot_of_floor();
    let solids: Vec<Block> = sim
        .run
        .loaded
        .encounter
        .statics
        .iter()
        .map(|s| Block {
            rect: Rect {
                left: s.rect.x as f32,
                bottom: floor,
                width: s.rect.w as f32,
                height: s.rect.h as f32,
            },
            material: Material::Timber,
            intact: true,
        })
        .collect();
    let bench = sim
        .run
        .loaded
        .encounter
        .statics
        .iter()
        .find(|s| s.id == "bench")
        .unwrap();
    let t = TrogTuning::active();
    let (vir, bas) = (t.species(Species::Virtuoso), t.species(Species::Basher));
    let (vx, bx) = (
        &sim.run.loaded.encounter.roster[0],
        &sim.run.loaded.encounter.roster[1],
    );
    let mut vir_max_right = f32::MIN;
    for x in (vx.x - i32::from(vir.leash_px))..=(vx.x + i32::from(vir.leash_px)) {
        for facing in [1i8, -1] {
            let band = crate::slime::world_box(&vir.hitbox, x as f32, facing, floor);
            if let Some(b) = clip_band(band, x as f32, facing, &solids) {
                vir_max_right = vir_max_right.max(b.right());
            }
        }
    }
    let mut bas_min_left = f32::MAX;
    for x in (bx.x - i32::from(bas.leash_px))..=(bx.x + i32::from(bas.leash_px)) {
        for facing in [1i8, -1] {
            let r = crate::slime::world_box(&bas.hitbox, x as f32, facing, floor);
            bas_min_left = bas_min_left.min(r.left);
        }
    }
    assert!(
        vir_max_right <= bench.rect.x as f32,
        "the Virtuoso's clipped band ends at the bench face: {vir_max_right}"
    );
    assert!(
        bas_min_left > bench.rect.x as f32,
        "the Basher's smash starts right of the bench's left face: {bas_min_left}"
    );
    assert!(vir_max_right < bas_min_left, "the two reaches never meet");
    // On top of the bench a hero is above both bands.
    let top = floor + bench.rect.h as f32;
    assert!(top > floor + f32::from(vir.hitbox.y1));
    assert!(top > floor + f32::from(bas.hitbox.y1));
}

/// TROG-F15: two players, and one player with the default companion, clear it.
#[test]
fn trog_strength_2p_and_solo_companion() {
    for case in [Case::Strength2P, Case::StrengthSoloCompanion] {
        let (r, sim) = twice(case);
        assert!(
            r.exit_tick.is_some(),
            "{}: the exit is reached",
            case.name()
        );
        assert_eq!(r.hero_defeats, 0, "{}", case.name());
        assert_eq!(sim.living().len(), 0, "{}", case.name());
        // No rescue and no teleport: the fixture never places a hero after setup and the room has
        // no rescue spot (the room has no `place` output at all: only the Dozer emits one).
        assert!(
            r.tells
                .iter()
                .all(|t| t.body.0 >= t.view.0 && t.body.1 <= t.view.1)
        );
    }
}

/// TROG-F14: respawn and restart in the Test of Strength.
#[test]
fn trog_strength_retry_and_respawn() {
    let a = run_retry_strength(&root()).unwrap();
    let b = run_retry_strength(&root()).unwrap();
    assert_eq!(a, b, "identical hash on the double run");
    assert!(
        a.party_defeats >= 1,
        "the party was defeated in front of the Basher"
    );
    assert!(
        a.virtuoso_down,
        "a defeated Virtuoso stays down across the respawn"
    );
    assert_eq!(
        a.basher_state,
        State::Dormant,
        "the living Basher is asleep again"
    );
    assert_eq!(a.basher_hp, crate::trog::TrogTuning::active().basher.hp);
    assert_eq!(a.basher_x, 800.0, "at its spawn");
    assert!(a.restart_all_alive);
    assert!(a.restart_states.iter().all(|s| *s == State::Dormant));
}

/// TROG-F16 in the Test of Strength: the same inputs as F09 while every presentation accessor is
/// read each tick (hurtboxes, death bodies, decor, the entry line) give the same log and hash.
#[test]
fn trog_strength_presentation_is_read_only() {
    let plain = run(&root(), Case::StrengthDirect).unwrap().0;
    let mut sim = TrogSim::for_room(
        &root(),
        TrogRoom::Strength,
        Party::Solo { uncontrolled: BOT },
        Some(HeroId::Donut),
    )
    .unwrap();
    let mut reads = 0usize;
    let mut last = 0u64;
    for _ in 0..MAX_TICKS {
        if sim.complete() {
            break;
        }
        for e in sim.combat.slot_enemies() {
            reads += e.hurt_rects().len() + usize::from(e.death_body().is_some());
        }
        reads += f01_trog::decor(TrogRoom::Strength).len();
        let _ = f01_trog::entry_line(&sim.run);
        let raw = match sim.living().first().copied() {
            Some(i) => {
                let t = sim.trog(i).clone();
                carl_vs(&sim.probe(), &t, &mut last)
            }
            None => moving(if sim.x_of(HeroId::Carl) < exit_x(&sim) {
                1.0
            } else {
                0.0
            }),
        };
        let raw = with_hop(&sim.probe(), HeroId::Carl, raw);
        sim.step1(raw);
    }
    assert!(reads > 0);
    assert_eq!(Report::of(plain.name, &sim).hash, plain.hash);
    assert_eq!(Report::of(plain.name, &sim).acts, plain.acts);
}

#[test]
#[ignore = "evidence helper: the carl pilot's event ticks in the Test of Strength"]
fn print_the_strength_pilot_events() {
    let mut sim = TrogSim::for_room(
        &root(),
        TrogRoom::Strength,
        Party::Solo { uncontrolled: BOT },
        None,
    )
    .unwrap();
    let exit = sim.run.loaded.encounter.exit.x as f32 + 32.0;
    let statics = sim.run.loaded.encounter.statics.clone();
    for _ in 0..800 {
        let raw = pilot(Pilot::Carl, &sim.combat, &sim.scene, &statics, exit);
        sim.step1(raw);
        if sim.complete() {
            break;
        }
    }
    println!(
        "STRENGTH_PILOT ticks={} complete={} hero_defeats={} hurts={}",
        sim.tick(),
        sim.complete(),
        sim.hero_defeats(),
        sim.hurts().len()
    );
    for (t, e, a) in sim.acts() {
        println!("STRENGTH_PILOT_ACT tick={t} enemy={e} act={}", a.name());
    }
    for e in &sim.combat.events {
        if let CombatEvent::EnemyDefeated { tick, target, .. } = e {
            println!("STRENGTH_PILOT_DEFEAT tick={tick} target={target:#x}");
        }
    }
}
