//! COMPANION-AI-02, stage CA2-A: the pure zone, drift and claim code of the
//! partner's autonomy inside the camera. Nothing here is wired: no scene,
//! director or camera reads it yet (the wiring and the camera rule are CA2-B2). Spec: `docs/03-tech/companion-ai-02-donut-
//! autonomy.md`, sections 3 and 4.
//!
//! Role-neutral and IP-neutral (spec 5.3): it speaks of the *leader* (the hero
//! the human controls) and the *partner* (the companion-driven hero). A
//! partner's own numbers come from the profile's `autonomy` block and its role
//! table; the code never names a character.
//!
//! - [`FreeZone`] (spec 3.1, 3.2): the horizontal zone `Z` around the leader
//!   that keeps the partner inside the simulation scroll camera under any
//!   input the human gives, plus the vertical band and the rise limit.
//! - [`DriftCheck`] and [`Commitment`] (spec 3.3): before a voluntary
//!   commitment the partner checks the worst case, the leader running away for
//!   its whole duration, at the leader's run speed (no credit for her own).
//! - [`LeaderClaims`] (spec 4.2): the enemies the leader is working, built
//!   from [`crate::companion::engage::leader_engages`] and the front claim.
//!
//! The zone, drift and claim code is a pure function of the view, the profile
//! and the camera geometry: no state, no RNG, no map iteration, nothing hashed.
//!
//! CA2-B1 adds the goal layer (spec 5.1 to 5.4): [`Goal`], [`AutonomyState`] and
//! [`AutonomyState::step`], which turn the view, the zone, the claims, the drift
//! check and the level's standing surfaces ([`SpotWorld`]) into an
//! [`AutonomyBrief`], including the [`GoalBrief`] that gates the director. It is
//! still unwired (CA2-B2 calls it).

use crate::boss::BossThreat;
use crate::combat::{MAGIC_MISSILE, MoveId};
use crate::companion::director::{Decision, GoalBrief, Veto};
use crate::companion::engage::leader_engages;
use crate::companion::profile::{Autonomy, FixAll, GoalRow, ProfileSet, SkirmisherTable};
use crate::companion::view::{
    BOSS_ID, BossModeClass, BossView, CompanionView, Control, EntityKind, HeroView, StableEntityId,
    TargetState, TargetView, ViewBounds,
};
use crate::donut_moves::{
    DonutMoveTuning, POUNCE_CONTACT_PX, POUNCE_HIT_STOP, POUNCE_PIN, POUNCE_RECOVERY,
    POUNCE_STARTUP, SWAT_HIT_STOP, SWAT_TOTAL,
};
use crate::party::HeroId;
use crate::prototype::{HERO_HALF_WIDTH, HeroMotion, TICK_SECONDS};
use crate::scroll_camera::{CameraConfig, DEADZONE_PX, LEAD_PX};

/// `trail_cap` (CA2-Q05, derived, never data): how far behind a leader who runs
/// away the simulation scroll camera leaves its trailing view edge once it has
/// settled, `view_w / 2 + DEADZONE_PX - LEAD_PX` (336 px at the 640 px view).
/// The camera settles with its centre `LEAD_PX - DEADZONE_PX` behind him. A
/// test pins the formula against the real [`crate::scroll_camera::ScrollCamera`].
pub fn trail_cap_px(view_w: f32) -> f32 {
    view_w / 2.0 + DEADZONE_PX as f32 - LEAD_PX as f32
}

/// The view's top and bottom edges in world rows (y up, the unit of a hero's
/// `foot_y`). The scroll camera has a fixed top, so these never move.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewRows {
    pub top_y: f32,
    pub bottom_y: f32,
}

impl ViewRows {
    /// The rows of a scroll camera configuration (layout rows run downward from
    /// the layout top, world rows upward from the layout bottom).
    pub fn of_camera(cfg: &CameraConfig) -> Self {
        let top_y = (cfg.layout_h - cfg.view_top) as f32;
        Self {
            top_y,
            bottom_y: top_y - cfg.view_h as f32,
        }
    }
}

/// What [`FreeZone::compute`] reads: the leader and the simulation view, all of
/// it from this tick's pre-tick state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneInput {
    pub leader_x: f32,
    pub leader_foot_y: f32,
    pub view: ViewBounds,
    pub rows: ViewRows,
}

impl ZoneInput {
    pub fn new(leader: &HeroView, view: ViewBounds, rows: ViewRows) -> Self {
        Self {
            leader_x: leader.x,
            leader_foot_y: leader.foot_y,
            view,
            rows,
        }
    }
}

/// The free zone `Z = [left, right]` (spec 3.1) and the vertical limits of a
/// spot. Nothing outside it is ever a goal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreeZone {
    pub leader_x: f32,
    /// The zone's horizontal interval, `leader_x - reach_left` to
    /// `leader_x + reach_right`.
    pub left: f32,
    pub right: f32,
    /// Distance from the leader to the zone edge on each side, the leash
    /// applied (never negative).
    pub reach_left: f32,
    pub reach_right: f32,
    /// The camera-geometry bound on each side, `min(edge, trail_cap)` less the
    /// edge margin and her half width, **before** the leash (may be negative
    /// near an edge). The drift check compares against this, not the leash.
    pub safe_left: f32,
    pub safe_right: f32,
    /// The lowest and highest standing surface (world rows, y up) a spot may
    /// have: inside the view with the CA2-Q08 margins, at most the CA2-Q08 rise
    /// limit above the leader's feet.
    pub surface_min_y: f32,
    pub surface_max_y: f32,
}

impl FreeZone {
    /// Spec 3.1, both vertical limits.
    ///
    /// ```text
    /// safe_s  = min(edge_s, trail_cap) - CA2-Q03 margin - her half width
    /// reach_s = min(CA2-Q04 leash, safe_s)          (at least 0)
    /// ```
    pub fn compute(input: &ZoneInput, a: &Autonomy) -> Self {
        let view_w = input.view.right - input.view.left;
        let cap = trail_cap_px(view_w);
        let margin = f32::from(a.ca2_q03_edge_margin_px);
        let leash = f32::from(a.ca2_q04_leash_px);
        let side = |edge: f32| edge.min(cap) - margin - HERO_HALF_WIDTH;
        let safe_left = side(input.leader_x - input.view.left);
        let safe_right = side(input.view.right - input.leader_x);
        let reach_left = leash.min(safe_left).max(0.0);
        let reach_right = leash.min(safe_right).max(0.0);
        let surface_max_y = (input.rows.top_y - f32::from(a.ca2_q08_view_top_margin_px))
            .min(input.leader_foot_y + f32::from(a.ca2_q08_max_rise_px));
        let surface_min_y = input.rows.bottom_y + f32::from(a.ca2_q08_view_bottom_margin_px);
        Self {
            leader_x: input.leader_x,
            left: input.leader_x - reach_left,
            right: input.leader_x + reach_right,
            reach_left,
            reach_right,
            safe_left,
            safe_right,
            surface_min_y,
            surface_max_y,
        }
    }

    /// `x` is inside the horizontal zone.
    pub fn contains_x(&self, x: f32) -> bool {
        x >= self.left && x <= self.right
    }

    /// A standing surface at row `surface_y` is inside the vertical band and
    /// under the rise limit.
    pub fn surface_ok(&self, surface_y: f32) -> bool {
        surface_y >= self.surface_min_y && surface_y <= self.surface_max_y
    }

    /// A spot (a standing position) is inside `Z` and the vertical band.
    pub fn contains_spot(&self, x: f32, surface_y: f32) -> bool {
        self.contains_x(x) && self.surface_ok(surface_y)
    }

    /// The camera-geometry bound on the side of the leader that `x` is on
    /// (the smaller of the two for `x` exactly on him).
    pub fn safe_toward(&self, x: f32) -> f32 {
        if x > self.leader_x {
            self.safe_right
        } else if x < self.leader_x {
            self.safe_left
        } else {
            self.safe_left.min(self.safe_right)
        }
    }

    /// The soft pull (CA2-Q06): utility a goal or spot loses at `distance_px`
    /// from the leader, `pull_per_px` for every px past the comfort radius.
    pub fn pull_penalty(a: &Autonomy, distance_px: f32) -> i32 {
        let past = (distance_px - f32::from(a.ca2_q06_comfort_px)).max(0.0) as i32;
        past * i32::from(a.ca2_q06_pull_per_px)
    }
}

/// The attacker hit-stop the Magic Missile's move table gives: none. The bolt
/// hits at range, so `MISSILE_FEEL`'s 2 ticks of hit-stop freeze the target
/// only (`src/feel.rs`); the caster's own move clock is not extended. A test
/// pins this against the feel table.
pub const MISSILE_ATTACKER_HIT_STOP: u16 = 0;

/// A voluntary commitment: a stretch of ticks during which the partner cannot
/// run with the leader (spec 3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Commitment {
    /// Crouch, the longest flight, the pin, the recovery and the attacker's
    /// hit-stop.
    Pounce,
    /// The swat's whole timeline and the attacker's hit-stop.
    Swat,
    /// The cast's whole timeline (the move table gives no attacker hit-stop,
    /// [`MISSILE_ATTACKER_HIT_STOP`]).
    Missile,
    /// A nav transit of this many ticks (the edge's simulated duration).
    Transit { ticks: u16 },
}

impl Commitment {
    /// The worst-case duration in ticks, summed from the move-table constants
    /// (a table change moves the number, not the rule). The pounce counts the
    /// longest flight, `pounce_air_max_ticks`, not the nominal 14.
    pub fn worst_case_ticks(self, tuning: &DonutMoveTuning) -> u16 {
        match self {
            Self::Pounce => {
                POUNCE_STARTUP
                    + tuning.pounce_air_max_ticks
                    + POUNCE_PIN
                    + POUNCE_RECOVERY
                    + u16::from(POUNCE_HIT_STOP.attacker)
            }
            Self::Swat => SWAT_TOTAL + u16::from(SWAT_HIT_STOP.attacker),
            Self::Missile => {
                MAGIC_MISSILE.startup
                    + MAGIC_MISSILE.active
                    + MAGIC_MISSILE.recovery
                    + MISSILE_ATTACKER_HIT_STOP
            }
            Self::Transit { ticks } => ticks,
        }
    }
}

/// The drift check (spec 3.3, CA2-Q09): she starts a commitment only if she
/// would still be in view when it ends even if the leader ran away from her end
/// point for its whole length.
///
/// ```text
/// drift   = ticks x the LEADER's run speed per tick
/// allowed = d_end + drift + CA2-Q09 slack <= safe_s on the end point's side
/// ```
///
/// Her own speed gives no credit, so "every frame outside the view is
/// involuntary" stays exact under any motion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriftCheck {
    leader_px_per_tick: f32,
    slack_px: f32,
}

impl DriftCheck {
    /// `leader_motion` is the leader's body motion: his run speed sets the
    /// drift (`HeroMotion::CARL`, 185 px/s, for Carl).
    pub fn new(leader_motion: HeroMotion, a: &Autonomy) -> Self {
        Self {
            leader_px_per_tick: leader_motion.run_speed * TICK_SECONDS,
            slack_px: f32::from(a.ca2_q09_drift_slack_px),
        }
    }

    /// The leader's run speed per tick.
    pub fn leader_px_per_tick(&self) -> f32 {
        self.leader_px_per_tick
    }

    /// How far the leader runs in `ticks`.
    pub fn drift_px(&self, ticks: u16) -> f32 {
        f32::from(ticks) * self.leader_px_per_tick
    }

    /// The farthest an end point may be from the leader on a side whose camera
    /// bound is `safe_px`: `safe - drift - slack` (negative when no end point
    /// is allowed).
    pub fn max_end_distance(&self, safe_px: f32, ticks: u16) -> f32 {
        safe_px - self.drift_px(ticks) - self.slack_px
    }

    /// The commitment ending at `end_x` lasting `ticks` is allowed.
    pub fn allows(&self, zone: &FreeZone, end_x: f32, ticks: u16) -> bool {
        let d_end = (end_x - zone.leader_x).abs();
        d_end <= self.max_end_distance(zone.safe_toward(end_x), ticks)
    }

    /// The pounce (spec 3.3, review N-6): the end point is checked after the whole
    /// commitment, and the start point after the startup, because she holds still
    /// there while the leader runs away for the first ticks. For a pounce toward
    /// the leader the start point can be the farther of the two.
    pub fn allows_pounce(
        &self,
        zone: &FreeZone,
        start_x: f32,
        end_x: f32,
        tuning: &DonutMoveTuning,
    ) -> bool {
        self.allows(zone, start_x, POUNCE_STARTUP)
            && self.allows_commitment(zone, end_x, Commitment::Pounce, tuning)
    }

    /// [`Self::allows`] for a named commitment.
    pub fn allows_commitment(
        &self,
        zone: &FreeZone,
        end_x: f32,
        commitment: Commitment,
        tuning: &DonutMoveTuning,
    ) -> bool {
        self.allows(zone, end_x, commitment.worst_case_ticks(tuning))
    }
}

/// The leader's claim set `C` (spec 4.2): the enemies he is working, which the
/// partner leaves to him (CA2-Q11) unless he is low or outnumbered.
///
/// - every enemy [`leader_engages`] (A08, A11, A12), and
/// - the front claim: the nearest enemy on the side he faces, on his level
///   (feet within `cf02_duo_a09_vertical_reach_px`), within CA2-Q10 of him: the
///   one he is about to work. It is a standing enemy, or a lying or staggered one
///   that is down **from the leader's own hit** (`carl_hit`; adopted default
///   AD-B1 of the CA2-A review N-5): a blanket rule on every downed enemy would
///   forbid her own swat-then-pounce on a goblin she staggered in front of him.
///
/// Claims expire on their own: once the leader is more than the claim radius
/// away, and his last hit is older than the A12 window, an enemy is no longer
/// claimed (owner decision 1 of 2026-11-21).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LeaderClaims {
    /// Sorted, no duplicates.
    ids: Vec<StableEntityId>,
    front: Option<StableEntityId>,
}

impl LeaderClaims {
    /// Build the set for this tick. A defeated leader claims nothing.
    pub fn compute(view: &CompanionView, set: &ProfileSet, leader: &HeroView) -> Self {
        if leader.defeated {
            return Self::default();
        }
        let radius = f32::from(set.autonomy.ca2_q10_claim_radius_px);
        let vertical = f32::from(set.cf02_duo.cf02_duo_a09_vertical_reach_px);
        let mut ids = Vec::new();
        let mut front: Option<(f32, StableEntityId)> = None;
        for target in &view.targets {
            if target.hp == 0 {
                continue;
            }
            if leader_engages(set, leader, target) {
                ids.push(target.id);
            }
            if let Some(distance) = front_claim_distance(leader, target, radius, vertical) {
                // Targets are ordered by id, so a strict `<` keeps the lowest
                // id among equals.
                if front.is_none_or(|(best, _)| distance < best) {
                    front = Some((distance, target.id));
                }
            }
        }
        let front = front.map(|(_, id)| id);
        if let Some(id) = front {
            ids.push(id);
        }
        // FA-Q34: the enemies reserved for the leader's finish are his (never Hunt's).
        ids.extend(view.reserved.iter().copied());
        ids.sort_unstable();
        ids.dedup();
        Self { ids, front }
    }

    /// `id` is claimed by the leader.
    pub fn contains(&self, id: StableEntityId) -> bool {
        self.ids.binary_search(&id).is_ok()
    }

    /// The claimed ids, in id order.
    pub fn ids(&self) -> &[StableEntityId] {
        &self.ids
    }

    /// The front claim, when there is one (it is also in [`Self::ids`]).
    pub fn front(&self) -> Option<StableEntityId> {
        self.front
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }
}

/// The distance from the leader to `target` when it qualifies for the front
/// claim: standing (or down from his hit), on his facing side (an enemy exactly on him is in front),
/// on his level and within `radius`.
fn front_claim_distance(
    leader: &HeroView,
    target: &TargetView,
    radius: f32,
    vertical: f32,
) -> Option<f32> {
    let dx = target.x - leader.x;
    let in_front = dx == 0.0 || (if dx < 0.0 { -1 } else { 1 }) == leader.facing;
    let distance = dx.abs();
    let counts = match target.state {
        TargetState::Standing => true,
        TargetState::Staggered { .. } | TargetState::Lying { .. } => target.carl_hit,
        TargetState::Airborne => false,
    };
    (counts && in_front && distance <= radius && (target.foot_y - leader.foot_y).abs() <= vertical)
        .then_some(distance)
}

// ===========================================================================
// COMPANION-AI-02, stage CA2-B1: the goal selector (spec 5.1 to 5.4).
//
// Pure and unwired: nothing in a scene calls it until CA2-B2. It turns the
// per-tick view, the free zone, the claim set and the drift check into one
// [`AutonomyBrief`]: the partner's goal, its spot and the [`GoalBrief`] that
// gates the director. No Bevy, no unordered maps, no RNG, integer utilities, ties
// by goal order, then stable id, then anchor key, then x.
// ===========================================================================

/// The spacing between sampled spot candidates along a surface (spec 5.1).
pub const SPOT_SAMPLE_PX: f32 = 16.0;

/// Show-off starts only within this many ticks of her landed hit: the window of
/// CF02-I's `smug` acting overlay (needs idle within 45 ticks of the hit and
/// shows 20 ticks). Adopted default AD-B3: a code constant, because spec 5.1 gives
/// the number and the data block has no knob for it.
pub const SHOW_OFF_HIT_WINDOW_TICKS: u64 = 45;

/// The partner's goal (spec 5.1). The interrupt is the director's, outside this
/// list; equal utilities are broken by [`Goal::tie_rank`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Goal {
    Retreat,
    SupportLeader,
    Hunt,
    Escort,
    ShowOff,
    /// Selected from CA2-C; B1 never picks it.
    Perch,
    Idle,
    /// FA-Q04: punish a vulnerable boss (Stun, Pinned, a recovery) with a cast,
    /// or a pounce when she is dry. Appended last so the other goals keep their
    /// index in the metrics arrays and the state hash.
    BossPunish,
}

impl Goal {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Retreat => "retreat",
            Self::SupportLeader => "support_leader",
            Self::Hunt => "hunt",
            Self::Escort => "escort",
            Self::ShowOff => "show_off",
            Self::Perch => "perch",
            Self::Idle => "idle",
            Self::BossPunish => "boss_punish",
        }
    }

    /// The order of equal utilities (adopted default AD-B2): Retreat, Support,
    /// then Escort before Hunt, so that while the leader travels (Escort 800 plus
    /// 400 ties a new Hunt at 1200) she comes along instead of starting a hunt.
    const fn tie_rank(self) -> u8 {
        match self {
            Self::Retreat => 0,
            Self::SupportLeader => 1,
            Self::Escort => 2,
            Self::Hunt => 3,
            Self::ShowOff => 4,
            Self::Perch => 5,
            Self::Idle => 6,
            Self::BossPunish => 7,
        }
    }

    fn row(self, table: &SkirmisherTable) -> &GoalRow {
        let goals = &table.goals;
        match self {
            Self::Retreat => &goals.retreat,
            Self::SupportLeader => &goals.support_leader,
            Self::Hunt => &goals.hunt,
            Self::Escort => &goals.escort,
            Self::ShowOff => &goals.show_off,
            Self::Perch => &goals.perch,
            Self::Idle => &goals.idle,
            Self::BossPunish => &goals.boss_punish,
        }
    }
}

/// Why the goal changed (a trace, never hashed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalReason {
    /// The first decision of a life (or after a restart or a swap back).
    Start,
    /// She is outside `Z`: Escort wins at once.
    OutsideZone,
    /// A Retreat trigger: a hurt with a standing enemy close.
    Hurt,
    /// A Retreat or a Show-off ended.
    Ended,
    /// The target or the leader's pressure is gone, or the move is no longer
    /// commit-safe.
    TargetLost,
    /// An ordinary utility decision.
    Utility,
    /// FA-Q04: a vulnerable boss window opened (or its goal ended): immediate,
    /// so the minimum goal time does not hold her back.
    BossWindow,
    /// FA-Q05: she stands closer to the boss than the standoff band, between
    /// the boss and the leader, or in the charge lane: immediate.
    BossStandoff,
    /// FA-Q25: she shadows the leader (the same way within the radius for more
    /// than the threshold): immediate.
    Shadow,
}

/// One standing surface a spot may be taken from: a flat interval already
/// inset from drops and walls (spec 3.5), open and standable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Surface {
    /// A stable ordering key (the nav anchor id): ties go to the lower key.
    pub key: u64,
    pub left: f32,
    pub right: f32,
    /// The standing height (world rows, y up).
    pub y: f32,
    pub risk: u16,
}

/// What the goal selector reads of the level: the standing surfaces and the
/// cost of a route. `NavSpots` (in `navfollow.rs`) is the live implementation.
pub trait SpotWorld {
    /// Append the open, safe-to-stand surfaces, in ascending `key` order.
    fn surfaces(&self, out: &mut Vec<Surface>);

    /// The route from where `partner` stands to `x` on `surface`: `None` when
    /// there is none, `Some(0)` when it is a walk she can run with the leader,
    /// otherwise the summed simulated duration of the jump and drop edges (the
    /// ticks of the nav transits, spec 3.3).
    fn route_ticks(&self, partner: &HeroView, x: f32, surface: &Surface) -> Option<u16>;

    /// CA2-C (K13, CA2-Q29): a hero-hurting hazard rectangle (telegraphed or
    /// active, whatever its class) overlaps her body standing at `x` on a surface
    /// at height `y`. The default is `false`: a world with no hazard feed.
    fn hazard_at(&self, _x: f32, _y: f32) -> bool {
        false
    }

    /// CA2-C: a hero-hurting hazard overlaps her body anywhere on the ground run
    /// from `x0` to `x1` at height `y` (a pounce's hop is such a run).
    fn hazard_between(&self, _x0: f32, _x1: f32, _y: f32) -> bool {
        false
    }

    /// RINGW-F1: she is **cut off** from the leader's ground: both stand on open
    /// surfaces, different ones, his is not above hers, and the route from hers
    /// to his needs a jump or a drop over ground higher than his (it climbs over
    /// a barrier, like the court's step and walkway over the closed shutter, or
    /// she is above him on the way down). A plain jump over a gap in one floor
    /// is no crossing. The default is `None`: a world with no such crossing.
    fn crossing(&self, _partner: &HeroView, _leader: &HeroView) -> Option<Crossing> {
        None
    }
}

/// RINGW-F1: the leader's standing surface across a crossing
/// ([`SpotWorld::crossing`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crossing {
    /// The leader's surface, inset like every [`SpotWorld::surfaces`] entry.
    pub surface: Surface,
    /// Both surfaces stand at the same height: a barrier between two stretches
    /// of one floor (RWF1-AD1: she neither holds nor escorts on her side of it).
    pub same_level: bool,
}

/// What a spot is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpotPurpose {
    /// The attack spot of a Hunt or Support target.
    Attack,
    /// The hit-and-run spring-back from the target.
    Recoil,
    Retreat,
    Escort,
}

/// A chosen standing position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spot {
    pub x: f32,
    pub surface_y: f32,
    pub key: u64,
    /// The leader's x when the spot was picked (CA2-Q30 re-pick reference).
    pub leader_ref_x: f32,
    pub purpose: SpotPurpose,
}

/// The goal layer's output for one tick.
#[derive(Debug, Clone, PartialEq)]
pub struct AutonomyBrief {
    pub goal: Goal,
    /// `Some` on the tick the goal changed.
    pub changed: Option<GoalReason>,
    pub target: Option<StableEntityId>,
    /// Where goal steering takes her; `None` means hold where she stands.
    pub spot: Option<Spot>,
    /// The x she faces once there (the target, or the leader for Show-off).
    pub face_x: Option<f32>,
    /// The hit-and-run recoil is running (Hunt only): no target is allowed.
    pub recoil: bool,
    pub in_zone: bool,
    /// What the director may do, passed to `CompanionDirector::step_briefed`.
    pub director: GoalBrief,
}

/// Counters for the M16 report (never hashed): how often the claim set and the
/// drift check held her back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AutonomyStats {
    /// Full decisions taken.
    pub decisions: u32,
    /// Hunt candidate evaluations (one per live target per decision).
    pub hunt_evaluations: u32,
    /// Evaluations dropped because the target is in the leader's claim set.
    pub hunt_claim_excluded: u32,
    /// Evaluations with no spot, where a spot failed only the drift check.
    pub hunt_drift_rejected: u32,
    /// Ticks the brief allowed interrupts only because the commitment from her
    /// current x fails the drift check (review F-2).
    pub brief_drift_blocked_ticks: u32,
    /// CA2-C: Hunt candidate evaluations where a hazard rectangle on the
    /// pounce's run (or under the spot) was the only thing that stopped every
    /// spot, and ticks on which the brief allowed interrupts only because the
    /// pounce from where she stands would cross one (M9 reports them).
    pub hunt_hazard_rejected: u32,
    pub brief_hazard_blocked_ticks: u32,
    /// Ticks on which the director skipped an interrupt press because the worst
    /// case of the commitment would leave the view (spec 3.3, M16; see
    /// [`AutonomyState::record_decision`]). It counts **press ticks**, not tells:
    /// while she stays in reach and outside `commit_x`, one tell is counted on
    /// every tick it stays legal. The metrics harness reports it as ticks.
    pub interrupts_drift_skipped: u32,
    /// Ticks the leader counted as travelling (the 12-tick walk or a pending
    /// camera pan), and of those the ticks that only the pending pan made true
    /// (review N-e: a leader who turns near the dead-zone edge shifts the lead
    /// point; reported with M13 and M15, never tuned here).
    pub travelling_ticks: u32,
    pub pan_only_travelling_ticks: u32,
    /// DP-E: blink presses the goal layer made (DP-Q39), by trigger. Whether a
    /// press became an arrival is in the combat events (resolved, fizzled,
    /// cancelled), which the fixtures count.
    pub blink_presses_retreat: u32,
    pub blink_presses_committed: u32,
    /// FA-Q01 (CA2-FU-BOSS): how many times the goal changed into Retreat. A
    /// plain counter (never hashed) so a run can prove she did, or did not,
    /// retreat without reading the capped event log.
    pub retreat_entries: u32,
}

/// Everything one tick's step reads.
pub struct AutonomyInput<'a> {
    pub view: &'a CompanionView,
    pub set: &'a ProfileSet,
    /// The companion-driven hero this layer steers (the leader is the other).
    pub partner: HeroId,
    pub zone: &'a FreeZone,
    pub drift: &'a DriftCheck,
    /// The partner's own body motion. The zone proof (spec 3.1) needs a partner
    /// at least as fast as the leader; [`AutonomyState::step`] asserts it in debug
    /// builds (a slower partner, for example a future AI-Carl brawler next to a
    /// human Donut, needs its own leash rule first).
    pub partner_motion: HeroMotion,
    pub tuning: &'a DonutMoveTuning,
    pub world: &'a dyn SpotWorld,
    /// `SceneSim::camera_pan_remaining()`: the pan the scroll camera still owes.
    pub pan_remaining: i32,
    /// One of her damaging moves connected on this tick's pre-tick state (the
    /// Show-off trigger; the scene reads it from the combat events).
    pub partner_hit_landed: bool,
}

/// What a tick observed, kept for the next one.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Seen {
    tick: u64,
    health: u8,
    hurt_ticks: u8,
    move_id: Option<MoveId>,
    leader_x: f32,
}

/// The attack mode of one target: how she would hit it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HuntMode {
    /// A pounce at close range (before the unlock, or a lying goblin).
    Pounce,
    /// A Magic Missile at spacing (after the unlock, with mana).
    Cast,
}

/// The partner's goal state (spec 5.4). Cooldowns are absolute "ready at"
/// ticks, so they never tick while unhashed and a swap round trip stays
/// deterministic.
#[derive(Debug, Clone, PartialEq)]
pub struct AutonomyState {
    started: bool,
    goal: Goal,
    goal_since: u64,
    target: Option<StableEntityId>,
    spot: Option<Spot>,
    last_decision: Option<u64>,
    recoil_until: u64,
    retreat_ready_at: u64,
    show_off_ready_at: u64,
    perch_ready_at: u64,
    last_hurt: Option<u64>,
    last_landed: Option<u64>,
    travel_dir: i8,
    travel_ticks: u16,
    prev: Option<Seen>,
    /// Each target's x at the previous tick, in view (id) order.
    seen: Vec<(StableEntityId, f32)>,
    /// DP-E (DP-Q41): the tick of the last blink press this layer made. Part of
    /// the hashed decision state, but mixed only once a press exists, so a run
    /// that never blinks hashes exactly as before.
    last_blink: Option<u64>,
    /// FA-Q09: a blink was pressed for the boss' charge that is still up: the
    /// follow layer's dodge jump stays silent for that charge. Keyed by the
    /// charge's start (`BossThreat::Charge::started`), so a chained charge is
    /// not silenced; cleared when no charge or another charge is up. Its marker
    /// is mixed into the hash only while set (the key is the boss' clock, which
    /// the boss hashes).
    charge_latch: Option<u64>,
    /// FA-Q25: consecutive ticks both heroes travelled the same way within the
    /// shadow radius, and her x on the previous tick. Mixed into the hash only
    /// while the count is above zero (the knob off never counts).
    shadow_ticks: u16,
    prev_me_x: Option<f32>,
    stats: AutonomyStats,
}

impl Default for AutonomyState {
    fn default() -> Self {
        Self {
            started: false,
            goal: Goal::Escort,
            goal_since: 0,
            target: None,
            spot: None,
            last_decision: None,
            recoil_until: 0,
            retreat_ready_at: 0,
            show_off_ready_at: 0,
            perch_ready_at: 0,
            last_hurt: None,
            last_landed: None,
            travel_dir: 0,
            travel_ticks: 0,
            prev: None,
            seen: Vec::new(),
            last_blink: None,
            charge_latch: None,
            shadow_ticks: 0,
            prev_me_x: None,
            stats: AutonomyStats::default(),
        }
    }
}

/// A scored candidate goal.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Cand {
    goal: Goal,
    utility: i32,
    target: Option<StableEntityId>,
    spot: Option<Spot>,
}

/// A spot search result: the best spot, and whether a drift rejection was the
/// only thing that stopped a candidate (the M16 signal).
struct SpotSearch {
    spot: Option<Spot>,
    drift_rejected: bool,
    /// CA2-C: a hazard on the pounce's run stopped at least one candidate spot.
    hazard_rejected: bool,
}

/// The facts of one step, derived once.
struct Ctx<'a> {
    inp: &'a AutonomyInput<'a>,
    a: &'a Autonomy,
    t: &'a SkirmisherTable,
    me: &'a HeroView,
    leader: &'a HeroView,
    claims: LeaderClaims,
    cast_ok: bool,
    boss: bool,
    /// FA-Q02: the boss as the goal layer sees it, `Some` only with
    /// `fa_q02_boss_play` on and the boss engaged (it has a body).
    bv: Option<BossView>,
    /// FA-Q25: she has shadowed the leader for more than the threshold: the escort
    /// spot is re-picked at least `fa_q25_respot_px` from him.
    shadowing: bool,
    travelling: bool,
    travel_dir: i8,
    /// She is in a move or in the air: committed, so no routine re-decision.
    committed: bool,
}

fn sign(dx: f32) -> i8 {
    if dx < 0.0 { -1 } else { 1 }
}

