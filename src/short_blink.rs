//! DONUT-PROG-01 DP-C: the short horizontal blink, simulation only. Bevy-free.
//!
//! The special (Circle/B) of a hero that holds the `special_short_blink`
//! capability (spec DP-Q20..DP-Q32, DP-Q35, DP-Q36): a press edge starts a
//! 6-tick windup, the arrival is on the tick after it (t6, a horizontal warp of
//! up to 112 px to the farthest valid candidate), a 6-tick recovery follows and
//! a 540-tick cooldown counts from the arrival.
//!
//! This module is the pure part: the destination search ([`search`], eight
//! rules, one pure function of the tick's state), the press gate, the arrival
//! resolver ([`resolve_arrival`]) and the [`Blink`] state machine (windup,
//! recovery, cooldown). The combat scene (`combat_scene/blink.rs`) feeds it the
//! press, the world and the hurt, and turns its outcomes into events, the
//! `BodyDrive::warp_x` of the arrival tick and the mana spend. An idle blink
//! (never pressed, never cooling down) mixes **nothing** into a state hash.
//!
//! IP-neutral: nothing here names a franchise item; names are fan-layer data.
//! Every number is a knob of `donut_prog_v0.json` (read through
//! [`DonutProgTuning`]). No RNG, no hash map: a fixed candidate order and exact
//! tick compares.

use crate::donut_prog::DonutProgTuning;
use crate::prototype::{Block, HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Rect};
use crate::thrown_charge::SpecialReject;

/// The hero index the blink events carry (Donut).
pub const BLINK_HERO: u8 = crate::donut_mana::DONUT_HERO;

/// Why an accepted blink did not happen on its arrival tick (DP-Q28).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FizzleReason {
    /// She held fewer charges than the cost on the arrival tick.
    NoMana,
    /// The search from her current position found no valid candidate.
    NoDestination,
}

impl FizzleReason {
    pub const ALL: [Self; 2] = [Self::NoMana, Self::NoDestination];

    pub const fn name(self) -> &'static str {
        match self {
            Self::NoMana => "no_mana",
            Self::NoDestination => "no_destination",
        }
    }

    /// Stable code for event hashes.
    pub const fn code(self) -> u64 {
        self as u64
    }
}

/// What cancelled a windup (DP-Q25, DP-Q29, DP-Q11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelCause {
    /// A primary, physical or jump press edge in the windup.
    Input,
    /// She was hurt or defeated in the windup.
    Hurt,
    /// A defeat retry, a checkpoint respawn or a fixture restart.
    Restart,
}

impl CancelCause {
    pub const ALL: [Self; 3] = [Self::Input, Self::Hurt, Self::Restart];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Hurt => "hurt",
            Self::Restart => "restart",
        }
    }

    /// Stable code for event hashes.
    pub const fn code(self) -> u64 {
        self as u64
    }
}

/// DP-Q21: the blink direction. The stick's horizontal component past the
/// deadzone decides, otherwise her facing; the vertical component is never read.
pub fn direction(move_x: f32, facing: i8, deadzone: f32) -> i8 {
    if move_x >= deadzone {
        1
    } else if move_x <= -deadzone || facing < 0 {
        -1
    } else {
        1
    }
}

/// Everything the destination search reads of the world, at the tick it runs.
/// All rectangles are in the world's y-up px.
#[derive(Debug, Clone, Copy)]
pub struct SearchWorld<'a> {
    /// The scene's solids. An intact one is a collider and a barrier (rules 2
    /// to 4a, including a closed boss gate).
    pub solids: &'a [Block],
    /// Rule 4b: the room's `blink_barriers` rectangles (absent = none).
    pub barriers: &'a [Rect],
    /// Rule 4c: the x ranges `[min_x, max_x)` of every story zone that has not
    /// fired. The blink may end inside one but never jump across it.
    pub zones: &'a [(f32, f32)],
    /// Rule 8: the hero-hurting hazard rectangles the scene declares.
    pub hazards: &'a [Rect],
    /// Rule 1: the largest centre `x` of the world.
    pub x_max: f32,
    /// Rule 5: the **simulation** camera's view `[left, right]`.
    pub view: (f32, f32),
    /// Rule 6, two players only: Carl's `x` and the separation limit.
    pub partner: Option<(f32, f32)>,
}

