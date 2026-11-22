//! DONUT-PROG-01 DP-D (DP-Q34): the stand-in cues of Donut's Puddle Jumper, the
//! pure half. Read-only from the event log, in the shape of
//! [`crate::donut_cues`]:
//!
//! - on `SpecialStarted` a 3-ring ripple at her feet (and the 6-tick crouch,
//!   which the presenter draws from the blink state with
//!   [`crate::donut_art::blink_crouch_frame`]);
//! - on `BlinkResolved` the same ripple at the destination and a dotted streak
//!   between the two points, for [`STREAK_TICKS`] ticks (the data's
//!   `dpq34_streak_frames`);
//! - on `BlinkFizzled` a single shrinking ring.
//!
//! The colours are the Magic Missile VFX palette (`vfx_magic_missile.grid.json`).
//! No aura, no glow on her body, no screen shake. **Presentation only**: nothing
//! here is hashed, replayed or read by the simulation, so a run with the cues
//! drawn and one without have the same state hash (a test in
//! `donut_prog_fixtures`). Every cue is a function of event ticks, so a render
//! that skips frames or a replay seek draws the same picture for the same tick.
//!
//! The numbers below (ring spacing, radii, the fizzle ring's length) are adopted
//! defaults of DP-Q34's stand-ins, recorded in spec section 16; the owner may
//! retune them after playing.

use crate::combat::CombatEvent;
use crate::combat_scene::CombatState;
use crate::donut_prog::DonutProgTuning;

/// The hero index of Donut in the event log.
const DONUT: u8 = 1;

/// A ripple's rings start this many ticks apart.
pub const RING_STAGGER_TICKS: u64 = 3;
/// One ring lives this many ticks.
pub const RING_LIFE_TICKS: u64 = 9;
/// A ring starts at this half width and grows by [`RING_GROW_PX`] per tick.
pub const RING_START_RX: i32 = 4;
pub const RING_GROW_PX: i32 = 2;
/// The fizzle ring starts at this half width, shrinks by [`RING_GROW_PX`] per tick
/// and lives [`FIZZLE_TICKS`] ticks.
pub const FIZZLE_START_RX: i32 = 20;
pub const FIZZLE_TICKS: u64 = 8;
/// The streak: a 2 x 1 dot every this many pixels.
pub const STREAK_DOT_STEP_PX: i32 = 6;
pub const STREAK_DOT_W_PX: i32 = 2;
/// The streak lies this many pixels above her feet (across her body, low).
pub const STREAK_LIFT_PX: i32 = 8;

/// The Magic Missile palette (`magenta_deep`, `magenta`, `pink_hot`, `core_pale`).
pub const PALETTE: [[u8; 3]; 4] = [
    [0x7a, 0x1a, 0x63],
    [0xe2, 0x30, 0x9f],
    [0xff, 0x7f, 0xd0],
    [0xff, 0xd8, 0xf4],
];

/// One ring: its centre column and foot row (world px, y up), its half width and
/// the palette entry it is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ring {
    pub x: i32,
    pub foot_y: i32,
    pub rx: i32,
    pub ry: i32,
    pub tone: usize,
}

/// The dotted streak between two columns on one foot row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Streak {
    pub from_x: i32,
    pub to_x: i32,
    pub foot_y: i32,
}

impl Streak {
    /// The left column of each dot, from the start to the end point (inclusive of
    /// the start), spaced [`STREAK_DOT_STEP_PX`].
    pub fn dots(&self) -> Vec<i32> {
        let dir = (self.to_x - self.from_x).signum();
        if dir == 0 {
            return Vec::new();
        }
        let n = (self.to_x - self.from_x).abs() / STREAK_DOT_STEP_PX;
        (0..=n)
            .map(|i| self.from_x + dir * i * STREAK_DOT_STEP_PX)
            .collect()
    }

    pub const fn y(&self) -> i32 {
        self.foot_y + STREAK_LIFT_PX
    }
}

/// The half height of a ring of half width `rx` (a flat ellipse, as the ground
/// reads it): a third, at least one row.
pub const fn ring_ry(rx: i32) -> i32 {
    let ry = rx / 3;
    if ry < 1 { 1 } else { ry }
}

