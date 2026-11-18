//! Camera operator of the Floor 1 Camera Scenarios Lab. Bevy-free and
//! presentation only: nothing here is read by the simulation, collision, damage
//! or any replay hash.
//!
//! The camera changes the **world coverage** of the view, never the sprite
//! source sizes (ADR-0007: 640x360 / 640x400 virtual pixels, Carl 52 px, Donut
//! 40 px). A coverage of `c` shows `640c x 360c` (16:9) or `640c x 400c` (16:10)
//! world pixels. The renderer draws that many texels 1:1 into a coverage sized
//! target, so the view is always a whole number of world pixels:
//!
//! * coverage widths are multiples of [`STEP_PX`] (32), which keeps the width,
//!   the height and half of each whole numbers at both aspects. The four lab
//!   candidates 1.0 / 1.25 / 1.5 / 1.75 are exactly 640 / 800 / 960 / 1120;
//! * the automatic mode therefore moves in whole 32 px steps (2.5 percent
//!   coverage each), smoothed over time, with hysteresis;
//! * the view origin is always a whole world pixel and is clamped to the
//!   authored camera bounds, so the camera can never show void.

/// Virtual width of the approved render baseline (ADR-0007).
pub const BASE_W: i32 = 640;
/// Smallest and largest coverage width the lab uses (1.0x and 1.75x).
pub const MIN_W: i32 = 640;
pub const MAX_W: i32 = 1120;
/// Whole-pixel quantum of the coverage width.
pub const STEP_PX: i32 = 32;
/// The four coverage candidates of the lab.
pub const COVERAGE_CANDIDATES: [f32; 4] = [1.0, 1.25, 1.5, 1.75];

/// Horizontal / vertical padding around an actor's body that must stay on
/// screen: the actor's safety region (world px).
pub const SAFETY_PAD_X: f32 = 96.0;
pub const SAFETY_PAD_Y: f32 = 56.0;
/// A readable tell is framed once it has this many ticks (or fewer) left.
pub const TELL_LOOKAHEAD_TICKS: u16 = 54;
/// Largest horizontal bias towards a threat, as a share of the view width.
pub const MAX_BIAS_SHARE: f32 = 0.16;
/// Ticks the framing target must stay at least [`ZOOM_IN_DEADBAND_PX`] smaller
/// than the current coverage before the camera zooms back in.
pub const ZOOM_IN_DELAY_TICKS: u32 = 96;
/// Minimum ticks between two zoom direction changes (dwell).
pub const MIN_DWELL_TICKS: u32 = 72;
pub const ZOOM_IN_DEADBAND_PX: i32 = 64;
/// The camera starts zooming out when the framing need is within this many
/// pixels of the current width (an early trigger: the safety regions already
/// carry padding, this keeps the zoom ahead of a walking actor).
pub const ZOOM_OUT_SLACK_PX: i32 = 24;
/// Extra width (one step) kept when zooming back in, so the new coverage is not
/// immediately too small again.
pub const ZOOM_IN_MARGIN_PX: i32 = STEP_PX;
/// Pan speed limits, world px per tick (60 Hz). The emergency speed applies only
/// while an actor is outside its safety region.
pub const MAX_PAN: f32 = 5.0;
pub const MAX_PAN_IMPACT: f32 = 1.5;
pub const MAX_PAN_EMERGENCY: f32 = 12.0;
/// Fraction of the remaining pan distance closed per tick.
pub const PAN_RATE: f32 = 0.11;
/// Zoom speed, world px of width per tick: outwards and inwards.
pub const ZOOM_OUT_RATE: f32 = 0.12;
pub const ZOOM_IN_RATE: f32 = 0.035;
pub const ZOOM_OUT_MIN_SPEED: f32 = 2.5;
pub const ZOOM_IN_MIN_SPEED: f32 = 0.6;
/// The framed point sits this share of the view height above the actors'
/// vertical centre, so the floor is in the lower part and the architecture
/// above stays visible.
pub const HEADROOM_SHARE: f32 = 0.06;

/// Aspect of the view: 16:9 (640x360 base) or the Steam Deck 16:10 (640x400).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aspect {
    Wide16x9,
    Deck16x10,
}

