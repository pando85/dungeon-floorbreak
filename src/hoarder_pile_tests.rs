//! HRD-B and HRD-C: the Hoarder's two piles in the real scene (bank A, the J-B stack and its
//! post). Library tests on the room-1 rig: tick numbers follow the sheet's counting convention
//! (`docs/01-design/enemy-gameplay-hoarder.md` sections 5 and 6, the entry tick is tick 1).
//! Prop 0 is bank A, prop 1 the J-B stack (the sidecar's statics, in file order).

use crate::combat::{CombatEvent, PropPhase};
use crate::combat_scene::{ENV_SOURCE_ID, EnvSolidKind};
use crate::env_prop::{MassState, SlideState, StimulusKind};
use crate::hoarder_fixtures::{
    ENV_SOLIDS, IDLE, Plan, Rig, attack_input, carl_input, jump_input, run_input,
};
use crate::party::HeroId;
use crate::pile_boss::{ObstacleKind, PileEvent, PileMode};
use std::path::PathBuf;

const BANK: usize = 0;
const STACK: usize = 1;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn slide(rig: &Rig) -> SlideState {
    rig.combat.env().unwrap().slide(BANK).unwrap().state()
}

fn mass(rig: &Rig) -> MassState {
    rig.combat.env().unwrap().mass_state(STACK).unwrap()
}

fn progress(rig: &Rig) -> u8 {
    rig.combat.env().unwrap().mass(STACK).unwrap().progress()
}

fn carl_foot(rig: &Rig) -> i32 {
    rig.combat.env_hero_foot_layout(&rig.scene, HeroId::Carl)
}

/// The tick of the first `PropPhase` of `prop` with `phase`.
fn phase_tick(rig: &Rig, prop: u16, phase: PropPhase) -> Option<u64> {
    rig.combat.events.iter().find_map(|e| match *e {
        CombatEvent::PropPhase {
            tick,
            prop: p,
            phase: ph,
            ..
        } if p == prop && ph == phase => Some(tick),
        _ => None,
    })
}

/// The tick of the first counted hit (`PropHit`) on the post at or after `from`.
fn post_hit_tick(rig: &Rig, from: u64) -> Option<u64> {
    rig.combat.events.iter().find_map(|e| match *e {
        CombatEvent::PropHit { tick, prop: 1, .. } if tick >= from => Some(tick),
        _ => None,
    })
}

fn pile_tick(rig: &Rig, want: impl Fn(&PileEvent) -> bool) -> Option<u64> {
    rig.piles().iter().find(|(_, e)| want(e)).map(|(t, _)| *t)
}

/// Dormant room, Carl at `x`, the gate open, nothing awake.
fn dormant(x: i32) -> Rig {
    Rig::new(
        &root(),
        Plan {
            carl_x: x,
            ..Plan::carl_at(60.0)
        }
        .dormant(),
    )
    .unwrap()
}

/// Carl at `carl`, her awake at pivot `her` facing `facing` (the gate closed, engaged).
fn awake(carl: i32, her: f32, facing: i8) -> Rig {
    Rig::new(
        &root(),
        Plan {
            carl_x: carl,
            awake: Some((her, facing)),
            ..Plan::carl_at(60.0)
        },
    )
    .unwrap()
}

// ------------------------------------------------------------------ bank A --

/// The owner's report: Carl walked through bank A. It is a solid now: its side blocks him
/// (his right edge stops at its left face, 352) and he lands on its 48 px top and stands there.
#[test]
fn a_hero_is_blocked_by_bank_a_and_lands_on_its_top() {
    let mut rig = dormant(300);
    rig.run(90, &|_| carl_input(run_input(1.0)));
    let x = rig.carl_x();
    assert!(
        (330.0..=340.5).contains(&x),
        "stopped at the side, right edge at 352: x {x}"
    );
    assert_eq!(carl_foot(&rig), 580, "still on the lane");
    // Jump it from the face: hold right for the first ticks of the jump only, so he comes down on
    // the top (an ordinary jump apex is 88 px, the top 48).
    let mut rig = dormant(300);
    rig.run(75, &|_| carl_input(run_input(1.0)));
    rig.step(carl_input(crate::input::RawInput {
        move_x: 1.0,
        ..jump_input()
    }));
    rig.run(14, &|_| carl_input(run_input(1.0)));
    rig.run(80, &|_| IDLE);
    assert!(rig.scene.world.hero.grounded, "landed");
    assert_eq!(carl_foot(&rig), 532, "on the 48 px top: layout y 580 - 48");
    // Bank A is the env solid the sync keeps intact; its top is standable.
    let env = rig.combat.env().unwrap();
    let bank = env
        .solids()
        .iter()
        .position(|s| s.kind == EnvSolidKind::Bank)
        .unwrap();
    assert!(env.solid_intact(bank));
    assert!(rig.scene.world.solids[env.solid_range().start + bank].intact);
}

