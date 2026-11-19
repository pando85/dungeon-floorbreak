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
/// Padding below an actor's feet (the floor line): small, the floor is
/// supposed to sit low in the view.
pub const SAFETY_PAD_BELOW: f32 = 20.0;
/// Where the lowest actor's feet sit in the view (share of the view height from
/// the top): the floor low, the architecture above visible.
pub const FLOOR_SHARE: f32 = 0.8;
/// The same at 1.5x and wider.
pub const FLOOR_SHARE_WIDE: f32 = 0.86;
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
/// Pan speed limits, world px per tick (60 Hz). The emergency speed applies only
/// while an actor is outside its safety region.
pub const MAX_PAN: f32 = 5.0;
pub const MAX_PAN_IMPACT: f32 = 1.5;
pub const MAX_PAN_EMERGENCY: f32 = 10.0;
/// Fraction of the remaining pan distance closed per tick.
pub const PAN_RATE: f32 = 0.11;

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
        // Exact for every multiple of 32; for the 2 px intermediate widths of a
        // transit the height is rounded down to an even number (a whole-pixel
        // centre), so the aspect is off by at most one pixel in 360.
        (w * self.base_h() / BASE_W) / 2 * 2
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
    out_w.is_multiple_of(cov_w as u32)
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

/// Something besides the actors worth keeping in frame: an imminent charge, a
/// telegraphing or aiming ranged foe, a spit in flight, a large foe next to an
/// actor. `readable` is false while the enemy shows nothing yet (the camera must
/// not react to hidden information).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Threat {
    pub x: f32,
    pub y: f32,
    pub half_w: f32,
    pub half_h: f32,
    /// Direction of travel: +1 right, -1 left.
    pub dir: i8,
    /// Ticks until the threat commits (0 = already moving or already aiming).
    pub tell_left: u16,
    pub readable: bool,
    /// Higher is framed first and dropped last: 3 imminent (tell, charge, spit
    /// in flight), 2 a ranged foe that has acquired an actor, 1 a large foe
    /// next to an actor.
    pub priority: u8,
    /// Extra lane ahead of the threat in its direction (a charge's path), px.
    pub lane: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraInput<'a> {
    pub subjects: &'a [Subject],
    /// Threats, in any order; the operator frames them by priority while the
    /// actors' safety regions and the width cap still fit.
    pub threats: &'a [Threat],
    /// Hit-stop or a fresh impact: no zoom-in and a slower pan; a safety
    /// zoom-out still happens.
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

/// Widest coverage the automatic camera may use (1.5x, 960x540): the proposal's
/// cap. Fixed modes are not limited by it.
pub const AUTO_CAP_W: i32 = 960;
/// Legacy stepped-transit pace (kept for the fixed-mode docs): the automatic
/// camera now eases between rest widths, see [`TRANSIT_BASE_TICKS`].
pub const ZOOM_STEP_TICKS: u32 = 2;
pub const ZOOM_STEP_TICKS_URGENT: u32 = 1;
/// Eased transit between rest widths (cubic Hermite, zero velocity at the end,
/// the starting velocity carried over so a retarget never kicks): duration in
/// ticks is `base + distance / per_px`. 640 to 960: 70 ticks (1.2 s); urgent:
/// 35 ticks (0.6 s). Intermediate widths are whole even pixels.
pub const TRANSIT_BASE_TICKS: f32 = 30.0;
pub const TRANSIT_TICKS_PER_PX: f32 = 8.0;
pub const TRANSIT_URGENT_BASE_TICKS: f32 = 12.0;
pub const TRANSIT_URGENT_TICKS_PER_PX: f32 = 14.0;
/// Pan acceleration limits, px per tick squared: the pan velocity changes by at
/// most this per tick, so a start, a stop or a change of urgency never kicks.
pub const PAN_ACCEL: f32 = 0.5;
pub const PAN_ACCEL_URGENT: f32 = 1.5;

/// Widths the automatic camera may **rest** on for an output of `out_w` pixels:
/// 640 plus every 32 px width up to [`AUTO_CAP_W`] that is an exact integer
/// scale of the output (1080p: 640, 960; 4K: 640, 768, 960; 1280 wide and 2560
/// wide outputs: 640). Every other width is only a transit frame.
pub fn rest_widths(out_w: u32) -> Vec<i32> {
    let mut v = vec![MIN_W];
    let mut w = MIN_W + STEP_PX;
    while w <= AUTO_CAP_W {
        if out_w.is_multiple_of(w as u32) {
            v.push(w);
        }
        w += STEP_PX;
    }
    v
}

