//! HUD-HYBRID-A-01: the Bevy adapter of the hybrid A HUD.
//!
//! One dedicated HUD `Camera2d` (render layer 15, fixed 640 x view-height
//! projection, never a `SceneCamera`, so scroll, camera kick and shake never
//! move it and the lighting pass never lights it) draws pooled sprites and
//! `Text2d` lines on whole-pixel positions. No `bevy_ui`. All art lookups go
//! through [`HudArt`]: the `hud_hybrid_a` atlas when its manifest is present,
//! palette-coloured placeholder rects of the spec sizes otherwise.
//!
//! The adapter reads only [`HudRuntime`] (a presentation state); it never
//! touches the simulation. The draw list is rebuilt only when the runtime's
//! revision changes, so a steady scene allocates nothing here.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::{
    camera::{ScalingMode, visibility::RenderLayers},
    prelude::*,
    sprite::Anchor,
    text::{FontHinting, FontSmoothing, LineHeight},
};

use crate::env_kit::IRect;
use crate::hud::{HudFont, HudLayout, HudRuntime, Rgb, VIEW_WIDTH};
use crate::hud_draw::{DrawItem, DrawKind, FrameId, build_draw_list, placeholder_rects};

/// The HUD's render layer (the lighting pass uses 10 to 14).
pub const LAYER_HUD: usize = 15;

const ATLAS_MANIFEST: &str = "assets/manifests/hud_hybrid_a_v1.json";
/// Asset path (below `generated/art`) of the atlas the manifest names.
const ATLAS_ASSET: &str = "hud_hybrid_a.png";
const FONT_JERSEY: &str = "assets/fonts/jersey10/Jersey10-Regular.ttf";
const FONT_SILK_REGULAR: &str = "assets/fonts/silkscreen/Silkscreen-Regular.ttf";
const FONT_SILK_BOLD: &str = "assets/fonts/silkscreen/Silkscreen-Bold.ttf";

/// Pool sizes (a test proves the worst case fits).
pub const SPRITE_POOL: usize = 1100;
pub const TEXT_POOL_PER_FONT: usize = 8;

/// Distance from the top of a text block to the top of its capital letters,
/// per font, measured on a 1x capture (see `docs/03-tech/floor01-scene.md`).
const fn caps_offset(font: HudFont) -> i32 {
    match font {
        HudFont::Jersey => 5,
        HudFont::SilkRegular | HudFont::SilkBold => 3,
    }
}

/// Font pixel sizes of the HUD grid: Jersey 10 at 1400/75 px gives 10 px caps
/// on integer advances; Silkscreen at 8 px gives 5 px caps.
const fn font_px(font: HudFont) -> f32 {
    match font {
        HudFont::Jersey => 1400.0 / 75.0,
        HudFont::SilkRegular | HudFont::SilkBold => 8.0,
    }
}

const fn line_height_px(font: HudFont) -> f32 {
    match font {
        HudFont::Jersey => 20.0,
        HudFont::SilkRegular | HudFont::SilkBold => 10.0,
    }
}

// ------------------------------------------------------------------ art data

/// Atlas rects by frame, from the manifest's `hud.regions`.
#[derive(Debug, Clone)]
pub struct HudAtlasData {
    pub rects: HashMap<FrameId, IRect>,
}

impl HudAtlasData {
    /// Read `assets/manifests/hud_hybrid_a_v1.json`. `Ok(None)` when the
    /// manifest or the exported atlas is missing (placeholders are used);
    /// `Err` when a manifest is present but does not match the spec.
    pub fn load(root: &Path) -> Result<Option<Self>, String> {
        let manifest = root.join(ATLAS_MANIFEST);
        let atlas = root.join("generated/art").join(ATLAS_ASSET);
        if !manifest.is_file() || !atlas.is_file() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&manifest)
            .map_err(|e| format!("{}: {e}", manifest.display()))?;
        Self::parse(&text).map(Some)
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|e| format!("HUD manifest: {e}"))?;
        let regions = value["hud"]["regions"]
            .as_array()
            .ok_or("HUD manifest: missing hud.regions")?;
        let mut by_name: HashMap<&str, IRect> = HashMap::new();
        for region in regions {
            let name = region["name"].as_str().ok_or("HUD region without a name")?;
            let rect = region["rect"]
                .as_array()
                .filter(|r| r.len() == 4)
                .ok_or_else(|| format!("HUD region {name}: bad rect"))?;
            let n = |i: usize| rect[i].as_i64().map(|v| v as i32);
            let (Some(x), Some(y), Some(w), Some(h)) = (n(0), n(1), n(2), n(3)) else {
                return Err(format!("HUD region {name}: bad rect"));
            };
            by_name.insert(name, IRect { x, y, w, h });
        }
        let mut rects = HashMap::new();
        for frame in FrameId::ALL {
            let rect = by_name
                .get(frame.name())
                .ok_or_else(|| format!("HUD manifest: missing region {}", frame.name()))?;
            if (rect.w, rect.h) != frame.size() {
                return Err(format!(
                    "HUD region {} is {}x{}, the spec says {:?}",
                    frame.name(),
                    rect.w,
                    rect.h,
                    frame.size()
                ));
            }
            rects.insert(frame, *rect);
        }
        Ok(Self { rects })
    }
}

