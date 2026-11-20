//! 02a-B tests: ObservationV2 schema, the v1 projection, initial masks per
//! seat, value checks and the AP-Q28 leakage tests.
//!
//! The leakage pairs call [`observe_v2`] directly on a `PartyFixtureSim`'s
//! `(scene, combat, party)`: the same entry point the live recorder uses.

use std::collections::BTreeSet;
use std::path::PathBuf;

use super::observation::*;
use super::scenario;
use super::*;
use crate::autoplay::bots::{BOT_IDS, make_bot};
use crate::autoplay::observation::{
    ENEMY_DAMAGED, OBS_V1, SELF_BUSY, SELF_HEALTH, SELF_LEN, SELF_MOVE_JAB, SELF_MOVE_PUSH_KICK,
    observe,
};
use crate::autoplay::{AutoplayConfig, ObservationV1};
use crate::boss_scene::Pilot;
use crate::combat::ENEMY_HEALTH;
use crate::combat_scene::{Fixture, FixtureSim};
use crate::donut_mana::DonutSetup;
use crate::env_kit::EnvKit;
use crate::input::{RawInput, SlotInputs};
use crate::party::{HeroId, PROVISIONAL_TUNING};
use crate::party_scene::{PartyFixture, PartyFixtureSim, base};

const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";
const FJB: &str = "follow_and_jump_bot";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn kit() -> EnvKit {
    EnvKit::load(&root(), LAYOUT).unwrap()
}

/// A roster-free stage with no hostiles and Donut's biscuit (8 mana): the quiet
/// stage the leakage pairs are built on (histories cannot disturb enemies).
static EMPTY_BASE: Fixture = base(
    "ap2a_test_empty",
    "test: no hostiles",
    0xA902_F001,
    &[],
    600,
);
static EMPTY: PartyFixture = PartyFixture {
    base: &EMPTY_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::unlocked(8),
};
/// The same stage with the biscuit still locked (Donut swats).
static EMPTY_LOCKED: PartyFixture = PartyFixture {
    base: &EMPTY_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

fn entry(seat: HeroId) -> SlotInputs {
    [
        RawInput {
            swap: seat == HeroId::Donut,
            ..RawInput::NONE
        },
        RawInput::NONE,
    ]
}

/// A sim after the seat-entry tick, like `reset` leaves it.
fn sim_of(fixture: &'static PartyFixture, seat: HeroId) -> PartyFixtureSim {
    let mut sim = PartyFixtureSim::new(&kit(), 360, fixture).unwrap();
    sim.step_slots(entry(seat));
    assert_eq!(sim.party.controlled_hero(), seat);
    sim
}

fn slot0(raw: RawInput) -> SlotInputs {
    [raw, RawInput::NONE]
}

fn run(move_x: f32) -> SlotInputs {
    slot0(RawInput {
        move_x,
        ..RawInput::NONE
    })
}

fn idle() -> SlotInputs {
    slot0(RawInput::NONE)
}

fn obs(sim: &PartyFixtureSim, seat: HeroId) -> ObservationV2 {
    observe_v2(&sim.scene, &sim.combat, &sim.party, seat).0
}

fn bits(sim: &PartyFixtureSim, seat: HeroId) -> Vec<u32> {
    obs(sim, seat).to_bits()
}

fn roster_fixture(id: &str) -> &'static PartyFixture {
    scenario::lookup(id, FJB).unwrap().fixture
}

/// A sim a few ticks into a roster fight (goblins in view and chasing).
fn engaged(id: &str, seat: HeroId, ticks: usize) -> PartyFixtureSim {
    let mut sim = sim_of(roster_fixture(id), seat);
    for _ in 0..ticks {
        sim.step_slots(idle());
    }
    sim
}

// ------------------------------------------------------------------ schema --

