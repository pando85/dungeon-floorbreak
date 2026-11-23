//! BOIL-E (K10): the Bevy half of the Boiler's prop stand-ins. The pure half is
//! `dungeon_floorbreak::env_prop_art::pieces`; this file only draws what it
//! returns with a pool of code-drawn rectangles. **Presentation only and no new
//! art**: it reads the combat state through a shared reference and writes only its
//! own sprites, so nothing here can reach the simulation, the replay or the hash.
//! The room's props are labelled stand-ins until ART-BOIL lands, and the label is
//! drawn on screen: a small "STAND-IN: <name>" text over each one (K10).

use super::*;
use dungeon_floorbreak::encounter::Encounter;
use dungeon_floorbreak::env_prop_art::labels::{
    GLYPH_HEIGHT_PX, MAX_TAGS, STAND_IN_TEXTS, tags, text_pixels,
};
use dungeon_floorbreak::env_prop_art::{Layer, MAX_PIECES, pieces};

#[derive(Component, Clone, Copy)]
pub(super) struct PropPiece(usize);

/// One pooled on-screen label; its image is swapped to the text it shows.
#[derive(Component, Clone, Copy)]
pub(super) struct PropTag(usize);

/// The room's sidecar, for the authored prop rectangles and the z of each layer.
#[derive(Resource)]
pub(super) struct PropArt {
    encounter: Encounter,
    back_z: f32,
    prop_z: f32,
    tell_z: f32,
    /// The label z: above everything, never hidden.
    tag_z: f32,
    /// One rasterised image per label text.
    tag_images: Vec<(&'static str, Handle<Image>)>,
}

pub(super) fn spawn(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    encounter: Encounter,
    layers: RenderLayers,
    actor_z: f32,
    foreground_z: f32,
) {
    for i in 0..MAX_PIECES {
        commands.spawn((
            PropPiece(i),
            Sprite::from_color(Color::WHITE, Vec2::ONE),
            Transform::from_xyz(0.0, 0.0, actor_z),
            Visibility::Hidden,
            layers.clone(),
        ));
    }
    let mut tag_images: Vec<(&'static str, Handle<Image>)> = Vec::new();
    for (_, text) in STAND_IN_TEXTS {
        if tag_images.iter().any(|(t, _)| *t == text) {
            continue;
        }
        let Some(raster) = text_pixels(text) else {
            continue;
        };
        let image = images.add(Image::new(
            Extent3d {
                width: raster.w,
                height: raster.h,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            raster.rgba,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ));
        tag_images.push((text, image));
    }
    if let Some((_, first)) = tag_images.first() {
        for i in 0..MAX_TAGS {
            commands.spawn((
                PropTag(i),
                Sprite::from_image(first.clone()),
                Transform::from_xyz(0.0, 0.0, foreground_z + 0.1),
                Visibility::Hidden,
                layers.clone(),
            ));
        }
    }
    commands.insert_resource(PropArt {
        encounter,
        // Behind the actors; with the lane props (the drum is a solid, so no
        // actor stands in it); above the foreground so a warning is never hidden.
        back_z: actor_z - 0.5,
        prop_z: actor_z - 0.05,
        tell_z: foreground_z + 0.1,
        // Labels are read first: above the warnings too.
        tag_z: foreground_z + 0.2,
        tag_images,
    });
}

pub(super) fn sync_prop_art(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<Res<PropArt>>,
    mut sprites: Query<(&PropPiece, &mut Transform, &mut Sprite, &mut Visibility)>,
    mut tag_sprites: Query<
        (&PropTag, &mut Transform, &mut Sprite, &mut Visibility),
        Without<PropPiece>,
    >,
) {
    let (Some(art), Some(env)) = (art, combat.0.as_ref().and_then(|c| c.env())) else {
        return;
    };
    let world = &sim.0.world;
    let layout_h = world.layout_height();
    let drawn = pieces(&art.encounter, env, world.tick);
    for (slot, mut transform, mut sprite, mut visibility) in &mut sprites {
        match drawn.get(slot.0) {
            Some(p) => {
                let [r, g, b, a] = p.tone.rgba();
                sprite.color = Color::srgba_u8(r, g, b, a);
                sprite.custom_size = Some(Vec2::new(p.rect.w as f32, p.rect.h as f32));
                // Layout px (y down) to world px (y up); the sprite is centred.
                transform.translation.x = p.rect.x as f32 + p.rect.w as f32 / 2.0;
                transform.translation.y =
                    (layout_h - p.rect.bottom()) as f32 + p.rect.h as f32 / 2.0;
                transform.translation.z = match p.layer {
                    Layer::Back => art.back_z,
                    Layer::Prop => art.prop_z,
                    Layer::Tell => art.tell_z,
                };
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    // K10: the visible "STAND-IN: <name>" labels (presentation only).
    let drawn_tags = tags(&drawn);
    let glyph_h = GLYPH_HEIGHT_PX as f32 + 1.0;
    for (slot, mut transform, mut sprite, mut visibility) in &mut tag_sprites {
        let shown = drawn_tags.get(slot.0).and_then(|tag| {
            art.tag_images
                .iter()
                .find(|(text, _)| *text == tag.text)
                .map(|(_, image)| (tag, image))
        });
        match shown {
            Some((tag, image)) => {
                if sprite.image != *image {
                    sprite.image = image.clone();
                }
                // Whole pixels, so the glyphs stay sharp: the image is centred and
                // its bottom row sits on `bottom_y` (layout y down to world y up).
                transform.translation = Vec3::new(
                    tag.centre_x as f32,
                    (layout_h - tag.bottom_y) as f32 + glyph_h / 2.0,
                    // A distinct z per pool slot: labels never overlap (`tags`
                    // keeps them apart), and no draw order is left to a tie.
                    art.tag_z + slot.0 as f32 * 0.001,
                );
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}
