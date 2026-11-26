//! ACT1-A scenario fixtures (`docs/01-design/floor-01-act-01-integration.md` section 10,
//! ACT1-F01..F08): the shore (`act1_shore`) and the guild (`act1_guild`) driven by scripted party
//! inputs on the launched room, deterministic, seeded and fixed-tick. Each fixture runs twice
//! and must end on an identical state hash; the final hashes are pinned here (new constants,
//! nothing moves). Bevy-free.
//!
//! The scripted **solo** fixtures pin the uncontrolled hero to `--companion off` (the
//! follow-and-jump bot), except where a row says "default companion" (the `cf02_free`
//! companion). The shore fixtures run with no story flag (the rooms are story-less, a Story
//! run only differs by carrying an empty ledger); the guild's run under the Story loadout, the
//! guild's retry and restart fixtures included.

use crate::camera_lab::room::IRect;
use crate::carl_prog::CarlProgFlags;
use crate::combat::{CombatEvent, DefeatCause};
use crate::combat_scene::{CombatState, EnvSolidKind};
use crate::companion::profile::ProfileSetId;
use crate::corridor::SceneSim;
use crate::donut_mana::ManaEvent;
use crate::donut_prog::DonutProgFlags;
use crate::f01_act1::{self, Act1Room, Act1Run};
use crate::f01_late::Loadout;
use crate::input::{RawInput, SlotInputs};
use crate::party::{HeroId, PROVISIONAL_TUNING, PadId, PartyState, PartyTuning, UncontrolledHero};
use std::path::Path;

/// The fixed-tick budget of every fixture.
pub const MAX_TICKS: u64 = 3600;
/// The seed of the fixtures' `--cf02-seed`.
pub const SEED: u64 = 0x0F01_5EED;
/// The column Carl strikes the rotten shore from: the strike column (local 367) minus the
/// jab's 19 px reach ([`crate::combat_scene::CP03_SHORE_KILL`]).
pub const STRIKE_FROM_X: f32 = 348.0;

/// How the party of a fixture is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Party {
    /// One human (slot 0) drives the active hero; the other hero is `uncontrolled`.
    Solo { uncontrolled: UncontrolledHero },
    /// P1 Carl, P2 Donut.
    TwoPlayer,
}

/// The follow-and-jump bot (`--companion off`).
pub const BOT: UncontrolledHero = UncontrolledHero::FollowAndJumpBot;
/// The default of a room scene: `cf02_free`.
pub const FREE: UncontrolledHero = UncontrolledHero::Companion(ProfileSetId::Cf02Free);

/// A launched Act I room with a party, stepped the way the interactive scene steps it.
pub struct Act1Sim {
    pub run: Act1Run,
    pub scene: SceneSim,
    pub combat: CombatState,
    pub party: PartyState,
}

impl Act1Sim {
    pub fn new(
        root: &Path,
        room: Act1Room,
        carl: CarlProgFlags,
        donut: DonutProgFlags,
        party: Party,
    ) -> Result<Self, String> {
        let run = f01_act1::launch(root, room)?;
        let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x)?;
        let combat = f01_act1::combat_for(&run, &mut scene, carl, donut, Some(SEED))?;
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
        })
    }

    /// One tick, the order of the interactive scene: the swap, the party routing, the scene
    /// drive and the combat step.
    pub fn step(&mut self, slots: SlotInputs) {
        let hurt = self.combat.hurt_ticks_of(self.party.controlled_hero()) > 0;
        self.party.request_swap(slots[0].swap, hurt);
        let hero = self.party.hero_inputs(slots);
        let drive = self.party.drive(true);
        self.scene.set_drive(drive);
        let restarts = self.combat.restarts;
        self.combat.step_party(&mut self.scene, hero);
        let respawned = self.combat.take_respawned();
        if self.combat.restarts != restarts || respawned {
            self.party.restart();
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

    /// The exit's middle column.
    pub fn exit_x(&self) -> f32 {
        let e = self.run.loaded.encounter.exit;
        (e.x + e.w / 2) as f32
    }

    /// A checkpoint respawn (the party's defeat retry): the heroes stand at the checkpoint,
    /// the living goblins are restored.
    pub fn respawn(&mut self) {
        self.combat.respawn(&mut self.scene);
        self.combat.take_respawned();
        self.party.restart();
    }

    /// A fixture restart (R / Select): the room's declared start state.
    pub fn restart(&mut self) {
        self.combat.restart(&mut self.scene);
        self.party.restart();
    }

    /// Whether the guild door's world solid stands now (it collides with heroes).
    pub fn door_stands(&self) -> Option<bool> {
        let env = self.combat.env()?;
        let range = env.solid_range();
        env.solids()
            .iter()
            .position(|s| matches!(s.kind, EnvSolidKind::ManaLock { .. }))
            .and_then(|i| self.scene.world.solids.get(range.start + i))
            .map(|b| b.intact)
    }

    /// The guild door's rectangle, if the room has one.
    pub fn door(&self) -> Option<IRect> {
        self.run.door()
    }

    /// `(tick, hero)` of every `UnlockGranted`, in order.
    pub fn unlocks(&self) -> Vec<(u64, u8)> {
        self.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Mana(ManaEvent::UnlockGranted { tick, hero, .. }) => {
                    Some((tick, hero))
                }
                _ => None,
            })
            .collect()
    }

    /// The tick of each door opening, in order.
    pub fn door_events(&self) -> Vec<u64> {
        self.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::ManaLockOpened { tick, .. } => Some(tick),
                _ => None,
            })
            .collect()
    }

    /// Donut's mana spends (she casts): empty before the biscuit.
    pub fn casts(&self) -> usize {
        self.combat
            .events
            .iter()
            .filter(|e| matches!(e, CombatEvent::Mana(ManaEvent::ManaSpent { .. })))
            .count()
    }

    /// `(target, cause)` of every enemy defeat, in order.
    pub fn defeats(&self) -> Vec<(u32, DefeatCause)> {
        self.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::EnemyDefeated { target, cause, .. } => Some((target, cause)),
                _ => None,
            })
            .collect()
    }

    /// Both shore goblins are down.
    pub fn roster_cleared(&self) -> bool {
        self.combat.roster_cleared()
    }
}

