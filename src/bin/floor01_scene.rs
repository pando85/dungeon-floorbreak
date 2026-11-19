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
    camera::{RenderTarget, ScalingMode, Viewport, visibility::RenderLayers},
    diagnostic::DiagnosticsStore,
    input::gamepad::{GamepadConnection, GamepadEvent},
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
    camera_lab::camera_op::{Aspect, Bounds, View},
    combat::{Attacker, ENEMY_HALF_HEIGHT, ENEMY_HALF_WIDTH, Phase, hit_box},
    combat_anim::{
        CombatAnimators, GOBLIN_VARIANTS, SpriteClips, carl_above_goblin, goblin_variant,
    },
    combat_scene::{CombatState, Fixture},
    corridor::{CorridorWorld, Scenario, SceneSim},
    death_present::{
        DeathPresenter, DrawLayer, FLOOR_DECAL_POOL_CAPACITY, FLOOR_DECAL_POOL_SLOTS, Finish,
        PART_POOL_CAPACITY, PART_POOL_SLOTS, WALL_DECAL_POOL_CAPACITY, WALL_DECAL_POOL_SLOTS,
        draw_z,
    },
    demo::{self, SelectAction, SelectControl},
    env_kit::{
        EnvKit, IRect, PieceDraw, animation_frame, parallax_copies, parallax_start, to_world_y,
    },
    feel::{HERO_HURT_TICKS, IMPACT_FLASH_TICKS},
    goblin::{AttackPhase, GoblinMode},
    gore::{DEATH_VARIANTS, DeathArt, GorePreset, Sheet},
    hud::{GlyphSet, HudConfig, HudLayout, HudPreview, HudRuntime},
    hud_adapter::{HudPlugin, HudState},
    hud_settings::HudSettings,
    input::RawInput,
    light_pass::{
        LAYER_DEBUG, LightPassPlugin, LightRig, LightingMaterial, PassFrame, SceneCamera,
        slot_layer, spawn_rig, update_rig,
    },
    lighting::{MAX_SHADOWS, ShadowEllipse, ViewRect, contact_shadow},
    pad_slots::{PadButtons, PadEvent, PadFrame, PadSlots},
    party::{HeroId, PadId, PartyState},
    party_scene::{self, PartyFixture},
    prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Hero},
    scene_anim::SceneAnimators,
    scene_camera::{
        CameraChoice, CameraInterest, RigConfig, SceneCameraRig, synthetic_interest, test_room_kit,
    },
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
    /// WORK-005/006 party fixture (headless replay or a `--capture-tick`
    /// capture): its base combat fixture is also stored in `fixture`.
    party_fixture: Option<&'static PartyFixture>,
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
    /// Fixture only, lab: play this finish for ordinary melee defeats (the
    /// decapitation has no authoritative cause before PR D).
    death_lab_finish: Option<Finish>,
    /// Headless fixture only: print the presenter's `DEATH_PRESENT` summary.
    death_trace: bool,
    /// HUD-HYBRID-A-01: draw a labelled HUD preview fixture. Presentation
    /// only: it never reaches the simulation, the combat state or a replay.
    hud_preview: Option<HudPreview>,
    /// HUD glyph set override (`--glyphs=xbox|dualsense`). `None` follows the
    /// last-used gamepad (Xbox until one is seen).
    glyphs: Option<GlyphSet>,
    /// HUD-DEBT-06: reduced UI motion and high contrast. Environment first
    /// (`FLOORBREAK_HUD_*`), then `--hud-reduced-motion[=on|off]` and
    /// `--hud-high-contrast[=on|off]`. Presentation only.
    hud_settings: HudSettings,
    /// DEMO-01 PR C: the presentation camera (`auto`, `scroll`, `1.0`, `1.25`,
    /// `1.5`). `None` is the default of the run (`scroll` outside `--demo`).
    camera: Option<CameraChoice>,
    /// Evidence: run on room 1 to 6 of the camera lab (a tall greybox
    /// composition from the approved kit) instead of the 400 px corridor.
    camera_test_room: Option<usize>,
    /// Evidence: a synthetic foe that has taken aim, `X:FIRST:LAST` (world
    /// column, first and last elapsed tick). It only feeds the camera.
    camera_test_threat: Option<(f32, u64, u64)>,
    /// Headless: print one `FLOOR1_VIEW` line per tick of the presentation view.
    view_trace: bool,
    /// DEMO-01: the continuous demo route on the demo hall (`--demo`).
    demo: bool,
    /// DEMO-01: start at this beat's checkpoint (`--demo-beat N`, 0 to 6).
    demo_beat: usize,
    /// DEMO-01: the deterministic autopilot drives Carl (probes, evidence).
    autopilot: bool,
}

impl Args {
    fn hud_config(&self) -> HudConfig {
        HudConfig {
            preview: self.hud_preview,
            glyphs: self.glyphs,
            settings: self.hud_settings,
        }
    }

    /// The camera of this run: `--camera`, else `auto` for `--demo` and the
    /// scroll camera for everything else (every fixture, scenario and capture).
    fn camera_choice(&self) -> CameraChoice {
        if self.reference_frame {
            return CameraChoice::Scroll;
        }
        self.camera
            .unwrap_or_else(|| CameraChoice::default_for(self.demo))
    }
}

fn parse_hud_preview(value: &str) -> Result<HudPreview, String> {
    HudPreview::parse(value).ok_or_else(|| {
        format!(
            "unknown HUD preview {value:?} (expected {})",
            HudPreview::names()
        )
    })
}

fn parse_glyphs(value: &str) -> Result<GlyphSet, String> {
    GlyphSet::parse(value)
        .ok_or_else(|| format!("unknown glyph set {value:?} (expected xbox, dualsense)"))
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
    parse_args_with(args, HudSettings::from_env())
}

