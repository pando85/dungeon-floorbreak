//! DONUT-MOBILITY-01 DM-B: the scene hooks of the cat reflex that need private
//! access: the reset rules (DM-Q13), the check order (DM-Q20), the ram and the
//! glob (DM-Q19), the never-evadable sources (DM-Q18) and the allow-list scan.

use std::path::PathBuf;

use super::super::*;
use crate::agility::{EvadeSource, Reflex};
use crate::boss_scene::Pilot;
use crate::cf02::arena;
use crate::combat::EnemyAi;
use crate::donut_mana::DonutSetup;
use crate::env_kit::EnvKit;
use crate::party::{HeroId, PROVISIONAL_TUNING};
use crate::party_scene::{PartyFixture, PartyFixtureSim, SlotHold, base, jump, swap};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn corridor() -> EnvKit {
    EnvKit::load(&root(), "assets/levels/floor01/corridor_abc_sample.json").unwrap()
}

fn events_of<T>(sim: &PartyFixtureSim, pick: impl Fn(&CombatEvent) -> Option<T>) -> Vec<T> {
    sim.combat.events.iter().filter_map(pick).collect()
}

fn armed_ticks(sim: &PartyFixtureSim) -> Vec<u64> {
    events_of(sim, |e| match *e {
        CombatEvent::EvadeArmed { tick, .. } => Some(tick),
        _ => None,
    })
}

fn evaded(sim: &PartyFixtureSim) -> Vec<(u64, EvadeSource)> {
    events_of(sim, |e| match *e {
        CombatEvent::HeroEvaded { tick, kind, .. } => Some((tick, kind)),
        _ => None,
    })
}

/// `(tick, hero, source)` of every hurt.
fn hurts(sim: &PartyFixtureSim) -> Vec<(u64, HeroId, u32)> {
    events_of(sim, |e| match *e {
        CombatEvent::HeroHurt {
            tick, hero, source, ..
        } => Some((tick, hero, source)),
        _ => None,
    })
}

/// An empty room: no enemy, so only what a test injects can hurt her.
static EMPTY_BASE: Fixture = base("dm_reflex_empty", "empty corridor", 0x0D4D_0030, &[], 2000);

