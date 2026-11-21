//! CARL-PROG-01 CP-B2: the ANIM-CARL-03 throw as a pure function of the throw
//! clock (CP-Q14), standing and moving.
//!
//! Presentation only. The simulation owns the clock
//! ([`crate::thrown_charge::Thrower::throw_clock`]); nothing here flows back, so
//! no frame can change an event, a hash or an input timing. The frame is a pure
//! function of `(clock, the run frame Carl's locomotion shows)`, held through a
//! hit-stop for free because the clock does not move.
//!
//! * **Standing** (idle or any non-run locomotion): the nine frames of the
//!   `dynamite_throw` tag by clock, `t0_draw` .. `t8_settle` (spec section 3).
//! * **Moving** (the locomotion state is `run`): the six phases that move the
//!   arms (light, hold, hold_b, wind, release, follow) draw
//!   `dynamite_move_<phase>` frame `run frame - 4` (spec section 5); draw, brace
//!   and settle return `None`, so the plain run frame shows ("the arms swing as
//!   usual"). The clock and the spawn tick are the same in both cases.

use super::{ArtPose, SpriteClips};

/// Total ticks of the throw (CP-Q14, spec section 3).
pub const THROW_TICKS: u64 = 70;

/// The nine standing phases, in frame order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrowPhase {
    Draw,
    Light,
    Hold,
    HoldB,
    Wind,
    Release,
    Follow,
    Brace,
    Settle,
}

impl ThrowPhase {
    /// `(phase, first clock tick)`: the starts of the nine frames (spec section
    /// 3: t0, t6, t16, t23, t30, t38, t42, t50, t62; the last ends on t69).
    pub const STARTS: [(Self, u64); 9] = [
        (Self::Draw, 0),
        (Self::Light, 6),
        (Self::Hold, 16),
        (Self::HoldB, 23),
        (Self::Wind, 30),
        (Self::Release, 38),
        (Self::Follow, 42),
        (Self::Brace, 50),
        (Self::Settle, 62),
    ];

    /// The phase shown on throw clock `clock`, `None` past the 70 ticks.
    pub fn of(clock: u64) -> Option<Self> {
        if clock >= THROW_TICKS {
            return None;
        }
        Self::STARTS
            .iter()
            .rev()
            .find(|(_, start)| clock >= *start)
            .map(|(phase, _)| *phase)
    }

    /// The frame inside the `dynamite_throw` tag.
    pub fn standing_index(self) -> usize {
        self as usize
    }

    /// The moving tag of a phase that moves the arms, `None` for draw, brace and
    /// settle (the plain run frame shows).
    pub fn moving_tag(self) -> Option<&'static str> {
        match self {
            Self::Light => Some("dynamite_move_light"),
            Self::Hold => Some("dynamite_move_hold"),
            Self::HoldB => Some("dynamite_move_hold_b"),
            Self::Wind => Some("dynamite_move_wind"),
            Self::Release => Some("dynamite_move_release"),
            Self::Follow => Some("dynamite_move_follow"),
            Self::Draw | Self::Brace | Self::Settle => None,
        }
    }
}

/// The run frames the moving variant bakes (the locomotion run clip is six
/// frames, atlas 4 to 9; spec section 5 draws frame `n - 4`).
pub const RUN_FRAMES: usize = 6;

/// Carl's frame while the throw runs. `run_frame` is the frame inside the
/// locomotion `run` clip when Carl runs (`Some(0..6)`), `None` otherwise
/// (standing, jumping: the standing frames).
///
/// `None` is returned past the throw and for the moving draw, brace and settle
/// phases (the locomotion animation draws Carl).
pub fn throw_pose(clips: &SpriteClips, clock: u64, run_frame: Option<usize>) -> Option<ArtPose> {
    let phase = ThrowPhase::of(clock)?;
    match run_frame {
        None => Some(ArtPose {
            tag: "dynamite_throw",
            frame: clips.frame_in("dynamite_throw", phase.standing_index()),
        }),
        Some(run) => {
            let tag = phase.moving_tag()?;
            Some(ArtPose {
                tag,
                frame: clips.frame_in(tag, run.min(RUN_FRAMES - 1)),
            })
        }
    }
}

