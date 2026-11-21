//! COMPANION-AI-01 stage 01B-wire: the nav-driven follow executor and the
//! camera-safe regroup (CA-08, CA-11), pure and Bevy-free.
//!
//! [`NavDriver`] wraps the 01B-core [`NavGraph`] of the live level and one small
//! [`HeroNav`] per hero. Each tick, for a companion-driven hero that is free to
//! move, [`NavDriver::follow`] answers one of three things:
//!
//! * `None`: the nav has nothing to add. The caller keeps the follow-and-jump
//!   bot's input, which is today's behaviour on flat ground (the hero stands on
//!   the leader's anchor, or the whole route to it is walking, or one of the two
//!   is not standing on a known anchor).
//! * a [`Drive`] along a route that needs a jump or a drop: reach the edge's
//!   `launch_x` and stop on it (the nav model replays a transit from rest at
//!   that x, [`crate::companion::nav::EdgeView::launch_x`]), jump there, then
//!   steer to the edge's target x, staying beside a ledge until the feet are
//!   above it, exactly the steering of the nav replay test through the real
//!   `step_hero`. A transit is committed (so the plan is not re-decided in the
//!   air) and abandoned when its edge closes (a destruction) or it times out.
//! * a hold ([`Drive`] with no movement): no route to the leader exists (a
//!   ledge the hero cannot reach, a destroyed bridge, a capability gap), so the
//!   hero waits at the safe anchor the graph names, inside the camera bounds,
//!   and the reason is reported.
//!
//! [`NavDriver::regroup`] replaces the legacy "off-screen for N ticks, place the
//! hero at the view edge on the leader's foot line" rule for a companion-driven
//! hero. It places the hero only on an anchor it could walk, jump or drop to on
//! its own (so a mandatory gate or a destroyed bridge is never bypassed), from
//! which it can still reach the leader, just outside the view on the side
//! behind the leader and nearest the view edge (never a pop-in inside the
//! camera, never ahead of the leader), and otherwise says why it did not.
//!
//! Determinism: no randomness, no `HashMap`; the nav graph's tie breaks are
//! stable. [`HeroNav::hash_into`] mixes only the committed transit and the jump
//! press, which are state of the simulation (the graph itself is derived from
//! the solids, which are hashed already).

use crate::companion::nav::{
    EdgeKind, EdgePlan, NavAnchorId, NavConfig, NavError, NavGraph, NavLevel, Plan, Reason,
    SolidId, UNSAFE_RISK, Unlocks,
};
use crate::party::HeroId;
use crate::prototype::{Block, HERO_HALF_WIDTH, RUN_SPEED, TICK_SECONDS};

/// The closest a hero can steer to a target x: the stick dead zone leaves up
/// to `STICK_DEADZONE * RUN_SPEED * TICK_SECONDS` (0.55 px) unspent.
pub const ARRIVE_EPS_PX: f32 = 0.6;
/// A hero standing this close to an edge's takeoff x counts as at rest on it.
const LAND_EPS_PX: f32 = 0.6;
/// Standing inset from an anchor end for a regroup placement.
const PLACE_INSET_PX: f32 = 2.0;
/// Regroup candidates whose distance from the view edge falls in the same
/// bucket of this width tie, and the height closest to the leader's decides.
const REGROUP_EDGE_BUCKET_PX: f32 = 16.0;
/// A waiting hero stands this far from the leader's x on its own anchor (the
/// bot's follow distance), with the bot's slack before it moves again.
const WAIT_BAND_PX: f32 = 40.0;
const FOLLOW_SLACK_PX: f32 = 8.0;
/// A committed transit is dropped after this many ticks beyond its ETA times
/// three (a hero stuck behind something the model does not know).
const TRANSIT_SLACK_TICKS: u16 = 120;

/// The regroup ranking: edge bucket, height gap, exact edge gap.
type RegroupKey = (u64, u64, u64);

/// Where a body is, as the executor sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body {
    pub x: f32,
    pub foot_y: f32,
    pub grounded: bool,
}

/// One tick of movement the executor wants.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Drive {
    pub move_x: f32,
    pub jump: bool,
}

/// A fact the executor reports once, in tick order (the scene logs them as
/// `CompanionEvent`s).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavNote {
    /// A transit `from -> to` of `kind` was committed.
    Begin {
        from: NavAnchorId,
        to: NavAnchorId,
        kind: EdgeKind,
    },
    /// The hero stands on the destination of its committed transit.
    Arrived { anchor: NavAnchorId },
    /// The transit was dropped (its edge closed, it timed out or the landing
    /// missed).
    Abandoned,
    /// No route to the leader: the hero waits at `anchor` (once per anchor).
    Waiting { anchor: NavAnchorId, reason: Reason },
    /// No safe anchor is reachable at all.
    Stranded { reason: Reason },
}

