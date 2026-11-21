//! BOIL-B core: environmental hazard volumes. Pure, Bevy-free, integer math.
//!
//! A hazard volume is a **contact area with a window**:
//!
//! * a **timed** volume is triggered, shows its telegraph for a fixed number of
//!   ticks, is active for a fixed number of ticks and then ends (steam, a
//!   falling mass, a drum blast). While active a victim is hurt **at most once
//!   per activation** (the activation token);
//! * a **persistent** volume is active from the start and stays active (an
//!   acid patch, a wheel, a slide). It has no telegraph and no trigger, its
//!   rectangle may change every tick ([`HazardVolume::set_rect`]), and its
//!   tokens are re-armed by the caller ([`HazardVolume::rearm`]). Persistent
//!   and moving volumes are **exempt from the timed fairness rules** below;
//!   each sheet that appends one (HAZ, HRD, DOZER-CORE) brings its own cooldown
//!   and fairness rules on top of this core.
//!
//! Who is hurt is the volume's data, never the initiator's: a
//! [`HeroDamagePolicy`] for heroes plus a `hurts_enemies` flag. A world hazard
//! hurts every hero by its own data (integration plan K3, [`HeroDamagePolicy::
//! AllHeroes`]); the hero who set it off only receives the kill credit
//! (`initiator`). [`HeroDamagePolicy::OwnerReduced`] is rejected: a world
//! hazard has no owner.
//!
//! Tick order (the contract for the future wire step): triggers and stimuli of
//! the tick, then [`HazardVolume::step`], then [`HazardSet::resolve`]. After
//! `trigger` a timed volume is telegraphed for exactly `telegraph_ticks` calls
//! of `step`, is then active for exactly `active_ticks` evaluation ticks and
//! the `step` after the last of them ends it. So `active_ticks = 1` is a single
//! evaluation (a falling mass, a blast).
//!
//! Every [`HazardClass`] is **never evadable** (DM-Q18: hazards, falling
//! masses, blasts and body contacts stay a positioning problem). The class has
//! no way to opt in; a future enemy design sheet that wants a single targeted
//! attack to be evadable must not use a hazard volume for it.
//!
//! Each volume exports its telegraphed and active rectangles per tick
//! ([`HazardVolume::nav_rect`], [`HazardSet::nav_feed`]) as plain data in the
//! room's layout pixels (y down), with the ticks left in the phase. That is the
//! input CA2-C needs for the nav graph (K13); this module does not depend on
//! the companion code.
//!
//! No wall-clock time, no RNG, no floats.

use std::collections::BTreeSet;

use crate::camera_lab::room::IRect;
use crate::hero_damage::HeroDamagePolicy;

/// A timed volume that can hurt a hero shows its telegraph at least this long
/// (HAZ-0's fairness analysis: 12 ticks of reaction is unfair from a standing
/// start; every known hero hazard is at 24 or more).
pub const MIN_HERO_TELEGRAPH_TICKS: u16 = 24;
/// A timed volume that can hurt a hero stays active at most this long (2 s),
/// so no timed volume can seal the ground lane for long.
pub const MAX_HERO_ACTIVE_TICKS: u16 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Team {
    Hero,
    Enemy,
}

/// A body a volume can hurt. `id` is the player slot for heroes and the enemy
/// instance id for enemies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Victim {
    pub team: Team,
    pub id: u32,
}

