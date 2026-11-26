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
    (Mode::CarlSolo, Plan::Run, 0x91a2edd3267eb68f),
    (Mode::CarlSolo, Plan::Stall, 0xea39b87fa67dc1cd),
    (Mode::CarlSolo, Plan::CloseCall, 0x7e05509cd6581b8f),
    (Mode::CarlSolo, Plan::Wait, 0x4f5c29befa7dea42),
    (Mode::DonutSolo, Plan::Run, 0x3752981d69ebc9eb),
    (Mode::DonutSolo, Plan::Stall, 0x7854bfd94ab9288d),
    (Mode::DonutSolo, Plan::CloseCall, 0x7dbe9c4b76ff613e),
    (Mode::DonutSolo, Plan::Wait, 0xf8c2ddac414bc67d),
    (Mode::TwoPlayer, Plan::Run, 0x33cb1fce84a6fdc7),
    (Mode::TwoPlayer, Plan::Stall, 0x0732509820863b93),
    (Mode::TwoPlayer, Plan::CloseCall, 0xfd8f460c4c862407),
    (Mode::TwoPlayer, Plan::Wait, 0x757de1a073844255),
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
        0xe753b488533b39a6,
    );
}

#[test]
fn dz_rescue_at_leader_apex_replays() {
    replay(
        "dz_rescue_at_leader_apex",
        dz_rescue_at_leader_apex,
        0xa61577cded2248fa,
    );
}

#[test]
fn dz_rescue_lane_spot_replays() {
    replay(
        "dz_rescue_lane_spot",
        dz_rescue_lane_spot,
        0xec4d6980723b7b36,
    );
}

#[test]
fn dz_rescue_no_loop_replays() {
    replay("dz_rescue_no_loop", dz_rescue_no_loop, 0xdd0e7a5a0f61738e);
}

#[test]
fn dz_rescue_behind_axle_replays() {
    replay(
        "dz_rescue_behind_axle",
        dz_rescue_behind_axle,
        0x1e909024663360cb,
    );
}

// --------------------------------------------------------- escape end --

#[test]
fn dz_escaped_wheel_off_replays() {
    replay(
        "dz_escaped_wheel_off",
        dz_escaped_wheel_off,
        0x124c2560d63a5642,
    );
}

#[test]
fn dz_stop_face_safe_replays() {
    replay("dz_stop_face_safe", dz_stop_face_safe, 0xc8675d5587259f47);
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
        0xb55091b6de38a3c8,
    );
    let (lead, at) = smallest.unwrap();
    println!("dz_visible_lead: the smallest lead is {lead} ticks (Carl at x {at})");
    assert!(lead >= min_lead);
}

// ------------------------------------------------------------ DOZR-05 --

#[test]
fn dz_hull_clang_replays() {
    replay("dz_hull_clang", dz_hull_clang, 0xf364908cb41367ee);
}

#[test]
fn dz_wheel_strike_replays() {
    replay("dz_wheel_strike", dz_wheel_strike, 0x95497d74188ff262);
}

#[test]
fn dz_blast_on_hull_replays() {
    replay("dz_blast_on_hull", dz_blast_on_hull, 0xa47f54985ee8bf3e);
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
    no_jump_over("carl", 0xf5cf87623c5cb63f);
}

#[test]
fn dz_no_jump_over_donut() {
    no_jump_over("donut", 0xe4dfeb47dd4b2419);
}

#[test]
fn dz_no_jump_over_donut_boosted() {
    no_jump_over("donut_boosted", 0x29188b69b2ab52dd);
}

// ------------------------------------------------------------ DOZR-07 --

#[test]
fn dz_retry_dormant_replays() {
    replay("dz_retry_dormant", dz_retry_dormant, 0x385114c2a5441bc3);
}

#[test]
fn dz_retry_chase_replays() {
    replay("dz_retry_chase", dz_retry_chase, 0x888a3bc74ebc2f3e);
}

// ------------------------------------------------- DR-B, DOZR-02 to 04 --

