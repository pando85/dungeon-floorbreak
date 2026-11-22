//! CORRAL-B core unit tests: the pin, the machine, the fairness tables of the
//! sheet (section 6), the F10 hit table, the leash clamp, the brain and the AI
//! lane. The geometry sweep of section 10 is in `geometry_tests.rs`.

use super::*;
use crate::combat::EnemyAi;
use crate::input::{Actions, InputFrame};
use crate::llama::{LLAMA_HALF_HEIGHT, LLAMA_HALF_WIDTH, Llama, LlamaMode};
use crate::prototype::{BodyDrive, HERO_HALF_HEIGHT, Hero, HeroMotion, Material, step_hero_motion};
use serde_json::Value;

pub(super) fn profile() -> RushProfile {
    RushProfile::pinned().unwrap()
}

const WIDE: Leash = Leash {
    min: -100_000.0,
    max: 100_000.0,
};
const EDGE: f32 = 100_000.0;
const VIEW: FireView = FireView {
    left: 0.0,
    right: 100_000.0,
};

pub(super) fn solid(left: f32, width: f32, height: f32) -> Block {
    Block {
        rect: Rect {
            left,
            bottom: 0.0,
            width,
            height,
        },
        material: Material::Stone,
        intact: true,
    }
}

fn floor() -> Block {
    Block {
        rect: Rect {
            left: -50_000.0,
            bottom: -100.0,
            width: 150_000.0,
            height: 100.0,
        },
        material: Material::Stone,
        intact: true,
    }
}

// ------------------------------------------------------------------ the file --

#[test]
fn the_file_is_pinned_and_round_trips() {
    assert_eq!(content_hash(TUNING_JSON), TUNING_HASH);
    let p = profile();
    let original: Value = serde_json::from_str(TUNING_JSON).unwrap();
    assert_eq!(serde_json::to_value(&p).unwrap(), original);
    let drifted = TUNING_JSON.replace("\"crlq14_damage\": 1", "\"crlq14_damage\": 2");
    match RushProfile::from_json_pinned(&drifted, TUNING_HASH) {
        Err(RushProfileError::HashMismatch { expected, .. }) => assert_eq!(expected, TUNING_HASH),
        other => panic!("a drifted file must fail the pin: {other:?}"),
    }
    assert_eq!(active(), &p);
}

#[test]
fn the_adopted_values_are_the_sheets() {
    let p = profile();
    assert_eq!(p.crlq04_walk_px_s, 40);
    assert_eq!(p.crlq04_rush_approach_px, 112);
    assert_eq!(p.crlq04_spit_standoff_px, 200);
    assert_eq!(p.crlq05_trigger_px, 128);
    assert_eq!(p.crlq06_tell_ticks, 32);
    assert_eq!(p.crlq07_speed_px_s, 240);
    assert_eq!(p.crlq07_active_ticks, 24);
    assert_eq!(
        p.crlq08_box,
        BoxPx {
            x0: 0,
            x1: 28,
            y0: 2,
            y1: 30
        }
    );
    assert_eq!((p.crlq09_recover_ticks, p.crlq10_bonk_ticks), (40, 60));
    assert_eq!(p.crlq10_self_damage, 0);
    assert_eq!((p.crlq12_rush_cooldown, p.crlq12_spit_gap), (150, 30));
    assert_eq!(p.crlq13_interrupt_cooldown, 30);
    assert_eq!(
        (p.crlq14_damage, p.crlq14_knock_x, p.crlq14_knock_y),
        (1, 120, 160)
    );
    assert_eq!(p.crlq15_target_range_px, 320);
    assert_eq!(p.crlq16_floor_tolerance_px, 8);
    assert!(!p.crlq18_rush_evadable);
    assert_eq!(p.crlq24_wake_px, 320);
    assert!(p.crlq26_exit_gate && p.crlq40_enabled);
    assert_eq!(LLAMA_RUSH_SOURCE_ID, 0x4C52);
    // 24 active ticks at 4 px a tick: 96 px.
    assert_eq!(p.px_per_tick(), 4.0);
    assert_eq!(p.rush_distance(), 96.0);
    assert_eq!(p.standing_reach(DONUT_HALF_WIDTH), 134.0);
    assert_eq!(p.standing_reach(CARL_HALF_WIDTH), 136.0);
}

#[test]
fn every_question_has_a_knob_or_a_recorded_reason() {
    let value: Value = serde_json::from_str(TUNING_JSON).unwrap();
    let keys: Vec<&String> = value.as_object().unwrap().keys().collect();
    let exempt: Vec<u8> = NO_KNOB.iter().map(|(q, _)| *q).collect();
    assert_eq!(exempt.len(), NO_KNOB.len(), "no duplicate in NO_KNOB");
    for q in 1..=43u8 {
        let prefix = format!("crlq{q:02}_");
        let has_key = keys.iter().any(|k| k.starts_with(&prefix));
        if exempt.contains(&q) {
            assert!(!has_key, "CRL-Q{q:02} is listed as knob-less but has a key");
        } else {
            assert!(has_key, "CRL-Q{q:02} has no knob and is not in NO_KNOB");
        }
    }
    for key in keys {
        let named = key.starts_with("crlq")
            && key[4..6].parse::<u8>().is_ok_and(|q| (1..=43).contains(&q))
            && key.as_bytes()[6] == b'_';
        assert!(
            named || key == "schema_version",
            "{key} is not named after a CRL question id"
        );
    }
}

#[test]
fn a_bad_value_names_its_field() {
    let cases = [
        (
            "\"schema_version\": 1",
            "\"schema_version\": 2",
            "schema_version",
        ),
        (
            "\"crlq07_speed_px_s\": 240",
            "\"crlq07_speed_px_s\": 300",
            "crlq07_speed_px_s",
        ),
        (
            "\"crlq07_speed_px_s\": 240",
            "\"crlq07_speed_px_s\": 0",
            "crlq07_speed_px_s",
        ),
        (
            "\"crlq07_speed_px_s\": 240",
            "\"crlq07_speed_px_s\": 100",
            "crlq07_speed_px_s",
        ),
        (
            "\"crlq04_rush_approach_px\": 112",
            "\"crlq04_rush_approach_px\": 140",
            "crlq04_rush_approach_px",
        ),
        // 128 -> 134 is not below Donut's standing reach (134).
        (
            "\"crlq05_trigger_px\": 128",
            "\"crlq05_trigger_px\": 134",
            "crlq05_trigger_px",
        ),
        ("\"x0\": 0", "\"x0\": 30", "crlq08_box"),
        ("\"y1\": 30", "\"y1\": 40", "crlq08_box"),
        ("\"y0\": 2", "\"y0\": -2", "crlq08_box"),
        (
            "\"crlq06_tell_ticks\": 32",
            "\"crlq06_tell_ticks\": 0",
            "crlq06_tell_ticks",
        ),
        (
            "\"crlq07_active_ticks\": 24",
            "\"crlq07_active_ticks\": 0",
            "crlq07_active_ticks",
        ),
        (
            "\"crlq09_recover_ticks\": 40",
            "\"crlq09_recover_ticks\": 0",
            "crlq09_recover_ticks",
        ),
        (
            "\"crlq10_bonk_ticks\": 60",
            "\"crlq10_bonk_ticks\": 0",
            "crlq10_bonk_ticks",
        ),
    ];
    for (from, to, field) in cases {
        assert!(TUNING_JSON.contains(from), "{from}");
        let text = TUNING_JSON.replace(from, to);
        match RushProfile::from_json(&text) {
            Err(RushProfileError::Field { field: f, .. }) => assert_eq!(f, field, "{from}"),
            other => panic!("{field}: {other:?}"),
        }
    }
    // An unknown key is a parse error.
    let extra = TUNING_JSON.replace("\"schema_version\": 1,", "\"schema_version\": 1, \"x\": 1,");
    assert!(matches!(
        RushProfile::from_json(&extra),
        Err(RushProfileError::Parse(_))
    ));
}

#[test]
fn the_kill_switch_turns_every_switch_off() {
    let mut p = profile();
    assert!(p.on(p.crlq11_spit_height_check));
    assert!(p.on(p.crlq26_exit_gate));
    p.crlq40_enabled = false;
    assert!(!p.on(true));
    assert!(!p.enabled());
}

// ------------------------------------------------------------------ machine --

struct Run {
    /// Attack tick of the commit.
    commit: Option<u16>,
    /// `(attack tick, end)` of the end event.
    end: Option<(u16, RushEnd)>,
    blow: Option<(u16, RushBlow)>,
    /// Attack tick on which the machine finished.
    finished: Option<u16>,
    x: f32,
    state: RushState,
}

/// Run a rush from pivot `x0` with a fresh state against fixed `heroes`.
#[allow(clippy::too_many_arguments)]
fn run_fixed(
    p: &RushProfile,
    x0: f32,
    dir: i8,
    solids: &[Block],
    partition: Option<usize>,
    leash: Leash,
    heroes: &[RushHero],
    ticks: u16,
) -> Run {
    let mut state = RushState::start(x0, dir, HeroId::Carl, 1);
    let mut x = x0;
    let mut run = Run {
        commit: None,
        end: None,
        blow: None,
        finished: None,
        x,
        state,
    };
    for _ in 0..ticks {
        let world = RushWorld {
            feet: 0.0,
            solids,
            partition,
            leash,
            max_x: EDGE,
            heroes,
        };
        let out = state.step(p, x, &world);
        x = out.x;
        if out.commit {
            run.commit = Some(state.tick);
        }
        if let Some(end) = out.end {
            run.end = Some((state.tick, end));
        }
        if let Some(blow) = out.blow {
            run.blow = Some((state.tick, blow));
        }
        if out.finished {
            run.finished = Some(state.tick);
            break;
        }
    }
    run.x = x;
    run.state = state;
    run
}

/// A hero's hurtbox `(half width, height)`, read from its sources: Carl's is
/// his 24 x 52 collider, Donut's the shipped party tuning's hurtbox.
pub(super) fn hurtbox_of(id: HeroId) -> (f32, f32) {
    match id {
        HeroId::Carl => (CARL_HALF_WIDTH, 2.0 * HERO_HALF_HEIGHT),
        HeroId::Donut => {
            let (w, h) = crate::party::PROVISIONAL_TUNING.donut_hurtbox.size();
            (w / 2.0, h)
        }
    }
}

/// A hero's motion, read from its sources: Carl's `HeroMotion::CARL`, Donut's
/// agile motion, and with `boost` the DP-A Light on Your Feet launch.
pub(super) fn motion_of(id: HeroId, boost: bool) -> HeroMotion {
    match id {
        HeroId::Carl => HeroMotion::CARL,
        HeroId::Donut => {
            let mut motion = crate::agility::agile_motion();
            if boost {
                motion.jump_speed =
                    f32::from(crate::donut_prog::DonutProgTuning::active().dpq14_boost_launch_px_s);
            }
            motion
        }
    }
}

