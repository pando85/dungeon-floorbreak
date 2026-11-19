//! CINE-01 fixture tests (CINE-T01 to T08, T09 pure parity, T13): the real
//! combat fixtures and the DEMO-01 B3 probe run through the adapter exactly
//! as `print_combat_events` feeds it (one call per resolved tick), and the
//! moments it produces are checked against the spec's binding expectations.
//! The windowed T09 run lives in `scripts/ci_floor01_combat.sh cine`.

use std::path::Path;

use dungeon_floorbreak::cine::{CineTier, CinematicMoment, PresentationSettings};
use dungeon_floorbreak::cine_adapter::CineAdapter;
use dungeon_floorbreak::combat_scene::{CombatState, FIXTURES, Fixture, FixtureSim};
use dungeon_floorbreak::corridor::SceneSim;
use dungeon_floorbreak::demo::{self, DEMO_RULES};
use dungeon_floorbreak::env_kit::EnvKit;
use dungeon_floorbreak::input::RawInput;

const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn kit() -> EnvKit {
    EnvKit::load(root(), LAYOUT).unwrap()
}

/// One resolved tick of a fixture: step, then feed the adapter the events the
/// tick logged (the `print_combat_events` order), collecting the moments.
fn feed(sim: &mut FixtureSim, adapter: Option<&mut CineAdapter>, out: &mut Vec<CinematicMoment>) {
    if let Some(adapter) = adapter {
        for event in sim.combat.take_new_events() {
            adapter.push(event);
        }
        out.extend(adapter.flush().into_iter().map(|e| e.moment));
    }
}

fn run_fixture(
    kit: &EnvKit,
    fixture: &'static Fixture,
    mut adapter: Option<&mut CineAdapter>,
) -> (FixtureSim, Vec<CinematicMoment>) {
    let mut sim = FixtureSim::new(kit, 360, fixture).unwrap();
    let mut moments = Vec::new();
    for _ in 0..fixture.ticks {
        sim.step_scripted();
        feed(&mut sim, adapter.as_deref_mut(), &mut moments);
    }
    (sim, moments)
}

fn demo_b3(adapter: Option<&mut CineAdapter>) -> (FixtureSim, Vec<CinematicMoment>) {
    let kit = demo::load_hall(root()).unwrap();
    let fixture = demo::fixture_for_beat(3);
    let scene = SceneSim::new_at(&kit, 360, fixture.carl_x).unwrap();
    let combat = CombatState::new(fixture, &kit, &scene)
        .unwrap()
        .with_demo(&DEMO_RULES);
    let mut sim = FixtureSim { scene, combat };
    let mut moments = Vec::new();
    let mut adapter = adapter;
    for _ in 0..fixture.ticks {
        let raw: RawInput = demo::autopilot(&sim.combat, &sim.scene);
        sim.step(raw);
        feed(&mut sim, adapter.as_deref_mut(), &mut moments);
    }
    (sim, moments)
}

fn on() -> CineAdapter {
    CineAdapter::new(PresentationSettings::DEFAULT)
}

#[test]
fn t01_t02_only_the_two_shore_fixtures_ever_produce_a_moment() {
    // Every fixture, every gore-independent force: routine melee kills, the
    // lone shore crush, kick launches, llama deaths and the blast breakup
    // (`Explosion` defeats, three corpses) give nothing; the two fixtures the
    // spec names give exactly one.
    let kit = kit();
    for fixture in FIXTURES {
        let mut adapter = on();
        let (_, moments) = run_fixture(&kit, fixture, Some(&mut adapter));
        let expected = match fixture.name {
            "cp03_goblin_pileup" | "cp03_block_edge_decap" => 1,
            _ => 0,
        };
        assert_eq!(moments.len(), expected, "{}: {moments:?}", fixture.name);
    }
}

#[test]
fn t03_the_pileup_is_one_ordinary_mixed_chain_not_one_per_kill() {
    let kit = kit();
    let fixture = Fixture::by_name("cp03_goblin_pileup").unwrap();
    let (_, moments) = run_fixture(&kit, fixture, Some(&mut on()));
    assert_eq!(moments.len(), 1);
    let m = moments[0];
    // Melee at tick 83, the shore broken at 103, the block at 139: the chain
    // completes when the block lands. Score 5 plus the extra-kill cap of 2.
    assert_eq!((m.tick, m.tier, m.score), (139, CineTier::Ordinary, 7));
    assert!(m.feats.mixed_chain && !m.feats.precision_edge && !m.feats.contact_setup);
    assert_eq!(m.credited, 0);
}

