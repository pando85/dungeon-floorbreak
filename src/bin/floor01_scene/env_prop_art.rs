//! BOIL-E (K10) and AREA-GOBLINWORKSHOP-01: the Bevy half of the Boiler's prop
//! art. The pure half is `dungeon_floorbreak::env_prop_art`: `sprites` lists the
//! frames of the Goblin Workshop kit (one atlas per canvas, `Sheet`) and `pieces`
//! the code-drawn rectangles. This file draws both with pools. A rectangle that
//! stands in for a sheet is drawn only while that sheet's atlas is missing
//! (`bash scripts/export_assets.sh` was not run, one notice per sheet); the
//! footprints (steam warning area and edge, the drum's blast area) always draw.
//! **Presentation only**: it reads the combat state through a shared reference and
//! writes only its own sprites, so nothing here can reach the simulation, the
//! replay or the hash.

use super::*;
use dungeon_floorbreak::encounter::Encounter;
use dungeon_floorbreak::env_kit::ATLAS_ASSET_PREFIX;
use dungeon_floorbreak::env_prop_art::{Layer, MAX_PIECES, MAX_SPRITES, Sheet, pieces, sprites};
use dungeon_floorbreak::ring_shutter_art::{self, PhaseClock};
use std::collections::BTreeMap;

#[derive(Component, Clone, Copy)]
pub(super) struct PropPiece(usize);

/// One pooled kit sprite.
#[derive(Component, Clone, Copy)]
pub(super) struct PropSprite(usize);

/// The room's sidecar, for the authored prop rectangles, the z of each layer and
/// the atlases that loaded.
#[derive(Resource)]
pub(super) struct PropArt {
    encounter: Encounter,
    back_z: f32,
    prop_z: f32,
    tell_z: f32,
    sheets: Vec<(Sheet, Handle<Image>, Handle<TextureAtlasLayout>)>,
}

impl PropArt {
    fn sheet(&self, sheet: Sheet) -> Option<&(Sheet, Handle<Image>, Handle<TextureAtlasLayout>)> {
        self.sheets.iter().find(|(s, _, _)| *s == sheet)
    }
}

fn atlas_present(path: &str) -> bool {
    std::env::current_dir().is_ok_and(|root| root.join(path).is_file())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    handles: &mut Vec<(String, Handle<Image>)>,
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
    let mut sheets = Vec::new();
    for sheet in Sheet::ALL {
        let path = sheet.atlas();
        if !atlas_present(path) {
            println!(
                "GW_KIT missing {path} (run scripts/export_assets.sh): its stand-in rectangles draw instead"
            );
            continue;
        }
        // The asset server's root is the atlas folder (`ATLAS_ASSET_PREFIX`).
        let asset = path
            .strip_prefix(ATLAS_ASSET_PREFIX)
            .unwrap_or(path)
            .to_owned();
        let texture: Handle<Image> = asset_server.load(asset.clone());
        handles.push((asset, texture.clone()));
        let (w, h) = sheet.frame_size();
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(w as u32, h as u32),
            sheet.frames() as u32,
            1,
            None,
            None,
        ));
        sheets.push((sheet, texture, layout));
    }
    if let Some((_, texture, layout)) = sheets.first() {
        for i in 0..MAX_SPRITES {
            commands.spawn((
                PropSprite(i),
                Sprite::from_atlas_image(
                    texture.clone(),
                    TextureAtlas {
                        layout: layout.clone(),
                        index: 0,
                    },
                ),
                Transform::from_xyz(0.0, 0.0, actor_z),
                Visibility::Hidden,
                layers.clone(),
            ));
        }
    }
    println!(
        "GW_KIT sheets={}/{} (prototype art, owner review pending)",
        sheets.len(),
        Sheet::ALL.len()
    );
    commands.insert_resource(PropArt {
        encounter,
        // Behind the actors; with the lane props (the drum is a solid, so no
        // actor stands in it); above the foreground so a warning is never hidden.
        back_z: actor_z - 0.5,
        prop_z: actor_z - 0.05,
        tell_z: foreground_z + 0.1,
        sheets,
    });
}

