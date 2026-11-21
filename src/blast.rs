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
//! Disclosed limits of the **QA profile** ([`BlastProfile::QA`], every fixture
//! charge): no line of sight, no terrain damage, no friendly fire, no damage to
//! Carl or Donut. CARL-PROG-01 CP-B1 generalizes the charge with a
//! [`BlastProfile`]: the QA profile reproduces DEATH-D exactly, the thrown
//! charge's profile adds the outer ring, line of sight and self-damage (its
//! resolution lives in `combat_scene/thrown.rs`). The chain reaction and the
//! structure breaking that the profile's `breaks_structures` flag announces are
//! CP-C; CP-B1 only carries the flag.

use crate::hero_damage::HeroDamagePolicy;
use crate::prototype::{Block, Rect};

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
    /// Within the core radius ([`BLAST_RADIUS`] for the QA profile) but not
    /// inside: thrown dead (heavy).
    Ring,
    /// CP-B1: beyond the core and within the outer radius (CP-Q20, CP-Q21): one
    /// damage and a living launch. Only a profile with an outer band returns it.
    Outer,
}

/// CP-B1: the data of one blast (CARL-PROG-01 CP-Q20..CP-Q26, integration plan
/// K3). Every charge carries one; [`Self::QA`] is the DEATH-D rule. IP-neutral:
/// the thrown charge's profile is built from `carl_prog_v0.json`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlastProfile {
    /// Core radius in px: inside the body is point blank, up to it is a heavy
    /// ring kill. **0 means "no core band"**: nothing is point blank or a core
    /// ring kill, the outer result applies everywhere (a body that contains the
    /// centre under a coreless profile gets the outer result).
    pub core_radius: f32,
    /// Outer radius in px (the QA profile: equal to the core, so no outer band).
    pub outer_radius: f32,
    /// Damage of the outer ring to a goblin (a lethal one is a ring kill).
    pub outer_damage: u8,
    /// Damage of the outer ring to a llama (CP-Q26).
    pub llama_outer_damage: u8,
    /// Damage to a boss, anywhere in reach (CP-Q25).
    pub boss_damage: u8,
    /// The outer ring's living launch `(vx, vy)` in px/s, away from the centre.
    pub launch: (f32, f32),
    /// Whether solids block the blast (CP-Q22).
    pub los: bool,
    /// What the blast does to heroes (K3). The shared enum.
    pub hero_damage: HeroDamagePolicy,
    /// Damage a hurt hero takes (reduced for the owner under `OwnerReduced`).
    pub hero_hp: u8,
    /// The knock `(vx, vy)` of a hurt hero in px/s, away from the centre.
    pub hero_knock: (f32, f32),
    /// CP-C hook (CP-Q35): the blast breaks the authored supports. CP-B1 carries
    /// the flag and acts on nothing.
    pub breaks_structures: bool,
}

impl BlastProfile {
    /// The DEATH-D rule: a 20 px lethal ring, nothing else (the QA fixtures and
    /// DEMO-01's trip charge). Never mixed into a hash.
    pub const QA: Self = Self {
        core_radius: BLAST_RADIUS,
        outer_radius: BLAST_RADIUS,
        outer_damage: 0,
        llama_outer_damage: 0,
        boss_damage: 0,
        launch: (0.0, 0.0),
        los: false,
        hero_damage: HeroDamagePolicy::NoHeroes,
        hero_hp: 0,
        hero_knock: (0.0, 0.0),
        breaks_structures: false,
    };

    pub fn is_qa(&self) -> bool {
        *self == Self::QA
    }

    /// Fold the profile into a state hash: nothing for the QA profile, so a
    /// fixture charge hashes exactly as before.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        if self.is_qa() {
            return;
        }
        mix(0x5052_4f46); // "PROF"
        mix(u64::from(self.core_radius.to_bits()));
        mix(u64::from(self.outer_radius.to_bits()));
        mix(u64::from(self.outer_damage));
        mix(u64::from(self.llama_outer_damage));
        mix(u64::from(self.boss_damage));
        mix(u64::from(self.launch.0.to_bits()));
        mix(u64::from(self.launch.1.to_bits()));
        mix(u64::from(self.los));
        mix(self.hero_damage.code());
        mix(u64::from(self.hero_hp));
        mix(u64::from(self.hero_knock.0.to_bits()));
        mix(u64::from(self.hero_knock.1.to_bits()));
        mix(u64::from(self.breaks_structures));
    }
}

/// The point of `body` (a closed box) closest to `(x, y)`: the target end of the
/// line-of-sight segment (CP-Q22). A point inside the box is its own closest.
pub fn closest_point(body: Rect, (x, y): (f32, f32)) -> (f32, f32) {
    (
        x.clamp(body.left, body.right()),
        y.clamp(body.bottom, body.top()),
    )
}

