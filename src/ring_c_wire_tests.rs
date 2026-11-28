//! RING-C wire: the moving shutter of `ring_court` (`docs/03-tech/ring-c-wire.md`).
//! Headless tests of the phase cycle, the per-phase solid sync, the pin, the
//! held edge, a court traversal for the pilot and the companion, the nav after a
//! full cycle and the presenter's state-hash neutrality.

use std::path::PathBuf;

use crate::env_prop::ShutterPhase;
use crate::f01_ring::RingRoom;
use crate::f01_ring_fixtures::{SEEDS, Who, run, start};
use crate::goblin::GoblinMode;
use crate::input::RawInput;
use crate::party::HeroId;
use crate::party_scene::PartyFixtureSim;
use crate::prototype::Material;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The tick numbers below are the adopted RINGC-AD1..AD3 numbers.
const WARNING: usize = 40;
const CLOSING: usize = 34;
const CLOSED: usize = 150;

fn phase(sim: &PartyFixtureSim) -> ShutterPhase {
    sim.combat.env().unwrap().shutter(0).unwrap().phase()
}

/// Both heroes just left of the shutter's arm line... right of `arm_x` 1008 and
/// within 320 px of the shutter, on the floor; both goblins removed.
fn armed_court(who: Who) -> PartyFixtureSim {
    let (mut sim, _ring) = start(&root(), RingRoom::Court, who, 0).unwrap();
    for g in &mut sim.combat.enemies {
        g.mode = GoblinMode::Corpse;
        g.health = 0;
    }
    sim.scene.world.hero.x = 1030.0;
    sim.scene.world.donut_body.x = 1000.0;
    sim
}

fn step(sim: &mut PartyFixtureSim) {
    sim.step_slots([RawInput::NONE, RawInput::NONE]);
}

fn intact_slices_in_world(sim: &PartyFixtureSim) -> Vec<bool> {
    let env = sim.combat.env().unwrap();
    env.solid_range()
        .map(|i| sim.scene.world.solids[i].intact)
        .collect()
}

/// The phase cycle of one closure: warning 40 ticks, the edge moves 2 px per
/// tick for 34 ticks, closed 150 ticks, opening 34 ticks, then open for good
/// (`cycles` 1). The slices follow the edge in the world on every tick: the top
/// `edge_px / 4` blocks stand and every block is Timber.
#[test]
fn the_shutter_cycles_and_its_slices_follow_the_edge() {
    let mut sim = armed_court(Who::CarlSolo);
    assert_eq!(phase(&sim), ShutterPhase::Open);
    let mut trace: Vec<(ShutterPhase, i32)> = Vec::new();
    for _ in 0..(WARNING + CLOSING + CLOSED + CLOSING + 120) {
        step(&mut sim);
        let env = sim.combat.env().unwrap();
        let sh = env.shutter(0).unwrap();
        let standing = intact_slices_in_world(&sim);
        let n = sh.intact_slices();
        assert_eq!(standing.len(), 17);
        assert!(
            standing.iter().enumerate().all(|(k, &s)| s == (k < n)),
            "tick {}: {standing:?} vs {n} intact",
            sim.scene.world.tick
        );
        for i in env.solid_range() {
            assert_eq!(sim.scene.world.solids[i].material, Material::Timber);
        }
        trace.push((sh.phase(), sh.edge_px()));
    }
    let count = |p: ShutterPhase| trace.iter().filter(|(q, _)| *q == p).count();
    // The arming step is the first warning step; the step that ends the warning
    // is already `Closing`, the edge first moves one step later (core docs).
    eprintln!(
        "phase ticks: warning {} closing {} closed {} opening {}",
        count(ShutterPhase::Warning),
        count(ShutterPhase::Closing),
        count(ShutterPhase::Closed),
        count(ShutterPhase::Opening)
    );
    assert_eq!(count(ShutterPhase::Warning), WARNING - 1);
    assert_eq!(count(ShutterPhase::Closing), CLOSING);
    assert_eq!(count(ShutterPhase::Closed), CLOSED);
    assert_eq!(count(ShutterPhase::Opening), CLOSING);
    let (last, edge) = *trace.last().unwrap();
    assert_eq!(
        (last, edge),
        (ShutterPhase::Open, 0),
        "once, then open for good"
    );
    let sh = sim.combat.env().unwrap().shutter(0).unwrap();
    assert_eq!(sh.closures(), 1);
    // The unheld blocked window (closing, closed, opening) is under the stuck
    // limit of 300 ticks (KR19): 34 + 150 + 34.
    const { assert!(CLOSING + CLOSED + CLOSING < 300) };
    // Fully closed: all 17 slices stand, from the floor up to the walkway slab.
    assert!(
        trace
            .iter()
            .any(|&(p, e)| p == ShutterPhase::Closed && e == 68)
    );
}

