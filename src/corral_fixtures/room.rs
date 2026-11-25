//! CORRAL-C: room 0, the Infernal Corral, on real geometry (plan KC16, spec
//! `enemy-gameplay-llama-rush.md` sections 10 and 11).
//!
//! One geometry file, [`ROOM_PATH`], loaded as an [`EnvKit`] by the fixtures and
//! the CORRAL-EVAL evaluator alike. Its raised surfaces are authored as
//! full-height steps (the CF02 parapet precedent: the `parapet` list of the room
//! file), never as platforms, so no gap under a surface leaks a bolt:
//!
//! * the optional entry shelf, 1120..1216, top 32 (CRL-Q29);
//! * the dock bumper, 1856..1888, top 80 (CRL-Q41);
//! * the exit dock and the M3 threshold, 1888..2304, top 32 (CRL-Q29).
//!
//! The two fixtures, [`CORRAL_M1`] and [`CORRAL_M2`], carry the llamas and their
//! opt-ins until CORRAL-WIRE moves the roster into `f01_corral.encounter.json`.
//! They run under CF02 rules through the exact-name hooks of the parent module
//! and are **tests only** (not in `PartyFixture::all()`): replaying them by name
//! needs the corral room kit, which is CORRAL-WIRE's corral scene arm.
//!
//! * `corral_m1`: the M1 llama at 848, no rush profile, the legacy spit, the
//!   legacy leash (848 +- 160) and party targeting (CRL-Q27).
//! * `corral_m2`: llama B on the pen floor (home 1488, leash 1248..1830) and
//!   llama A on the dock (home 1968, feet 32 px up, leash 1920..2016), both with
//!   the corral profile, facing -1 and dormant until the CRL-Q24 wake
//!   (CRL-Q25, CRL-Q28).

use std::path::Path;

use crate::boss_scene::Pilot;
use crate::camera_lab::room::{IRect, Room, parse_rooms};
use crate::combat::EnemyAi;
use crate::combat_scene::corral::LlamaOptInSet;
use crate::combat_scene::{Fixture, LlamaSpawn};
use crate::donut_mana::DonutSetup;
use crate::env_kit::EnvKit;
use crate::llama::{LlamaOptIn, RushOptIn};
use crate::llama_rush::{self, Leash};
use crate::party::PROVISIONAL_TUNING;
use crate::party_scene::PartyFixture;

/// The room file (KC16).
pub const ROOM_PATH: &str = "assets/levels/floor01/f01_corral.room.json";

/// The walking line of the room in world y (y up): the room's render height
/// (672) less its floor line (580). A test pins it to the loaded kit, so the
/// dock llama's authored surface is an exact world height.
pub const FLOOR_WORLD_Y: f32 = 92.0;

/// The authored steps, `(left, right, top above the walking line)` (CRL-Q29,
/// CRL-Q41). The room file is the source; a test pins the two together.
pub const ENTRY_SHELF: (i32, i32, i32) = (1120, 1216, 32);
pub const BUMPER: (i32, i32, i32) = (1856, 1888, 80);
pub const DOCK: (i32, i32, i32) = (1888, 2304, 32);

/// M1 (CRL-Q27): the first llama, its legacy home.
pub const M1_X: i32 = 848;
/// M2 (CRL-Q28): llama B on the pen floor, llama A on the dock.
pub const B_X: i32 = 1488;
pub const B_LEASH: Leash = Leash {
    min: 1248.0,
    max: 1830.0,
};
pub const A_X: i32 = 1968;
pub const A_LEASH: Leash = Leash {
    min: 1920.0,
    max: 2016.0,
};
/// The dock's height above the walking line (A's surface).
pub const DOCK_RISE: f32 = 32.0;

/// Spec section 11: Carl is parked here in the decisive (`isolated`) variant,
/// 368 px or more from every position B can take (CRL-Q15's 320 px range).
pub const CARL_PARK_M1: i32 = 300;
pub const CARL_PARK_M2: i32 = 880;
/// Spec section 11: Carl's start on the companion AI (`default`, reported).
pub const CARL_ASSIST_M2: i32 = 1040;

/// The isolated M2 horizon (CORRAL-AUTO section 3) is the fixtures' tick count.
const TICKS: u64 = 1800;

static M1_BASE: Fixture = Fixture {
    name: "corral_m1",
    summary: "Room 0, M1 One Burning Throat: one legacy llama at 848 in a flat pocket with no cover (party targeting, no rush)",
    seed: 0xC0DC_0001,
    carl_x: CARL_PARK_M1,
    enemies: &[],
    shore_dx: None,
    boss: None,
    engineers: &[],
    llamas: &[LlamaSpawn {
        x: M1_X,
        ai: EnemyAi::Chase,
    }],
    charges: &[],
    script: &[],
    ticks: TICKS,
    room: crate::combat_scene::FixtureRoom::Corridor,
};

