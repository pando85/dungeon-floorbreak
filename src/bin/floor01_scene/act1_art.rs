//! AREA-ACT1-01: the Bevy half of the three act-1 rooms' state-keyed pieces. The dressing
//! (`dungeon_floorbreak::act1_kit`) places the rooms' wall, structure and prop pieces through the
//! kit; the timber sets, the chase barricade, the guild's grant box and the guild portal's open
//! overlay are **not** placed. They are presenter sprites whose piece is a pure function of the
//! combat state (`act1_kit::{timber_piece, barricade_piece, box_piece}`):
//!
//! * a timber set (80 x 104 at (1104, 456) and (1424, 456)) is `timber_set_smashed` once the
//!   machine's front smashed it (`Dozer::timber_smashed`), and stays so;
//! * the barricade (96 x 280 at (2304, 304)) is `timber_barricade_full_smashed` while
//!   `Dozer::barrier_hit()` (rewind- and respawn-safe: the break stays through the defeat delay
//!   and the respawn closes the lock again);
//! * the grant box (24 x 28 at (340, 552)) is `act1_grant_box_open` once the guild door's lock is
//!   open (`donut_unlocked`) or a hero's Interact verb opened it (GUILD-VERB-01), and the composer's portal gets the `guild_portal_open` overlay.
//!
//! The shore's wedged machine is a static pose (`dozer_art::spawn_wedge`).
//! **Presentation only**: it reads the combat state through a shared reference and writes only the
//! rect and the visibility of its own sprites, so nothing here reaches the simulation, the replay
//! or the state hash.

use super::*;
use dungeon_floorbreak::act1_kit as kit_art;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(super) enum Act1Sprite {
    Timber(usize),
    Barricade,
    Box,
    PortalOpen,
}

/// The atlas rect of every presenter piece, read from the contract at spawn.
#[derive(Resource)]
pub(super) struct Act1Art {
    rects: kit_art::PresenterRects,
}

fn atlas_rect(r: [i32; 4]) -> Rect {
    Rect::new(
        r[0] as f32,
        r[1] as f32,
        (r[0] + r[2]) as f32,
        (r[1] + r[3]) as f32,
    )
}

fn atlas_of(handles: &[(String, Handle<Image>)], name: &str) -> Option<Handle<Image>> {
    handles
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, h)| h.clone())
}

/// Spawn the presenter sprites of the room (the chase's timbers and barricade, the guild's box and
/// portal overlay; the shore has none). `prop_z` is just above the dressing's props and below the
/// actors; `portal_z` is the composer's closed portal piece's z. Returns whether the presenter is
/// live (a missing contract piece or atlas leaves the room's pieces undrawn).
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn(
    commands: &mut Commands,
    handles: &[(String, Handle<Image>)],
    room: f01_act1::Act1Room,
    height: i32,
    prop_z: f32,
    portal_z: Option<f32>,
    layers: RenderLayers,
) -> bool {
    let root = std::env::current_dir().unwrap_or_default();
    let rects = match kit_art::presenter_rects(&root) {
        Ok(rects) => rects,
        Err(e) => {
            println!("ACT1_ART not drawn: {e}");
            return false;
        }
    };
    let mut spawn_one =
        |kind: Act1Sprite, atlas: &str, rect: [i32; 4], at: (i32, i32), z: f32, visible: bool| {
            let Some(image) = atlas_of(handles, atlas) else {
                println!("ACT1_ART missing atlas {atlas} (run scripts/export_assets.sh)");
                return;
            };
            commands.spawn((
                kind,
                Sprite {
                    image,
                    rect: Some(atlas_rect(rect)),
                    ..default()
                },
                Anchor::TOP_LEFT,
                Transform::from_xyz(at.0 as f32, to_world_y(height, at.1), z),
                if visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                },
                layers.clone(),
            ));
        };
    match room {
        f01_act1::Act1Room::Chase => {
            for (i, at) in kit_art::TIMBER_AT.into_iter().enumerate() {
                let (atlas, rect) = &rects.timber[0];
                spawn_one(Act1Sprite::Timber(i), atlas, *rect, at, prop_z, true);
            }
            let (atlas, rect) = &rects.barricade[0];
            spawn_one(
                Act1Sprite::Barricade,
                atlas,
                *rect,
                kit_art::BARRICADE_AT,
                prop_z,
                true,
            );
        }
        f01_act1::Act1Room::Guild => {
            let (atlas, rect) = &rects.boxes[0];
            spawn_one(Act1Sprite::Box, atlas, *rect, kit_art::BOX_AT, prop_z, true);
            if let Some(z) = portal_z {
                let (atlas, rect) = &rects.portal_open;
                spawn_one(
                    Act1Sprite::PortalOpen,
                    atlas,
                    *rect,
                    kit_art::PORTAL_AT,
                    z + 0.0005,
                    false,
                );
            }
        }
        f01_act1::Act1Room::Shore => {}
    }
    commands.insert_resource(Act1Art { rects });
    println!(
        "ACT1_ART drawn: the timbers, the barricade, the grant box and the portal overlay from the \
         floor01_act1_* / opening atlases (prototype art, owner review pending)"
    );
    true
}

/// Show the piece of each sprite's state. Presentation only.
pub(super) fn sync_act1_art(
    combat: Res<Combat>,
    art: Option<Res<Act1Art>>,
    mut sprites: Query<(&Act1Sprite, &mut Sprite, &mut Visibility)>,
) {
    let (Some(art), Some(state)) = (art, combat.0.as_ref()) else {
        return;
    };
    let machine = dungeon_floorbreak::dozer_art::dozer_of(state);
    let door_open = state.donut_unlocked();
    // GUILD-VERB-01: a hero's Interact verb opens the strongbox too (the grant stays the zone's).
    let box_open = door_open || state.interact_box_open();
    for (kind, mut sprite, mut visibility) in &mut sprites {
        let rect = match kind {
            Act1Sprite::Timber(i) => machine.map(|d| {
                let smashed = d.timber_smashed(*i);
                art.rects.timber[usize::from(smashed)].1
            }),
            Act1Sprite::Barricade => machine.map(|d| {
                let hit = d.barrier_hit();
                art.rects.barricade[usize::from(hit)].1
            }),
            Act1Sprite::Box => Some(art.rects.boxes[usize::from(box_open)].1),
            Act1Sprite::PortalOpen => Some(art.rects.portal_open.1),
        };
        if let Some(r) = rect {
            sprite.rect = Some(atlas_rect(r));
        }
        if *kind == Act1Sprite::PortalOpen {
            let want = if kit_art::portal_open(door_open) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *visibility != want {
                *visibility = want;
            }
        }
    }
}
