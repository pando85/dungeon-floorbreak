//! AREA-TROG-01 stage 1: the dressing of the Floor 01 troglodyte run `trog_run` (TROG-A). Pure and
//! headless, no Bevy types.
//!
//! The room is composed from the Boiler's catacomb kit exactly as before
//! ([`crate::f01_boiler::compose_kit_hooked`]); this module then applies the room's `room_dressing`
//! block of the trog contract with the dressing code of the Broken Ring kit
//! ([`crate::ring_kit::dress_with`]): bays 1 and 4 are bricked up (the slam crack and knuckle
//! marks, the fist-shaped hole), their chains and one doubled torch with its pool are removed, and
//! the wall, floor and prop pieces are placed. Pieces never produce solids, so the dressing is
//! **presentation only**: the DP-Q45 ceiling slab and the sidecar's plain `solid` statics (the low
//! refuge and the bell step) are pushed in by [`lane_solids`], one code path for the dressed and the
//! plain composition, and the tests prove the solids, spawns, checkpoints, exits, views and walking
//! line are identical with and without the dressing.
//!
//! The strength room `trog_strength` (stage 2) adds the bench stack's presenter inputs: the pure
//! piece of each partition state ([`bench_piece`]), the canvas origin ([`bench_origin`]) and the
//! 6-frame break burst ([`debris_frame`], [`break_tick`], [`break_mirror`]), restored by TROGB-AD1
//! (TROG-Q14). Spec: `docs/02-art/area-trog-01-spec.md`.

use std::path::Path;

use crate::encounter::{Loaded, StaticKind};
use crate::env_kit::{EnvKit, IRect as KitRect, load_contract};
use crate::env_prop::PartitionState;
use crate::f01_trog::TrogRoom;
use crate::ring_kit::{Dressing, dress_with, dressing_in};

/// The troglodyte kit contract (extends the gym kit).
pub const TROG_CONTRACT: &str = "assets/source/terrain/floor01/floor01_trog_kit.json";
/// The room id of the run's dressing block.
pub const RUN_ROOM: &str = "trog_run";

/// The atlas image of the bench stack's three states and the break burst (TROGB-AD1).
pub const BENCH_ATLAS: &str = "generated/art/floor01_trog_bench.png";
/// The bench stack's state frames, in [`PartitionState`] order (intact, weakened, broken).
pub const BENCH_PIECES: [&str; 3] = [
    "trog_bench_intact",
    "trog_bench_weakened",
    "trog_bench_broken",
];
/// The break burst's frames: 0..4 fly, 5 is the settled litter (it holds while the bench is broken).
pub const DEBRIS_PIECES: [&str; 6] = [
    "trog_bench_debris_0",
    "trog_bench_debris_1",
    "trog_bench_debris_2",
    "trog_bench_debris_3",
    "trog_bench_debris_4",
    "trog_bench_debris_5",
];
/// The ticks frames 0..4 last (spec section 7: 50, 67, 67, 83 and 100 ms at 60 Hz).
pub const DEBRIS_TICKS: [u64; 5] = [3, 4, 4, 5, 6];
/// The burst canvas's top-left (layout px, y down): 112 x 96, centred on the bench (x 656).
pub const DEBRIS_ORIGIN: (i32, i32) = (600, 484);
/// The bench's centre x, the mirror axis of the burst.
pub const BENCH_CENTRE_X: f32 = 656.0;

/// The state frame piece of the bench stack (presentation only).
pub const fn bench_piece(state: PartitionState) -> &'static str {
    BENCH_PIECES[state as usize]
}

/// The bench canvas's top-left (layout px, y down): the prop's collider top-left.
pub const fn bench_origin(target: crate::camera_lab::room::IRect) -> (i32, i32) {
    (target.x, target.y)
}

/// The bench's three state rectangles and the burst's six, `[x, y, w, h]` in the bench atlas.
pub type BenchRects = ([[i32; 4]; 3], [[i32; 4]; 6]);

/// The atlas rectangles `[x, y, w, h]` of the bench's three state frames and the burst's six frames,
/// from the trog contract.
pub fn bench_atlas_rects(root: &Path) -> Result<BenchRects, String> {
    let contract = load_contract(root, TROG_CONTRACT).map_err(|e| e.to_string())?;
    let find = |id: &str| -> Result<[i32; 4], String> {
        contract
            .pieces
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.rect)
            .ok_or_else(|| format!("the trog contract has no piece {id}"))
    };
    let mut bench = [[0; 4]; 3];
    for (slot, id) in bench.iter_mut().zip(BENCH_PIECES) {
        *slot = find(id)?;
    }
    let mut debris = [[0; 4]; 6];
    for (slot, id) in debris.iter_mut().zip(DEBRIS_PIECES) {
        *slot = find(id)?;
    }
    Ok((bench, debris))
}

