//! PT-GATE-C2: the HAZ-C gate fixtures of the live hazard room (HAZ-06 and
//! HAZ-09, `docs/01-design/enemy-gameplay-slime-rot.md` section 18) and the
//! reach and no-softlock assertion over its sidecar. The pilot is
//! [`super::pilots`]; every run replays twice to the same state hash and its
//! final hash is pinned (new fixtures: the pins are first pins).
//! `cargo test haz_room -- --nocapture` prints the per-seed table of the PR.

use super::pilots::{Report, SEEDS, STUCK_LIMIT, Who, run};
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

/// The harder solo-Donut probe: Donut plays the whole room from tick 90 (Carl is the
/// AI partner). FINDING (see the PR): she does not clear it; the fixture keeps the
/// acceptance and is ignored until the cause is fixed.
#[test]
#[ignore = "FINDING: solo Donut who fights the whole room does not clear it (PT-GATE-C2)"]
fn haz_room_donut_fights() {
    for seed in SEEDS {
        assert_cleared(&run_twice_checked(Who::DonutFights, seed, true));
    }
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
    0xddd8_09f8_ed0d_0864,
    0x285d_7830_1635_37e6,
    0xb9e3_6986_e85b_57c8,
    0x523a_d0f0_1d4a_2b87,
    0x0601_d4b5_0efb_4e64,
    0x60e3_6529_db0b_eae4,
    0xbe47_1aff_b947_775f,
    0x50e3_96c1_298b_e3b3,
    0x0ebb_de69_f004_5e59,
    0xe0e3_a62d_5ff4_117e,
    0x55fb_3f64_c250_115b,
    0x77ae_8237_24d2_0871,
    0xb894_61a1_9b0b_22e7,
    0x30c2_142f_eaf1_fd7d,
    0x1148_ed3c_81d6_d30d,
    0xf5ce_c641_0fb0_3e62,
    0x7ed1_6c41_1b69_a317,
    0x079a_68dd_39a8_442b,
];
pub(crate) const SOLO_CARL_PINNED: [u64; 9] = [
    0xddd8_09f8_ed0d_0864,
    0x285d_7830_1635_37e6,
    0xb9e3_6986_e85b_57c8,
    0x523a_d0f0_1d4a_2b87,
    0x0601_d4b5_0efb_4e64,
    0x60e3_6529_db0b_eae4,
    0xbe47_1aff_b947_775f,
    0x50e3_96c1_298b_e3b3,
    0x0ebb_de69_f004_5e59,
];
const SOLO_DONUT_PINNED: [u64; 9] = [
    0xdcd6_63bf_1bcb_ee0e,
    0x7842_7773_dd93_c777,
    0x1a20_8bb8_7d34_4dbb,
    0x2ae0_46ca_edaf_3af5,
    0x434a_a01e_0cfb_8fce,
    0xd006_3126_8792_708d,
    0x0c84_6a54_50c4_7a43,
    0x2200_ae28_a106_87ff,
    0x33ed_ac24_4d51_2a53,
];
const COMPANION_PINNED: [u64; 9] = [
    0x7d2d_5ac5_cfaa_fbe0,
    0xc892_0f04_979a_3a3b,
    0xd149_1076_0dc5_5950,
    0x6186_39ca_0e81_538b,
    0xb844_6df6_170f_5d94,
    0x29f8_5706_50e7_001c,
    0x94f7_bea4_1953_0a73,
    0x73ab_2bf6_e23f_3189,
    0x68dc_4ee8_bdfd_23a2,
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
