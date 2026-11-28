//! BOS-A: `bos_eval`, the STATIONARY gate of the Ball of Swine
//! (`docs/01-design/floor-01-bos-integration.md` 9.3, BOS-AD22, BOS-AD23).
//!
//! It answers one question with numbers: can a hero who holds one position (a
//! hop or a turn in place still counts as standing) beat the ball at no cost? Every
//! row drives the real party simulation ([`crate::ball_fixtures::Rig`]: the live
//! party step order, the real mana, collisions, the boss machine and the follower
//! bot) with legal per-slot raw inputs, one tick at a time.
//!
//! Rows G-0 (the control: `carl_bait_punish` must clear) and G-1..G-7 (the
//! stationary policies). Each row first closes the gate, then holds its policy for
//! [`HOLD_TICKS`] ticks.
//!
//! **Setup-only writes:** a hero's tick-0 column (the heroes cannot pass the ball
//! to reach the right alcove, so a right-alcove hero starts there) and, in the
//! solo rows, the swap press. After that the only mutation is the step.
//!
//! **Pass** (G-1..G-7): the ball is not defeated with a party health loss of 1 or
//! less. **Validity** (BOS-AD23): a row is invalid, and the gate fails, when the
//! gate never closed, when the policy neither dealt 6 boss damage nor took a ram,
//! shove or siege hit, or when a scripted position is outside its range.

use std::path::Path;

use crate::ball_fixtures::{
    BALL01_DUO, BALL01_DUO_COOP, BALL01_ENV_CLEAR_SOLO, BALL01_FULL_CLEAR_SOLO, BALL01_SOLO, Rig,
    Tally,
};
use crate::input::RawInput;
use crate::party::HeroId;
use crate::party_scene::PartyFixture;

/// The ticks a row holds its policy after the gate closed.
pub const HOLD_TICKS: u64 = 7200;
/// Bump when a policy, a row or a metric definition changes.
pub const POLICY_VERSION: u32 = 1;
/// A hero in the left alcove is in these pivots; the right alcove likewise.
pub const LEFT_ALCOVE: (f32, f32) = (40.0, 110.0);
pub const RIGHT_ALCOVE: (f32, f32) = (530.0, 596.0);
/// The lane column of G-4 and its tolerance.
const LANE_X: f32 = 200.0;
const START_X: f32 = 150.0;

/// What a hero does once it holds its column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Act {
    Idle,
    /// Magic Missile at every legal press edge.
    Cast,
    /// A jab every 16 ticks.
    Jab,
}

/// One hero's plan.
#[derive(Debug, Clone, Copy)]
struct Plan {
    /// The tick-0 column (setup).
    start: f32,
    /// The column it walks to once the gate closed (`None`: it holds its start).
    target: Option<f32>,
    /// The way it faces while it acts (+1 right, -1 left).
    face: f32,
    act: Act,
}

/// A row of the gate.
#[derive(Debug, Clone)]
pub struct Row {
    pub id: &'static str,
    pub setup: String,
    solo: bool,
    carl: Plan,
    donut: Plan,
    /// The column range the held positions must be in (validity), per hero.
    ranges: [Option<(f32, f32)>; 2],
}

/// The result of one row.
#[derive(Debug, Clone)]
pub struct RowResult {
    pub id: &'static str,
    pub setup: String,
    pub ticks: u64,
    pub gate_closed: bool,
    pub engaged: bool,
    pub positions_valid: bool,
    pub tally: Tally,
    pub defeated: bool,
    pub final_x: [f32; 2],
}

impl RowResult {
    pub fn valid(&self) -> bool {
        self.gate_closed && self.engaged && self.positions_valid
    }

    /// G-0 must clear; every stationary row passes when it does **not** defeat the
    /// ball at a party health loss of 1 or less.
    pub fn passes(&self) -> bool {
        if self.id.starts_with("G-0") {
            self.defeated
        } else {
            !(self.defeated && self.tally.party_loss <= 1)
        }
    }

    fn ok(&self) -> bool {
        self.valid() && self.passes()
    }
}

