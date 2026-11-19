//! Floor 1 corridor scene: COMP-01 integration plus the COMP-02 lighting pass
//! and the ANIM-INT-01 locomotion animation.
//!
//! Renders the ENV-01 kit layout with the approved Carl 52 px and Donut 40 px
//! locomotion atlases (idle, run, jump, land; the deterministic state machine
//! lives in `anim` / `scene_anim`, this binary only maps its state to an atlas
//! cell). Pixel layers stay at the virtual resolution; the A1 lighting pass
//! (ambient, torch pools, shadow, haze, glow, flame bloom) runs at output
//! resolution (`light_pass`) and is presentation only. `--no-lighting` draws
//! the COMP-01 look. The normal route has no enemies and no new mechanics; the
//! COMBAT-INT-01 `--fixture` scenarios add Carl's melee and placeholder goblins
//! on clean ground (see `combat_scene`). Simulation,
//! camera and collision come from the pure library modules; this binary only
//! reads input, then draws that state. Character art is fan-concept material
//! and does not imply game distribution rights.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use bevy::{
    app::AppExit,
    asset::AssetPlugin,
    asset::LoadState,
    asset::RenderAssetUsages,
    camera::{ScalingMode, visibility::RenderLayers},
    diagnostic::DiagnosticsStore,
    prelude::*,
    render::{
        diagnostic::RenderDiagnosticsPlugin,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        view::screenshot::{Screenshot, save_to_disk},
    },
    sprite::Anchor,
    window::{PresentMode, PrimaryWindow, WindowResolution},
};
use dungeon_floorbreak::{
    actor_sprite::{
        ActorAssets, ActorSpec, FrameSource, actor_center, load_carl, load_donut, shadow_center_x,
    },
    anim::{Animator, Tag},
    audio::{
        AudioCommand, AudioSettings, StyleBank, cues_from_combat_event, scene_ambience, scene_music,
    },
    audio_adapter::{AudioQueue, FloorAudioPlugin, resolve_pack_dir},
    blast::BLAST_CENTRE_DY,
    blast_art::{BURST_IMAGE, BlastArt, DYNAMITE_IMAGE},
    combat::{Attacker, ENEMY_HALF_HEIGHT, ENEMY_HALF_WIDTH, Phase, hit_box},
    combat_anim::{
        CombatAnimators, GOBLIN_VARIANTS, SpriteClips, carl_above_goblin, goblin_variant,
    },
    combat_scene::{CombatState, Fixture},
    corridor::{CorridorWorld, Scenario, SceneSim},
    death_present::{
        DeathPresenter, DrawLayer, FLOOR_DECAL_POOL_CAPACITY, FLOOR_DECAL_POOL_SLOTS,
        PART_POOL_CAPACITY, PART_POOL_SLOTS, WALL_DECAL_POOL_CAPACITY, WALL_DECAL_POOL_SLOTS,
        blast_vfx_frame, charge_frame, draw_z,
    },
    env_kit::{
        EnvKit, IRect, PieceDraw, animation_frame, parallax_copies, parallax_start, to_world_y,
    },
    goblin::{AttackPhase, GoblinMode},
    gore::{DEATH_VARIANTS, DeathArt, GorePreset, Sheet},
    input::RawInput,
    light_pass::{
        LAYER_DEBUG, LightPassPlugin, LightRig, LightingMaterial, PassFrame, SceneCamera,
        slot_layer, spawn_rig, update_rig,
    },
    lighting::{MAX_SHADOWS, ShadowEllipse, ViewRect, contact_shadow},
    prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Hero},
    scene_anim::SceneAnimators,
    shore_art::{ShoreArt, ShoreSlot, landed_age},
};

/// Draw order above the actor plane. Carl is drawn above every goblin, except
/// that a goblin in its smear or impact is drawn over him (its club lands on
/// him) while he is not in his own active window; during Carl's active frames he
/// is above his target (the melee spec's integration rule: his fist overlaps the
/// goblin's head by up to 6 px). Lying goblins (down, corpse) sit below Donut and everything else.
const CARL_Z_OFFSET: f32 = 0.1;
const GOBLIN_Z_OFFSET: f32 = 0.05;
const GOBLIN_SWING_Z_OFFSET: f32 = 0.15;
const GOBLIN_LYING_Z_OFFSET: f32 = -0.01;
/// Per-goblin z step so two goblins never share a depth: the earlier goblin is
/// in front (a stacked later one is drawn behind it).
const GOBLIN_Z_STEP: f32 = 0.001;
/// Impact flash (presentation): the sprite is drawn this much brighter for the
/// flash ticks after a hit.
const FLASH_TINT: Color = Color::linear_rgb(2.0, 2.0, 2.0);

const LAYOUT_PATH: &str = "assets/levels/floor01/corridor_abc_sample.json";
const VIRTUAL_WIDTH: i32 = 640;
const WATCHDOG_SECONDS: f32 = 10.0;
const TITLE: &str = "THE CRAWL - Floor 1 Corridor";

#[derive(Debug, Clone)]
struct Args {
    deck: bool,
    headless: bool,
    freeze: bool,
    debug_collision: bool,
    scenario: Option<Scenario>,
    /// COMBAT-INT-01 combat fixture (mutually exclusive with `--scenario`).
    fixture: Option<&'static Fixture>,
    /// Fixture only: the keyboard or pad drives Carl instead of the script.
    manual: bool,
    /// Fixture only: draw hit and hurt boxes (debug overlay).
    debug_combat: bool,
    capture_tick: Option<u64>,
    scale: u32,
    /// Draw without the lighting pass (the COMP-01 look).
    no_lighting: bool,
    /// Review capture: camera and actors pinned to the ENV-01 lit preview frame.
    reference_frame: bool,
    /// Measure this many frame intervals after warm-up, print them and exit.
    perf_frames: Option<usize>,
    /// DEATH-E1, fixture only (with `--perf-frames`): pre-simulate this many
    /// ticks, then step exactly one tick per measured frame (instead of the
    /// real-time fixed step), so every frame covers the same tick on every run
    /// and the probe sees the same deaths whatever the frame time.
    perf_from_tick: Option<u64>,
    /// Live run: step the scripted scenario for this many fixed ticks in the
    /// real render loop, print `FLOOR1_SCENE_STATE` and exit.
    exit_after_ticks: Option<u64>,
    /// Review capture sequence `first:step:count`: pre-simulate `first` ticks
    /// of the scenario, then save `count` window screenshots `step` ticks
    /// apart into `capture_dir` and exit.
    capture_sequence: Option<(u64, u64, u64)>,
    capture_dir: Option<PathBuf>,
    /// Headless only: print one `FLOOR1_ANIM_TRACE` line per simulated tick of a
    /// scenario, or one `COMBAT_ANIM_TRACE` line per tick of a fixture.
    anim_trace: bool,
    /// Fixture only, headless: one `COMBAT_TRACE` row per simulated tick.
    combat_trace: bool,
    /// No audio device and no sound (AUDIO-01); `FLOORBREAK_AUDIO=off` does the same.
    no_audio: bool,
    /// Style bank override (`arcade`, `cinematic`, `hybrid`); F9 cycles it live.
    audio_style: Option<StyleBank>,
    /// DEATH-C: the gore preset 1 to 4 (presentation only; default 4, full).
    /// It never reaches the simulation: every replay hash is the same at any
    /// preset.
    gore_preset: GorePreset,
    /// Fixture only, lab: keep the death presenter's pools loaded with
    /// synthetic defeats (frame-time measurement at max load).
    death_stress: bool,
    /// Headless fixture only: print the presenter's `DEATH_PRESENT` summary.
    death_trace: bool,
}

