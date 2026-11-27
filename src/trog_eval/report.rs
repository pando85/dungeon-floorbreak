//! The judged sweep of `trog_eval`: the gate of plan 7.3 asserted over the records, and the JSON
//! and text tables. Everything here is a pure function of the records, in a fixed order, with no
//! clock, so two sweeps write the same bytes.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{Value, json};

use super::{
    Checked, EvalPolicy, HORIZON_TICKS, MOVE_HORIZON_TICKS, PHASES, POLICY_VERSION, RunRecord,
    Seat, Stage, SweepConfig, Terminal,
};
use crate::corral_eval::Evasion;
use crate::corral_eval::policy::PolicyId;

/// One row of the gate.
#[derive(Debug, Clone, PartialEq)]
pub struct GateRow {
    pub id: &'static str,
    pub name: &'static str,
    /// `None`: the row needs a stage the sweep did not run.
    pub passes: Option<bool>,
    pub detail: String,
}

/// The gate of plan 7.3.
#[derive(Debug, Clone, PartialEq)]
pub struct Gate {
    pub rows: Vec<GateRow>,
}

impl Gate {
    /// Every row that ran passes.
    pub fn passes(&self) -> bool {
        self.rows.iter().all(|r| r.passes != Some(false))
    }

    pub fn row(&self, id: &str) -> Option<&GateRow> {
        self.rows.iter().find(|r| r.id == id)
    }
}

/// The judged sweep.
#[derive(Debug, Clone)]
pub struct Evaluation {
    pub config: SweepConfig,
    pub records: Vec<Checked>,
    pub gate: Gate,
}

/// The decisive variant: the isolated seat (Carl parked) with the shipped reflex.
fn decisive(r: &RunRecord) -> bool {
    r.spec.seat == Seat::Isolated && r.spec.evasion == Evasion::Reflex
}

/// A natural-start gated stationary run on the Test of Strength: the rows 1 and 2 population.
fn gated_stationary(r: &RunRecord, naturals: &[super::Start]) -> bool {
    r.spec.stage == Stage::Strength
        && r.spec.policy.is_stationary_gate()
        && decisive(r)
        && naturals.contains(&r.spec.start)
}

/// A start group in which nothing happened in any phase: no defeat, no tell, no damage (a
/// planted Donut out of everything's reach). Reported as a count, not as rows.
fn quiet(runs: &[&RunRecord]) -> bool {
    runs.iter().all(|r| {
        r.defeated() == 0 && r.tells == 0 && r.donut_hp_lost() == 0 && r.carl_hp_lost() == 0
    })
}

/// How one run reads in a counterexample line.
fn line_of(r: &RunRecord) -> String {
    format!(
        "{} {} ticks={} defeated={}/{} donut_hp_lost={} carl_hp_lost={}",
        r.run_id,
        r.terminal.name(),
        r.ticks,
        r.defeated(),
        r.watched(),
        r.donut_hp_lost(),
        r.carl_hp_lost()
    )
}

impl Evaluation {
    pub fn judge(config: &SweepConfig, records: Vec<Checked>) -> Self {
        let gate = judge_gate(config, &records);
        Self {
            config: config.clone(),
            records,
            gate,
        }
    }

    pub fn recs(&self) -> impl Iterator<Item = &RunRecord> {
        self.records.iter().map(|c| &c.record)
    }

    /// The run groups (every phase of one start) of the records `keep` selects, in id order.
    fn groups(&self, keep: impl Fn(&RunRecord) -> bool) -> BTreeMap<String, Vec<&RunRecord>> {
        let mut out: BTreeMap<String, Vec<&RunRecord>> = BTreeMap::new();
        for r in self.recs().filter(|r| keep(r)) {
            out.entry(r.spec.group_id()).or_default().push(r);
        }
        out
    }

