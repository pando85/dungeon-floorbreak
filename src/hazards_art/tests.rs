use super::*;
use crate::combat::CombatEvent;
use crate::hazards_fixtures::{Layout, Rig, SlimeAt, walk};
use crate::input::RawInput;
use crate::party::HeroId;
use crate::rot_sticker::StickerState;

/// `haz_presentation_timing` (HAZ-Q37): a warning provoked at `t` shows frame 7
/// on `t..t+5`, frame 8 on `t+6..t+22`, frame 9 on `t+23` and frame 10 on `t+24`;
/// the ring is drawn on `t..t+23`.
#[test]
fn haz_presentation_timing() {
    for k in 0..=24u64 {
        let remaining = Some(24 - k);
        let want = match k {
            0..=5 => 7,
            6..=22 => 8,
            23 => 9,
            _ => 10,
        };
        assert_eq!(
            sticker_frame(StickerState::Warning, remaining, 100 + k, 0),
            Some(want),
            "tick t+{k}"
        );
        assert_eq!(ring_drawn(remaining), k <= 23, "ring at t+{k}");
    }
    // A chain at `c` pulls the detonation to c+8: frame 8 on c..c+6, 9 on c+7, 10
    // on c+8.
    for k in 0..=8u64 {
        let want = match k {
            0..=6 => 8,
            7 => 9,
            _ => 10,
        };
        assert_eq!(
            sticker_frame(StickerState::Warning, Some(8 - k), 0, 0),
            Some(want),
            "chain c+{k}"
        );
    }
    // Spent: the body is gone.
    assert_eq!(sticker_frame(StickerState::Spent, None, 0, 0), None);
    // The loops: latched cycles frames 0 to 2, scuttling 3 to 6.
    let seen: std::collections::BTreeSet<u8> = (0..64)
        .filter_map(|t| sticker_frame(StickerState::Latched, None, t, 0))
        .collect();
    assert_eq!(seen, ROT_IDLE.into_iter().collect());
    let seen: std::collections::BTreeSet<u8> = (0..64)
        .filter_map(|t| sticker_frame(StickerState::Scuttling, None, t, 0))
        .collect();
    assert_eq!(seen, ROT_WALK.into_iter().collect());
}

/// The slime's frame follows its phase (section 3): the half-rise 6 ticks, the
/// reared pose 24, the launch 2, the impact 6, the recover 20.
#[test]
fn the_slime_frames_follow_the_sheets_timeline() {
    let look = SlimeLook::default();
    let expect = [
        (SlimePhase::TellRise, 7),
        (SlimePhase::TellHold, 8),
        (SlimePhase::Launch, 9),
        (SlimePhase::Impact, 10),
        (SlimePhase::Recover, 11),
    ];
    for (phase, frame) in expect {
        assert_eq!(slime_frame(phase, false, false, look, 0), frame);
        // An attack pose is never replaced by the hurt frame (a gel hit does not
        // interrupt it).
        assert_eq!(slime_frame(phase, true, false, look, 0), frame);
    }
    assert_eq!(
        slime_frame(SlimePhase::Idle, true, false, look, 0),
        SLIME_HURT
    );
    assert_eq!(
        slime_frame(SlimePhase::Cooldown, true, false, look, 0),
        SLIME_HURT
    );
    let moving = SlimeLook {
        moving: true,
        ..look
    };
    let creep: std::collections::BTreeSet<u8> = (0..64)
        .map(|t| slime_frame(SlimePhase::Idle, false, false, moving, t))
        .collect();
    assert_eq!(creep, SLIME_CREEP.into_iter().collect());
    let idle: std::collections::BTreeSet<u8> = (0..64)
        .map(|t| slime_frame(SlimePhase::Cooldown, false, false, look, t))
        .collect();
    assert_eq!(idle, SLIME_IDLE.into_iter().collect());
    // Defeated: the three down frames, the last one held.
    let down = |age| {
        slime_frame(
            SlimePhase::Idle,
            false,
            true,
            SlimeLook {
                moving: false,
                defeated_age: age,
            },
            0,
        )
    };
    assert_eq!(
        [
            down(None),
            down(Some(5)),
            down(Some(6)),
            down(Some(12)),
            down(Some(900))
        ],
        [13, 13, 14, 15, 15]
    );
}

