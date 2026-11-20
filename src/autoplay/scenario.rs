//! The v1 scenario roster (AP-Q17): Carl-only corridor fights against Chase
//! goblins, no hazards. Two development and two hold-out layouts.
//!
//! The `ap_*` fixtures are owned by autoplay and live only here; the game's own
//! fixture table never sees them, so `Fixture::by_name` and `FIXTURES` stay as
//! they are. They are therefore not replayable with `floor01_scene`.

use crate::combat::EnemyAi;
use crate::combat_scene::{CP01_SINGLE_GOBLIN, CP02_TWO_GOBLINS, EnemySpawn, FIXTURES, Fixture};

use super::error::AutoplayError;

/// Most enemies (goblins plus llamas) an observation can hold (AP-Q07).
pub const MAX_OBSERVED_ENEMIES: usize = 16;

/// Which side of the generalisation split a scenario belongs to (AP-Q17).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Split {
    Development,
    Holdout,
}

/// Two goblins far apart: the near one in view, the far one a short walk away.
/// The camera leads Carl by 24 px, so the initial view is x 104..744 (the
/// nominal centred view would be 80..720); 704 is at least 16 px inside both.
pub static AP_TWO_GOBLINS_WIDE: Fixture = Fixture {
    name: "ap_two_goblins_wide",
    summary: "autoplay: two chasing goblins far apart, the far one near the view edge",
    seed: 0xA901_0001,
    carl_x: 400,
    enemies: &[
        EnemySpawn {
            x: 480,
            ai: EnemyAi::Chase,
        },
        EnemySpawn {
            x: 704,
            ai: EnemyAi::Chase,
        },
    ],
    script: &[],
    ticks: 600,
    ..CP02_TWO_GOBLINS
};

/// One goblin that starts off-screen: the first observation is empty.
/// 776 is 32 px beyond the real initial right view edge (744), 56 px beyond the
/// nominal 720.
pub static AP_SINGLE_GOBLIN_FAR: Fixture = Fixture {
    name: "ap_single_goblin_far",
    summary: "autoplay: one chasing goblin that starts off-screen to the right",
    seed: 0xA901_0002,
    carl_x: 400,
    enemies: &[EnemySpawn {
        x: 776,
        ai: EnemyAi::Chase,
    }],
    script: &[],
    ticks: 600,
    ..CP01_SINGLE_GOBLIN
};

/// One roster entry: the fixture and the `state_hash` of its fresh sim.
#[derive(Debug, Clone, Copy)]
pub struct RosterEntry {
    pub id: &'static str,
    pub split: Split,
    pub fixture: &'static Fixture,
    /// Pinned initial hash (AP-Q11); a reset fails loudly when it differs.
    pub initial_hash: u64,
}

/// The v1 roster, development first.
pub fn roster() -> [RosterEntry; 4] {
    [
        RosterEntry {
            id: "cp01_single_goblin",
            split: Split::Development,
            fixture: &CP01_SINGLE_GOBLIN,
            initial_hash: 0x707a_cdf0_0ae4_52b5,
        },
        RosterEntry {
            id: "ap_two_goblins_wide",
            split: Split::Development,
            fixture: &AP_TWO_GOBLINS_WIDE,
            initial_hash: 0x04d6_cedf_ac32_f789,
        },
        RosterEntry {
            id: "cp02_two_goblins",
            split: Split::Holdout,
            fixture: &CP02_TWO_GOBLINS,
            initial_hash: 0xa820_6ad8_287d_62e4,
        },
        RosterEntry {
            id: "ap_single_goblin_far",
            split: Split::Holdout,
            fixture: &AP_SINGLE_GOBLIN_FAR,
            initial_hash: 0x355d_2937_254a_087b,
        },
    ]
}

/// Resolve a scenario id; any other fixture is rejected as unsupported.
pub fn lookup(id: &str) -> Result<RosterEntry, AutoplayError> {
    let roster = roster();
    roster.iter().copied().find(|e| e.id == id).ok_or_else(|| {
        let supported: Vec<&str> = roster.iter().map(|e| e.id).collect();
        AutoplayError::UnsupportedScenario(format!("{id} (supported: {})", supported.join(", ")))
    })
}

/// The layout fingerprint: kit (always the Floor 01 corridor), start column,
/// enemy columns and AI, llamas, shore, charges and boss. Not name, script,
/// seed or ticks, so script-only variants are duplicates (AP-Q17).
pub fn layout_fingerprint(fixture: &Fixture) -> String {
    format!(
        "corridor|{}|{:?}|{:?}|{:?}|{:?}|{:?}",
        fixture.carl_x,
        fixture
            .enemies
            .iter()
            .map(|e| (e.x, e.ai))
            .collect::<Vec<_>>(),
        fixture.llamas,
        fixture.shore_dx,
        fixture.charges,
        fixture.boss,
    )
}

/// Scenario validation: the v1 roster has no hazards and every enemy must fit
/// the observation (never silently truncated).
pub fn validate_fixture(fixture: &Fixture) -> Result<(), AutoplayError> {
    let count = fixture.enemies.len() + fixture.llamas.len();
    if count > MAX_OBSERVED_ENEMIES {
        return Err(AutoplayError::UnsupportedScenario(format!(
            "{} has {count} enemies, more than the {MAX_OBSERVED_ENEMIES} an observation holds",
            fixture.name
        )));
    }
    if fixture.enemies.is_empty() {
        return Err(AutoplayError::UnsupportedScenario(format!(
            "{} has no required hostiles",
            fixture.name
        )));
    }
    if fixture.boss.is_some() || fixture.shore_dx.is_some() || !fixture.charges.is_empty() {
        return Err(AutoplayError::UnsupportedScenario(format!(
            "{} has hazards or a boss; the v1 roster has none",
            fixture.name
        )));
    }
    Ok(())
}

/// True when `name` collides with the game's own fixture table.
pub fn collides_with_game_fixtures(name: &str) -> bool {
    FIXTURES.iter().any(|f| f.name == name) || Fixture::by_name(name).is_some()
}
