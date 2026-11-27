//! TROG-B `trog_eval` tests: the gate of plan 7.3 as measured on the Test of Strength **under the
//! declared static fallback** (TROG-AD12, TROG-AD43), the setup-only rule, the determinism of the
//! report and the positive control.

use super::*;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn one(
    stage: Stage,
    seat: Seat,
    policy: EvalPolicy,
    start: Start,
    facing: i8,
    phase: usize,
) -> RunSpec {
    RunSpec {
        stage,
        seat,
        policy,
        evasion: Evasion::Reflex,
        start,
        facing,
        phase,
        jump_offset: 0,
        horizon: HORIZON_TICKS,
    }
}

/// The report of a sweep, run once and shared by the gate tests.
fn quick() -> &'static report::Evaluation {
    use std::sync::OnceLock;
    static QUICK: OnceLock<report::Evaluation> = OnceLock::new();
    QUICK.get_or_init(|| evaluate(&root(), &SweepConfig::quick()))
}

/// G-1 and G-2 hold for every **fixed** gated stationary policy (the cast, the burst, the aimed
/// cast, the turret and the aimed jump-cast) from the five natural starts, both facings, three
/// phases, both reflex variants and both seats: none defeats both troglodytes.
#[test]
fn trog_gate_holds_for_the_fixed_stationary_policies() {
    let e = quick();
    for id in ["G-1", "G-2", "G-4", "G-5"] {
        let row = e.gate.row(id).expect("the row");
        assert_eq!(row.passes, Some(true), "{id}: {}", row.detail);
    }
    // The Virtuoso falls to no natural-start fixed policy: from the spawn nothing is in range,
    // and the other starts are on the shelf, on the bench or past it.
    for r in e.recs().filter(|r| r.spec.policy.is_stationary_gate()) {
        assert!(!r.all_defeated_by_player(), "{}", r.run_id);
        assert!(
            !r.kill_by_player[0],
            "{}: the Virtuoso fell to Donut",
            r.run_id
        );
        assert_ne!(r.terminal, Terminal::InvalidSetup, "{}", r.run_id);
    }
}

/// G-4: the mobile Donut, Carl melee and the co-op probe each clear the Test of Strength in
/// every phase variant, with no party defeat and the bench standing (it is a static).
#[test]
fn trog_gate_movement_rows_clear_the_room() {
    let e = quick();
    let moves: Vec<_> = e
        .recs()
        .filter(|r| r.spec.policy.movement_seat().is_some())
        .collect();
    assert_eq!(moves.len(), 9);
    for r in moves {
        assert_eq!(r.terminal, Terminal::Victory, "{}", r.run_id);
        assert!(r.ticks <= MOVE_HORIZON_TICKS);
        assert_eq!(r.donut_hp_lost().max(r.carl_hp_lost()), 0, "{}", r.run_id);
    }
}

/// **G-1 and G-2 hold with the 80 px static bench** (TROG-AD46). Under the 48 px static of
/// TROG-AD43 the swept `stationary_jump_cast` at offsets 0 and 1 from the far landing (x 720)
/// defeated both troglodytes with no damage taken (history: a bolt released at feet 32..54 flew
/// over a 48 px bench and still met a hurtbox, tops 58..70). With the bench at 80 px a bolt below
/// feet 64 stops on it and one from 64 up flies at 80 or above, over every hurtbox. The same 12
/// runs now defeat at most one troglodyte, and the whole decisive sweep (natural starts, every
/// gated policy, the swept jump-cast, both facings, three phases, both seats and reflex variants)
/// passes every row it covers.
#[test]
fn trog_gate_hard_invariant_holds_with_the_80_px_bench() {
    let mut specs = Vec::new();
    for o in [0i8, 1] {
        for facing in [1i8, -1] {
            for phase in 0..3 {
                let mut s = one(
                    Stage::Strength,
                    Seat::Isolated,
                    EvalPolicy::Stationary(PolicyId::StationaryJumpCast),
                    at(720, 0),
                    facing,
                    phase,
                );
                s.jump_offset = o;
                specs.push(s);
            }
        }
    }
    let runs = run_all(&root(), &specs, true);
    assert_eq!(runs.len(), 12);
    for c in &runs {
        let r = &c.record;
        assert!(!r.all_defeated_by_player(), "{}: both defeated", r.run_id);
        assert_eq!(c.identical, Some(true), "{}: the double run", r.run_id);
    }
    // The bench-top start exists at its sidecar height (no invalid setup).
    let top = run(
        &root(),
        &one(
            Stage::Strength,
            Seat::Isolated,
            EvalPolicy::Stationary(PolicyId::StationaryCastAimed),
            at(656, BENCH_RISE_PX),
            1,
            0,
        ),
    );
    assert_ne!(top.terminal, Terminal::InvalidSetup, "{}", top.note);
    // The whole gated sweep with the jump-cast offsets.
    let judged = evaluate(
        &root(),
        &SweepConfig {
            strength: true,
            pockets: false,
            grid: false,
            jump_sweep: true,
            double_run: true,
        },
    );
    for id in ["G-1", "G-2", "G-4", "G-5"] {
        let row = judged.gate.row(id).unwrap();
        assert_eq!(row.passes, Some(true), "{id}: {}", row.detail);
    }
}

