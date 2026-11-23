//! DONUT-PROG-01 DP-F fixtures: encounter completion with deterministic pilots
//! (spec section 10, DP-Q47). AUTOPLAY cannot run these scenes (`scenario.rs`
//! rejects a boss, shore or charges, and its observation masks `special`), so the
//! acceptance is the scripted pilots of the shipped fixtures overlaid with Donut
//! pins, in the `with_tuning` / `DonutSetup` pattern of `donut_fixtures.rs`:
//!
//! - `donut_prog_pilot_arena_boost`: the solo arena pilots `cf02_arena_direct`,
//!   `cf02_arena_swap` and `eng_solo_assist` (the companion drives Donut) at
//!   seeds 0..8, with the biscuit and `jump_reach_boost` held.
//! - `donut_prog_pilot_arena_blink`: the new [`Pilot::ArenaDuoBlink`] (the P2
//!   Donut pilot presses the blink) on the `cf02_arena_coop` room, seeds 0..8,
//!   with all three Donut capabilities held.
//! - `donut_prog_pilot_boss_boost`: `boss01_full_clear_solo` and
//!   `boss01_full_clear_2p` under the `--donut-prog` boss pin, at their
//!   committed seeds; the run ends with the Puddle Jumper grant on the defeat tick.
//!
//! Harness only, in the shape of DP-C's and DP-E's fixtures: no CLI name, no
//! committed replay and no change to any original fixture (every overlay is a
//! new [`PartyFixture`] value). Each run is replayed twice by its test, and the
//! per-run report (deaths, restarts, blinks, cancels, fizzles, mana spent, dry
//! presses, the longest standstill of each hero) is printed with `--nocapture`;
//! `DP_F_PILOT_OUT=<file>` writes it for the evidence README.

use crate::boss_scene::{BOSS01_FULL_CLEAR_2P, BOSS01_FULL_CLEAR_SOLO, Pilot};
use crate::carl_prog::CarlProgFlags;
use crate::cf02::arena;
use crate::cf02_fixtures::{ENG_SOLO_ASSIST, find_party_fixture};
use crate::combat::CombatEvent;
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::donut_mana::{DonutSetup, ManaEvent};
use crate::donut_prog::{DonutProgFlags, story_setup as program_setup};
use crate::party_scene::{PartyFixture, PartyFixtureSim};
use crate::story_unlock::{SceneKind, StorySetup};
use std::path::PathBuf;

/// The seeds of the arena pilots (DP-Q47: seeds 0..8).
const SEEDS: u64 = 8;
/// No hero may stand still this long inside a run that is not over (the spec's
/// softlock bound).
const STUCK_LIMIT: u64 = 300;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn flags(light_feet: bool, puddle: bool) -> DonutProgFlags {
    DonutProgFlags {
        light_feet,
        puddle,
        ..DonutProgFlags::OFF
    }
}

/// The arena story with the boost held (`--donut-light-feet`) or, with `puddle`,
/// the blink as well (`--donut-puddle`).
fn arena_story(puddle: bool) -> StorySetup {
    program_setup(
        CarlProgFlags::OFF,
        flags(!puddle, puddle),
        SceneKind::CfArena,
    )
}

fn overlay(f: &PartyFixture, pilot: Pilot) -> &'static PartyFixture {
    Box::leak(Box::new(PartyFixture {
        base: f.base,
        two_player: f.two_player,
        tuning: f.tuning,
        script: f.script,
        pilot,
        donut: DonutSetup::unlocked(8),
    }))
}

/// What a run reports (DP-Q47).
#[derive(Debug, Default, Clone, PartialEq)]
struct Report {
    name: String,
    seed: Option<u64>,
    /// First tick the encounter was over (arena: every enemy defeated; boss: the
    /// Juicer defeated).
    done_at: Option<u64>,
    ticks: u64,
    deaths: u32,
    restarts: u32,
    blinks: u32,
    started: u32,
    cancels: u32,
    fizzles: u32,
    /// `ManaSpent` events on a blink's arrival tick, and every other one (the
    /// missiles).
    blink_spends: u32,
    missile_spends: u32,
    /// A missile cast refused for no mana, plus a blink press rejected.
    dry_presses: u32,
    /// The longest run of ticks each hero's position did not change.
    stuck: [u64; 2],
    hash: u64,
}