/// Which frame of the burst to draw at `tick` for a bench in state `bench` that broke at tick
/// `broke_at` (the break event of the current attempt, [`break_tick`]), and whether it is mirrored
/// about [`BENCH_CENTRE_X`]; `None` while the bench stands (spec section 7).
///
/// A pure function of the event log and the tick, so a presenter that skips frames (a capture run)
/// draws the same frame as one that renders every tick: frames 0..4 last 3, 4, 4, 5 and 6 ticks from
/// the break, frame 5 then holds. A bench broken with no break event in the attempt (authored
/// broken, or a respawn that keeps it broken after the burst ended) shows the settled litter with
/// no burst; a fixture restart clears the event scan and restores the bench, so a rewind never
/// replays a burst.
pub fn debris_frame(
    bench: PartitionState,
    broke_at: Option<u64>,
    tick: u64,
    mirror: bool,
) -> Option<(usize, bool)> {
    if bench != PartitionState::Broken {
        return None;
    }
    let Some(at) = broke_at else {
        return Some((5, false));
    };
    let mut age = tick.saturating_sub(at);
    for (frame, len) in DEBRIS_TICKS.iter().enumerate() {
        if age < *len {
            return Some((frame, mirror));
        }
        age -= len;
    }
    Some((5, mirror))
}

/// Whether the burst is mirrored (TROG-ART-AD9): the frames fly left, away from a Basher at the
/// bench's right face. A break the Basher's `BenchSmash` made keeps them; otherwise the breaker is
/// the living hero nearest to the bench, and a breaker left of it mirrors the burst.
pub fn break_mirror(by_smash: bool, hero_xs: &[f32]) -> bool {
    if by_smash {
        return false;
    }
    hero_xs
        .iter()
        .copied()
        .min_by(|a, b| {
            (a - BENCH_CENTRE_X)
                .abs()
                .total_cmp(&(b - BENCH_CENTRE_X).abs())
        })
        .is_some_and(|x| x < BENCH_CENTRE_X)
}

/// The tick of the latest break of partition `prop` in the current attempt (the scan stops at the
/// latest `Restart`, so an earlier attempt's break is never drawn again).
pub fn break_tick(events: &[crate::combat::CombatEvent], prop: usize) -> Option<u64> {
    use crate::combat::{CombatEvent, PropPhase};
    events
        .iter()
        .rev()
        .take_while(|e| !matches!(e, CombatEvent::Restart { .. }))
        .find_map(|e| match *e {
            CombatEvent::PropPhase {
                tick,
                prop: p,
                phase: PropPhase::Broken,
                ..
            } if usize::from(p) == prop => Some(tick),
            _ => None,
        })
}

/// Whether the break at `at` carries a `BenchSmash` of the Basher on its tick (or the tick before:
/// the shove is offered before the env step).
pub fn break_was_smash(events: &[crate::combat::CombatEvent], at: u64) -> bool {
    use crate::combat::CombatEvent;
    events
        .iter()
        .rev()
        .take_while(|e| !matches!(e, CombatEvent::Restart { .. }))
        .any(|e| {
            matches!(*e, CombatEvent::Trog { tick, act: crate::trog::TrogAct::BenchSmash { .. }, .. }
                if tick <= at && at - tick <= 1)
        })
}

/// The dressing block of the room `room_id` in a parsed contract: `None` when the contract has no
/// block (or a null one) for it, an error when the block is malformed.
pub fn dressing_from(
    contract: &serde_json::Value,
    room_id: &str,
) -> Result<Option<Dressing>, crate::env_kit::EnvError> {
    let block = &contract["room_dressing"][room_id];
    if block.is_null() {
        return Ok(None);
    }
    serde_json::from_value(block.clone())
        .map(Some)
        .map_err(|e| {
            crate::env_kit::EnvError::Parse(format!("{TROG_CONTRACT} room_dressing.{room_id}: {e}"))
        })
}

