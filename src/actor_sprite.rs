//! Actor sprite placement maths shared with the Character Playground rules:
//! feet anchor to the manifest pivot, the pivot is mirrored when the sprite is
//! flipped, and every position is an integer so no sprite sits on a half pixel.
//!
//! Foot rule (owner decision, deep floor): the lowest opaque row of the actor
//! sits on the walking line, `FOOT_SINK_PX = 0`.
//!
//! Known 1 px asymmetry (accepted): the flipped sprite's pivot column lands one
//! pixel to the left of the unflipped one. The Playground has the same
//! behaviour; half-pixel centres are deliberately not used to hide it.

pub use crate::env_kit::FOOT_SINK_PX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorSpec {
    pub name: &'static str,
    pub atlas: &'static str,
    pub width: i32,
    pub height: i32,
    /// Pivot pixel: x column and the lowest opaque row (y, from the top).
    pub pivot_x: i32,
    pub pivot_y: i32,
    pub opaque_height: i32,
}

pub const CARL: ActorSpec = ActorSpec {
    name: "CARL",
    atlas: "carl_book1_h52.png",
    width: 36,
    height: 56,
    pivot_x: 19,
    pivot_y: 55,
    opaque_height: 52,
};

pub const DONUT: ActorSpec = ActorSpec {
    name: "DONUT",
    atlas: "donut_scale_h40.png",
    width: 46,
    height: 44,
    pivot_x: 22,
    pivot_y: 43,
    opaque_height: 40,
};

/// Idle frames in the atlas row, and fixed ticks per frame (150 ms at 60 Hz,
/// the Playground's pace). Driven by the sim tick, never by wall-clock time.
pub const IDLE_FRAMES: u64 = 4;
pub const IDLE_TICKS_PER_FRAME: u64 = 9;

pub fn idle_frame(tick: u64, phase: u64) -> usize {
    ((tick / IDLE_TICKS_PER_FRAME + phase) % IDLE_FRAMES) as usize
}

/// Sprite centre in y-up world pixels for a centre-anchored sprite.
///
/// `foot_x` is the pivot x; `ground_y` is the y-up world height of the bottom
/// edge of the actor's feet (the walking line while grounded).
pub fn actor_center(spec: ActorSpec, foot_x: f32, ground_y: f32, flip_x: bool) -> (f32, f32) {
    let offset_x = spec.width as f32 / 2.0 - spec.pivot_x as f32;
    let center_x = if flip_x {
        foot_x - offset_x
    } else {
        foot_x + offset_x
    };
    // The pivot row is the lowest opaque row: its bottom edge is the foot line.
    let frame_bottom = ground_y - FOOT_SINK_PX as f32 - (spec.height - 1 - spec.pivot_y) as f32;
    (
        center_x.round(),
        (frame_bottom + spec.height as f32 / 2.0).round(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Character Playground's `actor_transform` x rule, copied verbatim.
    fn playground_center_x(spec: ActorSpec, x: f32, flip_x: bool) -> f32 {
        let offset_x = spec.width as f32 / 2.0 - spec.pivot_x as f32;
        let center_x = if flip_x { x - offset_x } else { x + offset_x };
        center_x.round()
    }

    #[test]
    fn x_matches_the_playground_for_both_facings() {
        for spec in [CARL, DONUT] {
            for flip in [false, true] {
                for x in [72.0, 300.4, 1784.0] {
                    assert_eq!(
                        actor_center(spec, x, 92.0, flip).0,
                        playground_center_x(spec, x, flip),
                        "{} flip={flip} x={x}",
                        spec.name
                    );
                }
            }
        }
    }

    #[test]
    fn pivot_foot_stays_put_up_to_the_documented_one_pixel_flip_offset() {
        for spec in [CARL, DONUT] {
            let foot = |flip: bool| {
                let cx = actor_center(spec, 500.0, 92.0, flip).0;
                // Left edge of the pivot pixel column in world x.
                let left = cx - spec.width as f32 / 2.0;
                if flip {
                    left + (spec.width - 1 - spec.pivot_x) as f32
                } else {
                    left + spec.pivot_x as f32
                }
            };
            assert_eq!(foot(false), 500.0);
            assert_eq!(foot(true), 499.0);
        }
    }

    #[test]
    fn lowest_opaque_row_sits_on_the_walking_line_with_zero_sink() {
        assert_eq!(FOOT_SINK_PX, 0);
        for spec in [CARL, DONUT] {
            for ground in [92.0, 76.0, 60.0] {
                let (_, cy) = actor_center(spec, 100.0, ground, false);
                let frame_top = cy + spec.height as f32 / 2.0;
                let pivot_row_bottom = frame_top - (spec.pivot_y + 1) as f32;
                assert_eq!(pivot_row_bottom, ground, "{} on {ground}", spec.name);
                assert_eq!(cy.fract(), 0.0);
            }
        }
    }

    #[test]
    fn approved_native_heights_and_pivots_are_pinned() {
        assert_eq!((CARL.opaque_height, DONUT.opaque_height), (52, 40));
        assert_eq!((CARL.pivot_x, CARL.pivot_y), (19, 55));
        assert_eq!((DONUT.pivot_x, DONUT.pivot_y), (22, 43));
        // Even canvases keep centre-anchored sprites on whole pixels.
        assert!(
            [CARL, DONUT]
                .iter()
                .all(|s| s.width % 2 == 0 && s.height % 2 == 0)
        );
    }

    #[test]
    fn idle_frame_depends_on_the_tick_only() {
        assert_eq!(idle_frame(0, 0), 0);
        assert_eq!(idle_frame(8, 0), 0);
        assert_eq!(idle_frame(9, 0), 1);
        assert_eq!(idle_frame(36, 0), 0);
        assert_eq!(idle_frame(0, 2), 2);
    }
}