#[test]
fn the_schema_is_pinned_and_v1_is_a_prefix() {
    assert_eq!(OBSERVATION_VERSION_V2, 2);
    assert_eq!(OBS_V2.version, 2);
    assert_eq!((SELF_V2_LEN, ALLY_LEN), (22, 24));
    assert_eq!(&OBS_V2.self_names[..SELF_LEN], &OBS_V1.self_names);
    assert_eq!(&OBS_V2.self_bounds[..SELF_LEN], &OBS_V1.self_bounds);
    assert_eq!(&OBS_V2.ally_names[..SELF_V2_LEN], &OBS_V2.self_names);
    assert_eq!(&OBS_V2.ally_bounds[..SELF_V2_LEN], &OBS_V2.self_bounds);
    assert_eq!(OBS_V2.ally_names[ALLY_DX], "dx");
    assert_eq!(OBS_V2.ally_names[ALLY_DY], "dy");
    for (index, name) in [
        (SELF_IS_CARL, "is_carl"),
        (SELF_IS_DONUT, "is_donut"),
        (SELF_MOVE_SWAT, "move_swat"),
        (SELF_MOVE_POUNCE, "move_pounce"),
        (SELF_MOVE_MAGIC_MISSILE, "move_magic_missile"),
        (SELF_MANA, "mana"),
        (SELF_MAGIC_UNLOCKED, "magic_unlocked"),
        (SELF_CAN_CAST, "can_cast"),
    ] {
        assert_eq!(OBS_V2.self_names[index], name);
    }
    let unique: BTreeSet<_> = OBS_V2.self_names.iter().collect();
    assert_eq!(unique.len(), SELF_V2_LEN, "names are unique");
    // Plate features (kept off screen) are exactly the HUD-visible ones.
    let kept: Vec<&str> = ALLY_PLATE_FEATURES
        .iter()
        .map(|i| OBS_V2.ally_names[*i])
        .collect();
    assert_eq!(
        kept,
        [
            "health",
            "is_carl",
            "is_donut",
            "mana",
            "magic_unlocked",
            "can_cast"
        ]
    );
}

/// Every schema invariant of one observation.
fn check_obs(o: &ObservationV2, what: &str) {
    for (i, v) in o.own.iter().enumerate() {
        let (lo, hi) = OBS_V2.self_bounds[i];
        assert!(
            v.is_finite() && *v >= lo && *v <= hi,
            "{what}: own[{i}] = {v}"
        );
    }
    for (i, v) in o.ally.iter().enumerate() {
        let (lo, hi) = OBS_V2.ally_bounds[i];
        assert!(
            v.is_finite() && *v >= lo && *v <= hi,
            "{what}: ally[{i}] = {v}"
        );
    }
    assert!(o.ally_mask[0] == 0.0 || o.ally_mask[0] == 1.0);
    assert_eq!(o.own[SELF_IS_CARL] + o.own[SELF_IS_DONUT], 1.0, "{what}");
    assert_eq!(o.ally[SELF_IS_CARL] + o.ally[SELF_IS_DONUT], 1.0, "{what}");
    assert_ne!(o.own[SELF_IS_CARL], o.ally[SELF_IS_CARL], "{what}");
    if o.ally_mask[0] == 0.0 {
        for (i, v) in o.ally.iter().enumerate() {
            if !ALLY_PLATE_FEATURES.contains(&i) {
                assert_eq!(*v, 0.0, "{what}: masked ally[{i}] leaked");
            }
        }
    }
    for (slot, row) in o.enemies.iter().enumerate() {
        let on = o.enemy_mask[slot];
        assert!(on == 0.0 || on == 1.0);
        for (i, v) in row.iter().enumerate() {
            let (lo, hi) = OBS_V1.enemy_bounds[i];
            assert!(
                v.is_finite() && *v >= lo && *v <= hi,
                "{what}: enemy {slot}.{i}"
            );
            if on == 0.0 {
                assert_eq!(*v, 0.0, "{what}: masked enemy row {slot}");
            }
        }
    }
    assert!(o.hazards.iter().flatten().all(|v| *v == 0.0));
    assert!(o.hazard_mask.iter().all(|v| *v == 0.0));
    assert_eq!(o.available, [0.0, 0.0]);
    // Moves are Donut's xor Carl's, per hero.
    for block in [&o.own[..], &o.ally[..SELF_V2_LEN]] {
        if block[SELF_IS_CARL] == 1.0 {
            for i in [SELF_MOVE_SWAT, SELF_MOVE_POUNCE, SELF_MOVE_MAGIC_MISSILE] {
                assert_eq!(block[i], 0.0, "{what}: Carl with a Donut move");
            }
            assert_eq!(&block[SELF_MANA..=SELF_CAN_CAST], &[0.0, 0.0, 0.0]);
        } else {
            assert_eq!(
                &block[SELF_MOVE_JAB..=SELF_MOVE_PUSH_KICK],
                &[0.0, 0.0, 0.0],
                "{what}: Donut with a Carl move"
            );
        }
    }
}

