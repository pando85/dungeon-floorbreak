//! The Floor 01 boss scene (HUD-DEBT-03): the arena kit, the named boss
//! fixtures and the deterministic autopilot that plays them. Bevy-free.
//!
//! The arena is one locked 640 px screen composed from the approved Floor 01
//! kit by [`crate::camera_lab::layout`]; this module adds the colliders the
//! kit does not have (the ceiling 224 px above the walking line, the right
//! wall) and keeps the entrance gate and the exit wall in step with the boss
//! ([`sync_arena_solids`]). The boss itself is [`crate::boss`].
//!
//! Fixtures are [`PartyFixture`]s (a live party: Carl, Donut as the follower
//! bot or P2) on a [`Fixture`] with `boss_hp > 0`. Their scripts are tick
//! exact where a behaviour needs one; the long ones are played by [`Pilot`], a
//! pure function of the finished state, so a replay is bit-identical.

use std::path::Path;

use crate::boss::{Boss, BossMode, BossSetup, builtin};
use crate::camera_lab::layout::build_kit;
use crate::camera_lab::room::parse_rooms;
use crate::combat_scene::{CombatState, Fixture};
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::env_kit::{EnvKit, IRect, ViewDef};
use crate::input::{RawInput, SlotInputs};
use crate::party::{PROVISIONAL_TUNING, PartyTuning};
use crate::party_scene::{PartyFixture, run};
use crate::prototype::{Block, HERO_HALF_WIDTH, Material, Rect};

/// The arena geometry (camera-lab room schema, one room).
pub const ARENA_PATH: &str = "assets/levels/floor01/boss01_arena.room.json";
/// The layout id of the arena kit (`FLOOR 01` on the HUD).
pub const ARENA_LAYOUT_ID: &str = "floor01_boss_arena";
/// Width of the right wall collider (608..640).
const WALL_W: f32 = 32.0;
/// How far the wall and gate colliders reach below the walking line, and how
/// tall they are (they close the whole screen height).
const WALL_BELOW_FLOOR: f32 = 12.0;
const WALL_H: f32 = 400.0;

/// Load the arena kit: the composed room plus the colliders the boss needs.
pub fn load_arena(root: &Path) -> Result<EnvKit, String> {
    let path = root.join(ARENA_PATH);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let rooms = parse_rooms(&text)?;
    let room = rooms.first().ok_or("the arena file holds no room")?;
    let (mut kit, _contract, _plan) = build_kit(root, room).map_err(|e| e.to_string())?;
    let walk = kit.base_walk_line_y;
    let data = builtin();
    // The kit's 16 px end walls go: the left side is the entrance gate (open
    // until the boss engages, a collider after), the right one is the wall.
    kit.solid_rects
        .retain(|r| !(r.w == 16 && r.y == 32 && (r.x == 0 || r.x == 624)));
    // The floor reaches the left edge, so a hero in the open doorway stands on
    // the walking line.
    kit.solid_rects.push(IRect {
        x: 0,
        y: walk,
        w: 16,
        h: 12,
    });
    kit.solid_rects.push(IRect {
        x: data.arena.right_wall_x as i32,
        y: 32,
        w: WALL_W as i32,
        h: WALL_H as i32,
    });
    // The ceiling: a solid slab whose underside is `ceiling_px` above the
    // walking line, so every jump stays below the HUD band.
    kit.solid_rects.push(IRect {
        x: 0,
        y: walk - data.arena.ceiling_px - 16,
        w: kit.width,
        h: 16,
    });
    // The exit lies behind the right wall; heroes reach it when the wall opens.
    kit.exit = Some(IRect {
        x: data.arena.right_wall_x as i32 + 4,
        y: walk - 96,
        w: WALL_W as i32 - 4,
        h: 96,
    });
    kit.layout_id = ARENA_LAYOUT_ID.to_owned();
    // The same framing as the corridor: the walking line 288 px below the top
    // of the 640x360 view and 308 px below the top of the 640x400 Deck view.
    kit.views.clear();
    kit.views.insert(
        "abc_640x360".to_owned(),
        ViewDef {
            x: 0,
            y: walk - 288,
            w: 640,
            h: 360,
        },
    );
    kit.views.insert(
        "abc_640x400".to_owned(),
        ViewDef {
            x: 0,
            y: walk - 308,
            w: 640,
            h: 400,
        },
    );
    Ok(kit)
}

fn gate_block(boss: &Boss) -> Block {
    let gate = &boss.data().gate;
    Block {
        rect: Rect {
            left: 0.0,
            bottom: boss.floor_y() - WALL_BELOW_FLOOR,
            width: gate.solid_x,
            height: WALL_H,
        },
        material: Material::Stone,
        intact: true,
    }
}

fn wall_block(boss: &Boss) -> Block {
    Block {
        rect: Rect {
            left: boss.data().arena.right_wall_x,
            bottom: boss.floor_y() - WALL_BELOW_FLOOR,
            width: WALL_W,
            height: WALL_H,
        },
        material: Material::Stone,
        intact: true,
    }
}

/// Keep the arena's dynamic colliders where the boss says they are: the gate
/// is a collider exactly while it is closed, and the right wall is gone once
/// the boss is defeated (the exit is open). Idempotent; called every tick.
pub fn sync_arena_solids(solids: &mut Vec<Block>, boss: Option<&Boss>) {
    let Some(boss) = boss else {
        return;
    };
    let gate = gate_block(boss);
    let wall = wall_block(boss);
    let is_gate = |b: &Block| b.rect.left == gate.rect.left && b.rect.width == gate.rect.width;
    let is_wall = |b: &Block| b.rect.left == wall.rect.left && b.rect.width == wall.rect.width;
    let gate_present = solids.iter().any(is_gate);
    if boss.gate_closed() && !gate_present {
        solids.push(gate);
    } else if !boss.gate_closed() && gate_present {
        solids.retain(|b| !is_gate(b));
    }
    let wall_present = solids.iter().any(is_wall);
    if boss.released() && wall_present {
        solids.retain(|b| !is_wall(b));
    } else if !boss.released() && !wall_present {
        solids.push(wall);
    }
}

