//! CF02-H: Carl's press handling under `cf02_rules` (Q46..Q56).
//!
//! The one place where a press becomes a move under `--cf02`: the move buffer,
//! the same-tick rule, the landing grace, the closed cancel table (C1..C5), the
//! air kick, the dive and the bail-out hop. The pure decisions live in
//! [`crate::input_buffer`]; this module owns the state they need (the buffer, the
//! airborne period) and drives Carl's body. With `cf02_rules` off nothing here
//! runs and the legacy press handling in `step_party` is untouched.
//!
//! Order within a tick: [`CombatState::plan_carl_cf02`] (before the move runner
//! steps) decides the request, the cancels, the bail-out and the landing recovery;
//! [`CombatState::carl_cf02_drive`] (after the strike) turns the running move and
//! the hop into the body drive of that tick and ends a dive on its first hit.

use super::CombatState;
use crate::cf02::{Cf02Tuning, StompBinding};
use crate::combat::{MoveId, MoveRunner, MoveSpec, MoveTick, carl_move, cf02_moves};
use crate::input::{Actions, RawInput};
use crate::input_buffer::{
    ActorCtx, AirRoute, AirState, BufferedPress, Button, CancelId, MoveBuffer, MoveKind, Outcome,
    Phase, PressSample, RunningMove, StartKind, c2_start_tick, cancel_for, classify_press,
    resolve_frame, route_air_attack,
};
use crate::prototype::{BodyDrive, Hero, feet_height_above_solid};

/// Carl's CF02 press and air state. All of it is constant-time bookkeeping that
/// is mixed into the state hash only while it is not the default, so a CF02 run
/// that never buffers, jumps or fights in the air hashes like the rest.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct CarlCf02 {
    buffer: MoveBuffer,
    /// The hero was airborne on the previous planning tick (landing detection).
    airborne: bool,
    /// Air kicks and dives used in this airborne period (Q50, Q52).
    pub(crate) air_kicks_used: u8,
    pub(crate) dives_used: u8,
    /// The bail-out hop in flight (Q54): its horizontal speed. While it is set
    /// no attack starts until landing.
    pub(crate) bail_hop: Option<f32>,
    /// Set by a bail-out, cleared only on landing or restart (Q54): a hurt
    /// mid-hop drops the hop's drive but not "no attack until landing" nor the
    /// bail-out's landing recovery.
    pub(crate) no_attack_until_landing: bool,
    /// The airborne state after a dive's first hit (C5's source).
    pub(crate) bounce: bool,
    /// The dive's horizontal speed during its tuck (half the speed at the start).
    dive_vx: f32,
    /// Ticks of landing recovery still to run (Q56): no move, no jump.
    pub(crate) landing_lock: u8,
    /// A Jump pressed before a C3 window opened, waiting for it (N1).
    jump_press_age: Option<u8>,
    /// The launch of this tick's bail-out or bounce, consumed by the drive.
    pending_launch: Option<(f32, f32)>,
}

impl CarlCf02 {
    /// Mixed only while something is set, behind a marker word.
    pub(crate) fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        // The landing detector's `airborne` flag is the previous tick's grounded
        // state: it has no effect while the counters are quiet (a plain jump has
        // no landing recovery), so it neither counts as state nor is mixed.
        let quiet = Self {
            airborne: self.airborne,
            ..Self::default()
        };
        if *self == quiet {
            return;
        }
        mix(0x4341_524c_4346_3032);
        match self.buffer.peek() {
            Some(press) => {
                mix(1 + press.button as u64);
                mix(press.move_x_sign as u64);
                mix(u64::from(press.down));
                mix(u64::from(press.age()));
            }
            None => mix(0),
        }
        mix(u64::from(self.air_kicks_used));
        mix(u64::from(self.dives_used));
        mix(self.bail_hop.map_or(0, |vx| u64::from(vx.to_bits()) + 1));
        mix(u64::from(self.no_attack_until_landing));
        mix(u64::from(self.bounce));
        mix(u64::from(self.dive_vx.to_bits()));
        mix(u64::from(self.landing_lock));
        mix(self.jump_press_age.map_or(0, |age| u64::from(age) + 1));
    }

    /// Forget a buffered press and every in-flight air move (Carl was hurt or
    /// defeated); the airborne period counters and `no_attack_until_landing`
    /// stay until he lands.
    fn drop_moves(&mut self) {
        self.buffer.clear();
        self.bail_hop = None;
        self.bounce = false;
        self.jump_press_age = None;
        self.pending_launch = None;
    }
}

