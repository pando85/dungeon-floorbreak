//! CF02 replay fixtures (CF02-00: an empty registry, plus the listing helper).
//!
//! The registry has the shape of [`crate::donut_fixtures::DONUT_FIXTURES`]
//! (`&PartyFixture`) so a `cf02_*` fixture replays through `--party-fixture`.
//! CF02-A, H, B, G and F append here; Donut and llama fixtures stay in
//! `donut_fixtures.rs` and the COMPANION ones in `companion_fixtures.rs`.
//! `CF02_FIXTURES` is deliberately not chained into `PartyFixture::all()` (so
//! `PartyFixture::by_name` and generic tests over `all()` do not see it);
//! `--party-fixture` and `--list-fixtures` use [`find_party_fixture`] and
//! [`list_fixtures`].

use crate::boss_scene::{FULL, Pilot, boss_base};
use crate::combat::EnemyAi;
use crate::combat_scene::{EnemySpawn, FIXTURES, Fixture};
use crate::companion_fixtures::COMPANION_FIXTURES;
use crate::donut_fixtures::{physical, primary};
use crate::donut_mana::DonutSetup;
use crate::input::RawInput;
use crate::party::PROVISIONAL_TUNING;
use crate::party_scene::{PartyFixture, SlotHold, base, jump, run};

/// A stationary goblin test dummy (it never tells or walks; it still gets hurt,
/// launched, lies down and gets up).
const fn dummy(x: i32) -> EnemySpawn {
    EnemySpawn {
        x,
        ai: EnemyAi::Passive,
    }
}

/// A goblin that chases and attacks.
const fn chase(x: i32) -> EnemySpawn {
    EnemySpawn {
        x,
        ai: EnemyAi::Chase,
    }
}

/// A button held together with a direction on `slot`.
const fn with_dir(from: u64, to: u64, slot: usize, input: RawInput, move_x: f32) -> SlotHold {
    SlotHold {
        from,
        to,
        slot,
        input: RawInput { move_x, ..input },
    }
}

/// Primary held together with a direction (it turns Carl when it starts).
const fn primary_dir(from: u64, to: u64, move_x: f32) -> SlotHold {
    with_dir(
        from,
        to,
        0,
        RawInput {
            attack: true,
            ..RawInput::NONE
        },
        move_x,
    )
}

/// Physical held together with a direction.
const fn physical_dir(from: u64, to: u64, move_x: f32) -> SlotHold {
    with_dir(
        from,
        to,
        0,
        RawInput {
            physical: true,
            ..RawInput::NONE
        },
        move_x,
    )
}

/// Jump and Physical pressed on the same tick.
const fn jump_physical(from: u64, to: u64) -> SlotHold {
    SlotHold {
        from,
        to,
        slot: 0,
        input: RawInput {
            jump: true,
            physical: true,
            ..RawInput::NONE
        },
    }
}

/// Down + Physical held over `from..to` on `slot`: the stomp (or, in the air, the
/// dive). Its press edge is on `from`.
pub(crate) const fn stomp(from: u64, to: u64, slot: usize) -> SlotHold {
    SlotHold {
        from,
        to,
        slot,
        input: RawInput {
            physical: true,
            down: true,
            ..RawInput::NONE
        },
    }
}

/// Hit points of a CF02 fixture goblin that needs more than the legacy 2 (so a
/// non-lethal hit can land on a lying goblin before the contact knockdown of
/// CF02-B can put one on the floor unhurt). `None`: the legacy 2.
pub fn goblin_health(fixture: &str) -> Option<u8> {
    match fixture {
        "cf02_stomp_no_stand_up"
        | "cf02_pounce_keeps_down"
        | "cf02_pounce_hold"
        | "cf02_dive_bounce_kick"
        | "cf02_air_kick_relaunch_once" => Some(4),
        "cf02_cancel_table" => Some(9),
        // COMPANION-AI-01 01A-wire: the `cf02_assist_*` fixtures (src/companion_fixtures.rs).
        name if name.starts_with("cf02_assist_") => crate::companion_fixtures::goblin_health(name),
        _ => None,
    }
}

/// CF02-G: whether the goblin morale (Q57..Q68) runs in this fixture. The A, H and
/// B fixtures keep their replays and run without it; only the `cf02_morale_*`
/// fixtures and the G scenarios listed here run with it (the `--cf02` scenario
/// always does).
pub fn morale_fixture(fixture: &str) -> bool {
    matches!(
        fixture,
        "cf02_morale_witness_hesitate"
            | "cf02_morale_spectacle_panic"
            | "cf02_morale_panic_once"
            | "cf02_morale_last_goblin"
            | "cf02_morale_cornered"
            | "cf02_morale_enraged_tell_unchanged"
            | "cf02_coward_backoff_kick"
            | "cf02_coward_jab_still_hits"
            | "cf02_showoff_taunt_punish"
            | "cf02_pincer_token"
            | "cf02_restart_reshuffles"
    )
}

// -------------------------------------------------------------- CF02-A --

/// Every fixture here starts Carl at column 400, facing right. A `dummy` goblin
/// never tells or walks, so each scenario isolates one rule. Ticks are the 1-based
/// ticks of [`crate::combat_scene`]; a hold over `from..to` gives its press edge on
/// `from`.
const KICK_THEN_STOMP_BASE: Fixture = base(
    "cf02_kick_then_stomp",
    "CF02-A: Carl kicks a goblin down, walks to it and stomps it while it lies: three beats (kick, walk, ground finish)",
    0xCF02_0A01,
    &[dummy(430)],
    120,
);

