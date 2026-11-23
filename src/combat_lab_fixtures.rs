//! COMBAT-LAB-01 LAB-00 fixtures: the interned combat fixtures of the lab
//! rooms and the scripted probes the tests (and later the headless CI step)
//! drive. Bevy-free.
//!
//! A [`crate::combat_scene::Fixture`] is `'static` by contract (the whole
//! simulation borrows it). The lab builds its fixtures from the manifest, so
//! [`intern_fixture`] leaks one fixture per distinct key (room, variant, seed,
//! start column, roster) and hands out the same reference afterwards: a run
//! that launches a room a thousand times leaks one fixture, not a thousand.
//! The probes are plain `const` data, like the fixtures of
//! [`crate::party_scene`].

use std::sync::Mutex;

use crate::combat::EnemyAi;
use crate::combat_lab::PlayerMode;
use crate::combat_scene::{DemoRules, EnemySpawn, Fixture, LlamaSpawn};
use crate::input::{RawInput, SlotInputs};
use crate::party_scene::SlotHold;

/// Everything that makes two lab fixtures different.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureKey {
    pub name: String,
    pub seed: u64,
    pub carl_x: i32,
    /// `(column, passive)` of each goblin, in roster order.
    pub goblins: Vec<(i32, bool)>,
    /// `(column, passive)` of each llama, in roster order.
    pub llamas: Vec<(i32, bool)>,
    /// The shore's strike column relative to Carl's start column.
    pub shore_dx: Option<i32>,
    pub ticks: u64,
}

static INTERNED: Mutex<Vec<(FixtureKey, &'static Fixture)>> = Mutex::new(Vec::new());

fn ai(passive: bool) -> EnemyAi {
    if passive {
        EnemyAi::Passive
    } else {
        EnemyAi::Chase
    }
}

/// The stand-in of a lab launch while the arguments are still being parsed:
/// the scene binary resolves the real interned fixture of the room right
/// after (`LabCatalog::launch`). It is not in any fixture registry
/// (`Fixture::by_name` never returns it).
pub const PENDING_FIXTURE: Fixture = Fixture {
    name: "lab_pending",
    summary: "COMBAT-LAB-01 launch placeholder (replaced by the room fixture)",
    seed: 0,
    carl_x: 0,
    enemies: &[],
    shore_dx: None,
    boss: None,
    engineers: &[],
    llamas: &[],
    charges: &[],
    script: &[],
    ticks: 1,
    room: crate::combat_scene::FixtureRoom::Corridor,
};

static RULES: Mutex<Vec<(Vec<i32>, &'static DemoRules)>> = Mutex::new(Vec::new());

/// The `'static` checkpoint rules of a room (one leak per distinct list).
pub fn intern_rules(checkpoints: &[i32]) -> &'static DemoRules {
    let mut table = RULES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, rules)) = table.iter().find(|(c, _)| c == checkpoints) {
        return rules;
    }
    let list: &'static [i32] = Box::leak(checkpoints.to_vec().into_boxed_slice());
    let rules: &'static DemoRules = Box::leak(Box::new(DemoRules { checkpoints: list }));
    table.push((checkpoints.to_vec(), rules));
    rules
}

/// The `'static` fixture of `key`, built once.
pub fn intern_fixture(key: FixtureKey) -> &'static Fixture {
    let mut table = INTERNED.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, fixture)) = table.iter().find(|(k, _)| *k == key) {
        return fixture;
    }
    let name: &'static str = Box::leak(key.name.clone().into_boxed_str());
    let goblins: &'static [EnemySpawn] = Box::leak(
        key.goblins
            .iter()
            .map(|&(x, passive)| EnemySpawn { x, ai: ai(passive) })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    );
    let llamas: &'static [LlamaSpawn] = Box::leak(
        key.llamas
            .iter()
            .map(|&(x, passive)| LlamaSpawn { x, ai: ai(passive) })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    );
    let fixture: &'static Fixture = Box::leak(Box::new(Fixture {
        name,
        summary: "COMBAT-LAB-01 room fixture (built from the lab manifest)",
        seed: key.seed,
        carl_x: key.carl_x,
        enemies: goblins,
        shore_dx: key.shore_dx,
        boss: None,
        llamas,
        engineers: &[],
        charges: &[],
        script: &[],
        ticks: key.ticks,
        room: crate::combat_scene::FixtureRoom::Corridor,
    }));
    table.push((key, fixture));
    fixture
}

/// How many distinct fixtures were built so far (a test of the interning).
pub fn interned_count() -> usize {
    INTERNED.lock().unwrap_or_else(|e| e.into_inner()).len()
}

