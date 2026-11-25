//! DONUT-MOBILITY-01 DM-C: Donut's reflex cues (spec section 3.4, DM-Q21), the
//! pure half. The ready tell (a tail glint while armed, a tail flick on arming),
//! the evade response (a cosmetic arc, five evade frames, a swish) and the
//! hand-off to the existing CF02-I `smug` overlay.
//!
//! **Presentation only.** [`DonutCues::step`] reads a `&CombatState` and writes
//! nothing back; no cue is hashed, replayed or read by the simulation, so a run
//! with the cues drawn and a run without them have the same state hash (a test
//! below). The Bevy binary only draws what [`DonutCues`] reports, on existing
//! approved Donut pixels and a handful of code-drawn rectangles: **no new art**
//! (the production frames are a later DM-ART order).
//!
//! Every cue is a function of ticks recorded from the event log (the event
//! `tick`, not the frame it was noticed on), so a render that skips frames or a
//! replay viewer that seeks draws the same picture for the same tick.
//!
//! Numbers are the spec's (section 3.4) unless a deviation is named; the
//! deviations are adopted defaults recorded in spec section 11 (`DM-Q21/C1` to
//! `DM-Q21/C4`) and the owner may retune them after playing.

use crate::combat::CombatEvent;
use crate::combat_scene::CombatState;
use crate::party::HeroId;

/// Glint blink: on for this many ticks, then off for [`GLINT_OFF_TICKS`] (a
/// 30 tick cycle, 2 Hz). Spec 3.4. The glint is drawn from the Puddle Jumper strip
/// (`donut_art::puddle_vfx_frame(PuddleVfx::Glint, ..)`): a 3 x 3 plus, with the
/// small twinkle on the 4th on-tick (DMA-Q05).
pub const GLINT_ON_TICKS: u64 = 4;
pub const GLINT_OFF_TICKS: u64 = 26;
/// The tail flick on arming lasts this many ticks (spec 3.4: 6): the
/// `flick_up` tail for the first [`FLICK_PHASE_TICKS`], the `flick_snap` tail for
/// the next [`FLICK_PHASE_TICKS`], then the base locomotion frame for the rest.
pub const FLICK_TICKS: u64 = 6;
pub const FLICK_PHASE_TICKS: u64 = 2;
/// The evade response lasts this many ticks (spec 3.4: 12).
pub const EVADE_TICKS: u64 = 12;
/// The cosmetic arc rises this many pixels over her authoritative position.
pub const ARC_PEAK_PX: i32 = 10;
/// DM-Q21/C3: the swish stays this many ticks (`swish_a` then `swish_b`). The spec
/// says "one frame"; two ticks survive a 30 fps capture and read at gameplay scale.
pub const STREAK_TICKS: u64 = 2;

/// The cosmetic height of the evade arc `age` ticks after the evade: a parabola
/// from 0 at the start, [`ARC_PEAK_PX`] at the middle, 0 again at
/// [`EVADE_TICKS`]; whole pixels (pixel art).
pub fn arc_lift_px(age: u64) -> i32 {
    if age >= EVADE_TICKS {
        return 0;
    }
    let (a, n) = (age as f32, EVADE_TICKS as f32);
    (4.0 * ARC_PEAK_PX as f32 * a * (n - a) / (n * n)).round() as i32
}

/// What the evade response looks like on one tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvadeView {
    /// Ticks since the evade (0 on the tick of the event).
    pub age: u64,
    /// Cosmetic lift of her sprite, in pixels.
    pub lift_px: i32,
    /// The swish, while it lasts: the side the attack came from
    /// (-1 left, +1 right, 0 unknown).
    pub streak: Option<i8>,
}

/// The pure cue state: what the event log said, as ticks.
#[derive(Debug, Clone, Default)]
pub struct DonutCues {
    seen_events: usize,
    last_tick: u64,
    armed: bool,
    /// Tick the current armed spell began (the event tick, else the tick it was
    /// first seen armed).
    armed_since: Option<u64>,
    flick_start: Option<u64>,
    evade_start: Option<u64>,
    evade_side: i8,
}

