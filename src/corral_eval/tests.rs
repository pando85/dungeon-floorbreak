//! `corral_auto_*`: the CORRAL-C gate on the Infernal Corral (asserted) and the
//! CORRAL-EVAL core baseline checks. Run with
//! `cargo test --locked corral_auto -- --nocapture` to print the report.

use super::report::{
    Gate, LOW_RISK_HP_LOSS, Outcome, is_low_risk_stationary_clear, report_md, runs_csv,
};
use super::*;
use std::path::PathBuf;
use std::sync::OnceLock;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn kits() -> &'static Kits {
    static KITS: OnceLock<Kits> = OnceLock::new();
    KITS.get_or_init(|| load_kits(&root()).unwrap())
}

fn spec(stage: Stage, seat: Seat, policy: PolicyId, x: i32) -> RunSpec {
    RunSpec {
        stage,
        seat,
        policy,
        evasion: Evasion::Off,
        donut_x: x,
        donut_rise: 0,
        facing: 1,
        phase: [(0, 0), (0, 0)],
        jump_offset: 0,
        horizon: HORIZON_TICKS,
    }
}

/// The gate sweep, run once for every test that reads it.
fn gate_records() -> &'static Vec<RunRecord> {
    static RECORDS: OnceLock<Vec<RunRecord>> = OnceLock::new();
    RECORDS.get_or_init(|| sweep(kits(), &SweepConfig::gate()))
}

// ------------------------------------------------------------- baselines --

#[test]
fn corral_auto_m1_positive_control_a_stationary_donut_wins_m1() {
    let r = run(
        kits(),
        &spec(Stage::M1, Seat::SoloDonutBot, PolicyId::StationaryCast, 420),
    );
    assert_eq!(r.terminal, Terminal::Victory, "{r:?}");
    // Three real bolts, each a real hit, no Carl or melee contribution.
    assert_eq!(r.bolt_hits, 3);
    assert_eq!(r.carl_hits + r.donut_melee_hits, 0);
    assert!(!r.companion_assisted());
    assert!(r.donut_hp_lost() <= LOW_RISK_HP_LOSS);
    assert_eq!(r.donut_move_inputs, 0, "the policy sent no move input");
    assert!(is_low_risk_stationary_clear(&r));
    let records = sweep(kits(), &SweepConfig::quick());
    let gate = Gate::evaluate(&records);
    assert_eq!(gate.m1_control, Outcome::Pass, "{:?}", gate.m1_control_best);
}