/// A trailing hero beyond `arm_trail_px` keeps the shutter open (KR18, RING-09:
/// the warning waits for the party); it arms the tick the trailing hero is near.
#[test]
fn a_trailing_hero_holds_the_warning_back() {
    let mut sim = armed_court(Who::CarlSolo);
    for _ in 0..60 {
        sim.scene.world.donut_body.x = 700.0; // 500 px from the shutter
        step(&mut sim);
        assert_eq!(phase(&sim), ShutterPhase::Open);
    }
    sim.scene.world.donut_body.x = 900.0; // 300 px: inside 320
    step(&mut sim);
    assert_eq!(phase(&sim), ShutterPhase::Warning);
}

/// KR4: a living goblin under the closing edge holds it and is pinned once
/// (staggered `pin_ticks` with no damage); a hero there holds it too and is
/// never hurt; the edge goes on the tick the body leaves.
#[test]
fn a_body_under_the_closing_edge_holds_it_and_a_goblin_is_pinned_once() {
    let mut sim = armed_court(Who::CarlSolo);
    // Goblin 0 stands in the arch, awake, and is kept there.
    {
        let g = &mut sim.combat.enemies[0];
        g.mode = GoblinMode::Neutral;
        g.health = 3;
        g.x = 1216.0;
    }
    let health = sim.combat.enemies[0].health;
    let mut pinned_at: Option<u64> = None;
    let mut pins = 0;
    let mut held_edge: Option<i32> = None;
    let mut released_at: Option<u64> = None;
    for _ in 0..400 {
        if released_at.is_none() {
            let g = &mut sim.combat.enemies[0];
            g.x = 1216.0;
            g.vx = 0.0;
        }
        let before = sim.combat.enemies[0].mode;
        step(&mut sim);
        let t = sim.scene.world.tick;
        let g = &sim.combat.enemies[0];
        if before != GoblinMode::Hurt && g.mode == GoblinMode::Hurt && released_at.is_none() {
            pins += 1;
            pinned_at.get_or_insert(t);
            assert_eq!(
                g.stagger_left,
                36 - 1 + 1,
                "pin_ticks, then one tick of stagger"
            );
        }
        let sh = sim.combat.env().unwrap().shutter(0).unwrap();
        // Held for well over the window: the hold has no time bound (KR4).
        if pinned_at.is_some_and(|p| t >= p + 150) && released_at.is_none() {
            assert_eq!(sh.phase(), ShutterPhase::Closing, "still held at tick {t}");
            held_edge = Some(sh.edge_px());
            released_at = Some(t);
            // The goblin leaves (it is no longer kept in the arch).
            sim.combat.enemies[0].x = 1500.0;
        }
        if released_at.is_some_and(|r| t > r + 80) {
            break;
        }
    }
    assert_eq!(pins, 1, "one pin token per enemy per closure, no re-pin");
    assert_eq!(sim.combat.enemies[0].health, health, "a pin does no damage");
    let edge = held_edge.expect("held");
    assert!(
        (16..68).contains(&edge),
        "the edge stopped above the goblin's head: {edge}"
    );
    let sh = sim.combat.env().unwrap().shutter(0).unwrap();
    assert!(
        matches!(sh.phase(), ShutterPhase::Closed | ShutterPhase::Opening),
        "the edge went on once the goblin left: {:?}",
        sh.phase()
    );
    assert_eq!(sim.combat.deaths, 0);
    let hurt = sim
        .combat
        .events
        .iter()
        .filter(|e| matches!(e, crate::combat::CombatEvent::HeroHurt { .. }))
        .count();
    assert_eq!(hurt, 0, "heroes are never hurt by the shutter");
}

/// A hero standing under the edge holds it above his head (never squeezed).
#[test]
fn a_hero_under_the_edge_holds_it_and_is_not_hurt() {
    let mut sim = armed_court(Who::CarlSolo);
    sim.scene.world.donut_body.x = 1000.0;
    for _ in 0..(WARNING + CLOSING + 60) {
        sim.scene.world.hero.x = 1216.0;
        step(&mut sim);
    }
    let sh = sim.combat.env().unwrap().shutter(0).unwrap();
    assert_eq!(sh.phase(), ShutterPhase::Closing);
    // Carl is 52 px tall, the arch 68: the edge is held within a slice of his head.
    assert!(sh.edge_px() <= 68 - 52, "{}", sh.edge_px());
    assert_eq!(sim.combat.deaths, 0);
    assert_eq!(
        sim.combat
            .events
            .iter()
            .filter(|e| matches!(e, crate::combat::CombatEvent::HeroHurt { .. }))
            .count(),
        0
    );
}

