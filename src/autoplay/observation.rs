//! ObservationV1 (AP-Q06, AP-Q07): a fixed, bounded, player-like view of the
//! simulation.
//!
//! Everything on [`crate::combat_scene::FixtureSim`] is `pub`, so nothing here
//! is hidden by privacy: this module filters by policy. It exposes what a
//! player reads from the screen or knows from the rules and from memory, and
//! never exact enemy health, cooldowns, fuse timing, scripted triggers or
//! anything outside the game's own 640 px scroll-camera frame.
//!
//! [`observe`] is a pure function of the sim: same state, same observation, no
//! mutation. Clamped values, unknown goblin modes and overflow are reported in
//! [`ObsStats`], which only feeds diagnostics.
//!
//! Conventions: `dx` is positive to the right; `dy`, `vy` and every other
//! vertical quantity follow the sim (y-up: positive is up); only `y_in_view`
//! is screen-down (AP-Q06). `facing` is +1 right, -1 left. Vertical positions
//! are feet lines.

use crate::combat::{ENEMY_HALF_HEIGHT, ENEMY_HEALTH, HERO_HEALTH, MoveId};
use crate::combat_scene::FixtureSim;
use crate::feel::{LLAMA_HEALTH, LLAMA_TELL_TICKS};
use crate::goblin::{AttackPhase, Goblin, GoblinMode};
use crate::llama::{LLAMA_HALF_HEIGHT, Llama, LlamaMode};
use crate::prototype::HERO_HALF_HEIGHT;

/// Schema version of [`ObservationV1`] (`observation_id`).
pub const OBSERVATION_VERSION: u32 = 1;

pub const SELF_LEN: usize = 14;
pub const MAX_ENEMIES: usize = 16;
pub const ENEMY_FEATURES: usize = 14;
pub const MAX_HAZARDS: usize = 8;
pub const HAZARD_FEATURES: usize = 6;
pub const AVAILABLE_LEN: usize = 2;

/// Velocity normaliser in px/s (AP-Q06).
pub(crate) const VELOCITY_SCALE: f32 = 480.0;
/// The sim's scroll-camera width; the horizontal scale of `dx` and `x_in_view`.
pub(crate) const VIEW_W: f32 = 640.0;

