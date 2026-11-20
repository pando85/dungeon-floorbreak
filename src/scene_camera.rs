//! DEMO-01 PR C: the camera-lab automatic camera in the real Floor 01 scene.
//! Bevy-free and **presentation only**.
//!
//! The scroll camera of [`crate::corridor::SceneSim`] stays in the simulation
//! (it is part of `FLOOR1_SCENE_STATE` and every replay hash) and is never
//! replaced. This module owns a second, presentation-only camera: the pure
//! [`CameraOperator`] of the camera lab (eased coupled zoom and pan, integer
//! scale rest widths, 960 px cap, never outside the room), fed after each fixed
//! tick from a read-only look at the simulation. Nothing here is read by
//! movement, collision, combat, a replay or a hash.
//!
//! Inputs, all read-only:
//!
//! * the subjects are Carl and Donut, taken from [`SceneSim`];
//! * hit-stop and a fresh camera kick come from [`CombatState`] (no zoom-in and
//!   a slower pan, as in the lab);
//! * threats come from the [`CameraInterest`] list, which gameplay code (the
//!   Bad Llama of DEMO-01 PR B, later enemies) fills after its own tick. This is
//!   the whole interface: gameplay writes plain data, the camera never writes
//!   back.
//!
//! Coordinates of the interface are the scene's own: **y up**, the centre of the
//! body, in world pixels, like `Hero::y` and `Goblin::y`.

use crate::camera_lab::camera_op::{
    Aspect, BASE_W, Bounds, CameraInput, CameraMode, CameraOperator, Subject, Threat, View,
    coverage_width, rest_widths,
};
use std::path::Path;

use crate::camera_lab::layout::build_kit;
use crate::camera_lab::room::load_rooms;
use crate::camera_lab::sim::{DONUT_HALF_HEIGHT, DONUT_HALF_WIDTH};
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::env_kit::{EnvKit, ViewDef};
use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH};

/// Most threats handed to the operator at once (the lab's limit).
pub const MAX_THREATS: usize = 4;
/// A hero jump of at least this many px in one tick is a teleport (a respawn at
/// a checkpoint, a restart): the camera reframes at once instead of panning
/// across the room. Carl never moves this far in one tick by himself.
pub const TELEPORT_PX: f32 = 160.0;
/// More ticks than this since the last call are a jump in time (a capture
/// pre-simulation): reframe instead of replaying the easing.
pub const MAX_CATCH_UP_TICKS: u64 = 240;

/// Which camera the presentation uses. `Scroll` is the existing deterministic
/// scroll camera (every existing capture and CI image); the others are the
/// camera-lab operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraChoice {
    Scroll,
    Auto,
    /// A fixed coverage: 1.0, 1.25 or 1.5 (640, 800 or 960 px wide).
    Fixed(f32),
}

impl CameraChoice {
    /// `auto`, `scroll`, `1.0`, `1.25` or `1.5`.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "scroll" => Ok(Self::Scroll),
            other => match other.parse::<f32>() {
                Ok(c) if [1.0, 1.25, 1.5].contains(&c) => Ok(Self::Fixed(c)),
                _ => Err(format!(
                    "bad camera {other:?} (expected auto, scroll, 1.0, 1.25 or 1.5)"
                )),
            },
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Scroll => "scroll".into(),
            Self::Auto => "auto".into(),
            Self::Fixed(c) => format!("{c}"),
        }
    }

    /// The default: the scroll camera everywhere. The `--demo` run (DEMO-01
    /// PR A) passes `true` and gets `Auto`; every fixture, scenario and capture
    /// of the repository keeps the scroll camera.
    pub fn default_for(demo: bool) -> Self {
        if demo { Self::Auto } else { Self::Scroll }
    }

    fn mode(self) -> Option<CameraMode> {
        match self {
            Self::Scroll => None,
            Self::Auto => Some(CameraMode::Auto),
            Self::Fixed(c) => Some(CameraMode::Fixed(c)),
        }
    }

    /// Size in world pixels of the render targets the lighting pass must
    /// allocate: the widest view this choice can show **at this output**. The
    /// scroll camera keeps the 640 px baseline (and so every existing capture);
    /// the automatic camera takes its widest integer-scale rest width (960 at
    /// 1080p, 640 on the Deck and at 720p), so an output that cannot zoom pays
    /// for no larger target. The rig narrows it further to the room.
    pub fn slot_target(self, aspect: Aspect, out_w: u32) -> (i32, i32) {
        let w = match self {
            Self::Scroll => BASE_W,
            Self::Auto => rest_widths(out_w).last().copied().unwrap_or(BASE_W),
            Self::Fixed(c) => coverage_width(c),
        };
        (w, aspect.height_for(w))
    }
}

/// How the camera should treat one threat (the lab's priorities).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterestKind {
    /// A readable tell or charge about to commit, `ticks_left` ticks from now
    /// (priority 3).
    Tell { ticks_left: u16 },
    /// A projectile in flight, or a ranged foe in the shot's first ticks
    /// (priority 3).
    InFlight,
    /// A ranged foe that has acquired an actor with a clear line: the camera
    /// starts widening when it takes aim, before the tell (priority 2).
    Aiming,
    /// A large foe next to an actor (priority 1).
    Adjacent,
}