/// A cubic Hermite width transit.
#[derive(Debug, Clone, Copy)]
struct Transit {
    p0: f32,
    v0: f32,
    p1: f32,
    t: f32,
    dur: f32,
    urgent: bool,
}

impl Transit {
    /// Width and velocity (px per tick) at `t` ticks.
    fn at(&self, t: f32) -> (f32, f32) {
        let u = (t / self.dur).clamp(0.0, 1.0);
        let (u2, u3) = (u * u, u * u * u);
        let h00 = 2.0 * u3 - 3.0 * u2 + 1.0;
        let h10 = u3 - 2.0 * u2 + u;
        let h01 = -2.0 * u3 + 3.0 * u2;
        let p = h00 * self.p0 + h10 * self.dur * self.v0 + h01 * self.p1;
        let dh00 = 6.0 * u2 - 6.0 * u;
        let dh10 = 3.0 * u2 - 4.0 * u + 1.0;
        let dh01 = -6.0 * u2 + 6.0 * u;
        let v = (dh00 * self.p0 + dh10 * self.dur * self.v0 + dh01 * self.p1) / self.dur;
        (p, v)
    }
}

#[derive(Debug, Clone)]
pub struct CameraOperator {
    mode: CameraMode,
    aspect: Aspect,
    bounds: Bounds,
    cx: f32,
    cy: f32,
    /// Output width the rest widths were derived for.
    out_w: u32,
    rest: Vec<i32>,
    /// The width the automatic camera rests on (or is heading to).
    rest_w: i32,
    cov_w: i32,
    /// Smooth width (px) and its velocity (px per tick); `cov_w` is this
    /// rounded to an even whole number.
    w_f: f32,
    w_v: f32,
    transit: Option<Transit>,
    pan_v: (f32, f32),
    want_in_ticks: u32,
    dwell: u32,
    view: View,
    /// Rest-width transitions so far (a widen or a return counts once): the
    /// zoom pumping measure.
    pub zoom_changes: u32,
    /// Whole 32 px steps taken so far.
    pub zoom_steps: u32,
    last_urgent: bool,
}

/// The tightest coverage width that fits every actor's safety region and the
/// readable threats that still fit. Not clamped to the bounds of a room.
pub fn required_width(aspect: Aspect, input: &CameraInput) -> f32 {
    required_width_in(aspect, input, None, f32::MAX)
}

/// [`required_width`] with the regions clipped to `bounds` and threats added
/// only while the box stays within `cap_w`.
pub fn required_width_in(
    aspect: Aspect,
    input: &CameraInput,
    bounds: Option<Bounds>,
    cap_w: f32,
) -> f32 {
    let b = framing_box_in(input, bounds, aspect, cap_w);
    box_width(aspect, b)
}

fn box_width(aspect: Aspect, (x0, x1, y0, y1): (f32, f32, f32, f32)) -> f32 {
    (x1 - x0).max((y1 - y0) * BASE_W as f32 / aspect.base_h() as f32)
}

fn threat_in_set(t: &Threat) -> bool {
    t.readable && t.tell_left <= TELL_LOOKAHEAD_TICKS
}

/// Threats in framing order: priority first, then the least time left, then
/// position (stable).
fn ordered_threats<'a>(input: &'a CameraInput) -> Vec<&'a Threat> {
    let mut v: Vec<&Threat> = input.threats.iter().filter(|t| threat_in_set(t)).collect();
    v.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then(a.tell_left.cmp(&b.tell_left))
            .then(a.x.total_cmp(&b.x))
    });
    v
}

fn extend_box(b: (f32, f32, f32, f32), t: &Threat) -> (f32, f32, f32, f32) {
    let ahead = t.lane * f32::from(t.dir);
    (
        b.0.min(t.x - t.half_w).min(t.x + ahead - t.half_w),
        b.1.max(t.x + t.half_w).max(t.x + ahead + t.half_w),
        b.2.min(t.y - t.half_h),
        b.3.max(t.y + t.half_h),
    )
}