#[test]
fn every_observation_in_play_obeys_the_schema_and_the_bots_see_it_as_v1() {
    for seat in HeroId::ALL {
        for entry in scenario::roster(FJB).unwrap() {
            for bot_id in BOT_IDS {
                let mut env =
                    PartyAutoplayEnv::new(AutoplayConfig::party_solo(entry.id, seat, FJB), &root())
                        .unwrap();
                env.reset(1).unwrap();
                let mut bot = make_bot(bot_id).unwrap();
                bot.reset(1);
                for _ in 0..400 {
                    let o = env.observe();
                    check_obs(&o, &format!("{} {seat:?} {bot_id}", entry.id));
                    let v1 = o.as_v1();
                    assert_eq!(&v1.own[..], &o.own[..SELF_LEN]);
                    let action = bot.act(&v1);
                    if env.step(action).unwrap().outcome.is_some() {
                        break;
                    }
                }
            }
        }
    }
}

#[test]
fn as_v1_keeps_the_self_prefix_and_the_unchanged_blocks() {
    let sim = engaged("ap2a_two_goblins_wide", HeroId::Donut, 20);
    let o = obs(&sim, HeroId::Donut);
    let v1 = o.as_v1();
    assert_eq!(v1.own.to_vec(), o.own[..SELF_LEN].to_vec());
    assert_eq!(v1.enemies, o.enemies);
    assert_eq!(v1.enemy_mask, o.enemy_mask);
    assert_eq!(v1.hazards, o.hazards);
    assert_eq!(v1.hazard_mask, o.hazard_mask);
    assert_eq!(v1.available, o.available);
    assert_eq!(v1.visible_enemies(), o.visible_enemies());
}

#[test]
fn on_a_fresh_sim_the_carl_view_equals_the_v1_observation() {
    // Same layout, no party effects yet: ObservationV2 for Carl projects to the
    // ObservationV1 of the plain fixture sim (same enemy ordering and rows).
    let kit = kit();
    for entry in scenario::roster(FJB).unwrap() {
        let plain = FixtureSim::new(&kit, 360, entry.fixture.base).unwrap();
        let party = PartyFixtureSim::new(&kit, 360, entry.fixture).unwrap();
        let v1: ObservationV1 = observe(&plain).0;
        let v2 = obs(&party, HeroId::Carl).as_v1();
        assert_eq!(v2.to_bits(), v1.to_bits(), "{}", entry.id);
    }
}

// ---------------------------------------------------------- initial masks --

#[test]
fn the_initial_masks_are_recorded_per_scenario_and_seat() {
    // The reset observation is post-entry; the camera follows the seat hero, so
    // the enemy mask can differ between seats (AP-Q25).
    let mut table = String::new();
    for entry in scenario::roster(FJB).unwrap() {
        for seat in HeroId::ALL {
            let mut env =
                PartyAutoplayEnv::new(AutoplayConfig::party_solo(entry.id, seat, FJB), &root())
                    .unwrap();
            env.reset(0).unwrap();
            let o = env.observe();
            let mask: String = o
                .enemy_mask
                .iter()
                .take(entry.fixture.base.enemies.len())
                .map(|m| if *m > 0.0 { '1' } else { '0' })
                .collect();
            // Independent of the builder: count enemies inside the sim camera.
            let sim = env.sim();
            let inside = sim
                .combat
                .enemies
                .iter()
                .filter(|g| g.alive() && !sim.scene.is_offscreen(g.x, 0.0))
                .count();
            assert_eq!(o.visible_enemies(), inside, "{} {seat:?}", entry.id);
            table.push_str(&format!(
                "{} {} enemies={} ally={}\n",
                entry.id,
                seat_name(seat),
                mask,
                o.ally_mask[0]
            ));
        }
    }
    assert_eq!(table, EXPECTED_INITIAL_MASKS);
}

