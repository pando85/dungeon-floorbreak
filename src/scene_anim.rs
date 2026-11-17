//! Animation of the Floor 1 scene actors (ANIM-INT-01): derives each actor's
//! [`AnimInput`] from the finished simulation state and steps its
//! [`Animator`]. Presentation only: this module reads a `&CorridorWorld` and
//! nothing flows back, so the simulation hash cannot depend on it.
//!
//! * Carl: `grounded` and `vy` straight from the hero. His animation speed is
//!   his actual displacement per tick (times 60), not the commanded `vx`, so
//!   pushing into a wall does not show a run in place. While the stick is
//!   released the speed is zero at once.
//! * Donut (passive trail follower): velocity from the difference between her
//!   consecutive trail positions, airborne when her foot is above the floor
//!   under her (the highest solid top below her foot at her x).
//!
//! A stall shorter than [`STALL_HOLD_TICKS`] ticks of a running actor holds the
//! current run frame (no idle flash, no restart of the cycle); only the
//! [`STALL_HOLD_TICKS`]th consecutive still tick goes to idle.
//!
//! After the exit is reached the simulation stops; both actors then see zero
//! horizontal velocity, so they settle into idle instead of running in place.

use crate::anim::{AnimInput, Animator, MOVE_THRESHOLD_PX_S};
use crate::corridor::CorridorWorld;
use crate::prototype::TICK_SECONDS;

/// A running actor whose measured speed drops to zero keeps its run frame for
/// this many ticks minus one; the 6th consecutive still tick shows idle. Donut's
/// trail stalls for a few ticks when Carl jumps in place or presses into a
/// wall, and every flash of the taller idle pose restarted her run at frame 0.
pub const STALL_HOLD_TICKS: u32 = 6;

/// What the speed of one tick means for the animation.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Speed {
    /// Moving at this speed (px/s).
    Moving(f32),
    /// Still, but for fewer than [`STALL_HOLD_TICKS`] ticks: hold a run frame.
    Stalled,
    /// Still for good: idle.
    Still,
}

/// Counts consecutive still ticks.
#[derive(Debug, Clone, Copy, Default)]
struct StallFilter {
    still_ticks: u32,
}

impl StallFilter {
    fn apply(&mut self, vx: f32) -> Speed {
        if vx.abs() > MOVE_THRESHOLD_PX_S {
            self.still_ticks = 0;
            Speed::Moving(vx)
        } else {
            self.still_ticks = self.still_ticks.saturating_add(1);
            if self.still_ticks < STALL_HOLD_TICKS {
                Speed::Stalled
            } else {
                Speed::Still
            }
        }
    }

    /// Standing still on purpose (stick released, scene finished): no hold.
    fn release(&mut self) -> Speed {
        self.still_ticks = STALL_HOLD_TICKS;
        Speed::Still
    }
}

fn drive(animator: &mut Animator, speed: Speed, grounded: bool, vy: f32) {
    let vx = match speed {
        Speed::Moving(vx) => vx,
        Speed::Stalled => {
            if animator.hold_run(grounded) {
                return;
            }
            0.0
        }
        Speed::Still => 0.0,
    };
    animator.step(AnimInput { grounded, vx, vy });
}

/// Donut counts as airborne when her foot is more than this far above the
/// floor under her. Her trail position is interpolated, so allow float noise.
pub const DONUT_AIRBORNE_EPS_PX: f32 = 0.25;

#[derive(Debug, Clone)]
pub struct SceneAnimators {
    pub carl: Animator,
    pub donut: Animator,
    /// Donut's `(x, foot_y)` on the previous tick.
    donut_prev: (f32, f32),
    /// Carl's x on the previous tick.
    carl_prev_x: f32,
    carl_stall: StallFilter,
    donut_stall: StallFilter,
}

impl SceneAnimators {
    /// Both animators idle, tick 0 (no step yet).
    pub fn new(carl: Animator, donut: Animator, world: &CorridorWorld) -> Self {
        Self {
            carl,
            donut,
            donut_prev: (world.donut.x, world.donut.foot_y),
            carl_prev_x: world.hero.x,
            carl_stall: StallFilter::default(),
            donut_stall: StallFilter::default(),
        }
    }

    /// Back to tick 0 for a restarted world.
    pub fn reset(&mut self, world: &CorridorWorld) {
        self.carl.reset();
        self.donut.reset();
        self.donut_prev = (world.donut.x, world.donut.foot_y);
        self.carl_prev_x = world.hero.x;
        self.carl_stall = StallFilter::default();
        self.donut_stall = StallFilter::default();
    }

