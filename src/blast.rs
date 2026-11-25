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
//! resolution lives in `combat_scene/thrown.rs`). CP-C adds the chain reaction
//! ([`BlastCharge::chain_set`], CP-Q34) and the structure hook ([`BlastTarget`],
//! [`blast_break`], CP-Q35): a blast whose profile has `breaks_structures` breaks
//! what implements the hook, never an env solid by index range (KC17).

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
    /// The `HeroHurt` source id this blast logs on a hero it hurts: one id per
    /// setpiece (integration plan section 3), so a world blast routed through
    /// this profile never logs another setpiece's id. 0 for the QA profile.
    pub hero_source: u32,
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
        hero_source: 0,
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
        mix(u64::from(self.hero_source));
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

/// CP-Q22: whether the segment `from -> to` is free of every intact solid. It
/// is blocked by the **interior** of a block and (BLAST-SEAM) by a **seam**: an
/// axis-aligned stretch of positive length along an edge that two intact blocks
/// share from opposite sides (one stacked on the other, two side by side, a
/// shutter's top flush with a walkway's underside), where the two together are
/// solid. A segment that merely touches the outer edge or a corner of the
/// solids passes. Exact `f32` arithmetic, no square root.
pub fn line_of_sight(solids: &[Block], from: (f32, f32), to: (f32, f32)) -> bool {
    solids
        .iter()
        .filter(|block| block.intact)
        .all(|block| !segment_crosses_interior(from, to, block.rect))
        && !runs_along_a_seam(solids, from, to)
}

