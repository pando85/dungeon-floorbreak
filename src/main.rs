//! Combat-first, IP-neutral, right-scrolling Bevy greybox.
//! The approved Carl and Donut sprite masters will replace placeholders later.

use bevy::{camera::ScalingMode, prelude::*};
use dungeon_floorbreak::{
    audio::{AudioSettings, cues_from_wall_break},
    audio_adapter::{AudioQueue, FloorAudioPlugin, resolve_pack_dir},
    camera::{VIRTUAL_HEIGHT, VIRTUAL_WIDTH, WINDOW_HEIGHT, WINDOW_WIDTH},
    input::{InputFrame, RawInput},
    prototype::{Material, STAGE_WIDTH, World},
};

#[derive(Resource, Default)]
struct Simulation(World);
#[derive(Resource, Default)]
struct PlayerInput(InputFrame);
#[derive(Component)]
enum VisualRole {
    Hero,
    Enemy(usize),
    Block(usize),
}
#[derive(Component)]
struct TemporaryVisual;

fn main() {
    if std::env::args().any(|arg| arg == "--headless-smoke") {
        run_headless_smoke();
        return;
    }
    if std::env::args().any(|arg| arg == "--headless-combat-smoke") {
        run_combat_smoke();
        return;
    }
    // AUDIO-01: the greybox only has the bomb wall-break sound (no scene music).
    let mut audio_settings = AudioSettings::from_env();
    audio_settings.enabled &= !std::env::args().any(|arg| arg == "--no-audio");
    let mut plugins = DefaultPlugins.set(ImagePlugin::default_nearest());
    if !audio_settings.enabled {
        plugins = plugins.disable::<bevy::audio::AudioPlugin>();
    }
    let pack_dir = resolve_pack_dir();
    App::new()
        .add_plugins(plugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "The Crawl | A/D move, Space jump, J melee, K bomb".into(),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FloorAudioPlugin {
            settings: audio_settings,
            pack_dir,
            initial: Vec::new(),
        })
        .insert_resource(ClearColor(Color::srgb_u8(16, 17, 30)))
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_resource::<Simulation>()
        .init_resource::<PlayerInput>()
        .add_systems(Startup, setup_scene)
        .add_systems(FixedUpdate, advance_game)
        .add_systems(Update, sync_scene)
        .run();
}

fn setup_scene(mut commands: Commands, simulation: Res<Simulation>) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: VIRTUAL_WIDTH as f32,
                height: VIRTUAL_HEIGHT as f32,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(VIRTUAL_WIDTH as f32 * 0.5, 0.0, 0.0),
    ));
    // Abstract industrial atmosphere; no copyrighted or final production assets.
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(27, 31, 51), Vec2::new(STAGE_WIDTH, 310.0)),
        Transform::from_xyz(STAGE_WIDTH * 0.5, 23.0, -10.0),
    ));
    for i in 0..12 {
        let x = 85.0 + i as f32 * 180.0;
        for (color, size, dx, dy, depth) in [
            (
                Color::srgb_u8(40, 40, 62),
                Vec2::new(22.0, 250.0),
                0.0,
                12.0,
                -8.0,
            ),
            (
                Color::srgb_u8(56, 48, 65),
                Vec2::new(110.0, 9.0),
                55.0,
                108.0,
                -8.0,
            ),
            (
                Color::srgb_u8(163, 91, 44),
                Vec2::splat(10.0),
                20.0,
                44.0,
                -5.0,
            ),
        ] {
            commands.spawn((
                Sprite::from_color(color, size),
                Transform::from_xyz(x + dx, dy, depth),
            ));
        }
    }
    // Recognizable entry and exit markers for the greybox only.
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(80, 69, 82), Vec2::new(75.0, 145.0)),
        Transform::from_xyz(45.0, -45.0, -2.0),
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(28, 24, 41), Vec2::new(55.0, 124.0)),
        Transform::from_xyz(45.0, -48.0, -1.0),
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(224, 159, 65), Vec2::new(16.0, 100.0)),
        Transform::from_xyz(STAGE_WIDTH - 65.0, -65.0, 0.0),
    ));
    for (index, block) in simulation.0.blocks.iter().enumerate() {
        let (x, y) = block.rect.center();
        let color = match block.material {
            Material::Stone => Color::srgb_u8(75, 74, 97),
            Material::Steel => Color::srgb_u8(78, 105, 122),
            Material::Catwalk => Color::srgb_u8(174, 103, 61),
            Material::Timber => Color::srgb_u8(221, 175, 76),
        };
        commands.spawn((
            VisualRole::Block(index),
            Sprite::from_color(color, Vec2::new(block.rect.width, block.rect.height)),
            Transform::from_xyz(x, y, 0.5),
        ));
    }
    for (index, enemy) in simulation.0.enemies.iter().enumerate() {
        commands.spawn((
            VisualRole::Enemy(index),
            Sprite::from_color(Color::srgb_u8(179, 92, 82), Vec2::new(24.0, 38.0)),
            Transform::from_xyz(enemy.x, enemy.y, 1.5),
        ));
    }
    commands.spawn((
        VisualRole::Hero,
        Sprite::from_color(Color::srgb_u8(108, 212, 196), Vec2::new(24.0, 52.0)),
        Transform::from_xyz(simulation.0.hero.x, simulation.0.hero.y, 2.0),
    ));
}