/// The sheet's section 6 and 10 tables are computed from these numbers. If a
/// source is retuned, the tables (and this test) must be revisited on purpose.
#[test]
fn the_physics_behind_the_sheets_tables_is_still_the_sheets() {
    use crate::prototype::GRAVITY;
    let (carl, donut, boosted) = (
        motion_of(HeroId::Carl, false),
        motion_of(HeroId::Donut, false),
        motion_of(HeroId::Donut, true),
    );
    assert_eq!((carl.run_speed, carl.jump_speed), (185.0, 440.0));
    assert_eq!((donut.run_speed, donut.jump_speed), (207.0, 500.0));
    assert_eq!(boosted.jump_speed, 547.0);
    assert_eq!(GRAVITY, 1050.0);
    assert_eq!(hurtbox_of(HeroId::Carl), (12.0, 52.0));
    assert_eq!(hurtbox_of(HeroId::Donut), (10.0, 24.0));
    assert_eq!((CARL_HALF_WIDTH, DONUT_HALF_WIDTH), (12.0, 10.0));
    // The bolt: 6 px a tick, 240 px of range, spawned 27 px ahead and 19 up.
    let mana = crate::donut_mana::ManaTuning::pinned(true).unwrap();
    assert_eq!(mana.projectile_speed_px_per_tick, 6);
    assert_eq!(mana.projectile_range_px, 240);
    assert_eq!(crate::combat_scene::BOLT_SPAWN_FORWARD, 27.0);
    assert_eq!(crate::combat_scene::BOLT_SPAWN_UP, 19.0);
}

fn hero_at(id: HeroId, x: f32) -> RushHero {
    let (half, height) = hurtbox_of(id);
    RushHero {
        id,
        hurtbox: Rect {
            left: x - half,
            bottom: 0.0,
            width: half * 2.0,
            height,
        },
        hittable: true,
    }
}

#[test]
fn the_machine_runs_tell_commit_rush_recover_then_finishes() {
    let p = profile();
    let mut state = RushState::start(600.0, -1, HeroId::Donut, 7);
    assert_eq!((state.phase, state.tick), (RushPhase::Tell, 0));
    let mut x = 600.0;
    let mut phases = Vec::new();
    let mut events = Vec::new();
    for _ in 0..200 {
        let world = RushWorld {
            feet: 0.0,
            solids: &[],
            partition: None,
            leash: WIDE,
            max_x: EDGE,
            heroes: &[],
        };
        let out = state.step(&p, x, &world);
        x = out.x;
        phases.push((state.tick, state.phase));
        if out.commit {
            events.push((state.tick, "commit"));
        }
        if let Some(end) = out.end {
            assert_eq!(end, RushEnd::Miss);
            events.push((state.tick, "miss"));
        }
        if out.finished {
            events.push((state.tick, "finished"));
            break;
        }
    }
    // The tell is attack ticks 0..31, the commit is attack tick 32, 24 active
    // ticks end on attack tick 55, Recover covers 56..95.
    assert_eq!(events, vec![(32, "commit"), (55, "miss"), (95, "finished")]);
    let at = |t: u16| phases.iter().find(|(tick, _)| *tick == t).unwrap().1;
    assert_eq!(at(1), RushPhase::Tell);
    assert_eq!(at(31), RushPhase::Tell);
    assert_eq!(at(32), RushPhase::Committed);
    assert_eq!(at(54), RushPhase::Committed);
    assert_eq!(at(55), RushPhase::Recover);
    assert_eq!(at(95), RushPhase::Recover);
    // The llama moved 96 px along its locked direction, none during the tell.
    assert_eq!(x, 600.0 - 96.0);
    assert_eq!(state.target, HeroId::Donut);
    assert_eq!(state.seq, 7);
}

#[test]
fn the_phase_mapping_is_the_one_kind_aware_function() {
    assert_eq!(RushPhase::Tell.target_state(), TargetState::Tell);
    assert_eq!(RushPhase::Committed.target_state(), TargetState::Impact);
    assert_eq!(RushPhase::Recover.target_state(), TargetState::Recovery);
    assert_eq!(RushPhase::Bonk.target_state(), TargetState::Recovery);
    assert!(RushPhase::Committed.has_box());
    assert!(!RushPhase::Tell.has_box() && !RushPhase::Recover.has_box());
    // Only the tell is a tell: a hit during it cancels the rush.
    let mut s = RushState::start(0.0, 1, HeroId::Carl, 1);
    assert!(s.telling());
    s.phase = RushPhase::Committed;
    assert!(!s.telling());
}

#[test]
fn nothing_moves_and_nothing_is_hit_during_the_tell() {
    let p = profile();
    // A hero inside the box's reach all the time, from attack tick 0.
    let heroes = [hero_at(HeroId::Carl, 624.0)];
    let mut state = RushState::start(600.0, 1, HeroId::Carl, 1);
    for _ in 1..p.crlq06_tell_ticks {
        let world = RushWorld {
            feet: 0.0,
            solids: &[],
            partition: None,
            leash: WIDE,
            max_x: EDGE,
            heroes: &heroes,
        };
        let out = state.step(&p, 600.0, &world);
        assert_eq!(out.x, 600.0);
        assert!(out.blow.is_none() && out.end.is_none() && !out.commit);
        assert_eq!(state.phase, RushPhase::Tell);
    }
}

#[test]
fn contact_hits_once_on_the_expected_tick_and_hands_over_the_blow() {
    let p = profile();
    // The fixture `corral_rush_hits_idle_donut`, mirrored: the llama at 600
    // facing -1, Donut at d 96 (x 504).
    let heroes = [hero_at(HeroId::Donut, 504.0)];
    let run = run_fixed(&p, 600.0, -1, &[], None, WIDE, &heroes, 200);
    // Active tick 15 is attack tick 46, the llama at 540.
    let (tick, blow) = run.blow.unwrap();
    assert_eq!(tick, 46);
    assert_eq!(run.end, Some((46, RushEnd::Contact(HeroId::Donut))));
    assert_eq!(blow.hero, HeroId::Donut);
    assert_eq!(blow.source, LLAMA_RUSH_SOURCE_ID);
    assert_eq!(blow.damage, 1);
    assert_eq!(blow.from_x, 540.0);
    assert_eq!((blow.knock_vx, blow.knock_vy), (-120.0, 160.0));
    assert_eq!(blow.instance, 1);
    // The rush stops where it hit, then Recover runs 40 ticks.
    assert_eq!(run.x, 540.0);
    assert_eq!(run.finished, Some(46 + 40));
    // Carl at the same distance is hit on the same tick (the box is the same).
    let heroes = [hero_at(HeroId::Carl, 504.0)];
    let carl = run_fixed(&p, 600.0, -1, &[], None, WIDE, &heroes, 200);
    assert_eq!(carl.blow.unwrap().0, 46);
    // Point-blank: contact on active tick 1 (attack tick 32).
    let heroes = [hero_at(HeroId::Donut, 576.0)];
    let near = run_fixed(&p, 600.0, -1, &[], None, WIDE, &heroes, 200);
    assert_eq!(near.blow.unwrap().0, 32);
}

#[test]
fn one_blow_per_instance_and_the_first_hero_only() {
    let p = profile();
    // Carl d 72 and Donut d 96 in the lane (`corral_rush_2p_one_hit`).
    let heroes = [hero_at(HeroId::Carl, 528.0), hero_at(HeroId::Donut, 504.0)];
    let run = run_fixed(&p, 600.0, -1, &[], None, WIDE, &heroes, 200);
    assert_eq!(run.blow.unwrap().1.hero, HeroId::Carl);
    // Recover follows the contact and no second blow ever comes out of it.
    assert_eq!(run.state.phase, RushPhase::Recover);
    let mut state = RushState::start(600.0, -1, HeroId::Carl, 1);
    let mut x = 600.0;
    let mut blows = 0;
    for _ in 0..200 {
        let world = RushWorld {
            feet: 0.0,
            solids: &[],
            partition: None,
            leash: WIDE,
            max_x: EDGE,
            heroes: &heroes,
        };
        let out = state.step(&p, x, &world);
        x = out.x;
        blows += usize::from(out.blow.is_some());
        if out.finished {
            break;
        }
    }
    assert_eq!(blows, 1);
}

#[test]
fn an_invulnerable_hero_is_passed_through_and_the_next_one_is_hit() {
    let p = profile();
    // `corral_rush_invulnerable_pass`: Carl invulnerable at d 60, Donut d 96.
    let mut carl = hero_at(HeroId::Carl, 540.0);
    carl.hittable = false;
    let heroes = [carl, hero_at(HeroId::Donut, 504.0)];
    let run = run_fixed(&p, 600.0, -1, &[], None, WIDE, &heroes, 200);
    assert_eq!(run.blow.unwrap().1.hero, HeroId::Donut);
    // Only an invulnerable hero: the rush runs its whole distance.
    let run = run_fixed(&p, 600.0, -1, &[], None, WIDE, &[carl], 200);
    assert!(run.blow.is_none());
    assert_eq!(run.end.unwrap().1, RushEnd::Miss);
    assert_eq!(run.x, 504.0);
}

#[test]
fn on_a_simultaneous_overlap_the_hero_the_front_passed_first_takes_it_ties_to_carl() {
    let p = profile();
    // CRL-Q14 ruling: of two heroes in the box on one tick, the one the box
    // front passed first (the one nearest the llama, the largest penetration)
    // takes the blow; exact ties go to Carl.
    let boxed = rush_box(&p, 12.0, 1, 0.0); // x 12..40, front at 40
    let hero = |id, left: f32, width, height| RushHero {
        id,
        hurtbox: Rect {
            left,
            bottom: 0.0,
            width,
            height,
        },
        hittable: true,
    };
    // Carl's near edge 36 (penetration 4), Donut's 39 (penetration 1): Carl was
    // reached first, whatever the slice order.
    let carl = hero(HeroId::Carl, 36.0, 24.0, 52.0);
    let donut = hero(HeroId::Donut, 39.0, 20.0, 24.0);
    assert_eq!(
        first_hero_in_box(boxed, 1, &[carl, donut]),
        Some(HeroId::Carl)
    );
    assert_eq!(
        first_hero_in_box(boxed, 1, &[donut, carl]),
        Some(HeroId::Carl)
    );
    // The other way round: Donut deeper (near edge 30) than Carl (38): Donut.
    let deep_donut = hero(HeroId::Donut, 30.0, 20.0, 24.0);
    let shallow_carl = hero(HeroId::Carl, 38.0, 24.0, 52.0);
    assert_eq!(
        first_hero_in_box(boxed, 1, &[shallow_carl, deep_donut]),
        Some(HeroId::Donut)
    );
    // A hero standing deep inside the box does not lose to one just inside the
    // front (the literal `abs` gap would have picked the shallow one).
    let deepest = hero(HeroId::Donut, 14.0, 20.0, 24.0);
    assert_eq!(
        first_hero_in_box(boxed, 1, &[carl, deepest]),
        Some(HeroId::Donut)
    );
    // Equal penetration: Carl.
    let donut_same = hero(HeroId::Donut, 36.0, 20.0, 24.0);
    assert_eq!(
        first_hero_in_box(boxed, 1, &[donut_same, carl]),
        Some(HeroId::Carl)
    );
    // Facing left mirrors: the box x -12..-40, front at -40; "near edge" is the
    // hurtbox's right edge. Carl's right edge -36 (penetration 4), Donut's -39.
    let boxed_left = rush_box(&p, -12.0, -1, 0.0);
    let carl_l = hero(HeroId::Carl, -60.0, 24.0, 52.0);
    let donut_l = hero(HeroId::Donut, -59.0, 20.0, 24.0);
    assert_eq!(carl_l.hurtbox.right(), -36.0);
    assert_eq!(
        first_hero_in_box(boxed_left, -1, &[donut_l, carl_l]),
        Some(HeroId::Carl)
    );
    // An unhittable hero never takes it, and nobody in the box is `None`.
    let mut ghost = carl;
    ghost.hittable = false;
    assert_eq!(
        first_hero_in_box(boxed, 1, &[ghost, donut]),
        Some(HeroId::Donut)
    );
    assert_eq!(first_hero_in_box(boxed, 1, &[]), None);
}

