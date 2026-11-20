//! COMPANION-AI-01 stage 01B-core: a pure, Bevy-free platform navigation graph
//! (CA-11, CA-33).
//!
//! The graph is built from read-only level data (static solids, authored
//! hazards) and is **capability-aware**: every edge is judged separately for
//! Carl and for Donut by running the real hero body step
//! ([`crate::prototype::step_hero_driven`]) with that hero's launch speed and
//! run speed, so there is no generic leap that ignores the physics.
//! Carl's speeds are the prototype constants; Donut's jump comes from the
//! camera-lab room data ([`DONUT_MOBILITY`]), the only place a Donut-only
//! perch exists on main today (CA-33). No gate gameplay is invented here: the
//! only authored capability data are [`HeroMask`] (who may stand on a solid's
//! top) and [`Unlocks`] (what an actor must have unlocked to use it), and both
//! default to "everyone, nothing".
//!
//! Model:
//! * **Anchors** are standable intervals of a solid's top face. A solid's top
//!   is cut at every x boundary of anything that blocks the 52 px of head room
//!   above it (or a hazard), so an anchor is either fully standable or fully
//!   blocked. The cut set depends only on the as-built geometry, so an
//!   [`NavAnchorId`] (`solid`, `slot`) stays stable when solids are destroyed.
//! * **Edges** are walk, jump or drop transits between two anchors. The
//!   candidate set is static; each candidate stores one [`EdgeState`] per hero
//!   (plus the as-built state, for [`Reason::Destroyed`]) and the solids its
//!   transit corridor touches (`deps`).
//! * **Destruction** ([`NavGraph::set_solid_intact`]) re-evaluates only the
//!   edges whose `deps` contain the solid, then bumps [`NavGraph::revision`].
//! * **Queries** ([`NavGraph::can_reach`], [`NavGraph::path_or_reason`],
//!   [`NavGraph::safe_anchor`], [`NavGraph::plan`]) are deterministic Dijkstra
//!   runs with stable tie breaks, and return a typed [`Reason`] when there is
//!   no route.
//!
//! Determinism: integer costs and risks, no `HashMap`, solids sorted by id at
//! build, no randomness. [`NavGraph::state_hash`] and
//! [`NavGraph::canonical_dump`] are identical for identical input.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::camera_lab::room::{DONUT_MOBILITY, PropKind, Room};
use crate::input::InputFrame;
use crate::party::HeroId;
use crate::prototype::{
    Block, BodyDrive, GRAVITY, HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Hero, JUMP_SPEED, Material,
    RUN_SPEED, Rect, TICK_SECONDS, step_hero_driven,
};

/// Head room a hero needs above a standing surface.
pub const HEAD_ROOM_PX: f32 = HERO_HALF_HEIGHT * 2.0;
/// Highest risk value; a hazard-covered anchor carries it.
pub const RISK_MAX: u16 = 1000;
/// Anchors and edges at or above this risk are unsafe to stop on or traverse.
pub const UNSAFE_RISK: u16 = 500;
/// Risk per 4 px of drop, capped at [`DROP_RISK_CAP`].
const DROP_RISK_PER_4PX: u16 = 1;
const DROP_RISK_CAP: u16 = 200;
/// Risk of one jump transit (it commits the actor to the air).
const JUMP_RISK: u16 = 10;
/// Risk per simulated tick spent inside a hazard.
const HAZARD_TICK_RISK: u16 = 50;
/// Standing inset from an anchor end for takeoff and landing targets.
const EDGE_INSET_PX: f32 = 2.0;
/// Longest simulated transit.
const SIM_MAX_TICKS: u32 = 150;
/// A grounded actor that has not left the ground by this tick is stuck.
const SIM_STUCK_TICKS: u32 = 20;
/// Landing tolerance on the surface height and the anchor span.
const LAND_EPS_PX: f32 = 0.6;
/// Slack of the analytic candidate filter (the simulation decides).
const CANDIDATE_SLACK_PX: f32 = 8.0;
/// Anchors narrower than this are not kept.
const MIN_ANCHOR_WIDTH_PX: f32 = 1.0;

// ------------------------------------------------------------------ ids ----

/// Stable id of a solid in the input level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SolidId(pub u32);

/// Stable anchor id: the solid whose top face it stands on and its ordinal in
/// that solid's as-built partition. Never reused, survives destruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NavAnchorId {
    pub solid: SolidId,
    pub slot: u16,
}

/// Ability unlocks an actor must hold to use an anchor (authored data; none are
/// defined by the graph itself).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Unlocks(pub u32);

impl Unlocks {
    pub const NONE: Self = Self(0);
    pub const ALL: Self = Self(u32::MAX);

    pub const fn contains(self, need: Self) -> bool {
        self.0 & need.0 == need.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn missing(self, have: Self) -> Self {
        Self(self.0 & !have.0)
    }
}

/// Which heroes may stand on a solid's top (a `donut_only` perch is
/// [`HeroMask::DONUT`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeroMask(u8);

impl HeroMask {
    pub const CARL: Self = Self(1);
    pub const DONUT: Self = Self(2);
    pub const BOTH: Self = Self(3);

    pub const fn allows(self, hero: HeroId) -> bool {
        self.0 & (1 << hero.index()) != 0
    }
}

// ---------------------------------------------------------------- input ----

/// One static solid of the level (y up, px).
#[derive(Debug, Clone, PartialEq)]
pub struct NavSolid {
    pub id: SolidId,
    pub rect: Rect,
    /// Can be destroyed at runtime (see [`NavGraph::set_solid_intact`]).
    pub destructible: bool,
    /// As-built state; a solid that starts destroyed is allowed.
    pub intact: bool,
    pub access: HeroMask,
    pub requires: Unlocks,
}

impl NavSolid {
    pub fn new(id: u32, rect: Rect) -> Self {
        Self {
            id: SolidId(id),
            rect,
            destructible: false,
            intact: true,
            access: HeroMask::BOTH,
            requires: Unlocks::NONE,
        }
    }
}

/// A damaging region (y up, px): anchors under it are unsafe.
#[derive(Debug, Clone, PartialEq)]
pub struct NavHazard {
    pub id: u32,
    pub rect: Rect,
}

/// Everything the graph reads. Plain data: no handles into the sim.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NavLevel {
    pub solids: Vec<NavSolid>,
    pub hazards: Vec<NavHazard>,
}

impl NavLevel {
    /// The shared hero-physics input: the greybox blocks or
    /// `EnvKit::blocks()` of a Floor 01 layout. Ids are the slice indices.
    pub fn from_blocks(blocks: &[Block], hazards: Vec<NavHazard>) -> Self {
        let solids = blocks
            .iter()
            .enumerate()
            .map(|(i, b)| NavSolid {
                destructible: b.material.destructible(),
                intact: b.intact,
                ..NavSolid::new(i as u32, b.rect)
            })
            .collect();
        Self { solids, hazards }
    }

    /// A camera-lab room (y down, integer px): the floor slab, the two wall
    /// columns, the platforms (the not-`carl_access` ones become
    /// [`HeroMask::DONUT`], CA-33), the access steps and the solid props.
    /// Gates and hanging weights are not static solids and are skipped.
    pub fn from_camera_lab_room(room: &Room) -> Self {
        let up = |x: i32, y: i32, w: i32, h: i32| Rect {
            left: x as f32,
            bottom: -(y + h) as f32,
            width: w as f32,
            height: h as f32,
        };
        let (w, h) = room.render;
        let wall = crate::camera_lab::room::WALL_PX;
        let mut rects: Vec<(Rect, bool, HeroMask)> = vec![
            (
                up(0, room.floor_y, w, (h - room.floor_y).max(wall)),
                false,
                HeroMask::BOTH,
            ),
            (up(0, 0, wall, room.floor_y), false, HeroMask::BOTH),
            (up(w - wall, 0, wall, room.floor_y), false, HeroMask::BOTH),
        ];
        for p in &room.platforms {
            let mask = if p.carl_access {
                HeroMask::BOTH
            } else {
                HeroMask::DONUT
            };
            rects.push((up(p.rect.x, p.rect.y, p.rect.w, p.rect.h), false, mask));
        }
        for s in &room.steps {
            rects.push((up(s.x, s.y, s.w, s.h), false, HeroMask::BOTH));
        }
        for p in room
            .props
            .iter()
            .filter(|p| p.kind.solid() && p.kind != PropKind::Gate)
        {
            rects.push((
                up(p.rect.x, p.rect.y, p.rect.w, p.rect.h),
                p.breakable,
                HeroMask::BOTH,
            ));
        }
        let solids = rects
            .into_iter()
            .enumerate()
            .map(|(i, (rect, destructible, access))| NavSolid {
                destructible,
                access,
                ..NavSolid::new(i as u32, rect)
            })
            .collect();
        let hazards = room
            .hazards
            .iter()
            .enumerate()
            .map(|(i, z)| NavHazard {
                id: i as u32,
                rect: up(z.rect.x, z.rect.y, z.rect.w, z.rect.h),
            })
            .collect();
        Self { solids, hazards }
    }
}

