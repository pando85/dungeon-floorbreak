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
use super::room::{EnemyKind, Room};
use super::sim::{CARL, FoeMode, Hittable, LabInput, LabSim, Mode, foe_phase_progress};

pub struct Session {
    pub sim: LabSim,
    pub camera: CameraOperator,
    pub aspect: Aspect,
    pub out_w: u32,
}

/// Sight of a ranged foe that has acquired an actor (px), the margin around an
/// actor in which a large foe counts as "next to" it (in body widths) and the
/// most threats handed to the camera.
const ACQUIRE_RANGE: f32 = 520.0;
const ADJACENT_BODIES: f32 = 1.5;
const MAX_THREATS: usize = 4;

/// Subjects, threats and the impact flag of the current simulation state, in
/// layout (y down) pixels.
///
/// Threats, highest priority first: a telegraphing or firing ranged foe, a
/// boss in its tell or charge and a spit in flight (3); a ranged foe that has
/// acquired an actor with a clear line (2, the camera starts widening when it
/// takes aim, before the tell); a large foe next to an actor (1).
pub fn camera_input_of(sim: &LabSim) -> (Vec<Subject>, Vec<Threat>, bool) {
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
    let mut threats: Vec<Threat> = Vec::new();
    for t in &sim.targets {
        let Hittable::Foe(f) = t else { continue };
        if !f.alive() || f.mode == FoeMode::Corpse || f.mode == FoeMode::Down {
            continue;
        }
        let base = Threat {
            x: f.x,
            y: h - f.y,
            half_w: f.spec.hw,
            half_h: f.spec.hh,
            dir: f.facing,
            tell_left: 0,
            readable: true,
            priority: 0,
            lane: 0.0,
        };
        let near = sim
            .actors
            .iter()
            .filter(|a| !a.down())
            .min_by(|a, b| (a.body.x - f.x).abs().total_cmp(&(b.body.x - f.x).abs()));
        let ball = f.spec.kind == EnemyKind::Ball;
        match (f.mode, foe_phase_progress(f)) {
            (FoeMode::Tell, Some((_, left))) if f.spec.ranged || ball => {
                threats.push(Threat {
                    tell_left: left,
                    priority: 3,
                    lane: if ball { 96.0 } else { 0.0 },
                    ..base
                });
                continue;
            }
            (FoeMode::Active, _) if ball => {
                threats.push(Threat {
                    dir: f.charge_dir,
                    priority: 3,
                    lane: 96.0,
                    ..base
                });
                continue;
            }
            (FoeMode::Active, _) | (FoeMode::Recover, _) if f.spec.ranged && f.mode_tick < 14 => {
                // The spit leaves on the first active tick; keep the shooter in
                // frame through the shot and the start of its recovery.
                threats.push(Threat {
                    priority: 3,
                    ..base
                });
                continue;
            }
            _ => {}
        }
        let Some(n) = near else { continue };
        let dx = (n.body.x - f.x).abs();
        if f.spec.ranged && dx <= ACQUIRE_RANGE && sim.line_clear((f.x, f.y), (n.body.x, n.body.y))
        {
            threats.push(Threat {
                priority: 2,
                ..base
            });
        } else if f.spec.hw >= 24.0 && dx <= ADJACENT_BODIES * f.spec.hw * 2.0 + n.body.hw {
            threats.push(Threat {
                priority: 1,
                ..base
            });
        }
    }
    for p in &sim.projectiles {
        threats.push(Threat {
            x: p.x,
            y: h - p.y,
            half_w: 6.0,
            half_h: 6.0,
            dir: if p.vx >= 0.0 { 1 } else { -1 },
            tell_left: 0,
            readable: true,
            priority: 3,
            lane: 0.0,
        });
    }
    threats.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.x.total_cmp(&b.x)));
    threats.truncate(MAX_THREATS);
    let impact = sim
        .actors
        .iter()
        .any(|a| a.hit_stop > 0 || a.runner.hit_stop_left() > 0)
        || sim.tick.saturating_sub(sim.last_hit_tick) < 10 && sim.last_hit_tick > 0
        || sim.targets.iter().any(|t| match t {
            Hittable::Goblin(g) => g.hit_stop > 0,
            Hittable::Foe(f) => f.hit_stop > 0,
            _ => false,
        });
    (subjects, threats, impact)
}