/// The stick as drawn (`vfx_dynamite` atlas frames, spec section 7).
///
/// * Flight: `dynamite_flight`, frames 0 to 3, 3 ticks each (a forward tumble,
///   12 ticks per turn), from the release.
/// * Landed: `dynamite_fizzle`, frames 4 and 5, 4 ticks each, from the landing.
pub const FLIGHT_FRAMES: usize = 4;
pub const FLIGHT_FRAME_TICKS: u64 = 3;
pub const FIZZLE_FIRST: usize = 4;
pub const FIZZLE_FRAMES: usize = 2;
pub const FIZZLE_FRAME_TICKS: u64 = 4;

/// The atlas frame of the stick `age` ticks after its release (flying) or after
/// its landing (landed). Pure.
pub const fn stick_frame(landed: bool, age: u64) -> usize {
    if landed {
        FIZZLE_FIRST + ((age / FIZZLE_FRAME_TICKS) % FIZZLE_FRAMES as u64) as usize
    } else {
        ((age / FLIGHT_FRAME_TICKS) % FLIGHT_FRAMES as u64) as usize
    }
}

/// `n` points on a circle of `radius` around the origin (screen-integer), for
/// the `--debug-combat` blast ring (CP-Q33: the 64 px reach is wider than the
/// burst art, so the ring shows the real reach). Pure; point `i` sits at angle
/// `i * 360 / n` degrees starting on the +x axis.
pub fn ring_points(radius: f32, n: usize) -> Vec<(i32, i32)> {
    (0..n)
        .map(|i| {
            let a = std::f32::consts::TAU * i as f32 / n as f32;
            (
                (radius * a.cos()).round() as i32,
                (radius * a.sin()).round() as i32,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::{ClipSet, Tag};
    use crate::cf02_fixtures::find_party_fixture;
    use crate::combat_anim::{CombatAnimators, FrameSource};
    use crate::env_kit::EnvKit;
    use crate::party_scene::PartyFixtureSim;
    use std::path::Path;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn clips() -> SpriteClips {
        SpriteClips::load_carl_dynamite(root(), FrameSource::GridSource).unwrap()
    }

    #[test]
    fn the_sheet_loads_with_nine_standing_frames_and_six_moving_tags() {
        let c = clips();
        assert_eq!(c.frame_count, 45);
        assert_eq!(c.tag("dynamite_throw"), (0, 9));
        for (i, name) in [
            "dynamite_move_light",
            "dynamite_move_hold",
            "dynamite_move_hold_b",
            "dynamite_move_wind",
            "dynamite_move_release",
            "dynamite_move_follow",
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(c.tag(name), (9 + 6 * i, 6), "{name}");
        }
        // The nine standing frames add up to the 70 ticks of CP-Q14.
        assert_eq!(c.tag_ticks("dynamite_throw"), Some(70));
        assert_eq!(THROW_TICKS, 70);
        assert_eq!((c.spec.width, c.spec.height), (48, 56));
        assert_eq!((c.spec.pivot_x, c.spec.pivot_y), (25, 55));
    }

    #[test]
    fn the_phase_starts_equal_the_manifest_tick_ranges() {
        let text =
            std::fs::read_to_string(root().join("assets/manifests/carl_book1_h52_dynamite.json"))
                .unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&text).unwrap();
        let frames = manifest["dynamite"]["moves"]["dynamite_throw"]["frames"]
            .as_array()
            .unwrap();
        for (i, (phase, start)) in ThrowPhase::STARTS.iter().enumerate() {
            let range = &frames[i]["ticks"];
            assert_eq!(range[0].as_u64(), Some(*start), "{phase:?} starts");
            assert_eq!(phase.standing_index(), i);
            // The last tick in, and the first tick out, of every phase.
            let last = range[1].as_u64().unwrap();
            assert_eq!(ThrowPhase::of(*start), Some(*phase));
            assert_eq!(ThrowPhase::of(last), Some(*phase), "{phase:?} last tick");
            if i + 1 < 9 {
                assert_eq!(ThrowPhase::of(last + 1), Some(ThrowPhase::STARTS[i + 1].0));
            }
        }
        assert_eq!(ThrowPhase::of(69), Some(ThrowPhase::Settle));
        assert_eq!(ThrowPhase::of(70), None, "the throw is 70 ticks");
        // The release key is the frame of t38..t41 (the stick spawns on t41).
        assert_eq!(ThrowPhase::of(41), Some(ThrowPhase::Release));
        assert_eq!(ThrowPhase::of(42), Some(ThrowPhase::Follow));
    }

    #[test]
    fn standing_shows_the_nine_frames_in_order_and_nothing_past_the_throw() {
        let c = clips();
        let mut seen = Vec::new();
        for clock in 0..THROW_TICKS {
            let pose = throw_pose(&c, clock, None).unwrap();
            assert_eq!(pose.tag, "dynamite_throw");
            if seen.last() != Some(&pose.frame) {
                seen.push(pose.frame);
            }
        }
        assert_eq!(seen, (0..9).collect::<Vec<_>>());
        assert_eq!(throw_pose(&c, 70, None), None);
        assert_eq!(throw_pose(&c, 70, Some(2)), None);
    }

    /// The moving overlay frame mapping (spec section 5): while Carl runs, the six
    /// arm phases draw `dynamite_move_<phase>` frame `run frame`; draw, brace and
    /// settle draw nothing (the plain run frame shows).
    #[test]
    fn moving_draws_the_overlay_of_the_phase_on_the_run_frame_and_nothing_for_draw_brace_settle() {
        let c = clips();
        for clock in 0..THROW_TICKS {
            let phase = ThrowPhase::of(clock).unwrap();
            for run in 0..RUN_FRAMES {
                let pose = throw_pose(&c, clock, Some(run));
                match phase.moving_tag() {
                    Some(tag) => {
                        let pose = pose.unwrap();
                        assert_eq!(pose.tag, tag, "clock {clock}");
                        assert_eq!(pose.frame, c.tag(tag).0 + run, "clock {clock} run {run}");
                    }
                    None => assert_eq!(pose, None, "clock {clock} ({phase:?}) shows the run frame"),
                }
            }
        }
        // The release overlay on the last tick it is drawn, and the follow on the next.
        assert_eq!(
            throw_pose(&c, 41, Some(3)).unwrap().frame,
            c.tag("dynamite_move_release").0 + 3
        );
        assert_eq!(
            throw_pose(&c, 42, Some(3)).unwrap().frame,
            c.tag("dynamite_move_follow").0 + 3
        );
        assert_eq!(
            throw_pose(&c, 49, Some(0)).unwrap().tag,
            "dynamite_move_follow"
        );
        assert_eq!(
            throw_pose(&c, 50, Some(0)),
            None,
            "the brace is the plain run"
        );
    }

    #[test]
    fn the_run_frame_index_of_the_locomotion_clip_matches_the_overlay_layout() {
        let read = |p: &str| std::fs::read_to_string(root().join(p)).unwrap();
        let clips = ClipSet::from_grid_json(
            &read("assets/manifests/carl_book1_h52_locomotion.json"),
            &read("assets/source/characters/carl/carl_h52_locomotion.grid.json"),
        )
        .unwrap();
        // Spec section 5: locomotion frames 4 to 9, six run frames.
        let run = clips.clip(Tag::Run);
        assert_eq!((run.first, run.len()), (4, RUN_FRAMES));
    }

    #[test]
    fn the_stick_tumbles_in_flight_and_burns_once_landed() {
        // Flight: 3 ticks a frame, 12 ticks a turn (edges on both sides).
        let flight: Vec<usize> = (0..13).map(|a| stick_frame(false, a)).collect();
        assert_eq!(flight, [0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 0]);
        // Fizzle: frames 4 and 5, 4 ticks each.
        let fizzle: Vec<usize> = (0..9).map(|a| stick_frame(true, a)).collect();
        assert_eq!(fizzle, [4, 4, 4, 4, 5, 5, 5, 5, 4]);
    }

    #[test]
    fn the_debug_ring_lies_on_the_blast_radius() {
        let ring = ring_points(64.0, 40);
        assert_eq!(ring.len(), 40);
        assert_eq!(ring[0], (64, 0));
        assert_eq!(ring[10], (0, 64));
        assert_eq!(ring[20], (-64, 0));
        assert_eq!(ring[30], (0, -64));
        for (x, y) in ring {
            let r = f32::hypot(x as f32, y as f32);
            assert!((r - 64.0).abs() <= 1.0, "({x}, {y}) is {r} from the centre");
        }
    }

    /// The anim trace of a real standing and moving throw: every tick's frame is
    /// `throw_pose` of the sim's own throw clock, the throw starts on the press
    /// tick and the animators change no state line outside it.
    #[test]
    fn the_animator_follows_the_throw_clock_of_the_simulation() {
        let kit = EnvKit::load(root(), "assets/levels/floor01/corridor_abc_sample.json").unwrap();
        for (name, run) in [
            ("carl_prog_dyn_arc_standing", None),
            ("carl_prog_dyn_arc_moving", Some(2)),
        ] {
            let fixture = find_party_fixture(name).unwrap();
            let mut sim = PartyFixtureSim::new(&kit, 360, fixture).unwrap();
            let carl = SpriteClips::load_carl_melee(root(), FrameSource::GridSource).unwrap();
            let goblin = |v| SpriteClips::load_goblin(root(), v, FrameSource::GridSource).unwrap();
            let mut anim = CombatAnimators::new(carl, [goblin(0), goblin(1)], &sim.combat)
                .with_dynamite(clips());
            let sheet = clips();
            let mut drawn = Vec::new();
            for _ in 0..fixture.base.ticks {
                sim.step_scripted();
                anim.step(&sim.scene, &sim.combat);
                // The moving case hands the animator a run frame.
                let moving = run.filter(|_| sim.scene.world.hero.vx.abs() > 1.0);
                anim.step_throw(&sim.scene, &sim.combat, moving);
                let tick = sim.scene.world.tick;
                let clock = sim.combat.thrower().throw_clock(tick);
                let expected = clock.and_then(|c| throw_pose(&sheet, c, moving));
                assert_eq!(anim.carl_throw_pose(), expected, "{name} tick {tick}");
                if let Some(pose) = expected {
                    drawn.push((tick, clock.unwrap(), pose.label()));
                    assert!(
                        anim.state_line(tick)
                            .ends_with(&format!(" throw={}", pose.label()))
                    );
                } else {
                    assert!(!anim.state_line(tick).contains(" throw="));
                }
            }
            // The clock is 0 on the press tick; the moving draw phase shows the plain
            // run frame, so its first drawn tick is t6 (the fuse lit).
            let press = sim
                .combat
                .events
                .iter()
                .find_map(|e| match e {
                    crate::combat::CombatEvent::SpecialStarted { tick, .. } => Some(*tick),
                    _ => None,
                })
                .unwrap();
            let first = drawn.first().unwrap();
            if run.is_none() {
                assert_eq!((first.0, first.1), (press, 0), "{name}");
                assert_eq!(drawn.len() as u64, THROW_TICKS, "{name}");
                assert_eq!(drawn.last().unwrap().1, 69);
            } else {
                // Moving: draw (6), brace (12) and settle (8) draw the plain run frame.
                assert_eq!((first.0, first.1), (press + 6, 6), "{name}");
                assert_eq!(drawn.len() as u64, THROW_TICKS - 26, "{name}");
            }
        }
    }
}
