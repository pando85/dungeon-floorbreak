//! PT-GATE-C2: the HAZ-C gate fixtures of the live hazard room (HAZ-06 and
//! HAZ-09, `docs/01-design/enemy-gameplay-slime-rot.md` section 18) and the
//! reach and no-softlock assertion over its sidecar. The pilot is
//! [`super::pilots`]; every run replays twice to the same state hash and its
//! final hash is pinned (new fixtures: the pins are first pins).
//! `cargo test haz_room -- --nocapture` prints the per-seed table of the PR.

use super::pilots::{Report, SEEDS, STUCK_LIMIT, Who, run, run_with};
use super::*;
use crate::encounter::ReachHero;

/// One run, twice: the table line goes to stderr, the replay must agree.
fn run_twice_checked(who: Who, seed: u64, rot: bool) -> Report {
    let a = run(&root(), who, seed, rot).expect("the pilot runs");
    let b = run(&root(), who, seed, rot).expect("the pilot runs");
    assert_eq!(
        a.hash,
        b.hash,
        "{} seed {seed} rot {rot}: replay",
        who.name()
    );
    assert_eq!(
        a.events,
        b.events,
        "{} seed {seed} rot {rot}: events",
        who.name()
    );
    eprintln!("HAZ_ROOM {}", a.line());
    a
}

fn assert_cleared(r: &Report) {
    let name = r.who.name();
    assert!(
        r.cleared_tick.is_some(),
        "{name} seed {} rot {}: not cleared",
        r.seed,
        r.rot_enabled
    );
    assert!(
        r.exit_tick.is_some(),
        "{name} seed {} rot {}: exit not reached",
        r.seed,
        r.rot_enabled
    );
    assert!(
        r.max_stuck.iter().all(|s| *s <= STUCK_LIMIT),
        "{name} seed {} rot {}: stuck {:?}",
        r.seed,
        r.rot_enabled,
        r.max_stuck
    );
}

/// HAZ-06 (`haz_room_direct_route`): a melee-only route clears every slime and
/// goblin and reaches the exit; the same route also clears with the Rot kind
/// removed (`hazq02_rot_enabled` false), so no step depends on a sticker blast.
/// The pilot never presses the ability (it is melee only, with the partner's
/// bolts as the only ranged damage), and it is Carl's route at every seed.
#[test]
fn haz_room_direct_route() {
    let mut pins = Vec::new();
    for rot in [true, false] {
        // The kill-switch build really has no sticker, the shipped one has both.
        let (_, stickers) = pilots::start(&root(), Who::CarlSolo, 0, rot).unwrap();
        assert_eq!(stickers, if rot { 2 } else { 0 }, "rot {rot}");
        for seed in SEEDS {
            let r = run_twice_checked(Who::CarlSolo, seed, rot);
            assert_cleared(&r);
            assert_eq!(r.deaths, 0, "seed {seed} rot {rot}");
            if !rot {
                assert!(r.provokes.is_empty(), "no sticker, no provoke");
                assert_eq!(r.rot_blasts_hurting_anyone(), 0);
            }
            pins.push(r.hash);
        }
    }
    assert_eq!(pins, DIRECT_PINNED, "pins: {pins:#018x?}");
}

/// FIX-TOOL: the Rot-off fixtures use the real switch. The sticker props stay in the env as
/// `Machine::Rot` (the prop count and every prop index are the Rot-on room's), and only
/// the sticker state is gone; the pilot, the room and the env solids are the same.
#[test]
fn the_rot_off_room_keeps_its_sticker_props_and_drops_only_their_state() {
    let (on, on_stickers) = pilots::start(&root(), Who::CarlSolo, 0, true).unwrap();
    let (off, off_stickers) = pilots::start(&root(), Who::CarlSolo, 0, false).unwrap();
    assert_eq!((on_stickers, off_stickers), (2, 0));
    let (on_env, off_env) = (on.combat.env().unwrap(), off.combat.env().unwrap());
    assert_eq!(on_env.props(), off_env.props(), "no prop is removed");
    assert_eq!(on_env.solids().len(), off_env.solids().len());
    assert!(off_env.stickers().next().is_none());
    assert!(off_env.sticker(0).is_none() && off_env.sticker(1).is_none());
    // The shipped room with the switch on is untouched by the injection.
    let shipped = crate::f01_hazards::env_runtime_with_rot(
        &crate::f01_hazards::launch(&root()).unwrap(),
        crate::rot_sticker::active().enabled(),
    )
    .unwrap();
    assert_eq!(shipped.sticker_count(), 2);
}

