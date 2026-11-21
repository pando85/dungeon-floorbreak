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
        (Fixture::ValveUntouched, 0x2950_c3bb_7328_5f55),
        (Fixture::ValveMeleeTwoStrikes, 0x153a_6684_e909_6401),
        (Fixture::ValveLaunchedBody, 0xeed7_249c_356e_8e01),
        (Fixture::ValveAuthoredBlast, 0xa814_3d0a_d157_16b4),
        (Fixture::ValveRepeatAfterSpent, 0xd23b_4504_a7ec_7c72),
        (Fixture::ValveWalkInIsNothing, 0x72e2_0910_16e2_f673),
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
            assert!(feed[0].ticks_left.is_some_and(|t| (1..=23).contains(&t)));
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

// ---------------------------------------------------------------------------
// BOIL-B wire and BOIL-C wire: the room's scripted scene pilots.
// ---------------------------------------------------------------------------

/// Scripted **scene pilots** of the Goblin Boiler (integration plan section 10,
/// item 5): a deterministic function of the finished sim state plays the real
/// room on the real combat scene with the live env step ([`crate::combat_scene`]
/// `env`), for solo Carl, solo Donut by swap and two players, at the CF02 seeds
/// 0..8. They are the gate's per-room fixtures that exist before the companion
/// mode (`cf02_free`, CA2-B2): the companion-mode fixtures land in the first
/// BOIL owner step after CA2-B2. The room has no roster of its own yet (BOIL-E),
/// so a plan that needs a goblin brings one (a passive goblin under the weight's
/// footprint).
///
/// A pilot is **not** AUTOPLAY and not an evaluator of fun: it proves the
/// critical path (the exit is reachable and reached in every destruction state
/// the plans produce, no hero is stuck for more than 300 ticks while walking),
/// the exact event ordering of each plan and the replay determinism (each plan
/// replays to the same state hash). It runs in `cargo test`; it is not
/// registered as a CLI fixture name (the CLI route is `--scene f01_boiler`).
pub mod scene_pilots {
    use std::path::Path;

    use crate::boss_scene::Pilot;
    use crate::combat::{CombatEvent, DefeatCause, EnemyAi, PropPhase};
    use crate::combat_scene::{CombatState, EnemySpawn, Fixture};
    use crate::corridor::SceneSim;
    use crate::donut_mana::DonutSetup;
    use crate::env_prop::{MassState, ValveState};
    use crate::f01_boiler;
    use crate::input::RawInput;
    use crate::party::{HeroId, PROVISIONAL_TUNING};
    use crate::party_scene::{PartyFixture, PartyFixtureSim};

    /// Where Carl's jab reaches the valve and the brace (the jab is 16..22 px
    /// ahead of his column).
    pub const VALVE_X: f32 = 460.0;
    pub const BRACE_X: f32 = 740.0;
    /// A passive goblin standing under the weight's footprint (784..848).
    pub const GOBLIN_UNDER_WEIGHT: i32 = 816;
    /// A walking hero that does not move for this many ticks is stuck. The gate
    /// allows at most 300.
    pub const STUCK_LIMIT: u32 = 300;
    /// The run length of every plan (15 s of game time).
    pub const PLAN_TICKS: u64 = 900;