// ------------------------------------------------------------------- probes --

/// A scripted lab scenario: a room, a variant, a player mode and slot inputs.
/// The headless runner replays it tick by tick; it is the reproducible
/// scenario of a LAB-00 mechanic test.
#[derive(Debug)]
pub struct Probe {
    pub name: &'static str,
    pub summary: &'static str,
    pub room: &'static str,
    pub variant: &'static str,
    pub mode: PlayerMode,
    pub ticks: u64,
    pub script: &'static [SlotHold],
}

impl Probe {
    /// The slot inputs of `tick` (1-based): every hold that covers it, merged.
    pub fn slot_inputs(&self, tick: u64) -> SlotInputs {
        let mut out = [RawInput::default(); 2];
        for hold in self
            .script
            .iter()
            .filter(|h| (h.from..h.to).contains(&tick))
        {
            let raw = &mut out[hold.slot];
            raw.jump |= hold.input.jump;
            raw.swap |= hold.input.swap;
            raw.attack |= hold.input.attack;
            raw.physical |= hold.input.physical;
            if hold.input.move_x != 0.0 {
                raw.move_x = hold.input.move_x;
            }
        }
        out
    }

    pub fn by_name(name: &str) -> Option<&'static Probe> {
        PROBES.iter().copied().find(|p| p.name == name)
    }
}

const fn hold(from: u64, to: u64, slot: usize, input: RawInput) -> SlotHold {
    SlotHold {
        from,
        to,
        slot,
        input,
    }
}

const fn walk(from: u64, to: u64, slot: usize, move_x: f32) -> SlotHold {
    hold(
        from,
        to,
        slot,
        RawInput {
            move_x,
            ..RawInput::NONE
        },
    )
}

/// A held primary press (Carl's jab, Donut's Magic Missile).
const fn primary(from: u64, to: u64, slot: usize) -> SlotHold {
    hold(
        from,
        to,
        slot,
        RawInput {
            attack: true,
            ..RawInput::NONE
        },
    )
}

/// A held physical press (Carl's push kick, Donut's pounce).
const fn physical(from: u64, to: u64, slot: usize) -> SlotHold {
    hold(
        from,
        to,
        slot,
        RawInput {
            physical: true,
            ..RawInput::NONE
        },
    )
}

const fn solo_swap(from: u64) -> SlotHold {
    hold(
        from,
        from + 1,
        0,
        RawInput {
            swap: true,
            ..RawInput::NONE
        },
    )
}

pub static LAB_R01_CARL_BRAWL: Probe = Probe {
    name: "lab_r01_carl_brawl",
    summary: "R01 solo Carl walks into the pit and jabs and kicks",
    room: "R01",
    variant: "v1",
    mode: PlayerMode::SoloCarl,
    ticks: 600,
    script: &[
        walk(1, 150, 0, 1.0),
        primary(150, 152, 0),
        primary(166, 168, 0),
        primary(182, 184, 0),
        physical(200, 202, 0),
        walk(210, 300, 0, 1.0),
        primary(300, 302, 0),
        physical(320, 322, 0),
    ],
};

pub static LAB_R01_DUO_MISSILES: Probe = Probe {
    name: "lab_r01_duo_missiles",
    summary: "R01 two pads: Carl walks and jabs, Donut casts Magic Missiles then pounces",
    room: "R01",
    variant: "v1",
    mode: PlayerMode::CoopLocal,
    ticks: 600,
    script: &[
        walk(1, 120, 0, 1.0),
        walk(1, 100, 1, 1.0),
        primary(100, 102, 1),
        primary(130, 132, 1),
        primary(160, 162, 1),
        physical(190, 192, 1),
        primary(150, 152, 0),
        primary(170, 172, 0),
    ],
};

pub static LAB_R02_DONUT_SOLO: Probe = Probe {
    name: "lab_r02_donut_solo",
    summary: "R02 solo with Donut controlled from tick 1: missiles and a pounce, Carl stays the AI",
    room: "R02",
    variant: "v1",
    mode: PlayerMode::SoloDonut,
    ticks: 600,
    script: &[
        walk(1, 90, 0, 1.0),
        primary(90, 92, 0),
        primary(110, 112, 0),
        physical(140, 142, 0),
        solo_swap(200),
        walk(210, 260, 0, 1.0),
    ],
};

pub const PROBES: [&Probe; 3] = [
    &LAB_R01_CARL_BRAWL,
    &LAB_R01_DUO_MISSILES,
    &LAB_R02_DONUT_SOLO,
];