fn solo_table(who: Who, pinned: [u64; 9]) {
    let mut pins = Vec::new();
    for seed in SEEDS {
        let r = run_twice_checked(who, seed, true);
        assert_cleared(&r);
        pins.push(r.hash);
    }
    assert_eq!(pins, pinned, "{} pins: {pins:#018x?}", who.name());
}

/// HAZ-09: solo Carl (`cf02_assist` partner) clears at every seed. It is the same
/// route as `haz_room_direct_route` with the Rot kind present, so its nine hashes
/// equal the first nine of `DIRECT_PINNED`.
#[test]
fn haz_room_solo_carl() {
    solo_table(Who::CarlSolo, SOLO_CARL_PINNED);
}

/// HAZ-09: solo Donut by swap: Carl clears the roster, the pilot swaps on the tick
/// after the clear and Donut walks to the exit (Carl the AI partner); no human P2
/// is needed.
#[test]
fn haz_room_solo_donut() {
    solo_table(Who::DonutBySwap, SOLO_DONUT_PINNED);
}

/// The harder solo-Donut probe (FA-Q14, PR-D): Donut plays the whole room from tick 90
/// (Carl is the AI partner) and clears it at every seed. It was `#[ignore]`d while her
/// Magic Missile spawned 27 px ahead and missed a goblin closer than that; the
/// `missile_spawn_sweep` knob of PR-B (FA-Q14, default on) sweeps the segment from her
/// front edge to the spawn point on the cast tick, and the pilot of FA-Q12 plays her.
/// The room has no random stream under this pilot and the route never reaches a morale
/// draw, so before the follow-up the nine seeds were one fight (cleared on tick 551, exit
/// on 633 at every seed). FA-Q14 ruling 2: the pilot now offsets Donut's press phase by
/// `seed % 4` ticks (FA-Q17's treatment, scoped to this pilot), and the nine runs show
/// four distinct clear ticks. The nine hashes are re-pinned.
#[test]
fn haz_room_donut_fights() {
    solo_table(Who::DonutFights, DONUT_FIGHTS_PINNED);
}

/// FA-Q14 ruling 2: the seed phase varies the fight (at least 3 distinct clear ticks over
/// seeds 0..=8) and the room still clears 9 of 9 with no death; with the phase off the
/// nine seeds are one fight again and replay the first pins.
#[test]
fn haz_room_donut_fights_varies_by_seed_and_still_clears_every_seed() {
    let mut clears = std::collections::BTreeSet::new();
    let mut outcomes = std::collections::BTreeSet::new();
    for seed in SEEDS {
        let r = run_twice_checked(Who::DonutFights, seed, true);
        assert_cleared(&r);
        assert_eq!(r.deaths, 0, "seed {seed}");
        clears.insert(r.cleared_tick);
        outcomes.insert((r.cleared_tick, r.exit_tick, r.hurts.len()));
    }
    assert!(clears.len() >= 3, "distinct clear ticks: {clears:?}");
    assert!(outcomes.len() >= 3, "distinct outcomes: {outcomes:?}");
    // Phase 0 is the old pilot byte for byte: seeds 0, 4 and 8 keep their first pins.
    for seed in [0u64, 4, 8] {
        let r = run(&root(), Who::DonutFights, seed, true).unwrap();
        assert_eq!(
            r.hash, DONUT_FIGHTS_PINNED_NO_PHASE[seed as usize],
            "seed {seed}"
        );
    }
    // Phase off: the nine seeds are one fight again, with the first pins.
    let mut ticks = std::collections::BTreeSet::new();
    for seed in SEEDS {
        let r = run_with(&root(), Who::DonutFights, seed, true, false).unwrap();
        assert_eq!(
            r.hash, DONUT_FIGHTS_PINNED_NO_PHASE[seed as usize],
            "seed {seed}"
        );
        ticks.insert((r.cleared_tick, r.exit_tick));
    }
    assert_eq!(ticks.len(), 1, "{ticks:?}");
    // Other pilots never read the phase: Carl's solo run is the same either way.
    let a = run(&root(), Who::CarlSolo, 3, true).unwrap().hash;
    let b = run_with(&root(), Who::CarlSolo, 3, true, false)
        .unwrap()
        .hash;
    assert_eq!(a, b);
}

