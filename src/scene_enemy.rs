//! ENEMY-SLOT: the per-enemy update slot of the combat scene. Pure, Bevy-free.
//!
//! A new enemy (the Hoarder, the slime, a Rot sticker, the Dozer) is **one
//! module and one data file**: it implements [`SlotEnemy`], registers one row
//! in [`SLOT_KINDS`] and never adds a hook in `combat_scene.rs`, `companion/**`
//! or `contacts_cf02.rs` (integration plan section 3). The glue that calls it
//! is `combat_scene/slot.rs`; this module is the contract, the registry, the
//! roster spawn and the supplier rule.
//!
//! # What the slot gives an enemy
//!
//! * **Hero melee and missiles.** [`SlotEnemy::hurt_rects`] are the hurtboxes.
//!   Carl's active move box, Donut's pounce, Donut's swat (a glance with poise,
//!   as on the boss) and Donut's Magic Missile land through
//!   [`SlotEnemy::apply_hit`] with a [`SlotHit`], one hit per move instance.
//! * **Blasts.** A blast with a non-QA `BlastProfile` reaches the hurtboxes
//!   through CP-B1's victim loop and calls [`SlotEnemy::apply_blast`] with the
//!   [`BlastReach`] band, after the goblins, the llamas and the boss and before
//!   the heroes. Line of sight is evaluated for every target before the blast
//!   changes anything.
//! * **Contacts.** A launched goblin (or a bowling body) that overlaps
//!   [`SlotEnemy::contact_rect`] calls [`SlotEnemy::apply_contact`] once per
//!   launch (`cf02_rules` rooms only: the legacy contact path is not touched).
//! * **Env prop and hazard victims.** `CombatState::slot_env_hits` takes the
//!   hits an env step resolved against [`SlotEnemy::victim_rect`] bodies and
//!   calls [`SlotEnemy::apply_env`]. [`SlotStepCtx::env`] gives read access to
//!   the runtime env solids and prop hit targets ([`EnvView`], empty until
//!   BOIL-C wire fills it).
//! * **Hero hurt.** A step returns [`HeroBlow`]s in [`SlotOut::blows`]. The scene
//!   applies them through the one existing `hurt_hero` path (invulnerability,
//!   the solo last-HP guard, the thrower rule, `HeroHurt` event). Each enemy owns
//!   its `HeroBlow.source` constant, which is above `0xFF` and distinct from every
//!   other source (the plan's distinct-source-id test lists them).
//! * **Defeat.** An enemy emits the existing `CombatEvent::EnemyDefeated` through
//!   [`SlotOut::events`] (`CombatEvent::enemy_defeated`); this step adds no
//!   defeat variant.
//! * **Threat feed.** [`SlotEnemy::threat`] may return the existing
//!   `Option<BossThreat>`; the scene folds it into the single `boss` input at
//!   **both** supply sites. At most one supplier *kind* per room or fixture
//!   ([`SlotSet::new`], [`SlotError::SupplierConflict`]).
//!
//! Target ids: every slot enemy has its own [`SlotEnemy::target_id`], at or
//! above [`SLOT_ID_MIN`] (goblins are `0..n`, llamas follow them, the boss is
//! `0x4255`), distinct within the room. Morale and the death presenter look
//! ids up below the goblin and llama counts, so a slot id is invisible to them.
//!
//! A scene with no slot enemy is the legacy game byte for byte: nothing here
//! runs, mixes into a hash or logs an event.

use std::fmt;
use std::ops::Range;

use crate::blast::{BlastCharge, BlastReach};
use crate::boss::{BOSS_ENEMY_ID, BossThreat, HeroBlow, HeroView};
use crate::camera_lab::room::EnemyKind;
use crate::combat::{CombatEvent, DefeatCause, MoveId, MoveSpec};
use crate::donut_moves::TargetState;
use crate::encounter::{Encounter, RosterEntry, RosterKind};
use crate::env_hazard::{HazardClass, HazardHit, Team};
use crate::party::HeroId;
use crate::prototype::{Block, Rect};

/// The lowest target id of a slot enemy (above every goblin and llama index).
pub const SLOT_ID_MIN: u32 = 0x1000;

/// The lowest `HeroBlow.source` of a slot enemy: the goblin swing sources are
/// the goblin's index, below this.
pub const SLOT_SOURCE_MIN: u32 = 0x100;

// ------------------------------------------------------------------- hits

/// What landed a hero hit on a slot enemy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitSource {
    /// A melee move of Carl (jab, haymaker, push kick, follow kick, air kick).
    CarlMelee(MoveId),
    /// Donut's swat, which glances a boss-class target: `damage` 0, `poise`
    /// from the swat rule, only while the target stands neutral.
    DonutSwat,
    /// Donut's pounce.
    DonutPounce,
    /// Donut's Magic Missile bolt.
    MagicMissile,
}

/// One landed hero hit on a slot enemy (the shape of the boss' `BossHit`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotHit {
    pub source: HitSource,
    /// The move instance (the bolt's cast instance for a missile).
    pub instance: u32,
    /// Damage after the kill-policy glance rule (CA-20) read the enemy's HP.
    pub damage: u16,
    /// Counts toward poise (Donut's swat counts 1 with 0 damage).
    pub poise: u16,
    /// The credited player slot.
    pub player: u32,
    /// The move's hit-stop.
    pub hit_stop: u8,
    /// +1 / -1: the way the hit travels.
    pub facing: i8,
    /// The move, for the defeat event's force tier.
    pub spec: &'static MoveSpec,
}

impl SlotHit {
    /// The defeat event of a lethal hit: the existing `EnemyDefeated`, a melee
    /// cause credited to the hit's player.
    pub fn defeat_event(&self, tick: u64, target: u32) -> CombatEvent {
        CombatEvent::enemy_defeated(
            tick,
            target,
            DefeatCause::Melee,
            Some(self.player),
            Some(self.spec),
        )
    }
}

/// A non-QA blast that reached a slot enemy's hurtbox (CP-B1's victim loop).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotBlast {
    /// The charge slot that went off.
    pub charge_index: u32,
    /// The charge with its profile and owner.
    pub charge: BlastCharge,
    /// The band the closest hurtbox is in.
    pub reach: BlastReach,
    /// +1 / -1: the way the blast throws the body.
    pub away: i8,
    /// The launch instance a living launch carries.
    pub launch: u32,
}

impl SlotBlast {
    /// The defeat cause of a blast kill: `Explosion`, credited to the owner.
    pub fn cause(&self) -> DefeatCause {
        DefeatCause::Explosion {
            charge: self.charge_index,
            player: self.charge.owner,
            point_blank: self.reach == BlastReach::PointBlank,
        }
    }
}

