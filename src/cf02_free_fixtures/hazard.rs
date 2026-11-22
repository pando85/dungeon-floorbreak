//! COMPANION-AI-02 CA2-C hazard fixtures (K13, CA2-Q29): the autonomous partner
//! in the real Goblin Boiler room, with its steam lane and its weight's
//! footprint live (the env step of BOIL-B and BOIL-C wire).
//!
//! These are **not** goldens: a golden is an arena fixture, and the Boiler needs
//! its env runtime ([`crate::combat_scene::EnvRuntime`]), which the registry's
//! `CombatState::new` does not build. Each test runs the whole room through the
//! real `step_slots`, reads the real hurt events as the oracle, and runs twice
//! where a hash is compared. The acid patch and the wheel have no scene producer
//! yet (HAZ-A2 and DOZER-CORE add one `NavRect` source each); the nav layer's
//! `every_hazard_class_is_just_a_rectangle_to_the_nav` covers every class of the
//! hazard core through the same feed.
//!
//! The setups move a hero with a direct write to the world (a teleport) only to
//! stage the situation; every tick after that is the simulation.

use super::FREE;
use crate::boss_scene::Pilot;
use crate::combat::CombatEvent;
use crate::combat_scene::{CombatState, EnemySpawn, Fixture};
use crate::companion::events::CompanionEvent;
use crate::companion::profile::ProfileSetId;
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::env_hazard::{NavPhase, NavRect};
use crate::f01_boiler;
use crate::input::RawInput;
use crate::party::{HeroId, PROVISIONAL_TUNING, PartyTuning, UncontrolledHero};
use crate::party_scene::{PartyFixture, PartyFixtureSim};
use crate::prototype::HERO_HALF_WIDTH;
use crate::scene_enemy::SlotSet;
use crate::scene_enemy::testing::Dummy;
use std::path::PathBuf;

const VALVE: u16 = 0;
const WEIGHT: u16 = 1;
/// Carl's jab column for each prop (the env tests' columns).
const VALVE_X: f32 = 460.0;
const BRACE_X: f32 = 740.0;
/// The steam lane (the sidecar's rectangle, layout px).
const STEAM_LEFT: f32 = 688.0;
const STEAM_RIGHT: f32 = 768.0;
/// The weight's footprint.
const FOOT_LEFT: f32 = 784.0;
const FOOT_RIGHT: f32 = 848.0;

const ATTACK: RawInput = RawInput {
    attack: true,
    ..RawInput::NONE
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The partner under the assist set, for the comparison run (it has no goal
/// layer and gets no hazard feed).
fn assist() -> PartyTuning {
    PartyTuning {
        uncontrolled_hero: UncontrolledHero::Companion(ProfileSetId::Cf02Assist),
        ..PROVISIONAL_TUNING
    }
    .with_cf02_swap()
}

fn rig_with(tuning: PartyTuning, slot: Vec<Dummy>) -> PartyFixtureSim {
    let run = f01_boiler::launch(&root()).expect("the Boiler launches");
    let base: &'static Fixture = Box::leak(Box::new(Fixture {
        name: "cf02_free_boiler_hazards",
        summary: "CA2-C: the autonomous partner in the Boiler room",
        seed: 0xCF02_FE10,
        carl_x: 112,
        enemies: &[] as &[EnemySpawn],
        shore_dx: None,
        boss: None,
        engineers: &[],
        llamas: &[],
        charges: &[],
        script: &[],
        ticks: 900,
    }));
    let mut scene = SceneSim::new_at(&run.kit, 360, base.carl_x).unwrap();
    let mut combat = CombatState::new(base, &run.kit, &scene)
        .unwrap()
        .with_donut(DonutSetup::unlocked(8))
        .with_demo(run.rules)
        .with_cf02()
        .with_env(f01_boiler::env_runtime(&run).unwrap());
    if !slot.is_empty() {
        let set = SlotSet::new(slot.into_iter().map(Dummy::boxed).collect(), false).unwrap();
        combat = combat.with_slot_enemies(set).unwrap();
    }
    combat.sync_env(&mut scene);
    let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
        base,
        two_player: false,
        tuning,
        script: &[],
        pilot: Pilot::None,
        donut: DonutSetup::unlocked(8),
    }));
    PartyFixtureSim::from_parts(scene, combat, fixture)
}

