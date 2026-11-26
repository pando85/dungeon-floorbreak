//! DEMO-01: one continuous run of the Floor 1 scene on a taller demo hall
//! (`docs/04-production/demo-01-plan.md`). Bevy-free.
//!
//! This module holds the demo as data and pure functions: the hall (a room
//! description composed from the approved Floor 01 kit by
//! [`crate::camera_lab::layout`]), the beat table, the checkpoints, the goblin
//! spawns, the Select tap / hold rule, an autopilot for the
//! headless probes and the evidence captures, and the completion test. The
//! mechanics live in [`crate::combat_scene`] (`DemoRules`, `respawn`); nothing
//! here changes a fixture or a hash outside the demo.
//!
//! Every column and every count below is a playtest proposal from the plan, not
//! an accepted design.

use std::path::Path;

use crate::blast::Arm;
use crate::camera_lab::layout::build_kit;
use crate::camera_lab::room::parse_rooms;
use crate::combat::EnemyAi;
use crate::combat_scene::{ChargeSpawn, CombatState, DemoRules, EnemySpawn, Fixture, LlamaSpawn};
use crate::corridor::SceneSim;
use crate::env_kit::{EnvKit, ViewDef};
use crate::input::RawInput;

/// The hall geometry (camera-lab room schema, one room).
pub const HALL_PATH: &str = "assets/levels/floor01/demo01_hall.room.json";
/// Walking line of the hall (the standing surface, y down).
pub const HALL_FLOOR_Y: i32 = 580;
/// Hall width in px.
pub const HALL_WIDTH: i32 = 3200;

/// Ticks Select must be held to restart the whole demo; a shorter press is a
/// respawn (1 s at 60 Hz).
pub const RESTART_HOLD_TICKS: u32 = 60;

/// One beat of the route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Beat {
    pub id: &'static str,
    pub name: &'static str,
    /// First column of the beat; also its checkpoint.
    pub from_x: i32,
    /// First column of the next beat (the hall end for the last).
    pub to_x: i32,
}

/// The route of the plan, section (b). B4 (llama) has no enemy yet: see the
/// hook below.
pub const BEATS: [Beat; 7] = [
    Beat {
        id: "B0",
        name: "Wake-up",
        from_x: 160,
        to_x: 520,
    },
    Beat {
        id: "B1",
        name: "First rematch",
        from_x: 520,
        to_x: 1020,
    },
    Beat {
        id: "B2",
        name: "Kick alley",
        from_x: 1020,
        to_x: 1520,
    },
    Beat {
        id: "B3",
        name: "Rotten shore",
        from_x: 1520,
        to_x: 2070,
    },
    Beat {
        id: "B4",
        name: "Bad Llama",
        from_x: 2070,
        to_x: 2620,
    },
    Beat {
        id: "B5",
        name: "Lit charge (QA test prop)",
        from_x: 2620,
        to_x: 2780,
    },
    Beat {
        id: "B6",
        name: "Pileup",
        from_x: 2780,
        to_x: 3200,
    },
];

/// Respawn columns: the start column of each beat (B0 is the spawn).
pub const CHECKPOINTS: [i32; 7] = [160, 520, 1020, 1520, 2070, 2620, 2780];

/// The rotten shore's strike column (CP-03), in the B3 stretch. The block's
/// footprint is `strike + 25 .. strike + 105`.
pub const SHORE_STRIKE_X: i32 = 1640;

const fn goblin(x: i32, ai: EnemyAi) -> EnemySpawn {
    EnemySpawn { x, ai }
}

/// Every goblin of the hall, in fixture index order. A chaser notices Carl
/// within `GOBLIN_SIGHT` (220 px) and stays put until then, so each beat's
/// goblins wake as Carl walks in; the passive ones (the shore victims) stay put
/// for the whole run.
///
/// | index | beat | role |
/// | --- | --- | --- |
/// | 0, 1 | B1 | two chasers |
/// | 2, 3 | B2 | two chasers that bunch up, a ledge face 60 px behind them |
/// | 4 | B3 | passive, mid footprint (crushed) |
/// | 5 | B3 | passive, standing on the footprint's right edge facing the block (DEATH-D: the edge decapitation) |
/// | 6 | B3 | chaser from the right |
/// | 7 | B4 | chaser in front of the llama |
/// | 8..=13 | B6 | six chasers in two clusters |
/// | 14, 15 | B5 | two passive goblins huddled on the QA test charge (point blank, ring) |
/// | 16 | B5 | one chaser from the right (the survivor) |
pub const DEMO_ENEMIES: [EnemySpawn; 17] = [
    goblin(820, EnemyAi::Chase),
    goblin(900, EnemyAi::Chase),
    goblin(1280, EnemyAi::Chase),
    goblin(1310, EnemyAi::Chase),
    goblin(1705, EnemyAi::Passive),
    goblin(1745, EnemyAi::Passive),
    goblin(1900, EnemyAi::Chase),
    goblin(2350, EnemyAi::Chase),
    goblin(3010, EnemyAi::Chase),
    goblin(3030, EnemyAi::Chase),
    goblin(3050, EnemyAi::Chase),
    goblin(3110, EnemyAi::Chase),
    goblin(3130, EnemyAi::Chase),
    goblin(3150, EnemyAi::Chase),
    goblin(B5_CHARGE_X, EnemyAi::Passive),
    goblin(B5_CHARGE_X + 28, EnemyAi::Passive),
    goblin(B5_CHASER_X, EnemyAi::Chase),
];

/// B4 (DEMO-01 PR B, the minimal Bad Llama): the llama's home column. The
/// llama stands behind the beat's chaser (goblin 7); the `demo01_b4` probe
/// kills it with push kicks (the lethal kick is heavy: the slam). Every beat
/// fixture carries it; a run that starts after its column retires it (like
/// the goblins behind the start).
pub const B4_LLAMA_HOME_X: i32 = 2420;

/// The demo's Bad Llama (beat B4). Its stand-off, leash and the fair-fire rule
/// keep it silent until Carl walks into its frame.
pub static DEMO01_LLAMAS: [LlamaSpawn; 1] = [LlamaSpawn {
    x: B4_LLAMA_HOME_X,
    ai: EnemyAi::Chase,
}];

