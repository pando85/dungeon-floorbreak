//! PR-A of the Floor 1 fix-all plan: the CA2 metric fixes (FA-Q25 to FA-Q28) in
//! the closing arena (`f01_mastery`, `cf02_free`, seeds 0..8), and the knob-off
//! identity of every fix-all flag there.

/// The closing arena's solo Carl row (`cf02_assist` partner) as it was before PR-A.
const OLD_ARENA_CARL_SOLO: [u64; 9] = [
    0x7777_7d11_2fa1_50d7,
    0xe399_d734_d95d_df83,
    0x5dcf_f6b2_8acc_9637,
    0x7386_55ae_e984_ec68,
    0x6315_1d85_712a_a683,
    0x14d8_c509_81c9_652b,
    0x8404_727c_6e86_fbf0,
    0x8855_c693_08a7_6f8e,
    0x15bf_6232_1dbb_4493,
];

/// The closing arena's `cf02_free` row as it was before PR-A.
const OLD_ARENA_COMPANION: [u64; 9] = [
    0xfaa2_a0ee_3dca_42ae,
    0x27e9_7530_db5a_88ec,
    0x7827_ae2f_afb7_8b93,
    0xd8f9_794a_597d_2062,
    0xed76_07ce_0ac9_2ea0,
    0xc288_461a_bc42_bdf8,
    0xaebe_9b55_7f9f_f9c2,
    0x360b_78f2_e5e4_1630,
    0x83a2_685d_3cac_ff08,
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
/// 0..8), re-measured on the PR-C base with the PR #250 review's rulings:
///
/// - **M4 (steals) is 0 on 9 of 9** (FA-Q26, now by Carl's real engage check,
///   FA-Q26-AD1).
/// - **M2e (FA-Q25-AD1): shadowing over the ticks her goal is Escort or Idle, target
///   at most 20 %: met on 9 of 9** (0 % over 27 to 82 ticks per seed). **M2-all** is
///   reported with a regression guard at the measured maximum plus 5 points: 41 /
///   29 / 33 / 32 / 30 / 42 / 32 / 32 / 62 % (guard 67). It is mostly joint attack on
///   the goblin he walks to, which is not the escort glue FA-Q25 targets; whether
///   she still reads as glued joins the owner's T36 verdict.
/// - **M10 (FA-Q27-AD2)**: the arena never makes her retreat (0 retreats on every
///   seed), so it is *not exercised* here; it is measured in the Juicer run of
///   FA-Q07 (60 ticks after each Retreat she is 111 px from the boss on 9 of 9,
///   target 96) and on `cf02_free_retreat_when_hurt` (bound back at 96).
/// - **M13 (FA-Q28-AD1): her landed hits, glanced ones included, against the
///   `cf02_assist` hits, on at least 7 of 9: shipped as measured** (the count prints
///   below). A withheld kill is the rule working; the figure still misses on
///   most seeds, and rating her contribution is the owner's (T36, CA2-FU-M13).
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
        assert!(
            m.shadow_escort_percent() <= 20,
            "M2e (FA-Q25-AD1), seed {seed}: {}%",
            m.shadow_escort_percent()
        );
        assert!(m.shadow_percent() <= 67, "M2-all guard, seed {seed}");
        assert_eq!(m.retreats, 0, "M10 is not exercised in the arena");
        assert!(m.hit_share_percent() <= 50, "M13 share, seed {seed}");
        assert!(m.retreat_share_percent() <= 25, "M15, seed {seed}");
        assert!(m.her_landed() >= 3, "M13 guard, seed {seed}");
        her.push(m.her_landed());
        assist.push(assist_hits);
    }
    let met = her.iter().zip(&assist).filter(|(a, b)| a >= b).count();
    println!("FA-Q28: her landed hits >= the assistant's on {met}/9 (target 7/9)");
}

/// FA-Q16: the AI partner never hits a setpiece prop it is not asked to: in the
/// live Boiler's Direct plan (the pilot touches no prop) the partner's prop hits
/// are 0 on every seed in all three hero modes, against 5 (`cf02_assist`) and 1
/// (`cf02_free`) per run with the flag off (3 before PR-C's wake moved the room).
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
    assert_eq!(total(false), [45, 45, 9], "FA-Q16 off: the old behaviour");
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