/// A hero's launch and run speed (px/s). Gravity is the shared
/// [`GRAVITY`] of the hero body step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeroPhysics {
    pub jump_speed: f32,
    pub run_speed: f32,
}

impl HeroPhysics {
    /// Carl: the prototype body constants.
    pub fn carl() -> Self {
        Self {
            jump_speed: JUMP_SPEED,
            run_speed: RUN_SPEED,
        }
    }

    /// Donut: the launch speed that gives the camera-lab Donut's authored
    /// jump rise ([`DONUT_MOBILITY`]); the lab's own speed constants are
    /// private, so the run speed is the shared body's.
    pub fn donut() -> Self {
        Self {
            jump_speed: (2.0 * GRAVITY * DONUT_MOBILITY.jump_h as f32).sqrt(),
            run_speed: RUN_SPEED,
        }
    }

    fn apex_px(self) -> f32 {
        self.jump_speed * self.jump_speed / (2.0 * GRAVITY)
    }

    /// Horizontal reach of a jump that lands `rise` px above the takeoff.
    fn reach_px(self, rise: f32) -> Option<f32> {
        let disc = self.jump_speed * self.jump_speed - 2.0 * GRAVITY * rise;
        (disc >= 0.0).then(|| self.run_speed * (self.jump_speed + disc.sqrt()) / GRAVITY)
    }
}

/// Per-hero physics the edge capabilities are derived from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NavConfig {
    pub carl: HeroPhysics,
    pub donut: HeroPhysics,
}

impl Default for NavConfig {
    fn default() -> Self {
        Self {
            carl: HeroPhysics::carl(),
            donut: HeroPhysics::donut(),
        }
    }
}

impl NavConfig {
    fn physics(&self, hero: HeroId) -> HeroPhysics {
        match hero {
            HeroId::Carl => self.carl,
            HeroId::Donut => self.donut,
        }
    }
}

// --------------------------------------------------------------- results ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    Walk,
    Jump,
    Drop,
}

/// Why an edge is closed for one hero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockCause {
    /// The hero's body cannot make the transit, or may not stand there.
    Capability,
    /// Something in the way, or the anchor is blocked overhead.
    Obstructed,
    /// An endpoint's solid is destroyed.
    Gone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeState {
    Open {
        kind: EdgeKind,
        /// Transit time in 60 Hz ticks (the simulated jump or drop; a walk is
        /// the centre-to-centre run). Walking along an anchor is not counted.
        ticks: u16,
        risk: u16,
    },
    Blocked(BlockCause),
}

/// Why there is no (safe) route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// No hero could route there, now or as built.
    Unreachable,
    /// Another hero could (`hero`), or this actor could with `missing`
    /// unlocks.
    Capability {
        hero: Option<HeroId>,
        missing: Unlocks,
    },
    /// An anchor is gone, or the route existed before a destruction.
    Destroyed,
    /// Only an unsafe (hazard) route exists, or the destination is unsafe.
    Hazard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    pub anchors: Vec<NavAnchorId>,
    pub ticks: u32,
    pub max_risk: u16,
    /// Unlocks the route uses (all held by the asker).
    pub needs: Unlocks,
}

/// Where the actor is and where it wants to go, for [`NavGraph::safe_anchor`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SafeContext {
    pub here: NavAnchorId,
    pub goal: Option<NavAnchorId>,
    pub unlocks: Unlocks,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    Route(Path),
    /// No route: wait at this safe, reachable anchor and report the reason.
    Wait {
        anchor: NavAnchorId,
        reason: Reason,
    },
    /// No safe anchor is reachable at all.
    Stranded(Reason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavError {
    DuplicateSolid(SolidId),
    BadGeometry(SolidId),
    UnknownSolid(SolidId),
    NotDestructible(SolidId),
}

/// What one destruction changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeReport {
    /// The graph revision after the change (unchanged when nothing flipped).
    pub revision: u32,
    pub edge_count: usize,
    pub recomputed_edges: usize,
    /// Edges (from, to) whose state differs for at least one hero.
    pub changed_edges: Vec<(NavAnchorId, NavAnchorId)>,
}

/// A read-only edge snapshot for tests and debug traces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeView {
    pub from: NavAnchorId,
    pub to: NavAnchorId,
    pub state: [EdgeState; 2],
    pub as_built: [EdgeState; 2],
    pub rev: u32,
    /// Solids whose destruction re-evaluates this edge (sorted).
    pub deps: Vec<SolidId>,
}

// ----------------------------------------------------------------- graph ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeStatus {
    Open,
    /// A blocker above it is intact.
    Closed,
    /// Its own solid is destroyed.
    Gone,
}

#[derive(Debug, Clone)]
struct Node {
    anchor: NavAnchorId,
    solid: u32,
    x0: f32,
    x1: f32,
    top: f32,
    blockers: Vec<u32>,
    risk: u16,
    status: NodeStatus,
    as_built: NodeStatus,
}

impl Node {
    fn mid(&self) -> f32 {
        (self.x0 + self.x1) * 0.5
    }
}

#[derive(Debug, Clone)]
struct Edge {
    from: u32,
    to: u32,
    walk: bool,
    /// Solid indices the transit corridor touches (sorted).
    deps: Vec<u32>,
    state: [EdgeState; 2],
    as_built: [EdgeState; 2],
    rev: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Current,
    AsBuilt,
}

#[derive(Debug, Clone, Copy)]
struct SearchOpts {
    actor: HeroId,
    unlocks: Unlocks,
    mode: Mode,
    allow_hazard: bool,
}

struct Search {
    dist: Vec<u32>,
    prev: Vec<u32>,
    via: Vec<u32>,
}

const NONE: u32 = u32::MAX;

/// The platform navigation graph. See the module docs.
#[derive(Debug, Clone)]
pub struct NavGraph {
    config: NavConfig,
    solids: Vec<NavSolid>,
    hazards: Vec<NavHazard>,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    out: Vec<Vec<u32>>,
    revision: u32,
    max_x: f32,
}