impl InterestKind {
    fn priority(self) -> u8 {
        match self {
            Self::Tell { .. } | Self::InFlight => 3,
            Self::Aiming => 2,
            Self::Adjacent => 1,
        }
    }

    fn tell_left(self) -> u16 {
        match self {
            Self::Tell { ticks_left } => ticks_left,
            _ => 0,
        }
    }
}

/// One thing besides the actors worth keeping in frame. Scene coordinates:
/// `x` and `y` are the body centre in world pixels, **y up**; `dir` is +1 right
/// or -1 left (the way it faces or travels).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InterestThreat {
    pub kind: InterestKind,
    pub x: f32,
    pub y: f32,
    pub half_w: f32,
    pub half_h: f32,
    pub dir: i8,
    /// Extra lane ahead of the threat in its direction (a charge's path), px;
    /// 0 for anything that does not charge.
    pub lane: f32,
}

impl InterestThreat {
    fn to_threat(self, layout_h: f32) -> Threat {
        Threat {
            x: self.x,
            y: layout_h - self.y,
            half_w: self.half_w,
            half_h: self.half_h,
            dir: self.dir,
            tell_left: self.kind.tell_left(),
            readable: true,
            priority: self.kind.priority(),
            lane: self.lane,
        }
    }
}

/// The read-only interface gameplay feeds: replace the list after each fixed
/// tick (`clear`, then `push`). Empty means nothing to frame beyond Carl and
/// Donut. As a Bevy resource it lives in the binary; this type stays pure.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CameraInterest {
    pub threats: Vec<InterestThreat>,
}

impl CameraInterest {
    pub fn clear(&mut self) {
        self.threats.clear();
    }

    pub fn push(&mut self, threat: InterestThreat) {
        self.threats.push(threat);
    }
}

/// DEMO-01 PR B: the Bad Llamas' contribution to the interest list, from the
/// finished tick of the simulation (simulation state only; nothing flows back).
/// A llama that has acquired Carl (alive, facing him, within its fire range) is
/// [`InterestKind::Aiming`], so the camera starts widening before the tell; in
/// its tell it is a [`InterestKind::Tell`] with the ticks left before the
/// release; from the release to the end of the attack it is
/// [`InterestKind::InFlight`], like every glob in flight (a one-tick threat is
/// safe: the camera steps per fixed tick).
pub fn llama_interest(sim: &SceneSim, combat: &CombatState, interest: &mut CameraInterest) {
    use crate::feel::{LLAMA_ATTACK_TICKS, LLAMA_FIRE_RANGE, LLAMA_TELL_TICKS};
    use crate::llama::{LLAMA_HALF_HEIGHT, LLAMA_HALF_WIDTH, LlamaMode};
    let carl_x = sim.world.hero.x;
    for llama in combat.llamas.iter().filter(|l| l.alive()) {
        let kind = match llama.mode {
            LlamaMode::Attack if llama.attack_tick < LLAMA_TELL_TICKS => Some(InterestKind::Tell {
                ticks_left: LLAMA_TELL_TICKS - llama.attack_tick,
            }),
            LlamaMode::Attack if llama.attack_tick < LLAMA_ATTACK_TICKS => {
                Some(InterestKind::InFlight)
            }
            LlamaMode::Neutral => {
                let dx = carl_x - llama.x;
                (llama.ai == crate::combat::EnemyAi::Chase
                    && dx.abs() <= LLAMA_FIRE_RANGE
                    && dx * f32::from(llama.facing) > 0.0)
                    .then_some(InterestKind::Aiming)
            }
            _ => None,
        };
        if let Some(kind) = kind {
            interest.push(InterestThreat {
                kind,
                x: llama.x,
                // The whole animal, head and neck included.
                y: llama.y - LLAMA_HALF_HEIGHT + 34.0,
                half_w: LLAMA_HALF_WIDTH + 24.0,
                half_h: 34.0,
                dir: llama.facing,
                lane: 0.0,
            });
        }
    }
    for glob in &combat.globs {
        interest.push(InterestThreat {
            kind: InterestKind::InFlight,
            x: glob.x,
            y: glob.y,
            half_w: 4.0,
            half_h: 3.0,
            dir: glob.dir,
            lane: 0.0,
        });
    }
}

/// Evidence helper behind `--camera-test-threat X:FIRST:LAST`: a synthetic
/// foe that has acquired Carl (an [`InterestKind::Aiming`] threat, the state a
/// Bad Llama takes before its tell) standing at world column `x` on the line
/// `floor_y` (y up) from tick `first` up to `last`, facing left. It stands in
/// for a llama in camera evidence; it is never a simulated enemy and nothing in
/// the simulation sees it. The real llama feeds [`llama_interest`].
pub fn synthetic_interest(
    x: f32,
    floor_y: f32,
    first: u64,
    last: u64,
    tick: u64,
) -> CameraInterest {
    let mut interest = CameraInterest::default();
    if (first..last).contains(&tick) {
        interest.push(InterestThreat {
            kind: InterestKind::Aiming,
            x,
            y: floor_y + 34.0,
            half_w: 24.0,
            half_h: 34.0,
            dir: -1,
            lane: 0.0,
        });
    }
    interest
}

