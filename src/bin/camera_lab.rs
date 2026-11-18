//! CAMERA-LAB: the Floor 01 Camera Scenarios Lab probe (Rust + Bevy).
//!
//! An isolated binary: six rooms composed from the approved Floor 01 env-kit
//! pieces, the real Carl and Donut production sprites, approved enemy atlases,
//! the existing combat / goblin / strike code through `camera_lab::sim`, and a
//! presentation-only camera operator with fixed 1.0 / 1.25 / 1.5 / 1.75
//! coverage or automatic shared framing. It does not touch `floor01_scene`.
//!
//! Rendering (ADR-0007 kept): pixel layers are drawn 1 texel per world pixel into
//! a coverage sized region of the slot targets and lit at output resolution by
//! the existing A1 pass with exact `textureLoad` sampling; the camera moves in
//! whole world pixels. At a non-integer output ratio (1.25x and 1.75x at 1080p,
//! 1.5x on the Deck) texels are 1 px wider on some columns: see
//! `docs/03-tech/camera-lab.md`.
//!
//! Run from the repository root after `scripts/export_assets.sh` and
//! `scripts/camera_lab_assets.sh`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use bevy::{
    app::AppExit,
    asset::{AssetPlugin, LoadState},
    camera::{Viewport, visibility::RenderLayers},
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    sprite::Anchor,
    window::{PresentMode, PrimaryWindow, WindowResolution},
};
use dungeon_floorbreak::{
    actor_sprite::{ActorSpec, actor_center, shadow_center_x},
    camera_lab::{
        art::{ArtSet, AtlasId, Clips, Pose},
        camera_op::{Aspect, CameraMode, effective_px, is_integer_scale, MAX_W, View},
        layout::build_kit,
        room::{PropKind, Room, load_rooms},
        script::{Script, script_inputs, scripts, solo_script},
        session::Session,
        sim::{
            CARL, DONUT, Hittable, LabEvent, LabInput, LabSim, Mode, PropMode, PropState,
            gate_open_at, gate_warning_at,
        },
    },
    combat::CombatEvent,
    env_kit::{Contract, EnvKit, IRect, animation_frame, parallax_copies, parallax_start, to_world_y},
    light_pass::{LightPassPlugin, LightRig, LightingMaterial, PassFrame, SceneCamera, slot_layer, spawn_rig, update_rig},
    lighting::{MAX_SHADOWS, ShadowEllipse, ViewRect, contact_shadow},
};

const TITLE: &str = "THE CRAWL - Camera Lab";
const WATCHDOG_SECONDS: f32 = 20.0;
const ACTOR_Z_CARL: f32 = 0.1;
const FOE_Z: f32 = 0.05;

// ------------------------------------------------------------------ arguments

#[derive(Debug, Clone)]
struct Args {
    room: usize,
    coverage: CameraMode,
    mode: Mode,
    output: (u32, u32),
    seed: u64,
    script: bool,
    headless: bool,
    ticks: Option<u64>,
    exit_after_ticks: Option<u64>,
    capture_sequence: Option<(u64, u64, u64)>,
    capture_dir: Option<PathBuf>,
    perf_frames: Option<usize>,
    /// Pre-simulate this many ticks of the script before the first frame.
    start_tick: u64,
    /// Place the actors for the 2P separation probe (see `separate`).
    separate: Option<i32>,
    /// No scripted or manual input: both actors stand still (separation probes).
    idle: bool,
}

fn parse_args(raw: &[String]) -> Result<Args, String> {
    let mut a = Args {
        room: 0,
        coverage: CameraMode::Auto,
        mode: Mode::Solo,
        output: (1920, 1080),
        seed: 1,
        script: false,
        headless: false,
        ticks: None,
        exit_after_ticks: None,
        capture_sequence: None,
        capture_dir: None,
        perf_frames: None,
        start_tick: 0,
        separate: None,
        idle: false,
    };
    let mut it = raw.iter().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().cloned().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--room" => {
                let n: usize = value()?.parse().map_err(|_| "--room 1..6")?;
                if !(1..=6).contains(&n) {
                    return Err("--room 1..6".into());
                }
                a.room = n - 1;
            }
            "--coverage" => {
                let v = value()?;
                a.coverage = if v == "auto" {
                    CameraMode::Auto
                } else {
                    let c: f32 = v.parse().map_err(|_| "--coverage auto|1|1.25|1.5|1.75")?;
                    if ![1.0, 1.25, 1.5, 1.75].contains(&c) {
                        return Err("--coverage auto|1|1.25|1.5|1.75".into());
                    }
                    CameraMode::Fixed(c)
                };
            }
            "--mode" => {
                a.mode = match value()?.as_str() {
                    "solo" => Mode::Solo,
                    "2p" => Mode::TwoPlayer,
                    _ => return Err("--mode solo|2p".into()),
                }
            }
            "--output" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or("--output WxH")?;
                a.output = (w.parse().map_err(|_| "--output WxH")?, h.parse().map_err(|_| "--output WxH")?);
            }
            "--seed" => a.seed = value()?.parse().map_err(|_| "--seed N")?,
            "--script" => a.script = true,
            "--idle" => a.idle = true,
            "--headless" => a.headless = true,
            "--ticks" => a.ticks = Some(value()?.parse().map_err(|_| "--ticks N")?),
            "--exit-after-ticks" => a.exit_after_ticks = Some(value()?.parse().map_err(|_| "--exit-after-ticks N")?),
            "--capture-sequence" => {
                let v = value()?;
                let p: Vec<&str> = v.split(':').collect();
                if p.len() != 3 {
                    return Err("--capture-sequence first:step:count".into());
                }
                let n = |s: &str| s.parse::<u64>().map_err(|_| "--capture-sequence first:step:count".to_owned());
                a.capture_sequence = Some((n(p[0])?, n(p[1])?, n(p[2])?));
            }
            "--capture-dir" => a.capture_dir = Some(PathBuf::from(value()?)),
            "--perf-frames" => a.perf_frames = Some(value()?.parse().map_err(|_| "--perf-frames N")?),
            "--start-tick" => a.start_tick = value()?.parse().map_err(|_| "--start-tick N")?,
            "--separate" => a.separate = Some(value()?.parse().map_err(|_| "--separate X")?),
            "--help" | "-h" => {
                return Err(
                    "camera_lab [--room 1..6] [--coverage auto|1|1.25|1.5|1.75] [--mode solo|2p] [--output WxH]\n\
                     [--script] [--seed N] [--headless --ticks N] [--exit-after-ticks N]\n\
                     [--capture-sequence first:step:count --capture-dir DIR] [--perf-frames N] [--start-tick N]"
                        .into(),
                );
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(a)
}