fn ipx(distance: f32) -> i32 {
    distance as i32
}

/// Sample the x positions along a surface: every [`SPOT_SAMPLE_PX`] from the
/// left end, and the right end when it is not already covered.
fn sample_xs(surface: &Surface, mut f: impl FnMut(f32)) {
    if surface.right < surface.left {
        return;
    }
    let mut x = surface.left;
    let mut last = x;
    while x <= surface.right {
        f(x);
        last = x;
        x += SPOT_SAMPLE_PX;
    }
    if surface.right > last {
        f(surface.right);
    }
}

/// Keeps the best of `(score, key, x)`: the highest score, then the lower key,
/// then the lower x (candidates arrive in ascending key and x order, so a
/// strict improvement is all that is needed).
struct BestSpot {
    best: Option<(i32, Spot)>,
}

impl BestSpot {
    fn offer(&mut self, score: i32, spot: Spot) {
        if self.best.is_none_or(|(s, _)| score > s) {
            self.best = Some((score, spot));
        }
    }
}

impl<'a> Ctx<'a> {
    fn zone(&self) -> &FreeZone {
        self.inp.zone
    }

    fn vertical(&self) -> f32 {
        f32::from(self.inp.set.cf02_duo.cf02_duo_a09_vertical_reach_px)
    }

    /// How she would hit `t`, if she can at all (spec 4.2: before the unlock a
    /// llama is no target, CA2-Q31; a lying goblin is a pounce target).
    fn mode_for(&self, t: &TargetView) -> Option<HuntMode> {
        if t.hp == 0 || !t.in_view || t.guarded || !self.inp.view.view.in_view(t.x) {
            return None;
        }
        let standing_or_staggered = matches!(
            t.state,
            TargetState::Standing | TargetState::Staggered { .. }
        );
        match t.id.kind {
            // FA-Q04: the boss is hunted only as BossPunish, while it is
            // vulnerable (a cast with mana, a pounce when she is dry).
            EntityKind::Boss => self
                .bv
                .is_some_and(|b| b.mode_class == BossModeClass::Vulnerable)
                .then_some(if self.cast_ok {
                    HuntMode::Cast
                } else {
                    HuntMode::Pounce
                }),
            EntityKind::Llama => (self.cast_ok && standing_or_staggered).then_some(HuntMode::Cast),
            EntityKind::Goblin => {
                if self.cast_ok && standing_or_staggered {
                    Some(HuntMode::Cast)
                } else if matches!(t.state, TargetState::Airborne) {
                    None
                } else {
                    Some(HuntMode::Pounce)
                }
            }
        }
    }

    /// The press band (centre distance) of a mode: the pounce band, or the
    /// casting band from the back-off radius to the missile maximum.
    fn band(&self, mode: HuntMode) -> (f32, f32) {
        let duo = &self.inp.set.cf02_duo;
        match mode {
            HuntMode::Pounce => (
                f32::from(duo.cf02_duo_a06_pounce_min_px),
                f32::from(duo.cf02_duo_a06_pounce_max_px),
            ),
            HuntMode::Cast => (
                f32::from(self.t.ca2_q20_backoff_px),
                f32::from(duo.cf02_duo_a05_missile_max_px),
            ),
        }
    }

    fn spacing(&self, mode: HuntMode, recoil: bool) -> f32 {
        if recoil {
            f32::from(self.t.ca2_q19_recoil_px)
        } else {
            match mode {
                HuntMode::Pounce => f32::from(self.t.ca2_q19_pounce_prefer_px),
                HuntMode::Cast => f32::from(self.t.ca2_q20_cast_spacing_px),
            }
        }
    }

    fn standing_within(&self, from_x: f32, radius: f32) -> bool {
        // FA-Q03: the boss counts as a standing enemy except in Stun, Pinned and
        // Defeated (and before it has a body).
        self.bv
            .is_some_and(|b| !b.inert && (b.x - from_x).abs() <= radius)
            || self.inp.view.targets.iter().any(|t| {
                t.hp > 0 && t.state == TargetState::Standing && (t.x - from_x).abs() <= radius
            })
    }

    /// A target of the goal layer by id: a goblin or llama of the view, or the
    /// boss (FA-Q04), which is never in `targets`.
    fn target_of(&self, id: StableEntityId) -> Option<TargetView> {
        if id == BOSS_ID {
            return self
                .bv
                .map(|b| b.as_target(self.inp.view.view.in_view(b.x)));
        }
        self.inp.view.target(id).copied()
    }

    fn leader_low(&self) -> bool {
        self.leader.health <= self.a.ca2_q12_leader_low_hp
    }

    /// Live enemies within the outnumbered radius of the leader.
    fn pressure_near_leader(&self) -> u8 {
        let radius = f32::from(self.a.ca2_q13_outnumbered_radius_px);
        self.inp
            .view
            .targets
            .iter()
            .filter(|t| t.hp > 0 && (t.x - self.leader.x).abs() <= radius)
            .count()
            .min(255) as u8
    }

    fn leader_outnumbered(&self) -> bool {
        self.pressure_near_leader() >= self.a.ca2_q13_outnumbered_count
    }

    /// Is there a live, unclaimed enemy within the radius of the leader that
    /// she could hit?
    fn unclaimed_near_leader(&self) -> bool {
        let radius = f32::from(self.a.ca2_q13_outnumbered_radius_px);
        self.inp.view.targets.iter().any(|t| {
            t.hp > 0
                && !self.claims.contains(t.id)
                && (t.x - self.leader.x).abs() <= radius
                && self.mode_for(t).is_some()
        })
    }

    /// May Support-leader take `t` right now (spec 4.3)?
    fn support_allows(&self, t: &TargetView) -> bool {
        if self.mode_for(t).is_none()
            || (t.x - self.leader.x).abs() > f32::from(self.a.ca2_q13_outnumbered_radius_px)
        {
            return false;
        }
        let low = self.leader_low();
        let outnumbered = self.leader_outnumbered();
        if !low && !outnumbered {
            return false;
        }
        if !self.claims.contains(t.id) {
            return true;
        }
        // A claimed enemy only when the leader is low and no unclaimed one is
        // close (never as a finish: the director's A08 veto still applies).
        low && !self.unclaimed_near_leader()
    }

    /// RINGW-F1: the crossing that cuts her off from the leader's ground, while
    /// the `rwf1_cut_off_escort` knob is on ([`SpotWorld::crossing`]).
    fn cut_off(&self) -> Option<Crossing> {
        if self.inp.set.fix_all.rwf1_cut_off_escort {
            self.inp.world.crossing(self.me, self.leader)
        } else {
            None
        }
    }

    /// The worst-case commitment of attacking `t` from `x` is inside the view,
    /// and so is the walk or the nav transit that gets her there (spec 3.3).
    /// Returns `(ok, only_the_drift_check_failed)`.
    fn commit_ok(
        &self,
        x: f32,
        surface: &Surface,
        t: Option<(&TargetView, HuntMode)>,
    ) -> (bool, bool) {
        let drift = self.inp.drift;
        let zone = self.zone();
        let Some(ticks) = self.inp.world.route_ticks(self.me, x, surface) else {
            return (false, false);
        };
        let mut allowed = ticks == 0 || drift.allows(zone, x, ticks);
        if let Some((target, mode)) = t {
            allowed &= self.drift_ok(x, target, mode);
        }
        (allowed, !allowed)
    }

    /// The worst-case commitment of attacking `target` from `x` (a pounce with its
    /// start and end point, or a cast) passes the drift check (spec 3.3, N-6).
    fn drift_ok(&self, x: f32, target: &TargetView, mode: HuntMode) -> bool {
        let drift = self.inp.drift;
        let zone = self.zone();
        match mode {
            HuntMode::Pounce => {
                let dir = f32::from(sign(target.x - x));
                let end = target.x - dir * POUNCE_CONTACT_PX;
                drift.allows_pounce(zone, x, end, self.inp.tuning)
            }
            HuntMode::Cast => {
                drift.allows_commitment(zone, x, Commitment::Missile, self.inp.tuning)
            }
        }
    }

    /// CA2-C (K13): the pounce from `x` on a surface at `y` would run over a
    /// hero-hurting hazard (its hop covers `x` to the landing beside the target).
    /// A cast stands in place; it is never blocked here.
    fn hazard_blocks(&self, x: f32, y: f32, target: &TargetView, mode: HuntMode) -> bool {
        match mode {
            HuntMode::Pounce => {
                let dir = f32::from(sign(target.x - x));
                let end = target.x - dir * POUNCE_CONTACT_PX;
                self.inp.world.hazard_between(x, end, y)
            }
            HuntMode::Cast => false,
        }
    }

    /// The partner positions (x range) from which an interrupt's worst case (a
    /// swat or a missile, in place) stays in view (spec 3.3: interrupts get no
    /// exemption, review F-3). Empty (`lo > hi`) when no position qualifies.
    fn interrupt_commit_x(&self) -> (f32, f32) {
        let ticks = Commitment::Swat
            .worst_case_ticks(self.inp.tuning)
            .max(Commitment::Missile.worst_case_ticks(self.inp.tuning));
        let zone = self.zone();
        let drift = self.inp.drift;
        let left = drift.max_end_distance(zone.safe_left, ticks);
        let right = drift.max_end_distance(zone.safe_right, ticks);
        let lo = if left >= 0.0 {
            self.leader.x - left
        } else {
            self.leader.x + 0.01
        };
        let hi = if right >= 0.0 {
            self.leader.x + right
        } else {
            self.leader.x - 0.01
        };
        (lo, hi)
    }

    /// The best spot from which to attack `t` (or spring back from it), spec 5.1.
    fn attack_spot(
        &self,
        surfaces: &[Surface],
        t: &TargetView,
        mode: HuntMode,
        recoil: bool,
    ) -> SpotSearch {
        let zone = self.zone();
        let (band_min, band_max) = self.band(mode);
        let wanted = self.spacing(mode, recoil);
        let mut best = BestSpot { best: None };
        let mut drift_failed = false;
        let mut hazard_failed = false;
        for surface in surfaces {
            if surface.risk >= self.a.ca2_q29_spot_max_risk
                || (surface.y - t.foot_y).abs() > self.vertical()
            {
                continue;
            }
            sample_xs(surface, |x| {
                let distance = (x - t.x).abs();
                if !zone.contains_spot(x, surface.y)
                    || (!recoil && !(band_min..=band_max).contains(&distance))
                {
                    return;
                }
                let (ok, drift_only) = self.commit_ok(x, surface, (!recoil).then_some((t, mode)));
                if !ok {
                    drift_failed |= drift_only;
                    return;
                }
                // CA2-C: the pounce's hop never runs over a hazard (a spring-back
                // is a walk, which the graph and the fence already keep clear).
                if !recoil && self.hazard_blocks(x, surface.y, t, mode) {
                    hazard_failed = true;
                    return;
                }
                // Stay on her own side of the target: no crossing it.
                let side_penalty = if (x - t.x) * (self.me.x - t.x) < 0.0 {
                    1000
                } else {
                    0
                };
                let score = -ipx((distance - wanted).abs())
                    - side_penalty
                    - FreeZone::pull_penalty(self.a, (x - self.leader.x).abs());
                best.offer(
                    score,
                    Spot {
                        x,
                        surface_y: surface.y,
                        key: surface.key,
                        leader_ref_x: self.leader.x,
                        purpose: if recoil {
                            SpotPurpose::Recoil
                        } else {
                            SpotPurpose::Attack
                        },
                    },
                );
            });
        }
        SpotSearch {
            spot: best.best.map(|(_, s)| s),
            drift_rejected: drift_failed,
            hazard_rejected: hazard_failed,
        }
    }

    /// Retreat spot terms (CA2-Q22): far from the nearest enemy, the leader
    /// between her and it, higher is better. Under boss play (FA-Q03) the boss
    /// is an enemy too, and a spot must be on her own side of it (she cannot
    /// cross its body), at least `fa_q03_retreat_boss_clear_px` from it, outside
    /// the charge lane and not across a Committed hitbox; with none, the spot
    /// farthest from the boss that is still on her side and path-safe.
    fn retreat_spot(&self, surfaces: &[Surface]) -> Option<Spot> {
        let zone = self.zone();
        let mut best = BestSpot { best: None };
        // FA-Q03: the farthest legal anchor from the boss, when the strict rules
        // leave none.
        let mut fallback = BestSpot { best: None };
        // FA-Q27: the best spot of the old rule, when no spot is clear of every
        // standing enemy (the plan's "highest anchor" measured worse: a perch beside
        // the enemy, 39 px away in `cf02_free_retreat_when_hurt`).
        let mut unclear = BestSpot { best: None };
        let boss = self.bv.filter(|_| self.fa().fa_q02_boss_play);
        for surface in surfaces {
            if surface.risk >= self.a.ca2_q29_spot_max_risk {
                continue;
            }
            sample_xs(surface, |x| {
                if !zone.contains_spot(x, surface.y) || !self.commit_ok(x, surface, None).0 {
                    return;
                }
                // The nearest *standing* enemy: a downed goblin is no threat while
                // it lies (review N-f).
                let nearest = self
                    .inp
                    .view
                    .targets
                    .iter()
                    .filter(|e| e.hp > 0 && e.state == TargetState::Standing)
                    .min_by(|p, q| {
                        (p.x - x)
                            .abs()
                            .total_cmp(&(q.x - x).abs())
                            .then(p.id.cmp(&q.id))
                    });
                // FA-Q03: a standing boss is an enemy; a goblin wins a tie.
                let enemy_x = match (nearest.map(|e| e.x), boss.filter(|b| !b.inert)) {
                    (Some(g), Some(b)) if (b.x - x).abs() < (g - x).abs() => Some(b.x),
                    (Some(g), _) => Some(g),
                    (None, Some(b)) => Some(b.x),
                    (None, None) => None,
                };
                let (away, between) = match enemy_x {
                    Some(ex) => {
                        let lo = x.min(ex);
                        let hi = x.max(ex);
                        (
                            ipx((ex - x).abs()),
                            if self.leader.x > lo && self.leader.x < hi {
                                i32::from(self.t.ca2_q22_between_bonus)
                            } else {
                                0
                            },
                        )
                    }
                    None => (0, 0),
                };
                let rise = (surface.y - self.leader.foot_y).max(0.0);
                let height = (rise / 16.0) as i32 * i32::from(self.t.ca2_q22_height_bonus_per_16px);
                let score = away + between + height
                    - FreeZone::pull_penalty(self.a, (x - self.leader.x).abs());
                let spot = Spot {
                    x,
                    surface_y: surface.y,
                    key: surface.key,
                    leader_ref_x: self.leader.x,
                    purpose: SpotPurpose::Retreat,
                };
                if let Some(b) = boss {
                    // Never across the boss body, never across a live hitbox.
                    if (x - b.x) * (self.me.x - b.x) < 0.0 || self.path_crosses_committed(&b, x) {
                        return;
                    }
                    fallback.offer(ipx((x - b.x).abs()), spot);
                    if (x - b.x).abs() < f32::from(self.fa().fa_q03_retreat_boss_clear_px)
                        || self.in_lane(&b, x)
                    {
                        return;
                    }
                }
                // FA-Q27: at least `fa_q27_retreat_clear_px` from the nearest standing
                // enemy; one that is closer only competes when none is.
                if self.fa().fa_q27_retreat_clear
                    && enemy_x.is_some_and(|ex| {
                        (ex - x).abs() < f32::from(self.fa().fa_q27_retreat_clear_px)
                    })
                {
                    unclear.offer(score, spot);
                    return;
                }
                best.offer(score, spot);
            });
        }
        // F01-FU-B10 (#250 review): FA-Q03's fallback is for "no legal spot under
        // the boss rules", and FA-Q27's `unclear` spots pass those rules (only the
        // 112 px from a goblin fails), so they come before the fallback: best, then
        // unclear, then fallback. Without a standing goblin beside a boss (the
        // Juicer alone) `unclear` is empty and the order changes nothing.
        best.best.or(unclear.best).or(fallback.best).map(|(_, s)| s)
    }

    /// The escort spot: 72 to 120 px behind the leader while he travels, on
    /// her own side of him otherwise, on his level and flat ground.
    fn escort_spot(&self, surfaces: &[Surface]) -> Option<Spot> {
        // While a charge is up the escort is the ordinary one (the dodge is the
        // answer to a ram, and the standoff would walk her to the wrong side of
        // the crash).
        if let Some(b) = self.bv.filter(|b| !b.charging) {
            return self.boss_escort_spot(surfaces, &b);
        }
        let zone = self.zone();
        let min = f32::from(self.a.ca2_q07_escort_min_px);
        let max = f32::from(self.a.ca2_q07_escort_max_px);
        let mid = (min + max) / 2.0;
        let side = if self.travelling && self.travel_dir != 0 {
            -self.travel_dir
        } else if self.travelling || self.me.x == self.leader.x {
            -self.leader.facing
        } else {
            sign(self.me.x - self.leader.x)
        };
        let mut best = BestSpot { best: None };
        // `drift`: whether the spot must pass the drift check (spec 3.3).
        let offer = |best: &mut BestSpot, surface: &Surface, drift: bool| {
            sample_xs(surface, |x| {
                if !zone.contains_spot(x, surface.y) {
                    return;
                }
                let (ok, drift_only) = self.commit_ok(x, surface, None);
                if !(ok || (!drift && drift_only)) {
                    return;
                }
                let from_leader = (x - self.leader.x).abs();
                // FA-Q25: out of the shadow radius.
                if self.shadowing && from_leader < f32::from(self.fa().fa_q25_respot_px) {
                    return;
                }
                let wrong_side = (x - self.leader.x) * f32::from(side) < 0.0;
                let score = -ipx((from_leader - mid).abs())
                    - if wrong_side { 1000 } else { 0 }
                    - FreeZone::pull_penalty(self.a, from_leader);
                best.offer(
                    score,
                    Spot {
                        x,
                        surface_y: surface.y,
                        key: surface.key,
                        leader_ref_x: self.leader.x,
                        purpose: SpotPurpose::Escort,
                    },
                );
            });
        };
        // RINGW-F1 (RWF1-AD2): she is cut off from the leader's ground (the
        // court's step and walkway over the closed shutter). The drift check
        // prices the whole crossing as one commitment, so every spot across
        // fails it and she would park at the barrier's face, or stall on the
        // walkway. She escorts on the leader's own surface instead, with the
        // drift check waived for the crossing: the executor commits one edge at
        // a time and the off-screen regroup still covers a leader who runs away
        // meanwhile. Across a barrier on one floor she always does (RWF1-AD1);
        // from above, only when no spot in the escort band passes the check.
        let cross = self
            .cut_off()
            .filter(|c| c.surface.risk < self.a.ca2_q29_spot_max_risk);
        let across = |best: &mut BestSpot, c: &Crossing| {
            offer(best, &c.surface, false);
            best.best.map(|(_, s)| s)
        };
        if let Some(c) = cross.filter(|c| c.same_level)
            && let Some(s) = across(&mut best, &c)
        {
            return Some(s);
        }
        for surface in surfaces {
            if surface.risk >= self.a.ca2_q29_spot_max_risk
                || (surface.y - self.leader.foot_y).abs() > self.vertical()
            {
                continue;
            }
            offer(&mut best, surface, true);
        }
        let found = best.best.map(|(_, s)| s);
        if found.is_none_or(|s| (s.x - self.leader.x).abs() > max)
            && let Some(c) = cross.filter(|c| !c.same_level)
            && let Some(s) = across(&mut BestSpot { best: None }, &c)
        {
            return Some(s);
        }
        found
    }
}