impl Victim {
    pub const fn hero(id: u32) -> Self {
        Self {
            team: Team::Hero,
            id,
        }
    }
    pub const fn enemy(id: u32) -> Self {
        Self {
            team: Team::Enemy,
            id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HazardClass {
    /// A venting pipe: a bounded horizontal contact area.
    Steam,
    /// A hanging mass that lands once on its footprint.
    FallingMass,
    /// A world blast ring (a drum, a Rot blast).
    Blast,
    /// An acid patch.
    Acid,
    /// A pile slide or any other push area.
    Slide,
    /// A vehicle wheel.
    Wheel,
}

impl HazardClass {
    pub const ALL: [Self; 6] = [
        Self::Steam,
        Self::FallingMass,
        Self::Blast,
        Self::Acid,
        Self::Slide,
        Self::Wheel,
    ];

    /// DM-Q18: **always** `false`. There is no data knob for it.
    pub const fn evadable(self) -> bool {
        false
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Steam => "steam",
            Self::FallingMass => "falling_mass",
            Self::Blast => "blast",
            Self::Acid => "acid",
            Self::Slide => "slide",
            Self::Wheel => "wheel",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.name() == name)
    }

    const fn code(self) -> u64 {
        self as u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Waiting for a trigger (an activation is left).
    Idle,
    Telegraph,
    Active,
    /// Every activation is used.
    Spent,
}

impl Phase {
    const fn code(self) -> u64 {
        self as u64
    }
}

/// How a volume's window runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardWindow {
    /// Triggered, telegraphed, active for a while, then idle or spent.
    Timed {
        telegraph_ticks: u16,
        active_ticks: u16,
        /// Activations per attempt (1 for every boiler volume).
        max_activations: u8,
    },
    /// Active from the start and for ever; no telegraph, no trigger.
    Persistent,
}

/// Authored data of one volume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HazardSpec {
    pub id: u16,
    pub class: HazardClass,
    /// The contact area while active (layout px, y down). The initial area of
    /// a persistent volume, whose area may then change per tick.
    pub rect: IRect,
    /// The area drawn and exported while telegraphed (a floor shadow, a
    /// footprint). The same as `rect` unless the data says otherwise.
    pub telegraph_rect: IRect,
    pub window: HazardWindow,
    /// Damage of the one hit per victim and activation.
    pub damage: u8,
    /// Horizontal push of the hit in px/s (signed, 0 for none).
    pub push_x: i16,
    /// What the volume does to heroes (never `OwnerReduced`).
    pub hero_damage: HeroDamagePolicy,
    /// Whether enemies are hurt too.
    pub hurts_enemies: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HazardSpecError {
    EmptyRect(&'static str),
    NoActiveTicks,
    NoActivations,
    /// A timed, hero-hurting volume with a telegraph shorter than the minimum.
    TelegraphTooShort {
        ticks: u16,
    },
    /// A timed, hero-hurting volume that stays active too long.
    ActiveTooLong {
        ticks: u16,
    },
    /// A world hazard has no owner to reduce the damage for.
    OwnerReducedWorldHazard,
    /// It hurts neither heroes nor enemies.
    HurtsNobody,
}

impl std::fmt::Display for HazardSpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyRect(which) => write!(f, "{which} has no area"),
            Self::NoActiveTicks => write!(f, "active_ticks must be at least 1"),
            Self::NoActivations => write!(f, "max_activations must be at least 1"),
            Self::TelegraphTooShort { ticks } => write!(
                f,
                "a timed hero-hurting volume needs at least {MIN_HERO_TELEGRAPH_TICKS} telegraph ticks, has {ticks}"
            ),
            Self::ActiveTooLong { ticks } => write!(
                f,
                "a timed hero-hurting volume may stay active at most {MAX_HERO_ACTIVE_TICKS} ticks, has {ticks}"
            ),
            Self::OwnerReducedWorldHazard => {
                write!(
                    f,
                    "owner_reduced is for hero-thrown sources, not a world hazard"
                )
            }
            Self::HurtsNobody => write!(f, "the volume hurts neither heroes nor enemies"),
        }
    }
}

impl HazardSpec {
    pub fn validate(&self) -> Result<(), HazardSpecError> {
        if self.rect.w <= 0 || self.rect.h <= 0 {
            return Err(HazardSpecError::EmptyRect("rect"));
        }
        if self.telegraph_rect.w <= 0 || self.telegraph_rect.h <= 0 {
            return Err(HazardSpecError::EmptyRect("telegraph_rect"));
        }
        if self.hero_damage == HeroDamagePolicy::OwnerReduced {
            return Err(HazardSpecError::OwnerReducedWorldHazard);
        }
        if !self.hurts_heroes() && !self.hurts_enemies {
            return Err(HazardSpecError::HurtsNobody);
        }
        if let HazardWindow::Timed {
            telegraph_ticks,
            active_ticks,
            max_activations,
        } = self.window
        {
            if active_ticks == 0 {
                return Err(HazardSpecError::NoActiveTicks);
            }
            if max_activations == 0 {
                return Err(HazardSpecError::NoActivations);
            }
            if self.hurts_heroes() {
                if telegraph_ticks < MIN_HERO_TELEGRAPH_TICKS {
                    return Err(HazardSpecError::TelegraphTooShort {
                        ticks: telegraph_ticks,
                    });
                }
                if active_ticks > MAX_HERO_ACTIVE_TICKS {
                    return Err(HazardSpecError::ActiveTooLong {
                        ticks: active_ticks,
                    });
                }
            }
        }
        Ok(())
    }

    pub const fn hurts_heroes(&self) -> bool {
        self.hero_damage.hurts_any_hero()
    }

    pub const fn hurts(&self, team: Team) -> bool {
        match team {
            Team::Hero => self.hero_damage.hurts_any_hero(),
            Team::Enemy => self.hurts_enemies,
        }
    }

    pub const fn is_persistent(&self) -> bool {
        matches!(self.window, HazardWindow::Persistent)
    }

    /// Telegraph ticks of a timed volume, 0 for a persistent one.
    pub const fn telegraph_ticks(&self) -> u16 {
        match self.window {
            HazardWindow::Timed {
                telegraph_ticks, ..
            } => telegraph_ticks,
            HazardWindow::Persistent => 0,
        }
    }

    /// Active ticks of a timed volume, 0 for a persistent one.
    pub const fn active_ticks(&self) -> u16 {
        match self.window {
            HazardWindow::Timed { active_ticks, .. } => active_ticks,
            HazardWindow::Persistent => 0,
        }
    }

    /// Activations of a timed volume, 0 for a persistent one.
    pub const fn max_activations(&self) -> u8 {
        match self.window {
            HazardWindow::Timed {
                max_activations, ..
            } => max_activations,
            HazardWindow::Persistent => 0,
        }
    }
}

/// One hit: at most one per victim and activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HazardHit {
    pub hazard: u16,
    pub class: HazardClass,
    pub victim: Victim,
    /// 1-based activation number (the token of this hit); the epoch of a
    /// persistent volume, which wraps from 255 to 1.
    pub activation: u8,
    pub damage: u8,
    pub push_x: i16,
    /// The player who set the volume off, for kill credit.
    pub initiator: Option<u32>,
}

