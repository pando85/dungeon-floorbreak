//! DEATH-D: the authoritative blast resolution, as authored deterministic
//! gameplay. Bevy-free.
//!
//! There is no explosive in the game before its story unlock, so the only
//! producer of a [`BlastCharge`] is a **QA fixture** (`Fixture::charges`): it
//! is not placed on any route, it is not an ability and Carl cannot create
//! one. The dynamite special will later reuse this contract (a thrown stick
//! that lands becomes a charge with an owner and a fuse tick).
//!
//! A charge detonates exactly once, on its attempt-local tick. Its blast is
//! evaluated once against the bodies of the living goblins: the squared
//! distance from the blast centre to a body box is compared with the squared
//! radius (never a square root), so the boundaries are exact.
//!
//! Disclosed limits (the dynamite PR must add them before any player-facing
//! explosive): no line of sight, no terrain damage, no friendly fire, no
//! damage to Carl or Donut.

use crate::prototype::Rect;

/// A body box within this distance of the blast centre is killed (thrown).
pub const BLAST_RADIUS: f32 = 20.0;
/// The blast centre sits this high above the walking line (the VFX-01 burst
/// centre for a floor blast).
pub const BLAST_CENTRE_DY: f32 = 8.0;

/// What sets a charge off.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Arm {
    /// The QA fixture rule: it burns from the first tick and detonates on its
    /// fixed tick (`detonate_tick`).
    Tick,
    /// DEMO-01: it lies unlit until Carl passes column `x` (his x greater than
    /// it), then burns and detonates `fuse_ticks` ticks later. A labelled test
    /// prop, never Carl's ability.
    CarlPast { x: f32, fuse_ticks: u64 },
}

/// How a blast reaches a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlastReach {
    /// The centre is inside the body box: the burst (catastrophic).
    PointBlank,
    /// Within [`BLAST_RADIUS`] but not inside: thrown dead (heavy).
    Ring,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlastCharge {
    /// Column of the charge.
    pub x: f32,
    /// Feet height of the walking line it lies on.
    pub foot_y: f32,
    /// The attempt-local tick it detonates on.
    pub detonate_tick: u64,
    /// The player credited with the blast (`None`: an authored charge).
    pub owner: Option<u32>,
    pub detonated: bool,
    /// What sets it off; `Arm::Tick` for every older fixture.
    pub arm: Arm,
    /// The tick a `CarlPast` charge was armed on (`None` until then).
    pub armed_tick: Option<u64>,
}

impl BlastCharge {
    pub fn new(x: f32, foot_y: f32, detonate_tick: u64, owner: Option<u32>) -> Self {
        Self {
            x,
            foot_y,
            detonate_tick,
            owner,
            detonated: false,
            arm: Arm::Tick,
            armed_tick: None,
        }
    }

    /// The same charge with another arming rule (the detonation tick is then
    /// decided by the rule, not by the constructor).
    #[must_use]
    pub fn with_arm(mut self, arm: Arm) -> Self {
        self.arm = arm;
        if matches!(arm, Arm::CarlPast { .. }) {
            self.detonate_tick = 0;
        }
        self
    }

    /// Whether the fuse is lit: always for `Arm::Tick`, from the arming for
    /// `Arm::CarlPast` (before it the charge is drawn as a still stick).
    pub fn lit(&self) -> bool {
        match self.arm {
            Arm::Tick => true,
            Arm::CarlPast { .. } => self.armed_tick.is_some(),
        }
    }

    /// One tick of the charge: true on the tick it detonates. `carl_x` is
    /// Carl's column (read only by `Arm::CarlPast`).
    pub fn step(&mut self, tick: u64, carl_x: f32) -> bool {
        match self.arm {
            Arm::Tick => self.detonate(tick),
            Arm::CarlPast { x, fuse_ticks } => {
                if self.detonated {
                    return false;
                }
                if self.armed_tick.is_none() && carl_x > x {
                    self.armed_tick = Some(tick);
                    self.detonate_tick = tick + fuse_ticks;
                }
                if self.armed_tick.is_some() && tick == self.detonate_tick {
                    self.detonated = true;
                    return true;
                }
                false
            }
        }
    }

    /// The blast centre `(x, y)` (y up).
    pub fn centre(&self) -> (f32, f32) {
        (self.x, self.foot_y + BLAST_CENTRE_DY)
    }

    /// Detonate on `tick`: true exactly once, on exactly the charge's tick
    /// (never before it, never again). Attempt-local ticks count 1, 2, 3 and
    /// are never skipped, so the tick always comes; a charge with tick 0 would
    /// never fire and is rejected when a fixture loads.
    pub fn detonate(&mut self, tick: u64) -> bool {
        if self.detonated || tick != self.detonate_tick {
            return false;
        }
        self.detonated = true;
        true
    }

    /// How the blast reaches `body`, if it does.
    pub fn reach(&self, body: Rect) -> Option<BlastReach> {
        let (cx, cy) = self.centre();
        let sq = body.squared_distance_to(cx, cy);
        if sq == 0.0 {
            Some(BlastReach::PointBlank)
        } else if sq <= BLAST_RADIUS * BLAST_RADIUS {
            Some(BlastReach::Ring)
        } else {
            None
        }
    }

