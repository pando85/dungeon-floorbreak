//! COMPANION-AI-02, stage CA2-A: the pure zone, drift and claim code of the
//! partner's autonomy inside the camera. Nothing here is wired: no scene,
//! director or camera reads it yet (the goal selector is CA2-B1, the wiring
//! and the camera rule are CA2-B2). Spec: `docs/03-tech/companion-ai-02-donut-
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
//! All of it is a pure function of the view, the profile and the camera
//! geometry: no state, no RNG, no map iteration, nothing hashed.

use crate::combat::MAGIC_MISSILE;
use crate::companion::engage::leader_engages;
use crate::companion::profile::{Autonomy, ProfileSet};
use crate::companion::view::{
    CompanionView, HeroView, StableEntityId, TargetState, TargetView, ViewBounds,
};
use crate::donut_moves::{
    DonutMoveTuning, POUNCE_HIT_STOP, POUNCE_PIN, POUNCE_RECOVERY, POUNCE_STARTUP, SWAT_HIT_STOP,
    SWAT_TOTAL,
};
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
/// - the front claim: the nearest **standing** enemy on the side he faces, on
///   his level (feet within `cf02_duo_a09_vertical_reach_px`), within CA2-Q10
///   of him: the one he is about to work (review FU-D: lying and staggered
///   enemies are not front claims; they are claimed only through engagement).
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
/// claim: standing, on his facing side (an enemy exactly on him is in front),
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
    (matches!(target.state, TargetState::Standing)
        && in_front
        && distance <= radius
        && (target.foot_y - leader.foot_y).abs() <= vertical)
        .then_some(distance)
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
        // FU-D: lying and staggered enemies are not front claims.
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
        down.x = 441.0;
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
}