#[test]
fn corral_auto_press_edges_and_mana_accounting_close_on_every_run() {
    let mut records = sweep(kits(), &SweepConfig::quick());
    records.extend(gate_records().iter().cloned());
    assert!(records.len() > 100);
    for r in &records {
        if r.terminal == Terminal::InvalidSetup {
            continue;
        }
        // Nothing but the real step touched the pool: no false shot accounting.
        assert!(r.mana_accounting_closes(), "{}: {r:?}", r.run_id);
        if r.spec.policy.is_stationary_gate() {
            // Every press edge is one real cast (the last press of a run that
            // ends on the kill tick may not have released yet), and a
            // legal-cast policy never presses dry.
            assert!(r.presses >= r.bolts_released, "{}", r.run_id);
            assert!(r.presses - r.bolts_released <= 1, "{}", r.run_id);
            assert_eq!(r.dry_presses, 0, "{}", r.run_id);
            if r.spec.policy == PolicyId::StationaryJumpCast {
                // CRL-Q42: every Primary press is paired with a Jump press.
                assert!(r.jump_presses + 1 >= r.presses, "{}", r.run_id);
            }
            if !r.spec.policy.jumps() && r.spec.policy != PolicyId::StationaryTurret {
                assert_eq!(r.donut_move_inputs, 0, "{}: she was told to move", r.run_id);
            }
        }
        match r.spec.policy {
            PolicyId::HoldAttackTrivial => {
                assert_eq!(r.presses, 1);
                assert!(r.bolts_released <= 1, "{}: {}", r.run_id, r.bolts_released);
            }
            PolicyId::NoInteraction => {
                assert_eq!(r.presses, 0);
                assert_eq!(r.bolts_released, 0);
            }
            _ => {}
        }
        assert!(r.bolt_hits <= r.bolts_released, "{}", r.run_id);
    }
    let gate = Gate::evaluate(&records);
    assert_eq!(gate.mana_accounting, Outcome::Pass);
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
        "hero.x =",
        "cooldown =",
        "park_uncontrolled(",
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
    let mut s = spec(Stage::M1, Seat::SoloDonutBot, PolicyId::NoInteraction, 420);
    s.horizon = 300;
    let early = run(kits(), &s);
    s.phase = [(89, 0), (0, 0)];
    let late = run(kits(), &s);
    let (e, l) = (
        early.first_tell_tick.unwrap(),
        late.first_tell_tick.unwrap(),
    );
    assert!(
        l >= 89 && l > e,
        "first tell {e} (phase 0) vs {l} (phase 89)"
    );
    let sim = build(kits(), &s).unwrap();
    assert_eq!(sim.combat.llamas[0].cooldown, 89);
    assert_eq!(sim.scene.world.tick, 0, "written before the first tick");
    // The corral writes both halves of the CRL-Q36 pair.
    let mut c = spec(
        Stage::CorralM2,
        Seat::SoloDonutParked,
        PolicyId::NoInteraction,
        1072,
    );
    c.phase = M2_PHASES[1];
    let sim = build(kits(), &c).unwrap();
    for (l, (cd, rcd)) in sim.combat.llamas.iter().zip(M2_PHASES[1]) {
        assert_eq!(
            (l.cooldown, l.corral.as_ref().unwrap().rush_cooldown),
            (cd, rcd)
        );
    }
}

#[test]
fn corral_auto_invalid_setup_is_not_a_pass() {
    let s = spec(
        Stage::M1,
        Seat::SoloDonutBot,
        PolicyId::StationaryCast,
        5000,
    );
    assert_eq!(run(kits(), &s).terminal, Terminal::InvalidSetup);
    // A corral start on a surface that is not there.
    let mut c = spec(
        Stage::CorralM2,
        Seat::SoloDonutParked,
        PolicyId::StationaryCast,
        1400,
    );
    c.donut_rise = 48;
    let r = run(kits(), &c);
    assert_eq!(r.terminal, Terminal::InvalidSetup, "{}", r.note);
}

/// The two reflex variants really differ: `off` never evades, `reflex` can.
#[test]
fn corral_auto_the_reflex_variants_are_real() {
    let mut s = spec(Stage::M1, Seat::SoloDonutBot, PolicyId::NoInteraction, 420);
    s.horizon = 600;
    assert_eq!(run(kits(), &s).evades, 0, "off");
    s.evasion = Evasion::Reflex;
    let sim = build(kits(), &s).unwrap();
    assert_eq!(
        sim.scene.drive().tuning.donut_reflex,
        crate::agility::ReflexProfile::Shipped
    );
    s.evasion = Evasion::Off;
    let sim = build(kits(), &s).unwrap();
    assert_eq!(
        sim.scene.drive().tuning.donut_reflex,
        crate::agility::ReflexProfile::Off
    );
}

// ---------------------------------------------------------------- corral --

/// Spec section 11, F3 and NB2: the decisive seat parks Carl at 880. He gets
/// a neutral input every tick and never moves, and Donut is the controlled
/// hero from tick 1.
#[test]
fn corral_auto_the_parked_carl_never_moves_and_draws_no_attack() {
    let mut s = spec(
        Stage::CorralM2,
        Seat::SoloDonutParked,
        PolicyId::StationaryCast,
        1072,
    );
    s.horizon = 900;
    let mut sim = build(kits(), &s).unwrap();
    for t in 1..=900u64 {
        let mut raw = RawInput::NONE;
        raw.swap = t == 1;
        sim.step_slots([raw, RawInput::NONE]);
    }
    assert_eq!(sim.scene.world.hero.x, room::CARL_PARK_M2 as f32);
    assert_eq!(sim.party.controlled_hero(), HeroId::Donut);
    let r = run(kits(), &s);
    assert_eq!(r.carl_hurt_events, 0, "{r:?}");
    assert_eq!(r.carl_hits, 0);
}

