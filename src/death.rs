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
//! its rarer, harder finishes first. A cause maps to an ordered **ladder** of
//! classes (DEATH-D): a shear that has no recipe falls to a crush, a blast to a
//! blunt throw, and the result is flagged `degraded`. A species without a
//! matching recipe plays its own fallback finish, and an unknown species the
//! generic one: the selection never fails and never invents a finish.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::combat::{CombatEvent, DefeatCause, ForceTier};

/// The only schema version this module reads. A profile file with another
/// version is rejected, never half-read (migrate the file explicitly).
pub const SCHEMA_VERSION: u32 = 2;

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
    /// DEATH-D: an edge that severs (the block's edge across the neck).
    Shear,
    /// DEATH-D: an explosion.
    Blast,
}

impl CauseClass {
    /// The ordered classes a cause may play as: the first with a matching
    /// recipe wins. An exhaustive match, so a new cause must decide.
    pub const fn ladder(cause: DefeatCause) -> &'static [Self] {
        match cause {
            DefeatCause::Melee => &[Self::Blunt],
            DefeatCause::ShoreBlock { .. } | DefeatCause::DropHazard { .. } => &[Self::Crush],
            DefeatCause::ShoreBlockEdge { .. } => &[Self::Shear, Self::Crush],
            DefeatCause::Explosion { .. } => &[Self::Blast, Self::Blunt],
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
/// the decapitation, the explosive breakup, or the fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeKind {
    KnockbackDown,
    Crushed,
    /// DEATH-D: only `shear` causes (and at least heavy force).
    Decapitated,
    /// DEATH-D: only `blast` causes (and at least heavy force).
    Exploded,
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

    /// The only cause class a recipe of this kind may list, so no data file
    /// can make a punch decapitate or explode (cross-class fallback is the
    /// ladder's job, not the data's).
    const fn required_class(kind: RecipeKind) -> Option<CauseClass> {
        match kind {
            RecipeKind::KnockbackDown => Some(CauseClass::Blunt),
            RecipeKind::Crushed => Some(CauseClass::Crush),
            RecipeKind::Decapitated => Some(CauseClass::Shear),
            RecipeKind::Exploded => Some(CauseClass::Blast),
            RecipeKind::Fallback => None,
        }
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
    /// True when the match came from a class after the first of the cause's
    /// ladder (a shear played as a crush, a blast played as a throw).
    pub degraded: bool,
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
                if let Some(class) = Recipe::required_class(recipe.kind)
                    && recipe.causes.iter().any(|c| *c != class)
                {
                    return Err(format!(
                        "{}: recipe {:?} lists causes other than {class:?}",
                        species.id, recipe.kind
                    ));
                }
                if matches!(recipe.kind, RecipeKind::Decapitated | RecipeKind::Exploded)
                    && recipe.min_force < ForceTier::Heavy
                {
                    return Err(format!(
                        "{}: recipe {:?} needs min_force of at least heavy",
                        species.id, recipe.kind
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn species(&self, id: &str) -> Option<&SpeciesProfile> {
        self.species.iter().find(|s| s.id == id)
    }

    /// The finish `species` plays when it is defeated by `cause` with `force`.
    /// Pure and total: walk the cause's ladder of classes; for each class the
    /// first matching recipe in file order wins, else the species fallback,
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
                degraded: false,
            };
        };
        for (rung, class) in CauseClass::ladder(cause).iter().enumerate() {
            if let Some(recipe) = profile.recipes.iter().find(|r| r.accepts(*class, force)) {
                return FinishSelection {
                    finish: &recipe.finish,
                    kind: recipe.kind,
                    fallback: false,
                    degraded: rung > 0,
                };
            }
        }
        FinishSelection {
            finish: &profile.fallback_finish,
            kind: RecipeKind::Fallback,
            fallback: true,
            degraded: false,
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
    const EDGE: DefeatCause = DefeatCause::ShoreBlockEdge { player: 0 };
    const BLAST_PB: DefeatCause = DefeatCause::Explosion {
        charge: 0,
        player: None,
        point_blank: true,
    };
    const BLAST_RING: DefeatCause = DefeatCause::Explosion {
        charge: 0,
        player: None,
        point_blank: false,
    };

    fn finish(species: &str, cause: DefeatCause, force: ForceTier) -> (String, RecipeKind, bool) {
        let s = DeathProfiles::builtin().select(species, cause, force);
        (s.finish.to_string(), s.kind, s.fallback)
    }

    #[test]
    fn the_embedded_profiles_are_valid_and_every_species_has_a_fallback() {
        let profiles = DeathProfiles::builtin();
        assert_eq!(profiles.schema_version, SCHEMA_VERSION);
        let ids: Vec<&str> = profiles.species.iter().map(|s| s.id.as_str()).collect();
        // DEATH-D changes this assertion on purpose: schema 2 adds the llama.
        assert_eq!(
            ids,
            [
                "goblin",
                "llama",
                "skeleton_mock",
                "slime_mock",
                "brute_boss"
            ]
        );
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
        use RecipeKind::{Crushed, Fallback, KnockbackDown};
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
        // DEATH-D changes this on purpose: schema 2 is current, version 1 is
        // rejected (never half-read).
        let bad_version = ok.replace("\"schema_version\": 2", "\"schema_version\": 1");
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

    #[test]
    fn schema_two_validation_forbids_a_melee_decapitation_or_explosion() {
        let ok = include_str!("../assets/data/death/species_death_profiles.json");
        let goblin_decap = r#"{ "kind": "decapitated", "finish": "goblin_decapitated", "causes": ["shear"], "min_force": "heavy" }"#;
        let goblin_boom = r#"{ "kind": "exploded", "finish": "goblin_exploded", "causes": ["blast"], "min_force": "catastrophic" }"#;
        assert!(ok.contains(goblin_decap) && ok.contains(goblin_boom));
        let rejected = |text: String, why: &str| {
            assert!(DeathProfiles::from_json(&text).is_err(), "{why}");
        };
        // A decapitation that accepts a blunt hit (a punch would decapitate).
        rejected(
            ok.replace(
                goblin_decap,
                &goblin_decap.replace("\"shear\"", "\"blunt\""),
            ),
            "decapitated with blunt",
        );
        rejected(
            ok.replace(
                goblin_decap,
                &goblin_decap.replace("[\"shear\"]", "[\"shear\", \"blunt\"]"),
            ),
            "decapitated with shear and blunt",
        );
        rejected(
            ok.replace(
                goblin_decap,
                &goblin_decap.replace("\"shear\"", "\"crush\""),
            ),
            "decapitated with crush",
        );
        // An explosion that accepts a blunt hit or a crush.
        rejected(
            ok.replace(goblin_boom, &goblin_boom.replace("\"blast\"", "\"blunt\"")),
            "exploded with blunt",
        );
        rejected(
            ok.replace(goblin_boom, &goblin_boom.replace("\"blast\"", "\"crush\"")),
            "exploded with crush",
        );
        // Cross-class recipes of the older kinds are refused too.
        let crushed = r#"{ "kind": "crushed", "finish": "goblin_crushed", "causes": ["crush"], "min_force": "routine" }"#;
        rejected(
            ok.replace(crushed, &crushed.replace("\"crush\"", "\"shear\"")),
            "crushed with shear",
        );
        // Both finishes need at least heavy force.
        rejected(
            ok.replace(
                goblin_decap,
                &goblin_decap.replace("\"heavy\"", "\"routine\""),
            ),
            "decapitated at routine force",
        );
        rejected(
            ok.replace(
                goblin_boom,
                &goblin_boom.replace("\"catastrophic\"", "\"routine\""),
            ),
            "exploded at routine force",
        );
        // Unknown class and kind, and the old version.
        rejected(ok.replace("\"shear\"", "\"slice\""), "unknown class");
        rejected(
            ok.replace("\"decapitated\"", "\"beheaded\""),
            "unknown kind",
        );
        // Heavy or catastrophic is accepted for both.
        assert!(
            DeathProfiles::from_json(&ok.replace(
                goblin_boom,
                &goblin_boom.replace("\"catastrophic\"", "\"heavy\"")
            ))
            .is_ok()
        );
    }

    #[test]
    fn the_ladders_and_the_selection_table_of_death_d() {
        use CauseClass::{Blast, Blunt, Crush, Shear};
        assert_eq!(CauseClass::ladder(MELEE), [Blunt]);
        assert_eq!(CauseClass::ladder(BLOCK), [Crush]);
        assert_eq!(CauseClass::ladder(EDGE), [Shear, Crush]);
        assert_eq!(CauseClass::ladder(BLAST_PB), [Blast, Blunt]);
        assert_eq!(CauseClass::ladder(BLAST_RING), [Blast, Blunt]);
    }

    /// (species, cause, force, finish, kind, degraded, fallback): the section
    /// 2.4 table of the DEATH-D spec, every row.
    #[test]
    fn selection_table_death_d_every_row() {
        use ForceTier::{Catastrophic, Heavy, Routine};
        use RecipeKind::{Crushed, Decapitated, Exploded, Fallback, KnockbackDown};
        #[allow(clippy::type_complexity)]
        let table: &[(&str, DefeatCause, ForceTier, &str, RecipeKind, bool, bool)] = &[
            (
                "goblin",
                MELEE,
                Routine,
                "goblin_knockback_down",
                KnockbackDown,
                false,
                false,
            ),
            (
                "goblin",
                MELEE,
                Heavy,
                "goblin_knockback_down",
                KnockbackDown,
                false,
                false,
            ),
            (
                "goblin",
                BLOCK,
                Catastrophic,
                "goblin_crushed",
                Crushed,
                false,
                false,
            ),
            (
                "goblin",
                EDGE,
                Catastrophic,
                "goblin_decapitated",
                Decapitated,
                false,
                false,
            ),
            (
                "goblin",
                BLAST_PB,
                Catastrophic,
                "goblin_exploded",
                Exploded,
                false,
                false,
            ),
            (
                "goblin",
                BLAST_RING,
                Heavy,
                "goblin_knockback_down",
                KnockbackDown,
                true,
                false,
            ),
            (
                "llama",
                EDGE,
                Catastrophic,
                "llama_decapitated",
                Decapitated,
                false,
                false,
            ),
            (
                "llama",
                BLAST_PB,
                Catastrophic,
                "llama_exploded",
                Exploded,
                false,
                false,
            ),
            (
                "llama",
                BLOCK,
                Catastrophic,
                "llama_crushed",
                Crushed,
                false,
                false,
            ),
            (
                "llama",
                MELEE,
                Routine,
                "llama_down",
                KnockbackDown,
                false,
                false,
            ),
            (
                "llama",
                MELEE,
                Heavy,
                "llama_down",
                KnockbackDown,
                false,
                false,
            ),
            (
                "llama",
                BLAST_RING,
                Heavy,
                "llama_down",
                KnockbackDown,
                true,
                false,
            ),
            (
                "skeleton_mock",
                EDGE,
                Catastrophic,
                "skeleton_flat_pile",
                Crushed,
                true,
                false,
            ),
            (
                "skeleton_mock",
                BLAST_PB,
                Catastrophic,
                "skeleton_scatter",
                KnockbackDown,
                true,
                false,
            ),
            (
                "skeleton_mock",
                BLAST_RING,
                Heavy,
                "skeleton_scatter",
                KnockbackDown,
                true,
                false,
            ),
            (
                "slime_mock",
                EDGE,
                Catastrophic,
                "slime_spread",
                Crushed,
                true,
                false,
            ),
            (
                "slime_mock",
                BLAST_PB,
                Catastrophic,
                "slime_puddle",
                Fallback,
                false,
                true,
            ),
            (
                "slime_mock",
                BLAST_RING,
                Heavy,
                "slime_puddle",
                Fallback,
                false,
                true,
            ),
            (
                "nobody",
                EDGE,
                Catastrophic,
                "generic_collapse",
                Fallback,
                false,
                true,
            ),
            (
                "nobody",
                BLAST_PB,
                Catastrophic,
                "generic_collapse",
                Fallback,
                false,
                true,
            ),
            (
                "nobody",
                BLAST_RING,
                Heavy,
                "generic_collapse",
                Fallback,
                false,
                true,
            ),
            (
                "nobody",
                MELEE,
                Routine,
                "generic_collapse",
                Fallback,
                false,
                true,
            ),
        ];
        for &(species, cause, force, want, kind, degraded, fallback) in table {
            let s = DeathProfiles::builtin().select(species, cause, force);
            assert_eq!(
                (s.finish, s.kind, s.degraded, s.fallback),
                (want, kind, degraded, fallback),
                "{species} {cause:?} {force:?}"
            );
        }
    }

    #[test]
    fn no_melee_defeat_of_any_force_selects_a_decapitation_or_an_explosion() {
        let profiles = DeathProfiles::builtin();
        for species in profiles
            .species
            .iter()
            .map(|s| s.id.as_str())
            .chain(["nobody"])
        {
            for force in [
                ForceTier::Routine,
                ForceTier::Heavy,
                ForceTier::Catastrophic,
            ] {
                let s = profiles.select(species, MELEE, force);
                assert!(
                    !matches!(s.kind, RecipeKind::Decapitated | RecipeKind::Exploded),
                    "{species} {force:?}: {}",
                    s.finish
                );
            }
        }
    }

    #[test]
    fn a_species_without_a_decapitation_or_explosion_recipe_never_gets_one() {
        let profiles = DeathProfiles::builtin();
        let causes = [MELEE, BLOCK, EDGE, BLAST_PB, BLAST_RING];
        for species in &profiles.species {
            for kind in [RecipeKind::Decapitated, RecipeKind::Exploded] {
                if species.recipes.iter().any(|r| r.kind == kind) {
                    continue;
                }
                for cause in causes {
                    for force in [
                        ForceTier::Routine,
                        ForceTier::Heavy,
                        ForceTier::Catastrophic,
                    ] {
                        let s = profiles.select(&species.id, cause, force);
                        assert_ne!(s.kind, kind, "{} {cause:?} {force:?}", species.id);
                    }
                }
            }
        }
    }

    #[test]
    fn a_shear_or_blast_only_selects_its_finish_from_a_species_that_has_one() {
        let profiles = DeathProfiles::builtin();
        for (cause, kind) in [
            (EDGE, RecipeKind::Decapitated),
            (BLAST_PB, RecipeKind::Exploded),
        ] {
            let with: Vec<&str> = profiles
                .species
                .iter()
                .filter(|s| s.recipes.iter().any(|r| r.kind == kind))
                .map(|s| s.id.as_str())
                .collect();
            assert_eq!(with, ["goblin", "llama"]);
            for species in with {
                let s = profiles.select(species, cause, ForceTier::Catastrophic);
                assert_eq!(s.kind, kind, "{species}");
                assert!(!s.degraded && !s.fallback);
            }
        }
    }

    #[test]
    fn force_tiers_are_ordered() {
        assert!(ForceTier::Routine < ForceTier::Heavy);
        assert!(ForceTier::Heavy < ForceTier::Catastrophic);
    }

    #[test]
    fn the_brute_boss_falls_like_a_felled_tree_or_is_crushed_under_the_rack() {
        let profiles = DeathProfiles::builtin();
        let rack = DefeatCause::DropHazard { player: 0, rack: 0 };
        let pick = |cause, force| profiles.select("brute_boss", cause, force);
        for force in [ForceTier::Routine, ForceTier::Heavy] {
            assert_eq!(pick(MELEE, force).finish, "brute_ko_timber");
        }
        let crushed = pick(rack, ForceTier::Catastrophic);
        assert_eq!(
            (crushed.finish, crushed.kind),
            ("brute_crushed", RecipeKind::Crushed)
        );
        // A later explosive finish has no recipe yet: it falls back to the
        // timber knockout (design section 5).
        let blown = pick(MELEE, ForceTier::Catastrophic);
        assert_eq!(blown.finish, "brute_ko_timber");
        assert_eq!(CauseClass::ladder(rack), [CauseClass::Crush]);
    }
}