/// One row of the presentation trace: what the camera did on one tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceRow {
    pub tick: u64,
    pub view: View,
    /// View centre (world px).
    pub cx: f32,
    pub cy: f32,
    /// Where the lowest actor's feet sit in the view (0 top .. 1 bottom).
    pub floor_anchor: f32,
    pub urgent: bool,
    pub rest_target: i32,
    pub impact: bool,
}

impl TraceRow {
    pub fn line(&self) -> String {
        format!(
            "LAB_VIEW tick={} w={} h={} left={} top={} cx={:.1} cy={:.1} floor_anchor={:.4} urgent={} rest={} hit_stop={}",
            self.tick,
            self.view.w,
            self.view.h,
            self.view.left,
            self.view.top,
            self.cx,
            self.cy,
            self.floor_anchor,
            u8::from(self.urgent),
            self.rest_target,
            u8::from(self.impact)
        )
    }
}

impl Session {
    /// The presentation trace row of the current tick.
    pub fn trace_row(&self) -> TraceRow {
        let (subjects, _, impact) = camera_input_of(&self.sim);
        let v = self.camera.view();
        let lowest = subjects
            .iter()
            .map(|s| s.y + s.half_h)
            .fold(f32::MIN, f32::max);
        TraceRow {
            tick: self.sim.tick,
            view: v,
            cx: v.left as f32 + v.w as f32 / 2.0,
            cy: v.top as f32 + v.h as f32 / 2.0,
            floor_anchor: (lowest - v.top as f32) / v.h as f32,
            urgent: self.camera.urgent(),
            rest_target: self.camera.rest_target(),
            impact,
        }
    }

    /// A session for a 1920 px wide output.
    pub fn new(room: &Room, mode: Mode, seed: u64, camera: CameraMode, aspect: Aspect) -> Self {
        Self::new_for_output(room, mode, seed, camera, aspect, 1920)
    }

    /// A session for an output `out_w` pixels wide: the output decides only the
    /// automatic camera's rest widths (presentation). The simulation, including
    /// the 2P leash, does not depend on it.
    pub fn new_for_output(
        room: &Room,
        mode: Mode,
        seed: u64,
        camera: CameraMode,
        aspect: Aspect,
        out_w: u32,
    ) -> Self {
        let sim = LabSim::new(room, mode, seed);
        let (subjects, threats, impact_hold) = camera_input_of(&sim);
        let op = CameraOperator::new_for_output(
            camera,
            aspect,
            Bounds {
                w: room.render.0,
                h: room.render.1,
            },
            out_w,
            &CameraInput {
                subjects: &subjects,
                threats: &threats,
                impact_hold,
            },
        );
        Self {
            sim,
            camera: op,
            aspect,
            out_w,
        }
    }

    /// Select the 2P leash explicitly (a scenario or config choice, never the
    /// output): [`super::sim::WIDE_LEASH_PX`] is the documented alternative.
    pub fn with_leash(mut self, px: f32) -> Self {
        self.sim.leash = px;
        self
    }

    /// Rebuild the operator on the current simulation state (no easing): used
    /// after a test setup moved the actors.
    pub fn reframe(&mut self) {
        let (subjects, threats, impact_hold) = camera_input_of(&self.sim);
        self.camera = CameraOperator::new_for_output(
            self.camera.mode(),
            self.aspect,
            Bounds {
                w: self.sim.room.render.0,
                h: self.sim.room.render.1,
            },
            self.out_w,
            &CameraInput {
                subjects: &subjects,
                threats: &threats,
                impact_hold,
            },
        );
    }