/// Parse `first:step:count` of `--capture-sequence`.
fn parse_sequence(value: &str) -> Result<(u64, u64, u64), String> {
    let bad = || format!("bad capture sequence {value:?} (expected first:step:count)");
    let mut parts = value.split(':');
    let mut next =
        || -> Result<u64, String> { parts.next().ok_or_else(bad)?.parse().map_err(|_| bad()) };
    let (first, step, count) = (next()?, next()?, next()?);
    if parts.next().is_some() || step == 0 || !(1..=1000).contains(&count) {
        return Err(bad());
    }
    Ok((first, step, count))
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut parsed = Args {
        deck: false,
        headless: false,
        freeze: false,
        debug_collision: false,
        scenario: None,
        fixture: None,
        manual: false,
        debug_combat: false,
        capture_tick: None,
        scale: 2,
        no_lighting: false,
        reference_frame: false,
        perf_frames: None,
        perf_from_tick: None,
        exit_after_ticks: None,
        capture_sequence: None,
        capture_dir: None,
        anim_trace: false,
        combat_trace: false,
        no_audio: false,
        audio_style: None,
        gore_preset: GorePreset::default(),
        death_stress: false,
        death_trace: false,
    };
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--deck" => parsed.deck = true,
            "--headless-floor01-smoke" => parsed.headless = true,
            "--freeze" => parsed.freeze = true,
            "--debug-collision" => parsed.debug_collision = true,
            "--manual" => parsed.manual = true,
            "--debug-combat" => parsed.debug_combat = true,
            "--fixture" => {
                let value = iter.next().ok_or("--fixture needs a name")?;
                parsed.fixture = Some(Fixture::by_name(value).ok_or_else(|| {
                    format!("unknown fixture {value:?} (expected {})", Fixture::names())
                })?);
            }
            "--no-lighting" => parsed.no_lighting = true,
            "--no-audio" => parsed.no_audio = true,
            "--audio-style" => {
                let value = iter.next().ok_or("--audio-style needs a name")?;
                parsed.audio_style = Some(StyleBank::parse(value).ok_or_else(|| {
                    format!("unknown audio style {value:?} (expected arcade, cinematic, hybrid)")
                })?);
            }
            "--gore-preset" => {
                let value = iter.next().ok_or("--gore-preset needs 1, 2, 3 or 4")?;
                parsed.gore_preset = GorePreset::parse(value).ok_or_else(|| {
                    format!(
                        "bad gore preset {value:?} (expected 1 to 4, or off, reduced, medium, full)"
                    )
                })?;
            }
            "--death-stress" => parsed.death_stress = true,
            "--death-trace" => parsed.death_trace = true,
            "--reference-frame" => parsed.reference_frame = true,
            "--anim-trace" => parsed.anim_trace = true,
            "--combat-trace" => parsed.combat_trace = true,
            "--capture-sequence" => {
                let value = iter
                    .next()
                    .ok_or("--capture-sequence needs first:step:count")?;
                parsed.capture_sequence = Some(parse_sequence(value)?);
            }
            "--capture-dir" => {
                let value = iter.next().ok_or("--capture-dir needs a directory")?;
                parsed.capture_dir = Some(PathBuf::from(value));
            }
            "--exit-after-ticks" => {
                let value = iter.next().ok_or("--exit-after-ticks needs a tick count")?;
                parsed.exit_after_ticks = Some(
                    value
                        .parse()
                        .ok()
                        .filter(|n| *n > 0)
                        .ok_or_else(|| format!("bad tick count {value:?}"))?,
                );
            }
            "--perf-frames" => {
                let value = iter.next().ok_or("--perf-frames needs a frame count")?;
                parsed.perf_frames = Some(
                    value
                        .parse()
                        .ok()
                        .filter(|n| (30..=100_000).contains(n))
                        .ok_or_else(|| format!("bad frame count {value:?} (30..=100000)"))?,
                );
            }
            "--perf-from-tick" => {
                let value = iter.next().ok_or("--perf-from-tick needs a tick")?;
                parsed.perf_from_tick =
                    Some(value.parse().map_err(|_| format!("bad tick {value:?}"))?);
            }
            "--scenario" => {
                let value = iter.next().ok_or("--scenario needs a name")?;
                parsed.scenario = Some(value.parse()?);
            }
            "--capture-tick" => {
                let value = iter.next().ok_or("--capture-tick needs a tick count")?;
                parsed.capture_tick = Some(
                    value
                        .parse()
                        .map_err(|_| format!("bad tick count {value:?}"))?,
                );
            }
            "--scale" => {
                let value = iter.next().ok_or("--scale needs an integer")?;
                parsed.scale = value
                    .parse()
                    .ok()
                    .filter(|s| (1..=8).contains(s))
                    .ok_or_else(|| format!("bad scale {value:?} (1..=8)"))?;
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    let driven = parsed.scenario.is_some() || parsed.fixture.is_some();
    if parsed.scenario.is_some() && parsed.fixture.is_some() {
        return Err("--scenario and --fixture are mutually exclusive".into());
    }
    if (parsed.manual || parsed.debug_combat) && parsed.fixture.is_none() {
        return Err("--manual and --debug-combat need --fixture".into());
    }
    if (parsed.death_stress || parsed.death_trace) && parsed.fixture.is_none() {
        return Err("--death-stress and --death-trace need --fixture".into());
    }
    if parsed.death_stress && (parsed.headless || parsed.capture_tick.is_some()) {
        return Err(
            "--death-stress is a live run: drop --headless-floor01-smoke and --capture-tick".into(),
        );
    }
    if parsed.death_trace && !parsed.headless {
        return Err("--death-trace needs --headless-floor01-smoke".into());
    }
    if parsed.manual
        && (parsed.headless
            || parsed.capture_tick.is_some()
            || parsed.capture_sequence.is_some()
            || parsed.exit_after_ticks.is_some()
            || parsed.freeze)
    {
        return Err("--manual is a live keyboard run: drop the scripted-run flags".into());
    }
    if parsed.capture_tick.is_some() && !driven {
        return Err("--capture-tick needs --scenario or --fixture".into());
    }
    if parsed.exit_after_ticks.is_some()
        && (!driven || parsed.capture_tick.is_some() || parsed.freeze || parsed.reference_frame)
    {
        return Err(
            "--exit-after-ticks needs --scenario or --fixture and a live run (no --capture-tick, --freeze or --reference-frame)"
                .into(),
        );
    }
    if parsed.perf_from_tick.is_some()
        && (parsed.fixture.is_none()
            || parsed.perf_frames.is_none()
            || parsed.headless
            || parsed.manual
            || parsed.capture_tick.is_some()
            || parsed.capture_sequence.is_some()
            || parsed.exit_after_ticks.is_some()
            || parsed.freeze
            || parsed.reference_frame)
    {
        return Err(
            "--perf-from-tick needs --fixture and --perf-frames, and no other run mode".into(),
        );
    }
    if parsed.capture_sequence.is_some() != parsed.capture_dir.is_some() {
        return Err("--capture-sequence and --capture-dir go together".into());
    }
    if parsed.capture_sequence.is_some()
        && (!driven
            || parsed.capture_tick.is_some()
            || parsed.exit_after_ticks.is_some()
            || parsed.freeze
            || parsed.reference_frame
            || parsed.perf_frames.is_some())
    {
        return Err(
            "--capture-sequence needs --scenario or --fixture and no --capture-tick, --exit-after-ticks, --freeze, --reference-frame or --perf-frames"
                .into(),
        );
    }
    if parsed.combat_trace && (!parsed.headless || parsed.fixture.is_none()) {
        return Err("--combat-trace needs --headless-floor01-smoke and --fixture".into());
    }
    if parsed.anim_trace && (!parsed.headless || !driven) {
        return Err(
            "--anim-trace needs --headless-floor01-smoke and --scenario or --fixture".into(),
        );
    }
    if parsed.reference_frame && (driven || parsed.capture_tick.is_some()) {
        return Err(
            "--reference-frame pins its own state; drop --scenario/--fixture/--capture-tick".into(),
        );
    }
    Ok(parsed)
}

fn view_height(deck: bool) -> i32 {
    if deck { 400 } else { 360 }
}

/// Ticks a scenario is pre-simulated for: the capture tick, or the first tick
/// of a capture sequence.
fn presim_ticks(args: &Args) -> Option<u64> {
    args.capture_tick
        .or(args.capture_sequence.map(|(first, _, _)| first))
        .or(args.perf_from_tick)
}

/// Build the simulation and its animators in the requested state (scenario
/// pre-simulated). The animators are stepped from the finished state of every
/// tick and never feed back into the simulation.
type Scene = (
    SceneSim,
    SceneAnimators,
    Option<CombatState>,
    Option<DeathPresenter>,
);

/// The cosmetic death presenter of a fixture run: the goblin death art at the
/// requested preset. Presentation only.
fn death_presenter(root: &Path, args: &Args) -> Result<DeathPresenter, String> {
    let art = Arc::new(DeathArt::load(root)?);
    let variant_b = DeathArt::load_variant(root, 1)?;
    DeathPresenter::with_variants(art, &variant_b, args.gore_preset)
}

fn build_scene(kit: &EnvKit, args: &Args, actors: &Actors, root: &Path) -> Result<Scene, String> {
    if let Some(fixture) = args.fixture {
        // A combat fixture: Carl starts at the fixture column; the script is
        // pre-simulated for a capture, otherwise left for the live loop.
        let mut sim = SceneSim::new_at(kit, view_height(args.deck), fixture.carl_x)?;
        let mut combat = CombatState::new(fixture, kit, &sim)?;
        let mut anim = actors.animators(&sim.world);
        anim.art = actors.combat_art(&combat);
        let mut death = death_presenter(root, args)?;
        for _ in 0..presim_ticks(args).unwrap_or(0) {
            let raw = fixture.input(sim.world.tick + 1);
            step_combat(&mut sim, &mut combat, &mut anim, Some(&mut death), raw);
            if args.death_stress && args.perf_from_tick.is_some() {
                // Lab: load the pools before the probe starts measuring.
                death.stress_tick(sim.world.hero.x, sim.world.hero.y - HERO_HALF_HEIGHT);
            }
            if args.combat_trace {
                println!("{}", combat.trace_line(&sim));
            }
            if args.anim_trace
                && let Some(art) = anim.art.as_ref()
            {
                println!("{}", art.trace_line(&sim, &combat));
            }
        }
        return Ok((sim, anim, Some(combat), Some(death)));
    }
    let mut sim = SceneSim::new(kit, view_height(args.deck))?;
    let mut anim = actors.animators(&sim.world);
    if let (Some(scenario), Some(ticks)) = (args.scenario, presim_ticks(args)) {
        let trace = args.anim_trace;
        sim.run_scenario_observed(scenario, ticks, |state| {
            anim.step(&state.world);
            if trace {
                println!("{}", trace_line(state, &anim));
            }
        });
    }
    Ok((sim, anim, None, None))
}

/// One fixed tick of a combat fixture, and the animation that follows it. A
/// restart inside the step (Carl was defeated) restarts the animation too.
fn step_combat(
    sim: &mut SceneSim,
    combat: &mut CombatState,
    anim: &mut SceneAnimators,
    death: Option<&mut DeathPresenter>,
    raw: RawInput,
) {
    let expected = sim.world.tick + 1;
    combat.step(sim, raw);
    if sim.world.tick == expected {
        anim.step(&sim.world);
        anim.step_art(sim, combat);
    } else {
        anim.reset(&sim.world);
        anim.reset_art(combat);
    }
    // Cosmetics follow the finished tick; nothing flows back.
    if let Some(death) = death {
        death.observe(combat, sim);
    }
}

/// The combat state lines of a fixture: the sim state, then the frames drawn.
fn print_combat_state(sim: &SceneSim, anim: &SceneAnimators, combat: &CombatState) {
    println!("{}", combat.state_line(sim));
    if let Some(art) = anim.art.as_ref() {
        println!("{}", art.state_line(sim.world.tick));
    }
}

/// Ticks stepped so far: over every attempt in a fixture (a restart brings
/// the scene tick back to 0), the scene tick otherwise.
fn elapsed_ticks(sim: &SceneSim, combat: Option<&CombatState>) -> u64 {
    combat.map_or(sim.world.tick, |combat| combat.elapsed)
}

/// Print the combat events recorded since the last call and queue their
/// sounds (presentation only: the queue is never read back by the simulation).
fn print_combat_events(combat: &mut CombatState, mut audio: Option<&mut AudioQueue>) {
    for event in combat.take_new_events() {
        println!("{event}");
        if let Some(audio) = audio.as_deref_mut() {
            audio.push_cues(cues_from_combat_event(event));
        }
    }
}

/// One `--anim-trace` row: simulation values the timeline needs, and what each
/// animator shows.
fn trace_line(sim: &SceneSim, anim: &SceneAnimators) -> String {
    let hero = &sim.world.hero;
    let donut = &sim.world.donut;
    format!(
        "FLOOR1_ANIM_TRACE tick={} hero_x={:.3} hero_foot_y={:.3} vx={:.3} vy={:.3} grounded={} donut_x={:.3} donut_foot_y={:.3} carl={} donut={}",
        sim.world.tick,
        hero.x,
        hero.y - HERO_HALF_HEIGHT,
        hero.vx,
        hero.vy,
        u8::from(hero.grounded),
        donut.x,
        donut.foot_y,
        anim.carl.label(),
        anim.donut.label(),
    )
}

fn failure(message: &str) -> ExitCode {
    println!("FLOOR1_ASSET_FAILURE: {message}");
    eprintln!("FLOOR1_ASSET_FAILURE: {message}");
    ExitCode::from(1)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let args = match parse_args(&args) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("floor01_scene: {message}");
            return ExitCode::from(2);
        }
    };
    let root = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(error) => return failure(&format!("no working directory: {error}")),
    };
    let kit = match EnvKit::load(&root, LAYOUT_PATH) {
        Ok(kit) => kit,
        Err(error) => return failure(&format!("{error} (run from the repository root)")),
    };
    // The windowed scene draws the exported atlases; the headless smoke needs
    // no export and reads the same frame data from the committed pixel grids.
    let source = if args.headless {
        FrameSource::GridSource
    } else {
        FrameSource::ExportedAtlas
    };
    // The blast atlases are exported art: only a rendered fixture with charges
    // needs them (the headless replay draws nothing).
    let draws_blast = !args.headless && args.fixture.is_some_and(|f| !f.charges.is_empty());
    let actors = match Actors::load(&root, source, args.fixture.is_some(), draws_blast) {
        Ok(actors) => actors,
        Err(error) => {
            return failure(&format!(
                "{error} (run bash scripts/export_assets.sh from the repository root)"
            ));
        }
    };
    if args.headless {
        return match headless_smoke(&kit, &args, &actors, &root) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                println!("FLOOR1_HEADLESS_FAILURE: {message}");
                ExitCode::from(1)
            }
        };
    }
    let (sim, anim, combat, death) = match build_scene(&kit, &args, &actors, &root) {
        Ok(scene) => scene,
        Err(message) => return failure(&message),
    };
    run_window(kit, sim, anim, combat, death, actors, args, &root);
    ExitCode::SUCCESS
}

// ------------------------------------------------------------------ resources

#[derive(Resource)]
struct Sim(SceneSim);

/// Presentation-only animation state, stepped once per fixed tick from the
/// finished simulation state.
#[derive(Resource)]
struct Anim(SceneAnimators);

/// Carl's and Donut's frame data, read from the manifests and atlas JSON. A
/// combat fixture also loads the melee atlas and the two goblin atlases.
#[derive(Resource, Clone)]
struct Actors {
    carl: ActorAssets,
    donut: ActorAssets,
    art: Option<CombatArt>,
    /// The ENV-02 shore and block art of a combat fixture (DEATH-C).
    shore: Option<ShoreArt>,
    /// DEATH-D: the QA charge and blast frames of a fixture with charges
    /// (unreviewed prototype art, drawn in the QA fixtures only).
    blast: Option<BlastArt>,
}

#[derive(Clone)]
struct CombatArt {
    carl: SpriteClips,
    goblins: [SpriteClips; GOBLIN_VARIANTS],
}

impl Actors {
    fn load(root: &Path, source: FrameSource, combat: bool, blast: bool) -> Result<Self, String> {
        let blast = if blast {
            Some(BlastArt::load(root)?)
        } else {
            None
        };
        let shore = if combat {
            Some(ShoreArt::load(root)?)
        } else {
            None
        };
        let art = if combat {
            Some(CombatArt {
                carl: SpriteClips::load_carl_melee(root, source)?,
                goblins: [
                    SpriteClips::load_goblin(root, 0, source)?,
                    SpriteClips::load_goblin(root, 1, source)?,
                ],
            })
        } else {
            None
        };
        Ok(Self {
            carl: load_carl(root, source)?,
            donut: load_donut(root, source)?,
            art,
            shore,
            blast,
        })
    }

    /// The melee and goblin animators of a combat fixture.
    fn combat_art(&self, combat: &CombatState) -> Option<CombatAnimators> {
        self.art
            .as_ref()
            .map(|art| CombatAnimators::new(art.carl.clone(), art.goblins.clone(), combat))
    }

    fn animators(&self, world: &CorridorWorld) -> SceneAnimators {
        SceneAnimators::new(
            Animator::new(self.carl.clips.clone(), self.carl.pacing),
            Animator::new(self.donut.clips.clone(), self.donut.pacing),
            world,
        )
    }
}

#[derive(Resource)]
struct Setup {
    kit: EnvKit,
    args: Args,
}

#[derive(Resource)]
struct Frozen(bool);

/// Fixed tick after which a live scripted run prints its state and exits.
#[derive(Resource)]
struct ExitAfter(Option<u64>);

/// Whether the lighting pass is in use (layout has a profile and the flag is
/// not set).
#[derive(Resource)]
struct LightingOn(bool);

/// Review-capture pose of the ENV-01 lit preview (`preview.actor_check` of the
/// sample layout): fixed camera, Carl and Donut standing at fixed x, first
/// frames, presentation tick 0. Pure presentation: the simulation is untouched.
#[derive(Resource)]
struct ReferencePose(Option<RefPose>);

#[derive(Clone, Copy)]
struct RefPose {
    carl_x: f32,
    donut_x: f32,
}

/// Preview actor positions of the sample layout (`preview.actor_check`, first
/// Carl and Donut pair); a test checks them against the layout file.
const REFERENCE_CARL_X: f32 = 392.0;
const REFERENCE_DONUT_X: f32 = 454.0;
/// Name of the layout view the preview uses.
const REFERENCE_VIEW: &str = "abc_640x360";
const REFERENCE_VIEW_DECK: &str = "abc_640x400";

/// Frame-interval probe for `--perf-frames`.
#[derive(Resource)]
struct Perf {
    target: usize,
    warmup: usize,
    seen: usize,
    intervals_ms: Vec<f32>,
    /// `--perf-from-tick`: one fixture tick per measured frame.
    step_ticks: bool,
    /// Per measured frame: (tick, ms, live parts, live floor decals, live wall
    /// decals, visible death sprites); printed with the stats.
    series: Vec<(u64, f32, usize, usize, usize, usize)>,
    peak_visible_death_sprites: usize,
    /// Per measured frame: summed GPU pass time (ms) and CPU time of the death
    /// presentation (the fixed tick of the fixture and the sprite sync, in us).
    gpu_ms: Vec<f32>,
    tick_us: Vec<f32>,
    death_sync_us: Vec<f32>,
}

/// Presentation ticks since the simulation stopped at the exit. The sim tick
/// stops there (so state hashes stay stable), but torches and idle frames keep
/// animating from `sim tick + this`. Never read by the simulation.
#[derive(Resource, Default)]
struct AfterExitTicks(u64);

#[derive(Resource)]
struct LiveScenario(Option<Scenario>);

/// Combat fixture state (COMBAT-INT-01); `None` in the normal story run, where
/// `attack` and `ability` stay ignored.
#[derive(Resource)]
struct Combat(Option<CombatState>);

/// DEATH-C: the cosmetic death presenter of a combat fixture (presentation
/// only: nothing in it is read by the simulation).
#[derive(Resource)]
struct Death(Option<DeathPresenter>);

