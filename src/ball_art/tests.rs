use super::*;
use crate::boss::{BossMode, RackState, ball};

fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn boss_in(mode: BossMode, tick: u16) -> Boss {
    let mut boss = Boss::new(ball(), 0.0);
    boss.mode = mode;
    boss.mode_tick = tick;
    boss
}

fn tags() -> serde_json::Value {
    let text = std::fs::read_to_string(
        root().join("assets/source/characters/swine/swine-01.pixelart.json"),
    )
    .unwrap();
    serde_json::from_str::<serde_json::Value>(&text).unwrap()["tags"].clone()
}

/// The frame table of the module is the approved readability set's (`swine-01.pixelart.json`).
#[test]
fn the_frame_constants_are_the_swine_01_tags() {
    let tags = tags();
    let frames = |tag: &str| -> Vec<(usize, Option<u32>)> {
        tags[tag]["frames"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                (
                    f[0].as_u64().unwrap() as usize,
                    f[1].as_u64().map(|t| t as u32),
                )
            })
            .collect()
    };
    assert_eq!(
        frames("idle"),
        frame::IDLE
            .iter()
            .map(|&(i, t)| (i, Some(t)))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        frames("roll"),
        frame::ROLL
            .iter()
            .map(|&i| (i, Some(frame::ROLL_TICKS)))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        frames("attack_tell"),
        vec![
            (frame::TELL_ROCK, Some(frame::TELL_ROCK_TICKS)),
            (frame::TELL_COIL, Some(28))
        ]
    );
    assert_eq!(
        frames("attack_charge"),
        vec![
            (frame::LAUNCH, Some(frame::LAUNCH_TICKS)),
            (frame::IMPACT, Some(frame::IMPACT_TICKS)),
            (frame::REBOUND, Some(24))
        ]
    );
    assert_eq!(frames("hurt"), vec![(frame::HURT, Some(12))]);
    assert_eq!(
        frames("down"),
        vec![
            (frame::CRACK, Some(frame::CRACK_TICKS)),
            (frame::SLUMP, Some(frame::SLUMP_TICKS)),
            (frame::HEAP, None)
        ]
    );
    assert_eq!(FRAMES, 18);
    assert_eq!((FRAME, PIVOT), ((176, 144), (88, 143)));
}