impl AutonomyState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn goal(&self) -> Goal {
        self.goal
    }

    pub fn target(&self) -> Option<StableEntityId> {
        self.target
    }

    pub fn spot(&self) -> Option<Spot> {
        self.spot
    }

    /// The tick the current goal started on.
    pub fn goal_since(&self) -> u64 {
        self.goal_since
    }

    /// The tick the Hunt recoil ends on (0: none).
    pub fn recoil_until(&self) -> u64 {
        self.recoil_until
    }

    /// The tick a cooldown is over on: `(retreat, show_off, perch)`.
    pub fn ready_at(&self) -> (u64, u64, u64) {
        (
            self.retreat_ready_at,
            self.show_off_ready_at,
            self.perch_ready_at,
        )
    }

    pub fn stats(&self) -> AutonomyStats {
        self.stats
    }

    /// Restore a run's counters into a fresh state (the scene's restart).
    pub fn carry_stats(&mut self, stats: AutonomyStats) {
        self.stats = stats;
    }

    /// Fold the director's decision of this tick into the M16 counters: the scene
    /// calls it after `step_briefed`.
    pub fn record_decision(&mut self, decision: &Decision) {
        if decision.veto == Some(Veto::DriftSkip) {
            self.stats.interrupts_drift_skipped += 1;
        }
    }

    /// DP-E: the goal layer pressed the blink on `tick` for `reason` (DP-Q41:
    /// the decision is hashed state; the counter is the M16 report).
    pub fn note_blink(&mut self, tick: u64, reason: BlinkReason) {
        self.last_blink = Some(tick);
        match reason {
            BlinkReason::Retreat => self.stats.blink_presses_retreat += 1,
            BlinkReason::Committed => self.stats.blink_presses_committed += 1,
        }
    }

    /// The tick of the last blink press (DP-E).
    pub fn last_blink(&self) -> Option<u64> {
        self.last_blink
    }

    /// FA-Q09: the press answered the boss' charge that started at `started`:
    /// the follow layer's dodge jump stays silent until that charge is over.
    pub fn latch_charge_blink(&mut self, started: u64) {
        self.charge_latch = Some(started);
    }

    /// FA-Q09: the dodge jump for the boss' charge is silenced (a blink answered
    /// it).
    pub fn charge_latched(&self) -> bool {
        self.charge_latch.is_some()
    }

    /// FA-Q09: the latch against the charge now up (`started` of the boss'
    /// `Charge` threat, `None` without one): it holds only for the charge it was
    /// set for, and is released when that charge is over or another one began
    /// (chained charges). Returns whether the dodge stays silenced.
    pub fn sync_charge_latch(&mut self, current: Option<u64>) -> bool {
        if self.charge_latch.is_some() && self.charge_latch != current {
            self.charge_latch = None;
        }
        self.charge_latch.is_some()
    }

    /// Forget the plan (a swap, the partner is defeated, the hero stops being
    /// companion-driven). Cooldowns and the hurt and landed-hit ticks survive
    /// (spec 5.4: they are absolute ticks, so they cannot go stale).
    pub fn clear_plan(&mut self) {
        let keep = (
            self.retreat_ready_at,
            self.show_off_ready_at,
            self.perch_ready_at,
            self.last_hurt,
            self.last_landed,
            self.last_blink,
            self.charge_latch,
            self.stats,
        );
        *self = Self::default();
        (
            self.retreat_ready_at,
            self.show_off_ready_at,
            self.perch_ready_at,
            self.last_hurt,
            self.last_landed,
            self.last_blink,
            self.charge_latch,
            self.stats,
        ) = keep;
    }

    /// A respawn, a defeat retry or a restart: everything back to default except
    /// the M16 counters, which are a run's report and never hashed (PR #213
    /// review F-1: a restart must not wipe them).
    pub fn reset(&mut self) {
        let stats = self.stats;
        *self = Self::default();
        self.stats = stats;
    }

    /// Mix the state (the scene folds it into its hash only while the partner
    /// is companion-driven under an autonomy set). Counters and the trace are
    /// never mixed.
    pub fn hash_into(&self, mix: &mut impl FnMut(u64)) {
        let opt = |mix: &mut dyn FnMut(u64), v: Option<u64>| match v {
            None => mix(0),
            Some(t) => {
                mix(1);
                mix(t);
            }
        };
        mix(u64::from(self.started));
        mix(self.goal as u64);
        mix(self.goal_since);
        match self.target {
            None => mix(0),
            Some(id) => {
                mix(1);
                id.hash_into(mix);
            }
        }
        match self.spot {
            None => mix(0),
            Some(s) => {
                mix(1);
                mix(u64::from(s.x.to_bits()));
                mix(u64::from(s.surface_y.to_bits()));
                mix(s.key);
                mix(u64::from(s.leader_ref_x.to_bits()));
                mix(s.purpose as u64);
            }
        }
        opt(mix, self.last_decision);
        mix(self.recoil_until);
        mix(self.retreat_ready_at);
        mix(self.show_off_ready_at);
        mix(self.perch_ready_at);
        opt(mix, self.last_hurt);
        opt(mix, self.last_landed);
        mix(self.travel_dir as u8 as u64);
        mix(u64::from(self.travel_ticks));
        match self.prev {
            None => mix(0),
            Some(p) => {
                mix(1);
                mix(p.tick);
                mix(u64::from(p.health));
                mix(u64::from(p.hurt_ticks));
                match p.move_id {
                    None => mix(0),
                    Some(m) => {
                        mix(1);
                        mix(m as u64);
                    }
                }
                mix(u64::from(p.leader_x.to_bits()));
            }
        }
        mix(self.seen.len() as u64);
        for (id, x) in &self.seen {
            id.hash_into(mix);
            mix(u64::from(x.to_bits()));
        }
        // DP-E: nothing until she has blinked once.
        if let Some(tick) = self.last_blink {
            mix(BLINK_HASH_MARK);
            mix(tick);
        }
        // FA-Q09: nothing unless a charge blink is latched.
        if self.charge_latch.is_some() {
            mix(CHARGE_LATCH_MARK);
        }
        // FA-Q25: nothing unless she is counting a shadow run.
        if self.shadow_ticks > 0 {
            mix(SHADOW_HASH_MARK);
            mix(u64::from(self.shadow_ticks));
        }
    }

    /// FNV-1a 64 over [`Self::hash_into`].
    pub fn state_hash(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        self.hash_into(&mut |value| {
            for byte in value.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        });
        hash
    }

    fn set_goal(&mut self, goal: Goal, tick: u64, a: &SkirmisherTable) {
        // Cooldowns run from the end of a goal (spec 5.1, 5.4).
        match self.goal {
            Goal::Retreat if goal != Goal::Retreat => {
                self.retreat_ready_at = tick + u64::from(a.ca2_q21_cooldown_ticks);
            }
            Goal::ShowOff if goal != Goal::ShowOff => {
                self.show_off_ready_at = tick + u64::from(a.ca2_q24_cooldown_ticks);
            }
            _ => {}
        }
        if goal == Goal::Retreat && self.goal != Goal::Retreat {
            self.stats.retreat_entries += 1;
        }
        self.goal = goal;
        self.goal_since = tick;
    }

    /// This tick's observations (the view's facts across ticks).
    fn observe(
        &mut self,
        inp: &AutonomyInput<'_>,
        me: &HeroView,
        leader: &HeroView,
        t: &SkirmisherTable,
    ) {
        let tick = inp.view.tick;
        let consecutive = self.prev.is_some_and(|p| tick == p.tick + 1);
        if !consecutive {
            // Missed ticks (a swap, a pause in the call): the counters restart.
            self.travel_dir = 0;
            self.travel_ticks = 0;
            self.seen.clear();
        }
        if let Some(p) = self.prev.filter(|_| consecutive) {
            let dx = leader.x - p.leader_x;
            if dx.abs() > 0.01 {
                let dir = sign(dx);
                if dir == self.travel_dir {
                    self.travel_ticks = self.travel_ticks.saturating_add(1);
                } else {
                    self.travel_dir = dir;
                    self.travel_ticks = 1;
                }
            } else {
                self.travel_ticks = 0;
            }
            if me.health < p.health || me.hurt_ticks > p.hurt_ticks {
                self.last_hurt = Some(tick);
            }
            // Her pounce or swat ended: the recoil window starts (CA2-Q19).
            let was_hit_and_run = matches!(p.move_id, Some(MoveId::Pounce | MoveId::Swat));
            let still_same = me.active_move.map(|m| m.id) == p.move_id;
            if was_hit_and_run && !still_same {
                self.recoil_until = tick + u64::from(t.ca2_q19_recoil_ticks);
            }
        }
        // FA-Q25: she shadows the leader while both walk the same way within the
        // radius.
        if inp.set.fix_all.fa_q25_shadow_respot {
            let fa = &inp.set.fix_all;
            let me_dx = self.prev_me_x.filter(|_| consecutive).map(|x| me.x - x);
            let leader_dx = self
                .prev
                .filter(|_| consecutive)
                .map(|p| leader.x - p.leader_x);
            let same_way = matches!((me_dx, leader_dx), (Some(a), Some(b))
                if a.abs() > 0.01 && b.abs() > 0.01 && (a > 0.0) == (b > 0.0));
            if same_way && (me.x - leader.x).abs() <= f32::from(fa.fa_q25_shadow_px) {
                self.shadow_ticks = self.shadow_ticks.saturating_add(1);
            } else {
                self.shadow_ticks = 0;
            }
        }
        self.prev_me_x = Some(me.x);
        if inp.partner_hit_landed {
            self.last_landed = Some(tick);
        }
        self.prev = Some(Seen {
            tick,
            health: me.health,
            hurt_ticks: me.hurt_ticks,
            move_id: me.active_move.map(|m| m.id),
            leader_x: leader.x,
        });
    }

    fn moving_toward_leader(&self, t: &TargetView, leader_x: f32) -> bool {
        self.seen
            .binary_search_by(|(id, _)| id.cmp(&t.id))
            .ok()
            .is_some_and(|i| (t.x - leader_x).abs() < (self.seen[i].1 - leader_x).abs() - 0.01)
    }

    fn remember_targets(&mut self, view: &CompanionView) {
        self.seen.clear();
        self.seen.extend(view.targets.iter().map(|t| (t.id, t.x)));
    }

    /// The Retreat trigger: a hurt in the window with a standing enemy close,
    /// the shared cooldown over (spec 5.1, review F-7).
    fn retreat_trigger(&self, c: &Ctx<'_>) -> bool {
        let tick = c.inp.view.tick;
        let radius = if c.me.health <= c.t.ca2_q21_low_hp {
            c.t.ca2_q21_low_enemy_px
        } else {
            c.t.ca2_q21_hurt_enemy_px
        };
        // FA-Q03 (FA-Q07 tuning iteration 1, recorded): the boss' moves are
        // telegraphed, so she also answers a tell or a committed boss move that is
        // within the hurt radius of her, without waiting for the hurt (the hurt
        // trigger alone leaves Retreat unreachable once she no longer gets hit).
        if c.fa().fa_q03_retreat_on_boss_tell
            && let Some(b) = c.bv
            && !b.charging
            && matches!(b.mode_class, BossModeClass::Tell | BossModeClass::Committed)
            && (b.x - c.me.x).abs() <= f32::from(radius)
            && tick >= self.retreat_ready_at
        {
            return true;
        }
        let Some(hurt) = self.last_hurt else {
            return false;
        };
        tick.saturating_sub(hurt) <= u64::from(c.t.ca2_q21_hurt_window_ticks)
            && tick >= self.retreat_ready_at
            && c.standing_within(c.me.x, f32::from(radius))
    }

    fn retreat_ended(&self, c: &Ctx<'_>) -> bool {
        let tick = c.inp.view.tick;
        tick.saturating_sub(self.goal_since) >= u64::from(c.t.ca2_q21_exit_ticks)
            || !c.standing_within(c.me.x, f32::from(c.t.ca2_q21_exit_clear_px))
    }

    fn threat_aimed(view: &CompanionView) -> bool {
        view.threats.iter().any(|t| t.aimed_at.is_some())
    }

    fn show_off_ended(&self, c: &Ctx<'_>) -> bool {
        let tick = c.inp.view.tick;
        tick.saturating_sub(self.goal_since) >= u64::from(c.t.ca2_q24_hold_ticks)
            || Self::threat_aimed(c.inp.view)
            || self.last_hurt == Some(tick)
    }

    fn show_off_start(&self, c: &Ctx<'_>) -> bool {
        let tick = c.inp.view.tick;
        self.last_landed
            .is_some_and(|l| tick.saturating_sub(l) <= SHOW_OFF_HIT_WINDOW_TICKS)
            && !Self::threat_aimed(c.inp.view)
            && !c.standing_within(c.me.x, f32::from(c.t.ca2_q24_safe_px))
            && tick >= self.show_off_ready_at
            && !c.travelling
    }

    /// Is the current target still one this goal may work (it exists, she can hit
    /// it, and the claim rules still allow it)?
    fn target_valid(&self, c: &Ctx<'_>) -> bool {
        let Some(id) = self.target else {
            return false;
        };
        let Some(t) = c.target_of(id) else {
            return false;
        };
        let t = &t;
        if c.mode_for(t).is_none() {
            return false;
        }
        let claim_ok = match self.goal {
            Goal::Hunt => !(c.t.ca2_q11_exclude_claimed_from_hunt && c.claims.contains(id)),
            Goal::SupportLeader => c.support_allows(t),
            _ => true,
        };
        if !claim_ok {
            return false;
        }
        // Commit-safety of the attack is the spot's business (`spot_valid`, the
        // brief): a spot that stops passing the drift check is re-picked, and the
        // goal falls back when no spot does.
        true
    }

    fn recoil_active(&self, tick: u64) -> bool {
        self.goal == Goal::Hunt && tick < self.recoil_until
    }

    /// Is the held spot still good (spec 5.1: re-picked only when it becomes
    /// invalid, or the leader moved more than CA2-Q30 from its reference)?
    fn spot_valid(&self, c: &Ctx<'_>) -> bool {
        let Some(spot) = self.spot else {
            return true;
        };
        if !c.zone().contains_spot(spot.x, spot.surface_y)
            || (c.leader.x - spot.leader_ref_x).abs() > f32::from(c.a.ca2_q30_respot_px)
            // CA2-C: a hazard that turned up over the held spot (a telegraph
            // begins) makes it a bad place to stand, whatever the goal.
            || c.inp.world.hazard_at(spot.x, spot.surface_y)
        {
            return false;
        }
        let recoil = self.recoil_active(c.inp.view.tick);
        match (self.goal, spot.purpose) {
            (Goal::BossPunish, SpotPurpose::Attack) => {
                // FA-Q04: in the union of the cast and pounce bands, and the
                // commitment of the mode that distance gives still passes the
                // drift check.
                let Some(t) = c.target_of(BOSS_ID) else {
                    return false;
                };
                let d = (spot.x - t.x).abs();
                let (cast_lo, cast_hi) = c.band(HuntMode::Cast);
                let (pounce_lo, pounce_hi) = c.band(HuntMode::Pounce);
                let mode = c.punish_mode_at(spot.x, t.x);
                (cast_lo.min(pounce_lo)..=cast_hi.max(pounce_hi)).contains(&d)
                    && (spot.x - t.x) * (c.me.x - t.x) >= 0.0
                    && c.drift_ok(spot.x, &t, mode)
            }
            (Goal::Hunt | Goal::SupportLeader, SpotPurpose::Attack | SpotPurpose::Recoil) => {
                if (spot.purpose == SpotPurpose::Recoil) != (self.goal == Goal::Hunt && recoil) {
                    return false;
                }
                let Some(t) = self.target.and_then(|id| c.target_of(id)) else {
                    return false;
                };
                let t = &t;
                let Some(mode) = c.mode_for(t) else {
                    return false;
                };
                if spot.purpose == SpotPurpose::Recoil {
                    return true;
                }
                let (lo, hi) = c.band(mode);
                // In band, and the commitment from the spot still passes the
                // drift check (review F-2: a stale spot is re-picked).
                (lo..=hi).contains(&(spot.x - t.x).abs())
                    && c.drift_ok(spot.x, t, mode)
                    && !c.hazard_blocks(spot.x, spot.surface_y, t, mode)
            }
            (Goal::Retreat, SpotPurpose::Retreat) => true,
            // FA-Q05: a boss standoff spot goes stale when the boss closes in.
            (Goal::Escort, SpotPurpose::Escort) => {
                c.bv.is_none_or(|b| !c.standoff_violated(&b, spot.x))
                    && !(c.shadowing
                        && (spot.x - c.leader.x).abs() < f32::from(c.fa().fa_q25_respot_px))
                    // RINGW-F1 (RWF1-AD1): a spot on her side of a barrier that
                    // cuts her off from the leader on one floor goes stale.
                    && c.cut_off()
                        .is_none_or(|x| !x.same_level || x.surface.key == spot.key)
            }
            _ => false,
        }
    }

    /// The step of one tick: observe, apply the immediate triggers, decide when
    /// due, and return the brief. `None` while the layer does not steer her (the
    /// set has it off or no role, she is not companion-driven, or she is down):
    /// the caller then passes no brief to the director.
    pub fn step(&mut self, inp: &AutonomyInput<'_>) -> Option<AutonomyBrief> {
        let a = &inp.set.autonomy;
        let role = match inp.partner {
            HeroId::Carl => inp.set.carl.autonomy_role,
            HeroId::Donut => inp.set.donut.autonomy_role,
        };
        let table = a.table(role).filter(|_| a.enabled)?;
        let view = inp.view;
        let me = view.hero(inp.partner);
        let leader = view.hero(match inp.partner {
            HeroId::Carl => HeroId::Donut,
            HeroId::Donut => HeroId::Carl,
        });
        if me.control != Control::Companion {
            self.clear_plan();
            return None;
        }
        if me.defeated {
            // Nothing is decided while down; the plan is dropped (spec 5.4).
            self.clear_plan();
            return None;
        }
        debug_assert!(
            inp.partner_motion.run_speed * TICK_SECONDS + 1e-4 >= inp.drift.leader_px_per_tick(),
            "the zone proof needs a partner at least as fast as the leader (spec 3.1, N-3)"
        );
        let tick = view.tick;
        self.observe(inp, me, leader, table);
        let mana_cast = me.mana.is_some_and(|m| m.unlocked && m.ai_can_cast);
        let walking = self.travel_ticks >= a.ca2_q07_travel_ticks;
        let travelling = walking || inp.pan_remaining > 0;
        self.stats.travelling_ticks += u32::from(travelling);
        self.stats.pan_only_travelling_ticks += u32::from(travelling && !walking);
        let a_fix = &inp.set.fix_all;
        // FA-Q02: the boss, only with the knob on and a body to speak of.
        let bv = view
            .boss_view
            .filter(|b| inp.set.fix_all.fa_q02_boss_play && b.hurtbox.is_some());
        let ctx = Ctx {
            inp,
            a,
            t: table,
            me,
            leader,
            claims: LeaderClaims::compute(view, inp.set, leader),
            cast_ok: mana_cast,
            boss: view.boss.is_some() || bv.is_some(),
            bv,
            shadowing: a_fix.fa_q25_shadow_respot && self.shadow_ticks > a_fix.fa_q25_shadow_ticks,
            travelling,
            travel_dir: self.travel_dir,
            committed: me.active_move.is_some() || !me.grounded,
        };

        let mut changed = None;
        if leader.defeated {
            // The leader is down: Idle, no new commitment, interrupts only.
            if !self.started || self.goal != Goal::Idle {
                self.set_goal(Goal::Idle, tick, table);
                self.started = true;
                changed = Some(GoalReason::TargetLost);
            }
            self.target = None;
            self.spot = None;
            self.last_decision = Some(tick);
            return Some(self.brief(&ctx, changed));
        }

        let in_zone = inp.zone.contains_x(me.x);
        let cadence = u64::from(inp.set.ca10_decision_cadence_ticks);
        let due = self
            .last_decision
            .is_none_or(|t| tick.saturating_sub(t) >= cadence);

        // Immediate triggers (spec 5.1): they skip the cadence and the minimum
        // goal time.
        let mut cause: Option<GoalReason> = None;
        if !self.started {
            cause = Some(GoalReason::Start);
        } else if !in_zone {
            if self.goal != Goal::Escort || self.spot.is_none() || !self.spot_valid(&ctx) {
                cause = Some(GoalReason::OutsideZone);
            }
        } else if (self.goal == Goal::Retreat && self.retreat_ended(&ctx))
            || (self.goal == Goal::ShowOff && self.show_off_ended(&ctx))
        {
            cause = Some(GoalReason::Ended);
        } else if self.goal != Goal::Retreat
            && self.goal != Goal::ShowOff
            && !ctx.boss_blocks_retreat()
            && self.retreat_trigger(&ctx)
            && Goal::Retreat.row(table).enabled
        {
            cause = Some(GoalReason::Hurt);
        } else if self.goal == Goal::BossPunish && !ctx.punish_window(true) {
            // FA-Q04: it drops out the moment the boss leaves its vulnerable
            // modes; the 24-tick floor is waived for this exit.
            cause = Some(GoalReason::BossWindow);
        } else if due
            && !ctx.committed
            && ctx.bv.is_some()
            && !matches!(self.goal, Goal::Retreat | Goal::BossPunish)
            && ctx.punish_window(false)
        {
            // FA-Q04: a vulnerable window opened: decide now, past the minimum
            // goal time (only at the cadence, so a window she cannot use costs
            // one decision per cadence, not one per tick).
            cause = Some(GoalReason::BossWindow);
        } else if due
            && ctx.bv.is_some_and(|b| ctx.standoff_violated(&b, me.x))
            && matches!(self.goal, Goal::Idle | Goal::Escort)
        {
            // FA-Q05: the boss closed in (or she stands between it and Carl).
            cause = Some(GoalReason::BossStandoff);
        } else if due && ctx.shadowing && matches!(self.goal, Goal::Idle | Goal::Escort) {
            // FA-Q25: she has shadowed him past the threshold: re-pick the escort
            // spot past the radius (immediate, at the cadence).
            cause = Some(GoalReason::Shadow);
        } else if matches!(self.goal, Goal::Hunt | Goal::SupportLeader) && !self.target_valid(&ctx)
        {
            cause = Some(GoalReason::TargetLost);
        } else if !ctx.committed && (due || !self.spot_valid(&ctx)) {
            // No routine re-decision while she is in a move or in the air (spec
            // 3.3: a commitment). A real pounce would otherwise drop the Hunt
            // before the recoil (review F-1). The immediate triggers above stay.
            cause = Some(GoalReason::Utility);
        }

        if let Some(cause) = cause {
            changed = self.decide(&ctx, cause, in_zone);
        }
        Some(self.brief(&ctx, changed))
    }

    /// A full decision: score every eligible goal and take the best, honouring
    /// the stickiness and the minimum goal time.
    fn decide(&mut self, c: &Ctx<'_>, cause: GoalReason, in_zone: bool) -> Option<GoalReason> {
        let tick = c.inp.view.tick;
        // An ended Retreat or Show-off starts its cooldown now, so the same
        // decision cannot pick it up again.
        if cause == GoalReason::Ended {
            match self.goal {
                Goal::Retreat => {
                    self.retreat_ready_at = tick + u64::from(c.t.ca2_q21_cooldown_ticks);
                }
                Goal::ShowOff => {
                    self.show_off_ready_at = tick + u64::from(c.t.ca2_q24_cooldown_ticks);
                }
                _ => {}
            }
        }
        self.stats.decisions += 1;
        self.last_decision = Some(tick);
        let mut surfaces = Vec::new();
        c.inp.world.surfaces(&mut surfaces);
        surfaces.sort_by_key(|s| s.key);
        let immediate = !matches!(cause, GoalReason::Utility);
        let cands = self.candidates(c, &surfaces, in_zone);

        // Hysteresis: the current goal keeps for its minimum time unless an
        // immediate trigger fired (CA2-Q16).
        let held = tick.saturating_sub(self.goal_since) < u64::from(c.a.ca2_q16_min_goal_ticks);
        let keep = (!immediate && self.started && held)
            .then(|| cands.iter().find(|k| k.goal == self.goal).copied())
            .flatten();
        let chosen = keep.or_else(|| {
            cands.iter().copied().reduce(|best, k| {
                if k.utility > best.utility
                    || (k.utility == best.utility && k.goal.tie_rank() < best.goal.tie_rank())
                {
                    k
                } else {
                    best
                }
            })
        })?;

        let was_started = self.started;
        let goal_changed = !was_started || chosen.goal != self.goal;
        let reason = if !was_started {
            GoalReason::Start
        } else {
            cause
        };
        if goal_changed {
            self.set_goal(chosen.goal, tick, c.t);
            self.started = true;
            self.target = chosen.target;
            self.spot = chosen.spot;
        } else {
            // Same goal: keep a still-valid spot and target (no twitching);
            // take the fresh ones when the target changed or the spot is stale.
            if self.target != chosen.target {
                self.target = chosen.target;
                self.spot = chosen.spot;
            } else if !self.spot_valid(c) || self.spot.is_none() {
                self.spot = chosen.spot;
            }
        }
        // What "closing on the leader's back" is measured against: the targets as
        // of this decision (the next one compares with it).
        self.remember_targets(c.inp.view);
        goal_changed.then_some(reason)
    }

    fn candidates(&mut self, c: &Ctx<'_>, surfaces: &[Surface], in_zone: bool) -> Vec<Cand> {
        let tick = c.inp.view.tick;
        let table = c.t;
        let stick = |goal: Goal, base: i32, current: Goal| -> i32 {
            base + if goal == current {
                i32::from(c.a.ca2_q16_stickiness)
            } else {
                0
            }
        };
        let allowed = |goal: Goal| -> bool {
            let row = goal.row(table);
            row.enabled && (!c.boss || row.under_boss)
        };
        let util = |goal: Goal| i32::from(goal.row(table).utility);
        let mut out = Vec::new();

        // Outside `Z` only Escort is eligible, and it wins at once.
        if !in_zone {
            if let Some(spot) = c.escort_spot(surfaces) {
                out.push(Cand {
                    goal: Goal::Escort,
                    utility: i32::MAX,
                    target: None,
                    spot: Some(spot),
                });
            } else {
                out.push(Cand {
                    goal: Goal::Escort,
                    utility: i32::MAX,
                    target: None,
                    spot: None,
                });
            }
            return out;
        }

        // Retreat.
        if allowed(Goal::Retreat) {
            let active = self.goal == Goal::Retreat && !self.retreat_ended(c);
            if (active || self.retreat_trigger(c))
                && let Some(spot) = if active && self.spot.is_some() && self.spot_valid(c) {
                    self.spot
                } else {
                    c.retreat_spot(surfaces)
                }
            {
                out.push(Cand {
                    goal: Goal::Retreat,
                    utility: stick(Goal::Retreat, util(Goal::Retreat), self.goal),
                    target: None,
                    spot: Some(spot),
                });
            }
        }

        // FA-Q04: punish a vulnerable boss (the knob's goal; `bv` is `None` with
        // it off).
        if c.bv.is_some() && allowed(Goal::BossPunish) {
            let running = self.goal == Goal::BossPunish;
            if c.punish_window(running) {
                let spot = if running && self.spot.is_some() && self.spot_valid(c) {
                    self.spot
                } else {
                    c.punish_spot(surfaces)
                };
                if let Some(spot) = spot {
                    out.push(Cand {
                        goal: Goal::BossPunish,
                        utility: stick(Goal::BossPunish, util(Goal::BossPunish), self.goal),
                        target: Some(BOSS_ID),
                        spot: Some(spot),
                    });
                }
            }
        }

        // Support-leader (spec 4.3): the enemy nearest the leader that she may
        // take, with an attack spot.
        if allowed(Goal::SupportLeader) {
            let mut best: Option<(f32, &TargetView, Spot)> = None;
            for t in &c.inp.view.targets {
                if !c.support_allows(t) {
                    continue;
                }
                let Some(mode) = c.mode_for(t) else { continue };
                let Some(spot) = c.attack_spot(surfaces, t, mode, false).spot else {
                    continue;
                };
                let d = (t.x - c.leader.x).abs();
                // Unclaimed enemies first, then the nearest to the leader.
                let rank = d + if c.claims.contains(t.id) {
                    10_000.0
                } else {
                    0.0
                };
                if best.is_none_or(|(r, ..)| rank < r) {
                    best = Some((rank, t, spot));
                }
            }
            if let Some((_, t, spot)) = best {
                out.push(Cand {
                    goal: Goal::SupportLeader,
                    utility: stick(Goal::SupportLeader, util(Goal::SupportLeader), self.goal),
                    target: Some(t.id),
                    spot: Some(spot),
                });
            }
        }

        // Hunt: an unclaimed candidate, scored by spec 4.2.
        if allowed(Goal::Hunt) {
            let mut best: Option<(i32, &TargetView, Spot)> = None;
            for t in &c.inp.view.targets {
                if t.hp == 0 {
                    continue;
                }
                self.stats.hunt_evaluations += 1;
                if table.ca2_q11_exclude_claimed_from_hunt && c.claims.contains(t.id) {
                    self.stats.hunt_claim_excluded += 1;
                    continue;
                }
                let Some(mode) = c.mode_for(t) else { continue };
                let search = c.attack_spot(surfaces, t, mode, false);
                let Some(spot) = search.spot else {
                    self.stats.hunt_drift_rejected += u32::from(search.drift_rejected);
                    self.stats.hunt_hazard_rejected += u32::from(search.hazard_rejected);
                    continue;
                };
                let base = match (t.id.kind, t.state) {
                    (EntityKind::Llama, _) => i32::from(table.ca2_q14_base_llama),
                    (_, TargetState::Lying { .. } | TargetState::Staggered { .. }) => {
                        i32::from(table.ca2_q14_base_lying)
                    }
                    _ => i32::from(table.ca2_q14_base_standing),
                };
                let isolation = ipx((t.x - c.leader.x)
                    .abs()
                    .min(f32::from(table.ca2_q14_isolation_cap_px)))
                    / i32::from(table.ca2_q14_isolation_divisor.max(1));
                let behind =
                    sign(t.x - c.leader.x) != c.leader.facing && (t.x - c.leader.x).abs() > 0.0;
                let back = if behind && self.moving_toward_leader(t, c.leader.x) {
                    i32::from(table.ca2_q15_back_threat_bonus)
                } else {
                    0
                };
                let travel =
                    ipx((c.me.x - t.x).abs()) / i32::from(table.ca2_q14_travel_divisor.max(1));
                let sticky = if self.target == Some(t.id) && self.goal == Goal::Hunt {
                    i32::from(c.a.ca2_q16_stickiness)
                } else {
                    0
                };
                let score = base + isolation + back - travel + sticky;
                // Ties by stable id: the view is id-ordered, so a strict `>`.
                if best.is_none_or(|(s, ..)| score > s) {
                    best = Some((score, t, spot));
                }
            }
            if let Some((_, t, spot)) = best {
                // The recoil spot for any target while the window runs (review
                // N-b): a Hunt that moved to a new target springs back first.
                let recoil = self.recoil_active(tick);
                let spot = if recoil {
                    let mode = c.mode_for(t).unwrap_or(HuntMode::Pounce);
                    c.attack_spot(surfaces, t, mode, true).spot.or(Some(spot))
                } else {
                    Some(spot)
                };
                out.push(Cand {
                    goal: Goal::Hunt,
                    utility: stick(Goal::Hunt, util(Goal::Hunt), self.goal),
                    target: Some(t.id),
                    spot,
                });
            }
        }

        // Escort when she has an escort to do: the leader travels, or she is past
        // the comfort radius (a leader who is low pulls her closer, to the escort
        // band). A partner who is already comfortable and in no hurry holds, which
        // is Idle: adopted default AD-B4. Spec 5.1 lists Escort (800) as "always
        // eligible", which would put it above Show-off (700) and Perch (600) in
        // every quiet moment, so neither could ever win; this keeps the table's
        // numbers and makes them reachable (flagged to the top tier in the PR).
        // While escorting, the hold radius is the escort band (review F-4): she walks
        // back into it before she holds, so the minimum goal time applies.
        let escorting = self.started && self.goal == Goal::Escort;
        let hold_radius = if c.leader_low() || escorting {
            f32::from(c.a.ca2_q07_escort_max_px)
        } else {
            f32::from(c.a.ca2_q06_comfort_px)
        };
        // FA-Q05: under a boss she holds only on ground the standoff allows.
        // RINGW-F1 (RWF1-AD1): she does not hold on her side of a barrier that
        // cuts her off from the leader on one floor; she escorts across it.
        let hold = !c.travelling
            && (c.me.x - c.leader.x).abs() <= hold_radius
            && c.bv.is_none_or(|b| !c.standoff_violated(&b, c.me.x))
            && c.cut_off().is_none_or(|x| !x.same_level);
        // A running Escort that has just arrived keeps its minimum time (CA2-Q16)
        // as a hold, so it does not flip to Idle the tick she enters the band.
        let settling = escorting
            && tick.saturating_sub(self.goal_since) < u64::from(c.a.ca2_q16_min_goal_ticks);
        if allowed(Goal::Escort) && (!hold || settling) {
            // FA-Q05: the travel bonus is suspended under a boss.
            let bonus = if c.travelling && c.bv.is_none() {
                // FA-Q28: a smaller bonus while an unclaimed Hunt candidate is near
                // her, so the hunt can win over the escort.
                let near = c.fa().fa_q28_hunt_escort_bonus
                    && c.inp.view.targets.iter().any(|t| {
                        t.hp > 0
                            && !c.claims.contains(t.id)
                            && (t.x - c.me.x).abs() <= f32::from(c.fa().fa_q28_hunt_near_px)
                            && c.mode_for(t).is_some()
                    });
                if near {
                    i32::from(c.fa().fa_q28_escort_bonus)
                } else {
                    i32::from(c.a.ca2_q07_escort_bonus)
                }
            } else {
                0
            };
            let keep = self.goal == Goal::Escort && self.spot.is_some() && self.spot_valid(c);
            let spot = if hold {
                None
            } else if keep {
                self.spot
            } else {
                c.escort_spot(surfaces)
            };
            out.push(Cand {
                goal: Goal::Escort,
                utility: stick(Goal::Escort, util(Goal::Escort) + bonus, self.goal),
                target: None,
                spot,
            });
        }

        // Show-off: stand still after her hit landed and nothing threatens her.
        if allowed(Goal::ShowOff) {
            let active = self.goal == Goal::ShowOff && !self.show_off_ended(c);
            if active || self.show_off_start(c) {
                out.push(Cand {
                    goal: Goal::ShowOff,
                    utility: stick(Goal::ShowOff, util(Goal::ShowOff), self.goal),
                    target: None,
                    spot: None,
                });
            }
        }

        // Perch is CA2-C. Idle: always.
        if allowed(Goal::Idle) {
            out.push(Cand {
                goal: Goal::Idle,
                utility: stick(Goal::Idle, util(Goal::Idle), self.goal),
                target: None,
                spot: None,
            });
        }
        out
    }

    /// The brief for this tick from the state.
    fn brief(&mut self, c: &Ctx<'_>, changed: Option<GoalReason>) -> AutonomyBrief {
        let tick = c.inp.view.tick;
        let recoil = self.recoil_active(tick) && self.target.is_some();
        let target = self.target.and_then(|id| c.target_of(id));
        let target = target.as_ref();
        let mut director = match (self.goal, target) {
            (Goal::Hunt, Some(t)) if !recoil => c
                .mode_for(t)
                .map(|m| {
                    let (lo, hi) = c.band(m);
                    GoalBrief::targeting(t.id, lo, hi)
                })
                .unwrap_or_else(GoalBrief::interrupts_only),
            (Goal::SupportLeader, Some(t)) => c
                .mode_for(t)
                .map(|m| {
                    let (lo, hi) = c.band(m);
                    GoalBrief::targeting(t.id, lo, hi)
                })
                .unwrap_or_else(GoalBrief::interrupts_only),
            // FA-Q04: the union of the cast and pounce bands; the director's own
            // per-action reach picks the legal action at her distance.
            (Goal::BossPunish, Some(t)) => {
                let (cast_lo, cast_hi) = c.band(HuntMode::Cast);
                let (pounce_lo, pounce_hi) = c.band(HuntMode::Pounce);
                GoalBrief::targeting(t.id, cast_lo.min(pounce_lo), cast_hi.max(pounce_hi))
            }
            (Goal::ShowOff, _) => GoalBrief::silent(),
            _ => GoalBrief::interrupts_only(),
        };
        // The drift check from where she stands now, every tick (review F-2): the
        // director presses from anywhere inside both bands, which is wider than
        // the spot that was checked. When the commitment would leave the view the
        // brief allows interrupts only (the goal, target and spot are kept).
        if !director.allowed.is_empty()
            && let Some(t) = target
            && let Some(mode) = c.mode_in_brief(t)
            && !c.drift_ok(c.me.x, t, mode)
        {
            director = GoalBrief::interrupts_only();
            if !c.committed {
                self.stats.brief_drift_blocked_ticks += 1;
            }
        }
        // CA2-C: the same from where she stands now, every tick: a hazard that
        // began between her and the target keeps her from pouncing across it.
        if !director.allowed.is_empty()
            && let Some(t) = target
            && let Some(mode) = c.mode_in_brief(t)
            && c.hazard_blocks(c.me.x, c.me.foot_y, t, mode)
        {
            director = GoalBrief::interrupts_only();
            if !c.committed {
                self.stats.brief_hazard_blocked_ticks += 1;
            }
        }
        director.commit_x = Some(c.interrupt_commit_x());
        let face_x = match self.goal {
            Goal::Hunt | Goal::SupportLeader | Goal::BossPunish => target.map(|t| t.x),
            Goal::ShowOff => Some(c.leader.x),
            _ => None,
        };
        AutonomyBrief {
            goal: self.goal,
            changed,
            target: self.target,
            spot: self.spot,
            face_x,
            recoil: self.goal == Goal::Hunt && recoil,
            in_zone: c.inp.zone.contains_x(c.me.x),
            director,
        }
    }
}

impl Ctx<'_> {
    /// Under a boss only the goals with `under_boss` are eligible (CA2-Q35,
    /// applied in `candidates`); Retreat is one of them, so the trigger stays.
    /// With boss play (FA-Q03) the "path crosses a Committed hitbox" check lives
    /// in [`Self::retreat_spot`] (such a spot is no spot), so the gate itself
    /// never blocks.
    fn boss_blocks_retreat(&self) -> bool {
        if self.bv.is_some() {
            return false;
        }
        self.boss && !Goal::Retreat.row(self.t).under_boss
    }

    fn fa(&self) -> &FixAll {
        &self.inp.set.fix_all
    }

    /// FA-Q05: the near edge of the standoff band from the boss: the plan's
    /// 112 px, and from phase 2 also clear of the shockwave ripples' reach
    /// (the recorded deviation).
    fn standoff_min(&self, b: &BossView) -> f32 {
        let fa = self.fa();
        let base = f32::from(fa.fa_q05_escort_boss_min_px);
        if b.ripple_reach_px > 0.0 {
            base.max(b.ripple_reach_px + HERO_HALF_WIDTH + f32::from(fa.fa_q05_ripple_margin_px))
        } else {
            base
        }
    }

    /// FA-Q05: the far edge of the standoff band (the band keeps the plan's
    /// 112..160 width).
    fn standoff_max(&self, b: &BossView) -> f32 {
        let fa = self.fa();
        self.standoff_min(b)
            + f32::from(fa.fa_q05_escort_boss_max_px - fa.fa_q05_escort_boss_min_px)
    }

    /// `x` lies strictly between the boss and the leader.
    fn between_boss_and_leader(&self, b: &BossView, x: f32) -> bool {
        x > b.x.min(self.leader.x) && x < b.x.max(self.leader.x)
    }

    /// A body at `x` overlaps the charge lane: the ram box (x 36..82) projected
    /// ahead of the boss (FA-Q03).
    fn in_lane(&self, b: &BossView, x: f32) -> bool {
        let (lo, hi) = b.ram_box;
        x + HERO_HALF_WIDTH > lo && x - HERO_HALF_WIDTH < hi
    }

    /// FA-Q03: a walk from her feet to `x` crosses a Committed boss hitbox.
    fn path_crosses_committed(&self, b: &BossView, x: f32) -> bool {
        b.committed_box.is_some_and(|(lo, hi)| {
            let (a, z) = (self.me.x.min(x), self.me.x.max(x));
            z + HERO_HALF_WIDTH > lo && a - HERO_HALF_WIDTH < hi
        })
    }

    /// FA-Q05: she stands where the standoff forbids: closer than the band, or
    /// between the boss and the leader. (The charge lane is the spot search's
    /// business: from inside it a dodge, not a walk, is the answer.)
    fn standoff_violated(&self, b: &BossView, x: f32) -> bool {
        // While a charge is up she holds and dodges (the follow layer's jump):
        // walking away from a ram only leaves her on the wrong side of the crash.
        !b.charging
            && ((x - b.x).abs() < self.standoff_min(b) || self.between_boss_and_leader(b, x))
    }

    /// FA-Q05: the boss escort (Standoff) spot. 112..160 px from the boss (from
    /// phase 2 beyond the ripples), never between the boss and the leader, on her
    /// own side of the boss, out of the charge lane while a tell or a committed
    /// move is up, and near the leader.
    fn boss_escort_spot(&self, surfaces: &[Surface], b: &BossView) -> Option<Spot> {
        let zone = self.zone();
        let (min, max) = (self.standoff_min(b), self.standoff_max(b));
        let mid = (min + max) / 2.0;
        let lane_matters = matches!(b.mode_class, BossModeClass::Tell | BossModeClass::Committed);
        let mut best = BestSpot { best: None };
        for surface in surfaces {
            if surface.risk >= self.a.ca2_q29_spot_max_risk
                || (surface.y - self.leader.foot_y).abs() > self.vertical()
            {
                continue;
            }
            sample_xs(surface, |x| {
                if !zone.contains_spot(x, surface.y) || !self.commit_ok(x, surface, None).0 {
                    return;
                }
                let from_boss = (x - b.x).abs();
                if self.between_boss_and_leader(b, x)
                    || (x - b.x) * (self.me.x - b.x) < 0.0
                    || (lane_matters && self.in_lane(b, x))
                {
                    return;
                }
                let from_leader = (x - self.leader.x).abs();
                // Inside the band is best; beyond it she is merely farther; closer
                // than the band costs four times its shortfall, so with no ground
                // in the band (a corner) she takes the farthest she can reach.
                let outside = (from_boss - mid).abs() - (max - min) / 2.0;
                let shortfall = (min - from_boss).max(0.0);
                let score = -ipx(outside.max(0.0))
                    - 4 * ipx(shortfall)
                    - FreeZone::pull_penalty(self.a, from_leader);
                best.offer(
                    score,
                    Spot {
                        x,
                        surface_y: surface.y,
                        key: surface.key,
                        leader_ref_x: self.leader.x,
                        purpose: SpotPurpose::Escort,
                    },
                );
            });
        }
        best.best.map(|(_, s)| s)
    }

    /// FA-Q04: BossPunish may start: the boss is vulnerable with at least
    /// `fa_q04_punish_min_ticks` left; once running it lasts while the boss stays
    /// vulnerable (the floor is waived for the exit).
    fn punish_window(&self, running: bool) -> bool {
        self.bv.is_some_and(|b| {
            b.mode_class == BossModeClass::Vulnerable
                // A pound recovery with ripples on their way (phase 2 on) is no
                // window: walking to a spot would cross them (recorded deviation).
                && !(b.pound_recovery && b.ripple_reach_px > 0.0)
                && b.vulnerable_ticks_left
                    >= if running {
                        1
                    } else {
                        self.fa().fa_q04_punish_min_ticks
                    }
        })
    }

    /// The BossPunish attack spot on her own side of the boss (she cannot cross its
    /// body): the cast spacing (112 px) when she can cast, else, or when no ground
    /// on her side holds that spacing (a corner), the pounce band (FA-Q04: "or the
    /// pounce band of 56 when she is dry").
    fn punish_spot(&self, surfaces: &[Surface]) -> Option<Spot> {
        let b = self.bv?;
        let t = self.target_of(BOSS_ID)?;
        let side: Vec<Surface> = surfaces
            .iter()
            .filter_map(|s| {
                let (left, right) = if self.me.x < b.x {
                    (s.left, s.right.min(b.x))
                } else {
                    (s.left.max(b.x), s.right)
                };
                (right >= left).then_some(Surface { left, right, ..*s })
            })
            .collect();
        if self.cast_ok
            && let Some(spot) = self.attack_spot(&side, &t, HuntMode::Cast, false).spot
        {
            return Some(spot);
        }
        self.attack_spot(&side, &t, HuntMode::Pounce, false).spot
    }

    /// [`Self::mode_for`], except that the boss (BossPunish) is hit in the mode her
    /// distance from it gives.
    fn mode_in_brief(&self, t: &TargetView) -> Option<HuntMode> {
        if t.id == BOSS_ID {
            self.mode_for(t)
                .map(|_| self.punish_mode_at(self.me.x, t.x))
        } else {
            self.mode_for(t)
        }
    }

    /// How she would hit the boss from `x`: a cast when she can and the cast band
    /// holds, else the pounce (BossPunish is not tied to one mode: the director
    /// presses whichever legal action the distance allows).
    fn punish_mode_at(&self, x: f32, boss_x: f32) -> HuntMode {
        let (lo, hi) = self.band(HuntMode::Cast);
        if self.cast_ok && (lo..=hi).contains(&(x - boss_x).abs()) {
            HuntMode::Cast
        } else {
            HuntMode::Pounce
        }
    }
}