    pub fn json(&self) -> String {
        let naturals = Stage::Strength.natural_starts();
        let gate_rows: Vec<Value> = self
            .gate
            .rows
            .iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "name": r.name,
                    "passes": r.passes,
                    "detail": r.detail,
                })
            })
            .collect();
        let all_groups = self.groups(|r| gated_stationary(r, &naturals));
        let quiet_groups = all_groups.values().filter(|runs| quiet(runs)).count();
        let group_rows: Vec<Value> = all_groups
            .iter()
            .filter(|(_, runs)| !quiet(runs))
            .map(|(id, runs)| {
                json!({
                    "group": id,
                    "phases": runs.iter().map(|r| json!({
                        "run": r.run_id,
                        "terminal": r.terminal.name(),
                        "ticks": r.ticks,
                        "defeated": r.defeated(),
                        "by_player": r.kill_by_player,
                        "donut_hp_lost": r.donut_hp_lost(),
                        "carl_hp_lost": r.carl_hp_lost(),
                        "tells": r.tells,
                        "strikes": r.strikes,
                        "bolt_hits": r.bolt_hits,
                        "final_hp": r.final_hp,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        let movement: Vec<Value> = self
            .recs()
            .filter(|r| r.spec.policy.movement_seat().is_some())
            .map(|r| {
                json!({
                    "run": r.run_id,
                    "terminal": r.terminal.name(),
                    "ticks": r.ticks,
                    "defeated": r.defeated(),
                    "donut_hp_lost": r.donut_hp_lost(),
                    "carl_hp_lost": r.carl_hp_lost(),
                    "tells": r.tells,
                    "strikes": r.strikes,
                    "evades": r.evades,
                })
            })
            .collect();
        let value = json!({
            "policy_version": POLICY_VERSION,
            "horizon_ticks": HORIZON_TICKS,
            "move_horizon_ticks": MOVE_HORIZON_TICKS,
            "phases": PHASES,
            "runs": self.records.len(),
            "gate": { "passes": self.gate.passes(), "rows": gate_rows },
            "gated_groups": group_rows,
            "quiet_groups": quiet_groups,
            "movement_runs": movement,
            "creditors": self.credit_rows(),
            "pockets": self.pocket_rows(),
            "grid": self.grid_rows(),
            "system_failures": self.failure_rows(),
            "companion_assisted_runs": self.recs().filter(|r| r.companion_assisted()).count(),
        });
        serde_json::to_string_pretty(&value).expect("the report serialises") + "\n"
    }

    /// Defeats by troglodyte, by Donut credit, per gated stationary policy in the decisive
    /// variant (the grounded-Donut row of TROG-Q13 and the bench's G1, reported).
    fn credit_rows(&self) -> Vec<Value> {
        let naturals = Stage::Strength.natural_starts();
        let mut by_policy: BTreeMap<String, [u32; 5]> = BTreeMap::new();
        for r in self.recs().filter(|r| gated_stationary(r, &naturals)) {
            let row = by_policy.entry(r.spec.policy.name()).or_default();
            row[0] += 1;
            if r.kill_ticks[0].is_some() && r.kill_by_player[0] {
                row[1] += 1;
            }
            if r.kill_ticks[1].is_some() && r.kill_by_player[1] {
                row[2] += 1;
            }
            if r.all_defeated_by_player() {
                row[3] += 1;
            }
            row[4] += r.tells;
        }
        by_policy
            .into_iter()
            .map(|(policy, [runs, vir, bas, both, tells])| {
                json!({
                    "policy": policy,
                    "runs": runs,
                    "virtuoso_defeated_by_donut": vir,
                    "basher_defeated_by_donut": bas,
                    "both_defeated_by_donut": both,
                    "tells": tells,
                })
            })
            .collect()
    }

    fn pocket_rows(&self) -> Vec<Value> {
        let mut rows: BTreeMap<(Stage, super::Start, String), [u32; 4]> = BTreeMap::new();
        for r in self.recs().filter(|r| !r.spec.stage.gated()) {
            if !decisive(r) {
                continue;
            }
            let row = rows
                .entry((r.spec.stage, r.spec.start, r.spec.policy.name()))
                .or_default();
            row[0] += 1;
            if r.all_defeated() {
                row[1] += 1;
            }
            row[2] += u32::from(r.donut_hp_lost());
            row[3] += r.tells;
        }
        rows.into_iter()
            .map(|((stage, start, policy), [runs, won, hp, tells])| {
                json!({
                    "stage": stage.name(),
                    "start": { "x": start.x, "rise": start.rise },
                    "policy": policy,
                    "runs": runs,
                    "victories": won,
                    "donut_hp_lost_sum": hp,
                    "tells": tells,
                })
            })
            .collect()
    }

    fn grid_rows(&self) -> Value {
        let naturals = Stage::Strength.natural_starts();
        let diagnostics = Stage::Strength.diagnostic_starts();
        let mut runs = 0u32;
        let (mut virtuoso, mut basher, mut both) = (0u32, 0u32, 0u32);
        let mut best: Option<&RunRecord> = None;
        for r in self.recs().filter(|r| {
            r.spec.stage == Stage::Strength
                && decisive(r)
                && r.spec.policy.is_stationary_gate()
                && diagnostics.contains(&r.spec.start)
                && !naturals.contains(&r.spec.start)
        }) {
            if r.terminal == Terminal::InvalidSetup {
                continue;
            }
            runs += 1;
            virtuoso += u32::from(r.kill_ticks[0].is_some() && r.kill_by_player[0]);
            basher += u32::from(r.kill_ticks[1].is_some() && r.kill_by_player[1]);
            if r.all_defeated_by_player() {
                both += 1;
                if best.is_none_or(|b| r.donut_hp_lost() < b.donut_hp_lost()) {
                    best = Some(r);
                }
            }
        }
        json!({
            "runs": runs,
            "virtuoso_defeated_by_donut": virtuoso,
            "basher_defeated_by_donut": basher,
            "both_defeated_by_donut": both,
            "best_counterexample": best.map(line_of),
        })
    }

    fn failure_rows(&self) -> Value {
        let mut total = super::SystemFailures::default();
        for r in self.recs() {
            total.blow_before_window += r.failures.blow_before_window;
            total.tell_in_protected += r.failures.tell_in_protected;
            total.offscreen += r.failures.offscreen;
            total.unreachable_target += r.failures.unreachable_target;
            total.outside_leash += r.failures.outside_leash;
        }
        let mismatches = self
            .records
            .iter()
            .filter(|c| c.identical == Some(false))
            .count();
        json!({
            "blow_before_window": total.blow_before_window,
            "tell_in_protected": total.tell_in_protected,
            "offscreen": total.offscreen,
            "unreachable_target": total.unreachable_target,
            "outside_leash": total.outside_leash,
            "double_run_mismatches": mismatches,
            "double_runs": self.records.iter().filter(|c| c.identical.is_some()).count(),
        })
    }

    /// The text table.
    pub fn text(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "trog_eval policy_version={POLICY_VERSION} runs={} gate={}",
            self.records.len(),
            if self.gate.passes() { "PASS" } else { "FAIL" }
        );
        let _ = writeln!(out, "\nGATE (plan 7.3, the Test of Strength)");
        for r in &self.gate.rows {
            let verdict = match r.passes {
                Some(true) => "PASS",
                Some(false) => "FAIL",
                None => "NOT RUN",
            };
            let _ = writeln!(out, "  {} {:<24} {}  {}", r.id, r.name, verdict, r.detail);
        }
        let _ = writeln!(
            out,
            "\nGATED GROUPS (natural starts, decisive variant: isolated seat, reflex on)"
        );
        let _ = writeln!(
            out,
            "  group | phases (terminal, defeated, donut hp lost, tells)"
        );
        let naturals = Stage::Strength.natural_starts();
        let all_groups = self.groups(|r| gated_stationary(r, &naturals));
        let quiet_groups = all_groups.values().filter(|runs| quiet(runs)).count();
        let _ = writeln!(
            out,
            "  {quiet_groups} of {} groups are quiet (no defeat, no tell, no damage in any phase) and are not listed",
            all_groups.len()
        );
        for (id, runs) in all_groups.iter().filter(|(_, runs)| !quiet(runs)) {
            let cells: Vec<String> = runs
                .iter()
                .map(|r| {
                    format!(
                        "{}/{}/{}/{}",
                        r.terminal.name(),
                        r.defeated(),
                        r.donut_hp_lost(),
                        r.tells
                    )
                })
                .collect();
            let _ = writeln!(out, "  {id} | {}", cells.join("  "));
        }
        let _ = writeln!(out, "\nMOVEMENT ROWS (reflex on, 2400 tick limit)");
        for r in self
            .recs()
            .filter(|r| r.spec.policy.movement_seat().is_some())
        {
            let _ = writeln!(
                out,
                "  {} {} ticks={} defeated={}/{} donut_hp_lost={} carl_hp_lost={} tells={} strikes={} evades={}",
                r.run_id,
                r.terminal.name(),
                r.ticks,
                r.defeated(),
                r.watched(),
                r.donut_hp_lost(),
                r.carl_hp_lost(),
                r.tells,
                r.strikes,
                r.evades
            );
        }
        let _ = writeln!(
            out,
            "\nCREDIT (gated stationary policies, natural starts, decisive variant; TROG-Q13 and G1)"
        );
        for row in self.credit_rows() {
            let _ = writeln!(
                out,
                "  {:<28} runs={} virtuoso_by_donut={} basher_by_donut={} both_by_donut={} tells={}",
                row["policy"].as_str().unwrap_or(""),
                row["runs"],
                row["virtuoso_defeated_by_donut"],
                row["basher_defeated_by_donut"],
                row["both_defeated_by_donut"],
                row["tells"]
            );
        }
        if self.config.pockets {
            let _ = writeln!(
                out,
                "\nPOCKETS (reported, not gated; decisive variant, all stationary policies)"
            );
            for row in self.pocket_rows() {
                let _ = writeln!(
                    out,
                    "  {} x={} rise={} {:<28} runs={} victories={} donut_hp_lost_sum={} tells={}",
                    row["stage"].as_str().unwrap_or(""),
                    row["start"]["x"],
                    row["start"]["rise"],
                    row["policy"].as_str().unwrap_or(""),
                    row["runs"],
                    row["victories"],
                    row["donut_hp_lost_sum"],
                    row["tells"]
                );
            }
        }
        if self.config.grid {
            let g = self.grid_rows();
            let _ = writeln!(
                out,
                "\nGRID (16 px floor grid 240..928 plus the shelf and bench tops; reported, not gated)\n  runs={} virtuoso_by_donut={} basher_by_donut={} both_by_donut={} best_counterexample={}",
                g["runs"],
                g["virtuoso_defeated_by_donut"],
                g["basher_defeated_by_donut"],
                g["both_defeated_by_donut"],
                g["best_counterexample"]
            );
        }
        let f = self.failure_rows();
        let _ = writeln!(
            out,
            "\nSYSTEM FAILURES (hard)\n  blow_before_window={} tell_in_protected={} offscreen={} unreachable_target={} outside_leash={} double_run_mismatches={} of {} double runs",
            f["blow_before_window"],
            f["tell_in_protected"],
            f["offscreen"],
            f["unreachable_target"],
            f["outside_leash"],
            f["double_run_mismatches"],
            f["double_runs"]
        );
        out
    }
}

fn judge_gate(config: &SweepConfig, records: &[Checked]) -> Gate {
    let recs: Vec<&RunRecord> = records.iter().map(|c| &c.record).collect();
    let naturals = Stage::Strength.natural_starts();
    let mut rows = Vec::new();

    // Row 1: the hard invariant.
    let population: Vec<&&RunRecord> = recs
        .iter()
        .filter(|r| gated_stationary(r, &naturals))
        .collect();
    let counterexamples: Vec<&&RunRecord> = population
        .iter()
        .copied()
        .filter(|r| r.all_defeated_by_player())
        .collect();
    let row1 = if !config.strength {
        GateRow {
            id: "G-1",
            name: "hard_invariant",
            passes: None,
            detail: "the Test of Strength was not run".into(),
        }
    } else if counterexamples.is_empty() {
        GateRow {
            id: "G-1",
            name: "hard_invariant",
            passes: Some(true),
            detail: format!(
                "{} runs, none defeated both troglodytes by a Donut-credited hit",
                population.len()
            ),
        }
    } else {
        let best = counterexamples
            .iter()
            .min_by_key(|r| (r.donut_hp_lost(), r.ticks))
            .expect("one");
        GateRow {
            id: "G-1",
            name: "hard_invariant",
            passes: Some(false),
            detail: format!(
                "{} of {} runs defeated both; best counterexample {}",
                counterexamples.len(),
                population.len(),
                line_of(best)
            ),
        }
    };
    rows.push(row1);

    // Row 2: KC19, by start group.
    let mut groups: BTreeMap<String, Vec<&RunRecord>> = BTreeMap::new();
    for r in &population {
        groups.entry(r.spec.group_id()).or_default().push(**r);
    }
    let kc19_fail: Vec<&String> = groups
        .iter()
        .filter(|(_, runs)| {
            let good = runs
                .iter()
                .filter(|r| r.all_defeated() && r.ticks <= HORIZON_TICKS && r.donut_hp_lost() <= 1)
                .count();
            good * 2 >= runs.len()
        })
        .map(|(id, _)| id)
        .collect();
    rows.push(GateRow {
        id: "G-2",
        name: "kc19",
        passes: config.strength.then_some(kc19_fail.is_empty()),
        detail: if config.strength {
            format!(
                "{} start groups; {} cleared both within {HORIZON_TICKS} ticks with at most 1 HP lost in half or more of the phases",
                groups.len(),
                kc19_fail.len()
            )
        } else {
            "the Test of Strength was not run".into()
        },
    });

    // Row 3: the positive control (T1, the Pygmy).
    let control: Vec<&&RunRecord> = recs
        .iter()
        .filter(|r| {
            r.spec.stage == Stage::T1
                && decisive(r)
                && r.spec.start.x == 432
                && r.spec.facing == 1
                && r.spec.policy == EvalPolicy::Stationary(PolicyId::StationaryCastAimed)
        })
        .collect();
    rows.push(GateRow {
        id: "G-3",
        name: "positive_control",
        passes: (!control.is_empty()).then(|| {
            control
                .iter()
                .all(|r| r.all_defeated() && r.donut_hp_lost() <= 1)
        }),
        detail: if control.is_empty() {
            "the run's pockets were not run (TROG-F02 asserts the fixture)".into()
        } else {
            format!(
                "T1 from x 432, aimed stationary cast, {} phases: {}",
                control.len(),
                control
                    .iter()
                    .map(|r| format!("{}/{}hp", r.terminal.name(), r.donut_hp_lost()))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        },
    });

    // Row 4: movement.
    let mut detail = Vec::new();
    let mut ok = true;
    let mut any = false;
    for policy in [
        EvalPolicy::MobileCast,
        EvalPolicy::CarlMelee,
        EvalPolicy::CoopProbe,
    ] {
        let runs: Vec<&&RunRecord> = recs.iter().filter(|r| r.spec.policy == policy).collect();
        if runs.is_empty() {
            continue;
        }
        any = true;
        let good = runs
            .iter()
            .filter(|r| r.terminal == Terminal::Victory && r.ticks <= MOVE_HORIZON_TICKS)
            .count();
        let pass = good * 2 >= runs.len();
        ok &= pass;
        detail.push(format!("{} {}/{} cleared", policy.name(), good, runs.len()));
    }
    rows.push(GateRow {
        id: "G-4",
        name: "movement_success",
        passes: any.then_some(ok),
        detail: if any {
            detail.join(", ")
        } else {
            "the Test of Strength was not run".into()
        },
    });

    // Row 5: the hard system failures.
    let mut total = 0;
    for r in &recs {
        total += r.failures.total();
    }
    let mismatches = records
        .iter()
        .filter(|c| c.identical == Some(false))
        .count();
    let doubles = records.iter().filter(|c| c.identical.is_some()).count();
    rows.push(GateRow {
        id: "G-5",
        name: "system_failures",
        passes: Some(total == 0 && mismatches == 0),
        detail: format!(
            "{total} failures over {} runs; {mismatches} of {doubles} double runs differ",
            recs.len()
        ),
    });
    Gate { rows }
}