/// B5 (DEATH-D, PR #108): the QA test charge lies at [`B5_CHARGE_X`] with two
/// passive goblins huddled on it and one chaser coming from the right. It is
/// armed when Carl passes column 2640 (`Arm::CarlPast`) and blows
/// [`B5_FUSE_TICKS`] ticks later: one point blank burst, one ring kill thrown
/// intact and one survivor. A **labelled test prop** (the beat name says so)
/// and never Carl's ability: nothing is given to Carl, the charge hurts nobody
/// and damages no terrain. Its goblins are appended at the end of
/// [`DEMO_ENEMIES`] so no index above changes.
pub const B5_CHARGE_ARM_X: i32 = 2640;
pub const B5_CHARGE_X: i32 = 2735;
pub const B5_CHASER_X: i32 = 2778;
pub const B5_FUSE_TICKS: u64 = 75;

/// The test charge of a run that starts at column `from`: its column is
/// relative to Carl's start (`ChargeSpawn::dx`).
const fn b5_charge(from: i32) -> ChargeSpawn {
    ChargeSpawn {
        dx: B5_CHARGE_X - from,
        tick: 0,
        owner: None,
        arm: Arm::CarlPast {
            x: B5_CHARGE_ARM_X as f32,
            fuse_ticks: B5_FUSE_TICKS,
        },
    }
}

const CHARGES_B0: [ChargeSpawn; 1] = [b5_charge(160)];
const CHARGES_B1: [ChargeSpawn; 1] = [b5_charge(520)];
const CHARGES_B2: [ChargeSpawn; 1] = [b5_charge(1020)];
const CHARGES_B3: [ChargeSpawn; 1] = [b5_charge(1520)];
const CHARGES_B4: [ChargeSpawn; 1] = [b5_charge(2070)];
const CHARGES_B5: [ChargeSpawn; 1] = [b5_charge(2620)];

/// The checkpoint rules of the demo.
pub static DEMO_RULES: DemoRules = DemoRules {
    checkpoints: &CHECKPOINTS,
};

const fn beat_fixture(
    name: &'static str,
    summary: &'static str,
    beat: usize,
    ticks: u64,
    charges: &'static [ChargeSpawn],
) -> Fixture {
    Fixture {
        name,
        summary,
        seed: 0xDE30_0000 + beat as u64,
        carl_x: CHECKPOINTS[beat],
        enemies: &DEMO_ENEMIES,
        shore_dx: Some(SHORE_STRIKE_X - CHECKPOINTS[beat]),
        boss: None,
        engineers: &[],
        llamas: &DEMO01_LLAMAS,
        charges,
        script: &[],
        ticks,
        room: crate::combat_scene::FixtureRoom::Corridor,
    }
}

/// One fixture per beat: Carl starts at the beat's checkpoint (a full restart
/// returns there too), nothing is scripted. Beat 0 is the live demo. The
/// probes drive them with [`autopilot`] (`--demo --demo-beat N --autopilot`).
/// They are not part of `FIXTURES`.
pub static BEAT_FIXTURES: [Fixture; 7] = [
    beat_fixture(
        "demo01",
        "DEMO-01: the continuous route B0 to B6 on the demo hall",
        0,
        2000,
        &CHARGES_B0,
    ),
    beat_fixture("demo01_b1", "B1 first rematch", 1, 200, &CHARGES_B1),
    beat_fixture("demo01_b2", "B2 kick alley", 2, 260, &CHARGES_B2),
    beat_fixture("demo01_b3", "B3 rotten shore", 3, 300, &CHARGES_B3),
    beat_fixture(
        "demo01_b4",
        "B4 bad llama: a chaser, then push kicks and the slam",
        4,
        400,
        &CHARGES_B4,
    ),
    beat_fixture(
        "demo01_b5",
        "B5 lit charge (QA test prop)",
        5,
        300,
        &CHARGES_B5,
    ),
    // A run that starts after B5 has no test charge (nor its goblins).
    beat_fixture("demo01_b6", "B6 pileup", 6, 300, &[]),
];

/// The fixture that starts at beat `beat`.
pub fn fixture_for_beat(beat: usize) -> &'static Fixture {
    &BEAT_FIXTURES[beat.min(BEAT_FIXTURES.len() - 1)]
}

/// Load the demo hall: the room geometry composed by the camera-lab composer
/// into an [`EnvKit`] with the two scroll-camera views the scene asks for and
/// no exit trigger (the demo never ends on reaching the portal; the portal is
/// scenery).
pub fn load_hall(root: &Path) -> Result<EnvKit, String> {
    let path = root.join(HALL_PATH);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let rooms = parse_rooms(&text)?;
    let room = rooms.first().ok_or("the hall file holds no room")?;
    let (mut kit, _contract, _plan) = build_kit(root, room).map_err(|e| e.to_string())?;
    // The scene's camera takes its vertical position from a named view. Same
    // framing as the corridor: the walking line 288 px below the top of the
    // 640x360 view (308 px below the top of the 640x400 Deck view).
    kit.views.insert(
        "abc_640x360".to_owned(),
        ViewDef {
            x: 0,
            y: HALL_FLOOR_Y - 288,
            w: 640,
            h: 360,
        },
    );
    kit.views.insert(
        "abc_640x400".to_owned(),
        ViewDef {
            x: 0,
            y: HALL_FLOOR_Y - 308,
            w: 640,
            h: 400,
        },
    );
    kit.exit = None;
    Ok(kit)
}

/// The beat Carl's column is in.
pub fn beat_at(x: f32) -> usize {
    CHECKPOINTS
        .iter()
        .rposition(|&c| x >= c as f32)
        .unwrap_or(0)
}

/// `DEMO01_COMPLETE` condition: every goblin and the llama of the hall are
/// defeated.
pub fn complete(combat: &CombatState) -> bool {
    !combat.enemies.is_empty()
        && combat.enemies.iter().all(|g| !g.alive())
        && combat.llamas.iter().all(|l| !l.alive())
}

/// What the Select button asks for this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectAction {
    None,
    /// A tap (released before [`RESTART_HOLD_TICKS`]): respawn now.
    Respawn,
    /// Held for [`RESTART_HOLD_TICKS`]: restart the whole demo.
    Restart,
}

