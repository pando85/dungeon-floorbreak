//! REST-WIRE scenario fixtures (`docs/01-design/floor-01-quiet-setpieces-and-transitions.md`
//! section G.9): the refuge (`rest_refuge`), the camp (`rest_camp`) and the aftermath
//! (`rest_aftermath`) driven by scripted party inputs on the launched room, deterministic,
//! seeded and fixed-tick. Each fixture runs twice and must
//! end on an identical state hash; the final hashes are pinned here (new constants, nothing
//! moves). Bevy-free.
//!
//! The scripted **solo** fixtures pin the uncontrolled hero to `--companion off` (the
//! follow-and-jump bot) so the autonomous companion cannot open the door before the
//! scripted swap; exactly one fixture ([`Case::SoloCompanionCrossing`]) runs the default
//! `cf02_free` companion. Scripts are closed-loop on the heroes' columns (no tick
//! counting): walk to a column, swap, jump at the shelf's takeoff column, walk on.
//!
//! The room is only completable under the Story or All loadout (see
//! [`crate::f01_rest`]); a fixture that must not grant uses no story flags.

use crate::carl_prog::CarlProgFlags;
use crate::combat::CombatEvent;
use crate::combat_scene::CombatState;
use crate::companion::profile::ProfileSetId;
use crate::corridor::SceneSim;
use crate::donut_mana::ManaEvent;
use crate::donut_prog::DonutProgFlags;
use crate::f01_late::Loadout;
use crate::f01_rest::{self, RestRoom, RestRun};
use crate::input::{RawInput, SlotInputs};
use crate::party::{HeroId, PROVISIONAL_TUNING, PadId, PartyState, PartyTuning, UncontrolledHero};
use crate::prototype::HERO_HALF_HEIGHT;
use crate::story_unlock::LedgerSnapshot;
use std::path::Path;

/// The fixed-tick budget of every fixture.
pub const MAX_TICKS: u64 = 2400;
/// The column where the scripted Donut presses the jump onto the shelf (a running
/// takeoff, found with the physics-exact witness of the shelf edge).
pub const SHELF_TAKEOFF_X: f32 = 1215.0;
/// Carl waits this far left of the door's edge (inside the 96 px hint reach).
pub const DOOR_WAIT_X: f32 = 1400.0;
/// The seed of the fixtures' `--cf02-seed`.
pub const SEED: u64 = 0x0F01_5EED;

/// How the party of a fixture is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Party {
    /// One human (slot 0) drives `controlled`; the other hero is `uncontrolled`.
    Solo { uncontrolled: UncontrolledHero },
    /// P1 Carl, P2 Donut.
    TwoPlayer,
}

/// The follow-and-jump bot (`--companion off`).
pub const BOT: UncontrolledHero = UncontrolledHero::FollowAndJumpBot;
/// The default of a room scene: `cf02_free`.
pub const FREE: UncontrolledHero = UncontrolledHero::Companion(ProfileSetId::Cf02Free);

/// A launched quiet room with a party, stepped the way the interactive scene steps it.
pub struct RestSim {
    pub run: RestRun,
    pub scene: SceneSim,
    pub combat: CombatState,
    pub party: PartyState,
    /// Donut's pivot column on each tick a grant was logged (evidence for "the grant
    /// fires when her pivot enters the reward column").
    pub grant_x: Vec<f32>,
    /// Where Donut stood (`x`, feet layout y, grounded) on each tick the door opened.
    pub door_stance: Vec<(f32, i32, bool)>,
    /// Whether both heroes were inside the camera view on the first door-opening tick.
    pub both_in_view_at_door: Option<bool>,
    /// The ledger the room started with (after any carry), for "the ledger did not move".
    pub start_snapshot: Option<LedgerSnapshot>,
}

impl RestSim {
    /// The refuge.
    pub fn new(
        root: &Path,
        view_h: i32,
        carl: CarlProgFlags,
        donut: DonutProgFlags,
        party: Party,
    ) -> Result<Self, String> {
        Self::new_in(root, RestRoom::Refuge, view_h, carl, donut, party, None)
    }

    /// Any quiet room, optionally entered with a carried ledger (the run's relaunch).
    pub fn new_in(
        root: &Path,
        room: RestRoom,
        view_h: i32,
        carl: CarlProgFlags,
        donut: DonutProgFlags,
        party: Party,
        carry: Option<&LedgerSnapshot>,
    ) -> Result<Self, String> {
        let run = f01_rest::launch(root, room)?;
        let mut scene = SceneSim::new_at(&run.kit, view_h, run.fixture.carl_x)?;
        let mut combat = f01_rest::combat_for(&run, &mut scene, carl, donut, Some(SEED))?;
        if let Some(snapshot) = carry {
            combat.carry_story(snapshot).map_err(|e| e.to_string())?;
        }
        let start_snapshot = combat.story_snapshot();
        let uncontrolled = match party {
            Party::Solo { uncontrolled } => uncontrolled,
            Party::TwoPlayer => BOT,
        };
        let mut state = PartyState::new(
            PartyTuning {
                uncontrolled_hero: uncontrolled,
                ..PROVISIONAL_TUNING
            }
            .with_cf02_swap(),
        );
        if party == Party::TwoPlayer {
            state.pad_connected(PadId(1));
            state.pad_start_pressed(PadId(2));
        }
        Ok(Self {
            run,
            scene,
            combat,
            party: state,
            grant_x: Vec::new(),
            door_stance: Vec::new(),
            both_in_view_at_door: None,
            start_snapshot,
        })
    }

    /// One tick, the order of the interactive scene: the swap, the party routing, the
    /// scene drive and the combat step.
    pub fn step(&mut self, slots: SlotInputs) {
        let hurt = self.combat.hurt_ticks_of(self.party.controlled_hero()) > 0;
        self.party.request_swap(slots[0].swap, hurt);
        let hero = self.party.hero_inputs(slots);
        let drive = self.party.drive(true);
        self.scene.set_drive(drive);
        let restarts = self.combat.restarts;
        let (grants, doors) = (self.grants().len(), self.door_events().len());
        self.combat.step_party(&mut self.scene, hero);
        let respawned = self.combat.take_respawned();
        if self.combat.restarts != restarts || respawned {
            self.party.restart();
        }
        if self.grants().len() > grants || self.door_events().len() > doors {
            let body = self.scene.world.body(HeroId::Donut);
            let foot =
                self.scene.world.layout_height() - (body.y - HERO_HALF_HEIGHT).round() as i32;
            if self.grants().len() > grants {
                self.grant_x.push(body.x);
            }
            if self.door_events().len() > doors {
                self.door_stance.push((body.x, foot, body.grounded));
                if self.both_in_view_at_door.is_none() {
                    self.both_in_view_at_door = Some(
                        HeroId::ALL
                            .into_iter()
                            .all(|id| !self.scene.is_offscreen(self.scene.world.body(id).x, 0.0)),
                    );
                }
            }
        }
    }