/// What Carl's CF02 planning asks of the tick.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CarlCf02Plan {
    /// The move to hand to the runner this tick (`None`: no new move).
    pub(crate) request: Option<&'static MoveSpec>,
    /// The jump press was used (a bail-out) or is held for a cancel window: it
    /// must not also be the ordinary jump.
    pub(crate) strip_jump: bool,
    /// Landing recovery: the feet stay planted like a ground commitment.
    pub(crate) planted: bool,
}

fn sign_of(move_x: f32) -> i8 {
    if move_x > 0.0 {
        1
    } else if move_x < 0.0 {
        -1
    } else {
        0
    }
}

/// The CF02 move kind of a Carl move id (`None`: not a Carl move the cancel table
/// knows).
fn kind_of(id: MoveId) -> Option<MoveKind> {
    Some(match id {
        MoveId::Jab => MoveKind::Jab,
        MoveId::Haymaker => MoveKind::Haymaker,
        MoveId::PushKick => MoveKind::PushKick,
        MoveId::FollowKick => MoveKind::FollowKick,
        MoveId::Stomp => MoveKind::Stomp,
        MoveId::AirKick => MoveKind::AirKick,
        MoveId::DiveStomp => MoveKind::DiveStomp,
        _ => return None,
    })
}

/// The move a ground press starts (Q01): Down + Physical is the stomp, Physical the
/// kick, Primary the jab.
fn ground_spec(t: &Cf02Tuning, press: &BufferedPress) -> Option<&'static MoveSpec> {
    match press.button {
        Button::Primary => carl_move(MoveId::Jab),
        Button::Physical => {
            if press.down && t.q01_stomp_binding == StompBinding::DownPhysical {
                Some(&cf02_moves().stomp)
            } else {
                carl_move(MoveId::PushKick)
            }
        }
        Button::Jump => None,
    }
}

/// What the CF02-I presentation may read of Carl's CF02 state: a copy of a few
/// bookkeeping values, never a handle to the simulation. Presentation only; the
/// sim never reads it back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CarlActingView {
    /// A bail-out hop (Q54) is in flight.
    pub bail_hop: bool,
    /// Ticks of landing recovery still to run (Q56).
    pub landing_lock: u8,
    /// Carl pressed Jab, Physical, Jump, Swap or the ability on this tick (a press edge, whether or
    /// not it started anything).
    pub press_edge: bool,
    /// CF02-FU-C2F FU2: Donut pressed a button on this tick (her own press edge,
    /// whether the player or the assistant drives her).
    pub donut_press_edge: bool,
}

impl CombatState {
    /// Carl's CF02 state for the acting presentation (CF02-I).
    pub fn carl_acting_view(&self) -> CarlActingView {
        CarlActingView {
            bail_hop: self.carl_cf02.bail_hop.is_some(),
            landing_lock: self.carl_cf02.landing_lock,
            press_edge: self.input.attack_pressed
                || self.input.physical_pressed
                || self.input.jump_pressed
                || self.input.swap_pressed
                || self.input.ability_pressed,
            donut_press_edge: self.donut_input.attack_pressed
                || self.donut_input.physical_pressed
                || self.donut_input.jump_pressed
                || self.donut_input.ability_pressed,
        }
    }
}