/// The traversal: the pilot (Carl solo, Donut solo, and Carl with the
/// autonomous partner) clears the court at every seed with the shutter cycling,
/// never crosses under the closed arch, and the partner survives the shutter
/// (KR19: stuck under the 300-tick limit).
#[test]
fn the_court_stays_traversable_with_the_moving_shutter() {
    for who in Who::ALL {
        for seed in SEEDS.take(4) {
            let r = run(&root(), RingRoom::Court, who, seed, false).unwrap();
            assert!(
                r.exit_tick.is_some() && r.deaths == 0,
                "{who:?} seed {seed}: {}",
                r.line()
            );
            assert!(!r.under_the_shutter, "{}", r.line());
            let (closures, _) = r.shutter.expect("the court has its shutter");
            assert_eq!(closures, 1, "{who:?} seed {seed}: the shutter closed once");
            assert!(
                r.max_stuck.iter().all(|&m| u64::from(m) < 300),
                "{who:?} seed {seed}: {}",
                r.line()
            );
        }
    }
}

/// Retry (RING-10): a restart returns the shutter to open with its cycle and
/// tokens spent anew, and no slice stands in the world; the second attempt runs
/// the same cycle as the first.
#[test]
fn a_restart_reopens_the_shutter_and_replays_the_same_cycle() {
    let mut sim = armed_court(Who::CarlSolo);
    let first: Vec<_> = (0..(WARNING + CLOSING + 20))
        .map(|_| {
            step(&mut sim);
            let sh = sim.combat.env().unwrap().shutter(0).unwrap();
            (sh.phase(), sh.edge_px())
        })
        .collect();
    assert_eq!(phase(&sim), ShutterPhase::Closed);
    let PartyFixtureSim { scene, combat, .. } = &mut sim;
    combat.restart(scene);
    let sh = sim.combat.env().unwrap().shutter(0).unwrap();
    assert_eq!(
        (sh.phase(), sh.edge_px(), sh.closures()),
        (ShutterPhase::Open, 0, 0)
    );
    assert!(intact_slices_in_world(&sim).iter().all(|&s| !s));
    // The same arming positions give the same cycle.
    sim.scene.world.hero.x = 1030.0;
    sim.scene.world.donut_body.x = 1000.0;
    for g in &mut sim.combat.enemies {
        g.mode = GoblinMode::Corpse;
        g.health = 0;
    }
    let tick0 = sim.scene.world.tick;
    let second: Vec<_> = (0..first.len())
        .map(|_| {
            step(&mut sim);
            let sh = sim.combat.env().unwrap().shutter(0).unwrap();
            (sh.phase(), sh.edge_px())
        })
        .collect();
    let _ = tick0;
    assert_eq!(first, second);
}

/// The nav graph's canonical dump without what a history leaves behind: the
/// revision counters (a fresh build starts at 0) and each edge's `as_built`
/// state (the state it was first built in, by design). Equal dumps mean equal
/// solids, anchors, node status and current edge states and launch columns.
fn nav_dump(graph: &crate::companion::nav::NavGraph) -> Vec<String> {
    graph
        .canonical_dump()
        .lines()
        .skip(1)
        .map(|l| {
            let mut out = String::new();
            let mut it = l.split(' ').peekable();
            while let Some(w) = it.next() {
                if w.starts_with("rev") && w[3..].chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                out.push_str(w);
                out.push(' ');
                let _ = it.peek();
            }
            if out.starts_with("edge ")
                && let Some(x) = out.rfind(" x")
                && let Some(g) = out[..x].rfind(" [")
            {
                out.replace_range(g..x, "");
            }
            out
        })
        .collect()
}

/// KR6: the nav after a full cycle by incremental `sync` equals a fresh build
/// of the same solids (revision counters aside), at every phase boundary and
/// every few ticks between.
#[test]
fn the_incremental_nav_after_a_full_cycle_equals_a_fresh_build() {
    use crate::companion::nav::{NavConfig, NavGraph, NavLevel};
    use crate::companion::navfollow::NavDriver;
    let mut sim = armed_court(Who::Companion);
    let cfg = NavConfig::agile();
    let mut driver = NavDriver::new(&sim.scene.world.solids, cfg).unwrap();
    let mut checked = 0;
    let mut closed_dump = None;
    for t in 0..(WARNING + CLOSING + CLOSED + CLOSING + 20) {
        step(&mut sim);
        driver.sync(&sim.scene.world.solids).unwrap();
        let p = phase(&sim);
        if t % 7 == 0 || (p == ShutterPhase::Closed && t % 40 == 0) {
            let fresh = NavGraph::build(
                &NavLevel::from_blocks(&sim.scene.world.solids, Vec::new()),
                cfg,
            )
            .unwrap();
            let (inc, fre) = (nav_dump(driver.graph()), nav_dump(&fresh));
            let first_diff = inc.iter().zip(&fre).find(|(a, b)| a != b);
            assert!(
                inc == fre,
                "tick {t} phase {p:?}: first difference {first_diff:?}"
            );
            checked += 1;
            if p == ShutterPhase::Closed {
                closed_dump.get_or_insert(fre);
            }
        }
    }
    assert!(checked > 30);
    // Open again at the end, and the closed state is a different graph.
    let open = NavGraph::build(
        &NavLevel::from_blocks(&sim.scene.world.solids, Vec::new()),
        cfg,
    )
    .unwrap();
    assert_eq!(nav_dump(driver.graph()), nav_dump(&open));
    assert_ne!(closed_dump.expect("a closed sample"), nav_dump(&open));
}