    pub fn x_of(&self, id: HeroId) -> f32 {
        self.scene.world.body(id).x
    }

    pub fn tick(&self) -> u64 {
        self.scene.world.tick
    }

    pub fn complete(&self) -> bool {
        self.scene.world.complete
    }

    pub fn hash(&self) -> u64 {
        self.combat.state_hash(&self.scene)
    }

    /// The staff door has opened.
    pub fn door_open(&self) -> bool {
        self.combat
            .env()
            .and_then(|env| env.surface_locks_open().first().copied())
            .unwrap_or(false)
    }

    /// `(tick, hero, capability)` of every grant, in order.
    pub fn grants(&self) -> Vec<(u64, u8, &'static str)> {
        self.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::CapabilityGranted {
                    tick,
                    hero,
                    capability,
                } => Some((tick, hero, capability)),
                _ => None,
            })
            .collect()
    }

    /// The tick of each door opening, in order.
    pub fn door_events(&self) -> Vec<(u64, u8)> {
        self.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::SurfaceLockOpened { tick, hero, .. } => Some((tick, hero)),
                _ => None,
            })
            .collect()
    }

    /// Whether Donut stands on the shelf (platform 2) now.
    pub fn donut_on_shelf(&self) -> bool {
        let Some(shelf) = self.run.loaded.room.platforms.get(2).map(|p| p.rect) else {
            return false;
        };
        let body = self.scene.world.body(HeroId::Donut);
        let foot = self.scene.world.layout_height() - (body.y - HERO_HALF_HEIGHT).round() as i32;
        body.grounded && foot == shelf.y && (shelf.x as f32..shelf.right() as f32).contains(&body.x)
    }

    /// Hurt events (the refuge has no damage source: always zero).
    pub fn damage(&self) -> usize {
        self.combat
            .events
            .iter()
            .filter(|e| matches!(e, CombatEvent::HeroHurt { .. }))
            .count()
    }
}

/// Walk `from` toward `to`: the `move_x` that closes the gap (zero inside 3 px).
pub fn walk(from: f32, to: f32) -> f32 {
    let d = to - from;
    if d.abs() <= 3.0 { 0.0 } else { d.signum() }
}

/// The story flags of the Story loadout.
pub fn story() -> (CarlProgFlags, DonutProgFlags) {
    Loadout::Story.flags()
}

/// A short report of a run.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub ticks: u64,
    pub hash: u64,
    pub exit_tick: Option<u64>,
    pub grants: Vec<(u64, u8, &'static str)>,
    pub doors: Vec<(u64, u8)>,
    pub damage: usize,
    /// Donut's column on each grant tick.
    pub grant_x: Vec<f32>,
    /// Donut's `(x, feet layout y, grounded)` on each door-opening tick.
    pub door_stance: Vec<(f32, i32, bool)>,
    /// Both heroes were inside the camera view on the door-opening tick (2P evidence).
    pub both_in_view: Option<bool>,
    /// Donut stood on the shelf (feet on its top, inside its span) on the final tick.
    pub donut_on_shelf: bool,
    /// The receipts of the final ledger.
    pub receipts: usize,
    /// The final ledger equals the one the room started with (a quiet room grants nothing).
    pub ledger_unchanged: bool,
    /// `(tick, from_x, to_x)` of each resolved blink (Puddle Jumper).
    pub blinks: Vec<(u64, i32, i32)>,
    /// Blinks rejected, fizzled or cancelled.
    pub blink_failures: usize,
    /// `(tick, mana left)` of each mana spend.
    pub spends: Vec<(u64, u8)>,
}

impl Outcome {
    pub fn of(sim: &RestSim) -> Self {
        Self {
            ticks: sim.tick(),
            hash: sim.hash(),
            exit_tick: sim.complete().then(|| sim.tick()),
            grants: sim.grants(),
            doors: sim.door_events(),
            damage: sim.damage(),
            grant_x: sim.grant_x.clone(),
            door_stance: sim.door_stance.clone(),
            both_in_view: sim.both_in_view_at_door,
            donut_on_shelf: sim.donut_on_shelf(),
            receipts: sim.combat.story_snapshot().map_or(0, |s| s.receipts.len()),
            ledger_unchanged: sim.combat.story_snapshot() == sim.start_snapshot,
            blinks: sim
                .combat
                .events
                .iter()
                .filter_map(|e| match *e {
                    CombatEvent::BlinkResolved {
                        tick, from_x, to_x, ..
                    } => Some((tick, from_x, to_x)),
                    _ => None,
                })
                .collect(),
            blink_failures: sim
                .combat
                .events
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        CombatEvent::SpecialRejected { hero: 1, .. }
                            | CombatEvent::BlinkFizzled { .. }
                            | CombatEvent::BlinkCancelled { .. }
                    )
                })
                .count(),
            spends: sim
                .combat
                .events
                .iter()
                .filter_map(|e| match *e {
                    CombatEvent::Mana(ManaEvent::ManaSpent { tick, left, .. }) => {
                        Some((tick, left))
                    }
                    _ => None,
                })
                .collect(),
        }
    }
}

