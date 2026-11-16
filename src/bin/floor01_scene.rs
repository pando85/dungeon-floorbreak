//! Floor 1 corridor scene: COMP-01 integration plus the COMP-02 lighting pass.
//!
//! Renders the ENV-01 kit layout with the approved Carl 52 px and Donut 40 px
//! atlases. Pixel layers stay at the virtual resolution; the A1 lighting pass
//! (ambient, torch pools, shadow, haze, glow, flame bloom) runs at output
//! resolution (`light_pass`) and is presentation only. `--no-lighting` draws
//! the COMP-01 look. There are no enemies and no new mechanics. Simulation,
//! camera and collision come from the pure library modules; this binary only
//! reads input, then draws that state. Character art is fan-concept material
//! and does not imply game distribution rights.

use std::path::Path;
use std::process::ExitCode;

use bevy::{
    app::AppExit,
    asset::AssetPlugin,
    asset::LoadState,
    camera::{ScalingMode, visibility::RenderLayers},
    prelude::*,
    sprite::Anchor,
    window::{PresentMode, PrimaryWindow},
};
use dungeon_floorbreak::{
    actor_sprite::{ActorSpec, CARL, DONUT, actor_center, idle_frame},
    corridor::{Scenario, SceneSim},
    env_kit::{
        EnvKit, IRect, PieceDraw, animation_frame, parallax_copies, parallax_start, to_world_y,
    },
    input::RawInput,
    light_pass::{
        LAYER_DEBUG, LightPassPlugin, LightRig, LightingMaterial, PassFrame, SceneCamera,
        slot_layer, spawn_rig, update_rig,
    },
    lighting::{ShadowEllipse, ViewRect, contact_shadow},
    prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH},
};

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
    capture_tick: Option<u64>,
    scale: u32,
    /// Draw without the lighting pass (the COMP-01 look).
    no_lighting: bool,
    /// Review capture: camera and actors pinned to the ENV-01 lit preview frame.
    reference_frame: bool,
    /// Measure this many frame intervals after warm-up, print them and exit.
    perf_frames: Option<usize>,
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut parsed = Args {
        deck: false,
        headless: false,
        freeze: false,
        debug_collision: false,
        scenario: None,
        capture_tick: None,
        scale: 2,
        no_lighting: false,
        reference_frame: false,
        perf_frames: None,
    };
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--deck" => parsed.deck = true,
            "--headless-floor01-smoke" => parsed.headless = true,
            "--freeze" => parsed.freeze = true,
            "--debug-collision" => parsed.debug_collision = true,
            "--no-lighting" => parsed.no_lighting = true,
            "--reference-frame" => parsed.reference_frame = true,
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
    if parsed.capture_tick.is_some() && parsed.scenario.is_none() {
        return Err("--capture-tick needs --scenario".into());
    }
    if parsed.reference_frame && (parsed.scenario.is_some() || parsed.capture_tick.is_some()) {
        return Err("--reference-frame pins its own state; drop --scenario/--capture-tick".into());
    }
    Ok(parsed)
}

fn view_height(deck: bool) -> i32 {
    if deck { 400 } else { 360 }
}

/// Build the simulation in the requested state (scenario pre-simulated).
fn build_sim(kit: &EnvKit, args: &Args) -> Result<SceneSim, String> {
    let mut sim = SceneSim::new(kit, view_height(args.deck))?;
    if let (Some(scenario), Some(ticks)) = (args.scenario, args.capture_tick) {
        sim.run_scenario(scenario, ticks);
    }
    Ok(sim)
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
    if args.headless {
        return match headless_smoke(&kit, &args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                println!("FLOOR1_HEADLESS_FAILURE: {message}");
                ExitCode::from(1)
            }
        };
    }
    let sim = match build_sim(&kit, &args) {
        Ok(sim) => sim,
        Err(message) => return failure(&message),
    };
    run_window(kit, sim, args, &root);
    ExitCode::SUCCESS
}

// ------------------------------------------------------------------ resources

#[derive(Resource)]
struct Sim(SceneSim);

#[derive(Resource)]
struct Setup {
    kit: EnvKit,
    args: Args,
}

#[derive(Resource)]
struct Frozen(bool);

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
}

/// Presentation ticks since the simulation stopped at the exit. The sim tick
/// stops there (so state hashes stay stable), but torches and idle frames keep
/// animating from `sim tick + this`. Never read by the simulation.
#[derive(Resource, Default)]
struct AfterExitTicks(u64);

#[derive(Resource)]
struct LiveScenario(Option<Scenario>);

