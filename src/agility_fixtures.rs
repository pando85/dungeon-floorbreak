//! DONUT-MOBILITY-01 (DM-A) motion fixtures: named, deterministic scripted
//! body runs of one hero on the real [`step_hero_motion`], and the `dm_`
//! party fixture of the 2P wall.
//!
//! A [`MotionFixture`] is a flat floor (top at y = 0), an optional ledge, one
//! hero standing on the floor and a script of [`Hold`]s. [`run`] steps it with
//! an explicit [`HeroMotion`] and returns the trail and its FNV-1a hash (the
//! golden each fixture pins, one per motion). Nothing here is registered in
//! [`crate::party_scene::PartyFixture::all`]: these are tests of the motion,
//! not of a scene, so no CLI name and no committed replay depend on them.
//!
//! Tick numbering: a [`Hold`] over `from..to` covers the steps `from..to - 1`
//! (0-based), and a jump held from `from` gives its press edge on `from`.

use crate::boss_scene::Pilot;
use crate::combat_scene::Fixture;
use crate::donut_mana::DonutSetup;
use crate::input::{InputFrame, RawInput};
use crate::party::{PROVISIONAL_TUNING, PartyTuning};
use crate::party_scene::{PartyFixture, SlotHold, base, jump, run as run_hold};
use crate::prototype::{
    Block, BodyDrive, HERO_HALF_HEIGHT, Hero, HeroMotion, Material, Rect, step_hero_motion,
};

/// One held input of a [`MotionFixture`] script.
#[derive(Debug, Clone, Copy)]
pub struct Hold {
    pub from: u32,
    pub to: u32,
    pub move_x: f32,
    pub jump: bool,
}

/// A scripted body run on a flat floor with an optional ledge.
#[derive(Debug)]
pub struct MotionFixture {
    pub name: &'static str,
    pub summary: &'static str,
    /// Hero centre x at tick 0 (feet on the floor).
    pub start_x: f32,
    /// A ledge `(left, width, height)` above the floor, or none.
    pub ledge: Option<(f32, f32, f32)>,
    pub script: &'static [Hold],
    pub ticks: u32,
}

/// Where the ledge fixtures stand: the hero touches the ledge's left face.
const LEDGE_LEFT: f32 = 200.0;
const LEDGE_W: f32 = 400.0;
/// Hero centre x at the face of the ledge.
const AT_FACE_X: f32 = LEDGE_LEFT - crate::prototype::HERO_HALF_WIDTH;

/// Hold right the whole time and jump on tick 0: the hold-into-the-wall climb.
const CLIMB: [Hold; 2] = [
    Hold {
        from: 0,
        to: 90,
        move_x: 1.0,
        jump: false,
    },
    Hold {
        from: 0,
        to: 1,
        move_x: 0.0,
        jump: true,
    },
];

const fn ledge_fixture(name: &'static str, summary: &'static str, height: f32) -> MotionFixture {
    MotionFixture {
        name,
        summary,
        start_x: AT_FACE_X,
        ledge: Some((LEDGE_LEFT, LEDGE_W, height)),
        script: &CLIMB,
        ticks: 90,
    }
}

/// 60 ticks of full right input on flat ground (spec test 1).
pub const DM_RUN_RACE: MotionFixture = MotionFixture {
    name: "dm_run_race",
    summary: "Full right input on flat ground for 60 ticks",
    start_x: 100.0,
    ledge: None,
    script: &[Hold {
        from: 0,
        to: 60,
        move_x: 1.0,
        jump: false,
    }],
    ticks: 60,
};

pub const DM_JUMP_LEDGE_96: MotionFixture = ledge_fixture(
    "dm_jump_ledge_96",
    "Hold-into-the-wall jump at a 96 px ledge: Donut lands, Carl does not",
    96.0,
);
pub const DM_JUMP_LEDGE_104: MotionFixture = ledge_fixture(
    "dm_jump_ledge_104",
    "The same at the boss rack's 104 px ledge",
    104.0,
);
pub const DM_JUMP_LEDGE_112: MotionFixture = ledge_fixture(
    "dm_jump_ledge_112",
    "The same at LAB-R-Q14's 112 px perch (a 2.9 px margin)",
    112.0,
);
pub const DM_JUMP_LEDGE_116_FAILS: MotionFixture = ledge_fixture(
    "dm_jump_ledge_116_fails",
    "The same at 116 px: out of reach, she falls back",
    116.0,
);
/// Carl's own ledge (spec test 3): he lands on 80 and not on 96.
pub const DM_JUMP_LEDGE_80: MotionFixture = ledge_fixture(
    "dm_jump_ledge_80",
    "The same at 80 px (Carl lands on it)",
    80.0,
);

/// Every motion fixture, in a stable order.
pub const MOTION_FIXTURES: [&MotionFixture; 6] = [
    &DM_RUN_RACE,
    &DM_JUMP_LEDGE_80,
    &DM_JUMP_LEDGE_96,
    &DM_JUMP_LEDGE_104,
    &DM_JUMP_LEDGE_112,
    &DM_JUMP_LEDGE_116_FAILS,
];