/// The scripted cases (REST-WIRE section G.9): the refuge's seven, then the camp's two and
/// the aftermath's three.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    /// Companion off. Carl active walks right (Donut follows), stops at the door, swaps,
    /// Donut jumps onto the shelf, both walk to the exit.
    SoloCarlSwap,
    /// Companion off. Donut active from the first tick, the same crossing.
    SoloDonut,
    /// P1 Carl, P2 Donut.
    TwoPlayer,
    /// The default `cf02_free` companion; Carl active walks right to the exit, swapping
    /// once at the closed door as a human would after the hint.
    SoloCompanionCrossing,
    /// Donut walks back and forth across the reward column.
    GrantOnceAtTheBox,
    /// No story flags: nothing arms the trigger; Donut jumps at the shelf repeatedly.
    BaseDonutCannotOpen,
    /// Grant, a checkpoint respawn, a fixture restart, a second crossing.
    RetryKeepsTheGrant,
    /// The camp under the post-Hoarder pin, solo: Carl walks to the exit.
    CampCrossingSolo,
    /// The camp, two players: both walk to the exit.
    CampCrossing2P,
    /// The aftermath entered with a post-Juicer ledger (all four capabilities with
    /// receipts): Carl walks to the exit and nothing is granted twice.
    AftermathNoDuplicateUnlock,
    /// The aftermath, Story loadout: Donut walks to the blink lane and blinks across it.
    AftermathBlinkLane,
    /// The aftermath with no story flag (base moves only): Carl walks to the exit.
    AftermathCrossingBaseMoves,
}

impl Case {
    pub const ALL: [Case; 12] = [
        Case::SoloCarlSwap,
        Case::SoloDonut,
        Case::TwoPlayer,
        Case::SoloCompanionCrossing,
        Case::GrantOnceAtTheBox,
        Case::BaseDonutCannotOpen,
        Case::RetryKeepsTheGrant,
        Case::CampCrossingSolo,
        Case::CampCrossing2P,
        Case::AftermathNoDuplicateUnlock,
        Case::AftermathBlinkLane,
        Case::AftermathCrossingBaseMoves,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Case::SoloCarlSwap => "rest_refuge_solo_carl_swap",
            Case::SoloDonut => "rest_refuge_solo_donut",
            Case::TwoPlayer => "rest_refuge_2p",
            Case::SoloCompanionCrossing => "rest_refuge_solo_companion_crossing",
            Case::GrantOnceAtTheBox => "rest_refuge_grant_once_at_the_box",
            Case::BaseDonutCannotOpen => "rest_refuge_base_donut_cannot_open",
            Case::RetryKeepsTheGrant => "rest_refuge_retry_keeps_the_grant",
            Case::CampCrossingSolo => "rest_camp_crossing_solo",
            Case::CampCrossing2P => "rest_camp_crossing_2p",
            Case::AftermathNoDuplicateUnlock => "rest_aftermath_no_duplicate_unlock",
            Case::AftermathBlinkLane => "rest_aftermath_blink_lane",
            Case::AftermathCrossingBaseMoves => "rest_aftermath_crossing_base_moves",
        }
    }

    /// The quiet room the case runs in.
    pub const fn room(self) -> RestRoom {
        match self {
            Case::CampCrossingSolo | Case::CampCrossing2P => RestRoom::Camp,
            Case::AftermathNoDuplicateUnlock
            | Case::AftermathBlinkLane
            | Case::AftermathCrossingBaseMoves => RestRoom::Aftermath,
            _ => RestRoom::Refuge,
        }
    }
}

/// What a scripted Donut does, closed-loop on her column (the PLAYTRACE recording test
/// drives a live-shaped rig with the same script).
#[derive(Debug, Clone, Copy)]
pub struct DonutScript {
    /// The tick the takeoff jump was pressed on.
    jumped_at: Option<u64>,
    /// The takeoff columns tried in turn (a missed landing moves to the next).
    takeoffs: &'static [f32],
    attempt: usize,
}

impl Default for DonutScript {
    fn default() -> Self {
        Self::with(&[SHELF_TAKEOFF_X])
    }
}

impl DonutScript {
    /// The script of the base-Donut probe (a spread of takeoff columns, evidence and the
    /// `rest_refuge_base_donut_cannot_open` fixture).
    pub const fn base_probe() -> Self {
        Self::with(BASE_TAKEOFFS)
    }

    const fn with(takeoffs: &'static [f32]) -> Self {
        Self {
            jumped_at: None,
            takeoffs,
            attempt: 0,
        }
    }

    fn takeoff(&self) -> f32 {
        self.takeoffs[self.attempt % self.takeoffs.len()]
    }

    fn input(&mut self, sim: &RestSim, exit_x: f32) -> RawInput {
        self.decide(
            sim.x_of(HeroId::Donut),
            sim.scene.world.body(HeroId::Donut).grounded,
            sim.door_open(),
            sim.tick(),
            exit_x,
        )
    }

    /// The input of Donut toward the shelf: back off to the left of the takeoff column,
    /// run right, press the jump at the column, then (door open) walk to the exit. A pure
    /// function of her column, whether she is grounded, the door and the tick.
    pub fn decide(
        &mut self,
        x: f32,
        grounded: bool,
        door_open: bool,
        tick: u64,
        exit_x: f32,
    ) -> RawInput {
        let takeoff = self.takeoff();
        let mut input = RawInput::NONE;
        if door_open {
            input.move_x = walk(x, exit_x);
            return input;
        }
        match self.jumped_at {
            None if x > takeoff + 4.0 => input.move_x = walk(x, takeoff - 60.0),
            None if x < takeoff - 90.0 => input.move_x = 1.0,
            None => {
                input.move_x = 1.0;
                if x >= takeoff && grounded {
                    self.jumped_at = Some(tick);
                    input.jump = true;
                }
            }
            Some(at) => {
                input.move_x = 1.0;
                input.jump = tick <= at + 6;
                // A missed landing starts over from the next takeoff column.
                if tick > at + 90 {
                    self.jumped_at = None;
                    self.attempt += 1;
                }
            }
        }
        input
    }
}

fn exit_x(sim: &RestSim) -> f32 {
    let e = sim.run.loaded.encounter.exit;
    (e.x + e.w / 2) as f32
}

fn swap() -> RawInput {
    RawInput {
        swap: true,
        ..RawInput::NONE
    }
}