/// Every art lookup of the HUD, behind one resource keyed by frame name.
#[derive(Resource)]
pub struct HudArt {
    texture: Option<Handle<Image>>,
    rects: HashMap<FrameId, IRect>,
}

impl HudArt {
    pub fn is_placeholder(&self) -> bool {
        self.texture.is_none()
    }

    /// The atlas rect of a frame, if the art is loaded.
    fn region(&self, frame: FrameId) -> Option<(&Handle<Image>, IRect)> {
        Some((self.texture.as_ref()?, *self.rects.get(&frame)?))
    }
}

// ------------------------------------------------------------------ plugin

/// Font bytes read at startup (a missing font is a setup failure).
struct FontBytes {
    jersey: Vec<u8>,
    silk_regular: Vec<u8>,
    silk_bold: Vec<u8>,
}

/// Everything the HUD needs, read before the app starts. `msaa` must equal the
/// scene camera's sample count (all cameras on one target must agree): `Off`
/// with the lighting pass, the Bevy default without it.
pub struct HudPlugin {
    runtime: HudRuntime,
    layout: HudLayout,
    camera_order: isize,
    msaa: Msaa,
    atlas: Option<HudAtlasData>,
    fonts: std::sync::Mutex<Option<FontBytes>>,
}

impl HudPlugin {
    pub fn load(
        root: &Path,
        runtime: HudRuntime,
        layout: HudLayout,
        camera_order: isize,
        msaa: Msaa,
    ) -> Result<Self, String> {
        let read = |rel: &str| -> Result<Vec<u8>, String> {
            let path: PathBuf = root.join(rel);
            std::fs::read(&path).map_err(|e| format!("HUD font {}: {e}", path.display()))
        };
        Ok(Self {
            runtime,
            layout,
            camera_order,
            msaa,
            atlas: HudAtlasData::load(root)?,
            fonts: std::sync::Mutex::new(Some(FontBytes {
                jersey: read(FONT_JERSEY)?,
                silk_regular: read(FONT_SILK_REGULAR)?,
                silk_bold: read(FONT_SILK_BOLD)?,
            })),
        })
    }

    /// Asset path of the atlas image when the art is present (so the scene's
    /// loading watchdog can wait for it).
    pub fn atlas_asset(&self) -> Option<&'static str> {
        self.atlas.as_ref().map(|_| ATLAS_ASSET)
    }
}

/// The runtime the scene steps once per fixed tick.
#[derive(Resource)]
pub struct HudState(pub HudRuntime);

#[derive(Resource)]
struct HudRender {
    layout: HudLayout,
    camera_order: isize,
    msaa: Msaa,
    atlas: Option<HudAtlasData>,
    fonts: Option<FontBytes>,
    last_revision: Option<u64>,
    items: Vec<DrawItem>,
    sprites: Vec<Entity>,
    texts: [Vec<Entity>; 3],
    used_sprites: usize,
    used_texts: [usize; 3],
    overflow_reported: bool,
}

#[derive(Component)]
struct HudSprite;

#[derive(Component)]
struct HudText;

type SpriteQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Sprite,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    With<HudSprite>,
>;
type TextQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Text2d,
        &'static mut TextColor,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (With<HudText>, Without<HudSprite>),
>;

