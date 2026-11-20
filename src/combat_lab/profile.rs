//! The test-only capability profile `floor01-postbiscuit-lab-v1` (LAB-Q01).
//!
//! Pinned at "Floor 01, after the Guild biscuit, before the Juicer":
//!
//! * Carl: the implemented jab, haymaker and push kick. The CF02 moves are
//!   declared but exist only behind `cf02_rules` ([`CapabilityProfile::effective_moves`]).
//! * Donut: Magic Missile with the real `DonutMana` and the pounce. **No
//!   swat.** The post-biscuit state is [`DonutSetup::unlocked`], the same
//!   value `--donut-biscuit` uses (`donut_setup` in `floor01_scene`), not a
//!   new unlock mechanism.
//! * No gear inventory (ADR-0010), no audience, boons, momentum, viewers,
//!   sponsors, shoulder mode, Mongo or new dodge button.
//!
//! The profile is data in the manifest, validated against
//! [`crate::combat_lab::ability`] when the manifest loads.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::LabError;
use super::ability::{AbilityStatus, classify};
use crate::donut_mana::{DonutSetup, ManaTuning};
use crate::party::HeroId;

/// The only profile id of v1. A pre-biscuit profile is a separate, labelled
/// profile that is not defined yet (design open question 4).
pub const PROFILE_ID: &str = "floor01-postbiscuit-lab-v1";
/// Reserved, not defined in v1.
pub const RESERVED_PROFILE_IDS: [&str; 1] = ["floor01-prebiscuit-lab-v1"];
/// The story pin of [`PROFILE_ID`].
pub const STORY_PIN: &str = "floor01_after_guild_biscuit_before_juicer";
/// The systems a v1 profile must declare, all `false`.
pub const SYSTEM_KEYS: [&str; 9] = [
    "audience",
    "boons",
    "momentum",
    "viewers",
    "sponsors",
    "shoulder_mode",
    "mongo",
    "new_dodge_button",
    "gear_inventory",
];

/// A profile as written in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileSpec {
    pub id: String,
    pub story_pin: String,
    /// Carl's always-on moves (implemented ones only).
    pub carl_moves: Vec<String>,
    /// Carl's CF02 moves: declared, live only under `cf02_rules`.
    pub carl_cf02_moves: Vec<String>,
    /// Donut's moves (implemented ones only; never the swat).
    pub donut_moves: Vec<String>,
    pub donut_biscuit: bool,
    /// `"real"`: the real `DonutMana` pool (no fake or infinite mana).
    pub donut_mana: String,
    pub donut_start_mana: u8,
    /// Every system of [`SYSTEM_KEYS`], all `false`.
    pub systems: BTreeMap<String, bool>,
}

/// A validated profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityProfile {
    spec: ProfileSpec,
}

