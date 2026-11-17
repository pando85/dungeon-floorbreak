//! Animation of the Floor 1 scene actors (ANIM-INT-01): derives each actor's
//! [`AnimInput`] from the finished simulation state and steps its
//! [`Animator`]. Presentation only: this module reads a `&CorridorWorld` and
//! nothing flows back, so the simulation hash cannot depend on it.
//!
//! * Carl: `grounded`, `vx` and `vy` straight from the hero.
//! * Donut (passive trail follower): velocity from the difference between her
//!   consecutive trail positions, airborne when her foot is above the floor
//!   under her (the highest solid top below her foot at her x).
//!
//! After the exit is reached the simulation stops; both actors then see zero
//! horizontal velocity, so they settle into idle instead of running in place.

use crate::anim::{AnimInput, Animator};
use crate::corridor::CorridorWorld;
use crate::prototype::TICK_SECONDS;

/// Donut counts as airborne when her foot is more than this far above the
/// floor under her. Her trail position is interpolated, so allow float noise.
pub const DONUT_AIRBORNE_EPS_PX: f32 = 0.25;

#[derive(Debug, Clone)]
pub struct SceneAnimators {
    pub carl: Animator,
    pub donut: Animator,
    /// Donut's `(x, foot_y)` on the previous tick.
    donut_prev: (f32, f32),
}

impl SceneAnimators {
    /// Both animators idle, tick 0 (no step yet).
    pub fn new(carl: Animator, donut: Animator, world: &CorridorWorld) -> Self {
        Self {
            carl,
            donut,
            donut_prev: (world.donut.x, world.donut.foot_y),
        }
    }

    /// Back to tick 0 for a restarted world.
    pub fn reset(&mut self, world: &CorridorWorld) {
        self.carl.reset();
        self.donut.reset();
        self.donut_prev = (world.donut.x, world.donut.foot_y);
    }

    /// Advance both animators by one fixed tick of `world` (call after the
    /// simulation step of that tick).
    pub fn step(&mut self, world: &CorridorWorld) {
        let hero = &world.hero;
        self.carl.step(AnimInput {
            grounded: hero.grounded,
            vx: if world.complete { 0.0 } else { hero.vx },
            vy: hero.vy,
        });
        let donut = &world.donut;
        self.donut.step(AnimInput {
            grounded: donut_grounded(world),
            vx: if world.complete {
                0.0
            } else {
                (donut.x - self.donut_prev.0) / TICK_SECONDS
            },
            vy: (donut.foot_y - self.donut_prev.1) / TICK_SECONDS,
        });
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
            (Tag::JumpStart, 10),
            "jump_start plays on top of the takeoff, no delay"
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
        // Impact frame (80 ms = 4.8 ticks) + one recovery tick, then run.
        assert_eq!(first_cut - first_land, 6);
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
        let stop = RUN_JUMP_STOP_TICK as usize;
        assert_eq!(rows[stop + 1].4, Tag::Idle);
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
}
