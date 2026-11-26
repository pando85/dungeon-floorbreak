//! LLAMA-03 (CRL-Q39): which atlas frame the Bad Llama draws during a corral rush.
//!
//! A pure function of the rush state (`phase`, `tick`, `active`, `left`) and the two profile lengths, read
//! from the same data as the stand-in's settle age. The art never feeds back into the simulation: nothing
//! here mutates a state, draws a random number or is hashed or replayed. The tick map is
//! `docs/02-art/llama-03-rush-spec.md` section 3 and the `rush_poses` block of
//! `assets/manifests/llama_rush_h64.json` (a test keeps the two equal).
//!
//! * **Tell** (`tick` 0..32): four-tick slots `brace_in`, `brace`, `scrape`, `brace`, `scrape`, `brace`, `scrape`,
//!   `coil`, so the scrape beats are the stand-in's `tick >= 4 && tick % 8 < 4`.
//! * **Committed** (`active` from 1 on the commit tick): the run cycle `[push, reach, contact, gather]`, three
//!   ticks a frame.
//! * **Recover** (`left` counts down from the profile's recover ticks): `recover_skid` for ten ticks, then
//!   `recover_blown`.
//! * **Bonk** (`left` counts down from the profile's bonk ticks): the LLAMA-01 `hurt` frame for twelve ticks (the
//!   familiar hit read, LL3-AD1), then `recover_blown`.

use super::rush_stand_in::corral_ticks;
use crate::llama::{Llama, LlamaMode};
use crate::llama_rush::RushPhase;

/// Number of frames of the rush atlas (`llama_rush_h64`).
pub const RUSH_FRAMES: usize = 10;
/// The tell is eight slots of this many ticks.
pub const TELL_SLOT_TICKS: u16 = 4;
/// The tell slots, as rush-atlas frames: `brace_in`, `brace`, `scrape`, `brace`, `scrape`, `brace`, `scrape`, `coil`.
pub const TELL_SLOTS: [usize; 8] = [0, 1, 2, 1, 2, 1, 2, 3];
/// The committed run cycle: `run_push`, `run_reach`, `run_contact`, `run_gather`.
pub const RUN_CYCLE: [usize; 4] = [6, 7, 4, 5];
/// Committed ticks a run frame is shown.
pub const RUN_TICKS_PER_FRAME: u16 = 3;
/// `recover_skid` (the stop) shows for this many recover ticks.
pub const SKID_TICKS: u16 = 10;
/// The bonk shows the LLAMA-01 hurt frame for this many ticks.
pub const BONK_HURT_TICKS: u16 = 12;
/// The rush-atlas frames of the stop.
pub const RECOVER_SKID_FRAME: usize = 8;
pub const RECOVER_BLOWN_FRAME: usize = 9;
/// The LLAMA-01 `hurt` frame (atlas index 11).
pub const BASE_HURT_FRAME: usize = 11;

/// The frame a rushing llama draws, and the atlas it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RushFrame {
    /// A frame of the rush atlas (`llama_rush_h64`).
    Rush(usize),
    /// A frame of the LLAMA-01 atlas.
    Base(usize),
}

/// The frame of one rush state. `recover_ticks` and `bonk_ticks` are the profile's lengths of those phases.
pub fn rush_frame(
    phase: RushPhase,
    tick: u16,
    active: u16,
    left: u16,
    recover_ticks: u16,
    bonk_ticks: u16,
) -> RushFrame {
    match phase {
        RushPhase::Tell => {
            let slot = usize::from(tick / TELL_SLOT_TICKS).min(TELL_SLOTS.len() - 1);
            RushFrame::Rush(TELL_SLOTS[slot])
        }
        RushPhase::Committed => {
            let step = usize::from(active / RUN_TICKS_PER_FRAME) % RUN_CYCLE.len();
            RushFrame::Rush(RUN_CYCLE[step])
        }
        RushPhase::Recover => {
            let elapsed = recover_ticks.saturating_sub(left);
            RushFrame::Rush(if elapsed < SKID_TICKS {
                RECOVER_SKID_FRAME
            } else {
                RECOVER_BLOWN_FRAME
            })
        }
        RushPhase::Bonk => {
            let elapsed = bonk_ticks.saturating_sub(left);
            if elapsed < BONK_HURT_TICKS {
                RushFrame::Base(BASE_HURT_FRAME)
            } else {
                RushFrame::Rush(RECOVER_BLOWN_FRAME)
            }
        }
    }
}