fn empty_fixture(two_player: bool, script: &'static [SlotHold]) -> &'static PartyFixture {
    Box::leak(Box::new(PartyFixture {
        base: &EMPTY_BASE,
        two_player,
        tuning: PROVISIONAL_TUNING,
        script,
        pilot: Pilot::None,
        donut: DonutSetup::unlocked(8),
    }))
}

fn empty_sim(two_player: bool, script: &'static [SlotHold]) -> PartyFixtureSim {
    PartyFixtureSim::new(&corridor(), 360, empty_fixture(two_player, script)).unwrap()
}

/// A non-evadable blow on Donut through the boss and block path (`hurt_hero`).
fn blow(sim: &mut PartyFixtureSim) -> bool {
    let tick = sim.scene.world.tick;
    let from_x = sim.scene.world.donut_body.x + 10.0;
    sim.combat.hurt_hero(
        &mut sim.scene,
        HeroBlow {
            hero: HeroId::Donut,
            source: arena::SHORE_BLOCK_SOURCE,
            damage: 1,
            from_x,
            knock_vx: 0.0,
            knock_vy: 0.0,
        },
        tick,
    )
}

// ------------------------------------------------------- the reset rules --

/// DM-Q13: a `HeroHurt` from **any** source, evadable or not, restarts the wait,
/// and a non-evadable blow is never evaded: this is the rush or body hit of a
/// later enemy under DM-Q18 (default never), which resets an armed reflex.
#[test]
fn a_non_evadable_hurt_is_not_evaded_and_restarts_the_wait() {
    let mut sim = empty_sim(true, &[]);
    sim.run_scripted(800);
    assert_eq!(armed_ticks(&sim), vec![720]);
    assert!(sim.combat.reflex().armed());
    let last = sim.scene.world.tick;
    assert!(
        blow(&mut sim),
        "the non-evadable blow lands on an armed Donut"
    );
    assert!(evaded(&sim).is_empty(), "never evaded");
    assert!(!sim.combat.reflex().armed(), "she is rattled: disarmed");
    assert_eq!(sim.combat.reflex().last_reset_tick(), last);
    sim.run_scripted(1500);
    assert_eq!(
        armed_ticks(&sim),
        vec![720, last + 720],
        "re-armed exactly 720 ticks after the hurt"
    );
}

/// DM-Q13: an unarmed hurt also resets: the wait counts from the last hurt.
#[test]
fn a_hurt_before_the_arm_pushes_the_arm_back() {
    let mut sim = empty_sim(true, &[]);
    sim.run_scripted(500);
    let last = sim.scene.world.tick;
    assert!(blow(&mut sim));
    sim.run_scripted(1300);
    assert_eq!(armed_ticks(&sim), vec![last + 720]);
}

/// DM-Q13: Carl's hurt does not rattle her.
#[test]
fn carls_hurt_does_not_reset_her_wait() {
    let mut sim = empty_sim(true, &[]);
    sim.run_scripted(500);
    let tick = sim.scene.world.tick;
    sim.combat.log(CombatEvent::HeroHurt {
        tick,
        source: 0,
        health_left: 3,
        hero: HeroId::Carl,
    });
    sim.run_scripted(300);
    assert_eq!(armed_ticks(&sim), vec![720]);
}

/// DM-Q13: an evade resets the wait: it re-arms 720 ticks after the evade.
#[test]
fn an_evade_restarts_the_wait() {
    let mut sim = empty_sim(true, &[]);
    sim.combat.arm_reflex_for_test();
    sim.run_scripted(10);
    let tick = sim.scene.world.tick;
    let key = (EvadeSource::GoblinSwing, 0, 5);
    assert_eq!(
        sim.combat.evade_check(&sim.scene, key, 0, tick),
        Verdict::Evaded
    );
    assert_eq!(
        sim.combat.evade_check(&sim.scene, key, 0, tick + 1),
        Verdict::Passed,
        "the same instance never hits her again"
    );
    assert_eq!(sim.combat.reflex().last_reset_tick(), tick);
    sim.run_scripted(800);
    assert_eq!(armed_ticks(&sim), vec![tick + 720]);
    assert_eq!(evaded(&sim).len(), 1);
}

/// DM-Q13: a solo swap does not reset it (it changes who drives her body, not
/// the body).
#[test]
fn a_solo_swap_does_not_reset_the_wait() {
    static SCRIPT: [SlotHold; 2] = [swap(730, 731), swap(790, 791)];
    let mut sim = empty_sim(false, &SCRIPT);
    sim.run_scripted(725);
    assert!(sim.combat.reflex().armed());
    sim.run_scripted(100);
    assert_eq!(
        sim.scene.drive().controlled,
        HeroId::Carl,
        "two swaps: the control is back on Carl"
    );
    assert!(sim.combat.reflex().armed(), "still armed after the swaps");
    assert_eq!(sim.combat.reflex().last_reset_tick(), 0);
    assert_eq!(armed_ticks(&sim), vec![720], "armed once");
}

/// DM-Q13: a fixture restart restarts the wait at 0 of the new attempt and a
/// checkpoint respawn at its tick; nothing of the old attempt stays.
#[test]
fn a_restart_and_a_respawn_restart_the_wait() {
    let mut sim = empty_sim(true, &[]);
    sim.run_scripted(800);
    assert!(sim.combat.reflex().ever_armed());
    sim.combat.restart(&mut sim.scene);
    assert_eq!(*sim.combat.reflex(), Reflex::default());
    sim.run_scripted(730);
    assert_eq!(
        armed_ticks(&sim),
        vec![720, 720],
        "again 720 into the attempt"
    );

    static RULES: DemoRules = DemoRules {
        checkpoints: &[400],
    };
    let fixture = empty_fixture(true, &[]);
    let kit = corridor();
    let scene = crate::corridor::SceneSim::new_at(&kit, 360, EMPTY_BASE.carl_x).unwrap();
    let combat = CombatState::new(&EMPTY_BASE, &kit, &scene)
        .unwrap()
        .with_demo(&RULES)
        .with_donut(fixture.donut);
    let mut sim = PartyFixtureSim::from_parts(scene, combat, fixture);
    sim.run_scripted(800);
    assert!(sim.combat.reflex().armed());
    sim.combat.respawn(&mut sim.scene);
    let respawn = sim.scene.world.tick;
    assert!(!sim.combat.reflex().armed());
    assert_eq!(sim.combat.reflex().last_reset_tick(), respawn);
    sim.run_scripted(730);
    assert_eq!(armed_ticks(&sim), vec![720, respawn + 720]);
}

// ----------------------------------------------------------- the guards --

/// DM-Q20: an existing guard consumes no arm. A Donut in her hurt i-frames is
/// skipped before the evasion check, so the swing that would have been
/// evaded is simply not a hit, and the reflex stays armed for the next one.
#[test]
fn a_guard_consumes_no_arm() {
    use crate::agility_fixtures::DM_REFLEX_NOT_ARMED;
    let kit = corridor();
    let mut sim = PartyFixtureSim::new(&kit, 360, &DM_REFLEX_NOT_ARMED).unwrap();
    sim.combat.arm_reflex_for_test();
    // Her i-frames cover the first swing (tick 129) and the second (224).
    sim.run_scripted(100);
    sim.scene.world.donut_body.invulnerable_ticks = 200;
    sim.run_scripted(180);
    assert!(
        hurts(&sim).iter().all(|h| h.1 != HeroId::Donut),
        "no hurt through the i-frames"
    );
    assert!(evaded(&sim).is_empty(), "and nothing was evaded either");
    assert!(sim.combat.reflex().armed(), "the guard consumed no arm");
    // The first unguarded swing is the one that is evaded.
    sim.run_scripted(120);
    let evades = evaded(&sim);
    assert_eq!(evades.len(), 1, "{evades:?}");
    assert!(evades[0].0 > 300);
}

/// DM-Q20 and D1 M4: the off-screen guard consumes no arm either. In solo the AI
/// Donut is guarded; a swing aimed at her beyond the view margin is not a hit,
/// is not evaded, and leaves her armed. The same swing on screen reaches the
/// check and is evaded (the control).
#[test]
fn an_offscreen_guard_consumes_no_arm() {
    use crate::goblin::GoblinMode;
    use crate::party_scene::SOLO_GUARD_FLOORS_AT_ONE;
    let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
        donut: DonutSetup::unlocked(8),
        base: SOLO_GUARD_FLOORS_AT_ONE.base,
        two_player: false,
        tuning: SOLO_GUARD_FLOORS_AT_ONE.tuning,
        script: &[],
        pilot: Pilot::None,
    }));
    for (donut_x, on_screen) in [(40.0, false), (250.0, true)] {
        let mut sim = PartyFixtureSim::new(&corridor(), 360, fixture).unwrap();
        sim.combat.arm_reflex_for_test();
        sim.scene.world.donut_body.x = donut_x;
        sim.scene.world.sync_donut_for_tests();
        let g = &mut sim.combat.enemies[0];
        g.x = donut_x - 24.0;
        g.facing = 1;
        g.mode = GoblinMode::Attack;
        g.attack_tick = 24;
        assert_eq!(sim.scene.is_offscreen(donut_x, 16.0), !on_screen);
        sim.run_scripted(6);
        assert!(
            hurts(&sim).iter().all(|h| h.1 != HeroId::Donut),
            "donut at {donut_x}: never hurt"
        );
        assert_eq!(
            !evaded(&sim).is_empty(),
            on_screen,
            "donut at {donut_x}: the check is reached only on screen"
        );
        assert_eq!(
            sim.combat.reflex().armed(),
            !on_screen,
            "donut at {donut_x}: the guard spends no arm, the evade does"
        );
    }
}