/// Both heroes meet the same solid: Donut is blocked too, from either side.
#[test]
fn bank_a_blocks_donut_and_a_hero_coming_from_the_right() {
    let mut rig = dormant(300);
    rig.run(150, &|_| carl_input(run_input(1.0)));
    let bank_left = 352.0;
    assert!(rig.carl_x() <= bank_left - 11.0, "{}", rig.carl_x());
    // From the right: Carl at x 430 (gate open, before the engage line is only crossed after
    // 8 ticks) walks left and stops at the right face 416.
    let mut rig = dormant(428);
    rig.step(IDLE);
    rig.run(40, &|_| carl_input(run_input(-1.0)));
    assert!(rig.carl_x() >= 416.0 + 11.0, "{}", rig.carl_x());
}

/// HRD-Q22: hero melee never disturbs the bank (ten jabs and a missile line), so cover cannot be
/// knocked over by accident.
#[test]
fn hero_melee_never_disturbs_bank_a() {
    let mut rig = dormant(330);
    rig.run(120, &|t| {
        if t.is_multiple_of(2) {
            carl_input(attack_input())
        } else {
            IDLE
        }
    });
    assert_eq!(slide(&rig), SlideState::Stable);
    assert!(phase_tick(&rig, 0, PropPhase::Warning).is_none());
}

/// The bait (HRD-Q21, HRD-Q23): her shove jams on the face on tick J, the bank is `disturbed` on J,
/// slides J+24..J+39, `settled` from J+40; Carl stands still behind it and the mass reaches him:
/// one push, 0 damage, credited to nobody (a hero who stood still baited nothing).
#[test]
fn a_baited_shove_disturbs_bank_a_slides_it_once_and_pushes_a_hero_once() {
    let mut rig = awake(330, 470.0, -1);
    let mut states = Vec::new();
    for _ in 0..200 {
        rig.step(IDLE);
        states.push((rig.next_tick() - 1, slide(&rig)));
    }
    let j = pile_tick(&rig, |e| {
        matches!(
            e,
            PileEvent::Jammed {
                obstacle: ObstacleKind::BankA
            }
        )
    })
    .expect("jam on bank A");
    let at = |t: u64| states.iter().find(|(tick, _)| *tick == t).unwrap().1;
    assert_eq!(at(j - 1), SlideState::Stable);
    assert_eq!(at(j), SlideState::Disturbed, "disturbed on J");
    assert_eq!(
        at(j + 23),
        SlideState::Slid,
        "the step of J+23 starts the slide"
    );
    assert_eq!(at(j + 39), SlideState::Slid);
    assert_eq!(at(j + 40), SlideState::Settled, "settled from J+40");
    // The phases: the shudder (Warning) on J, the slide (Falling) on J+23, the settle (Landed).
    assert_eq!(phase_tick(&rig, 0, PropPhase::Warning), Some(j));
    assert_eq!(phase_tick(&rig, 0, PropPhase::Falling), Some(j + 23));
    assert_eq!(phase_tick(&rig, 0, PropPhase::Landed), Some(j + 40));
    // Once: exactly one slide, one Landed.
    let landed = rig
        .combat
        .events
        .iter()
        .filter(|e| {
            matches!(
                e,
                CombatEvent::PropPhase {
                    prop: 0,
                    phase: PropPhase::Landed,
                    ..
                }
            )
        })
        .count();
    assert_eq!(landed, 1);
    // One push on Carl: an environment blow of 0 damage (his HP never changed).
    let pushes: Vec<_> = rig
        .hurts()
        .into_iter()
        .filter(|(_, src, _, hero)| *src == ENV_SOURCE_ID && *hero == 0)
        .collect();
    assert_eq!(
        pushes.len(),
        1,
        "one push across the 16 slide ticks: {pushes:?}"
    );
    assert!(
        (j + 24..=j + 40).contains(&pushes[0].0),
        "pushed while the mass moves: {pushes:?} J={j}"
    );
    let full = rig.scene.world.body(HeroId::Carl).health;
    assert_eq!(pushes[0].2, full, "0 damage: health left equals health");
    // After the settle the bank is a 16 px remnant: nobody stands inside it.
    assert!(matches!(carl_foot(&rig), 564 | 580));
    // The attribution: she was targeting the idle Carl, a human: credit Carl (0).
    let credit = rig.combat.events.iter().find_map(|e| match e {
        CombatEvent::PropHit {
            prop: 0, player, ..
        } => Some(*player),
        _ => None,
    });
    assert_eq!(credit, Some(Some(0)), "credited to the player she baited");
}

/// The remnant (HRD-Q21): a 16 px solid that stands for the rest of the attempt, jumped by an
/// ordinary jump, and no face of hers: her shove and her body never stop on it.
#[test]
fn the_settled_bank_is_a_16_px_remnant_and_no_face_of_hers() {
    let mut rig = awake(330, 470.0, -1);
    rig.run(160, &|_| IDLE);
    assert_eq!(slide(&rig), SlideState::Settled);
    let env = rig.combat.env().unwrap();
    assert!(
        !env.faces()
            .iter()
            .any(|(_, k, _)| *k == crate::scene_enemy::FaceKind::Bank),
        "the remnant is no face"
    );
    let remnant = env
        .solids()
        .iter()
        .enumerate()
        .find(|(i, s)| s.kind == EnvSolidKind::Remnant && env.solid_intact(*i))
        .expect("the bank's remnant stands");
    assert_eq!(
        (remnant.1.rect.x, remnant.1.rect.w, remnant.1.rect.h),
        (296, 80, 16)
    );
    // Carl jumps from where the slide left him and comes down on the lane or the remnant top, never
    // inside the solid.
    let mut rig = rig;
    rig.step(carl_input(jump_input()));
    rig.run(70, &|_| IDLE);
    assert!(rig.scene.world.hero.grounded);
    assert!(matches!(carl_foot(&rig), 564 | 580), "{}", carl_foot(&rig));
}

