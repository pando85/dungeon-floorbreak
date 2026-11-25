//! DONUT-PROG-01 DP-D (DP-Q34) and DONUT-MOVES-ART-01: the cues of Donut's
//! Puddle Jumper, the pure half. Read-only from the event log, in the shape of
//! [`crate::donut_cues`]:
//!
//! - on `SpecialStarted` the puddle opens at her feet (`ring_open` of the
//!   Puddle Jumper strip, 3 ticks a frame) while the windup plays the moves
//!   frames `blink_crouch`, `blink_dip` and `blink_vanish` (the presenter draws
//!   those from the blink state with [`crate::donut_art::blink_crouch_frame`]);
//! - on `BlinkResolved` the same ring opens at the destination, the origin ring
//!   closes (`ring_close`, from age 6), a tiled streak (`streak_dash` and
//!   `streak_head`) runs between the two points for [`STREAK_TICKS`] ticks (the
//!   data's `dpq34_streak_frames`) and she rises out of the destination puddle
//!   (`blink_pop`, `blink_arrive`, `blink_settle`, ages 6 to 12);
//! - on `BlinkFizzled` she bonks and huffs (`fizzle_bonk`, `fizzle_huff`) over
//!   [`FIZZLE_TICKS`] ticks while the strip's drying ring (`fizzle_0..2`) plays;
//! - on `BlinkCancelled` there is no new frame: whatever owns her sprite takes
//!   over, and the origin ring closes from the cancel tick.
//!
//! The colours are the Magic Missile VFX palette (the strip's own). No aura, no
//! glow on her body, no screen shake. **Presentation only**: nothing here is
//! hashed, replayed or read by the simulation, so a run with the cues drawn and
//! one without have the same state hash (a test in `donut_prog_fixtures`). Every
//! cue is a function of event ticks, so a render that skips frames or a replay
//! seek draws the same picture for the same tick.
//!
//! The numbers below are adopted defaults of DP-Q34's stand-ins and of the
//! DONUT-MOVES-ART-01 spec (sections 5 and 6); the owner may retune them after
//! playing.

use crate::combat::CombatEvent;
use crate::combat_scene::CombatState;
use crate::donut_art::{PuddleVfx, arrive_frame, fizzle_frame, puddle_vfx_frame};
use crate::donut_prog::DonutProgTuning;

/// The hero index of Donut in the event log.
const DONUT: u8 = 1;

/// The arrival tick counted from the press (DP-Q25: t0 to t5 windup, t6 arrival).
pub const ARRIVE_AGE: u64 = 6;
/// The fizzle cue lasts this many ticks (DP-Q34; the strip's drying ring is 8 ticks too).
pub const FIZZLE_TICKS: u64 = 8;
/// The streak lies this many pixels above her feet (across her body, low).
pub const STREAK_LIFT_PX: i32 = 8;
/// The width of one tiled streak dash and of the streak head, in px (strip frames 7 and 8).
pub const DASH_W_PX: i32 = 16;
pub const HEAD_W_PX: i32 = 4;

/// One puddle effect of the strip: its centre column and foot row (world px, y
/// up) and the strip frame to draw there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PuddleCue {
    pub x: i32,
    pub foot_y: i32,
    pub frame: usize,
}

/// The tiled streak between two columns on one foot row: `streak_dash` tiles from
/// the origin and the `streak_head` at the arrival end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Streak {
    pub from_x: i32,
    pub to_x: i32,
    pub foot_y: i32,
}

impl Streak {
    /// +1 when she blinks right, -1 when left (0 when the two columns are equal).
    pub const fn dir(&self) -> i32 {
        (self.to_x - self.from_x).signum()
    }

    /// The near edge of each dash tile, in the streak's own direction: the dashes
    /// start at the origin column and run toward the destination, keeping
    /// [`HEAD_W_PX`] free for the head. A tile spans `edge` to `edge + dir *
    /// DASH_W_PX`. Empty when the span is shorter than the head.
    pub fn dash_edges(&self) -> Vec<i32> {
        let dir = self.dir();
        let reach = (self.to_x - self.from_x).abs() - HEAD_W_PX;
        if dir == 0 || reach <= 0 {
            return Vec::new();
        }
        let n = (reach as u32).div_ceil(DASH_W_PX as u32) as i32;
        (0..n).map(|i| self.from_x + dir * i * DASH_W_PX).collect()
    }

    /// The far edge of the head: the head lies on the last [`HEAD_W_PX`] columns before the destination column.
    pub const fn head_edge(&self) -> i32 {
        self.to_x - self.dir() * HEAD_W_PX
    }

    pub const fn y(&self) -> i32 {
        self.foot_y + STREAK_LIFT_PX
    }
}

/// What the cues look like on one tick.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlinkCueView {
    /// The strip's puddle effects (rings and the drying fizzle ring).
    pub puddles: Vec<PuddleCue>,
    pub streak: Option<Streak>,
    /// The moves frame she is drawn in at the destination (ages 6 to 12 of the press).
    pub arrive: Option<usize>,
    /// The moves frame of a fizzle (she never left the origin).
    pub fizzle: Option<usize>,
}

