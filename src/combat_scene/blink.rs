//! DONUT-PROG-01 DP-C: Donut's short blink in the combat scene (spec DP-Q20 to
//! DP-Q32). The pure rules are [`crate::short_blink`]; this file feeds them the
//! press, the world and the hurt, and turns the outcomes into events, the
//! `BodyDrive::warp_x` of the arrival tick and the mana spend.
//!
//! It runs inside [`CombatState::step_donut`], after Donut's mana step and the
//! press edges and before her moves, so a windup-cancelling press is then
//! handled by her normal step on the same tick exactly as if no blink had
//! started (DP-Q25). Nothing runs while the hero does not hold
//! `special_short_blink` (a press without it is inert and silent), so every run
//! without a Donut program is untouched.

use super::{CombatEvent, CombatState, HeroId, InputFrame, SceneSim};
use crate::companion::navfollow::hazards_from_feed;
use crate::donut_prog::{DonutProgTuning, SPECIAL_SHORT_BLINK};
use crate::prototype::Rect;
use crate::short_blink::{
    Arrival, BLINK_HERO, CancelCause, PressView, SearchWorld, direction, resolve_arrival, search,
};

/// What Donut's blink decided for this tick's step.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct BlinkTick {
    /// The windup (t0..t5) or the arrival tick (t6): horizontal input is
    /// ignored and `vx` is 0, gravity still acts (DP-Q25, DP-Q30).
    pub windup: bool,
    /// t0: the jump buffer is cleared (DP-Q25).
    pub clear_buffer: bool,
    /// The arrival: the column the body is set to (DP-Q30).
    pub warp: Option<f32>,
    /// The windup's last tick, the arrival and the recovery ignore the primary
    /// and physical presses: not buffered (DP-Q25).
    pub mask_presses: bool,
}

impl CombatState {
    /// The destination search from the scene's state now. Builds the world the
    /// rules read (solids, barriers, unfired zones, hazards, the simulation
    /// camera, the partner) and runs the pure search; only a press or an
    /// arrival calls it.
    fn blink_search(&self, scene: &SceneSim, x: f32, y: f32, dir: i8) -> Option<f32> {
        let zones: Vec<(f32, f32)> = self
            .story
            .as_ref()
            .map(|story| {
                story
                    .unfired_zones(BLINK_HERO)
                    .iter()
                    .map(|zone| (zone.min_x, zone.max_x))
                    .collect()
            })
            .unwrap_or_default();
        // The same hero-hurting set the companion's nav graph is fed (CA2-C).
        let hazards: Vec<Rect> = self
            .env
            .as_ref()
            .map(|env| {
                hazards_from_feed(&env.nav_feed(), scene.world.layout_height())
                    .into_iter()
                    .map(|hazard| hazard.rect)
                    .collect()
            })
            .unwrap_or_default();
        let view = scene.view_x();
        let drive = scene.drive();
        let partner = drive.two_player.then(|| {
            let view_w = (view.1 - view.0) as i32;
            (
                scene.world.hero.x,
                drive.tuning.max_separation_px(view_w) as f32,
            )
        });
        let world = SearchWorld {
            solids: &scene.world.solids,
            barriers: &self.blink_barriers,
            zones: &zones,
            hazards: &hazards,
            x_max: self.x_max - crate::prototype::HERO_HALF_WIDTH,
            view,
            partner,
        };
        search(&world, x, y, dir, DonutProgTuning::active())
    }