/// Q01, Q02, Q04: the kick lands at 18, the goblin lies from about 43 for 40 ticks
/// and the stomp (pressed 66, active 75) finishes it in place as a heavy kill.
pub const CF02_KICK_THEN_STOMP: PartyFixture = PartyFixture {
    base: &KICK_THEN_STOMP_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[physical(5, 6, 0), run(48, 66, 0, 1.0), stomp(66, 67, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const STOMP_WHIFF_BASE: Fixture = base(
    "cf02_stomp_whiff",
    "CF02-A: a stomp with no goblin in reach plays out its 26 ticks, steps Carl forward 6 px and hits nothing",
    0xCF02_0A02,
    &[dummy(520)],
    60,
);

/// Q02: startup 9, active 3, recovery 14; the 6 px root step.
pub const CF02_STOMP_WHIFF: PartyFixture = PartyFixture {
    base: &STOMP_WHIFF_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[stomp(5, 6, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const STOMP_STANDING_BASE: Fixture = base(
    "cf02_stomp_standing",
    "CF02-A: a stomp on a standing goblin is an ordinary 1-damage hit: stagger 10, no knock, no knockdown",
    0xCF02_0A03,
    &[dummy(430)],
    60,
);

/// Q02: standing target, stagger 10, knock 0.
pub const CF02_STOMP_STANDING: PartyFixture = PartyFixture {
    base: &STOMP_STANDING_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[stomp(5, 6, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const STOMP_NO_STAND_UP_BASE: Fixture = base(
    "cf02_stomp_no_stand_up",
    "CF02-A: a 4 HP goblin kicked down and stomped (non-lethal) stays lying: the stomp deals its damage and never stands it up",
    0xCF02_0A04,
    &[dummy(430)],
    130,
);

/// Q34: a non-lethal stomp on a lying goblin keeps it down, its timer unchanged
/// (a tough goblin: see [`goblin_health`]).
pub const CF02_STOMP_NO_STAND_UP: PartyFixture = PartyFixture {
    base: &STOMP_NO_STAND_UP_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[physical(5, 6, 0), run(48, 66, 0, 1.0), stomp(66, 67, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const CONFIRM_KICK_BASE: Fixture = base(
    "cf02_confirm_kick",
    "CF02-A: a jab that connects, then Physical in its window: the follow-through kick starts on jab tick 9 (startup 9, not 13)",
    0xCF02_0A05,
    &[dummy(430)],
    70,
);

/// Q05: the hit-confirm branch.
pub const CF02_CONFIRM_KICK: PartyFixture = PartyFixture {
    base: &CONFIRM_KICK_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), physical(10, 11, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const WHIFF_NO_CONFIRM_BASE: Fixture = base(
    "cf02_whiff_no_confirm",
    "CF02-A: a jab that whiffs gives no follow-through: the Physical press is buffered and starts the plain kick when the jab ends",
    0xCF02_0A06,
    &[dummy(470)],
    70,
);

/// Q05: a whiffed jab is not a hit-confirm.
pub const CF02_WHIFF_NO_CONFIRM: PartyFixture = PartyFixture {
    base: &WHIFF_NO_CONFIRM_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), physical(15, 16, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DOWN_PHYSICAL_BEATS_CONFIRM_BASE: Fixture = base(
    "cf02_down_physical_beats_confirm",
    "CF02-A: Down + Physical inside the confirm window of a connecting jab is the stomp (buffered past the jab), never the follow-through",
    0xCF02_0A07,
    &[dummy(430)],
    70,
);

/// Q35: Down + Physical is always the stomp.
pub const CF02_DOWN_PHYSICAL_BEATS_CONFIRM: PartyFixture = PartyFixture {
    base: &DOWN_PHYSICAL_BEATS_CONFIRM_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), stomp(17, 18, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const JAB_OVER_LYING_BASE: Fixture = base(
    "cf02_jab_over_lying",
    "CF02-A: Carl's jab and haymaker pass over a lying goblin (lying hurtbox 36 x 20); it stays down and gets up on schedule",
    0xCF02_0A08,
    &[dummy(430)],
    130,
);

/// Q03: the lying hurtbox.
pub const CF02_JAB_OVER_LYING: PartyFixture = PartyFixture {
    base: &JAB_OVER_LYING_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[
        physical(5, 6, 0),
        run(48, 66, 0, 1.0),
        primary(68, 69, 0),
        primary(75, 76, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const LETHAL_KICK_FAR_BASE: Fixture = base(
    "cf02_lethal_kick_far",
    "CF02-A: a kick that kills throws the body 240 / 220 px/s, about 125 px, far past a jab kill's 75 px",
    0xCF02_0A09,
    &[dummy(430)],
    100,
);

/// Q06: the lethal kick knock.
pub const CF02_LETHAL_KICK_FAR: PartyFixture = PartyFixture {
    base: &LETHAL_KICK_FAR_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), physical(30, 31, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const POUNCE_KEEPS_DOWN_BASE: Fixture = base(
    "cf02_pounce_keeps_down",
    "CF02-A, two players: Carl kicks a 4 HP goblin down and P2 Donut's non-lethal pounce on it keeps it lying (it never stands up)",
    0xCF02_0A0A,
    &[dummy(430)],
    110,
);

/// Q34, Q38: a non-lethal pounce on a lying goblin keeps it down.
pub const CF02_POUNCE_KEEPS_DOWN: PartyFixture = PartyFixture {
    base: &POUNCE_KEEPS_DOWN_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        physical(5, 6, 0),
        run(48, 68, 0, 1.0),
        run(30, 56, 1, 1.0),
        physical(56, 57, 1),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const POUNCE_HOLD_BASE: Fixture = base(
    "cf02_pounce_hold",
    "CF02-A, two players: Donut's pounce holds the lying goblin's down timer for 20 ticks, once per down: a second pounce does not hold it again",
    0xCF02_0A0B,
    &[dummy(430)],
    130,
);

/// Q38: the hold, once per down instance.
pub const CF02_POUNCE_HOLD: PartyFixture = PartyFixture {
    base: &POUNCE_HOLD_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        physical(5, 6, 0),
        run(48, 68, 0, 1.0),
        run(30, 56, 1, 1.0),
        physical(56, 57, 1),
        physical(92, 93, 1),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const STOMP_BOSS_BASE: Fixture = boss_base(
    "cf02_stomp_boss",
    "CF02-A: Carl's stomp on the Juicer is an ordinary 1-damage hero hit under his super armour (no interrupt, no knockdown)",
    0xCF02_0A0C,
    380,
    FULL,
    70,
);

/// Q08: the boss keeps its armour and poise rules.
pub const CF02_STOMP_BOSS: PartyFixture = PartyFixture {
    base: &STOMP_BOSS_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[run(1, 20, 0, 1.0), stomp(20, 21, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const SAME_TICK_2P_BASE: Fixture = base(
    "cf02_2p_same_tick",
    "CF02-A, two players: both press Down + Physical on one tick; Carl's is the stomp and Donut's is her ordinary pounce (she ignores Down)",
    0xCF02_0A0D,
    &[dummy(425), dummy(447)],
    50,
);

/// Q35: Donut ignores Down.
pub const CF02_2P_SAME_TICK: PartyFixture = PartyFixture {
    base: &SAME_TICK_2P_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[stomp(10, 11, 0), stomp(10, 11, 1)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

/// QA hook: fixture goblin 0 starts already launched with this `(vx, vy)` in px/s
/// (so an air kick can meet a flying goblin: a kick's own flight is over before
/// Carl can act again). `None` for every other fixture.
pub fn goblin_launch(fixture: &str) -> Option<(f32, f32)> {
    match fixture {
        "cf02_air_kick_relaunch_once" | "cf02_air_relaunch_contact" => Some((40.0, 330.0)),
        // CF02-B: a flier below the 120 px/s contact threshold.
        "cf02_contact_below_threshold" => Some((100.0, 150.0)),
        // CF02-B: a long flight from behind Carl into a goblin that is in its tell
        // (Carl's own kick would also hit a goblin that close to him).
        "cf02_contact_cancels_swing" => Some((170.0, 330.0)),
        _ => None,
    }
}

// -------------------------------------------------------------- CF02-H --

const AIR_KICK_JUMP_IN_BASE: Fixture = base(
    "cf02_air_kick_jump_in",
    "CF02-H: Carl jumps in and kicks: an air kick (startup 5, active 6) on the way up hits a standing goblin (stagger 14, knock 90) while his feet are below 54 px",
    0xCF02_0B01,
    &[dummy(445)],
    60,
);

/// Q50: the jump-in kick.
pub const CF02_AIR_KICK_JUMP_IN: PartyFixture = PartyFixture {
    base: &AIR_KICK_JUMP_IN_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[run(1, 24, 0, 1.0), jump(1, 2, 0), physical(3, 4, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const AIR_KICK_RELAUNCH_BASE: Fixture = base(
    "cf02_air_kick_relaunch_once",
    "CF02-H: an air kick on a flying goblin relaunches it at 150 / 130 px/s as a new launch, once per original launch (the goblin starts launched, a QA hook)",
    0xCF02_0B02,
    &[dummy(430)],
    70,
);

/// Q51: the relaunch.
pub const CF02_AIR_KICK_RELAUNCH_ONCE: PartyFixture = PartyFixture {
    base: &AIR_KICK_RELAUNCH_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[run(1, 30, 0, 1.0), jump(1, 2, 0), physical(6, 7, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DIVE_LYING_BASE: Fixture = base(
    "cf02_dive_lying",
    "CF02-H: Carl kicks a goblin down, jumps and dives on it while it lies: the dive finishes it in place as a heavy defeat",
    0xCF02_0B03,
    &[dummy(430)],
    110,
);

/// Q52, Q34: the dive on a lying goblin.
pub const CF02_DIVE_LYING: PartyFixture = PartyFixture {
    base: &DIVE_LYING_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[
        physical(5, 6, 0),
        run(49, 80, 0, 1.0),
        jump(49, 50, 0),
        stomp(64, 65, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DIVE_STANDING_BASE: Fixture = base(
    "cf02_dive_standing_knockdown",
    "CF02-H: a dive on a standing goblin knocks it down (lying 40 ticks, no hop) and bounces Carl (vy 240)",
    0xCF02_0B04,
    &[dummy(455)],
    110,
);

/// Q52, Q53: the dive knockdown and the bounce.
pub const CF02_DIVE_STANDING_KNOCKDOWN: PartyFixture = PartyFixture {
    base: &DIVE_STANDING_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[run(5, 20, 0, 1.0), jump(5, 6, 0), stomp(20, 21, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DIVE_BOUNCE_KICK_BASE: Fixture = base(
    "cf02_dive_bounce_kick",
    "CF02-H: dive, bounce, then the period's one air kick on the goblin it knocked down: two damage in one airborne period, never more",
    0xCF02_0B05,
    &[dummy(455)],
    90,
);

/// Q53 (C5): the emergent combination with its 2-damage cap (a 4 HP goblin).
pub const CF02_DIVE_BOUNCE_KICK: PartyFixture = PartyFixture {
    base: &DIVE_BOUNCE_KICK_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[
        run(5, 20, 0, 1.0),
        jump(5, 6, 0),
        stomp(20, 21, 0),
        physical(57, 58, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DIVE_WHIFF_BASE: Fixture = base(
    "cf02_dive_whiff_recovery",
    "CF02-H: a dive that lands on nothing costs 10 ticks of landing recovery: an attack pressed during it starts only afterwards",
    0xCF02_0B06,
    &[dummy(700)],
    100,
);

/// Q52, Q56: the dive whiff.
pub const CF02_DIVE_WHIFF_RECOVERY: PartyFixture = PartyFixture {
    base: &DIVE_WHIFF_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[jump(5, 6, 0), stomp(20, 21, 0), primary(40, 41, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const BAILOUT_SWING_BASE: Fixture = base(
    "cf02_bailout_escapes_swing",
    "CF02-H: Carl whiffs a jab facing away, then Jump in the jab's recovery 6 ticks before the goblin's swing lands: the bail-out hop leaves its reach (pressed 3 ticks before, it is hit)",
    0xCF02_0B07,
    &[chase(440)],
    80,
);

/// Q54: the bail-out answers a tell seen at least 5 ticks before impact.
pub const CF02_BAILOUT_ESCAPES_SWING: PartyFixture = PartyFixture {
    base: &BAILOUT_SWING_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[
        primary_dir(24, 25, -1.0),
        run(25, 50, 0, -1.0),
        jump(31, 32, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const BAILOUT_NO_ATTACK_BASE: Fixture = base(
    "cf02_bailout_no_attack_until_land",
    "CF02-H: during the bail-out hop no attack starts: a press in the air is dropped and one made just before landing starts after the 4-tick landing recovery",
    0xCF02_0B08,
    &[dummy(700)],
    100,
);

/// Q54, Q56: no attack until landing.
pub const CF02_BAILOUT_NO_ATTACK_UNTIL_LAND: PartyFixture = PartyFixture {
    base: &BAILOUT_NO_ATTACK_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[
        primary_dir(24, 25, -1.0),
        run(25, 50, 0, -1.0),
        jump(31, 32, 0),
        primary(36, 37, 0),
        primary(58, 59, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const BUFFER_TURNS_BASE: Fixture = base(
    "cf02_buffer_turns",
    "CF02-H: a Physical pressed during a jab with a direction held is buffered with that direction: the kick starts when the jab ends and turns Carl",
    0xCF02_0B09,
    &[dummy(700)],
    60,
);

/// Q46: a buffered fresh move turns Carl toward the direction of its press.
pub const CF02_BUFFER_TURNS: PartyFixture = PartyFixture {
    base: &BUFFER_TURNS_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), physical_dir(15, 16, -1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const BUFFER_NO_EARLY_CHAIN_BASE: Fixture = base(
    "cf02_buffer_no_early_chain",
    "CF02-H: a Primary pressed on jab tick 1 does not chain into the haymaker (the legacy chain opens on tick 3) and is too old to start a second jab",
    0xCF02_0B0A,
    &[dummy(700)],
    50,
);

/// Q46: no chain from a press before the window.
pub const CF02_BUFFER_NO_EARLY_CHAIN: PartyFixture = PartyFixture {
    base: &BUFFER_NO_EARLY_CHAIN_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), primary(6, 7, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const JUMP_ATTACK_BASE: Fixture = base(
    "cf02_jump_attack_same_tick",
    "CF02-H: Jump and Physical on one tick: Carl jumps and the kick starts as its air version on the first airborne tick",
    0xCF02_0B0B,
    &[dummy(700)],
    60,
);

/// Q47: Jump wins, the attack fires in the air.
pub const CF02_JUMP_ATTACK_SAME_TICK: PartyFixture = PartyFixture {
    base: &JUMP_ATTACK_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[jump_physical(5, 6)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const LANDING_GRACE_BASE: Fixture = base(
    "cf02_landing_grace",
    "CF02-H: Down + Physical pressed while falling within 8 px of the floor waits and fires as the ground stomp on landing, never as a dive that whiffs into the floor",
    0xCF02_0B0C,
    &[dummy(700)],
    90,
);

/// Q48: the landing grace.
pub const CF02_LANDING_GRACE: PartyFixture = PartyFixture {
    base: &LANDING_GRACE_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[jump(5, 6, 0), stomp(54, 55, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const CANCEL_TABLE_BASE: Fixture = base(
    "cf02_cancel_table",
    "CF02-H: C4 on a 9 HP goblin: an air kick that hits, then Down + Physical in its recovery cancels into the dive (C1, C2, C3 and C5 are in the confirm, bail-out and dive-bounce fixtures)",
    0xCF02_0B0D,
    &[dummy(445)],
    80,
);

/// Q49 (C4): air kick that hit -> dive.
pub const CF02_CANCEL_TABLE: PartyFixture = PartyFixture {
    base: &CANCEL_TABLE_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[
        run(1, 24, 0, 1.0),
        jump(1, 2, 0),
        physical(3, 4, 0),
        stomp(20, 21, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DIVE_BOSS_BASE: Fixture = boss_base(
    "cf02_dive_boss",
    "CF02-H: Carl's dive on the Juicer is an ordinary 1-damage hero hit under super armour; the tell it lands in still becomes its strike",
    0xCF02_0B0E,
    380,
    FULL,
    70,
);

/// Q08: the boss keeps its armour and poise rules.
pub const CF02_DIVE_BOSS: PartyFixture = PartyFixture {
    base: &DIVE_BOSS_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[run(1, 30, 0, 1.0), jump(20, 21, 0), stomp(28, 29, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// -------------------------------------------------------------- CF02-B --

const CONTACT_KNOCKDOWN_BASE: Fixture = base(
    "cf02_contact_knockdown",
    "CF02-B: a kicked goblin (170 px/s) hits a second one: the second goblin is knocked down (a short hop, then it lies 40 ticks), no damage, the flier keeps 25 % of its speed",
    0xCF02_0C01,
    &[dummy(430), dummy(462)],
    120,
);

/// Q09, Q12: a contact at kick speed is a knockdown, not a stagger.
pub const CF02_CONTACT_KNOCKDOWN: PartyFixture = PartyFixture {
    base: &CONTACT_KNOCKDOWN_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[physical(5, 6, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const CONTACT_BELOW_THRESHOLD_BASE: Fixture = base(
    "cf02_contact_below_threshold",
    "CF02-B: a flier slower than 120 px/s (a QA launch of 100 px/s) still gets the legacy contact: the second goblin staggers 24 ticks and is pushed 60 px/s, it is not knocked down",
    0xCF02_0C02,
    &[dummy(430), dummy(452)],
    80,
);

/// Q09: below the threshold the legacy stagger applies.
pub const CF02_CONTACT_BELOW_THRESHOLD: PartyFixture = PartyFixture {
    base: &CONTACT_BELOW_THRESHOLD_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const CONTACT_CANCELS_SWING_BASE: Fixture = base(
    "cf02_contact_cancels_swing",
    "CF02-B: a goblin in its tell is hit by a flier (a QA launch from behind Carl, 170 px/s): its attack ends, its token goes back and it is knocked down",
    0xCF02_0C03,
    &[dummy(335), chase(445)],
    120,
);

/// Q09, Q35: a goblin hit in its tell is knocked down the same way.
pub const CF02_CONTACT_CANCELS_SWING: PartyFixture = PartyFixture {
    base: &CONTACT_CANCELS_SWING_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const NO_CHAIN_BASE: Fixture = base(
    "cf02_no_chain",
    "CF02-B: three goblins in a row: the kicked flier knocks the second down, and the knocked goblin (no launch instance) never knocks the third down: one contact only",
    0xCF02_0C04,
    &[dummy(430), dummy(462), dummy(490)],
    120,
);

/// Q09: a knocked-down goblin carries no launch, so nothing chains.
pub const CF02_NO_CHAIN: PartyFixture = PartyFixture {
    base: &NO_CHAIN_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[physical(5, 6, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const BODY_BOWL_BASE: Fixture = base(
    "cf02_body_bowl",
    "CF02-B: a jab then a lethal kick: the killed body keeps the kick instance, flies and knocks a living goblin down (BodyContact, not GoblinContact)",
    0xCF02_0C05,
    &[dummy(430), dummy(500)],
    120,
);

/// Q10: a killed body bowls.
pub const CF02_BODY_BOWL: PartyFixture = PartyFixture {
    base: &BODY_BOWL_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), physical(30, 31, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const AIR_RELAUNCH_CONTACT_BASE: Fixture = base(
    "cf02_air_relaunch_contact",
    "CF02-B: an air kick relaunches a flying goblin as a new launch instance and the relaunched goblin knocks a second one down by the same rule (a QA launch starts goblin 0 in the air)",
    0xCF02_0C06,
    &[dummy(430), dummy(475)],
    90,
);

/// Q09, Q51: a relaunch is a launch instance like any kick.
pub const CF02_AIR_RELAUNCH_CONTACT: PartyFixture = PartyFixture {
    base: &AIR_RELAUNCH_CONTACT_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[run(1, 30, 0, 1.0), jump(1, 2, 0), physical(6, 7, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

/// Carl stands 30 px left of a raised block (x 1392) on the corridor sample.
const WALL_SLAM_KILL_BASE: Fixture = Fixture {
    carl_x: 1330,
    ..base(
        "cf02_wall_slam_kill",
        "CF02-B: a kicked goblin meets the raised block at 170 px/s: WallSlam, 1 damage, defeated in place (Melee, Heavy, credited to the kicker)",
        0xCF02_0C07,
        &[dummy(1360)],
        100,
    )
};

/// Q11: a living flier dies against a wall.
pub const CF02_WALL_SLAM_KILL: PartyFixture = PartyFixture {
    base: &WALL_SLAM_KILL_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[physical(5, 6, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const WALL_SLAM_BODY_BASE: Fixture = Fixture {
    carl_x: 1330,
    ..base(
        "cf02_wall_slam_body",
        "CF02-B: a goblin killed by a lethal kick flies into the raised block: WallSlam is logged for the dead body, which only crumples (no extra damage or defeat)",
        0xCF02_0C08,
        &[dummy(1360)],
        120,
    )
};

/// Q11: a dead flier only crumples.
pub const CF02_WALL_SLAM_BODY: PartyFixture = PartyFixture {
    base: &WALL_SLAM_BODY_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), physical(30, 31, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

/// Carl stands 30 px from the right layout edge of the corridor sample (2048),
/// which is not a solid.
const LAYOUT_EDGE_NO_SLAM_BASE: Fixture = Fixture {
    carl_x: 1980,
    ..base(
        "cf02_layout_edge_no_slam",
        "CF02-B: a kicked goblin flies into the right edge of the layout (not a solid): it stops with its body inside, no WallSlam, no damage from the edge",
        0xCF02_0C09,
        &[dummy(2010)],
        100,
    )
};

/// Q11: the layout edge without a solid is not a wall.
pub const CF02_LAYOUT_EDGE_NO_SLAM: PartyFixture = PartyFixture {
    base: &LAYOUT_EDGE_NO_SLAM_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[physical(5, 6, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// -------------------------------------------------------------- CF02-G --
//
// Seeds are chosen (scripts: the Q58 draw is stateless) so each fixture's goblins
// draw the temperament its scenario needs; the table is in the PR body and in
// `docs/evidence/cf02/cf02-g/README.md`. A `dummy` goblin is a `Passive` QA body:
// it has no morale and only serves as the victim.

const MORALE_WITNESS_HESITATE_BASE: Fixture = base(
    "cf02_morale_witness_hesitate",
    "CF02-G: Carl kills a goblin with a jab and a haymaker (routine, -20): a Show-off 90 px away, with another goblin still alive, loses its turn (hesitate, 20 ticks)",
    0xCF02_0D07,
    &[dummy(430), chase(520), chase(900)],
    120,
);

/// Q60, Q61: a routine death witnessed by a Show-off (nerve 62 -> 42) is a hesitate.
pub const CF02_MORALE_WITNESS_HESITATE: PartyFixture = PartyFixture {
    base: &MORALE_WITNESS_HESITATE_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), primary(10, 11, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const MORALE_SPECTACLE_PANIC_BASE: Fixture = base(
    "cf02_morale_spectacle_panic",
    "CF02-G: a lethal kick (heavy, spectacular, -35) is witnessed by a Coward behind Carl: it panics and flees from the scare for 60..90 ticks, with no attacks",
    0xCF02_0D00,
    &[dummy(430), chase(340), chase(900)],
    260,
);

/// Q60, Q61: a spectacular death drops a Coward below 25.
pub const CF02_MORALE_SPECTACLE_PANIC: PartyFixture = PartyFixture {
    base: &MORALE_SPECTACLE_PANIC_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), physical(30, 31, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const MORALE_PANIC_ONCE_BASE: Fixture = base(
    "cf02_morale_panic_once",
    "CF02-G: a Coward panics at the first spectacular death and, after the panic ended, a routine death on its other side only makes it hesitate: panic is once per attempt",
    0xCF02_0D01,
    &[dummy(430), dummy(385), chase(340)],
    260,
);

/// Q57, Q61: panic is once per attempt.
pub const CF02_MORALE_PANIC_ONCE: PartyFixture = PartyFixture {
    base: &MORALE_PANIC_ONCE_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[
        primary(5, 6, 0),
        physical(30, 31, 0),
        primary_dir(150, 151, -1.0),
        primary_dir(158, 159, -1.0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const MORALE_LAST_GOBLIN_BASE: Fixture = base(
    "cf02_morale_last_goblin",
    "CF02-G: the same routine death as the hesitate fixture, but the witness is the last goblin standing: it panics at nerve < 50 instead of hesitating",
    0xCF02_0D07,
    &[dummy(430), chase(520)],
    200,
);

/// Q62: the last goblin panics more easily.
pub const CF02_MORALE_LAST_GOBLIN: PartyFixture = PartyFixture {
    base: &MORALE_LAST_GOBLIN_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), primary(10, 11, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

/// Carl stands 62 px left of the raised block (x 1392) on the corridor sample.
const MORALE_CORNERED_BASE: Fixture = Fixture {
    carl_x: 1300,
    ..base(
        "cf02_morale_cornered",
        "CF02-G: a panicking goblin runs into the raised block, is blocked for 20 ticks, ends its panic with nerve 40 and fights (no softlock, no endless chase)",
        0xCF02_0D00,
        &[dummy(1330), chase(1360)],
        260,
    )
};

/// Q63: the cornered rat bites.
pub const CF02_MORALE_CORNERED: PartyFixture = PartyFixture {
    base: &MORALE_CORNERED_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), primary(10, 11, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const MORALE_ENRAGED_BASE: Fixture = base(
    "cf02_morale_enraged_tell_unchanged",
    "CF02-G: a goblin hurts Carl (+15 and +10 for its own hit): the Brawlers that witness it become enraged for 120 ticks (walk 81, attack cooldown 24); every tell is still exactly 24 ticks",
    0xCF02_0D3C,
    &[chase(430), chase(500)],
    260,
);

/// Q61: enrage never shortens the tell.
pub const CF02_MORALE_ENRAGED_TELL_UNCHANGED: PartyFixture = PartyFixture {
    base: &MORALE_ENRAGED_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const COWARD_BACKOFF_KICK_BASE: Fixture = base(
    "cf02_coward_backoff_kick",
    "CF02-G: a Coward hops back 28 px over 10 ticks out of Carl's kick (startup 13) and the kick whiffs",
    0xCF02_0D15,
    &[chase(445)],
    90,
);

/// Q64: the slow attack is dodged.
pub const CF02_COWARD_BACKOFF_KICK: PartyFixture = PartyFixture {
    base: &COWARD_BACKOFF_KICK_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[physical(5, 6, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const COWARD_JAB_STILL_HITS_BASE: Fixture = base(
    "cf02_coward_jab_still_hits",
    "CF02-G: the same Coward against Carl's jab (startup 3): it never hops back and the jab hits",
    0xCF02_0D15,
    &[chase(432)],
    90,
);

/// Q64: the jab is the opener.
pub const CF02_COWARD_JAB_STILL_HITS: PartyFixture = PartyFixture {
    base: &COWARD_JAB_STILL_HITS_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const SHOWOFF_TAUNT_PUNISH_BASE: Fixture = base(
    "cf02_showoff_taunt_punish",
    "CF02-G: Carl's jab whiffs next to a Show-off: it taunts for 36 ticks, and Carl's next hit during the taunt staggers it 20 ticks",
    0xCF02_0D03,
    &[chase(470)],
    140,
);

/// Q65: arrogance is punishable.
pub const CF02_SHOWOFF_TAUNT_PUNISH: PartyFixture = PartyFixture {
    base: &SHOWOFF_TAUNT_PUNISH_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[primary(5, 6, 0), run(19, 34, 0, 1.0), primary(36, 37, 0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const PINCER_TOKEN_BASE: Fixture = Fixture {
    carl_x: 600,
    ..base(
        "cf02_pincer_token",
        "CF02-G: two goblins close in from Carl's left and one from his right: the second attack token goes to the goblin on the other side (a pincer), not to the second-nearest on the left",
        0xCF02_0D03,
        &[chase(565), chase(575), chase(655)],
        120,
    )
};

/// Q66: the pincer token.
pub const CF02_PINCER_TOKEN: PartyFixture = PartyFixture {
    base: &PINCER_TOKEN_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const RESTART_RESHUFFLES_BASE: Fixture = base(
    "cf02_restart_reshuffles",
    "CF02-G: three chasing goblins fight Carl; the room is restarted three times and each attempt draws its own temperaments (a replayed attempt is identical)",
    0xCF02_0D00,
    &[chase(520), chase(600), chase(680)],
    300,
);

/// Q58: restarts reshuffle; each attempt replays identically.
pub const CF02_RESTART_RESHUFFLES: PartyFixture = PartyFixture {
    base: &RESTART_RESHUFFLES_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[
        primary(94, 95, 0),
        primary(101, 102, 0),
        physical(135, 136, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

pub const CF02_FIXTURES: [&PartyFixture; 47] = [
    &CF02_KICK_THEN_STOMP,
    &CF02_STOMP_WHIFF,
    &CF02_STOMP_STANDING,
    &CF02_STOMP_NO_STAND_UP,
    &CF02_CONFIRM_KICK,
    &CF02_WHIFF_NO_CONFIRM,
    &CF02_DOWN_PHYSICAL_BEATS_CONFIRM,
    &CF02_JAB_OVER_LYING,
    &CF02_LETHAL_KICK_FAR,
    &CF02_POUNCE_KEEPS_DOWN,
    &CF02_POUNCE_HOLD,
    &CF02_STOMP_BOSS,
    &CF02_2P_SAME_TICK,
    &CF02_AIR_KICK_JUMP_IN,
    &CF02_AIR_KICK_RELAUNCH_ONCE,
    &CF02_DIVE_LYING,
    &CF02_DIVE_STANDING_KNOCKDOWN,
    &CF02_DIVE_BOUNCE_KICK,
    &CF02_DIVE_WHIFF_RECOVERY,
    &CF02_BAILOUT_ESCAPES_SWING,
    &CF02_BAILOUT_NO_ATTACK_UNTIL_LAND,
    &CF02_BUFFER_TURNS,
    &CF02_BUFFER_NO_EARLY_CHAIN,
    &CF02_JUMP_ATTACK_SAME_TICK,
    &CF02_LANDING_GRACE,
    &CF02_CANCEL_TABLE,
    &CF02_DIVE_BOSS,
    &CF02_CONTACT_KNOCKDOWN,
    &CF02_CONTACT_BELOW_THRESHOLD,
    &CF02_CONTACT_CANCELS_SWING,
    &CF02_NO_CHAIN,
    &CF02_BODY_BOWL,
    &CF02_AIR_RELAUNCH_CONTACT,
    &CF02_WALL_SLAM_KILL,
    &CF02_WALL_SLAM_BODY,
    &CF02_LAYOUT_EDGE_NO_SLAM,
    &CF02_MORALE_WITNESS_HESITATE,
    &CF02_MORALE_SPECTACLE_PANIC,
    &CF02_MORALE_PANIC_ONCE,
    &CF02_MORALE_LAST_GOBLIN,
    &CF02_MORALE_CORNERED,
    &CF02_MORALE_ENRAGED_TELL_UNCHANGED,
    &CF02_COWARD_BACKOFF_KICK,
    &CF02_COWARD_JAB_STILL_HITS,
    &CF02_SHOWOFF_TAUNT_PUNISH,
    &CF02_PINCER_TOKEN,
    &CF02_RESTART_RESHUFFLES,
];

/// Looks a party fixture up in `PartyFixture::all()` and then in the CF02
/// registry (which is not chained into `all()`).
pub fn find_party_fixture(name: &str) -> Option<&'static PartyFixture> {
    PartyFixture::by_name(name).or_else(|| CF02_FIXTURES.iter().copied().find(|f| f.name() == name))
}

/// Every fixture name starting with `prefix`, over every registry: the legacy
/// combat fixtures, every party fixture (party, boss, Donut and COMPANION) and
/// the CF02 registry, each name once, in registry order. An unmatched prefix
/// gives an empty list; an empty prefix lists everything.
pub fn list_fixtures(prefix: &str) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    let all = FIXTURES
        .iter()
        .map(|f| f.name)
        .chain(PartyFixture::all().map(PartyFixture::name))
        .chain(CF02_FIXTURES.iter().map(|f| f.name()))
        .chain(COMPANION_FIXTURES.iter().map(|f| f.name()));
    for name in all {
        if name.starts_with(prefix) && !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registries_list_only_their_own_names() {
        // The COMPANION registry (01A-wire) lists under its own two prefixes.
        let companion: Vec<_> = COMPANION_FIXTURES.iter().map(|f| f.name()).collect();
        let mut listed_companion = list_fixtures("companion_");
        listed_companion.extend(list_fixtures("cf02_assist_"));
        let (mut a, mut b) = (listed_companion, companion);
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b);
        assert!(list_fixtures("no_such_prefix_").is_empty());
        // The CF02-C1 llama fixtures live in `donut_fixtures.rs`, which
        // `PartyFixture::all()` chains; the listing shows them first, then this
        // registry in order.
        let listed = list_fixtures("cf02_");
        let own: Vec<_> = listed
            .iter()
            .copied()
            .filter(|n| {
                !(n.starts_with("cf02_llama_")
                    || n.starts_with("cf02_assist_")
                    || *n == "cf02_donut_solo_llama")
            })
            .collect();
        let expected: Vec<_> = CF02_FIXTURES.iter().map(|f| f.name()).collect();
        assert_eq!(own, expected);
        for name in &listed {
            assert!(find_party_fixture(name).is_some());
        }
    }

    /// The CI loops select fixtures by prefix: a name outside its registry's
    /// prefix would never run (a silent skip).
    #[test]
    fn registry_names_carry_the_prefix_the_ci_loops_select() {
        for f in CF02_FIXTURES {
            let name = f.name();
            assert!(name.starts_with("cf02_"), "{name}");
            assert!(
                !name.starts_with("cf02_assist_"),
                "{name} belongs to COMPANION"
            );
        }
        for f in COMPANION_FIXTURES {
            let name = f.name();
            assert!(
                name.starts_with("companion_") || name.starts_with("cf02_assist_"),
                "{name}"
            );
        }
    }

    #[test]
    fn listing_covers_every_existing_registry_once() {
        let all = list_fixtures("");
        let mut unique = all.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), all.len(), "each name is listed once");
        for name in [
            "cp01_kick",
            "cp05_llama_crush",
            "solo_swap_continuity",
            "boss01_full_clear_2p",
            "q10_pounce_boss",
        ] {
            assert!(all.contains(&name), "{name}");
        }
        assert_eq!(list_fixtures("q10_").len(), 8);
        assert_eq!(list_fixtures("cp05_llama").len(), 5);
        assert!(find_party_fixture("q10_pounce_hit").is_some());
        assert!(find_party_fixture("cf02_nope").is_none());
    }

    /// CF02-00 baseline for CF02-Q20: the legacy solo-swap pan, measured on the
    /// same setup as the party test `q7_a_swap_pan_may_exceed_...` (Donut far
    /// behind, Carl runs 30 ticks, swap, 80 idle ticks). Run with
    /// `cargo test --locked cf02_baseline_swap_pan_trace -- --nocapture` to print
    /// the table recorded in `docs/evidence/cf02/audit/camera_trace.md`.
    #[test]
    fn cf02_baseline_swap_pan_trace() {
        use crate::env_kit::EnvKit;
        use crate::input::RawInput;
        use crate::party::HeroId;
        use crate::party_scene::{PartyFixtureSim, SOLO_SWAP_CONTINUITY};
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let kit = EnvKit::load(&root, "assets/levels/floor01/corridor_abc_sample.json").unwrap();
        let mut sim = PartyFixtureSim::new(&kit, 360, &SOLO_SWAP_CONTINUITY).unwrap();
        sim.scene.world.donut_body.x = 100.0;
        sim.scene.world.sync_donut_for_tests();
        let idle = RawInput::NONE;
        for _ in 0..30 {
            sim.step_slots([
                RawInput {
                    move_x: 1.0,
                    ..RawInput::NONE
                },
                idle,
            ]);
        }
        let distance_before = (sim.scene.world.hero.x - sim.scene.world.donut_body.x).round();
        sim.step_slots([
            RawInput {
                swap: true,
                ..RawInput::NONE
            },
            idle,
        ]);
        assert_eq!(sim.party.controlled_hero(), HeroId::Donut);
        let mut last = sim.scene.camera.left();
        let mut steps = Vec::new();
        for n in 1..=80 {
            sim.step_slots([idle, idle]);
            let now = sim.scene.camera.left();
            steps.push(now - last);
            if n <= 21 {
                println!(
                    "CF02_BASELINE_SWAP_PAN tick_after_swap={n} left={now} dx={}",
                    now - last
                );
            }
            last = now;
        }
        println!("CF02_BASELINE_SWAP_PAN hero_gap_px={distance_before}");
        // Legacy pan (the baseline CF02-Q20 replaces): a hard start at the full
        // 8 px/tick cap on the first tick, 18 identical steps (the last one
        // shorter on arrival), then a hard stop.
        let mut expected = vec![-8; 18];
        expected.push(-5);
        expected.resize(80, 0);
        assert_eq!(steps, expected);
        assert!(!sim.scene.pan_boosted());
    }
    // ------------------------------------------------------------ CF02-A --

    use crate::combat::{CombatEvent, ForceTier, MoveId};
    use crate::goblin::GoblinMode;

    fn kit_of(fixture: &PartyFixture) -> crate::env_kit::EnvKit {
        use crate::env_kit::EnvKit;
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        if fixture.base.boss.is_some() {
            crate::boss_scene::load_arena(&root).unwrap()
        } else {
            EnvKit::load(&root, "assets/levels/floor01/corridor_abc_sample.json").unwrap()
        }
    }

    fn replay_for(name: &str, ticks: u64) -> crate::party_scene::PartyFixtureSim {
        let fixture = find_party_fixture(name).unwrap();
        let mut sim =
            crate::party_scene::PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        sim.run_scripted(ticks);
        sim
    }

    fn replay(name: &str) -> crate::party_scene::PartyFixtureSim {
        replay_for(name, find_party_fixture(name).unwrap().base.ticks)
    }

    /// `(tick, move, damage, health_left)` of every `Hit`.
    fn hits(sim: &crate::party_scene::PartyFixtureSim) -> Vec<(u64, MoveId, u8, u8)> {
        sim.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Hit {
                    tick,
                    move_id,
                    damage,
                    health_left,
                    ..
                } => Some((tick, move_id, damage, health_left)),
                _ => None,
            })
            .collect()
    }

    fn starts(sim: &crate::party_scene::PartyFixtureSim) -> Vec<(u64, MoveId)> {
        sim.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::MoveStarted { tick, move_id, .. } => Some((tick, move_id)),
                _ => None,
            })
            .collect()
    }

    /// The goldens: one FNV state hash per fixture, replayed twice. A change is a
    /// deliberate re-baseline of that fixture only.
    const GOLDENS: [(&str, u64); 47] = [
        ("cf02_kick_then_stomp", 0xeade1505d6805381),
        ("cf02_stomp_whiff", 0xfc5181f6b763f50b),
        ("cf02_stomp_standing", 0x46b56f9470306d5e),
        ("cf02_stomp_no_stand_up", 0x789ca7bb0835fbaa),
        ("cf02_confirm_kick", 0x838403b03715e566),
        ("cf02_whiff_no_confirm", 0xab76e42fd4550ea2),
        ("cf02_down_physical_beats_confirm", 0x439c2ea3a5c7f26a),
        ("cf02_jab_over_lying", 0x778f03a3fa2222fb),
        ("cf02_lethal_kick_far", 0x13f5c26a5a91056c),
        ("cf02_pounce_keeps_down", 0x633bd81017697c67),
        ("cf02_pounce_hold", 0xaa75cd4ff812e46b),
        ("cf02_stomp_boss", 0xc191ddce479e568b),
        ("cf02_2p_same_tick", 0xda1a168a6bc17a3e),
        ("cf02_air_kick_jump_in", 0x102430d4574559e2),
        ("cf02_air_kick_relaunch_once", 0xff60bd8c7ad3e597),
        ("cf02_dive_lying", 0xe464c78d1d28214e),
        ("cf02_dive_standing_knockdown", 0xc129d7541a266920),
        ("cf02_dive_bounce_kick", 0x8decf9ef002355ab),
        ("cf02_dive_whiff_recovery", 0xf6e6165ea7f2b805),
        ("cf02_bailout_escapes_swing", 0xac675d2f15e777af),
        ("cf02_bailout_no_attack_until_land", 0x2d5c027bbe128174),
        ("cf02_buffer_turns", 0x1a5f85cefbd120ef),
        ("cf02_buffer_no_early_chain", 0x027414d51818e9e7),
        ("cf02_jump_attack_same_tick", 0xd0ecfb8433d01685),
        ("cf02_landing_grace", 0x75ce9b552aea4b3c),
        ("cf02_cancel_table", 0x2812995f9f619829),
        ("cf02_dive_boss", 0xaca9425328074169),
        ("cf02_contact_knockdown", 0xad3e4dbdf76df04c),
        ("cf02_contact_below_threshold", 0xcb311ad6d6bbf3cc),
        ("cf02_contact_cancels_swing", 0x2a19dd6ac146207b),
        ("cf02_no_chain", 0x81bb784964470183),
        ("cf02_body_bowl", 0xbdfd16d8979ebe75),
        ("cf02_air_relaunch_contact", 0x45769c837cbfe8f8),
        ("cf02_wall_slam_kill", 0xe887c36e54d1848e),
        ("cf02_wall_slam_body", 0x0e6f84ebfe3f2e09),
        ("cf02_layout_edge_no_slam", 0xfc74bfc67a33e54b),
        ("cf02_morale_witness_hesitate", 0x9bea8f387364f45e),
        ("cf02_morale_spectacle_panic", 0x2a24ee538108ee29),
        ("cf02_morale_panic_once", 0x7931e494b4664052),
        ("cf02_morale_last_goblin", 0x1aaf8db3ff0b63d1),
        ("cf02_morale_cornered", 0x9cae17f3befaa95f),
        ("cf02_morale_enraged_tell_unchanged", 0x897bdf0bdb9921bd),
        ("cf02_coward_backoff_kick", 0xfc6afbe4fc6a2ab4),
        ("cf02_coward_jab_still_hits", 0x5fa5b79dab68a5f7),
        ("cf02_showoff_taunt_punish", 0x1b204602f6c3dc63),
        ("cf02_pincer_token", 0x2f3867df0c7c1b1a),
        ("cf02_restart_reshuffles", 0x4962783a758ca230),
    ];

    /// `CF02_TRACE=<fixture>[:<from>-<to>] cargo test print_trace -- --ignored --nocapture`:
    /// goblin mode, position, speed and health per tick, plus the new events.
    #[test]
    #[ignore = "prints a per-tick trace of the fixture in CF02_TRACE"]
    fn print_trace() {
        let spec = std::env::var("CF02_TRACE").unwrap_or_default();
        let (name, range) = spec.split_once(':').unwrap_or((&spec, "1-400"));
        let (from, to) = range.split_once('-').unwrap();
        let (from, to): (u64, u64) = (from.parse().unwrap(), to.parse().unwrap());
        let fixture = find_party_fixture(name).unwrap();
        let mut sim =
            crate::party_scene::PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        let mut seen = 0;
        for tick in 1..=fixture.base.ticks.min(to) {
            sim.step_scripted();
            let mut line = format!("t{tick:03}");
            for (i, g) in sim.combat.enemies.iter().enumerate() {
                line += &format!(
                    " | g{i} {:?} x={:.1} y={:.1} vx={:.0} hp={} st={} dt={} hs={}",
                    g.mode, g.x, g.y, g.vx, g.health, g.stagger_left, g.down_ticks, g.hit_stop
                );
            }
            for e in &sim.combat.events[seen..] {
                line += &format!("\n      {e}");
            }
            seen = sim.combat.events.len();
            if tick >= from {
                println!("{line}");
            }
        }
    }

    #[test]
    #[ignore = "prints the reactions per attempt over three restarts"]
    fn print_restart_table() {
        let fixture = find_party_fixture("cf02_restart_reshuffles").unwrap();
        let mut sim =
            crate::party_scene::PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        let mut already = 0;
        for attempt in 0..3 {
            sim.run_scripted(200);
            let temps: Vec<String> = (0..3)
                .map(|g| {
                    let m = sim.combat.goblin_morale(g).unwrap();
                    format!(
                        "{}{:+}",
                        letter(m.temperament()),
                        i16::from(m.baseline())
                            - match m.temperament() {
                                Temperament::Brawler => 70,
                                Temperament::ShowOff => 60,
                                Temperament::Coward => 45,
                            }
                    )
                })
                .collect();
            let all = reactions(&sim);
            let seen: Vec<String> = all[already..]
                .iter()
                .map(|r| format!("g{} {} @{} (nerve {})", r.1, r.2.name(), r.0, r.3))
                .collect();
            already = all.len();
            println!(
                "ATTEMPT {attempt} | {} | {}",
                temps.join(" "),
                seen.join(", ")
            );
            sim.combat.restart(&mut sim.scene);
        }
    }

    #[test]
    #[ignore = "prints the goldens"]
    fn print_goldens() {
        for fixture in CF02_FIXTURES {
            println!(
                "GOLDEN {} {:#018x}",
                fixture.name(),
                replay(fixture.name()).state_hash()
            );
        }
    }

    #[test]
    fn every_cf02_fixture_replays_identically_and_matches_its_golden() {
        assert_eq!(GOLDENS.len(), CF02_FIXTURES.len());
        for (name, golden) in GOLDENS {
            let (a, b) = (replay(name), replay(name));
            assert_eq!(a.combat.events, b.combat.events, "{name} events replay");
            assert_eq!(a.state_hash(), b.state_hash(), "{name} replays");
            assert_eq!(a.state_hash(), golden, "{name}: {}", a.state_line());
        }
    }

    #[test]
    fn the_cf02_fixtures_run_under_cf02_rules() {
        for fixture in CF02_FIXTURES {
            let sim = replay_for(fixture.name(), 1);
            assert!(
                sim.combat.cf02().is_some_and(|t| t.cf02_rules),
                "{}",
                fixture.name()
            );
        }
        // A legacy fixture of the same engine does not.
        let sim = replay_for("q10_pounce_hit", 1);
        assert!(sim.combat.cf02().is_none());
    }

    #[test]
    fn q01_q02_q04_a_kick_then_a_stomp_finishes_the_lying_goblin_in_place() {
        let sim = replay("cf02_kick_then_stomp");
        assert_eq!(
            hits(&sim),
            vec![(18, MoveId::PushKick, 1, 1), (75, MoveId::Stomp, 1, 0)]
        );
        // The stomp pressed on 66 starts on 66 (startup 9, hit on 75) and lands
        // inside the 40-tick lying window that began about tick 43.
        assert!(starts(&sim).contains(&(66, MoveId::Stomp)));
        let goblin = &sim.combat.enemies[0];
        assert!(matches!(goblin.mode, GoblinMode::Down | GoblinMode::Corpse));
        assert!(!goblin.alive());
        assert!(
            sim.combat.events.iter().any(|e| matches!(
                e,
                CombatEvent::EnemyDefeated {
                    tick: 75,
                    force: ForceTier::Heavy,
                    ..
                }
            )),
            "a stomp finish is a heavy defeat"
        );
        // The body stays where it fell (lethal knock 0 / 0).
        assert!((goblin.x - 491.7).abs() < 1.0, "stayed at {}", goblin.x);
    }

    #[test]
    fn q02_a_stomp_with_nothing_in_reach_runs_26_ticks_steps_6_px_and_hits_nothing() {
        let sim = replay("cf02_stomp_whiff");
        assert_eq!(starts(&sim), vec![(5, MoveId::Stomp)]);
        assert!(hits(&sim).is_empty());
        assert!(
            (sim.scene.world.hero.x - 406.0).abs() < 0.01,
            "root step 6 px, got {}",
            sim.scene.world.hero.x
        );
        assert!(
            !sim.scene.world.hero.melee.busy(),
            "the 26-tick move is over by tick 60"
        );
    }

    #[test]
    fn q02_a_stomp_on_a_standing_goblin_staggers_it_without_knock_or_knockdown() {
        let sim = replay_for("cf02_stomp_standing", 16);
        assert_eq!(hits(&sim), vec![(14, MoveId::Stomp, 1, 1)]);
        let goblin = &sim.combat.enemies[0];
        assert_eq!(goblin.mode, GoblinMode::Hurt, "stagger, not Downed");
        assert_eq!(goblin.x, 430.0, "knock 0");
        assert_eq!(
            goblin.stagger_left, 10,
            "stagger 10 (frozen by the hit-stop)"
        );
    }

    #[test]
    fn q34_a_non_lethal_stomp_keeps_a_lying_goblin_down_and_never_stands_it_up() {
        let sim = replay_for("cf02_stomp_no_stand_up", 74);
        let before = &sim.combat.enemies[0];
        assert_eq!(before.mode, GoblinMode::Downed);
        let lying_at = before.down_ticks;
        let mut sim = sim;
        sim.step_scripted();
        assert_eq!(
            hits(&sim),
            vec![(18, MoveId::PushKick, 1, 3), (75, MoveId::Stomp, 1, 2)]
        );
        // Damaged, still lying, the timer not reset (it only waits out the
        // 7-tick freeze, like every hit-stop).
        let goblin = &sim.combat.enemies[0];
        assert_eq!((goblin.mode, goblin.health), (GoblinMode::Downed, 2));
        assert!(goblin.down_ticks >= lying_at, "the down timer is not reset");
        for _ in 0..100 {
            sim.step_scripted();
            let g = &sim.combat.enemies[0];
            assert_ne!(g.mode, GoblinMode::Hurt, "the stomp never stands it up");
        }
    }

    #[test]
    fn q05_a_jab_that_connects_then_physical_is_the_follow_through_on_jab_tick_9() {
        let sim = replay("cf02_confirm_kick");
        // The jab hit-stop (3) delays jab tick 9 from tick 14 to 17.
        assert_eq!(
            starts(&sim),
            vec![(5, MoveId::Jab), (17, MoveId::FollowKick)]
        );
        // Startup 9 (a plain push kick is 13): the first active tick is 26.
        assert_eq!(
            hits(&sim),
            vec![(8, MoveId::Jab, 1, 1), (26, MoveId::FollowKick, 1, 0)]
        );
        assert!(
            sim.combat.events.iter().any(|e| matches!(
                e,
                CombatEvent::EnemyDefeated {
                    force: ForceTier::Heavy,
                    ..
                }
            )),
            "the follow-through is a launching kick: a heavy defeat"
        );
    }

    #[test]
    fn q05_a_whiffed_jab_gives_no_follow_through_the_kick_is_buffered() {
        let sim = replay("cf02_whiff_no_confirm");
        assert_eq!(
            starts(&sim),
            vec![(5, MoveId::Jab), (19, MoveId::PushKick)],
            "the plain kick starts when the jab ends (tick 19)"
        );
        assert!(hits(&sim).is_empty());
    }

    #[test]
    fn q35_down_physical_in_the_confirm_window_is_the_stomp_never_the_follow_through() {
        let sim = replay("cf02_down_physical_beats_confirm");
        assert_eq!(
            starts(&sim),
            vec![(5, MoveId::Jab), (22, MoveId::Stomp)],
            "buffered past the jab like any press"
        );
        assert!(!starts(&sim).iter().any(|&(_, m)| m == MoveId::FollowKick));
        assert_eq!(hits(&sim).last(), Some(&(31, MoveId::Stomp, 1, 0)));
    }

    #[test]
    fn q03_the_jab_and_haymaker_pass_over_a_lying_goblin() {
        let sim = replay("cf02_jab_over_lying");
        // Only the kick ever hit it: the jab (68) and the haymaker (75) were
        // thrown while it lay.
        assert_eq!(hits(&sim), vec![(18, MoveId::PushKick, 1, 1)]);
        assert!(starts(&sim).contains(&(68, MoveId::Jab)));
        assert!(starts(&sim).contains(&(75, MoveId::Haymaker)));
        let goblin = &sim.combat.enemies[0];
        assert_eq!(goblin.health, 1);
        assert_eq!(goblin.mode, GoblinMode::Neutral, "it got up on schedule");
    }

    #[test]
    fn q03_the_lying_hurtbox_is_36_by_20_only_while_lying() {
        use crate::combat::StrikeTarget;
        let mut sim = replay_for("cf02_jab_over_lying", 60);
        let goblin = &sim.combat.enemies[0];
        assert!(goblin.lying());
        let body = StrikeTarget::body(goblin);
        assert_eq!((body.width, body.height), (36.0, 20.0));
        assert_eq!(body.bottom, goblin.y - 19.0);
        // Standing (or getting up): the 24 x 38 body again.
        for _ in 0..60 {
            sim.step_scripted();
        }
        let goblin = &sim.combat.enemies[0];
        assert!(!goblin.lying());
        let body = StrikeTarget::body(goblin);
        assert_eq!((body.width, body.height), (24.0, 38.0));
    }

    #[test]
    fn q04_a_kicked_goblin_lies_40_ticks_then_22_getting_up() {
        let mut sim = replay_for("cf02_jab_over_lying", 20);
        let mut landed = None;
        let mut up = None;
        for tick in 21..=130u64 {
            sim.step_scripted();
            let g = &sim.combat.enemies[0];
            if landed.is_none() && g.mode == GoblinMode::Downed {
                landed = Some(tick);
            }
            if landed.is_some() && up.is_none() && g.mode == GoblinMode::Neutral {
                up = Some(tick);
            }
        }
        let (landed, up) = (landed.unwrap(), up.unwrap());
        assert_eq!(up - landed, 40 + 22, "DOWNED_TICKS 40 + GETUP_TICKS 22");
    }

    #[test]
    fn q06_a_kick_that_kills_throws_the_body_about_125_px() {
        let sim = replay("cf02_lethal_kick_far");
        let goblin = &sim.combat.enemies[0];
        assert!(!goblin.alive());
        let flown = goblin.x - 433.0;
        assert!(flown > 110.0, "lethal kick flew {flown} px");
        // The same goblin killed by a jab flies the legacy 160 / 210 (about 75 px).
        assert!(hits(&sim).contains(&(43, MoveId::PushKick, 1, 0)));
    }

    #[test]
    fn q34_q38_a_non_lethal_pounce_keeps_a_lying_goblin_down() {
        let mut sim = replay_for("cf02_pounce_keeps_down", 68);
        assert_eq!(sim.combat.enemies[0].mode, GoblinMode::Downed);
        for _ in 0..20 {
            sim.step_scripted();
        }
        assert!(
            hits(&sim).contains(&(69, MoveId::Pounce, 1, 2)),
            "{:?}",
            hits(&sim)
        );
        let g = &sim.combat.enemies[0];
        assert_eq!(
            (g.mode, g.health),
            (GoblinMode::Downed, 2),
            "never stood up"
        );
    }

    #[test]
    fn q38_the_pounce_hold_pauses_the_down_timer_20_ticks_once_per_down() {
        let mut sim = replay_for("cf02_pounce_hold", 69);
        let g = &sim.combat.enemies[0];
        // Hit on 69, then the hold: the timer waits 20 ticks.
        assert_eq!(
            (g.mode, g.hold_left, g.hold_used),
            (GoblinMode::Downed, 20, true)
        );
        let frozen_at = g.down_ticks;
        let mut waited = 0;
        while sim.combat.enemies[0].hold_left > 0 {
            sim.step_scripted();
            if sim.combat.enemies[0].hold_left > 0 {
                assert_eq!(sim.combat.enemies[0].down_ticks, frozen_at, "timer paused");
                waited += 1;
            }
        }
        assert!(
            waited >= 17,
            "the hold lasted {waited} ticks (the first 3 were the freeze)"
        );
        // The second pounce (hit on 102) lands inside the same down: no new hold.
        for _ in 0..40 {
            sim.step_scripted();
        }
        assert!(
            hits(&sim)
                .iter()
                .any(|h| h.0 == 102 && h.1 == MoveId::Pounce)
        );
        let g = &sim.combat.enemies[0];
        assert_eq!(g.hold_left, 0);
        assert!(g.hold_used, "this down already used its hold");
        assert_eq!(g.health, 1);
    }

    #[test]
    fn q08_a_stomp_on_the_boss_is_an_ordinary_one_damage_hit_without_an_interrupt() {
        let sim = replay("cf02_stomp_boss");
        let hurt: Vec<_> = sim
            .combat
            .events
            .iter()
            .filter(|e| matches!(e, CombatEvent::BossHurt { .. }))
            .collect();
        assert_eq!(hurt.len(), 1, "{hurt:?}");
        let boss = sim.combat.boss_state().unwrap();
        assert_eq!(boss.hp, boss.hp_max - 1, "1 damage");
        assert!(
            sim.combat
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::BossStrike { .. })),
            "super armour: the tell the stomp landed in still became its strike"
        );
    }

    #[test]
    fn q35_donut_ignores_down_her_physical_is_the_pounce_on_the_same_tick() {
        let sim = replay("cf02_2p_same_tick");
        let s = starts(&sim);
        assert!(s.contains(&(10, MoveId::Stomp)), "{s:?}");
        assert!(s.contains(&(10, MoveId::Pounce)), "{s:?}");
        // Carl's stomp hits first (Carl, then Donut, then the goblins).
        let h = hits(&sim);
        let stomp = h.iter().position(|e| e.1 == MoveId::Stomp).unwrap();
        let pounce = h.iter().position(|e| e.1 == MoveId::Pounce).unwrap();
        assert!(stomp < pounce);
    }

    #[test]
    fn cf02_a_moves_resolve_by_id_and_carry_the_data_file_values() {
        use crate::combat::{carl_move, cf02_moves};
        let m = cf02_moves();
        assert_eq!(carl_move(MoveId::Stomp), Some(&m.stomp));
        assert_eq!(carl_move(MoveId::FollowKick), Some(&m.follow_kick));
        assert_eq!(
            (
                m.stomp.startup,
                m.stomp.active,
                m.stomp.recovery,
                m.stomp.damage
            ),
            (9, 3, 14, 1)
        );
        assert_eq!(m.stomp.total(), 26);
        assert_eq!(
            (
                m.follow_kick.startup,
                m.follow_kick.active,
                m.follow_kick.recovery
            ),
            (9, 4, 19)
        );
        assert!(
            m.follow_kick.reaction.launch,
            "the same reaction as the push kick"
        );
        assert_eq!(ForceTier::of_move(&m.stomp), ForceTier::Heavy);
        assert_eq!(ForceTier::of_move(&m.follow_kick), ForceTier::Heavy);
    }
    // ------------------------------------------------------------ CF02-H --

    fn hurt_ticks(sim: &crate::party_scene::PartyFixtureSim) -> Vec<u64> {
        sim.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::HeroHurt { tick, .. } => Some(tick),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn q50_a_jump_in_air_kick_hits_a_standing_goblin_for_stagger_14_and_knock_90() {
        let sim = replay_for("cf02_air_kick_jump_in", 9);
        assert_eq!(starts(&sim), vec![(3, MoveId::AirKick)]);
        assert_eq!(hits(&sim), vec![(8, MoveId::AirKick, 1, 1)]);
        let goblin = &sim.combat.enemies[0];
        assert_eq!(goblin.mode, GoblinMode::Hurt, "stagger, not launched");
        assert_eq!(goblin.stagger_left, 14);
        // The kick reaches 16 px below the feet: Carl's feet were below 54 px.
        let feet = sim.scene.world.hero.y - 26.0 - 92.0;
        assert!(feet < 54.0, "feet {feet} px above the floor");
    }

    #[test]
    fn q51_an_air_kick_relaunches_a_flying_goblin_once_as_a_new_launch() {
        let mut sim = replay_for("cf02_air_kick_relaunch_once", 12);
        assert_eq!(hits(&sim), vec![(11, MoveId::AirKick, 1, 3)]);
        let g = &sim.combat.enemies[0];
        assert!(g.relaunched, "spent");
        assert_eq!(g.mode, GoblinMode::Launched, "still flying");
        assert!(
            g.launch.is_some_and(|id| id != 0),
            "stamped with the air kick's instance"
        );
        // 150 px/s forward and 130 px/s up after the freeze.
        for _ in 0..8 {
            sim.step_scripted();
        }
        let g = &sim.combat.enemies[0];
        assert!(g.vx == 150.0, "vx {}", g.vx);
        // One per airborne period: the rest of the script throws no second kick.
        let mut sim = replay("cf02_air_kick_relaunch_once");
        assert_eq!(
            starts(&sim)
                .iter()
                .filter(|s| s.1 == MoveId::AirKick)
                .count(),
            1
        );
        sim.step_scripted();
    }

    #[test]
    fn q52_the_dive_on_a_lying_goblin_finishes_it_in_place_after_a_6_tick_tuck() {
        let sim = replay("cf02_dive_lying");
        assert!(starts(&sim).contains(&(64, MoveId::DiveStomp)));
        assert_eq!(hits(&sim).last(), Some(&(76, MoveId::DiveStomp, 1, 0)));
        assert!(
            sim.combat.events.iter().any(|e| matches!(
                e,
                CombatEvent::EnemyDefeated {
                    tick: 76,
                    force: ForceTier::Heavy,
                    ..
                }
            )),
            "a dive finish is heavy"
        );
        let g = &sim.combat.enemies[0];
        assert!((g.x - 491.7).abs() < 1.0, "in place, at {}", g.x);
    }

    #[test]
    fn q52_q53_the_dive_knocks_a_standing_goblin_down_without_a_hop_and_bounces_carl() {
        let mut sim = replay_for("cf02_dive_standing_knockdown", 30);
        // Tuck 6 (20..26), plunge, hit on move tick 9 (tick 29).
        assert_eq!(hits(&sim), vec![(29, MoveId::DiveStomp, 1, 1)]);
        let g = &sim.combat.enemies[0];
        assert_eq!((g.mode, g.vx, g.vy), (GoblinMode::Downed, 0.0, 0.0));
        assert_eq!(g.y, g.floor_y, "no hop");
        // The bounce starts when the 6-tick hit-stop ends: vy 240 (minus one
        // gravity step), vx 0.
        for _ in 0..7 {
            sim.step_scripted();
        }
        let vy = sim.scene.world.hero.vy;
        assert!(vy > 200.0 && vy <= 240.0, "bounce vy {vy}");
        assert_eq!(sim.scene.world.hero.vx, 0.0);
    }

    #[test]
    fn q53_dive_bounce_air_kick_deals_two_damage_in_one_period_and_no_more() {
        let sim = replay("cf02_dive_bounce_kick");
        assert_eq!(
            hits(&sim),
            vec![(29, MoveId::DiveStomp, 1, 3), (65, MoveId::AirKick, 1, 2)]
        );
        // The air kick on the knocked-down goblin keeps it down (Q34).
        assert_eq!(sim.combat.enemies[0].mode, GoblinMode::Downed);
    }

    #[test]
    fn q53_no_cf02_fixture_deals_more_than_two_air_damage_per_airborne_period() {
        for fixture in CF02_FIXTURES {
            let mut sim =
                crate::party_scene::PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
            let (mut period, mut seen) = (0u32, 0usize);
            for _ in 0..fixture.base.ticks {
                sim.step_scripted();
                if sim.scene.world.hero.grounded {
                    period = 0;
                }
                for e in &sim.combat.events[seen..] {
                    if let CombatEvent::Hit {
                        move_id: MoveId::AirKick | MoveId::DiveStomp,
                        damage,
                        ..
                    } = *e
                    {
                        period += u32::from(damage);
                    }
                }
                seen = sim.combat.events.len();
                assert!(period <= 2, "{}: {period} in one period", fixture.name());
            }
        }
    }

    #[test]
    fn q56_a_dive_whiff_costs_10_ticks_of_landing_recovery() {
        let sim = replay("cf02_dive_whiff_recovery");
        let s = starts(&sim);
        assert_eq!(s[0], (20, MoveId::DiveStomp));
        // The press on 40 waits out the recovery; the jab starts on tick 46,
        // 10 ticks after the landing on 36.
        assert_eq!(s[1], (46, MoveId::Jab), "{s:?}");
        assert!(hits(&sim).is_empty());
    }

    /// The bail-out fixture with the Jump pressed on `press` instead of 31 (the
    /// goblin's swing lands on 37), plus extra script holds.
    fn bailout_sim(script: &'static [SlotHold], ticks: u64) -> crate::party_scene::PartyFixtureSim {
        let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            script,
            ..CF02_BAILOUT_ESCAPES_SWING
        }));
        let mut sim =
            crate::party_scene::PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
        sim.run_scripted(ticks);
        sim
    }

    #[test]
    fn q54_the_bailout_escapes_a_tell_seen_4_or_more_ticks_before_impact_and_not_later() {
        let sim = replay("cf02_bailout_escapes_swing");
        assert!(
            sim.combat
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::GoblinSwing { tick: 37, .. })),
            "the swing lands on 37"
        );
        assert!(hurt_ticks(&sim).is_empty(), "6 ticks before impact: no hit");
        // The hop is vx 150 away from the facing (the held direction here), vy 260.
        let early = replay_for("cf02_bailout_escapes_swing", 31);
        assert!(early.scene.world.hero.vy > 240.0, "hop started on 31");
        // Measured boundary (a deviation from the acceptance wording "5 or more
        // ticks", recorded for the owner's game-feel call): Jump 7, 6, 5 and 4
        // ticks before impact escapes; 3 ticks before is hit.
        static J30: [SlotHold; 3] = [
            primary_dir(24, 25, -1.0),
            run(25, 50, 0, -1.0),
            jump(30, 31, 0),
        ];
        static J33: [SlotHold; 3] = [
            primary_dir(24, 25, -1.0),
            run(25, 50, 0, -1.0),
            jump(33, 34, 0),
        ];
        static J34: [SlotHold; 3] = [
            primary_dir(24, 25, -1.0),
            run(25, 50, 0, -1.0),
            jump(34, 35, 0),
        ];
        assert!(hurt_ticks(&bailout_sim(&J30, 80)).is_empty(), "7 ticks");
        assert!(hurt_ticks(&bailout_sim(&J33, 80)).is_empty(), "4 ticks");
        assert_eq!(hurt_ticks(&bailout_sim(&J34, 80)), vec![37], "3 ticks: hit");
    }

    #[test]
    fn q54_a_hurt_mid_hop_keeps_no_attack_until_landing_and_the_landing_recovery() {
        // The 3-tick-late bail-out (Jump on 34) is hit on 37 mid-hop. The hurt
        // flinch is over by 63 and Carl is still in the air: a Primary pressed on
        // 63 must wait for the landing (67) and its 4-tick recovery, so the jab
        // starts on 71 (landing 67 + 4). (With the flag lost on the hurt it would start in the air.)
        static LATE: [SlotHold; 4] = [
            primary_dir(24, 25, -1.0),
            run(25, 50, 0, -1.0),
            jump(34, 35, 0),
            primary(63, 64, 0),
        ];
        let mut sim = bailout_sim(&LATE, 0);
        let mut landed = None;
        for tick in 1..=120u64 {
            sim.step_scripted();
            if tick > 40 && landed.is_none() && sim.scene.world.hero.grounded {
                landed = Some(tick);
            }
        }
        assert_eq!(hurt_ticks(&sim), vec![37]);
        let landed = landed.expect("Carl lands");
        assert!(
            landed > 63,
            "still airborne when the press is made ({landed})"
        );
        let jabs: Vec<u64> = starts(&sim)
            .into_iter()
            .filter(|&(t, m)| t > 24 && m == MoveId::Jab)
            .map(|(t, _)| t)
            .collect();
        // The body touches down on `landed` (66); the landing is planned on 67 and
        // its 4-tick recovery runs 67..=70.
        assert_eq!(jabs, vec![landed + 1 + 4], "landed {landed}");
        assert_eq!(jabs, vec![71]);
    }

    #[test]
    fn q49_n1_a_down_physical_pressed_during_the_air_kick_takes_c4_in_the_recovery() {
        // The air kick hits on 8 (hit-stop 5). A Down + Physical pressed on any
        // tick 10..=18 (hit-stop, then the rest of the active and the startup of
        // the recovery) is remembered and fires on the first tick C4 opens.
        let mut first = None;
        for press in 10u64..=18 {
            let script: &'static [SlotHold] = Box::leak(
                vec![
                    run(1, 24, 0, 1.0),
                    jump(1, 2, 0),
                    physical(3, 4, 0),
                    stomp(press, press + 1, 0),
                ]
                .into_boxed_slice(),
            );
            let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
                script,
                ..CF02_CANCEL_TABLE
            }));
            let mut sim =
                crate::party_scene::PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
            sim.run_scripted(70);
            let dive: Vec<u64> = starts(&sim)
                .into_iter()
                .filter(|s| s.1 == MoveId::DiveStomp)
                .map(|s| s.0)
                .collect();
            assert_eq!(dive.len(), 1, "press {press}: {:?}", starts(&sim));
            let tick = dive[0];
            assert!(tick >= press, "press {press}");
            // Every early press fires on the same first tick the window opens.
            assert_eq!(*first.get_or_insert(tick), tick, "press {press}");
            assert!(
                hits(&sim).iter().any(|h| h.1 == MoveId::DiveStomp),
                "press {press}: the dive lands"
            );
        }
    }

    #[test]
    fn the_cf02_fixture_predicate_is_one_function_and_the_llama_switch_follows_it() {
        use crate::cf02::fixture_runs_cf02;
        assert!(fixture_runs_cf02("cf02_llama_pounce") && fixture_runs_cf02("cf02_stomp_whiff"));
        assert!(!fixture_runs_cf02("q10_pounce_hit") && !fixture_runs_cf02("demo01_b1"));
        // The llama switch is derived from `CombatState::cf02()`.
        let on = replay_for("cf02_stomp_whiff", 1);
        assert!(on.combat.cf02().is_some() && on.combat.llama_targets_live().is_some());
        let off = replay_for("q10_pounce_hit", 1);
        assert!(off.combat.cf02().is_none() && off.combat.llama_targets_live().is_none());
    }

    #[test]
    fn a_lying_goblin_reports_its_down_ticks_left_and_a_standing_one_none() {
        let sim = replay_for("cf02_jab_over_lying", 60);
        let g = &sim.combat.enemies[0];
        assert!(g.lying());
        assert_eq!(g.down_ticks_left(), Some(40 - g.down_ticks));
        let sim = replay_for("cf02_jab_over_lying", 125);
        assert_eq!(sim.combat.enemies[0].down_ticks_left(), None);
    }

    #[test]
    fn q54_q56_no_attack_starts_during_the_hop_and_the_landing_recovery_is_4_ticks() {
        let sim = replay("cf02_bailout_no_attack_until_land");
        assert_eq!(
            starts(&sim),
            vec![(24, MoveId::Jab), (64, MoveId::Jab)],
            "the press on 36 (in the air) is dropped; the one on 58 starts after landing + 4"
        );
    }

    #[test]
    fn q46_a_buffered_kick_turns_carl_toward_the_direction_it_was_pressed_with() {
        let sim = replay("cf02_buffer_turns");
        assert_eq!(starts(&sim), vec![(5, MoveId::Jab), (19, MoveId::PushKick)]);
        assert_eq!(sim.scene.world.hero.facing, -1);
    }

    #[test]
    fn q46_a_primary_on_jab_tick_1_does_not_chain_and_is_too_old_to_start_a_second_jab() {
        let sim = replay("cf02_buffer_no_early_chain");
        assert_eq!(starts(&sim), vec![(5, MoveId::Jab)]);
    }

    #[test]
    fn q46_the_window_is_8_under_cf02_and_6_in_the_legacy_game() {
        // A Physical pressed N ticks before a jab ends starts a kick when N <= 8
        // under CF02 and N <= 6 in the legacy runner (the same script, rules off).
        fn kick_start(cf02: bool, press: u64) -> Option<u64> {
            const BASE: Fixture = base("latency_probe", "probe", 1, &[dummy(700)], 40);
            static LEGACY: Fixture = BASE;
            let kit = kit_of(&CF02_STOMP_WHIFF);
            let scene = crate::corridor::SceneSim::new_at(&kit, 360, LEGACY.carl_x).unwrap();
            let mut combat = crate::combat_scene::CombatState::new(&LEGACY, &kit, &scene).unwrap();
            if cf02 {
                combat = combat.with_cf02();
            }
            let mut scene = scene;
            let press_tick = press;
            for tick in 1..=40u64 {
                let raw = RawInput {
                    attack: tick == 5,
                    physical: tick == press_tick,
                    ..RawInput::NONE
                };
                combat.step(&mut scene, raw);
            }
            combat.events.iter().find_map(|e| match *e {
                CombatEvent::MoveStarted {
                    tick,
                    move_id: MoveId::PushKick,
                    ..
                } => Some(tick),
                _ => None,
            })
        }
        // The jab ends on tick 19.
        assert_eq!(kick_start(true, 11), Some(19), "8 ticks before the end");
        assert_eq!(kick_start(true, 10), None, "9 ticks is too early");
        assert_eq!(kick_start(false, 13), Some(19), "6 ticks (legacy)");
        assert_eq!(kick_start(false, 12), None, "7 ticks is too early (legacy)");
    }

    #[test]
    fn q47_jump_plus_physical_on_one_tick_jumps_and_kicks_in_the_air() {
        let sim = replay("cf02_jump_attack_same_tick");
        assert_eq!(starts(&sim), vec![(6, MoveId::AirKick)]);
        // Jump + Down + Physical: the dive on the first tick it is high enough.
        const DIVE: PartyFixture = PartyFixture {
            script: &[SlotHold {
                from: 5,
                to: 6,
                slot: 0,
                input: RawInput {
                    jump: true,
                    physical: true,
                    down: true,
                    ..RawInput::NONE
                },
            }],
            ..CF02_JUMP_ATTACK_SAME_TICK
        };
        let mut sim = crate::party_scene::PartyFixtureSim::new(&kit_of(&DIVE), 360, &DIVE).unwrap();
        sim.run_scripted(DIVE.base.ticks);
        assert_eq!(starts(&sim), vec![(7, MoveId::DiveStomp)]);
    }

    #[test]
    fn q48_down_physical_near_the_floor_waits_and_fires_as_the_ground_stomp() {
        let sim = replay("cf02_landing_grace");
        assert_eq!(starts(&sim), vec![(55, MoveId::Stomp)]);
    }

    #[test]
    fn q49_c4_an_air_kick_that_hit_cancels_into_the_dive_only_in_its_recovery() {
        let sim = replay("cf02_cancel_table");
        assert_eq!(
            starts(&sim),
            vec![(3, MoveId::AirKick), (20, MoveId::DiveStomp)]
        );
        assert_eq!(
            hits(&sim),
            vec![(8, MoveId::AirKick, 1, 8), (29, MoveId::DiveStomp, 1, 7)]
        );
    }

    /// Q49: the table is closed. Whatever press comes at whatever tick of a
    /// running ground move, no new move starts before the running one has ended,
    /// except the jab's own C1 chain (Primary on jab ticks 3..=13); a Jump never
    /// starts a move (a bail-out only hops).
    #[test]
    fn q49_no_press_cancels_a_ground_move_outside_the_table() {
        #[derive(Clone, Copy)]
        struct Press {
            primary: bool,
            physical: bool,
            down: bool,
            jump: bool,
        }
        let presses = [
            Press {
                primary: true,
                physical: false,
                down: false,
                jump: false,
            },
            Press {
                primary: false,
                physical: true,
                down: false,
                jump: false,
            },
            Press {
                primary: false,
                physical: true,
                down: true,
                jump: false,
            },
            Press {
                primary: false,
                physical: false,
                down: false,
                jump: true,
            },
        ];
        let starters = [
            (
                Press {
                    primary: true,
                    physical: false,
                    down: false,
                    jump: false,
                },
                MoveId::Jab,
                14u64,
            ),
            (
                Press {
                    primary: false,
                    physical: true,
                    down: false,
                    jump: false,
                },
                MoveId::PushKick,
                36,
            ),
            (
                Press {
                    primary: false,
                    physical: true,
                    down: true,
                    jump: false,
                },
                MoveId::Stomp,
                26,
            ),
        ];
        for (starter, first, total) in starters {
            for offset in 1..total {
                for press in presses {
                    let fixture = &CF02_STOMP_WHIFF;
                    let mut sim =
                        crate::party_scene::PartyFixtureSim::new(&kit_of(fixture), 360, fixture)
                            .unwrap();
                    for tick in 1..=(5 + total + 12) {
                        let at_start = tick == 5;
                        let at_press = tick == 5 + offset;
                        let raw = RawInput {
                            attack: (at_start && starter.primary) || (at_press && press.primary),
                            physical: (at_start && starter.physical)
                                || (at_press && press.physical),
                            down: (at_start && starter.down) || (at_press && press.down),
                            jump: at_press && press.jump,
                            ..RawInput::NONE
                        };
                        sim.step_slots([raw, RawInput::NONE]);
                    }
                    let early: Vec<_> = starts(&sim)
                        .into_iter()
                        .filter(|&(t, m)| t > 5 && t < 5 + total && m != first)
                        .collect();
                    let c1 = first == MoveId::Jab
                        && press.primary
                        && (3..=13).contains(&offset)
                        && !press.jump;
                    if c1 {
                        continue;
                    }
                    assert!(
                        early.is_empty(),
                        "{first:?} press at +{offset} ({:?}) started {early:?}",
                        (press.primary, press.physical, press.down, press.jump)
                    );
                }
            }
        }
    }

    #[test]
    fn q08_the_dive_on_the_boss_is_an_ordinary_one_damage_hit_under_super_armour() {
        let sim = replay("cf02_dive_boss");
        assert!(starts(&sim).contains(&(28, MoveId::DiveStomp)));
        let boss = sim.combat.boss_state().unwrap();
        assert_eq!(boss.hp, boss.hp_max - 1);
        assert!(
            sim.combat
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::BossStrike { .. })),
            "super armour: the tell still became its strike"
        );
    }

    #[test]
    fn the_cf02_a_and_h_states_are_not_hashed_while_idle() {
        // A CF02 run that never buffers or jumps hashes its Carl state to nothing.
        use crate::combat_scene::CombatState;
        let kit = kit_of(&CF02_STOMP_WHIFF);
        let scene = crate::corridor::SceneSim::new_at(&kit, 360, 400).unwrap();
        let a = CombatState::new(CF02_STOMP_WHIFF.base, &kit, &scene).unwrap();
        let b = CombatState::new(CF02_STOMP_WHIFF.base, &kit, &scene).unwrap();
        assert_eq!(a.state_hash(&scene), b.state_hash(&scene));
    }

    // ------------------------------------------------------------ CF02-B --

    /// Every goblin event of `sim` as `(tick, text)`, for a readable failure.
    fn events_of(sim: &crate::party_scene::PartyFixtureSim) -> Vec<CombatEvent> {
        sim.combat.events.clone()
    }

    /// `(tick, launch, source, target)` of every `GoblinContact`.
    fn contacts(sim: &crate::party_scene::PartyFixtureSim) -> Vec<(u64, u32, u32, u32)> {
        events_of(sim)
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::GoblinContact {
                    tick,
                    launch,
                    source,
                    target,
                } => Some((tick, launch, source, target)),
                _ => None,
            })
            .collect()
    }

    fn body_contacts(
        sim: &crate::party_scene::PartyFixtureSim,
    ) -> Vec<(u64, u32, u32, u32, Option<u32>)> {
        events_of(sim)
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::BodyContact {
                    tick,
                    launch,
                    source,
                    target,
                    instigator,
                } => Some((tick, launch, source, target, instigator)),
                _ => None,
            })
            .collect()
    }

    /// `(tick, launch, goblin, instigator)` of every `WallSlam`.
    fn slams(sim: &crate::party_scene::PartyFixtureSim) -> Vec<(u64, u32, u32, Option<u32>)> {
        events_of(sim)
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::WallSlam {
                    tick,
                    launch,
                    goblin,
                    instigator,
                } => Some((tick, launch, goblin, instigator)),
                _ => None,
            })
            .collect()
    }

    /// The tick on which goblin `index` first has `mode`, stepping `sim` from its
    /// current tick for at most `limit` ticks.
    fn first_tick_in(
        sim: &mut crate::party_scene::PartyFixtureSim,
        index: usize,
        mode: GoblinMode,
        limit: u64,
    ) -> Option<u64> {
        for _ in 0..limit {
            sim.step_scripted();
            if sim.combat.enemies[index].mode == mode {
                return Some(sim.combat.elapsed);
            }
        }
        None
    }

    #[test]
    fn q09_q12_a_kick_speed_contact_knocks_the_second_goblin_down_without_damage() {
        let mut sim = replay_for("cf02_contact_knockdown", 28);
        // The kick lands on tick 18; the flier meets goblin 1 on tick 29.
        assert_eq!(hits(&sim), vec![(18, MoveId::PushKick, 1, 1)]);
        assert!(contacts(&sim).is_empty());
        sim.step_scripted();
        assert_eq!(contacts(&sim), vec![(29, 1, 0, 1)]);
        let (flier, target) = (&sim.combat.enemies[0], &sim.combat.enemies[1]);
        // The flier keeps 25 % of 170 px/s, shows the contact pose and freezes 4.
        assert!((flier.vx - 42.5).abs() < 0.01, "{}", flier.vx);
        assert_eq!((flier.contact_ticks, flier.hit_stop), (6, 4));
        // The target hops (90 / 90) as a living flier with no launch: no chain.
        assert_eq!(target.mode, GoblinMode::Launched);
        assert_eq!(target.launch, None);
        assert_eq!((target.vx, target.vy), (90.0, 90.0));
        assert_eq!(target.hit_stop, 4);
        // No damage: Q12.
        assert_eq!(target.health, 2);
        // It lands, lies the CF02 40 ticks and gets up 22 ticks later.
        let landed = first_tick_in(&mut sim, 1, GoblinMode::Downed, 40).unwrap();
        let up = first_tick_in(&mut sim, 1, GoblinMode::Neutral, 90).unwrap();
        assert_eq!(up - landed, 40 + 22);
        assert_eq!(sim.combat.enemies[1].health, 2);
        // The only damage in the whole run is the kick's.
        assert_eq!(contacts(&sim).len(), 1);
    }

    #[test]
    fn q09_below_120_px_s_the_legacy_stagger_and_push_apply() {
        let mut sim = replay_for("cf02_contact_below_threshold", 1);
        assert_eq!(contacts(&sim), vec![(1, 0, 0, 1)]);
        let target = &sim.combat.enemies[1];
        assert_eq!(target.mode, GoblinMode::Hurt);
        assert_eq!(target.stagger_left, 24);
        assert_eq!(target.vx, 60.0);
        assert_eq!(target.health, 2);
        // It is a stagger, never a knockdown: it never lies down.
        for _ in 0..80 {
            sim.step_scripted();
            assert!(
                !matches!(
                    sim.combat.enemies[1].mode,
                    GoblinMode::Launched | GoblinMode::Downed
                ),
                "tick {}",
                sim.combat.elapsed
            );
        }
    }

    #[test]
    fn q09_q35_a_goblin_hit_in_its_tell_has_its_attack_ended_and_the_token_returned() {
        let mut sim = replay_for("cf02_contact_cancels_swing", 27);
        let tell = sim.combat.enemies[1].clone();
        assert_eq!(tell.mode, GoblinMode::Attack);
        assert!(tell.token_held(), "it holds a token in its tell");
        sim.step_scripted();
        let hit = &sim.combat.enemies[1];
        assert_eq!(hit.mode, GoblinMode::Launched);
        assert!(!hit.token_held());
        assert_eq!(hit.attack_tick, 0);
        let events = events_of(&sim);
        assert!(events.contains(&CombatEvent::GoblinInterrupted {
            tick: 28,
            goblin: 1
        }));
        assert_eq!(contacts(&sim), vec![(28, 0, 0, 1)]);
        // Nobody was hurt: the swing never came.
        let hurt = events
            .iter()
            .any(|e| matches!(e, CombatEvent::HeroHurt { .. }));
        assert!(!hurt);
        // The token is free again for the next tick's rule.
        assert_eq!(
            sim.combat.enemies.iter().filter(|g| g.token_held()).count(),
            0
        );
    }

    #[test]
    fn q09_a_knocked_down_goblin_never_chains_into_a_third() {
        let mut sim = replay_for("cf02_no_chain", 1);
        let mut seen_launch_for_target = false;
        for _ in 0..119 {
            sim.step_scripted();
            let g1 = &sim.combat.enemies[1];
            if g1.mode == GoblinMode::Launched {
                seen_launch_for_target = true;
                assert_eq!(g1.launch, None, "a knocked-down goblin carries no launch");
            }
            let g2 = &sim.combat.enemies[2];
            assert_eq!(g2.mode, GoblinMode::Neutral, "tick {}", sim.combat.elapsed);
            assert_eq!(g2.health, 2);
        }
        assert!(seen_launch_for_target);
        assert_eq!(contacts(&sim), vec![(29, 1, 0, 1)]);
        assert!(body_contacts(&sim).is_empty() && slams(&sim).is_empty());
    }

    #[test]
    fn q10_a_body_killed_by_a_lethal_kick_bowls_a_living_goblin_down() {
        let sim = replay("cf02_body_bowl");
        assert_eq!(
            hits(&sim),
            vec![(8, MoveId::Jab, 1, 1), (43, MoveId::PushKick, 1, 0)]
        );
        // A BodyContact, not a GoblinContact, credited to Carl.
        assert!(contacts(&sim).is_empty());
        assert_eq!(body_contacts(&sim), vec![(65, 2, 0, 1, Some(0))]);
        let (body, target) = (&sim.combat.enemies[0], &sim.combat.enemies[1]);
        assert!(!body.alive());
        assert_eq!(
            body.bowl, None,
            "the bowl instance ends when the body lands"
        );
        assert_eq!(target.health, 2, "no contact damage (Q12)");
        // It was knocked down: it lay and got up on schedule.
        let mut again = replay_for("cf02_body_bowl", 66);
        assert_eq!(again.combat.enemies[1].mode, GoblinMode::Launched);
        assert_eq!(again.combat.enemies[1].launch, None);
        assert!(first_tick_in(&mut again, 1, GoblinMode::Downed, 30).is_some());
    }

    #[test]
    fn q10_only_a_lethal_kick_bowls_a_jab_or_haymaker_kill_does_not() {
        use crate::combat::{CARL_HAYMAKER, CARL_JAB, carl_move};
        use crate::combat_scene::CombatState;
        let fixture = find_party_fixture("cf02_body_bowl").unwrap();
        let kit = kit_of(fixture);
        let scene = crate::corridor::SceneSim::new_at(&kit, 360, 400).unwrap();
        for (spec, bowls) in [
            (&CARL_JAB, false),
            (&CARL_HAYMAKER, false),
            (carl_move(MoveId::PushKick).unwrap(), true),
            (carl_move(MoveId::FollowKick).unwrap(), true),
            (carl_move(MoveId::AirKick).unwrap(), true),
        ] {
            let mut combat = CombatState::new(fixture.base, &kit, &scene).unwrap();
            combat.enemies[0].health = 1;
            crate::combat::StrikeTarget::take_hit(&mut combat.enemies[0], spec, 1);
            assert!(!combat.enemies[0].alive());
            combat.stamp_bowl_for_test(0, spec.id, 9);
            assert_eq!(
                combat.enemies[0].bowl,
                bowls.then_some(9),
                "{:?} (bowl is a lethal kick only)",
                spec.id
            );
        }
    }

    #[test]
    fn q09_q51_an_air_relaunch_is_a_launch_instance_like_any_kick() {
        let sim = replay("cf02_air_relaunch_contact");
        assert_eq!(hits(&sim), vec![(11, MoveId::AirKick, 1, 1)]);
        // The relaunch carries the air kick's instance (1), not the QA launch (0).
        assert_eq!(contacts(&sim), vec![(31, 1, 0, 1)]);
        let target = &sim.combat.enemies[1];
        assert_eq!(target.health, 2);
        assert_eq!(target.launch, None);
    }

    #[test]
    fn q11_a_kicked_goblin_that_meets_a_wall_is_slammed_and_defeated_in_place() {
        let sim = replay("cf02_wall_slam_kill");
        assert_eq!(slams(&sim), vec![(34, 1, 0, Some(0))]);
        let defeats: Vec<_> = events_of(&sim)
            .into_iter()
            .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
            .collect();
        // One defeat, on the slam tick: Melee, force Heavy, credited to Carl,
        // and no second Hit: the slam is the lethal damage.
        assert_eq!(
            defeats,
            vec![CombatEvent::EnemyDefeated {
                tick: 34,
                target: 0,
                cause: crate::combat::DefeatCause::Melee,
                force: ForceTier::Heavy,
                instigator: Some(0),
            }]
        );
        assert_eq!(hits(&sim), vec![(18, MoveId::PushKick, 1, 1)]);
        let goblin = &sim.combat.enemies[0];
        assert_eq!(goblin.health, 0);
        // It crumples at the wall: it never moved through the block (x 1392).
        assert!(goblin.x + 12.0 <= 1392.0 + 0.01, "x = {}", goblin.x);
    }

    #[test]
    fn q11_a_dead_flier_only_crumples_at_a_wall() {
        let sim = replay("cf02_wall_slam_body");
        assert_eq!(slams(&sim), vec![(59, 2, 0, Some(0))]);
        // Only the one defeat (the lethal kick, tick 43): the slam adds none.
        let defeated = events_of(&sim)
            .iter()
            .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
            .count();
        assert_eq!(defeated, 1);
        let body = &sim.combat.enemies[0];
        assert!(!body.alive() && body.bowl.is_none());
        assert_eq!(body.health, 0);
    }

    #[test]
    fn q11_the_layout_edge_without_a_solid_is_not_a_wall() {
        let kit = kit_of(&CF02_LAYOUT_EDGE_NO_SLAM);
        let width = kit.width as f32;
        let mut sim = replay_for("cf02_layout_edge_no_slam", 1);
        let mut touched_edge = false;
        for _ in 0..99 {
            sim.step_scripted();
            let g = &sim.combat.enemies[0];
            assert!(
                g.x + 12.0 <= width + 1e-3,
                "tick {} x {}",
                sim.combat.elapsed,
                g.x
            );
            touched_edge |= g.x + 12.0 >= width - 1e-3;
        }
        assert!(
            touched_edge,
            "the kick must reach the edge for this test to mean anything"
        );
        assert!(slams(&sim).is_empty());
        // The kick's 1 damage only: the edge hurt nobody.
        assert_eq!(sim.combat.enemies[0].health, 1);
        assert!(sim.combat.enemies[0].alive());
    }

    #[test]
    fn q09_q10_q11_one_effect_per_launch_and_target_and_no_damage_but_the_hits_and_the_wall() {
        for name in [
            "cf02_contact_knockdown",
            "cf02_contact_below_threshold",
            "cf02_contact_cancels_swing",
            "cf02_no_chain",
            "cf02_body_bowl",
            "cf02_air_relaunch_contact",
            "cf02_wall_slam_kill",
            "cf02_wall_slam_body",
            "cf02_layout_edge_no_slam",
        ] {
            let sim = replay(name);
            let mut pairs: Vec<(u32, u32)> = contacts(&sim)
                .iter()
                .map(|c| (c.1, c.3))
                .chain(body_contacts(&sim).iter().map(|c| (c.1, c.3)))
                .collect();
            let n = pairs.len();
            pairs.sort_unstable();
            pairs.dedup();
            assert_eq!(pairs.len(), n, "{name}: a duplicate (launch, target)");
            let mut wall: Vec<(u32, u32)> = slams(&sim).iter().map(|c| (c.1, c.2)).collect();
            let m = wall.len();
            wall.sort_unstable();
            wall.dedup();
            assert_eq!(wall.len(), m, "{name}: a duplicate (launch, goblin) slam");
            // Health lost = the damage of the hits + the 1 damage of each living slam.
            let dealt: u32 = hits(&sim).iter().map(|h| u32::from(h.2)).sum();
            let slammed = events_of(&sim)
                .iter()
                .filter(|e| matches!(e, CombatEvent::WallSlam { .. }))
                .count() as u32;
            let lost: u32 = sim
                .combat
                .enemies
                .iter()
                .map(|g| u32::from(crate::combat::ENEMY_HEALTH.saturating_sub(g.health)))
                .sum();
            let living_slams = if name == "cf02_wall_slam_kill" {
                slammed
            } else {
                0
            };
            assert_eq!(lost, dealt + living_slams, "{name}: unexplained damage");
            assert!(!sim.combat.hero_defeated, "{name}");
        }
    }

    #[test]
    fn cine_a_body_contact_never_scores_the_contact_setup() {
        use crate::cine::PresentationSettings;
        use crate::cine::{CineEvent, CineTier};
        use crate::cine_adapter::CineAdapter;
        use crate::combat::DefeatCause;
        let body = CombatEvent::BodyContact {
            tick: 110,
            launch: 5,
            source: 9,
            target: 2,
            instigator: Some(0),
        };
        let slam = CombatEvent::WallSlam {
            tick: 111,
            launch: 5,
            goblin: 9,
            instigator: Some(0),
        };
        // Neither is mapped to a cinematic input at all.
        assert_eq!(CineEvent::from_combat(&body), None);
        assert_eq!(CineEvent::from_combat(&slam), None);
        // The t06 chain with the contact replaced by a BodyContact is not the
        // legendary contact setup: it never proves a launch.
        let chain = |contact: CombatEvent| {
            let mut a = CineAdapter::new(PresentationSettings::DEFAULT);
            let defeat = |tick, target, cause| {
                CombatEvent::enemy_defeated(tick, target, cause, Some(0), None)
            };
            for e in [
                CombatEvent::Hit {
                    tick: 100,
                    move_id: MoveId::PushKick,
                    instance: 5,
                    target: 9,
                    damage: 1,
                    health_left: 2,
                    hit_stop: 8,
                },
                contact,
                defeat(120, 1, DefeatCause::Melee),
                CombatEvent::ShoreBroken {
                    tick: 125,
                    player: 0,
                },
                defeat(150, 2, DefeatCause::ShoreBlock { player: 0 }),
                defeat(150, 3, DefeatCause::Melee),
            ] {
                a.push(&e);
            }
            a.flush()
        };
        let living = chain(CombatEvent::GoblinContact {
            tick: 110,
            launch: 5,
            source: 9,
            target: 2,
        });
        assert!(living[0].moment.feats.contact_setup);
        assert_eq!(living[0].moment.tier, CineTier::Legendary);
        let bowled = chain(body);
        assert!(
            bowled.iter().all(|e| !e.moment.feats.contact_setup),
            "{bowled:?}"
        );
    }

    #[test]
    fn the_cf02_b_states_are_not_hashed_while_idle() {
        // A goblin that never bowls hashes like one built before CF02-B.
        let mut a = crate::goblin::Goblin::new(100.0, 111.0, crate::combat::EnemyAi::Passive);
        let before = {
            let mut v = Vec::new();
            a.mix_into(&mut |x| v.push(x));
            v
        };
        a.wall_stop = Some(170.0);
        let mut after = Vec::new();
        a.mix_into(&mut |x| after.push(x));
        assert_eq!(before, after, "wall_stop is transient and never hashed");
        a.bowl = Some(3);
        let mut with = Vec::new();
        a.mix_into(&mut |x| with.push(x));
        assert_ne!(before, with);
    }

    // ------------------------------------------------------------ CF02-G --

    use crate::cf02::Temperament;
    use crate::combat::ReactionKind;
    use crate::morale::Reaction;

    /// `(tick, goblin, reaction, nerve)` of every `GoblinReaction`.
    fn reactions(sim: &crate::party_scene::PartyFixtureSim) -> Vec<(u64, u32, ReactionKind, u8)> {
        sim.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::GoblinReaction {
                    tick,
                    goblin,
                    reaction,
                    nerve,
                } => Some((tick, goblin, reaction, nerve)),
                _ => None,
            })
            .collect()
    }

    fn ticks_of(
        sim: &crate::party_scene::PartyFixtureSim,
        pick: fn(&CombatEvent) -> Option<(u64, u32)>,
    ) -> Vec<(u64, u32)> {
        sim.combat.events.iter().filter_map(pick).collect()
    }

    fn tells(sim: &crate::party_scene::PartyFixtureSim) -> Vec<(u64, u32)> {
        ticks_of(sim, |e| match *e {
            CombatEvent::GoblinTell { tick, goblin } => Some((tick, goblin)),
            _ => None,
        })
    }

    fn swings(sim: &crate::party_scene::PartyFixtureSim) -> Vec<(u64, u32)> {
        ticks_of(sim, |e| match *e {
            CombatEvent::GoblinSwing { tick, goblin } => Some((tick, goblin)),
            _ => None,
        })
    }

    /// The G fixtures with the temperament each goblin must draw (`'-'` = a dummy
    /// or irrelevant goblin).
    const G_DRAWS: [(&str, &str); 11] = [
        ("cf02_morale_witness_hesitate", "-SC"),
        ("cf02_morale_spectacle_panic", "-C-"),
        ("cf02_morale_panic_once", "--C"),
        ("cf02_morale_last_goblin", "-S"),
        ("cf02_morale_cornered", "-C"),
        ("cf02_morale_enraged_tell_unchanged", "BB"),
        ("cf02_coward_backoff_kick", "C"),
        ("cf02_coward_jab_still_hits", "C"),
        ("cf02_showoff_taunt_punish", "S"),
        ("cf02_pincer_token", "SCB"),
        ("cf02_restart_reshuffles", "BCB"),
    ];

    fn letter(t: Temperament) -> char {
        match t {
            Temperament::Brawler => 'B',
            Temperament::Coward => 'C',
            Temperament::ShowOff => 'S',
        }
    }

    #[test]
    fn q58_every_g_fixture_draws_the_temperaments_its_scenario_needs() {
        for (name, want) in G_DRAWS {
            let sim = replay_for(name, 1);
            for (i, w) in want.chars().enumerate() {
                let m = sim.combat.goblin_morale(i).expect(name);
                if w != '-' {
                    assert_eq!(letter(m.temperament()), w, "{name} goblin {i}");
                }
            }
        }
    }

    #[test]
    fn q60_q61_a_routine_death_makes_a_nearby_show_off_hesitate_for_20_ticks() {
        let mut sim = replay_for("cf02_morale_witness_hesitate", 20);
        // The haymaker kill is routine: nerve 62 - 20 = 42 < 50 (hesitate), not < 25.
        assert_eq!(reactions(&sim), vec![(20, 1, ReactionKind::Hesitate, 43)]);
        let x = sim.combat.enemies[1].x;
        let before = x;
        for _ in 0..20 {
            sim.step_scripted();
            assert_eq!(sim.combat.enemies[1].x, x, "tick {}", sim.combat.elapsed);
        }
        assert!(matches!(sim.combat.goblin_reaction(1), Reaction::Neutral));
        sim.step_scripted();
        assert!(sim.combat.enemies[1].x < before, "it walks again");
        // The goblin 380 px away did not witness anything, the dummy has no morale.
        assert!(reactions(&sim).iter().all(|r| r.1 == 1));
        // It rests at its own baseline: Coward 45 + its offset of 2.
        assert_eq!(sim.combat.goblin_morale(2).unwrap().nerve(), 47);
    }

    #[test]
    fn q60_q61_a_spectacular_death_makes_a_coward_panic_and_flee_without_attacking() {
        let mut sim = replay_for("cf02_morale_spectacle_panic", 43);
        // The lethal kick is heavy: 46 - 35 = 11 (< 25). The panic ended its tell.
        assert_eq!(reactions(&sim), vec![(43, 1, ReactionKind::Panic, 11)]);
        assert!(events_of(&sim).contains(&CombatEvent::GoblinInterrupted {
            tick: 43,
            goblin: 1
        }));
        let start_x = sim.combat.enemies[1].x;
        let mut ended = None;
        let mut max_run = 0.0f32;
        for k in 1..=100u64 {
            sim.step_scripted();
            let g = &sim.combat.enemies[1];
            if matches!(sim.combat.goblin_reaction(1), Reaction::Neutral) {
                ended = Some(k);
                break;
            }
            // It flees away from the event (the event is to its right) at 78 px/s.
            assert_eq!(g.facing, -1);
            max_run = max_run.max(start_x - g.x);
            assert!(g.mode == GoblinMode::Neutral, "no attack while panicking");
        }
        let ended = ended.expect("a panic always ends");
        assert!((60..=91).contains(&ended), "panic lasted {ended} ticks");
        assert!(max_run > 60.0, "it ran {max_run} px");
        // No tell of goblin 1 while the panic ran.
        assert!(tells(&sim).iter().all(|t| t.1 != 1 || t.0 < 40));
    }

    #[test]
    fn q61_panic_is_once_per_attempt_a_second_scare_only_hesitates() {
        let sim = replay("cf02_morale_panic_once");
        let r: Vec<_> = reactions(&sim).into_iter().filter(|r| r.1 == 2).collect();
        assert_eq!(
            r,
            vec![
                (43, 2, ReactionKind::Panic, 5),
                (165, 2, ReactionKind::Hesitate, 0)
            ]
        );
        assert!(sim.combat.goblin_morale(2).unwrap().panic_used());
    }

    #[test]
    fn q62_the_last_goblin_standing_panics_where_another_would_hesitate() {
        let last = replay("cf02_morale_last_goblin");
        let r = reactions(&last);
        // Same event, same Show-off, same nerve (43) as the hesitate fixture, but
        // nobody else is alive: panic (< 50), not hesitate.
        assert_eq!(r, vec![(20, 1, ReactionKind::Panic, 43)]);
        let not_last = replay("cf02_morale_witness_hesitate");
        assert_eq!(reactions(&not_last)[0].2, ReactionKind::Hesitate);
    }

    #[test]
    fn q63_a_cornered_goblin_ends_its_panic_with_nerve_40_and_fights() {
        let mut sim = replay_for("cf02_morale_cornered", 64);
        let r = reactions(&sim);
        assert_eq!(r[0], (20, 1, ReactionKind::Panic, 27));
        assert_eq!(r[1], (64, 1, ReactionKind::Cornered, 40));
        // Blocked by the block (x 1392, body 24 wide): it stopped at 1380.
        assert!((sim.combat.enemies[1].x - 1380.0).abs() < 1.5);
        assert_eq!(sim.combat.goblin_morale(1).unwrap().nerve(), 40);
        assert!(matches!(sim.combat.goblin_reaction(1), Reaction::Neutral));
        // It fights: it tells and swings after the panic.
        sim.run_scripted(200);
        assert!(tells(&sim).iter().any(|t| t.1 == 1 && t.0 > 64));
        // The panic (60..90 ticks) was cut short, 44 ticks after it began.
    }

    #[test]
    fn q61_an_enraged_brawler_walks_faster_and_rests_less_but_the_tell_stays_24_ticks() {
        let mut sim = replay_for("cf02_morale_enraged_tell_unchanged", 29);
        let t = crate::cf02::active();
        sim.step_scripted();
        // Tick 30: goblin 0 hurt Carl; the Brawler witness (70 + 1 + 15 = 86 >= 85)
        // is enraged. The hitter itself was in its swing (no reaction, Q68).
        assert_eq!(reactions(&sim), vec![(30, 1, ReactionKind::Enraged, 86)]);
        let m = sim.combat.goblin_morale(1).unwrap();
        assert!(matches!(m.reaction(), Reaction::Enraged { .. }));
        assert_eq!(m.walk_speed_px_s(t, 62.0), 81.0);
        assert_eq!(m.attack_cooldown(t, 40), 24);
        // The attack cooldown after goblin 1's attack is 24; goblin 0's is 40.
        let (mut was_attack, mut seen1, mut seen0) = ([false; 2], None, None);
        for _ in 0..200 {
            sim.step_scripted();
            for (i, seen) in [(0usize, &mut seen0), (1usize, &mut seen1)] {
                let g = &sim.combat.enemies[i];
                if was_attack[i] && g.mode == GoblinMode::Neutral && seen.is_none() {
                    *seen = Some(g.attack_cooldown);
                }
                was_attack[i] = g.mode == GoblinMode::Attack;
            }
        }
        assert_eq!(seen0, Some(40), "the goblin that did not enrage rests 40");
        assert_eq!(seen1, Some(24), "the enraged one rests 24");
        // Every tell is 24 ticks before the swing (swing event = tell + 26).
        for (tick, goblin) in tells(&sim) {
            let swing = swings(&sim)
                .iter()
                .any(|s| s.1 == goblin && s.0 == tick + 26);
            let cut = events_of(&sim).iter().any(
                |e| matches!(*e, CombatEvent::GoblinInterrupted { goblin: g, .. } if g == goblin),
            );
            assert!(
                swing || cut || tick + 26 > sim.combat.elapsed,
                "tell {tick} g{goblin}"
            );
        }
    }

    #[test]
    fn q64_a_coward_hops_back_28_px_over_10_ticks_out_of_a_kick_and_the_kick_whiffs() {
        let mut sim = replay_for("cf02_coward_backoff_kick", 4);
        sim.step_scripted();
        assert_eq!(reactions(&sim), vec![(5, 0, ReactionKind::BackOff, 43)]);
        // The first 2 px of the 28 were applied on tick 5; the other nine ticks
        // add the remaining 26 and the goblin walks nowhere meanwhile.
        let x5 = sim.combat.enemies[0].x;
        for _ in 0..9 {
            sim.step_scripted();
        }
        let hopped = sim.combat.enemies[0].x - x5;
        assert!((hopped - 26.0).abs() < 0.01, "hopped {hopped}");
        sim.run_scripted(80);
        // The kick (startup 13) found nothing; the jab-less Coward was not hit.
        assert!(hits(&sim).is_empty(), "{:?}", hits(&sim));
        assert_eq!(sim.combat.enemies[0].health, 2);
    }

    #[test]
    fn q64_the_jab_still_catches_a_coward() {
        let sim = replay("cf02_coward_jab_still_hits");
        assert!(reactions(&sim).is_empty());
        assert_eq!(hits(&sim), vec![(8, MoveId::Jab, 1, 1)]);
    }

    #[test]
    fn q65_a_show_off_taunts_for_36_ticks_after_a_whiff_and_a_hit_staggers_it_20() {
        let mut sim = replay_for("cf02_showoff_taunt_punish", 9);
        sim.step_scripted();
        // The jab (startup 3, active 2) left its active phase without a hit on 10.
        assert_eq!(reactions(&sim), vec![(10, 0, ReactionKind::Taunt, 57)]);
        let x = sim.combat.enemies[0].x;
        // It stands still while it taunts.
        let mut at_hit = None;
        for _ in 0..40 {
            sim.step_scripted();
            let g = &sim.combat.enemies[0];
            if g.mode == GoblinMode::Hurt && at_hit.is_none() {
                at_hit = Some((sim.combat.elapsed, g.stagger_left, g.vx));
            }
            if at_hit.is_none() {
                assert_eq!(g.x, x, "tick {}", sim.combat.elapsed);
            }
        }
        // The jab at 39 hit during the taunt: 20 ticks of stagger instead of 10.
        let (tick, stagger, _) = at_hit.unwrap();
        assert_eq!((tick, stagger), (39, 20));
        assert!(matches!(sim.combat.goblin_reaction(0), Reaction::Neutral));
        // The taunt did not change nerve (57 before and after the hit).
        assert_eq!(sim.combat.goblin_morale(0).unwrap().nerve(), 57);
    }

    #[test]
    fn q66_the_second_token_goes_to_the_other_side_of_the_first_holder_not_the_second_nearest() {
        let sim = replay_for("cf02_pincer_token", 60);
        let t = tells(&sim);
        // Goblin 1 (nearest, left) holds the first token, goblin 2 (right, the
        // other side) the second; goblin 0 (left, second nearest) waits.
        assert_eq!(t[0], (1, 1));
        assert_eq!(t[1].1, 2);
        assert!(t.iter().filter(|x| x.1 == 0).all(|x| x.0 > 60));
        // The legacy rule (morale off) gives it to the second-nearest on the left.
        let mut legacy = replay_for("cf02_pincer_token", 1);
        legacy.combat.set_morale_for_test(false);
        legacy.scene.reset();
        for _ in 0..60 {
            legacy.step_scripted();
        }
        let lt = tells(&legacy);
        assert!(lt.iter().any(|x| x.1 == 0 && x.0 < 20), "{lt:?}");
        assert!(lt.iter().all(|x| x.1 != 2 || x.0 > 20));
    }

    #[test]
    fn q68_no_morale_reaction_shortens_a_tell_and_a_reacting_goblin_takes_no_token() {
        for (name, _) in G_DRAWS {
            let sim = replay(name);
            let swings = swings(&sim);
            let interrupts: Vec<_> = events_of(&sim)
                .iter()
                .filter_map(|e| match *e {
                    CombatEvent::GoblinInterrupted { tick, goblin } => Some((tick, goblin)),
                    _ => None,
                })
                .collect();
            for (tick, goblin) in tells(&sim) {
                let swung = swings.contains(&(tick + 26, goblin));
                let cut = interrupts
                    .iter()
                    .any(|i| i.1 == goblin && i.0 > tick && i.0 < tick + 26);
                let open = tick + 26 > sim.combat.elapsed;
                // Only a hit, a contact or a panic ends a tell; none moves the swing.
                assert!(
                    swung || cut || open || sim.combat.restarts > 0,
                    "{name}: tell {tick} g{goblin}"
                );
            }
        }
        // A goblin in a reaction other than enraged is never approved.
        let mut sim = replay_for("cf02_morale_spectacle_panic", 44);
        for _ in 0..60 {
            sim.step_scripted();
            if !matches!(sim.combat.goblin_reaction(1), Reaction::Panic { .. }) {
                break;
            }
            assert!(!sim.combat.enemies[1].token_held());
        }
    }

    #[test]
    fn q58_a_goblin_in_its_first_approach_delay_takes_no_attack_token() {
        // Three goblins around Carl; goblin 1 (nearest) is held by a long delay: the
        // token must go to the others instead of idling in its slot.
        let mut sim = replay_for("cf02_pincer_token", 0);
        for i in 0..3 {
            sim.combat.set_delay_for_test(i, 0);
        }
        sim.combat.set_delay_for_test(1, 20);
        sim.run_scripted(30);
        let t = tells(&sim);
        assert!(t.iter().all(|x| x.1 != 1), "goblin 1 is held: {t:?}");
        // The nearest goblin that can tell (goblin 0, 35 px away) gets the token at
        // once; with the held goblin ranked first it would have waited.
        assert!(t.iter().any(|x| x.1 == 0 && x.0 <= 20), "{t:?}");
        // Once the delay is over it ranks (and tells) like any goblin.
        sim.run_scripted(60);
        assert!(tells(&sim).iter().any(|x| x.1 == 1));
    }

    #[test]
    fn q58_restarts_reshuffle_and_each_attempt_replays_identically() {
        let draws = |restarts: u32| -> Vec<char> {
            (0..3)
                .map(|g| {
                    letter(
                        crate::morale::draw_goblin(crate::cf02::active(), 0xCF02_0D00, restarts, g)
                            .temperament,
                    )
                })
                .collect()
        };
        assert_ne!(draws(0), draws(1), "restart 0 and 1 draw differently");
        let run = || {
            let fixture = find_party_fixture("cf02_restart_reshuffles").unwrap();
            let mut sim =
                crate::party_scene::PartyFixtureSim::new(&kit_of(fixture), 360, fixture).unwrap();
            let mut attempts = Vec::new();
            for _ in 0..3 {
                sim.run_scripted(140);
                let temps: Vec<char> = (0..3)
                    .map(|g| letter(sim.combat.goblin_morale(g).unwrap().temperament()))
                    .collect();
                attempts.push((temps, sim.combat.events.clone(), sim.state_hash()));
                sim.combat.restart(&mut sim.scene);
            }
            attempts
        };
        let (a, b) = (run(), run());
        assert_eq!(a, b, "every attempt replays identically");
        // The attempts' temperaments are the Q58 draws of their own `restarts`.
        for (k, attempt) in a.iter().enumerate() {
            assert_eq!(attempt.0, draws(k as u32), "attempt {k}");
        }
        let distinct: std::collections::BTreeSet<_> = a.iter().map(|x| x.0.clone()).collect();
        assert!(
            distinct.len() >= 2,
            "three attempts are not all the same draw"
        );
    }

    #[test]
    fn q58_the_seed_override_changes_the_draw_and_the_same_seed_repeats_it() {
        use crate::combat_scene::CombatState;
        let fixture = find_party_fixture("cf02_restart_reshuffles").unwrap();
        let kit = kit_of(fixture);
        let scene = crate::corridor::SceneSim::new_at(&kit, 360, 400).unwrap();
        let of = |seed: Option<u64>| -> Vec<char> {
            let c = CombatState::new(fixture.base, &kit, &scene)
                .unwrap()
                .with_cf02()
                .with_cf02_seed(seed);
            (0..3)
                .map(|g| letter(c.goblin_morale(g).unwrap().temperament()))
                .collect()
        };
        assert_eq!(of(Some(7)), of(Some(7)));
        let all: std::collections::BTreeSet<_> = (1..40u64).map(|s| of(Some(s))).collect();
        assert!(all.len() > 3, "different seeds draw different rooms");
        assert_eq!(of(None), of(Some(fixture.base.seed)));
    }

    #[test]
    fn cf02_morale_off_parity() {
        // Morale only exists under its own switch: with it off every G fixture
        // logs no reaction, hashes no morale and ranks the tokens the legacy way.
        for (name, _) in G_DRAWS {
            let fixture = find_party_fixture(name).unwrap();
            let run = || {
                let mut sim =
                    crate::party_scene::PartyFixtureSim::new(&kit_of(fixture), 360, fixture)
                        .unwrap();
                sim.combat.set_morale_for_test(false);
                sim.run_scripted(fixture.base.ticks);
                sim
            };
            let (a, b) = (run(), run());
            assert!(reactions(&a).is_empty(), "{name}");
            assert!(a.combat.goblin_morale(0).is_none(), "{name}");
            assert_eq!(a.state_hash(), b.state_hash(), "{name}");
            // And the hash does see the morale when it is on.
            let on = replay(name);
            assert_ne!(on.state_hash(), a.state_hash(), "{name}");
        }
        // The A, H and B fixtures run without morale: their pinned goldens (every
        // earlier test in this module) are the parity proof; none lists a reaction.
        for fixture in &CF02_FIXTURES[..36] {
            assert!(
                !crate::cf02_fixtures::morale_fixture(fixture.name()),
                "{}",
                fixture.name()
            );
        }
    }
}
