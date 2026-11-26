//! DR-A: the Bevy half of room 3's presentation (the Dozer revenge). The pure half is
//! `dungeon_floorbreak::dozer_art`; this file only draws what it returns: the approved DOZ-01 rig
//! (hull, tread, riders, front, wheel, yoke), its smoke and steam puffs, the DR-D effects (the
//! metal hit-spark and the ENV-02 splinter burst), and the labelled
//! code-drawn stand-ins of the rear gate, the buffer stop and the damage marker (DZ-AD13: until
//! DOZ-02). The machine itself is never a box.
//! **Presentation only**: the sync system reads the combat state through a shared reference and
//! writes only its own sprites, so nothing here reaches the simulation, the replay or the state
//! hash (the pure half proves it with a state-hash test).

use super::*;
use dungeon_floorbreak::dozer_art::{
    FX_Z, Layer as StandInLayer, MAX_FX, MAX_PUFFS, MAX_STAND_INS, MAX_TAGS, Part,
    Sheet as DozerSheet, TAG_TEXT, draw,
};
use dungeon_floorbreak::env_kit::ATLAS_ASSET_PREFIX;

/// One pooled rig or puff sprite.
#[derive(Component, Clone, Copy)]
pub(super) struct DozerSprite(usize);

/// One pooled stand-in rectangle.
#[derive(Component, Clone, Copy)]
pub(super) struct DozerStandIn(usize);

/// One pooled "STAND-IN" tag.
#[derive(Component, Clone, Copy)]
pub(super) struct DozerTag(usize);

/// One sheet's atlas: its image and its frame layout.
struct SheetAtlas {
    sheet: DozerSheet,
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

#[derive(Resource)]
pub(super) struct DozerArt {
    sheets: Vec<SheetAtlas>,
    /// The rig's base z: among the goblins' band, behind Carl (+0.1) and Donut.
    rig_z: f32,
    back_z: f32,
    front_z: f32,
    /// The DR-D effects: in front of every actor and the front stand-ins, under the tags.
    fx_z: f32,
    tag_z: f32,
    /// The "STAND-IN" tag image's size (px).
    tag_size: Vec2,
}

/// The z step between two rig layers (the rig's `z` is 0..=7; the effects, `FX_Z`, draw at
/// `fx_z` instead).
const RIG_Z_STEP: f32 = 0.001;
const SPRITE_POOL: usize = Part::ALL.len() + MAX_PUFFS + MAX_FX;

fn atlas_present(path: &str) -> bool {
    std::env::current_dir().is_ok_and(|root| root.join(path).is_file())
}

fn asset_path(path: &str) -> String {
    path.strip_prefix(ATLAS_ASSET_PREFIX)
        .unwrap_or(path)
        .to_owned()
}

/// Spawn the pools of room 3. A missing atlas (`bash scripts/export_assets.sh` was not run)
/// skips that sheet with a one-line notice.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    images: &mut Assets<Image>,
    layers: RenderLayers,
    kit: &EnvKit,
    handles: &mut Vec<(String, Handle<Image>)>,
    entities: &mut usize,
) {
    println!(
        "DRA_ART DOZ-01 rig ({} parts, {MAX_PUFFS} puffs, {MAX_FX} effects: hit-spark, splinter burst), labelled stand-ins for the gate, the buffer stop and damage until DOZ-02",
        Part::ALL.len()
    );
    let actor_z = kit.actor_z;
    let mut sheets = Vec::new();
    for sheet in DozerSheet::ALL {
        if !atlas_present(sheet.path()) {
            println!(
                "DRA_ART missing {} (run scripts/export_assets.sh): that sheet is skipped",
                sheet.path()
            );
            continue;
        }
        let asset = asset_path(sheet.path());
        let image: Handle<Image> = asset_server.load(asset.clone());
        handles.push((asset, image.clone()));
        let (w, h) = sheet.frame_size();
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(w, h),
            sheet.frames(),
            1,
            None,
            None,
        ));
        sheets.push(SheetAtlas {
            sheet,
            image,
            layout,
        });
    }
    let rig_z = actor_z + GOBLIN_Z_OFFSET - 40.0 * GOBLIN_Z_STEP;
    let text = dungeon_floorbreak::label_font::text_pixels(TAG_TEXT);
    commands.insert_resource(DozerArt {
        sheets,
        rig_z,
        back_z: actor_z - 0.5,
        // In front of every actor (Carl is +0.1, a glob +0.2).
        front_z: actor_z + 0.3,
        fx_z: actor_z + 0.305,
        tag_z: actor_z + 0.31,
        tag_size: text
            .as_ref()
            .map_or(Vec2::ZERO, |t| Vec2::new(t.w as f32, t.h as f32)),
    });
    for index in 0..SPRITE_POOL {
        commands.spawn((
            DozerSprite(index),
            Sprite::default(),
            Transform::from_xyz(0.0, 0.0, rig_z),
            Visibility::Hidden,
            layers.clone(),
        ));
        *entities += 1;
    }
    for index in 0..MAX_STAND_INS {
        commands.spawn((
            DozerStandIn(index),
            Sprite::from_color(Color::WHITE, Vec2::ONE),
            Transform::from_xyz(0.0, 0.0, actor_z),
            Visibility::Hidden,
            layers.clone(),
        ));
        *entities += 1;
    }
    if let Some(text) = text {
        let image = images.add(Image::new(
            Extent3d {
                width: text.w,
                height: text.h,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            text.rgba,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ));
        for index in 0..MAX_TAGS {
            commands.spawn((
                DozerTag(index),
                Sprite::from_image(image.clone()),
                Transform::from_xyz(0.0, 0.0, actor_z),
                Visibility::Hidden,
                layers.clone(),
            ));
            *entities += 1;
        }
    }
}