/// Gate item 3: the positive control on the room (M1-A and M1-B).
#[test]
fn corral_auto_m1_static_control_on_the_room() {
    for start in M1_STARTS {
        for evasion in Evasion::ALL {
            let mut s = spec(
                Stage::CorralM1,
                Seat::SoloDonutParked,
                PolicyId::StationaryCastAimed,
                start.x,
            );
            s.evasion = evasion;
            let r = run(kits(), &s);
            assert_eq!(r.terminal, Terminal::Victory, "{}: {r:?}", start.id);
            assert_eq!(r.donut_hp_lost(), 0, "{}", start.id);
            assert_eq!(r.bolt_hits, 3, "{}: three honest bolts", start.id);
            assert_eq!(r.rush_tells, 0, "{}: the M1 llama never rushes", start.id);
            assert_eq!(r.failures.total(), 0, "{}: {}", start.id, r.note);
        }
    }
}

/// Spec section 11, M2-13 (the review's B1 counterexample): Donut at 1728 on
/// the floor, every jump offset and the aimed jump-cast: A is never hit and
/// never damaged.
#[test]
fn corral_auto_m2_b1_counterexample_never_touches_the_dock_llama() {
    let mut specs = Vec::new();
    for facing in [1, -1] {
        for phase in M2_PHASES {
            let mut s = spec(
                Stage::CorralM2,
                Seat::SoloDonutParked,
                PolicyId::StationaryJumpCastAimed,
                1728,
            );
            s.evasion = Evasion::Reflex;
            s.facing = facing;
            s.phase = phase;
            specs.push(s);
            for o in policy::JUMP_OFFSETS {
                specs.push(RunSpec {
                    policy: PolicyId::StationaryJumpCast,
                    jump_offset: o,
                    ..s
                });
            }
        }
    }
    let records = run_all(kits(), &specs);
    assert_eq!(records.len(), specs.len());
    for r in &records {
        assert_eq!(r.llama_final_hp[1], 3, "{}: A was damaged", r.run_id);
        assert_eq!(r.failures.dock_hit_from_floor, 0, "{}", r.run_id);
        assert!(!r.both_llamas_defeated(), "{}", r.run_id);
    }
    assert!(
        records
            .iter()
            .any(|r| r.jump_presses > 0 && r.bolts_released > 0),
        "the jump-cast really jumped and cast"
    );
}

/// The asserted CORRAL-C gate (spec section 11, gate items 1 to 5) over the
/// gate sweep: every named start, both facings, P0..P2, the decisive seat
/// with the reflex on, every gated policy (the jump-cast at `aimed`, 0, 5 and
/// 45), the M1 control and the movement and co-op runs.
#[test]
fn corral_auto_the_corral_gate_passes() {
    let records = gate_records();
    let config = SweepConfig::gate();
    let gate = Gate::evaluate(records);
    println!("{}", report_md(records, &config, None));
    assert_eq!(
        gate.corral_kc19,
        Outcome::Pass,
        "{:?}",
        gate.corral_best_counterexample
    );
    assert!(gate.corral_groups >= 12 * 2 * 8, "{}", gate.corral_groups);
    assert_eq!(
        gate.corral_invariant,
        Outcome::Pass,
        "{:?}",
        gate.corral_invariant_breaches
    );
    assert_eq!(gate.corral_mobile_win, Outcome::Pass, "mobile Donut");
    assert_eq!(gate.corral_carl_melee_win, Outcome::Pass, "Carl melee");
    assert_eq!(gate.corral_coop_win, Outcome::Pass, "2P co-op");
    assert_eq!(
        gate.corral_system_failures,
        Outcome::Pass,
        "{:?}",
        gate.corral_failure_runs
    );
    assert!(gate.corral_passes());
    // The report machinery itself: well-formed CSV, one row per run.
    let csv = runs_csv(records);
    assert_eq!(csv.lines().count(), records.len() + 1);
    let columns = csv.lines().next().unwrap().split(',').count();
    assert!(csv.lines().all(|l| l.split(',').count() == columns));
}

