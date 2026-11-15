//! WORK-001: IP-neutral Bevy Linux input/camera/rendering spike.

use bevy::{camera::ScalingMode, prelude::*};
use dungeon_floorbreak::{
    camera::{VIRTUAL_HEIGHT, VIRTUAL_WIDTH, WINDOW_HEIGHT, WINDOW_WIDTH},
    input::{InputFrame, RawInput},
};

#[derive(Component)]
struct PlaceholderHero;

#[derive(Resource, Default)]
struct PlayerInput(InputFrame);

fn main() {
    if std::env::args().any(|arg| arg == "--headless-smoke") {
        run_headless_smoke();
        return;
    }

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Dungeon Floorbreak - WORK-001".into(),
                        resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::srgb_u8(20, 24, 36)))
        .init_resource::<PlayerInput>()
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (read_input, move_placeholder).chain())
        .run();
}

fn setup_scene(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: VIRTUAL_WIDTH as f32,
                height: VIRTUAL_HEIGHT as f32,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));

    for x in -20..=20 {
        commands.spawn((
            Sprite::from_color(Color::srgb_u8(83, 96, 111), Vec2::splat(15.0)),
            Transform::from_xyz(x as f32 * 16.0, -118.0, 0.0),
        ));
    }
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(188, 125, 75), Vec2::new(42.0, 64.0)),
        Transform::from_xyz(110.0, -78.0, 0.0),
    ));
    commands.spawn((
        PlaceholderHero,
        Sprite::from_color(Color::srgb_u8(95, 215, 180), Vec2::new(26.0, 43.0)),
        Transform::from_xyz(-100.0, -89.0, 1.0),
    ));
}

fn read_input(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    mut player_input: ResMut<PlayerInput>,
) {
    let left = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft);
    let right = keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight);
    let keyboard_x = (right as i8 - left as i8) as f32;
    let mut raw = RawInput {
        move_x: keyboard_x,
        jump: keys.pressed(KeyCode::Space),
        attack: keys.pressed(KeyCode::KeyJ),
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
        raw.pause |= pad.pressed(GamepadButton::Start);
    }
    player_input.0.advance(raw);
}

fn move_placeholder(
    time: Res<Time>,
    input: Res<PlayerInput>,
    mut heroes: Query<&mut Transform, With<PlaceholderHero>>,
) {
    for mut transform in &mut heroes {
        transform.translation.x = (transform.translation.x
            + input.0.held.move_x * 135.0 * time.delta_secs())
            .clamp(-290.0, 290.0)
            .round();
    }
}

fn run_headless_smoke() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(PlaceholderHero);
        });
    app.update();
    let mut heroes = app
        .world_mut()
        .query_filtered::<Entity, With<PlaceholderHero>>();
    assert_eq!(heroes.iter(app.world()).count(), 1);
    println!("HEADLESS_SMOKE_OK: Bevy startup and ECS update executed");
}
