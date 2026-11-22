//! BOIL-E (K10): the Bevy half of the Boiler's prop stand-ins. The pure half is
//! `dungeon_floorbreak::env_prop_art::pieces`; this file only draws what it
//! returns with a pool of code-drawn rectangles. **Presentation only and no new
//! art**: it reads the combat state through a shared reference and writes only its
//! own sprites, so nothing here can reach the simulation, the replay or the hash.
//! The room's props are labelled stand-ins until ART-BOIL lands.

use super::*;
use dungeon_floorbreak::encounter::Encounter;
use dungeon_floorbreak::env_prop_art::{Layer, MAX_PIECES, pieces};

#[derive(Component, Clone, Copy)]
pub(super) struct PropPiece(usize);

/// The room's sidecar, for the authored prop rectangles and the z of each layer.
#[derive(Resource)]
pub(super) struct PropArt {
    encounter: Encounter,
    back_z: f32,
    prop_z: f32,
    tell_z: f32,
}

pub(super) fn spawn(
    commands: &mut Commands,
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
    commands.insert_resource(PropArt {
        encounter,
        // Behind the actors; with the lane props (the drum is a solid, so no
        // actor stands in it); above the foreground so a warning is never hidden.
        back_z: actor_z - 0.5,
        prop_z: actor_z - 0.05,
        tell_z: foreground_z + 0.1,
    });
}

pub(super) fn sync_prop_art(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<Res<PropArt>>,
    mut sprites: Query<(&PropPiece, &mut Transform, &mut Sprite, &mut Visibility)>,
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
}