impl CarlCf02 {
    /// The move that is running on this tick's press, as the cancel table sees it.
    fn running_of(&self, melee: &MoveRunner) -> Option<RunningMove> {
        if let Some((id, tick)) = melee.press_tick() {
            let spec = carl_move(id)?;
            let kind = kind_of(id)?;
            let phase = if tick < spec.startup {
                Phase::Startup
            } else if tick < spec.startup + spec.active {
                Phase::Active
            } else {
                Phase::Recovery
            };
            return Some(RunningMove {
                kind,
                phase,
                tick: u8::try_from(tick).unwrap_or(u8::MAX),
                hit: melee.current_hit(),
                in_hit_stop: melee.hit_stop_left() > 0,
                air_kicks_used: self.air_kicks_used,
            });
        }
        self.bounce.then_some(RunningMove {
            kind: MoveKind::DiveBounce,
            phase: Phase::Recovery,
            tick: 0,
            hit: false,
            in_hit_stop: false,
            air_kicks_used: self.air_kicks_used,
        })
    }
}

/// The air state a press is routed with.
fn air_state(state: &CarlCf02, hero: &Hero, solids: &[crate::prototype::Block]) -> AirState {
    AirState {
        vy_px_s: hero.vy as i32,
        height_px: feet_height_above_solid(hero, solids).map_or(i32::MAX, |h| h as i32),
        air_kicks_used: state.air_kicks_used,
        dives_used: state.dives_used,
        no_attack_until_landing: state.no_attack_until_landing,
    }
}