/// A committed jump, drop or walk between two anchors.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Transit {
    from: NavAnchorId,
    to: NavAnchorId,
    kind: EdgeKind,
    /// Takeoff x (jump and drop edges), 0 for a walk.
    launch_x: f32,
    target_x: f32,
    /// Destination span and standing height.
    dst: (f32, f32, f32),
    /// The hero is past takeoff: it jumped, or walks off the edge.
    go: bool,
    /// The hero has been seen in the air on this transit.
    air: bool,
    age: u16,
    /// Ticks after which the transit is abandoned.
    budget: u16,
}

/// Per-hero executor state.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HeroNav {
    transit: Option<Transit>,
    /// The jump press went out last tick (a held jump never repeats, so the
    /// next tick releases it).
    pressed_last: bool,
    /// The anchor the hero last reported waiting at (a trace guard, not
    /// simulation state: never hashed).
    waiting: Option<NavAnchorId>,
    /// The reason of the last `Stranded` note (the edge guard: a stranded hero
    /// reports once per change, not once per tick; trace only, never hashed).
    stranded: Option<Reason>,
}

impl HeroNav {
    /// Is any simulation state live (so the hash must carry it)?
    pub fn is_active(&self) -> bool {
        self.transit.is_some() || self.pressed_last
    }

    /// The committed transit's anchors, if any.
    pub fn transit_anchors(&self) -> Option<(NavAnchorId, NavAnchorId)> {
        self.transit.map(|t| (t.from, t.to))
    }

    /// The anchor the hero is waiting at because no route to the leader exists.
    pub fn waiting_at(&self) -> Option<NavAnchorId> {
        self.waiting
    }

    /// Mix the live state (call only while [`Self::is_active`]).
    pub fn hash_into(&self, mix: &mut impl FnMut(u64)) {
        mix(u64::from(self.pressed_last));
        match self.transit {
            None => mix(0),
            Some(t) => {
                mix(1);
                for id in [t.from, t.to] {
                    mix(u64::from(id.solid.0));
                    mix(u64::from(id.slot));
                }
                mix(t.kind as u64);
                mix(u64::from(t.launch_x.to_bits()));
                mix(u64::from(t.target_x.to_bits()));
                mix(u64::from(t.go) | (u64::from(t.air) << 1));
                mix(u64::from(t.age));
            }
        }
    }
}

/// The camera facts a regroup needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegroupView {
    pub view_left: f32,
    pub view_right: f32,
    /// -1 when "behind the leader" is to the left, +1 to the right.
    pub behind: f32,
}

/// What a regroup decided.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Regroup {
    /// Place the hero on `anchor`, centred at `x`, feet at `foot_y`.
    Place {
        anchor: NavAnchorId,
        x: f32,
        foot_y: f32,
    },
    /// The hero is not on the graph (it stands on no known open anchor): the
    /// scene's legacy rule is the escape hatch.
    Legacy,
    /// No safe, reachable, off-screen anchor exists (the camera warning):
    /// the hero stays where it is and the reason is traced.
    Denied(Reason),
    /// The leader is in the air over no known anchor, so whether it stays
    /// reachable cannot be checked this tick: do nothing and ask again next
    /// tick (the off-screen counter keeps running).
    Defer,
}

/// The nav graph of a live level plus the executor state of both heroes.
#[derive(Debug, Clone)]
pub struct NavDriver {
    graph: NavGraph,
    pub heroes: [HeroNav; 2],
}

impl NavDriver {
    /// Build the graph from the scene's solids under the integrated physics
    /// (`NavConfig::integrated`, the config 01B-wire uses).
    pub fn new(solids: &[Block]) -> Result<Self, NavError> {
        Ok(Self {
            graph: NavGraph::build(
                &NavLevel::from_blocks(solids, Vec::new()),
                NavConfig::integrated(),
            )?,
            heroes: [HeroNav::default(); 2],
        })
    }

    pub fn graph(&self) -> &NavGraph {
        &self.graph
    }

