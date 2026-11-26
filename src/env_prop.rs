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
        /// BOIL-D (PR #200 review NB-4): the chain depth of the blast (0 for the
        /// dynamite's own blast and any blast no chain reached). A prop this
        /// blast sets off carries it, so a link out of it continues the count.
        depth: u8,
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

    /// Chain depth: 0 for a stimulus that is not part of a chain (a blast
    /// carries the depth of the charge chain that reached it).
    pub const fn depth(&self) -> u8 {
        match self {
            Self::PropContact { depth, .. } | Self::Blast { depth, .. } => *depth,
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
/// The fall every falling mass is built with (the sidecar authors the warning
/// only; DR-B's rule 5 reads it).
pub const DEFAULT_FALL_TICKS: u16 = 8;

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
    /// Weight floor of one melee hit (NB-9): a melee move counts for its own
    /// weight, at least this. Default 1 (every move counts, the Boiler rule);
    /// 0 turns melee **off** (`Ignored::NotAccepted`), for a prop that takes no
    /// hero melee (the Hoarder's J-A bank, HRD-Q22, or a melee-proof stop frame).
    pub melee_weight: u8,
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
            melee_weight: 1,
            launch_min_impulse_px_s: 160,
            launch_weight: 1,
            blast_weight: 2,
            contact_weight: 0,
            projectile_weight: 1,
            shove_weight: 0,
            shove_min_impulse_px_s: 160,
            warning_ticks: 24,
            fall_ticks: DEFAULT_FALL_TICKS,
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
        // NB-6: the mass hangs over its footprint, not beside it.
        if self.hanging.x < self.footprint.x || self.hanging.right() > self.footprint.right() {
            return Err("the mass must hang inside the footprint's x range".into());
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
    /// Bodies of **non-heroes** that overlap a remnant they cannot step over
    /// ([`slot_enemy_ignores_remnant`] is false), in `bodies` order (PR #200
    /// review NB-3). An enemy that survives the landing (a slot boss maps the
    /// 255 request through its own profile) would stand inside a solid, so the
    /// wire refuses a room that holds such a kind (see `combat_scene::env`).
    pub embedded: Vec<Victim>,
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
    /// The combat tick of the landing (F01-SIM-TICKS), for presentation.
    /// Not mixed into the hash: a copy of the scene tick taken at the hashed
    /// `Falling -> Landed` edge, so it adds no state of its own.
    landed_tick: Option<u64>,
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
            landed_tick: None,
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

    /// The combat tick on which the mass landed, once it landed through
    /// [`FallingMass::step_at`]; `None` before the landing and after a reset.
    /// Presentation keys the landing dust off it; it is not hashed.
    pub fn landed_tick(&self) -> Option<u64> {
        self.landed_tick
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

    /// BOILC-AD9 (PR #200 review NB-5): who is credited with the fall. The
    /// **releasing** stimulus' initiator, except when it has none and is not an
    /// enemy shove: then the last initiator that weakened the support keeps the
    /// credit (Carl weakens the brace, an ownerless drum blast releases it: the
    /// work order's "damage source must retain the initiating hero even when
    /// the final cause is a secondary explosion"). A shove is exempt: the
    /// Hoarder's bait rule credits a shove with `None` when no player baited it,
    /// whatever hit the post before. The weakening initiator is kept in
    /// `initiator`, which is hashed already.
    fn add(&mut self, weight: u8, stimulus: &Stimulus) -> Offer {
        self.progress = self.progress.saturating_add(weight);
        let by = stimulus.initiator();
        if self.progress >= self.cfg.release_weight {
            self.state = MassState::Released;
            self.ticks_left = self.cfg.warning_ticks;
            self.initiator = if stimulus.kind() == StimulusKind::Shove {
                by
            } else {
                by.or(self.initiator)
            };
            self.depth = stimulus.depth();
            Offer::Triggered {
                initiator: self.initiator,
                depth: self.depth,
            }
        } else {
            self.state = MassState::Weakened;
            if by.is_some() {
                self.initiator = by;
            }
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
                if cfg.melee_weight == 0 {
                    return Offer::Ignored(Ignored::NotAccepted);
                }
                if self.melee_tokens.contains(&token) {
                    return Offer::Ignored(Ignored::Duplicate);
                }
                self.melee_tokens.push(token);
                self.add(weight.max(cfg.melee_weight), &stimulus)
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
    /// carries (its id, its owner and its chain depth), never a state of this
    /// prop, so the hook stays independent of the machine behind it. A prop this
    /// blast sets off carries `depth`; its link out is `depth + 1` (NB-4).
    pub fn break_by_blast(&mut self, blast_id: u32, owner: Option<u32>, depth: u8) -> Offer {
        self.offer(Stimulus::Blast {
            blast_id,
            owner,
            depth,
        })
    }

    /// [`FallingMass::step`] on the combat tick `tick`: the same edges, and the
    /// landing edge also records `tick` as [`FallingMass::landed_tick`].
    pub fn step_at(&mut self, tick: u64, bodies: &[MassBody]) -> Option<MassEdge> {
        let edge = self.step(bodies);
        if matches!(edge, Some(MassEdge::Landed(_))) {
            self.landed_tick = Some(tick);
        }
        edge
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
        let embedded = if slot_enemy_ignores_remnant(cfg.remnant) {
            Vec::new()
        } else {
            bodies
                .iter()
                .filter(|b| b.victim.team == Team::Enemy && b.cur.overlaps(cfg.remnant))
                .map(|b| b.victim)
                .collect()
        };
        Landing {
            embedded,
            hits,
            lifts: remnant_lifts(cfg.remnant, bodies),
            remnant: cfg.remnant,
            initiator: self.initiator,
        }
    }

    /// Fold the full state into a state hash (all of it except
    /// `landed_tick`, a copy of the scene tick at a hashed edge).
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

// ---------------------------------------------------------------------------
// RING-B core: the shutter's knobs. RING-C core: the shutter machine.
// ---------------------------------------------------------------------------

/// The closing (and opening) of a shutter takes this many ticks at least:
/// #189 "moving closure 24-36 fixed ticks" (KR2, KR19).
pub const SHUTTER_MIN_CLOSING_TICKS: u16 = 24;
/// ... and at most this many (#189, KR19).
pub const SHUTTER_MAX_CLOSING_TICKS: u16 = 36;
/// A shutter descends at most this many px per tick (KR2: slow, readable).
pub const SHUTTER_MAX_SPEED_PX_PER_TICK: u8 = 2;
/// The warning lasts 36..=48 ticks (#189 "36-48 fixed ticks", KR2 adopts 40).
pub const SHUTTER_MIN_WARNING_TICKS: u16 = 36;
pub const SHUTTER_MAX_WARNING_TICKS: u16 = 48;
/// The arming position is at most this far from the shutter (KR18: within 256 px).
pub const SHUTTER_MAX_ARM_DISTANCE_PX: i32 = 256;
/// A moving shutter stays closed at most this many ticks (#189 "dwell 90-150",
/// KR19).
pub const SHUTTER_MAX_CLOSED_TICKS: u16 = 150;
/// The stuck limit an AI partner pressing on a closed shutter must stay under
/// (KR19: 300 ticks).
pub const SHUTTER_STUCK_LIMIT_TICKS: u32 = 300;
/// A pin is written to a goblin's `stagger_left`, a `u8` (KR4).
pub const SHUTTER_MAX_PIN_TICKS: u16 = 255;

/// KR19: `2 x closing ticks + max_closed_ticks` is under the stuck limit. With
/// the closing and dwell bounds above it holds by construction (at most
/// 2 x 36 + 150 = 222); it stays as its own check so that moving a bound cannot
/// silently break the AI partner's guarantee.
pub const fn shutter_window_ok(closing_ticks: u16, max_closed_ticks: u16) -> bool {
    2 * closing_ticks as u32 + (max_closed_ticks as u32) < SHUTTER_STUCK_LIMIT_TICKS
}

/// Whether the shutter starts open (a moving one) or closed (a static gate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutterInitial {
    Open,
    Closed,
}

impl ShutterInitial {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "open" => Self::Open,
            "closed" => Self::Closed,
            _ => return None,
        })
    }
}

/// Tunables of one shutter (RING-B core appends the type, RING-C core runs it).
/// The geometry is the prop's `target`: the **closed** rectangle, standing on
/// the floor under a platform slab (KR3). Defaults and where each number comes
/// from:
///
/// * `warning_ticks` 40: KR2 (inside #189's 36-48);
/// * `speed_px_per_tick` 2: KR2 (at most 2); a 68 px shutter then closes in 34
///   ticks, inside #189's 24-36;
/// * `slice_px` 4: KR6 (the 68 px shutter is 17 slices);
/// * `cycles` 1: KR5 (one close per attempt, then it reopens and stays open);
///   0 is a **static** gate that stays as authored;
/// * `pin_ticks` 36: KR4;
/// * `max_closed_ticks` 150: #189's longest dwell, KR19;
/// * `arm_trail_px` 320: KR18.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShutterConfig {
    /// `Closed` is a static gate (`cycles` 0), `Open` a moving shutter.
    pub initial: ShutterInitial,
    /// A hero that has crossed this x arms the warning (KR18). Required for a
    /// moving shutter, within [`SHUTTER_MAX_ARM_DISTANCE_PX`] of it.
    pub arm_x: Option<i32>,
    /// The trailing living hero must be within this many px of the shutter for
    /// it to warn (KR18).
    pub arm_trail_px: i32,
    pub warning_ticks: u16,
    pub speed_px_per_tick: u8,
    /// Height of one collision slice; it divides the shutter's height.
    pub slice_px: u8,
    /// Closures per attempt; 0 = static.
    pub cycles: u8,
    /// Ticks a pinned goblin is staggered (KR4), `1..=`[`SHUTTER_MAX_PIN_TICKS`].
    pub pin_ticks: u16,
    /// A moving shutter's closed dwell, `1..=`[`SHUTTER_MAX_CLOSED_TICKS`].
    pub max_closed_ticks: u16,
}

impl ShutterConfig {
    /// The adopted defaults around a given start (see the type docs). The
    /// `initial` and `cycles` pair must agree: [`ShutterConfig::validate`].
    pub const fn defaults(initial: ShutterInitial) -> Self {
        Self {
            initial,
            arm_x: None,
            arm_trail_px: 320,
            warning_ticks: 40,
            speed_px_per_tick: 2,
            slice_px: 4,
            cycles: match initial {
                ShutterInitial::Open => 1,
                ShutterInitial::Closed => 0,
            },
            pin_ticks: 36,
            max_closed_ticks: 150,
        }
    }

    pub const fn is_static(&self) -> bool {
        self.cycles == 0
    }

    /// Ticks the lower edge needs to travel `height_px`: `ceil(height / speed)`
    /// (KR19). Opening takes as long as closing.
    pub const fn closing_ticks(&self, height_px: i32) -> u16 {
        let speed = self.speed_px_per_tick as i32;
        if speed == 0 || height_px <= 0 {
            return 0;
        }
        let ticks = (height_px + speed - 1) / speed;
        if ticks > u16::MAX as i32 {
            u16::MAX
        } else {
            ticks as u16
        }
    }

    /// The longest an **unheld** cycle blocks the lane: closing, closed and
    /// opening (KR19, about 218 ticks at the authored numbers).
    pub const fn blocked_window_ticks(&self, height_px: i32) -> u32 {
        2 * self.closing_ticks(height_px) as u32 + self.max_closed_ticks as u32
    }

    /// Every bound of the knobs for a shutter whose closed rectangle is
    /// `target`. One source of truth: `encounter.rs` calls it for the sidecar
    /// and [`Shutter::new`] for the machine.
    pub fn validate(&self, target: IRect) -> Result<(), String> {
        if target.w <= 0 || target.h <= 0 {
            return Err("the shutter needs an area".into());
        }
        if self.speed_px_per_tick == 0 || self.speed_px_per_tick > SHUTTER_MAX_SPEED_PX_PER_TICK {
            return Err(format!(
                "speed_px_per_tick must be 1..={SHUTTER_MAX_SPEED_PX_PER_TICK}, is {}",
                self.speed_px_per_tick
            ));
        }
        let closing = self.closing_ticks(target.h);
        if !(SHUTTER_MIN_CLOSING_TICKS..=SHUTTER_MAX_CLOSING_TICKS).contains(&closing) {
            return Err(format!(
                "a {} px shutter at {} px/tick closes in {closing} ticks, must be \
                 {SHUTTER_MIN_CLOSING_TICKS}..={SHUTTER_MAX_CLOSING_TICKS}",
                target.h, self.speed_px_per_tick
            ));
        }
        if !(SHUTTER_MIN_WARNING_TICKS..=SHUTTER_MAX_WARNING_TICKS).contains(&self.warning_ticks) {
            return Err(format!(
                "warning_ticks must be {SHUTTER_MIN_WARNING_TICKS}..={SHUTTER_MAX_WARNING_TICKS}, is {}",
                self.warning_ticks
            ));
        }
        if self.slice_px == 0 || target.h % i32::from(self.slice_px) != 0 {
            return Err(format!(
                "slice_px {} must be at least 1 and divide the {} px height",
                self.slice_px, target.h
            ));
        }
        if self.pin_ticks == 0 || self.pin_ticks > SHUTTER_MAX_PIN_TICKS {
            return Err(format!(
                "pin_ticks must be 1..={SHUTTER_MAX_PIN_TICKS}, is {}",
                self.pin_ticks
            ));
        }
        if self.arm_trail_px <= 0 {
            return Err("arm_trail_px must be positive".into());
        }
        if let Some(x) = self.arm_x {
            let dist = (target.x - x).max(x - target.right()).max(0);
            if dist > SHUTTER_MAX_ARM_DISTANCE_PX {
                return Err(format!(
                    "arm_x {x} is {dist} px from the shutter, at most {SHUTTER_MAX_ARM_DISTANCE_PX}"
                ));
            }
        }
        match (self.is_static(), self.initial) {
            (true, ShutterInitial::Open) => {
                return Err("a static shutter (cycles 0) starts closed".into());
            }
            (false, ShutterInitial::Closed) => {
                return Err("a moving shutter (cycles above 0) starts open".into());
            }
            _ => {}
        }
        if !self.is_static() {
            if self.arm_x.is_none() {
                return Err("a moving shutter needs arm_x".into());
            }
            if self.max_closed_ticks == 0 || self.max_closed_ticks > SHUTTER_MAX_CLOSED_TICKS {
                return Err(format!(
                    "max_closed_ticks must be 1..={SHUTTER_MAX_CLOSED_TICKS}, is {}",
                    self.max_closed_ticks
                ));
            }
            // KR19: an AI pressing on the shutter is blocked for the closing,
            // closed and opening windows, under the stuck limit.
            if !shutter_window_ok(closing, self.max_closed_ticks) {
                return Err(format!(
                    "2 x closing ticks + max_closed_ticks is {}, must be under \
                     {SHUTTER_STUCK_LIMIT_TICKS} (KR19)",
                    self.blocked_window_ticks(target.h)
                ));
            }
        }
        Ok(())
    }

    /// The deterministic arm rule (KR18): a living hero has crossed `arm_x`
    /// **and** the trailing living hero is within `arm_trail_px` of the shutter
    /// (distance to its nearest edge). A position bound, never a camera read.
    /// `hero_xs` are the centres of the **living** heroes. A static shutter
    /// never arms.
    pub fn arm_ready(&self, target: IRect, hero_xs: &[i32]) -> bool {
        let Some(arm_x) = self.arm_x else {
            return false;
        };
        if self.is_static() || hero_xs.is_empty() {
            return false;
        }
        // Heroes advance toward the shutter: crossed means on its side of arm_x.
        let from_left = arm_x <= target.x;
        let crossed = hero_xs
            .iter()
            .any(|&x| if from_left { x >= arm_x } else { x <= arm_x });
        let trailing = hero_xs
            .iter()
            .map(|&x| (target.x - x).max(x - target.right()).max(0))
            .max()
            .unwrap_or(0);
        crossed && trailing <= self.arm_trail_px
    }
}

/// The phases of a shutter. A static gate stays `Closed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutterPhase {
    Open,
    /// Armed: the tell (stone scraping, the lower-edge cue). Nothing moves.
    Warning,
    /// The lower edge descends; slices turn intact behind it.
    Closing,
    Closed,
    /// The lower edge rises; slices turn non-intact from the bottom.
    Opening,
}