impl DonutCues {
    /// Back to nothing (a new attempt, a replay seek backwards).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Advance to `tick` from the finished simulation state. Read-only: `combat`
    /// is never written. `donut_x` is her body column, for the streak's side.
    pub fn step(&mut self, tick: u64, combat: &CombatState, donut_x: f32) {
        let total = combat.dropped_events + combat.events.len();
        if tick < self.last_tick || total < self.seen_events {
            // A restart or a seek back: nothing of the last attempt stays.
            self.reset();
        }
        self.last_tick = tick;
        let fresh = total
            .saturating_sub(self.seen_events)
            .min(combat.events.len());
        self.seen_events = total;
        let events = &combat.events[combat.events.len() - fresh..];
        for event in events {
            match *event {
                CombatEvent::EvadeArmed {
                    tick,
                    hero: HeroId::Donut,
                } => {
                    self.flick_start = Some(tick);
                    self.armed_since = Some(tick);
                }
                CombatEvent::HeroEvaded {
                    tick,
                    hero: HeroId::Donut,
                    source,
                    kind,
                } => {
                    self.evade_start = Some(tick);
                    self.evade_side = attacker_side(combat, source, kind, donut_x);
                }
                _ => {}
            }
        }
        let armed = combat.reflex().armed();
        if armed {
            self.armed_since.get_or_insert(tick);
        } else {
            self.armed_since = None;
        }
        self.armed = armed;
    }

    /// Whether the tail glint is lit on `tick`: armed, in the 4-on part of the
    /// 30 tick cycle that began when the armed spell did.
    pub fn glint_on(&self, tick: u64) -> bool {
        self.glint_age(tick).is_some()
    }

    /// The tick inside the 4-on window of the glint on `tick` (0 to 3), or `None`
    /// while it is dark. The 4th on-tick is the small twinkle (DMA-Q05).
    pub fn glint_age(&self, tick: u64) -> Option<u64> {
        if !self.armed {
            return None;
        }
        let since = self.armed_since?;
        let at = tick.saturating_sub(since) % (GLINT_ON_TICKS + GLINT_OFF_TICKS);
        (at < GLINT_ON_TICKS).then_some(at)
    }

    /// The tail flick on `tick` as a tail-swap phase: `Some(0)` (`flick_up`) for the
    /// first two ticks, `Some(1)` (`flick_snap`) for the next two, `None` from the
    /// fifth tick on (the base locomotion frame; [`FLICK_TICKS`] stays 6).
    pub fn flick(&self, tick: u64) -> Option<usize> {
        let age = tick.checked_sub(self.flick_start?)?;
        (age < 2 * FLICK_PHASE_TICKS).then_some((age / FLICK_PHASE_TICKS) as usize)
    }

    /// The evade response on `tick`, or `None` outside its [`EVADE_TICKS`].
    pub fn evade(&self, tick: u64) -> Option<EvadeView> {
        let age = tick.checked_sub(self.evade_start?)?;
        if age >= EVADE_TICKS {
            return None;
        }
        Some(EvadeView {
            age,
            lift_px: arc_lift_px(age),
            streak: (age < STREAK_TICKS).then_some(self.evade_side),
        })
    }

    /// Whether any cue wants her locomotion sprite replaced on `tick` (the evade
    /// arc draws her itself).
    pub fn replaces_locomotion(&self, tick: u64) -> bool {
        self.evade(tick).is_some()
    }
}

/// The side the attack came from, relative to her column: -1 left, +1 right.
fn attacker_side(
    combat: &CombatState,
    source: u32,
    kind: crate::agility::EvadeSource,
    donut_x: f32,
) -> i8 {
    use crate::agility::EvadeSource;
    let x = match kind {
        EvadeSource::GoblinSwing | EvadeSource::EngineerRam => {
            combat.enemies.get(source as usize).map(|g| g.x)
        }
        EvadeSource::LlamaGlob => combat.globs.iter().find(|g| g.owner == source).map(|g| g.x),
        // HAZ-B: the slime's lunge; `source` is its slot target id.
        EvadeSource::SlimeLunge => combat
            .slot_enemies()
            .iter()
            .find(|e| e.target_id() == source)
            .and_then(|e| e.as_any())
            .and_then(|a| a.downcast_ref::<crate::combat_scene::slime::SlimeEnemy>())
            .map(|s| s.slime().x),
    };
    match x {
        Some(x) if x < donut_x => -1,
        Some(x) if x > donut_x => 1,
        _ => 0,
    }
}