impl HazardHit {
    /// DM-Q18, always `false`.
    pub const fn evadable(&self) -> bool {
        self.class.evadable()
    }
}

/// A state change of one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardEdge {
    ActiveStarted,
    /// The active window ended (the volume is idle again or spent).
    Ended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavPhase {
    Telegraph,
    Active,
}

/// What the nav feed of CA2-C reads: a rectangle that is, or is about to be,
/// dangerous. Layout pixels, y down (the room file convention); the consumer
/// converts to its own y-up rectangles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavRect {
    pub hazard: u16,
    pub class: HazardClass,
    pub rect: IRect,
    pub phase: NavPhase,
    /// Ticks until the phase ends: until the volume turns active while
    /// telegraphed, until it ends while active. `None` for a persistent
    /// volume.
    pub ticks_left: Option<u16>,
    pub hurts_heroes: bool,
    pub hurts_enemies: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HazardVolume {
    spec: HazardSpec,
    /// The contact rectangle now (the spec's, unless a caller moved it).
    rect: IRect,
    phase: Phase,
    ticks_left: u16,
    activations: u8,
    initiator: Option<u32>,
    /// `(activation, victim)` tokens already spent.
    applied: BTreeSet<(u8, Victim)>,
}

impl HazardVolume {
    pub fn new(spec: HazardSpec) -> Result<Self, HazardSpecError> {
        spec.validate()?;
        let persistent = spec.is_persistent();
        Ok(Self {
            rect: spec.rect,
            phase: if persistent {
                Phase::Active
            } else {
                Phase::Idle
            },
            ticks_left: 0,
            // A persistent volume starts in its first epoch.
            activations: u8::from(persistent),
            initiator: None,
            applied: BTreeSet::new(),
            spec,
        })
    }

    pub fn spec(&self) -> &HazardSpec {
        &self.spec
    }
    pub fn id(&self) -> u16 {
        self.spec.id
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn activations(&self) -> u8 {
        self.activations
    }
    pub fn initiator(&self) -> Option<u32> {
        self.initiator
    }
    /// The contact rectangle now.
    pub fn rect(&self) -> IRect {
        self.rect
    }
    /// DM-Q18, always `false`.
    pub const fn evadable(&self) -> bool {
        self.spec.class.evadable()
    }

    /// Move or reshape the contact rectangle (a moving mass, a shrinking
    /// patch). `false` for an empty rectangle.
    pub fn set_rect(&mut self, rect: IRect) -> bool {
        if rect.w <= 0 || rect.h <= 0 {
            return false;
        }
        self.rect = rect;
        true
    }

    /// Persistent volumes only: start a new epoch, so every victim may be hit
    /// again. The spent tokens are cleared (the state stays bounded however
    /// often it is re-armed) and the epoch number wraps from 255 back to 1.
    /// `false` for a timed volume.
    pub fn rearm(&mut self) -> bool {
        if !self.spec.is_persistent() {
            return false;
        }
        self.activations = self.activations.checked_add(1).unwrap_or(1);
        self.applied.clear();
        true
    }

    /// Set a timed volume off. Only an idle volume with an activation left
    /// starts. Returns whether it started.
    pub fn trigger(&mut self, initiator: Option<u32>) -> bool {
        let HazardWindow::Timed {
            telegraph_ticks,
            active_ticks,
            max_activations,
        } = self.spec.window
        else {
            return false;
        };
        if self.phase != Phase::Idle || self.activations >= max_activations {
            return false;
        }
        self.activations += 1;
        self.initiator = initiator;
        if telegraph_ticks == 0 {
            self.phase = Phase::Active;
            self.ticks_left = active_ticks;
        } else {
            self.phase = Phase::Telegraph;
            self.ticks_left = telegraph_ticks;
        }
        true
    }

    /// One fixed tick.
    pub fn step(&mut self) -> Option<HazardEdge> {
        let HazardWindow::Timed {
            active_ticks,
            max_activations,
            ..
        } = self.spec.window
        else {
            return None;
        };
        match self.phase {
            Phase::Telegraph => {
                self.ticks_left -= 1;
                if self.ticks_left == 0 {
                    self.phase = Phase::Active;
                    self.ticks_left = active_ticks;
                    return Some(HazardEdge::ActiveStarted);
                }
                None
            }
            Phase::Active => {
                self.ticks_left -= 1;
                if self.ticks_left == 0 {
                    self.phase = if self.activations >= max_activations {
                        Phase::Spent
                    } else {
                        Phase::Idle
                    };
                    return Some(HazardEdge::Ended);
                }
                None
            }
            Phase::Idle | Phase::Spent => None,
        }
    }

    /// The rectangle the victims are tested against right now, if active.
    pub fn active_rect(&self) -> Option<IRect> {
        (self.phase == Phase::Active).then_some(self.rect)
    }

    /// The rectangle of the warning, if telegraphed.
    pub fn telegraph_rect(&self) -> Option<IRect> {
        (self.phase == Phase::Telegraph).then_some(self.spec.telegraph_rect)
    }

    /// The nav-feed entry of this tick: the telegraph rectangle while
    /// telegraphed, the active rectangle while active, nothing otherwise.
    pub fn nav_rect(&self) -> Option<NavRect> {
        let (rect, phase) = match self.phase {
            Phase::Telegraph => (self.spec.telegraph_rect, NavPhase::Telegraph),
            Phase::Active => (self.rect, NavPhase::Active),
            Phase::Idle | Phase::Spent => return None,
        };
        Some(NavRect {
            hazard: self.spec.id,
            class: self.spec.class,
            rect,
            phase,
            ticks_left: (!self.spec.is_persistent()).then_some(self.ticks_left),
            hurts_heroes: self.spec.hurts(Team::Hero),
            hurts_enemies: self.spec.hurts(Team::Enemy),
        })
    }

    /// Test one victim body. Hurts only while active, only a team the spec
    /// names, and only once per victim and activation (or epoch).
    pub fn contact(&mut self, victim: Victim, body: IRect) -> Option<HazardHit> {
        if self.phase != Phase::Active || !self.spec.hurts(victim.team) || !body.overlaps(self.rect)
        {
            return None;
        }
        if !self.applied.insert((self.activations, victim)) {
            return None;
        }
        Some(HazardHit {
            hazard: self.spec.id,
            class: self.spec.class,
            victim,
            activation: self.activations,
            damage: self.spec.damage,
            push_x: self.spec.push_x,
            initiator: self.initiator,
        })
    }

    /// Fold the full state into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(u64::from(self.spec.id));
        mix(self.spec.class.code());
        mix(self.spec.hero_damage.code());
        mix(u64::from(self.spec.hurts_enemies));
        mix(u64::from(self.spec.is_persistent()));
        for v in [self.rect.x, self.rect.y, self.rect.w, self.rect.h] {
            mix(v as u32 as u64);
        }
        mix(self.phase.code());
        mix(u64::from(self.ticks_left));
        mix(u64::from(self.activations));
        mix(self.initiator.map_or(0, |p| u64::from(p) + 1));
        mix(self.applied.len() as u64);
        for (activation, victim) in &self.applied {
            mix(u64::from(*activation));
            mix(match victim.team {
                Team::Hero => 0,
                Team::Enemy => 1,
            });
            mix(u64::from(victim.id));
        }
    }
}