/// The evidence pilots of the quiet rooms (`floor01_scene --rest-pilot MODE`): a scripted
/// run that stands a hero in every beat of the room for the captures. Each is a pure function
/// of the finished state (the party swap on the first tick is the pilot's own press).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pilot {
    /// Swap to Donut on the first tick, walk to the box, run and jump onto the shelf, walk
    /// to the exit (the Story crossing).
    Donut,
    /// Carl walks to the closed door and waits (the `SWAP TO DONUT` hint shows).
    CarlDoor,
    /// Swap to Donut and jump at the shelf from a column before the box: a base jump, which
    /// does not reach the 128 px shelf (the "before" of the before/after strip).
    BaseJump,
    /// Any quiet room: Carl walks right to the exit (the camp's entrance, the walk).
    Walk,
    /// The aftermath: swap to Donut on the first tick, walk to the blink lane's near mark,
    /// blink across it, walk on to the exit.
    Blink,
}

impl Pilot {
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "donut" => Ok(Self::Donut),
            "carl_door" => Ok(Self::CarlDoor),
            "base_jump" => Ok(Self::BaseJump),
            "walk" => Ok(Self::Walk),
            "blink" => Ok(Self::Blink),
            other => Err(format!(
                "--rest-pilot {other:?}: expected donut, carl_door, base_jump, walk or blink"
            )),
        }
    }

    /// Whether the pilot plays in `room`: the refuge pilots need the staff door and the
    /// shelf, the blink the aftermath's lane, the walk any quiet room.
    pub const fn fits(self, room: RestRoom) -> bool {
        match self {
            Self::Donut | Self::CarlDoor | Self::BaseJump => matches!(room, RestRoom::Refuge),
            Self::Walk => true,
            Self::Blink => matches!(room, RestRoom::Aftermath),
        }
    }
}

/// The pure shelf rule: Donut backs off to the left of the takeoff column, runs right and
/// jumps (held while airborne), then walks to the exit once the door is open.
pub fn shelf_input(x: f32, grounded: bool, door_open: bool, takeoff: f32, exit_x: f32) -> RawInput {
    let mut input = RawInput::NONE;
    if door_open {
        input.move_x = walk(x, exit_x);
    } else if !grounded {
        input.move_x = 1.0;
        input.jump = true;
    } else if x > takeoff + 4.0 {
        input.move_x = walk(x, takeoff - 60.0);
    } else if x < takeoff - 90.0 {
        input.move_x = 1.0;
    } else {
        input.move_x = 1.0;
        input.jump = x >= takeoff;
    }
    input
}

/// The next input of a [`Pilot`], a pure function of the finished state. `exit_x` is the
/// middle of the room's exit (the walk and the blink pilots read it).
pub fn pilot(kind: Pilot, combat: &CombatState, scene: &SceneSim, exit_x: f32) -> RawInput {
    let donut = scene.world.body(HeroId::Donut);
    let carl = scene.world.body(HeroId::Carl);
    let open = combat
        .env()
        .and_then(|env| env.surface_locks_open().first().copied())
        .unwrap_or(false);
    let swap_now = scene.world.tick == 0;
    match kind {
        Pilot::Donut if swap_now => swap(),
        Pilot::BaseJump if swap_now => swap(),
        Pilot::Blink if swap_now => swap(),
        Pilot::Walk => RawInput {
            move_x: walk(carl.x, exit_x),
            ..RawInput::NONE
        },
        Pilot::Blink => {
            // Walk to the lane's near mark, settle, press once (the blink carries her out of
            // the window, so one press lands); past the lane, walk to the exit.
            let near = f01_rest::BLINK_LANE_X.0 as f32;
            let mut input = RawInput::NONE;
            if donut.x > near + 4.0 {
                input.move_x = walk(donut.x, exit_x);
            } else {
                input.move_x = walk(donut.x, near);
                input.ability = input.move_x == 0.0 && donut.grounded && scene.world.tick % 20 == 0;
            }
            input
        }
        Pilot::Donut => shelf_input(donut.x, donut.grounded, open, SHELF_TAKEOFF_X, 1536.0),
        Pilot::CarlDoor => RawInput {
            move_x: walk(carl.x, DOOR_WAIT_X),
            ..RawInput::NONE
        },
        Pilot::BaseJump => {
            // Jump from before the box (x < 1200): the launch is the base one.
            let mut input = RawInput {
                move_x: 1.0,
                ..RawInput::NONE
            };
            input.jump = !donut.grounded || (1100.0..1190.0).contains(&donut.x);
            input
        }
    }
}

/// Takeoff columns a base Donut tries at the shelf (she never lands: 104 px < 128 px).
const BASE_TAKEOFFS: &[f32] = &[1100.0, 1160.0, 1200.0, 1245.0, 1280.0, 1230.0];

/// Run a case to its end and return the sim (its combat state carries the exit ledger).
pub fn run_sim(root: &Path, case: Case) -> Result<RestSim, String> {
    drive(root, case)
}

/// Run a case to its end (the exit, or [`MAX_TICKS`]).
pub fn run(root: &Path, case: Case) -> Result<Outcome, String> {
    drive(root, case).map(|sim| Outcome::of(&sim))
}

