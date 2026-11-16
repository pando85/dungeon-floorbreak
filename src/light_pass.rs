//! Bevy adapter for the Floor 1 lighting pass (COMP-02). Presentation only.
//!
//! Pipeline (ADR-0007: pixel layers at virtual resolution, light maths at
//! output resolution):
//!
//! ```text
//!  sprites (RenderLayers 1..=N, one per slot)
//!     |  N cameras, Fixed 640x360 ortho, whole-pixel positions
//!     v
//!  N slot images  (virtual resolution, Rgba8UnormSrgb, transparent clear)
//!     |
//!     +--> [emissive pass, virtual res]  flame pixels, lit, bright-passed
//!     |         v
//!     |    [blur H] -> [blur V]          gaussian, sigma = bloom radius
//!     |         v
//!     +--> [composite pass, window res]  textureLoad (exact nearest) per slot,
//!                                        ambient + pools + shadow + haze,
//!                                        additive glow, + bloom  -> window
//! ```
//!
//! Every stage after the slot cameras is a full-screen `Mesh2d` quad with a
//! [`LightingMaterial`] seen by its own camera; the stages are ordered by
//! `Camera::order`. All inputs are plain data derived from `lighting`; nothing
//! here reads or writes the simulation.

use bevy::{
    asset::uuid_handle,
    camera::{ClearColorConfig, RenderTarget, ScalingMode, visibility::RenderLayers},
    mesh::MeshVertexBufferLayoutRef,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
        TextureFormat,
        encase::{ShaderSize, UniformBuffer},
    },
    shader::{Shader, ShaderRef},
    sprite_render::{AlphaMode2d, Material2d, Material2dKey, Material2dPlugin},
};

use crate::lighting::{
    GLOW_FALLOFF_K, GatheredLights, LightingScene, MAX_EMISSIVE_COLOURS, MAX_LIGHTS, MAX_SHADOWS,
    MAX_SLOTS, POOL_FALLOFF_K, ShadowEllipse, hex_unit, parse_hex,
};

const SHADER_BODY: &str = include_str!("light_pass.wgsl");

/// The shader module source: constants from Rust prepended to the WGSL body, so
/// array lengths, flag bits and falloff constants have one home.
pub fn shader_source() -> String {
    format!(
        "const MAX_SHADOWS: u32 = {}u;\nconst MAX_EMISSIVE: u32 = {}u;\nconst MAX_SLOTS: u32 = {}u;\nconst MAX_LIGHTS: u32 = {}u;\n\
         const FLAG_EMISSIVE: u32 = {}u;\nconst FLAG_SHADOW: u32 = {}u;\nconst FLAG_PARALLAX: u32 = {}u;\n\
         const POOL_K: f32 = {:?};\nconst GLOW_K: f32 = {:?};\n{SHADER_BODY}",
        MAX_SHADOWS,
        MAX_EMISSIVE_COLOURS,
        MAX_SLOTS,
        MAX_LIGHTS,
        FLAG_EMISSIVE,
        FLAG_SHADOW,
        FLAG_PARALLAX,
        POOL_FALLOFF_K,
        GLOW_FALLOFF_K,
    )
}
const LIGHT_SHADER: Handle<Shader> = uuid_handle!("7c0f3a52-6c1d-4f6e-9a8b-2f6d1d0b5e01");

/// Render layer of slot `i` (layers 1..=MAX_SLOTS; layer 0 is the plain scene).
pub fn slot_layer(slot: usize) -> RenderLayers {
    RenderLayers::layer(1 + slot)
}

const LAYER_EMISSIVE: usize = 10;
const LAYER_BLUR_H: usize = 11;
const LAYER_BLUR_V: usize = 12;
const LAYER_COMPOSITE: usize = 13;
/// Layer of the debug overlay, drawn after the composite.
pub const LAYER_DEBUG: usize = 14;

/// Marks every camera that follows the scroll camera (slot cameras, the plain
/// camera when lighting is off, and the debug overlay camera).
#[derive(Component)]
pub struct SceneCamera;

// -------------------------------------------------------------- GPU structs --

/// One light as the shader reads it (3 x vec4, 48 bytes).
#[derive(Debug, Clone, Copy, Default, PartialEq, ShaderType)]
pub struct GpuLight {
    /// x, y view px (y down); z radius; w squash_y.
    pub pos: Vec4,
    /// rgb colour 0..1; w intensity with flicker.
    pub colour: Vec4,
    /// x glow radius; y glow strength with flicker.
    pub glow: Vec4,
}

