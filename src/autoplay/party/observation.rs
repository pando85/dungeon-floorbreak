//! ObservationV2 (AP-Q27, AP-Q28): the party observation of one hero.
//!
//! [`observe_v2`] is a pure function of `(&SceneSim, &CombatState,
//! &PartyState)` plus the hero whose view it builds, so the AUTOPLAY env
//! (seat = the policy's hero) and the live PLAYTRACE recorder (PT-C: once per
//! hero) share this one builder (PT-Q14).
//!
//! The view is the player's: the seat hero's own state, the other hero **as a
//! player sees it** (on screen, plus the HUD plate), and the v1 enemy layout
//! relative to the seat hero. The companion's AI internals are never read: no
//! follower trail, regroup counter, dodge latch, director decision or order
//! state. Every feature comes from `pub` views only, and a source test pins
//! that this file never names a path under `crate::companion` or
//! `crate::combat_scene::companion`.
//!
//! Donut's attacker freeze from her own moves (`donut_kit.moves.hit_stop_left`)
//! sits behind a private field and has no pub view, so `hit_stop` for Donut
//! only covers the blow freeze (`combat.donut.hit_stop`). Recorded gap: 02a
//! adds no accessor (it would break "no shared files").

use crate::combat::MoveId;
use crate::combat_scene::CombatState;
use crate::corridor::{DONUT_HEALTH, SceneSim};
use crate::donut_moves::MoveKind;
use crate::party::{HeroId, PartyState};
use crate::prototype::HERO_HALF_HEIGHT;

use super::super::observation::{
    AVAILABLE_LEN, ENEMY_FEATURES, HAZARD_FEATURES, MAX_ENEMIES, MAX_HAZARDS, ObsStats,
    ObservationV1, SELF_LEN, SIGNED, UNIT, VELOCITY_SCALE, VIEW_W, bounded, flag, goblin_row,
    known_goblin_mode, llama_row,
};

/// Schema version of [`ObservationV2`].
pub const OBSERVATION_VERSION_V2: u32 = 2;

/// `self` block length: the 14 v1 features plus 8 party features.
pub const SELF_V2_LEN: usize = 22;
/// `ally` block length: the same 22 features plus `dx` and `dy`.
pub const ALLY_LEN: usize = 24;

// Indices of the v2-only features (0..14 are the v1 indices).
pub const SELF_IS_CARL: usize = 14;
pub const SELF_IS_DONUT: usize = 15;
pub const SELF_MOVE_SWAT: usize = 16;
pub const SELF_MOVE_POUNCE: usize = 17;
pub const SELF_MOVE_MAGIC_MISSILE: usize = 18;
pub const SELF_MANA: usize = 19;
pub const SELF_MAGIC_UNLOCKED: usize = 20;
pub const SELF_CAN_CAST: usize = 21;
pub const ALLY_DX: usize = 22;
pub const ALLY_DY: usize = 23;

/// Ally features that stay when the ally is off screen: the HUD plate shows
/// health, identity, mana, the unlock and the segment-derived `can_cast` on or
/// off screen (AP-Q28). Everything else is zeroed with `ally_mask = 0`.
pub const ALLY_PLATE_FEATURES: [usize; 6] = [
    super::super::observation::SELF_HEALTH,
    SELF_IS_CARL,
    SELF_IS_DONUT,
    SELF_MANA,
    SELF_MAGIC_UNLOCKED,
    SELF_CAN_CAST,
];

/// The binding ObservationV2 schema (AP-Q27). All tensors are `float32`.
pub struct ObsSchemaV2 {
    pub version: u32,
    pub self_names: [&'static str; SELF_V2_LEN],
    pub self_bounds: [(f32, f32); SELF_V2_LEN],
    pub ally_names: [&'static str; ALLY_LEN],
    pub ally_bounds: [(f32, f32); ALLY_LEN],
}

pub const OBS_V2: ObsSchemaV2 = ObsSchemaV2 {
    version: OBSERVATION_VERSION_V2,
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
        "is_carl",
        "is_donut",
        "move_swat",
        "move_pounce",
        "move_magic_missile",
        "mana",
        "magic_unlocked",
        "can_cast",
    ],
    self_bounds: [
        SIGNED, SIGNED, SIGNED, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT,
        UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT,
    ],
    ally_names: [
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
        "is_carl",
        "is_donut",
        "move_swat",
        "move_pounce",
        "move_magic_missile",
        "mana",
        "magic_unlocked",
        "can_cast",
        "dx",
        "dy",
    ],
    ally_bounds: [
        SIGNED, SIGNED, SIGNED, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT,
        UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, UNIT, SIGNED, SIGNED,
    ],
};

/// One v2 observation: the policy input and nothing else.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationV2 {
    pub own: [f32; SELF_V2_LEN],
    pub ally: [f32; ALLY_LEN],
    pub ally_mask: [f32; 1],
    pub enemies: [[f32; ENEMY_FEATURES]; MAX_ENEMIES],
    pub enemy_mask: [f32; MAX_ENEMIES],
    /// Always zero: the `ap2a_*` roster has no hazards.
    pub hazards: [[f32; HAZARD_FEATURES]; MAX_HAZARDS],
    pub hazard_mask: [f32; MAX_HAZARDS],
    /// `[special, solo_swap]`: both unavailable for the seat (AP-Q26, AP-Q23).
    pub available: [f32; AVAILABLE_LEN],
}