#[test]
fn t05_the_edge_shear_fixture_and_b3_are_precision_edge_moments() {
    let kit = kit();
    let fixture = Fixture::by_name("cp03_block_edge_decap").unwrap();
    let (_, moments) = run_fixture(&kit, fixture, Some(&mut on()));
    assert_eq!(moments.len(), 1);
    assert!(moments[0].feats.precision_edge);
    assert_eq!(moments[0].tier, CineTier::Ordinary);

    // DEMO-01 B3: shore_broken at 123, then at tick 159 target 4 shore_block
    // and target 5 shore_block_edge, no hurt: CINE-PRECISION-EDGE, 5 + 1.
    // The haymaker kill at tick 243 is inside the 900-tick cooldown.
    let (sim, moments) = demo_b3(Some(&mut on()));
    assert_eq!(moments.len(), 1, "{moments:?}");
    let m = moments[0];
    assert_eq!((m.tick, m.tier, m.score), (159, CineTier::Ordinary, 6));
    assert!(m.feats.precision_edge && !m.feats.mixed_chain);
    assert_eq!(m.feats.extra_kills, 1);
    let kills_after = sim
        .combat
        .events
        .iter()
        .filter(|e| {
            matches!(e, dungeon_floorbreak::combat::CombatEvent::EnemyDefeated { tick, .. }
                if *tick > 159)
        })
        .count();
    assert!(kills_after >= 1, "a later kill sits inside the cooldown");
}

#[test]
fn t07_the_cooldown_survives_a_restart_on_the_monotonic_clock() {
    let kit = kit();
    let fixture = Fixture::by_name("cp03_block_edge_decap").unwrap();
    let mut adapter = on();
    let mut sim = FixtureSim::new(&kit, 360, fixture).unwrap();
    let mut moments: Vec<(u64, CinematicMoment)> = Vec::new();
    for _ in 0..12 {
        let mut now = Vec::new();
        for _ in 0..fixture.ticks {
            sim.step_scripted();
            feed(&mut sim, Some(&mut adapter), &mut now);
        }
        let elapsed = sim.combat.elapsed;
        moments.extend(now.into_iter().map(|m| (elapsed, m)));
        sim.restart();
        // The Restart event is fed with the next attempt's first tick.
    }
    // The first attempt pays out; the ones inside 900 simulated ticks do not;
    // the first one past the cooldown does again.
    assert!(moments.len() >= 2, "{moments:?}");
    let (first_elapsed, first) = moments[0];
    let (second_elapsed, second) = moments[1];
    let first_clock = first_elapsed - fixture.ticks + first.tick;
    let second_clock = second_elapsed - fixture.ticks + second.tick;
    assert!(
        second_clock - first_clock >= 900,
        "second at {second_clock}, first at {first_clock}"
    );
    // Reused target ids scored again after the cooldown: nothing stale.
    assert_eq!(second.credited, 0);
}

#[test]
fn t09_the_adapter_changes_no_authoritative_state_event_or_hash() {
    let kit = kit();
    for fixture in FIXTURES {
        let (plain, _) = run_fixture(&kit, fixture, None);
        let mut a = on();
        let mut off = CineAdapter::new(PresentationSettings::OFF);
        let (with_on, m_on) = run_fixture(&kit, fixture, Some(&mut a));
        let (with_off, m_off) = run_fixture(&kit, fixture, Some(&mut off));
        assert_eq!(plain.state_hash(), with_on.state_hash(), "{}", fixture.name);
        assert_eq!(
            plain.state_hash(),
            with_off.state_hash(),
            "{}",
            fixture.name
        );
        assert_eq!(
            plain.combat.events, with_on.combat.events,
            "{}",
            fixture.name
        );
        assert_eq!(
            plain.state_line(),
            with_off.state_line(),
            "{}",
            fixture.name
        );
        // The evaluator's verdict does not depend on the presentation setting.
        assert_eq!(m_on, m_off, "{}", fixture.name);
    }
    let (plain, _) = demo_b3(None);
    let (with_on, _) = demo_b3(Some(&mut on()));
    assert_eq!(plain.state_hash(), with_on.state_hash());
    assert_eq!(plain.combat.events, with_on.combat.events);
}

#[test]
fn t13_the_evaluator_memory_is_bounded_over_the_whole_corpus() {
    let kit = kit();
    for fixture in FIXTURES {
        let mut adapter = on();
        run_fixture(&kit, fixture, Some(&mut adapter));
        assert!(
            adapter.evaluator().held_entries()
                <= 4 * adapter.evaluator().tuning().max_history_entries,
            "{}",
            fixture.name
        );
    }
}
