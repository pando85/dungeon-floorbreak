//! ROOM-PORTALS-01: the Bevy half of the room exit of rooms 2, 4, 4a, 4b and 6 (the
//! pure half is `dungeon_floorbreak::room_exit_art`). In the Boiler, the hazard room,
//! the two Broken Ring rooms and the mastery arena the Guild door the composer places
//! as the exit marker (`guild_portal_closed`) is not spawned (nor the end torch behind
//! it, whose flame tip showed over the cap); one sprite of the `f01-dfx-exit` atlas
//! takes its place, its z and its render layer, and shows the grate down while the
//! room's exit gate stands and raised once it opens. The Guild door's light
//! emitters stay (presentation lighting only, adopted default EXIT-AD4). A missing
//! atlas (`bash scripts/export_assets.sh` was not run) prints one `ROOM_EXIT
//! missing` line and the Guild door stays. **Presentation only**: it reads the
//! combat state through a shared reference and writes only its own sprite, so
//! nothing here reaches the simulation, the replay or the state hash (the pure half
//! proves it with a state-hash test).

use super::*;
use dungeon_floorbreak::camera_lab::room::IRect as RoomRect;
use dungeon_floorbreak::env_kit::ATLAS_ASSET_PREFIX;
use dungeon_floorbreak::env_kit::PieceDraw;
use dungeon_floorbreak::room_exit_art::{
    ATLAS, OPENING_H, OPENING_W, PORTAL_PIECE, exit_draw, hides_piece,
};

/// The exit sprite.
#[derive(Component)]
pub(super) struct ExitSprite;

/// The sidecar exit the sprite is drawn around (layout px, y down).
#[derive(Resource)]
pub(super) struct RoomExitArt {
    exit: RoomRect,
}

/// The sidecar exit of a room whose Guild door this art replaces (rooms 2, 4, 4a,
/// 4b and 6), when its opening is the canvas's and the atlas was exported; `None` keeps
/// the Guild door.
pub(super) fn replaced_exit(args: &Args) -> Option<RoomRect> {
    let exit = if let Some(run) = &args.boiler_run {
        run.loaded.encounter.exit
    } else if let Some(run) = &args.hazards_run {
        run.loaded.encounter.exit
    } else if let Some(run) = &args.mastery_run {
        run.loaded.encounter.exit
    } else if let Some(run) = &args.ring_run {
        run.loaded.encounter.exit
    } else {
        return None;
    };
    if exit.w != OPENING_W || exit.h != OPENING_H {
        return None;
    }
    let present = std::env::current_dir().is_ok_and(|root| root.join(ATLAS).is_file());
    if !present {
        println!("ROOM_EXIT missing {ATLAS} (run scripts/export_assets.sh): the Guild door stays");
        return None;
    }
    Some(exit)
}

/// Whether `piece` is the Guild door the exit art replaces.
pub(super) fn is_replaced(piece: &str) -> bool {
    piece == PORTAL_PIECE
}

/// Whether the doorway around `exit` replaces or covers `piece` (the Guild door, or
/// the end torch whose flame tip would stand on the cap): it is not spawned.
pub(super) fn skips(exit: RoomRect, piece: &PieceDraw) -> bool {
    is_replaced(&piece.piece)
        || hides_piece(
            exit,
            &piece.piece,
            piece.x,
            piece.y,
            piece.rect.w,
            piece.rect.h,
        )
}

/// Load the atlas and spawn the sprite at the replaced door's `z` and render layer.
pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    handles: &mut Vec<(String, Handle<Image>)>,
    layer: RenderLayers,
    z: f32,
    exit: RoomRect,
) {
    let asset = ATLAS.strip_prefix(ATLAS_ASSET_PREFIX).unwrap_or(ATLAS);
    let texture: Handle<Image> = asset_server.load(asset.to_owned());
    handles.push((asset.to_owned(), texture.clone()));
    commands.spawn((
        ExitSprite,
        Sprite {
            image: texture,
            rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, z),
        Visibility::Hidden,
        layer,
    ));
    println!(
        "ROOM_EXIT drawn at x {} (prototype art, review pending)",
        exit.x
    );
    commands.insert_resource(RoomExitArt { exit });
}

pub(super) fn sync_room_exit(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<Res<RoomExitArt>>,
    mut sprites: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<ExitSprite>>,
) {
    let (Some(art), Some(state)) = (art, combat.0.as_ref()) else {
        return;
    };
    let draw = exit_draw(state, art.exit, sim.0.world.layout_height());
    for (mut transform, mut sprite, mut visibility) in &mut sprites {
        let Some(d) = draw else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let left = (d.frame.index() as i32 * d.w) as f32;
        sprite.rect = Some(Rect::new(left, 0.0, left + d.w as f32, d.h as f32));
        sprite.custom_size = Some(Vec2::new(d.w as f32, d.h as f32));
        // World px, y up: the draw gives the bottom-left; the sprite is centred.
        transform.translation.x = d.x as f32 + d.w as f32 / 2.0;
        transform.translation.y = d.y as f32 + d.h as f32 / 2.0;
        *visibility = Visibility::Inherited;
    }
}
