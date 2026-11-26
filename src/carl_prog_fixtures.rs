//! CARL-PROG-01 fixtures (the `carl_prog_*` namespace): the story-unlock
//! ledger as replay fixtures.
//!
//! Bevy-free party fixtures in the shape of [`crate::party_scene`]. Each one
//! declares its own story pin ([`story_setup`]: the capabilities it starts with
//! and the triggers armed in it, never a debug flag), has a committed replay in
//! `docs/evidence/carl-prog-01/replays` and a golden hash pinned by a test in
//! this file. They are **not** in [`PartyFixture::all`]: the registry is
//! [`CARL_PROG_FIXTURES`], listed by `floor01_scene --list-fixtures carl_prog_`
//! and replayed by the CI loop, so every earlier fixture, golden and replay is
//! untouched. Later CARL-PROG PRs append their fixtures here.
//!
//! Tick numbering is the one of [`crate::combat_scene`]: a hold over
//! `from..to` covers the steps that produce `tick = from .. to - 1`.

use crate::blast::Arm;
use crate::boss_scene::{FULL, FULL_NO_RACKS, Pilot, boss_base, hp};
use crate::carl_prog::CarlProgFlags;
use crate::combat::EnemyAi;
use crate::combat_scene::{ChargeSpawn, EnemySpawn, Fixture, LlamaSpawn};
use crate::donut_mana::DonutSetup;
use crate::party::PROVISIONAL_TUNING;
use crate::party_scene::{PartyFixture, SlotHold, base, run, swap};
use crate::story_unlock::{SceneKind, StorySetup};

/// The placeholder zone of `pile_boss_f01_defeated` (K1, f01 integration plan
/// #182: the thrown charge is granted on the Hoarder's defeat; the Hoarder does
/// not exist yet, HRD-A, so a stub zone stands in): Carl's pivot inside this half-open x range grants the
/// thrown charge. Read from `story_unlocks_v0.json`, repeated here only so the
/// scripts below can be read next to it.
pub const FIND_STUB_ZONE_X: (f32, f32) = (600.0, 640.0);

/// Whether a fixture runs under `cf02_rules` (CP-Q36: the body contacts and wall
/// slams are CF02-B rules, so the two fixtures that prove them switch it on by
/// name, like every `cf02_*` fixture). Read by [`crate::cf02::fixture_runs_cf02`].
pub fn runs_cf02(name: &str) -> bool {
    matches!(name, "carl_prog_dyn_bowling" | "carl_prog_dyn_wall_slam")
        || name.starts_with("carl_prog_pierce_engineer")
}

const fn chase(x: i32) -> EnemySpawn {
    EnemySpawn {
        x,
        ai: EnemyAi::Chase,
    }
}

/// The story a `carl_prog_*` fixture declares for itself (a scripted fixture
/// never reads a debug flag, CP-Q09). Every other fixture name is the legacy
/// game: [`StorySetup::NONE`].
pub fn story_setup(fixture: &str) -> StorySetup {
    match fixture {
        // The placeholder Hoarder-defeat zone is armed by the fixture, by id: no scene arms it.
        "carl_prog_story_event_grants_dynamite" | "carl_prog_grant_survives_respawn" => {
            StorySetup::NONE.with_trigger("carl_prog_find_zone_stub")
        }
        // The boss pin and the Juicer trigger, as `--carl-prog` gives the arena.
        "carl_prog_juicer_grants_fist" => CarlProgFlags {
            prog: true,
            ..CarlProgFlags::OFF
        }
        .story_setup(SceneKind::Boss),
        // CP-C: the dynamite clear runs under the boss pin (`--carl-prog` on the arena:
        // the thrown charge is held, the Juicer's defeat grants the fist).
        "carl_prog_juicer_dynamite_clear" => CarlProgFlags {
            prog: true,
            ..CarlProgFlags::OFF
        }
        .story_setup(SceneKind::Boss),
        // CP-B1: before the grant (the ledger exists, nothing is held): a press is inert.
        "carl_prog_special_locked" => StorySetup::NONE.with_trigger("carl_prog_find_zone_stub"),
        // CP-D: the reinforced fist is held from tick 0 (a declared start, not a grant).
        // The pierce control and the kick control are the same rooms without it.
        "carl_prog_fist_stagger"
        | "carl_prog_pierce_goblin"
        | "carl_prog_pierce_engineer"
        | "carl_prog_boss_poise"
        | "carl_prog_shore_combo"
        | "carl_prog_kick_unchanged"
        | "carl_prog_pierce_goblin_jab"
        // CORRAL-B wire, KC12: the haymaker against the llama's committed rush.
        // Its control (`corral_rush_pierce_control`) is the same room without it.
        | "corral_rush_pierce_haymaker" => StorySetup::NONE.with_hold("fist_reinforced"),
        // CP-B1: the thrown charge is held from tick 0 (a declared start, not a grant).
        name if name.starts_with("carl_prog_dyn_") => {
            StorySetup::NONE.with_hold("special_thrown_charge")
        }
        // The second hero's registry (DONUT-PROG-01) declares its own pins the same way.
        other => crate::donut_prog_fixtures::story_setup(other),
    }
}

const FIND_BASE: Fixture = base(
    "carl_prog_story_event_grants_dynamite",
    "Solo: Carl walks into the placeholder Hoarder-defeat zone and the ledger grants the thrown charge once; walking on and back grants nothing more",
    0x0CA1_0001,
    &[],
    240,
);

