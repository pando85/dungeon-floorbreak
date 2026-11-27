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
//! species without art) draws nothing. DEATH-D adds two authoritative finishes:
//! `goblin_decapitated` (the shore block's edge across the neck, cause
//! `ShoreBlockEdge`) plays the decapitation script, and `goblin_exploded` (a
//! point blank blast charge, cause `Explosion`) hides the goblin sprite and
//! flies its parts, gibs, drops and rags radially from the charge.

use std::sync::Arc;

use crate::blast::{BLAST_CENTRE_DY, BlastCharge};
use crate::cf02::Cf02Tuning;
use crate::combat::{CombatEvent, DefeatCause, ENEMY_HALF_HEIGHT, ForceTier};
use crate::combat_anim::goblin_variant;
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::death::{DeathProfiles, SlamPose, slam_pose_for};
use crate::feel::{GROUND_FRICTION, KNOCK_GRAVITY};
use crate::goblin::GoblinMode;
use crate::gore::{
    Cf02GoreBudget, DEATH_VARIANTS, DeathArt, ENGINEER_VARIANT, GorePreset, LLAMA_VARIANT, Mass,
    PartClass, PartThrow, PresentationRng, PresetSheets, Settle, SpinClass, TROG_FIRST_VARIANT,
    TROG_KINDS, Ttl, keeps_stream_item, lowest_below_centre, part_center_for_anchor,
    pose_anchor_world, rest_angle, scale_count,
};
use crate::llama::{LLAMA_HALF_HEIGHT, LlamaMode};
use crate::prototype::TICK_SECONDS;
use crate::shore::FOOTPRINT_WIDTH;

/// The cosmetic budget of the death presenter (DEATH-E1): every cap the
/// presenter honours, in one place. Capacities are fixed pool sizes (sprites
/// are pre-spawned, nothing is allocated at runtime); a full pool recycles by
/// [`ttl_rank`] and then oldest first, and what it recycles fades out over
/// `recycle_fade_ticks` instead of popping, so a budget never lowers what one
/// death draws, it only decides what is taken back first when many deaths
/// overlap. Set from the measurements in `docs/evidence/death-e/README.md`:
/// measured on a development machine, not on the Steam Deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeathBudget {
    /// Launched parts, gibs and blood drops that count against the budget
    /// (a settled piece stays until a pool recycles it, or, for a short-lived
    /// one, until its time to live ends).
    pub parts: usize,
    /// Floor decals: splats, pools (a two-cell pool is one entry), smears.
    pub floor_decals: usize,
    /// Back-wall spatters.
    pub wall_decals: usize,
    /// Extra sprite slots of each pool that hold the entries fading out after
    /// a recycle (they no longer count against the caps above). A burst of
    /// recycles larger than this takes back the oldest fading entry at once.
    pub part_fade_slots: usize,
    pub floor_decal_fade_slots: usize,
    pub wall_decal_fade_slots: usize,
    /// Death scripts alive at once (real defeats plus synthetic stress ones).
    pub deaths: usize,
    /// Splat decals one death may leave from landing drops (spec section 8).
    pub drop_splats_per_death: u8,
    /// Splat decals one death may leave from landing pieces (the head's
    /// bounces, gibs): kept apart so the drops cannot starve them.
    pub part_stains_per_death: u8,
    /// Ticks a `Short` piece (blood drops, gibs, the ear, rags) stays at full
    /// opacity after it lands, then fades over `short_fade_ticks`. `Long` and
    /// `Keep` pieces and every stain have no time to live: they leave only
    /// when a pool recycles them or their corpse retires.
    pub short_ttl_ticks: u64,
    pub short_fade_ticks: u64,
    /// Ticks an entry recycled by a full pool takes to fade out.
    pub recycle_fade_ticks: u64,
    /// Settled corpses kept visible; the oldest beyond this fade out over
    /// `corpse_fade_ticks` and retire (with their death script and parts).
    pub settled_corpses: usize,
    pub corpse_fade_ticks: u64,
}

/// The budget in force. Preset 4 is the default and the one measured (a preset
/// below 4 only draws less, so it fits in the same budget).
pub const DEATH_BUDGET: DeathBudget = DeathBudget {
    parts: 256,
    floor_decals: 192,
    wall_decals: 32,
    part_fade_slots: 64,
    floor_decal_fade_slots: 48,
    wall_decal_fade_slots: 8,
    deaths: 48,
    drop_splats_per_death: 8,
    part_stains_per_death: 6,
    short_ttl_ticks: 120,
    short_fade_ticks: 12,
    recycle_fade_ticks: 30,
    settled_corpses: 8,
    corpse_fade_ticks: 30,
};

/// Launched parts and drops that count against the budget.
pub const PART_POOL_CAPACITY: usize = DEATH_BUDGET.parts;
/// Floor decals: splats, pools (a two-cell pool is one entry).
pub const FLOOR_DECAL_POOL_CAPACITY: usize = DEATH_BUDGET.floor_decals;
/// Back-wall spatters.
pub const WALL_DECAL_POOL_CAPACITY: usize = DEATH_BUDGET.wall_decals;
/// Sprite slots of each pool: the budget plus the slots of the entries fading
/// out (the adapter pre-spawns one sprite per slot).
pub const PART_POOL_SLOTS: usize = DEATH_BUDGET.parts + DEATH_BUDGET.part_fade_slots;
pub const FLOOR_DECAL_POOL_SLOTS: usize =
    DEATH_BUDGET.floor_decals + DEATH_BUDGET.floor_decal_fade_slots;
pub const WALL_DECAL_POOL_SLOTS: usize =
    DEATH_BUDGET.wall_decals + DEATH_BUDGET.wall_decal_fade_slots;
/// Death scripts alive at once (real defeats plus synthetic stress ones).
pub const MAX_DEATHS: usize = DEATH_BUDGET.deaths;

/// Splat decals one death may leave from landing drops (spec section 8).
pub const MAX_DROP_SPLATS: u8 = DEATH_BUDGET.drop_splats_per_death;
/// Splat decals one death may leave from landing pieces.
pub const MAX_PART_STAINS: u8 = DEATH_BUDGET.part_stains_per_death;
/// Most molten decals one llama death leaves (LLAMA-02 spec section 8).
pub const MAX_MOLTEN_DECALS: u8 = 6;
/// Settled corpses kept visible (owner decision: a limit keeps the room from
/// getting too noisy and confused).
pub const MAX_SETTLED_CORPSES: usize = DEATH_BUDGET.settled_corpses;

/// Crush pool stages (1, 2, 3) after the landing, spec section 8.
const CRUSH_POOL_TICKS: (u64, u64, u64) = (4, 20, 60);
/// The same at preset 4: the stages come earlier.
const CRUSH_POOL_TICKS_FULL: (u64, u64, u64) = (2, 10, 24);

/// Ticks a `Short` piece stays after it lands, then fades. `Long` and `Keep`
/// pieces and stains have no time to live (orchestrator design decision
/// 2026-11-19 under the antihero tone, owner game-feel review pending: the hero
/// walks off through the mess he made).
pub const SHORT_TTL_TICKS: u64 = DEATH_BUDGET.short_ttl_ticks;
pub const SHORT_FADE_TICKS: u64 = DEATH_BUDGET.short_fade_ticks;

/// The recycling order of a full pool: a `Short` piece (drops, gibs, scraps)
/// goes before a `Long` one (limbs, splats), which goes before a `Keep` piece
/// (the head, the torso, the club, pools, smears, wall spatters); the oldest
/// first within a class. A fresh drop never evicts a corpse's pool while any
/// shorter-lived entry remains.
pub const fn ttl_rank(ttl: Ttl) -> u8 {
    match ttl {
        Ttl::Short => 0,
        Ttl::Long => 1,
        Ttl::Keep => 2,
    }
}

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
    /// DEATH-D: the QA charge and its blast burst (VFX), between the floor
    /// decals and the debris, under every actor.
    Blast,
    /// DEATH-D, preset 4: the bigger burst, over the debris and still under
    /// every actor.
    BlastFront,
    /// DEATH-D: the lit (fuse) charge: above the goblins, below Carl.
    ChargeLit,
}

/// z of the debris layer below `actor_z`.
pub const DEBRIS_BELOW_ACTORS: f32 = 0.4;
/// z of the floor decals below `actor_z`.
pub const FLOOR_DECAL_BELOW_ACTORS: f32 = 0.5;
/// z of the charge and the blast burst below `actor_z`: over the floor decals
/// (0.5) and the props, under the debris (0.4) and every actor.
pub const BLAST_BELOW_ACTORS: f32 = 0.45;
/// z of the preset 4 burst below `actor_z`: over the debris (0.4 plus the slot
/// bias, at most 0.032 below it), under every actor.
pub const BLAST_FRONT_BELOW_ACTORS: f32 = 0.30;
/// z of the lit charge above `actor_z`: over a standing goblin (+0.05), under
/// Carl (+0.1) and a swinging goblin (+0.15).
pub const CHARGE_LIT_ABOVE_ACTORS: f32 = 0.08;
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
        DrawLayer::Blast => actor_z - BLAST_BELOW_ACTORS + bias,
        DrawLayer::BlastFront => actor_z - BLAST_FRONT_BELOW_ACTORS + bias,
        DrawLayer::ChargeLit => actor_z + CHARGE_LIT_ABOVE_ACTORS + bias,
        DrawLayer::WallDecal => wall_decal_z + WALL_DECAL_ABOVE_LAYER + bias,
    }
}

// ----------------------------------------------------------------- pool -----

/// One pool slot: the spawn sequence, the item and, once a recycle or a retire
/// took it back, the tick it started to fade out.
#[derive(Debug, Clone)]
struct Entry<T> {
    seq: u64,
    item: T,
    fade_since: Option<u64>,
}

/// A fixed-size pool of `slots` sprites. Up to `limit` entries count against
/// the budget; once `limit` is reached, a new entry recycles the lowest-ranked,
/// oldest one, which keeps its slot and fades out (the extra `slots - limit`
/// slots hold those) and is freed when the fade ends. If every slot is taken,
/// the oldest fading entry goes at once. A pool entry is always one unit: a
/// two-cell decal is one entry, never half of one.
#[derive(Debug, Clone)]
pub struct Pool<T> {
    slots: Vec<Option<Entry<T>>>,
    limit: usize,
    next_seq: u64,
    evicted: u64,
}

impl<T> Pool<T> {
    /// A pool whose `capacity` slots all count against the budget (a recycle
    /// takes the entry back at once, there is no fading slot).
    pub fn new(capacity: usize) -> Self {
        Self::with_fade_slots(capacity, 0)
    }

    /// A pool with `limit` budgeted entries and `fade_slots` more slots for the
    /// entries fading out.
    pub fn with_fade_slots(limit: usize, fade_slots: usize) -> Self {
        Self {
            slots: (0..limit + fade_slots).map(|_| None).collect(),
            limit,
            next_seq: 0,
            evicted: 0,
        }
    }

    /// Sprite slots (the budget plus the fading slots).
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// The budget: entries that may be live (not fading) at once.
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Live entries: those that count against the budget (not fading out).
    pub fn live(&self) -> usize {
        self.slots
            .iter()
            .flatten()
            .filter(|e| e.fade_since.is_none())
            .count()
    }

    /// Every entry in a slot, fading ones included.
    pub fn occupied(&self) -> usize {
        self.slots.iter().flatten().count()
    }

    /// Entries recycled while the pool was full.
    pub fn evicted(&self) -> u64 {
        self.evicted
    }

    /// Put `item` in a free slot, or recycle the oldest. Returns the slot and
    /// the spawn sequence number (to recognise the entry later).
    pub fn spawn(&mut self, item: T, now: u64) -> (usize, u64) {
        self.spawn_ranked(item, now, |_| 0)
    }

    /// Put `item` in a slot. When the budget is full, the entry with the lowest
    /// `rank` (the oldest among equals) is recycled: it fades out from `now` in
    /// its slot, and the new entry takes a free slot (or, with none, the slot
    /// of the oldest fading entry).
    pub fn spawn_ranked(&mut self, item: T, now: u64, rank: impl Fn(&T) -> u8) -> (usize, u64) {
        let seq = self.next_seq;
        self.next_seq += 1;
        if self.live() >= self.limit {
            let victim = self
                .slots
                .iter()
                .enumerate()
                .filter_map(|(i, s)| {
                    s.as_ref()
                        .filter(|e| e.fade_since.is_none())
                        .map(|e| (i, e))
                })
                .min_by_key(|(_, e)| (rank(&e.item), e.seq))
                .map(|(i, _)| i);
            if let Some(victim) = victim {
                self.evicted += 1;
                if let Some(entry) = self.slots[victim].as_mut() {
                    entry.fade_since = Some(now);
                }
            }
        }
        let slot = match self.slots.iter().position(Option::is_none) {
            Some(free) => free,
            // Every slot is taken: the oldest fading entry (else the oldest of
            // all) is taken back at once.
            None => self
                .slots
                .iter()
                .enumerate()
                .min_by_key(|(_, s)| {
                    s.as_ref().map_or((true, u64::MAX, u64::MAX), |e| {
                        (e.fade_since.is_none(), e.fade_since.unwrap_or(0), e.seq)
                    })
                })
                .map(|(i, _)| i)
                .expect("a pool has at least one slot"),
        };
        self.slots[slot] = Some(Entry {
            seq,
            item,
            fade_since: None,
        });
        (slot, seq)
    }

    pub fn get(&self, slot: usize) -> Option<&T> {
        self.slots.get(slot)?.as_ref().map(|e| &e.item)
    }

    pub fn get_mut(&mut self, slot: usize) -> Option<&mut T> {
        self.slots.get_mut(slot)?.as_mut().map(|e| &mut e.item)
    }

    /// The live entry in `slot`, only if it is still the one spawned as `seq`
    /// and has not started to fade out.
    pub fn get_seq_mut(&mut self, slot: usize, seq: u64) -> Option<&mut T> {
        match self.slots.get_mut(slot)? {
            Some(e) if e.seq == seq && e.fade_since.is_none() => Some(&mut e.item),
            _ => None,
        }
    }

    /// The tick the entry in `slot` started to fade out, if it did.
    pub fn fade_since(&self, slot: usize) -> Option<u64> {
        self.slots.get(slot)?.as_ref()?.fade_since
    }

    /// Start fading the entry in `slot` out from `now` (a corpse retiring).
    pub fn start_fade(&mut self, slot: usize, now: u64) {
        if let Some(Some(e)) = self.slots.get_mut(slot)
            && e.fade_since.is_none()
        {
            e.fade_since = Some(now);
        }
    }

    /// Free every entry whose fade of `fade_ticks` has ended.
    pub fn expire_faded(&mut self, now: u64, fade_ticks: u64) {
        for slot in &mut self.slots {
            if slot
                .as_ref()
                .is_some_and(|e| e.fade_since.is_some_and(|f| now >= f + fade_ticks))
            {
                *slot = None;
            }
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
            .filter_map(|(i, s)| s.as_ref().map(|e| (i, &e.item)))
    }
}

// --------------------------------------------------------------- species ----

/// The species keys of the death profiles the presenter draws.
const GOBLIN_SPECIES: &str = "goblin";
/// ENEMY-GAMEPLAY-ENGINEER (ENG-G-Q21): the Goblin Engineer's species. ENG-02: each
/// recipe has its own finish (`engineer_slammed`, `engineer_exploded`,
/// `engineer_decapitated`, `engineer_crushed`, `engineer_knockback_down`), drawn
/// from his own death art ([`ENGINEER_VARIANT`]) when the run has it.
const ENGINEER_SPECIES: &str = "goblin_engineer";
const LLAMA_SPECIES: &str = "llama";

/// A species' death art and its composed sheets for the active preset: the Bad
/// Llama (LLAMA-02-DEATH) and the Goblin Engineer (ENG-02). Each is a separate set
/// with its own part indices; every piece, decal and pose carries the `variant`
/// that says whose set it belongs to ([`LLAMA_VARIANT`], [`ENGINEER_VARIANT`], or 0
/// and 1 for the goblins).
struct SpeciesDeath {
    art: Arc<DeathArt>,
    sheets: PresetSheets,
}

/// TROG-FX-01: the troglodyte species keys (`trog::Species::kind_name`), in the order of the
/// death sets (`TROG_FIRST_VARIANT + index`).
pub const TROG_SPECIES: [&str; TROG_KINDS] = ["trog_pygmy", "trog_basher", "trog_virtuoso"];

/// The death-set variant of a troglodyte species key, `None` for any other species.
pub fn trog_variant(species: &str) -> Option<usize> {
    TROG_SPECIES
        .iter()
        .position(|s| *s == species)
        .map(|i| TROG_FIRST_VARIANT + i)
}

/// The species sets of a presenter, after the two goblin variants.
#[derive(Default)]
struct SpeciesSets {
    llama: Option<SpeciesDeath>,
    engineer: Option<SpeciesDeath>,
    /// TROG-FX-01: the Pygmy, the Basher and the Virtuoso, at `TROG_FIRST_VARIANT + index`.
    trogs: [Option<SpeciesDeath>; TROG_KINDS],
}

impl SpeciesSets {
    /// The loaded set of a non-goblin `variant`.
    fn of(&self, variant: usize) -> Option<&SpeciesDeath> {
        match variant {
            LLAMA_VARIANT => self.llama.as_ref(),
            ENGINEER_VARIANT => self.engineer.as_ref(),
            v if (TROG_FIRST_VARIANT..TROG_FIRST_VARIANT + TROG_KINDS).contains(&v) => {
                self.trogs[v - TROG_FIRST_VARIANT].as_ref()
            }
            _ => None,
        }
    }
}

/// The art a `variant` is drawn from (field-wise, so a pool can stay borrowed).
/// The Engineer's variant draws the Engineer art when it is loaded; only a run
/// without it falls back to the goblin set (an Engineer target then gets its
/// goblin variant in `begin_death`, so this arm is never reached with one).
fn art_for<'a>(goblin: &'a DeathArt, sets: &'a SpeciesSets, variant: usize) -> &'a DeathArt {
    sets.of(variant).map_or(goblin, |s| &s.art)
}

/// The set a decal of a death of `variant` is drawn from: blood, stains and
/// pools are identical in both goblin sets and the Engineer's (variant 0); the
/// llama has its own.
const fn decal_set(variant: usize) -> usize {
    if variant == LLAMA_VARIANT { variant } else { 0 }
}

/// The goblin set a `variant` falls back to when its own art is not loaded (the
/// goblin variants are themselves; any other variant draws as goblin A).
const fn goblin_set(variant: usize) -> usize {
    if variant < DEATH_VARIANTS { variant } else { 0 }
}

/// The composed sheets a `variant` is drawn from.
fn sheets_for<'a>(
    goblins: &'a [PresetSheets; DEATH_VARIANTS],
    sets: &'a SpeciesSets,
    variant: usize,
) -> &'a PresetSheets {
    sets.of(variant)
        .map_or(&goblins[goblin_set(variant)], |s| &s.sheets)
}

/// `--death-lab-edge`: a block kill read as a block-edge cut by the same
/// player (every other event unchanged).
fn lab_edge_event(event: CombatEvent) -> CombatEvent {
    match event {
        CombatEvent::EnemyDefeated {
            tick,
            target,
            cause: DefeatCause::ShoreBlock { player },
            force,
            instigator,
        } => CombatEvent::EnemyDefeated {
            tick,
            target,
            cause: DefeatCause::ShoreBlockEdge { player },
            force,
            instigator,
        },
        other => other,
    }
}

// ------------------------------------------------------ blast frames ------

/// The `vfx_dynamite` atlas frames of the fizzling charge (`dynamite_fizzle`:
/// frames 4 and 5, 4 ticks each), which loop.
pub const CHARGE_FRAMES: [usize; 2] = [4, 5];
/// Ticks each fizzle frame is held.
pub const CHARGE_FRAME_TICKS: u64 = 4;

/// The fizzle frame of the charge `tick` ticks after the attempt began (frame
/// 4 on ticks 0 to 3 of every 8, frame 5 on 4 to 7). Pure.
pub const fn charge_frame(tick: u64) -> usize {
    CHARGE_FRAMES[((tick / CHARGE_FRAME_TICKS) % 2) as usize]
}

/// The still stick drawn while a `CarlPast` charge is not armed yet
/// (`vfx_dynamite` frame 0, the unlit stick).
pub const CHARGE_UNLIT_FRAME: usize = 0;

/// The frame of a charge: the still stick before its fuse is lit, then the
/// fizzle loop from the tick it was lit (`ticks_lit` = ticks since then, 0 on
/// the first). Pure.
pub const fn charge_draw_frame(lit: bool, ticks_lit: u64) -> usize {
    if lit {
        charge_frame(ticks_lit)
    } else {
        CHARGE_UNLIT_FRAME
    }
}

/// The VFX-01 `burst` tag: (atlas frame, ticks held), from the manifest tick
/// table (a test compares it with `assets/manifests/vfx-01.json`).
pub const BURST_FRAMES: [(usize, u64); 9] = [
    (3, 2),
    (4, 3),
    (5, 4),
    (6, 4),
    (7, 5),
    (8, 6),
    (9, 8),
    (10, 6),
    (11, 4),
];

/// Ticks the whole burst lasts.
pub const fn burst_ticks() -> u64 {
    let mut total = 0;
    let mut i = 0;
    while i < BURST_FRAMES.len() {
        total += BURST_FRAMES[i].1;
        i += 1;
    }
    total
}

/// The `vfx-01` frame of a blast that went off on `blast_tick`, at `tick`:
/// `None` before the blast and after the burst's last tick. Pure.
pub fn blast_vfx_frame(tick: u64, blast_tick: u64) -> Option<usize> {
    let mut age = tick.checked_sub(blast_tick)?;
    for (frame, ticks) in BURST_FRAMES {
        if age < ticks {
            return Some(frame);
        }
        age -= ticks;
    }
    None
}

// ------------------------------------------------- explosion tables --------

/// DEATH-D: what one explosive breakup spawns at a preset, as explicit
/// constants (spec section 4.2), not derived from the preset's quarters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExplosionTable {
    /// Body part ids, each launched from its `frame0_origin`.
    pub parts: &'static [&'static str],
    /// Gib ids and how many of each.
    pub gibs: &'static [(&'static str, u32)],
    /// Rag count (class clean: the same at every preset), cycled over
    /// `rag_ids`.
    pub rags: u32,
    /// `drop_blood` count.
    pub drops: u32,
    /// Splat decals the landing drops may leave, and the landing pieces.
    pub drop_splat_cap: u8,
    pub part_stain_cap: u8,
    /// Pools grown from the first rest of `torso_upper` and `torso_lower`.
    pub pools: u8,
    /// Preset 4 punch-up (presentation only): the decals of the pools and the
    /// extra wall spatters are drawn at this scale (1.0 = the art's size), the
    /// burst sprite is scaled by `burst_scale` and drawn over the debris, and
    /// shows a bright flash (the existing impact tint) for `flash_ticks`.
    pub pool_scale: f32,
    pub extra_wall_spatters: u8,
    pub burst_scale: f32,
    pub flash_ticks: u64,
    /// The part whose `frame0_origin` emits the gibs, rags and blood drops
    /// (the goblin's `torso_upper`, the llama's `torso_front`).
    pub torso: &'static str,
    /// The rag ids, cycled for `rags`: the goblin's `gib_rag`, the llama's
    /// two wool tufts (LLAMA-02 section 5, the clean class of every preset).
    pub rag_ids: &'static [&'static str],
    /// LLAMA-02: lava drops out of the burst gland (`drop_molten`, shown at
    /// every preset, section 5); 0 for the goblin.
    pub molten_drops: u32,
    /// The part whose `frame0_origin` places a listed part that has none, and
    /// emits the lava drops: the llama's `neck` (the throat, where the gland
    /// sits); the goblin's `torso_upper` (every goblin part has an origin).
    pub throat: &'static str,
}

/// Indexed by `preset.level() - 1`.
pub static EXPLOSION_TABLES: [ExplosionTable; 4] = [
    // 1: the sprite stays (the sim's own flight); only the two rags fly.
    ExplosionTable {
        parts: &[],
        gibs: &[],
        rags: 2,
        drops: 0,
        drop_splat_cap: 0,
        part_stain_cap: 0,
        pools: 0,
        pool_scale: 1.0,
        extra_wall_spatters: 0,
        burst_scale: 1.0,
        flash_ticks: 0,
        torso: "torso_upper",
        rag_ids: &["gib_rag"],
        molten_drops: 0,
        throat: "torso_upper",
    },
    // 2: head, torso_upper and the club; gore muted.
    ExplosionTable {
        parts: &["head", "torso_upper", "club"],
        gibs: &[],
        rags: 2,
        drops: 8,
        drop_splat_cap: 4,
        part_stain_cap: 4,
        pools: 0,
        pool_scale: 1.0,
        extra_wall_spatters: 0,
        burst_scale: 1.0,
        flash_ticks: 0,
        torso: "torso_upper",
        rag_ids: &["gib_rag"],
        molten_drops: 0,
        throat: "torso_upper",
    },
    // 3: five parts, four gibs, one pool to stage 2.
    ExplosionTable {
        parts: &["head", "torso_upper", "arm_near", "leg_far", "club"],
        gibs: &[("gib_flesh_a", 2), ("gib_flesh_b", 2)],
        rags: 2,
        drops: 16,
        drop_splat_cap: 8,
        part_stain_cap: 6,
        pools: 1,
        pool_scale: 1.0,
        extra_wall_spatters: 0,
        burst_scale: 1.0,
        flash_ticks: 0,
        torso: "torso_upper",
        rag_ids: &["gib_rag"],
        molten_drops: 0,
        throat: "torso_upper",
    },
    // 4: ten parts (the club splits), ten gibs, two pools to stage 3.
    ExplosionTable {
        parts: &[
            "head",
            "torso_upper",
            "torso_lower",
            "arm_near",
            "arm_far",
            "leg_near",
            "leg_far",
            "ear",
            "club_stick",
            "pineapple",
        ],
        gibs: &[("gib_flesh_a", 4), ("gib_flesh_b", 4), ("gib_bone", 2)],
        rags: 2,
        drops: 32,
        drop_splat_cap: 14,
        part_stain_cap: 10,
        pools: 2,
        // The owner wants more intensity at preset 4: thicker pools, two
        // more wall spatters at the larger size, a 2x burst over the debris
        // and a 3 tick flash at detonation. Presentation only.
        pool_scale: 2.0,
        extra_wall_spatters: 2,
        burst_scale: 2.0,
        flash_ticks: 3,
        torso: "torso_upper",
        rag_ids: &["gib_rag"],
        molten_drops: 0,
        throat: "torso_upper",
    },
];

/// ENG-02: the Engineer's explosive breakup (spec section 9): the goblin tables with
/// his own part lists, every other field equal to the goblin row. Preset 1 keeps
/// the sprite, with its pot; presets 2 to 4 throw the bare head, the wrench and the
/// pot (never the potted `head`, a `club` or a `pineapple`).
pub static ENGINEER_EXPLOSION_TABLES: [ExplosionTable; 4] = [
    ExplosionTable {
        parts: &[],
        ..EXPLOSION_TABLES[0]
    },
    ExplosionTable {
        parts: &["head_bare", "torso_upper", "wrench", "pot"],
        ..EXPLOSION_TABLES[1]
    },
    ExplosionTable {
        parts: &[
            "head_bare",
            "torso_upper",
            "arm_near",
            "leg_far",
            "wrench",
            "pot",
        ],
        ..EXPLOSION_TABLES[2]
    },
    ExplosionTable {
        parts: &[
            "head_bare",
            "torso_upper",
            "torso_lower",
            "arm_near",
            "arm_far",
            "leg_near",
            "leg_far",
            "ear",
            "wrench",
            "pot",
        ],
        ..EXPLOSION_TABLES[3]
    },
];

/// The table of `preset`.
pub fn explosion_table(preset: GorePreset) -> &'static ExplosionTable {
    &EXPLOSION_TABLES[preset.level() as usize - 1]
}

/// LLAMA-DEATH-WIRE: the Bad Llama's explosive breakup (LLAMA-02 spec sections
/// 5 and 8, `death_poses.spawner_hints.explosive` of `llama_death_h64.json`),
/// indexed by `preset.level() - 1`. Every list is the spec's; the counts
/// follow the section 5 scales (gibs and blood 1.0 / 0.5 / 0.25 / 0, lava
/// 1.0 / 1.0 / 0.5 / 0.5). Adopted defaults (LDW-AD2, LDW-AD3): two wool tufts
/// stand in for the goblin's rags at every preset (the section 5 preset 1 list
/// names them) and one knocked-out `tooth` flies at every preset (the section 5
/// preset 1 list; the crush and the slam throw one too). The preset 4
/// punch-up (pool and spatter scale, burst, flash) is the goblin's.
pub static LLAMA_EXPLOSION_TABLES: [ExplosionTable; 4] = [
    // 1: the LLAMA-01 `down` stays (the sim's own slide); tufts, a tooth and
    // the lava fly.
    ExplosionTable {
        parts: &[],
        gibs: &[("tooth", 1)],
        rags: 2,
        drops: 0,
        drop_splat_cap: 0,
        part_stain_cap: 0,
        pools: 0,
        pool_scale: 1.0,
        extra_wall_spatters: 0,
        burst_scale: 1.0,
        flash_ticks: 0,
        torso: "torso_front",
        rag_ids: &["wool_tuft_a", "wool_tuft_b"],
        molten_drops: 4,
        throat: "neck",
    },
    // 2: head, front torso and the gland; blood muted, no gibs.
    ExplosionTable {
        parts: &["head", "torso_front", "gland"],
        gibs: &[("tooth", 1)],
        rags: 2,
        drops: 7,
        drop_splat_cap: 4,
        part_stain_cap: 4,
        pools: 0,
        pool_scale: 1.0,
        extra_wall_spatters: 0,
        burst_scale: 1.0,
        flash_ticks: 0,
        torso: "torso_front",
        rag_ids: &["wool_tuft_a", "wool_tuft_b"],
        molten_drops: 4,
        throat: "neck",
    },
    // 3: six parts (no neck, ear, jaw or far legs), four gibs, one pool.
    ExplosionTable {
        parts: &[
            "head",
            "torso_front",
            "torso_rump",
            "leg_fore",
            "leg_hind",
            "gland",
        ],
        gibs: &[("gib_flesh_a", 2), ("gib_flesh_b", 2), ("tooth", 1)],
        rags: 2,
        drops: 14,
        drop_splat_cap: 8,
        part_stain_cap: 6,
        pools: 1,
        pool_scale: 1.0,
        extra_wall_spatters: 0,
        burst_scale: 1.0,
        flash_ticks: 0,
        torso: "torso_front",
        rag_ids: &["wool_tuft_a", "wool_tuft_b"],
        molten_drops: 8,
        throat: "neck",
    },
    // 4: every body part, ten gibs and two ribs, two pools.
    ExplosionTable {
        parts: &[
            "head",
            "neck",
            "torso_front",
            "torso_rump",
            "leg_fore",
            "leg_hind",
            "leg_fore_far",
            "leg_hind_far",
            "ear",
            "jaw",
            "gland",
        ],
        gibs: &[
            ("gib_flesh_a", 4),
            ("gib_flesh_b", 4),
            ("gib_bone", 2),
            ("gib_rib", 2),
            ("tooth", 1),
        ],
        rags: 2,
        drops: 28,
        drop_splat_cap: 14,
        part_stain_cap: 10,
        pools: 2,
        pool_scale: 2.0,
        extra_wall_spatters: 2,
        burst_scale: 2.0,
        flash_ticks: 3,
        torso: "torso_front",
        rag_ids: &["wool_tuft_a", "wool_tuft_b"],
        molten_drops: 8,
        throat: "neck",
    },
];

/// TROG-FX-01: the troglodytes' explosive breakup, one row set per kind (spec section 4), the goblin
/// tables with the kind's own part list and torso: preset 1 keeps the sprite (only the two rags fly),
/// preset 2 throws the head and the upper torso (and the Virtuoso's bell), preset 3 adds an arm and a
/// leg, preset 4 every cut. A kind lists only parts its sheet has (a part a sheet lacks is skipped:
/// the Pygmy has no far arm or far leg, the Basher and the Pygmy carry no bell).
pub static TROG_EXPLOSION_TABLES: [[ExplosionTable; 4]; TROG_KINDS] = [
    [
        ExplosionTable {
            parts: &[],
            ..EXPLOSION_TABLES[0]
        },
        ExplosionTable {
            parts: &["head", "torso_upper"],
            ..EXPLOSION_TABLES[1]
        },
        ExplosionTable {
            parts: &["head", "torso_upper", "arm_near", "leg_near"],
            ..EXPLOSION_TABLES[2]
        },
        ExplosionTable {
            parts: &["head", "torso_upper", "torso_lower", "arm_near", "leg_near"],
            ..EXPLOSION_TABLES[3]
        },
    ],
    [
        ExplosionTable {
            parts: &[],
            ..EXPLOSION_TABLES[0]
        },
        ExplosionTable {
            parts: &["head", "torso_upper"],
            ..EXPLOSION_TABLES[1]
        },
        ExplosionTable {
            parts: &["head", "torso_upper", "arm_near", "leg_near"],
            ..EXPLOSION_TABLES[2]
        },
        ExplosionTable {
            parts: &[
                "head",
                "torso_upper",
                "torso_lower",
                "arm_near",
                "arm_far",
                "leg_near",
                "leg_far",
            ],
            ..EXPLOSION_TABLES[3]
        },
    ],
    [
        ExplosionTable {
            parts: &[],
            ..EXPLOSION_TABLES[0]
        },
        ExplosionTable {
            parts: &["head", "torso_upper", "bell"],
            ..EXPLOSION_TABLES[1]
        },
        ExplosionTable {
            parts: &["head", "torso_upper", "arm_near", "leg_near", "bell"],
            ..EXPLOSION_TABLES[2]
        },
        ExplosionTable {
            parts: &[
                "head",
                "torso_upper",
                "torso_lower",
                "arm_near",
                "leg_near",
                "leg_far",
                "bell",
            ],
            ..EXPLOSION_TABLES[3]
        },
    ],
];