fn advance_game(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut input: ResMut<PlayerInput>,
    mut simulation: ResMut<Simulation>,
    mut audio: ResMut<AudioQueue>,
) {
    let left = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft);
    let right = keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight);
    let keyboard_x = (right as i8 - left as i8) as f32;
    let mut raw = RawInput {
        move_x: keyboard_x,
        jump: keys.pressed(KeyCode::Space),
        attack: keys.pressed(KeyCode::KeyJ),
        physical: false,
        ability: keys.pressed(KeyCode::KeyK),
        pause: keys.pressed(KeyCode::Escape),
    };
    if let Some(pad) = pads.iter().next() {
        let stick_x = pad.left_stick().x;
        let pad_x = if stick_x.abs() > 0.18 {
            stick_x
        } else {
            pad.dpad().x
        };
        if keyboard_x == 0.0 {
            raw.move_x = pad_x;
        }
        raw.jump |= pad.pressed(GamepadButton::South);
        raw.attack |= pad.pressed(GamepadButton::West);
        raw.ability |= pad.pressed(GamepadButton::East);
    }
    input.0.advance(raw);
    simulation.0.step(&input.0);
    audio.push_cues(cues_from_wall_break(simulation.0.timbers_broken_last_step));
}

fn sync_scene(
    mut commands: Commands,
    simulation: Res<Simulation>,
    mut visuals: Query<(&mut Transform, &mut Visibility, &VisualRole), Without<Camera2d>>,
    mut camera: Query<&mut Transform, With<Camera2d>>,
    temporary: Query<Entity, With<TemporaryVisual>>,
) {
    let world = &simulation.0;
    for (mut transform, mut visibility, role) in &mut visuals {
        match role {
            VisualRole::Hero => {
                transform.translation.x = world.hero.x.round();
                transform.translation.y = world.hero.y.round();
                *visibility = if world.hero.invulnerable_ticks % 8 >= 4 {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                };
            }
            VisualRole::Enemy(index) => {
                if let Some(enemy) = world.enemies.get(*index) {
                    transform.translation.x = enemy.x.round();
                    transform.translation.y = enemy.y.round();
                    *visibility = if enemy.health == 0 {
                        Visibility::Hidden
                    } else {
                        Visibility::Inherited
                    };
                }
            }
            VisualRole::Block(index) => {
                if let Some(block) = world.blocks.get(*index) {
                    *visibility = if block.intact {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    };
                }
            }
        }
    }
    if let Ok(mut camera) = camera.single_mut() {
        camera.translation.x = world
            .hero
            .x
            .clamp(
                VIRTUAL_WIDTH as f32 * 0.5,
                STAGE_WIDTH - VIRTUAL_WIDTH as f32 * 0.5,
            )
            .round();
    }
    for entity in &temporary {
        commands.entity(entity).despawn();
    }
    for bomb in &world.bombs {
        commands.spawn((
            TemporaryVisual,
            Sprite::from_color(Color::srgb_u8(245, 175, 53), Vec2::splat(13.0)),
            Transform::from_xyz(bomb.x, bomb.y, 2.0),
        ));
    }
    for blast in &world.blasts {
        commands.spawn((
            TemporaryVisual,
            Sprite::from_color(Color::srgba(1.0, 0.49, 0.13, 0.37), Vec2::splat(168.0)),
            Transform::from_xyz(blast.x, blast.y, 3.0),
        ));
    }
}

fn run_headless_smoke() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(VisualRole::Hero);
        });
    app.update();
    let mut heroes = app.world_mut().query_filtered::<Entity, With<VisualRole>>();
    assert_eq!(heroes.iter(app.world()).count(), 1);
    println!("HEADLESS_SMOKE_OK: Bevy startup and ECS update executed");
}

fn run_combat_smoke() {
    let mut world = World::new();
    world.hero.x = 568.0;
    let starting_timbers = world.intact_timbers();
    let mut frame = InputFrame::default();
    frame.advance(RawInput {
        ability: true,
        ..RawInput::default()
    });
    world.step(&frame);
    for _ in 0..30 {
        frame.advance(RawInput::default());
        world.step(&frame);
    }
    assert_eq!(world.intact_timbers(), starting_timbers - 1);
    println!("LINEAR_COMBAT_SMOKE_OK: timed bomb breached timber and left fixed stage intact");
}
