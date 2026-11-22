//! DONUT-MOBILITY-01 DM-C: her animation at the agile numbers (spec 3.4, "run
//! cycle and jump phases"). The run cycle must be slip-free at 207 px/s, and the
//! jump phases (keyed on `vy`) must sequence at a 500 px/s launch and through a
//! DM-Q27 ceiling bonk. Checked against the real clip data and the real motion.

use crate::actor_sprite::{FrameSource, load_donut};
use crate::agility;
use crate::agility_fixtures::{DM_JUMP_LEDGE_96, run};
use crate::anim::{
    AnimInput, Animator, DONUT_RUN_GROUND_TENTHS_PX, DONUT_RUN_PACING, Tag, speed_milli,
};
use crate::cf02::arena;
use crate::env_kit::to_world_y;
use crate::input::{InputFrame, RawInput};
use crate::prototype::{BodyDrive, HERO_HALF_HEIGHT, Hero, HeroMotion, step_hero_motion};

fn animator() -> Animator {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let donut = load_donut(&root, FrameSource::GridSource).unwrap();
    Animator::new(donut.clips, DONUT_RUN_PACING)
}

/// Spec 3.4: `frame_ms` scales with speed and her run stays slip-free at 207
/// px/s: every run frame's duration covers its authored ground distance to
/// within a fraction of a pixel, and none sits on the floor or ceiling clamp.
#[test]
fn her_run_cycle_is_slip_free_at_207_px_per_second() {
    let speed = agility::agile_motion().run_speed;
    assert_eq!(speed, 207.0, "DM-Q01");
    let ground_px = DONUT_RUN_GROUND_TENTHS_PX as f32 / 10.0;
    let weights = crate::anim::DONUT_RUN_WEIGHTS;
    let mut total_ms = 0;
    for (i, (num, den)) in weights.iter().enumerate() {
        let ms = DONUT_RUN_PACING.frame_ms(i, speed_milli(speed));
        assert!(
            (50..130).contains(&ms),
            "frame {i}: {ms} ms is inside the clamp (50..130), so the pacing is not capped"
        );
        let covered = ms as f32 * speed / 1000.0;
        let authored = ground_px * *num as f32 / *den as f32;
        assert!(
            (covered - authored).abs() <= 0.2,
            "frame {i}: covers {covered:.2} px, authored {authored:.2} px"
        );
        total_ms += ms;
    }
    // The 6 frame cycle covers its authored 14.8 px x 6.125 weight units.
    let cycle_px = total_ms as f32 * speed / 1000.0;
    let authored_cycle: f32 = weights
        .iter()
        .map(|(n, d)| ground_px * *n as f32 / *d as f32)
        .sum();
    assert!(
        (cycle_px - authored_cycle).abs() <= 1.0,
        "cycle {cycle_px:.2} px against {authored_cycle:.2} px"
    );
    // Faster than Carl's 185: her frames are shorter, never longer.
    for i in 0..weights.len() {
        assert!(
            DONUT_RUN_PACING.frame_ms(i, speed_milli(207.0))
                <= DONUT_RUN_PACING.frame_ms(i, speed_milli(185.0))
        );
    }
}

fn input_of(state: (f32, f32, f32, bool)) -> AnimInput {
    AnimInput {
        grounded: state.3,
        vx: 0.0,
        vy: state.2,
    }
}

/// The tag sequence of a jump, runs of equal tags collapsed.
fn phases(states: &[(f32, f32, f32, bool)]) -> Vec<Tag> {
    let mut a = animator();
    let mut seq: Vec<Tag> = Vec::new();
    for s in states {
        a.step(input_of(*s));
        if seq.last() != Some(&a.tag()) {
            seq.push(a.tag());
        }
    }
    seq
}

/// Spec 3.4: the jump phases key on `vy`, so the 500 px/s launch sequences
/// `jump_start`, `jump_rise` (held while she rises), `jump_fall` at the apex and
/// `land` on touchdown, once each, in order, with no flicker.
#[test]
fn the_jump_phases_sequence_at_a_500_launch() {
    let trail = run(agility::agile_motion(), &DM_JUMP_LEDGE_96);
    // The first jump of the fixture: from the first airborne tick to touchdown.
    let first = trail.states.iter().position(|s| !s.3).expect("a jump");
    // The launch is DM-Q02's 500 px/s: the first airborne tick has taken one
    // gravity step from it.
    assert!(
        (480.0..=500.0).contains(&trail.states[first].2),
        "{:?}",
        trail.states[first]
    );
    let end = trail.states[first..]
        .iter()
        .position(|s| s.3)
        .map(|i| first + i + 1)
        .expect("she lands");
    let mut slice = vec![(0.0, 0.0, 0.0, true)];
    slice.extend_from_slice(&trail.states[first..end]);
    slice.extend([(0.0, 0.0, 0.0, true); 20]);
    let seq = phases(&slice);
    assert_eq!(
        seq,
        [
            Tag::Idle,
            Tag::JumpStart,
            Tag::JumpRise,
            Tag::JumpFall,
            Tag::Land,
            Tag::Idle
        ],
        "{seq:?}"
    );
}