    /// Advance both animators by one fixed tick of `world` (call after the
    /// simulation step of that tick).
    pub fn step(&mut self, world: &CorridorWorld) {
        let hero = &world.hero;
        let carl_speed = if world.complete || hero.vx.abs() <= MOVE_THRESHOLD_PX_S {
            self.carl_stall.release()
        } else {
            self.carl_stall
                .apply((hero.x - self.carl_prev_x) / TICK_SECONDS)
        };
        drive(&mut self.carl, carl_speed, hero.grounded, hero.vy);
        self.carl_prev_x = hero.x;
        let donut = &world.donut;
        let donut_speed = if world.complete {
            self.donut_stall.release()
        } else {
            self.donut_stall
                .apply((donut.x - self.donut_prev.0) / TICK_SECONDS)
        };
        drive(
            &mut self.donut,
            donut_speed,
            donut_grounded(world),
            (donut.foot_y - self.donut_prev.1) / TICK_SECONDS,
        );
        self.donut_prev = (donut.x, donut.foot_y);
    }

    /// The line CI compares between the headless smoke and the render run.
    pub fn state_line(&self, tick: u64) -> String {
        format!(
            "FLOOR1_ANIM_STATE tick={tick} carl={} donut={}",
            self.carl.label(),
            self.donut.label()
        )
    }
}