fn failure(message: &str) -> ExitCode {
    eprintln!("camera_lab: {message}");
    ExitCode::from(1)
}

fn inputs_of(sim: &LabSim, script: &Script, idle: bool) -> [LabInput; 2] {
    if idle { [LabInput::default(); 2] } else { script_inputs(sim, script) }
}

fn script_for(room: usize, mode: Mode) -> Script {
    if mode == Mode::Solo { solo_script(room) } else { scripts().remove(room) }
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().collect();
    let args = match parse_args(&raw) {
        Ok(a) => a,
        Err(m) => {
            eprintln!("{m}");
            return ExitCode::from(2);
        }
    };
    let root = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => return failure(&format!("no working directory: {e}")),
    };
    let rooms = match load_rooms(&root) {
        Ok(r) => r,
        Err(e) => return failure(&format!("{e} (run from the repository root)")),
    };
    let room = rooms[args.room].clone();
    let aspect = Aspect::from_output(args.output.0, args.output.1);
    let script = script_for(args.room, args.mode);
    let mut session = Session::new(&room, args.mode, if args.script { script.seed } else { args.seed }, args.coverage, aspect);
    if let Some(x) = args.separate {
        separate(&mut session, x);
        session.reframe();
    }
    if args.headless {
        return headless(&room, &mut session, &script, &args);
    }
    let (kit, contract, _plan) = match build_kit(&root, &room) {
        Ok(k) => k,
        Err(e) => return failure(&format!("{e} (run from the repository root)")),
    };
    let art = match ArtSet::load(&root) {
        Ok(a) => a,
        Err(e) => {
            return failure(&format!(
                "{e} (run bash scripts/export_assets.sh and bash scripts/camera_lab_assets.sh)"
            ));
        }
    };
    // Pre-simulate the script to the requested start tick.
    let start = args.capture_sequence.map_or(args.start_tick, |c| c.0.max(args.start_tick));
    if args.script || start > 0 {
        for _ in 0..start {
            let inputs = inputs_of(&session.sim, &script, args.idle);
            session.step(inputs);
        }
    }
    run_window(kit, contract, art, session, script, room, args, &root);
    ExitCode::SUCCESS
}

/// Put the actors far apart for the 2P separation probe: Carl stays, Donut is
/// placed at x on the floor (a test setup, not a gameplay rule).
fn separate(session: &mut Session, x: i32) {
    let h = session.sim.height;
    let top = session.sim.floor_top();
    let a = &mut session.sim.actors[DONUT];
    a.body.x = x as f32;
    a.body.y = top + a.body.hh + 1.0;
    let _ = h;
}

// ------------------------------------------------------------------- headless

fn headless(room: &Room, session: &mut Session, script: &Script, args: &Args) -> ExitCode {
    let ticks = args.ticks.unwrap_or(script.ticks);
    println!(
        "LAB_START room={} name={:?} mode={:?} seed={} ticks={} coverage={:?} aspect={:?} render={}x{} floor={}",
        room.id, room.name, script.mode, script.seed, ticks, args.coverage, session.aspect, room.render.0, room.render.1, room.floor_y
    );
    for _ in 0..ticks {
        let inputs = if args.script || script.mode == Mode::Solo || args.mode == Mode::TwoPlayer {
            script_inputs(&session.sim, script)
        } else {
            [LabInput::default(); 2]
        };
        let view = session.step(inputs);
        for e in session.sim.take_new_events() {
            println!("{e}");
        }
        if session.sim.tick % 60 == 0 {
            println!("{}", session.sim.state_line());
            println!(
                "LAB_CAMERA tick={} left={} top={} w={} h={} coverage={:.3} zoom_changes={}",
                session.sim.tick, view.left, view.top, view.w, view.h, session.camera.coverage(), session.camera.zoom_changes
            );
        }
    }
    println!("{}", session.sim.state_line());
    println!("LAB_FINAL_HASH {:016x}", session.sim.state_hash());
    ExitCode::SUCCESS
}

// ------------------------------------------------------------------ resources

#[derive(Resource)]
struct Lab {
    session: Session,
    script: Script,
    scripted: bool,
    frozen: bool,
    idle: bool,
    exit_after: Option<u64>,
    exit_reported: bool,
}

#[derive(Resource)]
struct Setup {
    kit: EnvKit,
    contract: Contract,
    art: ArtSet,
    room: Room,
    args: Args,
}

#[derive(Resource)]
struct Loading {
    handles: Vec<(String, Handle<Image>)>,
    reported: bool,
}

#[derive(Resource, Default)]
struct Taps {
    jump: [bool; 2],
    primary: [bool; 2],
    physical: [bool; 2],
    switch: bool,
    restart: bool,
}

#[derive(Resource)]
struct Perf {
    target: usize,
    warmup: usize,
    seen: usize,
    intervals_ms: Vec<f32>,
}

/// Kit atlas name -> image handle (props pick their piece's atlas each frame).
#[derive(Resource)]
struct AtlasHandles(std::collections::BTreeMap<String, Handle<Image>>);

#[derive(Resource, Default)]
struct Fx {
    events_seen: usize,
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
struct ActorSprite {
    actor: usize,
    melee: bool,
}

#[derive(Component)]
struct FoeSprite {
    index: usize,
}

#[derive(Component)]
struct PropSprite {
    prop: usize,
    /// Second part of the gate (stacked leaf).
    part: usize,
}

#[derive(Component)]
struct ProjSprite {
    slot: usize,
}

#[derive(Component)]
struct FxSprite {
    start: u64,
    rects: Vec<Rect>,
    durations: Vec<u32>,
}

// ---------------------------------------------------------------------- setup

fn image_rect(r: IRect) -> Rect {
    Rect::new(r.x as f32, r.y as f32, (r.x + r.w) as f32, (r.y + r.h) as f32)
}

fn atlas_sprite(image: &Handle<Image>, rect: IRect) -> Sprite {
    Sprite { image: image.clone(), rect: Some(image_rect(rect)), ..default() }
}

fn piece_of<'a>(contract: &'a Contract, id: &str) -> &'a dungeon_floorbreak::env_kit::PieceDef {
    contract.pieces.iter().find(|p| p.id == id).unwrap_or_else(|| panic!("kit piece {id}"))
}

