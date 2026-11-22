//! CP-Q42 (CP-E carry-over from the CP-D review): the Bevy half of the pierce
//! spark. When the reinforced fist takes a goblin or a llama out of a committed
//! attack, the combat state caches a [`PierceCue`] (`CombatState::pierce_cue`,
//! never hashed, never read by the simulation); this file draws the shared
//! `vfx_hit_spark_metal` atlas (3 frames, 2 ticks each) once, at the contact
//! face of the pierced body. **Presentation only**: one pre-spawned sprite, no
//! write to the simulation, the animators or any resource but its own sprite.
//! The art is `prototype_fixture_unapproved`; nothing here approves it.

use super::*;
use dungeon_floorbreak::blast_art::{SPARK_IMAGE, spark_frame};

/// How far in front of the pierced body's centre (towards the puncher) the
/// spark is centred, px: the contact face of a goblin's 22 px body.
const FACE_OFFSET_PX: f32 = 10.0;
/// z above the actor layer: over a standing goblin (+0.05) and the lit charge
/// (+0.08), under Carl (+0.1).
const SPARK_ABOVE_ACTORS: f32 = 0.08;

#[derive(Component)]
pub(super) struct PierceSpark;

/// Pre-spawn the one spark sprite (hidden); the image joins `handles`, which
/// the loading screen waits on.
pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    handles: &mut Vec<(String, Handle<Image>)>,
    layers: RenderLayers,
    actor_z: f32,
) {
    let image: Handle<Image> = asset_server.load(SPARK_IMAGE);
    handles.push((SPARK_IMAGE.to_owned(), image.clone()));
    commands.spawn((
        PierceSpark,
        Sprite {
            image,
            rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, actor_z + SPARK_ABOVE_ACTORS),
        Visibility::Hidden,
        layers,
    ));
}

/// Draw the spark of the last pierce while its three frames last. Reads the
/// simulation only.
pub(super) fn sync_pierce_spark(
    sim: Res<Sim>,
    combat: Res<Combat>,
    actors: Res<Actors>,
    mut sparks: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<PierceSpark>>,
) {
    let (Some(state), Some(art)) = (combat.0.as_ref(), actors.blast.as_ref()) else {
        return;
    };
    let tick = sim.0.world.tick;
    let frame = state
        .pierce_cue()
        .and_then(|cue| spark_frame(tick.checked_sub(cue.tick)?).map(|frame| (cue, frame)));
    for (mut transform, mut sprite, mut visibility) in &mut sparks {
        match frame {
            Some((cue, frame)) => {
                let [x, y, w, h] = art.spark_rect(frame);
                sprite.rect = Some(Rect::new(
                    x as f32,
                    y as f32,
                    (x + w) as f32,
                    (y + h) as f32,
                ));
                // The face of the body that Carl's fist reached: back towards him.
                let face = cue.x - f32::from(cue.direction) * FACE_OFFSET_PX;
                transform.translation.x = face.round();
                transform.translation.y = cue.y.round();
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}