/// The whole evaluation.
#[derive(Debug, Clone)]
pub struct Evaluation {
    pub rows: Vec<RowResult>,
    /// BOS-Q11 switched on for this evaluation (the solo bot-decoy rule).
    pub q11: bool,
}

impl Evaluation {
    pub fn gate_passes(&self) -> bool {
        self.rows.iter().all(RowResult::ok)
    }
}

const fn plan(start: f32, target: Option<f32>, face: f32, act: Act) -> Plan {
    Plan {
        start,
        target,
        face,
        act,
    }
}

const PARKED_LEFT: Plan = plan(START_X, Some(60.0), 1.0, Act::Idle);

/// The rows of the gate.
pub fn rows() -> Vec<Row> {
    let mut out = vec![
        Row {
            id: "G-1",
            setup: "Donut stationary cast in the left alcove (80); Carl idle in it (60)".into(),
            solo: false,
            carl: PARKED_LEFT,
            donut: plan(START_X, Some(80.0), 1.0, Act::Cast),
            ranges: [Some(LEFT_ALCOVE), Some(LEFT_ALCOVE)],
        },
        Row {
            id: "G-2",
            setup: "the same in the right alcove (Donut 560, Carl 580)".into(),
            solo: false,
            carl: plan(580.0, None, -1.0, Act::Idle),
            donut: plan(560.0, None, -1.0, Act::Cast),
            ranges: [Some(RIGHT_ALCOVE), Some(RIGHT_ALCOVE)],
        },
        Row {
            id: "G-3",
            setup: "split: Donut casts from the left alcove (80), Carl idles in the right (580)"
                .into(),
            solo: false,
            carl: plan(580.0, None, -1.0, Act::Idle),
            donut: plan(START_X, Some(80.0), 1.0, Act::Cast),
            ranges: [Some(RIGHT_ALCOVE), Some(LEFT_ALCOVE)],
        },
        Row {
            id: "G-4",
            setup: "Donut stationary cast in the lane (200); Carl idle in the left alcove (60)"
                .into(),
            solo: false,
            carl: PARKED_LEFT,
            donut: plan(START_X, Some(LANE_X), 1.0, Act::Cast),
            ranges: [Some(LEFT_ALCOVE), Some((LANE_X - 4.0, LANE_X + 4.0))],
        },
        Row {
            id: "G-5",
            setup: "Carl stationary jab at the left alcove edge (pivot 110, held at 107..110 after the turn step); Donut idle in the alcove (60)"
                .into(),
            solo: false,
            carl: plan(START_X, Some(106.9), 1.0, Act::Jab),
            donut: PARKED_LEFT,
            ranges: [Some(LEFT_ALCOVE), Some(LEFT_ALCOVE)],
        },
    ];
    for (i, p) in [50.0, 80.0, 100.0].into_iter().enumerate() {
        out.push(Row {
            id: ["G-6a", "G-6b", "G-6c"][i],
            setup: format!(
                "solo: the player is Donut, casting from the left alcove at pivot {p}; Carl is the live follower bot (M1)"
            ),
            solo: true,
            carl: plan(130.0, None, 1.0, Act::Idle),
            donut: plan(START_X, Some(p), 1.0, Act::Cast),
            ranges: [None, Some(LEFT_ALCOVE)],
        });
    }
    for (i, p) in [590.0, 560.0, 540.0].into_iter().enumerate() {
        out.push(Row {
            id: ["G-7a", "G-7b", "G-7c"][i],
            setup: format!(
                "solo: the player is Donut, casting from the right alcove at pivot {p}; Carl is the live follower bot (M1), stranded on the far side (the decoy case is G-6)"
            ),
            solo: true,
            carl: plan(130.0, None, 1.0, Act::Idle),
            donut: plan(p, None, -1.0, Act::Cast),
            ranges: [None, Some(RIGHT_ALCOVE)],
        });
    }
    out
}

/// Per-hero state of the policy.
#[derive(Debug, Clone, Copy, Default)]
struct Hold {
    turned: bool,
}

impl Hold {
    fn crossed(&self, start: f32, x: f32, target: f32) -> bool {
        if target >= start {
            x >= target
        } else {
            x <= target
        }
    }
}

