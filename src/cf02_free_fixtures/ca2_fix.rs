//! PR-A of the Floor 1 fix-all plan: the CA2 metric fixes (FA-Q25 to FA-Q28) in
//! the closing arena (`f01_mastery`, `cf02_free`, seeds 0..8), and the knob-off
//! identity of every fix-all flag there.

/// The closing arena's solo Carl row (`cf02_assist` partner) as it was before PR-A.
const OLD_ARENA_CARL_SOLO: [u64; 9] = [
    0x8a0f_e792_7530_65f6,
    0xd369_6e2d_8ad6_f931,
    0x3255_f01a_be82_377d,
    0x08bf_ca31_3a9a_fbae,
    0xf0e1_f76d_ca31_d6c0,
    0x6b5a_006a_3151_66b6,
    0x77c5_25ab_8ad7_d7d5,
    0xeb99_a639_92dd_edc2,
    0x9459_94ed_88c5_1e8e,
];

/// The closing arena's `cf02_free` row as it was before PR-A.
const OLD_ARENA_COMPANION: [u64; 9] = [
    0x5d4b_d23c_4866_23ac,
    0x594e_3503_067b_8480,
    0x9c73_45ca_fed8_467d,
    0xed30_d09b_b00f_bf1a,
    0x91dc_e607_19a8_c68e,
    0x6e7a_d5b5_7362_f147,
    0xebfd_ae24_95a6_68d4,
    0x3af1_8c36_ed1f_b3c6,
    0x011c_f171_1af0_ee18,
];

fn all_knobs_off(f: &mut crate::companion::profile::ProfileFile) {
    f.cf02_free.fix_all = crate::companion::profile::FixAll::default();
    f.cf02_assist.fix_all = crate::companion::profile::FixAll::default();
}

fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Knob off: with every fix-all flag off the closing arena's `cf02_free` and
/// `cf02_assist` runs hash as they did before PR-A, on every seed (FA-Q15 and
/// FA-Q25 to FA-Q29 are all off together).
#[test]
fn the_closing_arena_with_the_fix_all_knobs_off_keeps_its_old_hashes() {
    use crate::f01_mastery_fixtures::{Who, run};
    crate::combat_scene::with_profile_override(all_knobs_off, || {
        for seed in 0..9u64 {
            let carl = run(&root(), Who::CarlSolo, seed).unwrap().hash;
            assert_eq!(
                carl, OLD_ARENA_CARL_SOLO[seed as usize],
                "carl_solo seed {seed}"
            );
            let free = run(&root(), Who::Companion, seed).unwrap().hash;
            assert_eq!(
                free, OLD_ARENA_COMPANION[seed as usize],
                "companion seed {seed}"
            );
        }
    });
}

/// FA-Q25 to FA-Q28 in the closing arena (`f01_mastery`, `cf02_free`, seeds
/// 0..8): M4 (steals) is 0 on 9 of 9. The other rows are FA-Q30 targets that
/// **miss** after the allowed iterations and ship as measured:
///
/// - M2 (shadowing, target at most 20 %): 60 / 31 / 31 / 52 / 34 / 60 / 52 / 52 /
///   52 % before, 60 / 31 / 31 / 54 / 34 / 50 / 54 / 54 / 54 % after. Two tuning
///   iterations changed nothing useful: the first (the shadow re-pick also for
///   Hunt, threshold 12) brought seeds 0 and 4 to 21 % and seeds 1 to 3 and 6 to
///   8 up to 56 %, the second (threshold 6) was identical; both were dropped for
///   the Escort-only re-pick. She shadows him mostly while hunting the goblin he
///   walks to, which an escort re-pick does not reach.
/// - M10 (retreat distance at least 96 px): the arena never makes her retreat
///   (0 retreats on every seed, before and after), so the figure is not exercised
///   there; the rule is pinned by the unit tests.
/// - M13 (her hits at least the `cf02_assist` hits on at least 7 of 9): 4 to 7
///   against 7 to 9. M4 = 0 turns her lethal hits on his enemies into glances,
///   which the share counts as no hit.
///
/// The bounds below are regression pins of the measured values.
#[test]
fn fa_q25_to_q28_the_ca2_metrics_in_the_closing_arena_as_measured() {
    use crate::f01_mastery_fixtures::{metrics, metrics_line};
    let (mut her, mut assist) = (Vec::new(), Vec::new());
    for seed in 0..9u64 {
        let (m, assist_hits) = metrics(&root(), seed).unwrap();
        println!("{}", metrics_line(seed, &m, assist_hits));
        let (again, _) = metrics(&root(), seed).unwrap();
        assert_eq!(m, again, "twice identical, seed {seed}");
        assert_eq!(m.steals + m.start_steals, 0, "FA-Q26: M4 is 0, seed {seed}");
        assert!(m.shadow_percent() <= 60, "M2 regression pin, seed {seed}");
        assert!(m.retreat_distances.iter().flatten().all(|d| *d >= 90.0));
        assert!(m.hit_share_percent() <= 50, "M13 share, seed {seed}");
        assert!(m.retreat_share_percent() <= 25, "M15, seed {seed}");
        her.push(m.her_hits);
        assist.push(assist_hits);
    }
    let met = her.iter().zip(&assist).filter(|(a, b)| a >= b).count();
    println!("FA-Q28: her hits >= the assistant's on {met}/9 (target 7/9)");
}