/// BLAST-SEAM: whether an axis-aligned segment runs, over a positive length,
/// along an edge shared by two intact blocks on its two sides. A horizontal
/// segment at `y` needs a block whose top is `y` and one whose bottom is `y`, a
/// vertical one at `x` a block whose right is `x` and one whose left is `x`; the
/// open overlap of the segment's span and both blocks' spans must be non-empty
/// (blocks that touch only corner to corner, or a segment that only reaches the
/// seam's end point, do not count). A block with no area is never solid. Testing
/// the pairs suffices: the solid part of the line is the union of their overlaps.
/// A diagonal or degenerate segment never runs along a seam (an oblique one
/// that crosses a seam enters an interior on one side of it).
fn runs_along_a_seam(solids: &[Block], from: (f32, f32), to: (f32, f32)) -> bool {
    type Edge = fn(Rect) -> f32;
    type Span = fn(Rect) -> (f32, f32);
    let seam = |line: f32, (lo, hi): (f32, f32), low_side: Edge, high_side: Edge, span: Span| {
        let solid = || {
            solids
                .iter()
                .filter(|b| b.intact && b.rect.width > 0.0 && b.rect.height > 0.0)
                .map(|b| b.rect)
        };
        solid().filter(|a| low_side(*a) == line).any(|a| {
            let (a_lo, a_hi) = span(a);
            solid().filter(|b| high_side(*b) == line).any(|b| {
                let (b_lo, b_hi) = span(b);
                lo.max(a_lo).max(b_lo) < hi.min(a_hi).min(b_hi)
            })
        })
    };
    if from.1 == to.1 && from.0 != to.0 {
        seam(
            from.1,
            (from.0.min(to.0), from.0.max(to.0)),
            Rect::top,
            |r| r.bottom,
            |r| (r.left, r.right()),
        )
    } else if from.0 == to.0 && from.1 != to.1 {
        seam(
            from.0,
            (from.1.min(to.1), from.1.max(to.1)),
            Rect::right,
            |r| r.left,
            |r| (r.bottom, r.top()),
        )
    } else {
        false
    }
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
    /// CP-C (CP-Q34): how many chain links lie between the dynamite's own blast
    /// (depth 0) and this charge. 0 for every charge that no chain has set
    /// (hashed only while non-zero, so every older hash is unchanged).
    pub chain_depth: u8,
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
            chain_depth: 0,
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

    /// CP-Q34: whether this charge detonates on `tick` (lit, not yet blown, due
    /// exactly then). The per-tick cap defers on this test.
    pub fn due_on(&self, tick: u64) -> bool {
        !self.detonated && self.lit() && self.detonate_tick == tick
    }

    /// CP-Q34 (the per-tick cap): push a charge that is due on `tick` to the next
    /// tick. It keeps its depth and its owner; stable index order decides which
    /// charges the cap defers.
    pub fn defer_one_tick(&mut self) {
        self.detonate_tick += 1;
    }

    /// CP-Q34: a chain reaches this charge on `now`, from a blast of `depth - 1`.
    ///
    /// **Reached** and **scheduled** are two separate things.
    ///
    /// * Every reached charge that has not detonated takes the chain's credit and
    ///   depth, **first writer keeps**: `owner` only when it is `None`, `chain_depth`
    ///   only when it is 0. This holds for a charge left alone and for one due on
    ///   `now` (not yet stepped, possibly deferred by the per-tick cap), so its own
    ///   blast is still the chain's and chains onward (CP-Q34: "a charge it reaches
    ///   is depth 1 ... a chained charge chains further").
    /// * The schedule: a blown charge is deduped (nothing at all changes; each
    ///   charge detonates once). A charge that blows on `now` keeps its tick. An
    ///   **unlit** charge (an unarmed `Arm::CarlPast`, or a stored `detonate_tick`
    ///   of 0 that has not detonated) is set to `now + delay`; an unarmed
    ///   `CarlPast` is armed on `now`, so its tick is never read as "earlier". A lit
    ///   charge due later than `now + delay` is pulled forward to it; one due
    ///   inside `now < tick <= now + delay` is left alone.
    ///
    /// Returns the new `detonate_tick` when the schedule changed.
    pub fn chain_set(
        &mut self,
        now: u64,
        delay: u64,
        owner: Option<u32>,
        depth: u8,
    ) -> Option<u64> {
        if self.detonated {
            return None;
        }
        if self.owner.is_none() {
            self.owner = owner;
        }
        if self.chain_depth == 0 {
            self.chain_depth = depth;
        }
        if self.lit() && self.detonate_tick == now {
            return None;
        }
        let due = now + delay;
        let unlit = match self.arm {
            Arm::Tick => self.detonate_tick == 0,
            Arm::CarlPast { .. } => self.armed_tick.is_none(),
        };
        // A stored tick at or before `now` is never a future detonation: it is
        // read as unlit (`detonate_tick <= now` gives `now + delay`).
        let stale = self.detonate_tick < now;
        if unlit || stale || self.detonate_tick > due {
            if matches!(self.arm, Arm::CarlPast { .. }) && self.armed_tick.is_none() {
                self.armed_tick = Some(now);
            }
            self.detonate_tick = due;
            return Some(due);
        }
        None
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
        // CP-C: the chain depth, only while a chain has set it.
        if self.chain_depth > 0 {
            mix(0x4348_4e44); // "CHND"
            mix(u64::from(self.chain_depth));
        }
    }
}

/// CP-C (CP-Q34): the chain rule's knobs, read from `carl_prog_v0.json`
/// (`cpq34_*`) by the caller; this module stays free of the tuning file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainRule {
    /// Ticks between the reaching blast and the chained detonation (8).
    pub delay: u64,
    /// The deepest link: a blast at this depth chains nothing (4).
    pub max_depth: u8,
    /// Detonations allowed on one tick; the excess is deferred (8).
    pub max_per_tick: usize,
}

/// What a blast hands to a structure it breaks (CP-Q35, K-plan section 3's
/// generic hook). It carries only what a blast has, never a state of the
/// target: `blast_id` is the one id of this blast (for a target's dedupe),
/// `owner` the credit and `depth` the chain depth of the blast (NB-4 of #200:
/// a prop's link graph can continue the count).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlastBreak {
    pub blast_id: u32,
    pub owner: Option<u32>,
    pub depth: u8,
}