#[test]
fn a_rush_never_tracks_after_the_tell_and_never_turns() {
    let p = profile();
    // `corral_rush_no_tracking`: the hero jumps over and lands behind the llama.
    let mut state = RushState::start(600.0, -1, HeroId::Donut, 1);
    let (dir, target, start_x) = (state.dir, state.target, state.start_x);
    let mut x = 600.0;
    let mut last = x;
    for t in 1..=100u16 {
        // The hero stands wherever: ahead of the rush, then far behind it.
        let hero_x = if t < 40 { 504.0 } else { 900.0 };
        let heroes = [hero_at(HeroId::Donut, hero_x)];
        let world = RushWorld {
            feet: 0.0,
            solids: &[],
            partition: None,
            leash: WIDE,
            max_x: EDGE,
            heroes: &heroes,
        };
        let out = state.step(&p, x, &world);
        assert!(out.x <= last, "the llama never moves backward or turns");
        last = out.x;
        x = out.x;
        assert_eq!(
            (state.dir, state.target, state.start_x),
            (dir, target, start_x)
        );
        if out.finished {
            break;
        }
    }
}

fn faced(face: f32) -> Vec<Block> {
    vec![solid(face, 16.0, 200.0)]
}

#[test]
fn the_bonk_ticks_and_stop_columns_follow_the_sheet() {
    let p = profile();
    // `bonk_tick`: with a face F px ahead of the start pivot the box front rests
    // on it; no bonk when F >= 124.
    for (face, tick) in [
        (24.0, Some(1)),
        (56.0, Some(8)),
        (84.0, Some(15)),
        (92.0, Some(17)),
        (123.0, Some(24)),
        (124.0, None),
    ] {
        assert_eq!(bonk_tick(&p, face), tick, "face {face}");
    }
    const ORIGIN: f32 = 200.0;
    for (face, active, stops) in [
        (24.0_f32, 1u16, 0.0_f32),
        (56.0, 8, 28.0),
        (84.0, 15, 56.0),
        (92.0, 17, 64.0),
        (123.0, 24, 95.0),
    ] {
        let wall = faced(ORIGIN + face);
        let run = run_fixed(&p, ORIGIN, 1, &wall, None, WIDE, &[], 300);
        let (tick, end) = run.end.unwrap();
        assert_eq!(end, RushEnd::Blocked, "face {face}");
        assert_eq!(tick, 31 + active, "bonk attack tick for face {face}");
        assert_eq!(run.x - ORIGIN, stops, "stop column for face {face}");
        // The sheet's `corral_rush_bonk_wall`: pivot at 56 for face 84.
        // Bonk lasts 60 ticks after the contact tick, no self-damage.
        assert_eq!(run.finished, Some(tick + p.crlq10_bonk_ticks));
        assert_eq!(run.state.phase, RushPhase::Bonk);
    }
    // No bonk from 124 on: the rush runs its 96 px.
    let wall = faced(ORIGIN + 124.0);
    let run = run_fixed(&p, ORIGIN, 1, &wall, None, WIDE, &[], 300);
    assert_eq!(run.end.unwrap().1, RushEnd::Miss);
    assert_eq!(run.x - ORIGIN, 96.0);
}

#[test]
fn a_wall_ahead_of_a_corral_llama_bonks_with_no_self_damage_and_names_the_end() {
    let p = profile();
    // `corral_rush_bonk_wall`: a wall face 84 px ahead of E 600 facing -1.
    let wall = [solid(600.0 - 84.0 - 16.0, 16.0, 200.0)];
    let mut state = RushState::start(600.0, -1, HeroId::Donut, 1);
    let mut x = 600.0;
    let mut bonk = None;
    for _ in 0..80 {
        let world = RushWorld {
            feet: 0.0,
            solids: &wall,
            partition: None,
            leash: WIDE,
            max_x: EDGE,
            heroes: &[],
        };
        let out = state.step(&p, x, &world);
        x = out.x;
        if out.end.is_some() {
            bonk = Some((state.tick, out));
            break;
        }
    }
    let (tick, out) = bonk.unwrap();
    assert_eq!(tick, 46, "bonk on active tick 15");
    assert_eq!(out.end, Some(RushEnd::Blocked));
    assert_eq!(out.x, 544.0, "the box front rests on the face");
    assert_eq!(out.self_damage, 0);
    assert!(!out.partition_hit && out.blow.is_none());
}

#[test]
fn the_partition_is_named_and_stimulated_but_a_remnant_is_not_a_solid() {
    let p = profile();
    let wall = [solid(1000.0, 16.0, 48.0)];
    let run = run_fixed(&p, 920.0, 1, &wall, Some(0), WIDE, &[], 300);
    assert_eq!(run.end.unwrap().1, RushEnd::Partition);
    let mut state = RushState::start(920.0, 1, HeroId::Carl, 1);
    let mut x = 920.0;
    let mut stimulus = false;
    for _ in 0..80 {
        let world = RushWorld {
            feet: 0.0,
            solids: &wall,
            partition: Some(0),
            leash: WIDE,
            max_x: EDGE,
            heroes: &[],
        };
        let out = state.step(&p, x, &world);
        x = out.x;
        stimulus |= out.partition_hit;
    }
    assert!(
        stimulus,
        "the rush contact sends the partition its stimulus"
    );
    // The same face without the partition index is a plain wall.
    let plain = run_fixed(&p, 920.0, 1, &wall, None, WIDE, &[], 300);
    assert_eq!(plain.end.unwrap().1, RushEnd::Blocked);
    // CRL-Q32: a 16 px remnant is no solid for the llama; the rush runs through.
    let remnant = [solid(1000.0, 16.0, 16.0)];
    let run = run_fixed(&p, 920.0, 1, &remnant, Some(0), WIDE, &[], 300);
    assert_eq!(run.end.unwrap().1, RushEnd::Miss);
    assert_eq!(run.x, 920.0 + 96.0);
    assert!(!llama_solid(&remnant[0]));
    assert!(llama_solid(&solid(0.0, 16.0, 17.0)));
    // A broken (not intact) solid never stops it.
    let mut gone = wall.clone();
    gone[0].intact = false;
    assert_eq!(
        run_fixed(&p, 920.0, 1, &gone, Some(0), WIDE, &[], 300)
            .end
            .unwrap()
            .1,
        RushEnd::Miss
    );
}

#[test]
fn a_leash_stop_short_of_a_face_is_a_miss_and_never_breaks_the_partition() {
    let p = profile();
    let leash = Leash {
        min: 0.0,
        max: 240.0,
    };
    // The face is well past the bound plus the box front: a plain leash stop.
    let run = run_fixed(&p, 200.0, 1, &faced(284.0), None, leash, &[], 300);
    assert_eq!(run.end.unwrap().1, RushEnd::Miss);
    assert_eq!(run.x, 240.0);
    // F1: the leash clamp lands 1 px short of the face (llama at 238, bound 240,
    // face at 269: the box front at the bound is 268). The step's own solid
    // clamp would have called it a contact; the leash wins: Miss, Recover, and
    // the partition stays intact (sheet section 10, "Near miss").
    let partition = [solid(269.0, 16.0, 48.0)];
    let run = run_fixed(&p, 238.0, 1, &partition, Some(0), leash, &[], 300);
    assert_eq!(run.end.unwrap().1, RushEnd::Miss);
    assert_eq!(run.x, 240.0);
    assert_eq!(run.state.phase, RushPhase::Recover);
    let mut state = RushState::start(238.0, 1, HeroId::Carl, 1);
    let mut x = 238.0;
    for _ in 0..80 {
        let world = RushWorld {
            feet: 0.0,
            solids: &partition,
            partition: Some(0),
            leash,
            max_x: EDGE,
            heroes: &[],
        };
        let out = state.step(&p, x, &world);
        x = out.x;
        assert!(!out.partition_hit, "a near miss never breaks the partition");
        assert_ne!(out.end, Some(RushEnd::Partition));
        assert_eq!(out.self_damage, 0);
    }
    // The motion itself reports it: leash stop, not met, no block named.
    let m = rush_motion(&p, 238.0, 1, 0.0, &partition, leash, EDGE);
    assert!(m.leash_stop && !m.met && m.met_block.is_none());
    assert_eq!(m.x, 240.0);
    // Front exactly on the face at the bound: that is a real contact. Bonk, and
    // the partition breaks.
    let flush = [solid(268.0, 16.0, 48.0)];
    let run = run_fixed(&p, 200.0, 1, &flush, Some(0), leash, &[], 300);
    assert_eq!(run.end.unwrap().1, RushEnd::Partition);
    assert_eq!(run.x, 240.0);
    let run = run_fixed(&p, 200.0, 1, &flush, None, leash, &[], 300);
    assert_eq!(run.end.unwrap().1, RushEnd::Blocked);
}

#[test]
fn a_hero_in_the_box_beats_the_leash_stop_and_is_hit_before_a_wall_is_reached() {
    let p = profile();
    let leash = Leash {
        min: 0.0,
        max: 240.0,
    };
    // A hero in the box on the very tick the leash would stop it takes the blow.
    let heroes = [hero_at(HeroId::Carl, 240.0 + 30.0)];
    let run = run_fixed(&p, 200.0, 1, &[], None, leash, &heroes, 300);
    assert!(matches!(run.end.unwrap().1, RushEnd::Contact(HeroId::Carl)));
    // A hero standing in front of a wall is hit before the box reaches it (the
    // box front meets the hero at active tick 11, the wall at 15): contact, no bonk.
    let wall = faced(284.0);
    let heroes = [hero_at(HeroId::Donut, 284.0 - 6.0)];
    let run = run_fixed(&p, 200.0, 1, &wall, None, WIDE, &heroes, 300);
    assert!(matches!(
        run.end.unwrap().1,
        RushEnd::Contact(HeroId::Donut)
    ));
}

