//! PT-C: the per-hero observation and the privileged diagnostics of one
//! state, shared by the live recorder and the verifier so both sides build
//! them with one function (PT-Q14).
//!
//! The policy observation is the AUTOPLAY ObservationV2 builder
//! (`autoplay::party::observe_v2`), called once per hero (seat carl, then seat
//! donut). Its visibility uses the **sim `ScrollCamera`**; the live
//! presentation rig's view rectangle is only a diagnostic field. Nothing here
//! defines a competing observation type.

use crate::autoplay::observation::ObsStats;
use crate::autoplay::party::observe_v2;
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::party::{HeroId, PartyState};

use super::codec::{DiagRecord, encode_obs_bits};

/// The flattened `float32` bits of both heroes' ObservationV2 `[carl, donut]`
/// and what clamping the build counted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObsPair {
    pub bits: [Vec<u32>; 2],
    pub stats: ObsStats,
}

impl ObsPair {
    /// The `[carl, donut]` strings stored in a `TickRecord` or an `EpisodeEnd`.
    pub fn encode(&self) -> [String; 2] {
        [
            encode_obs_bits(&self.bits[0]),
            encode_obs_bits(&self.bits[1]),
        ]
    }
}

/// ObservationV2 of both heroes of the current state. Pure: reads the sim only
/// (`party` supplies the static rules the HUD shows, never hidden AI state).
pub fn observe_pair(scene: &SceneSim, combat: &CombatState, party: &PartyState) -> ObsPair {
    let (carl, carl_stats) = observe_v2(scene, combat, party, HeroId::Carl);
    let (donut, donut_stats) = observe_v2(scene, combat, party, HeroId::Donut);
    let mut stats = carl_stats;
    stats.add(donut_stats);
    ObsPair {
        bits: [carl.to_bits(), donut.to_bits()],
        stats,
    }
}

/// The privileged diagnostics of tick `t` from the state before the step. The
/// presentation rig rectangle (`rig`) is passed in by the live adapter; a
/// replay has no rig and passes `None`.
pub fn diag_of(
    t: u64,
    scene: &SceneSim,
    combat: &CombatState,
    rig: Option<[i32; 4]>,
    stats: ObsStats,
) -> DiagRecord {
    let cam = scene.camera.config();
    let mut enemies = 0_u32;
    let mut hidden = 0_u32;
    let positions = combat
        .enemies
        .iter()
        .filter(|g| g.alive())
        .map(|g| g.x)
        .chain(combat.llamas.iter().filter(|l| l.health > 0).map(|l| l.x));
    for x in positions {
        enemies += 1;
        if scene.is_offscreen(x, 0.0) {
            hidden += 1;
        }
    }
    DiagRecord {
        t,
        rig,
        cam: [
            scene.camera.left(),
            scene.camera.top(),
            cam.view_w,
            cam.view_h,
        ],
        hp: [
            i32::from(scene.world.body(HeroId::Carl).health),
            i32::from(scene.world.body(HeroId::Donut).health),
        ],
        mana: combat
            .donut_mana_view()
            .map(|(mana, max)| [u32::from(mana), u32::from(max)]),
        hurt: [
            u32::from(combat.hurt_ticks_of(HeroId::Carl)),
            u32::from(combat.hurt_ticks_of(HeroId::Donut)),
        ],
        enemies,
        enemies_hidden: hidden,
        obs_clipped: stats.clipped,
        obs_unknown_modes: stats.unknown_modes,
        obs_overflow: stats.overflow,
    }
}