    /// What a plan does with the props.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Plan {
        /// BOIL-01: walk to the exit, touch nothing.
        Direct,
        /// BOIL-02: two jabs on the valve, wait out the steam, walk on.
        Valve,
        /// BOIL-05: two jabs on the brace with a goblin under the footprint.
        Weight,
        /// BOIL-06/07: the valve, then the brace; the room ends broken and
        /// settled (the rubble variant).
        Both,
    }

    /// Who plays.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Who {
        /// Solo Carl (the default start).
        CarlSolo,
        /// Carl plays `plan`, then swaps to Donut, who walks to the exit.
        DonutBySwap,
        /// P1 Carl plays `plan`, P2 Donut walks right.
        TwoPlayer,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Step {
        WalkTo(f32),
        Jab2,
        WaitValveSpent,
        WaitLanded,
        Swap,
        WalkExit,
    }

    fn steps(plan: Plan, swap_before_exit: bool) -> Vec<Step> {
        let mut s = match plan {
            Plan::Direct => Vec::new(),
            Plan::Valve => vec![Step::WalkTo(VALVE_X), Step::Jab2, Step::WaitValveSpent],
            Plan::Weight => vec![Step::WalkTo(BRACE_X), Step::Jab2, Step::WaitLanded],
            Plan::Both => vec![
                Step::WalkTo(VALVE_X),
                Step::Jab2,
                Step::WaitValveSpent,
                Step::WalkTo(BRACE_X),
                Step::Jab2,
                Step::WaitLanded,
            ],
        };
        if swap_before_exit {
            s.push(Step::Swap);
        }
        s.push(Step::WalkExit);
        s
    }

    /// The pilot's own counters; everything else is read from the sim.
    #[derive(Debug, Clone)]
    struct Driver {
        steps: Vec<Step>,
        at: usize,
        timer: u32,
    }

    const WALK: RawInput = RawInput {
        move_x: 1.0,
        ..RawInput::NONE
    };

    impl Driver {
        fn new(steps: Vec<Step>) -> Self {
            Self {
                steps,
                at: 0,
                timer: 0,
            }
        }

        /// Whether the current step is a walk (the stuck metric counts only
        /// those: a jab or a wait stands still on purpose).
        fn walking(&self) -> bool {
            matches!(
                self.steps.get(self.at),
                Some(Step::WalkTo(_) | Step::WalkExit)
            )
        }

        fn advance(&mut self) {
            self.at += 1;
            self.timer = 0;
        }

        /// A jump when a solid ahead rises above the feet (the drum and the
        /// remnant are 32 px, every class clears them).
        fn walk(sim: &PartyFixtureSim, hero: HeroId) -> RawInput {
            let body = sim.scene.world.body(hero);
            let foot = body.y - crate::prototype::HERO_HALF_HEIGHT;
            let blocked = sim.scene.world.solids.iter().any(|b| {
                b.intact
                    && b.rect.left > body.x + 8.0
                    && b.rect.left < body.x + 34.0
                    && b.rect.top() > foot + 1.0
                    && b.rect.bottom < foot + 40.0
            });
            RawInput {
                jump: blocked && body.grounded,
                ..WALK
            }
        }

        fn input(&mut self, sim: &PartyFixtureSim, hero: HeroId) -> RawInput {
            let Some(step) = self.steps.get(self.at).copied() else {
                return RawInput::NONE;
            };
            let x = sim.scene.world.body(hero).x;
            let env = sim.combat.env().expect("the Boiler has props");
            match step {
                Step::WalkTo(target) => {
                    if x >= target {
                        self.advance();
                        return self.input(sim, hero);
                    }
                    Self::walk(sim, hero)
                }
                Step::Jab2 => {
                    self.timer += 1;
                    match self.timer {
                        1 | 31 => RawInput {
                            attack: true,
                            ..RawInput::NONE
                        },
                        46.. => {
                            self.advance();
                            RawInput::NONE
                        }
                        _ => RawInput::NONE,
                    }
                }
                Step::WaitValveSpent => {
                    if env.valve_state(0) == Some(ValveState::Spent) {
                        self.advance();
                    }
                    RawInput::NONE
                }
                Step::WaitLanded => {
                    if env.mass_state(1) == Some(MassState::Landed) {
                        self.advance();
                    }
                    RawInput::NONE
                }
                Step::Swap => {
                    self.advance();
                    RawInput {
                        swap: true,
                        ..RawInput::NONE
                    }
                }
                Step::WalkExit => Self::walk(sim, hero),
            }
        }
    }

    /// What a pilot run saw.
    #[derive(Debug, Clone)]
    pub struct Report {
        /// The tick the exit was reached on.
        pub exit_tick: Option<u64>,
        /// The longest run of walking ticks without moving, per hero.
        pub max_stuck: [u32; 2],
        pub hash: u64,
        pub events: Vec<CombatEvent>,
    }

    impl Report {
        pub fn phases(&self, prop: u16, phase: PropPhase) -> usize {
            self.events
                .iter()
                .filter(|e| {
                    matches!(e, CombatEvent::PropPhase { prop: p, phase: ph, .. }
                        if *p == prop && *ph == phase)
                })
                .count()
        }

        pub fn prop_hits(&self) -> usize {
            self.events
                .iter()
                .filter(|e| matches!(e, CombatEvent::PropHit { .. }))
                .count()
        }

        pub fn hero_hurts(&self) -> usize {
            self.events
                .iter()
                .filter(|e| matches!(e, CombatEvent::HeroHurt { .. }))
                .count()
        }

        pub fn environment_kills(&self) -> Vec<(u32, Option<u32>)> {
            self.events
                .iter()
                .filter_map(|e| match e {
                    CombatEvent::EnemyDefeated {
                        target,
                        cause: DefeatCause::Environment { player, .. },
                        ..
                    } => Some((*target, *player)),
                    _ => None,
                })
                .collect()
        }
    }

    /// Run one plan on the real room for [`PLAN_TICKS`] (or until the exit and
    /// a short tail), at CF02 seed `seed`.
    pub fn run(root: &Path, who: Who, plan: Plan, seed: u64) -> Result<Report, String> {
        let run = f01_boiler::launch(root)?;
        let goblins: &'static [EnemySpawn] = if plan == Plan::Weight {
            Box::leak(Box::new([EnemySpawn {
                x: GOBLIN_UNDER_WEIGHT,
                ai: EnemyAi::Passive,
            }]))
        } else {
            &[]
        };
        let base: &'static Fixture = Box::leak(Box::new(Fixture {
            name: "boiler_pilot",
            summary: "BOIL-B/C wire: a scripted scene pilot of the Goblin Boiler",
            seed,
            carl_x: run.loaded.room.spawn_carl,
            enemies: goblins,
            shore_dx: None,
            boss: None,
            engineers: &[],
            llamas: &[],
            charges: &[],
            script: &[],
            ticks: PLAN_TICKS,
        }));
        let mut scene = SceneSim::new_at(&run.kit, 360, base.carl_x)?;
        let mut combat = CombatState::new(base, &run.kit, &scene)?
            .with_donut(DonutSetup::unlocked(8))
            .with_demo(run.rules)
            .with_cf02()
            .with_cf02_seed(Some(seed))
            .with_env(f01_boiler::env_runtime(&run)?);
        combat.sync_env(&mut scene);
        let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            base,
            two_player: who == Who::TwoPlayer,
            tuning: PROVISIONAL_TUNING,
            script: &[],
            pilot: Pilot::None,
            donut: DonutSetup::unlocked(8),
        }));
        let mut sim = PartyFixtureSim::from_parts(scene, combat, fixture);
        let mut carl = Driver::new(steps(plan, who == Who::DonutBySwap));
        let mut donut = Driver::new(vec![Step::WalkExit]);
        let mut last_x = [sim.scene.world.hero.x, sim.scene.world.donut_body.x];
        let mut stuck = [0u32; 2];
        let mut max_stuck = [0u32; 2];
        let mut exit_tick = None;
        let mut swapped = false;
        for _ in 0..PLAN_TICKS {
            // Who the plan's first driver steers: Carl until the swap, then
            // Donut (solo), or Carl all the time (two players).
            let (slots, walking) = match who {
                Who::CarlSolo => (
                    [carl.input(&sim, HeroId::Carl), RawInput::NONE],
                    [carl.walking(), false],
                ),
                Who::DonutBySwap if !swapped => {
                    let input = carl.input(&sim, HeroId::Carl);
                    swapped = input.swap;
                    ([input, RawInput::NONE], [carl.walking(), false])
                }
                Who::DonutBySwap => {
                    let input = donut.input(&sim, HeroId::Donut);
                    ([input, RawInput::NONE], [false, true])
                }
                Who::TwoPlayer => {
                    // P2 follows P1 at about 90 px behind (the scroll camera
                    // holds the lead player back when the partner lags); she
                    // never gets ahead of him, so the exit is his to reach.
                    let carl_x = sim.scene.world.hero.x;
                    let donut_x = sim.scene.world.donut_body.x;
                    let follow = donut_x < carl_x - 90.0;
                    let p2 = if follow {
                        Driver::walk(&sim, HeroId::Donut)
                    } else {
                        RawInput::NONE
                    };
                    (
                        [carl.input(&sim, HeroId::Carl), p2],
                        [carl.walking(), follow],
                    )
                }
            };
            sim.step_slots(slots);
            for (i, id) in HeroId::ALL.into_iter().enumerate() {
                let x = sim.scene.world.body(id).x;
                if walking[i] && (x - last_x[i]).abs() < 0.1 {
                    stuck[i] += 1;
                } else {
                    stuck[i] = 0;
                }
                max_stuck[i] = max_stuck[i].max(stuck[i]);
                last_x[i] = x;
            }
            if exit_tick.is_none() && sim.scene.world.complete {
                exit_tick = Some(sim.scene.world.tick);
            }
            // The exit is the last step of every plan.
            if exit_tick.is_some() {
                break;
            }
        }
        Ok(Report {
            exit_tick,
            max_stuck,
            hash: sim.combat.state_hash(&sim.scene),
            events: sim.combat.events.clone(),
        })
    }
}