impl NavGraph {
    /// Build the graph. Solids are ordered by id; two builds of equal input are
    /// identical.
    pub fn build(level: &NavLevel, config: NavConfig) -> Result<Self, NavError> {
        let mut solids = level.solids.clone();
        solids.sort_by_key(|s| s.id);
        for pair in solids.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(NavError::DuplicateSolid(pair[0].id));
            }
        }
        for s in &solids {
            let r = s.rect;
            let finite = [r.left, r.bottom, r.width, r.height]
                .iter()
                .all(|v| v.is_finite());
            if !finite || r.width <= 0.0 || r.height <= 0.0 {
                return Err(NavError::BadGeometry(s.id));
            }
        }
        let mut hazards = level.hazards.clone();
        hazards.sort_by_key(|h| h.id);
        let max_x = solids.iter().map(|s| s.rect.right()).fold(0.0, f32::max) + 64.0;
        let mut graph = Self {
            config,
            solids,
            hazards,
            nodes: Vec::new(),
            edges: Vec::new(),
            out: Vec::new(),
            revision: 0,
            max_x,
        };
        graph.build_nodes();
        graph.build_edges();
        Ok(graph)
    }

    /// Bumped on every destruction (or restoration) that flips a solid.
    pub fn revision(&self) -> u32 {
        self.revision
    }

    pub fn anchor_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn solid_intact(&self, id: SolidId) -> Option<bool> {
        self.solid_index(id).map(|i| self.solids[i].intact)
    }

    /// Anchors in stable (id) order.
    pub fn anchors(&self) -> impl Iterator<Item = NavAnchorId> + '_ {
        self.nodes.iter().map(|n| n.anchor)
    }

    /// Centre x and standing height (y up) of an anchor.
    pub fn anchor_pos(&self, id: NavAnchorId) -> Option<(f32, f32)> {
        self.node_index(id)
            .map(|i| (self.nodes[i as usize].mid(), self.nodes[i as usize].top))
    }

    /// Left and right x of an anchor.
    pub fn anchor_span(&self, id: NavAnchorId) -> Option<(f32, f32)> {
        self.node_index(id)
            .map(|i| (self.nodes[i as usize].x0, self.nodes[i as usize].x1))
    }

    pub fn anchor_risk(&self, id: NavAnchorId) -> Option<u16> {
        self.node_index(id).map(|i| self.nodes[i as usize].risk)
    }

    /// Is the anchor standable right now?
    pub fn anchor_open(&self, id: NavAnchorId) -> bool {
        self.node_index(id)
            .is_some_and(|i| self.nodes[i as usize].status == NodeStatus::Open)
    }

    /// The lowest-id open anchor supporting a hero whose feet are at
    /// (`x`, `foot_y`), within 4 px of the surface.
    pub fn anchor_at(&self, x: f32, foot_y: f32) -> Option<NavAnchorId> {
        self.nodes
            .iter()
            .find(|n| {
                n.status == NodeStatus::Open
                    && x >= n.x0
                    && x <= n.x1
                    && (n.top - foot_y).abs() <= 4.0
            })
            .map(|n| n.anchor)
    }

    /// Every edge in stable order.
    pub fn edge_views(&self) -> Vec<EdgeView> {
        self.edges
            .iter()
            .map(|e| EdgeView {
                from: self.nodes[e.from as usize].anchor,
                to: self.nodes[e.to as usize].anchor,
                state: e.state,
                as_built: e.as_built,
                rev: e.rev,
                deps: e.deps.iter().map(|&d| self.solids[d as usize].id).collect(),
            })
            .collect()
    }

    /// The state of one edge for one hero.
    pub fn edge_state(
        &self,
        from: NavAnchorId,
        to: NavAnchorId,
        hero: HeroId,
    ) -> Option<EdgeState> {
        let (a, b) = (self.node_index(from)?, self.node_index(to)?);
        self.edges
            .iter()
            .find(|e| e.from == a && e.to == b)
            .map(|e| e.state[hero.index()])
    }

    // ------------------------------------------------------------ build ----

    fn solid_index(&self, id: SolidId) -> Option<usize> {
        self.solids.binary_search_by_key(&id, |s| s.id).ok()
    }

    fn node_index(&self, id: NavAnchorId) -> Option<u32> {
        self.nodes
            .binary_search_by_key(&id, |n| n.anchor)
            .ok()
            .map(|i| i as u32)
    }

    fn build_nodes(&mut self) {
        let mut nodes = Vec::new();
        for (si, s) in self.solids.iter().enumerate() {
            let (l, r, top) = (s.rect.left, s.rect.right(), s.rect.top());
            let mut cuts = vec![l, r];
            let mut blockers: Vec<u32> = Vec::new();
            for (oi, o) in self.solids.iter().enumerate() {
                if oi != si && blocks_head(o.rect, l, r, top) {
                    blockers.push(oi as u32);
                    cuts.push(o.rect.left.clamp(l, r));
                    cuts.push(o.rect.right().clamp(l, r));
                }
            }
            let mut hazard_spans: Vec<(f32, f32)> = Vec::new();
            for h in &self.hazards {
                if blocks_head(h.rect, l, r, top) {
                    hazard_spans.push((h.rect.left, h.rect.right()));
                    cuts.push(h.rect.left.clamp(l, r));
                    cuts.push(h.rect.right().clamp(l, r));
                }
            }
            cuts.sort_by(f32::total_cmp);
            cuts.dedup();
            let mut slot: u16 = 0;
            for w in cuts.windows(2) {
                let (x0, x1) = (w[0], w[1]);
                if x1 - x0 < MIN_ANCHOR_WIDTH_PX {
                    continue;
                }
                let piece_blockers: Vec<u32> = blockers
                    .iter()
                    .copied()
                    .filter(|&b| {
                        let o = self.solids[b as usize].rect;
                        o.left < x1 && o.right() > x0
                    })
                    .collect();
                let risk = if hazard_spans.iter().any(|&(hl, hr)| hl < x1 && hr > x0) {
                    RISK_MAX
                } else {
                    0
                };
                nodes.push(Node {
                    anchor: NavAnchorId { solid: s.id, slot },
                    solid: si as u32,
                    x0,
                    x1,
                    top,
                    blockers: piece_blockers,
                    risk,
                    status: NodeStatus::Open,
                    as_built: NodeStatus::Open,
                });
                slot = slot.saturating_add(1);
            }
        }
        for n in &mut nodes {
            n.status = node_status(&self.solids, n);
            n.as_built = n.status;
        }
        self.nodes = nodes;
    }

    fn max_apex(&self) -> f32 {
        self.config.carl.apex_px().max(self.config.donut.apex_px())
    }

    /// Analytic upper bound used only to pick candidate pairs.
    fn candidate_gap_limit(&self, rise: f32) -> Option<f32> {
        let a = self.config.carl.reach_px(rise);
        let b = self.config.donut.reach_px(rise);
        match (a, b) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        }
        .map(|g| g + CANDIDATE_SLACK_PX)
    }

    fn build_edges(&mut self) {
        let mut edges = Vec::new();
        let apex = self.max_apex() + 2.0;
        for (i, a) in self.nodes.iter().enumerate() {
            for (j, b) in self.nodes.iter().enumerate() {
                if i == j {
                    continue;
                }
                let gap = (b.x0 - a.x1).max(a.x0 - b.x1).max(0.0);
                let rise = b.top - a.top;
                let walk = rise.abs() <= 0.5 && gap <= 0.5;
                if !walk {
                    if rise > apex {
                        continue;
                    }
                    match self.candidate_gap_limit(rise) {
                        Some(limit) if gap <= limit => {}
                        _ => continue,
                    }
                }
                let bbox = Rect {
                    left: a.x0.min(b.x0) - HERO_HALF_WIDTH - 4.0,
                    bottom: a.top.min(b.top) - 2.0,
                    width: a.x1.max(b.x1) - a.x0.min(b.x0) + 2.0 * (HERO_HALF_WIDTH + 4.0),
                    height: a.top.max(b.top) - a.top.min(b.top) + apex + HEAD_ROOM_PX + 6.0,
                };
                let mut deps: Vec<u32> = self
                    .solids
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.rect.overlaps(bbox))
                    .map(|(k, _)| k as u32)
                    .collect();
                for extra in [a.solid, b.solid]
                    .into_iter()
                    .chain(a.blockers.iter().copied())
                    .chain(b.blockers.iter().copied())
                {
                    if !deps.contains(&extra) {
                        deps.push(extra);
                    }
                }
                deps.sort_unstable();
                edges.push(Edge {
                    from: i as u32,
                    to: j as u32,
                    walk,
                    deps,
                    state: [EdgeState::Blocked(BlockCause::Capability); 2],
                    as_built: [EdgeState::Blocked(BlockCause::Capability); 2],
                    rev: 0,
                });
            }
        }
        for e in &mut edges {
            let state = [
                self.eval_edge(e, HeroId::Carl),
                self.eval_edge(e, HeroId::Donut),
            ];
            e.state = state;
            e.as_built = state;
        }
        let mut out = vec![Vec::new(); self.nodes.len()];
        for (k, e) in edges.iter().enumerate() {
            out[e.from as usize].push(k as u32);
        }
        self.edges = edges;
        self.out = out;
    }

    // ------------------------------------------------------ edge physics ---

    fn eval_edge(&self, e: &Edge, hero: HeroId) -> EdgeState {
        let a = &self.nodes[e.from as usize];
        let b = &self.nodes[e.to as usize];
        if a.status == NodeStatus::Gone || b.status == NodeStatus::Gone {
            return EdgeState::Blocked(BlockCause::Gone);
        }
        if a.status == NodeStatus::Closed || b.status == NodeStatus::Closed {
            return EdgeState::Blocked(BlockCause::Obstructed);
        }
        let allowed = |n: &Node| self.solids[n.solid as usize].access.allows(hero);
        if !allowed(a) || !allowed(b) {
            return EdgeState::Blocked(BlockCause::Capability);
        }
        let phys = self.config.physics(hero);
        if e.walk {
            let ticks = walk_ticks((a.mid() - b.mid()).abs(), phys.run_speed);
            return EdgeState::Open {
                kind: EdgeKind::Walk,
                ticks,
                risk: 0,
            };
        }
        let rise = b.top - a.top;
        let gap = (b.x0 - a.x1).max(a.x0 - b.x1).max(0.0);
        let reachable = rise <= phys.apex_px() + 2.0
            && phys
                .reach_px(rise)
                .is_some_and(|reach| gap <= reach + CANDIDATE_SLACK_PX);
        if !reachable {
            return EdgeState::Blocked(BlockCause::Capability);
        }
        let blocks = self.local_blocks(&e.deps, None);
        if let Some(open) = self.transit(a, b, phys, &blocks) {
            return open;
        }
        // Distinguish "the hero cannot" from "something is in the way".
        let owners = [a.solid, b.solid];
        let others = e.deps.iter().any(|d| !owners.contains(d));
        if others {
            let clear = self.local_blocks(&owners, None);
            if self.transit(a, b, phys, &clear).is_some() {
                return EdgeState::Blocked(BlockCause::Obstructed);
            }
        }
        EdgeState::Blocked(BlockCause::Capability)
    }

    fn local_blocks(&self, deps: &[u32], skip: Option<u32>) -> Vec<Block> {
        deps.iter()
            .filter(|&&d| Some(d) != skip)
            .filter_map(|&d| {
                let s = &self.solids[d as usize];
                s.intact.then_some(Block {
                    rect: s.rect,
                    material: Material::Stone,
                    intact: true,
                })
            })
            .collect()
    }

    /// Try the takeoff variants in a fixed order and return the first transit
    /// the real body step completes.
    fn transit(
        &self,
        a: &Node,
        b: &Node,
        phys: HeroPhysics,
        blocks: &[Block],
    ) -> Option<EdgeState> {
        let rise = b.top - a.top;
        let overlap = a.x0 < b.x1 && a.x1 > b.x0;
        let dirs: &[f32] = if overlap {
            &[1.0, -1.0]
        } else if b.x0 >= a.x1 {
            &[1.0]
        } else {
            &[-1.0]
        };
        let launches: &[bool] = if rise > 0.5 { &[true] } else { &[false, true] };
        for &launch in launches {
            for &dir in dirs {
                // A hero stays supported until its whole body is past the edge,
                // so a drop (or a transit between overlapping spans) aims past
                // it; a jump up onto a ledge aims at the ledge's near edge and,
                // when the lower anchor spans under the ledge, takes off beside
                // the ledge rather than at the far end of the span.
                let clear = if dir > 0.0 {
                    a.x1 + HERO_HALF_WIDTH + 1.0
                } else {
                    a.x0 - HERO_HALF_WIDTH - 1.0
                };
                let (x0, want) = if rise > 0.5 {
                    let near = if dir > 0.0 { b.x0 } else { b.x1 };
                    if overlap {
                        let beside = if dir > 0.0 {
                            b.x0 - HERO_HALF_WIDTH - 1.0
                        } else {
                            b.x1 + HERO_HALF_WIDTH + 1.0
                        };
                        (clamp_inset(beside, a.x0, a.x1), near)
                    } else {
                        (takeoff_x(a, dir), near)
                    }
                } else if dir > 0.0 {
                    (takeoff_x(a, dir), clear.max(b.x0))
                } else {
                    (takeoff_x(a, dir), clear.min(b.x1))
                };
                let x1 = clamp_inset(want, b.x0, b.x1);
                let run = Run {
                    blocks,
                    hazards: &self.hazards,
                    phys,
                    launch,
                    dir,
                    start_x: x0,
                    start_top: a.top,
                    target_x: x1,
                    dst: (b.x0, b.x1, b.top),
                    max_x: self.max_x,
                };
                let Some(done) = simulate(&run) else { continue };
                let ticks = done.ticks as u16;
                let drop = (a.top - b.top).max(0.0);
                let kind = if launch {
                    EdgeKind::Jump
                } else {
                    EdgeKind::Drop
                };
                let mut risk = u32::from(done.hazard_ticks) * u32::from(HAZARD_TICK_RISK);
                risk += if launch { u32::from(JUMP_RISK) } else { 0 };
                risk += ((drop / 4.0) as u32 * u32::from(DROP_RISK_PER_4PX))
                    .min(u32::from(DROP_RISK_CAP));
                return Some(EdgeState::Open {
                    kind,
                    ticks,
                    risk: risk.min(u32::from(RISK_MAX)) as u16,
                });
            }
        }
        None
    }

    // ------------------------------------------------------ destruction ----

    /// Destroy (or restore) a solid and re-evaluate only the edges whose
    /// transit corridor touches it. Bumps the revision when the flag flips.
    pub fn set_solid_intact(
        &mut self,
        id: SolidId,
        intact: bool,
    ) -> Result<ChangeReport, NavError> {
        let si = self.solid_index(id).ok_or(NavError::UnknownSolid(id))?;
        if !intact && !self.solids[si].destructible && self.solids[si].intact {
            return Err(NavError::NotDestructible(id));
        }
        if self.solids[si].intact == intact {
            return Ok(ChangeReport {
                revision: self.revision,
                edge_count: self.edges.len(),
                recomputed_edges: 0,
                changed_edges: Vec::new(),
            });
        }
        self.solids[si].intact = intact;
        self.revision += 1;
        let si = si as u32;
        for k in 0..self.nodes.len() {
            if self.nodes[k].solid == si || self.nodes[k].blockers.contains(&si) {
                self.nodes[k].status = node_status(&self.solids, &self.nodes[k]);
            }
        }
        let mut recomputed = 0;
        let mut changed = Vec::new();
        for k in 0..self.edges.len() {
            if self.edges[k].deps.binary_search(&si).is_err() {
                continue;
            }
            recomputed += 1;
            let state = [
                self.eval_edge(&self.edges[k], HeroId::Carl),
                self.eval_edge(&self.edges[k], HeroId::Donut),
            ];
            if state != self.edges[k].state {
                self.edges[k].state = state;
                self.edges[k].rev = self.revision;
                changed.push((
                    self.nodes[self.edges[k].from as usize].anchor,
                    self.nodes[self.edges[k].to as usize].anchor,
                ));
            }
        }
        Ok(ChangeReport {
            revision: self.revision,
            edge_count: self.edges.len(),
            recomputed_edges: recomputed,
            changed_edges: changed,
        })
    }

    /// [`Self::set_solid_intact`] with `intact = false`.
    pub fn destroy_solid(&mut self, id: SolidId) -> Result<ChangeReport, NavError> {
        self.set_solid_intact(id, false)
    }

    // ---------------------------------------------------------- queries ----

    fn run_search(&self, src: u32, goal: Option<u32>, o: &SearchOpts) -> Search {
        let n = self.nodes.len();
        let mut s = Search {
            dist: vec![u32::MAX; n],
            prev: vec![NONE; n],
            via: vec![NONE; n],
        };
        let mut heap = BinaryHeap::new();
        s.dist[src as usize] = 0;
        heap.push(Reverse((0u32, src)));
        while let Some(Reverse((d, u))) = heap.pop() {
            if d > s.dist[u as usize] {
                continue;
            }
            if Some(u) == goal {
                break;
            }
            for &ei in &self.out[u as usize] {
                let e = &self.edges[ei as usize];
                let state = match o.mode {
                    Mode::Current => e.state[o.actor.index()],
                    Mode::AsBuilt => e.as_built[o.actor.index()],
                };
                let EdgeState::Open { ticks, risk, .. } = state else {
                    continue;
                };
                let to = &self.nodes[e.to as usize];
                if !o.unlocks.contains(self.solids[to.solid as usize].requires) {
                    continue;
                }
                if !o.allow_hazard && (risk >= UNSAFE_RISK || to.risk >= UNSAFE_RISK) {
                    continue;
                }
                let next = d
                    .saturating_add(u32::from(ticks))
                    .saturating_add(u32::from(risk));
                if next < s.dist[e.to as usize] {
                    s.dist[e.to as usize] = next;
                    s.prev[e.to as usize] = u;
                    s.via[e.to as usize] = ei;
                    heap.push(Reverse((next, e.to)));
                }
            }
        }
        s
    }

    fn route(&self, src: u32, dst: u32, o: &SearchOpts) -> Option<Path> {
        let s = self.run_search(src, Some(dst), o);
        if s.dist[dst as usize] == u32::MAX {
            return None;
        }
        let mut chain = vec![dst];
        let (mut ticks, mut max_risk, mut needs) = (0u32, 0u16, Unlocks::NONE);
        let mut at = dst;
        while at != src {
            let e = &self.edges[s.via[at as usize] as usize];
            let state = match o.mode {
                Mode::Current => e.state[o.actor.index()],
                Mode::AsBuilt => e.as_built[o.actor.index()],
            };
            if let EdgeState::Open { ticks: t, risk, .. } = state {
                ticks += u32::from(t);
                max_risk = max_risk.max(risk);
            }
            needs = needs.union(self.solids[self.nodes[at as usize].solid as usize].requires);
            max_risk = max_risk.max(self.nodes[at as usize].risk);
            at = s.prev[at as usize];
            chain.push(at);
        }
        chain.reverse();
        Some(Path {
            anchors: chain
                .iter()
                .map(|&i| self.nodes[i as usize].anchor)
                .collect(),
            ticks,
            max_risk,
            needs,
        })
    }

    /// Cost in ticks of the best safe route, or why there is none.
    pub fn can_reach(
        &self,
        actor: HeroId,
        source: NavAnchorId,
        destination: NavAnchorId,
        unlocks: Unlocks,
    ) -> Result<u32, Reason> {
        self.path_or_reason(actor, source, destination, unlocks)
            .map(|p| p.ticks)
    }

    /// The best safe route for `actor`, or the typed reason there is none.
    pub fn path_or_reason(
        &self,
        actor: HeroId,
        from: NavAnchorId,
        to: NavAnchorId,
        unlocks: Unlocks,
    ) -> Result<Path, Reason> {
        let (Some(a), Some(b)) = (self.node_index(from), self.node_index(to)) else {
            return Err(Reason::Unreachable);
        };
        for n in [a, b] {
            if self.nodes[n as usize].status == NodeStatus::Gone {
                return Err(Reason::Destroyed);
            }
        }
        let base = SearchOpts {
            actor,
            unlocks,
            mode: Mode::Current,
            allow_hazard: false,
        };
        if let Some(path) = self.route(a, b, &base) {
            return Ok(path);
        }
        if self.nodes[b as usize].status == NodeStatus::Closed {
            return Err(Reason::Unreachable);
        }
        let risky = SearchOpts {
            allow_hazard: true,
            ..base
        };
        if self.route(a, b, &risky).is_some() {
            return Err(Reason::Hazard);
        }
        // Could this actor have routed before a destruction?
        let built = SearchOpts {
            mode: Mode::AsBuilt,
            ..risky
        };
        if self.route(a, b, &built).is_some() {
            return Err(Reason::Destroyed);
        }
        let other = actor.other();
        if self
            .route(
                a,
                b,
                &SearchOpts {
                    actor: other,
                    ..risky
                },
            )
            .is_some()
        {
            return Err(Reason::Capability {
                hero: Some(other),
                missing: Unlocks::NONE,
            });
        }
        let all = SearchOpts {
            unlocks: Unlocks::ALL,
            ..risky
        };
        if let Some(p) = self.route(a, b, &all) {
            return Err(Reason::Capability {
                hero: None,
                missing: p.needs.missing(unlocks),
            });
        }
        Err(Reason::Unreachable)
    }

    /// The safe anchor `actor` should stand on: the reachable safe anchor
    /// nearest the goal (or `here` when there is no goal and it is safe, else
    /// the lowest-risk, cheapest one). Ties go to the lower anchor id.
    pub fn safe_anchor(&self, actor: HeroId, ctx: SafeContext) -> Result<NavAnchorId, Reason> {
        let Some(here) = self.node_index(ctx.here) else {
            return Err(Reason::Unreachable);
        };
        if self.nodes[here as usize].status == NodeStatus::Gone {
            return Err(Reason::Destroyed);
        }
        let opts = SearchOpts {
            actor,
            unlocks: ctx.unlocks,
            mode: Mode::Current,
            allow_hazard: false,
        };
        let s = self.run_search(here, None, &opts);
        let goal = ctx
            .goal
            .and_then(|g| self.node_index(g))
            .map(|g| (self.nodes[g as usize].mid(), self.nodes[g as usize].top));
        let mut best: Option<(u64, u16, u32, u32)> = None;
        for (i, n) in self.nodes.iter().enumerate() {
            if s.dist[i] == u32::MAX || n.status != NodeStatus::Open || n.risk >= UNSAFE_RISK {
                continue;
            }
            let near = goal.map_or(0, |(gx, gy)| {
                let dx = (n.mid() - gx).abs();
                let dy = (n.top - gy).abs();
                ((dx + dy * 2.0) * 16.0) as u64
            });
            let key = (near, n.risk, s.dist[i], i as u32);
            if best.is_none_or(|b| key < b) {
                best = Some(key);
            }
        }
        best.map(|(_, _, _, i)| self.nodes[i as usize].anchor)
            .ok_or(Reason::Hazard)
    }

    /// Route, or wait at a safe anchor with the reason.
    pub fn plan(
        &self,
        actor: HeroId,
        from: NavAnchorId,
        to: NavAnchorId,
        unlocks: Unlocks,
    ) -> Plan {
        match self.path_or_reason(actor, from, to, unlocks) {
            Ok(path) => Plan::Route(path),
            Err(reason) => {
                let ctx = SafeContext {
                    here: from,
                    goal: Some(to),
                    unlocks,
                };
                match self.safe_anchor(actor, ctx) {
                    Ok(anchor) => Plan::Wait { anchor, reason },
                    Err(r) => Plan::Stranded(if r == Reason::Hazard { reason } else { r }),
                }
            }
        }
    }

    // ------------------------------------------------------ determinism ----

    /// FNV-1a of the whole state: config, solids, anchors, edges, revision.
    pub fn state_hash(&self) -> u64 {
        let mut h = Fnv::new();
        for line in self.dump_lines() {
            for b in line.bytes() {
                h.byte(b);
            }
            h.byte(b'\n');
        }
        h.0
    }

    /// A stable text form (float bits in hex); byte-identical for equal state.
    pub fn canonical_dump(&self) -> String {
        let mut out = self.dump_lines().join("\n");
        out.push('\n');
        out
    }

    fn dump_lines(&self) -> Vec<String> {
        let f = |v: f32| format!("{:08x}", v.to_bits());
        let mut lines = vec![format!(
            "cfg {} {} {} {} rev {}",
            f(self.config.carl.jump_speed),
            f(self.config.carl.run_speed),
            f(self.config.donut.jump_speed),
            f(self.config.donut.run_speed),
            self.revision
        )];
        for s in &self.solids {
            lines.push(format!(
                "solid {} {} {} {} {} d{} i{} m{} u{}",
                s.id.0,
                f(s.rect.left),
                f(s.rect.bottom),
                f(s.rect.width),
                f(s.rect.height),
                u8::from(s.destructible),
                u8::from(s.intact),
                s.access.0,
                s.requires.0
            ));
        }
        for z in &self.hazards {
            lines.push(format!(
                "hazard {} {} {} {} {}",
                z.id,
                f(z.rect.left),
                f(z.rect.bottom),
                f(z.rect.width),
                f(z.rect.height)
            ));
        }
        for n in &self.nodes {
            lines.push(format!(
                "node {}:{} {} {} {} r{} s{:?} b{:?}",
                n.anchor.solid.0,
                n.anchor.slot,
                f(n.x0),
                f(n.x1),
                f(n.top),
                n.risk,
                n.status,
                n.blockers
            ));
        }
        for e in &self.edges {
            lines.push(format!(
                "edge {} {} w{} rev{} deps{:?} {:?} {:?}",
                e.from,
                e.to,
                u8::from(e.walk),
                e.rev,
                e.deps,
                e.state,
                e.as_built
            ));
        }
        lines
    }
}