/// CP-Q22: whether the segment `from -> to` is free of every intact solid: it
/// is blocked only by the **interior** of a block (a segment that merely
/// touches an edge or corner passes). Exact `f32` arithmetic, no square root.
pub fn line_of_sight(solids: &[Block], from: (f32, f32), to: (f32, f32)) -> bool {
    solids
        .iter()
        .filter(|block| block.intact)
        .all(|block| !segment_crosses_interior(from, to, block.rect))
}

/// The open interval of `t` (in `0..=1` parametrization of the segment) where
/// one coordinate lies strictly inside `(lo, hi)`: `None` is "never",
/// `Some((-inf, inf))` "always" (the segment is parallel to the slab and inside).
fn slab(a: f32, d: f32, lo: f32, hi: f32) -> Option<(f32, f32)> {
    if d == 0.0 {
        return (a > lo && a < hi).then_some((f32::NEG_INFINITY, f32::INFINITY));
    }
    let (t0, t1) = ((lo - a) / d, (hi - a) / d);
    Some(if t0 < t1 { (t0, t1) } else { (t1, t0) })
}

fn segment_crosses_interior(from: (f32, f32), to: (f32, f32), rect: Rect) -> bool {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let (Some(sx), Some(sy)) = (
        slab(from.0, dx, rect.left, rect.right()),
        slab(from.1, dy, rect.bottom, rect.top()),
    ) else {
        return false;
    };
    // The open parameter interval inside both slabs meets the closed segment
    // `[0, 1]` iff it is non-empty, starts before 1 and ends after 0.
    let open_lo = sx.0.max(sy.0);
    let open_hi = sx.1.min(sy.1);
    open_lo < open_hi && open_lo < 1.0 && open_hi > 0.0
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
    /// CP-B1: the blast's data. [`BlastProfile::QA`] for every fixture charge.
    pub profile: BlastProfile,
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
            profile: BlastProfile::QA,
        }
    }

    /// The same charge with another blast profile (CP-B1).
    #[must_use]
    pub fn with_profile(mut self, profile: BlastProfile) -> Self {
        self.profile = profile;
        self
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

    /// How the blast reaches `body`, if it does (squared compares, exact
    /// boundaries). The core band exists only when `core_radius > 0`: with no
    /// core a body that contains the centre gets the outer result, never point
    /// blank (CP-B1, `core_zero_never_point_blank`).
    pub fn reach(&self, body: Rect) -> Option<BlastReach> {
        let (cx, cy) = self.centre();
        let sq = body.squared_distance_to(cx, cy);
        let core = self.profile.core_radius;
        let outer = self.profile.outer_radius;
        if core > 0.0 && sq == 0.0 {
            Some(BlastReach::PointBlank)
        } else if core > 0.0 && sq <= core * core {
            Some(BlastReach::Ring)
        } else if sq <= outer * outer {
            Some(BlastReach::Outer)
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
        // A non-QA profile adds its words after everything above (a QA charge
        // mixes nothing more, so every older hash is unchanged).
        self.profile.mix_into(mix);
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

    /// A dynamite-like profile for the unit tests (the real one is built from
    /// the knob file in `thrown_charge`).
    fn wide() -> BlastProfile {
        BlastProfile {
            core_radius: 20.0,
            outer_radius: 64.0,
            outer_damage: 1,
            llama_outer_damage: 1,
            boss_damage: 4,
            launch: (200.0, 220.0),
            los: true,
            hero_damage: HeroDamagePolicy::OwnerReduced,
            hero_hp: 1,
            hero_knock: (120.0, 160.0),
            breaks_structures: true,
        }
    }

    #[test]
    fn the_qa_profile_hashes_byte_identically_to_a_charge_with_no_profile() {
        let words = |c: &BlastCharge| {
            let mut out = Vec::new();
            c.mix_into(&mut |w| out.push(w));
            out
        };
        // The QA profile is what `new` gives: six words for a Tick charge, ten
        // for a CarlPast one, exactly as before CP-B1.
        let tick = charge();
        assert!(tick.profile.is_qa());
        assert_eq!(words(&tick).len(), 6);
        let mut nothing = Vec::new();
        BlastProfile::QA.mix_into(&mut |w| nothing.push(w));
        assert!(nothing.is_empty(), "the QA profile mixes nothing");
        // Any other profile adds words after them and changes the hash.
        let dynamite = tick.with_profile(wide());
        assert!(words(&dynamite).len() > 6);
        assert_eq!(words(&dynamite)[..6], words(&tick)[..]);
        let mut other = wide();
        other.outer_radius = 65.0;
        assert_ne!(words(&dynamite), words(&tick.with_profile(other)));
    }

    #[test]
    fn core_zero_never_point_blank() {
        // A coreless profile: the body that contains the centre gets the outer
        // result, never point blank, and there is no core ring either.
        let coreless = charge().with_profile(BlastProfile {
            core_radius: 0.0,
            ..wide()
        });
        assert_eq!(coreless.reach(body(500.0, 100.0)), Some(BlastReach::Outer));
        assert_eq!(coreless.reach(body(472.0, 100.0)), Some(BlastReach::Outer));
        assert_eq!(coreless.reach(body(467.99, 100.0)), Some(BlastReach::Outer));
        // Beyond the outer radius it is still untouched.
        assert_eq!(coreless.reach(body(500.0 - 12.0 - 64.01, 100.0)), None);
    }

    #[test]
    fn a_wide_profile_has_a_core_an_outer_band_and_exact_edges() {
        let c = charge().with_profile(wide());
        assert_eq!(c.reach(body(500.0, 100.0)), Some(BlastReach::PointBlank));
        assert_eq!(
            c.reach(body(468.0, 100.0)),
            Some(BlastReach::Ring),
            "exactly 20 px"
        );
        assert_eq!(c.reach(body(467.99, 100.0)), Some(BlastReach::Outer));
        // Exactly 64 px from the body edge is reached, beyond is not.
        assert_eq!(
            c.reach(body(500.0 - 12.0 - 64.0, 100.0)),
            Some(BlastReach::Outer)
        );
        assert_eq!(c.reach(body(500.0 - 12.0 - 64.01, 100.0)), None);
        assert_eq!(
            c.reach(body(500.0 + 12.0 + 64.0, 100.0)),
            Some(BlastReach::Outer)
        );
        assert_eq!(c.reach(body(500.0 + 12.0 + 64.01, 100.0)), None);
        // The QA charge still has no outer band at 40 px.
        assert_eq!(charge().reach(body(500.0 - 12.0 - 40.0, 100.0)), None);
    }

    fn wall(left: f32, bottom: f32, width: f32, height: f32) -> Block {
        Block {
            rect: Rect {
                left,
                bottom,
                width,
                height,
            },
            material: crate::prototype::Material::Stone,
            intact: true,
        }
    }

    #[test]
    fn line_of_sight_is_blocked_only_by_the_interior_of_an_intact_solid() {
        let from = (100.0, 108.0);
        let to = (200.0, 108.0);
        // Nothing in the way.
        assert!(line_of_sight(&[], from, to));
        // A wall across the segment blocks it, a broken one does not.
        let mut across = wall(140.0, 100.0, 20.0, 60.0);
        assert!(!line_of_sight(&[across.clone()], from, to));
        across.intact = false;
        assert!(line_of_sight(&[across], from, to));
        // Touching edges do not block: along the top of a floor, and along a
        // wall's face or past its corner.
        assert!(
            line_of_sight(&[wall(0.0, 0.0, 400.0, 108.0)], from, to),
            "along a top"
        );
        assert!(
            line_of_sight(&[wall(150.0, 0.0, 20.0, 108.0)], from, to),
            "grazing a top corner"
        );
        assert!(
            line_of_sight(&[wall(100.0, 108.0, 100.0, 20.0)], from, to),
            "along an underside"
        );
        assert!(
            line_of_sight(&[wall(150.0, 108.0, 20.0, 50.0)], from, to),
            "touching a wall's bottom edge"
        );
        // A segment that ends exactly on a wall face is not blocked...
        assert!(line_of_sight(&[wall(200.0, 90.0, 20.0, 40.0)], from, to));
        // ...one that ends a hair inside is.
        assert!(!line_of_sight(&[wall(199.5, 90.0, 20.0, 40.0)], from, to));
        // A diagonal that clips a corner interior is blocked, one that only
        // touches the corner point is not.
        assert!(!line_of_sight(
            &[wall(140.0, 100.0, 20.0, 20.0)],
            (100.0, 90.0),
            (200.0, 130.0)
        ));
        assert!(line_of_sight(
            &[wall(140.0, 100.0, 20.0, 20.0)],
            (100.0, 80.0),
            (140.0, 100.0)
        ));
        // A degenerate segment (the centre inside the target) inside a solid is blocked.
        assert!(!line_of_sight(&[wall(0.0, 0.0, 400.0, 400.0)], from, from));
        assert!(line_of_sight(&[wall(0.0, 0.0, 400.0, 100.0)], from, from));
    }

    #[test]
    fn the_closest_point_clamps_into_the_box() {
        let b = body(500.0, 100.0);
        assert_eq!(closest_point(b, (400.0, 108.0)), (488.0, 108.0));
        assert_eq!(closest_point(b, (500.0, 300.0)), (500.0, 138.0));
        assert_eq!(
            closest_point(b, (500.0, 108.0)),
            (500.0, 108.0),
            "inside: itself"
        );
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
