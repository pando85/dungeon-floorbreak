//! ACT1-B scenario fixtures (`docs/01-design/floor-01-act-01-integration.md` section 10,
//! ACT1-F10..F18): the chase (`act1_chase`) driven by scripted party inputs on the launched
//! room, deterministic, seeded and fixed-tick. Each fixture runs twice and must end on an
//! identical state hash; the final hashes are pinned in [`PINNED`] (new constants, nothing
//! moves). Bevy-free.
//!
//! The scripted hero is a [`Runner`]: it walks to a target column, hopping the 16 px lane
//! statics on the way (the humps and the alcove ledge), and holds there; the uncontrolled hero of
//! a solo run is the follow-and-jump bot (`--companion off`) except where a row says "default
//! companion". The room is story-less, so the fixtures run with no story flag.

use crate::act1_fixtures::{Act1Sim, BOT, FREE, Party, walk};
use crate::carl_prog::CarlProgFlags;
use crate::combat::CombatEvent;
use crate::combat_scene::EnvSolidKind;
use crate::donut_prog::DonutProgFlags;
use crate::dozer::{ChargeStage, Dozer, DozerEvent, DozerState, RescueSpot};
use crate::f01_act1::{self, Act1Room};
use crate::input::RawInput;
use crate::party::HeroId;
use std::path::Path;

/// The fixed-tick budget of every fixture.
pub const MAX_TICKS: u64 = 3000;
/// The column the scripted hero waits at: the pocket's right end (its body ends at the
/// pocket's right edge), so the bot that trails it by 48 px stands inside the pocket too.
pub const POCKET_X: f32 = 2136.0;
/// The lane statics' left edges (the humps and the alcove ledge), a hop target each.
const HOP_EDGES: [f32; 3] = [1168.0, 1456.0, 2064.0];
/// A hero starts its hop when its body's right edge is this close to a static's left edge.
const HOP_LEAD_PX: f32 = 14.0;

/// A scripted hero: walk to `target`, hopping the lane statics on the way.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Runner {
    pub target: f32,
}

impl Runner {
    /// The next input of the hero at `(x, grounded)`.
    pub fn input(self, x: f32, grounded: bool) -> RawInput {
        let mut raw = RawInput {
            move_x: walk(x, self.target),
            ..RawInput::NONE
        };
        if grounded
            && raw.move_x > 0.0
            && HOP_EDGES
                .iter()
                .any(|&e| e > x && e - (x + 12.0) <= HOP_LEAD_PX && e < self.target + 60.0)
        {
            raw.jump = true;
        }
        raw
    }
}

/// What the machine and the barricade looked like on the tick after a respawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RespawnView {
    pub tick: u64,
    pub machine: &'static str,
    pub barrier_hit: bool,
    /// The `dozer_contact` solid stands (collides) in the scene's world solids.
    pub barricade_stands: bool,
    pub hint: bool,
}

/// A launched chase with a party.
pub struct ChaseSim {
    pub sim: Act1Sim,
    /// `(tick, event)` of every dozer event seen, in order (across respawns).
    pub log: Vec<(u64, DozerEvent)>,
    seen: usize,
    /// The distance between the heroes on the tick `ChargeArmed` fired (the lag of F13).
    pub arm_separation: Option<f32>,
    /// The alcove hint was live on some tick (never on a fresh run).
    pub hint_seen: bool,
    /// The first respawn's view (the tick after it).
    pub respawn_view: Option<RespawnView>,
}

impl ChaseSim {
    pub fn new(root: &Path, party: Party) -> Result<Self, String> {
        let sim = Act1Sim::new(
            root,
            Act1Room::Chase,
            CarlProgFlags::default(),
            DonutProgFlags::default(),
            party,
        )?;
        Ok(Self {
            sim,
            log: Vec::new(),
            seen: 0,
            arm_separation: None,
            hint_seen: false,
            respawn_view: None,
        })
    }