/// DM-Q27: a perch jump under the lane ceiling bonks at feet 176 (`vy` is zeroed
/// while she is still rising). The phases still sequence: the rise hands over to
/// the fall on the bonk and the landing follows; no phase repeats or is skipped.
#[test]
fn the_jump_phases_sequence_through_a_ceiling_bonk() {
    let kit = arena::load_arena(&crate::agility_geometry_tests::root()).unwrap();
    let walk = to_world_y(kit.height, kit.base_walk_line_y);
    let mut blocks = kit.blocks();
    for b in &mut blocks {
        b.rect.bottom -= walk;
    }
    let mut hero = Hero::at(600.0, 80.0 + HERO_HALF_HEIGHT);
    let mut frame = InputFrame::default();
    let mut states = vec![(hero.x, hero.y - HERO_HALF_HEIGHT, hero.vy, hero.grounded)];
    for tick in 0..120 {
        frame.advance(RawInput {
            jump: tick == 0,
            ..RawInput::NONE
        });
        step_hero_motion(
            &mut hero,
            agility::agile_motion(),
            &frame,
            &blocks,
            1152.0,
            &BodyDrive::default(),
        );
        states.push((hero.x, hero.y - HERO_HALF_HEIGHT, hero.vy, hero.grounded));
    }
    let bonk = states
        .iter()
        .position(|s| (s.1 - 176.0).abs() < 0.01 && s.2 <= 0.0)
        .expect("she bonks at feet 176");
    assert!(states[bonk - 1].2 > 0.0, "she was still rising before it");
    let seq = phases(&states);
    assert_eq!(
        seq,
        [
            Tag::Idle,
            Tag::JumpStart,
            Tag::JumpRise,
            Tag::JumpFall,
            Tag::Land,
            Tag::Idle
        ],
        "{seq:?}"
    );
    let _ = HeroMotion::CARL;
}

/// DM-C evidence GIF 4: the committed perch fixture does what its summary says,
/// in the real scene: she lands on the 80 px perch, then bonks under the lane
/// ceiling at feet 176 (never higher), and lands on the perch again.
#[test]
fn the_perch_fixture_bonks_her_under_the_ceiling_at_176() {
    let kit = arena::load_arena(&crate::agility_geometry_tests::root()).unwrap();
    let fixture = &crate::agility_fixtures::CF02_ARENA_DM_PERCH_JUMP;
    let mut sim = crate::party_scene::PartyFixtureSim::new(&kit, 360, fixture).unwrap();
    let mut feet = Vec::new();
    for _ in 0..fixture.base.ticks {
        sim.step_scripted();
        feet.push((
            sim.scene.world.donut.foot_y,
            sim.scene.world.donut_body.grounded,
        ));
    }
    let floor = feet[0].0;
    let rel: Vec<f32> = feet.iter().map(|f| f.0 - floor).collect();
    let apex_first = rel[..140].iter().copied().fold(0.0, f32::max);
    let apex_second = rel[140..].iter().copied().fold(0.0, f32::max);
    assert!(
        apex_first > 100.0,
        "the running jump rises above the perch: {apex_first}"
    );
    assert!(
        (apex_second - 176.0).abs() < 1.0,
        "the bonk is at feet 176: {apex_second}"
    );
    // Drawn top is under the HUD band (DM-Q26/Q27): feet 176 + 44.
    assert!(apex_second + 44.0 <= 220.5);
    assert!(feet.last().unwrap().1, "she ends on the perch");
    assert!(
        (rel[250] - 80.0).abs() < 0.5,
        "the perch top is 80 up: {}",
        rel[250]
    );
}

/// DONUT-PROG-01 DP-D (the jump-phase check of the work order): with
/// `jump_reach_boost` held the launch is `dpq14_boost_launch_px_s` (547 px/s). The
/// phases key on the sign of `vy`, so they sequence exactly as at 500: once each,
/// in order, no flicker, and the rise lasts longer (a higher arc), never shorter.
#[test]
fn the_jump_phases_sequence_at_the_boosted_launch() {
    let boosted = HeroMotion {
        jump_speed: f32::from(crate::donut_prog::DonutProgTuning::active().dpq14_boost_launch_px_s),
        ..agility::agile_motion()
    };
    let rise_ticks = |motion| {
        let trail = run(motion, &DM_JUMP_LEDGE_96);
        let first = trail.states.iter().position(|s| !s.3).expect("a jump");
        let end = trail.states[first..]
            .iter()
            .position(|s| s.3)
            .map(|i| first + i + 1)
            .expect("she lands");
        let rising = trail.states[first..end]
            .iter()
            .filter(|s| s.2 > 0.0)
            .count();
        let mut slice = vec![(0.0, 0.0, 0.0, true)];
        slice.extend_from_slice(&trail.states[first..end]);
        slice.extend([(0.0, 0.0, 0.0, true); 20]);
        (trail.states[first].2, rising, phases(&slice))
    };
    let (launch, rising, seq) = rise_ticks(boosted);
    // The first airborne tick has taken one gravity step from 547.
    assert!((520.0..=547.0).contains(&launch), "launch {launch}");
    assert_eq!(
        seq,
        [
            Tag::Idle,
            Tag::JumpStart,
            Tag::JumpRise,
            Tag::JumpFall,
            Tag::Land,
            Tag::Idle
        ],
        "{seq:?}"
    );
    let (_, base_rising, base_seq) = rise_ticks(agility::agile_motion());
    assert_eq!(seq, base_seq, "the same phases as the unboosted jump");
    assert!(
        rising > base_rising,
        "the boosted rise ({rising} ticks) outlasts the 500 rise ({base_rising})"
    );
}