fn drive_one(plan: &Plan, x: f32, facing: i8, hold: &mut Hold, tick: u64) -> RawInput {
    // It walks until it has crossed its column (so a column just inside an alcove's edge
    // is held after the turn step, never outside it).
    if let Some(target) = plan.target
        && !hold.turned
        && !hold.crossed(plan.start, x, target)
    {
        return RawInput {
            move_x: if target > x { 1.0 } else { -1.0 },
            ..RawInput::NONE
        };
    }
    // At its column: one tick of turning in place (a stationary policy), then it acts.
    if !hold.turned {
        hold.turned = true;
        if f32::from(facing) != plan.face {
            return RawInput {
                move_x: plan.face,
                ..RawInput::NONE
            };
        }
    }
    match plan.act {
        Act::Idle => RawInput::NONE,
        Act::Cast => RawInput {
            attack: tick.is_multiple_of(2),
            ..RawInput::NONE
        },
        Act::Jab => RawInput {
            attack: tick.is_multiple_of(16),
            ..RawInput::NONE
        },
    }
}

fn in_range(x: f32, range: Option<(f32, f32)>) -> bool {
    range.is_none_or(|(lo, hi)| (lo..=hi).contains(&x))
}

/// Run one stationary row. `q11` switches the solo bot-decoy rule on.
pub fn run_row(root: &Path, row: &Row, q11: bool) -> Result<RowResult, String> {
    let fixture = if row.solo { &BALL01_SOLO } else { &BALL01_DUO };
    let mut rig = Rig::new(root, fixture)?;
    if q11 {
        rig.set_ignore_bot_decoy(true);
    }
    rig.place(HeroId::Carl, row.carl.start);
    rig.place(HeroId::Donut, row.donut.start);
    let (mut carl_hold, mut donut_hold) = (Hold::default(), Hold::default());
    let mut closed_at: Option<u64> = None;
    let mut swap_pressed = false;
    let mut positions_valid = true;
    let mut checked = false;
    let mut ticks = 0u64;
    for tick in 1..=(HOLD_TICKS + 600) {
        ticks = tick;
        if rig.boss().gate_closed() && closed_at.is_none() {
            closed_at = Some(tick);
        }
        if closed_at.is_some_and(|at| tick > at + HOLD_TICKS) || rig.tally.defeated_at.is_some() {
            break;
        }
        let gated = closed_at.is_some();
        let slots = if row.solo {
            // The player starts as Carl and swaps to Donut (setup), then plays her.
            if rig.sim.party.controlled_hero() != HeroId::Donut {
                let press = !swap_pressed;
                swap_pressed = true;
                [
                    RawInput {
                        swap: press,
                        ..RawInput::NONE
                    },
                    RawInput::NONE,
                ]
            } else if gated {
                let x = rig.x_of(HeroId::Donut);
                let facing = rig.sim.scene.world.donut_body.facing;
                [
                    drive_one(&row.donut, x, facing, &mut donut_hold, tick),
                    RawInput::NONE,
                ]
            } else {
                [RawInput::NONE; 2]
            }
        } else if gated {
            let carl = drive_one(
                &row.carl,
                rig.x_of(HeroId::Carl),
                rig.sim.scene.world.hero.facing,
                &mut carl_hold,
                tick,
            );
            let donut = drive_one(
                &row.donut,
                rig.x_of(HeroId::Donut),
                rig.sim.scene.world.donut_body.facing,
                &mut donut_hold,
                tick,
            );
            [carl, donut]
        } else {
            [RawInput::NONE; 2]
        };
        rig.step(slots);
        // The scripted columns are checked once, on the tick every acting hero
        // holds its column (after its turn): a later knockback is the fight's.
        if !checked && gated && (row.solo || carl_hold.turned) && donut_hold.turned {
            checked = true;
            let held = [rig.x_of(HeroId::Carl), rig.x_of(HeroId::Donut)];
            for i in 0..2 {
                if !(row.solo && i == 0) && !in_range(held[i], row.ranges[i]) {
                    positions_valid = false;
                }
            }
        }
    }
    let t = rig.tally.clone();
    Ok(RowResult {
        id: row.id,
        setup: row.setup.clone(),
        ticks,
        gate_closed: closed_at.is_some(),
        engaged: t.boss_damage >= 6 || t.hero_hits >= 1,
        positions_valid: positions_valid && checked,
        defeated: t.defeated_at.is_some(),
        final_x: [rig.x_of(HeroId::Carl), rig.x_of(HeroId::Donut)],
        tally: t,
    })
}

