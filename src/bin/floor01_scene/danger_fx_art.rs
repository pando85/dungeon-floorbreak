//! F01 danger FX: the Bevy half of the danger art (the acid pool, the blast reach
//! rings, the blast wave, the room 5 gate and its drop, the slam dust and the acid
//! entry splash). The pure half
//! is `dungeon_floorbreak::danger_fx_art`; this file only draws its list with one pool
//! of sprites, each a source rectangle of its sheet's atlas (the acid is cropped
//! to the pool's live width), and, AREA-HAZARDS-01B, the splash cue
//! (`dungeon_floorbreak::acid_splash`) with its own small pool, so a splash can never
//! push a warning ring out of the shared pool. A missing atlas (`bash scripts/export_assets.sh` was
//! not run) prints one `DANGER_FX missing` line and that visual keeps its
//! code-drawn stand-in (the acid strip, the ring dots, the gate slab) or, for the
//! wave, the VFX-01 burst alone. **Presentation only**: it reads the combat state
//! through a shared reference and writes only its own sprites, so nothing here
//! reaches the simulation, the replay or the state hash (the pure half proves it
//! with a state-hash test).

use super::*;
use dungeon_floorbreak::acid_splash::{AcidSplashes, SLOTS, SPRITES_PER_SPLASH};
use dungeon_floorbreak::danger_fx_art::{
    FxDraw, Layer as FxLayer, MAX_DRAWS, Sheet as FxSheet, acid_drawn, draws, rot_ring_sheet,
};
use dungeon_floorbreak::env_kit::ATLAS_ASSET_PREFIX;
use dungeon_floorbreak::party::HeroId;

/// One pooled danger FX sprite.
#[derive(Component, Clone, Copy)]
pub(super) struct DangerSprite(usize);

/// One pooled acid splash sprite (a splash is a crown and a cropped lip).
#[derive(Component, Clone, Copy)]
pub(super) struct SplashSprite(usize);

/// The splash cue state (render-local; read-only from the event log).
#[derive(Resource, Default)]
pub(super) struct SplashCueRes {
    cues: AcidSplashes,
}

/// z of the splash: a hair above the highest actor sprite (Carl and Donut, `CARL_Z_OFFSET`),
/// under the warning rings. The spec's numeric hint (`blast_z - 0.004`) cannot hold with its
/// own intent: the blast layer lies 0.455 *under* the actors (`BLAST_BELOW_ACTORS` plus the
/// wave's 0.005), so the splash takes the stated intent, above every actor.
const SPLASH_ABOVE_ACTORS: f32 = CARL_Z_OFFSET + 0.005;

/// The atlases that loaded and the z of each layer.
#[derive(Resource)]
pub(super) struct DangerFxArt {
    sheets: Vec<(FxSheet, Handle<Image>)>,
    back_z: f32,
    floor_z: f32,
    dust_z: f32,
    blast_z: f32,
    gate_z: f32,
    splash_z: f32,
    tell_z: f32,
}

impl DangerFxArt {
    pub(super) fn image(&self, sheet: FxSheet) -> Option<&Handle<Image>> {
        self.sheets
            .iter()
            .find(|(s, _)| *s == sheet)
            .map(|(_, image)| image)
    }

    /// Whether `sheet`'s atlas loaded (its stand-in is then not drawn).
    pub(super) fn has(&self, sheet: FxSheet) -> bool {
        self.image(sheet).is_some()
    }

    /// The acid art replaces the code-drawn acid strip and lip.
    pub(super) fn draws_acid(&self) -> bool {
        self.has(FxSheet::Acid) && acid_drawn()
    }

    /// The Rot ring art replaces the code-drawn ring dots (only when the active
    /// radius has a ring canvas).
    pub(super) fn draws_rot_ring(&self) -> bool {
        rot_ring_sheet().is_some_and(|s| self.has(s))
    }

    /// The gate art replaces the code-drawn gate slab (only when the collider is
    /// the 28 px the canvas covers).
    pub(super) fn draws_gate(&self, boss: &dungeon_floorbreak::boss::Boss) -> bool {
        self.has(FxSheet::Gate) && dungeon_floorbreak::danger_fx_art::gate_drawn(boss)
    }
}

fn atlas_present(path: &str) -> bool {
    std::env::current_dir().is_ok_and(|root| root.join(path).is_file())
}

