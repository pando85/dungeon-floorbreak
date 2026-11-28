//! TROG-A and TROG-B scenario fixtures (`docs/01-design/floor-01-troglodyte-integration.md`
//! section 7.1): the troglodytes, `trog_run` and `trog_strength` driven by scripted party inputs
//! on the launched room, deterministic, seeded and fixed-tick. Every fixture runs twice and must end on an identical
//! state hash. Bevy-free.
//!
//! The scripts are **closed loops on the state** (the heroes' columns and the troglodytes'
//! phases), never tick counts. Where a plan row names an exact pivot (F01's "Carl at L - 48,
//! facing it", F02's "Donut planted at 432") the fixture writes that one position at setup, once,
//! exactly as the llama sheet's F11 exempts setup writes; every other step is an input. The
//! uncontrolled hero of an isolated fixture is **parked** in the protected spawn (x 80) each tick
//! so no companion or follower can add a second fighter (the evaluator's `isolated` variant).

use crate::carl_prog::CarlProgFlags;
use crate::combat::CombatEvent;
use crate::combat_scene::CombatState;
use crate::combat_scene::trog::TrogEnemy;
use crate::corridor::SceneSim;
use crate::donut_prog::DonutProgFlags;
use crate::f01_late::Loadout;
use crate::f01_trog::{self, TrogRoom, TrogRun};
use crate::input::{RawInput, SlotInputs};
use crate::party::{HeroId, PROVISIONAL_TUNING, PadId, PartyState, PartyTuning};
use crate::prototype::HERO_HALF_HEIGHT;
use crate::rest_fixtures::{BOT, FREE, Party};
use crate::trog::{State, Trog, TrogAct};
use std::path::Path;

/// The fixed-tick budget of a room fixture.
pub const MAX_TICKS: u64 = 6000;
/// The seed of the fixtures' `--cf02-seed`.
pub const SEED: u64 = 0x0F01_7206_5EED;
/// Where an isolated fixture parks the uncontrolled hero (inside `protected_spawn`).
pub const PARK_X: f32 = 80.0;

/// A launched troglodyte room with a party, stepped the way the interactive scene steps it.
pub struct TrogSim {
    pub room: TrogRoom,
    pub run: TrogRun,
    pub scene: SceneSim,
    pub combat: CombatState,
    pub party: PartyState,
    /// The uncontrolled hero parked in the protected spawn each tick (`None`: free).
    pub parked: Option<HeroId>,
    pub solo_donut: bool,
    /// What the scene looked like at each `Tell` (the view and the target), in log order.
    pub tell_log: Vec<TellFact>,
    /// The same for each **proximity** `Wake` (TROG-AD21): the view the slot step read.
    pub wake_log: Vec<TellFact>,
    /// Ticks on which a hero stood under two live troglodyte hitboxes (F13; counted by the
    /// fixtures that check it).
    pub double_live: u32,
}

/// One `Tell` as the scene saw it: the enemy's idle hurtbox x range, the camera view the slot
/// step read, and the target hero's pivot x (TROG-F07's in-view and protected-spawn rules).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TellFact {
    pub tick: u64,
    pub enemy: usize,
    pub body: (f32, f32),
    pub view: (f32, f32),
    pub target_x: f32,
    /// The target's hurtbox can meet the species' attack band in y (TROG-AD17); `true` for a
    /// wake.
    pub meets_band: bool,
}

/// How a fixture or an evaluator run builds its room: the party seat, the setup writes (the
/// per-enemy phase, the reflex) and the flags.
#[derive(Debug, Clone)]
pub struct SimSetup {
    pub room: TrogRoom,
    pub party: Party,
    pub parked: Option<HeroId>,
    pub carl: CarlProgFlags,
    pub donut: DonutProgFlags,
    /// Setup write (TROG-AD37): each roster entry's `delay_ticks`, the extra wait after a wake
    /// (the evaluator's phase variant P0..P2). Empty: the sidecar's own values.
    pub delays: Vec<u32>,
    /// DM-B reflex profile of the party tuning (`None`: the shipped one).
    pub reflex: Option<crate::agility::ReflexProfile>,
}

impl SimSetup {
    /// The room under the Story flags and the shipped reflex.
    pub fn story(room: TrogRoom, party: Party, parked: Option<HeroId>) -> Self {
        let (carl, donut) = Loadout::Story.flags();
        Self {
            room,
            party,
            parked,
            carl,
            donut,
            delays: Vec::new(),
            reflex: None,
        }
    }
}

impl TrogSim {
    /// The run room under the Story flags.
    pub fn new(root: &Path, party: Party, parked: Option<HeroId>) -> Result<Self, String> {
        Self::for_room(root, TrogRoom::Run, party, parked)
    }

    /// A troglodyte room under the Story flags.
    pub fn for_room(
        root: &Path,
        room: TrogRoom,
        party: Party,
        parked: Option<HeroId>,
    ) -> Result<Self, String> {
        Self::build(root, &SimSetup::story(room, party, parked))
    }

    pub fn with_flags(
        root: &Path,
        party: Party,
        parked: Option<HeroId>,
        carl: CarlProgFlags,
        donut: DonutProgFlags,
    ) -> Result<Self, String> {
        let mut setup = SimSetup::story(TrogRoom::Run, party, parked);
        setup.carl = carl;
        setup.donut = donut;
        Self::build(root, &setup)
    }

