//! CARL-PROG-01: the run flags and the tuning of Carl's Floor 01 combat
//! progression (CP-A scaffold, spec sections 3 to 5).
//!
//! Bevy-free. CP-A lands the **complete** knob file
//! (`assets/data/gameplay/carl_prog_v0.json`, every key named by its question
//! ID) with its loader, validation and FNV-1a pin, plus the three debug flags
//! `--carl-prog`, `--carl-dynamite` and `--carl-gauntlet` as a value
//! ([`CarlProgFlags`]). Nothing here reads a knob in the simulation: the
//! thrown charge (CP-B1), the interactions (CP-C) and the reinforced fist
//! (CP-D) only read values. The capability ids and the unlock rules are the
//! generic ones of [`crate::story_unlock`].

use serde::Deserialize;

use crate::combat::{MoveId, MoveSpec};
use crate::donut_mana::content_hash;
use crate::story_unlock::{SceneKind, StorySetup};

/// The tuning file, embedded so the binary and the tests read the same bytes.
pub const TUNING_JSON: &str = include_str!("../assets/data/gameplay/carl_prog_v0.json");

/// The committed FNV-1a 64 content hash of [`TUNING_JSON`]. Editing the file
/// without updating this value fails [`CarlProgTuning::pinned`] and the pin
/// test, so a knob edit never changes a recorded outcome silently.
pub const TUNING_HASH: u64 = 0x371a_22a2_c9b5_240d;

/// The schema version [`CarlProgTuning`] accepts.
pub const SCHEMA_VERSION: u32 = 1;

/// The story flag `--carl-prog` turns on (its pins and triggers).
pub const STORY_FLAG: &str = "carl_prog";

/// The three debug flags of the run (CP-Q09). All are refused by the
/// command line in the combat lab scene and do nothing to a scripted fixture.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CarlProgFlags {
    /// `--carl-prog`: the pins and triggers of the scene.
    pub prog: bool,
    /// `--carl-dynamite`: the thrown charge is held from tick 0.
    pub dynamite: bool,
    /// `--carl-gauntlet`: the reinforced fist is held from tick 0.
    pub gauntlet: bool,
}

impl CarlProgFlags {
    pub const OFF: Self = Self {
        prog: false,
        dynamite: false,
        gauntlet: false,
    };

    pub const fn any(self) -> bool {
        self.prog || self.dynamite || self.gauntlet
    }

    /// The story setup these flags give `scene`. With no flag it is
    /// [`StorySetup::NONE`], so a run without them is unchanged. It is the
    /// shared entry point with the Donut program off ([`crate::donut_prog::story_setup`]).
    pub fn story_setup(self, scene: SceneKind) -> StorySetup {
        crate::donut_prog::story_setup(self, crate::donut_prog::DonutProgFlags::OFF, scene)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuningError {
    /// The JSON did not parse or has an unknown or missing field.
    Parse(String),
    /// A field failed validation; `field` names it.
    Field { field: &'static str, reason: String },
    /// The file's content hash is not the pinned one.
    HashMismatch { expected: u64, actual: u64 },
}

impl core::fmt::Display for TuningError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "carl prog tuning: {msg}"),
            Self::Field { field, reason } => {
                write!(f, "carl prog tuning: field `{field}`: {reason}")
            }
            Self::HashMismatch { expected, actual } => write!(
                f,
                "carl prog tuning hash {actual:#018x} is not the pinned {expected:#018x}"
            ),
        }
    }
}

impl std::error::Error for TuningError {}

/// A terrain row of the blast-break table (CP-Q37). Both ship disabled: no
/// route solid is breakable until TERRAIN-L1.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainRow {
    pub material: String,
    pub enabled: bool,
}

