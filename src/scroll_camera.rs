//! Deterministic horizontal scroll camera, stepped in the fixed tick.
//!
//! All state and output are integers in virtual pixels, so the rendered camera
//! can never sit on a half pixel. The rig follows a lead point ahead of the
//! hero, only moves when that point leaves a deadzone, caps its speed and
//! clamps to the layout. Rendering reads it; it never feeds simulation.

/// Look-ahead in the facing direction, virtual px.
pub const LEAD_PX: i32 = 24;
/// Half width of the deadzone around the camera centre, virtual px.
pub const DEADZONE_PX: i32 = 40;
/// Largest camera movement per fixed tick, virtual px.
pub const MAX_SPEED_PX: i32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CameraConfig {
    pub view_w: i32,
    pub view_h: i32,
    pub layout_w: i32,
    pub layout_h: i32,
    /// Fixed top edge of the view in layout pixels (from the layout `views`).
    pub view_top: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollCamera {
    cfg: CameraConfig,
    centre_x: i32,
}

impl ScrollCamera {
    /// Start already framed on the hero (no ease-in at spawn).
    pub fn new(cfg: CameraConfig, hero_x: f32, facing: i8) -> Self {
        let mut camera = Self { cfg, centre_x: 0 };
        camera.centre_x = camera.clamp_centre(Self::lead_point(hero_x, facing));
        camera
    }

    fn lead_point(hero_x: f32, facing: i8) -> i32 {
        // Follow the pixel the hero is drawn at, so hero and camera move in
        // whole-pixel lockstep and the hero does not shimmer on screen.
        hero_x.round() as i32 + LEAD_PX * i32::from(facing.signum())
    }

    fn clamp_centre(&self, centre: i32) -> i32 {
        let half = self.cfg.view_w / 2;
        centre.clamp(half, self.cfg.layout_w - half)
    }

    pub fn step(&mut self, hero_x: f32, facing: i8) {
        self.step_capped(hero_x, facing, MAX_SPEED_PX);
    }

    /// [`Self::step`] with an explicit speed cap (the party layer raises it
    /// briefly after a solo swap). Returns how many pixels the camera still
    /// wants to move after this step, so the caller can drop the raised cap
    /// once the pan has caught up. `facing` 0 means no look-ahead (the
    /// midpoint of two heroes).
    pub fn step_capped(&mut self, hero_x: f32, facing: i8, max_speed: i32) -> i32 {
        self.step_capped_deadzone(hero_x, facing, max_speed, DEADZONE_PX)
    }

    /// [`Self::step_capped`] with an explicit deadzone half width. Two players
    /// follow their midpoint with `0` so the leader at the separation wall
    /// keeps the full frame margin (Q7); with the 40 px deadzone the centre
    /// could lag the midpoint by that much and eat into the margin.
    pub fn step_capped_deadzone(
        &mut self,
        hero_x: f32,
        facing: i8,
        max_speed: i32,
        deadzone: i32,
    ) -> i32 {
        let desired = Self::lead_point(hero_x, facing);
        let offset = desired - self.centre_x;
        let target = if offset > deadzone {
            desired - deadzone
        } else if offset < -deadzone {
            desired + deadzone
        } else {
            self.centre_x
        };
        let delta = (target - self.centre_x).clamp(-max_speed, max_speed);
        self.centre_x = self.clamp_centre(self.centre_x + delta);
        (self.clamp_centre(target) - self.centre_x).abs()
    }

    /// Where the view's centre column is, in layout pixels.
    pub fn centre_x(&self) -> i32 {
        self.centre_x
    }

    /// Left edge of the view in layout pixels, always inside the layout.
    pub fn left(&self) -> i32 {
        (self.centre_x - self.cfg.view_w / 2).clamp(0, self.cfg.layout_w - self.cfg.view_w)
    }

    /// Top edge of the view in layout pixels (fixed).
    pub fn top(&self) -> i32 {
        self.cfg.view_top
    }

    pub fn config(&self) -> CameraConfig {
        self.cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(view_h: i32, view_top: i32) -> CameraConfig {
        CameraConfig {
            view_w: 640,
            view_h,
            layout_w: 2048,
            layout_h: 400,
            view_top,
        }
    }

    const VIEWS: [(i32, i32); 2] = [(360, 20), (400, 0)];

    #[test]
    fn clamps_at_both_ends_for_both_views() {
        for (h, top) in VIEWS {
            let cfg = config(h, top);
            // Left end: hero hugging the end wall, facing left.
            let mut camera = ScrollCamera::new(cfg, 20.0, -1);
            for _ in 0..200 {
                camera.step(20.0, -1);
            }
            assert_eq!(camera.left(), 0, "view {h}");
            // Right end.
            let mut camera = ScrollCamera::new(cfg, 2040.0, 1);
            for _ in 0..200 {
                camera.step(2040.0, 1);
            }
            assert_eq!(camera.left(), 2048 - 640, "view {h}");
            assert!(camera.top() >= 0 && camera.top() + h <= cfg.layout_h);
            // Walking the whole corridor never leaves the layout.
            let mut camera = ScrollCamera::new(cfg, 72.0, 1);
            let mut x = 72.0_f32;
            for _ in 0..1000 {
                x = (x + 3.08).min(2040.0);
                camera.step(x, 1);
                assert!((0..=2048 - 640).contains(&camera.left()));
            }
        }
    }

    #[test]
    fn speed_is_capped_and_output_is_integer_steps() {
        let mut camera = ScrollCamera::new(config(360, 20), 100.0, 1);
        let mut previous = camera.left();
        // A teleport of the hero (restart, far spawn) must not snap the view.
        for _ in 0..60 {
            camera.step(1500.0, 1);
            let now = camera.left();
            assert!((now - previous).abs() <= MAX_SPEED_PX);
            previous = now;
        }
        assert!(previous > 200);
    }

    #[test]
    fn a_raised_cap_pans_faster_and_reports_the_distance_left() {
        let mut slow = ScrollCamera::new(config(360, 20), 700.0, 1);
        let mut fast = slow;
        let (mut remaining_slow, mut remaining_fast) = (0, 0);
        for _ in 0..10 {
            remaining_slow = slow.step_capped(1400.0, 1, MAX_SPEED_PX);
            remaining_fast = fast.step_capped(1400.0, 1, 8);
        }
        assert_eq!(
            slow.left() - ScrollCamera::new(config(360, 20), 700.0, 1).left(),
            40
        );
        assert_eq!(
            fast.left() - ScrollCamera::new(config(360, 20), 700.0, 1).left(),
            80
        );
        assert_eq!(remaining_slow - remaining_fast, 40);
        // `step` is `step_capped` at the normal cap, and a still target leaves 0.
        let mut a = ScrollCamera::new(config(360, 20), 700.0, 1);
        let mut b = a;
        a.step(900.0, 1);
        b.step_capped(900.0, 1, MAX_SPEED_PX);
        assert_eq!(a, b);
        assert_eq!(a.step_capped(a.centre_x() as f32 - 24.0, 1, 4), 0);
        // No look-ahead for facing 0 (the two-player midpoint).
        assert_eq!(
            ScrollCamera::new(config(360, 20), 900.0, 0).left(),
            900 - 320
        );
    }

    #[test]
    fn deadzone_keeps_the_view_still_for_small_moves() {
        let mut camera = ScrollCamera::new(config(360, 20), 900.0, 1);
        let left = camera.left();
        for dx in 0..30 {
            camera.step(900.0 + dx as f32, 1);
        }
        assert_eq!(camera.left(), left, "hero inside the deadzone");
    }

    #[test]
    fn lead_shows_more_space_in_the_facing_direction() {
        let mut right = ScrollCamera::new(config(360, 20), 900.0, 1);
        let mut left = ScrollCamera::new(config(360, 20), 900.0, -1);
        assert_eq!(right.left() - left.left(), 2 * LEAD_PX);
        // Hero walks steadily right: the hero settles right of the screen centre.
        let mut x = 900.0;
        for _ in 0..200 {
            x += 3.0;
            right.step(x, 1);
        }
        let screen_x = x as i32 - right.left();
        assert_eq!(screen_x, 320 + DEADZONE_PX - LEAD_PX);
        // Walking left settles the hero left of centre.
        for _ in 0..200 {
            x -= 3.0;
            left.step(x, -1);
        }
        let screen_x = x as i32 - left.left();
        assert_eq!(screen_x, 320 - DEADZONE_PX + LEAD_PX);
    }

    #[test]
    fn hero_keeps_a_constant_screen_position_while_the_camera_follows() {
        let mut camera = ScrollCamera::new(config(360, 20), 700.0, 1);
        let mut x = 700.0_f32;
        let mut screen = Vec::new();
        for _ in 0..300 {
            x += 3.083_333;
            camera.step(x, 1);
            screen.push(x.round() as i32 - camera.left());
        }
        // After the camera engages, the drawn hero pixel does not shimmer.
        let tail = &screen[100..];
        assert!(tail.iter().all(|&s| s == tail[0]), "{tail:?}");
    }
}