fn drive(root: &Path, case: Case) -> Result<RestSim, String> {
    let (carl, donut) = story();
    let (carl_off, donut_off) = (CarlProgFlags::default(), DonutProgFlags::default());
    let solo_bot = Party::Solo { uncontrolled: BOT };
    match case {
        Case::SoloCarlSwap | Case::SoloDonut => {
            let mut sim = RestSim::new(root, 360, carl, donut, solo_bot)?;
            let ex = exit_x(&sim);
            let mut script = DonutScript::default();
            let mut swapped = case == Case::SoloDonut;
            let mut at_door = 0u32;
            if swapped {
                sim.step([swap(), RawInput::NONE]);
            }
            while sim.tick() < MAX_TICKS && !sim.complete() {
                let input = if swapped {
                    script.input(&sim, ex)
                } else {
                    // Carl walks right, stops at the door, waits, swaps.
                    let x = sim.x_of(HeroId::Carl);
                    let mut input = RawInput {
                        move_x: walk(x, DOOR_WAIT_X),
                        ..RawInput::NONE
                    };
                    if (x - DOOR_WAIT_X).abs() <= 3.0 {
                        at_door += 1;
                        if at_door > 100 {
                            input = swap();
                            swapped = true;
                        }
                    }
                    input
                };
                sim.step([input, RawInput::NONE]);
            }
            Ok(sim)
        }
        Case::TwoPlayer => {
            let mut sim = RestSim::new(root, 360, carl, donut, Party::TwoPlayer)?;
            let ex = exit_x(&sim);
            let mut script = DonutScript::default();
            while sim.tick() < MAX_TICKS && !sim.complete() {
                let cx = sim.x_of(HeroId::Carl);
                let carl_in = RawInput {
                    move_x: if sim.door_open() {
                        walk(cx, ex)
                    } else {
                        walk(cx, DOOR_WAIT_X)
                    },
                    ..RawInput::NONE
                };
                let donut_in = script.input(&sim, ex);
                sim.step([carl_in, donut_in]);
            }
            Ok(sim)
        }
        Case::SoloCompanionCrossing => {
            let mut sim = RestSim::new(root, 360, carl, donut, Party::Solo { uncontrolled: FREE })?;
            let ex = exit_x(&sim);
            let mut script = DonutScript::default();
            // REST-IMP-7: the free companion did not open the door by herself, so the script
            // adds a human swap at the closed door (as the hint asks) and a swap back.
            // 0: Carl walks; 1: Donut active (swapped at the closed door, a human after the
            // hint); 2: swapped back to Carl once the door is open.
            let mut phase = 0u8;
            let mut waited = 0u32;
            while sim.tick() < MAX_TICKS && !sim.complete() {
                let input = match phase {
                    0 => {
                        let x = sim.x_of(HeroId::Carl);
                        let mut input = RawInput {
                            move_x: walk(x, ex),
                            ..RawInput::NONE
                        };
                        // Blocked by the closed door: a human swaps after the hint.
                        if !sim.door_open() && x >= DOOR_WAIT_X - 4.0 {
                            waited += 1;
                            if waited > 200 {
                                input = swap();
                                phase = 1;
                            }
                        }
                        input
                    }
                    1 => {
                        if sim.door_open() {
                            phase = 2;
                            swap()
                        } else {
                            script.input(&sim, ex)
                        }
                    }
                    _ => RawInput {
                        move_x: walk(sim.x_of(HeroId::Carl), ex),
                        ..RawInput::NONE
                    },
                };
                sim.step([input, RawInput::NONE]);
            }
            Ok(sim)
        }
        Case::GrantOnceAtTheBox => {
            let mut sim = RestSim::new(root, 360, carl, donut, solo_bot)?;
            sim.step([swap(), RawInput::NONE]);
            // Back and forth across the column x 1200..1240, three times.
            let mut target = 1260.0;
            let mut legs = 0;
            while sim.tick() < 1200 && legs < 6 {
                let x = sim.x_of(HeroId::Donut);
                if (x - target).abs() <= 4.0 {
                    legs += 1;
                    target = if target > 1220.0 { 1160.0 } else { 1260.0 };
                }
                sim.step([
                    RawInput {
                        move_x: walk(x, target),
                        ..RawInput::NONE
                    },
                    RawInput::NONE,
                ]);
            }
            Ok(sim)
        }
        Case::BaseDonutCannotOpen => {
            let mut sim = RestSim::new(root, 360, carl_off, donut_off, solo_bot)?;
            sim.step([swap(), RawInput::NONE]);
            let ex = exit_x(&sim);
            // Donut runs at the shelf from a spread of takeoff columns, again and again.
            let mut script = DonutScript::base_probe();
            while sim.tick() < 1500 && !sim.complete() {
                let input = script.input(&sim, ex);
                sim.step([input, RawInput::NONE]);
            }
            Ok(sim)
        }
        Case::RetryKeepsTheGrant => retry_keeps_the_grant(root),
        Case::CampCrossingSolo => {
            let mut sim = RestSim::new_in(root, RestRoom::Camp, 360, carl, donut, solo_bot, None)?;
            walk_to_the_exit(&mut sim, 1200);
            Ok(sim)
        }
        Case::CampCrossing2P => {
            let mut sim = RestSim::new_in(
                root,
                RestRoom::Camp,
                360,
                carl,
                donut,
                Party::TwoPlayer,
                None,
            )?;
            let ex = exit_x(&sim);
            while sim.tick() < 1200 && !sim.complete() {
                let walk_in = |x: f32| RawInput {
                    move_x: walk(x, ex),
                    ..RawInput::NONE
                };
                let inputs = [
                    walk_in(sim.x_of(HeroId::Carl)),
                    walk_in(sim.x_of(HeroId::Donut)),
                ];
                sim.step(inputs);
            }
            Ok(sim)
        }
        Case::AftermathNoDuplicateUnlock => {
            let carry = post_juicer_snapshot(root)?;
            let mut sim = RestSim::new_in(
                root,
                RestRoom::Aftermath,
                360,
                carl,
                donut,
                solo_bot,
                Some(&carry),
            )?;
            walk_to_the_exit(&mut sim, 1200);
            Ok(sim)
        }
        Case::AftermathBlinkLane => {
            let mut sim =
                RestSim::new_in(root, RestRoom::Aftermath, 360, carl, donut, solo_bot, None)?;
            let ex = exit_x(&sim);
            sim.step([swap(), RawInput::NONE]);
            let mut pressed = false;
            let mut settled = 0u32;
            while sim.tick() < 1500 && !sim.complete() {
                let body = sim.scene.world.body(HeroId::Donut);
                let mut input = RawInput::NONE;
                if pressed {
                    // Walk on to the exit once the blink has landed.
                    input.move_x = walk(body.x, ex);
                } else {
                    input.move_x = walk(body.x, f01_rest::BLINK_LANE_X.0 as f32);
                    if input.move_x == 0.0 && body.grounded {
                        // Stopped at the lane's near mark, facing right: let her settle, press.
                        settled += 1;
                        if settled > 20 {
                            input.ability = true;
                            pressed = true;
                        }
                    }
                }
                sim.step([input, RawInput::NONE]);
            }
            Ok(sim)
        }
        Case::AftermathCrossingBaseMoves => {
            let mut sim = RestSim::new_in(
                root,
                RestRoom::Aftermath,
                360,
                carl_off,
                donut_off,
                solo_bot,
                None,
            )?;
            walk_to_the_exit(&mut sim, 1200);
            Ok(sim)
        }
    }
}