/// Restart mid-slide: the machine is back to `stable`, and the state hash equals a fresh attempt's.
#[test]
fn a_restart_mid_slide_is_a_fresh_attempt() {
    let mut rig = awake(330, 470.0, -1);
    while slide(&rig) != SlideState::Slid {
        rig.step(IDLE);
    }
    rig.combat.restart(&mut rig.scene);
    assert_eq!(slide(&rig), SlideState::Stable);
    let fresh = awake(330, 470.0, -1);
    assert_eq!(
        rig.combat.env().unwrap().solid_intact(1),
        fresh.combat.env().unwrap().solid_intact(1)
    );
    let mut env_a = Vec::new();
    rig.combat.env().unwrap().mix_into(&mut |v| env_a.push(v));
    let mut env_b = Vec::new();
    fresh.combat.env().unwrap().mix_into(&mut |v| env_b.push(v));
    assert_eq!(env_a, env_b, "the env hash is the pristine one again");
}

// ---------------------------------------------------------------- J-B post --

/// HRD-Q25: the post is a hit target, never a solid: a hero walks straight through it, and no env
/// solid has its rectangle.
#[test]
fn the_post_is_not_solid() {
    let mut rig = dormant(790);
    rig.run(70, &|_| carl_input(run_input(1.0)));
    assert!(
        rig.carl_x() > 836.0,
        "walked through the post: {}",
        rig.carl_x()
    );
    let env = rig.combat.env().unwrap();
    // A solid only for the bank, the two remnants (when they stand), the door and the gate.
    for (i, s) in env.solids().iter().enumerate() {
        let post = (808, 516, 16, 64);
        assert_ne!(
            (s.rect.x, s.rect.y, s.rect.w, s.rect.h),
            post,
            "solid {i} is the post"
        );
    }
    assert_eq!(env.solids().len(), ENV_SOLIDS);
    // The face list gives her a post face to jam on, and it is not a solid of the scene.
    assert!(
        env.faces()
            .iter()
            .any(|(_, k, _)| *k == crate::scene_enemy::FaceKind::Post)
    );
}

/// HRD-Q26: two distinct qualifying hits release the stack; `released` R..R+23, `falling` R+24..R+31,
/// the landing evaluation runs on R+31 once, `landed` from R+32. Her at pivot 300 (far away).
#[test]
fn two_hits_on_the_post_drop_the_stack_on_the_sheets_clock() {
    let mut rig = awake(790, 290.0, 1);
    // A first hit weakens, nothing more.
    let mut weakened_at = None;
    let mut released_at = None;
    let mut states = Vec::new();
    for i in 0..220 {
        let tick = rig.next_tick();
        rig.step(if tick.is_multiple_of(2) && released_at.is_none() {
            carl_input(attack_input())
        } else {
            IDLE
        });
        states.push((tick, mass(&rig)));
        if weakened_at.is_none() && progress(&rig) == 1 {
            weakened_at = Some(tick);
        }
        if released_at.is_none() && mass(&rig) == MassState::Released {
            released_at = Some(tick);
        }
        let _ = i;
    }
    let weakened = weakened_at.expect("the first hit weakens");
    let r = released_at.expect("the second hit releases");
    assert!(
        weakened < r,
        "two distinct hits: weakened {weakened}, released {r}"
    );
    let at = |t: u64| states.iter().find(|(tick, _)| *tick == t).unwrap().1;
    assert_eq!(at(r), MassState::Released);
    assert_eq!(
        at(r + 23),
        MassState::Falling,
        "falling from the step of R+23"
    );
    assert_eq!(at(r + 30), MassState::Falling);
    assert_eq!(at(r + 31), MassState::Landed, "landing evaluated on R+31");
    assert_eq!(phase_tick(&rig, 1, PropPhase::Released), Some(r));
    assert_eq!(phase_tick(&rig, 1, PropPhase::Landed), Some(r + 31));
    // The remnant stands after it: a 16 px solid.
    let env = rig.combat.env().unwrap();
    let remnants: Vec<_> = env
        .solids()
        .iter()
        .enumerate()
        .filter(|(i, s)| s.kind == EnvSolidKind::Remnant && env.solid_intact(*i))
        .collect();
    assert_eq!(remnants.len(), 1);
    assert_eq!(remnants[0].1.rect.h, 16);
    assert_eq!(remnants[0].1.rect.x, 752);
}

/// A single jab touching the post over several active ticks is one hit (one move instance).
#[test]
fn one_move_instance_counts_once() {
    let mut rig = awake(790, 290.0, 1);
    rig.run(1, &|_| carl_input(attack_input()));
    rig.run(40, &|_| IDLE);
    assert_eq!(progress(&rig), 1, "one jab, one weight");
    assert_eq!(mass(&rig), MassState::Weakened);
}

