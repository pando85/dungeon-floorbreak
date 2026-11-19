//! DEATH-C: the cosmetic death presenter.
//!
//! Pure, Bevy-free and **presentation only**. It reads the finished simulation
//! (the defeat events and the goblins' positions) after every fixed tick and
//! plays the goblin death art: the GOB-02-DEATH poses (crush, decapitation) as
//! sprite overrides, and launched parts, gore drops and floor or wall decals
//! from fixed-size pools, with the spawner timing of the art spec (section 8)
//! for the active [`GorePreset`]. Nothing here flows back: no simulation value,
//! event, seed or replay hash reads it, and the preset is not an input of the
//! simulation. The only randomness is a [`PresentationRng`] seeded from the
//! defeat event.
//!
//! The Bevy adapter (`src/bin/floor01_scene.rs`) draws [`DeathPresenter::poses`],
//! [`DeathPresenter::parts`], [`DeathPresenter::floor_decals`] and
//! [`DeathPresenter::wall_decals`] one to one into pre-spawned sprites, on the
//! draw layers of [`draw_z`] (debris and decals always under the actors).
//!
//! Finish selection is [`DeathProfiles`](crate::death::DeathProfiles) (DEATH-B):
//! `goblin_crushed` plays the crush, `goblin_knockback_down` keeps the GOB-01
//! knockback and corpse frames and only adds blood, any other finish id (a
//! species without art) draws nothing. `goblin_decapitated` is reserved for
//! PR D (the blade hazard); no authoritative cause selects it yet, it is played
//! by the presenter's lab override and its tests.

use std::sync::Arc;

use crate::combat::{CombatEvent, DefeatCause, ENEMY_HALF_HEIGHT, ForceTier};
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::death::DeathProfiles;
use crate::feel::{GROUND_FRICTION, KNOCK_GRAVITY};
use crate::goblin::GoblinMode;
use crate::gore::{
    DeathArt, GorePreset, PartClass, PresentationRng, PresetSheets, Settle, Ttl, keeps_stream_item,
    lowest_below_centre, part_center_for_anchor, pose_anchor_world, rest_angle, scale_count,
};
use crate::prototype::TICK_SECONDS;

/// Launched parts and drops (settled pieces stay in the pool until their ttl
/// or until the pool recycles them, oldest first). Budgets are first guesses:
/// they are set after profiling on the target machine (see the PR evidence).
pub const PART_POOL_CAPACITY: usize = 160;
/// Floor decals: splats, pools (a two-cell pool is one entry), smears.
pub const FLOOR_DECAL_POOL_CAPACITY: usize = 96;
/// Back-wall spatters.
pub const WALL_DECAL_POOL_CAPACITY: usize = 12;
/// Death scripts alive at once (real defeats plus synthetic stress ones).
pub const MAX_DEATHS: usize = 24;

/// Splat decals one death may leave from landing drops (spec section 8).
pub const MAX_DROP_SPLATS: u8 = 8;
/// Splat decals one death may leave from landing pieces (the head's bounces,
/// gibs): kept apart so the drops cannot starve them.
pub const MAX_PART_STAINS: u8 = 6;

/// Crush pool stages (1, 2, 3) after the landing, spec section 8.
const CRUSH_POOL_TICKS: (u64, u64, u64) = (4, 20, 60);
/// The same at preset 4: the stages come earlier.
const CRUSH_POOL_TICKS_FULL: (u64, u64, u64) = (2, 10, 24);

/// Ticks a `Long` piece stays, then fades.
pub const LONG_TTL_TICKS: u64 = 360;
pub const LONG_FADE_TICKS: u64 = 30;
/// Ticks a `Short` piece stays, then fades.
pub const SHORT_TTL_TICKS: u64 = 120;
pub const SHORT_FADE_TICKS: u64 = 12;

/// Slower than this on landing, a part stops bouncing and settles (px/s).
const BOUNCE_MIN_SPEED: f32 = 40.0;
/// Deceleration of a rolling head (px/s^2): a 4 to 12 px roll.
const ROLL_DECEL: f32 = 300.0;
/// Rolling radius used to turn roll speed into spin (px).
const ROLL_RADIUS: f32 = 7.0;
/// Gravity scale of fluttering scraps (ears, rags).
const FLUTTER_GRAVITY: f32 = 0.3;

/// The synthetic target id of a stress or lab death (no real goblin).
pub const SYNTHETIC_TARGET: u32 = u32::MAX;

// ------------------------------------------------------------ draw order ----

/// Which draw layer a piece belongs to. The rule (art spec section 7): nothing
/// is ever drawn over Carl or Donut. Debris sits just under the actor layer,
/// floor decals under that, and wall spatters on the `backwall_decal` layer
/// only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawLayer {
    Debris,
    FloorDecal,
    WallDecal,
}

/// z of the debris layer below `actor_z`.
pub const DEBRIS_BELOW_ACTORS: f32 = 0.4;
/// z of the floor decals below `actor_z`.
pub const FLOOR_DECAL_BELOW_ACTORS: f32 = 0.5;
/// z of the wall spatters above the `backwall_decal` layer (it must stay below
/// the collision layer, one step up).
pub const WALL_DECAL_ABOVE_LAYER: f32 = 0.5;
/// Tie-break between two pieces of one layer, by pool slot.
pub const SLOT_Z_STEP: f32 = 0.0001;

/// Bevy z of pool slot `slot` of `layer`: `actor_z` is the actor layer,
/// `wall_decal_z` the `backwall_decal` layer.
pub fn draw_z(layer: DrawLayer, slot: usize, actor_z: f32, wall_decal_z: f32) -> f32 {
    let bias = slot as f32 * SLOT_Z_STEP;
    match layer {
        DrawLayer::Debris => actor_z - DEBRIS_BELOW_ACTORS + bias,
        DrawLayer::FloorDecal => actor_z - FLOOR_DECAL_BELOW_ACTORS + bias,
        DrawLayer::WallDecal => wall_decal_z + WALL_DECAL_ABOVE_LAYER + bias,
    }
}

// ----------------------------------------------------------------- pool -----

/// A fixed-capacity pool. A full pool recycles its oldest entry. A pool entry is
/// always one unit: a two-cell decal is one entry, never half of one.
#[derive(Debug, Clone)]
pub struct Pool<T> {
    slots: Vec<Option<(u64, T)>>,
    next_seq: u64,
    evicted: u64,
}

impl<T> Pool<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            slots: (0..capacity).map(|_| None).collect(),
            next_seq: 0,
            evicted: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub fn live(&self) -> usize {
        self.slots.iter().flatten().count()
    }

    /// Entries recycled while the pool was full.
    pub fn evicted(&self) -> u64 {
        self.evicted
    }

    /// Put `item` in a free slot, or recycle the oldest. Returns the slot and
    /// the spawn sequence number (to recognise the entry later).
    pub fn spawn(&mut self, item: T) -> (usize, u64) {
        let seq = self.next_seq;
        self.next_seq += 1;
        let slot = match self.slots.iter().position(Option::is_none) {
            Some(free) => free,
            None => {
                self.evicted += 1;
                self.slots
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, s)| s.as_ref().map_or(u64::MAX, |(seq, _)| *seq))
                    .map(|(i, _)| i)
                    .expect("a pool has at least one slot")
            }
        };
        self.slots[slot] = Some((seq, item));
        (slot, seq)
    }

    pub fn get(&self, slot: usize) -> Option<&T> {
        self.slots.get(slot)?.as_ref().map(|(_, t)| t)
    }

    pub fn get_mut(&mut self, slot: usize) -> Option<&mut T> {
        self.slots.get_mut(slot)?.as_mut().map(|(_, t)| t)
    }

    /// The entry in `slot`, only if it is still the one spawned as `seq`.
    pub fn get_seq_mut(&mut self, slot: usize, seq: u64) -> Option<&mut T> {
        match self.slots.get_mut(slot)? {
            Some((s, t)) if *s == seq => Some(t),
            _ => None,
        }
    }

    pub fn free(&mut self, slot: usize) {
        if let Some(entry) = self.slots.get_mut(slot) {
            *entry = None;
        }
    }

    pub fn clear(&mut self) {
        self.slots.iter_mut().for_each(|s| *s = None);
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, &T)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|(_, t)| (i, t)))
    }
}

// ---------------------------------------------------------------- finish ----

/// What a defeated goblin plays, from the DEATH-B finish id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    /// The falling block: `crush_contact`, `crushed_remains`, the squeeze.
    Crushed,
    /// The GOB-01 knockback and corpse frames, plus blood.
    Knockback,
    /// PR D (blade hazard): `decap_contact`, `headless_collapse`, the head.
    Decapitated,
    /// A finish without art: nothing is drawn.
    NoArt,
}

impl Finish {
    pub fn from_id(id: &str) -> Self {
        match id {
            "goblin_crushed" => Self::Crushed,
            "goblin_knockback_down" => Self::Knockback,
            "goblin_decapitated" => Self::Decapitated,
            _ => Self::NoArt,
        }
    }

    /// Whether the finish replaces the goblin sprite with a death pose.
    pub const fn has_pose(self) -> bool {
        matches!(self, Self::Crushed | Self::Decapitated)
    }

    /// The pose frame (index into the poses atlas) `age` ticks after the defeat.
    pub fn pose_frame(self, age: u64) -> Option<usize> {
        match self {
            // L: pose 0 (3 ticks), L+3: pose 1 (3 ticks), L+6: pose 2, hold.
            Self::Crushed => Some(match age {
                0..=2 => 0,
                3..=5 => 1,
                _ => 2,
            }),
            // D: pose 3 (4 ticks), D+4: pose 4 (6 ticks), D+10: pose 5, hold.
            Self::Decapitated => Some(match age {
                0..=3 => 3,
                4..=9 => 4,
                _ => 5,
            }),
            Self::Knockback | Self::NoArt => None,
        }
    }
}

// ---------------------------------------------------------------- state -----

/// A goblin as the presenter sees it (read from the simulation, never written).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoblinView {
    pub x: f32,
    /// y-up world height of its feet on the floor.
    pub ground_y: f32,
    pub facing: i8,
    pub mode: GoblinMode,
}

/// The pose a dead goblin is drawn with instead of its GOB-01 sprite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoseView {
    pub target: u32,
    /// Index into the death poses atlas.
    pub frame: usize,
    pub foot_x: f32,
    pub ground_y: f32,
    pub flip: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PartState {
    Flying,
    /// On the floor, sliding or rolling to a stop.
    Sliding,
    Resting {
        since: u64,
    },
}