    /// Bring the graph in line with the scene's solids (a destruction or a
    /// restoration). Only the affected edges are re-evaluated; a change the
    /// graph cannot take incrementally (a different solid count, a flip of an
    /// indestructible solid) rebuilds it, keeping the executor state.
    pub fn sync(&mut self, solids: &[Block]) -> Result<(), NavError> {
        let mut rebuild = solids.len() != self.graph.solid_count();
        if !rebuild {
            for (i, b) in solids.iter().enumerate() {
                let id = SolidId(i as u32);
                if self.graph.solid_intact(id) != Some(b.intact)
                    && self.graph.set_solid_intact(id, b.intact).is_err()
                {
                    rebuild = true;
                    break;
                }
            }
        }
        if rebuild {
            self.graph = NavGraph::build(
                &NavLevel::from_blocks(solids, Vec::new()),
                NavConfig::integrated(),
            )?;
        }
        Ok(())
    }

    fn anchor_of(&self, b: Body) -> Option<NavAnchorId> {
        b.grounded
            .then(|| self.graph.anchor_at(b.x, b.foot_y))
            .flatten()
    }

    /// The executor's tick for `actor`: see the module docs. `bounds` is the x
    /// range a wait anchor must lie in (the camera, so a hero never waits out of
    /// view when it can wait inside it).
    pub fn follow(
        &mut self,
        actor: HeroId,
        me: Body,
        leader: Body,
        bounds: Option<(f32, f32)>,
    ) -> (Option<Drive>, Vec<NavNote>) {
        let i = actor.index();
        let mut notes = Vec::new();
        if let Some(t) = self.heroes[i].transit
            && self.graph.edge_plan(t.from, t.to, actor).is_none()
        {
            // The edge closed (a destruction) or never was: re-plan from here.
            self.heroes[i].transit = None;
            self.heroes[i].pressed_last = false;
            notes.push(NavNote::Abandoned);
        }
        if let Some(drive) = self.continue_transit(actor, me, &mut notes) {
            return (Some(drive), notes);
        }
        let drive = self.plan(actor, me, leader, bounds, &mut notes);
        (drive, notes)
    }

    /// Advance the committed transit one tick. `None` when there is none (or it
    /// just ended: the caller plans afresh).
    fn continue_transit(
        &mut self,
        actor: HeroId,
        me: Body,
        notes: &mut Vec<NavNote>,
    ) -> Option<Drive> {
        let i = actor.index();
        let mut t = self.heroes[i].transit?;
        t.age = t.age.saturating_add(1);
        let here = self.anchor_of(me);
        if me.grounded && here == Some(t.to) {
            self.heroes[i].transit = None;
            self.heroes[i].pressed_last = false;
            notes.push(NavNote::Arrived { anchor: t.to });
            return None;
        }
        // Timed out, or landed somewhere other than the destination.
        if t.age > t.budget || (me.grounded && t.air) {
            self.heroes[i].transit = None;
            self.heroes[i].pressed_last = false;
            notes.push(NavNote::Abandoned);
            return None;
        }
        let mut jump = false;
        let drive = if !me.grounded {
            t.air = true;
            Drive {
                move_x: steer(me, &t),
                jump: false,
            }
        } else if t.kind == EdgeKind::Walk {
            Drive {
                move_x: steer(me, &t),
                jump: false,
            }
        } else {
            // A jump press that did not take (the director dropped it, or the
            // hero was not free) leaves the hero grounded after takeoff: go back
            // to the approach and press again after a release tick.
            if t.go && t.kind == EdgeKind::Jump {
                t.go = false;
            }
            if t.go {
                Drive {
                    move_x: steer(me, &t),
                    jump: false,
                }
            } else if (t.launch_x - me.x).abs() > ARRIVE_EPS_PX {
                Drive {
                    move_x: approach(me.x, t.launch_x),
                    jump: false,
                }
            } else if t.kind == EdgeKind::Jump && self.pressed(i) {
                // The release tick: a held jump never repeats.
                Drive::default()
            } else {
                t.go = true;
                jump = t.kind == EdgeKind::Jump;
                Drive {
                    move_x: steer(me, &t),
                    jump,
                }
            }
        };
        self.heroes[i].pressed_last = jump;
        self.heroes[i].transit = Some(t);
        Some(drive)
    }

    fn pressed(&self, i: usize) -> bool {
        self.heroes[i].pressed_last
    }