fn rig() -> PartyFixtureSim {
    rig_with(FREE, Vec::new())
}

fn step(s: &mut PartyFixtureSim) {
    s.step_slots([RawInput::NONE, RawInput::NONE]);
}

fn tick(s: &PartyFixtureSim) -> u64 {
    s.scene.world.tick
}

fn put(s: &mut PartyFixtureSim, hero: HeroId, x: f32) {
    let body = s.scene.world.body_mut(hero);
    body.x = x;
    body.vx = 0.0;
}

fn feed(s: &PartyFixtureSim) -> Vec<NavRect> {
    s.combat.env().map(|e| e.nav_feed()).unwrap_or_default()
}

/// Two jabs 30 ticks apart on a prop's target from column `x`, until the prop's
/// first phase event (`Warning` of the valve, `Released` of the weight).
/// Returns the trigger tick `T` (the sim stands on tick `T`).
fn trigger(s: &mut PartyFixtureSim, x: f32, prop: u16) -> u64 {
    put(s, HeroId::Carl, x);
    s.scene.world.hero.facing = 1;
    let from = s.combat.events.len();
    for t in 0..200 {
        let press = t % 30 == 0 && t / 30 < 2;
        s.step_slots([if press { ATTACK } else { RawInput::NONE }, RawInput::NONE]);
        if let Some(at) = s.combat.events[from..].iter().find_map(|e| match *e {
            CombatEvent::PropPhase { tick, prop: p, .. } if p == prop => Some(tick),
            _ => None,
        }) {
            return at;
        }
    }
    panic!("two jabs never set prop {prop} off");
}

fn donut_hurts(s: &PartyFixtureSim) -> Vec<u64> {
    s.combat
        .events
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::HeroHurt {
                tick,
                hero: HeroId::Donut,
                ..
            } => Some(tick),
            _ => None,
        })
        .collect()
}

fn donut_x(s: &PartyFixtureSim) -> f32 {
    s.scene.world.donut_body.x
}

/// Whether her body is over a rectangle `[left, right]` of the feed (x only: the
/// Boiler's lanes sit on its floor, she walks on it).
fn over(x: f32, left: f32, right: f32) -> bool {
    x + HERO_HALF_WIDTH > left && x - HERO_HALF_WIDTH < right
}

/// What one run of the steam scenario recorded.
#[derive(Debug, Clone, PartialEq)]
struct SteamRun {
    trigger: u64,
    /// `(tick, donut x, feed rects, carl x)` for every tick after the trigger.
    trace: Vec<(u64, f32, usize, f32)>,
    hurts: Vec<u64>,
    /// First tick her body was over the lane while it was in the feed.
    entered: Option<u64>,
    /// The closest her body edge came to the lane's left edge in the feed window.
    closest: f32,
    /// The last tick the feed held the lane.
    cleared: u64,
    end_x: f32,
    carl_x: f32,
    hash: u64,
    feed_changes: u32,
}