/// Hits before she is out (Dormant, then the whole 90-tick Reveal) do nothing (HRD-Q26, HRDC-AD4):
/// solo Carl at 790 jabs the post from tick 1. Every tick whose hits meet her Dormant or in the
/// Reveal leaves the post untouched (no progress, no prop event); the first counted hit comes after
/// she is out, and two of them release the stack.
#[test]
fn hits_before_she_is_out_do_nothing() {
    // Carl at 790 already past the engage line: she engages 8 ticks after tick 1 (gate settle).
    let mut rig = dormant(790);
    let jab = |t: u64| {
        if t.is_multiple_of(2) {
            carl_input(attack_input())
        } else {
            IDLE
        }
    };
    let mut out_at = None;
    let mut reveal_ticks = 0;
    for _ in 0..200 {
        let tick = rig.next_tick();
        // The hero strike runs before her step, so this tick's hits meet the state of now.
        let open = rig.boss().boss().is_out();
        assert_eq!(
            rig.combat
                .env_hit_counts_for_test(&rig.scene, 1, HeroId::Carl),
            open,
            "tick {tick}: the post opens exactly when she is out"
        );
        let before = progress(&rig);
        rig.step(jab(tick));
        if !open {
            assert_eq!(
                progress(&rig),
                before,
                "tick {tick}: not out yet, no hit counts"
            );
            assert_eq!(mass(&rig), MassState::Supported, "tick {tick}");
        }
        if rig.mode() == PileMode::Reveal {
            reveal_ticks += 1;
        }
        if out_at.is_none() && rig.boss().boss().is_out() {
            out_at = Some(tick);
        }
    }
    assert_eq!(reveal_ticks, 90, "the full first Reveal was jabbed through");
    let out_at = out_at.expect("she comes out");
    let engaged = pile_tick(&rig, |e| *e == PileEvent::Engaged).expect("Engaged");
    assert_eq!(out_at, engaged + 90, "out on the first Neutral tick");
    // No prop event of the post while she was hidden.
    assert!(rig.combat.events.iter().all(|e| match *e {
        CombatEvent::PropPhase { tick, prop: 1, .. }
        | CombatEvent::PropHit { tick, prop: 1, .. } => tick > out_at,
        _ => true,
    }));
    // After it the same jabs count: weakened on a hit after `out_at`, then released.
    let weakened = post_hit_tick(&rig, 0).expect("a jab weakens the post");
    let released = phase_tick(&rig, 1, PropPhase::Released).expect("a second jab releases it");
    assert!(
        out_at < weakened && weakened < released,
        "out {out_at}, weakened {weakened}, released {released}"
    );
    assert!(
        weakened <= out_at + 24,
        "the first jab after she is out counts: weakened {weakened}, out {out_at}"
    );
}

/// The window's two edges (HRDC-AD4), a human Donut's swat on the post evaluated as the real strike
/// is (before her step): on the tick whose step ends the Reveal the swat still meets her Reveal and
/// counts nothing; on the next tick, the first she can be hit, it counts.
#[test]
fn the_post_opens_on_the_first_tick_she_can_be_hit() {
    let plan = Plan {
        carl_x: 600,
        donut_x: 790,
        ..Plan::carl_at(60.0)
    }
    .dormant()
    .with_control(crate::hoarder_fixtures::Control::SoloDonut);
    // A probe run finds T, the tick whose step takes her out of the Reveal.
    let mut probe = Rig::new(&root(), plan).unwrap();
    let mut t_out = None;
    for _ in 0..300 {
        let tick = probe.next_tick();
        probe.step(IDLE);
        if probe.boss().boss().is_out() {
            t_out = Some(tick);
            break;
        }
    }
    let t_out = t_out.expect("she comes out");
    let mut rig = Rig::new(&root(), plan).unwrap();
    rig.run(t_out - 1, &|_| IDLE);
    assert_eq!(rig.mode(), PileMode::Reveal, "the last Reveal state");
    let layout_h = rig.scene.world.layout_height();
    let floor = (layout_h - 580) as f32;
    let swat = crate::prototype::Rect {
        left: 806.0,
        bottom: floor + 4.0,
        width: 10.0,
        height: 20.0,
    };
    // Tick T: the strike meets the Reveal.
    assert!(
        !rig.combat
            .env_hit_counts_for_test(&rig.scene, 1, HeroId::Donut)
    );
    rig.combat
        .env_donut_melee_for_test(&rig.scene, swat, 1, t_out);
    assert_eq!(progress(&rig), 0, "the last Reveal tick: nothing");
    rig.step(IDLE);
    assert!(rig.boss().boss().is_out(), "her step on T took her out");
    // Tick T + 1: the first tick a hero hit can reach her; the post takes it.
    assert!(
        rig.combat
            .env_hit_counts_for_test(&rig.scene, 1, HeroId::Donut)
    );
    rig.combat
        .env_donut_melee_for_test(&rig.scene, swat, 2, t_out + 1);
    assert_eq!(progress(&rig), 1, "the first hittable tick counts");
    assert_eq!(mass(&rig), MassState::Weakened);
}

