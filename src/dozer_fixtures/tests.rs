//! The DOZER-CORE fixtures (`docs/01-design/enemy-gameplay-dozer.md`,
//! section 14): every fixture replays twice with a committed hash.

use super::*;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Run a fixture twice: both runs pass, agree, and match the committed hash.
fn replay(name: &str, mut f: impl FnMut(&Path) -> Result<u64, String>, committed: u64) {
    let a = f(&root()).unwrap_or_else(|e| panic!("{name}: {e}"));
    let b = f(&root()).unwrap_or_else(|e| panic!("{name} (replay): {e}"));
    assert_eq!(a, b, "{name}: the replay differs");
    assert_eq!(a, committed, "{name}: hash {a:#018x}");
}

// ------------------------------------------------------------------- K5 --

/// The committed fold of the nine seeds' final hashes per `(mode, plan)`.
const K5_HASHES: [(Mode, Plan, u64); 12] = [
    (Mode::CarlSolo, Plan::Run, 0x27f52ab3c7b1ef6b),
    (Mode::CarlSolo, Plan::Stall, 0xe1b55120ec6deca9),
    (Mode::CarlSolo, Plan::CloseCall, 0xeb15a5e413088743),
    (Mode::CarlSolo, Plan::Wait, 0x7a7b5a5a2d58a4fe),
    (Mode::DonutSolo, Plan::Run, 0x339885e5df70cd97),
    (Mode::DonutSolo, Plan::Stall, 0x0afb54ae8cf75731),
    (Mode::DonutSolo, Plan::CloseCall, 0x5ac171c1efce249a),
    (Mode::DonutSolo, Plan::Wait, 0x688d18fb347f3ab5),
    (Mode::TwoPlayer, Plan::Run, 0x82ce782ffaa6673b),
    (Mode::TwoPlayer, Plan::Stall, 0xaa68c1bb07a2bfa7),
    (Mode::TwoPlayer, Plan::CloseCall, 0x67afc815208a89db),
    (Mode::TwoPlayer, Plan::Wait, 0x6567d1f97fd238e9),
];