/// Subjects, threats and the impact flag of the scene at its current tick, in
/// layout (y down) pixels.
fn camera_inputs(
    sim: &SceneSim,
    combat: Option<&CombatState>,
    interest: &CameraInterest,
) -> (Vec<Subject>, Vec<Threat>, bool) {
    let world = &sim.world;
    let h = world.layout_height() as f32;
    let hero = &world.hero;
    let donut = &world.donut;
    let subjects = vec![
        Subject {
            x: hero.x,
            y: h - hero.y,
            half_w: HERO_HALF_WIDTH,
            half_h: HERO_HALF_HEIGHT,
        },
        Subject {
            x: donut.x,
            y: h - (donut.foot_y + DONUT_HALF_HEIGHT),
            half_w: DONUT_HALF_WIDTH,
            half_h: DONUT_HALF_HEIGHT,
        },
    ];
    let mut threats: Vec<Threat> = interest.threats.iter().map(|t| t.to_threat(h)).collect();
    threats.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.x.total_cmp(&b.x)));
    threats.truncate(MAX_THREATS);
    let impact = combat.is_some_and(|c| {
        c.hero_hit_stop > 0
            || hero.melee.hit_stop_left() > 0
            || c.enemies.iter().any(|g| g.hit_stop > 0)
            || c.camera_kick(world.tick) != 0
    });
    (subjects, threats, impact)
}

/// Evidence and test helper: room `index` (1 to 6) of the camera lab composed
/// from the approved Floor 01 kit as a scene layout, with the two views the
/// scene's scroll camera reads. Those rooms are 760 to 1080 px tall, so a 960 px
/// view fits; the 400 px corridor of the sample layout cannot show any zoom.
/// A greybox composition, not an authored level (DEMO-01 PR A builds its own
/// hall the same way).
pub fn test_room_kit(root: &Path, index: usize) -> Result<EnvKit, String> {
    let rooms = load_rooms(root)?;
    let mut room = index
        .checked_sub(1)
        .and_then(|i| rooms.get(i))
        .cloned()
        .ok_or_else(|| format!("no camera test room {index} (1 to {})", rooms.len()))?;
    // Flat floor: the lab's steps and platforms are its own gameplay, and the
    // scene fixtures stand on columns 400 to 560.
    room.steps.clear();
    room.platforms.clear();
    let (mut kit, _, _) = build_kit(root, &room).map_err(|e| e.to_string())?;
    for (name, h) in [("abc_640x360", 360), ("abc_640x400", 400)] {
        kit.views.insert(
            name.into(),
            ViewDef {
                x: 0,
                y: room.floor_y - 300,
                w: 640,
                h,
            },
        );
    }
    Ok(kit)
}

/// The camera kick of CP-01 in whole pixels, scaled by the width so that it
/// moves the picture by the same number of output pixels at 1.5x as at 1.0x
/// (a world pixel is smaller on screen in a wider view).
pub fn scaled_kick(kick: i32, view_w: i32) -> i32 {
    (kick as f32 * view_w as f32 / BASE_W as f32).round() as i32
}

/// Where the walking line sits in the view at rest width `w`, in the HUD's
/// 360 px high layout space (the lab's floor share of the view height). The
/// HUD's notice box starts at 320; at 960 the line is at 310, about 10 px above
/// it (a known limit and a playtest item, see the scene doc).
pub fn walk_line_hud_px(w: i32) -> i32 {
    (crate::camera_lab::camera_op::floor_share(w) * 360.0).round() as i32
}

/// What a rig is built for: the camera, the output and the room.
#[derive(Debug, Clone, Copy)]
pub struct RigConfig {
    pub choice: CameraChoice,
    pub aspect: Aspect,
    /// Output width in pixels (decides the integer-scale rest widths).
    pub out_w: u32,
    pub bounds: Bounds,
}

/// The presentation camera of one scene run.
#[derive(Debug, Clone)]
pub struct SceneCameraRig {
    op: CameraOperator,
    cfg: RigConfig,
    last_elapsed: u64,
    last_hero: (f32, f32),
    cut: bool,
}

impl SceneCameraRig {
    /// A rig already framed on the current state (no ease-in). `None` for
    /// [`CameraChoice::Scroll`]: the scroll camera needs no rig.
    pub fn new(
        cfg: RigConfig,
        sim: &SceneSim,
        combat: Option<&CombatState>,
        interest: &CameraInterest,
        elapsed: u64,
    ) -> Option<Self> {
        let mode = cfg.choice.mode()?;
        let (subjects, threats, impact_hold) = camera_inputs(sim, combat, interest);
        let op = CameraOperator::new_for_output(
            mode,
            cfg.aspect,
            cfg.bounds,
            cfg.out_w,
            &CameraInput {
                subjects: &subjects,
                threats: &threats,
                impact_hold,
            },
        );
        Some(Self {
            op,
            cfg,
            last_elapsed: elapsed,
            last_hero: (sim.world.hero.x, sim.world.hero.y),
            cut: false,
        })
    }