    /// Launch the room and build the party around it.
    pub fn build(root: &Path, setup: &SimSetup) -> Result<Self, String> {
        let SimSetup {
            room,
            party,
            parked,
            carl,
            donut,
            ..
        } = *setup;
        let mut run = f01_trog::launch(root, room)?;
        for (entry, delay) in run.loaded.encounter.roster.iter_mut().zip(&setup.delays) {
            entry.delay_ticks = *delay;
        }
        let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x)?;
        let combat = f01_trog::combat_for(&run, &mut scene, carl, donut, Some(SEED))?;
        let uncontrolled = match party {
            Party::Solo { uncontrolled } => uncontrolled,
            Party::TwoPlayer => BOT,
        };
        let mut tuning = PartyTuning {
            uncontrolled_hero: uncontrolled,
            ..PROVISIONAL_TUNING
        }
        .with_cf02_swap();
        if let Some(reflex) = setup.reflex {
            tuning.donut_reflex = reflex;
        }
        let mut state = PartyState::new(tuning);
        if party == Party::TwoPlayer {
            state.pad_connected(PadId(1));
            state.pad_start_pressed(PadId(2));
        }
        Ok(Self {
            room,
            run,
            scene,
            combat,
            party: state,
            parked,
            solo_donut: false,
            tell_log: Vec::new(),
            wake_log: Vec::new(),
            double_live: 0,
        })
    }

    /// One tick: the swap, the party routing, the scene drive and the combat step.
    pub fn step(&mut self, slots: SlotInputs) {
        let hurt = self.combat.hurt_ticks_of(self.party.controlled_hero()) > 0;
        self.party.request_swap(slots[0].swap, hurt);
        let hero = self.party.hero_inputs(slots);
        let drive = self.party.drive(true);
        self.scene.set_drive(drive);
        let restarts = self.combat.restarts;
        let seen = self.combat.events.len() as u64 + self.combat.dropped_events as u64;
        self.combat.step_party(&mut self.scene, hero);
        self.log_tells(seen);
        let respawned = self.combat.take_respawned();
        if self.combat.restarts != restarts || respawned {
            self.party.restart();
        }
        if let Some(id) = self.parked {
            let body = self.scene.world.body_mut(id);
            body.x = PARK_X;
            body.vx = 0.0;
        }
    }

    /// Record the `Tell`s and the proximity `Wake`s logged since the event counter `seen`.
    fn log_tells(&mut self, seen: u64) {
        let from = seen.saturating_sub(self.combat.dropped_events as u64) as usize;
        let facts: Vec<(u64, usize, Option<HeroId>)> = self.combat.events
            [from.min(self.combat.events.len())..]
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Trog {
                    tick,
                    enemy,
                    act: TrogAct::Tell { target },
                } => Some((
                    tick,
                    (enemy - crate::combat_scene::trog::TROG_ID_BASE) as usize,
                    Some(target),
                )),
                CombatEvent::Trog {
                    tick,
                    enemy,
                    act:
                        TrogAct::Wake {
                            cause: crate::trog::WakeCause::Proximity,
                        },
                } => Some((
                    tick,
                    (enemy - crate::combat_scene::trog::TROG_ID_BASE) as usize,
                    None,
                )),
                _ => None,
            })
            .collect();
        for (tick, enemy, target) in facts {
            let t = crate::trog::TrogTuning::active();
            let trog = self.trog(enemy);
            let b = &t.species(trog.species).hurt_idle;
            let (x0, x1) = if trog.facing >= 0 {
                (trog.x + f32::from(b.x0), trog.x + f32::from(b.x1))
            } else {
                (trog.x - f32::from(b.x1), trog.x - f32::from(b.x0))
            };
            let sp = t.species(trog.species);
            let meets_band = target.is_none_or(|id| {
                let hurt = CombatState::hurtbox(&self.scene, id);
                let (lo, hi) = (
                    trog.foot_y + f32::from(sp.hitbox.y0),
                    trog.foot_y + f32::from(sp.hitbox.y1),
                );
                hurt.bottom < hi && hurt.top() > lo
            });
            let fact = TellFact {
                tick,
                enemy,
                body: (x0, x1),
                view: self.scene.view_x(),
                target_x: target.map_or(f32::NAN, |id| self.x_of(id)),
                meets_band,
            };
            if target.is_some() {
                self.tell_log.push(fact);
            } else {
                self.wake_log.push(fact);
            }
        }
    }

    /// One tick with `raw` on P1's slot.
    pub fn step1(&mut self, raw: RawInput) {
        self.step([raw, RawInput::default()]);
    }

    /// Make Donut the controlled hero (solo): the first tick presses the swap.
    pub fn swap_to_donut(&mut self) {
        self.step1(RawInput {
            swap: true,
            ..RawInput::NONE
        });
        self.solo_donut = true;
    }

    pub fn tick(&self) -> u64 {
        self.scene.world.tick
    }

    pub fn x_of(&self, id: HeroId) -> f32 {
        self.scene.world.body(id).x
    }

    pub fn foot_of(&self, id: HeroId) -> f32 {
        self.scene.world.body(id).y - HERO_HALF_HEIGHT
    }

    /// World y of the walking line.
    pub fn foot_of_floor(&self) -> f32 {
        self.scene.world.hero_foot_y()
    }

    pub fn complete(&self) -> bool {
        self.scene.world.complete
    }

    pub fn hash(&self) -> u64 {
        self.combat.state_hash(&self.scene)
    }

    /// A setup write: a hero stands at `x` facing `facing`, at rest.
    pub fn place(&mut self, id: HeroId, x: f32, facing: i8) {
        let body = self.scene.world.body_mut(id);
        body.x = x;
        body.vx = 0.0;
        body.facing = facing;
    }

    /// The bench stack's partition state (`None` in a room with no partition prop).
    pub fn bench_state(&self) -> Option<crate::env_prop::PartitionState> {
        let prop = self.bench_prop()?;
        self.combat.env()?.partition(prop).map(|p| p.state())
    }

    /// The player credited with the bench's break, if it broke and anybody is credited.
    pub fn bench_initiator(&self) -> Option<u32> {
        let prop = self.bench_prop()?;
        self.combat.env()?.partition(prop)?.initiator()
    }

    fn bench_prop(&self) -> Option<usize> {
        self.run
            .loaded
            .encounter
            .props
            .iter()
            .position(|p| p.kind == crate::encounter::PropKind::Partition)
    }

    /// The `BenchSmash` events, `(tick, bait)`, in log order.
    pub fn bench_smashes(&self) -> Vec<(u64, HeroId)> {
        self.acts()
            .into_iter()
            .filter_map(|(t, _, a)| match a {
                TrogAct::BenchSmash { bait } => Some((t, bait)),
                _ => None,
            })
            .collect()
    }

    /// The troglodyte at roster index `i`.
    pub fn trog(&self, i: usize) -> &Trog {
        self.combat.slot_enemies()[i]
            .as_any()
            .and_then(|a| a.downcast_ref::<TrogEnemy>())
            .expect("a troglodyte")
            .trog()
    }

    pub fn health(&self, id: HeroId) -> u8 {
        self.scene.world.body(id).health
    }

    /// Every troglodyte act, `(tick, enemy index, act)`, in log order.
    pub fn acts(&self) -> Vec<(u64, usize, TrogAct)> {
        self.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Trog { tick, enemy, act } => Some((
                    tick,
                    (enemy - crate::combat_scene::trog::TROG_ID_BASE) as usize,
                    act,
                )),
                _ => None,
            })
            .collect()
    }

    pub fn hurts(&self) -> Vec<(u64, HeroId)> {
        self.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::HeroHurt { tick, hero, .. } => Some((tick, hero)),
                _ => None,
            })
            .collect()
    }

    pub fn defeats(&self) -> usize {
        self.combat
            .events
            .iter()
            .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
            .count()
    }

    pub fn hero_defeats(&self) -> usize {
        self.combat
            .events
            .iter()
            .filter(|e| matches!(e, CombatEvent::HeroDefeated { .. }))
            .count()
    }

    pub fn living(&self) -> Vec<usize> {
        (0..self.combat.slot_enemies().len())
            .filter(|&i| self.combat.slot_enemies()[i].alive())
            .collect()
    }
}