fn body_box(x: f32, y: f32) -> Rect {
    Rect {
        left: x - HERO_HALF_WIDTH,
        bottom: y - HERO_HALF_HEIGHT,
        width: HERO_HALF_WIDTH * 2.0,
        height: HERO_HALF_HEIGHT * 2.0,
    }
}

/// The union of the start and end boxes (the sweep).
fn swept_box(from: f32, to: f32, y: f32) -> Rect {
    let (low, high) = if from <= to { (from, to) } else { (to, from) };
    Rect {
        left: low - HERO_HALF_WIDTH,
        bottom: y - HERO_HALF_HEIGHT,
        width: (high - low) + HERO_HALF_WIDTH * 2.0,
        height: HERO_HALF_HEIGHT * 2.0,
    }
}

/// Whether the blink from `x` to `x2` jumps across the zone `[min, max)`: she
/// starts outside it on one side and ends past its far edge. Ending inside it
/// is allowed (the zone fires on the arrival tick's story step).
fn crosses_zone(x: f32, x2: f32, (min, max): (f32, f32)) -> bool {
    (x < min && x2 >= max) || (x >= max && x2 < min)
}

/// The eight validity rules of DP-Q27 (spec section 5.2) for one candidate
/// column `x2`, from her pivot `(x, y)`.
pub fn candidate_valid(w: &SearchWorld<'_>, x: f32, y: f32, x2: f32, t: &DonutProgTuning) -> bool {
    // 1. World.
    if x2 < HERO_HALF_WIDTH || x2 > w.x_max {
        return false;
    }
    let end = body_box(x2, y);
    let sweep = swept_box(x, x2, y);
    // 2. Collision-safe, 3. no wall crossed (the sweep contains the end box).
    if w.solids
        .iter()
        .filter(|b| b.intact)
        .any(|b| end.overlaps(b.rect) || sweep.overlaps(b.rect))
    {
        return false;
    }
    // 4. No protected gate crossed: (a) is rule 3, (b) the room's rectangles,
    // (c) the zones not yet fired.
    if w.barriers.iter().any(|r| sweep.overlaps(*r)) {
        return false;
    }
    if w.zones.iter().any(|z| crosses_zone(x, x2, *z)) {
        return false;
    }
    // 5. Visible: the body box inside the simulation view, inset on each side.
    let inset = f32::from(t.dpq27_view_inset_px);
    if end.left < w.view.0 + inset || end.right() > w.view.1 - inset {
        return false;
    }
    // 6. Shared camera (two players): within the separation limit, or no
    // farther from the partner than now.
    if let Some((partner_x, limit)) = w.partner {
        let after = (x2 - partner_x).abs();
        if after > limit && after > (x - partner_x).abs() {
            return false;
        }
    }
    // 7. Support: a downward probe from her feet at `x2` meets the top of an
    // intact solid within the probe length. Only the destination needs it.
    let feet = y - HERO_HALF_HEIGHT;
    let probe = f32::from(t.dpq27_support_probe_px);
    let supported = w
        .solids
        .iter()
        .filter(|b| b.intact)
        .filter(|b| x2 >= b.rect.left && x2 <= b.rect.right())
        .map(|b| b.rect.top())
        .any(|top| top <= feet + 0.01 && feet - top <= probe);
    if !supported {
        return false;
    }
    // 8. Hazards.
    !w.hazards.iter().any(|h| end.overlaps(*h))
}

/// DP-Q22, DP-Q27: the candidates from the maximum down to the minimum in
/// fixed steps (25 at the adopted values); the **farthest valid** one wins. A
/// pure function of the tick's state; ties cannot happen (one direction, a
/// strict order).
pub fn search(w: &SearchWorld<'_>, x: f32, y: f32, dir: i8, t: &DonutProgTuning) -> Option<f32> {
    let step = i32::from(t.dpq22_range_step_px).max(1);
    let min = i32::from(t.dpq22_range_min_px);
    let mut distance = i32::from(t.dpq22_range_max_px);
    while distance >= min {
        let x2 = x + f32::from(dir) * distance as f32;
        if candidate_valid(w, x, y, x2, t) {
            return Some(x2);
        }
        distance -= step;
    }
    None
}