/// All volumes of a room, in id order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HazardSet {
    volumes: Vec<HazardVolume>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HazardSetError {
    DuplicateId(u16),
    Spec { id: u16, error: HazardSpecError },
}

impl std::fmt::Display for HazardSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(f, "duplicate hazard id {id}"),
            Self::Spec { id, error } => write!(f, "hazard {id}: {error}"),
        }
    }
}

impl HazardSet {
    pub fn new(specs: Vec<HazardSpec>) -> Result<Self, HazardSetError> {
        let mut volumes = Vec::with_capacity(specs.len());
        for spec in specs {
            let id = spec.id;
            volumes
                .push(HazardVolume::new(spec).map_err(|error| HazardSetError::Spec { id, error })?);
        }
        volumes.sort_by_key(HazardVolume::id);
        if let Some(pair) = volumes.windows(2).find(|p| p[0].id() == p[1].id()) {
            return Err(HazardSetError::DuplicateId(pair[0].id()));
        }
        Ok(Self { volumes })
    }

    pub fn volumes(&self) -> &[HazardVolume] {
        &self.volumes
    }

    pub fn get(&self, id: u16) -> Option<&HazardVolume> {
        self.volumes.iter().find(|v| v.id() == id)
    }

    pub fn get_mut(&mut self, id: u16) -> Option<&mut HazardVolume> {
        self.volumes.iter_mut().find(|v| v.id() == id)
    }

    /// Trigger a volume by id. `false` for an unknown id or a volume that
    /// cannot start.
    pub fn trigger(&mut self, id: u16, initiator: Option<u32>) -> bool {
        self.get_mut(id).is_some_and(|v| v.trigger(initiator))
    }

    /// One tick of every volume, in id order.
    pub fn step(&mut self) -> Vec<(u16, HazardEdge)> {
        self.volumes
            .iter_mut()
            .filter_map(|v| v.step().map(|edge| (v.id(), edge)))
            .collect()
    }

    /// Resolve every active volume against the bodies. The result order is by
    /// volume id, then victim (team, id), whatever the order of `bodies`.
    pub fn resolve(&mut self, bodies: &[(Victim, IRect)]) -> Vec<HazardHit> {
        let mut order: Vec<&(Victim, IRect)> = bodies.iter().collect();
        order.sort_by_key(|(victim, _)| *victim);
        let mut hits = Vec::new();
        for volume in &mut self.volumes {
            for (victim, body) in &order {
                if let Some(hit) = volume.contact(*victim, *body) {
                    hits.push(hit);
                }
            }
        }
        hits
    }

    /// Every telegraphed and active rectangle of this tick, in id order. This
    /// is the CA2-C feed: filter with `hurts_heroes`.
    pub fn nav_feed(&self) -> Vec<NavRect> {
        self.volumes
            .iter()
            .filter_map(HazardVolume::nav_rect)
            .collect()
    }

    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.volumes.len() as u64);
        for volume in &self.volumes {
            volume.mix_into(mix);
        }
    }
}