/// [`replay`] for a fixture returning a hash and a value: the value is the
/// same on both runs and is returned.
fn replay_with<T: PartialEq + std::fmt::Debug>(
    name: &str,
    f: impl Fn(&Path) -> Result<(u64, T), String>,
    committed: u64,
) -> T {
    let (a, va) = f(&root()).unwrap_or_else(|e| panic!("{name}: {e}"));
    let (b, vb) = f(&root()).unwrap_or_else(|e| panic!("{name} (replay): {e}"));
    assert_eq!((a, &va), (b, &vb), "{name}: the replay differs");
    assert_eq!(a, committed, "{name}: hash {a:#018x}");
    va
}

#[test]
fn dz_frame_window_sweep_replays() {
    let r = replay_with(
        "dz_frame_window_sweep",
        frame::dz_frame_window_sweep,
        0xa1d6a0c28486dbbb,
    );
    println!("dz_frame_window_sweep: {r:?}");
    let t = crate::dozer::DozerTuning::active();
    assert!(r.window.ticks >= u32::from(t.dzq23_min_release_window_ticks));
}

#[test]
fn dz_frame_early_miss_replays() {
    replay(
        "dz_frame_early_miss",
        frame::dz_frame_early_miss,
        0xfaff1a6c60104892,
    );
}

#[test]
fn dz_frame_disable_once_replays() {
    let r = replay_with(
        "dz_frame_disable_once",
        frame::dz_frame_disable_once,
        0xfb38882a666a83d6,
    );
    println!("dz_frame_disable_once: released on R {r}");
}

#[test]
fn dz_frame_rear_glance_replays() {
    let front = replay_with(
        "dz_frame_rear_glance",
        frame::dz_frame_rear_glance,
        0xe9a74a2daaa936f0,
    );
    println!("dz_frame_rear_glance: the glance at front {front}");
}

#[test]
fn dz_frame_dynamite_replays() {
    let r = replay_with(
        "dz_frame_dynamite",
        frame::dz_frame_dynamite,
        0xe61692ba0bcba6d0,
    );
    println!("dz_frame_dynamite: released on R {r}");
}

#[test]
fn dz_frame_missile_replays() {
    let r = replay_with(
        "dz_frame_missile",
        frame::dz_frame_missile,
        0x99084fa4f4351760,
    );
    println!("dz_frame_missile: released on R {r}");
}

// ------------------------------------------------- DR-C, DOZR-06 and 07 --

#[test]
fn dz_crew_bounded_replays() {
    let spots = replay_with("dz_crew_bounded", crew::dz_crew_bounded, 0x9662385c797c8658);
    println!("dz_crew_bounded: landed at {spots:?} (plain A, B; nudged A, B)");
}

#[test]
fn dz_respawn_after_disable_replays() {
    replay(
        "dz_respawn_after_disable",
        crew::dz_respawn_after_disable,
        0x8ff597d554b704d8,
    );
}

#[test]
fn dz_retry_stunned_replays() {
    replay(
        "dz_retry_stunned",
        crew::dz_retry_stunned,
        0xb9172884d5b16681,
    );
}

#[test]
fn dz_retry_disabled_replays() {
    replay(
        "dz_retry_disabled",
        crew::dz_retry_disabled,
        0x26df30727dd00b4e,
    );
}

/// The committed fold of the nine seeds' final hashes per solo mode.
const CREW_HASHES: [(Mode, u64); 2] = [
    (Mode::CarlSolo, 0x6ea71fa5fe6e9a9b),
    (Mode::DonutSolo, 0x7fe33291845521f9),
];

/// The companion crew pilots: every seed disables the machine and completes
/// with at most 3 living enemies and no stuck hero, twice, and the fold of
/// each mode matches.
#[test]
fn dz_crew_companion_pilots() {
    let mut mismatches = Vec::new();
    for (mode, committed) in CREW_HASHES {
        let mut folds = [Vec::new(), Vec::new()];
        for seed in SEEDS {
            for (pass, fold_of) in folds.iter_mut().enumerate() {
                let r =
                    crew::dz_crew_companion(&root(), mode, seed).unwrap_or_else(|e| panic!("{e}"));
                r.check().unwrap_or_else(|e| panic!("{e}"));
                fold_of.push(r.hash);
                if pass == 0 {
                    println!("{}", r.line());
                }
            }
        }
        assert_eq!(
            folds[0],
            folds[1],
            "dz_crew {}: the replay differs",
            mode.name()
        );
        let got = fold(folds[0].iter().copied());
        if got != committed {
            mismatches.push(format!("dz_crew_{}: hash {got:#018x}", mode.name()));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