/// G-0: the control. `carl_bait_punish` must clear.
pub fn run_control(root: &Path, q11: bool) -> Result<RowResult, String> {
    run_policy(
        root,
        q11,
        &BALL01_FULL_CLEAR_SOLO,
        "G-0",
        "carl_bait_punish (F11), the follower bot at his back",
    )
}

/// G-0e and G-0d (BOSB-AD13): the other two clearing policies, `carl_environment` (the chain
/// weight) and `duo_coop` (one hero on the Tusklings, one on the Ball). Each must clear.
pub fn run_policies(root: &Path, q11: bool) -> Result<Vec<RowResult>, String> {
    Ok(vec![
        run_policy(
            root,
            q11,
            &BALL01_ENV_CLEAR_SOLO,
            "G-0e",
            "carl_environment (BOSB-AD13): the weight and the jab on the Tusklings, the follower bot at his back",
        )?,
        run_policy(
            root,
            q11,
            &BALL01_DUO_COOP,
            "G-0d",
            "duo_coop (BOSB-AD13): Carl on the Ball, Donut on the Tusklings",
        )?,
    ])
}

fn run_policy(
    root: &Path,
    q11: bool,
    fixture: &'static PartyFixture,
    id: &'static str,
    setup: &str,
) -> Result<RowResult, String> {
    let mut rig = Rig::new(root, fixture)?;
    if q11 {
        rig.set_ignore_bot_decoy(true);
    }
    let mut closed = false;
    let mut ticks = 0;
    for tick in 1..=HOLD_TICKS + 600 {
        ticks = tick;
        rig.step_scripted();
        closed |= rig.boss().gate_closed() || rig.boss().released();
        if rig.tally.defeated_at.is_some() {
            break;
        }
    }
    let t = rig.tally.clone();
    Ok(RowResult {
        id,
        setup: setup.into(),
        ticks,
        gate_closed: closed,
        engaged: t.boss_damage >= 6,
        positions_valid: true,
        defeated: t.defeated_at.is_some(),
        final_x: [rig.x_of(HeroId::Carl), rig.x_of(HeroId::Donut)],
        tally: t,
    })
}

/// Run the whole gate.
pub fn evaluate(root: &Path, q11: bool) -> Result<Evaluation, String> {
    let mut results = vec![run_control(root, q11)?];
    results.extend(run_policies(root, q11)?);
    for row in rows() {
        results.push(run_row(root, &row, q11)?);
    }
    Ok(Evaluation { rows: results, q11 })
}

impl Evaluation {
    /// The report text (one line per row, then the verdict).
    pub fn text(&self) -> String {
        let mut out = format!(
            "bos_eval policy_version={POLICY_VERSION} hold_ticks={HOLD_TICKS} bos_q11={}\n",
            self.q11
        );
        out.push_str(
            "row    result  valid  defeated  loss boss_dmg hits debris crashes ticks  setup\n",
        );
        for r in &self.rows {
            out.push_str(&format!(
                "{:<6} {:<7} {:<6} {:<9} {:>4} {:>8} {:>4} {:>6} {:>7} {:>5}  {}\n",
                r.id,
                if r.passes() { "pass" } else { "FAIL" },
                if r.valid() { "valid" } else { "INVALID" },
                if r.defeated {
                    r.tally
                        .defeated_at
                        .map_or_else(|| "yes".to_owned(), |t| format!("t={t}"))
                } else {
                    "no".to_owned()
                },
                r.tally.party_loss,
                r.tally.boss_damage,
                r.tally.hero_hits,
                r.tally.debris_hits,
                r.tally.crashes.len(),
                r.ticks,
                r.setup
            ));
            if !r.valid() {
                out.push_str(&format!(
                    "       invalid: gate_closed={} engaged={} positions_valid={} final_x={:?}\n",
                    r.gate_closed, r.engaged, r.positions_valid, r.final_x
                ));
            }
        }
        out.push_str(&format!(
            "GATE {}\n",
            if self.gate_passes() { "PASS" } else { "FAIL" }
        ));
        out
    }

