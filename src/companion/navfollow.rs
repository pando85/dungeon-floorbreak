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
//! [`NavDriver::go_to`] (COMPANION-AI-02, CA2-B1) is the same executor steered
//! to a *spot* instead of to the leader: the same committed transits, closure
//! handling and holds, but a hold closes on the spot itself (no 40 px follow
//! band). [`NavSpots`] gives the autonomy layer the graph's standing surfaces
//! and the cost of a route ([`crate::companion::autonomy::SpotWorld`]).
//!
//! **Hazard input** (COMPANION-AI-02 CA2-C, K13, CA2-Q29): [`NavDriver::set_hazards`]
//! takes the rectangles that can hurt a hero this tick (the hazard core's
//! telegraphed and active volumes, converted by [`hazards_from_feed`]) and
//! rebuilds the graph only when that list changes. The graph already treats a
//! hazard as unsafe ground: an anchor under it is cut out and carries
//! [`RISK_MAX`](crate::companion::nav::RISK_MAX), a transit through it is
//! unsafe, a route never enters it and a hero standing in it is sent to safe
//! ground, so every consumer (the follow executor, `go_to`, [`NavSpots`], the
//! regroup) avoids it with no code that knows what a hazard is. Nothing here
//! reads a hazard's class: a steam lane, a footprint, an acid patch and a wheel
//! are rectangles. [`fence_step`] is the last guard for the movement that does
//! not go through the graph (an interrupt's approach, a dodge) and
//! [`NavDriver::evacuation`] walks a hero out of a hazard it stands in.
//!
//! Determinism: no randomness, no `HashMap`; the nav graph's tie breaks are
//! stable. [`HeroNav::hash_into`] mixes only the committed transit and the jump
//! press, which are state of the simulation (the graph itself is derived from
//! the solids, which are hashed already).

use crate::companion::autonomy::{SpotWorld, Surface};
use crate::companion::nav::{
    EdgeKind, EdgePlan, NavAnchorId, NavConfig, NavError, NavGraph, NavHazard, NavLevel, Plan,
    Reason, SafeContext, SolidId, UNSAFE_RISK, Unlocks, walk_ticks,
};
use crate::companion::view::HeroView;
use crate::env_hazard::NavRect;
use crate::input::STICK_DEADZONE;
use crate::party::HeroId;
use crate::prototype::{Block, HERO_HALF_HEIGHT, HERO_HALF_WIDTH, RUN_SPEED, Rect, TICK_SECONDS};

/// Carl's arrive tolerance, the floor of [`arrive_eps_px`]. The closest a hero
/// can steer to a target x: the stick dead zone leaves up to
/// `STICK_DEADZONE * run * TICK_SECONDS` unspent (0.555 px at Carl's 185 px/s),
/// and this value keeps 0.045 px of headroom over it.
pub const ARRIVE_EPS_PX: f32 = 0.6;

/// The arrive tolerance of a body that runs `run` px/s (DM-A, N1). It must stay
/// above what the stick dead zone can leave unspent, `STICK_DEADZONE * run *
/// TICK_SECONDS`, or a residual just above it is zeroed by [`approach`], the
/// hero stands still and the transit stalls in a deterministic loop. The
/// headroom of Carl's value is kept (0.045 px): Carl's tolerance is exactly
/// [`ARRIVE_EPS_PX`] (bit-identical), Donut's 207 px/s gives 0.666 px against a
/// 0.621 px leftover.
pub fn arrive_eps_px(run: f32) -> f32 {
    ARRIVE_EPS_PX + (run - RUN_SPEED).max(0.0) * STICK_DEADZONE * TICK_SECONDS
}
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
/// CA2-Q29 (hazard feed): how close, in px, a voluntary ground step may bring
/// her body to a hazard rectangle she is not in ([`fence_step`]). A hazard's
/// contact test is on integer boxes and the nav rounds nothing, so 2 px keeps
/// her clear of the rounding.
pub const HAZARD_FENCE_MARGIN_PX: f32 = 2.0;

/// What a hold waits for: the leader (with the follow band) or a spot (on it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Wait {
    Follow,
    Spot,
}

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
    /// The leader is in the air (a jump or a fall): the surface under him is
    /// not where he lands, so whether the hero stays able to reach him cannot
    /// be judged. Do nothing and hold the off-screen counter at its due value
    /// (`SceneSim::regroup_hold`), so the regroup is asked again on the very
    /// next tick and is placed on the first tick the leader stands. Bounded by
    /// the leader's airtime (a landing, a respawn or the hero walking into the
    /// view ends it).
    Defer,
}

/// The nav graph of a live level plus the executor state of both heroes.
#[derive(Debug, Clone)]
pub struct NavDriver {
    graph: NavGraph,
    /// COMPANION-AI-02 CA2-C: the hero-hurting hazard rectangles the graph is
    /// built with (y up, sorted by id). Empty unless the wiring feeds them
    /// ([`Self::set_hazards`]), which only the autonomous partner's set does.
    hazards: Vec<NavHazard>,
    /// The physics the graph is built under (rebuilds keep it).
    config: NavConfig,
    /// COMPANION-AI-02 CA2-B2 (Revision 3, DP-Q38): no Jump edge is planned from
    /// an anchor whose top is above this y (the apex-aware rise limit: a full
    /// jump from there would put her head above the view). `None`: no limit (the
    /// follow layer and every other set). A control flag set each tick by the
    /// wiring, never hashed.
    jump_ceiling_y: Option<f32>,
    pub heroes: [HeroNav; 2],
}

impl NavDriver {
    /// Build the graph from the scene's solids under `config`: the preset that
    /// matches the party's Donut motion (`NavConfig::for_motion`, which is
    /// `NavConfig::agile()` under the shipped tuning).
    pub fn new(solids: &[Block], config: NavConfig) -> Result<Self, NavError> {
        Ok(Self {
            graph: NavGraph::build(&NavLevel::from_blocks(solids, Vec::new()), config)?,
            hazards: Vec::new(),
            config,
            jump_ceiling_y: None,
            heroes: [HeroNav::default(); 2],
        })
    }

    /// Set (or clear) the highest anchor top a Jump edge may start from (see
    /// the field). Drop and walk edges are never limited.
    pub fn set_jump_ceiling(&mut self, y: Option<f32>) {
        self.jump_ceiling_y = y;
    }

    /// Whether the edge `from -> to` of `kind` may be planned under the jump
    /// ceiling.
    fn edge_allowed(&self, from: NavAnchorId, kind: EdgeKind) -> bool {
        kind != EdgeKind::Jump
            || self.jump_ceiling_y.is_none_or(|limit| {
                self.graph
                    .anchor_pos(from)
                    .is_none_or(|(_, top)| top <= limit)
            })
    }

    pub fn graph(&self) -> &NavGraph {
        &self.graph
    }

    /// The physics the graph is built under.
    pub fn config(&self) -> NavConfig {
        self.config
    }

    /// DONUT-PROG-01 DP-B: bring the graph in line with the physics Donut has
    /// **now** (a grant or a retry changed her launch): a changed `config`
    /// rebuilds the graph from `solids` and keeps the executor state (a
    /// committed transit that no longer plans is dropped by the next tick's
    /// re-plan, exactly as a closed edge is). The same `config` is a no-op, so a
    /// run that never holds the boost never rebuilds.
    pub fn set_config(&mut self, config: NavConfig, solids: &[Block]) -> Result<(), NavError> {
        if config == self.config {
            return Ok(());
        }
        self.graph = NavGraph::build(&NavLevel::from_blocks(solids, self.hazards.clone()), config)?;
        self.config = config;
        Ok(())
    }

    /// COMPANION-AI-02 CA2-C (K13): the hero-hurting hazard rectangles of this
    /// tick. A list equal to the one the graph was built with is a no-op (so a
    /// room without hazards, and every tick between two hazard edges, costs
    /// nothing); a different one rebuilds the graph and keeps the executor state
    /// (a committed transit whose edge no longer plans is dropped by the next
    /// tick's re-plan, exactly as a closed edge is). Returns whether it
    /// rebuilt. A list the graph rejects (a duplicate id) changes nothing.
    pub fn set_hazards(
        &mut self,
        mut hazards: Vec<NavHazard>,
        solids: &[Block],
    ) -> Result<bool, NavError> {
        hazards.sort_by_key(|h| h.id);
        if hazards == self.hazards {
            return Ok(false);
        }
        self.graph = NavGraph::build(&NavLevel::from_blocks(solids, hazards.clone()), self.config)?;
        self.hazards = hazards;
        Ok(true)
    }

    /// The hazard rectangles the graph is built with (y up, sorted by id).
    pub fn hazards(&self) -> &[NavHazard] {
        &self.hazards
    }