/// Every boss mode has a frame inside the atlas, and the map is the spec's (BOS-ART section 3).
#[test]
fn the_state_to_frame_map() {
    let f = |mode, tick| body_frame(&boss_in(mode, tick), 0, false, None);
    // Idle loop by the sim clock; the walk is the roll at half rate.
    assert_eq!(
        body_frame(&boss_in(BossMode::Neutral, 0), 0, false, None),
        0
    );
    assert_eq!(
        body_frame(&boss_in(BossMode::Neutral, 0), 16, false, None),
        1
    );
    assert_eq!(
        body_frame(&boss_in(BossMode::Neutral, 0), 28, false, None),
        2
    );
    assert_eq!(
        body_frame(&boss_in(BossMode::Neutral, 0), 44, false, None),
        0
    );
    assert_eq!(body_frame(&boss_in(BossMode::Neutral, 0), 0, true, None), 3);
    assert_eq!(body_frame(&boss_in(BossMode::Neutral, 0), 8, true, None), 4);
    assert_eq!(f(BossMode::Dormant, 0), 0);
    assert_eq!(f(BossMode::Turn, 5), 0);
    // The tell: the rock back, then the coil, for the roll and the shove alike.
    for mode in [
        BossMode::ChargeTell,
        BossMode::PoundTell,
        BossMode::SecondTell,
    ] {
        assert_eq!(f(mode, 0), 9);
        assert_eq!(f(mode, 7), 9);
        assert_eq!(f(mode, 8), 10);
        assert_eq!(f(mode, 35), 10);
    }
    // The roll: the launch smear, then the six native frames, 4 ticks each, looping.
    assert_eq!(f(BossMode::Charge, 0), 11);
    assert_eq!(f(BossMode::Charge, 2), 11);
    let rolled: Vec<usize> = (3..3 + 24).map(|t| f(BossMode::Charge, t)).collect();
    assert_eq!(rolled[0..4], [3, 3, 3, 3]);
    assert_eq!(rolled[4..8], [4, 4, 4, 4]);
    assert_eq!(rolled[20..24], [8, 8, 8, 8]);
    assert_eq!(f(BossMode::Charge, 27), 3);
    // The crash: the flat face for six ticks, then the rebound; the stun holds the rebound.
    assert_eq!(f(BossMode::Rebound, 0), 12);
    assert_eq!(f(BossMode::Rebound, 5), 12);
    assert_eq!(f(BossMode::Rebound, 6), 13);
    assert_eq!(f(BossMode::Stun, 0), 13);
    assert_eq!(f(BossMode::Stun, 65), 13);
    // The shove: smear, impact, recover.
    assert_eq!(f(BossMode::PoundStrike, 2), 11);
    assert_eq!(f(BossMode::PoundStrike, 3), 12);
    assert_eq!(f(BossMode::PoundRecover, 0), 13);
    // Hurt, roar, pinned.
    assert_eq!(f(BossMode::Flinch, 0), 14);
    assert_eq!(f(BossMode::Backstep, 3), 14);
    assert_eq!(f(BossMode::Roar, 10), 10);
    assert_eq!(f(BossMode::Pinned, 0), 16);
    assert_eq!(f(BossMode::Pinned, 8), 17);
    // Defeat: crack, slump, heap by the age of the defeat; a pinned death is the heap at once.
    let down = |age| body_frame(&boss_in(BossMode::Defeated, 0), 0, false, Some(age));
    assert_eq!(
        [down(0), down(7), down(8), down(19), down(20), down(500)],
        [15, 15, 16, 16, 17, 17]
    );
    let mut pinned_dead = boss_in(BossMode::Defeated, 0);
    pinned_dead.defeated_cause_rack = true;
    assert_eq!(body_frame(&pinned_dead, 0, false, Some(0)), 17);
    // No mode leaves the atlas.
    for mode in [
        BossMode::Dormant,
        BossMode::Neutral,
        BossMode::Turn,
        BossMode::PoundTell,
        BossMode::PoundStrike,
        BossMode::PoundRecover,
        BossMode::DoubleStep,
        BossMode::SecondTell,
        BossMode::SecondStrike,
        BossMode::SecondRecover,
        BossMode::ChargeTell,
        BossMode::Charge,
        BossMode::Rebound,
        BossMode::Stun,
        BossMode::Roar,
        BossMode::Flinch,
        BossMode::Backstep,
        BossMode::Pinned,
        BossMode::Defeated,
    ] {
        for tick in [0u16, 3, 9, 40, 200] {
            assert!(f(mode, tick) < FRAMES, "{mode:?} tick {tick}");
        }
    }
}

/// The roll never rotates or scales: the frames are the six atlas cels, distinct, and the
/// presenter draws them at 1:1 (the module has no angle or scale to give).
#[test]
fn the_roll_cycles_six_distinct_native_frames() {
    let mut seen: Vec<usize> = frame::ROLL.to_vec();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen, vec![3, 4, 5, 6, 7, 8]);
    assert_eq!(frame::ROLL.len() as u32 * frame::ROLL_TICKS, 24);
}

fn hash_of(boss: &Boss) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    boss.mix_into(&mut |v| {
        h ^= v;
        h = h.wrapping_mul(0x0100_0000_01b3);
    });
    h
}

fn rack_state(state: RackState) -> Boss {
    let mut boss = Boss::new(ball(), 0.0);
    boss.racks[0].state = state;
    boss
}

