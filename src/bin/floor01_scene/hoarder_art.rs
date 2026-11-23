//! FA-Q23 (HRD-P): the Bevy half of the Hoarder room's stand-in presentation. The pure half is
//! `dungeon_floorbreak::hoarder_art`; this file only draws what it returns: the HOARD-01
//! atlas frames (the committed, unapproved candidate, reused as they are), the labelled
//! greybox of the door, the entry gate, bank A, the J-B post and the lair mound, and the
//! visible "STAND-IN" labels (rasterised from a 3 x 5 pixel font into one small image each).
//! **Presentation only**: every system reads the combat state through a shared reference and
//! writes only its own sprites, so nothing here reaches the simulation, the replay or the
//! state hash (the pure half proves it with a state-hash test).

use super::*;
use dungeon_floorbreak::encounter::Encounter;
use dungeon_floorbreak::env_kit::ATLAS_ASSET_PREFIX;
use dungeon_floorbreak::hoarder_art::{
    ATLAS, FRAME_SIZE, FRAMES, HoarderLook, HoarderMemo, Layer, MAX_PIECES, TagKind, draw,
    text_pixels,
};

/// Her pooled sprite.
#[derive(Component, Clone, Copy)]
pub(super) struct HoarderSprite;

/// One pooled code-drawn piece (door, gate, bank, post, mound).
#[derive(Component, Clone, Copy)]
pub(super) struct HoarderPiece(usize);

/// One visible label.
#[derive(Component, Clone, Copy)]
pub(super) struct HoarderTagSprite(TagKind);

#[derive(Resource)]
pub(super) struct HoarderArt {
    encounter: Encounter,
    hoarder_z: f32,
    back_z: f32,
    piece_z: f32,
    tag_z: f32,
}

fn atlas_present(path: &str) -> bool {
    std::env::current_dir().is_ok_and(|root| root.join(path).is_file())
}

/// Spawn the pools of the Hoarder room. A missing atlas (`bash scripts/export_assets.sh` was
/// not run) skips her sprite with a one-line notice; the greybox and the labels still draw.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    images: &mut Assets<Image>,
    encounter: Encounter,
    layers: RenderLayers,
    kit: &EnvKit,
    handles: &mut Vec<(String, Handle<Image>)>,
    entities: &mut usize,
) {
    println!("HRD_P_ART stand-in presentation (HOARD-01 frames, labelled greybox)");
    let actor_z = kit.actor_z;
    commands.insert_resource(HoarderArt {
        encounter,
        // Among the goblins' band, behind Carl and Donut (the slime's convention).
        hoarder_z: actor_z + GOBLIN_Z_OFFSET - 32.0 * GOBLIN_Z_STEP,
        back_z: actor_z - 0.5,
        piece_z: actor_z - 0.045,
        // The labels are read first: above everything, never hidden.
        tag_z: kit.foreground_z + 0.1,
    });
    if atlas_present(ATLAS) {
        let asset = ATLAS
            .strip_prefix(ATLAS_ASSET_PREFIX)
            .unwrap_or(ATLAS)
            .to_owned();
        let texture: Handle<Image> = asset_server.load(asset.clone());
        handles.push((asset, texture.clone()));
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(FRAME_SIZE.0, FRAME_SIZE.1),
            FRAMES,
            1,
            None,
            None,
        ));
        commands.spawn((
            HoarderSprite,
            Sprite::from_atlas_image(texture, TextureAtlas { layout, index: 0 }),
            Transform::from_xyz(0.0, 0.0, actor_z),
            Visibility::Hidden,
            layers.clone(),
        ));
        *entities += 1;
    } else {
        println!("HRD_P_ART missing {ATLAS} (run scripts/export_assets.sh): her sprite is skipped");
    }
    for index in 0..MAX_PIECES {
        commands.spawn((
            HoarderPiece(index),
            Sprite::from_color(Color::WHITE, Vec2::ONE),
            Transform::from_xyz(0.0, 0.0, actor_z),
            Visibility::Hidden,
            layers.clone(),
        ));
        *entities += 1;
    }
    for kind in TagKind::ALL {
        let Some(text) = text_pixels(kind.text()) else {
            continue;
        };
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
        commands.spawn((
            HoarderTagSprite(kind),
            Sprite::from_image(image),
            Transform::from_xyz(0.0, 0.0, actor_z),
            Visibility::Hidden,
            layers.clone(),
        ));
        *entities += 1;
    }
}

#[allow(clippy::type_complexity)]
pub(super) fn sync_hoarder_art(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<Res<HoarderArt>>,
    mut memo: Local<HoarderMemo>,
    mut hoarder_sprite: Query<
        (&mut Transform, &mut Sprite, &mut Visibility),
        (
            With<HoarderSprite>,
            Without<HoarderPiece>,
            Without<HoarderTagSprite>,
        ),
    >,
    mut piece_sprites: Query<
        (&HoarderPiece, &mut Transform, &mut Sprite, &mut Visibility),
        (Without<HoarderSprite>, Without<HoarderTagSprite>),
    >,
    mut tag_sprites: Query<
        (&HoarderTagSprite, &mut Transform, &mut Visibility),
        (Without<HoarderSprite>, Without<HoarderPiece>),
    >,
) {
    let (Some(art), Some(state)) = (art, combat.0.as_ref()) else {
        return;
    };
    let world = &sim.0.world;
    let tick = world.tick;
    // Render-local looks (the hit flash, the age of her defeat), kept by the pure half.
    let look = dungeon_floorbreak::combat_scene::pile_boss::of(state)
        .map_or_else(HoarderLook::default, |enemy| {
            memo.observe(enemy.boss().hp(), enemy.boss().alive(), tick)
        });
    let Some(drawn) = draw(state, &art.encounter, world.layout_height(), tick, look) else {
        return;
    };
    for (mut transform, mut sprite, mut visibility) in &mut hoarder_sprite {
        match drawn.hoarder {
            Some(d) => {
                transform.translation = Vec3::new(d.centre.0, d.centre.1, art.hoarder_z);
                sprite.flip_x = d.flip_x;
                sprite.color = if d.flash { FLASH_TINT } else { Color::WHITE }.with_alpha(d.alpha);
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = usize::from(d.frame);
                }
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    for (piece_id, mut transform, mut sprite, mut visibility) in &mut piece_sprites {
        match drawn.pieces.get(piece_id.0) {
            Some(p) => {
                let [r, g, b, a] = p.tone.rgba();
                sprite.color = Color::srgba_u8(r, g, b, a);
                sprite.custom_size = Some(Vec2::new(p.w as f32, p.h as f32));
                transform.translation.x = p.x as f32 + p.w as f32 / 2.0;
                transform.translation.y = p.y as f32 + p.h as f32 / 2.0;
                transform.translation.z = match p.layer {
                    Layer::Back => art.back_z,
                    Layer::Prop => art.piece_z,
                };
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    for (tag_id, mut transform, mut visibility) in &mut tag_sprites {
        match drawn.tags.iter().find(|t| t.kind == tag_id.0) {
            Some(t) => {
                // The image is centred; its bottom row sits on `bottom_y`. Whole pixels, so the
                // glyphs stay sharp.
                let h = dungeon_floorbreak::hoarder_art::GLYPH_HEIGHT_PX as f32 + 1.0;
                transform.translation = Vec3::new(
                    t.centre_x.round(),
                    (t.bottom_y + h / 2.0).round(),
                    art.tag_z,
                );
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}