fn read_contract(root: &Path) -> Result<serde_json::Value, crate::env_kit::EnvError> {
    let path = root.join(TROG_CONTRACT);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| crate::env_kit::EnvError::Io(format!("{}: {e}", path.display())))?;
    serde_json::from_str(&text)
        .map_err(|e| crate::env_kit::EnvError::Parse(format!("{}: {e}", path.display())))
}

/// The dressing block of a troglodyte room (`None`: the room is not dressed yet and launches plain).
pub fn dressing_of(
    root: &Path,
    room: TrogRoom,
) -> Result<Option<Dressing>, crate::env_kit::EnvError> {
    dressing_from(&read_contract(root)?, room.id())
}

/// Whether the room has a dressing block (the binary prints `decor=dressed|plain` from it).
pub fn is_dressed(root: &Path, room: TrogRoom) -> bool {
    dressing_of(root, room).is_ok_and(|d| d.is_some())
}

/// The dressing block of the run.
pub fn run_dressing(root: &Path) -> Result<Dressing, crate::env_kit::EnvError> {
    dressing_in(root, TROG_CONTRACT, RUN_ROOM)
}

/// The room's composition **before** the lane solids: dressed (the trog contract's dressing block,
/// an unmatched `remove_at` entry is an error) or plain (the Boiler's catacomb kit). A room whose
/// contract has no `room_dressing` block composes plain even when `dressed` is asked for: stage 2
/// rooms launch plain until their block lands (TROG-ART-AD19).
pub fn compose(
    root: &Path,
    loaded: &Loaded,
    room: TrogRoom,
    dressed: bool,
) -> Result<EnvKit, String> {
    compose_by_id(root, loaded, room.id(), dressed)
}

/// [`compose`] for a room id.
pub fn compose_by_id(
    root: &Path,
    loaded: &Loaded,
    room_id: &str,
    dressed: bool,
) -> Result<EnvKit, String> {
    let dressing = if dressed {
        dressing_from(&read_contract(root).map_err(|e| e.to_string())?, room_id)
            .map_err(|e| e.to_string())?
    } else {
        None
    };
    let Some(dressing) = dressing else {
        return crate::f01_boiler::compose_kit(root, &loaded.room);
    };
    let contract = load_contract(root, TROG_CONTRACT).map_err(|e| e.to_string())?;
    crate::f01_boiler::compose_kit_hooked(root, &loaded.room, &[], Some(contract), |layout| {
        match dress_with(layout, &dressing, TROG_CONTRACT) {
            0 => Ok(()),
            n => Err(format!("{room_id}: {n} remove_at entries match no piece")),
        }
    })
}

/// The lane solids of a troglodyte room (DP-Q45 / DM-Q27 / KR9): the lane ceiling slab and the
/// sidecar's plain `solid` statics (the low refuge and the bell step) as lane steps. The same code
/// for the dressed and the plain composition.
pub fn lane_solids(kit: &mut EnvKit, loaded: &Loaded, path: &str) -> Result<(), String> {
    let ceiling = loaded
        .ceiling_px
        .ok_or_else(|| format!("{path}: a troglodyte room needs a lane ceiling (DP-Q45)"))?;
    kit.solid_rects.push(KitRect {
        x: 0,
        y: kit.base_walk_line_y - ceiling - 16,
        w: kit.width,
        h: 16,
    });
    for st in loaded
        .encounter
        .statics
        .iter()
        .filter(|st| st.kind == StaticKind::Solid && st.slide.is_none())
    {
        kit.solid_rects.push(KitRect {
            x: st.rect.x,
            y: st.rect.y,
            w: st.rect.w,
            h: st.rect.h,
        });
    }
    Ok(())
}

/// The env kit of a troglodyte room as [`crate::f01_trog::launch`] builds it: the composition (dressed
/// or plain) with the lane solids pushed in. Checks the kit's walking line against the room's floor.
pub fn compose_room_kit(
    root: &Path,
    loaded: &Loaded,
    room: TrogRoom,
    dressed: bool,
) -> Result<EnvKit, String> {
    let mut kit = compose(root, loaded, room, dressed)?;
    if kit.base_walk_line_y != loaded.room.floor_y {
        return Err(format!(
            "the kit's walking line {} is not the room's floor {}",
            kit.base_walk_line_y, loaded.room.floor_y
        ));
    }
    lane_solids(&mut kit, loaded, room.encounter_path())?;
    Ok(kit)
}

#[cfg(test)]
mod tests;
