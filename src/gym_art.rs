//! AREA-GYM-01: the state pair of the gym's exit door (the pure half). The
//! door is drawn `closed` while the boss lives and `open` once the right wall
//! is released (`Boss::released()`). The Bevy half only toggles the visibility
//! of the two placed kit pieces; nothing here writes the simulation, so it can
//! never reach the replay or the state hash (test below). Spec:
//! `docs/02-art/area-gym-01-spec.md` section 5, GYM-AD2 / GYM-AD7.

use crate::boss::Boss;

/// The two states of the `gym_exit` state group of the gym kit contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitState {
    Closed,
    Open,
}

/// The state a placed kit piece shows, `None` for every other piece.
pub fn piece_state(piece: &str) -> Option<ExitState> {
    match piece {
        "gym_exit_closed" => Some(ExitState::Closed),
        "gym_exit_open" => Some(ExitState::Open),
        _ => None,
    }
}

/// The state the door shows now: open exactly when the right wall is gone.
/// Without a boss (a scene that is not the arena) the door stays closed.
pub fn exit_state(boss: Option<&Boss>) -> ExitState {
    if boss.is_some_and(Boss::released) {
        ExitState::Open
    } else {
        ExitState::Closed
    }
}

/// Whether the placed piece `piece` is drawn this tick (`true` for every piece
/// that is not part of the state pair).
pub fn piece_visible(piece: &str, boss: Option<&Boss>) -> bool {
    piece_state(piece).is_none_or(|s| s == exit_state(boss))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss_scene::{self, BOSS01_FULL_CLEAR_SOLO, BOSS01_PARTY_DEFEAT_RETRY};
    use crate::party_scene::PartyFixtureSim;
    use std::path::Path;

    #[test]
    fn exactly_one_door_state_is_visible_without_a_boss() {
        assert_eq!(exit_state(None), ExitState::Closed);
        assert!(piece_visible("gym_exit_closed", None));
        assert!(!piece_visible("gym_exit_open", None));
        assert!(piece_visible("gym_mirror_bay", None));
        assert_eq!(piece_state("torch_wall"), None);
    }

    /// The dressed arena (gym pieces) and the plain catacomb composition run
    /// the same fixture to the same state hash on every tick, with the
    /// presenter reading the boss on every tick of the dressed run; the door
    /// is closed for the whole fight and opens on the tick the wall is
    /// released.
    #[test]
    fn the_dressing_and_the_presenter_do_not_change_the_state_hash() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let dressed = boss_scene::load_arena(root).unwrap();
        let plain = boss_scene::load_arena_undressed(root).unwrap();
        for fixture in [&BOSS01_FULL_CLEAR_SOLO, &BOSS01_PARTY_DEFEAT_RETRY] {
            let mut a = PartyFixtureSim::new(&dressed, 360, fixture).unwrap();
            let mut b = PartyFixtureSim::new(&plain, 360, fixture).unwrap();
            let mut opened = None;
            for tick in 0..fixture.base.ticks {
                a.step_scripted();
                b.step_scripted();
                let boss = a.combat.boss.as_ref();
                let open = piece_visible("gym_exit_open", boss);
                assert_ne!(open, piece_visible("gym_exit_closed", boss));
                assert_eq!(open, boss.is_some_and(Boss::released));
                if open && opened.is_none() {
                    opened = Some(tick);
                }
                assert_eq!(
                    a.state_hash(),
                    b.state_hash(),
                    "{} tick {tick}",
                    fixture.name()
                );
            }
            if fixture.name() == "boss01_full_clear_solo" {
                assert!(opened.is_some(), "the solo clear releases the wall");
            }
        }
    }
}