/// Whether Donut's foot rests on the surface under her.
fn donut_grounded(world: &CorridorWorld) -> bool {
    let (x, foot_y) = (world.donut.x, world.donut.foot_y);
    world
        .solids
        .iter()
        .filter(|block| block.intact && block.rect.left <= x && x < block.rect.right())
        .map(|block| block.rect.top())
        .filter(|top| *top <= foot_y + DONUT_AIRBORNE_EPS_PX)
        .fold(None::<f32>, |best, top| {
            Some(best.map_or(top, |b| b.max(top)))
        })
        .is_some_and(|floor| foot_y - floor <= DONUT_AIRBORNE_EPS_PX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_sprite::{FrameSource, load_carl, load_donut};
    use crate::anim::Tag;
    use crate::corridor::{
        RUN_JUMP_RUNNING_JUMP_TICK, RUN_JUMP_STANDING_JUMP_TICK, RUN_JUMP_STOP_TICK,
        RUN_JUMP_TOTAL_TICKS, Scenario, SceneSim,
    };
    use crate::env_kit::EnvKit;
    use std::path::Path;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn kit() -> EnvKit {
        EnvKit::load(root(), "assets/levels/floor01/corridor_abc_sample.json").unwrap()
    }

    fn animators(world: &CorridorWorld) -> SceneAnimators {
        let carl = load_carl(root(), FrameSource::GridSource).unwrap();
        let donut = load_donut(root(), FrameSource::GridSource).unwrap();
        SceneAnimators::new(
            Animator::new(carl.clips, carl.pacing),
            Animator::new(donut.clips, donut.pacing),
            world,
        )
    }

    /// One row per tick: (hero grounded, hero vy, carl tag, carl atlas index,
    /// donut tag, donut atlas index, sim hash).
    type Row = (bool, f32, Tag, usize, Tag, usize, u64);

    fn run(scenario: Scenario, ticks: u64, animated: bool) -> Vec<Row> {
        let kit = kit();
        let mut sim = SceneSim::new(&kit, 360).unwrap();
        let mut anim = animators(&sim.world);
        let mut rows = Vec::new();
        sim.run_scenario_observed(scenario, ticks, |sim| {
            if animated {
                anim.step(&sim.world);
            }
            rows.push((
                sim.world.hero.grounded,
                sim.world.hero.vy,
                anim.carl.tag(),
                anim.carl.atlas_index(),
                anim.donut.tag(),
                anim.donut.atlas_index(),
                sim.state_hash(),
            ));
        });
        rows
    }

    /// Consecutive duplicates removed.
    fn tags(rows: &[Row], pick: impl Fn(&Row) -> Tag) -> Vec<Tag> {
        let mut out: Vec<Tag> = Vec::new();
        for row in rows {
            let tag = pick(row);
            if out.last() != Some(&tag) {
                out.push(tag);
            }
        }
        out
    }

    /// Whether `needle` occurs in `haystack` as an ordered subsequence.
    fn contains_in_order(haystack: &[Tag], needle: &[Tag]) -> bool {
        let mut rest = needle.iter();
        let mut want = rest.next();
        for tag in haystack {
            if Some(tag) == want {
                want = rest.next();
            }
        }
        want.is_none()
    }

    #[test]
    fn run_jump_plays_every_carl_state_in_order() {
        let rows = run(Scenario::RunJump, RUN_JUMP_TOTAL_TICKS, true);
        use Tag::*;
        let carl = tags(&rows, |r| r.2);
        // Stand, run, jump the low steps, run, a running jump with a run-cut
        // landing, stop, then a standing jump and a full landing.
        assert!(
            contains_in_order(
                &carl,
                &[
                    Idle, Run, JumpStart, JumpRise, JumpFall, Land, Run, JumpStart, JumpRise,
                    JumpFall, Land, Run, Idle, JumpStart, JumpRise, JumpFall, Land, Idle
                ]
            ),
            "{carl:?}"
        );
        assert_eq!(carl.last(), Some(&Idle));
    }

    #[test]
    fn carl_takes_off_on_the_impulse_tick_and_turns_at_the_apex() {
        let rows = run(Scenario::RunJump, RUN_JUMP_TOTAL_TICKS, true);
        // The jump input applies to step number tick + 1 (0-based row = tick).
        let takeoff = RUN_JUMP_RUNNING_JUMP_TICK as usize;
        assert!(rows[takeoff - 1].0, "grounded before");
        assert!(!rows[takeoff].0, "airborne on the impulse tick");
        assert_eq!(
            (rows[takeoff].2, rows[takeoff].3),
            (Tag::JumpStart, 11),
            "jump_start (toe-off frame) plays on top of the takeoff, no delay"
        );
        let apex = (takeoff..rows.len())
            .find(|&i| rows[i].1 <= 0.0)
            .expect("the jump has an apex");
        assert_eq!(rows[apex].2, Tag::JumpFall, "fall begins on the apex tick");
        assert_eq!(rows[apex - 1].2, Tag::JumpRise);
        assert!(rows[apex - 1].1 > 0.0);
        let touchdown = (apex..rows.len())
            .find(|&i| rows[i].0)
            .expect("the jump lands");
        assert_eq!((rows[touchdown].2, rows[touchdown].3), (Tag::Land, 14));
        assert_eq!(rows[touchdown - 1].2, Tag::JumpFall);
    }

    #[test]
    fn running_land_is_cut_into_run_but_the_standing_one_plays_out() {
        let rows = run(Scenario::RunJump, RUN_JUMP_TOTAL_TICKS, true);
        // The running jump on the open floor.
        let first_land = (RUN_JUMP_RUNNING_JUMP_TICK as usize..rows.len())
            .find(|&i| rows[i].2 == Tag::Land)
            .unwrap();
        let first_cut = (first_land..rows.len())
            .find(|&i| rows[i].2 == Tag::Run)
            .unwrap();
        // The impact frame (80 ms = 4.8 ticks) shows for 5 ticks and run follows
        // at once: the recovery frame never flashes.
        assert_eq!(first_cut - first_land, 5);
        assert!(
            rows[first_land..first_cut].iter().all(|r| r.3 == 14),
            "only the impact frame is shown before the cut"
        );
        let standing_land = (RUN_JUMP_STANDING_JUMP_TICK as usize..rows.len())
            .find(|&i| rows[i].2 == Tag::Land)
            .unwrap();
        let end = (standing_land..rows.len())
            .find(|&i| rows[i].2 != Tag::Land)
            .unwrap();
        // 80 ms + 100 ms = 10.8 ticks of land, then idle.
        assert_eq!(end - standing_land, 11);
        assert_eq!(rows[end].2, Tag::Idle);
    }

    #[test]
    fn donut_derives_her_own_states_from_her_trail() {
        let rows = run(Scenario::RunJump, RUN_JUMP_TOTAL_TICKS, true);
        let donut = tags(&rows, |r| r.4);
        for tag in [
            Tag::Run,
            Tag::JumpStart,
            Tag::JumpRise,
            Tag::JumpFall,
            Tag::Land,
        ] {
            assert!(donut.contains(&tag), "Donut never shows {tag:?}: {donut:?}");
        }
        // She starts running at the same tick as Carl: her trail speed is his
        // ground speed from the first step.
        let carl_run = rows.iter().position(|r| r.2 == Tag::Run).unwrap();
        let donut_run = rows.iter().position(|r| r.4 == Tag::Run).unwrap();
        assert_eq!(carl_run, donut_run);
        // She takes off where Carl did, so later than his takeoff tick in time
        // only by the trail length at that speed (32 px = 10.4 ticks), and stops
        // running when Carl does.
        let from = RUN_JUMP_RUNNING_JUMP_TICK as usize;
        let carl_jump = (from..rows.len())
            .find(|&i| rows[i].2 == Tag::JumpStart)
            .unwrap();
        let donut_jump = (from..rows.len())
            .find(|&i| rows[i].4 == Tag::JumpStart)
            .unwrap();
        assert!(
            (9..=13).contains(&(donut_jump - carl_jump)),
            "Donut takes off {} ticks after Carl",
            donut_jump - carl_jump
        );
        // Her stall hold makes her go idle on the 6th still tick, 5 ticks
        // (83 ms) after his intentional stop.
        let stop = RUN_JUMP_STOP_TICK as usize;
        assert_eq!(rows[stop + 4].4, Tag::Run);
        assert_eq!(rows[stop + 5].4, Tag::Idle);
    }

    #[test]
    fn animation_does_not_change_the_simulation() {
        for (scenario, ticks) in [
            (Scenario::RunJump, RUN_JUMP_TOTAL_TICKS),
            (Scenario::WalkRight, 700),
            (Scenario::TurnAround, 400),
            (Scenario::Idle, 100),
        ] {
            let with = run(scenario, ticks, true);
            let without = run(scenario, ticks, false);
            assert_eq!(with.len(), without.len());
            for (tick, (a, b)) in with.iter().zip(&without).enumerate() {
                assert_eq!(a.6, b.6, "{scenario:?}: sim hash differs at tick {tick}");
            }
        }
    }

    #[test]
    fn the_frame_sequence_is_deterministic() {
        let a = run(Scenario::WalkRight, 700, true);
        let b = run(Scenario::WalkRight, 700, true);
        assert_eq!(a, b);
        let frames = |rows: &[Row]| rows.iter().map(|r| (r.3, r.5)).collect::<Vec<_>>();
        assert_eq!(frames(&a), frames(&b));
    }

    #[test]
    fn the_reset_world_restarts_the_animation() {
        let kit = kit();
        let mut sim = SceneSim::new(&kit, 360).unwrap();
        let mut anim = animators(&sim.world);
        sim.run_scenario_observed(Scenario::RunJump, 185, |s| anim.step(&s.world));
        assert_eq!(
            anim.carl.tag(),
            Tag::JumpRise,
            "14 ticks into the running jump"
        );
        sim.reset();
        anim.reset(&sim.world);
        assert_eq!((anim.carl.tag(), anim.carl.atlas_index()), (Tag::Idle, 0));
        assert_eq!((anim.donut.tag(), anim.donut.atlas_index()), (Tag::Idle, 0));
    }

    #[test]
    fn walking_off_a_platform_goes_to_fall_not_jump_start() {
        // walk_right climbs onto low platforms and walks off them: a fall
        // without a jump_start must exist, and every jump_start is a rising one.
        let rows = run(Scenario::WalkRight, 613, true);
        for window in rows.windows(2) {
            if window[1].2 == Tag::JumpStart && window[0].2 != Tag::JumpStart {
                assert!(window[1].1 > 0.0, "jump_start with vy <= 0");
            }
        }
        assert!(rows.iter().any(|r| r.2 == Tag::JumpFall));
    }

    #[test]
    fn a_finished_scene_settles_into_idle() {
        let kit = kit();
        let mut sim = SceneSim::new(&kit, 360).unwrap();
        let mut anim = animators(&sim.world);
        sim.run_scenario_observed(Scenario::WalkRight, 700, |s| anim.step(&s.world));
        assert!(sim.world.complete);
        assert_eq!(anim.carl.tag(), Tag::Idle);
        assert_eq!(anim.donut.tag(), Tag::Idle);
    }

    #[test]
    fn state_line_is_stable_text() {
        let kit = kit();
        let sim = SceneSim::new(&kit, 360).unwrap();
        let anim = animators(&sim.world);
        assert_eq!(
            anim.state_line(0),
            "FLOOR1_ANIM_STATE tick=0 carl=idle:0 donut=idle:0"
        );
    }

    // ---- stalls (B2) and wall pushes (F2) ----

    fn donut_tag_at(rows: &[Row], tick: usize) -> Tag {
        rows[tick - 1].4
    }

    #[test]
    fn the_stall_filter_holds_for_five_ticks_and_idles_on_the_sixth() {
        let mut filter = StallFilter::default();
        assert_eq!(filter.apply(185.0), Speed::Moving(185.0));
        for _ in 0..5 {
            assert_eq!(filter.apply(0.0), Speed::Stalled);
        }
        assert_eq!(filter.apply(0.0), Speed::Still);
        assert_eq!(filter.apply(0.0), Speed::Still);
        // Moving again restarts the count.
        assert_eq!(filter.apply(-90.0), Speed::Moving(-90.0));
        assert_eq!(filter.apply(0.0), Speed::Stalled);
        assert_eq!(filter.release(), Speed::Still);
    }

    #[test]
    fn donut_keeps_running_through_one_tick_trail_stalls() {
        // run_jump tick 41: her trail stops for one tick while Carl jumps the
        // first step; she used to flash idle and restart the run at frame 0.
        let rows = run(Scenario::RunJump, RUN_JUMP_TOTAL_TICKS, true);
        for tick in 38..=45 {
            assert_eq!(donut_tag_at(&rows, tick), Tag::Run, "run_jump tick {tick}");
        }
        assert_eq!(
            rows[40].5, rows[39].5,
            "the run frame is held, not restarted"
        );
        // walk_right tick 11: the same one-tick stall at the first step.
        let rows = run(Scenario::WalkRight, 613, true);
        for tick in 8..=14 {
            assert_eq!(
                donut_tag_at(&rows, tick),
                Tag::Run,
                "walk_right tick {tick}"
            );
        }
    }

    #[test]
    fn donut_keeps_running_through_a_three_tick_stall_and_idles_after_six() {
        // walk_right ticks 468-470: a three-tick stall at the second step.
        let rows = run(Scenario::WalkRight, 613, true);
        for tick in 462..=478 {
            assert_eq!(
                donut_tag_at(&rows, tick),
                Tag::Run,
                "walk_right tick {tick}"
            );
        }
        let held = rows[466].5;
        assert!(
            (467..=470).all(|t| rows[t - 1].5 == held),
            "the frame does not advance while the trail is stalled"
        );
        // A long stall still ends in idle: Carl jumping in place at the first
        // wall (run_jump ticks 46-50 hold, tick 51 is the 6th still tick).
        let rows = run(Scenario::RunJump, RUN_JUMP_TOTAL_TICKS, true);
        assert_eq!(donut_tag_at(&rows, 50), Tag::Run);
        assert_eq!(donut_tag_at(&rows, 51), Tag::Idle);
    }

    #[test]
    fn carl_pushing_into_a_wall_settles_into_idle_after_the_stall_hold() {
        use crate::input::RawInput;
        let kit = kit();
        let mut sim = SceneSim::new(&kit, 360).unwrap();
        let mut anim = animators(&sim.world);
        let mut rows: Vec<(f32, Tag, u64)> = Vec::new();
        for _ in 0..90 {
            sim.step(RawInput {
                move_x: 1.0,
                ..RawInput::default()
            });
            anim.step(&sim.world);
            rows.push((sim.world.hero.x, anim.carl.tag(), sim.state_hash()));
        }
        // The first still tick is the first row whose x equals the previous one.
        let first_still = (1..rows.len())
            .find(|&i| rows[i].0 == rows[i - 1].0)
            .expect("Carl reaches the wall");
        for (i, row) in rows[..first_still + 5].iter().enumerate() {
            assert_eq!(row.1, Tag::Run, "row {i} (still from row {first_still})");
        }
        assert_eq!(
            rows[first_still + 5].1,
            Tag::Idle,
            "the 6th still tick is idle"
        );
        assert_eq!(rows.last().unwrap().1, Tag::Idle);
        // Animation does not touch the sim: the same input without animators
        // gives the same hashes.
        let mut bare = SceneSim::new(&kit, 360).unwrap();
        for row in &rows {
            bare.step(RawInput {
                move_x: 1.0,
                ..RawInput::default()
            });
            assert_eq!(bare.state_hash(), row.2);
        }
    }

    #[test]
    fn releasing_the_stick_stops_the_run_at_once() {
        // No stall hold for an intentional stop: idle on the first still tick.
        let rows = run(Scenario::RunJump, RUN_JUMP_TOTAL_TICKS, true);
        let stop = RUN_JUMP_STOP_TICK as usize;
        assert_eq!(rows[stop - 1].2, Tag::Run);
        assert_eq!(rows[stop].2, Tag::Idle);
    }

    #[test]
    fn a_coyote_jump_has_no_jump_start() {
        // Walk off a platform, then jump from the air: fall -> rise only. The
        // animator rule is tested in `anim`; here the real sim must agree that
        // walk-offs never show jump_start.
        let rows = run(Scenario::WalkRight, 613, true);
        for window in rows.windows(2) {
            if window[1].2 == Tag::JumpStart && window[0].2 != Tag::JumpStart {
                assert!(window[0].0, "jump_start only from the ground");
            }
        }
    }
}
