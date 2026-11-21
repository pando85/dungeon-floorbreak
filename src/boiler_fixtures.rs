//! BOIL-A/B core: headless fixtures of the Goblin Boiler's valve and steam.
//! Bevy-free and **not wired into any scene**; nothing here runs in the game.
//!
//! The rig loads the real room and sidecar ([`crate::encounter::load`]), builds
//! the valve ([`crate::env_prop::Valve`]) with the sidecar's tunables and the
//! steam volume ([`crate::env_hazard::HazardVolume`]) from the sidecar's
//! hazard, and runs scripted ticks in the contract order of the future wire
//! step: stimuli, valve step, hazard step, contact resolution. Each fixture is
//! deterministic (integer ticks, no RNG, no wall-clock), replays to the same
//! state hash, and has its final hash pinned in the tests.
//!
//! What this proves: the valve and the hazard window agree tick by tick, one
//! activation, one hit per victim, the initiator is kept, and a restart
//! restores the first state. What it does not prove: real contacts (the
//! launched-body and melee overlaps come from the combat scene), the falling
//! weight (BOIL-C), the drum chain or any rendering.

use std::path::Path;

use crate::camera_lab::room::IRect;
use crate::encounter::{BOILER_ENCOUNTER_PATH, Loaded, load};
use crate::env_hazard::{HazardClass, HazardEdge, HazardHit, HazardSet, NavRect, Phase, Victim};
use crate::env_prop::{Offer, Stimulus, Valve, ValveEdge, ValveState};

/// What the rig logged on one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Logged {
    Offered { stimulus: Stimulus, result: Offer },
    Valve(ValveEdge),
    Steam(HazardEdge),
    Hit(HazardHit),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub tick: u64,
    pub event: Logged,
}