impl Aspect {
    pub const fn base_h(self) -> i32 {
        match self {
            Self::Wide16x9 => 360,
            Self::Deck16x10 => 400,
        }
    }

    /// View height for a coverage width: `w x base_h / 640`, an integer for every
    /// multiple of [`STEP_PX`].
    pub const fn height_for(self, w: i32) -> i32 {
        w * self.base_h() / BASE_W
    }

    /// The aspect of an output window: 16:10 or wider-than-16:10 chooses the
    /// Deck view, everything else 16:9.
    pub fn from_output(w: u32, h: u32) -> Self {
        if (w as f32 / h as f32) < 1.65 {
            Self::Deck16x10
        } else {
            Self::Wide16x9
        }
    }
}

/// A view in world pixels (top-left origin, y down, whole pixels).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct View {
    pub left: i32,
    pub top: i32,
    pub w: i32,
    pub h: i32,
}

impl View {
    pub fn right(self) -> i32 {
        self.left + self.w
    }
    pub fn bottom(self) -> i32 {
        self.top + self.h
    }
    pub fn contains_point(self, x: f32, y: f32) -> bool {
        x >= self.left as f32
            && x < self.right() as f32
            && y >= self.top as f32
            && y < self.bottom() as f32
    }
}

/// Coverage width (a multiple of [`STEP_PX`]) nearest to `coverage` x 640.
pub fn coverage_width(coverage: f32) -> i32 {
    let w = (BASE_W as f32 * coverage / STEP_PX as f32).round() as i32 * STEP_PX;
    w.clamp(MIN_W, MAX_W)
}

/// Coverage (1.0 = 640 wide) of a width.
pub fn coverage_of(w: i32) -> f32 {
    w as f32 / BASE_W as f32
}

/// Whole-pixel dimensions of a view for a coverage and aspect.
pub fn coverage_dims(aspect: Aspect, coverage: f32) -> (i32, i32) {
    let w = coverage_width(coverage);
    (w, aspect.height_for(w))
}

/// Output pixels per world pixel for a coverage width shown on `out_w` pixels.
pub fn output_scale(out_w: u32, cov_w: i32) -> f32 {
    out_w as f32 / cov_w as f32
}

/// True when every world pixel maps to a whole number of output pixels (no
/// uneven texel widths).
pub fn is_integer_scale(out_w: u32, cov_w: i32) -> bool {
    out_w % cov_w as u32 == 0
}

/// Widths in output pixels that the texels of a coverage take: `(narrow, wide)`.
/// Equal for an integer scale; otherwise some texels are one pixel wider.
pub fn texel_widths(out_w: u32, cov_w: i32) -> (u32, u32) {
    let narrow = out_w / cov_w as u32;
    if is_integer_scale(out_w, cov_w) {
        (narrow, narrow)
    } else {
        (narrow, narrow + 1)
    }
}

/// On-screen height in output pixels of a sprite of `source_px` world pixels.
pub fn effective_px(source_px: f32, out_w: u32, cov_w: i32) -> f32 {
    source_px * output_scale(out_w, cov_w)
}

/// What the camera frames: one actor (or the companion), in y-down layout
/// pixels. The body box is `x +- half_w`, `y +- half_h` around the centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Subject {
    pub x: f32,
    pub y: f32,
    pub half_w: f32,
    pub half_h: f32,
}

/// An imminent charge or ranged attack worth framing. `readable` is false
/// while the enemy shows no tell yet (the camera must not react to hidden
/// information).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Threat {
    pub x: f32,
    pub y: f32,
    pub half_w: f32,
    pub half_h: f32,
    /// Direction of travel: +1 right, -1 left.
    pub dir: i8,
    /// Ticks until the threat commits (0 = already moving).
    pub tell_left: u16,
    pub readable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraInput<'a> {
    pub subjects: &'a [Subject],
    pub threat: Option<Threat>,
    /// Hit-stop or a fresh impact: no zoom-in, no zoom change except the safety
    /// zoom-out, and a slower pan.
    pub impact_hold: bool,
}

/// Fixed coverage or automatic shared framing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraMode {
    Fixed(f32),
    Auto,
}