fn report(sim: &PartyFixtureSim, mut r: Report) -> Report {
    r.deaths = sim.combat.deaths;
    let mut arrivals = Vec::new();
    for e in &sim.combat.events {
        match *e {
            CombatEvent::Restart { .. } => r.restarts += 1,
            CombatEvent::SpecialStarted { hero: 1, .. } => r.started += 1,
            CombatEvent::BlinkResolved { tick, .. } => {
                r.blinks += 1;
                arrivals.push(tick);
            }
            CombatEvent::BlinkCancelled { .. } => r.cancels += 1,
            CombatEvent::BlinkFizzled { .. } => r.fizzles += 1,
            CombatEvent::SpecialRejected { hero: 1, .. }
            | CombatEvent::Mana(ManaEvent::CastRejected { .. }) => r.dry_presses += 1,
            _ => {}
        }
    }
    for e in &sim.combat.events {
        if let CombatEvent::Mana(ManaEvent::ManaSpent { tick, .. }) = *e {
            if arrivals.contains(&tick) {
                r.blink_spends += 1;
            } else {
                r.missile_spends += 1;
            }
        }
    }
    r.hash = sim.state_hash();
    r
}

/// Step `sim` for the fixture's ticks, tracking the standstill of each hero while
/// the encounter is not over, and the first tick it is (`over`).
fn play(
    mut sim: PartyFixtureSim,
    mut r: Report,
    over: impl Fn(&PartyFixtureSim) -> bool,
) -> (PartyFixtureSim, Report) {
    let ticks = sim.combat.fixture.ticks;
    let mut last = [(f32::NAN, f32::NAN); 2];
    let mut run = [0u64; 2];
    for _ in 0..ticks {
        sim.step_scripted();
        if r.done_at.is_none() {
            if over(&sim) {
                r.done_at = Some(sim.scene.world.tick);
            }
            let now = [
                (sim.scene.world.hero.x, sim.scene.world.hero.y),
                (sim.scene.world.donut_body.x, sim.scene.world.donut_body.y),
            ];
            for i in 0..2 {
                if now[i] == last[i] {
                    run[i] += 1;
                } else {
                    run[i] = 0;
                }
                last[i] = now[i];
                r.stuck[i] = r.stuck[i].max(run[i]);
            }
        }
    }
    r.ticks = sim.scene.world.tick;
    let r = report(&sim, r);
    (sim, r)
}

/// An arena run of `fixture` at `seed` (the `agility_reflex_tests` seeding).
fn arena_sim(fixture: &'static PartyFixture, seed: u64, story: StorySetup) -> PartyFixtureSim {
    let kit = arena::load_arena(&root()).unwrap();
    let mut scene = SceneSim::new_at(&kit, 360, fixture.base.carl_x).unwrap();
    fixture.place_donut(&mut scene);
    let combat = CombatState::new(fixture.base, &kit, &scene)
        .unwrap()
        .with_donut(fixture.donut)
        .with_story(story)
        .with_cf02_seed(Some(seed));
    PartyFixtureSim::from_parts(scene, combat, fixture)
}

fn arena_run(fixture: &'static PartyFixture, seed: u64, puddle: bool) -> (PartyFixtureSim, Report) {
    let r = Report {
        name: fixture.name().to_owned(),
        seed: Some(seed),
        ..Report::default()
    };
    play(arena_sim(fixture, seed, arena_story(puddle)), r, |s| {
        arena::complete(&s.combat)
    })
}