/// Walk `from` toward `to`: the `move_x` that closes the gap (zero inside 3 px).
pub fn walk(from: f32, to: f32) -> f32 {
    let d = to - from;
    if d.abs() <= 3.0 { 0.0 } else { d.signum() }
}

fn walk_in(from: f32, to: f32) -> RawInput {
    RawInput {
        move_x: walk(from, to),
        ..RawInput::NONE
    }
}

fn swap() -> RawInput {
    RawInput {
        swap: true,
        ..RawInput::NONE
    }
}

/// The Story loadout's flags (the guild runs under them).
pub fn story() -> (CarlProgFlags, DonutProgFlags) {
    Loadout::Story.flags()
}

/// The jab's reach in px: Carl closes inside it and presses.
const JAB_REACH: f32 = 30.0;
/// Carl presses the jab every this many ticks.
const PRESS_CADENCE: u64 = 4;

/// Carl fights on the lane: close on the nearest living goblin, jab it on a fixed cadence, and
/// with nothing left alive walk to `exit_x` (the Act I rooms have no llama and no obstacle on
/// the lane, so no jump is needed). A pure function of the finished state.
pub fn clear_and_exit(combat: &CombatState, scene: &SceneSim, exit_x: f32) -> RawInput {
    let carl = scene.world.body(HeroId::Carl);
    let press = (scene.world.tick + 1).is_multiple_of(PRESS_CADENCE);
    let goblin = combat
        .enemies
        .iter()
        .filter(|g| g.alive())
        .min_by(|a, b| (a.x - carl.x).abs().total_cmp(&(b.x - carl.x).abs()));
    let Some(goblin) = goblin else {
        return walk_in(carl.x, exit_x);
    };
    let mut raw = walk_in(carl.x, goblin.x);
    if (goblin.x - carl.x).abs() <= JAB_REACH {
        raw.move_x = (goblin.x - carl.x).signum();
        raw.attack = press;
    }
    raw
}

/// The second human of a scripted co-op run: Donut keeps 48 px behind Carl.
pub fn follow(scene: &SceneSim) -> RawInput {
    let carl = scene.world.body(HeroId::Carl);
    let goal = carl.x - 48.0 * f32::from(scene.world.hero.facing);
    walk_in(scene.world.body(HeroId::Donut).x, goal)
}

/// The evidence pilots of the Act I rooms (`floor01_scene --act1-pilot MODE`): a scripted run
/// that stands a hero in every beat of the room for the captures. Each is a pure function of
/// the finished state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pilot {
    /// Either room: Carl walks right to the exit (the shore's goblins make a fight of it).
    Walk,
    /// The guild: swap to Donut on the first tick, walk right past the guide and through the
    /// door.
    Donut,
    /// The shore: Carl strikes the rotten shore from its column, then fights on and walks to
    /// the exit.
    Strike,
}