#[allow(clippy::too_many_arguments)]
fn run_window(
    kit: EnvKit,
    contract: Contract,
    art: ArtSet,
    session: Session,
    script: Script,
    room: Room,
    args: Args,
    root: &Path,
) {
    let asset_directory = root.join("generated/art").to_string_lossy().into_owned();
    let frozen = args.capture_sequence.is_some();
    let present_mode = if args.perf_frames.is_some() { PresentMode::AutoNoVsync } else { PresentMode::AutoVsync };
    let lab = Lab {
        session,
        scripted: args.script || args.capture_sequence.is_some() || args.perf_frames.is_some() || args.exit_after_ticks.is_some(),
        script,
        frozen,
        idle: args.idle,
        exit_after: args.exit_after_ticks,
        exit_reported: false,
    };
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(AssetPlugin { file_path: asset_directory, ..default() })
            .disable::<bevy::audio::AudioPlugin>()
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: TITLE.into(),
                    resolution: WindowResolution::new(args.output.0, args.output.1).with_scale_factor_override(1.0),
                    resizable: false,
                    present_mode,
                    ..default()
                }),
                ..default()
            }),
    )
    .insert_resource(ClearColor(Color::srgb_u8(6, 5, 12)))
    .insert_resource(Time::<Fixed>::from_hz(60.0))
    .insert_resource(lab)
    .insert_resource(Setup { kit, contract, art, room, args: args.clone() })
    .init_resource::<Taps>()
    .init_resource::<Fx>()
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (capture_input, sync_visuals, spawn_fx, update_lighting, watchdog, perf_probe, capture_sequence).chain(),
    )
    .add_systems(FixedUpdate, fixed_step)
    .add_plugins(LightPassPlugin);
    if let Some(target) = args.perf_frames {
        app.insert_resource(Perf { target, warmup: 120, seen: 0, intervals_ms: Vec::with_capacity(target) });
    }
    if let Some((first, step, count)) = args.capture_sequence {
        app.insert_resource(CaptureSequence {
            _first: first,
            step,
            count,
            dir: args.capture_dir.clone().unwrap_or_else(|| PathBuf::from("artifacts/camera-lab")),
            index: 0,
            phase: CapturePhase::WaitReady,
        });
    }
    app.run();
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LightingMaterial>>,
    setup: Res<Setup>,
    lab: Res<Lab>,
) {
    let _ = &mut layouts;
    let kit = &setup.kit;
    let height = kit.height;
    let lighting = kit.lighting.as_ref().expect("the lab layouts carry the A1 lighting profile");
    let layers_of = |layer: &str| -> RenderLayers {
        lighting.plan.layer_slot.get(layer).map_or_else(RenderLayers::default, |slot| slot_layer(*slot))
    };
    let aspect = lab.session.aspect;
    let max_h = aspect.height_for(MAX_W);
    // Slot targets are allocated for the largest coverage; each frame renders the
    // current coverage 1:1 into the top-left region (see `sync_visuals`).
    let rig = spawn_rig(&mut commands, &mut images, &mut meshes, &mut materials, lighting, UVec2::new(MAX_W as u32, max_h as u32));
    commands.insert_resource(rig);

    let mut handles: Vec<(String, Handle<Image>)> = Vec::new();
    for (name, asset) in &kit.atlases {
        handles.push((name.clone(), asset_server.load(asset.clone())));
    }
    let kit_handles = handles.clone();
    commands.insert_resource(AtlasHandles(kit_handles.iter().cloned().collect()));
    let atlas = |name: &str| -> Handle<Image> {
        kit_handles.iter().find(|(n, _)| n == name).map(|(_, h)| h.clone()).unwrap_or_else(|| panic!("atlas {name}"))
    };

    for tile in &kit.tiles {
        commands.spawn((
            atlas_sprite(&atlas(&tile.atlas), tile.rect),
            Anchor::TOP_LEFT,
            Transform::from_xyz((tile.col * kit.tile) as f32, to_world_y(height, tile.row * kit.tile), tile.z),
            layers_of(&tile.layer),
        ));
    }
    for piece in &kit.pieces {
        let mut e = commands.spawn((
            atlas_sprite(&atlas(&piece.atlas), piece.rect),
            Anchor::TOP_LEFT,
            Transform::from_xyz(piece.x as f32, to_world_y(height, piece.y), piece.z),
            layers_of(&piece.layer),
        ));
        if piece.frames.len() > 1 {
            e.insert(Torch {
                rects: piece.frames.iter().map(|(r, _)| image_rect(*r)).collect(),
                durations: piece.frames.iter().map(|(_, d)| *d).collect(),
                offset: piece.frame_offset,
            });
        }
    }
    let mut strip_copies = 0usize;
    for (si, strip) in kit.strips.iter().enumerate() {
        let copies = (MAX_W / strip.rect.w + 3) as usize;
        for copy in 0..copies {
            commands.spawn((
                atlas_sprite(&atlas(&strip.atlas), strip.rect),
                Anchor::TOP_LEFT,
                Transform::from_xyz(0.0, to_world_y(height, strip.y), strip.z),
                Parallax { strip: si, copy },
                layers_of(&strip.layer),
            ));
            strip_copies += 1;
        }
    }

    // Actor and enemy textures.
    let mut tex: std::collections::BTreeMap<AtlasId, Handle<Image>> = Default::default();
    for (id, _) in setup.art.clips.iter() {
        let h: Handle<Image> = asset_server.load(id.image().to_owned());
        handles.push((id.image().to_owned(), h.clone()));
        tex.insert(*id, h);
    }
    let sprite_of = |id: AtlasId, z: f32, layers: RenderLayers| {
        (
            Sprite { image: tex[&id].clone(), rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)), ..default() },
            Transform::from_xyz(0.0, 0.0, z),
            Visibility::Hidden,
            layers,
        )
    };
    let actors_layer = layers_of("actors");
    for (i, (loco, melee)) in [(AtlasId::CarlLoco, AtlasId::CarlMelee), (AtlasId::DonutLoco, AtlasId::DonutMelee)].into_iter().enumerate() {
        let z = kit.actor_z + if i == CARL { ACTOR_Z_CARL } else { 0.0 };
        commands.spawn((ActorSprite { actor: i, melee: false }, sprite_of(loco, z, actors_layer.clone())));
        commands.spawn((ActorSprite { actor: i, melee: true }, sprite_of(melee, z, actors_layer.clone())));
    }
    let sim = &lab.session.sim;
    for (index, t) in sim.targets.iter().enumerate() {
        let id = match t {
            Hittable::Goblin(_) => {
                if index % 2 == 0 { AtlasId::GoblinA } else { AtlasId::GoblinB }
            }
            Hittable::Foe(f) => match f.spec.kind {
                dungeon_floorbreak::camera_lab::room::EnemyKind::Llama => AtlasId::Llama,
                dungeon_floorbreak::camera_lab::room::EnemyKind::Engineer => AtlasId::Engineer,
                dungeon_floorbreak::camera_lab::room::EnemyKind::Scatterer => AtlasId::Scatterer,
                dungeon_floorbreak::camera_lab::room::EnemyKind::Hoarder => AtlasId::Hoarder,
                dungeon_floorbreak::camera_lab::room::EnemyKind::Trog => AtlasId::Trog,
                dungeon_floorbreak::camera_lab::room::EnemyKind::Juicer => AtlasId::Juicer,
                _ => AtlasId::Swine,
            },
            Hittable::Prop(_) => continue,
        };
        commands.spawn((
            FoeSprite { index },
            sprite_of(id, kit.actor_z + FOE_Z - index as f32 * 0.001, actors_layer.clone()),
        ));
    }
    // Props: a sprite per prop (two leaves for the gate), art chosen per state.
    for (index, t) in sim.targets.iter().enumerate() {
        let Hittable::Prop(p) = t else { continue };
        let prop = index - sim.room.enemies.len();
        let parts = if p.def.kind == PropKind::Gate { 3 } else { 1 };
        for part in 0..parts {
            let piece = piece_of(&setup.contract, "loose_block_seated");
            commands.spawn((
                PropSprite { prop, part },
                Sprite { image: atlas(&piece.atlas), rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)), ..default() },
                Transform::from_xyz(0.0, 0.0, layer_z_of(kit, "prop") + 0.5),
                Visibility::Hidden,
                layers_of("prop"),
            ));
        }
    }
    // Projectile pool.
    for slot in 0..16 {
        let piece = piece_of(&setup.contract, "loose_block_seated");
        commands.spawn((
            ProjSprite { slot },
            Sprite { image: atlas(&piece.atlas), custom_size: Some(Vec2::new(5.0, 5.0)), color: Color::srgb(0.7, 1.0, 0.35), rect: None, ..default() },
            Transform::from_xyz(0.0, 0.0, kit.actor_z + 0.3),
            Visibility::Hidden,
            actors_layer.clone(),
        ));
    }
    let _ = (&setup.room, atlas_clone(&handles));
    handles.push(("fx".into(), asset_server.load("vfx_hit_spark_metal.png".to_owned())));
    println!(
        "LAB_ENTITIES room={} tiles={} pieces={} parallax_copies={} foes={} props={} render={}x{}",
        setup.room.id,
        kit.tiles.len(),
        kit.pieces.len(),
        strip_copies,
        sim.room.enemies.len(),
        sim.room.props.len(),
        kit.width,
        kit.height
    );
    commands.insert_resource(Loading { handles, reported: false });
}