fn boss_run(base: &'static PartyFixture) -> (PartyFixtureSim, Report) {
    let fixture = overlay(base, base.pilot);
    let kit = crate::boss_scene::load_arena(&root()).unwrap();
    let scene = SceneSim::new_at(&kit, 360, fixture.base.carl_x).unwrap();
    let combat = CombatState::new(fixture.base, &kit, &scene)
        .unwrap()
        .with_donut(fixture.donut)
        .with_story(program_setup(
            CarlProgFlags::OFF,
            DonutProgFlags {
                prog: true,
                ..DonutProgFlags::OFF
            },
            SceneKind::Boss,
        ));
    let sim = PartyFixtureSim::from_parts(scene, combat, fixture);
    let r = Report {
        name: fixture.name().to_owned(),
        seed: None,
        ..Report::default()
    };
    play(sim, r, |s| {
        s.combat
            .events
            .iter()
            .any(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
    })
}

fn line(r: &Report) -> String {
    format!(
        "{} seed={} done_at={} ticks={} deaths={} restarts={} blinks={} started={} cancels={} \
         fizzles={} blink_spends={} missile_spends={} dry={} stuck={:?} hash={:016x}",
        r.name,
        r.seed.map_or("committed".to_owned(), |s| s.to_string()),
        r.done_at.map_or("never".to_owned(), |t| t.to_string()),
        r.ticks,
        r.deaths,
        r.restarts,
        r.blinks,
        r.started,
        r.cancels,
        r.fizzles,
        r.blink_spends,
        r.missile_spends,
        r.dry_presses,
        r.stuck,
        r.hash
    )
}

fn emit(out: &mut String, r: &Report) {
    out.push_str(&line(r));
    out.push('\n');
}

fn arena_pilots() -> [&'static PartyFixture; 3] {
    [
        overlay(
            find_party_fixture("cf02_arena_direct").unwrap(),
            Pilot::ArenaDirect,
        ),
        overlay(
            find_party_fixture("cf02_arena_swap").unwrap(),
            find_party_fixture("cf02_arena_swap").unwrap().pilot,
        ),
        overlay(&ENG_SOLO_ASSIST, ENG_SOLO_ASSIST.pilot),
    ]
}

/// FA-Q19 (T26): the coop blink rows isolate the blink from the cat reflex, so
/// both the pilot and its control run the profile without the armed attempt
/// start (with it, the armed reflex evades the swings the blink rule answers and
/// some seeds never press).
fn unarmed(f: &'static PartyFixture) -> &'static PartyFixture {
    Box::leak(Box::new(PartyFixture {
        tuning: crate::party::PartyTuning {
            donut_reflex: crate::agility::ReflexProfile::ShippedNoOpener,
            ..f.tuning
        },
        base: f.base,
        two_player: f.two_player,
        script: f.script,
        pilot: f.pilot,
        donut: f.donut,
    }))
}

fn blink_pilot() -> &'static PartyFixture {
    unarmed(overlay(
        find_party_fixture("cf02_arena_coop").unwrap(),
        Pilot::ArenaDuoBlink,
    ))
}

/// The run of the Juicer's full clear with the AI Donut under `cf02_free` holding
/// the blink (the boss arena, DP-F's "boss-arena duo blink").
fn boss_free_run() -> (PartyFixtureSim, Report) {
    let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
        base: BOSS01_FULL_CLEAR_SOLO.base,
        two_player: false,
        tuning: crate::cf02_free_fixtures::FREE,
        script: BOSS01_FULL_CLEAR_SOLO.script,
        pilot: BOSS01_FULL_CLEAR_SOLO.pilot,
        donut: DonutSetup::unlocked(8),
    }));
    let kit = crate::boss_scene::load_arena(&root()).unwrap();
    let scene = SceneSim::new_at(&kit, 360, fixture.base.carl_x).unwrap();
    let combat = CombatState::new(fixture.base, &kit, &scene)
        .unwrap()
        .with_donut(fixture.donut)
        .with_story(program_setup(
            CarlProgFlags::OFF,
            DonutProgFlags {
                prog: true,
                puddle: true,
                ..DonutProgFlags::OFF
            },
            SceneKind::Boss,
        ));
    let sim = PartyFixtureSim::from_parts(scene, combat, fixture);
    let r = Report {
        name: "cf02_free_blink_boss_arena".to_owned(),
        ..Report::default()
    };
    play(sim, r, boss_defeated)
}