/// The weight's rect is the sim's rack footprint: 80 wide, centred on `footprint_centre`, on the
/// floor once dropped; hung 189 up; the chain reaches the ceiling girder.
#[test]
fn the_weight_sits_on_the_sim_footprint_in_each_state() {
    let boss = rack_state(RackState::Idle { hits: 0 });
    let foot = boss.footprint(0);
    let hung = weight_draws(&boss);
    let w = hung.iter().find(|d| d.piece == Piece::WeightHung).unwrap();
    assert_eq!((w.left, w.bottom, w.band), (foot.left, 189.0, Band::Back));
    assert_eq!(w.left + Piece::WeightHung.size().0, foot.right());
    assert_eq!(foot.width, 80.0);
    // The hung weight's top touches the girder's underside (224) to within the 1 px shackle.
    assert!((w.bottom + Piece::WeightHung.size().1 - CEILING_PX).abs() <= 1.0);
    // The tie reaches from the weight to the post's top (the strut at x 72, 196 tall).
    let tie: Vec<_> = hung.iter().filter(|d| d.piece == Piece::ChainH).collect();
    assert_eq!(tie.len(), TIE_TILES);
    assert_eq!(tie.last().unwrap().left, foot.left - 80.0);
    assert!(tie.iter().all(|d| d.bottom == 197.0));
    assert!(
        tie.last().unwrap().left <= 92.0 + 4.0,
        "the tie meets the post's right side"
    );

    let landed = weight_draws(&rack_state(RackState::Landed));
    assert_eq!(landed.len(), 1);
    assert_eq!(landed[0].piece, Piece::WeightLanded);
    assert_eq!((landed[0].left, landed[0].bottom), (foot.left, 0.0));
    assert_eq!(
        landed[0].bottom + 24.0,
        foot.height,
        "the slab is the footprint's height"
    );
}

#[test]
fn the_falling_weight_pays_its_chain_out_from_the_girder() {
    let mut last = f32::MAX;
    for ticks in [0u16, 1, 9, 18, 27, 35] {
        let boss = rack_state(RackState::Falling { ticks, player: 0 });
        let draws = weight_draws(&boss);
        let w = draws.iter().find(|d| d.piece == Piece::WeightHung).unwrap();
        assert!(w.bottom < last || ticks == 0, "it falls");
        last = w.bottom;
        let top = w.bottom + 36.0;
        let chain: Vec<_> = draws.iter().filter(|d| d.piece == Piece::ChainV).collect();
        // The chain is continuous from the girder (224) down to the shackle's top.
        let covered: f32 = chain.iter().map(|d| d.rows).sum();
        assert!(
            (covered - (CEILING_PX - top).max(0.0)).abs() < 0.01,
            "ticks {ticks}"
        );
        assert!(
            chain
                .iter()
                .all(|d| d.bottom + d.rows <= CEILING_PX + 0.01 && d.bottom >= top - 0.01)
        );
        assert!(chain.iter().all(|d| d.left == w.left + SHACKLE_X));
        assert!(chain.len() <= 14);
    }
}

#[test]
fn the_pinned_ball_has_the_weight_resting_on_its_heap() {
    let mut boss = rack_state(RackState::Landed);
    boss.mode = BossMode::Pinned;
    let pinned = weight_draws(&boss);
    assert_eq!(pinned[0].bottom, PINNED_REST_PX);
    // The heap is the hurtbox's 64 px: the slab (24) ends exactly on its top.
    assert_eq!(PINNED_REST_PX + 24.0, 64.0);
}

#[test]
fn a_room_without_a_left_rack_draws_no_weight() {
    let mut boss = Boss::new(ball(), 0.0);
    boss.racks[0].enabled = false;
    assert!(weight_draws(&boss).is_empty());
}

/// The greybox stand-ins of BOS-A are never drawn once the room is dressed.
#[test]
fn no_greybox_is_drawn_when_the_room_is_dressed() {
    for piece in [
        Greybox::Body,
        Greybox::Eye,
        Greybox::Rack,
        Greybox::Strut,
        Greybox::Ripple,
        Greybox::Arch,
        Greybox::Label,
    ] {
        assert!(greybox_shown(piece, false, false), "{piece:?} plain");
        assert!(!greybox_shown(piece, true, true), "{piece:?} dressed");
    }
    // Only a missing reused sheet keeps its rect (a dev fallback, never in a full export).
    assert!(greybox_shown(Greybox::Strut, true, false));
    assert!(greybox_shown(Greybox::Ripple, true, false));
    assert!(!greybox_shown(Greybox::Body, true, false));
}

/// The art is presentation only: stepping a run with and without reading the art gives one hash.
#[test]
fn reading_the_art_does_not_touch_the_boss() {
    let mut boss = rack_state(RackState::Idle { hits: 1 });
    let before = hash_of(&boss);
    let _ = body_frame(&boss, 9, true, None);
    let _ = weight_draws(&boss);
    assert_eq!(hash_of(&boss), before);
    boss.mode_tick += 1;
    assert_ne!(hash_of(&boss), before);
}