fn tone_colour(rgba: [u8; 4]) -> Color {
    Color::srgba_u8(rgba[0], rgba[1], rgba[2], rgba[3])
}

#[allow(clippy::type_complexity)]
pub(super) fn sync_dozer_art(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<Res<DozerArt>>,
    mut sprites: Query<
        (&DozerSprite, &mut Transform, &mut Sprite, &mut Visibility),
        (Without<DozerStandIn>, Without<DozerTag>),
    >,
    mut stand_ins: Query<
        (&DozerStandIn, &mut Transform, &mut Sprite, &mut Visibility),
        (Without<DozerSprite>, Without<DozerTag>),
    >,
    mut tags: Query<
        (&DozerTag, &mut Transform, &mut Visibility),
        (Without<DozerSprite>, Without<DozerStandIn>),
    >,
) {
    let (Some(art), Some(state)) = (art, combat.0.as_ref()) else {
        return;
    };
    let world = &sim.0.world;
    let Some(drawn) = draw(state, [world.hero.x, world.donut_body.x], world.tick) else {
        return;
    };
    for (slot, mut transform, mut sprite, mut visibility) in &mut sprites {
        let entry = drawn.sprites.get(slot.0);
        let atlas = entry.and_then(|s| art.sheets.iter().find(|a| a.sheet == s.sheet));
        match (entry, atlas) {
            (Some(s), Some(a)) => {
                let (x, y) = s.centre();
                // Later entries draw over earlier ones with the same rig z (puffs come
                // first). The DR-D effects draw in front of every actor: a burst on the
                // brace or a spark on the hull is where the hero's fist is.
                let base = if s.z == FX_Z {
                    art.fx_z
                } else {
                    art.rig_z + f32::from(s.z) * RIG_Z_STEP
                };
                transform.translation = Vec3::new(x, y, base + slot.0 as f32 * 0.00001);
                sprite.image = a.image.clone();
                sprite.texture_atlas = Some(TextureAtlas {
                    layout: a.layout.clone(),
                    index: usize::from(s.frame),
                });
                *visibility = Visibility::Inherited;
            }
            _ => *visibility = Visibility::Hidden,
        }
    }
    for (slot, mut transform, mut sprite, mut visibility) in &mut stand_ins {
        match drawn.stand_ins.get(slot.0) {
            Some(r) => {
                let z = match r.layer {
                    StandInLayer::Back => art.back_z,
                    StandInLayer::Front => art.front_z,
                } + slot.0 as f32 * 0.0001;
                *transform = Transform::from_xyz(
                    r.left as f32 + r.w as f32 / 2.0,
                    r.bottom as f32 + r.h as f32 / 2.0,
                    z,
                )
                .with_scale(Vec3::new(r.w as f32, r.h as f32, 1.0));
                sprite.color = tone_colour(r.tone.rgba());
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    for (slot, mut transform, mut visibility) in &mut tags {
        match drawn.tags.get(slot.0) {
            Some(&(x, y)) => {
                transform.translation = Vec3::new(
                    x as f32 + art.tag_size.x / 2.0,
                    y as f32 + art.tag_size.y / 2.0,
                    art.tag_z,
                );
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}
