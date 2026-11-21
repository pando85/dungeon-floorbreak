//! CARL-PROG-01 CP-B1: the thrown timed charge, simulation only. Bevy-free.
//!
//! The special (Circle/B) of a hero that holds the `special_thrown_charge`
//! capability: a 70-tick throw clock, a stick that flies a predictable arc,
//! stops dead on a walking surface and blows exactly [`CarlProgTuning`]'s
//! `cpq18_fuse_ticks` ticks after it was released, a recharge counted from the
//! detonation, one stick at a time and unlimited uses (spec CP-Q12..CP-Q19,
//! CP-Q27, CP-Q29).
//!
//! This module is the pure state machine: the clock, the stick's flight and the
//! press gate. The combat scene (`combat_scene/thrown.rs`) feeds it the press,
//! the solids and the hurt events and turns its outcomes into events, the one
//! reserved [`crate::blast::BlastCharge`] slot and the blast resolution. A hero
//! that never pressed (or never held the capability) has the default, idle
//! [`Thrower`], which mixes **nothing** into a state hash.
//!
//! IP-neutral: nothing here names a franchise item; the fan layer is data.
//! Every number is a knob of `carl_prog_v0.json` (read through
//! [`CarlProgTuning::active`]). No RNG anywhere.

use crate::blast::BlastProfile;
use crate::carl_prog::CarlProgTuning;
use crate::hero_damage::HeroDamagePolicy;
use crate::prototype::{Block, GRAVITY, TICK_SECONDS};

/// The `HeroBlow` / `HeroHurt` source id of the thrown charge's blast on a hero
/// (the self-damage of CP-Q23). Distinct from every other hero-hurt source id
/// (see `every_hero_hurt_source_id_is_distinct`).
pub const THROWN_CHARGE_SOURCE: u32 = 0x5443; // "TC"

/// Why a special press was refused (CP-Q13). Shared with every hero's special:
/// DONUT-PROG (DP-Q26) appends `NoMana` and `NoDestination`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialReject {
    /// The hero holds no such capability. Never emitted for a press (a press of
    /// a hero without the capability is inert and silent); the shared vocabulary.
    Locked,
    /// A throw is running, a stick is live, the hero is hurt, defeated, frozen
    /// by hit-stop or in a melee move.
    Busy,
    /// The recharge after the last detonation is not over.
    Cooldown,
    /// The hero is not on the ground (checked at the press only, CP-Q13).
    Airborne,
}

impl SpecialReject {
    pub const ALL: [Self; 4] = [Self::Locked, Self::Busy, Self::Cooldown, Self::Airborne];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Locked => "locked",
            Self::Busy => "busy",
            Self::Cooldown => "cooldown",
            Self::Airborne => "airborne",
        }
    }

    /// Stable code for event hashes.
    pub const fn code(self) -> u64 {
        self as u64
    }
}

/// What the scene knows about the pressing hero at the press tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PressView {
    pub grounded: bool,
    /// Hurt flinch running.
    pub hurt: bool,
    pub defeated: bool,
    /// Frozen by hit-stop.
    pub frozen: bool,
    /// A melee move is running (started this tick or earlier).
    pub melee_busy: bool,
}

/// One running throw: the press tick and whether the stick has been released.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Throw {
    start: u64,
    released: bool,
}

/// What a hurt hero's running throw does (CP-Q27).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HurtOutcome {
    /// No throw was running.
    None,
    /// A hit on t0..t5: the throw is cancelled, nothing was spent.
    Cancelled,
    /// A hit on t6..t40: the lit stick drops at the hero's feet; its fuse
    /// restarts from this tick.
    Drop,
    /// A hit on t41 or later: the stick already left, only the recovery ends.
    RecoveryEnded,
}

/// The live stick: a point that flies, lands and burns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stick {
    /// Column and height of the stick's foot (y up).
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    /// It stopped on a walking surface (never again moves).
    pub landed: bool,
    /// The tick it detonates on (`==` compare, like DEATH-D).
    pub detonate_tick: u64,
}

/// What one tick of the stick's flight did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StickStep {
    /// It stopped on a surface this tick (`ChargeLanded` is emitted once).
    pub landed: bool,
}