fn atlas_clone(h: &[(String, Handle<Image>)]) -> usize {
    h.len()
}

fn layer_z_of(kit: &EnvKit, layer: &str) -> f32 {
    // Draw order of the contract layers: prop 6, foreground 8 (kit.foreground_z).
    match layer {
        "prop" => kit.actor_z - 1.0,
        _ => kit.foreground_z,
    }
}

// -------------------------------------------------------------------- input

fn capture_input(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut taps: ResMut<Taps>,
    mut lab: ResMut<Lab>,
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
    taps.restart |= keys.just_pressed(KeyCode::KeyR);
    // Player 1 keyboard (also the solo player): A D move, W / Space jump, F or J
    // primary, G or K physical, Q / Tab switch. Player 2 keyboard: arrows, Up
    // jump, M primary, N physical.
    taps.jump[0] |= keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::KeyW);
    taps.primary[0] |= keys.just_pressed(KeyCode::KeyF) || keys.just_pressed(KeyCode::KeyJ);
    taps.physical[0] |= keys.just_pressed(KeyCode::KeyG) || keys.just_pressed(KeyCode::KeyK);
    taps.switch |= keys.just_pressed(KeyCode::KeyQ) || keys.just_pressed(KeyCode::Tab);
    taps.jump[1] |= keys.just_pressed(KeyCode::ArrowUp);
    taps.primary[1] |= keys.just_pressed(KeyCode::KeyM);
    taps.physical[1] |= keys.just_pressed(KeyCode::KeyN);
    for (i, pad) in pads.iter().take(2).enumerate() {
        taps.jump[i] |= pad.just_pressed(GamepadButton::South);
        taps.primary[i] |= pad.just_pressed(GamepadButton::West);
        taps.physical[i] |= pad.just_pressed(GamepadButton::North);
        if i == 0 {
            taps.switch |= pad.just_pressed(GamepadButton::LeftTrigger);
        }
    }
    // Coverage hotkeys: 0 auto, 1..4 fixed 1.0 / 1.25 / 1.5 / 1.75 (a live
    // switch rebuilds the operator at the current framing).
    let pick = [
        (KeyCode::Digit0, CameraMode::Auto),
        (KeyCode::Digit1, CameraMode::Fixed(1.0)),
        (KeyCode::Digit2, CameraMode::Fixed(1.25)),
        (KeyCode::Digit3, CameraMode::Fixed(1.5)),
        (KeyCode::Digit4, CameraMode::Fixed(1.75)),
    ];
    for (key, mode) in pick {
        if keys.just_pressed(key) {
            let aspect = lab.session.aspect;
            let room = lab.session.sim.room.clone();
            let (subjects, threat, hold) = dungeon_floorbreak::camera_lab::session::camera_input_of(&lab.session.sim);
            lab.session.camera = dungeon_floorbreak::camera_lab::camera_op::CameraOperator::new(
                mode,
                aspect,
                dungeon_floorbreak::camera_lab::camera_op::Bounds { w: room.render.0, h: room.render.1 },
                &dungeon_floorbreak::camera_lab::camera_op::CameraInput { subjects: &subjects, threat, impact_hold: hold },
            );
            println!("LAB_COVERAGE {mode:?}");
        }
    }
}

