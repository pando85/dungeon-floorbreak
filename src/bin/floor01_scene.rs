//! COMP-01: first integrated Floor 1 corridor scene (pixel layers only).
//!
//! Renders the ENV-01 kit layout with the approved Carl 52 px and Donut 40 px
//! atlases. There is no lighting, bloom or haze yet (COMP-02), no enemies and
//! no new mechanics. Simulation, camera and collision come from the pure
//! library modules; this binary only reads input, then draws that state.
//! Character art is fan-concept material and does not imply game distribution
//! rights.

use std::path::Path;
use std::process::ExitCode;

use bevy::{
    app::AppExit, asset::AssetPlugin, asset::LoadState, camera::ScalingMode, prelude::*,
    sprite::Anchor,
};
use dungeon_floorbreak::{
    actor_sprite::{ActorSpec, CARL, DONUT, actor_center, idle_frame},
    corridor::{Scenario, SceneSim},
    env_kit::{
        EnvKit, IRect, PieceDraw, animation_frame, parallax_copies, parallax_start, to_world_y,
    },
    input::RawInput,
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
    };
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--deck" => parsed.deck = true,
            "--headless-floor01-smoke" => parsed.headless = true,
            "--freeze" => parsed.freeze = true,
            "--debug-collision" => parsed.debug_collision = true,
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
    spec: ActorSpec,
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
    let freeze = args.freeze || args.capture_tick.is_some();
    let scenario = if freeze { None } else { args.scenario };
    App::new()
        .add_plugins(
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
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::srgb_u8(6, 5, 12)))
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(Sim(sim))
        .insert_resource(Frozen(freeze))
        .insert_resource(LiveScenario(scenario))
        .insert_resource(Setup { kit, args })
        .init_resource::<Controls>()
        .init_resource::<AfterExitTicks>()
        .add_systems(Startup, setup)
        .add_systems(Update, (capture_input, sync_visuals, watchdog).chain())
        .add_systems(FixedUpdate, fixed_step)
        .run();
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

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    setup: Res<Setup>,
) {
    let kit = &setup.kit;
    let args = &setup.args;
    let height = kit.height;
    let view_h = view_height(args.deck);

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
    ));

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
        ));
    }

    // Placed pieces (decals, structure, props, foreground).
    for piece in &kit.pieces {
        let mut entity = commands.spawn((
            atlas_sprite(&atlas(&piece.atlas), piece.rect),
            Anchor::TOP_LEFT,
            Transform::from_xyz(piece.x as f32, to_world_y(height, piece.y), piece.z),
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
            ActorVisual { spec, is_hero },
            Sprite::from_atlas_image(texture.clone(), TextureAtlas { layout, index: 0 }),
            Transform::from_xyz(0.0, 0.0, z),
        ));
    };
    spawn_actor(DONUT, &donut_texture, false, kit.actor_z);
    spawn_actor(CARL, &carl_texture, true, kit.actor_z + 0.1);

    // Optional collision overlay: merged solids and walking lines.
    let mut debug_entities = 0usize;
    if args.debug_collision {
        let overlay_z = kit.foreground_z + 1.0;
        for rect in &kit.solid_rects {
            commands.spawn((
                Sprite::from_color(
                    Color::srgba(0.95, 0.15, 0.2, 0.38),
                    Vec2::new(rect.w as f32, rect.h as f32),
                ),
                Anchor::TOP_LEFT,
                Transform::from_xyz(rect.x as f32, to_world_y(height, rect.y), overlay_z),
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

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn sync_visuals(
    sim: Res<Sim>,
    after_exit: Res<AfterExitTicks>,
    setup: Res<Setup>,
    mut camera: Query<&mut Transform, With<Camera2d>>,
    mut parallax: Query<
        (&Parallax, &mut Transform, &mut Visibility),
        (Without<Camera2d>, Without<ActorVisual>, Without<HeroBox>),
    >,
    mut torches: Query<(&Torch, &mut Sprite), Without<ActorVisual>>,
    mut actors: Query<
        (&ActorVisual, &mut Transform, &mut Sprite),
        (Without<Camera2d>, Without<Parallax>, Without<HeroBox>),
    >,
    mut hero_box: Query<
        &mut Transform,
        (
            With<HeroBox>,
            Without<Camera2d>,
            Without<Parallax>,
            Without<ActorVisual>,
        ),
    >,
) {
    let kit = &setup.kit;
    let state = &sim.0;
    let height = kit.height;
    let view_h = view_height(setup.args.deck);
    let (left, top) = (state.camera.left(), state.camera.top());
    let anim_tick = state.world.tick + after_exit.0;

    if let Ok(mut transform) = camera.single_mut() {
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

    let hero = &state.world.hero;
    let donut = &state.world.donut;
    for (actor, mut transform, mut sprite) in &mut actors {
        let (foot_x, foot_y, flip, phase) = if actor.is_hero {
            (hero.x, hero.y - HERO_HALF_HEIGHT, hero.facing < 0, 0)
        } else {
            (donut.x, donut.foot_y, donut.facing < 0, 2)
        };
        let (cx, cy) = actor_center(actor.spec, foot_x, foot_y, flip);
        transform.translation.x = cx;
        transform.translation.y = cy;
        sprite.flip_x = flip;
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = idle_frame(anim_tick, phase);
        }
    }

    for mut transform in &mut hero_box {
        transform.translation.x = hero.x.round();
        transform.translation.y = hero.y.round();
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