// ---------------------------------------------------------------- atlas ----

/// One opaque pixel of a frame: `(x, y, rgba)` with `y` from the top.
pub type FramePixel = (u32, u32, [u8; 4]);

/// The tail tip of one locomotion frame and the opaque pixels around it (the
/// tip and its 8-neighbourhood), which the flick shifts by one pixel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TailTip {
    pub x: u32,
    pub y: u32,
    pub cluster: Vec<FramePixel>,
}

/// What the binary reads once from Donut's locomotion atlas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasCues {
    /// Per frame, in atlas order; `None` for an empty frame.
    pub tips: Vec<Option<TailTip>>,
    /// The brightest opaque colour of the atlas (the palette's highlight), the
    /// glint's colour. DM-Q21/C1.
    pub highlight: [u8; 3],
}

const OPAQUE: u8 = 128;

fn luma(c: [u8; 4]) -> u32 {
    u32::from(c[0]) * 299 + u32::from(c[1]) * 587 + u32::from(c[2]) * 114
}

/// Find each frame's tail tip and the palette highlight in an RGBA8 atlas of
/// `frames` frames of `frame_w` x `frame_h`, laid out in one row. Her art faces
/// right, so the tail trails on the left and is the farthest-back part of every
/// frame (the ears and the tiara are higher but further forward): the tip is the
/// **topmost** opaque pixel of the two leftmost opaque columns. A pure function
/// of the pixels. DM-Q21/C1.
pub fn analyze_atlas(
    rgba: &[u8],
    atlas_w: u32,
    frame_w: u32,
    frame_h: u32,
    frames: usize,
) -> AtlasCues {
    let pixel = |frame: usize, x: u32, y: u32| -> [u8; 4] {
        let ax = frame as u32 * frame_w + x;
        let at = ((y * atlas_w + ax) * 4) as usize;
        rgba.get(at..at + 4)
            .map_or([0; 4], |p| [p[0], p[1], p[2], p[3]])
    };
    let mut highlight = ([0_u8; 3], 0_u32);
    let mut tips = Vec::with_capacity(frames);
    for frame in 0..frames {
        let mut min_x = u32::MAX;
        for y in 0..frame_h {
            for x in 0..frame_w {
                let c = pixel(frame, x, y);
                if c[3] >= OPAQUE {
                    min_x = min_x.min(x);
                    let l = luma(c);
                    if l > highlight.1 {
                        highlight = ([c[0], c[1], c[2]], l);
                    }
                }
            }
        }
        if min_x == u32::MAX {
            tips.push(None);
            continue;
        }
        let limit = (min_x + 2).min(frame_w);
        let tip = (0..frame_h)
            .flat_map(|y| (min_x..limit).map(move |x| (x, y)))
            .find(|&(x, y)| pixel(frame, x, y)[3] >= OPAQUE);
        let Some((tx, ty)) = tip else {
            tips.push(None);
            continue;
        };
        let mut cluster = Vec::new();
        for y in ty.saturating_sub(1)..=(ty + 1).min(frame_h - 1) {
            for x in tx.saturating_sub(1)..=(tx + 1).min(frame_w - 1) {
                let c = pixel(frame, x, y);
                if c[3] >= OPAQUE {
                    cluster.push((x, y, c));
                }
            }
        }
        tips.push(Some(TailTip {
            x: tx,
            y: ty,
            cluster,
        }));
    }
    AtlasCues {
        tips,
        highlight: highlight.0,
    }
}

#[cfg(test)]
mod tests;