/// The binding schema: names, order and bounds of every tensor (AP-Q06). Kept
/// as a constant so a test can pin it and a Gymnasium adapter can build
/// `observation_space` from it.
pub struct ObsSchema {
    pub version: u32,
    pub self_names: [&'static str; SELF_LEN],
    pub self_bounds: [(f32, f32); SELF_LEN],
    pub enemy_names: [&'static str; ENEMY_FEATURES],
    pub enemy_bounds: [(f32, f32); ENEMY_FEATURES],
    pub hazard_names: [&'static str; HAZARD_FEATURES],
    pub hazard_bounds: [(f32, f32); HAZARD_FEATURES],
    pub available_names: [&'static str; AVAILABLE_LEN],
}

pub(crate) const SIGNED: (f32, f32) = (-1.0, 1.0);
pub(crate) const UNIT: (f32, f32) = (0.0, 1.0);

/// ObservationV1 schema. All tensors are `float32`.
///
/// Sign convention (AP-Q06): `dy` and `vy` (own and enemy) follow the sim's
/// y-up frame, so positive is up; only `y_in_view` is screen-down (0 at the
/// top of the view). A bot author must not assume a screen-down `dy`.
pub const OBS_V1: ObsSchema = ObsSchema {
    version: OBSERVATION_VERSION,
    self_names: [
        "vx",
        "vy",
        "facing",
        "grounded",
        "health",
        "hurt",
        "invulnerable",
        "busy",
        "move_jab",
        "move_haymaker",
        "move_push_kick",
        "hit_stop",
        "x_in_view",
        "y_in_view",
    ],
    self_bounds: [
        SIGNED, SIGNED, SIGNED, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT,
    ],
    enemy_names: [
        "dx",
        "dy",
        "facing",
        "kind_goblin",
        "kind_llama",
        "vx",
        "vy",
        "grounded",
        "tell",
        "swing",
        "recover",
        "staggered",
        "launched",
        "damaged",
    ],
    enemy_bounds: [
        SIGNED, SIGNED, SIGNED, UNIT, UNIT, SIGNED, SIGNED, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT,
        UNIT,
    ],
    hazard_names: [
        "dx",
        "dy",
        "kind_block",
        "kind_charge",
        "kind_glob",
        "active",
    ],
    hazard_bounds: [SIGNED, SIGNED, UNIT, UNIT, UNIT, UNIT],
    available_names: ["special", "solo_swap"],
};

// Indices into the `self` vector.
pub const SELF_VX: usize = 0;
pub const SELF_VY: usize = 1;
pub const SELF_FACING: usize = 2;
pub const SELF_GROUNDED: usize = 3;
pub const SELF_HEALTH: usize = 4;
pub const SELF_HURT: usize = 5;
pub const SELF_INVULNERABLE: usize = 6;
pub const SELF_BUSY: usize = 7;
pub const SELF_MOVE_JAB: usize = 8;
pub const SELF_MOVE_HAYMAKER: usize = 9;
pub const SELF_MOVE_PUSH_KICK: usize = 10;
pub const SELF_HIT_STOP: usize = 11;
pub const SELF_X_IN_VIEW: usize = 12;
pub const SELF_Y_IN_VIEW: usize = 13;

// Indices into one `enemies` row.
pub const ENEMY_DX: usize = 0;
pub const ENEMY_DY: usize = 1;
pub const ENEMY_FACING: usize = 2;
pub const ENEMY_KIND_GOBLIN: usize = 3;
pub const ENEMY_KIND_LLAMA: usize = 4;
pub const ENEMY_VX: usize = 5;
pub const ENEMY_VY: usize = 6;
pub const ENEMY_GROUNDED: usize = 7;
pub const ENEMY_TELL: usize = 8;
pub const ENEMY_SWING: usize = 9;
pub const ENEMY_RECOVER: usize = 10;
pub const ENEMY_STAGGERED: usize = 11;
pub const ENEMY_LAUNCHED: usize = 12;
pub const ENEMY_DAMAGED: usize = 13;

/// One observation: the policy input and nothing else.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationV1 {
    pub own: [f32; SELF_LEN],
    pub enemies: [[f32; ENEMY_FEATURES]; MAX_ENEMIES],
    pub enemy_mask: [f32; MAX_ENEMIES],
    /// Always zero in v1: the v1 roster has no hazards.
    pub hazards: [[f32; HAZARD_FEATURES]; MAX_HAZARDS],
    pub hazard_mask: [f32; MAX_HAZARDS],
    /// `[special, solo_swap]`: both unavailable for Carl on this build.
    pub available: [f32; AVAILABLE_LEN],
}

impl ObservationV1 {
    pub const VERSION: u32 = OBSERVATION_VERSION;

    /// Every value, flattened in schema order (self, enemies, enemy_mask,
    /// hazards, hazard_mask, available), as raw bits for exact comparison.
    pub fn to_bits(&self) -> Vec<u32> {
        let mut out = Vec::with_capacity(
            SELF_LEN
                + MAX_ENEMIES * ENEMY_FEATURES
                + MAX_ENEMIES
                + MAX_HAZARDS * HAZARD_FEATURES
                + MAX_HAZARDS
                + AVAILABLE_LEN,
        );
        out.extend(self.own.iter().map(|v| v.to_bits()));
        out.extend(self.enemies.iter().flatten().map(|v| v.to_bits()));
        out.extend(self.enemy_mask.iter().map(|v| v.to_bits()));
        out.extend(self.hazards.iter().flatten().map(|v| v.to_bits()));
        out.extend(self.hazard_mask.iter().map(|v| v.to_bits()));
        out.extend(self.available.iter().map(|v| v.to_bits()));
        out
    }

    /// Number of populated enemy entries.
    pub fn visible_enemies(&self) -> usize {
        self.enemy_mask.iter().filter(|m| **m > 0.0).count()
    }
}

/// Counters of observation-side events; privileged (diagnostics only).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ObsStats {
    /// Values clamped to their declared bound.
    pub clipped: u32,
    /// Goblin modes this schema does not know (all mode flags stay zero).
    pub unknown_modes: u32,
    /// Visible enemies beyond the 16 entries (must stay zero; the roster
    /// validation rejects such scenarios).
    pub overflow: u32,
}

impl ObsStats {
    pub fn add(&mut self, other: ObsStats) {
        self.clipped += other.clipped;
        self.unknown_modes += other.unknown_modes;
        self.overflow += other.overflow;
    }
}

/// A feature clamped to its bound, counting the clamp.
pub(crate) fn bounded(value: f32, bounds: (f32, f32), stats: &mut ObsStats) -> f32 {
    let value = if value.is_nan() { 0.0 } else { value };
    let clamped = value.clamp(bounds.0, bounds.1);
    if clamped != value {
        stats.clipped += 1;
    }
    clamped
}

pub(crate) fn flag(on: bool) -> f32 {
    f32::from(u8::from(on))
}

/// One perceived enemy before ordering.
struct Seen {
    order_dx: f32,
    id: u32,
    row: [f32; ENEMY_FEATURES],
}

/// Build the observation of the current state. Pure: reads the sim only.
pub fn observe(sim: &FixtureSim) -> (ObservationV1, ObsStats) {
    let mut stats = ObsStats::default();
    let world = &sim.scene.world;
    let combat = &sim.combat;
    let hero = &world.hero;
    let cam = sim.scene.camera.config();
    let cam_left = sim.scene.camera.left() as f32;
    let cam_top = sim.scene.camera.top() as f32;
    let view_h = cam.view_h as f32;
    let hero_foot = hero.y - HERO_HALF_HEIGHT;

    // self
    let current = hero.melee.current().map(|m| m.spec.id);
    let layout_y = cam.layout_h as f32 - hero_foot;
    let own_raw = [
        hero.vx / VELOCITY_SCALE,
        hero.vy / VELOCITY_SCALE,
        f32::from(hero.facing),
        flag(hero.grounded),
        f32::from(hero.health) / f32::from(HERO_HEALTH),
        flag(combat.hurt_ticks > 0),
        flag(hero.invulnerable_ticks > 0),
        flag(hero.melee.busy()),
        flag(current == Some(MoveId::Jab)),
        flag(current == Some(MoveId::Haymaker)),
        flag(current == Some(MoveId::PushKick)),
        flag(hero.melee.hit_stop_left() > 0 || combat.hero_hit_stop > 0),
        (hero.x - cam_left) / VIEW_W,
        (layout_y - cam_top) / view_h,
    ];
    let mut own = [0.0; SELF_LEN];
    for (i, raw) in own_raw.into_iter().enumerate() {
        own[i] = bounded(raw, OBS_V1.self_bounds[i], &mut stats);
    }

    // enemies: goblins keep ids 0..n, llamas follow (the game's own target ids).
    let mut seen: Vec<Seen> = Vec::new();
    let goblin_count = combat.enemies.len() as u32;
    for (index, goblin) in combat.enemies.iter().enumerate() {
        if goblin.alive() && visible(sim, goblin.x) {
            let row = goblin_row(goblin, hero.x, hero_foot, view_h, &mut stats);
            seen.push(Seen {
                order_dx: (goblin.x - hero.x).abs(),
                id: index as u32,
                row,
            });
        }
    }
    for (index, llama) in combat.llamas.iter().enumerate() {
        if llama.health > 0 && visible(sim, llama.x) {
            let row = llama_row(llama, hero.x, hero_foot, view_h, &mut stats);
            seen.push(Seen {
                order_dx: (llama.x - hero.x).abs(),
                id: goblin_count + index as u32,
                row,
            });
        }
    }
    seen.sort_by(|a, b| a.order_dx.total_cmp(&b.order_dx).then(a.id.cmp(&b.id)));
    if seen.len() > MAX_ENEMIES {
        stats.overflow += (seen.len() - MAX_ENEMIES) as u32;
    }
    let mut enemies = [[0.0; ENEMY_FEATURES]; MAX_ENEMIES];
    let mut enemy_mask = [0.0; MAX_ENEMIES];
    for (slot, entry) in seen.iter().take(MAX_ENEMIES).enumerate() {
        enemies[slot] = entry.row;
        enemy_mask[slot] = 1.0;
    }
    stats.unknown_modes += combat
        .enemies
        .iter()
        .filter(|g| g.alive() && !known_goblin_mode(g.mode))
        .count() as u32;

    let obs = ObservationV1 {
        own,
        enemies,
        enemy_mask,
        hazards: [[0.0; HAZARD_FEATURES]; MAX_HAZARDS],
        hazard_mask: [0.0; MAX_HAZARDS],
        available: [0.0; AVAILABLE_LEN],
    };
    (obs, stats)
}

/// The perception mask (AP-Q07): the sim's own scroll-camera frame, margin 0.
fn visible(sim: &FixtureSim, x: f32) -> bool {
    !sim.scene.is_offscreen(x, 0.0)
}

pub(crate) fn known_goblin_mode(mode: GoblinMode) -> bool {
    matches!(
        mode,
        GoblinMode::Neutral
            | GoblinMode::Attack
            | GoblinMode::Hurt
            | GoblinMode::Airborne
            | GoblinMode::Down
            | GoblinMode::Corpse
            | GoblinMode::Launched
            | GoblinMode::Downed
    )
}

pub(crate) fn goblin_row(
    g: &Goblin,
    hero_x: f32,
    hero_foot: f32,
    view_h: f32,
    stats: &mut ObsStats,
) -> [f32; ENEMY_FEATURES] {
    let phase = g.attack_phase();
    let raw = [
        (g.x - hero_x) / VIEW_W,
        ((g.y - ENEMY_HALF_HEIGHT) - hero_foot) / view_h,
        f32::from(g.facing),
        1.0,
        0.0,
        g.vx / VELOCITY_SCALE,
        g.vy / VELOCITY_SCALE,
        flag(g.grounded()),
        flag(phase == Some(AttackPhase::Tell)),
        flag(matches!(
            phase,
            Some(AttackPhase::Smear | AttackPhase::Impact)
        )),
        flag(phase == Some(AttackPhase::Recover)),
        flag(g.mode == GoblinMode::Hurt),
        flag(matches!(g.mode, GoblinMode::Launched | GoblinMode::Downed)),
        flag(g.health < ENEMY_HEALTH),
    ];
    clamp_row(raw, stats)
}

pub(crate) fn llama_row(
    l: &Llama,
    hero_x: f32,
    hero_foot: f32,
    view_h: f32,
    stats: &mut ObsStats,
) -> [f32; ENEMY_FEATURES] {
    let attacking = l.mode == LlamaMode::Attack;
    let raw = [
        (l.x - hero_x) / VIEW_W,
        ((l.y - LLAMA_HALF_HEIGHT) - hero_foot) / view_h,
        f32::from(l.facing),
        0.0,
        1.0,
        l.vx / VELOCITY_SCALE,
        0.0,
        flag(l.y <= l.floor_y),
        flag(attacking && l.attack_tick < LLAMA_TELL_TICKS),
        0.0,
        flag(attacking && l.attack_tick >= LLAMA_TELL_TICKS),
        flag(l.mode == LlamaMode::Hurt),
        0.0,
        flag(l.health < LLAMA_HEALTH),
    ];
    clamp_row(raw, stats)
}

fn clamp_row(raw: [f32; ENEMY_FEATURES], stats: &mut ObsStats) -> [f32; ENEMY_FEATURES] {
    let mut row = [0.0; ENEMY_FEATURES];
    for (i, value) in raw.into_iter().enumerate() {
        row[i] = bounded(value, OBS_V1.enemy_bounds[i], stats);
    }
    row
}