/// `<scenario> <seat> enemies=<one flag per observation slot of the scenario's
/// enemy count, nearest first> ally=<ally mask>`: recorded from the first run.
/// Both seats see the same enemies at reset on the current roster, and only
/// `ap2a_single_goblin_far` starts with an empty enemy view.
const EXPECTED_INITIAL_MASKS: &str = "\
ap2a_single_goblin carl enemies=1 ally=1
ap2a_single_goblin donut enemies=1 ally=1
ap2a_two_goblins_wide carl enemies=11 ally=1
ap2a_two_goblins_wide donut enemies=11 ally=1
ap2a_two_goblins_wide_biscuit carl enemies=11 ally=1
ap2a_two_goblins_wide_biscuit donut enemies=11 ally=1
ap2a_two_goblins carl enemies=11 ally=1
ap2a_two_goblins donut enemies=11 ally=1
ap2a_single_goblin_far carl enemies=0 ally=1
ap2a_single_goblin_far donut enemies=0 ally=1
ap2a_two_goblins_biscuit carl enemies=11 ally=1
ap2a_two_goblins_biscuit donut enemies=11 ally=1
";

// ----------------------------------------------------------------- values --

#[test]
fn identity_health_and_mana_read_from_the_right_hero() {
    for seat in HeroId::ALL {
        let mut sim = sim_of(&EMPTY, seat);
        sim.scene.world.donut_body.health = 2;
        sim.scene.world.hero.health = 3;
        let o = obs(&sim, seat);
        let (own_health, ally_health) = match seat {
            HeroId::Carl => (0.75, 0.5),
            HeroId::Donut => (0.5, 0.75),
        };
        assert_eq!(o.own[SELF_HEALTH], own_health, "{seat:?}");
        assert_eq!(o.ally[SELF_HEALTH], ally_health, "{seat:?}");
        assert_eq!(o.own[SELF_IS_CARL], f32::from(seat == HeroId::Carl));
        // Donut's plate: 8 of 8 segments, unlocked, castable.
        let donut = if seat == HeroId::Donut {
            &o.own[..]
        } else {
            &o.ally[..22]
        };
        assert_eq!(&donut[SELF_MANA..=SELF_CAN_CAST], &[1.0, 1.0, 1.0]);
    }
    // Locked pool: nothing to show.
    let o = obs(&sim_of(&EMPTY_LOCKED, HeroId::Donut), HeroId::Donut);
    assert_eq!(&o.own[SELF_MANA..=SELF_CAN_CAST], &[0.0, 0.0, 0.0]);
}

#[test]
fn donuts_moves_and_cast_show_in_her_block_for_either_seat() {
    // Pounce (physical), Swat (primary, locked), Magic Missile (primary, biscuit).
    let press = |raw: RawInput| slot0(raw);
    let physical = RawInput {
        physical: true,
        ..RawInput::NONE
    };
    let attack = RawInput {
        attack: true,
        ..RawInput::NONE
    };
    let flags = |o: &ObservationV2, seat: HeroId| -> [f32; 5] {
        let b = if seat == HeroId::Donut {
            &o.own[..]
        } else {
            &o.ally[..22]
        };
        [
            b[7],
            b[SELF_MOVE_SWAT],
            b[SELF_MOVE_POUNCE],
            b[SELF_MOVE_MAGIC_MISSILE],
            b[SELF_MANA],
        ]
    };
    let mut seen = [false; 3];
    let mut sim = sim_of(&EMPTY_LOCKED, HeroId::Donut);
    for t in 0..14 {
        sim.step_slots(press(if t == 0 { physical } else { RawInput::NONE }));
        let f = flags(&obs(&sim, HeroId::Donut), HeroId::Donut);
        if f[2] == 1.0 {
            assert_eq!(f[0], 1.0, "a running pounce is busy");
            seen[0] = true;
        }
    }
    let mut sim = sim_of(&EMPTY_LOCKED, HeroId::Donut);
    for t in 0..14 {
        sim.step_slots(press(if t == 0 { attack } else { RawInput::NONE }));
        let f = flags(&obs(&sim, HeroId::Donut), HeroId::Donut);
        if f[1] == 1.0 {
            assert_eq!(f[0], 1.0);
            seen[1] = true;
        }
    }
    let mut sim = sim_of(&EMPTY, HeroId::Donut);
    let mut lowest_mana = 1.0_f32;
    for t in 0..30 {
        sim.step_slots(press(if t == 0 { attack } else { RawInput::NONE }));
        let f = flags(&obs(&sim, HeroId::Donut), HeroId::Donut);
        if f[3] == 1.0 {
            assert_eq!(f[0], 1.0, "a running cast is busy");
            seen[2] = true;
        }
        lowest_mana = lowest_mana.min(f[4]);
    }
    assert_eq!(seen, [true; 3], "pounce, swat and cast each showed");
    assert_eq!(
        lowest_mana,
        7.0 / 8.0,
        "the cast spent one of eight segments"
    );

    // Carl's jab shows in his own block.
    let mut sim = sim_of(&EMPTY, HeroId::Carl);
    let mut jab = false;
    for t in 0..14 {
        sim.step_slots(press(if t == 0 { attack } else { RawInput::NONE }));
        let o = obs(&sim, HeroId::Carl);
        jab |= o.own[SELF_MOVE_JAB] == 1.0 && o.own[SELF_BUSY] == 1.0;
    }
    assert!(jab);
}

