//! One lab session: the simulation plus the presentation camera operator.
//!
//! The session derives the camera input from the simulation state (actor
//! bodies, a readable boss tell, a hit-stop flag) and steps the operator after
//! the simulation tick. Information only flows simulation -> camera; the camera
//! never writes into the simulation, so the authoritative hash is the same at
//! every coverage and aspect (proved by a test).

use super::camera_op::{
    Aspect, Bounds, CameraInput, CameraMode, CameraOperator, Subject, Threat, View,
};
use super::room::Room;
use super::sim::{CARL, FoeMode, Hittable, LabInput, LabSim, Mode, foe_phase_progress};
use crate::combat::EnemyAi;
use crate::goblin::GoblinMode;

pub struct Session {
    pub sim: LabSim,
    pub camera: CameraOperator,
    pub aspect: Aspect,
}

/// Subjects and threat of the current simulation state, in layout (y down)
/// pixels.
pub fn camera_input_of(sim: &LabSim) -> (Vec<Subject>, Option<Threat>, bool) {
    let h = sim.height;
    let subjects: Vec<Subject> = sim
        .actors
        .iter()
        .map(|a| Subject {
            x: a.body.x,
            y: h - a.body.y,
            half_w: a.body.hw,
            half_h: a.body.hh,
        })
        .collect();
    // The imminent charge or shot with the least time left that shows a tell.
    let mut threat: Option<Threat> = None;
    for t in &sim.targets {
        if let Hittable::Foe(f) = t
            && f.alive()
            && matches!(
                f.spec.kind,
                crate::camera_lab::room::EnemyKind::Ball | crate::camera_lab::room::EnemyKind::Llama
            )
        {
            let (left, dir) = match (f.mode, foe_phase_progress(f)) {
                (FoeMode::Tell, Some((_, left))) => (left, f.facing),
                (FoeMode::Active, _) if f.spec.kind == crate::camera_lab::room::EnemyKind::Ball => (0, f.charge_dir),
                _ => continue,
            };
            let candidate = Threat {
                x: f.x,
                y: h - f.y,
                half_w: f.spec.hw,
                half_h: f.spec.hh,
                dir,
                tell_left: left,
                readable: true,
            };
            if threat.is_none_or(|t| candidate.tell_left < t.tell_left) {
                threat = Some(candidate);
            }
        }
    }
    let impact = sim.actors.iter().any(|a| a.hit_stop > 0 || a.runner.hit_stop_left() > 0)
        || sim.tick.saturating_sub(sim.last_hit_tick) < 10 && sim.last_hit_tick > 0
        || sim.targets.iter().any(|t| match t {
            Hittable::Goblin(g) => g.hit_stop > 0,
            Hittable::Foe(f) => f.hit_stop > 0,
            _ => false,
        });
    let _ = (EnemyAi::Chase, GoblinMode::Neutral);
    (subjects, threat, impact)
}

impl Session {
    pub fn new(room: &Room, mode: Mode, seed: u64, camera: CameraMode, aspect: Aspect) -> Self {
        let sim = LabSim::new(room, mode, seed);
        let (subjects, threat, impact_hold) = camera_input_of(&sim);
        let op = CameraOperator::new(
            camera,
            aspect,
            Bounds { w: room.render.0, h: room.render.1 },
            &CameraInput { subjects: &subjects, threat, impact_hold },
        );
        Self { sim, camera: op, aspect }
    }

    pub fn step(&mut self, inputs: [LabInput; 2]) -> View {
        self.sim.step(inputs);
        let (subjects, threat, impact_hold) = camera_input_of(&self.sim);
        self.camera.step(&CameraInput { subjects: &subjects, threat, impact_hold })
    }

    pub fn view(&self) -> View {
        self.camera.view()
    }