#[derive(Resource, Default)]
struct Controls {
    jump_tap: bool,
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

fn run_window(kit: EnvKit, sim: SceneSim, args: Args, root: &Path) {
    // Bevy 0.20 resolves a relative AssetPlugin path from the binary directory,
    // so give it an absolute directory below the repository root.
    let asset_directory = root.join("generated/art").to_string_lossy().into_owned();
    let window_size = (
        VIRTUAL_WIDTH as u32 * args.scale,
        view_height(args.deck) as u32 * args.scale,
    );
    let freeze = args.freeze || args.capture_tick.is_some() || args.reference_frame;
    let scenario = if freeze { None } else { args.scenario };
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
    });
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(AssetPlugin {
                file_path: asset_directory,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: TITLE.into(),
                    resolution: window_size.into(),
                    // ADR-0007: integer scaling only; the window is exactly
                    // virtual size x scale (letterboxing is deferred).
                    resizable: false,
                    present_mode,
                    ..default()
                }),
                ..default()
            }),
    )
    .insert_resource(ClearColor(Color::srgb_u8(6, 5, 12)))
    .insert_resource(Time::<Fixed>::from_hz(60.0))
    .insert_resource(Sim(sim))
    .insert_resource(Frozen(freeze))
    .insert_resource(LightingOn(lighting_on))
    .insert_resource(ReferencePose(reference))
    .insert_resource(LiveScenario(scenario))
    .insert_resource(Setup { kit, args })
    .init_resource::<Controls>()
    .init_resource::<AfterExitTicks>()
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            capture_input,
            sync_visuals,
            update_lighting,
            watchdog,
            perf_probe,
        )
            .chain(),
    )
    .add_systems(FixedUpdate, fixed_step);
    if lighting_on {
        app.add_plugins(LightPassPlugin);
    }
    if let Some(perf) = perf {
        app.insert_resource(perf);
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
    let carl_texture: Handle<Image> = asset_server.load(CARL.atlas);
    let donut_texture: Handle<Image> = asset_server.load(DONUT.atlas);

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
    let mut spawn_actor = |spec: ActorSpec, texture: &Handle<Image>, is_hero: bool, z: f32| {
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(spec.width as u32, spec.height as u32),
            4,
            1,
            None,
            None,
        ));
        commands.spawn((
            ActorVisual { is_hero },
            Sprite::from_atlas_image(texture.clone(), TextureAtlas { layout, index: 0 }),
            Transform::from_xyz(0.0, 0.0, z),
            layers_of("actors"),
        ));
    };
    spawn_actor(DONUT, &donut_texture, false, kit.actor_z);
    spawn_actor(CARL, &carl_texture, true, kit.actor_z + 0.1);

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

    handles.push((CARL.atlas.to_owned(), carl_texture));
    handles.push((DONUT.atlas.to_owned(), donut_texture));
    let tiles = kit.tiles.len();
    let pieces = kit.pieces.len();
    let counts = format!(
        "tiles={tiles} pieces={pieces} parallax_copies={strip_copies} actors=2 debug={debug_entities} sprites={} solids={}",
        tiles + pieces + strip_copies + 2 + debug_entities,
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
    controls.restart |= keys.just_pressed(KeyCode::KeyR);
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
    // The first connected gamepad drives the hero; no pad (hot-unplug) simply
    // means no pad input on the next tick, so nothing sticks.
    if let Some(pad) = pads.iter().next() {
        controls.jump_tap |= pad.just_pressed(GamepadButton::South);
        controls.restart |= pad.just_pressed(GamepadButton::North);
        if pad.just_pressed(GamepadButton::Start) {
            exit.write(AppExit::Success);
        }
    }
}