/// Per slot parameters (4 x vec4, 64 bytes).
#[derive(Debug, Clone, Copy, Default, PartialEq, ShaderType)]
pub struct GpuSlot {
    /// rgb ambient; w light_scale.
    pub ambient: Vec4,
    /// rgb haze colour; w constant amount.
    pub haze: Vec4,
    /// from_y, to_y, amount, enabled.
    pub fog: Vec4,
    /// light range start, end, flags (1 emissive, 2 shadow, 4 parallax), 0.
    pub info: UVec4,
}

pub const FLAG_EMISSIVE: u32 = 1;
pub const FLAG_SHADOW: u32 = 2;
pub const FLAG_PARALLAX: u32 = 4;

/// The whole uniform block (identical in every pass; per-pass fields differ).
#[derive(Debug, Clone, PartialEq, ShaderType)]
pub struct LightingUniform {
    /// view w, h (virtual px), view top (layout y), slot count.
    pub view: Vec4,
    /// target w, h (px), shadow count, shadow strength.
    pub target_px: Vec4,
    /// bright-pass threshold 0..1, gain, blur sigma, emissive colour count.
    pub bloom: Vec4,
    /// blur step x, y.
    pub blur: Vec4,
    /// far end, mid end, scene end, light count.
    pub ranges: UVec4,
    pub shadows: [Vec4; MAX_SHADOWS],
    pub emissive: [Vec4; MAX_EMISSIVE_COLOURS],
    pub slots: [GpuSlot; MAX_SLOTS],
    pub lights: [GpuLight; MAX_LIGHTS],
}

impl Default for LightingUniform {
    fn default() -> Self {
        Self {
            view: Vec4::ZERO,
            target_px: Vec4::ZERO,
            bloom: Vec4::ZERO,
            blur: Vec4::ZERO,
            ranges: UVec4::ZERO,
            shadows: [Vec4::ZERO; MAX_SHADOWS],
            emissive: [Vec4::ZERO; MAX_EMISSIVE_COLOURS],
            slots: [GpuSlot::default(); MAX_SLOTS],
            lights: [GpuLight::default(); MAX_LIGHTS],
        }
    }
}

/// Everything that varies per frame, as plain data.
pub struct PassFrame<'a> {
    pub scene: &'a LightingScene,
    pub gathered: &'a GatheredLights,
    pub shadows: &'a [ShadowEllipse],
    pub view_w: u32,
    pub view_h: u32,
    /// Top edge of the view in layout px (haze ramps are in layout y).
    pub view_top: i32,
}

