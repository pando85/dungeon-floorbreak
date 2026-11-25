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
        && r.spec.seat == Seat::decisive(r.spec.stage)
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
    // Keyed by the group id plus the column as a number.
    type Key = (String, i32, i32);
    let mut groups: BTreeMap<Key, Vec<&RunRecord>> = BTreeMap::new();
    for r in records {
        if stages.contains(&r.spec.stage)
            && r.spec.evasion == evasion
            && r.spec.seat == Seat::decisive(r.spec.stage)
            && r.spec.policy.is_stationary_gate()
            && !matches!(r.terminal, Terminal::Blocked | Terminal::InvalidSetup)
        {
            let key = (r.spec.group_id(), r.spec.donut_x, r.spec.donut_rise);
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

/// The gates of one sweep: the CORRAL-C corral gate (asserted by the
/// `corral_auto_*` tests) and the CORRAL-EVAL core baselines (the "before",
/// report-only).
#[derive(Debug, Clone)]
pub struct Gate {
    /// Baseline M1 positive control on the corridor kit.
    pub m1_control: Outcome,
    pub m1_control_best: Option<String>,
    /// Baseline KC19 on the corridor M2 stages, reflex off (report-only).
    pub m2_stationary: Outcome,
    pub m2_failing_groups: Vec<GroupVerdict>,
    pub m2_groups: usize,
    pub m2_best_counterexample: Option<Counterexample>,
    /// Baseline KC19 with the reflex on (report-only).
    pub m2_stationary_reflex: Outcome,
    /// Baseline movement wins (report-only).
    pub mobile_win: Outcome,
    pub carl_melee_win: Outcome,
    pub coop_win: Outcome,
    /// Corral gate item 3: `stationary_cast_aimed` wins M1-A and M1-B in their
    /// natural facing with 0 damage, in every reflex variant run.
    pub corral_m1_control: Outcome,
    /// Corral gate item 1 (KC19), decided on the decisive variant: the parked
    /// seat with the reflex on.
    pub corral_kc19: Outcome,
    pub corral_failing_groups: Vec<GroupVerdict>,
    pub corral_groups: usize,
    pub corral_best_counterexample: Option<Counterexample>,
    /// KC19 with the reflex off (reported).
    pub corral_kc19_reflex_off: Outcome,
    /// Corral gate item 2 (CRL-Q35): no gated static run ends with both M2
    /// llamas defeated by Donut-credited hits.
    pub corral_invariant: Outcome,
    pub corral_invariant_breaches: Vec<String>,
    /// Corral gate item 4: one mobile Donut, one Carl melee, one co-op win.
    pub corral_mobile_win: Outcome,
    pub corral_carl_melee_win: Outcome,
    pub corral_coop_win: Outcome,
    /// Corral gate item 5: the hard system failures over every corral run.
    pub corral_system_failures: Outcome,
    pub corral_failure_runs: Vec<String>,
    /// Diagnostic: gated static M2 runs that defeated their own level's llama.
    pub layer_static_clears: usize,
    pub gated_static_m2_runs: usize,
    /// `hold_attack_trivial` never produced more than the one press.
    pub trivial_hold_is_one_press: Outcome,
    /// Every run's mana arithmetic closes (asserted).
    pub mana_accounting: Outcome,
}

fn any_win(
    records: &[RunRecord],
    stages: &[Stage],
    seat: Seat,
    policy: PolicyId,
    evasion: Option<Evasion>,
) -> Outcome {
    let mut seen = false;
    for r in records.iter().filter(|r| {
        stages.contains(&r.spec.stage)
            && r.spec.seat == seat
            && r.spec.policy == policy
            && evasion.is_none_or(|e| r.spec.evasion == e)
    }) {
        seen = true;
        if r.terminal == Terminal::Victory {
            return Outcome::Pass;
        }
    }
    if seen { Outcome::Fail } else { Outcome::NotRun }
}

/// KC19 over `stages` for one reflex variant: the outcome, the failing groups,
/// the group count and the global best counterexample.
fn kc19(
    records: &[RunRecord],
    stages: &[Stage],
    evasion: Evasion,
) -> (Outcome, Vec<GroupVerdict>, usize, Option<Counterexample>) {
    let groups = stationary_groups(records, stages, evasion);
    let failing: Vec<GroupVerdict> = groups.iter().filter(|g| g.fails).cloned().collect();
    let outcome = if groups.is_empty() {
        Outcome::NotRun
    } else if failing.is_empty() {
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
    (outcome, failing, groups.len(), best)
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
        let (m2_stationary, failing, m2_groups, best) = kc19(records, &m2, Evasion::Off);
        let (m2_stationary_reflex, _, _, _) = kc19(records, &m2, Evasion::Reflex);

        // ---- the corral (CORRAL-C) ----
        let corral = [Stage::CorralM2];
        let controls: Vec<&RunRecord> = records
            .iter()
            .filter(|r| {
                r.spec.stage == Stage::CorralM1
                    && r.spec.policy == PolicyId::StationaryCastAimed
                    && r.spec.seat == Seat::SoloDonutParked
                    && r.spec.facing == 1
            })
            .collect();
        let corral_m1_control = if controls.is_empty() {
            Outcome::NotRun
        } else if controls
            .iter()
            .all(|r| r.terminal == Terminal::Victory && r.donut_hp_lost() == 0)
        {
            Outcome::Pass
        } else {
            Outcome::Fail
        };
        let (corral_kc19, corral_failing_groups, corral_groups, corral_best) =
            kc19(records, &corral, Evasion::Reflex);
        let (corral_kc19_reflex_off, _, _, _) = kc19(records, &corral, Evasion::Off);
        let gated: Vec<&RunRecord> = records
            .iter()
            .filter(|r| {
                r.spec.stage == Stage::CorralM2
                    && r.spec.policy.is_stationary_gate()
                    && matches!(r.spec.seat, Seat::SoloDonutParked | Seat::SoloDonutAssist)
                    && r.terminal != Terminal::InvalidSetup
            })
            .collect();
        let breaches: Vec<String> = gated
            .iter()
            .filter(|r| {
                if r.spec.seat == Seat::SoloDonutParked {
                    r.both_llamas_defeated()
                } else {
                    r.both_llamas_defeated() && r.all_defeated_by_donut()
                }
            })
            .map(|r| r.run_id.clone())
            .collect();
        let corral_invariant = if gated.is_empty() {
            Outcome::NotRun
        } else if breaches.is_empty() {
            Outcome::Pass
        } else {
            Outcome::Fail
        };
        let corral_runs: Vec<&RunRecord> = records
            .iter()
            .filter(|r| r.spec.stage.is_corral() && r.terminal != Terminal::InvalidSetup)
            .collect();
        let failure_runs: Vec<String> = corral_runs
            .iter()
            .filter(|r| r.failures.total() > 0)
            .map(|r| format!("{} ({})", r.run_id, r.note))
            .collect();
        let corral_system_failures = if corral_runs.is_empty() {
            Outcome::NotRun
        } else if failure_runs.is_empty() {
            Outcome::Pass
        } else {
            Outcome::Fail
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
        Self {
            m1_control,
            m1_control_best: m1_best,
            m2_stationary,
            m2_failing_groups: failing,
            m2_groups,
            m2_best_counterexample: best,
            m2_stationary_reflex,
            mobile_win: any_win(
                records,
                &m2,
                Seat::SoloDonutBot,
                PolicyId::MobileCast,
                Some(Evasion::Off),
            ),
            carl_melee_win: any_win(
                records,
                &m2,
                Seat::SoloCarl,
                PolicyId::CarlMelee,
                Some(Evasion::Off),
            ),
            coop_win: any_win(
                records,
                &m2,
                Seat::TwoPlayer,
                PolicyId::CoopProbe,
                Some(Evasion::Off),
            ),
            corral_m1_control,
            corral_kc19,
            corral_failing_groups,
            corral_groups,
            corral_best_counterexample: corral_best,
            corral_kc19_reflex_off,
            corral_invariant,
            corral_invariant_breaches: breaches,
            corral_mobile_win: any_win(
                records,
                &corral,
                Seat::SoloDonutParked,
                PolicyId::MobileCast,
                None,
            ),
            corral_carl_melee_win: any_win(
                records,
                &corral,
                Seat::SoloCarl,
                PolicyId::CarlMelee,
                None,
            ),
            corral_coop_win: any_win(records, &corral, Seat::TwoPlayer, PolicyId::CoopProbe, None),
            corral_system_failures,
            corral_failure_runs: failure_runs,
            layer_static_clears: gated
                .iter()
                .filter(|r| r.spec.seat == Seat::SoloDonutParked && r.layer_static_clear())
                .count(),
            gated_static_m2_runs: gated
                .iter()
                .filter(|r| r.spec.seat == Seat::SoloDonutParked)
                .count(),
            trivial_hold_is_one_press: trivial,
            mana_accounting: accounting,
        }
    }

    /// Every asserted corral gate passes.
    pub fn corral_passes(&self) -> bool {
        [
            self.corral_m1_control,
            self.corral_kc19,
            self.corral_invariant,
            self.corral_mobile_win,
            self.corral_carl_melee_win,
            self.corral_coop_win,
            self.corral_system_failures,
        ]
        .iter()
        .all(|o| *o == Outcome::Pass)
    }
}

/// `runs.csv`: one row per run, CORRAL-AUTO section 4 columns the schema can
/// attribute honestly.
pub fn runs_csv(records: &[RunRecord]) -> String {
    let columns = [
        "run_id",
        "stage",
        "seat",
        "policy",
        "jump_offset",
        "policy_version",
        "evasion",
        "donut_x",
        "donut_rise",
        "facing",
        "cooldown_a",
        "rush_cooldown_a",
        "cooldown_b",
        "rush_cooldown_b",
        "terminal",
        "ticks",
        "donut_hp_start",
        "donut_hp_final",
        "carl_hp_start",
        "carl_hp_final",
        "mana_start",
        "mana_final",
        "mana_min",
        "presses",
        "jump_presses",
        "bolts_released",
        "dry_presses",
        "bolt_hits",
        "donut_melee_hits",
        "carl_hits",
        "llamas",
        "llamas_defeated",
        "kill_tick_a",
        "kill_tick_b",
        "kill_by_donut_a",
        "kill_by_donut_b",
        "tells",
        "spits",
        "tells_interrupted",
        "rush_tells",
        "rush_commits",
        "rush_contacts",
        "rush_misses",
        "rush_bonks",
        "donut_hurt_events",
        "carl_hurt_events",
        "donut_rush_hurts",
        "carl_rush_hurts",
        "evades",
        "donut_move_inputs",
        "donut_displacement_px",
        "donut_max_offset_px",
        "stationary_position_valid",
        "no_damage_clear",
        "no_movement_clear",
        "both_llamas_defeated",
        "companion_assisted",
        "low_risk_stationary_clear",
        "layer_static_clear",
        "system_failures",
        "state_hash",
    ];
    let mut out = columns.join(",");
    out.push('\n');
    let tick = |t: Option<&Option<u64>>| match t.copied().flatten() {
        Some(t) => t.to_string(),
        None => String::new(),
    };
    let flag = |v: Option<&bool>| v.map_or(String::new(), |b| b.to_string());
    for r in records {
        let s = &r.spec;
        let row: Vec<String> = vec![
            r.run_id.clone(),
            s.stage.name().into(),
            s.seat.name().into(),
            s.policy.name().into(),
            s.jump_offset.to_string(),
            POLICY_VERSION.to_string(),
            s.evasion.name().into(),
            s.donut_x.to_string(),
            s.donut_rise.to_string(),
            s.facing.to_string(),
            s.phase[0].0.to_string(),
            s.phase[0].1.to_string(),
            s.phase[1].0.to_string(),
            s.phase[1].1.to_string(),
            r.terminal.name().into(),
            r.ticks.to_string(),
            r.donut_hp_start.to_string(),
            r.donut_hp_final.to_string(),
            r.carl_hp_start.to_string(),
            r.carl_hp_final.to_string(),
            r.mana_start.to_string(),
            r.mana_final.to_string(),
            r.mana_min.to_string(),
            r.presses.to_string(),
            r.jump_presses.to_string(),
            r.bolts_released.to_string(),
            r.dry_presses.to_string(),
            r.bolt_hits.to_string(),
            r.donut_melee_hits.to_string(),
            r.carl_hits.to_string(),
            r.llamas.to_string(),
            r.llamas_defeated.to_string(),
            tick(r.kill_ticks.first()),
            tick(r.kill_ticks.get(1)),
            flag(r.kill_by_donut.first()),
            flag(r.kill_by_donut.get(1)),
            r.tells.to_string(),
            r.spits.to_string(),
            r.tells_interrupted.to_string(),
            r.rush_tells.to_string(),
            r.rush_commits.to_string(),
            r.rush_contacts.to_string(),
            r.rush_misses.to_string(),
            r.rush_bonks.to_string(),
            r.donut_hurt_events.to_string(),
            r.carl_hurt_events.to_string(),
            r.donut_rush_hurts.to_string(),
            r.carl_rush_hurts.to_string(),
            r.evades.to_string(),
            r.donut_move_inputs.to_string(),
            r.donut_displacement_px.to_string(),
            r.donut_max_offset_px.to_string(),
            r.stationary_position_valid().to_string(),
            r.no_damage_clear().to_string(),
            r.no_movement_clear().to_string(),
            r.both_llamas_defeated().to_string(),
            r.companion_assisted().to_string(),
            is_low_risk_stationary_clear(r).to_string(),
            r.layer_static_clear().to_string(),
            r.failures.total().to_string(),
            format!("{:016x}", r.state_hash),
        ];
        debug_assert_eq!(row.len(), columns.len());
        out.push_str(&row.join(","));
        out.push('\n');
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
    let counter = |c: &Option<Counterexample>| {
        c.as_ref().map(|c| {
            json!({
                "run_id": c.run_id, "ticks": c.ticks, "donut_hp_lost": c.donut_hp_lost,
                "rank": "fewest ticks, then least Donut HP lost, over all failing groups",
            })
        })
    };
    json!({
        "tool": "corral_eval",
        "policy_version": POLICY_VERSION,
        "horizon_ticks": config.horizon,
        "runs": records.len(),
        "counting_unit": "fixtures (seeds do not vary gameplay)",
        "stages": config.stages().iter().map(|s| s.name()).collect::<Vec<_>>(),
        "partition_variants": {
            "absent": "run",
            "intact": "not_run (CORRAL-D has not built the partition)",
            "broken": "not_run (CORRAL-D has not built the partition)",
        },
        "carl_only_llama_targeting_baseline": targeting,
        "corral_gate": {
            "passes": gate.corral_passes(),
            "m1_positive_control": gate.corral_m1_control.name(),
            "kc19_decisive_parked_reflex": gate.corral_kc19.name(),
            "kc19_groups": gate.corral_groups,
            "kc19_failing_groups": gate.corral_failing_groups.iter().map(group_json).collect::<Vec<_>>(),
            "kc19_best_counterexample": counter(&gate.corral_best_counterexample),
            "kc19_reflex_off_reported": gate.corral_kc19_reflex_off.name(),
            "crlq35_invariant": gate.corral_invariant.name(),
            "crlq35_breaches": gate.corral_invariant_breaches,
            "mobile_donut_win": gate.corral_mobile_win.name(),
            "carl_melee_win": gate.corral_carl_melee_win.name(),
            "coop_win": gate.corral_coop_win.name(),
            "system_failures": gate.corral_system_failures.name(),
            "system_failure_runs": gate.corral_failure_runs,
            "layer_static_clear_diagnostic": {
                "runs": gate.layer_static_clears,
                "of_gated_parked_runs": gate.gated_static_m2_runs,
            },
            "stuck_enemy_check": "not measured (no progress definition for a llama that legitimately holds)",
        },
        "baseline_report_only": {
            "m1_positive_control": gate.m1_control.name(),
            "m1_positive_control_best": gate.m1_control_best,
            "m2_stationary_kc19_reflex_off": gate.m2_stationary.name(),
            "m2_stationary_groups": gate.m2_groups,
            "m2_stationary_best_counterexample": counter(&gate.m2_best_counterexample),
            "m2_stationary_kc19_reflex_on": gate.m2_stationary_reflex.name(),
            "mobile_donut_win": gate.mobile_win.name(),
            "carl_melee_win": gate.carl_melee_win.name(),
            "coop_win": gate.coop_win.name(),
        },
        "trivial_hold_is_one_press": gate.trivial_hold_is_one_press.name(),
        "mana_accounting": gate.mana_accounting.name(),
        "tallies": rows,
    })
}

/// `report.md`, generated from the records.
pub fn report_md(records: &[RunRecord], config: &SweepConfig, targeting: Option<bool>) -> String {
    let gate = Gate::evaluate(records);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# CORRAL-AUTO report: the Infernal Corral (after) and the corridor baselines (before)\n"
    );
    let _ = writeln!(
        out,
        "Policy version {POLICY_VERSION}, horizon {} ticks, {} runs. The unit is a fixture (start x surface x facing x attack phase x reflex x seat x policy), never a seed: seeds do not vary gameplay. The corral stages run on `f01_corral.room.json` with the CORRAL-B rush and llama party targeting; the baselines run on the corridor kit with today's legacy llamas (no rush, no cover). Partition variants `intact` and `broken` are `not_run` until CORRAL-D.\n",
        config.horizon,
        records.len(),
    );
    let _ = writeln!(out, "## Corral gate (asserted)\n");
    let _ = writeln!(out, "| Gate | Outcome |\n| --- | --- |");
    for (name, outcome) in [
        (
            "M1 positive control: `stationary_cast_aimed` wins M1-A and M1-B with 0 damage",
            gate.corral_m1_control,
        ),
        (
            "KC19 on M2, decisive variant (Carl parked, reflex on)",
            gate.corral_kc19,
        ),
        (
            "CRL-Q35 invariant: no gated static run defeats both M2 llamas by Donut hits",
            gate.corral_invariant,
        ),
        ("A mobile Donut victory on M2", gate.corral_mobile_win),
        ("A Carl melee victory on M2", gate.corral_carl_melee_win),
        ("A 2P co-op victory on M2", gate.corral_coop_win),
        (
            "No hard system failure on any corral run",
            gate.corral_system_failures,
        ),
        (
            "KC19 on M2 with the reflex off (reported)",
            gate.corral_kc19_reflex_off,
        ),
    ] {
        let _ = writeln!(out, "| {name} | {} |", outcome.name());
    }
    let _ = writeln!(
        out,
        "\nKC19 groups judged: {}; failing: {}. `layer_static_clear` (diagnostic, not gated): {} of {} gated parked M2 runs defeated the llama of their own level.\n",
        gate.corral_groups,
        gate.corral_failing_groups.len(),
        gate.layer_static_clears,
        gate.gated_static_m2_runs,
    );
    if let Some(c) = &gate.corral_best_counterexample {
        let _ = writeln!(
            out,
            "Best corral counterexample: `{}` ({} ticks, Donut lost {} HP).\n",
            c.run_id, c.ticks, c.donut_hp_lost
        );
    }
    for id in gate.corral_invariant_breaches.iter().take(10) {
        let _ = writeln!(out, "- CRL-Q35 breach: `{id}`");
    }
    for id in gate.corral_failure_runs.iter().take(10) {
        let _ = writeln!(out, "- system failure: `{id}`");
    }
    let _ = writeln!(out, "## Baselines (report-only, the \"before\")\n");
    let _ = writeln!(out, "| Gate | Outcome |\n| --- | --- |");
    for (name, outcome) in [
        ("M1 positive control", gate.m1_control),
        ("M2 stationary KC19, reflex off", gate.m2_stationary),
        ("M2 stationary KC19, reflex on", gate.m2_stationary_reflex),
        ("Mobile Donut victory on M2", gate.mobile_win),
        ("Carl melee victory on M2", gate.carl_melee_win),
        ("2P co-op victory on M2", gate.coop_win),
        ("Mana accounting closes on every run", gate.mana_accounting),
        (
            "`hold_attack_trivial` is one press",
            gate.trivial_hold_is_one_press,
        ),
    ] {
        let _ = writeln!(out, "| {name} | {} |", outcome.name());
    }
    let _ = writeln!(
        out,
        "\nBaseline Carl-only llama targeting (legacy llamas; measured, not asserted): {}.\n",
        match targeting {
            Some(true) => "present (every legacy llama aims at Carl)",
            Some(false) => "absent",
            None => "not measured",
        }
    );
    if let Some(c) = &gate.m2_best_counterexample {
        let _ = writeln!(
            out,
            "Best baseline counterexample: `{}` ({} ticks, Donut lost {} HP).\n",
            c.run_id, c.ticks, c.donut_hp_lost
        );
    }
    let _ = writeln!(out, "## Tallies\n");
    let _ = writeln!(
        out,
        "| Stage | Seat | Policy | Evasion | Runs | Victory | Low-risk | No-damage | Defeat | Timeout | Invalid | Companion-assisted | Win ticks |\n| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |"
    );
    for t in tallies(records) {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
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
            t.invalid,
            t.companion_assisted,
            t.ticks_range(),
        );
    }
    out
}