/// Bounding box (x0, x1, y0, y1) of the actors' safety regions (clipped to the
/// camera bounds: an actor against a wall cannot be padded beyond it) plus the
/// threats that still fit within `cap_w`, highest priority first.
fn framing_box_in(
    input: &CameraInput,
    bounds: Option<Bounds>,
    aspect: Aspect,
    cap_w: f32,
) -> (f32, f32, f32, f32) {
    let mut x0 = f32::MAX;
    let mut x1 = f32::MIN;
    let mut y0 = f32::MAX;
    let mut y1 = f32::MIN;
    let (bw, bh) = bounds.map_or((f32::MAX, f32::MAX), |b| (b.w as f32, b.h as f32));
    for s in input.subjects {
        x0 = x0.min((s.x - s.half_w - SAFETY_PAD_X).max(0.0));
        x1 = x1.max((s.x + s.half_w + SAFETY_PAD_X).min(bw));
        y0 = y0.min((s.y - s.half_h - SAFETY_PAD_Y).max(0.0));
        y1 = y1.max((s.y + s.half_h + SAFETY_PAD_BELOW).min(bh));
    }
    if input.subjects.is_empty() {
        return (0.0, BASE_W as f32, 0.0, 360.0);
    }
    let mut b = (x0, x1, y0, y1);
    for t in ordered_threats(input) {
        let candidate = extend_box(b, t);
        if box_width(aspect, candidate) <= cap_w {
            b = candidate;
        }
    }
    b
}

/// True when every threat admitted to the framing set is at least 95 percent
/// inside `view`.
fn threats_in_view(view: View, input: &CameraInput, aspect: Aspect, cap_w: f32) -> bool {
    // Only the threats that were admitted to the framing box count.
    let mut b = framing_box_in(
        &CameraInput {
            threats: &[],
            ..*input
        },
        None,
        aspect,
        cap_w,
    );
    for t in ordered_threats(input) {
        let candidate = extend_box(b, t);
        if box_width(aspect, candidate) <= cap_w {
            b = candidate;
            let body = Subject {
                x: t.x,
                y: t.y,
                half_w: t.half_w,
                half_h: t.half_h,
            };
            if visible_share(view, &body) < 0.95 {
                return false;
            }
        }
    }
    true
}

/// Where the lowest actor's feet sit in the view (share of the height from the
/// top): 0.8 at 1.0x, 0.86 from 1.5x, so a wider view shows architecture, not
/// foundation.
pub fn floor_share(cov_w: i32) -> f32 {
    floor_share_f(cov_w as f32)
}

/// [`floor_share`] for a smooth (fractional) width, so the floor anchor moves on
/// the same curve as the zoom.
pub fn floor_share_f(w: f32) -> f32 {
    let t = ((w - MIN_W as f32) / 320.0).clamp(0.0, 1.0);
    FLOOR_SHARE + (FLOOR_SHARE_WIDE - FLOOR_SHARE) * t
}

impl CameraOperator {
    /// Start already framed (no ease-in at spawn), for a 1920 px wide output.
    pub fn new(mode: CameraMode, aspect: Aspect, bounds: Bounds, input: &CameraInput) -> Self {
        Self::new_for_output(mode, aspect, bounds, 1920, input)
    }