/// The blocks of a fixture: a wide floor (top at y = 0) and the ledge.
pub fn blocks_of(fixture: &MotionFixture) -> Vec<Block> {
    let block = |left, bottom, width, height| Block {
        rect: Rect {
            left,
            bottom,
            width,
            height,
        },
        material: Material::Stone,
        intact: true,
    };
    let mut blocks = vec![block(-64.0, -64.0, 4000.0, 64.0)];
    if let Some((left, width, height)) = fixture.ledge {
        blocks.push(block(left, 0.0, width, height));
    }
    blocks
}

/// What a run leaves behind.
#[derive(Debug, Clone)]
pub struct Trail {
    /// `(x, feet_y, vy, grounded)` after each tick.
    pub states: Vec<(f32, f32, f32, bool)>,
    /// FNV-1a 64 over every tick's `x`, `y` and `vy` bits.
    pub hash: u64,
}

impl Trail {
    pub fn last(&self) -> (f32, f32, f32, bool) {
        *self.states.last().expect("a run has at least one tick")
    }

    /// The highest feet height reached.
    pub fn apex(&self) -> f32 {
        self.states.iter().map(|s| s.1).fold(0.0, f32::max)
    }
}

/// Step `fixture` with `motion`.
pub fn run(motion: HeroMotion, fixture: &MotionFixture) -> Trail {
    run_on(motion, fixture, &blocks_of(fixture), 4000.0)
}