fn fixed_step(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    frozen: Res<Frozen>,
    scenario: Res<LiveScenario>,
    mut controls: ResMut<Controls>,
    mut sim: ResMut<Sim>,
    mut after_exit: ResMut<AfterExitTicks>,
) {
    let restart = std::mem::take(&mut controls.restart);
    let jump_tap = std::mem::take(&mut controls.jump_tap);
    if frozen.0 {
        return;
    }
    if restart {
        sim.0.reset();
        after_exit.0 = 0;
        println!("FLOOR1_RESTART");
    }
    let raw = if let Some(scenario) = scenario.0 {
        scenario.input(&sim.0.world)
    } else {
        let left = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft);
        let right = keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight);
        let mut raw = RawInput {
            move_x: f32::from(i8::from(right) - i8::from(left)),
            jump: keys.pressed(KeyCode::Space) || jump_tap,
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
        }
        raw
    };
    let was_complete = sim.0.world.complete;
    sim.0.step(raw);
    if sim.0.world.complete && !was_complete {
        println!("FLOOR1_EXIT_REACHED tick={}", sim.0.world.tick);
    }
    if was_complete {
        after_exit.0 += 1;
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

/// View top-left in layout px: the scroll camera, or the reference view.
fn view_origin(state: &SceneSim, setup: &Setup, reference: &ReferencePose) -> (i32, i32) {
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
    (state.camera.left(), state.camera.top())
}

/// Carl and Donut as drawn: the simulation state, or the reference pose.
fn actor_poses(
    state: &SceneSim,
    setup: &Setup,
    reference: &ReferencePose,
    anim_tick: u64,
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
                frame: 0,
            }
        };
        return [at(CARL, true, pose.carl_x), at(DONUT, false, pose.donut_x)];
    }
    let hero = &state.world.hero;
    let donut = &state.world.donut;
    [
        ActorPose {
            spec: CARL,
            is_hero: true,
            foot_x: hero.x,
            foot_y: hero.y - HERO_HALF_HEIGHT,
            flip: hero.facing < 0,
            frame: idle_frame(anim_tick, 0),
        },
        ActorPose {
            spec: DONUT,
            is_hero: false,
            foot_x: donut.x,
            foot_y: donut.foot_y,
            flip: donut.facing < 0,
            frame: idle_frame(anim_tick, 2),
        },
    ]
}

/// Presentation tick: torches, idle frames and light flicker. The reference
/// frame pins it to 0 (the preview's `time_ms = 0`).
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
    after_exit: Res<AfterExitTicks>,
    setup: Res<Setup>,
    reference: Res<ReferencePose>,
    mut cameras: Query<&mut Transform, With<SceneCamera>>,
    mut parallax: Query<
        (&Parallax, &mut Transform, &mut Visibility),
        (Without<SceneCamera>, Without<ActorVisual>, Without<HeroBox>),
    >,
    mut torches: Query<(&Torch, &mut Sprite), Without<ActorVisual>>,
    mut actors: Query<
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
    let (left, top) = view_origin(state, &setup, &reference);
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

    let poses = actor_poses(state, &setup, &reference, anim_tick);
    for (actor, mut transform, mut sprite) in &mut actors {
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

/// Derive this frame's lights from the view, the presentation tick and the
/// actors' positions, and hand them to the lighting stages. Reads simulation
/// state through shared references only; nothing flows back.
#[allow(clippy::too_many_arguments)]
fn update_lighting(
    rig: Option<Res<LightRig>>,
    sim: Res<Sim>,
    after_exit: Res<AfterExitTicks>,
    setup: Res<Setup>,
    reference: Res<ReferencePose>,
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
    let (left, top) = view_origin(state, &setup, &reference);
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
        for pose in actor_poses(state, &setup, &reference, tick) {
            // The shadow lies on the walking line under the actor (also while
            // it jumps), centred on the sprite box.
            let (centre_x, _) = actor_center(pose.spec, pose.foot_x, pose.foot_y, pose.flip);
            let line = kit
                .walking_line_at(pose.foot_x.round() as i32)
                .unwrap_or(kit.height - pose.foot_y.round() as i32);
            shadows.push(contact_shadow(
                profile,
                centre_x,
                line as f32,
                pose.spec.width as f32,
                view,
            ));
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

fn perf_probe(
    perf: Option<ResMut<Perf>>,
    time: Res<Time<Real>>,
    loading: Option<Res<Loading>>,
    window: Query<&Window, With<PrimaryWindow>>,
    lighting_on: Res<LightingOn>,
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
    if perf.intervals_ms.len() >= perf.target {
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
        }
        exit.write(AppExit::Success);
    }
}

fn watchdog(
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    time: Res<Time<Real>>,
    sim: Res<Sim>,
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
        println!("{}", sim.0.state_line());
        loading.reported = true;
    } else if time.elapsed_secs() > WATCHDOG_SECONDS {
        println!("FLOOR1_ASSET_FAILURE: textures not loaded within {WATCHDOG_SECONDS} s");
        exit.write(AppExit::error());
        loading.reported = true;
    }
}

// ------------------------------------------------------------ headless smoke

fn headless_smoke(kit: &EnvKit, args: &Args) -> Result<(), String> {
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
    let sim = build_sim(kit, args)?;
    println!("{}", sim.state_line());
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