/// The retry path (HRD-Q05, 30-tick Reveal): after a restart she is Dormant again, the post is
/// closed again until she is out of the short Reveal, and it opens on the same edge.
#[test]
fn the_retry_reveal_keeps_the_post_closed_too() {
    let mut rig = dormant(790);
    while !rig.boss().boss().is_out() {
        rig.step(IDLE);
        assert!(rig.next_tick() < 300, "she never came out");
    }
    rig.combat.restart(&mut rig.scene);
    assert_eq!(rig.mode(), PileMode::Dormant, "fresh state");
    assert_eq!(mass(&rig), MassState::Supported);
    assert!(rig.carl_x() > 432.0, "Carl respawns past the engage line");
    let start = rig.next_tick();
    let mut reveal_ticks = 0;
    let mut out_at = None;
    for _ in 0..160 {
        let tick = rig.next_tick();
        let open = rig.boss().boss().is_out();
        assert_eq!(
            rig.combat
                .env_hit_counts_for_test(&rig.scene, 1, HeroId::Carl),
            open,
            "tick {tick}"
        );
        let before = progress(&rig);
        rig.step(if tick.is_multiple_of(2) {
            carl_input(attack_input())
        } else {
            IDLE
        });
        if !open {
            assert_eq!(progress(&rig), before, "tick {tick}: hidden, no hit counts");
        }
        if rig.mode() == PileMode::Reveal {
            reveal_ticks += 1;
        }
        if out_at.is_none() && rig.boss().boss().is_out() {
            out_at = Some(tick);
        }
    }
    assert_eq!(reveal_ticks, 30, "the retry Reveal");
    let out_at = out_at.expect("she comes out of the retry Reveal");
    let weakened =
        post_hit_tick(&rig, start).expect("a jab after the retry Reveal weakens the post");
    assert!(weakened > out_at, "weakened {weakened}, out {out_at}");
    let released = rig
        .combat
        .events
        .iter()
        .find_map(|e| match *e {
            CombatEvent::PropPhase {
                tick,
                prop: 1,
                phase: PropPhase::Released,
                ..
            } if tick >= start => Some(tick),
            _ => None,
        })
        .expect("a second jab releases the stack");
    assert!(released > weakened);
}

#[test]
fn a_bot_driven_hero_never_triggers_a_pile() {
    // P1 controls Donut; Carl is the bot: his strike never counts (the Juicer rule).
    let rig = Rig::new(
        &root(),
        Plan {
            carl_x: 790,
            awake: Some((290.0, 1)),
            ..Plan::carl_at(60.0)
        }
        .with_control(crate::hoarder_fixtures::Control::SoloDonut),
    )
    .unwrap();
    assert!(
        !rig.combat
            .env_hit_counts_for_test(&rig.scene, 1, HeroId::Carl),
        "Carl is the bot"
    );
    assert!(
        rig.combat
            .env_hit_counts_for_test(&rig.scene, 1, HeroId::Donut)
    );
    // A plain prop (none here) always counts: the Boiler's rule is untouched.
    let rig2 = awake(790, 290.0, 1);
    assert!(
        rig2.combat
            .env_hit_counts_for_test(&rig2.scene, 1, HeroId::Carl)
    );
}

/// Donut's Magic Missile is a qualifying hit (DV-5, HRD-Q26): two missiles from a human Donut
/// release the stack, the first weakens it, and each counted bolt ends on the post.
#[test]
fn two_magic_missiles_release_the_stack() {
    let mut rig = Rig::new(
        &root(),
        Plan {
            carl_x: 112,
            donut_x: 700,
            awake: Some((290.0, 1)),
            ..Plan::carl_at(60.0)
        }
        .with_control(crate::hoarder_fixtures::Control::SoloDonut),
    )
    .unwrap();
    rig.run(300, &|t| {
        if t % 24 == 1 {
            carl_input(attack_input())
        } else {
            IDLE
        }
    });
    let weakened = post_hit_tick(&rig, 0).expect("the first missile weakens");
    let released = phase_tick(&rig, 1, PropPhase::Released).expect("the second releases");
    assert!(weakened < released, "two distinct missiles");
    assert_eq!(phase_tick(&rig, 1, PropPhase::Landed), Some(released + 31));
    assert_eq!(mass(&rig), MassState::Landed);
    // Both counted hits are Donut's projectiles.
    let hits: Vec<_> = rig
        .combat
        .events
        .iter()
        .filter_map(|e| match e {
            CombatEvent::PropHit {
                prop: 1,
                kind,
                player,
                ..
            } => Some((*kind, *player)),
            _ => None,
        })
        .collect();
    assert_eq!(
        hits,
        vec![
            (
                StimulusKind::Projectile,
                Some(crate::donut_moves::DONUT_HERO)
            );
            2
        ],
        "two counted missiles, both Donut's"
    );
}