/// The rush is real in the sweep: M2 starts see rush tells, commits and
/// contacts, and the bumper top (M2-12) is the stall spot of the sheet: 0
/// defeats, 0 damage.
#[test]
fn corral_auto_the_rush_runs_and_the_bumper_top_is_a_stall_spot() {
    let records = gate_records();
    let m2: Vec<&RunRecord> = records
        .iter()
        .filter(|r| r.spec.stage == Stage::CorralM2)
        .collect();
    assert!(m2.iter().map(|r| r.rush_tells).sum::<u32>() > 0);
    assert!(m2.iter().map(|r| r.rush_commits).sum::<u32>() > 0);
    assert!(m2.iter().map(|r| r.rush_contacts).sum::<u32>() > 0);
    for r in m2
        .iter()
        .filter(|r| r.spec.donut_rise == 80 && r.spec.policy.is_stationary_gate())
    {
        assert_eq!(r.llamas_defeated, 0, "{}", r.run_id);
        assert_eq!(r.donut_hp_lost(), 0, "{}", r.run_id);
    }
}

/// Each counterexample-class run and one mobile success run twice in process
/// with identical event order and final hash (spec section 14.4).
#[test]
fn corral_auto_double_run_equality() {
    let mut cases = vec![
        spec(Stage::M2, Seat::SoloDonutBot, PolicyId::StationaryCast, 420),
        spec(Stage::M2, Seat::TwoPlayer, PolicyId::CoopProbe, 420),
    ];
    for (policy, seat, x, rise) in [
        (
            PolicyId::StationaryJumpCastAimed,
            Seat::SoloDonutParked,
            1728,
            0,
        ),
        (PolicyId::StationaryTurret, Seat::SoloDonutParked, 1904, 32),
        (PolicyId::MobileCast, Seat::SoloDonutParked, 1072, 0),
        (PolicyId::CarlMelee, Seat::SoloCarl, 1072, 0),
        (PolicyId::CoopProbe, Seat::TwoPlayer, 1904, 32),
        (PolicyId::StationaryCast, Seat::SoloDonutAssist, 1424, 0),
    ] {
        let mut s = spec(Stage::CorralM2, seat, policy, x);
        s.donut_rise = rise;
        s.evasion = Evasion::Reflex;
        cases.push(s);
    }
    for s in cases {
        let (a, b) = (run(kits(), &s), run(kits(), &s));
        assert_eq!(a.state_hash, b.state_hash, "{}", a.run_id);
        assert_eq!(a.event_lines, b.event_lines, "{}", a.run_id);
        assert_eq!(a, b, "{}", a.run_id);
        assert!(!a.event_lines.is_empty());
    }
}

#[test]
fn corral_auto_outputs_are_written() {
    let out = std::env::temp_dir().join(format!("corral-eval-test-{}", std::process::id()));
    let summary = evaluate_to(&root(), &out, &SweepConfig::quick()).unwrap();
    for name in ["summary.json", "runs.csv", "report.md"] {
        assert!(out.join(name).is_file(), "{name}");
    }
    let json: serde_json::Value = serde_json::from_str(&summary).unwrap();
    assert_eq!(json["baseline_report_only"]["m1_positive_control"], "pass");
    assert_eq!(json["policy_version"], POLICY_VERSION);
    let _ = std::fs::remove_dir_all(&out);
}