/// HAZ-09 with `cf02_free`: Carl is the pilot's, Donut the autonomous partner.
#[test]
fn haz_room_companion() {
    solo_table(Who::Companion, COMPANION_PINNED);
}

/// The reach matrix of the shipped hazard sidecar (the validator's own output,
/// every class and every destruction state), mirroring the Boiler's
/// `the_reach_matrix_covers_every_class_and_state_and_all_pass`: Carl, base
/// Donut and boosted Donut each reach the exit along the ground lane alone, the
/// two spawns share the open stretch (the 2P regroup), and every perch whose
/// gate class a hero meets is reachable and returnable. The room has no changeable prop
/// (a sticker's soffit is a static solid), so there is exactly one state per
/// class: no destruction state can softlock a hero.
#[test]
fn haz_room_reach_matrix_has_no_softlock_for_any_class() {
    let run = f01_hazards::launch(&root()).unwrap();
    let e = &run.loaded.encounter;
    assert_eq!(e.reach.len(), ReachHero::ALL.len(), "{:?}", e.reach);
    for hero in ReachHero::ALL {
        let rows: Vec<_> = e.reach.iter().filter(|r| r.hero == hero).collect();
        assert_eq!(rows.len(), 1, "{hero:?}: one state, no changeable prop");
        let row = rows[0];
        assert_eq!(row.state, 0);
        assert!(row.exit, "{hero:?} reaches the exit: {row:?}");
        assert!(row.lane, "{hero:?} walks the ground lane alone: {row:?}");
        assert!(row.regroup, "2P regroup for {hero:?}: {row:?}");
    }
    // Carl (the gate class of both perches) reaches and returns from each.
    let carl = e.reach.iter().find(|r| r.hero == ReachHero::Carl).unwrap();
    assert_eq!(carl.perches.len(), 2, "{carl:?}");
    assert!(carl.perches.iter().all(|p| *p == (true, true)), "{carl:?}");
}

/// The assertion above can fail: the same matrix with a wall across the whole
/// lane (a static solid at x 600, the room's full height) cuts the ground lane
/// for every class (a perch route may still reach the exit), and without the wall
/// it passes.
#[test]
fn haz_room_reach_matrix_catches_a_wall_across_the_lane() {
    let run = f01_hazards::launch(&root()).unwrap();
    let loaded = &run.loaded;
    let props = &loaded.encounter.props;
    let open = crate::encounter::reach_matrix(&loaded.room, &loaded.classes, props, &[], &[]);
    assert!(
        open.iter().all(|r| r.exit && r.lane && r.regroup),
        "{open:?}"
    );
    let wall = IRect::new(600, 0, 16, 700);
    let cut = crate::encounter::reach_matrix(&loaded.room, &loaded.classes, props, &[], &[wall]);
    for hero in ReachHero::ALL {
        let row = cut.iter().find(|r| r.hero == hero).unwrap();
        assert!(!row.lane, "{hero:?} must not walk past the wall: {row:?}");
    }
}

