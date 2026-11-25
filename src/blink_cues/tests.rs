use super::*;

fn cues() -> BlinkCues {
    BlinkCues {
        start: Some((10, 640, 100)),
        arrival: Some((16, 640, 752, 100)),
        ..BlinkCues::default()
    }
}

fn frames_at(c: &BlinkCues, x: i32, ticks: impl Iterator<Item = u64>) -> Vec<Option<usize>> {
    ticks
        .map(|t| c.view(t).puddles.iter().find(|p| p.x == x).map(|p| p.frame))
        .collect()
}

/// DONUT-MOVES-ART-01 spec 5: the origin puddle opens on the press (`ring_open_0`
/// for 3 ticks, `ring_open_1` for 3), and after the arrival it closes
/// (`ring_close_0..2`, 3 ticks each, ages 6 to 14), then it is gone.
#[test]
fn the_origin_puddle_opens_then_closes_after_she_has_left() {
    let c = cues();
    let f = frames_at(&c, 640, 9..=26);
    assert_eq!(f[0], None, "nothing before the press");
    assert_eq!(
        f[1..7],
        [Some(0), Some(0), Some(0), Some(1), Some(1), Some(1)],
        "ages 0 to 5"
    );
    assert_eq!(
        f[7..16],
        [
            Some(4),
            Some(4),
            Some(4),
            Some(5),
            Some(5),
            Some(5),
            Some(6),
            Some(6),
            Some(6)
        ],
        "ages 6 to 14"
    );
    assert_eq!(f[16..], [None, None], "gone after 9 ticks of closing");
}

/// The arrival opens the destination puddle: `ring_open_0..3`, 3 ticks each (12).
#[test]
fn the_destination_puddle_opens_over_twelve_ticks() {
    let c = cues();
    let f = frames_at(&c, 752, 15..=29);
    assert_eq!(f[0], None, "before the arrival");
    assert_eq!(
        f[1..13],
        [
            Some(0),
            Some(0),
            Some(0),
            Some(1),
            Some(1),
            Some(1),
            Some(2),
            Some(2),
            Some(2),
            Some(3),
            Some(3),
            Some(3)
        ]
    );
    assert_eq!(f[13..], [None, None]);
}

/// She rises out of the destination puddle: a0 on press ages 6-7, the key on 8-10,
/// a2 on 11-12 (the arrival is tick 16 here), then locomotion.
#[test]
fn she_rises_out_of_the_destination_puddle_over_seven_ticks() {
    let c = cues();
    let arrive: Vec<Option<usize>> = (15..=24).map(|t| c.view(t).arrive).collect();
    assert_eq!(
        arrive,
        [
            None,
            Some(8),
            Some(8),
            Some(9),
            Some(9),
            Some(9),
            Some(10),
            Some(10),
            None,
            None
        ]
    );
}

/// DP-Q34: the arrival draws a one-frame streak between the two columns, in either direction.
#[test]
fn the_arrival_draws_a_one_frame_tiled_streak() {
    let c = cues();
    let v = c.view(16);
    assert_eq!(
        v.streak,
        Some(Streak {
            from_x: 640,
            to_x: 752,
            foot_y: 100
        })
    );
    // The data says one frame: gone on the next tick, and absent before.
    assert_eq!(DonutProgTuning::active().dpq34_streak_frames, 1);
    assert_eq!(c.view(17).streak, None);
    assert_eq!(c.view(15).streak, None);
    // 112 px less the 4 px head: seven 16 px dashes from the origin, the head on the last 4 columns.
    let right = Streak {
        from_x: 640,
        to_x: 752,
        foot_y: 100,
    };
    let edges = right.dash_edges();
    assert_eq!(
        (edges[0], *edges.last().unwrap(), edges.len()),
        (640, 736, 7)
    );
    assert_eq!(right.head_edge(), 748);
    let left = Streak {
        from_x: 752,
        to_x: 640,
        foot_y: 100,
    };
    assert_eq!(left.dash_edges()[1], 736, "mirrored: tiles run toward -x");
    assert_eq!(left.head_edge(), 644);
    assert!(
        Streak {
            from_x: 5,
            to_x: 5,
            foot_y: 0
        }
        .dash_edges()
        .is_empty()
    );
    assert!(
        Streak {
            from_x: 0,
            to_x: 4,
            foot_y: 0
        }
        .dash_edges()
        .is_empty(),
        "a span that only fits the head has no dashes"
    );
    assert_eq!(right.y(), 108);
}

/// DMA-Q08: a fizzle is the bonk for 4 ticks, the huff for 4, and the drying ring (3, 3, 2) at the
/// origin, and only that (no streak, no arrival).
#[test]
fn a_fizzle_bonks_huffs_and_dries_up() {
    let c = BlinkCues {
        start: Some((14, 500, 90)),
        fizzle: Some((20, 500, 90)),
        ..BlinkCues::default()
    };
    let body: Vec<Option<usize>> = (19..=28).map(|t| c.view(t).fizzle).collect();
    assert_eq!(
        body,
        [
            None,
            Some(11),
            Some(11),
            Some(11),
            Some(11),
            Some(12),
            Some(12),
            Some(12),
            Some(12),
            None
        ]
    );
    let ring: Vec<Option<usize>> = (20..=28)
        .map(|t| c.view(t).puddles.first().map(|p| p.frame))
        .collect();
    assert_eq!(
        ring,
        [
            Some(9),
            Some(9),
            Some(9),
            Some(10),
            Some(10),
            Some(10),
            Some(11),
            Some(11),
            None
        ]
    );
    assert!((20..28).all(|t| {
        let v = c.view(t);
        v.streak.is_none() && v.arrive.is_none() && v.puddles.len() == 1
    }));
    // The origin ring opened during the windup and did not close: the fizzle ends the press.
    assert_eq!(c.view(17).puddles.first().map(|p| p.frame), Some(1));
}

/// A cancelled windup has no new frame of her own; the origin puddle closes from the cancel tick.
#[test]
fn a_cancel_closes_the_origin_puddle_and_draws_nothing_else() {
    let c = BlinkCues {
        start: Some((10, 640, 100)),
        cancel: Some(13),
        ..BlinkCues::default()
    };
    let f: Vec<Option<usize>> = (10..=23)
        .map(|t| c.view(t).puddles.first().map(|p| p.frame))
        .collect();
    assert_eq!(
        f,
        [
            Some(0),
            Some(0),
            Some(0),
            Some(4),
            Some(4),
            Some(4),
            Some(5),
            Some(5),
            Some(5),
            Some(6),
            Some(6),
            Some(6),
            None,
            None
        ]
    );
    assert!((10..24).all(|t| {
        let v = c.view(t);
        v.arrive.is_none() && v.fizzle.is_none() && v.streak.is_none()
    }));
}

/// A stale press (no resolution event ever arrives) does not leave a ring on screen for ever.
#[test]
fn an_unresolved_press_does_not_hold_the_ring() {
    let c = BlinkCues {
        start: Some((10, 640, 100)),
        ..BlinkCues::default()
    };
    assert!(c.view(16).puddles.len() == 1);
    assert!(c.view(17).puddles.is_empty());
}

/// A seek back or a new attempt (fewer events than seen) clears everything.
#[test]
fn reset_clears_the_cues() {
    let mut c = cues();
    c.reset();
    let v = c.view(16);
    assert!(v.puddles.is_empty() && v.streak.is_none() && v.arrive.is_none());
}