/// The explosion table of a death set `variant` (goblin A or B, the llama, the
/// Engineer or a troglodyte) at `preset`.
pub fn explosion_table_of(variant: usize, preset: GorePreset) -> &'static ExplosionTable {
    let at = preset.level() as usize - 1;
    match variant {
        LLAMA_VARIANT => &LLAMA_EXPLOSION_TABLES[at],
        ENGINEER_VARIANT => &ENGINEER_EXPLOSION_TABLES[at],
        v if (TROG_FIRST_VARIANT..TROG_FIRST_VARIANT + TROG_KINDS).contains(&v) => {
            &TROG_EXPLOSION_TABLES[v - TROG_FIRST_VARIANT][at]
        }
        _ => explosion_table(preset),
    }
}

/// Pool stages of a piece that rests at tick R: stage 1 on R, 2 on R + 16,
/// 3 on R + 40 (the 38 px pair, preset 4 only: the manifest filter).
const EXPLOSION_POOL_TICKS: [(u64, &str); 3] = [
    (0, "decal_pool_1"),
    (16, "decal_pool_2"),
    (40, "decal_pool_3"),
];

/// Q43: drops in one transient spray of a routine blunt kill (preset 4).
const SPRAY_DROPS: u32 = 6;

/// Squeeze drops of a decapitation by the block edge (preset 4).
const EDGE_SQUEEZE_DROPS: u32 = 8;

// ---------------------------------------------------------------- finish ----

/// What a defeated goblin plays, from the DEATH-B finish id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    /// The falling block: `crush_contact`, `crushed_remains`, the squeeze.
    Crushed,
    /// The GOB-01 knockback and corpse frames, plus blood.
    Knockback,
    /// DEATH-D (the block's edge across the neck): `decap_contact`,
    /// `headless_collapse`, the head.
    Decapitated,
    /// LLAMA-02 slam (owner decision 2026-11-19: a lethal heavy hit):
    /// `slam_contact` for 6 ticks, then `broken_heap` held.
    Slammed,
    /// DEATH-D (a point blank blast): the sprite is hidden (presets 2 to 4)
    /// and the pieces fly from the charge.
    Exploded,
    /// CF02-D (Q21, cf02 overlay): the heavy blunt goblin finish, in one of
    /// three poses picked from the defeat's events. The pose frames are wired by
    /// CF02-I; until then the GOB-01 knockback sprite stays and only the gore
    /// baseline (Q43) differs.
    GoblinSlammed(SlamPose),
    /// A finish without art: nothing is drawn.
    NoArt,
}

impl Finish {
    pub fn from_id(id: &str) -> Self {
        match id {
            "goblin_crushed" | "brute_crushed" => Self::Crushed,
            // ENG-G-Q21: the routine knockback keeps the ENG-01 `down` frames (blood is
            // added, the sprite the scene draws stays), like `llama_down`.
            "goblin_knockback_down" | "brute_ko_timber" | "engineer_knockback_down" => {
                Self::Knockback
            }
            "goblin_decapitated" => Self::Decapitated,
            // The llama: the crush plays the same pose sequence, the slam its
            // own, and `llama_down` keeps the LLAMA-01 `down` frames like the
            // goblin's knockback (blood is added, the sprite stays).
            "llama_crushed" => Self::Crushed,
            "llama_slammed" => Self::Slammed,
            "llama_down" => Self::Knockback,
            // LLAMA-DEATH-WIRE: the LLAMA-02 decapitation pair (`decap_contact`,
            // `headless_collapse`, the head) and the explosive breakup of its
            // parts sheet play through the goblin scripts with the llama's own
            // numbers (`script_decap`, `LLAMA_EXPLOSION_TABLES`).
            "llama_decapitated" => Self::Decapitated,
            "llama_exploded" => Self::Exploded,
            "goblin_exploded" => Self::Exploded,
            // ENG-02: the Engineer's own finishes play the goblin scripts with the
            // Engineer set (the flat pot under the block, the potted head, the pot
            // and the wrench as parts).
            "engineer_crushed" => Self::Crushed,
            "engineer_decapitated" => Self::Decapitated,
            "engineer_exploded" => Self::Exploded,
            // The pose is picked from the defeat's events (`slam_pose_for`).
            "goblin_slammed" | "engineer_slammed" => Self::GoblinSlammed(SlamPose::Flung),
            // FA-Q23 (HRD sheet section 10): every recipe of the Hoarder (`pile_boss`) maps to her
            // HOARD-01 `down` frames 13 to 15, which the room's own presenter draws
            // (`hoarder_art`). The generic presenter adds nothing: no gibs, no dismemberment, no
            // blood (a ruined person, not a target for spectacle), and she is not CINE-eligible.
            "pile_boss_down" => Self::NoArt,
            // TROG-FX-01: a troglodyte keeps its approved `down` frames and gains blood (the
            // knockback script); a point blank blast throws its own cut parts (the explosion
            // script with the kind's set and table).
            "trog_pygmy_down" | "trog_basher_down" | "trog_virtuoso_down" => Self::Knockback,
            "trog_pygmy_exploded" | "trog_basher_exploded" | "trog_virtuoso_exploded" => {
                Self::Exploded
            }
            _ => Self::NoArt,
        }
    }

    /// ENG-02 evidence lab (`--death-lab-engineer`): the finish an Engineer plays for a lab `name`
    /// (`crushed`, `decapitated`, `exploded`, `slammed_stomped`, `slammed_wall`, `slammed_flung`).
    pub fn lab_engineer(name: &str) -> Option<Self> {
        Some(match name {
            "crushed" => Self::Crushed,
            "decapitated" => Self::Decapitated,
            "exploded" => Self::Exploded,
            "slammed_stomped" => Self::GoblinSlammed(SlamPose::Stomped),
            "slammed_wall" => Self::GoblinSlammed(SlamPose::WallCrumple),
            "slammed_flung" => Self::GoblinSlammed(SlamPose::Flung),
            _ => return None,
        })
    }

    /// TROG-FX-01 evidence lab (`--death-lab-trog`): the finish a troglodyte plays for a lab `name`
    /// (`exploded` or `down`).
    pub fn lab_trog(name: &str) -> Option<Self> {
        Some(match name {
            "exploded" => Self::Exploded,
            "down" => Self::Knockback,
            _ => return None,
        })
    }

    /// Whether the finish replaces the goblin sprite with a death pose.
    pub const fn has_pose(self) -> bool {
        matches!(self, Self::Crushed | Self::Decapitated | Self::Slammed)
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
            // S: pose 6 (6 ticks), S+6: pose 7, hold.
            Self::Slammed => Some(if age < 6 { 6 } else { 7 }),
            Self::Knockback | Self::Exploded | Self::GoblinSlammed(_) | Self::NoArt => None,
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

/// A llama as the presenter sees it (read from the simulation, never written).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LlamaView {
    pub x: f32,
    /// y-up world height of its feet on the floor.
    pub ground_y: f32,
    pub facing: i8,
    pub mode: LlamaMode,
}

/// The impact dust of a llama slam (LLAMA-02 spec section 8, tick S): where it
/// is on the floor and how many ticks it has played.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DustView {
    pub x: f32,
    pub ground_y: f32,
    pub age: u64,
}

/// CF02-D (Q21): a slammed goblin finish as the presenter sees it, for the
/// pose wiring of CF02-I: which of the three poses, where and how faded. The
/// impact-then-rest clock is not here (F01-FU-B10, CF02-FU-I FU7): the one clock
/// is the acting selector's (`combat_anim::cf02::SlamArtInput::dead_age`, the
/// `acting_slammed=` field of `COMBAT_ANIM_STATE`), which the drawn frame reads
/// and which starts after a Q72 taunt hold. The presenter's own copy (a `rest`
/// flag counted from the defeat) was read by nothing but its test and disagreed
/// with the drawn frame after a taunt hold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlamView {
    pub target: u32,
    pub pose: SlamPose,
    pub foot_x: f32,
    pub ground_y: f32,
    pub flip: bool,
    pub variant: usize,
    pub alpha: f32,
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
    /// The variant whose poses sheet is drawn (0 = goblin A, 1 = B,
    /// [`LLAMA_VARIANT`] = the Bad Llama).
    pub variant: usize,
    /// 1 until the corpse starts to retire, then fading to 0.
    pub alpha: f32,
    /// `hazard_draw: "over"` on this frame (LLAMA-02 gate ruling): the world
    /// point of the pose's mirrored `hit_origin`, where the falling block's
    /// bottom-centre anchor is drawn, directly above this enemy and below every
    /// player actor. `None`: the goblin order (the block behind).
    pub hazard_anchor: Option<(f32, f32)>,
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
    /// CF02-D (Q43): the presenter tick this piece vanishes at (a transient
    /// spray). It leaves no decal or stain and is gone after its lifetime.
    transient_until: Option<u64>,
    /// The goblin variant whose parts sheet is drawn (0 = A, 1 = B): the head,
    /// torso and ear of B carry the accessory layer. Blood, gibs and decals
    /// are identical in both.
    pub variant: usize,
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
    /// Draw scale about the centre (1.0 except the preset 4 explosion's
    /// pools and extra wall spatters); the position already accounts for it.
    pub scale: f32,
    born: u64,
    /// Whose set the decal belongs to (see [`PoseView::variant`]).
    pub variant: usize,
}

/// What a finished tick of the presenter draws for one decal or part.
fn fade_alpha(ttl: Ttl, since: u64, clock: u64) -> f32 {
    let (keep, fade) = match ttl {
        Ttl::Keep | Ttl::Long => return 1.0,
        Ttl::Short => (SHORT_TTL_TICKS, SHORT_FADE_TICKS),
    };
    let age = clock.saturating_sub(since);
    if age <= keep {
        1.0
    } else {
        1.0 - ((age - keep) as f32 / fade as f32).min(1.0)
    }
}

/// Opacity of a pool entry that started fading out at `since` (a recycle or a
/// retiring corpse): 1 while it has not, then falling to 0 over
/// `recycle_fade_ticks`.
fn recycle_alpha(since: Option<u64>, clock: u64) -> f32 {
    match since {
        Some(since) => {
            let fade = DEATH_BUDGET.recycle_fade_ticks as f32;
            (1.0 - clock.saturating_sub(since) as f32 / fade).clamp(0.0, 1.0)
        }
        None => 1.0,
    }
}

fn expired(ttl: Ttl, since: u64, clock: u64) -> bool {
    fade_alpha(ttl, since, clock) <= 0.0
}

#[derive(Debug, Clone)]
struct Death {
    id: u32,
    /// Real target id (goblins first, then llamas), or [`SYNTHETIC_TARGET`].
    target: u32,
    /// The goblin variant (A or B) whose art plays.
    variant: usize,
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
    /// Molten decals this death left (LLAMA-02: at most 6).
    molten_decals: u8,
    /// Splat decals the landing drops and pieces may leave (the explosion
    /// uses the caps of its table).
    drop_cap: u8,
    stain_cap: u8,
    drops_emitted: u32,
    /// The pool decals that grow by frame swaps: (slot, spawn seq). The other
    /// finishes use index 0 only; the explosion grows index 0 under
    /// `torso_upper` and index 1 under `torso_lower`.
    pools: [Option<(usize, u64)>; 2],
    /// The first tick each tracked piece (`torso_upper`, `torso_lower`)
    /// entered `Resting`, and where; recorded once, never again.
    rest_ticks: [Option<u64>; 2],
    rest_x: [f32; 2],
    /// Scale of this death's pool decals (1.0 unless the preset 4 explosion).
    pool_scale: f32,
    /// The impact point `I` of an explosion (the charge centre), or of a
    /// synthetic one.
    impact: Option<(f32, f32)>,
    /// The floor x of the block edge that cut the neck (a `ShoreBlockEdge`).
    edge_splat_x: Option<f32>,
    landed: Option<u64>,
    /// ENG-02: the slammed Engineer's pot and wrench have been launched (exactly once per death).
    slam_launched: bool,
    done_after: u64,
    /// The tick the corpse started to fade out because too many settled
    /// corpses were visible (`MAX_SETTLED_CORPSES`).
    retire_since: Option<u64>,
}

impl Death {
    /// A death with no pieces yet; `Death::new` callers set what is specific
    /// to their finish (caps, impact point, edge).
    #[allow(clippy::too_many_arguments)]
    fn new(
        id: u32,
        target: u32,
        variant: usize,
        finish: Finish,
        force: ForceTier,
        start: u64,
        foot: (f32, f32),
        flip: bool,
        away: f32,
        rng: PresentationRng,
    ) -> Self {
        Self {
            id,
            target,
            variant,
            finish,
            force,
            start,
            foot_x: foot.0,
            ground_y: foot.1,
            flip,
            away,
            rng,
            drop_splats: 0,
            part_stains: 0,
            molten_decals: 0,
            drop_cap: MAX_DROP_SPLATS,
            stain_cap: MAX_PART_STAINS,
            drops_emitted: 0,
            pools: [None, None],
            rest_ticks: [None, None],
            rest_x: [0.0, 0.0],
            pool_scale: 1.0,
            impact: None,
            edge_splat_x: None,
            landed: None,
            slam_launched: false,
            done_after: 0,
            retire_since: None,
        }
    }
}

/// Everything one presenter step needs from the simulation.
#[derive(Debug, Clone, Copy)]
pub struct Observation<'a> {
    pub goblins: &'a [GoblinView],
    /// The llamas; their target ids follow the goblins'.
    pub llamas: &'a [LlamaView],
    pub hero_x: f32,
    /// x of the middle of the shore's hazard footprint, if the fixture has one.
    pub block_x: Option<f32>,
    /// Whether a back wall is within reach of the spatter (art spec section 8:
    /// "when a wall face is within 48 px"). The corridor has its back wall
    /// everywhere, so the fixture scene always passes true; a route with open
    /// background passes false and gets no wall spatter.
    pub back_wall: bool,
    /// DEATH-D: the fixture's QA blast charges (the impact point of an
    /// explosion is read from the one that defeated the goblin).
    pub charges: &'a [BlastCharge],
    /// BOIL-E: where each env hazard's blow comes from: `(prop, class, x)` with
    /// x the centre of the hazard's volume (a falling weight's footprint, the
    /// steam's area, the drum's ring). A body the env defeated is flung away from
    /// this point, not from the hero. Empty outside a room with props.
    pub env_origins: &'a [(u16, crate::env_hazard::HazardClass, f32)],
}

/// Counters for evidence and tests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PresenterStats {
    pub parts_spawned: u64,
    pub floor_decals_spawned: u64,
    pub wall_decals_spawned: u64,
    pub peak_parts: usize,
    pub peak_floor_decals: usize,
    pub peak_wall_decals: usize,
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
    /// A piece came to rest (the explosion tracks its two torso pieces).
    Rested {
        death: u32,
        meta: usize,
        x: f32,
    },
}

const RAD_TO_DEG: f32 = 57.295_78;

/// The cosmetic presenter: one per run, reset on a restart.
pub struct DeathPresenter {
    art: Arc<DeathArt>,
    sheets: [PresetSheets; DEATH_VARIANTS],
    /// The Bad Llama's and the Engineer's art and sheets, when the run has them.
    sets: SpeciesSets,
    profiles: &'static DeathProfiles,
    /// TROG-FX-01: the troglodyte death art is loaded (the profiles then carry the three troglodyte rows).
    trog_profiles: bool,
    /// The species key of the goblin targets (tests swap it to check a species
    /// without art); the llama's is [`LLAMA_SPECIES`].
    species: &'static str,
    /// ENEMY-GAMEPLAY-ENGINEER: the target ids of the Engineers of the run (their
    /// species is [`ENGINEER_SPECIES`]); empty for every earlier run.
    engineers: Vec<u32>,
    preset: GorePreset,
    /// The finish id chosen for the boss' defeat (HUD-DEBT-03).
    boss_finish: Option<String>,
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
    /// Real goblins whose corpse has retired: drawn no more this attempt.
    retired: Vec<u32>,
    /// DEATH-D: (target, start tick) of every explosion at presets 2 to 4:
    /// the goblin sprite is hidden from `start` for the rest of the attempt.
    exploded: Vec<(u32, u64)>,
    /// CF02-D (Q36): the cf02 tuning this presenter reads, if any (`with_cf02`).
    cf02: Option<&'static Cf02Tuning>,
    /// CF02-D (Q22, Q43): the cf02 gore budget (Q36 switch on).
    gore_budget: Option<Cf02GoreBudget>,
    /// CF02-D (Q36): the cf02 death overlay is on (slammed finishes exist).
    slam_overlay: bool,
    /// While set, every launched piece is a transient (see `Part`).
    transient_until: Option<u64>,
    /// LLAMA-DEATH-WIRE evidence lab (`--death-lab-edge`): the presenter plays
    /// every block kill as a block-edge cut. Presentation only: the
    /// simulation, its log and its hash never see it.
    lab_edge: bool,
    /// ENG-02 evidence lab (`--death-lab-engineer`): every Engineer defeat plays this finish, whatever recipe
    /// the defeat would pick (the Floor 1 pilots only ever kill him with a jab). Presentation only.
    lab_engineer: Option<Finish>,
    /// TROG-FX-01 evidence lab (`--death-lab-trog`): every troglodyte defeat plays this finish, whatever its cause.
    lab_trog: Option<Finish>,
}

impl DeathPresenter {
    /// A presenter with one art set for every goblin (variant B is drawn with
    /// variant A's sheets). The game uses [`Self::with_variants`].
    pub fn new(art: Arc<DeathArt>, preset: GorePreset) -> Result<Self, String> {
        let sheets = art.compose(preset)?;
        Self::build(art, [sheets.clone(), sheets], preset)
    }

    /// A presenter with the death art of both goblin variants. `b` must share
    /// `a`'s contract ([`DeathArt::same_contract`]): the presenter reads the
    /// metadata of `a` for both and only swaps the sheets.
    pub fn with_variants(
        a: Arc<DeathArt>,
        b: &DeathArt,
        preset: GorePreset,
    ) -> Result<Self, String> {
        a.same_contract(b)
            .map_err(|e| format!("goblin variant B death art: {e}"))?;
        let sheets = [a.compose(preset)?, b.compose(preset)?];
        Self::build(a, sheets, preset)
    }

    fn build(
        art: Arc<DeathArt>,
        sheets: [PresetSheets; DEATH_VARIANTS],
        preset: GorePreset,
    ) -> Result<Self, String> {
        Ok(Self {
            art,
            sheets,
            sets: SpeciesSets::default(),
            profiles: DeathProfiles::builtin(),
            trog_profiles: false,
            species: GOBLIN_SPECIES,
            engineers: Vec::new(),
            preset,
            boss_finish: None,
            clock: 0,
            attempt: 0,
            seen_events: 0,
            next_death_id: 0,
            deaths: Vec::new(),
            parts: Pool::with_fade_slots(DEATH_BUDGET.parts, DEATH_BUDGET.part_fade_slots),
            floor_decals: Pool::with_fade_slots(
                DEATH_BUDGET.floor_decals,
                DEATH_BUDGET.floor_decal_fade_slots,
            ),
            wall_decals: Pool::with_fade_slots(
                DEATH_BUDGET.wall_decals,
                DEATH_BUDGET.wall_decal_fade_slots,
            ),
            last_wall: None,
            back_wall: true,
            stress_rng: PresentationRng::new(0xD1E5_7AE5),
            stats: PresenterStats::default(),
            retired: Vec::new(),
            exploded: Vec::new(),
            cf02: None,
            gore_budget: None,
            slam_overlay: false,
            transient_until: None,
            lab_edge: false,
            lab_engineer: None,
            lab_trog: None,
        })
    }

    /// LLAMA-DEATH-WIRE evidence lab: read every `ShoreBlock` defeat as a
    /// `ShoreBlockEdge` cut by the same player, so the decapitation of a
    /// species the simulation never shears (the llama) can be looked at in
    /// the real render loop. Presentation only: the events the simulation
    /// logged and hashed are untouched; only this presenter's copy differs.
    /// Remove it when a simulation cause can behead a llama.
    #[must_use]
    pub fn with_lab_edge(mut self) -> Self {
        self.lab_edge = true;
        self
    }

    /// ENG-02 evidence lab: play `finish` for every Engineer defeat (`Finish::lab_engineer`). Presentation
    /// only: the events the simulation logged and hashed are untouched; a window run ends on the same
    /// `COMBAT_STATE` hash as the headless run without the flag.
    #[must_use]
    pub fn with_lab_engineer(mut self, finish: Finish) -> Self {
        self.lab_engineer = Some(finish);
        self
    }

    /// CF02-D (Q36): read the cf02 presentation inputs of `tuning`: the death
    /// overlay (a goblin `slammed` finish), the gore budget (Q22, Q43) and the
    /// slammed impact hold (Q21). Each is read through [`Cf02Tuning::on`], so a
    /// tuning with `cf02_rules` off changes nothing. Presentation only.
    #[must_use]
    pub fn with_cf02(mut self, tuning: &'static Cf02Tuning) -> Self {
        self.cf02 = Some(tuning);
        if tuning.on(tuning.q36_death_overlay) {
            self.profiles = if self.trog_profiles {
                DeathProfiles::with_trogs(true)
            } else {
                DeathProfiles::cf02()
            };
            self.slam_overlay = true;
        }
        self.gore_budget = Cf02GoreBudget::from_tuning(tuning);
        self
    }

    /// Whether the cf02 death overlay is active in this presenter.
    pub fn cf02_overlay(&self) -> bool {
        self.slam_overlay
    }

    /// Gib quarters of a finish of `force`: the cf02 budget's (Q22: 1.5x at
    /// preset 4 for heavy and catastrophic), the preset's own otherwise.
    fn gib_quarters_for(&self, force: ForceTier) -> u32 {
        self.gore_budget.map_or(self.preset.gib_quarters(), |b| {
            b.gib_quarters(self.preset, force)
        })
    }

    /// Add the Bad Llama's death art (LLAMA-02-DEATH): its poses and parts are
    /// composed for the active preset like the goblins' (the `molten` layer is
    /// shown at every preset). Without it a llama defeat draws nothing.
    pub fn with_llama(mut self, art: Arc<DeathArt>) -> Result<Self, String> {
        if art.variant != LLAMA_VARIANT {
            return Err("the llama death art has the wrong variant".into());
        }
        let sheets = art.compose(self.preset)?;
        self.sets.llama = Some(SpeciesDeath { art, sheets });
        Ok(self)
    }

    /// ENG-02: add the Goblin Engineer's death art (`eng_death_h36`, `eng_death_parts`): his crush, decapitation and
    /// explosion are drawn from it, and a slam launches its pot and wrench. Without it an Engineer's defeat plays
    /// the goblin set (variant A or B by index).
    pub fn with_engineer(mut self, art: Arc<DeathArt>) -> Result<Self, String> {
        if art.variant != ENGINEER_VARIANT {
            return Err("the Engineer death art has the wrong variant".into());
        }
        let sheets = art.compose(self.preset)?;
        self.sets.engineer = Some(SpeciesDeath { art, sheets });
        Ok(self)
    }

    /// TROG-FX-01: add the troglodytes' death art (Pygmy, Basher, Virtuoso: a pose and a parts sheet each) and their
    /// profiles (`DeathProfiles::with_trogs`, in front of the global or the cf02 profiles). Their point blank blast
    /// throws their own cut parts; every other defeat keeps the approved `down` frames and adds blood. Without it a
    /// troglodyte's defeat draws nothing extra, exactly as before.
    pub fn with_trogs(mut self, arts: [Arc<DeathArt>; TROG_KINDS]) -> Result<Self, String> {
        for (index, art) in arts.into_iter().enumerate() {
            if art.variant != TROG_FIRST_VARIANT + index {
                return Err("a troglodyte death art has the wrong variant".into());
            }
            let sheets = art.compose(self.preset)?;
            self.sets.trogs[index] = Some(SpeciesDeath { art, sheets });
        }
        self.trog_profiles = true;
        self.profiles = DeathProfiles::with_trogs(self.slam_overlay);
        Ok(self)
    }

    /// TROG-FX-01 evidence lab: play `finish` for every troglodyte defeat (`Finish::lab_trog`). Presentation only.
    pub fn with_lab_trog(mut self, finish: Finish) -> Self {
        self.lab_trog = Some(finish);
        self
    }

    /// Whether the run has the troglodyte death art.
    pub fn has_trogs(&self) -> bool {
        self.sets.trogs.iter().all(Option::is_some)
    }

    pub fn preset(&self) -> GorePreset {
        self.preset
    }

    /// The art of `variant` (goblin A or B, the llama or the Engineer).
    pub fn art_of(&self, variant: usize) -> &DeathArt {
        art_for(&self.art, &self.sets, variant)
    }

    /// Whether the run has the llama death art.
    pub fn has_llama(&self) -> bool {
        self.sets.llama.is_some()
    }

    /// Whether the run has the Engineer death art.
    pub fn has_engineer(&self) -> bool {
        self.sets.engineer.is_some()
    }

    pub fn art(&self) -> &DeathArt {
        &self.art
    }

    /// The composed sheets of the active preset (what the adapter uploads).
    pub fn sheets(&self) -> &PresetSheets {
        &self.sheets[0]
    }

    /// The composed sheets of death set `variant` (0 = goblin A, 1 = B, the llama, the Engineer).
    pub fn sheets_of(&self, variant: usize) -> &PresetSheets {
        sheets_for(&self.sheets, &self.sets, variant)
    }

    pub fn stats(&self) -> PresenterStats {
        self.stats
    }

    pub fn clock(&self) -> u64 {
        self.clock
    }

    /// Whether the GOB-01 body of goblin `target` (or the LLAMA-01 body of a
    /// llama target) is still in the scene: not replaced by an explosion's
    /// pieces (`hides`) and not retired. The sprite and its cast shadow share
    /// this one rule.
    pub fn draws_goblin(&self, target: u32) -> bool {
        !self.hides(target) && !self.is_retired(target)
    }

    /// DEATH-D: whether the GOB-01 sprite of goblin `target` (the LLAMA-01
    /// sprite of a llama target, LLAMA-DEATH-WIRE) is skipped: its
    /// death is an explosion at gore presets 2 to 4 (the pieces replace it)
    /// and the presenter clock has reached the death's start. Kept true for
    /// the rest of the attempt (a restart clears it). At preset 1 the sprite
    /// stays and plays the simulation's own knockback and corpse frames.
    pub fn hides(&self, target: u32) -> bool {
        self.exploded
            .iter()
            .any(|&(t, start)| t == target && self.clock >= start)
    }

