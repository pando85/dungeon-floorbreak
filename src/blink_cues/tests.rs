use super::*;

fn cues() -> BlinkCues {
    BlinkCues {
        start: Some((10, 640, 100)),
        arrival: Some((16, 640, 752, 100)),
        ..BlinkCues::default()
    }
}

/// DP-Q34: the ripple at her feet is three rings, 3 ticks apart, each alive 9
/// ticks, growing 2 px per tick, brightest first.
#[test]
fn the_start_ripple_is_three_staggered_rings() {
    let c = BlinkCues {
        start: Some((10, 640, 100)),
        ..BlinkCues::default()
    };
    let rings_at = |tick| c.view(tick).rings;
    assert!(rings_at(9).is_empty(), "nothing before the press");
    let r10 = rings_at(10);
    assert_eq!(r10.len(), 1);
    assert_eq!(
        (r10[0].x, r10[0].foot_y, r10[0].rx, r10[0].ry),
        (640, 100, 4, 1)
    );
    assert_eq!(r10[0].tone, 3);
    // Tick 13: the first ring is 3 old, the second starts.
    let r13 = rings_at(13);
    assert_eq!(r13.iter().map(|r| r.rx).collect::<Vec<_>>(), [10, 4]);
    // Tick 16: three rings alive; tick 18: the first ring's last tick.
    assert_eq!(rings_at(16).len(), 3);
    assert_eq!(rings_at(18).len(), 3);
    assert_eq!(rings_at(18)[0].rx, 4 + 2 * 8);
    // Tick 19: the first ring is gone; the last ring ends on 10 + 6 + 8 = 24.
    assert_eq!(rings_at(19).len(), 2);
    assert_eq!(rings_at(24).len(), 1);
    assert!(rings_at(25).is_empty());
    // Every tone is in the Magic Missile palette.
    assert!((10..25).all(|t| rings_at(t).iter().all(|r| r.tone < PALETTE.len())));
}

/// DP-Q34: the arrival draws the same ripple at the destination and a one-frame
/// dotted streak between the two columns, in either direction.
#[test]
fn the_arrival_draws_a_ripple_at_the_destination_and_a_one_frame_streak() {
    let c = cues();
    let v = c.view(16);
    assert!(v.rings.iter().any(|r| r.x == 752), "the destination ripple");
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
    // The dots: from the start to the end, 6 px apart, both ways.
    let right = Streak {
        from_x: 640,
        to_x: 752,
        foot_y: 100,
    };
    let dots = right.dots();
    assert_eq!((dots[0], *dots.last().unwrap(), dots.len()), (640, 748, 19));
    let left = Streak {
        from_x: 752,
        to_x: 640,
        foot_y: 100,
    };
    assert_eq!(left.dots()[1], 746);
    assert!(
        Streak {
            from_x: 5,
            to_x: 5,
            foot_y: 0
        }
        .dots()
        .is_empty()
    );
    assert_eq!(right.y(), 108);
}

/// DP-Q34: a fizzle is one shrinking ring, 8 ticks, and only that.
#[test]
fn a_fizzle_is_one_shrinking_ring() {
    let c = BlinkCues {
        fizzle: Some((20, 500, 90)),
        ..BlinkCues::default()
    };
    let widths: Vec<i32> = (20..28).map(|t| c.view(t).rings[0].rx).collect();
    assert_eq!(widths, [20, 18, 16, 14, 12, 10, 8, 6]);
    assert!(c.view(28).rings.is_empty() && c.view(19).rings.is_empty());
    assert!((20..28).all(|t| c.view(t).rings.len() == 1 && c.view(t).streak.is_none()));
}

/// The outline is closed (each row joins the next), symmetric by construction,
/// and bounded for the presenter's pool.
#[test]
fn an_ellipse_outline_is_connected_and_bounded() {
    for rx in [4, 6, 10, 14, 20] {
        let ry = ring_ry(rx);
        let rows = ellipse_spans(rx, ry);
        assert_eq!(rows.len() as i32, 2 * ry + 1);
        // The widest row is the middle one and reaches rx.
        assert_eq!(rows[ry as usize].2, rx, "rx {rx}");
        for &(dy, from, to) in &rows {
            assert!(0 <= from && from <= to && to <= rx, "{rx} {dy}");
            if dy.abs() == ry {
                assert_eq!(from, 0, "the end rows run from the middle");
            }
        }
        // Each row reaches the next row's extent, so the outline has no gap.
        for w in rows.windows(2) {
            let (a, b) = (w[0], w[1]);
            if a.0.abs() < ry && b.0.abs() < ry || a.0 < 0 && b.0 <= 0 {
                assert!(
                    a.2.min(b.2) >= a.1.max(b.1) || (a.2 - b.2).abs() <= 1,
                    "{a:?} {b:?}"
                );
            }
        }
    }
    // At most 2 spans per row: the pool bound of the presenter.
    assert!(max_ring_spans() <= 30, "{}", max_ring_spans());
}

/// A seek back or a new attempt (fewer events than seen) clears everything.
#[test]
fn reset_clears_the_cues() {
    let mut c = cues();
    c.reset();
    assert!(c.view(16).rings.is_empty() && c.view(16).streak.is_none());
}