// ----------------------------------------------------------------- fixtures --

/// The autopilot of a boss fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pilot {
    /// The script only (every earlier fixture).
    None,
    /// Carl plays the fight with his base kit: dodge, punish, no racks.
    Carl,
    /// Carl plays the rack plan: bait the charge, jab the strut, then punish.
    CarlRack,
    /// [`Pilot::Carl`] plus a P2 Donut that keeps clear and dodges.
    Duo,
    /// CF02-F: Carl plays the microarena straight through both waves
    /// ([`crate::cf02::arena::Plan::Direct`]); P2 (if present) is scripted.
    ArenaDirect,
    /// CF02-F: Carl breaks the rotten shore first and lets the goblins that chase
    /// him walk under the block ([`crate::cf02::arena::Plan::Environment`]).
    ArenaEnvironment,
    /// CF02-F: [`Pilot::ArenaDirect`] plus a P2 Donut that swats the tell aimed at
    /// Carl and holds a lying goblin ([`crate::cf02::arena::donut_pilot`]).
    ArenaDuo,
    /// CARL-PROG-01 CP-C (appended): [`Pilot::Carl`] that also presses the special
    /// (Circle/B) on a fixed rule while it is ready: it holds the boss at the
    /// stick's throw distance, facing it, and throws; the rest is the base kit.
    /// Used only by `carl_prog_juicer_dynamite_clear`; the 13 boss fixtures never
    /// select it. The name is neutral (it presses whatever special Carl holds).
    ThrownCharge,
    /// DONUT-PROG-01 DP-F (appended): [`Pilot::ArenaDuo`] whose P2 Donut also
    /// presses the blink (`crate::cf02::arena::donut_blink_pilot`). Used only by
    /// the `donut_prog_pilot_arena_blink` harness; no committed fixture selects
    /// it.
    ArenaDuoBlink,
}

pub(crate) const fn boss_base(
    name: &'static str,
    summary: &'static str,
    seed: u64,
    carl_x: i32,
    boss: BossSetup,
    ticks: u64,
) -> Fixture {
    Fixture {
        name,
        summary,
        seed,
        carl_x,
        enemies: &[],
        shore_dx: None,
        boss: Some(boss),
        charges: &[],
        engineers: &[],
        llamas: &[],
        script: &[],
        ticks,
    }
}

/// Full boss HP and the racks on.
pub(crate) const FULL: BossSetup = BossSetup {
    hp: u16::MAX,
    left_rack: true,
};
/// Full HP and no racks (the strut is not there).
pub(crate) const FULL_NO_RACKS: BossSetup = BossSetup {
    hp: u16::MAX,
    left_rack: false,
};
pub(crate) const fn hp(hp: u16) -> BossSetup {
    BossSetup {
        hp,
        left_rack: true,
    }
}

const POUND_BASE: Fixture = boss_base(
    "boss01_pound_punish",
    "Carl meets the Juicer at 100 px: the pound tell, the dodge and the free hits",
    0x0B05_0001,
    380,
    FULL,
    420,
);
const SCENE_FIXTURE: Fixture = boss_base(
    "boss01_scene",
    "The Floor 01 boss arena for live play (`--scene boss_f01`): Carl at the entrance, the Juicer dormant",
    0x0B05_00FF,
    56,
    FULL,
    7200,
);
/// The fixture behind `floor01_scene --scene boss_f01` (interactive play and
/// captures). Not a replay fixture: it has no script and no committed golden.
pub const BOSS01_SCENE: &Fixture = &SCENE_FIXTURE;

/// The pound: Carl steps out of the tell, then punishes the recovery.
pub const BOSS01_POUND_PUNISH: PartyFixture = PartyFixture {
    base: &POUND_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::Carl,
    donut: DonutSetup::NONE,
};

const CHARGE_JUMP_BASE: Fixture = boss_base(
    "boss01_charge_jump",
    "The charge tell, Carl jumps the ram, the wall crash and the stun punish",
    0x0B05_0002,
    260,
    FULL,
    520,
);
/// The charge dodged by a jump, the crash and the punish.
pub const BOSS01_CHARGE_JUMP: PartyFixture = PartyFixture {
    base: &CHARGE_JUMP_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::Carl,
    donut: DonutSetup::NONE,
};

