//! BOIL-B core: the stimulus a world prop can receive, and the pressure valve
//! state machine `intact -> warning -> venting -> spent`. Pure, Bevy-free,
//! integer math. BOIL-C core adds the falling mass (`FallingMass`), its swept
//! landing and the remnant-lift rule further down.
//!
//! A [`Stimulus`] is the *only* way a prop is ever advanced. Six kinds exist:
//! the four physical causes of the boiler design plus two that the Hoarder
//! sheet needs (an enemy shove and a projectile):
//!
//! * a **melee hit** of one move instance, with a per-move hit weight (the
//!   gauntlet's support-hit weight arrives from CP-D as data; today every move
//!   has weight 1). The token is the move instance, so a held button or a
//!   hit-stop frame cannot count twice;
//! * a **launched body** that struck the prop: launch id plus impulse. The
//!   impulse must reach the prop's threshold, and one launch counts once;
//! * a **blast** (dynamite, a drum): a blast id, counted once;
//! * a **projectile** (Donut's Magic Missile): one projectile id, counted once,
//!   credited to its owner;
//! * an enemy **shove** (a displacement attack such as the Hoarder's; the
//!   Hoarder sheet calls it `EnemyImpact`): one shove id with an impulse and an
//!   optional `credit` (the player of the hero the shove baited, `None` when the
//!   target was a bot or the shove was not baited). Whether a prop accepts it is
//!   the prop's data (the valve's default is no);
//! * a **prop contact**: another prop reached this one through an authored
//!   link. It carries the **chain depth**, so links obey the same depth cap as
//!   the charge chains (K2, CP-Q34: [`MAX_CHAIN_DEPTH`]).
//!
//! Something that only *walks into* a prop produces no stimulus at all, so it
//! never triggers anything (BOIL design: "a normal enemy merely walking into
//! the valve never triggers it").
//!
//! The initiator (the player who caused the stimulus) travels with it, so the
//! one whose hit completed the activation is credited for what follows.
//!
//! Every counter is an integer tick; there is no RNG and no wall-clock time.

use crate::camera_lab::room::IRect;
use crate::env_hazard::{Team, Victim};
use crate::hero_damage::HeroDamagePolicy;

/// CP-Q34: a charge chain is at most 4 deep. Prop-to-prop links carry the same
/// counter.
pub const MAX_CHAIN_DEPTH: u8 = 4;
/// The valve's warning is at least this long (an audible and visible tell; the
/// same floor as a hero-hurting hazard telegraph).
pub const MIN_WARNING_TICKS: u16 = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stimulus {
    /// One melee move instance that overlapped the prop's target.
    MeleeHit {
        token: u32,
        player: u32,
        /// Per-move hit weight, at least 1.
        weight: u8,
    },
    /// A launched body struck the prop.
    LaunchedBody {
        launch_id: u32,
        /// Launch speed at the impact (px/s).
        impulse_px_s: u16,
        initiator: Option<u32>,
    },
    Blast {
        blast_id: u32,
        owner: Option<u32>,
    },
    /// A projectile (Magic Missile) hit the prop. Counted once per id.
    Projectile {
        projectile_id: u32,
        owner: Option<u32>,
        /// Hit weight of the projectile, at least 1.
        weight: u8,
    },
    /// An enemy displacement attack (a boss shove) reached the prop. Counted
    /// once per id; no player is credited.
    Shove {
        shove_id: u32,
        /// Shove speed at the contact (px/s).
        impulse_px_s: u16,
        /// The enemy instance that shoved.
        source: u32,
        /// The player credited with what the shove causes, if any.
        credit: Option<u32>,
    },
    /// Another prop (`source`, its index in the encounter) reached this one.
    PropContact {
        source: u16,
        initiator: Option<u32>,
        /// Chain depth of this contact (the first link is depth 1).
        depth: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StimulusKind {
    MeleeHit,
    LaunchedBody,
    Blast,
    Projectile,
    Shove,
    PropContact,
}

impl Stimulus {
    pub const fn kind(&self) -> StimulusKind {
        match self {
            Self::MeleeHit { .. } => StimulusKind::MeleeHit,
            Self::LaunchedBody { .. } => StimulusKind::LaunchedBody,
            Self::Blast { .. } => StimulusKind::Blast,
            Self::Projectile { .. } => StimulusKind::Projectile,
            Self::Shove { .. } => StimulusKind::Shove,
            Self::PropContact { .. } => StimulusKind::PropContact,
        }
    }

    /// The player credited with what this stimulus causes.
    pub const fn initiator(&self) -> Option<u32> {
        match self {
            Self::MeleeHit { player, .. } => Some(*player),
            Self::LaunchedBody { initiator, .. } | Self::PropContact { initiator, .. } => {
                *initiator
            }
            Self::Blast { owner, .. } | Self::Projectile { owner, .. } => *owner,
            Self::Shove { credit, .. } => *credit,
        }
    }

    /// Chain depth: 0 for a stimulus that is not a link.
    pub const fn depth(&self) -> u8 {
        match self {
            Self::PropContact { depth, .. } => *depth,
            _ => 0,
        }
    }
}

/// Valve tunables. The timing comes from the owned steam hazard of the
/// encounter data, the rest from the prop's `valve` object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValveConfig {
    /// Total hit weight that sets the valve off: 2 distinct light strikes or
    /// one strong impact (design default).
    pub trigger_weight: u8,
    /// A launched body counts only from this speed (px/s). Default 160: a
    /// living kicked goblin (170, `KICK_LAUNCH_VX`) and every lethal launch
    /// (160 melee, 200 air kick, 240 kick) open the valve; the 150 px/s
    /// relaunch and the 90 px/s air-kick knock do not (BOIL-03).
    pub launch_min_impulse_px_s: u16,
    /// Weight of a qualifying launched body (the full trigger by default).
    pub launch_weight: u8,
    /// Weight of a blast (the full trigger by default).
    pub blast_weight: u8,
    /// Weight of a prop contact; 0 (default): the valve ignores them.
    pub contact_weight: u8,
    /// Weight of one projectile; the design lets a projectile open the valve
    /// as a distinct light strike (1). 0: ignored.
    pub projectile_weight: u8,
    /// Weight of an enemy shove; 0 (default): the valve ignores shoves (the
    /// Hoarder sheet decides which props take them).
    pub shove_weight: u8,
    /// A shove counts only from this speed (px/s). Default 160, so the
    /// Hoarder's 180 px/s lunge counts; 0 removes the threshold.
    pub shove_min_impulse_px_s: u16,
    pub warning_ticks: u16,
    pub vent_ticks: u16,
}

impl ValveConfig {
    /// The design defaults: 2 light strikes, 24 warning ticks, 72 vent ticks.
    pub const DEFAULT: Self = Self {
        trigger_weight: 2,
        launch_min_impulse_px_s: 160,
        launch_weight: 2,
        blast_weight: 2,
        contact_weight: 0,
        projectile_weight: 1,
        shove_weight: 0,
        shove_min_impulse_px_s: 160,
        warning_ticks: 24,
        vent_ticks: 72,
    };

    pub fn validate(&self) -> Result<(), String> {
        if self.trigger_weight == 0 {
            return Err("trigger_weight must be at least 1".into());
        }
        if self.warning_ticks < MIN_WARNING_TICKS {
            return Err(format!(
                "warning_ticks must be at least {MIN_WARNING_TICKS}, is {}",
                self.warning_ticks
            ));
        }
        if self.vent_ticks == 0 {
            return Err("vent_ticks must be at least 1".into());
        }
        Ok(())
    }
}

impl Default for ValveConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValveState {
    Intact,
    Warning,
    Venting,
    Spent,
}

impl ValveState {
    const fn code(self) -> u64 {
        self as u64
    }
}

/// Why a stimulus changed nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ignored {
    /// The valve is already warning, venting or spent: one activation only.
    AlreadyActivated,
    /// The same melee token, launch id or blast id was already counted.
    Duplicate,
    /// A launched body slower than the threshold.
    TooWeak,
    /// The valve has no weight for this kind (a prop contact by default).
    NotAccepted,
    /// A shove slower than the threshold.
    ShoveTooWeak,
    /// A prop contact at or over [`MAX_CHAIN_DEPTH`].
    ChainTooDeep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offer {
    Ignored(Ignored),
    /// Counted, not yet enough. `progress` is the weight so far.
    Progress {
        progress: u8,
    },
    /// This stimulus set the valve off (`Intact -> Warning`).
    Triggered {
        initiator: Option<u32>,
        /// Chain depth of the triggering stimulus; a link out of the valve
        /// carries `depth + 1`.
        depth: u8,
    },
}

/// A state change of one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValveEdge {
    /// The warning ended; the vent opens.
    VentStarted,
    /// The vent ended: the valve is spent for the attempt.
    Spent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Valve {
    cfg: ValveConfig,
    state: ValveState,
    ticks_left: u16,
    progress: u8,
    melee_tokens: Vec<u32>,
    launches: Vec<u32>,
    blasts: Vec<u32>,
    projectiles: Vec<u32>,
    shoves: Vec<u32>,
    initiator: Option<u32>,
    depth: u8,
}

impl Valve {
    pub fn new(cfg: ValveConfig) -> Result<Self, String> {
        cfg.validate()?;
        Ok(Self {
            cfg,
            state: ValveState::Intact,
            ticks_left: 0,
            progress: 0,
            melee_tokens: Vec::new(),
            launches: Vec::new(),
            blasts: Vec::new(),
            projectiles: Vec::new(),
            shoves: Vec::new(),
            initiator: None,
            depth: 0,
        })
    }

    pub fn config(&self) -> &ValveConfig {
        &self.cfg
    }
    pub fn state(&self) -> ValveState {
        self.state
    }
    pub fn progress(&self) -> u8 {
        self.progress
    }
    /// The player credited with the vent.
    pub fn initiator(&self) -> Option<u32> {
        self.initiator
    }
    /// Chain depth of the stimulus that set the valve off.
    pub fn trigger_depth(&self) -> u8 {
        self.depth
    }
    /// Restart the attempt: the same state as [`Valve::new`].
    pub fn reset(&mut self) {
        *self = Self::new(self.cfg).expect("a built valve has a valid config");
    }

    fn add(&mut self, weight: u8, stimulus: &Stimulus) -> Offer {
        self.progress = self.progress.saturating_add(weight);
        if self.progress >= self.cfg.trigger_weight {
            self.state = ValveState::Warning;
            self.ticks_left = self.cfg.warning_ticks;
            self.initiator = stimulus.initiator();
            self.depth = stimulus.depth();
            Offer::Triggered {
                initiator: self.initiator,
                depth: self.depth,
            }
        } else {
            Offer::Progress {
                progress: self.progress,
            }
        }
    }

    /// Offer one stimulus. Only an intact valve reacts; every ignored case is
    /// named.
    pub fn offer(&mut self, stimulus: Stimulus) -> Offer {
        if self.state != ValveState::Intact {
            return Offer::Ignored(Ignored::AlreadyActivated);
        }
        match stimulus {
            Stimulus::MeleeHit { token, weight, .. } => {
                if self.melee_tokens.contains(&token) {
                    return Offer::Ignored(Ignored::Duplicate);
                }
                self.melee_tokens.push(token);
                self.add(weight.max(1), &stimulus)
            }
            Stimulus::LaunchedBody {
                launch_id,
                impulse_px_s,
                ..
            } => {
                if impulse_px_s < self.cfg.launch_min_impulse_px_s {
                    return Offer::Ignored(Ignored::TooWeak);
                }
                if self.launches.contains(&launch_id) {
                    return Offer::Ignored(Ignored::Duplicate);
                }
                self.launches.push(launch_id);
                self.add(self.cfg.launch_weight, &stimulus)
            }
            Stimulus::Blast { blast_id, .. } => {
                if self.blasts.contains(&blast_id) {
                    return Offer::Ignored(Ignored::Duplicate);
                }
                self.blasts.push(blast_id);
                self.add(self.cfg.blast_weight, &stimulus)
            }
            Stimulus::Projectile {
                projectile_id,
                weight,
                ..
            } => {
                if self.cfg.projectile_weight == 0 {
                    return Offer::Ignored(Ignored::NotAccepted);
                }
                if self.projectiles.contains(&projectile_id) {
                    return Offer::Ignored(Ignored::Duplicate);
                }
                self.projectiles.push(projectile_id);
                // The projectile's own weight, at least the valve's per-hit
                // weight: a missile never counts for less than a jab.
                self.add(weight.max(self.cfg.projectile_weight), &stimulus)
            }
            Stimulus::Shove {
                shove_id,
                impulse_px_s,
                ..
            } => {
                if self.cfg.shove_weight == 0 {
                    return Offer::Ignored(Ignored::NotAccepted);
                }
                if impulse_px_s < self.cfg.shove_min_impulse_px_s {
                    return Offer::Ignored(Ignored::ShoveTooWeak);
                }
                if self.shoves.contains(&shove_id) {
                    return Offer::Ignored(Ignored::Duplicate);
                }
                self.shoves.push(shove_id);
                self.add(self.cfg.shove_weight, &stimulus)
            }
            Stimulus::PropContact { depth, .. } => {
                if depth >= MAX_CHAIN_DEPTH {
                    return Offer::Ignored(Ignored::ChainTooDeep);
                }
                if self.cfg.contact_weight == 0 {
                    return Offer::Ignored(Ignored::NotAccepted);
                }
                self.add(self.cfg.contact_weight, &stimulus)
            }
        }
    }

    /// One fixed tick: the warning counts down into the vent, the vent into
    /// the spent state.
    pub fn step(&mut self) -> Option<ValveEdge> {
        match self.state {
            ValveState::Warning => {
                self.ticks_left -= 1;
                if self.ticks_left == 0 {
                    self.state = ValveState::Venting;
                    self.ticks_left = self.cfg.vent_ticks;
                    return Some(ValveEdge::VentStarted);
                }
                None
            }
            ValveState::Venting => {
                self.ticks_left -= 1;
                if self.ticks_left == 0 {
                    self.state = ValveState::Spent;
                    return Some(ValveEdge::Spent);
                }
                None
            }
            ValveState::Intact | ValveState::Spent => None,
        }
    }

    /// Fold the full state into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.state.code());
        mix(u64::from(self.ticks_left));
        mix(u64::from(self.progress));
        mix(self.initiator.map_or(0, |p| u64::from(p) + 1));
        mix(u64::from(self.depth));
        for list in [
            &self.melee_tokens,
            &self.launches,
            &self.blasts,
            &self.projectiles,
            &self.shoves,
        ] {
            mix(list.len() as u64);
            for id in list {
                mix(u64::from(*id));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// BOIL-C core: the falling mass.
// ---------------------------------------------------------------------------

/// A remnant this high or lower is stepped over: slot enemies ignore it for
/// body collision, and it is the rise a hero is lifted by when a remnant
/// appears under it (Hoarder re-review P3; the J-B remnant is 16 px, HRD-Q25).
pub const STEP_OVER_PX: i32 = 16;
/// A fall is at most this many ticks (1 s): longer is a drift, not a drop.
/// Adopted default BOILC-AD5.
pub const MAX_FALL_TICKS: u16 = 60;

/// The smallest rectangle that holds both: the **swept** volume of something
/// that moved from `a` to `b` in one tick. Testing the swept volume instead of
/// the end position is what stops a fast mover (or a fast mass) from
/// tunnelling through a thin target between two ticks. It is conservative on a
/// diagonal move, which is the safe side for a hazard.
pub fn swept_rect(a: IRect, b: IRect) -> IRect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    IRect::new(
        x,
        y,
        a.right().max(b.right()) - x,
        a.bottom().max(b.bottom()) - y,
    )
}

/// Slot enemies ignore a remnant of [`STEP_OVER_PX`] or less for body
/// collision (they step over it); a taller remnant is a wall to them.
pub const fn slot_enemy_ignores_remnant(remnant: IRect) -> bool {
    remnant.h <= STEP_OVER_PX
}

/// A body the landing tests: where it stood on the previous tick and where it
/// stands now (layout px, y down). `prev == cur` for a body that did not move.
///
/// `prev` is last tick's rectangle for **continuous motion only**. A teleport,
/// blink, warp or respawn must pass `prev == cur`: the swept volume is the
/// union of the two rectangles, so a discontinuous move would otherwise count
/// as a pass through every footprint on the line between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MassBody {
    pub victim: Victim,
    pub prev: IRect,
    pub cur: IRect,
}

impl MassBody {
    pub const fn still(victim: Victim, rect: IRect) -> Self {
        Self {
            victim,
            prev: rect,
            cur: rect,
        }
    }
}

/// A hero moved straight up onto a remnant that appeared under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemnantLift {
    pub victim: Victim,
    /// Pixels the hero moves **up** (positive): its feet end on the remnant's
    /// top. Never a sideways snap, and never a knock-back out of anything.
    pub up_px: i32,
}

/// The remnant-lift rule: on the tick a remnant appears, every **hero** whose
/// body overlaps it is lifted straight up onto its top. Enemies are not lifted
/// (a slot enemy ignores a low remnant, see [`slot_enemy_ignores_remnant`]).
/// A hero that is hurt or invulnerable is lifted all the same, so no hero is
/// left inside a solid. Order follows `bodies`.
///
/// Precondition (the core cannot see solids): the column above the remnant is
/// clear for the remnant's height plus the tallest hero body, so the lift
/// never ends inside a ceiling or slab. The wire applies the lift as a
/// position write **in the same step** that makes the remnant solid (the
/// remnant is an intact solid from the landing step on), so a lifted hero
/// never spends a tick on a remnant that is not yet solid, where a party
/// push-out could snap it sideways.
pub fn remnant_lifts(remnant: IRect, bodies: &[MassBody]) -> Vec<RemnantLift> {
    let mut out: Vec<RemnantLift> = Vec::new();
    for b in bodies {
        if b.victim.team != Team::Hero
            || !b.cur.overlaps(remnant)
            || out.iter().any(|l| l.victim == b.victim)
        {
            continue;
        }
        out.push(RemnantLift {
            victim: b.victim,
            up_px: b.cur.bottom() - remnant.y,
        });
    }
    out
}

/// The states of a falling mass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassState {
    /// Hanging from its support, untouched.
    Supported,
    /// Part of the release weight is in: visibly damaged, still holding.
    Weakened,
    /// The support let go; the warning runs (creak, dust, footprint shadow).
    Released,
    /// Dropping; the landing is evaluated on the last falling tick.
    Falling,
    /// Landed: the low remnant stands, once per attempt.
    Landed,
}