fn manual_inputs(keys: &ButtonInput<KeyCode>, pads: &Query<&Gamepad>, taps: &mut Taps, mode: Mode) -> [LabInput; 2] {
    let axis = |neg: &[KeyCode], pos: &[KeyCode]| -> f32 {
        let l = neg.iter().any(|k| keys.pressed(*k));
        let r = pos.iter().any(|k| keys.pressed(*k));
        f32::from(i8::from(r) - i8::from(l))
    };
    let mut a = LabInput {
        move_x: axis(&[KeyCode::KeyA], &[KeyCode::KeyD]),
        jump: keys.pressed(KeyCode::Space) || keys.pressed(KeyCode::KeyW) || taps.jump[0],
        primary: keys.pressed(KeyCode::KeyF) || keys.pressed(KeyCode::KeyJ) || taps.primary[0],
        physical: keys.pressed(KeyCode::KeyG) || keys.pressed(KeyCode::KeyK) || taps.physical[0],
        special: keys.pressed(KeyCode::KeyL),
        switch: std::mem::take(&mut taps.switch),
    };
    let mut b = LabInput {
        move_x: axis(&[KeyCode::ArrowLeft], &[KeyCode::ArrowRight]),
        jump: keys.pressed(KeyCode::ArrowUp) || taps.jump[1],
        primary: keys.pressed(KeyCode::KeyM) || taps.primary[1],
        physical: keys.pressed(KeyCode::KeyN) || taps.physical[1],
        ..LabInput::default()
    };
    if mode == Mode::Solo && a.move_x == 0.0 {
        a.move_x = axis(&[KeyCode::ArrowLeft], &[KeyCode::ArrowRight]);
        a.jump |= keys.pressed(KeyCode::ArrowUp);
    }
    for (i, pad) in pads.iter().take(2).enumerate() {
        let stick = pad.left_stick().x;
        let x = if stick.abs() > 0.18 { stick } else { pad.dpad().x };
        let target = if mode == Mode::Solo { &mut a } else if i == 0 { &mut a } else { &mut b };
        if x != 0.0 {
            target.move_x = x;
        }
        target.jump |= pad.pressed(GamepadButton::South);
        target.primary |= pad.pressed(GamepadButton::West);
        target.physical |= pad.pressed(GamepadButton::North);
        target.special |= pad.pressed(GamepadButton::East);
        if i == 0 {
            a.switch |= pad.just_pressed(GamepadButton::LeftTrigger);
        }
        if mode == Mode::Solo {
            break;
        }
    }
    *taps = Taps { switch: false, restart: taps.restart, ..Taps::default() };
    [a, b]
}

fn fixed_step(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut taps: ResMut<Taps>,
    mut lab: ResMut<Lab>,
    loading: Option<Res<Loading>>,
    mut exit: MessageWriter<AppExit>,
) {
    if lab.frozen || loading.is_some_and(|l| !l.reported) {
        return;
    }
    if let Some(limit) = lab.exit_after
        && lab.session.sim.tick >= limit
    {
        return;
    }
    let restart = std::mem::take(&mut taps.restart);
    if restart {
        let room = lab.session.sim.room.clone();
        let (mode, seed) = (lab.session.sim.mode, lab.session.sim.seed);
        let (cam, aspect) = (lab.session.camera.mode(), lab.session.aspect);
        lab.session = Session::new(&room, mode, seed, cam, aspect);
        println!("LAB_RESTART");
    }
    let inputs = if lab.idle {
        [LabInput::default(); 2]
    } else if lab.scripted {
        script_inputs(&lab.session.sim, &lab.script)
    } else {
        let mode = lab.session.sim.mode;
        manual_inputs(&keys, &pads, &mut taps, mode)
    };
    lab.session.step(inputs);
    for e in lab.session.sim.take_new_events() {
        println!("{e}");
    }
    if let Some(limit) = lab.exit_after
        && lab.session.sim.tick >= limit
        && !lab.exit_reported
    {
        println!("{}", lab.session.sim.state_line());
        println!("LAB_FINAL_HASH {:016x}", lab.session.sim.state_hash());
        lab.exit_reported = true;
        exit.write(AppExit::Success);
    }
}