    /// Switch the camera mode (fixed or automatic) at the current framing.
    pub fn set_camera_mode(&mut self, mode: CameraMode) {
        let (subjects, threats, impact_hold) = camera_input_of(&self.sim);
        self.camera = CameraOperator::new_for_output(
            mode,
            self.aspect,
            Bounds {
                w: self.sim.room.render.0,
                h: self.sim.room.render.1,
            },
            self.out_w,
            &CameraInput {
                subjects: &subjects,
                threats: &threats,
                impact_hold,
            },
        );
    }

    pub fn step(&mut self, inputs: [LabInput; 2]) -> View {
        self.sim.step(inputs);
        let (subjects, threats, impact_hold) = camera_input_of(&self.sim);
        self.camera.step(&CameraInput {
            subjects: &subjects,
            threats: &threats,
            impact_hold,
        })
    }

    pub fn view(&self) -> View {
        self.camera.view()
    }

    /// Active focus actor (solo) or Carl.
    pub fn focus(&self) -> usize {
        if self.sim.mode == Mode::Solo {
            self.sim.active
        } else {
            CARL
        }
    }
}

/// What the camera showed over one scripted run (headless, deterministic).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Metrics {
    pub ticks: u64,
    /// Ticks where both actors were at least 95 percent inside the view.
    pub both_visible: u64,
    /// Ticks where at least one actor was less than half inside the view.
    pub actor_lost: u64,
    /// Attack ticks (a foe in its tell or active frames within 700 px of an
    /// actor) and how many of them had the foe fully on screen.
    pub telegraph_ticks: u64,
    pub telegraph_visible: u64,
    /// Confirmed hits and how many had attacker and target both on screen.
    pub hits: u64,
    pub hits_visible: u64,
    pub zoom_changes: u32,
    pub mean_coverage: f32,
    pub max_pan_px: i32,
    pub max_actor_gap_px: f32,
    /// Ticks per view width (the histogram of the run).
    pub width_ticks: std::collections::BTreeMap<i32, u64>,
    /// Longest run of consecutive ticks outside the rest widths (a transit).
    pub longest_transit: u32,
    /// Largest view width seen.
    pub max_width: i32,
}

pub fn measure(
    room: &Room,
    script: &super::script::Script,
    camera: CameraMode,
    aspect: Aspect,
) -> Metrics {
    let out_w = if aspect == Aspect::Wide16x9 {
        1920
    } else {
        1280
    };
    measure_for_output(room, script, camera, aspect, out_w)
}

/// [`measure`] for an output `out_w` pixels wide.
pub fn measure_for_output(
    room: &Room,
    script: &super::script::Script,
    camera: CameraMode,
    aspect: Aspect,
    out_w: u32,
) -> Metrics {
    measure_with_leash(
        room,
        script,
        camera,
        aspect,
        out_w,
        super::sim::DEFAULT_LEASH_PX,
    )
}