    /// Ask for a cut at the next [`Self::sync`]: gameplay that knows it moved
    /// Carl (a checkpoint respawn, a restart) calls this instead of relying on
    /// the distance heuristic.
    pub fn request_cut(&mut self) {
        self.cut = true;
    }

    /// Size in world pixels of the lighting slot targets for this rig: the
    /// widest view it can show in this room at this output.
    pub fn slot_target(&self) -> (i32, i32) {
        let w = match self.cfg.choice {
            CameraChoice::Auto => self.op.auto_cap(),
            _ => self.op.view().w,
        };
        let w = w.max(BASE_W.min(self.cfg.bounds.w));
        (w, self.cfg.aspect.height_for(w))
    }

    /// Drop the easing and frame the current state at once.
    pub fn reframe(
        &mut self,
        sim: &SceneSim,
        combat: Option<&CombatState>,
        interest: &CameraInterest,
        elapsed: u64,
    ) {
        if let Some(fresh) = Self::new(self.cfg, sim, combat, interest, elapsed) {
            *self = fresh;
        }
    }

    /// Bring the camera up to the simulation: one operator step per simulated
    /// tick since the last call (`elapsed` counts ticks over restarts, like
    /// `CombatState::elapsed`). A restart, a teleport or a long jump in time
    /// reframes at once. Returns the view (before the kick).
    pub fn sync(
        &mut self,
        sim: &SceneSim,
        combat: Option<&CombatState>,
        interest: &CameraInterest,
        elapsed: u64,
    ) -> View {
        let behind = elapsed.saturating_sub(self.last_elapsed);
        let limit = TELEPORT_PX + 8.0 * behind as f32;
        let teleported = (sim.world.hero.x - self.last_hero.0).abs() > limit
            || (sim.world.hero.y - self.last_hero.1).abs() > limit;
        if self.cut || elapsed < self.last_elapsed || behind > MAX_CATCH_UP_TICKS || teleported {
            self.reframe(sim, combat, interest, elapsed);
            return self.op.view();
        }
        self.last_elapsed = elapsed;
        self.last_hero = (sim.world.hero.x, sim.world.hero.y);
        let (subjects, threats, impact_hold) = camera_inputs(sim, combat, interest);
        for _ in 0..behind {
            self.op.step(&CameraInput {
                subjects: &subjects,
                threats: &threats,
                impact_hold,
            });
        }
        self.op.view()
    }

    /// The view without the kick.
    pub fn view(&self) -> View {
        self.op.view()
    }

    /// The view as drawn: the CP-01 kick (the scroll camera's whole-pixel
    /// offset, scaled by the width) added to the left edge and kept inside the
    /// room.
    pub fn presented(&self, kick: i32) -> View {
        let v = self.op.view();
        let kick = scaled_kick(kick, v.w);
        View {
            left: (v.left + kick).clamp(0, (self.cfg.bounds.w - v.w).max(0)),
            ..v
        }
    }

    pub fn operator(&self) -> &CameraOperator {
        &self.op
    }

    pub fn choice(&self) -> CameraChoice {
        self.cfg.choice
    }