/// The review's case (HRDC-AD4): a human Donut fires Magic Missiles at the post from its right
/// while she emerges under the stack. Every bolt of the Reveal passes the post (it is not there for
/// a bolt yet) and the post shows no damage; once she is out, two bolts release the stack, and a
/// stack dropped then can find her body.
#[test]
fn a_human_donut_s_missiles_during_the_reveal_pass_the_post() {
    let mut rig = Rig::new(
        &root(),
        Plan {
            carl_x: 600,
            donut_x: 900,
            ..Plan::carl_at(60.0)
        }
        .dormant()
        .with_control(crate::hoarder_fixtures::Control::SoloDonut),
    )
    .unwrap();
    // Face -x, then fire at the refire cadence from tick 4.
    let script = |t: u64| {
        if t <= 3 {
            carl_input(run_input(-1.0))
        } else if t % 24 == 4 {
            carl_input(attack_input())
        } else {
            IDLE
        }
    };
    assert_missiles_wait_for_her(&mut rig, &script);
}

/// The 2P variant: Carl (P1) waits at the post and jabs it through the Reveal, Donut (P2) fires at
/// it from its right. Nothing counts until she is out; then the stack drops.
#[test]
fn a_2p_party_at_the_post_cannot_drop_the_stack_during_the_reveal() {
    let mut rig = Rig::new(
        &root(),
        Plan {
            carl_x: 790,
            donut_x: 900,
            ..Plan::carl_at(60.0)
        }
        .dormant()
        .with_control(crate::hoarder_fixtures::Control::TwoPlayer),
    )
    .unwrap();
    let script = |t: u64| {
        let carl = if t.is_multiple_of(2) {
            attack_input()
        } else {
            crate::input::RawInput::NONE
        };
        let donut = if t <= 3 {
            run_input(-1.0)
        } else if t % 24 == 4 {
            attack_input()
        } else {
            crate::input::RawInput::NONE
        };
        [carl, donut]
    };
    assert_missiles_wait_for_her(&mut rig, &script);
}

/// Runs `script` until the stack is released: the post takes nothing while she is Dormant or in the
/// Reveal, at least one bolt flies past the post in that time, and the release comes after she is
/// out.
fn assert_missiles_wait_for_her(
    rig: &mut Rig,
    script: &dyn Fn(u64) -> [crate::input::RawInput; 2],
) {
    let mut out_at = None;
    let mut passed = false;
    for _ in 0..400 {
        let tick = rig.next_tick();
        let open = rig.boss().boss().is_out();
        rig.step(script(tick));
        if !open {
            assert_eq!(progress(rig), 0, "tick {tick}: the post shows no damage");
            assert_eq!(mass(rig), MassState::Supported, "tick {tick}");
            passed |= rig.combat.donut_bolts().iter().any(|b| b.x < 800.0);
        }
        if out_at.is_none() && rig.boss().boss().is_out() {
            out_at = Some(tick);
        }
        if mass(rig) == MassState::Released {
            break;
        }
    }
    let engaged = pile_tick(rig, |e| *e == PileEvent::Engaged).expect("Engaged");
    let out_at = out_at.expect("she comes out");
    assert_eq!(out_at, engaged + 90);
    assert!(passed, "a bolt of the Reveal flew past the post");
    let released = phase_tick(rig, 1, PropPhase::Released).expect("released once she is out");
    assert!(released > out_at, "released {released}, out {out_at}");
}

/// HRD-Q26's switch off (`hrdq26_missile = 0`): a Magic Missile is no stimulus of the post and the
/// bolt passes it instead of ending there; melee still counts. With the switch on, the same bolt
/// ends on the post and weakens it.
#[test]
fn the_missile_switch_off_lets_a_bolt_pass_the_post() {
    let plan = Plan {
        carl_x: 112,
        donut_x: 900,
        awake: Some((290.0, 1)),
        ..Plan::carl_at(60.0)
    }
    .with_control(crate::hoarder_fixtures::Control::SoloDonut);
    let switch_off = |run: &mut crate::f01_hoarder::HoarderRun| {
        let stack = run
            .loaded
            .encounter
            .statics
            .iter_mut()
            .find_map(|st| st.stack.as_mut())
            .expect("the post's stack");
        stack.config.projectile_weight = 0;
    };
    let bolt_at = |rig: &Rig| {
        let floor = (rig.scene.world.layout_height() - 580) as f32;
        crate::combat_scene::Bolt {
            instance: 7,
            x: 830.0,
            y: floor + 30.0,
            dir: -1,
            travelled: 0.0,
        }
    };
    let mut off = Rig::new_edited(&root(), plan, &switch_off).unwrap();
    let tick = off.next_tick();
    let bolt = bolt_at(&off);
    assert!(
        !off.combat.env_missile_hit_for_test(&bolt, &off.scene, tick),
        "the bolt passes the post"
    );
    assert_eq!(progress(&off), 0);
    assert_eq!(mass(&off), MassState::Supported);
    // In the scene: four bolts from the right leave the post untouched.
    off.run(120, &|t| {
        if t <= 3 {
            carl_input(run_input(-1.0))
        } else if t % 24 == 4 {
            carl_input(attack_input())
        } else {
            IDLE
        }
    });
    assert_eq!(progress(&off), 0, "no missile counted");
    assert_eq!(mass(&off), MassState::Supported);
    // The switch on (the sidecar): the same bolt ends on the post.
    let mut on = Rig::new(&root(), plan).unwrap();
    let bolt = bolt_at(&on);
    assert!(on.combat.env_missile_hit_for_test(&bolt, &on.scene, tick));
    assert_eq!(progress(&on), 1);
}

