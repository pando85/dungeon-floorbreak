//! `corral_auto_*`: the assertions that hold against today's llamas (F13), and
//! the report-only measurement of the M2 gate. Run with
//! `cargo test --locked corral_auto -- --nocapture` to print the report.

use super::report::{Gate, Outcome, is_low_risk_stationary_clear, report_md, runs_csv};
use super::*;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn kit() -> EnvKit {
    load_kit(&root()).unwrap()
}

fn spec(stage: Stage, seat: Seat, policy: PolicyId, x: i32) -> RunSpec {
    RunSpec {
        stage,
        seat,
        policy,
        evasion: Evasion::Off,
        donut_x: x,
        facing: 1,
        phase: [0, 0],
        horizon: HORIZON_TICKS,
    }
}

#[test]
fn corral_auto_m1_positive_control_a_stationary_donut_wins_m1() {
    let kit = kit();
    let r = run(
        &kit,
        &spec(Stage::M1, Seat::SoloDonutBot, PolicyId::StationaryCast, 420),
    );
    assert_eq!(r.terminal, Terminal::Victory, "{r:?}");
    // Three real bolts, each a real hit, no Carl or melee contribution.
    assert_eq!(r.bolt_hits, 3);
    assert_eq!(r.carl_hits + r.donut_melee_hits, 0);
    assert!(!r.companion_assisted());
    assert!(r.donut_hp_lost() <= LOW_RISK_HP_LOSS_FOR_TESTS);
    assert!(r.stationary_position_valid(), "she never moved");
    assert!(is_low_risk_stationary_clear(&r));
    // The gate agrees on a whole quick sweep.
    let records = sweep(&kit, &SweepConfig::quick());
    let gate = Gate::evaluate(&records);
    assert_eq!(gate.m1_control, Outcome::Pass, "{:?}", gate.m1_control_best);
}

const LOW_RISK_HP_LOSS_FOR_TESTS: u8 = 1;

#[test]
fn corral_auto_press_edges_and_mana_accounting_close_on_every_run() {
    let kit = kit();
    let records = sweep(&kit, &SweepConfig::quick());
    assert!(records.len() > 100);
    for r in &records {
        if matches!(r.terminal, Terminal::Blocked | Terminal::InvalidSetup) {
            continue;
        }
        // Nothing but the real step touched the pool: no false shot accounting.
        assert!(r.mana_accounting_closes(), "{}: {r:?}", r.run_id);
        match r.spec.policy {
            PolicyId::StationaryCast | PolicyId::StationaryCastBurst => {
                // Every press edge is one real cast (the last press of a run
                // that ends on the kill tick may not have released yet), and a
                // legal-cast policy never presses dry.
                assert!(r.presses >= r.bolts_released, "{}", r.run_id);
                assert!(r.presses - r.bolts_released <= 1, "{}", r.run_id);
                assert_eq!(r.dry_presses, 0, "{}", r.run_id);
                assert!(r.stationary_position_valid(), "{}: she moved", r.run_id);
            }
            PolicyId::HoldAttackTrivial => {
                // 1,800 held ticks are one press, not 1,800 spells.
                assert_eq!(r.presses, 1);
                assert!(r.bolts_released <= 1, "{}: {}", r.run_id, r.bolts_released);
            }
            PolicyId::NoInteraction => {
                assert_eq!(r.presses, 0);
                assert_eq!(r.bolts_released, 0);
            }
            _ => {}
        }
        // Bolts that hit are bounded by bolts released.
        assert!(r.bolt_hits <= r.bolts_released, "{}", r.run_id);
    }
    let gate = Gate::evaluate(&records);
    assert_eq!(gate.mana_accounting, Outcome::Pass);
    assert_eq!(gate.trivial_hold_is_one_press, Outcome::Pass);
}

/// The evaluator writes game state only at tick 0, in `build`. A source scan
/// keeps the other files free of the writes the protocol forbids.
#[test]
fn corral_auto_no_state_writes_after_setup() {
    let forbidden = [
        "missile_hits_llama(",
        "mana = ",
        "health = ",
        "donut_kit",
        ".health -=",
        "hero_defeated =",
        "llamas.push",
        "donut_body.x =",
        "donut_body.y =",
    ];
    let policy = include_str!("policy.rs")
        .split("#[cfg(test)]")
        .next()
        .unwrap();
    let report = include_str!("report.rs");
    let module = include_str!("mod.rs");
    for token in forbidden {
        assert!(!policy.contains(token), "policy.rs contains `{token}`");
        assert!(!report.contains(token), "report.rs contains `{token}`");
    }
    // In mod.rs the only writes are in `build` (donut column, facing, phase).
    let build_start = module.find("pub fn build(").unwrap();
    let build_end = module[build_start..].find("\n}\n").unwrap() + build_start;
    let outside = format!("{}{}", &module[..build_start], &module[build_end..]);
    for token in forbidden {
        assert!(
            !outside.contains(token),
            "mod.rs outside build has `{token}`"
        );
    }
}