/// The ROT-01 frame sits so the pivot is the dome centre, mirrored for the wall
/// at the right and turned a quarter clockwise for the ceiling.
#[test]
fn a_sticker_sits_on_its_dome_in_every_orientation() {
    let dome = (100.0, 50.0);
    assert_eq!(
        sticker_placement(Orientation::WallLeft, dome),
        ((110.0, 50.0), false, 0)
    );
    assert_eq!(
        sticker_placement(Orientation::WallRight, dome),
        ((90.0, 50.0), true, 0)
    );
    assert_eq!(
        sticker_placement(Orientation::Ceiling, dome),
        ((100.0, 40.0), false, 1)
    );
}

fn provoked_rig() -> (Rig, u64) {
    let mut rig = Layout {
        stickers: vec![Layout::under_slab(600, 600, 600)],
        carl_x: 590,
        ..Layout::default()
    }
    .sim(None);
    let mut at = None;
    for _ in 0..10 {
        rig.carl(RawInput::NONE);
        at = rig.provokes().iter().find_map(|e| match e {
            CombatEvent::RotProvoked { tick, .. } => Some(*tick),
            _ => None,
        });
        if at.is_some() {
            break;
        }
    }
    (rig, at.expect("provoked"))
}

/// The same table read from a real provoked sticker, tick by tick.
#[test]
fn the_real_warning_draws_the_timing_table_and_the_ring() {
    let (mut rig, t) = provoked_rig();
    let layout_h = rig.scene().world.layout_height();
    loop {
        let tick = rig.tick();
        let k = tick.saturating_sub(t);
        let env = rig.combat().env().unwrap();
        let draw = stickers(rig.combat(), tick);
        let ring = pieces(env, layout_h, tick)
            .iter()
            .filter(|p| p.tone == Tone::Ring)
            .count();
        if k <= 24 {
            let want = match k {
                0..=5 => 7,
                6..=22 => 8,
                23 => 9,
                _ => 10,
            };
            // The env step of the provoke tick has run; the state is that of
            // tick `t` when the rig's tick is `t`.
            assert_eq!(draw.len(), 1, "t+{k}");
            assert_eq!(draw[0].frame, want, "t+{k}");
            assert_eq!(ring, if k <= 23 { RING_DOTS } else { 0 }, "ring at t+{k}");
        } else {
            assert!(draw.is_empty(), "the body is gone after the blast");
            break;
        }
        rig.carl(RawInput::NONE);
    }
    // The ring sits at the 48 px blast radius around the dome centre.
    let (mut rig, t) = provoked_rig();
    rig.carl(RawInput::NONE);
    let env = rig.combat().env().unwrap();
    let tuning = crate::rot_sticker::active();
    let dome = env.stickers().next().unwrap().1.dome(tuning);
    let layout_h = rig.scene().world.layout_height();
    let ring: Vec<Piece> = pieces(env, layout_h, t + 1)
        .into_iter()
        .filter(|p| p.tone == Tone::Ring)
        .collect();
    assert_eq!(ring.len(), RING_DOTS);
    for p in ring {
        let (cx, cy) = (p.x as f32 + 1.5, p.y as f32 + 1.5);
        let d = ((cx - dome.0).powi(2) + (cy - dome.1).powi(2)).sqrt();
        assert!((d - 48.0).abs() < 2.1, "{d}");
    }
    // After the blast the spent sticker leaves a drip on the slab.
    for _ in 0..40 {
        rig.carl(RawInput::NONE);
    }
    let env = rig.combat().env().unwrap();
    let drips: Vec<Piece> = pieces(env, layout_h, rig.tick())
        .into_iter()
        .filter(|p| p.tone == Tone::Drip)
        .collect();
    assert_eq!(drips.len(), 1);
}