    /// `summary.json`.
    pub fn json(&self) -> String {
        let rows: Vec<serde_json::Value> = self
            .rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "row": r.id,
                    "setup": r.setup,
                    "pass": r.passes(),
                    "valid": r.valid(),
                    "defeated": r.defeated,
                    "defeated_at": r.tally.defeated_at,
                    "party_loss": r.tally.party_loss,
                    "boss_damage": r.tally.boss_damage,
                    "hero_hits": r.tally.hero_hits,
                    "debris_hits": r.tally.debris_hits,
                    "crashes": r.tally.crashes.len(),
                    "restarts": r.tally.restarts,
                    "ticks": r.ticks,
                })
            })
            .collect();
        serde_json::to_string_pretty(&serde_json::json!({
            "policy_version": POLICY_VERSION,
            "hold_ticks": HOLD_TICKS,
            "bos_q11": self.q11,
            "gate_passes": self.gate_passes(),
            "rows": rows,
        }))
        .expect("the summary serializes")
    }
}

/// Write `report.txt` and `summary.json` under `dir`.
pub fn write_to(evaluation: &Evaluation, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for (name, text) in [
        ("report.txt", evaluation.text()),
        ("summary.json", evaluation.json() + "\n"),
    ] {
        std::fs::write(dir.join(name), text).map_err(|e| format!("{name}: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    /// The gate (G-0..G-7): the control clears and no stationary policy defeats the
    /// ball at a party health loss of 1 or less, every row valid.
    #[test]
    fn the_stationary_gate_passes_and_every_row_is_valid() {
        let e = evaluate(root(), false).unwrap();
        println!("{}", e.text());
        assert_eq!(e.rows.len(), 3 + rows().len());
        for id in ["G-0", "G-0e", "G-0d"] {
            let control = e.rows.iter().find(|r| r.id == id).unwrap();
            assert!(control.defeated, "{id}: the clearing policy clears");
        }
        for r in &e.rows {
            assert!(r.valid(), "{} is invalid: {:?}", r.id, r.setup);
            assert!(r.passes(), "{} fails", r.id);
        }
        assert!(e.gate_passes());
        // Deterministic: the same report twice.
        assert_eq!(e.text(), evaluate(root(), false).unwrap().text());
    }

    /// BOS-AD23: a setup that never closes the gate, or holds an invalid column, is
    /// invalid and fails the gate (it proves nothing).
    #[test]
    fn a_vacuous_or_misplaced_row_is_invalid() {
        let mut never = rows().remove(0);
        never.donut.start = 56.0;
        never.carl.start = 56.0;
        never.donut.target = None;
        never.carl.target = None;
        let r = run_row(root(), &never, false).unwrap();
        assert!(
            !r.gate_closed && !r.valid(),
            "two seats outside the gate never close it"
        );
        let mut off = rows().remove(0);
        off.ranges = [Some((40.0, 50.0)), Some((40.0, 50.0))];
        let r = run_row(root(), &off, false).unwrap();
        assert!(
            !r.positions_valid && !r.valid(),
            "a held column outside its range"
        );
        let e = Evaluation {
            rows: vec![r],
            q11: false,
        };
        assert!(!e.gate_passes());
    }

    /// The control can fail the gate: a ball that is never defeated is not a gate.
    #[test]
    fn a_control_that_does_not_clear_fails_the_gate() {
        let mut g0 = run_control(root(), false).unwrap();
        assert!(g0.passes());
        g0.defeated = false;
        assert!(!g0.passes());
    }

    /// BOS-Q11 is an evaluator switch over the committed value.
    #[test]
    fn bos_q11_is_off_in_the_committed_data_and_the_switch_runs() {
        assert!(!crate::boss::ball().siege.as_ref().unwrap().ignore_bot_decoy);
        let row = rows().into_iter().find(|r| r.id == "G-6b").unwrap();
        let on = run_row(root(), &row, true).unwrap();
        assert!(on.valid());
    }
}