#[test]
fn the_leash_stops_the_rush_at_the_bound_with_a_miss_and_recover() {
    let p = profile();
    // `corral_rush_leash_stop`: the leash bound 40 px ahead.
    let leash = Leash {
        min: 0.0,
        max: 240.0,
    };
    let run = run_fixed(&p, 200.0, 1, &[], None, leash, &[], 300);
    assert_eq!(run.x, 240.0);
    let (tick, end) = run.end.unwrap();
    assert_eq!(end, RushEnd::Miss);
    // The bound is reached on active tick 10; the step that would cross it
    // (active tick 11) is the one that stops the rush.
    assert_eq!(tick, 31 + 11);
    assert_eq!(run.finished, Some(tick + p.crlq09_recover_ticks));
    // Facing left, mirrored.
    let leash = Leash {
        min: 160.0,
        max: 1000.0,
    };
    let run = run_fixed(&p, 200.0, -1, &[], None, leash, &[], 300);
    assert_eq!(run.x, 160.0);
    // A leash bound behind the llama never pulls it backward (no teleport).
    let leash = Leash {
        min: 300.0,
        max: 400.0,
    };
    let run = run_fixed(&p, 200.0, -1, &[], None, leash, &[], 300);
    assert_eq!(run.x, 200.0);
    assert_eq!(run.end.unwrap().1, RushEnd::Miss);
    // The layout edge stops it too.
    let mut state = RushState::start(100.0, -1, HeroId::Carl, 1);
    let mut x = 100.0;
    for _ in 0..60 {
        let world = RushWorld {
            feet: 0.0,
            solids: &[],
            partition: None,
            leash: WIDE,
            max_x: EDGE,
            heroes: &[],
        };
        x = state.step(&p, x, &world).x;
    }
    assert_eq!(x, LLAMA_HALF_WIDTH, "the layout edge clamps the pivot");
}

#[test]
fn a_dock_llama_is_clamped_to_its_own_leash_while_rushing() {
    let p = profile();
    // Llama A: dock feet 32, leash 1920..2016, home 1968, facing -1. The dock
    // top is a solid at 32, the bumper 1856..1888 up to 80.
    let dock = Block {
        rect: Rect {
            left: 1856.0,
            bottom: 0.0,
            width: 192.0,
            height: 32.0,
        },
        material: Material::Stone,
        intact: true,
    };
    let bumper = solid(1856.0, 32.0, 80.0);
    let leash = Leash {
        min: 1920.0,
        max: 2016.0,
    };
    let mut state = RushState::start(1968.0, -1, HeroId::Donut, 1);
    let mut x = 1968.0;
    let solids = [dock, bumper];
    for _ in 0..120 {
        let world = RushWorld {
            feet: 32.0,
            solids: &solids,
            partition: None,
            leash,
            max_x: 2400.0,
            heroes: &[],
        };
        x = state.step(&p, x, &world).x;
        assert!(leash.contains(x), "x {x} left the leash");
    }
    assert_eq!(
        x, 1920.0,
        "the rush stops at the leash minimum, short of the bumper"
    );
}

#[test]
fn cooldowns_after_a_rush_and_after_an_interrupt() {
    let p = profile();
    assert_eq!(cooldowns_after_rush(&p, 0), (150, 30));
    assert_eq!(cooldowns_after_rush(&p, 12), (150, 30));
    assert_eq!(cooldowns_after_rush(&p, 90), (150, 90));
    assert_eq!(rush_cooldown_after_interrupt(&p, 0), 30);
    assert_eq!(rush_cooldown_after_interrupt(&p, 100), 100);
}

#[test]
fn the_state_hash_follows_the_state() {
    let p = profile();
    let hash = |s: &RushState| {
        let mut v = Vec::new();
        s.mix_into(&mut |x| v.push(x));
        v
    };
    let a = RushState::start(10.0, 1, HeroId::Carl, 1);
    assert_eq!(hash(&a), hash(&a));
    let mut b = a;
    b.step(
        &p,
        10.0,
        &RushWorld {
            feet: 0.0,
            solids: &[],
            partition: None,
            leash: WIDE,
            max_x: EDGE,
            heroes: &[],
        },
    );
    assert_ne!(hash(&a), hash(&b));
    assert_ne!(hash(&a), hash(&RushState::start(10.0, -1, HeroId::Carl, 1)));
    assert_ne!(hash(&a), hash(&RushState::start(10.0, 1, HeroId::Donut, 1)));
    assert_ne!(hash(&a), hash(&RushState::start(10.0, 1, HeroId::Carl, 2)));
    assert_ne!(Activation::Dormant.code(), Activation::Awake.code());
}

// ---------------------------------------------------------------- hit table --

#[test]
fn the_f10_hit_table_for_every_state() {
    let mut s = RushState::start(0.0, 1, HeroId::Carl, 1);
    // Not in a rush (Neutral, a spit Attack, Hurt): the legacy path.
    assert_eq!(hit_reaction(None, false), HitReaction::Legacy);
    assert_eq!(hit_reaction(None, true), HitReaction::Legacy);
    // The tell: interrupted by any non-lethal hit; a lethal hit is the legacy Down.
    s.phase = RushPhase::Tell;
    assert_eq!(hit_reaction(Some(&s), false), HitReaction::Interrupt);
    assert_eq!(hit_reaction(Some(&s), true), HitReaction::Legacy);
    // Committed: damage only; lethal kills in place.
    s.phase = RushPhase::Committed;
    assert_eq!(hit_reaction(Some(&s), false), HitReaction::DamageOnly);
    assert_eq!(hit_reaction(Some(&s), true), HitReaction::DownInPlace);
    // Recover: any hit staggers it (legacy).
    s.phase = RushPhase::Recover;
    assert_eq!(hit_reaction(Some(&s), false), HitReaction::Legacy);
    assert_eq!(hit_reaction(Some(&s), true), HitReaction::Legacy);
    // Bonk: damage and hit-stop only, timer unchanged; lethal kills in place.
    s.phase = RushPhase::Bonk;
    assert_eq!(hit_reaction(Some(&s), false), HitReaction::DamageOnly);
    assert_eq!(hit_reaction(Some(&s), true), HitReaction::DownInPlace);
}

// --------------------------------------------------------------- slide clamp --

#[test]
fn the_corral_slide_is_the_legacy_slide_inside_a_wide_leash() {
    // The same knockback through `Llama::step` (Down slides) and `corral_slide`.
    let feet = 32.0;
    let solids = [floor()];
    for (vx, facing) in [(265.0, -1), (-265.0, 1), (120.0, 1), (60.0, -1)] {
        let mut llama = Llama::new(500.0, feet + LLAMA_HALF_HEIGHT, EnemyAi::Chase);
        llama.facing = facing;
        llama.mode = LlamaMode::Down;
        llama.vx = vx;
        let (mut x, mut v) = (500.0, vx);
        for _ in 0..30 {
            llama.step(0.0, true, &solids, EDGE, VIEW);
            (x, v) = corral_slide(x, v, feet, facing, &solids, EDGE, WIDE);
            assert_eq!(llama.x, x, "x for vx {vx}");
            assert_eq!(llama.vx, v, "vx for vx {vx}");
        }
    }
}

#[test]
fn a_kick_or_a_death_slide_at_the_leash_edge_stops_at_the_bound() {
    // `corral_kick_at_leash_edge`: a dock llama at its leash minimum, pushed
    // toward the bound by a kick (265 px/s) and by a lethal slide.
    let leash = Leash {
        min: 1920.0,
        max: 2016.0,
    };
    let (mut x, mut vx) = (1920.0, -265.0);
    (x, vx) = corral_slide(x, vx, 32.0, 1, &[], 4000.0, leash);
    assert_eq!((x, vx), (1920.0, 0.0), "stays at the bound, vx becomes 0");
    // Starting inside, it slides up to the bound and no further, in every mode.
    let (mut x, mut vx) = (1928.0, -265.0);
    for _ in 0..30 {
        (x, vx) = corral_slide(x, vx, 32.0, 1, &[], 4000.0, leash);
        assert!(x >= 1920.0);
    }
    assert_eq!((x, vx), (1920.0, 0.0));
    // The upper bound too.
    let (mut x, mut vx) = (2010.0, 265.0);
    for _ in 0..30 {
        (x, vx) = corral_slide(x, vx, 32.0, -1, &[], 4000.0, leash);
        assert!(x <= 2016.0);
    }
    assert_eq!(x, 2016.0);
    assert_eq!(vx, 0.0);
    // Already outside the interval: it does not teleport.
    assert_eq!(
        corral_slide(1900.0, -100.0, 32.0, 1, &[], 4000.0, leash).0,
        1900.0
    );
    // A solid stops it as before, a remnant does not.
    let wall = [solid(1950.0, 16.0, 80.0)];
    let (x, vx) = corral_slide(1920.0, 600.0, 32.0, 1, &wall, 4000.0, WIDE);
    assert!(x < 1950.0 || vx == 0.0 || x == 1920.0);
    let remnant = [solid(1930.0, 16.0, 16.0)];
    let (x, _) = corral_slide(1920.0, 600.0, 32.0, 1, &remnant, 4000.0, WIDE);
    assert!(x > 1920.0);
}

#[test]
fn walking_is_clamped_to_the_leash_and_never_enters_a_solid() {
    let p = profile();
    let leash = Leash {
        min: 100.0,
        max: 200.0,
    };
    let step = p.walk_px_per_tick();
    assert!((step - 40.0 / 60.0).abs() < 1e-6);
    assert_eq!(walk_step(&p, 150.0, 1, 0.0, &[], leash), 150.0 + step);
    assert_eq!(walk_step(&p, 150.0, -1, 0.0, &[], leash), 150.0 - step);
    assert_eq!(walk_step(&p, 200.0, 1, 0.0, &[], leash), 200.0);
    assert_eq!(walk_step(&p, 100.0, -1, 0.0, &[], leash), 100.0);
    // A wall in front of the body stops it; a remnant does not.
    let wall = [solid(172.0, 16.0, 100.0)];
    let x = 150.0 + 20.0 - 0.1;
    assert_eq!(walk_step(&p, x, 1, 0.0, &wall, WIDE), x);
    let remnant = [solid(172.0, 16.0, 16.0)];
    assert!(walk_step(&p, x, 1, 0.0, &remnant, WIDE) > x);
}

// ---------------------------------------------------------------- activation --