/// Carl stands beyond the lane (x 800); she starts 90 px short of it. The valve
/// is set off, the lane is telegraphed (24 ticks) then active (72), and she wants
/// to be with Carl on the other side. Once it is spent Carl steps on to x 900.
fn steam(tuning: PartyTuning) -> SteamRun {
    let mut s = rig_with(tuning, Vec::new());
    let t = trigger(&mut s, VALVE_X, VALVE);
    put(&mut s, HeroId::Carl, 800.0);
    put(&mut s, HeroId::Donut, 600.0);
    let mut run = SteamRun {
        trigger: t,
        trace: Vec::new(),
        hurts: Vec::new(),
        entered: None,
        closest: f32::MAX,
        cleared: 0,
        end_x: 0.0,
        carl_x: 0.0,
        hash: 0,
        feed_changes: 0,
    };
    let mut moved_on = false;
    for _ in 0..420 {
        step(&mut s);
        let rects = feed(&s).len();
        let x = donut_x(&s);
        if rects > 0 {
            run.cleared = tick(&s);
            run.closest = run.closest.min(STEAM_LEFT - (x + HERO_HALF_WIDTH));
            if run.entered.is_none() && over(x, STEAM_LEFT, STEAM_RIGHT) {
                run.entered = Some(tick(&s));
            }
        } else if tick(&s) > t + 100 && !moved_on {
            moved_on = true;
            put(&mut s, HeroId::Carl, 900.0);
        }
        run.trace.push((tick(&s), x, rects, s.scene.world.hero.x));
    }
    run.hurts = donut_hurts(&s);
    run.end_x = donut_x(&s);
    run.carl_x = s.scene.world.hero.x;
    run.hash = s.state_hash();
    run.feed_changes = s.combat.companion_stats().hazard_feed_changes;
    run
}

#[test]
fn steam_she_holds_short_of_the_lane_while_it_is_telegraphed_and_active_and_crosses_when_it_clears()
{
    let r = steam(FREE);
    let windows: Vec<u64> = r
        .trace
        .iter()
        .filter(|(_, _, n, _)| *n > 0)
        .map(|(t, ..)| *t)
        .collect();
    // The lane is in the feed from its trigger to the tick it is spent: the 24
    // tick telegraph and the 72 tick burst (T + 0 ..= T + 94, spent on T + 95).
    assert_eq!(windows.first(), Some(&(r.trigger + 1)));
    assert_eq!(windows.last(), Some(&(r.trigger + 94)), "{windows:?}");
    assert_eq!(
        r.entered, None,
        "she never enters the lane while it is live"
    );
    assert!(r.hurts.is_empty(), "no steam hurt: {:?}", r.hurts);
    // She did go for it: she came within a step or two of its edge and waited.
    assert!(
        r.closest >= 0.0 && r.closest < 40.0,
        "she closed on the lane without entering it: {}",
        r.closest
    );
    // The telegraph alone already keeps her out: at the first active tick she is
    // not over it and neither is she over it in the 24 ticks before.
    let active_from = r.trigger + 24;
    assert!(
        r.trace
            .iter()
            .filter(|(t, ..)| *t <= active_from)
            .all(|(_, x, ..)| !over(*x, STEAM_LEFT, STEAM_RIGHT))
    );
    // Once it is spent she reaches her goal across it: beside Carl at x 900.
    assert!(r.end_x > STEAM_RIGHT + HERO_HALF_WIDTH, "{}", r.end_x);
    assert!(
        (r.carl_x - r.end_x).abs() < 160.0,
        "with Carl: she {} he {}",
        r.end_x,
        r.carl_x
    );
    // The graph was rebuilt twice for the lane: it came in and it went out.
    assert_eq!(r.feed_changes, 2, "{r:?}");
}

#[test]
fn steam_the_assist_partner_without_the_feed_walks_into_it_so_the_fixture_guards_the_case() {
    let r = steam(assist());
    assert!(
        r.entered.is_some() || !r.hurts.is_empty(),
        "the same run under cf02_assist (no hazard feed) enters the lane: {:?}",
        r.entered
    );
}

#[test]
fn steam_is_deterministic_and_the_feed_is_the_only_thing_that_differs_from_a_run_without_it() {
    let a = steam(FREE);
    let b = steam(FREE);
    assert_eq!(a, b, "two runs of the same inputs");
    assert_eq!(a.hash, b.hash);
}

