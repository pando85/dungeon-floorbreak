//! CORRAL-EVAL outputs and the gate logic: `summary.json`, `runs.csv` and the
//! `report.md` generated from the data. Pure functions of [`RunRecord`]s.
//!
//! Outcomes are `pass`, `fail`, `blocked` or `not_run`; `blocked` is never a
//! `pass`. Which gates are *asserted* by tests and which are *report-only* is
//! stated in the module docs of [`super`] (F13).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{Value, json};

use super::policy::PolicyId;
use super::{
    Evasion, HORIZON_TICKS, POLICY_VERSION, RunRecord, Seat, Stage, SweepConfig, Terminal,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    Fail,
    Blocked,
    NotRun,
}

impl Outcome {
    pub const fn name(self) -> &'static str {
        match self {
            Outcome::Pass => "pass",
            Outcome::Fail => "fail",
            Outcome::Blocked => "blocked",
            Outcome::NotRun => "not_run",
        }
    }
}

/// KC19: the tested hero may lose at most this much of 4 HP and still count
/// as a low-risk clear.
pub const LOW_RISK_HP_LOSS: u8 = 1;

/// A stationary run that clears the stage at low risk by Donut alone: the
/// decisive KC19 event. A run in which Carl's AI dealt any damage is
/// companion-assisted and never counts as proof.
///
/// The 1,800-tick limit is the KC19 definition, not the sweep's `--horizon`:
/// a longer horizon never widens what counts as a clear.
pub fn is_low_risk_stationary_clear(r: &RunRecord) -> bool {
    r.spec.policy.is_stationary_gate()
        && r.spec.seat == Seat::SoloDonutBot
        && r.terminal == Terminal::Victory
        && r.ticks <= HORIZON_TICKS
        && r.donut_hp_lost() <= LOW_RISK_HP_LOSS
        && !r.companion_assisted()
}

/// The verdict on one start group (every phase variant of one start).
#[derive(Debug, Clone, PartialEq)]
pub struct GroupVerdict {
    pub group_id: String,
    pub runs: usize,
    pub clears: usize,
    /// KC19: every phase variant clears, or at least half of them do.
    pub fails: bool,
    /// The clearing run with the fewest ticks and then the least HP lost.
    pub best_counterexample: Option<String>,
    /// Its `(ticks, Donut HP lost)`, for ranking across groups.
    pub best_key: Option<(u64, u8)>,
}

/// Group the records of `stages` (evasion `evasion`, the decisive Donut-bot
/// seat, the two stationary policies) and judge each group by KC19.
pub fn stationary_groups(
    records: &[RunRecord],
    stages: &[Stage],
    evasion: Evasion,
) -> Vec<GroupVerdict> {
    // Keyed with the column as a number, so `x1000` sorts after `x240`.
    type Key = (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        i32,
        i8,
    );
    let mut groups: BTreeMap<Key, Vec<&RunRecord>> = BTreeMap::new();
    for r in records {
        if stages.contains(&r.spec.stage)
            && r.spec.evasion == evasion
            && r.spec.seat == Seat::SoloDonutBot
            && r.spec.policy.is_stationary_gate()
            && !matches!(r.terminal, Terminal::Blocked | Terminal::InvalidSetup)
        {
            let key = (
                r.spec.stage.name(),
                r.spec.seat.name(),
                r.spec.policy.name(),
                r.spec.evasion.name(),
                r.spec.donut_x,
                r.spec.facing,
            );
            groups.entry(key).or_default().push(r);
        }
    }
    groups
        .into_values()
        .map(|runs| {
            let group_id = runs[0].spec.group_id();
            let clears: Vec<&&RunRecord> = runs
                .iter()
                .filter(|r| is_low_risk_stationary_clear(r))
                .collect();
            let best = clears
                .iter()
                .min_by_key(|r| (r.ticks, r.donut_hp_lost(), r.run_id.as_str()));
            GroupVerdict {
                group_id,
                runs: runs.len(),
                clears: clears.len(),
                fails: !clears.is_empty() && clears.len() * 2 >= runs.len(),
                best_counterexample: best.map(|r| r.run_id.clone()),
                best_key: best.map(|r| (r.ticks, r.donut_hp_lost())),
            }
        })
        .collect()
}

/// A run that defeats the gate, with the numbers it is ranked by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counterexample {
    pub run_id: String,
    pub ticks: u64,
    pub donut_hp_lost: u8,
}

