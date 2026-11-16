//! Isolated native-scale cast inspection scene; not a Floor 1 gameplay implementation.
//! Character art is fan-concept material and does not imply game distribution rights.

use bevy::{asset::AssetPlugin, camera::ScalingMode, prelude::*};

const FLOOR_Y: f32 = -107.0;
const TICK_SECONDS: f32 = 1.0 / 60.0;
const MOVE_SPEED: f32 = 145.0;
const JUMP_SPEED: f32 = 245.0;
const GRAVITY: f32 = 760.0;
const MAX_X: f32 = 298.0;

#[derive(Clone, Copy, Debug)]
struct CastSpec {
    name: &'static str,
    atlas: &'static str,
    width: u32,
    height: u32,
    pivot_x: f32,
    pivot_y: f32,
    opaque_height: u32,
    start_x: f32,
}

const CAST: [CastSpec; 3] = [
    CastSpec {
        name: "CARL",
        atlas: "carl_book1_h52.png",
        width: 36,
        height: 56,
        pivot_x: 19.0,
        pivot_y: 55.0,
        opaque_height: 52,
        start_x: -174.0,
    },
    CastSpec {
        name: "DONUT",
        atlas: "donut_scale_h40.png",
        width: 46,
        height: 44,
        pivot_x: 22.0,
        pivot_y: 43.0,
        opaque_height: 40,
        start_x: 0.0,
    },
    CastSpec {
        name: "MONGO",
        atlas: "mongo_adult_h76.png",
        width: 108,
        height: 80,
        pivot_x: 68.0,
        pivot_y: 79.0,
        opaque_height: 76,
        start_x: 174.0,
    },
];

#[derive(Debug, Clone, Copy)]
struct Motion {
    x: f32,
    elevation: f32,
    vertical_speed: f32,
}

impl Motion {
    fn new(x: f32) -> Self {
        Self {
            x,
            elevation: 0.0,
            vertical_speed: 0.0,
        }
    }

    fn step(&mut self, horizontal: f32, jump: bool) {
        self.x =
            (self.x + horizontal.clamp(-1.0, 1.0) * MOVE_SPEED * TICK_SECONDS).clamp(-MAX_X, MAX_X);
        if jump && self.elevation <= 0.0 {
            self.vertical_speed = JUMP_SPEED;
        }
        self.vertical_speed -= GRAVITY * TICK_SECONDS;
        self.elevation += self.vertical_speed * TICK_SECONDS;
        if self.elevation <= 0.0 {
            self.elevation = 0.0;
            self.vertical_speed = 0.0;
        }
    }
}

#[derive(Component)]
struct Character {
    slot: usize,
    spec: CastSpec,
    motion: Motion,
}

#[derive(Component)]
struct SelectionMarker;

#[derive(Component)]
struct IdleAnimation(Timer);

#[derive(Resource, Default)]
struct Controls {
    selected: usize,
    axis: f32,
    jump_pending: bool,
    reset_pending: bool,
}

#[derive(Resource)]
struct LoadedTextures {
    handles: Vec<Handle<Image>>,
    reported: bool,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--headless-playground-smoke") {
        headless_smoke();
        return;
    }
    let deck = args.iter().any(|arg| arg == "--deck");
    let (virtual_height, window_height): (f32, u32) =
        if deck { (400.0, 800) } else { (360.0, 720) };
    // Bevy 0.20 resolves a relative AssetPlugin path from the binary directory.
    // Use an absolute directory so source exports remain in the repository root.
    let asset_directory = std::env::current_dir()
        .expect("Character Playground must run from the repository root")
        .join("generated/art")
        .to_string_lossy()
        .into_owned();

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
                        title: "THE CRAWL - Character Playground".into(),
                        resolution: (1280, window_height).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::srgb_u8(14, 19, 27)))
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_resource::<Controls>()
        .add_systems(
            Startup,
            move |commands: Commands,
                  asset_server: Res<AssetServer>,
                  layouts: ResMut<Assets<TextureAtlasLayout>>| {
                setup(commands, asset_server, layouts, virtual_height);
            },
        )
        .add_systems(
            Update,
            (
                capture_input,
                advance_idle,
                update_marker,
                report_texture_loading,
            ),
        )
        .add_systems(FixedUpdate, fixed_movement)
        .run();
}

fn rectangle(commands: &mut Commands, size: Vec2, center: Vec3, color: Color) {
    commands.spawn((
        Sprite::from_color(color, size),
        Transform::from_translation(center),
    ));
}