    /// Start already framed for an output `out_w` pixels wide (it decides the
    /// automatic camera's rest widths).
    pub fn new_for_output(
        mode: CameraMode,
        aspect: Aspect,
        bounds: Bounds,
        out_w: u32,
        input: &CameraInput,
    ) -> Self {
        let limit = (bounds.w)
            .min(bounds.h * BASE_W / aspect.base_h())
            .max(MIN_W.min(bounds.w));
        let mut rest: Vec<i32> = rest_widths(out_w)
            .into_iter()
            .filter(|w| *w <= limit)
            .collect();
        if rest.is_empty() {
            rest.push(MIN_W.min(limit));
        }
        let cap = *rest.last().unwrap_or(&MIN_W);
        let (cov_w, rest_w) = match mode {
            CameraMode::Fixed(c) => {
                let w = coverage_width(c).min(limit).max(MIN_W.min(bounds.w));
                (w, w)
            }
            CameraMode::Auto => {
                let raw = required_width_in(aspect, input, Some(bounds), cap as f32);
                let w = rest
                    .iter()
                    .copied()
                    .find(|r| *r as f32 >= raw)
                    .unwrap_or(cap);
                (w, w)
            }
        };
        let (x0, x1, y0, y1) = framing_box_in(input, Some(bounds), aspect, cap as f32);
        let mut op = Self {
            mode,
            aspect,
            bounds,
            cx: (x0 + x1) / 2.0,
            cy: (y0 + y1) / 2.0,
            out_w,
            rest,
            rest_w,
            cov_w,
            w_f: cov_w as f32,
            w_v: 0.0,
            transit: None,
            pan_v: (0.0, 0.0),
            want_in_ticks: 0,
            dwell: MIN_DWELL_TICKS,
            view: View {
                left: 0,
                top: 0,
                w: cov_w,
                h: aspect.height_for(cov_w),
            },
            zoom_changes: 0,
            zoom_steps: 0,
            last_urgent: false,
        };
        op.cy = op.target_cy(input, y0, y1);
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

    /// Whether the last step ran at the urgent pace (something to frame was
    /// outside the view).
    pub fn urgent(&self) -> bool {
        self.last_urgent
    }

    /// The width the automatic camera is resting on or heading to.
    pub fn rest_target(&self) -> i32 {
        self.rest_w
    }

    pub fn output_width(&self) -> u32 {
        self.out_w
    }

    /// The widths the automatic camera rests on at this output (never above the
    /// cap and the room).
    pub fn rest_set(&self) -> &[i32] {
        &self.rest
    }

    /// Widest width the automatic camera may use: the cap, at most the room.
    pub fn auto_cap(&self) -> i32 {
        *self.rest.last().unwrap_or(&MIN_W)
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
        subjects
            .iter()
            .all(|s| safety_inside_in(self.view, s, self.bounds))
    }

    /// One presentation tick.
    pub fn step(&mut self, input: &CameraInput) -> View {
        self.dwell = self.dwell.saturating_add(1);
        let urgent = self.something_outside(input);
        self.last_urgent = urgent;
        self.step_zoom(input, urgent);
        self.step_pan(input, urgent);
        self.view = self.compose_view();
        self.view
    }

    /// An actor's safety region or a framed threat is outside the current view.
    fn something_outside(&self, input: &CameraInput) -> bool {
        let cap = self.auto_cap() as f32;
        input
            .subjects
            .iter()
            .any(|s| !safety_inside_in(self.view, s, self.bounds))
            || !threats_in_view(self.view, input, self.aspect, cap)
    }

    fn step_zoom(&mut self, input: &CameraInput, urgent: bool) {
        if let CameraMode::Fixed(_) = self.mode {
            return;
        }
        let cap = self.auto_cap() as f32;
        let raw = required_width_in(self.aspect, input, Some(self.bounds), cap);
        let smallest_at_least = |rest: &[i32], w: f32| -> i32 {
            rest.iter()
                .copied()
                .find(|r| *r as f32 >= w)
                .unwrap_or(*rest.last().unwrap_or(&MIN_W))
        };
        if raw + ZOOM_OUT_SLACK_PX as f32 > self.rest_w as f32 && self.rest_w < self.auto_cap() {
            // Widen at once, straight to the rest width that fits (a short transit).
            let target = smallest_at_least(&self.rest, raw + ZOOM_OUT_SLACK_PX as f32);
            if target > self.rest_w {
                self.rest_w = target;
                self.zoom_changes += 1;
                self.dwell = 0;
            }
            self.want_in_ticks = 0;
        } else if input.impact_hold {
            self.want_in_ticks = 0;
        } else {
            let lower = smallest_at_least(&self.rest, raw + ZOOM_IN_DEADBAND_PX as f32);
            if lower < self.rest_w {
                self.want_in_ticks += 1;
                if self.want_in_ticks >= ZOOM_IN_DELAY_TICKS && self.dwell >= MIN_DWELL_TICKS {
                    self.rest_w = lower;
                    self.zoom_changes += 1;
                    self.dwell = 0;
                    self.want_in_ticks = 0;
                }
            } else {
                self.want_in_ticks = 0;
            }
        }
        // Transit: an eased Hermite curve to the rest width. A return is held
        // during an impact; an urgent widening restarts a slower one faster,
        // keeping the velocity.
        let target = self.rest_w as f32;
        let moving = self.transit.is_some();
        if !moving && (self.w_f - target).abs() > 0.01 {
            self.start_transit(target, urgent && target > self.w_f);
        } else if let Some(tr) = self.transit {
            let retarget = (tr.p1 - target).abs() > 0.01;
            let hurry = urgent && target > self.w_f && !tr.urgent;
            if retarget || hurry {
                self.start_transit(target, urgent && target > self.w_f);
            }
        }
        if let Some(mut tr) = self.transit {
            let inward = tr.p1 < tr.p0;
            if !(input.impact_hold && inward) {
                tr.t += 1.0;
                let (p, v) = tr.at(tr.t);
                self.w_f = p.clamp(MIN_W as f32, self.auto_cap() as f32);
                self.w_v = v;
                if tr.t >= tr.dur {
                    self.w_f = tr.p1;
                    self.w_v = 0.0;
                    self.transit = None;
                } else {
                    self.transit = Some(tr);
                }
            } else {
                self.w_v = 0.0;
            }
        }
        let rounded = ((self.w_f / 2.0).round() as i32) * 2;
        let rounded = rounded.clamp(MIN_W.min(self.auto_cap()), self.auto_cap());
        if rounded != self.cov_w {
            self.zoom_steps += 1;
        }
        self.cov_w = rounded;
    }

    fn start_transit(&mut self, target: f32, urgent: bool) {
        let dist = (target - self.w_f).abs();
        let dur = if urgent {
            TRANSIT_URGENT_BASE_TICKS + dist / TRANSIT_URGENT_TICKS_PER_PX * 1.0
        } else {
            TRANSIT_BASE_TICKS + dist / TRANSIT_TICKS_PER_PX
        }
        .max(2.0);
        self.transit = Some(Transit {
            p0: self.w_f,
            v0: self.w_v,
            p1: target,
            t: 0.0,
            dur,
            urgent,
        });
    }

    /// Vertical centre: the lowest actor's feet at [`floor_share`] of the view,
    /// clamped so every safety region stays inside the view.
    fn target_cy(&self, input: &CameraInput, y0: f32, y1: f32) -> f32 {
        let h = self.aspect.height_for(self.cov_w) as f32;
        let lowest = input
            .subjects
            .iter()
            .map(|s| s.y + s.half_h)
            .fold(f32::MIN, f32::max);
        if input.subjects.is_empty() {
            return (y0 + y1) / 2.0;
        }
        let want = lowest - (floor_share_f(self.w_f) - 0.5) * h;
        let (lo, hi) = (y1 - h / 2.0, y0 + h / 2.0);
        if lo <= hi {
            want.clamp(lo, hi)
        } else {
            (y0 + y1) / 2.0
        }
    }

    fn step_pan(&mut self, input: &CameraInput, urgent: bool) {
        let cap = match self.mode {
            CameraMode::Auto => self.auto_cap() as f32,
            CameraMode::Fixed(_) => self.cov_w as f32,
        };
        let (x0, x1, y0, y1) = framing_box_in(input, Some(self.bounds), self.aspect, cap);
        let mut tx = (x0 + x1) / 2.0;
        let ty = self.target_cy(input, y0, y1);
        // Horizontal bias towards the highest-priority readable threat, bounded
        // so that every safety region stays inside the view.
        let w = self.cov_w as f32;
        if let Some(t) = ordered_threats(input).first() {
            let want = ((t.x - tx) * 0.5).clamp(-MAX_BIAS_SHARE * w, MAX_BIAS_SHARE * w);
            let want = if want * f32::from(t.dir) < 0.0 {
                want * 0.4
            } else {
                want
            };
            let mut lo = f32::MIN;
            let mut hi = f32::MAX;
            for s in input.subjects {
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
        // Aim only at centres the view can actually reach, so the follower slows
        // down towards a room edge instead of the clamp stopping it dead.
        let (hw, hh) = (
            self.cov_w as f32 / 2.0,
            self.aspect.height_for(self.cov_w) as f32 / 2.0,
        );
        if self.bounds.w as f32 > 2.0 * hw {
            tx = tx.clamp(hw, self.bounds.w as f32 - hw);
        }
        let ty = if self.bounds.h as f32 > 2.0 * hh {
            ty.clamp(hh, self.bounds.h as f32 - hh)
        } else {
            ty
        };
        // Urgent: an actor or a framed threat is outside the view, pan faster.
        // The pan is a bounded-acceleration follower: the velocity chases a
        // capped, proportional speed and may change by at most `accel` per tick,
        // so starting, stopping and a change of urgency never kick.
        let (v_cap, accel) = if urgent {
            (MAX_PAN_EMERGENCY, PAN_ACCEL_URGENT)
        } else if input.impact_hold {
            (MAX_PAN_IMPACT, PAN_ACCEL)
        } else {
            (MAX_PAN, PAN_ACCEL)
        };
        let follow = |target: f32, c: f32, v: f32| -> f32 {
            let want = ((target - c) * PAN_RATE).clamp(-v_cap, v_cap);
            v + (want - v).clamp(-accel, accel)
        };
        self.pan_v.0 = follow(tx, self.cx, self.pan_v.0);
        self.pan_v.1 = follow(ty, self.cy, self.pan_v.1);
        self.cx += self.pan_v.0;
        self.cy += self.pan_v.1;
    }
}

/// [`safety_inside`] with the region clipped to the camera bounds.
pub fn safety_inside_in(view: View, s: &Subject, bounds: Bounds) -> bool {
    ((s.x - s.half_w - SAFETY_PAD_X).max(0.0) >= view.left as f32)
        && ((s.x + s.half_w + SAFETY_PAD_X).min(bounds.w as f32) <= view.right() as f32)
        && ((s.y - s.half_h - SAFETY_PAD_Y * 0.5).max(0.0) >= view.top as f32)
        && ((s.y + s.half_h + SAFETY_PAD_BELOW * 0.5).min(bounds.h as f32) <= view.bottom() as f32)
}

/// The safety region of a subject (body plus padding) lies inside the view.
pub fn safety_inside(view: View, s: &Subject) -> bool {
    s.x - s.half_w - SAFETY_PAD_X >= view.left as f32
        && s.x + s.half_w + SAFETY_PAD_X <= view.right() as f32
        && s.y - s.half_h - SAFETY_PAD_Y * 0.5 >= view.top as f32
        && s.y + s.half_h + SAFETY_PAD_BELOW * 0.5 <= view.bottom() as f32
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

    fn input<'a>(subjects: &'a [Subject], threats: &'a [Threat], hold: bool) -> CameraInput<'a> {
        CameraInput {
            subjects,
            threats,
            impact_hold: hold,
        }
    }

    fn auto(out_w: u32, subs: &[Subject]) -> CameraOperator {
        CameraOperator::new_for_output(
            CameraMode::Auto,
            Aspect::Wide16x9,
            ROOM,
            out_w,
            &input(subs, &[], false),
        )
    }

    fn charge(x: f32, readable: bool, left: u16) -> Threat {
        Threat {
            x,
            y: 840.0,
            half_w: 60.0,
            half_h: 64.0,
            dir: -1,
            tell_left: left,
            readable,
            priority: 3,
            lane: 96.0,
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
        assert_eq!(effective_px(52.0, 1920, 640), 156.0);
        assert_eq!(effective_px(52.0, 1920, 960), 104.0);
        assert!((effective_px(52.0, 1920, 800) - 124.8).abs() < 1e-3);
        assert!((effective_px(40.0, 1920, 1120) - 68.571).abs() < 1e-2);
        assert_eq!(effective_px(52.0, 3840, 640), 312.0);
        assert_eq!(effective_px(52.0, 3840, 960), 208.0);
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
    fn rest_widths_are_the_exact_integer_scales_up_to_the_cap() {
        assert_eq!(rest_widths(1920), vec![640, 960]);
        assert_eq!(rest_widths(3840), vec![640, 768, 960]);
        assert_eq!(rest_widths(1280), vec![640]); // Deck 1280x800 and 720p
        assert_eq!(rest_widths(2560), vec![640]); // 1440p
        for out in [1280u32, 1920, 2560, 3840, 1366, 1600] {
            for w in rest_widths(out) {
                assert!((MIN_W..=AUTO_CAP_W).contains(&w));
                assert!(w == MIN_W || is_integer_scale(out, w), "{out} {w}");
            }
        }
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
                    &input(&subs, &[], false),
                );
                let (w, h) = coverage_dims(aspect, c);
                let mut x = -200.0f32;
                for _ in 0..1500 {
                    x += 1.6;
                    let s = [subj(x, 900.0), subj(x + 60.0, 880.0)];
                    let v = cam.step(&input(&s, &[], false));
                    assert_eq!((v.w, v.h), (w, h));
                    assert!(v.left >= 0 && v.right() <= ROOM.w, "void at {v:?}");
                    assert!(v.top >= 0 && v.bottom() <= ROOM.h, "void at {v:?}");
                }
            }
        }
    }

    #[test]
    fn auto_never_shows_void_and_never_exceeds_the_cap() {
        for out in [1920u32, 3840, 1280] {
            let mut cam = auto(out, &[subj(100.0, 900.0), subj(160.0, 900.0)]);
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
                let v = cam.step(&input(&subs, &[], i % 7 == 0));
                assert!(v.left >= 0 && v.right() <= ROOM.w, "{v:?}");
                assert!(v.top >= 0 && v.bottom() <= ROOM.h, "{v:?}");
                assert!(v.w >= MIN_W && v.w <= AUTO_CAP_W, "{out}: width {}", v.w);
            }
        }
    }

    #[test]
    fn auto_rests_only_on_integer_scale_widths_and_transits_fast() {
        for out in [1920u32, 3840, 1280] {
            let rest = rest_widths(out);
            let mut cam = auto(out, &[subj(500.0, 900.0), subj(560.0, 900.0)]);
            let mut transit_run = 0u32;
            let mut longest_transit = 0u32;
            let mut seen_rest = std::collections::BTreeSet::new();
            for tick in 0..3000u32 {
                // The separation sweeps up and down slowly.
                let sep = 60.0 + 620.0 * (0.5 - 0.5 * ((tick as f32) * 0.004).cos());
                let subs = [subj(500.0, 900.0), subj(500.0 + sep, 900.0)];
                let v = cam.step(&input(&subs, &[], false));
                if rest.contains(&v.w) {
                    seen_rest.insert(v.w);
                    transit_run = 0;
                } else {
                    transit_run += 1;
                    longest_transit = longest_transit.max(transit_run);
                }
            }
            assert!(
                longest_transit <= 72,
                "{out}: a transit lasted {longest_transit} ticks"
            );
            assert!(seen_rest.iter().all(|w| rest.contains(w)));
            if out == 1920 {
                assert!(seen_rest.contains(&640) && seen_rest.contains(&960));
            }
        }
    }

    #[test]
    fn auto_keeps_both_safety_regions_in_view_once_settled_within_the_cap() {
        let mut subs = [subj(300.0, 900.0), subj(360.0, 900.0)];
        let mut cam = auto(1920, &subs);
        assert_eq!(cam.view().w, 640, "a close pair stays at 1.0x");
        let mut missed = 0;
        for tick in 0..900 {
            if tick < 300 {
                subs[1].x = 360.0 + tick as f32 * (370.0 / 300.0);
            }
            cam.step(&input(&subs, &[], false));
            if tick > 40 && !cam.safety_ok(&subs) {
                missed += 1;
            }
        }
        // 730 px apart: 960 fits both safety regions (744).
        assert_eq!(cam.view().w, 960);
        assert!(cam.safety_ok(&subs));
        assert!(missed < 60, "safety violated for {missed} ticks");
    }

    #[test]
    fn hysteresis_prevents_zoom_pumping_at_a_threshold() {
        let mut cam = auto(1920, &[subj(500.0, 900.0), subj(520.0, 900.0)]);
        let mut seen = std::collections::BTreeSet::new();
        for tick in 0..2400u32 {
            let sep = 380.0 + 40.0 * ((tick as f32) * 0.2).sin();
            let subs = [subj(500.0, 900.0), subj(500.0 + sep, 900.0)];
            let v = cam.step(&input(&subs, &[], false));
            if tick > 200 {
                seen.insert(v.w);
            }
        }
        assert!(seen.len() <= 2, "coverage flickered over {seen:?}");
        assert!(cam.zoom_changes <= 3, "{} rest changes", cam.zoom_changes);
    }

    #[test]
    fn zoom_back_in_needs_a_delay_and_never_happens_during_an_impact() {
        let wide = [subj(500.0, 900.0), subj(1100.0, 900.0)];
        let near = [subj(500.0, 900.0), subj(580.0, 900.0)];
        let mut cam = auto(1920, &wide);
        assert_eq!(cam.view().w, 960);
        for _ in 0..300 {
            cam.step(&input(&near, &[], true));
        }
        assert_eq!(cam.view().w, 960);
        for _ in 0..(ZOOM_IN_DELAY_TICKS - 1) {
            cam.step(&input(&near, &[], false));
        }
        assert_eq!(cam.view().w, 960, "no zoom-in before the delay");
        for _ in 0..80 {
            cam.step(&input(&near, &[], false));
        }
        assert_eq!(cam.view().w, 640, "returned within a short transit");
    }

    #[test]
    fn no_hard_snap_during_impacts_and_the_pan_is_speed_limited() {
        let mut subs = [subj(700.0, 900.0), subj(760.0, 900.0)];
        let mut cam = auto(1920, &subs);
        let mut prev = cam.view();
        for tick in 0..600 {
            let hold = (100..160).contains(&tick);
            if tick == 110 {
                subs[0].x += 90.0;
            }
            let v = cam.step(&input(&subs, &[], hold));
            // A pan step plus at most one 32 px zoom step (16 px per edge).
            let dl = (v.left - prev.left).abs() as f32;
            assert!(
                dl <= MAX_PAN_EMERGENCY + 16.0 + 1.0,
                "tick {tick}: moved {dl}"
            );
            if hold {
                assert_eq!(v.w, prev.w.min(v.w).max(prev.w.min(v.w)), "tick {tick}");
                assert!(
                    v.w <= prev.w,
                    "no widening change in the hold unless required"
                );
            }
            prev = v;
        }
    }

    #[test]
    fn a_readable_charge_biases_the_view_but_keeps_both_actors() {
        let subs = [subj(900.0, 900.0), subj(960.0, 900.0)];
        let settle = |threats: &[Threat]| {
            let mut cam = auto(1920, &subs);
            let mut v = cam.view();
            for _ in 0..600 {
                v = cam.step(&input(&subs, threats, false));
            }
            (v, cam)
        };
        let (plain, _) = settle(&[]);
        let (hidden, _) = settle(&[charge(1500.0, false, 20)]);
        let (shown, cam) = settle(&[charge(1500.0, true, 20)]);
        assert_eq!(plain, hidden, "hidden information is not used");
        assert!(
            shown.right() > plain.right() || shown.w > plain.w,
            "{shown:?} {plain:?}"
        );
        assert!(cam.safety_ok(&subs), "{shown:?}");
        assert!(
            shown.contains_point(1500.0 - 60.0, 840.0),
            "boss on screen: {shown:?}"
        );
        assert!(shown.w <= AUTO_CAP_W);
    }

    #[test]
    fn threats_are_dropped_by_priority_when_the_cap_cannot_hold_them() {
        let subs = [subj(900.0, 900.0), subj(960.0, 900.0)];
        let mut low = charge(1700.0, true, 0);
        low.priority = 1;
        let mut high = charge(1100.0, true, 0);
        high.priority = 3;
        let w = |threats: &[Threat]| {
            required_width_in(Aspect::Wide16x9, &input(&subs, threats, false), None, 960.0)
        };
        // The far low-priority threat alone does not fit; with a high-priority
        // one the high one is kept and the low one dropped.
        assert_eq!(w(&[low]), w(&[]));
        assert!(w(&[high]) > w(&[]));
        assert_eq!(w(&[low, high]), w(&[high]));
    }

    #[test]
    fn a_threat_outside_the_view_pans_and_widens_at_the_urgent_pace() {
        let subs = [subj(900.0, 900.0), subj(960.0, 900.0)];
        let mut cam = auto(1920, &subs);
        assert_eq!(cam.view().w, 640);
        let t = [Threat {
            lane: 0.0,
            ..charge(1500.0, true, 0)
        }];
        for _ in 0..50 {
            cam.step(&input(&subs, &t, false));
        }
        // The urgent eased transit (about 32 ticks) has reached 960.
        assert_eq!(cam.view().w, 960);
    }

    #[test]
    fn a_threat_far_in_the_future_is_ignored() {
        let subs = [subj(900.0, 900.0), subj(960.0, 900.0)];
        let t = [charge(1700.0, true, TELL_LOOKAHEAD_TICKS + 30)];
        assert_eq!(
            required_width(Aspect::Wide16x9, &input(&subs, &t, false)),
            required_width(Aspect::Wide16x9, &input(&subs, &[], false))
        );
    }

    #[test]
    fn the_view_never_exceeds_a_small_room() {
        let small = Bounds { w: 900, h: 500 };
        let subs = [subj(100.0, 450.0), subj(800.0, 300.0)];
        let mut cam = CameraOperator::new_for_output(
            CameraMode::Auto,
            Aspect::Deck16x10,
            small,
            3840,
            &input(&subs, &[], false),
        );
        for _ in 0..300 {
            let v = cam.step(&input(&subs, &[], false));
            assert!(v.right() <= small.w && v.bottom() <= small.h, "{v:?}");
        }
    }

    #[test]
    fn the_floor_sits_lower_when_wider() {
        assert_eq!(floor_share(640), FLOOR_SHARE);
        assert_eq!(floor_share(960), FLOOR_SHARE_WIDE);
        assert!(floor_share(800) > FLOOR_SHARE && floor_share(800) < FLOOR_SHARE_WIDE);
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