/// The positive control (TROG-F02): a planted Donut defeats the T1 Pygmy from x 432 with at most
/// 1 HP lost, as the fixture and as an evaluator run.
#[test]
fn trog_gate_positive_control_is_the_pygmy() {
    assert!(positive_control_fixture(&root()).unwrap());
    for phase in 0..3 {
        let r = run(
            &root(),
            &one(
                Stage::T1,
                Seat::Isolated,
                EvalPolicy::Stationary(PolicyId::StationaryCastAimed),
                at(432, 0),
                1,
                phase,
            ),
        );
        assert!(r.all_defeated(), "{}: {}", r.run_id, r.terminal.name());
        assert!(r.donut_hp_lost() <= 1, "{}", r.run_id);
    }
}

/// The report is a pure function of the records: two sweeps write the same bytes, in a fixed
/// order, and the sweep is the same whatever the worker count.
#[test]
fn trog_eval_report_is_deterministic() {
    let again = evaluate(&root(), &SweepConfig::quick());
    assert_eq!(again.json(), quick().json());
    assert_eq!(again.text(), quick().text());
}

/// Setup-only writes: a stationary run with nothing in reach never moves Donut, and a start
/// inside a solid (the bench's own column on the floor) is refused as `invalid_setup`.
#[test]
fn trog_eval_writes_game_state_only_at_setup() {
    let r = run(
        &root(),
        &one(
            Stage::Strength,
            Seat::Isolated,
            EvalPolicy::Stationary(PolicyId::StationaryCast),
            at(80, 0),
            1,
            0,
        ),
    );
    assert_eq!(r.terminal, Terminal::Timeout);
    assert_eq!(r.donut_displacement_px, 0.0, "she never moved");
    assert_eq!(r.donut_start, r.donut_final);
    let inside = run(
        &root(),
        &one(
            Stage::Strength,
            Seat::Isolated,
            EvalPolicy::Stationary(PolicyId::StationaryCast),
            at(656, 0),
            1,
            0,
        ),
    );
    assert_eq!(inside.terminal, Terminal::InvalidSetup, "{}", inside.note);
}

/// The phase variant is the roster's `delay_ticks`, written at build.
#[test]
fn the_phase_variants_are_the_rosters_delay() {
    let spec = one(
        Stage::Strength,
        Seat::Isolated,
        EvalPolicy::Stationary(PolicyId::StationaryCast),
        at(80, 0),
        1,
        1,
    );
    let sim = build(&root(), &spec).unwrap();
    let delays: Vec<u32> = sim
        .run
        .loaded
        .encounter
        .roster
        .iter()
        .map(|e| e.delay_ticks)
        .collect();
    assert_eq!(delays, [30, 60]);
}

/// Review R1: a natural-start run that never ran (`invalid_setup`) fails G-1 and G-2 and the
/// count is shown, instead of passing vacuously. Here the shelf-top start is made illegal by a
/// bench-sized start inside a solid, relabelled as a natural start.
#[test]
fn trog_gate_fails_when_a_natural_start_is_invalid() {
    let mut s = one(
        Stage::Strength,
        Seat::Isolated,
        EvalPolicy::Stationary(PolicyId::StationaryCast),
        at(656, 0),
        1,
        0,
    );
    let mut record = run(&root(), &s);
    assert_eq!(record.terminal, Terminal::InvalidSetup, "{}", record.note);
    // Relabel it as the natural start it stands in for (a sidecar edit that moved the bench
    // would make (656, 80) fail exactly this way).
    s.start = at(656, BENCH_RISE_PX);
    record.spec = s;
    let judged = report::Evaluation::judge(
        &SweepConfig {
            strength: true,
            pockets: false,
            grid: false,
            jump_sweep: false,
            double_run: false,
        },
        vec![Checked {
            record,
            identical: None,
        }],
    );
    let g1 = judged.gate.row("G-1").unwrap();
    assert_eq!(g1.passes, Some(false), "{}", g1.detail);
    assert!(
        g1.detail.contains("1 of 1") && g1.detail.contains("invalid_setup"),
        "{}",
        g1.detail
    );
    let g2 = judged.gate.row("G-2").unwrap();
    assert_eq!(g2.passes, Some(false), "{}", g2.detail);
    assert!(g2.detail.contains("1 invalid_setup"), "{}", g2.detail);
    assert!(judged.text().contains("invalid_setup"));
    // The shipped sweep reports zero.
    let g1 = quick().gate.row("G-1").unwrap();
    assert!(g1.detail.contains("0 invalid_setup"), "{}", g1.detail);
}

/// Review R8: the quick sweep does not run the positive control, and its header says so.
#[test]
fn the_quick_report_says_the_positive_control_was_not_run() {
    let text = quick().text();
    assert!(
        text.lines()
            .next()
            .unwrap()
            .contains("PASS (partial: a row was not run)"),
        "{}",
        text.lines().next().unwrap()
    );
}