impl CombatState {
    /// Carl's CF02 planning for this tick (call before the move runner steps).
    /// `raw` is Carl's input, `can_act` whether he may act, `frozen` whether this
    /// tick is hit-stop (the buffer does not age in it).
    pub(super) fn plan_carl_cf02(
        &mut self,
        scene: &mut crate::corridor::SceneSim,
        raw: RawInput,
        can_act: bool,
        frozen: bool,
    ) -> CarlCf02Plan {
        let Some(t) = self.cf02 else {
            return CarlCf02Plan::default();
        };
        let mut plan = CarlCf02Plan::default();
        let solids = &scene.world.solids;
        let hero = &mut scene.world.hero;
        let grounded = hero.grounded;
        let state = &mut self.carl_cf02;

        // Landing (Q56): the tick after the body touched down.
        let landed = grounded && state.airborne;
        state.airborne = !grounded;
        if landed {
            let recovery = match hero.melee.current().map(|m| m.spec.id) {
                Some(MoveId::AirKick) => t.q56_landing_air_kick,
                Some(MoveId::DiveStomp) if hero.melee.current_hit() => t.q56_landing_dive_hit,
                Some(MoveId::DiveStomp) => t.q56_landing_dive_whiff,
                _ if state.no_attack_until_landing => t.q56_landing_bailout,
                _ => t.q56_landing_jump,
            };
            if matches!(
                hero.melee.current().map(|m| m.spec.id),
                Some(MoveId::AirKick | MoveId::DiveStomp)
            ) {
                hero.melee.cancel();
            }
            state.air_kicks_used = 0;
            state.dives_used = 0;
            state.dive_vx = 0.0;
            state.bail_hop = None;
            state.no_attack_until_landing = false;
            state.bounce = false;
            state.landing_lock = recovery;
        }
        let blocked = state.landing_lock > 0;
        state.landing_lock = state.landing_lock.saturating_sub(1);
        plan.planted = blocked && grounded;

        if !can_act {
            state.drop_moves();
            return plan;
        }

        let primary = self.input.attack_pressed;
        let physical = self.input.physical_pressed;
        let jump = self.input.jump_pressed;
        let sign = sign_of(Actions::from(raw).move_x);
        let idle_next = hero.melee.idle_next_step();
        let can_jump = grounded && idle_next && !blocked;
        let res = resolve_frame(t, primary, physical, jump, grounded, can_jump);
        let mut hold_for_air = false;

        // An attack press (Q35: Primary wins over Physical on one tick).
        if let Some(button) = res.attack {
            if res.defer_attack {
                // Q47: Jump wins; the attack fires its air version after take-off.
                state
                    .buffer
                    .store(BufferedPress::new(button, sign, raw.down));
                hold_for_air = true;
            } else {
                let actor = ActorCtx {
                    running: state.running_of(&hero.melee),
                    hurt: false,
                };
                let sample = PressSample {
                    button,
                    down: raw.down,
                    move_x_sign: sign,
                    facing: hero.facing,
                };
                let stored = BufferedPress::new(button, sign, raw.down);
                match classify_press(t, &actor, &sample) {
                    Outcome::Cancel(CancelId::C1) => plan.request = carl_move(MoveId::Jab),
                    Outcome::Cancel(CancelId::C2) => {
                        if let Some((_, tick)) = hero.melee.press_tick() {
                            let earliest = c2_start_tick(t, u8::try_from(tick).unwrap_or(u8::MAX));
                            hero.melee.request_cancel(
                                MoveId::Jab,
                                &cf02_moves().follow_kick,
                                u16::from(earliest),
                            );
                        }
                    }
                    Outcome::Cancel(CancelId::C4) => {
                        // An air kick that hit, Down + Physical in its recovery.
                        let air = air_state(state, hero, solids);
                        if route_air_attack(t, button, raw.down, &air) == AirRoute::Dive {
                            hero.melee.cancel();
                            state.dives_used += 1;
                            state.dive_vx = hero.vx * f32::from(t.q52_dive_tuck_vx_pct) / 100.0;
                            plan.request = Some(&cf02_moves().dive_stomp);
                        } else {
                            state.buffer.store(stored);
                        }
                    }
                    Outcome::Cancel(CancelId::C5) => {
                        // Dive bounce -> the period's one air kick.
                        let air = air_state(state, hero, solids);
                        if route_air_attack(t, button, raw.down, &air) == AirRoute::AirKick {
                            state.bounce = false;
                            state.air_kicks_used += 1;
                            plan.request = Some(&cf02_moves().air_kick);
                        } else {
                            state.buffer.store(stored);
                        }
                    }
                    Outcome::Fresh | Outcome::Buffer => {
                        state.buffer.store(stored);
                    }
                    Outcome::Cancel(CancelId::C3 | CancelId::C6) | Outcome::Ignore => {}
                }
            }
        }

        // A Jump that is not a take-off: the C3 bail-out (Q54), or held for the
        // window (N1: a press before the recovery fires on the first tick it opens).
        if jump && !res.jump && grounded {
            state.jump_press_age = Some(0);
        }
        if let Some(age) = state.jump_press_age {
            if !grounded {
                state.jump_press_age = None;
            } else {
                let eligible = state.running_of(&hero.melee).is_some_and(|run| {
                    cancel_for(
                        t,
                        &run,
                        &PressSample {
                            button: Button::Jump,
                            down: false,
                            move_x_sign: sign,
                            facing: hero.facing,
                        },
                    ) == Some(CancelId::C3)
                });
                if eligible && plan.request.is_none() {
                    let toward = Actions::from(raw).move_x;
                    let threshold = f32::from(t.q54_bailout_direction_threshold_milli) / 1000.0;
                    let dir = if toward.abs() >= threshold {
                        f32::from(sign_of(toward))
                    } else {
                        -f32::from(hero.facing)
                    };
                    let vx = dir * f32::from(t.q54_bailout_vx);
                    hero.melee.cancel();
                    state.bail_hop = Some(vx);
                    state.no_attack_until_landing = true;
                    state.bounce = false;
                    state.pending_launch = Some((vx, f32::from(t.q54_bailout_vy)));
                    state.jump_press_age = None;
                    plan.strip_jump = true;
                } else if hero.melee.busy() && age < crate::prototype::JUMP_BUFFER_TICKS {
                    state.jump_press_age = Some(age + 1);
                    plan.strip_jump = true;
                } else {
                    state.jump_press_age = None;
                }
            }
        }

        // N1: a Down + Physical remembered from the air kick's active or hit-stop
        // ticks takes C4 on the first tick the window opens (recovery, after the
        // hit, outside hit-stop) instead of waiting for the kick to end.
        if plan.request.is_none()
            && !hold_for_air
            && !blocked
            && let Some(press) = state.buffer.peek().copied()
            && let Some(run) = state.running_of(&hero.melee)
            && cancel_for(
                t,
                &run,
                &PressSample {
                    button: press.button,
                    down: press.down,
                    move_x_sign: press.move_x_sign,
                    facing: hero.facing,
                },
            ) == Some(CancelId::C4)
        {
            let air = air_state(state, hero, solids);
            if route_air_attack(t, press.button, press.down, &air) == AirRoute::Dive {
                hero.melee.cancel();
                state.dives_used += 1;
                state.dive_vx = hero.vx * f32::from(t.q52_dive_tuck_vx_pct) / 100.0;
                plan.request = Some(&cf02_moves().dive_stomp);
                state.buffer.clear();
            }
        }

        // Offer the remembered press to the runner (Q46): only a runner that can
        // start a fresh move takes it.
        if plan.request.is_none()
            && !hold_for_air
            && !blocked
            && let Some(press) = state.buffer.peek().copied()
            && hero.melee.idle_next_step()
        {
            if grounded {
                if let Some(spec) = ground_spec(t, &press) {
                    if let Some(dir) = press.turn_on_start(StartKind::Fresh) {
                        hero.facing = dir;
                    }
                    plan.request = Some(spec);
                    state.buffer.clear();
                }
            } else {
                let air = air_state(state, hero, solids);
                match route_air_attack(t, press.button, press.down, &air) {
                    AirRoute::AirKick => {
                        state.air_kicks_used += 1;
                        state.bounce = false;
                        plan.request = Some(&cf02_moves().air_kick);
                        state.buffer.clear();
                    }
                    AirRoute::Dive => {
                        state.dives_used += 1;
                        state.bounce = false;
                        state.dive_vx = hero.vx * f32::from(t.q52_dive_tuck_vx_pct) / 100.0;
                        plan.request = Some(&cf02_moves().dive_stomp);
                        state.buffer.clear();
                    }
                    AirRoute::NoAirMove => {
                        // Primary has no air move: the legacy jab.
                        plan.request = carl_move(MoveId::Jab);
                        state.buffer.clear();
                    }
                    AirRoute::WaitForLanding | AirRoute::Hold => {}
                }
            }
        }
        state.buffer.advance(t, frozen);
        plan
    }