/// One hero's thrower. `Default` is idle: it mixes nothing and costs nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Thrower {
    throw: Option<Throw>,
    stick: Option<Stick>,
    /// The first tick a press is accepted again (detonation + recharge).
    cooldown_until: Option<u64>,
}

impl Thrower {
    /// Nothing running, nothing recharging: nothing to hash.
    pub fn is_idle(&self) -> bool {
        self.throw.is_none() && self.stick.is_none() && self.cooldown_until.is_none()
    }

    pub fn stick(&self) -> Option<&Stick> {
        self.stick.as_ref()
    }

    pub fn stick_mut(&mut self) -> Option<&mut Stick> {
        self.stick.as_mut()
    }

    /// Whether the special is recharging (after a detonation, before the ready
    /// tick).
    pub fn recharging(&self) -> bool {
        self.cooldown_until.is_some()
    }

    /// The clock tick of the running throw on `tick` (0 on the press tick).
    pub fn throw_clock(&self, tick: u64) -> Option<u64> {
        self.throw.map(|t| tick.saturating_sub(t.start))
    }

    /// The special is ready again on exactly this tick (the recharge ends):
    /// true once, and the press of this very tick is accepted.
    pub fn take_ready(&mut self, tick: u64) -> bool {
        if self.cooldown_until == Some(tick) {
            self.cooldown_until = None;
            return true;
        }
        false
    }

    /// The throw clock ends on the tick it reaches its length.
    pub fn end_clock(&mut self, tick: u64) {
        let length = u64::from(CarlProgTuning::active().cpq14_throw_ticks);
        if self.throw_clock(tick).is_some_and(|t| t >= length) {
            self.throw = None;
        }
    }

    /// The press gate (CP-Q13). On success the throw starts: its clock is 0 on
    /// `tick`. Rejection order: busy, then cooldown, then airborne.
    pub fn press(&mut self, tick: u64, view: &PressView) -> Result<(), SpecialReject> {
        if self.throw.is_some()
            || self.stick.is_some()
            || view.hurt
            || view.defeated
            || view.frozen
            || view.melee_busy
        {
            return Err(SpecialReject::Busy);
        }
        if self.cooldown_until.is_some_and(|until| tick < until) {
            return Err(SpecialReject::Cooldown);
        }
        if !view.grounded {
            return Err(SpecialReject::Airborne);
        }
        self.throw = Some(Throw {
            start: tick,
            released: false,
        });
        Ok(())
    }

    /// A melee request is blocked until the recovery may be cancelled (CP-Q14:
    /// "nothing cancels earlier").
    pub fn blocks_melee(&self, tick: u64) -> bool {
        self.throw_clock(tick)
            .is_some_and(|t| t < u64::from(CarlProgTuning::active().cpq14_cancel_from_tick))
    }

    /// Jump presses are ignored from t0 to t49 (CP-Q15).
    pub fn strips_jump(&self, tick: u64) -> bool {
        self.throw_clock(tick)
            .is_some_and(|t| t < u64::from(CarlProgTuning::active().cpq15_jump_ignored_until_tick))
    }

    /// From t50 any melee move or jump cancels the recovery (CP-Q14). The stick
    /// has long left, so cancelling only frees the hero.
    pub fn cancel_recovery(&mut self, tick: u64) {
        if self
            .throw_clock(tick)
            .is_some_and(|t| t >= u64::from(CarlProgTuning::active().cpq14_cancel_from_tick))
        {
            self.throw = None;
        }
    }

    /// Whether the stick leaves the hand on `tick` (t41, once).
    pub fn releases_now(&self, tick: u64) -> bool {
        self.throw.is_some_and(|t| {
            !t.released
                && tick.saturating_sub(t.start)
                    == u64::from(CarlProgTuning::active().cpq14_spawn_tick)
        })
    }