    pub fn dozer(&self) -> &Dozer {
        crate::dozer_art::dozer_of(&self.sim.combat).expect("the chase's machine")
    }

    pub fn tick(&self) -> u64 {
        self.sim.tick()
    }

    pub fn x_of(&self, id: HeroId) -> f32 {
        self.sim.x_of(id)
    }

    pub fn step(&mut self, slots: [RawInput; 2]) {
        self.sim.step(slots);
        self.collect();
        if self.arm_separation.is_none() && self.dozer().armed_once() {
            self.arm_separation = Some((self.x_of(HeroId::Carl) - self.x_of(HeroId::Donut)).abs());
        }
        let hint = f01_act1::alcove_hint_live(&self.sim.combat, self.tick());
        self.hint_seen |= hint;
        if self.respawn_view.is_none() && self.sim.combat.respawns > 0 {
            self.respawn_view = Some(RespawnView {
                tick: self.tick(),
                machine: self.state().name(),
                barrier_hit: self.dozer().barrier_hit(),
                barricade_stands: self.barricade_stands(),
                hint,
            });
        }
    }

    /// Whether the barricade's world solid stands now (it collides with heroes).
    pub fn barricade_stands(&self) -> bool {
        let Some(env) = self.sim.combat.env() else {
            return false;
        };
        let range = env.solid_range();
        env.solids()
            .iter()
            .position(|s| matches!(s.kind, EnvSolidKind::ContactLock { .. }))
            .and_then(|i| self.sim.scene.world.solids.get(range.start + i))
            .is_some_and(|b| b.intact)
    }

    fn collect(&mut self) {
        let events = &self.sim.combat.events;
        for e in &events[self.seen.min(events.len())..] {
            if let CombatEvent::Dozer { tick, event } = *e {
                self.log.push((tick, event));
            }
        }
        self.seen = events.len();
    }

    /// A checkpoint respawn (the party's defeat retry).
    pub fn respawn(&mut self) {
        self.sim.respawn();
        self.collect();
    }

    pub fn events(&self) -> impl Iterator<Item = DozerEvent> + '_ {
        self.log.iter().map(|(_, e)| *e)
    }

    pub fn count(&self, pick: impl Fn(&DozerEvent) -> bool) -> usize {
        self.log.iter().filter(|(_, e)| pick(e)).count()
    }

    pub fn wheel_hits(&self) -> usize {
        self.count(|e| matches!(e, DozerEvent::WheelHit { .. }))
    }

    pub fn rescues(&self) -> usize {
        self.count(|e| matches!(e, DozerEvent::AiRescue { .. }))
    }

    pub fn rescues_by_spot(&self, spot: RescueSpot) -> usize {
        self.count(|e| matches!(e, DozerEvent::AiRescue { spot: s, .. } if *s == spot))
    }

    /// The tick of the first `pick` event.
    pub fn first(&self, pick: impl Fn(&DozerEvent) -> bool) -> Option<u64> {
        self.log.iter().find(|(_, e)| pick(e)).map(|(t, _)| *t)
    }

    pub fn complete(&self) -> bool {
        self.sim.complete()
    }

    pub fn hash(&self) -> u64 {
        self.sim.hash()
    }

    pub fn exit_x(&self) -> f32 {
        self.sim.exit_x()
    }

    pub fn stage(&self) -> ChargeStage {
        self.dozer().charge
    }

    pub fn state(&self) -> DozerState {
        self.dozer().state
    }
}

/// The scripted cases (ACT1-F10..F16), by their design ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    /// F10: Carl scripted, Donut the follow bot (companion off).
    SoloFirstRun,
    /// F10b: Donut scripted, Carl the follow bot.
    SoloFirstRunDonut,
    /// F11: Carl jabs the hull from the pocket while the machine passes.
    HullCannotBeHurt,
    /// F12: both heroes scripted into the pocket together.
    TwoPlayerTogether,
    /// F13: P2 `lag` px behind the leader until the leader arms, then both hide.
    TwoPlayerLag(u16),
    /// F14: Carl waits at the closed barricade; defeat, respawn, a clean run.
    SkipPocket,
    /// F15: the default companion, Carl active (the F10 script).
    BotCarlActive,
    /// F15: the default companion, Donut active (the F10 script).
    BotDonutActive,
    /// F16: P1 waits in the pocket, P2 shadows the front so rule 2 never holds.
    NoChargePath,
}

