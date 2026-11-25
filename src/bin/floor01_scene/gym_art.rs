//! AREA-GYM-01: the Bevy half of the gym's exit door. The two door pieces are
//! ordinary kit pieces placed by the arena dressing; this file marks them at
//! spawn and shows the one `dungeon_floorbreak::gym_art` names for the boss
//! state each frame. **Presentation only**: it reads the combat state and
//! writes only the visibility of its own sprites (the pure half proves the
//! state hash is unchanged).

use super::*;
use dungeon_floorbreak::gym_art::piece_visible;

/// A placed kit piece that belongs to the `gym_exit` state pair.
#[derive(Component)]
pub(super) struct GymStatePiece(pub(super) String);

/// The marker for a placed piece, `None` for every piece outside the pair.
pub(super) fn marker(piece: &str) -> Option<GymStatePiece> {
    dungeon_floorbreak::gym_art::piece_state(piece).map(|_| GymStatePiece(piece.to_owned()))
}

/// Before the first tick the door is closed.
pub(super) fn initial_visibility(piece: &str) -> Visibility {
    if piece_visible(piece, None) {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

pub(super) fn sync_gym_art(
    combat: Res<Combat>,
    mut pieces: Query<(&GymStatePiece, &mut Visibility)>,
) {
    let boss = combat.0.as_ref().and_then(|c| c.boss.as_ref());
    for (piece, mut visibility) in &mut pieces {
        let want = if piece_visible(&piece.0, boss) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
    }
}