/// One launched piece: a part, a gib or a blood drop.
#[derive(Debug, Clone)]
pub struct Part {
    /// Index into `DeathArt::parts`.
    pub meta: usize,
    /// Atlas frame drawn (changes when the head swaps to `head_dead`).
    pub frame: usize,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub angle_deg: f32,
    pub spin_dps: f32,
    pub flip: bool,
    pub ground_y: f32,
    bounces_left: u8,
    contacts: u8,
    state: PartState,
    gravity_scale: f32,
    roll_dir: f32,
    roll_distance: f32,
    death: u32,
}

impl Part {
    pub fn is_resting(&self) -> bool {
        matches!(self.state, PartState::Resting { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecalKind {
    Floor,
    Wall,
}

/// A floor or wall decal: static, one pool entry even when it spans two cells.
#[derive(Debug, Clone)]
pub struct Decal {
    pub meta: usize,
    /// The second cell of a two-cell decal (the right half of pool stage 3),
    /// drawn `pair_offset` px right of the first.
    pub pair_meta: Option<usize>,
    /// Centre of the (first) cell, y-up world px.
    pub x: f32,
    pub y: f32,
    pub flip: bool,
    pub kind: DecalKind,
    born: u64,
}

/// What a finished tick of the presenter draws for one decal or part.
fn fade_alpha(ttl: Ttl, since: u64, clock: u64) -> f32 {
    let (keep, fade) = match ttl {
        Ttl::Keep => return 1.0,
        Ttl::Long => (LONG_TTL_TICKS, LONG_FADE_TICKS),
        Ttl::Short => (SHORT_TTL_TICKS, SHORT_FADE_TICKS),
    };
    let age = clock.saturating_sub(since);
    if age <= keep {
        1.0
    } else {
        1.0 - ((age - keep) as f32 / fade as f32).min(1.0)
    }
}

fn expired(ttl: Ttl, since: u64, clock: u64) -> bool {
    fade_alpha(ttl, since, clock) <= 0.0
}

#[derive(Debug, Clone)]
struct Death {
    id: u32,
    /// Real goblin index, or [`SYNTHETIC_TARGET`].
    target: u32,
    finish: Finish,
    force: ForceTier,
    start: u64,
    foot_x: f32,
    ground_y: f32,
    flip: bool,
    /// +1 or -1: the way debris and the head travel (away from the blow or the
    /// block).
    away: f32,
    rng: PresentationRng,
    drop_splats: u8,
    part_stains: u8,
    drops_emitted: u32,
    /// The pool decal that grows by frame swaps: (slot, spawn seq).
    pool: Option<(usize, u64)>,
    landed: Option<u64>,
    done_after: u64,
}

/// Everything one presenter step needs from the simulation.
#[derive(Debug, Clone, Copy)]
pub struct Observation<'a> {
    pub goblins: &'a [GoblinView],
    pub hero_x: f32,
    /// x of the middle of the shore's hazard footprint, if the fixture has one.
    pub block_x: Option<f32>,
    /// Whether a back wall is within reach of the spatter (art spec section 8:
    /// "when a wall face is within 48 px"). The corridor has its back wall
    /// everywhere, so the fixture scene always passes true; a route with open
    /// background passes false and gets no wall spatter.
    pub back_wall: bool,
}

/// Counters for evidence and tests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PresenterStats {
    pub parts_spawned: u64,
    pub floor_decals_spawned: u64,
    pub wall_decals_spawned: u64,
    pub peak_parts: usize,
    pub peak_floor_decals: usize,
}

/// Something a piece did on the floor, handled once the pool is no longer
/// borrowed.
#[derive(Debug, Clone, Copy)]
enum Contact {
    DropLanded {
        death: u32,
        meta: usize,
        x: f32,
    },
    Touched {
        death: u32,
        meta: usize,
        x: f32,
        first: bool,
    },
    RollStarted {
        death: u32,
        meta: usize,
        x: f32,
        dir: f32,
    },
}

const RAD_TO_DEG: f32 = 57.295_78;

/// The cosmetic presenter: one per run, reset on a restart.
pub struct DeathPresenter {
    art: Arc<DeathArt>,
    sheets: PresetSheets,
    profiles: &'static DeathProfiles,
    species: &'static str,
    preset: GorePreset,
    lab_finish: Option<Finish>,
    clock: u64,
    attempt: u32,
    seen_events: usize,
    next_death_id: u32,
    deaths: Vec<Death>,
    parts: Pool<Part>,
    floor_decals: Pool<Decal>,
    wall_decals: Pool<Decal>,
    last_wall: Option<(usize, bool)>,
    back_wall: bool,
    stress_rng: PresentationRng,
    stats: PresenterStats,
}

impl DeathPresenter {
    pub fn new(art: Arc<DeathArt>, preset: GorePreset) -> Result<Self, String> {
        let sheets = art.compose(preset)?;
        Ok(Self {
            art,
            sheets,
            profiles: DeathProfiles::builtin(),
            species: "goblin",
            preset,
            lab_finish: None,
            clock: 0,
            attempt: 0,
            seen_events: 0,
            next_death_id: 0,
            deaths: Vec::new(),
            parts: Pool::new(PART_POOL_CAPACITY),
            floor_decals: Pool::new(FLOOR_DECAL_POOL_CAPACITY),
            wall_decals: Pool::new(WALL_DECAL_POOL_CAPACITY),
            last_wall: None,
            back_wall: true,
            stress_rng: PresentationRng::new(0xD1E5_7AE5),
            stats: PresenterStats::default(),
        })
    }

    pub fn preset(&self) -> GorePreset {
        self.preset
    }

    pub fn art(&self) -> &DeathArt {
        &self.art
    }

    /// The composed sheets of the active preset (what the adapter uploads).
    pub fn sheets(&self) -> &PresetSheets {
        &self.sheets
    }

    pub fn stats(&self) -> PresenterStats {
        self.stats
    }

    pub fn clock(&self) -> u64 {
        self.clock
    }

    /// Lab only: play `finish` for every defeat whose selected finish is the
    /// ordinary knockback (the decapitation has no authoritative cause before
    /// PR D). Presentation only; the simulation never learns of it.
    pub fn set_lab_finish(&mut self, finish: Option<Finish>) {
        self.lab_finish = finish;
    }

    /// Forget every piece and death (a restart). The attempt counter is kept.
    pub fn clear(&mut self) {
        self.deaths.clear();
        self.parts.clear();
        self.floor_decals.clear();
        self.wall_decals.clear();
        self.last_wall = None;
        self.clock = 0;
    }

    pub fn live_parts(&self) -> usize {
        self.parts.live()
    }

    pub fn live_floor_decals(&self) -> usize {
        self.floor_decals.live()
    }

    pub fn live_wall_decals(&self) -> usize {
        self.wall_decals.live()
    }

    pub fn recycled(&self) -> (u64, u64, u64) {
        (
            self.parts.evicted(),
            self.floor_decals.evicted(),
            self.wall_decals.evicted(),
        )
    }

    // ---------------------------------------------------------- views ----

    /// The death poses to draw instead of the GOB-01 sprites.
    pub fn poses(&self) -> impl Iterator<Item = PoseView> + '_ {
        self.deaths.iter().filter_map(|d| {
            if d.target == SYNTHETIC_TARGET {
                return None;
            }
            let frame = d.finish.pose_frame(self.clock.saturating_sub(d.start))?;
            Some(PoseView {
                target: d.target,
                frame,
                foot_x: d.foot_x,
                ground_y: d.ground_y,
                flip: d.flip,
            })
        })
    }

    /// The pose override of goblin `target`, if it has one.
    pub fn pose_of(&self, target: u32) -> Option<PoseView> {
        self.poses().find(|p| p.target == target)
    }

    pub fn parts(&self) -> impl Iterator<Item = (usize, &Part)> {
        self.parts.iter()
    }

    pub fn floor_decals(&self) -> impl Iterator<Item = (usize, &Decal)> {
        self.floor_decals.iter()
    }

    pub fn wall_decals(&self) -> impl Iterator<Item = (usize, &Decal)> {
        self.wall_decals.iter()
    }

    /// Opacity of a part now (1 until it fades).
    pub fn part_alpha(&self, part: &Part) -> f32 {
        match part.state {
            PartState::Resting { since } => {
                fade_alpha(self.art.parts[part.meta].ttl, since, self.clock)
            }
            _ => 1.0,
        }
    }

    pub fn decal_alpha(&self, decal: &Decal) -> f32 {
        fade_alpha(self.art.parts[decal.meta].ttl, decal.born, self.clock)
    }

    // ---------------------------------------------------------- stepping ----

    /// One fixed tick after the simulation step: ingest the new defeat events
    /// of `combat`, then advance the presentation.
    pub fn observe(&mut self, combat: &CombatState, scene: &SceneSim) {
        let goblins: Vec<GoblinView> = combat
            .enemies
            .iter()
            .map(|g| GoblinView {
                x: g.x,
                ground_y: g.floor_y - ENEMY_HALF_HEIGHT,
                facing: g.facing,
                mode: g.mode,
            })
            .collect();
        let block_x = combat.shore.as_ref().map(|s| {
            let f = s.footprint();
            f.left + f.width / 2.0
        });
        let total = combat.dropped_events + combat.events.len();
        let fresh = total
            .saturating_sub(self.seen_events)
            .min(combat.events.len());
        self.seen_events = total;
        let events = &combat.events[combat.events.len() - fresh..];
        self.step(
            events,
            Observation {
                goblins: &goblins,
                hero_x: scene.world.hero.x,
                block_x,
                back_wall: true,
            },
        );
    }

    /// One fixed tick with the events logged during it.
    pub fn step(&mut self, events: &[CombatEvent], obs: Observation<'_>) {
        for event in events {
            match *event {
                CombatEvent::Restart { .. } => {
                    self.clear();
                    self.attempt += 1;
                }
                CombatEvent::EnemyDefeated {
                    tick,
                    target,
                    cause,
                    force,
                    ..
                } => self.begin_death(tick, target, cause, force, &obs),
                _ => {}
            }
        }
        self.back_wall = obs.back_wall;
        self.clock += 1;
        self.step_parts();
        self.step_deaths(&obs);
    }

    fn begin_death(
        &mut self,
        tick: u64,
        target: u32,
        cause: DefeatCause,
        force: ForceTier,
        obs: &Observation<'_>,
    ) {
        let Some(goblin) = obs.goblins.get(target as usize) else {
            return;
        };
        let selection = self.profiles.select(self.species, cause, force);
        let mut finish = Finish::from_id(selection.finish);
        if finish == Finish::Knockback
            && let Some(lab) = self.lab_finish
        {
            finish = lab;
        }
        let flip = goblin.facing < 0;
        let from_x = match (cause, obs.block_x) {
            (DefeatCause::ShoreBlock { .. }, Some(block)) => block,
            _ => obs.hero_x,
        };
        let away = if (goblin.x - from_x).abs() < 0.5 {
            -f32::from(goblin.facing)
        } else {
            (goblin.x - from_x).signum()
        };
        self.start_death(Death {
            id: self.next_death_id,
            target,
            finish,
            force,
            start: self.clock + 1,
            foot_x: goblin.x,
            ground_y: goblin.ground_y,
            flip,
            away,
            rng: PresentationRng::for_event(self.attempt, tick, target),
            drop_splats: 0,
            part_stains: 0,
            drops_emitted: 0,
            pool: None,
            landed: None,
            done_after: 0,
        });
    }

    fn start_death(&mut self, death: Death) {
        if self.deaths.len() >= MAX_DEATHS {
            // Retire the oldest finished synthetic one, else refuse.
            if let Some(at) = self
                .deaths
                .iter()
                .position(|d| d.target == SYNTHETIC_TARGET)
            {
                self.deaths.remove(at);
            } else {
                return;
            }
        }
        self.next_death_id += 1;
        self.deaths.push(death);
    }

    /// Stress and lab: a death with no goblin behind it (no pose override).
    /// Deterministic for a given `salt`.
    pub fn spawn_synthetic(
        &mut self,
        finish: Finish,
        x: f32,
        ground_y: f32,
        flip: bool,
        salt: u64,
    ) {
        let away = if flip { 1.0 } else { -1.0 };
        self.start_death(Death {
            id: self.next_death_id,
            target: SYNTHETIC_TARGET,
            finish,
            force: ForceTier::Catastrophic,
            start: self.clock + 1,
            foot_x: x,
            ground_y,
            flip,
            away,
            rng: PresentationRng::for_event(self.attempt, salt, SYNTHETIC_TARGET),
            drop_splats: 0,
            part_stains: 0,
            drops_emitted: 0,
            pool: None,
            landed: None,
            done_after: 0,
        });
    }

    /// Lab stress (`--death-stress`): keep the pools loaded by starting a
    /// synthetic crush or decapitation every few ticks around `hero_x`.
    pub fn stress_tick(&mut self, hero_x: f32, ground_y: f32) {
        if !self.clock.is_multiple_of(3) {
            return;
        }
        let side = if self.stress_rng.coin() { 1.0 } else { -1.0 };
        let x = hero_x + side * self.stress_rng.range(30.0, 220.0);
        let finish = if self.stress_rng.coin() {
            Finish::Crushed
        } else {
            Finish::Decapitated
        };
        let flip = self.stress_rng.coin();
        let salt = self.clock;
        self.spawn_synthetic(finish, x, ground_y, flip, salt);
    }

    // ----------------------------------------------------------- parts ----

    fn step_parts(&mut self) {
        let clock = self.clock;
        let slots: Vec<usize> = self.parts.iter().map(|(i, _)| i).collect();
        let mut contacts: Vec<Contact> = Vec::new();
        for slot in slots {
            let Some(part) = self.parts.get_mut(slot) else {
                continue;
            };
            let meta = &self.art.parts[part.meta];
            if let PartState::Resting { since } = part.state {
                if expired(meta.ttl, since, clock) {
                    self.parts.free(slot);
                }
                continue;
            }
            let hull = &self.sheets.parts.hull[part.frame];
            if hull.is_empty() {
                self.parts.free(slot);
                continue;
            }
            let dt = TICK_SECONDS;
            if part.state == PartState::Sliding {
                // Rolling or sliding to a stop on the floor line.
                let rolls = meta.settle == Settle::Roll;
                let decel = if rolls { ROLL_DECEL } else { GROUND_FRICTION };
                part.x += part.vx * dt;
                part.vx = (part.vx.abs() - decel * dt).max(0.0) * part.vx.signum();
                part.angle_deg += part.spin_dps * dt;
                part.spin_dps = if rolls {
                    -part.vx / ROLL_RADIUS * RAD_TO_DEG
                } else {
                    part.spin_dps * 0.9
                };
                // The turning piece stays on the floor line.
                part.y = part.ground_y + lowest_below_centre(hull, part.flip, part.angle_deg);
                if part.vx.abs() < 1.0 {
                    part.rest(clock, hull);
                }
                continue;
            }
            part.vy -= KNOCK_GRAVITY * part.gravity_scale * dt;
            part.x += part.vx * dt;
            part.y += part.vy * dt;
            part.angle_deg += part.spin_dps * dt;
            if part.gravity_scale < 1.0 {
                part.vx *= 0.985;
                part.spin_dps *= 0.985;
            }
            let low = lowest_below_centre(hull, part.flip, part.angle_deg);
            if part.y - low > part.ground_y {
                continue;
            }
            // Clamp to the floor line, always.
            part.y = part.ground_y + low;
            if part.vy > 0.0 {
                continue;
            }
            part.contacts += 1;
            let first = part.contacts == 1;
            let (death, x) = (part.death, part.x);
            if meta.class == PartClass::Blood {
                contacts.push(Contact::DropLanded {
                    death,
                    meta: part.meta,
                    x,
                });
                self.parts.free(slot);
                continue;
            }
            // First contact swap (head -> head_dead).
            if first
                && let Some(swap) = meta.swap_on_first_contact.as_deref()
                && let Some(index) = self.art.part_index(swap)
            {
                part.meta = index;
                part.frame = self.art.parts[index].frame;
            }
            let meta_index = part.meta;
            let bounce = self.art.parts[meta_index].bounce;
            let rolls = self.art.parts[meta_index].settle == Settle::Roll;
            contacts.push(Contact::Touched {
                death,
                meta: meta_index,
                x,
                first,
            });
            if part.bounces_left > 0 && part.vy.abs() > BOUNCE_MIN_SPEED {
                part.vy = -part.vy * bounce;
                part.vx *= 0.7;
                part.spin_dps *= 0.6;
                part.bounces_left -= 1;
            } else {
                part.vy = 0.0;
                if rolls && part.roll_distance > 0.0 {
                    // The head rolls 4 to 12 px and leaves a smear.
                    part.vx = part.roll_dir * (2.0 * ROLL_DECEL * part.roll_distance).sqrt();
                    part.spin_dps = -part.vx / ROLL_RADIUS * RAD_TO_DEG;
                    part.roll_distance = 0.0;
                    part.state = PartState::Sliding;
                    contacts.push(Contact::RollStarted {
                        death,
                        meta: meta_index,
                        x,
                        dir: part.roll_dir,
                    });
                } else if part.vx.abs() > 5.0 {
                    part.state = PartState::Sliding;
                } else {
                    part.rest(clock, hull);
                }
            }
        }
        for contact in contacts {
            self.on_contact(contact);
        }
        // Faded decals leave their pool.
        for pool in [&mut self.floor_decals, &mut self.wall_decals] {
            let gone: Vec<usize> = pool
                .iter()
                .filter(|(_, d)| expired(self.art.parts[d.meta].ttl, d.born, clock))
                .map(|(i, _)| i)
                .collect();
            for slot in gone {
                pool.free(slot);
            }
        }
    }

    /// A drop landed (a splat), a piece touched the floor (a stain) or a head
    /// started rolling (its smear).
    fn on_contact(&mut self, contact: Contact) {
        let (death, meta_index) = match contact {
            Contact::DropLanded { death, meta, .. }
            | Contact::Touched { death, meta, .. }
            | Contact::RollStarted { death, meta, .. } => (death, meta),
        };
        let part = &self.art.parts[meta_index];
        let Some(slot) = self.deaths.iter().position(|d| d.id == death) else {
            return;
        };
        let ground_y = self.deaths[slot].ground_y;
        match contact {
            Contact::DropLanded { x, .. } => {
                if self.deaths[slot].drop_splats >= MAX_DROP_SPLATS {
                    return;
                }
                if let Some(id) = part.on_contact.clone()
                    && self.spawn_floor_decal(&id, x, ground_y, false)
                {
                    self.deaths[slot].drop_splats += 1;
                }
            }
            Contact::RollStarted { x, dir, .. } => {
                if let Some(trail) = part.trail.clone() {
                    self.spawn_floor_decal(&trail, x, ground_y, dir < 0.0);
                }
            }
            Contact::Touched { x, first, .. } => {
                // A stain on the first touch (the head: on every bounce).
                if let Some(stain) = part.stain.clone()
                    && (first || part.trail.is_some())
                    && self.deaths[slot].part_stains < MAX_PART_STAINS
                    && self.spawn_floor_decal(&stain, x, ground_y, false)
                {
                    self.deaths[slot].part_stains += 1;
                }
            }
        }
    }

    // ---------------------------------------------------------- spawning ----

    /// Launch a part (or drop) if the preset allows it. `at` is the world point
    /// of the part's anchor `anchor` (or of its centre when `None`).
    #[allow(clippy::too_many_arguments)]
    fn launch(
        &mut self,
        death: &mut Death,
        id: &str,
        anchor: Option<&str>,
        at: (f32, f32),
        vel: (f32, f32),
        angle_deg: f32,
        spin_dps: f32,
        flip: bool,
    ) -> bool {
        let Some(index) = self.art.part_index(id) else {
            return false;
        };
        let meta = &self.art.parts[index];
        if !meta.allowed_at(self.preset) {
            return false;
        }
        let cell = self.art.part_cell;
        let anchor_px = anchor
            .and_then(|a| meta.anchor(a))
            .unwrap_or((cell.0 / 2, cell.1 / 2));
        let centre = if anchor.is_some() {
            part_center_for_anchor(cell, anchor_px, at, flip)
        } else {
            at
        };
        let bounces = match meta.settle {
            Settle::Static | Settle::Flutter => 0,
            Settle::Roll => 2,
            Settle::Flop => 1,
            Settle::Clatter => 2,
            Settle::Bounce => 3,
        };
        let part = Part {
            meta: index,
            frame: meta.frame,
            x: centre.0,
            y: centre.1.max(death.ground_y),
            vx: vel.0,
            vy: vel.1,
            angle_deg,
            spin_dps,
            flip,
            ground_y: death.ground_y,
            bounces_left: if meta.class == PartClass::Blood {
                0
            } else {
                bounces
            },
            contacts: 0,
            state: PartState::Flying,
            gravity_scale: if meta.settle == Settle::Flutter {
                FLUTTER_GRAVITY
            } else {
                1.0
            },
            roll_dir: vel.0.signum(),
            roll_distance: if meta.settle == Settle::Roll {
                death.rng.range(4.0, 12.0)
            } else {
                0.0
            },
            death: death.id,
        };
        self.parts.spawn(part);
        self.stats.parts_spawned += 1;
        self.stats.peak_parts = self.stats.peak_parts.max(self.parts.live());
        true
    }

    /// A blood drop from `at` thrown `speed` px/s at `elevation_deg` above the
    /// horizontal toward `dir` (+1 / -1).
    fn drop_blood(
        &mut self,
        death: &mut Death,
        at: (f32, f32),
        dir: f32,
        elevation_deg: f32,
        speed: f32,
    ) {
        let (sin, cos) = elevation_deg.to_radians().sin_cos();
        self.launch(
            death,
            "drop_blood",
            None,
            at,
            (dir * cos * speed, sin * speed),
            0.0,
            0.0,
            false,
        );
    }

    /// Spawn a floor decal (a splat, a pool stage, a smear) on the floor line
    /// at world x `x`. Returns false when the preset does not draw it.
    fn spawn_floor_decal(&mut self, id: &str, x: f32, ground_y: f32, flip: bool) -> bool {
        let Some((meta, decal)) = self.floor_decal_for(id, x, ground_y, flip) else {
            return false;
        };
        let _ = meta;
        self.floor_decals.spawn(decal);
        self.stats.floor_decals_spawned += 1;
        self.stats.peak_floor_decals = self.stats.peak_floor_decals.max(self.floor_decals.live());
        true
    }

    fn floor_decal_for(
        &self,
        id: &str,
        x: f32,
        ground_y: f32,
        flip: bool,
    ) -> Option<(usize, Decal)> {
        let index = self.art.part_index(id)?;
        let meta = &self.art.parts[index];
        if !meta.allowed_at(self.preset) {
            return None;
        }
        let cell = self.art.part_cell;
        let bbox = self.sheets.parts.bbox[meta.frame]?;
        // Horizontal: the named anchor pixel on `x`; vertical: the lowest
        // opaque row on the floor line.
        let anchor = meta
            .anchor("centre_bottom")
            .or_else(|| meta.anchor("start"))
            .unwrap_or((cell.0 / 2, cell.1 / 2));
        let centre = part_center_for_anchor(cell, anchor, (x, 0.0), flip);
        let pair_meta = meta
            .pair
            .as_deref()
            .filter(|_| meta.pair_offset.is_some())
            .and_then(|p| self.art.part_index(p));
        Some((
            index,
            Decal {
                meta: index,
                pair_meta,
                x: centre.0,
                y: ground_y + (bbox.bottom - cell.1 / 2) as f32,
                flip,
                kind: DecalKind::Floor,
                born: self.clock,
            },
        ))
    }

    /// Grow the death's pool decal to `id` (one decal that swaps its frame, so
    /// each stage contains the previous one); spawn it when none lives.
    fn pool_stage(&mut self, death: &mut Death, id: &str, x: f32) {
        let Some((index, fresh)) = self.floor_decal_for(id, x, death.ground_y, false) else {
            return;
        };
        let pair = fresh.pair_meta;
        if let Some((slot, seq)) = death.pool
            && let Some(decal) = self.floor_decals.get_seq_mut(slot, seq)
        {
            decal.meta = index;
            decal.pair_meta = pair;
            decal.x = fresh.x;
            decal.y = fresh.y;
            return;
        }
        let (slot, seq) = self.floor_decals.spawn(fresh);
        death.pool = Some((slot, seq));
        self.stats.floor_decals_spawned += 1;
        self.stats.peak_floor_decals = self.stats.peak_floor_decals.max(self.floor_decals.live());
    }

    /// A back-wall spatter whose origin pixel is on `at`. The variant and the
    /// facing are seeded; the same pair never repeats back to back.
    fn wall_spatter(&mut self, death: &mut Death, at: (f32, f32)) {
        if !self.back_wall {
            return;
        }
        let mut variant = usize::from(death.rng.coin());
        let mut flip = death.rng.coin();
        if self.last_wall == Some((variant, flip)) {
            if death.rng.coin() {
                variant = 1 - variant;
            } else {
                flip = !flip;
            }
        }
        self.last_wall = Some((variant, flip));
        let id = ["decal_wall_spatter", "decal_wall_spatter_b"][variant];
        self.wall_spatter_variant(id, flip, at);
    }

    /// One named back-wall spatter whose origin pixel is on `at`.
    fn wall_spatter_variant(&mut self, id: &str, flip: bool, at: (f32, f32)) {
        if !self.back_wall {
            return;
        }
        let Some(index) = self.art.part_index(id) else {
            return;
        };
        let meta = &self.art.parts[index];
        if !meta.allowed_at(self.preset) {
            return;
        }
        let cell = self.art.part_cell;
        let anchor = meta.anchor("origin").unwrap_or((cell.0 / 2, cell.1 / 2));
        let centre = part_center_for_anchor(cell, anchor, at, flip);
        self.wall_decals.spawn(Decal {
            meta: index,
            pair_meta: None,
            x: centre.0,
            y: centre.1,
            flip,
            kind: DecalKind::Wall,
            born: self.clock,
        });
        self.stats.wall_decals_spawned += 1;
    }

    // ---------------------------------------------------------- scripts ----

    fn step_deaths(&mut self, obs: &Observation<'_>) {
        for i in 0..self.deaths.len() {
            let mut death = self.deaths[i].clone();
            let age = self.clock.saturating_sub(death.start);
            if self.clock >= death.start {
                match death.finish {
                    Finish::Crushed => self.script_crush(&mut death, age),
                    Finish::Decapitated => self.script_decap(&mut death, age),
                    Finish::Knockback => self.script_knockback(&mut death, age, obs),
                    Finish::NoArt => {}
                }
            }
            self.deaths[i] = death;
        }
        // Retire finished synthetic scripts (a real defeat keeps its pose).
        let clock = self.clock;
        self.deaths
            .retain(|d| d.target != SYNTHETIC_TARGET || clock <= d.start + d.done_after.max(150));
    }

    /// World position of anchor `name` of pose frame `frame` of `death`.
    fn pose_point(&self, death: &Death, frame: usize, name: &str) -> (f32, f32) {
        let anchor = self
            .art
            .pose(frame)
            .anchor(name)
            .unwrap_or(self.art.pose_pivot);
        pose_anchor_world(
            self.art.pose_pivot,
            anchor,
            death.foot_x,
            death.ground_y,
            death.flip,
        )
    }

    /// +1 / -1: toward the front of the pose (the way its face points).
    fn local_dir(death: &Death, local: f32) -> f32 {
        if death.flip { -local } else { local }
    }

    /// Crush, spec section 8: the landing tick L.
    fn script_crush(&mut self, d: &mut Death, age: u64) {
        d.done_after = 70;
        if age == 0 {
            // The club leaves the hand.
            let hand = self.pose_point(d, 0, "hand_near");
            let (vx, vy) = (d.away * d.rng.range(90.0, 150.0), d.rng.range(120.0, 180.0));
            let spin = -d.away * d.rng.range(540.0, 900.0);
            self.launch(d, "club", Some("grip"), hand, (vx, vy), 0.0, spin, d.flip);
            // 16 to 20 drops: the mouth and the two squeeze points.
            let drops = self.preset.drop_quarters();
            for (anchor, lo, hi, local) in [
                ("mouth", 6, 8, 1.0),
                ("squeeze_back", 5, 6, -1.0),
                ("squeeze_front", 5, 6, 1.0),
            ] {
                let n = scale_count(d.rng.int(lo, hi), drops);
                let from = self.pose_point(d, 0, anchor);
                let dir = Self::local_dir(d, local);
                for _ in 0..n {
                    let elevation = d.rng.range(10.0, 35.0);
                    let speed = d.rng.range(80.0, 160.0);
                    self.drop_blood(d, from, dir, elevation, speed);
                }
            }
            // Two flesh gibs and a bone squeezed out sideways.
            let gibs: Vec<(&str, &str)> = [
                ("gib_flesh_a", "squeeze_front"),
                ("gib_flesh_b", "squeeze_back"),
                ("gib_bone", "squeeze_front"),
            ]
            .into_iter()
            .filter(|(id, _)| {
                self.art
                    .part_index(id)
                    .is_some_and(|i| self.art.parts[i].allowed_at(self.preset))
            })
            .collect();
            let keep = scale_count(gibs.len() as u32, self.preset.gib_quarters()) as usize;
            for (id, anchor) in gibs.into_iter().take(keep) {
                let from = self.pose_point(d, 0, anchor);
                let local = if anchor == "squeeze_back" { -1.0 } else { 1.0 };
                let dir = Self::local_dir(d, local);
                let (sin, cos) = d.rng.range(10.0, 35.0).to_radians().sin_cos();
                let speed = d.rng.range(60.0, 120.0);
                let spin = if d.rng.coin() { 720.0 } else { -720.0 };
                self.launch(
                    d,
                    id,
                    None,
                    from,
                    (dir * cos * speed, sin * speed),
                    0.0,
                    spin,
                    false,
                );
            }
        }
        // Pool stages and the squeeze splats. Preset 4 brings the stages
        // forward (art spec section 9, ruling 6d: the crush empties the body at
        // once, "earlier and bigger crush pool"); the other presets keep the
        // section 8 ticks (their later stages are filtered out by the preset).
        let (first, second, third) = if self.preset == GorePreset::Full {
            CRUSH_POOL_TICKS_FULL
        } else {
            CRUSH_POOL_TICKS
        };
        if age == first {
            for anchor in ["squeeze_front", "squeeze_back"] {
                let from = self.pose_point(d, 0, anchor);
                self.spawn_floor_decal("decal_splat_m", from.0, d.ground_y, false);
            }
        }
        let pool = self.pose_point(d, 2, "pool_origin");
        for (tick, id) in [
            (first, "decal_pool_1"),
            (second, "decal_pool_2"),
            (third, "decal_pool_3"),
        ] {
            if age == tick {
                self.pool_stage(d, id, pool.0);
            }
        }
        // L+8: the squeeze blows sideways onto the back wall. Preset 4 puts
        // both spatter variants up, one fanning each way and a third between
        // them, so it reads as a splash, not one mark (the parts are the
        // spec's; presets below 4 do not draw them).
        if age == 8 {
            let at = (d.foot_x, d.ground_y + 8.0);
            let left = d.rng.coin();
            self.wall_spatter_variant("decal_wall_spatter", left, (at.0 - 4.0, at.1));
            self.wall_spatter_variant("decal_wall_spatter", !left, (at.0 + 4.0, at.1));
            let centre = d.rng.coin();
            self.wall_spatter_variant("decal_wall_spatter_b", centre, (at.0, at.1 + 2.0));
        }
    }

    /// Decapitation, spec section 8: the blade tick D (PR D wires the cause).
    fn script_decap(&mut self, d: &mut Death, age: u64) {
        d.done_after = 150;
        match age {
            0 => {
                let neck = self.pose_point(d, 3, "neck");
                let (vx, vy) = (d.away * d.rng.range(60.0, 100.0), d.rng.range(220.0, 280.0));
                // Chin up, spinning backward (the face tips toward the sky).
                self.launch(
                    d,
                    "head",
                    Some("neck"),
                    neck,
                    (vx, vy),
                    -20.0 * d.away,
                    -720.0 * d.away,
                    d.flip,
                );
            }
            4 => {
                let neck = self.pose_point(d, 4, "neck");
                self.wall_spatter(d, (neck.0, d.ground_y + 16.0));
            }
            14 => {
                let pool = self.pose_point(d, 5, "pool_origin");
                self.pool_stage(d, "decal_pool_1", pool.0);
            }
            40 => {
                let pool = self.pose_point(d, 5, "pool_origin");
                self.pool_stage(d, "decal_pool_2", pool.0);
            }
            100 => {
                let pool = self.pose_point(d, 5, "pool_origin");
                self.pool_stage(d, "decal_pool_3", pool.0);
            }
            _ => {}
        }
        // The jet: 2 drops every 3 ticks for 30 ticks from D+4.
        if (4..34).contains(&age) && (age - 4).is_multiple_of(3) {
            let base = self.pose_point(d, 4, "jet_base");
            for _ in 0..2 {
                let k = d.drops_emitted;
                d.drops_emitted += 1;
                if !keeps_stream_item(k, self.preset.drop_quarters()) {
                    continue;
                }
                let spread = d.rng.range(-12.0, 12.0).to_radians();
                let up = d.rng.range(150.0, 220.0);
                self.launch(
                    d,
                    "drop_blood",
                    None,
                    base,
                    (spread.tan() * up, up),
                    0.0,
                    0.0,
                    false,
                );
            }
        }
        // The trickle: 1 drop every 8 ticks from D+10 for 40 ticks.
        if (10..50).contains(&age) && (age - 10).is_multiple_of(8) {
            let neck = self.pose_point(d, 5, "neck");
            let k = d.drops_emitted;
            d.drops_emitted += 1;
            if keeps_stream_item(k, self.preset.drop_quarters()) {
                let vx = d.rng.range(-10.0, 10.0);
                let vy = d.rng.range(20.0, 60.0);
                self.launch(d, "drop_blood", None, neck, (vx, vy), 0.0, 0.0, false);
            }
        }
    }

    /// The ordinary kill: the GOB-01 frames stay; blood is added. A routine
    /// jab is modest, the kick's launch is heavier (presenter tuning, owner
    /// game-feel review pending).
    fn script_knockback(&mut self, d: &mut Death, age: u64, obs: &Observation<'_>) {
        let heavy = d.force >= ForceTier::Heavy;
        d.done_after = 120;
        if age == 0 {
            let (count, lo, hi) = if heavy {
                (24, 100.0, 220.0)
            } else {
                (12, 70.0, 150.0)
            };
            let n = scale_count(count, self.preset.drop_quarters());
            let head = (d.foot_x, d.ground_y + 26.0);
            for _ in 0..n {
                let elevation = d.rng.range(20.0, 70.0);
                let speed = d.rng.range(lo, hi);
                self.drop_blood(d, head, d.away, elevation, speed);
            }
            // Chunks knocked loose: the kick throws two flesh gibs and a bone,
            // a jab one flesh gib.
            let gibs: &[&str] = if heavy {
                &["gib_flesh_a", "gib_flesh_b", "gib_bone"]
            } else {
                &["gib_flesh_a"]
            };
            let allowed: Vec<&str> = gibs
                .iter()
                .copied()
                .filter(|id| {
                    self.art
                        .part_index(id)
                        .is_some_and(|i| self.art.parts[i].allowed_at(self.preset))
                })
                .collect();
            let keep = scale_count(allowed.len() as u32, self.preset.gib_quarters()) as usize;
            for id in allowed.into_iter().take(keep) {
                let (sin, cos) = d.rng.range(30.0, 70.0).to_radians().sin_cos();
                let speed = d.rng.range(80.0, 160.0);
                let spin = if d.rng.coin() { 720.0 } else { -720.0 };
                self.launch(
                    d,
                    id,
                    None,
                    head,
                    (d.away * cos * speed, sin * speed),
                    0.0,
                    spin,
                    false,
                );
            }
            // The kick's blow throws a spray up the back wall (full only).
            if heavy {
                let flip = d.away < 0.0;
                self.wall_spatter_variant(
                    "decal_wall_spatter",
                    flip,
                    (d.foot_x, d.ground_y + 14.0),
                );
                self.wall_spatter_variant(
                    "decal_wall_spatter_b",
                    d.rng.coin(),
                    (d.foot_x + d.away * 10.0, d.ground_y + 16.0),
                );
            }
        }
        // The goblin has landed (lying down): stain the floor under it.
        if d.landed.is_none()
            && age > 0
            && obs
                .goblins
                .get(d.target as usize)
                .is_some_and(|g| matches!(g.mode, GoblinMode::Down | GoblinMode::Corpse))
        {
            d.landed = Some(age);
            if let Some(g) = obs.goblins.get(d.target as usize) {
                d.foot_x = g.x;
            }
        }
        if let Some(landed) = d.landed {
            let since = age - landed;
            let x = d.foot_x;
            let (stage1, stage2, stage3) = if heavy {
                (3, 14, 36)
            } else {
                (4, 24, u64::MAX)
            };
            if since == 0 {
                if self.spawn_floor_decal("decal_splat_s", x, d.ground_y, false) {
                    d.part_stains += 1;
                }
                self.spawn_floor_decal("decal_splat_m", x, d.ground_y, false);
                if heavy {
                    // The slide leaves a smear (full only), toward the way
                    // the body travelled.
                    self.spawn_floor_decal("decal_smear", x, d.ground_y, d.away < 0.0);
                }
            }
            for (tick, id) in [
                (stage1, "decal_pool_1"),
                (stage2, "decal_pool_2"),
                (stage3, "decal_pool_3"),
            ] {
                if since == tick {
                    self.pool_stage(d, id, x);
                }
            }
        }
    }

    /// A digest of every piece, decal and pose of the presenter (determinism
    /// tests and the headless trace compare it).
    pub fn digest(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        let mut mix = |v: u64| {
            hash ^= v;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        };
        mix(self.clock);
        for (slot, p) in self.parts.iter() {
            for v in [p.x, p.y, p.vx, p.vy, p.angle_deg] {
                mix(u64::from(v.to_bits()));
            }
            mix((slot as u64) << 8 | p.frame as u64);
        }
        for pool in [&self.floor_decals, &self.wall_decals] {
            for (slot, d) in pool.iter() {
                mix(u64::from(d.x.to_bits()));
                mix(u64::from(d.y.to_bits()));
                mix((slot as u64) << 8 | d.meta as u64 | u64::from(d.flip) << 40);
                mix(d.pair_meta.map_or(0, |p| p as u64 + 1));
            }
        }
        for pose in self.poses() {
            mix(u64::from(pose.target) << 8 | pose.frame as u64);
        }
        hash
    }

    /// One summary line for the headless trace.
    pub fn state_line(&self) -> String {
        format!(
            "DEATH_PRESENT preset={} clock={} deaths={} parts={} floor_decals={} wall_decals={} spawned_parts={} digest={:016x}",
            self.preset.level(),
            self.clock,
            self.deaths.len(),
            self.parts.live(),
            self.floor_decals.live(),
            self.wall_decals.live(),
            self.stats.parts_spawned,
            self.digest(),
        )
    }
}

impl Part {
    fn rest(&mut self, clock: u64, hull: &[(f32, f32)]) {
        // Lie as flat as the piece goes (an eighth turn), on the floor line.
        self.angle_deg = rest_angle(hull, self.flip, self.angle_deg);
        self.vx = 0.0;
        self.vy = 0.0;
        self.spin_dps = 0.0;
        self.y = self.ground_y + lowest_below_centre(hull, self.flip, self.angle_deg);
        self.state = PartState::Resting { since: clock };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_scene::{Fixture, FixtureSim};
    use crate::env_kit::EnvKit;
    use crate::gore::{PartClass, lowest_below_centre};
    use std::path::{Path, PathBuf};

    const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";
    const SHORE_KILL_HASH: u64 = 0x098e_86cd_e2e6_b065;
    const KICK_BLOCK_HASH: u64 = 0x73e4_0916_1ce5_434f;
    const KICK_HASH: u64 = 0x9e13_d646_1914_9b71;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn art() -> Arc<DeathArt> {
        Arc::new(DeathArt::load(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap())
    }

    fn presenter(preset: GorePreset) -> DeathPresenter {
        DeathPresenter::new(art(), preset).unwrap()
    }

    /// A goblin standing at `x` facing left, on a floor at y 100.
    fn goblin_at(x: f32) -> GoblinView {
        GoblinView {
            x,
            ground_y: 100.0,
            facing: -1,
            mode: GoblinMode::Down,
        }
    }

    fn defeat(tick: u64, target: u32, cause: DefeatCause, force: ForceTier) -> CombatEvent {
        CombatEvent::EnemyDefeated {
            tick,
            target,
            cause,
            force,
            instigator: Some(0),
        }
    }

    const BLOCK: DefeatCause = DefeatCause::ShoreBlock { player: 0 };

    fn obs(goblins: &[GoblinView]) -> Observation<'_> {
        Observation {
            goblins,
            hero_x: 400.0,
            block_x: Some(484.0),
            back_wall: true,
        }
    }

    /// Run a crush (or lab finish) for `ticks` ticks; returns the presenter.
    fn run_crush(preset: GorePreset, ticks: u64) -> DeathPresenter {
        let mut p = presenter(preset);
        let goblins = [goblin_at(484.0)];
        p.step(
            &[defeat(139, 0, BLOCK, ForceTier::Catastrophic)],
            obs(&goblins),
        );
        for _ in 1..ticks {
            p.step(&[], obs(&goblins));
        }
        p
    }

    fn run_decap(preset: GorePreset, ticks: u64) -> DeathPresenter {
        let mut p = presenter(preset);
        p.set_lab_finish(Some(Finish::Decapitated));
        let goblins = [goblin_at(484.0)];
        p.step(
            &[defeat(50, 0, DefeatCause::Melee, ForceTier::Routine)],
            obs(&goblins),
        );
        for _ in 1..ticks {
            p.step(&[], obs(&goblins));
        }
        p
    }

    fn part_ids(p: &DeathPresenter) -> Vec<String> {
        p.parts()
            .map(|(_, part)| p.art().parts[part.meta].id.clone())
            .collect()
    }

    fn floor_ids(p: &DeathPresenter) -> Vec<String> {
        p.floor_decals()
            .map(|(_, d)| p.art().parts[d.meta].id.clone())
            .collect()
    }

    fn count(ids: &[String], id: &str) -> usize {
        ids.iter().filter(|i| i.as_str() == id).count()
    }

    // ---------------------------------------------------------------- pool ----

    #[test]
    fn the_pool_is_fixed_size_and_recycles_the_oldest_first() {
        let mut pool: Pool<u32> = Pool::new(3);
        let a = pool.spawn(10);
        let b = pool.spawn(11);
        let c = pool.spawn(12);
        assert_eq!((pool.live(), pool.capacity()), (3, 3));
        assert_eq!(pool.evicted(), 0);
        // Full: the next spawn takes the slot of the oldest entry.
        let d = pool.spawn(13);
        assert_eq!(d.0, a.0);
        assert_eq!((pool.live(), pool.capacity(), pool.evicted()), (3, 3, 1));
        assert!(
            pool.get_seq_mut(a.0, a.1).is_none(),
            "the old entry is gone"
        );
        assert_eq!(pool.get(b.0), Some(&11));
        let e = pool.spawn(14);
        assert_eq!(e.0, b.0, "then the next oldest");
        assert_eq!(pool.get(c.0), Some(&12));
        // A freed slot is reused before anything is recycled.
        pool.free(c.0);
        let f = pool.spawn(15);
        assert_eq!(f.0, c.0);
        assert_eq!(pool.evicted(), 2);
        pool.clear();
        assert_eq!(pool.live(), 0);
    }

    #[test]
    fn presenter_pools_never_exceed_their_capacity_under_stress() {
        let mut p = presenter(GorePreset::Full);
        for _ in 0..1500 {
            p.stress_tick(400.0, 100.0);
            p.step(&[], obs(&[]));
            assert!(p.live_parts() <= PART_POOL_CAPACITY);
            assert!(p.live_floor_decals() <= FLOOR_DECAL_POOL_CAPACITY);
            assert!(p.live_wall_decals() <= WALL_DECAL_POOL_CAPACITY);
            assert!(p.deaths.len() <= MAX_DEATHS);
        }
        // The pools were really loaded, and recycled rather than grown.
        assert_eq!(p.live_floor_decals(), FLOOR_DECAL_POOL_CAPACITY);
        assert_eq!(p.live_wall_decals(), WALL_DECAL_POOL_CAPACITY);
        let (parts, floors, walls) = p.recycled();
        assert!(floors > 0 && walls > 0, "decals recycled: {floors} {walls}");
        assert!(p.stats().peak_parts > 100, "{:?}", p.stats());
        let _ = parts;
    }

    #[test]
    fn a_two_cell_pool_decal_is_one_pool_entry_spawned_and_recycled_as_a_unit() {
        let mut p = presenter(GorePreset::Full);
        let right = p.art().part_index("decal_pool_3_r").unwrap();
        let left = p.art().part_index("decal_pool_3").unwrap();
        let mut d = p.deaths_for_test();
        p.pool_stage(&mut d, "decal_pool_3", 200.0);
        assert_eq!(p.live_floor_decals(), 1, "the pair is a single entry");
        let (_, decal) = p.floor_decals().next().unwrap();
        assert_eq!((decal.meta, decal.pair_meta), (left, Some(right)));
        // Fill the pool: the pair goes as one entry when it is the oldest.
        for i in 0..FLOOR_DECAL_POOL_CAPACITY {
            p.spawn_floor_decal("decal_splat_s", i as f32, 100.0, false);
        }
        assert_eq!(p.live_floor_decals(), FLOOR_DECAL_POOL_CAPACITY);
        assert!(
            p.floor_decals().all(|(_, d)| d.pair_meta.is_none()),
            "the pair was recycled whole"
        );
        // The pair's cells line up: the right cell abuts the left by the
        // manifest offset, on the same floor row.
        let meta = &p.art().parts[left];
        assert_eq!(meta.pair_offset, Some([30, 0]));
    }

    impl DeathPresenter {
        fn deaths_for_test(&mut self) -> Death {
            Death {
                id: 99,
                target: SYNTHETIC_TARGET,
                finish: Finish::Crushed,
                force: ForceTier::Catastrophic,
                start: 0,
                foot_x: 200.0,
                ground_y: 100.0,
                flip: false,
                away: 1.0,
                rng: PresentationRng::new(1),
                drop_splats: 0,
                part_stains: 0,
                drops_emitted: 0,
                pool: None,
                landed: None,
                done_after: 0,
            }
        }
    }

    // ------------------------------------------------------- spec counts ----

    #[test]
    fn the_crush_follows_the_spec_poses_and_counts_at_preset_four() {
        let mut p = presenter(GorePreset::Full);
        let goblins = [goblin_at(484.0)];
        p.step(
            &[defeat(139, 0, BLOCK, ForceTier::Catastrophic)],
            obs(&goblins),
        );
        // L: pose 0, the club leaves, 16 to 20 drops and 3 gibs.
        assert_eq!(p.pose_of(0).unwrap().frame, 0);
        let ids = part_ids(&p);
        assert_eq!(count(&ids, "club"), 1);
        let drops = count(&ids, "drop_blood");
        assert!((16..=20).contains(&drops), "{drops} drops");
        assert_eq!(count(&ids, "gib_flesh_a"), 1);
        assert_eq!(count(&ids, "gib_flesh_b"), 1);
        assert_eq!(count(&ids, "gib_bone"), 1);
        // The pose timing: 0 for 3 ticks, 1 for 3, then the remains.
        let mut frames = vec![p.pose_of(0).unwrap().frame];
        for _ in 0..8 {
            p.step(&[], obs(&goblins));
            frames.push(p.pose_of(0).unwrap().frame);
        }
        assert_eq!(frames, [0, 0, 0, 1, 1, 1, 2, 2, 2]);
    }

    #[test]
    fn the_crush_pool_grows_in_place_on_the_spec_ticks() {
        let mut p = presenter(GorePreset::Full);
        let goblins = [goblin_at(484.0)];
        p.step(
            &[defeat(139, 0, BLOCK, ForceTier::Catastrophic)],
            obs(&goblins),
        );
        let stage = |p: &DeathPresenter| -> Vec<String> {
            floor_ids(p)
                .into_iter()
                .filter(|i| i.starts_with("decal_pool"))
                .collect()
        };
        let mut seen = Vec::new();
        for age in 1..=65u64 {
            p.step(&[], obs(&goblins));
            let pools = stage(&p);
            assert!(pools.len() <= 1, "one growing pool decal, not several");
            if let Some(id) = pools.first()
                && seen.last() != Some(&(id.clone(), age))
                && seen.last().map(|(i, _)| i) != Some(id)
            {
                seen.push((id.clone(), age));
            }
        }
        assert_eq!(
            seen,
            [
                ("decal_pool_1".to_string(), 2),
                ("decal_pool_2".to_string(), 10),
                ("decal_pool_3".to_string(), 24),
            ]
        );
        // L+8: three wall spatters (both variants) on the back wall, none on
        // the floor layer.
        assert_eq!(p.live_wall_decals(), 3);
        assert!(!floor_ids(&p).iter().any(|i| i.contains("wall_spatter")));
        // L+4: a medium splat at each squeeze point.
        assert_eq!(count(&floor_ids(&p), "decal_splat_m"), 2);
    }

    #[test]
    fn dropped_blood_leaves_at_most_eight_splats_per_death() {
        let p = run_crush(GorePreset::Full, 200);
        let splats = count(&floor_ids(&p), "decal_splat_s");
        assert!(
            splats <= usize::from(MAX_DROP_SPLATS + MAX_PART_STAINS),
            "{splats}"
        );
        assert!(splats >= 4, "{splats}");
        // Every drop landed and was recycled: none left flying.
        assert_eq!(count(&part_ids(&p), "drop_blood"), 0);
    }

    #[test]
    fn the_decapitation_follows_the_spec_poses_head_and_swap() {
        let mut p = presenter(GorePreset::Full);
        p.set_lab_finish(Some(Finish::Decapitated));
        let goblins = [goblin_at(484.0)];
        p.step(
            &[defeat(50, 0, DefeatCause::Melee, ForceTier::Routine)],
            obs(&goblins),
        );
        let mut frames = vec![p.pose_of(0).unwrap().frame];
        for _ in 0..12 {
            p.step(&[], obs(&goblins));
            frames.push(p.pose_of(0).unwrap().frame);
        }
        assert_eq!(frames, [3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 5, 5, 5]);
        // The head is launched on the blade tick and swaps to head_dead on its
        // first floor contact.
        assert_eq!(count(&part_ids(&p), "head"), 1);
        for _ in 0..90 {
            p.step(&[], obs(&goblins));
        }
        let ids = part_ids(&p);
        assert_eq!(count(&ids, "head_dead"), 1, "{ids:?}");
        assert_eq!(count(&ids, "head"), 0);
        let head = p.parts().find(|(_, part)| part.is_resting()).unwrap().1;
        assert!(head.is_resting());
        // Pool stages 1, 2, 3 at D+14, D+40, D+100 (the clock is 103 here).
        let pools: Vec<_> = floor_ids(&p)
            .into_iter()
            .filter(|i| i.starts_with("decal_pool"))
            .collect();
        assert_eq!(pools, ["decal_pool_3"]);
        // The head's roll left a smear, and the jet a wall spatter.
        assert!(floor_ids(&p).iter().any(|i| i == "decal_smear"));
        assert_eq!(p.live_wall_decals(), 1);
    }

    #[test]
    fn the_jet_emits_two_drops_every_three_ticks_for_thirty_ticks() {
        let mut p = presenter(GorePreset::Full);
        p.set_lab_finish(Some(Finish::Decapitated));
        let goblins = [goblin_at(484.0)];
        p.step(
            &[defeat(50, 0, DefeatCause::Melee, ForceTier::Routine)],
            obs(&goblins),
        );
        let mut jet = 0;
        for _ in 1..36 {
            let before = p.stats().parts_spawned;
            p.step(&[], obs(&goblins));
            jet += p.stats().parts_spawned - before;
        }
        // 10 emissions of 2 from the jet, plus the trickle's drops from D+10
        // (every 8 ticks, 4 emissions up to D+34).
        let trickle = (10..36u64).filter(|a| (a - 10) % 8 == 0).count() as u64;
        assert_eq!(jet, 20 + trickle);
    }

    // ------------------------------------------------------ preset filters ----

    #[test]
    fn preset_one_draws_no_blood_no_gore_parts_and_no_gibs() {
        for p in [
            run_crush(GorePreset::Off, 80),
            run_decap(GorePreset::Off, 120),
        ] {
            for (_, part) in p.parts() {
                let meta = &p.art().parts[part.meta];
                assert_ne!(meta.class, PartClass::Blood, "{}", meta.id);
                assert_ne!(meta.class, PartClass::Gib, "{}", meta.id);
                assert!(meta.allowed_at(GorePreset::Off), "{}", meta.id);
            }
            assert_eq!(p.live_floor_decals(), 0);
            assert_eq!(p.live_wall_decals(), 0);
        }
        // The crush still clatters the club; the decapitation still flies the head.
        assert!(part_ids(&run_crush(GorePreset::Off, 80)).contains(&"club".to_string()));
        let decap = part_ids(&run_decap(GorePreset::Off, 120));
        assert!(decap.contains(&"head_dead".to_string()), "{decap:?}");
        // And the sheets the adapter uploads hold not one gore-layer colour.
        let sheets = run_crush(GorePreset::Off, 1).sheets().clone();
        let blood = [0x6e, 0x1a, 0x22, 0xff];
        let wet = [0xe2, 0x43, 0x4b, 0xff];
        for sheet in [&sheets.poses, &sheets.parts] {
            for frame in 0..sheet.frames {
                let colours = sheet.colours(frame);
                assert!(!colours.contains(&blood) && !colours.contains(&wet));
            }
        }
    }

    #[test]
    fn gib_bone_the_wall_spatters_and_pool_three_exist_at_preset_four_only() {
        let seen = |preset: GorePreset| -> (Vec<String>, Vec<String>, usize) {
            let p = run_crush(preset, 80);
            (part_ids(&p), floor_ids(&p), p.live_wall_decals())
        };
        for preset in [GorePreset::Off, GorePreset::Reduced, GorePreset::Medium] {
            let (parts, floors, walls) = seen(preset);
            assert_eq!(count(&parts, "gib_bone"), 0, "{preset:?}");
            assert_eq!(count(&floors, "decal_pool_3"), 0, "{preset:?}");
            assert_eq!(walls, 0, "{preset:?}");
        }
        let (parts, floors, walls) = seen(GorePreset::Full);
        assert_eq!(count(&parts, "gib_bone"), 1);
        assert_eq!(count(&floors, "decal_pool_3"), 1);
        assert_eq!(walls, 3);
    }

    #[test]
    fn counts_scale_down_with_the_preset() {
        let drops_at = |preset: GorePreset| {
            let mut p = presenter(preset);
            let goblins = [goblin_at(484.0)];
            p.step(
                &[defeat(139, 0, BLOCK, ForceTier::Catastrophic)],
                obs(&goblins),
            );
            let ids = part_ids(&p);
            (
                count(&ids, "drop_blood"),
                count(&ids, "gib_flesh_a") + count(&ids, "gib_flesh_b") + count(&ids, "gib_bone"),
            )
        };
        let (d4, g4) = drops_at(GorePreset::Full);
        let (d3, g3) = drops_at(GorePreset::Medium);
        let (d2, g2) = drops_at(GorePreset::Reduced);
        let (d1, g1) = drops_at(GorePreset::Off);
        assert_eq!((g4, g3, g2, g1), (3, 1, 0, 0));
        assert!(d4 > d3 && d3 > d2 && d2 > d1, "{d4} {d3} {d2} {d1}");
        assert!((16..=20).contains(&d4));
        assert!((8..=11).contains(&d3), "{d3}");
        assert!((4..=6).contains(&d2), "{d2}");
        assert_eq!(d1, 0);
    }

    #[test]
    fn preset_three_and_four_differ_visibly() {
        let digest = |preset: GorePreset| run_crush(preset, 80).digest();
        let all: Vec<u64> = GorePreset::ALL.into_iter().map(digest).collect();
        for i in 0..4 {
            for j in i + 1..4 {
                assert_ne!(all[i], all[j], "presets {} and {}", i + 1, j + 1);
            }
        }
    }

    // ----------------------------------------------------- floor and order ----

    #[test]
    fn no_part_ever_goes_below_the_floor_line() {
        for preset in GorePreset::ALL {
            let mut p = presenter(preset);
            p.set_lab_finish(Some(Finish::Decapitated));
            let goblins = [goblin_at(484.0), goblin_at(520.0)];
            p.step(
                &[
                    defeat(1, 0, DefeatCause::Melee, ForceTier::Routine),
                    defeat(1, 1, BLOCK, ForceTier::Catastrophic),
                ],
                obs(&goblins),
            );
            for tick in 0..420 {
                p.step(&[], obs(&goblins));
                if tick % 30 == 0 {
                    p.stress_tick(400.0, 100.0);
                }
                for (_, part) in p.parts() {
                    let hull = &p.sheets().parts.hull[part.frame];
                    let low = lowest_below_centre(hull, part.flip, part.angle_deg);
                    assert!(
                        part.y - low >= part.ground_y - 1e-3,
                        "{preset:?} tick {tick}: {} below the floor",
                        p.art().parts[part.meta].id
                    );
                }
            }
        }
    }

    #[test]
    fn a_resting_part_lies_flat_on_the_floor_within_twelve_pixels() {
        let p = run_crush(GorePreset::Full, 100);
        let mut resting = 0;
        for (_, part) in p.parts() {
            if !part.is_resting() {
                continue;
            }
            resting += 1;
            let hull = &p.sheets().parts.hull[part.frame];
            let low = lowest_below_centre(hull, part.flip, part.angle_deg);
            assert!((part.y - low - part.ground_y).abs() < 1e-3);
            assert_eq!(part.angle_deg % 45.0, 0.0, "settled at an eighth turn");
            // Lying: no piece stands higher than 12 px (the spec's stack cap).
            let (sin, cos) = part.angle_deg.to_radians().sin_cos();
            let top = hull
                .iter()
                .map(|&(x, y)| (if part.flip { -x } else { x }) * sin + y * cos)
                .fold(f32::NEG_INFINITY, f32::max);
            assert!(
                top + low <= 12.5,
                "{} stands {} px",
                p.art().parts[part.meta].id,
                top + low
            );
        }
        assert!(resting >= 2);
    }

    #[test]
    fn debris_and_decals_always_draw_under_the_actors() {
        // The actor layer of the Floor 1 kit is 7 and the backwall decal layer
        // 3; the lowest actor z is a lying goblin (-0.01) with a 0.001 step per
        // goblin index, Carl is +0.1 and Donut +0.0.
        let (actor_z, wall_z) = (7.0f32, 3.0f32);
        let lowest_actor = actor_z - 0.01 - 0.001 * 64.0;
        let highest_slot = PART_POOL_CAPACITY.max(FLOOR_DECAL_POOL_CAPACITY);
        for slot in [0, 1, highest_slot - 1] {
            let debris = draw_z(DrawLayer::Debris, slot, actor_z, wall_z);
            let floor = draw_z(DrawLayer::FloorDecal, slot, actor_z, wall_z);
            assert!(debris < lowest_actor, "debris {debris} over an actor");
            assert!(floor < lowest_actor, "decal {floor} over an actor");
            assert!(
                floor < draw_z(DrawLayer::Debris, 0, actor_z, wall_z),
                "decals under debris"
            );
            assert!(debris > actor_z - 1.0, "debris stays just under the actors");
            let wall = draw_z(DrawLayer::WallDecal, slot, actor_z, wall_z);
            assert!(
                wall > wall_z && wall < wall_z + 1.0,
                "wall spatter on its own layer"
            );
        }
    }

    #[test]
    fn launched_parts_are_never_larger_than_the_spec_cap_and_decals_stay_flat() {
        let p = presenter(GorePreset::Full);
        let sheet = &p.sheets().parts;
        for meta in &p.art().parts {
            let Some(b) = sheet.bbox[meta.frame] else {
                continue;
            };
            if meta.floor_decal {
                assert!(b.width() <= 40 && b.height() <= 4, "{}", meta.id);
            } else if meta.class != PartClass::Blood || meta.id == "drop_blood" {
                assert!(b.width() <= 28 && b.height() <= 28, "{}", meta.id);
            }
        }
    }

    // -------------------------------------------------- randomness, hashes ----

    #[test]
    fn presentation_randomness_is_event_seeded_and_never_the_simulation_seed() {
        // Same event, same cosmetics; another attempt or tick, other cosmetics.
        let a = run_crush(GorePreset::Full, 40).digest();
        assert_eq!(a, run_crush(GorePreset::Full, 40).digest());
        let mut p = presenter(GorePreset::Full);
        p.attempt = 1;
        let goblins = [goblin_at(484.0)];
        p.step(
            &[defeat(139, 0, BLOCK, ForceTier::Catastrophic)],
            obs(&goblins),
        );
        for _ in 1..40 {
            p.step(&[], obs(&goblins));
        }
        assert_ne!(a, p.digest(), "the attempt is part of the seed");
    }

    fn replay(
        fixture: &'static Fixture,
        preset: GorePreset,
        presenter_seed: Option<u32>,
    ) -> (u64, Vec<CombatEvent>, DeathPresenter) {
        let kit = EnvKit::load(&root(), LAYOUT).unwrap();
        let mut sim = FixtureSim::new(&kit, 360, fixture).unwrap();
        let mut presenter = presenter(preset);
        if let Some(seed) = presenter_seed {
            presenter.attempt = seed;
        }
        for _ in 0..fixture.ticks {
            sim.step_scripted();
            presenter.observe(&sim.combat, &sim.scene);
        }
        (sim.state_hash(), sim.combat.events.clone(), presenter)
    }

    #[test]
    fn the_same_replay_at_preset_one_and_four_has_the_identical_replay_hash() {
        // The required test of the death-system proposal: the preset is
        // presentation only, so every authoritative byte is the same.
        for (fixture, golden) in [
            (&crate::combat_scene::CP03_SHORE_KILL, SHORE_KILL_HASH),
            (&crate::combat_scene::CP03_KICK_BLOCK, KICK_BLOCK_HASH),
            (&crate::combat_scene::CP01_KICK, KICK_HASH),
        ] {
            let mut results = Vec::new();
            for preset in GorePreset::ALL {
                let (hash, events, p) = replay(fixture, preset, None);
                assert_eq!(
                    hash,
                    golden,
                    "{} at preset {}",
                    fixture.name,
                    preset.level()
                );
                results.push((events, p.digest(), p.stats().parts_spawned));
            }
            // Identical events at every preset ...
            for r in &results[1..] {
                assert_eq!(r.0, results[0].0, "{}", fixture.name);
            }
            // ... while the presentation really differs (preset 1 spawns
            // fewer pieces than preset 4 and the digests differ).
            assert!(results[0].2 < results[3].2, "{}", fixture.name);
            assert_ne!(results[0].1, results[3].1, "{}", fixture.name);
        }
    }

    #[test]
    fn a_different_presentation_seed_never_changes_the_simulation() {
        let (hash_a, events_a, pa) = replay(
            &crate::combat_scene::CP03_SHORE_KILL,
            GorePreset::Full,
            None,
        );
        let (hash_b, events_b, pb) = replay(
            &crate::combat_scene::CP03_SHORE_KILL,
            GorePreset::Full,
            Some(7),
        );
        assert_eq!(hash_a, hash_b);
        assert_eq!(events_a, events_b);
        assert_ne!(pa.digest(), pb.digest(), "only the cosmetics move");
    }

    #[test]
    fn the_scripted_fixtures_replay_the_same_presentation_twice() {
        for fixture in [
            &crate::combat_scene::CP03_SHORE_KILL,
            &crate::combat_scene::CP01_KICK,
        ] {
            let a = replay(fixture, GorePreset::Full, None).2.digest();
            let b = replay(fixture, GorePreset::Full, None).2.digest();
            assert_eq!(a, b, "{}", fixture.name);
        }
    }

    // ------------------------------------------------------------ fixtures ----

    #[test]
    fn the_shore_crush_plays_the_crush_and_the_kick_kill_stays_a_knockback() {
        let (_, _, crush) = replay(
            &crate::combat_scene::CP03_SHORE_KILL,
            GorePreset::Full,
            None,
        );
        let pose = crush
            .pose_of(0)
            .expect("the crushed goblin is drawn in a death pose");
        assert_eq!(pose.frame, 2, "the remains hold at the end of the fixture");
        assert!(count(&floor_ids(&crush), "decal_pool_3") >= 1);
        let (_, events, kick) = replay(&crate::combat_scene::CP01_KICK, GorePreset::Full, None);
        assert!(events.iter().any(|e| matches!(
            e,
            CombatEvent::EnemyDefeated {
                force: ForceTier::Heavy,
                cause: DefeatCause::Melee,
                ..
            }
        )));
        assert!(
            kick.pose_of(0).is_none(),
            "the knockback keeps the GOB-01 frames"
        );
        // Blood on the floor where the goblin landed, and a pool.
        let floors = floor_ids(&kick);
        assert!(floors.iter().any(|i| i == "decal_splat_s"), "{floors:?}");
        assert!(
            floors.iter().any(|i| i.starts_with("decal_pool")),
            "{floors:?}"
        );
        // The kick kill is heavier than the jab kill.
        let (_, _, jab) = replay(
            &crate::combat_scene::CP01_SINGLE_GOBLIN,
            GorePreset::Full,
            None,
        );
        assert!(kick.stats().parts_spawned > jab.stats().parts_spawned);
        assert!(jab.stats().parts_spawned > 0);
    }

    #[test]
    fn a_finish_without_art_and_an_unknown_target_draw_nothing() {
        let mut p = presenter(GorePreset::Full);
        let goblins = [goblin_at(484.0)];
        // Target 5 does not exist.
        p.step(
            &[defeat(1, 5, BLOCK, ForceTier::Catastrophic)],
            obs(&goblins),
        );
        assert_eq!((p.live_parts(), p.live_floor_decals()), (0, 0));
        // A species with no goblin art: its finish ids have none.
        assert_eq!(Finish::from_id("skeleton_scatter"), Finish::NoArt);
        assert_eq!(Finish::from_id("generic_collapse"), Finish::NoArt);
        assert!(Finish::NoArt.pose_frame(0).is_none());
        let mut q = presenter(GorePreset::Full);
        q.species = "skeleton_mock";
        q.step(
            &[defeat(1, 0, BLOCK, ForceTier::Catastrophic)],
            obs(&goblins),
        );
        for _ in 0..30 {
            q.step(&[], obs(&goblins));
        }
        assert_eq!(
            q.live_parts() + q.live_floor_decals() + q.live_wall_decals(),
            0
        );
        assert!(q.pose_of(0).is_none());
    }

    #[test]
    fn a_restart_clears_the_pieces_and_moves_the_attempt_seed() {
        let mut p = run_crush(GorePreset::Full, 30);
        assert!(p.live_parts() > 0);
        let goblins = [goblin_at(484.0)];
        p.step(
            &[CombatEvent::Restart {
                tick: 300,
                deaths: 1,
            }],
            obs(&goblins),
        );
        assert_eq!(
            (p.live_parts(), p.live_floor_decals(), p.live_wall_decals()),
            (0, 0, 0)
        );
        assert!(p.pose_of(0).is_none());
        assert_eq!(p.attempt, 1);
    }

    #[test]
    fn faded_pieces_leave_the_pool_after_their_time_to_live() {
        let mut p = run_crush(GorePreset::Full, 1);
        let goblins = [goblin_at(484.0)];
        for _ in 0..(SHORT_TTL_TICKS + SHORT_FADE_TICKS + LONG_TTL_TICKS + LONG_FADE_TICKS + 60) {
            p.step(&[], obs(&goblins));
        }
        // Gibs (short) are gone; the club and the pools (keep) stay.
        assert_eq!(count(&part_ids(&p), "gib_flesh_a"), 0);
        assert_eq!(count(&part_ids(&p), "club"), 1);
        assert_eq!(
            count(&floor_ids(&p), "decal_splat_s"),
            0,
            "long splats faded"
        );
        assert_eq!(count(&floor_ids(&p), "decal_pool_3"), 1);
        assert_eq!(fade_alpha(Ttl::Short, 0, SHORT_TTL_TICKS), 1.0);
        let half = fade_alpha(Ttl::Short, 0, SHORT_TTL_TICKS + SHORT_FADE_TICKS / 2);
        assert!((half - 0.5).abs() < 1e-5);
        assert_eq!(fade_alpha(Ttl::Keep, 0, 1_000_000), 1.0);
    }

    #[test]
    fn preset_three_keeps_the_spec_pool_ticks_and_stops_at_stage_two() {
        let mut p = presenter(GorePreset::Medium);
        let goblins = [goblin_at(484.0)];
        p.step(
            &[defeat(139, 0, BLOCK, ForceTier::Catastrophic)],
            obs(&goblins),
        );
        let mut stages = Vec::new();
        for age in 1..=70u64 {
            p.step(&[], obs(&goblins));
            let pools = floor_ids(&p)
                .into_iter()
                .find(|i| i.starts_with("decal_pool"));
            if let Some(id) = pools
                && stages.last().map(|(i, _): &(String, u64)| i) != Some(&id)
            {
                stages.push((id, age));
            }
        }
        assert_eq!(
            stages,
            [
                ("decal_pool_1".to_string(), 4),
                ("decal_pool_2".to_string(), 20)
            ]
        );
    }

    #[test]
    fn with_no_wall_nearby_no_wall_spatter_is_drawn() {
        // The wall spatter is conditional on a wall face (spec section 8): a
        // route with open background passes `back_wall: false`.
        fn open(goblins: &[GoblinView]) -> Observation<'_> {
            Observation {
                goblins,
                hero_x: 400.0,
                block_x: Some(484.0),
                back_wall: false,
            }
        }
        let goblins = [goblin_at(484.0)];
        let mut crush = presenter(GorePreset::Full);
        crush.step(
            &[defeat(139, 0, BLOCK, ForceTier::Catastrophic)],
            open(&goblins),
        );
        let mut kick = presenter(GorePreset::Full);
        kick.step(
            &[defeat(260, 0, DefeatCause::Melee, ForceTier::Heavy)],
            open(&goblins),
        );
        let mut decap = presenter(GorePreset::Full);
        decap.set_lab_finish(Some(Finish::Decapitated));
        decap.step(
            &[defeat(50, 0, DefeatCause::Melee, ForceTier::Routine)],
            open(&goblins),
        );
        for p in [&mut crush, &mut kick, &mut decap] {
            for _ in 0..120 {
                p.step(&[], open(&goblins));
            }
            assert_eq!(p.live_wall_decals(), 0);
            assert!(p.live_floor_decals() > 0, "the floor blood stays");
        }
    }