impl ShutterPhase {
    const fn code(self) -> u64 {
        self as u64
    }
}

/// What a body is for the shutter. The core cannot see goblin modes, so the
/// wire maps them (KR4): `Neutral` and `Attack` are `Free`; `Launched`,
/// `Downed`, `Airborne` and `Hurt` (a staggered or pinned goblin: it holds the
/// edge, but a pin never re-pins it) are `Busy`; `Down` and `Corpse` are
/// `Gone`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutterBodyState {
    /// Alive and standing: holds the shutter, and a goblin can be pinned.
    Free,
    /// Alive but mid-launch, down-in-progress or airborne: holds the shutter,
    /// is never pinned here (a pin never overwrites those modes, KR4).
    Busy,
    /// A downed or dead body: no collision, never holds (KR4).
    Gone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShutterBody {
    /// Where the body was and is (layout px); `prev == cur` after a teleport.
    pub body: MassBody,
    pub state: ShutterBodyState,
}

impl ShutterBody {
    pub const fn new(body: MassBody, state: ShutterBodyState) -> Self {
        Self { body, state }
    }
}

/// A state change of one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutterEdge {
    /// The warning ended; the edge moves from the next tick.
    ClosingStarted,
    /// The edge reached the floor: every slice is intact.
    Closed,
    /// The closed dwell ended; the edge rises from the next tick.
    OpeningStarted,
    /// The edge is fully up: no slice is intact; the closure is over.
    Opened,
}

/// A pin to apply to a goblin: set it staggered for `ticks` with no damage
/// (KR4). The wire writes the public goblin fields; it never overwrites a
/// launched, downed or airborne goblin (those bodies are `Busy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShutterPin {
    pub victim: Victim,
    pub ticks: u16,
    /// The launch's instigator; `None` for a goblin that walked in (KR4).
    pub source: Option<u32>,
}

/// What one tick of a shutter did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShutterTick {
    pub edge: Option<ShutterEdge>,
    /// The closing edge would have turned a slice intact over a living body, so
    /// it did not move this tick.
    pub held: bool,
    /// New pins of this tick (one per enemy per closure).
    pub pins: Vec<ShutterPin>,
}

/// The outcome of a launched goblin hitting the shutter's face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceHit {
    /// The token of this enemy for this closure is spent: no walk-in pin will
    /// follow (the existing slam is the effect, KR4: no new damage).
    Spent,
    /// This enemy already used its token in this closure.
    Duplicate,
    /// No slice is intact, so there is no face to hit.
    NotSolid,
    /// Heroes carry no pin token.
    NotAnEnemy,
}

/// The shutter: `Open -> Warning -> Closing -> Closed -> Opening -> Open`
/// (KR2, KR4, KR5). A kinematic stack of collision **slices** descending at
/// most [`SHUTTER_MAX_SPEED_PX_PER_TICK`] px per tick.
///
/// Tick contract (the future wire step, KR18): when
/// [`ShutterConfig::arm_ready`] holds, call [`Shutter::arm`] on an open
/// shutter; then [`Shutter::step`] once per tick with the bodies. A closure
/// armed before step `R` warns during steps `R..R + warning - 1`, is `Closing`
/// from the step that ended the warning (the edge first moves one step later)
/// for `ceil(height / speed)` unheld steps, is `Closed` for `max_closed_ticks`
/// and `Opening` for as many steps as it closed.
///
/// **Hold rule (KR4).** A slice turns intact once the lower edge has passed
/// its bottom. If any **living** body (hero or enemy, [`ShutterBodyState::Gone`]
/// excluded) overlaps the band the edge would occupy this step (from the
/// current edge down to the slice boundary the next position lands in, using
/// the body's swept volume, [`swept_rect`]), the edge does not move: nobody is
/// squeezed, shoved or hurt, no slice turns intact over a body, and the drawn
/// lower edge never sinks into one, for any legal `slice_px`. The hold has no
/// time bound (a body under the edge holds it above its own head). Opening
/// never holds.
///
/// **Pins (KR4).** A `Free` enemy that holds the edge is pinned once per
/// closure ([`ShutterPin`]); a launched enemy that hits the face spends its
/// token through [`Shutter::face_hit`]. Heroes are never pinned.
///
/// Integer math only; no RNG, no wall-clock time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shutter {
    cfg: ShutterConfig,
    /// The closed rectangle.
    rect: IRect,
    phase: ShutterPhase,
    ticks_left: u16,
    /// Px the lower edge has descended from the top, `0..=rect.h`.
    edge_px: i32,
    /// Closures armed this attempt.
    closures: u8,
    /// Enemy ids whose pin token of this closure is spent.
    spent: Vec<u32>,
}

impl Shutter {
    pub fn new(cfg: ShutterConfig, rect: IRect) -> Result<Self, String> {
        cfg.validate(rect)?;
        let closed = cfg.initial == ShutterInitial::Closed;
        Ok(Self {
            cfg,
            rect,
            phase: if closed {
                ShutterPhase::Closed
            } else {
                ShutterPhase::Open
            },
            ticks_left: 0,
            edge_px: if closed { rect.h } else { 0 },
            closures: 0,
            spent: Vec::new(),
        })
    }

    pub fn config(&self) -> &ShutterConfig {
        &self.cfg
    }
    pub fn phase(&self) -> ShutterPhase {
        self.phase
    }
    /// The closed rectangle (the footprint).
    pub fn footprint(&self) -> IRect {
        self.rect
    }
    /// Px the lower edge has descended from the top of the arch.
    pub fn edge_px(&self) -> i32 {
        self.edge_px
    }
    /// The y of the lower edge (layout px, y down).
    pub fn lower_edge_y(&self) -> i32 {
        self.rect.y + self.edge_px
    }
    /// Whether this enemy's pin token of the current closure is spent (by a
    /// walk-in pin or a face hit). Heroes never carry one.
    pub fn token_spent(&self, victim: Victim) -> bool {
        victim.team == Team::Enemy && self.spent.contains(&victim.id)
    }
    /// Closures armed so far this attempt.
    pub fn closures(&self) -> u8 {
        self.closures
    }
    /// Restart the attempt: the same state as [`Shutter::new`].
    pub fn reset(&mut self) {
        *self = Self::new(self.cfg, self.rect).expect("a built shutter has a valid config");
    }

    pub fn slice_count(&self) -> usize {
        (self.rect.h / i32::from(self.cfg.slice_px)) as usize
    }

    /// Slice `k` counted from the top of the arch.
    pub fn slice_rect(&self, k: usize) -> IRect {
        let s = i32::from(self.cfg.slice_px);
        IRect::new(self.rect.x, self.rect.y + s * k as i32, self.rect.w, s)
    }

    /// How many slices are intact: those whose bottom the lower edge passed.
    pub fn intact_slices(&self) -> usize {
        (self.edge_px / i32::from(self.cfg.slice_px)) as usize
    }

    pub fn slice_intact(&self, k: usize) -> bool {
        k < self.intact_slices()
    }

    /// The intact stack as one rectangle (the slices touch end to end), the
    /// face a launched body hits. `None` when no slice is intact.
    pub fn solid_rect(&self) -> Option<IRect> {
        let n = self.intact_slices();
        (n > 0).then(|| {
            IRect::new(
                self.rect.x,
                self.rect.y,
                self.rect.w,
                i32::from(self.cfg.slice_px) * n as i32,
            )
        })
    }

    /// The footprint shadow the warning draws, while warning or closing.
    pub fn telegraph_rect(&self) -> Option<IRect> {
        matches!(self.phase, ShutterPhase::Warning | ShutterPhase::Closing).then_some(self.rect)
    }

    /// Start a closure from `Open`: the warning begins and the pin tokens of
    /// the previous closure are cleared. `false` (nothing changes) when the
    /// shutter is not open, is static, or has used its `cycles`.
    pub fn arm(&mut self) -> bool {
        if self.phase != ShutterPhase::Open
            || self.cfg.is_static()
            || self.closures >= self.cfg.cycles
        {
            return false;
        }
        self.closures += 1;
        self.phase = ShutterPhase::Warning;
        self.ticks_left = self.cfg.warning_ticks;
        self.spent.clear();
        true
    }

    /// A launched enemy hit the shutter's face: spend its pin token (KR4: the
    /// existing slam is the effect, so a walk-in pin must not follow). The
    /// wire calls it when the slam's down state happens.
    pub fn face_hit(&mut self, victim: Victim) -> FaceHit {
        if victim.team != Team::Enemy {
            return FaceHit::NotAnEnemy;
        }
        if self.solid_rect().is_none() {
            return FaceHit::NotSolid;
        }
        if self.spent.contains(&victim.id) {
            return FaceHit::Duplicate;
        }
        self.spent.push(victim.id);
        FaceHit::Spent
    }

    /// One fixed tick. See the type docs for the order and the hold rule.
    pub fn step(&mut self, bodies: &[ShutterBody]) -> ShutterTick {
        let mut tick = ShutterTick::default();
        if self.cfg.is_static() {
            return tick;
        }
        match self.phase {
            ShutterPhase::Open => {}
            ShutterPhase::Warning => {
                self.ticks_left -= 1;
                if self.ticks_left == 0 {
                    self.phase = ShutterPhase::Closing;
                    tick.edge = Some(ShutterEdge::ClosingStarted);
                }
            }
            ShutterPhase::Closing => self.step_closing(bodies, &mut tick),
            ShutterPhase::Closed => {
                self.ticks_left = self.ticks_left.saturating_sub(1);
                if self.ticks_left == 0 {
                    self.phase = ShutterPhase::Opening;
                    tick.edge = Some(ShutterEdge::OpeningStarted);
                }
            }
            ShutterPhase::Opening => {
                self.edge_px = (self.edge_px - i32::from(self.cfg.speed_px_per_tick)).max(0);
                if self.edge_px == 0 {
                    self.phase = ShutterPhase::Open;
                    tick.edge = Some(ShutterEdge::Opened);
                }
            }
        }
        tick
    }

    fn step_closing(&mut self, bodies: &[ShutterBody], tick: &mut ShutterTick) {
        let height = self.rect.h;
        let next = (self.edge_px + i32::from(self.cfg.speed_px_per_tick)).min(height);
        let slice = i32::from(self.cfg.slice_px);
        // The band the edge would occupy: from where it is now down to the slice
        // boundary the next step lands in. A body in it holds the edge, so the
        // drawn lower edge never sinks into a body and no slice turns intact
        // over one (KR4), whatever `slice_px` is.
        let band_bottom = ((next + slice - 1) / slice * slice).min(height);
        let band = IRect::new(
            self.rect.x,
            self.rect.y + self.edge_px,
            self.rect.w,
            band_bottom - self.edge_px,
        );
        let mut held = false;
        for b in bodies {
            if b.state == ShutterBodyState::Gone {
                continue;
            }
            let swept = swept_rect(b.body.prev, b.body.cur);
            if !swept.overlaps(band) {
                continue;
            }
            held = true;
            let v = b.body.victim;
            if v.team == Team::Enemy
                && b.state == ShutterBodyState::Free
                && !self.spent.contains(&v.id)
            {
                self.spent.push(v.id);
                tick.pins.push(ShutterPin {
                    victim: v,
                    ticks: self.cfg.pin_ticks,
                    source: None,
                });
            }
        }
        if held {
            tick.held = true;
            return;
        }
        self.edge_px = next;
        if next == height {
            self.phase = ShutterPhase::Closed;
            self.ticks_left = self.cfg.max_closed_ticks;
            tick.edge = Some(ShutterEdge::Closed);
        }
    }

    /// Fold the full state into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.phase.code());
        mix(u64::from(self.ticks_left));
        mix(self.edge_px as u64);
        mix(u64::from(self.closures));
        mix(self.spent.len() as u64);
        for id in &self.spent {
            mix(u64::from(*id));
        }
    }
}

// ---------------------------------------------------------------------------
// BOIL-C wire: the J-A slide variant (a horizontal moving mass with a remnant).
// ---------------------------------------------------------------------------

/// A slide is at most this many ticks (1 s), like a fall: longer is a drift.
pub const MAX_SLIDE_TICKS: u16 = 60;

/// The states of a slide, `stable -> disturbed -> slid -> settled` (Hoarder
/// sheet section 5, #178).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlideState {
    /// Standing: a solid with a standable top, untouched.
    Stable,
    /// A stimulus disturbed it; the shudder runs (the solid still stands).
    Disturbed,
    /// The solid is gone and a moving mass translates `dx_per_tick`.
    Slid,
    /// The low remnant stands for the rest of the attempt.
    Settled,
}

impl SlideState {
    const fn code(self) -> u64 {
        self as u64
    }
}