impl MassState {
    const fn code(self) -> u64 {
        self as u64
    }
}

/// Tunables and geometry of one falling mass (layout px, y down).
///
/// Defaults follow the Hoarder J-B sheet and the Boiler design (the sheet IDs
/// are in `enemy-gameplay-hoarder.md`, the Boiler lines in
/// `floor-01-goblin-boiler-encounter.md` P2):
///
/// * `release_weight` 2, `projectile_weight` 1, a blast releases at once, a
///   shove releases at once when its data sets `shove_weight` to the release
///   weight: HRD-Q26 (Boiler P2: "2 distinct melee hits or 1 actual dynamite
///   blast");
/// * `warning_ticks` 24 and `fall_ticks` 8, landing on R+31: HRD-Q27 (Boiler
///   P2: "24-tick falling-warning");
/// * `damage` 1, knock 120 px/s away from the centre and 140 up, every hero,
///   credited to the initiator: HRD-Q28 (Boiler P2: "player loses at most 1
///   HP");
/// * the 16 px remnant and the footprint geometry of the tests: HRD-Q25;
/// * genuinely new defaults: `launch_weight` 1 (BOILC-AD1), the 160 px/s
///   launch and shove thresholds (BOILC-AD2), `enemy_damage` 255 (BOILC-AD3),
///   `contact_weight` 0 (BOILC-AD4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FallingMassConfig {
    /// The mass while it hangs. It lands `footprint.bottom() - hanging.bottom()`
    /// px lower.
    pub hanging: IRect,
    /// The area the landing evaluation tests, resting on the floor.
    pub footprint: IRect,
    /// The solid that stays after the landing. It rests on the footprint's
    /// bottom edge and lies inside its x range.
    pub remnant: IRect,
    /// Total hit weight that releases the support (default 2, HRD-Q26).
    pub release_weight: u8,
    /// A launched body counts only from this speed (px/s). Default 160, the
    /// valve's value (BOILC-AD2).
    pub launch_min_impulse_px_s: u16,
    /// Weight of a qualifying launched body (one hit by default: it weakens;
    /// BOILC-AD1, Boiler anchor row "can be weakened by ... launched body").
    pub launch_weight: u8,
    /// Weight of a blast (the full release by default, HRD-Q26). It must be at
    /// least `release_weight` for "one blast releases it" to hold: raising
    /// `release_weight` alone does not raise this.
    pub blast_weight: u8,
    /// Weight of a prop contact; 0 (default): the support ignores them until
    /// the BOIL-D link graph exists (BOILC-AD4).
    pub contact_weight: u8,
    /// Weight of one projectile (a distinct light strike, HRD-Q26); 0: ignored.
    pub projectile_weight: u8,
    /// Weight of an enemy shove; 0 (default): ignored. The Hoarder's pile
    /// sets it to the release weight ("released at once").
    pub shove_weight: u8,
    /// A shove counts only from this speed (px/s); 0 removes the threshold.
    /// Default 160, the valve's value; the Hoarder lunge is 180 (BOILC-AD2).
    pub shove_min_impulse_px_s: u16,
    /// Released ticks before the drop (the tell), at least
    /// [`MIN_WARNING_TICKS`]. Default 24 (HRD-Q27).
    pub warning_ticks: u16,
    /// Falling ticks, `1..=`[`MAX_FALL_TICKS`]. Default 8 (HRD-Q27).
    pub fall_ticks: u16,
    /// What the landing does to heroes. Never `OwnerReduced`: a world hazard
    /// has no owner.
    pub hero_damage: HeroDamagePolicy,
    /// Damage to each overlapped hero (when `hero_damage` hurts heroes).
    /// Default 1 (HRD-Q28).
    pub damage: u8,
    /// Whether overlapped enemies are hurt.
    pub hurts_enemies: bool,
    /// Damage requested of each overlapped enemy (255: lethal to any ordinary
    /// body, BOILC-AD3). It is a request: the enemy's own profile maps the hit
    /// (an ordinary goblin dies, a slot boss runs its own reaction, as the
    /// Hoarder's crush does).
    pub enemy_damage: u8,
    /// Horizontal knock of a hit, px/s, away from the footprint's centre
    /// (default 120, HRD-Q28).
    pub knock_x_px_s: i16,
    /// Upward knock of a hit, px/s (default 140, HRD-Q28).
    pub knock_up_px_s: i16,
}