impl Pilot {
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "walk" => Ok(Self::Walk),
            "donut" => Ok(Self::Donut),
            "strike" => Ok(Self::Strike),
            other => Err(format!(
                "--act1-pilot {other:?}: expected walk, donut or strike"
            )),
        }
    }

    /// Whether the pilot plays in `room`.
    pub const fn fits(self, room: Act1Room) -> bool {
        match self {
            Self::Walk => true,
            Self::Donut => matches!(room, Act1Room::Guild),
            Self::Strike => matches!(room, Act1Room::Shore),
        }
    }
}

/// The next input of a [`Pilot`], a pure function of the finished state. `exit_x` is the
/// middle of the room's exit.
pub fn pilot(kind: Pilot, combat: &CombatState, scene: &SceneSim, exit_x: f32) -> RawInput {
    let donut = scene.world.body(HeroId::Donut);
    match kind {
        Pilot::Donut if scene.world.tick == 0 => swap(),
        Pilot::Donut => walk_in(donut.x, exit_x),
        Pilot::Walk => clear_and_exit(combat, scene, exit_x),
        Pilot::Strike => strike_input(combat, scene, exit_x),
    }
}

/// Carl walks to the strike column, jabs the post three times (a jab per 40 ticks) and then
/// fights and walks on.
fn strike_input(combat: &CombatState, scene: &SceneSim, exit_x: f32) -> RawInput {
    let carl = scene.world.body(HeroId::Carl);
    let shore = combat.shore.as_ref();
    let broken = shore.is_none_or(|s| s.state() == crate::shore::ShoreState::Broken);
    if broken {
        // The beam is down: hold still while the block falls (the goblin that followed Carl
        // stands in the footprint), then fight on.
        let falling = shore.is_some_and(|s| s.block_state() != crate::shore::BlockState::Landed);
        if falling {
            return RawInput::NONE;
        }
        return clear_and_exit(combat, scene, exit_x);
    }
    let mut raw = walk_in(carl.x, STRIKE_FROM_X);
    if raw.move_x == 0.0 {
        raw.attack = (scene.world.tick + 1).is_multiple_of(40);
    }
    raw
}

/// The scripted cases (ACT1-F01..F08), by their design ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    /// F01: companion off, Carl fights the two goblins and walks to the exit.
    ShoreSoloCarl,
    /// F02: three jabs at the strike column while the goblin stands in the footprint: the
    /// block falls and defeats it.
    ShoreBlockKill,
    /// F03: P1 Carl, P2 Donut.
    Shore2P,
    /// F03: the default `cf02_free` companion, Carl active.
    ShoreSoloCompanion,
    /// F04: a party defeat respawns at the checkpoint; the goblins are restored.
    ShoreRetryFromPostChase,
    /// F05: companion off, Carl walks, swaps to Donut at the zone's approach.
    GuildSoloCarl,
    /// F05: companion off, Donut active from the first tick.
    GuildSoloDonut,
    /// F05: P1 Carl, P2 Donut.
    Guild2P,
    /// F06: the default companion, Carl active, walks right.
    GuildCompanionCrossing,
    /// F07: grant, a checkpoint respawn, a fixture restart, a second crossing.
    GuildRetryKeepsTheBiscuit,
    /// F08: under the All loadout the biscuit is held at tick 0 and the door is open.
    GuildAllLoadoutIsOpen,
}

impl Case {
    pub const ALL: [Case; 11] = [
        Case::ShoreSoloCarl,
        Case::ShoreBlockKill,
        Case::Shore2P,
        Case::ShoreSoloCompanion,
        Case::ShoreRetryFromPostChase,
        Case::GuildSoloCarl,
        Case::GuildSoloDonut,
        Case::Guild2P,
        Case::GuildCompanionCrossing,
        Case::GuildRetryKeepsTheBiscuit,
        Case::GuildAllLoadoutIsOpen,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Case::ShoreSoloCarl => "act1_shore_solo_carl",
            Case::ShoreBlockKill => "act1_shore_block_kill",
            Case::Shore2P => "act1_shore_2p",
            Case::ShoreSoloCompanion => "act1_shore_solo_companion",
            Case::ShoreRetryFromPostChase => "act1_shore_retry_from_post_chase",
            Case::GuildSoloCarl => "act1_guild_solo_carl",
            Case::GuildSoloDonut => "act1_guild_solo_donut",
            Case::Guild2P => "act1_guild_2p",
            Case::GuildCompanionCrossing => "act1_guild_companion_crossing",
            Case::GuildRetryKeepsTheBiscuit => "act1_guild_retry_keeps_the_biscuit",
            Case::GuildAllLoadoutIsOpen => "act1_guild_all_loadout_is_open",
        }
    }

    /// The room the case runs in.
    pub const fn room(self) -> Act1Room {
        match self {
            Case::ShoreSoloCarl
            | Case::ShoreBlockKill
            | Case::Shore2P
            | Case::ShoreSoloCompanion
            | Case::ShoreRetryFromPostChase => Act1Room::Shore,
            _ => Act1Room::Guild,
        }
    }
}