/// `parse_args` with the HUD settings the environment supplied (a parameter so
/// tests do not depend on the process environment).
fn parse_args_with(args: &[String], hud_settings: HudSettings) -> Result<Args, String> {
    let mut parsed = Args {
        deck: false,
        headless: false,
        freeze: false,
        debug_collision: false,
        scenario: None,
        fixture: None,
        party_fixture: None,
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
        death_lab_finish: None,
        death_trace: false,
        hud_preview: None,
        glyphs: None,
        hud_settings,
        camera: None,
        camera_test_room: None,
        camera_test_threat: None,
        view_trace: false,
        demo: false,
        demo_beat: 0,
        autopilot: false,
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
                if parsed.party_fixture.is_some() {
                    return Err("--fixture and --party-fixture are mutually exclusive".into());
                }
                parsed.fixture = Some(Fixture::by_name(value).ok_or_else(|| {
                    format!("unknown fixture {value:?} (expected {})", Fixture::names())
                })?);
            }
            "--party-fixture" => {
                let value = iter.next().ok_or("--party-fixture needs a name")?;
                if parsed.fixture.is_some() {
                    return Err("--fixture and --party-fixture are mutually exclusive".into());
                }
                let party = PartyFixture::by_name(value).ok_or_else(|| {
                    format!(
                        "unknown party fixture {value:?} (expected {})",
                        PartyFixture::names()
                    )
                })?;
                parsed.party_fixture = Some(party);
                parsed.fixture = Some(party.base);
            }
            "--hud-preview" => {
                let value = iter.next().ok_or("--hud-preview needs a name")?;
                parsed.hud_preview = Some(parse_hud_preview(value)?);
            }
            other if other.starts_with("--hud-preview=") => {
                parsed.hud_preview = Some(parse_hud_preview(&other["--hud-preview=".len()..])?);
            }
            "--glyphs" => {
                let value = iter.next().ok_or("--glyphs needs xbox or dualsense")?;
                parsed.glyphs = Some(parse_glyphs(value)?);
            }
            other if other.starts_with("--glyphs=") => {
                parsed.glyphs = Some(parse_glyphs(&other["--glyphs=".len()..])?);
            }
            other if parsed.hud_settings.apply_flag(other)? => {}
            "--no-lighting" => parsed.no_lighting = true,
            "--demo" => parsed.demo = true,
            "--autopilot" => parsed.autopilot = true,
            "--demo-beat" => {
                let value = iter.next().ok_or("--demo-beat needs a beat 0 to 6")?;
                parsed.demo_beat = value
                    .parse()
                    .ok()
                    .filter(|beat| *beat < demo::BEATS.len())
                    .ok_or_else(|| format!("bad beat {value:?} (0 to 6)"))?;
            }
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
            "--death-lab-finish" => {
                let value = iter.next().ok_or("--death-lab-finish needs a finish")?;
                parsed.death_lab_finish = Some(match value.as_str() {
                    "decapitated" | "decap" => Finish::Decapitated,
                    other => {
                        return Err(format!("bad lab finish {other:?} (expected decapitated)"));
                    }
                });
            }
            "--camera" => {
                let value = iter.next().ok_or("--camera needs a mode")?;
                parsed.camera = Some(CameraChoice::parse(value)?);
            }
            "--camera-test-room" => {
                let value = iter.next().ok_or("--camera-test-room needs 1 to 6")?;
                parsed.camera_test_room = Some(
                    value
                        .parse()
                        .ok()
                        .filter(|n| (1..=6).contains(n))
                        .ok_or_else(|| format!("bad camera test room {value:?} (1 to 6)"))?,
                );
            }
            "--camera-test-threat" => {
                let value = iter
                    .next()
                    .ok_or("--camera-test-threat needs X:FIRST:LAST")?;
                let bad = || format!("bad camera test threat {value:?} (expected X:FIRST:LAST)");
                let mut parts = value.split(':');
                let x: f32 = parts.next().and_then(|v| v.parse().ok()).ok_or_else(bad)?;
                let first: u64 = parts.next().and_then(|v| v.parse().ok()).ok_or_else(bad)?;
                let last: u64 = parts.next().and_then(|v| v.parse().ok()).ok_or_else(bad)?;
                if parts.next().is_some() || last <= first {
                    return Err(bad());
                }
                parsed.camera_test_threat = Some((x, first, last));
            }
            "--view-trace" => parsed.view_trace = true,
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
    if parsed.demo {
        if parsed.fixture.is_some() || parsed.scenario.is_some() || parsed.reference_frame {
            return Err("--demo replaces --fixture, --scenario and --reference-frame".into());
        }
        parsed.fixture = Some(demo::fixture_for_beat(parsed.demo_beat));
        // A live run without a script or autopilot is the player's.
        parsed.manual = !parsed.autopilot
            && !(parsed.headless
                || parsed.capture_tick.is_some()
                || parsed.capture_sequence.is_some()
                || parsed.exit_after_ticks.is_some()
                || parsed.freeze
                || parsed.perf_frames.is_some());
    } else if parsed.autopilot || parsed.demo_beat != 0 {
        return Err("--autopilot and --demo-beat need --demo".into());
    }
    if parsed.demo && parsed.camera_test_room.is_some() {
        return Err(
            "--camera-test-room and --demo are mutually exclusive (the demo has its own hall)"
                .into(),
        );
    }
    if parsed.party_fixture.is_some() && !(parsed.headless || parsed.capture_tick.is_some())
        || parsed.party_fixture.is_some()
            && (parsed.manual
                || parsed.capture_sequence.is_some()
                || parsed.exit_after_ticks.is_some()
                || parsed.perf_frames.is_some()
                || parsed.death_stress
                || parsed.combat_trace
                || parsed.anim_trace)
    {
        return Err(
            "--party-fixture is a headless replay or a --capture-tick capture only (no --manual, --capture-sequence, --exit-after-ticks, --perf-frames, --death-stress, --combat-trace or --anim-trace)"
                .into(),
        );
    }
    let driven = parsed.scenario.is_some() || parsed.fixture.is_some();
    if parsed.scenario.is_some() && parsed.fixture.is_some() {
        return Err("--scenario and --fixture are mutually exclusive".into());
    }
    if (parsed.manual || parsed.debug_combat) && parsed.fixture.is_none() {
        return Err("--manual and --debug-combat need --fixture".into());
    }
    if (parsed.death_stress || parsed.death_lab_finish.is_some() || parsed.death_trace)
        && parsed.fixture.is_none()
    {
        return Err("--death-stress, --death-lab-finish and --death-trace need --fixture".into());
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
    if parsed.view_trace && !parsed.headless {
        return Err("--view-trace needs --headless-floor01-smoke".into());
    }
    if parsed.camera_test_threat.is_some() && parsed.camera_test_room.is_none() && !parsed.demo {
        return Err("--camera-test-threat needs --camera-test-room or --demo (a tall room)".into());
    }
    if parsed.reference_frame && parsed.camera.is_some_and(|c| c != CameraChoice::Scroll) {
        return Err("--reference-frame pins the scroll camera; drop --camera".into());
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
    PartyState,
);

/// The cosmetic death presenter of a fixture run: the goblin death art at the
/// requested preset. Presentation only.
fn death_presenter(root: &Path, args: &Args) -> Result<DeathPresenter, String> {
    let art = Arc::new(DeathArt::load(root)?);
    let variant_b = DeathArt::load_variant(root, 1)?;
    let mut presenter = DeathPresenter::with_variants(art, &variant_b, args.gore_preset)?;
    presenter.set_lab_finish(args.death_lab_finish);
    Ok(presenter)
}

/// The HUD runtime with its real identity: the actor manifests' display names
/// and the floor number of the layout.
fn new_hud(args: &Args, kit: &EnvKit, actors: &Actors) -> HudRuntime {
    let mut hud = HudRuntime::new(args.hud_config());
    hud.set_identity(
        &actors.carl.display_name,
        &actors.donut.display_name,
        &kit.layout_id,
    );
    hud
}

/// One HUD step after a finished simulation tick: the real health values and
/// the newest combat events. Read-only on the simulation.
fn step_hud(hud: &mut HudRuntime, sim: &SceneSim, combat: Option<&CombatState>) {
    let (events, dropped) =
        combat.map_or((&[][..], 0), |c| (c.events.as_slice(), c.dropped_events));
    // Melee input is live exactly when the combat state exists (see the
    // manual input gate in `fixed_step`).
    hud.set_melee_live(combat.is_some());
    hud.step(
        sim.world.hero.health,
        sim.world.donut.health,
        events,
        dropped,
    );
}

fn build_scene(
    kit: &EnvKit,
    args: &Args,
    actors: &Actors,
    root: &Path,
    hud: &mut HudRuntime,
) -> Result<Scene, String> {
    if let Some(fixture) = args.fixture {
        // A combat fixture: Carl starts at the fixture column; the script is
        // pre-simulated for a capture, otherwise left for the live loop.
        let mut sim = SceneSim::new_at(kit, view_height(args.deck), fixture.carl_x)?;
        let mut combat = CombatState::new(fixture, kit, &sim)?;
        if args.demo {
            combat = combat.with_demo(&demo::DEMO_RULES);
        }
        let mut anim = actors.animators(&sim.world);
        anim.art = actors.combat_art(&combat);
        let mut death = death_presenter(root, args)?;
        hud.set_melee_live(true);
        // A party fixture runs the live party exactly like the interactive
        // scene (slot inputs, solo swap, drive); a plain fixture stays legacy.
        let mut party = args.party_fixture.map_or_else(PartyState::default, |pf| {
            let mut party = PartyState::new(pf.tuning);
            if pf.two_player {
                party.pad_connected(PadId(1));
                party.pad_start_pressed(PadId(2));
            }
            party
        });
        if args.party_fixture.is_some() {
            sim.set_drive(party.drive(true));
        }
        for _ in 0..presim_ticks(args).unwrap_or(0) {
            let inputs = if let Some(pf) = args.party_fixture {
                let slots = pf.slot_inputs(sim.world.tick + 1);
                let hurt = combat.hurt_ticks_of(party.controlled_hero()) > 0;
                party.request_swap(slots[0].swap, hurt);
                sim.set_drive(party.drive(true));
                party.hero_inputs(slots)
            } else {
                [
                    scripted_input(args, fixture, &sim, &combat),
                    RawInput::default(),
                ]
            };
            let restarts = combat.restarts;
            let _ = step_combat(&mut sim, &mut combat, &mut anim, Some(&mut death), inputs);
            if combat.restarts != restarts {
                party.restart();
            }
            step_hud(hud, &sim, Some(&combat));
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
        return Ok((sim, anim, Some(combat), Some(death), party));
    }
    let mut sim = SceneSim::new(kit, view_height(args.deck))?;
    let mut anim = actors.animators(&sim.world);
    if let (Some(scenario), Some(ticks)) = (args.scenario, presim_ticks(args)) {
        let trace = args.anim_trace;
        sim.run_scenario_observed(scenario, ticks, |state| {
            anim.step(&state.world);
            step_hud(hud, state, None);
            if trace {
                println!("{}", trace_line(state, &anim));
            }
        });
    }
    Ok((sim, anim, None, None, PartyState::default()))
}

/// One fixed tick of a combat fixture, and the animation that follows it. A
/// restart inside the step (Carl was defeated) restarts the animation too.
/// Returns true when a DEMO-01 respawn happened inside the step.
fn step_combat(
    sim: &mut SceneSim,
    combat: &mut CombatState,
    anim: &mut SceneAnimators,
    death: Option<&mut DeathPresenter>,
    inputs: [RawInput; 2],
) -> bool {
    let expected = sim.world.tick + 1;
    combat.step_party(sim, inputs);
    // A demo respawn keeps the tick counter but moves Carl: restart his
    // animation like a fixture restart does.
    let respawned = combat.take_respawned();
    if sim.world.tick == expected && !respawned {
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
    respawned
}

/// The input of the scripted run of a fixture at the next tick: the fixture's
/// script, or the DEMO-01 autopilot (a pure function of the finished state).
fn scripted_input(
    args: &Args,
    fixture: &Fixture,
    sim: &SceneSim,
    combat: &CombatState,
) -> RawInput {
    if args.autopilot {
        demo::autopilot(combat, sim)
    } else {
        fixture.input(sim.world.tick + 1)
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
    let kit = if args.demo {
        demo::load_hall(&root)
    } else if let Some(room) = args.camera_test_room {
        test_room_kit(&root, room)
    } else {
        EnvKit::load(&root, LAYOUT_PATH).map_err(|error| error.to_string())
    };
    let kit = match kit {
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
    let actors = match Actors::load(&root, source, args.fixture.is_some()) {
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
    let mut hud = new_hud(&args, &kit, &actors);
    let (sim, anim, combat, death, party) = match build_scene(&kit, &args, &actors, &root, &mut hud)
    {
        Ok(scene) => scene,
        Err(message) => return failure(&message),
    };
    run_window(
        kit, sim, anim, combat, death, party, actors, hud, args, &root,
    );
    ExitCode::SUCCESS
}

// ------------------------------------------------------------------ resources

#[derive(Resource)]
struct Sim(SceneSim);

/// Asset path of the HUD atlas when its art is exported (the loading
/// watchdog waits for it like for every other atlas).
#[derive(Resource)]
struct HudAtlas(Option<&'static str>);

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
}

#[derive(Clone)]
struct CombatArt {
    carl: SpriteClips,
    goblins: [SpriteClips; GOBLIN_VARIANTS],
}

impl Actors {
    fn load(root: &Path, source: FrameSource, combat: bool) -> Result<Self, String> {
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

/// DEMO-01 PR C: the presentation camera of the run, `None` for the scroll
/// camera (every existing capture). Presentation only: it reads the simulation
/// after the tick and nothing reads it back.
#[derive(Resource)]
struct SceneCam {
    rig: Option<SceneCameraRig>,
    /// Size in world pixels of the lighting slot targets: the widest view of
    /// the chosen camera (640 x 360 or 640 x 400 for the scroll camera).
    slot: (i32, i32),
    /// The rest width last printed as `FLOOR1_CAMERA_REST`.
    reported_rest: Option<i32>,
}

/// The read-only threat list gameplay feeds the camera (a llama that has taken
/// aim, its tell, a spit in flight). Empty in this PR unless `--camera-test-threat`
/// is given.
#[derive(Resource, Default)]
struct Interest(CameraInterest);

/// The fixture whose script drives the live run (not set for `--manual`).
#[derive(Resource)]
struct LiveFixture(Option<&'static Fixture>);

/// Keyboard taps latched between fixed steps (pad edges live in [`Pads`]).
#[derive(Resource, Default)]
struct Controls {
    jump_tap: bool,
    attack_tap: bool,
    physical_tap: bool,
    swap_tap: bool,
    restart: bool,
    /// DEMO-01 PR C: Carl was moved on purpose (a respawn, a restart): the
    /// presentation camera cuts to the new framing instead of panning.
    camera_cut: bool,
    /// DEMO-01: Select (R or the pad's Select) went down since the last tick.
    select_tap: bool,
    /// DEMO-01: a pad's Select is held (polled every frame).
    select_held: bool,
    select: SelectControl,
    /// DEMO-01: the furthest beat announced and whether the end was.
    beat_printed: Option<usize>,
    complete_printed: bool,
}

/// Local-player slot ownership (`pad_slots`): which pad drives P1 and P2, the
/// solo swap, the disconnect pause and the per-plate glyph families.
#[derive(Resource, Default)]
struct Pads(PadSlots);

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
    /// Donut's own hurtbox (Q4), shown while the party is live.
    DonutHurt,
}

/// The presentation camera of a run: the rig (none for the scroll camera) and
/// the lighting slot target size. The output width decides the automatic
/// camera's integer-scale rest widths.
fn scene_camera_of(
    kit: &EnvKit,
    args: &Args,
    sim: &SceneSim,
    combat: Option<&CombatState>,
    window: (u32, u32),
) -> SceneCam {
    let choice = args.camera_choice();
    let aspect = Aspect::from_output(window.0, window.1);
    let rig = SceneCameraRig::new(
        RigConfig {
            choice,
            aspect,
            out_w: window.0,
            bounds: Bounds {
                w: kit.width,
                h: kit.height,
            },
        },
        sim,
        combat,
        &test_interest(kit, args, elapsed_ticks(sim, combat)),
        elapsed_ticks(sim, combat),
    );
    // The slot targets cover the widest view this rig can show in this room at
    // this output (the scroll camera: 640, as always).
    let slot = rig
        .as_ref()
        .map_or_else(|| choice.slot_target(aspect, window.0), |r| r.slot_target());
    SceneCam {
        rig,
        slot,
        reported_rest: None,
    }
}

/// The threats of this tick: the synthetic one of `--camera-test-threat`, if
/// any. Gameplay (the Bad Llama of DEMO-01 PR B) feeds the same
/// [`CameraInterest`] from its own state.
fn test_interest(kit: &EnvKit, args: &Args, elapsed: u64) -> CameraInterest {
    match args.camera_test_threat {
        Some((x, first, last)) => synthetic_interest(
            x,
            to_world_y(kit.height, kit.base_walk_line_y),
            first,
            last,
            elapsed,
        ),
        None => CameraInterest::default(),
    }
}

/// The view as drawn this frame (layout px, y down): the presentation camera
/// with the CP-01 kick scaled for its width, or the scroll camera with the kick
/// at 640 wide.
fn presentation_view(
    state: &SceneSim,
    setup: &Setup,
    reference: &ReferencePose,
    combat: &Combat,
    cam: &SceneCam,
) -> View {
    let kick = camera_kick(state, combat);
    match cam.rig.as_ref() {
        Some(rig) => rig.presented(kick),
        None => {
            let (left, top) = view_origin(state, setup, reference, kick);
            View {
                left,
                top,
                w: VIRTUAL_WIDTH,
                h: view_height(setup.args.deck),
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_window(
    kit: EnvKit,
    sim: SceneSim,
    anim: SceneAnimators,
    combat: Option<CombatState>,
    death: Option<DeathPresenter>,
    party: PartyState,
    actors: Actors,
    hud: HudRuntime,
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
    let camera = scene_camera_of(&kit, &args, &sim, combat.as_ref(), window_size);
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
    // The HUD camera draws after the last scene camera: one above the debug
    // overlay when lit (`first_post + 4`), one above the plain camera otherwise.
    let view_h = view_height(args.deck);
    // The unlit scene camera keeps Bevy's default sample count; the lit cameras
    // are all `Msaa::Off`. The HUD camera must match its target's cameras.
    let hud_msaa = if lighting_on {
        Msaa::Off
    } else {
        Msaa::default()
    };
    let hud_order = match kit.lighting.as_ref().filter(|_| lighting_on) {
        Some(lighting) => lighting.plan.slots.len() as isize + 5,
        None => 1,
    };
    let view_key = if args.deck {
        REFERENCE_VIEW_DECK
    } else {
        REFERENCE_VIEW
    };
    let Some(view) = kit.view(view_key) else {
        let message = format!("layout has no view {view_key} for the HUD");
        println!("FLOOR1_ASSET_FAILURE: {message}");
        eprintln!("FLOOR1_ASSET_FAILURE: {message}");
        std::process::exit(1);
    };
    let view_top = view.y;
    let hud_layout = HudLayout::new(view_h, kit.base_walk_line_y - view_top);
    let hud_plugin = match HudPlugin::load(root, hud, hud_layout, hud_order, hud_msaa) {
        Ok(plugin) => plugin.hidden(args.reference_frame),
        Err(message) => {
            println!("FLOOR1_ASSET_FAILURE: {message}");
            eprintln!("FLOOR1_ASSET_FAILURE: {message}");
            std::process::exit(1);
        }
    };
    let hud_atlas = hud_plugin.atlas_asset();
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
    .add_plugins(hud_plugin)
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
    .insert_resource(camera)
    .init_resource::<Interest>()
    .insert_resource(LiveFixture(live_fixture))
    .insert_resource(HudAtlas(hud_atlas))
    .insert_resource(Setup { kit, args })
    .init_resource::<Controls>()
    .insert_resource(Pads(PadSlots::new(party)))
    .init_resource::<AfterExitTicks>()
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            pad_connections,
            capture_input,
            sync_party_hud,
            perf_step,
            sync_visuals,
            sync_combat_visuals,
            sync_shore_visuals,
            sync_death_visuals,
            update_lighting,
            watchdog,
            perf_probe,
            capture_sequence,
        )
            .chain(),
    )
    .add_systems(FixedUpdate, (fixed_step, update_scene_camera).chain());
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
    hud_atlas: Res<HudAtlas>,
    cam: Res<SceneCam>,
) {
    let kit = &setup.kit;
    let args = &setup.args;
    let height = kit.height;
    let view_h = view_height(args.deck);
    // The lighting slot targets and the parallax copies cover the widest view
    // of the chosen camera (the scroll camera: 640 wide, as always).
    let (slot_w, slot_h) = cam.slot;
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
    if let Some(asset) = hud_atlas.0 {
        handles.push((asset.to_owned(), asset_server.load(asset)));
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
            UVec2::new(slot_w as u32, slot_h as u32),
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
        let copies = (slot_w / strip.rect.w + 2) as usize;
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
            overlay(
                CombatDebugBox::DonutHurt,
                Color::srgba(1.0, 0.5, 0.9, 0.35),
                Vec2::ONE,
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

/// The opaque pad identity `pad_slots` keys on: the Bevy gamepad entity.
fn pad_id(entity: Entity) -> PadId {
    PadId(entity.to_bits())
}

/// Route gamepad connection messages to the slot owner table (D5/D6): the
/// first pad is P1, a later pad waits for Start, an owning pad that goes away
/// pauses the fixed step until a pad reclaims its slot. Presentation and input
/// routing only; the vendor id comes from `GamepadConnection::Connected`
/// (bevy_input 0.20.1, filled from the OS by bevy_gilrs).
fn pad_connections(mut events: MessageReader<GamepadEvent>, mut pads: ResMut<Pads>) {
    for event in events.read() {
        if let GamepadEvent::Connection(connection) = event {
            let pad = pad_id(connection.gamepad);
            match &connection.connection {
                GamepadConnection::Connected { vendor_id, .. } => {
                    pads.0.apply(PadEvent::Connected {
                        pad,
                        vendor_id: *vendor_id,
                    });
                }
                GamepadConnection::Disconnected => {
                    pads.0.apply(PadEvent::Disconnected { pad });
                }
            }
        }
    }
}

/// One pad's state for this frame, by logical action.
fn pad_frame(entity: Entity, pad: &Gamepad) -> PadFrame {
    let stick = pad.left_stick().x;
    PadFrame {
        pad: pad_id(entity),
        vendor_id: pad.vendor_id(),
        move_x: if stick.abs() > 0.18 {
            stick
        } else {
            pad.dpad().x
        },
        held: PadButtons {
            jump: pad.pressed(GamepadButton::South),
            attack: pad.pressed(GamepadButton::West),
            // PhysicalAttack is Triangle/Y (North); North no longer restarts.
            physical: pad.pressed(GamepadButton::North),
            swap: pad.pressed(GamepadButton::LeftTrigger),
            restart: pad.pressed(GamepadButton::Select),
            start: pad.pressed(GamepadButton::Start),
        },
        pressed: PadButtons {
            jump: pad.just_pressed(GamepadButton::South),
            attack: pad.just_pressed(GamepadButton::West),
            physical: pad.just_pressed(GamepadButton::North),
            swap: pad.just_pressed(GamepadButton::LeftTrigger),
            restart: pad.just_pressed(GamepadButton::Select),
            start: pad.just_pressed(GamepadButton::Start),
        },
    }
}

fn capture_input(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<(Entity, &Gamepad)>,
    mut controls: ResMut<Controls>,
    mut pads: ResMut<Pads>,
    mut exit: MessageWriter<AppExit>,
) {
    controls.jump_tap |= keys.just_pressed(KeyCode::Space);
    controls.attack_tap |= keys.just_pressed(KeyCode::KeyJ);
    controls.physical_tap |= keys.just_pressed(KeyCode::KeyK);
    // Temporary keyboard default for the solo swap (the contract only fixes
    // L1/LB); the keyboard stays P1.
    controls.swap_tap |= keys.just_pressed(KeyCode::KeyQ);
    controls.restart |= keys.just_pressed(KeyCode::KeyR);
    controls.select_tap |= keys.just_pressed(KeyCode::KeyR);
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
    // Each pad is polled on its own and routed by `pad_slots`: the owner of
    // P1 or P2 drives its hero, any other pad is ignored except for Start
    // (which joins P2). A pad that vanishes (hot-unplug) leaves the query, so
    // nothing sticks.
    controls.select_held = false;
    for (entity, pad) in &gamepads {
        controls.select_held |= pad.pressed(GamepadButton::Select);
        let commands = pads.0.sample(pad_frame(entity, pad));
        controls.restart |= commands.restart;
        controls.select_tap |= commands.restart;
        if commands.exit {
            exit.write(AppExit::Success);
        }
    }
}

/// Push the real party state to the HUD every frame: who drives each plate,
/// each plate's own pad glyph family (HUD-DEBT-05 per slot, provisional Q11)
/// and the reconnect request of a disconnect pause. `set_party` bumps the HUD
/// revision only on a change, and this runs in `Update` so the message shows
/// while the fixed step is paused. Presentation only.
fn sync_party_hud(pads: Res<Pads>, mut hud: ResMut<HudState>) {
    hud.0.set_party(pads.0.hud_view());
}

/// The controlled hero is in a hit reaction (blocks the solo swap, Q8): Carl's
/// or Donut's own hurt state.
fn controlled_hero_hurt(party: &PartyState, combat: Option<&CombatState>) -> bool {
    combat.is_some_and(|c| c.hurt_ticks_of(party.controlled_hero()) > 0)
}

#[allow(clippy::too_many_arguments)]
fn fixed_step(
    keys: Res<ButtonInput<KeyCode>>,
    mut pads: ResMut<Pads>,
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
    mut hud: ResMut<HudState>,
    mut exit: MessageWriter<AppExit>,
) {
    let mut restart = std::mem::take(&mut controls.restart);
    let select_tap = std::mem::take(&mut controls.select_tap);
    let jump_tap = std::mem::take(&mut controls.jump_tap);
    let attack_tap = std::mem::take(&mut controls.attack_tap);
    let physical_tap = std::mem::take(&mut controls.physical_tap);
    let swap_tap = std::mem::take(&mut controls.swap_tap);
    // Keyboard is P1. Pad edges are consumed here too, so a tap made during a
    // pause is dropped and nothing fires on resume.
    let keyboard = {
        let left = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft);
        let right = keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight);
        RawInput {
            move_x: f32::from(i8::from(right) - i8::from(left)),
            jump: keys.pressed(KeyCode::Space) || jump_tap,
            attack: keys.pressed(KeyCode::KeyJ) || attack_tap,
            physical: keys.pressed(KeyCode::KeyK) || physical_tap,
            swap: swap_tap,
            ..RawInput::default()
        }
    };
    let mut live_slots = pads.0.take_inputs(keyboard);
    if frozen.0 {
        return;
    }
    // D6: an owning pad is gone; the sim waits for it (the HUD says so).
    if pads.0.is_paused() {
        return;
    }
    if let Some(limit) = exit_after.0
        && elapsed_ticks(&sim.0, combat.0.as_ref()) >= limit
    {
        // Already reported; further fixed steps of this frame do nothing.
        return;
    }
    // DEMO-01 live run: Select tap respawns, Select held for a second restarts
    // the demo (R on the keyboard is the same button).
    let mut respawn = false;
    if setup.args.demo && setup.args.manual {
        restart = false;
        let down = keys.pressed(KeyCode::KeyR) || controls.select_held || select_tap;
        match controls.select.tick(down) {
            SelectAction::Respawn => respawn = true,
            SelectAction::Restart => restart = true,
            SelectAction::None => {}
        }
    }
    if respawn && let Some(combat) = combat.0.as_mut() {
        combat.respawn(&mut sim.0);
        let _ = combat.take_respawned();
        controls.camera_cut = true;
        pads.0.party_mut().restart();
        anim.0.reset(&sim.0.world);
        anim.0.reset_art(combat);
        print_combat_events(combat, Some(&mut audio));
        println!("DEMO01_RESPAWN checkpoint={}", combat.checkpoint);
    }
    if restart {
        match combat.0.as_mut() {
            Some(combat) => combat.restart(&mut sim.0),
            None => sim.0.reset(),
        }
        controls.beat_printed = None;
        controls.complete_printed = false;
        anim.0.reset(&sim.0.world);
        if let Some(combat) = combat.0.as_ref() {
            anim.0.reset_art(combat);
        }
        if let Some(death) = death.0.as_mut() {
            death.clear();
        }
        hud.0.restart();
        pads.0.party_mut().restart();
        controls.camera_cut = true;
        after_exit.0 = 0;
        println!("FLOOR1_RESTART");
    }
    // Melee exists only in a combat fixture (`--fixture ... --manual`).
    if combat.0.is_none() {
        for slot in &mut live_slots {
            slot.attack = false;
            slot.physical = false;
        }
    }
    let slot_inputs = if let Some(scenario) = scenario.0 {
        [scenario.input(&sim.0.world), RawInput::default()]
    } else if let Some(fixture) = fixture.0 {
        let raw = match combat.0.as_ref() {
            Some(combat) => scripted_input(&setup.args, fixture, &sim.0, combat),
            None => fixture.input(sim.0.world.tick + 1),
        };
        [raw, RawInput::default()]
    } else {
        live_slots
    };
    // Solo swap (L1/LB, Q on the keyboard), then slot -> hero routing. With no
    // pad and no swap this is the identity on Carl, so scripted runs and
    // replays are unchanged.
    let hurt = controlled_hero_hurt(pads.0.party(), combat.0.as_ref());
    pads.0.party_mut().request_swap(slot_inputs[0].swap, hurt);
    let hero_inputs = pads.0.party().hero_inputs(slot_inputs);
    // A live party (Donut is a real hero, a bot or P2) exists only in
    // interactive play; scripted scenario and fixture runs keep the legacy
    // single-hero simulation, so every recorded hash is unchanged.
    let live_party = scenario.0.is_none() && fixture.0.is_none();
    sim.0.set_drive(pads.0.party().drive(live_party));
    let was_complete = sim.0.world.complete;
    if let Some(combat) = combat.0.as_mut() {
        let restarts = combat.restarts;
        let respawned = step_combat(
            &mut sim.0,
            combat,
            &mut anim.0,
            death.0.as_mut(),
            hero_inputs,
        );
        if combat.restarts != restarts || respawned {
            // A party defeat restarted the attempt (or the demo respawned
            // Carl): Carl is controlled again.
            pads.0.party_mut().restart();
            controls.camera_cut = true;
        }
        print_combat_events(combat, Some(&mut audio));
        if respawned {
            println!("DEMO01_RESPAWN checkpoint={}", combat.checkpoint);
        }
        if setup.args.demo {
            // Console progress of the demo (presentation only).
            // The first tick only records the beat the run started at.
            let printed = controls.beat_printed.get_or_insert(combat.checkpoint);
            while *printed < combat.checkpoint {
                *printed += 1;
                let beat = demo::BEATS[*printed];
                println!("DEMO01_BEAT {} {}", beat.id, beat.name);
            }
            if demo::complete(combat) && !controls.complete_printed {
                controls.complete_printed = true;
                println!("DEMO01_COMPLETE tick={}", sim.0.world.tick);
            }
        }
        if setup.args.death_stress
            && let Some(death) = death.0.as_mut()
        {
            // Lab: keep the pools loaded (presentation only).
            let hero = &sim.0.world.hero;
            death.stress_tick(hero.x, hero.y - HERO_HALF_HEIGHT);
        }
    } else {
        sim.0.step_party(hero_inputs);
        anim.0.step(&sim.0.world);
    }
    step_hud(&mut hud.0, &sim.0, combat.0.as_ref());
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

/// DEMO-01 PR C: one presentation camera step for the tick the simulation just
/// finished. Reads the simulation and the interest list through shared
/// references only; nothing flows back. Called after every simulated tick: by
/// the fixed schedule (right after `fixed_step`), and inside the loops that step
/// the simulation by hand (`--capture-sequence`, the `--perf-from-tick` probe),
/// so the drawn camera is the per-tick camera of `--view-trace` at any frame rate.
fn camera_tick(
    cam: &mut SceneCam,
    interest: &mut Interest,
    setup: &Setup,
    sim: &SceneSim,
    combat: Option<&CombatState>,
    cut: bool,
) {
    let Some(rig) = cam.rig.as_mut() else {
        return;
    };
    if cut {
        // The explicit signal; the distance check inside `sync` is the fallback.
        rig.request_cut();
    }
    let elapsed = elapsed_ticks(sim, combat);
    if setup.args.camera_test_threat.is_some() {
        interest.0 = test_interest(&setup.kit, &setup.args, elapsed);
    }
    let view = rig.sync(sim, combat, &interest.0, elapsed);
    if cam.reported_rest != Some(rig.operator().rest_target()) {
        cam.reported_rest = Some(rig.operator().rest_target());
        println!(
            "FLOOR1_CAMERA_REST tick={elapsed} rest={} view={}x{}",
            rig.operator().rest_target(),
            view.w,
            view.h
        );
    }
}

/// The fixed-schedule camera step: chained after `fixed_step`, it runs once per
/// simulated tick, with that tick's threat list.
fn update_scene_camera(
    mut cam: ResMut<SceneCam>,
    mut interest: ResMut<Interest>,
    sim: Res<Sim>,
    combat: Res<Combat>,
    setup: Res<Setup>,
    mut controls: ResMut<Controls>,
) {
    let cut = std::mem::take(&mut controls.camera_cut);
    camera_tick(
        &mut cam,
        &mut interest,
        &setup,
        &sim.0,
        combat.0.as_ref(),
        cut,
    );
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
    scroll_origin(state, setup.kit.width, kick)
}

/// The scroll camera's view top-left plus the kick, kept inside the layout.
fn scroll_origin(state: &SceneSim, layout_width: i32, kick: i32) -> (i32, i32) {
    let widest = (layout_width - VIRTUAL_WIDTH).max(0);
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

/// Donut's hit reaction as a tint (presentation only, live party only): the
/// impact flash for the first ticks of her flinch, then a blink through her
/// invulnerability. She has no hurt art yet; Carl has his own.
fn donut_tint(sim: &SceneSim, combat: Option<&CombatState>) -> Color {
    let Some(combat) = combat.filter(|_| sim.party_live()) else {
        return Color::WHITE;
    };
    let hurt = combat.hurt_ticks_of(HeroId::Donut);
    let invulnerable = sim.world.donut_body.invulnerable_ticks;
    if hurt > HERO_HURT_TICKS.saturating_sub(IMPACT_FLASH_TICKS) {
        FLASH_TINT
    } else if invulnerable > 0 && (invulnerable / 4).is_multiple_of(2) {
        Color::srgba(1.0, 1.0, 1.0, 0.5)
    } else {
        Color::WHITE
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
    cam: Res<SceneCam>,
    mut cameras: Query<
        (
            &mut Transform,
            Option<&mut Camera>,
            Option<&mut Projection>,
            Option<&RenderTarget>,
        ),
        With<SceneCamera>,
    >,
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
    let view = presentation_view(state, &setup, &reference, &combat, &cam);
    let (left, top) = (view.left, view.top);
    let anim_tick = presentation_tick(state, &after_exit, &reference);

    // Every scene camera (the plain one, or the lighting slot cameras and the
    // debug overlay camera) sits on the same whole-pixel position.
    for (mut transform, camera, projection, target) in &mut cameras {
        transform.translation.x = (left + view.w / 2) as f32;
        transform.translation.y = to_world_y(height, top + view.h / 2);
        if cam.rig.is_none() {
            continue;
        }
        // The presentation camera changes how much world the view shows: the
        // projection covers exactly the current view, and a lighting slot
        // camera draws it 1:1 into the top-left region of its (largest-view)
        // target. The composite pass lights that region at output resolution.
        if let Some(mut projection) = projection
            && let Projection::Orthographic(ortho) = projection.as_mut()
        {
            ortho.scaling_mode = ScalingMode::Fixed {
                width: view.w as f32,
                height: view.h as f32,
            };
        }
        if let (Some(mut camera), Some(RenderTarget::Image(_))) = (camera, target) {
            camera.viewport = Some(Viewport {
                physical_position: UVec2::ZERO,
                physical_size: UVec2::new(view.w as u32, view.h as u32),
                ..default()
            });
        }
    }

    for (strip_marker, mut transform, mut visibility) in &mut parallax {
        let strip = &kit.strips[strip_marker.strip];
        let start = parallax_start(strip.x_origin, left, strip.factor);
        let copies = parallax_copies(start, strip.rect.w, left, view.w);
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
        if !actor.is_hero {
            sprite.color = donut_tint(state, combat.0.as_ref());
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
/// knockdown pose is drawn) and the `--debug-combat` boxes (Donut's hurtbox
/// while the party is live).
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
            CombatDebugBox::DonutHurt => {
                if sim.0.party_live() {
                    rect(&mut transform, CombatState::hurtbox(&sim.0, HeroId::Donut));
                    *visibility = Visibility::Inherited;
                } else {
                    *visibility = Visibility::Hidden;
                }
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
    let hide_goblin = |index: usize| {
        presenter.pose_of(index as u32).is_some() || presenter.is_retired(index as u32)
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
    cam: Res<SceneCam>,
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
    let drawn = presentation_view(state, &setup, &reference, &combat, &cam);
    let tick = presentation_tick(state, &after_exit, &reference);
    let view = ViewRect {
        left: drawn.left,
        top: drawn.top,
        w: drawn.w,
        h: drawn.h,
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
        view_w: drawn.w as u32,
        view_h: drawn.h as u32,
        view_top: drawn.top,
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
    mut cam: ResMut<SceneCam>,
    mut interest: ResMut<Interest>,
) {
    let Some(mut perf) = perf else { return };
    if !perf.step_ticks || loading.is_some_and(|l| !l.reported) || perf.seen < perf.warmup {
        return;
    }
    let (Some(fixture), Some(combat)) = (setup.args.fixture, combat.0.as_mut()) else {
        return;
    };
    let started = std::time::Instant::now();
    let raw = scripted_input(&setup.args, fixture, &sim.0, combat);
    let _ = step_combat(
        &mut sim.0,
        combat,
        &mut anim.0,
        death.0.as_mut(),
        [raw, RawInput::default()],
    );
    if setup.args.death_stress
        && let Some(death) = death.0.as_mut()
    {
        let hero = &sim.0.world.hero;
        death.stress_tick(hero.x, hero.y - HERO_HALF_HEIGHT);
    }
    perf.tick_us.push(started.elapsed().as_secs_f32() * 1e6);
    // This probe steps the simulation outside the fixed schedule.
    camera_tick(
        &mut cam,
        &mut interest,
        &setup,
        &sim.0,
        Some(&*combat),
        false,
    );
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
    mut cam: ResMut<SceneCam>,
    mut interest: ResMut<Interest>,
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
                    let raw = scripted_input(&setup.args, fixture, &sim.0, combat);
                    let _ = step_combat(
                        &mut sim.0,
                        combat,
                        &mut anim.0,
                        death.0.as_mut(),
                        [raw, RawInput::default()],
                    );
                } else if let Some(scenario) = scenario {
                    let raw = scenario.input(&sim.0.world);
                    sim.0.step(raw);
                    anim.0.step(&sim.0.world);
                }
                // One camera step per simulated tick, as in the live loop.
                camera_tick(
                    &mut cam,
                    &mut interest,
                    &setup,
                    &sim.0,
                    combat.0.as_ref(),
                    false,
                );
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
    if args.view_trace {
        // The view trace runs on any layout (the corridor checks below assume
        // the sample corridor, not a camera test room).
        return view_trace(kit, args, actors, root);
    }
    if args.demo {
        // The demo hall has no exit and a different spawn: the story-run checks
        // below do not apply. Replay the fixture.
        println!(
            "FLOOR1_HEADLESS_OK: demo hall {} tiles, {} pieces, {} merged solids",
            kit.tiles.len(),
            kit.pieces.len(),
            kit.solid_rects.len()
        );
        return headless_fixture(kit, args, actors, root);
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
    if args.fixture.is_some() {
        return headless_fixture(kit, args, actors, root);
    }
    let mut hud = new_hud(args, kit, actors);
    let (sim, anim, _, _, _) = build_scene(kit, args, actors, root, &mut hud)?;
    println!("{}", sim.state_line());
    println!("{}", anim.state_line(sim.world.tick));
    Ok(())
}

/// Headless fixture replay (and the demo): the scripted run for
/// `--capture-tick` ticks (default the fixture's own length), every event,
/// then the state lines.
fn headless_fixture(kit: &EnvKit, args: &Args, actors: &Actors, root: &Path) -> Result<(), String> {
    if let Some(fixture) = args.fixture {
        let mut args = args.clone();
        args.capture_tick.get_or_insert(fixture.ticks);
        let mut hud = new_hud(&args, kit, actors);
        let (sim, anim, combat, death, party) = build_scene(kit, &args, actors, root, &mut hud)?;
        let mut combat = combat.ok_or("fixture without combat state")?;
        print_combat_events(&mut combat, None);
        println!("{}", sim.state_line());
        println!("{}", anim.state_line(sim.world.tick));
        print_combat_state(&sim, &anim, &combat);
        if let Some(party_fixture) = args.party_fixture {
            println!(
                "{}",
                party_scene::party_state_line(party_fixture, &combat, &sim, &party)
            );
        }
        if args.death_trace
            && let Some(death) = death
        {
            println!("{}", death.state_line());
        }
        return Ok(());
    }
    let mut hud = new_hud(args, kit, actors);
    let (sim, anim, _, _, _) = build_scene(kit, args, actors, root, &mut hud)?;
    println!("{}", sim.state_line());
    println!("{}", anim.state_line(sim.world.tick));
    Ok(())
}

/// `--view-trace`: one `FLOOR1_VIEW` line per simulated tick with the view the
/// presentation would draw (the CP-01 kick included), for the fixture's script
/// or the scenario (default: walk right for 600 ticks), then the final state
/// hash. The trace is the same for any lighting, audio or gore setting, and the
/// hash is the same for any `--camera`.
fn view_trace(kit: &EnvKit, args: &Args, actors: &Actors, root: &Path) -> Result<(), String> {
    let mut start = args.clone();
    start.capture_tick = Some(0);
    start.capture_sequence = None;
    start.perf_from_tick = None;
    let mut hud = new_hud(&start, kit, actors);
    let (mut sim, mut anim, mut combat, mut death, _party) =
        build_scene(kit, &start, actors, root, &mut hud)?;
    let ticks = args
        .capture_tick
        .or(args.fixture.map(|f| f.ticks))
        .unwrap_or(600);
    let window = (
        VIRTUAL_WIDTH as u32 * args.scale,
        view_height(args.deck) as u32 * args.scale,
    );
    let scene_camera = scene_camera_of(kit, args, &sim, combat.as_ref(), window);
    let mut rig = scene_camera.rig;
    println!(
        "FLOOR1_VIEW_TRACE camera={} output={}x{} slot={}x{}",
        args.camera_choice().label(),
        window.0,
        window.1,
        scene_camera.slot.0,
        scene_camera.slot.1
    );
    let scenario = args.scenario.unwrap_or(Scenario::WalkRight);
    for _ in 0..ticks {
        let mut cut = false;
        match (args.fixture, combat.as_mut()) {
            (Some(fixture), Some(combat)) => {
                // The demo trace is the autopilot's recorded run of the route.
                let raw = if args.demo {
                    demo::autopilot(combat, &sim)
                } else {
                    fixture.input(sim.world.tick + 1)
                };
                cut = step_combat(
                    &mut sim,
                    combat,
                    &mut anim,
                    death.as_mut(),
                    [raw, RawInput::default()],
                );
            }
            _ => {
                let raw = scenario.input(&sim.world);
                sim.step(raw);
                anim.step(&sim.world);
            }
        }
        let elapsed = elapsed_ticks(&sim, combat.as_ref());
        let kick = combat.as_ref().map_or(0, |c| c.camera_kick(sim.world.tick));
        match rig.as_mut() {
            Some(rig) => {
                let interest = test_interest(kit, args, elapsed);
                if cut {
                    rig.request_cut();
                }
                rig.sync(&sim, combat.as_ref(), &interest, elapsed);
                println!("{}", rig.trace_line(elapsed, kick));
            }
            None => {
                let (left, top) = scroll_origin(&sim, kit.width, kick);
                println!(
                    "FLOOR1_VIEW tick={elapsed} left={left} top={top} w={VIRTUAL_WIDTH} h={} rest=0 kick={kick} kick_px={kick}",
                    view_height(args.deck)
                );
            }
        }
    }
    println!("{}", sim.state_line());
    if let Some(combat) = combat.as_ref() {
        println!(
            "FLOOR1_VIEW_TRACE_HASH combat={:016x}",
            combat.state_hash(&sim)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Result<Args, String> {
        let mut full = vec!["floor01_scene".to_owned()];
        full.extend(list.iter().map(|s| (*s).to_owned()));
        parse_args_with(&full, HudSettings::default())
    }

    #[test]
    fn a_party_fixture_is_a_headless_or_capture_run_and_excludes_a_plain_fixture() {
        let a = args(&[
            "--headless-floor01-smoke",
            "--party-fixture",
            "donut_defeat_party",
        ])
        .unwrap();
        assert_eq!(
            a.party_fixture.map(|f| f.name()),
            Some("donut_defeat_party")
        );
        assert_eq!(a.fixture.map(|f| f.name), Some("donut_defeat_party"));
        assert!(
            args(&[
                "--party-fixture",
                "duo_independent_moves",
                "--capture-tick",
                "60"
            ])
            .is_ok()
        );
        assert!(args(&["--party-fixture", "nope", "--capture-tick", "60"]).is_err());
        // A live party fixture run, or one mixed with other modes, is refused.
        assert!(args(&["--party-fixture", "duo_independent_moves"]).is_err());
        assert!(
            args(&[
                "--party-fixture",
                "duo_independent_moves",
                "--fixture",
                "cp01_kick",
                "--capture-tick",
                "60"
            ])
            .is_err()
        );
        assert!(
            args(&[
                "--fixture",
                "cp01_kick",
                "--party-fixture",
                "duo_independent_moves",
                "--capture-tick",
                "60"
            ])
            .is_err()
        );
        assert!(
            args(&[
                "--party-fixture",
                "duo_independent_moves",
                "--capture-tick",
                "60",
                "--manual"
            ])
            .is_err()
        );
    }

    #[test]
    fn the_hud_flags_parse_both_forms_and_reject_unknown_values() {
        assert_eq!(args(&[]).unwrap().hud_preview, None);
        assert_eq!(args(&[]).unwrap().glyphs, None);
        assert_eq!(
            args(&["--glyphs=xbox"]).unwrap().glyphs,
            Some(GlyphSet::Xbox)
        );
        assert_eq!(args(&[]).unwrap().hud_settings, HudSettings::default());
        for preview in HudPreview::ALL {
            let spaced = args(&["--hud-preview", preview.name()]).unwrap();
            let equals = args(&[&format!("--hud-preview={}", preview.name())]).unwrap();
            assert_eq!(spaced.hud_preview, Some(preview));
            assert_eq!(equals.hud_preview, Some(preview));
        }
        assert!(
            args(&["--hud-preview", "nope"])
                .unwrap_err()
                .contains("expected")
        );
        assert!(args(&["--hud-preview=nope"]).is_err());
        assert!(args(&["--hud-preview"]).is_err());
        assert_eq!(
            args(&["--glyphs=dualsense"]).unwrap().glyphs,
            Some(GlyphSet::DualSense)
        );
        assert_eq!(
            args(&["--glyphs", "dualsense"]).unwrap().glyphs,
            Some(GlyphSet::DualSense)
        );
        assert!(args(&["--glyphs=ps5"]).is_err());
        assert!(
            args(&["--hud-reduced-motion"])
                .unwrap()
                .hud_settings
                .reduced_motion
        );
        let both = args(&["--hud-reduced-motion=on", "--hud-high-contrast"]).unwrap();
        assert!(both.hud_settings.reduced_motion && both.hud_settings.high_contrast);
        let env = HudSettings {
            reduced_motion: true,
            high_contrast: true,
        };
        let full: Vec<String> = ["floor01_scene", "--hud-high-contrast=off"]
            .iter()
            .map(|a| a.to_string())
            .collect();
        let overridden = parse_args_with(&full, env).unwrap().hud_settings;
        assert!(overridden.reduced_motion && !overridden.high_contrast);
        assert!(args(&["--hud-high-contrast=maybe"]).is_err());
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
        assert!(args(&["--death-lab-finish", "decapitated"]).is_err());
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
        assert!(args(&["--fixture", "cp03_shore_kill", "--death-lab-finish", "bomb"]).is_err());
    }

    #[test]
    fn the_death_layers_draw_under_every_scene_actor() {
        // The scene's own actor offsets: Carl +0.1, Donut 0, a goblin
        // +0.05 / +0.15 (swinging), a lying goblin -0.01 minus 0.001 per index
        // (64 goblins is far more than any fixture).
        let lowest_actor = 7.0 + GOBLIN_LYING_Z_OFFSET - GOBLIN_Z_STEP * 64.0;
        let slots = PART_POOL_SLOTS.max(FLOOR_DECAL_POOL_SLOTS);
        for slot in [0, slots - 1] {
            for layer in [DrawLayer::Debris, DrawLayer::FloorDecal] {
                assert!(draw_z(layer, slot, 7.0, 3.0) < lowest_actor, "{layer:?}");
            }
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

    #[test]
    fn the_automatic_camera_is_the_demo_default_and_nothing_else_changes() {
        let demo = args(&["--demo", "--scale", "3"]).unwrap();
        assert_eq!(demo.camera_choice(), CameraChoice::Auto);
        let scroll = args(&["--demo", "--camera", "scroll", "--scale", "3"]).unwrap();
        assert_eq!(scroll.camera_choice(), CameraChoice::Scroll);
        // Every fixture, scenario and capture keeps the scroll camera.
        for list in [
            vec!["--fixture", "cp01_kick"],
            vec!["--scenario", "walk_right"],
            vec![],
        ] {
            assert_eq!(args(&list).unwrap().camera_choice(), CameraChoice::Scroll);
        }
        // The test room and the demo hall are two different rooms.
        assert!(args(&["--demo", "--camera-test-room", "1"]).is_err());
        assert!(args(&["--camera-test-room", "1"]).is_ok());
    }

    #[test]
    fn the_demo_flags_parse_and_validate() {
        let a = args(&["--demo", "--scale", "3"]).unwrap();
        assert!(a.demo && a.manual && !a.autopilot);
        assert_eq!(a.fixture.map(|f| f.name), Some("demo01"));
        assert_eq!(a.gore_preset, GorePreset::Full, "preset 4 by default");
        let a = args(&["--demo", "--demo-beat", "3", "--scale", "3"]).unwrap();
        assert_eq!(a.fixture.map(|f| f.name), Some("demo01_b3"));
        // A scripted run is never manual.
        let a = args(&[
            "--demo",
            "--demo-beat",
            "1",
            "--autopilot",
            "--headless-floor01-smoke",
        ])
        .unwrap();
        assert!(!a.manual && a.autopilot && a.headless);
        let a = args(&["--demo", "--autopilot", "--exit-after-ticks", "9"]).unwrap();
        assert!(!a.manual);
        assert!(
            args(&[
                "--demo",
                "--autopilot",
                "--capture-sequence",
                "5:1:1",
                "--capture-dir",
                "d"
            ])
            .is_ok()
        );
        // Beat and autopilot need --demo; the beat is 0 to 6; --demo replaces the others.
        assert!(args(&["--demo-beat", "3"]).is_err());
        assert!(args(&["--autopilot"]).is_err());
        assert!(args(&["--demo", "--demo-beat", "7"]).is_err());
        assert!(args(&["--demo", "--demo-beat"]).is_err());
        assert!(args(&["--demo", "--fixture", "cp01_kick"]).is_err());
        assert!(args(&["--demo", "--scenario", "idle"]).is_err());
        assert!(args(&["--demo", "--reference-frame"]).is_err());
    }
}