impl FallingMassConfig {
    pub const fn new(hanging: IRect, footprint: IRect, remnant: IRect) -> Self {
        Self {
            hanging,
            footprint,
            remnant,
            release_weight: 2,
            launch_min_impulse_px_s: 160,
            launch_weight: 1,
            blast_weight: 2,
            contact_weight: 0,
            projectile_weight: 1,
            shove_weight: 0,
            shove_min_impulse_px_s: 160,
            warning_ticks: 24,
            fall_ticks: 8,
            hero_damage: HeroDamagePolicy::AllHeroes,
            damage: 1,
            hurts_enemies: true,
            enemy_damage: 255,
            knock_x_px_s: 120,
            knock_up_px_s: 140,
        }
    }

    /// How far the mass drops from its hanging position to the footprint.
    pub const fn drop_px(&self) -> i32 {
        self.footprint.bottom() - self.hanging.bottom()
    }

    pub fn validate(&self) -> Result<(), String> {
        let area = |r: IRect| r.w > 0 && r.h > 0;
        if !area(self.hanging) || !area(self.footprint) || !area(self.remnant) {
            return Err("hanging, footprint and remnant need an area".into());
        }
        if self.hanging.bottom() > self.footprint.y {
            return Err("the mass must hang clear above the footprint".into());
        }
        let r = self.remnant;
        let f = self.footprint;
        if r.bottom() != f.bottom() || r.x < f.x || r.right() > f.right() || r.h > f.h {
            return Err("the remnant must rest on the footprint floor inside its x range".into());
        }
        if self.release_weight == 0 {
            return Err("release_weight must be at least 1".into());
        }
        if self.warning_ticks < MIN_WARNING_TICKS {
            return Err(format!(
                "warning_ticks must be at least {MIN_WARNING_TICKS}, is {}",
                self.warning_ticks
            ));
        }
        if self.fall_ticks == 0 || self.fall_ticks > MAX_FALL_TICKS {
            return Err(format!(
                "fall_ticks must be 1..={MAX_FALL_TICKS}, is {}",
                self.fall_ticks
            ));
        }
        if self.hero_damage == HeroDamagePolicy::OwnerReduced {
            return Err("owner_reduced is for hero-thrown sources, not a world mass".into());
        }
        if self.hero_damage.hurts_any_hero() && self.damage == 0 {
            return Err("a mass that hurts heroes needs damage of at least 1".into());
        }
        if self.hurts_enemies && self.enemy_damage == 0 {
            return Err("a mass that hurts enemies needs enemy_damage of at least 1".into());
        }
        Ok(())
    }
}

/// One victim of the landing: at most one per victim (the one evaluation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LandingHit {
    pub victim: Victim,
    pub damage: u8,
    /// Signed horizontal knock (px/s), away from the footprint's centre; a
    /// victim exactly on the centre line goes toward +x.
    pub knock_x_px_s: i16,
    pub knock_up_px_s: i16,
    /// The player credited with the fall, whatever the final cause.
    pub initiator: Option<u32>,
}

impl LandingHit {
    /// Falling masses are never evadable (DM-Q18).
    pub const fn evadable(&self) -> bool {
        false
    }
}

/// What the one landing evaluation produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Landing {
    pub hits: Vec<LandingHit>,
    /// Heroes lifted onto the remnant (the remnant stands from this tick).
    pub lifts: Vec<RemnantLift>,
    pub remnant: IRect,
    pub initiator: Option<u32>,
}

/// A state change of one tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MassEdge {
    /// The warning ended; the drop starts. Emitted in the step of the last
    /// released tick (R+23); the state is `Falling` from R+24.
    FallStarted,
    /// The last falling tick: the one landing evaluation ran.
    Landed(Landing),
}

/// The falling mass: `Supported -> Weakened -> Released -> Falling -> Landed`.
///
/// Tick contract (the future wire step): the stimuli of the tick are offered,
/// then [`FallingMass::step`] runs once with the victim bodies. A release on
/// tick `R` leaves the mass `Released` during `R..=R + warning - 1`,
/// `Falling` during the next `fall_ticks`, and the landing is evaluated in the
/// step of the last of them (`R + warning + fall_ticks - 1`; with the defaults
/// R + 31); the mass is `Landed` from the following tick, and the remnant is
/// a solid from that same step on.
///
/// **Damage authority.** [`FallingMass::step`] (its landing evaluation) is the
/// only damage source of a weight. The `env_hazard` falling-mass volume and
/// the sidecar `weight_fall` hazard entry are **data sources only**: they are
/// never triggered as a damage volume, or a hero would be hurt twice and the
/// volume would evaluate on R+24 instead of R+31. BOIL-C wire maps the
/// volume's `rect` to `footprint`, `telegraph_ticks` to `warning_ticks`, and
/// `damage`, `hero_damage` and `hurts_enemies` to the config. Its `push_x` is
/// 0 in the Boiler sidecar while the core default knock is 120 / 140 (the data
/// wins per room), `fall_ticks` and the upward knock have no sidecar field
/// yet (defaults 8 and 140).
///
/// Adopted defaults of the mechanics: a tied centre knocks toward +x
/// (BOILC-AD6); the sweep is a bounding-box union, conservative, which also
/// hits a hero that leaves the footprint on the landing tick (BOILC-AD7); the
/// mass follows `drop * k^2 / n^2`, not the repo's gravity constant (BOILC-AD8).
///
/// Integer math only; no RNG, no wall-clock time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallingMass {
    cfg: FallingMassConfig,
    state: MassState,
    ticks_left: u16,
    /// Falling ticks done.
    fallen: u16,
    progress: u8,
    melee_tokens: Vec<u32>,
    launches: Vec<u32>,
    blasts: Vec<u32>,
    projectiles: Vec<u32>,
    shoves: Vec<u32>,
    /// Victims already evaluated by the landing (the one outcome per victim).
    evaluated: Vec<Victim>,
    initiator: Option<u32>,
    depth: u8,
}

impl FallingMass {
    pub fn new(cfg: FallingMassConfig) -> Result<Self, String> {
        cfg.validate()?;
        Ok(Self {
            cfg,
            state: MassState::Supported,
            ticks_left: 0,
            fallen: 0,
            progress: 0,
            melee_tokens: Vec::new(),
            launches: Vec::new(),
            blasts: Vec::new(),
            projectiles: Vec::new(),
            shoves: Vec::new(),
            evaluated: Vec::new(),
            initiator: None,
            depth: 0,
        })
    }

    pub fn config(&self) -> &FallingMassConfig {
        &self.cfg
    }
    pub fn state(&self) -> MassState {
        self.state
    }
    /// Hit weight counted so far.
    pub fn progress(&self) -> u8 {
        self.progress
    }
    /// The player credited with the fall.
    pub fn initiator(&self) -> Option<u32> {
        self.initiator
    }
    /// Chain depth of the stimulus that released the support; a link out of
    /// the landing carries `depth + 1`.
    pub fn trigger_depth(&self) -> u8 {
        self.depth
    }
    /// Restart the attempt: the same state as [`FallingMass::new`].
    pub fn reset(&mut self) {
        *self = Self::new(self.cfg).expect("a built mass has a valid config");
    }

    /// Whether the support can still be hit (the hit target is live).
    pub fn is_supported(&self) -> bool {
        matches!(self.state, MassState::Supported | MassState::Weakened)
    }

    /// The solid remnant, once landed.
    pub fn remnant(&self) -> Option<IRect> {
        (self.state == MassState::Landed).then_some(self.cfg.remnant)
    }

    /// The floor shadow the warning draws and the nav feed exports: the
    /// footprint, while released or falling.
    pub fn telegraph_rect(&self) -> Option<IRect> {
        matches!(self.state, MassState::Released | MassState::Falling).then_some(self.cfg.footprint)
    }

    /// Drop distance after `k` falling ticks: a constant-gravity curve
    /// `drop * k^2 / fall_ticks^2`, exactly `drop` at the last tick.
    fn offset_after(&self, k: u16) -> i32 {
        let n = i64::from(self.cfg.fall_ticks);
        let k = i64::from(k);
        let d = i64::from(self.cfg.drop_px());
        i32::try_from(d * k * k / (n * n)).expect("the offset is at most the drop")
    }

    /// The mass now (`None` once it has become the remnant).
    pub fn mass_rect(&self) -> Option<IRect> {
        if self.state == MassState::Landed {
            return None;
        }
        let mut r = self.cfg.hanging;
        r.y += self.offset_after(self.fallen);
        Some(r)
    }

    /// The volume the mass crossed in the last falling tick (its swept path),
    /// the union of its rectangle before and after. The paths of consecutive
    /// ticks touch end to end, so even a very tall drop leaves no gap.
    pub fn swept_mass_rect(&self) -> Option<IRect> {
        if self.state != MassState::Falling || self.fallen == 0 {
            return None;
        }
        let now = self.mass_rect()?;
        let mut before = self.cfg.hanging;
        before.y += self.offset_after(self.fallen - 1);
        Some(swept_rect(before, now))
    }