/// `assets/data/gameplay/carl_prog_v0.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarlProgTuning {
    pub schema_version: u32,
    pub provenance: String,
    pub cpq13_grounded_checked_at_press_only: bool,
    pub cpq14_throw_ticks: u16,
    pub cpq14_fuse_lit_tick: u16,
    pub cpq14_hold_tick: u16,
    pub cpq14_wind_tick: u16,
    pub cpq14_release_tick: u16,
    pub cpq14_spawn_tick: u16,
    pub cpq14_follow_tick: u16,
    pub cpq14_brace_tick: u16,
    pub cpq14_settle_tick: u16,
    pub cpq14_cancel_from_tick: u16,
    pub cpq15_jump_ignored_until_tick: u16,
    pub cpq16_spawn_dx_px: u16,
    pub cpq16_spawn_dy_px: u16,
    pub cpq17_vx_base_px_s: u16,
    pub cpq17_vx_thrower_share_pct: u8,
    pub cpq17_vy_px_s: u16,
    pub cpq17_gravity_px_s2: u16,
    pub cpq18_fuse_ticks: u16,
    pub cpq19_recharge_ticks: u16,
    pub cpq19_max_live: u8,
    pub cpq20_core_radius_px: u16,
    pub cpq20_outer_radius_px: u16,
    pub cpq20_centre_dy_px: u16,
    pub cpq21_outer_damage: u8,
    pub cpq21_launch_vx_px_s: u16,
    pub cpq21_launch_vy_px_s: u16,
    pub cpq23_self_damage: u8,
    pub cpq23_self_knock_vx_px_s: u16,
    pub cpq23_self_knock_vy_px_s: u16,
    pub cpq23_self_invulnerable_ticks: u16,
    pub cpq25_boss_damage: u8,
    pub cpq26_llama_outer_damage: u8,
    pub cpq27_drop_from_tick: u16,
    pub cpq32_socket_label: String,
    pub cpq34_chain_delay_ticks: u16,
    pub cpq34_chain_max_depth: u8,
    pub cpq34_chain_max_per_tick: u8,
    pub cpq37_terrain_rows: Vec<TerrainRow>,
    pub cpq38_jab_reinforced: bool,
    pub cpq38_haymaker_reinforced: bool,
    pub cpq39_jab_stagger_ticks: u16,
    pub cpq39_haymaker_stagger_ticks: u16,
    pub cpq40_jab_pierces_commit: bool,
    pub cpq40_haymaker_pierces_commit: bool,
    pub cpq40_boss_poise_multiplier: u8,
    pub cpq41_jab_support_hits: u8,
    pub cpq41_haymaker_support_hits: u8,
    pub cpq42_haymaker_hit_stop_bonus_ticks: u8,
    pub cpq42_pierce_camera_kick_px: u8,
}

impl CarlProgTuning {
    /// Parse and validate.
    pub fn from_json(text: &str) -> Result<Self, TuningError> {
        let tuning: Self =
            serde_json::from_str(text).map_err(|err| TuningError::Parse(err.to_string()))?;
        tuning.validate()?;
        Ok(tuning)
    }

    /// Parse the embedded file and require the pinned content hash.
    pub fn pinned() -> Result<Self, TuningError> {
        Self::from_json_pinned(TUNING_JSON, TUNING_HASH)
    }