impl CapabilityProfile {
    /// Validate `spec` against the implemented abilities and the Q10 moveset.
    pub fn validate(spec: ProfileSpec) -> Result<Self, LabError> {
        let at = format!("profile {}", spec.id);
        let schema = |reason: &str| LabError::Schema {
            at: at.clone(),
            reason: reason.into(),
        };
        if RESERVED_PROFILE_IDS.contains(&spec.id.as_str()) {
            return Err(schema("this profile id is reserved and not defined in v1"));
        }
        if spec.id != PROFILE_ID {
            return Err(schema(
                "the only profile of v1 is floor01-postbiscuit-lab-v1",
            ));
        }
        if spec.story_pin != STORY_PIN {
            return Err(schema(
                "the story pin must be floor01_after_guild_biscuit_before_juicer",
            ));
        }
        if !spec.donut_biscuit {
            return Err(schema("a post-biscuit profile needs donut_biscuit: true"));
        }
        if spec.donut_mana != "real" {
            return Err(schema(
                "donut_mana must be \"real\": no fake or infinite mana",
            ));
        }
        let pool = ManaTuning::pinned(true)
            .map_err(|e| LabError::Io(e.to_string()))?
            .mana_max;
        if spec.donut_start_mana > pool {
            return Err(schema("donut_start_mana is above the mana pool"));
        }
        for key in SYSTEM_KEYS {
            match spec.systems.get(key) {
                Some(false) => {}
                Some(true) => {
                    return Err(LabError::IllegalAbility {
                        at,
                        id: key.into(),
                        reason: "this system is off in the baseline profile and unimplemented in the lab".into(),
                    });
                }
                None => return Err(schema(&format!("systems.{key} is missing"))),
            }
        }
        if let Some(extra) = spec
            .systems
            .keys()
            .find(|k| !SYSTEM_KEYS.contains(&k.as_str()))
        {
            return Err(LabError::UnknownAbility {
                at,
                id: extra.clone(),
            });
        }
        let check = |list: &[String], hero: HeroId, gated: bool| -> Result<(), LabError> {
            for id in list {
                match classify(id) {
                    AbilityStatus::Implemented(owner) if owner == hero && !gated => {
                        if id == "swat" {
                            return Err(LabError::IllegalAbility {
                                at: at.clone(),
                                id: id.clone(),
                                reason: "post-biscuit Donut has Magic Missile and the pounce, no swat (Q10 section 3)".into(),
                            });
                        }
                    }
                    AbilityStatus::Cf02Gated(owner) if owner == hero && gated => {}
                    AbilityStatus::Cf02Gated(_) => {
                        return Err(LabError::IllegalAbility {
                            at: at.clone(),
                            id: id.clone(),
                            reason: "a CF02 move belongs in carl_cf02_moves, behind cf02_rules"
                                .into(),
                        });
                    }
                    AbilityStatus::Implemented(_) => {
                        return Err(LabError::IllegalAbility {
                            at: at.clone(),
                            id: id.clone(),
                            reason: format!("not a move of {hero:?} in this list"),
                        });
                    }
                    AbilityStatus::Unimplemented(reason) => {
                        return Err(LabError::UnimplementedAbility {
                            at: at.clone(),
                            id: id.clone(),
                            reason: reason.into(),
                        });
                    }
                    AbilityStatus::Unknown => {
                        return Err(LabError::UnknownAbility {
                            at: at.clone(),
                            id: id.clone(),
                        });
                    }
                }
            }
            Ok(())
        };
        check(&spec.carl_moves, HeroId::Carl, false)?;
        check(&spec.carl_cf02_moves, HeroId::Carl, true)?;
        check(&spec.donut_moves, HeroId::Donut, false)?;
        // Magic Missile and the pounce are the post-biscuit kit; both must be there.
        for needed in ["magic_missile", "pounce"] {
            if !spec.donut_moves.iter().any(|m| m == needed) {
                return Err(schema(&format!("donut_moves must name {needed}")));
            }
        }
        Ok(Self { spec })
    }

    pub fn id(&self) -> &str {
        &self.spec.id
    }

    pub fn spec(&self) -> &ProfileSpec {
        &self.spec
    }

    /// Donut's post-biscuit state: the value `--donut-biscuit` starts with.
    pub fn donut_setup(&self) -> DonutSetup {
        DonutSetup::unlocked(self.spec.donut_start_mana)
    }

    /// The moves a run actually has: the legacy ones, plus the CF02 ones only
    /// when `cf02_rules` is on.
    pub fn effective_moves(&self, cf02_rules: bool) -> Vec<&str> {
        let mut moves: Vec<&str> = self
            .spec
            .carl_moves
            .iter()
            .chain(&self.spec.donut_moves)
            .map(String::as_str)
            .collect();
        if cf02_rules {
            moves.extend(self.spec.carl_cf02_moves.iter().map(String::as_str));
        }
        moves
    }

    /// Whether `id` is a move this profile allows (CF02 moves count: they are
    /// declared, whether or not `cf02_rules` is on).
    pub fn allows_move(&self, id: &str) -> bool {
        self.spec
            .carl_moves
            .iter()
            .chain(&self.spec.carl_cf02_moves)
            .chain(&self.spec.donut_moves)
            .any(|m| m == id)
    }
}