    fn add(&mut self, weight: u8, stimulus: &Stimulus) -> Offer {
        self.progress = self.progress.saturating_add(weight);
        if self.progress >= self.cfg.release_weight {
            self.state = MassState::Released;
            self.ticks_left = self.cfg.warning_ticks;
            self.initiator = stimulus.initiator();
            self.depth = stimulus.depth();
            Offer::Triggered {
                initiator: self.initiator,
                depth: self.depth,
            }
        } else {
            self.state = MassState::Weakened;
            Offer::Progress {
                progress: self.progress,
            }
        }
    }

    /// Offer one stimulus. Only a hanging mass reacts (`Supported` or
    /// `Weakened`); every ignored case is named. `Offer::Progress` means the
    /// support is now `Weakened`; `Offer::Triggered` that it let go.
    pub fn offer(&mut self, stimulus: Stimulus) -> Offer {
        if !self.is_supported() {
            return Offer::Ignored(Ignored::AlreadyActivated);
        }
        let cfg = self.cfg;
        match stimulus {
            Stimulus::MeleeHit { token, weight, .. } => {
                if self.melee_tokens.contains(&token) {
                    return Offer::Ignored(Ignored::Duplicate);
                }
                self.melee_tokens.push(token);
                self.add(weight.max(1), &stimulus)
            }
            Stimulus::LaunchedBody {
                launch_id,
                impulse_px_s,
                ..
            } => {
                if cfg.launch_weight == 0 {
                    return Offer::Ignored(Ignored::NotAccepted);
                }
                if impulse_px_s < cfg.launch_min_impulse_px_s {
                    return Offer::Ignored(Ignored::TooWeak);
                }
                if self.launches.contains(&launch_id) {
                    return Offer::Ignored(Ignored::Duplicate);
                }
                self.launches.push(launch_id);
                self.add(cfg.launch_weight, &stimulus)
            }
            Stimulus::Blast { blast_id, .. } => {
                if cfg.blast_weight == 0 {
                    return Offer::Ignored(Ignored::NotAccepted);
                }
                if self.blasts.contains(&blast_id) {
                    return Offer::Ignored(Ignored::Duplicate);
                }
                self.blasts.push(blast_id);
                self.add(cfg.blast_weight, &stimulus)
            }
            Stimulus::Projectile {
                projectile_id,
                weight,
                ..
            } => {
                if cfg.projectile_weight == 0 {
                    return Offer::Ignored(Ignored::NotAccepted);
                }
                if self.projectiles.contains(&projectile_id) {
                    return Offer::Ignored(Ignored::Duplicate);
                }
                self.projectiles.push(projectile_id);
                self.add(weight.max(cfg.projectile_weight), &stimulus)
            }
            Stimulus::Shove {
                shove_id,
                impulse_px_s,
                ..
            } => {
                if cfg.shove_weight == 0 {
                    return Offer::Ignored(Ignored::NotAccepted);
                }
                if impulse_px_s < cfg.shove_min_impulse_px_s {
                    return Offer::Ignored(Ignored::ShoveTooWeak);
                }
                if self.shoves.contains(&shove_id) {
                    return Offer::Ignored(Ignored::Duplicate);
                }
                self.shoves.push(shove_id);
                self.add(cfg.shove_weight, &stimulus)
            }
            Stimulus::PropContact { depth, .. } => {
                if depth >= MAX_CHAIN_DEPTH {
                    return Offer::Ignored(Ignored::ChainTooDeep);
                }
                if cfg.contact_weight == 0 {
                    return Offer::Ignored(Ignored::NotAccepted);
                }
                self.add(cfg.contact_weight, &stimulus)
            }
        }
    }

    /// The structure-break entry point for a blast: the one call a generic
    /// `BlastTarget` hook (CP-C) forwards to. It takes only what a blast
    /// carries (its id and its owner), never a state of this prop, so the hook
    /// stays independent of the machine behind it.
    pub fn break_by_blast(&mut self, blast_id: u32, owner: Option<u32>) -> Offer {
        self.offer(Stimulus::Blast { blast_id, owner })
    }

    /// One fixed tick, after the stimuli. `bodies` are the victims the landing
    /// would test; they are only read on the landing tick.
    pub fn step(&mut self, bodies: &[MassBody]) -> Option<MassEdge> {
        match self.state {
            MassState::Released => {
                self.ticks_left -= 1;
                if self.ticks_left == 0 {
                    self.state = MassState::Falling;
                    self.ticks_left = self.cfg.fall_ticks;
                    self.fallen = 0;
                    return Some(MassEdge::FallStarted);
                }
                None
            }
            MassState::Falling => {
                self.ticks_left -= 1;
                self.fallen += 1;
                if self.ticks_left == 0 {
                    self.state = MassState::Landed;
                    return Some(MassEdge::Landed(self.land(bodies)));
                }
                None
            }
            MassState::Supported | MassState::Weakened | MassState::Landed => None,
        }
    }

    /// The one swept landing evaluation: each body whose swept volume (the
    /// union of its previous and current rectangles) overlaps the footprint is
    /// hit once. Heroes by the damage policy, enemies by `hurts_enemies`. Then
    /// the remnant appears and lifts the heroes standing in it.
    fn land(&mut self, bodies: &[MassBody]) -> Landing {
        let cfg = self.cfg;
        let centre_x2 = cfg.footprint.x * 2 + cfg.footprint.w;
        let mut hits = Vec::new();
        for b in bodies {
            if self.evaluated.contains(&b.victim) {
                continue;
            }
            if !swept_rect(b.prev, b.cur).overlaps(cfg.footprint) {
                continue;
            }
            let damage = match b.victim.team {
                Team::Hero if cfg.hero_damage.hurts_any_hero() => cfg.damage,
                Team::Enemy if cfg.hurts_enemies => cfg.enemy_damage,
                _ => continue,
            };
            self.evaluated.push(b.victim);
            let body_centre_x2 = b.cur.x * 2 + b.cur.w;
            let away = if body_centre_x2 < centre_x2 { -1 } else { 1 };
            hits.push(LandingHit {
                victim: b.victim,
                damage,
                knock_x_px_s: cfg.knock_x_px_s.saturating_mul(away),
                knock_up_px_s: cfg.knock_up_px_s,
                initiator: self.initiator,
            });
        }
        Landing {
            hits,
            lifts: remnant_lifts(cfg.remnant, bodies),
            remnant: cfg.remnant,
            initiator: self.initiator,
        }
    }