/// The acid strip is drawn at the pool's **current** bounds: a draining pool
/// shrinks with its volume, and a removed one is gone.
#[test]
fn the_acid_strip_follows_the_live_pool() {
    let mut rig = Layout {
        acid: vec![(700, 764, Some(0))],
        slimes: vec![SlimeAt {
            x: 400,
            facing: 1,
            home: 732,
        }],
        carl_x: 112,
        ..Layout::default()
    }
    .sim(None);
    let layout_h = rig.scene().world.layout_height();
    let strip = |rig: &Rig| {
        pieces(rig.combat().env().unwrap(), layout_h, rig.tick())
            .into_iter()
            .filter(|p| p.tone == Tone::Acid)
            .collect::<Vec<_>>()
    };
    let full = strip(&rig);
    assert_eq!(full.len(), 1);
    assert_eq!((full[0].x, full[0].w, full[0].h), (700, 64, 6));
    let bounds = |rig: &Rig| rig.combat().env().unwrap().acid().get(1).unwrap().bounds();
    // Defeat the slime through the real path: the pool drains over 60 ticks. One tick
    // first: a slime that is down before the first env step was never alive (the
    // kill switch, HAZA2-AD6) and its pool is removed at once, not drained.
    rig.idle(1);
    rig.kill_slime(0);
    for _ in 0..30 {
        rig.carl(RawInput::NONE);
    }
    let half = strip(&rig);
    let (x0, x1) = bounds(&rig).unwrap();
    assert_eq!(half.len(), 1);
    assert_eq!((half[0].x, half[0].x + half[0].w), (x0, x1));
    assert!(half[0].w < 64);
    for _ in 0..40 {
        rig.carl(RawInput::NONE);
    }
    assert!(strip(&rig).is_empty(), "a drained pool draws nothing");
}

/// Rendering never changes authoritative state: a run that calls every
/// presenter function on every tick has, at every tick, the state hash of a run
/// that never does (the presenter holds only shared references).
#[test]
fn drawing_never_changes_the_state_hash() {
    let build = || {
        Layout {
            acid: vec![(700, 764, Some(0))],
            slimes: vec![SlimeAt {
                x: 400,
                facing: 1,
                home: 400,
            }],
            stickers: vec![Layout::under_slab(600, 600, 600)],
            carl_x: 380,
            ..Layout::default()
        }
        .sim(Some(true))
    };
    let (mut plain, mut drawn) = (build(), build());
    let layout_h = drawn.scene().world.layout_height();
    for k in 0..300u64 {
        let input = if k < 120 { walk(1.0) } else { RawInput::NONE };
        plain.step(input, RawInput::NONE);
        drawn.step(input, RawInput::NONE);
        let tick = drawn.tick();
        let mut moving = false;
        let _ = slimes(drawn.combat(), tick, |_, _, _| {
            moving = !moving;
            SlimeLook {
                moving,
                defeated_age: Some(k),
            }
        });
        let _ = stickers(drawn.combat(), tick);
        if let Some(env) = drawn.combat().env() {
            let _ = pieces(env, layout_h, tick);
        }
        assert_eq!(plain.hash(), drawn.hash(), "tick {k}");
        assert_eq!(plain.events().len(), drawn.events().len(), "tick {k}");
    }
    assert!(!plain.hurts(HeroId::Carl).is_empty() || !plain.provokes().is_empty());
}

/// The sticker's charge slot never draws a dynamite stick: the presenter hides
/// the stick of every charge the env owns, the stickers' included.
#[test]
fn a_sticker_charge_is_owned_by_the_env_and_a_free_charge_is_not() {
    let rig = Layout {
        stickers: vec![Layout::under_slab(600, 600, 600)],
        carl_x: 112,
        ..Layout::default()
    }
    .sim(None);
    let combat = rig.combat();
    let (_, _, charge) = combat.env().unwrap().stickers().next().unwrap();
    assert!(combat.env_owns_charge(charge), "the sticker's own charge");
    assert!(
        !combat.env_owns_charge(combat.dynamite_slot()),
        "the dynamite slot is the index after every authored charge"
    );
}