/// Tunables and geometry of one slide (layout px, y down). Defaults follow
/// HRD-Q21..Q23 of the Hoarder sheet (the `HRD-Q*` ids are in
/// `enemy-gameplay-hoarder.md`):
///
/// * `warning_ticks` 24 and `slide_ticks` 16, `dx_per_tick` -3: HRD-Q21
///   (disturbed T..T+23, slid T+24..T+39, settled from T+40);
/// * the stimuli that disturb it are a **launched body**, a **blast** and an
///   enemy **shove**; **hero melee and missiles never do** (HRD-Q22, so cover
///   is never knocked over by accident). The weights are 1: any counted
///   stimulus disturbs it, once;
/// * the 160 px/s thresholds are the valve's (BOILC-AD2), the Hoarder's lunge
///   is 180.
///
/// The push of the moving mass (0 damage, knock 180 / 160, HRD-Q23) is an
/// `env_hazard` persistent volume the owner of the room builds from
/// [`Slide::mass_rect`]; this machine only moves the mass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlideConfig {
    /// The bank while it stands.
    pub stable: IRect,
    /// The solid left after the slide: rests on the bank's floor and covers
    /// the mass' final position.
    pub remnant: IRect,
    /// Signed px per slide tick, never 0.
    pub dx_per_tick: i32,
    pub warning_ticks: u16,
    pub slide_ticks: u16,
    pub launch_min_impulse_px_s: u16,
    pub shove_min_impulse_px_s: u16,
    /// Whether each kind disturbs it. Melee and projectiles are never accepted:
    /// they have no switch (HRD-Q22).
    pub accepts_launched_body: bool,
    pub accepts_blast: bool,
    pub accepts_shove: bool,
}

impl SlideConfig {
    pub const fn new(stable: IRect, remnant: IRect) -> Self {
        Self {
            stable,
            remnant,
            dx_per_tick: -3,
            warning_ticks: 24,
            slide_ticks: 16,
            launch_min_impulse_px_s: 160,
            shove_min_impulse_px_s: 160,
            accepts_launched_body: true,
            accepts_blast: true,
            accepts_shove: true,
        }
    }

    /// The mass after `moved` slide ticks.
    pub const fn mass_after(&self, moved: u16) -> IRect {
        IRect::new(
            self.stable.x + self.dx_per_tick * moved as i32,
            self.stable.y,
            self.stable.w,
            self.stable.h,
        )
    }

    pub fn validate(&self) -> Result<(), String> {
        let area = |r: IRect| r.w > 0 && r.h > 0;
        if !area(self.stable) || !area(self.remnant) {
            return Err("stable and remnant need an area".into());
        }
        if self.dx_per_tick == 0 {
            return Err("dx_per_tick must not be 0".into());
        }
        if self.warning_ticks < MIN_WARNING_TICKS {
            return Err(format!(
                "warning_ticks must be at least {MIN_WARNING_TICKS}, is {}",
                self.warning_ticks
            ));
        }
        if self.slide_ticks == 0 || self.slide_ticks > MAX_SLIDE_TICKS {
            return Err(format!(
                "slide_ticks must be 1..={MAX_SLIDE_TICKS}, is {}",
                self.slide_ticks
            ));
        }
        let end = self.mass_after(self.slide_ticks);
        let r = self.remnant;
        if r.bottom() != self.stable.bottom() || r.h > self.stable.h {
            return Err("the remnant must rest on the bank's floor and be no taller".into());
        }
        if r.x > end.x || r.right() < end.right() {
            return Err("the remnant must cover the mass' final position".into());
        }
        Ok(())
    }
}

/// A state change of one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlideEdge {
    /// The shudder ended; the solid is gone and the mass starts to move from
    /// the next tick. Emitted in the step of the last disturbed tick (T+23).
    SlideStarted,
    /// The slide is over; the remnant stands from this step on (T+40).
    Settled,
}

/// The slide: `Stable -> Disturbed -> Slid -> Settled`, once per attempt.
///
/// Tick contract (the wire): offer the stimuli of the tick, then
/// [`Slide::step`] once. A disturbance on tick `T` leaves the bank a solid
/// during `T..=T + warning - 1`, the mass moves on the steps `T + warning ..=
/// T + warning + slide_ticks - 1` (after each of them [`Slide::mass_rect`] is
/// the bank shifted by `dx_per_tick * k`), and the step of `T + warning +
/// slide_ticks` settles it (HRD-Q21: with the defaults the mass moves on
/// T+24..=T+39 and the remnant stands from T+40).
///
/// Integer math only; no RNG, no wall-clock time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slide {
    cfg: SlideConfig,
    state: SlideState,
    ticks_left: u16,
    moved: u16,
    launches: Vec<u32>,
    blasts: Vec<u32>,
    shoves: Vec<u32>,
    initiator: Option<u32>,
    depth: u8,
}

impl Slide {
    pub fn new(cfg: SlideConfig) -> Result<Self, String> {
        cfg.validate()?;
        Ok(Self {
            cfg,
            state: SlideState::Stable,
            ticks_left: 0,
            moved: 0,
            launches: Vec::new(),
            blasts: Vec::new(),
            shoves: Vec::new(),
            initiator: None,
            depth: 0,
        })
    }

    pub fn config(&self) -> &SlideConfig {
        &self.cfg
    }
    pub fn state(&self) -> SlideState {
        self.state
    }
    /// The player credited with the slide.
    pub fn initiator(&self) -> Option<u32> {
        self.initiator
    }
    pub fn trigger_depth(&self) -> u8 {
        self.depth
    }
    /// Slide ticks done.
    pub fn moved_ticks(&self) -> u16 {
        self.moved
    }
    pub fn reset(&mut self) {
        *self = Self::new(self.cfg).expect("a built slide has a valid config");
    }

    /// The standing bank: a solid while `Stable` or `Disturbed`.
    pub fn solid_rect(&self) -> Option<IRect> {
        matches!(self.state, SlideState::Stable | SlideState::Disturbed).then_some(self.cfg.stable)
    }

    /// The moving mass (a hazard volume, not a solid), while `Slid`. Before the
    /// first slide step it is the bank itself.
    pub fn mass_rect(&self) -> Option<IRect> {
        (self.state == SlideState::Slid).then(|| self.cfg.mass_after(self.moved))
    }

    /// The remnant, once settled.
    pub fn remnant(&self) -> Option<IRect> {
        (self.state == SlideState::Settled).then_some(self.cfg.remnant)
    }

    /// Offer one stimulus. Only a `Stable` bank reacts; every ignored case is
    /// named. Melee and projectiles are `NotAccepted`, always (HRD-Q22).
    pub fn offer(&mut self, stimulus: Stimulus) -> Offer {
        if self.state != SlideState::Stable {
            return Offer::Ignored(Ignored::AlreadyActivated);
        }
        let cfg = self.cfg;
        match stimulus {
            Stimulus::LaunchedBody {
                launch_id,
                impulse_px_s,
                ..
            } => {
                if !cfg.accepts_launched_body {
                    return Offer::Ignored(Ignored::NotAccepted);
                }
                if impulse_px_s < cfg.launch_min_impulse_px_s {
                    return Offer::Ignored(Ignored::TooWeak);
                }
                if self.launches.contains(&launch_id) {
                    return Offer::Ignored(Ignored::Duplicate);
                }
                self.launches.push(launch_id);
            }
            Stimulus::Blast { blast_id, .. } => {
                if !cfg.accepts_blast {
                    return Offer::Ignored(Ignored::NotAccepted);
                }
                if self.blasts.contains(&blast_id) {
                    return Offer::Ignored(Ignored::Duplicate);
                }
                self.blasts.push(blast_id);
            }
            Stimulus::Shove {
                shove_id,
                impulse_px_s,
                ..
            } => {
                if !cfg.accepts_shove {
                    return Offer::Ignored(Ignored::NotAccepted);
                }
                if impulse_px_s < cfg.shove_min_impulse_px_s {
                    return Offer::Ignored(Ignored::ShoveTooWeak);
                }
                if self.shoves.contains(&shove_id) {
                    return Offer::Ignored(Ignored::Duplicate);
                }
                self.shoves.push(shove_id);
            }
            Stimulus::MeleeHit { .. } | Stimulus::Projectile { .. } => {
                return Offer::Ignored(Ignored::NotAccepted);
            }
            Stimulus::PropContact { depth, .. } => {
                if depth >= MAX_CHAIN_DEPTH {
                    return Offer::Ignored(Ignored::ChainTooDeep);
                }
                // A link into a bank is BOIL-D's decision, off by default.
                return Offer::Ignored(Ignored::NotAccepted);
            }
        }
        self.state = SlideState::Disturbed;
        self.ticks_left = cfg.warning_ticks;
        self.initiator = stimulus.initiator();
        self.depth = stimulus.depth();
        Offer::Triggered {
            initiator: self.initiator,
            depth: self.depth,
        }
    }

    /// One fixed tick, after the stimuli.
    pub fn step(&mut self) -> Option<SlideEdge> {
        match self.state {
            SlideState::Disturbed => {
                self.ticks_left -= 1;
                if self.ticks_left == 0 {
                    self.state = SlideState::Slid;
                    self.moved = 0;
                    return Some(SlideEdge::SlideStarted);
                }
                None
            }
            SlideState::Slid => {
                if self.moved < self.cfg.slide_ticks {
                    self.moved += 1;
                    None
                } else {
                    self.state = SlideState::Settled;
                    Some(SlideEdge::Settled)
                }
            }
            SlideState::Stable | SlideState::Settled => None,
        }
    }

    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.state.code());
        mix(u64::from(self.ticks_left));
        mix(u64::from(self.moved));
        mix(self.initiator.map_or(0, |p| u64::from(p) + 1));
        mix(u64::from(self.depth));
        for list in [&self.launches, &self.blasts, &self.shoves] {
            mix(list.len() as u64);
            for id in list {
                mix(u64::from(*id));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CORRAL-D: the breakable partition.
// ---------------------------------------------------------------------------

/// CRL-Q30: three distinct physical hero hits break the partition.
pub const PARTITION_BREAK_HITS: u8 = 3;
/// The most hits a partition may ask (the hit tokens are kept in a list).
pub const PARTITION_MAX_BREAK_HITS: u8 = 8;
/// CD-AD3: the slowest enemy shove that breaks the partition (px/s). The rush
/// is 240 px/s, so a rush contact always breaks it; the 160 px/s floor is the
/// other props' launch and shove threshold (BOILC-AD2).
pub const PARTITION_SHOVE_MIN_IMPULSE_PX_S: u16 = 160;

/// The partition's tunables (the sidecar's `partition` object).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartitionConfig {
    /// Distinct physical hero hits (one per move instance) that break it.
    pub break_hits: u8,
    /// The slowest enemy shove (the llama's rush) that breaks it at once.
    pub shove_min_impulse_px_s: u16,
}

impl PartitionConfig {
    pub const DEFAULT: Self = Self {
        break_hits: PARTITION_BREAK_HITS,
        shove_min_impulse_px_s: PARTITION_SHOVE_MIN_IMPULSE_PX_S,
    };

    pub fn validate(&self) -> Result<(), String> {
        if !(2..=PARTITION_MAX_BREAK_HITS).contains(&self.break_hits) {
            return Err(format!(
                "break_hits must be 2..={PARTITION_MAX_BREAK_HITS} (a hit that breaks at once is a shove)"
            ));
        }
        if self.shove_min_impulse_px_s == 0 {
            return Err("shove_min_impulse_px_s must be above 0".into());
        }
        Ok(())
    }
}

impl Default for PartitionConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The partition's states: `Intact` (no counted hit), `Weakened` (one or two
/// counted hits, still standing) and `Broken` (its 16 px remnant stands).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionState {
    Intact,
    Weakened,
    Broken,
}

impl PartitionState {
    const fn code(self) -> u64 {
        self as u64
    }
}

/// CORRAL-D: the rotten timber screen across the corral's lane (sheet
/// `enemy-gameplay-llama-rush.md` section 10, CRL-Q30, DV-9).
///
/// * a **melee hit** of a distinct move instance counts one (the per-move
///   weight is not read: three *distinct* hits, CD-AD2); the third breaks it;
/// * an enemy **shove** at or above the threshold (a rush contact) breaks it at
///   once, on the tick of the contact;
/// * a projectile (Magic Missile), a launched body, a blast and a prop contact
///   are `NotAccepted`: nothing but a body hit or a rush damages it (DV-9).
///
/// A broken partition stays broken for the attempt (`reset` restores it).
#[derive(Debug, Clone)]
pub struct Partition {
    cfg: PartitionConfig,
    state: PartitionState,
    hits: u8,
    tokens: Vec<u32>,
    /// Set on the offer that broke it, taken by the step that applies the
    /// remnant-lift rule (the same tick).
    just_broken: bool,
    initiator: Option<u32>,
}

impl Partition {
    pub fn new(cfg: PartitionConfig) -> Result<Self, String> {
        cfg.validate()?;
        Ok(Self {
            cfg,
            state: PartitionState::Intact,
            hits: 0,
            tokens: Vec::new(),
            just_broken: false,
            initiator: None,
        })
    }

    pub fn config(&self) -> &PartitionConfig {
        &self.cfg
    }
    pub fn state(&self) -> PartitionState {
        self.state
    }
    /// Counted hits so far (0 once restored).
    pub fn hits(&self) -> u8 {
        self.hits
    }
    pub fn is_standing(&self) -> bool {
        self.state != PartitionState::Broken
    }
    pub fn is_broken(&self) -> bool {
        self.state == PartitionState::Broken
    }
    /// The player credited with the break, if any.
    pub fn initiator(&self) -> Option<u32> {
        self.initiator
    }
    pub fn reset(&mut self) {
        *self = Self::new(self.cfg).expect("a built partition has a valid config");
    }

    /// A checkpoint respawn restarts the heroes' move counters: forget which move instances were
    /// counted (the counted hits stay), so the same token counts again.
    pub fn forget_hit_tokens(&mut self) {
        self.tokens.clear();
    }

    /// Setup only (the CORRAL-AUTO `broken` variant): the partition as authored
    /// pre-broken. Never called mid-run.
    pub fn set_broken_for_setup(&mut self) {
        self.state = PartitionState::Broken;
        self.hits = self.cfg.break_hits;
        self.just_broken = false;
    }

    /// Whether the last offer broke it and the remnant lift is still due; the
    /// flag clears.
    pub fn take_just_broken(&mut self) -> bool {
        std::mem::take(&mut self.just_broken)
    }