// --------------------------------------------------------------- helpers ---

/// Does `o` stand in the head room above a top face `[l, r]` at `top`?
fn blocks_head(o: Rect, l: f32, r: f32, top: f32) -> bool {
    o.top() > top + 0.01 && o.bottom < top + HEAD_ROOM_PX && o.left < r && o.right() > l
}

fn node_status(solids: &[NavSolid], n: &Node) -> NodeStatus {
    if !solids[n.solid as usize].intact {
        NodeStatus::Gone
    } else if n.blockers.iter().any(|&b| solids[b as usize].intact) {
        NodeStatus::Closed
    } else {
        NodeStatus::Open
    }
}

fn walk_ticks(distance: f32, run_speed: f32) -> u16 {
    (distance / run_speed / TICK_SECONDS)
        .ceil()
        .min(f32::from(u16::MAX)) as u16
}

fn clamp_inset(v: f32, l: f32, r: f32) -> f32 {
    if r - l <= 2.0 * EDGE_INSET_PX {
        (l + r) * 0.5
    } else {
        v.clamp(l + EDGE_INSET_PX, r - EDGE_INSET_PX)
    }
}

fn takeoff_x(a: &Node, dir: f32) -> f32 {
    clamp_inset(if dir > 0.0 { a.x1 } else { a.x0 }, a.x0, a.x1)
}