/// F1: the reported best counterexample is the minimum over every clearing run
/// of every failing group, not the first failing group (on the baselines,
/// where today's legacy llamas fail KC19 by design).
#[test]
fn corral_auto_best_counterexample_is_the_global_minimum() {
    let records = sweep(kits(), &SweepConfig::quick());
    let gate = Gate::evaluate(&records);
    let best = gate
        .m2_best_counterexample
        .expect("today's llamas fail KC19");
    let failing: Vec<&str> = gate
        .m2_failing_groups
        .iter()
        .map(|g| g.group_id.as_str())
        .collect();
    let min = records
        .iter()
        .filter(|r| {
            is_low_risk_stationary_clear(r) && failing.contains(&r.spec.group_id().as_str())
        })
        .map(|r| (r.ticks, r.donut_hp_lost(), r.run_id.clone()))
        .min()
        .unwrap();
    assert_eq!((best.ticks, best.donut_hp_lost, best.run_id), min);
}

/// MAG-07 with real globs and rushes (the spec section 10 truth table): B's
/// glob reaches a Donut on the 32 px entry shelf (CORRAL-FU (2): the wire's
/// Donut glob path, `half_w` from her hurtbox), never a floor Donut (it
/// rushes her instead), never a Donut on the dock (the bumper blocks it) and
/// nobody on the bumper top; A's glob band never meets a Donut anywhere.
///
/// A Donut planted on the shelf never brings B into the view, so B stays
/// dormant (CRL-Q24); the shelf case wakes B first: Donut steps down into the
/// pen, then hops back onto the shelf and waits.
#[test]
fn corral_auto_m2_heights_real_globs_and_rushes() {
    let mut shelf = spec(
        Stage::CorralM2,
        Seat::SoloDonutParked,
        PolicyId::NoInteraction,
        1204,
    );
    shelf.donut_rise = 32;
    let mut sim = build(kits(), &shelf).unwrap();
    let mut back_on_shelf = None;
    for t in 1..=900u64 {
        let d = &sim.scene.world.donut_body;
        let feet = d.y - crate::prototype::HERO_HALF_HEIGHT;
        let on_shelf = (feet - (room::FLOOR_WORLD_Y + 32.0)).abs() < 0.5 && d.grounded;
        let mut raw = RawInput::NONE;
        raw.swap = t == 1;
        if t < 40 {
            raw.move_x = 1.0;
        } else if !on_shelf && back_on_shelf.is_none() {
            // Back left; hop at the shelf's face and stop steering over it.
            raw.move_x = if d.x > 1180.0 { -1.0 } else { 0.0 };
            raw.jump = d.grounded && d.x < 1240.0;
        } else {
            back_on_shelf.get_or_insert(t);
        }
        sim.step_slots([raw, RawInput::NONE]);
    }
    assert!(back_on_shelf.is_some(), "she is back on the shelf");
    let spits_at_donut = sim
        .combat
        .events
        .iter()
        .filter(|e| matches!(e, CombatEvent::LlamaSpit { llama: 0, .. }))
        .count();
    let glob_hurts = sim
        .combat
        .events
        .iter()
        .filter(|e| {
            matches!(e, CombatEvent::HeroHurt { hero: HeroId::Donut, source, .. }
                if *source != crate::llama_rush::LLAMA_RUSH_SOURCE_ID)
        })
        .count();
    assert!(spits_at_donut > 0, "B spits at the shelf Donut");
    assert!(glob_hurts > 0, "and its glob hits her");
    for rec in gate_records()
        .iter()
        .filter(|r| r.spec.stage == Stage::CorralM2 && r.spec.policy.is_stationary_gate())
    {
        let on_shelf = rec.spec.donut_rise == 32 && rec.spec.donut_x < 1300;
        // A floor start near the shelf can be knocked onto it by a rush (then
        // she is a shelf Donut); every other start is never spat at.
        if !on_shelf && (rec.spec.donut_rise != 0 || rec.spec.donut_x >= 1400) {
            assert_eq!(rec.spits, 0, "{}: spat at", rec.run_id);
        }
        if rec.spec.donut_rise == 32 && rec.spec.donut_x > 1888 {
            assert_eq!(
                rec.llama_final_hp[0], 3,
                "{}: B hit from the dock",
                rec.run_id
            );
        }
        if on_shelf || rec.spec.donut_rise == 80 {
            assert_eq!(
                rec.donut_rush_hurts, 0,
                "{}: rushed above the floor",
                rec.run_id
            );
        }
    }
}