fn label(
    commands: &mut Commands,
    text: impl Into<String>,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
) {
    commands.spawn((
        Text2d::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        Transform::from_xyz(x, y, 15.0),
    ));
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    virtual_height: f32,
) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: 640.0,
                height: virtual_height,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
    let steel = Color::srgb_u8(47, 59, 69);
    rectangle(
        &mut commands,
        Vec2::new(640.0, 70.0),
        Vec3::new(0.0, -143.0, -2.0),
        steel,
    );
    rectangle(
        &mut commands,
        Vec2::new(640.0, 3.0),
        Vec3::new(0.0, FLOOR_Y - 3.0, -1.0),
        Color::srgb_u8(117, 136, 145),
    );
    for x in (-304..=304).step_by(32) {
        rectangle(
            &mut commands,
            Vec2::new(1.0, 55.0),
            Vec3::new(x as f32, -151.0, -1.0),
            Color::srgb_u8(61, 72, 82),
        );
    }
    for x in (-304..=304).step_by(64) {
        rectangle(
            &mut commands,
            Vec2::new(5.0, 3.0),
            Vec3::new(x as f32, -109.0, 0.0),
            Color::srgb_u8(188, 136, 60),
        );
    }

    label(
        &mut commands,
        "THE CRAWL   /   CHARACTER PLAYGROUND",
        0.0,
        virtual_height / 2.0 - 25.0,
        15.0,
        Color::srgb_u8(229, 209, 170),
    );
    label(
        &mut commands,
        "NO PANTS. NO MERCY.",
        0.0,
        virtual_height / 2.0 - 46.0,
        10.0,
        Color::srgb_u8(182, 103, 105),
    );
    label(
        &mut commands,
        "1 / 2 / 3 or TAB: select     A / D: move     SPACE: jump     R: reset",
        0.0,
        virtual_height / 2.0 - 68.0,
        10.0,
        Color::srgb_u8(176, 193, 199),
    );

    let mut handles = Vec::new();
    for (slot, spec) in CAST.into_iter().enumerate() {
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(spec.width, spec.height),
            4,
            1,
            None,
            None,
        ));
        let texture: Handle<Image> = asset_server.load(spec.atlas);
        handles.push(texture.clone());
        let motion = Motion::new(spec.start_x);
        commands.spawn((
            Character { slot, spec, motion },
            Sprite::from_atlas_image(texture, TextureAtlas { layout, index: 0 }),
            actor_transform(spec, motion, false),
            IdleAnimation(Timer::from_seconds(0.15, TimerMode::Repeating)),
        ));
        label(
            &mut commands,
            format!("{}   {} px", spec.name, spec.opaque_height),
            spec.start_x,
            -156.0,
            11.0,
            Color::srgb_u8(230, 229, 219),
        );
    }
    commands.insert_resource(LoadedTextures {
        handles,
        reported: false,
    });
    // The marker is a separate ECS entity so changing actors never scales source pixels.
    commands.spawn((
        SelectionMarker,
        Sprite::from_color(Color::srgb_u8(245, 214, 108), Vec2::new(34.0, 3.0)),
        Transform::from_xyz(CAST[0].start_x, FLOOR_Y - 12.0, 5.0),
    ));
}

fn actor_transform(spec: CastSpec, motion: Motion, flip_x: bool) -> Transform {
    // Mirror the pivot offset as well as the pixels: otherwise turning a
    // wide asymmetrical sprite (Mongo) jumps its feet sideways by ~28 px.
    let offset_x = spec.width as f32 / 2.0 - spec.pivot_x;
    let center_x = if flip_x {
        motion.x - offset_x
    } else {
        motion.x + offset_x
    };
    Transform::from_xyz(
        center_x.round(),
        (FLOOR_Y + motion.elevation + spec.pivot_y - spec.height as f32 / 2.0).round(),
        3.0,
    )
}

fn capture_input(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut controls: ResMut<Controls>,
) {
    if keys.just_pressed(KeyCode::Digit1) {
        controls.selected = 0;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        controls.selected = 1;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        controls.selected = 2;
    }
    if keys.just_pressed(KeyCode::Tab) {
        controls.selected = (controls.selected + 1) % CAST.len();
    }
    let axis = (keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight)) as i8
        - (keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)) as i8;
    controls.jump_pending |= keys.just_pressed(KeyCode::Space);
    controls.reset_pending |= keys.just_pressed(KeyCode::KeyR);
    if let Some(pad) = gamepads.iter().next() {
        if pad.just_pressed(GamepadButton::RightTrigger) {
            controls.selected = (controls.selected + 1) % CAST.len();
        }
        if pad.just_pressed(GamepadButton::LeftTrigger) {
            controls.selected = (controls.selected + CAST.len() - 1) % CAST.len();
        }
        if axis == 0 {
            let stick = pad.left_stick().x;
            let pad_x = if stick.abs() > 0.18 {
                stick
            } else {
                pad.dpad().x
            };
            controls.axis = if pad_x.abs() > 0.18 { pad_x } else { 0.0 };
        }
        controls.jump_pending |= pad.just_pressed(GamepadButton::South);
        controls.reset_pending |= pad.just_pressed(GamepadButton::North);
    } else {
        controls.axis = 0.0;
    }
    if axis != 0 {
        controls.axis = axis as f32;
    }
}