/// She stands inside the lane when the valve is set off: the telegraph gives her
/// 24 ticks, and she walks out before the first active tick.
#[test]
fn steam_standing_in_the_lane_when_it_is_telegraphed_she_leaves_before_it_is_active() {
    let mut s = rig();
    let t = trigger(&mut s, VALVE_X, VALVE);
    put(&mut s, HeroId::Donut, 730.0);
    let mut out_at = None;
    for _ in 0..140 {
        step(&mut s);
        if out_at.is_none() && !over(donut_x(&s), STEAM_LEFT, STEAM_RIGHT) {
            out_at = Some(tick(&s));
        }
    }
    let out_at = out_at.expect("she left the lane");
    assert!(
        out_at < t + 23,
        "out on tick {out_at}, the burst opens on {}",
        t + 23
    );
    assert!(donut_hurts(&s).is_empty(), "{:?}", donut_hurts(&s));
}

/// The weight's footprint: a thin telegraph on the floor for 24 ticks, then one
/// evaluation of the whole column. She stands in it when it is released.
#[test]
fn footprint_she_leaves_the_floor_shadow_before_the_weight_lands() {
    let mut s = rig();
    let t = trigger(&mut s, BRACE_X, WEIGHT);
    put(&mut s, HeroId::Donut, 816.0);
    let mut out_at = None;
    let mut shadow_ticks = 0;
    for _ in 0..120 {
        step(&mut s);
        if feed(&s).iter().any(|n| n.phase == NavPhase::Telegraph) {
            shadow_ticks += 1;
        }
        if out_at.is_none() && !over(donut_x(&s), FOOT_LEFT, FOOT_RIGHT) {
            out_at = Some(tick(&s));
        }
    }
    assert!(
        shadow_ticks >= 20,
        "the shadow was in the feed: {shadow_ticks}"
    );
    let out_at = out_at.expect("she left the footprint");
    assert!(out_at < t + 24, "out on {out_at}, T = {t}");
    assert!(donut_hurts(&s).is_empty(), "{:?}", donut_hurts(&s));
    // The landing leaves a remnant solid and the graph still builds with it.
    assert!(s.combat.companion_stats().hazard_feed_changes >= 2);
}

/// She would stand under the footprint as her escort spot: with Carl across it
/// she holds back, and she is not under it when it lands.
#[test]
fn footprint_she_does_not_wait_under_it_with_carl_beyond() {
    let mut s = rig();
    let t = trigger(&mut s, BRACE_X, WEIGHT);
    put(&mut s, HeroId::Carl, 900.0);
    put(&mut s, HeroId::Donut, 700.0);
    for _ in 0..90 {
        step(&mut s);
        if feed(&s).iter().any(|n| n.hazard == 2) {
            assert!(
                !over(donut_x(&s), FOOT_LEFT, FOOT_RIGHT),
                "tick {} (T = {t}): x {}",
                tick(&s),
                donut_x(&s)
            );
        }
    }
    assert!(donut_hurts(&s).is_empty());
}

/// Only the set with the goal layer is fed: under the assist set no event, no
/// rebuild, the graph is the hazard-free one it always was.
#[test]
fn only_the_autonomous_partners_set_is_fed() {
    let mut s = rig_with(assist(), Vec::new());
    trigger(&mut s, VALVE_X, VALVE);
    for _ in 0..40 {
        step(&mut s);
    }
    assert_eq!(s.combat.companion_stats().hazard_feed_changes, 0);
    let events = s.combat.take_companion_events();
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, CompanionEvent::HazardFeed { .. }))
    );
    let mut f = rig();
    trigger(&mut f, VALVE_X, VALVE);
    for _ in 0..40 {
        step(&mut f);
    }
    assert!(
        f.combat
            .take_companion_events()
            .iter()
            .any(|e| matches!(e, CompanionEvent::HazardFeed { rects: 1, .. }))
    );
}