// -------------------------------------------------------------- presentation

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn sync_visuals(
    lab: Res<Lab>,
    setup: Res<Setup>,
    atlas_handles: Option<Res<AtlasHandles>>,
    mut cameras: Query<(&mut Camera, &mut Projection, &mut Transform), With<SceneCamera>>,
    mut parallax: Query<(&Parallax, &mut Transform, &mut Visibility), (Without<SceneCamera>, Without<ActorSprite>)>,
    mut torches: Query<(&Torch, &mut Sprite), (Without<ActorSprite>, Without<FoeSprite>, Without<PropSprite>, Without<ProjSprite>, Without<FxSprite>)>,
    mut actor_sprites: Query<(&ActorSprite, &mut Transform, &mut Sprite, &mut Visibility), (Without<SceneCamera>, Without<Parallax>, Without<FoeSprite>, Without<PropSprite>, Without<ProjSprite>)>,
    mut foe_sprites: Query<(&FoeSprite, &mut Transform, &mut Sprite, &mut Visibility), (Without<SceneCamera>, Without<Parallax>, Without<ActorSprite>, Without<PropSprite>, Without<ProjSprite>)>,
    mut prop_sprites: Query<(&PropSprite, &mut Transform, &mut Sprite, &mut Visibility), (Without<SceneCamera>, Without<Parallax>, Without<ActorSprite>, Without<FoeSprite>, Without<ProjSprite>)>,
    mut proj_sprites: Query<(&ProjSprite, &mut Transform, &mut Visibility), (Without<SceneCamera>, Without<Parallax>, Without<ActorSprite>, Without<FoeSprite>, Without<PropSprite>)>,
    mut fx_sprites: Query<(&FxSprite, &mut Sprite), (Without<ActorSprite>, Without<FoeSprite>, Without<PropSprite>, Without<Torch>)>,
) {
    let kit = &setup.kit;
    let sim = &lab.session.sim;
    let view = lab.session.view();
    let height = kit.height;
    let tick = sim.tick;
    // Every scene camera sits on the same whole-pixel position and renders the
    // current coverage 1:1 into the top-left region of its (largest-coverage)
    // target.
    for (mut camera, mut projection, mut tf) in &mut cameras {
        camera.viewport = Some(Viewport {
            physical_position: UVec2::ZERO,
            physical_size: UVec2::new(view.w as u32, view.h as u32),
            ..default()
        });
        if let Projection::Orthographic(o) = projection.as_mut() {
            o.scaling_mode = bevy::camera::ScalingMode::Fixed { width: view.w as f32, height: view.h as f32 };
        }
        tf.translation.x = (view.left + view.w / 2) as f32;
        tf.translation.y = to_world_y(height, view.top + view.h / 2);
    }
    for (p, mut tf, mut vis) in &mut parallax {
        let strip = &kit.strips[p.strip];
        let start = parallax_start(strip.x_origin, view.left, strip.factor);
        let copies = parallax_copies(start, strip.rect.w, view.left, view.w);
        match copies.get(p.copy) {
            Some(x) => {
                tf.translation.x = *x as f32;
                *vis = Visibility::Inherited;
            }
            None => *vis = Visibility::Hidden,
        }
    }
    for (t, mut sprite) in &mut torches {
        let frame = animation_frame(&t.durations, t.offset, tick);
        sprite.rect = Some(t.rects[frame]);
    }

    let art = &setup.art;
    let spec_of = |c: &Clips| ActorSpec {
        name: "LAB",
        width: c.frame_w,
        height: c.frame_h,
        pivot_x: c.pivot.0,
        pivot_y: c.pivot.1,
        shadow_width: c.frame_w,
        shadow_pivot_x: c.pivot.0,
    };
    let place = |pose: &Pose, tf: &mut Transform, sprite: &mut Sprite, vis: &mut Visibility, shown: bool| {
        let c = art.get(pose.atlas);
        let (cx, cy) = actor_center(spec_of(c), pose.foot_x, pose.foot_y, pose.flip);
        tf.translation.x = cx;
        tf.translation.y = cy;
        sprite.flip_x = pose.flip;
        let (r, _) = c.frames[pose.frame];
        sprite.rect = Some(Rect::new(r.x as f32, r.y as f32, (r.x + r.w) as f32, (r.y + r.h) as f32));
        sprite.color = if pose.flash { Color::linear_rgb(2.0, 2.0, 2.0) } else { Color::WHITE };
        *vis = if shown { Visibility::Inherited } else { Visibility::Hidden };
    };
    for (a, mut tf, mut sprite, mut vis) in &mut actor_sprites {
        let pose = art.actor_pose(sim, a.actor);
        let melee_pose = matches!(pose.atlas, AtlasId::CarlMelee | AtlasId::DonutMelee);
        let flash = sim.actors[a.actor].invuln > 0 && (sim.tick / 3) % 2 == 0 && sim.actors[a.actor].invuln > 40;
        let pose = Pose { flash, ..pose };
        place(&pose, &mut tf, &mut sprite, &mut vis, melee_pose == a.melee);
    }
    for (f, mut tf, mut sprite, mut vis) in &mut foe_sprites {
        let (pose, hit) = match &sim.targets[f.index] {
            Hittable::Goblin(g) => (art.goblin_pose(sim, f.index, g), g.hit_stop > 0),
            Hittable::Foe(foe) => (art.foe_pose(sim, f.index, foe), foe.hit_stop > 0),
            Hittable::Prop(_) => continue,
        };
        let pose = Pose { flash: hit && tick.saturating_sub(sim.last_hit_tick) < 3, ..pose };
        place(&pose, &mut tf, &mut sprite, &mut vis, true);
    }
    // Props.
    let piece = |id: &str| piece_of(&setup.contract, id);
    for (p, mut tf, mut sprite, mut vis) in &mut prop_sprites {
        let Hittable::Prop(ps) = &sim.targets[sim.room.enemies.len() + p.prop] else { continue };
        let (id, shown) = prop_art(ps, p.part);
        let Some(id) = id else {
            *vis = Visibility::Hidden;
            continue;
        };
        let def = piece(id);
        if let Some(h) = atlas_handles.as_ref().and_then(|a| a.0.get(&def.atlas)) {
            sprite.image = h.clone();
        }
        sprite.rect = Some(Rect::new(def.rect[0] as f32, def.rect[1] as f32, (def.rect[0] + def.rect[2]) as f32, (def.rect[1] + def.rect[3]) as f32));
        let (w, h) = (def.rect[2] as f32, def.rect[3] as f32);
        let r = ps.rect;
        let cx = r.left + r.width / 2.0;
        // Bottom-centre on the prop (a falling weight: its current bottom).
        let (x, bottom) = if ps.def.kind == PropKind::Gate {
            let lift = gate_open_at(sim.tick) * (r.height - 6.0);
            (cx, r.bottom + lift + p.part as f32 * 88.0)
        } else if ps.def.kind == PropKind::HangingWeight {
            (cx, r.bottom)
        } else {
            (cx, r.bottom)
        };
        tf.translation.x = (x).round();
        tf.translation.y = (bottom + h / 2.0).round();
        let _ = w;
        *vis = if shown { Visibility::Inherited } else { Visibility::Hidden };
    }
    for (pj, mut tf, mut vis) in &mut proj_sprites {
        match sim.projectiles.get(pj.slot) {
            Some(p) => {
                tf.translation.x = p.x.round();
                tf.translation.y = p.y.round();
                *vis = Visibility::Inherited;
            }
            None => *vis = Visibility::Hidden,
        }
    }
    for (fx, mut sprite) in &mut fx_sprites {
        let age = tick.saturating_sub(fx.start);
        let mut idx = 0usize;
        let mut left = age as u32;
        for (i, d) in fx.durations.iter().enumerate() {
            if left < *d {
                idx = i;
                break;
            }
            left -= *d;
            idx = i;
        }
        sprite.rect = Some(fx.rects[idx.min(fx.rects.len() - 1)]);
    }
}

/// Art piece of a prop in its current state, and whether it is drawn.
fn prop_art(ps: &PropState, part: usize) -> (Option<&'static str>, bool) {
    use PropMode::*;
    match (ps.def.kind, ps.mode) {
        (PropKind::Crate, Intact) | (PropKind::Weight, Intact) => (Some("loose_block_seated"), true),
        (PropKind::Crate, _) | (PropKind::Weight, _) => (Some("loose_block_landed"), true),
        (PropKind::WeakWall, Intact) => (Some("timber_set_intact"), true),
        (PropKind::WeakWall, _) => (Some("timber_set_smashed"), true),
        (PropKind::Support, Intact) => (
            Some(match ps.hp {
                3.. => "shore_post_intact",
                2 => "shore_post_damaged_1",
                _ => "shore_post_damaged_2",
            }),
            true,
        ),
        (PropKind::Support, _) => (Some("shore_post_broken"), true),
        (PropKind::HangingWeight, Intact) => (Some("loose_block_seated"), true),
        (PropKind::HangingWeight, Falling) => (Some("loose_block_falling"), true),
        (PropKind::HangingWeight, _) => (Some("loose_block_landed"), true),
        (PropKind::Junk, Intact) => (Some("rubble_a"), true),
        (PropKind::Junk, _) => (Some("rubble_b"), true),
        (PropKind::Gate, _) => (Some("door_timber"), part < 2),
    }
}