/// The pure cue state: what the event log said, as ticks and columns.
#[derive(Debug, Clone, Default)]
pub struct BlinkCues {
    seen_events: usize,
    last_tick: u64,
    /// The press: (tick, column, foot row).
    start: Option<(u64, i32, i32)>,
    /// The arrival: (tick, from column, to column, foot row).
    arrival: Option<(u64, i32, i32, i32)>,
    /// The fizzle: (tick, column, foot row).
    fizzle: Option<(u64, i32, i32)>,
    /// The tick a windup was cancelled.
    cancel: Option<u64>,
}

impl BlinkCues {
    /// Back to nothing (a new attempt, a replay seek backwards).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Advance to `tick` from the finished simulation state. Read-only. `x` and
    /// `foot_y` are her body column and foot row (world px, y up) on this tick.
    pub fn step(&mut self, tick: u64, combat: &CombatState, x: i32, foot_y: i32) {
        let total = combat.dropped_events + combat.events.len();
        if tick < self.last_tick || total < self.seen_events {
            self.reset();
        }
        self.last_tick = tick;
        let fresh = total
            .saturating_sub(self.seen_events)
            .min(combat.events.len());
        self.seen_events = total;
        for event in &combat.events[combat.events.len() - fresh..] {
            match *event {
                CombatEvent::SpecialStarted {
                    tick,
                    hero: DONUT,
                    capability,
                } if capability == crate::donut_prog::SPECIAL_SHORT_BLINK.as_str() => {
                    self.start = Some((tick, x, foot_y));
                    self.cancel = None;
                }
                CombatEvent::BlinkResolved {
                    tick,
                    hero: DONUT,
                    from_x,
                    to_x,
                } => self.arrival = Some((tick, from_x, to_x, foot_y)),
                CombatEvent::BlinkFizzled {
                    tick, hero: DONUT, ..
                } => {
                    self.fizzle = Some((tick, x, foot_y));
                }
                CombatEvent::BlinkCancelled {
                    tick, hero: DONUT, ..
                } => self.cancel = Some(tick),
                _ => {}
            }
        }
    }

    /// The tick the press at `start` ended (arrival, fizzle or cancel), if it has.
    fn press_end(&self, start: u64) -> Option<u64> {
        [
            self.arrival.map(|a| a.0),
            self.fizzle.map(|f| f.0),
            self.cancel,
        ]
        .into_iter()
        .flatten()
        .filter(|t| *t >= start)
        .min()
    }

    /// The cues on `tick`.
    pub fn view(&self, tick: u64) -> BlinkCueView {
        let mut view = BlinkCueView::default();
        // The origin puddle: it opens on the press, then closes after she has
        // gone (arrival or cancel) or dries up (fizzle, below).
        if let Some((start, x, foot_y)) = self.start
            && let Some(age) = tick.checked_sub(start)
        {
            let end = self.press_end(start);
            let frame = match end {
                Some(end) if tick >= end => {
                    // After the arrival or a cancel: ring_close from the end tick.
                    let closing = self.fizzle.is_none_or(|f| f.0 != end);
                    closing
                        .then(|| puddle_vfx_frame(PuddleVfx::RingClose, tick - end))
                        .flatten()
                }
                // Still in the windup (the resolution event lands on the arrival tick):
                // ring_open's first two frames, 3 ticks each.
                None if age <= ARRIVE_AGE => {
                    puddle_vfx_frame(PuddleVfx::RingOpen, age.min(ARRIVE_AGE - 1))
                }
                Some(_) => puddle_vfx_frame(PuddleVfx::RingOpen, age.min(ARRIVE_AGE - 1)),
                None => None,
            };
            if let Some(frame) = frame {
                view.puddles.push(PuddleCue { x, foot_y, frame });
            }
        }
        if let Some((start, from_x, to_x, foot_y)) = self.arrival
            && let Some(age) = tick.checked_sub(start)
        {
            if let Some(frame) = puddle_vfx_frame(PuddleVfx::RingOpen, age) {
                view.puddles.push(PuddleCue {
                    x: to_x,
                    foot_y,
                    frame,
                });
            }
            view.arrive = arrive_frame(ARRIVE_AGE + age);
            if age < u64::from(DonutProgTuning::active().dpq34_streak_frames) {
                view.streak = Some(Streak {
                    from_x,
                    to_x,
                    foot_y,
                });
            }
        }
        if let Some((start, x, foot_y)) = self.fizzle
            && let Some(age) = tick.checked_sub(start)
            && age < FIZZLE_TICKS
        {
            if let Some(frame) = puddle_vfx_frame(PuddleVfx::Fizzle, age) {
                view.puddles.push(PuddleCue { x, foot_y, frame });
            }
            view.fizzle = fizzle_frame(age);
        }
        view
    }
}

#[cfg(test)]
mod tests;