#[derive(Debug)]
pub enum RigError {
    Load(String),
    Missing(&'static str),
}

pub struct BoilerRig {
    loaded: Loaded,
    valve: Valve,
    hazards: HazardSet,
    steam_id: u16,
    tick: u64,
    pub log: Vec<Entry>,
}

impl BoilerRig {
    pub fn new(root: &Path) -> Result<Self, RigError> {
        let loaded = load(root, BOILER_ENCOUNTER_PATH).map_err(|errs| {
            RigError::Load(
                errs.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })?;
        let (valve, hazards, steam_id) = Self::build(&loaded)?;
        Ok(Self {
            loaded,
            valve,
            hazards,
            steam_id,
            tick: 0,
            log: Vec::new(),
        })
    }

    fn build(loaded: &Loaded) -> Result<(Valve, HazardSet, u16), RigError> {
        let e = &loaded.encounter;
        let valve_idx = e
            .prop_index("valve")
            .ok_or(RigError::Missing("valve prop"))?;
        let cfg = e.props[valve_idx]
            .valve
            .ok_or(RigError::Missing("valve config"))?;
        let steam = e
            .owned_hazard(valve_idx, HazardClass::Steam)
            .ok_or(RigError::Missing("steam hazard"))?;
        let valve = Valve::new(cfg).map_err(RigError::Load)?;
        let hazards = e
            .hazard_set()
            .map_err(|err| RigError::Load(err.to_string()))?;
        Ok((valve, hazards, steam.spec.id))
    }

    pub fn loaded(&self) -> &Loaded {
        &self.loaded
    }
    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn valve(&self) -> &Valve {
        &self.valve
    }
    pub fn hazards(&self) -> &HazardSet {
        &self.hazards
    }
    pub fn steam_id(&self) -> u16 {
        self.steam_id
    }

    /// The nav feed of this tick (what CA2-C would read).
    pub fn nav_feed(&self) -> Vec<NavRect> {
        self.hazards.nav_feed()
    }

    /// Restart the attempt: the first state again.
    pub fn reset(&mut self) {
        let (valve, hazards, _) = Self::build(&self.loaded).expect("built once already");
        self.valve = valve;
        self.hazards = hazards;
        self.tick = 0;
        self.log.clear();
    }

    /// One fixed tick: stimuli, valve step, hazard step, contacts.
    pub fn run_tick(&mut self, stimuli: &[Stimulus], bodies: &[(Victim, IRect)]) {
        let now = self.tick;
        for stimulus in stimuli {
            let result = self.valve.offer(*stimulus);
            self.log.push(Entry {
                tick: now,
                event: Logged::Offered {
                    stimulus: *stimulus,
                    result,
                },
            });
            if let Offer::Triggered { initiator, .. } = result {
                let started = self.hazards.trigger(self.steam_id, initiator);
                assert!(started, "the steam starts with the valve, exactly once");
            }
        }
        if let Some(edge) = self.valve.step() {
            self.log.push(Entry {
                tick: now,
                event: Logged::Valve(edge),
            });
        }
        for (_, edge) in self.hazards.step() {
            self.log.push(Entry {
                tick: now,
                event: Logged::Steam(edge),
            });
        }
        for hit in self.hazards.resolve(bodies) {
            self.log.push(Entry {
                tick: now,
                event: Logged::Hit(hit),
            });
        }
        self.assert_in_agreement();
        self.tick += 1;
    }

    /// The valve and its steam window are one timeline: warning = telegraph,
    /// venting = active.
    fn assert_in_agreement(&self) {
        let steam = self.hazards.get(self.steam_id).expect("steam volume");
        let ok = matches!(
            (self.valve.state(), steam.phase()),
            (ValveState::Intact, Phase::Idle)
                | (ValveState::Warning, Phase::Telegraph)
                | (ValveState::Venting, Phase::Active)
                | (ValveState::Spent, Phase::Spent)
        );
        assert!(
            ok,
            "valve {:?} and steam {:?} disagree at tick {}",
            self.valve.state(),
            steam.phase(),
            self.tick
        );
    }

    /// FNV-1a over the whole state and the log.
    pub fn state_hash(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325_u64;
        let mut mix = |x: u64| {
            h ^= x;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        };
        mix(self.tick);
        self.valve.mix_into(&mut mix);
        self.hazards.mix_into(&mut mix);
        mix(self.log.len() as u64);
        for entry in &self.log {
            mix(entry.tick);
            match entry.event {
                Logged::Offered { stimulus, result } => {
                    mix(1);
                    mix(stimulus.kind() as u64);
                    mix(match result {
                        Offer::Ignored(why) => 10 + why as u64,
                        Offer::Progress { progress } => 100 + u64::from(progress),
                        Offer::Triggered { .. } => 1000,
                    });
                }
                Logged::Valve(edge) => {
                    mix(2);
                    mix(edge as u64);
                }
                Logged::Steam(edge) => {
                    mix(3);
                    mix(edge as u64);
                }
                Logged::Hit(hit) => {
                    mix(4);
                    mix(u64::from(hit.victim.id));
                    mix(u64::from(hit.activation));
                    mix(hit.initiator.map_or(0, |p| u64::from(p) + 1));
                }
            }
        }
        h
    }

    pub fn hits(&self) -> Vec<(u64, HazardHit)> {
        self.log
            .iter()
            .filter_map(|e| match e.event {
                Logged::Hit(h) => Some((e.tick, h)),
                _ => None,
            })
            .collect()
    }

    pub fn offers(&self) -> Vec<(u64, Offer)> {
        self.log
            .iter()
            .filter_map(|e| match e.event {
                Logged::Offered { result, .. } => Some((e.tick, result)),
                _ => None,
            })
            .collect()
    }
}

/// The scripted fixtures. Names are stable ids for tests and the later
/// recorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fixture {
    /// BOIL-01 machine part: nothing touches the valve.
    ValveUntouched,
    /// BOIL-02: two distinct light strikes, one repeated token.
    ValveMeleeTwoStrikes,
    /// BOIL-03: a slow launch is ignored, a fast one counts once.
    ValveLaunchedBody,
    /// An authored blast with no owner sets it off: nobody is credited.
    ValveAuthoredBlast,
    /// BOIL-07/12 machine part: strikes after the valve is spent do nothing.
    ValveRepeatAfterSpent,
    /// A prop contact (an enemy walking into the valve) is no stimulus.
    ValveWalkInIsNothing,
}

impl Fixture {
    pub const ALL: [Self; 6] = [
        Self::ValveUntouched,
        Self::ValveMeleeTwoStrikes,
        Self::ValveLaunchedBody,
        Self::ValveAuthoredBlast,
        Self::ValveRepeatAfterSpent,
        Self::ValveWalkInIsNothing,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::ValveUntouched => "boil_valve_untouched",
            Self::ValveMeleeTwoStrikes => "boil_valve_melee_two_strikes",
            Self::ValveLaunchedBody => "boil_valve_launched_body",
            Self::ValveAuthoredBlast => "boil_valve_authored_blast",
            Self::ValveRepeatAfterSpent => "boil_valve_repeat_after_spent",
            Self::ValveWalkInIsNothing => "boil_valve_walk_in_is_nothing",
        }
    }