    fn plan(
        &mut self,
        actor: HeroId,
        me: Body,
        leader: Body,
        bounds: Option<(f32, f32)>,
        notes: &mut Vec<NavNote>,
    ) -> Option<Drive> {
        let i = actor.index();
        let (Some(here), Some(lead)) = (self.anchor_of(me), self.anchor_of(leader)) else {
            self.heroes[i].waiting = None;
            self.heroes[i].stranded = None;
            return None;
        };
        if here == lead {
            self.heroes[i].waiting = None;
            self.heroes[i].stranded = None;
            return None;
        }
        match self
            .graph
            .plan_bounded(actor, here, lead, Unlocks::NONE, bounds)
        {
            Plan::Route(path) => {
                self.heroes[i].waiting = None;
                self.heroes[i].stranded = None;
                let needs_nav = path.anchors.windows(2).any(|w| {
                    self.graph
                        .edge_plan(w[0], w[1], actor)
                        .is_some_and(|p| p.kind != EdgeKind::Walk)
                });
                if !needs_nav || path.anchors.len() < 2 {
                    return None;
                }
                self.begin(actor, here, path.anchors[1], me, notes)
            }
            Plan::Wait { anchor, reason } => {
                self.heroes[i].stranded = None;
                if self.heroes[i].waiting != Some(anchor) {
                    self.heroes[i].waiting = Some(anchor);
                    notes.push(NavNote::Waiting { anchor, reason });
                }
                if anchor == here {
                    return Some(self.hold_near(here, me, leader));
                }
                let path = self
                    .graph
                    .path_or_reason(actor, here, anchor, Unlocks::NONE)
                    .ok()?;
                self.begin(actor, here, *path.anchors.get(1)?, me, notes)
            }
            Plan::Stranded(reason) => {
                // Edge: one note per change of the stranded state, never one per
                // tick (the event log is bounded).
                let was_waiting = self.heroes[i].waiting.take().is_some();
                if (was_waiting || reason != Reason::Unreachable)
                    && self.heroes[i].stranded != Some(reason)
                {
                    notes.push(NavNote::Stranded { reason });
                }
                self.heroes[i].stranded = Some(reason);
                None
            }
        }
    }

    /// Waiting on `here` for a route that does not exist: close in on the
    /// leader's x as far as the anchor goes (the follow band of the bot), and
    /// never leave the anchor.
    fn hold_near(&self, here: NavAnchorId, me: Body, leader: Body) -> Drive {
        let Some((x0, x1)) = self.graph.anchor_span(here) else {
            return Drive::default();
        };
        let goal = leader.x.clamp(x0 + PLACE_INSET_PX, x1 - PLACE_INSET_PX);
        let gap = goal - me.x;
        if gap.abs() <= WAIT_BAND_PX + FOLLOW_SLACK_PX {
            return Drive::default();
        }
        Drive {
            move_x: approach(me.x, goal - gap.signum() * WAIT_BAND_PX),
            jump: false,
        }
    }

    /// Commit the edge `from -> to` and take its first tick.
    fn begin(
        &mut self,
        actor: HeroId,
        from: NavAnchorId,
        to: NavAnchorId,
        me: Body,
        notes: &mut Vec<NavNote>,
    ) -> Option<Drive> {
        let plan: EdgePlan = self.graph.edge_plan(from, to, actor)?;
        let (x0, x1) = self.graph.anchor_span(to)?;
        let (mid, top) = self.graph.anchor_pos(to)?;
        let (launch_x, target_x) = match (plan.kind, plan.launch_x) {
            (EdgeKind::Walk, _) | (_, None) => (me.x, mid),
            (_, Some((lx, tx))) => (lx, tx),
        };
        let kind = if plan.launch_x.is_none() {
            EdgeKind::Walk
        } else {
            plan.kind
        };
        self.heroes[actor.index()].transit = Some(Transit {
            from,
            to,
            kind,
            launch_x,
            target_x,
            dst: (x0, x1, top),
            go: kind == EdgeKind::Walk,
            air: false,
            age: 0,
            budget: plan
                .ticks
                .saturating_mul(3)
                .saturating_add(TRANSIT_SLACK_TICKS),
        });
        notes.push(NavNote::Begin { from, to, kind });
        self.continue_transit(actor, me, notes)
    }

    /// The highest open anchor under a body in the air (its x within the span,
    /// the surface not above the feet).
    fn anchor_below(&self, b: Body) -> Option<NavAnchorId> {
        let mut best: Option<(f32, NavAnchorId)> = None;
        for a in self.graph.anchors() {
            let (Some((x0, x1)), Some((_, top))) =
                (self.graph.anchor_span(a), self.graph.anchor_pos(a))
            else {
                continue;
            };
            if !self.graph.anchor_open(a) || b.x < x0 || b.x > x1 || top > b.foot_y + 4.0 {
                continue;
            }
            if best.is_none_or(|(t, _)| top > t) {
                best = Some((top, a));
            }
        }
        best.map(|(_, a)| a)
    }