/// Pack a frame into the uniform block. Pure; `target_px` is the size of the
/// target the pass renders to (window pixels for the composite).
pub fn pack_uniform(frame: &PassFrame, target_px: Vec2) -> LightingUniform {
    let mut u = LightingUniform::default();
    let scene = frame.scene;
    let profile = &scene.profile;
    u.view = Vec4::new(
        frame.view_w as f32,
        frame.view_h as f32,
        frame.view_top as f32,
        scene.plan.slots.len() as f32,
    );
    let shadow_strength = profile
        .actor_contact_shadow
        .as_ref()
        .map_or(0.0, |s| s.strength);
    // The budgets are validated at load (`LightingScene::build`) and by
    // `gather`; truncating here would be silent, so exceeding them is a bug.
    assert!(
        frame.shadows.len() <= MAX_SHADOWS,
        "{} contact shadows exceed the pass budget of {MAX_SHADOWS}",
        frame.shadows.len()
    );
    assert!(
        frame.gathered.lights.len() <= MAX_LIGHTS,
        "{} lights exceed the pass budget of {MAX_LIGHTS}",
        frame.gathered.lights.len()
    );
    let shadow_count = frame.shadows.len();
    u.target_px = Vec4::new(
        target_px.x,
        target_px.y,
        if profile.actor_contact_shadow.is_some() {
            shadow_count as f32
        } else {
            0.0
        },
        shadow_strength,
    );
    for (i, e) in frame.shadows.iter().enumerate() {
        u.shadows[i] = Vec4::new(e.cx, e.cy, e.half_w, e.half_h);
    }
    let colours: Vec<[u8; 3]> = profile
        .bloom
        .emissive_colours
        .iter()
        .filter_map(|c| parse_hex(c))
        .collect();
    // `LightingProfile::validate` caps the list at MAX_EMISSIVE_COLOURS.
    assert!(colours.len() <= MAX_EMISSIVE_COLOURS);
    for (i, c) in colours.iter().enumerate() {
        u.emissive[i] = Vec4::new(f32::from(c[0]), f32::from(c[1]), f32::from(c[2]), 0.0);
    }
    u.bloom = Vec4::new(
        profile.bloom.threshold / 255.0,
        profile.bloom.intensity,
        profile.bloom.radius_virtual,
        colours.len() as f32,
    );
    let g = frame.gathered;
    u.ranges = UVec4::new(g.far_end, g.mid_end, g.scene_end, g.lights.len() as u32);
    for (i, l) in g.lights.iter().enumerate() {
        u.lights[i] = GpuLight {
            pos: Vec4::new(l.x, l.y, l.radius, l.squash_y),
            colour: Vec4::new(l.colour[0], l.colour[1], l.colour[2], l.intensity),
            glow: Vec4::new(l.glow_radius, l.glow_additive, 0.0, 0.0),
        };
    }
    for (i, slot) in scene.plan.slots.iter().enumerate() {
        let group = profile.group(slot.group);
        let (start, end) = g.range_for(slot.group);
        let mut flags = 0;
        if slot.emissive {
            flags |= FLAG_EMISSIVE;
        }
        if slot.group == crate::lighting::LightGroup::Scene {
            flags |= FLAG_SHADOW;
        }
        if slot.group.is_parallax() {
            flags |= FLAG_PARALLAX;
        }
        let (haze_rgb, haze_amount, fog) = match &group.haze {
            Some(haze) => {
                let rgb = hex_unit(&haze.colour).unwrap_or([0.0; 3]);
                let fog = haze
                    .floor_fog
                    .as_ref()
                    .map_or(Vec4::ZERO, |f| Vec4::new(f.from_y, f.to_y, f.amount, 1.0));
                (rgb, haze.amount, fog)
            }
            None => ([0.0; 3], 0.0, Vec4::ZERO),
        };
        u.slots[i] = GpuSlot {
            ambient: Vec4::new(
                group.ambient[0],
                group.ambient[1],
                group.ambient[2],
                group.light_scale.unwrap_or(1.0),
            ),
            haze: Vec4::new(haze_rgb[0], haze_rgb[1], haze_rgb[2], haze_amount),
            fog,
            info: UVec4::new(start, end, flags, 0),
        };
    }
    u
}

/// The uniform block as the bytes the GPU receives (std140-style layout from
/// encase). Used by the packing tests.
pub fn uniform_bytes(u: &LightingUniform) -> Vec<u8> {
    let mut buffer = UniformBuffer::new(Vec::<u8>::new());
    buffer.write(u).expect("uniform block fits");
    buffer.into_inner()
}

/// Byte size of the uniform block.
pub fn uniform_size() -> u64 {
    LightingUniform::SHADER_SIZE.get()
}

// ----------------------------------------------------------------- material --

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LightPass {
    Emissive,
    Blur,
    Composite,
}

impl LightPass {
    fn entry_point(self) -> &'static str {
        match self {
            Self::Emissive => "emissive",
            Self::Blur => "blur",
            Self::Composite => "composite",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LightPassKey {
    pass: LightPass,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(LightPassKey)]
pub struct LightingMaterial {
    pub pass: LightPass,
    #[uniform(0)]
    pub params: LightingUniform,
    #[texture(1)]
    pub slot0: Handle<Image>,
    #[texture(2)]
    pub slot1: Handle<Image>,
    #[texture(3)]
    pub slot2: Handle<Image>,
    #[texture(4)]
    pub slot3: Handle<Image>,
    #[texture(5)]
    pub slot4: Handle<Image>,
    #[texture(6)]
    pub slot5: Handle<Image>,
    #[texture(7)]
    pub aux: Handle<Image>,
}

impl From<&LightingMaterial> for LightPassKey {
    fn from(material: &LightingMaterial) -> Self {
        Self {
            pass: material.pass,
        }
    }
}

impl Material2d for LightingMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(LIGHT_SHADER)
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Opaque
    }