/// The guards sit before the evasion check in the three evadable sources
/// (DM-Q20), and the check has exactly those three call sites (DM-Q17, DM-Q18).
#[test]
fn the_allow_list_has_three_call_sites_and_the_guards_come_first() {
    let source = std::fs::read_to_string(root().join("src/combat_scene.rs")).unwrap();
    let function = |name: &str| -> &str {
        let start = source
            .find(&format!("fn {name}("))
            .unwrap_or_else(|| panic!("fn {name}"));
        let rest = &source[start..];
        let end = rest[1..].find("\n    fn ").map_or(rest.len(), |i| i + 1);
        &rest[..end]
    };
    for (name, guards) in [
        ("swing_on", ["invulnerable_ticks > 0", "unseen"]),
        ("ram_on_hero", ["invulnerable_ticks > 0", "unseen"]),
        ("step_globs", ["invulnerable_ticks == 0", "unseen"]),
    ] {
        let body = function(name);
        let check = body
            .find("evade_check(")
            .unwrap_or_else(|| panic!("{name}"));
        assert_eq!(body.matches("evade_check(").count(), 1, "{name}");
        for guard in guards {
            let at = body
                .find(guard)
                .unwrap_or_else(|| panic!("{name}: {guard}"));
            assert!(at < check, "{name}: `{guard}` must come before the check");
        }
    }
    // No other source in the crate asks the reflex: count the call sites in
    // every non-test source file.
    let mut calls = 0;
    let mut stack = vec![root().join("src")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                let code = text.split("#[cfg(test)]\nmod ").next().unwrap_or("");
                let in_tests = path.to_string_lossy().contains("/tests");
                if !in_tests && !path.ends_with("evade.rs") {
                    calls += code.matches(".evade_check(").count();
                }
            }
        }
    }
    assert_eq!(
        calls, 3,
        "swing, ram and glob: a new source must opt in here"
    );
}