/// Presentation effects from sim events: hit sparks, breakage and dust.
#[allow(clippy::too_many_arguments)]
fn spawn_fx(
    mut commands: Commands,
    mut fx: ResMut<Fx>,
    lab: Res<Lab>,
    setup: Res<Setup>,
    asset_server: Res<AssetServer>,
) {
    let sim = &lab.session.sim;
    let kit = &setup.kit;
    let lighting = kit.lighting.as_ref().unwrap();
    let actors = lighting.plan.layer_slot.get("actors").map_or_else(RenderLayers::default, |s| slot_layer(*s));
    for event in sim.events.iter().skip(fx.events_seen) {
        match *event {
            LabEvent::Combat(CombatEvent::Hit { target, tick, .. }) => {
                let (cx, cy) = sim.targets[target as usize].centre_for_fx();
                let c = setup.art.get(AtlasId::HitSpark);
                let frames: Vec<(Rect, u32)> = c
                    .tags
                    .get("hit_spark")
                    .map(|(a, b)| (*a..=*b).collect::<Vec<_>>())
                    .unwrap_or_else(|| (0..c.frames.len()).collect())
                    .into_iter()
                    .map(|i| {
                        let (r, d) = c.frames[i];
                        (Rect::new(r.x as f32, r.y as f32, (r.x + r.w) as f32, (r.y + r.h) as f32), d)
                    })
                    .collect();
                spawn_fx_sprite(&mut commands, asset_server.load("vfx_hit_spark_metal.png".to_owned()), tick, frames, (cx.round(), cy.round()), kit.actor_z + 0.4, actors.clone());
            }
            LabEvent::PropBroken { tick, prop, .. } => {
                if let Hittable::Prop(p) = &sim.targets[sim.room.enemies.len() + prop as usize] {
                    let def = piece_of(&setup.contract, "timber_splinter_burst");
                    let (cx, cy) = p.rect.center();
                    spawn_piece_fx(&mut commands, &asset_server, kit, def, tick, (cx.round(), cy.round()), actors.clone());
                }
            }
            LabEvent::WeightLanded { tick, prop, x } => {
                let def = piece_of(&setup.contract, "block_impact_dust");
                let floor = sim.floor_top();
                let _ = prop;
                spawn_piece_fx(&mut commands, &asset_server, kit, def, tick, (x as f32, floor + 20.0), actors.clone());
            }
            _ => {}
        }
    }
    fx.events_seen = sim.events.len();
}

trait CentreForFx {
    fn centre_for_fx(&self) -> (f32, f32);
}

impl CentreForFx for Hittable {
    fn centre_for_fx(&self) -> (f32, f32) {
        use dungeon_floorbreak::combat::StrikeTarget;
        self.centre()
    }
}

fn spawn_fx_sprite(
    commands: &mut Commands,
    image: Handle<Image>,
    start: u64,
    frames: Vec<(Rect, u32)>,
    at: (f32, f32),
    z: f32,
    layers: RenderLayers,
) {
    let rects: Vec<Rect> = frames.iter().map(|f| f.0).collect();
    let durations: Vec<u32> = frames.iter().map(|f| ((f.1 as f32) * 0.06).round().max(1.0) as u32).collect();
    commands.spawn((
        FxSprite { start, rects: rects.clone(), durations },
        Sprite { image, rect: Some(rects[0]), ..default() },
        Transform::from_xyz(at.0, at.1, z),
        layers,
    ));
}

fn spawn_piece_fx(
    commands: &mut Commands,
    asset_server: &AssetServer,
    kit: &EnvKit,
    def: &dungeon_floorbreak::env_kit::PieceDef,
    start: u64,
    at: (f32, f32),
    layers: RenderLayers,
) {
    let Some(anim) = def.animation.as_ref() else { return };
    let asset = kit.atlases[&def.atlas].clone();
    let frames: Vec<(Rect, u32)> = anim
        .frames
        .iter()
        .map(|f| (Rect::new(f.frame.x as f32, f.frame.y as f32, (f.frame.x + f.frame.w) as f32, (f.frame.y + f.frame.h) as f32), f.duration))
        .collect();
    let rects: Vec<Rect> = frames.iter().map(|f| f.0).collect();
    // The piece durations are in milliseconds.
    let durations: Vec<u32> = frames.iter().map(|f| ((f.1 as f32) * 0.06).round().max(1.0) as u32).collect();
    commands.spawn((
        FxSprite { start, rects: rects.clone(), durations },
        Sprite { image: asset_server.load(asset), rect: Some(rects[0]), ..default() },
        Transform::from_xyz(at.0, at.1, kit.actor_z + 0.35),
        layers,
    ));
}

#[allow(clippy::too_many_arguments)]
fn update_lighting(
    rig: Option<Res<LightRig>>,
    lab: Res<Lab>,
    setup: Res<Setup>,
    windows: Query<&Window, With<PrimaryWindow>>,
    materials: Option<ResMut<Assets<LightingMaterial>>>,
    mut dropped_reported: Local<bool>,
) {
    let (Some(rig), Some(mut materials), Ok(window)) = (rig, materials, windows.single()) else { return };
    let kit = &setup.kit;
    let Some(scene) = kit.lighting.as_ref() else { return };
    let sim = &lab.session.sim;
    let v = lab.session.view();
    let view = ViewRect { left: v.left, top: v.top, w: v.w, h: v.h };
    let (gathered, dropped) = scene.gather(kit, view, sim.tick);
    if dropped > 0 && !*dropped_reported {
        println!("LAB_LIGHTING_WARNING: {dropped} lights over the pass limit were dropped");
        *dropped_reported = true;
    }
    let mut shadows: Vec<ShadowEllipse> = Vec::new();
    if let Some(profile) = scene.profile.actor_contact_shadow.as_ref() {
        let height = sim.height;
        let mut put = |x: f32, feet_y: f32, width: f32, flip: bool, spec_pivot: (i32, i32)| {
            if shadows.len() >= MAX_SHADOWS {
                return;
            }
            let line = height - sim.ground_below(x, feet_y);
            let spec = ActorSpec {
                name: "LAB",
                width: width as i32,
                height: 0,
                pivot_x: spec_pivot.0,
                pivot_y: 0,
                shadow_width: width as i32,
                shadow_pivot_x: spec_pivot.0,
            };
            shadows.push(contact_shadow(profile, shadow_center_x(spec, x, flip), line, width, view));
        };
        for i in 0..2 {
            let a = &sim.actors[i];
            if a.down() {
                continue;
            }
            put(a.body.x, a.body.feet_y(), if i == CARL { 36.0 } else { 46.0 }, a.body.facing < 0, (if i == CARL { 19 } else { 22 }, 0));
        }
        for t in &sim.targets {
            match t {
                Hittable::Goblin(g) => put(g.x, g.y - 19.0, 28.0, g.facing < 0, (14, 0)),
                Hittable::Foe(f) => put(f.x, f.y - f.spec.hh, (f.spec.hw * 2.0).max(24.0), f.facing < 0, ((f.spec.hw) as i32, 0)),
                Hittable::Prop(_) => {}
            }
        }
    }
    let frame = PassFrame { scene, gathered: &gathered, shadows: &shadows, view_w: v.w as u32, view_h: v.h as u32, view_top: v.top };
    update_rig(&rig, &mut materials, &frame, Vec2::new(window.physical_width() as f32, window.physical_height() as f32));
}

// ----------------------------------------------------------------- watchdog