    /// Where `actor` should go to get out of a hazard it stands in: the nearest
    /// safe open anchor it can reach and come back from (the graph's
    /// `safe_anchor`), as a spot on that anchor close to its x. `None` when it
    /// is not on a risky anchor (nothing to do), is in the air, or no safe
    /// anchor is reachable. `bounds` is tried first and dropped when it leaves
    /// no candidate: out of the hazard beats inside the camera.
    pub fn evacuation(&self, actor: HeroId, me: Body, bounds: Option<(f32, f32)>) -> Option<Body> {
        let here = self.anchor_of(me)?;
        // On a risky anchor, or with her body over a hazard's edge while her
        // centre is already on safe ground (the anchor ends at the rectangle's
        // edge, her body is 12 px wide).
        let body = Rect {
            left: me.x - HERO_HALF_WIDTH,
            bottom: me.foot_y,
            width: 2.0 * HERO_HALF_WIDTH,
            height: 2.0 * HERO_HALF_HEIGHT,
        };
        if self.graph.anchor_risk(here)? < UNSAFE_RISK
            && !self.hazards.iter().any(|h| h.rect.overlaps(body))
        {
            return None;
        }
        let ask = |bounds, min_width| {
            self.graph
                .safe_anchor_wide(
                    actor,
                    SafeContext {
                        here,
                        goal: None,
                        unlocks: Unlocks::NONE,
                        bounds,
                    },
                    min_width,
                )
                .ok()
        };
        // CA2-FU-HAZ U-2: an anchor too narrow for her body plus the fence margins
        // would leave it on the hazard's edge, so wider anchors come first; only
        // when none exists does she take the nearest one.
        let fit = 2.0 * (HERO_HALF_WIDTH + PLACE_INSET_PX);
        let safe = ask(bounds, fit)
            .or_else(|| bounds.and_then(|_| ask(None, fit)))
            .or_else(|| ask(bounds, 0.0))
            .or_else(|| bounds.and_then(|_| ask(None, 0.0)))?;
        let (x0, x1) = self.graph.anchor_span(safe)?;
        let (_, top) = self.graph.anchor_pos(safe)?;
        let (lo, hi) = self.hold_span(safe, x0, x1);
        let inset = HERO_HALF_WIDTH + PLACE_INSET_PX;
        let x = if lo <= hi && x1 - x0 > 2.0 * inset {
            me.x.clamp(lo, hi)
        } else {
            (x0 + x1) / 2.0
        };
        Some(Body {
            x,
            foot_y: top,
            grounded: true,
        })
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
                &NavLevel::from_blocks(solids, self.hazards.clone()),
                self.config,
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
        self.drive_toward(actor, me, leader, bounds, Wait::Follow)
    }

    /// The executor's tick toward a spot (CA2-B1): `follow` with the goal body
    /// being where she wants to stand. `None`: the spot is on her own anchor or
    /// the whole route is walking, so the caller steers flat (the `approach`
    /// helper); a [`Drive`] is a committed transit or a hold that closes on the
    /// spot, with no wait band.
    pub fn go_to(
        &mut self,
        actor: HeroId,
        me: Body,
        spot: Body,
        bounds: Option<(f32, f32)>,
    ) -> (Option<Drive>, Vec<NavNote>) {
        self.drive_toward(actor, me, spot, bounds, Wait::Spot)
    }