    /// Carl's body drive for this tick (call after the strike, with the move that
    /// ran): the dive's tuck and plunge, the bail-out hop, the facing lock of the
    /// air moves, and the bounce that ends a dive on its first hit.
    pub(super) fn carl_cf02_drive(
        &mut self,
        scene: &mut crate::corridor::SceneSim,
        move_tick: Option<MoveTick>,
    ) -> BodyDrive {
        let Some(t) = self.cf02 else {
            return BodyDrive::default();
        };
        let hero = &mut scene.world.hero;
        let state = &mut self.carl_cf02;
        let mut drive = BodyDrive::default();
        if let Some((vx, vy)) = state.pending_launch.take() {
            drive.launch = Some((vx, vy));
        }
        if let Some(vx) = state.bail_hop {
            drive.vx = Some(vx);
            drive.facing_locked = true;
        }
        if let Some(tick) = move_tick {
            match tick.spec.id {
                MoveId::AirKick => drive.facing_locked = true,
                MoveId::DiveStomp => {
                    drive.facing_locked = true;
                    let hit_done = hero.melee.current_hit() && hero.melee.hit_stop_left() == 0;
                    if !tick.frozen && hit_done {
                        // Q53: the first hit bounces Carl and ends the dive; the
                        // period's one air kick is still allowed (C5).
                        hero.melee.cancel();
                        state.bounce = true;
                        drive.fly_vy = None;
                        drive.vx = Some(0.0);
                        if t.q53_bounce_vy > 0 {
                            drive.launch = Some((0.0, f32::from(t.q53_bounce_vy)));
                        }
                        drive.facing_locked = false;
                    } else if tick.tick < u16::from(t.q52_dive_tuck_ticks) {
                        drive.fly_vy = Some(0.0);
                        drive.vx = Some(state.dive_vx);
                    } else {
                        drive.fly_vy = Some(f32::from(t.q52_dive_plunge_vy));
                        drive.vx = Some(f32::from(hero.facing) * f32::from(t.q52_dive_plunge_vx));
                    }
                }
                _ => {}
            }
        }
        drive
    }
}