    #[test]
    fn preset_four_kick_kill_is_heavier_than_the_jab_and_throws_gibs_and_a_wall_spray() {
        let run = |preset: GorePreset, force: ForceTier| {
            let mut p = presenter(preset);
            let goblins = [GoblinView {
                mode: GoblinMode::Airborne,
                ..goblin_at(500.0)
            }];
            p.step(&[defeat(260, 0, DefeatCause::Melee, force)], obs(&goblins));
            let first = part_ids(&p);
            let walls = p.live_wall_decals();
            let landed = [GoblinView {
                mode: GoblinMode::Down,
                ..goblin_at(540.0)
            }];
            for _ in 0..60 {
                p.step(&[], obs(&landed));
            }
            (first, walls, floor_ids(&p))
        };
        let (kick, kick_walls, kick_floor) = run(GorePreset::Full, ForceTier::Heavy);
        let (jab, jab_walls, _) = run(GorePreset::Full, ForceTier::Routine);
        assert_eq!(count(&kick, "drop_blood"), 24);
        assert_eq!(count(&jab, "drop_blood"), 12);
        assert_eq!(count(&kick, "gib_bone"), 1);
        assert_eq!(count(&jab, "gib_bone"), 0);
        assert_eq!((kick_walls, jab_walls), (2, 0));
        assert_eq!(count(&kick_floor, "decal_pool_3"), 1);
        assert!(count(&kick_floor, "decal_smear") >= 1);
        assert!(count(&kick_floor, "decal_splat_m") >= 1);
        // Preset 1 stays bloodless: nothing but nothing.
        let (off_parts, off_walls, off_floor) = run(GorePreset::Off, ForceTier::Heavy);
        assert!(off_parts.is_empty() && off_walls == 0 && off_floor.is_empty());
    }

    /// Cost of one presenter step with every pool full (stress load). Run with
    /// `cargo test --locked --lib -- --ignored --nocapture presenter_step_cost`.
    #[test]
    #[ignore = "timing probe, not a pass/fail test"]
    fn presenter_step_cost_at_max_load() {
        for preset in [GorePreset::Full, GorePreset::Off] {
            let mut p = presenter(preset);
            for _ in 0..600 {
                p.stress_tick(400.0, 100.0);
                p.step(&[], obs(&[]));
            }
            let started = std::time::Instant::now();
            let ticks = 3000;
            for _ in 0..ticks {
                p.stress_tick(400.0, 100.0);
                p.step(&[], obs(&[]));
            }
            let per_tick = started.elapsed().as_secs_f64() * 1e6 / f64::from(ticks);
            println!(
                "PRESENTER_STEP preset={} {per_tick:.1} us/tick parts={} floor_decals={} wall_decals={}",
                preset.level(),
                p.live_parts(),
                p.live_floor_decals(),
                p.live_wall_decals()
            );
        }
    }
}