#[test]
fn a_visible_ally_move_changes_the_observation_and_the_mask_tracks_the_camera() {
    for seat in HeroId::ALL {
        let ally = seat.other();
        let reference = sim_of(&EMPTY, seat);
        let base_bits = bits(&reference, seat);
        let mut moved = sim_of(&EMPTY, seat);
        let o0 = obs(&moved, seat);
        assert!(o0.ally_visible(), "{seat:?}: the follower starts in view");
        moved.scene.world.body_mut(ally).x += 20.0;
        assert_ne!(
            bits(&moved, seat),
            base_bits,
            "{seat:?}: a visible move shows"
        );
        let o1 = obs(&moved, seat);
        assert!((o1.ally[ALLY_DX] - o0.ally[ALLY_DX]).abs() > 0.01);
        // Far outside the view: masked.
        let left = moved.scene.camera.left() as f32;
        moved.scene.world.body_mut(ally).x = left - 50.0;
        assert!(!obs(&moved, seat).ally_visible());
    }
}

// ----------------------------------------------------- leakage (AP-Q28) --

/// Everything the policy may see of one hero, from `pub` views.
fn visible_state(sim: &PartyFixtureSim, hero: HeroId) -> Vec<u32> {
    let b = sim.scene.world.body(hero);
    let mut out: Vec<u32> = [b.x, b.y, b.vx, b.vy].iter().map(|v| v.to_bits()).collect();
    out.extend([
        u32::from(b.grounded),
        u32::from(b.health),
        u32::from(b.invulnerable_ticks > 0),
        u32::from(sim.combat.hurt_ticks_of(hero) > 0),
        b.facing as u32,
    ]);
    out
}

fn equal_visible_state(a: &PartyFixtureSim, b: &PartyFixtureSim) {
    for hero in HeroId::ALL {
        assert_eq!(
            visible_state(a, hero),
            visible_state(b, hero),
            "{hero:?} differs"
        );
    }
    assert_eq!(a.scene.camera.left(), b.scene.camera.left());
    assert_eq!(a.combat.donut_mana_view(), b.combat.donut_mana_view());
    assert_eq!(a.combat.donut_move_view(), b.combat.donut_move_view());
    assert_eq!(a.combat.donut_cast(), b.combat.donut_cast());
}

#[test]
fn leakage_a_the_followers_hidden_state_never_shows() {
    // Two input histories that end at equal visible state but differ inside the
    // follower (its trail and regroup state are private): the hashes differ, the
    // observations do not.
    for seat in HeroId::ALL {
        let mut direct = sim_of(&EMPTY, seat);
        let mut detour = sim_of(&EMPTY, seat);
        for _ in 0..40 {
            direct.step_slots(run(-1.0));
        }
        for _ in 0..30 {
            detour.step_slots(run(1.0));
        }
        for _ in 0..800 {
            detour.step_slots(run(-1.0));
        }
        for _ in 0..400 {
            direct.step_slots(run(-1.0));
        }
        for _ in 0..120 {
            direct.step_slots(idle());
            detour.step_slots(idle());
        }
        // A construction that did not reach equal visible state fails loudly.
        equal_visible_state(&direct, &detour);
        assert_ne!(
            direct.state_hash(),
            detour.state_hash(),
            "{seat:?}: the histories must differ in hidden state"
        );
        assert_eq!(bits(&direct, seat), bits(&detour, seat), "{seat:?}");
    }
}