/// A launched goblin (or a bowling body) that overlapped the contact box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotContact {
    /// The launch instance (one contact per launch and target).
    pub launch: u32,
    /// A killed body that bowls, not a living flier.
    pub body: bool,
    /// The goblin that flew.
    pub source: u32,
    /// The player credited with the launch.
    pub instigator: Option<u32>,
    /// The flier's horizontal speed in px/s when the contact began.
    pub speed: f32,
    /// +1 / -1: the way the contact pushes the target.
    pub push: i8,
}

/// Where an env hit on a slot enemy comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvSource {
    /// A hazard volume of the encounter (`HazardSpec::id`).
    Hazard(u16),
    /// An env prop (a falling mass's landing footprint, a drum).
    Prop(u16),
}

/// One env prop or hazard hit on a slot enemy. An env step resolves it once per
/// victim and activation (`HazardSet::resolve` already dedupes by activation
/// token); the slot only routes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotEnvHit {
    /// The slot enemy's target id.
    pub victim: u32,
    pub source: EnvSource,
    pub class: HazardClass,
    pub damage: u8,
    pub push_x: i16,
    /// The player who set the volume or prop off, for kill credit.
    pub initiator: Option<u32>,
    /// The activation token (1-based) of the hit.
    pub activation: u8,
}

impl SlotEnvHit {
    /// A hazard hit on an enemy victim. `None` for a hero victim.
    pub fn from_hazard(hit: HazardHit) -> Option<Self> {
        (hit.victim.team == Team::Enemy).then_some(Self {
            victim: hit.victim.id,
            source: EnvSource::Hazard(hit.hazard),
            class: hit.class,
            damage: hit.damage,
            push_x: hit.push_x,
            initiator: hit.initiator,
            activation: hit.activation,
        })
    }
}

impl SlotEnvHit {
    /// The defeat cause of a lethal env hit: [`DefeatCause::Environment`]
    /// credited to the initiator. `prop` is the prop index of `source`: the
    /// env step always names the **owning prop** (`EnvSource::Prop`), also for
    /// the steam of a valve; a hazard no prop owns carries its hazard id.
    pub fn cause(&self) -> crate::combat::DefeatCause {
        let prop = match self.source {
            EnvSource::Hazard(id) | EnvSource::Prop(id) => id,
        };
        crate::combat::DefeatCause::Environment {
            player: self.initiator,
            prop,
            class: self.class,
        }
    }
}

/// What a hit did to the enemy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotHitResult {
    pub hp_left: u16,
    /// The hit defeated it (the enemy also logged `EnemyDefeated`).
    pub defeated: bool,
}

// ---------------------------------------------------------------- the step

/// What a step (or a hit) asks the scene to do.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SlotOut {
    /// Events to log, in order: `EnemyDefeated` and any variant the enemy's own
    /// step appended to `CombatEvent`.
    pub events: Vec<CombatEvent>,
    /// Blows on heroes, applied through `hurt_hero` in order.
    pub blows: Vec<HeroBlow>,
    /// Corrected hero x (a body or a roar push), applied before the blows.
    pub shifts: Vec<(HeroId, f32)>,
    /// Solo: put this bot hero at x on the leader's height (the boss regroup).
    pub regroup: Vec<(HeroId, f32)>,
}

impl SlotOut {
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
            && self.blows.is_empty()
            && self.shifts.is_empty()
            && self.regroup.is_empty()
    }
}

/// The room facts a slot enemy needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotRoom {
    /// World y of the walking line (feet).
    pub floor_y: f32,
    /// The room's walkable x range.
    pub walls: (f32, f32),
}

/// A runtime env prop hit target a missile or shove can hit (a support post).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvTarget {
    /// Index into the encounter's props.
    pub prop: u16,
    /// World rectangle (y up).
    pub rect: Rect,
    /// Whether the rectangle is also a solid of the runtime env sync.
    pub solid: bool,
}

/// Read access to the env props of the scene (empty until BOIL-C wire fills
/// it): the index range of the runtime env solids inside the scene's solids and
/// the props' hit targets. The slot only reads it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EnvView {
    /// The runtime env solids' index range inside `scene.world.solids`.
    pub solid_range: Range<usize>,
    /// Hit targets, in prop order.
    pub targets: Vec<EnvTarget>,
}

impl EnvView {
    /// The runtime env solids of `all` (empty when the range is unset or out of
    /// bounds: a consumer never panics on a stale range).
    pub fn solids<'a>(&self, all: &'a [Block]) -> &'a [Block] {
        all.get(self.solid_range.clone()).unwrap_or(&[])
    }

    pub fn is_empty(&self) -> bool {
        self.solid_range.is_empty() && self.targets.is_empty()
    }
}

/// What the scene tells a slot enemy each tick.
#[derive(Debug, Clone, Copy)]
pub struct SlotStepCtx<'a> {
    /// The scene tick of this step (attempt-local).
    pub tick: u64,
    pub heroes: [HeroView; 2],
    /// Every solid of the scene, the runtime env solids included.
    pub solids: &'a [Block],
    /// Env props: solids by index range and hit targets.
    pub env: &'a EnvView,
    /// Solo: the bot hero counts farther away.
    pub solo: bool,
    pub room: SlotRoom,
}

// ------------------------------------------------------------- the trait

/// One enemy of the slot. `Send + Sync` because the combat state is a Bevy
/// resource; `box_clone` because the state is `Clone`.
pub trait SlotEnemy: fmt::Debug + Send + Sync {
    /// The registry name of the kind.
    fn kind(&self) -> &'static str;

    /// The enemy's id in `EnemyDefeated.target`, at or above [`SLOT_ID_MIN`].
    fn target_id(&self) -> u32;

    fn box_clone(&self) -> Box<dyn SlotEnemy>;

    /// Alive: stepped, hit and drawn. A defeated enemy is never hit again.
    fn alive(&self) -> bool;

    /// Hit points, for the kill-policy glance rule (clamped to a byte there).
    fn hp(&self) -> u16;

    /// World hurtboxes (y up) for hero melee, missiles and blasts. Empty while
    /// the enemy has no hurtbox (a state in which hits pass through).
    fn hurt_rects(&self) -> Vec<Rect>;

    /// The box a launched body contacts. `None` (the default): not a contact
    /// target.
    fn contact_rect(&self) -> Option<Rect> {
        None
    }

    /// The box an env hit tests, in world y up. The default is the first
    /// hurtbox.
    fn victim_rect(&self) -> Option<Rect> {
        self.hurt_rects().first().copied()
    }