    /// Release the stick from the hero's pivot `(x, foot_y)` facing `facing`
    /// (+1 / -1) while moving at `hero_vx` px/s (CP-Q16, CP-Q17): spawn at
    /// `pivot + (facing * 16, 46 up)`, `vx = facing * 150 + 0.5 * carl_vx`
    /// (never backwards: only the share of the hero's speed along the facing
    /// counts), `vy = 260`. The fuse runs 150 ticks from `tick`.
    pub fn release(&mut self, tick: u64, x: f32, foot_y: f32, facing: i8, hero_vx: f32) {
        let t = CarlProgTuning::active();
        let dir = f32::from(facing);
        let along = (hero_vx * dir).max(0.0);
        let share = f32::from(t.cpq17_vx_thrower_share_pct) / 100.0;
        if let Some(throw) = self.throw.as_mut() {
            throw.released = true;
        }
        self.stick = Some(Stick {
            x: x + dir * f32::from(t.cpq16_spawn_dx_px),
            y: foot_y + f32::from(t.cpq16_spawn_dy_px),
            vx: dir * (f32::from(t.cpq17_vx_base_px_s) + share * along),
            vy: f32::from(t.cpq17_vy_px_s),
            landed: false,
            detonate_tick: tick + u64::from(t.cpq18_fuse_ticks),
        });
    }

    /// A hit on the hero (CP-Q27): the throw ends in every case; a hit on
    /// t6..t40 drops the lit stick at `(x, foot_y)` with its fuse restarted.
    pub fn on_hurt(&mut self, tick: u64, x: f32, foot_y: f32) -> HurtOutcome {
        let Some(throw) = self.throw.take() else {
            return HurtOutcome::None;
        };
        if throw.released {
            return HurtOutcome::RecoveryEnded;
        }
        let t = CarlProgTuning::active();
        if tick.saturating_sub(throw.start) < u64::from(t.cpq27_drop_from_tick) {
            return HurtOutcome::Cancelled;
        }
        self.stick = Some(Stick {
            x,
            y: foot_y,
            vx: 0.0,
            vy: 0.0,
            landed: false,
            detonate_tick: tick + u64::from(t.cpq18_fuse_ticks),
        });
        HurtOutcome::Drop
    }

    /// One tick of the live stick's flight against `solids` (CP-Q17). The stick
    /// ignores actors, stops dead on a walking surface (no bounce or slide),
    /// loses `vx` on a wall face and `vy` on a ceiling, never detonates on
    /// contact, and clamps to the world column range like a hero. With nothing
    /// below it keeps falling; there is no despawn path.
    pub fn step_stick(&mut self, solids: &[Block], x_range: (f32, f32)) -> StickStep {
        let Some(stick) = self.stick.as_mut() else {
            return StickStep::default();
        };
        if stick.landed {
            return StickStep::default();
        }
        let (x, y) = (stick.x, stick.y);
        let mut next_x = (x + stick.vx * TICK_SECONDS).clamp(x_range.0, x_range.1);
        for block in solids.iter().filter(|block| block.intact) {
            let rect = block.rect;
            // A wall face: the point is strictly between the block's bottom and
            // top (resting on a top is not "inside") and crosses a vertical face.
            if y > rect.bottom && y < rect.top() {
                if stick.vx > 0.0 && x <= rect.left && next_x >= rect.left {
                    next_x = next_x.min(rect.left);
                    stick.vx = 0.0;
                } else if stick.vx < 0.0 && x >= rect.right() && next_x <= rect.right() {
                    next_x = next_x.max(rect.right());
                    stick.vx = 0.0;
                }
            }
        }
        stick.x = next_x;
        stick.vy -= GRAVITY * TICK_SECONDS;
        let mut next_y = y + stick.vy * TICK_SECONDS;
        let mut landed = false;
        for block in solids.iter().filter(|block| block.intact) {
            let rect = block.rect;
            if stick.x < rect.left || stick.x > rect.right() {
                continue;
            }
            if stick.vy <= 0.0 && y >= rect.top() - 0.01 && next_y <= rect.top() {
                next_y = rect.top();
                stick.vy = 0.0;
                landed = true;
            } else if stick.vy > 0.0 && y <= rect.bottom + 0.01 && next_y >= rect.bottom {
                next_y = rect.bottom;
                stick.vy = 0.0;
            }
        }
        stick.y = next_y;
        if landed {
            stick.vx = 0.0;
            stick.landed = true;
        }
        StickStep { landed }
    }

    /// The stick blew on `tick`: it is gone and the recharge starts now.
    pub fn detonated(&mut self, tick: u64) {
        self.stick = None;
        self.cooldown_until = Some(tick + u64::from(CarlProgTuning::active().cpq19_recharge_ticks));
    }