/// The gates of one sweep.
#[derive(Debug, Clone)]
pub struct Gate {
    /// M1 positive control: a stationary Donut wins M1 (asserted).
    pub m1_control: Outcome,
    pub m1_control_best: Option<String>,
    /// KC19 on M2 and the pincer, decided on `evasion = off` today
    /// (REPORT-ONLY until CORRAL-C).
    pub m2_stationary: Outcome,
    pub m2_failing_groups: Vec<GroupVerdict>,
    pub m2_groups: usize,
    /// The best counterexample over all failing groups: the clearing run
    /// with the fewest ticks, then the least Donut HP lost (then run id).
    pub m2_best_counterexample: Option<Counterexample>,
    /// The same gate with the reflex on: blocked until DM-B.
    pub m2_stationary_reflex: Outcome,
    /// A mobile Donut / Carl melee / 2P victory exists on an M2 stage
    /// (REPORT-ONLY until CORRAL-C).
    pub mobile_win: Outcome,
    pub carl_melee_win: Outcome,
    pub coop_win: Outcome,
    /// `hold_attack_trivial` never produced more than the one press.
    pub trivial_hold_is_one_press: Outcome,
    /// Every run's mana arithmetic closes (asserted).
    pub mana_accounting: Outcome,
}

fn any_win(records: &[RunRecord], stages: &[Stage], seat: Seat, policy: PolicyId) -> Outcome {
    let mut seen = false;
    for r in records.iter().filter(|r| {
        stages.contains(&r.spec.stage)
            && r.spec.seat == seat
            && r.spec.policy == policy
            && r.spec.evasion == Evasion::Off
    }) {
        seen = true;
        if r.terminal == Terminal::Victory {
            return Outcome::Pass;
        }
    }
    if seen { Outcome::Fail } else { Outcome::NotRun }
}

impl Gate {
    pub fn evaluate(records: &[RunRecord]) -> Self {
        let m2 = [Stage::M2, Stage::M2Pincer];
        let m1_best = records
            .iter()
            .filter(|r| r.spec.stage == Stage::M1 && is_low_risk_stationary_clear(r))
            .min_by_key(|r| (r.ticks, r.donut_hp_lost()))
            .map(|r| r.run_id.clone());
        let m1_ran = records.iter().any(|r| {
            r.spec.stage == Stage::M1
                && r.spec.policy.is_stationary_gate()
                && r.terminal != Terminal::Blocked
        });
        let m1_control = match (m1_ran, &m1_best) {
            (false, _) => Outcome::NotRun,
            (true, Some(_)) => Outcome::Pass,
            (true, None) => Outcome::Fail,
        };
        let groups = stationary_groups(records, &m2, Evasion::Off);
        let failing: Vec<GroupVerdict> = groups.iter().filter(|g| g.fails).cloned().collect();
        let m2_stationary = if groups.is_empty() {
            Outcome::NotRun
        } else if failing.is_empty() {
            Outcome::Pass
        } else {
            Outcome::Fail
        };
        let reflex_ran = records
            .iter()
            .any(|r| r.spec.evasion == Evasion::Reflex && r.terminal != Terminal::Blocked);
        let reflex_listed = records.iter().any(|r| r.spec.evasion == Evasion::Reflex);
        let m2_stationary_reflex = if reflex_ran {
            let g = stationary_groups(records, &m2, Evasion::Reflex);
            if g.is_empty() {
                // Runs exist but no stationary group formed: not a pass.
                Outcome::NotRun
            } else if g.iter().any(|x| x.fails) {
                Outcome::Fail
            } else {
                Outcome::Pass
            }
        } else if reflex_listed {
            Outcome::Blocked
        } else {
            Outcome::NotRun
        };
        let hold: Vec<&RunRecord> = records
            .iter()
            .filter(|r| r.spec.policy == PolicyId::HoldAttackTrivial)
            .collect();
        let trivial = if hold.is_empty() {
            Outcome::NotRun
        } else if hold.iter().all(|r| r.presses <= 1 && r.bolts_released <= 1) {
            Outcome::Pass
        } else {
            Outcome::Fail
        };
        let live: Vec<&RunRecord> = records
            .iter()
            .filter(|r| !matches!(r.terminal, Terminal::Blocked | Terminal::InvalidSetup))
            .collect();
        let accounting = if live.is_empty() {
            Outcome::NotRun
        } else if live.iter().all(|r| r.mana_accounting_closes()) {
            Outcome::Pass
        } else {
            Outcome::Fail
        };
        let best = failing
            .iter()
            .filter_map(|g| Some((g.best_key?, g.best_counterexample.as_ref()?)))
            .min_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)))
            .map(|((ticks, hp), id)| Counterexample {
                run_id: id.clone(),
                ticks,
                donut_hp_lost: hp,
            });
        Self {
            m1_control,
            m1_control_best: m1_best,
            m2_stationary,
            m2_failing_groups: failing,
            m2_groups: groups.len(),
            m2_best_counterexample: best,
            m2_stationary_reflex,
            mobile_win: any_win(records, &m2, Seat::SoloDonutBot, PolicyId::MobileCast),
            carl_melee_win: any_win(records, &m2, Seat::SoloCarl, PolicyId::CarlMelee),
            coop_win: any_win(records, &m2, Seat::TwoPlayer, PolicyId::CoopProbe),
            trivial_hold_is_one_press: trivial,
            mana_accounting: accounting,
        }
    }
}

