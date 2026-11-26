//! AREA-HAZARDS-01B: the acid **entry splash**, the pure half
//! (`docs/02-art/area-hazards-01b-spec.md` sections 3.1 and 4.1). A short, ugly slop
//! of teal liquid thrown up around the boots of whoever enters an acid pool.
//!
//! Read-only from the event log, in the shape of [`crate::blink_cues`]:
//! [`AcidSplashes::step`] scans only the events that are new since its last call
//! (`dropped_events + events.len()` against its own `seen_events`), resets on a
//! backward seek, and [`AcidSplashes::view`] is a pure function of what the log said
//! and the tick.
//!
//! **Triggers** (HZB-AD3, existing hurt events only: the simulation has no entry tick
//! and gets none):
//!
//! - [`CombatEvent::HeroHurt`] whose `source & 0xFFFF` is [`ACID_SOURCE_ID`] (the
//!   patch is `source >> 16`);
//! - [`CombatEvent::EnvironmentHurt`] with class [`HazardClass::Acid`] (a goblin that
//!   landed in a patch and lived; the event exists only under `fu.acid_hurt_event`);
//! - [`CombatEvent::EnvironmentKill`] with class [`HazardClass::Acid`] (a lethal
//!   landing: the defeat event itself carries the acid class and the patch, so a
//!   goblin killed by the pool splashes too, whatever the flag says).
//!
//! An invulnerable entry produces no event and therefore no splash.
//!
//! **Placement.** The splash sits at the hurt actor's x at the event, clamped to
//! `[left + 6, right - 6]` of the patch's **live** rectangle (a drained patch has none,
//! so it draws nothing), mirrored when that x is below the patch's centre (the sign of
//! the simulation's pop), with rows 16..23 (the displaced lip and the trough) cropped
//! to the live rectangle so the lip never spills onto dry floor. At most [`SLOTS`]
//! splashes play; a repeat for the same actor restarts its splash, a full set replaces
//! the oldest, and two events of the same tick, patch and actor make one splash.
//!
//! **Presentation only.** Nothing here mutates a state, draws a random number or is
//! hashed, replayed or read back by the simulation: a run that steps and views the
//! cues each tick has the state hash of one that never does (a test).
//!
//! Coordinates are world px, y up, like the simulation.

use crate::combat::CombatEvent;
use crate::combat_scene::{CombatState, EnvRuntime};
use crate::danger_fx_art::{
    FxDraw, Layer, SPLASH_H, SPLASH_KEY_FRAME, SPLASH_TICKS, SPLASH_W, Sheet, splash_frame,
};
use crate::env_hazard::HazardClass;
use crate::party::HeroId;
use crate::slime::{ACID_SOURCE_ID, AcidPatch};

/// How many splashes play at once.
pub const SLOTS: usize = 4;
/// The splash is clamped this far inside the live rectangle's ends, px.
pub const CLAMP_MARGIN_PX: i32 = 6;
/// The canvas column on the walking line (the pivot).
pub const PIVOT_X: i32 = 16;
/// The first canvas row that is cropped to the live rectangle (the displaced lip).
pub const CROP_ROW: i32 = 16;
/// The whole splash lasts this many ticks (2 + 3 + 3 + 4 + 4).
pub const TOTAL_TICKS: u64 =
    SPLASH_TICKS[0] + SPLASH_TICKS[1] + SPLASH_TICKS[2] + SPLASH_TICKS[3] + SPLASH_TICKS[4];
/// Sprites per splash: the uncropped crown and the cropped lip.
pub const SPRITES_PER_SPLASH: usize = 2;

/// Who splashed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actor {
    Hero(HeroId),
    /// A goblin or Engineer, by target id (its index in the combat state's roster).
    Enemy(u32),
}

/// One splash slot: where and when (render-local, never hashed).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Slot {
    start: u64,
    patch: u16,
    actor: Actor,
    /// The actor's x at the event (world px).
    x: f32,
}

/// One splash to draw: its frame, mirroring, pivot column and walking line, and the
/// live x range of its pool (the crop of the lip).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplashDraw {
    pub frame: usize,
    pub flip_x: bool,
    /// The world column of the canvas pivot (clamped inside the live pool).
    pub x: i32,
    /// The world row just above the patch's walking line (the pivot row).
    pub floor: i32,
    /// The live pool's x range `left..right`.
    pub live: (i32, i32),
}

impl SplashDraw {
    /// The world column of canvas column 0: a mirrored canvas flips about the pivot
    /// column, which then lands on the canvas' column `W - 1 - PIVOT_X`.
    pub const fn left(&self) -> i32 {
        if self.flip_x {
            self.x - (SPLASH_W - 1 - PIVOT_X)
        } else {
            self.x - PIVOT_X
        }
    }