impl Case {
    pub const ALL: [Case; 11] = [
        Case::SoloFirstRun,
        Case::SoloFirstRunDonut,
        Case::HullCannotBeHurt,
        Case::TwoPlayerTogether,
        Case::TwoPlayerLag(0),
        Case::TwoPlayerLag(96),
        Case::TwoPlayerLag(224),
        Case::TwoPlayerLag(448),
        Case::SkipPocket,
        Case::BotCarlActive,
        Case::BotDonutActive,
    ];

    pub fn name(self) -> String {
        match self {
            Case::SoloFirstRun => "act1_chase_solo_first_run".into(),
            Case::SoloFirstRunDonut => "act1_chase_solo_first_run_donut".into(),
            Case::HullCannotBeHurt => "act1_chase_hull_cannot_be_hurt".into(),
            Case::TwoPlayerTogether => "act1_chase_2p_together".into(),
            Case::TwoPlayerLag(l) => format!("act1_chase_2p_lag_{l}"),
            Case::SkipPocket => "act1_chase_skip_pocket".into(),
            Case::BotCarlActive => "act1_chase_solo_bot_carl_active".into(),
            Case::BotDonutActive => "act1_chase_solo_bot_donut_active".into(),
            Case::NoChargePath => "act1_chase_no_charge_path".into(),
        }
    }
}

fn swap_input() -> RawInput {
    RawInput {
        swap: true,
        ..RawInput::NONE
    }
}

/// The scripted hero of a solo run: run to the pocket, wait there until the machine is wedged,
/// then walk to the exit. `uncontrolled` is the other hero's driver.
fn solo_script(
    root: &Path,
    donut_active: bool,
    uncontrolled: crate::party::UncontrolledHero,
) -> Result<ChaseSim, String> {
    let mut sim = ChaseSim::new(root, Party::Solo { uncontrolled })?;
    let exit = sim.exit_x();
    let hero = if donut_active {
        HeroId::Donut
    } else {
        HeroId::Carl
    };
    if donut_active {
        sim.step([swap_input(), RawInput::NONE]);
    }
    while sim.tick() < MAX_TICKS && !sim.complete() {
        let target = if sim.dozer().wedged() { exit } else { POCKET_X };
        let grounded = sim.sim.scene.world.body(hero).grounded;
        let input = Runner { target }.input(sim.x_of(hero), grounded);
        sim.step([input, RawInput::NONE]);
    }
    Ok(sim)
}

/// A varied solo first run (the robustness sweep): the scripted hero waits `delay` ticks at
/// the spawn and holds `stall` ticks when it first passes column `stall_x` (a stall of 0 holds
/// nothing), then plays the F10 script.
pub fn varied_run(
    root: &Path,
    donut_active: bool,
    uncontrolled: crate::party::UncontrolledHero,
    (delay, stall_x, stall): (u64, f32, u64),
) -> Result<ChaseSim, String> {
    let mut sim = ChaseSim::new(root, Party::Solo { uncontrolled })?;
    let exit = sim.exit_x();
    let hero = if donut_active {
        HeroId::Donut
    } else {
        HeroId::Carl
    };
    if donut_active {
        sim.step([swap_input(), RawInput::NONE]);
    }
    let mut held = 0;
    while sim.tick() < MAX_TICKS && !sim.complete() {
        let x = sim.x_of(hero);
        let target = if sim.dozer().wedged() { exit } else { POCKET_X };
        let waiting = sim.tick() < delay;
        let stalling = stall > 0 && x >= stall_x && held < stall && !sim.dozer().wedged();
        if stalling {
            held += 1;
        }
        let grounded = sim.sim.scene.world.body(hero).grounded;
        let input = if waiting || stalling {
            RawInput::NONE
        } else {
            Runner { target }.input(x, grounded)
        };
        sim.step([input, RawInput::NONE]);
    }
    Ok(sim)
}