/// FA-Q16: the AI partner never hits a setpiece prop it is not asked to: in the
/// live Boiler's Direct plan (the pilot touches no prop) the partner's prop hits
/// are 0 on every seed in all three hero modes, against 3 (`cf02_assist`) and 1
/// (`cf02_free`) per run with the flag off.
#[test]
fn fa_q16_the_partner_never_hits_the_valve_in_the_live_direct_boiler() {
    use crate::boiler_fixtures::scene_pilots::{Plan, Who, run_live};
    let total = |knob: bool| {
        let edit = move |f: &mut crate::companion::profile::ProfileFile| {
            f.cf02_free.fix_all.fa_q16_prop_safe_fire = knob;
            f.cf02_assist.fix_all.fa_q16_prop_safe_fire = knob;
        };
        crate::combat_scene::with_profile_override(edit, || {
            let mut per_mode = Vec::new();
            for who in [Who::CarlSolo, Who::DonutBySwap, Who::Companion] {
                let mut hits = 0;
                for seed in 0..=8u64 {
                    let r = run_live(&root(), who, Plan::Direct, seed).unwrap();
                    assert!(r.exit_tick.is_some(), "{who:?} seed {seed}: cleared");
                    hits += r.prop_hits_by(1);
                }
                per_mode.push(hits);
            }
            per_mode
        })
    };
    assert_eq!(total(true), [0, 0, 0], "FA-Q16 on");
    assert_eq!(total(false), [27, 27, 9], "FA-Q16 off: the old behaviour");
}

/// FA-Q16: the geometry of the veto: a span that overlaps a prop target at her
/// height crosses it, on both edges and for both facings; one beside or above it
/// does not.
#[test]
fn fa_q16_the_press_path_test_is_exact_at_the_edges() {
    use crate::combat_scene::path_crosses;
    use crate::prototype::Rect;
    // The valve at x 464..496, on the floor (feet at y 100).
    let valve = [Rect {
        left: 464.0,
        bottom: 100.0,
        width: 32.0,
        height: 40.0,
    }];
    assert!(
        path_crosses(&valve, (440.0, 465.0), 100.0),
        "right-facing, one px in"
    );
    assert!(
        !path_crosses(&valve, (440.0, 464.0), 100.0),
        "ends on the left edge"
    );
    assert!(
        path_crosses(&valve, (495.0, 520.0), 100.0),
        "left-facing, one px in"
    );
    assert!(
        !path_crosses(&valve, (496.0, 520.0), 100.0),
        "starts on the right edge"
    );
    // Well above it (a jump): no. A hero standing on it: its top is her feet.
    assert!(!path_crosses(&valve, (440.0, 520.0), 200.0));
    assert!(!path_crosses(&valve, (440.0, 520.0), 140.0 + 8.0 + 1.0));
    assert!(!path_crosses(&[], (0.0, 640.0), 100.0));
}

/// FA-Q15: in the hazard room (room 4) the `cf02_assist` partner is fenced from
/// an awake sticker's span: 0 partner Rot hurts on 9 of 9 seeds with solo Carl
/// and with Donut by swap (1 per run with the flag off), and every run clears.
#[test]
fn fa_q15_the_assist_partner_takes_no_rot_hurt_in_the_hazard_room() {
    use crate::hazards_fixtures::pilots::{HurtSource, Who, run};
    let rot_hurts = |knob: bool, who: Who| {
        crate::combat_scene::with_profile_override(
            move |f| f.cf02_assist.fix_all.fa_q15_assist_rot_avoidance = knob,
            || {
                (0..=8u64)
                    .map(|seed| {
                        let r = run(&root(), who, seed, true).unwrap();
                        assert!(r.cleared_tick.is_some(), "{who:?} seed {seed}: cleared");
                        assert!(r.exit_tick.is_some(), "{who:?} seed {seed}: exit");
                        r.ai_hurts_by(HurtSource::Rot)
                    })
                    .collect::<Vec<_>>()
            },
        )
    };
    for who in [Who::CarlSolo, Who::DonutBySwap] {
        assert_eq!(rot_hurts(true, who), [0; 9], "{who:?}");
        assert_eq!(rot_hurts(false, who), [1; 9], "{who:?}: the old behaviour");
    }
}