    /// The sprites: the crown (rows 0..15, not cropped) and the lip (rows 16..23,
    /// cropped to the live pool). A mirrored sprite takes its crop from the
    /// unmirrored cell.
    pub fn sprites(&self) -> Vec<FxDraw> {
        let left = self.left();
        let mut out = Vec::with_capacity(SPRITES_PER_SPLASH);
        let crown_h = CROP_ROW;
        out.push(FxDraw {
            sheet: Sheet::Splash,
            frame: self.frame,
            src_x: 0,
            src_y: 0,
            flip_x: self.flip_x,
            x: left,
            y: self.floor + (SPLASH_H - crown_h),
            w: SPLASH_W,
            h: crown_h,
            layer: Layer::Splash,
        });
        // The displayed columns a..b of the lip that fall inside the live pool.
        let a = (self.live.0 - left).clamp(0, SPLASH_W);
        let b = (self.live.1 - left).clamp(0, SPLASH_W);
        if b > a {
            let src_x = if self.flip_x { SPLASH_W - b } else { a };
            out.push(FxDraw {
                sheet: Sheet::Splash,
                frame: self.frame,
                src_x,
                src_y: CROP_ROW,
                flip_x: self.flip_x,
                x: left + a,
                y: self.floor,
                w: b - a,
                h: SPLASH_H - CROP_ROW,
                layer: Layer::Splash,
            });
        }
        out
    }
}

/// The pure cue state: what the event log said, as ticks, patches and columns.
#[derive(Debug, Clone, Default)]
pub struct AcidSplashes {
    seen_events: usize,
    last_tick: u64,
    slots: Vec<Slot>,
}

impl AcidSplashes {
    /// Back to nothing (a new attempt, a replay seek backwards).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Advance to `tick` from the finished simulation state. Read-only. `hero_x` is
    /// the body column of a hero on this tick (world px).
    pub fn step(&mut self, tick: u64, combat: &CombatState, hero_x: impl Fn(HeroId) -> f32) {
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
                CombatEvent::HeroHurt {
                    tick, source, hero, ..
                } if source & 0xFFFF == ACID_SOURCE_ID => {
                    self.trigger(tick, (source >> 16) as u16, Actor::Hero(hero), hero_x(hero));
                }
                CombatEvent::EnvironmentHurt {
                    tick,
                    target,
                    prop,
                    class: HazardClass::Acid,
                    ..
                }
                | CombatEvent::EnvironmentKill {
                    tick,
                    target,
                    prop,
                    class: HazardClass::Acid,
                    ..
                } => {
                    if let Some(g) = combat.enemies.get(target as usize) {
                        self.trigger(tick, prop, Actor::Enemy(target), g.x);
                    }
                }
                _ => {}
            }
        }
    }

    fn trigger(&mut self, tick: u64, patch: u16, actor: Actor, x: f32) {
        let slot = Slot {
            start: tick,
            patch,
            actor,
            x,
        };
        // Splashes that are over free their slot.
        self.slots
            .retain(|s| tick.saturating_sub(s.start) < TOTAL_TICKS);
        if let Some(same) = self.slots.iter_mut().find(|s| s.actor == actor) {
            // The same actor: one splash for a tick and patch, otherwise a restart.
            if !(same.start == tick && same.patch == patch) {
                *same = slot;
            }
        } else if self.slots.len() < SLOTS {
            self.slots.push(slot);
        } else if let Some(oldest) = self.slots.iter_mut().min_by_key(|s| s.start) {
            *oldest = slot;
        }
    }

    /// The splashes on `tick`: at most [`SLOTS`], in slot order. `layout_h` flips the
    /// room's layout rows into world rows.
    pub fn view(&self, tick: u64, env: &EnvRuntime, layout_h: i32) -> Vec<SplashDraw> {
        let mut out = Vec::with_capacity(SLOTS);
        for slot in &self.slots {
            let Some(age) = tick.checked_sub(slot.start) else {
                continue;
            };
            let Some(frame) = splash_frame(age) else {
                continue;
            };
            let Some(patch) = env.acid().get(slot.patch) else {
                continue;
            };
            if let Some(draw) = place(patch, slot.x, frame, layout_h) {
                out.push(draw);
            }
        }
        out
    }
}

/// The splash of an actor at `x` on `patch` at `frame`: `None` for a drained patch.
fn place(patch: &AcidPatch, x: f32, frame: usize, layout_h: i32) -> Option<SplashDraw> {
    let rect = patch.rect()?;
    let (left, right) = (rect.x, rect.right());
    let (lo, hi) = (left + CLAMP_MARGIN_PX, right - CLAMP_MARGIN_PX);
    let at = x.round() as i32;
    let col = if lo <= hi {
        at.clamp(lo, hi)
    } else {
        (left + right) / 2
    };
    Some(SplashDraw {
        frame,
        flip_x: x < patch.centre_x(),
        x: col,
        floor: layout_h - patch.spec().floor_y,
        live: (left, right),
    })
}

/// The frame the key (the crown) shows, for the tests and the evidence.
pub const KEY_FRAME: usize = SPLASH_KEY_FRAME;

#[cfg(test)]
mod tests;