    /// Active focus actor (solo) or Carl.
    pub fn focus(&self) -> usize {
        if self.sim.mode == Mode::Solo { self.sim.active } else { CARL }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_lab::camera_op::{COVERAGE_CANDIDATES, safety_inside_in, visible_share};
    use crate::camera_lab::room::load_rooms;
    use crate::camera_lab::script::{scripts, solo_script, script_inputs, run};
    use std::path::Path;

    fn rooms() -> Vec<Room> {
        load_rooms(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    fn drive(room: &Room, script: &crate::camera_lab::script::Script, cam: CameraMode, aspect: Aspect) -> (u64, Vec<View>, Vec<String>) {
        let mut s = Session::new(room, script.mode, script.seed, cam, aspect);
        let mut views = Vec::new();
        for _ in 0..script.ticks {
            let inputs = script_inputs(&s.sim, script);
            views.push(s.step(inputs));
        }
        (s.sim.state_hash(), views, s.sim.events.iter().map(|e| e.to_string()).collect())
    }

    /// The headline determinism check: the same scripted run at 1.0, 1.25,
    /// 1.5 and 1.75 (and the automatic camera, 16:9 and 16:10) gives the same
    /// authoritative final hash and the same event log, while the views differ.
    #[test]
    fn authoritative_hash_is_identical_at_every_coverage() {
        let rooms = rooms();
        for script in scripts() {
            let room = &rooms[script.room];
            let baseline = run(room, &script);
            let mut view_sets = Vec::new();
            let modes: Vec<(CameraMode, Aspect)> = COVERAGE_CANDIDATES
                .iter()
                .map(|c| (CameraMode::Fixed(*c), Aspect::Wide16x9))
                .chain([(CameraMode::Auto, Aspect::Wide16x9), (CameraMode::Auto, Aspect::Deck16x10), (CameraMode::Fixed(1.5), Aspect::Deck16x10)])
                .collect();
            for (cam, aspect) in modes {
                let (hash, views, events) = drive(room, &script, cam, aspect);
                assert_eq!(hash, baseline.final_hash, "{} {cam:?} {aspect:?}", script.name);
                assert_eq!(events, baseline.events, "{} {cam:?}", script.name);
                view_sets.push(views);
            }
            // The camera really did something different at each coverage.
            assert_ne!(view_sets[0].last(), view_sets[3].last(), "{}", script.name);
            assert!(view_sets[0].iter().all(|v| v.w == 640));
            assert!(view_sets[3].iter().all(|v| v.w == 1120));
        }
    }

    #[test]
    fn scripted_runs_are_reproducible_and_make_progress() {
        let rooms = rooms();
        for script in scripts() {
            let room = &rooms[script.room];
            let a = run(room, &script);
            let b = run(room, &script);
            assert_eq!(a.final_hash, b.final_hash, "{}", script.name);
            assert_eq!(a.tick_hashes, b.tick_hashes);
            // Something happened: hits and kills or breakage.
            assert!(
                a.events.iter().any(|e| e.contains("kind=hit")),
                "{}: no hit in {} events",
                script.name,
                a.events.len()
            );
        }
    }

    #[test]
    fn different_seeds_diverge_only_where_the_rng_is_used() {
        let rooms = rooms();
        let script = scripts().remove(1); // llamas: seeded spit jitter
        let a = run(&rooms[script.room], &script);
        let mut other = script.clone();
        other.seed = 99;
        let b = run(&rooms[script.room], &other);
        assert_ne!(a.final_hash, b.final_hash);
    }

    #[test]
    fn solo_runs_switch_once_and_the_companion_acts() {
        let rooms = rooms();
        for i in [0usize, 3, 5] {
            let script = solo_script(i);
            let r = run(&rooms[i], &script);
            assert!(r.events.iter().any(|e| e.contains("kind=switched")), "{}", rooms[i].id);
            let again = run(&rooms[i], &script);
            assert_eq!(r.final_hash, again.final_hash);
        }
    }

    #[test]
    fn auto_camera_keeps_both_actors_in_safety_in_every_scripted_run() {
        let rooms = rooms();
        for script in scripts() {
            let room = &rooms[script.room];
            let mut s = Session::new(room, script.mode, script.seed, CameraMode::Auto, Aspect::Wide16x9);
            let mut out_of_safety = 0u64;
            let mut worst_share = 1.0f32;
            for tick in 0..script.ticks {
                let inputs = script_inputs(&s.sim, &script);
                let v = s.step(inputs);
                let (subs, _, _) = camera_input_of(&s.sim);
                if tick > 90 {
                    for sub in &subs {
                        if !safety_inside_in(v, sub, Bounds { w: room.render.0, h: room.render.1 }) {
                            out_of_safety += 1;
                        }
                        worst_share = worst_share.min(visible_share(v, sub));
                    }
                }
                assert!(v.left >= 0 && v.right() <= room.render.0 && v.top >= 0 && v.bottom() <= room.render.1);
            }
            // Brief catch-up after a jump or a knockback is fine; a persistent
            // violation is not.
            assert!(out_of_safety < script.ticks / 4, "{}: {out_of_safety} ticks out of safety", script.name);
            assert!(worst_share > 0.5, "{}: an actor was {worst_share} visible", script.name);
        }
    }
}