const fn font_index(font: HudFont) -> usize {
    match font {
        HudFont::Jersey => 0,
        HudFont::SilkRegular => 1,
        HudFont::SilkBold => 2,
    }
}

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(HudState(self.runtime.clone()))
            .insert_resource(HudRender {
                layout: self.layout,
                camera_order: self.camera_order,
                msaa: self.msaa,
                atlas: self.atlas.clone(),
                fonts: self.fonts.lock().ok().and_then(|mut f| f.take()),
                last_revision: None,
                items: Vec::with_capacity(512),
                sprites: Vec::new(),
                texts: [Vec::new(), Vec::new(), Vec::new()],
                used_sprites: 0,
                used_texts: [0; 3],
                overflow_reported: false,
            })
            .add_systems(Startup, spawn_hud)
            .add_systems(Update, sync_hud);
    }
}

fn color_of(c: Rgb) -> Color {
    Color::srgb_u8(c.0, c.1, c.2)
}

fn tint_of(t: Rgb) -> Color {
    color_of(t)
}

/// Multiply a placeholder colour by a tint (the same rule the GPU applies to
/// an atlas frame, in sRGB for the placeholder).
fn multiply(c: Rgb, t: Rgb) -> Rgb {
    let m = |a: u8, b: u8| (u16::from(a) * u16::from(b) / 255) as u8;
    Rgb(m(c.0, t.0), m(c.1, t.1), m(c.2, t.2))
}

/// Top-left virtual pixel -> world centre-free translation (y up) for a
/// top-left anchored entity.
fn to_world(x: i32, y: i32, view_h: i32, z: f32) -> Vec3 {
    Vec3::new(x as f32, (view_h - y) as f32, z)
}

fn spawn_hud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut fonts: ResMut<Assets<Font>>,
    mut render: ResMut<HudRender>,
) {
    let layers = RenderLayers::layer(LAYER_HUD);
    let view_h = render.layout.view_h;
    commands.spawn((
        Camera2d,
        Camera {
            order: render.camera_order,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: VIEW_WIDTH as f32,
                height: view_h as f32,
            },
            ..OrthographicProjection::default_2d()
        }),
        // Constant: the HUD never follows the scroll camera.
        Transform::from_xyz(VIEW_WIDTH as f32 / 2.0, view_h as f32 / 2.0, 0.0),
        render.msaa,
        layers.clone(),
    ));
    let bytes = render.fonts.take().expect("HUD fonts are read at load");
    let handles = [
        fonts.add(Font::from_bytes(bytes.jersey)),
        fonts.add(Font::from_bytes(bytes.silk_regular)),
        fonts.add(Font::from_bytes(bytes.silk_bold)),
    ];
    let texture = render
        .atlas
        .as_ref()
        .map(|_| asset_server.load::<Image>(ATLAS_ASSET));
    commands.insert_resource(HudArt {
        texture,
        rects: render
            .atlas
            .as_ref()
            .map(|a| a.rects.clone())
            .unwrap_or_default(),
    });
    for _ in 0..SPRITE_POOL {
        let id = commands
            .spawn((
                HudSprite,
                Sprite::default(),
                Anchor::TOP_LEFT,
                Transform::default(),
                Visibility::Hidden,
                layers.clone(),
            ))
            .id();
        render.sprites.push(id);
    }
    for (i, font) in [HudFont::Jersey, HudFont::SilkRegular, HudFont::SilkBold]
        .into_iter()
        .enumerate()
    {
        for _ in 0..TEXT_POOL_PER_FONT {
            let id = commands
                .spawn((
                    HudText,
                    Text2d::new(""),
                    TextFont {
                        font: handles[i].clone().into(),
                        font_size: FontSize::Px(font_px(font)),
                        font_smoothing: FontSmoothing::None,
                        ..default()
                    },
                    LineHeight::Px(line_height_px(font)),
                    FontHinting::Enabled,
                    TextColor(Color::WHITE),
                    Anchor::TOP_LEFT,
                    Transform::default(),
                    Visibility::Hidden,
                    layers.clone(),
                ))
                .id();
            render.texts[i].push(id);
        }
    }
}

/// How many pooled sprites a draw list needs once frames are expanded.
pub fn sprite_cost(items: &[DrawItem], atlas: bool) -> usize {
    items
        .iter()
        .map(|item| match &item.kind {
            DrawKind::Rect { .. } => 1,
            DrawKind::Frame { frame, w, h, .. } => {
                if atlas {
                    1
                } else {
                    placeholder_rects(*frame, *w, *h).len()
                }
            }
            DrawKind::Text { .. } => 0,
        })
        .sum()
}