#[cfg(test)]
mod scene_pilot_tests {
    use super::scene_pilots::*;
    use crate::combat::PropPhase;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn go(who: Who, plan: Plan, seed: u64) -> Report {
        run(&root(), who, plan, seed).expect("the pilot runs")
    }

    /// BOIL-01-DIRECT: nothing touched, the exit is reached, nothing happens to
    /// the props or the heroes, for solo Carl, Donut by swap and two players.
    #[test]
    fn the_direct_route_reaches_the_exit_with_every_prop_untouched() {
        for who in [Who::CarlSolo, Who::DonutBySwap, Who::TwoPlayer] {
            for seed in 0..8 {
                let r = go(who, Plan::Direct, seed);
                let tick = r
                    .exit_tick
                    .unwrap_or_else(|| panic!("{who:?} seed {seed}: no exit"));
                assert!(tick < 600, "{who:?} seed {seed}: reached on {tick}");
                assert_eq!(r.prop_hits(), 0, "{who:?}");
                assert_eq!(r.hero_hurts(), 0, "{who:?}");
                assert!(
                    r.max_stuck.iter().all(|s| *s <= STUCK_LIMIT),
                    "{who:?} seed {seed}: {:?}",
                    r.max_stuck
                );
            }
        }
    }

    /// BOIL-02-VALVE-MELEE: one warning, one vent, one spend; nobody is hurt
    /// because the pilot waits the burst out; the exit is reached.
    #[test]
    fn the_valve_plan_vents_once_and_the_exit_is_reached() {
        for seed in 0..8 {
            let r = go(Who::CarlSolo, Plan::Valve, seed);
            assert_eq!(r.phases(0, PropPhase::Warning), 1, "seed {seed}");
            assert_eq!(r.phases(0, PropPhase::Venting), 1);
            assert_eq!(r.phases(0, PropPhase::Spent), 1);
            assert_eq!(r.hero_hurts(), 0);
            assert!(r.exit_tick.is_some(), "seed {seed}");
            assert!(r.max_stuck[0] <= STUCK_LIMIT);
        }
    }

