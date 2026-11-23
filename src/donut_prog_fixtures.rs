//! DONUT-PROG-01 fixtures (the `donut_prog_*` namespace): the second hero's
//! ability rows, flags and unlock triggers as replay fixtures.
//!
//! Bevy-free party fixtures in the shape of [`crate::party_scene`]. Each one
//! declares its own story setup ([`story_setup`]: the program flags it enables,
//! never a debug flag), has a committed replay in
//! `docs/evidence/donut-prog-01/replays` and a golden hash pinned by a test in
//! this file. They are **not** in [`PartyFixture::all`]: the registry is
//! [`DONUT_PROG_FIXTURES`], listed by `floor01_scene --list-fixtures
//! donut_prog_` and replayed by the CI loop, so every earlier fixture, golden
//! and replay is untouched. Later DONUT-PROG PRs append their fixtures here.
//!
//! Tick numbering is the one of [`crate::combat_scene`]: a hold over
//! `from..to` covers the steps that produce `tick = from .. to - 1`.

use crate::boss_scene::{Pilot, boss_base, hp};
use crate::carl_prog::CarlProgFlags;
use crate::combat::EnemyAi;
use crate::combat_scene::{EnemySpawn, Fixture};
use crate::donut_fixtures::{GUILD_PLACEHOLDER_ZONE, primary};
use crate::donut_mana::{DonutSetup, StartState};
use crate::donut_prog::{DonutProgFlags, story_setup as program_setup};
use crate::input::RawInput;
use crate::party::{PROVISIONAL_TUNING, PartyTuning};
use crate::party_scene::{PartyFixture, SlotHold, base, run};
use crate::story_unlock::{SceneKind, StorySetup};

const fn chase(x: i32) -> EnemySpawn {
    EnemySpawn {
        x,
        ai: EnemyAi::Chase,
    }
}

/// A held Circle/B on `slot`: its press edge is on `from`.
const fn special(from: u64, to: u64, slot: usize) -> SlotHold {
    SlotHold {
        from,
        to,
        slot,
        input: RawInput {
            ability: true,
            ..RawInput::NONE
        },
    }
}

const DONUT_PROG: DonutProgFlags = DonutProgFlags {
    prog: true,
    ..DonutProgFlags::OFF
};

/// The story a `donut_prog_*` fixture declares for itself (a scripted fixture
/// never reads a debug flag, DP-Q10). Every other fixture name is not in this
/// registry: [`StorySetup::NONE`].
pub fn story_setup(fixture: &str) -> StorySetup {
    match fixture {
        // The story corridor run under `--donut-prog`: the guild zone (the
        // biscuit, `DonutSetup`) and the safe-room placeholder zone.
        "donut_prog_story_unlocks"
        | "donut_prog_grant_survives_retry"
        | "donut_prog_primary_mapping" => {
            program_setup(CarlProgFlags::OFF, DONUT_PROG, SceneKind::Corridor)
        }
        // The boss pin and the Juicer trigger, Donut's program only.
        "donut_prog_boss_pins" => program_setup(CarlProgFlags::OFF, DONUT_PROG, SceneKind::Boss),
        // Both programs: the union of the pins and both entries of the grants list.
        "donut_prog_boss_both_programs" => program_setup(
            CarlProgFlags {
                prog: true,
                ..CarlProgFlags::OFF
            },
            DONUT_PROG,
            SceneKind::Boss,
        ),
        // DP-D: the grants of both Donut rows at the safe-room zone, for the render loop.
        "donut_prog_grants_2p" => grants_2p_setup(),
        // DP-D: the short blink held from tick 0 (`--donut-puddle`'s pin), in the
        // corridor and in the microarena.
        "donut_prog_blink_fizzle" => program_setup(CarlProgFlags::OFF, PUDDLE, SceneKind::Corridor),
        "donut_prog_arena_blink_tour"
        | "donut_prog_arena_blink_edge"
        | "donut_prog_arena_blink_cornered" => {
            program_setup(CarlProgFlags::OFF, PUDDLE, SceneKind::CfArena)
        }
        // DP-F: the boost alone (`--donut-light-feet`) at the 128 px ledge; the
        // base run holds the biscuit only.
        "donut_prog_arena_ledge_boost" => program_setup(
            CarlProgFlags::OFF,
            DonutProgFlags {
                light_feet: true,
                ..DonutProgFlags::OFF
            },
            SceneKind::CfArena,
        ),
        _ => StorySetup::NONE,
    }
}

/// `--donut-puddle`: the blink (and the boost and biscuit it requires) held from
/// tick 0.
const PUDDLE: DonutProgFlags = DonutProgFlags {
    puddle: true,
    ..DonutProgFlags::OFF
};

/// The biscuit starts locked and the placeholder guild zone grants it.
const GUILD: DonutSetup = DonutSetup {
    start: StartState::LOCKED,
    guild_zone: Some(GUILD_PLACEHOLDER_ZONE),
};

const STORY_BASE: Fixture = base(
    "donut_prog_story_unlocks",
    "Two players: both walk the corridor run under the Donut program; the guild zone gives the biscuit, the safe-room placeholder zone grants the jump boost once, and Circle/B stays inert",
    0x0D90_0001,
    &[],
    480,
);

/// DP-Q01, DP-Q02, DP-Q06: the biscuit at the guild zone, then
/// `jump_reach_boost` exactly once at the safe-room placeholder zone, with one
/// `CapabilityGranted { hero 1 }` on the first tick her pivot is inside. P2's
/// Circle/B presses (before and after the grant) emit nothing.
pub const DONUT_PROG_STORY_UNLOCKS: PartyFixture = PartyFixture {
    base: &STORY_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        run(5, 440, 0, 1.0),
        run(5, 440, 1, 1.0),
        special(20, 21, 1),
        special(300, 301, 1),
    ],
    pilot: Pilot::None,
    donut: GUILD,
};

const RETRY_BASE: Fixture = base(
    "donut_prog_grant_survives_retry",
    "Two players: Donut takes both unlocks, then two goblins defeat her (one hit does it here) and the party retries twice: the boost stays held and is never granted a second time",
    0x0D90_0002,
    &[chase(1300), chase(1340)],
    1200,
);

/// One hit defeats Donut here (a test tuning), so the retry comes quickly.
const ONE_HIT_DONUT: PartyTuning = PartyTuning {
    donut_damage_per_hit: 4,
    ..PROVISIONAL_TUNING
};

/// CP-Q07 for Donut's row: a grant survives a defeat retry (the demo's
/// checkpoint respawn and a fixture restart are unit-tested).
pub const DONUT_PROG_GRANT_SURVIVES_RETRY: PartyFixture = PartyFixture {
    base: &RETRY_BASE,
    two_player: true,
    tuning: ONE_HIT_DONUT,
    script: &[run(5, 440, 0, 1.0), run(5, 440, 1, 1.0)],
    pilot: Pilot::None,
    donut: GUILD,
};

const MAPPING_BASE: Fixture = base(
    "donut_prog_primary_mapping",
    "Two players: P2 Donut presses Square/X before the biscuit (swat), then at the guild zone (casts until the pool is dry), with Circle/B pressed throughout: it never does anything",
    0x0D90_0003,
    &[],
    420,
);

/// DP-Q01 and M1: the primary mapping is unchanged by the program: the swat
/// before the biscuit, a cast after it, a dry press at 0 mana; Circle/B is
/// inert until `special_short_blink` is held (no event, no state).
pub const DONUT_PROG_PRIMARY_MAPPING: PartyFixture = PartyFixture {
    base: &MAPPING_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        run(5, 140, 1, 1.0),
        primary(8, 9, 1),
        special(10, 11, 1),
        special(40, 41, 1),
        primary(70, 71, 1),
        primary(90, 91, 1),
        primary(110, 111, 1),
        primary(130, 131, 1),
        primary(150, 151, 1),
        primary(170, 171, 1),
        primary(190, 191, 1),
        primary(210, 211, 1),
        primary(230, 231, 1),
        primary(250, 251, 1),
        special(260, 261, 1),
        special(340, 341, 1),
    ],
    pilot: Pilot::None,
    donut: GUILD,
};

const PINS_BASE: Fixture = boss_base(
    "donut_prog_boss_pins",
    "The boss has 4 HP left and Donut holds the jump boost from the boss pin (the biscuit read through): the rack crush defeats it and the Juicer trigger grants only the blink on that tick; Carl's reinforced fist is not granted",
    0x0D90_0004,
    180,
    hp(4),
    420,
);

/// DP-Q03, DP-Q09, rule 3: under `--donut-prog` alone the Juicer trigger
/// grants only `special_short_blink` (hero 1); the pin emits nothing.
pub const DONUT_PROG_BOSS_PINS: PartyFixture = PartyFixture {
    base: &PINS_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::CarlRack,
    donut: DonutSetup::unlocked(8),
};