/// `runs.csv`: one row per run, CORRAL-AUTO section 4 columns the schema can
/// attribute honestly.
pub fn runs_csv(records: &[RunRecord]) -> String {
    let mut out = String::from(
        "run_id,stage,seat,policy,policy_version,evasion,donut_x,facing,phase_a,phase_b,terminal,ticks,\
donut_hp_start,donut_hp_final,carl_hp_start,carl_hp_final,mana_start,mana_final,mana_min,presses,\
bolts_released,dry_presses,bolt_hits,donut_melee_hits,carl_hits,llamas,llamas_defeated,kill_tick_a,\
kill_tick_b,tells,spits,tells_interrupted,donut_hurt_events,carl_hurt_events,donut_move_inputs,donut_displacement_px,\
stationary_position_valid,no_damage_clear,no_movement_clear,both_llamas_defeated,companion_assisted,\
low_risk_stationary_clear,state_hash\n",
    );
    let tick = |t: Option<&Option<u64>>| match t.copied().flatten() {
        Some(t) => t.to_string(),
        None => String::new(),
    };
    for r in records {
        let s = &r.spec;
        let _ = writeln!(
            out,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:016x}",
            r.run_id,
            s.stage.name(),
            s.seat.name(),
            s.policy.name(),
            POLICY_VERSION,
            s.evasion.name(),
            s.donut_x,
            s.facing,
            s.phase[0],
            s.phase[1],
            r.terminal.name(),
            r.ticks,
            r.donut_hp_start,
            r.donut_hp_final,
            r.carl_hp_start,
            r.carl_hp_final,
            r.mana_start,
            r.mana_final,
            r.mana_min,
            r.presses,
            r.bolts_released,
            r.dry_presses,
            r.bolt_hits,
            r.donut_melee_hits,
            r.carl_hits,
            r.llamas,
            r.llamas_defeated,
            tick(r.kill_ticks.first()),
            tick(r.kill_ticks.get(1)),
            r.tells,
            r.spits,
            r.tells_interrupted,
            r.donut_hurt_events,
            r.carl_hurt_events,
            r.donut_move_inputs,
            r.donut_displacement_px,
            r.stationary_position_valid(),
            r.no_damage_clear(),
            r.no_movement_clear(),
            r.both_llamas_defeated(),
            r.companion_assisted(),
            is_low_risk_stationary_clear(r),
            r.state_hash,
        );
    }
    out
}

fn group_json(g: &GroupVerdict) -> Value {
    json!({
        "group": g.group_id,
        "runs": g.runs,
        "clears": g.clears,
        "fails": g.fails,
        "best_counterexample": g.best_counterexample,
    })
}