/// The part of a source file a play path can reach: everything before its
/// `#[cfg(test)]` module, minus `encode_tuning` of `scenario.rs`, the one
/// non-test reader of the profile (it matches on the profile, it never builds
/// one).
fn play_code(name: &str, text: &str) -> String {
    let mut code = text
        .split("#[cfg(test)]\nmod ")
        .next()
        .unwrap_or("")
        .to_owned();
    if name == "scenario.rs"
        && let Some(start) = code.find("fn encode_tuning(")
    {
        let end = code[start..]
            .find("\nfn ")
            .map_or(code.len(), |i| start + i);
        code.replace_range(start..end, "");
    }
    code
}

/// DM-C F4, the negative test of the scan above: a profile built in a non-test
/// fixture file or outside `encode_tuning` is seen; one inside a test module,
/// or in `encode_tuning`, is not.
#[test]
fn the_profile_scan_sees_a_play_path_leak() {
    let leak = "pub const F: u8 = 1;\nfn make() { let _ = ReflexProfile::Rearm(1); }\n";
    assert!(play_code("agility_fixtures.rs", leak).contains("ReflexProfile::Rearm"));
    let in_tests = "fn ok() {}\n#[cfg(test)]\nmod tests {\n fn t() { ReflexProfile::Off; }\n}\n";
    assert!(!play_code("agility_fixtures.rs", in_tests).contains("ReflexProfile::Off"));
    let scenario = "fn encode_tuning(t: u8) {\n match p { ReflexProfile::Off => {} }\n}\n\nfn other() { ReflexProfile::Off; }\n";
    let code = play_code("scenario.rs", scenario);
    assert_eq!(code.matches("ReflexProfile::Off").count(), 1, "{code}");
    assert!(code.contains("fn other"), "{code}");
}

/// The test-only profiles are never built by a play path: `ReflexProfile::Off`
/// and `Rearm` appear only in tests (and the profile's own definition).
#[test]
fn no_play_path_constructs_a_non_shipped_reflex_profile() {
    let mut stack = vec![root().join("src"), root().join("tests")];
    while let Some(dir) = stack.pop() {
        let Ok(read) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in read {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            // DM-C F4: only the definition and the test files are exempt as a
            // whole; every other file, `agility_fixtures.rs` included (the binary
            // resolves its fixtures by name), is scanned up to its `#[cfg(test)]`
            // module.
            let allowed = ["agility.rs", "agility_reflex_tests.rs", "tests.rs"];
            if allowed.contains(&name.as_str()) {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let code = play_code(&name, &text);
            for needle in ["ReflexProfile::Off", "ReflexProfile::Rearm"] {
                assert!(
                    !code.contains(needle),
                    "{} builds {needle} outside tests",
                    path.display()
                );
            }
        }
    }
}

// ------------------------------------------- the ram, the glob, the rest --

/// A scene on the arena kit under `cf02_rules` with the biscuit, built by
/// hand: the Engineer and the llama are keyed by fixture-name prefixes that a
/// `dm_` name does not carry.
fn arena_sim(
    base: &'static Fixture,
    donut_x: f32,
    script: &'static [SlotHold],
) -> (PartyFixtureSim, &'static PartyFixture) {
    let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
        base,
        two_player: true,
        tuning: PROVISIONAL_TUNING,
        script,
        pilot: Pilot::None,
        donut: DonutSetup::unlocked(8),
    }));
    let kit = arena::load_arena(&root()).unwrap();
    let mut scene = crate::corridor::SceneSim::new_at(&kit, 360, base.carl_x).unwrap();
    scene.world.donut_body.x = donut_x;
    scene.world.sync_donut_for_tests();
    let combat = CombatState::new(base, &kit, &scene)
        .unwrap()
        .with_cf02()
        .with_donut(fixture.donut);
    (PartyFixtureSim::from_parts(scene, combat, fixture), fixture)
}