#[test]
fn leakage_b_the_companion_x_while_off_screen_never_shows() {
    for seat in HeroId::ALL {
        let ally = seat.other();
        let mut sim = sim_of(&EMPTY, seat);
        let left = sim.scene.camera.left() as f32;
        let right = left + 640.0;
        let mut reference: Option<Vec<u32>> = None;
        for x in [
            left - 40.0,
            left - 300.0,
            left - 1000.0,
            right + 80.0,
            right + 900.0,
        ] {
            sim.scene.world.body_mut(ally).x = x;
            assert!(sim.scene.is_offscreen(x, 0.0), "{x}");
            let o = obs(&sim, seat);
            assert!(!o.ally_visible());
            check_obs(&o, "offscreen ally");
            // The plate stays: health, identity, mana, unlock, can_cast.
            assert_eq!(o.ally[SELF_IS_CARL], f32::from(ally == HeroId::Carl));
            assert_eq!(o.ally[SELF_HEALTH], 1.0);
            let b = o.to_bits();
            match &reference {
                None => reference = Some(b),
                Some(r) => assert_eq!(&b, r, "{seat:?}: off-screen x={x} leaked"),
            }
        }
    }
}

#[test]
fn leakage_c_invulnerable_ticks_show_only_as_a_flag() {
    for seat in HeroId::ALL {
        for hero in HeroId::ALL {
            let with = |ticks: u8| {
                let mut sim = sim_of(&EMPTY, seat);
                sim.scene.world.body_mut(hero).invulnerable_ticks = ticks;
                bits(&sim, seat)
            };
            assert_eq!(with(5), with(50), "{seat:?}/{hero:?}: only the flag shows");
            assert_ne!(with(0), with(5), "{seat:?}/{hero:?}: the flag itself shows");
        }
    }
}

/// Donut presses the primary once at tick `cast_at` (after the seat entry) and
/// the sim runs `total` ticks: the cast spends one mana and restarts the regen
/// timer at a time that depends on `cast_at`.
fn cast_history(cast_at: usize, total: usize) -> PartyFixtureSim {
    let mut sim = sim_of(&EMPTY, HeroId::Donut);
    for t in 0..total {
        let attack = t == cast_at;
        sim.step_slots(slot0(RawInput {
            attack,
            ..RawInput::NONE
        }));
    }
    sim
}

#[test]
fn leakage_d_the_mana_regen_timer_never_shows_for_the_seat() {
    let (a, b) = (cast_history(2, 24), cast_history(3, 24));
    let (mana_a, mana_b) = (a.combat.donut_mana_view(), b.combat.donut_mana_view());
    assert_eq!(mana_a, mana_b, "equal visible mana first");
    assert!(mana_a.is_some_and(|(m, max)| m < max), "mana is below full");
    equal_visible_state(&a, &b);
    assert_ne!(a.state_hash(), b.state_hash(), "the regen timers differ");
    assert_eq!(bits(&a, HeroId::Donut), bits(&b, HeroId::Donut));
}

/// Carl is the seat; Donut is swapped in to cast once and swapped out again, so
/// her regen timer depends on `cast_at` while Carl's view of her mana is equal.
fn ally_cast_history(cast_at: usize, total: usize) -> PartyFixtureSim {
    let mut sim = sim_of(&EMPTY, HeroId::Carl);
    for t in 0..total {
        let raw = RawInput {
            swap: t == 0 || t == 22,
            attack: t == cast_at,
            ..RawInput::NONE
        };
        sim.step_slots(slot0(raw));
    }
    assert_eq!(sim.party.controlled_hero(), HeroId::Carl, "swapped back");
    sim
}

#[test]
fn leakage_d_the_mana_regen_timer_never_shows_for_the_ally_either() {
    let (a, b) = (ally_cast_history(3, 28), ally_cast_history(4, 28));
    let (mana_a, mana_b) = (a.combat.donut_mana_view(), b.combat.donut_mana_view());
    assert_eq!(mana_a, mana_b, "equal visible mana first");
    assert!(
        mana_a.is_some_and(|(m, max)| m < max),
        "the cast spent mana"
    );
    equal_visible_state(&a, &b);
    assert_ne!(a.state_hash(), b.state_hash(), "the regen timers differ");
    assert_eq!(bits(&a, HeroId::Carl), bits(&b, HeroId::Carl));
}

