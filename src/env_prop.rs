//! BOIL-B core: the stimulus a world prop can receive, and the pressure valve
//! state machine `intact -> warning -> venting -> spent`. Pure, Bevy-free,
//! integer math. (BOIL-C core adds the falling mass to this module later.)
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