// ----------------------------------------------------------------- inputs

pub fn press() -> RawInput {
    RawInput {
        attack: true,
        ..RawInput::NONE
    }
}

pub fn moving(dir: f32) -> RawInput {
    RawInput {
        move_x: dir,
        ..RawInput::NONE
    }
}

pub fn stomp() -> RawInput {
    RawInput {
        physical: true,
        down: true,
        ..RawInput::NONE
    }
}

/// A read-only view of a launched room: what the scripted pilots read (a pure function of it).
pub struct Probe<'a> {
    pub scene: &'a SceneSim,
    pub statics: &'a [crate::encounter::StaticEntry],
    pub props: &'a [crate::encounter::Prop],
}

impl Probe<'_> {
    pub fn x_of(&self, id: HeroId) -> f32 {
        self.scene.world.body(id).x
    }

    pub fn foot_of(&self, id: HeroId) -> f32 {
        self.scene.world.body(id).y - HERO_HALF_HEIGHT
    }

    /// World y of the walking line.
    pub fn foot_of_floor(&self) -> f32 {
        self.scene.world.hero_foot_y()
    }

    pub fn tick(&self) -> u64 {
        self.scene.world.tick
    }

    /// The lane steps `(left, right)` a hero hops: the sidecar's statics and the partition props'
    /// spans (the bench stack, standing or broken: its 16 px remnant is hopped too).
    pub fn lane_steps(&self) -> Vec<(f32, f32)> {
        self.statics
            .iter()
            .map(|st| (st.rect.x as f32, st.rect.right() as f32))
            .chain(
                self.props
                    .iter()
                    .filter(|p| p.kind == crate::encounter::PropKind::Partition)
                    .map(|p| (p.target.x as f32, p.target.right() as f32)),
            )
            .collect()
    }
}

impl TrogSim {
    /// The pilots' read-only view of this room.
    pub fn probe(&self) -> Probe<'_> {
        Probe {
            scene: &self.scene,
            statics: &self.run.loaded.encounter.statics,
            props: &self.run.loaded.encounter.props,
        }
    }
}

/// Carl's brawler policy against the troglodyte `t` (a pure function of the state): what a
/// competent player does with the plain moves (TROG-AD22): back out of a committed tell, punish
/// the recovery low (the stomp on the Pygmy's landing, the jab in Rest), step into the Virtuoso's
/// dead zone and jab, never trade in front of a smash.
pub fn carl_vs(p: &Probe, t: &Trog, last_press: &mut u64) -> RawInput {
    let carl = p.x_of(HeroId::Carl);
    let dx = t.x - carl;
    let d = dx.abs();
    let dir = if dx >= 0.0 { 1.0 } else { -1.0 };
    let now = p.tick();
    let idle = !p.scene.world.hero.melee.busy();
    let ready = idle && now.saturating_sub(*last_press) >= 14;
    let mut jab = |input: RawInput| {
        *last_press = now;
        input
    };
    match t.state {
        State::Dormant | State::Down => {
            // Walk toward it until it wakes.
            if d > 60.0 {
                moving(dir)
            } else {
                RawInput::NONE
            }
        }
        State::Tell => match t.species {
            crate::trog::Species::Pygmy => {
                // Back out until a stomp box from L - 44 is still reachable: away from the
                // landing, at least 48 px.
                let l = t.landing_x();
                let away = -dir;
                if (carl - l).abs() < 50.0 || (carl - l) * away < 0.0 {
                    moving(away)
                } else {
                    RawInput::NONE
                }
            }
            crate::trog::Species::Basher => {
                if d < 58.0 {
                    moving(-dir)
                } else {
                    RawInput::NONE
                }
            }
            crate::trog::Species::Virtuoso => {
                // In under the tongue: the dead zone is inside 44 px.
                if d > 36.0 {
                    moving(dir)
                } else {
                    RawInput::NONE
                }
            }
        },
        State::Active => match t.species {
            crate::trog::Species::Pygmy => {
                let l = t.landing_x();
                let away = -dir;
                if (carl - l).abs() < 50.0 && (carl - l) * away >= 0.0 {
                    moving(away)
                } else {
                    RawInput::NONE
                }
            }
            crate::trog::Species::Basher => {
                if d < 58.0 {
                    moving(-dir)
                } else {
                    RawInput::NONE
                }
            }
            crate::trog::Species::Virtuoso => {
                if d > 36.0 {
                    moving(dir)
                } else {
                    RawInput::NONE
                }
            }
        },
        State::Recover => match t.species {
            crate::trog::Species::Pygmy => {
                // The stomp on the landing: close to the box's reach, then press.
                let want = 44.0;
                let l = t.x;
                if d > want + 3.0 {
                    moving(dir)
                } else if ready {
                    jab(stomp())
                } else {
                    let _ = l;
                    RawInput::NONE
                }
            }
            _ => {
                if d > 46.0 {
                    moving(dir)
                } else if ready {
                    jab(press())
                } else {
                    RawInput::NONE
                }
            }
        },
        State::Approach | State::Rest | State::Hurt => {
            let reach = match t.species {
                crate::trog::Species::Pygmy => 36.0,
                crate::trog::Species::Basher => 44.0,
                crate::trog::Species::Virtuoso => 36.0,
            };
            if d > reach + 3.0 {
                moving(dir)
            } else if ready {
                jab(press())
            } else {
                RawInput::NONE
            }
        }
    }
}