    pub fn offer(&mut self, stimulus: Stimulus) -> Offer {
        if self.state == PartitionState::Broken {
            return Offer::Ignored(Ignored::AlreadyActivated);
        }
        match stimulus {
            Stimulus::MeleeHit { token, player, .. } => {
                if self.tokens.contains(&token) {
                    return Offer::Ignored(Ignored::Duplicate);
                }
                self.tokens.push(token);
                self.hits += 1;
                if self.hits >= self.cfg.break_hits {
                    self.broken_by(Some(player))
                } else {
                    self.state = PartitionState::Weakened;
                    Offer::Progress {
                        progress: self.hits,
                    }
                }
            }
            Stimulus::Shove {
                impulse_px_s,
                credit,
                ..
            } => {
                if impulse_px_s < self.cfg.shove_min_impulse_px_s {
                    return Offer::Ignored(Ignored::ShoveTooWeak);
                }
                self.broken_by(credit)
            }
            Stimulus::PropContact { depth, .. } => {
                if depth >= MAX_CHAIN_DEPTH {
                    return Offer::Ignored(Ignored::ChainTooDeep);
                }
                Offer::Ignored(Ignored::NotAccepted)
            }
            Stimulus::Projectile { .. }
            | Stimulus::LaunchedBody { .. }
            | Stimulus::Blast { .. } => Offer::Ignored(Ignored::NotAccepted),
        }
    }

    fn broken_by(&mut self, initiator: Option<u32>) -> Offer {
        self.state = PartitionState::Broken;
        self.just_broken = true;
        self.initiator = initiator;
        Offer::Triggered {
            initiator,
            depth: 0,
        }
    }

    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.state.code());
        mix(u64::from(self.hits));
        mix(self.initiator.map_or(0, |p| u64::from(p) + 1));
        mix(self.tokens.len() as u64);
        for t in &self.tokens {
            mix(u64::from(*t));
        }
    }
}

// ---------------------------------------------------------------------------
// BOIL-D: the fuel drum and the prop link graph.
// ---------------------------------------------------------------------------

/// The drum's tunables. The telegraph comes from the encounter data (the owned
/// blast hazard's `telegraph_ticks`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrumConfig {
    /// A launched body counts only from this speed (px/s). Default 160, the
    /// valve's and the brace's value (BOILC-AD2): a kicked goblin (170) counts.
    pub launch_min_impulse_px_s: u16,
    /// Ticks between the trigger and the blast, at least
    /// [`MIN_WARNING_TICKS`] (a hero-hurting blast shows its tell first).
    pub telegraph_ticks: u16,
}

impl DrumConfig {
    pub const DEFAULT: Self = Self {
        launch_min_impulse_px_s: 160,
        telegraph_ticks: 24,
    };

    pub fn validate(&self) -> Result<(), String> {
        if self.telegraph_ticks < MIN_WARNING_TICKS {
            return Err(format!(
                "the drum's telegraph must be at least {MIN_WARNING_TICKS} ticks, is {}",
                self.telegraph_ticks
            ));
        }
        Ok(())
    }
}

impl Default for DrumConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The states of a fuel drum, `armed -> telegraph -> detonated`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrumState {
    /// Standing, untouched: a solid on the lane.
    Armed,
    /// Set off: the area tell runs, the drum still stands.
    Telegraph,
    /// The blast went off: the drum is gone for the attempt.
    Detonated,
}

impl DrumState {
    const fn code(self) -> u64 {
        self as u64
    }
}

/// The fuel drum's acceptance machine (Boiler P3). It decides **whether** a
/// stimulus sets the drum off; **when** it blows is the clock of its
/// `BlastCharge` (the scene arms the charge with the trigger tick plus
/// [`DrumConfig::telegraph_ticks`], and reports the blast back through
/// [`Drum::detonated`]), so a chain-reaction blast (CP-Q34) and a prop link share
/// one detonation clock.
///
/// Accepted: a **launched body** from the threshold up (once per launch), a
/// **blast** (once per blast id) and a **prop contact** (a link, depth below
/// [`MAX_CHAIN_DEPTH`]). Hero melee, a projectile and a shove are
/// `NotAccepted` (BOILD-AD3): punching a fuel drum is not a blast, and the
/// drum is no hit target of the missile either. One activation per attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drum {
    cfg: DrumConfig,
    state: DrumState,
    launches: Vec<u32>,
    blasts: Vec<u32>,
    initiator: Option<u32>,
    depth: u8,
}

impl Drum {
    pub fn new(cfg: DrumConfig) -> Result<Self, String> {
        cfg.validate()?;
        Ok(Self {
            cfg,
            state: DrumState::Armed,
            launches: Vec::new(),
            blasts: Vec::new(),
            initiator: None,
            depth: 0,
        })
    }

    pub fn config(&self) -> &DrumConfig {
        &self.cfg
    }
    pub fn state(&self) -> DrumState {
        self.state
    }
    /// The player credited with the blast.
    pub fn initiator(&self) -> Option<u32> {
        self.initiator
    }
    /// Chain depth of what set the drum off; a link out of its blast carries
    /// `depth + 1`.
    pub fn trigger_depth(&self) -> u8 {
        self.depth
    }
    /// Whether a blast can still set it off (the blast hook's footprint).
    pub fn is_armed(&self) -> bool {
        self.state == DrumState::Armed
    }
    pub fn reset(&mut self) {
        *self = Self::new(self.cfg).expect("a built drum has a valid config");
    }

    fn trigger(&mut self, stimulus: &Stimulus) -> Offer {
        self.state = DrumState::Telegraph;
        self.initiator = stimulus.initiator();
        self.depth = stimulus.depth();
        Offer::Triggered {
            initiator: self.initiator,
            depth: self.depth,
        }
    }

    /// Offer one stimulus. Only an armed drum reacts; every ignored case is
    /// named. `Triggered` means the scene must arm the drum's charge.
    pub fn offer(&mut self, stimulus: Stimulus) -> Offer {
        if self.state != DrumState::Armed {
            return Offer::Ignored(Ignored::AlreadyActivated);
        }
        match stimulus {
            Stimulus::MeleeHit { .. } | Stimulus::Projectile { .. } | Stimulus::Shove { .. } => {
                Offer::Ignored(Ignored::NotAccepted)
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
                self.trigger(&stimulus)
            }
            Stimulus::Blast { blast_id, .. } => {
                if self.blasts.contains(&blast_id) {
                    return Offer::Ignored(Ignored::Duplicate);
                }
                self.blasts.push(blast_id);
                self.trigger(&stimulus)
            }
            Stimulus::PropContact { depth, .. } => {
                if depth >= MAX_CHAIN_DEPTH {
                    return Offer::Ignored(Ignored::ChainTooDeep);
                }
                self.trigger(&stimulus)
            }
        }
    }

    /// The drum's charge was lit by a chain reaction (CP-Q34) instead of an
    /// offer: the machine follows it into the telegraph with the credit and
    /// depth the charge took. Returns whether it changed (an armed drum only).
    pub fn adopt_chain(&mut self, initiator: Option<u32>, depth: u8) -> bool {
        if self.state != DrumState::Armed {
            return false;
        }
        self.state = DrumState::Telegraph;
        self.initiator = initiator;
        self.depth = depth;
        true
    }

    /// The charge blew: the drum is gone for the attempt.
    pub fn detonated(&mut self) {
        self.state = DrumState::Detonated;
    }

    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.state.code());
        mix(self.initiator.map_or(0, |p| u64::from(p) + 1));
        mix(u64::from(self.depth));
        for list in [&self.launches, &self.blasts] {
            mix(list.len() as u64);
            for id in list {
                mix(u64::from(*id));
            }
        }
    }
}

/// What a prop does that an authored link can carry (the `LinkKind` of the
/// sidecar, without the parser): the steam of a vent, the footprint of a
/// landing, the radius of a blast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkEmit {
    /// The valve's steam reaches the linked prop when the vent opens.
    Steam,
    /// The mass' landing footprint reaches the linked prop on the landing.
    Landing,
    /// The drum's blast radius reaches the linked prop when it blows.
    Blast,
}

/// The authored prop-to-prop links (K2): at most [`MAX_AUTHORED_LINKS`], acyclic,
/// one per ordered pair. A link carries the **chain depth**: a contact out of a
/// prop that was set off at depth `d` is depth `d + 1`, and the machines refuse
/// a contact at [`MAX_CHAIN_DEPTH`] or more ([`Ignored::ChainTooDeep`]), the
/// same counter as the charge chains (CP-Q34). The graph holds no prop state:
/// it only answers who a source reaches.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LinkGraph {
    /// `(from, to, emit)` in sidecar order.
    links: Vec<(u16, u16, LinkEmit)>,
}

/// K2 / #176: at most this many authored links (the encounter validator
/// enforces it as well; the runtime graph refuses more).
pub const MAX_LINKS: usize = 3;

impl LinkGraph {
    pub fn new(links: Vec<(u16, u16, LinkEmit)>) -> Result<Self, String> {
        if links.len() > MAX_LINKS {
            return Err(format!("{} links, at most {MAX_LINKS}", links.len()));
        }
        for (i, (from, to, _)) in links.iter().enumerate() {
            if from == to {
                return Err(format!("prop {from} links to itself"));
            }
            if links[..i].iter().any(|(f, t, _)| f == from && t == to) {
                return Err(format!("duplicate link {from} -> {to}"));
            }
        }
        let graph = Self { links };
        if graph.has_cycle() {
            return Err("the links form a cycle".into());
        }
        Ok(graph)
    }