/// The authored camera bounds: the view is clamped inside `0..w x 0..h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone)]
pub struct CameraOperator {
    mode: CameraMode,
    aspect: Aspect,
    bounds: Bounds,
    cx: f32,
    cy: f32,
    zoom_w: f32,
    cov_w: i32,
    zoom_target: i32,
    want_in_ticks: u32,
    dwell: u32,
    view: View,
    /// Number of whole-step coverage changes so far (a pumping measure).
    pub zoom_changes: u32,
}

fn ceil_step(w: f32) -> i32 {
    ((w / STEP_PX as f32).ceil() as i32 * STEP_PX).clamp(MIN_W, MAX_W)
}

/// The tightest coverage width that fits every actor's safety region and a
/// readable threat. Not clamped to the bounds of a room.
pub fn required_width(aspect: Aspect, input: &CameraInput) -> f32 {
    required_width_in(aspect, input, None)
}

pub fn required_width_in(aspect: Aspect, input: &CameraInput, bounds: Option<Bounds>) -> f32 {
    let (x0, x1, y0, y1) = framing_box_in(input, bounds);
    let w = x1 - x0;
    let h = y1 - y0;
    w.max(h * BASE_W as f32 / aspect.base_h() as f32)
}

/// Bounding box (x0, x1, y0, y1) of the safety regions and the readable threat.
fn framing_box(input: &CameraInput) -> (f32, f32, f32, f32) {
    framing_box_in(input, None)
}

/// [`framing_box`] with the safety regions clipped to the camera bounds: an
/// actor standing at a wall cannot be padded beyond the wall.
fn framing_box_in(input: &CameraInput, bounds: Option<Bounds>) -> (f32, f32, f32, f32) {
    let mut x0 = f32::MAX;
    let mut x1 = f32::MIN;
    let mut y0 = f32::MAX;
    let mut y1 = f32::MIN;
    let (bw, bh) = bounds.map_or((f32::MAX, f32::MAX), |b| (b.w as f32, b.h as f32));
    for s in input.subjects {
        x0 = x0.min((s.x - s.half_w - SAFETY_PAD_X).max(0.0));
        x1 = x1.max((s.x + s.half_w + SAFETY_PAD_X).min(bw));
        y0 = y0.min((s.y - s.half_h - SAFETY_PAD_Y).max(0.0));
        y1 = y1.max((s.y + s.half_h + SAFETY_PAD_Y).min(bh));
    }
    if input.subjects.is_empty() {
        return (0.0, BASE_W as f32, 0.0, 360.0);
    }
    if let Some(t) = input.threat
        && t.readable
        && t.tell_left <= TELL_LOOKAHEAD_TICKS
    {
        // The charge itself plus a lane ahead of it, so the impact point and the
        // path stay visible.
        let ahead = 96.0 * f32::from(t.dir);
        x0 = x0.min(t.x - t.half_w).min(t.x + ahead - t.half_w);
        x1 = x1.max(t.x + t.half_w).max(t.x + ahead + t.half_w);
        y0 = y0.min(t.y - t.half_h);
        y1 = y1.max(t.y + t.half_h);
    }
    (x0, x1, y0, y1)
}

impl CameraOperator {
    /// Start already framed (no ease-in at spawn).
    pub fn new(mode: CameraMode, aspect: Aspect, bounds: Bounds, input: &CameraInput) -> Self {
        let cov_w = match mode {
            CameraMode::Fixed(c) => coverage_width(c),
            CameraMode::Auto => ceil_step(required_width_in(aspect, input, Some(bounds))),
        }
        .min(bounds.w)
        .min(bounds.h * BASE_W / aspect.base_h())
        .max(MIN_W.min(bounds.w));
        let (x0, x1, y0, y1) = framing_box_in(input, Some(bounds));
        let mut op = Self {
            mode,
            aspect,
            bounds,
            cx: (x0 + x1) / 2.0,
            cy: (y0 + y1) / 2.0,
            zoom_w: cov_w as f32,
            cov_w,
            zoom_target: cov_w,
            want_in_ticks: 0,
            dwell: MIN_DWELL_TICKS,
            view: View {
                left: 0,
                top: 0,
                w: cov_w,
                h: aspect.height_for(cov_w),
            },
            zoom_changes: 0,
        };
        op.view = op.compose_view();
        op
    }

    pub fn view(&self) -> View {
        self.view
    }

    pub fn coverage(&self) -> f32 {
        coverage_of(self.cov_w)
    }