/// A hero pushing toward a low step jumps it (the refuge and the bell step are 32 px lane steps).
pub fn with_hop(p: &Probe, hero: HeroId, mut raw: RawInput) -> RawInput {
    let body = p.scene.world.body(hero);
    if raw.move_x == 0.0 || !body.grounded {
        return raw;
    }
    let ahead = body.x + raw.move_x.signum() * 30.0;
    let near = p.lane_steps().iter().any(|&(l, r)| {
        let inside = ahead >= l && ahead <= r;
        let on_it = body.x >= l && body.x <= r;
        inside && !on_it
    });
    raw.jump = near;
    raw
}

/// Donut's caster policy: walk to a firing column, face the troglodyte and cast on the cooldown.
pub fn donut_vs(p: &Probe, t: &Trog, last_press: &mut u64) -> RawInput {
    let donut = p.x_of(HeroId::Donut);
    // Standing on the 64 px platform her bolts fly over every troglodyte (a vantage, not a
    // turret): she walks off its right edge to the floor first.
    if p.foot_of(HeroId::Donut) > p.foot_of_floor() + 20.0 {
        return moving(1.0);
    }
    let dx = t.x - donut;
    let dir = if dx >= 0.0 { 1.0 } else { -1.0 };
    let now = p.tick();
    if dx.abs() > 200.0 {
        return moving(dir);
    }
    if now.saturating_sub(*last_press) >= 17 {
        *last_press = now;
        // Face it with the press: a move input of the same tick turns her.
        return RawInput {
            move_x: dir * 0.01,
            attack: true,
            ..RawInput::NONE
        };
    }
    moving(dir * 0.01)
}

// ----------------------------------------------------------------- pilots

/// The evidence pilots of `--trog-pilot` (window captures and headless runs of `trog_run`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pilot {
    /// Carl fights the troglodytes in order with the brawler policy, then walks to the exit.
    Carl,
    /// The first tick swaps to Donut, who casts at them from the floor, then walks to the exit.
    Donut,
    /// TROGB-AD1, the Test of Strength: Carl defeats the Virtuoso, stands in the bait zone (x 616)
    /// until the Basher smashes the bench, then fights it and walks to the exit.
    Bait,
}

impl Pilot {
    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "carl" => Ok(Self::Carl),
            "donut" => Ok(Self::Donut),
            "bait" => Ok(Self::Bait),
            other => Err(format!(
                "unknown --trog-pilot {other:?} (carl, donut or bait)"
            )),
        }
    }
}

/// The pilot's input for this tick: a pure function of the finished state (the brawler policies
/// with no memory, so a press repeats whenever Carl's move is idle).
pub fn pilot(
    kind: Pilot,
    combat: &CombatState,
    scene: &SceneSim,
    encounter: &crate::encounter::Encounter,
    exit_x: f32,
) -> RawInput {
    let probe = Probe {
        scene,
        statics: &encounter.statics,
        props: &encounter.props,
    };
    if kind == Pilot::Donut && scene.world.tick == 0 {
        return RawInput {
            swap: true,
            ..RawInput::NONE
        };
    }
    let hero = match kind {
        Pilot::Carl | Pilot::Bait => HeroId::Carl,
        Pilot::Donut => HeroId::Donut,
    };
    let target = combat.slot_enemies().iter().find_map(|e| {
        e.as_any()
            .and_then(|a| a.downcast_ref::<TrogEnemy>())
            .filter(|t| t.trog().alive())
            .map(|t| t.trog().clone())
    });
    if kind == Pilot::Bait {
        // The Virtuoso first (the Basher stays asleep left of x 600); then hold the bait column
        // while the bench stands; then the Basher (the bench is down) and the exit.
        let bench_up = combat
            .env()
            .and_then(|e| e.partition(0))
            .is_some_and(|p| p.is_standing());
        let virtuoso = combat
            .slot_enemies()
            .first()
            .and_then(|e| e.as_any())
            .and_then(|a| a.downcast_ref::<TrogEnemy>())
            .filter(|t| t.trog().alive())
            .map(|t| t.trog().clone());
        let x = probe.x_of(HeroId::Carl);
        let raw = match (&virtuoso, &target) {
            (Some(t), _) => carl_vs(&probe, t, &mut 0),
            (None, Some(_)) if bench_up => {
                // Walk to the column with no hop (the bench's face would trigger one).
                return if (x - 616.0).abs() <= 2.0 {
                    RawInput::NONE
                } else {
                    moving(if x < 616.0 { 1.0 } else { -1.0 })
                };
            }
            (None, Some(t)) => carl_vs(&probe, t, &mut 0),
            (None, None) => moving(if x < exit_x { 1.0 } else { 0.0 }),
        };
        return with_hop(&probe, HeroId::Carl, raw);
    }
    let raw = match (&target, kind) {
        (Some(t), Pilot::Carl) => carl_vs(&probe, t, &mut 0),
        (Some(t), Pilot::Donut) => donut_vs(&probe, t, &mut 0),
        (Some(_), Pilot::Bait) => unreachable!("the bait pilot returned above"),
        (None, _) => moving(if probe.x_of(hero) < exit_x { 1.0 } else { 0.0 }),
    };
    with_hop(&probe, hero, raw)
}

// ---------------------------------------------------------------- reports

/// What a fixture run reports.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub name: &'static str,
    pub ticks: u64,
    pub hash: u64,
    pub exit_tick: Option<u64>,
    pub defeats: usize,
    pub hero_defeats: usize,
    pub hurts: Vec<(u64, HeroId)>,
    pub acts: Vec<(u64, usize, TrogAct)>,
    pub tells: Vec<TellFact>,
}

impl Report {
    pub fn of(name: &'static str, sim: &TrogSim) -> Self {
        Self {
            name,
            ticks: sim.tick(),
            hash: sim.hash(),
            exit_tick: sim.complete().then(|| sim.tick()),
            defeats: sim.defeats(),
            hero_defeats: sim.hero_defeats(),
            hurts: sim.hurts(),
            acts: sim.acts(),
            tells: sim.tell_log.clone(),
        }
    }

    pub fn count(&self, f: impl Fn(&TrogAct) -> bool) -> usize {
        self.acts.iter().filter(|(_, _, a)| f(a)).count()
    }