/// K5: the charge reaches her as the existing `BossThreat::Charge` through the
/// view's boss and the follow layer's dodge (no hazard rectangle, no per-boss
/// code). A supplier whose ram is about to arrive makes her jump; the same room
/// without the threat does not.
#[test]
fn a_boss_charge_reaches_the_autonomous_partner_through_the_threat_feed() {
    use crate::boss::BossThreat;
    let jumps = |threat: Option<BossThreat>| {
        let mut d = Dummy::new(crate::scene_enemy::SLOT_ID_MIN, 300.0, 0.0);
        d.supplies = true;
        d.threat = threat;
        let mut s = rig_with(FREE, vec![d]);
        put(&mut s, HeroId::Carl, 260.0);
        put(&mut s, HeroId::Donut, 230.0);
        let mut air = false;
        for _ in 0..8 {
            step(&mut s);
            air |= !s.scene.world.donut_body.grounded;
        }
        air
    };
    // Ram at x 80 heading right: its front is at 162, she stands 68 px ahead of it,
    // inside the 18 tick lead.
    let charge = BossThreat::Charge {
        tell_left: 0,
        x: 80.0,
        dir: 1,
        running: true,
        front: 82.0,
        depth: 46.0,
        speed: 5.0,
        lead_ticks: 18,
    };
    assert!(jumps(Some(charge)), "she jumps the ram");
    assert!(!jumps(None), "and not without it");
}

/// Prints the two steam traces as CSV for the evidence sheet:
/// `cargo test --lib print_evidence_trace -- --ignored --nocapture`.
#[test]
#[ignore = "prints the evidence sheet's data"]
fn print_evidence_trace() {
    for (name, tuning) in [("free", FREE), ("assist", assist())] {
        let r = steam(tuning);
        for (t, x, n, carl) in &r.trace {
            println!("TRACE,{name},{},{t},{x:.1},{n},{carl:.1}", r.trigger);
        }
        println!(
            "SUMMARY,{name},trigger {},entered {:?},hurts {:?},closest {:.1},end_x {:.1}",
            r.trigger, r.entered, r.hurts, r.closest, r.end_x
        );
    }
}

// ------------------------------------------------------------ acid (HAZ-A2)

mod acid {
    use super::*;
    use crate::hazards_fixtures::{Layout, Rig, SlimeAt};
    use crate::scene_enemy::{EnvSource, SlotEnvHit};

    /// The acid patch of the pocket fixtures (layout px).
    const POOL: (i32, i32) = (700, 764);

    fn rig(link: Option<u8>) -> Rig {
        Layout {
            acid: vec![(POOL.0, POOL.1, link)],
            slimes: if link.is_some() {
                vec![SlimeAt {
                    x: POOL.0 + 32,
                    facing: -1,
                    home: POOL.0 + 32,
                }]
            } else {
                Vec::new()
            },
            carl_x: 830,
            tuning: Some(FREE),
            ..Layout::default()
        }
        .sim(Some(false))
    }

    fn kill_slime(rig: &mut Rig) {
        let hit = SlotEnvHit {
            victim: crate::combat_scene::slime::target_id(0),
            source: EnvSource::Prop(0),
            class: crate::env_hazard::HazardClass::FallingMass,
            damage: 9,
            push_x: 0,
            initiator: Some(0),
            activation: 1,
        };
        let tick = rig.tick();
        let (scene, combat) = rig.parts();
        combat.slot_env_hits(scene, &[hit], tick);
    }

    struct Run {
        acid_hurts: usize,
        any_hurts: usize,
        entered_live: bool,
        feed_changes: u32,
        feed_errors: u32,
        reached: Option<u64>,
        end_x: f32,
        hash: u64,
    }