    /// BOIL-05-WEIGHT: the brace releases, the weight lands once, the goblin
    /// under the footprint dies credited to Carl, and the exit is reached over
    /// the rubble.
    #[test]
    fn the_weight_plan_kills_the_goblin_under_it_once_and_the_rubble_is_crossed() {
        for seed in 0..8 {
            let r = go(Who::CarlSolo, Plan::Weight, seed);
            assert_eq!(r.phases(1, PropPhase::Released), 1);
            assert_eq!(r.phases(1, PropPhase::Landed), 1);
            assert_eq!(r.environment_kills(), [(0, Some(0))], "seed {seed}");
            assert_eq!(r.hero_hurts(), 0, "the pilot never stands under it");
            assert!(r.exit_tick.is_some(), "seed {seed}");
            assert!(r.max_stuck[0] <= STUCK_LIMIT);
        }
    }

    /// BOIL-06/07: both props used (broken and settled), the exit is still
    /// reached by Carl, by Donut after a swap and with two players.
    #[test]
    fn the_room_stays_walkable_for_every_hero_with_both_props_used() {
        for who in [Who::CarlSolo, Who::DonutBySwap, Who::TwoPlayer] {
            for seed in 0..8 {
                let r = go(who, Plan::Both, seed);
                assert_eq!(r.phases(0, PropPhase::Spent), 1, "{who:?} seed {seed}");
                assert_eq!(r.phases(1, PropPhase::Landed), 1, "{who:?} seed {seed}");
                assert!(r.exit_tick.is_some(), "{who:?} seed {seed}: no exit");
                assert!(
                    r.max_stuck.iter().all(|s| *s <= STUCK_LIMIT),
                    "{who:?} seed {seed}: {:?}",
                    r.max_stuck
                );
            }
        }
    }

    #[test]
    fn every_pilot_replays_to_the_same_hash_and_events() {
        for (who, plan) in [
            (Who::CarlSolo, Plan::Direct),
            (Who::CarlSolo, Plan::Valve),
            (Who::CarlSolo, Plan::Weight),
            (Who::CarlSolo, Plan::Both),
            (Who::DonutBySwap, Plan::Both),
            (Who::TwoPlayer, Plan::Both),
        ] {
            let a = go(who, plan, 3);
            let b = go(who, plan, 3);
            assert_eq!(a.hash, b.hash, "{who:?} {plan:?}");
            assert_eq!(a.events, b.events, "{who:?} {plan:?}");
        }
    }

    /// The final hashes are pinned: a behaviour edit updates them on purpose.
    #[test]
    fn the_pilot_hashes_are_pinned() {
        let plans = [Plan::Direct, Plan::Valve, Plan::Weight, Plan::Both];
        let got: Vec<u64> = plans
            .iter()
            .map(|p| go(Who::CarlSolo, *p, 0).hash)
            .collect();
        assert_eq!(got, PINNED, "pins: {got:#018x?}");
    }

    /// Solo Carl at CF02 seed 0: direct, valve, weight, both.
    const PINNED: [u64; 4] = [
        0x66c5_27ba_bbe8_8786,
        0x0c8c_baed_23ea_b1b6,
        0x9270_8c31_daad_cf8a,
        0x1634_84e0_9bdd_beaf,
    ];
}