/// Solo Carl walks right to the exit's middle (the other hero is the follow-and-jump bot).
fn walk_to_the_exit(sim: &mut RestSim, budget: u64) {
    let ex = exit_x(sim);
    while sim.tick() < budget && !sim.complete() {
        let input = RawInput {
            move_x: walk(sim.x_of(HeroId::Carl), ex),
            ..RawInput::NONE
        };
        sim.step([input, RawInput::NONE]);
    }
}

/// A post-Juicer ledger the way the run builds it: Light on Your Feet from a refuge
/// crossing, then (each room carries the ledger on) the Hoarder's dynamite and the Juicer's
/// fist and blink. A snapshot replaces the ledger it is carried into, so the rooms' snapshots
/// are united here in the order the run meets them: the held set sorted and deduplicated,
/// the receipts in the order they were earned (every capability with its receipt, so a
/// second grant would show).
pub fn post_juicer_snapshot(root: &Path) -> Result<LedgerSnapshot, String> {
    let refuge = drive(root, Case::SoloDonut)?
        .combat
        .story_snapshot()
        .ok_or("the refuge ran no story")?;
    let hoarder = crate::f01_late_fixtures::hoarder_defeat(root)?
        .snapshot()
        .ok_or("the Hoarder ran no story")?;
    // The Juicer room carries the Hoarder's ledger on: its snapshot holds both rooms' grants.
    let juicer = crate::f01_late_fixtures::juicer_relaunch(root, Loadout::Story, Some(&hoarder))?
        .combat
        .story_snapshot()
        .ok_or("the Juicer ran no story")?;
    let mut held: Vec<_> = refuge.held.iter().chain(&juicer.held).cloned().collect();
    held.sort_by(|a, b| (a.hero, &a.capability).cmp(&(b.hero, &b.capability)));
    held.dedup();
    let mut receipts = refuge.receipts.clone();
    receipts.extend(
        juicer
            .receipts
            .iter()
            .filter(|r| !receipts.iter().any(|seen| seen.capability == r.capability))
            .cloned()
            .collect::<Vec<_>>(),
    );
    let merged = LedgerSnapshot {
        version: refuge.version,
        held,
        receipts,
    };
    // The aftermath accepts it as is (every id and hero checked by the ledger's restore).
    let (carl, donut) = story();
    RestSim::new_in(
        root,
        RestRoom::Aftermath,
        360,
        carl,
        donut,
        Party::Solo { uncontrolled: BOT },
        Some(&merged),
    )?;
    Ok(merged)
}

/// The retry case: a crossing, a checkpoint respawn (keeps the grant and the open door),
/// a fixture restart (drops the boost, closes the door, re-arms the trigger), a second
/// crossing that grants once more and opens the door once more.
fn retry_keeps_the_grant(root: &Path) -> Result<RestSim, String> {
    let (carl, donut) = story();
    let mut sim = RestSim::new(root, 360, carl, donut, Party::Solo { uncontrolled: BOT })?;
    let ex = exit_x(&sim);
    sim.step([swap(), RawInput::NONE]);
    let mut script = DonutScript::default();
    while sim.tick() < MAX_TICKS && !sim.door_open() {
        let input = script.input(&sim, ex);
        sim.step([input, RawInput::NONE]);
    }
    let boost = crate::donut_prog::JUMP_REACH_BOOST;
    if !sim.combat.story_holds(boost) || sim.grants().len() != 1 {
        return Err("the first crossing did not grant once".into());
    }
    // A checkpoint respawn: the boost and the open door stay, no second receipt.
    sim.combat.respawn(&mut sim.scene);
    sim.combat.take_respawned();
    sim.party.restart();
    if !sim.combat.story_holds(boost)
        || !sim.door_open()
        || sim.grants().len() != 1
        || sim.combat.story_zone_armed(1)
    {
        return Err("a checkpoint respawn lost the grant, the door or re-armed the box".into());
    }
    // A fixture restart: the room's start state (no boost, closed door, armed box).
    sim.combat.restart(&mut sim.scene);
    sim.party.restart();
    if sim.combat.story_holds(boost) || sim.door_open() || !sim.combat.story_zone_armed(1) {
        return Err("a fixture restart kept the boost or the open door".into());
    }
    // The second crossing grants once and opens the door once more.
    sim.step([swap(), RawInput::NONE]);
    let mut script = DonutScript::default();
    while sim.tick() < MAX_TICKS && !sim.complete() {
        let input = script.input(&sim, ex);
        sim.step([input, RawInput::NONE]);
    }
    Ok(sim)
}