    fn specialize(
        _pipeline: &bevy::sprite_render::Material2dPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.entry_point = Some(key.bind_group_data.pass.entry_point().into());
        }
        Ok(())
    }
}

pub struct LightPassPlugin;

impl Plugin for LightPassPlugin {
    fn build(&self, app: &mut App) {
        app.world_mut()
            .resource_mut::<Assets<Shader>>()
            .insert(
                &LIGHT_SHADER,
                Shader::from_wgsl(shader_source(), "floor01_light_pass.wgsl"),
            )
            .expect("shader handle is a UUID");
        app.add_plugins(Material2dPlugin::<LightingMaterial>::default());
    }
}

// ---------------------------------------------------------------------- rig --

/// Handles the per-frame update needs.
#[derive(Resource)]
pub struct LightRig {
    pub emissive: Handle<LightingMaterial>,
    pub composite: Handle<LightingMaterial>,
    pub view: UVec2,
}

/// A render-target image. Slot images are `Rgba8UnormSrgb` (sprites write
/// linear values, the target stores them encoded). The emissive and blur
/// targets are `Rgba8Unorm`: the shader writes and reads encoded values directly,
/// which avoids a pow() per blur tap.
fn target_image(images: &mut Assets<Image>, size: UVec2, format: TextureFormat) -> Handle<Image> {
    images.add(Image::new_target_texture(size.x, size.y, format, None))
}

fn fixed_projection(width: f32, height: f32) -> Projection {
    Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::Fixed { width, height },
        ..OrthographicProjection::default_2d()
    })
}