#[allow(clippy::too_many_arguments)]
fn sync_hud(
    state: Res<HudState>,
    art: Option<Res<HudArt>>,
    mut render: ResMut<HudRender>,
    mut sprites: SpriteQuery,
    mut texts: TextQuery,
) {
    let Some(art) = art else { return };
    let revision = state.0.revision();
    if render.last_revision == Some(revision) {
        return;
    }
    render.last_revision = Some(revision);
    let snapshot = state.0.snapshot();
    let render = &mut *render;
    let layout = render.layout;
    build_draw_list(&snapshot, &layout, &mut render.items);
    let view_h = layout.view_h;

    let mut si = 0usize;
    let mut ti = [0usize; 3];
    let mut overflow = false;
    for item in &render.items {
        match &item.kind {
            DrawKind::Rect { w, h, color } => {
                place_sprite(
                    &mut sprites,
                    &render.sprites,
                    &mut si,
                    &mut overflow,
                    Placement {
                        x: item.x,
                        y: item.y,
                        z: item.z,
                        w: *w,
                        h: *h,
                        color: *color,
                        frame: None,
                    },
                    &art,
                    view_h,
                );
            }
            DrawKind::Frame { frame, w, h, tint } => {
                if let Some((_, rect)) = art.region(*frame) {
                    place_sprite(
                        &mut sprites,
                        &render.sprites,
                        &mut si,
                        &mut overflow,
                        Placement {
                            x: item.x,
                            y: item.y,
                            z: item.z,
                            w: *w,
                            h: *h,
                            color: tint.unwrap_or(Rgb(255, 255, 255)),
                            frame: Some(rect),
                        },
                        &art,
                        view_h,
                    );
                } else {
                    for (k, (rx, ry, rw, rh, color)) in
                        placeholder_rects(*frame, *w, *h).into_iter().enumerate()
                    {
                        let color = tint.map_or(color, |t| multiply(color, t));
                        place_sprite(
                            &mut sprites,
                            &render.sprites,
                            &mut si,
                            &mut overflow,
                            Placement {
                                x: item.x + rx,
                                y: item.y + ry,
                                z: item.z + k as f32 * 0.00001,
                                w: rw,
                                h: rh,
                                color,
                                frame: None,
                            },
                            &art,
                            view_h,
                        );
                    }
                }
            }
            DrawKind::Text { font, text, color } => {
                let f = font_index(*font);
                let Some(&entity) = render.texts[f].get(ti[f]) else {
                    overflow = true;
                    continue;
                };
                ti[f] += 1;
                if let Ok((mut t, mut c, mut tr, mut vis)) = texts.get_mut(entity) {
                    if t.0 != *text {
                        t.0.clear();
                        t.0.push_str(text);
                    }
                    c.set_if_neq(TextColor(color_of(*color)));
                    tr.translation = to_world(item.x, item.y - caps_offset(*font), view_h, item.z);
                    vis.set_if_neq(Visibility::Inherited);
                }
            }
        }
    }
    // Hide the pool entities this list no longer uses.
    for &entity in render.sprites[si..render.used_sprites.max(si)].iter() {
        if let Ok((_, _, mut vis)) = sprites.get_mut(entity) {
            vis.set_if_neq(Visibility::Hidden);
        }
    }
    for f in 0..3 {
        for &entity in render.texts[f][ti[f]..render.used_texts[f].max(ti[f])].iter() {
            if let Ok((_, _, _, mut vis)) = texts.get_mut(entity) {
                vis.set_if_neq(Visibility::Hidden);
            }
        }
    }
    render.used_sprites = si;
    render.used_texts = ti;
    if overflow && !render.overflow_reported {
        render.overflow_reported = true;
        eprintln!("FLOOR1_HUD_WARNING: HUD pool exhausted; some HUD items were not drawn");
    }
}

struct Placement {
    x: i32,
    y: i32,
    z: f32,
    w: i32,
    h: i32,
    color: Rgb,
    frame: Option<IRect>,
}