struct Run<'a> {
    blocks: &'a [Block],
    hazards: &'a [NavHazard],
    phys: HeroPhysics,
    launch: bool,
    /// Direction of travel: +1 right, -1 left.
    dir: f32,
    start_x: f32,
    start_top: f32,
    target_x: f32,
    /// Destination span and standing height.
    dst: (f32, f32, f32),
    max_x: f32,
}

struct Done {
    ticks: u32,
    hazard_ticks: u16,
}

/// Run the real hero body step from `start_x` on the source surface towards
/// `target_x`, with an optional launch. Succeeds when the hero lands on the
/// destination surface.
fn simulate(run: &Run) -> Option<Done> {
    let mut hero = Hero::at(run.start_x, run.start_top + HERO_HALF_HEIGHT);
    let input = InputFrame::default();
    let lowest = run.start_top.min(run.dst.2) - 1.0;
    let mut airborne = false;
    let mut hazard_ticks: u16 = 0;
    for tick in 1..=SIM_MAX_TICKS {
        // Climbing onto a ledge, the hero stays beside it until its feet are
        // above the top: moving under the underside would bump its head.
        let mut target = run.target_x;
        if hero.y - HERO_HALF_HEIGHT < run.dst.2 - LAND_EPS_PX {
            target = if run.dir > 0.0 {
                target.min(run.dst.0 - HERO_HALF_WIDTH)
            } else {
                target.max(run.dst.1 + HERO_HALF_WIDTH)
            };
        }
        let vx = ((target - hero.x) / TICK_SECONDS).clamp(-run.phys.run_speed, run.phys.run_speed);
        let drive = BodyDrive {
            launch: (run.launch && tick == 1).then_some((0.0, run.phys.jump_speed)),
            vx: Some(vx),
            ..BodyDrive::default()
        };
        step_hero_driven(&mut hero, &input, run.blocks, run.max_x, &drive);
        if !hero.grounded {
            airborne = true;
        }
        let body = Rect {
            left: hero.x - HERO_HALF_WIDTH,
            bottom: hero.y - HERO_HALF_HEIGHT,
            width: HERO_HALF_WIDTH * 2.0,
            height: HERO_HALF_HEIGHT * 2.0,
        };
        if run.hazards.iter().any(|z| z.rect.overlaps(body)) {
            hazard_ticks = hazard_ticks.saturating_add(1);
        }
        let foot = hero.y - HERO_HALF_HEIGHT;
        if foot < lowest {
            return None;
        }
        if hero.grounded && airborne {
            let on_dst = (foot - run.dst.2).abs() <= LAND_EPS_PX
                && hero.x >= run.dst.0 - LAND_EPS_PX
                && hero.x <= run.dst.1 + LAND_EPS_PX;
            return on_dst.then_some(Done {
                ticks: tick,
                hazard_ticks,
            });
        }
        if !run.launch && !airborne && tick >= SIM_STUCK_TICKS {
            return None;
        }
    }
    None
}

struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn byte(&mut self, b: u8) {
        self.0 ^= u64::from(b);
        self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::RawInput;
    use crate::prototype::step_hero;

    fn solid(id: u32, left: f32, top: f32, width: f32, height: f32) -> NavSolid {
        NavSolid::new(
            id,
            Rect {
                left,
                bottom: top - height,
                width,
                height,
            },
        )
    }

    fn anchor_on(g: &NavGraph, solid: u32) -> NavAnchorId {
        g.anchors()
            .find(|a| a.solid == SolidId(solid) && g.anchor_open(*a))
            .expect("open anchor")
    }

    /// Floor 0 (top 0), ledge 1 (rise 60: both heroes), perch 2 (rise 140:
    /// Donut only, by physics), tower 3 (rise 260: neither).
    fn room() -> NavLevel {
        NavLevel {
            solids: vec![
                solid(0, 0.0, 0.0, 1200.0, 40.0),
                solid(1, 180.0, 60.0, 80.0, 8.0),
                solid(2, 420.0, 140.0, 80.0, 8.0),
                solid(3, 700.0, 260.0, 80.0, 8.0),
            ],
            hazards: Vec::new(),
        }
    }

    fn graph() -> NavGraph {
        NavGraph::build(&room(), NavConfig::default()).expect("build")
    }

    #[test]
    fn donut_derives_the_lab_jump_and_carl_the_prototype_one() {
        let cfg = NavConfig::default();
        assert_eq!(cfg.carl.jump_speed, JUMP_SPEED);
        assert_eq!(cfg.carl.run_speed, RUN_SPEED);
        assert_eq!(cfg.donut.apex_px().round() as i32, DONUT_MOBILITY.jump_h);
        assert!(cfg.carl.apex_px() > 91.0 && cfg.carl.apex_px() < 93.0);
    }

    #[test]
    fn an_ordinary_ledge_is_reachable_by_both_heroes() {
        let g = graph();
        let floor = anchor_on(&g, 0);
        let ledge = anchor_on(&g, 1);
        for hero in HeroId::ALL {
            let up = g.can_reach(hero, floor, ledge, Unlocks::NONE);
            assert!(up.is_ok(), "{hero:?} up: {up:?}");
            let down = g.can_reach(hero, ledge, floor, Unlocks::NONE);
            assert!(down.is_ok(), "{hero:?} down: {down:?}");
        }
        let path = g
            .path_or_reason(HeroId::Carl, floor, ledge, Unlocks::NONE)
            .unwrap();
        assert_eq!(path.anchors.first(), Some(&floor));
        assert_eq!(path.anchors.last(), Some(&ledge));
        assert_eq!(path.max_risk, JUMP_RISK);
    }

    #[test]
    fn carl_cannot_reach_a_donut_only_perch_and_donut_can() {
        let g = graph();
        let floor = anchor_on(&g, 0);
        let perch = anchor_on(&g, 2);
        assert!(
            g.can_reach(HeroId::Donut, floor, perch, Unlocks::NONE)
                .is_ok()
        );
        assert_eq!(
            g.can_reach(HeroId::Carl, floor, perch, Unlocks::NONE),
            Err(Reason::Capability {
                hero: Some(HeroId::Donut),
                missing: Unlocks::NONE
            })
        );
        // No edge of Carl's lands on the perch, from anywhere.
        for e in g.edge_views().into_iter().filter(|e| e.to == perch) {
            assert!(
                matches!(e.state[HeroId::Carl.index()], EdgeState::Blocked(_)),
                "{e:?}"
            );
        }
    }

    #[test]
    fn an_authored_donut_only_mask_closes_carl_edges_even_where_physics_allows() {
        let mut level = room();
        level.solids[1].access = HeroMask::DONUT; // the rise-60 ledge
        let g = NavGraph::build(&level, NavConfig::default()).unwrap();
        let (floor, ledge) = (anchor_on(&g, 0), anchor_on(&g, 1));
        assert!(
            g.can_reach(HeroId::Donut, floor, ledge, Unlocks::NONE)
                .is_ok()
        );
        assert!(matches!(
            g.can_reach(HeroId::Carl, floor, ledge, Unlocks::NONE),
            Err(Reason::Capability {
                hero: Some(HeroId::Donut),
                ..
            })
        ));
    }

    #[test]
    fn an_unreachable_ledge_gives_a_safe_wait_anchor_and_a_reason() {
        let g = graph();
        let floor = anchor_on(&g, 0);
        let tower = anchor_on(&g, 3);
        for hero in HeroId::ALL {
            assert_eq!(
                g.can_reach(hero, floor, tower, Unlocks::NONE),
                Err(Reason::Unreachable)
            );
            let plan = g.plan(hero, floor, tower, Unlocks::NONE);
            let Plan::Wait { anchor, reason } = plan else {
                panic!("{hero:?}: {plan:?}");
            };
            assert_eq!(reason, Reason::Unreachable);
            // Nearest safe anchor to the tower that the hero can stand on.
            assert!(g.anchor_open(anchor));
            assert!(g.anchor_risk(anchor).unwrap() < UNSAFE_RISK);
            assert!(g.can_reach(hero, floor, anchor, Unlocks::NONE).is_ok());
        }
        // Carl waits under the perch Donut can reach, on the floor or ledge,
        // never on the perch.
        let Plan::Wait { anchor, .. } =
            g.plan(HeroId::Carl, floor, anchor_on(&g, 2), Unlocks::NONE)
        else {
            panic!("carl perch plan");
        };
        assert_ne!(anchor.solid, SolidId(2));
    }

    #[test]
    fn a_hazard_anchor_is_unsafe_and_reported_as_hazard() {
        let mut level = room();
        level.hazards.push(NavHazard {
            id: 7,
            rect: Rect {
                left: 150.0,
                bottom: 60.0,
                width: 140.0,
                height: 30.0,
            },
        });
        let g = NavGraph::build(&level, NavConfig::default()).unwrap();
        let (floor, ledge) = (anchor_on(&g, 0), anchor_on(&g, 1));
        assert_eq!(g.anchor_risk(ledge), Some(RISK_MAX));
        assert_eq!(
            g.can_reach(HeroId::Carl, floor, ledge, Unlocks::NONE),
            Err(Reason::Hazard)
        );
        let Plan::Wait { anchor, reason } = g.plan(HeroId::Carl, floor, ledge, Unlocks::NONE)
        else {
            panic!("hazard plan");
        };
        assert_eq!(reason, Reason::Hazard);
        assert!(g.anchor_risk(anchor).unwrap() < UNSAFE_RISK);
    }

    #[test]
    fn a_missing_unlock_is_a_capability_reason() {
        let mut level = room();
        level.solids[1].requires = Unlocks(0b10);
        let g = NavGraph::build(&level, NavConfig::default()).unwrap();
        let (floor, ledge) = (anchor_on(&g, 0), anchor_on(&g, 1));
        assert_eq!(
            g.can_reach(HeroId::Carl, floor, ledge, Unlocks(0b01)),
            Err(Reason::Capability {
                hero: None,
                missing: Unlocks(0b10)
            })
        );
        assert!(
            g.can_reach(HeroId::Carl, floor, ledge, Unlocks(0b11))
                .is_ok()
        );
    }

    /// Floor, a destructible crate-wall (top 200, neither hero can jump it),
    /// a low ledge, and a destructible catwalk joining two high ledges (rise
    /// 120: Carl cannot climb back up to them from the floor, Donut can).
    fn destructible_room() -> NavLevel {
        let mut wall = solid(1, 600.0, 200.0, 40.0, 200.0);
        wall.destructible = true;
        let mut bridge = solid(4, 840.0, 120.0, 320.0, 8.0);
        bridge.destructible = true;
        NavLevel {
            solids: vec![
                solid(0, 0.0, 0.0, 1400.0, 40.0),
                wall,
                solid(2, 100.0, 60.0, 80.0, 8.0),
                solid(3, 800.0, 120.0, 40.0, 8.0),
                bridge,
                solid(5, 1160.0, 120.0, 40.0, 8.0),
            ],
            hazards: Vec::new(),
        }
    }

    #[test]
    fn destroying_a_solid_changes_only_the_affected_edges_and_bumps_the_revision() {
        let mut g = NavGraph::build(&destructible_room(), NavConfig::default()).unwrap();
        let (left, right) = (anchor_on(&g, 2), anchor_on(&g, 5));
        let before = g.edge_views();
        assert_eq!(g.revision(), 0);
        // The wall splits the floor: neither hero can cross it.
        let floor_left = g.anchors().find(|a| a.solid == SolidId(0)).unwrap();
        let floor_right = g
            .anchors()
            .filter(|a| a.solid == SolidId(0))
            .max_by(|a, b| {
                g.anchor_pos(*a)
                    .unwrap()
                    .0
                    .total_cmp(&g.anchor_pos(*b).unwrap().0)
            })
            .unwrap();
        assert_eq!(
            g.can_reach(HeroId::Carl, floor_left, floor_right, Unlocks::NONE),
            Err(Reason::Unreachable)
        );
        let report = g.destroy_solid(SolidId(1)).unwrap();
        assert_eq!(report.revision, 1);
        assert_eq!(g.revision(), 1);
        assert!(!report.changed_edges.is_empty());
        assert!(report.recomputed_edges < report.edge_count, "{report:?}");
        let after = g.edge_views();
        assert_eq!(before.len(), after.len());
        let dep_edges: Vec<(NavAnchorId, NavAnchorId)> = report.changed_edges.clone();
        for (b, a) in before.iter().zip(&after) {
            assert_eq!((b.from, b.to), (a.from, a.to), "stable edge order");
            let moved = (b.state != a.state) as usize;
            assert_eq!(
                moved == 1,
                dep_edges.contains(&(b.from, b.to)),
                "only the reported edges changed"
            );
            assert_eq!(b.as_built, a.as_built, "as-built state never changes");
            if moved == 0 {
                assert_eq!(b.rev, a.rev);
            } else {
                assert_eq!(a.rev, 1);
            }
        }
        // Exactly the edges whose corridor touches the wall were re-evaluated,
        // and every change is among them.
        let touching = after
            .iter()
            .filter(|e| e.deps.contains(&SolidId(1)))
            .count();
        assert_eq!(report.recomputed_edges, touching);
        assert!(touching > 0);
        for (b, a) in before.iter().zip(&after) {
            if b.state != a.state {
                assert!(a.deps.contains(&SolidId(1)), "{:?} -> {:?}", a.from, a.to);
            }
        }
        assert!(
            g.can_reach(HeroId::Carl, floor_left, floor_right, Unlocks::NONE)
                .is_ok()
        );
        // Destroying again is a no-op (no revision bump).
        let again = g.destroy_solid(SolidId(1)).unwrap();
        assert_eq!((again.revision, again.recomputed_edges), (1, 0));
        // Anchors of other solids keep their ids.
        assert!(g.anchor_open(left) && g.anchor_open(right));
    }

    #[test]
    fn destroying_a_bridge_reports_destroyed_and_restoring_it_reopens_the_route() {
        let mut g = NavGraph::build(&destructible_room(), NavConfig::default()).unwrap();
        let (a, b) = (anchor_on(&g, 3), anchor_on(&g, 5));
        let bridge = anchor_on(&g, 4);
        assert!(g.can_reach(HeroId::Carl, a, b, Unlocks::NONE).is_ok());
        g.destroy_solid(SolidId(4)).unwrap();
        // The ledges are 320 px apart: no hero jumps it, and Carl cannot climb
        // back up from the floor (rise 120). Donut can, so she still routes.
        assert!(g.can_reach(HeroId::Donut, a, b, Unlocks::NONE).is_ok());
        assert_eq!(
            g.can_reach(HeroId::Carl, a, b, Unlocks::NONE),
            Err(Reason::Destroyed)
        );
        assert_eq!(
            g.can_reach(HeroId::Carl, a, bridge, Unlocks::NONE),
            Err(Reason::Destroyed)
        );
        g.set_solid_intact(SolidId(4), true).unwrap();
        assert!(g.can_reach(HeroId::Carl, a, b, Unlocks::NONE).is_ok());
        assert_eq!(g.revision(), 2);
    }

    #[test]
    fn indestructible_solids_cannot_be_destroyed_and_unknown_ids_error() {
        let mut g = graph();
        assert_eq!(
            g.destroy_solid(SolidId(0)),
            Err(NavError::NotDestructible(SolidId(0)))
        );
        assert_eq!(
            g.destroy_solid(SolidId(99)),
            Err(NavError::UnknownSolid(SolidId(99)))
        );
        assert_eq!(g.revision(), 0);
    }

    #[test]
    fn bad_input_is_rejected() {
        let mut level = room();
        level.solids.push(solid(1, 0.0, 0.0, 10.0, 10.0));
        assert_eq!(
            NavGraph::build(&level, NavConfig::default()).unwrap_err(),
            NavError::DuplicateSolid(SolidId(1))
        );
        let mut level = room();
        level.solids[0].rect.width = f32::NAN;
        assert_eq!(
            NavGraph::build(&level, NavConfig::default()).unwrap_err(),
            NavError::BadGeometry(SolidId(0))
        );
    }

    #[test]
    fn two_builds_hash_identically_whatever_the_input_order() {
        let a = graph();
        let b = graph();
        assert_eq!(a.state_hash(), b.state_hash());
        assert_eq!(a.canonical_dump(), b.canonical_dump());
        let mut shuffled = room();
        shuffled.solids.reverse();
        let c = NavGraph::build(&shuffled, NavConfig::default()).unwrap();
        assert_eq!(a.state_hash(), c.state_hash());
        // Destruction keeps the two in lockstep too.
        let (mut x, mut y) = (
            NavGraph::build(&destructible_room(), NavConfig::default()).unwrap(),
            NavGraph::build(&destructible_room(), NavConfig::default()).unwrap(),
        );
        x.destroy_solid(SolidId(1)).unwrap();
        y.destroy_solid(SolidId(1)).unwrap();
        assert_eq!(x.state_hash(), y.state_hash());
        assert_ne!(x.state_hash(), a.state_hash());
    }

    #[test]
    fn anchor_ids_survive_destruction_of_a_blocker() {
        let mut g = NavGraph::build(&destructible_room(), NavConfig::default()).unwrap();
        let ids: Vec<NavAnchorId> = g.anchors().collect();
        g.destroy_solid(SolidId(1)).unwrap();
        assert_eq!(ids, g.anchors().collect::<Vec<_>>());
        assert!(!g.anchor_open(NavAnchorId {
            solid: SolidId(1),
            slot: 0
        }));
        assert_eq!(
            g.can_reach(
                HeroId::Carl,
                NavAnchorId {
                    solid: SolidId(1),
                    slot: 0
                },
                anchor_on(&g, 0),
                Unlocks::NONE
            ),
            Err(Reason::Destroyed)
        );
    }

    /// The graph's jump verdict matches the real body step driven by real
    /// inputs (a press and a held direction), for Carl.
    #[test]
    fn a_reachable_jump_is_confirmed_by_the_real_hero_step() {
        let level = NavLevel {
            solids: vec![
                solid(0, 0.0, 0.0, 600.0, 40.0),
                solid(1, 150.0, 70.0, 60.0, 20.0),
            ],
            hazards: Vec::new(),
        };
        let g = NavGraph::build(&level, NavConfig::default()).unwrap();
        let (floor, ledge) = (anchor_on(&g, 0), anchor_on(&g, 1));
        assert!(
            g.can_reach(HeroId::Carl, floor, ledge, Unlocks::NONE)
                .is_ok()
        );
        let blocks: Vec<Block> = level
            .solids
            .iter()
            .map(|s| Block {
                rect: s.rect,
                material: Material::Stone,
                intact: true,
            })
            .collect();
        let mut hero = Hero::at(100.0, HERO_HALF_HEIGHT);
        let mut frame = InputFrame::default();
        let mut landed = false;
        for tick in 0..120 {
            frame.advance(RawInput {
                move_x: 1.0,
                jump: tick == 0 || tick == 30,
                ..RawInput::default()
            });
            step_hero(&mut hero, &frame, &blocks, 700.0);
            if hero.grounded && (hero.y - HERO_HALF_HEIGHT - 70.0).abs() < 0.6 {
                landed = true;
                break;
            }
        }
        assert!(landed, "Carl lands on the ledge: {hero:?}");
        // And a 120 px rise is out of his reach in the graph and the step.
        let high = NavLevel {
            solids: vec![
                solid(0, 0.0, 0.0, 600.0, 40.0),
                solid(1, 150.0, 120.0, 60.0, 20.0),
            ],
            hazards: Vec::new(),
        };
        let gh = NavGraph::build(&high, NavConfig::default()).unwrap();
        assert!(
            gh.can_reach(
                HeroId::Carl,
                anchor_on(&gh, 0),
                anchor_on(&gh, 1),
                Unlocks::NONE
            )
            .is_err()
        );
    }

    #[test]
    fn a_ceiling_blocks_the_anchor_below_it_until_it_is_destroyed() {
        let mut slab = solid(1, 200.0, 40.0, 200.0, 20.0);
        slab.destructible = true;
        let level = NavLevel {
            solids: vec![solid(0, 0.0, 0.0, 800.0, 40.0), slab],
            hazards: Vec::new(),
        };
        let mut g = NavGraph::build(&level, NavConfig::default()).unwrap();
        let floors: Vec<NavAnchorId> = g.anchors().filter(|a| a.solid == SolidId(0)).collect();
        assert_eq!(floors.len(), 3, "left, under the slab, right");
        let under = floors[1];
        assert!(!g.anchor_open(under));
        g.destroy_solid(SolidId(1)).unwrap();
        assert!(g.anchor_open(under));
        assert!(
            g.can_reach(HeroId::Carl, floors[0], floors[2], Unlocks::NONE)
                .is_ok()
        );
    }

    fn real_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn the_camera_lab_donut_only_platform_is_a_perch_carl_cannot_follow() {
        let rooms = crate::camera_lab::room::load_rooms(&real_root()).expect("rooms");
        let room = &rooms[5]; // ROOM-06: the high central platform is Donut-only
        let level = NavLevel::from_camera_lab_room(room);
        let g = NavGraph::build(&level, NavConfig::default()).unwrap();
        let perch_solid = level
            .solids
            .iter()
            .find(|s| s.access == HeroMask::DONUT)
            .expect("a Donut-only platform");
        let perch = g
            .anchors()
            .find(|a| a.solid == perch_solid.id && g.anchor_open(*a))
            .unwrap();
        let (cx, _) = g.anchor_pos(perch).unwrap();
        let floor = g
            .anchor_at(room.spawn_carl as f32, -(room.floor_y as f32))
            .unwrap();
        assert!(cx > 0.0);
        // Carl has no edge onto the perch; Donut reaches it from the floor.
        assert!(
            g.edge_views()
                .iter()
                .filter(|e| e.to == perch)
                .all(|e| matches!(e.state[HeroId::Carl.index()], EdgeState::Blocked(_)))
        );
        // Carl's refusal is a capability/unreachable verdict, never a route.
        let carl = g.can_reach(HeroId::Carl, floor, perch, Unlocks::NONE);
        let donut = g.can_reach(HeroId::Donut, floor, perch, Unlocks::NONE);
        println!("NAV-LAB ROOM-06 perch: carl {carl:?} donut {donut:?}");
        assert!(carl.is_err());
        // Every lab room builds and both heroes route spawn to exit anchor.
        for room in &rooms {
            let level = NavLevel::from_camera_lab_room(room);
            let g = NavGraph::build(&level, NavConfig::default()).unwrap();
            assert!(g.anchor_count() > 3, "{}", room.id);
            let goal = g
                .anchor_at(room.goal_x as f32, -(room.floor_y as f32))
                .unwrap_or_else(|| panic!("{}: no anchor at the goal", room.id));
            for (hero, from) in [
                (HeroId::Carl, room.spawn_carl),
                (HeroId::Donut, room.spawn_donut),
            ] {
                // Some lab spawns sit under an access step: take the nearest
                // open floor anchor.
                let floor_top = -(room.floor_y as f32);
                let start = g
                    .anchors()
                    .filter(|a| g.anchor_open(*a) && g.anchor_pos(*a).unwrap().1 == floor_top)
                    .min_by(|a, b| {
                        let d = |n: &NavAnchorId| {
                            let (l, r) = g.anchor_span(*n).unwrap();
                            (from as f32 - from as f32)
                                .max(l - from as f32)
                                .max(from as f32 - r)
                        };
                        d(a).total_cmp(&d(b))
                    })
                    .unwrap_or_else(|| panic!("{}: no floor anchor", room.id));
                let plan = g.plan(hero, start, goal, Unlocks::NONE);
                assert!(
                    matches!(plan, Plan::Route(_)) || matches!(plan, Plan::Wait { .. }),
                    "{} {hero:?}: {plan:?}",
                    room.id
                );
            }
        }
    }

    /// Build and query cost of one level, printed with `--nocapture`.
    fn measure(label: &str, level: &NavLevel) {
        use std::time::Instant;
        let t = Instant::now();
        let g = NavGraph::build(level, NavConfig::default()).unwrap();
        let build = t.elapsed();
        let again = NavGraph::build(level, NavConfig::default()).unwrap();
        assert_eq!(g.state_hash(), again.state_hash(), "{label}");
        let y_of = |a: &NavAnchorId| g.anchor_pos(*a).unwrap().1;
        let floor_y = g
            .anchors()
            .filter(|a| g.anchor_open(*a))
            .map(|a| y_of(&a))
            .fold(f32::MAX, f32::min);
        // The ends of the walking floor.
        let open: Vec<NavAnchorId> = g
            .anchors()
            .filter(|a| g.anchor_open(*a) && y_of(a) == floor_y)
            .collect();
        let x_of = |a: &NavAnchorId| g.anchor_pos(*a).unwrap().0;
        let first = *open
            .iter()
            .min_by(|a, b| x_of(a).total_cmp(&x_of(b)))
            .unwrap();
        let last = *open
            .iter()
            .max_by(|a, b| x_of(a).total_cmp(&x_of(b)))
            .unwrap();
        let runs = 200;
        let mut sink = 0u64;
        let t = Instant::now();
        for _ in 0..runs {
            sink += u64::from(
                g.can_reach(HeroId::Carl, first, last, Unlocks::NONE)
                    .is_ok(),
            );
        }
        let query = t.elapsed() / runs;
        let t = Instant::now();
        for _ in 0..runs {
            sink += matches!(
                g.plan(HeroId::Donut, first, last, Unlocks::NONE),
                Plan::Route(_)
            ) as u64;
        }
        let plan = t.elapsed() / runs;
        println!(
            "NAV-PERF {label}: solids {} anchors {} edges {} build {build:?} can_reach {query:?} plan {plan:?} (sink {sink})",
            level.solids.len(),
            g.anchor_count(),
            g.edge_count(),
        );
        assert!(build.as_secs() < 30, "{label}");
    }

    /// Timing on real Floor 01 rooms: the corridor layout through `EnvKit`
    /// and the camera-lab rooms.
    #[test]
    fn floor01_build_and_query_cost() {
        use crate::env_kit::EnvKit;
        let kit = EnvKit::load(
            &real_root(),
            "assets/levels/floor01/corridor_abc_sample.json",
        )
        .expect("floor 01 corridor");
        measure(
            "floor01 corridor_abc_sample",
            &NavLevel::from_blocks(&kit.blocks(), Vec::new()),
        );
        for room in crate::camera_lab::room::load_rooms(&real_root()).expect("rooms") {
            measure(&room.id, &NavLevel::from_camera_lab_room(&room));
        }
    }
}
