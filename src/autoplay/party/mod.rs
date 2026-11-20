//! AUTOPLAY-02a solo-party env (stages 02a-A and 02a-B).
//!
//! `party_solo` runs the game's real party path ([`crate::party_scene::PartyFixtureSim`])
//! with the policy on **one** seat (Carl or Donut) and the game's own
//! uncontrolled-hero AI on the other (AP-Q22). Nothing here owns a gameplay
//! rule: the env turns an [`super::ActionV1`] into slot 0 of the party input,
//! steps the same tick order as the interactive scene and decides the episode
//! status (AP-Q29).
//!
//! - [`scenario`]: the `ap2a_*` roster, the companion registry, the pinned
//!   initial hashes and the `PartyTuning` encoding hash (AP-Q24, AP-Q25).
//! - [`observation`]: ObservationV2 and its builder, which takes
//!   `(&SceneSim, &CombatState, &PartyState)` plus the hero so the live
//!   recorder (PLAYTRACE PT-C, PT-Q14) shares the one function (AP-Q27, AP-Q28).
//! - [`env`]: [`PartyAutoplayEnv`] with seat entry, swap stripping and the
//!   party terminal rules (AP-Q23, AP-Q29).
//!
//! Not in these stages: party metrics, the `combat-party-v1` reward, the
//! `autoplay-episode/2` replay and the eval CLI (02a-C); the `cf02_assist` and
//! `companion_v0_*` companions (02a-D, after COMPANION 01A-wire).

pub mod env;
pub mod observation;
pub mod scenario;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_obs;

pub use env::{PartyAutoplayEnv, PartyStepOutcome};
pub use observation::{
    ALLY_LEN, OBS_V2, OBSERVATION_VERSION_V2, ObservationV2, SELF_V2_LEN, observe_v2,
};
pub use scenario::{PartyRosterEntry, party_tuning_hash};

use crate::party::HeroId;

/// Placeholder `reward_id` of a `party_solo` config until the
/// `combat-party-v1` profile lands with 02a-C (AP-Q32). A v1 reward id is
/// rejected in `party_solo` and this marker is rejected by the `solo` env.
pub const PARTY_REWARD_PENDING: &str = "pending-02a-c";

/// Which env a config selects (AP-Q22).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AutoplayMode {
    /// The v1 Carl-only `FixtureSim` env (the default).
    #[default]
    Solo,
    /// The policy drives `seat`; the game's own AI drives the other hero.
    PartySolo {
        seat: HeroId,
        companion_profile: String,
    },
}

impl AutoplayMode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Solo => "solo",
            Self::PartySolo { .. } => "party_solo",
        }
    }

    pub fn is_party(&self) -> bool {
        matches!(self, Self::PartySolo { .. })
    }

    /// The observation schema the mode uses: 1 for `solo`, 2 for `party_solo`.
    pub fn obs_version(&self) -> u32 {
        match self {
            Self::Solo => 1,
            Self::PartySolo { .. } => 2,
        }
    }

    pub fn seat(&self) -> Option<HeroId> {
        match self {
            Self::Solo => None,
            Self::PartySolo { seat, .. } => Some(*seat),
        }
    }
}

/// The seat's serialized name (`carl` or `donut`); its numeric value is
/// [`HeroId::index`], equal to the AP-Q14 instigator id.
pub fn seat_name(seat: HeroId) -> &'static str {
    match seat {
        HeroId::Carl => "carl",
        HeroId::Donut => "donut",
    }
}

pub fn parse_seat(name: &str) -> Option<HeroId> {
    HeroId::ALL.into_iter().find(|h| seat_name(*h) == name)
}