fn place_sprite(
    sprites: &mut SpriteQuery,
    pool: &[Entity],
    cursor: &mut usize,
    overflow: &mut bool,
    p: Placement,
    art: &HudArt,
    view_h: i32,
) {
    let Some(&entity) = pool.get(*cursor) else {
        *overflow = true;
        return;
    };
    *cursor += 1;
    let Ok((mut sprite, mut transform, mut vis)) = sprites.get_mut(entity) else {
        return;
    };
    sprite.custom_size = Some(Vec2::new(p.w as f32, p.h as f32));
    match (p.frame, art.texture.as_ref()) {
        (Some(rect), Some(texture)) => {
            sprite.image = texture.clone();
            sprite.rect = Some(Rect::new(
                rect.x as f32,
                rect.y as f32,
                (rect.x + rect.w) as f32,
                (rect.y + rect.h) as f32,
            ));
            sprite.color = tint_of(p.color);
        }
        _ => {
            if sprite.image != Handle::<Image>::default() {
                sprite.image = Handle::default();
            }
            sprite.rect = None;
            sprite.color = color_of(p.color);
        }
    }
    transform.translation = to_world(p.x, p.y, view_h, p.z);
    vis.set_if_neq(Visibility::Inherited);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::{FaceFrame, GlyphSet, HudConfig, HudPreview, RealHudInputs, build_snapshot};

    fn lists() -> Vec<Vec<DrawItem>> {
        let mut all = Vec::new();
        for deck in [false, true] {
            let layout = if deck {
                HudLayout::new(400, 308)
            } else {
                HudLayout::new(360, 288)
            };
            for glyphs in [GlyphSet::Xbox, GlyphSet::DualSense] {
                for reduced in [false, true] {
                    for preview in
                        std::iter::once(None).chain(HudPreview::ALL.into_iter().map(Some))
                    {
                        for (hp, age) in [(4u8, None), (1, Some(0u8)), (2, Some(8))] {
                            let mut real = RealHudInputs::normal(hp, 4);
                            real.glyphs = glyphs;
                            real.reduced_motion = reduced;
                            real.hp_flash = age.map(|a| crate::hud::HpFlash {
                                lost_from: hp + 1,
                                lost_to: hp,
                                age_ticks: a,
                            });
                            real.notice = Some(crate::hud::NoticeView {
                                headline: crate::hud::LabelId::Flattened,
                                age_ticks: 10,
                            });
                            let mut out = Vec::new();
                            build_draw_list(
                                &build_snapshot(&real, preview, 239),
                                &layout,
                                &mut out,
                            );
                            all.push(out);
                        }
                    }
                }
            }
        }
        all
    }

    #[test]
    fn the_worst_case_fits_the_pools_with_and_without_the_atlas() {
        let mut worst_placeholder = 0;
        for list in lists() {
            worst_placeholder = worst_placeholder.max(sprite_cost(&list, false));
            assert!(sprite_cost(&list, true) <= SPRITE_POOL);
            for font in [HudFont::Jersey, HudFont::SilkRegular, HudFont::SilkBold] {
                let n = list
                    .iter()
                    .filter(|i| matches!(&i.kind, DrawKind::Text { font: f, .. } if *f == font))
                    .count();
                assert!(n <= TEXT_POOL_PER_FONT, "{font:?}: {n}");
            }
        }
        assert!(worst_placeholder <= SPRITE_POOL, "{worst_placeholder}");
    }

    #[test]
    fn the_manifest_parser_binds_every_frame_and_checks_sizes() {
        let mut regions = Vec::new();
        for (i, f) in FrameId::ALL.iter().enumerate() {
            let (w, h) = f.size();
            regions.push(format!(
                r#"{{"name":"{}","rect":[{},0,{w},{h}]}}"#,
                f.name(),
                i * 300
            ));
        }
        let ok = format!(r#"{{"hud":{{"regions":[{}]}}}}"#, regions.join(","));
        let atlas = HudAtlasData::parse(&ok).unwrap();
        assert_eq!(atlas.rects.len(), FrameId::ALL.len());
        let bad = ok.replace(r#""rect":[0,0,164,48]"#, r#""rect":[0,0,160,48]"#);
        assert!(
            HudAtlasData::parse(&bad)
                .unwrap_err()
                .contains("plate_frame")
        );
        assert!(HudAtlasData::parse(r#"{"hud":{"regions":[]}}"#).is_err());
    }

    #[test]
    fn the_real_manifest_when_present_matches_the_spec() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let path = root.join(ATLAS_MANIFEST);
        if let Ok(text) = std::fs::read_to_string(path) {
            HudAtlasData::parse(&text).expect("art manifest matches the HUD frame names");
        }
    }

    #[test]
    fn micro_checks_on_helpers() {
        assert_eq!(to_world(10, 8, 360, 1.0), Vec3::new(10.0, 352.0, 1.0));
        assert_eq!(
            multiply(Rgb(255, 255, 255), Rgb(168, 116, 42)),
            Rgb(168, 116, 42)
        );
        let _ = (FaceFrame::Idle, HudConfig::default());
    }
}