/// The bait (HRD-Q18, HRD-Q26 heavy impact): her shove, clamped on the post's left face, leaves her
/// at pivot 760; the stack is released at once (J); the landing evaluation on J+31 finds her body
/// under it: 6 damage (HP 30 -> 24), `Exposed` 150 ticks, and a hero in the footprint takes 1.
#[test]
fn a_baited_shove_into_the_post_drops_the_stack_on_her() {
    let mut rig = awake(852, 700.0, 1);
    let mut j = None;
    for _ in 0..260 {
        rig.step(IDLE);
        if j.is_none() {
            j = pile_tick(&rig, |e| {
                matches!(
                    e,
                    PileEvent::Jammed {
                        obstacle: ObstacleKind::Post
                    }
                )
            });
        }
    }
    let j = j.expect("jam on the post");
    assert_eq!(
        phase_tick(&rig, 1, PropPhase::Released),
        Some(j),
        "released at once"
    );
    assert_eq!(phase_tick(&rig, 1, PropPhase::Landed), Some(j + 31));
    let exposed = pile_tick(&rig, |e| matches!(e, PileEvent::Exposed { .. })).expect("exposed");
    assert_eq!(exposed, j + 31, "the exposure is the landing's");
    // 30 - 6, never lethal; Exposed for 150 ticks, then Get-up.
    let hurt = rig.piles().into_iter().find_map(|(t, e)| match e {
        PileEvent::Hurt {
            hp_left, damage, ..
        } if t == j + 31 => Some((damage, hp_left)),
        _ => None,
    });
    assert_eq!(hurt, Some((6, 24)));
    let hero_hits: Vec<_> = rig
        .hurts()
        .into_iter()
        .filter(|(t, src, _, hero)| *t == j + 31 && *src == ENV_SOURCE_ID && *hero == 0)
        .collect();
    assert_eq!(
        hero_hits.len(),
        1,
        "Carl stood in the footprint: one landing blow"
    );
    assert!(rig.boss().boss().alive());
}

/// The landing misses her when she is outside the footprint: no effect on her (`hrd_jb_misses_her`).
#[test]
fn a_landing_that_misses_her_has_no_effect_on_her() {
    let mut rig = awake(790, 290.0, 1);
    rig.run(200, &|t| {
        if t.is_multiple_of(2) {
            carl_input(attack_input())
        } else {
            IDLE
        }
    });
    assert_eq!(mass(&rig), MassState::Landed);
    assert!(pile_tick(&rig, |e| matches!(e, PileEvent::Exposed { .. })).is_none());
    assert_eq!(rig.boss().boss().hp(), 30);
    assert_ne!(rig.boss().boss().mode(), PileMode::Exposed);
}

/// A restart mid-fall is a fresh attempt: the post stands again.
#[test]
fn a_restart_mid_fall_is_a_fresh_attempt() {
    let mut rig = awake(852, 700.0, 1);
    while mass(&rig) != MassState::Falling {
        rig.step(IDLE);
    }
    rig.combat.restart(&mut rig.scene);
    assert_eq!(mass(&rig), MassState::Supported);
    let env = rig.combat.env().unwrap();
    assert_eq!(env.live_targets().len(), 1, "the post is a target again");
}

// ------------------------------------------------------------------- pins --

/// The pristine piles leave the env hash and the state line as they were, so a room 1 run that never
/// reaches a pile keeps every other pin.
#[test]
fn an_untouched_pile_is_not_hashed_and_not_in_the_state_line() {
    let rig = dormant(100);
    assert!(rig.combat.env().unwrap().token().trim() == "env=");
    let mut words = Vec::new();
    rig.combat.env().unwrap().mix_into(&mut |v| words.push(v));
    // The marker, a count of 0 authored props, the (empty) hazard set; no pile words.
    assert!(!words.contains(&0x5049_4c45_0001));
}

/// Donut's swat and pounce are qualifying hits on the post (the sheet's HRD-Q26 rows): one hit per
/// move instance (a second tick of the same swat counts nothing), two instances release the stack.
/// Her melee is no stimulus of any other prop (BOILBC-AD7 stays), and a bot Donut's swat counts
/// nothing.
#[test]
fn donut_melee_counts_on_the_post_once_per_move_instance() {
    use crate::prototype::Rect;
    let mut rig = Rig::new(
        &root(),
        Plan {
            carl_x: 112,
            donut_x: 790,
            awake: Some((290.0, 1)),
            ..Plan::carl_at(60.0)
        }
        .with_control(crate::hoarder_fixtures::Control::SoloDonut),
    )
    .unwrap();
    let layout_h = rig.scene.world.layout_height();
    // The post [808, 824) x [0, 64) above the lane, in world px.
    let floor = (layout_h - 580) as f32;
    let swat = Rect {
        left: 806.0,
        bottom: floor + 4.0,
        width: 10.0,
        height: 20.0,
    };
    let tick = rig.next_tick();
    rig.combat
        .env_donut_melee_for_test(&rig.scene, swat, 1, tick);
    assert_eq!(progress(&rig), 1);
    rig.combat
        .env_donut_melee_for_test(&rig.scene, swat, 1, tick + 1);
    assert_eq!(progress(&rig), 1, "the same move instance counts once");
    // A box beside the post touches nothing.
    let miss = Rect {
        left: 700.0,
        ..swat
    };
    rig.combat
        .env_donut_melee_for_test(&rig.scene, miss, 2, tick + 2);
    assert_eq!(progress(&rig), 1);
    rig.combat
        .env_donut_melee_for_test(&rig.scene, swat, 2, tick + 3);
    assert_eq!(
        mass(&rig),
        MassState::Released,
        "a second instance releases it"
    );
}