fn layer_z(art: &PropArt, layer: Layer) -> f32 {
    match layer {
        Layer::Back => art.back_z,
        Layer::Prop => art.prop_z,
        Layer::Tell => art.tell_z,
    }
}

pub(super) fn sync_prop_art(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<Res<PropArt>>,
    ring_shutter: Option<Res<RingShutterArt>>,
    mut pieces_q: Query<(&PropPiece, &mut Transform, &mut Sprite, &mut Visibility)>,
    mut sprites_q: Query<
        (&PropSprite, &mut Transform, &mut Sprite, &mut Visibility),
        Without<PropPiece>,
    >,
) {
    let Some(state) = combat.0.as_ref() else {
        return;
    };
    let (Some(art), Some(env)) = (art, state.env()) else {
        return;
    };
    let world = &sim.0.world;
    let layout_h = world.layout_height();
    // The code-drawn rectangles: the footprints, and the stand-in of a missing atlas.
    let drawn: Vec<_> = pieces(&art.encounter, env, world.tick)
        .into_iter()
        .filter(|p| p.sheet().is_none_or(|s| art.sheet(s).is_none()))
        // AREA-RINGS-01: the stone shutter replaces the greybox stand-in.
        .filter(|p| !(p.label == ring_shutter_art::STANDIN_LABEL && ring_shutter.is_some()))
        .collect();
    for (slot, mut transform, mut sprite, mut visibility) in &mut pieces_q {
        match drawn.get(slot.0) {
            Some(p) => {
                let [r, g, b, a] = p.tone.rgba();
                sprite.color = Color::srgba_u8(r, g, b, a);
                sprite.custom_size = Some(Vec2::new(p.rect.w as f32, p.rect.h as f32));
                // Layout px (y down) to world px (y up); the sprite is centred.
                transform.translation.x = p.rect.x as f32 + p.rect.w as f32 / 2.0;
                transform.translation.y =
                    (layout_h - p.rect.bottom()) as f32 + p.rect.h as f32 / 2.0;
                transform.translation.z = layer_z(&art, p.layer);
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    // The kit sprites.
    let list: Vec<_> = sprites(&art.encounter, env, &state.charges, world.tick)
        .into_iter()
        .filter_map(|d| art.sheet(d.sheet).map(|loaded| (d, loaded)))
        .collect();
    for (slot, mut transform, mut sprite, mut visibility) in &mut sprites_q {
        match list.get(slot.0) {
            Some((d, (_, texture, layout))) => {
                if sprite.image != *texture {
                    sprite.image = texture.clone();
                }
                sprite.texture_atlas = Some(TextureAtlas {
                    layout: layout.clone(),
                    index: d.frame,
                });
                sprite.color = Color::srgba_u8(255, 255, 255, d.alpha);
                sprite.custom_size = Some(Vec2::new(d.rect.w as f32, d.rect.h as f32));
                transform.translation.x = d.rect.x as f32 + d.rect.w as f32 / 2.0;
                transform.translation.y =
                    (layout_h - d.rect.bottom()) as f32 + d.rect.h as f32 / 2.0;
                // A distinct z per pool slot: no draw order is left to a tie.
                transform.translation.z = layer_z(&art, d.layer) + slot.0 as f32 * 0.0001;
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

/// One pooled sprite of the stone shutter's presenter (AREA-RINGS-01): the slab
/// (`ring_shutter_closed`, cropped to its bottom `edge_px` rows while it moves) or
/// one of the extras of spec section 6.
#[derive(Component, Clone, Copy)]
pub(super) struct RingShutterSprite(&'static str);

/// The stone shutter's presenter state: the sidecar prop it reads, the atlas
/// rects of its pieces and the presentation-side phase clock.
#[derive(Resource)]
pub(super) struct RingShutterArt {
    /// The prop index of the shutter in the sidecar (`Shutter` of the env step).
    index: usize,
    rects: BTreeMap<&'static str, [i32; 4]>,
    clock: PhaseClock,
}

/// Spawn the slab and the extras of the court's stone shutter, hidden. The slab
/// sits just under the dressing's `ring_shutter_housing` (a prop-layer piece), so
/// the housing draws over its top rows; the dust draws in the foreground layer.
/// Returns whether the presenter is live (the sidecar has a shutter and the
/// contract holds every piece).
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_ring_shutter(
    commands: &mut Commands,
    texture: &Handle<Image>,
    encounter: &Encounter,
    slab_z: f32,
    foreground_z: f32,
    prop_layers: RenderLayers,
    foreground_layers: RenderLayers,
) -> bool {
    let Some(index) = encounter
        .props
        .iter()
        .position(|p| p.kind == dungeon_floorbreak::encounter::PropKind::Shutter)
    else {
        return false;
    };
    let root = std::env::current_dir().unwrap_or_default();
    let rects = match ring_shutter_art::atlas_rects(&root) {
        Ok(rects) => rects,
        Err(e) => {
            println!("RING_SHUTTER not drawn: {e}");
            return false;
        }
    };
    for (n, (piece, r)) in rects.iter().enumerate() {
        let is_dust = piece.starts_with("ring_shutter_dust");
        commands.spawn((
            RingShutterSprite(piece),
            Sprite {
                image: texture.clone(),
                rect: Some(Rect::new(
                    r[0] as f32,
                    r[1] as f32,
                    (r[0] + r[2]) as f32,
                    (r[1] + r[3]) as f32,
                )),
                ..default()
            },
            Anchor::TOP_LEFT,
            // A distinct z per piece: no draw order is left to a tie.
            Transform::from_xyz(
                0.0,
                0.0,
                if is_dust {
                    foreground_z + 0.05 + n as f32 * 0.0001
                } else {
                    slab_z + n as f32 * 0.00001
                },
            ),
            Visibility::Hidden,
            if is_dust {
                foreground_layers.clone()
            } else {
                prop_layers.clone()
            },
        ));
    }
    commands.insert_resource(RingShutterArt {
        index,
        rects,
        clock: PhaseClock::default(),
    });
    println!(
        "RING_SHUTTER stone shutter drawn from the ring atlas (prototype art, owner review pending)"
    );
    true
}

/// Show the slab and the extras of this tick (`ring_shutter_art::shutter_frame`).
/// Presentation only: it reads the shutter through a shared reference and writes
/// only its own sprites and clock.
pub(super) fn sync_ring_shutter(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<ResMut<RingShutterArt>>,
    mut sprites_q: Query<(
        &RingShutterSprite,
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
    )>,
) {
    let (Some(mut art), Some(state)) = (art, combat.0.as_ref()) else {
        return;
    };
    let Some(shutter) = state.env().and_then(|env| env.shutter(art.index)) else {
        return;
    };
    let world = &sim.0.world;
    let layout_h = world.layout_height();
    let ticks = art.clock.observe(shutter.phase(), world.tick);
    let footprint = shutter.footprint();
    let frame =
        ring_shutter_art::shutter_frame(footprint, shutter.phase(), shutter.edge_px(), ticks);
    for (piece, mut transform, mut sprite, mut visibility) in &mut sprites_q {
        let r = art.rects[piece.0];
        // Where the piece draws (layout px, top-left) and which of its atlas rows show.
        let placed = if piece.0 == ring_shutter_art::SLAB {
            (frame.crop_rows > 0).then(|| {
                let rows = frame.crop_rows.min(r[3]);
                (footprint.x, footprint.y, r[1] + r[3] - rows)
            })
        } else {
            frame
                .extras
                .iter()
                .find(|e| e.piece == piece.0)
                .map(|e| (e.x, e.y, r[1]))
        };
        match placed {
            Some((x, y, top_row)) => {
                // The key's bottom `crop_rows` rows, top-anchored at the shutter rect.
                sprite.rect = Some(Rect::new(
                    r[0] as f32,
                    top_row as f32,
                    (r[0] + r[2]) as f32,
                    (r[1] + r[3]) as f32,
                ));
                transform.translation.x = x as f32;
                transform.translation.y = (layout_h - y) as f32;
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}