/// HAZ-A1: the per-entry token policy of a persistent contact volume (an acid
/// patch, a slime body). BOIL-B core's token is once per (activation, victim)
/// and a persistent volume has one activation, so a victim standing in it
/// would be hurt once for ever and a victim re-entering never again.
///
/// The rule (HAZ-0 section 8): one hurt per victim per **entry**. The token of
/// a victim is spent by the hurt and re-armed on the first tick on which the
/// victim's body is **outside** the volume, so the victim must leave for at
/// least one tick before it can be hurt again. An entry while the victim cannot
/// be hurt (invulnerable) spends nothing: the token stays pending, and the hurt
/// applies on the first tick the victim can be hurt while still inside. The
/// caller supplies the facts of the tick; the state is a small ordered set
/// (bounded by the number of victims, hashed in order).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntryTokens {
    spent: BTreeSet<Victim>,
}

impl EntryTokens {
    pub fn new() -> Self {
        Self::default()
    }

    /// One victim, one tick. `inside` is whether its body overlaps the volume
    /// now, `vulnerable` whether a hurt would land now (not invulnerable, not
    /// defeated). Returns whether the victim is hurt on this tick.
    pub fn offer(&mut self, victim: Victim, inside: bool, vulnerable: bool) -> bool {
        if !inside {
            self.spent.remove(&victim);
            return false;
        }
        if self.spent.contains(&victim) || !vulnerable {
            return false;
        }
        self.spent.insert(victim);
        true
    }

    /// Whether the victim's token is spent (it was hurt on this entry).
    pub fn is_spent(&self, victim: Victim) -> bool {
        self.spent.contains(&victim)
    }

    /// Forget every entry (an attempt restart).
    pub fn clear(&mut self) {
        self.spent.clear();
    }

    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.spent.len() as u64);
        for victim in &self.spent {
            mix(match victim.team {
                Team::Hero => 0,
                Team::Enemy => 1,
            });
            mix(u64::from(victim.id));
        }
    }
}

/// HAZ-A1: a per-launch-instance token. A launched body (a kicked goblin, a
/// CF02 flier) that lands in a persistent volume is hurt **once per launch**,
/// however long it lies there and however many times its landing is tested.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LaunchTokens {
    seen: BTreeSet<u32>,
}