    /// The text of the evidence line.
    pub fn line(&self) -> String {
        format!(
            "TROG_FIXTURE {} ticks={} exit_tick={} defeats={} hero_defeats={} hurts={} tells={} \
             whiffs={} strikes={} interrupted={} hash={:#018x}",
            self.name,
            self.ticks,
            self.exit_tick.map_or("none".to_owned(), |t| t.to_string()),
            self.defeats,
            self.hero_defeats,
            self.hurts.len(),
            self.count(|a| matches!(a, TrogAct::Tell { .. })),
            self.count(|a| *a == TrogAct::Whiff),
            self.count(|a| *a == TrogAct::Strike),
            self.count(|a| *a == TrogAct::Interrupted),
            self.hash,
        )
    }
}

/// The scripted cases, by their design ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    /// F01: Carl dodges the leap, stomps the landing and jabs the Rest.
    PygmyDodgeAndPunish,
    /// F02: Donut planted at 432 casts: the positive control.
    PygmyStationaryDonut,
    /// F03: Carl flanks the Basher: one interrupt, then a jab per recovery.
    BasherFlankAndPunish,
    /// F04: four bolts defeat the Basher; none interrupts.
    BasherBoltsDoNotInterrupt,
    /// F07: Carl solo (companion parked) clears the run.
    RunSoloCarl,
    /// F07: Donut solo (Carl parked) clears the run.
    RunSoloDonut,
    /// F07: P1 Carl and P2 Donut.
    Run2P,
    /// F07: Carl solo with the default `cf02_free` companion.
    RunSoloCompanion,
    /// F09 (TROG-B): `trog_strength`, Carl solo (Donut parked) defeats the Virtuoso left of
    /// x 600, climbs the 80 px bench and defeats the Basher: the bench is passed, never broken.
    StrengthDirect,
    /// F10 (TROGB-AD1): Carl stands in the bait zone (x 616) after the Virtuoso is down; the
    /// Basher, walk-blocked by the bench, smashes it: one `BenchSmash`, credited to Carl.
    StrengthBait,
    /// F11 (TROGB-AD1): Carl breaks the bench by hand with three separate jabs from pivot 620;
    /// no `BenchSmash`.
    StrengthBenchByHand,
    /// F12 (TROGB-AD1): Donut fights the Basher from x 720 and from x 760: the bench stays
    /// standing, no `BenchSmash`.
    StrengthNoBreakFarSide,
    /// F13: both troglodytes eligible at once, one hero each (2P): the tell gap holds.
    StrengthTellGap,
    /// F15: P1 Carl and P2 Donut clear the Test of Strength.
    Strength2P,
    /// F15: Carl solo with the default `cf02_free` companion clears it.
    StrengthSoloCompanion,
}

impl Case {
    pub const ALL: [Case; 15] = [
        Case::PygmyDodgeAndPunish,
        Case::PygmyStationaryDonut,
        Case::BasherFlankAndPunish,
        Case::BasherBoltsDoNotInterrupt,
        Case::RunSoloCarl,
        Case::RunSoloDonut,
        Case::Run2P,
        Case::RunSoloCompanion,
        Case::StrengthDirect,
        Case::StrengthBait,
        Case::StrengthBenchByHand,
        Case::StrengthNoBreakFarSide,
        Case::StrengthTellGap,
        Case::Strength2P,
        Case::StrengthSoloCompanion,
    ];

    /// The room the case runs in.
    pub const fn room(self) -> TrogRoom {
        match self {
            Self::StrengthDirect
            | Self::StrengthBait
            | Self::StrengthBenchByHand
            | Self::StrengthNoBreakFarSide
            | Self::StrengthTellGap
            | Self::Strength2P
            | Self::StrengthSoloCompanion => TrogRoom::Strength,
            _ => TrogRoom::Run,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::PygmyDodgeAndPunish => "trog_pygmy_dodge_and_punish",
            Self::PygmyStationaryDonut => "trog_pygmy_stationary_donut",
            Self::BasherFlankAndPunish => "trog_basher_flank_and_punish",
            Self::BasherBoltsDoNotInterrupt => "trog_basher_bolts_do_not_interrupt",
            Self::RunSoloCarl => "trog_run_solo_carl",
            Self::RunSoloDonut => "trog_run_solo_donut",
            Self::Run2P => "trog_run_2p",
            Self::RunSoloCompanion => "trog_run_solo_companion",
            Self::StrengthDirect => "trog_strength_direct",
            Self::StrengthBait => "trog_strength_bait",
            Self::StrengthBenchByHand => "trog_strength_bench_by_hand",
            Self::StrengthNoBreakFarSide => "trog_strength_no_break_from_far_side",
            Self::StrengthTellGap => "trog_strength_tell_gap",
            Self::Strength2P => "trog_strength_2p",
            Self::StrengthSoloCompanion => "trog_strength_solo_companion",
        }
    }
}