/// The fixture whose script drives the live run (not set for `--manual`).
#[derive(Resource)]
struct LiveFixture(Option<&'static Fixture>);

#[derive(Resource, Default)]
struct Controls {
    jump_tap: bool,
    attack_tap: bool,
    physical_tap: bool,
    restart: bool,
}

#[derive(Resource)]
struct Loading {
    handles: Vec<(String, Handle<Image>)>,
    reported: bool,
    counts: String,
}

#[derive(Component)]
struct Parallax {
    strip: usize,
    copy: usize,
}

#[derive(Component)]
struct Torch {
    rects: Vec<Rect>,
    durations: Vec<u32>,
    offset: usize,
}

#[derive(Component)]
struct ActorVisual {
    is_hero: bool,
}

#[derive(Component)]
struct HeroBox;

/// One goblin, drawn from its variant's atlas (index parity: A, B, A, ...).
#[derive(Component)]
struct GoblinSprite {
    index: usize,
}

/// Carl's second sprite: the melee atlas (punches, kick, hurt, knockdown). It
/// replaces the locomotion sprite while a melee pose is drawn.
#[derive(Component)]
struct CarlMelee;

/// One pooled sprite of the death presenter (DEATH-C).
#[derive(Component, Clone, Copy)]
enum DeathPiece {
    /// The death pose of goblin `index` (replaces its GOB-01 sprite).
    Pose(usize),
    /// A launched part or blood drop (pool slot).
    Part(usize),
    /// A floor decal; `second` is the right cell of a two-cell pool.
    Floor { slot: usize, second: bool },
    /// A back-wall spatter.
    Wall(usize),
}

/// DEATH-D: a sprite of the QA charge `n` of the fixture: the fizzling charge
/// until it detonates, then its blast burst. Presentation only.
#[derive(Component, Clone, Copy)]
enum BlastPiece {
    Charge(usize),
    Burst(usize),
}

/// The loaded `vfx_dynamite` and `vfx-01` atlas images.
#[derive(Resource)]
struct BlastImages {
    dynamite: Handle<Image>,
    burst: Handle<Image>,
}

/// One sprite of the ENV-02 shore (post, block, cavity, crumbs, impact dust).
#[derive(Component, Clone, Copy)]
struct ShorePiece(ShoreSlot);

/// The loaded ENV-02 images, in the order of `ShoreArt::images`.
#[derive(Resource)]
struct ShoreImages(Vec<Handle<Image>>);

/// Geometry the death sprites need (set at startup).
#[derive(Resource)]
struct DeathLook {
    /// The composed sheets of goblin variants A and B: death poses and parts.
    poses_images: [Handle<Image>; DEATH_VARIANTS],
    parts_images: [Handle<Image>; DEATH_VARIANTS],
    pose_spec: ActorSpec,
    /// z of the `backwall_decal` layer.
    wall_z: f32,
}

/// `--debug-combat` overlay boxes (hit and hurt boxes; nothing else is drawn
/// outside the art).
#[derive(Component)]
enum CombatDebugBox {
    /// Carl's active hit box.
    HitBox,
    /// A goblin's 24 x 38 hurt box.
    Goblin(usize),
    /// A goblin's swing box while it is active.
    Swing(usize),
    /// Carl's 24 x 52 hurt box.
    CarlHurt,
}

#[allow(clippy::too_many_arguments)]
fn run_window(
    kit: EnvKit,
    sim: SceneSim,
    anim: SceneAnimators,
    combat: Option<CombatState>,
    death: Option<DeathPresenter>,
    actors: Actors,
    args: Args,
    root: &Path,
) {
    // Bevy 0.20 resolves a relative AssetPlugin path from the binary directory,
    // so give it an absolute directory below the repository root.
    let asset_directory = root.join("generated/art").to_string_lossy().into_owned();
    let window_size = (
        VIRTUAL_WIDTH as u32 * args.scale,
        view_height(args.deck) as u32 * args.scale,
    );
    let capture = args
        .capture_sequence
        .zip(args.capture_dir.clone())
        .map(|((first, step, count), dir)| CaptureSequence::new(first, step, count, dir));
    let freeze = args.freeze
        || args.capture_tick.is_some()
        || args.reference_frame
        || capture.is_some()
        || args.perf_from_tick.is_some();
    let scenario = if freeze { None } else { args.scenario };
    let live_fixture = if freeze || args.manual {
        None
    } else {
        args.fixture
    };
    let exit_after = args.exit_after_ticks;
    let lighting_on = !args.no_lighting && kit.lighting.is_some();
    let reference = args.reference_frame.then_some(RefPose {
        carl_x: REFERENCE_CARL_X,
        donut_x: REFERENCE_DONUT_X,
    });
    let present_mode = if args.perf_frames.is_some() {
        PresentMode::AutoNoVsync
    } else {
        PresentMode::AutoVsync
    };
    let perf = args.perf_frames.map(|target| Perf {
        target,
        warmup: 120,
        seen: 0,
        intervals_ms: Vec::with_capacity(target),
        step_ticks: args.perf_from_tick.is_some(),
        series: Vec::with_capacity(target),
        peak_visible_death_sprites: 0,
        gpu_ms: Vec::with_capacity(target),
        tick_us: Vec::with_capacity(target),
        death_sync_us: Vec::with_capacity(target),
    });
    // AUDIO-01: sound is presentation only. Screenshot, reference and perf runs
    // stay silent and open no device, like `--no-audio` and `FLOORBREAK_AUDIO=off`.
    let mut audio_settings = AudioSettings::from_env();
    if let Some(style) = args.audio_style {
        audio_settings.style = style;
    }
    audio_settings.enabled &= !args.no_audio && !freeze && args.perf_frames.is_none();
    let audio_initial = vec![
        AudioCommand::Music(scene_music(combat.is_some())),
        AudioCommand::Ambience(scene_ambience()),
    ];
    let mut plugins = DefaultPlugins
        .set(ImagePlugin::default_nearest())
        .set(AssetPlugin {
            file_path: asset_directory,
            ..default()
        });
    if !audio_settings.enabled {
        plugins = plugins.disable::<bevy::audio::AudioPlugin>();
    }
    let mut app = App::new();
    app.add_plugins(
        plugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: TITLE.into(),
                resolution: WindowResolution::new(window_size.0, window_size.1)
                    // ADR-0007: window = virtual size x scale in real pixels;
                    // an OS scale factor must not change the integer ratio.
                    .with_scale_factor_override(1.0),
                // ADR-0007: integer scaling only; the window is exactly
                // virtual size x scale (letterboxing is deferred).
                resizable: false,
                present_mode,
                ..default()
            }),
            ..default()
        }),
    )
    .add_plugins(FloorAudioPlugin {
        settings: audio_settings,
        pack_dir: resolve_pack_dir(),
        initial: audio_initial,
    })
    .insert_resource(ClearColor(Color::srgb_u8(6, 5, 12)))
    .insert_resource(Time::<Fixed>::from_hz(60.0))
    .insert_resource(Sim(sim))
    .insert_resource(Anim(anim))
    .insert_resource(actors)
    .insert_resource(Frozen(freeze))
    .insert_resource(ExitAfter(exit_after))
    .insert_resource(LightingOn(lighting_on))
    .insert_resource(ReferencePose(reference))
    .insert_resource(LiveScenario(scenario))
    .insert_resource(Combat(combat))
    .insert_resource(Death(death))
    .insert_resource(LiveFixture(live_fixture))
    .insert_resource(Setup { kit, args })
    .init_resource::<Controls>()
    .init_resource::<AfterExitTicks>()
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            capture_input,
            perf_step,
            sync_visuals,
            sync_combat_visuals,
            sync_shore_visuals,
            sync_blast_visuals,
            sync_death_visuals,
            update_lighting,
            watchdog,
            perf_probe,
            capture_sequence,
        )
            .chain(),
    )
    .add_systems(FixedUpdate, fixed_step);
    if lighting_on {
        app.add_plugins(LightPassPlugin);
    }
    if let Some(perf) = perf {
        // GPU pass timings (Vulkan timestamp queries) for the probe's report.
        app.add_plugins(RenderDiagnosticsPlugin);
        app.insert_resource(perf);
    }
    if let Some(capture) = capture {
        app.insert_resource(capture);
    }
    app.run();
}

// ---------------------------------------------------------------------- setup

fn image_rect(r: IRect) -> Rect {
    Rect::new(
        r.x as f32,
        r.y as f32,
        (r.x + r.w) as f32,
        (r.y + r.h) as f32,
    )
}