    /// The camera-safe regroup of a hero that has been off-screen too long (see
    /// the module docs). `leader` is the other hero.
    pub fn regroup(&self, actor: HeroId, me: Body, leader: Body, view: &RegroupView) -> Regroup {
        // The body is usually standing: take the anchor under the feet even
        // in a hop, within the anchor tolerance.
        let Some(here) = self.graph.anchor_at(me.x, me.foot_y) else {
            return Regroup::Legacy;
        };
        // The anchor the leader stands on, or, in a jump or a fall, the one
        // under it: the "leader stays reachable" check runs in the air too, so
        // a dead end is never picked on the tick the counter happens to expire
        // mid-jump. Over no anchor in the air nothing can be checked: wait.
        let lead = if leader.grounded {
            self.graph.anchor_at(leader.x, leader.foot_y)
        } else {
            match self.anchor_below(leader) {
                Some(a) => Some(a),
                None => return Regroup::Defer,
            }
        };
        let edge_x = if view.behind < 0.0 {
            view.view_left - HERO_HALF_WIDTH - 2.0
        } else {
            view.view_right + HERO_HALF_WIDTH + 2.0
        };
        let mut best: Option<(RegroupKey, NavAnchorId, f32, f32)> = None;
        for a in self.graph.anchors() {
            let (Some((x0, x1)), Some((_, top)), Some(risk)) = (
                self.graph.anchor_span(a),
                self.graph.anchor_pos(a),
                self.graph.anchor_risk(a),
            ) else {
                continue;
            };
            if !self.graph.anchor_open(a) || risk >= UNSAFE_RISK {
                continue;
            }
            let x = if x1 - x0 <= 2.0 * PLACE_INSET_PX {
                (x0 + x1) * 0.5
            } else {
                edge_x.clamp(x0 + PLACE_INSET_PX, x1 - PLACE_INSET_PX)
            };
            // Only the behind side: outside the view on the side behind the
            // leader (never a pop-in, never ahead of the leader, so no
            // unexplored terrain or encounter is skipped).
            let outside_behind = if view.behind < 0.0 {
                x < view.view_left - HERO_HALF_WIDTH
            } else {
                x > view.view_right + HERO_HALF_WIDTH
            };
            if !outside_behind || (x - leader.x) * view.behind <= 0.0 {
                continue;
            }
            // Reachable on its own legs (no gate or bridge is bypassed), and the
            // leader is still reachable from there (nobody is trapped).
            if a != here && self.graph.can_reach(actor, here, a, Unlocks::NONE).is_err() {
                continue;
            }
            if let Some(l) = lead
                && a != l
                && self.graph.can_reach(actor, a, l, Unlocks::NONE).is_err()
            {
                continue;
            }
            // Nearest to the view edge first (a coarse bucket, so a hero enters
            // the view from where it was cut off), then the closest height to
            // the leader, then the exact distance.
            let edge_gap = (x - edge_x).abs() * 16.0;
            let key = (
                (edge_gap / (REGROUP_EDGE_BUCKET_PX * 16.0)) as u64,
                ((top - leader.foot_y).abs() * 16.0) as u64,
                edge_gap as u64,
            );
            if best.is_none_or(|(k, ..)| key < k) {
                best = Some((key, a, x, top));
            }
        }
        match best {
            Some((_, anchor, x, foot_y)) => Regroup::Place { anchor, x, foot_y },
            None => Regroup::Denied(Reason::Unreachable),
        }
    }
}

/// Steering toward `target_x`, the analog stick of the nav replay: full stick
/// far away, the exact remaining step near it.
fn approach(x: f32, target_x: f32) -> f32 {
    ((target_x - x) / (RUN_SPEED * TICK_SECONDS)).clamp(-1.0, 1.0)
}