/// CP-Q01, CP-Q11: a zone trigger grants a capability exactly once, with one
/// `CapabilityGranted` on the first tick Carl's pivot is inside.
pub const CARL_PROG_STORY_EVENT_GRANTS_DYNAMITE: PartyFixture = PartyFixture {
    base: &FIND_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[run(5, 110, 0, 1.0), run(130, 200, 0, -1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const RESPAWN_BASE: Fixture = base(
    "carl_prog_grant_survives_respawn",
    "Solo: Carl takes the placeholder zone grant, two goblins defeat him and the party retries: the grant stays and is not granted twice",
    0x0CA1_0002,
    &[chase(700), chase(740)],
    1500,
);

/// CP-Q07: a grant survives a defeat retry (the demo's checkpoint respawn and a
/// solo swap are unit-tested); a fixture restart removes it.
pub const CARL_PROG_GRANT_SURVIVES_RESPAWN: PartyFixture = PartyFixture {
    base: &RESPAWN_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[run(5, 110, 0, 1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const JUICER_BASE: Fixture = boss_base(
    "carl_prog_juicer_grants_fist",
    "The boss has 4 HP left and Carl holds the thrown charge from the pin: the rack crush defeats it and the Juicer trigger grants the reinforced fist on that tick",
    0x0CA1_0003,
    180,
    hp(4),
    420,
);

/// CP-Q02: the boss defeat is the event; exactly one `CapabilityGranted`
/// (`fist_reinforced`) on the defeat tick, and none for the pin.
pub const CARL_PROG_JUICER_GRANTS_FIST: PartyFixture = PartyFixture {
    base: &JUICER_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::CarlRack,
    donut: DonutSetup::NONE,
};

/// A Circle/B press edge on `slot` at `tick` (CP-B1: held for one tick).
const fn special(tick: u64, slot: usize) -> SlotHold {
    SlotHold {
        from: tick,
        to: tick + 1,
        slot,
        input: crate::input::RawInput {
            ability: true,
            ..crate::input::RawInput::NONE
        },
    }
}

const fn passive(x: i32) -> EnemySpawn {
    EnemySpawn {
        x,
        ai: EnemyAi::Passive,
    }
}

const LOCKED_BASE: Fixture = base(
    "carl_prog_special_locked",
    "Solo: 40 Circle/B presses before any grant; nothing happens and nothing is logged (the ledger exists and holds nothing)",
    0x0CA1_0010,
    &[],
    90,
);

/// CP-B1: before the dynamite grant the special is inert.
pub const CARL_PROG_SPECIAL_LOCKED: PartyFixture = PartyFixture {
    base: &LOCKED_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[
        special(2, 0),
        special(4, 0),
        special(6, 0),
        special(8, 0),
        special(10, 0),
        special(12, 0),
        special(14, 0),
        special(16, 0),
        special(18, 0),
        special(20, 0),
        special(22, 0),
        special(24, 0),
        special(26, 0),
        special(28, 0),
        special(30, 0),
        special(32, 0),
        special(34, 0),
        special(36, 0),
        special(38, 0),
        special(40, 0),
        special(42, 0),
        special(44, 0),
        special(46, 0),
        special(48, 0),
        special(50, 0),
        special(52, 0),
        special(54, 0),
        special(56, 0),
        special(58, 0),
        special(60, 0),
        special(62, 0),
        special(64, 0),
        special(66, 0),
        special(68, 0),
        special(70, 0),
        special(72, 0),
        special(74, 0),
        special(76, 0),
        special(78, 0),
        special(80, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const ARC_STANDING_BASE: Fixture = base(
    "carl_prog_dyn_arc_standing",
    "Solo, thrown charge held: a standing throw on tick 4, the stick spawns on t41, lands about 111 px ahead and blows 150 ticks after the release",
    0x0CA1_0011,
    &[],
    260,
);

/// CP-Q14..CP-Q18: the standing arc, spawn, landing and fuse.
pub const CARL_PROG_DYN_ARC_STANDING: PartyFixture = PartyFixture {
    base: &ARC_STANDING_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const ARC_MOVING_BASE: Fixture = base(
    "carl_prog_dyn_arc_moving",
    "Solo, thrown charge held: Carl runs right and throws on tick 5; the stick carries half his speed and lands about 170 px ahead",
    0x0CA1_0012,
    &[],
    260,
);

/// CP-Q15, CP-Q17: the moving throw.
pub const CARL_PROG_DYN_ARC_MOVING: PartyFixture = PartyFixture {
    base: &ARC_MOVING_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[run(1, 120, 0, 1.0), special(5, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const ONE_ACTIVE_BASE: Fixture = base(
    "carl_prog_dyn_one_active",
    "Solo, thrown charge held: presses during the throw, the flight and the fuse are rejected as busy, during the recharge as cooldown; the press on detonation + 420 starts a second throw",
    0x0CA1_0013,
    &[],
    700,
);

/// CP-Q13, CP-Q19: one stick at a time, the 420-tick recharge.
pub const CARL_PROG_DYN_ONE_ACTIVE: PartyFixture = PartyFixture {
    base: &ONE_ACTIVE_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[
        special(4, 0),
        special(15, 0),
        special(50, 0),
        special(120, 0),
        special(194, 0),
        special(196, 0),
        special(400, 0),
        special(613, 0),
        special(615, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const CORE_KILL_BASE: Fixture = base(
    "carl_prog_dyn_core_kill",
    "Solo, thrown charge held: a goblin stands where the stick lands; the core bursts it on the detonation tick, credited to Carl",
    0x0CA1_0014,
    &[passive(511)],
    240,
);

/// CP-Q20: the 20 px core (point blank).
pub const CARL_PROG_DYN_CORE_KILL: PartyFixture = PartyFixture {
    base: &CORE_KILL_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const OUTER_LAUNCH_BASE: Fixture = base(
    "carl_prog_dyn_outer_launch",
    "Solo, thrown charge held: a goblin about 40 px from the landing is launched alive with 1 of 2 HP, credited to Carl",
    0x0CA1_0015,
    &[passive(563)],
    330,
);

/// CP-Q21: the 64 px outer ring (1 damage, a living launch).
pub const CARL_PROG_DYN_OUTER_LAUNCH: PartyFixture = PartyFixture {
    base: &OUTER_LAUNCH_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const NO_FRIENDLY_FIRE_BASE: Fixture = base(
    "carl_prog_dyn_no_friendly_fire",
    "Two players, thrown charge held: Donut walks into the blast and stands in its core; she is not damaged, launched or logged (Carl stands clear)",
    0x0CA1_0016,
    &[],
    260,
);

/// CP-Q24: no friendly fire (the `OwnerReduced` profile spares every other hero).
pub const CARL_PROG_DYN_NO_FRIENDLY_FIRE: PartyFixture = PartyFixture {
    base: &NO_FRIENDLY_FIRE_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0), run(100, 142, 1, 1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const SWAP_KEEPS_STICK_BASE: Fixture = base(
    "carl_prog_dyn_swap_keeps_stick",
    "Solo, thrown charge held: the player swaps to Donut while the fuse burns; the stick, its fuse and the kill credit stay Carl's",
    0x0CA1_0017,
    &[passive(511)],
    240,
);

/// CP-Q29: a solo swap mid-fuse changes nothing.
pub const CARL_PROG_DYN_SWAP_KEEPS_STICK: PartyFixture = PartyFixture {
    base: &SWAP_KEEPS_STICK_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0), swap(100, 101)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const BOSS_BASE: Fixture = boss_base(
    "carl_prog_dyn_boss",
    "The boss arena, thrown charge held: the stick lands at the boss and blows on tick 195 for 4 damage; his pound in progress is not interrupted",
    0x0CA1_0018,
    380,
    hp(10),
    260,
);

/// CP-Q25: 4 damage on the boss, super armour holds, no launch.
pub const CARL_PROG_DYN_BOSS: PartyFixture = PartyFixture {
    base: &BOSS_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// ------------------------------------------------------------------ CP-C ----
//
// The standing throw of Carl at tick 4 on `carl_x` 400 spawns the stick on t41
// (tick 45), lands it at x 511 on tick 83 and blows it on tick 195 (CP-B1). The
// blast centre is (511, walking line + 8); every column below is read against it.

/// An authored QA charge `dx` px right of Carl's start column, due on `tick`
/// (0 is not allowed for an `Arm::Tick` charge: a far tick is "lit, due later").
const fn charge(dx: i32, tick: u64) -> ChargeSpawn {
    ChargeSpawn {
        dx,
        tick,
        owner: None,
        arm: Arm::Tick,
    }
}

const CHAIN_CHARGE_BASE: Fixture = Fixture {
    charges: &[charge(165, 900)],
    ..base(
        "carl_prog_dyn_chain_charge",
        "Solo, thrown charge held: the blast reaches an authored charge 54 px away (lit, due on tick 900) and sets it to blow exactly 8 ticks later; it kills the goblin beside it, credited to Carl",
        0x0CA1_0020,
        &[passive(590)],
        260,
    )
};

/// CP-Q34: a chain sets an authored charge in reach to `now + 8`; the initiator
/// is inherited by a charge with no owner.
pub const CARL_PROG_DYN_CHAIN_CHARGE: PartyFixture = PartyFixture {
    base: &CHAIN_CHARGE_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const ARMS_TRIP_BASE: Fixture = Fixture {
    charges: &[ChargeSpawn {
        dx: 165,
        tick: 0,
        owner: None,
        arm: Arm::CarlPast {
            x: 1900.0,
            fuse_ticks: 75,
        },
    }],
    ..base(
        "carl_prog_dyn_arms_trip_charge",
        "Solo, thrown charge held: an unlit trip charge (armed only when Carl passes x 1900, which he never does) is armed by the blast on tick 195 and blows on tick 203, never read as already due",
        0x0CA1_0021,
        &[passive(590)],
        260,
    )
};

/// CP-Q34: an unlit `CarlPast` charge counts as armed; its stored tick of 0 is
/// never read as "earlier".
pub const CARL_PROG_DYN_ARMS_TRIP_CHARGE: PartyFixture = PartyFixture {
    base: &ARMS_TRIP_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DEPTH_CAP_BASE: Fixture = Fixture {
    charges: &[
        charge(165, 4000),
        charge(183, 4000),
        charge(201, 4000),
        charge(219, 4000),
        charge(237, 4000),
    ],
    ..base(
        "carl_prog_dyn_chain_depth_cap",
        "Solo, thrown charge held: five authored charges 18 px apart; the chain runs dynamite (0), then depths 1 to 4 on ticks 203, 211, 219 and 227, and the fifth (a depth-4 blast reaches it) never blows",
        0x0CA1_0022,
        &[],
        300,
    )
};

/// CP-Q34: depth 4 at most; a depth-4 blast chains nothing. Each QA charge chains
/// from its own 20 px radius, so only its neighbour is reached.
pub const CARL_PROG_DYN_CHAIN_DEPTH_CAP: PartyFixture = PartyFixture {
    base: &DEPTH_CAP_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DUE_EARLIER_BASE: Fixture = Fixture {
    charges: &[
        charge(170, 198),
        charge(158, 203),
        charge(166, 204),
        charge(174, 400),
        charge(189, 4000),
    ],
    ..base(
        "carl_prog_dyn_chain_due_earlier",
        "Solo, thrown charge held: the blast on tick 195 reaches charges due 198 and 203 (inside the 8-tick delay, both edges: left alone, but still reached, so depth 1) and due 204 and 400 (pulled to 203); charge 0 is out of the stick's reach for charge 4, which only charge 0's blast (depth 2, on tick 198) lights, for tick 206",
        0x0CA1_0023,
        &[],
        260,
    )
};

/// CP-Q34: a lit charge due inside `now < tick <= now + 8` is left alone; one due
/// later is pulled forward to `now + 8`.
pub const CARL_PROG_DYN_CHAIN_DUE_EARLIER: PartyFixture = PartyFixture {
    base: &DUE_EARLIER_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DEFER_BASE: Fixture = Fixture {
    charges: &[
        charge(56, 4000),
        charge(70, 4000),
        charge(80, 4000),
        charge(90, 4000),
        charge(100, 4000),
        charge(110, 4000),
        charge(120, 4000),
        charge(130, 4000),
        charge(140, 4000),
        // Out of the stick's reach (71 px) but lit for tick 203 on its own: only
        // charge 0's blast on tick 203 reaches it, and the cap then defers it.
        charge(40, 203),
    ],
    ..base(
        "carl_prog_dyn_chain_defer_over_8",
        "Solo, thrown charge held: the blast reaches nine authored charges at once, all due on tick 203, and a tenth (out of its reach, lit for 203) is reached by charge 0's blast on 203; eight blow on tick 203 and the other two, in index order, on tick 204, keeping the chain's depth and credit",
        0x0CA1_0024,
        &[],
        260,
    )
};

/// CP-Q34: at most 8 detonations per tick; the excess is deferred in index order.
pub const CARL_PROG_DYN_CHAIN_DEFER_OVER_8: PartyFixture = PartyFixture {
    base: &DEFER_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const BREAKS_SHORE_BASE: Fixture = Fixture {
    shore_dx: Some(140),
    ..base(
        "carl_prog_dyn_breaks_shore",
        "Solo, thrown charge held: the blast on tick 195 reaches the untouched rotten shore (29 px away) and breaks it outright; the block falls and crushes the goblin under it, credited to Carl",
        0x0CA1_0025,
        &[passive(600)],
        330,
    )
};

/// CP-Q35: the shore breaks outright by blast; the fall, crush and credit are
/// CP-03's, credited to the thrower.
pub const CARL_PROG_DYN_BREAKS_SHORE: PartyFixture = PartyFixture {
    base: &BREAKS_SHORE_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const BOWLING_BASE: Fixture = base(
    "carl_prog_dyn_bowling",
    "CF02 rules, thrown charge held: the blast launches goblin 0 (about 40 px away) alive; it flies into goblin 1 (beyond the ring) and knocks it down (GoblinContact), the contact rules of a normal launched body",
    0x0CA1_0026,
    &[passive(553), passive(600)],
    330,
);

/// CP-Q36: an outer-ring launch is a normal launched body, so the CF02-B body
/// contact applies. Bodies launched by one blast share one launch id (N9 of the
/// CP-B1 review), so one blast can bowl a given third goblin once.
pub const CARL_PROG_DYN_BOWLING: PartyFixture = PartyFixture {
    base: &BOWLING_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

/// Carl stands at x 1219 so the stick lands at 1330, 31 px left of a goblin that
/// stands 22 px short of the raised block at x 1392 on the corridor sample.
const WALL_SLAM_BASE: Fixture = Fixture {
    carl_x: 1219,
    ..base(
        "carl_prog_dyn_wall_slam",
        "CF02 rules, thrown charge held: the blast launches a goblin toward the raised block 10 px away; it meets the wall at speed and is slammed (WallSlam, 1 damage), the launch credited to Carl",
        0x0CA1_0027,
        &[passive(1370)],
        330,
    )
};

/// CP-Q36: a launched body meets a solid at speed: the CF02-B wall slam.
pub const CARL_PROG_DYN_WALL_SLAM: PartyFixture = PartyFixture {
    base: &WALL_SLAM_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const RACK_STRUT_BASE: Fixture = boss_base(
    "carl_prog_dyn_rack_strut",
    "The boss arena with the left rack on, thrown charge held: the stick lands at the strut and the blast on tick 195 drops the rack outright (no strut hit was thrown), credited to Carl",
    0x0CA1_0028,
    100,
    FULL,
    300,
);

/// CP-Q35: the rack strut breaks outright by blast; the fall and the crush are
/// the boss module's, credited to the thrower.
pub const CARL_PROG_DYN_RACK_STRUT: PartyFixture = PartyFixture {
    base: &RACK_STRUT_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const JUICER_CLEAR_BASE: Fixture = boss_base(
    "carl_prog_juicer_dynamite_clear",
    "Solo, boss pin (the thrown charge held): the thrower pilot plays the base kit and throws the charge at the Juicer when it is ready; the Juicer falls inside 2400..7200 ticks and the defeat grants the fist",
    0x0CA1_0030,
    56,
    FULL_NO_RACKS,
    7200,
);

/// CP-C, the Juicer with dynamite: a scripted solo clear under the boss pin. It
/// is the only user of `Pilot::ThrownCharge`; `boss01_full_clear_solo` (no flags,
/// no capability) is unchanged.
pub const CARL_PROG_JUICER_DYNAMITE_CLEAR: PartyFixture = PartyFixture {
    base: &JUICER_CLEAR_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::ThrownCharge,
    donut: DonutSetup::NONE,
};

/// A one-tick attack press on Carl (CP-D): the jab; a second press inside the
/// jab's chain window (move ticks 3..=13) starts the haymaker.
const fn punch(tick: u64) -> SlotHold {
    SlotHold {
        from: tick,
        to: tick + 1,
        slot: 0,
        input: crate::input::RawInput {
            attack: true,
            ..crate::input::RawInput::NONE
        },
    }
}

/// A one-tick physical (kick) press on Carl (CP-D).
const fn kick(tick: u64) -> SlotHold {
    SlotHold {
        from: tick,
        to: tick + 1,
        slot: 0,
        input: crate::input::RawInput {
            physical: true,
            ..crate::input::RawInput::NONE
        },
    }
}

const FIST_STAGGER_BASE: Fixture = base(
    "carl_prog_fist_stagger",
    "Solo, reinforced fist held: a whiffed jab chains into a haymaker that lands on the goblin at 436 (stagger 28 ticks, 18 without the fist); Carl then walks up to the goblin at 560 and a single jab staggers it 16 ticks (10 without); damage unchanged",
    0x0CA1_0040,
    &[passive(436), passive(560)],
    160,
);

/// CP-Q39: the reinforced punches stagger longer (jab 16, haymaker 28). The
/// first goblin stands just beyond the jab's reach and inside the haymaker's
/// step, so the string's second blow is the one that lands.
pub const CARL_PROG_FIST_STAGGER: PartyFixture = PartyFixture {
    base: &FIST_STAGGER_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[punch(10), punch(15), run(40, 85, 0, 1.0), punch(110)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

/// A one-tick attack press that turns Carl right (CP-D).
const fn punch_right(tick: u64) -> SlotHold {
    SlotHold {
        from: tick,
        to: tick + 1,
        slot: 0,
        input: crate::input::RawInput {
            attack: true,
            move_x: 1.0,
            ..crate::input::RawInput::NONE
        },
    }
}

const PIERCE_GOBLIN_BASE: Fixture = base(
    "carl_prog_pierce_goblin",
    "Solo, reinforced fist held: a chasing goblin tells on tick 40, its swing is committed from attack tick 24. Carl steps back 6 px so the jab (pressed on tick 53) whiffs, the chained haymaker (pressed on 58) hits on tick 65 (attack tick 24, the smear) and cancels the swing before the impact would land on tick 66: Carl is not hurt before the goblin's next attack",
    0x0CA1_0041,
    &[chase(470)],
    125,
);

/// CP-Q40, F-12: the haymaker pierces the goblin's committed smear. Its controls
/// are [`CARL_PROG_PIERCE_GOBLIN_CONTROL`] (no fist) and
/// [`CARL_PROG_PIERCE_GOBLIN_JAB`] (the fist, the jab).
pub const CARL_PROG_PIERCE_GOBLIN: PartyFixture = PartyFixture {
    base: &PIERCE_GOBLIN_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[run(41, 43, 0, -1.0), punch_right(53), punch(58)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const PIERCE_GOBLIN_CONTROL_BASE: Fixture = base(
    "carl_prog_pierce_goblin_control",
    "The control of carl_prog_pierce_goblin without the reinforced fist: the same string lands its haymaker on tick 65 but only damages the goblin, and the swing's impact lands on Carl on tick 72 (the hit's stop froze the goblin)",
    0x0CA1_0042,
    &[chase(470)],
    125,
);

/// CP-Q40 control: the same input without the capability lets the impact land.
pub const CARL_PROG_PIERCE_GOBLIN_CONTROL: PartyFixture = PartyFixture {
    base: &PIERCE_GOBLIN_CONTROL_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[run(41, 43, 0, -1.0), punch_right(53), punch(58)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const PIERCE_GOBLIN_JAB_BASE: Fixture = base(
    "carl_prog_pierce_goblin_jab",
    "Solo, reinforced fist held: the jab pressed on tick 62 hits the goblin on tick 65 (the smear) but does not pierce (F-12: the haymaker only): it is only hurt and the impact lands on Carl on tick 70",
    0x0CA1_0047,
    &[chase(470)],
    125,
);

/// F-12 control: with the fist held the jab no longer pierces (`cpq40_jab_pierces_commit`
/// is false); flipping that one row would make this the pierce fixture.
pub const CARL_PROG_PIERCE_GOBLIN_JAB: PartyFixture = PartyFixture {
    base: &PIERCE_GOBLIN_JAB_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[punch(62)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

/// A one-tick attack press that turns Carl left (the Engineer stands to his left).
const fn punch_left(tick: u64) -> SlotHold {
    SlotHold {
        from: tick,
        to: tick + 1,
        slot: 0,
        input: crate::input::RawInput {
            attack: true,
            move_x: -1.0,
            ..crate::input::RawInput::NONE
        },
    }
}

const PIERCE_ENGINEER_BASE: Fixture = Fixture {
    carl_x: 236,
    enemies: &[],
    engineers: &[chase(200)],
    shore_dx: Some(crate::cf02::arena::SHORE_STRIKE_X - 236),
    llamas: &[],
    ..crate::cf02::arena::fixture(
        "carl_prog_pierce_engineer",
        "CF02 rules with the Goblin Engineer, reinforced fist held: Carl stands 36 px from him; the committed launch begins on tick 33 and the ram would hit Carl on tick 36. Carl's jab (pressed on tick 23) whiffs and the chained haymaker (pressed on 28) hits on tick 35 (his launch) and cancels the ram, so Carl is not hurt until the Engineer's next attack",
        0x0CA1_0043,
        100,
    )
};

/// CP-Q40, F-12: the haymaker pierces the Engineer's committed launch (the phases
/// `take_hit` treats as committed: his launch and ram). Its control is
/// [`CARL_PROG_PIERCE_ENGINEER_CONTROL`].
pub const CARL_PROG_PIERCE_ENGINEER: PartyFixture = PartyFixture {
    base: &PIERCE_ENGINEER_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[punch_left(23), punch(28)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const PIERCE_ENGINEER_CONTROL_BASE: Fixture = Fixture {
    carl_x: 236,
    enemies: &[],
    engineers: &[chase(200)],
    shore_dx: Some(crate::cf02::arena::SHORE_STRIKE_X - 236),
    llamas: &[],
    ..crate::cf02::arena::fixture(
        "carl_prog_pierce_engineer_control",
        "The control of carl_prog_pierce_engineer without the reinforced fist: the same string lands its haymaker on tick 35 inside the launch but only damages him, and the ram, delayed by the hit's stop, hits Carl on tick 42 (without any press it hits on tick 36)",
        0x0CA1_0048,
        100,
    )
};

/// CP-Q40 control: the same input without the capability lets the ram land.
pub const CARL_PROG_PIERCE_ENGINEER_CONTROL: PartyFixture = PartyFixture {
    base: &PIERCE_ENGINEER_CONTROL_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[punch_left(23), punch(28)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const BOSS_POISE_BASE: Fixture = boss_base(
    "carl_prog_boss_poise",
    "The boss arena, reinforced fist held: Carl walks up and lands a jab and a haymaker on the Juicer during its pound tell; its super armour holds (no flinch, the tell runs on) and the haymaker freezes Carl one tick longer",
    0x0CA1_0044,
    380,
    FULL,
    120,
);

/// CP-Q40, CP-Q42: the Juicer's armour is never pierced; the poise a reinforced
/// punch adds is doubled (observable only in Neutral or Turn, unit-tested in
/// `boss.rs`).
pub const CARL_PROG_BOSS_POISE: PartyFixture = PartyFixture {
    base: &BOSS_POISE_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[run(10, 30, 0, 1.0), punch(22), punch(27)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const SHORE_COMBO_BASE: Fixture = Fixture {
    shore_dx: Some(19),
    ..base(
        "carl_prog_shore_combo",
        "Solo, reinforced fist held: a jab (1 hit) and its haymaker (2 hits) break the fresh rotten shore in one combo; the block falls and crushes the goblin under it, credited to Carl (cp03_shore_kill needs three jabs)",
        0x0CA1_0045,
        &[passive(484)],
        200,
    )
};

/// CP-Q41: jab + haymaker break a fresh support.
pub const CARL_PROG_SHORE_COMBO: PartyFixture = PartyFixture {
    base: &SHORE_COMBO_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[punch(20), punch(25)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const KICK_UNCHANGED_BASE: Fixture = base(
    "carl_prog_kick_unchanged",
    "Solo, reinforced fist held: the push kick launches the goblin at 430 exactly as it does without the fist (damage, hit-stop, launch and the whole event log are the control's)",
    0x0CA1_0046,
    &[passive(430)],
    100,
);

/// CP-Q38: the push kick is not a punch; the capability leaves it alone.
pub const CARL_PROG_KICK_UNCHANGED: PartyFixture = PartyFixture {
    base: &KICK_UNCHANGED_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[kick(10)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DYN_LLAMA_CORE_BASE: Fixture = Fixture {
    llamas: &[LlamaSpawn {
        x: 511,
        ai: EnemyAi::Passive,
    }],
    ..base(
        "carl_prog_dyn_llama_core",
        "Solo, thrown charge held: a passive Bad Llama stands where the stick lands; the core kills it point blank on the detonation tick (a llama_exploded finish), credited to Carl",
        0x0CA1_0047,
        &[],
        330,
    )
};

/// LLAMA-DEATH-WIRE evidence (CP-Q26): the dynamite kill of a llama, the
/// replay the explosive breakup of its LLAMA-02 parts is captured from. The
/// simulation rule is CP-Q26's, unchanged (`thrown.rs` tests it).
pub const CARL_PROG_DYN_LLAMA_CORE: PartyFixture = PartyFixture {
    base: &DYN_LLAMA_CORE_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DYN_THROWN_TRIPLE_BASE: Fixture = base(
    "carl_prog_dyn_thrown_triple",
    "Solo, thrown charge held: three passive goblins stand inside the 20 px core where the stick lands; the one blast defeats all three on its tick, credited to Carl (AUDIENCE-RUN CINE-THROWN-MULTI evidence)",
    0x0CA1_0049,
    &[passive(499), passive(511), passive(523)],
    330,
);

/// AUDIENCE-RUN (AR-T23): a deliberate throw whose one blast defeats three
/// goblins on one tick. The simulation rule is CP-Q20's, unchanged.
pub const CARL_PROG_DYN_THROWN_TRIPLE: PartyFixture = PartyFixture {
    base: &DYN_THROWN_TRIPLE_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DYN_LLAMA_PACK_BASE: Fixture = Fixture {
    llamas: &[LlamaSpawn {
        x: 511,
        ai: EnemyAi::Passive,
    }],
    ..base(
        "carl_prog_dyn_llama_pack",
        "Solo, thrown charge held: four passive goblins and a passive Bad Llama stand inside the 20 px core where the stick lands; the one blast defeats all five on its tick (the llama_exploded finish among them), credited to Carl (AUDIENCE-RUN legendary showcase evidence)",
        0x0CA1_004A,
        &[passive(493), passive(503), passive(519), passive(529)],
        330,
    )
};

/// AUDIENCE-RUN (AR-T24): the run's "best one": a thrown stick that defeats
/// four goblins and a llama in one blast. The simulation rule is CP-Q20's and
/// CP-Q26's, unchanged.
pub const CARL_PROG_DYN_LLAMA_PACK: PartyFixture = PartyFixture {
    base: &DYN_LLAMA_PACK_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[special(4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

/// Every `carl_prog_*` replay fixture, in committed order.
pub const CARL_PROG_FIXTURES: [&PartyFixture; 34] = [
    &CARL_PROG_STORY_EVENT_GRANTS_DYNAMITE,
    &CARL_PROG_GRANT_SURVIVES_RESPAWN,
    &CARL_PROG_JUICER_GRANTS_FIST,
    &CARL_PROG_SPECIAL_LOCKED,
    &CARL_PROG_DYN_ARC_STANDING,
    &CARL_PROG_DYN_ARC_MOVING,
    &CARL_PROG_DYN_ONE_ACTIVE,
    &CARL_PROG_DYN_CORE_KILL,
    &CARL_PROG_DYN_OUTER_LAUNCH,
    &CARL_PROG_DYN_NO_FRIENDLY_FIRE,
    &CARL_PROG_DYN_SWAP_KEEPS_STICK,
    &CARL_PROG_DYN_BOSS,
    &CARL_PROG_DYN_CHAIN_CHARGE,
    &CARL_PROG_DYN_ARMS_TRIP_CHARGE,
    &CARL_PROG_DYN_CHAIN_DEPTH_CAP,
    &CARL_PROG_DYN_CHAIN_DUE_EARLIER,
    &CARL_PROG_DYN_CHAIN_DEFER_OVER_8,
    &CARL_PROG_DYN_BREAKS_SHORE,
    &CARL_PROG_DYN_BOWLING,
    &CARL_PROG_DYN_WALL_SLAM,
    &CARL_PROG_DYN_RACK_STRUT,
    &CARL_PROG_JUICER_DYNAMITE_CLEAR,
    &CARL_PROG_FIST_STAGGER,
    &CARL_PROG_PIERCE_GOBLIN,
    &CARL_PROG_PIERCE_GOBLIN_CONTROL,
    &CARL_PROG_PIERCE_GOBLIN_JAB,
    &CARL_PROG_PIERCE_ENGINEER,
    &CARL_PROG_PIERCE_ENGINEER_CONTROL,
    &CARL_PROG_BOSS_POISE,
    &CARL_PROG_SHORE_COMBO,
    &CARL_PROG_KICK_UNCHANGED,
    &CARL_PROG_DYN_LLAMA_CORE,
    &CARL_PROG_DYN_THROWN_TRIPLE,
    &CARL_PROG_DYN_LLAMA_PACK,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss::BOSS_ENEMY_ID;
    use crate::boss_scene::{BOSS_FIXTURES, load_arena};
    use crate::carl_prog::CarlProgTuning;
    use crate::cf02_fixtures::{find_party_fixture, list_fixtures};
    use crate::combat::CombatEvent;
    use crate::combat_scene::{CombatState, FixtureSim};
    use crate::corridor::SceneSim;
    use crate::demo::{DEMO_RULES, autopilot, fixture_for_beat, load_hall};
    use crate::env_kit::EnvKit;
    use crate::hud::{
        AbilityRow, Chip, HudConfig, HudRuntime, LabelId, LogicalAction, NoticeView, SpecialPhase,
        SpecialSlot,
    };
    use crate::party_scene::PartyFixtureSim;
    use crate::story_unlock::{
        CapabilityId, FIST_REINFORCED, PRIMARY_RANGED, SPECIAL_THROWN_CHARGE, StoryTable,
    };
    use std::path::{Path, PathBuf};

    const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn kit_of(fixture: &PartyFixture) -> EnvKit {
        if fixture.base.boss.is_some() {
            load_arena(&root()).unwrap()
        } else if fixture.base.room == crate::combat_scene::FixtureRoom::Arena {
            crate::cf02::arena::load_arena(&root()).unwrap()
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

    /// The committed final hashes of the `carl_prog_*` fixtures.
    const GOLDENS: [(&str, u64); 34] = [
        (
            "carl_prog_story_event_grants_dynamite",
            0x7594_ba64_4fd2_dbaf,
        ),
        ("carl_prog_grant_survives_respawn", 0x8eeb_9036_87de_c95d),
        ("carl_prog_juicer_grants_fist", 0xddd6_81ab_67f5_26ac),
        ("carl_prog_special_locked", 0xc955_52f3_0a6a_9051),
        ("carl_prog_dyn_arc_standing", 0x5f39_bdb0_1c95_8237),
        ("carl_prog_dyn_arc_moving", 0x9b12_5289_22e4_8d00),
        ("carl_prog_dyn_one_active", 0x0d20_035e_4ed7_67f7),
        ("carl_prog_dyn_core_kill", 0x7398_427c_74e7_94d9),
        ("carl_prog_dyn_outer_launch", 0x9cb6_2929_74d4_2de5),
        ("carl_prog_dyn_no_friendly_fire", 0xada1_8a6f_91e9_e012),
        ("carl_prog_dyn_swap_keeps_stick", 0x1325_56e5_5531_094c),
        ("carl_prog_dyn_boss", 0x90b2_b0d3_e323_2388),
        ("carl_prog_dyn_chain_charge", 0x2c8d_2496_de05_117e),
        ("carl_prog_dyn_arms_trip_charge", 0xaa2c_b16d_4e4e_f9d7),
        ("carl_prog_dyn_chain_depth_cap", 0x0443_83ce_ef4c_2af7),
        ("carl_prog_dyn_chain_due_earlier", 0xb4f3_086c_2445_db89),
        ("carl_prog_dyn_chain_defer_over_8", 0xa7a0_a485_638c_c59a),
        ("carl_prog_dyn_breaks_shore", 0xbdd2_997f_3974_e813),
        ("carl_prog_dyn_bowling", 0x9edd_d9cf_fd1d_e939),
        ("carl_prog_dyn_wall_slam", 0xe7f8_1f9b_f589_ce3c),
        ("carl_prog_dyn_rack_strut", 0x4ea9_be59_43b0_9a2e),
        ("carl_prog_juicer_dynamite_clear", 0xaa48_5ad2_e95f_cbe8),
        ("carl_prog_fist_stagger", 0xadb4_14af_d625_1b47),
        ("carl_prog_pierce_goblin", 0x6090_fad0_7bfc_6ac7),
        ("carl_prog_pierce_goblin_control", 0x840a_f5f1_1a9e_5335),
        ("carl_prog_pierce_goblin_jab", 0x04ab_e489_69b7_ccd9),
        ("carl_prog_pierce_engineer", 0x2af5_8e02_8da6_a723),
        ("carl_prog_pierce_engineer_control", 0x6d91_cbe8_4a15_208a),
        ("carl_prog_boss_poise", 0x7ce8_2d1e_f101_63c1),
        ("carl_prog_shore_combo", 0xc155_9bef_ceb0_67bf),
        ("carl_prog_kick_unchanged", 0x0755_27d1_bab1_0490),
        ("carl_prog_dyn_llama_core", 0xa00c_a20c_6dcf_ece8),
        ("carl_prog_dyn_thrown_triple", 0xcee2_8f1e_e914_b7aa),
        ("carl_prog_dyn_llama_pack", 0xf46c_22a6_4d10_893a),
    ];

    #[test]
    fn the_registry_is_listed_and_every_fixture_matches_its_golden_and_replays_identically() {
        assert_eq!(GOLDENS.len(), CARL_PROG_FIXTURES.len());
        let listed = list_fixtures("carl_prog_");
        for (fixture, (name, expected)) in CARL_PROG_FIXTURES.iter().zip(GOLDENS) {
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
            CARL_PROG_FIXTURES.len(),
            "nothing else in the namespace"
        );
        let mut hashes: Vec<u64> = GOLDENS.iter().map(|g| g.1).collect();
        hashes.sort_unstable();
        hashes.dedup();
        assert_eq!(hashes.len(), GOLDENS.len(), "every fixture is distinct");
    }

    #[test]
    fn the_committed_replays_are_the_replay() {
        for fixture in CARL_PROG_FIXTURES {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("docs/evidence/carl-prog-01/replays")
                .join(format!("headless_{}.txt", fixture.name()));
            let committed = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let committed: Vec<&str> = committed.lines().collect();
            assert_eq!(committed, lines(&replay(fixture)), "{}", fixture.name());
        }
    }

    #[test]
    fn no_other_registry_holds_a_carl_prog_name_and_the_boss_fixtures_stay_unpinned() {
        for name in crate::cf02_fixtures::list_fixtures("") {
            // CORRAL-B wire (KC12): the one corral fixture that holds the fist.
            if !name.starts_with("carl_prog_")
                && !name.starts_with("donut_prog_")
                && name != "corral_rush_pierce_haymaker"
            {
                assert!(
                    story_setup(name).is_inert(),
                    "{name} is a legacy fixture: no pin, no trigger"
                );
            }
        }
        for fixture in BOSS_FIXTURES {
            assert!(story_setup(fixture.name()).is_inert());
        }
    }

    /// CP-Q01 and CP-Q11 (simulation and events only; the notice and the socket
    /// are HUD assertions of CP-B2): the stub zone grants once.
    #[test]
    fn carl_prog_story_event_grants_dynamite() {
        let sim = named("carl_prog_story_event_grants_dynamite");
        let granted = grants(&sim);
        assert_eq!(
            granted.len(),
            1,
            "exactly one CapabilityGranted: {granted:?}"
        );
        let (tick, hero, capability) = granted[0];
        assert_eq!((hero, capability), (0, "special_thrown_charge"));
        // The first tick Carl's pivot is inside the half-open zone.
        let mut probe = PartyFixtureSim::new(
            &kit_of(&CARL_PROG_STORY_EVENT_GRANTS_DYNAMITE),
            360,
            &CARL_PROG_STORY_EVENT_GRANTS_DYNAMITE,
        )
        .unwrap();
        let mut first_inside = None;
        for _ in 0..CARL_PROG_STORY_EVENT_GRANTS_DYNAMITE.base.ticks {
            probe.step_scripted();
            let x = probe.scene.world.hero.x;
            if first_inside.is_none() && (FIND_STUB_ZONE_X.0..FIND_STUB_ZONE_X.1).contains(&x) {
                first_inside = Some(probe.scene.world.tick);
            }
        }
        assert_eq!(first_inside, Some(tick), "granted on the first tick inside");
        assert!(sim.combat.story_holds(SPECIAL_THROWN_CHARGE));
        assert!(!sim.combat.story_holds(FIST_REINFORCED));
        // The data row carries the notice text (13 characters at most).
        let notice = StoryTable::active()
            .notice("special_thrown_charge")
            .unwrap();
        assert_eq!(notice.text, "GOBLIN BOOM");
        // The receipt names the trigger that granted it.
        let receipts = sim.combat.story_ledger().unwrap().receipts();
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].source, "carl_prog_find_zone_stub");
    }

    /// CP-Q07: a solo swap and a defeat retry keep the grant, no second grant is
    /// logged, and a fixture restart restores the declared (empty) start.
    #[test]
    fn carl_prog_grant_survives_respawn() {
        let mut sim = named("carl_prog_grant_survives_respawn");
        let granted = grants(&sim);
        assert_eq!(
            granted.len(),
            1,
            "one grant, after the retry too: {granted:?}"
        );
        let restart_at = sim
            .combat
            .events
            .iter()
            .position(|e| matches!(e, CombatEvent::Restart { .. }))
            .expect("the goblin defeated Carl and the party retried");
        let grant_at = sim
            .combat
            .events
            .iter()
            .position(|e| matches!(e, CombatEvent::CapabilityGranted { .. }))
            .unwrap();
        assert!(grant_at < restart_at, "the grant came before the defeat");
        assert!(sim.combat.deaths >= 1);
        assert!(
            sim.combat.story_holds(SPECIAL_THROWN_CHARGE),
            "the retry keeps the grant"
        );
        // Receipt ticks are attempt-local and keep the first attempt's tick.
        let receipts = sim.combat.story_ledger().unwrap().receipts();
        assert_eq!((receipts.len(), receipts[0].tick), (1, granted[0].0));
        // A fixture restart (R / Select) restores the declared start: nothing.
        sim.combat.restart(&mut sim.scene);
        assert!(!sim.combat.story_holds(SPECIAL_THROWN_CHARGE));
        assert!(sim.combat.story_ledger().unwrap().is_empty());
        // The zone is armed again: walking in grants once more.
        sim.party.restart();
        sim.run_scripted(120);
        assert!(sim.combat.story_holds(SPECIAL_THROWN_CHARGE));
    }

    /// CP-Q07: a solo swap neither grants nor removes anything and the ledger
    /// hash does not move with it.
    #[test]
    fn a_solo_swap_leaves_the_ledger_alone() {
        let mut sim = PartyFixtureSim::new(
            &kit_of(&CARL_PROG_STORY_EVENT_GRANTS_DYNAMITE),
            360,
            &CARL_PROG_STORY_EVENT_GRANTS_DYNAMITE,
        )
        .unwrap();
        sim.run_scripted(100);
        assert!(sim.combat.story_holds(SPECIAL_THROWN_CHARGE));
        let mix = |sim: &PartyFixtureSim| {
            let mut out = Vec::new();
            sim.combat
                .story_ledger()
                .unwrap()
                .hash_into(&mut |v| out.push(v));
            out
        };
        let before = mix(&sim);
        let mut slots = [crate::input::RawInput::default(); 2];
        slots[0].swap = true;
        sim.step_slots(slots);
        sim.run_scripted(0);
        for _ in 0..30 {
            sim.step_slots([crate::input::RawInput::default(); 2]);
        }
        assert_eq!(sim.party.controlled_hero(), crate::party::HeroId::Donut);
        assert!(sim.combat.story_holds(SPECIAL_THROWN_CHARGE));
        assert_eq!(mix(&sim), before);
        assert_eq!(grants(&sim).len(), 1);
    }

    /// CP-Q07 on the checkpoint respawn of the demo hall: a grant survives it, a
    /// fixture restart removes it.
    #[test]
    fn a_checkpoint_respawn_keeps_a_grant_and_a_fixture_restart_removes_it() {
        let kit = load_hall(&root()).unwrap();
        let fixture = fixture_for_beat(1);
        let scene = SceneSim::new_at(&kit, 360, fixture.carl_x).unwrap();
        let combat = CombatState::new(fixture, &kit, &scene)
            .unwrap()
            .with_demo(&DEMO_RULES)
            .with_story(StorySetup::NONE.with_hold("fist_reinforced"));
        let mut run = FixtureSim { scene, combat };
        assert!(
            run.combat.story_holds(FIST_REINFORCED),
            "a pin is held from tick 0"
        );
        run.combat.respawn(&mut run.scene);
        assert!(
            run.combat.story_holds(FIST_REINFORCED),
            "the respawn keeps it"
        );
        assert!(
            run.combat
                .events
                .iter()
                .all(|e| !matches!(e, CombatEvent::CapabilityGranted { .. })),
            "a pin is a declared start, never a grant"
        );
        // A grant earned in play survives the respawn; the fixture restart
        // drops it and keeps the pin.
        let mut earned = CombatState::new(fixture, &kit, &run.scene)
            .unwrap()
            .with_demo(&DEMO_RULES)
            .with_story(StorySetup::NONE.with_trigger("carl_prog_find_zone_stub"));
        let mut scene = run.scene.clone();
        scene.world.hero.x = 610.0;
        earned.step(&mut scene, crate::input::RawInput::default());
        assert!(earned.story_holds(SPECIAL_THROWN_CHARGE));
        earned.respawn(&mut scene);
        assert!(
            earned.story_holds(SPECIAL_THROWN_CHARGE),
            "the respawn keeps a grant"
        );
        earned.restart(&mut scene);
        assert!(
            !earned.story_holds(SPECIAL_THROWN_CHARGE),
            "R restores the empty start"
        );
        run.combat.restart(&mut run.scene);
        assert!(
            run.combat.story_holds(FIST_REINFORCED),
            "R restores the pin"
        );
    }

    /// CP-Q02: under the boss pin the boss defeat emits exactly one
    /// `CapabilityGranted { fist_reinforced }` on the defeat tick.
    #[test]
    fn carl_prog_juicer_grants_fist() {
        let sim = named("carl_prog_juicer_grants_fist");
        let granted = grants(&sim);
        assert_eq!(granted.len(), 1, "{granted:?}");
        let defeat = sim
            .combat
            .events
            .iter()
            .find_map(|e| match *e {
                CombatEvent::EnemyDefeated { tick, target, .. } if target == BOSS_ENEMY_ID => {
                    Some(tick)
                }
                _ => None,
            })
            .expect("the boss fell");
        assert_eq!(
            granted[0],
            (defeat, 0, "fist_reinforced"),
            "on the defeat tick"
        );
        // The pin holds the thrown charge from tick 0 and emitted nothing.
        assert!(sim.combat.story_holds(SPECIAL_THROWN_CHARGE));
        assert!(sim.combat.story_holds(FIST_REINFORCED));
        let receipts = sim.combat.story_ledger().unwrap().receipts();
        assert_eq!(receipts.len(), 1, "a pin has no receipt");
        assert_eq!(receipts[0].source, "brute_boss_f01_defeated");
    }

    /// CP-Q45: the biscuit is a read-through. With the ledger live around it
    /// (triggers armed, nothing held), `--scene boss_f01`'s biscuit and a `q10_*`
    /// run hash and log exactly as without, `holds` follows `DonutMana`, and the
    /// ledger never stores, hashes, receipts or emits it.
    #[test]
    fn carl_prog_biscuit_readthrough_hash() {
        let armed = || {
            // The Juicer trigger and the placeholder zone armed, no pin: the
            // ledger exists and is empty. The fixtures below neither defeat the
            // boss nor walk Carl into the zone.
            StorySetup::NONE
                .with_trigger("brute_boss_f01_defeated")
                .with_trigger("carl_prog_find_zone_stub")
        };
        for name in [
            "q10_unlock_persists",
            "q10_mana_regen",
            "boss01_pound_punish",
            "boss01_charge_jump",
        ] {
            let fixture = find_party_fixture(name).unwrap();
            let kit = kit_of(fixture);
            let run = |story: Option<StorySetup>| {
                let mut combat = CombatState::new(
                    fixture.base,
                    &kit,
                    &SceneSim::new_at(&kit, 360, fixture.base.carl_x).unwrap(),
                )
                .unwrap()
                .with_donut(fixture.donut);
                if let Some(story) = story {
                    combat = combat.with_story(story);
                }
                let scene = SceneSim::new_at(&kit, 360, fixture.base.carl_x).unwrap();
                let mut sim = PartyFixtureSim::from_parts(scene, combat, fixture);
                sim.run_scripted(fixture.base.ticks);
                sim
            };
            let plain = run(None);
            let with_ledger = run(Some(armed()));
            assert_eq!(
                plain.state_hash(),
                with_ledger.state_hash(),
                "{name}: same hash"
            );
            assert_eq!(lines(&plain), lines(&with_ledger), "{name}: same log");
            assert!(
                with_ledger
                    .combat
                    .events
                    .iter()
                    .all(|e| !matches!(e, CombatEvent::CapabilityGranted { .. })),
                "{name}: the biscuit is never a CapabilityGranted"
            );
            let ledger = with_ledger.combat.story_ledger().expect("a live ledger");
            let unlocked = with_ledger.combat.donut_mana_view().is_some();
            assert_eq!(
                with_ledger.combat.story_holds(PRIMARY_RANGED),
                unlocked,
                "{name}: the read-through is DonutMana::unlocked() and nothing else"
            );
            assert!(
                ledger.is_empty() && ledger.receipts().is_empty(),
                "{name}: never stored"
            );
            // The ledger would mix nothing even if asked.
            let mut mixed = false;
            ledger.hash_into(&mut |_| mixed = true);
            assert!(!mixed, "{name}: nothing hashed");
        }
        // The Q10 unlock really flips the read-through (the fixture unlocks the
        // biscuit at the guild zone).
        let sim = named("q10_unlock_persists");
        assert!(sim.combat.donut_mana_view().is_some());
        assert!(
            !sim.combat
                .story_holds(CapabilityId::new("special_thrown_charge"))
        );
    }

    /// `--carl-prog` on the demo hall and the CF02 microarena reproduces the
    /// unflagged run exactly: no pin, no trigger is reachable there. The opening
    /// route corridor runs no combat state at all.
    #[test]
    fn carl_prog_flag_parity() {
        let flags = CarlProgFlags {
            prog: true,
            ..CarlProgFlags::OFF
        };
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
        let flagged = demo(Some(flags.story_setup(SceneKind::Demo)));
        assert_eq!(plain.state_hash(), flagged.state_hash());
        assert_eq!(plain.combat.events, flagged.combat.events);
        assert!(flagged.combat.story_ledger().is_none());
        // CF02 microarena, the direct plan.
        let arena = crate::cf02::arena::load_arena(&root()).unwrap();
        let cf02 = |story: Option<StorySetup>| {
            let fixture = crate::cf02::arena::fixture_for_run(0, false, crate::engineer::active());
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
        let flagged = cf02(Some(flags.story_setup(SceneKind::CfArena)));
        assert_eq!(plain.state_hash(), flagged.state_hash());
        assert_eq!(plain.combat.events, flagged.combat.events);
        // The debug flags are different: they hold a capability from tick 0 and
        // so change the hash (the opt-in is visible, never silent).
        let dynamite = CarlProgFlags {
            dynamite: true,
            ..CarlProgFlags::OFF
        };
        let held = demo(Some(dynamite.story_setup(SceneKind::Demo)));
        assert_ne!(held.state_hash(), plain_demo_hash());
        assert!(held.combat.story_holds(SPECIAL_THROWN_CHARGE));
        assert!(
            held.combat
                .events
                .iter()
                .all(|e| !matches!(e, CombatEvent::CapabilityGranted { .. })),
            "a start flag is a declared start state, not a grant"
        );

        fn plain_demo_hash() -> u64 {
            let hall = load_hall(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
            let fixture = fixture_for_beat(0);
            let scene = SceneSim::new_at(&hall, 360, fixture.carl_x).unwrap();
            let combat = CombatState::new(fixture, &hall, &scene)
                .unwrap()
                .with_demo(&DEMO_RULES);
            let mut run = FixtureSim { scene, combat };
            for _ in 0..1500 {
                let raw = autopilot(&run.combat, &run.scene);
                run.step(raw);
            }
            run.state_hash()
        }
    }

    fn kinds(sim: &PartyFixtureSim, kind: &str) -> Vec<String> {
        sim.combat
            .events
            .iter()
            .map(ToString::to_string)
            .filter(|l| l.contains(&format!("kind={kind}")))
            .collect()
    }

    /// CP-B1: what each registered dynamite fixture shows, in words (the
    /// goldens pin the bytes).
    #[test]
    fn the_dynamite_fixtures_show_what_their_summaries_say() {
        // Locked: forty presses, nothing logged, nothing mixed by the thrower.
        let locked = named("carl_prog_special_locked");
        assert!(locked.combat.events.is_empty());
        assert!(locked.combat.thrower().is_idle() && locked.combat.charges.is_empty());
        // The moving throw lands farther than the standing one.
        let landing = |name: &str| -> i32 {
            let sim = named(name);
            let line = &kinds(&sim, "charge_landed")[0];
            line.split("x=")
                .nth(1)
                .unwrap()
                .split(' ')
                .next()
                .unwrap()
                .parse()
                .unwrap()
        };
        assert_eq!(landing("carl_prog_dyn_arc_standing"), 511);
        assert!(landing("carl_prog_dyn_arc_moving") > 560);
        // Core kill: point blank, credited to Carl; outer launch: alive, 1 HP.
        let core = named("carl_prog_dyn_core_kill");
        assert!(kinds(&core, "enemy_defeated")[0].contains("point_blank"));
        let outer = named("carl_prog_dyn_outer_launch");
        assert_eq!(outer.combat.enemies[0].health, 1);
        assert!(kinds(&outer, "enemy_defeated").is_empty());
        // One active: five busy rejections... then a second throw on detonation + 420.
        let one = named("carl_prog_dyn_one_active");
        assert_eq!(kinds(&one, "special_started").len(), 2);
        assert_eq!(kinds(&one, "special_rejected").len(), 7);
        // The swap changes nothing about the blast or its credit.
        let swap = named("carl_prog_dyn_swap_keeps_stick");
        assert_eq!(swap.party.controlled_hero(), crate::party::HeroId::Donut);
        assert_eq!(
            kinds(&swap, "enemy_defeated"),
            kinds(&core, "enemy_defeated"),
            "the same kill, the same credit"
        );
        // Boss: 4 damage on tick 195.
        let boss = named("carl_prog_dyn_boss");
        assert_eq!(
            kinds(&boss, "boss_hurt"),
            ["COMBAT_EVENT tick=195 kind=boss_hurt source=0 damage=4 hp_left=6"]
        );
    }

    /// CP-Q24 in 2P: Donut stands inside the 20 px core when the stick blows
    /// and nothing happens to her.
    #[test]
    fn donut_stands_in_the_blast_and_is_not_touched() {
        let fixture = &CARL_PROG_DYN_NO_FRIENDLY_FIRE;
        let mut sim = PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        for _ in 0..194 {
            sim.step_scripted();
        }
        let stick = sim.combat.thrower().stick().expect("the stick burns").x;
        let donut = sim.scene.world.donut_body.x;
        assert!(
            (donut - stick).abs() < 20.0,
            "Donut stands in the core: donut {donut} stick {stick}"
        );
        sim.step_scripted();
        assert_eq!(kinds(&sim, "blast").len(), 1);
        assert_eq!(
            sim.scene.world.donut_body.health,
            sim.scene.world.hero.health
        );
        assert!(kinds(&sim, "hero_hurt").is_empty() && kinds(&sim, "hero_defeated").is_empty());
        assert_eq!(sim.combat.donut.hurt_ticks, 0);
    }

    // ----------------------------------------------------------- CP-B2: HUD --

    /// The socket of Carl's plate and the notice after every tick of `ticks` ticks of
    /// `fixture`, as the scene's `step_hud` feeds a [`HudRuntime`]. Index 0 is the
    /// state before the first step.
    fn socket_trace(
        fixture: &'static PartyFixture,
        ticks: u64,
    ) -> (PartyFixtureSim, Vec<(SpecialSlot, Option<NoticeView>)>) {
        let mut sim = PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        let mut hud = HudRuntime::new(HudConfig::default());
        hud.set_melee_live(true);
        hud.set_special_label(&CarlProgTuning::active().cpq32_socket_label);
        let read = |hud: &HudRuntime| match hud.snapshot().plates[0].abilities {
            AbilityRow::Chips { special, .. } => (special, hud.snapshot().notice),
            other => panic!("Carl's plate shows chips: {other:?}"),
        };
        let mut trace = vec![read(&hud)];
        for _ in 0..ticks {
            sim.step_scripted();
            hud.set_special(SpecialPhase::of(&sim.combat));
            hud.step(
                sim.scene.world.hero.health,
                sim.scene.world.donut.health,
                &sim.combat.events,
                sim.combat.dropped_events,
            );
            trace.push(read(&hud));
        }
        (sim, trace)
    }

    fn equipped(muted: bool) -> SpecialSlot {
        SpecialSlot::Equipped(Chip {
            action: LogicalAction::Special,
            label: LabelId::Boom,
            muted,
        })
    }

    /// CP-Q11: the socket turns from padlock to equipped, and the grant notice
    /// appears, on the grant tick itself.
    #[test]
    fn carl_prog_hud_grant_socket() {
        let fixture = &CARL_PROG_STORY_EVENT_GRANTS_DYNAMITE;
        let (sim, trace) = socket_trace(fixture, fixture.base.ticks);
        let tick = grants(&sim)[0].0 as usize;
        for (before, (slot, notice)) in trace[..tick].iter().enumerate() {
            assert_eq!(*slot, SpecialSlot::Locked, "tick {before}");
            assert_eq!(*notice, None, "no notice on tick {before}");
        }
        assert_eq!(trace[tick].0, equipped(false), "equipped on the grant tick");
        let notice = trace[tick].1.expect("the notice starts on the grant tick");
        assert_eq!(
            (notice.headline, notice.age_ticks),
            (LabelId::GoblinBoom, 0)
        );
        // The label is the data row's text, so the two cannot drift.
        let row = StoryTable::active()
            .notice("special_thrown_charge")
            .unwrap();
        assert_eq!(
            LabelId::for_notice_text(&row.text),
            Some(LabelId::GoblinBoom)
        );
        assert_eq!(LabelId::GoblinBoom.text(), row.text);
        // It stays equipped for the rest of the run, ready (no stick was thrown).
        assert!(
            trace[tick..]
                .iter()
                .all(|(slot, _)| *slot == equipped(false))
        );
    }

    /// CP-Q11: before any grant the socket stays a padlock, whatever the presses.
    #[test]
    fn carl_prog_hud_socket_locked() {
        let fixture = &CARL_PROG_SPECIAL_LOCKED;
        let (_, trace) = socket_trace(fixture, fixture.base.ticks);
        assert!(
            trace
                .iter()
                .all(|(slot, notice)| *slot == SpecialSlot::Locked && notice.is_none())
        );
    }

    /// CP-Q32: ready, live (the press to the detonation) and recharging (the
    /// detonation to the ready tick), exact on both edges; live and recharging are
    /// muted.
    #[test]
    fn carl_prog_hud_socket_ready_live_and_recharging() {
        let fixture = &CARL_PROG_DYN_ARC_STANDING;
        let (sim, trace) = socket_trace(fixture, 700);
        let tick_of = |kind: &str| -> usize {
            sim.combat
                .events
                .iter()
                .find(|e| e.to_string().contains(&format!("kind={kind} ")))
                .map(|e| e.to_string())
                .and_then(|l| l.split("tick=").nth(1).map(str::to_owned))
                .and_then(|l| l.split(' ').next().map(str::to_owned))
                .and_then(|n| n.parse().ok())
                .unwrap_or_else(|| panic!("no {kind} event"))
        };
        let (press, blast, ready) = (
            tick_of("special_started"),
            tick_of("blast"),
            tick_of("special_ready"),
        );
        assert_eq!((press, blast, ready), (4, 195, 615));
        // Held from tick 0 (a declared start): equipped, ready, no notice (index 0 is
        // the HUD before its first step).
        assert!(
            trace[1..press]
                .iter()
                .all(|(s, n)| *s == equipped(false) && n.is_none())
        );
        // Live from the press tick to the tick before the blast.
        assert!(
            trace[press..blast]
                .iter()
                .all(|(s, _)| *s == equipped(true))
        );
        assert_eq!(trace[press - 1].0, equipped(false), "last ready tick");
        // Recharging from the blast tick to the tick before the ready tick.
        assert!(
            trace[blast..ready]
                .iter()
                .all(|(s, _)| *s == equipped(true))
        );
        // Ready again exactly on the ready tick.
        assert_eq!(trace[ready - 1].0, equipped(true), "last muted tick");
        assert!(trace[ready..].iter().all(|(s, _)| *s == equipped(false)));
        // The phases themselves, on the same edges.
        assert_eq!(SpecialPhase::Live.slot(LabelId::Boom), equipped(true));
        assert_eq!(SpecialPhase::Recharging.slot(LabelId::Boom), equipped(true));
        assert_eq!(SpecialPhase::Ready.slot(LabelId::Boom), equipped(false));
        assert_eq!(
            SpecialPhase::Locked.slot(LabelId::Boom),
            SpecialSlot::Locked
        );
    }

    /// CP-Q32: the data label drives the socket; a text with no HUD label keeps `BOOM`.
    #[test]
    fn the_socket_label_comes_from_the_data_row() {
        let label = &CarlProgTuning::active().cpq32_socket_label;
        assert_eq!(LabelId::for_socket_text(label), Some(LabelId::Boom));
        assert_eq!(LabelId::Boom.text(), label.as_str());
        assert_eq!(LabelId::for_socket_text("KABOOM"), None);
        let mut hud = HudRuntime::new(HudConfig::default());
        hud.set_special(SpecialPhase::Ready);
        hud.set_special_label("KABOOM");
        let AbilityRow::NoActions { special } = hud.snapshot().plates[0].abilities else {
            panic!("melee is not live: no chips, the socket stays");
        };
        assert_eq!(special, equipped(false), "an unknown label keeps BOOM");
    }

    // ------------------------------------------------------ CP-C: interactions --

    /// `(tick, charge)` of every `blast` event.
    fn blasts(sim: &PartyFixtureSim) -> Vec<(u64, u32)> {
        sim.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Blast { tick, charge } => Some((tick, charge)),
                _ => None,
            })
            .collect()
    }

    /// `(tick, charge, source, due, depth)` of every `charge_chained` event.
    fn chained(sim: &PartyFixtureSim) -> Vec<(u64, u32, u32, u64, u8)> {
        sim.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::ChargeChained {
                    tick,
                    charge,
                    source,
                    due,
                    depth,
                } => Some((tick, charge, source, due, depth)),
                _ => None,
            })
            .collect()
    }

    /// CP-Q34: the stick (the reserved slot, index 1 here) blows on tick 195, the
    /// charge it reaches is lit for exactly 8 ticks later and not a tick before,
    /// and it inherits the thrower's credit.
    #[test]
    fn carl_prog_dyn_chain_charge() {
        let sim = named("carl_prog_dyn_chain_charge");
        assert_eq!(
            blasts(&sim),
            [(195, 1), (203, 0)],
            "195 + 8, not 202 or 204"
        );
        assert_eq!(chained(&sim), [(195, 0, 1, 203, 1)]);
        // The chained charge hit the goblin beside it, credited to Carl by inheritance.
        assert_eq!(
            sim.combat.charges[0].owner,
            Some(0),
            "the empty owner is filled"
        );
        assert_eq!(sim.combat.charges[0].chain_depth, 1);
        let kills = kinds(&sim, "enemy_defeated");
        assert_eq!(kills.len(), 1);
        assert!(kills[0].contains("tick=203") && kills[0].contains("charge=0"));
        assert!(kills[0].contains("credit_player=0"), "{}", kills[0]);
    }

    /// CP-Q34 (unlit rule): an unarmed trip charge (a stored tick of 0, Carl never
    /// passes its column) counts as armed: the chain sets it to now + 8, it is never
    /// read as already due, and it blows on tick 203 and not on 195 or 202.
    #[test]
    fn carl_prog_dyn_arms_trip_charge() {
        let fixture = &CARL_PROG_DYN_ARMS_TRIP_CHARGE;
        let mut sim = PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        for _ in 0..194 {
            sim.step_scripted();
        }
        let before = sim.combat.charges[0];
        assert!(
            !before.lit() && before.armed_tick.is_none(),
            "unlit on tick 194"
        );
        assert_eq!(before.detonate_tick, 0, "a stored tick of 0");
        sim.step_scripted(); // tick 195: the stick blows
        let armed = sim.combat.charges[0];
        assert_eq!((armed.armed_tick, armed.detonate_tick), (Some(195), 203));
        assert_eq!((armed.chain_depth, armed.owner), (1, Some(0)));
        while sim.scene.world.tick < fixture.base.ticks {
            sim.step_scripted();
        }
        assert_eq!(blasts(&sim), [(195, 1), (203, 0)]);
        assert_eq!(chained(&sim), [(195, 0, 1, 203, 1)]);
    }

    /// CP-Q34: depth 4 at most. The stick is depth 0; the charges it reaches are
    /// depths 1 to 4 on ticks 203, 211, 219 and 227, a depth-4 blast chains nothing,
    /// and no charge is chained twice (the neighbour behind each one already blew).
    #[test]
    fn carl_prog_dyn_chain_depth_cap() {
        let sim = named("carl_prog_dyn_chain_depth_cap");
        assert_eq!(
            blasts(&sim),
            [(195, 5), (203, 0), (211, 1), (219, 2), (227, 3)],
            "charge 4 never blows"
        );
        assert_eq!(
            chained(&sim),
            [
                (195, 0, 5, 203, 1),
                (203, 1, 0, 211, 2),
                (211, 2, 1, 219, 3),
                (219, 3, 2, 227, 4),
            ],
            "each charge is chained once, in order; the depth-4 blast reaches charge 4 \
             at 18 px and chains nothing"
        );
        assert!(!sim.combat.charges[4].detonated);
        assert_eq!(sim.combat.charges[4].chain_depth, 0);
        assert_eq!(sim.combat.charges[3].chain_depth, 4);
        // Dedupe: no charge was chained twice, though each blast reaches both neighbours.
        let mut targets: Vec<u32> = chained(&sim).iter().map(|c| c.1).collect();
        targets.sort_unstable();
        targets.dedup();
        assert_eq!(targets.len(), 4);
    }

    /// CP-Q34: "due earlier" means lit and `now < tick <= now + 8`: due 198 and
    /// due 203 (the last tick in) are left alone, due 204 (the first tick out) and
    /// due 400 are pulled to 203.
    #[test]
    fn carl_prog_dyn_chain_due_earlier() {
        let sim = named("carl_prog_dyn_chain_due_earlier");
        assert_eq!(
            chained(&sim),
            [
                (195, 2, 5, 203, 1),
                (195, 3, 5, 203, 1),
                (198, 4, 0, 206, 2),
            ],
            "charges 0 (due 198) and 1 (due 203) are left alone; charge 0's blast chains charge 4"
        );
        assert_eq!(
            blasts(&sim),
            [(195, 5), (198, 0), (203, 1), (203, 2), (203, 3), (206, 4)]
        );
        // Left alone is a scheduling rule only: they were reached, so they are depth 1
        // (B-1), and charge 0's own blast on tick 198 chains charge 4 at depth 2.
        for (i, depth) in [(0, 1), (1, 1), (2, 1), (3, 1), (4, 2)] {
            assert_eq!(sim.combat.charges[i].chain_depth, depth, "charge {i}");
        }
        // The first initiator keeps the credit: left-alone charges are still filled.
        assert!(sim.combat.charges[..5].iter().all(|c| c.owner == Some(0)));
    }

    /// CP-Q34: at most 8 detonations on one tick; the ninth, in index order, goes
    /// to the next tick and keeps its depth and owner; each charge blows once.
    #[test]
    fn carl_prog_dyn_chain_defer_over_8() {
        let sim = named("carl_prog_dyn_chain_defer_over_8");
        let blasts = blasts(&sim);
        let mut expected = vec![(195, 10)];
        expected.extend((0..8).map(|c| (203, c)));
        expected.extend([(204, 8), (204, 9)]);
        assert_eq!(blasts, expected, "eight on 203, the other two on 204");
        for tick in 190..210 {
            assert!(
                blasts.iter().filter(|b| b.0 == tick).count() <= 8,
                "tick {tick}"
            );
        }
        let mut charges: Vec<u32> = blasts.iter().map(|b| b.1).collect();
        charges.sort_unstable();
        assert_eq!(
            charges,
            (0..11).collect::<Vec<u32>>(),
            "each charge blows once"
        );
        // Pulled by the stick, then deferred: depth 1, Carl's credit.
        let pulled = &sim.combat.charges[8];
        assert_eq!((pulled.detonate_tick, pulled.chain_depth), (204, 1));
        assert_eq!(pulled.owner, Some(0));
        // Due this tick on its own, reached by charge 0's blast (depth 1) on tick 203
        // before it was stepped, then deferred by the cap: depth 2 and the credit
        // still arrive (B-1). It has no `charge_chained` event: its tick is unchanged.
        let late = &sim.combat.charges[9];
        assert_eq!((late.detonate_tick, late.chain_depth), (204, 2));
        assert_eq!(late.owner, Some(0));
        assert_eq!(
            chained(&sim).len(),
            9,
            "nine were scheduled on tick 195, none later"
        );
        assert!(chained(&sim).iter().all(|c| c.1 != 9));
    }

    /// CP-Q35: the blast breaks the untouched shore outright, credited to Carl; the
    /// fall, the crush and the credit are the CP-03 rules.
    #[test]
    fn carl_prog_dyn_breaks_shore() {
        let sim = named("carl_prog_dyn_breaks_shore");
        assert_eq!(
            kinds(&sim, "shore_broken"),
            ["COMBAT_EVENT tick=195 kind=shore_broken player=0"]
        );
        assert_eq!(kinds(&sim, "shore_landed").len(), 1);
        let kills = kinds(&sim, "enemy_defeated");
        assert_eq!(kills.len(), 1);
        assert!(kills[0].contains("cause=shore_block") && kills[0].contains("credit_player=0"));
        // No melee hit was thrown: the shore had taken none.
        assert!(kinds(&sim, "shore_hit").is_empty());
    }

    /// CP-Q35: a shore out of the 64 px reach stands.
    #[test]
    fn a_shore_out_of_reach_stands_and_a_qa_blast_never_breaks_it() {
        static FAR: Fixture = Fixture {
            shore_dx: Some(260),
            ..base(
                "carl_prog_dyn_test_far_shore",
                "test only",
                0x0CA1_00F0,
                &[],
                260,
            )
        };
        static FAR_FIXTURE: PartyFixture = PartyFixture {
            base: &FAR,
            two_player: false,
            tuning: PROVISIONAL_TUNING,
            script: &[special(4, 0)],
            pilot: Pilot::None,
            donut: DonutSetup::NONE,
        };
        let mut sim = PartyFixtureSim::new(&kit_of(&FAR_FIXTURE), 360, &FAR_FIXTURE).unwrap();
        sim.run_scripted(260);
        assert_eq!(blasts(&sim).len(), 1, "the stick blew");
        assert!(kinds(&sim, "shore_broken").is_empty());
        // A fixture (QA) charge beside an untouched shore does not break it either.
        static QA: Fixture = Fixture {
            shore_dx: Some(140),
            charges: &[charge(140, 60)],
            ..base(
                "carl_prog_dyn_test_qa_shore",
                "test only",
                0x0CA1_00F1,
                &[],
                120,
            )
        };
        static QA_FIXTURE: PartyFixture = PartyFixture {
            base: &QA,
            two_player: false,
            tuning: PROVISIONAL_TUNING,
            script: &[],
            pilot: Pilot::None,
            donut: DonutSetup::NONE,
        };
        let mut sim = PartyFixtureSim::new(&kit_of(&QA_FIXTURE), 360, &QA_FIXTURE).unwrap();
        sim.run_scripted(120);
        assert_eq!(blasts(&sim), [(60, 0)]);
        assert!(
            kinds(&sim, "shore_broken").is_empty(),
            "profile QA: breaks_structures off"
        );
    }

    /// CP-Q35: the blast drops the left rack outright, credited to Carl; the crush
    /// is the boss module's.
    #[test]
    fn carl_prog_dyn_rack_strut() {
        let sim = named("carl_prog_dyn_rack_strut");
        assert_eq!(
            kinds(&sim, "rack_dropped"),
            ["COMBAT_EVENT tick=195 kind=rack_dropped rack=0 player=0"]
        );
        let landed = kinds(&sim, "rack_landed");
        assert_eq!(landed.len(), 1);
        assert!(landed[0].contains("crushed=1"), "{}", landed[0]);
        // 4 damage from the blast (CP-Q25) and 4 from the crush, no strut hit.
        let hurt = kinds(&sim, "boss_hurt");
        assert_eq!(hurt.len(), 2, "{hurt:?}");
        assert!(hurt.iter().all(|l| l.contains("damage=4")));
    }

    /// CP-Q35: a boss plan without the rack has no strut to break.
    #[test]
    fn no_rack_plan_has_no_strut_for_the_blast() {
        static BASE: Fixture = boss_base(
            "carl_prog_dyn_test_no_rack",
            "test only",
            0x0CA1_00F2,
            100,
            FULL_NO_RACKS,
            300,
        );
        static FIXTURE: PartyFixture = PartyFixture {
            base: &BASE,
            two_player: false,
            tuning: PROVISIONAL_TUNING,
            script: &[special(4, 0)],
            pilot: Pilot::None,
            donut: DonutSetup::NONE,
        };
        let mut sim = PartyFixtureSim::new(&kit_of(&FIXTURE), 360, &FIXTURE).unwrap();
        sim.run_scripted(300);
        assert_eq!(blasts(&sim).len(), 1);
        assert!(kinds(&sim, "rack_dropped").is_empty());
    }

    /// CP-Q36: an outer-ring launch is a normal launched body: it flies into the
    /// goblin beyond the ring and the CF02-B body contact knocks it down (the
    /// contact is deduped per launch, so one blast bowls a given goblin once, N9).
    #[test]
    fn carl_prog_dyn_bowling() {
        let sim = named("carl_prog_dyn_bowling");
        let contacts = kinds(&sim, "goblin_contact");
        assert_eq!(contacts.len(), 1, "{contacts:?}");
        assert!(contacts[0].contains("source=0 target=1"), "{}", contacts[0]);
        assert!(
            kinds(&sim, "enemy_defeated").is_empty(),
            "both goblins live"
        );
        assert!(sim.combat.enemies[0].alive() && sim.combat.enemies[1].alive());
    }

    /// CP-Q36: a launched goblin meets the raised block at speed: the CF02-B wall
    /// slam, with the launch credited to Carl.
    #[test]
    fn carl_prog_dyn_wall_slam() {
        let sim = named("carl_prog_dyn_wall_slam");
        let slams = kinds(&sim, "wall_slam");
        assert_eq!(slams.len(), 1, "{slams:?}");
        assert!(slams[0].contains("goblin=0") && slams[0].contains("instigator=0"));
        assert_eq!(kinds(&sim, "enemy_defeated").len(), 1);
    }

    /// The Juicer with dynamite, solo: the thrower pilot throws the charge, the boss
    /// falls inside the 2400..7200 tick window and the defeat grants the fist.
    #[test]
    fn carl_prog_juicer_dynamite_clear() {
        let fixture = &CARL_PROG_JUICER_DYNAMITE_CLEAR;
        let sim = named("carl_prog_juicer_dynamite_clear");
        let defeat = sim
            .combat
            .events
            .iter()
            .find_map(|e| match *e {
                CombatEvent::EnemyDefeated { tick, target, .. } if target == BOSS_ENEMY_ID => {
                    Some(tick)
                }
                _ => None,
            })
            .expect("the Juicer falls");
        assert!((2400..=7200).contains(&defeat), "cleared on tick {defeat}");
        assert!(fixture.base.ticks >= 7200);
        assert!(
            !kinds(&sim, "charge_thrown").is_empty(),
            "the pilot threw the charge"
        );
        let boss_blasts = kinds(&sim, "boss_hurt")
            .into_iter()
            .filter(|l| l.contains("damage=4"))
            .count();
        assert!(boss_blasts >= 1, "a stick reached the boss ({boss_blasts})");
        let grants = grants(&sim);
        assert_eq!(grants.len(), 1);
        assert_eq!(grants[0].2, "fist_reinforced");
        assert!(grants[0].0 >= defeat, "granted on the defeat");
    }

    /// The same fixture under another name, so it has no story: the no-gauntlet
    /// control of a CP-D fixture (the story is chosen by name).
    fn control_of(of: &PartyFixture, name: &'static str) -> &'static PartyFixture {
        let base: &'static Fixture = Box::leak(Box::new(Fixture { name, ..*of.base }));
        Box::leak(Box::new(PartyFixture {
            base,
            two_player: of.two_player,
            tuning: of.tuning,
            script: of.script,
            pilot: of.pilot,
            donut: of.donut,
        }))
    }

    /// Steps to the end of tick `tick` and returns the sim.
    fn at_tick(fixture: &'static PartyFixture, tick: u64) -> PartyFixtureSim {
        let mut sim = PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        sim.run_scripted(tick);
        sim
    }

    fn hurts(sim: &PartyFixtureSim) -> Vec<u64> {
        sim.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::HeroHurt { tick, .. } => Some(tick),
                _ => None,
            })
            .collect()
    }

    /// CP-Q39: jab 16 ticks and haymaker 28 (10 and 18 without the fist), read on
    /// the tick of each hit; damage, hit tick and the string are the control's.
    #[test]
    fn carl_prog_fist_stagger() {
        let fixture = find_party_fixture("carl_prog_fist_stagger").unwrap();
        let control = control_of(fixture, "carl_prog_fist_stagger_control");
        // The haymaker lands on goblin 0 on tick 22 and the jab on goblin 1 on 113.
        for (sim_fixture, haymaker, jab, extra_stop) in [(fixture, 28, 16, 1), (control, 18, 10, 0)]
        {
            let hay = at_tick(sim_fixture, 22);
            assert_eq!(
                hay.combat.enemies[0].stagger_left,
                haymaker,
                "{}",
                sim_fixture.name()
            );
            let jab_sim = at_tick(sim_fixture, 113);
            assert_eq!(
                jab_sim.combat.enemies[1].stagger_left,
                jab,
                "{}",
                sim_fixture.name()
            );
            let sim = replay(sim_fixture);
            let hits = kinds(&sim, "hit");
            assert_eq!(hits.len(), 2, "{hits:?}");
            // Damage 1 each; the haymaker's stop is 5 (+1 with the fist), the jab's 3.
            assert!(
                hits[0].contains("tick=22") && hits[0].contains("damage=1"),
                "{hits:?}"
            );
            assert!(
                hits[0].contains(&format!("hit_stop={}", 5 + extra_stop)),
                "{hits:?}"
            );
            assert!(
                hits[1].contains("tick=113") && hits[1].contains("hit_stop=3"),
                "{hits:?}"
            );
        }
    }

    /// CP-Q40, F-12: the haymaker on attack tick 24 (the smear) cancels the swing;
    /// the same input without the capability lets the impact land, and with the
    /// fist held the jab on the same tick no longer pierces.
    #[test]
    fn carl_prog_pierce_goblin() {
        let sim = named("carl_prog_pierce_goblin");
        let hit = &kinds(&sim, "hit")[0];
        assert!(
            hit.contains("tick=65") && hit.contains("move=haymaker"),
            "{hit}"
        );
        assert_eq!(kinds(&sim, "goblin_interrupted").len(), 1);
        assert!(
            kinds(&sim, "goblin_swing").is_empty(),
            "the swing never lands"
        );
        assert_eq!(hurts(&sim), Vec::<u64>::new());
        assert_eq!(sim.scene.world.hero.health, 4);
        // The presentation cache: one pierce, on the hit tick (not hashed).
        assert_eq!(sim.combat.pierce_count(), 1);
        let cue = sim.combat.pierce_cue().expect("a pierce cue");
        assert_eq!((cue.tick, cue.direction), (65, 1));
        // Control: the haymaker hits, the goblin is not interrupted and the impact lands.
        let control = named("carl_prog_pierce_goblin_control");
        let hit = &kinds(&control, "hit")[0];
        assert!(
            hit.contains("tick=65") && hit.contains("move=haymaker"),
            "{hit}"
        );
        assert!(kinds(&control, "goblin_interrupted").is_empty());
        assert_eq!(hurts(&control), vec![72]);
        assert_eq!(control.combat.pierce_count(), 0);
        assert!(control.combat.pierce_cue().is_none());
        // F-12 control: the fist is held but the jab on the same smear only damages.
        let jab = named("carl_prog_pierce_goblin_jab");
        let hit = &kinds(&jab, "hit")[0];
        assert!(hit.contains("tick=65") && hit.contains("move=jab"), "{hit}");
        assert!(kinds(&jab, "goblin_interrupted").is_empty());
        assert_eq!(hurts(&jab), vec![70]);
        assert_eq!(jab.combat.pierce_count(), 0);
        assert!(jab.combat.pierce_cue().is_none());
    }

    /// CP-Q40: the Engineer's committed launch is pierced the same way (the
    /// ram, which would hit on tick 36, never starts).
    #[test]
    fn carl_prog_pierce_engineer() {
        let sim = named("carl_prog_pierce_engineer");
        let hit = &kinds(&sim, "hit")[0];
        assert!(
            hit.contains("tick=35") && hit.contains("move=haymaker"),
            "{hit}"
        );
        assert_eq!(kinds(&sim, "goblin_interrupted").len(), 1);
        assert_eq!(hurts(&sim), Vec::<u64>::new(), "the first ram is cancelled");
        assert_eq!(sim.combat.pierce_count(), 1);
        // Control, no fist: the same haymaker on tick 35 only damages him, he is not
        // interrupted and the ram hits Carl on tick 36.
        let control = named("carl_prog_pierce_engineer_control");
        let hit = &kinds(&control, "hit")[0];
        assert!(
            hit.contains("tick=35") && hit.contains("move=haymaker"),
            "{hit}"
        );
        assert!(kinds(&control, "goblin_interrupted").is_empty());
        assert_eq!(hurts(&control)[0], 42, "the ram, held by the hit's stop");
        // And with no press at all the ram hits Carl on tick 36, the tick the
        // fist's haymaker beat.
        let idle: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            script: &[],
            ..*control_of(
                &CARL_PROG_PIERCE_ENGINEER_CONTROL,
                "carl_prog_pierce_engineer_control_idle",
            )
        }));
        assert_eq!(hurts(&replay(idle))[0], 36);
        assert_eq!(control.combat.pierce_count(), 0);
    }

    /// CP-Q40, CP-Q42: the Juicer's super armour is never pierced (no flinch, the
    /// pound tell runs on to its strike) and the reinforced haymaker freezes
    /// Carl one tick longer, so the strike lands one tick later than the control's.
    #[test]
    fn carl_prog_boss_poise() {
        let sim = named("carl_prog_boss_poise");
        let control = named_control(&CARL_PROG_BOSS_POISE, "carl_prog_boss_poise_control");
        for s in [&sim, &control] {
            assert_eq!(kinds(s, "boss_hurt").len(), 2, "both punches hurt it");
            assert!(kinds(s, "boss_flinch").is_empty(), "the armour holds");
            assert_eq!(kinds(s, "boss_tell").len(), 1);
        }
        let strike = |s: &PartyFixtureSim| -> String {
            kinds(s, "boss_strike")[0]
                .split_whitespace()
                .nth(1)
                .unwrap()
                .to_owned()
        };
        assert_eq!(strike(&control), "tick=60");
        assert_eq!(strike(&sim), "tick=61", "haymaker hit-stop +1");
        assert_eq!(sim.combat.pierce_count(), 0, "a boss is not a pierce");
    }

    fn named_control(of: &PartyFixture, name: &'static str) -> PartyFixtureSim {
        replay(control_of(of, name))
    }

    /// CP-Q41: jab (1) + haymaker (2) break a fresh shore in one combo; without
    /// the fist the same two blows leave it at 2 of 3.
    #[test]
    fn carl_prog_shore_combo() {
        let sim = named("carl_prog_shore_combo");
        let hits = kinds(&sim, "shore_hit");
        assert_eq!(hits.len(), 2, "{hits:?}");
        assert!(
            hits[0].contains("tick=23") && hits[0].contains("hits=1"),
            "{hits:?}"
        );
        assert!(
            hits[1].contains("tick=32") && hits[1].contains("hits=3"),
            "{hits:?}"
        );
        let broken = kinds(&sim, "shore_broken");
        assert!(broken[0].contains("tick=32") && broken[0].contains("player=0"));
        let defeat = kinds(&sim, "enemy_defeated");
        assert!(
            defeat[0].contains("cause=shore_block") && defeat[0].contains("credit_player=0"),
            "{defeat:?}"
        );
        let control = named_control(&CARL_PROG_SHORE_COMBO, "carl_prog_shore_combo_control");
        let hits = kinds(&control, "shore_hit");
        assert!(hits[1].contains("hits=2"), "{hits:?}");
        assert!(kinds(&control, "shore_broken").is_empty());
    }

    /// CP-Q38: the push kick is untouched by the capability: the same events,
    /// damage, hit-stop and launch as the control.
    #[test]
    fn carl_prog_kick_unchanged() {
        let sim = named("carl_prog_kick_unchanged");
        let control = named_control(
            &CARL_PROG_KICK_UNCHANGED,
            "carl_prog_kick_unchanged_control",
        );
        let events = |s: &PartyFixtureSim| -> Vec<String> {
            s.combat.events.iter().map(ToString::to_string).collect()
        };
        assert_eq!(events(&sim), events(&control));
        assert!(kinds(&sim, "hit")[0].contains("hit_stop=8"));
        assert_eq!(
            sim.combat.enemies[0].x, control.combat.enemies[0].x,
            "the same launch"
        );
        assert_eq!(sim.combat.pierce_count(), 0);
    }

    /// The registry's gauntlet fixtures hold the fist from tick 0 and the controls
    /// do not (a hold is a declared start, never a grant: no CapabilityGranted).
    #[test]
    fn the_gauntlet_fixtures_declare_the_hold_and_the_controls_do_not() {
        use crate::story_unlock::FIST_REINFORCED;
        for name in [
            "carl_prog_fist_stagger",
            "carl_prog_pierce_goblin",
            "carl_prog_pierce_goblin_jab",
            "carl_prog_pierce_engineer",
            "carl_prog_boss_poise",
            "carl_prog_shore_combo",
            "carl_prog_kick_unchanged",
        ] {
            let sim = named(name);
            assert!(sim.combat.story_holds(FIST_REINFORCED), "{name}");
            assert!(grants(&sim).is_empty(), "{name}");
        }
        for control in [
            "carl_prog_pierce_goblin_control",
            "carl_prog_pierce_engineer_control",
        ] {
            assert!(
                !named(control).combat.story_holds(FIST_REINFORCED),
                "{control}"
            );
        }
    }

    /// The notices a fixture's HUD shows, fed as the scene feeds it: `(step, label)`
    /// on the first tick of each.
    fn notices_of(name: &str) -> Vec<(u64, crate::hud::LabelId)> {
        use crate::hud::{HudConfig, HudRuntime};
        let fixture = find_party_fixture(name).unwrap();
        let mut sim = PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        let mut hud = HudRuntime::new(HudConfig::default());
        hud.set_melee_live(true);
        let mut notices = Vec::new();
        // Past the end of the script, so a queued notice can come up.
        for tick in 1..=fixture.base.ticks + 240 {
            sim.step_scripted();
            hud.step(
                sim.scene.world.hero.health,
                sim.scene.world.donut.health,
                &sim.combat.events,
                sim.combat.dropped_events,
            );
            if let Some(n) = hud.snapshot().notice
                && n.age_ticks == 0
            {
                notices.push((tick, n.headline));
            }
        }
        notices
    }

    /// CP-Q43, DP-Q12 end to end: the Juicer's defeat grants the fist and the HUD
    /// shows HEAVY FIST. With both programs the defeat tick raises four notices
    /// (the rack crush, the knockout, Carl's grant, Donut's grant); the two grants
    /// are never replaced and show in grants-list order, HEAVY FIST then PUDDLE
    /// JUMPER, one queue step (136 + 30 ticks) apart.
    #[test]
    fn the_juicer_defeat_shows_the_grant_notices_in_order() {
        use crate::hud::LabelId;
        let carl = notices_of("carl_prog_juicer_grants_fist");
        assert_eq!(
            carl.last().map(|n| n.1),
            Some(LabelId::HeavyFist),
            "{carl:?}"
        );
        assert!(
            !carl.iter().any(|n| n.1 == LabelId::PuddleJumper),
            "{carl:?}"
        );
        let both = notices_of("donut_prog_boss_both_programs");
        let n = both.len();
        assert_eq!(
            [both[n - 2].1, both[n - 1].1],
            [LabelId::HeavyFist, LabelId::PuddleJumper],
            "{both:?}"
        );
        assert_eq!(both[n - 1].0 - both[n - 2].0, 136 + 30, "{both:?}");
    }
}