/// RING-FU held edge (KR4, KR19): a living goblin parked under the closing edge
/// holds the lane shut for Carl without a time bound, and Carl still crosses the
/// court over the walkway within the 300-tick stuck limit, as the pilot and with
/// the autonomous partner.
#[test]
fn a_held_edge_does_not_strand_carl_or_the_partner() {
    for who in [Who::CarlSolo, Who::Companion] {
        let (mut sim, ring) = start(&root(), RingRoom::Court, who, 0).unwrap();
        let route = crate::f01_ring_fixtures::Route::of(&ring, false);
        let mut closing_at: Option<u64> = None;
        let mut crossed_at: Option<u64> = None;
        for _ in 0..1500u64 {
            let tick = sim.scene.world.tick + 1;
            // Goblin 1 stands in the arch from the moment the shutter warns and
            // keeps standing there: it holds the edge for the whole run.
            if closing_at.is_some() && sim.combat.enemies[1].alive() {
                let g = &mut sim.combat.enemies[1];
                if g.mode == GoblinMode::Neutral || g.mode == GoblinMode::Hurt {
                    g.x = 1216.0;
                    g.vx = 0.0;
                }
            }
            let controlled = sim.party.controlled_hero();
            let input =
                crate::f01_ring_fixtures::pilot(&route, &sim.combat, &sim.scene, controlled);
            sim.step_slots([input, RawInput::NONE]);
            if closing_at.is_none() && phase(&sim) != ShutterPhase::Open {
                closing_at = Some(tick);
                // Wake and place it: it has no wake gate of its own here.
                let g = &mut sim.combat.enemies[1];
                g.mode = GoblinMode::Neutral;
                g.x = 1216.0;
            }
            if crossed_at.is_none() && sim.scene.world.body(HeroId::Carl).x > 1340.0 {
                crossed_at = Some(tick);
            }
            if crossed_at.is_some() {
                break;
            }
        }
        let (a, b) = (
            closing_at.expect("armed"),
            crossed_at.expect("Carl crossed"),
        );
        assert!(b - a < 300, "{who:?}: armed at {a}, crossed at {b}");
        assert_eq!(sim.combat.deaths, 0, "{who:?}");
    }
}

/// The pin moment as text evidence (`docs/evidence/ring-c-wire/pin_moment.txt`):
/// a goblin kept in the arch from the arming tick, one line per interesting
/// tick, run twice with the same hashes.
/// `cargo test ring_c_pin_trace -- --ignored --nocapture`
#[test]
#[ignore = "evidence generator"]
fn ring_c_pin_trace() {
    let trace = || -> Vec<String> {
        let mut sim = armed_court(Who::CarlSolo);
        {
            let g = &mut sim.combat.enemies[0];
            g.mode = GoblinMode::Neutral;
            g.health = 3;
            g.x = 1216.0;
        }
        let mut out = Vec::new();
        let mut last = String::new();
        for _ in 0..330 {
            if sim.scene.world.tick < 290 {
                let g = &mut sim.combat.enemies[0];
                g.x = 1216.0;
                g.vx = 0.0;
            }
            step(&mut sim);
            let t = sim.scene.world.tick;
            let g = &sim.combat.enemies[0];
            let sh = sim.combat.env().unwrap().shutter(0).unwrap();
            let key = format!("{:?} {:?} {}", sh.phase(), g.mode, sh.edge_px());
            let edge = sh.edge_px();
            if key != last || t.is_multiple_of(25) {
                out.push(format!(
                    "RING_PIN tick={t} phase={:?} edge_px={edge} intact_slices={} goblin={:?} stagger_left={} goblin_hp={} hash={:016x}",
                    sh.phase(),
                    sh.intact_slices(),
                    g.mode,
                    g.stagger_left,
                    g.health,
                    sim.state_hash()
                ));
            }
            last = key;
        }
        out
    };
    let (a, b) = (trace(), trace());
    assert_eq!(a, b, "two runs, the same trace");
    for l in &a {
        println!("{l}");
    }
    println!("RING_PIN_RUNS_IDENTICAL lines={}", a.len());
}