    fn has_cycle(&self) -> bool {
        // Kahn's algorithm over the props the links name.
        let mut nodes: Vec<u16> = self.links.iter().flat_map(|(f, t, _)| [*f, *t]).collect();
        nodes.sort_unstable();
        nodes.dedup();
        let mut indegree: Vec<usize> = nodes
            .iter()
            .map(|n| self.links.iter().filter(|(_, t, _)| t == n).count())
            .collect();
        let mut stack: Vec<usize> = (0..nodes.len()).filter(|i| indegree[*i] == 0).collect();
        let mut seen = 0;
        while let Some(i) = stack.pop() {
            seen += 1;
            for (f, t, _) in &self.links {
                if *f == nodes[i] {
                    let j = nodes.binary_search(t).expect("a named node");
                    indegree[j] -= 1;
                    if indegree[j] == 0 {
                        stack.push(j);
                    }
                }
            }
        }
        seen != nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    pub fn links(&self) -> &[(u16, u16, LinkEmit)] {
        &self.links
    }

    /// The props `from` reaches with `emit`, in sidecar order.
    pub fn targets(&self, from: u16, emit: LinkEmit) -> Vec<u16> {
        self.links
            .iter()
            .filter(|(f, _, e)| *f == from && *e == emit)
            .map(|(_, t, _)| *t)
            .collect()
    }

    /// The links that end in `to`.
    pub fn incoming(&self, to: u16) -> Vec<(u16, LinkEmit)> {
        self.links
            .iter()
            .filter(|(_, t, _)| *t == to)
            .map(|(f, _, e)| (*f, *e))
            .collect()
    }

    /// The contact a prop `from`, set off at `trigger_depth` by `initiator`,
    /// hands to a linked prop: one link deeper (saturating).
    pub const fn contact(from: u16, initiator: Option<u32>, trigger_depth: u8) -> Stimulus {
        Stimulus::PropContact {
            source: from,
            initiator,
            depth: trigger_depth.saturating_add(1),
        }
    }

    /// The longest path of the graph in links (at most [`MAX_LINKS`]).
    pub fn longest_chain(&self) -> usize {
        fn from(links: &[(u16, u16, LinkEmit)], node: u16) -> usize {
            links
                .iter()
                .filter(|(f, _, _)| *f == node)
                .map(|(_, t, _)| 1 + from(links, *t))
                .max()
                .unwrap_or(0)
        }
        self.links
            .iter()
            .map(|(f, _, _)| from(&self.links, *f))
            .max()
            .unwrap_or(0)
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
            depth: 0,
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
            depth: 0,
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
                depth: 0,
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
                owner: Some(1),
                depth: 0
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
            m.break_by_blast(3, Some(0), 0),
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
            half.break_by_blast(4, None, 0),
            Offer::Progress { progress: 1 }
        );
        assert_eq!(
            half.break_by_blast(4, None, 0),
            Offer::Ignored(Ignored::Duplicate)
        );
        assert_eq!(
            m.break_by_blast(9, None, 0),
            Offer::Triggered {
                initiator: None,
                depth: 0
            }
        );
        // The entry point is the Blast stimulus, nothing more.
        let mut a = mass();
        let mut b = mass();
        a.break_by_blast(5, Some(1), 0);
        b.offer(Stimulus::Blast {
            blast_id: 5,
            owner: Some(1),
            depth: 0,
        });
        assert_eq!(a, b);
        // A mass that takes no blasts says so.
        let mut deaf = FallingMass::new(FallingMassConfig {
            blast_weight: 0,
            ..jb()
        })
        .unwrap();
        assert_eq!(
            deaf.break_by_blast(1, None, 0),
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
        m.break_by_blast(1, None, 0);
        let all = [
            jab(9, 0),
            launch(2, 400),
            Stimulus::Blast {
                blast_id: 7,
                owner: None,
                depth: 0,
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

    /// F01-SIM-TICKS: `step_at` records the scene tick of the landing edge,
    /// exactly once, deterministically, and outside the state hash.
    #[test]
    fn step_at_records_the_landing_tick_once_unhashed_and_reset_clears_it() {
        let run = || {
            let mut m = mass();
            m.offer(jab(1, 0));
            m.offer(jab(2, 0));
            let mut seen = Vec::new();
            for tick in 1000..1040u64 {
                let edge = m.step_at(tick, &[]);
                if matches!(edge, Some(MassEdge::Landed(_))) {
                    assert_eq!(m.landed_tick(), Some(tick), "set on the landing step");
                }
                seen.push(m.landed_tick());
            }
            (m, seen)
        };
        let (m, seen) = run();
        // Landing evaluated on R + 31 (the 32nd step): None before, then the
        // same value on every later step (set once, never moved).
        assert!(seen[..31].iter().all(Option::is_none));
        assert!(seen[31..].iter().all(|t| *t == Some(1031)));
        // Deterministic: a second identical run records the same ticks.
        assert_eq!(run().1, seen);
        // Unhashed: the plain `step` twin has the same hash and no tick.
        let mut twin = mass();
        twin.offer(jab(1, 0));
        twin.offer(jab(2, 0));
        for _ in 0..40 {
            twin.step(&[]);
        }
        assert_eq!(twin.landed_tick(), None);
        assert_eq!(hash(&twin), hash(&m));
        // A reset clears it like every other field.
        let mut m = m;
        m.reset();
        assert_eq!(m.landed_tick(), None);
        assert_eq!(m, mass());
    }

    #[test]
    fn the_mass_drops_on_a_gravity_curve_and_lands_exactly_on_the_floor() {
        let mut m = mass();
        let hang = jb().hanging;
        assert_eq!(m.mass_rect(), Some(hang));
        assert_eq!(m.swept_mass_rect(), None);
        m.break_by_blast(1, None, 0);
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
        m.break_by_blast(1, None, 0);
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
            m.break_by_blast(1, None, 0);
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
        m.break_by_blast(1, None, 0);
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
        m.break_by_blast(1, None, 0);
        let (_, l) = run_to_landing(&mut m, &[leaving]);
        assert_eq!(l.hits.len(), 1);
        // A teleport across the footprint passes prev == cur: no hit.
        let warped = MassBody::still(
            Victim::hero(0),
            IRect::new(footprint.right() + 10, FLOOR - 56, 28, 56),
        );
        let mut m = mass();
        m.break_by_blast(1, None, 0);
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
            m.break_by_blast(1, Some(0), 0);
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
        m.break_by_blast(1, None, 0);
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
        m.break_by_blast(1, None, 0);
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
        m.break_by_blast(1, None, 0);
        let bodies = [hero_at(0, 790), hero_at(1, 840), hero_at(2, 816)];
        let (_, l) = run_to_landing(&mut m, &bodies);
        let knocks: Vec<i16> = l.hits.iter().map(|h| h.knock_x_px_s).collect();
        assert_eq!(knocks, [-120, 120, 120]);
        assert!(l.hits.iter().all(|h| h.knock_up_px_s == 140));
    }

    #[test]
    fn one_outcome_per_victim_even_when_a_body_is_listed_twice() {
        let mut m = mass();
        m.break_by_blast(1, None, 0);
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
        m.break_by_blast(1, Some(0), 0);
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
        m.break_by_blast(1, None, 0);
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
        m.break_by_blast(1, None, 0);
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
        d.break_by_blast(1, None, 0);
        e.break_by_blast(1, None, 0);
        run_to_landing(&mut d, &[hero_at(0, 800)]);
        run_to_landing(&mut e, &[]);
        assert_ne!(hash(&d), hash(&e), "who was hit is state");
    }

    #[test]
    fn reset_restores_the_initial_state_exactly_and_it_can_fall_again() {
        let fresh = hash(&mass());
        let mut m = mass();
        m.offer(jab(1, 0));
        m.break_by_blast(2, Some(1), 0);
        run_to_landing(&mut m, &[hero_at(0, 800)]);
        assert_ne!(hash(&m), fresh);
        m.reset();
        assert_eq!(hash(&m), fresh);
        assert_eq!(m, mass());
        assert_eq!(m.remnant(), None);
        assert_eq!(m.telegraph_rect(), None);
        // A second attempt is identical to the first.
        let mut a = mass();
        a.break_by_blast(2, Some(1), 0);
        let (t1, l1) = run_to_landing(&mut a, &[hero_at(0, 800)]);
        a.reset();
        a.break_by_blast(2, Some(1), 0);
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
        m.break_by_blast(1, None, 0);
        let (tick, _) = run_to_landing(&mut m, &[]);
        assert_eq!(tick, 30 + 12 - 1);
    }
}

#[cfg(test)]
mod shutter_tests {
    use super::*;

    const FLOOR: i32 = 580;
    /// The Ring court's arch: 68 px from the slab underside to the floor.
    const ARCH: IRect = IRect::new(1200, FLOOR - 68, 32, 68);

    fn cfg() -> ShutterConfig {
        ShutterConfig {
            arm_x: Some(1008),
            ..ShutterConfig::defaults(ShutterInitial::Open)
        }
    }

    fn shutter() -> Shutter {
        Shutter::new(cfg(), ARCH).unwrap()
    }

    fn body(victim: Victim, cx: i32, h: i32, state: ShutterBodyState) -> ShutterBody {
        ShutterBody::new(
            MassBody::still(victim, IRect::new(cx - 14, FLOOR - h, 28, h)),
            state,
        )
    }

    fn goblin(id: u32) -> ShutterBody {
        body(Victim::enemy(id), 1216, 38, ShutterBodyState::Free)
    }

    fn carl() -> ShutterBody {
        body(Victim::hero(0), 1216, 52, ShutterBodyState::Free)
    }

    fn hash(s: &Shutter) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325_u64;
        s.mix_into(&mut |x| {
            h ^= x;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        });
        h
    }

    /// Step until `pred` holds, returning how many steps it took (at most 600).
    fn run_until(s: &mut Shutter, bodies: &[ShutterBody], pred: impl Fn(&Shutter) -> bool) -> u32 {
        for n in 1..=600 {
            s.step(bodies);
            if pred(s) {
                return n;
            }
        }
        panic!("never reached the state");
    }

    // ---- the knobs (RING-B core)

    #[test]
    fn the_defaults_are_the_adopted_numbers_and_validate() {
        let c = cfg();
        assert_eq!(
            (
                c.warning_ticks,
                c.speed_px_per_tick,
                c.slice_px,
                c.cycles,
                c.pin_ticks,
                c.max_closed_ticks,
                c.arm_trail_px
            ),
            (40, 2, 4, 1, 36, 150, 320)
        );
        assert_eq!(c.validate(ARCH), Ok(()));
        assert_eq!(c.closing_ticks(68), 34);
        assert_eq!(c.blocked_window_ticks(68), 218);
        let s = ShutterConfig::defaults(ShutterInitial::Closed);
        assert!(s.is_static());
        assert_eq!(s.validate(ARCH), Ok(()));
    }

    #[test]
    fn the_closing_tick_bounds_hold_on_both_sides() {
        // ceil(h / 2): 48 and 47 px close in 24, 46 px in 23; 72 px in 36, 73 in 37.
        let at = |h: i32, speed: u8| {
            ShutterConfig {
                speed_px_per_tick: speed,
                arm_x: Some(0),
                slice_px: 1,
                ..cfg()
            }
            .validate(IRect::new(0, 0, 32, h))
        };
        assert_eq!(at(48, 2), Ok(()));
        assert_eq!(at(47, 2), Ok(()));
        assert!(at(46, 2).unwrap_err().contains("closes in 23"));
        assert_eq!(at(72, 2), Ok(()));
        assert!(at(73, 2).unwrap_err().contains("closes in 37"));
        // One px per tick: 24..=36 px.
        assert_eq!(at(24, 1), Ok(()));
        assert_eq!(at(36, 1), Ok(()));
        assert!(at(23, 1).is_err());
        assert!(at(37, 1).is_err());
        assert!(at(0, 2).is_err());
    }

    #[test]
    fn arm_ready_needs_a_crossed_arm_x_and_a_near_trailing_hero() {
        let c = cfg();
        // arm_x 1008, the shutter at [1200, 1232), trail 320.
        assert!(!c.arm_ready(ARCH, &[]), "no living hero");
        assert!(!c.arm_ready(ARCH, &[1007]), "not crossed");
        assert!(c.arm_ready(ARCH, &[1008]), "crossed, trailing 192 px away");
        // The trailing hero decides: 1200 - 880 = 320 is the limit.
        assert!(c.arm_ready(ARCH, &[1100, 880]));
        assert!(
            !c.arm_ready(ARCH, &[1100, 879]),
            "trailing hero 321 px back"
        );
        // A hero that passed the shutter is at distance 0 (the far side).
        assert!(c.arm_ready(ARCH, &[1300, 1100]));
        // A static gate never arms.
        let s = ShutterConfig {
            arm_x: Some(1008),
            ..ShutterConfig::defaults(ShutterInitial::Closed)
        };
        assert!(!s.arm_ready(ARCH, &[1100]));
    }

    // ---- the phases

    #[test]
    fn nothing_moves_before_the_warning_ends() {
        let mut s = shutter();
        assert_eq!(s.phase(), ShutterPhase::Open);
        assert_eq!((s.edge_px(), s.intact_slices()), (0, 0));
        assert_eq!(s.solid_rect(), None);
        // Stepping an open shutter changes nothing.
        let h0 = hash(&s);
        for _ in 0..10 {
            assert_eq!(s.step(&[]), ShutterTick::default());
        }
        assert_eq!(hash(&s), h0);
        assert!(s.arm());
        assert_eq!(s.phase(), ShutterPhase::Warning);
        assert_eq!(s.telegraph_rect(), Some(ARCH));
        // Warning steps 1..=39: nothing moves, no slice is intact.
        for n in 1..40 {
            let t = s.step(&[]);
            assert_eq!(t, ShutterTick::default(), "step {n}");
            assert_eq!(s.phase(), ShutterPhase::Warning);
            assert_eq!((s.edge_px(), s.intact_slices()), (0, 0), "step {n}");
        }
        // Step 40 ends the warning: the phase is Closing, the edge still at 0.
        let t = s.step(&[]);
        assert_eq!(t.edge, Some(ShutterEdge::ClosingStarted));
        assert_eq!(s.phase(), ShutterPhase::Closing);
        assert_eq!((s.edge_px(), s.intact_slices()), (0, 0));
        // The first motion is the next step: 2 px, still no whole slice.
        s.step(&[]);
        assert_eq!((s.edge_px(), s.intact_slices()), (2, 0));
        s.step(&[]);
        assert_eq!((s.edge_px(), s.intact_slices()), (4, 1));
    }

    #[test]
    fn an_unheld_cycle_takes_exactly_the_authored_ticks() {
        let mut s = shutter();
        assert!(s.arm());
        let mut edges = Vec::new();
        for n in 1..=300u32 {
            if let Some(e) = s.step(&[]).edge {
                edges.push((n, e));
            }
        }
        // Warning 40, closing 34 (68 px at 2 px/tick), closed 150, opening 34.
        assert_eq!(
            edges,
            [
                (40, ShutterEdge::ClosingStarted),
                (40 + 34, ShutterEdge::Closed),
                (40 + 34 + 150, ShutterEdge::OpeningStarted),
                (40 + 34 + 150 + 34, ShutterEdge::Opened),
            ]
        );
        assert_eq!(s.phase(), ShutterPhase::Open);
        assert_eq!(s.intact_slices(), 0);
        // KR19: the lane is blocked from Closing to Opened: 34 + 150 + 34.
        assert_eq!(edges[3].0 - edges[0].0, 218);
        assert!(edges[3].0 - edges[0].0 < SHUTTER_STUCK_LIMIT_TICKS);
    }

    #[test]
    fn the_edge_moves_at_most_two_px_per_tick_and_slices_follow_it() {
        let mut s = shutter();
        s.arm();
        run_until(&mut s, &[], |s| s.phase() == ShutterPhase::Closing);
        let mut last = 0;
        for tick in 1..=34 {
            s.step(&[]);
            assert!(s.edge_px() - last <= 2, "tick {tick}");
            last = s.edge_px();
            // A slice is intact once the edge has passed its bottom (4 px each).
            assert_eq!(s.intact_slices(), (last / 4) as usize);
            for k in 0..s.slice_count() {
                assert_eq!(s.slice_intact(k), (k as i32 + 1) * 4 <= last, "slice {k}");
            }
            assert_eq!(s.lower_edge_y(), ARCH.y + last);
        }
        assert_eq!(s.phase(), ShutterPhase::Closed);
        assert_eq!(s.slice_count(), 17);
        assert_eq!(s.intact_slices(), 17);
        assert_eq!(s.solid_rect(), Some(ARCH));
        // The stack is contiguous from the top and each slice is 4 px.
        assert_eq!(s.slice_rect(0), IRect::new(1200, ARCH.y, 32, 4));
        assert_eq!(s.slice_rect(16).bottom(), FLOOR);
    }

    #[test]
    fn opening_removes_slices_from_the_bottom_at_the_same_speed() {
        let mut s = shutter();
        s.arm();
        run_until(&mut s, &[], |s| s.phase() == ShutterPhase::Opening);
        assert_eq!(s.intact_slices(), 17);
        let mut last = 68;
        for _ in 0..34 {
            s.step(&[]);
            assert!(last - s.edge_px() <= 2);
            last = s.edge_px();
            assert_eq!(s.intact_slices(), (last / 4) as usize);
        }
        assert_eq!(s.phase(), ShutterPhase::Open);
        assert_eq!(s.solid_rect(), None);
    }

    // ---- the hold rule (KR4)

    #[test]
    fn a_living_body_under_the_edge_holds_it_and_a_downed_one_does_not() {
        // A goblin (38 px) standing in the arch: the slice that would reach its
        // head (y 542) is the 8th (rows 32..36 from the top, y 544..548) .
        let mut s = shutter();
        s.arm();
        run_until(&mut s, &[], |s| s.phase() == ShutterPhase::Closing);
        let g = [goblin(1)];
        let mut held_ticks = 0;
        for _ in 0..80 {
            let t = s.step(&g);
            held_ticks += u32::from(t.held);
        }
        assert!(held_ticks > 0, "the goblin holds the edge");
        assert_eq!(s.phase(), ShutterPhase::Closing, "never closes over it");
        // The edge stopped above the goblin's head: no intact slice overlaps it.
        let head = FLOOR - 38;
        assert!(s.solid_rect().unwrap().bottom() <= head);
        assert!(s.edge_px() < 68);
        // Carl (52 px) is held the same way, and higher up.
        let mut s2 = shutter();
        s2.arm();
        run_until(&mut s2, &[], |s| s.phase() == ShutterPhase::Closing);
        for _ in 0..80 {
            s2.step(&[carl()]);
        }
        assert!(s2.solid_rect().unwrap().bottom() <= FLOOR - 52);
        assert!(
            s2.intact_slices() < s.intact_slices(),
            "the taller body holds higher"
        );
        // A downed or dead goblin is not a body: the shutter closes over it.
        let mut s3 = shutter();
        s3.arm();
        let gone = [body(Victim::enemy(1), 1216, 38, ShutterBodyState::Gone)];
        let n = run_until(&mut s3, &gone, |s| s.phase() == ShutterPhase::Closed);
        assert_eq!(n, 40 + 34);
    }

    #[test]
    fn held_for_n_ticks_then_released_it_completes_n_ticks_late() {
        for n_held in [1u32, 17, 60] {
            let mut s = shutter();
            s.arm();
            run_until(&mut s, &[], |s| s.phase() == ShutterPhase::Closing);
            // Let it close until it reaches the goblin's head, then hold N ticks.
            let mut steps = 0u32;
            let mut held = 0u32;
            while held < n_held {
                let t = s.step(&[goblin(1)]);
                steps += 1;
                held += u32::from(t.held);
                assert!(steps < 500);
            }
            assert_eq!(s.phase(), ShutterPhase::Closing);
            // Released: the body is gone (or walked away).
            let mut free_steps = 0;
            while s.phase() == ShutterPhase::Closing {
                s.step(&[]);
                free_steps += 1;
                assert!(free_steps < 100);
            }
            assert_eq!(s.phase(), ShutterPhase::Closed);
            assert_eq!(s.intact_slices(), 17);
            // Total closing steps = 34 (the travel) + the held ticks.
            assert_eq!(steps + free_steps, 34 + n_held, "held {n_held}");
        }
    }

    #[test]
    fn no_slice_ever_turns_intact_over_a_body() {
        // Bodies of several heights drift through the arch while it closes and
        // opens: after every step no intact slice overlaps any living body.
        let mut s = shutter();
        s.arm();
        for tick in 0..300u32 {
            let cx = 1180 + ((tick * 7) % 60) as i32;
            let cx2 = 1250 - ((tick * 5) % 70) as i32;
            let prev = |c: i32, h: i32| IRect::new(c - 14, FLOOR - h, 28, h);
            let bodies = [
                ShutterBody::new(
                    MassBody {
                        victim: Victim::hero(0),
                        prev: prev(cx - 7, 52),
                        cur: prev(cx, 52),
                    },
                    ShutterBodyState::Free,
                ),
                ShutterBody::new(
                    MassBody {
                        victim: Victim::enemy(3),
                        prev: prev(cx2 + 5, 38),
                        cur: prev(cx2, 38),
                    },
                    ShutterBodyState::Busy,
                ),
            ];
            let before = s.intact_slices();
            s.step(&bodies);
            // Every slice that turned intact on this step is clear of every
            // living body (a body that walks into a closed arch later is the
            // collision's business, not the shutter's).
            for k in before..s.intact_slices() {
                for b in &bodies {
                    let swept = swept_rect(b.body.prev, b.body.cur);
                    assert!(
                        !s.slice_rect(k).overlaps(swept) && !s.slice_rect(k).overlaps(b.body.cur),
                        "tick {tick}: slice {k} turned intact over {:?}",
                        b.body.cur
                    );
                }
            }
        }
        // The same run with a body that stays under the edge never closes it.
        let mut s = shutter();
        s.arm();
        for _ in 0..300 {
            s.step(&[goblin(1)]);
            assert!(
                s.solid_rect()
                    .is_none_or(|r| !r.overlaps(goblin(1).body.cur))
            );
        }
        assert_eq!(s.phase(), ShutterPhase::Closing);
    }

    /// Close `s` against one still body for 200 closing steps; return the edge
    /// it settled at. The drawn lower edge must never be below the body's top.
    fn settle_edge(s: &mut Shutter, h: i32) -> i32 {
        s.arm();
        run_until(s, &[], |s| s.phase() == ShutterPhase::Closing);
        let b = [body(Victim::hero(0), 1216, h, ShutterBodyState::Free)];
        let top = FLOOR - h;
        for _ in 0..200 {
            s.step(&b);
            assert!(
                s.lower_edge_y() <= top,
                "edge {} sank into a body whose top is {top}",
                s.lower_edge_y()
            );
        }
        assert_eq!(s.phase(), ShutterPhase::Closing, "the body holds it");
        s.edge_px()
    }

    #[test]
    fn the_drawn_edge_holds_above_the_head_at_the_exact_pixel_both_sides() {
        // Defaults: 4 px slices at 2 px per tick, arch top at FLOOR - 68 = 512.
        // Head 32 px below the arch top: the band [28, 32) does not touch it, so
        // the edge comes down to exactly the head and holds there.
        let mut s = shutter();
        assert_eq!(settle_edge(&mut s, 36), 32);
        assert_eq!(s.lower_edge_y(), FLOOR - 36);
        // One pixel higher head (31 below the arch top): the band [28, 32)
        // overlaps it, so the edge holds at the boundary before, 28.
        let mut s = shutter();
        assert_eq!(settle_edge(&mut s, 37), 28);
        // Two pixels lower head (30): same band, same hold at 28 (the old
        // in-slice step would have sunk the edge to 30).
        let mut s = shutter();
        assert_eq!(settle_edge(&mut s, 38), 28);
        // A head 33 below the arch top: the edge may reach 32 (band [32, 36)
        // overlaps the head, so it holds there), a pixel above the head.
        let mut s = shutter();
        assert_eq!(settle_edge(&mut s, 35), 32);
    }

    #[test]
    fn a_wide_slice_holds_the_edge_above_a_body_in_it() {
        // 17 px slices: a body whose head is 8 px below the arch top sits in the
        // first slice, so the edge never leaves the top (the old rule let it
        // descend 15 px into the body).
        let wide = ShutterConfig {
            slice_px: 17,
            ..cfg()
        };
        let mut s = Shutter::new(wide, ARCH).unwrap();
        assert_eq!(settle_edge(&mut s, 60), 0);
        assert_eq!(s.intact_slices(), 0);
        // A head 30 below the top (second slice): the edge holds at the 17 px
        // boundary before the band that touches it.
        let mut s = Shutter::new(wide, ARCH).unwrap();
        assert_eq!(settle_edge(&mut s, 38), 16);
        // One slice for the whole arch (68): any body holds from the start.
        let one = ShutterConfig {
            slice_px: 68,
            ..cfg()
        };
        let mut s = Shutter::new(one, ARCH).unwrap();
        assert_eq!(settle_edge(&mut s, 38), 0);
    }

    #[test]
    fn the_pin_token_is_readable() {
        let mut s = shutter();
        assert!(!s.token_spent(Victim::enemy(1)));
        s.arm();
        run_until(&mut s, &[], |s| s.phase() == ShutterPhase::Closing);
        let pins: Vec<_> = (0..60).flat_map(|_| s.step(&[goblin(1)]).pins).collect();
        assert_eq!(pins.len(), 1);
        assert!(s.token_spent(Victim::enemy(1)));
        assert!(!s.token_spent(Victim::enemy(2)));
        assert!(!s.token_spent(Victim::hero(1)), "heroes carry none");
        let mut two = Shutter::new(ShutterConfig { cycles: 2, ..cfg() }, ARCH).unwrap();
        two.arm();
        run_until(&mut two, &[], |s| s.phase() == ShutterPhase::Closing);
        for _ in 0..60 {
            two.step(&[goblin(1)]);
        }
        assert!(two.token_spent(Victim::enemy(1)));
        run_until(&mut two, &[], |s| s.phase() == ShutterPhase::Open);
        assert!(two.arm());
        assert!(!two.token_spent(Victim::enemy(1)), "arm clears the tokens");
    }

    #[test]
    fn a_fast_body_crossing_the_arch_in_one_tick_still_holds_it() {
        // Swept volume: the body was left of the arch and is right of it now, so
        // it crossed the column during the tick.
        let mut s = shutter();
        s.arm();
        run_until(&mut s, &[], |s| s.phase() == ShutterPhase::Closing);
        // Close until the next step would turn the slice at the goblin's head.
        let mut held = false;
        for _ in 0..40 {
            let b = ShutterBody::new(
                MassBody {
                    victim: Victim::hero(1),
                    prev: IRect::new(1100, FLOOR - 52, 28, 52),
                    cur: IRect::new(1260, FLOOR - 52, 28, 52),
                },
                ShutterBodyState::Busy,
            );
            held |= s.step(&[b]).held;
        }
        assert!(held);
        // A body that is clear of the column by a pixel does not hold it.
        let mut s = shutter();
        s.arm();
        let clear = [ShutterBody::new(
            MassBody::still(Victim::hero(1), IRect::new(1232, FLOOR - 52, 28, 52)),
            ShutterBodyState::Free,
        )];
        let n = run_until(&mut s, &clear, |s| s.phase() == ShutterPhase::Closed);
        assert_eq!(n, 74);
    }

    // ---- pins (KR4)

    #[test]
    fn one_pin_per_enemy_per_closure() {
        let mut s = shutter();
        s.arm();
        run_until(&mut s, &[], |s| s.phase() == ShutterPhase::Closing);
        let mut pins = Vec::new();
        for _ in 0..120 {
            pins.extend(s.step(&[goblin(1), goblin(2)]).pins);
        }
        // Each goblin is pinned once, however long it holds the edge.
        assert_eq!(pins.len(), 2, "{pins:?}");
        assert_eq!(pins[0].victim, Victim::enemy(1));
        assert_eq!(pins[1].victim, Victim::enemy(2));
        assert!(pins.iter().all(|p| p.ticks == 36 && p.source.is_none()));
    }

    #[test]
    fn heroes_and_busy_bodies_hold_but_are_never_pinned() {
        let mut s = shutter();
        s.arm();
        run_until(&mut s, &[], |s| s.phase() == ShutterPhase::Closing);
        let busy = body(Victim::enemy(4), 1216, 38, ShutterBodyState::Busy);
        for _ in 0..100 {
            let t = s.step(&[carl(), busy]);
            assert!(t.pins.is_empty(), "{:?}", t.pins);
        }
        assert_eq!(s.phase(), ShutterPhase::Closing);
        // A goblin pinned by walking in is in a Free body, not a Busy one: the
        // busy goblin (mid-launch) is not pinned even when it holds the edge.
        let mut s = shutter();
        s.arm();
        run_until(&mut s, &[], |s| s.phase() == ShutterPhase::Closing);
        let t: Vec<_> = (0..100).flat_map(|_| s.step(&[busy]).pins).collect();
        assert!(t.is_empty());
    }

    #[test]
    fn a_face_hit_spends_the_token_and_prevents_a_later_pin() {
        let mut s = shutter();
        // No face before any slice is intact.
        assert_eq!(s.face_hit(Victim::enemy(1)), FaceHit::NotSolid);
        s.arm();
        run_until(&mut s, &[], |s| s.phase() == ShutterPhase::Closed);
        assert_eq!(s.face_hit(Victim::hero(0)), FaceHit::NotAnEnemy);
        assert_eq!(s.face_hit(Victim::enemy(1)), FaceHit::Spent);
        assert_eq!(s.face_hit(Victim::enemy(1)), FaceHit::Duplicate);
        assert_eq!(s.face_hit(Victim::enemy(2)), FaceHit::Spent);
        // Next closure: tokens are fresh (cycles 2).
        let mut two = Shutter::new(ShutterConfig { cycles: 2, ..cfg() }, ARCH).unwrap();
        two.arm();
        run_until(&mut two, &[], |s| s.phase() == ShutterPhase::Closed);
        assert_eq!(two.face_hit(Victim::enemy(1)), FaceHit::Spent);
        run_until(&mut two, &[], |s| s.phase() == ShutterPhase::Open);
        assert!(two.arm());
        run_until(&mut two, &[], |s| s.phase() == ShutterPhase::Closing);
        // The token of the first closure is gone: a walk-in pins goblin 1 again.
        let pins: Vec<_> = (0..60).flat_map(|_| two.step(&[goblin(1)]).pins).collect();
        assert_eq!(pins.len(), 1);
        // A spent token blocks the pin in the same closure.
        let mut s = shutter();
        s.arm();
        run_until(&mut s, &[], |s| s.phase() == ShutterPhase::Closing);
        s.step(&[]);
        s.step(&[]);
        s.step(&[]);
        assert_eq!(s.face_hit(Victim::enemy(1)), FaceHit::Spent);
        let pins: Vec<_> = (0..120).flat_map(|_| s.step(&[goblin(1)]).pins).collect();
        assert!(pins.is_empty(), "the slam consumed the token: {pins:?}");
    }

    // ---- cycles, static, reset, hash

    #[test]
    fn cycles_limit_the_closures_and_it_stays_open_afterwards() {
        let mut s = shutter();
        assert!(s.arm());
        assert!(!s.arm(), "already warning");
        run_until(&mut s, &[], |s| s.phase() == ShutterPhase::Open);
        assert_eq!(s.closures(), 1);
        assert!(!s.arm(), "cycles 1: one closure per attempt");
        for _ in 0..50 {
            assert_eq!(s.step(&[]), ShutterTick::default());
        }
        assert_eq!(s.phase(), ShutterPhase::Open);
        // Two cycles: a second closure, then the lane stays open.
        let mut two = Shutter::new(ShutterConfig { cycles: 2, ..cfg() }, ARCH).unwrap();
        for expected in [1u8, 2] {
            assert!(two.arm());
            assert_eq!(two.closures(), expected);
            run_until(&mut two, &[], |s| s.phase() == ShutterPhase::Open);
        }
        assert!(!two.arm());
    }

    #[test]
    fn a_static_gate_stays_closed_and_never_arms() {
        let mut s = Shutter::new(ShutterConfig::defaults(ShutterInitial::Closed), ARCH).unwrap();
        assert_eq!(s.phase(), ShutterPhase::Closed);
        assert_eq!(s.solid_rect(), Some(ARCH));
        assert!(!s.arm());
        let h0 = hash(&s);
        for _ in 0..500 {
            assert_eq!(s.step(&[goblin(1), carl()]), ShutterTick::default());
        }
        assert_eq!(hash(&s), h0);
        assert_eq!(s.phase(), ShutterPhase::Closed);
        assert_eq!(s.intact_slices(), 17);
    }

    #[test]
    fn a_bad_config_builds_no_machine() {
        for bad in [
            ShutterConfig {
                speed_px_per_tick: 3,
                ..cfg()
            },
            ShutterConfig {
                warning_ticks: 20,
                ..cfg()
            },
            ShutterConfig {
                arm_x: None,
                ..cfg()
            },
            ShutterConfig {
                slice_px: 5,
                ..cfg()
            },
        ] {
            assert!(Shutter::new(bad, ARCH).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn reset_mid_closing_restores_the_first_state_exactly() {
        let fresh = hash(&shutter());
        let mut s = shutter();
        s.arm();
        run_until(&mut s, &[], |s| s.edge_px() >= 20);
        // Mid-closing, with a goblin pinned and a token spent.
        s.step(&[goblin(1)]);
        s.face_hit(Victim::enemy(2));
        assert_ne!(hash(&s), fresh);
        s.reset();
        assert_eq!(hash(&s), fresh);
        assert_eq!(s, shutter());
        assert_eq!(s.phase(), ShutterPhase::Open);
        assert_eq!((s.edge_px(), s.closures(), s.intact_slices()), (0, 0, 0));
        // A retry after the reset replays the same closure: same hash trail.
        let trail = |s: &mut Shutter| {
            s.arm();
            (0..120)
                .map(|_| {
                    s.step(&[]);
                    hash(s)
                })
                .collect::<Vec<_>>()
        };
        let mut a = shutter();
        let first = trail(&mut a);
        a.reset();
        assert_eq!(trail(&mut a), first);
    }

    #[test]
    fn two_runs_give_the_same_hash() {
        let run = || {
            let mut s = shutter();
            let mut trail = Vec::new();
            for tick in 0..400u32 {
                if tick == 25 {
                    s.arm();
                }
                let bodies: Vec<ShutterBody> = match tick % 90 {
                    0..=29 => vec![goblin(1)],
                    30..=49 => vec![carl(), goblin(2)],
                    _ => Vec::new(),
                };
                let t = s.step(&bodies);
                if tick == 200 {
                    s.face_hit(Victim::enemy(1));
                }
                trail.push((hash(&s), t.held, t.pins.len()));
            }
            (hash(&s), trail)
        };
        let (a, ta) = run();
        let (b, tb) = run();
        assert_eq!(a, b);
        assert_eq!(ta, tb);
        // And the final hash is pinned: a behaviour edit must update it on purpose.
        assert_eq!(a, GOLDEN_HASH, "update with the shutter behaviour");
    }

    const GOLDEN_HASH: u64 = 14_285_659_963_568_459_907;
}

/// PR #200 review items NB-3, NB-5, NB-6 and NB-9, carried by BOIL-C wire.
#[cfg(test)]
mod boilc_review_tests {
    use super::*;

    const FLOOR: i32 = 580;

    /// The Boiler weight: 64 px wide, a 32 px remnant, hanging 128 px up.
    fn boiler() -> FallingMassConfig {
        FallingMassConfig::new(
            IRect::new(784, FLOOR - 128 - 32, 64, 32),
            IRect::new(784, FLOOR - 96, 64, 96),
            IRect::new(784, FLOOR - 32, 64, 32),
        )
    }

    fn jab(token: u32, player: u32) -> Stimulus {
        Stimulus::MeleeHit {
            token,
            player,
            weight: 1,
        }
    }

    fn blast(id: u32, owner: Option<u32>) -> Stimulus {
        Stimulus::Blast {
            blast_id: id,
            owner,
            depth: 0,
        }
    }

    // ---- NB-5 / BOILC-AD9

    #[test]
    fn an_ownerless_blast_that_completes_the_release_keeps_the_weakening_credit() {
        let mut m = FallingMass::new(boiler()).unwrap();
        assert!(matches!(
            m.offer(jab(1, 1)),
            Offer::Progress { progress: 1 }
        ));
        assert_eq!(m.initiator(), Some(1), "the weakening initiator is kept");
        // A drum blast nobody owns releases it: Donut keeps the credit.
        let o = m.offer(blast(9, None));
        assert!(
            matches!(
                o,
                Offer::Triggered {
                    initiator: Some(1),
                    ..
                }
            ),
            "{o:?}"
        );
        assert_eq!(m.initiator(), Some(1));
    }

    #[test]
    fn a_releasing_initiator_wins_over_the_weakening_one_and_nobody_is_not_invented() {
        let mut m = FallingMass::new(boiler()).unwrap();
        m.offer(jab(1, 0));
        m.offer(jab(2, 1));
        assert_eq!(m.initiator(), Some(1), "the hit that completed it");
        // With no weakening hit at all, an ownerless blast has nobody to credit.
        let mut m = FallingMass::new(boiler()).unwrap();
        let o = m.offer(blast(9, None));
        assert!(matches!(
            o,
            Offer::Triggered {
                initiator: None,
                ..
            }
        ));
    }

    #[test]
    fn a_shove_release_is_exempt_the_bot_bait_credit_stays_none() {
        let mut c = boiler();
        c.shove_weight = c.release_weight;
        let mut m = FallingMass::new(c).unwrap();
        m.offer(jab(1, 0));
        let o = m.offer(Stimulus::Shove {
            shove_id: 1,
            impulse_px_s: 180,
            source: 5,
            credit: None,
        });
        assert!(
            matches!(
                o,
                Offer::Triggered {
                    initiator: None,
                    ..
                }
            ),
            "{o:?}"
        );
        assert_eq!(m.initiator(), None);
    }

    // ---- NB-6

    #[test]
    fn the_mass_must_hang_over_its_footprint() {
        assert!(FallingMass::new(boiler()).is_ok());
        for (dx, w) in [(-8, 64), (8, 64), (0, 72)] {
            let mut c = boiler();
            c.hanging.x += dx;
            c.hanging.w = w;
            let err = FallingMass::new(c).expect_err("hangs beside its footprint");
            assert!(err.contains("x range"), "{err}");
        }
        // Narrower and inside is fine.
        let mut c = boiler();
        c.hanging = IRect::new(792, c.hanging.y, 48, 32);
        assert!(FallingMass::new(c).is_ok());
    }

    // ---- NB-9

    #[test]
    fn melee_can_be_switched_off_and_the_default_counts_every_move() {
        let mut c = boiler();
        c.melee_weight = 0;
        let mut m = FallingMass::new(c).unwrap();
        assert_eq!(m.offer(jab(1, 0)), Offer::Ignored(Ignored::NotAccepted));
        assert_eq!(m.progress(), 0);
        assert_eq!(m.state(), MassState::Supported);
        // Every other stimulus still works.
        assert!(matches!(
            m.offer(blast(1, Some(0))),
            Offer::Triggered { .. }
        ));
        // The default (1) is the old behaviour: a heavier move counts for its weight.
        let mut m = FallingMass::new(boiler()).unwrap();
        let heavy = Stimulus::MeleeHit {
            token: 1,
            player: 0,
            weight: 2,
        };
        assert!(matches!(m.offer(heavy), Offer::Triggered { .. }));
        // A floor of 2 makes a weight-1 move count for 2.
        let mut c = boiler();
        c.melee_weight = 2;
        let mut m = FallingMass::new(c).unwrap();
        assert!(matches!(m.offer(jab(1, 0)), Offer::Triggered { .. }));
    }

    // ---- NB-3

    #[test]
    fn a_non_hero_inside_a_remnant_it_cannot_step_over_is_reported_embedded() {
        let drop = |c: FallingMassConfig| {
            let mut m = FallingMass::new(c).unwrap();
            m.offer(blast(1, None));
            let bodies = [
                MassBody::still(Victim::enemy(3), IRect::new(800, FLOOR - 52, 24, 52)),
                MassBody::still(Victim::hero(0), IRect::new(820, FLOOR - 52, 24, 52)),
                MassBody::still(Victim::enemy(4), IRect::new(900, FLOOR - 52, 24, 52)),
            ];
            for _ in 0..400 {
                if let Some(MassEdge::Landed(l)) = m.step(&bodies) {
                    return l;
                }
            }
            panic!("never landed");
        };
        let tall = drop(boiler());
        assert_eq!(
            tall.embedded,
            [Victim::enemy(3)],
            "32 px: a wall to a slot enemy"
        );
        assert_eq!(tall.lifts.len(), 1, "the hero is lifted, the enemy is not");
        // A 16 px remnant is stepped over: nobody is embedded.
        let mut c = boiler();
        c.remnant = IRect::new(784, FLOOR - 16, 64, 16);
        assert!(drop(c).embedded.is_empty());
    }
}

/// The J-A slide variant (BOIL-C wire, plan section 3; Hoarder sheet section 5).
#[cfg(test)]
mod slide_tests {
    use super::*;

    const FLOOR: i32 = 48;

    /// Bank A: `[352, 416) x [0, 48)` above the floor, the remnant
    /// `[296, 376) x [0, 16)` (HRD-Q21), layout y down with the floor at 48.
    fn bank() -> SlideConfig {
        SlideConfig::new(
            IRect::new(352, FLOOR - 48, 64, 48),
            IRect::new(296, FLOOR - 16, 80, 16),
        )
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
    fn the_timeline_is_disturbed_t_to_t_plus_23_slid_t_plus_24_to_39_settled_from_t_plus_40() {
        let mut s = Slide::new(bank()).unwrap();
        // The disturbance on tick T = 100; the steps run every tick.
        let t = 100u32;
        let mut states = Vec::new();
        let mut edges = Vec::new();
        for tick in 90..=150u32 {
            if tick == t {
                assert!(matches!(
                    s.offer(shove(1, 180, Some(0))),
                    Offer::Triggered {
                        initiator: Some(0),
                        ..
                    }
                ));
            }
            // The state the tick *starts* its step in, like the sheet's table.
            states.push((tick, s.state(), s.solid_rect().is_some(), s.mass_rect()));
            if let Some(e) = s.step() {
                edges.push((tick, e));
            }
        }
        let at = |tick: u32| *states.iter().find(|(x, ..)| *x == tick).unwrap();
        assert_eq!(at(99).1, SlideState::Stable);
        assert_eq!(at(t).1, SlideState::Disturbed);
        assert_eq!(at(t + 23).1, SlideState::Disturbed, "the last shudder tick");
        assert!(at(t + 23).2, "still a solid on T + 23");
        assert_eq!(at(t + 24).1, SlideState::Slid, "slid from T + 24");
        assert!(!at(t + 24).2, "the solid is gone on the first slid tick");
        assert_eq!(
            at(t + 24).3,
            Some(IRect::new(352, 0, 64, 48)),
            "not yet moved at the read"
        );
        assert_eq!(
            at(t + 40).1,
            SlideState::Slid,
            "the settling step is T + 40"
        );
        assert_eq!(at(t + 41).1, SlideState::Settled);
        assert_eq!(
            edges,
            [
                (t + 23, SlideEdge::SlideStarted),
                (t + 40, SlideEdge::Settled)
            ]
        );
        // 16 moves of 3 px: 352 -> 304 after the step of T + 39, the remnant
        // covers it.
        assert_eq!(s.remnant(), Some(IRect::new(296, 32, 80, 16)));
        assert_eq!(bank().mass_after(16), IRect::new(304, 0, 64, 48));
    }

    #[test]
    fn the_mass_moves_three_px_per_step_for_sixteen_steps() {
        let mut s = Slide::new(bank()).unwrap();
        s.offer(shove(1, 180, None));
        let mut xs = Vec::new();
        for _ in 0..60 {
            s.step();
            if let Some(m) = s.mass_rect() {
                xs.push(m.x);
            }
        }
        // The first Slid step moves it by 3 (the step after SlideStarted).
        assert_eq!(
            xs.first(),
            Some(&352),
            "starts on the bank (after SlideStarted)"
        );
        assert_eq!(xs.last(), Some(&304));
        assert_eq!(xs.len(), 17, "16 moves plus the start");
        assert!(xs.windows(2).all(|w| w[1] == w[0] - 3));
    }

    #[test]
    fn melee_and_missiles_never_disturb_it_and_the_others_do_once() {
        let mut s = Slide::new(bank()).unwrap();
        let jab = Stimulus::MeleeHit {
            token: 1,
            player: 0,
            weight: 2,
        };
        let missile = Stimulus::Projectile {
            projectile_id: 1,
            owner: Some(1),
            weight: 2,
        };
        assert_eq!(s.offer(jab), Offer::Ignored(Ignored::NotAccepted));
        assert_eq!(s.offer(missile), Offer::Ignored(Ignored::NotAccepted));
        assert_eq!(s.state(), SlideState::Stable);
        // A launched body counts from 160 px/s, a shove too (both edges).
        let launch = |id, speed| Stimulus::LaunchedBody {
            launch_id: id,
            impulse_px_s: speed,
            initiator: Some(0),
        };
        assert_eq!(s.offer(launch(1, 159)), Offer::Ignored(Ignored::TooWeak));
        assert_eq!(
            s.offer(shove(1, 159, None)),
            Offer::Ignored(Ignored::ShoveTooWeak)
        );
        assert!(matches!(s.offer(launch(1, 160)), Offer::Triggered { .. }));
        // One disturbance: nothing else counts.
        assert_eq!(
            s.offer(Stimulus::Blast {
                blast_id: 1,
                owner: None,
                depth: 0
            }),
            Offer::Ignored(Ignored::AlreadyActivated)
        );
    }

    #[test]
    fn each_accepted_kind_can_be_switched_off_and_a_blast_disturbs_it() {
        let mut c = bank();
        c.accepts_shove = false;
        let mut s = Slide::new(c).unwrap();
        assert_eq!(
            s.offer(shove(1, 200, None)),
            Offer::Ignored(Ignored::NotAccepted)
        );
        assert!(matches!(
            s.offer(Stimulus::Blast {
                blast_id: 4,
                owner: Some(1),
                depth: 0
            }),
            Offer::Triggered {
                initiator: Some(1),
                ..
            }
        ));
        assert_eq!(s.initiator(), Some(1));
    }

    #[test]
    fn a_duplicate_launch_and_a_deep_link_are_named() {
        let mut s = Slide::new(bank()).unwrap();
        assert_eq!(
            s.offer(Stimulus::PropContact {
                source: 0,
                initiator: None,
                depth: MAX_CHAIN_DEPTH
            }),
            Offer::Ignored(Ignored::ChainTooDeep)
        );
        assert_eq!(
            s.offer(Stimulus::PropContact {
                source: 0,
                initiator: None,
                depth: 1
            }),
            Offer::Ignored(Ignored::NotAccepted)
        );
    }

    #[test]
    fn the_config_rejects_bad_geometry_and_timing() {
        assert!(Slide::new(bank()).is_ok());
        let bad = |f: fn(&mut SlideConfig)| {
            let mut c = bank();
            f(&mut c);
            Slide::new(c).is_err()
        };
        assert!(bad(|c| c.dx_per_tick = 0));
        assert!(bad(|c| c.warning_ticks = MIN_WARNING_TICKS - 1));
        assert!(bad(|c| c.slide_ticks = 0));
        assert!(bad(|c| c.slide_ticks = MAX_SLIDE_TICKS + 1));
        assert!(bad(|c| c.remnant.y -= 4), "the remnant rests on the floor");
        assert!(bad(|c| c.remnant.h = 64), "no taller than the bank");
        assert!(bad(|c| c.remnant.x = 310), "it covers the final position");
        assert!(bad(|c| c.remnant.w = 60), "it covers the final position");
        // A +x slide is allowed with a remnant on the right.
        let mut c = bank();
        c.dx_per_tick = 3;
        c.remnant = IRect::new(392, FLOOR - 16, 80, 16);
        assert!(Slide::new(c).is_ok());
    }

    #[test]
    fn reset_restores_the_first_state_and_the_hash_follows_the_state() {
        let hash = |s: &Slide| {
            let mut h = 0xcbf2_9ce4_8422_2325_u64;
            s.mix_into(&mut |x| {
                h ^= x;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            });
            h
        };
        let fresh = Slide::new(bank()).unwrap();
        let mut s = fresh.clone();
        s.offer(shove(1, 180, Some(0)));
        for _ in 0..30 {
            s.step();
        }
        assert_ne!(hash(&s), hash(&fresh));
        s.reset();
        assert_eq!(hash(&s), hash(&fresh));
        assert_eq!(s, fresh);
    }
}

#[cfg(test)]
mod drum_link_tests {
    use super::*;

    fn drum() -> Drum {
        Drum::new(DrumConfig::DEFAULT).unwrap()
    }

    fn launched(id: u32, speed: u16) -> Stimulus {
        Stimulus::LaunchedBody {
            launch_id: id,
            impulse_px_s: speed,
            initiator: Some(1),
        }
    }

    #[test]
    fn a_drum_takes_a_launch_at_the_threshold_and_not_below_it() {
        let mut d = drum();
        assert_eq!(d.offer(launched(1, 159)), Offer::Ignored(Ignored::TooWeak));
        assert_eq!(d.state(), DrumState::Armed);
        assert_eq!(
            d.offer(launched(1, 160)),
            Offer::Triggered {
                initiator: Some(1),
                depth: 0
            }
        );
        assert_eq!(d.state(), DrumState::Telegraph);
        // A used drum counts nothing more: one activation.
        assert_eq!(
            d.offer(launched(2, 400)),
            Offer::Ignored(Ignored::AlreadyActivated)
        );
    }

    #[test]
    fn a_drum_refuses_melee_projectiles_and_shoves_and_stays_armed() {
        for stimulus in [
            Stimulus::MeleeHit {
                token: 1,
                player: 0,
                weight: 3,
            },
            Stimulus::Projectile {
                projectile_id: 1,
                owner: Some(1),
                weight: 3,
            },
            Stimulus::Shove {
                shove_id: 1,
                impulse_px_s: 400,
                source: 0,
                credit: Some(0),
            },
        ] {
            let mut d = drum();
            assert_eq!(d.offer(stimulus), Offer::Ignored(Ignored::NotAccepted));
            assert_eq!(d.state(), DrumState::Armed, "{stimulus:?}");
        }
    }

    #[test]
    fn a_blast_and_a_prop_contact_set_a_drum_off_and_carry_their_depth() {
        let mut d = drum();
        assert_eq!(
            d.offer(Stimulus::Blast {
                blast_id: 7,
                owner: Some(0),
                depth: 3,
            }),
            Offer::Triggered {
                initiator: Some(0),
                depth: 3
            }
        );
        assert_eq!(d.trigger_depth(), 3);
        let mut c = drum();
        assert_eq!(
            c.offer(LinkGraph::contact(1, Some(1), 1)),
            Offer::Triggered {
                initiator: Some(1),
                depth: 2
            }
        );
    }

    #[test]
    fn a_contact_at_the_depth_cap_is_refused_and_one_below_it_counts() {
        let at = |depth| Stimulus::PropContact {
            source: 0,
            initiator: None,
            depth,
        };
        let mut d = drum();
        assert_eq!(
            d.offer(at(MAX_CHAIN_DEPTH)),
            Offer::Ignored(Ignored::ChainTooDeep)
        );
        assert_eq!(d.state(), DrumState::Armed);
        assert!(matches!(
            d.offer(at(MAX_CHAIN_DEPTH - 1)),
            Offer::Triggered { depth: 3, .. }
        ));
    }

    #[test]
    fn a_drum_follows_a_chain_lit_charge_once_and_reset_arms_it_again() {
        let mut d = drum();
        assert!(d.adopt_chain(Some(0), 1));
        assert_eq!(
            (d.state(), d.initiator(), d.trigger_depth()),
            (DrumState::Telegraph, Some(0), 1)
        );
        assert!(!d.adopt_chain(Some(1), 3), "the first writer keeps");
        assert_eq!(d.initiator(), Some(0));
        d.detonated();
        assert_eq!(d.state(), DrumState::Detonated);
        assert!(!d.adopt_chain(None, 0));
        d.reset();
        assert_eq!(d, drum(), "reset equals new");
    }

    #[test]
    fn the_drum_state_and_tokens_are_hashed() {
        let words = |d: &Drum| {
            let mut out = Vec::new();
            d.mix_into(&mut |w| out.push(w));
            out
        };
        let mut a = drum();
        let before = words(&a);
        a.offer(launched(5, 200));
        assert_ne!(words(&a), before);
        let mut b = drum();
        b.offer(launched(6, 200));
        assert_ne!(words(&a), words(&b), "the launch id is part of the state");
    }

    #[test]
    fn a_drum_telegraph_under_the_floor_is_rejected() {
        let short = DrumConfig {
            telegraph_ticks: MIN_WARNING_TICKS - 1,
            ..DrumConfig::DEFAULT
        };
        assert!(Drum::new(short).is_err());
        let exact = DrumConfig {
            telegraph_ticks: MIN_WARNING_TICKS,
            ..DrumConfig::DEFAULT
        };
        assert!(Drum::new(exact).is_ok());
    }

    #[test]
    fn a_blast_depth_reaches_a_brace_and_its_link_is_one_deeper() {
        let hanging = IRect::new(784, 452, 64, 32);
        let footprint = IRect::new(784, 484, 64, 96);
        let remnant = IRect::new(784, 548, 64, 32);
        let mut m = FallingMass::new(FallingMassConfig::new(hanging, footprint, remnant)).unwrap();
        assert!(matches!(
            m.break_by_blast(1, Some(0), 2),
            Offer::Triggered { depth: 2, .. }
        ));
        assert_eq!(m.trigger_depth(), 2);
        let contact = LinkGraph::contact(1, m.initiator(), m.trigger_depth());
        assert_eq!(contact.depth(), 3, "NB-4: the link continues the count");
    }

    fn graph(links: &[(u16, u16)]) -> Result<LinkGraph, String> {
        LinkGraph::new(
            links
                .iter()
                .map(|(f, t)| (*f, *t, LinkEmit::Landing))
                .collect(),
        )
    }

    #[test]
    fn the_link_graph_rejects_a_cycle_a_self_link_a_duplicate_and_a_fourth_link() {
        assert!(graph(&[(0, 1), (1, 2)]).is_ok());
        assert!(graph(&[(0, 1), (1, 0)]).unwrap_err().contains("cycle"));
        assert!(
            graph(&[(0, 1), (1, 2), (2, 0)])
                .unwrap_err()
                .contains("cycle")
        );
        assert!(graph(&[(1, 1)]).unwrap_err().contains("itself"));
        assert!(graph(&[(0, 1), (0, 1)]).unwrap_err().contains("duplicate"));
        assert!(
            graph(&[(0, 1), (1, 2), (2, 3), (3, 4)])
                .unwrap_err()
                .contains("at most")
        );
    }

    #[test]
    fn the_link_graph_answers_by_source_and_emit_and_measures_its_chain() {
        let g = LinkGraph::new(vec![
            (0, 1, LinkEmit::Steam),
            (1, 2, LinkEmit::Landing),
            (1, 3, LinkEmit::Blast),
        ])
        .unwrap();
        assert_eq!(g.targets(0, LinkEmit::Steam), [1]);
        assert!(
            g.targets(0, LinkEmit::Landing).is_empty(),
            "the emit matters"
        );
        assert_eq!(g.targets(1, LinkEmit::Landing), [2]);
        assert_eq!(g.targets(1, LinkEmit::Blast), [3]);
        assert_eq!(g.incoming(1), [(0, LinkEmit::Steam)]);
        assert_eq!(g.longest_chain(), 2);
        assert_eq!(LinkGraph::default().longest_chain(), 0);
        assert!(LinkGraph::default().is_empty());
    }

    #[test]
    fn a_contact_depth_saturates_and_the_machines_refuse_it() {
        assert_eq!(LinkGraph::contact(0, None, u8::MAX).depth(), u8::MAX);
        let mut v = Valve::new(ValveConfig {
            contact_weight: 2,
            ..ValveConfig::DEFAULT
        })
        .unwrap();
        assert_eq!(
            v.offer(LinkGraph::contact(0, None, MAX_CHAIN_DEPTH - 1)),
            Offer::Ignored(Ignored::ChainTooDeep)
        );
    }
}

#[cfg(test)]
mod partition_tests {
    use super::*;

    fn melee(token: u32) -> Stimulus {
        Stimulus::MeleeHit {
            token,
            player: 0,
            weight: 1,
        }
    }

    fn shove(id: u32, speed: u16) -> Stimulus {
        Stimulus::Shove {
            shove_id: id,
            impulse_px_s: speed,
            source: 7,
            credit: None,
        }
    }

    fn partition() -> Partition {
        Partition::new(PartitionConfig::DEFAULT).unwrap()
    }

    /// CRL-Q30: three distinct physical hits break it; the first two weaken it.
    #[test]
    fn three_distinct_hits_break_it_and_the_first_two_weaken_it() {
        let mut p = partition();
        assert_eq!(p.state(), PartitionState::Intact);
        assert_eq!(p.offer(melee(1)), Offer::Progress { progress: 1 });
        assert_eq!(p.state(), PartitionState::Weakened);
        assert_eq!(p.offer(melee(2)), Offer::Progress { progress: 2 });
        assert_eq!(p.state(), PartitionState::Weakened);
        assert!(p.is_standing());
        assert_eq!(
            p.offer(melee(3)),
            Offer::Triggered {
                initiator: Some(0),
                depth: 0
            }
        );
        assert_eq!(p.state(), PartitionState::Broken);
        assert!(p.is_broken() && !p.is_standing());
        assert!(p.take_just_broken(), "the remnant lift is due once");
        assert!(!p.take_just_broken());
        // A broken partition counts nothing more.
        assert_eq!(p.offer(melee(4)), Offer::Ignored(Ignored::AlreadyActivated));
        assert_eq!(p.hits(), 3);
    }

    /// One move instance counts once, however many ticks its box overlaps.
    #[test]
    fn one_move_instance_counts_once() {
        let mut p = partition();
        assert_eq!(p.offer(melee(9)), Offer::Progress { progress: 1 });
        for _ in 0..5 {
            assert_eq!(p.offer(melee(9)), Offer::Ignored(Ignored::Duplicate));
        }
        assert_eq!(p.hits(), 1);
        // The per-move weight is not read: a heavy move is still one hit (CD-AD2).
        let heavy = Stimulus::MeleeHit {
            token: 10,
            player: 0,
            weight: 2,
        };
        assert_eq!(p.offer(heavy), Offer::Progress { progress: 2 });
    }

    /// A rush contact breaks it at once; a slower shove does not.
    #[test]
    fn a_rush_shove_breaks_it_at_once_and_a_weak_one_does_not() {
        let mut p = partition();
        assert_eq!(
            p.offer(shove(1, PARTITION_SHOVE_MIN_IMPULSE_PX_S - 1)),
            Offer::Ignored(Ignored::ShoveTooWeak)
        );
        assert_eq!(p.state(), PartitionState::Intact);
        assert!(!p.take_just_broken());
        assert_eq!(
            p.offer(shove(2, 240)),
            Offer::Triggered {
                initiator: None,
                depth: 0
            }
        );
        assert_eq!(p.state(), PartitionState::Broken);
        assert!(p.take_just_broken());
    }

    /// DV-9: a missile, a launched body, a blast and a prop contact do nothing.
    #[test]
    fn nothing_but_a_body_hit_or_a_rush_damages_it() {
        let mut p = partition();
        for stimulus in [
            Stimulus::Projectile {
                projectile_id: 1,
                owner: Some(1),
                weight: 9,
            },
            Stimulus::LaunchedBody {
                launch_id: 1,
                impulse_px_s: 900,
                initiator: None,
            },
            Stimulus::Blast {
                blast_id: 1,
                owner: None,
                depth: 0,
            },
            Stimulus::PropContact {
                source: 0,
                initiator: None,
                depth: 1,
            },
        ] {
            assert_eq!(p.offer(stimulus), Offer::Ignored(Ignored::NotAccepted));
        }
        assert_eq!(
            p.offer(Stimulus::PropContact {
                source: 0,
                initiator: None,
                depth: MAX_CHAIN_DEPTH
            }),
            Offer::Ignored(Ignored::ChainTooDeep)
        );
        assert_eq!((p.state(), p.hits()), (PartitionState::Intact, 0));
    }

    #[test]
    fn forgetting_the_tokens_keeps_the_hits_and_lets_a_token_count_again() {
        let mut p = partition();
        p.offer(melee(1));
        p.forget_hit_tokens();
        assert_eq!(p.hits(), 1);
        assert_eq!(p.offer(melee(1)), Offer::Progress { progress: 2 });
    }

    #[test]
    fn reset_restores_it_and_the_hash_follows_the_state() {
        let hash = |p: &Partition| {
            let mut h = 0u64;
            p.mix_into(&mut |v| h = h.rotate_left(7) ^ v.wrapping_mul(0x9e37_79b9_7f4a_7c15));
            h
        };
        let fresh = partition();
        let mut p = partition();
        p.offer(melee(1));
        assert_ne!(hash(&p), hash(&fresh));
        p.offer(shove(1, 240));
        assert_ne!(hash(&p), hash(&fresh));
        p.reset();
        assert_eq!(hash(&p), hash(&fresh));
        assert_eq!(p.state(), PartitionState::Intact);
        // The setup write: authored pre-broken.
        p.set_broken_for_setup();
        assert!(p.is_broken());
        assert!(!p.take_just_broken(), "a setup break lifts nobody");
    }

    #[test]
    fn the_config_is_bounded() {
        for bad in [
            PartitionConfig {
                break_hits: 1,
                ..PartitionConfig::DEFAULT
            },
            PartitionConfig {
                break_hits: PARTITION_MAX_BREAK_HITS + 1,
                ..PartitionConfig::DEFAULT
            },
            PartitionConfig {
                shove_min_impulse_px_s: 0,
                ..PartitionConfig::DEFAULT
            },
        ] {
            assert!(Partition::new(bad).is_err(), "{bad:?}");
        }
        assert!(PartitionConfig::DEFAULT.validate().is_ok());
        assert_eq!(PartitionConfig::DEFAULT.break_hits, 3);
    }
}