/// The crossing of the guild with Donut active from the first tick (companion off): she
/// walks right through the zone, the door and to the exit.
fn donut_crossing(sim: &mut Act1Sim, budget: u64) {
    let ex = sim.exit_x();
    sim.step([swap(), RawInput::NONE]);
    while sim.tick() < budget && !sim.complete() {
        let input = walk_in(sim.x_of(HeroId::Donut), ex);
        sim.step([input, RawInput::NONE]);
    }
}

/// Run a case to its end and return the sim.
pub fn run_sim(root: &Path, case: Case) -> Result<Act1Sim, String> {
    drive(root, case)
}

/// The shore fought by Carl: `clear_and_exit` of the combat lab's library pilot on his slot.
fn fight(sim: &mut Act1Sim, budget: u64) {
    let ex = sim.exit_x();
    while sim.tick() < budget && !sim.complete() {
        let carl = clear_and_exit(&sim.combat, &sim.scene, ex);
        let second = if sim.party.controlled_hero() == HeroId::Carl
            && matches!(sim.party.mode(), crate::party::PartyMode::TwoPlayer)
        {
            follow(&sim.scene)
        } else {
            RawInput::NONE
        };
        sim.step([carl, second]);
    }
}

fn drive(root: &Path, case: Case) -> Result<Act1Sim, String> {
    let (carl_on, donut_on) = story();
    let (carl, donut) = (CarlProgFlags::default(), DonutProgFlags::default());
    let solo_bot = Party::Solo { uncontrolled: BOT };
    match case {
        Case::ShoreSoloCarl => {
            let mut sim = Act1Sim::new(root, Act1Room::Shore, carl, donut, solo_bot)?;
            fight(&mut sim, MAX_TICKS);
            Ok(sim)
        }
        Case::ShoreBlockKill => {
            let mut sim = Act1Sim::new(root, Act1Room::Shore, carl, donut, solo_bot)?;
            while sim.tick() < 600 && !sim.complete() {
                let input = strike_input(&sim.combat, &sim.scene, sim.exit_x());
                sim.step([input, RawInput::NONE]);
            }
            Ok(sim)
        }
        Case::Shore2P => {
            let mut sim = Act1Sim::new(root, Act1Room::Shore, carl, donut, Party::TwoPlayer)?;
            fight(&mut sim, MAX_TICKS);
            Ok(sim)
        }
        Case::ShoreSoloCompanion => {
            let mut sim = Act1Sim::new(
                root,
                Act1Room::Shore,
                carl,
                donut,
                Party::Solo { uncontrolled: FREE },
            )?;
            fight(&mut sim, MAX_TICKS);
            Ok(sim)
        }
        Case::ShoreRetryFromPostChase => {
            let mut sim = Act1Sim::new(root, Act1Room::Shore, carl, donut, solo_bot)?;
            let ex = sim.exit_x();
            while sim.tick() < 100 {
                let input = clear_and_exit(&sim.combat, &sim.scene, ex);
                sim.step([input, RawInput::NONE]);
            }
            sim.respawn();
            for _ in 0..60 {
                sim.step([RawInput::NONE, RawInput::NONE]);
            }
            Ok(sim)
        }
        Case::GuildSoloCarl => {
            let mut sim = Act1Sim::new(root, Act1Room::Guild, carl_on, donut_on, solo_bot)?;
            let ex = sim.exit_x();
            // Carl walks to the column before the zone, then swaps to Donut, who walks on.
            let mut swapped = false;
            while sim.tick() < MAX_TICKS && !sim.complete() {
                let input = if swapped {
                    walk_in(sim.x_of(HeroId::Donut), ex)
                } else if sim.x_of(HeroId::Carl) >= 300.0 {
                    swapped = true;
                    swap()
                } else {
                    walk_in(sim.x_of(HeroId::Carl), ex)
                };
                sim.step([input, RawInput::NONE]);
            }
            Ok(sim)
        }
        Case::GuildSoloDonut => {
            let mut sim = Act1Sim::new(root, Act1Room::Guild, carl_on, donut_on, solo_bot)?;
            donut_crossing(&mut sim, MAX_TICKS);
            Ok(sim)
        }
        Case::Guild2P => {
            let mut sim = Act1Sim::new(root, Act1Room::Guild, carl_on, donut_on, Party::TwoPlayer)?;
            let ex = sim.exit_x();
            while sim.tick() < MAX_TICKS && !sim.complete() {
                let inputs = [
                    walk_in(sim.x_of(HeroId::Carl), ex),
                    walk_in(sim.x_of(HeroId::Donut), ex),
                ];
                sim.step(inputs);
            }
            Ok(sim)
        }
        Case::GuildCompanionCrossing => {
            let mut sim = Act1Sim::new(
                root,
                Act1Room::Guild,
                carl_on,
                donut_on,
                Party::Solo { uncontrolled: FREE },
            )?;
            let ex = sim.exit_x();
            while sim.tick() < MAX_TICKS && !sim.complete() {
                sim.step([walk_in(sim.x_of(HeroId::Carl), ex), RawInput::NONE]);
            }
            Ok(sim)
        }
        Case::GuildRetryKeepsTheBiscuit => guild_retry(root),
        Case::GuildAllLoadoutIsOpen => {
            let (carl_all, donut_all) = Loadout::All.flags();
            let mut sim = Act1Sim::new(root, Act1Room::Guild, carl_all, donut_all, solo_bot)?;
            let ex = sim.exit_x();
            while sim.tick() < MAX_TICKS && !sim.complete() {
                sim.step([walk_in(sim.x_of(HeroId::Carl), ex), RawInput::NONE]);
            }
            Ok(sim)
        }
    }
}