const CHARGE_HIT_BASE: Fixture = boss_base(
    "boss01_charge_hit",
    "Carl stands in the charge: the ram hits for 1, then the boss hits the wall",
    0x0B05_0003,
    300,
    FULL,
    420,
);
/// The charge lands on a hero who does not jump.
pub const BOSS01_CHARGE_HIT: PartyFixture = PartyFixture {
    base: &CHARGE_HIT_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const HUG_BASE: Fixture = boss_base(
    "boss01_hug_guard",
    "Carl walks into the body: pushed out, then pounded",
    0x0B05_0004,
    380,
    FULL,
    300,
);
/// The body is solid: nobody walks through it.
pub const BOSS01_HUG_GUARD: PartyFixture = PartyFixture {
    base: &HUG_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[run(1, 300, 0, 1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const RACK_CARL_BASE: Fixture = boss_base(
    "boss01_rack_carl",
    "Carl baits the charge, the boss crashes under the left rack and Carl drops it",
    0x0B05_0005,
    180,
    FULL,
    420,
);
/// The left rack: bait, crash, three strut hits, crush.
pub const BOSS01_RACK_CARL: PartyFixture = PartyFixture {
    base: &RACK_CARL_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::CarlRack,
    donut: DonutSetup::NONE,
};

const PHASE_ROAR_BASE: Fixture = boss_base(
    "boss01_phase_roar",
    "The boss starts at 25 HP: the first hit crosses 24 and the roar begins",
    0x0B05_0006,
    380,
    hp(25),
    360,
);
/// The first threshold roar.
pub const BOSS01_PHASE_ROAR: PartyFixture = PartyFixture {
    base: &PHASE_ROAR_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::Carl,
    donut: DonutSetup::NONE,
};

const ROAR_QUEUED_BASE: Fixture = boss_base(
    "boss01_roar_queued",
    "The boss starts at 28 HP: the rack crush crosses 24 during the pin and the roar waits",
    0x0B05_0007,
    180,
    hp(28),
    620,
);
/// A threshold crossed during a pin queues the roar behind it.
pub const BOSS01_ROAR_QUEUED: PartyFixture = PartyFixture {
    base: &ROAR_QUEUED_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::CarlRack,
    donut: DonutSetup::NONE,
};

const GATE_2P_BASE: Fixture = boss_base(
    "boss01_2p_gate",
    "Two players: Carl is in, P2 lags at x 20 and the gate waits, then P2 walks in",
    0x0B05_0008,
    52,
    FULL,
    320,
);
/// The gate waits for P2.
pub const BOSS01_2P_GATE: PartyFixture = PartyFixture {
    base: &GATE_2P_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[run(1, 90, 0, 1.0), run(200, 260, 1, 1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const SOLO_AI_BASE: Fixture = boss_base(
    "boss01_solo_ai_guard",
    "Solo: the AI Donut follows, dodges the pound and the charge and is never worn below 1 HP",
    0x0B05_0009,
    300,
    FULL,
    900,
);
/// Solo safeguards M1 to M3 against the boss.
pub const BOSS01_SOLO_AI_GUARD: PartyFixture = PartyFixture {
    base: &SOLO_AI_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::Carl,
    donut: DonutSetup::NONE,
};

const DEFEAT_BASE: Fixture = boss_base(
    "boss01_party_defeat_retry",
    "Carl never dodges: the party is defeated and both heroes restart at the entrance",
    0x0B05_000A,
    330,
    FULL,
    900,
);
/// D1: a party defeat and the restart.
pub const BOSS01_PARTY_DEFEAT_RETRY: PartyFixture = PartyFixture {
    base: &DEFEAT_BASE,
    two_player: false,
    tuning: PartyTuning {
        // The bot is the target only when it is nearer; here Carl stands still.
        target_ai_bias_px: 40,
        ..PROVISIONAL_TUNING
    },
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const CRUSH_BASE: Fixture = boss_base(
    "boss01_crush_kill",
    "The boss has 4 HP left: the rack crush defeats it (drop hazard, catastrophic)",
    0x0B05_000B,
    180,
    hp(4),
    420,
);
/// The lethal rack crush.
pub const BOSS01_CRUSH_KILL: PartyFixture = PartyFixture {
    base: &CRUSH_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::CarlRack,
    donut: DonutSetup::NONE,
};

const CLEAR_SOLO_BASE: Fixture = boss_base(
    "boss01_full_clear_solo",
    "Carl's base kit and an AI Donut that never attacks defeat the Juicer, no racks",
    0x0B05_000C,
    56,
    FULL_NO_RACKS,
    7200,
);
/// The fight is winnable alone, without Donut attacks and without racks.
pub const BOSS01_FULL_CLEAR_SOLO: PartyFixture = PartyFixture {
    base: &CLEAR_SOLO_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::Carl,
    donut: DonutSetup::NONE,
};

const CLEAR_2P_BASE: Fixture = boss_base(
    "boss01_full_clear_2p",
    "Two players: Carl fights, P2 Donut keeps clear and draws the boss, the Juicer falls",
    0x0B05_000D,
    56,
    FULL,
    7200,
);
/// A full clear with two players.
pub const BOSS01_FULL_CLEAR_2P: PartyFixture = PartyFixture {
    base: &CLEAR_2P_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::Duo,
    donut: DonutSetup::NONE,
};

/// Every boss fixture, in committed order. `boss01_rack_donut` is pending
/// (the right rack waits for Q10's higher cat jump; `rack.right.enabled`).
pub const BOSS_FIXTURES: [&PartyFixture; 13] = [
    &BOSS01_POUND_PUNISH,
    &BOSS01_CHARGE_JUMP,
    &BOSS01_CHARGE_HIT,
    &BOSS01_HUG_GUARD,
    &BOSS01_RACK_CARL,
    &BOSS01_PHASE_ROAR,
    &BOSS01_ROAR_QUEUED,
    &BOSS01_2P_GATE,
    &BOSS01_SOLO_AI_GUARD,
    &BOSS01_PARTY_DEFEAT_RETRY,
    &BOSS01_CRUSH_KILL,
    &BOSS01_FULL_CLEAR_SOLO,
    &BOSS01_FULL_CLEAR_2P,
];

// -------------------------------------------------------------------- pilot --

/// A hero's slot input with the autopilot applied on top of the script.
pub fn pilot_inputs(
    pilot: Pilot,
    mut slots: SlotInputs,
    tick: u64,
    combat: &CombatState,
    scene: &SceneSim,
) -> SlotInputs {
    // CF02-F: the microarena pilots read no boss.
    match pilot {
        Pilot::ArenaDirect => {
            slots[0] =
                crate::cf02::arena::autopilot(combat, scene, crate::cf02::arena::Plan::Direct);
            return slots;
        }
        Pilot::ArenaEnvironment => {
            slots[0] =
                crate::cf02::arena::autopilot(combat, scene, crate::cf02::arena::Plan::Environment);
            return slots;
        }
        Pilot::ArenaDuo => {
            slots[0] =
                crate::cf02::arena::autopilot(combat, scene, crate::cf02::arena::Plan::Direct);
            slots[1] = crate::cf02::arena::donut_pilot(combat, scene);
            return slots;
        }
        Pilot::ArenaDuoBlink => {
            slots[0] =
                crate::cf02::arena::autopilot(combat, scene, crate::cf02::arena::Plan::Direct);
            slots[1] = crate::cf02::arena::donut_blink_pilot(combat, scene);
            return slots;
        }
        _ => {}
    }
    let Some(boss) = combat.boss.as_ref() else {
        return slots;
    };
    match pilot {
        Pilot::None
        | Pilot::ArenaDirect
        | Pilot::ArenaEnvironment
        | Pilot::ArenaDuo
        | Pilot::ArenaDuoBlink => {}
        Pilot::ThrownCharge => slots[0] = thrower_pilot(tick, combat, scene, boss),
        Pilot::Carl => slots[0] = carl_pilot(tick, combat, scene, boss, false),
        Pilot::CarlRack => slots[0] = carl_pilot(tick, combat, scene, boss, true),
        Pilot::Duo => {
            slots[0] = carl_pilot(tick, combat, scene, boss, false);
            slots[1] = donut_pilot(scene, boss);
        }
    }
    slots
}

/// The pilot presses the jab every this many ticks: a first-clear player, not
/// a frame-perfect bot.
const SWING_PERIOD: u64 = 12;

/// Jump over the pound once its box is this many ticks from turning on (the
/// jump clears the 22 px box after 4 ticks).
const POUND_JUMP_AT: u16 = 10;
/// Step out of the pound box once it is this many ticks from turning on.
const STEP_OUT_AT: u16 = 13;
/// No new swing once the pound is this close: a string takes 22 ticks.
const NO_NEW_SWING_AT: u16 = 33;

/// Where a hero would be hit by the next pound or ram, and when.
struct Danger {
    /// Ticks until the pound box turns on (pound modes only).
    pound_in: Option<u16>,
    /// In front of the boss, inside the pound reach.
    in_pound_zone: bool,
    /// Ticks until the ram box reaches the hero, if it is in the path.
    ram_in: Option<f32>,
}

fn danger(boss: &Boss, x: f32) -> Danger {
    let rel = x - boss.x;
    let ahead = rel * f32::from(boss.facing);
    let pound_in = match boss.mode {
        BossMode::PoundTell
        | BossMode::SecondTell
        | BossMode::PoundStrike
        | BossMode::SecondStrike => boss.ticks_to_active(),
        _ => None,
    };
    let ram_in = if matches!(boss.mode, BossMode::ChargeTell | BossMode::Charge) {
        let dir = f32::from(boss.run_dir());
        let ram = &boss.data().charge.ram_box;
        let lead = boss.x + dir * ram.x[1];
        let gap = (x - lead) * dir - HERO_HALF_WIDTH;
        let in_path = (x - boss.x) * dir > 0.0 && gap > -(ram.x[1] - ram.x[0] + 24.0);
        let wait = f32::from(boss.ticks_to_active().unwrap_or(0));
        let run = gap.max(0.0) / (boss.data().charge.speed_px_s / 60.0);
        in_path.then_some(wait + run)
    } else {
        None
    };
    Danger {
        pound_in,
        in_pound_zone: ahead > 0.0 && rel.abs() <= 84.0,
        ram_in,
    }
}

fn sign(v: f32) -> f32 {
    if v >= 0.0 { 1.0 } else { -1.0 }
}

/// The dodge shared by both pilots: a jump over a ram, a step out of a pound
/// (or a jump when cornered). `None` when nothing threatens.
fn dodge(boss: &Boss, x: f32, grounded: bool) -> Option<RawInput> {
    let d = danger(boss, x);
    let lead = f32::from(boss.data().ai_charge_jump_lead);
    if let Some(ram) = d.ram_in
        && ram <= lead
        && grounded
    {
        return Some(RawInput {
            jump: true,
            ..RawInput::NONE
        });
    }
    // A shockwave ripple 10 px tall: jump it when it is about to arrive.
    if grounded
        && boss.ripples().any(|(rx, dir)| {
            let gap = (x - rx) * f32::from(dir) - HERO_HALF_WIDTH - 6.0;
            (-12.0..=36.0).contains(&gap)
        })
    {
        return Some(RawInput {
            jump: true,
            ..RawInput::NONE
        });
    }
    if let (Some(t), true) = (d.pound_in, d.in_pound_zone) {
        let away = sign(x - boss.x);
        let a = &boss.data().arena;
        let room = if away > 0.0 {
            a.right_wall_x - x
        } else {
            x - a.left_wall_x
        };
        // Phase 1 has no ripples: step out of the box and come back for the
        // recovery. From phase 2 the ripples cross the floor, so jump the
        // pound (the box is 22 px tall) and land among the free hits.
        let jump_it = boss.phase >= 2 || room < 40.0;
        if jump_it && t <= POUND_JUMP_AT && grounded {
            return Some(RawInput {
                jump: true,
                ..RawInput::NONE
            });
        }
        if !jump_it && t <= STEP_OUT_AT {
            return Some(RawInput {
                move_x: away,
                ..RawInput::NONE
            });
        }
    }
    None
}

fn carl_pilot(
    tick: u64,
    combat: &CombatState,
    scene: &SceneSim,
    boss: &Boss,
    rack_plan: bool,
) -> RawInput {
    let hero = &scene.world.hero;
    let idle = RawInput::default();
    if combat.hero_defeated {
        return idle;
    }
    if boss.released() {
        return RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        };
    }
    if !boss.gate_closed() {
        return RawInput {
            move_x: if hero.x < 140.0 { 1.0 } else { 0.0 },
            ..RawInput::NONE
        };
    }
    if !hero.grounded && danger(boss, hero.x).ram_in.is_some() {
        // Airborne over a ram: no air control, so the jump clears it as planned.
        return idle;
    }
    if let Some(raw) = dodge(boss, hero.x, hero.grounded) {
        return raw;
    }
    // A jab string is 22 ticks: no new swing once the pound is under 30 ticks
    // away, so Carl is free to jump when it is 10 away.
    if danger(boss, hero.x)
        .pound_in
        .is_some_and(|t| t <= NO_NEW_SWING_AT)
    {
        return idle;
    }
    let landed = matches!(boss.racks[0].state, crate::boss::RackState::Landed);
    if rack_plan && !landed && boss.racks[0].enabled {
        return rack_job(tick, hero.x, boss);
    }
    let Some(body) = boss.hurtbox() else {
        return idle;
    };
    // Keep the left rack out of it: no swing whose fist would reach the strut.
    let gap = if hero.x < body.left {
        body.left - hero.x
    } else if hero.x > body.right() {
        hero.x - body.right()
    } else {
        0.0
    };
    let side = if hero.x < body.left { 1.0 } else { -1.0 };
    if gap > 18.0 {
        return RawInput {
            move_x: side,
            ..RawInput::NONE
        };
    }
    if tick.is_multiple_of(SWING_PERIOD) {
        RawInput {
            attack: true,
            move_x: side,
            ..RawInput::NONE
        }
    } else {
        idle
    }
}

/// CP-C: the gap (px, hero pivot to the boss body's near edge) at which the
/// thrower pilot throws: the standing stick lands about 111 px ahead of the
/// pivot, so a stick thrown from this window lands within the boss's reach
/// (the 64 px ring) while Carl stands out of the pound.
const THROW_GAP_MIN: f32 = 96.0;
const THROW_GAP_MAX: f32 = 124.0;
/// The thrower pilot keeps this far (px) from its own live stick: the ring is 64.
const STICK_CLEARANCE: f32 = 84.0;

/// [`Pilot::Carl`] that throws the special when it is ready and the boss stands
/// in the throw window; otherwise it is `carl_pilot` exactly. A pure function of
/// the finished state (the ledger's hold, the thrower's readiness, the boss).
fn thrower_pilot(tick: u64, combat: &CombatState, scene: &SceneSim, boss: &Boss) -> RawInput {
    let base = carl_pilot(tick, combat, scene, boss, false);
    let hero = &scene.world.hero;
    let thrower = combat.thrower();
    if combat.hero_defeated
        || !boss.gate_closed()
        || !combat.story_holds(crate::story_unlock::SPECIAL_THROWN_CHARGE)
    {
        return base;
    }
    // A running throw: stand still until the recovery may be cancelled (t50);
    // after that the base kit plays on (a swing cancels the brace, as designed).
    if let Some(clock) = thrower.throw_clock(tick) {
        let cancel_from =
            u64::from(crate::carl_prog::CarlProgTuning::active().cpq14_cancel_from_tick);
        return if clock < cancel_from {
            RawInput::default()
        } else {
            base
        };
    }
    // A live stick: keep out of its ring (the dodge still comes first).
    if let Some(stick) = thrower.stick()
        && hero.grounded
        && dodge(boss, hero.x, hero.grounded).is_none()
        && (hero.x - stick.x).abs() < STICK_CLEARANCE
    {
        return RawInput {
            move_x: sign(hero.x - stick.x),
            ..RawInput::NONE
        };
    }
    if !thrower.is_idle() || !hero.grounded {
        return base;
    }
    // Dodging comes first, as in the base pilot.
    if dodge(boss, hero.x, hero.grounded).is_some()
        || danger(boss, hero.x)
            .pound_in
            .is_some_and(|t| t <= NO_NEW_SWING_AT)
    {
        return base;
    }
    let Some(body) = boss.hurtbox() else {
        return base;
    };
    let side = if hero.x < body.left { 1.0 } else { -1.0 };
    let gap = if hero.x < body.left {
        body.left - hero.x
    } else if hero.x > body.right() {
        hero.x - body.right()
    } else {
        0.0
    };
    if gap > THROW_GAP_MAX {
        return RawInput {
            move_x: side,
            ..RawInput::NONE
        };
    }
    if gap < THROW_GAP_MIN {
        return RawInput {
            move_x: -side,
            ..RawInput::NONE
        };
    }
    // In the window: face the boss, then press (a held direction faces Carl).
    if f32::from(hero.facing) != side {
        return RawInput {
            move_x: side,
            ..RawInput::NONE
        };
    }
    RawInput {
        ability: true,
        ..RawInput::NONE
    }
}

/// The rack plan: stand at x 180, and while the boss is stunned at the left
/// wall jab the strut every 16 ticks (three separate moves).
fn rack_job(tick: u64, x: f32, boss: &Boss) -> RawInput {
    let idle = RawInput::default();
    if x < 176.0 {
        return RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        };
    }
    if x > 188.0 {
        return RawInput {
            move_x: -1.0,
            ..RawInput::NONE
        };
    }
    let stunned_left = matches!(boss.mode, BossMode::Stun | BossMode::Rebound) && boss.x < 200.0;
    if stunned_left
        && matches!(boss.racks[0].state, crate::boss::RackState::Idle { .. })
        && tick.is_multiple_of(16)
    {
        return RawInput {
            attack: true,
            move_x: 1.0,
            ..RawInput::NONE
        };
    }
    idle
}

/// Donut as P2: walks in so the gate closes, then keeps 130 px clear of the
/// boss, jumps the ram and steps out of a pound. She never attacks.
fn donut_pilot(scene: &SceneSim, boss: &Boss) -> RawInput {
    let body = &scene.world.donut_body;
    let idle = RawInput::default();
    if boss.released() {
        return RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        };
    }
    if !boss.gate_closed() {
        return RawInput {
            move_x: if body.x < 60.0 { 1.0 } else { 0.0 },
            ..RawInput::NONE
        };
    }
    if !body.grounded {
        return idle;
    }
    if let Some(raw) = dodge(boss, body.x, body.grounded) {
        return raw;
    }
    let away = sign(body.x - boss.x);
    let a = &boss.data().arena;
    let room = if away > 0.0 {
        a.right_wall_x - body.x
    } else {
        body.x - a.left_wall_x
    };
    if (body.x - boss.x).abs() < 130.0 && room > 40.0 {
        RawInput {
            move_x: away,
            ..RawInput::NONE
        }
    } else {
        idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn the_arena_has_the_ceiling_the_right_wall_and_no_left_wall() {
        let kit = load_arena(root()).unwrap();
        let walk = kit.base_walk_line_y;
        assert_eq!(kit.width, 640);
        let ceiling = kit
            .solid_rects
            .iter()
            .find(|r| r.w == 640 && r.y > 32 && r.y < walk)
            .expect("a ceiling slab");
        assert_eq!(ceiling.y + ceiling.h, walk - 224, "underside 224 px above");
        assert!(
            kit.solid_rects
                .iter()
                .any(|r| r.x == 608 && r.w == 32 && r.h >= 400)
        );
        assert!(
            !kit.solid_rects
                .iter()
                .any(|r| r.x == 0 && r.w <= 32 && r.y == 32),
            "the entrance is open until the gate closes"
        );
        assert_eq!(kit.layout_id, ARENA_LAYOUT_ID);
        for name in ["abc_640x360", "abc_640x400"] {
            assert!(kit.view(name).is_some());
        }
        assert!(kit.walking_line_at(20).is_some(), "the doorway has a floor");
    }

    use crate::boss::RackState;
    use crate::combat::CombatEvent;
    use crate::party_scene::PartyFixtureSim;
    use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH};

    fn sim_of(fixture: &'static PartyFixture) -> PartyFixtureSim {
        PartyFixtureSim::new(&load_arena(root()).unwrap(), 360, fixture).unwrap()
    }

    fn run_fixture(fixture: &'static PartyFixture) -> PartyFixtureSim {
        let mut sim = sim_of(fixture);
        sim.run_scripted(fixture.base.ticks);
        sim
    }

    /// The ticks of every logged event whose line contains `needle`.
    fn ticks_of(sim: &PartyFixtureSim, needle: &str) -> Vec<u64> {
        sim.combat
            .events
            .iter()
            .filter(|e| e.to_string().contains(needle))
            .map(|e| {
                let line = e.to_string();
                let from = line.find("tick=").unwrap() + 5;
                line[from..]
                    .split(' ')
                    .next()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .collect()
    }

    /// Committed goldens: the final `PARTY_STATE` hash of each boss fixture at
    /// its default length (the same headless and in the render loop).
    const GOLDENS: [(&str, u64); 13] = [
        ("boss01_pound_punish", 0x8d59_32fb_88c0_1ad6),
        ("boss01_charge_jump", 0x3c3a_a27f_de84_0fcb),
        ("boss01_charge_hit", 0x4e44_0a03_da30_d8ce),
        ("boss01_hug_guard", 0x8a0b_c2ad_bed6_8dfd),
        ("boss01_rack_carl", 0xe4f1_28fe_1e63_1a96),
        ("boss01_phase_roar", 0x47af_5ea0_0814_a409),
        ("boss01_roar_queued", 0x992f_8561_f59a_7b29),
        ("boss01_2p_gate", 0x81fd_ff98_a3a1_6d3a),
        ("boss01_solo_ai_guard", 0x91f9_6841_3670_c6f8),
        ("boss01_party_defeat_retry", 0xce08_73f7_26ce_2d06),
        ("boss01_crush_kill", 0xcf36_88b2_e1a5_e881),
        ("boss01_full_clear_solo", 0x6223_dbdb_f4c3_73b2),
        ("boss01_full_clear_2p", 0x274c_21f9_24e3_8399),
    ];

    #[test]
    fn every_boss_fixture_is_named_once_and_matches_its_committed_golden() {
        assert_eq!(GOLDENS.len(), BOSS_FIXTURES.len());
        for (fixture, (name, golden)) in BOSS_FIXTURES.iter().zip(GOLDENS) {
            assert_eq!(fixture.name(), name);
            let a = run_fixture(fixture);
            let b = run_fixture(fixture);
            assert_eq!(a.state_hash(), b.state_hash(), "{name} is deterministic");
            assert_eq!(a.state_line(), b.state_line());
            assert_eq!(a.state_hash(), golden, "{name} golden");
        }
    }

    #[test]
    fn the_pound_has_a_tell_a_strike_and_a_punish_and_no_hero_is_hurt() {
        let sim = run_fixture(&BOSS01_POUND_PUNISH);
        let tell = ticks_of(&sim, "kind=boss_tell move=pound")[0];
        let strike = ticks_of(&sim, "kind=boss_strike move=pound")[0];
        assert!(strike - tell >= 36, "tell {tell} strike {strike}");
        assert!(!ticks_of(&sim, "kind=boss_hurt").is_empty(), "free hits");
        assert!(
            ticks_of(&sim, "kind=hero_hurt").is_empty(),
            "stepped out of it"
        );
    }

    #[test]
    fn the_charge_is_jumped_the_boss_crashes_and_the_stun_is_punished() {
        let sim = run_fixture(&BOSS01_CHARGE_JUMP);
        let strike = ticks_of(&sim, "kind=boss_strike move=charge")[0];
        let crash = ticks_of(&sim, "kind=boss_crash wall=left")[0];
        assert!(crash > strike);
        let hurt = ticks_of(&sim, "kind=hero_hurt");
        assert!(hurt.iter().all(|t| *t > crash), "jumped the ram: {hurt:?}");
        let punish: Vec<_> = ticks_of(&sim, "kind=boss_hurt")
            .into_iter()
            .filter(|t| *t > crash)
            .collect();
        assert!(punish.len() >= 3, "{punish:?}");
    }

    #[test]
    fn a_hero_in_the_charge_is_hit_for_one_then_the_boss_hits_the_wall() {
        let sim = run_fixture(&BOSS01_CHARGE_HIT);
        let strike = ticks_of(&sim, "kind=boss_strike move=charge")[0];
        let hurt = ticks_of(&sim, "kind=hero_hurt source=16981 health_left=3");
        assert!(!hurt.is_empty());
        assert!(hurt[0] > strike && hurt[0] - strike <= 30);
        assert!(!ticks_of(&sim, "kind=boss_crash").is_empty());
    }

    #[test]
    fn nobody_stands_inside_the_body_and_the_hug_ends_in_a_pound() {
        let fixture = &BOSS01_HUG_GUARD;
        let mut sim = sim_of(fixture);
        // The pose boxes differ by up to 16 px, and a hero running into the body
        // gains 3 px a tick against the 6 px push: an overlap closes by 3 px a
        // tick, so it may last six ticks, never longer.
        let mut streak = [0u32; 2];
        for _ in 0..fixture.base.ticks {
            sim.step_scripted();
            let boss = sim.combat.boss.as_ref().unwrap();
            if let Some(body) = boss.solid_body() {
                for (i, hero) in [&sim.scene.world.hero, &sim.scene.world.donut_body]
                    .into_iter()
                    .enumerate()
                {
                    let inside = hero.x + HERO_HALF_WIDTH > body.left + 0.5
                        && hero.x - HERO_HALF_WIDTH < body.right() - 0.5
                        && hero.y - HERO_HALF_HEIGHT < body.top() - 0.5;
                    // A defeated hero lies where he fell (the attempt restarts).
                    streak[i] = if inside && !sim.combat.hero_defeated {
                        streak[i] + 1
                    } else {
                        0
                    };
                    assert!(
                        streak[i] <= 6,
                        "hero {i} at {} inside the body {body:?} for {} ticks",
                        hero.x,
                        streak[i]
                    );
                }
            }
        }
        assert!(!ticks_of(&sim, "kind=boss_strike move=pound").is_empty());
    }

    #[test]
    fn the_left_rack_is_dropped_by_carl_and_crushes_the_stunned_boss() {
        let sim = run_fixture(&BOSS01_RACK_CARL);
        assert_eq!(ticks_of(&sim, "kind=rack_hit").len(), 3);
        let dropped = ticks_of(&sim, "kind=rack_dropped rack=0 player=0")[0];
        let landed = ticks_of(&sim, "kind=rack_landed rack=0 crushed=1")[0];
        assert_eq!(landed - dropped, 36, "36 ticks of fall");
        let crash = ticks_of(&sim, "kind=boss_crash")[0];
        assert!(dropped > crash && landed < crash + 90, "inside the stun");
        let hit = ticks_of(&sim, "kind=boss_hurt source=0 damage=4");
        assert_eq!(hit, vec![landed]);
    }

    #[test]
    fn a_threshold_crossed_by_a_hit_starts_the_roar_after_the_current_move() {
        let sim = run_fixture(&BOSS01_PHASE_ROAR);
        let first = ticks_of(&sim, "kind=boss_hurt")[0];
        let roar = ticks_of(&sim, "kind=boss_phase phase=2");
        assert_eq!(roar.len(), 1);
        assert!(roar[0] > first);
    }

    #[test]
    fn a_threshold_crossed_during_the_pin_queues_the_roar_behind_it() {
        let sim = run_fixture(&BOSS01_ROAR_QUEUED);
        let landed = ticks_of(&sim, "kind=rack_landed rack=0 crushed=1")[0];
        let roar = ticks_of(&sim, "kind=boss_phase phase=2")[0];
        assert!(roar >= landed + 90, "pin {landed}, roar {roar}");
        assert_eq!(ticks_of(&sim, "kind=boss_phase").len(), 1);
    }

    #[test]
    fn in_two_players_the_gate_waits_for_p2() {
        let sim = run_fixture(&BOSS01_2P_GATE);
        let engaged = ticks_of(&sim, "kind=boss_engaged");
        assert_eq!(engaged.len(), 1);
        assert!(engaged[0] > 200, "P2 walks in from tick 200: {engaged:?}");
        assert!(engaged[0] < 260);
        // Before it, the boss stood dormant and nothing was hurt.
        let mut early = sim_of(&BOSS01_2P_GATE);
        early.run_scripted(190);
        let boss = early.combat.boss.as_ref().unwrap();
        assert!(!boss.gate_closed());
        assert_eq!(boss.mode, crate::boss::BossMode::Dormant);
        assert!(early.scene.world.donut_body.x < 40.0);
    }

    #[test]
    fn the_solo_ai_hero_is_never_worn_below_one_and_never_triggers_a_rack() {
        let fixture = &BOSS01_SOLO_AI_GUARD;
        let mut sim = sim_of(fixture);
        let mut min = 4u8;
        for _ in 0..fixture.base.ticks {
            sim.step_scripted();
            min = min.min(sim.scene.world.donut.health);
        }
        assert!(min >= 1, "the guard floors the AI hero at 1 HP");
        assert!(ticks_of(&sim, "hero=donut kind=hero_defeated").is_empty());
        // M3: the AI jumps the first charge (takeoff 14..22 ticks before the
        // ram), so Donut is not hit between that tell and the wall crash.
        let tell = ticks_of(&sim, "kind=boss_tell move=charge")[0];
        let crash = ticks_of(&sim, "kind=boss_crash")[0];
        let donut_hurt: Vec<u64> = sim
            .combat
            .events
            .iter()
            .filter(|e| {
                e.to_string().contains("kind=hero_hurt") && e.to_string().contains("hero=donut")
            })
            .map(|e| {
                let line = e.to_string();
                let from = line.find("tick=").unwrap() + 5;
                line[from..].split(' ').next().unwrap().parse().unwrap()
            })
            .collect();
        assert!(
            donut_hurt.iter().all(|t| *t < tell || *t > crash),
            "Donut was hit during the first charge: {donut_hurt:?} (tell {tell}, crash {crash})"
        );
        let dropped = sim
            .combat
            .events
            .iter()
            .filter(|e| matches!(e, CombatEvent::RackDropped { player, .. } if *player != 0))
            .count();
        assert_eq!(dropped, 0);
    }

    #[test]
    fn a_party_defeat_restarts_both_heroes_and_the_boss_at_the_entrance() {
        let fixture = &BOSS01_PARTY_DEFEAT_RETRY;
        let mut sim = sim_of(fixture);
        let mut restart_at = None;
        for t in 0..fixture.base.ticks {
            sim.step_scripted();
            if restart_at.is_none() && sim.combat.restarts == 1 {
                restart_at = Some(t);
                let boss = sim.combat.boss.as_ref().unwrap();
                assert_eq!(boss.hp, boss.data().hp_max);
                assert!(!boss.gate_closed(), "the gate is open again");
                assert_eq!(sim.scene.world.hero.health, 4);
                assert_eq!(sim.scene.world.donut.health, 4);
                assert_eq!(sim.scene.world.hero.x, fixture.base.carl_x as f32);
                assert!(
                    !sim.scene
                        .world
                        .solids
                        .iter()
                        .any(|b| b.rect.left == 0.0 && b.rect.width == 28.0)
                );
            }
        }
        assert!(restart_at.is_some());
        assert_eq!(sim.combat.deaths, 1);
        assert_eq!(ticks_of(&sim, "kind=boss_engaged").len(), 2);
        assert_eq!(ticks_of(&sim, "kind=restart").len(), 1);
    }

    #[test]
    fn the_lethal_crush_is_a_catastrophic_drop_hazard_and_opens_the_exit() {
        let sim = run_fixture(&BOSS01_CRUSH_KILL);
        let lines: Vec<String> = sim.combat.events.iter().map(ToString::to_string).collect();
        let defeat = lines
            .iter()
            .find(|l| l.contains("enemy_defeated"))
            .expect("defeated");
        assert!(
            defeat.contains("cause=drop_hazard credit_player=0 rack=0"),
            "{defeat}"
        );
        let boss = sim.combat.boss.as_ref().unwrap();
        assert!(boss.released());
        assert!(!sim.scene.world.solids.iter().any(|b| b.rect.left == 608.0));
        assert!(matches!(boss.racks[0].state, RackState::Landed));
    }

    #[test]
    fn a_full_clear_alone_without_donut_attacks_or_racks_lands_inside_the_window() {
        let sim = run_fixture(&BOSS01_FULL_CLEAR_SOLO);
        let clear = ticks_of(&sim, "kind=enemy_defeated")[0];
        assert!((2400..=7200).contains(&clear), "cleared on tick {clear}");
        assert!(ticks_of(&sim, "kind=rack_dropped").is_empty(), "no racks");
        assert_eq!(sim.combat.deaths, 0, "one attempt");
    }

    #[test]
    fn a_full_clear_with_two_players_lands_inside_the_window() {
        let sim = run_fixture(&BOSS01_FULL_CLEAR_2P);
        let clear = ticks_of(&sim, "kind=enemy_defeated")[0];
        assert!((2400..=7200).contains(&clear), "cleared on tick {clear}");
        assert_eq!(sim.combat.deaths, 0);
    }

    #[test]
    fn a_jump_stays_below_the_hud_band_and_a_ceiling_collider_sits_224_px_up() {
        let fixture = &BOSS01_HUG_GUARD;
        let mut sim = sim_of(fixture);
        let floor = sim.combat.boss.as_ref().unwrap().floor_y();
        let slots = |jump: bool| {
            [
                RawInput {
                    jump,
                    ..RawInput::NONE
                },
                RawInput::default(),
            ]
        };
        let mut apex = 0.0f32;
        for t in 0..120u32 {
            sim.step_slots(slots(t % 60 == 1));
            let top = sim.scene.world.hero.y + HERO_HALF_HEIGHT - floor;
            apex = apex.max(top);
        }
        // Screen y of the highest hero top at 640x360 (walk line 288 below the
        // top): it stays at or below the HUD band y >= 60.
        assert!(288.0 - apex >= 60.0, "apex {apex}");
        assert!(apex <= 224.0);
        // The apex of today's jump is far under the ceiling, so the collider
        // itself is checked: a solid whose underside is 224 px over the floor.
        assert!(
            sim.scene
                .world
                .solids
                .iter()
                .any(|b| (b.rect.bottom - (floor + 224.0)).abs() < 0.01 && b.rect.width >= 576.0),
            "the ceiling collider is missing"
        );
    }

    #[test]
    fn two_players_may_stand_the_whole_floor_apart_in_the_locked_arena() {
        let fixture = &BOSS01_2P_GATE;
        let mut sim = sim_of(fixture);
        let run_right = [
            RawInput {
                move_x: 1.0,
                ..RawInput::NONE
            },
            RawInput::default(),
        ];
        for _ in 0..260 {
            sim.step_slots(run_right);
        }
        let apart = (sim.scene.world.hero.x - sim.scene.world.donut_body.x).abs();
        assert!(apart > 448.0, "separation {apart} is past the 448 px wall");
    }

    #[test]
    fn the_boss_rack_footprints_are_clear_of_the_walls_and_the_exit_is_behind_the_right_wall() {
        let kit = load_arena(root()).unwrap();
        let exit = kit.exit.unwrap();
        assert!(exit.x >= 608);
        let d = builtin();
        for facing in [-1i8, 1] {
            let c = d.footprint_centre(facing);
            assert!(c - 40.0 >= d.arena.left_wall_x && c + 40.0 <= d.arena.right_wall_x);
        }
    }
}