    fn drive_toward(
        &mut self,
        actor: HeroId,
        me: Body,
        goal: Body,
        bounds: Option<(f32, f32)>,
        wait: Wait,
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
        let drive = self.plan(actor, me, goal, bounds, wait, &mut notes);
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
        let run = self.config.run_speed(actor);
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
                move_x: steer(me, &t, run),
                jump: false,
            }
        } else if t.kind == EdgeKind::Walk {
            Drive {
                move_x: steer(me, &t, run),
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
                    move_x: steer(me, &t, run),
                    jump: false,
                }
            } else if (t.launch_x - me.x).abs() > arrive_eps_px(run) {
                Drive {
                    move_x: approach(me.x, t.launch_x, run),
                    jump: false,
                }
            } else if t.kind == EdgeKind::Jump && self.pressed(i) {
                // The release tick: a held jump never repeats.
                Drive::default()
            } else {
                t.go = true;
                jump = t.kind == EdgeKind::Jump;
                Drive {
                    move_x: steer(me, &t, run),
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
        wait: Wait,
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
                    return Some(self.hold_near(actor, here, me, leader, wait));
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
    fn hold_near(
        &self,
        actor: HeroId,
        here: NavAnchorId,
        me: Body,
        leader: Body,
        wait: Wait,
    ) -> Drive {
        let Some((x0, x1)) = self.graph.anchor_span(here) else {
            return Drive::default();
        };
        let (lo, hi) = self.hold_span(here, x0, x1);
        let goal = leader.x.clamp(lo.min(hi), hi);
        let gap = goal - me.x;
        if wait == Wait::Spot {
            // A spot is a place to stand on: close on it, no wait band.
            let run = self.config.run_speed(actor);
            return if gap.abs() <= arrive_eps_px(run) {
                Drive::default()
            } else {
                Drive {
                    move_x: approach(me.x, goal, run),
                    jump: false,
                }
            };
        }
        if gap.abs() <= WAIT_BAND_PX + FOLLOW_SLACK_PX {
            return Drive::default();
        }
        Drive {
            move_x: approach(
                me.x,
                goal - gap.signum() * WAIT_BAND_PX,
                self.config.run_speed(actor),
            ),
            jump: false,
        }
    }

    /// The x range she may hold on in the anchor `[x0, x1]`: the anchor inset by
    /// [`PLACE_INSET_PX`], and at an end that borders a hazard (the graph cuts an
    /// anchor at a hazard's edge) the whole body kept clear of it by
    /// [`HAZARD_FENCE_MARGIN_PX`]. Without a hazard this is the old
    /// `[x0 + 2, x1 - 2]`.
    fn hold_span(&self, here: NavAnchorId, x0: f32, x1: f32) -> (f32, f32) {
        let (mut lo, mut hi) = (x0 + PLACE_INSET_PX, x1 - PLACE_INSET_PX);
        let Some((_, top)) = self.graph.anchor_pos(here) else {
            return (lo, hi);
        };
        let clear = HERO_HALF_WIDTH + HAZARD_FENCE_MARGIN_PX;
        for h in &self.hazards {
            let r = h.rect;
            if r.bottom >= top + 2.0 * HERO_HALF_HEIGHT || r.top() <= top {
                continue;
            }
            if (r.left - x1).abs() < 1.0 {
                hi = hi.min(r.left - clear);
            }
            if (r.right() - x0).abs() < 1.0 {
                lo = lo.max(r.right() + clear);
            }
        }
        (lo, hi)
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
        if !self.edge_allowed(from, plan.kind) {
            // Above the apex-aware rise limit: she holds and leaves by a drop.
            return Some(Drive::default());
        }
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

    /// The camera-safe regroup of a hero that has been off-screen too long (see
    /// the module docs). `leader` is the other hero.
    pub fn regroup(&self, actor: HeroId, me: Body, leader: Body, view: &RegroupView) -> Regroup {
        // The body is usually standing: take the anchor under the feet even
        // in a hop, within the anchor tolerance.
        let Some(here) = self.graph.anchor_at(me.x, me.foot_y) else {
            return Regroup::Legacy;
        };
        // A leader in the air has no standing anchor to check against (the
        // surface under him is not where he lands: over a pit it is the pit
        // bottom, a dead end), so nothing is placed until he stands.
        if !leader.grounded {
            return Regroup::Defer;
        }
        let lead = self.graph.anchor_at(leader.x, leader.foot_y);
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

/// The nav graph as the autonomy layer's [`SpotWorld`]: every open anchor is a
/// standing surface, its span inset by the body half width plus the ledge inset
/// (spec 3.5), and a route's cost is the summed duration of its jump and drop
/// edges for `actor`.
pub struct NavSpots<'a> {
    driver: &'a NavDriver,
    actor: HeroId,
    ledge_inset_px: f32,
    /// `ca2_q36_transit_slack_ticks`: per jump or drop edge (0 by default).
    transit_slack_ticks: u32,
}

impl<'a> NavSpots<'a> {
    /// `ledge_inset_px` is `ca2_q29_ledge_inset_px`.
    pub fn new(driver: &'a NavDriver, actor: HeroId, ledge_inset_px: f32) -> Self {
        Self {
            driver,
            actor,
            ledge_inset_px,
            transit_slack_ticks: 0,
        }
    }

    /// CA2-Q36: the per-edge slack of the route price.
    #[must_use]
    pub fn with_transit_slack(mut self, ticks: u16) -> Self {
        self.transit_slack_ticks = u32::from(ticks);
        self
    }

    fn key(id: NavAnchorId) -> u64 {
        (u64::from(id.solid.0) << 16) | u64::from(id.slot)
    }

    /// The open anchor directly under `x`: the highest top at or just below
    /// `foot_y` (ties by the lowest id, the graph's order).
    fn anchor_below(graph: &NavGraph, x: f32, foot_y: f32) -> Option<NavAnchorId> {
        let mut best: Option<(f32, NavAnchorId)> = None;
        for a in graph.anchors() {
            let (Some((x0, x1)), Some((_, top))) = (graph.anchor_span(a), graph.anchor_pos(a))
            else {
                continue;
            };
            if graph.anchor_open(a)
                && x >= x0
                && x <= x1
                && top <= foot_y + 4.0
                && best.is_none_or(|(t, _)| top > t)
            {
                best = Some((top, a));
            }
        }
        best.map(|(_, a)| a)
    }

    fn anchor_of_key(key: u64) -> NavAnchorId {
        NavAnchorId {
            solid: SolidId((key >> 16) as u32),
            slot: (key & 0xffff) as u16,
        }
    }

    fn swept_hits(&self, x0: f32, x1: f32, y: f32) -> bool {
        let hazards = self.driver.hazards();
        if hazards.is_empty() {
            return false;
        }
        let (lo, hi) = (x0.min(x1), x0.max(x1));
        let swept = Rect {
            left: lo - HERO_HALF_WIDTH,
            bottom: y,
            width: hi - lo + 2.0 * HERO_HALF_WIDTH,
            height: 2.0 * HERO_HALF_HEIGHT,
        };
        hazards.iter().any(|h| h.rect.overlaps(swept))
    }
}

impl SpotWorld for NavSpots<'_> {
    fn surfaces(&self, out: &mut Vec<Surface>) {
        let graph = self.driver.graph();
        let inset = HERO_HALF_WIDTH + self.ledge_inset_px;
        // `anchors()` is in stable id order, and so is the key.
        for a in graph.anchors() {
            let (Some((x0, x1)), Some((_, top)), Some(risk)) = (
                graph.anchor_span(a),
                graph.anchor_pos(a),
                graph.anchor_risk(a),
            ) else {
                continue;
            };
            if !graph.anchor_open(a) || x1 - x0 < 2.0 * inset {
                continue;
            }
            out.push(Surface {
                key: Self::key(a),
                left: x0 + inset,
                right: x1 - inset,
                y: top,
                risk,
            });
        }
    }

    /// CA2-Q29: a hero-hurting hazard (telegraphed or active) overlaps her body
    /// standing at `x` on a surface at `y`.
    fn hazard_at(&self, x: f32, y: f32) -> bool {
        self.swept_hits(x, x, y)
    }

    /// CA2-Q29: a hero-hurting hazard overlaps her body anywhere on the ground
    /// run from `x0` to `x1` at height `y`.
    fn hazard_between(&self, x0: f32, x1: f32, y: f32) -> bool {
        self.swept_hits(x0, x1, y)
    }

    /// CA2-Q36: the price of the route in ticks, from where she stands. Each
    /// jump or drop edge costs the walk from where the leg starts (her x, then the
    /// previous landing) to its launch point, its simulated air time (the edge's
    /// centre-to-centre ticks less the two centre walks the edge adds) and the
    /// per-edge slack; the walk from the last landing to `x` closes the route.
    /// A route that is only walking costs 0.
    fn route_ticks(&self, partner: &HeroView, x: f32, surface: &Surface) -> Option<u16> {
        let graph = self.driver.graph();
        // In the air (a pounce, a jump) no anchor is within 4 px of her feet: use
        // the open anchor under her (review F-1), so a decision taken in flight
        // can still price its routes.
        let here = graph
            .anchor_at(partner.x, partner.foot_y)
            .or_else(|| Self::anchor_below(graph, partner.x, partner.foot_y))?;
        let dest = Self::anchor_of_key(surface.key);
        if here == dest {
            return Some(0);
        }
        let path = graph
            .path_or_reason(self.actor, here, dest, Unlocks::NONE)
            .ok()?;
        let run = self.driver.config().run_speed(self.actor);
        let walk = |d: f32| u32::from(walk_ticks(d, run));
        let (mut ticks, mut cursor, mut any) = (0u32, partner.x, false);
        for w in path.anchors.windows(2) {
            let plan = graph.edge_plan(w[0], w[1], self.actor)?;
            if !self.driver.edge_allowed(w[0], plan.kind) {
                return None;
            }
            if plan.kind == EdgeKind::Walk {
                continue;
            }
            any = true;
            match plan.launch_x {
                Some((launch, target)) => {
                    let mid = |a| graph.anchor_pos(a).map(|p| p.0);
                    let (mid_a, mid_b) = (mid(w[0])?, mid(w[1])?);
                    let air = u32::from(plan.ticks).saturating_sub(
                        walk((mid_a - launch).abs()) + walk((target - mid_b).abs()),
                    );
                    ticks += walk((cursor - launch).abs()) + air + self.transit_slack_ticks;
                    cursor = target;
                }
                // No simulated takeoff: the whole edge time, as before.
                None => ticks += u32::from(plan.ticks) + self.transit_slack_ticks,
            }
        }
        if any {
            ticks += walk((cursor - x).abs());
        }
        Some(ticks.min(u32::from(u16::MAX)) as u16)
    }
}

/// CA2-Q29 (adopted default, PR #220 review ruling 6): every fed rectangle is
/// rounded **outward** to this grid (layout px) before it reaches the graph, so a
/// shrinking or creeping volume (the acid drain moves an edge by about 1 px every
/// 1.25 ticks) changes the list, and rebuilds the graph, once per 8 px of travel
/// instead of nearly every tick. Class-blind; the steam lane and the acid
/// patches of the sidecars already sit on the grid.
pub const HAZARD_FEED_GRID_PX: i32 = 8;

fn grid_floor(v: i32) -> i32 {
    v.div_euclid(HAZARD_FEED_GRID_PX) * HAZARD_FEED_GRID_PX
}

fn grid_ceil(v: i32) -> i32 {
    -grid_floor(-v)
}

/// The hero-hurting rectangles of a hazard feed, as the graph's y-up
/// [`NavHazard`]s (the feed is in layout px, y down; `layout_h` is the layout's
/// height). A volume that only hurts enemies is not fed in (spec 3.5). Each
/// rectangle is rounded outward to [`HAZARD_FEED_GRID_PX`].
///
/// **Ids are made here and are unique** (PR #220 review F-2): the producers share
/// the `u16` hazard id space with no contract (sidecar volumes, acid patches, the
/// weight's footprint), and the graph refuses a duplicate. The feed is put in a
/// fixed order (hazard id, class, rectangle) and each entry takes its index as
/// the id, so equal feeds give equal ids and no producer can collide with another.
pub fn hazards_from_feed(feed: &[NavRect], layout_h: i32) -> Vec<NavHazard> {
    let mut rows: Vec<&NavRect> = feed.iter().filter(|n| n.hurts_heroes).collect();
    rows.sort_by_key(|n| (n.hazard, n.class, n.rect.x, n.rect.y, n.rect.w, n.rect.h));
    rows.into_iter()
        .enumerate()
        .map(|(i, n)| {
            let (x0, x1) = (grid_floor(n.rect.x), grid_ceil(n.rect.x + n.rect.w));
            let (y0, y1) = (grid_floor(n.rect.y), grid_ceil(n.rect.y + n.rect.h));
            NavHazard {
                id: i as u32,
                rect: Rect {
                    left: x0 as f32,
                    bottom: (layout_h - y1) as f32,
                    width: (x1 - x0) as f32,
                    height: (y1 - y0) as f32,
                },
            }
        })
        .collect()
}

/// CA2-C: the last guard of a voluntary ground step (the movement that does not
/// go through the graph: an interrupt's approach, a dodge). A step that would
/// bring her body within [`HAZARD_FENCE_MARGIN_PX`] of a hazard rectangle ahead
/// of her is cut to end at that distance, and a step into it is refused. A hero
/// already inside a hazard, or in the air, is not fenced (she may leave by any
/// way). `run_px_per_tick` is her own run speed. Returns the stick to use.
pub fn fence_step(hazards: &[NavHazard], me: Body, move_x: f32, run_px_per_tick: f32) -> f32 {
    if move_x == 0.0 || hazards.is_empty() || !me.grounded {
        return move_x;
    }
    let body = Rect {
        left: me.x - HERO_HALF_WIDTH,
        bottom: me.foot_y,
        width: 2.0 * HERO_HALF_WIDTH,
        height: 2.0 * HERO_HALF_HEIGHT,
    };
    if hazards.iter().any(|h| h.rect.overlaps(body)) {
        return move_x;
    }
    let step = move_x * run_px_per_tick;
    let mut allowed = step;
    for h in hazards {
        let r = h.rect;
        if r.bottom >= body.top() || r.top() <= body.bottom {
            continue;
        }
        if step > 0.0 && r.left >= body.right() {
            allowed = allowed.min((r.left - HAZARD_FENCE_MARGIN_PX - body.right()).max(0.0));
        } else if step < 0.0 && r.right() <= body.left {
            allowed = allowed.max(-(body.left - r.right() - HAZARD_FENCE_MARGIN_PX).max(0.0));
        }
    }
    if allowed == step {
        move_x
    } else {
        allowed / run_px_per_tick
    }
}

/// Steering toward `target_x`, the analog stick of the nav replay: full stick
/// far away, the exact remaining step near it. `run` is the body's own run
/// speed (px/s): Donut's is not Carl's.
fn approach(x: f32, target_x: f32, run: f32) -> f32 {
    ((target_x - x) / (run * TICK_SECONDS)).clamp(-1.0, 1.0)
}

/// The transit's steering: toward its `target_x`, but beside the destination
/// ledge while the feet are below its top (moving under the underside would
/// bump the head), exactly as the nav model replays a transit.
fn steer(me: Body, t: &Transit, run: f32) -> f32 {
    let mut target = t.target_x;
    if t.kind != EdgeKind::Walk && me.foot_y < t.dst.2 - LAND_EPS_PX {
        target = if t.target_x >= t.launch_x {
            target.min(t.dst.0 - HERO_HALF_WIDTH)
        } else {
            target.max(t.dst.1 + HERO_HALF_WIDTH)
        };
    }
    approach(me.x, target, run)
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

    /// N1 (DM-A): the arrive tolerance must exceed what the stick dead zone can
    /// leave unspent at the body's run speed, or a residual just above the old
    /// 0.6 px tolerance is zeroed by [`approach`] and the transit stalls.
    #[test]
    fn the_arrive_tolerance_covers_the_deadzone_leftover_at_both_speeds() {
        use crate::input::Actions;
        // Carl's tolerance is exactly the historic constant (bit-identical).
        assert_eq!(arrive_eps_px(RUN_SPEED).to_bits(), ARRIVE_EPS_PX.to_bits());
        assert_eq!(ARRIVE_EPS_PX, 0.6);
        let donut = crate::agility::agile_motion().run_speed;
        let eps = arrive_eps_px(donut);
        assert!((eps - 0.666).abs() < 0.001, "{eps}");
        assert!(eps > STICK_DEADZONE * donut * TICK_SECONDS);
        // The bug the fix prevents: at 207 px/s a 0.61 px residual is above the
        // old tolerance and below the dead-zone leftover (0.621 px).
        let stuck = Actions::from(crate::input::RawInput {
            move_x: approach(0.0, 0.61, donut),
            ..crate::input::RawInput::NONE
        })
        .move_x;
        assert_eq!(stuck, 0.0, "the stall the old tolerance allowed");
        // Sweep residuals 0..=3.45 px in 0.01 steps, both signs, at both speeds:
        // outside the tolerance the stick always moves the hero, a step never
        // overshoots the target, and the hero is inside the tolerance within a
        // few ticks.
        for run in [RUN_SPEED, donut] {
            let eps = arrive_eps_px(run);
            for n in 0..=345 {
                for sign in [1.0_f32, -1.0] {
                    let mut residual = sign * (n as f32) * 0.01;
                    let mut ticks = 0;
                    while residual.abs() > eps {
                        let stick = Actions::from(crate::input::RawInput {
                            move_x: approach(0.0, residual, run),
                            ..crate::input::RawInput::NONE
                        })
                        .move_x;
                        assert_ne!(stick, 0.0, "stalled at {residual} px, {run} px/s");
                        let step = stick * run * TICK_SECONDS;
                        assert!(
                            step.abs() <= residual.abs() + 1e-4,
                            "overshoot {step} of {residual}"
                        );
                        residual -= step;
                        ticks += 1;
                        assert!(ticks <= 3, "no arrival from {n}");
                    }
                    assert!(residual.abs() <= eps);
                }
            }
        }
    }

    /// DM-A: the executor on the agile preset climbs a 104 px ledge with
    /// Donut's agile body (the boss rack's height) where Carl's cannot.
    #[test]
    fn the_agile_preset_climbs_a_104_px_ledge_with_her_agile_body() {
        let blocks = vec![
            stone(0.0, 0.0, 2000.0, 40.0),
            stone(300.0, 104.0, 300.0, 144.0),
        ];
        let motion = crate::agility::agile_motion();
        let leader = standing(450.0, 104.0);
        let mut driver = NavDriver::new(&blocks, NavConfig::agile()).unwrap();
        let mut hero = Hero::at(100.0, HERO_HALF_HEIGHT);
        let mut frame = InputFrame::default();
        let mut arrived = false;
        for _ in 0..400 {
            let (drive, notes) = driver.follow(HeroId::Donut, body(&hero), leader, None);
            arrived |= notes.iter().any(|n| matches!(n, NavNote::Arrived { .. }));
            let d = drive.unwrap_or_default();
            frame.advance(RawInput {
                move_x: d.move_x,
                jump: d.jump,
                ..RawInput::NONE
            });
            crate::prototype::step_hero_motion(
                &mut hero,
                motion,
                &frame,
                &blocks,
                2000.0,
                &crate::prototype::BodyDrive::default(),
            );
        }
        assert!(arrived, "{hero:?}");
        assert!((hero.y - HERO_HALF_HEIGHT - 104.0).abs() < 0.01, "{hero:?}");
        // Carl, on the integrated preset, has no route: the ledge is out of reach.
        let mut carl = NavDriver::new(&blocks, NavConfig::agile()).unwrap();
        let (hero, notes, highest) = run(&mut carl, &blocks, 100.0, leader, 300);
        assert!(highest < 0.01, "Carl never jumped: {highest}");
        assert!(hero.y - HERO_HALF_HEIGHT < 0.01);
        assert!(notes.iter().any(|n| matches!(n, NavNote::Waiting { .. })));
    }

    /// CA2-B1: `go_to` is the follow executor toward a spot. On her own anchor it
    /// says nothing (the caller steers flat); to a spot on a ledge her agile jump
    /// reaches it commits a transit and arrives through the real body step.
    #[test]
    fn go_to_commits_the_same_transits_toward_a_spot() {
        let blocks = vec![
            stone(0.0, 0.0, 2000.0, 40.0),
            stone(300.0, 104.0, 300.0, 144.0),
        ];
        let motion = crate::agility::agile_motion();
        let mut driver = NavDriver::new(&blocks, NavConfig::agile()).unwrap();
        // Same anchor: nothing to add.
        let flat = driver.go_to(
            HeroId::Donut,
            standing(100.0, 0.0),
            standing(250.0, 0.0),
            None,
        );
        assert_eq!(flat.0, None);
        // A spot on the ledge: a transit, then arrival on the ledge.
        let spot = standing(450.0, 104.0);
        let mut hero = Hero::at(100.0, HERO_HALF_HEIGHT);
        let mut frame = InputFrame::default();
        let (mut begun, mut arrived) = (false, false);
        for _ in 0..400 {
            let (drive, notes) = driver.go_to(HeroId::Donut, body(&hero), spot, None);
            begun |= notes.iter().any(|n| matches!(n, NavNote::Begin { .. }));
            arrived |= notes.iter().any(|n| matches!(n, NavNote::Arrived { .. }));
            let d = drive.unwrap_or_default();
            frame.advance(RawInput {
                move_x: d.move_x,
                jump: d.jump,
                ..RawInput::NONE
            });
            crate::prototype::step_hero_motion(
                &mut hero,
                motion,
                &frame,
                &blocks,
                2000.0,
                &crate::prototype::BodyDrive::default(),
            );
        }
        assert!(begun && arrived, "{hero:?}");
        assert!((hero.y - HERO_HALF_HEIGHT - 104.0).abs() < 0.01, "{hero:?}");
    }

    /// CA2-B1: with no route a hold on a spot closes on the spot itself, where the
    /// follow hold stops short of the leader (its 40 px band and 8 px slack).
    #[test]
    fn a_hold_toward_a_spot_has_no_wait_band() {
        // A 200 px tower nobody climbs, the floor only: the spot on top has no
        // route and the nearest safe anchor is the floor she stands on.
        let blocks = vec![
            stone(0.0, 0.0, 2000.0, 40.0),
            stone(700.0, 200.0, 60.0, 200.0),
        ];
        let me = standing(690.0, 0.0);
        let target = standing(730.0, 200.0);
        let mut follow = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
        let (drive, notes) = follow.follow(HeroId::Carl, me, target, None);
        assert!(notes.iter().any(|n| matches!(n, NavNote::Waiting { .. })));
        assert_eq!(drive.unwrap().move_x, 0.0, "within the 48 px follow band");
        let mut goto = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
        let (drive, notes) = goto.go_to(HeroId::Carl, me, target, None);
        assert!(notes.iter().any(|n| matches!(n, NavNote::Waiting { .. })));
        assert!(drive.unwrap().move_x > 0.0, "a go-to closes on the spot");
    }

    #[test]
    fn nav_spots_list_open_anchors_inset_and_price_a_route_in_transit_ticks() {
        use crate::companion::autonomy::{SpotWorld, Surface};
        use crate::companion::view::{Control, fixtures};
        let blocks = vec![
            stone(0.0, 0.0, 2000.0, 40.0),
            stone(300.0, 104.0, 300.0, 144.0),
            stone(1200.0, 20.0, 40.0, 20.0), // narrower than 2 x (12 + 16): no spot
        ];
        let driver = NavDriver::new(&blocks, NavConfig::agile()).unwrap();
        let spots = NavSpots::new(&driver, HeroId::Donut, 16.0);
        let mut out = Vec::new();
        spots.surfaces(&mut out);
        assert!(
            out.windows(2).all(|w| w[0].key < w[1].key),
            "ascending keys"
        );
        let floor = out.iter().find(|s| s.y == 0.0).copied().unwrap();
        // The anchor span inset by 12 + 16 on each end.
        assert!(floor.left > 0.0 && floor.left <= 28.0 + 1.0, "{floor:?}");
        let ledge = out.iter().find(|s| s.y == 104.0).copied().unwrap();
        assert!((ledge.left - 328.0).abs() < 1.0 && (ledge.right - 572.0).abs() < 1.0);
        assert!(out.iter().all(|s: &Surface| s.right >= s.left));
        // She stands on the floor: a walk is free, the ledge costs its transit.
        let mut me = fixtures::hero(HeroId::Donut, 100.0, Control::Companion);
        me.foot_y = 0.0;
        assert_eq!(spots.route_ticks(&me, 150.0, &floor), Some(0));
        let ticks = spots
            .route_ticks(&me, 450.0, &ledge)
            .expect("agile reaches it");
        assert!(ticks > 0);
        // In the air over the floor (a pounce: feet 12 px up, no anchor within 4 px)
        // she is priced from the anchor under her, and over the ledge from the ledge.
        let mut flying = me;
        flying.foot_y = 12.0;
        flying.grounded = false;
        assert_eq!(spots.route_ticks(&flying, 150.0, &floor), Some(0));
        assert_eq!(spots.route_ticks(&flying, 450.0, &ledge), Some(ticks));
        let mut over_ledge = flying;
        over_ledge.x = 450.0;
        over_ledge.foot_y = 120.0;
        assert_eq!(spots.route_ticks(&over_ledge, 450.0, &ledge), Some(0));
        // Below every surface there is nothing to price from.
        let mut under = flying;
        under.foot_y = -50.0;
        assert_eq!(spots.route_ticks(&under, 150.0, &floor), None);
        let integrated = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
        let cs = NavSpots::new(&integrated, HeroId::Carl, 16.0);
        let mut out2 = Vec::new();
        cs.surfaces(&mut out2);
        let ledge2 = out2.iter().find(|s| s.y == 104.0).copied().unwrap();
        assert_eq!(cs.route_ticks(&me, 450.0, &ledge2), None);
    }

    /// CA2 Revision 3 / DP-Q38: under a jump ceiling no Jump edge starts from an
    /// anchor above it (she holds there), a jump from below it, a drop from above
    /// it and every walk are unaffected, and a spot whose route needs such a jump
    /// is not priced. The ceiling is a per-tick control flag (cleared with `None`).
    #[test]
    fn the_jump_ceiling_stops_jump_edges_from_high_anchors_only() {
        use crate::companion::autonomy::{SpotWorld, Surface};
        use crate::companion::view::{Control, fixtures};
        let blocks = level();
        let on_step = standing(230.0, 60.0);
        let up_goal = standing(330.0, 120.0);
        let mut driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
        // Free: from the 60 step to the 120 step is a committed jump.
        let (drive, _) = driver.go_to(HeroId::Carl, on_step, up_goal, None);
        assert!(drive.is_some() && driver.heroes[0].is_active());
        // Ceiling at 50 (below the 60 step): she holds, no transit begins.
        let mut driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
        driver.set_jump_ceiling(Some(50.0));
        let (drive, notes) = driver.go_to(HeroId::Carl, on_step, up_goal, None);
        assert_eq!(drive, Some(Drive::default()));
        assert!(!driver.heroes[0].is_active());
        assert!(!notes.iter().any(|n| matches!(n, NavNote::Begin { .. })));
        // Exactly at the ceiling is allowed (60 <= 60), one px below is not.
        driver.set_jump_ceiling(Some(60.0));
        let (drive, _) = driver.go_to(HeroId::Carl, on_step, up_goal, None);
        assert!(drive.is_some() && driver.heroes[0].is_active());
        driver.heroes[0] = HeroNav::default();
        driver.set_jump_ceiling(Some(59.9));
        let (drive, _) = driver.go_to(HeroId::Carl, on_step, up_goal, None);
        assert_eq!(drive, Some(Drive::default()));
        // A jump from the floor (top 0) is allowed under the same ceiling.
        driver.set_jump_ceiling(Some(50.0));
        let (drive, _) = driver.go_to(
            HeroId::Carl,
            standing(100.0, 0.0),
            standing(230.0, 60.0),
            None,
        );
        assert!(drive.is_some() && driver.heroes[0].is_active());
        driver.heroes[0] = HeroNav::default();
        // A drop from the top step to the floor is never limited.
        let (drive, _) = driver.go_to(
            HeroId::Carl,
            standing(330.0, 120.0),
            standing(500.0, 0.0),
            None,
        );
        assert!(drive.is_some());
        // Pricing: the 120 step is not priced from the 60 step under the ceiling,
        // and is when the ceiling is lifted.
        let mut out = Vec::new();
        NavSpots::new(&driver, HeroId::Carl, 16.0).surfaces(&mut out);
        let top: Surface = out.iter().find(|s| s.y == 120.0).copied().unwrap();
        let mut me = fixtures::hero(HeroId::Carl, 230.0, Control::Companion);
        me.foot_y = 60.0;
        assert_eq!(
            NavSpots::new(&driver, HeroId::Carl, 16.0).route_ticks(&me, 330.0, &top),
            None
        );
        driver.set_jump_ceiling(None);
        assert!(
            NavSpots::new(&driver, HeroId::Carl, 16.0)
                .route_ticks(&me, 330.0, &top)
                .is_some()
        );
    }

    /// DP-B: `set_config` rebuilds the graph only when the physics changed (the
    /// grant tick), keeps the executor state, and the same config is a no-op.
    #[test]
    fn set_config_rebuilds_on_a_new_launch_and_never_otherwise() {
        let blocks = vec![
            stone(0.0, 0.0, 2000.0, 40.0),
            stone(300.0, 128.0, 300.0, 128.0),
        ];
        let base = NavConfig::agile();
        let boosted = base.with_donut_launch(547.0);
        let mut driver = NavDriver::new(&blocks, base).unwrap();
        let edges = |d: &NavDriver, hero: HeroId| {
            use crate::companion::nav::EdgeState;
            d.graph()
                .edge_views()
                .iter()
                .filter(|e| matches!(e.state[hero.index()], EdgeState::Open { .. }))
                .count()
        };
        let (donut0, carl0) = (edges(&driver, HeroId::Donut), edges(&driver, HeroId::Carl));
        let revision = driver.graph().revision();
        driver.heroes[1].pressed_last = true;
        driver.set_config(base, &blocks).unwrap();
        assert_eq!(
            driver.graph().revision(),
            revision,
            "the same config is a no-op"
        );
        driver.set_config(boosted, &blocks).unwrap();
        assert_eq!(driver.config(), boosted);
        assert!(
            edges(&driver, HeroId::Donut) > donut0,
            "the 128 px edge opens"
        );
        assert_eq!(
            edges(&driver, HeroId::Carl),
            carl0,
            "Carl's edges are identical"
        );
        assert!(driver.heroes[1].pressed_last, "the executor state is kept");
        // And back (a retry that drops the boost): the original graph.
        driver.set_config(base, &blocks).unwrap();
        assert_eq!(edges(&driver, HeroId::Donut), donut0);
    }

    /// CA2-Q36 (PR #213 review B-1): a jump across a 32 px gap is priced from where
    /// she stands, not from anchor centre to centre. Two 400 px floors with a 32 px
    /// gap: from x 380 to a spot at x 460 the price is the walk to the launch
    /// point, the simulated air time and the landing walk; the old price was the
    /// whole edge's centre-to-centre time (which drove every spot across the gap
    /// out of the drift budget).
    #[test]
    fn route_ticks_prices_a_jump_from_her_position_not_centre_to_centre() {
        use crate::companion::autonomy::SpotWorld;
        use crate::companion::nav::EdgeState;
        use crate::companion::view::{Control, fixtures};
        let blocks = vec![stone(0.0, 0.0, 400.0, 40.0), stone(432.0, 0.0, 400.0, 40.0)];
        let driver = NavDriver::new(&blocks, NavConfig::agile()).unwrap();
        let mut out = Vec::new();
        NavSpots::new(&driver, HeroId::Donut, 16.0).surfaces(&mut out);
        let far = out.iter().find(|s| s.left > 400.0).copied().unwrap();
        // The edge: its centre-to-centre ticks and its simulated takeoff/target.
        let views = driver.graph().edge_views();
        let edge = views
            .iter()
            .find(|e| {
                driver.graph().anchor_pos(e.from).unwrap().0 < 300.0
                    && driver.graph().anchor_pos(e.to).unwrap().0 > 500.0
            })
            .unwrap();
        let EdgeState::Open {
            ticks: whole, kind, ..
        } = edge.state[HeroId::Donut.index()]
        else {
            panic!("the 32 px gap is a Donut jump");
        };
        assert_eq!(kind, EdgeKind::Jump);
        assert!(whole > 150, "the centre-to-centre time is large: {whole}");
        let mut me = fixtures::hero(HeroId::Donut, 380.0, Control::Companion);
        me.foot_y = 0.0;
        let me = me;
        let price = |from: f32, slack: u16, x: f32| {
            let mut me = me;
            me.x = from;
            NavSpots::new(&driver, HeroId::Donut, 16.0)
                .with_transit_slack(slack)
                .route_ticks(&me, x, &far)
                .unwrap()
        };
        let exact = price(380.0, 0, 460.0);
        // Launch x 398, target x 434, centres 200 and 632: from x 380 the walk to the
        // launch is 6 ticks (18 px), the air time 173 - 58 - 58 = 57, and the landing
        // walk to x 460 is 8 ticks (26 px).
        assert_eq!(exact, 6 + 57 + 8, "walk to launch + air + landing walk");
        assert_eq!(whole, 173);
        assert!(exact * 2 < whole, "{exact} against the old {whole}");
        // The slack is per jump edge; the landing walk follows the spot's x.
        assert_eq!(price(380.0, 4, 460.0), exact + 4);
        assert!(
            price(380.0, 0, 520.0) > exact,
            "a farther spot costs its landing walk"
        );
        // Standing 100 px back from the launch point costs more than at it.
        assert!(
            price(280.0, 0, 460.0) > exact,
            "she is farther from the launch"
        );
    }

    #[test]
    fn a_two_hop_route_is_climbed_through_the_real_body_step() {
        let blocks = level();
        let mut driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
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
        let mut driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
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
        let mut driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
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
        let mut driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
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
        let mut driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
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
        let driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
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
        let driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
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
        let driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
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
    fn a_regroup_waits_for_the_leader_to_stand() {
        // The reviewer's probe: the hero stands on the left floor, a pit
        // nobody can climb out of lies between it and the leader's platforms,
        // and the leader jumps over the pit between stone A and platform B.
        let blocks = vec![
            stone(0.0, 0.0, 700.0, 40.0),
            stone(700.0, -200.0, 900.0, 40.0),
            stone(740.0, 0.0, 40.0, 16.0),
            stone(820.0, 0.0, 40.0, 16.0),
            stone(900.0, 0.0, 40.0, 16.0),
            stone(980.0, 0.0, 40.0, 16.0),
            stone(1060.0, 0.0, 140.0, 16.0),
            stone(1240.0, 0.0, 1700.0, 16.0),
        ];
        let driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
        let v = RegroupView {
            view_left: 900.0,
            view_right: 1540.0,
            behind: -1.0,
        };
        let hero = standing(100.0, 0.0);
        let jumping = Body {
            grounded: false,
            ..standing(1220.0, 40.0)
        };
        assert_eq!(
            driver.regroup(HeroId::Donut, hero, jumping, &v),
            Regroup::Defer
        );
        // Falling over the pit itself, and in the air over no anchor at all.
        for (x, foot) in [(1220.0, -100.0), (5000.0, 70.0)] {
            let air = Body {
                grounded: false,
                ..standing(x, foot)
            };
            assert_eq!(driver.regroup(HeroId::Donut, hero, air, &v), Regroup::Defer);
        }
        // The same leader standing on platform A: a stone, never the pit
        // bottom (x 886, foot -200), which the hero could not climb out of.
        let r = driver.regroup(HeroId::Donut, hero, standing(1130.0, 0.0), &v);
        let Regroup::Place { anchor, x, foot_y } = r else {
            panic!("{r:?}")
        };
        assert!(foot_y >= 0.0, "not the pit bottom: {anchor:?} {x} {foot_y}");
        assert!(x < 900.0 - HERO_HALF_WIDTH);
    }

    #[test]
    fn a_regroup_checks_the_leader_is_reachable_once_he_stands() {
        let blocks = dead_end_level();
        let driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
        let v = RegroupView {
            view_left: 900.0,
            view_right: 1540.0,
            behind: -1.0,
        };
        // The pit is a dead end for a leader on the main floor: refused.
        let r = driver.regroup(
            HeroId::Donut,
            standing(2800.0, 0.0),
            standing(1400.0, 0.0),
            &v,
        );
        assert!(matches!(r, Regroup::Denied(_)), "{r:?}");
    }

    #[test]
    fn a_stranded_hero_reports_once_not_every_tick() {
        // The route to the leader existed as built and a destruction closed it
        // (reason Destroyed); the camera bounds hold no safe anchor, so the hero
        // is stranded on every tick.
        let mut blocks = level();
        blocks[1].material = Material::Timber;
        let mut driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
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

    // ---- CA2-Q36 follow-up: the transit price of two-hop and drop routes

    /// A route's priced ticks from where she stands, in `level()` with Carl's
    /// integrated physics, against the same route driven through the real body
    /// step (the executor's landing tick plus the walk to `x`). The price is the
    /// drift check's input (spec 3.3), so it must track the real transit.
    fn priced_and_driven(from: f32, from_y: f32, to_x: f32, to_y: f32) -> (u16, u32) {
        use crate::companion::autonomy::SpotWorld;
        use crate::companion::view::{Control, fixtures};
        let blocks = level();
        let mut driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
        let mut out = Vec::new();
        NavSpots::new(&driver, HeroId::Carl, 16.0).surfaces(&mut out);
        let dest = *out
            .iter()
            .rfind(|s| (s.y - to_y).abs() < 0.5 && s.left <= to_x && s.right >= to_x)
            .expect("a destination surface");
        let mut view = fixtures::hero(HeroId::Carl, from, Control::Companion);
        view.foot_y = from_y;
        let price = NavSpots::new(&driver, HeroId::Carl, 16.0)
            .route_ticks(&view, to_x, &dest)
            .expect("a route");
        // Drive it: the hero starts standing at `from` on `from_y`.
        let mut hero = Hero::at(from, from_y + HERO_HALF_HEIGHT);
        let goal = standing(to_x, to_y);
        let mut frame = InputFrame::default();
        let mut arrived = None;
        for t in 1..=600u32 {
            let (drive, n) = driver.go_to(HeroId::Carl, body(&hero), goal, None);
            if arrived.is_none() && n.iter().any(|n| matches!(n, NavNote::Arrived { .. })) {
                arrived = Some(t);
            }
            let d = drive.unwrap_or_else(|| Drive {
                move_x: approach(hero.x, to_x, RUN_SPEED),
                jump: false,
            });
            frame.advance(RawInput {
                move_x: d.move_x,
                jump: d.jump,
                ..RawInput::NONE
            });
            step_hero(&mut hero, &frame, &blocks, 2000.0);
            if arrived.is_some() && (hero.x - to_x).abs() < 1.0 {
                return (price, t);
            }
        }
        panic!("the route was never driven to its end ({arrived:?})");
    }

    #[test]
    fn route_ticks_prices_a_two_hop_climb_and_a_drop_like_the_real_transit() {
        // The review table of PR #213 (slack 0): the price is within a few ticks of
        // the driven transit, never the old centre-to-centre sum.
        // Two-hop climb from the left: floor (0), step 60, step 120.
        let (climb_left, driven) = priced_and_driven(100.0, 0.0, 330.0, 120.0);
        assert_eq!(climb_left, 136, "two hops from the left");
        assert!(
            (i64::from(climb_left) - i64::from(driven)).abs() <= 4,
            "priced {climb_left}, driven {driven}"
        );
        // From the right of the stairs the same climb starts from the far side. The
        // price grows 16 ticks per 50 px of start distance (Carl's walk): 162 from
        // x = 500 and 195 from x = 600, the value of the #213 review's table (PR #220
        // review ruling 4); each is within 2 ticks of the driven transit.
        let (climb_right, driven) = priced_and_driven(500.0, 0.0, 330.0, 120.0);
        assert_eq!(climb_right, 162, "two hops from x = 500");
        assert!(
            (i64::from(climb_right) - i64::from(driven)).abs() <= 4,
            "priced {climb_right}, driven {driven}"
        );
        let (climb_far, driven) = priced_and_driven(600.0, 0.0, 330.0, 120.0);
        assert_eq!(climb_far, 195, "two hops from x = 600 (the #213 table)");
        assert!(
            (i64::from(climb_far) - i64::from(driven)).abs() <= 4,
            "priced {climb_far}, driven {driven}"
        );
        assert!(climb_far > climb_right && climb_right > climb_left);
        // One hop (floor to the 60 px step), for the reference value.
        let (hop, driven) = priced_and_driven(100.0, 0.0, 230.0, 60.0);
        assert_eq!(hop, 78, "one hop");
        assert!((i64::from(hop) - i64::from(driven)).abs() <= 12);
        // Drops: from the top step down to the floor, to the right and to the left.
        let (drop_right, driven) = priced_and_driven(330.0, 120.0, 500.0, 0.0);
        assert_eq!(drop_right, 85, "a drop to the right");
        assert!(
            (i64::from(drop_right) - i64::from(driven)).abs() <= 12,
            "priced {drop_right}, driven {driven}"
        );
        let (drop_left, driven) = priced_and_driven(330.0, 120.0, 100.0, 0.0);
        assert_eq!(drop_left, 106, "a drop to the left");
        assert!(
            (i64::from(drop_left) - i64::from(driven)).abs() <= 12,
            "priced {drop_left}, driven {driven}"
        );
    }

    // ---- CA2-C: the hazard input

    fn lane(left: f32, width: f32, id: u32) -> NavHazard {
        NavHazard {
            id,
            rect: Rect {
                left,
                bottom: 0.0,
                width,
                height: 60.0,
            },
        }
    }

    fn flat() -> Vec<Block> {
        vec![stone(0.0, 0.0, 2000.0, 40.0)]
    }

    #[test]
    fn set_hazards_rebuilds_on_a_change_only_and_keeps_the_executor_state() {
        let blocks = flat();
        let mut driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
        let plain = driver.graph().state_hash();
        assert_eq!(driver.set_hazards(Vec::new(), &blocks), Ok(false));
        assert_eq!(driver.graph().state_hash(), plain, "no feed, no rebuild");
        assert_eq!(
            driver.set_hazards(vec![lane(600.0, 80.0, 1)], &blocks),
            Ok(true)
        );
        let fed = driver.graph().state_hash();
        assert_ne!(fed, plain);
        assert_eq!(
            driver.set_hazards(vec![lane(600.0, 80.0, 1)], &blocks),
            Ok(false),
            "the same list is a no-op"
        );
        assert_eq!(driver.graph().state_hash(), fed);
        // The list is kept sorted by id whatever the order it came in.
        let mut a = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
        let mut b = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
        a.set_hazards(vec![lane(600.0, 80.0, 1), lane(900.0, 40.0, 2)], &blocks)
            .unwrap();
        b.set_hazards(vec![lane(900.0, 40.0, 2), lane(600.0, 80.0, 1)], &blocks)
            .unwrap();
        assert_eq!(a.graph().state_hash(), b.graph().state_hash());
        // A duplicate id is rejected and changes nothing.
        let before = a.graph().state_hash();
        assert!(
            a.set_hazards(vec![lane(1.0, 2.0, 5), lane(10.0, 2.0, 5)], &blocks)
                .is_err()
        );
        assert_eq!(a.graph().state_hash(), before);
        assert_eq!(a.hazards().len(), 2);
        // Clearing the feed restores the plain graph (the hazard has passed).
        driver.set_hazards(Vec::new(), &blocks).unwrap();
        assert_eq!(driver.graph().state_hash(), plain);
        // A destruction sync after a feed keeps the hazards (the rebuild path).
        driver
            .set_hazards(vec![lane(600.0, 80.0, 1)], &blocks)
            .unwrap();
        driver.sync(&blocks).unwrap();
        assert_eq!(driver.graph().state_hash(), fed);
        let mut more = blocks.clone();
        more.push(stone(1200.0, 100.0, 40.0, 8.0));
        driver.sync(&more).unwrap();
        assert_eq!(driver.hazards().len(), 1, "the rebuild kept the feed");
        assert!(
            driver
                .graph()
                .anchors()
                .any(|a| driver.graph().anchor_risk(a) == Some(crate::companion::nav::RISK_MAX)),
            "and the graph still has the hazard anchor"
        );
    }

    #[test]
    fn the_feed_converts_layout_rectangles_and_leaves_out_what_only_hurts_enemies() {
        use crate::camera_lab::room::IRect;
        use crate::env_hazard::{HazardClass, NavPhase};
        let rect = |x, y, w, h| IRect::new(x, y, w, h);
        let feed = [
            NavRect {
                hazard: 3,
                class: HazardClass::Steam,
                rect: rect(688, 484, 80, 96),
                phase: NavPhase::Active,
                ticks_left: Some(10),
                hurts_heroes: true,
                hurts_enemies: true,
            },
            NavRect {
                hazard: 4,
                class: HazardClass::FallingMass,
                rect: rect(784, 572, 64, 8),
                phase: NavPhase::Telegraph,
                ticks_left: Some(24),
                hurts_heroes: true,
                hurts_enemies: false,
            },
            NavRect {
                hazard: 5,
                class: HazardClass::Blast,
                rect: rect(0, 0, 10, 10),
                phase: NavPhase::Active,
                ticks_left: Some(1),
                hurts_heroes: false,
                hurts_enemies: true,
            },
        ];
        let got = hazards_from_feed(&feed, 580);
        assert_eq!(got.len(), 2, "the enemy-only volume is not fed in");
        // Sorted by hazard id, ids are the index. Layout y 484 to 580 rounds
        // outward to 480 to 584 on the 8 px grid: world bottom -4, height 104.
        assert_eq!(got[0].id, 0);
        assert_eq!(
            got[0].rect,
            Rect {
                left: 688.0,
                bottom: -4.0,
                width: 80.0,
                height: 104.0
            }
        );
        assert_eq!(got[1].id, 1);
        // The footprint's 572..580 shadow rounds to 568..584.
        assert_eq!(got[1].rect.bottom, -4.0);
        assert_eq!(got[1].rect.height, 16.0);
        // An off-grid rectangle rounds outward on every side.
        let off = [NavRect {
            hazard: 9,
            class: HazardClass::Acid,
            rect: rect(701, 570, 62, 10),
            phase: NavPhase::Active,
            ticks_left: None,
            hurts_heroes: true,
            hurts_enemies: false,
        }];
        let r = hazards_from_feed(&off, 580)[0].rect;
        assert_eq!((r.left, r.right()), (696.0, 768.0));
        assert_eq!((r.bottom, r.top()), (-4.0, 12.0));
        // A shrinking volume changes the list once per 8 px of travel, not per px.
        let lists: std::collections::BTreeSet<i64> = (0..32)
            .map(|k| {
                let n = NavRect {
                    rect: rect(700 + k, 560, 64 - 2 * k, 20),
                    ..off[0]
                };
                let r = hazards_from_feed(&[n], 580)[0].rect;
                (r.left as i64) * 10_000 + r.right() as i64
            })
            .collect();
        assert!(
            lists.len() <= 9,
            "32 px of drain gave {} lists",
            lists.len()
        );
        // Two producers with the same id never collide: the ids are the index.
        let same = [
            NavRect {
                hazard: 1,
                ..off[0]
            },
            NavRect {
                hazard: 1,
                class: HazardClass::Steam,
                rect: rect(0, 500, 32, 80),
                ..off[0]
            },
        ];
        let ids: Vec<u32> = hazards_from_feed(&same, 580).iter().map(|h| h.id).collect();
        assert_eq!(ids, [0, 1]);
        // And the graph takes the list the two colliding sources made.
        let blocks = flat();
        let mut driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
        assert_eq!(
            driver.set_hazards(hazards_from_feed(&same, 580), &blocks),
            Ok(true)
        );
        assert_eq!(driver.hazards().len(), 2);
        assert!(hazards_from_feed(&[], 580).is_empty());
    }

    #[test]
    fn a_hazard_lane_holds_her_on_the_near_side_and_the_route_reopens_when_it_clears() {
        let blocks = flat();
        let mut driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
        let leader = standing(900.0, 0.0);
        let mut hero = Hero::at(300.0, HERO_HALF_HEIGHT);
        let mut frame = InputFrame::default();
        let hazard = lane(600.0, 80.0, 1);
        let mut waiting = 0;
        let mut step_once =
            |driver: &mut NavDriver, hero: &mut Hero, fence: &[NavHazard]| -> usize {
                let (drive, notes) = driver.go_to(HeroId::Carl, body(hero), leader, None);
                let held = notes
                    .iter()
                    .filter(|n| {
                        matches!(
                            n,
                            NavNote::Waiting {
                                reason: Reason::Hazard,
                                ..
                            }
                        )
                    })
                    .count();
                // The flat part the wiring falls back on, then the fence.
                let move_x =
                    drive.map_or_else(|| approach(hero.x, leader.x, RUN_SPEED), |d| d.move_x);
                let move_x = fence_step(fence, body(hero), move_x, RUN_SPEED * TICK_SECONDS);
                frame.advance(RawInput {
                    move_x,
                    ..RawInput::NONE
                });
                step_hero(hero, &frame, &blocks, 2000.0);
                held
            };
        // The lane is telegraphed from the start: she walks up to it and stops.
        driver.set_hazards(vec![hazard.clone()], &blocks).unwrap();
        for _ in 0..400 {
            waiting += step_once(&mut driver, &mut hero, std::slice::from_ref(&hazard));
        }
        assert!(
            hero.x + HERO_HALF_WIDTH <= 600.0,
            "she never enters the lane: {}",
            hero.x
        );
        assert!(
            hero.x + HERO_HALF_WIDTH >= 590.0,
            "and she waits at its edge, not far back: {}",
            hero.x
        );
        assert!(waiting >= 1, "the hold names the hazard as the reason");
        // The lane ends: the same executor reaches the leader.
        driver.set_hazards(Vec::new(), &blocks).unwrap();
        for _ in 0..400 {
            step_once(&mut driver, &mut hero, &[]);
        }
        assert!(
            (hero.x - 900.0).abs() < 50.0,
            "she crosses once it clears: {}",
            hero.x
        );
    }

    #[test]
    fn the_fence_stops_a_step_at_the_hazard_and_never_traps_a_hero_inside_it() {
        let hz = [lane(600.0, 80.0, 1)];
        let run = RUN_SPEED * TICK_SECONDS;
        let at = |x: f32| standing(x, 0.0);
        // Far away: untouched, both ways.
        assert_eq!(fence_step(&hz, at(300.0), 1.0, run), 1.0);
        assert_eq!(fence_step(&hz, at(300.0), -1.0, run), -1.0);
        // Next to it: the step is cut to leave the 2 px margin (body edge 598).
        let near = 600.0 - HAZARD_FENCE_MARGIN_PX - HERO_HALF_WIDTH - 1.0;
        let cut = fence_step(&hz, at(near), 1.0, run);
        assert!((cut * run - 1.0).abs() < 1e-3, "{}", cut * run);
        // At the margin: refused. Walking away: free.
        let edge = 600.0 - HAZARD_FENCE_MARGIN_PX - HERO_HALF_WIDTH;
        assert_eq!(fence_step(&hz, at(edge), 1.0, run), 0.0);
        assert_eq!(fence_step(&hz, at(edge), -1.0, run), -1.0);
        // The far side, walking left into it.
        let far = 680.0 + HAZARD_FENCE_MARGIN_PX + HERO_HALF_WIDTH + 1.0;
        let cut = fence_step(&hz, at(far), -1.0, run);
        assert!((cut * run + 1.0).abs() < 1e-3, "{}", cut * run);
        assert_eq!(fence_step(&hz, at(far - 1.0), -1.0, run), 0.0);
        // Already inside: she may walk out either way (no trap).
        assert_eq!(fence_step(&hz, at(640.0), 1.0, run), 1.0);
        assert_eq!(fence_step(&hz, at(640.0), -1.0, run), -1.0);
        // In the air, or with a hazard above her head or below her feet: free.
        let mut air = at(edge);
        air.grounded = false;
        assert_eq!(fence_step(&hz, air, 1.0, run), 1.0);
        let high = [NavHazard {
            id: 2,
            rect: Rect {
                left: 600.0,
                bottom: 80.0,
                width: 80.0,
                height: 20.0,
            },
        }];
        assert_eq!(fence_step(&high, at(edge), 1.0, run), 1.0, "above her head");
        assert_eq!(fence_step(&[], at(edge), 1.0, run), 1.0, "no feed");
        assert_eq!(fence_step(&hz, at(edge), 0.0, run), 0.0);
        // The nearer of two lanes is the fence.
        let two = [lane(900.0, 40.0, 3), lane(600.0, 80.0, 1)];
        assert_eq!(fence_step(&two, at(edge), 1.0, run), 0.0);
    }

    #[test]
    fn an_evacuation_leads_out_of_the_hazard_and_only_when_she_stands_in_one() {
        let blocks = flat();
        let mut driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
        let hero = |x: f32| standing(x, 0.0);
        assert_eq!(driver.evacuation(HeroId::Donut, hero(640.0), None), None);
        driver
            .set_hazards(vec![lane(600.0, 80.0, 1)], &blocks)
            .unwrap();
        // Nowhere near it, or at its edge but outside: nothing to do.
        assert_eq!(driver.evacuation(HeroId::Donut, hero(300.0), None), None);
        // Her body 12 px over the lane's edge with her centre still on safe
        // ground (the anchor ends at the rectangle): still in it, and the way out
        // is the few px that clear it. Just clear of it (2 px margin): nothing.
        let edge = driver
            .evacuation(HeroId::Donut, hero(596.0), None)
            .expect("her body is over the edge");
        assert!(edge.x + HERO_HALF_WIDTH <= 600.0, "{edge:?}");
        assert_eq!(driver.evacuation(HeroId::Donut, hero(587.0), None), None);
        // Inside, nearer the left: out to the left, a body clear of the lane.
        let out = driver
            .evacuation(HeroId::Donut, hero(620.0), None)
            .expect("a way out");
        assert!(out.x + HERO_HALF_WIDTH <= 600.0 && out.x > 500.0, "{out:?}");
        // The camera bounds are a preference: with none left on its side it still
        // leaves through the other side rather than staying in.
        let bounded = driver
            .evacuation(HeroId::Donut, hero(620.0), Some((650.0, 760.0)))
            .expect("bounds are dropped before the hazard is kept");
        assert!(bounded.x >= 680.0 - 1.0 || bounded.x + HERO_HALF_WIDTH <= 600.0);
        // In the air: nothing (the transit or the fall decides).
        let mut air = hero(620.0);
        air.grounded = false;
        assert_eq!(driver.evacuation(HeroId::Donut, air, None), None);
        // Driven through the real body: she walks out in the lane's fewest ticks.
        let mut hero_body = Hero::at(620.0, HERO_HALF_HEIGHT);
        let mut frame = InputFrame::default();
        for _ in 0..90 {
            let me = body(&hero_body);
            let Some(goal) = driver.evacuation(HeroId::Donut, me, None) else {
                break;
            };
            let (drive, _) = driver.go_to(HeroId::Donut, me, goal, None);
            let move_x = drive.map_or_else(|| approach(me.x, goal.x, RUN_SPEED), |d| d.move_x);
            frame.advance(RawInput {
                move_x,
                ..RawInput::NONE
            });
            step_hero(&mut hero_body, &frame, &blocks, 2000.0);
        }
        assert!(
            hero_body.x + HERO_HALF_WIDTH <= 600.0 + 0.01 || hero_body.x - HERO_HALF_WIDTH >= 680.0,
            "out of the lane: {}",
            hero_body.x
        );
    }

    /// CA2-FU-HAZ U-2: an evacuation never ends with her body over a hazard's edge
    /// when a wider safe anchor exists. Between two lanes 20 px apart (too narrow
    /// for her 24 px body plus the fence margins) the nearest safe anchor is that
    /// gap, whose hold span is empty; the evacuation must go on to the open ground
    /// beyond instead of parking in the gap.
    #[test]
    fn an_evacuation_never_parks_on_an_anchor_too_narrow_for_her_body() {
        let blocks = flat();
        let mut driver = NavDriver::new(&blocks, NavConfig::integrated()).unwrap();
        driver
            .set_hazards(vec![lane(600.0, 80.0, 1), lane(700.0, 80.0, 2)], &blocks)
            .unwrap();
        for x in [610.0, 640.0, 670.0, 720.0, 760.0] {
            let me = standing(x, 0.0);
            let goal = driver
                .evacuation(HeroId::Donut, me, None)
                .unwrap_or_else(|| panic!("{x}: a way out"));
            let body = Rect {
                left: goal.x - HERO_HALF_WIDTH,
                bottom: 0.0,
                width: 2.0 * HERO_HALF_WIDTH,
                height: 2.0 * HERO_HALF_HEIGHT,
            };
            assert!(
                !driver.hazards().iter().any(|h| h.rect.overlaps(body)),
                "{x}: the goal {goal:?} leaves her body on a hazard"
            );
        }
    }

    #[test]
    fn every_hazard_class_is_just_a_rectangle_to_the_nav() {
        use crate::camera_lab::room::IRect;
        use crate::env_hazard::{
            HazardClass, HazardSpec, HazardVolume, HazardWindow, NavPhase, Phase,
        };
        use crate::hero_damage::HeroDamagePolicy;
        let blocks = flat();
        let layout_h = 100;
        // Steam, a footprint and a blast are timed; acid, a slide and a wheel are
        // persistent. All six block the same route in the same way.
        let mut routes = Vec::new();
        for class in HazardClass::ALL {
            let window = match class {
                HazardClass::Steam | HazardClass::FallingMass | HazardClass::Blast => {
                    HazardWindow::Timed {
                        telegraph_ticks: 24,
                        active_ticks: 30,
                        max_activations: 1,
                    }
                }
                _ => HazardWindow::Persistent,
            };
            let rect = IRect::new(600, 40, 80, 60);
            let mut volume = HazardVolume::new(HazardSpec {
                id: 1,
                class,
                rect,
                telegraph_rect: rect,
                window,
                damage: 1,
                push_x: 0,
                hero_damage: HeroDamagePolicy::AllHeroes,
                hurts_enemies: false,
            })
            .unwrap();
            let timed = !volume.spec().is_persistent();
            if timed {
                assert!(volume.trigger(None));
            }
            let mut driver = NavDriver::new(&blocks, NavConfig::agile()).unwrap();
            let goal = standing(900.0, 0.0);
            let mut held_ticks = 0u32;
            let mut phases = Vec::new();
            for _ in 0..80 {
                let feed: Vec<NavRect> = volume.nav_rect().into_iter().collect();
                phases.push(feed.first().map(|n| n.phase));
                driver
                    .set_hazards(hazards_from_feed(&feed, layout_h), &blocks)
                    .unwrap();
                let (drive, notes) = driver.go_to(HeroId::Donut, standing(300.0, 0.0), goal, None);
                let blocked = notes.iter().any(|n| {
                    matches!(
                        n,
                        NavNote::Waiting {
                            reason: Reason::Hazard,
                            ..
                        }
                    )
                }) || drive.is_some();
                held_ticks += u32::from(blocked);
                volume.step();
            }
            if timed {
                // Telegraph (24) then active (30): blocked on both, free after.
                assert_eq!(phases[0], Some(NavPhase::Telegraph), "{class:?}");
                assert!(phases.contains(&Some(NavPhase::Active)), "{class:?}");
                assert_eq!(phases[79], None, "{class:?}: ended");
                assert_eq!(volume.phase(), Phase::Spent);
                assert!((1..80).contains(&held_ticks), "{class:?}: {held_ticks}");
            } else {
                assert!(phases.iter().all(|p| *p == Some(NavPhase::Active)));
                assert_eq!(held_ticks, 80, "{class:?}: a persistent hazard keeps");
            }
            routes.push((class, driver.hazards().len()));
        }
        // No class is special: nothing but the rectangle decided any of it.
        assert_eq!(routes.len(), HazardClass::ALL.len());
    }

    #[test]
    fn nav_spots_see_a_hazard_over_a_spot_and_on_a_pounce_run() {
        use crate::companion::autonomy::SpotWorld;
        let blocks = flat();
        let mut driver = NavDriver::new(&blocks, NavConfig::agile()).unwrap();
        let world = |d: &NavDriver| NavSpots::new(d, HeroId::Donut, 16.0).hazard_at(640.0, 0.0);
        assert!(!world(&driver));
        driver
            .set_hazards(vec![lane(600.0, 80.0, 1)], &blocks)
            .unwrap();
        let spots = NavSpots::new(&driver, HeroId::Donut, 16.0);
        assert!(spots.hazard_at(640.0, 0.0), "her body over the lane");
        assert!(spots.hazard_at(590.0, 0.0), "the body's width counts");
        assert!(!spots.hazard_at(560.0, 0.0), "clear of it");
        assert!(!spots.hazard_at(640.0, 200.0), "a lane far below a perch");
        assert!(spots.hazard_between(520.0, 700.0, 0.0), "a run across it");
        assert!(spots.hazard_between(700.0, 520.0, 0.0), "either way");
        assert!(
            !spots.hazard_between(300.0, 560.0, 0.0),
            "a run short of it"
        );
        // No hazard surface is offered as a spot: the lane's own anchor is risky.
        let mut surfaces = Vec::new();
        spots.surfaces(&mut surfaces);
        assert!(
            surfaces.iter().any(|s| s.risk >= UNSAFE_RISK),
            "the lane's anchor carries the risk the goal layer filters on"
        );
        assert!(
            surfaces
                .iter()
                .filter(|s| s.risk < UNSAFE_RISK)
                .all(|s| s.right <= 600.0 - HERO_HALF_WIDTH || s.left >= 680.0 + HERO_HALF_WIDTH),
            "every safe spot leaves her body clear of the lane"
        );
    }
}