    /// Fold the full state into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.state.code());
        mix(u64::from(self.ticks_left));
        mix(u64::from(self.fallen));
        mix(u64::from(self.progress));
        mix(self.initiator.map_or(0, |p| u64::from(p) + 1));
        mix(u64::from(self.depth));
        for list in [
            &self.melee_tokens,
            &self.launches,
            &self.blasts,
            &self.projectiles,
            &self.shoves,
        ] {
            mix(list.len() as u64);
            for id in list {
                mix(u64::from(*id));
            }
        }
        mix(self.evaluated.len() as u64);
        for v in &self.evaluated {
            mix(u64::from(v.team == Team::Enemy));
            mix(u64::from(v.id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valve() -> Valve {
        Valve::new(ValveConfig::DEFAULT).unwrap()
    }

    fn jab(token: u32, player: u32) -> Stimulus {
        Stimulus::MeleeHit {
            token,
            player,
            weight: 1,
        }
    }

    fn launch(id: u32, speed: u16) -> Stimulus {
        Stimulus::LaunchedBody {
            launch_id: id,
            impulse_px_s: speed,
            initiator: Some(0),
        }
    }

    fn hash(v: &Valve) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325_u64;
        v.mix_into(&mut |x| {
            h ^= x;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        });
        h
    }

    #[test]
    fn two_distinct_light_strikes_set_it_off_and_the_second_player_is_credited() {
        let mut v = valve();
        assert_eq!(v.offer(jab(1, 0)), Offer::Progress { progress: 1 });
        assert_eq!(v.state(), ValveState::Intact);
        assert_eq!(
            v.offer(jab(2, 1)),
            Offer::Triggered {
                initiator: Some(1),
                depth: 0
            }
        );
        assert_eq!(v.state(), ValveState::Warning);
        assert_eq!(v.initiator(), Some(1));
    }

    #[test]
    fn one_move_instance_counts_once() {
        let mut v = valve();
        assert_eq!(v.offer(jab(7, 0)), Offer::Progress { progress: 1 });
        assert_eq!(v.offer(jab(7, 0)), Offer::Ignored(Ignored::Duplicate));
        assert_eq!(v.progress(), 1);
        assert_eq!(v.state(), ValveState::Intact);
    }

    #[test]
    fn a_heavy_move_weight_sets_it_off_alone() {
        let mut v = valve();
        let heavy = Stimulus::MeleeHit {
            token: 1,
            player: 0,
            weight: 2,
        };
        assert!(matches!(v.offer(heavy), Offer::Triggered { .. }));
        // A zero weight is read as 1: a hit never counts for nothing.
        let mut v = valve();
        let zero = Stimulus::MeleeHit {
            token: 1,
            player: 0,
            weight: 0,
        };
        assert_eq!(v.offer(zero), Offer::Progress { progress: 1 });
    }

    #[test]
    fn a_launched_body_needs_the_impulse_and_counts_once_per_launch() {
        let mut v = valve();
        assert_eq!(v.offer(launch(1, 150)), Offer::Ignored(Ignored::TooWeak));
        assert_eq!(v.offer(launch(1, 159)), Offer::Ignored(Ignored::TooWeak));
        assert_eq!(v.state(), ValveState::Intact);
        assert!(matches!(v.offer(launch(1, 160)), Offer::Triggered { .. }));
        assert_eq!(v.state(), ValveState::Warning);

        // With a weaker launch weight the same launch id still counts once.
        let cfg = ValveConfig {
            launch_weight: 1,
            ..ValveConfig::DEFAULT
        };
        let mut v = Valve::new(cfg).unwrap();
        assert_eq!(v.offer(launch(5, 250)), Offer::Progress { progress: 1 });
        assert_eq!(v.offer(launch(5, 250)), Offer::Ignored(Ignored::Duplicate));
        assert!(matches!(v.offer(launch(6, 250)), Offer::Triggered { .. }));
    }

    #[test]
    fn a_blast_sets_it_off_and_keeps_its_owner() {
        let mut v = valve();
        let blast = Stimulus::Blast {
            blast_id: 3,
            owner: Some(0),
        };
        assert_eq!(
            v.offer(blast),
            Offer::Triggered {
                initiator: Some(0),
                depth: 0
            }
        );
        // An authored blast has no owner: nobody is credited.
        let mut v = valve();
        let drum = Stimulus::Blast {
            blast_id: 9,
            owner: None,
        };
        assert_eq!(
            v.offer(drum),
            Offer::Triggered {
                initiator: None,
                depth: 0
            }
        );
    }

    fn missile(id: u32, owner: Option<u32>) -> Stimulus {
        Stimulus::Projectile {
            projectile_id: id,
            owner,
            weight: 1,
        }
    }

    fn shove(id: u32, speed: u16, credit: Option<u32>) -> Stimulus {
        Stimulus::Shove {
            shove_id: id,
            impulse_px_s: speed,
            source: 5,
            credit,
        }
    }

    #[test]
    fn projectiles_count_once_each_like_distinct_light_strikes_and_keep_their_owner() {
        let mut v = valve();
        assert_eq!(
            v.offer(missile(1, Some(1))),
            Offer::Progress { progress: 1 }
        );
        assert_eq!(
            v.offer(missile(1, Some(1))),
            Offer::Ignored(Ignored::Duplicate)
        );
        // A missile and a jab add up: two distinct light hits.
        assert_eq!(
            v.offer(jab(9, 0)),
            Offer::Triggered {
                initiator: Some(0),
                depth: 0
            }
        );
        // Two missiles do it alone; the second owner is credited.
        let mut v = valve();
        v.offer(missile(1, Some(0)));
        assert_eq!(
            v.offer(missile(2, Some(1))),
            Offer::Triggered {
                initiator: Some(1),
                depth: 0
            }
        );
        // A heavier projectile (weight 2) opens it alone; an ownerless one
        // credits nobody.
        let mut v = valve();
        let heavy = Stimulus::Projectile {
            projectile_id: 3,
            owner: None,
            weight: 2,
        };
        assert_eq!(
            v.offer(heavy),
            Offer::Triggered {
                initiator: None,
                depth: 0
            }
        );
        // A valve that takes no projectiles says so.
        let cfg = ValveConfig {
            projectile_weight: 0,
            ..ValveConfig::DEFAULT
        };
        let mut v = Valve::new(cfg).unwrap();
        assert_eq!(
            v.offer(missile(1, Some(0))),
            Offer::Ignored(Ignored::NotAccepted)
        );
    }

    #[test]
    fn a_shove_is_not_accepted_by_default_and_credits_only_who_the_data_says() {
        let mut v = valve();
        assert_eq!(
            v.offer(shove(1, 400, None)),
            Offer::Ignored(Ignored::NotAccepted)
        );
        assert_eq!(v.state(), ValveState::Intact);
        let cfg = ValveConfig {
            shove_weight: 2,
            ..ValveConfig::DEFAULT
        };
        let mut v = Valve::new(cfg).unwrap();
        assert_eq!(
            v.offer(shove(1, 159, None)),
            Offer::Ignored(Ignored::ShoveTooWeak)
        );
        // The Hoarder's 180 px/s lunge counts at the default threshold, and a
        // baited shove credits the baited hero's player.
        assert_eq!(
            v.offer(shove(1, 180, Some(1))),
            Offer::Triggered {
                initiator: Some(1),
                depth: 0
            }
        );
        assert_eq!(v.initiator(), Some(1));
        // A shove with no credit (a bot hero, an unbaited shove) credits nobody.
        let mut v = Valve::new(cfg).unwrap();
        assert_eq!(
            v.offer(shove(2, 180, None)),
            Offer::Triggered {
                initiator: None,
                depth: 0
            }
        );
        assert_eq!(v.initiator(), None);
        // One shove id counts once.
        let cfg = ValveConfig {
            shove_weight: 1,
            ..ValveConfig::DEFAULT
        };
        let mut v = Valve::new(cfg).unwrap();
        assert_eq!(
            v.offer(shove(4, 300, None)),
            Offer::Progress { progress: 1 }
        );
        assert_eq!(
            v.offer(shove(4, 300, None)),
            Offer::Ignored(Ignored::Duplicate)
        );
        // A zero threshold accepts any shove.
        let cfg = ValveConfig {
            shove_weight: 2,
            shove_min_impulse_px_s: 0,
            ..ValveConfig::DEFAULT
        };
        let mut v = Valve::new(cfg).unwrap();
        assert!(matches!(
            v.offer(shove(5, 1, None)),
            Offer::Triggered { .. }
        ));
    }

    /// BOIL-03: "kick a living goblin into the valve". The threshold is checked
    /// against the real launch constants, so a retune cannot silently break it.
    #[test]
    fn the_living_kick_and_every_lethal_launch_open_the_valve_and_the_relaunch_does_not() {
        use crate::cf02::Cf02Tuning;
        use crate::feel::{KICK_LAUNCH_VX, LETHAL_KNOCK_VX};
        let t = Cf02Tuning::pinned().expect("the pinned CF02 tuning");
        let opens = |speed: f32| {
            let mut v = valve();
            let r = v.offer(Stimulus::LaunchedBody {
                launch_id: 1,
                impulse_px_s: speed.round() as u16,
                initiator: Some(0),
            });
            matches!(r, Offer::Triggered { .. })
        };
        // Opens: the living goblin's kick launch, the legacy lethal knock and
        // every CF02 lethal launch.
        for (name, speed) in [
            ("living kick", KICK_LAUNCH_VX),
            ("legacy lethal knock", LETHAL_KNOCK_VX),
            ("lethal kick", f32::from(t.q06_lethal_kick_knock_x)),
            ("lethal melee", f32::from(t.q06_lethal_melee_knock_x)),
            ("lethal air kick", f32::from(t.q51_lethal_launch_x)),
        ] {
            assert!(opens(speed), "{name} ({speed} px/s) must open the valve");
        }
        // Does not open: the relaunch of a downed goblin and the air-kick knock.
        for (name, speed) in [
            ("relaunch", f32::from(t.q51_relaunch_x)),
            ("air-kick knock", f32::from(t.q50_air_kick_knock_x)),
        ] {
            assert!(!opens(speed), "{name} ({speed} px/s) must not open it");
        }
        // The sidecar and the default agree on the number.
        assert_eq!(ValveConfig::DEFAULT.launch_min_impulse_px_s, 160);
    }

    #[test]
    fn a_prop_contact_is_not_accepted_by_default_and_never_past_the_depth_cap() {
        let mut v = valve();
        let contact = |depth| Stimulus::PropContact {
            source: 1,
            initiator: Some(0),
            depth,
        };
        assert_eq!(v.offer(contact(1)), Offer::Ignored(Ignored::NotAccepted));
        assert_eq!(
            v.offer(contact(MAX_CHAIN_DEPTH)),
            Offer::Ignored(Ignored::ChainTooDeep)
        );
        // A valve that does accept contacts carries the chain depth on.
        let cfg = ValveConfig {
            contact_weight: 2,
            ..ValveConfig::DEFAULT
        };
        let mut v = Valve::new(cfg).unwrap();
        assert_eq!(
            v.offer(contact(MAX_CHAIN_DEPTH)),
            Offer::Ignored(Ignored::ChainTooDeep)
        );
        assert_eq!(
            v.offer(contact(MAX_CHAIN_DEPTH - 1)),
            Offer::Triggered {
                initiator: Some(0),
                depth: MAX_CHAIN_DEPTH - 1
            }
        );
        assert_eq!(v.trigger_depth(), MAX_CHAIN_DEPTH - 1);
    }

    #[test]
    fn the_state_machine_runs_intact_warning_venting_spent_with_exact_lengths() {
        let mut v = valve();
        v.offer(launch(1, 300));
        let mut warning = 0;
        let mut venting = 0;
        let mut edges = Vec::new();
        for _ in 0..300 {
            match v.state() {
                ValveState::Warning => warning += 1,
                ValveState::Venting => venting += 1,
                _ => {}
            }
            if let Some(edge) = v.step() {
                edges.push(edge);
            }
        }
        assert_eq!((warning, venting), (24, 72));
        assert_eq!(edges, [ValveEdge::VentStarted, ValveEdge::Spent]);
        assert_eq!(v.state(), ValveState::Spent);
    }

    #[test]
    fn a_warning_or_spent_valve_ignores_everything() {
        let mut v = valve();
        v.offer(launch(1, 300));
        for stimulus in [
            jab(9, 0),
            launch(2, 400),
            Stimulus::Blast {
                blast_id: 1,
                owner: None,
            },
        ] {
            assert_eq!(v.offer(stimulus), Offer::Ignored(Ignored::AlreadyActivated));
        }
        for _ in 0..24 + 72 {
            v.step();
        }
        assert_eq!(v.state(), ValveState::Spent);
        let before = hash(&v);
        assert_eq!(
            v.offer(jab(10, 0)),
            Offer::Ignored(Ignored::AlreadyActivated)
        );
        assert_eq!(
            hash(&v),
            before,
            "a repeat strike after spent changes nothing"
        );
    }

    #[test]
    fn reset_restores_the_initial_state_exactly() {
        let fresh = hash(&valve());
        let mut v = valve();
        v.offer(jab(1, 0));
        v.offer(launch(2, 300));
        for _ in 0..40 {
            v.step();
        }
        assert_ne!(hash(&v), fresh);
        v.reset();
        assert_eq!(hash(&v), fresh);
        assert_eq!(v, valve());
    }

    #[test]
    fn the_config_rejects_a_missing_or_unreadable_warning() {
        let short = ValveConfig {
            warning_ticks: MIN_WARNING_TICKS - 1,
            ..ValveConfig::DEFAULT
        };
        assert!(Valve::new(short).is_err());
        let zero = ValveConfig {
            trigger_weight: 0,
            ..ValveConfig::DEFAULT
        };
        assert!(Valve::new(zero).is_err());
        let no_vent = ValveConfig {
            vent_ticks: 0,
            ..ValveConfig::DEFAULT
        };
        assert!(Valve::new(no_vent).is_err());
    }

    #[test]
    fn stimulus_accessors_carry_initiator_kind_and_depth() {
        assert_eq!(jab(1, 3).initiator(), Some(3));
        assert_eq!(jab(1, 3).kind(), StimulusKind::MeleeHit);
        assert_eq!(launch(1, 1).kind(), StimulusKind::LaunchedBody);
        let contact = Stimulus::PropContact {
            source: 4,
            initiator: None,
            depth: 2,
        };
        assert_eq!((contact.initiator(), contact.depth()), (None, 2));
        assert_eq!(contact.kind(), StimulusKind::PropContact);
        assert_eq!(missile(1, Some(2)).kind(), StimulusKind::Projectile);
        assert_eq!(missile(1, Some(2)).initiator(), Some(2));
        assert_eq!(shove(1, 1, None).kind(), StimulusKind::Shove);
        assert_eq!(
            (shove(1, 1, None).initiator(), shove(1, 1, None).depth()),
            (None, 0)
        );
        assert_eq!(shove(1, 1, Some(3)).initiator(), Some(3));
        assert_eq!(
            Stimulus::Blast {
                blast_id: 1,
                owner: Some(1)
            }
            .kind(),
            StimulusKind::Blast
        );
        assert_eq!(jab(1, 0).depth(), 0);
    }

    #[test]
    fn the_hash_follows_the_progress_and_the_counted_tokens() {
        let mut a = valve();
        let mut b = valve();
        a.offer(jab(1, 0));
        b.offer(jab(2, 0));
        assert_ne!(hash(&a), hash(&b), "different tokens, different state");
        let mut c = valve();
        c.offer(jab(1, 0));
        assert_eq!(hash(&a), hash(&c));
        let mut d = valve();
        let mut e = valve();
        d.offer(missile(1, None));
        e.offer(missile(2, None));
        assert_ne!(hash(&d), hash(&e), "projectile ids are state");
    }
}

#[cfg(test)]
mod falling_mass_tests {
    use super::*;

    const FLOOR: i32 = 304;

    /// The Hoarder J-B geometry on a 304 px walking line (y down): stack
    /// `[752, 880) x [96, 160)` above the floor, footprint `[0, 48)`, remnant
    /// 16 px.
    fn jb() -> FallingMassConfig {
        FallingMassConfig::new(
            IRect::new(752, FLOOR - 160, 128, 64),
            IRect::new(752, FLOOR - 48, 128, 48),
            IRect::new(752, FLOOR - 16, 128, 16),
        )
    }

    fn mass() -> FallingMass {
        FallingMass::new(jb()).unwrap()
    }

    fn jab(token: u32, player: u32) -> Stimulus {
        Stimulus::MeleeHit {
            token,
            player,
            weight: 1,
        }
    }

    fn launch(id: u32, speed: u16) -> Stimulus {
        Stimulus::LaunchedBody {
            launch_id: id,
            impulse_px_s: speed,
            initiator: Some(0),
        }
    }

    fn shove(id: u32, speed: u16, credit: Option<u32>) -> Stimulus {
        Stimulus::Shove {
            shove_id: id,
            impulse_px_s: speed,
            source: 5,
            credit,
        }
    }

    /// A 28 x 56 hero body standing on the floor with its centre at `cx`.
    fn hero_at(slot: u32, cx: i32) -> MassBody {
        MassBody::still(Victim::hero(slot), IRect::new(cx - 14, FLOOR - 56, 28, 56))
    }

    fn goblin_at(id: u32, cx: i32) -> MassBody {
        MassBody::still(Victim::enemy(id), IRect::new(cx - 14, FLOOR - 48, 28, 48))
    }

    fn hash(m: &FallingMass) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325_u64;
        m.mix_into(&mut |x| {
            h ^= x;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        });
        h
    }

    /// Release on tick 0, then step with `bodies` until it lands. Returns the
    /// tick of the landing step and the landing.
    fn run_to_landing(m: &mut FallingMass, bodies: &[MassBody]) -> (u32, Landing) {
        for tick in 0..400 {
            if let Some(MassEdge::Landed(l)) = m.step(bodies) {
                return (tick, l);
            }
        }
        panic!("the mass never landed");
    }

    #[test]
    fn the_config_rejects_bad_geometry_timing_and_policy() {
        assert!(FallingMass::new(jb()).is_ok());
        let bad = |f: fn(&mut FallingMassConfig)| {
            let mut c = jb();
            f(&mut c);
            FallingMass::new(c).is_err()
        };
        assert!(bad(|c| c.warning_ticks = MIN_WARNING_TICKS - 1));
        assert!(bad(|c| c.fall_ticks = 0));
        assert!(bad(|c| c.fall_ticks = MAX_FALL_TICKS + 1));
        assert!(bad(|c| c.release_weight = 0));
        assert!(bad(|c| c.hero_damage = HeroDamagePolicy::OwnerReduced));
        assert!(bad(|c| c.damage = 0));
        assert!(bad(|c| c.enemy_damage = 0));
        // The mass hangs clear above the footprint.
        assert!(bad(|c| c.hanging.y = FLOOR - 100));
        // The remnant rests on the floor, inside the footprint's x range.
        assert!(bad(|c| c.remnant.y -= 4));
        assert!(bad(|c| c.remnant.x -= 8));
        assert!(bad(|c| c.remnant.w += 8));
        // A harmless mass is allowed: it still lands and leaves a remnant.
        let mut c = jb();
        c.hero_damage = HeroDamagePolicy::NoHeroes;
        c.hurts_enemies = false;
        assert!(FallingMass::new(c).is_ok());
        assert_eq!(jb().drop_px(), 96);
    }

    #[test]
    fn two_distinct_hits_walk_supported_weakened_released_and_the_second_player_is_credited() {
        let mut m = mass();
        assert_eq!(m.state(), MassState::Supported);
        assert!(m.is_supported());
        assert_eq!(m.offer(jab(1, 0)), Offer::Progress { progress: 1 });
        assert_eq!(m.state(), MassState::Weakened);
        assert!(m.is_supported());
        assert_eq!(
            m.offer(jab(2, 1)),
            Offer::Triggered {
                initiator: Some(1),
                depth: 0
            }
        );
        assert_eq!(m.state(), MassState::Released);
        assert!(!m.is_supported());
        assert_eq!(m.initiator(), Some(1));
    }

    #[test]
    fn one_move_instance_counts_once_and_a_zero_weight_is_read_as_one() {
        let mut m = mass();
        assert_eq!(m.offer(jab(7, 0)), Offer::Progress { progress: 1 });
        assert_eq!(m.offer(jab(7, 0)), Offer::Ignored(Ignored::Duplicate));
        assert_eq!(m.progress(), 1);
        assert_eq!(m.state(), MassState::Weakened);
        let mut m = mass();
        let zero = Stimulus::MeleeHit {
            token: 1,
            player: 0,
            weight: 0,
        };
        assert_eq!(m.offer(zero), Offer::Progress { progress: 1 });
        // A heavy move releases it alone, straight from Supported.
        let mut m = mass();
        let heavy = Stimulus::MeleeHit {
            token: 1,
            player: 0,
            weight: 2,
        };
        assert!(matches!(m.offer(heavy), Offer::Triggered { .. }));
        assert_eq!(m.state(), MassState::Released);
    }

    #[test]
    fn a_blast_releases_it_alone_through_the_blast_entry_point_and_keeps_its_owner() {
        let mut m = mass();
        assert_eq!(
            m.break_by_blast(3, Some(0)),
            Offer::Triggered {
                initiator: Some(0),
                depth: 0
            }
        );
        assert_eq!(m.state(), MassState::Released);
        // The same blast id never counts twice, and an ownerless blast (an
        // authored drum) credits nobody.
        let mut m = mass();
        let one = FallingMassConfig {
            blast_weight: 1,
            ..jb()
        };
        let mut half = FallingMass::new(one).unwrap();
        assert_eq!(
            half.break_by_blast(4, None),
            Offer::Progress { progress: 1 }
        );
        assert_eq!(
            half.break_by_blast(4, None),
            Offer::Ignored(Ignored::Duplicate)
        );
        assert_eq!(
            m.break_by_blast(9, None),
            Offer::Triggered {
                initiator: None,
                depth: 0
            }
        );
        // The entry point is the Blast stimulus, nothing more.
        let mut a = mass();
        let mut b = mass();
        a.break_by_blast(5, Some(1));
        b.offer(Stimulus::Blast {
            blast_id: 5,
            owner: Some(1),
        });
        assert_eq!(a, b);
        // A mass that takes no blasts says so.
        let mut deaf = FallingMass::new(FallingMassConfig {
            blast_weight: 0,
            ..jb()
        })
        .unwrap();
        assert_eq!(
            deaf.break_by_blast(1, None),
            Offer::Ignored(Ignored::NotAccepted)
        );
    }

    #[test]
    fn a_launched_body_weakens_from_the_threshold_once_per_launch() {
        let mut m = mass();
        assert_eq!(m.offer(launch(1, 159)), Offer::Ignored(Ignored::TooWeak));
        assert_eq!(m.state(), MassState::Supported);
        assert_eq!(m.offer(launch(1, 160)), Offer::Progress { progress: 1 });
        assert_eq!(m.state(), MassState::Weakened);
        assert_eq!(m.offer(launch(1, 400)), Offer::Ignored(Ignored::Duplicate));
        // A launch plus a jab releases it.
        assert!(matches!(m.offer(jab(2, 1)), Offer::Triggered { .. }));
        // A launch-deaf mass ignores launches.
        let mut deaf = FallingMass::new(FallingMassConfig {
            launch_weight: 0,
            ..jb()
        })
        .unwrap();
        assert_eq!(
            deaf.offer(launch(1, 400)),
            Offer::Ignored(Ignored::NotAccepted)
        );
    }

    #[test]
    fn a_missile_counts_as_a_distinct_light_strike_once_per_id() {
        let mut m = mass();
        let missile = |id, owner| Stimulus::Projectile {
            projectile_id: id,
            owner,
            weight: 1,
        };
        assert_eq!(
            m.offer(missile(1, Some(1))),
            Offer::Progress { progress: 1 }
        );
        assert_eq!(
            m.offer(missile(1, Some(1))),
            Offer::Ignored(Ignored::Duplicate)
        );
        assert_eq!(
            m.offer(missile(2, Some(0))),
            Offer::Triggered {
                initiator: Some(0),
                depth: 0
            }
        );
        let mut deaf = FallingMass::new(FallingMassConfig {
            projectile_weight: 0,
            ..jb()
        })
        .unwrap();
        assert_eq!(
            deaf.offer(missile(1, None)),
            Offer::Ignored(Ignored::NotAccepted)
        );
    }

    #[test]
    fn a_shove_is_refused_by_default_and_releases_at_once_when_the_data_says_so() {
        let mut m = mass();
        assert_eq!(
            m.offer(shove(1, 400, Some(0))),
            Offer::Ignored(Ignored::NotAccepted)
        );
        let cfg = FallingMassConfig {
            shove_weight: 2,
            ..jb()
        };
        let mut m = FallingMass::new(cfg).unwrap();
        assert_eq!(
            m.offer(shove(1, 159, None)),
            Offer::Ignored(Ignored::ShoveTooWeak)
        );
        // The Hoarder's 180 px/s lunge: released at once, credited to the
        // baited hero's player, or to nobody when the bait was a bot.
        assert_eq!(
            m.offer(shove(1, 180, Some(1))),
            Offer::Triggered {
                initiator: Some(1),
                depth: 0
            }
        );
        assert_eq!(m.state(), MassState::Released);
        let mut m = FallingMass::new(cfg).unwrap();
        assert!(matches!(
            m.offer(shove(2, 180, None)),
            Offer::Triggered {
                initiator: None,
                ..
            }
        ));
    }

    #[test]
    fn a_prop_contact_is_refused_by_default_and_never_past_the_depth_cap() {
        let contact = |depth| Stimulus::PropContact {
            source: 1,
            initiator: Some(0),
            depth,
        };
        let mut m = mass();
        assert_eq!(m.offer(contact(1)), Offer::Ignored(Ignored::NotAccepted));
        let cfg = FallingMassConfig {
            contact_weight: 2,
            ..jb()
        };
        let mut m = FallingMass::new(cfg).unwrap();
        assert_eq!(
            m.offer(contact(MAX_CHAIN_DEPTH)),
            Offer::Ignored(Ignored::ChainTooDeep)
        );
        assert_eq!(
            m.offer(contact(MAX_CHAIN_DEPTH - 1)),
            Offer::Triggered {
                initiator: Some(0),
                depth: MAX_CHAIN_DEPTH - 1
            }
        );
        assert_eq!(m.trigger_depth(), MAX_CHAIN_DEPTH - 1);
    }

    #[test]
    fn a_released_falling_or_landed_mass_ignores_everything() {
        let mut m = mass();
        m.break_by_blast(1, None);
        let all = [
            jab(9, 0),
            launch(2, 400),
            Stimulus::Blast {
                blast_id: 7,
                owner: None,
            },
        ];
        for s in all {
            assert_eq!(m.offer(s), Offer::Ignored(Ignored::AlreadyActivated));
        }
        for _ in 0..24 {
            m.step(&[]);
        }
        assert_eq!(m.state(), MassState::Falling);
        for s in all {
            assert_eq!(m.offer(s), Offer::Ignored(Ignored::AlreadyActivated));
        }
        run_to_landing(&mut m, &[]);
        let before = hash(&m);
        for s in all {
            assert_eq!(m.offer(s), Offer::Ignored(Ignored::AlreadyActivated));
        }
        assert_eq!(hash(&m), before, "a hit after landing changes nothing");
    }

    /// The J-B timeline from the Hoarder sheet: released on tick R, released
    /// R..R+23, falling R+24..R+31, landing evaluated on R+31, landed from R+32.
    #[test]
    fn the_timeline_is_released_24_falling_8_landing_on_r_plus_31_landed_from_r_plus_32() {
        let mut m = mass();
        m.offer(jab(1, 0));
        m.offer(jab(2, 0));
        let mut seen: Vec<(u32, MassState)> = Vec::new();
        let mut edges: Vec<(u32, bool)> = Vec::new();
        for tick in 0..40u32 {
            // The state during the tick, before its step.
            seen.push((tick, m.state()));
            match m.step(&[]) {
                Some(MassEdge::FallStarted) => edges.push((tick, false)),
                Some(MassEdge::Landed(_)) => edges.push((tick, true)),
                None => {}
            }
        }
        let count = |s: MassState| seen.iter().filter(|(_, x)| *x == s).count();
        assert_eq!(count(MassState::Released), 24);
        assert_eq!(count(MassState::Falling), 8);
        for (tick, s) in &seen {
            let want = match tick {
                0..=23 => MassState::Released,
                24..=31 => MassState::Falling,
                _ => MassState::Landed,
            };
            assert_eq!(*s, want, "tick {tick}");
        }
        // The warning ends on R+23 (the fall starts on R+24); the landing
        // evaluation runs on R+31.
        assert_eq!(edges, [(23, false), (31, true)]);
        assert_eq!(m.state(), MassState::Landed);
    }

    #[test]
    fn the_mass_drops_on_a_gravity_curve_and_lands_exactly_on_the_floor() {
        let mut m = mass();
        let hang = jb().hanging;
        assert_eq!(m.mass_rect(), Some(hang));
        assert_eq!(m.swept_mass_rect(), None);
        m.break_by_blast(1, None);
        for _ in 0..24 {
            m.step(&[]);
        }
        let mut bottoms = vec![m.mass_rect().unwrap().bottom()];
        for _ in 0..7 {
            m.step(&[]);
            bottoms.push(m.mass_rect().unwrap().bottom());
        }
        assert!(
            bottoms.windows(2).all(|w| w[1] > w[0]),
            "monotonic: {bottoms:?}"
        );
        let steps: Vec<i32> = bottoms.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(
            steps.windows(2).all(|w| w[1] >= w[0]),
            "it accelerates: {steps:?}"
        );
        // The 8th falling step lands it: the mass is gone, the remnant stands.
        m.step(&[]);
        assert_eq!(m.mass_rect(), None);
        assert_eq!(m.remnant(), Some(jb().remnant));
        // After the 7th falling step the mass bottom is 96 * 49 / 64 = 73 px
        // below its hanging bottom, and its last swept path starts at the
        // 6th step's position (96 * 36 / 64 = 54 px).
        assert_eq!(bottoms[7], hang.bottom() + 73);
        assert_eq!(
            bottoms,
            [0, 1, 4, 9, 16, 25, 36, 49]
                .map(|k| hang.bottom() + 96 * k / 64)
                .to_vec()
        );
    }

    #[test]
    fn the_swept_path_has_no_gap_even_for_a_very_tall_drop() {
        let tall = FallingMassConfig::new(
            IRect::new(0, -9000, 64, 64),
            IRect::new(0, FLOOR - 48, 64, 48),
            IRect::new(0, FLOOR - 16, 64, 16),
        );
        let mut m = FallingMass::new(tall).unwrap();
        m.break_by_blast(1, None);
        for _ in 0..24 {
            m.step(&[]);
        }
        let mut prev_bottom: Option<i32> = None;
        let mut max_advance = 0;
        for _ in 0..7 {
            m.step(&[]);
            let swept = m.swept_mass_rect().unwrap();
            // Consecutive paths touch end to end: the next one starts where
            // the last one ended, minus the mass's own height (no gap a thin
            // body could hide in).
            if let Some(pb) = prev_bottom {
                assert!(swept.y <= pb, "gap before {swept:?}");
                max_advance = max_advance.max(swept.bottom() - pb);
            }
            prev_bottom = Some(swept.bottom());
        }
        assert!(
            max_advance > 300,
            "a single tick moves far more than any body is tall: {max_advance}"
        );
        let last = {
            m.step(&[]);
            m.mass_rect()
        };
        assert_eq!(last, None);
    }

    #[test]
    fn the_landing_hits_only_the_overlapped_bodies_once_on_the_landing_tick() {
        let mut m = mass();
        m.offer(jab(1, 0));
        m.offer(jab(2, 1));
        let bodies = [
            hero_at(0, 700),     // clear on the left
            hero_at(1, 800),     // under the stack
            goblin_at(10, 860),  // under the stack
            goblin_at(11, 1000), // clear on the right
        ];
        // No hit before the landing tick, whatever overlaps.
        for tick in 0..31 {
            assert!(m.step(&bodies).is_none() || tick == 23, "tick {tick}");
        }
        let Some(MassEdge::Landed(l)) = m.step(&bodies) else {
            panic!("the landing is the 32nd step");
        };
        let victims: Vec<Victim> = l.hits.iter().map(|h| h.victim).collect();
        assert_eq!(victims, [Victim::hero(1), Victim::enemy(10)]);
        assert_eq!(l.initiator, Some(1));
        assert!(l.hits.iter().all(|h| h.initiator == Some(1)));
        let hero = l.hits[0];
        assert_eq!((hero.damage, hero.evadable()), (1, false));
        let goblin = l.hits[1];
        assert_eq!(goblin.damage, 255, "lethal to an ordinary body");
        // After landing there is no second evaluation.
        assert!(m.step(&bodies).is_none());
    }

    #[test]
    fn a_body_that_crosses_the_footprint_in_one_tick_is_still_hit_and_a_body_that_does_not_is_not()
    {
        let footprint = jb().footprint;
        // Hero speed is a few px per tick; a launched or blasted body is far
        // faster. Sweep the body across the whole footprint and well beyond.
        for speed in [1, 6, 16, 128, 500, 4000, 100_000] {
            // Start left of the footprint, end right of it.
            let start = footprint.x - 40 - speed / 2;
            let prev = IRect::new(start, FLOOR - 56, 28, 56);
            let cur = IRect::new(footprint.right() + 40 + speed / 2, FLOOR - 56, 28, 56);
            let body = MassBody {
                victim: Victim::hero(0),
                prev,
                cur,
            };
            // The end positions alone miss it.
            assert!(!prev.overlaps(footprint) && !cur.overlaps(footprint));
            let mut m = mass();
            m.break_by_blast(1, None);
            let (_, l) = run_to_landing(&mut m, &[body]);
            assert_eq!(l.hits.len(), 1, "speed {speed} tunnelled");
        }
        // A body moving entirely outside the footprint (also vertically: well
        // above it) is not hit.
        let above = MassBody {
            victim: Victim::hero(0),
            prev: IRect::new(760, FLOOR - 200, 28, 56),
            cur: IRect::new(900, FLOOR - 200, 28, 56),
        };
        let beside = MassBody {
            victim: Victim::hero(1),
            prev: IRect::new(600, FLOOR - 56, 28, 56),
            cur: IRect::new(720, FLOOR - 56, 28, 56),
        };
        let mut m = mass();
        m.break_by_blast(1, None);
        let (_, l) = run_to_landing(&mut m, &[above, beside]);
        assert!(l.hits.is_empty());
    }

    #[test]
    fn a_hero_that_leaves_the_footprint_on_the_landing_tick_is_still_hit_and_a_teleport_is_not() {
        let footprint = jb().footprint;
        // Inside last tick, outside now (one-tick widening, BOILC-AD7).
        let leaving = MassBody {
            victim: Victim::hero(0),
            prev: IRect::new(800, FLOOR - 56, 28, 56),
            cur: IRect::new(footprint.right() + 10, FLOOR - 56, 28, 56),
        };
        let mut m = mass();
        m.break_by_blast(1, None);
        let (_, l) = run_to_landing(&mut m, &[leaving]);
        assert_eq!(l.hits.len(), 1);
        // A teleport across the footprint passes prev == cur: no hit.
        let warped = MassBody::still(
            Victim::hero(0),
            IRect::new(footprint.right() + 10, FLOOR - 56, 28, 56),
        );
        let mut m = mass();
        m.break_by_blast(1, None);
        let (_, l) = run_to_landing(&mut m, &[warped]);
        assert!(l.hits.is_empty());
    }

    #[test]
    fn the_swept_rect_is_the_union_and_is_symmetric() {
        let a = IRect::new(0, 10, 4, 4);
        let b = IRect::new(20, 0, 6, 6);
        assert_eq!(swept_rect(a, b), IRect::new(0, 0, 26, 14));
        assert_eq!(swept_rect(a, b), swept_rect(b, a));
        assert_eq!(swept_rect(a, a), a);
    }

    #[test]
    fn heroes_are_hurt_by_the_damage_policy_and_a_spared_hero_is_still_lifted() {
        let bodies = [hero_at(0, 800), hero_at(1, 860)];
        for (policy, hurts) in [
            (HeroDamagePolicy::AllHeroes, true),
            (HeroDamagePolicy::NoHeroes, false),
        ] {
            let mut cfg = jb();
            cfg.hero_damage = policy;
            let mut m = FallingMass::new(cfg).unwrap();
            m.break_by_blast(1, Some(0));
            let (_, l) = run_to_landing(&mut m, &bodies);
            assert_eq!(l.hits.len(), if hurts { 2 } else { 0 }, "{policy:?}");
            // The owner of the blast is hurt in full, not spared: the initiator
            // only decides credit.
            if hurts {
                assert!(l.hits.iter().any(|h| h.victim == Victim::hero(0)));
            }
            assert_eq!(l.lifts.len(), 2, "lifted whatever the policy");
        }
        // An enemy-sparing mass leaves enemies alone.
        let mut cfg = jb();
        cfg.hurts_enemies = false;
        let mut m = FallingMass::new(cfg).unwrap();
        m.break_by_blast(1, None);
        let (_, l) = run_to_landing(&mut m, &[goblin_at(3, 800), hero_at(0, 800)]);
        assert_eq!(
            l.hits.iter().map(|h| h.victim).collect::<Vec<_>>(),
            [Victim::hero(0)]
        );
        // A tuned damage and knock reach the hit.
        let mut cfg = jb();
        cfg.damage = 2;
        cfg.knock_x_px_s = 200;
        cfg.knock_up_px_s = 90;
        let mut m = FallingMass::new(cfg).unwrap();
        m.break_by_blast(1, None);
        let (_, l) = run_to_landing(&mut m, &[hero_at(0, 900 - 80)]);
        assert_eq!(
            (
                l.hits[0].damage,
                l.hits[0].knock_x_px_s,
                l.hits[0].knock_up_px_s
            ),
            (2, 200, 90)
        );
    }

    #[test]
    fn knock_goes_away_from_the_footprint_centre_and_a_body_on_it_goes_to_plus_x() {
        // The footprint centre is x = 816.
        let mut m = mass();
        m.break_by_blast(1, None);
        let bodies = [hero_at(0, 790), hero_at(1, 840), hero_at(2, 816)];
        let (_, l) = run_to_landing(&mut m, &bodies);
        let knocks: Vec<i16> = l.hits.iter().map(|h| h.knock_x_px_s).collect();
        assert_eq!(knocks, [-120, 120, 120]);
        assert!(l.hits.iter().all(|h| h.knock_up_px_s == 140));
    }

    #[test]
    fn one_outcome_per_victim_even_when_a_body_is_listed_twice() {
        let mut m = mass();
        m.break_by_blast(1, None);
        let b = hero_at(0, 800);
        let (_, l) = run_to_landing(&mut m, &[b, b, goblin_at(4, 820), goblin_at(4, 820)]);
        assert_eq!(l.hits.len(), 2);
        assert_eq!(l.lifts.len(), 1, "one lift per hero too");
    }

    #[test]
    fn a_hero_where_the_remnant_appears_is_lifted_straight_up_onto_its_16_px_top() {
        let rem = jb().remnant;
        assert_eq!(rem.h, STEP_OVER_PX);
        // Standing on the floor: feet at 304, remnant top at 288: up 16.
        let standing = hero_at(0, 800);
        // Feet 6 px above the floor, still overlapping: up 6, feet on the top.
        let hopping = MassBody::still(Victim::hero(1), IRect::new(846, FLOOR - 6 - 56, 28, 56));
        // Feet exactly on the remnant top: not overlapping, not lifted.
        let on_top = MassBody::still(Victim::hero(2), IRect::new(786, rem.y - 56, 28, 56));
        // Airborne above it: not lifted.
        let above = MassBody::still(Victim::hero(3), IRect::new(786, FLOOR - 40 - 56, 28, 56));
        // Beside the remnant: not lifted.
        let beside = hero_at(4, 700);
        let lifts = remnant_lifts(rem, &[standing, hopping, on_top, above, beside]);
        assert_eq!(
            lifts,
            [
                RemnantLift {
                    victim: Victim::hero(0),
                    up_px: 16
                },
                RemnantLift {
                    victim: Victim::hero(1),
                    up_px: 10
                },
            ]
        );
        // After the lift the feet rest on the remnant top and the body no
        // longer overlaps it: nothing is snapped sideways.
        for l in &lifts {
            let b = [standing, hopping][l.victim.id as usize];
            let lifted = IRect::new(b.cur.x, b.cur.y - l.up_px, b.cur.w, b.cur.h);
            assert_eq!(lifted.bottom(), rem.y);
            assert!(!lifted.overlaps(rem));
            assert_eq!(lifted.x, b.cur.x);
        }
    }

    #[test]
    fn the_landing_lifts_a_hero_on_the_landing_tick_and_not_an_enemy() {
        let mut m = mass();
        m.break_by_blast(1, Some(0));
        let (tick, l) = run_to_landing(&mut m, &[hero_at(0, 800), goblin_at(5, 840)]);
        assert_eq!(tick, 31);
        assert_eq!(
            l.lifts,
            [RemnantLift {
                victim: Victim::hero(0),
                up_px: 16
            }]
        );
        assert_eq!(l.remnant, jb().remnant);
        // The remnant is a solid from the landing step on.
        assert_eq!(m.remnant(), Some(jb().remnant));
    }

    #[test]
    fn a_hero_hurt_by_the_landing_is_also_lifted_so_nobody_ends_inside_the_solid() {
        let mut m = mass();
        m.break_by_blast(1, None);
        let (_, l) = run_to_landing(&mut m, &[hero_at(0, 800)]);
        assert_eq!(l.hits.len(), 1);
        assert_eq!(l.lifts.len(), 1);
        assert_eq!(l.lifts[0].victim, l.hits[0].victim);
    }

    #[test]
    fn slot_enemies_ignore_a_remnant_of_16_px_or_less_and_not_a_taller_one() {
        let rem = |h| IRect::new(752, FLOOR - h, 128, h);
        assert!(slot_enemy_ignores_remnant(rem(1)));
        assert!(slot_enemy_ignores_remnant(rem(15)));
        assert!(slot_enemy_ignores_remnant(rem(16)));
        assert!(!slot_enemy_ignores_remnant(rem(17)));
        assert!(!slot_enemy_ignores_remnant(rem(32)));
        // The Boiler's low rubble (32 px) is a wall to them; the Hoarder's
        // J-B remnant (16 px) is not.
        assert!(slot_enemy_ignores_remnant(jb().remnant));
        // The step the heroes are lifted by is the same number.
        assert_eq!(STEP_OVER_PX, 16);
    }

    #[test]
    fn a_taller_remnant_lifts_a_hero_by_its_full_height() {
        let mut cfg = jb();
        cfg.remnant = IRect::new(752, FLOOR - 32, 128, 32);
        let mut m = FallingMass::new(cfg).unwrap();
        m.break_by_blast(1, None);
        let (_, l) = run_to_landing(&mut m, &[hero_at(0, 800)]);
        assert_eq!(l.lifts[0].up_px, 32);
    }

    #[test]
    fn the_hash_follows_the_progress_the_tokens_and_the_evaluated_victims() {
        let mut a = mass();
        let mut b = mass();
        a.offer(jab(1, 0));
        b.offer(jab(2, 0));
        assert_ne!(hash(&a), hash(&b));
        let mut c = mass();
        c.offer(jab(1, 0));
        assert_eq!(hash(&a), hash(&c));
        let mut d = mass();
        let mut e = mass();
        d.break_by_blast(1, None);
        e.break_by_blast(1, None);
        run_to_landing(&mut d, &[hero_at(0, 800)]);
        run_to_landing(&mut e, &[]);
        assert_ne!(hash(&d), hash(&e), "who was hit is state");
    }

    #[test]
    fn reset_restores_the_initial_state_exactly_and_it_can_fall_again() {
        let fresh = hash(&mass());
        let mut m = mass();
        m.offer(jab(1, 0));
        m.break_by_blast(2, Some(1));
        run_to_landing(&mut m, &[hero_at(0, 800)]);
        assert_ne!(hash(&m), fresh);
        m.reset();
        assert_eq!(hash(&m), fresh);
        assert_eq!(m, mass());
        assert_eq!(m.remnant(), None);
        assert_eq!(m.telegraph_rect(), None);
        // A second attempt is identical to the first.
        let mut a = mass();
        a.break_by_blast(2, Some(1));
        let (t1, l1) = run_to_landing(&mut a, &[hero_at(0, 800)]);
        a.reset();
        a.break_by_blast(2, Some(1));
        let (t2, l2) = run_to_landing(&mut a, &[hero_at(0, 800)]);
        assert_eq!((t1, l1), (t2, l2));
    }

    #[test]
    fn the_telegraph_shadow_shows_from_the_release_to_the_landing_only() {
        let mut m = mass();
        assert_eq!(m.telegraph_rect(), None);
        m.offer(jab(1, 0));
        assert_eq!(m.telegraph_rect(), None, "weakened shows no shadow");
        m.offer(jab(2, 0));
        assert_eq!(m.telegraph_rect(), Some(jb().footprint));
        for _ in 0..31 {
            m.step(&[]);
            assert_eq!(m.telegraph_rect(), Some(jb().footprint));
        }
        m.step(&[]);
        assert_eq!(m.state(), MassState::Landed);
        assert_eq!(m.telegraph_rect(), None);
    }

    #[test]
    fn a_custom_warning_and_fall_shift_the_landing_tick() {
        let cfg = FallingMassConfig {
            warning_ticks: 30,
            fall_ticks: 12,
            ..jb()
        };
        let mut m = FallingMass::new(cfg).unwrap();
        m.break_by_blast(1, None);
        let (tick, _) = run_to_landing(&mut m, &[]);
        assert_eq!(tick, 30 + 12 - 1);
    }
}
