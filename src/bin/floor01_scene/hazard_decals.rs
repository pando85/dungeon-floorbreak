//! AREA-HAZARDS-01B: the Bevy half of the hazard decals: the corrosion under every acid
//! patch, the Rot nests and track scuffs on the ceilings and the spent sticker's drip.
//! The pure half is `dungeon_floorbreak::hazard_decals`; this file only draws what it
//! returns, with two small pools of sprites (the static decals and one drip per
//! sticker), each a source rectangle of its sheet's atlas. The static decals are
//! computed once, on the first frame, from the authored specs (they are dressing, not
//! state: HZB-AD8). A missing atlas (`bash scripts/export_assets.sh` was not run) prints
//! one `HAZARD_DECALS missing` line and that sheet is simply not drawn; the spent drip
//! then keeps its code-drawn stand-in (`hazards_art`). **Presentation only**: it reads
//! the combat state through a shared reference and writes only its own sprites, so
//! nothing here reaches the simulation, the replay or the state hash (the pure half
//! proves it with a state-hash test).

use super::*;
use dungeon_floorbreak::env_kit::ATLAS_ASSET_PREFIX;
use dungeon_floorbreak::hazard_decals::{
    Decal, DecalLayer, DecalSheet, drips, static_capacity, static_decals,
};

/// One pooled decal sprite: a static decal or the drip of the `.0`-th spent sticker.
#[derive(Component, Clone, Copy)]
pub(super) enum DecalSprite {
    Static(usize),
    Drip(usize),
}

/// The atlases that loaded and the z of each layer.
#[derive(Resource)]
pub(super) struct HazardDecalArt {
    sheets: Vec<(DecalSheet, Handle<Image>)>,
    /// The floor stains: over the floor solid, just under the acid (`floor_z - 0.002`).
    floor_z: f32,
    /// The backwall trail: the backwall decal layer's z + 0.01.
    wall_z: f32,
    /// The nest and its scuffs: just under the Rot shell (`sticker_z - 0.002`).
    nest_z: f32,
    /// The spent drip: the stand-in's z (`piece_z`).
    drip_z: f32,
}

impl HazardDecalArt {
    fn image(&self, sheet: DecalSheet) -> Option<&Handle<Image>> {
        self.sheets
            .iter()
            .find(|(s, _)| *s == sheet)
            .map(|(_, image)| image)
    }

    /// The drip art replaces the code-drawn drip stand-in (and its label).
    pub(super) fn draws_drip(&self) -> bool {
        self.image(DecalSheet::RotDrip).is_some()
    }

    fn z(&self, layer: DecalLayer) -> f32 {
        match layer {
            DecalLayer::Wall => self.wall_z,
            DecalLayer::Floor => self.floor_z,
            DecalLayer::Nest => self.nest_z,
            DecalLayer::Drip => self.drip_z,
        }
    }
}

fn atlas_present(path: &str) -> bool {
    std::env::current_dir().is_ok_and(|root| root.join(path).is_file())
}

/// Load the sheets and spawn the pools. A room without acid or stickers spawns nothing.
pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    handles: &mut Vec<(String, Handle<Image>)>,
    layers: RenderLayers,
    kit: &EnvKit,
    state: &CombatState,
    entities: &mut usize,
) {
    let Some(env) = state.env() else {
        return;
    };
    let statics = static_capacity(env, state.fixture.name);
    let n_drips = env.sticker_count();
    if statics + n_drips == 0 {
        return;
    }
    let mut sheets = Vec::new();
    for sheet in DecalSheet::ALL {
        let path = sheet.atlas();
        if !atlas_present(path) {
            println!(
                "HAZARD_DECALS missing {path} (run scripts/export_assets.sh): that decal is not drawn"
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
        sheets.push((sheet, texture));
    }
    println!(
        "HAZARD_DECALS sheets={}/{} static={statics} drips={n_drips} (prototype art, review pending)",
        sheets.len(),
        DecalSheet::ALL.len()
    );
    let Some((_, texture)) = sheets.first().cloned() else {
        return;
    };
    let actor_z = kit.actor_z;
    let mut pool = |kind: DecalSprite| {
        commands.spawn((
            kind,
            Sprite {
                image: texture.clone(),
                rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, actor_z),
            Visibility::Hidden,
            layers.clone(),
        ));
        *entities += 1;
    };
    for i in 0..statics {
        pool(DecalSprite::Static(i));
    }
    for i in 0..n_drips {
        pool(DecalSprite::Drip(i));
    }
    let wall_z = kit
        .pieces
        .iter()
        .find(|p| p.layer == "backwall_decal")
        .map_or(3.0, |p| p.z.floor());
    commands.insert_resource(HazardDecalArt {
        sheets,
        // The acid's z is `actor_z - 0.045`; the stain lies a hair under it.
        floor_z: actor_z - 0.045 - 0.002,
        wall_z: wall_z + 0.01,
        // The Rot shell is `actor_z - 0.04`.
        nest_z: actor_z - 0.04 - 0.002,
        // The drip stand-in's z (`HazardArt::piece_z`).
        drip_z: actor_z - 0.045,
    });
}

fn place(d: &Decal, z: f32, transform: &mut Transform, sprite: &mut Sprite) {
    let (cell_w, _) = d.sheet.frame_size();
    let left = (d.frame as i32 * cell_w + d.src_x) as f32;
    sprite.rect = Some(Rect::new(left, 0.0, left + d.w as f32, d.h as f32));
    sprite.flip_x = d.flip_x;
    sprite.custom_size = Some(Vec2::new(d.w as f32, d.h as f32));
    // World px, y up: the decal gives the bottom-left; the sprite is centred.
    transform.translation.x = d.x as f32 + d.w as f32 / 2.0;
    transform.translation.y = d.y as f32 + d.h as f32 / 2.0;
    transform.translation.z = z;
}

pub(super) fn sync_hazard_decals(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<Res<HazardDecalArt>>,
    mut statics: Local<Option<Vec<Decal>>>,
    mut sprites: Query<(&DecalSprite, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (Some(art), Some(state)) = (art, combat.0.as_ref()) else {
        return;
    };
    let Some(env) = state.env() else {
        return;
    };
    let world = &sim.0.world;
    let list = statics
        .get_or_insert_with(|| static_decals(env, world.layout_height(), state.fixture.name));
    let spent = drips(state, world.tick);
    for (kind, mut transform, mut sprite, mut visibility) in &mut sprites {
        let draw = match *kind {
            DecalSprite::Static(i) => list.get(i),
            DecalSprite::Drip(i) => spent.get(i),
        };
        match draw.and_then(|d| art.image(d.sheet).map(|image| (d, image))) {
            Some((d, image)) => {
                if sprite.image != *image {
                    sprite.image = image.clone();
                }
                let slot = match *kind {
                    DecalSprite::Static(i) | DecalSprite::Drip(i) => i,
                };
                place(
                    d,
                    art.z(d.layer) + slot as f32 * 0.0001,
                    &mut transform,
                    &mut sprite,
                );
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}