/// CP-C: the generic structure hook (integration plan section 3, KC17). A thing
/// a blast can break, as a box and a break call: the CP-03 shore and the boss
/// rack strut implement it here; BOIL-D's props implement it by forwarding to
/// `env_prop::Mass::break_by_blast`. A blast reaches it only through
/// [`blast_break`] (`breaks_structures`, the outer circle, line of sight).
pub trait BlastTarget {
    /// The box the blast must reach, or `None` when there is nothing left to
    /// break (already broken, not armed, not enabled).
    fn blast_footprint(&self) -> Option<Rect>;
    /// Break it. Returns true when it broke on this call.
    fn break_by_blast(&mut self, blast: BlastBreak) -> bool;
}

impl BlastCharge {
    /// Whether this blast reaches a structure box: the profile breaks structures,
    /// the box lies within the outer circle (squared compare, touching counts)
    /// and the segment from the centre to its closest point is free of solids
    /// (CP-Q22; evaluated before any break of the tick).
    pub fn reaches_structure(&self, solids: &[Block], footprint: Rect) -> bool {
        if !self.profile.breaks_structures {
            return false;
        }
        let (cx, cy) = self.centre();
        let outer = self.profile.outer_radius;
        footprint.squared_distance_to(cx, cy) <= outer * outer
            && line_of_sight(solids, (cx, cy), closest_point(footprint, (cx, cy)))
    }
}