#[test]
fn a_dormant_llama_wakes_when_in_view_with_a_hero_near_and_stays_awake() {
    let p = profile();
    let view = FireView {
        left: 1000.0,
        right: 1640.0,
    };
    let body = llama_body(1488.0, 0.0, -1);
    let mut act = Activation::Dormant;
    // A hero 320 px away, the body inside the view: wakes (inclusive).
    assert!(act.step(&p, body, 1488.0, view, [1168.0]));
    assert_eq!(act, Activation::Awake);
    // Awake for the rest of the attempt: it never reports a second wake.
    assert!(!act.step(&p, body, 1488.0, view, []));
    assert!(act.awake());
    // 321 px: asleep.
    let mut act = Activation::Dormant;
    assert!(!act.step(&p, body, 1488.0, view, [1167.0]));
    assert_eq!(act, Activation::Dormant);
    // Body not whole inside the view: asleep even next to a hero.
    let narrow = FireView {
        left: 1000.0,
        right: 1500.0,
    };
    assert!(!act.step(&p, body, 1488.0, narrow, [1450.0]));
    assert!(!act.step(&p, body, 1488.0, view, [])); // nobody near
    assert!(!act.awake());
    // A dormant llama decides nothing.
    let facts = LlamaFacts {
        x: 1488.0,
        feet: 0.0,
        vx_zero: true,
        cooldown: 0,
        rush_cooldown: 0,
        activation: Activation::Dormant,
    };
    let heroes = [hero_facts(HeroId::Donut, 1400.0, 0.0)];
    let d = decide(&p, &facts, &heroes, &env(&[], view));
    assert_eq!(d.action, Action::Hold);
    assert_eq!(d.target, None);
}

// ----------------------------------------------------------------- the brain --

fn hero_facts(id: HeroId, x: f32, support: f32) -> HeroFacts {
    let (half, height) = hurtbox_of(id);
    HeroFacts {
        id,
        x,
        hurtbox: Rect {
            left: x - half,
            bottom: support,
            width: half * 2.0,
            height,
        },
        support,
        grounded: true,
        alive: true,
        bias_px: 0.0,
    }
}

fn env(solids: &[Block], view: FireView) -> DecideEnv<'_> {
    DecideEnv {
        solids,
        partition: None,
        view,
        held: None,
        switch_px: 24.0,
    }
}

fn awake_llama(x: f32, feet: f32) -> LlamaFacts {
    LlamaFacts {
        x,
        feet,
        vx_zero: true,
        cooldown: 0,
        rush_cooldown: 0,
        activation: Activation::Awake,
    }
}

const E: f32 = 600.0;

#[test]
fn the_rush_trigger_is_128_with_no_minimum() {
    let p = profile();
    let llama = awake_llama(E, 0.0);
    // `corral_rush_trigger_edge`: d 128 rushes; 132 walks to 112 first.
    for (d, expected) in [
        (0.5, Action::Rush),
        (24.0, Action::Rush),
        (112.0, Action::Rush),
        (128.0, Action::Rush),
        (129.0, Action::Walk { dir: -1 }),
        (132.0, Action::Walk { dir: -1 }),
    ] {
        let heroes = [hero_facts(HeroId::Donut, E - d, 0.0)];
        let dec = decide(&p, &llama, &heroes, &env(&[], VIEW));
        assert_eq!(dec.action, expected, "d {d}");
        assert_eq!(dec.target, Some(HeroId::Donut));
        assert_eq!(dec.facing, Some(-1));
    }
}

#[test]
fn the_walk_stops_at_the_approach_distance_and_never_goes_backward() {
    let p = profile();
    let llama = awake_llama(E, 0.0);
    // A rushable Donut on cooldown: walk to 112 and hold, never back off.
    let mut cooling = llama;
    cooling.rush_cooldown = 90;
    for (d, expected) in [
        (300.0, Action::Walk { dir: -1 }),
        (113.0, Action::Walk { dir: -1 }),
        (112.0, Action::Hold),
        (60.0, Action::Hold),
    ] {
        let heroes = [hero_facts(HeroId::Donut, E - d, 0.0)];
        let dec = decide(&p, &cooling, &heroes, &env(&[], VIEW));
        assert_eq!(dec.action, expected, "d {d}");
    }
    // The other side mirrors.
    let heroes = [hero_facts(HeroId::Donut, E + 200.0, 0.0)];
    assert_eq!(
        decide(&p, &cooling, &heroes, &env(&[], VIEW)).action,
        Action::Walk { dir: 1 }
    );
}

#[test]
fn the_rush_waits_for_both_cooldowns_a_still_llama_and_a_grounded_target() {
    let p = profile();
    let heroes = [hero_facts(HeroId::Donut, E - 96.0, 0.0)];
    let mut llama = awake_llama(E, 0.0);
    assert_eq!(
        decide(&p, &llama, &heroes, &env(&[], VIEW)).action,
        Action::Rush
    );
    llama.rush_cooldown = 1;
    assert_ne!(
        decide(&p, &llama, &heroes, &env(&[], VIEW)).action,
        Action::Rush
    );
    llama.rush_cooldown = 0;
    llama.cooldown = 1;
    assert_ne!(
        decide(&p, &llama, &heroes, &env(&[], VIEW)).action,
        Action::Rush
    );
    llama.cooldown = 0;
    llama.vx_zero = false;
    assert_ne!(
        decide(&p, &llama, &heroes, &env(&[], VIEW)).action,
        Action::Rush
    );
    llama.vx_zero = true;
    // A tell starts only against a grounded target.
    let mut airborne = heroes;
    airborne[0].grounded = false;
    let dec = decide(&p, &llama, &airborne, &env(&[], VIEW));
    assert_ne!(dec.action, Action::Rush);
    // ...and she is still a rush target: never spat at, the llama holds (d 96 is
    // inside the approach distance).
    assert_eq!(dec.target, Some(HeroId::Donut));
    assert_eq!(dec.action, Action::Hold);
}

#[test]
fn a_hopping_floor_donut_is_never_spat_at_and_stays_a_rush_target() {
    let p = profile();
    // `corral_spit_hop_classified_floor`: Donut at d 160 hopping in place. Her
    // hurtbox is mid-air (bottom 30) but her support height is the floor.
    let llama = awake_llama(E, 0.0);
    let mut hop = hero_facts(HeroId::Donut, E - 160.0, 0.0);
    hop.hurtbox.bottom = 30.0;
    hop.grounded = false;
    let class = classify(&p, &llama, -1, &hop, &[]);
    assert!(class.rushable && !class.spittable);
    let dec = decide(&p, &llama, &[hop], &env(&[], VIEW));
    assert_eq!(dec.action, Action::Walk { dir: -1 });
    // Without the Q11 switch the hop would be spittable by the legacy rule.
    let mut off = p.clone();
    off.crlq11_spit_height_check = false;
    assert!(classify(&off, &llama, -1, &hop, &[]).spittable);
}

#[test]
fn the_spit_height_rule_floor_donut_block_donut_and_carl() {
    let p = profile();
    let llama = awake_llama(E, 0.0);
    // `corral_spit_height_rule`.
    let floor_donut = [hero_facts(HeroId::Donut, E - 160.0, 0.0)];
    assert_eq!(
        decide(&p, &llama, &floor_donut, &env(&[], VIEW)).action,
        Action::Walk { dir: -1 },
        "floor Donut is never spat at: walk to 112, then rush"
    );
    let on_block = [hero_facts(HeroId::Donut, E - 160.0, 32.0)];
    assert_eq!(
        decide(&p, &llama, &on_block, &env(&[], VIEW)).action,
        Action::Spit,
        "Donut one step up is in the glob band"
    );
    let carl = [hero_facts(HeroId::Carl, E - 160.0, 0.0)];
    assert_eq!(
        decide(&p, &llama, &carl, &env(&[], VIEW)).action,
        Action::Spit,
        "Carl is both rushable and spittable; at 160 he is spat at"
    );
    // The same Donut in the trigger gets the rush, never a spit.
    let near = [hero_facts(HeroId::Donut, E - 96.0, 0.0)];
    assert_eq!(
        decide(&p, &llama, &near, &env(&[], VIEW)).action,
        Action::Rush
    );
}

#[test]
fn a_floor_donut_is_never_spittable_by_the_dock_llama_and_the_bumper_blocks_floor_globs() {
    let p = profile();
    // Llama A on the dock (feet 32): its band is 74..80; a floor Donut is far
    // below it and a dock Donut is under it. Carl on the dock (hurtbox 32..84)
    // is in the band.
    let dock = awake_llama(1968.0, 32.0);
    let floor_donut = hero_facts(HeroId::Donut, 1900.0, 0.0);
    assert!(!classify(&p, &dock, -1, &floor_donut, &[]).spittable);
    let dock_donut = hero_facts(HeroId::Donut, 1900.0, 32.0);
    assert!(!classify(&p, &dock, -1, &dock_donut, &[]).spittable);
    let dock_carl = hero_facts(HeroId::Carl, 1900.0, 32.0);
    assert!(classify(&p, &dock, -1, &dock_carl, &[]).spittable);
    // Llama B on the floor (feet 0): a Donut on the dock stands in B's band
    // (42..48 against 32..56), but the 80 px bumper blocks the glob lane.
    let b = awake_llama(1488.0, 0.0);
    let bumper = [solid(1856.0, 32.0, 80.0)];
    let donut_on_dock = hero_facts(HeroId::Donut, 1920.0, 32.0);
    assert!(classify(&p, &b, 1, &donut_on_dock, &[]).spittable);
    assert!(!classify(&p, &b, 1, &donut_on_dock, &bumper).spittable);
    // On the entry shelf (32) she is in B's glob (no bumper between).
    let shelf = hero_facts(HeroId::Donut, 1168.0, 32.0);
    let b_far = awake_llama(1488.0, 0.0);
    assert!(classify(&p, &b_far, -1, &shelf, &[]).spittable);
}

#[test]
fn a_hero_within_eight_px_of_the_floor_is_rushable_and_one_on_a_step_is_not() {
    let p = profile();
    let llama = awake_llama(E, 0.0);
    for (support, rushable) in [
        (0.0, true),
        (8.0, true),
        (8.5, false),
        (32.0, false),
        (-8.0, true),
    ] {
        let hero = hero_facts(HeroId::Carl, E - 100.0, support);
        assert_eq!(
            classify(&p, &llama, -1, &hero, &[]).rushable,
            rushable,
            "support {support}"
        );
    }
}

#[test]
fn the_rush_lane_rule_the_partition_is_passable_and_a_wall_is_not() {
    let p = profile();
    let heroes = [hero_facts(HeroId::Donut, E - 96.0, 0.0)];
    let llama = awake_llama(E, 0.0);
    // A wall between the llama and the hero: no rush (a stall, CRL-Q19).
    let wall = [solid(E - 70.0, 16.0, 100.0)];
    let dec = decide(&p, &llama, &heroes, &env(&wall, VIEW));
    assert_ne!(dec.action, Action::Rush);
    // The same face as the breakable partition: the bait works.
    let mut with_partition = env(&wall, VIEW);
    with_partition.partition = Some(0);
    assert_eq!(
        decide(&p, &llama, &heroes, &with_partition).action,
        Action::Rush
    );
    // A remnant (16 px) is not in the lane.
    let remnant = [solid(E - 70.0, 16.0, 16.0)];
    assert_eq!(
        decide(&p, &llama, &heroes, &env(&remnant, VIEW)).action,
        Action::Rush
    );
    // A wall behind the hero is not between them.
    let behind = [solid(E - 140.0, 16.0, 100.0)];
    assert_eq!(
        decide(&p, &llama, &heroes, &env(&behind, VIEW)).action,
        Action::Rush
    );
    // A wall low enough to be under the box (top 2) does not count.
    let rail = [solid(E - 70.0, 16.0, 2.0)];
    assert!(rush_lane_clear(&p, E, 0.0, -1, E - 96.0, &rail, None));
}