    pub const fn ticks(self) -> u64 {
        match self {
            Self::ValveRepeatAfterSpent => 260,
            _ => 200,
        }
    }

    fn stimuli(self, tick: u64) -> Vec<Stimulus> {
        let jab = |token, player| Stimulus::MeleeHit {
            token,
            player,
            weight: 1,
        };
        let launch = |launch_id, impulse_px_s| Stimulus::LaunchedBody {
            launch_id,
            impulse_px_s,
            initiator: Some(0),
        };
        match (self, tick) {
            (Self::ValveMeleeTwoStrikes, 10) => vec![jab(1, 0)],
            (Self::ValveMeleeTwoStrikes, 20) => vec![jab(1, 0)],
            (Self::ValveMeleeTwoStrikes, 30) => vec![jab(2, 0)],
            (Self::ValveLaunchedBody, 5) => vec![launch(1, 150)],
            (Self::ValveLaunchedBody, 10) => vec![launch(2, 240)],
            (Self::ValveLaunchedBody, 12) => vec![launch(2, 240)],
            (Self::ValveAuthoredBlast, 8) => vec![Stimulus::Blast {
                blast_id: 1,
                owner: None,
            }],
            (Self::ValveRepeatAfterSpent, 5) => vec![Stimulus::Blast {
                blast_id: 1,
                owner: Some(0),
            }],
            (Self::ValveRepeatAfterSpent, 210) => vec![jab(7, 0)],
            (Self::ValveRepeatAfterSpent, 220) => vec![jab(8, 0)],
            (Self::ValveWalkInIsNothing, 20) | (Self::ValveWalkInIsNothing, 40) => {
                vec![Stimulus::PropContact {
                    source: 7,
                    initiator: None,
                    depth: 1,
                }]
            }
            _ => Vec::new(),
        }
    }

    /// The bodies of this tick: a goblin standing in the steam lane the whole
    /// time and Carl stepping into it on tick 60 (after the vent has opened in
    /// the melee fixture) and staying.
    fn bodies(self, tick: u64) -> Vec<(Victim, IRect)> {
        let goblin = (Victim::enemy(1), IRect::new(700, 528, 24, 52));
        let carl_x = if tick < 60 { 560 } else { 700 };
        let carl = (Victim::hero(0), IRect::new(carl_x, 528, 24, 52));
        vec![goblin, carl]
    }

