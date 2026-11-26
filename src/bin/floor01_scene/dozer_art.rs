//! DR-A: the Bevy half of room 3's presentation (the Dozer revenge). The pure half is
//! `dungeon_floorbreak::dozer_art`; this file only draws what it returns: the approved DOZ-01 rig
//! (hull, tread, riders, front, wheel, yoke), its smoke and steam puffs, the DR-D effects (the
//! metal hit-spark and the ENV-02 splinter burst) and the DOZ-02 art: the damage states, the crew's
//! tumbles and the driver's slump, the rear gate, the buffer stop, the stop frame with its haul
//! line, brace, remnant and rear glance. Every one is an atlas sprite; the machine is never a box.
//! **Presentation only**: the sync system reads the combat state through a shared reference and
//! writes only its own sprites, so nothing here reaches the simulation, the replay or the state
//! hash (the pure half proves it with a state-hash test).

use super::*;
use dungeon_floorbreak::dozer_art::{
    FX_Z, MAX_SPRITES, Sheet as DozerSheet, SpriteDraw, Z_BACK, Z_FRONT, draw,
};
use dungeon_floorbreak::env_kit::ATLAS_ASSET_PREFIX;

/// One pooled rig or puff sprite.
#[derive(Component, Clone, Copy)]
pub(super) struct DozerSprite(usize);

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
    /// The DR-D effects: in front of every actor and the front pieces.
    fx_z: f32,
}

/// The z step between two rig layers (the rig's `z` is 0..=7; the effects, `FX_Z`, draw at
/// `fx_z` instead).
const RIG_Z_STEP: f32 = 0.001;
const SPRITE_POOL: usize = MAX_SPRITES;

fn atlas_present(path: &str) -> bool {
    std::env::current_dir().is_ok_and(|root| root.join(path).is_file())
}

fn asset_path(path: &str) -> String {
    path.strip_prefix(ATLAS_ASSET_PREFIX)
        .unwrap_or(path)
        .to_owned()
}

/// Spawn the pool of room 3. A missing atlas (`bash scripts/export_assets.sh` was not run)
/// skips that sheet with a one-line notice.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    layers: RenderLayers,
    lintel: &[SpriteDraw],
    door: Option<(f32, RenderLayers)>,
    kit: &EnvKit,
    handles: &mut Vec<(String, Handle<Image>)>,
    entities: &mut usize,
) {
    println!(
        "DRA_ART DOZ-01 rig, DOZ-02 damage states, crew, rear gate, buffer stop and stop frame ({} atlases, a pool of {SPRITE_POOL} sprites); no stand-in remains",
        DozerSheet::ALL.len()
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
    let lintel_atlas = sheets
        .iter()
        .find(|a| a.sheet == DozerSheet::Lintel)
        .map(|a| (a.image.clone(), a.layout.clone()));
    commands.insert_resource(DozerArt {
        sheets,
        rig_z,
        back_z: actor_z - 0.5,
        // In front of every actor (Carl is +0.1, a glob +0.2).
        front_z: actor_z + 0.3,
        fx_z: actor_z + 0.305,
    });
    // The tunnel lintel (DZ-Q22), a static piece: the tiles sit just behind the Guild door piece's depth and in its
    // render layer (the structure's lighting), so the door's own cap stays in front of the stone.
    if let Some((image, layout)) = lintel_atlas {
        let (z, lintel_layers) =
            door.map_or((actor_z - 0.5, layers.clone()), |(z, l)| (z - 0.001, l));
        for tile in lintel {
            let (x, y) = tile.centre();
            let (vw, vh) = tile.visible();
            let mut sprite = Sprite::from_atlas_image(
                image.clone(),
                TextureAtlas {
                    layout: layout.clone(),
                    index: usize::from(tile.frame),
                },
            );
            let (w, h) = tile.sheet.frame_size();
            if tile.crop_top > 0 || tile.crop_right > 0 {
                sprite.rect = Some(Rect::new(
                    0.0,
                    f32::from(tile.crop_top),
                    (w - u32::from(tile.crop_right)) as f32,
                    h as f32,
                ));
                sprite.custom_size = Some(Vec2::new(vw as f32, vh as f32));
            }
            commands.spawn((sprite, Transform::from_xyz(x, y, z), lintel_layers.clone()));
            *entities += 1;
        }
    }
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
}

/// The depth of a drawn sprite: rig parts and smoke among the goblins' band, the room pieces behind or in
/// front of every actor, the effects over everything.
fn depth(art: &DozerArt, s: &SpriteDraw, slot: usize) -> f32 {
    let base = match s.z {
        FX_Z => art.fx_z,
        Z_BACK => art.back_z,
        Z_FRONT => art.front_z,
        z => art.rig_z + f32::from(z) * RIG_Z_STEP,
    };
    base + slot as f32 * 0.00001
}

pub(super) fn sync_dozer_art(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<Res<DozerArt>>,
    mut sprites: Query<(&DozerSprite, &mut Transform, &mut Sprite, &mut Visibility)>,
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
                transform.translation = Vec3::new(x, y, depth(&art, s, slot.0));
                sprite.image = a.image.clone();
                sprite.texture_atlas = Some(TextureAtlas {
                    layout: a.layout.clone(),
                    index: usize::from(s.frame),
                });
                // A cropped tile (the gate's last) shows its bottom rows only; a faded one (the
                // warning's floor shadow) tints its alpha.
                let (w, h) = s.sheet.frame_size();
                if s.crop_top > 0 || s.crop_right > 0 {
                    let (vw, vh) = s.visible();
                    sprite.rect = Some(Rect::new(
                        0.0,
                        f32::from(s.crop_top),
                        (w - u32::from(s.crop_right)) as f32,
                        h as f32,
                    ));
                    sprite.custom_size = Some(Vec2::new(vw as f32, vh as f32));
                } else {
                    sprite.rect = None;
                    sprite.custom_size = None;
                }
                // ACT1-B: the greybox wedge darkens the rig (ACT1-Q3: the end-on art is later).
                let tint = if drawn.wedged { 0.42 } else { 1.0 };
                sprite.color = Color::srgba(tint, tint, tint, f32::from(s.alpha) / 255.0);
                *visibility = Visibility::Inherited;
            }
            _ => *visibility = Visibility::Hidden,
        }
    }
}