/// The transit's steering: toward its `target_x`, but beside the destination
/// ledge while the feet are below its top (moving under the underside would
/// bump the head), exactly as the nav model replays a transit.
fn steer(me: Body, t: &Transit) -> f32 {
    let mut target = t.target_x;
    if t.kind != EdgeKind::Walk && me.foot_y < t.dst.2 - LAND_EPS_PX {
        target = if t.target_x >= t.launch_x {
            target.min(t.dst.0 - HERO_HALF_WIDTH)
        } else {
            target.max(t.dst.1 + HERO_HALF_WIDTH)
        };
    }
    approach(me.x, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{InputFrame, RawInput};
    use crate::prototype::{HERO_HALF_HEIGHT, Hero, Material, Rect, step_hero};

    fn block(left: f32, top: f32, width: f32, height: f32, material: Material) -> Block {
        Block {
            rect: Rect {
                left,
                bottom: top - height,
                width,
                height,
            },
            material,
            intact: true,
        }
    }

    fn stone(left: f32, top: f32, width: f32, height: f32) -> Block {
        block(left, top, width, height, Material::Stone)
    }

    /// The floor (top 0) with a two-step staircase (tops 60 and 120, a 40 px gap
    /// between them) and a 200 px tower that nobody can climb.
    fn level() -> Vec<Block> {
        vec![
            stone(0.0, 0.0, 2000.0, 40.0),
            stone(200.0, 60.0, 60.0, 16.0),
            stone(300.0, 120.0, 60.0, 16.0),
            stone(700.0, 200.0, 60.0, 200.0),
        ]
    }

    fn body(h: &Hero) -> Body {
        Body {
            x: h.x,
            foot_y: h.y - HERO_HALF_HEIGHT,
            grounded: h.grounded,
        }
    }

    fn standing(x: f32, foot: f32) -> Body {
        Body {
            x,
            foot_y: foot,
            grounded: true,
        }
    }

    /// Run the executor against the real hero body for `ticks` fixed ticks: the
    /// hero starts on the floor at `x`, the leader stands at `leader`.
    fn run(
        driver: &mut NavDriver,
        blocks: &[Block],
        x: f32,
        leader: Body,
        ticks: u32,
    ) -> (Hero, Vec<NavNote>, f32) {
        let mut hero = Hero::at(x, HERO_HALF_HEIGHT);
        let mut frame = InputFrame::default();
        let (mut notes, mut highest) = (Vec::new(), 0.0f32);
        for _ in 0..ticks {
            let (drive, n) = driver.follow(HeroId::Carl, body(&hero), leader, None);
            notes.extend(n);
            let d = drive.unwrap_or_default();
            frame.advance(RawInput {
                move_x: d.move_x,
                jump: d.jump,
                ..RawInput::NONE
            });
            step_hero(&mut hero, &frame, blocks, 2000.0);
            highest = highest.max(hero.y - HERO_HALF_HEIGHT);
        }
        (hero, notes, highest)
    }

    #[test]
    fn a_two_hop_route_is_climbed_through_the_real_body_step() {
        let blocks = level();
        let mut driver = NavDriver::new(&blocks).unwrap();
        let (hero, notes, _) = run(&mut driver, &blocks, 100.0, standing(330.0, 120.0), 400);
        assert!((hero.y - HERO_HALF_HEIGHT - 120.0).abs() < 0.01, "{hero:?}");
        assert!(hero.x >= 300.0 && hero.x <= 360.0, "{hero:?}");
        let begins = notes
            .iter()
            .filter(|n| {
                matches!(
                    n,
                    NavNote::Begin {
                        kind: EdgeKind::Jump,
                        ..
                    }
                )
            })
            .count();
        assert!(begins >= 2, "two jumps expected: {notes:?}");
        assert!(notes.iter().any(|n| matches!(n, NavNote::Arrived { .. })));
        assert!(!driver.heroes[0].is_active(), "the transit ended");
    }

    #[test]
    fn a_ledge_it_cannot_reach_is_waited_at_safely() {
        // Floor and tower only: the staircase would be the nearest reachable
        // anchor to the tower and the hero would wait up there.
        let blocks = vec![
            stone(0.0, 0.0, 2000.0, 40.0),
            stone(700.0, 200.0, 60.0, 200.0),
        ];
        let mut driver = NavDriver::new(&blocks).unwrap();
        let (hero, notes, highest) = run(&mut driver, &blocks, 100.0, standing(730.0, 200.0), 300);
        assert!(highest < 0.01, "it never jumped: {highest}");
        assert!(hero.y - HERO_HALF_HEIGHT < 0.01);
        let waits: Vec<_> = notes
            .iter()
            .filter(|n| matches!(n, NavNote::Waiting { .. }))
            .collect();
        assert_eq!(waits.len(), 1, "one report per anchor: {notes:?}");
        // It closed in on the tower's foot, inside the follow band, and holds.
        assert!(
            (hero.x - (700.0 - HERO_HALF_WIDTH - 1.0)).abs() < 48.0,
            "{hero:?}"
        );
        let (drive, _) = driver.follow(HeroId::Carl, body(&hero), standing(730.0, 200.0), None);
        assert_eq!(drive, Some(Drive::default()));
    }

    #[test]
    fn a_destroyed_step_closes_the_route_and_a_restored_one_reopens_it() {
        let mut blocks = level();
        blocks[1].material = Material::Timber;
        let mut driver = NavDriver::new(&blocks).unwrap();
        blocks[1].intact = false;
        driver.sync(&blocks).unwrap();
        let leader = standing(330.0, 120.0);
        let (hero, notes, highest) = run(&mut driver, &blocks, 100.0, leader, 300);
        assert!(highest < 0.01 && hero.y - HERO_HALF_HEIGHT < 0.01);
        assert!(notes.iter().any(|n| matches!(n, NavNote::Waiting { .. })));
        blocks[1].intact = true;
        driver.sync(&blocks).unwrap();
        let (hero, _, _) = run(&mut driver, &blocks, 100.0, leader, 400);
        assert!((hero.y - HERO_HALF_HEIGHT - 120.0).abs() < 0.01, "{hero:?}");
    }

    #[test]
    fn a_transit_in_progress_is_dropped_when_its_edge_closes() {
        let mut blocks = level();
        blocks[1].material = Material::Timber;
        let mut driver = NavDriver::new(&blocks).unwrap();
        let mut hero = Hero::at(100.0, HERO_HALF_HEIGHT);
        let mut frame = InputFrame::default();
        let leader = standing(330.0, 120.0);
        let mut abandoned = false;
        for tick in 0..200 {
            if tick == 20 {
                blocks[1].intact = false;
                driver.sync(&blocks).unwrap();
            }
            let (drive, n) = driver.follow(HeroId::Carl, body(&hero), leader, None);
            abandoned |= n.contains(&NavNote::Abandoned);
            let d = drive.unwrap_or_default();
            frame.advance(RawInput {
                move_x: d.move_x,
                jump: d.jump,
                ..RawInput::NONE
            });
            step_hero(&mut hero, &frame, &blocks, 2000.0);
        }
        assert!(abandoned);
        assert!(
            hero.y - HERO_HALF_HEIGHT < 0.01,
            "it did not climb: {hero:?}"
        );
    }

    #[test]
    fn flat_ground_leaves_the_follow_layer_to_the_bot() {
        let blocks = level();
        let mut driver = NavDriver::new(&blocks).unwrap();
        let hero = Hero::at(900.0, HERO_HALF_HEIGHT);
        let (drive, notes) = driver.follow(HeroId::Carl, body(&hero), standing(1200.0, 0.0), None);
        assert_eq!((drive, notes.len()), (None, 0));
        // An airborne leader (no anchor) is not planned for either.
        let air = Body {
            grounded: false,
            ..standing(330.0, 150.0)
        };
        assert_eq!(driver.follow(HeroId::Carl, body(&hero), air, None).0, None);
    }

    fn view(left: f32) -> RegroupView {
        RegroupView {
            view_left: left,
            view_right: left + 640.0,
            behind: -1.0,
        }
    }

    #[test]
    fn a_regroup_places_the_hero_on_a_reachable_anchor_outside_the_view() {
        let blocks = level();
        let driver = NavDriver::new(&blocks).unwrap();
        // Off-screen far to the right of a view at 900.
        let r = driver.regroup(
            HeroId::Donut,
            standing(1900.0, 0.0),
            standing(1200.0, 0.0),
            &view(900.0),
        );
        let Regroup::Place { x, foot_y, .. } = r else {
            panic!("{r:?}")
        };
        assert!(x < 900.0 - HERO_HALF_WIDTH, "outside the view: {x}");
        assert_eq!(foot_y, 0.0);
    }

    #[test]
    fn a_regroup_never_crosses_a_gate_the_hero_could_not_cross() {
        // A 200 px wall splits the floor: the hero is on its right, the leader
        // on its left, and the only off-screen anchor behind the leader is on the
        // wrong side of the wall for the hero.
        let mut blocks = level();
        blocks.push(stone(1000.0, 200.0, 20.0, 200.0));
        let driver = NavDriver::new(&blocks).unwrap();
        let r = driver.regroup(
            HeroId::Donut,
            standing(1500.0, 0.0),
            standing(500.0, 0.0),
            &view(400.0),
        );
        assert!(matches!(r, Regroup::Denied(_)), "{r:?}");
        // A hero standing on no anchor is the legacy escape hatch.
        let r = driver.regroup(
            HeroId::Donut,
            Body {
                grounded: false,
                ..standing(1500.0, 500.0)
            },
            standing(500.0, 0.0),
            &view(400.0),
        );
        assert_eq!(r, Regroup::Legacy);
    }

    /// The review probe of B-1: floor 0..3000, the leader on a ledge in view, a
    /// second ledge far ahead of him at the same height. The hero (off-screen
    /// left) must come back behind the leader, never jump ahead of him.
    fn probe_level() -> Vec<Block> {
        vec![
            stone(0.0, 0.0, 3000.0, 40.0),
            stone(1200.0, 60.0, 60.0, 16.0),
            stone(2000.0, 60.0, 60.0, 16.0),
        ]
    }

    #[test]
    fn a_regroup_never_places_the_hero_ahead_of_the_leader() {
        let blocks = probe_level();
        let driver = NavDriver::new(&blocks).unwrap();
        let v = RegroupView {
            view_left: 900.0,
            view_right: 1540.0,
            behind: -1.0,
        };
        let r = driver.regroup(
            HeroId::Donut,
            standing(100.0, 0.0),
            standing(1230.0, 60.0),
            &v,
        );
        let Regroup::Place { x, foot_y, .. } = r else {
            panic!("{r:?}")
        };
        assert!(x < 900.0 - HERO_HALF_WIDTH, "behind the leader: {x}");
        assert_eq!(foot_y, 0.0, "the floor at the view edge, not the far ledge");
        assert!((x - (900.0 - HERO_HALF_WIDTH - 2.0)).abs() < 0.01, "{x}");

        // Mirrored: the leader faces left, behind is the right, the hero is
        // off-screen right of everything but the far ledge ahead of the leader.
        let v = RegroupView {
            view_left: 1460.0,
            view_right: 2100.0,
            behind: 1.0,
        };
        let r = driver.regroup(
            HeroId::Donut,
            standing(2900.0, 0.0),
            standing(1230.0, 60.0),
            &v,
        );
        let Regroup::Place { x, .. } = r else {
            panic!("{r:?}")
        };
        assert!(x > 2100.0 + HERO_HALF_WIDTH, "behind the leader: {x}");
    }

    /// Behind the view there is only a pit bottom the hero can drop to and never
    /// leave (a dead end); the leader stands on the main floor.
    fn dead_end_level() -> Vec<Block> {
        vec![
            stone(0.0, -200.0, 900.0, 40.0),
            stone(900.0, 0.0, 2100.0, 40.0),
        ]
    }

    #[test]
    fn a_regroup_checks_the_leader_is_reachable_while_he_is_airborne() {
        let blocks = dead_end_level();
        let driver = NavDriver::new(&blocks).unwrap();
        let v = RegroupView {
            view_left: 900.0,
            view_right: 1540.0,
            behind: -1.0,
        };
        let hero = standing(2800.0, 0.0);
        // Grounded on the main floor: the pit is a dead end, so it is refused.
        let r = driver.regroup(HeroId::Donut, hero, standing(1400.0, 0.0), &v);
        assert!(matches!(r, Regroup::Denied(_)), "{r:?}");
        // The same leader in mid-jump: the anchor under him is used.
        let jumping = Body {
            grounded: false,
            ..standing(1400.0, 70.0)
        };
        let r = driver.regroup(HeroId::Donut, hero, jumping, &v);
        assert!(matches!(r, Regroup::Denied(_)), "{r:?}");
        // In the air over no anchor nothing can be judged: ask again next tick.
        let over_void = Body {
            grounded: false,
            ..standing(5000.0, 70.0)
        };
        let r = driver.regroup(HeroId::Donut, hero, over_void, &v);
        assert_eq!(r, Regroup::Defer);
    }

    #[test]
    fn a_stranded_hero_reports_once_not_every_tick() {
        // The route to the leader existed as built and a destruction closed it
        // (reason Destroyed); the camera bounds hold no safe anchor, so the hero
        // is stranded on every tick.
        let mut blocks = level();
        blocks[1].material = Material::Timber;
        let mut driver = NavDriver::new(&blocks).unwrap();
        blocks[1].intact = false;
        driver.sync(&blocks).unwrap();
        let hero = Hero::at(100.0, HERO_HALF_HEIGHT);
        let leader = standing(330.0, 120.0);
        let count = |n: &[NavNote]| {
            n.iter()
                .filter(|n| matches!(n, NavNote::Stranded { .. }))
                .count()
        };
        let far = Some((5000.0, 5100.0));
        let mut stranded = 0;
        for _ in 0..50 {
            let (drive, n) = driver.follow(HeroId::Carl, body(&hero), leader, far);
            assert_eq!(drive, None);
            stranded += count(&n);
        }
        assert_eq!(stranded, 1, "one edge, not one note per tick");
        assert!(driver.heroes[0].stranded.is_some());
    }
}
