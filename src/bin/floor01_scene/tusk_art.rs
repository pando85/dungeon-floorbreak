//! BOSB: the Bevy half of the Ball's Tusklings (the pure half is `dungeon_floorbreak::tusk_art`).
//! With the TUSK-01 atlases exported (`bash scripts/export_assets.sh`: `tusk_01` the tux and
//! `tusk_01_gown`), one pooled sprite per budgeted spawn index draws that Tuskling's frame at its
//! simulated pivot. The sprites are **native** (never scaled or rotated) and alternate costume by
//! spawn index (even tux, odd gown). A missing atlas prints one `TUSK_ART not drawn` line and the
//! Tusklings stay invisible bodies (the simulation is unchanged).
//!
//! **Presentation only**: it reads the combat state through a shared reference and writes only the
//! frame, position, mirror and tint of its own sprites, so nothing here reaches the simulation, the
//! replay or the state hash.

use super::*;
use dungeon_floorbreak::combat_scene::tuskling::TuskEnemy;
use dungeon_floorbreak::env_kit::ATLAS_ASSET_PREFIX;
use dungeon_floorbreak::tusk_art as art;
use dungeon_floorbreak::tuskling::State;

/// One pooled sprite for the Tuskling of spawn index `.0`.
#[derive(Component, Clone, Copy)]
pub(super) struct TuskSprite(u8);

/// The presenter is live.
#[derive(Resource)]
pub(super) struct TuskArt {
    /// A living Tuskling draws above the ball and the weight (it walks out of the ball, a kicked
    /// one flies into it) and below Carl; a defeated one lies below every actor.
    z_live: f32,
    z_lying: f32,
}

fn atlas_present(path: &str) -> bool {
    std::env::current_dir().is_ok_and(|root| root.join(path).is_file())
}

/// Spawn one hidden sprite per budgeted Tuskling. Returns whether the presenter is live.
pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    handles: &mut Vec<(String, Handle<Image>)>,
    budget: u8,
    actor_z: f32,
    layers: RenderLayers,
) -> bool {
    let mut sheets: [Option<(Handle<Image>, Handle<TextureAtlasLayout>)>; 2] = [None, None];
    for (slot, index) in [(0usize, 0u8), (1, 1)] {
        let path = art::atlas(index);
        if !atlas_present(path) {
            println!("TUSK_ART not drawn: {path} is missing (run scripts/export_assets.sh)");
            return false;
        }
        let asset = path
            .strip_prefix(ATLAS_ASSET_PREFIX)
            .unwrap_or(path)
            .to_owned();
        let texture: Handle<Image> = asset_server.load(asset.clone());
        handles.push((asset, texture.clone()));
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(art::FRAME.0, art::FRAME.1),
            art::FRAMES,
            1,
            None,
            None,
        ));
        sheets[slot] = Some((texture, layout));
    }
    // Above the ball and the weight's front band, below Carl (+0.1).
    let z = actor_z + BOSS_Z_FRONT + 0.005;
    for index in 0..budget {
        let Some((texture, layout)) = sheets[usize::from(index % 2)].clone() else {
            continue;
        };
        commands.spawn((
            TuskSprite(index),
            Sprite::from_atlas_image(texture, TextureAtlas { layout, index: 0 }),
            Transform::from_xyz(0.0, 0.0, z + f32::from(index) * 0.0001),
            Visibility::Hidden,
            layers.clone(),
        ));
    }
    commands.insert_resource(TuskArt {
        z_live: z,
        z_lying: actor_z + GOBLIN_LYING_Z_OFFSET,
    });
    println!(
        "TUSK_ART drawn: TUSK-01 tux and gown frames, native scale (prototype art, owner review pending)"
    );
    true
}

/// The Tusklings' frames for this tick. Presentation only.
pub(super) fn sync_tusk_art(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art_res: Option<Res<TuskArt>>,
    mut down_since: Local<Vec<Option<u64>>>,
    mut sprites: Query<(&TuskSprite, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (Some(live), Some(state)) = (art_res, combat.0.as_ref()) else {
        return;
    };
    let tick = sim.0.world.tick;
    for (id, mut transform, mut sprite, mut visibility) in &mut sprites {
        let tusk = state
            .slot_enemies()
            .iter()
            .filter_map(|e| e.as_any()?.downcast_ref::<TuskEnemy>())
            .map(TuskEnemy::tusk)
            .find(|t| t.index == id.0);
        let slot = usize::from(id.0);
        if down_since.len() <= slot {
            down_since.resize(slot + 1, None);
        }
        let Some(tusk) = tusk else {
            down_since[slot] = None;
            *visibility = Visibility::Hidden;
            continue;
        };
        if tusk.state == State::Down {
            down_since[slot].get_or_insert(tick);
        } else {
            down_since[slot] = None;
        }
        let age = down_since[slot].map(|s| tick.saturating_sub(s) as u32);
        let frame = art::frame(tusk, age);
        let ((x, floor), flip) = art::placement(tusk);
        let spec = ActorSpec {
            name: "tuskling",
            width: art::FRAME.0 as i32,
            height: art::FRAME.1 as i32,
            pivot_x: art::PIVOT.0 as i32,
            pivot_y: art::PIVOT.1 as i32,
            shadow_width: 0,
            shadow_pivot_x: 0,
        };
        let (cx, cy) = actor_center(spec, x, floor, flip);
        let z = if tusk.state == State::Down {
            live.z_lying
        } else {
            live.z_live
        };
        transform.translation = Vec3::new(cx, cy, z + f32::from(id.0) * 0.0001);
        sprite.flip_x = flip;
        sprite.color = if tusk.state == State::Hurt || tusk.state == State::Kicked {
            FLASH_TINT
        } else {
            Color::WHITE
        };
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = usize::from(frame);
        }
        *visibility = Visibility::Inherited;
    }
}