const BOTH_BASE: Fixture = boss_base(
    "donut_prog_boss_both_programs",
    "The boss has 4 HP left under both programs (Carl's dynamite and Donut's boost pinned): the Juicer defeat grants the reinforced fist and then the blink, in list order on one tick",
    0x0D90_0005,
    180,
    hp(4),
    420,
);

/// DP-Q03, DP-Q09: with both programs the two grants land on the defeat tick
/// in grants-list order, `fist_reinforced` then `special_short_blink`.
pub const DONUT_PROG_BOSS_BOTH_PROGRAMS: PartyFixture = PartyFixture {
    base: &BOTH_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::CarlRack,
    donut: DonutSetup::unlocked(8),
};

const GRANTS_BASE: Fixture = base(
    "donut_prog_grants_2p",
    "DP-D evidence: both players walk the corridor under the Donut program; at the safe-room placeholder zone Donut is granted Light on Your Feet and the blink on one tick (an evidence-only table puts the blink on that zone): the two notices queue, LIGHT FEET then PUDDLE JUMPER, and her socket turns from the padlock to the equipped chip",
    0x0D90_0006,
    &[],
    520,
);

/// DP-Q12 (the grant presentation) in the render loop. The shipped table grants
/// the blink on the Juicer's defeat, which the render loop cannot run (the boss
/// pilot is a headless script), so this fixture swaps in a table variant whose
/// safe-room zone also grants the blink: the same event path, the same order
/// (the boost first, then the blink) and the same HUD code. The Juicer grant
/// itself is `donut_prog_boss_both_programs` and its HUD test.
pub const DONUT_PROG_GRANTS_2P: PartyFixture = PartyFixture {
    base: &GRANTS_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[run(5, 520, 0, 1.0), run(5, 520, 1, 1.0)],
    pilot: Pilot::None,
    donut: GUILD,
};

/// The evidence table of [`DONUT_PROG_GRANTS_2P`]: the safe-room zone's grants
/// list also names `special_short_blink` (its `requires` is met by the boost
/// granted just before it). Built once and leaked, like the table of the DP-C rig.
fn grants_2p_setup() -> StorySetup {
    static TABLE: std::sync::OnceLock<&'static crate::story_unlock::StoryTable> =
        std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let text = crate::story_unlock::TABLE_JSON.replacen(
            "\"grants\": [\"jump_reach_boost\"]",
            "\"grants\": [\"jump_reach_boost\", \"special_short_blink\"]",
            1,
        );
        assert_ne!(text, crate::story_unlock::TABLE_JSON);
        crate::story_unlock::StoryTable::from_json(&text)
            .expect("the evidence table parses")
            .leak()
    });
    StorySetup::for_scene(table, &["donut_prog"], &[], SceneKind::Corridor)
}

const FIZZLE_BASE: Fixture = Fixture {
    carl_x: 300,
    ..base(
        "donut_prog_blink_fizzle",
        "DP-D evidence: Carl runs right and the scroll camera follows; P2 Donut, left behind at the view's edge, presses the blink to the left: the destination is valid at the press and not at the arrival, so the blink fizzles (no mana, no cooldown) and the single shrinking ring shows",
        0x0D90_0007,
        &[],
        260,
    )
};