    pub fn aspect(&self) -> Aspect {
        self.aspect
    }

    pub fn mode(&self) -> CameraMode {
        self.mode
    }

    /// Largest coverage width the room allows at this aspect (the view must fit
    /// in the camera bounds).
    pub fn max_width(&self) -> i32 {
        let by_h = self.bounds.h * BASE_W / self.aspect.base_h();
        let m = MAX_W.min(self.bounds.w).min(by_h);
        (m / STEP_PX) * STEP_PX
    }

    fn compose_view(&self) -> View {
        let w = self.cov_w;
        let h = self.aspect.height_for(w);
        let left = (self.cx - w as f32 / 2.0).round() as i32;
        let top = (self.cy - h as f32 / 2.0).round() as i32;
        View {
            left: left.clamp(0, (self.bounds.w - w).max(0)),
            top: top.clamp(0, (self.bounds.h - h).max(0)),
            w,
            h,
        }
    }

    /// True when every subject's safety region is inside the current view.
    pub fn safety_ok(&self, subjects: &[Subject]) -> bool {
        subjects.iter().all(|s| safety_inside_in(self.view, s, self.bounds))
    }

    /// One presentation tick.
    pub fn step(&mut self, input: &CameraInput) -> View {
        self.dwell = self.dwell.saturating_add(1);
        self.step_zoom(input);
        self.step_pan(input);
        self.view = self.compose_view();
        self.view
    }

    fn step_zoom(&mut self, input: &CameraInput) {
        if let CameraMode::Fixed(_) = self.mode {
            return;
        }
        let raw = required_width_in(self.aspect, input, Some(self.bounds));
        let need = ceil_step(raw).min(self.max_width());
        let current = self.cov_w;
        if need > current
            || (raw + ZOOM_OUT_SLACK_PX as f32 > current as f32 && current < self.max_width())
        {
            // Safety first: zoom out at once (smoothly), also during an impact.
            self.zoom_target = need.max((current + STEP_PX).min(self.max_width()));
            self.want_in_ticks = 0;
        } else if input.impact_hold {
            // No zoom-in while a hit lands: hold the current coverage.
            self.zoom_target = current;
            self.want_in_ticks = 0;
        } else if need + ZOOM_IN_DEADBAND_PX <= current {
            self.want_in_ticks += 1;
            if self.want_in_ticks >= ZOOM_IN_DELAY_TICKS && self.dwell >= MIN_DWELL_TICKS {
                self.zoom_target = (need + ZOOM_IN_MARGIN_PX).min(current);
            }
        } else {
            self.want_in_ticks = 0;
            if self.zoom_target < current {
                // The reason to zoom in ended: stay.
                self.zoom_target = current;
            }
        }
        let target = self.zoom_target as f32;
        let delta = target - self.zoom_w;
        if delta.abs() > 0.01 {
            let (rate, min_speed) = if delta > 0.0 {
                (ZOOM_OUT_RATE, ZOOM_OUT_MIN_SPEED)
            } else {
                (ZOOM_IN_RATE, ZOOM_IN_MIN_SPEED)
            };
            let step = (delta.abs() * rate).max(min_speed).min(delta.abs());
            self.zoom_w += step * delta.signum();
        }
        // Quantise with hysteresis: switch to the next whole step only when the
        // smooth value is well past the boundary.
        // (The smooth value only travels between whole-step targets, so the
        // switch point can sit close to the old step without flicker.)
        let q = 8.0;
        let mut cov = self.cov_w;
        // A step is only taken in the direction the zoom is travelling.
        if self.zoom_target > self.cov_w && self.zoom_w >= cov as f32 + q {
            cov += STEP_PX;
        } else if self.zoom_target < self.cov_w && self.zoom_w <= cov as f32 - q {
            cov -= STEP_PX;
        }
        let cov = cov.clamp(MIN_W.min(self.max_width()), self.max_width());
        if cov != self.cov_w {
            if (cov > self.cov_w) != (self.zoom_target >= self.cov_w) || cov < self.cov_w {
                self.dwell = 0;
            }
            self.cov_w = cov;
            self.zoom_changes += 1;
        }
    }