/// [`run`] against explicit `blocks` and `max_x`.
pub fn run_on(motion: HeroMotion, fixture: &MotionFixture, blocks: &[Block], max_x: f32) -> Trail {
    let mut hero = Hero::at(fixture.start_x, HERO_HALF_HEIGHT);
    let mut frame = InputFrame::default();
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut states = Vec::with_capacity(fixture.ticks as usize);
    for tick in 0..fixture.ticks {
        let mut raw = RawInput::default();
        for hold in fixture
            .script
            .iter()
            .filter(|h| (h.from..h.to).contains(&tick))
        {
            raw.jump |= hold.jump;
            if hold.move_x != 0.0 {
                raw.move_x = hold.move_x;
            }
        }
        frame.advance(raw);
        step_hero_motion(
            &mut hero,
            motion,
            &frame,
            blocks,
            max_x,
            &BodyDrive::default(),
        );
        for value in [hero.x, hero.y, hero.vy] {
            for byte in value.to_bits().to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        states.push((hero.x, hero.y - HERO_HALF_HEIGHT, hero.vy, hero.grounded));
    }
    Trail { states, hash }
}

/// DM-Q09 / spec section 2.1: P2 (Donut) runs right with a running jump every
/// 60 ticks for 600 ticks while P1 (Carl) stands still, so the separation wall
/// is the only thing between her and the end of the room.
const WALL_RUN: [SlotHold; 11] = [
    run_hold(1, 601, 1, 1.0),
    jump(5, 6, 1),
    jump(65, 66, 1),
    jump(125, 126, 1),
    jump(185, 186, 1),
    jump(245, 246, 1),
    jump(305, 306, 1),
    jump(365, 366, 1),
    jump(425, 426, 1),
    jump(485, 486, 1),
    jump(545, 546, 1),
];

const DM_2P_WALL_BASE: Fixture = base(
    "dm_2p_wall_running_jump",
    "Two players: Donut runs and makes running jumps away from a still Carl for 600 ticks",
    0x0D4D_0001,
    &[],
    600,
);

/// Not registered in [`PartyFixture::all`] (see the module docs).
pub const DM_2P_WALL_RUNNING_JUMP: PartyFixture = PartyFixture {
    base: &DM_2P_WALL_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &WALL_RUN,
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

/// GIF 1 of the evidence: both heroes start in Carl's column and run right on
/// the corridor for 120 ticks; she ends 44 px ahead.
const RACE_2P: [SlotHold; 2] = [run_hold(2, 122, 0, 1.0), run_hold(2, 122, 1, 1.0)];

const DM_RACE_2P_BASE: Fixture = base(
    "dm_run_race_2p",
    "Two players from the same column: both run right for 120 ticks; Donut ends 44 px ahead",
    0x0D4D_0002,
    &[],
    130,
);

pub const DM_RUN_RACE_2P: PartyFixture = PartyFixture {
    base: &DM_RACE_2P_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &RACE_2P,
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

/// GIF 2: Donut 60 px behind Carl, a standing jump at tick 10, then both run right
/// and make a running jump at tick 80. Her apex is 114.9 px against his 88.5.
const JUMP_2P: [SlotHold; 6] = [
    jump(10, 11, 0),
    jump(10, 11, 1),
    run_hold(60, 160, 0, 1.0),
    run_hold(60, 160, 1, 1.0),
    jump(80, 81, 0),
    jump(80, 81, 1),
];

const DM_JUMP_2P_BASE: Fixture = base(
    "dm_jump_race_2p",
    "Two players, Donut 60 px behind Carl: a standing jump, then a running jump; she jumps higher and further",
    0x0D4D_0003,
    &[],
    170,
);

pub const DM_JUMP_RACE_2P: PartyFixture = PartyFixture {
    base: &DM_JUMP_2P_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &JUMP_2P,
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// ---------------------------------------------------------------- DM-B --
//
// The reflex fixtures. Every one starts with the biscuit and a full pool unless
// noted (evasion exists only after it). Names follow the spec (section 5); the
// two that need `cf02_rules` and the arena kit (`dm_reflex_ram_passes`,
// `dm_reflex_glob_passes`) are keyed by name prefix elsewhere, so they are built
// by the tests through `PartyFixtureSim::from_parts` and are not name-resolvable.

use crate::combat::EnemyAi;
use crate::combat_scene::EnemySpawn;

const fn chase(x: i32) -> EnemySpawn {
    EnemySpawn {
        x,
        ai: EnemyAi::Chase,
    }
}

const GOBLIN_AT_720: [EnemySpawn; 1] = [chase(720)];

/// Carl stands at column 400 and a goblin at 720: out of its sight (220) until
/// Donut walks up to it, so the first swing lands only when the script says.
const fn goblin_base(name: &'static str, summary: &'static str, seed: u64, ticks: u64) -> Fixture {
    Fixture {
        carl_x: 400,
        ..base(name, summary, seed, &GOBLIN_AT_720, ticks)
    }
}

/// An armed Donut meets a goblin's swing: P2 Donut walks up to a sleeping goblin
/// after the reflex armed at tick 720, the first swing is evaded, the second lands.
const DM_REFLEX_EVADES_SWING_BASE: Fixture = goblin_base(
    "dm_reflex_evades_swing",
    "Two players, biscuit from the start: Donut walks into a goblin after tick 720; the first swing is evaded and the next lands",
    0x0D4D_0010,
    1000,
);

pub const DM_REFLEX_EVADES_SWING: PartyFixture = PartyFixture {
    base: &DM_REFLEX_EVADES_SWING_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[run_hold(620, 700, 1, 1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::unlocked(8),
};

const DM_REFLEX_NOT_ARMED_BASE: Fixture = goblin_base(
    "dm_reflex_not_armed",
    "Two players, biscuit from the start: Donut walks into the goblin at tick 40, long before 720; the swing lands",
    0x0D4D_0011,
    300,
);

pub const DM_REFLEX_NOT_ARMED: PartyFixture = PartyFixture {
    base: &DM_REFLEX_NOT_ARMED_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[run_hold(20, 100, 1, 1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::unlocked(8),
};

/// Locked: no biscuit, 800 hurt-free ticks, then Donut walks into the goblin.
/// The swing lands: the reflex is a story unlock and nothing else.
const DM_EVASION_LOCKED_BASE: Fixture = goblin_base(
    "dm_evasion_locked_before_biscuit",
    "Two players, no biscuit: 900 hurt-free ticks pass, then Donut walks into the goblin and its swing lands",
    0x0D4D_0012,
    1000,
);

pub const DM_EVASION_LOCKED_BEFORE_BISCUIT: PartyFixture = PartyFixture {
    base: &DM_EVASION_LOCKED_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[run_hold(800, 880, 1, 1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

/// The placeholder Guild zone of this fixture: Donut's pivot inside `400..440`
/// grants the biscuit (the corridor has no Guild room yet).
pub const DM_GUILD_ZONE: (f32, f32) = (400.0, 440.0);

const DM_EVASION_UNLOCKS_BASE: Fixture = goblin_base(
    "dm_evasion_unlocks_at_guild",
    "Two players, locked at the start: Donut enters the guild zone, the reflex arms 720 ticks later and evades the next swing",
    0x0D4D_0013,
    1000,
);

pub const DM_EVASION_UNLOCKS_AT_GUILD: PartyFixture = PartyFixture {
    base: &DM_EVASION_UNLOCKS_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[run_hold(10, 30, 1, 1.0), run_hold(652, 712, 1, 1.0)],
    pilot: Pilot::None,
    donut: DonutSetup {
        start: crate::donut_mana::StartState {
            unlocked: false,
            mana: 0,
        },
        guild_zone: Some(DM_GUILD_ZONE),
    },
};

/// Human and AI parity (spec test 11): the same room twice, Donut human-driven
/// (two players, P2 idle) and AI-driven (solo, the follow bot), standing at the
/// same column. The test brings the goblin in on tick 690 so the swing lands
/// after the reflex armed; the reflex reads nothing of who drives her.
const DM_AI_PARITY_BASE: Fixture = goblin_base(
    "dm_ai_reflex_same_as_human",
    "Donut human-driven and AI-driven at the same column: the same evade decisions",
    0x0D4D_0014,
    1000,
);

pub const DM_AI_REFLEX_HUMAN: PartyFixture = PartyFixture {
    base: &DM_AI_PARITY_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::unlocked(8),
};

/// The AI twin: solo, Donut is the follow bot, which does not jump a tell here
/// (`dodge_radius_px` 0) and counts no farther for the goblin's target choice
/// (`target_ai_bias_px` 0), so her body and her attacker are those of the human twin.
pub const DM_AI_REFLEX_SAME_AS_HUMAN: PartyFixture = PartyFixture {
    base: &DM_AI_PARITY_BASE,
    two_player: false,
    tuning: PartyTuning {
        dodge_radius_px: 0,
        target_ai_bias_px: 0,
        ..PROVISIONAL_TUNING
    },
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::unlocked(8),
};

// ---------------------------------------------------------------- DM-C --

/// A single `attack` press on `slot` at `tick`.
const fn press(tick: u64, slot: usize) -> SlotHold {
    SlotHold {
        from: tick,
        to: tick + 1,
        slot,
        input: crate::input::RawInput {
            attack: true,
            ..crate::input::RawInput::NONE
        },
    }
}

/// The reflex cues in one room (DM-C evidence): under `cf02_rules` (the smug
/// overlay is a CF02-I acting overlay), biscuit from the start. P2 Donut walks
/// up to the sleeping goblin and takes its first swing on the armed reflex
/// (armed 720, evaded 729), steps back out of its reach, and the smug follows
/// the evade response; Carl walks up and jabs the goblin down; nothing hurts
/// her, so the reflex arms again 720 ticks after the evade (1449). The name
/// carries the `cf02_` prefix because that prefix is what runs a fixture under
/// `cf02_rules`.
const DM_CUES_BASE: Fixture = goblin_base(
    "cf02_dm_reflex_cues",
    "DM-C evidence: the biscuit from the start; Donut's armed reflex evades the first swing (tick 729), she steps back and is smug, Carl kills the goblin, and the reflex arms again on tick 1449",
    0x0D4D_00C1,
    1800,
);

const DM_CUES_SCRIPT: [SlotHold; 13] = [
    run_hold(620, 700, 1, 1.0),
    run_hold(736, 756, 1, -1.0),
    run_hold(740, 806, 0, 1.0),
    press(812, 0),
    press(826, 0),
    press(840, 0),
    press(854, 0),
    press(868, 0),
    press(882, 0),
    press(896, 0),
    press(910, 0),
    press(924, 0),
    press(938, 0),
];

pub const CF02_DM_REFLEX_CUES: PartyFixture = PartyFixture {
    base: &DM_CUES_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &DM_CUES_SCRIPT,
    pilot: Pilot::None,
    donut: DonutSetup::unlocked(8),
};

/// DM-C evidence (GIF 4): the microarena with nobody in it. P2 Donut runs to the
/// left perch, makes a running jump onto it (80 px up; her apex is 114.92), then
/// a standing jump from the perch: she bonks under the DM-Q27 lane ceiling at
/// feet 176, her drawn top stays under the HUD band. The name carries the
/// `cf02_arena_` prefix, which selects the arena kit and `cf02_rules`.
const DM_PERCH_BASE: Fixture = Fixture {
    carl_x: 300,
    enemies: &[],
    engineers: &[],
    shore_dx: None,
    llamas: &[],
    ..crate::cf02::arena::fixture(
        "cf02_arena_dm_perch_jump",
        "DM-C evidence: the empty microarena; Donut jumps onto the left perch and jumps again under the lane ceiling (DM-Q27)",
        0x0D4D_00C2,
        260,
    )
};

const DM_PERCH_SCRIPT: [SlotHold; 4] = [
    run_hold(1, 74, 1, 1.0),
    jump(74, 75, 1),
    run_hold(75, 100, 1, 1.0),
    jump(150, 151, 1),
];

pub const CF02_ARENA_DM_PERCH_JUMP: PartyFixture = PartyFixture {
    base: &DM_PERCH_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &DM_PERCH_SCRIPT,
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

/// The party fixtures of this module, resolvable by name in the binary
/// (`--party-fixture dm_run_race_2p`) through
/// [`crate::cf02_fixtures::find_party_fixture`], like the CF02 and Engineer
/// registries: not chained into [`PartyFixture::all`], so no registry-order
/// test, listing or committed replay depends on them.
pub const AGILITY_PARTY_FIXTURES: [&PartyFixture; 9] = [
    &DM_2P_WALL_RUNNING_JUMP,
    &DM_RUN_RACE_2P,
    &DM_JUMP_RACE_2P,
    // DM-B: the reflex fixtures that run without `cf02_rules`.
    &DM_REFLEX_EVADES_SWING,
    &DM_REFLEX_NOT_ARMED,
    &DM_EVASION_LOCKED_BEFORE_BISCUIT,
    &DM_EVASION_UNLOCKS_AT_GUILD,
    // DM-C: the cue evidence room, under `cf02_rules`.
    &CF02_DM_REFLEX_CUES,
    &CF02_ARENA_DM_PERCH_JUMP,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agility;
    use crate::input::STICK_DEADZONE;
    use crate::party::{HeroId, MotionProfile, PartyTuning};
    use crate::party_scene::PartyFixtureSim;
    use crate::prototype::{GRAVITY, RUN_SPEED};
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    const CARL: HeroMotion = HeroMotion::CARL;

    fn donut() -> HeroMotion {
        agility::agile_motion()
    }

    /// A standing jump on flat ground: apex, air ticks and the landing x.
    fn standing_jump(motion: HeroMotion) -> (f32, u32, f32) {
        const SCRIPT: [Hold; 1] = [Hold {
            from: 0,
            to: 1,
            move_x: 0.0,
            jump: true,
        }];
        let fixture = MotionFixture {
            name: "probe",
            summary: "",
            start_x: 100.0,
            ledge: None,
            script: &SCRIPT,
            ticks: 120,
        };
        let trail = run(motion, &fixture);
        let air = trail.states.iter().take_while(|s| !s.3).count() as u32 + 1;
        (trail.apex(), air, trail.last().0)
    }

    #[test]
    fn run_ratio_donut_207_carl_185() {
        let d = run(donut(), &DM_RUN_RACE);
        let c = run(CARL, &DM_RUN_RACE);
        assert!((d.last().0 - 100.0 - 207.0).abs() < 0.01, "{:?}", d.last());
        assert!((c.last().0 - 100.0 - 185.0).abs() < 0.01, "{:?}", c.last());
    }

    #[test]
    fn jump_numbers_donut_and_carl() {
        let (apex, air, _) = standing_jump(donut());
        assert!((apex - 114.92).abs() < 0.05, "{apex}");
        assert_eq!(air, 57);
        let (apex, air, _) = standing_jump(CARL);
        assert!((apex - 88.54).abs() < 0.05, "{apex}");
        assert_eq!(air, 50);
        // The helper agrees with the spec's discrete numbers.
        assert_eq!(agility::air_ticks(500.0), 57);
        assert_eq!(agility::air_ticks(440.0), 50);
    }

    #[test]
    fn running_jump_distance_is_196_65_and_154_17() {
        const SCRIPT: [Hold; 2] = [
            Hold {
                from: 0,
                to: 120,
                move_x: 1.0,
                jump: false,
            },
            Hold {
                from: 0,
                to: 1,
                move_x: 0.0,
                jump: true,
            },
        ];
        let fixture = MotionFixture {
            name: "probe",
            summary: "",
            start_x: 100.0,
            ledge: None,
            script: &SCRIPT,
            ticks: 120,
        };
        let flat = |motion: HeroMotion| {
            let trail = run(motion, &fixture);
            let landed = trail
                .states
                .iter()
                .position(|s| s.3)
                .expect("she lands within 120 ticks");
            // The landing tick's x minus the start, less the one grounded step.
            trail.states[landed].0 - 100.0
        };
        let d = flat(donut());
        let c = flat(CARL);
        assert!((d - 196.65).abs() < 0.05, "{d}");
        assert!((c - 154.17).abs() < 0.05, "{c}");
    }

    fn lands_on(motion: HeroMotion, fixture: &MotionFixture) -> bool {
        let (left, width, height) = fixture.ledge.unwrap();
        let (x, feet, _, grounded) = run(motion, fixture).last();
        grounded && (feet - height).abs() < 0.01 && x > left && x < left + width
    }

    #[test]
    fn ledges_donut_96_104_112_not_116_and_carl_80_not_96() {
        assert!(lands_on(donut(), &DM_JUMP_LEDGE_96));
        assert!(lands_on(donut(), &DM_JUMP_LEDGE_104));
        assert!(lands_on(donut(), &DM_JUMP_LEDGE_112));
        assert!(!lands_on(donut(), &DM_JUMP_LEDGE_116_FAILS));
        let trail = run(donut(), &DM_JUMP_LEDGE_116_FAILS);
        assert!(trail.apex() < 116.0, "{}", trail.apex());
        assert!(lands_on(CARL, &DM_JUMP_LEDGE_80));
        assert!(!lands_on(CARL, &DM_JUMP_LEDGE_96));
    }

    /// Spec test 3, nav part: the agile nav graph agrees with the body on the
    /// same ledges. Donut's floor-to-ledge jump edge is open at 96, 104 and 112
    /// and blocked at 116; Carl's is open at 80 and blocked at 96.
    #[test]
    fn the_agile_nav_graph_agrees_with_the_body_on_the_ledges() {
        use crate::companion::nav::{EdgeState, NavConfig, NavGraph, NavLevel};
        let edge_open = |fixture: &MotionFixture, hero: HeroId| {
            let (_, _, height) = fixture.ledge.unwrap();
            // The nav model needs x >= 0: shift the fixture's blocks right.
            let blocks: Vec<Block> = blocks_of(fixture)
                .into_iter()
                .map(|mut b| {
                    b.rect.left += 100.0;
                    b
                })
                .collect();
            let level = NavLevel::from_blocks(&blocks, Vec::new());
            let graph = NavGraph::build(&level, NavConfig::agile()).unwrap();
            let top = |id| graph.anchor_pos(id).unwrap().1;
            let views = graph.edge_views();
            let edge = views
                .iter()
                .find(|e| top(e.from) == 0.0 && (top(e.to) - height).abs() < 0.01)
                .unwrap_or_else(|| panic!("no floor to {height} px ledge edge"));
            matches!(edge.state[hero.index()], EdgeState::Open { .. })
        };
        assert!(edge_open(&DM_JUMP_LEDGE_96, HeroId::Donut));
        assert!(edge_open(&DM_JUMP_LEDGE_104, HeroId::Donut));
        assert!(edge_open(&DM_JUMP_LEDGE_112, HeroId::Donut));
        assert!(!edge_open(&DM_JUMP_LEDGE_116_FAILS, HeroId::Donut));
        assert!(edge_open(&DM_JUMP_LEDGE_80, HeroId::Carl));
        assert!(!edge_open(&DM_JUMP_LEDGE_96, HeroId::Carl));
    }

    #[test]
    fn the_ledge_margins_and_the_ai_reach_follow_the_spec() {
        let apex = agility::discrete_apex_px(500.0);
        assert!((apex - 96.0 - 18.92).abs() < 0.05);
        assert!((apex - 104.0 - 10.92).abs() < 0.05);
        assert!((apex - 112.0 - 2.92).abs() < 0.05);
        // DM-Q08: the bot tries 96 and 104 and not 112; Carl keeps 88.19.
        let reach = crate::follower::jump_reach_px(
            HeroId::Donut,
            &PartyTuning {
                donut_motion: MotionProfile::Agile,
                ..PROVISIONAL_TUNING
            },
        );
        assert!((reach - 110.92).abs() < 0.05, "{reach}");
        assert!((104.0..112.0).contains(&reach));
        let carl = crate::follower::jump_reach_px(HeroId::Carl, &PROVISIONAL_TUNING);
        let want = 440.0_f32 * 440.0 / (2.0 * GRAVITY) - 4.0;
        assert_eq!(carl.to_bits(), want.to_bits(), "Carl's reach is exact");
    }

    /// The pinned trail hashes: `(fixture, Donut, Carl)`. New fixtures, so these
    /// are pinned from their first run; the physical asserts above are the
    /// independent check of what they mean.
    const GOLDENS: [(&str, u64, u64); 6] = [
        ("dm_run_race", 0x2ae0_186b_4acb_82fc, 0x72bf_98d2_af54_d78f),
        (
            "dm_jump_ledge_80",
            0x60a8_797d_fb88_2e9a,
            0x070f_c689_2249_8f9f,
        ),
        (
            "dm_jump_ledge_96",
            0x52ea_16ba_5dbb_e9b9,
            0xdc3d_d096_ab9b_f76a,
        ),
        (
            "dm_jump_ledge_104",
            0xdba7_0fce_f225_b018,
            0xdc3d_d096_ab9b_f76a,
        ),
        (
            "dm_jump_ledge_112",
            0x36f7_fb9e_0765_4583,
            0xdc3d_d096_ab9b_f76a,
        ),
        (
            "dm_jump_ledge_116_fails",
            0xac75_4b69_40b1_cc9f,
            0xdc3d_d096_ab9b_f76a,
        ),
    ];

    #[test]
    fn the_motion_fixtures_match_their_goldens_and_replay_identically() {
        assert_eq!(GOLDENS.len(), MOTION_FIXTURES.len());
        for (fixture, (name, donut_hash, carl_hash)) in MOTION_FIXTURES.iter().zip(GOLDENS) {
            assert_eq!(fixture.name, name);
            let d = run(donut(), fixture);
            let c = run(CARL, fixture);
            assert_eq!(d.hash, run(donut(), fixture).hash, "{name} twice");
            assert_eq!(
                (d.hash, c.hash),
                (donut_hash, carl_hash),
                "{name}: {:#018x} {:#018x}",
                d.hash,
                c.hash
            );
        }
    }

    #[test]
    fn carl_motion_is_the_shared_step_bit_for_bit() {
        for fixture in MOTION_FIXTURES {
            let blocks = blocks_of(fixture);
            let mut a = Hero::at(fixture.start_x, HERO_HALF_HEIGHT);
            let mut b = a.clone();
            let mut frame = InputFrame::default();
            for tick in 0..fixture.ticks {
                let mut raw = RawInput::default();
                for hold in fixture
                    .script
                    .iter()
                    .filter(|h| (h.from..h.to).contains(&tick))
                {
                    raw.jump |= hold.jump;
                    if hold.move_x != 0.0 {
                        raw.move_x = hold.move_x;
                    }
                }
                frame.advance(raw);
                crate::prototype::step_hero_driven(
                    &mut a,
                    &frame,
                    &blocks,
                    4000.0,
                    &BodyDrive::default(),
                );
                step_hero_motion(
                    &mut b,
                    HeroMotion::CARL,
                    &frame,
                    &blocks,
                    4000.0,
                    &BodyDrive::default(),
                );
                assert_eq!(a.x.to_bits(), b.x.to_bits());
                assert_eq!(a.y.to_bits(), b.y.to_bits());
                assert_eq!(a.vy.to_bits(), b.vy.to_bits());
            }
        }
        assert_eq!(HeroMotion::CARL.run_speed, RUN_SPEED);
    }

    // --- the live scene: the motion is applied every tick ---------------------

    use crate::corridor::SceneSim;
    use crate::env_kit::EnvKit;
    use crate::party::PartyDrive;

    const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

    fn kit() -> EnvKit {
        EnvKit::load(&root(), LAYOUT).unwrap()
    }

    fn agile_drive(two_player: bool, controlled: HeroId) -> PartyDrive {
        let mut bots = [false, false];
        if !two_player {
            bots[controlled.other().index()] = true;
        }
        PartyDrive {
            live: true,
            two_player,
            controlled,
            bots,
            tuning: PartyTuning {
                donut_motion: MotionProfile::Agile,
                ..PROVISIONAL_TUNING
            },
        }
    }

    fn right() -> RawInput {
        RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        }
    }

    /// Donut's x advance over 10 ticks of a full right input on flat ground.
    fn donut_advance(scene: &mut SceneSim, inputs: impl Fn() -> [RawInput; 2]) -> f32 {
        let x0 = scene.world.donut_body.x;
        for _ in 0..10 {
            scene.step_party(inputs());
        }
        scene.world.donut_body.x - x0
    }

    fn assert_agile(advance: f32, what: &str) {
        let want = 10.0 * 207.0 / 60.0;
        assert!((advance - want).abs() < 0.05, "{what}: {advance} vs {want}");
    }

    #[test]
    fn donut_is_agile_after_the_live_start_a_restart_a_respawn_a_swap_and_in_2p() {
        let kit = kit();
        // 2P: both run right (Donut slot 1), the wall is far.
        let mut scene = SceneSim::new(&kit, 360).unwrap();
        scene.set_drive(agile_drive(true, HeroId::Carl));
        // Carl stands still so the wall never bites; Donut runs.
        let go = || [RawInput::default(), right()];
        assert_agile(donut_advance(&mut scene, go), "2P");
        // Restart: bodies rebuilt by Hero::at, nothing stored to reset.
        scene.reset();
        scene.set_drive(agile_drive(true, HeroId::Carl));
        assert_agile(donut_advance(&mut scene, go), "restart");
        // Checkpoint respawn.
        scene.respawn_at(500.0);
        assert_agile(donut_advance(&mut scene, go), "respawn");
        // Solo, Donut the controlled hero: her input is slot 0.
        let mut solo = SceneSim::new(&kit, 360).unwrap();
        solo.set_drive(agile_drive(false, HeroId::Donut));
        solo.respawn_at(500.0);
        assert_agile(
            donut_advance(&mut solo, || [RawInput::default(), right()]),
            "solo",
        );
        // After a swap (the controlled hero changes, the body keeps its motion).
        solo.set_drive(agile_drive(false, HeroId::Carl));
        solo.set_drive(agile_drive(false, HeroId::Donut));
        assert_agile(
            donut_advance(&mut solo, || [RawInput::default(), right()]),
            "after a swap",
        );
    }

    #[test]
    fn carl_is_never_agile_whatever_the_party_tuning() {
        let kit = kit();
        let mut scene = SceneSim::new(&kit, 360).unwrap();
        scene.set_drive(agile_drive(true, HeroId::Carl));
        scene.respawn_at(500.0);
        let x0 = scene.world.hero.x;
        for _ in 0..10 {
            scene.step_party([right(), RawInput::default()]);
        }
        let want = 10.0 * 185.0 / 60.0;
        assert!(
            (scene.world.hero.x - x0 - want).abs() < 0.05,
            "{} vs {want}",
            scene.world.hero.x - x0
        );
    }

    #[test]
    fn the_legacy_profile_is_carls_motion_and_agile_is_hers() {
        assert_eq!(MotionProfile::Legacy.motion(), HeroMotion::CARL);
        assert_eq!(MotionProfile::Agile.motion().run_speed, 207.0);
        assert_eq!(MotionProfile::Agile.motion().jump_speed, 500.0);
    }

    #[test]
    fn the_2p_wall_holds_while_donut_runs_and_jumps_for_600_ticks() {
        let mut sim = PartyFixtureSim::new(
            &kit(),
            360,
            // The fixture's own tuning, with the agile motion explicit so the
            // test holds whichever profile ships.
            Box::leak(Box::new(PartyFixture {
                tuning: PartyTuning {
                    donut_motion: MotionProfile::Agile,
                    ..DM_2P_WALL_RUNNING_JUMP.tuning
                },
                ..DM_2P_WALL_RUNNING_JUMP
            })),
        )
        .unwrap();
        let max = sim.party.tuning().max_separation_px(640) as f32;
        let mut widest = 0.0_f32;
        let mut jumped = false;
        for _ in 0..600 {
            sim.step_scripted();
            let w = &sim.scene.world;
            widest = widest.max((w.hero.x - w.donut_body.x).abs());
            jumped |= !w.donut_body.grounded;
        }
        assert!(jumped, "she made running jumps");
        assert!(widest > max - 4.0, "she reached the wall: {widest}");
        assert!(widest <= max + 0.01, "{widest} vs {max}");
    }

    #[test]
    fn the_race_fixtures_show_her_44_px_ahead_and_her_taller_jump() {
        let mut sim = PartyFixtureSim::new(&kit(), 360, &DM_RUN_RACE_2P).unwrap();
        assert_eq!(sim.scene.world.donut_body.x, sim.scene.world.hero.x);
        let start = sim.scene.world.hero.x;
        sim.run_scripted(120);
        let (carl, donut) = (sim.scene.world.hero.x, sim.scene.world.donut_body.x);
        // 120 ticks of input: 59 + ... the run starts on tick 2, 120 steps.
        assert!(
            ((donut - start) - (carl - start) - 44.0).abs() < 1.0,
            "{carl} {donut}"
        );
        let mut sim = PartyFixtureSim::new(&kit(), 360, &DM_JUMP_RACE_2P).unwrap();
        let (mut carl_apex, mut donut_apex) = (0.0_f32, 0.0_f32);
        let floor = sim.scene.world.hero.y - HERO_HALF_HEIGHT;
        for _ in 0..170 {
            sim.step_scripted();
            carl_apex = carl_apex.max(sim.scene.world.hero.y - HERO_HALF_HEIGHT - floor);
            donut_apex = donut_apex.max(sim.scene.world.donut_body.y - HERO_HALF_HEIGHT - floor);
        }
        assert!((carl_apex - 88.54).abs() < 0.1, "{carl_apex}");
        assert!((donut_apex - 114.92).abs() < 0.1, "{donut_apex}");
    }

    #[test]
    fn the_dm_party_fixtures_replay_identically_and_match_their_goldens() {
        const PARTY_GOLDENS: [(&str, u64); 9] = [
            ("dm_2p_wall_running_jump", 0xa53e_1ee5_fa99_5334),
            ("dm_run_race_2p", 0x7193_59bc_ab34_44ae),
            ("dm_jump_race_2p", 0x0d0c_c1c7_d90b_346d),
            // DM-B: pinned again by `agility_reflex_tests` with their events.
            ("dm_reflex_evades_swing", 0x4aca_1b4e_e7b3_9bbe),
            ("dm_reflex_not_armed", 0x8281_2a44_46dd_6c78),
            ("dm_evasion_locked_before_biscuit", 0x5a19_8a6c_5553_6f0d),
            ("dm_evasion_unlocks_at_guild", 0x1f1c_3dfb_5495_417e),
            // DM-C: pinned from its first run; the events are asserted in
            // `donut_cues::tests`.
            ("cf02_dm_reflex_cues", 0xff5e_7ce8_007f_28ac),
            ("cf02_arena_dm_perch_jump", 0x4fa9_bab3_c208_d618),
        ];
        assert_eq!(PARTY_GOLDENS.len(), AGILITY_PARTY_FIXTURES.len());
        for (fixture, (name, golden)) in AGILITY_PARTY_FIXTURES.iter().zip(PARTY_GOLDENS) {
            assert_eq!(fixture.name(), name);
            assert!(crate::cf02_fixtures::find_party_fixture(name).is_some());
            let run = || {
                let kit = if crate::cf02::arena::fixture_uses_arena(name) {
                    crate::cf02::arena::load_arena(&root()).unwrap()
                } else {
                    kit()
                };
                let mut sim = PartyFixtureSim::new(&kit, 360, fixture).unwrap();
                sim.run_scripted(fixture.base.ticks);
                sim.state_hash()
            };
            let hash = run();
            assert_eq!(hash, run(), "{name} twice");
            assert_eq!(hash, golden, "{name}: {hash:#018x}");
        }
    }

    /// Spec section 2.1 (Engineer counterplay row, goblin swing, charge): the
    /// ticks of a standing jump with her feet above a box of a given height,
    /// counting the press tick as tick 1 (`states[i]` is after tick `i + 1`).
    #[test]
    fn a_standing_jump_clears_the_ram_the_swing_and_the_charge_for_longer() {
        const SCRIPT: [Hold; 1] = [Hold {
            from: 0,
            to: 1,
            move_x: 0.0,
            jump: true,
        }];
        let probe = MotionFixture {
            name: "probe",
            summary: "",
            start_x: 100.0,
            ledge: None,
            script: &SCRIPT,
            ticks: 80,
        };
        let above = |motion: HeroMotion, height: f32| {
            let trail = run(motion, &probe);
            let ticks: Vec<usize> = trail
                .states
                .iter()
                .enumerate()
                .filter(|(_, s)| s.1 > height)
                .map(|(i, _)| i + 1)
                .collect();
            (ticks[0], *ticks.last().unwrap(), ticks.len())
        };
        // The Engineer's ram box is 33 px high: Donut is over it on ticks 5..51
        // (47 ticks), Carl on 6..44 (39).
        let (d, c) = (above(donut(), 33.0), above(CARL, 33.0));
        assert_eq!((d.0, d.1, d.2), (5, 51, 47), "ram, Donut");
        assert_eq!((c.0, c.1, c.2), (6, 44, 39), "ram, Carl");
        // The goblin swing box is 30 px high (impact ticks 26..30): over it from
        // tick 4 (Carl 5), so a tell dodged by a jump started on attack tick 0..20
        // still clears it (`DODGE_LATEST_TELL_TICK`).
        assert_eq!(above(donut(), 30.0).0, 4);
        assert_eq!(above(CARL, 30.0).0, 5);
        // The boss charge's 72 px ram: 35 ticks for Donut, 22 for Carl.
        assert_eq!(above(donut(), 72.0).2, 35);
        assert_eq!(above(CARL, 72.0).2, 22);
    }

    #[test]
    fn no_play_path_constructs_the_legacy_motion() {
        // DM-A: Legacy exists for tests and for the playtrace rebuild of a
        // pre-DM recording. This list is every source file allowed to name it;
        // a new use must be added here on purpose.
        let allowed = [
            "agility.rs",
            "agility_fixtures.rs",
            "party.rs",
            "follower.rs",
            "playtrace/replay.rs",
            "playtrace/manifest.rs",
            "playtrace/tests_motion.rs",
            "playtrace/tests_verify.rs",
            "autoplay/party/scenario.rs",
            // CA2-B2: the bolt sweep runs under both motions (spec 7.2).
            "cf02_free_fixtures.rs",
            "companion/nav.rs",
            "companion/navfollow.rs",
        ];
        let mut offenders = Vec::new();
        let mut stack = vec![root().join("src")];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let text = std::fs::read_to_string(&path).unwrap();
                    if text.contains("MotionProfile::Legacy") {
                        let rel = path.strip_prefix(root().join("src")).unwrap();
                        let rel = rel.to_string_lossy().replace('\\', "/");
                        if !allowed.contains(&rel.as_str()) {
                            offenders.push(rel);
                        }
                    }
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "unexpected Legacy uses: {offenders:?}"
        );
    }

    #[test]
    fn the_deadzone_leftover_is_what_the_arrive_tolerance_covers() {
        // The invariant the nav executor relies on (see arrive_eps_px).
        for run in [RUN_SPEED, 207.0] {
            let leftover = STICK_DEADZONE * run / 60.0;
            let eps = crate::companion::navfollow::arrive_eps_px(run);
            assert!(eps > leftover, "{run}: {eps} vs {leftover}");
        }
    }
}