/// Donut is blocked by bank A too (both heroes share the one solid).
#[test]
fn bank_a_blocks_donut() {
    let mut rig = Rig::new(
        &root(),
        Plan {
            carl_x: 112,
            donut_x: 300,
            ..Plan::carl_at(60.0)
        }
        .dormant()
        .with_control(crate::hoarder_fixtures::Control::SoloDonut),
    )
    .unwrap();
    rig.run(150, &|_| carl_input(run_input(1.0)));
    let x = rig.scene.world.body(HeroId::Donut).x;
    assert!((330.0..=342.5).contains(&x), "stopped at the face: {x}");
}

// ------------------------------------------------------- review additions --

/// `hrd_ja_nest_push` (K8, HRD-Q24): a percher on bank A's top is never in an untouchable nest. Carl
/// climbs onto the 48 px top; she comes to the face and the bank is disturbed by her shove (a
/// nest push or the ordinary shove, both jam on the face), it slides and settles, and no raised
/// surface above 16 px is left. The slide never costs HP.
#[test]
fn a_percher_on_bank_a_loses_the_vantage() {
    let mut rig = awake(300, 600.0, -1);
    rig.run(75, &|_| carl_input(run_input(1.0)));
    rig.step(carl_input(crate::input::RawInput {
        move_x: 1.0,
        ..jump_input()
    }));
    rig.run(14, &|_| carl_input(run_input(1.0)));
    let mut perched = false;
    for _ in 0..600 {
        rig.step(IDLE);
        perched |= rig.scene.world.hero.grounded && carl_foot(&rig) == 532;
        if slide(&rig) == SlideState::Settled {
            break;
        }
    }
    assert!(perched, "Carl stood on the 48 px top");
    let jam = pile_tick(&rig, |e| {
        matches!(
            e,
            PileEvent::Jammed {
                obstacle: ObstacleKind::BankA
            }
        )
    });
    assert!(jam.is_some(), "her shove jammed on the face");
    assert_eq!(slide(&rig), SlideState::Settled, "the vantage is gone");
    rig.run(60, &|_| IDLE);
    assert!(
        matches!(carl_foot(&rig), 564 | 580),
        "no 48 px top left: foot {}",
        carl_foot(&rig)
    );
    let slide_blows: Vec<_> = rig
        .hurts()
        .into_iter()
        .filter(|(_, src, _, hero)| *src == ENV_SOURCE_ID && *hero == 0)
        .collect();
    assert!(slide_blows.len() <= 1, "one push at most: {slide_blows:?}");
}

/// `hrd_ja_bot_target_credit`: her shove aimed at a bot-driven hero still jams on bank A and
/// disturbs it, credited to nobody.
#[test]
fn a_shove_at_a_bot_hero_disturbs_bank_a_with_no_credit() {
    let mut rig = Rig::new(
        &root(),
        Plan {
            carl_x: 330,
            donut_x: 250,
            awake: Some((470.0, -1)),
            ..Plan::carl_at(60.0)
        }
        .with_control(crate::hoarder_fixtures::Control::SoloDonut),
    )
    .unwrap();
    rig.run(200, &|_| IDLE);
    assert_ne!(slide(&rig), SlideState::Stable, "the bank was disturbed");
    let credit = rig.combat.events.iter().find_map(|e| match e {
        CombatEvent::PropHit {
            prop: 0, player, ..
        } => Some(*player),
        _ => None,
    });
    assert_eq!(credit, Some(None), "no player baited it");
}

/// `hrd_jb_bait_right`: her shove clamped on the post's right face leaves her pivot at 872 (body
/// 844..900, under the footprint); the landing on J+31 exposes her (6 damage, never lethal).
#[test]
fn a_baited_shove_into_the_posts_right_face_drops_the_stack_on_her() {
    let mut rig = awake(720, 940.0, -1);
    let mut j = None;
    for _ in 0..260 {
        rig.step(IDLE);
        if j.is_none() {
            j = pile_tick(&rig, |e| {
                matches!(
                    e,
                    PileEvent::Jammed {
                        obstacle: ObstacleKind::Post
                    }
                )
            });
            if j.is_some() {
                assert!(
                    (rig.boss().boss().x() - 872.0).abs() < 1.0,
                    "clamped on the right face: {}",
                    rig.boss().boss().x()
                );
            }
        }
    }
    let j = j.expect("jam on the post's right face");
    assert_eq!(phase_tick(&rig, 1, PropPhase::Landed), Some(j + 31));
    let exposed = pile_tick(&rig, |e| matches!(e, PileEvent::Exposed { .. }));
    assert_eq!(exposed, Some(j + 31));
    assert_eq!(rig.boss().boss().hp(), 24);
}