    fn step_pan(&mut self, input: &CameraInput) {
        let (x0, x1, y0, y1) = framing_box_in(input, Some(self.bounds));
        let mut tx = (x0 + x1) / 2.0;
        let ty = (y0 + y1) / 2.0 - HEADROOM_SHARE * self.aspect.height_for(self.cov_w) as f32;
        // Horizontal bias towards a readable imminent threat, bounded so that
        // every safety region stays inside the view.
        let mut emergency = false;
        let w = self.cov_w as f32;
        if let Some(t) = input.threat
            && t.readable
            && t.tell_left <= TELL_LOOKAHEAD_TICKS
        {
            let want = ((t.x - tx) * 0.5).clamp(-MAX_BIAS_SHARE * w, MAX_BIAS_SHARE * w);
            let want = if want * f32::from(t.dir) < 0.0 {
                want * 0.4
            } else {
                want
            };
            // Largest bias that keeps all safety regions in view.
            let mut lo = f32::MIN;
            let mut hi = f32::MAX;
            for s in input.subjects {
                // view left = tx + b - w/2 <= s.x - s.half_w - pad  =>  b <= ...
                hi = hi.min(s.x - s.half_w - SAFETY_PAD_X - (tx - w / 2.0));
                lo = lo.max(s.x + s.half_w + SAFETY_PAD_X - (tx + w / 2.0));
            }
            let b = if lo <= hi {
                want.clamp(lo.min(0.0), hi.max(0.0))
            } else {
                0.0
            };
            tx += b;
        }
        // Emergency: a subject outside the safety region pans faster.
        let probe = self.view;
        for s in input.subjects {
            if !safety_inside_in(probe, s, self.bounds) {
                emergency = true;
            }
        }
        let cap = if emergency {
            MAX_PAN_EMERGENCY
        } else if input.impact_hold {
            MAX_PAN_IMPACT
        } else {
            MAX_PAN
        };
        let dx = ((tx - self.cx) * PAN_RATE).clamp(-cap, cap);
        let dy = ((ty - self.cy) * PAN_RATE).clamp(-cap, cap);
        self.cx += dx;
        self.cy += dy;
    }
}

/// [`safety_inside`] with the region clipped to the camera bounds.
pub fn safety_inside_in(view: View, s: &Subject, bounds: Bounds) -> bool {
    ((s.x - s.half_w - SAFETY_PAD_X).max(0.0) >= view.left as f32)
        && ((s.x + s.half_w + SAFETY_PAD_X).min(bounds.w as f32) <= view.right() as f32)
        && ((s.y - s.half_h - SAFETY_PAD_Y * 0.5).max(0.0) >= view.top as f32)
        && ((s.y + s.half_h + SAFETY_PAD_Y * 0.5).min(bounds.h as f32) <= view.bottom() as f32)
}

/// The safety region of a subject (body plus padding) lies inside the view.
pub fn safety_inside(view: View, s: &Subject) -> bool {
    s.x - s.half_w - SAFETY_PAD_X >= view.left as f32
        && s.x + s.half_w + SAFETY_PAD_X <= view.right() as f32
        && s.y - s.half_h - SAFETY_PAD_Y * 0.5 >= view.top as f32
        && s.y + s.half_h + SAFETY_PAD_Y * 0.5 <= view.bottom() as f32
}