#[test]
fn leakage_e_every_v1_hidden_state_pair_stays_hidden_for_both_seats() {
    for seat in HeroId::ALL {
        let reference = bits(&engaged("ap2a_single_goblin", seat, 30), seat);
        // Private enemy timers.
        for (cooldown, hurt, tick) in [(0u8, 0u8, 0u8), (25, 7, 3), (200, 99, 17)] {
            let mut sim = engaged("ap2a_single_goblin", seat, 30);
            let g = &mut sim.combat.enemies[0];
            if (g.attack_cooldown, g.hurt_cooldown) == (cooldown, hurt) {
                continue;
            }
            g.attack_cooldown = cooldown;
            g.hurt_cooldown = hurt;
            let _ = tick;
            assert_eq!(
                bits(&sim, seat),
                reference,
                "{seat:?}: timer ({cooldown}, {hurt})"
            );
        }
        // A health pair on the same side of the maximum: not damaged.
        let (mut a, mut b) = (
            engaged("ap2a_single_goblin", seat, 30),
            engaged("ap2a_single_goblin", seat, 30),
        );
        a.combat.enemies[0].health = ENEMY_HEALTH;
        b.combat.enemies[0].health = ENEMY_HEALTH + 3;
        assert_eq!(bits(&a, seat), bits(&b, seat), "{seat:?}");
        // The one intentional health bit: `damaged`, and only it.
        b.combat.enemies[0].health = ENEMY_HEALTH - 1;
        let (oa, mut ob) = (obs(&a, seat), obs(&b, seat));
        assert_eq!(ob.enemies[0][ENEMY_DAMAGED], 1.0);
        assert_eq!(oa.enemies[0][ENEMY_DAMAGED], 0.0);
        ob.enemies[0][ENEMY_DAMAGED] = 0.0;
        assert_eq!(ob, oa, "{seat:?}: nothing else moves with enemy health");
        // Remaining hurt-flinch counts are rule knowledge (a flag, not a count).
        let (mut a, mut b) = (
            engaged("ap2a_single_goblin", seat, 30),
            engaged("ap2a_single_goblin", seat, 30),
        );
        a.combat.hurt_ticks = 5;
        b.combat.hurt_ticks = 40;
        a.combat.donut.hurt_ticks = 5;
        b.combat.donut.hurt_ticks = 40;
        assert_eq!(bits(&a, seat), bits(&b, seat), "{seat:?}");
        // An enemy outside the view: position, health and timers all hidden.
        let mut base_sim = engaged("ap2a_single_goblin", seat, 0);
        let right = base_sim.scene.camera.left() as f32 + 640.0;
        base_sim.combat.enemies[0].x = right + 100.0;
        let off_reference = bits(&base_sim, seat);
        for (dx, health, cooldown) in [(300.0, 1u8, 0u8), (900.0, 2, 40), (2000.0, 2, 255)] {
            let mut other = engaged("ap2a_single_goblin", seat, 0);
            let g = &mut other.combat.enemies[0];
            g.x = right + dx;
            g.health = health;
            g.attack_cooldown = cooldown;
            g.attack_tick = 17;
            assert!(other.scene.is_offscreen(right + dx, 0.0));
            assert_eq!(
                bits(&other, seat),
                off_reference,
                "{seat:?}: off-screen leaked"
            );
        }
    }
}

// ------------------------------------------------------------------ source --

/// The code of `observation.rs` (comments removed), the only file the
/// ObservationV2 features come from.
fn observation_code() -> String {
    let path = root().join("src/autoplay/party/observation.rs");
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_builder_never_names_the_companion_modules_or_private_ai_state() {
    let code = observation_code();
    for banned in [
        "crate::companion",
        "crate::combat_scene::companion",
        "CompanionView",
        "CompanionDirector",
        "ProfileSet",
        "KillPolicy",
        "Decision",
        "bot_offscreen",
        "regroup",
        "trail",
        "dodge",
        "ai_started",
        "can_start_move",
        "kill_policy",
        "uncontrolled_hero",
        "regen_timer",
        "attack_cooldown",
        "hurt_cooldown",
        "attack_tick",
    ] {
        assert!(!code.contains(banned), "observation.rs names {banned}");
    }
    // The builder takes the three sim parts and the hero (PT-Q14).
    let signature = code
        .split("pub fn observe_v2(")
        .nth(1)
        .and_then(|s| s.split(") ->").next())
        .expect("observe_v2 signature");
    for part in ["&SceneSim", "&CombatState", "&PartyState", "HeroId"] {
        assert!(signature.contains(part), "observe_v2 takes {part}");
    }
    assert!(!signature.contains("PartyFixtureSim"));
}