/// F10 and F10b: the first run.
pub fn first_run(root: &Path, donut_active: bool) -> Result<ChaseSim, String> {
    solo_script(root, donut_active, BOT)
}

/// A hero's input for the 2P scripts: its [`Runner`] at `target`.
fn runner(sim: &ChaseSim, hero: HeroId, target: f32) -> RawInput {
    let grounded = sim.sim.scene.world.body(hero).grounded;
    Runner { target }.input(sim.x_of(hero), grounded)
}

/// Both heroes scripted (P1 Carl, P2 Donut): `plan` gives each hero's target column this tick.
fn two_player(
    root: &Path,
    plan: impl Fn(&ChaseSim) -> (f32, f32),
    until_wedged_then: f32,
) -> Result<ChaseSim, String> {
    let mut sim = ChaseSim::new(root, Party::TwoPlayer)?;
    while sim.tick() < MAX_TICKS && !sim.complete() {
        let (carl, donut) = if sim.dozer().wedged() {
            (until_wedged_then, until_wedged_then)
        } else {
            plan(&sim)
        };
        let inputs = [
            runner(&sim, HeroId::Carl, carl),
            runner(&sim, HeroId::Donut, donut),
        ];
        sim.step(inputs);
    }
    Ok(sim)
}

/// F12: both into the pocket (Carl at its right end, Donut left of him).
pub fn together(root: &Path) -> Result<ChaseSim, String> {
    let exit = f32::from(2960_u16);
    two_player(root, |_| (POCKET_X, POCKET_X - 40.0), exit)
}

/// F13: P2 (Donut) trails the leader (Carl) by `lag` px until the leader is at the arm column,
/// then both run for the pocket.
pub fn lagged(root: &Path, lag: u16) -> Result<ChaseSim, String> {
    let exit = f32::from(2960_u16);
    // The 2P camera leash holds the pair at most 447 px apart (Donut starts 24 px behind), so
    // the 448 case is held at 440: the achieved separation is recorded (`arm_separation`).
    let held = f32::from(lag).min(LEASH_HOLD_PX);
    let arm = crate::dozer::DozerTuning::opening()
        .charge
        .map_or(1888.0, |c| c.arm_x as f32);
    two_player(
        root,
        move |sim| {
            let carl = sim.x_of(HeroId::Carl);
            let donut = if carl < arm && !sim.dozer().armed_once() {
                (carl - held).max(48.0)
            } else {
                POCKET_X - 40.0
            };
            (POCKET_X, donut)
        },
        exit,
    )
}

/// F16: P1 (Carl) waits in the pocket; P2 (Donut) walks ahead of the wheel front, its body's left
/// edge about `SHADOW_GAP_PX` in front of it, from the emergence into the pocket.
pub fn no_charge_path(root: &Path) -> Result<ChaseSim, String> {
    let exit = f32::from(2960_u16);
    two_player(
        root,
        |sim| {
            let d = sim.dozer();
            // Before the emergence P2 runs to the trigger column and waits just past it.
            let donut = if d.state.emerged() {
                (d.wheel_front() + SHADOW_GAP_PX + 12.0).min(POCKET_X - 40.0)
            } else {
                1100.0
            };
            (POCKET_X, donut)
        },
        exit,
    )
}

/// The most P2 is held behind the leader: the 2P camera leash stops the leader at 447 px.
pub const LEASH_HOLD_PX: f32 = 440.0;

/// F16: the body's left edge of the shadowing hero, ahead of the front (100..130 px).
pub const SHADOW_GAP_PX: f32 = 115.0;