/// [`measure_for_output`] with an explicit 2P leash.
pub fn measure_with_leash(
    room: &Room,
    script: &super::script::Script,
    camera: CameraMode,
    aspect: Aspect,
    out_w: u32,
    leash: f32,
) -> Metrics {
    use super::camera_op::{rest_widths, visible_share};
    use crate::combat::CombatEvent;
    let rest = rest_widths(out_w);
    let mut transit = 0u32;
    let mut session =
        Session::new_for_output(room, script.mode, script.seed, camera, aspect, out_w)
            .with_leash(leash);
    let mut m = Metrics::default();
    let mut prev = session.view();
    let mut cov_sum = 0.0f32;
    let mut seen_events = 0usize;
    for _ in 0..script.ticks {
        let inputs = super::script::script_inputs(&session.sim, script);
        let view = session.step(inputs);
        m.ticks += 1;
        *m.width_ticks.entry(view.w).or_insert(0) += 1;
        m.max_width = m.max_width.max(view.w);
        if rest.contains(&view.w) {
            transit = 0;
        } else {
            transit += 1;
            m.longest_transit = m.longest_transit.max(transit);
        }
        cov_sum += session.camera.coverage();
        m.max_pan_px = m.max_pan_px.max(
            (view.left - prev.left)
                .abs()
                .max((view.top - prev.top).abs()),
        );
        prev = view;
        let (subjects, _, _) = camera_input_of(&session.sim);
        let shares: Vec<f32> = subjects.iter().map(|s| visible_share(view, s)).collect();
        if shares.iter().all(|s| *s >= 0.95) {
            m.both_visible += 1;
        }
        if shares.iter().any(|s| *s < 0.5) {
            m.actor_lost += 1;
        }
        m.max_actor_gap_px = m
            .max_actor_gap_px
            .max((subjects[0].x - subjects[1].x).abs());
        let h = session.sim.height;
        for t in &session.sim.targets {
            if let Hittable::Foe(f) = t
                && matches!(f.mode, FoeMode::Tell | FoeMode::Active)
                && f.alive()
                && subjects.iter().any(|s| (s.x - f.x).abs() < 700.0)
            {
                m.telegraph_ticks += 1;
                let body = Subject {
                    x: f.x,
                    y: h - f.y,
                    half_w: f.spec.hw,
                    half_h: f.spec.hh,
                };
                if visible_share(view, &body) >= 0.95 {
                    m.telegraph_visible += 1;
                }
            }
            if let Hittable::Goblin(g) = t
                && g.mode == crate::goblin::GoblinMode::Attack
                && g.attack_tick < crate::feel::IMPACT_END
                && subjects.iter().any(|s| (s.x - g.x).abs() < 700.0)
            {
                m.telegraph_ticks += 1;
                let body = Subject {
                    x: g.x,
                    y: h - g.y,
                    half_w: 12.0,
                    half_h: 19.0,
                };
                if visible_share(view, &body) >= 0.95 {
                    m.telegraph_visible += 1;
                }
            }
        }
        let events = session.sim.events.len();
        for e in &session.sim.events[seen_events..events] {
            if let super::sim::LabEvent::Combat(CombatEvent::Hit { target, .. }) = e {
                m.hits += 1;
                let (tx, ty) = {
                    use crate::combat::StrikeTarget;
                    session.sim.targets[*target as usize].centre()
                };
                let target_in = view.contains_point(tx, h - ty);
                if target_in && shares.iter().any(|s| *s >= 0.95) {
                    m.hits_visible += 1;
                }
            }
        }
        seen_events = events;
    }
    m.zoom_changes = session.camera.zoom_changes;
    m.mean_coverage = cov_sum / m.ticks.max(1) as f32;
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_lab::camera_op::{COVERAGE_CANDIDATES, safety_inside_in, visible_share};
    use crate::camera_lab::room::load_rooms;
    use crate::camera_lab::script::{run, script_inputs, scripts, solo_script};
    use std::path::Path;

    fn rooms() -> Vec<Room> {
        load_rooms(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    fn drive(
        room: &Room,
        script: &crate::camera_lab::script::Script,
        cam: CameraMode,
        aspect: Aspect,
    ) -> (u64, Vec<View>, Vec<String>) {
        let mut s = Session::new(room, script.mode, script.seed, cam, aspect);
        let mut views = Vec::new();
        for _ in 0..script.ticks {
            let inputs = script_inputs(&s.sim, script);
            views.push(s.step(inputs));
        }
        (
            s.sim.state_hash(),
            views,
            s.sim.events.iter().map(|e| e.to_string()).collect(),
        )
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
                .chain([
                    (CameraMode::Auto, Aspect::Wide16x9),
                    (CameraMode::Auto, Aspect::Deck16x10),
                    (CameraMode::Fixed(1.5), Aspect::Deck16x10),
                ])
                .collect();
            for (cam, aspect) in modes {
                let (hash, views, events) = drive(room, &script, cam, aspect);
                assert_eq!(
                    hash, baseline.final_hash,
                    "{} {cam:?} {aspect:?}",
                    script.name
                );
                assert_eq!(events, baseline.events, "{} {cam:?}", script.name);
                view_sets.push(views);
            }
            // The camera really did something different at each coverage.
            assert_ne!(view_sets[0].last(), view_sets[3].last(), "{}", script.name);
            assert!(view_sets[0].iter().all(|v| v.w == 640));
            assert!(view_sets[3].iter().all(|v| v.w == 1120));
        }
    }

    /// The authoritative state does not depend on the output: the same 2P
    /// scripted run on a Deck (1280x800), a 1080p and a 4K machine, with the
    /// automatic camera and with fixed coverages, ends on one hash and one event
    /// log (the leash is a platform-independent constant).
    #[test]
    fn authoritative_hash_is_identical_at_every_output_resolution() {
        let rooms = rooms();
        for script in scripts() {
            let room = &rooms[script.room];
            let baseline = run(room, &script);
            for (aspect, out_w) in [
                (Aspect::Deck16x10, 1280u32),
                (Aspect::Wide16x9, 1920),
                (Aspect::Wide16x9, 3840),
                (Aspect::Wide16x9, 2560),
            ] {
                for cam in [CameraMode::Auto, CameraMode::Fixed(1.5)] {
                    let mut s =
                        Session::new_for_output(room, script.mode, script.seed, cam, aspect, out_w);
                    for _ in 0..script.ticks {
                        let inputs = script_inputs(&s.sim, &script);
                        s.step(inputs);
                    }
                    assert_eq!(
                        s.sim.state_hash(),
                        baseline.final_hash,
                        "{} {out_w} {cam:?}",
                        script.name
                    );
                    assert_eq!(s.sim.leash, crate::camera_lab::sim::DEFAULT_LEASH_PX);
                    let events: Vec<String> = s.sim.events.iter().map(|e| e.to_string()).collect();
                    assert_eq!(events, baseline.events, "{} {out_w}", script.name);
                }
            }
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
    fn metrics_show_that_a_wider_fixed_coverage_keeps_more_of_the_duo_in_view() {
        let rooms = rooms();
        // ROOM-05: the separation room.
        let script = scripts().remove(4);
        // The wide-leash alternative (736 px) is what separates the actors far
        // enough to need widening; at the default 416 px both fit a 1.0x frame.
        let m = |cam| {
            measure_with_leash(
                &rooms[4],
                &script,
                cam,
                Aspect::Wide16x9,
                1920,
                crate::camera_lab::sim::WIDE_LEASH_PX,
            )
        };
        let w1 = m(CameraMode::Fixed(1.0));
        let w15 = m(CameraMode::Fixed(1.5));
        let auto = m(CameraMode::Auto);
        assert!(w15.both_visible > w1.both_visible, "{w1:?} {w15:?}");
        assert!(auto.both_visible >= w15.both_visible.saturating_sub(script.ticks / 10));
        assert!(auto.actor_lost <= w1.actor_lost);
        assert!(
            auto.zoom_changes < 40,
            "no zoom pumping: {}",
            auto.zoom_changes
        );
        // A pan never snaps; the largest single-tick move of the auto camera is one
        // whole 32 px zoom step against a clamped room edge.
        assert!(
            w15.max_pan_px <= 12,
            "fixed pan snapped: {}",
            w15.max_pan_px
        );
        assert!(
            auto.max_pan_px <= 32,
            "auto moved {} px in a tick",
            auto.max_pan_px
        );
        // The metrics are reproducible.
        assert_eq!(auto, m(CameraMode::Auto));
    }

    #[test]
    fn auto_histogram_only_rest_widths_and_the_cap_holds_in_every_room() {
        use crate::camera_lab::camera_op::{AUTO_CAP_W, rest_widths};
        let rooms = rooms();
        for script in scripts() {
            let room = &rooms[script.room];
            for (aspect, out_w) in [
                (Aspect::Wide16x9, 1920u32),
                (Aspect::Wide16x9, 3840),
                (Aspect::Deck16x10, 1280),
            ] {
                let m = measure_for_output(room, &script, CameraMode::Auto, aspect, out_w);
                let rest = rest_widths(out_w);
                assert!(
                    m.max_width <= AUTO_CAP_W,
                    "{} {out_w}: width {}",
                    room.id,
                    m.max_width
                );
                let on_rest: u64 = m
                    .width_ticks
                    .iter()
                    .filter(|(w, _)| rest.contains(w))
                    .map(|(_, t)| *t)
                    .sum();
                assert!(
                    on_rest * 100 >= m.ticks * 90,
                    "{} {out_w}: only {on_rest} of {} ticks on {rest:?}: {:?}",
                    room.id,
                    m.ticks,
                    m.width_ticks
                );
                assert!(
                    m.longest_transit <= 72,
                    "{} {out_w}: a transit of {} ticks",
                    room.id,
                    m.longest_transit
                );
                if out_w == 1280 {
                    assert!(m.width_ticks.keys().all(|w| *w <= 640 || !rest.contains(w)));
                }
            }
        }
    }

    /// Transitions are continuous and eased: no per-tick jump of the width or
    /// the centre, bounded second differences (no kick when a transit or a pan
    /// starts, stops or changes urgency) and no jitter, in every room at 1080p,
    /// 4K and 1280x800.
    #[test]
    fn auto_camera_transitions_are_smooth_in_every_room_and_output() {
        let rooms = rooms();
        for script in scripts() {
            let room = &rooms[script.room];
            for (aspect, out_w) in [
                (Aspect::Wide16x9, 1920u32),
                (Aspect::Wide16x9, 3840),
                (Aspect::Deck16x10, 1280),
            ] {
                let mut s = Session::new_for_output(
                    room,
                    script.mode,
                    script.seed,
                    CameraMode::Auto,
                    aspect,
                    out_w,
                );
                let mut rows = vec![s.trace_row()];
                for _ in 0..script.ticks {
                    let inputs = script_inputs(&s.sim, &script);
                    s.step(inputs);
                    rows.push(s.trace_row());
                }
                let d = |f: &dyn Fn(&TraceRow) -> f32| -> Vec<f32> {
                    rows.windows(2).map(|w| f(&w[1]) - f(&w[0])).collect()
                };
                let dw = d(&|r| r.view.w as f32);
                let dcx = d(&|r| r.cx);
                let dcy = d(&|r| r.cy);
                let max = |v: &[f32]| v.iter().fold(0.0f32, |a, b| a.max(b.abs()));
                let ctx = format!("{} {out_w}", room.id);
                assert!(
                    max(&dw) <= 14.0,
                    "{ctx}: width jumped {} px in a tick",
                    max(&dw)
                );
                assert!(
                    max(&dcx) <= 12.5 && max(&dcy) <= 12.5,
                    "{ctx}: centre jumped {} {}",
                    max(&dcx),
                    max(&dcy)
                );
                let d2 = |v: &[f32]| -> Vec<f32> { v.windows(2).map(|w| w[1] - w[0]).collect() };
                // Second differences: the eased width accelerates by at most about
                // 1.3 px per tick squared (plus 2 px of rounding to even widths);
                // the pan by at most its acceleration limit plus rounding.
                assert!(
                    max(&d2(&dw)) <= 4.5,
                    "{ctx}: width kicked {}",
                    max(&d2(&dw))
                );
                assert!(
                    max(&d2(&dcx)) <= 3.5,
                    "{ctx}: pan kicked {}",
                    max(&d2(&dcx))
                );
                assert!(
                    max(&d2(&dcy)) <= 3.5,
                    "{ctx}: vertical pan kicked {}",
                    max(&d2(&dcy))
                );
                // No jitter: the width never reverses direction inside a transit
                // (a reversal needs a dwell of at least 72 ticks), and the pan does
                // not flip sign at speed more than a couple of times.
                let nz: Vec<f32> = dw.iter().copied().filter(|x| *x != 0.0).collect();
                let reversals = nz.windows(2).filter(|w| w[0] * w[1] < 0.0).count();
                assert!(
                    reversals <= 2 * 6,
                    "{ctx}: the width reversed {reversals} times"
                );
                let flips = dcx
                    .windows(2)
                    .filter(|w| w[0] * w[1] < 0.0 && w[0].abs() > 1.5 && w[1].abs() > 1.5)
                    .count();
                assert!(flips <= 3, "{ctx}: pan flipped sign {flips} times at speed");
            }
        }
    }

    #[test]
    fn auto_frames_the_ranged_tell_the_boss_and_the_duo() {
        let rooms = rooms();
        let scripts = scripts();
        let m = |i: usize| {
            measure_for_output(
                &rooms[i],
                &scripts[i],
                CameraMode::Auto,
                Aspect::Wide16x9,
                1920,
            )
        };
        // ROOM-02: at least the 85 percent a fixed 1.5x gives.
        let r2 = m(1);
        assert!(r2.telegraph_ticks > 100);
        assert!(
            r2.telegraph_visible * 100 >= r2.telegraph_ticks * 85,
            "{r2:?}"
        );
        // ROOM-05 with the wide-leash alternative: the separating run never
        // loses an actor (with the default 416 px leash both fit 1.0x).
        let r5 = measure_with_leash(
            &rooms[4],
            &scripts[4],
            CameraMode::Auto,
            Aspect::Wide16x9,
            1920,
            crate::camera_lab::sim::WIDE_LEASH_PX,
        );
        assert_eq!(r5.actor_lost, 0, "{r5:?}");
        assert_eq!(r5.both_visible, r5.ticks, "{r5:?}");
        // ROOM-06: the boss, tell and charge, is mostly in view (the eased, slower pan trades a few percent against the old 89 percent).
        let r6 = m(5);
        assert!(
            r6.telegraph_visible * 100 >= r6.telegraph_ticks * 80,
            "{r6:?}"
        );
        // Short brawls stay at 1.0x.
        for i in [0usize, 2, 3] {
            assert_eq!(m(i).max_width, 640, "{}", rooms[i].id);
        }
    }

    #[test]
    fn solo_runs_switch_once_and_the_companion_acts() {
        let rooms = rooms();
        for i in [0usize, 3, 5] {
            let script = solo_script(i);
            let r = run(&rooms[i], &script);
            assert!(
                r.events.iter().any(|e| e.contains("kind=switched")),
                "{}",
                rooms[i].id
            );
            let again = run(&rooms[i], &script);
            assert_eq!(r.final_hash, again.final_hash);
        }
    }

    #[test]
    fn auto_camera_keeps_both_actors_in_safety_in_every_scripted_run() {
        let rooms = rooms();
        for script in scripts() {
            let room = &rooms[script.room];
            let mut s = Session::new(
                room,
                script.mode,
                script.seed,
                CameraMode::Auto,
                Aspect::Wide16x9,
            );
            let mut out_of_safety = 0u64;
            let mut worst_share = 1.0f32;
            for tick in 0..script.ticks {
                let inputs = script_inputs(&s.sim, &script);
                let v = s.step(inputs);
                let (subs, _, _) = camera_input_of(&s.sim);
                if tick > 90 {
                    for sub in &subs {
                        if !safety_inside_in(
                            v,
                            sub,
                            Bounds {
                                w: room.render.0,
                                h: room.render.1,
                            },
                        ) {
                            out_of_safety += 1;
                        }
                        worst_share = worst_share.min(visible_share(v, sub));
                    }
                }
                assert!(
                    v.left >= 0
                        && v.right() <= room.render.0
                        && v.top >= 0
                        && v.bottom() <= room.render.1
                );
            }
            // Brief catch-up after a jump or a knockback is fine; a persistent
            // violation is not.
            assert!(
                out_of_safety < script.ticks / 4,
                "{}: {out_of_safety} ticks out of safety",
                script.name
            );
            assert!(
                worst_share > 0.5,
                "{}: an actor was {worst_share} visible",
                script.name
            );
        }
    }
}