/// Select (keyboard R, pad Select): a tap respawns at the last checkpoint on
/// release, holding for one second restarts the whole demo (once, when the
/// hold completes; releasing afterwards does nothing). Pure, ticked once per
/// fixed tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SelectControl {
    held: u32,
    fired: bool,
}

impl SelectControl {
    pub fn tick(&mut self, down: bool) -> SelectAction {
        if down {
            self.held += 1;
            if self.held == RESTART_HOLD_TICKS && !self.fired {
                self.fired = true;
                return SelectAction::Restart;
            }
            return SelectAction::None;
        }
        let tap = self.held > 0 && !self.fired;
        *self = Self::default();
        if tap {
            SelectAction::Respawn
        } else {
            SelectAction::None
        }
    }
}

// ----------------------------------------------------------------- autopilot --

/// Carl's reach, in px of centre distance, for a jab and for the push kick
/// (hit box plus the goblin's half width, with a margin), and for a kick on
/// the llama (its 46 px body reaches 42 px from its column).
const JAB_REACH: f32 = 30.0;
const KICK_REACH: f32 = 30.0;
const LLAMA_KICK_REACH: f32 = 36.0;
/// Ticks between two button presses of the autopilot (a human mashes slower).
const PRESS_CADENCE: u64 = 4;
/// Column Carl stands at to strike the shore.
const SHORE_STAND_X: f32 = SHORE_STRIKE_X as f32 - 20.0;

fn press(tick: u64) -> bool {
    tick.is_multiple_of(PRESS_CADENCE)
}

/// A deterministic stand-in for the player, from the finished simulation
/// state only: it walks the route, fights the nearest awake goblin (the push
/// kick in B2, jabs elsewhere), strikes the shore three times from the left in
/// B3 and moves on when the beat is cleared. Used by the headless probes and
/// the evidence captures; the live demo is driven by the keyboard or pad.
pub fn autopilot(combat: &CombatState, scene: &SceneSim) -> RawInput {
    let hero = &scene.world.hero;
    let tick = scene.world.tick + 1;
    let beat = combat.checkpoint;
    let towards = |x: f32| {
        let dx = x - hero.x;
        if dx.abs() < 4.0 { 0.0 } else { dx.signum() }
    };
    // The nearest awake goblin that still matters.
    let threat = combat
        .enemies
        .iter()
        .filter(|g| g.alive() && g.ai == EnemyAi::Chase)
        .min_by(|a, b| (a.x - hero.x).abs().total_cmp(&(b.x - hero.x).abs()));
    let mut raw = RawInput::default();
    // The Bad Llama (B4): once no goblin is awake, close in and push-kick it
    // (the lethal kick is the slam).
    if let Some(l) = combat.llamas.iter().find(|l| l.alive())
        && threat.is_none_or(|g| (g.x - hero.x).abs() >= 220.0)
        && (l.x - hero.x).abs() < 300.0
    {
        let dx = l.x - hero.x;
        raw.move_x = dx.signum();
        if dx.abs() <= LLAMA_KICK_REACH && press(tick) {
            raw.physical = true;
        }
        return raw;
    }
    // B5: walk just past the arming column, then let the fuse burn (the
    // probe stands still and does not fight until the test charge has blown,
    // so the blast, not Carl's jabs, takes the huddled goblins).
    if beat == 5 && combat.charges.iter().any(|c| !c.detonated) {
        raw.move_x = if hero.x < B5_CHARGE_ARM_X as f32 + 10.0 {
            1.0
        } else {
            0.0
        };
        return raw;
    }
    if let Some(g) = threat
        && (g.x - hero.x).abs() < 220.0
    {
        let dx = g.x - hero.x;
        let dir = dx.signum();
        let reach = if beat == 2 { KICK_REACH } else { JAB_REACH };
        if dx.abs() > reach {
            raw.move_x = dir;
        } else {
            // Turn towards the goblin together with the attack.
            raw.move_x = dir;
            if press(tick) {
                if beat == 2 {
                    raw.physical = true;
                } else {
                    raw.attack = true;
                }
            }
        }
        return raw;
    }
    // Nothing to fight: the beat's objective, then on to the next beat. Goblins
    // notice Carl at 220 px, so walking on wakes the next ones.
    let shore_open = combat.shore.as_ref().is_some_and(|shore| !shore.landed);
    let goal = if beat == 3 && shore_open && hero.x < SHORE_STRIKE_X as f32 {
        SHORE_STAND_X
    } else {
        CHECKPOINTS
            .get(beat + 1)
            .copied()
            .unwrap_or(HALL_WIDTH - 60) as f32
            + 8.0
    };
    raw.move_x = towards(goal);
    // Strike the shore from its left: face right, press on the cadence.
    if beat == 3
        && shore_open
        && (hero.x - SHORE_STAND_X).abs() < 6.0
        && combat.shore.as_ref().is_some_and(|s| s.hits < 3)
        && tick.is_multiple_of(40)
    {
        raw.attack = true;
        raw.move_x = 1.0;
    }
    // Jump a ledge face that blocks the way.
    if hero.grounded && raw.move_x != 0.0 && blocked_ahead(scene, raw.move_x) {
        raw.jump = true;
    }
    raw
}