fn fixed_movement(
    mut controls: ResMut<Controls>,
    mut actors: Query<(&mut Character, &mut Transform, &mut Sprite)>,
) {
    for (mut actor, mut transform, mut sprite) in &mut actors {
        if controls.reset_pending {
            actor.motion = Motion::new(actor.spec.start_x);
        } else if actor.slot == controls.selected {
            actor.motion.step(controls.axis, controls.jump_pending);
            if controls.axis < 0.0 {
                sprite.flip_x = true;
            }
            if controls.axis > 0.0 {
                sprite.flip_x = false;
            }
        }
        *transform = actor_transform(actor.spec, actor.motion, sprite.flip_x);
    }
    controls.jump_pending = false;
    controls.reset_pending = false;
}

fn advance_idle(time: Res<Time>, mut actors: Query<(&mut Sprite, &mut IdleAnimation)>) {
    for (mut sprite, mut animation) in &mut actors {
        if animation.0.tick(time.delta()).just_finished()
            && let Some(atlas) = sprite.texture_atlas.as_mut()
        {
            atlas.index = (atlas.index + 1) % 4;
        }
    }
}

fn update_marker(
    controls: Res<Controls>,
    actors: Query<&Character>,
    mut markers: Query<&mut Transform, With<SelectionMarker>>,
) {
    if let Some(actor) = actors.iter().find(|actor| actor.slot == controls.selected) {
        for mut marker in &mut markers {
            marker.translation.x = actor.motion.x.round();
        }
    }
}

fn report_texture_loading(images: Res<Assets<Image>>, mut textures: ResMut<LoadedTextures>) {
    if !textures.reported
        && textures
            .handles
            .iter()
            .all(|handle| images.get(handle).is_some())
    {
        println!("PLAYGROUND_TEXTURES_READY: 3 approved-scale atlases loaded");
        textures.reported = true;
    }
}

fn headless_smoke() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Startup, |mut commands: Commands| {
            for (slot, spec) in CAST.into_iter().enumerate() {
                commands.spawn(Character {
                    slot,
                    spec,
                    motion: Motion::new(spec.start_x),
                });
            }
        });
    app.update();
    let mut query = app.world_mut().query::<&Character>();
    let mut heights: Vec<u32> = query
        .iter(app.world())
        .map(|actor| actor.spec.opaque_height)
        .collect();
    heights.sort_unstable();
    assert_eq!(heights, vec![40, 52, 76]);
    let mut motion = Motion::new(0.0);
    motion.step(1.0, true);
    assert!(motion.x > 0.0 && motion.elevation > 0.0);
    println!("PLAYGROUND_HEADLESS_OK: 3 characters and fixed-step jump validated");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_three_approved_native_heights_are_preserved() {
        assert_eq!(CAST.map(|spec| spec.opaque_height), [52, 40, 76]);
        assert_eq!(
            CAST.map(|spec| spec.atlas),
            [
                "carl_book1_h52.png",
                "donut_scale_h40.png",
                "mongo_adult_h76.png"
            ]
        );
    }

    #[test]
    fn jump_lands_on_floor_without_tunneling() {
        let mut motion = Motion::new(0.0);
        motion.step(0.0, true);
        assert!(motion.elevation > 0.0);
        for _ in 0..120 {
            motion.step(0.0, false);
        }
        assert_eq!(motion.elevation, 0.0);
        assert_eq!(motion.vertical_speed, 0.0);
    }

    #[test]
    fn motion_clamps_to_playground_and_no_movement_on_idle() {
        let mut motion = Motion::new(MAX_X - 1.0);
        motion.step(1.0, false);
        assert_eq!(motion.x, MAX_X);
        motion.step(0.0, false);
        assert_eq!(motion.x, MAX_X);
    }

    #[test]
    fn changing_facing_does_not_shift_pixel_foot_pivot() {
        for spec in CAST {
            for flipped in [false, true] {
                let center_x = actor_transform(spec, Motion::new(spec.start_x), flipped)
                    .translation
                    .x;
                // Pixel-space pivot is measured from the left side of the atlas
                // frame and reflected when the sprite is flipped horizontally.
                let pivot_from_left = if flipped {
                    spec.width as f32 - spec.pivot_x
                } else {
                    spec.pivot_x
                };
                let world_foot_x = center_x - spec.width as f32 / 2.0 + pivot_from_left;
                assert_eq!(
                    world_foot_x, spec.start_x,
                    "{} flipped={flipped}",
                    spec.name
                );
            }
        }
    }

    #[test]
    fn pivot_anchors_at_native_pixel_coordinates() {
        for spec in CAST {
            let transform = actor_transform(spec, Motion::new(spec.start_x), false);
            let expected_y = (FLOOR_Y + spec.pivot_y - spec.height as f32 / 2.0).round();
            assert_eq!(transform.translation.y, expected_y);
            assert_eq!(transform.scale, Vec3::ONE);
        }
    }
}