    /// Forget every piece and death (a restart). The attempt counter is kept.
    pub fn clear(&mut self) {
        self.boss_finish = None;
        self.deaths.clear();
        self.retired.clear();
        self.exploded.clear();
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
            let art = art_for(&self.art, &self.sets, d.variant);
            let meta = art.poses.get(frame)?;
            let hazard_anchor = meta
                .hazard_over()
                .then(|| meta.anchor("hit_origin"))
                .flatten()
                .map(|hit| pose_anchor_world(art.pose_pivot, hit, d.foot_x, d.ground_y, d.flip));
            Some(PoseView {
                target: d.target,
                frame,
                foot_x: d.foot_x,
                ground_y: d.ground_y,
                flip: d.flip,
                variant: d.variant,
                alpha: self.death_alpha(d),
                hazard_anchor,
            })
        })
    }

    /// Where the falling block's anchor is drawn this tick when a pose with
    /// `hazard_draw: "over"` is playing (a crushed llama's contact frames), else
    /// `None`. Presentation only; the block's simulated position never moves.
    pub fn hazard_over_anchor(&self) -> Option<(f32, f32)> {
        self.poses().find_map(|p| p.hazard_anchor)
    }

    /// The impact dusts of the slams in progress (at most 20 ticks old: four
    /// frames of 5 ticks), at the floor under each slam's `hit_origin`.
    /// Presentation only.
    pub fn impact_dusts(&self) -> Vec<DustView> {
        self.deaths
            .iter()
            .filter(|d| d.finish == Finish::Slammed && d.target != SYNTHETIC_TARGET)
            .filter_map(|d| {
                let age = self.clock.saturating_sub(d.start);
                (age < 20).then(|| DustView {
                    x: self.pose_point(d, 6, "hit_origin").0,
                    ground_y: d.ground_y,
                    age,
                })
            })
            .collect()
    }

    /// Opacity of a corpse: 1, fading to 0 once it retires.
    fn death_alpha(&self, d: &Death) -> f32 {
        match d.retire_since {
            Some(since) => {
                let fade = DEATH_BUDGET.corpse_fade_ticks as f32;
                (1.0 - self.clock.saturating_sub(since) as f32 / fade).clamp(0.0, 1.0)
            }
            None => 1.0,
        }
    }

    /// Opacity of the corpse of goblin `target` (the GOB-01 corpse sprite
    /// included): 1 while it is drawn, fading to 0 as it retires, 0 once
    /// retired.
    pub fn corpse_alpha(&self, target: u32) -> f32 {
        if self.retired.contains(&target) {
            return 0.0;
        }
        self.deaths
            .iter()
            .find(|d| d.target == target)
            .map_or(1.0, |d| self.death_alpha(d))
    }

    /// Whether the corpse of goblin `target` has retired (not drawn at all).
    pub fn is_retired(&self, target: u32) -> bool {
        self.retired.contains(&target)
    }

    /// Settled corpses still drawn (not retiring): at most
    /// [`MAX_SETTLED_CORPSES`] once the presenter has stepped.
    pub fn visible_settled_corpses(&self) -> usize {
        self.deaths
            .iter()
            .filter(|d| self.is_settled(d) && d.retire_since.is_none())
            .count()
    }

    /// CINE-01: whether goblin or llama `target` still has a defeat sequence
    /// playing (a death entry that has not settled). The cinematic planner
    /// skips a moment rather than slow empty scenery. Presentation only.
    pub fn finish_active(&self, target: u32) -> bool {
        self.deaths
            .iter()
            .any(|d| d.target == target && !self.is_settled(d))
    }

    fn is_settled(&self, d: &Death) -> bool {
        d.target != SYNTHETIC_TARGET && d.done_after > 0 && self.clock >= d.start + d.done_after
    }

    /// The pose override of goblin `target`, if it has one.
    pub fn pose_of(&self, target: u32) -> Option<PoseView> {
        self.poses().find(|p| p.target == target)
    }

    /// CF02-D (Q21): the slammed goblin finishes in progress or settled (empty
    /// unless the cf02 overlay is on). Presentation only.
    pub fn slammed_poses(&self) -> impl Iterator<Item = SlamView> + '_ {
        self.deaths.iter().filter_map(|d| {
            let Finish::GoblinSlammed(pose) = d.finish else {
                return None;
            };
            Some(SlamView {
                target: d.target,
                pose,
                foot_x: d.foot_x,
                ground_y: d.ground_y,
                flip: d.flip,
                variant: d.variant,
                alpha: self.death_alpha(d),
            })
        })
    }

    /// CF02-D (Q21): the slammed pose goblin `target` was defeated with, if any.
    pub fn slam_pose_of(&self, target: u32) -> Option<SlamPose> {
        self.slammed_poses()
            .find(|v| v.target == target)
            .map(|v| v.pose)
    }

    pub fn parts(&self) -> impl Iterator<Item = (usize, &Part)> {
        self.parts.iter()
    }

    /// The part in pool slot `slot` (the adapter draws one sprite per slot).
    pub fn part_at(&self, slot: usize) -> Option<&Part> {
        self.parts.get(slot)
    }

    /// The floor decal in pool slot `slot`.
    pub fn floor_decal_at(&self, slot: usize) -> Option<&Decal> {
        self.floor_decals.get(slot)
    }

    /// The wall decal in pool slot `slot`.
    pub fn wall_decal_at(&self, slot: usize) -> Option<&Decal> {
        self.wall_decals.get(slot)
    }

    pub fn floor_decals(&self) -> impl Iterator<Item = (usize, &Decal)> {
        self.floor_decals.iter()
    }

    pub fn wall_decals(&self) -> impl Iterator<Item = (usize, &Decal)> {
        self.wall_decals.iter()
    }

    /// Opacity of the part in pool slot `slot` now: its time to live (short
    /// pieces only) times the fade of a recycle or a retiring corpse.
    pub fn part_alpha(&self, slot: usize, part: &Part) -> f32 {
        let base = match part.state {
            PartState::Resting { since } => fade_alpha(
                self.art_of(part.variant).parts[part.meta].ttl,
                since,
                self.clock,
            ),
            _ => 1.0,
        };
        base * recycle_alpha(self.parts.fade_since(slot), self.clock)
    }

    /// Opacity of the floor decal in slot `slot`.
    pub fn floor_decal_alpha(&self, slot: usize, decal: &Decal) -> f32 {
        fade_alpha(
            self.art_of(decal.variant).parts[decal.meta].ttl,
            decal.born,
            self.clock,
        ) * recycle_alpha(self.floor_decals.fade_since(slot), self.clock)
    }

    /// Opacity of the wall decal in slot `slot`.
    pub fn wall_decal_alpha(&self, slot: usize, decal: &Decal) -> f32 {
        fade_alpha(
            self.art_of(decal.variant).parts[decal.meta].ttl,
            decal.born,
            self.clock,
        ) * recycle_alpha(self.wall_decals.fade_since(slot), self.clock)
    }

    // ---------------------------------------------------------- stepping ----

    /// One fixed tick after the simulation step: ingest the new defeat events
    /// of `combat`, then advance the presentation.
    pub fn observe(&mut self, combat: &CombatState, scene: &SceneSim) {
        if self.engineers.is_empty() && combat.enemies.iter().any(|g| g.is_engineer()) {
            self.engineers = combat
                .enemies
                .iter()
                .enumerate()
                .filter(|(_, g)| g.is_engineer())
                .map(|(i, _)| i as u32)
                .collect();
        }
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
        let llamas: Vec<LlamaView> = combat
            .llamas
            .iter()
            .map(|l| LlamaView {
                x: l.x,
                ground_y: l.floor_y - LLAMA_HALF_HEIGHT,
                facing: l.facing,
                mode: l.mode,
            })
            .collect();
        let block_x = combat.shore.as_ref().map(|s| {
            let f = s.footprint();
            f.left + f.width / 2.0
        });
        let env_origins = combat.env().map(|e| e.hazard_origins()).unwrap_or_default();
        let total = combat.dropped_events + combat.events.len();
        let fresh = total
            .saturating_sub(self.seen_events)
            .min(combat.events.len());
        self.seen_events = total;
        let events = &combat.events[combat.events.len() - fresh..];
        if let Some(boss) = combat.boss.as_ref() {
            for event in events {
                self.begin_boss_death(
                    event,
                    boss.data().species.as_str(),
                    boss.x,
                    boss.floor_y(),
                    boss.facing < 0,
                );
            }
        }
        // ENEMY-SLOT: a slot enemy's fall goes through the same death language
        // (the species hook); one with no death art draws nothing.
        for event in events {
            if let CombatEvent::EnemyDefeated { target, .. } = *event
                && target >= crate::scene_enemy::SLOT_ID_MIN
                && let Some(body) = combat.slot_death_body(target)
            {
                self.begin_species_death(
                    event,
                    body.species,
                    body.x,
                    body.ground_y,
                    body.facing < 0,
                    Some(target),
                );
            }
        }
        self.step(
            events,
            Observation {
                goblins: &goblins,
                llamas: &llamas,
                hero_x: scene.world.hero.x,
                block_x,
                back_wall: true,
                charges: &combat.charges,
                env_origins: &env_origins,
            },
        );
    }

    /// One fixed tick with the events logged during it.
    pub fn step(&mut self, events: &[CombatEvent], obs: Observation<'_>) {
        let recaused: Vec<CombatEvent>;
        let events = if self.lab_edge {
            recaused = events.iter().map(|&e| lab_edge_event(e)).collect();
            &recaused[..]
        } else {
            events
        };
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
                } => self.begin_death(tick, target, cause, force, events, &obs),
                _ => {}
            }
        }
        self.back_wall = obs.back_wall;
        self.clock += 1;
        self.step_parts();
        self.step_deaths(&obs);
        self.retire_corpses();
        let fade = DEATH_BUDGET.recycle_fade_ticks;
        self.parts.expire_faded(self.clock, fade);
        self.floor_decals.expire_faded(self.clock, fade);
        self.wall_decals.expire_faded(self.clock, fade);
    }

    /// The corpse limit (owner decision): once more than
    /// [`MAX_SETTLED_CORPSES`] settled corpses are drawn, the oldest start to
    /// fade out over `corpse_fade_ticks`, together with the parts their death
    /// launched; when the fade ends the death script is freed and the goblin
    /// is remembered as retired. Stains stay (they leave when a pool recycles
    /// them). Presentation only.
    fn retire_corpses(&mut self) {
        let clock = self.clock;
        let fade = DEATH_BUDGET.corpse_fade_ticks;
        let mut gone = Vec::new();
        self.deaths.retain(|d| match d.retire_since {
            Some(since) if clock >= since + fade => {
                gone.push(d.target);
                false
            }
            _ => true,
        });
        self.retired.extend(gone);
        let mut settled: Vec<(u64, u32, usize)> = self
            .deaths
            .iter()
            .enumerate()
            .filter(|(_, d)| self.is_settled(d) && d.retire_since.is_none())
            .map(|(i, d)| (d.start, d.target, i))
            .collect();
        if settled.len() <= MAX_SETTLED_CORPSES {
            return;
        }
        settled.sort_unstable();
        let excess = settled.len() - MAX_SETTLED_CORPSES;
        for (_, _, index) in settled.into_iter().take(excess) {
            self.deaths[index].retire_since = Some(clock);
            let id = self.deaths[index].id;
            let slots: Vec<usize> = self
                .parts
                .iter()
                .filter(|(_, p)| p.death == id)
                .map(|(slot, _)| slot)
                .collect();
            for slot in slots {
                self.parts.start_fade(slot, clock);
            }
        }
    }

    /// HUD-DEBT-03: the boss' defeat through the same death language. The
    /// finish comes from the `brute_boss` profile (`brute_ko_timber` for a
    /// melee knockout, `brute_crushed` under the rack). Until the boss death
    /// art exists the cosmetics are the goblin crush or knockback burst (dust
    /// and, at preset 4, blood and a pool) at the boss' feet; nothing here
    /// reaches the simulation. The finish chosen is kept for the trace.
    ///
    /// BOS-AD31: the species is the boss data's (`brute_boss` for the Juicer);
    /// the Ball's `rolling_brute` has no profile and plays the generic fallback.
    fn begin_boss_death(
        &mut self,
        event: &CombatEvent,
        species: &str,
        x: f32,
        ground_y: f32,
        flip: bool,
    ) {
        if matches!(event, CombatEvent::EnemyDefeated { target, .. } if *target == crate::boss::BOSS_ENEMY_ID)
            && let Some(id) = self.begin_species_death(event, species, x, ground_y, flip, None)
        {
            self.boss_finish = Some(id);
        }
    }

    /// The finish `species` plays for a defeat event, spawned as a synthetic
    /// death at `(x, ground_y)` (the boss' and the slot enemies' path); returns
    /// the finish id. ENEMY-SLOT: a species with no profile, or whose finish
    /// has no art yet, draws nothing (`None`); the simulation is untouched.
    ///
    /// TROG-FX-01: a troglodyte's death plays its own set (`trog_variant`), and a point blank
    /// blast that replaces its body at gore presets 2 to 4 hides the sprite of the slot enemy
    /// `slot` (`hides`), like an exploded goblin.
    fn begin_species_death(
        &mut self,
        event: &CombatEvent,
        species: &str,
        x: f32,
        ground_y: f32,
        flip: bool,
        slot: Option<u32>,
    ) -> Option<String> {
        let CombatEvent::EnemyDefeated {
            tick, cause, force, ..
        } = *event
        else {
            return None;
        };
        let selection = self.profiles.select(species, cause, force);
        let finish_id = selection.finish.to_owned();
        let mut finish = Finish::from_id(&finish_id);
        let variant = trog_variant(species).filter(|v| self.sets.of(*v).is_some());
        if variant.is_some()
            && let Some(lab) = self.lab_trog
        {
            finish = lab;
        }
        let start = self.spawn_synthetic_in(finish, variant, x, ground_y, flip, tick);
        if let (Some(slot), Some(start), Finish::Exploded) = (slot, start, finish)
            && variant.is_some()
            && self.preset >= GorePreset::Reduced
        {
            self.exploded.push((slot, start));
        }
        Some(finish_id)
    }

    /// The finish id chosen for the boss' defeat, if it fell.
    pub fn boss_finish(&self) -> Option<&str> {
        self.boss_finish.as_deref()
    }

    fn begin_death(
        &mut self,
        tick: u64,
        target: u32,
        cause: DefeatCause,
        force: ForceTier,
        events: &[CombatEvent],
        obs: &Observation<'_>,
    ) {
        // Goblins first, then llamas (the same target ids as the simulation).
        let goblins = obs.goblins.len();
        let (x, ground_y, facing, variant, species) = if (target as usize) < goblins {
            let g = &obs.goblins[target as usize];
            let engineer = self.engineers.contains(&target);
            (
                g.x,
                g.ground_y,
                g.facing,
                // ENG-02: an Engineer plays his own set when the run has it.
                if engineer && self.sets.engineer.is_some() {
                    ENGINEER_VARIANT
                } else {
                    goblin_variant(target as usize)
                },
                if engineer {
                    ENGINEER_SPECIES
                } else {
                    self.species
                },
            )
        } else if self.sets.llama.is_some()
            && let Some(l) = obs.llamas.get(target as usize - goblins)
        {
            (l.x, l.ground_y, l.facing, LLAMA_VARIANT, LLAMA_SPECIES)
        } else {
            return;
        };
        let selection = self.profiles.select(species, cause, force);
        let mut finish = Finish::from_id(selection.finish);
        if let Finish::GoblinSlammed(_) = finish {
            // CF02-D (Q21): the pose comes from the defeat's same-tick events.
            finish = Finish::GoblinSlammed(slam_pose_for(events, target));
        }
        if species == ENGINEER_SPECIES
            && let Some(lab) = self.lab_engineer
        {
            finish = lab;
        }
        let flip = facing < 0;
        let mut x = x;
        // A crushed llama's pose is drawn with its mirrored `hit_origin` under
        // the block's centre (`hazard_draw: "over"`): the stone never moves
        // sideways on a partial overlap, the llama's drawn pose does (render
        // only, the simulated body does not move).
        if variant == LLAMA_VARIANT
            && let (DefeatCause::ShoreBlock { .. }, Some(block)) = (cause, obs.block_x)
            && let Some(art) = self.sets.llama.as_ref().map(|l| &l.art)
            && let Some((hx, _)) = art.pose(0).anchor("hit_origin")
        {
            let dx = hx as f32 + 0.5 - art.pose_pivot.0 as f32;
            x = if flip { block + dx } else { block - dx };
        }
        // The point the blow came from: the block's centre for the block and
        // its edge, the charge for an explosion, Carl for a melee hit.
        let charge = match cause {
            DefeatCause::Explosion { charge, .. } => obs.charges.get(charge as usize),
            _ => None,
        };
        let from_x = match (cause, obs.block_x) {
            (DefeatCause::ShoreBlock { .. } | DefeatCause::ShoreBlockEdge { .. }, Some(block)) => {
                block
            }
            // BOIL-E: a prop's blow comes from its volume, not from the hero.
            (DefeatCause::Environment { prop, class, .. }, _) => obs
                .env_origins
                .iter()
                .find(|(p, c, _)| *p == prop && *c == class)
                .map_or(obs.hero_x, |(_, _, x)| *x),
            _ => charge.map_or(obs.hero_x, |c| c.x),
        };
        let away = if (x - from_x).abs() < 0.5 {
            -f32::from(facing)
        } else {
            (x - from_x).signum()
        };
        let mut death = Death::new(
            self.next_death_id,
            target,
            variant,
            finish,
            force,
            self.clock + 1,
            (x, ground_y),
            flip,
            away,
            PresentationRng::for_event(self.attempt, tick, target),
        );
        if finish == Finish::Exploded {
            let table = explosion_table_of(variant, self.preset);
            death.drop_cap = table.drop_splat_cap;
            death.stain_cap = table.part_stain_cap;
            death.pool_scale = table.pool_scale;
            death.impact = Some(charge.map_or((x, ground_y + BLAST_CENTRE_DY), |c| c.centre()));
        }
        if let (DefeatCause::ShoreBlockEdge { .. }, Some(block)) = (cause, obs.block_x) {
            // The edge that cut the neck: the footprint edge on the goblin's
            // side of the block.
            let side = if x >= block { 1.0 } else { -1.0 };
            death.edge_splat_x = Some(block + side * FOOTPRINT_WIDTH / 2.0);
        }
        if finish == Finish::Exploded && self.preset >= GorePreset::Reduced {
            self.exploded.push((target, death.start));
        }
        self.start_death(death);
    }

    /// Start `death`; `false` when the pool refused it.
    fn start_death(&mut self, death: Death) -> bool {
        if self.deaths.len() >= MAX_DEATHS {
            // Retire the oldest finished synthetic one, else refuse.
            if let Some(at) = self
                .deaths
                .iter()
                .position(|d| d.target == SYNTHETIC_TARGET)
            {
                self.deaths.remove(at);
            } else {
                return false;
            }
        }
        self.next_death_id += 1;
        self.deaths.push(death);
        true
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
        self.spawn_synthetic_in(finish, None, x, ground_y, flip, salt);
    }

    /// [`Self::spawn_synthetic`] in death set `variant` (a troglodyte's; `None`: the goblin
    /// variant the salt picks). Returns the presenter tick the death starts on, `None` when the
    /// pool refused it.
    fn spawn_synthetic_in(
        &mut self,
        finish: Finish,
        variant: Option<usize>,
        x: f32,
        ground_y: f32,
        flip: bool,
        salt: u64,
    ) -> Option<u64> {
        let away = if flip { 1.0 } else { -1.0 };
        let mut death = Death::new(
            self.next_death_id,
            SYNTHETIC_TARGET,
            // Both variants in the stress, so both sheets are exercised.
            variant.unwrap_or((salt as usize / 3) % DEATH_VARIANTS),
            finish,
            ForceTier::Catastrophic,
            self.clock + 1,
            (x, ground_y),
            flip,
            away,
            PresentationRng::for_event(self.attempt, salt, SYNTHETIC_TARGET),
        );
        if finish == Finish::Exploded {
            // A synthetic explosion has no charge: the impact is under it.
            let table = explosion_table_of(death.variant, self.preset);
            death.drop_cap = table.drop_splat_cap;
            death.stain_cap = table.part_stain_cap;
            death.pool_scale = table.pool_scale;
            death.impact = Some((x, ground_y + BLAST_CENTRE_DY));
        }
        let start = death.start;
        self.start_death(death).then_some(start)
    }

    /// Lab stress (`--death-stress`): keep the pools loaded by starting a
    /// synthetic crush or decapitation every few ticks around `hero_x`.
    pub fn stress_tick(&mut self, hero_x: f32, ground_y: f32) {
        if !self.clock.is_multiple_of(3) {
            return;
        }
        let side = if self.stress_rng.coin() { 1.0 } else { -1.0 };
        let x = hero_x + side * self.stress_rng.range(30.0, 220.0);
        // Crush, decapitation and explosion in thirds (the clock advances 3
        // ticks per synthetic death).
        let finish = match (self.clock / 3) % 3 {
            0 => Finish::Crushed,
            1 => Finish::Decapitated,
            _ => Finish::Exploded,
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
            // CF02-D (Q43): a transient spray is gone once its lifetime ends.
            if part.transient_until.is_some_and(|until| clock >= until) {
                self.parts.free(slot);
                continue;
            }
            let art = art_for(&self.art, &self.sets, part.variant);
            let meta = &art.parts[part.meta];
            if let PartState::Resting { since } = part.state {
                if expired(meta.ttl, since, clock) {
                    self.parts.free(slot);
                }
                continue;
            }
            let hull = &sheets_for(&self.sheets, &self.sets, part.variant)
                .parts
                .hull[part.frame];
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
                    contacts.push(Contact::Rested {
                        death: part.death,
                        meta: part.meta,
                        x: part.x,
                    });
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
            // A transient spray drop dies on contact and leaves nothing.
            if part.transient_until.is_some() {
                self.parts.free(slot);
                continue;
            }
            // A blood drop, or a lava drop that leaves a molten decal, dies on
            // contact and leaves its decal.
            if meta.class == PartClass::Blood
                || (meta.class == PartClass::Molten && meta.on_contact.is_some())
            {
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
                && let Some(index) = art.part_index(swap)
            {
                part.meta = index;
                part.frame = art.parts[index].frame;
            }
            let meta_index = part.meta;
            let bounce = art.parts[meta_index].bounce;
            let rolls = art.parts[meta_index].settle == Settle::Roll;
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
                    contacts.push(Contact::Rested {
                        death: part.death,
                        meta: part.meta,
                        x: part.x,
                    });
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
                .filter(|(_, d)| {
                    expired(
                        art_for(&self.art, &self.sets, d.variant).parts[d.meta].ttl,
                        d.born,
                        clock,
                    )
                })
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
            | Contact::RollStarted { death, meta, .. }
            | Contact::Rested { death, meta, .. } => (death, meta),
        };
        let Some(slot) = self.deaths.iter().position(|d| d.id == death) else {
            return;
        };
        let variant = self.deaths[slot].variant;
        let part = &art_for(&self.art, &self.sets, variant).parts[meta_index];
        let ground_y = self.deaths[slot].ground_y;
        match contact {
            Contact::DropLanded { x, .. } => {
                // Lava drops have their own cap (at most 6 molten decals).
                let molten = part.class == PartClass::Molten;
                if molten {
                    if self.deaths[slot].molten_decals >= MAX_MOLTEN_DECALS {
                        return;
                    }
                } else if self.deaths[slot].drop_splats >= self.deaths[slot].drop_cap {
                    return;
                }
                if let Some(id) = part.on_contact.clone()
                    && self.spawn_floor_decal(variant, &id, x, ground_y, false)
                {
                    if molten {
                        self.deaths[slot].molten_decals += 1;
                    } else {
                        self.deaths[slot].drop_splats += 1;
                    }
                }
            }
            Contact::RollStarted { x, dir, .. } => {
                if let Some(trail) = part.trail.clone() {
                    self.spawn_floor_decal(variant, &trail, x, ground_y, dir < 0.0);
                }
            }
            Contact::Rested { x, .. } => {
                // The first rest of a tracked torso piece grows a pool under it.
                // The llama's two heavy pieces (LLAMA-02 section 8: pools
                // under the two heaviest) are its torso halves.
                let tracked = match part.id.as_str() {
                    "torso_upper" | "torso_front" => Some(0),
                    "torso_lower" | "torso_rump" => Some(1),
                    _ => None,
                };
                let clock = self.clock;
                if let Some(k) = tracked
                    && self.deaths[slot].finish == Finish::Exploded
                    && self.deaths[slot].rest_ticks[k].is_none()
                {
                    self.deaths[slot].rest_ticks[k] = Some(clock);
                    self.deaths[slot].rest_x[k] = x;
                }
            }
            Contact::Touched { x, first, .. } => {
                // A stain on the first touch (the head: on every bounce).
                if let Some(stain) = part.stain.clone()
                    && (first || part.trail.is_some())
                    && self.deaths[slot].part_stains < self.deaths[slot].stain_cap
                    && self.spawn_floor_decal(variant, &stain, x, ground_y, false)
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
        let art = art_for(&self.art, &self.sets, death.variant);
        let Some(index) = art.part_index(id) else {
            return false;
        };
        let meta = &art.parts[index];
        if !meta.allowed_at(self.preset) {
            return false;
        }
        let cell = art.part_cell;
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
            transient_until: self.transient_until,
            variant: death.variant,
        };
        let (goblin_art, sets, now) = (&self.art, &self.sets, self.clock);
        self.parts.spawn_ranked(part, now, |p| {
            ttl_rank(art_for(goblin_art, sets, p.variant).parts[p.meta].ttl)
        });
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
    fn spawn_floor_decal(
        &mut self,
        variant: usize,
        id: &str,
        x: f32,
        ground_y: f32,
        flip: bool,
    ) -> bool {
        let Some((meta, decal)) = self.floor_decal_for(variant, id, x, ground_y, flip, 1.0) else {
            return false;
        };
        let _ = meta;
        let (goblin_art, sets, now) = (&self.art, &self.sets, self.clock);
        self.floor_decals.spawn_ranked(decal, now, |d| {
            ttl_rank(art_for(goblin_art, sets, d.variant).parts[d.meta].ttl)
        });
        self.stats.floor_decals_spawned += 1;
        self.stats.peak_floor_decals = self.stats.peak_floor_decals.max(self.floor_decals.live());
        true
    }

    fn floor_decal_for(
        &self,
        variant: usize,
        id: &str,
        x: f32,
        ground_y: f32,
        flip: bool,
        scale: f32,
    ) -> Option<(usize, Decal)> {
        // Decals are the same in both goblin sets: they stay variant 0 (the
        // sheet and digest they always had); only the llama has its own.
        let variant = decal_set(variant);
        let art = art_for(&self.art, &self.sets, variant);
        let index = art.part_index(id)?;
        let meta = &art.parts[index];
        if !meta.allowed_at(self.preset) {
            return None;
        }
        let cell = art.part_cell;
        let bbox = sheets_for(&self.sheets, &self.sets, variant).parts.bbox[meta.frame]?;
        // Horizontal: the named anchor pixel on `x`; vertical: the lowest
        // opaque row on the floor line.
        let anchor = meta
            .anchor("centre_bottom")
            .or_else(|| meta.anchor("start"))
            .unwrap_or((cell.0 / 2, cell.1 / 2));
        let centre = part_center_for_anchor(cell, anchor, (x, 0.0), flip);
        // Scaled about the centre: the anchor offset and the bottom row scale.
        let centre = (x + scale * (centre.0 - x), centre.1);
        let pair_meta = meta
            .pair
            .as_deref()
            .filter(|_| meta.pair_offset.is_some())
            .and_then(|p| art.part_index(p));
        Some((
            index,
            Decal {
                meta: index,
                pair_meta,
                x: centre.0,
                y: ground_y + scale * (bbox.bottom - cell.1 / 2) as f32,
                flip,
                kind: DecalKind::Floor,
                scale,
                born: self.clock,
                variant,
            },
        ))
    }

    /// Grow the death's pool decal to `id` (one decal that swaps its frame, so
    /// each stage contains the previous one); spawn it when none lives.
    fn pool_stage(&mut self, death: &mut Death, which: usize, id: &str, x: f32) {
        let Some((index, fresh)) = self.floor_decal_for(
            death.variant,
            id,
            x,
            death.ground_y,
            false,
            death.pool_scale,
        ) else {
            return;
        };
        let pair = fresh.pair_meta;
        if let Some((slot, seq)) = death.pools[which]
            && let Some(decal) = self.floor_decals.get_seq_mut(slot, seq)
        {
            decal.meta = index;
            decal.pair_meta = pair;
            decal.x = fresh.x;
            decal.y = fresh.y;
            decal.scale = fresh.scale;
            return;
        }
        let (goblin_art, sets, now) = (&self.art, &self.sets, self.clock);
        let (slot, seq) = self.floor_decals.spawn_ranked(fresh, now, |d| {
            ttl_rank(art_for(goblin_art, sets, d.variant).parts[d.meta].ttl)
        });
        death.pools[which] = Some((slot, seq));
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
        self.wall_spatter_variant(death.variant, id, flip, at);
    }

    /// One named back-wall spatter whose origin pixel is on `at`.
    fn wall_spatter_variant(&mut self, variant: usize, id: &str, flip: bool, at: (f32, f32)) {
        self.wall_spatter_scaled(variant, id, flip, at, 1.0);
    }

    /// A back-wall spatter drawn `scale` times the art's size about its
    /// centre, its origin pixel still on `at`.
    fn wall_spatter_scaled(
        &mut self,
        variant: usize,
        id: &str,
        flip: bool,
        at: (f32, f32),
        scale: f32,
    ) {
        if !self.back_wall {
            return;
        }
        let variant = decal_set(variant);
        let art = art_for(&self.art, &self.sets, variant);
        let Some(index) = art.part_index(id) else {
            return;
        };
        let meta = &art.parts[index];
        if !meta.allowed_at(self.preset) {
            return;
        }
        let cell = art.part_cell;
        let anchor = meta.anchor("origin").unwrap_or((cell.0 / 2, cell.1 / 2));
        let centre = part_center_for_anchor(cell, anchor, at, flip);
        let centre = (
            at.0 + scale * (centre.0 - at.0),
            at.1 + scale * (centre.1 - at.1),
        );
        let now = self.clock;
        self.wall_decals.spawn(
            Decal {
                meta: index,
                pair_meta: None,
                x: centre.0,
                y: centre.1,
                flip,
                kind: DecalKind::Wall,
                scale,
                born: now,
                variant,
            },
            now,
        );
        self.stats.wall_decals_spawned += 1;
        self.stats.peak_wall_decals = self.stats.peak_wall_decals.max(self.wall_decals.live());
    }

    // ---------------------------------------------------------- scripts ----

    fn step_deaths(&mut self, obs: &Observation<'_>) {
        for i in 0..self.deaths.len() {
            let mut death = self.deaths[i].clone();
            // A slammed llama slides clear of its attacker for a few ticks (the
            // simulated body moves); its pose follows the body.
            if death.finish == Finish::Slammed
                && let Some(x) = Self::lying_x(obs, death.target)
            {
                death.foot_x = x;
            }
            let age = self.clock.saturating_sub(death.start);
            if self.clock >= death.start {
                match death.finish {
                    Finish::Crushed => self.script_crush(&mut death, age),
                    Finish::Decapitated => self.script_decap(&mut death, age),
                    Finish::Knockback | Finish::GoblinSlammed(_) => {
                        self.script_knockback(&mut death, age, obs);
                    }
                    Finish::Slammed => self.script_slam(&mut death, age),
                    Finish::Exploded => self.script_explode(&mut death, age),
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

    /// Whether part `id` of `variant`'s set exists and the preset draws it.
    fn part_allowed(&self, variant: usize, id: &str) -> bool {
        let art = self.art_of(variant);
        art.part_index(id)
            .is_some_and(|i| art.parts[i].allowed_at(self.preset))
    }

    /// World position of anchor `name` of pose frame `frame` of `death`.
    fn pose_point(&self, death: &Death, frame: usize, name: &str) -> (f32, f32) {
        let art = self.art_of(death.variant);
        let anchor = art.pose(frame).anchor(name).unwrap_or(art.pose_pivot);
        pose_anchor_world(
            art.pose_pivot,
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
        if age == 0 && d.variant == LLAMA_VARIANT {
            self.llama_crush_burst(d);
        }
        if age == 0 {
            // The club leaves the hand; the Engineer's wrench (ENG2-AD10) leaves the back hip, with the
            // club's draws.
            let (held, from) = if d.variant == ENGINEER_VARIANT {
                ("wrench", "hip_back")
            } else {
                ("club", "hand_near")
            };
            let hand = self.pose_point(d, 0, from);
            let (vx, vy) = (d.away * d.rng.range(90.0, 150.0), d.rng.range(120.0, 180.0));
            let spin = -d.away * d.rng.range(540.0, 900.0);
            self.launch(d, held, Some("grip"), hand, (vx, vy), 0.0, spin, d.flip);
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
            .filter(|(id, _)| self.part_allowed(d.variant, id))
            .collect();
            // Q22 (cf02, preset 4): 6 quarters can ask for more gibs than the
            // set has, so the set repeats; below that this is the legacy take.
            let keep = scale_count(gibs.len() as u32, self.gib_quarters_for(d.force)) as usize;
            for (id, anchor) in gibs.into_iter().cycle().take(keep) {
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
                self.spawn_floor_decal(d.variant, "decal_splat_m", from.0, d.ground_y, false);
            }
        }
        let pool = self.pose_point(d, 2, "pool_origin");
        for (tick, id) in [
            (first, "decal_pool_1"),
            (second, "decal_pool_2"),
            (third, "decal_pool_3"),
        ] {
            if age == tick {
                self.pool_stage(d, 0, id, pool.0);
            }
        }
        // L+8: the squeeze blows sideways onto the back wall. Preset 4 puts
        // both spatter variants up, one fanning each way and a third between
        // them, so it reads as a splash, not one mark (the parts are the
        // spec's; presets below 4 do not draw them).
        if age == 8 {
            let at = (d.foot_x, d.ground_y + 8.0);
            let left = d.rng.coin();
            self.wall_spatter_variant(d.variant, "decal_wall_spatter", left, (at.0 - 4.0, at.1));
            self.wall_spatter_variant(d.variant, "decal_wall_spatter", !left, (at.0 + 4.0, at.1));
            let centre = d.rng.coin();
            self.wall_spatter_variant(
                d.variant,
                "decal_wall_spatter_b",
                centre,
                (at.0, at.1 + 2.0),
            );
        }
    }

    /// Decapitation, spec section 8: the tick D the block's edge cut the neck
    /// (DEATH-D: cause `ShoreBlockEdge`).
    fn script_decap(&mut self, d: &mut Death, age: u64) {
        d.done_after = 150;
        self.edge_pinch(d, age);
        if d.variant == LLAMA_VARIANT {
            self.script_llama_decap(d, age);
            return;
        }
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
                // Q22 (cf02, preset 4): a heavy finish may separate one more
                // part where the anatomy allows (an arm).
                if let Some(budget) = self.gore_budget {
                    let more = budget.extra_parts(self.preset, d.force) as usize;
                    for id in ["arm_near", "arm_far"].into_iter().take(more) {
                        let (sin, cos) = d.rng.range(35.0, 65.0).to_radians().sin_cos();
                        let speed = d.rng.range(70.0, 130.0);
                        let spin = if d.rng.coin() { 540.0 } else { -540.0 };
                        let from = (neck.0, neck.1 - 8.0);
                        self.launch(
                            d,
                            id,
                            Some("shoulder"),
                            from,
                            (-d.away * cos * speed, sin * speed),
                            0.0,
                            spin,
                            d.flip,
                        );
                    }
                }
            }
            4 => {
                let neck = self.pose_point(d, 4, "neck");
                self.wall_spatter(d, (neck.0, d.ground_y + 16.0));
            }
            14 => {
                let pool = self.pose_point(d, 5, "pool_origin");
                self.pool_stage(d, 0, "decal_pool_1", pool.0);
            }
            40 => {
                let pool = self.pose_point(d, 5, "pool_origin");
                self.pool_stage(d, 0, "decal_pool_2", pool.0);
            }
            100 => {
                let pool = self.pose_point(d, 5, "pool_origin");
                self.pool_stage(d, 0, "decal_pool_3", pool.0);
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

    /// The block's edge pinched the neck (a real `ShoreBlockEdge` only): a
    /// squeeze of low, fast drops from the neck away from the stone on D, and
    /// a medium splat on the walking line at the edge on D+2.
    fn edge_pinch(&mut self, d: &mut Death, age: u64) {
        if let Some(edge_x) = d.edge_splat_x {
            if age == 0 {
                let neck = self.pose_point(d, 3, "neck");
                let n = scale_count(EDGE_SQUEEZE_DROPS, self.preset.drop_quarters());
                for _ in 0..n {
                    let elevation = d.rng.range(10.0, 35.0);
                    let speed = d.rng.range(80.0, 160.0);
                    self.drop_blood(d, neck, d.away, elevation, speed);
                }
            }
            if age == 2 {
                self.spawn_floor_decal(d.variant, "decal_splat_m", edge_x, d.ground_y, false);
            }
        }
    }

    /// LLAMA-DEATH-WIRE: the llama's decapitation, LLAMA-02 spec section 8
    /// (`death_poses.spawner_hints.decapitation` of `llama_death_h64.json`),
    /// tick D: pose 3 for 4 ticks, pose 4 for 6, pose 5 held (the shared
    /// `pose_frame`). The head flies muzzle up from the pose 3 `neck` and
    /// swaps to `head_dead` on its first floor contact (manifest); the jet is
    /// 3 drops every 3 ticks for 36 ticks from D+4; the cut duct dribbles one
    /// lava drop every 6 ticks for 24 ticks (shown at every preset, half as
    /// many at presets 1 and 2, as the crush burst); the trickle and the pool
    /// stages are the goblin's.
    fn script_llama_decap(&mut self, d: &mut Death, age: u64) {
        match age {
            0 => {
                let neck = self.pose_point(d, 3, "neck");
                let (vx, vy) = (d.away * d.rng.range(50.0, 90.0), d.rng.range(240.0, 300.0));
                self.launch(
                    d,
                    "head",
                    Some("neck"),
                    neck,
                    (vx, vy),
                    -25.0 * d.away,
                    -600.0 * d.away,
                    d.flip,
                );
            }
            4 => {
                let neck = self.pose_point(d, 4, "neck");
                self.wall_spatter(d, (neck.0, d.ground_y + 16.0));
            }
            14 => {
                let pool = self.pose_point(d, 5, "pool_origin");
                self.pool_stage(d, 0, "decal_pool_1", pool.0);
            }
            40 => {
                let pool = self.pose_point(d, 5, "pool_origin");
                self.pool_stage(d, 0, "decal_pool_2", pool.0);
            }
            100 => {
                let pool = self.pose_point(d, 5, "pool_origin");
                self.pool_stage(d, 0, "decal_pool_3", pool.0);
            }
            _ => {}
        }
        // LDW-AD5: the jet and the lava leave the buckle's (pose 4) anchors
        // while it shows and the propped stump of the hold (pose 5) after, so
        // they never hang in the air above the lying body.
        let held = age >= 10;
        if (4..40).contains(&age) && (age - 4).is_multiple_of(3) {
            let base = if held {
                self.pose_point(d, 5, "neck")
            } else {
                self.pose_point(d, 4, "jet_base")
            };
            for _ in 0..3 {
                let k = d.drops_emitted;
                d.drops_emitted += 1;
                if !keeps_stream_item(k, self.preset.drop_quarters()) {
                    continue;
                }
                let spread = d.rng.range(-12.0, 12.0).to_radians();
                let up = d.rng.range(170.0, 240.0);
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
        if (4..28).contains(&age) && (age - 4).is_multiple_of(6) {
            let quarters = if self.preset >= GorePreset::Medium {
                4
            } else {
                2
            };
            if keeps_stream_item(((age - 4) / 6) as u32, quarters) {
                let gland = self.pose_point(d, if held { 5 } else { 4 }, "gland");
                // LDW-AD6: a slow dribble out of the duct (the spec gives no
                // speed): it lands within a few pixels of the stump.
                let (vx, vy) = (d.rng.range(-15.0, 15.0), d.rng.range(40.0, 90.0));
                self.launch(d, "drop_molten", None, gland, (vx, vy), 0.0, 0.0, false);
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

    /// A spin in degrees per second from a manifest spin class, sign seeded.
    fn class_spin(rng: &mut PresentationRng, class: SpinClass) -> f32 {
        let spin = match class {
            SpinClass::Fast => rng.range(720.0, 1080.0),
            SpinClass::Slow => rng.range(180.0, 360.0),
            SpinClass::None => 0.0,
        };
        if rng.coin() { spin } else { -spin }
    }

    /// Explosive breakup (DEATH-D, spec section 4.2): every piece of the
    /// preset's table leaves the live pose radially from the impact point `I`
    /// (the charge centre); the goblin sprite is hidden by the adapter at
    /// presets 2 to 4 (`hides`).
    fn script_explode(&mut self, d: &mut Death, age: u64) {
        d.done_after = 240;
        let table = explosion_table_of(d.variant, self.preset);
        let (ix, iy) = d.impact.unwrap_or((d.foot_x, d.ground_y + BLAST_CENTRE_DY));
        let (foot_x, ground_y, flip) = (d.foot_x, d.ground_y, d.flip);
        match age {
            0 => {
                // The set of this death (goblin A, or the llama: its own
                // frame, pivot and parts); a cheap handle, so `launch` may
                // borrow the presenter.
                let art = Arc::clone(self.sets.of(d.variant).map_or(&self.art, |set| &set.art));
                let pivot = art.pose_pivot;
                let origin_of = |art: &DeathArt, id: &str| {
                    art.part_index(id)
                        .and_then(|i| art.parts[i].frame0_origin)
                        .map(|o| pose_anchor_world(pivot, (o[0], o[1]), foot_x, ground_y, flip))
                };
                // The torso point: where the torso is on frame 0.
                let torso = origin_of(&art, table.torso).unwrap_or((foot_x, ground_y + 20.0));
                // The throat: where a part with no origin of its own (the
                // llama's gland) starts, and where the lava comes out.
                let throat = origin_of(&art, table.throat).unwrap_or(torso);
                for id in table.parts {
                    let Some(index) = art.part_index(id) else {
                        continue;
                    };
                    let (spin_class, mass) = (art.parts[index].spin, art.parts[index].mass);
                    let start = match origin_of(&art, id) {
                        Some(start) => start,
                        // LDW-AD1: only the llama lists a part without a
                        // `frame0_origin` (the gland, in the throat).
                        None if d.variant == LLAMA_VARIANT => throat,
                        None => continue,
                    };
                    let (dx, dy) = (start.0 - ix, start.1 - iy);
                    let len = dx.hypot(dy);
                    let unit = if len < 2.0 {
                        (0.0, 1.0)
                    } else {
                        (dx / len, dy / len)
                    };
                    let speed =
                        d.rng.range(160.0, 300.0) * if mass == Mass::Heavy { 0.7 } else { 1.0 };
                    let vel = (unit.0 * speed, unit.1 * speed + d.rng.range(60.0, 120.0));
                    let spin = Self::class_spin(&mut d.rng, spin_class);
                    self.launch(d, id, None, start, vel, 0.0, spin, d.flip);
                }
                for &(id, n) in table.gibs {
                    // Q22 (cf02, preset 4): catastrophic force asks 6 quarters.
                    // A clean piece (the llama's one knocked-out tooth,
                    // LDW-AD3) is not a gib and is never scaled.
                    let clean = art
                        .part_index(id)
                        .is_some_and(|i| art.parts[i].class == PartClass::Clean);
                    let n = match self.gore_budget {
                        Some(b) if self.preset == GorePreset::Full && !clean => {
                            scale_count(n, b.gib_quarters(self.preset, d.force))
                        }
                        _ => n,
                    };
                    // The manifest's spin class drives the gib, as for a part.
                    let gib_class = art
                        .part_index(id)
                        .map_or(SpinClass::Fast, |i| art.parts[i].spin);
                    for _ in 0..n {
                        let (sin, cos) = d.rng.range(20.0, 160.0).to_radians().sin_cos();
                        let speed = d.rng.range(140.0, 260.0);
                        let spin = Self::class_spin(&mut d.rng, gib_class);
                        self.launch(
                            d,
                            id,
                            None,
                            torso,
                            (cos * speed, sin * speed),
                            0.0,
                            spin,
                            false,
                        );
                    }
                }
                for k in 0..table.rags as usize {
                    let (sin, cos) = d.rng.range(40.0, 140.0).to_radians().sin_cos();
                    let speed = d.rng.range(80.0, 160.0);
                    let rag = table.rag_ids[k % table.rag_ids.len()];
                    let rag_class = art
                        .part_index(rag)
                        .map_or(SpinClass::Slow, |i| art.parts[i].spin);
                    let spin = Self::class_spin(&mut d.rng, rag_class);
                    self.launch(
                        d,
                        rag,
                        None,
                        torso,
                        (cos * speed, sin * speed),
                        0.0,
                        spin,
                        false,
                    );
                }
                for _ in 0..table.drops {
                    let elevation = d.rng.range(10.0, 170.0);
                    let speed = d.rng.range(120.0, 280.0);
                    self.drop_blood(d, torso, 1.0, elevation, speed);
                }
                // LLAMA-02 section 8: the gland bursts; its lava leaves the
                // throat in a fan (each landing becomes `decal_molten`, at most
                // six per death).
                for _ in 0..table.molten_drops {
                    let elevation = d.rng.range(30.0, 150.0);
                    let speed = d.rng.range(140.0, 260.0);
                    let (sin, cos) = elevation.to_radians().sin_cos();
                    self.launch(
                        d,
                        "drop_molten",
                        None,
                        throat,
                        (cos * speed, sin * speed),
                        0.0,
                        0.0,
                        false,
                    );
                }
            }
            1 => {
                self.spawn_floor_decal(d.variant, "decal_splat_m", ix, d.ground_y, false);
            }
            2 => {
                // Both wall spatters, one fanning each way from I (preset 4,
                // back wall in reach).
                let y = d.ground_y + 14.0;
                self.wall_spatter_variant(d.variant, "decal_wall_spatter", true, (ix - 12.0, y));
                self.wall_spatter_variant(d.variant, "decal_wall_spatter_b", false, (ix + 12.0, y));
                // Preset 4 punch-up: two more at the larger size, higher and
                // wider, so the splash reads as one big mark.
                if table.extra_wall_spatters >= 1 {
                    let s = table.pool_scale;
                    self.wall_spatter_scaled(
                        d.variant,
                        "decal_wall_spatter_b",
                        true,
                        (ix - 26.0, y + 8.0),
                        s,
                    );
                }
                if table.extra_wall_spatters >= 2 {
                    let s = table.pool_scale;
                    self.wall_spatter_scaled(
                        d.variant,
                        "decal_wall_spatter",
                        false,
                        (ix + 26.0, y + 8.0),
                        s,
                    );
                }
            }
            _ => {}
        }
        // Pools grow from the first rest of each tracked torso piece.
        for k in 0..usize::from(table.pools) {
            let Some(rest) = d.rest_ticks[k] else {
                continue;
            };
            for (after, id) in EXPLOSION_POOL_TICKS {
                if self.clock == rest + after {
                    self.pool_stage(d, k, id, d.rest_x[k]);
                }
            }
        }
    }

    /// The ordinary kill: the GOB-01 frames stay; blood is added. A routine
    /// jab is modest, the kick's launch is heavier (presenter tuning, owner
    /// game-feel review pending).
    fn script_knockback(&mut self, d: &mut Death, age: u64, obs: &Observation<'_>) {
        let heavy = d.force >= ForceTier::Heavy;
        // CF02-D: the goblin slammed finish shares this script; its gore
        // baseline is Q43's per pose (the budget), else the legacy one.
        let slam = match d.finish {
            Finish::GoblinSlammed(pose) => Some(pose),
            _ => None,
        };
        d.done_after = 120;
        if age == 0 {
            let (count, lo, hi) = if heavy {
                (24, 100.0, 220.0)
            } else {
                (12, 70.0, 150.0)
            };
            let n = scale_count(count, self.preset.drop_quarters());
            let head_height = if d.variant == LLAMA_VARIANT {
                36.0
            } else {
                26.0
            };
            let head = (d.foot_x, d.ground_y + head_height);
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
                .filter(|id| self.part_allowed(d.variant, id))
                .collect();
            let quarters = match (slam, self.gore_budget) {
                (Some(pose), Some(budget)) => budget.slam_quarters(self.preset, pose),
                _ => self.gib_quarters_for(d.force),
            };
            // Q22/Q43 (cf02): 6 quarters can ask for more gibs than the set
            // has, so the set repeats; below that this is the legacy take.
            let keep = scale_count(allowed.len() as u32, quarters) as usize;
            for id in allowed.into_iter().cycle().take(keep) {
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
            if let (Some(pose), Some(budget)) = (slam, self.gore_budget) {
                // Q43: a wall crumple separates its arm on the wall side now; a
                // flung body separates it on landing (below); a stomped one never.
                if pose == SlamPose::WallCrumple {
                    self.separate_arm(d, budget.slam_parts(self.preset, pose), true);
                }
            } else if !heavy && let Some(budget) = self.gore_budget {
                // Q43: a routine blunt kill adds one transient spray at preset 4
                // (not a decal; gone after its lifetime).
                for _ in 0..budget.routine_sprays(self.preset) {
                    self.transient_spray(d, head, budget.spray_ticks());
                }
            }
            // The kick's blow throws a spray up the back wall (full only).
            if heavy {
                let flip = d.away < 0.0;
                self.wall_spatter_variant(
                    d.variant,
                    "decal_wall_spatter",
                    flip,
                    (d.foot_x, d.ground_y + 14.0),
                );
                self.wall_spatter_variant(
                    d.variant,
                    "decal_wall_spatter_b",
                    d.rng.coin(),
                    (d.foot_x + d.away * 10.0, d.ground_y + 16.0),
                );
            }
        }
        // The goblin has landed (lying down): stain the floor under it.
        if d.landed.is_none()
            && age > 0
            && let Some(x) = Self::lying_x(obs, d.target)
        {
            d.landed = Some(age);
            d.foot_x = x;
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
                // Q43: a flung body separates its arm as it lands.
                if let (Some(SlamPose::Flung), Some(budget)) = (slam, self.gore_budget) {
                    self.separate_arm(d, budget.slam_parts(self.preset, SlamPose::Flung), false);
                }
                if self.spawn_floor_decal(d.variant, "decal_splat_s", x, d.ground_y, false) {
                    d.part_stains += 1;
                }
                self.spawn_floor_decal(d.variant, "decal_splat_m", x, d.ground_y, false);
                if heavy {
                    // The slide leaves a smear (full only), toward the way
                    // the body travelled.
                    self.spawn_floor_decal(d.variant, "decal_smear", x, d.ground_y, d.away < 0.0);
                }
            }
            for (tick, id) in [
                (stage1, "decal_pool_1"),
                (stage2, "decal_pool_2"),
                (stage3, "decal_pool_3"),
            ] {
                if since == tick {
                    self.pool_stage(d, 0, id, x);
                }
            }
        }
        // ENG-02: the Engineer's pot and wrench, after every other draw of this tick.
        self.engineer_slam_launch(d, age);
    }

    /// ENG-02 (spec section 7): a slammed Engineer's pot pops off and his wrench leaves the belt, as two parts
    /// from the launch points of `death_poses.spawner_hints.slam` (measured on the slammed impact cel). Stomped
    /// and wall crumple launch at age 0, where the slammed cel (which wears no pot) replaces the sprite on the same
    /// tick; the flung body launches on the first tick it is not airborne (the landing), because the airborne
    /// base sprite is ENG-01 frame 14, which still wears the pot: at most one pot is visible on every tick. Once
    /// per death; the draws follow `script_knockback`'s on that tick.
    fn engineer_slam_launch(&mut self, d: &mut Death, age: u64) {
        let Finish::GoblinSlammed(pose) = d.finish else {
            return;
        };
        let ready = match pose {
            SlamPose::Flung => d.landed.is_some(),
            SlamPose::Stomped | SlamPose::WallCrumple => age == 0,
        };
        if d.variant != ENGINEER_VARIANT || d.slam_launched || !ready {
            return;
        }
        let key = match pose {
            SlamPose::Stomped => "stomped",
            SlamPose::WallCrumple => "wall",
            SlamPose::Flung => "flung",
        };
        let art = self.art_of(d.variant);
        let Some(hint) = art.slam_launch.get(key).cloned() else {
            return;
        };
        let pivot = art.pose_pivot;
        d.slam_launched = true;
        let world =
            |p: [i32; 2]| pose_anchor_world(pivot, (p[0], p[1]), d.foot_x, d.ground_y, d.flip);
        let (pot_at, wrench_at) = (world(hint.pot_launch), world(hint.wrench_launch));
        let throw = |rng: &mut PresentationRng, away: f32, t: &PartThrow| {
            let vx = away * rng.range(t.vx_away[0], t.vx_away[1]);
            let vy = rng.range(t.vy_up[0], t.vy_up[1]);
            let spin = if rng.coin() { t.spin_dps } else { -t.spin_dps };
            ((vx, vy), spin)
        };
        let (vel, spin) = throw(&mut d.rng, d.away, &hint.pot);
        self.launch(d, "pot", None, pot_at, vel, 0.0, spin, d.flip);
        let (vel, spin) = throw(&mut d.rng, d.away, &hint.wrench);
        self.launch(d, "wrench", Some("grip"), wrench_at, vel, 0.0, spin, d.flip);
    }

    /// Q43: separate up to `count` arms from the body (the manifest's preset
    /// filter still decides whether one flies). `toward_wall` throws it away
    /// from the blow, toward the wall the body met; otherwise it drops from the
    /// body where it landed.
    fn separate_arm(&mut self, d: &mut Death, count: u32, toward_wall: bool) {
        for id in ["arm_near", "arm_far"].into_iter().take(count as usize) {
            let from = (d.foot_x, d.ground_y + 16.0);
            let (dir, speed) = if toward_wall {
                (d.away, d.rng.range(50.0, 110.0))
            } else {
                (d.away, d.rng.range(20.0, 60.0))
            };
            let (sin, cos) = d.rng.range(40.0, 75.0).to_radians().sin_cos();
            let spin = if d.rng.coin() { 540.0 } else { -540.0 };
            self.launch(
                d,
                id,
                Some("shoulder"),
                from,
                (dir * cos * speed, sin * speed),
                0.0,
                spin,
                d.flip,
            );
        }
    }

    /// Q43: one transient blood spray from `at` toward the way the body is
    /// thrown: a short fan of drops that leave no decal and are gone after
    /// `ticks` presenter ticks.
    fn transient_spray(&mut self, d: &mut Death, at: (f32, f32), ticks: u64) {
        self.transient_until = Some(self.clock + ticks);
        for _ in 0..SPRAY_DROPS {
            let elevation = d.rng.range(25.0, 65.0);
            let speed = d.rng.range(90.0, 170.0);
            self.drop_blood(d, at, d.away, elevation, speed);
        }
        self.transient_until = None;
    }

    /// The column of target `target` once it lies down (a goblin or a llama in
    /// `Down` or `Corpse`), else `None`.
    fn lying_x(obs: &Observation<'_>, target: u32) -> Option<f32> {
        let goblins = obs.goblins.len();
        if (target as usize) < goblins {
            let g = &obs.goblins[target as usize];
            matches!(g.mode, GoblinMode::Down | GoblinMode::Corpse).then_some(g.x)
        } else {
            let l = obs.llamas.get(target as usize - goblins)?;
            matches!(l.mode, LlamaMode::Down | LlamaMode::Corpse).then_some(l.x)
        }
    }

    /// LLAMA-02 section 8, the crush burst on the landing tick: 10 to 14 molten
    /// drops from the gland (lava is shown at every preset; half as many at
    /// presets 1 and 2) and a tooth from the mouth.
    fn llama_crush_burst(&mut self, d: &mut Death) {
        let gland = self.pose_point(d, 0, "gland");
        let quarters = if self.preset >= GorePreset::Medium {
            4
        } else {
            2
        };
        let n = scale_count(d.rng.int(10, 14), quarters);
        for _ in 0..n {
            let spread = d.rng.range(-15.0, 15.0).to_radians();
            let up = d.rng.range(220.0, 320.0);
            self.launch(
                d,
                "drop_molten",
                None,
                gland,
                (spread.tan() * up, up),
                0.0,
                0.0,
                false,
            );
        }
        let mouth = self.pose_point(d, 0, "mouth");
        let (vx, vy) = (d.away * d.rng.range(60.0, 120.0), d.rng.range(180.0, 240.0));
        let spin = -d.away * 900.0;
        self.launch(d, "tooth", None, mouth, (vx, vy), 0.0, spin, false);
    }

    /// Slam (LLAMA-02 spec section 8): a lethal heavy hit at tick S. Pose 6 for
    /// 6 ticks, then pose 7 held. On S: a tooth from the mouth, 8 to 12 blood
    /// drops rising from the mouth, one molten drop from the gland, three wool
    /// tufts from the hit point; pool stages 1 and 2 at S+8 and S+30 (no stage
    /// 3: the slam is less bloody than the crush, by design). Counts scale with
    /// the preset; the lava and the tufts stay at every preset.
    fn script_slam(&mut self, d: &mut Death, age: u64) {
        d.done_after = 70;
        if age == 0 {
            let mouth = self.pose_point(d, 6, "mouth");
            let (vx, vy) = (d.away * d.rng.range(60.0, 100.0), d.rng.range(200.0, 260.0));
            let spin = if d.rng.coin() { 900.0 } else { -900.0 };
            self.launch(d, "tooth", None, mouth, (vx, vy), 0.0, spin, false);
            let n = scale_count(d.rng.int(8, 12), self.preset.drop_quarters());
            for _ in 0..n {
                let elevation = d.rng.range(55.0, 85.0);
                let speed = d.rng.range(120.0, 200.0);
                self.drop_blood(d, mouth, d.away, elevation, speed);
            }
            let gland = self.pose_point(d, 6, "gland");
            let (vx, vy) = (d.away * d.rng.range(20.0, 60.0), d.rng.range(180.0, 240.0));
            self.launch(d, "drop_molten", None, gland, (vx, vy), 0.0, 0.0, false);
            let hit = self.pose_point(d, 6, "hit_origin");
            for id in ["wool_tuft_a", "wool_tuft_b", "wool_tuft_a"] {
                let (vx, vy) = (d.away * d.rng.range(-30.0, 90.0), d.rng.range(90.0, 170.0));
                let spin = d.rng.range(-300.0, 300.0);
                self.launch(d, id, None, hit, (vx, vy), 0.0, spin, false);
            }
        }
        let pool = self.pose_point(d, 7, "pool_origin");
        for (tick, id) in [(8, "decal_pool_1"), (30, "decal_pool_2")] {
            if age == tick {
                self.pool_stage(d, 0, id, pool.0);
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
            if p.variant != 0 {
                mix(0x5641_5231 + p.variant as u64);
            }
        }
        for pool in [&self.floor_decals, &self.wall_decals] {
            for (slot, d) in pool.iter() {
                mix(u64::from(d.x.to_bits()));
                mix(u64::from(d.y.to_bits()));
                mix((slot as u64) << 8 | d.meta as u64 | u64::from(d.flip) << 40);
                mix(d.pair_meta.map_or(0, |p| p as u64 + 1));
                if d.variant != 0 {
                    mix(0x5641_5231 + d.variant as u64);
                }
                if d.scale != 1.0 {
                    mix(u64::from(d.scale.to_bits()));
                }
            }
        }
        for pose in self.poses() {
            mix(u64::from(pose.target) << 8 | pose.frame as u64);
            if pose.variant != 0 {
                mix(0x5641_5231 + pose.variant as u64);
            }
        }
        // CF02-D: mixed only when a slammed finish exists (cf02 overlay only).
        for slam in self.slammed_poses() {
            mix(0x534c_414d ^ u64::from(slam.target) << 8 ^ slam.pose as u64);
        }
        for (_, p) in self.parts.iter() {
            if let Some(until) = p.transient_until {
                mix(0x5350_5259 ^ until);
            }
        }
        hash
    }

    /// One summary line for the headless trace.
    pub fn state_line(&self) -> String {
        format!(
            "DEATH_PRESENT preset={} clock={} deaths={} parts={} floor_decals={} wall_decals={} spawned_parts={} peak_parts={} peak_floor_decals={} peak_wall_decals={} digest={:016x}",
            self.preset.level(),
            self.clock,
            self.deaths.len(),
            self.parts.live(),
            self.floor_decals.live(),
            self.wall_decals.live(),
            self.stats.parts_spawned,
            self.stats.peak_parts,
            self.stats.peak_floor_decals,
            self.stats.peak_wall_decals,
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
    use crate::shore_art::ShoreSlot;
    use std::path::{Path, PathBuf};

    const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";
    const SHORE_KILL_HASH: u64 = 0x098e_86cd_e2e6_b065;
    const KICK_BLOCK_HASH: u64 = 0x73e4_0916_1ce5_434f;
    const KICK_HASH: u64 = 0x9e13_d646_1914_9b71;
    const PILEUP_HASH: u64 = 0x6a82_811d_8f31_c475;
    const BLOCK_EDGE_DECAP_HASH: u64 = 0x13d9_8f79_0282_17e9;
    const BLAST_BREAKUP_HASH: u64 = 0xc3d2_b370_3f49_5284;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn art() -> Arc<DeathArt> {
        Arc::new(DeathArt::load(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap())
    }

    fn art_b() -> DeathArt {
        DeathArt::load_variant(Path::new(env!("CARGO_MANIFEST_DIR")), 1).unwrap()
    }

    /// The presenter the game builds: variant A and variant B death art.
    fn presenter(preset: GorePreset) -> DeathPresenter {
        DeathPresenter::with_variants(art(), &art_b(), preset).unwrap()
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
    /// DEATH-D: the block's edge across the neck (the only decapitation cause).
    const EDGE: DefeatCause = DefeatCause::ShoreBlockEdge { player: 0 };

    fn obs(goblins: &[GoblinView]) -> Observation<'_> {
        Observation {
            goblins,
            llamas: &[],
            hero_x: 400.0,
            block_x: Some(484.0),
            back_wall: true,
            charges: &[],
            env_origins: &[],
        }
    }

    /// BOIL-E (the #212 follow-up): a body the env defeated is flung away from the
    /// prop's volume, not from the hero. The goblin stands at 600 and the hero at
    /// 400: with no origin the old rule throws it right (away from the hero), with
    /// the weight's footprint centre at 816 it goes left, away from the weight, and
    /// the lookup is keyed on the prop and the class, so another prop's origin is
    /// not used.
    #[test]
    fn boil_e_an_env_death_is_flung_from_the_prop_not_the_hero() {
        use crate::env_hazard::HazardClass::{FallingMass, Steam};
        let cause = DefeatCause::Environment {
            player: Some(0),
            prop: 1,
            class: FallingMass,
        };
        let away = |origins: &[(u16, crate::env_hazard::HazardClass, f32)]| {
            let mut p = presenter(GorePreset::Full);
            let goblins = [goblin_at(600.0)];
            let ob = Observation {
                env_origins: origins,
                ..obs(&goblins)
            };
            p.step(&[defeat(1, 0, cause, ForceTier::Heavy)], ob);
            p.deaths.first().map(|d| d.away)
        };
        assert_eq!(away(&[]), Some(1.0), "no origin: from the hero");
        assert_eq!(away(&[(1, FallingMass, 816.0)]), Some(-1.0));
        assert_eq!(
            away(&[(1, FallingMass, 500.0)]),
            Some(1.0),
            "the other side"
        );
        assert_eq!(away(&[(0, FallingMass, 816.0)]), Some(1.0), "another prop");
        assert_eq!(away(&[(1, Steam, 816.0)]), Some(1.0), "another class");
    }

    /// Run a crush for `ticks` ticks; returns the presenter.
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
        let goblins = [goblin_at(484.0)];
        p.step(
            &[defeat(139, 0, EDGE, ForceTier::Catastrophic)],
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
        let a = pool.spawn(10, 0);
        let b = pool.spawn(11, 0);
        let c = pool.spawn(12, 0);
        assert_eq!((pool.live(), pool.capacity()), (3, 3));
        assert_eq!(pool.evicted(), 0);
        // Full: the next spawn takes the slot of the oldest entry.
        let d = pool.spawn(13, 0);
        assert_eq!(d.0, a.0);
        assert_eq!((pool.live(), pool.capacity(), pool.evicted()), (3, 3, 1));
        assert!(
            pool.get_seq_mut(a.0, a.1).is_none(),
            "the old entry is gone"
        );
        assert_eq!(pool.get(b.0), Some(&11));
        let e = pool.spawn(14, 0);
        assert_eq!(e.0, b.0, "then the next oldest");
        assert_eq!(pool.get(c.0), Some(&12));
        // A freed slot is reused before anything is recycled.
        pool.free(c.0);
        let f = pool.spawn(15, 0);
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
        p.pool_stage(&mut d, 0, "decal_pool_3", 200.0);
        assert_eq!(p.live_floor_decals(), 1, "the pair is a single entry");
        let (_, decal) = p.floor_decals().next().unwrap();
        assert_eq!((decal.meta, decal.pair_meta), (left, Some(right)));
        // Fill the pool with pools of the same class (kept): the pair goes as
        // one entry when it is the oldest.
        for i in 0..FLOOR_DECAL_POOL_CAPACITY {
            p.spawn_floor_decal(0, "decal_pool_1", i as f32, 100.0, false);
        }
        assert_eq!(p.live_floor_decals(), FLOOR_DECAL_POOL_CAPACITY);
        // The pair is recycled whole: both cells fade together as one entry
        // (the pair's single slot) and leave together when the fade ends.
        let pair_slots: Vec<usize> = p
            .floor_decals()
            .filter(|(_, d)| d.pair_meta.is_some())
            .map(|(slot, _)| slot)
            .collect();
        assert_eq!(pair_slots.len(), 1);
        assert!(p.floor_decals.fade_since(pair_slots[0]).is_some());
        for _ in 0..DEATH_BUDGET.recycle_fade_ticks {
            p.step(&[], obs(&[]));
        }
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
            Death::new(
                99,
                SYNTHETIC_TARGET,
                0,
                Finish::Crushed,
                ForceTier::Catastrophic,
                0,
                (200.0, 100.0),
                false,
                1.0,
                PresentationRng::new(1),
            )
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
        let goblins = [goblin_at(484.0)];
        p.step(
            &[defeat(139, 0, EDGE, ForceTier::Catastrophic)],
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
        let goblins = [goblin_at(484.0)];
        p.step(
            &[defeat(139, 0, EDGE, ForceTier::Catastrophic)],
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
            let goblins = [goblin_at(484.0), goblin_at(520.0)];
            p.step(
                &[
                    defeat(1, 0, EDGE, ForceTier::Catastrophic),
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
        // DEATH-D: the charge and the blast burst sit between the floor decals
        // (-0.5) and the debris (-0.4), at actor_z - 0.45, under every actor.
        for slot in [0, 1, 7] {
            let blast = draw_z(DrawLayer::Blast, slot, actor_z, wall_z);
            assert!(blast < lowest_actor, "blast {blast} over an actor");
            let top_floor = draw_z(
                DrawLayer::FloorDecal,
                FLOOR_DECAL_POOL_SLOTS,
                actor_z,
                wall_z,
            );
            let low_debris = draw_z(DrawLayer::Debris, 0, actor_z, wall_z);
            assert!(blast > top_floor, "over every floor decal");
            assert!(blast < low_debris, "under every piece of debris");
            assert!((blast - (actor_z - 0.45)).abs() < 0.01);
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
            (&crate::combat_scene::CP03_GOBLIN_PILEUP, PILEUP_HASH),
            (
                &crate::combat_scene::CP03_BLOCK_EDGE_DECAP,
                BLOCK_EDGE_DECAP_HASH,
            ),
            (&crate::combat_scene::QA_BLAST_BREAKUP, BLAST_BREAKUP_HASH),
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

    /// FA-Q23 (HRD sheet section 10): the Hoarder's species is in the cf02 overlay, every cause
    /// (the blast included) and force selects `pile_boss_down` (never a goblin finish), and the
    /// generic presenter draws nothing for it: no part, no gib, no blood decal, no pose. Her
    /// own presenter draws the HOARD-01 `down` frames.
    #[test]
    fn the_pile_boss_species_selects_its_down_finish_and_draws_no_gore() {
        let profiles = DeathProfiles::cf02();
        assert!(profiles.species("pile_boss").is_some());
        assert!(DeathProfiles::builtin().species("pile_boss").is_none());
        let causes = [
            DefeatCause::Melee,
            BLOCK,
            EDGE,
            DefeatCause::Explosion {
                charge: 0,
                player: Some(0),
                point_blank: true,
            },
            DefeatCause::Explosion {
                charge: 0,
                player: None,
                point_blank: false,
            },
        ];
        for cause in causes {
            for force in [
                ForceTier::Routine,
                ForceTier::Heavy,
                ForceTier::Catastrophic,
            ] {
                let picked = profiles.select("pile_boss", cause, force);
                assert_eq!(picked.finish, "pile_boss_down", "{cause:?} {force:?}");
            }
        }
        assert_eq!(Finish::from_id("pile_boss_down"), Finish::NoArt);
        for (cause, force) in [
            (BLOCK, ForceTier::Catastrophic),
            (DefeatCause::Melee, ForceTier::Routine),
            (causes[3], ForceTier::Catastrophic),
        ] {
            let mut q = presenter(GorePreset::Full);
            q.profiles = DeathProfiles::cf02();
            q.species = "pile_boss";
            let goblins = [goblin_at(484.0)];
            q.step(&[defeat(1, 0, cause, force)], obs(&goblins));
            for _ in 0..30 {
                q.step(&[], obs(&goblins));
            }
            assert_eq!(
                q.live_parts() + q.live_floor_decals() + q.live_wall_decals(),
                0,
                "{cause:?}"
            );
            assert!(q.pose_of(0).is_none());
        }
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
        for _ in 0..200 {
            p.step(&[], obs(&goblins));
        }
        let splats = count(&floor_ids(&p), "decal_splat_s");
        for _ in 0..(SHORT_TTL_TICKS + SHORT_FADE_TICKS + 3000) {
            p.step(&[], obs(&goblins));
        }
        // Gibs (short) are gone after their time to live; the club and the
        // pools (keep) stay, and so do the stains (no time to live: they leave
        // only when a pool recycles them).
        assert_eq!(count(&part_ids(&p), "gib_flesh_a"), 0);
        assert_eq!(count(&part_ids(&p), "club"), 1);
        assert!(splats > 0);
        assert_eq!(
            count(&floor_ids(&p), "decal_splat_s"),
            splats,
            "stains have no time to live"
        );
        assert_eq!(count(&floor_ids(&p), "decal_pool_3"), 1);
        assert_eq!(fade_alpha(Ttl::Long, 0, 1_000_000), 1.0);
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
                llamas: &[],
                hero_x: 400.0,
                block_x: Some(484.0),
                back_wall: false,
                charges: &[],
                env_origins: &[],
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
        decap.step(
            &[defeat(139, 0, EDGE, ForceTier::Catastrophic)],
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

    // ------------------------------------------------- DEATH-E1: budgets ----

    #[test]
    fn the_budget_is_the_single_source_of_every_cap() {
        let b = DEATH_BUDGET;
        assert_eq!(
            (
                PART_POOL_CAPACITY,
                FLOOR_DECAL_POOL_CAPACITY,
                WALL_DECAL_POOL_CAPACITY
            ),
            (b.parts, b.floor_decals, b.wall_decals)
        );
        assert_eq!(MAX_DEATHS, b.deaths);
        assert_eq!(
            (MAX_DROP_SPLATS, MAX_PART_STAINS),
            (b.drop_splats_per_death, b.part_stains_per_death)
        );
        assert_eq!(
            (SHORT_TTL_TICKS, SHORT_FADE_TICKS),
            (b.short_ttl_ticks, b.short_fade_ticks)
        );
        // The pools of the first measured run were 160 / 96 / 12: a budget
        // change may only raise them (the owner asked for more, not less).
        assert!(b.parts >= 160 && b.floor_decals >= 96 && b.wall_decals >= 12);
        // Every fade exists, so nothing pops: short pieces, recycled entries
        // and retiring corpses all fade out (owner: no pop).
        assert!(b.short_fade_ticks > 0 && b.recycle_fade_ticks > 0 && b.corpse_fade_ticks > 0);
        // The corpse limit (owner decision) and the sprite slots of each pool:
        // the budget plus the slots of the entries fading out.
        assert_eq!((MAX_SETTLED_CORPSES, b.settled_corpses), (8, 8));
        assert_eq!(PART_POOL_SLOTS, b.parts + b.part_fade_slots);
        assert_eq!(
            FLOOR_DECAL_POOL_SLOTS,
            b.floor_decals + b.floor_decal_fade_slots
        );
        assert_eq!(
            WALL_DECAL_POOL_SLOTS,
            b.wall_decals + b.wall_decal_fade_slots
        );
        assert!(
            b.part_fade_slots > 0 && b.floor_decal_fade_slots > 0 && b.wall_decal_fade_slots > 0
        );
        // Every real defeat of the largest fixture keeps its script.
        let most = crate::combat_scene::FIXTURES
            .iter()
            .map(|f| f.enemies.len())
            .max()
            .unwrap();
        assert!(b.deaths >= most, "{} deaths for {most} goblins", b.deaths);
        // The z bias of the last slot of any pool stays far under the gap
        // between two draw layers (a piece never crosses a layer, so never
        // draws over Carl or Donut).
        let last = PART_POOL_SLOTS
            .max(FLOOR_DECAL_POOL_SLOTS)
            .max(WALL_DECAL_POOL_SLOTS) as f32
            * SLOT_Z_STEP;
        assert!(
            last < (FLOOR_DECAL_BELOW_ACTORS - DEBRIS_BELOW_ACTORS) / 2.0,
            "{last}"
        );
    }

    #[test]
    fn the_pools_never_pass_their_caps_and_recycle_short_pieces_first() {
        let mut p = presenter(GorePreset::Full);
        let mut d = p.deaths_for_test();
        // One kept piece (the club), then far more blood drops than the pool
        // holds: the club is never the one recycled.
        p.launch(
            &mut d,
            "club",
            None,
            (10.0, 200.0),
            (0.0, 0.0),
            0.0,
            0.0,
            false,
        );
        for i in 0..PART_POOL_CAPACITY * 3 {
            p.launch(
                &mut d,
                "drop_blood",
                None,
                (i as f32, 200.0),
                (0.0, 0.0),
                0.0,
                0.0,
                false,
            );
            assert!(p.live_parts() <= PART_POOL_CAPACITY);
        }
        assert_eq!(p.live_parts(), PART_POOL_CAPACITY);
        assert!(p.recycled().0 >= (PART_POOL_CAPACITY * 2) as u64);
        assert_eq!(count(&part_ids(&p), "club"), 1, "the kept club survives");
        // Floor decals: a pool stage (kept) survives any number of splats.
        let mut d = p.deaths_for_test();
        p.pool_stage(&mut d, 0, "decal_pool_3", 200.0);
        for i in 0..FLOOR_DECAL_POOL_CAPACITY * 3 {
            p.spawn_floor_decal(0, "decal_splat_s", i as f32, 100.0, false);
            assert!(p.live_floor_decals() <= FLOOR_DECAL_POOL_CAPACITY);
        }
        assert_eq!(p.live_floor_decals(), FLOOR_DECAL_POOL_CAPACITY);
        assert_eq!(count(&floor_ids(&p), "decal_pool_3"), 1);
        assert!(p.recycled().1 >= (FLOOR_DECAL_POOL_CAPACITY * 2) as u64);
        // Wall spatters are all kept: the oldest goes first, the cap holds.
        for i in 0..WALL_DECAL_POOL_CAPACITY * 3 {
            p.wall_spatter_variant(0, "decal_wall_spatter", false, (i as f32, 120.0));
            assert!(p.live_wall_decals() <= WALL_DECAL_POOL_CAPACITY);
        }
        assert_eq!(p.live_wall_decals(), WALL_DECAL_POOL_CAPACITY);
        assert_eq!(p.recycled().2, (WALL_DECAL_POOL_CAPACITY * 2) as u64);
    }

    #[test]
    fn a_full_pool_recycles_the_lowest_rank_then_the_oldest() {
        let mut pool: Pool<(u8, u32)> = Pool::new(3);
        let rank = |t: &(u8, u32)| t.0;
        let (keep, _) = pool.spawn_ranked((2, 0), 0, rank);
        let (short_old, _) = pool.spawn_ranked((0, 1), 0, rank);
        let (short_new, _) = pool.spawn_ranked((0, 2), 0, rank);
        // Full: the oldest of the lowest rank goes, whatever the free order.
        assert_eq!(pool.spawn_ranked((1, 3), 0, rank).0, short_old);
        assert_eq!(pool.spawn_ranked((1, 4), 0, rank).0, short_new);
        // Now ranks are 2, 1, 1: the older rank-1 entry goes, then the next.
        assert_eq!(pool.spawn_ranked((1, 5), 0, rank).0, short_old);
        assert_eq!(pool.get(keep), Some(&(2, 0)));
        assert_eq!(pool.evicted(), 3);
        assert!(ttl_rank(Ttl::Short) < ttl_rank(Ttl::Long));
        assert!(ttl_rank(Ttl::Long) < ttl_rank(Ttl::Keep));
    }

    #[test]
    fn the_pileup_stays_inside_the_budget_at_every_preset() {
        for preset in GorePreset::ALL {
            let (_, events, p) = replay(&crate::combat_scene::CP03_GOBLIN_PILEUP, preset, None);
            let defeats = events
                .iter()
                .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
                .count();
            assert_eq!(defeats, 10, "ten goblins fall in the pileup");
            // No death was refused: every one of the ten has its script.
            assert_eq!(
                p.deaths
                    .iter()
                    .filter(|d| d.target != SYNTHETIC_TARGET)
                    .count(),
                10,
                "preset {}",
                preset.level()
            );
            // The four crushed goblins (6 to 9) hold their crush pose;
            // knockback finishes (0 to 5) have none (the GOB-01 frames stay).
            for target in 0..10 {
                assert_eq!(
                    p.pose_of(target).is_some(),
                    target >= 6,
                    "goblin {target} at preset {}: {:?}",
                    preset.level(),
                    p.pose_of(target)
                );
            }
        }
    }

    #[test]
    fn the_pileup_at_preset_four_is_not_starved_by_the_budget() {
        // The recycling counts say how much of what the deaths drew the budget
        // had to take back while it was still wanted; a budget that is too
        // small shows here as pieces recycled before their time to live.
        let (_, _, p) = replay(
            &crate::combat_scene::CP03_GOBLIN_PILEUP,
            GorePreset::Full,
            None,
        );
        let (parts, floor, wall) = p.recycled();
        assert_eq!(parts, 0, "no part recycled early");
        assert_eq!(
            (floor, wall),
            (0, 0),
            "no decal recycled early; stats {:?}",
            p.stats()
        );
    }

    // ------------------------------- DEATH-E1: fades, corpse limit, stains ----

    #[test]
    fn a_recycled_entry_fades_out_in_its_slot_and_the_new_one_takes_a_free_slot() {
        let fade = DEATH_BUDGET.recycle_fade_ticks;
        let mut pool: Pool<u32> = Pool::with_fade_slots(2, 2);
        let a = pool.spawn(1, 0);
        let b = pool.spawn(2, 0);
        // Over the budget: the oldest is recycled, it keeps its slot and fades,
        // the new entry takes a free slot, and only the live ones are counted.
        let c = pool.spawn(3, 10);
        assert_ne!(c.0, a.0);
        assert_eq!((pool.live(), pool.occupied(), pool.evicted()), (2, 3, 1));
        assert_eq!(pool.fade_since(a.0), Some(10));
        assert_eq!(pool.get(a.0), Some(&1), "still drawn while it fades");
        assert!(
            pool.get_seq_mut(a.0, a.1).is_none(),
            "a fading entry is not live"
        );
        assert_eq!(pool.fade_since(b.0), None);
        pool.expire_faded(10 + fade - 1, fade);
        assert_eq!(pool.occupied(), 3);
        pool.expire_faded(10 + fade, fade);
        assert_eq!((pool.live(), pool.occupied()), (2, 2));
        assert_eq!(pool.get(a.0), None);
        // A burst with no free slot takes the oldest fading entry back at once.
        let mut pool: Pool<u32> = Pool::with_fade_slots(1, 1);
        let first = pool.spawn(1, 0);
        let second = pool.spawn(2, 1);
        assert_eq!(pool.occupied(), 2);
        let third = pool.spawn(3, 2);
        assert_eq!(third.0, first.0, "the fading first entry is taken back");
        assert_eq!((pool.live(), pool.occupied()), (1, 2));
        assert!(pool.fade_since(second.0).is_some());
    }

    #[test]
    fn recycled_stains_fade_out_over_about_thirty_ticks_instead_of_popping() {
        let mut p = presenter(GorePreset::Full);
        for i in 0..FLOOR_DECAL_POOL_CAPACITY {
            p.spawn_floor_decal(0, "decal_splat_s", i as f32, 100.0, false);
        }
        assert_eq!(p.recycled().1, 0);
        p.spawn_floor_decal(0, "decal_splat_s", 999.0, 100.0, false);
        assert_eq!(p.recycled().1, 1);
        let (slot, _) = p
            .floor_decals()
            .find(|(s, _)| p.floor_decals.fade_since(*s).is_some())
            .expect("the oldest splat is fading");
        let at = |p: &DeathPresenter| p.floor_decal_alpha(slot, p.floor_decal_at(slot).unwrap());
        assert_eq!(at(&p), 1.0);
        let mut last = 1.0;
        for _ in 0..DEATH_BUDGET.recycle_fade_ticks - 1 {
            p.step(&[], obs(&[]));
            let alpha = at(&p);
            assert!(alpha < last && alpha > 0.0, "{alpha} after {last}");
            last = alpha;
        }
        p.step(&[], obs(&[]));
        assert!(p.floor_decal_at(slot).is_none(), "gone when the fade ends");
        assert_eq!(p.live_floor_decals(), FLOOR_DECAL_POOL_CAPACITY);
    }

    #[test]
    fn stains_and_long_parts_have_no_time_to_live_but_short_pieces_keep_theirs() {
        assert_eq!(fade_alpha(Ttl::Long, 0, 10_000_000), 1.0);
        assert_eq!(fade_alpha(Ttl::Keep, 0, 10_000_000), 1.0);
        assert!(fade_alpha(Ttl::Short, 0, SHORT_TTL_TICKS + SHORT_FADE_TICKS) <= 0.0);
        // Every stain is still there a very long time later; the gibs and
        // drops (short) are not.
        let mut p = run_crush(GorePreset::Full, 1);
        let goblins = [goblin_at(484.0)];
        for _ in 0..30_000 {
            p.step(&[], obs(&goblins));
        }
        assert!(count(&floor_ids(&p), "decal_splat_s") > 0);
        assert_eq!(count(&floor_ids(&p), "decal_pool_3"), 1);
        assert_eq!(count(&part_ids(&p), "gib_flesh_a"), 0);
        assert_eq!(count(&part_ids(&p), "drop_blood"), 0);
    }

    /// Twelve goblins defeated one tick apart: even ones crushed (a pose), odd
    /// ones struck (the GOB-01 corpse stays).
    fn twelve_corpses() -> (DeathPresenter, Vec<GoblinView>) {
        let mut p = presenter(GorePreset::Full);
        let goblins: Vec<GoblinView> = (0..12)
            .map(|i| goblin_at(300.0 + 20.0 * i as f32))
            .collect();
        for t in 0..12u32 {
            let (cause, force) = if t % 2 == 0 {
                (BLOCK, ForceTier::Catastrophic)
            } else {
                (DefeatCause::Melee, ForceTier::Routine)
            };
            p.step(&[defeat(10 + u64::from(t), t, cause, force)], obs(&goblins));
        }
        (p, goblins)
    }

    #[test]
    fn at_most_eight_settled_corpses_stay_visible_and_the_oldest_retire_first() {
        let (mut p, goblins) = twelve_corpses();
        let mut first_fade = [None::<u64>; 12];
        let mut gone = [None::<u64>; 12];
        for _ in 0..400 {
            p.step(&[], obs(&goblins));
            assert!(
                p.visible_settled_corpses() <= MAX_SETTLED_CORPSES,
                "tick {}",
                p.clock()
            );
            for t in 0..12u32 {
                let alpha = p.corpse_alpha(t);
                if alpha < 1.0 && first_fade[t as usize].is_none() {
                    first_fade[t as usize] = Some(p.clock());
                }
                if alpha <= 0.0 && gone[t as usize].is_none() {
                    gone[t as usize] = Some(p.clock());
                }
            }
        }
        // The four oldest retired, in order, and nothing newer did.
        for (t, at) in gone.iter().enumerate().take(4) {
            assert!(at.is_some(), "goblin {t} retired");
            assert!(p.is_retired(t as u32));
        }
        for (t, at) in first_fade.iter().enumerate().skip(4) {
            assert!(at.is_none(), "goblin {t} stays");
            assert_eq!(p.corpse_alpha(t as u32), 1.0);
            assert!(!p.is_retired(t as u32));
        }
        for t in 1..4 {
            assert!(first_fade[t - 1] <= first_fade[t], "oldest first");
            assert!(gone[t - 1] <= gone[t], "oldest first");
        }
        // The fade takes about corpse_fade_ticks.
        let span = gone[0].unwrap() - first_fade[0].unwrap();
        assert!(
            (DEATH_BUDGET.corpse_fade_ticks - 2..=DEATH_BUDGET.corpse_fade_ticks + 2)
                .contains(&span),
            "{span}"
        );
        // Their death scripts and parts are freed; the stains stay.
        assert_eq!(p.deaths.iter().filter(|d| d.target < 4).count(), 0);
        let live_ids: Vec<u32> = p.deaths.iter().map(|d| d.id).collect();
        assert!(p.parts().all(|(_, part)| live_ids.contains(&part.death)));
        assert!(p.live_floor_decals() > 0);
        // A retired goblin has no pose and a faded sprite.
        assert!(p.pose_of(0).is_none() && p.corpse_alpha(1) == 0.0);
    }

    #[test]
    fn a_retiring_crush_pose_fades_while_the_newer_ones_stay_opaque() {
        let (mut p, goblins) = twelve_corpses();
        let mut seen_fading = false;
        for _ in 0..400 {
            p.step(&[], obs(&goblins));
            if let Some(pose) = p.pose_of(0)
                && pose.alpha < 1.0
            {
                seen_fading = true;
                assert!(pose.alpha > 0.0);
                assert_eq!(p.pose_of(10).map(|q| q.alpha), Some(1.0), "newer stays");
            }
        }
        assert!(seen_fading);
        assert!(p.pose_of(0).is_none(), "retired once the fade ends");
    }

    #[test]
    fn a_restart_brings_every_retired_corpse_back() {
        let (mut p, goblins) = twelve_corpses();
        for _ in 0..400 {
            p.step(&[], obs(&goblins));
        }
        assert!(p.is_retired(0));
        p.step(
            &[CombatEvent::Restart { tick: 0, deaths: 0 }],
            obs(&goblins),
        );
        assert!(!p.is_retired(0));
        assert_eq!(p.corpse_alpha(0), 1.0);
        assert_eq!(p.deaths.len(), 0);
    }

    // ------------------------------------------- DEATH-E1: goblin variant B ----

    #[test]
    fn the_variants_share_the_contract_and_a_mismatch_is_refused() {
        let a = art();
        let b = art_b();
        assert_eq!((a.variant, b.variant), (0, 1));
        a.same_contract(&b).expect("B keeps A's contract");
        let mut bad = art_b();
        bad.parts[3].id = "head_dead_renamed".into();
        assert!(a.same_contract(&bad).is_err());
        assert!(DeathPresenter::with_variants(art(), &bad, GorePreset::Full).is_err());
        let mut moved = art_b();
        moved.poses[0].anchors.insert("mouth".into(), [0, 0]);
        assert!(a.same_contract(&moved).is_err());
        assert!(DeathArt::load_variant(&root(), DEATH_VARIANTS).is_err());
    }

    #[test]
    fn the_death_variant_of_a_goblin_is_its_gob01_variant() {
        assert_eq!(DEATH_VARIANTS, crate::combat_anim::GOBLIN_VARIANTS);
        let mut p = presenter(GorePreset::Full);
        let goblins: Vec<GoblinView> = (0..4).map(|i| goblin_at(400.0 + 30.0 * i as f32)).collect();
        let events: Vec<CombatEvent> = (0..4)
            .map(|t| defeat(50, t, DefeatCause::Melee, ForceTier::Routine))
            .collect();
        p.step(&events, obs(&goblins));
        for t in 0..4u32 {
            let want = crate::combat_anim::goblin_variant(t as usize);
            assert_eq!(want, (t % 2) as usize, "even goblins are A, odd are B");
            assert_eq!(
                p.deaths.iter().find(|d| d.target == t).unwrap().variant,
                want
            );
        }
        // A crush plays the pose of the goblin's own variant.
        let mut p = presenter(GorePreset::Full);
        let goblins = [goblin_at(420.0), goblin_at(484.0)];
        p.step(
            &[
                defeat(139, 0, BLOCK, ForceTier::Catastrophic),
                defeat(139, 1, BLOCK, ForceTier::Catastrophic),
            ],
            obs(&goblins),
        );
        assert_eq!(p.pose_of(0).unwrap().variant, 0);
        assert_eq!(p.pose_of(1).unwrap().variant, 1);
        for _ in 0..30 {
            p.step(&[], obs(&goblins));
        }
        // Every piece launched by goblin 1's death carries variant B, goblin
        // 0's carries A, and both sets exist.
        let dead_variant = |id: u32| p.deaths.iter().find(|d| d.target == id).unwrap().variant;
        let mut seen = [0usize; 2];
        for (_, part) in p.parts() {
            let owner = p.deaths.iter().find(|d| d.id == part.death).unwrap();
            assert_eq!(part.variant, owner.variant);
            seen[part.variant] += 1;
        }
        assert_eq!((dead_variant(0), dead_variant(1)), (0, 1));
        assert!(
            seen[0] > 0 && seen[1] > 0,
            "both variants launch pieces: {seen:?}"
        );
    }

    #[test]
    fn variant_b_sheets_add_the_accessory_and_keep_everything_else() {
        let p = presenter(GorePreset::Full);
        let (a, b) = (p.sheets_of(0), p.sheets_of(1));
        assert_eq!(
            (a.poses.frames, a.parts.frames),
            (b.poses.frames, b.parts.frames)
        );
        // The accessory frames differ (head, upper torso, dead head, ear and
        // every pose), the rest of the parts sheet is identical pixel for pixel.
        let differs = |sheet_a: &crate::gore::Sheet, sheet_b: &crate::gore::Sheet, frame: usize| {
            (0..sheet_a.cell_h)
                .flat_map(|y| (0..sheet_a.cell_w).map(move |x| (x, y)))
                .any(|(x, y)| sheet_a.pixel(frame, x, y) != sheet_b.pixel(frame, x, y))
        };
        for frame in 0..a.poses.frames {
            assert!(
                differs(&a.poses, &b.poses, frame),
                "pose {frame} has no accessory"
            );
        }
        let accessory_parts = ["head", "torso_upper", "head_dead", "ear"];
        for (index, meta) in p.art().parts.iter().enumerate() {
            let changed = differs(&a.parts, &b.parts, index);
            assert_eq!(
                changed,
                accessory_parts.contains(&meta.id.as_str()),
                "{} (frame {index})",
                meta.id
            );
        }
        // The accessory is not gore: it is drawn at preset 1 too (ruling
        // 13.1a), while no blood colour is.
        let off = presenter(GorePreset::Off);
        let (a1, b1) = (off.sheets_of(0), off.sheets_of(1));
        let head = off.art().part_index("head").unwrap();
        assert!(
            differs(&a1.parts, &b1.parts, head),
            "preset 1 keeps the accessory"
        );
        // The silhouettes the floor clamp uses come from the variant's sheet.
        for (index, meta) in p.art().parts.iter().enumerate() {
            assert!(
                !b.parts.hull[index].is_empty() || b.parts.opaque[index] == 0,
                "{}",
                meta.id
            );
        }
    }

    #[test]
    fn variant_b_pieces_rest_on_the_floor_like_variant_a() {
        let mut p = presenter(GorePreset::Full);
        let goblins = [goblin_at(420.0), goblin_at(484.0)];
        p.step(
            &[
                defeat(50, 0, DefeatCause::Melee, ForceTier::Heavy),
                defeat(50, 1, DefeatCause::Melee, ForceTier::Heavy),
            ],
            obs(&goblins),
        );
        for _ in 0..420 {
            p.step(&[], obs(&goblins));
        }
        for (_, part) in p.parts() {
            let hull = &p.sheets_of(part.variant).parts.hull[part.frame];
            let low = part.y - lowest_below_centre(hull, part.flip, part.angle_deg);
            assert!(low >= 100.0 - 0.01, "{low} below the floor line");
        }
    }

    // ------------------------------------------------------------ LLAMA-02 ----

    fn llama_art() -> Arc<DeathArt> {
        Arc::new(DeathArt::load_llama(&root()).unwrap())
    }

    /// The presenter the game builds for a run with llamas.
    fn llama_presenter(preset: GorePreset) -> DeathPresenter {
        presenter(preset).with_llama(llama_art()).unwrap()
    }

    /// A llama standing at `x` facing left on the floor at y 100.
    fn llama_at(x: f32) -> LlamaView {
        LlamaView {
            x,
            ground_y: 100.0,
            facing: -1,
            mode: LlamaMode::Down,
        }
    }

    fn lobs<'a>(goblins: &'a [GoblinView], llamas: &'a [LlamaView]) -> Observation<'a> {
        Observation {
            goblins,
            llamas,
            hero_x: 400.0,
            block_x: Some(484.0),
            back_wall: true,
            charges: &[],
            env_origins: &[],
        }
    }

    fn llama_part_ids(p: &DeathPresenter) -> Vec<String> {
        p.parts()
            .map(|(_, part)| p.art_of(part.variant).parts[part.meta].id.clone())
            .collect()
    }

    fn llama_floor_ids(p: &DeathPresenter) -> Vec<String> {
        p.floor_decals()
            .map(|(_, d)| p.art_of(d.variant).parts[d.meta].id.clone())
            .collect()
    }

    /// One llama (target 0, no goblins) defeated by `cause` with `force`.
    fn llama_death(
        preset: GorePreset,
        cause: DefeatCause,
        force: ForceTier,
        ticks: u64,
    ) -> DeathPresenter {
        let mut p = llama_presenter(preset);
        let llamas = [llama_at(484.0)];
        p.step(&[defeat(10, 0, cause, force)], lobs(&[], &llamas));
        for _ in 1..ticks {
            p.step(&[], lobs(&[], &llamas));
        }
        p
    }

    #[test]
    fn a_heavy_lethal_hit_plays_the_slam_pair_and_holds_the_broken_heap() {
        let mut p = llama_presenter(GorePreset::Full);
        let llamas = [llama_at(484.0)];
        p.step(
            &[defeat(10, 0, DefeatCause::Melee, ForceTier::Heavy)],
            lobs(&[], &llamas),
        );
        let first = p.pose_of(0).expect("the slam pose");
        assert_eq!((first.frame, first.variant), (6, LLAMA_VARIANT));
        // Pose 6 (`slam_contact`) for 6 ticks, then pose 7 (`broken_heap`).
        for age in 1..40u64 {
            p.step(&[], lobs(&[], &llamas));
            let frame = p.pose_of(0).unwrap().frame;
            assert_eq!(frame, if age < 6 { 6 } else { 7 }, "age {age}");
        }
        let art = p.art_of(LLAMA_VARIANT);
        assert_eq!(art.pose(6).tag, "slam_contact");
        assert_eq!(art.pose(7).tag, "broken_heap");
        // No hazard: the stone is not drawn over a slam.
        assert!(p.hazard_over_anchor().is_none());
    }

    #[test]
    fn a_routine_lethal_hit_keeps_the_llama_01_down_and_only_adds_blood() {
        let p = llama_death(GorePreset::Full, DefeatCause::Melee, ForceTier::Routine, 30);
        assert!(p.pose_of(0).is_none(), "no pose override: the down frames");
        assert!(p.live_parts() > 0, "blood and a gib fly at preset 4");
        let ids = llama_part_ids(&p);
        assert!(ids.iter().all(|id| id != "tooth" && id != "wool_tuft_a"));
        // The same lethal blow with a kick (Heavy) is the slam.
        let slam = llama_death(GorePreset::Full, DefeatCause::Melee, ForceTier::Heavy, 30);
        assert!(slam.pose_of(0).is_some());
    }

    #[test]
    fn the_slam_throws_its_pieces_by_the_spec_and_scales_with_the_preset() {
        let ids_at = |preset| {
            let p = llama_death(preset, DefeatCause::Melee, ForceTier::Heavy, 4);
            llama_part_ids(&p)
        };
        let full = ids_at(GorePreset::Full);
        let count = |ids: &[String], id: &str| ids.iter().filter(|i| *i == id).count();
        assert_eq!(count(&full, "tooth"), 1);
        assert_eq!(count(&full, "drop_molten"), 1);
        let tufts = count(&full, "wool_tuft_a") + count(&full, "wool_tuft_b");
        assert_eq!(tufts, 3);
        assert!(
            (8..=12).contains(&count(&full, "drop_blood")),
            "8 to 12 blood drops at preset 4"
        );
        // Preset 1: no blood sprite, but the tooth, the lava and the wool stay.
        let off = ids_at(GorePreset::Off);
        assert_eq!(count(&off, "drop_blood"), 0);
        assert_eq!(count(&off, "tooth"), 1);
        assert_eq!(count(&off, "drop_molten"), 1);
        assert_eq!(count(&off, "wool_tuft_a") + count(&off, "wool_tuft_b"), 3);
        // Preset 3 halves the blood drops (ceil(n / 2)).
        let medium = ids_at(GorePreset::Medium);
        assert!((4..=6).contains(&count(&medium, "drop_blood")));
    }

    #[test]
    fn the_slam_pool_grows_to_stage_two_and_never_to_stage_three() {
        let p = llama_death(GorePreset::Full, DefeatCause::Melee, ForceTier::Heavy, 200);
        let ids = llama_floor_ids(&p);
        // One pool decal that grew in place: stage 2 stands, no stage 3.
        assert!(ids.iter().any(|id| id == "decal_pool_2"), "{ids:?}");
        assert!(ids.iter().all(|id| id != "decal_pool_3"), "{ids:?}");
        // Preset 1 draws no blood decal at all, only lava.
        let off = llama_death(GorePreset::Off, DefeatCause::Melee, ForceTier::Heavy, 200);
        let off_ids = llama_floor_ids(&off);
        assert!(
            off_ids.iter().all(|id| id.starts_with("decal_molten")),
            "{off_ids:?}"
        );
    }

    #[test]
    fn the_gland_layer_is_visible_at_every_preset_and_no_gore_pixel_at_preset_one() {
        use crate::gore::Sheet;
        const MOLTEN: [[u8; 4]; 2] = [[0xff, 0xe6, 0x80, 0xff], [0xff, 0x9d, 0x2e, 0xff]];
        const BLOOD: [[u8; 4]; 3] = [
            [0x6e, 0x1a, 0x22, 0xff],
            [0xb8, 0x32, 0x2e, 0xff],
            [0xe2, 0x43, 0x4b, 0xff],
        ];
        let count = |sheet: &Sheet, frame: usize, colours: &[[u8; 4]]| {
            let mut n = 0;
            for y in 0..sheet.cell_h {
                for x in 0..sheet.cell_w {
                    if colours.contains(&sheet.pixel(frame, x, y)) {
                        n += 1;
                    }
                }
            }
            n
        };
        let art = llama_art();
        // Poses: the crush geyser (0), the cut duct (3), the coughed glob (6).
        for frame in [0, 3, 6] {
            let mut lava = Vec::new();
            for preset in GorePreset::ALL {
                let sheets = art.compose(preset).unwrap();
                let n = count(&sheets.poses, frame, &MOLTEN);
                assert!(n > 0, "lava on pose {frame} at preset {}", preset.level());
                lava.push(n);
                if preset == GorePreset::Off {
                    assert_eq!(
                        count(&sheets.poses, frame, &BLOOD),
                        0,
                        "no gore-layer pixel on pose {frame} at preset 1"
                    );
                }
            }
            assert!(
                lava.iter().all(|n| *n == lava[0]),
                "the lava is the same at every preset: {lava:?}"
            );
            let full = art.compose(GorePreset::Full).unwrap();
            assert!(count(&full.poses, frame, &BLOOD) > 0, "blood at preset 4");
        }
        // Parts: the gland piece itself keeps its lava at preset 1.
        let gland = art.part_index("gland").unwrap();
        let off = art.compose(GorePreset::Off).unwrap();
        assert!(count(&off.parts, art.parts[gland].frame, &MOLTEN) > 0);
        assert_eq!(count(&off.parts, art.parts[gland].frame, &BLOOD), 0);
        // The presenter composes the same sheets for the llama variant.
        let p = llama_presenter(GorePreset::Off);
        assert!(count(&p.sheets_of(LLAMA_VARIANT).poses, 6, &MOLTEN) > 0);
    }

    #[test]
    fn the_llama_art_reads_the_hazard_draw_flag_from_the_manifest() {
        let art = llama_art();
        let over: Vec<usize> = art
            .poses
            .iter()
            .filter(|p| p.hazard_over())
            .map(|p| p.index)
            .collect();
        assert_eq!(over, [0, 1], "crush_contact frames 0 and 1 only");
        // The goblin art has no flag: the stone stays behind.
        assert!(art_variant_a().poses.iter().all(|p| !p.hazard_over()));
    }

    fn art_variant_a() -> Arc<DeathArt> {
        art()
    }

    #[test]
    fn the_stone_is_drawn_over_the_crushed_llama_on_the_two_contact_frames_only() {
        for flip in [false, true] {
            let mut p = llama_presenter(GorePreset::Full);
            let mut llama = llama_at(484.0);
            llama.facing = if flip { -1 } else { 1 };
            let llamas = [llama];
            p.step(
                &[defeat(139, 0, BLOCK, ForceTier::Catastrophic)],
                lobs(&[], &llamas),
            );
            // Frame 0: the block's anchor on the (mirrored) hit_origin (42, 52):
            // pivot column 44, pivot row 71, pixel centres. The pose is placed
            // so that this point is the block's centre column (484).
            let (ax, ay) = p.hazard_over_anchor().expect("over on frame 0");
            assert_eq!((ax, ay), (484.0, 100.0 + 19.5), "flip {flip}");
            assert_eq!(p.pose_of(0).unwrap().frame, 0);
            for _ in 1..3 {
                p.step(&[], lobs(&[], &llamas));
            }
            assert!(p.hazard_over_anchor().is_some(), "still frame 0 at age 2");
            // Frame 1: the 3 px sink (hit_origin (42, 55)), a render-only offset.
            p.step(&[], lobs(&[], &llamas));
            let (_, ay) = p.hazard_over_anchor().expect("over on frame 1");
            assert_eq!(p.pose_of(0).unwrap().frame, 1);
            assert_eq!(ay, 100.0 + 16.5);
            for _ in 0..2 {
                p.step(&[], lobs(&[], &llamas));
            }
            assert!(p.hazard_over_anchor().is_some(), "frame 1 at age 5");
            // From `crushed_remains` on the goblin order is back.
            p.step(&[], lobs(&[], &llamas));
            assert_eq!(p.pose_of(0).unwrap().frame, 2);
            assert!(p.hazard_over_anchor().is_none());
        }
        // A crushed goblin never asks for it.
        let mut g = presenter(GorePreset::Full);
        let goblins = [goblin_at(484.0)];
        g.step(
            &[defeat(139, 0, BLOCK, ForceTier::Catastrophic)],
            lobs(&goblins, &[]),
        );
        assert!(g.hazard_over_anchor().is_none());
    }

    #[test]
    fn a_partly_overlapping_llama_is_drawn_under_the_stone_so_the_stone_never_moves() {
        // The llama stands 20 px off the block's centre (partial overlap): the
        // stone's anchor is the block centre on every contact frame (no
        // sideways jump and no snap back), and the pose is shifted to it.
        for (x, flip) in [(504.0, false), (464.0, true), (504.0, true)] {
            let mut p = llama_presenter(GorePreset::Full);
            let mut llama = llama_at(x);
            llama.facing = if flip { -1 } else { 1 };
            let llamas = [llama];
            p.step(
                &[defeat(139, 0, BLOCK, ForceTier::Catastrophic)],
                lobs(&[], &llamas),
            );
            for age in 0..6 {
                let (ax, _) = p.hazard_over_anchor().expect("over");
                assert_eq!(ax, 484.0, "age {age} at x {x} flip {flip}");
                p.step(&[], lobs(&[], &llamas));
            }
            assert!(p.hazard_over_anchor().is_none());
        }
    }

    #[test]
    fn the_slam_pose_follows_the_sliding_body_and_leaves_impact_dust() {
        let mut p = llama_presenter(GorePreset::Full);
        let mut llamas = [llama_at(484.0)];
        p.step(
            &[defeat(10, 0, DefeatCause::Melee, ForceTier::Heavy)],
            lobs(&[], &llamas),
        );
        assert_eq!(p.pose_of(0).unwrap().foot_x, 484.0);
        // The body slides 30 px; the pose and the dust follow it.
        llamas[0].x = 514.0;
        p.step(&[], lobs(&[], &llamas));
        assert_eq!(p.pose_of(0).unwrap().foot_x, 514.0);
        let dust = p.impact_dusts();
        assert_eq!(dust.len(), 1);
        assert_eq!((dust[0].ground_y, dust[0].age), (100.0, 1));
        // hit_origin (42, 68) of pose 6: 1.5 px behind the pivot column when
        // the llama faces left (mirrored).
        assert_eq!(dust[0].x, 514.0 + 1.5);
        // The dust ends after four frames of five ticks; the pose stays.
        for _ in 0..25 {
            p.step(&[], lobs(&[], &llamas));
        }
        assert!(p.impact_dusts().is_empty());
        assert_eq!(p.pose_of(0).unwrap().frame, 7);
        // A routine kill and a crush leave no slam dust.
        let down = llama_death(GorePreset::Full, DefeatCause::Melee, ForceTier::Routine, 3);
        assert!(down.impact_dusts().is_empty());
    }

    #[test]
    fn the_llama_crush_bursts_lava_at_every_preset_and_keeps_the_lava_cap() {
        for preset in GorePreset::ALL {
            let p = llama_death(
                preset,
                DefeatCause::ShoreBlock { player: 0 },
                ForceTier::Catastrophic,
                4,
            );
            let ids = llama_part_ids(&p);
            let lava = ids.iter().filter(|i| *i == "drop_molten").count();
            // 10 to 14 drops, half as many at presets 1 and 2.
            let (lo, hi) = if preset >= GorePreset::Medium {
                (10, 14)
            } else {
                (5, 7)
            };
            assert!(
                (lo..=hi).contains(&lava),
                "{lava} molten drops at preset {}",
                preset.level()
            );
            assert!(ids.iter().any(|i| i == "tooth"));
        }
        // The drops land and each becomes a molten decal, at most 6 per death.
        let p = llama_death(
            GorePreset::Full,
            DefeatCause::ShoreBlock { player: 0 },
            ForceTier::Catastrophic,
            200,
        );
        let molten = llama_floor_ids(&p)
            .iter()
            .filter(|id| id.starts_with("decal_molten"))
            .count();
        assert!(
            (1..=usize::from(MAX_MOLTEN_DECALS)).contains(&molten),
            "{molten}"
        );
    }

    #[test]
    fn a_llama_defeat_draws_nothing_without_the_llama_art() {
        let mut p = presenter(GorePreset::Full);
        let llamas = [llama_at(484.0)];
        p.step(
            &[defeat(10, 0, DefeatCause::Melee, ForceTier::Heavy)],
            lobs(&[], &llamas),
        );
        for _ in 0..30 {
            p.step(&[], lobs(&[], &llamas));
        }
        assert!(p.pose_of(0).is_none());
        assert_eq!(p.live_parts() + p.live_floor_decals(), 0);
    }

    #[test]
    fn llama_corpses_count_toward_the_corpse_cap_with_the_goblins() {
        // Six goblins and six llamas defeated within a few ticks: at most
        // `MAX_SETTLED_CORPSES` settled corpses stay visible, the oldest first.
        let mut p = llama_presenter(GorePreset::Full);
        let goblins: Vec<GoblinView> = (0..6).map(|i| goblin_at(200.0 + 20.0 * i as f32)).collect();
        let llamas: Vec<LlamaView> = (0..6).map(|i| llama_at(500.0 + 40.0 * i as f32)).collect();
        for t in 0..12u32 {
            let (cause, force) = if t % 2 == 0 {
                (DefeatCause::Melee, ForceTier::Heavy)
            } else {
                (DefeatCause::Melee, ForceTier::Routine)
            };
            p.step(
                &[defeat(10 + u64::from(t), t, cause, force)],
                lobs(&goblins, &llamas),
            );
        }
        let mut most = 0;
        for _ in 0..400 {
            p.step(&[], lobs(&goblins, &llamas));
            most = most.max(p.visible_settled_corpses());
        }
        assert!(most <= MAX_SETTLED_CORPSES, "{most} settled corpses");
        // The oldest (a goblin) retired first; the newest llama is still drawn.
        assert!(p.is_retired(0));
        assert!(!p.is_retired(11));
        assert_eq!(p.visible_settled_corpses(), MAX_SETTLED_CORPSES);
    }

    #[test]
    fn the_llama_fixtures_replay_the_same_hash_at_presets_one_and_four() {
        use crate::combat_scene::{
            CP05_LLAMA_CRUSH, CP05_LLAMA_DOWN, CP05_LLAMA_OFFSCREEN, CP05_LLAMA_SLAM,
            CP05_LLAMA_SPIT,
        };
        let run = |fixture: &'static Fixture, preset: GorePreset| {
            let kit = EnvKit::load(&root(), LAYOUT).unwrap();
            let mut sim = FixtureSim::new(&kit, 360, fixture).unwrap();
            let mut p = llama_presenter(preset);
            for _ in 0..fixture.ticks {
                sim.step_scripted();
                p.observe(&sim.combat, &sim.scene);
            }
            (sim.state_hash(), sim.combat.events.clone(), p.digest(), p)
        };
        for fixture in [
            &CP05_LLAMA_SLAM,
            &CP05_LLAMA_DOWN,
            &CP05_LLAMA_SPIT,
            &CP05_LLAMA_OFFSCREEN,
            &CP05_LLAMA_CRUSH,
        ] {
            let mut hashes = Vec::new();
            for preset in GorePreset::ALL {
                let (hash, events, digest, _) = run(fixture, preset);
                hashes.push((hash, events, digest));
            }
            for h in &hashes[1..] {
                assert_eq!(h.0, hashes[0].0, "{}: hash at every preset", fixture.name);
                assert_eq!(h.1, hashes[0].1, "{}: events at every preset", fixture.name);
            }
            // A death changes what is drawn between presets; no death, no change.
            let dies = hashes[0]
                .1
                .iter()
                .any(|e| matches!(e, CombatEvent::EnemyDefeated { .. }));
            assert_eq!(hashes[0].2 != hashes[3].2, dies, "{}", fixture.name);
            // The same replay twice draws the same pieces.
            let (_, _, a, _) = run(fixture, GorePreset::Full);
            let (_, _, b, _) = run(fixture, GorePreset::Full);
            assert_eq!(a, b, "{}", fixture.name);
        }
        // The slam fixture really plays the slam pair, the down fixture the
        // plain down (the slam rule, end to end through the simulation).
        let (_, _, _, slam) = run(&CP05_LLAMA_SLAM, GorePreset::Full);
        assert_eq!(slam.pose_of(0).map(|p| p.frame), Some(7));
        let (_, _, _, down) = run(&CP05_LLAMA_DOWN, GorePreset::Full);
        assert!(down.pose_of(0).is_none());
        assert!(down.stats().parts_spawned > 0);
        let (_, _, _, crush) = run(&CP05_LLAMA_CRUSH, GorePreset::Full);
        assert_eq!(crush.pose_of(0).map(|p| p.frame), Some(2));
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

    // ============================================================ DEATH-D ====

    const BLAST_PB: DefeatCause = DefeatCause::Explosion {
        charge: 0,
        player: None,
        point_blank: true,
    };
    const BLAST_RING: DefeatCause = DefeatCause::Explosion {
        charge: 0,
        player: None,
        point_blank: false,
    };

    /// The charge of the tests: under the goblin at x 484, on the floor at 100.
    fn charge_484() -> [BlastCharge; 1] {
        [BlastCharge::new(484.0, 100.0, 60, None)]
    }

    /// A goblin killed point blank by the charge (an airborne body at x 484),
    /// run for `ticks` presenter ticks at `preset`.
    fn run_blast(preset: GorePreset, ticks: u64) -> DeathPresenter {
        let mut p = presenter(preset);
        let goblins = [GoblinView {
            mode: GoblinMode::Airborne,
            ..goblin_at(484.0)
        }];
        let charges = charge_484();
        let ob = Observation {
            charges: &charges,
            env_origins: &[],
            ..obs(&goblins)
        };
        p.step(&[defeat(60, 0, BLAST_PB, ForceTier::Catastrophic)], ob);
        for _ in 1..ticks {
            p.step(&[], ob);
        }
        p
    }

    fn sorted(mut ids: Vec<String>) -> Vec<String> {
        ids.sort();
        ids
    }

    // ================================================ LLAMA-DEATH-WIRE ====

    /// The llama death set's `spawner_hints` (LLAMA-02 section 8).
    fn llama_hints() -> serde_json::Value {
        let text =
            std::fs::read_to_string(root().join("assets/manifests/llama_death_h64.json")).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&text).unwrap();
        manifest["death_poses"]["spawner_hints"].clone()
    }

    /// Parts spawned on each presenter tick of a llama death (index = age).
    fn spawned_per_tick(
        preset: GorePreset,
        cause: DefeatCause,
        ticks: u64,
        charges: &[BlastCharge],
    ) -> (DeathPresenter, Vec<u64>) {
        let mut p = llama_presenter(preset);
        let llamas = [llama_at(484.0)];
        let ob = Observation {
            charges,
            ..lobs(&[], &llamas)
        };
        let mut out = Vec::new();
        let mut before = 0;
        for age in 0..ticks {
            let events = if age == 0 {
                vec![defeat(10, 0, cause, ForceTier::Catastrophic)]
            } else {
                Vec::new()
            };
            p.step(&events, ob);
            out.push(p.stats().parts_spawned - before);
            before = p.stats().parts_spawned;
        }
        (p, out)
    }

    #[test]
    fn ldw_the_llama_decapitation_plays_the_llama_02_pair_and_throws_the_head() {
        for preset in GorePreset::ALL {
            let mut p = llama_presenter(preset);
            let llamas = [llama_at(484.0)];
            p.step(
                &[defeat(10, 0, EDGE, ForceTier::Catastrophic)],
                lobs(&[], &llamas),
            );
            let first = p.pose_of(0).expect("the decapitation pose");
            assert_eq!(
                (first.frame, first.variant),
                (3, LLAMA_VARIANT),
                "{preset:?}"
            );
            // The head leaves on D at every preset (preset 1 keeps the head:
            // LLAMA-02 section 5), from the pose 3 `neck`, anchored by its own.
            assert_eq!(count(&llama_part_ids(&p), "head"), 1, "{preset:?}");
            for age in 1..160u64 {
                p.step(&[], lobs(&[], &llamas));
                let want = match age {
                    0..=3 => 3,
                    4..=9 => 4,
                    _ => 5,
                };
                assert_eq!(p.pose_of(0).unwrap().frame, want, "age {age} {preset:?}");
            }
            let art = p.art_of(LLAMA_VARIANT);
            assert_eq!(art.pose(3).tag, "decap_contact");
            assert_eq!(art.pose(4).tag, "headless_collapse");
            assert_eq!(art.pose(5).tag, "headless_collapse");
            // The head swapped to `head_dead` on its first floor contact.
            let ids = llama_part_ids(&p);
            assert_eq!(count(&ids, "head"), 0, "{preset:?} {ids:?}");
            assert_eq!(count(&ids, "head_dead"), 1, "{preset:?} {ids:?}");
            // Not an explosion: the body is a pose, never hidden; no stone over it.
            assert!(!p.hides(0));
            assert!(p.hazard_over_anchor().is_none());
            let floors = llama_floor_ids(&p);
            if preset == GorePreset::Off {
                assert!(
                    floors.iter().all(|id| id.starts_with("decal_molten")),
                    "{floors:?}"
                );
                assert!(
                    floors.iter().any(|id| id.starts_with("decal_molten")),
                    "lava at preset 1"
                );
            }
            if preset == GorePreset::Full {
                assert!(floors.iter().any(|id| id == "decal_pool_3"), "{floors:?}");
                assert!(p.live_wall_decals() > 0, "the jet's wall spatter");
            }
        }
    }

    #[test]
    fn ldw_the_llama_jet_lava_and_trickle_follow_the_spawner_hints() {
        let hints = llama_hints();
        let decap = &hints["decapitation"];
        let (jet, lava, trickle) = (
            &decap["jet_drops"],
            &decap["molten_drops"],
            &decap["trickle"],
        );
        let n = |v: &serde_json::Value| v.as_u64().unwrap();
        assert_eq!(
            (
                n(&jet["every_ticks"]),
                n(&jet["per_emit"]),
                n(&jet["for_ticks"])
            ),
            (3, 3, 36)
        );
        assert_eq!((n(&lava["every_ticks"]), n(&lava["for_ticks"])), (6, 24));
        assert_eq!(
            (n(&trickle["every_ticks"]), n(&trickle["for_ticks"])),
            (8, 40)
        );
        assert_eq!(n(&decap["ticks"]["pose_4"]), 4);
        assert_eq!(n(&decap["ticks"]["pose_5"]), 10);
        // Per tick at preset 4: D = the head and the 8 squeeze drops of the
        // edge; the jet from D+4 every 3 ticks (3 drops) until D+40; the lava
        // from D+4 every 6 ticks until D+28; the trickle from D+10 every 8 ticks
        // until D+50.
        let want: Vec<u64> = (0..60u64)
            .map(|age| {
                let mut k = if age == 0 {
                    1 + EDGE_SQUEEZE_DROPS as u64
                } else {
                    0
                };
                if (4..40).contains(&age) && (age - 4) % 3 == 0 {
                    k += 3;
                }
                if (4..28).contains(&age) && (age - 4) % 6 == 0 {
                    k += 1;
                }
                if (10..50).contains(&age) && (age - 10) % 8 == 0 {
                    k += 1;
                }
                k
            })
            .collect();
        let (_, got) = spawned_per_tick(GorePreset::Full, EDGE, 60, &[]);
        assert_eq!(got, want);
        // The totals at each preset: the blood streams scale by the drop
        // quarters, the lava is kept whole at presets 3 and 4 and halved at 1
        // and 2, the head always flies.
        for (preset, total) in [
            (GorePreset::Full, 1 + 8 + 36 + 5 + 4),
            (GorePreset::Medium, 1 + 4 + 20 + 4),
            (GorePreset::Reduced, 1 + 2 + 10 + 2),
            (GorePreset::Off, 1 + 2),
        ] {
            let (p, _) = spawned_per_tick(preset, EDGE, 60, &[]);
            assert_eq!(p.stats().parts_spawned, total, "{preset:?}");
        }
    }

    #[test]
    fn ldw_the_llama_explosion_tables_are_the_spec_and_the_manifest() {
        let art = llama_art();
        let hints = llama_hints();
        let explosive = &hints["explosive"];
        let n = |v: &serde_json::Value| v.as_u64().unwrap() as u32;
        let full = explosion_table_of(LLAMA_VARIANT, GorePreset::Full);
        let gib = |t: &ExplosionTable, id: &str| {
            t.gibs.iter().find(|(g, _)| *g == id).map_or(0, |(_, k)| *k)
        };
        assert_eq!(
            gib(full, "gib_flesh_a") + gib(full, "gib_flesh_b") + gib(full, "gib_bone"),
            n(&explosive["gibs"])
        );
        assert_eq!(gib(full, "gib_rib"), n(&explosive["gib_rib"]));
        assert_eq!(full.drops, n(&explosive["drop_blood"]));
        assert_eq!(full.molten_drops, n(&explosive["drop_molten"]));
        assert_eq!(u32::from(full.pools), n(&explosive["pools_under"]));
        // Preset 4 lists every body part of the sheet: the wound pieces that
        // exist on frame 0 (not the swap target) and the gland.
        let mut body: Vec<&str> = art
            .parts
            .iter()
            .filter(|m| m.class == PartClass::Wound && m.frame0_origin.is_some())
            .map(|m| m.id.as_str())
            .filter(|id| *id != "head_dead")
            .chain(["gland"])
            .collect();
        let mut listed = full.parts.to_vec();
        body.sort_unstable();
        listed.sort_unstable();
        assert_eq!(listed, body);
        // Section 5's lists for presets 3 and 2, and preset 1's fallback.
        let lists: [&[&str]; 4] = [
            &[],
            &["head", "torso_front", "gland"],
            &[
                "head",
                "torso_front",
                "torso_rump",
                "leg_fore",
                "leg_hind",
                "gland",
            ],
            full.parts,
        ];
        for (i, preset) in GorePreset::ALL.into_iter().enumerate() {
            let t = explosion_table_of(LLAMA_VARIANT, preset);
            assert_eq!(t.parts, lists[i], "{preset:?}");
            assert_eq!(
                t.drops,
                scale_count(28, preset.drop_quarters()),
                "{preset:?}"
            );
            let lava = if preset >= GorePreset::Medium { 8 } else { 4 };
            assert_eq!(t.molten_drops, lava, "{preset:?}");
            for &(id, k) in t.gibs {
                let want = match id {
                    "tooth" => 1,
                    "gib_bone" | "gib_rib" => 2,
                    _ => scale_count(4, preset.gib_quarters()),
                };
                assert_eq!(k, want, "{id} at {preset:?}");
            }
            // Every id the table launches is allowed at its preset.
            // (`torso` and `throat` are emitter points, read from the manifest
            // whatever the preset: they only need a frame-0 origin.)
            for id in [t.torso, t.throat] {
                assert!(
                    art.parts[art.part_index(id).unwrap()]
                        .frame0_origin
                        .is_some()
                );
            }
            let named = ["drop_molten"];
            let ids = t
                .parts
                .iter()
                .chain(t.gibs.iter().map(|(id, _)| id))
                .chain(t.rag_ids.iter())
                .chain(named.iter());
            for id in ids {
                let meta = &art.parts[art.part_index(id).unwrap()];
                assert!(meta.allowed_at(preset), "{id} at {preset:?}");
            }
            if t.drops > 0 {
                assert!(art.parts[art.part_index("drop_blood").unwrap()].allowed_at(preset));
            }
            // The punch-up of preset 4 is the goblin's.
            let g = explosion_table(preset);
            assert_eq!(
                (
                    t.pool_scale,
                    t.extra_wall_spatters,
                    t.burst_scale,
                    t.flash_ticks
                ),
                (
                    g.pool_scale,
                    g.extra_wall_spatters,
                    g.burst_scale,
                    g.flash_ticks
                )
            );
        }
        // The goblin's own tables did not move (variants A and B).
        for preset in GorePreset::ALL {
            assert_eq!(explosion_table_of(0, preset), explosion_table(preset));
            assert_eq!(explosion_table_of(1, preset), explosion_table(preset));
            assert_eq!(explosion_table(preset).molten_drops, 0);
        }
    }

    #[test]
    fn ldw_a_dynamite_kill_of_a_llama_launches_exactly_its_table() {
        let charges = charge_484();
        for preset in GorePreset::ALL {
            let (p, _) = spawned_per_tick(preset, BLAST_PB, 1, &charges);
            let t = explosion_table_of(LLAMA_VARIANT, preset);
            let mut want: Vec<String> = t.parts.iter().map(ToString::to_string).collect();
            for &(id, k) in t.gibs {
                want.extend(std::iter::repeat_n(id.to_string(), k as usize));
            }
            for k in 0..t.rags as usize {
                want.push(t.rag_ids[k % t.rag_ids.len()].to_string());
            }
            want.extend(std::iter::repeat_n(
                "drop_blood".to_string(),
                t.drops as usize,
            ));
            want.extend(std::iter::repeat_n(
                "drop_molten".to_string(),
                t.molten_drops as usize,
            ));
            assert_eq!(sorted(llama_part_ids(&p)), sorted(want), "{preset:?}");
            assert!(p.parts().all(|(_, part)| part.variant == LLAMA_VARIANT));
            // The pieces replace the LLAMA-01 sprite at presets 2 to 4; at
            // preset 1 its `down` frames stay. Never a pose.
            assert_eq!(p.hides(0), preset != GorePreset::Off, "{preset:?}");
            assert_eq!(p.draws_goblin(0), preset == GorePreset::Off, "{preset:?}");
            assert!(p.pose_of(0).is_none());
            assert_eq!(p.recycled(), (0, 0, 0));
        }
    }

    #[test]
    fn ldw_the_exploded_llama_settles_pools_under_its_torso_halves_and_leaves_lava() {
        let charges = charge_484();
        let (p, _) = spawned_per_tick(GorePreset::Full, BLAST_PB, 240, &charges);
        let floors = llama_floor_ids(&p);
        let pools = floors
            .iter()
            .filter(|id| id.starts_with("decal_pool"))
            .count();
        assert!(pools >= 2, "{floors:?}");
        assert!(floors.iter().any(|id| id == "decal_molten"), "{floors:?}");
        assert!(p.live_wall_decals() >= 2, "both spatters at preset 4");
        // Every piece rests on the floor line (never below it).
        for (_, part) in p.parts() {
            assert!(part.y >= part.ground_y - 0.01, "{part:?}");
        }
        // Preset 1: no blood at all, the lava stays.
        let (off, _) = spawned_per_tick(GorePreset::Off, BLAST_PB, 240, &charges);
        let floors = llama_floor_ids(&off);
        assert!(
            floors.iter().all(|id| id.starts_with("decal_molten")),
            "{floors:?}"
        );
        assert!(!floors.is_empty(), "the lava lands at preset 1");
        assert_eq!(off.live_wall_decals(), 0);
        // The same death replays the same pieces (event-seeded).
        let (again, _) = spawned_per_tick(GorePreset::Full, BLAST_PB, 240, &charges);
        assert_eq!(p.digest(), again.digest());
    }

    /// The world point of anchor `name` of llama pose `frame` for the test
    /// llama (x 484, floor 100, facing left: mirrored).
    fn llama_pose_point(art: &DeathArt, frame: usize, name: &str) -> (f32, f32) {
        let anchor = art.pose(frame).anchor(name).unwrap();
        pose_anchor_world(art.pose_pivot, anchor, 484.0, 100.0, true)
    }

    /// Whether a live `id` piece sits exactly on `at` (a piece launched this
    /// tick has not moved yet).
    fn spawned_at(p: &DeathPresenter, id: &str, at: (f32, f32)) -> bool {
        p.parts().any(|(_, part)| {
            p.art_of(part.variant).parts[part.meta].id == id
                && part.x == at.0
                && part.y == at.1.max(part.ground_y)
        })
    }

    #[test]
    fn ldw_ad5_the_jet_leaves_the_buckle_then_the_propped_stump() {
        let art = llama_art();
        let buckle = llama_pose_point(&art, 4, "jet_base");
        let stump = llama_pose_point(&art, 5, "neck");
        assert_ne!(buckle, stump);
        let mut p = llama_presenter(GorePreset::Full);
        let llamas = [llama_at(484.0)];
        p.step(
            &[defeat(10, 0, EDGE, ForceTier::Catastrophic)],
            lobs(&[], &llamas),
        );
        for age in 1..=13u64 {
            p.step(&[], lobs(&[], &llamas));
            if age == 7 {
                // D+7: the buckle (pose 4) shows; the jet leaves its `jet_base`.
                assert!(spawned_at(&p, "drop_blood", buckle), "age 7");
                assert!(!spawned_at(&p, "drop_blood", stump), "age 7");
            }
        }
        // D+13: the hold (pose 5); the jet leaves the propped stump's `neck`.
        assert!(spawned_at(&p, "drop_blood", stump), "age 13");
    }

    #[test]
    fn ldw_ad1_the_gland_and_the_lava_start_at_the_throat() {
        let art = llama_art();
        let neck = art.parts[art.part_index("neck").unwrap()]
            .frame0_origin
            .unwrap();
        assert!(
            art.parts[art.part_index("gland").unwrap()]
                .frame0_origin
                .is_none()
        );
        let throat = pose_anchor_world(art.pose_pivot, (neck[0], neck[1]), 484.0, 100.0, true);
        let (p, _) = spawned_per_tick(GorePreset::Full, BLAST_PB, 1, &charge_484());
        assert!(spawned_at(&p, "gland", throat));
        assert!(spawned_at(&p, "drop_molten", throat));
    }

    #[test]
    fn ldw_ad3_the_cf02_gore_budget_never_doubles_the_tooth() {
        // Q22 (cf02, preset 4): a catastrophic blast scales the gibs by 6/4,
        // never the one clean tooth.
        let mut p = cf02_presenter(GorePreset::Full)
            .with_llama(llama_art())
            .unwrap();
        let llamas = [llama_at(484.0)];
        let charges = charge_484();
        let ob = Observation {
            charges: &charges,
            ..lobs(&[], &llamas)
        };
        p.step(&[defeat(10, 0, BLAST_PB, ForceTier::Catastrophic)], ob);
        let ids = llama_part_ids(&p);
        assert_eq!(count(&ids, "tooth"), 1, "{ids:?}");
        assert!(
            count(&ids, "gib_flesh_a") > 4,
            "the gibs are scaled: {ids:?}"
        );
    }

    #[test]
    fn ldw_the_evidence_lab_reads_a_block_kill_as_an_edge_cut_and_nothing_else() {
        let block = defeat(10, 0, BLOCK, ForceTier::Catastrophic);
        assert_eq!(
            lab_edge_event(block),
            defeat(10, 0, EDGE, ForceTier::Catastrophic)
        );
        for other in [
            defeat(10, 0, DefeatCause::Melee, ForceTier::Heavy),
            defeat(10, 0, BLAST_PB, ForceTier::Catastrophic),
            defeat(10, 0, EDGE, ForceTier::Catastrophic),
            CombatEvent::Restart { tick: 3, deaths: 1 },
        ] {
            assert_eq!(lab_edge_event(other), other);
        }
        // A crushed llama plays the decapitation under the lab, the crush
        // without it.
        let llamas = [llama_at(484.0)];
        let mut lab = llama_presenter(GorePreset::Full).with_lab_edge();
        lab.step(&[block], lobs(&[], &llamas));
        assert_eq!(lab.pose_of(0).unwrap().frame, 3);
        let mut plain = llama_presenter(GorePreset::Full);
        plain.step(&[block], lobs(&[], &llamas));
        assert_eq!(plain.pose_of(0).unwrap().frame, 0);
    }

    // ---- the decapitation through a real event ----

    #[test]
    fn the_block_edge_decapitation_plays_the_script_and_squeezes_the_neck() {
        for (preset, drops) in [
            (GorePreset::Full, 8),
            (GorePreset::Medium, 4),
            (GorePreset::Reduced, 2),
            (GorePreset::Off, 0),
        ] {
            let mut p = presenter(preset);
            let goblins = [goblin_at(484.0)];
            p.step(
                &[defeat(139, 0, EDGE, ForceTier::Catastrophic)],
                obs(&goblins),
            );
            // D: pose 3, the head leaves, and the 8 squeeze drops (scaled by
            // the preset: 8 / 4 / 2 / 0) from the neck.
            assert_eq!(p.pose_of(0).unwrap().frame, 3);
            let ids = part_ids(&p);
            assert_eq!(count(&ids, "head"), 1, "{preset:?}");
            assert_eq!(count(&ids, "drop_blood"), drops, "{preset:?}");
            // The squeeze drops fly low and away from the block (the goblin
            // stands at the block centre: away is its facing side).
            for (_, part) in p.parts() {
                if p.art().parts[part.meta].id == "drop_blood" {
                    assert!(part.vx.abs() > 0.0 && part.vy > 0.0 && part.vy <= 160.0);
                }
            }
            // D+2: the splat on the walking line at the edge (presets 3, 4).
            p.step(&[], obs(&goblins));
            assert!(!floor_ids(&p).contains(&"decal_splat_m".to_string()));
            p.step(&[], obs(&goblins));
            let splats: Vec<_> = p
                .floor_decals()
                .filter(|(_, d)| p.art().parts[d.meta].id == "decal_splat_m")
                .collect();
            if preset >= GorePreset::Medium {
                assert_eq!(splats.len(), 1, "{preset:?}");
                // The edge on the goblin's side of the block centre (484 + 40).
                let meta = &p.art().parts[splats[0].1.meta];
                let (ax, _) = meta
                    .anchor("centre_bottom")
                    .or_else(|| meta.anchor("start"))
                    .unwrap();
                let want = 524.0 - (ax as f32 + 0.5 - p.art().part_cell.0 as f32 / 2.0);
                assert_eq!(splats[0].1.x, want, "{preset:?}");
            } else {
                assert!(splats.is_empty(), "{preset:?}");
            }
        }
    }

    #[test]
    fn the_edge_splat_is_on_the_edge_nearest_the_goblin() {
        let mut p = presenter(GorePreset::Full);
        // A goblin left of the block centre: the left edge (484 - 40 = 444).
        let goblins = [goblin_at(446.0)];
        p.step(
            &[defeat(139, 0, EDGE, ForceTier::Catastrophic)],
            obs(&goblins),
        );
        for _ in 0..2 {
            p.step(&[], obs(&goblins));
        }
        let (_, decal) = p
            .floor_decals()
            .find(|(_, d)| p.art().parts[d.meta].id == "decal_splat_m")
            .unwrap();
        let meta = &p.art().parts[decal.meta];
        let (ax, _) = meta
            .anchor("centre_bottom")
            .or_else(|| meta.anchor("start"))
            .unwrap();
        assert_eq!(
            decal.x,
            444.0 - (ax as f32 + 0.5 - p.art().part_cell.0 as f32 / 2.0)
        );
        // An ordinary crush has no edge splat on D+2 (its stages are its own).
        let mut crush = presenter(GorePreset::Full);
        crush.step(
            &[defeat(139, 0, BLOCK, ForceTier::Catastrophic)],
            obs(&goblins),
        );
        assert!(crush.deaths[0].edge_splat_x.is_none());
    }

    #[test]
    fn the_edge_decapitation_head_lands_on_the_debris_layer_over_the_block() {
        // The head is a part: it draws on the debris layer, above the props.
        let p = run_decap(GorePreset::Full, 3);
        assert!(
            p.parts()
                .any(|(_, part)| p.art().parts[part.meta].id == "head")
        );
        // Above every prop the shore draws (the props layer is actor_z - 1.0,
        // the topmost shore piece is biased up by `ShoreSlot::Impact`).
        let props_top = 7.0 - 1.0
            + [
                ShoreSlot::Cavity,
                ShoreSlot::Post,
                ShoreSlot::Block,
                ShoreSlot::Trickle,
                ShoreSlot::Impact,
            ]
            .into_iter()
            .map(ShoreSlot::z_bias)
            .fold(0.0, f32::max);
        assert!(draw_z(DrawLayer::Debris, 0, 7.0, 3.0) > props_top);
        // The head is the part drawn: on the debris layer, not a decal layer.
        let head = p
            .parts()
            .find(|(_, part)| p.art().parts[part.meta].id == "head")
            .unwrap()
            .0;
        assert!(draw_z(DrawLayer::Debris, head, 7.0, 3.0) > props_top);
    }

    // ---- the explosion tables ----

    #[test]
    fn the_explosion_tables_are_explicit_and_match_the_spec_and_the_manifest() {
        let art = art();
        // (parts, gib counts, rags, drops, drop splat cap, stain cap, pools)
        #[allow(clippy::type_complexity)]
        let spec: [(&[&str], &[(&str, u32)], u32, u32, u8, u8, u8); 4] = [
            (&[], &[], 2, 0, 0, 0, 0),
            (&["head", "torso_upper", "club"], &[], 2, 8, 4, 4, 0),
            (
                &["head", "torso_upper", "arm_near", "leg_far", "club"],
                &[("gib_flesh_a", 2), ("gib_flesh_b", 2)],
                2,
                16,
                8,
                6,
                1,
            ),
            (
                &[
                    "head",
                    "torso_upper",
                    "torso_lower",
                    "arm_near",
                    "arm_far",
                    "leg_near",
                    "leg_far",
                    "ear",
                    "club_stick",
                    "pineapple",
                ],
                &[("gib_flesh_a", 4), ("gib_flesh_b", 4), ("gib_bone", 2)],
                2,
                32,
                14,
                10,
                2,
            ),
        ];
        let peaks = [2usize, 13, 27, 54];
        for (i, preset) in GorePreset::ALL.into_iter().enumerate() {
            let t = explosion_table(preset);
            let (parts, gibs, rags, drops, drop_cap, stain_cap, pools) = spec[i];
            assert_eq!(t.parts, parts, "{preset:?}");
            assert_eq!(t.gibs, gibs, "{preset:?}");
            assert_eq!(
                (t.rags, t.drops, t.drop_splat_cap, t.part_stain_cap, t.pools),
                (rags, drops, drop_cap, stain_cap, pools),
                "{preset:?}"
            );
            // Every listed id is allowed at its preset in the parts manifest.
            let listed = t
                .parts
                .iter()
                .chain(t.gibs.iter().map(|(id, _)| id))
                .chain(["gib_rag"].iter());
            for id in listed {
                let meta = &art.parts[art.part_index(id).unwrap()];
                assert!(meta.allowed_at(preset), "{id} at {preset:?}");
            }
            if t.drops > 0 {
                assert!(art.parts[art.part_index("drop_blood").unwrap()].allowed_at(preset));
            }
            // The totals equal the scaled counts of the spec.
            assert_eq!(
                t.drops,
                scale_count(32, preset.drop_quarters()),
                "{preset:?}"
            );
            for &(id, n) in t.gibs {
                let want = if id == "gib_bone" {
                    2
                } else {
                    scale_count(4, preset.gib_quarters())
                };
                assert_eq!(n, want, "{id} at {preset:?}");
            }
            let total = t.parts.len()
                + t.gibs.iter().map(|(_, n)| *n as usize).sum::<usize>()
                + t.rags as usize
                + t.drops as usize;
            assert_eq!(total, peaks[i], "peak launched pieces at {preset:?}");
        }
        // The club splits only at preset 4.
        assert!(!explosion_table(GorePreset::Full).parts.contains(&"club"));
        assert!(explosion_table(GorePreset::Medium).parts.contains(&"club"));
        // The preset 3 list is the art spec's, fewer than the manifest allows.
        let allowed_3 = art
            .parts
            .iter()
            .filter(|m| m.allowed_at(GorePreset::Medium) && m.frame0_origin.is_some())
            .count();
        assert!(explosion_table(GorePreset::Medium).parts.len() < allowed_3);
    }

    #[test]
    fn an_explosion_launches_exactly_the_table_at_every_preset() {
        for preset in GorePreset::ALL {
            let p = run_blast(preset, 1);
            let t = explosion_table(preset);
            let mut want: Vec<String> = t.parts.iter().map(ToString::to_string).collect();
            for &(id, n) in t.gibs {
                want.extend(std::iter::repeat_n(id.to_string(), n as usize));
            }
            want.extend(std::iter::repeat_n("gib_rag".to_string(), t.rags as usize));
            want.extend(std::iter::repeat_n(
                "drop_blood".to_string(),
                t.drops as usize,
            ));
            assert_eq!(sorted(part_ids(&p)), sorted(want), "{preset:?}");
            let peak = [2usize, 13, 27, 54][preset.level() as usize - 1];
            assert_eq!(p.stats().peak_parts, peak, "{preset:?}");
            assert_eq!(p.recycled(), (0, 0, 0));
            // Nothing is a pose: the sprite is the adapter's to hide.
            assert!(p.pose_of(0).is_none());
        }
    }

    #[test]
    fn an_explosion_leaves_the_decals_of_the_table() {
        for preset in GorePreset::ALL {
            let mut p = run_blast(preset, 1);
            let goblins = [GoblinView {
                mode: GoblinMode::Down,
                ..goblin_at(484.0)
            }];
            let charges = charge_484();
            let ob = Observation {
                charges: &charges,
                env_origins: &[],
                ..obs(&goblins)
            };
            // Age 1: the medium splat under the impact (presets 3 and 4).
            p.step(&[], ob);
            let mediums = count(&floor_ids(&p), "decal_splat_m");
            assert_eq!(
                mediums,
                usize::from(preset >= GorePreset::Medium),
                "{preset:?}"
            );
            // Age 2: both wall spatters (preset 4 only), one fanning each way.
            p.step(&[], ob);
            assert_eq!(
                p.live_wall_decals(),
                if preset == GorePreset::Full { 4 } else { 0 },
                "{preset:?}"
            );
            if preset == GorePreset::Full {
                let walls: Vec<_> = p
                    .wall_decals()
                    .map(|(_, d)| (d.meta, d.flip, d.x))
                    .collect();
                let a = p.art().part_index("decal_wall_spatter").unwrap();
                let b = p.art().part_index("decal_wall_spatter_b").unwrap();
                let left = walls.iter().find(|w| w.0 == a).unwrap();
                let right = walls.iter().find(|w| w.0 == b).unwrap();
                assert!(left.1 && !right.1, "each fans away from the impact");
                assert!(left.2 < right.2, "a on the left of I, b on the right");
            }
        }
    }

    #[test]
    fn the_floor_clamp_holds_for_every_exploded_piece_at_every_preset() {
        for preset in GorePreset::ALL {
            let mut p = run_blast(preset, 1);
            let goblins = [goblin_at(484.0)];
            let charges = charge_484();
            let ob = Observation {
                charges: &charges,
                env_origins: &[],
                ..obs(&goblins)
            };
            for tick in 0..420 {
                p.step(&[], ob);
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
    fn the_pieces_start_at_their_frame0_origin_and_fly_radially_from_the_impact() {
        let p = run_blast(GorePreset::Full, 1);
        let art = p.art();
        let (cx, cy) = (484.0, 100.0 + BLAST_CENTRE_DY);
        let mut bodies = 0;
        for (_, part) in p.parts() {
            let meta = &art.parts[part.meta];
            if !matches!(meta.class, PartClass::Wound | PartClass::Clean) || meta.id == "gib_rag" {
                continue;
            }
            bodies += 1;
            let origin = meta.frame0_origin.unwrap();
            // The goblin faces -1 (flipped): the origin mirrors around its pivot.
            let (sx, sy) =
                pose_anchor_world(art.pose_pivot, (origin[0], origin[1]), 484.0, 100.0, true);
            assert_eq!(part.x, sx, "{} x", meta.id);
            assert_eq!(part.y, sy.max(100.0), "{} y", meta.id);
            assert!(part.flip, "mirrored with the facing");
            // Radial: away from the impact in x (when not under it), with the
            // up-bias of 60 to 120 on top.
            if (sx - cx).abs() >= 2.0 {
                assert_eq!(
                    part.vx.signum(),
                    (sx - cx).signum(),
                    "{} direction",
                    meta.id
                );
            }
            let k = if meta.mass == Mass::Heavy { 0.7 } else { 1.0 };
            let radial = part.vx.hypot(part.vy - 90.0);
            assert!(
                radial <= 300.0 * k + 30.0 + 1e-3 && radial >= 160.0 * k - 30.0 - 1e-3,
                "{}: speed {radial}",
                meta.id
            );
            if sy > cy + 2.0 {
                assert!(part.vy > 0.0, "{} above the impact flies up", meta.id);
            }
            // Spin by class.
            let spin = part.spin_dps.abs();
            match meta.spin {
                SpinClass::Fast => assert!((720.0..=1080.0).contains(&spin), "{}", meta.id),
                SpinClass::Slow => assert!((180.0..=360.0).contains(&spin), "{}", meta.id),
                SpinClass::None => assert_eq!(spin, 0.0, "{}", meta.id),
            }
        }
        assert_eq!(bodies, 10, "the ten body parts");
    }

    #[test]
    fn the_head_swaps_and_the_two_pools_grow_from_the_first_rest_ticks() {
        let mut p = run_blast(GorePreset::Full, 1);
        let goblins = [goblin_at(484.0)];
        let charges = charge_484();
        let ob = Observation {
            charges: &charges,
            env_origins: &[],
            ..obs(&goblins)
        };
        let torso = [
            p.art().part_index("torso_upper").unwrap(),
            p.art().part_index("torso_lower").unwrap(),
        ];
        let mut first_rest: [Option<u64>; 2] = [None, None];
        let mut stages: [Vec<(u64, String)>; 2] = [Vec::new(), Vec::new()];
        for _ in 0..420 {
            p.step(&[], ob);
            let clock = p.clock();
            for k in 0..2 {
                if first_rest[k].is_none()
                    && p.parts()
                        .any(|(_, part)| part.meta == torso[k] && part.is_resting())
                {
                    first_rest[k] = Some(clock);
                }
                if let Some((slot, seq)) = p.deaths[0].pools[k] {
                    let decal = p.floor_decals.get(slot).unwrap();
                    let _ = seq;
                    let id = p.art().parts[decal.meta].id.clone();
                    if stages[k].last().map(|s| &s.1) != Some(&id) {
                        stages[k].push((clock, id));
                    }
                }
            }
        }
        for k in 0..2 {
            let r = first_rest[k].unwrap_or_else(|| panic!("torso {k} never rested"));
            assert_eq!(
                p.deaths[0].rest_ticks[k],
                Some(r),
                "recorded once, on the first rest"
            );
            assert_eq!(
                stages[k],
                [
                    (r, "decal_pool_1".to_string()),
                    (r + 16, "decal_pool_2".to_string()),
                    (r + 40, "decal_pool_3".to_string()),
                ],
                "pool {k}"
            );
        }
        // The head swapped to head_dead on its first floor contact and its
        // roll left one smear.
        let ids = part_ids(&p);
        assert_eq!(count(&ids, "head"), 0, "{ids:?}");
        assert_eq!(count(&ids, "head_dead"), 1, "{ids:?}");
        assert_eq!(count(&floor_ids(&p), "decal_smear"), 1);
        // Two pools, stage 3 (the 38 px pair), one entry each.
        let pools: Vec<_> = floor_ids(&p)
            .into_iter()
            .filter(|i| i.starts_with("decal_pool"))
            .collect();
        assert_eq!(pools, ["decal_pool_3", "decal_pool_3"]);
        // Preset 3: one pool (torso_upper), up to stage 2.
        let mut p3 = run_blast(GorePreset::Medium, 1);
        for _ in 0..420 {
            p3.step(&[], ob);
        }
        let pools: Vec<_> = floor_ids(&p3)
            .into_iter()
            .filter(|i| i.starts_with("decal_pool"))
            .collect();
        assert_eq!(pools, ["decal_pool_2"]);
        // Presets 1 and 2: no pool decal at all.
        for preset in [GorePreset::Off, GorePreset::Reduced] {
            let mut q = run_blast(preset, 1);
            for _ in 0..420 {
                q.step(&[], ob);
            }
            assert!(
                !floor_ids(&q).iter().any(|i| i.starts_with("decal_pool")),
                "{preset:?}"
            );
        }
    }

    #[test]
    fn a_tracked_piece_that_never_rested_gets_no_pool() {
        // Stop before the torsos can rest: no rest tick, no pool.
        let p = run_blast(GorePreset::Full, 12);
        assert_eq!(p.deaths[0].rest_ticks, [None, None]);
        assert!(p.deaths[0].pools.iter().all(Option::is_none));
    }

    #[test]
    fn exploded_pieces_settle_irregularly_and_overlap_at_preset_four() {
        let p = run_blast(GorePreset::Full, 300);
        let sheet = &p.sheets().parts;
        let mut resting: Vec<(f32, f32)> = p
            .parts()
            .filter(|(_, part)| part.is_resting())
            .filter(|(_, part)| {
                !matches!(
                    p.art().parts[part.meta].class,
                    PartClass::Blood | PartClass::Gib
                )
            })
            .map(|(_, part)| {
                let w = sheet.bbox[part.frame].map_or(0, |b| b.width()) as f32;
                (part.x, w)
            })
            .collect();
        assert!(resting.len() >= 6, "{resting:?}");
        resting.sort_by(|a, b| a.0.total_cmp(&b.0));
        let gaps: Vec<f32> = resting.windows(2).map(|w| w[1].0 - w[0].0).collect();
        assert!(
            gaps.iter().any(|g| (g - gaps[0]).abs() > 0.5),
            "an evenly spaced row: {gaps:?}"
        );
        let overlap = resting
            .windows(2)
            .any(|w| w[1].0 - w[0].0 < (w[0].1 + w[1].1) / 2.0);
        assert!(overlap, "no two pieces overlap: {resting:?}");
    }

    #[test]
    fn the_sprite_is_hidden_at_presets_two_to_four_and_shown_at_one() {
        for preset in GorePreset::ALL {
            let mut p = presenter(preset);
            let goblins = [GoblinView {
                mode: GoblinMode::Airborne,
                ..goblin_at(484.0)
            }];
            let charges = charge_484();
            let ob = Observation {
                charges: &charges,
                env_origins: &[],
                ..obs(&goblins)
            };
            assert!(!p.hides(0), "before the death");
            p.step(&[defeat(60, 0, BLAST_PB, ForceTier::Catastrophic)], ob);
            assert_eq!(p.hides(0), preset >= GorePreset::Reduced, "{preset:?}");
            assert!(!p.hides(1), "another goblin");
            for _ in 0..400 {
                p.step(&[], ob);
            }
            assert_eq!(
                p.hides(0),
                preset >= GorePreset::Reduced,
                "kept: {preset:?}"
            );
            // A restart clears it.
            p.step(&[CombatEvent::Restart { tick: 9, deaths: 1 }], ob);
            assert!(!p.hides(0), "cleared by the restart");
        }
    }

    #[test]
    fn no_other_finish_hides_the_sprite() {
        for (cause, force) in [
            (DefeatCause::Melee, ForceTier::Routine),
            (DefeatCause::Melee, ForceTier::Heavy),
            (BLOCK, ForceTier::Catastrophic),
            (EDGE, ForceTier::Catastrophic),
            (BLAST_RING, ForceTier::Heavy),
        ] {
            for preset in GorePreset::ALL {
                let mut p = presenter(preset);
                let goblins = [goblin_at(484.0)];
                let charges = charge_484();
                let ob = Observation {
                    charges: &charges,
                    env_origins: &[],
                    ..obs(&goblins)
                };
                p.step(&[defeat(60, 0, cause, force)], ob);
                for _ in 0..60 {
                    p.step(&[], ob);
                }
                assert!(!p.hides(0), "{cause:?} at {preset:?}");
            }
        }
    }

    #[test]
    fn a_ring_kill_plays_the_heavy_knockback_away_from_the_charge() {
        let mut p = presenter(GorePreset::Full);
        // The goblin stands 16 px right of the charge: thrown right.
        let goblins = [GoblinView {
            mode: GoblinMode::Airborne,
            ..goblin_at(500.0)
        }];
        let charges = charge_484();
        let ob = Observation {
            charges: &charges,
            env_origins: &[],
            ..obs(&goblins)
        };
        p.step(&[defeat(60, 0, BLAST_RING, ForceTier::Heavy)], ob);
        let d = &p.deaths[0];
        assert_eq!(d.finish, Finish::Knockback);
        assert_eq!(d.away, 1.0);
        assert!(d.impact.is_none(), "only a burst has an impact point");
        assert!(p.pose_of(0).is_none(), "the GOB-01 frames stay");
        // The heavy script: 24 drops and its gibs.
        let ids = part_ids(&p);
        assert_eq!(count(&ids, "drop_blood"), 24);
        assert!(!p.hides(0));
    }

    #[test]
    fn the_qa_blast_at_preset_four_evicts_nothing_and_peaks_at_81_pieces() {
        let (_, _, p) = replay(
            &crate::combat_scene::QA_BLAST_BREAKUP,
            GorePreset::Full,
            None,
        );
        assert_eq!(p.recycled(), (0, 0, 0), "no pool recycled anything");
        // 54 from the burst, 24 drops and 3 gibs from the thrown ring kill.
        assert_eq!(p.stats().peak_parts, 81);
        assert!(p.stats().peak_floor_decals <= FLOOR_DECAL_POOL_CAPACITY);
        // The burst's sprite is hidden at preset 4 (target 0), the untouched
        // goblin 2 never is, and the ring goblin keeps its sprite.
        assert!(p.hides(0) && !p.hides(1) && !p.hides(2));
    }

    #[test]
    fn the_block_edge_replay_decapitates_through_the_real_loop_inputs() {
        let (_, events, p) = replay(
            &crate::combat_scene::CP03_BLOCK_EDGE_DECAP,
            GorePreset::Full,
            None,
        );
        assert!(events.iter().any(|e| matches!(
            e,
            CombatEvent::EnemyDefeated {
                cause: DefeatCause::ShoreBlockEdge { .. },
                ..
            }
        )));
        // The remains hold the headless pose, the head lies as head_dead.
        assert_eq!(p.pose_of(0).unwrap().frame, 5);
        assert_eq!(count(&part_ids(&p), "head_dead"), 1);
        assert!(count(&floor_ids(&p), "decal_splat_m") >= 1);
    }

    #[test]
    fn the_stress_mix_rotates_crush_decapitation_and_explosion() {
        let mut p = presenter(GorePreset::Full);
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..90 {
            p.stress_tick(400.0, 100.0);
            p.step(&[], obs(&[]));
            for d in &p.deaths {
                seen.insert(format!("{:?}", d.finish));
            }
            assert!(p.live_parts() <= PART_POOL_CAPACITY);
            assert!(p.live_floor_decals() <= FLOOR_DECAL_POOL_CAPACITY);
            assert!(p.live_wall_decals() <= WALL_DECAL_POOL_CAPACITY);
        }
        assert_eq!(
            seen.into_iter().collect::<Vec<_>>(),
            ["Crushed", "Decapitated", "Exploded"]
        );
        // A synthetic explosion has no charge: the impact is under it.
        let mut q = presenter(GorePreset::Full);
        q.spawn_synthetic(Finish::Exploded, 300.0, 100.0, false, 1);
        assert_eq!(q.deaths[0].impact, Some((300.0, 100.0 + BLAST_CENTRE_DY)));
        q.step(&[], obs(&[]));
        assert_eq!(q.stats().peak_parts, 54);
        assert!(q.pose_of(0).is_none());
    }

    #[test]
    fn every_finish_the_goblin_profile_can_select_has_art() {
        let profiles = DeathProfiles::builtin();
        let goblin = profiles.species("goblin").unwrap();
        let mut ids: Vec<&str> = goblin.recipes.iter().map(|r| r.finish.as_str()).collect();
        ids.push(goblin.fallback_finish.as_str());
        for id in ids {
            assert_ne!(
                Finish::from_id(id),
                Finish::NoArt,
                "{id} has no presenter art"
            );
        }
        // The finishes the profile can pick are the four with art.
        assert_eq!(Finish::from_id("goblin_decapitated"), Finish::Decapitated);
        assert_eq!(Finish::from_id("goblin_exploded"), Finish::Exploded);
        // The llama: the crush, the slam and the plain down (DEMO-01 PR B),
        // and since LLAMA-DEATH-WIRE its decapitation and explosion, have art.
        // Every recipe of its profile and its fallback select one.
        let llama = profiles.species("llama").unwrap();
        let mut llama_ids: Vec<&str> = llama.recipes.iter().map(|r| r.finish.as_str()).collect();
        llama_ids.push(llama.fallback_finish.as_str());
        assert_eq!(llama_ids.len(), 6, "{llama_ids:?}");
        for id in llama_ids {
            let want = match id {
                "llama_crushed" => Finish::Crushed,
                "llama_slammed" => Finish::Slammed,
                "llama_down" => Finish::Knockback,
                "llama_decapitated" => Finish::Decapitated,
                "llama_exploded" => Finish::Exploded,
                other => panic!("an unexpected llama finish {other}"),
            };
            assert_eq!(Finish::from_id(id), want, "{id}");
        }
    }

    #[test]
    fn a_species_without_decapitation_art_draws_the_next_class_or_nothing() {
        // A species without the art is never drawn as a goblin: the block edge
        // falls to the skeleton mock's crush (`skeleton_flat_pile`), a finish
        // id with no presenter art, which draws nothing, never a mismatched
        // head. (The llama was this test's species until LLAMA-DEATH-WIRE
        // gave it the art.)
        let mut p = presenter(GorePreset::Full);
        p.species = "skeleton_mock";
        let goblins = [goblin_at(484.0)];
        let charges = charge_484();
        let ob = Observation {
            charges: &charges,
            env_origins: &[],
            ..obs(&goblins)
        };
        p.step(&[defeat(139, 0, EDGE, ForceTier::Catastrophic)], ob);
        for _ in 0..30 {
            p.step(&[], ob);
        }
        assert!(p.pose_of(0).is_none());
        assert_eq!(p.live_parts(), 0);
        assert_eq!(p.live_floor_decals(), 0);
        assert!(!p.hides(0));
    }

    // ---- the pure frame selection ----

    #[test]
    fn the_charge_fizzle_alternates_frames_four_and_five_every_four_ticks() {
        for tick in 0..64u64 {
            let want = if tick % 8 < 4 { 4 } else { 5 };
            assert_eq!(charge_frame(tick), want, "tick {tick}");
        }
    }

    #[test]
    fn the_blast_burst_follows_the_manifest_tick_table() {
        let text = std::fs::read_to_string(root().join("assets/manifests/vfx-01.json")).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&text).unwrap();
        let burst: Vec<(usize, u64)> = manifest["vfx"]["frames"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["tag"] == "burst")
            .map(|f| {
                (
                    f["index"].as_u64().unwrap() as usize,
                    f["ticks"].as_u64().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            burst, BURST_FRAMES,
            "the constants are the manifest's table"
        );
        // The charge frames are the manifest's dynamite_fizzle (4 ticks each).
        let text =
            std::fs::read_to_string(root().join("assets/manifests/vfx_dynamite.json")).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&text).unwrap();
        let fizzle: Vec<(usize, u64)> = manifest["vfx"]["frames"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["tag"] == "dynamite_fizzle")
            .map(|f| {
                (
                    f["index"].as_u64().unwrap() as usize,
                    f["ticks"].as_u64().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            fizzle,
            [
                (CHARGE_FRAMES[0], CHARGE_FRAME_TICKS),
                (CHARGE_FRAMES[1], CHARGE_FRAME_TICKS)
            ]
        );
        // The whole tick range of a blast that went off on tick 60: nothing
        // before it, each frame for its ticks, nothing after the last.
        assert_eq!(burst_ticks(), 42);
        for tick in 0..60 {
            assert_eq!(blast_vfx_frame(tick, 60), None, "before, tick {tick}");
        }
        let mut tick = 60;
        for (frame, ticks) in BURST_FRAMES {
            for _ in 0..ticks {
                assert_eq!(blast_vfx_frame(tick, 60), Some(frame), "tick {tick}");
                tick += 1;
            }
        }
        assert_eq!(tick, 60 + burst_ticks());
        for after in tick..tick + 200 {
            assert_eq!(blast_vfx_frame(after, 60), None, "after, tick {after}");
        }
    }

    #[test]
    fn one_explosion_peaks_at_the_floor_decals_of_the_table() {
        // Drop splats + part stains + the impact splat + the head smear + the
        // pools: 28 / 16 / 8 / 0 at presets 4 / 3 / 2 / 1.
        for (preset, peak) in [
            (GorePreset::Full, 28),
            (GorePreset::Medium, 16),
            (GorePreset::Reduced, 8),
            (GorePreset::Off, 0),
        ] {
            let mut p = run_blast(preset, 1);
            let goblins = [goblin_at(484.0)];
            let charges = charge_484();
            let ob = Observation {
                charges: &charges,
                env_origins: &[],
                ..obs(&goblins)
            };
            for _ in 0..420 {
                p.step(&[], ob);
            }
            assert_eq!(p.stats().peak_floor_decals, peak, "{preset:?}");
            assert_eq!(
                p.stats().peak_wall_decals,
                if preset == GorePreset::Full { 4 } else { 0 }
            );
        }
    }

    // ---- review fixes: shadow rule, punch-up, layers, spin class ----

    #[test]
    fn a_hidden_or_retired_goblin_has_no_body_and_so_no_shadow() {
        for preset in GorePreset::ALL {
            let mut p = presenter(preset);
            let goblins = [
                GoblinView {
                    mode: GoblinMode::Airborne,
                    ..goblin_at(484.0)
                },
                goblin_at(560.0),
            ];
            let charges = charge_484();
            let ob = Observation {
                charges: &charges,
                env_origins: &[],
                ..obs(&goblins)
            };
            assert!(p.draws_goblin(0) && p.draws_goblin(1));
            p.step(&[defeat(60, 0, BLAST_PB, ForceTier::Catastrophic)], ob);
            // The exploded goblin loses its body (and so its sprite and its
            // shadow) at presets 2 to 4; at preset 1 it keeps both.
            assert_eq!(p.draws_goblin(0), preset == GorePreset::Off, "{preset:?}");
            assert!(p.draws_goblin(1), "an untouched goblin keeps its body");
            assert_eq!(p.hides(0), !p.draws_goblin(0));
        }
        // A retired corpse has no body either.
        let mut p = presenter(GorePreset::Full);
        p.retired.push(3);
        assert!(!p.draws_goblin(3));
    }

    #[test]
    fn only_preset_four_is_punched_up() {
        for preset in GorePreset::ALL {
            let t = explosion_table(preset);
            // Pixel art: only whole-number sprite scales, never a fraction.
            assert_eq!(t.burst_scale.fract(), 0.0, "{preset:?}");
            assert_eq!(t.pool_scale.fract(), 0.0, "{preset:?}");
            if preset == GorePreset::Full {
                assert_eq!(t.burst_scale, 2.0);
                assert_eq!(t.pool_scale, 2.0);
                assert_eq!(t.extra_wall_spatters, 2);
                assert_eq!(t.flash_ticks, 3);
            } else {
                assert_eq!(
                    (
                        t.burst_scale,
                        t.pool_scale,
                        t.extra_wall_spatters,
                        t.flash_ticks
                    ),
                    (1.0, 1.0, 0, 0),
                    "{preset:?} is as before"
                );
            }
        }
    }

    #[test]
    fn the_preset_four_pools_and_extra_spatters_are_bigger_and_stay_on_the_floor() {
        let mut p = run_blast(GorePreset::Full, 1);
        let goblins = [goblin_at(484.0)];
        let charges = charge_484();
        let ob = Observation {
            charges: &charges,
            env_origins: &[],
            ..obs(&goblins)
        };
        for _ in 0..420 {
            p.step(&[], ob);
        }
        let pools: Vec<_> = p
            .floor_decals()
            .filter(|(_, d)| p.art().parts[d.meta].id.starts_with("decal_pool"))
            .collect();
        assert_eq!(pools.len(), 2);
        for (_, d) in pools {
            assert_eq!(d.scale, 2.0);
            // The scaled sprite's lowest opaque row is still on the floor.
            let meta = &p.art().parts[d.meta];
            let bbox = p.sheets().parts.bbox[meta.frame].unwrap();
            let bottom = d.y - 2.0 * (bbox.bottom - p.art().part_cell.1 / 2) as f32;
            assert!((bottom - 100.0).abs() < 1e-3, "bottom {bottom}");
        }
        // Four spatters, the two extra at the larger size.
        let scales: Vec<f32> = p.wall_decals().map(|(_, d)| d.scale).collect();
        assert_eq!(scales.len(), 4);
        assert_eq!(scales.iter().filter(|s| **s == 2.0).count(), 2);
        // The ordinary decals stay at 1.0, and so does every decal at preset 3.
        assert!(p.floor_decals().any(|(_, d)| d.scale == 1.0));
        let mut p3 = run_blast(GorePreset::Medium, 1);
        for _ in 0..420 {
            p3.step(&[], ob);
        }
        assert!(p3.floor_decals().all(|(_, d)| d.scale == 1.0));
        assert!(p3.wall_decals().all(|(_, d)| d.scale == 1.0));
        // Still inside the budget.
        assert!(p.live_floor_decals() <= FLOOR_DECAL_POOL_CAPACITY);
        assert!(p.live_wall_decals() <= WALL_DECAL_POOL_CAPACITY);
    }

    #[test]
    fn the_burst_and_the_lit_charge_layers_sit_where_the_review_asked() {
        let (actor_z, wall_z) = (7.0f32, 3.0f32);
        let debris_top = draw_z(DrawLayer::Debris, PART_POOL_SLOTS, actor_z, wall_z);
        let lowest_actor = actor_z - 0.01 - 0.001 * 64.0;
        for slot in [0, 7] {
            // The preset 4 burst is over every piece of debris, under every actor.
            let front = draw_z(DrawLayer::BlastFront, slot, actor_z, wall_z);
            assert!(front > debris_top && front < lowest_actor);
            // The lit charge is over a standing goblin (+0.05, at most 64
            // indices stepping down), under Carl (+0.1) and Donut-level actors
            // keep their own z.
            let lit = draw_z(DrawLayer::ChargeLit, slot, actor_z, wall_z);
            assert!(lit > actor_z + 0.05, "over a standing goblin");
            assert!(lit < actor_z + 0.1, "under Carl");
        }
    }

    #[test]
    fn explosion_gibs_spin_by_their_manifest_class() {
        let p = run_blast(GorePreset::Full, 1);
        let mut gibs = 0;
        for (_, part) in p.parts() {
            let meta = &p.art().parts[part.meta];
            if meta.class != PartClass::Gib {
                continue;
            }
            gibs += 1;
            let spin = part.spin_dps.abs();
            match meta.spin {
                SpinClass::Fast => assert!((720.0..=1080.0).contains(&spin), "{}", meta.id),
                SpinClass::Slow => assert!((180.0..=360.0).contains(&spin), "{}", meta.id),
                SpinClass::None => assert_eq!(spin, 0.0),
            }
        }
        assert_eq!(gibs, 10);
        // A gib is not a fixed 720: the class gives a range.
        let spins: std::collections::BTreeSet<u32> = p
            .parts()
            .filter(|(_, part)| p.art().parts[part.meta].class == PartClass::Gib)
            .map(|(_, part)| part.spin_dps.abs().to_bits())
            .collect();
        assert!(spins.len() > 1);
    }

    #[test]
    fn an_unarmed_charge_is_a_still_stick_and_the_fizzle_counts_from_the_arming() {
        for ticks in 0..40 {
            assert_eq!(charge_draw_frame(false, ticks), CHARGE_UNLIT_FRAME);
            assert_eq!(charge_draw_frame(true, ticks), charge_frame(ticks));
        }
        assert!(!CHARGE_FRAMES.contains(&CHARGE_UNLIT_FRAME));
        // The first lit tick starts the loop on its first frame.
        assert_eq!(charge_draw_frame(true, 0), CHARGE_FRAMES[0]);
    }

    // ------------------------------------------------ CF02-D (Q21..Q43) ----

    fn cf02_presenter(preset: GorePreset) -> DeathPresenter {
        presenter(preset).with_cf02(crate::cf02::active())
    }

    fn hit_on(move_id: crate::combat::MoveId, target: u32) -> CombatEvent {
        CombatEvent::Hit {
            tick: 60,
            move_id,
            instance: 1,
            target,
            damage: 1,
            health_left: 0,
            hit_stop: 0,
        }
    }

    fn slam_on(target: u32) -> CombatEvent {
        CombatEvent::WallSlam {
            tick: 60,
            launch: 1,
            goblin: target,
            instigator: Some(0),
        }
    }

    /// One heavy blunt defeat of goblin 0 with `lead` events before it on the
    /// same tick; steps `ticks` presenter ticks in all.
    fn run_slam(p: &mut DeathPresenter, lead: &[CombatEvent], force: ForceTier, ticks: u64) {
        let goblins = [goblin_at(484.0)];
        let mut events = lead.to_vec();
        events.push(defeat(60, 0, DefeatCause::Melee, force));
        p.step(&events, obs(&goblins));
        for _ in 1..ticks {
            p.step(&[], obs(&goblins));
        }
    }

    fn gibs_and_arms(p: &DeathPresenter) -> (usize, usize) {
        let ids = part_ids(p);
        let gibs = ["gib_flesh_a", "gib_flesh_b", "gib_bone"]
            .iter()
            .map(|id| count(&ids, id))
            .sum();
        (gibs, count(&ids, "arm_near") + count(&ids, "arm_far"))
    }

    #[test]
    fn q21_the_pose_follows_the_events_and_the_legacy_presenter_has_none() {
        use crate::combat::MoveId;
        let cases: [(&[CombatEvent], SlamPose); 5] = [
            (&[hit_on(MoveId::Stomp, 0)], SlamPose::Stomped),
            (&[hit_on(MoveId::DiveStomp, 0)], SlamPose::Stomped),
            (&[slam_on(0)], SlamPose::WallCrumple),
            (&[hit_on(MoveId::PushKick, 0)], SlamPose::Flung),
            (&[], SlamPose::Flung),
        ];
        for (lead, pose) in cases {
            let mut p = cf02_presenter(GorePreset::Full);
            run_slam(&mut p, lead, ForceTier::Heavy, 2);
            assert_eq!(p.slam_pose_of(0), Some(pose), "{lead:?}");
            // The legacy presenter plays the plain knockback for the same events.
            let mut legacy = presenter(GorePreset::Full);
            run_slam(&mut legacy, lead, ForceTier::Heavy, 2);
            assert_eq!(legacy.slam_pose_of(0), None);
            assert_eq!(legacy.slammed_poses().count(), 0);
        }
        // A stomp or a slam on ANOTHER goblin does not pick this one's pose.
        let mut p = cf02_presenter(GorePreset::Full);
        run_slam(
            &mut p,
            &[hit_on(MoveId::Stomp, 1), slam_on(1)],
            ForceTier::Heavy,
            2,
        );
        assert_eq!(p.slam_pose_of(0), Some(SlamPose::Flung));
    }

    /// F01-FU-B10 (CF02-FU-I FU7): the presenter reports the slammed pose for as
    /// long as the finish lasts and keeps no impact clock of its own; the impact
    /// hold (8 ticks, then the rest frame) is the acting selector's, tested in
    /// `combat_anim::cf02::tests` (`the_three_slammed_finishes_hold_their_impact_then_rest`,
    /// `a_lethal_stomp_on_a_taunting_goblin_holds_the_taunt_then_folds_then_rests`).
    #[test]
    fn fu7_the_presenter_reports_the_slammed_pose_and_leaves_the_clock_to_the_art() {
        let mut p = cf02_presenter(GorePreset::Full);
        assert!(p.cf02_overlay());
        let goblins = [goblin_at(484.0)];
        p.step(
            &[
                hit_on(crate::combat::MoveId::Stomp, 0),
                defeat(60, 0, DefeatCause::Melee, ForceTier::Heavy),
            ],
            obs(&goblins),
        );
        assert_eq!(crate::cf02::active().q21_slammed_impact_hold_ticks, 8);
        for _ in 0..20 {
            let view = p.slammed_poses().next().expect("the slammed view");
            assert_eq!((view.target, view.pose), (0, SlamPose::Stomped));
            p.step(&[], obs(&goblins));
        }
    }

    #[test]
    fn q43_each_slammed_pose_has_its_own_gore_baseline_at_preset_four() {
        use crate::combat::MoveId;
        let crumbs = |lead: &[CombatEvent], ticks: u64| {
            let mut p = cf02_presenter(GorePreset::Full);
            run_slam(&mut p, lead, ForceTier::Heavy, ticks);
            gibs_and_arms(&p)
        };
        // Stomped: 6 quarters of the 3-gib set (5 gibs), no separated part ever.
        assert_eq!(crumbs(&[hit_on(MoveId::Stomp, 0)], 1), (5, 0));
        assert_eq!(crumbs(&[hit_on(MoveId::Stomp, 0)], 3), (5, 0));
        // Wall crumple: 4 quarters (3 gibs) and one arm at once, on the wall side.
        assert_eq!(crumbs(&[slam_on(0)], 1), (3, 1));
        // Flung: 6 quarters (5 gibs); the arm separates on landing (tick 2).
        assert_eq!(crumbs(&[hit_on(MoveId::PushKick, 0)], 1), (5, 0));
        assert_eq!(crumbs(&[hit_on(MoveId::PushKick, 0)], 2), (5, 1));
        // Legacy heavy kick for comparison: 3 gibs, no arm.
        let mut legacy = presenter(GorePreset::Full);
        run_slam(
            &mut legacy,
            &[hit_on(MoveId::PushKick, 0)],
            ForceTier::Heavy,
            2,
        );
        assert_eq!(gibs_and_arms(&legacy), (3, 0));
    }

    #[test]
    fn q43_presets_one_to_three_scale_by_their_existing_ratio() {
        use crate::combat::MoveId;
        // Preset 3: 6 * 2 / 4 = 3 quarters of the two gibs it draws (the bone is
        // preset 4 only), so 2 gibs; the legacy kick draws 1 there.
        for (preset, stomped, wall_arms) in [
            (GorePreset::Medium, 2, 1),
            (GorePreset::Reduced, 0, 0),
            (GorePreset::Off, 0, 0),
        ] {
            let mut p = cf02_presenter(preset);
            run_slam(&mut p, &[hit_on(MoveId::Stomp, 0)], ForceTier::Heavy, 1);
            assert_eq!(gibs_and_arms(&p), (stomped, 0), "{preset:?} stomped");
            let mut w = cf02_presenter(preset);
            run_slam(&mut w, &[slam_on(0)], ForceTier::Heavy, 1);
            assert_eq!(gibs_and_arms(&w).1, wall_arms, "{preset:?} wall arm");
        }
        // Presets 1 and 2 are byte-identical to the legacy presenter's pieces
        // for the same kill: nothing the overlay adds is drawn there.
        for preset in [GorePreset::Off, GorePreset::Reduced] {
            let pieces = |p: &DeathPresenter| {
                p.parts()
                    .map(|(_, part)| (part.meta, part.x.to_bits(), part.y.to_bits()))
                    .collect::<Vec<_>>()
            };
            let mut new = cf02_presenter(preset);
            let mut old = presenter(preset);
            for p in [&mut new, &mut old] {
                run_slam(
                    p,
                    &[hit_on(crate::combat::MoveId::PushKick, 0)],
                    ForceTier::Heavy,
                    30,
                );
            }
            assert_eq!(pieces(&new), pieces(&old), "{preset:?}");
            assert_eq!(new.live_floor_decals(), old.live_floor_decals());
        }
    }

    #[test]
    fn q43_a_routine_blunt_kill_adds_one_transient_spray_gone_after_24_ticks() {
        let goblins = [goblin_at(484.0)];
        let transient = |p: &DeathPresenter| {
            p.parts
                .iter()
                .filter(|(_, part)| part.transient_until.is_some())
                .count()
        };
        let mut p = cf02_presenter(GorePreset::Full);
        p.step(
            &[defeat(60, 0, DefeatCause::Melee, ForceTier::Routine)],
            obs(&goblins),
        );
        assert_eq!(transient(&p), SPRAY_DROPS as usize, "one spray at the kill");
        let born = p.clock;
        let mut decals_seen = 0;
        while p.clock < born + 23 {
            p.step(&[], obs(&goblins));
            decals_seen = decals_seen.max(p.live_floor_decals());
        }
        assert!(transient(&p) <= SPRAY_DROPS as usize);
        p.step(&[], obs(&goblins));
        assert_eq!(p.clock, born + 24);
        assert_eq!(transient(&p), 0, "no spray piece survives 24 ticks");
        // It is not a decal: the legacy presenter leaves the same decals.
        let mut legacy = presenter(GorePreset::Full);
        legacy.step(
            &[defeat(60, 0, DefeatCause::Melee, ForceTier::Routine)],
            obs(&goblins),
        );
        for _ in 0..24 {
            legacy.step(&[], obs(&goblins));
        }
        let mut again = cf02_presenter(GorePreset::Full);
        again.step(
            &[defeat(60, 0, DefeatCause::Melee, ForceTier::Routine)],
            obs(&goblins),
        );
        for _ in 0..24 {
            again.step(&[], obs(&goblins));
        }
        assert_eq!(again.live_floor_decals(), legacy.live_floor_decals());
        let _ = decals_seen;
        // Presets 1 to 3 and heavy kills never spray.
        for preset in [GorePreset::Off, GorePreset::Reduced, GorePreset::Medium] {
            let mut q = cf02_presenter(preset);
            q.step(
                &[defeat(60, 0, DefeatCause::Melee, ForceTier::Routine)],
                obs(&goblins),
            );
            assert_eq!(transient(&q), 0, "{preset:?}");
        }
        let mut heavy = cf02_presenter(GorePreset::Full);
        run_slam(&mut heavy, &[], ForceTier::Heavy, 1);
        assert_eq!(transient(&heavy), 0);
    }

    #[test]
    fn q22_preset_four_heavy_and_catastrophic_finishes_break_up_more() {
        // The crush (catastrophic): 5 gibs instead of 3 at preset 4 only.
        let crush = |p: &DeathPresenter| gibs_and_arms(p).0;
        let goblins = [goblin_at(484.0)];
        let run = |mut p: DeathPresenter| {
            p.step(
                &[defeat(139, 0, BLOCK, ForceTier::Catastrophic)],
                obs(&goblins),
            );
            p
        };
        assert_eq!(crush(&run(presenter(GorePreset::Full))), 3);
        assert_eq!(crush(&run(cf02_presenter(GorePreset::Full))), 5);
        for preset in [GorePreset::Medium, GorePreset::Reduced, GorePreset::Off] {
            assert_eq!(
                crush(&run(cf02_presenter(preset))),
                crush(&run(presenter(preset))),
                "{preset:?} unchanged"
            );
        }
        // The decapitation separates one more part (an arm) at preset 4 only.
        let decap = |mut p: DeathPresenter| {
            p.step(
                &[defeat(139, 0, EDGE, ForceTier::Catastrophic)],
                obs(&goblins),
            );
            gibs_and_arms(&p).1
        };
        assert_eq!(decap(presenter(GorePreset::Full)), 0);
        assert_eq!(decap(cf02_presenter(GorePreset::Full)), 1);
        assert_eq!(decap(cf02_presenter(GorePreset::Medium)), 0);
        // The explosion's ten gibs become fifteen (6, 6 and 3).
        let blast = |mut p: DeathPresenter| {
            let ob_goblins = [GoblinView {
                mode: GoblinMode::Airborne,
                ..goblin_at(484.0)
            }];
            let charges = charge_484();
            let ob = Observation {
                charges: &charges,
                env_origins: &[],
                ..obs(&ob_goblins)
            };
            p.step(&[defeat(60, 0, BLAST_PB, ForceTier::Catastrophic)], ob);
            gibs_and_arms(&p).0
        };
        assert_eq!(blast(presenter(GorePreset::Full)), 10);
        assert_eq!(blast(cf02_presenter(GorePreset::Full)), 15);
        assert_eq!(
            blast(cf02_presenter(GorePreset::Medium)),
            blast(presenter(GorePreset::Medium))
        );
    }

    #[test]
    fn q36_a_tuning_with_the_rules_off_changes_nothing() {
        let off: &'static Cf02Tuning = Box::leak(Box::new(Cf02Tuning::pinned().unwrap()));
        assert!(!off.cf02_rules);
        let p = presenter(GorePreset::Full).with_cf02(off);
        assert!(!p.cf02_overlay());
        let goblins = [goblin_at(484.0)];
        let mut a = p;
        let mut b = presenter(GorePreset::Full);
        for q in [&mut a, &mut b] {
            q.step(
                &[defeat(60, 0, DefeatCause::Melee, ForceTier::Heavy)],
                obs(&goblins),
            );
            for _ in 0..40 {
                q.step(&[], obs(&goblins));
            }
        }
        assert_eq!(a.digest(), b.digest(), "same pieces, same digest");
        assert_eq!(a.slammed_poses().count(), 0);
    }

    /// The overlay and the budget are cosmetic: over every CF02 fixture with the
    /// corridor kit, the authoritative hash and the event log are identical at
    /// presets 1 to 4, with the overlay on or off; each Q21 pose is picked from
    /// the real events, and `--demo`'s legacy presenter never picks one.
    #[test]
    fn q21_q36_real_cf02_runs_pick_the_poses_and_keep_every_hash() {
        use crate::cf02_fixtures::CF02_FIXTURES;
        use crate::party_scene::PartyFixtureSim;
        let corridor = EnvKit::load(&root(), LAYOUT).unwrap();
        let arena = crate::cf02::arena::load_arena(&root()).unwrap();
        let mut picked: Vec<(&str, SlamPose)> = Vec::new();
        for fixture in CF02_FIXTURES.iter().filter(|f| f.base.boss.is_none()) {
            let kit = if fixture.base.room == crate::combat_scene::FixtureRoom::Arena {
                &arena
            } else {
                &corridor
            };
            let mut reference: Option<(u64, Vec<CombatEvent>)> = None;
            for preset in GorePreset::ALL {
                for overlay in [false, true] {
                    let mut sim = PartyFixtureSim::new(kit, 360, fixture).unwrap();
                    let mut p = if overlay {
                        cf02_presenter(preset)
                    } else {
                        presenter(preset)
                    };
                    // The pose each defeat must get: `slam_pose_for` over the
                    // events logged on the defeat's own tick.
                    let mut expected: Vec<(u32, SlamPose)> = Vec::new();
                    for _ in 0..fixture.base.ticks {
                        let before = sim.combat.events.len();
                        sim.step_scripted();
                        p.observe(&sim.combat, &sim.scene);
                        let fresh = &sim.combat.events[before.min(sim.combat.events.len())..];
                        for event in fresh {
                            if let CombatEvent::EnemyDefeated { target, force, .. } = *event
                                && force >= ForceTier::Heavy
                            {
                                expected.push((target, crate::death::slam_pose_for(fresh, target)));
                            }
                        }
                    }
                    let run = (sim.state_hash(), sim.combat.events.clone());
                    match &reference {
                        None => reference = Some(run),
                        Some(first) => assert_eq!(
                            &run,
                            first,
                            "{} preset {} overlay {overlay}",
                            fixture.name(),
                            preset.level()
                        ),
                    }
                    if !overlay {
                        assert_eq!(p.slammed_poses().count(), 0, "{}", fixture.name());
                    } else if preset == GorePreset::Full {
                        let seen: Vec<(u32, SlamPose)> =
                            p.slammed_poses().map(|v| (v.target, v.pose)).collect();
                        for (target, pose) in &seen {
                            assert!(
                                expected.contains(&(*target, *pose)),
                                "{}: {target} {pose:?} vs {expected:?}",
                                fixture.name()
                            );
                            picked.push((fixture.name(), *pose));
                        }
                    }
                }
            }
        }
        for pose in SlamPose::ALL {
            assert!(
                picked.iter().any(|(_, p)| *p == pose),
                "no CF02 fixture ends in a {} finish: {picked:?}",
                pose.name()
            );
        }
        let names = |pose: SlamPose| -> Vec<&str> {
            let mut v: Vec<&str> = picked
                .iter()
                .filter(|(_, p)| *p == pose)
                .map(|(n, _)| *n)
                .collect();
            v.dedup();
            v
        };
        assert!(names(SlamPose::WallCrumple).contains(&"cf02_wall_slam_kill"));
    }

    // ============================================================= ENG-02 ====

    fn engineer_art() -> Arc<DeathArt> {
        Arc::new(DeathArt::load_engineer(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap())
    }

    /// The cf02 presenter with the Engineer death art and goblin 0 an Engineer.
    fn engineer_presenter(preset: GorePreset) -> DeathPresenter {
        let mut p = cf02_presenter(preset)
            .with_engineer(engineer_art())
            .unwrap();
        p.engineers = vec![0];
        p
    }

    /// Part ids of `p`, each read from the art of its own set.
    fn set_part_ids(p: &DeathPresenter) -> Vec<String> {
        p.parts()
            .map(|(_, part)| p.art_of(part.variant).parts[part.meta].id.clone())
            .collect()
    }

    fn engineer_defeat(p: &mut DeathPresenter, cause: DefeatCause, force: ForceTier, ticks: u64) {
        let goblins = [goblin_at(484.0)];
        p.step(&[defeat(139, 0, cause, force)], obs(&goblins));
        for _ in 1..ticks {
            p.step(&[], obs(&goblins));
        }
    }

    #[test]
    fn e2_6_each_engineer_recipe_resolves_to_its_finish() {
        let profiles = DeathProfiles::cf02();
        let melee = DefeatCause::Melee;
        let cases: [(DefeatCause, ForceTier, &str, Finish); 5] = [
            (
                melee,
                ForceTier::Heavy,
                "engineer_slammed",
                Finish::GoblinSlammed(SlamPose::Flung),
            ),
            (
                BLAST_PB,
                ForceTier::Catastrophic,
                "engineer_exploded",
                Finish::Exploded,
            ),
            (
                EDGE,
                ForceTier::Heavy,
                "engineer_decapitated",
                Finish::Decapitated,
            ),
            (
                BLOCK,
                ForceTier::Routine,
                "engineer_crushed",
                Finish::Crushed,
            ),
            (
                melee,
                ForceTier::Routine,
                "engineer_knockback_down",
                Finish::Knockback,
            ),
        ];
        for (cause, force, id, finish) in cases {
            let selection = profiles.select(ENGINEER_SPECIES, cause, force);
            assert_eq!(selection.finish, id, "{cause:?} {force:?}");
            assert_eq!(Finish::from_id(id), finish, "{id}");
        }
        // The species fallback is the routine knockback, never a removed id.
        assert_eq!(
            profiles.species(ENGINEER_SPECIES).unwrap().fallback_finish,
            "engineer_knockback_down"
        );
    }

    /// Whether any text file under `dir` contains `needle`.
    fn tree_contains(dir: &Path, needle: &str, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                tree_contains(&path, needle, found);
                continue;
            }
            let text_like = matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("rs" | "json" | "toml" | "ron" | "txt" | "md" | "csv")
            );
            if text_like && std::fs::read_to_string(&path).is_ok_and(|text| text.contains(needle)) {
                found.push(path);
            }
        }
    }

    #[test]
    fn e2_6_the_removed_pre_eng02_finish_id_is_gone_from_src_and_assets() {
        // The needle is built at run time so this test never contains it.
        let needle = ["engineer", "down"].join("_");
        let mut found = Vec::new();
        for dir in ["src", "assets"] {
            tree_contains(&root().join(dir), &needle, &mut found);
        }
        assert!(found.is_empty(), "{found:?}");
        assert_eq!(Finish::from_id(&needle), Finish::NoArt);
    }

    #[test]
    fn an_engineer_death_resolves_the_engineer_sheets_not_a_goblin_variant() {
        let mut p = engineer_presenter(GorePreset::Full);
        assert!(p.has_engineer());
        // The set: 27 parts and the pot and wrench, against the goblin's 26.
        assert_eq!(p.art_of(ENGINEER_VARIANT).parts.len(), 27);
        assert_eq!(p.art_of(0).parts.len(), 26);
        assert!(p.art_of(ENGINEER_VARIANT).part_index("pot").is_some());
        assert!(p.art_of(0).part_index("pot").is_none());
        assert!(!std::ptr::eq(p.sheets_of(ENGINEER_VARIANT), p.sheets_of(0)));
        assert_eq!(p.sheets_of(ENGINEER_VARIANT).parts.frames, 27);
        // A crush of the Engineer plays the Engineer's poses.
        engineer_defeat(&mut p, BLOCK, ForceTier::Routine, 4);
        let pose = p.pose_of(0).expect("the crush pose");
        assert_eq!(pose.variant, ENGINEER_VARIANT);
        // Without the Engineer art he plays the goblin set of his index (never a
        // silent clamp onto another set's sheets).
        let mut plain = cf02_presenter(GorePreset::Full);
        plain.engineers = vec![0];
        engineer_defeat(&mut plain, BLOCK, ForceTier::Routine, 4);
        assert_eq!(plain.pose_of(0).unwrap().variant, goblin_variant(0));
        assert!(std::ptr::eq(
            plain.sheets_of(ENGINEER_VARIANT),
            plain.sheets_of(0)
        ));
        // The goblin and the llama keep their own.
        assert!(!std::ptr::eq(p.sheets_of(1), p.sheets_of(0)));
    }

    #[test]
    fn e2_7_an_engineer_crush_launches_one_wrench_and_no_club() {
        for preset in GorePreset::ALL {
            let mut p = engineer_presenter(preset);
            engineer_defeat(&mut p, BLOCK, ForceTier::Routine, 30);
            let ids = set_part_ids(&p);
            assert_eq!(count(&ids, "wrench"), 1, "{preset:?}");
            assert_eq!(count(&ids, "club"), 0, "{preset:?}");
            assert_eq!(
                count(&ids, "pot"),
                0,
                "the flat pot is in the pose, not a part"
            );
        }
        // A goblin crush is unchanged: one club, no wrench.
        let goblin = run_crush(GorePreset::Full, 30);
        assert_eq!(count(&part_ids(&goblin), "club"), 1);
        assert_eq!(count(&part_ids(&goblin), "wrench"), 0);
    }

    #[test]
    fn e2_7_an_engineer_decapitation_throws_the_potted_head() {
        for preset in GorePreset::ALL {
            let mut p = engineer_presenter(preset);
            engineer_defeat(&mut p, EDGE, ForceTier::Heavy, 20);
            let ids = set_part_ids(&p);
            assert_eq!(count(&ids, "head"), 1, "{preset:?}");
            assert_eq!(count(&ids, "head_bare"), 0, "{preset:?}");
            assert_eq!(count(&ids, "club"), 0, "{preset:?}");
        }
        // The head part is the potted one: it fills the cell where the bare head does not.
        let p = engineer_presenter(GorePreset::Full);
        let width = |id: &str| {
            let art = p.art_of(ENGINEER_VARIANT);
            let frame = art.parts[art.part_index(id).unwrap()].frame;
            p.sheets_of(ENGINEER_VARIANT).parts.bbox[frame]
                .unwrap()
                .width()
        };
        assert!(width("head") >= 30 && width("head_bare") < width("head"));
    }

    #[test]
    fn e2_7_an_engineer_explosion_throws_the_pot_and_the_bare_head() {
        let charges = charge_484();
        for preset in GorePreset::ALL {
            let mut p = engineer_presenter(preset);
            let goblins = [GoblinView {
                mode: GoblinMode::Airborne,
                ..goblin_at(484.0)
            }];
            let ob = Observation {
                charges: &charges,
                env_origins: &[],
                ..obs(&goblins)
            };
            p.step(&[defeat(60, 0, BLAST_PB, ForceTier::Catastrophic)], ob);
            for _ in 1..40 {
                p.step(&[], ob);
            }
            let ids = set_part_ids(&p);
            let table = explosion_table_of(ENGINEER_VARIANT, preset);
            for part in table.parts {
                assert!(count(&ids, part) >= 1, "{preset:?} {part}: {ids:?}");
            }
            for banned in ["head", "club", "club_stick", "pineapple"] {
                assert_eq!(count(&ids, banned), 0, "{preset:?} {banned}");
            }
            if preset >= GorePreset::Reduced {
                assert_eq!(count(&ids, "pot"), 1, "{preset:?}");
                assert_eq!(count(&ids, "head_bare"), 1, "{preset:?}");
                assert_eq!(count(&ids, "wrench"), 1, "{preset:?}");
            }
        }
        // The lists are the spec's; preset 4 has ten parts like the goblin's.
        assert_eq!(
            ENGINEER_EXPLOSION_TABLES[3].parts.len(),
            EXPLOSION_TABLES[3].parts.len()
        );
        assert!(ENGINEER_EXPLOSION_TABLES[0].parts.is_empty());
        assert_eq!(
            ENGINEER_EXPLOSION_TABLES[1].parts,
            ["head_bare", "torso_upper", "wrench", "pot"]
        );
    }

    /// One slammed Engineer: `lead` events and the observed mode per tick (`airborne` ticks first, for a
    /// flung body); returns, per tick after the defeat, the pots in the world (a launched part) and whether the
    /// base sprite wears one (ENG-01 frame 14 while a flung body is airborne; no other tick, the slammed cel
    /// replaces it).
    fn slam_pots(
        lead: &[CombatEvent],
        airborne: u64,
        ticks: u64,
    ) -> (Vec<(usize, bool)>, DeathPresenter) {
        let mut p = engineer_presenter(GorePreset::Full);
        let mut out = Vec::new();
        let mut events = lead.to_vec();
        events.push(defeat(60, 0, DefeatCause::Melee, ForceTier::Heavy));
        for tick in 0..ticks {
            let mode = if tick < airborne {
                GoblinMode::Airborne
            } else {
                GoblinMode::Down
            };
            let goblins = [GoblinView {
                mode,
                ..goblin_at(484.0)
            }];
            p.step(if tick == 0 { &events } else { &[] }, obs(&goblins));
            let pose = p.slam_pose_of(0);
            let base_pot = pose == Some(SlamPose::Flung) && mode == GoblinMode::Airborne;
            out.push((count(&set_part_ids(&p), "pot"), base_pot));
        }
        (out, p)
    }

    #[test]
    fn at_most_one_pot_is_visible_on_every_tick_of_every_slam_pose() {
        use crate::combat::MoveId;
        let cases: [(&str, &[CombatEvent], u64, SlamPose); 3] = [
            ("stomped", &[hit_on(MoveId::Stomp, 0)], 0, SlamPose::Stomped),
            ("wall", &[slam_on(0)], 0, SlamPose::WallCrumple),
            ("flung", &[], 14, SlamPose::Flung),
        ];
        for (name, lead, airborne, pose) in cases {
            let (pots, p) = slam_pots(lead, airborne, 90);
            assert_eq!(p.slam_pose_of(0), Some(pose), "{name}");
            for (tick, (part_pots, base_pot)) in pots.iter().enumerate() {
                assert!(
                    part_pots + usize::from(*base_pot) <= 1,
                    "{name}: tick {tick} shows {part_pots} part pot(s) and the base pot {base_pot}"
                );
            }
            // The pot is launched exactly once and stays on the floor.
            assert_eq!(pots.last().unwrap().0, 1, "{name}");
            assert!(
                pots.windows(2).all(|w| w[1].0 >= w[0].0),
                "{name}: never launched twice"
            );
            let ids = set_part_ids(&p);
            assert_eq!(
                (count(&ids, "pot"), count(&ids, "wrench")),
                (1, 1),
                "{name}"
            );
            // The flung body keeps its pot through the whole flight (the base sprite) and launches it on the landing.
            if name == "flung" {
                assert!(
                    pots[..14].iter().all(|(n, base)| *n == 0 && *base),
                    "airborne"
                );
                assert_eq!(pots[14].0, 1, "the landing tick launches the pot");
            } else {
                assert_eq!(pots[1].0, 1, "{name}: launched at age 0");
            }
        }
    }

    #[test]
    fn an_engineer_slam_reads_the_launch_points_of_the_manifest() {
        let art = engineer_art();
        let mut poses: Vec<_> = art.slam_launch.keys().cloned().collect();
        poses.sort();
        assert_eq!(poses, ["flung", "stomped", "wall"]);
        // The points lie on the 64 x 56 frame, on the impact cel: head above the belt.
        for hint in art.slam_launch.values() {
            for point in [hint.pot_launch, hint.wrench_launch] {
                assert!((0..64).contains(&point[0]) && (0..56).contains(&point[1]));
            }
        }
        // A goblin set has none.
        assert!(
            DeathArt::load(Path::new(env!("CARGO_MANIFEST_DIR")))
                .unwrap()
                .slam_launch
                .is_empty()
        );
    }

    #[test]
    fn the_engineer_art_leaves_goblin_slams_and_crushes_unchanged() {
        // With the Engineer art loaded, a goblin target (not in `engineers`) draws exactly as before.
        let plain = run_crush(GorePreset::Full, 40).digest();
        let mut with = presenter(GorePreset::Full)
            .with_engineer(engineer_art())
            .unwrap();
        with.engineers = vec![7];
        let goblins = [goblin_at(484.0)];
        with.step(
            &[defeat(139, 0, BLOCK, ForceTier::Catastrophic)],
            obs(&goblins),
        );
        for _ in 1..40 {
            with.step(&[], obs(&goblins));
        }
        assert_eq!(plain, with.digest());
    }

    #[test]
    fn the_engineer_lab_plays_the_named_finish_for_a_jab_kill() {
        for (name, want) in [
            ("crushed", Finish::Crushed),
            ("decapitated", Finish::Decapitated),
            ("exploded", Finish::Exploded),
            ("slammed_stomped", Finish::GoblinSlammed(SlamPose::Stomped)),
            ("slammed_wall", Finish::GoblinSlammed(SlamPose::WallCrumple)),
            ("slammed_flung", Finish::GoblinSlammed(SlamPose::Flung)),
        ] {
            assert_eq!(Finish::lab_engineer(name), Some(want), "{name}");
            let mut p = engineer_presenter(GorePreset::Full).with_lab_engineer(want);
            let goblins = [goblin_at(484.0)];
            // A routine jab: the recipe alone would be the knockback.
            p.step(
                &[defeat(10, 0, DefeatCause::Melee, ForceTier::Routine)],
                obs(&goblins),
            );
            assert_eq!(p.deaths[0].finish, want, "{name}");
            assert_eq!(p.deaths[0].variant, ENGINEER_VARIANT, "{name}");
        }
        assert_eq!(Finish::lab_engineer("knockback"), None);
        assert_eq!(Finish::lab_engineer(""), None);
        // A goblin is untouched by the flag.
        let mut p = presenter(GorePreset::Full).with_lab_engineer(Finish::Crushed);
        let goblins = [goblin_at(484.0)];
        p.step(
            &[defeat(10, 0, DefeatCause::Melee, ForceTier::Routine)],
            obs(&goblins),
        );
        assert_eq!(p.deaths[0].finish, Finish::Knockback);
    }

    // =========================================================== TROG-FX-01 ====

    const TROG_SLOT: u32 = crate::scene_enemy::SLOT_ID_MIN + 2;

    fn trog_arts() -> [Arc<DeathArt>; TROG_KINDS] {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        [0, 1, 2].map(|k| Arc::new(DeathArt::load_trog(root, k).unwrap()))
    }

    /// The presenter the floor run builds (cf02 overlay on) with the troglodyte death sets.
    fn trog_presenter(preset: GorePreset) -> DeathPresenter {
        cf02_presenter(preset).with_trogs(trog_arts()).unwrap()
    }

    /// A troglodyte `species` defeated at x 300 on the floor line 100, run for `ticks` presenter ticks.
    fn trog_defeat(
        p: &mut DeathPresenter,
        species: &str,
        cause: DefeatCause,
        force: ForceTier,
        ticks: u64,
    ) {
        let event = defeat(40, TROG_SLOT, cause, force);
        p.begin_species_death(&event, species, 300.0, 100.0, false, Some(TROG_SLOT));
        for _ in 0..ticks {
            p.step(&[], obs(&[]));
        }
    }

    #[test]
    fn trog_every_kind_has_a_profile_a_death_set_and_a_table() {
        let profiles = DeathProfiles::with_trogs(true);
        for (kind, species) in TROG_SPECIES.iter().enumerate() {
            assert!(profiles.species(species).is_some(), "{species}");
            let variant = trog_variant(species).unwrap();
            assert_eq!(variant, TROG_FIRST_VARIANT + kind);
            // The blast row throws the kind's own parts, every other defeat keeps the down frames.
            let blast = profiles.select(species, BLAST_PB, ForceTier::Catastrophic);
            assert_eq!(blast.finish, format!("{species}_exploded"));
            assert_eq!(Finish::from_id(blast.finish), Finish::Exploded);
            let jab = profiles.select(species, DefeatCause::Melee, ForceTier::Routine);
            assert_eq!(jab.finish, format!("{species}_down"));
            assert_eq!(Finish::from_id(jab.finish), Finish::Knockback);
        }
        assert_eq!(trog_variant("goblin"), None);
        assert_eq!(trog_variant("goblin_engineer"), None);
        // A run without the troglodyte art keeps the profiles it always had (no troglodyte row).
        assert!(
            cf02_presenter(GorePreset::Full)
                .profiles
                .species("trog_basher")
                .is_none()
        );
        let p = trog_presenter(GorePreset::Full);
        assert!(p.has_trogs() && p.cf02_overlay());
        assert!(p.profiles.species("trog_basher").is_some());
        assert!(p.profiles.species("goblin_engineer").is_some());
    }

    #[test]
    fn trog_the_parts_of_every_table_exist_in_the_kinds_sheet_and_fit_its_cell() {
        let p = trog_presenter(GorePreset::Full);
        for (kind, species) in TROG_SPECIES.iter().enumerate() {
            let variant = TROG_FIRST_VARIANT + kind;
            let art = p.art_of(variant);
            assert_eq!(art.variant, variant, "{species}");
            let sheets = p.sheets_of(variant);
            for preset in GorePreset::ALL {
                let table = explosion_table_of(variant, preset);
                for id in table.parts {
                    let at = art
                        .part_index(id)
                        .unwrap_or_else(|| panic!("{species} {preset:?}: no part {id}"));
                    assert!(art.parts[at].frame0_origin.is_some(), "{species} {id}");
                }
                for (id, _) in table.gibs {
                    assert!(art.part_index(id).is_some(), "{species} gib {id}");
                }
                for id in table.rag_ids {
                    assert!(art.part_index(id).is_some(), "{species} rag {id}");
                }
                assert!(art.part_index(table.torso).is_some(), "{species} torso");
            }
            assert!(art.part_index("drop_blood").is_some(), "{species}");
            // Every cut is drawn inside its cell, off the cell edge (the hull needs a margin).
            for (index, meta) in art.parts.iter().enumerate() {
                let bbox = sheets.parts.bbox[meta.frame]
                    .unwrap_or_else(|| panic!("{species} {index}: empty"));
                assert!(
                    bbox.left >= 1
                        && bbox.top >= 1
                        && bbox.right < art.part_cell.0
                        && bbox.bottom < art.part_cell.1,
                    "{species} {}: {bbox:?}",
                    meta.id
                );
            }
        }
    }

    #[test]
    fn trog_a_point_blank_blast_throws_the_kinds_own_parts_and_hides_its_slot() {
        let all = [
            (0usize, "trog_pygmy"),
            (1, "trog_basher"),
            (2, "trog_virtuoso"),
        ];
        for preset in GorePreset::ALL {
            for (kind, species) in all {
                let mut p = trog_presenter(preset);
                trog_defeat(&mut p, species, BLAST_PB, ForceTier::Catastrophic, 40);
                let variant = TROG_FIRST_VARIANT + kind;
                let ids = set_part_ids(&p);
                let table = explosion_table_of(variant, preset);
                for part in table.parts {
                    assert!(
                        count(&ids, part) >= 1,
                        "{species} {preset:?} {part}: {ids:?}"
                    );
                }
                // Only this kind's set draws (never a goblin or another kind's piece).
                assert!(
                    p.parts().all(|(_, part)| part.variant == variant),
                    "{species} {preset:?}"
                );
                // The body is replaced from preset 2 on; preset 1 keeps the sprite.
                p.step(&[], obs(&[]));
                assert_eq!(
                    p.hides(TROG_SLOT),
                    preset >= GorePreset::Reduced,
                    "{species} {preset:?}"
                );
                assert!(!p.hides(TROG_SLOT + 1), "{species} {preset:?}");
            }
        }
    }

    #[test]
    fn trog_a_jab_keeps_the_down_frames_and_adds_blood_in_the_kinds_set() {
        for (kind, species) in TROG_SPECIES.iter().enumerate() {
            let mut p = trog_presenter(GorePreset::Full);
            trog_defeat(&mut p, species, DefeatCause::Melee, ForceTier::Routine, 20);
            assert_eq!(p.deaths[0].finish, Finish::Knockback, "{species}");
            assert_eq!(p.deaths[0].variant, TROG_FIRST_VARIANT + kind, "{species}");
            assert!(!p.hides(TROG_SLOT), "{species}");
            let ids = set_part_ids(&p);
            assert!(count(&ids, "drop_blood") >= 1, "{species}: {ids:?}");
            // Blood and gibs only, never a body part (the sprite stays).
            assert!(
                ids.iter().all(|id| {
                    [
                        "drop_blood",
                        "gib_flesh_a",
                        "gib_flesh_b",
                        "gib_bone",
                        "gib_rag",
                    ]
                    .contains(&id.as_str())
                }),
                "{species}: {ids:?}"
            );
            // A kick (heavy) and a ring blast degrade to the same finish.
            for (cause, force) in [
                (DefeatCause::Melee, ForceTier::Heavy),
                (BLAST_RING, ForceTier::Heavy),
            ] {
                let mut q = trog_presenter(GorePreset::Full);
                trog_defeat(&mut q, species, cause, force, 2);
                assert_eq!(q.deaths[0].finish, Finish::Knockback, "{species} {cause:?}");
            }
        }
    }

    #[test]
    fn trog_without_the_death_art_a_troglodyte_defeat_draws_nothing() {
        let mut p = cf02_presenter(GorePreset::Full);
        let event = defeat(40, TROG_SLOT, BLAST_PB, ForceTier::Catastrophic);
        p.begin_species_death(&event, "trog_basher", 300.0, 100.0, false, Some(TROG_SLOT));
        for _ in 0..30 {
            p.step(&[], obs(&[]));
        }
        assert_eq!(p.deaths.first().map(|d| d.finish), Some(Finish::NoArt));
        assert_eq!(p.live_parts(), 0);
        assert!(!p.hides(TROG_SLOT));
    }

    #[test]
    fn trog_the_presenter_is_deterministic_and_the_set_does_not_move_the_goblin_digest() {
        // The same defeat twice gives the same pieces (the seed is the attempt, tick and target).
        let run = || {
            let mut p = trog_presenter(GorePreset::Full);
            trog_defeat(&mut p, "trog_basher", BLAST_PB, ForceTier::Catastrophic, 60);
            p.parts()
                .map(|(_, part)| (part.meta, part.x.to_bits(), part.y.to_bits()))
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
        // A goblin defeat reads the same with and without the troglodyte sets.
        let goblins = [goblin_at(484.0)];
        let mut a = cf02_presenter(GorePreset::Full);
        let mut b = trog_presenter(GorePreset::Full);
        for p in [&mut a, &mut b] {
            p.step(
                &[defeat(60, 0, DefeatCause::Melee, ForceTier::Heavy)],
                obs(&goblins),
            );
            for _ in 1..30 {
                p.step(&[], obs(&goblins));
            }
        }
        let key = |p: &DeathPresenter| {
            p.parts()
                .map(|(_, part)| (part.meta, part.x.to_bits(), part.y.to_bits()))
                .collect::<Vec<_>>()
        };
        assert_eq!(key(&a), key(&b));
    }
}