    /// Fold the state into a hash: nothing while idle (CP-Q31).
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        if self.is_idle() {
            return;
        }
        mix(0x5448_5257); // "THRW"
        match self.throw {
            Some(throw) => {
                mix(1 + throw.start);
                mix(u64::from(throw.released));
            }
            None => mix(0),
        }
        match self.stick {
            Some(stick) => {
                mix(1);
                mix(u64::from(stick.x.to_bits()));
                mix(u64::from(stick.y.to_bits()));
                mix(u64::from(stick.vx.to_bits()));
                mix(u64::from(stick.vy.to_bits()));
                mix(u64::from(stick.landed));
                mix(stick.detonate_tick);
            }
            None => mix(0),
        }
        mix(self.cooldown_until.map_or(0, |t| t + 1));
    }
}

/// The blast profile of the thrown charge, from the knobs (CP-Q20..CP-Q26,
/// K3): a 20 px lethal core, a 64 px ring that hurts and launches, line of
/// sight on, the thrower reduced and nobody else hurt (`OwnerReduced`), and the
/// structure-breaking flag CP-C will read.
pub fn blast_profile(t: &CarlProgTuning) -> BlastProfile {
    BlastProfile {
        core_radius: f32::from(t.cpq20_core_radius_px),
        outer_radius: f32::from(t.cpq20_outer_radius_px),
        outer_damage: t.cpq21_outer_damage,
        llama_outer_damage: t.cpq26_llama_outer_damage,
        boss_damage: t.cpq25_boss_damage,
        launch: (
            f32::from(t.cpq21_launch_vx_px_s),
            f32::from(t.cpq21_launch_vy_px_s),
        ),
        los: true,
        hero_damage: HeroDamagePolicy::OwnerReduced,
        hero_hp: t.cpq23_self_damage,
        hero_knock: (
            f32::from(t.cpq23_self_knock_vx_px_s),
            f32::from(t.cpq23_self_knock_vy_px_s),
        ),
        hero_source: THROWN_CHARGE_SOURCE,
        breaks_structures: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blast::BLAST_CENTRE_DY;
    use crate::prototype::{Material, Rect};

    fn floor(top: f32) -> Block {
        Block {
            rect: Rect {
                left: 0.0,
                bottom: top - 40.0,
                width: 2000.0,
                height: 40.0,
            },
            material: Material::Stone,
            intact: true,
        }
    }

    const GROUND: PressView = PressView {
        grounded: true,
        hurt: false,
        defeated: false,
        frozen: false,
        melee_busy: false,
    };

    const X_RANGE: (f32, f32) = (12.0, 1988.0);

    fn flight(thrower: &mut Thrower, solids: &[Block]) -> (u64, f32, f32) {
        for n in 1..400u64 {
            if thrower.step_stick(solids, X_RANGE).landed {
                let s = thrower.stick().unwrap();
                return (n, s.x, s.y);
            }
        }
        panic!("never landed");
    }

    #[test]
    fn the_press_gate_orders_busy_then_cooldown_then_airborne() {
        let mut t = Thrower::default();
        let air = PressView {
            grounded: false,
            ..GROUND
        };
        assert_eq!(t.press(10, &air), Err(SpecialReject::Airborne));
        assert!(t.is_idle(), "a rejected press changes nothing");
        for busy in [
            PressView {
                hurt: true,
                ..GROUND
            },
            PressView {
                defeated: true,
                ..GROUND
            },
            PressView {
                frozen: true,
                ..GROUND
            },
            PressView {
                melee_busy: true,
                ..GROUND
            },
        ] {
            assert_eq!(t.press(10, &busy), Err(SpecialReject::Busy));
        }
        assert_eq!(t.press(10, &GROUND), Ok(()));
        assert_eq!(
            t.press(11, &GROUND),
            Err(SpecialReject::Busy),
            "one throw at a time"
        );
        // A cooldown beats the airborne reason.
        let mut c = Thrower::default();
        c.detonated(100);
        assert_eq!(c.press(101, &air), Err(SpecialReject::Cooldown));
    }

    #[test]
    fn the_recharge_is_exactly_420_ticks_from_the_detonation() {
        let mut t = Thrower::default();
        t.detonated(1000);
        assert!(t.recharging());
        assert_eq!(t.press(1000 + 419, &GROUND), Err(SpecialReject::Cooldown));
        assert!(!t.take_ready(1000 + 419));
        assert!(
            t.take_ready(1000 + 420),
            "ready exactly once, on detonation + 420"
        );
        assert!(!t.take_ready(1000 + 420));
        assert!(t.is_idle());
        assert_eq!(t.press(1000 + 420, &GROUND), Ok(()));
    }

    #[test]
    fn the_clock_ends_after_70_ticks_and_melee_and_jump_follow_the_contract() {
        let mut t = Thrower::default();
        assert_eq!(t.press(100, &GROUND), Ok(()));
        for clock in 0..70u64 {
            let tick = 100 + clock;
            assert_eq!(t.throw_clock(tick), Some(clock));
            assert_eq!(t.blocks_melee(tick), clock < 50, "t{clock}");
            assert_eq!(t.strips_jump(tick), clock < 50, "t{clock}");
        }
        // Cancel only from t50.
        t.cancel_recovery(100 + 49);
        assert!(
            t.throw_clock(100 + 49).is_some(),
            "nothing cancels before t50"
        );
        t.cancel_recovery(100 + 50);
        assert!(t.throw_clock(100 + 50).is_none());
        let mut full = Thrower::default();
        full.press(100, &GROUND).unwrap();
        full.end_clock(100 + 69);
        assert!(full.throw_clock(100 + 69).is_some());
        full.end_clock(100 + 70);
        assert!(full.is_idle());
    }

    #[test]
    fn the_stick_leaves_on_t41_from_the_standing_offset_and_burns_150_ticks() {
        let mut t = Thrower::default();
        t.press(100, &GROUND).unwrap();
        assert!(!t.releases_now(100 + 40));
        assert!(t.releases_now(100 + 41));
        t.release(100 + 41, 400.0, 119.0, 1, 0.0);
        assert!(!t.releases_now(100 + 41), "released once");
        let s = *t.stick().unwrap();
        assert_eq!((s.x, s.y), (416.0, 165.0), "pivot + (16, 46 up)");
        assert_eq!((s.vx, s.vy), (150.0, 260.0));
        assert_eq!(s.detonate_tick, 100 + 41 + 150);
        // Facing left mirrors x.
        let mut l = Thrower::default();
        l.press(1, &GROUND).unwrap();
        l.release(42, 400.0, 119.0, -1, 0.0);
        assert_eq!(
            (l.stick().unwrap().x, l.stick().unwrap().vx),
            (384.0, -150.0)
        );
    }

    #[test]
    fn running_adds_half_the_speed_along_the_facing_and_never_goes_backwards() {
        let release = |facing: i8, vx: f32| {
            let mut t = Thrower::default();
            t.press(1, &GROUND).unwrap();
            t.release(42, 400.0, 119.0, facing, vx);
            t.stick().unwrap().vx
        };
        assert_eq!(release(1, 185.0), 150.0 + 92.5);
        assert_eq!(release(-1, -185.0), -(150.0 + 92.5));
        // Walking backwards while facing forward: only the base speed.
        assert_eq!(release(1, -185.0), 150.0);
        assert_eq!(release(-1, 185.0), -150.0);
    }

    #[test]
    fn a_standing_throw_lands_about_111_px_ahead_and_a_running_one_about_170() {
        let solids = [floor(119.0)];
        let mut standing = Thrower::default();
        standing.press(1, &GROUND).unwrap();
        standing.release(42, 400.0, 119.0, 1, 0.0);
        let (ticks, x, y) = flight(&mut standing, &solids);
        assert_eq!(y, 119.0, "stops dead on the floor top");
        assert!(
            (x - 400.0 - 111.0).abs() < 4.0,
            "standing lands near +111: {x}"
        );
        assert_eq!(ticks, 38, "landing tick pinned");
        let mut running = Thrower::default();
        running.press(1, &GROUND).unwrap();
        running.release(42, 400.0, 119.0, 1, 185.0);
        let (_, rx, _) = flight(&mut running, &solids);
        assert!(
            (rx - 400.0 - 170.0).abs() < 6.0,
            "running lands near +170: {rx}"
        );
        // It never moves again once landed.
        let before = *running.stick().unwrap();
        for _ in 0..30 {
            assert_eq!(running.step_stick(&solids, X_RANGE), StickStep::default());
        }
        assert_eq!(*running.stick().unwrap(), before);
    }

    #[test]
    fn a_wall_takes_vx_a_ceiling_takes_vy_and_nothing_below_means_it_keeps_falling() {
        // A wall at x 440 (floor 119): the stick loses vx and drops beside it.
        let wall = Block {
            rect: Rect {
                left: 440.0,
                bottom: 119.0,
                width: 20.0,
                height: 300.0,
            },
            material: Material::Stone,
            intact: true,
        };
        let solids = [floor(119.0), wall];
        let mut t = Thrower::default();
        t.press(1, &GROUND).unwrap();
        t.release(42, 400.0, 119.0, 1, 0.0);
        let (_, x, y) = flight(&mut t, &solids);
        assert_eq!(
            (x, y),
            (440.0, 119.0),
            "against the wall face, on the floor"
        );
        // A ceiling at 180 (the flat arc peaks near 195): vy is lost, the stick falls straight from there.
        let ceiling = Block {
            rect: Rect {
                left: 380.0,
                bottom: 180.0,
                width: 100.0,
                height: 20.0,
            },
            material: Material::Stone,
            intact: true,
        };
        let mut c = Thrower::default();
        c.press(1, &GROUND).unwrap();
        c.release(42, 400.0, 119.0, 1, 0.0);
        let mut max_y = 0.0_f32;
        for _ in 0..80 {
            c.step_stick(&[floor(119.0), ceiling.clone()], X_RANGE);
            max_y = max_y.max(c.stick().unwrap().y);
        }
        assert_eq!(max_y, 180.0, "stopped by the ceiling underside");
        // Nothing below: it keeps falling, never lands, never despawns.
        let mut pit = Thrower::default();
        pit.press(1, &GROUND).unwrap();
        pit.release(42, 400.0, 119.0, 1, 0.0);
        for _ in 0..400 {
            assert!(!pit.step_stick(&[], X_RANGE).landed);
        }
        let s = pit.stick().unwrap();
        assert!(!s.landed && s.y < -1000.0 && s.detonate_tick == 42 + 150);
    }

    #[test]
    fn the_stick_clamps_to_the_world_columns_like_a_hero() {
        let mut t = Thrower::default();
        t.press(1, &GROUND).unwrap();
        t.release(42, 1980.0, 119.0, 1, 185.0);
        let solids = [floor(119.0)];
        let (_, x, _) = flight(&mut t, &solids);
        assert_eq!(x, X_RANGE.1);
        let mut l = Thrower::default();
        l.press(1, &GROUND).unwrap();
        l.release(42, 20.0, 119.0, -1, -185.0);
        let (_, lx, _) = flight(&mut l, &solids);
        assert_eq!(lx, X_RANGE.0);
    }

    #[test]
    fn a_hit_cancels_drops_or_only_ends_the_recovery() {
        let t0 = |hit_at: u64| {
            let mut t = Thrower::default();
            t.press(100, &GROUND).unwrap();
            let outcome = t.on_hurt(100 + hit_at, 400.0, 119.0);
            (t, outcome)
        };
        for clock in 0..6 {
            let (t, outcome) = t0(clock);
            assert_eq!(outcome, HurtOutcome::Cancelled, "t{clock}");
            assert!(t.is_idle(), "nothing spent at t{clock}");
        }
        for clock in 6..=40 {
            let (t, outcome) = t0(clock);
            assert_eq!(outcome, HurtOutcome::Drop, "t{clock}");
            let s = t.stick().unwrap();
            assert_eq!((s.x, s.y, s.vx, s.vy), (400.0, 119.0, 0.0, 0.0));
            assert_eq!(
                s.detonate_tick,
                100 + clock + 150,
                "the fuse restarts at the drop"
            );
            assert!(t.throw_clock(100 + clock).is_none());
        }
        // t41: the stick has already spawned (the special step runs first).
        let mut t = Thrower::default();
        t.press(100, &GROUND).unwrap();
        t.release(141, 400.0, 119.0, 1, 0.0);
        let stick = *t.stick().unwrap();
        assert_eq!(t.on_hurt(141, 400.0, 119.0), HurtOutcome::RecoveryEnded);
        assert_eq!(*t.stick().unwrap(), stick, "the thrown stick is left alone");
        assert!(t.throw_clock(141).is_none(), "the recovery ended");
        assert_eq!(Thrower::default().on_hurt(5, 0.0, 0.0), HurtOutcome::None);
    }

    #[test]
    fn a_dropped_stick_in_the_air_falls_and_lands() {
        let mut t = Thrower::default();
        t.press(100, &GROUND).unwrap();
        assert_eq!(t.on_hurt(110, 400.0, 160.0), HurtOutcome::Drop);
        let (_, x, y) = flight(&mut t, &[floor(119.0)]);
        assert_eq!((x, y), (400.0, 119.0));
    }

    #[test]
    fn detonation_clears_the_stick_and_starts_the_recharge() {
        let mut t = Thrower::default();
        t.press(1, &GROUND).unwrap();
        t.release(42, 400.0, 119.0, 1, 0.0);
        t.end_clock(71);
        t.detonated(192);
        assert!(t.stick().is_none() && t.recharging());
        assert!(!t.is_idle());
    }

    #[test]
    fn an_idle_thrower_mixes_nothing_and_a_busy_one_does() {
        let words = |t: &Thrower| {
            let mut out = Vec::new();
            t.mix_into(&mut |w| out.push(w));
            out
        };
        assert!(words(&Thrower::default()).is_empty());
        let mut a = Thrower::default();
        a.press(10, &GROUND).unwrap();
        let mut b = a;
        b.release(51, 400.0, 119.0, 1, 0.0);
        assert!(!words(&a).is_empty());
        assert_ne!(words(&a), words(&b));
    }

    #[test]
    fn the_profile_follows_the_knobs_and_is_never_the_qa_profile() {
        let t = CarlProgTuning::active();
        let p = blast_profile(t);
        assert!(!p.is_qa());
        assert_eq!((p.core_radius, p.outer_radius), (20.0, 64.0));
        assert_eq!(p.launch, (200.0, 220.0));
        assert_eq!(p.hero_damage, HeroDamagePolicy::OwnerReduced);
        assert!(p.los && p.breaks_structures);
        assert_eq!(
            (p.outer_damage, p.llama_outer_damage, p.boss_damage),
            (1, 1, 4)
        );
        assert_eq!(
            f32::from(t.cpq20_centre_dy_px),
            BLAST_CENTRE_DY,
            "the data knob equals the const"
        );
    }

    /// The `HeroBlow` / `HeroHurt` source ids of every setpiece that hurts a
    /// hero must stay distinct (integration plan section 3). Each later step
    /// that adds an id appends it here. Goblin swings use the goblin's index as
    /// the source, below 0x100. The Rot, slime and acid sources carry an index
    /// in the high 16 bits, so the comparison is on the low 16 bits.
    #[test]
    fn every_hero_hurt_source_id_is_distinct() {
        let ids = [
            ("boss", crate::boss::BOSS_SOURCE_ID),
            ("rack", crate::boss::RACK_SOURCE_ID),
            ("shore block", crate::cf02::arena::SHORE_BLOCK_SOURCE),
            ("hoarder", crate::pile_boss::PILE_BOSS_SOURCE_ID),
            ("rot sticker", crate::rot_sticker::ROT_SOURCE_ID),
            ("slime", crate::slime::SLIME_SOURCE_ID),
            ("acid", crate::slime::ACID_SOURCE_ID),
            ("llama rush", crate::llama_rush::LLAMA_RUSH_SOURCE_ID),
            ("thrown charge", THROWN_CHARGE_SOURCE),
            // BOIL-B wire: the env step's falling mass and steam.
            ("environment", crate::combat_scene::ENV_SOURCE_ID),
            // ENEMY-SLOT: the test dummy; every real slot enemy appends its own id here.
            ("slot test dummy", crate::scene_enemy::testing::DUMMY_SOURCE),
        ];
        let low = |id: u32| id & 0xFFFF;
        for (i, (name, id)) in ids.iter().enumerate() {
            assert!(low(*id) > 0xFF, "{name}: above the goblin index range");
            for (other, other_id) in &ids[i + 1..] {
                assert_ne!(
                    low(*id),
                    low(*other_id),
                    "{name} and {other} share a source id"
                );
            }
        }
    }

    #[test]
    fn the_reject_reasons_have_distinct_names_and_codes() {
        let mut names: Vec<&str> = SpecialReject::ALL.iter().map(|r| r.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 4);
        let mut codes: Vec<u64> = SpecialReject::ALL.iter().map(|r| r.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), 4);
    }
}