    /// What Donut's swat sees: `Neutral` lets a swat count its poise, any other
    /// state glances it with none. Donut's swat and pounce treat a slot enemy as
    /// a boss-class target.
    fn donut_target_state(&self) -> TargetState {
        TargetState::Neutral
    }

    /// Whether this enemy supplies the single `boss: Option<BossThreat>`
    /// feed. At most one supplier kind per room or fixture.
    fn supplies_boss(&self) -> bool {
        false
    }

    /// The threat it feeds to the follow layer this tick (only while it
    /// supplies the feed).
    fn threat(&self) -> Option<BossThreat> {
        None
    }

    /// The death-species hook: where and what the cosmetic death presenter
    /// draws when this enemy falls. `None` (the default): nothing is drawn. Read
    /// after the defeat tick, so a defeated enemy still reports its body.
    fn death_body(&self) -> Option<SlotDeathBody> {
        None
    }

    /// One tick, after the goblins, the llamas and the globs.
    fn step(&mut self, ctx: &SlotStepCtx<'_>, out: &mut SlotOut);

    /// A landed hero hit (melee, pounce, swat glance, missile). `None`: the
    /// enemy takes no hit now. A lethal hit logs `EnemyDefeated` into `out`.
    fn apply_hit(&mut self, hit: &SlotHit, tick: u64, out: &mut SlotOut) -> Option<SlotHitResult>;

    /// A blast of a non-QA profile reached it. `None` (the default): immune.
    fn apply_blast(
        &mut self,
        _blast: &SlotBlast,
        _tick: u64,
        _out: &mut SlotOut,
    ) -> Option<SlotHitResult> {
        None
    }

    /// A launched body touched it. `None` (the default): no reaction. The
    /// output's scene-bound follow-ups (blows, shifts, regroups) apply on the
    /// **next** tick's slot step (the contacts resolve after the slot step);
    /// its events log at once.
    fn apply_contact(
        &mut self,
        _contact: &SlotContact,
        _tick: u64,
        _out: &mut SlotOut,
    ) -> Option<SlotHitResult> {
        None
    }

    /// An env prop or hazard hit it. `None` (the default): immune.
    fn apply_env(
        &mut self,
        _hit: &SlotEnvHit,
        _tick: u64,
        _out: &mut SlotOut,
    ) -> Option<SlotHitResult> {
        None
    }

    /// The entry gate this enemy owns while it is closed (world px, y up): a
    /// full-height solid the runtime env sync keeps in the scene's solids
    /// (HRD-Q41). The default: no gate.
    fn gate_solid(&self) -> Option<Rect> {
        None
    }

    /// Fixture and test inspection: the concrete enemy behind the trait object
    /// (`downcast_ref`). The default is `None`: nothing is exposed.
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }

    /// A new attempt began: the enemy was just put back to its spawn state by a
    /// defeat retry, a checkpoint respawn or a fixture restart (never called for
    /// the first attempt). An enemy whose retry differs from its first entry (the
    /// Hoarder's short reveal, HRD-Q05) marks itself here. The default: nothing.
    fn on_retry(&mut self) {}

    /// Fold the whole state into a state hash (everything the simulation reads).
    fn mix_into(&self, mix: &mut dyn FnMut(u64));
}

impl Clone for Box<dyn SlotEnemy> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

// ---------------------------------------------------------- the registry

/// What the death presenter needs to draw a slot enemy's fall (render only:
/// it never reaches the simulation). Same convention as the boss' defeat:
/// `ground_y` is the floor line under its feet, `facing` is +1 / -1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotDeathBody {
    pub x: f32,
    pub ground_y: f32,
    pub facing: i8,
    /// The death-profile species key (`DeathProfiles::species`); an unknown key
    /// (or a species whose finish has no art yet) draws nothing, and the
    /// simulation is untouched.
    pub species: &'static str,
}

/// One roster entry as the slot builds it from. Fields the slot does not
/// interpret (`trigger_x`, `delay_ticks`, `perch`) are carried for the enemy's
/// own dormancy rules.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotEntry {
    /// The index in the encounter's roster.
    pub index: usize,
    pub id: String,
    pub x: f32,
    pub perch: Option<usize>,
    pub trigger_x: Option<i32>,
    pub delay_ticks: u32,
}

/// What a kind's builder reads. `encounter` is `Some` for a roster spawn, so a
/// kind reads its own roster fields (`encounter.roster[entry.index]`, for
/// example the slime's `home` once HAZ-A1 has merged) and its props and hazards.
#[derive(Debug, Clone, Copy)]
pub struct SlotBuild<'a> {
    pub entry: &'a SlotEntry,
    pub room: SlotRoom,
    pub encounter: Option<&'a Encounter>,
}

/// One registered kind: a name (the roster `kind`) and its builder.
#[derive(Debug, Clone, Copy)]
pub struct SlotKindDef {
    pub name: &'static str,
    pub build: fn(&SlotBuild<'_>) -> Result<Box<dyn SlotEnemy>, SlotError>,
}

/// The registered slot kinds. A new enemy appends **one row** here. The row's
/// `name` is the roster kind string ([`entry_kind_name`]): `hoarder` for the
/// Hoarder (HRD-A2) and `slime` for the slime (HAZ-A2); a `pile_boss` row would
/// never match. A kind with no [`RosterKind`] variant yet (the Rot Sticker, the
/// Dozer) first appends a variant in `encounter.rs` and an arm in
/// [`entry_kind_name`]. The built enemy must report the row's name from
/// `kind()` and the roster kind's own `supplies_boss` flag from
/// `supplies_boss()`, or the spawn refuses it ([`SlotError::Build`]). A kind
/// that is in no row is rejected by the roster spawn, never silently dropped.
pub static SLOT_KINDS: &[SlotKindDef] = &[
    // HRD-A2: the Hoarder.
    SlotKindDef {
        name: crate::combat_scene::pile_boss::KIND,
        build: crate::combat_scene::pile_boss::build,
    },
];

/// The roster kind names the slot never builds: the existing goblin and
/// Engineer paths spawn them (BOIL-E, K16), and the llama is a fixture kind.
const EXISTING_KINDS: [&str; 3] = ["goblin", "engineer", "llama"];

/// The roster kind name of a room enemy kind (the registry key).
pub fn kind_name(kind: EnemyKind) -> &'static str {
    match kind {
        EnemyKind::Goblin => "goblin",
        EnemyKind::Llama => "llama",
        EnemyKind::Engineer => "engineer",
        EnemyKind::Scatterer => "scatterer",
        EnemyKind::Hoarder => "hoarder",
        EnemyKind::Trog => "trog",
        EnemyKind::Juicer => "juicer",
        EnemyKind::Ball => "ball",
    }
}