// ---------------------------------------------------------------------------
// DONUT-PROG-01 DP-E: the escape-only blink (DP-Q39, DP-Q40, DP-Q41, DP-Q46).
// ---------------------------------------------------------------------------

/// Marker word of the blink decision in the autonomy hash ("BLNK-AI").
const BLINK_HASH_MARK: u64 = 0x424c_4e4b_4149_0001;

/// Marker word of the shadow counter in the autonomy hash (FA-Q25).
const SHADOW_HASH_MARK: u64 = 0x5348_4144_4f57_0001;

/// Marker word of the per-charge dodge latch in the autonomy hash (FA-Q09).
const CHARGE_LATCH_MARK: u64 = 0x4c41_5443_4800_0001;

/// How many ticks past the windup a committed attack may still be from its
/// impact for the blink to be eligible (DP-Q39: "impact due within the windup
/// plus 6 ticks").
pub const BLINK_IMPACT_LEAD_TICKS: u16 = 6;

/// What made the goal layer press the blink (DP-Q39).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlinkReason {
    /// (a) The Retreat goal is active and a standing enemy is on top of her.
    Retreat,
    /// (b) A committed attack is aimed at her and its impact is due inside the
    /// window, and the follow layer's jump dodge would not clear it.
    Committed,
}

/// The numbers of the AI blink decision: the profile's three knobs (DP-Q41),
/// the hull of DP-Q46 and the blink's own timeline (DP-Q25), all data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlinkKnobs {
    /// `blink_threat_px` (DP-Q39: 48).
    pub threat_px: f32,
    /// `blink_min_gain_px` (DP-Q39: 64).
    pub min_gain_px: f32,
    /// `dpq46_ai_hull_px` (63).
    pub hull_px: f32,
    /// `dpq25_windup_ticks` (6): the arrival is this many ticks after the press.
    pub windup_ticks: u16,
    /// Windup plus recovery: the commitment the drift check reads (DP-Q39: 12).
    pub commit_ticks: u16,
    /// FA-Q09: the boss-charge window opens at `ai_charge_jump_lead` plus this
    /// many ticks (the follow layer's dodge jump fires at the lead, so the
    /// shipped window of windup plus 6 was dead). `None`: the shipped window.
    pub charge_window_extra: Option<u16>,
    /// FA-Q10: a goblin's swing triggers the blink only if her hurtbox overlaps
    /// its hitbox, widened by this margin (px). `None`: distance alone.
    pub reach_margin_px: Option<f32>,
}

/// The blink the goal layer wants to press this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlinkChoice {
    /// The nearest eligible threat was the boss' charge (FA-Q09: the press sets
    /// the per-charge latch that silences the follow layer's dodge jump).
    pub boss_charge: bool,
    /// -1 left, +1 right: away from the threat.
    pub dir: i8,
    /// Where the pure search says she lands (read-only prediction).
    pub to_x: f32,
    pub threat_x: f32,
    pub reason: BlinkReason,
}

/// Everything [`choose_blink`] reads.
pub struct BlinkInput<'a> {
    pub view: &'a CompanionView,
    pub partner: HeroId,
    pub goal: Goal,
    pub zone: &'a FreeZone,
    pub drift: &'a DriftCheck,
    pub knobs: BlinkKnobs,
    /// The follow layer's jump dodge fires this tick (it would clear a swing
    /// that is still far enough off, `follower`'s tell rule).
    pub dodge_jumps: bool,
}