/// Per (stage, seat, policy, evasion) tallies: the rows of the report table.
pub fn tallies(records: &[RunRecord]) -> Vec<Tally> {
    let mut map: BTreeMap<(String, &'static str, &'static str, &'static str), Tally> =
        BTreeMap::new();
    for r in records {
        let key = (
            r.spec.stage.name().to_string(),
            r.spec.seat.name(),
            r.spec.policy.name(),
            r.spec.evasion.name(),
        );
        let t = map.entry(key.clone()).or_insert_with(|| Tally {
            stage: key.0.clone(),
            seat: key.1,
            policy: key.2,
            evasion: key.3,
            ..Tally::default()
        });
        t.runs += 1;
        match r.terminal {
            Terminal::Victory => {
                t.victories += 1;
                t.victory_ticks.push(r.ticks);
                if r.donut_hp_lost() <= LOW_RISK_HP_LOSS {
                    t.low_risk_victories += 1;
                }
                if r.no_damage_clear() {
                    t.no_damage_victories += 1;
                }
            }
            Terminal::HeroDefeat => t.defeats += 1,
            Terminal::Timeout => t.timeouts += 1,
            Terminal::Blocked => t.blocked += 1,
            Terminal::InvalidSetup => t.invalid += 1,
            Terminal::Softlock => t.softlocks += 1,
        }
        if r.companion_assisted() {
            t.companion_assisted += 1;
        }
        t.donut_hp_lost += u32::from(r.donut_hp_lost());
        t.carl_hp_lost += u32::from(r.carl_hp_lost());
    }
    map.into_values().collect()
}

#[derive(Debug, Clone, Default)]
pub struct Tally {
    pub stage: String,
    pub seat: &'static str,
    pub policy: &'static str,
    pub evasion: &'static str,
    pub runs: usize,
    pub victories: usize,
    pub low_risk_victories: usize,
    pub no_damage_victories: usize,
    pub defeats: usize,
    pub timeouts: usize,
    pub softlocks: usize,
    pub blocked: usize,
    pub invalid: usize,
    pub companion_assisted: usize,
    pub donut_hp_lost: u32,
    pub carl_hp_lost: u32,
    pub victory_ticks: Vec<u64>,
}

impl Tally {
    fn ticks_range(&self) -> String {
        match (
            self.victory_ticks.iter().min(),
            self.victory_ticks.iter().max(),
        ) {
            (Some(a), Some(b)) => format!("{a}-{b}"),
            _ => "-".to_string(),
        }
    }
}

/// `summary.json`.
pub fn summary_json(records: &[RunRecord], config: &SweepConfig, targeting: Option<bool>) -> Value {
    let gate = Gate::evaluate(records);
    let rows: Vec<Value> = tallies(records)
        .iter()
        .map(|t| {
            json!({
                "stage": t.stage, "seat": t.seat, "policy": t.policy, "evasion": t.evasion,
                "runs": t.runs, "victories": t.victories,
                "low_risk_victories": t.low_risk_victories,
                "no_damage_victories": t.no_damage_victories,
                "defeats": t.defeats, "timeouts": t.timeouts, "softlocks": t.softlocks,
                "blocked": t.blocked, "invalid_setup": t.invalid,
                "companion_assisted": t.companion_assisted,
                "donut_hp_lost_total": t.donut_hp_lost,
                "carl_hp_lost_total": t.carl_hp_lost,
            })
        })
        .collect();
    json!({
        "tool": "corral_eval",
        "policy_version": POLICY_VERSION,
        "horizon_ticks": config.horizon,
        "runs": records.len(),
        "counting_unit": "fixtures (seeds do not vary gameplay)",
        "stages": config.stages.iter().map(|s| s.name()).collect::<Vec<_>>(),
        "reflex_variant": Evasion::Reflex.pending(),
        "carl_only_llama_targeting": targeting,
        "gates": {
            "m1_positive_control": gate.m1_control.name(),
            "m1_positive_control_best": gate.m1_control_best,
            "m2_stationary_kc19_report_only": gate.m2_stationary.name(),
            "m2_stationary_groups": gate.m2_groups,
            "m2_stationary_best_counterexample": gate.m2_best_counterexample.as_ref().map(|c| json!({
                "run_id": c.run_id, "ticks": c.ticks, "donut_hp_lost": c.donut_hp_lost,
                "rank": "fewest ticks, then least Donut HP lost, over all failing groups",
            })),
            "m2_stationary_failing_groups": gate.m2_failing_groups.iter().map(group_json).collect::<Vec<_>>(),
            "m2_stationary_reflex_report_only": gate.m2_stationary_reflex.name(),
            "mobile_donut_win_report_only": gate.mobile_win.name(),
            "carl_melee_win_report_only": gate.carl_melee_win.name(),
            "coop_win_report_only": gate.coop_win.name(),
            "trivial_hold_is_one_press": gate.trivial_hold_is_one_press.name(),
            "mana_accounting": gate.mana_accounting.name(),
        },
        "tallies": rows,
    })
}

/// `report.md`, generated from the records.
pub fn report_md(records: &[RunRecord], config: &SweepConfig, targeting: Option<bool>) -> String {
    let gate = Gate::evaluate(records);
    let mut out = String::new();
    let _ = writeln!(out, "# CORRAL-EVAL baseline report (reflex absent)\n");
    let _ = writeln!(
        out,
        "Policy version {POLICY_VERSION}, horizon {} ticks, {} runs. The unit is a fixture (start x facing x attack phase), never a seed: seeds do not vary gameplay. Stages are baselines on the corridor kit with today's llamas (no rush, no party targeting, no cover), not the Infernal Corral. The reflex variant is `{}`.\n",
        config.horizon,
        records.len(),
        Evasion::Reflex.pending().unwrap_or("available"),
    );
    let _ = writeln!(out, "## Gates\n");
    let _ = writeln!(out, "| Gate | Outcome | Status |\n| --- | --- | --- |");
    let rows = [
        (
            "M1 positive control (stationary Donut wins M1)",
            gate.m1_control,
            "asserted",
        ),
        (
            "Mana accounting closes on every run",
            gate.mana_accounting,
            "asserted",
        ),
        (
            "`hold_attack_trivial` is one press",
            gate.trivial_hold_is_one_press,
            "asserted",
        ),
        (
            "M2 stationary gate, KC19, reflex off",
            gate.m2_stationary,
            "REPORT-ONLY until CORRAL-C",
        ),
        (
            "M2 stationary gate, KC19, reflex on",
            gate.m2_stationary_reflex,
            "REPORT-ONLY, blocked until DM-B",
        ),
        (
            "Mobile Donut victory on M2",
            gate.mobile_win,
            "REPORT-ONLY until CORRAL-C",
        ),
        (
            "Carl melee victory on M2",
            gate.carl_melee_win,
            "REPORT-ONLY until CORRAL-C",
        ),
        (
            "2P co-op victory on M2",
            gate.coop_win,
            "REPORT-ONLY until CORRAL-C",
        ),
    ];
    for (name, outcome, status) in rows {
        let _ = writeln!(out, "| {name} | {} | {status} |", outcome.name());
    }
    let _ = writeln!(
        out,
        "\nCarl-only llama targeting (a hard system failure once CORRAL-B lands; measured, not asserted): {}.\n",
        match targeting {
            Some(true) => "present (every llama aims at Carl)",
            Some(false) => "absent",
            None => "not measured",
        }
    );
    let _ = writeln!(
        out,
        "M2 stationary groups judged: {}; failing: {}.\n",
        gate.m2_groups,
        gate.m2_failing_groups.len()
    );
    if let Some(c) = &gate.m2_best_counterexample {
        let _ = writeln!(
            out,
            "Best counterexample (fewest ticks, then least Donut HP lost, over all failing groups): `{}` ({} ticks, Donut lost {} HP).\n",
            c.run_id, c.ticks, c.donut_hp_lost
        );
    }
    let _ = writeln!(out, "## Tallies\n");
    let _ = writeln!(
        out,
        "| Stage | Seat | Policy | Evasion | Runs | Victory | Low-risk | No-damage | Defeat | Timeout | Softlock | Blocked | Invalid | Companion-assisted | Win ticks |\n| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |"
    );
    let all = tallies(records);
    let blocked_runs: usize = all
        .iter()
        .filter(|t| t.blocked == t.runs)
        .map(|t| t.runs)
        .sum();
    for t in all.iter().filter(|t| t.blocked != t.runs) {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            t.stage,
            t.seat,
            t.policy,
            t.evasion,
            t.runs,
            t.victories,
            t.low_risk_victories,
            t.no_damage_victories,
            t.defeats,
            t.timeouts,
            t.softlocks,
            t.blocked,
            t.invalid,
            t.companion_assisted,
            t.ticks_range(),
        );
    }
    if blocked_runs > 0 {
        let _ = writeln!(
            out,
            "\n{blocked_runs} runs of the `reflex` variant are `blocked` (not on main) and are left out of the table; they stay in `runs.csv`."
        );
    }
    out
}