    /// Carl stands beyond the patch (x 830), she starts short of it (x 640).
    fn run(link: Option<u8>, ticks: u64) -> Run {
        let mut r = rig(link);
        r.place(HeroId::Donut, 640.0);
        let mut out = Run {
            acid_hurts: 0,
            any_hurts: 0,
            entered_live: false,
            feed_changes: 0,
            feed_errors: 0,
            reached: None,
            end_x: 0.0,
            hash: 0,
        };
        for t in 1..=ticks {
            r.step(RawInput::NONE, RawInput::NONE);
            if link.is_some() && t == 2 {
                kill_slime(&mut r);
            }
            if t == 70 {
                // The drain is over (60 ticks): Carl steps on to x 1040: she is outside her free zone, so
                // she must get across the patch's ground to him (Escort wins at once).
                r.place(HeroId::Carl, 1040.0);
            }
            let x = r.x_of(HeroId::Donut);
            let live = r
                .combat()
                .env()
                .and_then(|e| e.acid().get(1))
                .and_then(|p| p.bounds());
            if let Some((a, b)) = live
                && over(x, a as f32, b as f32)
            {
                out.entered_live = true;
            }
            if out.reached.is_none() && (r.x_of(HeroId::Carl) - x).abs() < 120.0 {
                out.reached = Some(t);
            }
        }
        let hurts = r.hurts(HeroId::Donut);
        out.any_hurts = hurts.len();
        out.acid_hurts = hurts
            .iter()
            .filter(|h| h.1 == crate::slime::acid_source(1))
            .count();
        let stats = r.combat().companion_stats();
        out.feed_changes = stats.hazard_feed_changes;
        out.feed_errors = stats.hazard_feed_errors;
        out.end_x = r.x_of(HeroId::Donut);
        out.hash = r.hash();
        out
    }

    /// A persistent patch (no slime links it): the acid reaches her feed and she
    /// is not hurt by it. **Known limit, pinned so it cannot change unseen:** with
    /// Carl beyond and no other route she holds short of the patch for good. The
    /// graph offers no jump over it: a takeoff beside a hazard has the body over it
    /// while it is still low (`nav.rs` `transit` counts those ticks as hazard
    /// ticks), and the takeoff is not set back by the hazard's height. The pockets'
    /// patches are all linked to a slime and drain when it falls (the next test);
    /// a takeoff clearance in `nav.rs` is a design note for the top tier.
    #[test]
    fn a_persistent_patch_reaches_her_feed_she_is_not_hurt_and_the_stall_is_pinned() {
        let r = run(None, 600);
        assert!(r.feed_changes >= 1, "the acid reached the feed");
        assert_eq!(r.feed_errors, 0);
        assert_eq!(r.acid_hurts, 0, "no acid hurt");
        assert_eq!(r.any_hurts, 0);
        assert!(!r.entered_live, "never inside the live patch");
        assert_eq!(
            r.reached, None,
            "known limit: no safe way over a live patch"
        );
        assert!(r.end_x + 12.0 <= POOL.0 as f32, "held short: {}", r.end_x);
    }

    /// The drain: the slime falls on tick 2, the patch shrinks over 60 ticks and is
    /// removed on the 61st. She is not hurt, never enters the live patch and does
    /// not stall: once it is gone and Carl steps on, she crosses to him. The feed
    /// rounds outward to 8 px (CA2-Q29), so the graph is rebuilt 7 times across the
    /// drain (6 for the pool, one for the slime's row leaving the feed), not about 48 (the rectangle moves about 1 px every 1.25 ticks).
    #[test]
    fn a_draining_patch_is_avoided_and_costs_a_handful_of_rebuilds() {
        let r = run(Some(0), 400);
        assert_eq!(r.acid_hurts, 0);
        assert_eq!(r.any_hurts, 0, "the slime is dead; she takes no hurt");
        assert!(!r.entered_live, "never inside the live patch");
        assert_eq!(r.feed_errors, 0);
        assert_eq!(r.feed_changes, REBUILDS, "rebuilds across one drain");
        let at = r.reached.expect("she reached Carl");
        assert!(at <= 300, "reached Carl on tick {at}, end x {}", r.end_x);
        let again = run(Some(0), 400);
        assert_eq!((again.hash, again.feed_changes), (r.hash, r.feed_changes));
    }

    /// HAZ-B re-pin, 6 to 7: the slime's contact box is a feed row now (spec section
    /// 13, item 4), so the feed also changes when the slime that dies on tick 2
    /// leaves it (one more list change, not one more per pixel of drain).
    const REBUILDS: u32 = 7;
}