/// A solid within 14 px ahead of Carl's body at ledge height.
fn blocked_ahead(scene: &SceneSim, dir: f32) -> bool {
    use crate::combat::solid_at;
    use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Rect};
    let hero = &scene.world.hero;
    let probe = Rect {
        left: hero.x - HERO_HALF_WIDTH + dir * 14.0,
        bottom: hero.y - HERO_HALF_HEIGHT + 4.0,
        width: HERO_HALF_WIDTH * 2.0,
        height: 8.0,
    };
    solid_at(&scene.world.solids, probe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_lab::layout::build_kit;
    use crate::camera_lab::room::parse_rooms;
    use crate::combat::{CombatEvent, DefeatCause, EnemyAi, ForceTier};
    use crate::combat_scene::FixtureSim;
    use crate::death_present::DeathPresenter;
    use crate::feel::RESTART_DELAY_TICKS;
    use crate::gore::{DeathArt, GorePreset};
    use crate::shore::{FOOTPRINT_WIDTH, FOOTPRINT_X_MIN};
    use std::sync::Arc;

    /// Final state hash of the whole-route autopilot run (`demo01`, 2000 ticks).
    const ROUTE_HASH: u64 = 0x0363_078d_5855_8e95;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    /// A demo run at beat `beat` (fixture, scene and demo rules).
    fn sim(kit: &EnvKit, beat: usize) -> FixtureSim {
        let fixture = fixture_for_beat(beat);
        let scene = SceneSim::new_at(kit, 360, fixture.carl_x).unwrap();
        let combat = CombatState::new(fixture, kit, &scene)
            .unwrap()
            .with_demo(&DEMO_RULES);
        FixtureSim { scene, combat }
    }

    fn autopilot_steps(run: &mut FixtureSim, ticks: u64) {
        for _ in 0..ticks {
            let raw = autopilot(&run.combat, &run.scene);
            run.step(raw);
        }
    }

    fn defeats(run: &FixtureSim) -> Vec<CombatEvent> {
        run.combat
            .events
            .iter()
            .copied()
            .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
            .collect()
    }

    fn count(run: &FixtureSim, pick: fn(&CombatEvent) -> bool) -> usize {
        run.combat.events.iter().filter(|e| pick(e)).count()
    }

    /// Every checkpoint respawns Carl outside the sight of the goblins ahead,
    /// so a defeat cannot chain into the next one.
    #[test]
    fn no_checkpoint_respawns_carl_inside_a_goblin_s_sight() {
        for checkpoint in CHECKPOINTS {
            for (index, spawn) in DEMO_ENEMIES.iter().enumerate() {
                // The B5 chaser (16) stands 158 px from the B5 checkpoint, on
                // purpose: it is the blast's survivor, and a run that starts at
                // B6 (2780) retires it (it stands left of 2780).
                if index == 16 {
                    continue;
                }
                if spawn.ai == EnemyAi::Chase && spawn.x >= checkpoint {
                    let gap = spawn.x - checkpoint;
                    assert!(
                        gap as f32 >= crate::feel::GOBLIN_SIGHT,
                        "goblin {index} is {gap} px from checkpoint {checkpoint}"
                    );
                }
            }
        }
    }

    /// The shore block and its cavity sit on masonry: no ground arch opening
    /// of the hall overlaps the strike column or the footprint.
    #[test]
    fn no_arch_opening_sits_behind_the_shore() {
        let text = std::fs::read_to_string(root().join(HALL_PATH)).unwrap();
        let room = parse_rooms(&text).unwrap().remove(0);
        let (_, _, plan) = build_kit(root(), &room).unwrap();
        let from = SHORE_STRIKE_X - 8;
        let to = SHORE_STRIKE_X + (FOOTPRINT_X_MIN + FOOTPRINT_WIDTH) as i32;
        assert!(!plan.openings.is_empty());
        for (x, _, w, _) in plan.openings {
            assert!(
                x + w <= from || x >= to,
                "opening {x}..{} behind the shore",
                x + w
            );
        }
    }

    /// Plan acceptance 3 (B6): the whole route defeats 14 goblins; at most 8
    /// settled corpses stay drawn and no stain is ever dropped.
    #[test]
    fn the_route_keeps_eight_corpses_and_every_stain() {
        let kit = load_hall(root()).unwrap();
        let art = Arc::new(DeathArt::load(root()).unwrap());
        let art_b = DeathArt::load_variant(root(), 1).unwrap();
        let mut presenter = DeathPresenter::with_variants(art, &art_b, GorePreset::Full).unwrap();
        let mut run = sim(&kit, 0);
        for _ in 0..BEAT_FIXTURES[0].ticks {
            let raw = autopilot(&run.combat, &run.scene);
            run.step(raw);
            presenter.observe(&run.combat, &run.scene);
        }
        // Fourteen goblins and the llama.
        assert_eq!(defeats(&run).len(), DEMO_ENEMIES.len() + 1);
        let line = presenter.state_line();
        let field = |key: &str| -> usize {
            line.split_whitespace()
                .find_map(|t| t.strip_prefix(key))
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(|| panic!("{key} in {line}"))
        };
        assert_eq!(field("deaths="), 8, "{line}");
        assert!(field("peak_floor_decals=") > 0, "{line}");
        assert_eq!(
            field("floor_decals="),
            field("peak_floor_decals="),
            "{line}"
        );
    }

    #[test]
    fn the_beat_table_is_contiguous_and_matches_the_checkpoints() {
        for (index, beat) in BEATS.iter().enumerate() {
            assert_eq!(beat.from_x, CHECKPOINTS[index], "{}", beat.id);
            if let Some(next) = BEATS.get(index + 1) {
                assert_eq!(beat.to_x, next.from_x, "{}", beat.id);
            }
        }
        assert_eq!(BEATS.last().unwrap().to_x, HALL_WIDTH);
        assert_eq!(beat_at(0.0), 0);
        assert_eq!(beat_at(1020.0), 2);
        assert_eq!(beat_at(3199.0), 6);
        assert_eq!(BEAT_FIXTURES.len(), BEATS.len());
        for (index, fixture) in BEAT_FIXTURES.iter().enumerate() {
            assert_eq!(fixture.carl_x, CHECKPOINTS[index]);
        }
    }

    /// Acceptance 2: the hall composes through the existing `EnvKit` and the
    /// A1 lighting budget, like every lab room.
    #[test]
    fn the_hall_composes_and_stays_inside_the_lighting_budget() {
        use crate::lighting::{MAX_LIGHTS, ViewRect};
        let kit = load_hall(root()).unwrap();
        assert_eq!((kit.width, kit.height), (HALL_WIDTH, 672));
        assert!(kit.exit.is_none());
        // At least 470 px of drawn wall above the walking line, 80 below.
        assert!(HALL_FLOOR_Y >= 470 && kit.height - HALL_FLOOR_Y >= 80);
        let scene = kit.lighting.as_ref().expect("lighting profile");
        let mut worst = 0;
        let mut left = 0;
        while left + 960 <= kit.width {
            for top in [0, 66, 132] {
                let (gathered, dropped) = scene.gather(
                    &kit,
                    ViewRect {
                        left,
                        top,
                        w: 960,
                        h: 540,
                    },
                    0,
                );
                assert_eq!(dropped, 0, "lights dropped at {left},{top}");
                worst = worst.max(gathered.lights.len());
            }
            left += 160;
        }
        assert!(worst <= MAX_LIGHTS, "{worst} lights");
        // The scroll camera's two views put the walking line where the corridor does.
        for (name, line_from_top) in [("abc_640x360", 288), ("abc_640x400", 308)] {
            let view = kit.view(name).unwrap();
            assert_eq!(HALL_FLOOR_Y - view.y, line_from_top, "{name}");
            assert!(view.y >= 0 && view.y + view.h <= kit.height, "{name}");
        }
    }

    /// Acceptance 2: a clean flat walking line under every spawn, checkpoint,
    /// the shore's strike column and footprint, and the end walls stop Carl.
    #[test]
    fn the_walking_line_is_clean_where_the_route_needs_it() {
        let kit = load_hall(root()).unwrap();
        let flat = |x: i32, what: &str| {
            assert_eq!(
                kit.walking_line_at(x),
                Some(HALL_FLOOR_Y),
                "{what} at x={x}"
            );
        };
        for x in CHECKPOINTS {
            flat(x, "checkpoint");
        }
        for (index, spawn) in DEMO_ENEMIES.iter().enumerate() {
            flat(spawn.x, &format!("goblin {index}"));
        }
        let footprint_end = SHORE_STRIKE_X + (FOOTPRINT_X_MIN + FOOTPRINT_WIDTH) as i32;
        for x in (SHORE_STRIKE_X - 40..=footprint_end + 40).step_by(4) {
            flat(x, "the shore stretch");
        }
        // The route has exactly two raised ledges (2 tiles), away from every
        // beat start.
        let in_ledge = |x: i32| (304..416).contains(&x) || (1360..1472).contains(&x);
        let raised: Vec<i32> = (0..HALL_WIDTH)
            .step_by(8)
            .filter(|x| kit.walking_line_at(*x) != Some(HALL_FLOOR_Y))
            .collect();
        assert!(raised.iter().any(|x| (304..416).contains(x)), "ledge 1");
        assert!(raised.iter().any(|x| (1360..1472).contains(x)), "ledge 2");
        for x in &raised {
            // The end walls (the outermost tile columns) are not floor either.
            let wall = *x < 16 || *x >= HALL_WIDTH - 16;
            assert!(in_ledge(*x) || wall, "unexpected raised column {x}");
        }
        // The end walls stop Carl.
        // (The scene alone: the goblins of B6 would body-block the walk.)
        let walk = |start: i32, dir: f32| {
            let mut scene = SceneSim::new_at(&kit, 360, start).unwrap();
            for _ in 0..600 {
                scene.step(RawInput {
                    move_x: dir,
                    ..RawInput::NONE
                });
            }
            scene.world.hero.x
        };
        let x = walk(CHECKPOINTS[0], -1.0);
        assert!((12.0..40.0).contains(&x), "left end {x}");
        let x = walk(CHECKPOINTS[6], 1.0);
        let end = HALL_WIDTH as f32;
        assert!(x <= end - 12.0 && x > end - 60.0, "right end {x}");
    }

    #[test]
    fn select_tap_respawns_and_a_one_second_hold_restarts_once() {
        let mut select = SelectControl::default();
        // A tap: nothing while down, respawn on release.
        assert_eq!(select.tick(true), SelectAction::None);
        assert_eq!(select.tick(true), SelectAction::None);
        assert_eq!(select.tick(false), SelectAction::Respawn);
        assert_eq!(select.tick(false), SelectAction::None);
        // The longest tap still respawns.
        for _ in 0..RESTART_HOLD_TICKS - 1 {
            assert_eq!(select.tick(true), SelectAction::None);
        }
        assert_eq!(select.tick(false), SelectAction::Respawn);
        // A hold restarts when the second is full, once; the release is silent.
        for held in 1..=RESTART_HOLD_TICKS + 30 {
            let expected = if held == RESTART_HOLD_TICKS {
                SelectAction::Restart
            } else {
                SelectAction::None
            };
            assert_eq!(select.tick(true), expected, "held {held}");
        }
        assert_eq!(select.tick(false), SelectAction::None);
        assert_eq!(select.tick(false), SelectAction::None);
    }

    /// Acceptance 4: knocked down, Carl stands at the last checkpoint after
    /// `RESTART_DELAY_TICKS` with full health; defeated goblins stay defeated
    /// and the living ones go back to their spawn.
    #[test]
    fn a_defeat_respawns_carl_at_the_checkpoint_and_the_dead_stay_dead() {
        let kit = load_hall(root()).unwrap();
        let mut run = sim(&kit, 1);
        // Kill goblin 0 with the autopilot, then stand still until goblin 1
        // takes Carl down.
        while run.combat.enemies[0].alive() {
            autopilot_steps(&mut run, 1);
            assert!(run.scene.world.tick < 400);
        }
        assert!(run.combat.enemies[1].alive());
        let mut defeated_at = None;
        for _ in 0..1500 {
            run.step(RawInput::default());
            if defeated_at.is_none() && run.combat.hero_defeated {
                defeated_at = Some(run.scene.world.tick);
            }
            if count(&run, |e| matches!(e, CombatEvent::Respawn { .. })) > 0 {
                break;
            }
        }
        let defeated_at = defeated_at.expect("Carl was knocked down");
        let respawn_tick = run
            .combat
            .events
            .iter()
            .find_map(|e| match e {
                CombatEvent::Respawn { tick, .. } => Some(*tick),
                _ => None,
            })
            .expect("a respawn event");
        assert!(respawn_tick >= defeated_at + u64::from(RESTART_DELAY_TICKS) - 1);
        assert!(respawn_tick <= defeated_at + u64::from(RESTART_DELAY_TICKS) + 1);
        let hero = &run.scene.world.hero;
        assert_eq!(hero.x, CHECKPOINTS[1] as f32);
        assert_eq!(hero.health, crate::combat::HERO_HEALTH);
        assert!(hero.grounded && !run.combat.hero_defeated);
        assert_eq!(run.combat.deaths, 1);
        assert!(!run.combat.enemies[0].alive(), "the dead stay dead");
        let g1 = &run.combat.enemies[1];
        assert!(
            g1.alive() && g1.x == DEMO_ENEMIES[1].x as f32,
            "the living go back"
        );
        // No Restart: the event log and the tick counter keep going.
        assert_eq!(count(&run, |e| matches!(e, CombatEvent::Restart { .. })), 0);
        assert!(run.scene.world.tick >= respawn_tick);
        // Carl fights on and clears the beat after the respawn.
        autopilot_steps(&mut run, 400);
        assert!(!run.combat.enemies[1].alive());
    }

    /// A Select tap respawns at once, mid-run; the checkpoint is the furthest
    /// beat entered, not the one Carl is standing in.
    #[test]
    fn a_tap_respawn_goes_to_the_furthest_beat_entered() {
        let kit = load_hall(root()).unwrap();
        let mut run = sim(&kit, 0);
        autopilot_steps(&mut run, 700);
        let cp = run.combat.checkpoint;
        assert!(cp >= 2, "checkpoint {cp}");
        // Walk back to B0, then respawn: still at the furthest checkpoint.
        for _ in 0..600 {
            run.step(RawInput {
                move_x: -1.0,
                ..RawInput::NONE
            });
        }
        assert!(run.scene.world.hero.x < CHECKPOINTS[cp] as f32);
        run.combat.respawn(&mut run.scene);
        assert_eq!(run.scene.world.hero.x, CHECKPOINTS[cp] as f32);
        assert_eq!(run.combat.checkpoint, cp);
        assert!(run.combat.take_respawned());
        assert!(!run.combat.take_respawned());
    }

    /// HUD-TRY-01 (HT-AD1): the HUD's attempt counts every attempt start: the
    /// entry, each checkpoint respawn and each restart, and none of it reaches
    /// the state hash.
    #[test]
    fn the_attempt_counts_respawns_and_restarts_without_hashing_them() {
        let kit = load_hall(root()).unwrap();
        let mut run = sim(&kit, 0);
        assert_eq!(run.combat.attempt(), 1);
        autopilot_steps(&mut run, 200);
        let hash = run.combat.state_hash(&run.scene);
        run.combat.respawns += 5;
        assert_eq!(run.combat.state_hash(&run.scene), hash, "not hashed");
        run.combat.respawns -= 5;
        run.combat.respawn(&mut run.scene);
        assert_eq!((run.combat.respawns, run.combat.attempt()), (1, 2));
        run.combat.respawn(&mut run.scene);
        assert_eq!(run.combat.attempt(), 3);
        run.combat.restart(&mut run.scene);
        assert_eq!((run.combat.restarts, run.combat.respawns), (1, 2));
        assert_eq!(run.combat.attempt(), 4, "a restart keeps the respawns");
    }

    /// The interactive demo runs the live party (Donut is a real companion
    /// hero): a respawn stands her up beside Carl with full health too.
    #[test]
    fn a_respawn_restores_the_live_party_companion() {
        use crate::party::PartyState;
        let kit = load_hall(root()).unwrap();
        let mut run = sim(&kit, 2);
        let party = PartyState::default();
        run.scene.set_drive(party.drive(true));
        for _ in 0..120 {
            let inputs = party.hero_inputs([
                RawInput {
                    move_x: 1.0,
                    ..RawInput::NONE
                },
                RawInput::default(),
            ]);
            run.combat.step_party(&mut run.scene, inputs);
        }
        assert!(run.scene.party_live());
        run.scene.world.donut_body.health = 0;
        run.combat.respawn(&mut run.scene);
        let carl = run.scene.world.hero.x;
        assert_eq!(carl, CHECKPOINTS[2] as f32);
        assert!(run.scene.world.donut_body.x < carl);
        assert_eq!(
            run.scene.world.donut_body.health,
            crate::combat::HERO_HEALTH
        );
        assert_eq!(run.scene.world.donut.x, run.scene.world.donut_body.x);
    }

    /// A full restart (the held Select) is the fixture `Restart`: beat 0 again,
    /// everyone back.
    #[test]
    fn a_full_restart_puts_everything_back() {
        let kit = load_hall(root()).unwrap();
        let mut run = sim(&kit, 0);
        autopilot_steps(&mut run, 1800);
        assert!(complete(&run.combat));
        run.restart();
        assert!(!complete(&run.combat));
        assert_eq!(run.scene.world.hero.x, CHECKPOINTS[0] as f32);
        assert_eq!(run.combat.checkpoint, 0);
        assert!(run.combat.enemies.iter().all(|g| g.alive()));
        assert_eq!(count(&run, |e| matches!(e, CombatEvent::Restart { .. })), 1);
    }

    /// A run that starts at a later beat does not carry the goblins of the
    /// beats behind it.
    #[test]
    fn starting_at_a_later_beat_retires_the_goblins_behind() {
        let kit = load_hall(root()).unwrap();
        let run = sim(&kit, 3);
        for (index, spawn) in DEMO_ENEMIES.iter().enumerate() {
            let behind = spawn.x < CHECKPOINTS[3];
            assert_eq!(run.combat.enemies[index].alive(), !behind, "goblin {index}");
        }
        assert_eq!(run.combat.checkpoint, 3);
    }

    /// Acceptance 4: the presenter keeps its gore across a respawn (and a
    /// restart still clears it).
    #[test]
    fn the_gore_stays_across_a_respawn() {
        let kit = load_hall(root()).unwrap();
        let art = Arc::new(DeathArt::load(root()).unwrap());
        let art_b = DeathArt::load_variant(root(), 1).unwrap();
        let mut presenter = DeathPresenter::with_variants(art, &art_b, GorePreset::Full).unwrap();
        let mut run = sim(&kit, 1);
        for _ in 0..190 {
            let raw = autopilot(&run.combat, &run.scene);
            run.step(raw);
            presenter.observe(&run.combat, &run.scene);
        }
        assert!(!run.combat.enemies[0].alive() && !run.combat.enemies[1].alive());
        let floor = presenter.live_floor_decals();
        assert!(floor > 0, "the two deaths left stains");
        let line = presenter.state_line();
        assert!(line.contains("deaths=2"), "{line}");
        run.combat.respawn(&mut run.scene);
        presenter.observe(&run.combat, &run.scene);
        assert_eq!(presenter.live_floor_decals(), floor, "stains stay");
        let line = presenter.state_line();
        assert!(line.contains("deaths=2"), "the corpses stay: {line}");
        run.restart();
        presenter.observe(&run.combat, &run.scene);
        assert_eq!(presenter.live_floor_decals(), 0, "a restart clears it");
    }

    /// Acceptance 3 (B1): two melee defeats, no deaths.
    #[test]
    fn probe_b1_two_melee_defeats() {
        let kit = load_hall(root()).unwrap();
        let mut run = sim(&kit, 1);
        autopilot_steps(&mut run, BEAT_FIXTURES[1].ticks);
        let defeats = defeats(&run);
        assert_eq!(defeats.len(), 2);
        for event in defeats {
            let CombatEvent::EnemyDefeated { cause, .. } = event else {
                unreachable!()
            };
            assert!(matches!(cause, DefeatCause::Melee), "{cause:?}");
        }
        assert_eq!(run.combat.deaths, 0);
    }

    /// Acceptance 3 (B2): a goblin-on-goblin contact and a lethal kick.
    #[test]
    fn probe_b2_a_launch_and_a_contact() {
        let kit = load_hall(root()).unwrap();
        let mut run = sim(&kit, 2);
        autopilot_steps(&mut run, BEAT_FIXTURES[2].ticks);
        let contacts = count(&run, |e| matches!(e, CombatEvent::GoblinContact { .. }));
        assert!(contacts >= 1, "{contacts} contacts");
        let heavy = defeats(&run)
            .into_iter()
            .filter(|e| {
                matches!(
                    e,
                    CombatEvent::EnemyDefeated {
                        force: ForceTier::Heavy,
                        ..
                    }
                )
            })
            .count();
        assert!(heavy >= 1, "a lethal kick");
        // A goblin that was kicked is hit again (relaunched or downed): two
        // kick hits on one target.
        let mut kicked = [0u8; 14];
        for event in &run.combat.events {
            if let CombatEvent::Hit {
                move_id: crate::combat::MoveId::PushKick,
                target,
                ..
            } = event
            {
                kicked[*target as usize] += 1;
            }
        }
        assert!(
            kicked.iter().any(|n| *n >= 2),
            "no goblin was hit twice by kicks: {kicked:?}"
        );
        // The ledge face stops a launched goblin: one B2 goblin ends against it.
        let face = 1360.0;
        let stopped = run.combat.enemies[2..4]
            .iter()
            .any(|g| face - g.x <= crate::combat::ENEMY_HALF_WIDTH + 2.0 && g.x < face);
        assert!(stopped, "no goblin stopped at the ledge face");
    }

    /// Acceptance 3 (B3): three hits break the shore and one landing gives one
    /// crush (goblin 4, mid footprint) and one guaranteed edge decapitation
    /// (goblin 5, standing on the footprint's right edge facing the block).
    #[test]
    fn probe_b3_the_shore_breaks_and_the_block_crushes() {
        let kit = load_hall(root()).unwrap();
        let mut run = sim(&kit, 3);
        autopilot_steps(&mut run, BEAT_FIXTURES[3].ticks);
        assert_eq!(
            count(&run, |e| matches!(e, CombatEvent::ShoreHit { .. })),
            3
        );
        assert_eq!(
            count(&run, |e| matches!(e, CombatEvent::ShoreBroken { .. })),
            1
        );
        assert_eq!(
            count(&run, |e| matches!(e, CombatEvent::ShoreLanded { .. })),
            1
        );
        let crushed = defeats(&run)
            .into_iter()
            .filter(|e| {
                matches!(
                    e,
                    CombatEvent::EnemyDefeated {
                        cause: DefeatCause::ShoreBlock { .. },
                        ..
                    }
                )
            })
            .count();
        assert_eq!(crushed, 1, "{crushed} crushed");
        let sheared: Vec<u32> = defeats(&run)
            .into_iter()
            .filter_map(|e| match e {
                CombatEvent::EnemyDefeated {
                    target,
                    cause: DefeatCause::ShoreBlockEdge { .. },
                    ..
                } => Some(target),
                _ => None,
            })
            .collect();
        assert_eq!(sheared, [5], "the edge decapitation is goblin 5");
        // The two come from the same landing.
        let landing = run
            .combat
            .events
            .iter()
            .find_map(|e| match e {
                CombatEvent::ShoreLanded { tick } => Some(*tick),
                _ => None,
            })
            .unwrap();
        let same_tick = defeats(&run)
            .into_iter()
            .filter(|e| {
                matches!(e, CombatEvent::EnemyDefeated { tick, cause, .. }
                    if *tick == landing
                        && matches!(cause, DefeatCause::ShoreBlock { .. } | DefeatCause::ShoreBlockEdge { .. }))
            })
            .count();
        assert_eq!(
            same_tick, 2,
            "one crush and one decapitation on one landing"
        );
    }

    /// Acceptance 3 (B5): the test charge arms when Carl passes column 2640
    /// and blows 75 ticks later: one point blank burst, one ring kill thrown
    /// intact and one survivor (the chaser), and Carl is untouched.
    #[test]
    fn probe_b5_the_lit_charge_bursts_one_throws_one_and_spares_one() {
        let kit = load_hall(root()).unwrap();
        let mut run = sim(&kit, 5);
        assert!(!run.combat.charges[0].lit(), "unlit until Carl passes");
        autopilot_steps(&mut run, BEAT_FIXTURES[5].ticks);
        let blast: Vec<u64> = run
            .combat
            .events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::Blast { tick, .. } => Some(*tick),
                _ => None,
            })
            .collect();
        assert_eq!(blast.len(), 1);
        let armed = run.combat.charges[0].armed_tick.expect("armed");
        assert_eq!(blast[0], armed + B5_FUSE_TICKS, "75 ticks after the arming");
        let explosions: Vec<(u32, bool)> = defeats(&run)
            .into_iter()
            .filter_map(|e| match e {
                CombatEvent::EnemyDefeated {
                    target,
                    cause: DefeatCause::Explosion { point_blank, .. },
                    ..
                } => Some((target, point_blank)),
                _ => None,
            })
            .collect();
        assert_eq!(explosions, [(14, true), (15, false)]);
        // The survivor of the blast is the chaser: alive when it went off.
        let chaser_death = defeats(&run).into_iter().find_map(|e| match e {
            CombatEvent::EnemyDefeated {
                tick,
                target: 16,
                cause: DefeatCause::Melee,
                ..
            } => Some(tick),
            _ => None,
        });
        assert!(
            chaser_death.is_some_and(|t| t > blast[0]),
            "{chaser_death:?}"
        );
        assert_eq!(run.combat.deaths, 0);
        assert_eq!(run.scene.world.hero.health, crate::combat::HERO_HEALTH);
    }

    /// The test charge and its huddle stand on the clean walking line, ahead
    /// of the arming column, and the beat names it a test prop.
    #[test]
    fn the_test_charge_stands_on_the_walking_line_and_is_labelled() {
        let kit = load_hall(root()).unwrap();
        let line = kit.walking_line_at(CHECKPOINTS[5]);
        for column in [B5_CHARGE_X, B5_CHARGE_X + 28, B5_CHASER_X] {
            assert_eq!(kit.walking_line_at(column), line, "column {column}");
        }
        const { assert!(B5_CHARGE_X > B5_CHARGE_ARM_X) };
        assert!(BEATS[5].name.contains("test prop"));
        assert!(BEAT_FIXTURES[5].summary.contains("test prop"));
        // A run that starts after B5 has no charge, and none starts armed.
        assert!(BEAT_FIXTURES[6].charges.is_empty());
        for fixture in &BEAT_FIXTURES[..6] {
            assert_eq!(fixture.charges.len(), 1);
            let spawn = fixture.charges[0];
            assert_eq!(fixture.carl_x + spawn.dx, B5_CHARGE_X);
            assert_eq!(
                spawn.arm,
                Arm::CarlPast {
                    x: B5_CHARGE_ARM_X as f32,
                    fuse_ticks: B5_FUSE_TICKS
                }
            );
            assert_eq!(spawn.owner, None, "no credit: it is nobody's ability");
        }
    }

    /// A respawn rebuilds a charge that has not blown (unlit, with the huddle
    /// reset); one that blew stays blown.
    #[test]
    fn a_respawn_unlights_a_charge_that_has_not_blown() {
        let kit = load_hall(root()).unwrap();
        let mut run = sim(&kit, 5);
        for _ in 0..40 {
            let raw = autopilot(&run.combat, &run.scene);
            run.step(raw);
        }
        assert!(run.combat.charges[0].lit() && !run.combat.charges[0].detonated);
        run.combat.respawn(&mut run.scene);
        assert!(!run.combat.charges[0].lit());
        assert!(run.combat.enemies[14].alive() && run.combat.enemies[15].alive());
    }

    /// Acceptance 3 (B6): six defeats. The corpse cap and the stains are the
    /// presenter's (DEATH-E1 pileup test); this is the simulation side.
    #[test]
    fn probe_b6_six_defeats() {
        let kit = load_hall(root()).unwrap();
        let mut run = sim(&kit, 6);
        autopilot_steps(&mut run, BEAT_FIXTURES[6].ticks);
        assert_eq!(defeats(&run).len(), 6);
        assert_eq!(run.combat.deaths, 0);
    }

    /// B4: the chaser falls to jabs, the autopilot closes in on the llama and
    /// push-kicks it: three hits, the lethal kick is heavy (the slam), and the
    /// llama's glob may hurt Carl but never defeats him.
    #[test]
    fn b4_the_llama_dies_to_a_lethal_kick_and_the_presenter_slams_it() {
        let kit = load_hall(root()).unwrap();
        let art = Arc::new(DeathArt::load(root()).unwrap());
        let art_b = DeathArt::load_variant(root(), 1).unwrap();
        let mut presenter = DeathPresenter::with_variants(art, &art_b, GorePreset::Full)
            .unwrap()
            .with_llama(Arc::new(DeathArt::load_llama(root()).unwrap()))
            .unwrap();
        let mut run = sim(&kit, 4);
        for _ in 0..BEAT_FIXTURES[4].ticks {
            let raw = autopilot(&run.combat, &run.scene);
            run.step(raw);
            presenter.observe(&run.combat, &run.scene);
        }
        let llama_target = DEMO_ENEMIES.len() as u32;
        let slam: Vec<_> = defeats(&run)
            .into_iter()
            .filter(|e| {
                matches!(e, CombatEvent::EnemyDefeated { target, force: ForceTier::Heavy, .. }
                    if *target == llama_target)
            })
            .collect();
        assert_eq!(slam.len(), 1, "one slam from a lethal kick");
        assert!(!run.combat.llamas[0].alive());
        assert_eq!(run.combat.deaths, 0);
        assert_eq!(presenter.pose_of(llama_target).map(|p| p.frame), Some(7));
        // A run that starts after the llama's column has no llama to fight.
        let later = sim(&kit, 5);
        assert!(!later.combat.llamas[0].alive());
    }

    /// The headless demo smoke: the autopilot walks the whole route from B0 to
    /// the last goblin without a death; the final state is pinned.
    #[test]
    fn the_route_smoke_clears_every_beat() {
        let kit = load_hall(root()).unwrap();
        let mut run = sim(&kit, 0);
        autopilot_steps(&mut run, BEAT_FIXTURES[0].ticks);
        assert!(complete(&run.combat));
        assert_eq!(run.combat.checkpoint, BEATS.len() - 1);
        assert_eq!(run.combat.deaths, 0);
        assert_eq!(defeats(&run).len(), DEMO_ENEMIES.len() + 1);
        let line = run.state_line();
        assert!(line.contains("demo_cp=6"), "{line}");
        assert_eq!(run.state_hash(), ROUTE_HASH, "{line}");
        // Deterministic: a second run gives the same hash.
        let mut again = sim(&kit, 0);
        autopilot_steps(&mut again, BEAT_FIXTURES[0].ticks);
        assert_eq!(again.state_hash(), run.state_hash());
    }
}
