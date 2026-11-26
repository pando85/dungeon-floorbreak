//! AREA-FASTFOOD-01: the Bevy half of the fast-food refuge's three state-keyed props.
//!
//! The dressing (`dungeon_floorbreak::fastfood_kit`) places the room's wall, structure and
//! prop pieces through the kit; the staff door, the Light on Your Feet box, its grant sparkle
//! and the service bell are **not** placed. They are presenter sprites whose frame is a pure
//! function of the combat state (`fastfood_kit::{door_frame, box_frame, bell_frame}`):
//!
//! * the door (24 x 228 at (1436, 352)) reads the lock's open state first, then the tick of its
//!   opening event (14 pre-baked atlas frames: closed, 12 rising, open);
//! * the box (32 x 24 at (1204, 556)) is `empty` iff Donut holds `jump_reach_boost`, with the
//!   sparkle (16 x 16) over it for the 12 ticks after the grant;
//! * the bell (8 x 6 at (1380, 442)) rings on the door's open tick.
//!
//! **Presentation only**: it reads the combat state and the scene's tick through shared
//! references and writes only its own sprites, so nothing here reaches the simulation, the
//! replay or the state hash. The door is drawn just under the actors, the box, sparkle and
//! bell on the prop layer under the actors.

use super::*;
use dungeon_floorbreak::fastfood_kit as ff;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(super) enum FastFoodSprite {
    Door,
    Box,
    Sparkle,
    Bell,
}

/// The atlas rect of every presenter frame, read from the contract at spawn.
#[derive(Resource)]
pub(super) struct FastFoodArt {
    rects: ff::PresenterRects,
}

fn atlas_rect(r: [i32; 4]) -> Rect {
    Rect::new(
        r[0] as f32,
        r[1] as f32,
        (r[0] + r[2]) as f32,
        (r[1] + r[3]) as f32,
    )
}

/// Spawn the four presenter sprites (hidden until the first sync). `door` and `pickup` are the
/// two atlas images; `prop_z` is just above the dressing's props and below the actors.
/// Returns whether the presenter is live (a missing contract piece leaves the room undrawn).
pub(super) fn spawn(
    commands: &mut Commands,
    door: &Handle<Image>,
    pickup: &Handle<Image>,
    height: i32,
    door_z: f32,
    prop_z: f32,
    layers: RenderLayers,
) -> bool {
    let root = std::env::current_dir().unwrap_or_default();
    let rects = match ff::presenter_rects(&root) {
        Ok(rects) => rects,
        Err(e) => {
            println!("FASTFOOD_ART not drawn: {e}");
            return false;
        }
    };
    let sprites = [
        (FastFoodSprite::Door, door, ff::DOOR_AT, door_z),
        (FastFoodSprite::Box, pickup, ff::BOX_AT, prop_z),
        (
            FastFoodSprite::Sparkle,
            pickup,
            ff::SPARKLE_AT,
            prop_z + 0.0005,
        ),
        (FastFoodSprite::Bell, pickup, ff::BELL_AT, prop_z),
    ];
    for (kind, texture, at, z) in sprites {
        commands.spawn((
            kind,
            Sprite {
                image: texture.clone(),
                rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(at.0 as f32, to_world_y(height, at.1), z),
            Visibility::Hidden,
            layers.clone(),
        ));
    }
    commands.insert_resource(FastFoodArt { rects });
    println!(
        "FASTFOOD_ART drawn: the staff door (14 frames), the reward box, its sparkle and the bell from the \
         floor01_ff_* atlases (prototype art, owner review pending)"
    );
    true
}

/// Show the frame of each prop's state. Presentation only.
pub(super) fn sync_fastfood_art(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<Res<FastFoodArt>>,
    mut sprites: Query<(&FastFoodSprite, &mut Sprite, &mut Visibility)>,
) {
    let (Some(art), Some(state)) = (art, combat.0.as_ref()) else {
        return;
    };
    let lock_open = state
        .env()
        .and_then(|env| env.surface_locks_open().first().copied())
        .unwrap_or(false);
    let now = sim.0.world.tick;
    let events = state.events.as_slice();
    let open_tick = ff::last_open_tick(events);
    let held = state.story_holds(dungeon_floorbreak::donut_prog::JUMP_REACH_BOOST);
    let door = ff::door_frame(lock_open, open_tick, now);
    // The glint's timer is the scene's tick counter, so a still is a pure function of the run.
    let boxed = ff::box_frame(held, ff::last_grant_tick(events), now, now);
    let bell = ff::bell_frame(lock_open, open_tick, now);
    for (kind, mut sprite, mut visibility) in &mut sprites {
        let rect = match kind {
            FastFoodSprite::Door => Some(art.rects.door[door.index()]),
            FastFoodSprite::Box => Some(art.rects.boxes[boxed.index()]),
            FastFoodSprite::Sparkle => boxed.sparkle().map(|k| art.rects.sparkle[k]),
            FastFoodSprite::Bell => Some(art.rects.bell[bell.index()]),
        };
        match rect {
            Some(r) => {
                sprite.rect = Some(atlas_rect(r));
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}