/// Load the sheets and spawn the pool. Called once for every combat scene.
pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    handles: &mut Vec<(String, Handle<Image>)>,
    layers: RenderLayers,
    kit: &EnvKit,
    entities: &mut usize,
) {
    let mut sheets = Vec::new();
    for sheet in FxSheet::ALL {
        let path = sheet.atlas();
        if !atlas_present(path) {
            println!(
                "DANGER_FX missing {path} (run scripts/export_assets.sh): its stand-in draws instead"
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
        "DANGER_FX sheets={}/{} (prototype art, review pending)",
        sheets.len(),
        FxSheet::ALL.len()
    );
    if let Some((_, splash)) = sheets.iter().find(|(s, _)| *s == FxSheet::Splash) {
        for i in 0..SLOTS * SPRITES_PER_SPLASH {
            commands.spawn((
                SplashSprite(i),
                Sprite {
                    image: splash.clone(),
                    rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, kit.actor_z + SPLASH_ABOVE_ACTORS),
                Visibility::Hidden,
                layers.clone(),
            ));
            *entities += 1;
        }
        commands.insert_resource(SplashCueRes::default());
    }
    if let Some((_, texture)) = sheets.first() {
        for i in 0..MAX_DRAWS {
            commands.spawn((
                DangerSprite(i),
                Sprite {
                    image: texture.clone(),
                    rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, kit.actor_z),
                Visibility::Hidden,
                layers.clone(),
            ));
            *entities += 1;
        }
    }
    let actor_z = kit.actor_z;
    let wall_z = kit
        .pieces
        .iter()
        .find(|p| p.layer == "backwall_decal")
        .map_or(3.0, |p| p.z.floor());
    commands.insert_resource(DangerFxArt {
        sheets,
        // The raised gate: behind the actors, with the Boiler landmark's z.
        back_z: actor_z - 0.5,
        // The acid: the hazard pieces' z (under the slime and every actor).
        floor_z: actor_z - 0.045,
        // F01-SIM-TICKS: the slam dust over the props (`actor_z - 0.05`) and the
        // acid, under every actor.
        dust_z: actor_z - 0.03,
        // The wave: on the blast layer just under the VFX-01 core burst.
        blast_z: draw_z(DrawLayer::Blast, 0, actor_z, wall_z) - 0.005,
        // The closed gate: the stand-in slab's z.
        gate_z: actor_z + BOSS_Z_OFFSET,
        splash_z: actor_z + SPLASH_ABOVE_ACTORS,
        // The rings are warnings: above everything, never hidden.
        tell_z: kit.foreground_z + 0.1,
    });
}

fn layer_z(art: &DangerFxArt, layer: FxLayer) -> f32 {
    match layer {
        FxLayer::Back => art.back_z,
        FxLayer::Floor => art.floor_z,
        FxLayer::Dust => art.dust_z,
        FxLayer::Splash => art.splash_z,
        FxLayer::Blast => art.blast_z,
        FxLayer::Gate => art.gate_z,
        FxLayer::Tell => art.tell_z,
    }
}

pub(super) fn sync_danger_fx(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<Res<DangerFxArt>>,
    mut sprites: Query<(&DangerSprite, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (Some(art), Some(state)) = (art, combat.0.as_ref()) else {
        return;
    };
    let world = &sim.0.world;
    let list: Vec<(FxDraw, &Handle<Image>)> = draws(state, world.layout_height(), world.tick)
        .into_iter()
        .filter_map(|d| art.image(d.sheet).map(|image| (d, image)))
        .collect();
    for (slot, mut transform, mut sprite, mut visibility) in &mut sprites {
        match list.get(slot.0) {
            Some((d, image)) => {
                if sprite.image != **image {
                    sprite.image = (*image).clone();
                }
                place(
                    d,
                    layer_z(&art, d.layer) + slot.0 as f32 * 0.0001,
                    &mut transform,
                    &mut sprite,
                );
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

/// Aim one pooled sprite at `d`: the source rectangle (one row of equal cells; the
/// frame's cell, cropped to `src_x..src_x + w` and `src_y..src_y + h`), mirrored when
/// `flip_x`, with the bottom-left at world `(x, y)` (the sprite is centred) and a
/// distinct `z` per pool slot, so no draw order is left to a tie.
fn place(d: &FxDraw, z: f32, transform: &mut Transform, sprite: &mut Sprite) {
    let (cell_w, _) = d.sheet.frame_size();
    let left = (d.frame as i32 * cell_w + d.src_x) as f32;
    sprite.rect = Some(Rect::new(
        left,
        d.src_y as f32,
        left + d.w as f32,
        (d.src_y + d.h) as f32,
    ));
    sprite.flip_x = d.flip_x;
    sprite.custom_size = Some(Vec2::new(d.w as f32, d.h as f32));
    transform.translation.x = d.x as f32 + d.w as f32 / 2.0;
    transform.translation.y = d.y as f32 + d.h as f32 / 2.0;
    transform.translation.z = z;
}

/// AREA-HAZARDS-01B: the acid entry splashes. Reads the event log and the live pool
/// rectangles through shared references and writes only its own sprites.
pub(super) fn sync_acid_splash(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<Res<DangerFxArt>>,
    cues: Option<ResMut<SplashCueRes>>,
    mut sprites: Query<(&SplashSprite, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (Some(art), Some(mut cues), Some(state)) = (art, cues, combat.0.as_ref()) else {
        return;
    };
    let Some(image) = art.image(FxSheet::Splash) else {
        return;
    };
    let world = &sim.0.world;
    let tick = world.tick;
    cues.cues
        .step(tick, state, |hero: HeroId| world.body(hero).x);
    let list: Vec<FxDraw> = state
        .env()
        .map(|env| {
            cues.cues
                .view(tick, env, world.layout_height())
                .iter()
                .flat_map(|d| d.sprites())
                .collect()
        })
        .unwrap_or_default();
    for (slot, mut transform, mut sprite, mut visibility) in &mut sprites {
        match list.get(slot.0) {
            Some(d) => {
                if sprite.image != *image {
                    sprite.image = image.clone();
                }
                place(
                    d,
                    art.splash_z + slot.0 as f32 * 0.0001,
                    &mut transform,
                    &mut sprite,
                );
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}