/// Run a case to its end.
pub fn run(root: &Path, case: Case) -> Result<(Report, TrogSim), String> {
    let mut sim = match case {
        Case::PygmyDodgeAndPunish | Case::BasherFlankAndPunish | Case::RunSoloCarl => {
            TrogSim::new(root, Party::Solo { uncontrolled: BOT }, Some(HeroId::Donut))?
        }
        Case::StrengthDirect | Case::StrengthBait | Case::StrengthBenchByHand => TrogSim::for_room(
            root,
            TrogRoom::Strength,
            Party::Solo { uncontrolled: BOT },
            Some(HeroId::Donut),
        )?,
        Case::StrengthNoBreakFarSide => {
            let mut s = TrogSim::for_room(
                root,
                TrogRoom::Strength,
                Party::Solo { uncontrolled: BOT },
                Some(HeroId::Carl),
            )?;
            s.swap_to_donut();
            s
        }
        Case::StrengthTellGap | Case::Strength2P => {
            TrogSim::for_room(root, TrogRoom::Strength, Party::TwoPlayer, None)?
        }
        Case::StrengthSoloCompanion => TrogSim::for_room(
            root,
            TrogRoom::Strength,
            Party::Solo { uncontrolled: FREE },
            None,
        )?,
        Case::PygmyStationaryDonut | Case::BasherBoltsDoNotInterrupt | Case::RunSoloDonut => {
            let mut s = TrogSim::new(root, Party::Solo { uncontrolled: BOT }, Some(HeroId::Carl))?;
            s.swap_to_donut();
            s
        }
        Case::Run2P => TrogSim::new(root, Party::TwoPlayer, None)?,
        Case::RunSoloCompanion => TrogSim::new(root, Party::Solo { uncontrolled: FREE }, None)?,
    };
    match case {
        Case::PygmyDodgeAndPunish => pygmy_dodge_and_punish(&mut sim),
        Case::PygmyStationaryDonut => pygmy_stationary_donut(&mut sim),
        Case::BasherFlankAndPunish => basher_flank_and_punish(&mut sim),
        Case::BasherBoltsDoNotInterrupt => basher_bolts(&mut sim),
        Case::RunSoloCarl | Case::RunSoloCompanion => clear_run(&mut sim, HeroId::Carl),
        Case::RunSoloDonut => clear_run(&mut sim, HeroId::Donut),
        Case::Run2P | Case::Strength2P => clear_run_2p(&mut sim),
        Case::StrengthDirect | Case::StrengthSoloCompanion => clear_run(&mut sim, HeroId::Carl),
        Case::StrengthBait => strength_bait(&mut sim),
        Case::StrengthBenchByHand => strength_bench_by_hand(&mut sim),
        Case::StrengthNoBreakFarSide => strength_no_break_far_side(&mut sim),
        Case::StrengthTellGap => strength_tell_gap(&mut sim),
    }
    Ok((Report::of(case.name(), &sim), sim))
}

fn run_until(sim: &mut TrogSim, max: u64, mut f: impl FnMut(&TrogSim) -> bool) -> bool {
    for _ in 0..max {
        if f(sim) {
            return true;
        }
        sim.step1(RawInput::NONE);
    }
    f(sim)
}

/// F01. Carl walks in until the Pygmy wakes and commits its leap at his pivot; he is then placed
/// at L - 48 facing the landing, takes no blow, stomps on the first land tick and jabs the Rest.
fn pygmy_dodge_and_punish(sim: &mut TrogSim) {
    sim.place(HeroId::Carl, 560.0, -1);
    assert!(run_until(sim, 400, |s| s.trog(0).state == State::Tell));
    let l = sim.trog(0).landing_x();
    sim.place(HeroId::Carl, l - 48.0, 1);
    // The leap: the Pygmy flies by and lands (the first Recover tick).
    assert!(run_until(sim, 80, |s| s.trog(0).state == State::Recover));
    // The stomp on the first land tick; Carl's move runs out inside the Pygmy's Rest.
    sim.step1(stomp());
    for _ in 0..60 {
        sim.step1(RawInput::NONE);
        if !sim.scene.world.hero.melee.busy() {
            break;
        }
    }
    // Rest: one tick in, then the jab.
    sim.step1(moving(1.0));
    sim.step1(press());
    for _ in 0..40 {
        sim.step1(RawInput::NONE);
    }
}

/// F02. Donut is planted on the floor at 432 facing right and casts every cooldown.
fn pygmy_stationary_donut(sim: &mut TrogSim) {
    sim.place(HeroId::Donut, 432.0, 1);
    let mut last = 0;
    for _ in 0..600 {
        if !sim.trog(0).alive() {
            break;
        }
        let raw = if sim.tick().saturating_sub(last) >= 17 {
            last = sim.tick();
            press()
        } else {
            RawInput::NONE
        };
        sim.step1(raw);
    }
}

/// F03. Carl at 36 px in front of the Basher: he jabs inside the first tell ticks (one interrupt),
/// then backs out of every later smash and lands a jab in each recovery.
fn basher_flank_and_punish(sim: &mut TrogSim) {
    let bx = 1376.0;
    sim.place(HeroId::Carl, bx - 36.0, 1);
    let mut last = 0u64;
    let mut cycle = 0;
    let mut prev_tell = false;
    for _ in 0..MAX_TICKS {
        let t = sim.trog(1).clone();
        if !t.alive() {
            break;
        }
        let tell = t.state == State::Tell;
        if tell && !prev_tell {
            cycle += 1;
        }
        prev_tell = tell;
        let raw = if cycle == 1
            && tell
            && t.attack_tick() == 4
            && sim.tick().saturating_sub(last) >= 14
        {
            last = sim.tick();
            press()
        } else if cycle <= 1 && t.state != State::Dormant {
            // Hold the first cycle's column until the interrupt is spent.
            RawInput::NONE
        } else {
            carl_vs(&sim.probe(), &t, &mut last)
        };
        sim.step1(raw);
    }
}

/// F04. Donut casts from far away: the bolts deal damage and never interrupt.
fn basher_bolts(sim: &mut TrogSim) {
    sim.place(HeroId::Donut, 1376.0 - 200.0, 1);
    let mut last = 0;
    for _ in 0..1500 {
        if !sim.trog(1).alive() {
            break;
        }
        let raw = if sim.tick().saturating_sub(last) >= 17 {
            last = sim.tick();
            press()
        } else {
            RawInput::NONE
        };
        sim.step1(raw);
    }
}

/// What F08 reports.
#[derive(Debug, Clone, PartialEq)]
pub struct RetryFacts {
    /// A hero was defeated in T2 (the party defeat).
    pub party_defeats: usize,
    /// After the checkpoint respawn: the Pygmy is still down, the Basher stands at its spawn,
    /// whole and asleep again.
    pub pygmy_down: bool,
    pub basher_state: State,
    pub basher_hp: u16,
    pub basher_x: f32,
    /// After a fixture restart every troglodyte is rebuilt.
    pub restart_all_alive: bool,
    pub restart_states: Vec<State>,
    pub hash: u64,
}