/// DP-Q28 (the arrival check) and DP-Q34 (the fizzle ring): found by a search
/// over the press tick on the rig of the DP-C fixtures, then pinned.
pub const DONUT_PROG_BLINK_FIZZLE: PartyFixture = PartyFixture {
    base: &FIZZLE_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        run(1, 400, 0, 1.0),
        special(136, 137, 1),
        run(136, 144, 1, -1.0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::unlocked(8),
};

const TOUR_BASE: Fixture = Fixture {
    carl_x: 300,
    enemies: &[],
    engineers: &[],
    shore_dx: None,
    llamas: &[],
    ..crate::cf02::arena::fixture(
        "donut_prog_arena_blink_tour",
        "DP-D evidence: the empty microarena, P2 Donut with the blink: a flat 112 px blink, the socket cooling over 9 s (the underline shrinks), the ready flash, then a blink toward the parapet that stops short in front of it",
        0x0D90_0008,
        720,
    )
};

/// DP-Q27 (flat and wall), DP-Q33 (the cooldown underline and the ready flash),
/// DP-Q34 (the ripples and the streak). The arena kit comes from the name
/// ([`crate::cf02::arena::fixture_uses_arena`]).
pub const DONUT_PROG_ARENA_BLINK_TOUR: PartyFixture = PartyFixture {
    base: &TOUR_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        special(40, 41, 1),
        run(40, 48, 1, 1.0),
        run(60, 118, 1, 1.0),
        special(600, 601, 1),
        run(600, 608, 1, 1.0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::unlocked(8),
};

const EDGE_BASE: Fixture = Fixture {
    carl_x: 100,
    enemies: &[],
    engineers: &[],
    shore_dx: None,
    llamas: &[],
    ..crate::cf02::arena::fixture(
        "donut_prog_arena_blink_edge",
        "DP-D evidence: Carl waits at the left; P2 Donut walks to the farthest she can go, casts her pool down to one charge and presses the blink (no mana: refused, the socket flashes), later, with the charges back, presses it toward the view's edge: no destination lies inside the view (the 24 px inset), so that press is refused too and the socket flashes again",
        0x0D90_0009,
        520,
    )
};

/// DP-Q27 rule 5 (the view inset) and DP-Q26: the no-mana and the no-destination
/// refusals, and DP-Q33 (the flash both raise).
pub const DONUT_PROG_ARENA_BLINK_EDGE: PartyFixture = PartyFixture {
    base: &EDGE_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        run(1, 110, 1, 1.0),
        primary(130, 131, 1),
        primary(150, 151, 1),
        primary(170, 171, 1),
        primary(190, 191, 1),
        primary(210, 211, 1),
        primary(230, 231, 1),
        primary(250, 251, 1),
        special(270, 271, 1),
        run(270, 278, 1, 1.0),
        special(420, 421, 1),
        run(420, 428, 1, 1.0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::unlocked(8),
};

/// The shared script of the two ledge captures: P2 Donut runs right into the
/// 128 px ledge (`cf02::arena::LEDGE_128`), jumps once at the face on tick 40
/// and keeps pushing right (DM-A's climb script, in the room).
const LEDGE_SCRIPT: [SlotHold; 2] = [run(10, 140, 1, 1.0), jump(40, 41, 1)];

/// A jump press on `slot`: its edge is on `from`.
const fn jump(from: u64, to: u64, slot: usize) -> SlotHold {
    SlotHold {
        from,
        to,
        slot,
        input: RawInput {
            jump: true,
            ..RawInput::NONE
        },
    }
}

const LEDGE_BASE_BASE: Fixture = Fixture {
    carl_x: 100,
    enemies: &[],
    engineers: &[],
    shore_dx: None,
    llamas: &[],
    ..crate::cf02::arena::fixture(
        "donut_prog_arena_ledge_base",
        "DP-F evidence: the microarena with a 128 px ledge; P2 Donut (the biscuit, no jump boost) runs into it and jumps: she falls short of the top",
        0x0D90_000B,
        150,
    )
};

/// DP-Q18: Donut at the base launch cannot land a 128 px ledge (the left half of
/// the side-by-side capture).
pub const DONUT_PROG_ARENA_LEDGE_BASE: PartyFixture = PartyFixture {
    base: &LEDGE_BASE_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &LEDGE_SCRIPT,
    pilot: Pilot::None,
    donut: DonutSetup::unlocked(8),
};

const LEDGE_BOOST_BASE: Fixture = Fixture {
    carl_x: 100,
    enemies: &[],
    engineers: &[],
    shore_dx: None,
    llamas: &[],
    ..crate::cf02::arena::fixture(
        "donut_prog_arena_ledge_boost",
        "DP-F evidence: the same 128 px ledge and script with `jump_reach_boost` held (`--donut-light-feet`): she lands on top",
        0x0D90_000B,
        150,
    )
};

/// DP-Q14, DP-Q18: with Light on Your Feet she lands the 128 px ledge (the right
/// half of the side-by-side capture; same seed and script as the base run).
pub const DONUT_PROG_ARENA_LEDGE_BOOST: PartyFixture = PartyFixture {
    base: &LEDGE_BOOST_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &LEDGE_SCRIPT,
    pilot: Pilot::None,
    donut: DonutSetup::unlocked(8),
};

const CORNERED_BASE: Fixture = Fixture {
    carl_x: 120,
    enemies: &[chase(60)],
    engineers: &[],
    shore_dx: None,
    llamas: &[],
    ..crate::cf02::arena::fixture(
        "donut_prog_arena_blink_cornered",
        "DP-F evidence: Carl stands still at 120; one goblin (40 HP) starts at 60, 28 px from the AI Donut (88) with the end wall behind it; she holds the blink and exactly the reserve (2 charges), her follow-layer jump dodge is switched off (radius 0) and the AI target bias is 0, so its swing is aimed at her: she blinks out of it toward Carl (press 20, arrival 26, hull-capped at Carl + 63)",
        0x0D90_000A,
        120,
    )
};

/// DP-Q39 (b): the AI Donut escapes a committed swing with the blink (the GIF of
/// DP-E's deferral). The dodge radius 0 is the same staging as
/// `cf02_free_blink_escape`: with the dodge on, the jump wins and she never
/// blinks (`cf02_free_blink_jump_dodge_wins`).
pub const DONUT_PROG_ARENA_BLINK_CORNERED: PartyFixture = PartyFixture {
    base: &CORNERED_BASE,
    two_player: false,
    tuning: PartyTuning {
        dodge_radius_px: 0,
        target_ai_bias_px: 0,
        ..crate::cf02_free_fixtures::FREE
    },
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::unlocked(2),
};

/// Every `donut_prog_*` replay fixture, in committed order.
pub const DONUT_PROG_FIXTURES: [&PartyFixture; 12] = [
    &DONUT_PROG_STORY_UNLOCKS,
    &DONUT_PROG_GRANT_SURVIVES_RETRY,
    &DONUT_PROG_PRIMARY_MAPPING,
    &DONUT_PROG_BOSS_PINS,
    &DONUT_PROG_BOSS_BOTH_PROGRAMS,
    // DP-D (the render-loop evidence of the blink).
    &DONUT_PROG_GRANTS_2P,
    &DONUT_PROG_BLINK_FIZZLE,
    &DONUT_PROG_ARENA_BLINK_TOUR,
    &DONUT_PROG_ARENA_BLINK_EDGE,
    // DP-F (the evidence captures: the 128 px ledge pair and the AI's escape).
    &DONUT_PROG_ARENA_LEDGE_BASE,
    &DONUT_PROG_ARENA_LEDGE_BOOST,
    &DONUT_PROG_ARENA_BLINK_CORNERED,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss_scene::{BOSS_FIXTURES, load_arena};
    use crate::cf02_fixtures::{find_party_fixture, list_fixtures};
    use crate::combat::CombatEvent;
    use crate::combat_scene::{CombatState, FixtureSim};
    use crate::corridor::SceneSim;
    use crate::demo::{DEMO_RULES, autopilot, fixture_for_beat, load_hall};
    use crate::donut_mana::ManaEvent;
    use crate::env_kit::EnvKit;
    use crate::party_scene::PartyFixtureSim;
    use crate::story_unlock::{
        CapabilityId, FIST_REINFORCED, PRIMARY_RANGED, ReadThrough, SPECIAL_THROWN_CHARGE,
        StoryRuntime, StoryTable, UnlockLedger,
    };
    use std::path::{Path, PathBuf};

    const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn kit_of(fixture: &PartyFixture) -> EnvKit {
        if fixture.base.room == crate::combat_scene::FixtureRoom::Arena {
            crate::cf02::arena::load_arena_for(&root(), fixture.name()).unwrap()
        } else if fixture.base.boss.is_some() {
            load_arena(&root()).unwrap()
        } else {
            EnvKit::load(&root(), LAYOUT).unwrap()
        }
    }

    fn replay(fixture: &'static PartyFixture) -> PartyFixtureSim {
        let mut sim = PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        sim.run_scripted(fixture.base.ticks);
        sim
    }

    fn named(name: &str) -> PartyFixtureSim {
        replay(find_party_fixture(name).expect("a registered fixture"))
    }

    /// Every event of the run, as the headless replay prints them.
    fn lines(sim: &PartyFixtureSim) -> Vec<String> {
        assert_eq!(sim.combat.dropped_events, 0, "the log never hit its cap");
        let mut out: Vec<String> = sim.combat.events.iter().map(ToString::to_string).collect();
        out.push(sim.combat.state_line(&sim.scene));
        out.push(sim.state_line());
        out
    }

    fn grants(sim: &PartyFixtureSim) -> Vec<(u64, u8, &'static str)> {
        sim.combat
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

    fn boost() -> CapabilityId {
        crate::donut_prog::JUMP_REACH_BOOST
    }

    fn blink() -> CapabilityId {
        crate::donut_prog::SPECIAL_SHORT_BLINK
    }

    /// The committed final hashes of the `donut_prog_*` fixtures.
    const GOLDENS: [(&str, u64); 12] = [
        ("donut_prog_story_unlocks", 0xf7eb_5539_2e6c_5a98),
        ("donut_prog_grant_survives_retry", 0x4fe3_164a_dcb2_29a3),
        ("donut_prog_primary_mapping", 0xcd02_6290_cfd5_d561),
        ("donut_prog_boss_pins", 0x0aa6_2409_c3c4_7e08),
        ("donut_prog_boss_both_programs", 0xcc0c_9ac9_51ee_295c),
        // DP-D (pinned from their first run; the events are asserted below).
        ("donut_prog_grants_2p", 0xab69_21d3_eba4_608d),
        ("donut_prog_blink_fizzle", 0xd0cd_3847_7c85_8fb7),
        ("donut_prog_arena_blink_tour", 0x9099_8a5d_28e8_da53),
        ("donut_prog_arena_blink_edge", 0x6def_9c57_460a_b471),
        // DP-F (pinned from their first run; the events are asserted below).
        ("donut_prog_arena_ledge_base", 0xc8f9_6c1a_c605_b624),
        ("donut_prog_arena_ledge_boost", 0x57f1_0fad_98ac_fa80),
        ("donut_prog_arena_blink_cornered", 0xbad6_9d85_668f_ab8f),
    ];

    #[test]
    fn the_registry_is_listed_and_every_fixture_matches_its_golden_and_replays_identically() {
        assert_eq!(GOLDENS.len(), DONUT_PROG_FIXTURES.len());
        let listed = list_fixtures("donut_prog_");
        for (fixture, (name, expected)) in DONUT_PROG_FIXTURES.iter().zip(GOLDENS) {
            assert_eq!(fixture.name(), name);
            assert!(listed.contains(&name), "{name} is listed");
            assert!(find_party_fixture(name).is_some(), "{name} is runnable");
            let (a, b) = (replay(fixture), replay(fixture));
            assert_eq!(a.state_hash(), b.state_hash(), "{name} replays");
            assert_eq!(lines(&a), lines(&b), "{name}: the same events twice");
            assert_eq!(a.state_hash(), expected, "{name}: {}", a.state_line());
        }
        assert_eq!(
            listed.len(),
            DONUT_PROG_FIXTURES.len(),
            "nothing else in the namespace"
        );
        let mut hashes: Vec<u64> = GOLDENS.iter().map(|g| g.1).collect();
        hashes.sort_unstable();
        hashes.dedup();
        assert_eq!(hashes.len(), GOLDENS.len(), "every fixture is distinct");
    }

    /// DP-D: the ticks of the evidence fixtures' blink events, from the real
    /// simulation (the render-loop replays show these pictures).
    #[test]
    fn the_dp_d_fixtures_raise_the_events_their_pictures_show() {
        let events = |name: &str| -> Vec<String> {
            let f = find_party_fixture(name).unwrap();
            lines(&replay(f))
                .into_iter()
                .filter(|l| {
                    l.starts_with("COMBAT_EVENT")
                        && (l.contains("blink") || l.contains("special_") || l.contains("capab"))
                })
                .map(|l| l.trim_start_matches("COMBAT_EVENT ").to_string())
                .collect()
        };
        let grants = events("donut_prog_grants_2p");
        assert_eq!(grants.len(), 2, "{grants:?}");
        assert!(
            grants[0].ends_with("hero=1 capability=jump_reach_boost"),
            "{grants:?}"
        );
        assert!(
            grants[1].ends_with("hero=1 capability=special_short_blink"),
            "{grants:?}"
        );
        assert_eq!(
            grants[0].split(' ').next(),
            grants[1].split(' ').next(),
            "one tick, list order"
        );
        assert_eq!(
            events("donut_prog_blink_fizzle"),
            [
                "tick=136 kind=special_started hero=1 capability=special_short_blink",
                "tick=142 kind=blink_fizzled hero=1 reason=no_destination",
            ]
        );
        assert_eq!(
            events("donut_prog_arena_blink_tour"),
            [
                "tick=40 kind=special_started hero=1 capability=special_short_blink",
                "tick=46 kind=blink_resolved hero=1 from_x=340 to_x=452",
                "tick=586 kind=special_ready hero=1",
                "tick=600 kind=special_started hero=1 capability=special_short_blink",
                "tick=606 kind=blink_resolved hero=1 from_x=656 to_x=724",
            ],
            "flat, ready 540 ticks after the arrival, then the parapet stops her at 724"
        );
        assert_eq!(
            events("donut_prog_arena_blink_edge"),
            [
                "tick=270 kind=special_rejected hero=1 reason=no_mana",
                "tick=420 kind=special_rejected hero=1 reason=no_destination",
            ]
        );
    }

    /// DP-F: what the three capture fixtures show. The ledge pair differs only in
    /// the boost: the base run peaks under the 128 px top and falls back to the
    /// floor; the boosted run stands on the ledge. The AI escape is the exact
    /// press, arrival and swing ticks of the picture.
    #[test]
    fn the_dp_f_fixtures_show_what_their_captures_claim() {
        // The ledge top in sim y (the floor stand is 118, the ledge adds 128).
        let (floor_y, top_y) = (118.0_f32, 118.0_f32 + 128.0);
        let climb = |name: &str| {
            let f = find_party_fixture(name).unwrap();
            let mut sim = PartyFixtureSim::new(&kit_of(f), 360, f).unwrap();
            let (mut peak, mut on_top) = (0.0_f32, 0_u64);
            for _ in 0..f.base.ticks {
                sim.step_scripted();
                let body = &sim.scene.world.donut_body;
                peak = peak.max(body.y);
                if body.grounded && (body.y - top_y).abs() < 0.01 {
                    on_top += 1;
                }
            }
            (peak, on_top, sim.scene.world.donut_body.y)
        };
        let (peak, on_top, end_y) = climb("donut_prog_arena_ledge_base");
        assert!(peak < top_y, "the base jump peaks at {peak}, under {top_y}");
        assert_eq!((on_top, end_y), (0, floor_y), "she never stands on it");
        let (peak, on_top, _) = climb("donut_prog_arena_ledge_boost");
        assert!(peak > top_y, "the boosted jump clears the top: {peak}");
        assert!(on_top > 20, "and she stands on it for {on_top} ticks");
        // The AI escape: the swing is aimed at her, she presses on tick 20, arrives
        // on 26 (88 to 180, capped at Carl + 63) and the swing on 31 finds no one.
        let events = lines(&named("donut_prog_arena_blink_cornered"));
        let picked: Vec<&str> = events
            .iter()
            .map(String::as_str)
            .filter(|l| l.starts_with("COMBAT_EVENT"))
            .filter(|l| {
                l.contains("goblin_tell")
                    || l.contains("goblin_swing")
                    || l.contains("special_")
                    || l.contains("blink_")
                    || l.contains("hero_hurt")
            })
            .collect();
        assert_eq!(
            picked,
            [
                "COMBAT_EVENT tick=5 kind=goblin_tell goblin=0",
                "COMBAT_EVENT tick=20 kind=special_started hero=1 capability=special_short_blink",
                "COMBAT_EVENT tick=26 kind=blink_resolved hero=1 from_x=88 to_x=180",
                "COMBAT_EVENT tick=31 kind=goblin_swing goblin=0",
            ]
        );
    }

    #[test]
    fn the_committed_replays_are_the_replay() {
        for fixture in DONUT_PROG_FIXTURES {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("docs/evidence/donut-prog-01/replays")
                .join(format!("headless_{}.txt", fixture.name()));
            let committed = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let committed: Vec<&str> = committed.lines().collect();
            assert_eq!(committed, lines(&replay(fixture)), "{}", fixture.name());
        }
    }

    #[test]
    fn no_other_registry_holds_a_donut_prog_name_and_every_other_fixture_is_unpinned() {
        for name in list_fixtures("") {
            if !name.starts_with("donut_prog_") {
                assert!(
                    story_setup(name).is_inert(),
                    "{name} is a legacy fixture: no pin, no trigger"
                );
            }
        }
        for fixture in BOSS_FIXTURES {
            assert!(story_setup(fixture.name()).is_inert());
        }
        // Carl's registry keeps its own setups (the shared lookup falls through to ours).
        for name in list_fixtures("carl_prog_") {
            assert!(story_setup(name).is_inert());
        }
        assert!(!crate::carl_prog_fixtures::story_setup("carl_prog_juicer_grants_fist").is_inert());
    }

    /// DP-Q01, DP-Q02, DP-Q06 (the story corridor run, simulation and events
    /// only): the biscuit at the guild zone on the existing tick, then the
    /// jump boost exactly once at the safe-room placeholder zone.
    #[test]
    fn donut_prog_story_unlocks() {
        let sim = named("donut_prog_story_unlocks");
        let unlock: Vec<u64> = sim
            .combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Mana(ManaEvent::UnlockGranted { tick, .. }) => Some(tick),
                _ => None,
            })
            .collect();
        assert_eq!(unlock, [50], "the guild zone, on the agile body's tick");
        let granted = grants(&sim);
        assert_eq!(
            granted,
            [(246, 1, "jump_reach_boost")],
            "once, to hero 1, after the biscuit"
        );
        assert!(granted[0].0 > unlock[0]);
        assert!(sim.combat.story_holds(boost()));
        assert!(
            sim.combat.story_holds(PRIMARY_RANGED),
            "the biscuit reads through"
        );
        assert!(
            !sim.combat.story_holds(blink()),
            "no blink without the Juicer"
        );
        assert!(
            !sim.combat.story_holds(SPECIAL_THROWN_CHARGE),
            "the Donut program never grants a Carl row"
        );
        let ledger = sim.combat.story_ledger().unwrap();
        assert_eq!(ledger.receipts().len(), 1);
        assert_eq!(ledger.receipts()[0].source, "f01_safe_room_reward_found");
        // Circle/B before and after the grant: nothing was logged for it.
        let kinds: Vec<String> = sim.combat.events.iter().map(ToString::to_string).collect();
        assert_eq!(kinds.len(), 2, "only the unlock and the grant: {kinds:?}");
    }

    /// CP-Q07 for Donut's row: a defeat retry keeps the grant (twice), a
    /// fixture restart restores the empty declared start and re-arms the zone.
    #[test]
    fn donut_prog_grant_survives_retry() {
        let mut sim = named("donut_prog_grant_survives_retry");
        assert_eq!(
            grants(&sim),
            [(246, 1, "jump_reach_boost")],
            "one grant, never two"
        );
        let restarts: Vec<_> = sim
            .combat
            .events
            .iter()
            .filter(|e| matches!(e, CombatEvent::Restart { .. }))
            .collect();
        assert_eq!(restarts.len(), 2, "defeated and retried twice");
        assert!(sim.combat.deaths >= 2);
        assert!(sim.combat.story_holds(boost()), "the retries keep it");
        let receipts = sim.combat.story_ledger().unwrap().receipts();
        assert_eq!((receipts.len(), receipts[0].tick), (1, 246));
        // The biscuit survives the retry too (mana v0), so the order still holds.
        assert!(sim.combat.story_holds(PRIMARY_RANGED));
        // A fixture restart restores the declared start: nothing held, biscuit locked.
        sim.combat.restart(&mut sim.scene);
        assert!(!sim.combat.story_holds(boost()));
        assert!(sim.combat.story_ledger().is_none_or(|l| l.is_empty()));
        assert!(!sim.combat.story_holds(PRIMARY_RANGED));
        // The zone is armed again: walking in grants once more.
        sim.party.restart();
        sim.run_scripted(330);
        assert!(sim.combat.story_holds(boost()), "re-armed by the restart");
    }

    /// DP-Q11 on the checkpoint respawn of the demo hall: a grant earned in
    /// play survives it, a start hold is a declared start and never a grant, a
    /// fixture restart drops the grant and keeps the hold.
    #[test]
    fn a_checkpoint_respawn_keeps_a_donut_grant_and_a_fixture_restart_removes_it() {
        let kit = load_hall(&root()).unwrap();
        let fixture = fixture_for_beat(1);
        let scene = SceneSim::new_at(&kit, 360, fixture.carl_x).unwrap();
        let combat = CombatState::new(fixture, &kit, &scene)
            .unwrap()
            .with_demo(&DEMO_RULES)
            .with_donut(DonutSetup::unlocked(8))
            .with_story(StorySetup::NONE.with_hold("special_short_blink"));
        let mut run = FixtureSim { scene, combat };
        assert!(run.combat.story_holds(blink()), "a pin is held from tick 0");
        assert!(
            run.combat.story_holds(boost()),
            "a hold grants the requires closure"
        );
        run.combat.respawn(&mut run.scene);
        assert!(run.combat.story_holds(blink()), "the respawn keeps it");
        assert!(
            run.combat
                .events
                .iter()
                .all(|e| !matches!(e, CombatEvent::CapabilityGranted { .. })),
            "a pin is a declared start, never a grant"
        );
        // A grant earned in play: Donut's pivot inside the safe-room zone.
        let mut earned = CombatState::new(fixture, &kit, &run.scene)
            .unwrap()
            .with_demo(&DEMO_RULES)
            .with_donut(DonutSetup::unlocked(8))
            .with_story(StorySetup::NONE.with_trigger("f01_safe_room_reward_found"));
        let mut scene = run.scene.clone();
        scene.world.donut_body.x = 1210.0;
        scene.world.sync_donut_for_tests();
        earned.step(&mut scene, crate::input::RawInput::default());
        assert!(earned.story_holds(boost()), "the zone granted it");
        earned.respawn(&mut scene);
        assert!(earned.story_holds(boost()), "the respawn keeps a grant");
        earned.restart(&mut scene);
        assert!(!earned.story_holds(boost()), "R restores the empty start");
        run.combat.restart(&mut run.scene);
        assert!(run.combat.story_holds(blink()), "R restores the pin");
    }

    /// DP-Q01 and M1: the primary mapping is the legacy one (swat, then cast,
    /// then dry) and Circle/B does nothing before the blink is held: no event,
    /// and the run hashes exactly like the same inputs with no story at all.
    #[test]
    fn donut_prog_primary_mapping() {
        let sim = named("donut_prog_primary_mapping");
        let moves: Vec<(u64, &str)> = sim
            .combat
            .events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::MoveStarted { tick, move_id, .. } => Some((*tick, move_id.name())),
                _ => None,
            })
            .collect();
        assert_eq!(moves[0], (8, "swat"), "before the biscuit Square/X swats");
        assert!(
            moves[1..]
                .iter()
                .all(|(tick, name)| *tick >= 59 && *name == "magic_missile"),
            "after the grant (tick 59) it casts: {moves:?}"
        );
        assert_eq!(moves.len(), 9, "one swat, eight casts");
        let dry: Vec<u64> = sim
            .combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Mana(ManaEvent::CastRejected { tick, .. }) => Some(tick),
                _ => None,
            })
            .collect();
        assert_eq!(dry, [230, 250], "at 0 mana the press is dry");
        // Circle/B (ticks 10, 40, 260, 340): the event log holds nothing of it
        // and no capability is held.
        assert!(grants(&sim).is_empty());
        assert!(!sim.combat.story_holds(blink()));
        // The same script with no story: identical hash and events.
        let fixture = &DONUT_PROG_PRIMARY_MAPPING;
        let kit = kit_of(fixture);
        let scene = SceneSim::new_at(&kit, 360, fixture.base.carl_x).unwrap();
        let combat = CombatState::new(fixture.base, &kit, &scene)
            .unwrap()
            .with_donut(fixture.donut);
        let mut plain = PartyFixtureSim::from_parts(scene, combat, fixture);
        plain.run_scripted(fixture.base.ticks);
        assert_eq!(plain.state_hash(), sim.state_hash());
        assert_eq!(plain.combat.events, sim.combat.events);
    }

    /// DP-Q03, DP-Q09, rule 3: under the Donut program alone the Juicer trigger
    /// grants only the blink, on the defeat tick, and the pin emits nothing.
    #[test]
    fn donut_prog_boss_pins_grants_only_the_blink() {
        let sim = named("donut_prog_boss_pins");
        let defeat = sim
            .combat
            .events
            .iter()
            .find_map(|e| match *e {
                CombatEvent::EnemyDefeated { tick, .. } => Some(tick),
                _ => None,
            })
            .expect("the boss is defeated");
        assert_eq!(grants(&sim), [(defeat, 1, "special_short_blink")]);
        assert!(sim.combat.story_holds(boost()), "the pin");
        assert!(sim.combat.story_holds(blink()));
        assert!(
            !sim.combat.story_holds(FIST_REINFORCED),
            "Carl's entry is skipped"
        );
        assert!(!sim.combat.story_holds(SPECIAL_THROWN_CHARGE));
        let ledger = sim.combat.story_ledger().unwrap();
        assert_eq!(
            ledger.receipts().len(),
            1,
            "one receipt, none for the pin or the skip"
        );
    }

    /// DP-Q03: with both programs the two grants land on the defeat tick in
    /// grants-list order.
    #[test]
    fn donut_prog_boss_both_programs_grants_in_list_order() {
        let sim = named("donut_prog_boss_both_programs");
        let defeat = sim
            .combat
            .events
            .iter()
            .find_map(|e| match *e {
                CombatEvent::EnemyDefeated { tick, .. } => Some(tick),
                _ => None,
            })
            .expect("the boss is defeated");
        assert_eq!(
            grants(&sim),
            [
                (defeat, 0, "fist_reinforced"),
                (defeat, 1, "special_short_blink")
            ]
        );
        assert!(sim.combat.story_holds(SPECIAL_THROWN_CHARGE), "Carl's pin");
        assert!(sim.combat.story_holds(boost()), "Donut's pin");
    }

    /// The held set of the boss arena at tick 0 under each flag combination:
    /// CP-Q08's dynamite, Donut's boost (the biscuit read through) and their
    /// union.
    #[test]
    fn donut_prog_boss_pins_the_held_set_per_flag_combination() {
        let carl = CarlProgFlags {
            prog: true,
            ..CarlProgFlags::OFF
        };
        let held = |c: CarlProgFlags, d: DonutProgFlags| {
            let mut ids: Vec<&str> = program_setup(c, d, SceneKind::Boss)
                .holds()
                .iter()
                .map(|id| id.as_str())
                .collect();
            ids.sort_unstable();
            ids
        };
        assert_eq!(held(carl, DonutProgFlags::OFF), ["special_thrown_charge"]);
        assert_eq!(held(CarlProgFlags::OFF, DONUT_PROG), ["jump_reach_boost"]);
        assert_eq!(
            held(carl, DONUT_PROG),
            ["jump_reach_boost", "special_thrown_charge"]
        );
        // The scene accepts every one of them with the biscuit unlocked, and
        // the boss pin with the biscuit locked names the pin.
        for (c, d) in [
            (carl, DonutProgFlags::OFF),
            (CarlProgFlags::OFF, DONUT_PROG),
            (carl, DONUT_PROG),
        ] {
            let setup = program_setup(c, d, SceneKind::Boss);
            assert!(
                setup
                    .check_read_through(&ReadThrough {
                        primary_ranged: true
                    })
                    .is_ok()
            );
        }
        let locked = program_setup(CarlProgFlags::OFF, DONUT_PROG, SceneKind::Boss)
            .check_read_through(&ReadThrough::default())
            .expect_err("the boss pin needs the biscuit");
        assert!(
            locked
                .to_string()
                .contains("pins[f01_donut_boss_held].holds"),
            "{locked}"
        );
    }

    /// A pin that breaks the story order fails at setup, naming the pin, and a
    /// scene built that way is refused when the combat state is assembled.
    #[test]
    #[should_panic(expected = "pins[f01_donut_boss_held].holds")]
    fn a_donut_boss_pin_without_the_biscuit_is_refused_at_setup() {
        let fixture = &DONUT_PROG_BOSS_PINS;
        let kit = kit_of(fixture);
        let scene = SceneSim::new_at(&kit, 360, fixture.base.carl_x).unwrap();
        let _ = CombatState::new(fixture.base, &kit, &scene)
            .unwrap()
            .with_donut(DonutSetup::NONE)
            .with_story(story_setup(fixture.name()));
    }

    /// Rule 3 on the real Juicer row and the real safe-room row, at the runtime
    /// level (the fixtures above prove it in the scene): one trigger fires once,
    /// each entry is guarded by its program and by `requires`, and a skipped
    /// entry leaves no receipt, no hash and one debug line.
    #[test]
    fn the_per_entry_guards_on_the_real_rows() {
        let table = StoryTable::active();
        let view = ReadThrough {
            primary_ranged: true,
        };
        let juicer = |carl: bool, donut: bool, hold_boost: bool| {
            let mut setup = program_setup(
                CarlProgFlags {
                    prog: carl,
                    ..CarlProgFlags::OFF
                },
                DonutProgFlags {
                    prog: donut,
                    ..DonutProgFlags::OFF
                },
                SceneKind::Boss,
            );
            if hold_boost && !donut {
                setup = setup.with_hold("jump_reach_boost");
            }
            let mut runtime = StoryRuntime::new(&setup).expect("armed");
            runtime.observe("brute_boss_f01_defeated");
            let receipts = runtime.step(9, [(0.0, 0.0); 2], false, &view);
            let granted: Vec<(u8, &str)> = receipts
                .iter()
                .map(|r| (r.hero, r.capability.as_str()))
                .collect();
            let mut mixed = Vec::new();
            runtime.hash_into(&mut |v| mixed.push(v));
            (granted, runtime.debug_log().len(), mixed)
        };
        // Carl's program alone: only the fist, the skipped blink leaves one debug line.
        let (granted, skipped, _) = juicer(true, false, false);
        assert_eq!(granted, [(0, "fist_reinforced")]);
        assert_eq!(skipped, 1);
        // Donut's program alone: only the blink (her pin holds the boost).
        let (granted, skipped, _) = juicer(false, true, false);
        assert_eq!(granted, [(1, "special_short_blink")]);
        assert_eq!(skipped, 1, "Carl's entry is skipped");
        // Both, in list order on one tick.
        let (granted, skipped, _) = juicer(true, true, false);
        assert_eq!(
            granted,
            [(0, "fist_reinforced"), (1, "special_short_blink")]
        );
        assert_eq!(skipped, 0);
        // A skipped entry changes nothing a hash can see: Carl alone hashes
        // exactly like a ledger that started with the pin and granted only the
        // fist at the same tick, with no skip anywhere.
        let (_, _, carl_words) = juicer(true, false, false);
        let mut plain = UnlockLedger::new(table, &[SPECIAL_THROWN_CHARGE]);
        plain.grant(FIST_REINFORCED, 9, "brute_boss_f01_defeated");
        let mut plain_words = Vec::new();
        plain.hash_into(&mut |v| plain_words.push(v));
        assert_eq!(carl_words, plain_words);
        let (_, _, both_words) = juicer(true, true, false);
        assert!(carl_words.len() < both_words.len());
        // The safe-room zone: armed by the Donut program only, one grant, and
        // only with the biscuit read through (the `requires` guard).
        let corridor = |carl: bool, biscuit: bool| {
            let setup = program_setup(
                CarlProgFlags {
                    prog: carl,
                    ..CarlProgFlags::OFF
                },
                DonutProgFlags {
                    prog: !carl,
                    ..DonutProgFlags::OFF
                },
                SceneKind::Corridor,
            );
            let mut runtime = StoryRuntime::new(&setup)?;
            let view = ReadThrough {
                primary_ranged: biscuit,
            };
            let inside = [(0.0, 0.0), (1210.0, 0.0)];
            let first = runtime.step(1, inside, false, &view);
            let again = runtime.step(2, inside, false, &view);
            assert!(again.is_empty(), "a trigger is consumed, never re-armed");
            Some((first.len(), runtime.debug_log().len()))
        };
        assert_eq!(corridor(false, true), Some((1, 0)));
        assert_eq!(
            corridor(false, false),
            Some((0, 1)),
            "without the biscuit the entry is skipped and the trigger is spent"
        );
        assert_eq!(
            corridor(true, true),
            None,
            "`--carl-prog` never evaluates the safe-room zone"
        );
        assert_eq!(
            table
                .trigger("brute_boss_f01_defeated")
                .unwrap()
                .grants
                .len(),
            2
        );
    }

    /// `--donut-prog` (alone and with `--carl-prog`) on the DEMO-01 hall and the
    /// CF02 microarena reproduces the unflagged run exactly: nothing is held and
    /// nothing is armed there. The opening route runs no combat state at all.
    #[test]
    fn donut_prog_flag_parity() {
        for carl in [
            CarlProgFlags::OFF,
            CarlProgFlags {
                prog: true,
                ..CarlProgFlags::OFF
            },
        ] {
            // DEMO-01 hall, the autopilot from the first beat.
            let hall = load_hall(&root()).unwrap();
            let demo = |story: Option<StorySetup>| {
                let fixture = fixture_for_beat(0);
                let scene = SceneSim::new_at(&hall, 360, fixture.carl_x).unwrap();
                let mut combat = CombatState::new(fixture, &hall, &scene)
                    .unwrap()
                    .with_demo(&DEMO_RULES);
                if let Some(story) = story {
                    combat = combat.with_story(story);
                }
                let mut run = FixtureSim { scene, combat };
                for _ in 0..1500 {
                    let raw = autopilot(&run.combat, &run.scene);
                    run.step(raw);
                }
                run
            };
            let plain = demo(None);
            let flagged = demo(Some(program_setup(carl, DONUT_PROG, SceneKind::Demo)));
            assert_eq!(plain.state_hash(), flagged.state_hash());
            assert_eq!(plain.combat.events, flagged.combat.events);
            assert!(flagged.combat.story_ledger().is_none());
            // CF02 microarena, the direct plan.
            let arena = crate::cf02::arena::load_arena(&root()).unwrap();
            let cf02 = |story: Option<StorySetup>| {
                let fixture =
                    crate::cf02::arena::fixture_for_run(0, false, crate::engineer::active());
                let scene = SceneSim::new_at(&arena, 360, fixture.carl_x).unwrap();
                let mut combat = CombatState::new(fixture, &arena, &scene)
                    .unwrap()
                    .with_demo(&crate::cf02::arena::RULES)
                    .with_cf02();
                if let Some(story) = story {
                    combat = combat.with_story(story);
                }
                let mut run = FixtureSim { scene, combat };
                for _ in 0..1500 {
                    let raw = crate::cf02::arena::autopilot(
                        &run.combat,
                        &run.scene,
                        crate::cf02::arena::Plan::Direct,
                    );
                    run.step(raw);
                }
                run
            };
            let plain = cf02(None);
            let flagged = cf02(Some(program_setup(carl, DONUT_PROG, SceneKind::CfArena)));
            assert_eq!(plain.state_hash(), flagged.state_hash());
            assert_eq!(plain.combat.events, flagged.combat.events);
        }
        // The start flags are different: they hold a capability from tick 0 and
        // so change the hash (the opt-in is visible, never silent), and they
        // grant nothing (a declared start, no event).
        let hall = load_hall(&root()).unwrap();
        let held = {
            let fixture = fixture_for_beat(0);
            let scene = SceneSim::new_at(&hall, 360, fixture.carl_x).unwrap();
            let combat = CombatState::new(fixture, &hall, &scene)
                .unwrap()
                .with_demo(&DEMO_RULES)
                .with_donut(DonutSetup::unlocked(8))
                .with_story(program_setup(
                    CarlProgFlags::OFF,
                    DonutProgFlags {
                        puddle: true,
                        ..DonutProgFlags::OFF
                    },
                    SceneKind::Demo,
                ));
            FixtureSim { scene, combat }
        };
        assert!(held.combat.story_holds(blink()) && held.combat.story_holds(boost()));
        assert!(
            held.combat
                .events
                .iter()
                .all(|e| !matches!(e, CombatEvent::CapabilityGranted { .. })),
            "a start flag is a declared start state, not a grant"
        );
        assert!(!held.combat.story_ledger().unwrap().is_empty());
        let _ = (FIST_REINFORCED, PRIMARY_RANGED);
    }

    // ---------------------------------------------------------------- DP-B ----
    //
    // Light on Your Feet: the launch Donut has now (DP-Q14..Q17), her reach and
    // the nav graph from it, the lane-ceiling data test (DP-Q45). The motion
    // fixtures below are bodies on the real `step_hero_motion`, in the shape of
    // DM-A's `dm_*` motion fixtures: not registered, no CLI name, no replay.

    use crate::agility::{self, agile_motion, discrete_apex_px};
    use crate::agility_fixtures::{Hold, MotionFixture, blocks_of, run as run_motion};
    use crate::prototype::HeroMotion;

    /// DP-Q14: the adopted launch (px/s) while `jump_reach_boost` is held.
    fn boost_launch() -> f32 {
        f32::from(crate::donut_prog::DonutProgTuning::active().dpq14_boost_launch_px_s)
    }

    fn boosted() -> HeroMotion {
        HeroMotion {
            jump_speed: boost_launch(),
            ..agile_motion()
        }
    }

    const LEDGE_LEFT: f32 = 200.0;
    const AT_FACE_X: f32 = LEDGE_LEFT - crate::prototype::HERO_HALF_WIDTH;

    /// Hold right into the ledge and jump on tick 0 (DM-A's climb script).
    const CLIMB: [Hold; 2] = [
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

    const fn ledge(name: &'static str, summary: &'static str, height: f32) -> MotionFixture {
        MotionFixture {
            name,
            summary,
            start_x: AT_FACE_X,
            ledge: Some((LEDGE_LEFT, 400.0, height)),
            script: &CLIMB,
            ticks: 120,
        }
    }

    const DONUT_PROG_LEDGE_128: MotionFixture = ledge(
        "donut_prog_ledge_128",
        "DP-Q18: the 128 px gate class: boosted Donut lands (a 9.95 px margin), base Donut and Carl do not",
        128.0,
    );
    const DONUT_PROG_LEDGE_144_FAILS: MotionFixture = ledge(
        "donut_prog_ledge_144_fails",
        "DP-Q18: 144 px is out of reach for everyone, the boost included",
        144.0,
    );
    const DONUT_PROG_LEDGE_112_MARGIN: MotionFixture = ledge(
        "donut_prog_ledge_112_margin",
        "DP-Q14: the 112 px perch is no longer a 2.9 px squeeze: the boost lands it with 25.95 px to spare",
        112.0,
    );
    /// Three press edges (ticks 0, 12, 24, each released in between) in one air
    /// phase: DP-Q15, no double jump.
    const DONUT_PROG_NO_AIR_JUMP: MotionFixture = MotionFixture {
        name: "donut_prog_no_air_jump",
        summary: "DP-Q15: presses in the air do nothing: the apex is the single jump's",
        start_x: 100.0,
        ledge: None,
        script: &[
            Hold {
                from: 0,
                to: 1,
                move_x: 0.0,
                jump: true,
            },
            Hold {
                from: 12,
                to: 13,
                move_x: 0.0,
                jump: true,
            },
            Hold {
                from: 24,
                to: 25,
                move_x: 0.0,
                jump: true,
            },
        ],
        ticks: 90,
    };

    const DP_MOTION_FIXTURES: [&MotionFixture; 4] = [
        &DONUT_PROG_LEDGE_128,
        &DONUT_PROG_LEDGE_144_FAILS,
        &DONUT_PROG_LEDGE_112_MARGIN,
        &DONUT_PROG_NO_AIR_JUMP,
    ];

    fn lands_on(motion: HeroMotion, f: &MotionFixture) -> bool {
        let (left, width, height) = f.ledge.unwrap();
        let (x, feet, _, grounded) = run_motion(motion, f).last();
        grounded && (feet - height).abs() < 0.01 && x > left && x < left + width
    }

    #[test]
    fn dpq14_the_adopted_launch_has_the_spec_numbers() {
        assert_eq!(boost_launch(), 547.0);
        let apex = discrete_apex_px(boost_launch());
        assert!((apex - 137.95).abs() < 0.05, "{apex}");
        let base = discrete_apex_px(agile_motion().jump_speed);
        assert!((base - 114.92).abs() < 0.05, "{base}");
        assert!(((apex / base - 1.0) * 100.0 - 20.0).abs() < 0.2, "+20 %");
        assert_eq!(agility::air_ticks(boost_launch()), 62, "62 air ticks");
        assert_eq!(agility::air_ticks(agile_motion().jump_speed), 57);
        // Run speed, gravity and everything else are DM-A's, unchanged.
        assert_eq!(boosted().run_speed, agile_motion().run_speed);
    }

    #[test]
    fn donut_prog_ledge_128() {
        let f = &DONUT_PROG_LEDGE_128;
        assert!(lands_on(boosted(), f), "boosted Donut lands on 128");
        assert!(!lands_on(agile_motion(), f), "base Donut does not");
        assert!(!lands_on(HeroMotion::CARL, f), "Carl does not");
        let margin = discrete_apex_px(boost_launch()) - 128.0;
        assert!((margin - 9.95).abs() < 0.05, "margin {margin}");
    }

    #[test]
    fn donut_prog_ledge_144_fails() {
        let f = &DONUT_PROG_LEDGE_144_FAILS;
        assert!(!lands_on(boosted(), f));
        let apex = run_motion(boosted(), f).apex();
        assert!(apex < 144.0 && apex > 137.0, "apex {apex}");
    }

    #[test]
    fn donut_prog_ledge_112_margin() {
        let f = &DONUT_PROG_LEDGE_112_MARGIN;
        assert!(lands_on(boosted(), f));
        assert!(
            lands_on(agile_motion(), f),
            "base Donut still lands it, barely"
        );
        let boosted_margin = discrete_apex_px(boost_launch()) - 112.0;
        let base_margin = discrete_apex_px(agile_motion().jump_speed) - 112.0;
        assert!((boosted_margin - 25.95).abs() < 0.05, "{boosted_margin}");
        assert!((base_margin - 2.92).abs() < 0.05, "{base_margin}");
    }

    #[test]
    fn donut_prog_no_air_jump() {
        let f = &DONUT_PROG_NO_AIR_JUMP;
        let trail = run_motion(boosted(), f);
        let single = discrete_apex_px(boost_launch());
        assert!(
            (trail.apex() - single).abs() < 0.05,
            "apex {} vs {single}",
            trail.apex()
        );
        // One air phase only: after the first landing she never leaves the floor
        // again, and no tick of the run rises above the single apex.
        let landed = trail
            .states
            .iter()
            .position(|s| s.3 && s.1 == 0.0 && s.2 == 0.0);
        assert!(landed.is_some_and(|t| t > 40), "{landed:?}");
        let ground = trail
            .states
            .iter()
            .skip(landed.unwrap() + 2)
            .all(|s| s.1 == 0.0);
        assert!(ground, "no second jump after landing (no press then)");
        // The same presses with the base launch give the base apex: the rule is
        // the engine's, the boost only raises the launch (DP-Q15).
        let base = run_motion(agile_motion(), f);
        assert!((base.apex() - discrete_apex_px(agile_motion().jump_speed)).abs() < 0.05);
    }

    /// The pinned trail hashes `(fixture, boosted, base Donut)`, the physical
    /// asserts above are the independent check of what they mean.
    const MOTION_GOLDENS: [(&str, u64, u64); 4] = [
        (
            "donut_prog_ledge_128",
            0xfa2d9695b94b28b8,
            0xd4483f752453bc4f,
        ),
        (
            "donut_prog_ledge_144_fails",
            0xb93ebfad79d8e55a,
            0xd4483f752453bc4f,
        ),
        (
            "donut_prog_ledge_112_margin",
            0x9aec5010b46c8687,
            0x960e01ad04607be6,
        ),
        (
            "donut_prog_no_air_jump",
            0x60ff02c0e88a0a4e,
            0x79fd1e2a96795947,
        ),
    ];

    #[test]
    #[ignore = "prints the motion goldens"]
    fn print_motion_goldens() {
        for f in DP_MOTION_FIXTURES {
            println!(
                "        (\"{}\", {:#018x}, {:#018x}),",
                f.name,
                run_motion(boosted(), f).hash,
                run_motion(agile_motion(), f).hash
            );
        }
    }

    #[test]
    fn the_motion_fixtures_match_their_goldens_and_replay_identically() {
        assert_eq!(MOTION_GOLDENS.len(), DP_MOTION_FIXTURES.len());
        for (f, (name, boosted_hash, base_hash)) in DP_MOTION_FIXTURES.iter().zip(MOTION_GOLDENS) {
            assert_eq!(f.name, name);
            let _ = blocks_of(f);
            assert_eq!(run_motion(boosted(), f).hash, run_motion(boosted(), f).hash);
            assert_eq!(
                (
                    run_motion(boosted(), f).hash,
                    run_motion(agile_motion(), f).hash
                ),
                (boosted_hash, base_hash),
                "{name}"
            );
        }
    }

    /// DP-Q16: the follower bot's reach is the boosted apex less the DM-Q08
    /// margin (133.95 px at 547), exact; `None` and Carl are unchanged. A 128 px
    /// ledge is attempted by the boosted bot only.
    #[test]
    fn dpq16_the_followers_reach_follows_the_launch_she_has_now() {
        use crate::follower::{jump_reach_px, jump_reach_px_now};
        use crate::party::HeroId;
        let tuning = PROVISIONAL_TUNING;
        let reach = jump_reach_px_now(HeroId::Donut, &tuning, Some(boost_launch()));
        let want = discrete_apex_px(boost_launch()) - agility::tuning().dm_q08_ai_jump_margin_px;
        assert_eq!(reach.to_bits(), want.to_bits());
        assert!((reach - 133.95).abs() < 0.06, "{reach}");
        assert_eq!(
            jump_reach_px_now(HeroId::Donut, &tuning, None).to_bits(),
            jump_reach_px(HeroId::Donut, &tuning).to_bits()
        );
        assert_eq!(
            jump_reach_px_now(HeroId::Carl, &tuning, Some(boost_launch())).to_bits(),
            jump_reach_px(HeroId::Carl, &tuning).to_bits(),
            "the launch is Donut's only"
        );
        assert!(reach > 128.0 && jump_reach_px(HeroId::Donut, &tuning) < 128.0);
    }

    /// DP-Q16, nav part (the diff): the graph built with the boosted launch
    /// opens the floor-to-128 edge for Donut and no other: Carl's edges are
    /// identical, Donut's base edges stay open, and the 144 px edge stays closed.
    #[test]
    fn the_boosted_nav_graph_adds_only_donuts_128_edge() {
        use crate::companion::nav::{EdgeState, NavConfig, NavGraph, NavLevel};
        use crate::party::HeroId;
        let build = |height: f32, launch: Option<f32>| {
            let blocks: Vec<crate::prototype::Block> = blocks_of(&ledge("x", "x", height))
                .into_iter()
                .map(|mut b| {
                    b.rect.left += 100.0;
                    b
                })
                .collect();
            let config = launch.map_or(NavConfig::agile(), |l| {
                NavConfig::agile().with_donut_launch(l)
            });
            NavGraph::build(&NavLevel::from_blocks(&blocks, Vec::new()), config).unwrap()
        };
        let open = |g: &NavGraph, height: f32, hero: HeroId| {
            let top = |id| g.anchor_pos(id).unwrap().1;
            g.edge_views()
                .iter()
                .find(|e| top(e.from) == 0.0 && (top(e.to) - height).abs() < 0.01)
                .is_some_and(|e| matches!(e.state[hero.index()], EdgeState::Open { .. }))
        };
        let b = boost_launch();
        for (height, base_donut, boost_donut) in [
            (96.0, true, true),
            (104.0, true, true),
            (112.0, true, true),
            (128.0, false, true),
            (144.0, false, false),
        ] {
            let (g0, g1) = (build(height, None), build(height, Some(b)));
            assert_eq!(
                open(&g0, height, HeroId::Donut),
                base_donut,
                "base {height}"
            );
            assert_eq!(
                open(&g1, height, HeroId::Donut),
                boost_donut,
                "boosted {height}"
            );
            // Carl: the same edges with and without the boost.
            assert_eq!(
                open(&g0, height, HeroId::Carl),
                open(&g1, height, HeroId::Carl)
            );
            assert!(
                !open(&g1, height, HeroId::Carl) || height <= 80.0,
                "Carl {height}"
            );
        }
        // A graph built with the launch Donut already has is the graph of
        // `NavConfig::agile()`: a run without the boost never rebuilds.
        let base_launch = agile_motion().jump_speed;
        assert_eq!(
            NavConfig::agile().with_donut_launch(base_launch),
            NavConfig::agile()
        );
    }

    /// DP-Q17: the launch is chosen from the ledger **every tick** by the same
    /// step for every controller. A live party in a flat room: Donut's apex is
    /// the boosted one when `donut_launch` is set (a 2P pad, the solo-controlled
    /// body and the bot's own jump decision all go through the one
    /// `donut_motion_now`) and the base one when it is `None`; Carl's never moves.
    #[test]
    fn donut_prog_boost_every_controller() {
        use crate::corridor::PartyStepCtx;
        use crate::party::{HeroId, PartyDrive};
        let kit = EnvKit::load(&root(), LAYOUT).unwrap();
        let jump = RawInput {
            jump: true,
            ..RawInput::NONE
        };
        let apex_of = |drive: PartyDrive, launch: Option<f32>, who: HeroId| {
            let mut scene = SceneSim::new(&kit, 360).unwrap();
            scene.set_drive(drive);
            let ctx = PartyStepCtx {
                donut_launch: launch,
                ..Default::default()
            };
            let floor = scene.world.body(who).y;
            let mut apex = 0.0_f32;
            for tick in 0..90 {
                let mut inputs = [RawInput::NONE; 2];
                if tick == 2 {
                    inputs[who.index()] = jump;
                }
                scene.step_party_ctx(inputs, &ctx);
                apex = apex.max(scene.world.body(who).y - floor);
            }
            apex
        };
        let base = discrete_apex_px(agile_motion().jump_speed);
        let high = discrete_apex_px(boost_launch());
        let two_player = PartyDrive {
            live: true,
            two_player: true,
            controlled: HeroId::Carl,
            bots: [false, false],
            tuning: PROVISIONAL_TUNING,
        };
        let solo_donut = PartyDrive {
            live: true,
            two_player: false,
            controlled: HeroId::Donut,
            bots: [true, false],
            tuning: PROVISIONAL_TUNING,
        };
        for (name, drive) in [
            ("2P pad", two_player),
            ("solo, Donut controlled", solo_donut),
        ] {
            let with = apex_of(drive, Some(boost_launch()), HeroId::Donut);
            let without = apex_of(drive, None, HeroId::Donut);
            assert!(
                (with - high).abs() < 0.1,
                "{name}: boosted apex {with} vs {high}"
            );
            assert!(
                (without - base).abs() < 0.1,
                "{name}: base apex {without} vs {base}"
            );
        }
        // Carl's jump never takes the launch.
        let carl_with = apex_of(two_player, Some(boost_launch()), HeroId::Carl);
        let carl_without = apex_of(two_player, None, HeroId::Carl);
        assert_eq!(carl_with.to_bits(), carl_without.to_bits());
    }

    /// DP-Q17 through the combat layer: with the ledger holding the boost the
    /// launch is set every tick, and with no story (every run without a Donut
    /// capability) it is `None`.
    #[test]
    fn the_launch_comes_from_the_ledger_every_tick() {
        let before = replay_to(&DONUT_PROG_STORY_UNLOCKS, 100);
        assert_eq!(before.combat.donut_launch_now(), None, "before the grant");
        let after = replay(&DONUT_PROG_STORY_UNLOCKS);
        assert_eq!(after.combat.donut_launch_now(), Some(boost_launch()));
        // No story runtime: never.
        let plain = replay(find_party_fixture("cf02_assist_cadence").unwrap());
        assert_eq!(plain.combat.donut_launch_now(), None);
    }

    fn replay_to(fixture: &'static PartyFixture, ticks: u64) -> PartyFixtureSim {
        let mut sim = PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        sim.run_scripted(ticks);
        sim
    }

    // ------------------------------------------ DP-Q45: the lane-ceiling sweep ----

    /// The DP-Q45 rule on one room: a standing surface more than `limit` above
    /// the walking line needs a `ceiling_px` between 1 and `max`.
    fn dpq45_violation(
        room: &crate::camera_lab::room::Room,
        limit: i32,
        max: i32,
    ) -> Option<String> {
        let rise = room
            .platforms
            .iter()
            .map(|p| room.floor_y - p.rect.y)
            .chain(room.steps.iter().map(|r| room.floor_y - r.y))
            .max()
            .unwrap_or(0);
        if rise <= limit {
            return None;
        }
        match room.ceiling_px {
            None => Some(format!(
                "{}: a {rise} px surface without ceiling_px",
                room.id
            )),
            Some(c) if !(1..=max).contains(&c) => {
                Some(format!("{}: ceiling_px {c} is outside 1..={max}", room.id))
            }
            Some(_) => None,
        }
    }

    /// DP-Q45 (K8): every room file the game can load, present and future (the
    /// directory is listed, not a fixed list), obeys the lane-ceiling rule. A
    /// room file whose lowest surfaces need no ceiling passes; the lab rooms are
    /// excluded by DP-Q19 (the boost is refused there).
    #[test]
    fn dpq45_every_room_with_a_high_surface_declares_a_lane_ceiling() {
        let t = crate::donut_prog::DonutProgTuning::active();
        let (limit, max) = (
            i32::from(t.dpq45_surface_limit_px),
            i32::from(t.dpq45_ceiling_px_max),
        );
        let dir = root().join("assets/levels/floor01");
        let mut checked = 0;
        let mut high_rooms = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if !name.ends_with(".room.json") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let rooms = crate::camera_lab::room::parse_rooms(&text)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!rooms.is_empty(), "{name}");
            for room in &rooms {
                checked += 1;
                if let Some(why) = dpq45_violation(room, limit, max) {
                    panic!("{name}: {why}");
                }
                high_rooms += usize::from(room.ceiling_px.is_some());
            }
        }
        assert!(checked >= 8, "the sweep found the room files ({checked})");
        assert!(
            high_rooms >= 6,
            "the setpiece rooms declare their ceiling ({high_rooms})"
        );
    }

    /// The negative: the same fixture room with a 128 px ledge and no ceiling
    /// fails the rule, with the ceiling it passes, and a ceiling above the
    /// maximum fails too (a rule never silently skipped).
    #[test]
    fn dpq45_a_128_ledge_room_needs_its_ceiling() {
        let t = crate::donut_prog::DonutProgTuning::active();
        let (limit, max) = (
            i32::from(t.dpq45_surface_limit_px),
            i32::from(t.dpq45_ceiling_px_max),
        );
        let text =
            std::fs::read_to_string(root().join("assets/levels/floor01/f01_boiler.room.json"))
                .unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let rooms = crate::camera_lab::room::parse_rooms(&text).unwrap();
        assert_eq!(
            dpq45_violation(&rooms[0], limit, max),
            None,
            "the shipped room passes"
        );
        value["scenarios"][0]
            .as_object_mut()
            .unwrap()
            .remove("ceiling_px");
        let bare = crate::camera_lab::room::parse_rooms(&value.to_string()).unwrap();
        let why = dpq45_violation(&bare[0], limit, max).expect("no ceiling fails");
        assert!(why.contains("without ceiling_px"), "{why}");
        value["scenarios"][0]["ceiling_px"] = serde_json::json!(max + 4);
        let high = crate::camera_lab::room::parse_rooms(&value.to_string()).unwrap();
        assert!(
            dpq45_violation(&high[0], limit, max)
                .unwrap()
                .contains("outside")
        );
        value["scenarios"][0]["ceiling_px"] = serde_json::json!(max);
        let ok = crate::camera_lab::room::parse_rooms(&value.to_string()).unwrap();
        assert_eq!(dpq45_violation(&ok[0], limit, max), None);
    }

    /// The kit-format story layout has no authored perch: every solid above 42 px
    /// is a wall piece or the ceiling slab (so it is outside the sweep above).
    #[test]
    fn dpq45_the_story_corridor_layout_has_no_high_standing_surface() {
        let kit = EnvKit::load(&root(), LAYOUT).unwrap();
        for r in &kit.solid_rects {
            let rise = kit.base_walk_line_y - r.y;
            if rise > 42 {
                assert!(
                    r.w <= 16 || r.w >= 2048,
                    "a standable solid {r:?} rises {rise} px"
                );
            }
        }
    }
}

// DP-C: the blink's fixtures (spec section 10, M3), test harness only.
#[cfg(test)]
mod blink_fixtures;

// DP-E: the autonomous Donut's blink fixtures (spec section 9).
#[cfg(test)]
mod ai_blink_fixtures;

// DP-D: the presentation of the blink over the DP-C rig (spec section 17).
#[cfg(test)]
mod presentation_tests;

// DONUT-PROG-01 DP-F: encounter completion with deterministic pilots.
#[cfg(test)]
mod pilot_fixtures;