impl ObservationV2 {
    pub const VERSION: u32 = OBSERVATION_VERSION_V2;

    /// Every value, flattened in schema order, as raw bits for exact comparison.
    pub fn to_bits(&self) -> Vec<u32> {
        let mut out = Vec::new();
        out.extend(self.own.iter().map(|v| v.to_bits()));
        out.extend(self.ally.iter().map(|v| v.to_bits()));
        out.extend(self.ally_mask.iter().map(|v| v.to_bits()));
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

    /// Whether the companion is on screen (`ally_mask`).
    pub fn ally_visible(&self) -> bool {
        self.ally_mask[0] > 0.0
    }

    /// The v1 view: `self[0..14]` plus the unchanged enemy, hazard and
    /// available blocks, so the stage-B bots run unchanged on either seat
    /// (AP-Q27). The replay header records `obs_projection = v1`.
    pub fn as_v1(&self) -> ObservationV1 {
        let mut own = [0.0; SELF_LEN];
        own.copy_from_slice(&self.own[..SELF_LEN]);
        ObservationV1 {
            own,
            enemies: self.enemies,
            enemy_mask: self.enemy_mask,
            hazards: self.hazards,
            hazard_mask: self.hazard_mask,
            available: self.available,
        }
    }
}

/// One hero's 22 party features before clamping, from `pub` views only.
fn hero_features(
    scene: &SceneSim,
    combat: &CombatState,
    party: &PartyState,
    hero: HeroId,
) -> [f32; SELF_V2_LEN] {
    let world = &scene.world;
    let body = world.body(hero);
    let cam = scene.camera.config();
    let cam_left = scene.camera.left() as f32;
    let cam_top = scene.camera.top() as f32;
    let view_h = cam.view_h as f32;
    let foot = body.y - HERO_HALF_HEIGHT;
    let layout_y = cam.layout_h as f32 - foot;

    let carl = hero == HeroId::Carl;
    let current = body.melee.current().map(|m| m.spec.id);
    let move_view = combat.donut_move_view();
    let cast = combat.donut_cast();
    let donut_busy = move_view.is_some() || cast.is_some();
    let magic = combat.donut_mana_view();
    let mana_max = f32::from(magic.map_or(1, |(_, max)| max.max(1)));
    let donut_moves = party.tuning().donut_moves;

    let health_max = f32::from(match hero {
        HeroId::Carl => crate::combat::HERO_HEALTH,
        HeroId::Donut => DONUT_HEALTH,
    });
    let hit_stop = if carl {
        body.melee.hit_stop_left() > 0 || combat.hero_hit_stop > 0
    } else {
        combat.donut.hit_stop > 0
    };
    let on_donut = !carl;
    [
        body.vx / VELOCITY_SCALE,
        body.vy / VELOCITY_SCALE,
        f32::from(body.facing),
        flag(body.grounded),
        f32::from(body.health) / health_max,
        flag(combat.hurt_ticks_of(hero) > 0),
        flag(body.invulnerable_ticks > 0),
        flag(if carl { body.melee.busy() } else { donut_busy }),
        flag(carl && current == Some(MoveId::Jab)),
        flag(carl && current == Some(MoveId::Haymaker)),
        flag(carl && current == Some(MoveId::PushKick)),
        flag(hit_stop),
        (body.x - cam_left) / VIEW_W,
        (layout_y - cam_top) / view_h,
        flag(carl),
        flag(on_donut),
        flag(on_donut && move_view.is_some_and(|m| m.mv == MoveKind::Swat)),
        flag(on_donut && move_view.is_some_and(|m| m.mv == MoveKind::Pounce)),
        flag(on_donut && cast.is_some()),
        match (on_donut, magic) {
            (true, Some((mana, _))) => f32::from(mana) / mana_max,
            _ => 0.0,
        },
        flag(on_donut && magic.is_some()),
        flag(
            on_donut
                && donut_moves
                && magic.is_some_and(|(mana, _)| mana >= combat.donut_mana_tuning().missile_cost),
        ),
    ]
}

fn clamp_features<const N: usize>(
    raw: [f32; N],
    bounds: &[(f32, f32); N],
    stats: &mut ObsStats,
) -> [f32; N] {
    let mut out = [0.0; N];
    for (i, value) in raw.into_iter().enumerate() {
        out[i] = bounded(value, bounds[i], stats);
    }
    out
}

/// Build the observation of `seat`'s view of the current state. Pure: reads
/// the sim only. `party` supplies the static rules the HUD makes visible
/// (whether Donut has moves at all), never hidden AI state.
pub fn observe_v2(
    scene: &SceneSim,
    combat: &CombatState,
    party: &PartyState,
    seat: HeroId,
) -> (ObservationV2, ObsStats) {
    let mut stats = ObsStats::default();
    let world = &scene.world;
    let cam = scene.camera.config();
    let view_h = cam.view_h as f32;
    let seat_body = world.body(seat);
    let seat_foot = seat_body.y - HERO_HALF_HEIGHT;

    let own = clamp_features(
        hero_features(scene, combat, party, seat),
        &OBS_V2.self_bounds,
        &mut stats,
    );

    // ally: the other hero, as the player sees it.
    let ally_hero = seat.other();
    let ally_body = world.body(ally_hero);
    let ally_visible = !scene.is_offscreen(ally_body.x, 0.0);
    let mut ally_raw = [0.0; ALLY_LEN];
    ally_raw[..SELF_V2_LEN].copy_from_slice(&hero_features(scene, combat, party, ally_hero));
    ally_raw[ALLY_DX] = (ally_body.x - seat_body.x) / VIEW_W;
    ally_raw[ALLY_DY] = ((ally_body.y - HERO_HALF_HEIGHT) - seat_foot) / view_h;
    if !ally_visible {
        for (i, value) in ally_raw.iter_mut().enumerate() {
            if !ALLY_PLATE_FEATURES.contains(&i) {
                *value = 0.0;
            }
        }
    }
    let ally = clamp_features(ally_raw, &OBS_V2.ally_bounds, &mut stats);

    // enemies: the v1 layout relative to the seat hero (AP-Q07).
    let mut seen: Vec<(f32, u32, [f32; ENEMY_FEATURES])> = Vec::new();
    let goblin_count = combat.enemies.len() as u32;
    for (index, goblin) in combat.enemies.iter().enumerate() {
        if goblin.alive() && !scene.is_offscreen(goblin.x, 0.0) {
            let row = goblin_row(goblin, seat_body.x, seat_foot, view_h, &mut stats);
            seen.push(((goblin.x - seat_body.x).abs(), index as u32, row));
        }
    }
    for (index, llama) in combat.llamas.iter().enumerate() {
        if llama.health > 0 && !scene.is_offscreen(llama.x, 0.0) {
            let row = llama_row(llama, seat_body.x, seat_foot, view_h, &mut stats);
            seen.push((
                (llama.x - seat_body.x).abs(),
                goblin_count + index as u32,
                row,
            ));
        }
    }
    seen.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    if seen.len() > MAX_ENEMIES {
        stats.overflow += (seen.len() - MAX_ENEMIES) as u32;
    }
    let mut enemies = [[0.0; ENEMY_FEATURES]; MAX_ENEMIES];
    let mut enemy_mask = [0.0; MAX_ENEMIES];
    for (slot, entry) in seen.iter().take(MAX_ENEMIES).enumerate() {
        enemies[slot] = entry.2;
        enemy_mask[slot] = 1.0;
    }
    stats.unknown_modes += combat
        .enemies
        .iter()
        .filter(|g| g.alive() && !known_goblin_mode(g.mode))
        .count() as u32;

    let obs = ObservationV2 {
        own,
        ally,
        ally_mask: [flag(ally_visible)],
        enemies,
        enemy_mask,
        hazards: [[0.0; HAZARD_FEATURES]; MAX_HAZARDS],
        hazard_mask: [0.0; MAX_HAZARDS],
        available: [0.0; AVAILABLE_LEN],
    };
    (obs, stats)
}