/// F08: a party defeat in T2, the checkpoint respawn, then a fixture restart.
pub fn run_retry(root: &Path) -> Result<RetryFacts, String> {
    let mut sim = TrogSim::new(root, Party::Solo { uncontrolled: BOT }, Some(HeroId::Donut))?;
    sim.place(HeroId::Carl, 560.0, -1);
    let mut last = 0u64;
    // T1: Carl defeats the Pygmy.
    for _ in 0..MAX_TICKS {
        if !sim.trog(0).alive() {
            break;
        }
        let t = sim.trog(0).clone();
        let raw = carl_vs(&sim.probe(), &t, &mut last);
        let raw = with_hop(&sim.probe(), HeroId::Carl, raw);
        sim.step1(raw);
    }
    if sim.trog(0).alive() {
        return Err("the Pygmy was not defeated".into());
    }
    // T2: Carl stands in front of the Basher and takes every smash.
    let bx = sim.trog(1).spawn_x();
    sim.place(HeroId::Carl, bx - 40.0, 1);
    for _ in 0..MAX_TICKS {
        if sim.hero_defeats() > 0 {
            break;
        }
        // A blow knocks him back: he walks in again and takes the next smash.
        let gap = (bx - 40.0) - sim.x_of(HeroId::Carl);
        sim.step1(if gap > 6.0 {
            moving(1.0)
        } else {
            RawInput::NONE
        });
    }
    let party_defeats = sim.hero_defeats();
    for _ in 0..30 {
        sim.step1(RawInput::NONE);
    }
    sim.combat.respawn(&mut sim.scene);
    sim.party.restart();
    sim.step1(RawInput::NONE);
    let pygmy_down = !sim.trog(0).alive();
    let (basher_state, basher_hp, basher_x) = {
        let b = sim.trog(1);
        (b.state, b.hp, b.x)
    };
    // The fixture restart rebuilds everything.
    sim.combat.restart(&mut sim.scene);
    sim.party.restart();
    sim.step1(RawInput::NONE);
    let restart_states: Vec<State> = (0..3).map(|i| sim.trog(i).state).collect();
    Ok(RetryFacts {
        party_defeats,
        pygmy_down,
        basher_state,
        basher_hp,
        basher_x,
        restart_all_alive: sim.living().len() == 3,
        restart_states,
        hash: sim.hash(),
    })
}

fn exit_x(sim: &TrogSim) -> f32 {
    sim.run.loaded.encounter.exit.x as f32 + 32.0
}

/// One hero clears the run: fight the nearest living troglodyte, then walk to the exit.
fn clear_run(sim: &mut TrogSim, hero: HeroId) {
    if hero == HeroId::Donut {
        // Carl is parked and Donut was swapped in by the case.
    }
    let (mut last_c, mut last_d) = (0u64, 0u64);
    for _ in 0..MAX_TICKS {
        if sim.complete() {
            break;
        }
        let living = sim.living();
        let raw = match living.first().copied() {
            Some(i) => {
                let t = sim.trog(i).clone();
                match hero {
                    HeroId::Carl => carl_vs(&sim.probe(), &t, &mut last_c),
                    HeroId::Donut => donut_vs(&sim.probe(), &t, &mut last_d),
                }
            }
            None => {
                let x = sim.x_of(hero);
                moving(if x < exit_x(sim) { 1.0 } else { 0.0 })
            }
        };
        let raw = with_hop(&sim.probe(), hero, raw);
        sim.step1(raw);
    }
}

/// P1 Carl and P2 Donut each fight the nearest living troglodyte, then walk on.
fn clear_run_2p(sim: &mut TrogSim) {
    let (mut last_c, mut last_d) = (0u64, 0u64);
    for _ in 0..MAX_TICKS {
        if sim.complete() {
            break;
        }
        let (c, d) = match sim.living().first().copied() {
            Some(i) => {
                let t = sim.trog(i).clone();
                (
                    carl_vs(&sim.probe(), &t, &mut last_c),
                    donut_vs(&sim.probe(), &t, &mut last_d),
                )
            }
            None => (
                moving(if sim.x_of(HeroId::Carl) < exit_x(sim) {
                    1.0
                } else {
                    0.0
                }),
                moving(if sim.x_of(HeroId::Donut) < exit_x(sim) {
                    1.0
                } else {
                    0.0
                }),
            ),
        };
        let (c, d) = (
            with_hop(&sim.probe(), HeroId::Carl, c),
            with_hop(&sim.probe(), HeroId::Donut, d),
        );
        sim.step([c, d]);
    }
}

// ------------------------------------------------------------- TROG-B

/// Carl (the controlled hero, left of x 600 so the Basher stays Dormant) defeats the Virtuoso.
fn defeat_the_virtuoso(sim: &mut TrogSim) {
    let mut last = 0u64;
    for _ in 0..MAX_TICKS {
        if !sim.trog(0).alive() {
            break;
        }
        let t = sim.trog(0).clone();
        let raw = carl_vs(&sim.probe(), &t, &mut last);
        let raw = with_hop(&sim.probe(), HeroId::Carl, raw);
        sim.step1(raw);
    }
}

/// F10 (TROGB-AD1). The Virtuoso is down. Carl stands in the bait zone at 616 (a setup write) and
/// holds still: the Basher wakes at 600, walks to the bench face (pivot 696), and rule (b) starts a
/// smash at the bench. Its impact breaks the bench; Carl is out of reach and punishes the recovery.
fn strength_bait(sim: &mut TrogSim) {
    defeat_the_virtuoso(sim);
    sim.place(HeroId::Carl, 616.0, 1);
    let mut last = 0u64;
    let mut broke_at: Option<u64> = None;
    for _ in 0..MAX_TICKS {
        if !sim.trog(1).alive() {
            break;
        }
        let broken = sim.bench_state() == Some(crate::env_prop::PartitionState::Broken);
        if broken && broke_at.is_none() {
            broke_at = Some(sim.tick());
        }
        let raw = if broken {
            // The bench is gone: Carl fights the Basher (it is in its recovery).
            let t = sim.trog(1).clone();
            let raw = carl_vs(&sim.probe(), &t, &mut last);
            with_hop(&sim.probe(), HeroId::Carl, raw)
        } else {
            RawInput::NONE
        };
        sim.step1(raw);
    }
}

/// F11 (TROGB-AD1). The Virtuoso is down and Carl is left of 600 (the Basher is Dormant). He walks
/// to pivot 620 (crossing 600 wakes the Basher), then makes three separate jabs, each pressed
/// after the previous one ended, at the bench.
fn strength_bench_by_hand(sim: &mut TrogSim) {
    defeat_the_virtuoso(sim);
    sim.place(HeroId::Carl, 560.0, 1);
    let mut presses = 0u32;
    let mut last = 0u64;
    for _ in 0..400 {
        if sim.bench_state() == Some(crate::env_prop::PartitionState::Broken) {
            break;
        }
        let x = sim.x_of(HeroId::Carl);
        let idle = !sim.scene.world.hero.melee.busy();
        let raw = if x < 620.0 {
            moving(1.0)
        } else if idle && presses < 3 && sim.tick().saturating_sub(last) >= 14 {
            presses += 1;
            last = sim.tick();
            press()
        } else {
            RawInput::NONE
        };
        sim.step1(raw);
    }
    for _ in 0..30 {
        sim.step1(RawInput::NONE);
    }
}