#[test]
fn the_fair_rush_view_rule_needs_body_lane_and_target_inside_the_view() {
    let p = profile();
    let llama = awake_llama(E, 0.0);
    let heroes = [hero_facts(HeroId::Donut, E - 96.0, 0.0)];
    let inside = FireView {
        left: E - 200.0,
        right: E + 100.0,
    };
    assert_eq!(
        decide(&p, &llama, &heroes, &env(&[], inside)).action,
        Action::Rush
    );
    // The swept lane ends 124 px ahead: a view ending at E - 123 cuts it off.
    let cut = FireView {
        left: E - 123.0,
        right: E + 100.0,
    };
    assert_ne!(
        decide(&p, &llama, &heroes, &env(&[], cut)).action,
        Action::Rush
    );
    // The lane just fits (E - 124 .. body behind): the left edge is the lane end.
    let fits = FireView {
        left: E - 124.0,
        right: E + 100.0,
    };
    assert_eq!(
        decide(&p, &llama, &heroes, &env(&[], fits)).action,
        Action::Rush
    );
    // The body must be whole inside the view (the llama faces -1: body right is E + 20).
    let body_cut = FireView {
        left: E - 200.0,
        right: E + 19.0,
    };
    assert_ne!(
        decide(&p, &llama, &heroes, &env(&[], body_cut)).action,
        Action::Rush
    );
    // `corral_rush_offscreen_no_tell`: lane end outside the view: no tell, the
    // llama walks or holds.
    assert!(matches!(
        decide(&p, &llama, &heroes, &env(&[], cut)).action,
        Action::Hold | Action::Walk { .. }
    ));
    // The target must be inside the view too.
    let target_out = FireView {
        left: E - 90.0,
        right: E + 100.0,
    };
    assert!(!fair_rush_view(&p, E, -1, heroes[0].hurtbox, target_out));
}

#[test]
fn only_heroes_within_320_are_candidates_and_a_far_hero_draws_nothing() {
    let p = profile();
    let llama = awake_llama(E, 0.0);
    let far = [hero_facts(HeroId::Carl, E - 321.0, 0.0)];
    let dec = decide(&p, &llama, &far, &env(&[], VIEW));
    assert_eq!((dec.target, dec.action), (None, Action::Hold));
    let edge = [hero_facts(HeroId::Carl, E - 320.0, 0.0)];
    assert_eq!(
        decide(&p, &llama, &edge, &env(&[], VIEW)).target,
        Some(HeroId::Carl)
    );
    // A dead hero is never a candidate.
    let mut dead = hero_facts(HeroId::Carl, E - 50.0, 0.0);
    dead.alive = false;
    assert_eq!(decide(&p, &llama, &[dead], &env(&[], VIEW)).target, None);
}

#[test]
fn the_target_prefers_rushable_then_spittable_and_applies_the_party_hysteresis() {
    let p = profile();
    let llama = awake_llama(E, 0.0);
    // Carl on the floor 40 px nearer than Donut on a block: Carl is rushable,
    // Donut only spittable, so the rushable one wins whatever the distance.
    let heroes = [
        hero_facts(HeroId::Carl, E - 200.0, 0.0),
        hero_facts(HeroId::Donut, E - 100.0, 32.0),
    ];
    assert_eq!(
        decide(&p, &llama, &heroes, &env(&[], VIEW)).target,
        Some(HeroId::Carl)
    );
    // Both rushable: the nearer, ties to Carl.
    let both = [
        hero_facts(HeroId::Carl, E - 100.0, 0.0),
        hero_facts(HeroId::Donut, E - 100.0, 0.0),
    ];
    assert_eq!(
        decide(&p, &llama, &both, &env(&[], VIEW)).target,
        Some(HeroId::Carl)
    );
    // Solo bias: Donut is controlled, Carl (the AI) counts 40 px farther.
    let mut solo = [
        hero_facts(HeroId::Carl, E - 100.0, 0.0),
        hero_facts(HeroId::Donut, E - 130.0, 0.0),
    ];
    solo[0].bias_px = 40.0;
    assert_eq!(
        decide(&p, &llama, &solo, &env(&[], VIEW)).target,
        Some(HeroId::Donut),
        "100 + 40 against 130"
    );
    // Hysteresis: a held hero is kept unless the other is 24 px closer.
    let near_pair = [
        hero_facts(HeroId::Carl, E - 100.0, 0.0),
        hero_facts(HeroId::Donut, E - 80.0, 0.0),
    ];
    let mut held = env(&[], VIEW);
    held.held = Some(HeroId::Carl);
    assert_eq!(
        decide(&p, &llama, &near_pair, &held).target,
        Some(HeroId::Carl),
        "20 px closer is not enough"
    );
    let nearer = [
        hero_facts(HeroId::Carl, E - 100.0, 0.0),
        hero_facts(HeroId::Donut, E - 76.0, 0.0),
    ];
    assert_eq!(
        decide(&p, &llama, &nearer, &held).target,
        Some(HeroId::Donut),
        "24 px closer switches"
    );
    // Held hero not eligible: the nearest of the pool.
    held.held = Some(HeroId::Donut);
    let carl_only = [
        hero_facts(HeroId::Carl, E - 100.0, 0.0),
        hero_facts(HeroId::Donut, E - 90.0, 32.0),
    ];
    assert_eq!(
        decide(&p, &llama, &carl_only, &held).target,
        Some(HeroId::Carl)
    );
}

#[test]
fn a_hero_that_is_neither_rushable_nor_spittable_only_sets_the_facing() {
    let p = profile();
    let llama = awake_llama(E, 0.0);
    // Donut on the 80 px bumper: not on the floor, far above the glob band.
    let high = [hero_facts(HeroId::Donut, E + 150.0, 80.0)];
    let dec = decide(&p, &llama, &high, &env(&[], VIEW));
    assert_eq!(dec.target, Some(HeroId::Donut));
    assert_eq!(dec.facing, Some(1));
    assert_eq!(dec.action, Action::Hold);
}

#[test]
fn a_spit_only_target_is_approached_to_the_stand_off_of_200() {
    let p = profile();
    let llama = awake_llama(E, 0.0);
    // Donut on a block at 330: out of the 320 candidate range; at 300 the legacy
    // fair-fire holds and she is spat at; walking only past the stand-off if the
    // spit cannot start (cooldown).
    let mut cooling = llama;
    cooling.cooldown = 10;
    for (d, expected) in [
        (300.0, Action::Walk { dir: -1 }),
        (201.0, Action::Walk { dir: -1 }),
        (200.0, Action::Hold),
        (150.0, Action::Hold),
    ] {
        let heroes = [hero_facts(HeroId::Donut, E - d, 32.0)];
        assert_eq!(
            decide(&p, &cooling, &heroes, &env(&[], VIEW)).action,
            expected,
            "d {d}"
        );
    }
    // With no cooldown the spit starts instead of the walk.
    let heroes = [hero_facts(HeroId::Donut, E - 300.0, 32.0)];
    assert_eq!(
        decide(&p, &llama, &heroes, &env(&[], VIEW)).action,
        Action::Spit
    );
    // The legacy fair-fire view part: a body half outside the view does not spit.
    let cut = FireView {
        left: 0.0,
        right: E + 10.0,
    };
    assert_ne!(
        decide(&p, &llama, &heroes, &env(&[], cut)).action,
        Action::Spit
    );
}

#[test]
fn a_rushing_llama_never_spits_so_the_spit_and_the_rush_are_exclusive_by_state() {
    // `corral_rush_no_glob`: the machine has no spit release: `telling` is the
    // rush tell only, and the target state moves from Tell to Impact on the
    // commit, so the swat guard clears and the glob never spawns.
    let p = profile();
    let mut state = RushState::start(600.0, -1, HeroId::Donut, 1);
    let mut seen = Vec::new();
    let mut x = 600.0;
    for _ in 0..60 {
        let world = RushWorld {
            feet: 0.0,
            solids: &[],
            partition: None,
            leash: WIDE,
            max_x: EDGE,
            heroes: &[],
        };
        x = state.step(&p, x, &world).x;
        seen.push((state.tick, state.telling(), state.phase.target_state()));
    }
    assert!(seen.iter().take_while(|s| s.0 < 32).all(|s| s.1));
    assert!(seen.iter().filter(|s| s.0 >= 32).all(|s| !s.1));
    assert!(seen.iter().any(|s| s.0 == 32 && s.2 == TargetState::Impact));
}

// ----------------------------------------------------------- fairness tables --

#[test]
fn the_reach_table_contact_ticks() {
    let p = profile();
    // The sheet's fairness table: the active tick of a hit on a standing hero.
    for (d, tick) in [
        (12.0, Some(1)),
        (24.0, Some(1)),
        (36.0, Some(1)),
        (48.0, Some(3)),
        (64.0, Some(7)),
        (80.0, Some(11)),
        (96.0, Some(15)),
        (112.0, Some(19)),
        (128.0, Some(23)),
    ] {
        assert_eq!(contact_tick(&p, d, DONUT_HALF_WIDTH), tick, "Donut at {d}");
        assert_eq!(contact_tick(&p, d, CARL_HALF_WIDTH), tick, "Carl at {d}");
    }
    // Donut is hit when d < 134, Carl when d < 136; the trigger (128) is inside.
    assert_eq!(contact_tick(&p, 133.0, DONUT_HALF_WIDTH), Some(24));
    assert_eq!(contact_tick(&p, 134.0, DONUT_HALF_WIDTH), None);
    assert_eq!(contact_tick(&p, 135.0, CARL_HALF_WIDTH), Some(24));
    assert_eq!(contact_tick(&p, 136.0, CARL_HALF_WIDTH), None);
    assert!(f32::from(p.crlq05_trigger_px) < p.standing_reach(DONUT_HALF_WIDTH));
}

/// One hero of the fairness proof with the real hero physics.
struct Hero1 {
    hero: Hero,
    motion: HeroMotion,
    half_w: f32,
    height: f32,
    input: InputFrame,
}

impl Hero1 {
    fn new(id: HeroId, x: f32, boost: bool) -> Self {
        let (half_w, height) = hurtbox_of(id);
        Self {
            hero: Hero::at(x, HERO_HALF_HEIGHT),
            motion: motion_of(id, boost),
            half_w,
            height,
            input: InputFrame::default(),
        }
    }