/// Camera that renders the sprites of one slot into its own image.
fn slot_camera(
    order: isize,
    image: Handle<Image>,
    view: UVec2,
    layers: RenderLayers,
) -> impl Bundle {
    (
        Camera2d,
        Camera {
            order,
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
        RenderTarget::Image(image.into()),
        fixed_projection(view.x as f32, view.y as f32),
        Transform::default(),
        Msaa::Off,
        layers,
        SceneCamera,
    )
}

/// Camera that renders one full-screen quad (2 x 2 world units under a 2 x 2
/// fixed projection covers any target exactly).
fn quad_camera(order: isize, target: Option<Handle<Image>>, layers: RenderLayers) -> impl Bundle {
    (
        Camera2d,
        Camera {
            order,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        fixed_projection(2.0, 2.0),
        Transform::default(),
        Msaa::Off,
        layers,
        target.map_or_else(RenderTarget::default, |handle| {
            RenderTarget::Image(handle.into())
        }),
    )
}

/// Spawn the slot cameras, the three post stages and their cameras. Returns the
/// rig the per-frame update uses. Slot sprites must be given
/// `slot_layer(slot)` by the caller.
pub fn spawn_rig(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<LightingMaterial>,
    scene: &LightingScene,
    view: UVec2,
) -> LightRig {
    let slot_count = scene.plan.slots.len();
    assert!(slot_count <= MAX_SLOTS, "slot plan is validated at load");
    let slot_images: Vec<Handle<Image>> = (0..MAX_SLOTS)
        .map(|_| target_image(images, view, TextureFormat::Rgba8UnormSrgb))
        .collect();
    for (i, image) in slot_images.iter().enumerate().take(slot_count) {
        commands.spawn(slot_camera(i as isize, image.clone(), view, slot_layer(i)));
    }

    let emissive_image = target_image(images, view, TextureFormat::Rgba8Unorm);
    let blur_h_image = target_image(images, view, TextureFormat::Rgba8Unorm);
    let blur_v_image = target_image(images, view, TextureFormat::Rgba8Unorm);
    let material = |pass: LightPass, aux: &Handle<Image>, step: Vec2| {
        // Static values for the blur stages; the emissive and composite stages
        // are rewritten every frame.
        let params = LightingUniform {
            view: Vec4::new(view.x as f32, view.y as f32, 0.0, slot_count as f32),
            target_px: Vec4::new(view.x as f32, view.y as f32, 0.0, 0.0),
            blur: Vec4::new(step.x, step.y, 0.0, 0.0),
            bloom: Vec4::new(
                scene.profile.bloom.threshold / 255.0,
                scene.profile.bloom.intensity,
                scene.profile.bloom.radius_virtual,
                0.0,
            ),
            ..default()
        };
        LightingMaterial {
            pass,
            params,
            slot0: slot_images[0].clone(),
            slot1: slot_images[1].clone(),
            slot2: slot_images[2].clone(),
            slot3: slot_images[3].clone(),
            slot4: slot_images[4].clone(),
            slot5: slot_images[5].clone(),
            aux: aux.clone(),
        }
    };
    let quad = meshes.add(Rectangle::new(2.0, 2.0));
    let emissive = materials.add(material(LightPass::Emissive, &emissive_image, Vec2::ZERO));
    let blur_h = materials.add(material(LightPass::Blur, &emissive_image, Vec2::X));
    let blur_v = materials.add(material(LightPass::Blur, &blur_h_image, Vec2::Y));
    let composite = materials.add(material(LightPass::Composite, &blur_v_image, Vec2::ZERO));

    let first_post = slot_count as isize;
    let stages = [
        (
            first_post,
            LAYER_EMISSIVE,
            emissive.clone(),
            Some(emissive_image),
        ),
        (first_post + 1, LAYER_BLUR_H, blur_h, Some(blur_h_image)),
        (first_post + 2, LAYER_BLUR_V, blur_v, Some(blur_v_image)),
        (first_post + 3, LAYER_COMPOSITE, composite.clone(), None),
    ];
    for (order, layer, handle, target) in stages {
        let layers = RenderLayers::layer(layer);
        commands.spawn(quad_camera(order, target, layers.clone()));
        commands.spawn((
            Mesh2d(quad.clone()),
            MeshMaterial2d(handle),
            Transform::default(),
            layers,
        ));
    }
    // Debug overlay camera: composites over the lit frame, follows the scroll
    // camera, never lit.
    commands.spawn((
        Camera2d,
        Camera {
            order: first_post + 4,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        fixed_projection(view.x as f32, view.y as f32),
        Transform::default(),
        Msaa::Off,
        RenderLayers::layer(LAYER_DEBUG),
        SceneCamera,
    ));
    LightRig {
        emissive,
        composite,
        view,
    }
}

/// Write this frame's uniforms into the emissive and composite stages.
pub fn update_rig(
    rig: &LightRig,
    materials: &mut Assets<LightingMaterial>,
    frame: &PassFrame,
    window_px: Vec2,
) {
    let view = Vec2::new(rig.view.x as f32, rig.view.y as f32);
    if let Some(mut material) = materials.get_mut(&rig.emissive) {
        material.params = pack_uniform(frame, view);
    }
    if let Some(mut material) = materials.get_mut(&rig.composite) {
        material.params = pack_uniform(frame, window_px);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env_kit::EnvKit;
    use crate::lighting::{LightGroup, ViewRect, contact_shadow};
    use std::path::PathBuf;

    fn kit() -> EnvKit {
        EnvKit::load(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            "assets/levels/floor01/corridor_abc_sample.json",
        )
        .unwrap()
    }

    fn packed(left: i32, tick: u64) -> (LightingUniform, GatheredLights) {
        let kit = kit();
        let scene = kit.lighting.as_ref().unwrap();
        let view = ViewRect {
            left,
            top: 20,
            w: 640,
            h: 360,
        };
        let (gathered, _) = scene.gather(&kit, view, tick);
        let shadows = [contact_shadow(
            scene.profile.actor_contact_shadow.as_ref().unwrap(),
            391.0,
            308.0,
            36.0,
            view,
        )];
        let frame = PassFrame {
            scene,
            gathered: &gathered,
            shadows: &shadows,
            view_w: 640,
            view_h: 360,
            view_top: 20,
        };
        (pack_uniform(&frame, Vec2::new(1920.0, 1080.0)), gathered)
    }

    #[test]
    fn uniform_block_has_the_layout_the_shader_declares() {
        // 5 head vec4s + 8 shadows + 16 colours (16 B each), 6 slots x 64 B,
        // 64 lights x 48 B: the WGSL `Params` struct in light_pass.wgsl.
        let expected =
            (5 + MAX_SHADOWS + MAX_EMISSIVE_COLOURS) * 16 + MAX_SLOTS * 64 + MAX_LIGHTS * 48;
        assert_eq!(uniform_size() as usize, expected);
        assert_eq!(uniform_size(), 3920);
        assert_eq!(GpuLight::SHADER_SIZE.get(), 48);
        assert_eq!(GpuSlot::SHADER_SIZE.get(), 64);
        assert_eq!(<GpuLight as ShaderType>::min_size().get(), 48);
        // All array strides are multiples of 16 (uniform buffer rule).
        assert_eq!(GpuLight::SHADER_SIZE.get() % 16, 0);
        assert_eq!(GpuSlot::SHADER_SIZE.get() % 16, 0);
        let bytes = uniform_bytes(&LightingUniform::default());
        assert_eq!(bytes.len(), 3920);
    }

    #[test]
    fn field_offsets_match_the_wgsl_declaration() {
        let (u, g) = packed(328, 0);
        let bytes = uniform_bytes(&u);
        let f = |offset: usize| f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        let w = |offset: usize| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        assert_eq!((f(0), f(4), f(8), f(12)), (640.0, 360.0, 20.0, 6.0)); // view
        assert_eq!((f(16), f(20)), (1920.0, 1080.0)); // target
        assert_eq!(f(28), 0.7); // shadow strength
        assert_eq!(f(32), 214.0 / 255.0); // bloom threshold
        assert_eq!(w(64), g.far_end); // ranges
        assert_eq!(w(76), g.lights.len() as u32);
        // shadows start at 80, emissive at 80 + 128, slots at 80 + 128 + 256.
        assert_eq!(f(80), 391.0 - 328.0);
        assert_eq!(f(208), 138.0); // emissive[0].r (#8a2a12)
        let slots = 80 + 128 + 256;
        assert_eq!(f(slots), 0.74); // slot 0 ambient.r (parallax_far)
        let lights = slots + 6 * 64;
        let first = &g.lights[0];
        assert_eq!(f(lights), first.x);
        assert_eq!(f(lights + 8), first.radius);
        assert_eq!(f(lights + 16), first.colour[0]);
        assert_eq!(f(lights + 28), first.intensity);
        assert_eq!(f(lights + 32), first.glow_radius);
    }

    #[test]
    fn slot_parameters_come_from_the_group_profile() {
        let (u, g) = packed(328, 0);
        let kit = kit();
        let scene = kit.lighting.as_ref().unwrap();
        for (i, slot) in scene.plan.slots.iter().enumerate() {
            let p = scene.profile.group(slot.group);
            let s = &u.slots[i];
            assert_eq!(s.ambient.truncate().to_array(), p.ambient, "slot {i}");
            assert_eq!(s.ambient.w, p.light_scale.unwrap_or(1.0));
            let (start, end) = g.range_for(slot.group);
            assert_eq!((s.info.x, s.info.y), (start, end));
            let flags = s.info.z;
            assert_eq!(flags & FLAG_EMISSIVE != 0, slot.emissive);
            assert_eq!(flags & FLAG_PARALLAX != 0, slot.group.is_parallax());
            assert_eq!(flags & FLAG_SHADOW != 0, slot.group == LightGroup::Scene);
        }
        // Actors and foreground share the scene emitters.
        assert_eq!(
            (u.slots[4].info.x, u.slots[4].info.y),
            (u.slots[2].info.x, u.slots[2].info.y)
        );
        // Parallax slots have haze with ground fog; scene has none.
        assert_eq!(u.slots[0].fog.w, 1.0);
        assert_eq!(u.slots[2].fog.w, 0.0);
        assert_eq!(u.slots[0].haze.w, 0.26);
    }

    #[test]
    fn lights_are_packed_sorted_by_group_with_flicker_applied() {
        let (u0, g0) = packed(328, 0);
        let (u1, _) = packed(328, 9); // second frame of the 120/140/100/130 ms cycle
        assert!(g0.lights.len() <= MAX_LIGHTS);
        let n = g0.lights.len();
        // Unused tail stays zero (no stale lights).
        assert!(u0.lights[n..].iter().all(|l| *l == GpuLight::default()));
        // A flame's colour.w differs between frames (flicker 1.0 vs 0.93/...).
        let flame = |u: &LightingUniform| {
            u.lights
                .iter()
                .take(n)
                .filter(|l| l.pos.z == 250.0)
                .map(|l| l.colour.w)
                .collect::<Vec<_>>()
        };
        assert!(!flame(&u0).is_empty());
        assert_ne!(flame(&u0), flame(&u1));
        // Squash only on the floor pools.
        assert!(u0.lights.iter().take(n).any(|l| l.pos.w == 0.32));
    }

    // ---- shader <-> Rust agreement (naga is a test-only dependency)

    fn parsed_shader() -> naga::Module {
        let source = shader_source();
        naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)))
    }

    #[test]
    fn the_shader_validates_and_exposes_the_expected_entry_points() {
        let module = parsed_shader();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("light_pass.wgsl validates");
        let mut entries: Vec<&str> = module
            .entry_points
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        entries.sort_unstable();
        assert_eq!(entries, ["blur", "composite", "emissive"]);
        // Bindings: the uniform, one texture per slot, one aux texture.
        let bindings: Vec<u32> = module
            .global_variables
            .iter()
            .filter_map(|(_, g)| g.binding.as_ref().map(|b| b.binding))
            .collect();
        assert_eq!(bindings.len(), 2 + MAX_SLOTS);
        assert!(bindings.iter().all(|b| *b <= 1 + MAX_SLOTS as u32));
    }

    #[test]
    fn the_shader_params_block_matches_the_rust_uniform_layout() {
        let module = parsed_shader();
        let (_, params) = module
            .global_variables
            .iter()
            .find(|(_, g)| g.name.as_deref() == Some("params"))
            .expect("params uniform");
        let mut layouter = naga::proc::Layouter::default();
        layouter.update(module.to_ctx()).expect("layout");
        assert_eq!(
            u64::from(layouter[params.ty].size),
            uniform_size(),
            "WGSL Params size differs from LightingUniform (array lengths or fields out of step)"
        );
        // Member order and offsets, from the shader's own declaration.
        let naga::TypeInner::Struct { members, .. } = &module.types[params.ty].inner else {
            panic!("Params is a struct");
        };
        let names: Vec<&str> = members.iter().filter_map(|m| m.name.as_deref()).collect();
        assert_eq!(
            names,
            [
                "view",
                "target_px",
                "bloom",
                "blur",
                "ranges",
                "shadows",
                "emissive",
                "slots",
                "lights"
            ]
        );
        let offset = |name: &str| {
            members
                .iter()
                .find(|m| m.name.as_deref() == Some(name))
                .unwrap()
                .offset
        };
        assert_eq!(offset("shadows"), 80);
        assert_eq!(offset("emissive"), 80 + 16 * MAX_SHADOWS as u32);
        assert_eq!(
            offset("slots"),
            offset("emissive") + 16 * MAX_EMISSIVE_COLOURS as u32
        );
        assert_eq!(offset("lights"), offset("slots") + 64 * MAX_SLOTS as u32);
    }

    #[test]
    fn injected_constants_come_from_the_rust_side() {
        let source = shader_source();
        for needle in [
            "const MAX_LIGHTS: u32 = 64u;",
            "const MAX_SLOTS: u32 = 6u;",
            "const MAX_SHADOWS: u32 = 8u;",
            "const MAX_EMISSIVE: u32 = 16u;",
            "const FLAG_EMISSIVE: u32 = 1u;",
            "const FLAG_SHADOW: u32 = 2u;",
            "const FLAG_PARALLAX: u32 = 4u;",
            "const POOL_K: f32 = 5.0;",
            "const GLOW_K: f32 = 30.0;",
        ] {
            assert!(source.contains(needle), "missing {needle}");
        }
        // No second copy of a constant left in the body.
        assert!(!SHADER_BODY.contains("const FLAG_"));
        assert!(!SHADER_BODY.contains("5.0 * t2") && !SHADER_BODY.contains("30.0 * t2"));
    }

    #[test]
    #[should_panic(expected = "exceed the pass budget")]
    fn packing_more_lights_than_the_budget_is_a_loud_bug() {
        let kit = kit();
        let scene = kit.lighting.as_ref().unwrap();
        let template = crate::lighting::ViewLight {
            group: LightGroup::Scene,
            x: 0.0,
            y: 0.0,
            radius: 10.0,
            squash_y: 1.0,
            colour: [1.0; 3],
            intensity: 1.0,
            glow_radius: 0.0,
            glow_additive: 0.0,
            rank: 1.0,
        };
        let gathered = GatheredLights {
            lights: vec![template; MAX_LIGHTS + 1],
            ..Default::default()
        };
        let frame = PassFrame {
            scene,
            gathered: &gathered,
            shadows: &[],
            view_w: 640,
            view_h: 360,
            view_top: 20,
        };
        let _ = pack_uniform(&frame, Vec2::new(640.0, 360.0));
    }
}