/// DP-Q39 / DP-Q40 / DP-Q46: should she blink out of this? A pure function of
/// the view; `predict(dir)` is the scene's read-only destination search that
/// also answers whether the press would be accepted now (capability, cooldown,
/// mana, busy, destination), so a candidate the press gate would refuse is never
/// chosen.
///
/// Eligible threats, nearest first (ties by id order):
/// - (a) with Retreat active, a **standing** enemy within `threat_px`;
/// - (b) a committed attack aimed at her within `threat_px` whose impact is more
///   than the windup away (the arrival comes first) and at most the windup plus
///   [`BLINK_IMPACT_LEAD_TICKS`] away, while the follow layer's jump dodge does
///   not fire and she is not already above the swing box (a dodge jump under
///   way clears it);
/// - (b) likewise the boss's committed charge (`view.boss`, DP-F): the ram's
///   leading edge within `threat_px`, its arrival (the rest of the tell plus
///   the run to her) in the same window, she is grounded and the jump dodge is
///   silent.
///
/// Direction is away from the threat (toward the leader when level with it). The
/// predicted destination must add at least `min_gain_px` of distance from the
/// threat, lie inside the free zone `Z`, lie in the hull of her position and
/// the leader's `hull_px` region (DP-Q46), and pass the drift check with the
/// 12-tick commitment. Never for offence, as a route, to chase or in 2P (the
/// scene only calls this for the autonomous partner of a solo run).
pub fn choose_blink(
    input: &BlinkInput<'_>,
    predict: impl Fn(i8) -> Option<f32>,
) -> Option<BlinkChoice> {
    let view = input.view;
    let me = view.hero(input.partner);
    let leader = view.hero(input.partner.other());
    if me.defeated || me.hurt_ticks > 0 || me.active_move.is_some() {
        return None;
    }
    let k = &input.knobs;
    // The nearest eligible threat: (distance, x, reason, is the boss' charge).
    let mut best: Option<(f32, f32, BlinkReason, bool)> = None;
    let mut consider = |x: f32, reason: BlinkReason, boss: bool| {
        let d = (x - me.x).abs();
        if d <= k.threat_px && best.is_none_or(|(bd, ..)| d < bd) {
            best = Some((d, x, reason, boss));
        }
    };
    for threat in &view.threats {
        // A jump already under way that is above the swing box clears it (the
        // follow layer's M3 dodge, started on tell ticks 0 to 20): the blink
        // would only waste the mana and the cooldown.
        let above = me.foot_y > threat.foot_y + crate::feel::SWING_Y_MAX;
        if threat.aimed_at != Some(input.partner) || input.dodge_jumps || above {
            continue;
        }
        // FA-Q10: a swing that cannot reach her does not spend the charge: her
        // hurtbox must overlap the swing's hitbox (static for a goblin, so the
        // sweep over the remaining active ticks is the box itself) widened by the
        // margin.
        if let (Some(margin), Some((lo, hi))) = (k.reach_margin_px, threat.swing_x)
            && (me.x + HERO_HALF_WIDTH < lo - margin || me.x - HERO_HALF_WIDTH > hi + margin)
        {
            continue;
        }
        let left = threat.impact_tick.saturating_sub(threat.tell_tick);
        if left > k.windup_ticks && left <= k.windup_ticks + BLINK_IMPACT_LEAD_TICKS {
            consider(threat.x, BlinkReason::Committed, false);
        }
    }
    // DP-F (DP-Q39 b, CA2-Q35): the boss's committed charge. The ram's leading
    // edge stands in for the threat's position, and its arrival is read the way
    // the follow layer's boss dodge reads it (`follower_input_boss`): the rest of
    // the tell plus the run to her. Never from the air (a jump under way clears
    // the ram box) and never while the follow layer's jump dodge fires.
    if me.grounded
        && !input.dodge_jumps
        && let Some(BossThreat::Charge {
            tell_left,
            x,
            dir,
            front,
            depth,
            speed,
            lead_ticks,
            ..
        }) = view.boss
        && speed > 0.0
    {
        let lead = x + f32::from(dir) * front;
        let gap = (me.x - lead) * f32::from(dir) - HERO_HALF_WIDTH;
        if gap > -(depth + 2.0 * HERO_HALF_WIDTH) {
            let arrival = f32::from(tell_left) + (gap.max(0.0) / speed).ceil();
            // FA-Q09: the window closes at the follow layer's jump lead plus a
            // margin (20), not at windup plus 6 (12): the dodge jumps at 18, so
            // the shipped window never opened.
            let latest = k
                .charge_window_extra
                .map_or(k.windup_ticks + BLINK_IMPACT_LEAD_TICKS, |extra| {
                    lead_ticks + extra
                });
            if arrival > f32::from(k.windup_ticks) && arrival <= f32::from(latest) {
                consider(lead, BlinkReason::Committed, true);
            }
        }
    }
    if input.goal == Goal::Retreat {
        for target in &view.targets {
            if target.state == TargetState::Standing {
                consider(target.x, BlinkReason::Retreat, false);
            }
        }
    }
    let (_, threat_x, reason, boss_charge) = best?;
    let dir: i8 = if (me.x - threat_x).abs() < 0.5 {
        // Level with it: toward the leader, else against her facing.
        if (leader.x - me.x).abs() < 0.5 {
            if me.facing >= 0 { -1 } else { 1 }
        } else {
            sign(leader.x - me.x)
        }
    } else {
        sign(me.x - threat_x)
    };
    let to_x = predict(dir)?;
    let gain = (to_x - threat_x).abs() - (me.x - threat_x).abs();
    if gain < k.min_gain_px || !input.zone.contains_x(to_x) {
        return None;
    }
    // DP-Q46: the hull of her position and the leader's guaranteed region.
    let lo = me.x.min(leader.x - k.hull_px);
    let hi = me.x.max(leader.x + k.hull_px);
    if to_x < lo || to_x > hi {
        return None;
    }
    if !input.drift.allows(input.zone, to_x, k.commit_ticks) {
        return None;
    }
    Some(BlinkChoice {
        boss_charge,
        dir,
        to_x,
        threat_x,
        reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_lab::camera_op::PAN_RATE;
    use crate::companion::nav::{NavConfig, NavGraph, NavLevel, NavSolid};
    use crate::companion::profile::{ProfileFile, ProfileSetId};
    use crate::companion::view::{Control, EntityKind, fixtures};
    use crate::corridor::SceneSim;
    use crate::env_kit::EnvKit;
    use crate::input::RawInput;
    use crate::party::HeroId;
    use crate::prototype::{RUN_SPEED, Rect};
    use crate::scroll_camera::{MAX_SPEED_PX, ScrollCamera};

    fn free_set() -> ProfileSet {
        ProfileFile::pinned()
            .unwrap()
            .set(ProfileSetId::Cf02Free)
            .clone()
    }

    fn knobs() -> Autonomy {
        free_set().autonomy
    }

    fn cfg(layout_w: i32) -> CameraConfig {
        CameraConfig {
            view_w: 640,
            view_h: 360,
            layout_w,
            layout_h: 400,
            view_top: 20,
        }
    }

    fn rows() -> ViewRows {
        ViewRows::of_camera(&cfg(2048))
    }

    fn input(leader_x: f32, left: f32) -> ZoneInput {
        ZoneInput {
            leader_x,
            leader_foot_y: 40.0,
            view: ViewBounds {
                left,
                right: left + 640.0,
            },
            rows: rows(),
        }
    }

    /// The agile partner motion and Carl's: the two run speeds a leader or a
    /// partner can have in the shipped game (DM-A).
    fn motions() -> [(&'static str, HeroMotion); 2] {
        [
            ("legacy", HeroMotion::CARL),
            ("agile", crate::agility::agile_motion()),
        ]
    }

    // ------------------------------------------------------------- zone ----

    #[test]
    fn trail_cap_is_the_camera_geometry_and_the_default_is_336() {
        assert_eq!(trail_cap_px(640.0), 336.0);
        assert_eq!(trail_cap_px(640.0), 320.0 + 40.0 - 24.0);
        // The formula is not a number: another view width moves it.
        assert_eq!(trail_cap_px(960.0), 496.0);
    }

    /// The zone's proof (spec 3.1): the real `ScrollCamera` settles with its
    /// trailing edge `trail_cap` behind a leader who runs away, and a partner
    /// inside `Z` who keeps her distance stays in view. The leader starts at
    /// every offset inside the deadzone, facing either way, and runs either way
    /// for 300 ticks, at the run speed of either shipped motion.
    #[test]
    fn the_zone_keeps_a_partner_in_view_against_the_real_scroll_camera() {
        let a = knobs();
        let config = cfg(4096);
        let cap = trail_cap_px(640.0);
        for (name, motion) in motions() {
            let v = motion.run_speed * TICK_SECONDS;
            assert!(v < MAX_SPEED_PX as f32, "{name}: faster than the camera");
            for facing in [-1_i8, 1] {
                for offset in (-40..=40).step_by(4) {
                    for dir in [-1.0_f32, 1.0] {
                        let base = 2000.0;
                        let mut camera = ScrollCamera::new(config, base, facing);
                        let mut x = base + offset as f32;
                        let left0 = camera.left() as f32;
                        let initial_trail = if dir > 0.0 {
                            x - left0
                        } else {
                            left0 + 640.0 - x
                        };
                        let zone0 = FreeZone::compute(&input(x, left0), &a);
                        // Two partners who run with him at his speed, keeping
                        // their distance: one behind him (the trailing side),
                        // one ahead of him (the leading side).
                        let (trail_reach, lead_reach) = if dir > 0.0 {
                            (zone0.reach_left, zone0.reach_right)
                        } else {
                            (zone0.reach_right, zone0.reach_left)
                        };
                        let behind_gap = trail_reach;
                        let ahead_gap = lead_reach;
                        for tick in 0..300 {
                            x += dir * v;
                            camera.step(x, if dir > 0.0 { 1 } else { -1 });
                            let left = camera.left() as f32;
                            let right = left + 640.0;
                            let trail = if dir > 0.0 { x - left } else { right - x };
                            // Half a pixel of `round` in the camera's lead.
                            assert!(
                                trail >= initial_trail.min(cap) - 0.5,
                                "{name} facing {facing} offset {offset} dir {dir} tick {tick}: \
                                 trailing edge {trail} < min({initial_trail}, {cap})"
                            );
                            let behind_x = x - dir * behind_gap;
                            let ahead_x = x + dir * ahead_gap;
                            let ahead_in = ahead_x >= left + HERO_HALF_WIDTH
                                && ahead_x <= right - HERO_HALF_WIDTH;
                            assert!(
                                behind_x >= left + HERO_HALF_WIDTH
                                    && behind_x <= right - HERO_HALF_WIDTH,
                                "{name} facing {facing} offset {offset} dir {dir} tick {tick}: \
                                 the trailing partner left the view"
                            );
                            assert!(ahead_in, "{name} tick {tick}: the leading partner left");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_zone_is_the_leash_on_both_sides_when_the_leader_is_mid_view() {
        let a = knobs();
        // The camera settled on a leader facing right: edge 336 behind, 304 ahead.
        let zone = FreeZone::compute(&input(1000.0, 664.0), &a);
        assert_eq!((zone.safe_left, zone.safe_right), (292.0, 260.0));
        assert_eq!((zone.reach_left, zone.reach_right), (240.0, 240.0));
        assert_eq!((zone.left, zone.right), (760.0, 1240.0));
        assert!(zone.contains_x(760.0) && zone.contains_x(1240.0));
        assert!(!zone.contains_x(759.9) && !zone.contains_x(1240.1));
        // The leash is data (CA2-Q04), the camera bound is not.
        let mut b = a.clone();
        b.ca2_q04_leash_px = 100;
        let narrow = FreeZone::compute(&input(1000.0, 664.0), &b);
        assert_eq!((narrow.reach_left, narrow.reach_right), (100.0, 100.0));
        assert_eq!((narrow.safe_left, narrow.safe_right), (292.0, 260.0));
    }

    #[test]
    fn the_zone_shrinks_toward_a_view_edge_and_never_goes_negative() {
        let a = knobs();
        // 100 px from the left view edge: 100 - 32 margin - 12 half width.
        let zone = FreeZone::compute(&input(764.0, 664.0), &a);
        assert_eq!(zone.safe_left, 56.0);
        assert_eq!((zone.reach_left, zone.left), (56.0, 708.0));
        // The right side is not affected.
        assert_eq!(zone.reach_right, 240.0);
        // Within margin + half width of the edge there is no zone on that side
        // and the bound goes negative (no commitment fits).
        let tight = FreeZone::compute(&input(690.0, 664.0), &a);
        assert!(tight.safe_left < 0.0);
        assert_eq!((tight.reach_left, tight.left), (0.0, 690.0));
        // A view width other than 640 moves the cap, not the edge rule.
        let wide = ZoneInput {
            view: ViewBounds {
                left: 0.0,
                right: 960.0,
            },
            ..input(480.0, 0.0)
        };
        let z = FreeZone::compute(&wide, &a);
        assert_eq!((z.safe_left, z.safe_right), (436.0, 436.0));
        assert_eq!((z.reach_left, z.reach_right), (240.0, 240.0));
    }

    #[test]
    fn the_zone_shrinks_at_the_layout_ends_with_the_real_camera() {
        let a = knobs();
        let config = cfg(2048);
        // The left end: the camera cannot go left of 0, the leader hugs the wall.
        let mut camera = ScrollCamera::new(config, 20.0, -1);
        for _ in 0..200 {
            camera.step(20.0, -1);
        }
        assert_eq!(camera.left(), 0);
        let zone = FreeZone::compute(&input(20.0, camera.left() as f32), &a);
        assert!(
            zone.safe_left < 0.0,
            "20 px from the end is inside margin + half width"
        );
        assert_eq!(zone.reach_left, 0.0);
        assert_eq!(zone.reach_right, 240.0);
        // 150 px in at the left end: the view cannot scroll back, the zone is
        // the distance to the layout end minus margin and half width.
        let zone = FreeZone::compute(&input(150.0, 0.0), &a);
        assert_eq!(zone.safe_left, 150.0 - 32.0 - HERO_HALF_WIDTH);
        assert_eq!(zone.reach_left, 106.0);
        // The right end.
        let mut camera = ScrollCamera::new(config, 2040.0, 1);
        for _ in 0..200 {
            camera.step(2040.0, 1);
        }
        assert_eq!(camera.left(), 2048 - 640);
        let zone = FreeZone::compute(&input(2040.0, camera.left() as f32), &a);
        assert_eq!(zone.reach_right, 0.0);
        assert_eq!(zone.right, 2040.0);
        assert_eq!(zone.left, 2040.0 - 240.0);
    }

    #[test]
    fn the_vertical_band_and_the_rise_limit() {
        let a = knobs();
        // Layout 400 high, view top 20, view 360 high: world rows 20..=380.
        let r = rows();
        assert_eq!((r.top_y, r.bottom_y), (380.0, 20.0));
        // A leader on the floor (feet at 40): the rise limit (176) is the lower
        // of the two upper limits: 216 against 380 - 48 = 332.
        let zone = FreeZone::compute(&input(1000.0, 664.0), &a);
        assert_eq!(zone.surface_max_y, 216.0);
        assert_eq!(zone.surface_min_y, 28.0);
        assert!(zone.surface_ok(216.0) && !zone.surface_ok(216.5));
        assert!(zone.surface_ok(28.0) && !zone.surface_ok(27.5));
        assert!(zone.contains_spot(1000.0, 40.0));
        assert!(!zone.contains_spot(1000.0, 217.0));
        assert!(!zone.contains_spot(1300.0, 40.0));
        // A leader high up: the view top margin is the lower limit (332).
        let high = ZoneInput {
            leader_foot_y: 300.0,
            ..input(1000.0, 664.0)
        };
        let z = FreeZone::compute(&high, &a);
        assert_eq!(z.surface_max_y, 332.0);
        // The rise limit is a function of the leader's feet each tick: after he
        // drops, a spot that was fine is not (review FU-B).
        let dropped = ZoneInput {
            leader_foot_y: 40.0 - 96.0,
            ..input(1000.0, 664.0)
        };
        let z = FreeZone::compute(&dropped, &a);
        assert_eq!(z.surface_max_y, 120.0);
        assert!(!z.surface_ok(216.0));
    }

    #[test]
    fn the_soft_pull_is_two_utility_per_px_past_the_comfort_radius() {
        let a = knobs();
        assert_eq!(FreeZone::pull_penalty(&a, 0.0), 0);
        assert_eq!(FreeZone::pull_penalty(&a, 160.0), 0);
        assert_eq!(FreeZone::pull_penalty(&a, 161.0), 2);
        assert_eq!(FreeZone::pull_penalty(&a, 240.0), 160);
    }

    // ------------------------------------------------------------ drift ----

    #[test]
    fn the_worst_case_commitments_are_pinned_against_the_move_tables() {
        let t = DonutMoveTuning::SPEC;
        // Pounce: 10 startup + 20 air max (not the nominal 14) + 2 pin + 18
        // recovery + 4 attacker hit-stop.
        assert_eq!(t.pounce_air_max_ticks, 20);
        assert_eq!(Commitment::Pounce.worst_case_ticks(&t), 54);
        assert_eq!(POUNCE_STARTUP, 10);
        assert_eq!(
            (POUNCE_PIN, POUNCE_RECOVERY, POUNCE_HIT_STOP.attacker),
            (2, 18, 4)
        );
        // Swat: 4 + 3 + 7 + 2 attacker hit-stop.
        assert_eq!(Commitment::Swat.worst_case_ticks(&t), 16);
        // Magic Missile: 4 + 1 + 11 and no attacker hit-stop: the bolt hits at
        // range and its 2 ticks of hit-stop freeze the target only.
        let missile = MAGIC_MISSILE.startup + MAGIC_MISSILE.active + MAGIC_MISSILE.recovery;
        assert_eq!(missile, 16);
        assert_eq!(MAGIC_MISSILE.reaction.hit_stop, 2, "the target's freeze");
        assert_eq!(MISSILE_ATTACKER_HIT_STOP, 0);
        assert_eq!(Commitment::Missile.worst_case_ticks(&t), 16);
        // A transit is the edge's own duration.
        assert_eq!(Commitment::Transit { ticks: 37 }.worst_case_ticks(&t), 37);
        // A tuning change moves the sum, not the rule.
        let slower = DonutMoveTuning {
            pounce_air_max_ticks: 24,
            ..t
        };
        assert_eq!(Commitment::Pounce.worst_case_ticks(&slower), 58);
    }

    #[test]
    fn the_drift_check_uses_the_leaders_speed_and_the_spec_landing_distances() {
        let a = knobs();
        let t = DonutMoveTuning::SPEC;
        let check = DriftCheck::new(HeroMotion::CARL, &a);
        assert!((check.leader_px_per_tick() - 185.0 / 60.0).abs() < 1e-4);
        // Carl: 54 ticks of drift 166.5 px; the widest camera bound is 292.
        let pounce = Commitment::Pounce.worst_case_ticks(&t);
        assert!((check.drift_px(pounce) - 166.5).abs() < 0.01);
        let near = |value: f32, expected: f32| (value - expected).abs() < 0.6;
        assert!(near(check.max_end_distance(292.0, pounce), 117.5)); // about 117
        let swat = Commitment::Swat.worst_case_ticks(&t);
        assert!(near(check.max_end_distance(292.0, swat), 234.7)); // about 234
        let missile = Commitment::Missile.worst_case_ticks(&t);
        assert!(near(check.max_end_distance(292.0, missile), 234.7)); // 16 ticks, as the swat
        // Her own speed gives no credit: the check never sees it, and a faster
        // leader (a leader with the agile motion) shortens every allowance.
        let fast = DriftCheck::new(crate::agility::agile_motion(), &a);
        assert!(fast.max_end_distance(292.0, pounce) < check.max_end_distance(292.0, pounce));
    }

    #[test]
    fn the_drift_check_allows_a_commitment_only_when_the_worst_case_stays_in_view() {
        let a = knobs();
        let t = DonutMoveTuning::SPEC;
        let check = DriftCheck::new(HeroMotion::CARL, &a);
        // Mid-view, settled facing right: bounds 292 behind, 260 ahead.
        let zone = FreeZone::compute(&input(1000.0, 664.0), &a);
        let pounce = Commitment::Pounce;
        // 117 px behind him is allowed, 118 is not (117.5 allowed distance).
        assert!(check.allows_commitment(&zone, 1000.0 - 117.0, pounce, &t));
        assert!(!check.allows_commitment(&zone, 1000.0 - 118.0, pounce, &t));
        // Ahead of him the bound is 32 px tighter (260 against 292).
        assert!(check.allows_commitment(&zone, 1000.0 + 85.0, pounce, &t));
        assert!(!check.allows_commitment(&zone, 1000.0 + 86.0, pounce, &t));
        // A swat reaches much farther than a pounce.
        assert!(check.allows_commitment(&zone, 1000.0 - 230.0, Commitment::Swat, &t));
        assert!(!check.allows_commitment(&zone, 1000.0 - 236.0, Commitment::Swat, &t));
        // Next to a view edge nothing is allowed on that side.
        let tight = FreeZone::compute(&input(690.0, 664.0), &a);
        assert!(!check.allows_commitment(&tight, 690.0 - 1.0, Commitment::Swat, &t));
        // Exactly on the leader the tighter side decides.
        let edge = FreeZone::compute(&input(764.0, 664.0), &a);
        assert!(!check.allows(&edge, 764.0, 230));
        assert!(check.allows(&edge, 764.0, 4));
    }

    #[test]
    fn a_nav_transit_uses_the_edges_simulated_duration() {
        // Two platforms: a floor and a 64 px ledge Donut's agile jump reaches.
        let floor = Rect {
            left: 0.0,
            bottom: 0.0,
            width: 800.0,
            height: 40.0,
        };
        let ledge = Rect {
            left: 300.0,
            bottom: 64.0,
            width: 120.0,
            height: 16.0,
        };
        let level = NavLevel {
            solids: vec![NavSolid::new(0, floor), NavSolid::new(1, ledge)],
            hazards: Vec::new(),
        };
        let graph = NavGraph::build(&level, NavConfig::agile()).unwrap();
        let anchors: Vec<_> = graph.anchors().collect();
        let low = anchors
            .iter()
            .copied()
            .find(|&id| graph.anchor_pos(id).is_some_and(|(_, y)| y < 50.0))
            .unwrap();
        let high = anchors
            .iter()
            .copied()
            .find(|&id| graph.anchor_pos(id).is_some_and(|(_, y)| y > 70.0))
            .unwrap();
        let plan = graph
            .edge_plan(low, high, HeroId::Donut)
            .expect("an open climb edge for her");
        assert!(plan.ticks > 0);
        let a = knobs();
        let t = DonutMoveTuning::SPEC;
        let check = DriftCheck::new(HeroMotion::CARL, &a);
        let commitment = Commitment::Transit { ticks: plan.ticks };
        assert_eq!(commitment.worst_case_ticks(&t), plan.ticks);
        // The drift is the edge's ticks at the leader's speed; the end point is
        // the edge's target anchor.
        let drift = check.drift_px(plan.ticks);
        assert!((drift - f32::from(plan.ticks) * 185.0 / 60.0).abs() < 0.01);
        let zone = FreeZone::compute(&input(330.0, 100.0), &a);
        let (end_x, _) = graph.anchor_pos(high).unwrap();
        let allowed = (end_x - 330.0).abs() + drift + 8.0 <= zone.safe_toward(end_x);
        assert_eq!(
            check.allows_commitment(&zone, end_x, commitment, &t),
            allowed
        );
    }

    #[test]
    fn the_bound_pad_covers_the_pan_lag_of_every_partner_motion() {
        // CA2-Q25: pad >= ceil(partner run px per tick / PAN_RATE) + 4, so a
        // faster motion fails here instead of silently breaking M8.
        let a = knobs();
        for (name, motion) in motions() {
            let px_per_tick = motion.run_speed * TICK_SECONDS;
            let floor = (px_per_tick / PAN_RATE).ceil() as u16 + 4;
            assert!(
                a.ca2_q25_bound_pad_px >= floor,
                "{name}: pad {} < floor {floor}",
                a.ca2_q25_bound_pad_px
            );
        }
        // The validated run speed range stops below 240 px/s (`agility.rs`); the
        // floor reaches 41 only above about 237.6 px/s, where the scroll camera's
        // own 4 px per tick cap is the real limit (review N-1).
        // DM-A's agile run (207 px/s): the floor is 36, the adopted pad 40.
        let agile = crate::agility::agile_motion();
        assert_eq!(
            ((agile.run_speed * TICK_SECONDS) / PAN_RATE).ceil() as u16 + 4,
            36
        );
        assert_eq!(a.ca2_q25_bound_pad_px, 40);
        assert_eq!(RUN_SPEED, 185.0);
    }

    // ----------------------------------------------------------- claims ----

    fn leader(x: f32, facing: i8) -> HeroView {
        let mut h = fixtures::hero(HeroId::Carl, x, Control::Human(0));
        h.facing = facing;
        h
    }

    fn enemy(index: u32, x: f32, state: TargetState) -> TargetView {
        TargetView {
            id: StableEntityId::goblin(index),
            x,
            foot_y: 0.0,
            hp: 2,
            state,
            guarded: false,
            in_view: true,
            carl_hit: false,
            carl_hit_age: None,
        }
    }

    fn view_with(leader: HeroView, targets: Vec<TargetView>) -> CompanionView {
        let mut v = fixtures::view(
            leader,
            fixtures::hero(HeroId::Donut, 0.0, Control::Companion),
        );
        v.profile_set = ProfileSetId::Cf02Free;
        v.targets = targets;
        v
    }

    fn claims(leader: HeroView, targets: Vec<TargetView>) -> LeaderClaims {
        let set = free_set();
        let view = view_with(leader, targets);
        let l = view.heroes[0];
        LeaderClaims::compute(&view, &set, &l)
    }

    #[test]
    fn the_front_claim_is_the_nearest_standing_enemy_he_faces_within_64_px() {
        let c = claims(
            leader(400.0, 1),
            vec![
                enemy(0, 320.0, TargetState::Standing), // behind him
                enemy(1, 470.0, TargetState::Standing), // 70 px in front: outside
                enemy(2, 450.0, TargetState::Standing), // 50 px in front
                enemy(3, 440.0, TargetState::Standing), // 40 px in front: nearest
            ],
        );
        assert_eq!(c.front(), Some(StableEntityId::goblin(3)));
        assert!(c.contains(StableEntityId::goblin(3)));
        // Only the front claim: not the others in front, not the one behind.
        assert_eq!(c.ids(), &[StableEntityId::goblin(3)]);
        assert!(!c.contains(StableEntityId::goblin(2)));
        // 64 px exactly is inside, 65 is not.
        let edge = claims(
            leader(400.0, 1),
            vec![enemy(0, 464.0, TargetState::Standing)],
        );
        assert_eq!(edge.front(), Some(StableEntityId::goblin(0)));
        let out = claims(
            leader(400.0, 1),
            vec![enemy(0, 465.0, TargetState::Standing)],
        );
        assert!(out.is_empty() && out.front().is_none());
        // Facing left, the same enemy is behind him and not claimed.
        let behind = claims(
            leader(400.0, -1),
            vec![enemy(0, 440.0, TargetState::Standing)],
        );
        assert!(behind.is_empty());
        // An enemy exactly on him counts as in front.
        let on = claims(
            leader(400.0, -1),
            vec![enemy(0, 400.0, TargetState::Standing)],
        );
        assert_eq!(on.front(), Some(StableEntityId::goblin(0)));
    }

    #[test]
    fn the_front_claim_is_a_standing_enemy_on_his_level() {
        let lying = TargetState::Lying {
            down_ticks_left: 40,
            hold_used: false,
        };
        // A lying or staggered enemy that is not down from his hit is not a front
        // claim (N-5: the downed-by-his-hit case is pinned in the B1 tests).
        for state in [
            lying,
            TargetState::Staggered { ticks: 5 },
            TargetState::Airborne,
        ] {
            let c = claims(leader(400.0, 1), vec![enemy(0, 440.0, state)]);
            assert!(c.front().is_none(), "{state:?}");
        }
        // Another level (feet more than 24 px apart) is not his enemy.
        let mut up = enemy(0, 440.0, TargetState::Standing);
        up.foot_y = 25.0;
        assert!(claims(leader(400.0, 1), vec![up]).is_empty());
        up.foot_y = 24.0;
        assert!(!claims(leader(400.0, 1), vec![up]).is_empty());
        // A defeated leader claims nothing.
        let mut dead = leader(400.0, 1);
        dead.defeated = true;
        assert!(claims(dead, vec![enemy(0, 440.0, TargetState::Standing)]).is_empty());
        // A dead target is never claimed.
        let mut gone = enemy(0, 440.0, TargetState::Standing);
        gone.hp = 0;
        assert!(claims(leader(400.0, 1), vec![gone]).is_empty());
    }

    #[test]
    fn engagement_claims_a_recent_hit_on_either_side_and_a_running_move_in_reach() {
        // A12: hit 10 ticks ago, 200 px behind him: still his.
        let mut hit = enemy(0, 200.0, TargetState::Standing);
        hit.carl_hit = true;
        hit.carl_hit_age = Some(10);
        let c = claims(leader(400.0, 1), vec![hit]);
        assert!(c.contains(StableEntityId::goblin(0)));
        assert!(
            c.front().is_none(),
            "it is claimed by engagement, not in front"
        );
        // Age 31 is inside the window, 32 is not.
        hit.carl_hit_age = Some(31);
        assert!(claims(leader(400.0, 1), vec![hit]).contains(StableEntityId::goblin(0)));
        hit.carl_hit_age = Some(32);
        assert!(claims(leader(400.0, 1), vec![hit]).is_empty());
        // A08: a move running and the enemy within 40 px on his facing side.
        let mut l = leader(400.0, 1);
        l.active_move = Some(crate::companion::view::ActiveMove {
            id: crate::combat::MoveId::Jab,
            phase: crate::companion::view::MovePhase::Active,
            ticks_left: 6,
            ai_started: false,
        });
        let within = claims(l, vec![enemy(0, 440.0, TargetState::Standing)]);
        assert!(within.contains(StableEntityId::goblin(0)));
        // 41 px: out of the A08 reach (it is the front claim only through the
        // 64 px rule, so it is still in the set, by that rule).
        let c = claims(l, vec![enemy(0, 441.0, TargetState::Standing)]);
        assert!(c.contains(StableEntityId::goblin(0)) && c.front().is_some());
        // Behind him with a move running: A11 refuses it.
        let c = claims(l, vec![enemy(0, 380.0, TargetState::Standing)]);
        assert!(c.is_empty());
        // Down from his hit within 40 px (no move running): A08's knockdown.
        let mut down = enemy(
            0,
            430.0,
            TargetState::Lying {
                down_ticks_left: 50,
                hold_used: false,
            },
        );
        down.carl_hit = true;
        assert!(claims(leader(400.0, 1), vec![down]).contains(StableEntityId::goblin(0)));
        // 41 px: out of the A08 reach, but in front of him and down from his hit:
        // the front claim holds it (AD-B1) until the 64 px radius.
        down.x = 441.0;
        let c = claims(leader(400.0, 1), vec![down]);
        assert_eq!(c.front(), Some(StableEntityId::goblin(0)));
        down.x = 465.0;
        assert!(claims(leader(400.0, 1), vec![down]).is_empty());
    }

    #[test]
    fn a_claim_expires_when_he_walks_away_and_his_hit_window_passes() {
        // Owner decision 1: Carl wounds a goblin, then walks more than 96 px
        // away. While the A12 window runs the goblin stays his; after it, with
        // him beyond the claim radius, it is unclaimed and open to her.
        let mut g = enemy(0, 500.0, TargetState::Standing);
        g.carl_hit = true;
        let state = |leader_x: f32, age: u16| {
            let mut t = g;
            t.carl_hit_age = Some(age);
            claims(leader(leader_x, -1), vec![t])
        };
        // He stands beside it, then turns and walks left: 30 ticks after the hit
        // he is 100 px away and it is still claimed (A12, either side).
        assert!(state(400.0, 30).contains(StableEntityId::goblin(0)));
        // 32 ticks after the hit it is unclaimed: he faces away and is > 64 px.
        assert!(state(400.0, 32).is_empty());
        // Close to it again and facing it, it is claimed by the front rule.
        let near = claims(
            leader(460.0, 1),
            vec![{
                let mut t = g;
                t.carl_hit_age = Some(80);
                t
            }],
        );
        assert_eq!(near.front(), Some(StableEntityId::goblin(0)));
        // Walking away facing it (an A11 facing leader 100 px off) is not a claim.
        let away = claims(
            leader(400.0, 1),
            vec![{
                let mut t = g;
                t.carl_hit_age = Some(80);
                t
            }],
        );
        assert!(away.is_empty());
    }

    #[test]
    fn claims_are_sorted_deduplicated_and_deterministic() {
        // The front claim that is also engaged appears once; ids stay ordered.
        let mut hit = enemy(5, 430.0, TargetState::Standing);
        hit.carl_hit = true;
        hit.carl_hit_age = Some(3);
        let mut other = enemy(1, 150.0, TargetState::Standing);
        other.carl_hit = true;
        other.carl_hit_age = Some(1);
        let targets = vec![other, enemy(2, 600.0, TargetState::Standing), hit];
        let a = claims(leader(400.0, 1), targets.clone());
        let b = claims(leader(400.0, 1), targets);
        assert_eq!(a, b);
        assert_eq!(
            a.ids(),
            &[StableEntityId::goblin(1), StableEntityId::goblin(5)]
        );
        assert_eq!(a.len(), 2);
        // Ties on distance keep the lowest id (the view order).
        let tie = claims(
            leader(400.0, 1),
            vec![
                enemy(7, 440.0, TargetState::Standing),
                enemy(9, 440.0, TargetState::Standing),
            ],
        );
        assert_eq!(tie.front(), Some(StableEntityId::goblin(7)));
        // A llama is a target like any other kind.
        let mut llama = enemy(0, 440.0, TargetState::Standing);
        llama.id.kind = EntityKind::Llama;
        assert_eq!(
            claims(leader(400.0, 1), vec![llama]).front(),
            Some(llama.id)
        );
    }

    // --------------------------------------------- the real scene accessor ----

    #[test]
    fn the_pan_remaining_accessor_is_read_only_and_matches_the_cameras_step() {
        let kit = EnvKit::load(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
            "assets/levels/floor01/corridor_abc_sample.json",
        )
        .unwrap();
        let mut sim = SceneSim::new(&kit, 360).unwrap();
        // The camera starts framed on the hero: nothing left to pan.
        assert_eq!(sim.camera_pan_remaining(), 0);
        // The hero jumps ahead (a knockback, a launch): the camera wants to move.
        sim.world.hero.x += 800.0;
        let hash = sim.state_hash();
        let camera = sim.camera;
        let remaining = sim.camera_pan_remaining();
        assert!(remaining > 0);
        assert_eq!(sim.state_hash(), hash, "the accessor never writes");
        assert_eq!(sim.camera, camera);
        // The real step reports the same number after it has moved the camera
        // by its cap: the accessor is the step's `remaining` without the move.
        sim.step(RawInput::default());
        assert_eq!(
            sim.camera_pan_remaining(),
            remaining - MAX_SPEED_PX,
            "one step of the 4 px cap"
        );
        // Once the camera has settled on him, the zone from the live scene's own
        // numbers lies inside the view.
        for _ in 0..400 {
            if sim.camera_pan_remaining() == 0 {
                break;
            }
            sim.step(RawInput::default());
        }
        assert_eq!(sim.camera_pan_remaining(), 0);
        let cfg = sim.camera.config();
        let rows = ViewRows::of_camera(&cfg);
        let left = sim.camera.left() as f32;
        let leader_view = HeroView {
            x: sim.world.hero.x,
            foot_y: sim.world.hero_foot_y(),
            ..fixtures::hero(HeroId::Carl, 0.0, Control::Human(0))
        };
        let zone = FreeZone::compute(
            &ZoneInput::new(
                &leader_view,
                ViewBounds {
                    left,
                    right: left + cfg.view_w as f32,
                },
                rows,
            ),
            &knobs(),
        );
        assert!(zone.left >= left && zone.right <= left + cfg.view_w as f32);
        assert!(zone.surface_ok(leader_view.foot_y));
    }

    // ===================================================================
    // CA2-B1: the goal selector, the brief and the follow-ups of the CA2-A
    // review (N-3, N-5, N-6).
    // ===================================================================

    use crate::companion::director::{CompanionDirector, Mode};
    use crate::companion::view::{ActiveMove, FollowInput, MovePhase, ThreatView};

    /// The simulation view's rows for the goal tests: the floor (feet at 0) is
    /// inside the band.
    fn goal_rows() -> ViewRows {
        ViewRows {
            top_y: 360.0,
            bottom_y: -40.0,
        }
    }

    /// A flat world: one floor surface (already inset), plus optionally a
    /// second surface reachable by a transit of `climb` ticks.
    struct FlatWorld {
        surfaces: Vec<Surface>,
        climb: Option<u16>,
    }

    impl FlatWorld {
        fn floor() -> Self {
            Self {
                surfaces: vec![Surface {
                    key: 1,
                    left: 30.0,
                    right: 610.0,
                    y: 0.0,
                    risk: 0,
                }],
                climb: None,
            }
        }
    }

    impl SpotWorld for FlatWorld {
        fn surfaces(&self, out: &mut Vec<Surface>) {
            out.extend(self.surfaces.iter().copied());
        }

        fn route_ticks(&self, partner: &HeroView, _x: f32, surface: &Surface) -> Option<u16> {
            // Her surface: the one at her feet or, in the air, the highest one
            // below her (review F-1), like `NavSpots`.
            let mut here: Option<&Surface> = None;
            for s in &self.surfaces {
                if s.y <= partner.foot_y + 4.0 && here.is_none_or(|h| s.y > h.y) {
                    here = Some(s);
                }
            }
            match here {
                Some(h) if (surface.y - h.y).abs() <= 4.0 => Some(0),
                _ => self.climb,
            }
        }
    }

    /// One scene: Carl is the human leader at x 320 facing right in a view
    /// 0..640, Donut the companion at x 240, pre-biscuit, the `cf02_free` set.
    struct Rig {
        st: AutonomyState,
        set: ProfileSet,
        world: FlatWorld,
        v: CompanionView,
        landed: bool,
        pan: i32,
        leader_motion: HeroMotion,
        partner_motion: HeroMotion,
    }

    impl Rig {
        fn new() -> Self {
            let mut v = view_with(leader(320.0, 1), Vec::new());
            v.heroes[1] = fixtures::hero(HeroId::Donut, 240.0, Control::Companion);
            v.tick = 100;
            Self {
                st: AutonomyState::new(),
                set: free_set(),
                world: FlatWorld::floor(),
                v,
                landed: false,
                pan: 0,
                leader_motion: HeroMotion::CARL,
                partner_motion: crate::agility::agile_motion(),
            }
        }

        fn donut(&mut self) -> &mut HeroView {
            &mut self.v.heroes[1]
        }

        fn carl(&mut self) -> &mut HeroView {
            &mut self.v.heroes[0]
        }

        fn goblin(&mut self, index: u32, x: f32, state: TargetState) {
            let t = enemy(index, x, state);
            self.v.targets.retain(|e| e.id != t.id);
            self.v.targets.push(t);
            self.v.targets.sort_by_key(|e| e.id);
        }

        fn clear_targets(&mut self) {
            self.v.targets.clear();
        }

        /// Step the layer on the current view, then advance the tick.
        fn step(&mut self) -> Option<AutonomyBrief> {
            let a = self.set.autonomy.clone();
            let leader_view = self.v.heroes[0];
            let zone =
                FreeZone::compute(&ZoneInput::new(&leader_view, self.v.view, goal_rows()), &a);
            let drift = DriftCheck::new(self.leader_motion, &a);
            let tuning = DonutMoveTuning::SPEC;
            let brief = self.st.step(&AutonomyInput {
                view: &self.v,
                set: &self.set,
                partner: HeroId::Donut,
                zone: &zone,
                drift: &drift,
                partner_motion: self.partner_motion,
                tuning: &tuning,
                world: &self.world,
                pan_remaining: self.pan,
                partner_hit_landed: self.landed,
            });
            self.landed = false;
            self.v.tick += 1;
            brief
        }

        fn run(&mut self, ticks: u32) -> Option<AutonomyBrief> {
            let mut last = None;
            for _ in 0..ticks {
                last = self.step();
            }
            last
        }
    }

    fn standing() -> TargetState {
        TargetState::Standing
    }

    // ------------------------------------------------- review follow-ups ----

    /// N-5 / FU-D (adopted default AD-B1): a lying or staggered enemy counts as
    /// the front claim only when it is down from the leader's own hit. A blanket
    /// rule would forbid her swat-then-pounce on a goblin she staggered in front
    /// of him.
    #[test]
    fn the_front_claim_counts_a_downed_enemy_only_when_the_leaders_hit_downed_it() {
        let lying = TargetState::Lying {
            down_ticks_left: 40,
            hold_used: false,
        };
        let staggered = TargetState::Staggered { ticks: 5 };
        for state in [lying, staggered] {
            // Down from her swat (not his hit), 41 to 64 px in front of him:
            // not his, so her pounce may land.
            let hers = claims(leader(400.0, 1), vec![enemy(0, 450.0, state)]);
            assert!(hers.is_empty(), "{state:?} not from his hit");
            // Down from his hit, in front, beyond the 40 px A08 reach: his.
            let mut his = enemy(0, 450.0, state);
            his.carl_hit = true;
            let c = claims(leader(400.0, 1), vec![his]);
            assert_eq!(c.front(), Some(StableEntityId::goblin(0)), "{state:?}");
            // Behind him it is not a front claim (A11), even from his hit.
            his.x = 350.0;
            assert!(claims(leader(400.0, 1), vec![his]).is_empty());
            // Beyond the claim radius it is released.
            his.x = 470.0;
            assert!(claims(leader(400.0, 1), vec![his]).is_empty());
        }
        // A goblin in the air is never a front claim.
        let mut air = enemy(0, 440.0, TargetState::Airborne);
        air.carl_hit = true;
        assert!(claims(leader(400.0, 1), vec![air]).is_empty());
        // Her own swat-then-pounce is therefore possible on a goblin she
        // staggered in front of him: it is not in the set.
        let swatted = claims(
            leader(400.0, 1),
            vec![enemy(0, 460.0, TargetState::Staggered { ticks: 8 })],
        );
        assert!(!swatted.contains(StableEntityId::goblin(0)));
    }

    /// N-6: the pounce's start point is checked after the startup, not only the
    /// end point after the whole commitment. A pounce toward the leader from far
    /// out passes the end-point rule and fails the in-flight rule.
    #[test]
    fn the_drift_check_also_tests_the_pounce_start_after_its_startup() {
        let a = knobs();
        let t = DonutMoveTuning::SPEC;
        let check = DriftCheck::new(HeroMotion::CARL, &a);
        let zone = FreeZone::compute(&input(1000.0, 664.0), &a);
        // Behind him the camera bound is 292: the startup (10 ticks, 30.8 px)
        // allows a start up to 292 - 30.8 - 8 = 253.2 px out.
        let end = 1000.0 - 40.0; // lands 40 px from him
        assert!(check.allows(&zone, end, 54));
        assert!(check.allows_pounce(&zone, 1000.0 - 250.0, end, &t));
        assert!(!check.allows(&zone, 1000.0 - 256.0, 10));
        assert!(
            !check.allows_pounce(&zone, 1000.0 - 256.0, end, &t),
            "the end point alone would have allowed it"
        );
        // Away from him the end point decides (it is the farther one).
        assert!(check.allows_pounce(&zone, 1000.0 - 60.0, 1000.0 - 110.0, &t));
        assert!(!check.allows_pounce(&zone, 1000.0 - 60.0, 1000.0 - 120.0, &t));
        // With the shipped geometry the margin absorbs the in-flight drift: any
        // start the zone can hand out (the 240 px leash) passes the startup test
        // on the trailing side, so the check never costs her a real start.
        assert!(check.allows(&zone, 1000.0 - 240.0, 10));
    }

    /// N-3: the zone proof assumes the partner runs at least as fast as the
    /// leader. With a partner slower than the leader the trailing partner loses
    /// ground every tick and leaves the zone's guarantee; at equal speed it holds.
    /// A future AI leader-side brawler (185 px/s) partnering a faster human
    /// leader breaks the proof (note for that work order).
    #[test]
    fn the_zone_proof_needs_a_partner_at_least_as_fast_as_the_leader() {
        let a = knobs();
        let config = cfg(4096);
        let leader_v = crate::agility::agile_motion().run_speed * TICK_SECONDS; // 3.45
        let lose = |partner_v: f32| -> f32 {
            // The leader runs away; the trailing partner starts at the leash,
            // runs with him at `partner_v`, and we report the worst margin to
            // the trailing view edge over 300 ticks.
            let mut camera = ScrollCamera::new(config, 2000.0, 1);
            let mut x = 2000.0;
            let zone0 = FreeZone::compute(&input(x, camera.left() as f32), &a);
            let mut partner = x - zone0.reach_left;
            let mut worst = f32::MAX;
            for _ in 0..300 {
                x += leader_v;
                partner += partner_v;
                camera.step(x, 1);
                worst = worst.min(partner - HERO_HALF_WIDTH - camera.left() as f32);
            }
            worst
        };
        // Equal speed: she keeps the margin the zone promised (32 and more).
        assert!(lose(leader_v) >= 32.0 - 0.5, "{}", lose(leader_v));
        // Faster: no worse.
        assert!(lose(leader_v + 0.3) >= lose(leader_v));
        // Slower (a 185 px/s partner behind a 207 px/s leader): she is lost
        // inside 300 ticks, so the zone proof does not cover that pairing.
        let slower = 185.0 * TICK_SECONDS;
        assert!(lose(slower) < 0.0, "{}", lose(slower));
    }

    // -------------------------------------------------------- the goals ----

    #[test]
    fn with_the_layer_off_or_no_role_nothing_runs_and_nothing_changes() {
        let mut rig = Rig::new();
        rig.goblin(0, 230.0, standing());
        rig.set = set_of_id(ProfileSetId::Cf02Assist);
        let before = rig.st.state_hash();
        assert!(rig.step().is_none());
        assert_eq!(rig.st.state_hash(), before);
        // An enabled layer for a hero with no role (AI Carl in v1) is also off.
        let mut rig = Rig::new();
        let mut set = free_set();
        set.donut.autonomy_role = crate::companion::profile::AutonomyRole::None;
        rig.set = set;
        assert!(rig.step().is_none());
        // A hero who is not companion-driven forgets its plan.
        let mut rig = Rig::new();
        rig.step().unwrap();
        rig.donut().control = Control::Human(0);
        assert!(rig.step().is_none());
        assert!(rig.st.goal_since() == 0 && rig.st.target().is_none());
    }

    fn set_of_id(id: ProfileSetId) -> ProfileSet {
        ProfileFile::pinned().unwrap().set(id).clone()
    }

    #[test]
    fn an_empty_room_holds_inside_the_comfort_radius_and_escorts_beyond_it() {
        let mut rig = Rig::new();
        let b = rig.step().unwrap();
        // Comfortable and in no hurry: she holds, which is Idle (AD-B4).
        assert_eq!(b.goal, Goal::Idle);
        assert_eq!(b.changed, Some(GoalReason::Start));
        assert!(b.spot.is_none(), "80 px from him: hold where she stands");
        assert_eq!(b.director.allowed, Vec::new());
        assert!(b.director.interrupts, "interrupts only");
        assert!(b.in_zone);
        // Past the comfort radius (160 px) the escort walks back to 72..120.
        let mut rig = Rig::new();
        rig.donut().x = 500.0;
        let b = rig.step().unwrap();
        assert_eq!(b.goal, Goal::Escort);
        let spot = b.spot.expect("an escort spot");
        let from_leader = (spot.x - 320.0).abs();
        assert!(spot.x > 320.0, "she stays on her own side");
        assert!((from_leader - 96.0).abs() <= 8.0, "{from_leader}");
        assert_eq!(spot.purpose, SpotPurpose::Escort);
    }

    #[test]
    fn she_hunts_the_goblin_carl_is_not_working() {
        let mut rig = Rig::new();
        rig.goblin(0, 360.0, standing()); // 40 px in front of him: his
        rig.goblin(1, 200.0, standing()); // behind him, 120 px: hers
        let b = rig.step().unwrap();
        assert_eq!(b.goal, Goal::Hunt);
        assert_eq!(b.target, Some(StableEntityId::goblin(1)));
        assert_eq!(b.director.allowed, vec![StableEntityId::goblin(1)]);
        // The pre-unlock pounce band (30 to 80), aiming at 56.
        assert_eq!(
            (b.director.press_min_px, b.director.press_max_px),
            (30.0, 80.0)
        );
        let spot = b.spot.expect("an attack spot");
        assert_eq!(spot.purpose, SpotPurpose::Attack);
        assert!((spot.x - 200.0 - 56.0).abs() <= 8.0, "{}", spot.x);
        assert_eq!(b.face_x, Some(200.0));
        assert!(!b.recoil);
        // The claimed goblin is never allowed.
        assert!(!b.director.allowed.contains(&StableEntityId::goblin(0)));
        let stats = rig.st.stats();
        assert!(stats.hunt_claim_excluded >= 1 && stats.hunt_evaluations >= 2);
    }

    /// Review B-2: one goblin that Carl is working gives no Hunt goal, and the
    /// director presses nothing on it even with it inside her pounce band.
    #[test]
    fn one_claimed_goblin_gives_no_hunt_and_no_press() {
        let mut rig = Rig::new();
        rig.donut().x = 300.0;
        rig.goblin(0, 360.0, standing());
        let b = rig.step().unwrap();
        assert_ne!(b.goal, Goal::Hunt);
        assert!(b.director.allowed.is_empty());
        // The unbriefed director would pounce it (this guards the test).
        let set = free_set();
        let v = rig.v.clone();
        let mut free = CompanionDirector::new(HeroId::Donut);
        let (raw, d) = free.step_briefed(&v, &set, None);
        assert!(raw.physical && d.pressed, "{d:?}");
        // Briefed, it presses nothing, on any of 60 ticks.
        let mut briefed = CompanionDirector::new(HeroId::Donut);
        for tick in 0..60 {
            let mut v = rig.v.clone();
            v.tick += tick;
            let (raw, d) = briefed.step_briefed(&v, &set, Some(&b.director));
            assert!(!raw.attack && !raw.physical && !d.pressed, "tick {tick}");
            assert_eq!(d.target, None);
        }
    }

    #[test]
    fn a_hunt_candidate_outside_the_zone_or_failing_the_drift_check_is_no_candidate() {
        // 170 px behind Carl: a pounce there cannot be commit-safe (the landing
        // would be farther than the 101 px the drift allows).
        let mut rig = Rig::new();
        rig.goblin(1, 150.0, standing());
        let b = rig.step().unwrap();
        assert_ne!(b.goal, Goal::Hunt);
        assert_eq!(rig.st.stats().hunt_drift_rejected, 1);
        // The same goblin 90 px behind him is fine.
        let mut rig = Rig::new();
        rig.goblin(1, 230.0, standing());
        assert_eq!(rig.step().unwrap().goal, Goal::Hunt);
        // A goblin on another level, with no route, is no candidate.
        let mut rig = Rig::new();
        let mut high = enemy(1, 230.0, standing());
        high.foot_y = 96.0;
        rig.v.targets.push(high);
        assert_ne!(rig.step().unwrap().goal, Goal::Hunt);
        // A llama is no target before the unlock (CA2-Q31) and one after.
        let mut rig = Rig::new();
        let mut llama = enemy(0, 230.0, standing());
        llama.id.kind = EntityKind::Llama;
        rig.v.targets.push(llama);
        assert_ne!(rig.step().unwrap().goal, Goal::Hunt);
        let mut rig = Rig::new();
        rig.v.targets.push(llama);
        rig.donut().mana = Some(crate::companion::view::ManaView {
            mana: 6,
            max: 8,
            unlocked: true,
            ai_can_cast: true,
            projectile_speed_px_per_tick: 6,
            bolt_lead_px: 35,
        });
        let b = rig.step().unwrap();
        assert_eq!(b.goal, Goal::Hunt);
        // The casting band, aiming at 112.
        assert_eq!(
            (b.director.press_min_px, b.director.press_max_px),
            (72.0, 160.0)
        );
    }

    #[test]
    fn the_hunt_score_prefers_a_downed_enemy_and_a_back_threat_and_keeps_its_target() {
        // A lying goblin (base 300) beats a standing one (200) on equal terms.
        let mut rig = Rig::new();
        rig.donut().x = 250.0;
        rig.goblin(1, 200.0, standing());
        rig.goblin(
            2,
            200.0,
            TargetState::Lying {
                down_ticks_left: 60,
                hold_used: false,
            },
        );
        rig.step();
        assert_eq!(rig.st.target(), Some(StableEntityId::goblin(2)));
        // Two standing goblins behind him that tie (score 235 each): the lower id.
        let mut rig = Rig::new();
        rig.donut().x = 250.0;
        rig.goblin(1, 200.0, standing());
        rig.goblin(2, 215.0, standing());
        rig.step();
        assert_eq!(rig.st.target(), Some(StableEntityId::goblin(1)));
        // Goblin 2 closes on his back: the 300 bonus beats the 150 stickiness of
        // goblin 1, at the next decision, with no wait for the minimum goal time
        // (that holds the goal, not the target).
        rig.run(2);
        rig.goblin(2, 220.0, standing());
        let b = rig.run(6).unwrap();
        assert_eq!(b.goal, Goal::Hunt);
        assert_eq!(b.target, Some(StableEntityId::goblin(2)));
        // And with nothing closing, the current target keeps against an equal one.
        let mut rig = Rig::new();
        rig.donut().x = 250.0;
        rig.goblin(1, 215.0, standing());
        rig.goblin(2, 200.0, standing());
        rig.step();
        // Goblin 2 is farther from him (isolation 60 against 52) and from her
        // (travel 25 against 17): equal scores, so goblin 1 by id, then it sticks.
        assert_eq!(rig.st.target(), Some(StableEntityId::goblin(1)));
        rig.goblin(1, 214.0, standing());
        rig.run(12);
        assert_eq!(rig.st.target(), Some(StableEntityId::goblin(1)));
    }

    #[test]
    fn support_leader_takes_an_unclaimed_enemy_when_he_is_outnumbered() {
        let mut rig = Rig::new();
        rig.donut().x = 250.0;
        rig.goblin(0, 360.0, standing()); // front claim, 40 px
        rig.goblin(1, 290.0, standing()); // behind him 30 px, unclaimed
        let b = rig.step().unwrap();
        assert_eq!(b.goal, Goal::SupportLeader);
        assert_eq!(b.target, Some(StableEntityId::goblin(1)));
        assert_eq!(b.director.allowed, vec![StableEntityId::goblin(1)]);
        // Two enemies near him and both in the claim set: not eligible on this
        // trigger (and not a Hunt: they are claimed).
        let mut rig = Rig::new();
        rig.donut().x = 250.0;
        let mut a = enemy(0, 360.0, standing());
        a.carl_hit = true;
        a.carl_hit_age = Some(3);
        let mut b2 = enemy(1, 290.0, standing());
        b2.carl_hit = true;
        b2.carl_hit_age = Some(3);
        rig.v.targets = vec![a, b2];
        let b = rig.step().unwrap();
        assert!(
            !matches!(b.goal, Goal::SupportLeader | Goal::Hunt),
            "{:?}",
            b.goal
        );
    }

    #[test]
    fn a_low_leader_gets_support_and_may_have_a_claimed_enemy_only_when_none_is_free() {
        let mut rig = Rig::new();
        rig.carl().health = 2;
        rig.donut().x = 300.0;
        rig.goblin(0, 360.0, standing()); // the only enemy, claimed (front)
        let b = rig.step().unwrap();
        assert_eq!(b.goal, Goal::SupportLeader);
        assert_eq!(b.target, Some(StableEntityId::goblin(0)));
        // With an unclaimed enemy close to him she prefers it.
        let mut rig = Rig::new();
        rig.carl().health = 2;
        rig.donut().x = 270.0;
        rig.goblin(0, 360.0, standing());
        rig.goblin(1, 290.0, standing());
        let b = rig.step().unwrap();
        assert_eq!(b.target, Some(StableEntityId::goblin(1)));
        // A healthy leader with one claimed enemy: no support.
        let mut rig = Rig::new();
        rig.donut().x = 300.0;
        rig.goblin(0, 360.0, standing());
        assert_ne!(rig.step().unwrap().goal, Goal::SupportLeader);
    }

    #[test]
    fn the_goal_utilities_are_the_q18_table_and_ties_go_to_escort() {
        let set = free_set();
        let g = &set.autonomy.roles.skirmisher.goals;
        assert_eq!(
            [
                g.retreat.utility,
                g.support_leader.utility,
                g.hunt.utility,
                g.escort.utility,
                g.show_off.utility,
                g.perch.utility,
                g.idle.utility
            ],
            [1800, 1400, 1200, 800, 700, 600, 100]
        );
        // While he travels Escort is 800 + 400 = 1200: a tie with a new Hunt,
        // and the tie goes to Escort (AD-B2: she comes along).
        // (FA-Q28 off: with it on the bonus is 200 while a hunt candidate is near.)
        let mut rig = Rig::new();
        rig.set.fix_all.fa_q28_hunt_escort_bonus = false;
        rig.pan = 40;
        rig.goblin(1, 230.0, standing());
        let b = rig.step().unwrap();
        assert_eq!(b.goal, Goal::Escort, "escort 1200 ties a new hunt 1200");
        // Perch is not selected in B1 (CA2-C), whatever the room.
        let mut rig = Rig::new();
        for _ in 0..40 {
            assert_ne!(rig.step().unwrap().goal, Goal::Perch);
        }
    }

    #[test]
    fn under_a_boss_only_the_flagged_goals_are_eligible() {
        let mut rig = Rig::new();
        rig.goblin(1, 230.0, standing());
        rig.v.boss = Some(crate::boss::BossThreat::Pound {
            x: 500.0,
            facing: -1,
            target: None,
        });
        let b = rig.step().unwrap();
        assert!(
            matches!(b.goal, Goal::Retreat | Goal::Escort | Goal::Idle),
            "{:?}",
            b.goal
        );
    }

    // ------------------------------------------------- hit and run, recoil ----

    fn pounce_in_flight() -> ActiveMove {
        ActiveMove {
            id: MoveId::Pounce,
            phase: MovePhase::Active,
            ticks_left: 20,
            ai_started: true,
        }
    }

    #[test]
    fn after_a_pounce_the_director_has_no_walk_and_no_press_for_the_recoil() {
        let mut rig = Rig::new();
        rig.goblin(
            1,
            230.0,
            TargetState::Lying {
                down_ticks_left: 90,
                hold_used: false,
            },
        );
        rig.donut().x = 286.0;
        let b = rig.step().unwrap();
        assert_eq!(b.goal, Goal::Hunt);
        // She pounces for 10 ticks, then the move ends.
        rig.donut().active_move = Some(pounce_in_flight());
        rig.run(10);
        rig.donut().active_move = None;
        let b = rig.step().unwrap();
        assert!(b.recoil, "the recoil starts when the move ends");
        let end = rig.v.tick - 1;
        assert_eq!(rig.st.recoil_until(), end + 30);
        let set = free_set();
        let mut dir = CompanionDirector::new(HeroId::Donut);
        rig.v.follow[1] = FollowInput {
            move_x: 0.5,
            jump: false,
        };
        // 30 ticks: the brief allows no target, the director writes no move_x of
        // its own (the follow layer's 0.5 passes through untouched) and presses
        // nothing, although the goblin lies inside her pounce band.
        for tick in 0..29 {
            let b = rig.step().unwrap();
            assert!(b.recoil, "tick {tick}");
            assert!(b.director.allowed.is_empty());
            let (raw, d) = dir.step_briefed(&rig.v, &set, Some(&b.director));
            assert_eq!(raw.move_x, 0.5, "tick {tick}");
            assert!(!d.pressed && d.target.is_none());
            // The recoil spot is the 96 px spring-back, on her side of the target.
            let spot = b.spot.expect("a recoil spot");
            assert_eq!(spot.purpose, SpotPurpose::Recoil);
            assert!((spot.x - 230.0 - 96.0).abs() <= 8.0, "{}", spot.x);
        }
        // After the window the goblin is hunted again.
        let b = rig.run(3).unwrap();
        assert!(!b.recoil);
        assert_eq!(b.director.allowed, vec![StableEntityId::goblin(1)]);
        assert_eq!(b.spot.unwrap().purpose, SpotPurpose::Attack);
    }

    #[test]
    fn a_swat_also_starts_the_recoil_but_a_missile_does_not() {
        for (id, recoil) in [(MoveId::Swat, true), (MoveId::MagicMissile, false)] {
            let mut rig = Rig::new();
            rig.goblin(1, 230.0, standing());
            rig.step();
            rig.donut().active_move = Some(ActiveMove {
                id,
                phase: MovePhase::Active,
                ticks_left: 5,
                ai_started: true,
            });
            rig.step();
            rig.donut().active_move = None;
            rig.step();
            assert_eq!(rig.st.recoil_until() > 0, recoil, "{id:?}");
        }
    }

    #[test]
    fn a_plan_dies_on_the_first_tick_its_target_leaves_allowed() {
        let mut rig = Rig::new();
        rig.goblin(1, 230.0, standing());
        rig.donut().x = 286.0;
        let set = free_set();
        let mut dir = CompanionDirector::new(HeroId::Donut);
        let b = rig.step().unwrap();
        let (_, d) = dir.step_briefed(&rig.v, &set, Some(&b.director));
        assert_eq!(d.target, Some(StableEntityId::goblin(1)), "planned");
        // The next tick the goblin joins his claim set (he hits it): the brief
        // no longer allows it, and the plan made a tick ago dies at once.
        let mut hit = enemy(1, 230.0, standing());
        hit.hp = 3;
        hit.carl_hit = true;
        hit.carl_hit_age = Some(1);
        rig.v.targets = vec![hit];
        let b = rig.step().unwrap();
        assert!(b.director.allowed.is_empty());
        let mut v = rig.v.clone();
        v.tick -= 1;
        v.tick += 1;
        let (_, d) = dir.step_briefed(&v, &set, Some(&b.director));
        assert_eq!(d.target, None);
        assert_eq!(d.mode, Mode::Follow);
        // The same plan unbriefed would have survived to the next planning tick.
        let mut free = CompanionDirector::new(HeroId::Donut);
        let mut v0 = rig.v.clone();
        v0.targets[0].carl_hit = false;
        v0.targets[0].carl_hit_age = None;
        free.step_briefed(&v0, &set, None);
        let mut v1 = v0.clone();
        v1.tick += 1;
        v1.targets = vec![hit];
        // (A08 vetoes a hit target only inside its HP reserve: hp 3 is not)
        let (_, d) = free.step_briefed(&v1, &set, None);
        assert_eq!(d.target, Some(StableEntityId::goblin(1)));
    }

    #[test]
    fn under_a_brief_the_press_needs_both_bands_and_she_must_already_face_it() {
        let set = free_set();
        let mut rig = Rig::new();
        rig.goblin(
            1,
            230.0,
            TargetState::Lying {
                down_ticks_left: 90,
                hold_used: false,
            },
        );
        let b = rig.step().unwrap();
        let brief = b.director.clone();
        rig.v.follow[1] = FollowInput {
            move_x: 0.25,
            jump: false,
        };
        // In band (56 px) and facing (she faces right, so put her left of it).
        let mut v = rig.v.clone();
        v.heroes[1].x = 230.0 - 56.0;
        let (raw, d) = CompanionDirector::new(HeroId::Donut).step_briefed(&v, &set, Some(&brief));
        assert!(d.pressed && raw.physical, "{d:?}");
        // Facing away from it: no press and no turn of the director's own.
        let mut v = rig.v.clone();
        v.heroes[1].x = 230.0 + 56.0;
        let (raw, d) = CompanionDirector::new(HeroId::Donut).step_briefed(&v, &set, Some(&brief));
        assert!(!d.pressed && raw.move_x == 0.25, "{raw:?} {d:?}");
        // Inside the move's reach band but outside the brief's: no press.
        let tight = GoalBrief {
            press_min_px: 30.0,
            press_max_px: 40.0,
            ..brief.clone()
        };
        let mut v = rig.v.clone();
        v.heroes[1].x = 230.0 - 56.0;
        let (raw, d) = CompanionDirector::new(HeroId::Donut).step_briefed(&v, &set, Some(&tight));
        assert!(!d.pressed && raw.move_x == 0.25);
        // Out of reach altogether: the director does not close in under a brief.
        let mut v = rig.v.clone();
        v.heroes[1].x = 230.0 - 150.0;
        let (raw, d) = CompanionDirector::new(HeroId::Donut).step_briefed(&v, &set, Some(&brief));
        assert!(!d.pressed && raw.move_x == 0.25);
        // The same facing-away view unbriefed does turn to the goblin (this
        // guards the test).
        let mut v = rig.v.clone();
        v.heroes[1].x = 230.0 + 56.0;
        v.targets[0].state = TargetState::Standing; // A13 turns to a standing one
        let (raw, _) = CompanionDirector::new(HeroId::Donut).step_briefed(&v, &set, None);
        assert_eq!(raw.move_x, -1.0);
        let (raw, d) = CompanionDirector::new(HeroId::Donut).step_briefed(
            &v,
            &set,
            Some(&GoalBrief::targeting(StableEntityId::goblin(1), 30.0, 80.0)),
        );
        assert_eq!(raw.move_x, 0.25, "{d:?}");
    }

    #[test]
    fn every_director_veto_still_applies_under_a_brief() {
        let set = free_set();
        let mut rig = Rig::new();
        rig.goblin(
            1,
            230.0,
            TargetState::Lying {
                down_ticks_left: 90,
                hold_used: false,
            },
        );
        let brief = rig.step().unwrap().director;
        let mut base = rig.v.clone();
        base.heroes[1].x = 230.0 - 56.0;
        let press = |v: &CompanionView, dir: &mut CompanionDirector| {
            dir.step_briefed(v, &set, Some(&brief)).1
        };
        // The base case presses.
        assert!(press(&base, &mut CompanionDirector::new(HeroId::Donut)).pressed);
        // Cannot start a move (hurt, frozen, in a move).
        let mut v = base.clone();
        v.heroes[1].can_start_move = false;
        assert!(!press(&v, &mut CompanionDirector::new(HeroId::Donut)).pressed);
        // The action cadence (Q16): a second press within 20 ticks.
        let mut dir = CompanionDirector::new(HeroId::Donut);
        assert!(press(&base, &mut dir).pressed);
        let mut v = base.clone();
        v.tick += 4;
        let d = press(&v, &mut dir);
        assert!(!d.pressed);
        // A08: a target with HP inside the reserve that the leader just hit is
        // not hers even if it is in `allowed`.
        let mut v = base.clone();
        v.targets[0].hp = 1;
        v.targets[0].carl_hit = true;
        v.targets[0].carl_hit_age = Some(2);
        assert!(!press(&v, &mut CompanionDirector::new(HeroId::Donut)).pressed);
        // The light guard (a glancing swat is no candidate).
        let mut v = base.clone();
        v.targets[0].guarded = true;
        assert!(!press(&v, &mut CompanionDirector::new(HeroId::Donut)).pressed);
        // Outside the scroll view: no press.
        let mut v = base.clone();
        v.view = ViewBounds {
            left: 1000.0,
            right: 1640.0,
        };
        assert!(!press(&v, &mut CompanionDirector::new(HeroId::Donut)).pressed);
    }

    #[test]
    fn an_interrupt_keeps_the_directors_own_approach_under_a_brief() {
        let set = free_set();
        let mut rig = Rig::new();
        // A tell aimed at Carl, 90 px from her: the swat is 4 ticks in.
        rig.v.threats = vec![ThreatView {
            id: StableEntityId::goblin(3),
            x: 330.0,
            foot_y: 0.0,
            facing: -1,
            tell_tick: 6,
            tell_len: 24,
            impact_tick: 26,
            aimed_at: Some(HeroId::Carl),
            guarded: false,
            in_view: true,
            swing_x: None,
        }];
        rig.v.follow[1] = FollowInput {
            move_x: -0.5,
            jump: false,
        };
        let b = rig.step().unwrap();
        let (raw, d) =
            CompanionDirector::new(HeroId::Donut).step_briefed(&rig.v, &set, Some(&b.director));
        assert_eq!(d.mode, Mode::Interrupt);
        assert_eq!(
            raw.move_x, 1.0,
            "she closes in on the tell: the director's approach"
        );
        // The Show-off hold is silent: no interrupt at all.
        let silent = GoalBrief::silent();
        let (raw, d) =
            CompanionDirector::new(HeroId::Donut).step_briefed(&rig.v, &set, Some(&silent));
        assert_eq!(d.mode, Mode::Follow);
        assert_eq!(raw.move_x, -0.5);
    }

    // ---------------------------------------------------------- retreat ----

    fn hurt(rig: &mut Rig) {
        let h = rig.donut().health;
        rig.donut().health = h - 1;
        rig.donut().hurt_ticks = 20;
    }

    #[test]
    fn a_hurt_with_a_standing_enemy_close_starts_a_retreat_at_once() {
        let mut rig = Rig::new();
        rig.goblin(1, 200.0, standing());
        rig.step(); // Hunt or whatever, started this tick
        rig.run(2);
        hurt(&mut rig);
        let b = rig.step().unwrap();
        assert_eq!(
            b.goal,
            Goal::Retreat,
            "no 24 tick minimum against an immediate trigger"
        );
        assert_eq!(b.changed, Some(GoalReason::Hurt));
        let spot = b.spot.expect("a retreat spot");
        assert_eq!(spot.purpose, SpotPurpose::Retreat);
        // Away from the goblin, and Carl (x 320) is between them.
        assert!(spot.x > 320.0, "{}", spot.x);
        // Retreat's brief: interrupts only.
        assert!(b.director.allowed.is_empty() && b.director.interrupts);
    }

    #[test]
    fn low_health_alone_never_triggers_a_retreat_and_only_widens_the_radius() {
        // Health 2 with no hurt: no retreat, however close the enemy.
        let mut rig = Rig::new();
        rig.donut().health = 2;
        rig.goblin(1, 230.0, standing());
        for _ in 0..30 {
            assert_ne!(rig.step().unwrap().goal, Goal::Retreat);
        }
        // A hurt with the enemy 110 px away: outside 96 at health 3, inside 128
        // once health is 2 or less.
        let mut rig = Rig::new();
        rig.donut().health = 3;
        rig.goblin(1, 130.0, standing()); // 110 px from her at 240
        rig.run(3);
        hurt(&mut rig); // health 2
        assert_eq!(rig.step().unwrap().goal, Goal::Retreat);
        let mut rig = Rig::new();
        rig.donut().health = 4;
        rig.goblin(1, 130.0, standing());
        rig.run(3);
        hurt(&mut rig); // health 3
        assert_ne!(rig.step().unwrap().goal, Goal::Retreat);
    }

    #[test]
    fn one_cooldown_covers_every_retreat_trigger_and_a_retreat_is_bounded() {
        let mut rig = Rig::new();
        rig.goblin(1, 200.0, standing());
        rig.run(3);
        hurt(&mut rig);
        assert_eq!(rig.step().unwrap().goal, Goal::Retreat);
        let started = rig.v.tick - 1;
        // The enemy stays close: the retreat lasts at most 120 ticks.
        let mut left = None;
        for i in 1..=130u64 {
            let b = rig.step().unwrap();
            if b.goal != Goal::Retreat {
                left = Some(i);
                assert_eq!(b.changed, Some(GoalReason::Ended));
                break;
            }
        }
        let ended = rig.v.tick - 1;
        assert_eq!(left, Some(120));
        assert_eq!(rig.st.ready_at().0, ended + 180, "cooldown from the end");
        let _ = started;
        // A new hurt inside the cooldown does not retreat again.
        hurt(&mut rig);
        for _ in 0..10 {
            assert_ne!(rig.step().unwrap().goal, Goal::Retreat);
        }
        // Past the cooldown a hurt does.
        rig.run(200);
        hurt(&mut rig);
        assert_eq!(rig.step().unwrap().goal, Goal::Retreat);
    }

    #[test]
    fn a_retreat_ends_early_when_no_standing_enemy_is_within_160_px() {
        let mut rig = Rig::new();
        rig.goblin(1, 200.0, standing());
        rig.run(3);
        hurt(&mut rig);
        assert_eq!(rig.step().unwrap().goal, Goal::Retreat);
        rig.run(5);
        rig.clear_targets();
        let b = rig.step().unwrap();
        assert_ne!(b.goal, Goal::Retreat);
        assert_eq!(b.changed, Some(GoalReason::Ended));
    }

    // ---------------------------------------------------------- show-off ----

    #[test]
    fn show_off_ignores_lying_and_staggered_enemies_and_holds_for_36_ticks() {
        let mut rig = Rig::new();
        // Down, and out of her reach (the drift check rejects a pounce this far
        // from him), so only the show-off rule is under test.
        rig.goblin(
            0,
            100.0,
            TargetState::Lying {
                down_ticks_left: 60,
                hold_used: false,
            },
        );
        rig.goblin(1, 110.0, TargetState::Staggered { ticks: 8 });
        // The quiet room has settled into Idle (a goal keeps 24 ticks).
        rig.run(30);
        rig.landed = true;
        // The hit landed; nothing standing within 128 px, no tell: show off.
        let mut first = None;
        for _ in 0..30 {
            let b = rig.step().unwrap();
            if b.goal == Goal::ShowOff {
                first = Some(b);
                break;
            }
        }
        let b = first.expect("a show-off starts");
        assert_eq!(b.face_x, Some(320.0), "she faces the leader");
        assert!(b.spot.is_none(), "she stands still");
        assert!(!b.director.interrupts && b.director.allowed.is_empty());
        let since = rig.st.goal_since();
        // 36 ticks later it ends and the 360 tick cooldown runs.
        let mut ended = None;
        for _ in 0..50 {
            let b = rig.step().unwrap();
            if b.goal != Goal::ShowOff {
                ended = Some(b);
                break;
            }
        }
        let b = ended.unwrap();
        assert_eq!(b.changed, Some(GoalReason::Ended));
        assert_eq!(rig.v.tick - 1 - since, 36);
        assert_eq!(rig.st.ready_at().1, rig.v.tick - 1 + 360);
    }

    #[test]
    fn show_off_needs_the_quiet_and_ends_at_once_on_a_threat_or_a_hurt() {
        // A standing enemy within 128 px blocks the start.
        let mut rig = Rig::new();
        rig.goblin(0, 330.0, standing());
        rig.run(30);
        rig.landed = true;
        rig.run(20);
        assert_ne!(rig.st.goal(), Goal::ShowOff);
        // A tell aimed at either hero blocks the start too.
        let mut rig = Rig::new();
        let tell = |aim| ThreatView {
            id: StableEntityId::goblin(5),
            x: 600.0,
            foot_y: 0.0,
            facing: -1,
            tell_tick: 3,
            tell_len: 24,
            impact_tick: 26,
            aimed_at: Some(aim),
            guarded: false,
            in_view: true,
            swing_x: None,
        };
        rig.v.threats = vec![tell(HeroId::Donut)];
        rig.run(30);
        rig.landed = true;
        rig.run(20);
        assert_ne!(rig.st.goal(), Goal::ShowOff);
        // The window is 45 ticks from the hit: later than that, no.
        let mut rig = Rig::new();
        rig.run(30);
        rig.landed = true;
        rig.step();
        rig.v.tick += 60;
        rig.run(8);
        assert_ne!(rig.st.goal(), Goal::ShowOff);
        // A running hold ends on a tell, and on a hurt.
        for ending in ["tell", "hurt"] {
            let mut rig = Rig::new();
            rig.run(30);
            rig.landed = true;
            rig.run(8);
            assert_eq!(rig.st.goal(), Goal::ShowOff, "{ending}");
            match ending {
                "tell" => rig.v.threats = vec![tell(HeroId::Carl)],
                _ => hurt(&mut rig),
            }
            let b = rig.step().unwrap();
            assert_ne!(b.goal, Goal::ShowOff, "{ending}");
            assert_eq!(b.changed, Some(GoalReason::Ended));
        }
    }

    // ------------------------------------------- hysteresis and triggers ----

    #[test]
    fn a_goal_keeps_for_24_ticks_unless_an_immediate_trigger_fires() {
        let mut rig = Rig::new();
        let b = rig.step().unwrap();
        assert_eq!(b.goal, Goal::Idle);
        // A hunt candidate appears 5 ticks in: she keeps Idle for the rest of
        // the 24 ticks, then switches.
        rig.run(4);
        rig.goblin(1, 230.0, standing());
        let mut switched = None;
        for _ in 0..40 {
            let b = rig.step().unwrap();
            if b.goal == Goal::Hunt {
                switched = Some(rig.v.tick - 1 - 100);
                break;
            }
        }
        let at = switched.expect("hunts eventually");
        assert!(at >= 24, "switched at {at}");
        assert!(at <= 30, "switched at {at}");
    }

    #[test]
    fn leaving_the_zone_wins_for_escort_at_once() {
        let mut rig = Rig::new();
        rig.goblin(1, 230.0, standing());
        rig.donut().x = 286.0;
        assert_eq!(rig.step().unwrap().goal, Goal::Hunt);
        // A knockback throws her out of Z (leash 240 around 320: x 70).
        rig.donut().x = 40.0;
        let b = rig.step().unwrap();
        assert_eq!(b.goal, Goal::Escort);
        assert_eq!(b.changed, Some(GoalReason::OutsideZone));
        assert!(!b.in_zone);
        let spot = b.spot.expect("an escort spot back inside Z");
        assert!(spot.x >= 80.0 && spot.x <= 560.0);
        assert!(
            b.director.allowed.is_empty(),
            "every other goal is ineligible"
        );
        // Back inside Z the hunt resumes after the minimum time.
        rig.donut().x = 200.0;
        rig.run(30);
        assert_eq!(rig.st.goal(), Goal::Hunt);
    }

    #[test]
    fn the_spot_is_held_until_it_is_invalid_or_the_leader_moves_48_px() {
        let mut rig = Rig::new();
        rig.goblin(1, 260.0, standing());
        rig.donut().x = 316.0;
        let first = rig.step().unwrap().spot.unwrap();
        assert_eq!(first.leader_ref_x, 320.0);
        // The leader shuffles 10 px: the spot is kept (CA2-Q30).
        rig.carl().x = 330.0;
        rig.run(12);
        assert_eq!(rig.st.goal(), Goal::Hunt);
        assert_eq!(rig.st.spot().unwrap(), first);
        // 55 px from the reference: it is re-picked against the new leader x.
        rig.carl().x = 375.0;
        rig.run(8);
        assert_eq!(rig.st.goal(), Goal::Hunt);
        assert_eq!(rig.st.spot().unwrap().leader_ref_x, 375.0);
    }

    #[test]
    fn she_comes_along_when_the_leader_travels() {
        let mut rig = Rig::new();
        let mut x = 320.0;
        // He walks right for 12 ticks: the travel counter reaches CA2-Q07.
        for _ in 0..30 {
            x += 3.0;
            rig.carl().x = x;
            rig.step();
        }
        assert!(rig.st.travel_ticks >= 12);
        // Her spot is behind him (to the left), 72 to 120 px.
        rig.donut().x = x - 150.0;
        let mut b = None;
        for _ in 0..6 {
            x += 3.0;
            rig.carl().x = x;
            b = rig.step();
        }
        let b = b.unwrap();
        assert_eq!(b.goal, Goal::Escort);
        let spot = b.spot.unwrap();
        assert!(spot.x < x, "behind a leader who travels right");
        // 72 to 120 px behind him when it was picked (the 16 px sampling gives
        // a few px either way); it is held until he has moved 48 px (CA2-Q30).
        let picked = (spot.leader_ref_x - spot.x).abs();
        assert!((72.0 - 8.0..=120.0 + 8.0).contains(&picked), "{picked}");
        assert!((x - spot.leader_ref_x).abs() <= 48.0 + 3.0);
        // While he travels a new Hunt does not start (Escort wins the tie).
        rig.goblin(1, x - 90.0, standing());
        let mut goal = Goal::Escort;
        for _ in 0..8 {
            x += 3.0;
            rig.carl().x = x;
            goal = rig.step().unwrap().goal;
        }
        assert_eq!(goal, Goal::Escort);
    }

    // ---------------------------------------------------------- lifecycle ----

    #[test]
    fn the_lifecycle_table_of_spec_5_4() {
        let scripted = |rig: &mut Rig, tick: u64| {
            rig.goblin(1, 230.0 + (tick % 7) as f32, standing());
            if tick == 20 {
                hurt(rig);
            }
            if tick == 45 {
                rig.donut().health = 3;
            }
            rig.step();
        };
        // Equal inputs give equal hashes, tick for tick.
        let (mut a, mut b) = (Rig::new(), Rig::new());
        for t in 0..200 {
            scripted(&mut a, t);
            scripted(&mut b, t);
            assert_eq!(a.st.state_hash(), b.st.state_hash(), "tick {t}");
        }
        assert!(a.st.stats().decisions > 5);
        // Swap: the partner becomes human, the plan is cleared, cooldowns stay.
        let cooldown = a.st.ready_at();
        assert!(cooldown.0 > 0, "the script produced a retreat cooldown");
        a.donut().control = Control::Human(0);
        assert!(a.step().is_none());
        assert_eq!(a.st.goal(), Goal::Escort);
        assert!(a.st.target().is_none() && a.st.spot().is_none());
        assert_eq!(a.st.recoil_until(), 0);
        assert_eq!(a.st.ready_at(), cooldown, "cooldowns survive a swap");
        // The hero becomes companion-driven again: it starts at Escort, since =
        // now, and a spot is picked on the next decision tick.
        a.donut().control = Control::Companion;
        let now = a.v.tick;
        let restart = a.step().unwrap();
        assert_eq!(restart.changed, Some(GoalReason::Start));
        assert_eq!(a.st.goal_since(), now);
        // The hash after the round trip depends only on the inputs: a second run
        // that makes the same swap lands on the same hash at every tick.
        let mut c = Rig::new();
        for t in 0..200 {
            scripted(&mut c, t);
        }
        c.donut().control = Control::Human(0);
        c.step();
        c.donut().control = Control::Companion;
        c.step();
        assert_eq!(c.st.state_hash(), a.st.state_hash());
        // The partner is defeated: nothing is decided while down.
        a.donut().defeated = true;
        assert!(a.step().is_none());
        assert!(a.st.spot().is_none());
        // A respawn, a defeat retry or a restart resets everything, cooldowns too.
        let kept = a.st.stats();
        assert!(kept.decisions > 0, "the run counted decisions");
        a.st.reset();
        // The M16 counters are the run's report: a restart keeps them (review F-1).
        assert_eq!(a.st.stats(), kept);
        a.st.carry_stats(AutonomyStats::default());
        assert_eq!(a.st, AutonomyState::default());
        assert_eq!(a.st.state_hash(), AutonomyState::new().state_hash());
    }

    #[test]
    fn a_defeated_leader_means_idle_with_interrupts_only() {
        let mut rig = Rig::new();
        rig.goblin(1, 230.0, standing());
        rig.donut().x = 286.0;
        assert_eq!(rig.step().unwrap().goal, Goal::Hunt);
        rig.carl().defeated = true;
        let b = rig.step().unwrap();
        assert_eq!(b.goal, Goal::Idle);
        assert!(b.spot.is_none() && b.target.is_none());
        assert!(b.director.allowed.is_empty() && b.director.interrupts);
        // It stays Idle while he is down, whatever walks in.
        for _ in 0..20 {
            assert_eq!(rig.step().unwrap().goal, Goal::Idle);
        }
    }

    #[test]
    fn the_state_hash_follows_the_state_and_ignores_the_counters() {
        let mut rig = Rig::new();
        let h0 = rig.st.state_hash();
        rig.goblin(1, 230.0, standing());
        rig.step();
        let h1 = rig.st.state_hash();
        assert_ne!(h0, h1);
        let mut other = rig.st.clone();
        other.stats.decisions += 100;
        assert_eq!(other.state_hash(), h1, "counters are a trace, never hashed");
        other.retreat_ready_at = 7;
        assert_ne!(other.state_hash(), h1);
    }

    /// No unordered map and no RNG anywhere in the production code of this
    /// module (the test section is excluded: it names the needles).
    #[test]
    fn the_layer_has_no_hash_map_and_no_rng() {
        let src = include_str!("autonomy.rs");
        let production = src.split("\n#[cfg(test)]\nmod tests").next().unwrap();
        for needle in [
            concat!("Hash", "Map"),
            concat!("Hash", "Set"),
            concat!("ra", "nd::"),
            concat!("thread", "_rng"),
            concat!("Random", "State"),
            concat!("Instant", "::now"),
        ] {
            assert!(!production.contains(needle), "{needle}");
        }
    }

    /// M14 (partial, B1): the decision cost in a busy view, measured and printed,
    /// asserted only against a generous ceiling.
    #[test]
    fn the_goal_layer_cost_per_step_is_measured() {
        let mut rig = Rig::new();
        for i in 0..8 {
            rig.goblin(i, 40.0 + 70.0 * i as f32, standing());
        }
        rig.world = FlatWorld {
            surfaces: vec![
                Surface {
                    key: 1,
                    left: 30.0,
                    right: 610.0,
                    y: 0.0,
                    risk: 0,
                },
                Surface {
                    key: 2,
                    left: 100.0,
                    right: 300.0,
                    y: 64.0,
                    risk: 0,
                },
                Surface {
                    key: 3,
                    left: 340.0,
                    right: 560.0,
                    y: 96.0,
                    risk: 0,
                },
            ],
            climb: Some(40),
        };
        let mut samples = Vec::new();
        for _ in 0..600 {
            let t0 = std::time::Instant::now();
            let b = rig.step();
            samples.push(t0.elapsed().as_nanos() as u64);
            assert!(b.is_some());
        }
        samples.sort_unstable();
        let (median, max) = (samples[samples.len() / 2], samples[samples.len() - 1]);
        println!(
            "autonomy step: median {median} ns, max {max} ns (debug build, 8 goblins, 3 surfaces)"
        );
        assert!(median < 2_000_000, "{median} ns");
    }

    #[test]
    fn spots_use_the_sampled_surfaces_and_ties_go_to_the_lower_key() {
        let mut rig = Rig::new();
        // Two identical surfaces: the lower key wins the tie.
        rig.world = FlatWorld {
            surfaces: vec![
                Surface {
                    key: 9,
                    left: 30.0,
                    right: 610.0,
                    y: 0.0,
                    risk: 0,
                },
                Surface {
                    key: 4,
                    left: 30.0,
                    right: 610.0,
                    y: 0.0,
                    risk: 0,
                },
            ],
            climb: None,
        };
        rig.goblin(1, 230.0, standing());
        rig.donut().x = 286.0;
        let spot = rig.step().unwrap().spot.unwrap();
        assert_eq!(spot.key, 4);
        // A risky surface is never a spot (CA2-Q29).
        let mut rig = Rig::new();
        rig.world.surfaces[0].risk = 250;
        rig.goblin(1, 230.0, standing());
        assert_ne!(rig.step().unwrap().goal, Goal::Hunt);
        // Sampling: every 16 px from the left end and the right end.
        let s = Surface {
            key: 1,
            left: 10.0,
            right: 60.0,
            y: 0.0,
            risk: 0,
        };
        let mut xs = Vec::new();
        sample_xs(&s, |x| xs.push(x));
        assert_eq!(xs, vec![10.0, 26.0, 42.0, 58.0, 60.0]);
    }

    // ------------------------------------------- the PR 205 review fixes ----

    /// F-1: a real pounce is airborne. She keeps the Hunt through the move (no
    /// routine re-decision while committed, and a decision taken in the air still
    /// prices routes from the surface below her), and the recoil runs after it.
    #[test]
    fn an_airborne_pounce_keeps_the_hunt_and_the_recoil_runs() {
        let mut rig = Rig::new();
        rig.goblin(
            1,
            230.0,
            TargetState::Lying {
                down_ticks_left: 90,
                hold_used: false,
            },
        );
        rig.donut().x = 286.0;
        assert_eq!(rig.step().unwrap().goal, Goal::Hunt);
        // The flight: 10 ticks in the air, 12 px up, inside the pounce move.
        rig.donut().active_move = Some(pounce_in_flight());
        rig.donut().foot_y = 12.0;
        rig.donut().grounded = false;
        for tick in 0..10 {
            let b = rig.step().unwrap();
            assert_eq!(b.goal, Goal::Hunt, "airborne tick {tick}");
            assert_eq!(b.changed, None);
        }
        // She lands and the move ends: the recoil window runs for 29 more ticks
        // and the goal is still Hunt.
        rig.donut().active_move = None;
        rig.donut().foot_y = 0.0;
        rig.donut().grounded = true;
        for tick in 0..30 {
            let b = rig.step().unwrap();
            assert_eq!(b.goal, Goal::Hunt, "recoil tick {tick}");
            assert!(b.recoil, "recoil tick {tick}");
            assert!(b.director.allowed.is_empty());
        }
        // Even a routine decision that falls in the air is skipped, so it cannot
        // drop the Hunt, whatever the world answers for an airborne partner.
        let mut rig = Rig::new();
        rig.goblin(
            1,
            230.0,
            TargetState::Lying {
                down_ticks_left: 90,
                hold_used: false,
            },
        );
        rig.donut().x = 286.0;
        rig.step();
        rig.world.climb = None;
        rig.donut().foot_y = 40.0; // no surface within 4 px, none below but the floor
        rig.donut().grounded = false;
        rig.donut().active_move = Some(pounce_in_flight());
        rig.surfaces_only_at(100.0);
        for _ in 0..12 {
            assert_eq!(rig.step().unwrap().goal, Goal::Hunt);
        }
    }

    impl Rig {
        /// Replace the world with one surface far from her feet.
        fn surfaces_only_at(&mut self, y: f32) {
            self.world.surfaces = vec![Surface {
                key: 7,
                left: 30.0,
                right: 610.0,
                y,
                risk: 0,
            }];
        }
    }

    /// F-2: the drift check runs from where she stands, every tick. The probe: far
    /// from her spot (230 px behind the leader, inside Z) a cast is not commit-safe,
    /// so the brief allows interrupts only and the director presses nothing; once
    /// she is at the spot it allows the target and the director casts.
    #[test]
    fn the_drift_check_runs_from_her_own_x_every_tick() {
        let unlocked = crate::companion::view::ManaView {
            mana: 6,
            max: 8,
            unlocked: true,
            ai_can_cast: true,
            projectile_speed_px_per_tick: 6,
            bolt_lead_px: 35,
        };
        let mut rig = Rig::new();
        rig.donut().mana = Some(unlocked);
        rig.donut().x = 90.0;
        rig.goblin(1, 200.0, standing());
        let set = free_set();
        let b = rig.step().unwrap();
        assert_eq!(b.goal, Goal::Hunt);
        let spot = b.spot.unwrap();
        assert!((spot.x - 126.0).abs() <= 8.0, "{}", spot.x);
        // From x 90 the missile's worst case leaves the view: no target allowed.
        assert!(b.director.allowed.is_empty(), "{:?}", b.director);
        assert_eq!(rig.st.stats().brief_drift_blocked_ticks, 1);
        let unbriefed = {
            let mut d = CompanionDirector::new(HeroId::Donut);
            d.step_briefed(&rig.v, &set, None).1
        };
        assert!(
            unbriefed.pressed,
            "the bare director would have cast: {unbriefed:?}"
        );
        let (raw, d) =
            CompanionDirector::new(HeroId::Donut).step_briefed(&rig.v, &set, Some(&b.director));
        assert!(!d.pressed && !raw.attack && !raw.physical);
        // She walks to the spot (x 126): allowed, and the director casts.
        rig.donut().x = 126.0;
        let b = rig.step().unwrap();
        assert_eq!(b.director.allowed, vec![StableEntityId::goblin(1)]);
        let (_, d) =
            CompanionDirector::new(HeroId::Donut).step_briefed(&rig.v, &set, Some(&b.director));
        assert!(d.pressed, "{d:?}");
        // A pounce is checked the same way (pre-unlock, a goblin 90 px behind him).
        let mut rig = Rig::new();
        rig.donut().x = 40.0;
        rig.goblin(1, 230.0, standing());
        let b = rig.step().unwrap();
        assert!(b.director.allowed.is_empty() || b.goal != Goal::Hunt);
    }

    /// F-2 (re-review geometry): a spot that stops passing the drift check is
    /// dropped on the very tick after the leader moves, before any cadence
    /// decision could replace it. Goblin at 200, Donut at 256, the leader at 320:
    /// the first spot (254) is commit-safe. When he walks 46 px away from her,
    /// the same spot fails the pounce's worst case, and `spot_valid` must say so
    /// at once (the cadence is 6 ticks and not due).
    #[test]
    fn a_spot_that_no_longer_passes_the_drift_check_is_dropped_before_the_cadence() {
        let mut rig = Rig::new();
        rig.donut().x = 256.0;
        rig.goblin(1, 200.0, standing());
        let first = rig.step().unwrap().spot.expect("a first spot");
        assert_eq!(rig.st.goal(), Goal::Hunt);
        assert_eq!(first.x, 254.0, "the geometry of the re-review");
        // The spot is valid as long as the leader stays (a few quiet ticks).
        let quiet = rig.run(2).unwrap();
        assert_eq!(quiet.spot, Some(first));
        // He walks 46 px away from her (366 against the spot at 254): the pounce's
        // worst case from the spot now leaves the view, so the spot must fail.
        rig.carl().x += 46.0;
        let zone = FreeZone::compute(
            &ZoneInput::new(&rig.v.heroes[0], rig.v.view, goal_rows()),
            &rig.set.autonomy,
        );
        let drift = DriftCheck::new(HeroMotion::CARL, &rig.set.autonomy);
        let tx = rig.v.target(StableEntityId::goblin(1)).unwrap().x;
        let end = tx - f32::from(sign(tx - first.x)) * POUNCE_CONTACT_PX;
        assert!(
            !drift.allows_pounce(&zone, first.x, end, &DonutMoveTuning::SPEC),
            "the moved leader must make the first spot fail the drift check"
        );
        let b = rig.step().unwrap();
        assert_ne!(
            b.spot,
            Some(first),
            "the stale spot {first:?} was kept on the tick after the move"
        );
        // And what she holds never fails the check (she may hold none at all).
        if let Some(spot) = b.spot {
            let end = tx - f32::from(sign(tx - spot.x)) * POUNCE_CONTACT_PX;
            assert!(drift.allows_pounce(&zone, spot.x, end, &DonutMoveTuning::SPEC));
        }
    }

    /// F-3: an interrupt is not exempt from the drift check (spec 3.3). From a
    /// position where a swat's worst case leaves the view the press is skipped
    /// (`Veto::DriftSkip`) and counted; inside the range it presses.
    #[test]
    fn an_interrupt_outside_the_commit_range_is_skipped_and_counted() {
        let set = free_set();
        let tell = |x: f32| ThreatView {
            id: StableEntityId::goblin(3),
            x,
            foot_y: 0.0,
            facing: -1,
            tell_tick: 6,
            tell_len: 24,
            impact_tick: 26,
            aimed_at: Some(HeroId::Carl),
            guarded: false,
            in_view: true,
            swing_x: None,
        };
        let mut rig = Rig::new();
        rig.donut().x = 90.0;
        rig.v.threats = vec![tell(110.0)];
        let b = rig.step().unwrap();
        let (lo, hi) = b.director.commit_x.expect("the range");
        // 16 ticks at 185 px/s is 49.3 px of drift; the camera bound is 276 px.
        assert!((lo - (320.0 - (276.0 - 49.34 - 8.0))).abs() < 0.1, "{lo}");
        assert!((hi - (320.0 + (276.0 - 49.34 - 8.0))).abs() < 0.1, "{hi}");
        assert!(rig.v.heroes[1].x < lo);
        // The bare director swats; under the brief the press is skipped.
        let free = CompanionDirector::new(HeroId::Donut)
            .step_briefed(&rig.v, &set, None)
            .1;
        assert!(free.pressed && free.mode == Mode::Interrupt, "{free:?}");
        let (raw, d) =
            CompanionDirector::new(HeroId::Donut).step_briefed(&rig.v, &set, Some(&b.director));
        assert_eq!(d.mode, Mode::Interrupt, "the interrupt is still chosen");
        assert!(!d.pressed && !raw.attack && !raw.physical);
        assert_eq!(d.veto, Some(Veto::DriftSkip));
        rig.st.record_decision(&d);
        assert_eq!(rig.st.stats().interrupts_drift_skipped, 1);
        // Inside the range the same interrupt presses.
        let mut rig = Rig::new();
        rig.donut().x = 130.0;
        rig.v.threats = vec![tell(150.0)];
        let b = rig.step().unwrap();
        let (raw, d) =
            CompanionDirector::new(HeroId::Donut).step_briefed(&rig.v, &set, Some(&b.director));
        assert!(d.pressed && raw.attack, "{d:?}");
        rig.st.record_decision(&d);
        assert_eq!(rig.st.stats().interrupts_drift_skipped, 0);
        // A near-edge leader loses the side toward the edge.
        let mut rig = Rig::new();
        rig.carl().x = 40.0;
        rig.donut().x = 60.0;
        let b = rig.step().unwrap();
        let (lo, hi) = b.director.commit_x.unwrap();
        // Next to the left edge nothing on his left qualifies (his own x and
        // everything left of it), but the right side still does.
        assert!(lo > 40.0 && hi > lo, "{lo}..{hi}");
    }

    /// F-4: while escorting she walks back into the escort band before she holds,
    /// so Escort honours its 24 tick minimum and does not flip to Idle at the
    /// comfort radius.
    #[test]
    fn an_escort_walks_into_the_escort_band_and_keeps_its_minimum_time() {
        let mut rig = Rig::new();
        rig.donut().x = 500.0; // 180 px from him, past the 160 px comfort radius
        let mut goals = Vec::new();
        for _ in 0..40 {
            let b = rig.step().unwrap();
            goals.push(b.goal);
            // She walks home at 3.45 px per tick and stops inside the band.
            let x = rig.donut().x;
            if x - 320.0 > 100.0 {
                rig.donut().x = x - 3.45;
            }
        }
        assert!(goals[..24].iter().all(|g| *g == Goal::Escort), "{goals:?}");
        let end = rig.donut().x;
        assert_eq!(*goals.last().unwrap(), Goal::Idle, "{goals:?}");
        let gap = (end - 320.0).abs();
        assert!((72.0..=120.0).contains(&gap), "{gap}");
        // The comfort radius alone (160 px) still holds a fresh partner.
        let mut rig = Rig::new();
        rig.donut().x = 470.0;
        assert_eq!(rig.step().unwrap().goal, Goal::Idle);
    }

    /// N-b: a Hunt that moves to a new target during the recoil springs back from
    /// it first (the recoil spot, whatever the target), with no target allowed.
    #[test]
    fn a_new_target_during_the_recoil_still_gets_the_recoil_spot() {
        let mut rig = Rig::new();
        rig.goblin(
            1,
            230.0,
            TargetState::Lying {
                down_ticks_left: 90,
                hold_used: false,
            },
        );
        rig.donut().x = 286.0;
        rig.step();
        rig.donut().active_move = Some(pounce_in_flight());
        rig.run(6);
        rig.donut().active_move = None;
        rig.step();
        // The first goblin dies; another one stands nearby.
        rig.clear_targets();
        rig.goblin(2, 250.0, standing());
        let b = rig.run(8).unwrap();
        assert_eq!(b.goal, Goal::Hunt);
        assert!(b.recoil && b.director.allowed.is_empty());
        assert_eq!(b.spot.unwrap().purpose, SpotPurpose::Recoil);
    }

    /// N-3: the zone proof needs a partner at least as fast as the leader. A slower
    /// partner (a future brawler next to a faster leader) is refused in debug
    /// builds; equal speed and faster are fine.
    #[test]
    fn a_partner_slower_than_the_leader_is_refused_in_debug_builds() {
        for (leader, partner) in [
            (HeroMotion::CARL, HeroMotion::CARL),
            (HeroMotion::CARL, crate::agility::agile_motion()),
            (
                crate::agility::agile_motion(),
                crate::agility::agile_motion(),
            ),
        ] {
            let mut rig = Rig::new();
            rig.leader_motion = leader;
            rig.partner_motion = partner;
            assert!(rig.step().is_some());
        }
        if cfg!(debug_assertions) {
            let refused = std::panic::catch_unwind(|| {
                let mut rig = Rig::new();
                rig.leader_motion = crate::agility::agile_motion();
                rig.partner_motion = HeroMotion::CARL;
                rig.step();
            });
            assert!(refused.is_err(), "a slower partner must be refused");
        }
    }

    // ===================================================================
    // CA2-C: the hazard feed in the goal layer (K13, CA2-Q29)
    // ===================================================================

    /// `FlatWorld` plus hazard x spans (a hero-hurting rectangle on the floor):
    /// her body (12 px each side) over a span is a hazard.
    struct HazWorld {
        inner: FlatWorld,
        spans: Vec<(f32, f32)>,
    }

    impl SpotWorld for HazWorld {
        fn surfaces(&self, out: &mut Vec<Surface>) {
            self.inner.surfaces(out);
        }

        fn route_ticks(&self, partner: &HeroView, x: f32, surface: &Surface) -> Option<u16> {
            self.inner.route_ticks(partner, x, surface)
        }

        fn hazard_at(&self, x: f32, y: f32) -> bool {
            self.hazard_between(x, x, y)
        }

        fn hazard_between(&self, x0: f32, x1: f32, y: f32) -> bool {
            let (lo, hi) = (x0.min(x1) - 12.0, x0.max(x1) + 12.0);
            y.abs() < 1.0 && self.spans.iter().any(|&(l, r)| l < hi && r > lo)
        }
    }

    /// The `Rig` of this module with a hazard world: `Rig.world` is a `FlatWorld`,
    /// so the hazard cases step the layer by hand.
    fn haz_step(rig: &mut Rig, spans: &[(f32, f32)]) -> Option<AutonomyBrief> {
        let a = rig.set.autonomy.clone();
        let leader_view = rig.v.heroes[0];
        let zone = FreeZone::compute(&ZoneInput::new(&leader_view, rig.v.view, goal_rows()), &a);
        let drift = DriftCheck::new(rig.leader_motion, &a);
        let tuning = DonutMoveTuning::SPEC;
        let world = HazWorld {
            inner: FlatWorld {
                surfaces: rig.world.surfaces.clone(),
                climb: rig.world.climb,
            },
            spans: spans.to_vec(),
        };
        let brief = rig.st.step(&AutonomyInput {
            view: &rig.v,
            set: &rig.set,
            partner: HeroId::Donut,
            zone: &zone,
            drift: &drift,
            partner_motion: rig.partner_motion,
            tuning: &tuning,
            world: &world,
            pan_remaining: rig.pan,
            partner_hit_landed: false,
        });
        rig.v.tick += 1;
        brief
    }

    /// A pounce's hop never runs over a hazard: with the goblin 120 px behind
    /// Carl and a hazard between her attack spot (about x 256) and it, there is no
    /// spot, so no Hunt; the rejection is counted as a hazard rejection, not a
    /// drift rejection. The same goblin with the hazard elsewhere is hunted.
    #[test]
    fn a_hazard_on_the_pounce_run_leaves_no_attack_spot_and_is_counted_as_one() {
        let mut clear = Rig::new();
        clear.goblin(1, 200.0, standing());
        let b = haz_step(&mut clear, &[(500.0, 560.0)]).unwrap();
        assert_eq!(b.goal, Goal::Hunt, "a hazard elsewhere changes nothing");
        assert_eq!(clear.st.stats().hunt_hazard_rejected, 0);

        let mut blocked = Rig::new();
        blocked.goblin(1, 200.0, standing());
        let b = haz_step(&mut blocked, &[(220.0, 245.0)]).unwrap();
        assert_ne!(b.goal, Goal::Hunt, "{b:?}");
        assert!(b.director.allowed.is_empty());
        let stats = blocked.st.stats();
        assert_eq!(stats.hunt_hazard_rejected, 1);
    }

    /// A held spot that a hazard turns up over is no longer a spot: the layer
    /// re-picks (the goal never stands her in a telegraphed lane).
    #[test]
    fn a_hazard_over_the_held_spot_makes_it_stale_and_it_is_re_picked() {
        let mut rig = Rig::new();
        rig.goblin(1, 200.0, standing());
        let b = haz_step(&mut rig, &[]).unwrap();
        assert_eq!(b.goal, Goal::Hunt);
        let spot = b.spot.unwrap();
        // A lane begins over the spot and the whole pounce band on this side of the
        // goblin: she is not committed (standing on the floor), so the next tick
        // re-decides.
        let lane = (spot.x - 30.0, spot.x + 60.0);
        let b = haz_step(&mut rig, &[lane]).unwrap();
        assert!(
            b.spot
                .is_none_or(|s| !(lane.0 - 12.0 < s.x && s.x < lane.1 + 12.0)),
            "no spot over the lane: {b:?}"
        );
        assert_ne!(b.goal, Goal::Hunt, "the pounce from there would cross it");
    }

    /// From where she stands the pounce would run over a hazard even though the
    /// held spot's does not: the director may interrupt only (and the tick is
    /// counted).
    #[test]
    fn the_brief_allows_interrupts_only_when_the_pounce_from_where_she_stands_crosses_a_hazard() {
        let mut rig = Rig::new();
        rig.goblin(1, 200.0, standing());
        rig.donut().x = 262.0;
        // The hazard touches her body at 262 (250..274) but not the spot's 256
        // (244..268): a span starting at 270.
        let b = haz_step(&mut rig, &[(270.0, 290.0)]).unwrap();
        assert_eq!(b.goal, Goal::Hunt);
        assert!(b.director.allowed.is_empty(), "interrupts only: {b:?}");
        assert_eq!(rig.st.stats().brief_hazard_blocked_ticks, 1);
        // Without the hazard the same tick allows the goblin.
        let mut free = Rig::new();
        free.goblin(1, 200.0, standing());
        free.donut().x = 262.0;
        let b = haz_step(&mut free, &[]).unwrap();
        assert_eq!(b.director.allowed, vec![StableEntityId::goblin(1)]);
        assert_eq!(free.st.stats().brief_hazard_blocked_ticks, 0);
    }

    /// A world with no feed (the default methods) is today's behaviour, byte for
    /// byte: `FlatWorld` implements neither hazard method.
    #[test]
    fn a_world_without_a_hazard_feed_never_reports_one() {
        let w = FlatWorld::floor();
        assert!(!w.hazard_at(100.0, 0.0));
        assert!(!w.hazard_between(0.0, 600.0, 0.0));
    }

    // ------------------------------------------------ DP-E: the escape blink --

    const KNOBS: BlinkKnobs = BlinkKnobs {
        threat_px: 48.0,
        min_gain_px: 64.0,
        hull_px: 63.0,
        windup_ticks: 6,
        commit_ticks: 12,
        charge_window_extra: None,
        reach_margin_px: None,
    };

    fn wide_zone(leader_x: f32) -> FreeZone {
        FreeZone {
            leader_x,
            left: leader_x - 240.0,
            right: leader_x + 240.0,
            reach_left: 240.0,
            reach_right: 240.0,
            safe_left: 292.0,
            safe_right: 292.0,
            surface_min_y: -100.0,
            surface_max_y: 300.0,
        }
    }

    fn threat_at(
        x: f32,
        tell_tick: u16,
        aimed_at: Option<HeroId>,
    ) -> crate::companion::view::ThreatView {
        crate::companion::view::ThreatView {
            id: StableEntityId::goblin(0),
            x,
            foot_y: 0.0,
            facing: -1,
            tell_tick,
            tell_len: 24,
            impact_tick: 26,
            aimed_at,
            guarded: false,
            in_view: true,
            swing_x: None,
        }
    }

    /// Donut at `x`, Carl at `carl_x`; `choose` with the given threats, targets,
    /// goal and a prediction that lands `dist` px from her in `dir`.
    struct Case {
        x: f32,
        carl_x: f32,
        threats: Vec<crate::companion::view::ThreatView>,
        targets: Vec<TargetView>,
        goal: Goal,
        dodge_jumps: bool,
        foot_y: f32,
        facing: i8,
        /// The camera-geometry bound the drift check reads on both sides.
        safe: f32,
        /// DP-F: the boss's tell or run, if any.
        boss: Option<BossThreat>,
        grounded: bool,
        /// The blink's numbers (the shipped ones by default).
        knobs: BlinkKnobs,
    }

    impl Case {
        fn new() -> Self {
            Self {
                x: 440.0,
                carl_x: 400.0,
                threats: vec![threat_at(470.0, 16, Some(HeroId::Donut))],
                targets: Vec::new(),
                goal: Goal::Hunt,
                dodge_jumps: false,
                foot_y: 0.0,
                facing: 1,
                safe: 292.0,
                boss: None,
                grounded: true,
                knobs: KNOBS,
            }
        }

        fn choose(&self, predict: impl Fn(i8) -> Option<f32>) -> Option<BlinkChoice> {
            let mut v = fixtures::view(
                leader(self.carl_x, 1),
                fixtures::hero(HeroId::Donut, self.x, Control::Companion),
            );
            v.heroes[1].foot_y = self.foot_y;
            v.heroes[1].facing = self.facing;
            v.heroes[1].grounded = self.grounded;
            v.boss = self.boss;
            v.threats = self.threats.clone();
            v.targets = self.targets.clone();
            let a = knobs();
            let zone = FreeZone {
                safe_left: self.safe,
                safe_right: self.safe,
                ..wide_zone(self.carl_x)
            };
            choose_blink(
                &BlinkInput {
                    view: &v,
                    partner: HeroId::Donut,
                    goal: self.goal,
                    zone: &zone,
                    drift: &DriftCheck::new(HeroMotion::CARL, &a),
                    knobs: self.knobs,
                    dodge_jumps: self.dodge_jumps,
                },
                predict,
            )
        }
    }

    /// A destination `d` px from her in the asked direction.
    fn lands(x: f32, d: f32) -> impl Fn(i8) -> Option<f32> {
        move |dir| Some(x + f32::from(dir) * d)
    }

    // ------------------------------------------------ FA-Q09, FA-Q10: the blink ----

    /// FA-Q09: with the knob the boss-charge window opens at the follow layer's
    /// jump lead (18) plus 2, so a blink can start before the dodge jump does; the
    /// lower edge (the windup) and the 48 px range are unchanged.
    #[test]
    fn fa_q09_the_charge_window_opens_at_the_jump_lead_plus_two_ticks() {
        let on = BlinkKnobs {
            charge_window_extra: Some(2),
            ..KNOBS
        };
        let fires = |knobs: BlinkKnobs, left: u16| {
            let mut c = boss_case(charge(-1, 38.0, left));
            c.knobs = knobs;
            c.choose(lands(440.0, 100.0)).is_some()
        };
        for left in 7..=20 {
            assert!(fires(on, left), "{left} ticks, knob on");
        }
        assert!(
            !fires(on, 6),
            "arrival on t6 is the impact tick: still too late"
        );
        assert!(!fires(on, 21), "one past the lead + 2");
        // Knob off: the shipped window (windup + 6 = 12).
        assert!(fires(KNOBS, 12) && !fires(KNOBS, 13) && !fires(KNOBS, 20));
        // The range is read at the ram's leading edge, as before.
        let mut c = boss_case(charge(-1, 48.5, 15));
        c.knobs = on;
        assert!(c.choose(lands(440.0, 100.0)).is_none());
        // The choice names the boss' charge (the latch's cue); a goblin's swing
        // does not.
        let mut c = boss_case(charge(-1, 38.0, 15));
        c.knobs = on;
        assert!(c.choose(lands(440.0, 100.0)).unwrap().boss_charge);
        assert!(!Case::new().choose(lands(440.0, 100.0)).unwrap().boss_charge);
        // A dodge jump that fires this tick still wins over the blink.
        let mut c = boss_case(charge(-1, 38.0, 15));
        c.knobs = on;
        c.dodge_jumps = true;
        assert!(c.choose(lands(440.0, 100.0)).is_none());
    }

    /// FA-Q10: a goblin's swing triggers the blink only if her hurtbox (half
    /// width 12) overlaps the swing's hitbox (8..30 px ahead of its feet) widened
    /// by 4 px: a swing 46 px away (centre to centre) reaches her, 47 does not;
    /// 30 blinks and 48 does not. Distance alone (knob off) blinks up to 48.
    #[test]
    fn fa_q10_a_swing_that_cannot_reach_her_does_not_spend_the_blink() {
        let gated = BlinkKnobs {
            reach_margin_px: Some(4.0),
            ..KNOBS
        };
        // A goblin at `x` facing left: its box is x-30..x-8.
        let swing_at = |knobs: BlinkKnobs, d: f32, facing: i8| {
            let x = 440.0 + f32::from(-facing) * d;
            let box_x = if facing < 0 {
                (x - 30.0, x - 8.0)
            } else {
                (x + 8.0, x + 30.0)
            };
            let mut c = Case::new();
            c.knobs = knobs;
            c.carl_x = 440.0 - 100.0 * f32::from(-facing);
            let mut t = threat_at(x, 16, Some(HeroId::Donut));
            t.facing = facing;
            t.swing_x = Some(box_x);
            c.threats = vec![t];
            c.choose(lands(440.0, 100.0)).is_some()
        };
        for facing in [-1i8, 1] {
            assert!(
                swing_at(gated, 30.0, facing),
                "30 px blinks (facing {facing})"
            );
            assert!(
                swing_at(gated, 46.0, facing),
                "46 px: the margin edge is inclusive"
            );
            assert!(!swing_at(gated, 47.0, facing), "47 px cannot reach her");
            assert!(!swing_at(gated, 48.0, facing), "48 px cannot reach her");
            // Knob off: distance alone, the 48 px range inclusive.
            assert!(swing_at(KNOBS, 47.0, facing) && swing_at(KNOBS, 48.0, facing));
        }
        // A threat with no known box (the llama, the Engineer's ram) is not gated.
        let mut c = Case::new();
        c.knobs = gated;
        c.threats = vec![threat_at(487.0, 16, Some(HeroId::Donut))];
        assert!(c.choose(lands(440.0, 100.0)).is_some());
    }

    /// FA-Q09: the per-charge latch is decision state that hashes only while set,
    /// survives a plan clear (a swap) and not a restart.
    #[test]
    fn fa_q09_the_charge_latch_hashes_only_while_set() {
        let mut state = AutonomyState::new();
        let mut before = Vec::new();
        state.hash_into(&mut |w| before.push(w));
        state.latch_charge_blink(30);
        assert!(state.charge_latched());
        let mut after = Vec::new();
        state.hash_into(&mut |w| after.push(w));
        assert_eq!(&after[..before.len()], &before[..]);
        assert_eq!(after.len(), before.len() + 1);
        state.clear_plan();
        assert!(state.charge_latched(), "a swap keeps it");
        assert!(!state.sync_charge_latch(None), "no charge up: released");
        let mut released = Vec::new();
        state.hash_into(&mut |w| released.push(w));
        assert_eq!(released, before, "released: hashes as before");
        state.latch_charge_blink(30);
        state.reset();
        assert!(!state.charge_latched(), "a restart forgets it");
    }

    /// FA-Q09 (F01-FU-B10): the latch is keyed by the charge's start tick. It holds
    /// through the charge it was set for (tell and run), and a chained charge (a new
    /// tell right after the run, with no tick without a charge between them) gets
    /// the dodge back.
    #[test]
    fn fa_q09_the_charge_latch_is_per_charge_start_and_a_chained_charge_releases_it() {
        let mut state = AutonomyState::new();
        state.latch_charge_blink(30);
        assert!(
            state.sync_charge_latch(Some(30)),
            "same charge: still silenced"
        );
        assert!(state.sync_charge_latch(Some(30)));
        assert!(!state.sync_charge_latch(Some(52)), "a chained charge");
        assert!(!state.charge_latched());
        // A fresh latch on the chained charge holds for it.
        state.latch_charge_blink(52);
        assert!(state.sync_charge_latch(Some(52)));
        // Nothing latched: nothing to keep, whatever is up.
        let mut idle = AutonomyState::new();
        assert!(!idle.sync_charge_latch(Some(52)));
        assert!(!idle.sync_charge_latch(None));
    }

    #[test]
    fn a_committed_attack_in_range_makes_her_blink_away_from_it() {
        let c = Case::new();
        let pick = c.choose(lands(440.0, 100.0)).expect("blinks");
        assert_eq!(pick.reason, BlinkReason::Committed);
        // The threat is at 470 (right of her): away is left.
        assert_eq!((pick.dir, pick.to_x, pick.threat_x), (-1, 340.0, 470.0));
        // The same threat on her left: away is right (hull: Carl 400 + 63 = 463
        // is the far edge, so the prediction must stay inside it).
        let mut c = Case::new();
        c.carl_x = 480.0;
        c.threats = vec![threat_at(410.0, 16, Some(HeroId::Donut))];
        let pick = c.choose(lands(440.0, 100.0)).expect("blinks");
        assert_eq!((pick.dir, pick.to_x), (1, 540.0));
    }

    #[test]
    fn the_threat_range_is_48_px_inclusive() {
        let mut c = Case::new();
        c.threats = vec![threat_at(440.0 + 48.0, 16, Some(HeroId::Donut))];
        assert!(c.choose(lands(440.0, 100.0)).is_some());
        c.threats = vec![threat_at(440.0 + 48.5, 16, Some(HeroId::Donut))];
        assert!(c.choose(lands(440.0, 100.0)).is_none());
        // The same on her left (Carl at 480, so the hull holds the blink right).
        c.carl_x = 480.0;
        c.threats = vec![threat_at(440.0 - 48.0, 16, Some(HeroId::Donut))];
        assert!(c.choose(lands(440.0, 70.0)).is_some());
        c.threats = vec![threat_at(440.0 - 48.5, 16, Some(HeroId::Donut))];
        assert!(c.choose(lands(440.0, 70.0)).is_none());
    }

    // ------------------------------------------- DP-F: the boss's charge --

    /// The brute's ram numbers (`brute_boss_f01`: box x 36..82, 300 px/s).
    const RAM_FRONT: f32 = 82.0;
    const RAM_DEPTH: f32 = 46.0;
    const RAM_SPEED: f32 = 5.0;

    /// A charge with its leading edge `lead_dx` px from her (she stands at 440)
    /// that reaches her `left` ticks from now: the boss at the right (dir -1) or
    /// the left (dir +1) of her.
    fn charge(dir: i8, lead_dx: f32, left: u16) -> BossThreat {
        let lead = 440.0 - f32::from(dir) * lead_dx;
        let gap = lead_dx - HERO_HALF_WIDTH;
        let run = (gap.max(0.0) / RAM_SPEED).ceil() as u16;
        BossThreat::Charge {
            tell_left: left.saturating_sub(run),
            x: lead - f32::from(dir) * RAM_FRONT,
            dir,
            running: false,
            front: RAM_FRONT,
            depth: RAM_DEPTH,
            speed: RAM_SPEED,
            lead_ticks: 18,
            started: 0,
        }
    }

    fn boss_case(boss: BossThreat) -> Case {
        let mut c = Case::new();
        c.threats = Vec::new();
        c.boss = Some(boss);
        c
    }

    #[test]
    fn a_committed_boss_charge_makes_her_blink_away_from_the_ram() {
        // The ram comes from the right (dir -1), its leading edge 38 px off: away
        // is left, onto Carl's side, as the swing's rule has it.
        let pick = boss_case(charge(-1, 38.0, 9))
            .choose(lands(440.0, 100.0))
            .expect("blinks");
        assert_eq!(pick.reason, BlinkReason::Committed);
        assert_eq!((pick.dir, pick.to_x, pick.threat_x), (-1, 340.0, 478.0));
        // From the left (dir +1): away is right (Carl at 480, so the hull holds).
        let mut c = boss_case(charge(1, 38.0, 9));
        c.carl_x = 480.0;
        let pick = c.choose(lands(440.0, 100.0)).expect("blinks");
        assert_eq!((pick.dir, pick.to_x, pick.threat_x), (1, 540.0, 402.0));
    }

    #[test]
    fn the_charge_window_and_range_are_exact_on_both_sides() {
        let fires = |left: u16| {
            boss_case(charge(-1, 38.0, left))
                .choose(lands(440.0, 100.0))
                .is_some()
        };
        assert!(!fires(6), "arrival on t6 is the impact tick: too late");
        assert!(fires(7));
        assert!(fires(12), "windup + 6");
        assert!(!fires(13), "not yet due");
        // The 48 px range is read at the ram's leading edge, inclusive.
        let near = |dx: f32| {
            boss_case(charge(-1, dx, 9))
                .choose(lands(440.0, 100.0))
                .is_some()
        };
        assert!(near(48.0));
        assert!(!near(48.5));
    }

    #[test]
    fn the_boss_charge_is_vetoed_by_the_dodge_the_air_a_pound_and_a_ram_going_away() {
        let mut c = boss_case(charge(-1, 38.0, 9));
        c.dodge_jumps = true;
        assert!(c.choose(lands(440.0, 100.0)).is_none(), "the jump dodge");
        let mut c = boss_case(charge(-1, 38.0, 9));
        c.grounded = false;
        assert!(c.choose(lands(440.0, 100.0)).is_none(), "in the air");
        let pound = BossThreat::Pound {
            x: 470.0,
            facing: -1,
            target: Some(HeroId::Donut),
        };
        assert!(
            boss_case(pound).choose(lands(440.0, 100.0)).is_none(),
            "a pound is the step-out's, not the blink's"
        );
        // A ram that runs away from her: its box never reaches her.
        assert!(
            boss_case(charge(-1, 38.0, 9))
                .choose(lands(440.0, 100.0))
                .is_some()
        );
        let away = BossThreat::Charge {
            tell_left: 9,
            x: 560.0,
            dir: 1,
            running: false,
            front: RAM_FRONT,
            depth: RAM_DEPTH,
            speed: RAM_SPEED,
            lead_ticks: 18,
            started: 0,
        };
        assert!(boss_case(away).choose(lands(440.0, 100.0)).is_none());
    }

    #[test]
    fn the_boss_charge_blink_keeps_the_gain_zone_hull_and_drift_rules() {
        let c = || boss_case(charge(-1, 38.0, 9));
        // Gain: 100 px away from a ram edge 38 px off is a 100 px gain; 63 is not.
        assert!(c().choose(lands(440.0, 64.0)).is_some());
        assert!(c().choose(lands(440.0, 63.0)).is_none());
        // The hull of Carl (400) less 63 is 337: a landing at 330 is outside it.
        assert!(c().choose(lands(440.0, 110.0)).is_none());
        // No destination: no press.
        assert!(c().choose(|_| None).is_none());
    }

    #[test]
    fn the_impact_window_is_more_than_the_windup_and_at_most_windup_plus_six() {
        // impact 26: tell tick t leaves 26 - t ticks.
        let ticks_left = |left: u16| {
            let mut c = Case::new();
            c.threats = vec![threat_at(470.0, 26 - left, Some(HeroId::Donut))];
            c.choose(lands(440.0, 100.0)).is_some()
        };
        assert!(!ticks_left(5), "the arrival would come after the impact");
        assert!(!ticks_left(6), "arrival on t6 is the impact tick: too late");
        assert!(ticks_left(7), "the arrival comes first");
        assert!(ticks_left(12), "windup + 6");
        assert!(!ticks_left(13), "not yet due");
    }

    #[test]
    fn a_jump_dodge_a_swing_above_her_or_another_target_all_veto_the_committed_blink() {
        let mut c = Case::new();
        c.dodge_jumps = true;
        assert!(c.choose(lands(440.0, 100.0)).is_none(), "the dodge fires");
        let mut c = Case::new();
        c.foot_y = 31.0;
        assert!(
            c.choose(lands(440.0, 100.0)).is_none(),
            "above the swing box"
        );
        c.foot_y = 30.0;
        assert!(c.choose(lands(440.0, 100.0)).is_some(), "at the box top");
        let mut c = Case::new();
        c.threats = vec![threat_at(470.0, 16, Some(HeroId::Carl))];
        assert!(c.choose(lands(440.0, 100.0)).is_none(), "aimed at Carl");
        c.threats = vec![threat_at(470.0, 16, None)];
        assert!(c.choose(lands(440.0, 100.0)).is_none(), "aimed at no one");
    }

    #[test]
    fn retreat_blinks_from_a_standing_enemy_on_top_of_her_and_only_in_retreat() {
        let mut c = Case::new();
        c.threats = Vec::new();
        c.targets = vec![enemy(0, 470.0, TargetState::Standing)];
        // Not Retreat: never (DP-Q40: not to chase, not offence).
        for goal in [Goal::Hunt, Goal::Escort, Goal::SupportLeader, Goal::Idle] {
            c.goal = goal;
            assert!(c.choose(lands(440.0, 100.0)).is_none(), "{goal:?}");
        }
        c.goal = Goal::Retreat;
        let pick = c.choose(lands(440.0, 100.0)).expect("retreat blinks");
        assert_eq!(pick.reason, BlinkReason::Retreat);
        // A staggered or lying enemy cannot hurt her this tick.
        c.targets = vec![enemy(0, 470.0, TargetState::Staggered { ticks: 8 })];
        assert!(c.choose(lands(440.0, 100.0)).is_none());
        // Out of range.
        c.targets = vec![enemy(0, 500.0, TargetState::Standing)];
        assert!(c.choose(lands(440.0, 100.0)).is_none());
    }

    #[test]
    fn the_nearest_threat_decides_the_direction() {
        let mut c = Case::new();
        c.threats = vec![
            threat_at(440.0 - 40.0, 16, Some(HeroId::Donut)),
            threat_at(440.0 + 20.0, 16, Some(HeroId::Donut)),
        ];
        let pick = c.choose(lands(440.0, 100.0)).unwrap();
        assert_eq!((pick.threat_x, pick.dir), (460.0, -1));
        // Level with it: toward the leader, else against her facing (the asked
        // direction is read off the prediction, whatever it answers).
        let asked = |carl_x: f32, facing: i8| {
            let mut c = Case::new();
            c.threats = vec![threat_at(440.0, 16, Some(HeroId::Donut))];
            c.carl_x = carl_x;
            c.facing = facing;
            let dir = std::cell::Cell::new(0);
            let _ = c.choose(|d| {
                dir.set(d);
                None
            });
            dir.get()
        };
        assert_eq!(asked(300.0, 1), -1, "Carl on the left");
        assert_eq!(asked(600.0, -1), 1, "Carl on the right");
        assert_eq!(asked(440.0, 1), -1, "level with both: against her facing");
        assert_eq!(asked(440.0, -1), 1);
    }

    #[test]
    fn the_destination_must_gain_64_px_from_the_threat() {
        // Threat 30 px away: a 94 px blink gains 94 - 0 ... measured as the new
        // distance less the old one.
        let mut c = Case::new();
        c.threats = vec![threat_at(470.0, 16, Some(HeroId::Donut))];
        // Old distance 30: new distance must be at least 94, a 64 px blink.
        assert!(c.choose(lands(440.0, 64.0)).is_some());
        assert!(c.choose(lands(440.0, 63.0)).is_none());
        // No destination at all: nothing.
        assert!(c.choose(|_| None).is_none());
    }

    #[test]
    fn the_destination_must_be_inside_the_zone_the_hull_and_the_drift_check() {
        // Hull: [min(440, 400 - 63), max(440, 400 + 63)] = [337, 463].
        let c = Case::new();
        assert!(
            c.choose(lands(440.0, 103.0)).is_some(),
            "337 is on the edge"
        );
        assert!(
            c.choose(lands(440.0, 103.5)).is_none(),
            "just outside the hull"
        );
        // The hull's far side: a destination between her and Carl is inside.
        let mut c = Case::new();
        c.carl_x = 480.0;
        c.threats = vec![threat_at(410.0, 16, Some(HeroId::Donut))];
        assert!(c.choose(lands(440.0, 103.0)).is_some(), "543 = 480 + 63");
        assert!(c.choose(lands(440.0, 104.0)).is_none());
        // The zone: 240 px each side of Carl (a leader at 400: 160..640).
        let mut c = Case::new();
        c.x = 200.0;
        c.carl_x = 400.0;
        c.threats = vec![threat_at(230.0, 16, Some(HeroId::Donut))];
        // Away is left: 200 - 64 = 136 is outside the zone (160) and the hull.
        assert!(c.choose(lands(200.0, 64.0)).is_none());
        // Drift: 12 ticks of Carl at 3.08 px + 8 slack = 45 px, so an end point
        // is at most `safe - 45` px from him: the 340 landing is 60 px from Carl.
        let mut c = Case::new();
        c.safe = 106.0;
        assert!(c.choose(lands(440.0, 100.0)).is_some());
        c.safe = 104.0;
        assert!(c.choose(lands(440.0, 100.0)).is_none());
    }

    #[test]
    fn a_hurt_busy_or_defeated_partner_never_blinks() {
        let mut v = fixtures::view(
            leader(400.0, 1),
            fixtures::hero(HeroId::Donut, 440.0, Control::Companion),
        );
        v.threats = vec![threat_at(470.0, 16, Some(HeroId::Donut))];
        let a = knobs();
        let run = |v: &CompanionView| {
            choose_blink(
                &BlinkInput {
                    view: v,
                    partner: HeroId::Donut,
                    goal: Goal::Hunt,
                    zone: &wide_zone(400.0),
                    drift: &DriftCheck::new(HeroMotion::CARL, &a),
                    knobs: KNOBS,
                    dodge_jumps: false,
                },
                lands(440.0, 100.0),
            )
        };
        assert!(run(&v).is_some());
        let mut hurt = v.clone();
        hurt.heroes[1].hurt_ticks = 3;
        assert!(run(&hurt).is_none());
        let mut dead = v.clone();
        dead.heroes[1].defeated = true;
        assert!(run(&dead).is_none());
        let mut busy = v;
        busy.heroes[1].active_move = Some(crate::companion::view::ActiveMove {
            id: MoveId::Swat,
            phase: crate::companion::view::MovePhase::Startup,
            ticks_left: 10,
            ai_started: true,
        });
        assert!(run(&busy).is_none());
    }

    #[test]
    fn a_blink_press_is_hashed_decision_state_and_an_unpressed_layer_hashes_as_before() {
        let mut state = AutonomyState::new();
        let mut before = Vec::new();
        state.hash_into(&mut |w| before.push(w));
        state.note_blink(25, BlinkReason::Committed);
        let mut after = Vec::new();
        state.hash_into(&mut |w| after.push(w));
        assert_eq!(
            &after[..before.len()],
            &before[..],
            "a prefix: nothing moved"
        );
        assert_eq!(after.len(), before.len() + 2);
        assert_eq!(after[after.len() - 1], 25);
        assert_eq!(state.last_blink(), Some(25));
        assert_eq!(state.stats().blink_presses_committed, 1);
        // A plan clear keeps it (a swap must not forget a running blink); a
        // restart does not.
        state.clear_plan();
        assert_eq!(state.last_blink(), Some(25));
        state.reset();
        assert_eq!(state.last_blink(), None);
        assert_eq!(state.stats().blink_presses_committed, 1, "counters survive");
    }

    // ------------------------------------- FA-Q02..Q07: boss play (PR-A) ----

    use crate::companion::view::ManaView;

    /// A boss at `x` facing `facing` in `class`, the way `BossView::from_facts`
    /// would build it (a view of its own so the goal layer is tested without the
    /// scene).
    fn boss_at(x: f32, facing: i8, class: BossModeClass) -> BossView {
        let ahead = |a: f32, b: f32| {
            let (p, q) = (x + f32::from(facing) * a, x + f32::from(facing) * b);
            (p.min(q), p.max(q))
        };
        BossView {
            x,
            foot_y: 0.0,
            facing,
            hp: 30,
            phase: 1,
            hurtbox: Some(Rect {
                left: x - 40.0,
                bottom: 0.0,
                width: 80.0,
                height: 64.0,
            }),
            mode_class: class,
            vulnerable_ticks_left: if class == BossModeClass::Vulnerable {
                60
            } else {
                0
            },
            target: Some(HeroId::Carl),
            charging: false,
            pound_recovery: false,
            inert: false,
            ram_box: ahead(36.0, 82.0),
            committed_box: None,
            ripple_reach_px: 0.0,
        }
    }

    fn stunned(x: f32, facing: i8) -> BossView {
        BossView {
            inert: true,
            ..boss_at(x, facing, BossModeClass::Vulnerable)
        }
    }

    /// A rig with a boss in the view and Donut able to cast (full mana).
    fn boss_rig(boss: BossView) -> Rig {
        let mut rig = Rig::new();
        rig.v.boss_view = Some(boss);
        rig.donut().mana = Some(ManaView {
            mana: 8,
            max: 8,
            unlocked: true,
            ai_can_cast: true,
            projectile_speed_px_per_tick: 6,
            bolt_lead_px: 35,
        });
        rig
    }

    #[test]
    fn boss_play_is_off_without_its_knob_and_the_boss_view_is_ignored() {
        let mut rig = boss_rig(stunned(420.0, -1));
        rig.set.fix_all.fa_q02_boss_play = false;
        for _ in 0..40 {
            let b = rig.step().unwrap();
            assert_ne!(b.goal, Goal::BossPunish);
        }
        assert_eq!(rig.st.stats().retreat_entries, 0);
    }

    #[test]
    fn a_stunned_boss_with_24_ticks_left_is_a_window_and_with_23_it_is_not() {
        for (left, expect) in [(24u16, true), (23, false)] {
            let mut boss = stunned(420.0, -1);
            boss.vulnerable_ticks_left = left;
            let mut rig = boss_rig(boss);
            let b = rig.step().unwrap();
            assert_eq!(b.goal == Goal::BossPunish, expect, "{left} ticks left");
            if expect {
                // The cast spacing, on her own side of the boss, facing it.
                let spot = b.spot.expect("an attack spot");
                let d = (420.0 - spot.x).abs();
                assert!((72.0..=160.0).contains(&d), "cast band {d}");
                assert!(spot.x < 420.0, "her side");
                assert_eq!(b.face_x, Some(420.0));
                assert_eq!(b.director.allowed, vec![BOSS_ID]);
                assert_eq!(b.changed, Some(GoalReason::Start));
            }
        }
    }

    #[test]
    fn a_window_opening_mid_goal_is_taken_past_the_24_tick_minimum() {
        let mut rig = boss_rig(boss_at(420.0, -1, BossModeClass::Neutral));
        rig.run(3);
        rig.v.boss_view = Some(stunned(420.0, -1));
        // At most one cadence (6 ticks) later she has switched, with the reason.
        let mut changed = None;
        for _ in 0..7 {
            let b = rig.step().unwrap();
            if b.goal == Goal::BossPunish {
                changed = b.changed;
                break;
            }
        }
        assert_eq!(changed, Some(GoalReason::BossWindow));
    }

    #[test]
    fn boss_punish_ends_the_tick_the_boss_leaves_its_vulnerable_modes() {
        let mut rig = boss_rig(stunned(420.0, -1));
        assert_eq!(rig.step().unwrap().goal, Goal::BossPunish);
        rig.run(2);
        // 3 ticks old, far inside the 24-tick floor: the exit is immediate.
        rig.v.boss_view = Some(boss_at(420.0, -1, BossModeClass::Neutral));
        let b = rig.step().unwrap();
        assert_ne!(b.goal, Goal::BossPunish);
        assert_eq!(b.changed, Some(GoalReason::BossWindow));
        assert!(b.director.allowed.is_empty());
    }

    #[test]
    fn a_dry_partner_punishes_from_the_pounce_band() {
        let mut rig = boss_rig(stunned(420.0, -1));
        rig.donut().mana = Some(ManaView {
            mana: 0,
            max: 8,
            unlocked: true,
            ai_can_cast: false,
            projectile_speed_px_per_tick: 6,
            bolt_lead_px: 35,
        });
        let b = rig.step().unwrap();
        assert_eq!(b.goal, Goal::BossPunish);
        let d = (420.0 - b.spot.unwrap().x).abs();
        assert!((30.0..=80.0).contains(&d), "pounce band {d}");
    }

    #[test]
    fn a_pound_recovery_with_ripples_on_the_way_is_no_window() {
        let mut boss = boss_at(420.0, -1, BossModeClass::Vulnerable);
        boss.pound_recovery = true;
        boss.vulnerable_ticks_left = 24;
        // Phase 1 (no ripples): the recovery is a window...
        assert_eq!(boss_rig(boss).step().unwrap().goal, Goal::BossPunish);
        // ...from phase 2 it is not (walking to a spot would cross the ripples).
        boss.ripple_reach_px = 176.0;
        assert_ne!(boss_rig(boss).step().unwrap().goal, Goal::BossPunish);
    }

    #[test]
    fn the_boss_is_a_standing_enemy_for_a_retreat_except_in_stun_and_pinned() {
        // A hurt with the boss 60 px away.
        for (boss, retreats) in [
            (boss_at(300.0, -1, BossModeClass::Neutral), true),
            (boss_at(300.0, -1, BossModeClass::Tell), true),
            (stunned(300.0, -1), false),
        ] {
            let mut rig = boss_rig(boss);
            rig.donut().x = 240.0;
            rig.run(2);
            hurt(&mut rig);
            let b = rig.step().unwrap();
            assert_eq!(b.goal == Goal::Retreat, retreats, "{:?}", boss.mode_class);
        }
    }

    #[test]
    fn a_boss_retreat_spot_is_clear_of_the_boss_on_her_side_and_out_of_its_lane() {
        // Boss at 420 facing left, a ram box 338..384 ahead of it; she is at 300.
        let mut rig = boss_rig(boss_at(420.0, -1, BossModeClass::Neutral));
        rig.donut().x = 300.0;
        rig.run(2);
        hurt(&mut rig);
        let b = rig.step().unwrap();
        assert_eq!(b.goal, Goal::Retreat);
        let spot = b.spot.unwrap();
        assert!(spot.x < 420.0, "her side of the boss");
        assert!((420.0 - spot.x) >= 128.0, "128 px clear: {}", spot.x);
        // Not in the lane (338..384 each way by her half width).
        assert!(spot.x + HERO_HALF_WIDTH <= 338.0 || spot.x - HERO_HALF_WIDTH >= 384.0);
    }

    #[test]
    fn with_no_clear_ground_the_retreat_takes_the_farthest_anchor_on_her_side() {
        // Boss at 150 facing right, she is in the corner at 60: 128 px clear is
        // impossible on her side (the surface starts at 30).
        let mut rig = boss_rig(boss_at(150.0, 1, BossModeClass::Neutral));
        rig.donut().x = 60.0;
        rig.carl().x = 90.0;
        rig.run(2);
        hurt(&mut rig);
        let b = rig.step().unwrap();
        assert_eq!(b.goal, Goal::Retreat);
        let spot = b.spot.unwrap();
        assert!(spot.x <= 60.0, "the corner side of the boss: {}", spot.x);
    }

    /// F01-FU-B10 (#250 review): under a boss with an add, a spot that passes the
    /// boss rules but is within 112 px of the goblin (FA-Q27 `unclear`) beats the
    /// FA-Q03 fallback (the farthest spot from the boss, here beside the goblin).
    #[test]
    fn under_a_boss_an_unclear_spot_beats_the_farthest_anchor_fallback() {
        // A narrow floor 230..330 with a goblin at 250: no spot is 112 px clear of
        // it. The boss at 560 is 128+ px from all of it and its lane is far away.
        let mut rig = boss_rig(boss_at(560.0, -1, BossModeClass::Neutral));
        rig.world.surfaces[0].left = 230.0;
        rig.world.surfaces[0].right = 330.0;
        rig.goblin(1, 250.0, standing());
        rig.donut().x = 290.0;
        rig.carl().x = 300.0;
        rig.run(2);
        hurt(&mut rig);
        let b = rig.step().unwrap();
        assert_eq!(b.goal, Goal::Retreat);
        let spot = b.spot.expect("a spot");
        // The fallback would be the far left end, beside the goblin; the old rule's
        // best spot is as far from it as the floor allows.
        assert!(spot.x - 250.0 >= 40.0, "away from the goblin: {}", spot.x);
    }

    #[test]
    fn a_retreat_path_across_a_committed_boss_hitbox_is_no_spot() {
        // The pound hit box of a boss at 300 facing left covers 233..268. She is at
        // 240 inside it and the only clear ground is across it (to 150): no spot is
        // reached through the live box, so there is no Retreat.
        let mut boss = boss_at(300.0, -1, BossModeClass::Committed);
        boss.committed_box = Some((233.0, 268.0));
        let mut rig = boss_rig(boss);
        rig.donut().x = 240.0;
        rig.carl().x = 120.0;
        rig.run(2);
        hurt(&mut rig);
        let b = rig.step().unwrap();
        assert_ne!(b.goal, Goal::Retreat);
    }

    #[test]
    fn a_boss_tell_within_the_hurt_radius_starts_a_retreat_without_a_hurt() {
        let mut rig = boss_rig(boss_at(300.0, -1, BossModeClass::Tell));
        rig.donut().x = 240.0;
        let b = rig.run(3).unwrap();
        assert_eq!(b.goal, Goal::Retreat);
        assert_eq!(rig.st.stats().retreat_entries, 1);
        // ...but not for a charge (the dodge is the answer), nor beyond the radius.
        for boss in [
            BossView {
                charging: true,
                ..boss_at(300.0, -1, BossModeClass::Tell)
            },
            boss_at(420.0, -1, BossModeClass::Tell),
        ] {
            let mut rig = boss_rig(boss);
            rig.donut().x = 240.0;
            assert_ne!(rig.run(3).unwrap().goal, Goal::Retreat);
        }
        // And not with the knob off.
        let mut rig = boss_rig(boss_at(300.0, -1, BossModeClass::Tell));
        rig.set.fix_all.fa_q03_retreat_on_boss_tell = false;
        rig.donut().x = 240.0;
        assert_ne!(rig.run(3).unwrap().goal, Goal::Retreat);
    }

    #[test]
    fn the_boss_escort_stands_off_between_112_and_160_and_never_between_the_boss_and_carl() {
        // Carl at 320, boss at 520 facing left: the band is x 360..408, which lies
        // between the two, so the spot falls back to Carl's far side.
        let mut rig = boss_rig(boss_at(520.0, -1, BossModeClass::Neutral));
        rig.donut().x = 100.0;
        let b = rig.run(2).unwrap();
        assert_eq!(b.goal, Goal::Escort);
        let spot = b.spot.expect("an escort spot");
        assert!(spot.x <= 320.0, "not between the boss and Carl: {}", spot.x);
        assert!((520.0 - spot.x) >= 112.0);
        // Boss behind Carl (left of him) and she stands between them, 50 px from
        // the boss: the standoff pulls her out of the gap to Carl's far side.
        let mut rig = boss_rig(boss_at(200.0, 1, BossModeClass::Neutral));
        rig.donut().x = 250.0;
        let b = rig.run(2).unwrap();
        assert_eq!(b.goal, Goal::Escort);
        let spot = b.spot.expect("an escort spot");
        assert!(spot.x >= 320.0 && (spot.x - 200.0) >= 112.0, "{}", spot.x);
    }

    #[test]
    fn from_phase_2_the_standoff_clears_the_ripples_reach() {
        let mut boss = boss_at(200.0, 1, BossModeClass::Neutral);
        boss.phase = 2;
        boss.ripple_reach_px = 176.0;
        let mut rig = boss_rig(boss);
        rig.donut().x = 400.0;
        let spot = rig.run(2).unwrap().spot.expect("an escort spot");
        // 176 + 12 (her half width) + 16 (the margin) = 204.
        assert!((spot.x - 200.0) >= 204.0, "{}", spot.x);
    }

    #[test]
    fn the_boss_escort_goes_out_of_the_ram_box_under_a_tell_and_has_no_travel_bonus() {
        // A pound tell: the lane is the ram box ahead of the boss; spots in it are
        // out. (The ordinary 72..120 band behind Carl would be inside it.)
        let mut rig = boss_rig(boss_at(520.0, -1, BossModeClass::Tell));
        rig.donut().x = 100.0;
        let b = rig.run(2).unwrap();
        let spot = b.spot.expect("a spot");
        assert!(!(spot.x + HERO_HALF_WIDTH > 438.0 && spot.x - HERO_HALF_WIDTH < 484.0));
        // No travel bonus: while Carl walks (the leader travels) the Escort keeps
        // its 800 and a Retreat or BossPunish at 1500 still outranks it.
        let mut rig = boss_rig(stunned(520.0, -1));
        rig.pan = 50;
        assert_eq!(rig.step().unwrap().goal, Goal::BossPunish);
    }

    #[test]
    fn while_a_charge_is_up_the_escort_is_the_ordinary_one_and_holds_the_dodge() {
        let mut boss = boss_at(520.0, -1, BossModeClass::Tell);
        boss.charging = true;
        let mut rig = boss_rig(boss);
        rig.donut().x = 100.0;
        let b = rig.run(2).unwrap();
        // The standoff would hold her 112+ from the boss and out of the box; the
        // ordinary escort walks her up to Carl (72..120 behind him).
        assert_eq!(b.goal, Goal::Escort);
        let d = (b.spot.expect("a spot").x - 320.0).abs();
        assert!((72.0..=120.0).contains(&d), "{d}");
    }

    #[test]
    fn a_boss_closing_in_on_a_holding_partner_breaks_the_hold_past_the_minimum_goal_time() {
        // She holds 120 px from a boss that is 240 away (Idle, minimum goal time
        // running); the boss walks to 90 px from her, inside the standoff band: she
        // re-decides within one cadence, with the reason, and walks out.
        let mut rig = boss_rig(boss_at(560.0, -1, BossModeClass::Neutral));
        rig.donut().x = 320.0;
        rig.carl().x = 320.0;
        assert_eq!(rig.run(8).unwrap().goal, Goal::Idle);
        rig.v.boss_view = Some(boss_at(410.0, -1, BossModeClass::Neutral));
        let mut seen = None;
        for _ in 0..7 {
            let b = rig.step().unwrap();
            if b.goal == Goal::Escort {
                seen = Some((b.changed, b.spot));
                break;
            }
        }
        let (changed, spot) = seen.expect("she leaves Idle");
        assert_eq!(changed, Some(GoalReason::BossStandoff));
        assert!((410.0 - spot.expect("a spot").x) >= 112.0);
    }

    // ----------------------------------------- FA-Q25 to FA-Q28: the CA2 metrics ----

    /// FA-Q27: a retreat spot is at least 112 px from the nearest standing enemy
    /// when any ground allows it (knob on).
    #[test]
    fn fa_q27_the_retreat_spot_is_at_least_112_px_from_the_nearest_standing_enemy() {
        let mut rig = Rig::new();
        rig.goblin(1, 300.0, standing());
        rig.run(2);
        hurt(&mut rig);
        let b = rig.step().unwrap();
        assert_eq!(b.goal, Goal::Retreat);
        let spot = b.spot.unwrap();
        assert!((spot.x - 300.0).abs() >= 112.0, "{}", spot.x);
        // With no ground that clear (a goblin in the middle of a narrow floor) the
        // old rule's best spot still stands: a Retreat is never lost to the rule.
        let mut rig = Rig::new();
        rig.world.surfaces[0].left = 250.0;
        rig.world.surfaces[0].right = 350.0;
        rig.goblin(1, 300.0, standing());
        rig.run(2);
        hurt(&mut rig);
        assert!(rig.step().unwrap().spot.is_some());
    }

    /// FA-Q28: while he travels and an unclaimed Hunt candidate is within 160 px of
    /// her, the Escort bonus is 200, so the hunt (1200) beats Escort (1000); with
    /// the knob off it is the old 400 and Escort wins the tie.
    #[test]
    fn fa_q28_a_near_unclaimed_enemy_lets_the_hunt_win_over_a_travelling_escort() {
        let hunts = |on: bool| {
            let mut rig = Rig::new();
            rig.set.fix_all.fa_q28_hunt_escort_bonus = on;
            rig.pan = 40;
            rig.goblin(1, 230.0, standing());
            rig.step().unwrap().goal
        };
        assert_eq!(hunts(true), Goal::Hunt);
        assert_eq!(hunts(false), Goal::Escort);
        // Beyond 160 px of her the bonus stays 400.
        let mut rig = Rig::new();
        rig.pan = 40;
        rig.goblin(1, 405.0, standing());
        assert_ne!(rig.step().unwrap().goal, Goal::Hunt);
    }

    /// FA-Q25: both heroes walking the same way within 64 px for more than 12
    /// ticks makes her re-pick her escort spot at least 80 px from him.
    #[test]
    fn fa_q25_a_shadowing_partner_re_picks_the_escort_spot_80_px_from_him() {
        let run = |on: bool| {
            let mut rig = Rig::new();
            rig.set.fix_all.fa_q25_shadow_respot = on;
            rig.pan = 40;
            rig.carl().x = 200.0;
            rig.donut().x = 160.0;
            let mut last = None;
            for _ in 0..30 {
                rig.carl().x += 2.0;
                rig.donut().x += 2.0;
                last = rig.step();
            }
            (rig.st.shadow_ticks, last.unwrap())
        };
        let (ticks, b) = run(true);
        assert!(ticks > 12, "{ticks}");
        assert_eq!(b.goal, Goal::Escort);
        let spot = b.spot.expect("an escort spot");
        // Carl has walked to x 260.
        assert!((spot.x - 260.0).abs() >= 80.0, "{}", spot.x);
        assert_eq!(run(false).0, 0, "knob off never counts");
    }

    #[test]
    fn a_goal_utility_table_has_boss_punish_between_support_and_retreat() {
        let g = &free_set().autonomy.roles.skirmisher.goals;
        assert!(g.retreat.utility > g.boss_punish.utility);
        assert!(g.boss_punish.utility > g.support_leader.utility);
        assert!(g.boss_punish.under_boss && g.boss_punish.enabled);
        assert_eq!(Goal::BossPunish.name(), "boss_punish");
    }
}