/// The frame of `llama`, or `None` unless it is in a corral rush (the same gate as
/// [`super::rush_stand_in::rush_stand_in`]: an `Attack` with a rush state).
pub fn llama_rush_frame(llama: &Llama) -> Option<RushFrame> {
    if llama.mode != LlamaMode::Attack {
        return None;
    }
    let rush = llama.rush_state()?;
    let (recover_ticks, bonk_ticks) = corral_ticks(llama);
    Some(rush_frame(
        rush.phase,
        rush.tick,
        rush.active,
        rush.left,
        recover_ticks,
        bonk_ticks,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use RushFrame::{Base, Rush};

    const RECOVER: u16 = 40;
    const BONK: u16 = 60;

    fn tell(tick: u16) -> RushFrame {
        rush_frame(RushPhase::Tell, tick, 0, 0, RECOVER, BONK)
    }

    fn run(active: u16) -> RushFrame {
        rush_frame(RushPhase::Committed, 0, active, 0, RECOVER, BONK)
    }

    fn recover(elapsed: u16) -> RushFrame {
        rush_frame(RushPhase::Recover, 0, 0, RECOVER - elapsed, RECOVER, BONK)
    }

    fn bonk(elapsed: u16) -> RushFrame {
        rush_frame(RushPhase::Bonk, 0, 0, BONK - elapsed, RECOVER, BONK)
    }

    #[test]
    fn the_tell_walks_the_eight_slots() {
        for (tick, want) in [
            (0, 0),
            (3, 0),
            (4, 1),
            (8, 2),
            (11, 2),
            (12, 1),
            (27, 2),
            (28, 3),
            (31, 3),
        ] {
            assert_eq!(tell(tick), Rush(want), "tell tick {tick}");
        }
        // Past the tell length the last slot holds.
        assert_eq!(tell(200), Rush(3));
    }

    /// The scrape beats are the stand-in's: ticks 8-11, 16-19 and 24-27.
    #[test]
    fn the_scrape_frame_lands_on_the_stand_ins_scrape_beats() {
        for tick in 0..32u16 {
            let scrape = tick >= 4 && tick % 8 < 4;
            assert_eq!(tell(tick) == Rush(2), scrape, "tell tick {tick}");
        }
    }

    /// The commit tick already has `active` 1, so `active` 0 never occurs in the committed phase; it is still
    /// defined (the push).
    #[test]
    fn the_run_cycles_push_reach_contact_gather() {
        for (active, want) in [
            (0, 6),
            (1, 6),
            (2, 6),
            (3, 7),
            (6, 4),
            (9, 5),
            (12, 6),
            (17, 7),
            (18, 4),
            (23, 5),
            (24, 6),
        ] {
            assert_eq!(run(active), Rush(want), "active {active}");
        }
    }

    #[test]
    fn the_recover_skids_then_stands_blown() {
        for (elapsed, want) in [(0, 8), (9, 8), (10, 9), (39, 9)] {
            assert_eq!(recover(elapsed), Rush(want), "recover elapsed {elapsed}");
        }
        // `left` above the profile length (a data change) never underflows.
        assert_eq!(
            rush_frame(RushPhase::Recover, 0, 0, 50, RECOVER, BONK),
            Rush(8)
        );
    }

    #[test]
    fn the_bonk_keeps_the_llama01_hurt_frame_then_stands_blown() {
        for (elapsed, want) in [(0, Base(11)), (11, Base(11)), (12, Rush(9)), (59, Rush(9))] {
            assert_eq!(bonk(elapsed), want, "bonk elapsed {elapsed}");
        }
    }

    #[test]
    fn every_rush_frame_is_inside_the_atlas() {
        for tick in 0..40 {
            for frame in [tell(tick), run(tick), recover(tick), bonk(tick)] {
                if let Rush(i) = frame {
                    assert!(i < RUSH_FRAMES, "{frame:?}");
                }
            }
        }
    }

    fn root() -> &'static std::path::Path {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    /// B4: the rush clips load from the grid source (the path the scene's default-on art takes) with all
    /// ten frames, the LLAMA-01 frame size and pivot and the three tags.
    #[test]
    fn the_rush_clips_load_from_the_grid_with_all_ten_frames() {
        use crate::actor_sprite::FrameSource;
        use crate::combat_anim::SpriteClips;
        let rush =
            SpriteClips::load_llama_rush(root(), FrameSource::GridSource).expect("rush clips");
        let base =
            SpriteClips::load_llama(root(), FrameSource::GridSource).expect("LLAMA-01 clips");
        assert_eq!(rush.frame_count, RUSH_FRAMES);
        assert_eq!(rush.spec.width, base.spec.width);
        assert_eq!(rush.spec.height, base.spec.height);
        assert_eq!(rush.spec.pivot_x, base.spec.pivot_x);
        assert_eq!(rush.spec.pivot_y, base.spec.pivot_y);
        assert_eq!(rush.spec.shadow_width, base.spec.shadow_width);
        assert_eq!(rush.spec.shadow_pivot_x, base.spec.shadow_pivot_x);
        assert_eq!(rush.tag("rush_brace"), (0, 4));
        assert_eq!(rush.tag("rush_run"), (4, 4));
        assert_eq!(rush.tag("rush_recover"), (8, 2));
        assert_eq!(rush.atlas_asset, "llama_rush_h64.png");
    }

    /// The manifest's `rush_poses` tick map is the pose map's (the data and the code never drift).
    #[test]
    fn the_manifest_tick_map_is_the_pose_map() {
        let text =
            std::fs::read_to_string(root().join("assets/manifests/llama_rush_h64.json")).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&text).unwrap();
        let poses = &manifest["rush_poses"];
        let list = |v: &serde_json::Value| -> Vec<usize> {
            v.as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_u64().unwrap() as usize)
                .collect()
        };
        assert_eq!(list(&poses["tell"]["slots"]), TELL_SLOTS.to_vec());
        assert_eq!(
            poses["tell"]["slot_ticks"].as_u64(),
            Some(u64::from(TELL_SLOT_TICKS))
        );
        assert_eq!(list(&poses["committed"]["cycle"]), RUN_CYCLE.to_vec());
        assert_eq!(
            poses["committed"]["ticks_per_frame"].as_u64(),
            Some(u64::from(RUN_TICKS_PER_FRAME))
        );
        assert_eq!(
            poses["recover"],
            serde_json::json!([[0, RECOVER_SKID_FRAME], [SKID_TICKS, RECOVER_BLOWN_FRAME]])
        );
        assert_eq!(
            poses["bonk"],
            serde_json::json!([
                [0, format!("llama-01:{BASE_HURT_FRAME}")],
                [BONK_HURT_TICKS, RECOVER_BLOWN_FRAME]
            ])
        );
        assert_eq!(list(&poses["key_frames"]), vec![1, 2, 4, 8]);
    }
}