    /// Parse `text` and require its content hash to equal `expected`. A replay
    /// header or a fixture records the hash it ran with and calls this.
    pub fn from_json_pinned(text: &str, expected: u64) -> Result<Self, TuningError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(TuningError::HashMismatch { expected, actual });
        }
        Self::from_json(text)
    }

    /// The pinned tuning every simulation reads: one shared static.
    pub fn active() -> &'static Self {
        static ACTIVE: std::sync::OnceLock<CarlProgTuning> = std::sync::OnceLock::new();
        ACTIVE.get_or_init(|| {
            Self::pinned().expect("the embedded Carl progression data is pinned and valid")
        })
    }

    /// CP-Q38 to CP-Q42: the move as it lands while the reinforced fist is held.
    /// Only the jab and the haymaker change (CP-Q38; the kicks, the stomp, the air
    /// kick and the dive stomp come back as they are); damage never changes.
    /// The stagger is lengthened (CP-Q39), the haymaker freezes one tick longer
    /// (CP-Q42) and the punch pierces a committed attack (CP-Q40), each by its
    /// own data row. Pure; the caller applies it only while the capability is held.
    pub fn reinforce(&self, spec: &MoveSpec) -> MoveSpec {
        let (reinforced, stagger, pierces, hit_stop_bonus) = match spec.id {
            MoveId::Jab => (
                self.cpq38_jab_reinforced,
                self.cpq39_jab_stagger_ticks,
                self.cpq40_jab_pierces_commit,
                0,
            ),
            MoveId::Haymaker => (
                self.cpq38_haymaker_reinforced,
                self.cpq39_haymaker_stagger_ticks,
                self.cpq40_haymaker_pierces_commit,
                self.cpq42_haymaker_hit_stop_bonus_ticks,
            ),
            _ => return *spec,
        };
        if !reinforced {
            return *spec;
        }
        let mut landed = *spec;
        landed.reaction.stagger_ticks = u8::try_from(stagger).unwrap_or(u8::MAX);
        landed.reaction.hit_stop = landed.reaction.hit_stop.saturating_add(hit_stop_bonus);
        landed.pierces_commit = pierces;
        landed
    }

    /// CP-Q41: how many hits `id` counts on an authored support (the shore, the
    /// rack strut, a prop's brace) while the reinforced fist is held: the haymaker
    /// 2, the jab 1, every other move 1.
    pub fn support_hit_weight(&self, id: MoveId) -> u8 {
        match id {
            MoveId::Jab if self.cpq38_jab_reinforced => self.cpq41_jab_support_hits,
            MoveId::Haymaker if self.cpq38_haymaker_reinforced => self.cpq41_haymaker_support_hits,
            _ => 1,
        }
    }

    /// CP-Q40: the poise a landed move adds to a boss-class target that holds
    /// super armour through a committed phase: its damage, times the data
    /// multiplier for a reinforced punch (the Juicer's armour is never pierced).
    pub fn boss_poise(&self, spec: &MoveSpec) -> u16 {
        let reinforced = match spec.id {
            MoveId::Jab => self.cpq38_jab_reinforced,
            MoveId::Haymaker => self.cpq38_haymaker_reinforced,
            _ => false,
        };
        let multiplier = if reinforced {
            u16::from(self.cpq40_boss_poise_multiplier)
        } else {
            1
        };
        u16::from(spec.damage) * multiplier
    }

    pub fn validate(&self) -> Result<(), TuningError> {
        let bad = |field: &'static str, reason: &str| {
            Err(TuningError::Field {
                field,
                reason: reason.to_owned(),
            })
        };
        if self.schema_version != SCHEMA_VERSION {
            return bad("schema_version", "expected 1");
        }
        if !self.cpq13_grounded_checked_at_press_only {
            return bad(
                "cpq13_grounded_checked_at_press_only",
                "the throw clock never depends on ground contact (CP-Q13)",
            );
        }
        // The throw timeline (CP-Q14): strictly increasing phase starts inside
        // the clock, the spawn inside the release phase, the cancel window on
        // the brace.
        let phases = [
            ("cpq14_fuse_lit_tick", self.cpq14_fuse_lit_tick),
            ("cpq14_hold_tick", self.cpq14_hold_tick),
            ("cpq14_wind_tick", self.cpq14_wind_tick),
            ("cpq14_release_tick", self.cpq14_release_tick),
            ("cpq14_follow_tick", self.cpq14_follow_tick),
            ("cpq14_brace_tick", self.cpq14_brace_tick),
            ("cpq14_settle_tick", self.cpq14_settle_tick),
        ];
        let mut previous = 0;
        for (field, tick) in phases {
            if tick <= previous {
                return bad(field, "phase starts must increase from tick 0");
            }
            previous = tick;
        }
        if self.cpq14_settle_tick >= self.cpq14_throw_ticks {
            return bad(
                "cpq14_throw_ticks",
                "must end after the settle phase starts",
            );
        }
        if !(self.cpq14_release_tick..self.cpq14_follow_tick).contains(&self.cpq14_spawn_tick) {
            return bad(
                "cpq14_spawn_tick",
                "the stick spawns inside the release phase",
            );
        }
        if self.cpq14_cancel_from_tick < self.cpq14_brace_tick
            || self.cpq14_cancel_from_tick >= self.cpq14_throw_ticks
        {
            return bad(
                "cpq14_cancel_from_tick",
                "the recovery cancels from the brace on, inside the throw",
            );
        }
        if self.cpq15_jump_ignored_until_tick > self.cpq14_throw_ticks {
            return bad("cpq15_jump_ignored_until_tick", "inside the throw clock");
        }
        if !(self.cpq14_fuse_lit_tick..=self.cpq14_spawn_tick).contains(&self.cpq27_drop_from_tick)
        {
            return bad(
                "cpq27_drop_from_tick",
                "a hit drops the lit stick between the fuse and the spawn",
            );
        }
        if self.cpq16_spawn_dx_px == 0 || self.cpq16_spawn_dy_px == 0 {
            return bad("cpq16_spawn_dx_px", "the spawn offset is positive");
        }
        if self.cpq17_vx_base_px_s == 0 || self.cpq17_vy_px_s == 0 || self.cpq17_gravity_px_s2 == 0
        {
            return bad("cpq17_vx_base_px_s", "the arc needs speed and gravity");
        }
        if self.cpq17_vx_thrower_share_pct > 100 {
            return bad("cpq17_vx_thrower_share_pct", "a percentage");
        }
        if self.cpq18_fuse_ticks == 0 {
            return bad("cpq18_fuse_ticks", "the fuse burns at least one tick");
        }
        if self.cpq19_recharge_ticks == 0 {
            return bad(
                "cpq19_recharge_ticks",
                "the recharge lasts at least one tick",
            );
        }
        if self.cpq19_max_live != 1 {
            return bad("cpq19_max_live", "one stick at a time (CP-Q19)");
        }
        if self.cpq20_core_radius_px == 0 || self.cpq20_core_radius_px >= self.cpq20_outer_radius_px
        {
            return bad(
                "cpq20_core_radius_px",
                "the core is smaller than the outer ring",
            );
        }
        if self.cpq21_outer_damage == 0 {
            return bad("cpq21_outer_damage", "the outer ring hurts");
        }
        if self.cpq21_launch_vx_px_s == 0 || self.cpq21_launch_vy_px_s == 0 {
            return bad("cpq21_launch_vx_px_s", "the outer ring launches");
        }
        if self.cpq23_self_damage == 0 {
            return bad("cpq23_self_damage", "reduced, never zero (CP-Q23)");
        }
        if self.cpq23_self_invulnerable_ticks == 0 {
            return bad(
                "cpq23_self_invulnerable_ticks",
                "a self-hit grants invulnerability",
            );
        }
        if self.cpq25_boss_damage == 0 {
            return bad("cpq25_boss_damage", "the boss takes damage (CP-Q25)");
        }
        if self.cpq26_llama_outer_damage == 0 {
            return bad("cpq26_llama_outer_damage", "the ring hurts a llama");
        }
        if self.cpq32_socket_label.is_empty()
            || self.cpq32_socket_label.chars().count() > crate::story_unlock::NOTICE_MAX_CHARS
            || !self.cpq32_socket_label.is_ascii()
        {
            return bad("cpq32_socket_label", "1..=13 ascii characters");
        }
        if self.cpq34_chain_delay_ticks == 0 {
            return bad("cpq34_chain_delay_ticks", "a chain takes at least one tick");
        }
        if self.cpq34_chain_max_depth == 0 {
            return bad("cpq34_chain_max_depth", "at least depth 1");
        }
        if self.cpq34_chain_max_per_tick == 0 {
            return bad("cpq34_chain_max_per_tick", "at least one detonation a tick");
        }
        let materials = ["wood", "brittle_stone"];
        for (i, row) in self.cpq37_terrain_rows.iter().enumerate() {
            if !materials.contains(&row.material.as_str()) {
                return bad(
                    "cpq37_terrain_rows",
                    "only wood and brittle_stone rows exist",
                );
            }
            if row.enabled {
                return bad(
                    "cpq37_terrain_rows",
                    "no route solid breaks until TERRAIN-L1: rows ship disabled (CP-Q37)",
                );
            }
            if self.cpq37_terrain_rows[..i]
                .iter()
                .any(|prev| prev.material == row.material)
            {
                return bad("cpq37_terrain_rows", "duplicate material");
            }
        }
        // The gauntlet lengthens a stagger (jab 10, haymaker 18 today).
        if self.cpq39_jab_stagger_ticks == 0 || self.cpq39_haymaker_stagger_ticks == 0 {
            return bad("cpq39_jab_stagger_ticks", "a reinforced punch staggers");
        }
        if self.cpq40_boss_poise_multiplier == 0 {
            return bad("cpq40_boss_poise_multiplier", "at least 1");
        }
        if self.cpq41_jab_support_hits == 0 || self.cpq41_haymaker_support_hits == 0 {
            return bad("cpq41_jab_support_hits", "a punch counts at least one hit");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_data_file_is_pinned_valid_and_names_every_knob_by_its_question_id() {
        // Fails when carl_prog_v0.json changes without updating TUNING_HASH.
        assert_eq!(content_hash(TUNING_JSON), TUNING_HASH);
        let tuning = CarlProgTuning::pinned().expect("pinned tuning loads");
        assert_eq!(tuning.cpq18_fuse_ticks, 150);
        assert_eq!(tuning.cpq19_recharge_ticks, 420);
        assert_eq!(tuning.cpq20_outer_radius_px, 64);
        assert_eq!(tuning.cpq34_chain_max_depth, 4);
        assert_eq!(
            (
                tuning.cpq39_jab_stagger_ticks,
                tuning.cpq39_haymaker_stagger_ticks
            ),
            (16, 28)
        );
        let value: serde_json::Value = serde_json::from_str(TUNING_JSON).unwrap();
        let keys: Vec<&String> = value.as_object().unwrap().keys().collect();
        for key in keys {
            let knob = key != "schema_version" && key != "provenance";
            assert!(
                !knob || key.starts_with("cpq"),
                "{key}: a knob name starts with its question ID"
            );
        }
    }

    /// The data contract is IP-neutral: no key of the knob file names a franchise
    /// item or character (the values and the provenance are the fan layer).
    #[test]
    fn no_knob_key_names_a_franchise_item_or_character() {
        let banned = [
            "dynamite", "gauntlet", "carl", "hoarder", "juicer", "donut", "biscuit", "puddle",
        ];
        let value: serde_json::Value = serde_json::from_str(TUNING_JSON).unwrap();
        fn keys(value: &serde_json::Value, out: &mut Vec<String>) {
            match value {
                serde_json::Value::Object(map) => {
                    for (key, inner) in map {
                        out.push(key.clone());
                        keys(inner, out);
                    }
                }
                serde_json::Value::Array(items) => items.iter().for_each(|i| keys(i, out)),
                _ => {}
            }
        }
        let mut all = Vec::new();
        keys(&value, &mut all);
        assert!(all.len() > 40);
        for key in all {
            for word in banned {
                assert!(!key.contains(word), "knob key {key} says {word}");
            }
        }
    }

    /// CP-Q38 to CP-Q42: the reinforcement is the data's, applied to the jab and
    /// the haymaker only; damage, shape, chain and every other move are untouched.
    #[test]
    fn reinforce_maps_the_two_punches_by_data_and_nothing_else() {
        use crate::combat::{CARL_HAYMAKER, CARL_JAB, CARL_PUSH_KICK, cf02_moves};
        let t = CarlProgTuning::active();
        let jab = t.reinforce(&CARL_JAB);
        assert_eq!(
            (jab.reaction.stagger_ticks, jab.pierces_commit),
            (16, false)
        );
        assert_eq!(
            jab.reaction.hit_stop, CARL_JAB.reaction.hit_stop,
            "no jab stop bonus"
        );
        let hay = t.reinforce(&CARL_HAYMAKER);
        assert_eq!((hay.reaction.stagger_ticks, hay.pierces_commit), (28, true));
        assert_eq!(hay.reaction.hit_stop, CARL_HAYMAKER.reaction.hit_stop + 1);
        for (plain, landed) in [(CARL_JAB, jab), (CARL_HAYMAKER, hay)] {
            let restored = MoveSpec {
                reaction: plain.reaction,
                pierces_commit: false,
                ..landed
            };
            assert_eq!(restored, plain, "damage, shape, timing and chain unchanged");
            assert_eq!(landed.damage, plain.damage);
            assert_eq!(landed.reaction.knock_vx, plain.reaction.knock_vx);
            assert_eq!(
                landed.reaction.camera_kick_px,
                plain.reaction.camera_kick_px
            );
        }
        // Everything else is returned as it is (the kicks, the stomp, the air kick,
        // the dive stomp; Donut's moves).
        for spec in [&CARL_PUSH_KICK]
            .into_iter()
            .chain(cf02_moves().all())
            .chain(crate::combat::DONUT_MOVES)
        {
            assert_eq!(t.reinforce(spec), *spec, "{:?}", spec.id);
            assert!(!t.reinforce(spec).pierces_commit);
        }
        // The data decides: a punch the data leaves plain is returned plain.
        let mut off = t.clone();
        off.cpq38_haymaker_reinforced = false;
        assert_eq!(off.reinforce(&CARL_HAYMAKER), CARL_HAYMAKER);
        assert_eq!(off.support_hit_weight(MoveId::Haymaker), 1);
        assert_eq!(off.boss_poise(&CARL_HAYMAKER), 1);
        // F-12: the jab pierce is one data row away.
        let mut jab_pierces = t.clone();
        jab_pierces.cpq40_jab_pierces_commit = true;
        assert!(jab_pierces.reinforce(&CARL_JAB).pierces_commit);
        assert!(!t.reinforce(&CARL_JAB).pierces_commit);
        assert!(t.reinforce(&CARL_HAYMAKER).pierces_commit);
    }

    /// CP-Q40, CP-Q41: the support weight (haymaker 2, jab 1, everything else 1)
    /// and the boss poise (a reinforced punch 2x its damage, other moves 1x).
    #[test]
    fn support_weight_and_boss_poise_follow_the_data() {
        use crate::combat::{CARL_HAYMAKER, CARL_JAB, CARL_PUSH_KICK};
        let t = CarlProgTuning::active();
        assert_eq!(t.support_hit_weight(MoveId::Jab), 1);
        assert_eq!(t.support_hit_weight(MoveId::Haymaker), 2);
        for id in [
            MoveId::PushKick,
            MoveId::Stomp,
            MoveId::AirKick,
            MoveId::DiveStomp,
            MoveId::Swat,
        ] {
            assert_eq!(t.support_hit_weight(id), 1, "{id:?}");
        }
        assert_eq!(t.boss_poise(&CARL_JAB), 2);
        assert_eq!(t.boss_poise(&CARL_HAYMAKER), 2);
        assert_eq!(t.boss_poise(&CARL_PUSH_KICK), 1);
    }

    #[test]
    fn a_drifted_file_fails_the_pin_and_a_bad_field_is_named() {
        let drifted = TUNING_JSON.replace("\"cpq18_fuse_ticks\": 150", "\"cpq18_fuse_ticks\": 151");
        assert_ne!(drifted, TUNING_JSON);
        match CarlProgTuning::from_json_pinned(&drifted, TUNING_HASH) {
            Err(TuningError::HashMismatch { expected, .. }) => assert_eq!(expected, TUNING_HASH),
            other => panic!("a drifted file must fail the pin, got {other:?}"),
        }
        let field = |from: &str, to: &str| {
            let text = TUNING_JSON.replacen(from, to, 1);
            assert_ne!(text, TUNING_JSON, "{from}");
            match CarlProgTuning::from_json(&text) {
                Err(TuningError::Field { field, .. }) => field,
                other => panic!("expected a field error for {to}, got {other:?}"),
            }
        };
        assert_eq!(
            field("\"cpq14_spawn_tick\": 41", "\"cpq14_spawn_tick\": 60"),
            "cpq14_spawn_tick"
        );
        assert_eq!(
            field(
                "\"cpq20_core_radius_px\": 20",
                "\"cpq20_core_radius_px\": 64"
            ),
            "cpq20_core_radius_px"
        );
        assert_eq!(
            field("\"cpq19_max_live\": 1", "\"cpq19_max_live\": 2"),
            "cpq19_max_live"
        );
        assert_eq!(
            field("\"enabled\": false", "\"enabled\": true"),
            "cpq37_terrain_rows"
        );
        assert_eq!(
            field(
                "\"cpq32_socket_label\": \"BOOM\"",
                "\"cpq32_socket_label\": \"\""
            ),
            "cpq32_socket_label"
        );
        assert!(matches!(
            CarlProgTuning::from_json("{\"schema_version\": 1, \"bogus\": 1}"),
            Err(TuningError::Parse(_))
        ));
    }

    #[test]
    fn the_timeline_matches_the_throw_art_contract() {
        let t = CarlProgTuning::active();
        // ANIM-CARL-03: draw t0-5, fuse t6, hold t16-29, wind t30-37, release
        // t38-41, follow t42-49, brace t50-61, settle t62-69.
        assert_eq!(
            [
                t.cpq14_fuse_lit_tick,
                t.cpq14_hold_tick,
                t.cpq14_wind_tick,
                t.cpq14_release_tick,
                t.cpq14_follow_tick,
                t.cpq14_brace_tick,
                t.cpq14_settle_tick,
                t.cpq14_throw_ticks,
            ],
            [6, 16, 30, 38, 42, 50, 62, 70]
        );
        assert_eq!(t.cpq14_spawn_tick, 41);
    }

    #[test]
    fn no_flag_is_no_story_and_each_flag_maps_to_its_setup() {
        for scene in [
            SceneKind::Corridor,
            SceneKind::Demo,
            SceneKind::CfArena,
            SceneKind::Boss,
        ] {
            assert!(CarlProgFlags::OFF.story_setup(scene).is_inert());
        }
        let prog = CarlProgFlags {
            prog: true,
            ..CarlProgFlags::OFF
        };
        assert!(
            prog.story_setup(SceneKind::Demo).is_inert(),
            "no pin on the demo hall"
        );
        assert!(
            prog.story_setup(SceneKind::CfArena).is_inert(),
            "no pin on the arena"
        );
        assert!(!prog.story_setup(SceneKind::Boss).is_inert());
        // The flag applies only Carl's rows of a shared trigger (CP-Q44).
        assert_eq!(
            prog.story_setup(SceneKind::Boss).grant_heroes(),
            Some(&[0u8][..])
        );
        let dynamite = CarlProgFlags {
            dynamite: true,
            ..CarlProgFlags::OFF
        };
        assert_eq!(dynamite.story_setup(SceneKind::CfArena).holds().len(), 1);
        let gauntlet = CarlProgFlags {
            gauntlet: true,
            ..CarlProgFlags::OFF
        };
        assert_eq!(gauntlet.story_setup(SceneKind::Boss).holds().len(), 1);
        assert!(
            gauntlet
                .story_setup(SceneKind::Boss)
                .trigger_ids()
                .is_empty()
        );
    }
}