/// Final state hashes (`PartyFixtureSim::state_hash`) of the direct route: the
/// Rot kind present, then switched off, seeds 0..=8. First pins of new fixtures. FIX-TOOL
/// re-pinned the nine Rot-off hashes (the second nine) when the fixture moved from a room
/// with the sticker props removed to the real kill switch (`EnvRuntime::new_with_rot`:
/// the props stay as `Machine::Rot` without a sticker state); the outcomes are unchanged,
/// and the nine Rot-on hashes did not move. The old and new values are listed in the PR that moved them.
const DIRECT_PINNED: [u64; 18] = [
    0x1732_2c2f_4963_7b35,
    0xdb89_2cec_ea2f_bcb3,
    0x7489_3510_9ad5_dd73,
    0x5a4d_9ec3_b170_069e,
    0x7c98_b011_b795_c321,
    0x676b_cc53_bc0d_1443,
    0xd5f3_41e2_cc02_69a8,
    0x3459_49fa_92e8_4799,
    0xe35f_57c1_3201_ffa4,
    0x94c1_214a_4bd7_afe9,
    0xa652_739a_8c15_38f9,
    0x0a2b_088c_fa1e_2243,
    0x0f18_753f_67d3_717a,
    0xd5d8_5ccb_a1ef_882e,
    0xae56_4957_70d7_2ed1,
    0x3b16_983e_75ef_a99d,
    0xcd16_463c_b787_ad0d,
    0x50ae_43e9_a860_3e69,
];
pub(crate) const SOLO_CARL_PINNED: [u64; 9] = [
    0x1732_2c2f_4963_7b35,
    0xdb89_2cec_ea2f_bcb3,
    0x7489_3510_9ad5_dd73,
    0x5a4d_9ec3_b170_069e,
    0x7c98_b011_b795_c321,
    0x676b_cc53_bc0d_1443,
    0xd5f3_41e2_cc02_69a8,
    0x3459_49fa_92e8_4799,
    0xe35f_57c1_3201_ffa4,
];
const SOLO_DONUT_PINNED: [u64; 9] = [
    0x7572_3337_2c34_8c8b,
    0x2d84_ed17_2d96_4c59,
    0x0f1b_3947_ca6a_bab9,
    0xe4cb_608d_ba67_3368,
    0x386f_5504_1a1b_1c27,
    0x8c66_62b1_9008_d769,
    0x5ba0_7b39_d059_1e2b,
    0x422d_01ba_f97f_158e,
    0x530e_b5f2_d488_727b,
];
/// The hazard room hashes before PR-A (main at the PR-B merge): with every fix-all knob
/// off the room replays them (FA-Q15, FA-Q16).
const DIRECT_PINNED_KNOBS_OFF: [u64; 18] = [
    0x1d8a_7cab_ceff_8420,
    0x39fb_ed35_46a0_4a46,
    0x04a6_2a41_368d_8218,
    0xffa1_cb72_f89c_e4ca,
    0x0bfb_e5f1_af3e_f6e2,
    0xace7_9530_464c_2b6f,
    0xe308_29f7_5387_2a85,
    0xe715_aadb_b058_b65f,
    0x486f_87ca_9c4b_b527,
    0x8a5c_af4a_0a78_16c7,
    0x924b_bfa1_e288_9455,
    0xeff9_d5cc_1fb9_7e4c,
    0x226c_c0fa_eec0_b95c,
    0x4ab8_b7d5_94a7_6d85,
    0xae18_3198_0c4f_7fc7,
    0xeff4_30e8_7baa_83ca,
    0x5f17_277d_e0dc_27eb,
    0x5990_bc2a_72b9_be5e,
];
const SOLO_CARL_PINNED_KNOBS_OFF: [u64; 9] = [
    0x1d8a_7cab_ceff_8420,
    0x39fb_ed35_46a0_4a46,
    0x04a6_2a41_368d_8218,
    0xffa1_cb72_f89c_e4ca,
    0x0bfb_e5f1_af3e_f6e2,
    0xace7_9530_464c_2b6f,
    0xe308_29f7_5387_2a85,
    0xe715_aadb_b058_b65f,
    0x486f_87ca_9c4b_b527,
];
const SOLO_DONUT_PINNED_KNOBS_OFF: [u64; 9] = [
    0x2572_dd09_b087_e632,
    0x330a_1554_c17d_b294,
    0x89e2_1099_5538_9e5a,
    0x225f_8707_69ed_ada8,
    0xa413_10d1_38c4_cbd0,
    0x85fe_3ea6_02f5_72d5,
    0x786c_8a70_b401_6909,
    0x9e18_2455_7686_523b,
    0x0846_1fd3_b9d2_e7e3,
];
/// FA-Q14 (PR-D) first pins, re-pinned by the review follow-up (FA-Q14 ruling 2):
/// with the seed phase (`seed % 4` ticks on Donut's press) seeds 1, 2, 3, 5, 6 and 7
/// moved; seeds 0, 4 and 8 have phase 0 and keep their first pins.
const DONUT_FIGHTS_PINNED: [u64; 9] = [
    0x0ed1_1e96_74e1_7354,
    0xe79f_ffc3_2a48_8a72,
    0x87e9_731b_1672_e5d5,
    0xa6ed_8e39_f7ee_a9b9,
    0x203d_8b16_5243_ce0d,
    0xf877_3d65_9005_8e2c,
    0x122d_17fe_f0a3_1903,
    0xde65_79cc_4a50_5c80,
    0x2c56_bed8_c6a4_5145,
];
/// The first pins of PR-D: the pilot without the seed phase (nine seeds, one fight).
const DONUT_FIGHTS_PINNED_NO_PHASE: [u64; 9] = [
    0x0ed1_1e96_74e1_7354,
    0xcb56_1d14_e4a4_1784,
    0x9fae_f718_f59a_c7b8,
    0x1118_430c_5aec_d669,
    0x203d_8b16_5243_ce0d,
    0x1d12_e01e_00e0_bdb6,
    0x3a70_1378_f19d_b682,
    0xdadc_abce_096b_5138,
    0x2c56_bed8_c6a4_5145,
];
const DONUT_FIGHTS_PINNED_KNOBS_OFF: [u64; 9] = [
    0x8d5d_6885_d6b8_04c7,
    0xabe4_de8f_4347_fe57,
    0x5a73_3dab_4026_0b03,
    0x684b_bcd2_fd0f_8816,
    0xaea6_75ea_4daa_8272,
    0x1fc0_c183_d6e2_fc95,
    0x4bba_9a6d_6451_f259,
    0x5f2b_b532_9329_42c3,
    0xe4e1_d92b_48fd_ae1a,
];