/// The registry key of a roster entry: the one place the roster enum
/// ([`RosterKind`], HAZ-A1) meets the registry.
fn entry_kind_name(entry: &RosterEntry) -> &'static str {
    match entry.kind {
        RosterKind::Room(kind) => kind_name(kind),
        RosterKind::Slime => "slime",
    }
}

/// Why a slot could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotError {
    /// A roster entry names a kind that is neither an existing kind nor a
    /// registered slot kind.
    UnknownKind { entry: String, kind: String },
    /// A target id below [`SLOT_ID_MIN`] or equal to the boss' id.
    IdOutOfRange { kind: &'static str, id: u32 },
    /// Two enemies of one room share a target id.
    DuplicateId { id: u32 },
    /// More than one boss-threat supplier kind in one room (the roster
    /// validator rule: the single `boss` feed has one supplier).
    SupplierConflict { kinds: Vec<&'static str> },
    /// A slot kind supplies the boss feed in a fixture that has a boss.
    FixtureBossConflict { kind: &'static str },
    /// A kind's builder refused its entry.
    Build { kind: &'static str, reason: String },
}

impl fmt::Display for SlotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownKind { entry, kind } => {
                write!(
                    f,
                    "roster {entry}: kind {kind:?} is not a registered slot kind"
                )
            }
            Self::IdOutOfRange { kind, id } => write!(
                f,
                "{kind}: target id {id:#x} must be at least {SLOT_ID_MIN:#x} and not the boss id"
            ),
            Self::DuplicateId { id } => write!(f, "two slot enemies share the target id {id:#x}"),
            Self::SupplierConflict { kinds } => write!(
                f,
                "more than one boss-threat supplier kind in one room: {}",
                kinds.join(", ")
            ),
            Self::FixtureBossConflict { kind } => write!(
                f,
                "{kind} supplies the boss threat feed but the fixture already has a boss"
            ),
            Self::Build { kind, reason } => write!(f, "{kind}: {reason}"),
        }
    }
}

impl std::error::Error for SlotError {}