impl LaunchTokens {
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` the first time a launch id is offered, `false` afterwards.
    pub fn offer(&mut self, launch_id: u32) -> bool {
        self.seen.insert(launch_id)
    }

    pub fn clear(&mut self) {
        self.seen.clear();
    }

    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.seen.len() as u64);
        for id in &self.seen {
            mix(u64::from(*id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(id: u16) -> HazardSpec {
        HazardSpec {
            id,
            class: HazardClass::Steam,
            rect: IRect::new(100, 0, 40, 60),
            telegraph_rect: IRect::new(100, 0, 40, 60),
            window: HazardWindow::Timed {
                telegraph_ticks: 24,
                active_ticks: 72,
                max_activations: 1,
            },
            damage: 1,
            push_x: 120,
            hero_damage: HeroDamagePolicy::AllHeroes,
            hurts_enemies: true,
        }
    }

    fn timed(s: &mut HazardSpec, telegraph: u16, active: u16, max: u16) {
        s.window = HazardWindow::Timed {
            telegraph_ticks: telegraph,
            active_ticks: active,
            max_activations: max as u8,
        };
    }

    fn persistent(id: u16) -> HazardSpec {
        HazardSpec {
            window: HazardWindow::Persistent,
            class: HazardClass::Acid,
            ..spec(id)
        }
    }

    fn inside() -> IRect {
        IRect::new(110, 10, 24, 52)
    }

    fn outside() -> IRect {
        IRect::new(300, 10, 24, 52)
    }

    #[test]
    fn no_class_is_evadable_and_a_hit_says_so() {
        for class in HazardClass::ALL {
            assert!(!class.evadable(), "{}", class.name());
            assert_eq!(HazardClass::parse(class.name()), Some(class));
        }
        let mut v = HazardVolume::new(spec(1)).unwrap();
        assert!(!v.evadable());
        v.trigger(Some(0));
        for _ in 0..24 {
            v.step();
        }
        let hit = v.contact(Victim::hero(0), inside()).unwrap();
        assert!(!hit.evadable());
    }

    #[test]
    fn the_telegraph_and_active_windows_have_exact_lengths() {
        let mut v = HazardVolume::new(spec(1)).unwrap();
        assert_eq!(v.phase(), Phase::Idle);
        assert!(v.nav_rect().is_none());
        assert!(v.trigger(Some(1)));
        let mut telegraph = 0;
        let mut active = 0;
        let mut edges = Vec::new();
        for _ in 0..200 {
            match v.phase() {
                Phase::Telegraph => telegraph += 1,
                Phase::Active => active += 1,
                _ => {}
            }
            if let Some(edge) = v.step() {
                edges.push(edge);
            }
        }
        assert_eq!((telegraph, active), (24, 72));
        assert_eq!(edges, [HazardEdge::ActiveStarted, HazardEdge::Ended]);
        assert_eq!(v.phase(), Phase::Spent);
    }

    #[test]
    fn a_single_tick_volume_evaluates_exactly_one_tick() {
        let mut s = spec(1);
        timed(&mut s, 24, 1, 1);
        s.class = HazardClass::FallingMass;
        let mut v = HazardVolume::new(s).unwrap();
        v.trigger(None);
        for _ in 0..24 {
            v.step();
        }
        assert_eq!(v.phase(), Phase::Active);
        assert!(v.contact(Victim::enemy(5), inside()).is_some());
        assert_eq!(v.step(), Some(HazardEdge::Ended));
        assert_eq!(v.phase(), Phase::Spent);
        assert!(v.contact(Victim::enemy(6), inside()).is_none());
    }

    #[test]
    fn a_victim_is_hurt_once_per_activation_and_only_while_active() {
        let mut v = HazardVolume::new(spec(1)).unwrap();
        v.trigger(Some(0));
        // Telegraphed: nobody is hurt.
        assert!(v.contact(Victim::hero(0), inside()).is_none());
        for _ in 0..24 {
            v.step();
        }
        let first = v.contact(Victim::hero(0), inside()).unwrap();
        assert_eq!((first.activation, first.damage, first.push_x), (1, 1, 120));
        assert_eq!(first.initiator, Some(0));
        // The same victim again: no second hit, in or out and back in.
        assert!(v.contact(Victim::hero(0), inside()).is_none());
        assert!(v.contact(Victim::hero(0), outside()).is_none());
        assert!(v.contact(Victim::hero(0), inside()).is_none());
        // Another victim is its own token.
        assert!(v.contact(Victim::hero(1), inside()).is_some());
        assert!(v.contact(Victim::enemy(0), inside()).is_some());
        // Outside the rectangle: nothing.
        assert!(v.contact(Victim::enemy(1), outside()).is_none());
    }

    #[test]
    fn a_second_activation_gives_new_tokens() {
        let mut s = spec(1);
        timed(&mut s, 24, 3, 2);
        let mut v = HazardVolume::new(s).unwrap();
        for round in 1..=2u8 {
            assert!(v.trigger(None));
            while v.phase() != Phase::Active {
                v.step();
            }
            let hit = v.contact(Victim::hero(0), inside()).unwrap();
            assert_eq!(hit.activation, round);
            while !matches!(v.phase(), Phase::Idle | Phase::Spent) {
                v.step();
            }
        }
        assert_eq!(v.phase(), Phase::Spent);
        assert!(!v.trigger(None), "no activation left");
    }

    #[test]
    fn a_busy_or_spent_volume_ignores_a_trigger() {
        let mut v = HazardVolume::new(spec(1)).unwrap();
        assert!(v.trigger(Some(0)));
        assert!(!v.trigger(Some(1)), "already telegraphing");
        assert_eq!(v.initiator(), Some(0));
        for _ in 0..24 + 72 {
            v.step();
        }
        assert_eq!(v.phase(), Phase::Spent);
        assert!(!v.trigger(Some(1)));
        assert_eq!(v.activations(), 1);
    }

    #[test]
    fn the_hero_policy_is_the_volume_not_the_initiator() {
        for (policy, enemies, hero, enemy) in [
            (HeroDamagePolicy::AllHeroes, true, true, true),
            (HeroDamagePolicy::AllHeroes, false, true, false),
            (HeroDamagePolicy::NoHeroes, true, false, true),
        ] {
            let mut s = spec(1);
            s.hero_damage = policy;
            s.hurts_enemies = enemies;
            if !s.hurts_heroes() {
                timed(&mut s, 0, 72, 1);
            }
            let mut v = HazardVolume::new(s).unwrap();
            // A hero set it off; the world hazard still hurts that hero by its data.
            v.trigger(Some(0));
            while v.phase() != Phase::Active {
                v.step();
            }
            assert_eq!(v.contact(Victim::hero(0), inside()).is_some(), hero);
            assert_eq!(v.contact(Victim::enemy(0), inside()).is_some(), enemy);
        }
    }

    #[test]
    fn spec_validation_rejects_unfair_or_empty_volumes() {
        let mut s = spec(1);
        timed(&mut s, 23, 72, 1);
        assert_eq!(
            HazardVolume::new(s).unwrap_err(),
            HazardSpecError::TelegraphTooShort { ticks: 23 }
        );
        let mut s = spec(1);
        timed(&mut s, 24, 121, 1);
        assert_eq!(
            HazardVolume::new(s).unwrap_err(),
            HazardSpecError::ActiveTooLong { ticks: 121 }
        );
        let mut s = spec(1);
        timed(&mut s, 24, 0, 1);
        assert_eq!(
            HazardVolume::new(s).unwrap_err(),
            HazardSpecError::NoActiveTicks
        );
        let mut s = spec(1);
        timed(&mut s, 24, 72, 0);
        assert_eq!(
            HazardVolume::new(s).unwrap_err(),
            HazardSpecError::NoActivations
        );
        let mut s = spec(1);
        s.rect = IRect::new(0, 0, 0, 10);
        assert_eq!(
            HazardVolume::new(s).unwrap_err(),
            HazardSpecError::EmptyRect("rect")
        );
        // An enemies-only volume may be quick: the hero rules do not apply.
        let mut s = spec(1);
        s.hero_damage = HeroDamagePolicy::NoHeroes;
        timed(&mut s, 0, 255, 1);
        assert!(HazardVolume::new(s).is_ok());
    }

    #[test]
    fn a_world_hazard_has_no_owner_to_reduce_and_must_hurt_someone() {
        let mut s = spec(1);
        s.hero_damage = HeroDamagePolicy::OwnerReduced;
        assert_eq!(
            HazardVolume::new(s).unwrap_err(),
            HazardSpecError::OwnerReducedWorldHazard
        );
        let mut s = spec(1);
        s.hero_damage = HeroDamagePolicy::NoHeroes;
        s.hurts_enemies = false;
        assert_eq!(
            HazardVolume::new(s).unwrap_err(),
            HazardSpecError::HurtsNobody
        );
    }

    #[test]
    fn a_persistent_volume_is_always_on_with_no_trigger_and_no_telegraph() {
        let mut v = HazardVolume::new(persistent(5)).unwrap();
        assert_eq!(v.phase(), Phase::Active);
        assert!(!v.trigger(Some(0)), "nothing to trigger");
        assert_eq!(v.step(), None);
        assert_eq!(v.phase(), Phase::Active);
        // Exempt from the timed rules: hero-hurting, no telegraph at all.
        let feed = v.nav_rect().unwrap();
        assert_eq!(feed.phase, NavPhase::Active);
        assert_eq!(feed.ticks_left, None);
        assert!(feed.hurts_heroes);
        // One hit per victim per epoch; a re-arm gives new tokens.
        assert!(v.contact(Victim::hero(0), inside()).is_some());
        assert!(v.contact(Victim::hero(0), inside()).is_none());
        assert!(v.rearm());
        let again = v.contact(Victim::hero(0), inside()).unwrap();
        assert_eq!(again.activation, 2);
        // 400 re-arms, 400 hits: the epoch wraps and the tokens stay bounded.
        let mut long = HazardVolume::new(persistent(6)).unwrap();
        let mut hits = 0;
        for _ in 0..400 {
            if long.contact(Victim::hero(0), inside()).is_some() {
                hits += 1;
            }
            assert!(long.contact(Victim::hero(0), inside()).is_none());
            assert!(long.rearm());
            assert!(long.applied.len() <= 1, "bounded: {}", long.applied.len());
        }
        assert_eq!(hits, 400);
        assert!((1..=255).contains(&long.activations()));
        // A timed volume has no epochs to re-arm.
        let mut t = HazardVolume::new(spec(1)).unwrap();
        assert!(!t.rearm());
    }

    #[test]
    fn a_volume_rect_can_move_or_shrink_per_tick() {
        let mut v = HazardVolume::new(persistent(5)).unwrap();
        assert!(v.contact(Victim::enemy(1), outside()).is_none());
        // The patch grows over the victim: now it is inside.
        assert!(v.set_rect(IRect::new(290, 0, 60, 60)));
        assert_eq!(v.nav_rect().unwrap().rect, IRect::new(290, 0, 60, 60));
        assert!(v.contact(Victim::enemy(1), outside()).is_some());
        // An empty rectangle is refused and changes nothing.
        assert!(!v.set_rect(IRect::new(0, 0, 0, 5)));
        assert_eq!(v.rect(), IRect::new(290, 0, 60, 60));
        // The rectangle is state.
        fn hash(v: &HazardVolume) -> u64 {
            let mut h = 0xcbf2_9ce4_8422_2325_u64;
            v.mix_into(&mut |x| {
                h ^= x;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            });
            h
        }
        let before = hash(&v);
        v.set_rect(IRect::new(290, 0, 59, 60));
        assert_ne!(hash(&v), before);
        // A timed volume's active rectangle moves too (a sliding mass).
        let mut t = HazardVolume::new(spec(2)).unwrap();
        t.trigger(None);
        for _ in 0..24 {
            t.step();
        }
        assert!(t.set_rect(IRect::new(290, 0, 60, 60)));
        assert_eq!(t.active_rect(), Some(IRect::new(290, 0, 60, 60)));
    }

    #[test]
    fn the_nav_feed_exports_the_telegraph_then_the_active_rectangle() {
        let mut s = spec(2);
        s.telegraph_rect = IRect::new(100, 52, 40, 8);
        let mut set = HazardSet::new(vec![s, spec(1)]).unwrap();
        assert!(set.nav_feed().is_empty());
        assert!(set.trigger(2, Some(0)));
        let feed = set.nav_feed();
        assert_eq!(feed.len(), 1);
        assert_eq!(feed[0].phase, NavPhase::Telegraph);
        assert_eq!(feed[0].ticks_left, Some(24));
        assert_eq!(feed[0].rect, IRect::new(100, 52, 40, 8));
        assert!(feed[0].hurts_heroes && feed[0].hurts_enemies);
        for _ in 0..24 {
            set.step();
        }
        let feed = set.nav_feed();
        assert_eq!(feed[0].phase, NavPhase::Active);
        assert_eq!(feed[0].ticks_left, Some(72));
        assert_eq!(feed[0].rect, IRect::new(100, 0, 40, 60));
        assert_eq!(feed[0].class, HazardClass::Steam);
        for _ in 0..72 {
            set.step();
        }
        assert!(set.nav_feed().is_empty(), "spent volumes leave the feed");
    }

    #[test]
    fn the_feed_is_in_id_order_and_resolve_ignores_the_body_order() {
        let build = || HazardSet::new(vec![spec(9), spec(3)]).unwrap();
        let mut a = build();
        let mut b = build();
        for set in [&mut a, &mut b] {
            set.trigger(9, None);
            set.trigger(3, None);
            for _ in 0..24 {
                set.step();
            }
        }
        let feed_ids: Vec<u16> = a.nav_feed().iter().map(|n| n.hazard).collect();
        assert_eq!(feed_ids, [3, 9]);
        let bodies = [
            (Victim::enemy(2), inside()),
            (Victim::hero(1), inside()),
            (Victim::hero(0), inside()),
        ];
        let mut reversed = bodies;
        reversed.reverse();
        let ha = a.resolve(&bodies);
        let hb = b.resolve(&reversed);
        assert_eq!(ha, hb);
        let order: Vec<(u16, Victim)> = ha.iter().map(|h| (h.hazard, h.victim)).collect();
        assert_eq!(
            order,
            [
                (3, Victim::hero(0)),
                (3, Victim::hero(1)),
                (3, Victim::enemy(2)),
                (9, Victim::hero(0)),
                (9, Victim::hero(1)),
                (9, Victim::enemy(2)),
            ]
        );
    }

    #[test]
    fn a_set_rejects_duplicate_ids_and_bad_specs() {
        assert_eq!(
            HazardSet::new(vec![spec(1), spec(1)]).unwrap_err(),
            HazardSetError::DuplicateId(1)
        );
        let mut bad = spec(4);
        timed(&mut bad, 24, 0, 1);
        assert_eq!(
            HazardSet::new(vec![bad]).unwrap_err(),
            HazardSetError::Spec {
                id: 4,
                error: HazardSpecError::NoActiveTicks
            }
        );
    }

    #[test]
    fn the_state_hash_follows_every_field() {
        fn hash(v: &HazardVolume) -> u64 {
            let mut h = 0xcbf2_9ce4_8422_2325_u64;
            v.mix_into(&mut |x| {
                h ^= x;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            });
            h
        }
        let fresh = HazardVolume::new(spec(1)).unwrap();
        let mut seen = vec![hash(&fresh)];
        let mut v = fresh.clone();
        v.trigger(Some(0));
        seen.push(hash(&v));
        let mut v2 = fresh.clone();
        v2.trigger(Some(1));
        seen.push(hash(&v2));
        for _ in 0..24 {
            v.step();
        }
        seen.push(hash(&v));
        v.contact(Victim::hero(0), inside());
        seen.push(hash(&v));
        v.step();
        seen.push(hash(&v));
        let unique: BTreeSet<u64> = seen.iter().copied().collect();
        assert_eq!(unique.len(), seen.len(), "{seen:?}");
        // The same inputs give the same hash.
        let mut again = HazardVolume::new(spec(1)).unwrap();
        again.trigger(Some(0));
        for _ in 0..24 {
            again.step();
        }
        again.contact(Victim::hero(0), inside());
        again.step();
        assert_eq!(hash(&again), *seen.last().unwrap());
    }

    #[test]
    fn entry_tokens_hurt_once_per_entry_and_rearm_after_one_tick_outside() {
        let mut t = EntryTokens::new();
        let v = Victim::hero(0);
        assert!(t.offer(v, true, true), "the first tick inside hurts");
        for _ in 0..50 {
            assert!(!t.offer(v, true, true), "standing inside never hurts again");
        }
        assert!(t.is_spent(v));
        assert!(!t.offer(v, false, true), "one tick outside");
        assert!(!t.is_spent(v));
        assert!(t.offer(v, true, true), "a new entry hurts again");
    }

    #[test]
    fn entry_tokens_keep_an_invulnerable_entry_pending() {
        let mut t = EntryTokens::new();
        let v = Victim::hero(1);
        for _ in 0..30 {
            assert!(!t.offer(v, true, false));
            assert!(!t.is_spent(v), "an invulnerable entry spends nothing");
        }
        assert!(
            t.offer(v, true, true),
            "hurts when invulnerability ends inside"
        );
        assert!(!t.offer(v, true, true));
        // Leaving while invulnerable never spends the token either.
        let mut t = EntryTokens::new();
        assert!(!t.offer(v, true, false));
        assert!(!t.offer(v, false, false));
        assert!(t.offer(v, true, true));
    }

    #[test]
    fn entry_tokens_are_per_victim_and_hash_in_order() {
        let mut a = EntryTokens::new();
        let mut b = EntryTokens::new();
        assert!(a.offer(Victim::hero(0), true, true));
        assert!(a.offer(Victim::hero(1), true, true));
        assert!(b.offer(Victim::hero(1), true, true));
        assert!(b.offer(Victim::hero(0), true, true));
        let hash = |t: &EntryTokens| {
            let mut h = 0xcbf2_9ce4_8422_2325_u64;
            t.mix_into(&mut |v| h = (h ^ v).wrapping_mul(0x0000_0100_0000_01b3));
            h
        };
        assert_eq!(hash(&a), hash(&b));
        assert_ne!(hash(&a), hash(&EntryTokens::new()));
        a.clear();
        assert_eq!(a, EntryTokens::new());
    }

    #[test]
    fn launch_tokens_count_a_launch_once() {
        let mut t = LaunchTokens::new();
        assert!(t.offer(7));
        assert!(!t.offer(7));
        assert!(t.offer(8));
        t.clear();
        assert!(t.offer(7));
    }
}