    /// Fold the full state into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(0x4348_5247); // "CHRG"
        mix(u64::from(self.x.to_bits()));
        mix(u64::from(self.foot_y.to_bits()));
        mix(self.detonate_tick);
        mix(self.owner.map_or(0, |p| u64::from(p) + 1));
        mix(u64::from(self.detonated));
        // An armed-by-Carl charge adds its rule and state; a `Tick` charge
        // mixes nothing more, so every older hash is unchanged.
        if let Arm::CarlPast { x, fuse_ticks } = self.arm {
            mix(0x4152_4d44); // "ARMD"
            mix(u64::from(x.to_bits()));
            mix(fuse_ticks);
            mix(self.armed_tick.map_or(0, |t| t + 1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 24 x 38 body standing on `foot` with its centre column at `x`.
    fn body(x: f32, foot: f32) -> Rect {
        Rect {
            left: x - 12.0,
            bottom: foot,
            width: 24.0,
            height: 38.0,
        }
    }

    fn charge() -> BlastCharge {
        BlastCharge::new(500.0, 100.0, 60, None)
    }

    #[test]
    fn it_detonates_exactly_once_on_its_tick() {
        let mut c = charge();
        for tick in 1..60 {
            assert!(!c.detonate(tick), "not before tick 60 (tick {tick})");
        }
        assert!(!c.detonated);
        assert!(c.detonate(60));
        assert!(c.detonated);
        for tick in 60..200 {
            assert!(!c.detonate(tick), "never again (tick {tick})");
        }
        // Exact: a charge whose tick was never offered never fires late.
        let mut late = charge();
        assert!(!late.detonate(61));
        assert!(!late.detonated);
        // A tick-0 charge never fires (ticks count from 1).
        let mut zero = BlastCharge::new(500.0, 100.0, 0, None);
        assert!((1..100).all(|t| !zero.detonate(t)));
    }

    #[test]
    fn a_carl_past_charge_is_unlit_until_he_passes_then_blows_after_the_fuse() {
        let mut c = BlastCharge::new(500.0, 100.0, 0, None).with_arm(Arm::CarlPast {
            x: 480.0,
            fuse_ticks: 75,
        });
        assert!(!c.lit());
        // Carl at or behind the column: nothing happens, however long.
        for tick in 1..50 {
            assert!(!c.step(tick, 480.0));
        }
        assert!(!c.lit() && c.armed_tick.is_none());
        // He passes on tick 50: armed, and it blows exactly 75 ticks later.
        assert!(!c.step(50, 480.5));
        assert!(c.lit());
        assert_eq!((c.armed_tick, c.detonate_tick), (Some(50), 125));
        for tick in 51..125 {
            assert!(!c.step(tick, 700.0), "tick {tick}");
        }
        assert!(c.step(125, 700.0));
        assert!(c.detonated);
        for tick in 126..300 {
            assert!(!c.step(tick, 700.0), "never again");
        }
        // Walking back out and in again re-arms nothing.
        let mut once = BlastCharge::new(500.0, 100.0, 0, None).with_arm(Arm::CarlPast {
            x: 480.0,
            fuse_ticks: 3,
        });
        once.step(1, 490.0);
        once.step(2, 400.0);
        assert_eq!(once.armed_tick, Some(1));
    }

    #[test]
    fn the_arming_state_is_hashed_and_a_tick_charge_hashes_as_before() {
        let words = |c: &BlastCharge| {
            let mut out = Vec::new();
            c.mix_into(&mut |w| out.push(w));
            out
        };
        // The pre-arming words of a Tick charge: exactly six.
        assert_eq!(words(&charge()).len(), 6);
        let rule = Arm::CarlPast {
            x: 480.0,
            fuse_ticks: 75,
        };
        let a = BlastCharge::new(500.0, 100.0, 0, None).with_arm(rule);
        let mut b = a;
        b.step(7, 600.0);
        assert_eq!(words(&a).len(), 10);
        assert_ne!(words(&a), words(&b));
    }

    #[test]
    fn the_core_is_point_blank_the_ring_is_not_and_beyond_is_untouched() {
        let c = charge();
        // Centre (500, 108): inside a body standing on the charge.
        assert_eq!(c.reach(body(500.0, 100.0)), Some(BlastReach::PointBlank));
        // A body whose edge is exactly on the centre is inside (closed box).
        assert_eq!(c.reach(body(512.0, 100.0)), Some(BlastReach::PointBlank));
        // 16 px from the body edge (right edge 484): ring.
        assert_eq!(c.reach(body(472.0, 100.0)), Some(BlastReach::Ring));
        // Exactly 20.0 px away is a ring kill (compared as squares).
        assert_eq!(c.reach(body(468.0, 100.0)), Some(BlastReach::Ring));
        // 20.01 px away is untouched.
        assert_eq!(c.reach(body(467.99, 100.0)), None);
        // Mirror on the right.
        assert_eq!(c.reach(body(532.0, 100.0)), Some(BlastReach::Ring));
        assert_eq!(c.reach(body(532.01, 100.0)), None);
        // Above the body: the vertical distance counts too.
        let high = Rect {
            left: 488.0,
            bottom: 108.0 + 20.0,
            width: 24.0,
            height: 38.0,
        };
        assert_eq!(c.reach(high), Some(BlastReach::Ring));
        let higher = Rect {
            bottom: 108.0 + 20.5,
            ..high
        };
        assert_eq!(c.reach(higher), None);
    }

    #[test]
    fn the_state_hash_changes_with_every_field_and_the_owner_is_mixed() {
        let words = |c: &BlastCharge| {
            let mut out = Vec::new();
            c.mix_into(&mut |w| out.push(w));
            out
        };
        let base = charge();
        let mut fired = base;
        fired.detonate(60);
        assert_ne!(words(&base), words(&fired));
        let owned = BlastCharge::new(500.0, 100.0, 60, Some(0));
        assert_ne!(words(&base), words(&owned));
        assert_eq!(words(&base)[0], 0x4348_5247);
    }
}