static M2_BASE: Fixture = Fixture {
    name: "corral_m2",
    summary: "Room 0, M2 the Infernal Corral: rush llama B on the pen floor at 1488, rush llama A on the exit dock at 1968 behind the 80 px bumper",
    seed: 0xC0DC_0002,
    carl_x: CARL_PARK_M2,
    enemies: &[],
    shore_dx: None,
    boss: None,
    engineers: &[],
    llamas: &[
        LlamaSpawn {
            x: B_X,
            ai: EnemyAi::Chase,
        },
        LlamaSpawn {
            x: A_X,
            ai: EnemyAi::Chase,
        },
    ],
    charges: &[],
    script: &[],
    ticks: TICKS,
    room: crate::combat_scene::FixtureRoom::Corridor,
};

/// `corral_m1`: solo Carl, the M1 llama (a test drives it; the evaluator
/// builds its own seats on the same base).
pub static CORRAL_M1: PartyFixture = PartyFixture {
    base: &M1_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::unlocked(8),
};

/// `corral_m2`: solo Carl, the two M2 llamas.
pub static CORRAL_M2: PartyFixture = PartyFixture {
    base: &M2_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::unlocked(8),
};

/// The room's base fixtures, for the evaluator's seats.
pub fn m1_base() -> &'static Fixture {
    &M1_BASE
}

pub fn m2_base() -> &'static Fixture {
    &M2_BASE
}

/// Whether `name` is one of the room fixtures.
pub fn is_room_fixture(name: &str) -> bool {
    name == M1_BASE.name || name == M2_BASE.name
}

/// CORRAL-WIRE: the fixture name of the launched room (`--f01-room corral`,
/// `src/f01_corral.rs`). Its llamas are the sidecar's, in roster order: the M1
/// llama (index 0), then B (1) and A (2), at the columns above; the launch
/// refuses a sidecar whose llamas are elsewhere. It is not a room fixture of
/// this module ([`is_room_fixture`]): it runs under the room rules of the late
/// launcher, not under the `corral_` name hooks of `cf02_rules`.
pub const WIRED_NAME: &str = "f01_corral";

/// The llama columns of the launched room, in its llama index order.
pub const WIRED_LLAMAS: [i32; 3] = [M1_X, B_X, A_X];

/// CRL-Q27: no profile, the legacy leash and spit, party targeting.
fn m1_opt_in(index: usize) -> LlamaOptIn {
    LlamaOptIn {
        index,
        feet: None,
        facing: 0,
        dormant: false,
        health: None,
        rush: None,
    }
}

/// CRL-Q25, CRL-Q28: B on the pen floor and A on the dock, both with the corral
/// profile, facing -1 and dormant until the CRL-Q24 wake.
fn m2_opt_ins(b: usize, a: usize) -> [LlamaOptIn; 2] {
    let profile = llama_rush::active();
    [
        LlamaOptIn {
            index: b,
            feet: None,
            facing: -1,
            dormant: true,
            health: None,
            rush: Some(RushOptIn {
                profile,
                leash: B_LEASH,
            }),
        },
        LlamaOptIn {
            index: a,
            feet: Some(FLOOR_WORLD_Y + DOCK_RISE),
            facing: -1,
            dormant: true,
            health: None,
            rush: Some(RushOptIn {
                profile,
                leash: A_LEASH,
            }),
        },
    ]
}

/// The opt-ins of a room fixture (R1, R2, KC15) or of the launched room
/// ([`WIRED_NAME`]), `None` for every other name.
pub fn llama_opt_ins(name: &str) -> Option<LlamaOptInSet> {
    let opt_ins = if name == M1_BASE.name {
        vec![m1_opt_in(0)]
    } else if name == M2_BASE.name {
        m2_opt_ins(0, 1).to_vec()
    } else if name == WIRED_NAME {
        let mut all = vec![m1_opt_in(0)];
        all.extend(m2_opt_ins(1, 2));
        all
    } else {
        return None;
    };
    Some(LlamaOptInSet {
        party_target: true,
        opt_ins,
        solids: Vec::new(),
    })
}

/// The room and its authored full-height steps (layout px, floor up), read
/// from the room file's `parapet` list (the CF02 microarena's field).
pub fn load_room(root: &Path) -> Result<(Room, Vec<IRect>), String> {
    let path = root.join(ROOM_PATH);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let room = parse_rooms(&text)?
        .into_iter()
        .next()
        .ok_or("the corral file holds no room")?;
    let file: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let mut steps = Vec::new();
    for p in file["scenarios"][0]["parapet"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let field = |k: &str| {
            p[k].as_i64()
                .map(|v| v as i32)
                .ok_or_else(|| format!("{ROOM_PATH}: parapet {k}"))
        };
        let (x, w, h) = (field("x")?, field("w")?, field("h")?);
        if w <= 0 || h <= 0 || h % crate::camera_lab::room::TILE != 0 {
            return Err(format!(
                "{ROOM_PATH}: parapet at {x} must have a positive width and a whole-tile height"
            ));
        }
        steps.push(IRect::new(x, room.floor_y - h, w, h));
    }
    Ok((room, steps))
}

/// The room's kit: composed like the other Floor 01 setpiece rooms (no
/// generated staircases, the two scroll-camera views) with its authored steps.
pub fn load_kit(root: &Path) -> Result<EnvKit, String> {
    let (room, steps) = load_room(root)?;
    crate::f01_boiler::compose_kit_with_steps(root, &room, &steps)
}