fn watchdog(
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    time: Res<Time<Real>>,
    lab: Res<Lab>,
    setup: Res<Setup>,
    windows: Query<&Window, With<PrimaryWindow>>,
    loading: Option<ResMut<Loading>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut loading) = loading else { return };
    if loading.reported {
        return;
    }
    for (name, handle) in &loading.handles {
        if let LoadState::Failed(error) = asset_server.load_state(handle) {
            println!("LAB_ASSET_FAILURE: {name}: {error}");
            exit.write(AppExit::error());
            loading.reported = true;
            return;
        }
    }
    if loading.handles.iter().all(|(_, h)| images.get(h).is_some()) {
        let out = windows.single().map(|w| (w.physical_width(), w.physical_height())).unwrap_or((0, 0));
        let v = lab.session.view();
        println!("LAB_TEXTURES_READY: {} atlases loaded", loading.handles.len());
        println!(
            "LAB_OUTPUT window={}x{} view={}x{} coverage={:.3} scale={:.4} integer={} carl_px={:.1} donut_px={:.1}",
            out.0,
            out.1,
            v.w,
            v.h,
            lab.session.camera.coverage(),
            out.0 as f32 / v.w as f32,
            is_integer_scale(out.0, v.w),
            effective_px(52.0, out.0, v.w),
            effective_px(40.0, out.0, v.w)
        );
        println!("{}", lab.session.sim.state_line());
        let _ = &setup.room;
        loading.reported = true;
    } else if time.elapsed_secs() > WATCHDOG_SECONDS {
        println!("LAB_ASSET_FAILURE: textures not loaded within {WATCHDOG_SECONDS} s");
        exit.write(AppExit::error());
        loading.reported = true;
    }
}

// ---------------------------------------------------------------------- perf

fn read_proc_status(key: &str) -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines().find(|l| l.starts_with(key)).and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse().ok()))
        })
        .unwrap_or(0)
}

fn perf_probe(
    perf: Option<ResMut<Perf>>,
    time: Res<Time<Real>>,
    loading: Option<Res<Loading>>,
    lab: Res<Lab>,
    setup: Res<Setup>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut perf) = perf else { return };
    if loading.is_some_and(|l| !l.reported) {
        return;
    }
    perf.seen += 1;
    if perf.seen <= perf.warmup {
        return;
    }
    perf.intervals_ms.push(time.delta_secs() * 1000.0);
    if perf.intervals_ms.len() >= perf.target {
        let mut s = perf.intervals_ms.clone();
        s.sort_by(f32::total_cmp);
        let n = s.len();
        let rank = |q: f32| s[(((q * n as f32).ceil() as usize).max(1) - 1).min(n - 1)];
        let mean = s.iter().sum::<f32>() / n as f32;
        let size = window.single().map(|w| format!("{}x{}", w.physical_width(), w.physical_height())).unwrap_or_default();
        let v = lab.session.view();
        println!(
            "LAB_PERF room={} coverage={} view={}x{} frames={} median_ms={:.2} p95_ms={:.2} p99_ms={:.2} mean_ms={:.2} max_ms={:.2} window={size} present=AutoNoVsync rss_kb={} hwm_kb={}",
            setup.room.id,
            match setup.args.coverage {
                CameraMode::Auto => "auto".to_owned(),
                CameraMode::Fixed(c) => format!("{c}"),
            },
            v.w,
            v.h,
            n,
            rank(0.5),
            rank(0.95),
            rank(0.99),
            mean,
            s[n - 1],
            read_proc_status("VmRSS:"),
            read_proc_status("VmHWM:")
        );
        exit.write(AppExit::Success);
    }
}

// ------------------------------------------------------------ capture sequence

const CAPTURE_SETTLE_FRAMES: u32 = 6;
const CAPTURE_TIMEOUT_FRAMES: u32 = 1200;
const CAPTURE_STABLE_FRAMES: u32 = 2;

#[derive(Debug, Default)]
struct PendingFile {
    waited: u32,
    last_len: u64,
    stable: u32,
}

#[derive(Debug)]
enum CapturePhase {
    WaitReady,
    Settle(u32),
    Pending(PathBuf, PendingFile),
}

#[derive(Resource)]
struct CaptureSequence {
    _first: u64,
    step: u64,
    count: u64,
    dir: PathBuf,
    index: u64,
    phase: CapturePhase,
}

fn capture_sequence(
    mut commands: Commands,
    seq: Option<ResMut<CaptureSequence>>,
    loading: Option<Res<Loading>>,
    mut lab: ResMut<Lab>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut seq) = seq else { return };
    let phase = std::mem::replace(&mut seq.phase, CapturePhase::WaitReady);
    seq.phase = match phase {
        CapturePhase::WaitReady if loading.is_some_and(|l| l.reported) => CapturePhase::Settle(CAPTURE_SETTLE_FRAMES),
        CapturePhase::WaitReady => CapturePhase::WaitReady,
        CapturePhase::Settle(n) if n > 0 => CapturePhase::Settle(n - 1),
        CapturePhase::Settle(_) => {
            if let Err(e) = std::fs::create_dir_all(&seq.dir) {
                println!("LAB_CAPTURE_FAILURE: {}: {e}", seq.dir.display());
                exit.write(AppExit::error());
                return;
            }
            let path = seq.dir.join(format!("frame_{:04}_tick_{:05}.png", seq.index, lab.session.sim.tick));
            commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
            CapturePhase::Pending(path, PendingFile::default())
        }
        CapturePhase::Pending(path, mut file) => {
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
                    println!("LAB_CAPTURE_FAILURE: screenshot {} was never written", path.display());
                    exit.write(AppExit::error());
                    return;
                }
                seq.phase = CapturePhase::Pending(path, file);
                return;
            }
            let v = lab.session.view();
            println!(
                "LAB_CAPTURE index={} tick={} view={},{} {}x{} file={}",
                seq.index,
                lab.session.sim.tick,
                v.left,
                v.top,
                v.w,
                v.h,
                path.display()
            );
            println!("{}", lab.session.sim.state_line());
            seq.index += 1;
            if seq.index >= seq.count {
                println!("LAB_CAPTURE_SEQUENCE_DONE count={}", seq.count);
                exit.write(AppExit::Success);
                return;
            }
            for _ in 0..seq.step {
                let inputs = inputs_of(&lab.session.sim, &lab.script, lab.idle);
                lab.session.step(inputs);
            }
            let events: Vec<String> = lab.session.sim.take_new_events().iter().map(ToString::to_string).collect();
            for e in events {
                println!("{e}");
            }
            CapturePhase::Settle(CAPTURE_SETTLE_FRAMES)
        }
    };
}

#[allow(dead_code)]
fn _unused(_: View, _: &LabSim, _: &Hittable, _: gate_warning_fn) {}
#[allow(non_camel_case_types)]
type gate_warning_fn = fn(u64) -> bool;
#[allow(dead_code)]
const _: fn(u64) -> bool = gate_warning_at;