    pub fn run(self, root: &Path) -> Result<BoilerRig, RigError> {
        let mut rig = BoilerRig::new(root)?;
        for tick in 0..self.ticks() {
            let stimuli = self.stimuli(tick);
            let bodies = self.bodies(tick);
            rig.run_tick(&stimuli, &bodies);
        }
        Ok(rig)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env_prop::Ignored;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn run(f: Fixture) -> BoilerRig {
        f.run(root()).expect("fixture runs")
    }

    /// Final state hashes, pinned. A change here is a change of behaviour.
    const GOLDENS: [(Fixture, u64); 6] = [
        (Fixture::ValveUntouched, 0xfbb1_1746_af7c_827f),
        (Fixture::ValveMeleeTwoStrikes, 0x5950_fe14_6654_1093),
        (Fixture::ValveLaunchedBody, 0x44f4_b72b_c8ab_6ac7),
        (Fixture::ValveAuthoredBlast, 0xa323_6361_6dc6_0806),
        (Fixture::ValveRepeatAfterSpent, 0xba27_4e94_c3dc_153c),
        (Fixture::ValveWalkInIsNothing, 0x6f6c_fa9c_9f70_f991),
    ];

    #[test]
    fn every_fixture_replays_to_the_same_hash_and_matches_its_golden() {
        for (fixture, golden) in GOLDENS {
            let a = run(fixture);
            let b = run(fixture);
            assert_eq!(a.state_hash(), b.state_hash(), "{} replays", fixture.name());
            assert_eq!(a.log, b.log);
            assert_eq!(
                a.state_hash(),
                golden,
                "{}: {:#018x}",
                fixture.name(),
                a.state_hash()
            );
        }
        assert_eq!(GOLDENS.len(), Fixture::ALL.len());
        let names: std::collections::BTreeSet<_> = Fixture::ALL.iter().map(|f| f.name()).collect();
        assert_eq!(names.len(), Fixture::ALL.len());
    }

    #[test]
    fn the_rig_reads_the_real_data() {
        let rig = BoilerRig::new(root()).unwrap();
        let cfg = rig.valve().config();
        assert_eq!(
            (cfg.trigger_weight, cfg.warning_ticks, cfg.vent_ticks),
            (2, 24, 72)
        );
        assert_eq!(rig.loaded().encounter.id, "F01-BOILER-ENC");
        assert_eq!(rig.valve().state(), ValveState::Intact);
        assert!(rig.nav_feed().is_empty());
    }

    #[test]
    fn untouched_means_nothing_happens() {
        let rig = run(Fixture::ValveUntouched);
        assert!(rig.log.is_empty());
        assert_eq!(rig.valve().state(), ValveState::Intact);
    }

    #[test]
    fn melee_two_strikes_one_warning_one_vent_one_spend_one_hit_each() {
        let rig = run(Fixture::ValveMeleeTwoStrikes);
        let offers: Vec<Offer> = rig.offers().into_iter().map(|(_, o)| o).collect();
        assert_eq!(
            offers,
            [
                Offer::Progress { progress: 1 },
                Offer::Ignored(Ignored::Duplicate),
                Offer::Triggered {
                    initiator: Some(0),
                    depth: 0
                },
            ]
        );
        // Triggered on tick 30: the warning runs ticks 30..=53, the vent opens
        // on tick 53's step and runs 72 ticks, then the valve is spent.
        let edges: Vec<(u64, &str)> = rig
            .log
            .iter()
            .filter_map(|e| match e.event {
                Logged::Valve(ValveEdge::VentStarted) => Some((e.tick, "vent")),
                Logged::Valve(ValveEdge::Spent) => Some((e.tick, "spent")),
                _ => None,
            })
            .collect();
        assert_eq!(edges, [(53, "vent"), (125, "spent")]);
        let steam: Vec<(u64, HazardEdge)> = rig
            .log
            .iter()
            .filter_map(|e| match e.event {
                Logged::Steam(edge) => Some((e.tick, edge)),
                _ => None,
            })
            .collect();
        assert_eq!(
            steam,
            [(53, HazardEdge::ActiveStarted), (125, HazardEdge::Ended)]
        );
        // The goblin in the lane is hit on the first active tick, Carl on the
        // tick he steps in; each exactly once, both credited to player 0.
        let hits = rig.hits();
        assert_eq!(hits.len(), 2, "{hits:?}");
        assert_eq!((hits[0].0, hits[0].1.victim), (53, Victim::enemy(1)));
        assert_eq!((hits[1].0, hits[1].1.victim), (60, Victim::hero(0)));
        assert!(hits.iter().all(|(_, h)| h.initiator == Some(0)
            && h.activation == 1
            && h.damage == 1
            && h.push_x == 120
            && !h.evadable()
            && h.class == HazardClass::Steam));
        assert_eq!(rig.valve().state(), ValveState::Spent);
    }

    #[test]
    fn nobody_is_hurt_during_the_warning() {
        let mut rig = BoilerRig::new(root()).unwrap();
        let lane = [(Victim::hero(0), IRect::new(700, 528, 24, 52))];
        rig.run_tick(
            &[Stimulus::Blast {
                blast_id: 1,
                owner: Some(0),
            }],
            &lane,
        );
        for _ in 0..22 {
            rig.run_tick(&[], &lane);
            assert!(rig.hits().is_empty());
            let feed = rig.nav_feed();
            assert_eq!(feed.len(), 1);
            assert_eq!(feed[0].phase, crate::env_hazard::NavPhase::Telegraph);
            assert!(feed[0].hurts_heroes);
        }
        rig.run_tick(&[], &lane);
        assert_eq!(rig.hits().len(), 1, "the first active tick hurts");
        assert_eq!(rig.nav_feed()[0].phase, crate::env_hazard::NavPhase::Active);
        assert_eq!(rig.nav_feed()[0].rect, IRect::new(688, 484, 80, 96));
    }

    #[test]
    fn a_slow_launch_is_ignored_and_a_fast_one_counts_once() {
        let rig = run(Fixture::ValveLaunchedBody);
        let offers: Vec<Offer> = rig.offers().into_iter().map(|(_, o)| o).collect();
        assert_eq!(
            offers,
            [
                Offer::Ignored(Ignored::TooWeak),
                Offer::Triggered {
                    initiator: Some(0),
                    depth: 0
                },
                Offer::Ignored(Ignored::AlreadyActivated),
            ]
        );
        assert_eq!(rig.hits().len(), 2);
    }

    #[test]
    fn an_authored_blast_credits_nobody() {
        let rig = run(Fixture::ValveAuthoredBlast);
        let hits = rig.hits();
        assert_eq!(hits.len(), 2);
        // World hazards hurt every hero by their own data, credited to nobody.
        assert!(hits.iter().all(|(_, h)| h.initiator.is_none()));
        assert!(
            hits.iter()
                .any(|(_, h)| h.victim.team == crate::env_hazard::Team::Hero)
        );
    }

    #[test]
    fn strikes_after_the_spend_change_nothing_and_do_not_vent_again() {
        let rig = run(Fixture::ValveRepeatAfterSpent);
        let late: Vec<Offer> = rig
            .offers()
            .into_iter()
            .filter(|(t, _)| *t >= 200)
            .map(|(_, o)| o)
            .collect();
        assert_eq!(late, [Offer::Ignored(Ignored::AlreadyActivated); 2]);
        let vents = rig
            .log
            .iter()
            .filter(|e| matches!(e.event, Logged::Valve(ValveEdge::VentStarted)))
            .count();
        assert_eq!(vents, 1);
        assert_eq!(rig.hits().len(), 2, "one hit per victim, ever");
        assert_eq!(rig.hazards().get(rig.steam_id()).unwrap().activations(), 1);
    }

    #[test]
    fn walking_into_the_valve_is_no_stimulus() {
        let rig = run(Fixture::ValveWalkInIsNothing);
        let offers: Vec<Offer> = rig.offers().into_iter().map(|(_, o)| o).collect();
        assert_eq!(offers, [Offer::Ignored(Ignored::NotAccepted); 2]);
        assert_eq!(rig.valve().state(), ValveState::Intact);
        assert!(rig.hits().is_empty());
    }

    #[test]
    fn a_restart_restores_the_first_state_exactly() {
        let fresh = BoilerRig::new(root()).unwrap();
        let mut rig = run(Fixture::ValveMeleeTwoStrikes);
        assert_ne!(rig.state_hash(), fresh.state_hash());
        rig.reset();
        assert_eq!(rig.state_hash(), fresh.state_hash());
        // And it runs the same attempt again to the same final state.
        for tick in 0..Fixture::ValveMeleeTwoStrikes.ticks() {
            let s = Fixture::ValveMeleeTwoStrikes.stimuli(tick);
            let b = Fixture::ValveMeleeTwoStrikes.bodies(tick);
            rig.run_tick(&s, &b);
        }
        assert_eq!(
            rig.state_hash(),
            run(Fixture::ValveMeleeTwoStrikes).state_hash()
        );
    }

    #[test]
    fn a_death_in_the_steam_and_a_retry_keep_no_state() {
        // Mid-vent restart (BOIL-08 machine part): the volume and the valve
        // are fresh, so the same strikes warn and vent again from tick 0.
        let mut rig = BoilerRig::new(root()).unwrap();
        let lane = [(Victim::hero(0), IRect::new(700, 528, 24, 52))];
        rig.run_tick(
            &[Stimulus::Blast {
                blast_id: 1,
                owner: Some(0),
            }],
            &lane,
        );
        for _ in 0..40 {
            rig.run_tick(&[], &lane);
        }
        assert_eq!(rig.valve().state(), ValveState::Venting);
        rig.reset();
        assert_eq!(rig.valve().state(), ValveState::Intact);
        assert!(rig.hits().is_empty());
        rig.run_tick(
            &[Stimulus::Blast {
                blast_id: 1,
                owner: Some(0),
            }],
            &lane,
        );
        assert_eq!(rig.valve().state(), ValveState::Warning);
    }
}