/// F14: Carl runs past the pocket and waits at the closed barricade; the wheel defeats him, the
/// party respawns at the checkpoint and a clean run (the F10 script) exits.
pub fn skip_pocket(root: &Path) -> Result<ChaseSim, String> {
    let mut sim = ChaseSim::new(root, Party::Solo { uncontrolled: BOT })?;
    let exit = sim.exit_x();
    let mut clean = false;
    while sim.tick() < MAX_TICKS && !sim.complete() {
        if !clean && sim.sim.combat.respawns > 0 {
            clean = true;
        }
        let target = if clean {
            if sim.dozer().wedged() { exit } else { POCKET_X }
        } else {
            BARRICADE_WAIT_X
        };
        let input = runner(&sim, HeroId::Carl, target);
        sim.step([input, RawInput::NONE]);
    }
    Ok(sim)
}

/// F14: where Carl waits at the closed barricade (its left edge is 2312).
pub const BARRICADE_WAIT_X: f32 = 2300.0;

/// F11: Carl waits in the pocket and jabs on a fixed cadence while the machine passes.
pub fn hull_jabs(root: &Path) -> Result<ChaseSim, String> {
    let mut sim = ChaseSim::new(root, Party::Solo { uncontrolled: BOT })?;
    let exit = sim.exit_x();
    while sim.tick() < MAX_TICKS && !sim.complete() {
        let wedged = sim.dozer().wedged();
        let target = if wedged { exit } else { POCKET_X };
        let mut input = runner(&sim, HeroId::Carl, target);
        if !wedged && sim.x_of(HeroId::Carl) >= POCKET_X - 3.0 {
            input.attack = (sim.tick() + 1).is_multiple_of(4);
        }
        sim.step([input, RawInput::NONE]);
    }
    Ok(sim)
}

/// Every case, F16 included.
pub fn all_cases() -> Vec<Case> {
    Case::ALL.into_iter().chain([Case::NoChargePath]).collect()
}

/// The pinned final state hash of each case (new constants, ACT1-B).
pub const PINNED: [(&str, u64); 12] = [
    ("act1_chase_solo_first_run", 0x4732_eb77_0732_3aeb),
    ("act1_chase_solo_first_run_donut", 0x5d57_270a_ed00_98da),
    ("act1_chase_hull_cannot_be_hurt", 0xab68_96d6_8e50_063f),
    ("act1_chase_2p_together", 0xa855_d976_a7da_d829),
    ("act1_chase_2p_lag_0", 0x05f7_5fef_f1b2_6a73),
    ("act1_chase_2p_lag_96", 0x42a0_f3f6_536b_27a2),
    ("act1_chase_2p_lag_224", 0x2400_2da9_1bf4_bc68),
    ("act1_chase_2p_lag_448", 0xa387_89b4_549f_6626),
    ("act1_chase_skip_pocket", 0xb714_2ea5_4c27_8e27),
    ("act1_chase_solo_bot_carl_active", 0x3bc7_2950_7f89_0cdf),
    ("act1_chase_solo_bot_donut_active", 0x7b88_2856_8de0_e9a4),
    ("act1_chase_no_charge_path", 0x7e14_bc89_3f95_a10b),
];

/// Run a case to its end.
pub fn run(root: &Path, case: Case) -> Result<ChaseSim, String> {
    match case {
        Case::SoloFirstRun => first_run(root, false),
        Case::SoloFirstRunDonut => first_run(root, true),
        Case::HullCannotBeHurt => hull_jabs(root),
        Case::TwoPlayerTogether => together(root),
        Case::TwoPlayerLag(lag) => lagged(root, lag),
        Case::SkipPocket => skip_pocket(root),
        Case::BotCarlActive => solo_script(root, false, FREE),
        Case::BotDonutActive => solo_script(root, true, FREE),
        Case::NoChargePath => no_charge_path(root),
    }
}

#[cfg(test)]
mod tests;