    fn step(&mut self, jump: bool, run: bool) {
        self.input.jump_pressed = jump;
        self.input.held = Actions {
            move_x: if run { 1.0 } else { 0.0 },
            jump,
            ..Actions::default()
        };
        step_hero_motion(
            &mut self.hero,
            self.motion,
            &self.input,
            &[floor()],
            1.0e9,
            &BodyDrive::default(),
        );
    }

    fn rush_hero(&self, id: HeroId) -> RushHero {
        RushHero {
            id,
            hurtbox: Rect {
                left: self.hero.x - self.half_w,
                bottom: self.hero.y - HERO_HALF_HEIGHT,
                width: self.half_w * 2.0,
                height: self.height,
            },
            hittable: true,
        }
    }
}

#[derive(Clone, Copy)]
enum Answer {
    Stand,
    /// Press jump on this attack tick.
    Jump(u16),
    /// Hold run away from this attack tick on.
    Run(u16),
}

/// The attack tick of the hit, if any, of a rush from pivot 1000 facing +1
/// against a hero at pivot distance `d`, answering as `answer`. Heroes step
/// first on each tick, then the llama (tick 0 is the tell start tick).
fn rush_vs(id: HeroId, d: f32, boost: bool, answer: Answer) -> Option<u16> {
    let p = profile();
    let mut hero = Hero1::new(id, 1000.0 + d, boost);
    let mut state = RushState::start(1000.0, 1, id, 1);
    let mut x = 1000.0;
    for t in 0..=60u16 {
        let (jump, run) = match answer {
            Answer::Stand => (false, false),
            Answer::Jump(j) => (t == j, false),
            Answer::Run(r) => (false, t >= r),
        };
        hero.step(jump, run);
        if t == 0 {
            continue;
        }
        let heroes = [hero.rush_hero(id)];
        let world = RushWorld {
            feet: 0.0,
            solids: &[],
            partition: None,
            leash: WIDE,
            max_x: EDGE,
            heroes: &heroes,
        };
        let out = state.step(&p, x, &world);
        x = out.x;
        if out.blow.is_some() {
            return Some(state.tick);
        }
        if out.finished || state.phase == RushPhase::Recover {
            return None;
        }
    }
    None
}

/// The inclusive range of jump press ticks that clear (`None` hits) over 0..=60.
fn jump_window(id: HeroId, d: f32, boost: bool) -> Option<(u16, u16)> {
    let clears: Vec<u16> = (0..=60u16)
        .filter(|&j| rush_vs(id, d, boost, Answer::Jump(j)).is_none())
        .collect();
    let (&first, &last) = (clears.first()?, clears.last()?);
    // The window is contiguous.
    assert_eq!(clears.len() as u16, last - first + 1, "{id:?} d {d}");
    Some((first, last))
}

/// The latest run start that clears.
fn run_latest(id: HeroId, d: f32) -> Option<u16> {
    (0..=60u16)
        .filter(|&r| rush_vs(id, d, false, Answer::Run(r)).is_none())
        .max()
}

#[test]
fn a_standing_hero_is_hit_on_the_sheets_tick_for_every_distance() {
    for (d, attack) in [
        (24.0, 32),
        (36.0, 32),
        (48.0, 34),
        (64.0, 38),
        (80.0, 42),
        (96.0, 46),
        (112.0, 50),
        (128.0, 54),
    ] {
        for id in [HeroId::Carl, HeroId::Donut] {
            assert_eq!(
                rush_vs(id, d, false, Answer::Stand),
                Some(attack),
                "{id:?} d {d}"
            );
        }
    }
    assert_eq!(rush_vs(HeroId::Carl, 136.0, false, Answer::Stand), None);
    assert_eq!(rush_vs(HeroId::Donut, 134.0, false, Answer::Stand), None);
}

#[test]
fn the_jump_windows_of_the_fairness_table() {
    // The sheet's table, `d`: (Donut jump, Carl jump).
    for (d, donut, carl) in [
        (24.0, (0, 29), (0, 28)),
        (48.0, (0, 31), (2, 30)),
        (64.0, (0, 35), (6, 34)),
        (80.0, (2, 39), (10, 38)),
        (96.0, (4, 43), (12, 42)),
        (112.0, (4, 47), (12, 46)),
        (128.0, (4, 51), (12, 50)),
    ] {
        assert_eq!(
            jump_window(HeroId::Donut, d, false),
            Some(donut),
            "Donut d {d}"
        );
        assert_eq!(
            jump_window(HeroId::Carl, d, false),
            Some(carl),
            "Carl d {d}"
        );
    }
    // The fixtures `corral_rush_jump_clears_*`: d 96, jumps on 4 and 43 clear
    // (Donut), 3 and 44 are hit; Carl 12 and 42 clear, 11 and 43 are hit.
    let hit = |id, d, j| rush_vs(id, d, false, Answer::Jump(j)).is_some();
    assert!(!hit(HeroId::Donut, 96.0, 4) && !hit(HeroId::Donut, 96.0, 43));
    assert!(hit(HeroId::Donut, 96.0, 3) && hit(HeroId::Donut, 96.0, 44));
    assert!(!hit(HeroId::Carl, 96.0, 12) && !hit(HeroId::Carl, 96.0, 42));
    assert!(hit(HeroId::Carl, 96.0, 11) && hit(HeroId::Carl, 96.0, 43));
}

#[test]
fn a_hero_who_reacts_inside_15_ticks_always_has_a_jump_window_of_14() {
    // The fairness invariant: at every start distance a reaction within 15
    // ticks (250 ms) still leaves a jump window of at least 14 ticks.
    for d in (0..=128).step_by(8) {
        for id in [HeroId::Carl, HeroId::Donut] {
            let d = (d as f32).max(1.0);
            let (lo, hi) = jump_window(id, d, false).unwrap();
            let start = lo.max(15);
            assert!(hi + 1 >= start + 14, "{id:?} d {d}: window {lo}..{hi}");
        }
    }
}

#[test]
fn running_away_must_start_early_the_run_edges_at_96() {
    // `corral_rush_run_away`: Donut d 96 clears running from 44, is hit from 45;
    // Carl clears from 43.
    assert_eq!(run_latest(HeroId::Donut, 96.0), Some(44));
    assert!(rush_vs(HeroId::Donut, 96.0, false, Answer::Run(45)).is_some());
    assert_eq!(run_latest(HeroId::Carl, 96.0), Some(43));
    // The point-blank runs (d 0..36): latest 17..27 for Donut, 11..23 for Carl.
    let donut = run_latest(HeroId::Donut, 24.0).unwrap();
    let carl = run_latest(HeroId::Carl, 24.0).unwrap();
    assert!((17..=27).contains(&donut), "Donut point-blank run {donut}");
    assert!((11..=23).contains(&carl), "Carl point-blank run {carl}");
    // The table's other rows.
    for (d, donut, carl) in [
        (48.0, 31, 27),
        (64.0, 35, 32),
        (80.0, 40, 37),
        (112.0, 49, 48),
        (128.0, 54, 53),
    ] {
        assert_eq!(run_latest(HeroId::Donut, d), Some(donut), "Donut d {d}");
        assert_eq!(run_latest(HeroId::Carl, d), Some(carl), "Carl d {d}");
    }
}

#[test]
fn the_boosted_donut_window_starts_at_the_first_tick_everywhere() {
    // Light on Your Feet (launch 547): the window starts at attack tick 0 at
    // every distance; below d 80 it equals the base window, from 80 up it is
    // wider.
    for d in [24.0, 48.0, 64.0, 80.0, 96.0, 112.0, 128.0] {
        let base = jump_window(HeroId::Donut, d, false).unwrap();
        let boosted = jump_window(HeroId::Donut, d, true).unwrap();
        assert_eq!(boosted.0, 0, "boosted window starts at 0 at d {d}");
        if d < 80.0 {
            assert_eq!(boosted, base, "d {d}");
        } else {
            assert!(
                boosted.0 < base.0 || boosted.1 > base.1,
                "d {d}: {boosted:?} against {base:?}"
            );
            assert!(boosted.0 <= base.0 && boosted.1 >= base.1, "d {d}");
        }
    }
}

#[test]
fn a_hero_on_a_32_px_shelf_is_never_hit() {
    let p = profile();
    let boxed = rush_box(&p, 1000.0, 1, 0.0);
    assert_eq!(boxed.top(), 30.0);
    let on_shelf = Rect {
        left: 1010.0,
        bottom: 32.0,
        width: 20.0,
        height: 24.0,
    };
    assert!(!boxed.overlaps(on_shelf));
}

#[test]
fn a_clean_jump_leaves_the_llama_facing_away_in_recover() {
    let p = profile();
    // The punish: a whiff ends on attack tick 55; Recover covers 56..95. At the
    // approach stop (d 112) the llama ends 96 px further on, 16 px short of
    // where the hero stood.
    let heroes: [RushHero; 0] = [];
    let run = run_fixed(&p, 1000.0, 1, &[], None, WIDE, &heroes, 200);
    assert_eq!(run.x, 1096.0);
    assert_eq!(1000.0 + 112.0 - run.x, 16.0);
    assert_eq!(run.end.unwrap().0, 55);
    assert_eq!(run.finished, Some(95));
}

// ----------------------------------------------------------------- AI lane --

#[test]
fn the_rush_lane_reports_the_swept_extent_and_the_tick_counts() {
    let p = profile();
    // Facing +1 from 1000 with the tell just started: motion on the 32nd step.
    let state = RushState::start(1000.0, 1, HeroId::Donut, 1);
    let lane = rush_lane(&p, &state, 1000.0, 0.0, &[], WIDE, EDGE).unwrap();
    assert_eq!(lane.phase, RushPhase::Tell);
    assert_eq!(lane.dir, 1);
    assert_eq!(
        lane.ticks_to_motion, 31,
        "the 32nd step is the first active"
    );
    assert_eq!(lane.active_left, 24);
    assert_eq!((lane.y_from, lane.y_to), (2.0, 30.0));
    // The box at the first active tick is [1004, 1032); at the last [1096, 1124).
    assert_eq!((lane.x_from, lane.x_to), (1004.0, 1124.0));
    // Donut at pivot 1100 (half-width 10): the box first overlaps her column on
    // active tick 17 ([1068, 1096) does not, [1072, 1100) does): n = 31 + 16.
    let reach = ticks_until_box_reaches(&lane, 1100.0, 10.0).unwrap();
    let clear = ticks_until_box_clears(&lane, 1100.0, 10.0).unwrap();
    // The same answer from the machine itself.
    let mut real = state;
    let mut x = 1000.0;
    let mut first = None;
    let mut last = None;
    for n in 0..80u16 {
        let hero = RushHero {
            id: HeroId::Donut,
            hurtbox: Rect {
                left: 1090.0,
                bottom: 0.0,
                width: 20.0,
                height: 24.0,
            },
            hittable: false,
        };
        let world = RushWorld {
            feet: 0.0,
            solids: &[],
            partition: None,
            leash: WIDE,
            max_x: EDGE,
            heroes: &[hero],
        };
        let before = real.phase;
        let out = real.step(&p, x, &world);
        x = out.x;
        let boxed = rush_box(&p, x, 1, 0.0);
        if (before == RushPhase::Tell && real.phase == RushPhase::Committed
            || before == RushPhase::Committed)
            && boxed.overlaps(hero.hurtbox)
        {
            first.get_or_insert(n);
            last = Some(n);
        }
    }
    assert_eq!(Some(reach), first);
    assert_eq!(Some(clear), last.map(|l| l + 1));
    // Never reaches a column beyond the lane or behind the llama.
    assert_eq!(ticks_until_box_reaches(&lane, 1300.0, 10.0), None);
    assert_eq!(ticks_until_box_reaches(&lane, 900.0, 10.0), None);
    assert_eq!(ticks_until_box_clears(&lane, 1300.0, 10.0), None);
}