    /// One tick of Donut's blink (DP-Q20 to DP-Q32). `can_act` is the legacy
    /// gate (not hurt, not defeated, not frozen), `hurt` the hurt or defeated
    /// flag alone, `busy` a swat, pounce, cast or hop running, and `may_press`
    /// whether this controller may start a blink (a human or the companion's
    /// injected press, never a bare bot).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn step_blink(
        &mut self,
        scene: &SceneSim,
        frame: &InputFrame,
        can_act: bool,
        hurt: bool,
        busy: bool,
        may_press: bool,
        tick: u64,
    ) -> BlinkTick {
        let mut out = BlinkTick::default();
        if !self.story_holds(SPECIAL_SHORT_BLINK) {
            return out;
        }
        let t = DonutProgTuning::active();
        let cost = t.dpq23_cost_charges;
        let (x, y, facing) = {
            let body = scene.world.body(HeroId::Donut);
            (body.x, body.y, body.facing)
        };

        self.donut_kit.blink.tick_over(tick);
        if self.donut_kit.blink.take_ready(tick) {
            self.log(CombatEvent::SpecialReady {
                tick,
                hero: BLINK_HERO,
            });
        }
        // A press is judged only if nothing of the blink was running when the
        // tick began (a press in the windup or the recovery is ignored).
        let idle_at_start =
            self.donut_kit.blink.windup().is_none() && !self.donut_kit.blink.recovering(tick);
        let input_press = frame.attack_pressed || frame.physical_pressed || frame.jump_pressed;

        // Cancels: a hurt in the windup (t0..t6), else a primary, physical or
        // jump press on t0..t5 (DP-Q25, DP-Q29). Never a cost.
        if let Some(age) = self.donut_kit.blink.windup_age(tick) {
            let cause = if hurt {
                Some(CancelCause::Hurt)
            } else if input_press && age < u64::from(t.dpq25_windup_ticks) {
                Some(CancelCause::Input)
            } else {
                None
            };
            if let Some(cause) = cause {
                self.donut_kit.blink.cancel();
                self.log(CombatEvent::BlinkCancelled {
                    tick,
                    hero: BLINK_HERO,
                    cause,
                });
            }
        }

        // The arrival: the mana check first, then the search again from her
        // current position in the locked direction (DP-Q28).
        let mut arrived_now = false;
        if self.donut_kit.blink.arrival_due(tick, t.dpq25_windup_ticks)
            && let Some((_, dir)) = self.donut_kit.blink.windup()
        {
            let charges = self.donut_kit.mana.mana();
            match resolve_arrival(charges, cost, || self.blink_search(scene, x, y, dir)) {
                Arrival::Resolved(to) => {
                    if let Some(spent) = self.donut_kit.mana.spend(cost, tick) {
                        self.log(CombatEvent::Mana(spent));
                    }
                    self.donut_kit.blink.arrived(tick, t);
                    out.warp = Some(to);
                    arrived_now = true;
                    self.log(CombatEvent::BlinkResolved {
                        tick,
                        hero: BLINK_HERO,
                        from_x: x.round() as i32,
                        to_x: to.round() as i32,
                    });
                }
                Arrival::Fizzled(reason) => {
                    self.donut_kit.blink.fizzled();
                    self.log(CombatEvent::BlinkFizzled {
                        tick,
                        hero: BLINK_HERO,
                        reason,
                    });
                }
            }
        }

        // The press edge (DP-Q20, DP-Q26).
        if idle_at_start && frame.ability_pressed && may_press {
            let dir = direction(frame.held.move_x, facing, t.dpq21_direction_deadzone);
            let view = PressView {
                can_act,
                busy,
                charges: self.donut_kit.mana.mana(),
            };
            let gate = self
                .donut_kit
                .blink
                .press_gate(tick, &view, cost, || self.blink_search(scene, x, y, dir));
            match gate {
                Ok(()) => {
                    self.log(CombatEvent::SpecialStarted {
                        tick,
                        hero: BLINK_HERO,
                        capability: SPECIAL_SHORT_BLINK.as_str(),
                    });
                    if input_press {
                        // A press on the same tick cancels it at once: no windup,
                        // the press is handled by her normal step below.
                        self.log(CombatEvent::BlinkCancelled {
                            tick,
                            hero: BLINK_HERO,
                            cause: CancelCause::Input,
                        });
                    } else {
                        self.donut_kit.blink.begin(tick, dir);
                        out.clear_buffer = true;
                        self.donut_kit.clear_press_buffers();
                    }
                }
                Err(reason) => self.log(CombatEvent::SpecialRejected {
                    tick,
                    hero: BLINK_HERO,
                    reason,
                }),
            }
        }

        let running = self.donut_kit.blink.windup().is_some();
        out.windup = running || arrived_now;
        out.mask_presses = out.windup || self.donut_kit.blink.recovering(tick);
        out
    }
}

impl CombatState {
    /// Donut's blink state, read-only (the presentation and the tests).
    pub fn donut_blink(&self) -> &crate::short_blink::Blink {
        &self.donut_kit.blink
    }

    /// Tests only: hurt Donut as a goblin's blow would (flinch, hit-stop,
    /// damage), on `tick`. No invulnerability check is made here.
    #[doc(hidden)]
    pub fn test_hurt_donut(&mut self, scene: &mut SceneSim, tick: u64) -> bool {
        let source_x = scene.world.body(HeroId::Donut).x + 1.0;
        self.hurt_hero_by(scene, HeroId::Donut, 0xb11, source_x, tick)
    }

    /// Tests only: spend Donut's charges down to `to` (the regen timer restarts
    /// as a release does).
    #[doc(hidden)]
    pub fn test_drain_donut_charges(&mut self, to: u8) {
        let have = self.donut_kit.mana.mana();
        if have > to {
            let _ = self.donut_kit.mana.spend(have - to, 0);
        }
    }
}