/// The outcome of the arrival tick (DP-Q28).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Arrival {
    /// She lands on this column.
    Resolved(f32),
    Fizzled(FizzleReason),
}

/// DP-Q28: the arrival checks in order. (1) **Mana**: fewer charges than the
/// cost aborts (`NoMana`) **before** the search is run, so `find` is never
/// called then. (2) The search runs again from her current position (`find`);
/// no valid candidate aborts (`NoDestination`). A fizzle spends nothing and
/// starts no cooldown.
pub fn resolve_arrival(charges: u8, cost: u8, find: impl FnOnce() -> Option<f32>) -> Arrival {
    if charges < cost {
        return Arrival::Fizzled(FizzleReason::NoMana);
    }
    match find() {
        Some(x) => Arrival::Resolved(x),
        None => Arrival::Fizzled(FizzleReason::NoDestination),
    }
}

/// What the scene knows at the press tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PressView {
    /// Not hurt, not defeated, not in hit-stop (the legacy gate).
    pub can_act: bool,
    /// A swat, pounce, cast or bail-out hop is running.
    pub busy: bool,
    /// Her charges now.
    pub charges: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Idle,
    /// The press tick `start` (t0) and the locked direction.
    Windup { start: u64, dir: i8 },
    /// The last tick of the recovery.
    Recovery { until: u64 },
}

/// One hero's blink. `Default` is idle: it mixes nothing and costs nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Blink {
    phase: Phase,
    /// The absolute tick the special is ready again (`arrival + cooldown`).
    ready_tick: Option<u64>,
}

impl Blink {
    /// Nothing running and nothing cooling down.
    pub fn is_idle(&self) -> bool {
        *self == Self::default()
    }

    /// The running windup `(t0, direction)`.
    pub fn windup(&self) -> Option<(u64, i8)> {
        match self.phase {
            Phase::Windup { start, dir } => Some((start, dir)),
            _ => None,
        }
    }

    /// Ticks since the press (0 on t0) while a windup runs.
    pub fn windup_age(&self, tick: u64) -> Option<u64> {
        self.windup().map(|(start, _)| tick.saturating_sub(start))
    }

    /// The recovery is running on `tick` (t7..t12 after the press).
    pub fn recovering(&self, tick: u64) -> bool {
        matches!(self.phase, Phase::Recovery { until } if tick <= until)
    }

    /// The tick the cooldown ends on, while one is running.
    pub fn ready_tick(&self) -> Option<u64> {
        self.ready_tick
    }

    /// The cooldown is running on `tick`.
    pub fn cooling(&self, tick: u64) -> bool {
        self.ready_tick.is_some_and(|ready| tick < ready)
    }

    /// The first tick of a step: an over recovery ends.
    pub fn tick_over(&mut self, tick: u64) {
        if matches!(self.phase, Phase::Recovery { until } if tick > until) {
            self.phase = Phase::Idle;
        }
    }

    /// The cooldown ended on this tick (or earlier, never reported): once.
    pub fn take_ready(&mut self, tick: u64) -> bool {
        match self.ready_tick {
            Some(ready) if tick >= ready => {
                self.ready_tick = None;
                true
            }
            _ => false,
        }
    }

    /// DP-Q26: the press gate, in order: busy, cooldown, mana, destination.
    /// `find` (the search) runs only when everything before it passed.
    pub fn press_gate(
        &self,
        tick: u64,
        view: &PressView,
        cost: u8,
        find: impl FnOnce() -> Option<f32>,
    ) -> Result<(), SpecialReject> {
        if !view.can_act || view.busy {
            return Err(SpecialReject::Busy);
        }
        if self.cooling(tick) {
            return Err(SpecialReject::Cooldown);
        }
        if view.charges < cost {
            return Err(SpecialReject::NoMana);
        }
        match find() {
            Some(_) => Ok(()),
            None => Err(SpecialReject::NoDestination),
        }
    }

    /// An accepted press: t0 is this tick.
    pub fn begin(&mut self, tick: u64, dir: i8) {
        self.phase = Phase::Windup { start: tick, dir };
    }