const fn spawn(x: i32) -> EnemySpawn {
    EnemySpawn {
        x,
        ai: EnemyAi::Chase,
    }
}

static ENGINEER: [EnemySpawn; 1] = [spawn(200)];

/// Carl at 270 is the Engineer's target; Donut stands a few px ahead of him in
/// the lane (the closer to the ram).
static RAM_BASE: Fixture = Fixture {
    carl_x: 270,
    enemies: &[],
    engineers: &ENGINEER,
    shore_dx: None,
    llamas: &[],
    ..arena::fixture(
        "dm_reflex_ram_passes",
        "An armed Donut in the ram lane: the ram passes her and hits Carl behind her on the same tick",
        0x0D4D_0020,
        160,
    )
};

/// DM-C F2: the pinned `state_hash()` of the hand-built ram and glob sims.
const DM_RAM_PASSES_HASH: u64 = 0xd3b1_c849_50a8_6a45;
const DM_GLOB_PASSES_HASH: u64 = 0xd528_4158_593d_f315;

/// DM-Q19, U3: the evasion check runs inside the ram's candidate loop. An
/// evaded Donut is skipped like an invulnerable one, so Carl behind her is
/// chosen on the same tick, and the ram goes on.
#[test]
fn the_ram_passes_an_armed_donut_and_hits_carl_behind_her_on_the_same_tick() {
    let (mut sim, _) = arena_sim(&RAM_BASE, 266.0, &[]);
    sim.combat.arm_reflex_for_test();
    sim.run_scripted(100);
    let evades = evaded(&sim);
    assert_eq!(evades, vec![(44, EvadeSource::EngineerRam)]);
    let hurt = hurts(&sim);
    assert_eq!(hurt.len(), 1, "{hurt:?}");
    assert_eq!(
        (hurt[0].0, hurt[0].1),
        (44, HeroId::Carl),
        "Carl is hit on the same tick Donut is evaded"
    );
    assert_eq!(sim.scene.world.donut.health, 4, "Donut is untouched");
    assert!(!sim.combat.reflex().armed());
    // DM-C F2: the hand-built sim is deterministic, so its hash is pinned.
    assert_eq!(
        sim.state_hash(),
        DM_RAM_PASSES_HASH,
        "{:#018x}",
        sim.state_hash()
    );
}

/// The same ram on an unarmed Donut hits her (the ordinary path).
#[test]
fn an_unarmed_donut_is_rammed_as_before() {
    let (mut sim, _) = arena_sim(&RAM_BASE, 266.0, &[]);
    sim.run_scripted(100);
    assert!(evaded(&sim).is_empty());
    assert_eq!(hurts(&sim)[0].1, HeroId::Donut);
}

/// A Donut that is not the one the ram would hit does not spend her arm: with
/// Carl nearer to the Engineer she is a candidate, not the target. (An adopted
/// refinement of "inside the loop": the check runs for the hero the ram would
/// hit, never for one it would not.)
#[test]
fn the_arm_is_spent_only_when_she_would_have_been_the_one_hit() {
    let (mut sim, _) = arena_sim(&RAM_BASE, 270.0, &[]);
    sim.combat.arm_reflex_for_test();
    sim.run_scripted(100);
    assert!(evaded(&sim).is_empty());
    assert!(sim.combat.reflex().armed(), "the arm is still there");
    assert_eq!(hurts(&sim)[0].1, HeroId::Carl);
}

static LLAMA: [LlamaSpawn; 1] = [LlamaSpawn {
    x: 700,
    ai: EnemyAi::Chase,
}];

static GLOB_BASE: Fixture = Fixture {
    carl_x: 300,
    enemies: &[],
    engineers: &[],
    shore_dx: None,
    llamas: &LLAMA,
    ..arena::fixture(
        "dm_reflex_glob_passes",
        "An armed Donut jumps into the llama's glob on its way to Carl: it passes through her and hits him",
        0x0D4D_0021,
        300,
    )
};