/// Setup writes are the whole difference between two phase variants, and the
/// phase really delays the first tell.
#[test]
fn corral_auto_attack_phase_is_written_at_tick_zero() {
    let kit = kit();
    let mut s = spec(Stage::M1, Seat::SoloDonutBot, PolicyId::NoInteraction, 420);
    s.horizon = 300;
    let early = run(&kit, &s);
    s.phase = [89, 0];
    let late = run(&kit, &s);
    let (e, l) = (
        early.first_tell_tick.unwrap(),
        late.first_tell_tick.unwrap(),
    );
    assert!(
        l >= 89 && l > e,
        "first tell {e} (phase 0) vs {l} (phase 89)"
    );
    let sim = build(&kit, &s).unwrap();
    assert_eq!(sim.combat.llamas[0].cooldown, 89);
    assert_eq!(sim.scene.world.tick, 0, "written before the first tick");
}

#[test]
fn corral_auto_double_run_equality() {
    let kit = kit();
    let cases = [
        spec(Stage::M2, Seat::SoloDonutBot, PolicyId::StationaryCast, 420),
        spec(Stage::M2, Seat::SoloDonutBot, PolicyId::MobileCast, 420),
        spec(Stage::M2, Seat::SoloCarl, PolicyId::CarlMelee, 420),
        spec(Stage::M2, Seat::TwoPlayer, PolicyId::CoopProbe, 420),
        spec(
            Stage::M2Pincer,
            Seat::SoloDonutAssist,
            PolicyId::StationaryCastBurst,
            300,
        ),
    ];
    for s in cases {
        let (a, b) = (run(&kit, &s), run(&kit, &s));
        assert_eq!(a.state_hash, b.state_hash, "{}", a.run_id);
        assert_eq!(a.event_lines, b.event_lines, "{}", a.run_id);
        assert_eq!(a, b, "{}", a.run_id);
        assert!(!a.event_lines.is_empty());
    }
}

#[test]
fn corral_auto_invalid_setup_and_pending_variants_are_not_passes() {
    let kit = kit();
    let mut s = spec(
        Stage::M1,
        Seat::SoloDonutBot,
        PolicyId::StationaryCast,
        5000,
    );
    assert_eq!(run(&kit, &s).terminal, Terminal::InvalidSetup);
    s.donut_x = 420;
    s.evasion = Evasion::Reflex;
    let r = run(&kit, &s);
    assert_eq!(r.terminal, Terminal::Blocked);
    assert!(r.note.contains("DM-B"));
    // A blocked reflex gate is `blocked`, never `pass`.
    let mut config = SweepConfig::quick();
    config.stages = vec![Stage::M2];
    config.evasions = Evasion::ALL.to_vec();
    let gate = Gate::evaluate(&sweep(&kit, &config));
    assert_eq!(gate.m2_stationary_reflex, Outcome::Blocked);
}

/// The M2 gate (KC19) and the system-failure measurements depend on the rush
/// and on party targeting that CORRAL-B adds. Against today's llamas the gate
/// is expected to FAIL; this test prints the measurement and asserts none of
/// it (F13). CORRAL-C turns the gate into an assertion.
#[test]
fn corral_auto_m2_gate_report_only() {
    let kit = kit();
    let config = SweepConfig::full();
    let records = sweep(&kit, &config);
    let targeting = carl_only_targeting(&kit);
    let gate = Gate::evaluate(&records);
    println!("{}", report_md(&records, &config, targeting));
    println!(
        "M2 stationary (reflex off): {} ({} groups, {} failing); first failing groups:",
        gate.m2_stationary.name(),
        gate.m2_groups,
        gate.m2_failing_groups.len()
    );
    for g in gate.m2_failing_groups.iter().take(5) {
        println!(
            "  {} {}/{} best={:?}",
            g.group_id, g.clears, g.runs, g.best_counterexample
        );
    }
    assert!(targeting.is_some(), "the probe ran");
    // The report machinery itself is asserted: well-formed CSV, one row per run.
    let csv = runs_csv(&records);
    assert_eq!(csv.lines().count(), records.len() + 1);
    let columns = csv.lines().next().unwrap().split(',').count();
    assert!(csv.lines().all(|l| l.split(',').count() == columns));
}

#[test]
fn corral_auto_outputs_are_written() {
    let out = std::env::temp_dir().join(format!("corral-eval-test-{}", std::process::id()));
    let summary = evaluate_to(&root(), &out, &SweepConfig::quick()).unwrap();
    for name in ["summary.json", "runs.csv", "report.md"] {
        assert!(out.join(name).is_file(), "{name}");
    }
    let json: serde_json::Value = serde_json::from_str(&summary).unwrap();
    assert_eq!(json["gates"]["m1_positive_control"], "pass");
    assert!(json["gates"]["m2_stationary_kc19_report_only"].is_string());
    let _ = std::fs::remove_dir_all(&out);
}