fn atlas_sprite(image: &Handle<Image>, rect: IRect) -> Sprite {
    Sprite {
        image: image.clone(),
        rect: Some(image_rect(rect)),
        ..default()
    }
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Option<ResMut<Assets<LightingMaterial>>>,
    lighting_on: Res<LightingOn>,
    setup: Res<Setup>,
    actors: Res<Actors>,
    combat: Res<Combat>,
    death: Res<Death>,
) {
    let kit = &setup.kit;
    let args = &setup.args;
    let height = kit.height;
    let view_h = view_height(args.deck);
    let lighting = kit.lighting.as_ref().filter(|_| lighting_on.0);
    // Render layer of a contract layer: its lighting slot, or the default
    // layer when the pass is off.
    let layers_of = |layer: &str| -> RenderLayers {
        lighting
            .and_then(|l| l.plan.layer_slot.get(layer))
            .map_or_else(RenderLayers::default, |slot| slot_layer(*slot))
    };

    let mut handles: Vec<(String, Handle<Image>)> = Vec::new();
    for (name, asset) in &kit.atlases {
        handles.push((name.clone(), asset_server.load(asset.clone())));
    }
    let atlas = |name: &str| -> Handle<Image> {
        handles
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, h)| h.clone())
            .expect("atlas named by the contract")
    };
    let carl_texture: Handle<Image> = asset_server.load(actors.carl.atlas_asset.clone());
    let donut_texture: Handle<Image> = asset_server.load(actors.donut.atlas_asset.clone());

    if let (Some(lighting), Some(mut materials)) = (lighting, materials) {
        // Lighting on: slot cameras render the pixel layers at virtual
        // resolution, the post stages light them at window resolution.
        let rig = spawn_rig(
            &mut commands,
            &mut images,
            &mut meshes,
            &mut materials,
            lighting,
            UVec2::new(VIRTUAL_WIDTH as u32, view_h as u32),
        );
        commands.insert_resource(rig);
    } else {
        // Lighting off: the COMP-01 camera draws the sprites straight to the
        // window.
        commands.spawn((
            Camera2d,
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::Fixed {
                    width: VIRTUAL_WIDTH as f32,
                    height: view_h as f32,
                },
                ..OrthographicProjection::default_2d()
            }),
            Transform::from_xyz(0.0, 0.0, 0.0),
            SceneCamera,
        ));
    }

    // Tiles: one sprite per non-empty cell, top-left anchored on whole pixels.
    for tile in &kit.tiles {
        commands.spawn((
            atlas_sprite(&atlas(&tile.atlas), tile.rect),
            Anchor::TOP_LEFT,
            Transform::from_xyz(
                (tile.col * kit.tile) as f32,
                to_world_y(height, tile.row * kit.tile),
                tile.z,
            ),
            layers_of(&tile.layer),
        ));
    }

    // Placed pieces (decals, structure, props, foreground).
    for piece in &kit.pieces {
        let mut entity = commands.spawn((
            atlas_sprite(&atlas(&piece.atlas), piece.rect),
            Anchor::TOP_LEFT,
            Transform::from_xyz(piece.x as f32, to_world_y(height, piece.y), piece.z),
            layers_of(&piece.layer),
        ));
        if piece.frames.len() > 1 {
            entity.insert(torch_component(piece));
        }
    }

    // Parallax strips: enough copies to cover the view at any camera offset.
    let mut strip_copies = 0usize;
    for (strip_index, strip) in kit.strips.iter().enumerate() {
        let copies = (VIRTUAL_WIDTH / strip.rect.w + 2) as usize;
        for copy in 0..copies {
            commands.spawn((
                atlas_sprite(&atlas(&strip.atlas), strip.rect),
                Anchor::TOP_LEFT,
                Transform::from_xyz(0.0, to_world_y(height, strip.y), strip.z),
                Parallax {
                    strip: strip_index,
                    copy,
                },
                layers_of(&strip.layer),
            ));
            strip_copies += 1;
        }
    }

    // Actors at the contract's actor draw order; Carl in front of Donut.
    let mut spawn_actor = |assets: &ActorAssets, texture: &Handle<Image>, is_hero: bool, z: f32| {
        // One row of `frame_count` equally sized, unpadded frames.
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(assets.spec.width as u32, assets.spec.height as u32),
            assets.clips.frame_count as u32,
            1,
            None,
            None,
        ));
        commands.spawn((
            ActorVisual { is_hero },
            Sprite::from_atlas_image(
                texture.clone(),
                TextureAtlas {
                    layout,
                    index: assets.clips.clip(Tag::Idle).first,
                },
            ),
            Transform::from_xyz(0.0, 0.0, z),
            layers_of("actors"),
        ));
    };
    spawn_actor(&actors.donut, &donut_texture, false, kit.actor_z);
    spawn_actor(
        &actors.carl,
        &carl_texture,
        true,
        kit.actor_z + CARL_Z_OFFSET,
    );

    // Combat fixture only: Carl's melee sprite, one sprite per goblin (variant A
    // or B by index parity) and the optional debug boxes.
    let mut combat_entities = 0usize;
    let mut combat_handles: Vec<(String, Handle<Image>)> = Vec::new();
    if let (Some(state), Some(art)) = (combat.0.as_ref(), actors.art.as_ref()) {
        let mut layout_of = |clips: &SpriteClips| {
            layouts.add(TextureAtlasLayout::from_grid(
                UVec2::new(clips.spec.width as u32, clips.spec.height as u32),
                clips.frame_count as u32,
                1,
                None,
                None,
            ))
        };
        let carl_melee_texture: Handle<Image> = asset_server.load(art.carl.atlas_asset.clone());
        commands.spawn((
            CarlMelee,
            Sprite::from_atlas_image(
                carl_melee_texture.clone(),
                TextureAtlas {
                    layout: layout_of(&art.carl),
                    index: art.carl.tag("punch_a").0,
                },
            ),
            Transform::from_xyz(0.0, 0.0, kit.actor_z + CARL_Z_OFFSET),
            Visibility::Hidden,
            layers_of("actors"),
        ));
        combat_handles.push((art.carl.atlas_asset.clone(), carl_melee_texture));
        combat_entities += 1;
        let goblin_textures: Vec<(Handle<Image>, Handle<TextureAtlasLayout>)> = art
            .goblins
            .iter()
            .map(|clips| {
                let texture: Handle<Image> = asset_server.load(clips.atlas_asset.clone());
                combat_handles.push((clips.atlas_asset.clone(), texture.clone()));
                (texture, layout_of(clips))
            })
            .collect();
        for index in 0..state.enemies.len() {
            let variant = goblin_variant(index);
            let (texture, layout) = goblin_textures[variant].clone();
            commands.spawn((
                GoblinSprite { index },
                Sprite::from_atlas_image(
                    texture,
                    TextureAtlas {
                        layout,
                        index: art.goblins[variant].tag("idle").0,
                    },
                ),
                Transform::from_xyz(
                    0.0,
                    0.0,
                    kit.actor_z + GOBLIN_Z_OFFSET - index as f32 * GOBLIN_Z_STEP,
                ),
                layers_of("actors"),
            ));
            combat_entities += 1;
        }
        if args.debug_combat {
            let debug_layers = if lighting.is_some() {
                RenderLayers::layer(LAYER_DEBUG)
            } else {
                RenderLayers::default()
            };
            let z = kit.foreground_z + 1.0;
            let mut overlay = |kind: CombatDebugBox, colour: Color, size: Vec2| {
                commands.spawn((
                    kind,
                    Sprite::from_color(colour, size),
                    Transform::from_xyz(0.0, 0.0, z),
                    Visibility::Hidden,
                    debug_layers.clone(),
                ));
                combat_entities += 1;
            };
            overlay(
                CombatDebugBox::HitBox,
                Color::srgba(1.0, 0.1, 0.1, 0.55),
                Vec2::ONE,
            );
            overlay(
                CombatDebugBox::CarlHurt,
                Color::srgba(0.3, 0.5, 1.0, 0.3),
                Vec2::new(HERO_HALF_WIDTH * 2.0, HERO_HALF_HEIGHT * 2.0),
            );
            for index in 0..state.enemies.len() {
                overlay(
                    CombatDebugBox::Goblin(index),
                    Color::srgba(0.2, 1.0, 0.3, 0.3),
                    Vec2::new(ENEMY_HALF_WIDTH * 2.0, ENEMY_HALF_HEIGHT * 2.0),
                );
                overlay(
                    CombatDebugBox::Swing(index),
                    Color::srgba(1.0, 0.6, 0.1, 0.55),
                    Vec2::ONE,
                );
            }
        }
    }

    // DEATH-C: the ENV-02 shore, block and impact dust of a fixture with a
    // shore (presentation only, non-collidable, driven by the simulated state).
    if let (Some(shore_art), Some(_)) = (
        actors.shore.as_ref(),
        combat.0.as_ref().and_then(|c| c.shore.as_ref()),
    ) {
        let images: Vec<Handle<Image>> = shore_art
            .images
            .iter()
            .map(|name| {
                let handle: Handle<Image> = asset_server.load(name.clone());
                combat_handles.push((name.clone(), handle.clone()));
                handle
            })
            .collect();
        for slot in ShoreSlot::ALL {
            let layer = if slot == ShoreSlot::Cavity {
                layers_of("backwall_decal")
            } else {
                layers_of("prop")
            };
            commands.spawn((
                ShorePiece(slot),
                Sprite {
                    image: images[0].clone(),
                    rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, kit.actor_z - 1.0),
                Visibility::Hidden,
                layer,
            ));
            combat_entities += 1;
        }
        commands.insert_resource(ShoreImages(images));
    }

    // DEATH-D: the QA charge (the fizzle loop) and its blast burst, one pair of
    // pre-spawned sprites per charge of the fixture.
    if let (Some(blast_art), Some(state)) = (actors.blast.as_ref(), combat.0.as_ref()) {
        let _ = blast_art;
        let dynamite: Handle<Image> = asset_server.load(DYNAMITE_IMAGE);
        let burst: Handle<Image> = asset_server.load(BURST_IMAGE);
        combat_handles.push((DYNAMITE_IMAGE.to_owned(), dynamite.clone()));
        combat_handles.push((BURST_IMAGE.to_owned(), burst.clone()));
        for index in 0..state.charges.len() {
            for (kind, image) in [
                (BlastPiece::Charge(index), &dynamite),
                (BlastPiece::Burst(index), &burst),
            ] {
                commands.spawn((
                    kind,
                    Sprite {
                        image: image.clone(),
                        rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.0, kit.actor_z - 1.0),
                    Visibility::Hidden,
                    layers_of("actors"),
                ));
                combat_entities += 1;
            }
        }
        commands.insert_resource(BlastImages { dynamite, burst });
    }

    // DEATH-C: the death presenter's sprites, pre-spawned hidden and recycled
    // one to one with its fixed pools (nothing is spawned at runtime).
    if let Some(presenter) = death.0.as_ref() {
        let sheets = presenter.sheets();
        // One image per goblin variant (A, B): the layout is shared (the same
        // frame and cell sizes, checked when the presenter is built).
        let mut image_of = |sheet: &Sheet| -> Handle<Image> {
            images.add(Image::new(
                Extent3d {
                    width: sheet.width() as u32,
                    height: sheet.cell_h as u32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                sheet.rgba.clone(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            ))
        };
        let poses_images: [Handle<Image>; DEATH_VARIANTS] =
            std::array::from_fn(|v| image_of(&presenter.sheets_of(v).poses));
        let parts_images: [Handle<Image>; DEATH_VARIANTS] =
            std::array::from_fn(|v| image_of(&presenter.sheets_of(v).parts));
        let (poses_image, parts_image) = (poses_images[0].clone(), parts_images[0].clone());
        let mut layout_of = |sheet: &Sheet| {
            layouts.add(TextureAtlasLayout::from_grid(
                UVec2::new(sheet.cell_w as u32, sheet.cell_h as u32),
                sheet.frames as u32,
                1,
                None,
                None,
            ))
        };
        let poses_layout = layout_of(&sheets.poses);
        let parts_layout = layout_of(&sheets.parts);
        let sprite_of = |image: &Handle<Image>, layout: &Handle<TextureAtlasLayout>| {
            Sprite::from_atlas_image(
                image.clone(),
                TextureAtlas {
                    layout: layout.clone(),
                    index: 0,
                },
            )
        };
        let mut piece = |kind: DeathPiece, sprite: Sprite, layer: RenderLayers| {
            commands.spawn((
                kind,
                sprite,
                Transform::from_xyz(0.0, 0.0, kit.actor_z - 1.0),
                Visibility::Hidden,
                layer,
            ));
            combat_entities += 1;
        };
        for index in 0..combat.0.as_ref().map_or(0, |c| c.enemies.len()) {
            piece(
                DeathPiece::Pose(index),
                sprite_of(&poses_image, &poses_layout),
                layers_of("actors"),
            );
        }
        for slot in 0..PART_POOL_SLOTS {
            piece(
                DeathPiece::Part(slot),
                sprite_of(&parts_image, &parts_layout),
                layers_of("actors"),
            );
        }
        for slot in 0..FLOOR_DECAL_POOL_SLOTS {
            for second in [false, true] {
                piece(
                    DeathPiece::Floor { slot, second },
                    sprite_of(&parts_image, &parts_layout),
                    layers_of("actors"),
                );
            }
        }
        for slot in 0..WALL_DECAL_POOL_SLOTS {
            piece(
                DeathPiece::Wall(slot),
                sprite_of(&parts_image, &parts_layout),
                layers_of("backwall_decal"),
            );
        }
        let wall_z = kit
            .pieces
            .iter()
            .find(|p| p.layer == "backwall_decal")
            .map_or(3.0, |p| p.z.floor());
        commands.insert_resource(DeathLook {
            poses_images,
            parts_images,
            pose_spec: ActorSpec {
                name: "goblin_death",
                width: presenter.art().pose_frame_size.0,
                height: presenter.art().pose_frame_size.1,
                pivot_x: presenter.art().pose_pivot.0,
                pivot_y: presenter.art().pose_pivot.1,
                shadow_width: 0,
                shadow_pivot_x: 0,
            },
            wall_z,
        });
    }

    // Optional collision overlay: merged solids and walking lines.
    let mut debug_entities = 0usize;
    if args.debug_collision {
        let overlay_z = kit.foreground_z + 1.0;
        let debug_layers = if lighting.is_some() {
            RenderLayers::layer(LAYER_DEBUG)
        } else {
            RenderLayers::default()
        };
        for rect in &kit.solid_rects {
            commands.spawn((
                Sprite::from_color(
                    Color::srgba(0.95, 0.15, 0.2, 0.38),
                    Vec2::new(rect.w as f32, rect.h as f32),
                ),
                Anchor::TOP_LEFT,
                Transform::from_xyz(rect.x as f32, to_world_y(height, rect.y), overlay_z),
                debug_layers.clone(),
            ));
            debug_entities += 1;
        }
        // Walking lines: runs of columns that share a standing surface.
        let mut run: Option<(i32, i32)> = None; // (start x, line y)
        for x in 0..=kit.width {
            let line = if x < kit.width {
                kit.walking_line_at(x)
            } else {
                None
            };
            if let Some((start, y)) = run
                && line != Some(y)
            {
                commands.spawn((
                    Sprite::from_color(
                        Color::srgb(0.2, 1.0, 0.95),
                        Vec2::new((x - start) as f32, 1.0),
                    ),
                    Anchor::TOP_LEFT,
                    Transform::from_xyz(start as f32, to_world_y(height, y), overlay_z + 0.1),
                    debug_layers.clone(),
                ));
                debug_entities += 1;
                run = None;
            }
            if run.is_none()
                && let Some(y) = line
            {
                run = Some((x, y));
            }
        }
        commands.spawn((
            HeroBox,
            Sprite::from_color(
                Color::srgba(0.3, 1.0, 0.3, 0.4),
                Vec2::new(HERO_HALF_WIDTH * 2.0, HERO_HALF_HEIGHT * 2.0),
            ),
            Transform::from_xyz(0.0, 0.0, overlay_z + 0.2),
            debug_layers.clone(),
        ));
        debug_entities += 1;
    }

    handles.push((actors.carl.atlas_asset.clone(), carl_texture));
    handles.push((actors.donut.atlas_asset.clone(), donut_texture));
    handles.extend(combat_handles);
    let tiles = kit.tiles.len();
    let pieces = kit.pieces.len();
    let counts = format!(
        "tiles={tiles} pieces={pieces} parallax_copies={strip_copies} actors=2 debug={debug_entities} sprites={} solids={}",
        tiles + pieces + strip_copies + 2 + debug_entities + combat_entities,
        kit.solid_rects.len()
    );
    println!("FLOOR1_ENTITY_COUNTS {counts}");
    match lighting {
        Some(l) => println!(
            "FLOOR1_LIGHTING on: profile={} slots={} emitters={}",
            l.profile.profile_id,
            l.plan.slots.len(),
            l.emitters.len()
        ),
        None => println!("FLOOR1_LIGHTING off"),
    }
    commands.insert_resource(Loading {
        handles,
        reported: false,
        counts,
    });
}

fn torch_component(piece: &PieceDraw) -> Torch {
    Torch {
        rects: piece.frames.iter().map(|(r, _)| image_rect(*r)).collect(),
        durations: piece.frames.iter().map(|(_, d)| *d).collect(),
        offset: piece.frame_offset,
    }
}

// -------------------------------------------------------------------- systems

fn capture_input(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut controls: ResMut<Controls>,
    mut exit: MessageWriter<AppExit>,
) {
    controls.jump_tap |= keys.just_pressed(KeyCode::Space);
    controls.attack_tap |= keys.just_pressed(KeyCode::KeyJ);
    controls.physical_tap |= keys.just_pressed(KeyCode::KeyK);
    controls.restart |= keys.just_pressed(KeyCode::KeyR);
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
    // The first connected gamepad drives the hero; no pad (hot-unplug) simply
    // means no pad input on the next tick, so nothing sticks.
    if let Some(pad) = pads.iter().next() {
        controls.jump_tap |= pad.just_pressed(GamepadButton::South);
        controls.attack_tap |= pad.just_pressed(GamepadButton::West);
        // PhysicalAttack is Triangle/Y (North); North no longer restarts.
        controls.physical_tap |= pad.just_pressed(GamepadButton::North);
        controls.restart |= pad.just_pressed(GamepadButton::Select);
        if pad.just_pressed(GamepadButton::Start) {
            exit.write(AppExit::Success);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn fixed_step(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    frozen: Res<Frozen>,
    exit_after: Res<ExitAfter>,
    scenario: Res<LiveScenario>,
    fixture: Res<LiveFixture>,
    mut combat: ResMut<Combat>,
    mut death: ResMut<Death>,
    setup: Res<Setup>,
    mut controls: ResMut<Controls>,
    mut sim: ResMut<Sim>,
    mut anim: ResMut<Anim>,
    mut after_exit: ResMut<AfterExitTicks>,
    mut audio: ResMut<AudioQueue>,
    mut exit: MessageWriter<AppExit>,
) {
    let restart = std::mem::take(&mut controls.restart);
    let jump_tap = std::mem::take(&mut controls.jump_tap);
    let attack_tap = std::mem::take(&mut controls.attack_tap);
    let physical_tap = std::mem::take(&mut controls.physical_tap);
    if frozen.0 {
        return;
    }
    if let Some(limit) = exit_after.0
        && elapsed_ticks(&sim.0, combat.0.as_ref()) >= limit
    {
        // Already reported; further fixed steps of this frame do nothing.
        return;
    }
    if restart {
        match combat.0.as_mut() {
            Some(combat) => combat.restart(&mut sim.0),
            None => sim.0.reset(),
        }
        anim.0.reset(&sim.0.world);
        if let Some(combat) = combat.0.as_ref() {
            anim.0.reset_art(combat);
        }
        if let Some(death) = death.0.as_mut() {
            death.clear();
        }
        after_exit.0 = 0;
        println!("FLOOR1_RESTART");
    }
    let raw = if let Some(scenario) = scenario.0 {
        scenario.input(&sim.0.world)
    } else if let Some(fixture) = fixture.0 {
        fixture.input(sim.0.world.tick + 1)
    } else {
        let left = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft);
        let right = keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight);
        let mut raw = RawInput {
            move_x: f32::from(i8::from(right) - i8::from(left)),
            jump: keys.pressed(KeyCode::Space) || jump_tap,
            // Melee exists only in a combat fixture (`--fixture ... --manual`).
            attack: combat.0.is_some() && (keys.pressed(KeyCode::KeyJ) || attack_tap),
            physical: combat.0.is_some() && (keys.pressed(KeyCode::KeyK) || physical_tap),
            ..RawInput::default()
        };
        if let Some(pad) = pads.iter().next() {
            if raw.move_x == 0.0 {
                let stick = pad.left_stick().x;
                raw.move_x = if stick.abs() > 0.18 {
                    stick
                } else {
                    pad.dpad().x
                };
            }
            raw.jump |= pad.pressed(GamepadButton::South);
            if combat.0.is_some() {
                raw.attack |= pad.pressed(GamepadButton::West);
                raw.physical |= pad.pressed(GamepadButton::North);
            }
        }
        raw
    };
    let was_complete = sim.0.world.complete;
    if let Some(combat) = combat.0.as_mut() {
        step_combat(&mut sim.0, combat, &mut anim.0, death.0.as_mut(), raw);
        print_combat_events(combat, Some(&mut audio));
        if setup.args.death_stress
            && let Some(death) = death.0.as_mut()
        {
            // Lab: keep the pools loaded (presentation only).
            let hero = &sim.0.world.hero;
            death.stress_tick(hero.x, hero.y - HERO_HALF_HEIGHT);
        }
    } else {
        sim.0.step(raw);
        anim.0.step(&sim.0.world);
    }
    if sim.0.world.complete && !was_complete {
        println!("FLOOR1_EXIT_REACHED tick={}", sim.0.world.tick);
    }
    if was_complete {
        after_exit.0 += 1;
    }
    if let Some(limit) = exit_after.0
        && elapsed_ticks(&sim.0, combat.0.as_ref()) >= limit
    {
        println!("{}", sim.0.state_line());
        println!("{}", anim.0.state_line(sim.0.world.tick));
        if let Some(combat) = combat.0.as_ref() {
            print_combat_state(&sim.0, &anim.0, combat);
        }
        if let Some(death) = death.0.as_ref() {
            println!("{}", death.state_line());
        }
        exit.write(AppExit::Success);
    }
}

/// Where one actor stands this frame, as drawn (y-up world feet height).
#[derive(Clone, Copy)]
struct ActorPose {
    spec: ActorSpec,
    is_hero: bool,
    foot_x: f32,
    foot_y: f32,
    flip: bool,
    frame: usize,
}

/// View top-left in layout px: the scroll camera plus the combat camera kick
/// (presentation only, kept inside the layout), or the reference view.
fn view_origin(
    state: &SceneSim,
    setup: &Setup,
    reference: &ReferencePose,
    kick: i32,
) -> (i32, i32) {
    if reference.0.is_some() {
        let name = if setup.args.deck {
            REFERENCE_VIEW_DECK
        } else {
            REFERENCE_VIEW
        };
        if let Some(view) = setup.kit.view(name) {
            return (view.x, view.y);
        }
    }
    let widest = (setup.kit.width - VIRTUAL_WIDTH).max(0);
    (
        (state.camera.left() + kick).clamp(0, widest),
        state.camera.top(),
    )
}

/// The camera kick of the combat fixture at the current tick (0 outside one).
fn camera_kick(sim: &SceneSim, combat: &Combat) -> i32 {
    combat
        .0
        .as_ref()
        .map_or(0, |state| state.camera_kick(sim.world.tick))
}

/// Carl and Donut as drawn: the simulation state, or the reference pose.
fn actor_poses(
    state: &SceneSim,
    setup: &Setup,
    reference: &ReferencePose,
    actors: &Actors,
    anim: &SceneAnimators,
) -> [ActorPose; 2] {
    if let Some(pose) = reference.0 {
        // Both stand on the walking line of their column, first idle frame,
        // facing right (the ENV-01 preview's `actor_check`).
        let kit = &setup.kit;
        let at = |spec: ActorSpec, is_hero: bool, x: f32| {
            let line = kit
                .walking_line_at(x as i32)
                .unwrap_or(kit.base_walk_line_y);
            ActorPose {
                spec,
                is_hero,
                foot_x: x,
                foot_y: to_world_y(kit.height, line),
                flip: false,
                // First idle frame (the ENV-01 preview's neutral pose).
                frame: 0,
            }
        };
        return [
            at(actors.carl.spec, true, pose.carl_x),
            at(actors.donut.spec, false, pose.donut_x),
        ];
    }
    let hero = &state.world.hero;
    let donut = &state.world.donut;
    [
        ActorPose {
            spec: actors.carl.spec,
            is_hero: true,
            foot_x: hero.x,
            // The physics foot position, also while airborne: no snapping.
            foot_y: hero.y - HERO_HALF_HEIGHT,
            flip: hero.facing < 0,
            frame: anim.carl.atlas_index(),
        },
        ActorPose {
            spec: actors.donut.spec,
            is_hero: false,
            foot_x: donut.x,
            foot_y: donut.foot_y,
            flip: donut.facing < 0,
            frame: anim.donut.atlas_index(),
        },
    ]
}

/// Presentation tick: torches and light flicker. The reference frame pins it
/// to 0 (the preview's `time_ms = 0`). Actor frames come from [`Anim`].
fn presentation_tick(
    state: &SceneSim,
    after_exit: &AfterExitTicks,
    reference: &ReferencePose,
) -> u64 {
    if reference.0.is_some() {
        0
    } else {
        state.world.tick + after_exit.0
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn sync_visuals(
    sim: Res<Sim>,
    combat: Res<Combat>,
    anim: Res<Anim>,
    actors: Res<Actors>,
    after_exit: Res<AfterExitTicks>,
    setup: Res<Setup>,
    reference: Res<ReferencePose>,
    mut cameras: Query<&mut Transform, With<SceneCamera>>,
    mut parallax: Query<
        (&Parallax, &mut Transform, &mut Visibility),
        (Without<SceneCamera>, Without<ActorVisual>, Without<HeroBox>),
    >,
    mut torches: Query<(&Torch, &mut Sprite), Without<ActorVisual>>,
    mut actor_sprites: Query<
        (&ActorVisual, &mut Transform, &mut Sprite),
        (Without<SceneCamera>, Without<Parallax>, Without<HeroBox>),
    >,
    mut hero_box: Query<
        &mut Transform,
        (
            With<HeroBox>,
            Without<SceneCamera>,
            Without<Parallax>,
            Without<ActorVisual>,
        ),
    >,
) {
    let kit = &setup.kit;
    let state = &sim.0;
    let height = kit.height;
    let view_h = view_height(setup.args.deck);
    let (left, top) = view_origin(state, &setup, &reference, camera_kick(state, &combat));
    let anim_tick = presentation_tick(state, &after_exit, &reference);

    // Every scene camera (the plain one, or the lighting slot cameras and the
    // debug overlay camera) sits on the same whole-pixel position.
    for mut transform in &mut cameras {
        transform.translation.x = (left + VIRTUAL_WIDTH / 2) as f32;
        transform.translation.y = to_world_y(height, top + view_h / 2);
    }

    for (strip_marker, mut transform, mut visibility) in &mut parallax {
        let strip = &kit.strips[strip_marker.strip];
        let start = parallax_start(strip.x_origin, left, strip.factor);
        let copies = parallax_copies(start, strip.rect.w, left, VIRTUAL_WIDTH);
        match copies.get(strip_marker.copy) {
            Some(x) => {
                transform.translation.x = *x as f32;
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }

    for (torch, mut sprite) in &mut torches {
        let frame = animation_frame(&torch.durations, torch.offset, anim_tick);
        sprite.rect = Some(torch.rects[frame]);
    }

    let poses = actor_poses(state, &setup, &reference, &actors, &anim.0);
    for (actor, mut transform, mut sprite) in &mut actor_sprites {
        let pose = poses
            .iter()
            .find(|p| p.is_hero == actor.is_hero)
            .expect("one pose per actor");
        let (cx, cy) = actor_center(pose.spec, pose.foot_x, pose.foot_y, pose.flip);
        transform.translation.x = cx;
        transform.translation.y = cy;
        sprite.flip_x = pose.flip;
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = pose.frame;
        }
    }

    let hero = &state.world.hero;
    for mut transform in &mut hero_box {
        transform.translation.x = hero.x.round();
        transform.translation.y = hero.y.round();
    }
}

/// Combat fixture presentation (reads the simulation and the animators only):
/// the goblin sprites with their frame, flip, depth and impact flash, Carl's
/// melee sprite (shown instead of the locomotion sprite while a melee, hurt or
/// knockdown pose is drawn) and the `--debug-combat` boxes. Donut is shown as
/// the passive follower.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn sync_combat_visuals(
    sim: Res<Sim>,
    combat: Res<Combat>,
    anim: Res<Anim>,
    setup: Res<Setup>,
    mut goblins: Query<
        (&GoblinSprite, &mut Transform, &mut Sprite),
        (
            Without<CarlMelee>,
            Without<ActorVisual>,
            Without<CombatDebugBox>,
        ),
    >,
    mut melee: Query<
        (&mut Transform, &mut Sprite, &mut Visibility),
        (
            With<CarlMelee>,
            Without<GoblinSprite>,
            Without<ActorVisual>,
            Without<CombatDebugBox>,
        ),
    >,
    mut actors: Query<
        (&ActorVisual, &mut Visibility),
        (
            Without<GoblinSprite>,
            Without<CarlMelee>,
            Without<CombatDebugBox>,
        ),
    >,
    mut boxes: Query<
        (&CombatDebugBox, &mut Transform, &mut Visibility),
        (
            Without<GoblinSprite>,
            Without<CarlMelee>,
            Without<ActorVisual>,
        ),
    >,
) {
    let Some(state) = combat.0.as_ref() else {
        return;
    };
    let hero = &sim.0.world.hero;
    let tick = sim.0.world.tick;
    let actor_z = setup.kit.actor_z;
    let carl_active = CombatAnimators::carl_active(&sim.0);

    if let Some(art) = anim.0.art.as_ref() {
        for (goblin_sprite, mut transform, mut sprite) in &mut goblins {
            let index = goblin_sprite.index;
            let goblin = &state.enemies[index];
            let clips = art.goblin_clips(index);
            let pose = art.goblin_pose(index);
            // Airborne frames sit on the bottom of the sim body (no hover, no
            // sink); grounded ones keep the pivot on the physics foot.
            let foot = clips.anchor_foot_y(&pose, goblin.y - ENEMY_HALF_HEIGHT);
            let flip = goblin.facing < 0;
            // Drawn a little toward Carl while he strikes it in hit-stop, and
            // away from a goblin it would stand on (presentation only).
            let draw_x = goblin.x + art.goblin_draw_dx(index) as f32;
            let (cx, cy) = actor_center(clips.spec, draw_x, foot, flip);
            let lying = matches!(goblin.mode, GoblinMode::Down | GoblinMode::Corpse);
            let swinging = matches!(
                goblin.attack_phase(),
                Some(AttackPhase::Smear | AttackPhase::Impact)
            );
            let z_offset = if lying {
                GOBLIN_LYING_Z_OFFSET
            } else if carl_above_goblin(carl_active, swinging) {
                GOBLIN_Z_OFFSET
            } else {
                GOBLIN_SWING_Z_OFFSET
            };
            transform.translation.x = cx;
            transform.translation.y = cy;
            transform.translation.z = actor_z + z_offset - index as f32 * GOBLIN_Z_STEP;
            sprite.flip_x = flip;
            sprite.color = if state.flash(index, tick) {
                FLASH_TINT
            } else {
                Color::WHITE
            };
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = pose.frame;
            }
        }
    }

    // Carl: the melee atlas while a combat pose is drawn, otherwise the
    // locomotion sprite (idle, run, jump, land).
    let carl_pose = anim.0.art.as_ref().and_then(|art| art.carl_pose());
    for (mut transform, mut sprite, mut visibility) in &mut melee {
        match (carl_pose, anim.0.art.as_ref()) {
            (Some(pose), Some(art)) => {
                let foot = art.carl_anchor_foot_y(hero.y - HERO_HALF_HEIGHT);
                let flip = hero.facing < 0;
                let (cx, cy) = actor_center(art.carl.spec, hero.x, foot, flip);
                transform.translation.x = cx;
                transform.translation.y = cy;
                sprite.flip_x = flip;
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = pose.frame;
                }
                *visibility = Visibility::Inherited;
            }
            _ => *visibility = Visibility::Hidden,
        }
    }
    for (actor, mut visibility) in &mut actors {
        *visibility = if actor.is_hero && carl_pose.is_some() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }

    let attacker = attacker_of(hero);
    let hit = hero
        .melee
        .current()
        .filter(|m| m.phase == Phase::Active)
        .and_then(|m| hit_box(&m.spec.shape, &attacker));
    for (kind, mut transform, mut visibility) in &mut boxes {
        let rect = |transform: &mut Transform, rect: dungeon_floorbreak::prototype::Rect| {
            let (cx, cy) = rect.center();
            transform.translation.x = cx.round();
            transform.translation.y = cy.round();
            transform.scale = Vec3::new(rect.width, rect.height, 1.0);
        };
        match kind {
            CombatDebugBox::HitBox => match hit {
                Some(r) => {
                    rect(&mut transform, r);
                    *visibility = Visibility::Inherited;
                }
                None => *visibility = Visibility::Hidden,
            },
            CombatDebugBox::CarlHurt => {
                transform.translation.x = hero.x.round();
                transform.translation.y = hero.y.round();
                *visibility = Visibility::Inherited;
            }
            CombatDebugBox::Goblin(index) => {
                let enemy = &state.enemies[*index];
                transform.translation.x = enemy.x.round();
                transform.translation.y = enemy.y.round();
                *visibility = if enemy.alive() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
            CombatDebugBox::Swing(index) => {
                let enemy = &state.enemies[*index];
                if enemy.attack_phase() == Some(AttackPhase::Impact) {
                    rect(&mut transform, enemy.swing_box());
                    *visibility = Visibility::Inherited;
                } else {
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }
}

/// DEATH-C: the ENV-02 shore and block as the simulation has them (hits, block
/// state, block height, landing). Reads the simulation only.
#[allow(clippy::type_complexity)]
fn sync_shore_visuals(
    combat: Res<Combat>,
    sim: Res<Sim>,
    actors: Res<Actors>,
    setup: Res<Setup>,
    images: Option<Res<ShoreImages>>,
    mut pieces: Query<(&ShorePiece, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (Some(state), Some(art), Some(images)) = (combat.0.as_ref(), actors.shore.as_ref(), images)
    else {
        return;
    };
    let Some(shore) = state.shore.as_ref() else {
        return;
    };
    let tick = sim.0.world.tick;
    let list = art.draw_list(shore, landed_age(&state.events, tick), tick);
    let prop_z = setup.kit.actor_z - 1.0;
    let wall_z = setup
        .kit
        .pieces
        .iter()
        .find(|p| p.layer == "backwall_decal")
        .map_or(3.0, |p| p.z.floor());
    for (piece, mut transform, mut sprite, mut visibility) in &mut pieces {
        match list.iter().find(|d| d.slot == piece.0) {
            Some(draw) => {
                let z = if draw.slot == ShoreSlot::Cavity {
                    wall_z + 0.3
                } else {
                    prop_z + draw.slot.z_bias()
                };
                transform.translation = Vec3::new(draw.centre.0, draw.centre.1, z);
                sprite.image = images.0[draw.image].clone();
                sprite.rect = Some(Rect::new(
                    draw.rect[0] as f32,
                    draw.rect[1] as f32,
                    (draw.rect[0] + draw.rect[2]) as f32,
                    (draw.rect[1] + draw.rect[3]) as f32,
                ));
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

/// DEATH-D: the QA charge as the `dynamite_fizzle` loop from the first tick of
/// the attempt until it detonates, then the VFX-01 `burst` once, centred on the
/// blast centre. Both sit on the blast layer (`actor_z - 0.45`: over the floor
/// decals and the props, under the debris and every actor). Reads the
/// simulation only.
#[allow(clippy::type_complexity)]
fn sync_blast_visuals(
    combat: Res<Combat>,
    sim: Res<Sim>,
    actors: Res<Actors>,
    setup: Res<Setup>,
    images: Option<Res<BlastImages>>,
    mut pieces: Query<(&BlastPiece, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (Some(state), Some(art), Some(images)) = (combat.0.as_ref(), actors.blast.as_ref(), images)
    else {
        return;
    };
    let tick = sim.0.world.tick;
    let actor_z = setup.kit.actor_z;
    let wall_z = setup
        .kit
        .pieces
        .iter()
        .find(|p| p.layer == "backwall_decal")
        .map_or(3.0, |p| p.z.floor());
    let dynamite_spec = ActorSpec {
        name: "qa_charge",
        width: 24,
        height: 24,
        pivot_x: 12,
        pivot_y: 12,
        shadow_width: 0,
        shadow_pivot_x: 0,
    };
    let to_rect = |r: [i32; 4]| {
        Rect::new(
            r[0] as f32,
            r[1] as f32,
            (r[0] + r[2]) as f32,
            (r[1] + r[3]) as f32,
        )
    };
    for (piece, mut transform, mut sprite, mut visibility) in &mut pieces {
        let (index, burst) = match *piece {
            BlastPiece::Charge(i) => (i, false),
            BlastPiece::Burst(i) => (i, true),
        };
        let Some(charge) = state.charges.get(index) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let z = draw_z(DrawLayer::Blast, index, actor_z, wall_z);
        if burst {
            let frame = charge
                .detonated
                .then(|| blast_vfx_frame(tick, charge.detonate_tick))
                .flatten();
            match frame {
                Some(frame) => {
                    let (cx, cy) = (charge.x, charge.foot_y + BLAST_CENTRE_DY);
                    transform.translation = Vec3::new(cx.round(), cy.round(), z);
                    sprite.image = images.burst.clone();
                    sprite.rect = Some(to_rect(art.burst_rect(frame)));
                    *visibility = Visibility::Inherited;
                }
                None => *visibility = Visibility::Hidden,
            }
        } else if charge.detonated {
            *visibility = Visibility::Hidden;
        } else {
            let (cx, cy) = actor_center(dynamite_spec, charge.x, charge.foot_y, false);
            transform.translation = Vec3::new(cx, cy, z);
            sprite.image = images.dynamite.clone();
            sprite.rect = Some(to_rect(art.dynamite_rect(charge_frame(tick))));
            *visibility = Visibility::Inherited;
        }
    }
}

/// DEATH-C: draw the presenter's state. Reads the presenter only; the death
/// pose replaces the goblin's GOB-01 sprite, and every piece sits under the
/// actors (`draw_z`), so nothing is ever drawn over Carl or Donut.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn sync_death_visuals(
    mut perf: Option<ResMut<Perf>>,
    death: Res<Death>,
    look: Option<Res<DeathLook>>,
    combat: Res<Combat>,
    sim: Res<Sim>,
    setup: Res<Setup>,
    mut goblins: Query<(&GoblinSprite, &mut Visibility, &mut Sprite), Without<DeathPiece>>,
    mut pieces: Query<(&DeathPiece, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (Some(presenter), Some(look), Some(state)) = (death.0.as_ref(), look, combat.0.as_ref())
    else {
        return;
    };
    let started = std::time::Instant::now();
    let actor_z = setup.kit.actor_z;
    let tick = sim.0.world.tick;
    let art = presenter.art();
    // A death pose replaces the goblin sprite; a retired corpse is drawn no
    // more and a retiring one fades out (the GOB-01 corpse sprite included).
    // DEATH-D: an exploded goblin (presets 2 to 4) is replaced by its pieces.
    let hide_goblin = |index: usize| {
        presenter.pose_of(index as u32).is_some()
            || presenter.is_retired(index as u32)
            || presenter.hides(index as u32)
    };
    for (goblin, mut visibility, mut sprite) in &mut goblins {
        *visibility = if hide_goblin(goblin.index) {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        sprite.color = sprite
            .color
            .with_alpha(presenter.corpse_alpha(goblin.index as u32));
    }
    // Variant B draws from its own composed sheet (same layout, same frames).
    let use_image = |sprite: &mut Sprite, image: &Handle<Image>| {
        if sprite.image != *image {
            sprite.image = image.clone();
        }
    };
    let set_atlas = |sprite: &mut Sprite, frame: usize| {
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = frame;
        }
    };
    for (piece, mut transform, mut sprite, mut visibility) in &mut pieces {
        match *piece {
            DeathPiece::Pose(index) => match presenter.pose_of(index as u32) {
                Some(pose) => {
                    let (cx, cy) =
                        actor_center(look.pose_spec, pose.foot_x, pose.ground_y, pose.flip);
                    transform.translation = Vec3::new(
                        cx,
                        cy,
                        actor_z + GOBLIN_LYING_Z_OFFSET - index as f32 * GOBLIN_Z_STEP,
                    );
                    sprite.flip_x = pose.flip;
                    use_image(&mut sprite, &look.poses_images[pose.variant]);
                    sprite.color = if state.flash(index, tick) {
                        FLASH_TINT
                    } else {
                        Color::WHITE
                    }
                    .with_alpha(pose.alpha);
                    set_atlas(&mut sprite, pose.frame);
                    *visibility = Visibility::Inherited;
                }
                None => *visibility = Visibility::Hidden,
            },
            DeathPiece::Part(slot) => match presenter.part_at(slot) {
                Some(part) => {
                    transform.translation = Vec3::new(
                        part.x.round(),
                        part.y.round(),
                        draw_z(DrawLayer::Debris, slot, actor_z, look.wall_z),
                    );
                    transform.rotation = Quat::from_rotation_z(part.angle_deg.to_radians());
                    sprite.flip_x = part.flip;
                    use_image(&mut sprite, &look.parts_images[part.variant]);
                    sprite.color = Color::srgba(1.0, 1.0, 1.0, presenter.part_alpha(slot, part));
                    set_atlas(&mut sprite, part.frame);
                    *visibility = Visibility::Inherited;
                }
                None => *visibility = Visibility::Hidden,
            },
            DeathPiece::Floor { slot, second } => {
                let decal = presenter.floor_decal_at(slot);
                let shown = decal.and_then(|d| {
                    let meta = if second { d.pair_meta? } else { d.meta };
                    let offset = if second {
                        art.parts[d.meta].pair_offset.map_or(0.0, |o| o[0] as f32)
                    } else {
                        0.0
                    };
                    Some((d, art.parts[meta].frame, offset))
                });
                match shown {
                    Some((d, frame, offset)) => {
                        transform.translation = Vec3::new(
                            (d.x + offset).round(),
                            d.y.round(),
                            draw_z(DrawLayer::FloorDecal, slot, actor_z, look.wall_z),
                        );
                        sprite.flip_x = d.flip;
                        sprite.color =
                            Color::srgba(1.0, 1.0, 1.0, presenter.floor_decal_alpha(slot, d));
                        set_atlas(&mut sprite, frame);
                        *visibility = Visibility::Inherited;
                    }
                    None => *visibility = Visibility::Hidden,
                }
            }
            DeathPiece::Wall(slot) => match presenter.wall_decal_at(slot) {
                Some(d) => {
                    transform.translation = Vec3::new(
                        d.x.round(),
                        d.y.round(),
                        draw_z(DrawLayer::WallDecal, slot, actor_z, look.wall_z),
                    );
                    sprite.flip_x = d.flip;
                    sprite.color = Color::srgba(1.0, 1.0, 1.0, presenter.wall_decal_alpha(slot, d));
                    set_atlas(&mut sprite, art.parts[d.meta].frame);
                    *visibility = Visibility::Inherited;
                }
                None => *visibility = Visibility::Hidden,
            },
        }
    }
    if let Some(perf) = perf.as_mut()
        && perf.step_ticks
        && perf.seen >= perf.warmup
    {
        perf.death_sync_us
            .push(started.elapsed().as_secs_f32() * 1e6);
    }
}

fn attacker_of(hero: &Hero) -> Attacker {
    Attacker {
        x: hero.x,
        y: hero.y,
        half_height: HERO_HALF_HEIGHT,
        facing: hero.facing,
        player: 0,
    }
}

/// Derive this frame's lights from the view, the presentation tick and the
/// actors' positions, and hand them to the lighting stages. Reads simulation
/// state through shared references only; nothing flows back.
#[allow(clippy::too_many_arguments)]
fn update_lighting(
    rig: Option<Res<LightRig>>,
    sim: Res<Sim>,
    anim: Res<Anim>,
    actors: Res<Actors>,
    after_exit: Res<AfterExitTicks>,
    setup: Res<Setup>,
    reference: Res<ReferencePose>,
    combat: Res<Combat>,
    windows: Query<&Window, With<PrimaryWindow>>,
    materials: Option<ResMut<Assets<LightingMaterial>>>,
    mut dropped_reported: Local<bool>,
) {
    let (Some(rig), Some(mut materials), Ok(window)) = (rig, materials, windows.single()) else {
        return;
    };
    let kit = &setup.kit;
    let Some(scene) = kit.lighting.as_ref() else {
        return;
    };
    let state = &sim.0;
    let view_h = view_height(setup.args.deck);
    let (left, top) = view_origin(state, &setup, &reference, camera_kick(state, &combat));
    let tick = presentation_tick(state, &after_exit, &reference);
    let view = ViewRect {
        left,
        top,
        w: VIRTUAL_WIDTH,
        h: view_h,
    };
    let (gathered, dropped) = scene.gather(kit, view, tick);
    if dropped > 0 && !*dropped_reported {
        println!("FLOOR1_LIGHTING_WARNING: {dropped} lights over the pass limit were dropped");
        *dropped_reported = true;
    }
    let mut shadows: Vec<ShadowEllipse> = Vec::new();
    if let Some(profile) = scene.profile.actor_contact_shadow.as_ref() {
        for pose in actor_poses(state, &setup, &reference, &actors, &anim.0) {
            // The shadow lies on the walking line under the actor (also while
            // it jumps), centred on the footprint of the old idle canvas so the
            // wider locomotion frames do not change the lighting.
            let mut spec = pose.spec;
            if let (true, Some(art)) = (pose.is_hero, anim.0.art.as_ref())
                && let Some(carl_pose) = art.carl_pose()
            {
                // Carl's sprawl (knockdown) casts the shadow of the body on
                // the floor; every other frame keeps the idle footprint.
                let (width, pivot) = art.carl.shadow_footprint(&carl_pose);
                spec.shadow_width = width;
                spec.shadow_pivot_x = pivot;
            }
            let centre_x = shadow_center_x(spec, pose.foot_x, pose.flip);
            let line = kit
                .walking_line_at(pose.foot_x.round() as i32)
                .unwrap_or(kit.height - pose.foot_y.round() as i32);
            shadows.push(contact_shadow(
                profile,
                centre_x,
                line as f32,
                spec.shadow_width as f32,
                view,
            ));
        }
        // Combat fixture: goblins get the same contact shadow on the walking
        // line (also while airborne), shaped by the frame on the floor; Carl's
        // knocked-down sprawl follows the body the same way.
        if let (Some(art), Some(combat)) = (anim.0.art.as_ref(), combat.0.as_ref()) {
            let mut put = |clips: &SpriteClips,
                           pose: &dungeon_floorbreak::combat_anim::ArtPose,
                           x: f32,
                           flip: bool| {
                if shadows.len() >= MAX_SHADOWS {
                    return;
                }
                let (width, pivot) = clips.shadow_footprint(pose);
                let spec = ActorSpec {
                    shadow_width: width,
                    shadow_pivot_x: pivot,
                    ..clips.spec
                };
                let line = kit
                    .walking_line_at(x.round() as i32)
                    .unwrap_or(kit.base_walk_line_y);
                shadows.push(contact_shadow(
                    profile,
                    shadow_center_x(spec, x, flip),
                    line as f32,
                    width as f32,
                    view,
                ));
            };
            for (index, goblin) in combat.enemies.iter().enumerate() {
                put(
                    art.goblin_clips(index),
                    &art.goblin_pose(index),
                    goblin.x + art.goblin_draw_dx(index) as f32,
                    goblin.facing < 0,
                );
            }
        }
    }
    let frame = PassFrame {
        scene,
        gathered: &gathered,
        shadows: &shadows,
        view_w: VIRTUAL_WIDTH as u32,
        view_h: view_h as u32,
        view_top: top,
    };
    let window_px = Vec2::new(
        window.physical_width() as f32,
        window.physical_height() as f32,
    );
    update_rig(&rig, &mut materials, &frame, window_px);
}

/// Frame-interval statistics for `--perf-frames`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct FrameStats {
    frames: usize,
    median_ms: f32,
    p95_ms: f32,
    mean_ms: f32,
    max_ms: f32,
}

fn frame_stats(intervals_ms: &[f32]) -> Option<FrameStats> {
    if intervals_ms.is_empty() {
        return None;
    }
    let mut sorted = intervals_ms.to_vec();
    sorted.sort_by(f32::total_cmp);
    let n = sorted.len();
    // Nearest-rank percentiles.
    let rank = |q: f32| sorted[(((q * n as f32).ceil() as usize).max(1) - 1).min(n - 1)];
    Some(FrameStats {
        frames: n,
        median_ms: rank(0.5),
        p95_ms: rank(0.95),
        mean_ms: sorted.iter().sum::<f32>() / n as f32,
        max_ms: sorted[n - 1],
    })
}

/// GPU time (ms) of the last finished frame: for every timed pass, the sum of
/// all the measurements that share the newest timestamp. Several cameras (the
/// slot cameras, the post-stage quads) record the same pass names in one frame
/// and `Diagnostic::value` keeps only the last of them, so it is summed here.
fn gpu_ms_of_last_frame(diagnostics: &DiagnosticsStore) -> f64 {
    diagnostics
        .iter()
        .filter(|d| d.path().as_str().ends_with("elapsed_gpu"))
        .filter_map(|d| {
            let newest = d.measurement()?.time;
            Some(
                d.measurements()
                    .filter(|m| m.time == newest)
                    .map(|m| m.value)
                    .sum::<f64>(),
            )
        })
        .sum()
}

/// `--perf-from-tick`: after the warm-up, advance the fixture exactly one tick
/// per frame, before the frame is drawn (the frame time then covers the tick's
/// own presentation). Runs ahead of the visuals in the chain.
#[allow(clippy::too_many_arguments)]
fn perf_step(
    perf: Option<ResMut<Perf>>,
    loading: Option<Res<Loading>>,
    setup: Res<Setup>,
    mut sim: ResMut<Sim>,
    mut anim: ResMut<Anim>,
    mut combat: ResMut<Combat>,
    mut death: ResMut<Death>,
) {
    let Some(mut perf) = perf else { return };
    if !perf.step_ticks || loading.is_some_and(|l| !l.reported) || perf.seen < perf.warmup {
        return;
    }
    let (Some(fixture), Some(combat)) = (setup.args.fixture, combat.0.as_mut()) else {
        return;
    };
    let started = std::time::Instant::now();
    let raw = fixture.input(sim.0.world.tick + 1);
    step_combat(&mut sim.0, combat, &mut anim.0, death.0.as_mut(), raw);
    if setup.args.death_stress
        && let Some(death) = death.0.as_mut()
    {
        let hero = &sim.0.world.hero;
        death.stress_tick(hero.x, hero.y - HERO_HALF_HEIGHT);
    }
    perf.tick_us.push(started.elapsed().as_secs_f32() * 1e6);
}

/// Resident and peak resident memory of this process in MiB (Linux
/// `/proc/self/status`; `None` elsewhere).
fn resident_memory_mib() -> Option<(f32, f32)> {
    let text = std::fs::read_to_string("/proc/self/status").ok()?;
    let kib = |key: &str| -> Option<f32> {
        let line = text.lines().find(|l| l.starts_with(key))?;
        line.split_whitespace().nth(1)?.parse::<f32>().ok()
    };
    Some((kib("VmRSS:")? / 1024.0, kib("VmHWM:")? / 1024.0))
}

#[allow(clippy::too_many_arguments)]
fn perf_probe(
    perf: Option<ResMut<Perf>>,
    time: Res<Time<Real>>,
    loading: Option<Res<Loading>>,
    window: Query<&Window, With<PrimaryWindow>>,
    lighting_on: Res<LightingOn>,
    death: Res<Death>,
    sim: Res<Sim>,
    entities: Query<Entity>,
    death_sprites: Query<&Visibility, With<DeathPiece>>,
    diagnostics: Res<DiagnosticsStore>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut perf) = perf else { return };
    // Measure only once the textures are in and the loading report is out.
    if loading.is_some_and(|l| !l.reported) {
        return;
    }
    perf.seen += 1;
    if perf.seen <= perf.warmup {
        return;
    }
    let ms = time.delta_secs() * 1000.0;
    perf.intervals_ms.push(ms);
    perf.gpu_ms.push(gpu_ms_of_last_frame(&diagnostics) as f32);
    if perf.step_ticks {
        let visible = death_sprites
            .iter()
            .filter(|v| **v != Visibility::Hidden)
            .count();
        perf.peak_visible_death_sprites = perf.peak_visible_death_sprites.max(visible);
        let (parts, floor, wall) = death.0.as_ref().map_or((0, 0, 0), |d| {
            (d.live_parts(), d.live_floor_decals(), d.live_wall_decals())
        });
        let tick = sim.0.world.tick;
        perf.series.push((tick, ms, parts, floor, wall, visible));
    }
    if perf.intervals_ms.len() >= perf.target {
        if let Some((rss, peak)) = resident_memory_mib() {
            println!(
                "FLOOR1_PERF_MEM rss_mib={rss:.1} peak_rss_mib={peak:.1} entities={} death_sprites_spawned={} peak_visible_death_sprites={}",
                entities.iter().count(),
                death_sprites.iter().count(),
                perf.peak_visible_death_sprites,
            );
        }
        let mean = |v: &[f32]| (!v.is_empty()).then(|| v.iter().sum::<f32>() / v.len() as f32);
        let gpu = frame_stats(&perf.gpu_ms);
        let tick = frame_stats(&perf.tick_us);
        let sync = frame_stats(&perf.death_sync_us);
        let gpu_paths: Vec<String> = diagnostics
            .iter()
            .filter(|d| d.path().as_str().ends_with("elapsed_gpu"))
            .map(|d| d.path().as_str().to_string())
            .collect();
        println!(
            "FLOOR1_PERF_CPU_GPU gpu_ms_mean={:.3} gpu_ms_p95={:.3} gpu_ms_max={:.3} tick_us_mean={:.1} tick_us_p95={:.1} tick_us_max={:.1} death_sync_us_mean={:.1} death_sync_us_p95={:.1} death_sync_us_max={:.1} gpu_passes={}",
            mean(&perf.gpu_ms).unwrap_or(0.0),
            gpu.map_or(0.0, |s| s.p95_ms),
            gpu.map_or(0.0, |s| s.max_ms),
            mean(&perf.tick_us).unwrap_or(0.0),
            tick.map_or(0.0, |s| s.p95_ms),
            tick.map_or(0.0, |s| s.max_ms),
            mean(&perf.death_sync_us).unwrap_or(0.0),
            sync.map_or(0.0, |s| s.p95_ms),
            sync.map_or(0.0, |s| s.max_ms),
            gpu_paths.len(),
        );
        println!("FLOOR1_PERF_GPU_PASSES {}", gpu_paths.join(" "));
        for (tick, ms, parts, floor, wall, visible) in &perf.series {
            println!(
                "FLOOR1_PERF_TICK tick={tick} ms={ms:.2} parts={parts} floor_decals={floor} wall_decals={wall} visible_death_sprites={visible}"
            );
        }
        if let Some(stats) = frame_stats(&perf.intervals_ms) {
            let size = window
                .single()
                .map(|w| format!("{}x{}", w.physical_width(), w.physical_height()))
                .unwrap_or_default();
            println!(
                "FLOOR1_PERF frames={} median_ms={:.2} p95_ms={:.2} mean_ms={:.2} max_ms={:.2} lighting={} window={size} present=AutoNoVsync warmup_frames={}",
                stats.frames,
                stats.median_ms,
                stats.p95_ms,
                stats.mean_ms,
                stats.max_ms,
                if lighting_on.0 { "on" } else { "off" },
                perf.warmup,
            );
            if let Some(death) = death.0.as_ref() {
                // The pools at the end of the run (loaded by `--death-stress`).
                println!(
                    "{} part_capacity={} floor_decal_capacity={} wall_decal_capacity={} recycled={:?}",
                    death.state_line(),
                    PART_POOL_CAPACITY,
                    FLOOR_DECAL_POOL_CAPACITY,
                    WALL_DECAL_POOL_CAPACITY,
                    death.recycled(),
                );
            }
        }
        exit.write(AppExit::Success);
    }
}

#[allow(clippy::too_many_arguments)]
fn watchdog(
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    time: Res<Time<Real>>,
    sim: Res<Sim>,
    anim: Res<Anim>,
    mut combat: ResMut<Combat>,
    loading: Option<ResMut<Loading>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut loading) = loading else { return };
    if loading.reported {
        return;
    }
    for (name, handle) in &loading.handles {
        if let LoadState::Failed(error) = asset_server.load_state(handle) {
            println!("FLOOR1_ASSET_FAILURE: {name}: {error}");
            exit.write(AppExit::error());
            loading.reported = true;
            return;
        }
    }
    if loading
        .handles
        .iter()
        .all(|(_, handle)| images.get(handle).is_some())
    {
        println!(
            "FLOOR1_TEXTURES_READY: {} atlases loaded; {}",
            loading.handles.len(),
            loading.counts
        );
        if let Some(combat) = combat.0.as_mut() {
            print_combat_events(combat, None);
        }
        println!("{}", sim.0.state_line());
        println!("{}", anim.0.state_line(sim.0.world.tick));
        if let Some(combat) = combat.0.as_ref() {
            print_combat_state(&sim.0, &anim.0, combat);
        }
        loading.reported = true;
    } else if time.elapsed_secs() > WATCHDOG_SECONDS {
        println!("FLOOR1_ASSET_FAILURE: textures not loaded within {WATCHDOG_SECONDS} s");
        exit.write(AppExit::error());
        loading.reported = true;
    }
}

// ------------------------------------------------------------ capture sequence

/// Render frames to wait after a state change before the screenshot request,
/// and the most frames to wait for the file to appear.
const CAPTURE_SETTLE_FRAMES: u32 = 4;
const CAPTURE_TIMEOUT_FRAMES: u32 = 900;
/// Frames the saved file must keep its size before the next step.
const CAPTURE_STABLE_FRAMES: u32 = 2;

#[derive(Debug, Default)]
struct PendingFile {
    waited: u32,
    last_len: u64,
    stable: u32,
}

#[derive(Debug)]
enum CapturePhase {
    /// Textures not loaded yet.
    WaitReady,
    /// Frames left before the next screenshot request.
    Settle(u32),
    /// Requested; waiting for the file, with the frames waited so far.
    Pending(PathBuf, PendingFile),
}

/// `--capture-sequence`: step the scenario by hand in the real render loop and
/// save a window screenshot every `step` ticks (the fixed step is frozen).
#[derive(Resource)]
struct CaptureSequence {
    step: u64,
    count: u64,
    dir: PathBuf,
    index: u64,
    phase: CapturePhase,
}

impl CaptureSequence {
    fn new(_first: u64, step: u64, count: u64, dir: PathBuf) -> Self {
        Self {
            step,
            count,
            dir,
            index: 0,
            phase: CapturePhase::WaitReady,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn capture_sequence(
    mut commands: Commands,
    seq: Option<ResMut<CaptureSequence>>,
    loading: Option<Res<Loading>>,
    setup: Res<Setup>,
    mut sim: ResMut<Sim>,
    mut anim: ResMut<Anim>,
    mut combat: ResMut<Combat>,
    mut death: ResMut<Death>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut seq) = seq else { return };
    let (scenario, fixture) = (setup.args.scenario, setup.args.fixture);
    if scenario.is_none() && fixture.is_none() {
        return;
    }
    let fail = |exit: &mut MessageWriter<AppExit>, message: &str| {
        println!("FLOOR1_CAPTURE_FAILURE: {message}");
        exit.write(AppExit::error());
    };
    let phase = std::mem::replace(&mut seq.phase, CapturePhase::WaitReady);
    seq.phase = match phase {
        CapturePhase::WaitReady if loading.is_some_and(|l| l.reported) => {
            CapturePhase::Settle(CAPTURE_SETTLE_FRAMES)
        }
        CapturePhase::WaitReady => CapturePhase::WaitReady,
        CapturePhase::Settle(frames) if frames > 0 => CapturePhase::Settle(frames - 1),
        CapturePhase::Settle(_) => {
            if let Err(error) = std::fs::create_dir_all(&seq.dir) {
                fail(&mut exit, &format!("{}: {error}", seq.dir.display()));
                return;
            }
            let path = seq.dir.join(format!(
                "frame_{:04}_tick_{:05}.png",
                seq.index, sim.0.world.tick
            ));
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path.clone()));
            CapturePhase::Pending(path, PendingFile::default())
        }
        CapturePhase::Pending(path, mut file) => {
            // The file must exist, be non-empty and keep the same size for
            // CAPTURE_STABLE_FRAMES frames, so a still-growing PNG is not read.
            let len = std::fs::metadata(&path).map_or(0, |m| m.len());
            if len > 0 && len == file.last_len {
                file.stable += 1;
            } else {
                file.stable = 0;
            }
            file.last_len = len;
            if file.stable < CAPTURE_STABLE_FRAMES {
                file.waited += 1;
                if file.waited > CAPTURE_TIMEOUT_FRAMES {
                    fail(
                        &mut exit,
                        &format!("screenshot {} was never written", path.display()),
                    );
                    return;
                }
                seq.phase = CapturePhase::Pending(path, file);
                return;
            }
            println!(
                "FLOOR1_CAPTURE index={} tick={} file={}",
                seq.index,
                sim.0.world.tick,
                path.display()
            );
            if let Some(combat) = combat.0.as_mut() {
                print_combat_events(combat, None);
            }
            println!("{}", sim.0.state_line());
            println!("{}", anim.0.state_line(sim.0.world.tick));
            if let Some(combat) = combat.0.as_ref() {
                print_combat_state(&sim.0, &anim.0, combat);
            }
            seq.index += 1;
            if seq.index >= seq.count {
                println!("FLOOR1_CAPTURE_SEQUENCE_DONE count={}", seq.count);
                exit.write(AppExit::Success);
                return;
            }
            for _ in 0..seq.step {
                if let (Some(fixture), Some(combat)) = (fixture, combat.0.as_mut()) {
                    let raw = fixture.input(sim.0.world.tick + 1);
                    step_combat(&mut sim.0, combat, &mut anim.0, death.0.as_mut(), raw);
                } else if let Some(scenario) = scenario {
                    let raw = scenario.input(&sim.0.world);
                    sim.0.step(raw);
                    anim.0.step(&sim.0.world);
                }
            }
            CapturePhase::Settle(CAPTURE_SETTLE_FRAMES)
        }
    };
}

// ------------------------------------------------------------ headless smoke

fn headless_smoke(kit: &EnvKit, args: &Args, actors: &Actors, root: &Path) -> Result<(), String> {
    if kit.solid_rects.is_empty() || kit.tiles.is_empty() {
        return Err("layout produced no tiles or no solids".into());
    }
    // Idle: Carl stays grounded on the walking line.
    let mut idle = SceneSim::new(kit, 360)?;
    idle.run_scenario(Scenario::Idle, 600);
    if !idle.world.hero.grounded || idle.world.hero.x != 72.0 {
        return Err("idle hero left the spawn".into());
    }
    // Scripted run: the same replay on both views reaches the exit.
    let mut desktop = SceneSim::new(kit, 360)?;
    let mut deck = SceneSim::new(kit, 400)?;
    let mut ticks = 0u64;
    while !desktop.world.complete {
        if ticks >= 2400 {
            return Err("walk_right did not reach the exit in 2400 ticks".into());
        }
        let raw = Scenario::WalkRight.input(&desktop.world);
        desktop.step(raw);
        let raw = Scenario::WalkRight.input(&deck.world);
        deck.step(raw);
        ticks += 1;
    }
    if desktop.world.state_hash() != deck.world.state_hash() {
        return Err("640x360 and 640x400 simulations diverged".into());
    }
    println!(
        "FLOOR1_HEADLESS_OK: {} tiles, {} pieces, {} merged solids, exit reached in {ticks} ticks",
        kit.tiles.len(),
        kit.pieces.len(),
        kit.solid_rects.len()
    );
    if let Some(fixture) = args.fixture {
        // Fixture replay: the scripted run for `--capture-tick` ticks (default
        // the fixture's own length), every event, then the state lines.
        let mut args = args.clone();
        args.capture_tick.get_or_insert(fixture.ticks);
        let (sim, anim, combat, death) = build_scene(kit, &args, actors, root)?;
        let mut combat = combat.ok_or("fixture without combat state")?;
        print_combat_events(&mut combat, None);
        println!("{}", sim.state_line());
        println!("{}", anim.state_line(sim.world.tick));
        print_combat_state(&sim, &anim, &combat);
        if args.death_trace
            && let Some(death) = death
        {
            println!("{}", death.state_line());
        }
        return Ok(());
    }
    let (sim, anim, _, _) = build_scene(kit, args, actors, root)?;
    println!("{}", sim.state_line());
    println!("{}", anim.state_line(sim.world.tick));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Result<Args, String> {
        let mut full = vec!["floor01_scene".to_owned()];
        full.extend(list.iter().map(|s| (*s).to_owned()));
        parse_args(&full)
    }

    #[test]
    fn the_gore_preset_flag_defaults_to_full_and_is_strict() {
        assert_eq!(args(&[]).unwrap().gore_preset, GorePreset::Full);
        for (value, preset) in [
            ("1", GorePreset::Off),
            ("2", GorePreset::Reduced),
            ("3", GorePreset::Medium),
            ("4", GorePreset::Full),
        ] {
            let parsed = args(&["--fixture", "cp03_shore_kill", "--gore-preset", value]).unwrap();
            assert_eq!(parsed.gore_preset, preset);
        }
        for bad in ["0", "5", "gory", ""] {
            assert!(args(&["--gore-preset", bad]).is_err(), "{bad:?}");
        }
        assert!(args(&["--gore-preset"]).is_err());
        // The lab flags need a fixture; stress is live only; the trace is headless only.
        assert!(args(&["--death-stress"]).is_err());
        // DEATH-D: the decapitation lab flag is gone (it is an unknown argument).
        assert!(args(&["--death-lab-finish", "decapitated"]).is_err());
        assert!(
            args(&[
                "--fixture",
                "cp03_shore_kill",
                "--death-lab-finish",
                "decapitated"
            ])
            .is_err()
        );
        assert!(args(&["--fixture", "cp03_shore_kill", "--death-trace"]).is_err());
        assert!(
            args(&[
                "--fixture",
                "cp03_shore_kill",
                "--headless-floor01-smoke",
                "--death-trace"
            ])
            .is_ok()
        );
        assert!(
            args(&[
                "--fixture",
                "cp03_shore_kill",
                "--headless-floor01-smoke",
                "--death-stress"
            ])
            .is_err()
        );
    }

    #[test]
    fn the_death_layers_draw_under_every_scene_actor() {
        // The scene's own actor offsets: Carl +0.1, Donut 0, a goblin
        // +0.05 / +0.15 (swinging), a lying goblin -0.01 minus 0.001 per index
        // (64 goblins is far more than any fixture).
        let lowest_actor = 7.0 + GOBLIN_LYING_Z_OFFSET - GOBLIN_Z_STEP * 64.0;
        let slots = PART_POOL_SLOTS.max(FLOOR_DECAL_POOL_SLOTS);
        for slot in [0, slots - 1] {
            for layer in [DrawLayer::Debris, DrawLayer::FloorDecal, DrawLayer::Blast] {
                assert!(draw_z(layer, slot, 7.0, 3.0) < lowest_actor, "{layer:?}");
            }
        }
        // DEATH-D: the charge and the burst sit between the floor decals and
        // the debris (every charge slot is below the first debris piece).
        for slot in 0..8 {
            let blast = draw_z(DrawLayer::Blast, slot, 7.0, 3.0);
            assert!(blast > draw_z(DrawLayer::FloorDecal, slots - 1, 7.0, 3.0));
            assert!(blast < draw_z(DrawLayer::Debris, 0, 7.0, 3.0));
        }
        assert!(draw_z(DrawLayer::WallDecal, WALL_DECAL_POOL_SLOTS, 7.0, 3.0) < 4.0);
    }

    #[test]
    fn lighting_flags_parse() {
        let a = args(&["--no-lighting", "--scale", "3"]).unwrap();
        assert!(a.no_lighting && a.scale == 3);
        let a = args(&["--reference-frame", "--perf-frames", "300"]).unwrap();
        assert!(a.reference_frame && a.perf_frames == Some(300));
        assert!(!args(&[]).unwrap().no_lighting);
        assert!(args(&["--perf-frames", "3"]).is_err());
        assert!(args(&["--perf-frames"]).is_err());
        assert!(args(&["--reference-frame", "--scenario", "idle"]).is_err());
    }

    #[test]
    fn exit_after_ticks_needs_a_live_scenario() {
        let a = args(&["--scenario", "walk_right", "--exit-after-ticks", "250"]).unwrap();
        assert_eq!(a.exit_after_ticks, Some(250));
        assert!(args(&["--exit-after-ticks", "250"]).is_err());
        assert!(args(&["--scenario", "idle", "--exit-after-ticks", "0"]).is_err());
        assert!(
            args(&[
                "--scenario",
                "idle",
                "--capture-tick",
                "5",
                "--exit-after-ticks",
                "9"
            ])
            .is_err()
        );
        assert!(args(&["--scenario", "idle", "--freeze", "--exit-after-ticks", "9"]).is_err());
    }

    #[test]
    fn capture_sequence_flags_parse_and_validate() {
        let a = args(&[
            "--scenario",
            "run_jump",
            "--capture-sequence",
            "142:3:81",
            "--capture-dir",
            "frames",
        ])
        .unwrap();
        assert_eq!(a.capture_sequence, Some((142, 3, 81)));
        assert_eq!(a.capture_dir, Some(PathBuf::from("frames")));
        assert_eq!(a.scenario, Some(Scenario::RunJump));
        let bad = |list: &[&str]| args(list).is_err();
        // Needs both flags and a scenario.
        assert!(bad(&[
            "--scenario",
            "run_jump",
            "--capture-sequence",
            "0:3:5"
        ]));
        assert!(bad(&["--capture-sequence", "0:3:5", "--capture-dir", "d"]));
        // Malformed or degenerate sequences.
        for sequence in ["0:3", "0:0:5", "0:3:0", "0:3:5:7", "a:b:c", "0:3:1001"] {
            assert!(
                bad(&[
                    "--scenario",
                    "run_jump",
                    "--capture-sequence",
                    sequence,
                    "--capture-dir",
                    "d"
                ]),
                "{sequence}"
            );
        }
        // Mutually exclusive with the frozen and live modes.
        for extra in [
            ["--capture-tick", "5"],
            ["--exit-after-ticks", "9"],
            ["--freeze", ""],
        ] {
            let mut list = vec![
                "--scenario",
                "run_jump",
                "--capture-sequence",
                "0:3:5",
                "--capture-dir",
                "d",
            ];
            list.extend(extra.iter().filter(|s| !s.is_empty()));
            assert!(bad(&list), "{extra:?}");
        }
    }

    #[test]
    fn the_depth_order_follows_the_melee_integration_rule() {
        // Lying goblins below Donut (z 0), then Donut, then standing goblins
        // (the earlier goblin in front of a stacked later one), then Carl (his
        // fist must not be hidden by a goblin's head), then a goblin in its
        // smear or impact (its club lands on him).
        const { assert!(GOBLIN_LYING_Z_OFFSET + 8.0 * GOBLIN_Z_STEP < 0.0) };
        const { assert!(0.0 < GOBLIN_Z_OFFSET - 8.0 * GOBLIN_Z_STEP) };
        const { assert!(GOBLIN_Z_OFFSET < CARL_Z_OFFSET) };
        const { assert!(CARL_Z_OFFSET < GOBLIN_SWING_Z_OFFSET) };
        // Which side wins is the library's rule.
        assert!(carl_above_goblin(true, true));
        assert!(!carl_above_goblin(false, true));
        assert!(carl_above_goblin(false, false));
    }

    #[test]
    fn combat_trace_is_headless_fixture_only() {
        let a = args(&[
            "--headless-floor01-smoke",
            "--fixture",
            "cp01_two_hits",
            "--combat-trace",
        ])
        .unwrap();
        assert!(a.combat_trace);
        assert!(args(&["--combat-trace"]).is_err());
        assert!(args(&["--fixture", "cp01_two_hits", "--combat-trace"]).is_err());
    }

    #[test]
    fn fixture_flags_parse_and_validate() {
        let a = args(&["--fixture", "cp01_single_goblin", "--debug-combat"]).unwrap();
        assert_eq!(a.fixture.map(|f| f.name), Some("cp01_single_goblin"));
        assert!(a.debug_combat && !a.manual);
        assert!(args(&["--fixture", "cp01_single_goblin", "--manual"]).is_ok());
        assert!(args(&["--fixture", "nope"]).is_err());
        assert!(args(&["--fixture"]).is_err());
        // Needs a fixture; exclusive with a scenario; manual is a live run.
        assert!(args(&["--manual"]).is_err());
        assert!(args(&["--debug-combat"]).is_err());
        assert!(args(&["--fixture", "cp01_two_hits", "--scenario", "idle"]).is_err());
        assert!(
            args(&[
                "--fixture",
                "cp01_two_hits",
                "--manual",
                "--exit-after-ticks",
                "9"
            ])
            .is_err()
        );
        assert!(args(&["--fixture", "cp01_two_hits", "--capture-tick", "9"]).is_ok());
        assert!(args(&["--fixture", "cp01_two_hits", "--exit-after-ticks", "9"]).is_ok());
        assert!(args(&["--fixture", "cp01_two_hits", "--reference-frame"]).is_err());
    }

    #[test]
    fn anim_trace_is_headless_only() {
        let a = args(&[
            "--headless-floor01-smoke",
            "--scenario",
            "run_jump",
            "--anim-trace",
        ])
        .unwrap();
        assert!(a.anim_trace);
        assert!(args(&["--scenario", "run_jump", "--anim-trace"]).is_err());
        assert!(args(&["--headless-floor01-smoke", "--anim-trace"]).is_err());
    }

    #[test]
    fn presim_covers_the_capture_tick_and_the_sequence_start() {
        let a = args(&["--scenario", "idle", "--capture-tick", "7"]).unwrap();
        assert_eq!(presim_ticks(&a), Some(7));
        let a = args(&[
            "--scenario",
            "idle",
            "--capture-sequence",
            "142:3:2",
            "--capture-dir",
            "d",
        ])
        .unwrap();
        assert_eq!(presim_ticks(&a), Some(142));
        assert_eq!(presim_ticks(&args(&[]).unwrap()), None);
    }

    #[test]
    fn frame_stats_use_nearest_rank_percentiles() {
        assert_eq!(frame_stats(&[]), None);
        let values: Vec<f32> = (1..=100).map(|v| v as f32).collect();
        let s = frame_stats(&values).unwrap();
        assert_eq!(
            (s.frames, s.median_ms, s.p95_ms, s.max_ms),
            (100, 50.0, 95.0, 100.0)
        );
        assert_eq!(s.mean_ms, 50.5);
    }

    #[test]
    fn reference_pose_matches_the_layout_preview() {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(LAYOUT_PATH),
        )
        .unwrap();
        let layout: serde_json::Value = serde_json::from_str(&text).unwrap();
        let check = layout["preview"]["actor_check"].as_array().unwrap();
        assert_eq!(check[0]["x"].as_f64().unwrap() as f32, REFERENCE_CARL_X);
        assert_eq!(check[1]["x"].as_f64().unwrap() as f32, REFERENCE_DONUT_X);
        assert_eq!(layout["preview"]["primary_view"], REFERENCE_VIEW);
        assert!(layout["views"][REFERENCE_VIEW_DECK].is_array());
    }

    #[test]
    fn flags_parse() {
        let a = args(&[
            "--deck",
            "--scenario",
            "walk_right",
            "--capture-tick",
            "420",
            "--debug-collision",
        ])
        .unwrap();
        assert!(a.deck && a.debug_collision);
        assert_eq!(a.capture_tick, Some(420));
        assert_eq!(a.scenario, Some(Scenario::WalkRight));
        assert!(args(&["--capture-tick", "5"]).is_err());
        assert!(args(&["--scenario", "fly"]).is_err());
        assert!(args(&["--bogus"]).is_err());
        assert_eq!(args(&[]).unwrap().scale, 2);
    }
}