/// DM-Q19: a glob that meets an armed Donut passes through her and keeps
/// flying (it does not splat); the glob's identity is `(owner, spawn tick)`.
#[test]
fn the_glob_passes_through_an_armed_donut_and_flies_on_to_carl() {
    static SCRIPT: [SlotHold; 1] = [jump(130, 131, 1)];
    let (mut sim, _) = arena_sim(&GLOB_BASE, 500.0, &SCRIPT);
    sim.combat.arm_reflex_for_test();
    sim.run_scripted(300);
    let spit = events_of(&sim, |e| match *e {
        CombatEvent::LlamaSpit { tick, .. } => Some(tick),
        _ => None,
    });
    assert_eq!(spit.first(), Some(&157));
    let evades = evaded(&sim);
    assert_eq!(evades.len(), 1, "{evades:?}");
    assert_eq!(evades[0].1, EvadeSource::LlamaGlob);
    assert!((158..200).contains(&evades[0].0), "{evades:?}");
    let hurt = hurts(&sim);
    assert!(
        hurt.iter()
            .any(|h| h.1 == HeroId::Carl && h.0 > evades[0].0),
        "the glob flew on and hurt Carl: {hurt:?}"
    );
    assert!(
        hurt.iter().all(|h| h.1 != HeroId::Donut),
        "Donut took nothing: {hurt:?}"
    );
    assert_eq!(sim.scene.world.donut.health, 4);
    assert_eq!(sim.combat.globs.first().map(|g| g.spawn_tick), None);
    // DM-C F2: the hand-built sim is deterministic, so its hash is pinned.
    assert_eq!(
        sim.state_hash(),
        DM_GLOB_PASSES_HASH,
        "{:#018x}",
        sim.state_hash()
    );
}

/// The glob's spawn tick is set at spawn and is not hashed.
#[test]
fn the_glob_spawn_tick_is_set_at_spawn_and_not_hashed() {
    let (mut sim, _) = arena_sim(&GLOB_BASE, 500.0, &[]);
    sim.run_scripted(160);
    let glob = sim.combat.globs.first().copied().expect("a glob in flight");
    assert_eq!(glob.spawn_tick, 157);
    let before = sim.state_hash();
    sim.combat.globs[0].spawn_tick = 0;
    assert_eq!(
        sim.state_hash(),
        before,
        "spawn_tick is derivable, not hashed"
    );
}

/// DM-Q18: the shore block and a boss blow are never evaded, even by an armed
/// Donut. Both are the real fixtures with the biscuit and the reflex armed.
#[test]
fn the_shore_block_and_a_boss_blow_are_never_evaded() {
    for name in ["cf02_block_hurts_partner", "boss01_2p_gate"] {
        let fixture = crate::cf02_fixtures::find_party_fixture(name).unwrap();
        let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            donut: DonutSetup::unlocked(8),
            base: fixture.base,
            two_player: fixture.two_player,
            tuning: fixture.tuning,
            script: fixture.script,
            pilot: fixture.pilot,
        }));
        let kit = if fixture.base.boss.is_some() {
            crate::boss_scene::load_arena(&root()).unwrap()
        } else {
            arena::load_arena(&root()).unwrap()
        };
        let mut sim = PartyFixtureSim::new(&kit, 360, fixture).unwrap();
        sim.combat.arm_reflex_for_test();
        let want = if fixture.base.boss.is_some() {
            crate::boss::BOSS_SOURCE_ID
        } else {
            arena::SHORE_BLOCK_SOURCE
        };
        let mut hurt = None;
        let mut armed_before = false;
        for _ in 0..fixture.base.ticks {
            armed_before = sim.combat.reflex().armed();
            sim.step_scripted();
            if let Some(h) = hurts(&sim).into_iter().find(|h| h.1 == HeroId::Donut) {
                hurt = Some(h);
                break;
            }
        }
        let hurt = hurt.unwrap_or_else(|| panic!("{name}: Donut is hurt by the source"));
        assert_eq!(
            hurt.2, want,
            "{name}: the hurt came from the expected source"
        );
        assert!(armed_before, "{name}: she was armed when it landed");
        assert!(evaded(&sim).is_empty(), "{name}: never evaded");
        assert!(
            !sim.combat.reflex().armed(),
            "{name}: the hurt rattled her instead"
        );
    }
}