/// F07: Donut crosses (the biscuit, the open door); a checkpoint respawn keeps both; a fixture
/// restart closes the door and takes the biscuit back; the second crossing grants once more.
fn guild_retry(root: &Path) -> Result<Act1Sim, String> {
    let (carl, donut) = story();
    let mut sim = Act1Sim::new(
        root,
        Act1Room::Guild,
        carl,
        donut,
        Party::Solo { uncontrolled: BOT },
    )?;
    let ex = sim.exit_x();
    sim.step([swap(), RawInput::NONE]);
    while sim.tick() < MAX_TICKS && sim.unlocks().is_empty() {
        sim.step([walk_in(sim.x_of(HeroId::Donut), ex), RawInput::NONE]);
    }
    if sim.unlocks().len() != 1 || !sim.combat.donut_unlocked() {
        return Err("the first crossing did not grant once".into());
    }
    // A checkpoint respawn: the biscuit and the open door stay, no second grant.
    sim.respawn();
    sim.step([RawInput::NONE, RawInput::NONE]);
    if !sim.combat.donut_unlocked() || sim.door_stands() != Some(false) || sim.unlocks().len() != 1
    {
        return Err("a checkpoint respawn lost the biscuit or closed the door".into());
    }
    // A fixture restart: the room's start state (no biscuit, a standing door).
    sim.restart();
    sim.step([RawInput::NONE, RawInput::NONE]);
    if sim.combat.donut_unlocked() || sim.door_stands() != Some(true) {
        return Err("a fixture restart kept the biscuit or the open door".into());
    }
    // The second crossing grants once more and opens the door once more.
    donut_crossing(&mut sim, MAX_TICKS);
    Ok(sim)
}

/// The pinned final state hash of each case (new constants, ACT1-A).
pub const PINNED: [(&str, u64); 11] = [
    ("act1_shore_solo_carl", 0x95e3_5090_e54b_78c9),
    ("act1_shore_block_kill", 0xa41e_f2b4_9b85_2c09),
    ("act1_shore_2p", 0x16c4_c3c9_806a_cf0f),
    ("act1_shore_solo_companion", 0xf354_ef29_51b7_e634),
    ("act1_shore_retry_from_post_chase", 0xa5c1_984a_e83a_c10a),
    ("act1_guild_solo_carl", 0xbd2d_56f9_249f_656d),
    ("act1_guild_solo_donut", 0x01a7_1c04_7d66_e4de),
    ("act1_guild_2p", 0xb3b2_8a6b_e8eb_e3fa),
    ("act1_guild_companion_crossing", 0x10ef_660e_1004_b6ac),
    ("act1_guild_retry_keeps_the_biscuit", 0x2d19_a67d_c8a0_0293),
    ("act1_guild_all_loadout_is_open", 0xf5c5_224d_c5a8_cd95),
];

#[cfg(test)]
mod tests;