    /// One `FLOOR1_VIEW` line for `--view-trace`: the presented view of a tick.
    pub fn trace_line(&self, tick: u64, kick: i32) -> String {
        let v = self.presented(kick);
        format!(
            "FLOOR1_VIEW tick={tick} left={} top={} w={} h={} rest={} kick={kick} kick_px={}",
            v.left,
            v.top,
            v.w,
            v.h,
            self.op.rest_target(),
            scaled_kick(kick, v.w)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_lab::camera_op::{MIN_W, is_integer_scale};
    use crate::combat_scene::Fixture;
    use crate::input::RawInput;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    /// ROOM-01 of the camera lab as a Floor 01 scene layout (760 px tall, so a
    /// 960 px view fits).
    fn tall_kit() -> EnvKit {
        test_room_kit(root(), 1).unwrap()
    }

    fn bounds(kit: &EnvKit) -> Bounds {
        Bounds {
            w: kit.width,
            h: kit.height,
        }
    }

    fn walk(kit: &EnvKit) -> SceneSim {
        SceneSim::new(kit, 360).unwrap()
    }

    fn step_right(sim: &mut SceneSim) {
        sim.step(RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        });
    }

    fn aiming(sim: &SceneSim, dx: f32) -> InterestThreat {
        InterestThreat {
            kind: InterestKind::Aiming,
            x: sim.world.hero.x + dx,
            y: sim.world.hero.y,
            half_w: 24.0,
            half_h: 34.0,
            dir: -1,
            lane: 0.0,
        }
    }

    /// DEMO-01 PR B: the real llama feeds the interest list from simulation
    /// state: aiming once it has acquired Carl, a tell with the ticks left, then
    /// in flight with its glob; a dead llama feeds nothing.
    #[test]
    fn the_bad_llama_feeds_aiming_tell_and_in_flight_threats() {
        use crate::combat_scene::{CP05_LLAMA_SLAM, CP05_LLAMA_SPIT, FixtureSim};
        let kit = EnvKit::load(root(), "assets/levels/floor01/corridor_abc_sample.json").unwrap();
        let interest_of = |run: &FixtureSim| {
            let mut interest = CameraInterest::default();
            llama_interest(&run.scene, &run.combat, &mut interest);
            interest.threats
        };
        let mut run = FixtureSim::new(&kit, 360, &CP05_LLAMA_SPIT).unwrap();
        // Before its first tick: it faces Carl within range, so it is aiming.
        let before = interest_of(&run);
        assert_eq!(before.len(), 1);
        assert_eq!(before[0].kind, InterestKind::Aiming);
        // In the tell (started on tick 1, attack tick 0): 36 ticks left.
        run.step_scripted();
        let tell = interest_of(&run);
        assert!(matches!(
            tell[0].kind,
            InterestKind::Tell { ticks_left: 36 }
        ));
        // From the release (tick 37): the llama in the shot's first ticks and
        // the glob in flight.
        run.run_scripted(36);
        let shot = interest_of(&run);
        assert_eq!(shot.len(), 2, "the llama and its glob");
        assert!(shot.iter().all(|t| t.kind == InterestKind::InFlight));
        assert_eq!(shot[1].dir, -1);
        // A defeated llama feeds nothing.
        let mut slam = FixtureSim::new(&kit, 360, &CP05_LLAMA_SLAM).unwrap();
        slam.run_scripted(CP05_LLAMA_SLAM.ticks);
        assert!(interest_of(&slam).is_empty());
    }

    #[test]
    fn camera_choice_parses_and_defaults_to_scroll_outside_the_demo() {
        assert_eq!(CameraChoice::parse("auto"), Ok(CameraChoice::Auto));
        assert_eq!(CameraChoice::parse("scroll"), Ok(CameraChoice::Scroll));
        assert_eq!(CameraChoice::parse("1.5"), Ok(CameraChoice::Fixed(1.5)));
        assert_eq!(CameraChoice::parse("1.25"), Ok(CameraChoice::Fixed(1.25)));
        assert!(CameraChoice::parse("1.75").is_err());
        assert!(CameraChoice::parse("wide").is_err());
        assert_eq!(CameraChoice::default_for(false), CameraChoice::Scroll);
        assert_eq!(CameraChoice::default_for(true), CameraChoice::Auto);
    }

    #[test]
    fn slot_targets_keep_the_baseline_for_scroll_and_follow_the_output() {
        for aspect in [Aspect::Wide16x9, Aspect::Deck16x10] {
            let h = aspect.base_h();
            assert_eq!(CameraChoice::Scroll.slot_target(aspect, 1920), (640, h));
            assert_eq!(CameraChoice::Fixed(1.25).slot_target(aspect, 1920).0, 800);
        }
        // Auto: the widest integer-scale rest width of the output.
        assert_eq!(
            CameraChoice::Auto.slot_target(Aspect::Wide16x9, 1920),
            (960, 540)
        );
        assert_eq!(
            CameraChoice::Auto.slot_target(Aspect::Wide16x9, 3840),
            (960, 540)
        );
        assert_eq!(
            CameraChoice::Auto.slot_target(Aspect::Deck16x10, 1280),
            (640, 400)
        );
        assert_eq!(
            CameraChoice::Auto.slot_target(Aspect::Wide16x9, 1280),
            (640, 360)
        );
        // The rig narrows it to the room too: the 400 px corridor cannot show 960.
        let kit = tall_kit();
        let sim = walk(&kit);
        let rig = SceneCameraRig::new(
            RigConfig {
                choice: CameraChoice::Auto,
                aspect: Aspect::Wide16x9,
                out_w: 1920,
                bounds: Bounds { w: 1584, h: 400 },
            },
            &sim,
            None,
            &CameraInterest::default(),
            0,
        )
        .unwrap();
        assert_eq!(rig.slot_target(), (640, 360));
    }

    /// The HUD owner's number: where the walking line sits in the HUD's 360 px
    /// layout space at each rest width (the notice box starts at 320).
    #[test]
    fn the_walking_line_sits_low_at_rest_widths_hud_clearance_is_a_playtest_item() {
        assert_eq!(walk_line_hud_px(640), 288);
        assert_eq!(walk_line_hud_px(768), 297);
        assert_eq!(walk_line_hud_px(960), 310);
        // Margin to the notice box (320): 32, 23 and 10 px.
        assert_eq!(320 - walk_line_hud_px(960), 10);
    }

    #[test]
    fn rest_widths_are_exact_integer_scales_at_every_output() {
        let kit = tall_kit();
        let sim = walk(&kit);
        let interest = CameraInterest::default();
        for (out_w, out_h, expect) in [
            (1920, 1080, vec![640, 960]),
            (3840, 2160, vec![640, 768, 960]),
            (1280, 800, vec![640]),
            (1280, 720, vec![640]),
        ] {
            let rig = SceneCameraRig::new(
                RigConfig {
                    choice: CameraChoice::Auto,
                    aspect: Aspect::from_output(out_w, out_h),
                    out_w,
                    bounds: bounds(&kit),
                },
                &sim,
                None,
                &interest,
                0,
            )
            .unwrap();
            assert_eq!(rig.operator().rest_set(), expect, "{out_w}x{out_h}");
            assert_eq!(rest_widths(out_w), {
                let mut v = vec![640];
                v.extend(expect.iter().copied().filter(|w| *w != 640));
                v
            });
            assert!(rig.view().w == 640, "starts at the 640 rest width");
            assert!(is_integer_scale(out_w, rig.view().w));
        }
    }

    #[test]
    fn the_scroll_choice_builds_no_rig() {
        let kit = tall_kit();
        let sim = walk(&kit);
        let rig = SceneCameraRig::new(
            RigConfig {
                choice: CameraChoice::Scroll,
                aspect: Aspect::Wide16x9,
                out_w: 1920,
                bounds: bounds(&kit),
            },
            &sim,
            None,
            &CameraInterest::default(),
            0,
        );
        assert!(rig.is_none());
    }

    #[test]
    fn the_zoom_eases_out_and_back_and_the_pan_stays_coupled_and_limited() {
        let kit = tall_kit();
        let mut sim = walk(&kit);
        let mut interest = CameraInterest::default();
        let mut rig = SceneCameraRig::new(
            RigConfig {
                choice: CameraChoice::Auto,
                aspect: Aspect::Wide16x9,
                out_w: 1920,
                bounds: bounds(&kit),
            },
            &sim,
            None,
            &interest,
            0,
        )
        .unwrap();
        // Settle, walk 120 ticks, then a foe takes aim for 200 ticks, then calm.
        let mut views: Vec<View> = vec![rig.view()];
        let mut elapsed = 0u64;
        for tick in 0..900u64 {
            step_right(&mut sim);
            elapsed += 1;
            interest.clear();
            if (120..320).contains(&tick) {
                interest.push(aiming(&sim, 470.0));
            }
            views.push(rig.sync(&sim, None, &interest, elapsed));
        }
        // It widened to exactly 960 and returned to exactly 640.
        let widths: Vec<i32> = views.iter().map(|v| v.w).collect();
        assert_eq!(*widths.iter().max().unwrap(), 960);
        assert_eq!(*widths.last().unwrap(), 640);
        let first_wide = widths.iter().position(|w| *w > 640).unwrap();
        assert!(first_wide >= 120, "no zoom before the foe takes aim");
        // Every view is whole even pixels at the 16:9 aspect and never void.
        for v in &views {
            assert_eq!(v.w % 2, 0);
            assert!(v.left >= 0 && v.right() <= kit.width, "{v:?}");
            assert!(v.top >= 0 && v.bottom() <= kit.height, "{v:?}");
        }
        // Eased transit: the width changes by at most 14 px a tick, starts and
        // ends slowly (smaller steps than the middle of the transit).
        let steps: Vec<i32> = widths.windows(2).map(|p| (p[1] - p[0]).abs()).collect();
        assert!(steps.iter().all(|s| *s <= 14), "{:?}", steps.iter().max());
        let out_start = first_wide - 1;
        let out: Vec<i32> = steps[out_start..]
            .iter()
            .copied()
            .take_while(|s| *s > 0)
            .collect();
        let peak = *out.iter().max().unwrap();
        assert!(out.len() >= 20, "a transit takes many ticks: {}", out.len());
        assert!(out[0] < peak && *out.last().unwrap() < peak, "{out:?}");
        // The pan is coupled to the zoom (the left edge moves while the width
        // changes) and speed limited to 12.5 px a tick.
        let centre = |v: &View| v.left as f32 + v.w as f32 / 2.0;
        let pan: Vec<f32> = views
            .windows(2)
            .map(|p| (centre(&p[1]) - centre(&p[0])).abs())
            .collect();
        assert!(
            pan.iter().all(|p| *p <= 12.5),
            "{:?}",
            pan.iter().cloned().fold(0.0, f32::max)
        );
        // The pan has no kick: the centre speed changes smoothly (at most 2.5 px
        // a tick between two ticks, rounding included).
        let speed: Vec<f32> = views
            .windows(2)
            .map(|p| centre(&p[1]) - centre(&p[0]))
            .collect();
        assert!(
            speed.windows(2).all(|s| (s[1] - s[0]).abs() <= 3.5),
            "{:?}",
            speed
                .windows(2)
                .map(|s| (s[1] - s[0]).abs())
                .fold(0.0, f32::max)
        );
    }

    #[test]
    fn it_starts_framed_on_a_foe_and_returns_to_640_when_calm() {
        let kit = tall_kit();
        let sim = walk(&kit);
        let mut interest = CameraInterest::default();
        interest.push(aiming(&sim, 540.0));
        let mut rig = SceneCameraRig::new(
            RigConfig {
                choice: CameraChoice::Auto,
                aspect: Aspect::Wide16x9,
                out_w: 1920,
                bounds: bounds(&kit),
            },
            &sim,
            None,
            &interest,
            0,
        )
        .unwrap();
        assert_eq!(rig.view().w, 960, "starts framed on the foe");
        // Calm for a long time: it returns to 640.
        interest.clear();
        for elapsed in 1..=400 {
            rig.sync(&sim, None, &interest, elapsed);
        }
        assert_eq!(rig.view().w, 640);
    }

    #[test]
    fn the_kick_is_scaled_so_it_moves_the_picture_the_same_output_pixels() {
        assert_eq!(scaled_kick(6, 640), 6);
        assert_eq!(scaled_kick(-6, 640), -6);
        assert_eq!(scaled_kick(6, 960), 9);
        assert_eq!(scaled_kick(-6, 960), -9);
        assert_eq!(scaled_kick(0, 960), 0);
        // On screen: 1080p, 3 output px per world px at 640 and 2 at 960.
        assert_eq!(scaled_kick(6, 640) * 3, scaled_kick(6, 960) * 2);

        let kit = tall_kit();
        let sim = walk(&kit);
        let rig = SceneCameraRig::new(
            RigConfig {
                choice: CameraChoice::Fixed(1.5),
                aspect: Aspect::Wide16x9,
                out_w: 1920,
                bounds: bounds(&kit),
            },
            &sim,
            None,
            &CameraInterest::default(),
            0,
        )
        .unwrap();
        let base = rig.view();
        assert_eq!(base.w, 960);
        assert_eq!(rig.presented(0), base);
        assert_eq!(rig.presented(4).left - base.left, 6);
        // The kick never pushes the view out of the room.
        let far = rig.presented(10_000);
        assert!(far.right() <= kit.width && far.left >= 0);
        let near = rig.presented(-10_000);
        assert_eq!(near.left, 0);
    }

    #[test]
    fn a_teleport_or_a_restart_reframes_without_a_long_pan() {
        let kit = tall_kit();
        let mut sim = walk(&kit);
        let interest = CameraInterest::default();
        let mut rig = SceneCameraRig::new(
            RigConfig {
                choice: CameraChoice::Auto,
                aspect: Aspect::Wide16x9,
                out_w: 1920,
                bounds: bounds(&kit),
            },
            &sim,
            None,
            &interest,
            0,
        )
        .unwrap();
        let mut elapsed = 0;
        for _ in 0..600 {
            step_right(&mut sim);
            elapsed += 1;
            rig.sync(&sim, None, &interest, elapsed);
        }
        let before = rig.view();
        assert!(before.left > 100);
        // Restart: the simulation tick counter goes back to zero.
        sim.reset();
        let after = rig.sync(&sim, None, &interest, 0);
        assert!(after.left < before.left - 100, "{after:?} {before:?}");
        // A respawn: the hero is moved far away between two ticks.
        for _ in 0..300 {
            step_right(&mut sim);
            elapsed += 1;
            rig.sync(&sim, None, &interest, elapsed);
        }
        let mut moved = sim.clone();
        moved.world.hero.x = 1400.0;
        let jumped = rig.sync(&moved, None, &interest, elapsed + 1);
        assert!(jumped.left > 600, "{jumped:?}");
    }

    #[test]
    fn a_requested_cut_and_a_vertical_jump_reframe_at_once() {
        let kit = tall_kit();
        let mut sim = walk(&kit);
        let interest = CameraInterest::default();
        let mut rig = SceneCameraRig::new(
            RigConfig {
                choice: CameraChoice::Auto,
                aspect: Aspect::Wide16x9,
                out_w: 1920,
                bounds: bounds(&kit),
            },
            &sim,
            None,
            &interest,
            0,
        )
        .unwrap();
        let mut elapsed = 0;
        for _ in 0..300 {
            step_right(&mut sim);
            elapsed += 1;
            rig.sync(&sim, None, &interest, elapsed);
        }
        let before = rig.view();
        // A short respawn (60 px, under the heuristic) still cuts when asked.
        let mut moved = sim.clone();
        moved.world.hero.x -= 60.0;
        moved.world.donut.x -= 60.0;
        elapsed += 1;
        let eased = rig.sync(&moved, None, &interest, elapsed);
        assert!(
            (eased.left - before.left).abs() <= 12,
            "no cut without a request"
        );
        rig.request_cut();
        moved.world.hero.x -= 120.0;
        moved.world.donut.x -= 120.0;
        elapsed += 1;
        let cut = rig.sync(&moved, None, &interest, elapsed);
        assert!(cut.left < before.left - 100, "{cut:?} {before:?}");
        // A vertical jump over the limit cuts without a request.
        let mut up = moved.clone();
        up.world.hero.y += 300.0;
        elapsed += 1;
        let before_y = rig.view().top;
        let jumped = rig.sync(&up, None, &interest, elapsed);
        assert!((jumped.top - before_y).abs() > 12, "{jumped:?} {before_y}");
    }

    /// Recorded runs through the real scene simulation at the three outputs:
    /// the view is never outside the room, the width rests on integer scales,
    /// at most 14 px of width and 12.5 px of pan per tick, and a threat in the
    /// middle makes the 1080p and 4K cameras widen and return while the Deck and
    /// 720p cameras stay at 640.
    #[test]
    fn recorded_trace_at_the_three_outputs_keeps_the_lab_rules() {
        let kit = tall_kit();
        for (out_w, out_h) in [(1920u32, 1080u32), (3840, 2160), (1280, 800)] {
            let mut sim = walk(&kit);
            let mut interest = CameraInterest::default();
            let aspect = Aspect::from_output(out_w, out_h);
            let mut rig = SceneCameraRig::new(
                RigConfig {
                    choice: CameraChoice::Auto,
                    aspect,
                    out_w,
                    bounds: bounds(&kit),
                },
                &sim,
                None,
                &interest,
                0,
            )
            .unwrap();
            let rest = rig.operator().rest_set().to_vec();
            let mut prev = rig.view();
            let (mut on_rest, mut total, mut widest) = (0u32, 0u32, 0);
            for tick in 0..1100u64 {
                if tick < 700 {
                    step_right(&mut sim);
                }
                interest.clear();
                if (200..380).contains(&tick) {
                    interest.push(aiming(&sim, 470.0));
                }
                let v = rig.sync(&sim, None, &interest, tick + 1);
                assert!(v.left >= 0 && v.right() <= kit.width, "{out_w}: {v:?}");
                assert!(v.top >= 0 && v.bottom() <= kit.height, "{out_w}: {v:?}");
                assert!(v.w >= MIN_W && v.w <= 960);
                assert!((v.w - prev.w).abs() <= 14, "{out_w}: {prev:?} {v:?}");
                let c = |v: View| v.left as f32 + v.w as f32 / 2.0;
                assert!((c(v) - c(prev)).abs() <= 12.5, "{out_w}: {prev:?} {v:?}");
                total += 1;
                on_rest += u32::from(rest.contains(&v.w));
                widest = widest.max(v.w);
                prev = v;
            }
            assert!(
                f64::from(on_rest) >= 0.9 * f64::from(total),
                "{out_w}: {on_rest}/{total} ticks on a rest width"
            );
            if out_w == 1280 {
                assert_eq!(widest, 640, "no zoom at the Deck output");
            } else {
                // The smallest integer-scale rest width that holds the foe:
                // 960 at 1080p, 768 (5x) at 4K.
                assert_eq!(widest, if out_w == 1920 { 960 } else { 768 }, "{out_w}");
                assert_eq!(prev.w, 640, "{out_w}: it returned");
            }
        }
    }

    /// The presentation camera never feeds the simulation: a combat fixture
    /// replays to the identical state hash and event log with the camera off,
    /// scroll, automatic and fixed 1.5, a threat list and a kick every tick.
    #[test]
    fn the_replay_hash_is_identical_with_the_camera_on_and_off() {
        let kit = tall_kit();
        let fixture = Fixture::by_name("cp01_haymaker_chain").expect("fixture");
        let run = |choice: Option<CameraChoice>| -> (u64, Vec<String>, u64) {
            let mut sim = SceneSim::new_at(&kit, 360, fixture.carl_x).unwrap();
            let mut combat = CombatState::new(fixture, &kit, &sim).unwrap();
            let mut interest = CameraInterest::default();
            let mut rig = choice.and_then(|choice| {
                SceneCameraRig::new(
                    RigConfig {
                        choice,
                        aspect: Aspect::Wide16x9,
                        out_w: 1920,
                        bounds: bounds(&kit),
                    },
                    &sim,
                    Some(&combat),
                    &interest,
                    0,
                )
            });
            let mut views = 0u64;
            let mut impacts = 0;
            for _ in 0..fixture.ticks {
                let raw = fixture.input(sim.world.tick + 1);
                combat.step(&mut sim, raw);
                impacts += u32::from(camera_inputs(&sim, Some(&combat), &interest).2);
                if let Some(rig) = rig.as_mut() {
                    interest.clear();
                    interest.push(aiming(&sim, 470.0));
                    let kick = combat.camera_kick(sim.world.tick);
                    rig.sync(&sim, Some(&combat), &interest, combat.elapsed);
                    let v = rig.presented(kick);
                    views = views.wrapping_mul(31).wrapping_add((v.left + v.w) as u64);
                }
            }
            let events = combat.events.iter().map(|e| format!("{e}")).collect();
            if rig.is_some() {
                assert!(impacts > 0, "the fixture has hit-stop or a kick");
            }
            (combat.state_hash(&sim), events, views)
        };
        let off = run(None);
        assert!(!off.1.is_empty(), "the fixture produced events");
        for choice in [
            CameraChoice::Scroll,
            CameraChoice::Auto,
            CameraChoice::Fixed(1.0),
            CameraChoice::Fixed(1.5),
        ] {
            let on = run(Some(choice));
            assert_eq!((on.0, &on.1), (off.0, &off.1), "{choice:?}");
        }
        // The camera did work (the view trace differs between modes).
        assert_ne!(
            run(Some(CameraChoice::Auto)).2,
            run(Some(CameraChoice::Fixed(1.5))).2
        );
    }
}