/// FA-Q15 and FA-Q16 moved the three tables above; with every fix-all knob off the
/// hazard room replays the pre-PR-A hashes on every seed (the direct route with the
/// Rot kind on and off, solo Carl, solo Donut).
#[test]
fn the_hazard_room_with_the_fix_all_knobs_off_keeps_its_old_pins() {
    crate::combat_scene::with_profile_override(crate::combat_scene::fix_all_off, || {
        let mut direct = Vec::new();
        for rot in [true, false] {
            for seed in SEEDS {
                direct.push(run(&root(), Who::CarlSolo, seed, rot).unwrap().hash);
            }
        }
        assert_eq!(direct, DIRECT_PINNED_KNOBS_OFF, "direct: {direct:#018x?}");
        let donut: Vec<u64> = SEEDS
            .into_iter()
            .map(|seed| run(&root(), Who::DonutBySwap, seed, true).unwrap().hash)
            .collect();
        assert_eq!(donut, SOLO_DONUT_PINNED_KNOBS_OFF, "donut: {donut:#018x?}");
        assert_eq!(direct[..9], SOLO_CARL_PINNED_KNOBS_OFF[..]);
        let fights: Vec<u64> = SEEDS
            .into_iter()
            .map(|seed| {
                run_with(&root(), Who::DonutFights, seed, true, false)
                    .unwrap()
                    .hash
            })
            .collect();
        assert_eq!(
            fights, DONUT_FIGHTS_PINNED_KNOBS_OFF,
            "fights: {fights:#018x?}"
        );
    });
}

const COMPANION_PINNED: [u64; 9] = [
    0x5a9c_abdd_ecca_9386,
    0x6362_c33e_87fd_1513,
    0x453b_db9c_a28c_86c5,
    0x6ac9_1931_5f79_9c97,
    0x38a4_3dcc_1037_794b,
    0x7abd_7ebc_0512_c7be,
    0x2fb3_6b33_e09a_f44a,
    0xeb5b_b3a2_bf32_33cf,
    0xf29d_e1f8_1d65_3585,
];

/// FIX-TOOL (PT-FU-B8 item 10): the companion's "left behind" measure applied to the hero the
/// pilot does not steer, in every mode of the hazards room (the `cf02_assist` partner of the
/// solo runs, Carl after the swap, the `cf02_free` companion), seeds 0..=8: no partner is left
/// behind for more than the stuck limit. Measured: at most 5 ticks for the assist partner (solo
/// Carl and Donut by swap) and 29 for `cf02_free`. `cargo test partner_left_behind --
/// --nocapture` prints the table. The partner's AI is not tuned from it.
#[test]
fn the_uncontrolled_hero_is_never_left_behind_in_the_hazards_room() {
    for who in [Who::CarlSolo, Who::DonutBySwap, Who::Companion] {
        let v: Vec<u32> = SEEDS
            .map(|seed| run(&root(), who, seed, true).unwrap().partner_left_behind)
            .collect();
        eprintln!("PARTNER_LB hazards {} {v:?}", who.name());
        assert!(
            v.iter().all(|n| *n <= STUCK_LIMIT),
            "{}: the partner was left behind: {v:?}",
            who.name()
        );
    }
}