/// One mode's 36 cells, twice: every hard assert holds, the replays agree
/// and each plan's fold matches. Returns the reports.
fn k5_mode(mode: Mode) -> Vec<K5Report> {
    let mut reports = Vec::new();
    let mut mismatches = Vec::new();
    for plan in Plan::ALL {
        let mut folds = [Vec::new(), Vec::new()];
        for seed in SEEDS {
            for (pass, fold_of) in folds.iter_mut().enumerate() {
                let r = run_k5(&root(), mode, plan, seed).unwrap_or_else(|e| panic!("{e}"));
                r.check()
                    .unwrap_or_else(|e| panic!("{}: {e}\n{}", r.name(), r.line()));
                fold_of.push(r.hash);
                if pass == 0 {
                    println!("{}", r.line());
                    reports.push(r);
                }
            }
        }
        assert_eq!(
            folds[0],
            folds[1],
            "{} {}: the replay differs",
            mode.name(),
            plan.name()
        );
        let got = fold(folds[0].iter().copied());
        let committed = K5_HASHES
            .iter()
            .find(|(m, p, _)| *m == mode && *p == plan)
            .map(|(_, _, h)| *h)
            .unwrap();
        if got != committed {
            mismatches.push(format!(
                "dz_k5_{}_{}: hash {got:#018x}",
                mode.name(),
                plan.name()
            ));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    reports
}

#[test]
fn dz_k5_carl_solo() {
    k5_mode(Mode::CarlSolo);
}

#[test]
fn dz_k5_donut_solo() {
    k5_mode(Mode::DonutSolo);
}

#[test]
fn dz_k5_two_player() {
    k5_mode(Mode::TwoPlayer);
}

/// Across the matrix every band is entered, the pinned hold never fires in
/// room 3, and the CA2-DZ trigger (DZ-Q24) is evaluated and reported.
#[test]
fn dz_k5_matrix_bands_pinned_and_ca2dz() {
    let mut all = Vec::new();
    for mode in Mode::ALL {
        for plan in Plan::ALL {
            for seed in SEEDS {
                all.push(run_k5(&root(), mode, plan, seed).unwrap_or_else(|e| panic!("{e}")));
            }
        }
    }
    let mut behaviours: Vec<String> = all
        .iter()
        .map(|r| {
            format!(
                "{} {} {:?} {:?} {:?} {} {} {:?}",
                r.mode.name(),
                r.plan.name(),
                r.complete_tick,
                r.band_ticks,
                r.rescues,
                r.lane_rescues,
                r.tunnel_rescues,
                r.max_stuck
            )
        })
        .collect();
    behaviours.sort();
    behaviours.dedup();
    println!(
        "DZ_K5 distinct behaviours: {} of {}",
        behaviours.len(),
        all.len()
    );
    assert!(
        behaviours.len() > Mode::ALL.len() * Plan::ALL.len(),
        "the seed axis is degenerate"
    );
    for band in [Band::CatchUp, Band::Cruise, Band::Mercy] {
        assert!(
            all.iter().any(|r| r.band_ticks[band.index()] > 0),
            "the band {band:?} is never entered"
        );
    }
    // dz_pinned_never_in_room3
    let pinned: u32 = all.iter().map(|r| r.pinned_holds).sum();
    assert_eq!(pinned, 0, "dz_pinned_never_in_room3: {pinned} pinned holds");
    println!(
        "CA2-DZ: counts {:?} assisted per run {:.2} verdict {:?}",
        Ca2DzCounts::of(&all),
        Ca2DzCounts::of(&all).assisted_per_run(),
        ca2dz_verdict(&all)
    );
}

// ------------------------------------------------------------- rescue --

#[test]
fn dz_rescue_leader_in_tunnel_replays() {
    replay(
        "dz_rescue_leader_in_tunnel",
        dz_rescue_leader_in_tunnel,
        0x72a502603eb7d736,
    );
}

#[test]
fn dz_rescue_at_leader_apex_replays() {
    replay(
        "dz_rescue_at_leader_apex",
        dz_rescue_at_leader_apex,
        0x8c9a7b894c98d1c0,
    );
}

#[test]
fn dz_rescue_lane_spot_replays() {
    replay(
        "dz_rescue_lane_spot",
        dz_rescue_lane_spot,
        0xdec8a50995ba6d7e,
    );
}

#[test]
fn dz_rescue_no_loop_replays() {
    replay("dz_rescue_no_loop", dz_rescue_no_loop, 0x75491ae2e03c6a12);
}

#[test]
fn dz_rescue_behind_axle_replays() {
    replay(
        "dz_rescue_behind_axle",
        dz_rescue_behind_axle,
        0x9e7154749ade73b7,
    );
}

// --------------------------------------------------------- escape end --

#[test]
fn dz_escaped_wheel_off_replays() {
    replay(
        "dz_escaped_wheel_off",
        dz_escaped_wheel_off,
        0xebbd3d341e50181e,
    );
}

#[test]
fn dz_stop_face_safe_replays() {
    replay("dz_stop_face_safe", dz_stop_face_safe, 0x1e4a0ce496f0a1ab);
}

#[test]
fn dz_visible_lead_replays() {
    let min_lead = u64::from(crate::dozer::DozerTuning::active().dzq23_min_visible_lead_ticks);
    let mut smallest = None;
    replay(
        "dz_visible_lead",
        |r| {
            let (h, lead, at) = dz_visible_lead(r)?;
            smallest = Some((lead, at));
            Ok(h)
        },
        0xa865f66786edc886,
    );
    let (lead, at) = smallest.unwrap();
    println!("dz_visible_lead: the smallest lead is {lead} ticks (Carl at x {at})");
    assert!(lead >= min_lead);
}

// ------------------------------------------------------------ DOZR-05 --

#[test]
fn dz_hull_clang_replays() {
    replay("dz_hull_clang", dz_hull_clang, 0x8b8ceff2e9fe6d46);
}

#[test]
fn dz_wheel_strike_replays() {
    replay("dz_wheel_strike", dz_wheel_strike, 0xfc69723209a0ae7e);
}

#[test]
fn dz_blast_on_hull_replays() {
    replay("dz_blast_on_hull", dz_blast_on_hull, 0x9d2e126274356514);
}

fn no_jump_over(case: &str, committed: u64) {
    let (_, mode, flags) = jump_cases()
        .into_iter()
        .find(|(name, _, _)| *name == case)
        .unwrap();
    let hull_top = crate::dozer::DozerTuning::active().dzq02_push_box.y1 as f32;
    let mut highest = 0.0;
    replay(
        &format!("dz_no_jump_over_{case}"),
        |r| {
            let (h, top) = dz_no_jump_over(r, mode, flags)?;
            highest = top;
            Ok(h)
        },
        committed,
    );
    println!("dz_no_jump_over_{case}: the feet rose at most {highest:.1} px over the machine");
    assert!(highest + crate::dozer::HULL_APEX_MARGIN_PX <= hull_top);
}

#[test]
fn dz_no_jump_over_carl() {
    no_jump_over("carl", 0x80b6318ab3d8114f);
}

#[test]
fn dz_no_jump_over_donut() {
    no_jump_over("donut", 0x0447f59023577cad);
}

#[test]
fn dz_no_jump_over_donut_boosted() {
    no_jump_over("donut_boosted", 0x889e68a3195ae525);
}

// ------------------------------------------------------------ DOZR-07 --

#[test]
fn dz_retry_dormant_replays() {
    replay("dz_retry_dormant", dz_retry_dormant, 0x23f59fc80648127a);
}

#[test]
fn dz_retry_chase_replays() {
    replay("dz_retry_chase", dz_retry_chase, 0x3673581ff2a45255);
}