/// The roster rule: the distinct supplier kinds among `kinds` (a `(name,
/// supplies)` pair per enemy), in first-seen order. Two enemies of one kind are
/// one supplier kind.
pub fn supplier_kinds(kinds: &[(&'static str, bool)]) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for (name, supplies) in kinds {
        if *supplies && !out.contains(name) {
            out.push(name);
        }
    }
    out
}

/// The rule as a result: `Err(SupplierConflict)` for more than one supplier
/// kind. HAZ-A1's rule 9 (`supplier_conflict`) is the load-time form of the same
/// rule on the sidecar roster; this one also sees a fixture boss and runs at
/// spawn.
pub fn check_suppliers(
    kinds: &[(&'static str, bool)],
    fixture_boss: bool,
) -> Result<(), SlotError> {
    let suppliers = supplier_kinds(kinds);
    if suppliers.len() > 1 {
        return Err(SlotError::SupplierConflict { kinds: suppliers });
    }
    if fixture_boss && let Some(kind) = suppliers.first() {
        return Err(SlotError::FixtureBossConflict { kind });
    }
    Ok(())
}

/// The enemies of one room, validated.
#[derive(Debug, Clone, Default)]
pub struct SlotSet {
    enemies: Vec<Box<dyn SlotEnemy>>,
}

impl SlotSet {
    /// Validate `enemies`: ids in range and distinct, at most one boss-threat
    /// supplier kind, none when the fixture has a boss. Never silently skips a
    /// rule.
    pub fn new(enemies: Vec<Box<dyn SlotEnemy>>, fixture_boss: bool) -> Result<Self, SlotError> {
        let mut seen: Vec<u32> = Vec::new();
        for e in &enemies {
            let id = e.target_id();
            if id < SLOT_ID_MIN || id == BOSS_ENEMY_ID {
                return Err(SlotError::IdOutOfRange { kind: e.kind(), id });
            }
            if seen.contains(&id) {
                return Err(SlotError::DuplicateId { id });
            }
            seen.push(id);
        }
        let kinds: Vec<(&'static str, bool)> = enemies
            .iter()
            .map(|e| (e.kind(), e.supplies_boss()))
            .collect();
        check_suppliers(&kinds, fixture_boss)?;
        Ok(Self { enemies })
    }

    pub fn is_empty(&self) -> bool {
        self.enemies.is_empty()
    }

    pub fn len(&self) -> usize {
        self.enemies.len()
    }

    pub(crate) fn into_enemies(self) -> Vec<Box<dyn SlotEnemy>> {
        self.enemies
    }
}

/// The spawn of one sidecar roster.
#[derive(Debug, Clone, Default)]
pub struct RosterSpawn {
    /// The slot enemies, in roster order.
    pub slot: SlotSet,
    /// The roster indices of the existing kinds (goblin, Engineer, llama): the
    /// slot does not build them (K16, BOIL-E).
    pub existing: Vec<usize>,
}

/// Spawn the slot enemies of `encounter`'s roster from the registered kinds
/// ([`SLOT_KINDS`]).
pub fn spawn_from_roster(
    encounter: &Encounter,
    room: SlotRoom,
    fixture_boss: bool,
) -> Result<RosterSpawn, SlotError> {
    spawn_with(SLOT_KINDS, encounter, room, fixture_boss)
}

/// [`spawn_from_roster`] over an explicit kind table (tests, and a lab that
/// registers its own kinds).
pub fn spawn_with(
    table: &[SlotKindDef],
    encounter: &Encounter,
    room: SlotRoom,
    fixture_boss: bool,
) -> Result<RosterSpawn, SlotError> {
    // The roster rule first, on the roster's own data (`RosterKind::supplies_boss`,
    // the flag HAZ rule 9 reads at load time), so it holds even for a kind that no
    // module has registered yet. The built set checks it again, and against the
    // fixture boss.
    let roster_kinds: Vec<(&'static str, bool)> = encounter
        .roster
        .iter()
        .map(|r| (entry_kind_name(r), r.kind.supplies_boss()))
        .collect();
    check_suppliers(&roster_kinds, fixture_boss)?;
    let mut enemies: Vec<Box<dyn SlotEnemy>> = Vec::new();
    let mut existing = Vec::new();
    for (index, r) in encounter.roster.iter().enumerate() {
        let name = entry_kind_name(r);
        if EXISTING_KINDS.contains(&name) {
            existing.push(index);
            continue;
        }
        let Some(def) = table.iter().find(|d| d.name == name) else {
            return Err(SlotError::UnknownKind {
                entry: r.id.clone(),
                kind: name.to_owned(),
            });
        };
        let entry = SlotEntry {
            index,
            id: r.id.clone(),
            x: r.x as f32,
            perch: r.perch,
            trigger_x: r.trigger_x,
            delay_ticks: r.delay_ticks,
        };
        let built = (def.build)(&SlotBuild {
            entry: &entry,
            room,
            encounter: Some(encounter),
        })?;
        // The row and the roster must agree with what the builder made, so the
        // roster-level and the set-level supplier checks can never disagree.
        if built.kind() != def.name {
            return Err(SlotError::Build {
                kind: def.name,
                reason: format!(
                    "roster {}: the built enemy reports kind {:?}, not the registered name",
                    r.id,
                    built.kind()
                ),
            });
        }
        if built.supplies_boss() != r.kind.supplies_boss() {
            return Err(SlotError::Build {
                kind: def.name,
                reason: format!(
                    "roster {}: the built enemy's supplies_boss is {}, the roster kind's is {}",
                    r.id,
                    built.supplies_boss(),
                    r.kind.supplies_boss()
                ),
            });
        }
        enemies.push(built);
    }
    Ok(RosterSpawn {
        slot: SlotSet::new(enemies, fixture_boss)?,
        existing,
    })
}

/// How close two hurtbox bands rank: the strongest band of `charge` over
/// `rects`, `None` when no rect is in reach.
pub fn best_reach(charge: &BlastCharge, rects: &[Rect]) -> Option<BlastReach> {
    let rank = |r: BlastReach| match r {
        BlastReach::PointBlank => 0,
        BlastReach::Ring => 1,
        BlastReach::Outer => 2,
    };
    rects
        .iter()
        .filter_map(|r| charge.reach(*r))
        .min_by_key(|r| rank(*r))
}

// ------------------------------------------------------- the test enemy

/// A minimal slot enemy that proves the slot end to end. Test-only: no real
/// enemy is wired here (HRD-A2 and HAZ-A2 wire the Hoarder and the slime).
#[cfg(test)]
pub(crate) mod testing {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::combat::{carl_move, hit_box};

    /// The test dummy's `HeroBlow` source ("ST").
    pub const DUMMY_SOURCE: u32 = 0x5354;

    /// What the dummy saw, shared with the test that built it (the box is
    /// cloned by the combat state, so a test reads this log instead of the box).
    #[derive(Debug, Default)]
    pub struct DummyLog {
        pub hits: Vec<(u64, HitSource, u16, u16)>,
        pub blasts: Vec<(u64, BlastReach, i8)>,
        pub contacts: Vec<(u64, SlotContact)>,
        pub env: Vec<(u64, SlotEnvHit)>,
        pub steps: Vec<u64>,
        /// `(tick, solids, env solids, env targets, present heroes)` per step.
        pub seen: Vec<(u64, usize, usize, usize, usize)>,
        /// The tick and the x the dummy swung at.
        pub swings: Vec<(u64, f32)>,
    }

    #[derive(Debug, Clone)]
    pub struct Dummy {
        pub kind: &'static str,
        pub id: u32,
        pub x: f32,
        pub foot_y: f32,
        pub hp_max: u16,
        pub hp: u16,
        pub half_w: f32,
        pub height: f32,
        /// Swing at a hero within this many px (0: never swings).
        pub reach: f32,
        pub cooldown: u64,
        pub next_swing: u64,
        pub supplies: bool,
        pub threat: Option<BossThreat>,
        /// Immune to blasts, contacts and env hits when false.
        pub victim: bool,
        /// No hurtbox while true (a state in which hits pass through).
        pub hidden: bool,
        /// What Donut's swat sees.
        pub target_state: TargetState,
        /// Falls without logging `EnemyDefeated` (a defect the scene must catch).
        pub silent_death: bool,
        /// The death presenter's species (`None`: nothing is drawn).
        pub death_species: Option<&'static str>,
        /// A contact reaction asks for a blow on Carl (a scene-bound follow-up).
        pub contact_blow: bool,
        pub log: Arc<Mutex<DummyLog>>,
    }

    impl Dummy {
        pub fn new(id: u32, x: f32, foot_y: f32) -> Self {
            Self {
                kind: "test_dummy",
                id,
                x,
                foot_y,
                hp_max: 4,
                hp: 4,
                half_w: 12.0,
                height: 52.0,
                reach: 0.0,
                cooldown: 30,
                next_swing: 0,
                supplies: false,
                threat: None,
                victim: true,
                hidden: false,
                target_state: TargetState::Neutral,
                silent_death: false,
                death_species: None,
                contact_blow: false,
                log: Arc::default(),
            }
        }

        pub fn boxed(self) -> Box<dyn SlotEnemy> {
            Box::new(self)
        }

        pub fn boxed_pair(self) -> (Box<dyn SlotEnemy>, Arc<Mutex<DummyLog>>) {
            let log = self.log.clone();
            (Box::new(self), log)
        }

        fn body(&self) -> Rect {
            Rect {
                left: self.x - self.half_w,
                bottom: self.foot_y,
                width: self.half_w * 2.0,
                height: self.height,
            }
        }

        fn take(&mut self, damage: u16) -> SlotHitResult {
            self.hp = self.hp.saturating_sub(damage);
            SlotHitResult {
                hp_left: self.hp,
                defeated: self.hp == 0,
            }
        }
    }

    impl SlotEnemy for Dummy {
        fn kind(&self) -> &'static str {
            self.kind
        }
        fn target_id(&self) -> u32 {
            self.id
        }
        fn box_clone(&self) -> Box<dyn SlotEnemy> {
            Box::new(self.clone())
        }
        fn alive(&self) -> bool {
            self.hp > 0
        }
        fn hp(&self) -> u16 {
            self.hp
        }
        fn hurt_rects(&self) -> Vec<Rect> {
            if self.alive() && !self.hidden {
                vec![self.body()]
            } else {
                Vec::new()
            }
        }
        fn contact_rect(&self) -> Option<Rect> {
            (self.alive() && self.victim).then(|| self.body())
        }
        fn donut_target_state(&self) -> TargetState {
            self.target_state
        }
        fn death_body(&self) -> Option<SlotDeathBody> {
            self.death_species.map(|species| SlotDeathBody {
                x: self.x,
                ground_y: self.foot_y,
                facing: -1,
                species,
            })
        }
        fn supplies_boss(&self) -> bool {
            self.supplies
        }
        fn threat(&self) -> Option<BossThreat> {
            self.threat
        }
        fn step(&mut self, ctx: &SlotStepCtx<'_>, out: &mut SlotOut) {
            let mut log = self.log.lock().unwrap();
            log.steps.push(ctx.tick);
            let present = ctx.heroes.iter().filter(|h| h.present).count();
            log.seen.push((
                ctx.tick,
                ctx.solids.len(),
                ctx.env.solids(ctx.solids).len(),
                ctx.env.targets.len(),
                present,
            ));
            if !self.alive() || self.reach <= 0.0 || ctx.tick < self.next_swing {
                return;
            }
            for h in &ctx.heroes {
                // Swings at any present hero: invulnerability is the scene's hurt path.
                if h.present && (h.x - self.x).abs() <= self.reach {
                    self.next_swing = ctx.tick + self.cooldown;
                    log.swings.push((ctx.tick, h.x));
                    out.blows.push(HeroBlow {
                        hero: h.id,
                        source: DUMMY_SOURCE,
                        damage: 1,
                        from_x: self.x,
                        knock_vx: 0.0,
                        knock_vy: 0.0,
                    });
                    break;
                }
            }
        }
        fn apply_hit(
            &mut self,
            hit: &SlotHit,
            tick: u64,
            out: &mut SlotOut,
        ) -> Option<SlotHitResult> {
            if !self.alive() || self.hidden {
                return None;
            }
            self.log
                .lock()
                .unwrap()
                .hits
                .push((tick, hit.source, hit.damage, hit.poise));
            let result = self.take(hit.damage);
            if result.defeated && !self.silent_death {
                out.events.push(hit.defeat_event(tick, self.id));
            }
            Some(result)
        }
        fn apply_blast(
            &mut self,
            blast: &SlotBlast,
            tick: u64,
            out: &mut SlotOut,
        ) -> Option<SlotHitResult> {
            if !self.alive() || !self.victim {
                return None;
            }
            self.log
                .lock()
                .unwrap()
                .blasts
                .push((tick, blast.reach, blast.away));
            let damage = match blast.reach {
                BlastReach::PointBlank | BlastReach::Ring => self.hp,
                BlastReach::Outer => 1,
            };
            let result = self.take(damage);
            if result.defeated {
                out.events.push(CombatEvent::enemy_defeated(
                    tick,
                    self.id,
                    blast.cause(),
                    blast.charge.owner,
                    None,
                ));
            }
            Some(result)
        }
        fn apply_contact(
            &mut self,
            contact: &SlotContact,
            tick: u64,
            out: &mut SlotOut,
        ) -> Option<SlotHitResult> {
            if !self.alive() || !self.victim {
                return None;
            }
            self.log.lock().unwrap().contacts.push((tick, *contact));
            if self.contact_blow {
                out.blows.push(HeroBlow {
                    hero: HeroId::Carl,
                    source: DUMMY_SOURCE,
                    damage: 1,
                    from_x: self.x,
                    knock_vx: 0.0,
                    knock_vy: 0.0,
                });
            }
            Some(self.take(0))
        }
        fn apply_env(
            &mut self,
            hit: &SlotEnvHit,
            tick: u64,
            out: &mut SlotOut,
        ) -> Option<SlotHitResult> {
            if !self.alive() || !self.victim {
                return None;
            }
            self.log.lock().unwrap().env.push((tick, *hit));
            let result = self.take(u16::from(hit.damage));
            if result.defeated {
                out.events.push(CombatEvent::enemy_defeated(
                    tick,
                    self.id,
                    DefeatCause::Melee,
                    hit.initiator,
                    carl_move(MoveId::Jab),
                ));
            }
            Some(result)
        }
        fn mix_into(&self, mix: &mut dyn FnMut(u64)) {
            mix(u64::from(self.id));
            mix(u64::from(self.x.to_bits()));
            mix(u64::from(self.hp));
            mix(self.next_swing);
        }
    }

    /// The world box of Carl's jab at x facing right, for geometry tests.
    pub fn jab_reach(x: f32, y: f32, facing: i8) -> Option<Rect> {
        let spec = carl_move(MoveId::Jab)?;
        hit_box(
            &spec.shape,
            &crate::combat::Attacker {
                x,
                y,
                half_height: crate::prototype::HERO_HALF_HEIGHT,
                facing,
                player: 0,
            },
        )
    }

    pub fn from_entry(b: &SlotBuild<'_>, kind: &'static str, supplies: bool) -> Box<dyn SlotEnemy> {
        let mut d = Dummy::new(
            SLOT_ID_MIN + b.entry.index as u32,
            b.entry.x,
            b.room.floor_y,
        );
        d.kind = kind;
        d.supplies = supplies;
        d.boxed()
    }

    /// A kind table for the roster-spawn tests, consistent with the roster
    /// kinds' own `supplies_boss` flags: `trog` is quiet, `hoarder` and `slime`
    /// supply the boss threat.
    pub fn table() -> [SlotKindDef; 3] {
        [
            SlotKindDef {
                name: "trog",
                build: |b| Ok(from_entry(b, "trog", false)),
            },
            SlotKindDef {
                name: "hoarder",
                build: |b| Ok(from_entry(b, "hoarder", true)),
            },
            SlotKindDef {
                name: "slime",
                build: |b| Ok(from_entry(b, "slime", true)),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;

    fn dummy(id: u32) -> Dummy {
        Dummy::new(id, 100.0, 0.0)
    }

    #[test]
    fn the_supplier_rule_counts_kinds_not_enemies() {
        // Two enemies of one kind are one supplier kind.
        let one = [("pile", true), ("pile", true), ("goblin", false)];
        assert_eq!(supplier_kinds(&one), ["pile"]);
        assert_eq!(check_suppliers(&one, false), Ok(()));
        // Two supplier kinds are rejected, in first-seen order.
        let two = [("slime", true), ("pile", true)];
        assert_eq!(
            check_suppliers(&two, false),
            Err(SlotError::SupplierConflict {
                kinds: vec!["slime", "pile"]
            })
        );
        // A kind that supplies nothing never conflicts.
        let none = [("sticker", false), ("pile", true)];
        assert_eq!(check_suppliers(&none, false), Ok(()));
        // The fixture boss counts as a supplier: a slot supplier next to it is rejected,
        // a slot without a supplier is not.
        assert_eq!(
            check_suppliers(&none, true),
            Err(SlotError::FixtureBossConflict { kind: "pile" })
        );
        assert_eq!(check_suppliers(&[("sticker", false)], true), Ok(()));
        assert_eq!(check_suppliers(&[], true), Ok(()));
    }

    #[test]
    fn a_set_rejects_two_supplier_kinds_and_a_supplier_beside_a_fixture_boss() {
        let supplier = |kind: &'static str, id: u32| {
            let mut d = dummy(id);
            d.kind = kind;
            d.supplies = true;
            d.boxed()
        };
        let err = SlotSet::new(
            vec![supplier("a", SLOT_ID_MIN), supplier("b", SLOT_ID_MIN + 1)],
            false,
        )
        .unwrap_err();
        assert_eq!(
            err,
            SlotError::SupplierConflict {
                kinds: vec!["a", "b"]
            }
        );
        assert!(err.to_string().contains("a, b"));
        // Two of one kind are fine.
        assert!(
            SlotSet::new(
                vec![supplier("a", SLOT_ID_MIN), supplier("a", SLOT_ID_MIN + 1)],
                false
            )
            .is_ok()
        );
        // A supplier beside a fixture boss is rejected, a quiet enemy is not.
        assert_eq!(
            SlotSet::new(vec![supplier("a", SLOT_ID_MIN)], true).unwrap_err(),
            SlotError::FixtureBossConflict { kind: "a" }
        );
        assert!(SlotSet::new(vec![dummy(SLOT_ID_MIN).boxed()], true).is_ok());
    }

    #[test]
    fn target_ids_are_in_range_and_distinct() {
        // The boundary: the lowest id is accepted, one below it is not.
        assert!(SlotSet::new(vec![dummy(SLOT_ID_MIN).boxed()], false).is_ok());
        assert_eq!(
            SlotSet::new(vec![dummy(SLOT_ID_MIN - 1).boxed()], false).unwrap_err(),
            SlotError::IdOutOfRange {
                kind: "test_dummy",
                id: SLOT_ID_MIN - 1
            }
        );
        assert!(matches!(
            SlotSet::new(vec![dummy(BOSS_ENEMY_ID).boxed()], false),
            Err(SlotError::IdOutOfRange { .. })
        ));
        assert_eq!(
            SlotSet::new(
                vec![
                    dummy(SLOT_ID_MIN + 4).boxed(),
                    dummy(SLOT_ID_MIN + 4).boxed()
                ],
                false
            )
            .unwrap_err(),
            SlotError::DuplicateId {
                id: SLOT_ID_MIN + 4
            }
        );
        assert!(SlotSet::new(Vec::new(), true).unwrap().is_empty());
    }

    #[test]
    fn the_blast_band_is_the_strongest_of_the_hurtboxes() {
        use crate::blast::BlastProfile;
        let profile = BlastProfile {
            core_radius: 20.0,
            outer_radius: 60.0,
            ..BlastProfile::QA
        };
        let charge = BlastCharge::new(100.0, 0.0, 1, None).with_profile(profile);
        let (cx, cy) = charge.centre();
        let rect_at = |dx: f32| Rect {
            left: cx + dx,
            bottom: cy - 5.0,
            width: 10.0,
            height: 10.0,
        };
        // Centre inside the first box: point blank beats the other box's ring.
        assert_eq!(
            best_reach(&charge, &[rect_at(30.0), rect_at(-5.0)]),
            Some(BlastReach::PointBlank)
        );
        assert_eq!(
            best_reach(&charge, &[rect_at(30.0)]),
            Some(BlastReach::Outer)
        );
        assert_eq!(
            best_reach(&charge, &[rect_at(15.0)]),
            Some(BlastReach::Ring)
        );
        // Outside the outer radius, and with no rect at all.
        assert_eq!(best_reach(&charge, &[rect_at(61.0)]), None);
        assert_eq!(best_reach(&charge, &[]), None);
    }

    #[test]
    fn an_env_hit_routes_only_enemy_victims() {
        use crate::env_hazard::Victim;
        let hit = |victim| HazardHit {
            hazard: 3,
            class: HazardClass::Acid,
            victim,
            activation: 2,
            damage: 1,
            push_x: -40,
            initiator: Some(1),
        };
        let routed = SlotEnvHit::from_hazard(hit(Victim::enemy(SLOT_ID_MIN))).unwrap();
        assert_eq!(routed.victim, SLOT_ID_MIN);
        assert_eq!(routed.source, EnvSource::Hazard(3));
        assert_eq!(
            (routed.damage, routed.push_x, routed.activation),
            (1, -40, 2)
        );
        assert_eq!(routed.initiator, Some(1));
        assert_eq!(SlotEnvHit::from_hazard(hit(Victim::hero(0))), None);
    }

    #[test]
    fn the_env_view_never_panics_on_a_stale_range() {
        let block = |left: f32| Block {
            rect: Rect {
                left,
                bottom: 0.0,
                width: 8.0,
                height: 8.0,
            },
            material: crate::prototype::Material::Timber,
            intact: true,
        };
        let solids = [block(0.0), block(10.0), block(20.0)];
        let mut view = EnvView::default();
        assert!(view.is_empty() && view.solids(&solids).is_empty());
        view.solid_range = 1..3;
        assert_eq!(view.solids(&solids).len(), 2);
        // A range that outgrew the solids reads as empty, not a panic.
        view.solid_range = 2..9;
        assert!(view.solids(&solids).is_empty());
    }

    fn boiler() -> Encounter {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        crate::encounter::load(&root, crate::encounter::BOILER_ENCOUNTER_PATH)
            .expect("the boiler sidecar loads")
            .encounter
    }

    const ROOM: SlotRoom = SlotRoom {
        floor_y: 40.0,
        walls: (0.0, 1200.0),
    };

    #[test]
    fn a_roster_spawns_slot_kinds_and_passes_the_existing_kinds_through() {
        let mut enc = boiler();
        enc.roster[1].kind = RosterKind::Room(EnemyKind::Trog);
        let spawn = spawn_with(&table(), &enc, ROOM, false).unwrap();
        // g1, g3 and e1 are existing kinds; g2 (index 1) became a slot enemy.
        assert_eq!(spawn.existing, [0, 2, 3]);
        assert_eq!(spawn.slot.len(), 1);
        let e = spawn.slot.into_enemies().remove(0);
        assert_eq!(e.kind(), "trog");
        assert_eq!(
            e.target_id(),
            SLOT_ID_MIN + 1,
            "the roster index orders the ids"
        );
        assert!(!e.supplies_boss());
        // The entry's column and the room's floor reach the builder.
        let rect = e.hurt_rects()[0];
        assert_eq!(rect.bottom, ROOM.floor_y);
        assert_eq!(rect.left + 12.0, enc.roster[1].x as f32);
    }

    #[test]
    fn a_roster_of_only_existing_kinds_spawns_no_slot_enemy() {
        let spawn = spawn_with(&table(), &boiler(), ROOM, true).unwrap();
        assert!(spawn.slot.is_empty());
        assert_eq!(spawn.existing, [0, 1, 2, 3]);
        // The shipped registry builds nothing for a roster of existing kinds.
        assert!(
            spawn_from_roster(&boiler(), ROOM, false)
                .unwrap()
                .slot
                .is_empty()
        );
    }

    #[test]
    fn an_unregistered_kind_is_rejected_not_dropped() {
        let mut enc = boiler();
        enc.roster[2].kind = RosterKind::Room(EnemyKind::Ball);
        let err = spawn_with(&table(), &enc, ROOM, false).unwrap_err();
        assert_eq!(
            err,
            SlotError::UnknownKind {
                entry: "g3".into(),
                kind: "ball".into()
            }
        );
        assert_eq!(spawn_from_roster(&enc, ROOM, false).unwrap_err(), err);
        // The slime is a roster kind of its own, and an unregistered one too
        // (the quiet `trog` row alone does not register it).
        enc.roster[2].kind = RosterKind::Slime;
        assert_eq!(
            spawn_with(&table()[..1], &enc, ROOM, false).unwrap_err(),
            SlotError::UnknownKind {
                entry: "g3".into(),
                kind: "slime".into()
            }
        );
    }

    #[test]
    fn a_roster_with_two_boss_supplier_kinds_is_rejected() {
        let mut enc = boiler();
        // The shipped suppliers are the Hoarder and the slime (`supplies_boss`):
        // the roster rule rejects the pair before any kind is built, so it holds
        // with an empty registry.
        enc.roster[0].kind = RosterKind::Room(EnemyKind::Hoarder);
        enc.roster[1].kind = RosterKind::Slime;
        let err = spawn_with(&[], &enc, ROOM, false).unwrap_err();
        assert_eq!(
            err,
            SlotError::SupplierConflict {
                kinds: vec!["hoarder", "slime"]
            }
        );
        assert_eq!(spawn_from_roster(&enc, ROOM, false).unwrap_err(), err);
        // One supplier kind passes the roster rule (and then needs a registered
        // builder): two Hoarders are one kind. A fixture boss beside it is refused.
        enc.roster[1].kind = RosterKind::Room(EnemyKind::Hoarder);
        assert_eq!(
            spawn_with(&[], &enc, ROOM, false).unwrap_err(),
            SlotError::UnknownKind {
                entry: "g1".into(),
                kind: "hoarder".into()
            }
        );
        assert_eq!(
            spawn_with(&[], &enc, ROOM, true).unwrap_err(),
            SlotError::FixtureBossConflict { kind: "hoarder" }
        );
        // One built supplier kind passes, even with two enemies of it.
        let mut enc = boiler();
        enc.roster[0].kind = RosterKind::Room(EnemyKind::Hoarder);
        enc.roster[1].kind = RosterKind::Room(EnemyKind::Hoarder);
        enc.roster[2].kind = RosterKind::Room(EnemyKind::Trog);
        let ok = spawn_with(&table(), &enc, ROOM, false).unwrap();
        assert_eq!(ok.slot.len(), 3);
        assert_eq!(
            spawn_with(&table(), &enc, ROOM, true).unwrap_err(),
            SlotError::FixtureBossConflict { kind: "hoarder" }
        );
    }

    #[test]
    fn the_built_set_checks_the_supplier_rule_again() {
        // Two supplier kinds built directly (not through a roster) are refused,
        // and so is a supplier beside a fixture boss.
        let supplier = |id: u32, kind: &'static str| {
            let mut d = Dummy::new(id, 100.0, 0.0);
            d.kind = kind;
            d.supplies = true;
            d.boxed()
        };
        let err = SlotSet::new(
            vec![
                supplier(SLOT_ID_MIN, "hoarder"),
                supplier(SLOT_ID_MIN + 1, "slime"),
            ],
            false,
        )
        .unwrap_err();
        assert_eq!(
            err,
            SlotError::SupplierConflict {
                kinds: vec!["hoarder", "slime"]
            }
        );
        assert_eq!(
            SlotSet::new(vec![supplier(SLOT_ID_MIN, "hoarder")], true).unwrap_err(),
            SlotError::FixtureBossConflict { kind: "hoarder" }
        );
    }

    #[test]
    fn a_built_enemy_must_match_its_registry_row_and_roster_kind() {
        let table = [
            // A quiet roster kind whose builder made a supplier.
            SlotKindDef {
                name: "trog",
                build: |b| Ok(from_entry(b, "trog", true)),
            },
            // A row whose builder reports another kind name.
            SlotKindDef {
                name: "hoarder",
                build: |b| Ok(from_entry(b, "pile_boss", true)),
            },
            // A supplier roster kind whose builder made a quiet enemy.
            SlotKindDef {
                name: "slime",
                build: |b| Ok(from_entry(b, "slime", false)),
            },
        ];
        let mut enc = boiler();
        enc.roster[0].kind = RosterKind::Room(EnemyKind::Trog);
        let err = spawn_with(&table, &enc, ROOM, false).unwrap_err();
        assert!(
            matches!(&err, SlotError::Build { kind: "trog", reason } if reason.contains("supplies_boss")),
            "{err:?}"
        );
        enc.roster[0].kind = RosterKind::Room(EnemyKind::Hoarder);
        let err = spawn_with(&table, &enc, ROOM, false).unwrap_err();
        assert!(
            matches!(&err, SlotError::Build { kind: "hoarder", reason } if reason.contains("pile_boss")),
            "{err:?}"
        );
        enc.roster[0].kind = RosterKind::Slime;
        let err = spawn_with(&table, &enc, ROOM, false).unwrap_err();
        assert!(
            matches!(&err, SlotError::Build { kind: "slime", reason } if reason.contains("supplies_boss")),
            "{err:?}"
        );
        // The consistent table builds all three.
        for kind in [
            RosterKind::Room(EnemyKind::Trog),
            RosterKind::Room(EnemyKind::Hoarder),
            RosterKind::Slime,
        ] {
            enc.roster[0].kind = kind;
            assert!(spawn_with(&super::testing::table(), &enc, ROOM, false).is_ok());
        }
    }

    #[test]
    fn the_kind_names_are_distinct_registry_keys() {
        use EnemyKind::*;
        let names: Vec<&str> = [
            Goblin, Llama, Engineer, Scatterer, Hoarder, Trog, Juicer, Ball,
        ]
        .into_iter()
        .map(kind_name)
        .collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len());
        for name in names {
            assert_eq!(EnemyKind::parse(name).map(kind_name), Some(name), "{name}");
        }
    }
}