fn boss_defeated(s: &PartyFixtureSim) -> bool {
    s.combat
        .events
        .iter()
        .any(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
}

/// FNV-1a over the run hashes, in order: one pin per fixture group.
fn fold(reports: &[Report]) -> u64 {
    reports.iter().fold(0xcbf2_9ce4_8422_2325, |acc, r| {
        (acc ^ r.hash).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn arena_boost_reports() -> Vec<Report> {
    let mut out = Vec::new();
    for fixture in arena_pilots() {
        for seed in 0..SEEDS {
            out.push(arena_run(fixture, seed, false).1);
        }
    }
    out
}

fn arena_blink_reports() -> Vec<Report> {
    (0..SEEDS)
        .map(|seed| arena_run(blink_pilot(), seed, true).1)
        .collect()
}

/// The control of the blink pilot: the same room, seeds, capabilities **and
/// stance** (a step ahead of Carl) with no blink rule and nothing withheld
/// (`Pilot::ArenaDuoAhead`), so the two differ by the blink alone.
fn arena_blink_control() -> Vec<Report> {
    let control = unarmed(overlay(
        find_party_fixture("cf02_arena_coop").unwrap(),
        Pilot::ArenaDuoAhead,
    ));
    (0..SEEDS)
        .map(|seed| arena_run(control, seed, true).1)
        .collect()
}

/// The shipped trailing pilot (`Pilot::ArenaDuo`, 40 px behind Carl) with the
/// same capabilities, recorded for reference only: it differs from the blink
/// pilot in stance as well as in the blink.
fn arena_blink_trailing() -> Vec<Report> {
    let control = overlay(
        find_party_fixture("cf02_arena_coop").unwrap(),
        Pilot::ArenaDuo,
    );
    (0..SEEDS)
        .map(|seed| arena_run(control, seed, true).1)
        .collect()
}

fn boss_reports() -> Vec<Report> {
    [&BOSS01_FULL_CLEAR_SOLO, &BOSS01_FULL_CLEAR_2P]
        .into_iter()
        .map(|base| boss_run(base).1)
        .collect()
}

/// Every pilot report, for `--nocapture` and `DP_F_PILOT_OUT`.
#[test]
fn print_the_pilot_reports() {
    let mut out = String::from(
        "DP-F pilot reports (spec 10, DP-Q47); regenerate: \
         DP_F_PILOT_OUT=<file> cargo test --locked --lib print_the_pilot_reports -- --nocapture\n",
    );
    out.push_str("# donut_prog_pilot_arena_boost (boost held)\n");
    for r in arena_boost_reports() {
        emit(&mut out, &r);
    }
    out.push_str("# donut_prog_pilot_arena_blink (ArenaDuoBlink, boost, biscuit and blink held)\n");
    for r in arena_blink_reports() {
        emit(&mut out, &r);
    }
    out.push_str(
        "# control: the same stance (+32) with no blink rule (ArenaDuoAhead): the blink is the only difference\n",
    );
    for r in arena_blink_control() {
        emit(&mut out, &r);
    }
    out.push_str(
        "# reference only: the shipped trailing ArenaDuo pilot (stance -40, no blink): differs in stance as well\n",
    );
    for r in arena_blink_trailing() {
        emit(&mut out, &r);
    }
    out.push_str("# donut_prog_pilot_boss_boost (--donut-prog boss pin)\n");
    for r in boss_reports() {
        emit(&mut out, &r);
    }
    out.push_str("# cf02_free_blink_boss_arena (AI Donut under cf02_free, boss pin and blink)\n");
    emit(&mut out, &boss_free_run().1);
    println!("{out}");
    if let Ok(path) = std::env::var("DP_F_PILOT_OUT") {
        std::fs::write(path, &out).unwrap();
    }
}

// ------------------------------------------------------------------ asserts --

/// The committed folds of the run hashes of each group (print them with
/// `print_the_pilot_folds`).
const GOLDEN_ARENA_BOOST: u64 = 0xaf27_817c_00c4_b50a;
const GOLDEN_ARENA_BLINK: u64 = 0x336b_b672_e9a3_e696;
const GOLDEN_ARENA_AHEAD: u64 = 0xc64c_2fab_503c_c3f6;
const GOLDEN_BOSS_BOOST: u64 = 0x72e0_85f6_2c96_b8cb;
const GOLDEN_BOSS_ARENA: u64 = 0x514d_33c0_5081_14af;

#[test]
fn print_the_pilot_folds() {
    println!("FOLD arena_boost {:#018x}", fold(&arena_boost_reports()));
    println!("FOLD arena_blink {:#018x}", fold(&arena_blink_reports()));
    println!("FOLD arena_ahead {:#018x}", fold(&arena_blink_control()));
    println!("FOLD boss_boost {:#018x}", fold(&boss_reports()));
    println!("FOLD boss_arena {:#018x}", fold(&[boss_free_run().1]));
}

/// DP-Q47: the solo arena pilots with the biscuit and the jump boost held, at
/// seeds 0..8. `cf02_arena_direct` and `eng_solo_assist` clear the room inside
/// their budget in one attempt at every seed (`eng_solo_assist` does not without
/// the boost at seeds 5 and 7: it dies twice there); `cf02_arena_swap` is a
/// 300-tick scripted swap, not a clearing pilot, so it asserts only survival.
#[test]
fn donut_prog_pilot_arena_boost() {
    let reports = arena_boost_reports();
    assert_eq!(reports.len(), 3 * SEEDS as usize);
    for r in &reports {
        assert_eq!(r.deaths, 0, "{}", line(r));
        assert_eq!(r.restarts, 0, "{}", line(r));
        assert!(r.stuck.iter().all(|s| *s <= STUCK_LIMIT), "{}", line(r));
        assert_eq!(
            r.blinks + r.started,
            0,
            "the blink is not held: {}",
            line(r)
        );
        if r.name != "cf02_arena_swap" {
            assert!(r.done_at.is_some(), "not cleared: {}", line(r));
        }
    }
    // Replayed twice, identical, and pinned.
    assert_eq!(reports, arena_boost_reports());
    assert_eq!(
        fold(&reports),
        GOLDEN_ARENA_BOOST,
        "{:#018x}",
        fold(&reports)
    );
}

/// DP-Q47: the scripted blink pilot (`Pilot::ArenaDuoBlink`) in the two-player
/// room with all three Donut capabilities, seeds 0..8. It proves the human-style
/// blink is pressed, resolves and pays in a live room: the blink is pressed on
/// at least three seeds (every seed before FA-Q14), every press resolves (no cancel, no fizzle: the pilot withholds
/// its own presses during the windup), each arrival pays once and nobody stands
/// still for more than 300 ticks. It does **not** assert that the blink helps:
/// the control at the same stance (`Pilot::ArenaDuoAhead`, no blink rule) is
/// recorded beside it, and the measured numbers (the blink pilot has more deaths
/// than that control) are an owner game-feel note, see the evidence README.
#[test]
fn donut_prog_pilot_arena_blink() {
    let reports = arena_blink_reports();
    let control = arena_blink_control();
    // FA-Q14 (the missile spawn sweep) lets the human-style Donut clear the room
    // point-blank before a swing is aimed at her on some seeds, so "pressed on
    // every seed" no longer holds: it is pressed on at least a third of them
    // (3 of 9 measured), and every press that happens still resolves and pays.
    assert!(
        reports.iter().filter(|r| r.started >= 1).count() >= 3,
        "the pilot presses on too few seeds: {:?}",
        reports.iter().map(|r| r.started).collect::<Vec<_>>()
    );
    for r in &reports {
        assert_eq!(r.started, r.blinks, "every press resolves: {}", line(r));
        assert_eq!((r.cancels, r.fizzles), (0, 0), "{}", line(r));
        assert_eq!(r.blink_spends, r.blinks, "one payment each: {}", line(r));
        assert!(r.stuck.iter().all(|s| *s <= STUCK_LIMIT), "{}", line(r));
    }
    for r in &control {
        assert_eq!(
            r.blinks + r.started,
            0,
            "the control never blinks: {}",
            line(r)
        );
        assert!(r.stuck.iter().all(|s| *s <= STUCK_LIMIT), "{}", line(r));
    }
    assert_eq!(reports, arena_blink_reports());
    assert_eq!(control, arena_blink_control());
    assert_eq!(
        fold(&reports),
        GOLDEN_ARENA_BLINK,
        "{:#018x}",
        fold(&reports)
    );
    assert_eq!(
        fold(&control),
        GOLDEN_ARENA_AHEAD,
        "{:#018x}",
        fold(&control)
    );
}

/// DP-Q47: `boss01_full_clear_solo` and `boss01_full_clear_2p` under the
/// `--donut-prog` boss pin (the boost held, the biscuit read through): both still
/// clear on the same tick as without the pin, the run ends with the Puddle Jumper
/// grant on the defeat tick, no restart and no new standstill.
#[test]
fn donut_prog_pilot_boss_boost() {
    let reports = boss_reports();
    for (r, base) in reports
        .iter()
        .zip([&BOSS01_FULL_CLEAR_SOLO, &BOSS01_FULL_CLEAR_2P])
    {
        assert_eq!((r.deaths, r.restarts), (0, 0), "{}", line(r));
        let (sim, _) = boss_run(base);
        let defeat = r.done_at.unwrap();
        assert!((2400..=7200).contains(&defeat), "{}", line(r));
        let granted: Vec<(u64, u8, &str)> = sim
            .combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::CapabilityGranted {
                    tick,
                    hero,
                    capability,
                } => Some((tick, hero, capability)),
                _ => None,
            })
            .collect();
        assert_eq!(granted, [(defeat, 1, "special_short_blink")]);
        // The unpinned original: the same clear tick and no longer a standstill
        // (the parked P2 is the pilot's own, not a softlock).
        let plain_fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            base: base.base,
            two_player: base.two_player,
            tuning: base.tuning,
            script: base.script,
            pilot: base.pilot,
            donut: base.donut,
        }));
        let kit = crate::boss_scene::load_arena(&root()).unwrap();
        let scene = SceneSim::new_at(&kit, 360, plain_fixture.base.carl_x).unwrap();
        let combat = CombatState::new(plain_fixture.base, &kit, &scene)
            .unwrap()
            .with_donut(plain_fixture.donut);
        let (_, plain) = play(
            PartyFixtureSim::from_parts(scene, combat, plain_fixture),
            Report::default(),
            boss_defeated,
        );
        assert_eq!(plain.done_at, r.done_at, "the pin changes no clear tick");
        assert!(
            r.stuck[0] <= plain.stuck[0].max(STUCK_LIMIT)
                && r.stuck[1] <= plain.stuck[1].max(STUCK_LIMIT),
            "{} against {}",
            line(r),
            line(&plain)
        );
    }
    assert_eq!(reports, boss_reports());
    assert_eq!(
        fold(&reports),
        GOLDEN_BOSS_BOOST,
        "{:#018x}",
        fold(&reports)
    );
}

/// DP-F (deferral 1): the AI Donut under `cf02_free` in the boss arena with the
/// blink held. The Juicer's charge never makes her blink: the follow layer's
/// boss dodge jumps at 18 ticks and the blink window opens at 12, so the jump
/// has always started (DP-Q39 b: "the jump dodge would not clear it"); the
/// charge rule of `choose_blink` is pinned by its pure tests. The run clears on
/// the same tick, nobody dies, and any blink she does press resolves.
#[test]
fn cf02_free_blink_boss_arena() {
    let (sim, r) = boss_free_run();
    assert_eq!((r.deaths, r.restarts), (0, 0), "{}", line(&r));
    assert!(r.done_at.is_some(), "{}", line(&r));
    assert_eq!(r.started, r.blinks + r.cancels + r.fizzles, "{}", line(&r));
    assert_eq!((r.cancels, r.fizzles), (0, 0), "{}", line(&r));
    assert_eq!(
        sim.combat
            .autonomy_state(crate::party::HeroId::Donut)
            .stats()
            .blink_presses_committed,
        r.started
    );
    assert_eq!(fold(&[r]), GOLDEN_BOSS_ARENA);
}
