//! DEATH-B: data-driven species death profiles and the finish selection.
//!
//! Pure, Bevy-free domain code. The authoritative defeat is the one
//! [`CombatEvent::EnemyDefeated`] (`cause`, `force`, `instigator`); this module
//! only **derives** from it which finish a species plays: nothing here is
//! stored in simulation state or mixed into a replay hash, and the gore preset
//! (presentation intensity) is not an input. The Bevy layer (DEATH-C) reads
//! the selected finish id and draws it; it never feeds anything back.
//!
//! Profiles are data (`assets/data/death/species_death_profiles.json`, embedded
//! at build time, schema in `docs/03-tech/species-death-profiles.md`). A
//! recipe lists the cause classes it accepts and the least force it needs;
//! recipes are tried in file order and the first match wins, so a species puts
//! its rarer, harder finishes first. A species without a matching recipe plays
//! its own fallback finish, and an unknown species the generic one: the
//! selection never fails and never invents a finish.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::combat::{CombatEvent, DefeatCause, ForceTier};

/// The only schema version this module reads. A profile file with another
/// version is rejected, never half-read (migrate the file explicitly).
pub const SCHEMA_VERSION: u32 = 1;

const BUILTIN_JSON: &str = include_str!("../assets/data/death/species_death_profiles.json");

/// What kind of lethal contact it was, coarser than [`DefeatCause`]: profiles
/// speak in classes so a new cause maps onto an existing recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CauseClass {
    /// A melee hit: recoil, launch, crumple.
    Blunt,
    /// A falling mass.
    Crush,
}

impl CauseClass {
    pub const fn of(cause: DefeatCause) -> Self {
        match cause {
            DefeatCause::Melee => Self::Blunt,
            DefeatCause::ShoreBlock { .. } => Self::Crush,
        }
    }
}

/// The body material of a species (what it is made of, not how it looks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyMaterial {
    Organic,
    Skeletal,
    Gelatinous,
    Constructed,
}

/// The family of a finish: the ordinary knockback and down, the crushed body,
/// or the fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeKind {
    KnockbackDown,
    Crushed,
    /// LLAMA-02 (owner decision 2026-11-19): a lethal heavy blunt hit throws the
    /// body onto its back (`slam_contact`, `broken_heap`).
    Slammed,
    /// Not a recipe a file can declare: reported when no recipe matched.
    #[serde(skip)]
    Fallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub kind: RecipeKind,
    /// The finish id the presentation layer draws.
    pub finish: String,
    /// Cause classes this recipe accepts (never empty).
    pub causes: Vec<CauseClass>,
    /// The least force this recipe needs.
    pub min_force: ForceTier,
}