#[test]
fn the_box_overlaps_a_hero_column_for_11_or_12_ticks_donut_and_12_or_13_carl() {
    let p = profile();
    let state = RushState::start(1000.0, 1, HeroId::Donut, 1);
    let lane = rush_lane(&p, &state, 1000.0, 0.0, &[], WIDE, EDGE).unwrap();
    let span = |half: f32, x: f32| {
        let reach = ticks_until_box_reaches(&lane, x, half).unwrap();
        let clear = ticks_until_box_clears(&lane, x, half).unwrap();
        clear - reach
    };
    let counts = |half: f32| -> Vec<u16> {
        let mut v: Vec<u16> = (1050..=1086).map(|x| span(half, x as f32)).collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    // The box (28 px) sweeps 4 px a tick: a column of width 2h is overlapped on
    // 11 or 12 ticks (Donut) and 12 or 13 (Carl), depending on the sub-step
    // alignment of the hero's x. The sheet's earlier "up to 11 and 12" was the
    // grid-aligned count only (erratum recorded in section 9).
    assert_eq!(counts(DONUT_HALF_WIDTH), vec![11, 12]);
    assert_eq!(counts(CARL_HALF_WIDTH), vec![12, 13]);
}

/// A partner AI that reads `ticks_until_box_reaches` at its decision (before
/// its own step, each tick) and presses jump once, on the first tick where
/// `press_when(n)` holds. Hero steps come before the llama's in a tick, so the
/// helper's 0 (the next llama step) is the llama step of the same tick.
/// Returns `(pressed, hit)` for a hero at pivot distance `d`.
fn ai_jump_run(id: HeroId, d: f32, press_when: impl Fn(u16) -> bool) -> (bool, bool) {
    let p = profile();
    let half = match id {
        HeroId::Carl => CARL_HALF_WIDTH,
        HeroId::Donut => DONUT_HALF_WIDTH,
    };
    let mut hero = Hero1::new(id, 1000.0 + d, false);
    let mut state = RushState::start(1000.0, 1, id, 1);
    let mut x = 1000.0;
    let mut pressed = false;
    for t in 0..=60u16 {
        let mut jump = false;
        // Tick 0 is the tell start tick: the llama starts after the hero's step,
        // so nobody can have read the lane yet, and it has no llama step.
        if t == 0 {
            hero.step(false, false);
            continue;
        }
        if !pressed
            && let Some(lane) = rush_lane(&p, &state, x, 0.0, &[], WIDE, EDGE)
            && let Some(n) = ticks_until_box_reaches(&lane, hero.hero.x, half)
            && press_when(n)
        {
            jump = true;
            pressed = true;
        }
        hero.step(jump, false);
        let heroes = [hero.rush_hero(id)];
        let world = RushWorld {
            feet: 0.0,
            solids: &[],
            partition: None,
            leash: WIDE,
            max_x: EDGE,
            heroes: &heroes,
        };
        let out = state.step(&p, x, &world);
        x = out.x;
        if out.blow.is_some() {
            return (pressed, true);
        }
        if state.phase == RushPhase::Recover {
            break;
        }
    }
    (pressed, false)
}

/// The partner's jump bounds (sheet section 9): Donut jumps when
/// `ticks_until_box_reaches` is 3..40, Carl when it is 4..31. The helper counts
/// from 0 (0 = the next llama step) and CA2 must use the value as returned,
/// without adding 1. This pins the contract with the real hero physics: a hero
/// who presses when the helper first reads a value inside the bound is never
/// hit, at any sub-step alignment, and the bounds have no slack (one tick
/// earlier or later is a hit for some alignment).
#[test]
fn the_ai_jump_bounds_hold_against_the_helper_counted_from_zero() {
    for (id, lo, hi) in [(HeroId::Donut, 3u16, 40u16), (HeroId::Carl, 4, 31)] {
        let grid: Vec<f32> = (4..=512).map(|q| q as f32 * 0.25).collect();
        let mut fired = 0;
        let mut late_hits = 0;
        let mut early_hits = 0;
        for &d in &grid {
            // Inside the bound: the AI presses on the first tick with n <= hi
            // (n >= lo holds on that tick or the press is not made: the AI
            // acts only while n is in range).
            let (pressed, hit) = ai_jump_run(id, d, |n| (lo..=hi).contains(&n));
            if pressed {
                fired += 1;
                assert!(!hit, "{id:?} d {d}: hit inside the bound");
            }
            // The AI that presses one tick too late / too early is hit for some
            // alignments: the bounds have no slack. A smaller `n` is closer to
            // contact, so `lo - 1` is a late press and `hi + 1` an early one.
            let (p_late, hit_late) = ai_jump_run(id, d, |n| n == lo - 1);
            late_hits += usize::from(p_late && hit_late);
            let (p_early, hit_early) = ai_jump_run(id, d, |n| n == hi + 1);
            early_hits += usize::from(p_early && hit_early);
        }
        assert!(fired > 300, "{id:?}: the AI fired on {fired} starts");
        assert!(
            late_hits > 0,
            "{id:?}: a press at lo - 1 (late) is never a hit"
        );
        assert!(
            early_hits > 0,
            "{id:?}: a press at hi + 1 (early) is never a hit"
        );
    }
}

/// #201 fix-forward (re-review note 1): every exact helper value `n` in
/// `lo..=hi` is a safe press, at every sub-step alignment of the hero's x, not
/// only the first in-range value the loop above presses on. CA2 may engage
/// late, so the whole window is a permanent contract.
#[test]
fn every_press_value_in_the_jump_window_is_safe_at_every_alignment() {
    for (id, lo, hi) in [(HeroId::Donut, 3u16, 40u16), (HeroId::Carl, 4, 31)] {
        let grid: Vec<f32> = (4..=512).map(|q| q as f32 * 0.25).collect();
        for k in lo..=hi {
            let mut pressed_runs = 0;
            for &d in &grid {
                let (pressed, hit) = ai_jump_run(id, d, |n| n == k);
                if pressed {
                    pressed_runs += 1;
                    assert!(!hit, "{id:?} d {d}: a press at n = {k} was hit");
                }
            }
            assert!(pressed_runs > 0, "{id:?}: n = {k} was never reached");
        }
    }
}

#[test]
fn the_lane_follows_solids_and_the_leash_and_vanishes_in_recover() {
    let p = profile();
    let wall = [solid(1084.0, 16.0, 200.0)];
    let state = RushState::start(1000.0, 1, HeroId::Carl, 1);
    let lane = rush_lane(&p, &state, 1000.0, 0.0, &wall, WIDE, EDGE).unwrap();
    // Face 84: stops at 1056 on active tick 15; the lane ends at the face.
    assert_eq!(lane.x_to, 1084.0);
    assert_eq!(lane.active_left, 15);
    let leash = Leash {
        min: 0.0,
        max: 1040.0,
    };
    let lane = rush_lane(&p, &state, 1000.0, 0.0, &[], leash, EDGE).unwrap();
    assert_eq!(lane.active_left, 11);
    assert_eq!(lane.x_to, 1040.0 + 28.0);
    let mut s = state;
    s.phase = RushPhase::Recover;
    assert!(rush_lane(&p, &s, 1000.0, 0.0, &[], WIDE, EDGE).is_none());
    s.phase = RushPhase::Bonk;
    assert!(rush_lane(&p, &s, 1000.0, 0.0, &[], WIDE, EDGE).is_none());
    // Mid-commit: ticks_to_motion is 0 and the remaining steps shrink.
    let mut s = state;
    let mut x = 1000.0;
    for _ in 0..40 {
        let world = RushWorld {
            feet: 0.0,
            solids: &[],
            partition: None,
            leash: WIDE,
            max_x: EDGE,
            heroes: &[],
        };
        x = s.step(&p, x, &world).x;
    }
    let lane = rush_lane(&p, &s, x, 0.0, &[], WIDE, EDGE).unwrap();
    assert_eq!(lane.ticks_to_motion, 0);
    assert_eq!(lane.active_left, 24 - 9);
}

/// N-a (#201 review): the llama brain's target rule and the goblins' shared
/// `pick_two` are one rule. Every position pair, bias, held hero and switch.
#[test]
fn pick_target_and_pick_two_are_the_same_rule() {
    let hero = |id: HeroId, x: f32, bias: f32| HeroFacts {
        id,
        x,
        hurtbox: Rect {
            left: x - 10.0,
            bottom: 0.0,
            width: 20.0,
            height: 24.0,
        },
        support: 0.0,
        grounded: true,
        alive: true,
        bias_px: bias,
    };
    let class = HeroClass {
        rushable: true,
        spittable: true,
    };
    let llama_x = 1000.0;
    for carl_x in (880..=1000).step_by(8) {
        for donut_x in (880..=1000).step_by(8) {
            for bias in [0.0f32, 40.0] {
                for (carl_bias, donut_bias) in [(0.0, bias), (bias, 0.0), (0.0, 0.0)] {
                    let carl = hero(HeroId::Carl, carl_x as f32, carl_bias);
                    let donut = hero(HeroId::Donut, donut_x as f32, donut_bias);
                    let candidates = [(&carl, class), (&donut, class)];
                    for held in [None, Some(HeroId::Carl), Some(HeroId::Donut)] {
                        let eff = |h: &HeroFacts| (h.x - llama_x).abs() + h.bias_px;
                        let expected = pick_two(eff(&carl), eff(&donut), held, Some(24.0));
                        let got = pick_target(llama_x, &candidates, &[0, 1], held, 24.0)
                            .map(|i| candidates[i].0.id);
                        assert_eq!(got, Some(expected), "{carl_x} {donut_x} {held:?}");
                    }
                    // No hysteresis (the nearest rule): `None` switch and no held.
                    let nearest = pick_two(
                        (carl.x - llama_x).abs() + carl.bias_px,
                        (donut.x - llama_x).abs() + donut.bias_px,
                        None,
                        None,
                    );
                    let got = pick_target(llama_x, &candidates, &[0, 1], None, 24.0)
                        .map(|i| candidates[i].0.id);
                    assert_eq!(got, Some(nearest));
                }
            }
        }
    }
    // The goblins' tie goes to Carl.
    assert_eq!(pick_two(50.0, 50.0, None, Some(24.0)), HeroId::Carl);
}
