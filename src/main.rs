//! WORK-001: non-playable Bevy startup, pixel camera and a device-neutral action adapter.
//! Combat, movement and destructive terrain are deliberately outside this spike.

mod action;

use action::{PlayerAction, RawInput, replay_digest};
use bevy::{camera::ScalingMode, prelude::*};

const VIEW_HEIGHT_PX: f32 = 270.0;

#[derive(Resource, Default)]
struct ActionState {
    tick: u64,
    latest: PlayerAction,
}

fn main() {
    // Makes the synthetic input contract checkable in CI without a window or GPU.
    if std::env::args().any(|arg| arg == "--headless-action-test") {
        let input = [
            RawInput { move_x: 1.0, ..RawInput::default() },
            RawInput { jump: true, ..RawInput::default() },
            RawInput { attack: true, ..RawInput::default() },
        ];
        println!("WORK-001 deterministic input replay: {:#016x}", replay_digest(input));
        return;
    }

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Dungeon Floorbreak / WORK-001".into(),
                        resolution: WindowResolution::new(960, 540),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.12)))
        .init_resource::<ActionState>()
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, capture_actions)
        .add_systems(Update, snap_camera_to_integer_pixels)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: VIEW_HEIGHT_PX,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));

    // A geometric, original-IP test scene rather than playable content.
    // At the default 960x540 window, one world unit is exactly two screen pixels.
    for x in -12..=12 {
        commands.spawn((
            Sprite::from_color(
                if x % 2 == 0 {
                    Color::srgb(0.30, 0.34, 0.42)
                } else {
                    Color::srgb(0.40, 0.32, 0.32)
                },
                Vec2::splat(16.0),
            ),
            Transform::from_xyz(x as f32 * 16.0, -64.0, 0.0),
        ));
    }

    commands.spawn((
        Sprite::from_color(Color::srgb(0.88, 0.61, 0.39), Vec2::new(12.0, 20.0)),
        Transform::from_xyz(0.0, -46.0, 1.0),
    ));
}

fn capture_actions(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut state: ResMut<ActionState>,
) {
    // WORK-001 binds only one slot; explicit device ownership comes in WORK-005.
    // A disconnected gamepad must not leave stale directional input behind.
    let raw = if let Some(pad) = gamepads.iter().next() {
        let movement = pad.left_stick() + pad.dpad();
        RawInput {
            move_x: movement.x,
            move_y: movement.y,
            aim_x: pad.right_stick().x,
            aim_y: pad.right_stick().y,
            jump: pad.just_pressed(GamepadButton::South),
            attack: pad.pressed(GamepadButton::West),
            special: pad.just_pressed(GamepadButton::East),
            interact: pad.just_pressed(GamepadButton::North),
            swap: pad.just_pressed(GamepadButton::RightTrigger),
        }
    } else {
        RawInput {
            move_x: i32::from(keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight))
                as f32
                - i32::from(keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft))
                    as f32,
            move_y: 0.0,
            aim_x: 0.0,
            aim_y: 0.0,
            jump: keys.just_pressed(KeyCode::Space),
            attack: keys.pressed(KeyCode::KeyJ),
            special: keys.just_pressed(KeyCode::KeyK),
            interact: keys.just_pressed(KeyCode::KeyE),
            swap: keys.just_pressed(KeyCode::Tab),
        }
    };
    state.latest = PlayerAction::from_raw(state.tick, 0, raw);
    state.tick += 1;
}

fn snap_camera_to_integer_pixels(mut cameras: Query<&mut Transform, With<Camera2d>>) {
    // Only the camera presentation transform is snapped; game-domain positions
    // must remain unsnapped for deterministic collision and interpolation.
    for mut camera in &mut cameras {
        camera.translation.x = camera.translation.x.round();
        camera.translation.y = camera.translation.y.round();
    }
}