/// F12 (TROGB-AD1). Donut (Carl parked) stands on the floor at 720, then at 760, and casts at the
/// Basher: the open band lies inside the hitbox, so no smash reaches the bench.
fn strength_no_break_far_side(sim: &mut TrogSim) {
    let mut last = 0u64;
    for column in [720.0f32, 760.0] {
        sim.place(HeroId::Donut, column, -1);
        for _ in 0..450 {
            if !sim.trog(1).alive() {
                break;
            }
            let raw = if sim.tick().saturating_sub(last) >= 17 {
                last = sim.tick();
                RawInput {
                    move_x: -0.01,
                    attack: true,
                    ..RawInput::NONE
                }
            } else {
                RawInput::NONE
            };
            sim.step1(raw);
        }
    }
}

/// How many troglodyte hitboxes are live on `hero` this tick (the committed hitbox of each
/// troglodyte on a strike or impact tick, against the hero's hurtbox).
pub fn live_hits_on(sim: &TrogSim, hero: HeroId) -> usize {
    let t = crate::trog::TrogTuning::active();
    let hurt = CombatState::hurtbox(&sim.scene, hero);
    (0..sim.combat.slot_enemies().len())
        .filter(|&i| {
            sim.trog(i)
                .live_hitbox(t, &sim.scene.world.solids)
                .is_some_and(|b| b.overlaps(hurt))
        })
        .count()
}

/// F13. Both troglodytes eligible at once, one hero each: Carl on the floor in the Virtuoso's
/// reach and Donut on the floor in the Basher's (setup writes, then no input). The run ends
/// when each has told once (and its blow has resolved); every tick is checked for a hero under
/// two live hitboxes.
fn strength_tell_gap(sim: &mut TrogSim) {
    // Carl faces the Virtuoso's tongue from its right (the dead zone is inside +56, the band
    // 56..77): 66 px from its pivot at 496 puts his hurtbox in the band. Donut stands in the
    // Basher's open band (x 24..38 from its pivot 800 facing left).
    // The second player joins on the first tick (Donut is placed beside Carl then), so the
    // setup writes come after it.
    sim.step([RawInput::NONE, RawInput::NONE]);
    sim.place(HeroId::Carl, 562.0, -1);
    sim.place(HeroId::Donut, 780.0, 1);
    let mut told = [false; 2];
    for _ in 0..900 {
        for (i, seen) in told.iter_mut().enumerate() {
            *seen |= sim
                .acts()
                .iter()
                .any(|(_, e, a)| *e == i && matches!(a, TrogAct::Tell { .. }));
        }
        if told == [true, true]
            && sim.trog(0).state != State::Active
            && sim.trog(1).state != State::Active
        {
            break;
        }
        sim.step([RawInput::NONE, RawInput::NONE]);
        for hero in HeroId::ALL {
            let n = live_hits_on(sim, hero);
            if n > 1 {
                sim.double_live += 1;
            }
        }
    }
}

/// What F14 reports for the Test of Strength.
#[derive(Debug, Clone, PartialEq)]
pub struct StrengthRetryFacts {
    pub party_defeats: usize,
    /// After the checkpoint respawn: the Virtuoso (defeated first) stays down, the living Basher
    /// stands at its spawn, whole and asleep again.
    pub virtuoso_down: bool,
    pub basher_state: State,
    pub basher_hp: u16,
    pub basher_x: f32,
    /// After a fixture restart every troglodyte is rebuilt.
    pub restart_states: Vec<State>,
    pub restart_all_alive: bool,
    pub hash: u64,
}

/// F14: Carl defeats the Virtuoso, a party defeat in front of the Basher, the checkpoint
/// respawn, then a fixture restart. The bench is a static solid, so there is no state to keep.
pub fn run_retry_strength(root: &Path) -> Result<StrengthRetryFacts, String> {
    let mut sim = TrogSim::for_room(
        root,
        TrogRoom::Strength,
        Party::Solo { uncontrolled: BOT },
        Some(HeroId::Donut),
    )?;
    let mut last = 0u64;
    sim.place(HeroId::Carl, 400.0, 1);
    for _ in 0..MAX_TICKS {
        if !sim.trog(0).alive() {
            break;
        }
        let t = sim.trog(0).clone();
        let raw = carl_vs(&sim.probe(), &t, &mut last);
        let raw = with_hop(&sim.probe(), HeroId::Carl, raw);
        sim.step1(raw);
    }
    if sim.trog(0).alive() {
        return Err("the Virtuoso was not defeated".into());
    }
    // The Basher: Carl stands in front of it and takes every smash.
    let bx = sim.trog(1).spawn_x();
    sim.place(HeroId::Carl, bx - 40.0, 1);
    for _ in 0..MAX_TICKS {
        if sim.hero_defeats() > 0 {
            break;
        }
        let gap = (bx - 40.0) - sim.x_of(HeroId::Carl);
        sim.step1(if gap > 6.0 {
            moving(1.0)
        } else {
            RawInput::NONE
        });
    }
    let party_defeats = sim.hero_defeats();
    for _ in 0..30 {
        sim.step1(RawInput::NONE);
    }
    sim.combat.respawn(&mut sim.scene);
    sim.party.restart();
    sim.step1(RawInput::NONE);
    let virtuoso_down = !sim.trog(0).alive();
    let (basher_state, basher_hp, basher_x) = {
        let b = sim.trog(1);
        (b.state, b.hp, b.x)
    };
    sim.combat.restart(&mut sim.scene);
    sim.party.restart();
    sim.step1(RawInput::NONE);
    let restart_states: Vec<State> = (0..2).map(|i| sim.trog(i).state).collect();
    Ok(StrengthRetryFacts {
        party_defeats,
        virtuoso_down,
        basher_state,
        basher_hp,
        basher_x,
        restart_all_alive: sim.living().len() == 2,
        restart_states,
        hash: sim.hash(),
    })
}

#[cfg(test)]
mod tests;