/// The ring `index` of a ripple `age` ticks after its start, if it is alive.
fn ripple_ring(index: u64, age: u64) -> Option<(i32, usize)> {
    let started = index * RING_STAGGER_TICKS;
    let age = age.checked_sub(started)?;
    if age >= RING_LIFE_TICKS {
        return None;
    }
    let rx = RING_START_RX + RING_GROW_PX * age as i32;
    // Bright first, then fading to the deep magenta.
    let tone = match age {
        0..=2 => 3,
        3..=5 => 2,
        6..=7 => 1,
        _ => 0,
    };
    Some((rx, tone))
}

/// The outline of an ellipse of half width `rx` and half height `ry` as pixel
/// rows: for each `dy` in `-ry..=ry` the inclusive `(from, to)` offsets of the
/// right half (`0 <= from <= to`); the left half mirrors it. Rows join: each row
/// reaches from the extent of the row nearer the ends to its own extent, and the
/// top and bottom rows run from the middle.
pub fn ellipse_spans(rx: i32, ry: i32) -> Vec<(i32, i32, i32)> {
    let extent = |dy: i32| -> i32 {
        let t = f64::from(dy.abs()) / f64::from(ry.max(1));
        (f64::from(rx) * (1.0 - t * t).max(0.0).sqrt()).round() as i32
    };
    (-ry..=ry)
        .map(|dy| {
            let outer = extent(dy);
            let inner = if dy.abs() == ry {
                0
            } else {
                extent(dy.abs() + 1).min(outer)
            };
            (dy, inner, outer)
        })
        .collect()
}

/// What the cues look like on one tick.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlinkCueView {
    pub rings: Vec<Ring>,
    pub streak: Option<Streak>,
}

/// The pure cue state: what the event log said, as ticks and columns.
#[derive(Debug, Clone, Default)]
pub struct BlinkCues {
    seen_events: usize,
    last_tick: u64,
    /// The ripple at her feet: (tick, column, foot row).
    start: Option<(u64, i32, i32)>,
    /// The arrival: (tick, from column, to column, foot row).
    arrival: Option<(u64, i32, i32, i32)>,
    /// The fizzle: (tick, column, foot row).
    fizzle: Option<(u64, i32, i32)>,
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
                // A cancelled windup keeps the ripple already started: it is a
                // flourish of the press, not a promise of an arrival.
                _ => {}
            }
        }
    }

    /// The cues on `tick`.
    pub fn view(&self, tick: u64) -> BlinkCueView {
        let mut view = BlinkCueView::default();
        let rings = DonutProgTuning::active().dpq34_ripple_rings.into();
        let ripple = |at: Option<(u64, i32, i32)>, view: &mut BlinkCueView| {
            let Some((start, x, foot_y)) = at else { return };
            let Some(age) = tick.checked_sub(start) else {
                return;
            };
            for i in 0..rings {
                if let Some((rx, tone)) = ripple_ring(i, age) {
                    view.rings.push(Ring {
                        x,
                        foot_y,
                        rx,
                        ry: ring_ry(rx),
                        tone,
                    });
                }
            }
        };
        ripple(self.start, &mut view);
        ripple(self.arrival.map(|(t, _, to, y)| (t, to, y)), &mut view);
        if let Some((start, x, foot_y)) = self.fizzle
            && let Some(age) = tick.checked_sub(start)
            && age < FIZZLE_TICKS
        {
            let rx = FIZZLE_START_RX - RING_GROW_PX * age as i32;
            view.rings.push(Ring {
                x,
                foot_y,
                rx,
                ry: ring_ry(rx),
                tone: 1,
            });
        }
        if let Some((start, from_x, to_x, foot_y)) = self.arrival
            && let Some(age) = tick.checked_sub(start)
            && age < u64::from(DonutProgTuning::active().dpq34_streak_frames)
        {
            view.streak = Some(Streak {
                from_x,
                to_x,
                foot_y,
            });
        }
        view
    }
}

/// The most rows of spans one ring can need (the pool bound of the presenter):
/// the widest ring is the fizzle ring or the oldest ripple ring.
pub fn max_ring_spans() -> usize {
    let widest = FIZZLE_START_RX.max(RING_START_RX + RING_GROW_PX * (RING_LIFE_TICKS as i32 - 1));
    ellipse_spans(widest, ring_ry(widest)).len() * 2
}

#[cfg(test)]
mod tests;