/// Share of a subject's body box that is inside the view (0..=1).
pub fn visible_share(view: View, s: &Subject) -> f32 {
    let w = s.half_w * 2.0;
    let h = s.half_h * 2.0;
    let ox = ((s.x + s.half_w).min(view.right() as f32) - (s.x - s.half_w).max(view.left as f32))
        .max(0.0);
    let oy = ((s.y + s.half_h).min(view.bottom() as f32) - (s.y - s.half_h).max(view.top as f32))
        .max(0.0);
    (ox * oy) / (w * h)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOM: Bounds = Bounds { w: 1904, h: 1008 };

    fn subj(x: f32, y: f32) -> Subject {
        Subject {
            x,
            y,
            half_w: 12.0,
            half_h: 26.0,
        }
    }

    fn input<'a>(subjects: &'a [Subject], threat: Option<Threat>, hold: bool) -> CameraInput<'a> {
        CameraInput {
            subjects,
            threat,
            impact_hold: hold,
        }
    }

    #[test]
    fn candidate_coverages_are_exact_whole_pixel_views() {
        let expect = [
            (1.0, (640, 360), (640, 400)),
            (1.25, (800, 450), (800, 500)),
            (1.5, (960, 540), (960, 600)),
            (1.75, (1120, 630), (1120, 700)),
        ];
        for (c, wide, deck) in expect {
            assert_eq!(coverage_dims(Aspect::Wide16x9, c), wide, "{c}");
            assert_eq!(coverage_dims(Aspect::Deck16x10, c), deck, "{c}");
        }
        // Every step keeps even dimensions, so the view centre is a whole pixel.
        let mut w = MIN_W;
        while w <= MAX_W {
            for a in [Aspect::Wide16x9, Aspect::Deck16x10] {
                assert_eq!(w % 2, 0);
                assert_eq!(a.height_for(w) % 2, 0, "{w} {a:?}");
                assert_eq!(w * a.base_h() % BASE_W, 0);
            }
            w += STEP_PX;
        }
    }

    #[test]
    fn effective_sprite_heights_at_the_three_outputs() {
        // 1080p: x3 at 1.0, x2 at 1.5 (exact), 2.4 and 1.714 otherwise.
        assert_eq!(effective_px(52.0, 1920, 640), 156.0);
        assert_eq!(effective_px(52.0, 1920, 960), 104.0);
        assert!((effective_px(52.0, 1920, 800) - 124.8).abs() < 1e-3);
        assert!((effective_px(40.0, 1920, 1120) - 68.571).abs() < 1e-2);
        // 4K: x6 at 1.0, x4 at 1.5.
        assert_eq!(effective_px(52.0, 3840, 640), 312.0);
        assert_eq!(effective_px(52.0, 3840, 960), 208.0);
        // Deck 1280x800: x2 at 1.0 only.
        assert_eq!(effective_px(52.0, 1280, 640), 104.0);
        assert!((effective_px(52.0, 1280, 960) - 69.333).abs() < 1e-2);
        assert!(is_integer_scale(1920, 640) && is_integer_scale(1920, 960));
        assert!(!is_integer_scale(1920, 800) && !is_integer_scale(1920, 1120));
        assert!(is_integer_scale(3840, 640) && is_integer_scale(3840, 960));
        assert!(is_integer_scale(1280, 640) && !is_integer_scale(1280, 960));
        assert_eq!(texel_widths(1920, 800), (2, 3));
        assert_eq!(texel_widths(1920, 960), (2, 2));
    }

    #[test]
    fn aspect_of_an_output() {
        assert_eq!(Aspect::from_output(1920, 1080), Aspect::Wide16x9);
        assert_eq!(Aspect::from_output(3840, 2160), Aspect::Wide16x9);
        assert_eq!(Aspect::from_output(1280, 800), Aspect::Deck16x10);
    }

    #[test]
    fn fixed_modes_keep_their_coverage_and_clamp_to_the_bounds() {
        for &c in &COVERAGE_CANDIDATES {
            for aspect in [Aspect::Wide16x9, Aspect::Deck16x10] {
                let subs = [subj(40.0, 900.0), subj(80.0, 900.0)];
                let mut cam = CameraOperator::new(
                    CameraMode::Fixed(c),
                    aspect,
                    ROOM,
                    &input(&subs, None, false),
                );
                let (w, h) = coverage_dims(aspect, c);
                // Walk both actors across the room and beyond its edges.
                let mut x = -200.0f32;
                for _ in 0..1500 {
                    x += 1.6;
                    let s = [subj(x, 900.0), subj(x + 60.0, 880.0)];
                    let v = cam.step(&input(&s, None, false));
                    assert_eq!((v.w, v.h), (w, h));
                    assert!(v.left >= 0 && v.right() <= ROOM.w, "void at {v:?}");
                    assert!(v.top >= 0 && v.bottom() <= ROOM.h, "void at {v:?}");
                }
            }
        }
    }

    #[test]
    fn auto_never_shows_void_for_any_actor_position() {
        let mut cam = CameraOperator::new(
            CameraMode::Auto,
            Aspect::Wide16x9,
            ROOM,
            &input(&[subj(100.0, 900.0), subj(160.0, 900.0)], None, false),
        );
        // A deterministic pseudo random walk including far separations and
        // positions outside the room.
        let mut s = 0x1234_5678_u32;
        let mut rnd = || {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            (s % 4000) as f32 - 1000.0
        };
        for i in 0..4000 {
            let subs = if i % 400 == 0 {
                [subj(rnd(), 700.0 + rnd() % 300.0), subj(rnd(), 900.0)]
            } else {
                [subj(500.0 + (i % 400) as f32, 900.0), subj(1400.0, 600.0)]
            };
            let v = cam.step(&input(&subs, None, i % 7 == 0));
            assert!(v.left >= 0 && v.right() <= ROOM.w, "{v:?}");
            assert!(v.top >= 0 && v.bottom() <= ROOM.h, "{v:?}");
            assert!(v.w >= MIN_W && v.w <= MAX_W);
        }
    }

    #[test]
    fn auto_keeps_both_safety_regions_in_view_once_settled() {
        let mut subs = [subj(300.0, 900.0), subj(360.0, 900.0)];
        let mut cam = CameraOperator::new(
            CameraMode::Auto,
            Aspect::Wide16x9,
            ROOM,
            &input(&subs, None, false),
        );
        assert_eq!(cam.view().w, 640 + 0 * 32 + 0, "close pair stays at 1.0x");
        // Move Donut away slowly to a 700 px separation; the camera zooms out.
        let mut missed = 0;
        for tick in 0..900 {
            if tick < 300 {
                subs[1].x = 360.0 + tick as f32 * (700.0 / 300.0);
            }
            cam.step(&input(&subs, None, false));
            if tick > 40 && !cam.safety_ok(&subs) {
                missed += 1;
            }
        }
        assert!(cam.view().w >= 960 - 32, "zoomed out: {:?}", cam.view());
        assert!(cam.safety_ok(&subs));
        // Short lag while it catches up is allowed, a persistent violation is not.
        assert!(missed < 120, "safety violated for {missed} ticks");
    }

    #[test]
    fn hysteresis_prevents_zoom_pumping_at_a_threshold() {
        // The separation oscillates around the width that needs 1.25x.
        let mut cam = CameraOperator::new(
            CameraMode::Auto,
            Aspect::Wide16x9,
            ROOM,
            &input(&[subj(500.0, 900.0), subj(520.0, 900.0)], None, false),
        );
        let mut seen = std::collections::BTreeSet::new();
        for tick in 0..2400u32 {
            let sep = 380.0 + 40.0 * ((tick as f32) * 0.2).sin();
            let subs = [subj(500.0, 900.0), subj(500.0 + sep, 900.0)];
            let v = cam.step(&input(&subs, None, false));
            if tick > 200 {
                seen.insert(v.w);
            }
        }
        assert!(seen.len() <= 2, "coverage flickered over {seen:?}");
        assert!(
            cam.zoom_changes <= 4,
            "{} coverage changes for a 40 px oscillation",
            cam.zoom_changes
        );
    }

    #[test]
    fn zoom_back_in_needs_a_delay_and_never_happens_during_an_impact() {
        let wide = [subj(500.0, 900.0), subj(1100.0, 900.0)];
        let near = [subj(500.0, 900.0), subj(580.0, 900.0)];
        let mut cam = CameraOperator::new(
            CameraMode::Auto,
            Aspect::Wide16x9,
            ROOM,
            &input(&wide, None, false),
        );
        let wide_w = cam.view().w;
        assert!(wide_w >= 800);
        // Close again but a hit-stop hold for 300 ticks: no zoom-in at all.
        for _ in 0..300 {
            cam.step(&input(&near, None, true));
        }
        assert_eq!(cam.view().w, wide_w);
        // Released: still waits for the delay, then zooms in gradually.
        for _ in 0..(ZOOM_IN_DELAY_TICKS - 1) {
            cam.step(&input(&near, None, false));
        }
        assert_eq!(cam.view().w, wide_w, "no zoom-in before the delay");
        for _ in 0..600 {
            cam.step(&input(&near, None, false));
        }
        assert!(cam.view().w < wide_w);
    }

    #[test]
    fn no_hard_snap_during_impacts_and_the_pan_is_speed_limited() {
        let mut subs = [subj(700.0, 900.0), subj(760.0, 900.0)];
        let mut cam = CameraOperator::new(
            CameraMode::Auto,
            Aspect::Wide16x9,
            ROOM,
            &input(&subs, None, false),
        );
        let mut prev = cam.view();
        for tick in 0..600 {
            // A hit-stop window with a jerky actor (knockback of Carl).
            let hold = (100..160).contains(&tick);
            if tick == 110 {
                subs[0].x += 90.0;
            }
            let v = cam.step(&input(&subs, None, hold));
            let dx = (v.left - prev.left).abs() as f32 + (v.w - prev.w).abs() as f32 / 2.0;
            let cap = if hold {
                MAX_PAN_IMPACT
            } else {
                MAX_PAN_EMERGENCY
            };
            assert!(dx <= cap + 20.0 + 1.0, "tick {tick}: moved {dx}");
            if hold {
                assert!(
                    (v.left - prev.left).abs() as f32 <= MAX_PAN_EMERGENCY + 1.0,
                    "tick {tick}"
                );
                assert_eq!(v.w, prev.w, "no zoom change in the hold at tick {tick}");
            }
            prev = v;
        }
    }

    #[test]
    fn a_readable_charge_biases_the_view_but_keeps_both_actors() {
        let subs = [subj(900.0, 900.0), subj(960.0, 900.0)];
        let charge = |readable: bool, left: u16| Threat {
            x: 1500.0,
            y: 840.0,
            half_w: 60.0,
            half_h: 64.0,
            dir: -1,
            tell_left: left,
            readable,
        };
        let mk = |threat: Option<Threat>| {
            let mut cam = CameraOperator::new(
                CameraMode::Auto,
                Aspect::Wide16x9,
                ROOM,
                &input(&subs, threat, false),
            );
            let mut v = cam.view();
            for _ in 0..600 {
                v = cam.step(&input(&subs, threat, false));
            }
            (v, cam)
        };
        let (plain, _) = mk(None);
        let (hidden, _) = mk(Some(charge(false, 20)));
        let (shown, cam) = mk(Some(charge(true, 20)));
        // Hidden information is not used.
        assert_eq!(plain, hidden);
        // The boss (right of the actors) is framed: the view reaches towards it
        // and the actors stay inside their safety regions.
        assert!(
            shown.right() > plain.right() || shown.w > plain.w,
            "{shown:?} {plain:?}"
        );
        assert!(cam.safety_ok(&subs), "{shown:?}");
        assert!(
            shown.contains_point(1500.0 - 60.0, 840.0),
            "the charging boss is on screen: {shown:?}"
        );
    }

    #[test]
    fn a_threat_far_in_the_future_is_ignored() {
        let subs = [subj(900.0, 900.0), subj(960.0, 900.0)];
        let t = Threat {
            x: 1700.0,
            y: 840.0,
            half_w: 60.0,
            half_h: 64.0,
            dir: -1,
            tell_left: TELL_LOOKAHEAD_TICKS + 30,
            readable: true,
        };
        assert_eq!(
            required_width(Aspect::Wide16x9, &input(&subs, Some(t), false)),
            required_width(Aspect::Wide16x9, &input(&subs, None, false))
        );
    }

    #[test]
    fn the_view_never_exceeds_a_small_room() {
        let small = Bounds { w: 900, h: 500 };
        let subs = [subj(100.0, 450.0), subj(800.0, 300.0)];
        let mut cam = CameraOperator::new(
            CameraMode::Auto,
            Aspect::Deck16x10,
            small,
            &input(&subs, None, false),
        );
        for _ in 0..300 {
            let v = cam.step(&input(&subs, None, false));
            assert!(v.right() <= small.w && v.bottom() <= small.h, "{v:?}");
        }
    }

    #[test]
    fn visible_share_and_safety() {
        let v = View {
            left: 100,
            top: 100,
            w: 640,
            h: 360,
        };
        assert_eq!(visible_share(v, &subj(400.0, 300.0)), 1.0);
        assert_eq!(visible_share(v, &subj(100.0, 300.0)), 0.5);
        assert_eq!(visible_share(v, &subj(0.0, 300.0)), 0.0);
        assert!(safety_inside(v, &subj(400.0, 300.0)));
        assert!(!safety_inside(v, &subj(150.0, 300.0)));
    }
}