impl Recipe {
    fn accepts(&self, class: CauseClass, force: ForceTier) -> bool {
        self.causes.contains(&class) && force >= self.min_force
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeciesProfile {
    /// IP-neutral species key (the fan-concept name is replaceable data).
    pub id: String,
    pub body_material: BodyMaterial,
    /// Played when no recipe matches.
    pub fallback_finish: String,
    pub recipes: Vec<Recipe>,
}

/// The finish a species plays for one defeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinishSelection<'a> {
    pub finish: &'a str,
    pub kind: RecipeKind,
    /// True when the species had no matching recipe (its own or the generic
    /// fallback finish was chosen).
    pub fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeathProfiles {
    pub schema_version: u32,
    /// Played for a species that has no profile at all.
    pub generic_fallback_finish: String,
    pub species: Vec<SpeciesProfile>,
}

impl DeathProfiles {
    /// Parse and validate a profile file.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let profiles: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        profiles.validate()?;
        Ok(profiles)
    }

    /// The profiles shipped with the game. Panics if the embedded file is
    /// invalid (a unit test pins that it is not).
    pub fn builtin() -> &'static Self {
        static BUILTIN: OnceLock<DeathProfiles> = OnceLock::new();
        BUILTIN.get_or_init(|| Self::from_json(BUILTIN_JSON).expect("embedded death profiles"))
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "schema_version {} is not {SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        if self.generic_fallback_finish.is_empty() {
            return Err("generic_fallback_finish is empty".into());
        }
        let mut seen = BTreeSet::new();
        for species in &self.species {
            if species.id.is_empty() || !seen.insert(species.id.as_str()) {
                return Err(format!("empty or duplicate species id {:?}", species.id));
            }
            if species.fallback_finish.is_empty() {
                return Err(format!("{}: fallback_finish is empty", species.id));
            }
            for recipe in &species.recipes {
                if recipe.finish.is_empty() || recipe.causes.is_empty() {
                    return Err(format!("{}: recipe without finish or causes", species.id));
                }
                if recipe.kind == RecipeKind::Fallback {
                    return Err(format!("{}: a recipe cannot be a fallback", species.id));
                }
            }
        }
        Ok(())
    }

    pub fn species(&self, id: &str) -> Option<&SpeciesProfile> {
        self.species.iter().find(|s| s.id == id)
    }

    /// The finish `species` plays when it is defeated by `cause` with `force`.
    /// Pure and total: the first matching recipe, else the species fallback,
    /// else the generic one.
    pub fn select(
        &self,
        species: &str,
        cause: DefeatCause,
        force: ForceTier,
    ) -> FinishSelection<'_> {
        let Some(profile) = self.species(species) else {
            return FinishSelection {
                finish: &self.generic_fallback_finish,
                kind: RecipeKind::Fallback,
                fallback: true,
            };
        };
        let class = CauseClass::of(cause);
        match profile.recipes.iter().find(|r| r.accepts(class, force)) {
            Some(recipe) => FinishSelection {
                finish: &recipe.finish,
                kind: recipe.kind,
                fallback: false,
            },
            None => FinishSelection {
                finish: &profile.fallback_finish,
                kind: RecipeKind::Fallback,
                fallback: true,
            },
        }
    }

    /// The target and finish of a defeat event (`None` for any other event).
    /// The species is not in the event: the caller knows it from the target id.
    pub fn select_for_event(
        &self,
        species: &str,
        event: &CombatEvent,
    ) -> Option<(u32, FinishSelection<'_>)> {
        match *event {
            CombatEvent::EnemyDefeated {
                target,
                cause,
                force,
                ..
            } => Some((target, self.select(species, cause, force))),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MELEE: DefeatCause = DefeatCause::Melee;
    const BLOCK: DefeatCause = DefeatCause::ShoreBlock { player: 0 };

    fn finish(species: &str, cause: DefeatCause, force: ForceTier) -> (String, RecipeKind, bool) {
        let s = DeathProfiles::builtin().select(species, cause, force);
        (s.finish.to_string(), s.kind, s.fallback)
    }

    #[test]
    fn the_embedded_profiles_are_valid_and_every_species_has_a_fallback() {
        let profiles = DeathProfiles::builtin();
        assert_eq!(profiles.schema_version, SCHEMA_VERSION);
        let ids: Vec<&str> = profiles.species.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["goblin", "skeleton_mock", "slime_mock", "llama"]);
        assert!(
            profiles
                .species
                .iter()
                .all(|s| !s.fallback_finish.is_empty())
        );
    }

    #[test]
    fn selection_table_by_species_cause_and_force() {
        use ForceTier::{Catastrophic, Heavy, Routine};
        use RecipeKind::{Crushed, Fallback, KnockbackDown, Slammed};
        // (species, cause, force) -> (finish, kind, fallback used)
        let table: &[(&str, DefeatCause, ForceTier, &str, RecipeKind, bool)] = &[
            (
                "goblin",
                MELEE,
                Routine,
                "goblin_knockback_down",
                KnockbackDown,
                false,
            ),
            (
                "goblin",
                MELEE,
                Heavy,
                "goblin_knockback_down",
                KnockbackDown,
                false,
            ),
            (
                "goblin",
                BLOCK,
                Catastrophic,
                "goblin_crushed",
                Crushed,
                false,
            ),
            (
                "skeleton_mock",
                MELEE,
                Routine,
                "skeleton_knockback_down",
                KnockbackDown,
                false,
            ),
            (
                "skeleton_mock",
                MELEE,
                Heavy,
                "skeleton_scatter",
                KnockbackDown,
                false,
            ),
            (
                "skeleton_mock",
                BLOCK,
                Catastrophic,
                "skeleton_flat_pile",
                Crushed,
                false,
            ),
            (
                "slime_mock",
                BLOCK,
                Catastrophic,
                "slime_spread",
                Crushed,
                false,
            ),
            // The Bad Llama (DEMO-01 PR B): the slam rule (owner decision
            // 2026-11-19). A lethal Heavy blunt hit (the push kick) slams; a
            // Routine one (jab, haymaker) keeps the plain LLAMA-01 down; the
            // falling block crushes.
            ("llama", MELEE, Heavy, "llama_slammed", Slammed, false),
            ("llama", MELEE, Routine, "llama_down", KnockbackDown, false),
            (
                "llama",
                BLOCK,
                Catastrophic,
                "llama_crushed",
                Crushed,
                false,
            ),
            // No knockback recipe: the species fallback.
            ("slime_mock", MELEE, Routine, "slime_puddle", Fallback, true),
            ("slime_mock", MELEE, Heavy, "slime_puddle", Fallback, true),
            // No profile at all: the generic fallback.
            (
                "unknown_species",
                MELEE,
                Routine,
                "generic_collapse",
                Fallback,
                true,
            ),
            (
                "unknown_species",
                BLOCK,
                Catastrophic,
                "generic_collapse",
                Fallback,
                true,
            ),
        ];
        for &(species, cause, force, want, kind, fallback) in table {
            assert_eq!(
                finish(species, cause, force),
                (want.to_string(), kind, fallback),
                "{species} {cause:?} {force:?}"
            );
        }
    }

    #[test]
    fn a_routine_jab_never_picks_a_heavy_finish() {
        let profiles = DeathProfiles::builtin();
        for species in &profiles.species {
            let pick = profiles.select(&species.id, MELEE, ForceTier::Routine);
            let recipe = species.recipes.iter().find(|r| r.finish == pick.finish);
            if let Some(recipe) = recipe {
                assert_eq!(recipe.min_force, ForceTier::Routine, "{}", species.id);
            }
        }
    }

    #[test]
    fn selection_is_a_pure_function_of_the_event() {
        let event = CombatEvent::EnemyDefeated {
            tick: 9,
            target: 2,
            cause: BLOCK,
            force: ForceTier::Catastrophic,
            instigator: Some(0),
        };
        let profiles = DeathProfiles::builtin();
        let a = profiles.select_for_event("goblin", &event);
        let b = profiles.select_for_event("goblin", &event);
        assert_eq!(a, b);
        assert_eq!(a.unwrap().0, 2);
        assert_eq!(a.unwrap().1.finish, "goblin_crushed");
        // Other events select nothing.
        assert!(
            profiles
                .select_for_event("goblin", &CombatEvent::ShoreLanded { tick: 9 })
                .is_none()
        );
    }

    #[test]
    fn invalid_profile_files_are_rejected() {
        let ok = include_str!("../assets/data/death/species_death_profiles.json");
        assert!(DeathProfiles::from_json(ok).is_ok());
        let bad_version = ok.replace("\"schema_version\": 1", "\"schema_version\": 2");
        assert!(DeathProfiles::from_json(&bad_version).is_err());
        let dup = ok.replace("\"skeleton_mock\"", "\"goblin\"");
        assert!(DeathProfiles::from_json(&dup).is_err());
        let no_fallback = ok.replace(
            "\"fallback_finish\": \"slime_puddle\"",
            "\"fallback_finish\": \"\"",
        );
        assert!(DeathProfiles::from_json(&no_fallback).is_err());
        let unknown_tier = ok.replace("\"heavy\"", "\"mighty\"");
        assert!(DeathProfiles::from_json(&unknown_tier).is_err());
        let extra = ok.replace("\"id\": \"goblin\",", "\"id\": \"goblin\", \"gore\": 4,");
        assert!(DeathProfiles::from_json(&extra).is_err(), "no gore field");
    }

    /// The slam rule, from the defeat events themselves: a lethal push kick (a
    /// launching move: `Heavy`) selects the slam, a lethal jab or haymaker the
    /// plain down. The goblin is unaffected by the rule.
    #[test]
    fn the_slam_rule_follows_the_lethal_moves_force() {
        use crate::combat::{CARL_HAYMAKER, CARL_JAB, CARL_PUSH_KICK};
        let profiles = DeathProfiles::builtin();
        let defeat = |spec: &crate::combat::MoveSpec| {
            CombatEvent::enemy_defeated(5, 1, DefeatCause::Melee, Some(0), Some(spec))
        };
        let pick = |species: &str, spec: &crate::combat::MoveSpec| {
            profiles
                .select_for_event(species, &defeat(spec))
                .map(|(target, selection)| (target, selection.finish.to_string(), selection.kind))
        };
        assert_eq!(
            pick("llama", &CARL_PUSH_KICK),
            Some((1, "llama_slammed".into(), RecipeKind::Slammed))
        );
        for routine in [&CARL_JAB, &CARL_HAYMAKER] {
            assert_eq!(
                pick("llama", routine),
                Some((1, "llama_down".into(), RecipeKind::KnockbackDown))
            );
        }
        // The slam is a llama recipe only: a goblin's kick is still a knockback.
        assert_eq!(
            pick("goblin", &CARL_PUSH_KICK).map(|p| p.1),
            Some("goblin_knockback_down".into())
        );
        // The slam recipe is listed ahead of the plain down (first match wins).
        let llama = profiles.species("llama").unwrap();
        let slam = llama
            .recipes
            .iter()
            .position(|r| r.kind == RecipeKind::Slammed);
        let down = llama
            .recipes
            .iter()
            .position(|r| r.kind == RecipeKind::KnockbackDown);
        assert!(slam < down);
        assert_eq!(llama.fallback_finish, "llama_down");
    }

    #[test]
    fn force_tiers_are_ordered() {
        assert!(ForceTier::Routine < ForceTier::Heavy);
        assert!(ForceTier::Heavy < ForceTier::Catastrophic);
    }
}