/// The pinned final state hash of each case (new constants, REST-WIRE: the refuge's seven
/// from the first PR, the camp's and the aftermath's five from the second).
pub const PINNED: [(&str, u64); 12] = [
    ("rest_refuge_solo_carl_swap", 0x7fe5_e491_2b4f_3e00),
    ("rest_refuge_solo_donut", 0x115a_4858_7b20_34ed),
    ("rest_refuge_2p", 0x34ae_daa9_3266_dce4),
    ("rest_refuge_solo_companion_crossing", 0x028f_78b3_a3b2_8847),
    ("rest_refuge_grant_once_at_the_box", 0x747c_440c_3df2_923f),
    ("rest_refuge_base_donut_cannot_open", 0x82d7_00ff_7896_5938),
    ("rest_refuge_retry_keeps_the_grant", 0xa59f_a59b_24f9_3285),
    ("rest_camp_crossing_solo", 0x6f22_985f_dd47_69bc),
    ("rest_camp_crossing_2p", 0x5a4e_d69b_33c4_d6af),
    ("rest_aftermath_no_duplicate_unlock", 0x3d31_2cbe_23e3_d9af),
    ("rest_aftermath_blink_lane", 0xe501_b168_4cb9_4004),
    ("rest_aftermath_crossing_base_moves", 0x9873_c442_4d91_916b),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::f01_rest::REWARD_ZONE_X;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn outcome(case: Case) -> Outcome {
        run(&root(), case).unwrap_or_else(|e| panic!("{}: {e}", case.name()))
    }

    /// The shelf's top (layout y) and span.
    fn shelf() -> crate::camera_lab::room::IRect {
        RestSim::new(
            &root(),
            360,
            CarlProgFlags::default(),
            DonutProgFlags::default(),
            Party::Solo { uncontrolled: BOT },
        )
        .unwrap()
        .run
        .loaded
        .room
        .platforms[2]
            .rect
    }

    /// Every case runs twice to an identical outcome and final hash, pinned here.
    #[test]
    fn every_case_replays_to_the_same_pinned_hash() {
        for case in Case::ALL {
            let a = outcome(case);
            assert_eq!(a, outcome(case), "{}: not deterministic", case.name());
            let pinned = PINNED
                .iter()
                .find(|(name, _)| *name == case.name())
                .unwrap_or_else(|| panic!("{} is not pinned", case.name()))
                .1;
            assert_eq!(a.hash, pinned, "{}: {:#018x}", case.name(), a.hash);
        }
        assert_eq!(PINNED.len(), Case::ALL.len());
    }

    /// Evidence helper (ignored): one line per case for `docs/evidence/rest-wire`.
    #[test]
    #[ignore = "evidence helper: prints one line per case"]
    fn print_the_case_outcomes() {
        for case in Case::ALL {
            let o = outcome(case);
            println!(
                "REST_FIXTURE name={} ticks={} exit_tick={:?} grants={:?} doors={:?} damage={} receipts={} final_hash={:#018x}",
                case.name(),
                o.ticks,
                o.exit_tick,
                o.grants,
                o.doors,
                o.damage,
                o.receipts,
                o.hash
            );
        }
    }

    /// Evidence helper (ignored): Donut's `(tick, x, y, grounded)` under each evidence pilot,
    /// to pick the ticks of the before/after strip.
    #[test]
    #[ignore = "evidence helper: prints Donut's trace under the evidence pilots"]
    fn print_the_pilot_traces() {
        let (carl, donut) = story();
        for kind in [Pilot::Donut, Pilot::BaseJump] {
            let mut sim =
                RestSim::new(&root(), 360, carl, donut, Party::Solo { uncontrolled: BOT }).unwrap();
            while sim.tick() < 420 && !sim.complete() {
                let input = pilot(kind, &sim.combat, &sim.scene, 1536.0);
                sim.step([input, RawInput::NONE]);
                let b = sim.scene.world.body(HeroId::Donut);
                if sim.tick().is_multiple_of(4) && sim.tick() >= 240 {
                    println!(
                        "TRACE {kind:?} tick={} x={:.1} y={:.1} grounded={} door={}",
                        sim.tick(),
                        b.x,
                        b.y - crate::prototype::HERO_HALF_HEIGHT,
                        b.grounded,
                        sim.door_open()
                    );
                }
            }
        }
    }

    /// The pure evidence pilot crosses the room as well: the box grants once, the shelf opens the
    /// door, the exit is reached. The base jump does not reach the shelf.
    #[test]
    fn the_evidence_pilots_cross_and_fail_as_named() {
        let (carl, donut) = story();
        let run_pilot = |kind: Pilot, ticks: u64| {
            let mut sim =
                RestSim::new(&root(), 360, carl, donut, Party::Solo { uncontrolled: BOT }).unwrap();
            while sim.tick() < ticks && !sim.complete() {
                let input = pilot(kind, &sim.combat, &sim.scene, 1536.0);
                sim.step([input, RawInput::NONE]);
            }
            sim
        };
        let crossing = run_pilot(Pilot::Donut, 900);
        assert!(crossing.complete(), "the Donut pilot reaches the exit");
        assert_eq!(crossing.grants().len(), 1);
        assert_eq!(crossing.door_events().len(), 1);
        let waiting = run_pilot(Pilot::CarlDoor, 700);
        assert!(!waiting.door_open() && !waiting.complete());
        assert!(
            waiting.x_of(HeroId::Carl) > DOOR_WAIT_X - 4.0,
            "Carl waits at the door"
        );
        let base = run_pilot(Pilot::BaseJump, 700);
        assert!(
            base.door_events().is_empty(),
            "a base jump never opens the door"
        );
        assert!(Pilot::parse("nope").is_err());
    }

    /// The common crossing: exactly one grant of Donut's boost with her pivot inside the
    /// reward column, the door opening on the tick she lands on the shelf, the exit
    /// reached, no damage and one receipt.
    fn assert_crossing(case: Case) -> Outcome {
        let o = outcome(case);
        let name = case.name();
        assert_eq!(o.grants.len(), 1, "{name}: {:?}", o.grants);
        let (grant_tick, hero, capability) = o.grants[0];
        assert_eq!((hero, capability), (1, "jump_reach_boost"), "{name}");
        assert_eq!(o.grant_x.len(), 1);
        assert!(
            (REWARD_ZONE_X.0 as f32..REWARD_ZONE_X.1 as f32).contains(&o.grant_x[0]),
            "{name}: the grant fires with her pivot in the column, at {}",
            o.grant_x[0]
        );
        assert_eq!(o.doors.len(), 1, "{name}: {:?}", o.doors);
        assert_eq!(o.doors[0].1, 1, "{name}: Donut opened it");
        assert!(o.doors[0].0 > grant_tick, "{name}: the box comes first");
        let (x, foot, grounded) = o.door_stance[0];
        let shelf = shelf();
        assert!(
            grounded && foot == shelf.y && (shelf.x as f32..shelf.right() as f32).contains(&x),
            "{name}: the door opens on the tick she stands on the shelf: {x} {foot}"
        );
        assert!(o.exit_tick.is_some(), "{name}: the exit is reached");
        assert_eq!(o.damage, 0, "{name}: no damage event");
        assert_eq!(o.receipts, 1, "{name}");
        o
    }

    #[test]
    fn rest_refuge_solo_carl_swap() {
        assert_crossing(Case::SoloCarlSwap);
    }

    #[test]
    fn rest_refuge_solo_donut() {
        let o = assert_crossing(Case::SoloDonut);
        assert!(o.exit_tick.unwrap() < outcome(Case::SoloCarlSwap).exit_tick.unwrap());
    }

    /// 2P: the same, with both heroes inside the camera at the door.
    #[test]
    fn rest_refuge_2p() {
        let o = assert_crossing(Case::TwoPlayer);
        assert_eq!(
            o.both_in_view,
            Some(true),
            "both heroes inside the camera at the door"
        );
    }

    /// The default companion (`cf02_free`): the crossing is completed (the human swaps
    /// at the closed door after the hint, then back to Carl), one grant, and Donut is
    /// never left on the shelf when Carl exits.
    #[test]
    fn rest_refuge_solo_companion_crossing() {
        let o = assert_crossing(Case::SoloCompanionCrossing);
        assert!(
            !o.donut_on_shelf,
            "Donut is not left on the shelf when Carl exits"
        );
    }

    /// Donut walks back and forth across the column: one grant, one receipt, no door.
    #[test]
    fn rest_refuge_grant_once_at_the_box() {
        let o = outcome(Case::GrantOnceAtTheBox);
        assert_eq!(o.grants.len(), 1);
        assert_eq!(o.receipts, 1);
        assert!(o.doors.is_empty() && o.exit_tick.is_none());
    }

    /// With the trigger not armed (no story flags) base Donut never lands on the shelf:
    /// no grant, no door, no exit. The room is sealed by design.
    #[test]
    fn rest_refuge_base_donut_cannot_open() {
        let o = outcome(Case::BaseDonutCannotOpen);
        assert!(o.grants.is_empty(), "{:?}", o.grants);
        assert!(o.doors.is_empty() && o.exit_tick.is_none());
        assert_eq!(o.ticks, 1500);
        assert_eq!(o.receipts, 0);
    }

    /// The camp: no grant, no damage, no door, the exit inside 1200 ticks, and the ledger is
    /// the post-Hoarder pin's (dynamite and Light on Your Feet held, no receipt), solo and 2P.
    #[test]
    fn rest_camp_crossings_reach_the_exit_and_hold_the_pin() {
        let solo = outcome(Case::CampCrossingSolo);
        let duo = outcome(Case::CampCrossing2P);
        for (name, o) in [("solo", &solo), ("2p", &duo)] {
            assert!(
                o.exit_tick.is_some_and(|t| t <= 1200),
                "{name}: {:?}",
                o.exit_tick
            );
            assert!(o.grants.is_empty() && o.doors.is_empty(), "{name}");
            assert_eq!(o.damage, 0, "{name}");
            assert!(o.ledger_unchanged, "{name}: a quiet room grants nothing");
            assert_eq!(
                o.receipts, 0,
                "{name}: the pin holds, nothing was earned here"
            );
        }
        let (carl, donut) = story();
        let sim = RestSim::new_in(
            &root(),
            RestRoom::Camp,
            360,
            carl,
            donut,
            Party::Solo { uncontrolled: BOT },
            None,
        )
        .unwrap();
        let mut held: Vec<(u8, String)> = sim
            .start_snapshot
            .as_ref()
            .expect("the camp runs a story runtime")
            .held
            .iter()
            .map(|h| (h.hero, h.capability.clone()))
            .collect();
        held.sort();
        assert_eq!(
            held,
            [
                (0, "special_thrown_charge".to_owned()),
                (1, "jump_reach_boost".to_owned())
            ],
            "the post-Hoarder pin"
        );
    }

    /// The aftermath entered with a post-Juicer ledger (four capabilities, four receipts)
    /// grants nothing and leaves the ledger as it came: no duplicate unlock (REST-AD3).
    #[test]
    fn rest_aftermath_no_duplicate_unlock() {
        let carry = post_juicer_snapshot(&root()).unwrap();
        assert_eq!(carry.held.len(), 4, "{:?}", carry.held);
        assert_eq!(carry.receipts.len(), 4, "{:?}", carry.receipts);
        let o = outcome(Case::AftermathNoDuplicateUnlock);
        assert!(o.exit_tick.is_some_and(|t| t <= 1200), "{:?}", o.exit_tick);
        assert!(o.grants.is_empty(), "{:?}", o.grants);
        assert!(o.ledger_unchanged, "the exit snapshot equals the carry");
        assert_eq!(o.receipts, 4);
        assert_eq!(o.damage, 0);
    }

    /// Donut blinks across the 112 px lane: the blink is accepted and resolves 112 px on, no
    /// rejection, fizzle or cancel, mana spent by the normal rule, the heroes still walk on
    /// to the exit (no solid in or after the lane).
    #[test]
    fn rest_aftermath_blink_lane() {
        let o = outcome(Case::AftermathBlinkLane);
        assert_eq!(o.blinks.len(), 1, "{:?}", o.blinks);
        assert_eq!(o.blink_failures, 0);
        let (_, from, to) = o.blinks[0];
        assert!(
            (f01_rest::BLINK_LANE_X.0 - 8..=f01_rest::BLINK_LANE_X.0 + 8).contains(&from),
            "she blinks from the lane's near mark: {from}"
        );
        assert_eq!(
            to - from,
            112,
            "the whole 112 px, collision-safe: {from} -> {to}"
        );
        assert!(to <= f01_rest::BLINK_LANE_X.1 + 8);
        assert_eq!(o.spends.len(), 1, "{:?}", o.spends);
        assert_eq!(
            o.spends[0].1, 6,
            "two charges of eight (the Puddle Jumper cost)"
        );
        assert!(o.exit_tick.is_some(), "no solid in or after the lane");
        assert_eq!(o.damage, 0);
    }

    /// Base moves only (no story flag): the aftermath is crossed by walking.
    #[test]
    fn rest_aftermath_crossing_base_moves() {
        let o = outcome(Case::AftermathCrossingBaseMoves);
        assert!(o.exit_tick.is_some_and(|t| t <= 1200), "{:?}", o.exit_tick);
        assert!(o.grants.is_empty() && o.blinks.is_empty());
        assert_eq!(o.damage, 0);
    }

    /// A respawn keeps the grant and the open door; a restart drops both and re-arms the
    /// box; the second crossing grants once and opens the door once (the event log keeps
    /// both attempts, so two of each in total).
    #[test]
    fn rest_refuge_retry_keeps_the_grant() {
        let o = outcome(Case::RetryKeepsTheGrant);
        assert_eq!(o.grants.len(), 2, "one per attempt: {:?}", o.grants);
        assert_eq!(o.doors.len(), 2);
        assert_eq!(
            o.grants[0], o.grants[1],
            "the same tick in the same crossing"
        );
        assert!(o.exit_tick.is_some());
        assert_eq!(
            o.receipts, 1,
            "a restart cleared the first attempt's receipt"
        );
    }
}