    /// Cancel a running windup (no mana, no cooldown). `true` when one ran.
    pub fn cancel(&mut self) -> bool {
        let running = self.windup().is_some();
        if running {
            self.phase = Phase::Idle;
        }
        running
    }

    /// The arrival tick: the windup spent its ticks (`windup_ticks` after t0).
    pub fn arrival_due(&self, tick: u64, windup_ticks: u8) -> bool {
        self.windup()
            .is_some_and(|(start, _)| tick == start + u64::from(windup_ticks))
    }

    /// The blink resolved on `tick`: the recovery starts and the cooldown
    /// counts from here.
    pub fn arrived(&mut self, tick: u64, t: &DonutProgTuning) {
        self.phase = Phase::Recovery {
            until: tick + u64::from(t.dpq25_recovery_ticks),
        };
        self.ready_tick = Some(tick + u64::from(t.dpq24_cooldown_ticks));
    }

    /// The blink fizzled: nothing was spent, no cooldown, no recovery.
    pub fn fizzled(&mut self) {
        self.phase = Phase::Idle;
    }

    /// Fold into the state hash: **nothing** while idle.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        if self.is_idle() {
            return;
        }
        mix(0x424c_4e4b_0000_0001); // "BLNK"
        match self.phase {
            Phase::Idle => mix(0),
            Phase::Windup { start, dir } => {
                mix(1);
                mix(start);
                mix(dir as u64);
            }
            Phase::Recovery { until } => {
                mix(2);
                mix(until);
            }
        }
        mix(self.ready_tick.map_or(0, |t| t + 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prototype::Material;

    fn t() -> &'static DonutProgTuning {
        DonutProgTuning::active()
    }

    fn floor() -> Block {
        Block {
            rect: Rect {
                left: 0.0,
                bottom: 0.0,
                width: 2000.0,
                height: 48.0,
            },
            material: Material::Stone,
            intact: true,
        }
    }

    fn wall(left: f32, width: f32) -> Block {
        Block {
            rect: Rect {
                left,
                bottom: 48.0,
                width,
                height: 120.0,
            },
            material: Material::Stone,
            intact: true,
        }
    }

    /// Her pivot standing on the floor at column `x`.
    const Y: f32 = 48.0 + HERO_HALF_HEIGHT;

    fn world<'a>(solids: &'a [Block]) -> SearchWorld<'a> {
        SearchWorld {
            solids,
            barriers: &[],
            zones: &[],
            hazards: &[],
            x_max: 1988.0,
            view: (0.0, 2000.0),
            partner: None,
        }
    }

    #[test]
    fn the_direction_follows_the_stick_past_the_deadzone_else_her_facing() {
        assert_eq!(direction(0.5, -1, 0.5), 1, "exactly at the deadzone");
        assert_eq!(direction(0.49, -1, 0.5), -1, "just inside: her facing");
        assert_eq!(direction(-0.5, 1, 0.5), -1);
        assert_eq!(direction(-0.49, 1, 0.5), 1);
        assert_eq!(direction(0.0, 1, 0.5), 1);
        assert_eq!(direction(0.0, -1, 0.5), -1);
    }

    #[test]
    fn the_flat_search_takes_the_maximum_range_in_both_directions() {
        let solids = [floor()];
        let w = world(&solids);
        assert_eq!(search(&w, 500.0, Y, 1, t()), Some(612.0));
        assert_eq!(search(&w, 500.0, Y, -1, t()), Some(388.0));
    }

    #[test]
    fn the_candidate_count_is_25_and_the_minimum_is_16() {
        let steps: Vec<i32> = (16..=112).rev().step_by(4).collect();
        assert_eq!(steps.len(), 25);
        // A wall whose face is at 528 leaves the 16 px candidate only: its end
        // box (504..528) touches the face, the 20 px one (508..532) overlaps.
        let solids = [floor(), wall(528.0, 16.0)];
        let w = world(&solids);
        assert_eq!(search(&w, 500.0, Y, 1, t()), Some(516.0));
        // One px closer and even 16 px is blocked.
        let solids = [floor(), wall(527.0, 16.0)];
        assert_eq!(search(&world(&solids), 500.0, Y, 1, t()), None);
    }

    #[test]
    fn a_thin_wall_is_never_crossed() {
        // A 16 px wall 40 px ahead: she lands in front of it, never behind.
        let solids = [floor(), wall(540.0, 16.0)];
        let w = world(&solids);
        let x = search(&w, 500.0, Y, 1, t()).unwrap();
        assert!(x + HERO_HALF_WIDTH <= 540.0, "{x}");
        assert_eq!(x, 528.0 - 0.0, "the farthest column that clears the face");
        // A broken (not intact) wall is no obstacle.
        let mut broken = [floor(), wall(540.0, 16.0)];
        broken[1].intact = false;
        assert_eq!(search(&world(&broken), 500.0, Y, 1, t()), Some(612.0));
    }

    #[test]
    fn a_barrier_rectangle_stops_the_sweep_like_a_wall() {
        let solids = [floor()];
        let barrier = [Rect {
            left: 560.0,
            bottom: 48.0,
            width: 8.0,
            height: 10.0,
        }];
        let mut w = world(&solids);
        w.barriers = &barrier;
        let x = search(&w, 500.0, Y, 1, t()).unwrap();
        assert!(x + HERO_HALF_WIDTH <= 560.0, "{x}");
        // A barrier above her head (a body of 52 px) does not touch.
        let high = [Rect {
            left: 560.0,
            bottom: 48.0 + 52.0,
            width: 8.0,
            height: 10.0,
        }];
        w.barriers = &high;
        assert_eq!(search(&w, 500.0, Y, 1, t()), Some(612.0));
    }

    #[test]
    fn a_zone_not_yet_fired_may_be_entered_never_skipped() {
        let solids = [floor()];
        let mut w = world(&solids);
        // A 40 px zone 30..70 px ahead: a 112 px blink would skip it.
        let zone = [(530.0, 570.0)];
        w.zones = &zone;
        let x = search(&w, 500.0, Y, 1, t()).unwrap();
        assert!(x < 570.0, "ends inside or before the zone: {x}");
        assert_eq!(x, 568.0, "the farthest column that is still inside it");
        // Left of the zone the same rule applies mirrored.
        let x = search(&w, 600.0, Y, -1, t()).unwrap();
        assert_eq!(x, 532.0);
        // Starting inside a zone (it would have fired) is not blocked.
        assert_eq!(search(&w, 540.0, Y, 1, t()), Some(652.0));
        // Half-open: ending exactly on the max edge is past the zone.
        assert!(!candidate_valid(&w, 500.0, Y, 570.0, t()));
        assert!(candidate_valid(&w, 500.0, Y, 569.0, t()));
    }

    #[test]
    fn the_view_edge_shortens_the_blink_to_stay_24_px_inside() {
        let solids = [floor()];
        let mut w = world(&solids);
        w.view = (100.0, 740.0);
        // The right inset: the box right edge <= 740 - 24, so the centre <= 704.
        assert_eq!(search(&w, 660.0, Y, 1, t()), Some(704.0));
        // The left inset: the box left edge >= 100 + 24, so the centre >= 136.
        assert_eq!(search(&w, 180.0, Y, -1, t()), Some(136.0));
        // Already outside the inset band toward the edge: nothing is valid.
        assert_eq!(search(&w, 710.0, Y, 1, t()), None);
    }

    #[test]
    fn the_world_edges_bound_the_blink() {
        let solids = [floor()];
        let mut w = world(&solids);
        w.x_max = 600.0;
        w.view = (-200.0, 2000.0);
        assert_eq!(search(&w, 560.0, Y, 1, t()), Some(600.0));
        assert_eq!(search(&w, 40.0, Y, -1, t()), Some(12.0));
    }

    #[test]
    fn two_players_keep_the_separation_limit_or_do_not_widen_it() {
        let solids = [floor()];
        let mut w = world(&solids);
        // Carl at 500, limit 448: from 800 (300 away) a 112 px blink right
        // gives 412 away (inside).
        w.partner = Some((500.0, 448.0));
        assert_eq!(search(&w, 800.0, Y, 1, t()), Some(912.0));
        // From 900 (400 away): 112 px right would be 512 away (outside, and
        // wider), so the farthest candidate inside the limit wins.
        assert_eq!(search(&w, 900.0, Y, 1, t()), Some(948.0));
        // Already beyond the limit (a swap, a join): moving closer is allowed,
        // moving away is not.
        assert_eq!(search(&w, 1000.0, Y, -1, t()), Some(888.0));
        assert_eq!(search(&w, 1000.0, Y, 1, t()), None);
    }

    #[test]
    fn a_pit_is_crossed_but_never_entered() {
        // Two floors with a 100 px pit between them: 560..660.
        let left = Block {
            rect: Rect {
                left: 0.0,
                bottom: 0.0,
                width: 560.0,
                height: 48.0,
            },
            material: Material::Stone,
            intact: true,
        };
        let right = Block {
            rect: Rect {
                left: 660.0,
                bottom: 0.0,
                width: 1000.0,
                height: 48.0,
            },
            material: Material::Stone,
            intact: true,
        };
        let solids = [left, right];
        let w = world(&solids);
        // From the edge at 548 the 112 px candidate (660) is on the far floor.
        assert_eq!(search(&w, 548.0, Y, 1, t()), Some(660.0));
        // From 500 every candidate from 112 down to 64 lands over the pit; the
        // best supported one is 560 (the near edge) at 60 px.
        let x = search(&w, 500.0, Y, 1, t()).unwrap();
        assert_eq!(x, 560.0);
        // A pit too wide to cross and a start at its edge: no destination.
        let wide = [
            Block {
                rect: Rect {
                    left: 0.0,
                    bottom: 0.0,
                    width: 560.0,
                    height: 48.0,
                },
                material: Material::Stone,
                intact: true,
            },
            Block {
                rect: Rect {
                    left: 800.0,
                    bottom: 0.0,
                    width: 600.0,
                    height: 48.0,
                },
                material: Material::Stone,
                intact: true,
            },
        ];
        assert_eq!(search(&world(&wide), 560.0, Y, 1, t()), None);
        // Support is only the destination's: a destination more than the probe
        // below her feet has none.
        let low = [Block {
            rect: Rect {
                left: 0.0,
                bottom: -800.0,
                width: 2000.0,
                height: 400.0,
            },
            material: Material::Stone,
            intact: true,
        }];
        assert_eq!(search(&world(&low), 500.0, Y, 1, t()), None);
    }

    #[test]
    fn a_hazard_rectangle_is_never_a_destination_but_may_be_crossed() {
        let solids = [floor()];
        let mut w = world(&solids);
        let hazard = [Rect {
            left: 590.0,
            bottom: 48.0,
            width: 40.0,
            height: 20.0,
        }];
        w.hazards = &hazard;
        // 112 px (612) lands inside 590..630: rejected; the best is 580 (box
        // 568..592 still touches 590), then 576 (564..588) clears it.
        assert_eq!(search(&w, 500.0, Y, 1, t()), Some(576.0));
        // A hazard that is all behind the destination does not matter (it is
        // crossed, not entered).
        let behind = [Rect {
            left: 530.0,
            bottom: 48.0,
            width: 20.0,
            height: 20.0,
        }];
        w.hazards = &behind;
        assert_eq!(search(&w, 500.0, Y, 1, t()), Some(612.0));
    }

    #[test]
    fn the_arrival_resolver_checks_mana_before_it_searches() {
        let mut searched = false;
        let out = resolve_arrival(1, 2, || {
            searched = true;
            Some(10.0)
        });
        assert_eq!(out, Arrival::Fizzled(FizzleReason::NoMana));
        assert!(!searched, "the mana check runs before the search");
        assert_eq!(
            resolve_arrival(2, 2, || Some(10.0)),
            Arrival::Resolved(10.0)
        );
        assert_eq!(
            resolve_arrival(8, 2, || None),
            Arrival::Fizzled(FizzleReason::NoDestination)
        );
    }

    #[test]
    fn the_press_gate_runs_busy_cooldown_mana_destination_in_that_order() {
        let ok = PressView {
            can_act: true,
            busy: false,
            charges: 8,
        };
        let mut b = Blink::default();
        assert_eq!(b.press_gate(10, &ok, 2, || Some(1.0)), Ok(()));
        let hurt = PressView {
            can_act: false,
            ..ok
        };
        assert_eq!(
            b.press_gate(10, &hurt, 2, || panic!("never searched")),
            Err(SpecialReject::Busy)
        );
        let busy = PressView { busy: true, ..ok };
        assert_eq!(
            b.press_gate(10, &busy, 2, || panic!("never searched")),
            Err(SpecialReject::Busy)
        );
        let dry = PressView { charges: 1, ..ok };
        assert_eq!(
            b.press_gate(10, &dry, 2, || panic!("never searched")),
            Err(SpecialReject::NoMana)
        );
        assert_eq!(
            b.press_gate(10, &ok, 2, || None),
            Err(SpecialReject::NoDestination)
        );
        b.arrived(100, t());
        assert_eq!(b.ready_tick(), Some(640));
        // Cooldown beats mana and destination (and Busy beats cooldown).
        assert_eq!(
            b.press_gate(639, &dry, 2, || panic!("never searched")),
            Err(SpecialReject::Cooldown)
        );
        assert_eq!(
            b.press_gate(639, &hurt, 2, || panic!("never searched")),
            Err(SpecialReject::Busy)
        );
        // The first tick out: ready on +540, not on +539.
        assert!(!b.take_ready(639));
        assert!(b.take_ready(640));
        assert!(!b.take_ready(641), "reported once");
        assert_eq!(b.press_gate(640, &ok, 2, || Some(1.0)), Ok(()));
    }

    #[test]
    fn the_timeline_is_windup_t0_to_t5_arrival_t6_recovery_t7_to_t12() {
        let mut b = Blink::default();
        assert!(b.is_idle());
        b.begin(50, 1);
        assert_eq!(b.windup(), Some((50, 1)));
        for tick in 50..56 {
            assert!(!b.arrival_due(tick, 6), "t{} is windup", tick - 50);
        }
        assert!(b.arrival_due(56, 6), "t6 is the arrival");
        assert!(!b.arrival_due(57, 6));
        b.arrived(56, t());
        assert!(b.windup().is_none());
        for tick in 57..=62 {
            b.tick_over(tick);
            assert!(b.recovering(tick), "t{} is recovery", tick - 50);
        }
        b.tick_over(63);
        assert!(!b.recovering(63), "t13 is free");
        assert!(b.cooling(56 + 539) && !b.cooling(56 + 540));
    }

    #[test]
    fn a_cancel_and_a_fizzle_cost_nothing() {
        let mut b = Blink::default();
        b.begin(5, -1);
        assert!(b.cancel());
        assert!(b.is_idle(), "no cooldown after a cancel");
        assert!(!b.cancel());
        b.begin(5, -1);
        b.fizzled();
        assert!(b.is_idle(), "no cooldown, no recovery after a fizzle");
    }

    #[test]
    fn an_idle_blink_mixes_nothing_and_a_busy_one_does() {
        let hash = |b: &Blink| {
            let mut v = Vec::new();
            b.mix_into(&mut |x| v.push(x));
            v
        };
        assert!(hash(&Blink::default()).is_empty());
        let mut b = Blink::default();
        b.begin(5, 1);
        let windup = hash(&b);
        assert!(!windup.is_empty());
        b.arrived(11, t());
        let recovering = hash(&b);
        assert_ne!(windup, recovering);
        b.tick_over(100);
        let cooling = hash(&b);
        assert!(!cooling.is_empty(), "a cooldown is state");
        assert!(b.take_ready(551));
        assert!(hash(&b).is_empty(), "the hash returns to the idle value");
    }

    #[test]
    fn the_reason_and_cause_names_and_codes_are_distinct() {
        let mut names: Vec<&str> = FizzleReason::ALL.iter().map(|r| r.name()).collect();
        names.extend(CancelCause::ALL.iter().map(|c| c.name()));
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count);
        let codes: Vec<u64> = FizzleReason::ALL.iter().map(|r| r.code()).collect();
        assert_eq!(codes, [0, 1]);
        let codes: Vec<u64> = CancelCause::ALL.iter().map(|c| c.code()).collect();
        assert_eq!(codes, [0, 1, 2]);
    }
}