/// CP-Q35: break `target` with `charge`'s blast when it reaches it. Returns true
/// when the target broke. The blast never reads the env-solid index range: a
/// prop is a [`BlastTarget`] of its own (KC17).
pub fn blast_break(
    charge: &BlastCharge,
    blast_id: u32,
    solids: &[Block],
    target: &mut dyn BlastTarget,
) -> bool {
    let Some(footprint) = target.blast_footprint() else {
        return false;
    };
    charge.reaches_structure(solids, footprint)
        && target.break_by_blast(BlastBreak {
            blast_id,
            owner: charge.owner,
            depth: charge.chain_depth,
        })
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
            hero_source: 0x5443,
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
        // The hero-hurt source id is part of a non-QA profile's hash.
        let mut sourced = wide();
        sourced.hero_source = 0x5254;
        assert_ne!(words(&dynamite), words(&tick.with_profile(sourced)));
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
    fn line_of_sight_is_blocked_by_the_interior_of_an_intact_solid_not_by_one_edge() {
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

    /// BLAST-SEAM: a segment exactly along an edge two intact blocks share from
    /// opposite sides is blocked (the union is solid there); along the outer
    /// edge of one block, or of two blocks on the same side, it still passes.
    #[test]
    fn line_of_sight_is_blocked_along_a_seam_between_two_intact_blocks() {
        // A horizontal seam at y 108: a 4 px slice stacked on another.
        let low = wall(140.0, 104.0, 20.0, 4.0);
        let high = wall(140.0, 108.0, 20.0, 4.0);
        let (from, to) = ((100.0, 108.0), (200.0, 108.0));
        assert!(
            line_of_sight(std::slice::from_ref(&low), from, to),
            "a single top"
        );
        assert!(
            line_of_sight(std::slice::from_ref(&high), from, to),
            "a single underside"
        );
        assert!(
            !line_of_sight(&[low.clone(), high.clone()], from, to),
            "the stacked seam"
        );
        assert!(
            !line_of_sight(&[high.clone(), low.clone()], from, to),
            "in either order"
        );
        // Either half broken: the seam is an outer edge again.
        let mut broken = high.clone();
        broken.intact = false;
        assert!(line_of_sight(&[low.clone(), broken], from, to));
        // The segment must run along the shared part for a positive length:
        // one that ends on the seam's first point passes, a hair further is
        // blocked; one wholly beside the seam passes; one in the middle of it
        // (both ends on the seam) is blocked.
        assert!(line_of_sight(
            &[low.clone(), high.clone()],
            from,
            (140.0, 108.0)
        ));
        assert!(!line_of_sight(
            &[low.clone(), high.clone()],
            from,
            (140.5, 108.0)
        ));
        assert!(line_of_sight(
            &[low.clone(), high.clone()],
            from,
            (130.0, 108.0)
        ));
        assert!(!line_of_sight(
            &[low.clone(), high.clone()],
            (145.0, 108.0),
            (150.0, 108.0)
        ));
        // Offset halves: only their common span (150..160) is a seam.
        let shifted = wall(150.0, 108.0, 30.0, 4.0);
        assert!(!line_of_sight(&[low.clone(), shifted.clone()], from, to));
        assert!(line_of_sight(
            &[low.clone(), shifted.clone()],
            from,
            (150.0, 108.0)
        ));
        assert!(line_of_sight(&[low.clone(), shifted], (160.0, 108.0), to));
        // Halves that only touch corner to corner share no edge.
        assert!(line_of_sight(
            &[wall(100.0, 104.0, 40.0, 4.0), wall(140.0, 108.0, 40.0, 4.0)],
            (90.0, 108.0),
            (190.0, 108.0)
        ));
        // Two blocks side by side on the same side: a top, not a seam.
        assert!(line_of_sight(
            &[wall(100.0, 100.0, 50.0, 8.0), wall(150.0, 100.0, 50.0, 8.0)],
            from,
            to
        ));
        // A block with no area is not solid, even on itself.
        assert!(line_of_sight(&[wall(100.0, 108.0, 100.0, 0.0)], from, to));

        // A vertical seam at x 150: two blocks side by side.
        let left = wall(130.0, 100.0, 20.0, 40.0);
        let right = wall(150.0, 100.0, 20.0, 40.0);
        let (up, down) = ((150.0, 160.0), (150.0, 80.0));
        assert!(
            line_of_sight(std::slice::from_ref(&left), up, down),
            "a single right face"
        );
        assert!(
            line_of_sight(std::slice::from_ref(&right), up, down),
            "a single left face"
        );
        assert!(!line_of_sight(&[left.clone(), right.clone()], up, down));
        assert!(!line_of_sight(&[right.clone(), left.clone()], down, up));
        assert!(line_of_sight(
            &[left.clone(), right.clone()],
            up,
            (150.0, 140.0)
        ));
        assert!(line_of_sight(&[left, right], (150.0, 100.0), down));
    }

    /// BLAST-SEAM at a T-junction: a wide block under two side by side. The
    /// horizontal seam under either one blocks, and so does the vertical seam
    /// between the two upper blocks; the stem of the T past their tops is a
    /// plain outer edge again (nothing on its right).
    #[test]
    fn line_of_sight_is_blocked_along_both_seams_of_a_t_junction() {
        let base = wall(100.0, 100.0, 100.0, 8.0);
        let a = wall(100.0, 108.0, 50.0, 20.0);
        let b = wall(150.0, 108.0, 50.0, 20.0);
        let t = [base.clone(), a.clone(), b.clone()];
        assert!(!line_of_sight(&t, (90.0, 108.0), (120.0, 108.0)), "under a");
        assert!(
            !line_of_sight(&t, (180.0, 108.0), (210.0, 108.0)),
            "under b"
        );
        assert!(!line_of_sight(&t, (150.0, 140.0), (150.0, 110.0)), "a | b");
        assert!(
            !line_of_sight(&t, (150.0, 140.0), (150.0, 108.0)),
            "to the junction"
        );
        // Without b the stem is a's right face over the base's top.
        let stem = [base, a];
        assert!(line_of_sight(&stem, (150.0, 140.0), (150.0, 108.0)));
        assert!(
            line_of_sight(&stem, (150.0, 108.0), (210.0, 108.0)),
            "the base's top"
        );
        assert!(
            !line_of_sight(&stem, (90.0, 108.0), (210.0, 108.0)),
            "under a"
        );
    }

    /// BLAST-SEAM, unchanged: a segment that grazes the outer edge of one block
    /// (along its top, its underside, either face) or of a row of blocks on the
    /// same side still passes, whatever lies elsewhere.
    #[test]
    fn line_of_sight_along_a_single_outer_edge_is_unchanged() {
        let block = wall(140.0, 100.0, 20.0, 20.0);
        let far = wall(400.0, 120.0, 20.0, 20.0);
        let solids = [block, far];
        assert!(
            line_of_sight(&solids, (100.0, 120.0), (200.0, 120.0)),
            "top"
        );
        assert!(
            line_of_sight(&solids, (100.0, 100.0), (200.0, 100.0)),
            "underside"
        );
        assert!(
            line_of_sight(&solids, (140.0, 80.0), (140.0, 140.0)),
            "left face"
        );
        assert!(
            line_of_sight(&solids, (160.0, 80.0), (160.0, 140.0)),
            "right face"
        );
        // `far`'s underside is on the line y 120 too, but not over this span.
        assert!(line_of_sight(&solids, (100.0, 120.0), (400.0, 120.0)));
    }

    /// BLAST-SEAM at the Ring court's shutter: a static stack (32 x 68) under
    /// a 12 px walkway collider whose underside is flush with the stack's top.
    /// A blast exactly at that height is cut by the seam over the stack; the
    /// same line clear of the stack grazes only the walkway's underside.
    #[test]
    fn a_blast_on_the_shutter_top_and_walkway_underside_line_is_blocked() {
        let floor = 0.0;
        let shutter = wall(1200.0, floor, 32.0, 68.0);
        let walkway = wall(1120.0, floor + 68.0, 192.0, 12.0);
        let solids = [shutter, walkway];
        let y = floor + 68.0;
        assert!(!line_of_sight(&solids, (1180.0, y), (1252.0, y)), "across");
        assert!(!line_of_sight(&solids, (1252.0, y), (1180.0, y)), "back");
        assert!(
            line_of_sight(&solids, (1240.0, y), (1300.0, y)),
            "beside it"
        );
        // A sliced stack (17 slices of 4 px) blocks the floor charge's line,
        // centre 8 px up on a slice seam, as the one block does.
        let slices: Vec<Block> = (0..17)
            .map(|k| wall(1200.0, floor + 4.0 * k as f32, 32.0, 4.0))
            .collect();
        for k in 1..17 {
            let y = floor + 4.0 * k as f32;
            assert!(
                !line_of_sight(&slices, (1180.0, y), (1252.0, y)),
                "seam {k}"
            );
        }
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

    // ------------------------------------------------ CP-C (CP-Q34, CP-Q35) --

    const DELAY: u64 = 8;

    fn lit_at(tick: u64) -> BlastCharge {
        BlastCharge::new(500.0, 100.0, tick, None)
    }

    #[test]
    fn chain_an_unlit_charge_is_set_to_now_plus_8_and_a_stored_tick_of_0_is_never_earlier() {
        // An unlit tick charge keeps `detonate_tick` 0 (it never fires on its own).
        let mut c = lit_at(0);
        assert_eq!(c.chain_set(50, DELAY, Some(0), 1), Some(58));
        assert_eq!((c.detonate_tick, c.chain_depth, c.owner), (58, 1, Some(0)));
        assert!(!c.step(57, 0.0));
        assert!(c.step(58, 0.0), "blows exactly 8 ticks later");
        // The same on the very first ticks: 0 <= now gives now + 8.
        let mut early = lit_at(0);
        assert_eq!(early.chain_set(1, DELAY, None, 1), Some(9));
    }

    #[test]
    fn chain_a_carl_past_charge_that_is_unarmed_is_armed_not_read_as_due() {
        let rule = Arm::CarlPast {
            x: 480.0,
            fuse_ticks: 75,
        };
        let mut c = BlastCharge::new(500.0, 100.0, 0, None).with_arm(rule);
        assert!(!c.lit());
        assert_eq!(c.chain_set(120, DELAY, None, 2), Some(128));
        assert!(c.lit() && c.armed_tick == Some(120));
        // Carl far behind its column: the chain still blows it on tick 128.
        for tick in 121..128 {
            assert!(!c.step(tick, 0.0), "tick {tick}");
        }
        assert!(c.step(128, 0.0));
        // An armed one keeps its own rule (due inside the delay: left alone).
        let mut armed = BlastCharge::new(500.0, 100.0, 0, None).with_arm(rule);
        armed.step(10, 600.0);
        assert_eq!(armed.detonate_tick, 85);
        assert_eq!(
            armed.chain_set(80, DELAY, None, 1),
            None,
            "85 is inside 80..=88"
        );
        assert_eq!(armed.detonate_tick, 85);
    }

    #[test]
    fn chain_a_lit_charge_due_inside_the_delay_is_left_and_one_due_later_is_pulled() {
        let now = 100;
        // Due exactly on `now + 1` .. `now + 8` (both edges): left alone.
        for due in [101, 104, 108] {
            let mut c = lit_at(due);
            assert_eq!(c.chain_set(now, DELAY, Some(0), 1), None, "due {due}");
            // Reached but not rescheduled: the depth and the credit are still the
            // chain's, so this charge's own blast chains onward (B-1).
            assert_eq!((c.detonate_tick, c.chain_depth, c.owner), (due, 1, Some(0)));
        }
        // Due on `now + 9` and later: pulled to `now + 8`.
        for due in [109, 110, 400] {
            let mut c = lit_at(due);
            assert_eq!(c.chain_set(now, DELAY, Some(0), 1), Some(108), "due {due}");
            assert_eq!((c.detonate_tick, c.chain_depth), (108, 1));
        }
    }

    #[test]
    fn chain_dedupes_a_charge_that_blew_or_blows_this_tick() {
        let mut blown = lit_at(60);
        assert!(blown.detonate(60));
        assert_eq!(blown.chain_set(70, DELAY, Some(0), 1), None);
        assert_eq!(blown.owner, None, "a blown charge takes no credit");
        assert_eq!(blown.chain_depth, 0);
        // Due on `now` itself (a higher index that is still to step this tick).
        let mut now = lit_at(70);
        assert_eq!(now.chain_set(70, DELAY, Some(0), 1), None);
        assert_eq!(now.detonate_tick, 70, "it still blows this tick");
        // Not yet stepped this tick (a higher index, possibly cap-deferred): it is
        // reached, so it takes the credit and the depth even though its tick is kept.
        assert_eq!((now.chain_depth, now.owner), (1, Some(0)));
        assert!(now.step(70, 0.0));
    }

    #[test]
    fn chain_writes_the_owner_only_when_it_is_empty() {
        let mut free = lit_at(0);
        free.chain_set(10, DELAY, Some(1), 1);
        assert_eq!(free.owner, Some(1));
        // The first initiator keeps the credit, scheduled or left alone.
        let mut owned = BlastCharge::new(500.0, 100.0, 0, Some(0));
        owned.chain_set(10, DELAY, Some(1), 1);
        assert_eq!(owned.owner, Some(0));
        let mut sooner = BlastCharge::new(500.0, 100.0, 14, Some(0));
        assert_eq!(sooner.chain_set(10, DELAY, Some(1), 1), None);
        assert_eq!(sooner.owner, Some(0));
        let mut unowned_sooner = lit_at(14);
        assert_eq!(unowned_sooner.chain_set(10, DELAY, Some(1), 1), None);
        assert_eq!(unowned_sooner.owner, Some(1), "an empty owner is filled");
        // `None` never overwrites.
        let mut keep = BlastCharge::new(500.0, 100.0, 0, Some(1));
        keep.chain_set(10, DELAY, None, 1);
        assert_eq!(keep.owner, Some(1));
    }

    #[test]
    fn chain_leaves_a_cap_deferred_charge_to_the_cap_and_reschedules_a_stale_one() {
        // A charge the per-tick cap pushed to `now + 1` is inside the window: the
        // chain leaves it (its depth and owner were set when it was first chained).
        let mut deferred = lit_at(0);
        assert_eq!(deferred.chain_set(100, DELAY, Some(0), 2), Some(108));
        deferred.detonate_tick = 108;
        deferred.defer_one_tick();
        assert_eq!(deferred.detonate_tick, 109);
        assert_eq!(deferred.chain_set(108, DELAY, Some(1), 3), None);
        assert_eq!((deferred.detonate_tick, deferred.chain_depth), (109, 2));
        assert_eq!(
            deferred.owner,
            Some(0),
            "the first initiator keeps the credit"
        );
        // A stored tick that is in the past and never blew (it cannot arise in a
        // stepped run) is read as unlit, never as "earlier".
        let mut stale = lit_at(40);
        assert_eq!(stale.chain_set(100, DELAY, None, 1), Some(108));
        // A stored tick of exactly `now` blows this tick: nothing changes.
        let mut now = lit_at(100);
        assert_eq!(now.chain_set(100, DELAY, None, 1), None);
        assert_eq!(now.detonate_tick, 100);
    }

    #[test]
    fn chain_a_reached_charge_that_is_left_alone_or_cap_deferred_keeps_the_chain_going() {
        // Left alone (due inside the window): depth 1, credit filled, no new tick.
        let mut alone = lit_at(104);
        assert_eq!(alone.chain_set(100, DELAY, Some(0), 1), None);
        assert_eq!(
            (alone.detonate_tick, alone.chain_depth, alone.owner),
            (104, 1, Some(0))
        );
        // Its own blast then chains the next charge at depth 2.
        let mut next = lit_at(4000);
        assert_eq!(
            next.chain_set(104, DELAY, alone.owner, alone.chain_depth + 1),
            Some(112)
        );
        assert_eq!((next.chain_depth, next.owner), (2, Some(0)));
        // Due on `now` and then pushed to `now + 1` by the cap: it was reached on
        // `now`, so the deferral keeps the depth and the credit.
        let mut due_now = lit_at(100);
        assert_eq!(due_now.chain_set(100, DELAY, Some(1), 3), None);
        due_now.defer_one_tick();
        assert_eq!(
            (due_now.detonate_tick, due_now.chain_depth, due_now.owner),
            (101, 3, Some(1))
        );
        // First writer keeps both: a later, shallower chain changes neither.
        assert_eq!(due_now.chain_set(100, DELAY, Some(0), 1), None);
        assert_eq!((due_now.chain_depth, due_now.owner), (3, Some(1)));
        // The same for a charge that was scheduled by the first chain.
        let mut first = lit_at(4000);
        assert_eq!(first.chain_set(100, DELAY, Some(0), 2), Some(108));
        assert_eq!(first.chain_set(100, DELAY, Some(1), 1), None);
        assert_eq!((first.chain_depth, first.owner), (2, Some(0)));
    }

    #[test]
    fn the_chain_depth_is_hashed_only_while_non_zero() {
        let words = |c: &BlastCharge| {
            let mut out = Vec::new();
            c.mix_into(&mut |w| out.push(w));
            out
        };
        let plain = lit_at(60);
        assert_eq!(words(&plain).len(), 6, "a depth-0 charge hashes as before");
        let mut deep = plain;
        deep.chain_depth = 2;
        assert_eq!(words(&deep).len(), 8);
        assert_ne!(words(&deep), words(&plain));
    }

    #[test]
    fn the_cap_helpers_defer_exactly_the_due_charge() {
        let mut c = lit_at(30);
        assert!(c.due_on(30) && !c.due_on(29) && !c.due_on(31));
        c.defer_one_tick();
        assert!(c.due_on(31) && !c.due_on(30));
        let mut blown = lit_at(30);
        blown.detonate(30);
        assert!(!blown.due_on(30));
        let unarmed = BlastCharge::new(500.0, 100.0, 0, None).with_arm(Arm::CarlPast {
            x: 1.0,
            fuse_ticks: 3,
        });
        assert!(!unarmed.due_on(0), "an unlit charge is never due");
    }

    // The structure hook.

    struct Wall {
        footprint: Option<Rect>,
        broken_by: Vec<BlastBreak>,
    }

    impl BlastTarget for Wall {
        fn blast_footprint(&self) -> Option<Rect> {
            self.footprint
        }

        fn break_by_blast(&mut self, blast: BlastBreak) -> bool {
            self.broken_by.push(blast);
            true
        }
    }

    fn structure_profile() -> BlastProfile {
        BlastProfile {
            core_radius: 20.0,
            outer_radius: 64.0,
            los: true,
            breaks_structures: true,
            ..BlastProfile::QA
        }
    }

    fn box_at(x: f32) -> Rect {
        Rect {
            left: x,
            bottom: 108.0,
            width: 16.0,
            height: 16.0,
        }
    }

    #[test]
    fn a_blast_breaks_a_structure_only_with_the_flag_inside_64_px_and_with_sight() {
        // Centre (500, 108).
        let c = charge().with_profile(structure_profile());
        let mut wall = Wall {
            footprint: Some(box_at(564.0)),
            broken_by: Vec::new(),
        };
        assert!(blast_break(&c, 7, &[], &mut wall), "exactly 64 px reaches");
        assert_eq!(
            wall.broken_by,
            vec![BlastBreak {
                blast_id: 7,
                owner: None,
                depth: 0
            }]
        );
        let mut far = Wall {
            footprint: Some(box_at(564.01)),
            broken_by: Vec::new(),
        };
        assert!(!blast_break(&c, 7, &[], &mut far), "64.01 px does not");
        assert!(far.broken_by.is_empty());
        // Nothing left to break: the hook is not called.
        let mut gone = Wall {
            footprint: None,
            broken_by: Vec::new(),
        };
        assert!(!blast_break(&c, 7, &[], &mut gone));
        // The QA profile never breaks a structure, however close.
        let qa = charge();
        let mut near = Wall {
            footprint: Some(box_at(500.0)),
            broken_by: Vec::new(),
        };
        assert!(!blast_break(&qa, 7, &[], &mut near));
        assert!(near.broken_by.is_empty());
        // A solid between the centre and the box blocks it; touching does not.
        let pillar = |left: f32, width: f32| Block {
            rect: Rect {
                left,
                bottom: 90.0,
                width,
                height: 60.0,
            },
            material: crate::prototype::Material::Stone,
            intact: true,
        };
        let mut behind = Wall {
            footprint: Some(box_at(540.0)),
            broken_by: Vec::new(),
        };
        assert!(!blast_break(&c, 7, &[pillar(520.0, 10.0)], &mut behind));
        // A solid that only touches the segment's end (the box's near edge) lets
        // it through (the interior rule, CP-Q22).
        assert!(blast_break(&c, 7, &[pillar(540.0, 10.0)], &mut behind));
    }

    #[test]
    fn a_chained_blast_hands_its_owner_and_depth_to_the_hook() {
        let mut c = charge().with_profile(structure_profile());
        c.owner = Some(1);
        c.chain_depth = 3;
        let mut wall = Wall {
            footprint: Some(box_at(520.0)),
            broken_by: Vec::new(),
        };
        assert!(blast_break(&c, 9, &[], &mut wall));
        assert_eq!(wall.broken_by[0].owner, Some(1));
        assert_eq!(wall.broken_by[0].depth, 3);
    }
}
