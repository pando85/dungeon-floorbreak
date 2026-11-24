//! HRD-A1: the Hoarder as a boss, pure core (`docs/01-design/enemy-gameplay-hoarder.md`,
//! HRD-Q01..HRD-Q41). Bevy-free, deterministic, headless.
//!
//! Engine names are IP-neutral (`pile_boss_f01`, [`PILE_BOSS_ID`]); the fan name
//! lives only in art and display data. This module owns everything that does
//! not need the scene:
//!
//! * [`PileBossTuning`]: every `hrdq##` knob of
//!   `assets/data/gameplay/enemy_pile_boss_v0.json`, loaded through a validator
//!   whose errors name the field, FNV-1a pinned ([`TUNING_HASH`], registered in
//!   `playtrace::live::data_pins` under [`PIN_KEY`]);
//! * [`PileBoss`], the state machine: Dormant, Reveal, Neutral, Walk, Turn, the
//!   swipe (tell, smear, impact, recover), the shove (tell, lunge, recover or
//!   jammed), Flinch, Exposed, Get-up, Cooldown and Defeated, with the body box
//!   per mode, the push-out, the entry gate, the leash, the disengage and its
//!   follow answer (walk-home rule N1), the home hold, the nest walk goal and
//!   nest timer, targeting, and the `BossThreat` feed;
//! * [`obstacles_from_encounter`]: bank A, the J-B post and the closed door of
//!   the sidecar as [`Obstacle`]s.
//!
//! What it does **not** do: read a scene, emit `CombatEvent::PileBoss`, toggle
//! solids, grant anything. HRD-A2's slot module (`combat_scene/pile_boss.rs`)
//! wraps it for the scene. The defeat is the existing
//! `CombatEvent::EnemyDefeated { target: PILE_BOSS_ID }`; the thrown charge (K1)
//! is granted by the story ledger's event trigger [`DEFEAT_TRIGGER_ID`] on that
//! event, and [`PileOut::story_trigger`] is telemetry only, never a grant path.
//!
//! # Tick contract
//!
//! One [`PileBoss::step`] per scene tick, after the heroes' step and after
//! their hits ([`PileBoss::apply_hit`]), so a hero's same-tick hit lands first.
//! Counting convention of the sheet: the tick a state is entered is its tick 1
//! (`mode_tick == 1`); a state of N ticks entered on tick T occupies T..T+N-1
//! and the next state starts on T+N. A swipe or shove tell is **attack tick 0**
//! on the tick it is chosen. Coordinates are world px: x forward is mirrored by
//! the facing, y is up from the walking line ([`PileSetup::floor_y`]).
//!
//! # Adopted readings (recorded, not new design)
//!
//! * The shove's own 180-tick cooldown (HRD-Q14) starts on the tick the lunge
//!   ends (HRD-Q13: "shove ends" is the end of the lunge, Recover or Jammed
//!   follows it), and runs through Recover or Jammed and the 30-tick Cooldown.
//!   A whiff cycle is tell 40 + lunge 48 + 180 = 268 ticks: the next shove tell
//!   starts 268 ticks after the previous one.
//! * The 60 ticks without poise after a Flinch (HRD-Q16) count from the tick
//!   the Flinch starts.
//! * A NestPush resets the nest timer on its first tick (a static bank A would
//!   otherwise be pushed again at once). A prop that slides (HRD-B) removes the
//!   bank from the obstacle list, which ends the rule on its own.
//! * The lunge ends (Recover, Jammed) on the tick of its last lunge step; the
//!   push-out of her standing body still waits for the first tick after.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::boss::{BossThreat, HeroBlow, HeroView};
use crate::cf02::BoxPx;
use crate::combat::{CombatEvent, DefeatCause, MoveSpec};
use crate::donut_mana::content_hash;
use crate::encounter::{Encounter, StaticKind};
use crate::hero_damage::HeroDamagePolicy;
use crate::party::HeroId;
use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Rect};

/// Her target id in `CombatEvent::EnemyDefeated` (HRD-Q31). Disjoint from the
/// Juicer's `0x4255`, the rack's `0x5241` and the Engineer ram stamp space.
pub const PILE_BOSS_ID: u32 = 0x504C;
/// Her source id in `CombatEvent::HeroHurt` (HRD-Q31): her own `HeroBlow`
/// source constant, never the Juicer's.
pub const PILE_BOSS_SOURCE_ID: u32 = 0x504C;
/// The CP-A trigger row her defeat satisfies (K1).
pub const DEFEAT_TRIGGER_ID: &str = "pile_boss_f01_defeated";
/// Who her blows hurt: every hero, whoever baited her (K3).
pub const BLOW_POLICY: HeroDamagePolicy = HeroDamagePolicy::AllHeroes;
/// Her swipe, shove, slide and landing are never evadable (K6, DM-Q18).
pub const EVADE_CLASS: &str = "never";

/// The tuning file, embedded so the binary and the tests read the same bytes.
pub const TUNING_JSON: &str = include_str!("../assets/data/gameplay/enemy_pile_boss_v0.json");
/// The committed FNV-1a 64 content hash of [`TUNING_JSON`].
pub const TUNING_HASH: u64 = 0x73fe_ce44_acbc_b645;
/// The `content_hashes` key of the tuning pin.
pub const PIN_KEY: &str = "pile_boss_tuning";
/// The schema version [`PileBossTuning`] accepts.
pub const SCHEMA_VERSION: u32 = 1;

/// Her room and its sidecar, relative to the repository root.
pub const ROOM_PATH: &str = "assets/levels/floor01/f01_hoarder.room.json";
pub const ENCOUNTER_PATH: &str = "assets/levels/floor01/f01_hoarder.encounter.json";
/// The sidecar ids of bank A and the J-B support post.
pub const BANK_A_ID: &str = "bank_a";
pub const POST_ID: &str = "post_b";

/// The entry gate's width and height in px (a full-height solid: the lane
/// ceiling of her room).
pub const GATE_WIDTH_PX: f32 = 16.0;
pub const GATE_HEIGHT_PX: f32 = 228.0;
/// A remnant at most this high is traversable and ignored by her body.
pub const REMNANT_MAX_PX: f32 = 16.0;

const TICK: f32 = 1.0 / 60.0;
/// She turns toward a target only when it is behind her by more than this.
const TURN_DEADZONE_PX: f32 = 8.0;
/// Her pivot counts as "at" the nest face and at a leash edge within this.
const FACE_TOLERANCE_PX: f32 = 4.0;
const EDGE_TOLERANCE_PX: f32 = 0.01;

/// HRD questions that have no key in the file, and why. The completeness test
/// requires every other ID in 1..=41 to own at least one `hrdqNN_` key.
pub const NO_KNOB: [(u8, &str); 18] = [
    (1, "design: the vocabulary of the sheet"),
    (15, "code: the armour table of PileMode::reaction"),
    (17, "reuses CP-Q25 and CP-Q40 (merged hero damage)"),
    (21, "sidecar: J-A geometry and timing, HRD-B"),
    (22, "sidecar: J-A stimuli, HRD-B"),
    (23, "sidecar: J-A slide push, HRD-B"),
    (25, "sidecar: J-B geometry, HRD-C"),
    (26, "sidecar: J-B stimuli, HRD-C"),
    (27, "sidecar: J-B timing, HRD-C"),
    (28, "sidecar: J-B hero blow, HRD-C"),
    (29, "room and sidecar files"),
    (30, "validator rule (K8) of src/encounter.rs"),
    (31, "constants: PILE_BOSS_ID and PILE_BOSS_SOURCE_ID"),
    (32, "death profile data, HRD-A2"),
    (34, "presentation: stand-in frames"),
    (35, "no new audio asset"),
    (39, "code: bounded hit tokens per blow"),
    (40, "code: PileEvent, wrapped by HRD-A2"),
];

// ------------------------------------------------------------------ tuning --

/// A knob file problem. A load error names the field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PileBossError {
    Parse(String),
    Field { field: &'static str, reason: String },
    HashMismatch { expected: u64, actual: u64 },
}

impl core::fmt::Display for PileBossError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "pile boss tuning: {msg}"),
            Self::Field { field, reason } => {
                write!(f, "pile boss tuning: field `{field}`: {reason}")
            }
            Self::HashMismatch { expected, actual } => write!(
                f,
                "pile boss tuning hash {actual:#018x} is not the pinned {expected:#018x}"
            ),
        }
    }
}

impl std::error::Error for PileBossError {}

/// Every HRD knob (`docs/01-design/enemy-gameplay-hoarder.md`, section 12).
/// Boxes are pivot-relative: x forward, y up from the feet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PileBossTuning {
    pub schema_version: u32,
    /// Q02: health.
    pub hrdq02_hp: u16,
    /// Q03: standing body, head, prone body, push-out speed and side room.
    pub hrdq03_body: BoxPx,
    pub hrdq03_head: BoxPx,
    pub hrdq03_prone: BoxPx,
    pub hrdq03_push_px: u16,
    pub hrdq03_side_room_px: u16,
    /// Q04: walk speed, turn ticks, walk standoff.
    pub hrdq04_walk_px_s: u16,
    pub hrdq04_turn_ticks: u16,
    pub hrdq04_standoff_px: u16,
    /// Q05: engage line, reveal ticks (first entry, retry), stir cue.
    pub hrdq05_engage_x: u16,
    pub hrdq05_reveal_ticks: u16,
    pub hrdq05_retry_reveal_ticks: u16,
    pub hrdq05_stir_x: u16,
    /// Q06: eligible feet height, target hysteresis, solo bot bias.
    pub hrdq06_eligible_px: u16,
    pub hrdq06_switch_px: u16,
    pub hrdq06_ai_bias_px: u16,
    /// Q07: the swipe trigger distance.
    pub hrdq07_trigger_px: u16,
    /// Q08: swipe timing.
    pub hrdq08_tell_gather_ticks: u16,
    pub hrdq08_tell_hold_ticks: u16,
    pub hrdq08_smear_ticks: u16,
    pub hrdq08_impact_ticks: u16,
    pub hrdq08_recover_ticks: u16,
    pub hrdq08_cooldown_ticks: u16,
    /// Q09: swipe box, damage and knock.
    pub hrdq09_box: BoxPx,
    pub hrdq09_damage: u8,
    pub hrdq09_knock_x: u16,
    pub hrdq09_knock_y: u16,
    /// Q10: the shove trigger range.
    pub hrdq10_range_min_px: u16,
    pub hrdq10_range_max_px: u16,
    /// Q11: shove timing and speed.
    pub hrdq11_tell_ticks: u16,
    pub hrdq11_px_per_tick: u16,
    pub hrdq11_max_ticks: u16,
    /// Q12: shove box, damage and knock.
    pub hrdq12_box: BoxPx,
    pub hrdq12_damage: u8,
    pub hrdq12_knock_x: u16,
    pub hrdq12_knock_y: u16,
    /// Q13: shove ends.
    pub hrdq13_recover_ticks: u16,
    pub hrdq13_jam_ticks: u16,
    /// Q14: the shove's own cooldown.
    pub hrdq14_cooldown_ticks: u16,
    /// Q16: poise.
    pub hrdq16_poise: u16,
    pub hrdq16_window: u16,
    pub hrdq16_flinch_ticks: u16,
    pub hrdq16_immune_ticks: u16,
    /// Q18: exposure.
    pub hrdq18_damage: u16,
    pub hrdq18_floor_hp: u16,
    pub hrdq18_exposed_ticks: u16,
    pub hrdq18_getup_ticks: u16,
    pub hrdq18_hurtbox: BoxPx,
    /// Q19: leash, disengage and return.
    pub hrdq19_leash_min: u16,
    pub hrdq19_leash_max: u16,
    pub hrdq19_unreachable_ticks: u16,
    pub hrdq19_home_x: u16,
    pub hrdq19_return_ticks: u16,
    pub hrdq19_return_px: u16,
    /// Q20: the evasion class (`never` only).
    pub hrdq20_evade_class: String,
    /// Q24: the nest rule (0 ticks = off) and the face pivot.
    pub hrdq24_nest_ticks: u16,
    pub hrdq24_face_x: u16,
    /// Q33: the Pound dodge anchor and the AI jump lead.
    pub hrdq33_pound_anchor_px: u16,
    pub hrdq33_ai_jump_lead: u16,
    /// Q36: no boss bar in v1.
    pub hrdq36_boss_bar: bool,
    /// Q37: the data kill switch.
    pub hrdq37_enabled: bool,
    /// Q38: the fairness floors.
    pub hrdq38_min_tell: u16,
    pub hrdq38_min_punish: u16,
    pub hrdq38_min_warning: u16,
    /// Q41: the entry gate.
    pub hrdq41_gate_enabled: bool,
    pub hrdq41_gate_x: u16,
    pub hrdq41_inside_x: u16,
    pub hrdq41_settle_ticks: u16,
    pub hrdq41_regroup_ticks: u16,
    pub hrdq41_regroup_x: u16,
}

impl PileBossTuning {
    /// Parse and validate.
    pub fn from_json(text: &str) -> Result<Self, PileBossError> {
        let tuning: Self =
            serde_json::from_str(text).map_err(|err| PileBossError::Parse(err.to_string()))?;
        tuning.validate()?;
        Ok(tuning)
    }

    /// Parse the embedded file and require the pinned content hash.
    pub fn pinned() -> Result<Self, PileBossError> {
        Self::from_json_pinned(TUNING_JSON, TUNING_HASH)
    }

    /// Parse `text` and require its content hash to equal `expected`.
    pub fn from_json_pinned(text: &str, expected: u64) -> Result<Self, PileBossError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(PileBossError::HashMismatch { expected, actual });
        }
        Self::from_json(text)
    }

    /// The pinned tuning every simulation reads, shared as a static so a
    /// [`PileBoss`] can hold a reference.
    pub fn active() -> &'static Self {
        static ACTIVE: OnceLock<PileBossTuning> = OnceLock::new();
        ACTIVE.get_or_init(|| {
            Self::pinned().expect("the embedded pile boss data is pinned and valid")
        })
    }

    /// Whether she exists at all (the data kill switch).
    pub fn enabled(&self) -> bool {
        self.hrdq37_enabled
    }

    /// The swipe tell in ticks (gather plus rear-up hold).
    pub fn swipe_tell_ticks(&self) -> u16 {
        self.hrdq08_tell_gather_ticks + self.hrdq08_tell_hold_ticks
    }

    pub fn validate(&self) -> Result<(), PileBossError> {
        fn bad(field: &'static str, reason: &str) -> Result<(), PileBossError> {
            Err(PileBossError::Field {
                field,
                reason: reason.to_string(),
            })
        }
        fn positive(field: &'static str, value: u32) -> Result<(), PileBossError> {
            if value == 0 {
                return bad(field, "must be positive");
            }
            Ok(())
        }
        fn boxed(field: &'static str, b: &BoxPx) -> Result<(), PileBossError> {
            if b.x0 >= b.x1 || b.y0 >= b.y1 {
                return bad(field, "needs x0 < x1 and y0 < y1");
            }
            Ok(())
        }
        if self.schema_version != SCHEMA_VERSION {
            return bad("schema_version", "expected 1");
        }
        positive("hrdq02_hp", u32::from(self.hrdq02_hp))?;
        boxed("hrdq03_body", &self.hrdq03_body)?;
        boxed("hrdq03_head", &self.hrdq03_head)?;
        boxed("hrdq03_prone", &self.hrdq03_prone)?;
        boxed("hrdq09_box", &self.hrdq09_box)?;
        boxed("hrdq12_box", &self.hrdq12_box)?;
        boxed("hrdq18_hurtbox", &self.hrdq18_hurtbox)?;
        if self.hrdq03_body.x0 >= 0 || self.hrdq03_body.x1 <= 0 {
            return bad("hrdq03_body", "must straddle the pivot (x0 < 0 < x1)");
        }
        positive("hrdq03_push_px", u32::from(self.hrdq03_push_px))?;
        positive("hrdq04_walk_px_s", u32::from(self.hrdq04_walk_px_s))?;
        positive("hrdq04_turn_ticks", u32::from(self.hrdq04_turn_ticks))?;
        positive("hrdq05_reveal_ticks", u32::from(self.hrdq05_reveal_ticks))?;
        positive(
            "hrdq05_retry_reveal_ticks",
            u32::from(self.hrdq05_retry_reveal_ticks),
        )?;
        positive("hrdq06_eligible_px", u32::from(self.hrdq06_eligible_px))?;
        positive("hrdq07_trigger_px", u32::from(self.hrdq07_trigger_px))?;
        if self.hrdq04_standoff_px > self.hrdq07_trigger_px {
            return bad(
                "hrdq04_standoff_px",
                "must not exceed hrdq07_trigger_px (she swipes before she stops)",
            );
        }
        positive(
            "hrdq08_tell_gather_ticks",
            u32::from(self.hrdq08_tell_gather_ticks),
        )?;
        positive(
            "hrdq08_tell_hold_ticks",
            u32::from(self.hrdq08_tell_hold_ticks),
        )?;
        positive("hrdq08_smear_ticks", u32::from(self.hrdq08_smear_ticks))?;
        positive("hrdq08_impact_ticks", u32::from(self.hrdq08_impact_ticks))?;
        positive("hrdq08_recover_ticks", u32::from(self.hrdq08_recover_ticks))?;
        positive(
            "hrdq08_cooldown_ticks",
            u32::from(self.hrdq08_cooldown_ticks),
        )?;
        positive("hrdq09_damage", u32::from(self.hrdq09_damage))?;
        positive("hrdq10_range_min_px", u32::from(self.hrdq10_range_min_px))?;
        if self.hrdq10_range_min_px <= self.hrdq07_trigger_px {
            return bad(
                "hrdq10_range_min_px",
                "must exceed hrdq07_trigger_px (the shove is a middle-range attack)",
            );
        }
        if self.hrdq10_range_max_px < self.hrdq10_range_min_px {
            return bad(
                "hrdq10_range_max_px",
                "must not be below hrdq10_range_min_px",
            );
        }
        positive("hrdq11_tell_ticks", u32::from(self.hrdq11_tell_ticks))?;
        positive("hrdq11_px_per_tick", u32::from(self.hrdq11_px_per_tick))?;
        positive("hrdq11_max_ticks", u32::from(self.hrdq11_max_ticks))?;
        positive("hrdq12_damage", u32::from(self.hrdq12_damage))?;
        positive("hrdq13_recover_ticks", u32::from(self.hrdq13_recover_ticks))?;
        positive("hrdq13_jam_ticks", u32::from(self.hrdq13_jam_ticks))?;
        positive("hrdq16_poise", u32::from(self.hrdq16_poise))?;
        positive("hrdq16_window", u32::from(self.hrdq16_window))?;
        positive("hrdq16_flinch_ticks", u32::from(self.hrdq16_flinch_ticks))?;
        positive("hrdq18_damage", u32::from(self.hrdq18_damage))?;
        positive("hrdq18_floor_hp", u32::from(self.hrdq18_floor_hp))?;
        if self.hrdq18_floor_hp >= self.hrdq02_hp {
            return bad("hrdq18_floor_hp", "must be below hrdq02_hp");
        }
        positive("hrdq18_exposed_ticks", u32::from(self.hrdq18_exposed_ticks))?;
        positive("hrdq18_getup_ticks", u32::from(self.hrdq18_getup_ticks))?;
        if self.hrdq19_leash_min >= self.hrdq19_leash_max {
            return bad("hrdq19_leash_min", "must be below hrdq19_leash_max");
        }
        if self.hrdq19_home_x <= self.hrdq19_leash_min
            || self.hrdq19_home_x >= self.hrdq19_leash_max
        {
            return bad("hrdq19_home_x", "must lie strictly inside the leash");
        }
        positive(
            "hrdq19_unreachable_ticks",
            u32::from(self.hrdq19_unreachable_ticks),
        )?;
        positive("hrdq19_return_ticks", u32::from(self.hrdq19_return_ticks))?;
        positive("hrdq19_return_px", u32::from(self.hrdq19_return_px))?;
        if self.hrdq20_evade_class != EVADE_CLASS {
            return bad(
                "hrdq20_evade_class",
                "only `never` is valid (K6, DM-Q18: her blows are never evadable)",
            );
        }
        if self.hrdq36_boss_bar {
            return bad("hrdq36_boss_bar", "no boss bar in v1 (HRD-Q36)");
        }
        positive("hrdq24_face_x", u32::from(self.hrdq24_face_x))?;
        if self.hrdq41_gate_x >= self.hrdq41_inside_x {
            return bad("hrdq41_gate_x", "must be left of hrdq41_inside_x");
        }
        if self.hrdq41_inside_x > self.hrdq05_engage_x {
            return bad("hrdq41_inside_x", "must not exceed hrdq05_engage_x");
        }
        if self.hrdq41_regroup_x < self.hrdq41_inside_x {
            return bad("hrdq41_regroup_x", "must be inside hrdq41_inside_x");
        }
        positive("hrdq41_regroup_ticks", u32::from(self.hrdq41_regroup_ticks))?;
        // HRD-Q38: the fairness floors, checked on load.
        positive("hrdq38_min_tell", u32::from(self.hrdq38_min_tell))?;
        positive("hrdq38_min_punish", u32::from(self.hrdq38_min_punish))?;
        positive("hrdq38_min_warning", u32::from(self.hrdq38_min_warning))?;
        if self.swipe_tell_ticks() < self.hrdq38_min_tell {
            return bad(
                "hrdq08_tell_hold_ticks",
                "the swipe tell is below the fairness floor hrdq38_min_tell",
            );
        }
        if self.hrdq11_tell_ticks < self.hrdq38_min_tell {
            return bad(
                "hrdq11_tell_ticks",
                "the shove tell is below the fairness floor hrdq38_min_tell",
            );
        }
        for (field, ticks) in [
            ("hrdq08_recover_ticks", self.hrdq08_recover_ticks),
            ("hrdq13_recover_ticks", self.hrdq13_recover_ticks),
            ("hrdq13_jam_ticks", self.hrdq13_jam_ticks),
            ("hrdq18_exposed_ticks", self.hrdq18_exposed_ticks),
        ] {
            if ticks < self.hrdq38_min_punish {
                return bad(
                    field,
                    "the punish window is below the fairness floor hrdq38_min_punish",
                );
            }
        }
        Ok(())
    }
}

// ------------------------------------------------------------------- types --

/// Her states (design section 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PileMode {
    Dormant,
    Reveal,
    Neutral,
    Walk,
    Turn,
    SwipeTell,
    SwipeSmear,
    SwipeImpact,
    SwipeRecover,
    ShoveTell,
    Lunge,
    ShoveRecover,
    Jammed,
    Flinch,
    Exposed,
    GetUp,
    Cooldown,
    Defeated,
}

/// How a hit lands in a mode (design section 4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitReaction {
    /// She has no hurtbox.
    None,
    /// Damage and hit-stop, never an interrupt, no poise.
    Armour,
    /// Free damage, no poise.
    Free,
    /// Damage, hit-stop and poise.
    Poise,
}

impl PileMode {
    pub const ALL: [Self; 18] = [
        Self::Dormant,
        Self::Reveal,
        Self::Neutral,
        Self::Walk,
        Self::Turn,
        Self::SwipeTell,
        Self::SwipeSmear,
        Self::SwipeImpact,
        Self::SwipeRecover,
        Self::ShoveTell,
        Self::Lunge,
        Self::ShoveRecover,
        Self::Jammed,
        Self::Flinch,
        Self::Exposed,
        Self::GetUp,
        Self::Cooldown,
        Self::Defeated,
    ];

    /// HRD-Q15 and section 4.3: the armour table.
    pub const fn reaction(self) -> HitReaction {
        match self {
            Self::Dormant | Self::Reveal | Self::Defeated => HitReaction::None,
            Self::SwipeTell
            | Self::SwipeSmear
            | Self::SwipeImpact
            | Self::ShoveTell
            | Self::Lunge => HitReaction::Armour,
            Self::SwipeRecover
            | Self::ShoveRecover
            | Self::Jammed
            | Self::Flinch
            | Self::Exposed
            | Self::GetUp => HitReaction::Free,
            Self::Neutral | Self::Walk | Self::Turn | Self::Cooldown => HitReaction::Poise,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Dormant => "dormant",
            Self::Reveal => "reveal",
            Self::Neutral => "neutral",
            Self::Walk => "walk",
            Self::Turn => "turn",
            Self::SwipeTell => "swipe_tell",
            Self::SwipeSmear => "swipe_smear",
            Self::SwipeImpact => "swipe_impact",
            Self::SwipeRecover => "swipe_recover",
            Self::ShoveTell => "shove_tell",
            Self::Lunge => "lunge",
            Self::ShoveRecover => "shove_recover",
            Self::Jammed => "jammed",
            Self::Flinch => "flinch",
            Self::Exposed => "exposed",
            Self::GetUp => "get_up",
            Self::Cooldown => "cooldown",
            Self::Defeated => "defeated",
        }
    }

    /// Stable code for state hashes.
    pub const fn code(self) -> u64 {
        self as u64
    }
}

/// What a sidecar rectangle is to her.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObstacleKind {
    /// Bank A while `stable` or `disturbed`: blocks her walk and her shove,
    /// and heroes.
    BankA,
    /// The J-B support post: a plain obstacle for the shove, no solid for
    /// walkers.
    Post,
    /// Any other intact solid (the door, a closed gate, a wall).
    Solid,
    /// A remnant of 16 px or less: traversable, ignored by her body and her
    /// shove, solid under a hero's feet.
    Remnant,
}

/// One rectangle the boss reads (world px, y up).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obstacle {
    pub kind: ObstacleKind,
    pub rect: Rect,
}

impl Obstacle {
    /// Whether a hero's body is stopped by it.
    pub const fn blocks_heroes(&self) -> bool {
        !matches!(self.kind, ObstacleKind::Post)
    }

    /// Whether her walking body is stopped by it.
    pub const fn blocks_her_body(&self) -> bool {
        matches!(self.kind, ObstacleKind::BankA | ObstacleKind::Solid)
    }

    /// Whether her shove stops on its face.
    pub const fn stops_shove(&self) -> bool {
        !matches!(self.kind, ObstacleKind::Remnant)
    }
}

/// The two attacks (HRD-Q01).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PileAttack {
    Swipe,
    Shove,
}

/// The non-defeat events of HRD-Q40, local to this module until HRD-A2 wraps
/// them in `CombatEvent::PileBoss { tick, event }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PileEvent {
    Engaged,
    Tell {
        attack: PileAttack,
        target: HeroId,
    },
    Strike {
        attack: PileAttack,
    },
    Hurt {
        source: Option<u32>,
        damage: u8,
        hp_left: u16,
    },
    Flinch,
    Jammed {
        obstacle: ObstacleKind,
    },
    Exposed {
        credit: Option<u32>,
    },
    GetUp,
    NestPush,
    Disengage,
}

impl PileAttack {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Swipe => "swipe",
            Self::Shove => "shove",
        }
    }
}

impl ObstacleKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::BankA => "bank_a",
            Self::Post => "post",
            Self::Solid => "solid",
            Self::Remnant => "remnant",
        }
    }
}

impl PileEvent {
    /// The journal name of the event (`CombatEvent::PileBoss`, HRD-Q40).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Engaged => "engaged",
            Self::Tell { .. } => "tell",
            Self::Strike { .. } => "strike",
            Self::Hurt { .. } => "hurt",
            Self::Flinch => "flinch",
            Self::Jammed { .. } => "jammed",
            Self::Exposed { .. } => "exposed",
            Self::GetUp => "get_up",
            Self::NestPush => "nest_push",
            Self::Disengage => "disengage",
        }
    }

    /// Fold the event into a state hash: a variant code, then its fields.
    pub fn mix_into(self, mix: &mut impl FnMut(u64)) {
        let credit = |c: Option<u32>| c.map_or(0, |p| u64::from(p) + 1);
        match self {
            Self::Engaged => mix(1),
            Self::Tell { attack, target } => {
                mix(2);
                mix(attack as u64);
                mix(target.index() as u64);
            }
            Self::Strike { attack } => {
                mix(3);
                mix(attack as u64);
            }
            Self::Hurt {
                source,
                damage,
                hp_left,
            } => {
                mix(4);
                mix(credit(source));
                mix(u64::from(damage));
                mix(u64::from(hp_left));
            }
            Self::Flinch => mix(5),
            Self::Jammed { obstacle } => {
                mix(6);
                mix(obstacle as u64);
            }
            Self::Exposed { credit: c } => {
                mix(7);
                mix(credit(c));
            }
            Self::GetUp => mix(8),
            Self::NestPush => mix(9),
            Self::Disengage => mix(10),
        }
    }
}

/// A blow she lands on a hero, with the policy and class it carries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PileBlow {
    pub blow: HeroBlow,
    pub attack: PileAttack,
    pub policy: HeroDamagePolicy,
    /// Always false (HRD-Q20, K6).
    pub evadable: bool,
}

/// A shove that met an obstacle: HRD-B turns it into the prop stimulus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JamNotice {
    pub tick: u64,
    pub obstacle: ObstacleKind,
    /// The player of the hero she targeted, `None` for a bot hero.
    pub credit: Option<u32>,
}

/// What one step (or hit) asks the scene to do.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PileOut {
    /// Existing combat events only: `EnemyDefeated { target: PILE_BOSS_ID }`.
    pub events: Vec<CombatEvent>,
    /// The non-defeat events, with their tick.
    pub pile_events: Vec<(u64, PileEvent)>,
    pub blows: Vec<PileBlow>,
    /// Corrected hero x (the push-out).
    pub shifts: Vec<(HeroId, f32)>,
    /// Solo: put this bot hero at `hrdq41_regroup_x` before the gate closes.
    pub regroup: Option<HeroId>,
    pub jams: Vec<JamNotice>,
    /// On the defeat tick: the CP-A trigger row to satisfy (K1).
    pub story_trigger: Option<&'static str>,
}

/// What the scene tells her each tick.
#[derive(Debug, Clone, Copy)]
pub struct PileCtx<'a> {
    pub heroes: [HeroView; 2],
    pub obstacles: &'a [Obstacle],
    /// Solo: the bot hero counts `hrdq06_ai_bias_px` farther away.
    pub solo: bool,
    /// The scene tick of this step (attempt-local).
    pub tick: u64,
    /// HRD-D (review of #260): bank A still stands but is already `disturbed`, so it slides on its
    /// own. It still blocks and is still her nest walk goal, but the nest push needs it `stable`
    /// (design section 5): a push at a disturbed bank would be a spent shove. `false` when bank A
    /// is stable or gone.
    pub bank_a_disturbed: bool,
}

/// The room facts she needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PileSetup {
    /// World y of the walking line.
    pub floor_y: f32,
    /// The room's walkable x range (between the wall columns).
    pub walls: (f32, f32),
}

/// One landed hero hit on her.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PileHit {
    pub damage: u16,
    /// Counts toward poise (Donut's swat counts 1 with 0 damage).
    pub poise: u16,
    pub player: u32,
    pub hit_stop: u8,
    pub cause: DefeatCause,
}

/// The result of [`PileBoss::apply_hit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitResult {
    pub hp_left: u16,
    pub defeated: bool,
    pub flinched: bool,
}

/// The entry gate (design section 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    /// Open; `crossed` is the clock of the tick a hero reached the engage line.
    Open {
        crossed: Option<u64>,
    },
    Closed,
    /// Open for good: her defeat.
    Released,
}

/// Why she is walking home.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Home {
    None,
    /// The disengage walk; `edge` is the leash edge she left (-1 min, +1 max).
    Disengage {
        edge: i8,
    },
    /// The 240-tick return.
    Return,
    /// At home, holding.
    Hold,
}

/// What the home rule decided for this Neutral tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HomeRule {
    /// Ordinary choices, walking included.
    Continue,
    /// Ordinary choices without a walk (the home hold).
    NoWalk,
    /// The tick is spent (walking home).
    Spent,
}

// -------------------------------------------------------------------- boss --

/// The Hoarder's state. `Clone` so a fixture can fork a run.
#[derive(Debug, Clone)]
pub struct PileBoss {
    tun: &'static PileBossTuning,
    setup: PileSetup,
    retry: bool,
    clock: u64,
    /// The clock when the last shove tell began (`BossThreat::Charge::started`).
    /// Not mixed into the hash: a copy of the hashed `clock` taken at a hashed
    /// mode change.
    shove_started: u64,
    hp: u16,
    mode: PileMode,
    mode_tick: u32,
    x: f32,
    facing: i8,
    gate: Gate,
    target: Option<HeroId>,
    target_eligible: bool,
    hit_stop: u8,
    poise: Vec<(u64, u16)>,
    poise_immune_until: u64,
    shove_cd: u32,
    strike_hit: [bool; 2],
    nest_push: bool,
    credit: Option<u32>,
    nest_ticks: u32,
    unreachable_ticks: u32,
    away_ticks: u32,
    disengage_pending: bool,
    /// The leash edge she was stuck at when the disengage was set (-1, +1).
    disengage_edge: i8,
    home: Home,
    /// Transient: the step began in the lunge, so her body pushes nobody yet.
    suppress_push: bool,
}

fn rect(left: f32, bottom: f32, width: f32, height: f32) -> Rect {
    Rect {
        left,
        bottom,
        width,
        height,
    }
}

/// The hero's body for a collision probe: half a pixel inside top and bottom,
/// so a hero standing on a floor is not "inside" it.
fn hero_rect(x: f32, foot_y: f32) -> Rect {
    rect(
        x - HERO_HALF_WIDTH,
        foot_y + 0.5,
        HERO_HALF_WIDTH * 2.0,
        HERO_HALF_HEIGHT * 2.0 - 1.0,
    )
}

fn sign(value: f32) -> i8 {
    if value >= 0.0 { 1 } else { -1 }
}

impl PileBoss {
    /// A dormant boss at her home, facing -x, gate open.
    pub fn new(tun: &'static PileBossTuning, setup: PileSetup) -> Self {
        Self {
            tun,
            setup,
            retry: false,
            clock: 0,
            shove_started: 0,
            hp: tun.hrdq02_hp,
            mode: PileMode::Dormant,
            mode_tick: 0,
            x: f32::from(tun.hrdq19_home_x),
            facing: -1,
            gate: Gate::Open { crossed: None },
            target: None,
            target_eligible: false,
            hit_stop: 0,
            poise: Vec::new(),
            poise_immune_until: 0,
            shove_cd: 0,
            strike_hit: [false; 2],
            nest_push: false,
            credit: None,
            nest_ticks: 0,
            unreachable_ticks: 0,
            away_ticks: 0,
            disengage_pending: false,
            disengage_edge: -1,
            home: Home::None,
            suppress_push: false,
        }
    }

    /// The fixture hook of the sheet: awake in Neutral at pivot `x`, facing
    /// `facing`, the gate closed, piles in their initial states.
    pub fn awake_at(tun: &'static PileBossTuning, setup: PileSetup, x: f32, facing: i8) -> Self {
        let mut boss = Self::new(tun, setup);
        boss.x = x;
        boss.facing = facing;
        boss.gate = Gate::Closed;
        boss.mode = PileMode::Neutral;
        boss.mode_tick = 0;
        boss
    }

    /// A retry or a checkpoint restart: a fresh attempt. The gate is open
    /// again and the reveal is the short one.
    pub fn restart(&mut self) {
        let (tun, setup) = (self.tun, self.setup);
        *self = Self::new(tun, setup);
        self.retry = true;
    }

    /// HRD-Q14: the shove may be chosen again 180 full ticks after the tick the
    /// lunge ended. The step decrements at its start, so the end tick itself is
    /// armed one higher and does not count.
    fn shove_cooldown_after_lunge(tun: &PileBossTuning) -> u32 {
        u32::from(tun.hrdq14_cooldown_ticks) + 1
    }

    /// Fixture hook: preset the shove's own cooldown.
    pub fn with_shove_cooldown(mut self, ticks: u32) -> Self {
        self.shove_cd = ticks;
        self
    }

    /// Fixture hook (HRD-A2): preset her hit points (`hrd_defeat_grants_dynamite`
    /// starts her at 1 HP). At least 1: a fixture never starts her defeated.
    pub fn with_hp(mut self, hp: u16) -> Self {
        self.hp = hp.max(1);
        self
    }

    // ---- accessors

    pub fn tuning(&self) -> &'static PileBossTuning {
        self.tun
    }

    /// The room facts she was built with.
    pub fn setup(&self) -> PileSetup {
        self.setup
    }

    pub fn hp(&self) -> u16 {
        self.hp
    }

    pub fn mode(&self) -> PileMode {
        self.mode
    }

    /// "She is out" (HOARD-02 E2): neither Dormant nor in the Reveal, so a hero hit can reach her
    /// from here on (her hurtbox appears on the first Neutral tick). HRD-C reads it: the J-B post
    /// takes a qualifying hit only once she is out (HRDC-AD4).
    pub fn is_out(&self) -> bool {
        !matches!(self.mode, PileMode::Dormant | PileMode::Reveal)
    }

    /// Ticks in the current mode (1 on the tick it was entered).
    pub fn mode_tick(&self) -> u32 {
        self.mode_tick
    }

    pub fn x(&self) -> f32 {
        self.x
    }

    pub fn facing(&self) -> i8 {
        self.facing
    }

    pub fn target(&self) -> Option<HeroId> {
        self.target
    }

    pub fn gate(&self) -> Gate {
        self.gate
    }

    pub fn nest_ticks(&self) -> u32 {
        self.nest_ticks
    }

    pub fn shove_cooldown(&self) -> u32 {
        self.shove_cd
    }

    pub fn alive(&self) -> bool {
        self.mode != PileMode::Defeated
    }

    /// Whether she is on the way home (the disengage walk or the return).
    pub fn walking_home(&self) -> bool {
        matches!(self.home, Home::Disengage { .. } | Home::Return)
    }

    /// Whether she is holding at home.
    pub fn home_hold(&self) -> bool {
        self.home == Home::Hold
    }

    /// The exit door opens on her defeat tick.
    pub fn door_open(&self) -> bool {
        self.mode == PileMode::Defeated
    }

    /// The entry gate as a solid, while it is closed (and the lever is on).
    pub fn gate_solid(&self) -> Option<Rect> {
        (self.gate == Gate::Closed && self.tun.hrdq41_gate_enabled).then(|| {
            rect(
                f32::from(self.tun.hrdq41_gate_x),
                self.setup.floor_y,
                GATE_WIDTH_PX,
                GATE_HEIGHT_PX,
            )
        })
    }

    /// How many ticks the current attempt's Reveal lasts (the first or the retry value); read by the
    /// presentation, which cuts the emerge into thirds of it.
    pub fn reveal_ticks(&self) -> u32 {
        u32::from(if self.retry {
            self.tun.hrdq05_retry_reveal_ticks
        } else {
            self.tun.hrdq05_reveal_ticks
        })
    }

    // ---- geometry

    /// A pivot-relative box in world px for the facing `facing` at pivot `x`.
    fn local_box_at(&self, b: &BoxPx, x: f32, facing: i8) -> Rect {
        let (x0, x1) = (f32::from(b.x0), f32::from(b.x1));
        let left = if facing >= 0 { x + x0 } else { x - x1 };
        rect(
            left,
            self.setup.floor_y + f32::from(b.y0),
            x1 - x0,
            f32::from(b.y1 - b.y0),
        )
    }

    fn local_box(&self, b: &BoxPx) -> Rect {
        self.local_box_at(b, self.x, self.facing)
    }

    /// Her standing body box, whatever her mode (`None` only when she has no
    /// body at all: Dormant, Reveal, Defeated).
    pub fn standing_box(&self) -> Option<Rect> {
        match self.mode {
            PileMode::Dormant | PileMode::Reveal | PileMode::Defeated => None,
            _ => Some(self.local_box(&self.tun.hrdq03_body)),
        }
    }

    /// Her collision body per mode (design section 4.0).
    pub fn body_rect(&self) -> Option<Rect> {
        match self.mode {
            PileMode::Dormant | PileMode::Defeated | PileMode::Lunge => None,
            PileMode::Reveal => (self.mode_tick >= self.reveal_ticks())
                .then(|| self.local_box(&self.tun.hrdq03_body)),
            PileMode::Exposed | PileMode::GetUp => Some(self.local_box(&self.tun.hrdq03_prone)),
            _ => Some(self.local_box(&self.tun.hrdq03_body)),
        }
    }

    /// Her hurtboxes per mode: the standing body plus the head, the prone box
    /// while Exposed and in Get-up, none while Dormant, in Reveal or Defeated.
    pub fn hurt_rects(&self) -> Vec<Rect> {
        match self.mode {
            PileMode::Dormant | PileMode::Reveal | PileMode::Defeated => Vec::new(),
            PileMode::Exposed | PileMode::GetUp => {
                vec![self.local_box(&self.tun.hrdq18_hurtbox)]
            }
            _ => vec![
                self.local_box(&self.tun.hrdq03_body),
                self.local_box(&self.tun.hrdq03_head),
            ],
        }
    }

    pub fn hurt_overlaps(&self, other: Rect) -> bool {
        self.hurt_rects().iter().any(|r| r.overlaps(other))
    }

    /// The swipe's slam box at its pivot and facing.
    pub fn swipe_box(&self) -> Rect {
        self.local_box(&self.tun.hrdq09_box)
    }

    /// The shove's box at her current pivot.
    pub fn shove_box(&self) -> Rect {
        self.local_box(&self.tun.hrdq12_box)
    }

    // ---- hits

    /// A hero's hit. Hits deal damage and hit-stop but never interrupt a tell,
    /// the swipe or the lunge. Hits in Neutral, Walk, Turn and Cooldown add to
    /// poise. Returns `None` when she has no hurtbox. A lethal hit emits the
    /// existing `EnemyDefeated` event and the K1 story trigger.
    pub fn apply_hit(
        &mut self,
        hit: PileHit,
        lethal_move: Option<&MoveSpec>,
        tick: u64,
        out: &mut PileOut,
    ) -> Option<HitResult> {
        let reaction = self.mode.reaction();
        if reaction == HitReaction::None {
            return None;
        }
        self.hp = self.hp.saturating_sub(hit.damage);
        if hit.damage > 0 {
            out.pile_events.push((
                tick,
                PileEvent::Hurt {
                    source: Some(hit.player),
                    damage: u8::try_from(hit.damage).unwrap_or(u8::MAX),
                    hp_left: self.hp,
                },
            ));
        }
        self.hit_stop = self.hit_stop.max(hit.hit_stop);
        if self.hp == 0 {
            self.defeat();
            out.events.push(CombatEvent::enemy_defeated(
                tick,
                PILE_BOSS_ID,
                hit.cause,
                Some(hit.player),
                lethal_move,
            ));
            out.story_trigger = Some(DEFEAT_TRIGGER_ID);
            return Some(HitResult {
                hp_left: 0,
                defeated: true,
                flinched: false,
            });
        }
        let mut flinched = false;
        if reaction == HitReaction::Poise && self.clock >= self.poise_immune_until {
            let window = u64::from(self.tun.hrdq16_window);
            let now = self.clock;
            self.poise
                .retain(|(at, _)| now.saturating_sub(*at) < window);
            self.poise.push((now, hit.poise));
            let total: u16 = self.poise.iter().map(|(_, p)| *p).sum();
            if total >= self.tun.hrdq16_poise {
                self.poise.clear();
                self.poise_immune_until = now + u64::from(self.tun.hrdq16_immune_ticks);
                self.enter(PileMode::Flinch);
                // A hit lands before her step, which counts the tick first:
                // the next step is the Flinch's tick 1, so it lasts 12 ticks.
                self.mode_tick = 0;
                self.nest_push = false;
                flinched = true;
                out.pile_events.push((tick, PileEvent::Flinch));
            }
        }
        Some(HitResult {
            hp_left: self.hp,
            defeated: false,
            flinched,
        })
    }

    /// The J-B landing on her body (HRD-Q18): 6 damage that never kills (her
    /// HP floors at `hrdq18_floor_hp`), then Exposed, Get-up and Cooldown. It
    /// overrides any mode that has a body, the lunge included. Returns whether
    /// it applied.
    pub fn expose(&mut self, credit: Option<u32>, tick: u64, out: &mut PileOut) -> bool {
        if self.standing_box().is_none() {
            return false;
        }
        let floor = self.tun.hrdq18_floor_hp;
        let before = self.hp;
        if before > floor {
            self.hp = before.saturating_sub(self.tun.hrdq18_damage).max(floor);
        }
        let dealt = before - self.hp;
        if dealt > 0 {
            out.pile_events.push((
                tick,
                PileEvent::Hurt {
                    source: credit,
                    damage: u8::try_from(dealt).unwrap_or(u8::MAX),
                    hp_left: self.hp,
                },
            ));
        }
        self.enter(PileMode::Exposed);
        self.nest_push = false;
        self.poise.clear();
        out.pile_events.push((tick, PileEvent::Exposed { credit }));
        true
    }

    fn defeat(&mut self) {
        self.mode = PileMode::Defeated;
        self.mode_tick = 1;
        self.gate = Gate::Released;
        self.home = Home::None;
        self.nest_push = false;
        self.disengage_pending = false;
    }

    fn enter(&mut self, mode: PileMode) {
        self.mode = mode;
        self.mode_tick = 1;
        if mode == PileMode::ShoveTell {
            self.shove_started = self.clock;
        }
    }

    // ---- the follower's view

    /// The AI partner's view of her attack (design section 8): the swipe is a
    /// `Pound` whose `x` is a dodge anchor `hrdq33_pound_anchor_px` ahead of the
    /// pivot with `target: None` (it hits every hero in the box); the shove is
    /// a `Charge` on her real pivot.
    pub fn threat(&self) -> Option<BossThreat> {
        let anchor = f32::from(self.tun.hrdq33_pound_anchor_px);
        match self.mode {
            PileMode::SwipeTell | PileMode::SwipeSmear | PileMode::SwipeImpact => {
                Some(BossThreat::Pound {
                    x: self.x + f32::from(self.facing) * anchor,
                    facing: self.facing,
                    target: None,
                })
            }
            PileMode::ShoveTell | PileMode::Lunge => {
                let b = &self.tun.hrdq12_box;
                let tell = u32::from(self.tun.hrdq11_tell_ticks);
                Some(BossThreat::Charge {
                    tell_left: if self.mode == PileMode::ShoveTell {
                        u16::try_from(tell.saturating_sub(self.mode_tick)).unwrap_or(u16::MAX)
                    } else {
                        0
                    },
                    x: self.x,
                    dir: self.facing,
                    running: self.mode == PileMode::Lunge,
                    front: f32::from(b.x1),
                    depth: f32::from(b.x1 - b.x0),
                    speed: f32::from(self.tun.hrdq11_px_per_tick),
                    lead_ticks: self.tun.hrdq33_ai_jump_lead,
                    started: self.shove_started,
                })
            }
            _ => None,
        }
    }

    // ---- stepping

    /// One fixed tick. Call after the heroes' step and their hits.
    pub fn step(&mut self, ctx: &PileCtx) -> PileOut {
        let mut out = PileOut::default();
        if !self.tun.enabled() {
            return out;
        }
        self.clock += 1;
        self.suppress_push = false;
        if matches!(self.gate, Gate::Open { .. }) {
            self.step_gate(ctx, &mut out);
            return out;
        }
        if self.mode == PileMode::Defeated {
            return out;
        }
        if self.hit_stop > 0 {
            self.hit_stop -= 1;
        } else {
            self.shove_cd = self.shove_cd.saturating_sub(1);
            self.mode_tick += 1;
            self.suppress_push = self.mode == PileMode::Lunge;
            self.transitions();
            self.run_mode(ctx, &mut out);
        }
        self.push_heroes(ctx, &mut out);
        self.update_counters(ctx);
        out
    }

    /// The entry gate (the Juicer rule, DV-6, HRD-Q41).
    fn step_gate(&mut self, ctx: &PileCtx, out: &mut PileOut) {
        let Gate::Open { crossed } = self.gate else {
            return;
        };
        let tun = self.tun;
        let present: Vec<&HeroView> = ctx.heroes.iter().filter(|h| h.present).collect();
        let crossed = crossed.or_else(|| {
            present
                .iter()
                .any(|h| h.x >= f32::from(tun.hrdq05_engage_x))
                .then_some(self.clock)
        });
        self.gate = Gate::Open { crossed };
        let Some(at) = crossed else {
            return;
        };
        let elapsed = self.clock - at;
        let all_inside = present
            .iter()
            .all(|h| h.x >= f32::from(tun.hrdq41_inside_x));
        if elapsed >= u64::from(tun.hrdq41_settle_ticks) && all_inside && !present.is_empty() {
            self.gate = Gate::Closed;
            self.enter(PileMode::Reveal);
            out.pile_events.push((ctx.tick, PileEvent::Engaged));
            return;
        }
        // Solo: the bot hero is placed when it is still outside after the
        // regroup ticks. Two players wait indefinitely.
        if ctx.solo
            && elapsed >= u64::from(tun.hrdq41_regroup_ticks)
            && let Some(bot) = present
                .iter()
                .find(|h| h.bot && h.x < f32::from(tun.hrdq41_inside_x))
        {
            out.regroup = Some(bot.id);
        }
    }

    /// End-of-state transitions: a state of N ticks ends when its tick would
    /// be N + 1, and the next state starts on that same tick.
    fn transitions(&mut self) {
        let t = self.tun;
        let tick = self.mode_tick;
        let (next, flips) = match self.mode {
            PileMode::Reveal if tick > self.reveal_ticks() => (PileMode::Neutral, false),
            PileMode::Turn if tick > u32::from(t.hrdq04_turn_ticks) => (PileMode::Neutral, true),
            PileMode::SwipeTell if tick > u32::from(t.swipe_tell_ticks()) => {
                (PileMode::SwipeSmear, false)
            }
            PileMode::SwipeSmear if tick > u32::from(t.hrdq08_smear_ticks) => {
                (PileMode::SwipeImpact, false)
            }
            PileMode::SwipeImpact if tick > u32::from(t.hrdq08_impact_ticks) => {
                (PileMode::SwipeRecover, false)
            }
            PileMode::SwipeRecover if tick > u32::from(t.hrdq08_recover_ticks) => {
                (PileMode::Cooldown, false)
            }
            PileMode::ShoveTell if tick > u32::from(t.hrdq11_tell_ticks) => {
                (PileMode::Lunge, false)
            }
            PileMode::ShoveRecover if tick > u32::from(t.hrdq13_recover_ticks) => {
                (PileMode::Cooldown, false)
            }
            PileMode::Jammed if tick > u32::from(t.hrdq13_jam_ticks) => (PileMode::Cooldown, false),
            PileMode::Flinch if tick > u32::from(t.hrdq16_flinch_ticks) => {
                (PileMode::Cooldown, false)
            }
            PileMode::Exposed if tick > u32::from(t.hrdq18_exposed_ticks) => {
                (PileMode::GetUp, false)
            }
            PileMode::GetUp if tick > u32::from(t.hrdq18_getup_ticks) => {
                (PileMode::Cooldown, false)
            }
            PileMode::Cooldown if tick > u32::from(t.hrdq08_cooldown_ticks) => {
                (PileMode::Neutral, false)
            }
            _ => return,
        };
        if flips {
            self.facing = -self.facing;
        }
        self.enter(next);
    }

    fn run_mode(&mut self, ctx: &PileCtx, out: &mut PileOut) {
        match self.mode {
            PileMode::Neutral | PileMode::Walk => self.choose(ctx, out),
            PileMode::Cooldown => {
                // The nest push waits for any attack or recovery, never for the
                // shove cooldown: Cooldown counts as Neutral for it.
                if self.nest_ready(ctx) {
                    self.begin_nest_push(ctx, out);
                }
            }
            PileMode::SwipeImpact => self.swipe_impact(ctx, out),
            PileMode::Lunge => self.lunge(ctx, out),
            PileMode::GetUp if self.mode_tick == 1 => {
                out.pile_events.push((ctx.tick, PileEvent::GetUp));
            }
            _ => {}
        }
    }

    // ---- targeting

    fn eligible(&self, hero: &HeroView) -> bool {
        hero.present && hero.foot_y - self.setup.floor_y <= f32::from(self.tun.hrdq06_eligible_px)
    }

    /// The Juicer rule over eligible heroes (nearest by pivot distance, the
    /// hysteresis, the solo bias); with none eligible, over all present heroes.
    fn update_target(&mut self, ctx: &PileCtx) {
        let tun = self.tun;
        let effective = |h: &HeroView| {
            let bias = if ctx.solo && h.bot {
                f32::from(tun.hrdq06_ai_bias_px)
            } else {
                0.0
            };
            (h.x - self.x).abs() + bias
        };
        let eligible: Vec<&HeroView> = ctx.heroes.iter().filter(|h| self.eligible(h)).collect();
        let any_eligible = !eligible.is_empty();
        let pool: Vec<&HeroView> = if any_eligible {
            eligible
        } else {
            ctx.heroes.iter().filter(|h| h.present).collect()
        };
        let nearest = pool.iter().copied().min_by(|a, b| {
            effective(a)
                .total_cmp(&effective(b))
                .then(a.id.index().cmp(&b.id.index()))
        });
        let held = self
            .target
            .and_then(|id| pool.iter().copied().find(|h| h.id == id));
        let switch = f32::from(tun.hrdq06_switch_px);
        self.target = match (held, nearest) {
            (Some(kept), Some(best)) if best.id != kept.id => {
                if effective(best) + switch <= effective(kept) {
                    Some(best.id)
                } else {
                    Some(kept.id)
                }
            }
            (Some(kept), _) => Some(kept.id),
            (None, best) => best.map(|h| h.id),
        };
        self.target_eligible = any_eligible
            && self
                .target
                .is_some_and(|id| self.eligible(&ctx.heroes[id.index()]));
    }

    fn target_view<'a>(&self, ctx: &'a PileCtx) -> Option<&'a HeroView> {
        self.target
            .map(|id| &ctx.heroes[id.index()])
            .filter(|h| h.present)
    }

    fn credit_of(view: &HeroView) -> Option<u32> {
        (!view.bot).then(|| view.id.index() as u32)
    }

    // ---- the nest rule

    fn bank_a(ctx: &PileCtx) -> Option<Rect> {
        ctx.obstacles
            .iter()
            .find(|o| o.kind == ObstacleKind::BankA)
            .map(|o| o.rect)
    }

    /// A hero qualifies for the nest rule when its hurtbox left edge is below
    /// the bank's far edge, at any height (design section 5).
    fn qualifies(hero: &HeroView, bank: Rect) -> bool {
        hero.present && hero.hurt.left < bank.right()
    }

    /// The walk goal of the nest rule: the face of bank A with no standoff,
    /// while the walk target is on its top or at x below its far edge.
    pub fn nest_walk_goal(&self, ctx: &PileCtx) -> Option<f32> {
        if self.tun.hrdq24_nest_ticks == 0 {
            return None;
        }
        let bank = Self::bank_a(ctx)?;
        let target = self.target_view(ctx)?;
        Self::qualifies(target, bank).then(|| f32::from(self.tun.hrdq24_face_x))
    }

    fn nest_ready(&self, ctx: &PileCtx) -> bool {
        self.tun.hrdq24_nest_ticks > 0
            && Self::bank_a(ctx).is_some()
            && !ctx.bank_a_disturbed
            && self.nest_ticks >= u32::from(self.tun.hrdq24_nest_ticks)
    }

    fn begin_nest_push(&mut self, ctx: &PileCtx, out: &mut PileOut) {
        let Some(bank) = Self::bank_a(ctx) else {
            return;
        };
        let dir = sign(bank.left + bank.width * 0.5 - self.x);
        if self.facing != dir {
            // Turn first: the nest rule keeps its priority over choice 3.
            self.enter(PileMode::Turn);
            return;
        }
        let nearest = ctx
            .heroes
            .iter()
            .filter(|h| Self::qualifies(h, bank))
            .min_by(|a, b| {
                (a.x - self.x)
                    .abs()
                    .total_cmp(&(b.x - self.x).abs())
                    .then(a.id.index().cmp(&b.id.index()))
            });
        if let Some(view) = nearest {
            self.target = Some(view.id);
            self.credit = Self::credit_of(view);
        }
        self.nest_push = true;
        self.nest_ticks = 0;
        self.strike_hit = [false; 2];
        self.enter(PileMode::ShoveTell);
        out.pile_events.push((ctx.tick, PileEvent::NestPush));
        if let Some(id) = self.target {
            out.pile_events.push((
                ctx.tick,
                PileEvent::Tell {
                    attack: PileAttack::Shove,
                    target: id,
                },
            ));
        }
    }

    // ---- the Neutral choice

    /// The choice each Neutral tick, in the sheet's order: nest, home,
    /// turn, swipe, shove, walk.
    fn choose(&mut self, ctx: &PileCtx, out: &mut PileOut) {
        let tun = self.tun;
        self.update_target(ctx);
        if self.nest_ready(ctx) {
            self.begin_nest_push(ctx, out);
            return;
        }
        let home = self.home_rule(ctx, out);
        if home == HomeRule::Spent {
            return;
        }
        let Some(target) = self.target_view(ctx).copied() else {
            self.mode = PileMode::Neutral;
            return;
        };
        let gap = target.x - self.x;
        if sign(gap) != self.facing && gap.abs() > TURN_DEADZONE_PX {
            self.enter(PileMode::Turn);
            return;
        }
        let ahead = gap * f32::from(self.facing);
        if self.target_eligible && (0.0..=f32::from(tun.hrdq07_trigger_px)).contains(&ahead) {
            self.begin_swipe(&target, ctx, out);
            return;
        }
        if self.target_eligible
            && (f32::from(tun.hrdq10_range_min_px)..=f32::from(tun.hrdq10_range_max_px))
                .contains(&ahead)
            && self.shove_cd == 0
            && !self.shove_ends_short(&target)
        {
            self.begin_shove(&target, ctx, out);
            return;
        }
        if home == HomeRule::NoWalk {
            self.mode = PileMode::Neutral;
            return;
        }
        let moved = match self.nest_walk_goal(ctx) {
            Some(goal) => {
                let dir = sign(goal - self.x);
                self.walk(ctx, dir, Some(goal))
            }
            None if ahead > f32::from(tun.hrdq04_standoff_px) => self.walk(ctx, self.facing, None),
            None => false,
        };
        self.mode = if moved {
            PileMode::Walk
        } else {
            PileMode::Neutral
        };
    }

    /// The shove guard (review N7): no shove when the lunge would end at a
    /// leash edge before reaching the target, that is when the target's
    /// hurtbox lies wholly beyond the box front at the leash clamp.
    fn shove_ends_short(&self, target: &HeroView) -> bool {
        let tun = self.tun;
        let dir = f32::from(self.facing);
        let reach = f32::from(tun.hrdq11_px_per_tick) * f32::from(tun.hrdq11_max_ticks);
        let end = (self.x + dir * reach).clamp(
            f32::from(tun.hrdq19_leash_min),
            f32::from(tun.hrdq19_leash_max),
        );
        let front = end + dir * f32::from(tun.hrdq12_box.x1);
        if self.facing < 0 {
            target.hurt.right() <= front
        } else {
            target.hurt.left >= front
        }
    }

    fn begin_swipe(&mut self, target: &HeroView, ctx: &PileCtx, out: &mut PileOut) {
        self.strike_hit = [false; 2];
        self.credit = Self::credit_of(target);
        self.nest_push = false;
        self.enter(PileMode::SwipeTell);
        out.pile_events.push((
            ctx.tick,
            PileEvent::Tell {
                attack: PileAttack::Swipe,
                target: target.id,
            },
        ));
    }

    fn begin_shove(&mut self, target: &HeroView, ctx: &PileCtx, out: &mut PileOut) {
        self.strike_hit = [false; 2];
        self.credit = Self::credit_of(target);
        self.nest_push = false;
        self.enter(PileMode::ShoveTell);
        out.pile_events.push((
            ctx.tick,
            PileEvent::Tell {
                attack: PileAttack::Shove,
                target: target.id,
            },
        ));
    }

    /// Fixture hook: start a shove at the nearest present hero whatever the
    /// range and the cooldown (the forced shove of the reach tests).
    pub fn force_shove(&mut self, ctx: &PileCtx) -> PileOut {
        let mut out = PileOut::default();
        self.update_target(ctx);
        if let Some(target) = self.target_view(ctx).copied() {
            self.facing = sign(target.x - self.x);
            self.begin_shove(&target, ctx, &mut out);
            // The next step is the tell's first tick (attack tick 0).
            self.mode_tick = 0;
        }
        out
    }

    /// Fixture hook: start a swipe at the nearest present hero whatever its
    /// distance (the forced swipe of the reach tests).
    pub fn force_swipe(&mut self, ctx: &PileCtx) -> PileOut {
        let mut out = PileOut::default();
        self.update_target(ctx);
        if let Some(target) = self.target_view(ctx).copied() {
            self.facing = sign(target.x - self.x);
            self.begin_swipe(&target, ctx, &mut out);
            self.mode_tick = 0;
        }
        out
    }

    // ---- walking, the leash and the home rule

    /// One walking step toward `dir`, stopping at `goal` when given. Bank A
    /// (and any solid) stops her body; the leash clamps her. Returns whether
    /// she moved.
    fn walk(&mut self, ctx: &PileCtx, dir: i8, goal: Option<f32>) -> bool {
        let tun = self.tun;
        let mut step = f32::from(tun.hrdq04_walk_px_s) * TICK;
        if let Some(goal) = goal {
            step = step.min((goal - self.x).abs());
        }
        let old = self.x;
        let new = self.clamp_walk(ctx, old + f32::from(dir) * step);
        self.x = new;
        (new - old).abs() > f32::EPSILON
    }

    fn clamp_walk(&self, ctx: &PileCtx, wanted: f32) -> f32 {
        let tun = self.tun;
        let (half_back, half_front) = (
            -f32::from(tun.hrdq03_body.x0),
            f32::from(tun.hrdq03_body.x1),
        );
        let mut x = wanted;
        let floor = self.setup.floor_y;
        let top = floor + f32::from(tun.hrdq03_body.y1);
        for ob in ctx.obstacles.iter().filter(|o| o.blocks_her_body()) {
            let r = ob.rect;
            if r.top() <= floor || r.bottom >= top || r.height <= REMNANT_MAX_PX {
                continue;
            }
            if x + half_front > r.left && x - half_back < r.right() {
                x = if self.x < r.left + r.width * 0.5 {
                    r.left - half_front
                } else {
                    r.right() + half_back
                };
            }
        }
        x.clamp(
            f32::from(tun.hrdq19_leash_min),
            f32::from(tun.hrdq19_leash_max),
        )
    }

    fn at_leash_edge(&self) -> Option<i8> {
        let tun = self.tun;
        if self.x <= f32::from(tun.hrdq19_leash_min) + EDGE_TOLERANCE_PX {
            Some(-1)
        } else if self.x >= f32::from(tun.hrdq19_leash_max) - EDGE_TOLERANCE_PX {
            Some(1)
        } else {
            None
        }
    }

    /// The disengage answer (review N1): an eligible hero inside the swipe
    /// trigger on either side, or inside the shove trigger on either side and
    /// not beyond the leash edge she left.
    fn follow_answers(&self, ctx: &PileCtx, edge: i8) -> bool {
        let tun = self.tun;
        let (min, max) = (
            f32::from(tun.hrdq19_leash_min),
            f32::from(tun.hrdq19_leash_max),
        );
        ctx.heroes.iter().filter(|h| self.eligible(h)).any(|h| {
            let d = (h.x - self.x).abs();
            let beyond = if edge < 0 { h.x < min } else { h.x > max };
            d <= f32::from(tun.hrdq07_trigger_px)
                || ((f32::from(tun.hrdq10_range_min_px)..=f32::from(tun.hrdq10_range_max_px))
                    .contains(&d)
                    && !beyond)
        })
    }

    fn any_hero_within(&self, ctx: &PileCtx, px: f32) -> bool {
        ctx.heroes
            .iter()
            .any(|h| h.present && (h.x - self.x).abs() <= px)
    }

    fn face_nearest(&mut self, ctx: &PileCtx) {
        let nearest = ctx.heroes.iter().filter(|h| h.present).min_by(|a, b| {
            (a.x - self.x)
                .abs()
                .total_cmp(&(b.x - self.x).abs())
                .then(a.id.index().cmp(&b.id.index()))
        });
        if let Some(h) = nearest {
            self.facing = sign(h.x - self.x);
        }
    }

    /// Walk toward home; arriving starts the hold.
    fn walk_home(&mut self, ctx: &PileCtx) {
        let home = f32::from(self.tun.hrdq19_home_x);
        let dir = sign(home - self.x);
        self.facing = dir;
        let moved = self.walk(ctx, dir, Some(home));
        self.mode = if moved {
            PileMode::Walk
        } else {
            PileMode::Neutral
        };
        if (self.x - home).abs() <= f32::EPSILON {
            self.home = Home::Hold;
            self.face_nearest(ctx);
        }
    }

    fn home_rule(&mut self, ctx: &PileCtx, out: &mut PileOut) -> HomeRule {
        let tun = self.tun;
        if self.home == Home::None {
            if self.disengage_pending {
                self.disengage_pending = false;
                self.unreachable_ticks = 0;
                self.poise.clear();
                self.home = Home::Disengage {
                    edge: self.disengage_edge,
                };
                out.pile_events.push((ctx.tick, PileEvent::Disengage));
            } else if self.away_ticks >= u32::from(tun.hrdq19_return_ticks) {
                self.away_ticks = 0;
                self.poise.clear();
                self.home = Home::Return;
            }
        }
        match self.home {
            Home::None => HomeRule::Continue,
            Home::Disengage { edge } => {
                if self.follow_answers(ctx, edge) {
                    self.home = Home::None;
                    HomeRule::Continue
                } else {
                    self.walk_home(ctx);
                    HomeRule::Spent
                }
            }
            Home::Return => {
                if self.any_hero_within(ctx, f32::from(tun.hrdq19_return_px)) {
                    self.home = Home::None;
                    HomeRule::Continue
                } else {
                    self.walk_home(ctx);
                    HomeRule::Spent
                }
            }
            Home::Hold => {
                if self.any_hero_within(ctx, f32::from(tun.hrdq19_return_px)) {
                    self.home = Home::None;
                    HomeRule::Continue
                } else {
                    self.face_nearest(ctx);
                    HomeRule::NoWalk
                }
            }
        }
    }

    /// The consecutive-tick counters, counted in every mode after the tick's
    /// movement: the nest timer, the unreachable ticks and the away ticks.
    fn update_counters(&mut self, ctx: &PileCtx) {
        let tun = self.tun;
        let near_face = (self.x - f32::from(tun.hrdq24_face_x)).abs() <= FACE_TOLERANCE_PX;
        let nest = Self::bank_a(ctx)
            .is_some_and(|bank| near_face && ctx.heroes.iter().any(|h| Self::qualifies(h, bank)));
        self.nest_ticks = if nest && tun.hrdq24_nest_ticks > 0 {
            self.nest_ticks + 1
        } else {
            0
        };
        let unreachable = self.target_view(ctx).is_some_and(|t| {
            let beyond = match self.at_leash_edge() {
                Some(-1) => t.x < self.x,
                Some(_) => t.x > self.x,
                None => false,
            };
            beyond && (t.x - self.x).abs() > f32::from(tun.hrdq07_trigger_px)
        });
        self.unreachable_ticks = if unreachable {
            self.unreachable_ticks + 1
        } else {
            0
        };
        if self.unreachable_ticks >= u32::from(tun.hrdq19_unreachable_ticks)
            && self.home == Home::None
        {
            self.disengage_pending = true;
            if let Some(edge) = self.at_leash_edge() {
                self.disengage_edge = edge;
            }
        }
        self.away_ticks = if self.any_hero_within(ctx, f32::from(tun.hrdq19_return_px)) {
            0
        } else {
            self.away_ticks + 1
        };
    }

    // ---- the swipe

    fn blow(&self, view: &HeroView, attack: PileAttack, damage: u8, vx: f32, vy: f32) -> PileBlow {
        PileBlow {
            blow: HeroBlow {
                hero: view.id,
                source: PILE_BOSS_SOURCE_ID,
                damage,
                from_x: self.x,
                knock_vx: vx,
                knock_vy: vy,
            },
            attack,
            policy: BLOW_POLICY,
            evadable: false,
        }
    }

    fn swipe_impact(&mut self, ctx: &PileCtx, out: &mut PileOut) {
        let tun = self.tun;
        if self.mode_tick == 1 {
            out.pile_events.push((
                ctx.tick,
                PileEvent::Strike {
                    attack: PileAttack::Swipe,
                },
            ));
        }
        if !BLOW_POLICY.hurts_any_hero() {
            return;
        }
        let area = self.swipe_box();
        for hero in ctx.heroes.iter().filter(|h| h.present && h.vulnerable) {
            let i = hero.id.index();
            if !self.strike_hit[i] && hero.hurt.overlaps(area) {
                self.strike_hit[i] = true;
                let away = if hero.x >= self.x { 1.0 } else { -1.0 };
                out.blows.push(self.blow(
                    hero,
                    PileAttack::Swipe,
                    tun.hrdq09_damage,
                    away * f32::from(tun.hrdq09_knock_x),
                    f32::from(tun.hrdq09_knock_y),
                ));
            }
        }
    }

    // ---- the shove

    /// One lunge tick, in the sheet's order: step and clamp, test the heroes,
    /// the obstacle, the distance.
    fn lunge(&mut self, ctx: &PileCtx, out: &mut PileOut) {
        let tun = self.tun;
        let k = self.mode_tick;
        if k == 1 {
            out.pile_events.push((
                ctx.tick,
                PileEvent::Strike {
                    attack: PileAttack::Shove,
                },
            ));
        }
        let dir = f32::from(self.facing);
        let old = self.x;
        let front = f32::from(tun.hrdq12_box.x1);
        let mut new = old + dir * f32::from(tun.hrdq11_px_per_tick);
        // (1) obstacles: the box front may not pass a face; never backward.
        let floor = self.setup.floor_y;
        let box_top = floor + f32::from(tun.hrdq12_box.y1);
        let mut met: Option<(ObstacleKind, f32)> = None;
        for ob in ctx.obstacles.iter().filter(|o| o.stops_shove()) {
            let r = ob.rect;
            if r.top() <= floor || r.bottom >= box_top {
                continue;
            }
            let face = if self.facing > 0 { r.left } else { r.right() };
            let ahead = if self.facing > 0 {
                face >= old
            } else {
                face <= old
            };
            let passes = if self.facing > 0 {
                new + front > face
            } else {
                new - front < face
            };
            if ahead && passes {
                let nearer = met.is_none_or(|(_, best)| (face - old).abs() < (best - old).abs());
                if nearer {
                    met = Some((ob.kind, face));
                }
            }
        }
        if let Some((_, face)) = met {
            new = if self.facing > 0 {
                new.min((face - front).max(old))
            } else {
                new.max((face + front).min(old))
            };
        }
        let (min, max) = (
            f32::from(tun.hrdq19_leash_min),
            f32::from(tun.hrdq19_leash_max),
        );
        let clamped = new.clamp(min, max);
        let at_leash = clamped <= min || clamped >= max;
        self.x = clamped;
        // (2) heroes: once per hero, the lunge continues.
        if BLOW_POLICY.hurts_any_hero() {
            let area = self.shove_box();
            for hero in ctx.heroes.iter().filter(|h| h.present && h.vulnerable) {
                let i = hero.id.index();
                if !self.strike_hit[i] && hero.hurt.overlaps(area) {
                    self.strike_hit[i] = true;
                    out.blows.push(self.blow(
                        hero,
                        PileAttack::Shove,
                        tun.hrdq12_damage,
                        dir * f32::from(tun.hrdq12_knock_x),
                        f32::from(tun.hrdq12_knock_y),
                    ));
                }
            }
        }
        // (3) the obstacle, (4) the distance.
        if let Some((kind, _)) = met {
            out.pile_events
                .push((ctx.tick, PileEvent::Jammed { obstacle: kind }));
            out.jams.push(JamNotice {
                tick: ctx.tick,
                obstacle: kind,
                credit: self.credit,
            });
            self.shove_cd = Self::shove_cooldown_after_lunge(tun);
            self.enter(PileMode::Jammed);
        } else if at_leash || k >= u32::from(tun.hrdq11_max_ticks) {
            self.shove_cd = Self::shove_cooldown_after_lunge(tun);
            self.enter(PileMode::ShoveRecover);
        }
    }

    // ---- the body push-out

    /// After hero physics: no hero stands inside her body. A hero whose box
    /// overlaps it, with its feet below the box top, is pushed out at most
    /// `hrdq03_push_px` per tick toward the nearer edge, toward the other side
    /// when fewer than `hrdq03_side_room_px` are free before a solid on the
    /// nearer one (measured in the hero's own height band, from its feet up),
    /// never into a solid. Nothing on the first tick after... that is, nothing
    /// during a lunge tick (her body is not solid then).
    fn push_heroes(&self, ctx: &PileCtx, out: &mut PileOut) {
        if self.suppress_push {
            return;
        }
        let Some(body) = self.body_rect() else {
            return;
        };
        let tun = self.tun;
        let push = f32::from(tun.hrdq03_push_px);
        let min_room = f32::from(tun.hrdq03_side_room_px);
        let gate = self.gate_solid();
        for hero in ctx.heroes.iter().filter(|h| h.present) {
            let left = hero.x - HERO_HALF_WIDTH;
            let right = hero.x + HERO_HALF_WIDTH;
            if right <= body.left || left >= body.right() || hero.foot_y >= body.top() {
                continue;
            }
            let band_bottom = hero.foot_y + 0.01;
            let band_top = hero.foot_y + HERO_HALF_HEIGHT * 2.0;
            let in_band = |r: &Rect| r.top() > band_bottom && r.bottom < band_top;
            let solids: Vec<Rect> = ctx
                .obstacles
                .iter()
                .filter(|o| o.blocks_heroes())
                .map(|o| o.rect)
                .chain(gate)
                .filter(in_band)
                .collect();
            let wall_left = self.setup.walls.0;
            let wall_right = self.setup.walls.1;
            let limit_left = solids
                .iter()
                .filter(|r| r.right() <= body.left + f32::EPSILON)
                .map(|r| r.right())
                .fold(wall_left, f32::max);
            let limit_right = solids
                .iter()
                .filter(|r| r.left >= body.right() - f32::EPSILON)
                .map(|r| r.left)
                .fold(wall_right, f32::min);
            let room_left = body.left - limit_left;
            let room_right = limit_right - body.right();
            let to_left = body.left - HERO_HALF_WIDTH - 1.0;
            let to_right = body.right() + HERO_HALF_WIDTH + 1.0;
            let nearer_left = (hero.x - to_left).abs() <= (to_right - hero.x).abs();
            let go_left = if nearer_left {
                room_left >= min_room || room_right < min_room
            } else {
                room_right < min_room && room_left >= min_room
            };
            let goal = if go_left { to_left } else { to_right };
            let to = hero.x + (goal - hero.x).clamp(-push, push);
            let probe = hero_rect(to, hero.foot_y);
            let blocked = solids.iter().any(|r| r.overlaps(probe));
            if !blocked {
                out.shifts.push((hero.id, to));
            }
        }
    }

    // ---- hashing

    /// Fold the whole state into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.clock);
        mix(u64::from(self.hp));
        mix(self.mode.code());
        mix(u64::from(self.mode_tick));
        mix(u64::from(self.x.to_bits()));
        mix(i64::from(self.facing) as u64);
        match self.gate {
            Gate::Open { crossed } => {
                mix(1);
                mix(crossed.map_or(0, |c| c + 1));
            }
            Gate::Closed => mix(2),
            Gate::Released => mix(3),
        }
        mix(self.target.map_or(0, |t| t.index() as u64 + 1));
        mix(u64::from(self.target_eligible));
        mix(u64::from(self.hit_stop));
        mix(self.poise.len() as u64);
        for (at, amount) in &self.poise {
            mix(*at);
            mix(u64::from(*amount));
        }
        mix(self.poise_immune_until);
        mix(u64::from(self.shove_cd));
        mix(u64::from(self.strike_hit[0]) | u64::from(self.strike_hit[1]) << 1);
        mix(u64::from(self.nest_push));
        mix(self.credit.map_or(0, |c| u64::from(c) + 1));
        mix(u64::from(self.nest_ticks));
        mix(u64::from(self.unreachable_ticks));
        mix(u64::from(self.away_ticks));
        mix(u64::from(self.disengage_pending));
        mix(i64::from(self.disengage_edge) as u64);
        match self.home {
            Home::None => mix(0),
            Home::Disengage { edge } => {
                mix(1);
                mix(i64::from(edge) as u64);
            }
            Home::Return => mix(2),
            Home::Hold => mix(3),
        }
        mix(u64::from(self.retry));
    }
}

// ------------------------------------------------------------ sidecar data --

/// Bank A, the J-B post and the exit door of her sidecar as [`Obstacle`]s in
/// world px. `layout_floor_y` is the room's walking line in layout px (y
/// down), `floor_y` the same line in world px (y up). The door is an obstacle
/// only while it is closed.
pub fn obstacles_from_encounter(
    encounter: &Encounter,
    layout_floor_y: i32,
    floor_y: f32,
    door_open: bool,
) -> Vec<Obstacle> {
    let to_world = |r: crate::camera_lab::room::IRect| {
        rect(
            r.x as f32,
            floor_y + (layout_floor_y - r.bottom()) as f32,
            r.w as f32,
            r.h as f32,
        )
    };
    let mut out = Vec::new();
    for entry in &encounter.statics {
        let kind = match (entry.kind, entry.id.as_str()) {
            (StaticKind::HitTarget, _) => ObstacleKind::Post,
            (StaticKind::Solid, BANK_A_ID) => ObstacleKind::BankA,
            (StaticKind::Solid, _) => ObstacleKind::Solid,
        };
        out.push(Obstacle {
            kind,
            rect: to_world(entry.rect),
        });
    }
    for solid in encounter.lock_solids(&[door_open]) {
        out.push(Obstacle {
            kind: ObstacleKind::Solid,
            rect: to_world(solid),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_lab::room::IRect;
    use crate::input::InputFrame;
    use crate::prototype::{BodyDrive, Hero, Material};
    use serde_json::Value;

    const FLOOR: f32 = 100.0;
    const E: f32 = 760.0;
    const ROOM_FLOOR_LAYOUT: i32 = 580;

    fn tun() -> &'static PileBossTuning {
        PileBossTuning::active()
    }

    fn setup() -> PileSetup {
        PileSetup {
            floor_y: FLOOR,
            walls: (16.0, 1040.0),
        }
    }

    fn awake(x: f32) -> PileBoss {
        PileBoss::awake_at(tun(), setup(), x, -1)
    }

    fn view(id: HeroId, x: f32, foot: f32, half_w: f32, height: f32) -> HeroView {
        HeroView {
            id,
            present: true,
            x,
            foot_y: foot,
            grounded: foot <= FLOOR + 0.5,
            hurt: rect(x - half_w, foot, half_w * 2.0, height),
            vulnerable: true,
            bot: false,
        }
    }

    fn carl(x: f32) -> HeroView {
        view(HeroId::Carl, x, FLOOR, 12.0, 52.0)
    }

    fn donut(x: f32) -> HeroView {
        view(HeroId::Donut, x, FLOOR, 10.0, 24.0)
    }

    fn absent(id: HeroId) -> HeroView {
        HeroView {
            present: false,
            ..view(id, 0.0, FLOOR, 12.0, 52.0)
        }
    }

    fn heroes_carl(x: f32) -> [HeroView; 2] {
        [carl(x), absent(HeroId::Donut)]
    }

    fn heroes_donut(x: f32) -> [HeroView; 2] {
        [absent(HeroId::Carl), donut(x)]
    }

    fn ctx<'a>(heroes: [HeroView; 2], obstacles: &'a [Obstacle], tick: u64) -> PileCtx<'a> {
        PileCtx {
            heroes,
            obstacles,
            solo: false,
            tick,
            bank_a_disturbed: false,
        }
    }

    fn bank_a() -> Obstacle {
        Obstacle {
            kind: ObstacleKind::BankA,
            rect: rect(352.0, FLOOR, 64.0, 48.0),
        }
    }

    fn post() -> Obstacle {
        Obstacle {
            kind: ObstacleKind::Post,
            rect: rect(808.0, FLOOR, 16.0, 64.0),
        }
    }

    /// Run `n` ticks with fixed heroes; returns every output.
    fn run(
        boss: &mut PileBoss,
        heroes: [HeroView; 2],
        obstacles: &[Obstacle],
        n: u64,
    ) -> Vec<PileOut> {
        (1..=n)
            .map(|t| boss.step(&ctx(heroes, obstacles, t)))
            .collect()
    }

    fn hit(damage: u16, poise: u16) -> PileHit {
        PileHit {
            damage,
            poise,
            player: 0,
            hit_stop: 0,
            cause: DefeatCause::Melee,
        }
    }

    fn jab(boss: &mut PileBoss, tick: u64) -> (Option<HitResult>, PileOut) {
        let mut out = PileOut::default();
        let r = boss.apply_hit(hit(1, 1), None, tick, &mut out);
        (r, out)
    }

    fn state_hash(boss: &PileBoss) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325_u64;
        boss.mix_into(&mut |v| {
            h ^= v;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        });
        h
    }

    // ------------------------------------------------------------- data ---

    fn raw() -> Value {
        serde_json::from_str(TUNING_JSON).unwrap()
    }

    fn field_error(edit: impl FnOnce(&mut Value)) -> &'static str {
        let mut v = raw();
        edit(&mut v);
        match PileBossTuning::from_json(&v.to_string()) {
            Err(PileBossError::Field { field, .. }) => field,
            other => panic!("expected a field error, got {other:?}"),
        }
    }

    #[test]
    fn the_embedded_data_is_valid_and_pinned() {
        assert_eq!(content_hash(TUNING_JSON), TUNING_HASH, "update the pin");
        let t = PileBossTuning::pinned().expect("valid");
        assert_eq!(&t, PileBossTuning::active());
        assert_eq!(t.hrdq02_hp, 30);
        assert!(t.enabled());
        assert!(t.hrdq41_gate_enabled);
        assert_eq!(t.swipe_tell_ticks(), 34);
        assert!(
            crate::playtrace::live::data_pins()
                .iter()
                .any(|(k, h)| *k == PIN_KEY && *h == TUNING_HASH),
            "registered in data_pins"
        );
    }

    #[test]
    fn a_drifted_file_fails_the_pin_with_both_hashes() {
        let drifted = TUNING_JSON.replace("\"hrdq02_hp\": 30", "\"hrdq02_hp\": 31");
        match PileBossTuning::from_json_pinned(&drifted, TUNING_HASH) {
            Err(PileBossError::HashMismatch { expected, actual }) => {
                assert_eq!(expected, TUNING_HASH);
                assert_ne!(actual, TUNING_HASH);
            }
            other => panic!("expected a hash mismatch, got {other:?}"),
        }
    }

    #[test]
    fn validation_errors_name_the_field() {
        assert_eq!(field_error(|v| v["hrdq02_hp"] = 0.into()), "hrdq02_hp");
        assert_eq!(
            field_error(|v| v["schema_version"] = 2.into()),
            "schema_version"
        );
        assert_eq!(
            field_error(|v| v["hrdq09_box"]["x1"] = 42.into()),
            "hrdq09_box"
        );
        assert_eq!(
            field_error(|v| v["hrdq10_range_min_px"] = 88.into()),
            "hrdq10_range_min_px"
        );
        assert_eq!(
            field_error(|v| v["hrdq19_home_x"] = 290.into()),
            "hrdq19_home_x"
        );
        assert_eq!(
            field_error(|v| v["hrdq18_floor_hp"] = 30.into()),
            "hrdq18_floor_hp"
        );
        assert_eq!(
            field_error(|v| v["hrdq41_inside_x"] = 500.into()),
            "hrdq41_inside_x"
        );
        assert!(matches!(
            PileBossTuning::from_json("{"),
            Err(PileBossError::Parse(_))
        ));
        let mut v = raw();
        v["hrdq99_extra"] = 1.into();
        assert!(matches!(
            PileBossTuning::from_json(&v.to_string()),
            Err(PileBossError::Parse(_))
        ));
        let msg = PileBossError::Field {
            field: "hrdq02_hp",
            reason: "must be positive".into(),
        }
        .to_string();
        assert!(msg.contains("hrdq02_hp") && msg.contains("must be positive"));
    }

    #[test]
    fn the_fairness_floors_are_checked_on_load() {
        // Every tell at least 24, every punish window at least 24.
        assert_eq!(
            field_error(|v| v["hrdq08_tell_hold_ticks"] = 10.into()),
            "hrdq08_tell_hold_ticks"
        );
        assert_eq!(
            field_error(|v| v["hrdq11_tell_ticks"] = 23.into()),
            "hrdq11_tell_ticks"
        );
        for key in [
            "hrdq08_recover_ticks",
            "hrdq13_recover_ticks",
            "hrdq13_jam_ticks",
            "hrdq18_exposed_ticks",
        ] {
            assert_eq!(field_error(|v| v[key] = 23.into()), key);
        }
        let t = tun();
        assert!(t.swipe_tell_ticks() >= t.hrdq38_min_tell);
        assert_eq!(t.hrdq38_min_warning, 18);
    }

    #[test]
    fn hrd_evade_class_never() {
        assert_eq!(tun().hrdq20_evade_class, "never");
        assert_eq!(
            field_error(|v| v["hrdq20_evade_class"] = "evadable".into()),
            "hrdq20_evade_class"
        );
        assert_eq!(BLOW_POLICY, HeroDamagePolicy::AllHeroes);
        // Every blow of the sheet carries the never class.
        let mut boss = awake(E);
        let outs = run(&mut boss, heroes_carl(E - 60.0), &[], 50);
        let blows: Vec<&PileBlow> = outs.iter().flat_map(|o| &o.blows).collect();
        assert_eq!(blows.len(), 1);
        assert!(blows.iter().all(|b| !b.evadable && b.policy == BLOW_POLICY));
        assert_eq!(blows[0].blow.source, PILE_BOSS_SOURCE_ID);
        assert_ne!(PILE_BOSS_SOURCE_ID, crate::boss::BOSS_SOURCE_ID);
        assert_ne!(PILE_BOSS_SOURCE_ID, crate::boss::RACK_SOURCE_ID);
        assert_ne!(PILE_BOSS_ID, crate::boss::BOSS_ENEMY_ID);
    }

    #[test]
    fn the_boss_bar_is_off_in_v1() {
        assert!(!tun().hrdq36_boss_bar);
        assert_eq!(
            field_error(|v| v["hrdq36_boss_bar"] = true.into()),
            "hrdq36_boss_bar"
        );
    }

    #[test]
    fn every_question_has_a_knob_or_a_recorded_reason() {
        let value = raw();
        let keys: Vec<&String> = value.as_object().unwrap().keys().collect();
        let exempt: Vec<u8> = NO_KNOB.iter().map(|(q, _)| *q).collect();
        for q in 1..=41u8 {
            let prefix = format!("hrdq{q:02}_");
            let has_key = keys.iter().any(|k| k.starts_with(&prefix));
            if exempt.contains(&q) {
                assert!(!has_key, "HRD-Q{q:02} is listed as knob-less but has a key");
            } else {
                assert!(has_key, "HRD-Q{q:02} has no knob and is not in NO_KNOB");
            }
        }
        for key in keys {
            let named = key.starts_with("hrdq")
                && key[4..6].parse::<u8>().is_ok_and(|q| (1..=41).contains(&q))
                && key.as_bytes()[6] == b'_';
            assert!(
                named || key == "schema_version",
                "{key} is not named after an HRD question id"
            );
        }
        for (q, why) in NO_KNOB {
            assert!(!why.is_empty(), "HRD-Q{q:02} needs a reason");
        }
    }

    #[test]
    fn the_ids_are_disjoint_and_the_defeat_row_exists() {
        let ids = [
            PILE_BOSS_ID,
            crate::boss::BOSS_ENEMY_ID,
            crate::boss::RACK_SOURCE_ID,
        ];
        for (i, a) in ids.iter().enumerate() {
            for b in &ids[i + 1..] {
                assert_ne!(a, b);
            }
        }
        let table = crate::story_unlock::StoryTable::active();
        let row = table.trigger(DEFEAT_TRIGGER_ID).expect("the CP-A row");
        assert_eq!(row.grants, ["special_thrown_charge"]);
    }

    // ------------------------------------------------------ room and data ---

    fn root() -> &'static std::path::Path {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn loaded() -> crate::encounter::Loaded {
        crate::encounter::load(root(), ENCOUNTER_PATH).unwrap_or_else(|e| {
            panic!(
                "{}",
                e.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })
    }

    #[test]
    fn the_room_and_sidecar_load_and_validate() {
        let l = loaded();
        assert_eq!(l.room.id, "F01-HOARDER");
        assert_eq!(l.room.render, (1056, 672));
        assert_eq!(l.room.floor_y, ROOM_FLOOR_LAYOUT);
        assert_eq!(l.room.snap_dy, 0);
        assert_eq!(l.ceiling_px, Some(228));
        assert!(l.room.props.is_empty() && l.room.enemies.is_empty() && l.room.hazards.is_empty());
        let e = &l.encounter;
        assert_eq!(e.exit, l.room.exit);
        assert_eq!(e.roster.len(), 1);
        assert_eq!(e.roster[0].id, "pile_boss");
        assert_eq!(
            e.roster[0].kind,
            crate::encounter::RosterKind::Room(crate::camera_lab::room::EnemyKind::Hoarder)
        );
        assert_eq!(
            e.roster[0].trigger_x,
            Some(i32::from(tun().hrdq05_engage_x))
        );
        assert_eq!(e.roster[0].x, i32::from(tun().hrdq19_home_x));
        assert!(e.props.is_empty() && e.hazards.is_empty());
        // Bank A and the post are static sidecar entries.
        let statics: Vec<(&str, StaticKind, IRect)> = e
            .statics
            .iter()
            .map(|s| (s.id.as_str(), s.kind, s.rect))
            .collect();
        assert_eq!(
            statics,
            [
                (BANK_A_ID, StaticKind::Solid, IRect::new(352, 532, 64, 48)),
                (POST_ID, StaticKind::HitTarget, IRect::new(808, 516, 16, 64)),
            ]
        );
        // The door is closed until her defeat (DV-2).
        assert_eq!(e.locks.len(), 1);
        assert_eq!(
            e.locks_opened_by(0),
            [0],
            "the roster entry 0 (her) opens the door"
        );
    }

    #[test]
    fn the_room_composes_through_the_existing_kit() {
        use crate::camera_lab::layout::build_kit;
        let l = loaded();
        let (kit, _, _) = build_kit(root(), &l.room).expect("composes");
        assert_eq!((kit.width, kit.height), l.room.render);
    }

    #[test]
    fn the_file_hashes_are_pinned() {
        let l = loaded();
        assert_eq!(l.room_hash, ROOM_HASH, "update with the room file");
        assert_eq!(l.sidecar_hash, SIDECAR_HASH, "update with the sidecar");
    }

    // FNV-1a 64 of the two files: a data edit must update them on purpose.
    const ROOM_HASH: u64 = 6_580_550_607_703_705_209;
    const SIDECAR_HASH: u64 = 17_892_170_003_339_872_273;

    #[test]
    fn the_sidecar_geometry_matches_the_sheet_and_the_protected_rects_are_clear() {
        let l = loaded();
        let e = &l.encounter;
        let t = tun();
        // The entry pocket is 0..208: no attack box from the leash minimum
        // reaches it (her -x swipe box from 290 ends at 209).
        let left_edge = f32::from(t.hrdq19_leash_min);
        let swipe_left = left_edge - f32::from(t.hrdq09_box.x1);
        assert_eq!(swipe_left, 209.0);
        assert!(swipe_left >= e.protected_spawn.right() as f32);
        assert!(swipe_left >= e.checkpoint.right() as f32);
        // Her +x boxes from the leash maximum stop short of the exit pocket's
        // goal rectangle (and the door closes the pocket while she lives).
        let right_edge = f32::from(t.hrdq19_leash_max);
        let swipe_right = right_edge + f32::from(t.hrdq09_box.x1);
        let shove_right = right_edge + f32::from(t.hrdq12_box.x1);
        assert_eq!(swipe_right, 961.0);
        assert!(swipe_right < e.exit.x as f32 && shove_right < e.exit.x as f32);
        // The entry gate stands between the pocket and the arena.
        let gate = f32::from(t.hrdq41_gate_x);
        assert!(gate >= e.protected_spawn.right() as f32);
        assert!(gate + GATE_WIDTH_PX <= f32::from(t.hrdq41_inside_x));
        // Her home is on the roster entry, bank A is left of the arena.
        let bank = e.statics.iter().find(|s| s.id == BANK_A_ID).unwrap().rect;
        assert!(bank.right() as f32 <= f32::from(t.hrdq24_face_x) - 28.0);
        // The post is inside the leash, the door right of it.
        let post = e.statics.iter().find(|s| s.id == POST_ID).unwrap().rect;
        assert!(post.right() <= i32::from(t.hrdq19_leash_max));
        assert!(e.locks[0].solid.x >= i32::from(t.hrdq19_leash_max));
    }

    #[test]
    fn the_sidecar_becomes_obstacles_in_world_px() {
        let l = loaded();
        let closed = obstacles_from_encounter(&l.encounter, l.room.floor_y, FLOOR, false);
        let kinds: Vec<ObstacleKind> = closed.iter().map(|o| o.kind).collect();
        assert_eq!(
            kinds,
            [ObstacleKind::BankA, ObstacleKind::Post, ObstacleKind::Solid]
        );
        assert_eq!(closed[0].rect, rect(352.0, FLOOR, 64.0, 48.0));
        assert_eq!(closed[1].rect, rect(808.0, FLOOR, 16.0, 64.0));
        assert_eq!(closed[2].rect, rect(928.0, FLOOR, 16.0, 228.0));
        let open = obstacles_from_encounter(&l.encounter, l.room.floor_y, FLOOR, true);
        assert_eq!(open.len(), 2, "the door is gone once she is defeated");
    }

    /// HRD-B / HRD-C: the machines' knobs are validated, each rule with a failing case, and the
    /// shipped sidecar carries both machines on the statics the validator already reads.
    #[test]
    fn the_pile_machines_are_validated_and_shipped_on_the_statics() {
        let l = loaded();
        let bank = &l.encounter.statics[0];
        let post = &l.encounter.statics[1];
        let slide = bank.slide.expect("bank A carries its slide");
        assert_eq!(slide.config.stable, bank.rect);
        assert_eq!(
            (
                slide.config.warning_ticks,
                slide.config.slide_ticks,
                slide.config.dx_per_tick
            ),
            (24, 16, -3),
            "HRD-Q21"
        );
        assert_eq!(
            (
                slide.push.damage,
                slide.push.push_x_px_s,
                slide.push.knock_up_px_s
            ),
            (0, -180, 160),
            "HRD-Q23"
        );
        let stack = post.stack.expect("the post carries its stack").config;
        assert_eq!(
            (stack.release_weight, stack.warning_ticks, stack.fall_ticks),
            (2, 24, 8)
        );
        assert_eq!(
            (stack.damage, stack.knock_x_px_s, stack.knock_up_px_s),
            (1, 120, 140)
        );
        assert_eq!(
            stack.footprint,
            crate::camera_lab::room::IRect::new(752, 532, 128, 48)
        );
        let text = std::fs::read_to_string(root().join(ENCOUNTER_PATH)).unwrap();
        let ctx = crate::encounter::Context {
            room: l.room.clone(),
            classes: l.classes,
            ceiling_px: l.ceiling_px,
        };
        let check = |edit: &dyn Fn(&mut Value)| {
            let mut v: Value = serde_json::from_str(&text).unwrap();
            edit(&mut v);
            crate::encounter::parse_sidecar(&v.to_string(), &ctx)
                .expect_err("the edited sidecar must be rejected")
        };
        let invalid = |errs: &[crate::encounter::EncounterError]| {
            errs.iter()
                .any(|e| matches!(e, crate::encounter::EncounterError::StaticInvalid { .. }))
        };
        // A slide on a hit target, a stack on a solid.
        assert!(invalid(&check(&|v| {
            v["statics"][1]["slide"] = v["statics"][0]["slide"].clone();
        })));
        assert!(invalid(&check(&|v| {
            v["statics"][0]["stack"] = v["statics"][1]["stack"].clone();
        })));
        // A remnant taller than a step.
        assert!(invalid(&check(&|v| {
            v["statics"][0]["slide"]["hrdq21_remnant"] =
                serde_json::json!({"x":296,"y":548,"w":80,"h":32});
        })));
        // The warning under the fairness floor (HRD-Q38: at least 18; the machine's floor is 24).
        assert!(invalid(&check(&|v| {
            v["statics"][0]["slide"]["hrdq21_warning_ticks"] = 10.into();
        })));
        assert!(invalid(&check(&|v| {
            v["statics"][1]["stack"]["hrdq27_warning_ticks"] = 10.into();
        })));
        // The moving mass leaving the remnant, a remnant on the protected spawn.
        assert!(invalid(&check(&|v| {
            v["statics"][0]["slide"]["hrdq21_dx_per_tick"] = (-6).into();
        })));
        assert!(invalid(&check(&|v| {
            v["statics"][0]["slide"]["hrdq21_remnant"] =
                serde_json::json!({"x":140,"y":564,"w":80,"h":16});
            v["statics"][0]["slide"]["hrdq21_dx_per_tick"] = (-1).into();
        })));
        // A stack footprint that does not cover the hanging stack.
        assert!(invalid(&check(&|v| {
            v["statics"][1]["stack"]["hrdq25_footprint"] =
                serde_json::json!({"x":800,"y":532,"w":64,"h":48});
        })));
    }

    #[test]
    fn the_sidecar_schema_rejects_a_misplaced_static() {
        let l = loaded();
        let text = std::fs::read_to_string(root().join(ENCOUNTER_PATH)).unwrap();
        let ctx = crate::encounter::Context {
            room: l.room.clone(),
            classes: l.classes,
            ceiling_px: l.ceiling_px,
        };
        let check = |edit: &dyn Fn(&mut Value)| {
            let mut v: Value = serde_json::from_str(&text).unwrap();
            edit(&mut v);
            crate::encounter::parse_sidecar(&v.to_string(), &ctx)
                .expect_err("the edited sidecar must be rejected")
        };
        // Too tall for a lane obstacle.
        let errs = check(&|v| {
            v["statics"][0]["rect"] = serde_json::json!({"x":352,"y":512,"w":64,"h":68})
        });
        assert!(
            errs.iter()
                .any(|e| matches!(e, crate::encounter::EncounterError::StaticInvalid { .. }))
        );
        // On the protected spawn.
        let errs = check(&|v| {
            v["statics"][0]["rect"] = serde_json::json!({"x":100,"y":532,"w":64,"h":48})
        });
        assert!(
            errs.iter()
                .any(|e| matches!(e, crate::encounter::EncounterError::StaticInvalid { .. }))
        );
        // A static solid is in every reachability state: one wider than a
        // class can jump cuts the lane for it (it is not a perch to step over).
        let errs = check(&|v| {
            v["statics"][0]["rect"] = serde_json::json!({"x":352,"y":532,"w":480,"h":48});
        });
        assert!(
            errs.iter().any(|e| matches!(
                e,
                crate::encounter::EncounterError::LaneCut { .. }
                    | crate::encounter::EncounterError::ExitUnreachable { .. }
            )),
            "{errs:?}"
        );
        // A hit target is never a solid: the same rectangle as a post is fine.
        let ok = {
            let mut v: Value = serde_json::from_str(&text).unwrap();
            v["statics"][0].as_object_mut().unwrap().remove("slide");
            v["statics"][0]["kind"] = "hit_target".into();
            v["statics"][0]["rect"] = serde_json::json!({"x":352,"y":532,"w":480,"h":48});
            crate::encounter::parse_sidecar(&v.to_string(), &ctx)
        };
        assert!(ok.is_ok(), "{ok:?}");
        // Unknown kind.
        let errs = check(&|v| v["statics"][0]["kind"] = "ghost".into());
        assert!(
            errs.iter()
                .any(|e| matches!(e, crate::encounter::EncounterError::StaticInvalid { .. }))
        );
        // Without the lane ceiling the 48 px bank asks for one (DP-Q45).
        let no_ceiling = crate::encounter::Context {
            ceiling_px: None,
            ..ctx.clone()
        };
        let errs = crate::encounter::parse_sidecar(&text, &no_ceiling).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, crate::encounter::EncounterError::CeilingMissing { .. }))
        );
        // A room that grants anything is still refused.
        let errs = check(&|v| v["grants"] = serde_json::json!(["special_thrown_charge"]));
        assert!(errs.contains(&crate::encounter::EncounterError::GrantsNotEmpty));
    }

    // ------------------------------------------------- reach (section 4) ---

    /// Run an idle hero through one swipe and report the attack tick of the
    /// blow, if any.
    fn swipe_hit_tick(hero: impl Fn(f32) -> [HeroView; 2], d: f32) -> Option<usize> {
        let mut boss = awake(E);
        let heroes = hero(E - d);
        boss.force_swipe(&ctx(heroes, &[], 1));
        let outs = run(&mut boss, heroes, &[], 100);
        outs.iter().position(|o| !o.blows.is_empty())
    }

    #[test]
    fn the_swipe_reach_table_is_reproduced() {
        // Carl (half width 12): pivot distance 31..92; Donut (10): 33..90.
        // A forced swipe from her pivot (beyond 88 px she would walk first).
        for d in 0..=140 {
            let carl_hit = swipe_hit_tick(heroes_carl, d as f32).is_some();
            let donut_hit = swipe_hit_tick(heroes_donut, d as f32).is_some();
            assert_eq!(carl_hit, (31..=92).contains(&d), "Carl at {d}");
            assert_eq!(donut_hit, (33..=90).contains(&d), "Donut at {d}");
        }
    }

    #[test]
    fn hrd_swipe_hits_idle_carl_on_attack_tick_37() {
        let mut boss = awake(E);
        let outs = run(&mut boss, heroes_carl(E - 60.0), &[], 100);
        // Scene tick s is attack tick s - 1: the tell is chosen on tick 1.
        let tick = outs.iter().position(|o| !o.blows.is_empty()).unwrap();
        assert_eq!(tick, 37);
        let blow = outs[tick].blows[0];
        assert_eq!(blow.attack, PileAttack::Swipe);
        assert_eq!(blow.blow.damage, 2);
        assert_eq!(blow.blow.knock_vx, -160.0, "away from her pivot, -x here");
        assert_eq!(blow.blow.knock_vy, 200.0);
        assert_eq!(blow.blow.hero, HeroId::Carl);
        // One blow per hero per swipe, across all six impact ticks.
        assert_eq!(outs.iter().map(|o| o.blows.len()).sum::<usize>(), 1);
        // Phase edges: tell 0..33, smear 34..36, impact 37..42, recover 43..90.
        let mut boss = awake(E);
        let modes: Vec<PileMode> = (1..=125)
            .map(|t| {
                boss.step(&ctx(heroes_carl(E - 60.0), &[], t));
                boss.mode()
            })
            .collect();
        let at = |a: usize| modes[a];
        assert_eq!(at(0), PileMode::SwipeTell);
        assert_eq!(at(33), PileMode::SwipeTell);
        assert_eq!(at(34), PileMode::SwipeSmear);
        assert_eq!(at(36), PileMode::SwipeSmear);
        assert_eq!(at(37), PileMode::SwipeImpact);
        assert_eq!(at(42), PileMode::SwipeImpact);
        assert_eq!(at(43), PileMode::SwipeRecover);
        assert_eq!(at(90), PileMode::SwipeRecover);
        assert_eq!(at(91), PileMode::Cooldown);
        assert_eq!(at(120), PileMode::Cooldown);
        assert_ne!(at(121), PileMode::Cooldown, "Neutral from attack tick 121");
    }

    #[test]
    fn the_swipe_hits_both_heroes_once_each() {
        let mut boss = awake(E);
        let heroes = [carl(E - 50.0), donut(E - 70.0)];
        let outs = run(&mut boss, heroes, &[], 100);
        let hurt: Vec<HeroId> = outs
            .iter()
            .flat_map(|o| &o.blows)
            .map(|b| b.blow.hero)
            .collect();
        assert_eq!(hurt.len(), 2);
        assert!(hurt.contains(&HeroId::Carl) && hurt.contains(&HeroId::Donut));
    }

    /// A Carl simulation on the real hero integrator.
    struct CarlSim {
        hero: Hero,
        blocks: Vec<crate::prototype::Block>,
    }

    impl CarlSim {
        fn new(x: f32) -> Self {
            Self {
                hero: Hero::at(x, FLOOR + HERO_HALF_HEIGHT),
                blocks: vec![crate::prototype::Block {
                    rect: rect(0.0, FLOOR - 60.0, 4000.0, 60.0),
                    material: Material::Stone,
                    intact: true,
                }],
            }
        }

        fn step(&mut self, jump: bool) {
            let input = InputFrame {
                jump_pressed: jump,
                ..InputFrame::default()
            };
            crate::prototype::step_hero_driven(
                &mut self.hero,
                &input,
                &self.blocks,
                4000.0,
                &BodyDrive::default(),
            );
        }

        fn view(&self) -> HeroView {
            let foot = self.hero.y - HERO_HALF_HEIGHT;
            view(HeroId::Carl, self.hero.x, foot, 12.0, 52.0)
        }
    }

    /// Run a scripted Carl (jump pressed on attack tick `press`) against one
    /// attack started on scene tick 1; report whether any blow landed. The
    /// hero's step runs before hers in a tick.
    fn jump_run(
        d: f32,
        press: Option<usize>,
        forced_shove: bool,
        ticks: usize,
    ) -> (bool, f32, PileBoss) {
        let mut boss = awake(E);
        let mut sim = CarlSim::new(E - d);
        let mut landed = false;
        for s in 1..=ticks {
            sim.step(press == Some(s - 1));
            let c = ctx([sim.view(), absent(HeroId::Donut)], &[], s as u64);
            let out = if forced_shove && s == 1 {
                boss.force_shove(&c);
                boss.step(&c)
            } else {
                boss.step(&c)
            };
            landed |= !out.blows.is_empty();
            for (_, x) in out.shifts {
                sim.hero.x = x;
            }
        }
        (landed, sim.hero.x, boss)
    }

    #[test]
    fn hrd_swipe_jump_edges_on_the_real_integrator() {
        // Carl at d 60: a jump pressed on attack tick 0 is too early (he is
        // back down for the slam), 1..30 clear it, 31 is too late.
        for (press, hit) in [(0, true), (1, false), (30, false), (31, true)] {
            let (landed, _, _) = jump_run(60.0, Some(press), false, 100);
            assert_eq!(landed, hit, "jump on attack tick {press}");
        }
        // No jump at all: hit.
        assert!(jump_run(60.0, None, false, 100).0);
    }

    #[test]
    fn the_shove_reach_table_is_reproduced() {
        // A forced shove at a standing hero: the box front reaches 48 + 3k.
        // Carl is hit on lunge tick k when 48 + 3k > d - 12.
        for (d, lunge_tick) in [(96.0, 13usize), (128.0, 23), (160.0, 34), (176.0, 39)] {
            let mut boss = awake(E);
            let heroes = heroes_carl(E - d);
            let c = ctx(heroes, &[], 1);
            boss.force_shove(&c);
            let mut tick = None;
            for s in 1..=140u64 {
                let out = boss.step(&ctx(heroes, &[], s));
                if !out.blows.is_empty() {
                    tick = Some(s as usize - 1);
                    break;
                }
            }
            // Scene tick s is attack tick s - 1; lunge tick k is attack tick 39 + k.
            assert_eq!(tick, Some(39 + lunge_tick), "Carl at {d}");
        }
    }

    #[test]
    fn hrd_shove_out_of_reach_as_a_forced_shove() {
        // Carl at 203 is hit and 204 is clear; Donut at 201 hit, 202 clear.
        for (d, hit) in [(203.0, true), (204.0, false)] {
            let mut boss = awake(E);
            let heroes = heroes_carl(E - d);
            boss.force_shove(&ctx(heroes, &[], 1));
            let outs = run(&mut boss, heroes, &[], 140);
            assert_eq!(outs.iter().any(|o| !o.blows.is_empty()), hit, "Carl {d}");
        }
        for (d, hit) in [(201.0, true), (202.0, false)] {
            let mut boss = awake(E);
            let heroes = heroes_donut(E - d);
            boss.force_shove(&ctx(heroes, &[], 1));
            let outs = run(&mut boss, heroes, &[], 140);
            assert_eq!(outs.iter().any(|o| !o.blows.is_empty()), hit, "Donut {d}");
        }
    }

    #[test]
    fn hrd_shove_hits_idle_carl_and_the_lunge_continues() {
        let mut boss = awake(E);
        let heroes = heroes_carl(E - 128.0);
        let outs = run(&mut boss, heroes, &[], 100);
        let tick = outs.iter().position(|o| !o.blows.is_empty()).unwrap();
        // The shove is chosen at 128 px (96..176): attack tick 62 is lunge 23.
        assert_eq!(tick, 62);
        let b = outs[tick].blows[0];
        assert_eq!(b.attack, PileAttack::Shove);
        assert_eq!(b.blow.damage, 1);
        assert_eq!(b.blow.knock_vx, -300.0, "in the shove direction, -x here");
        assert_eq!(b.blow.knock_vy, 180.0);
        // The lunge continues to 144 px, then Recover 48.
        assert_eq!(boss.x(), E - 144.0);
        assert_eq!(boss.mode(), PileMode::ShoveRecover);
        // One hit only.
        assert_eq!(outs.iter().map(|o| o.blows.len()).sum::<usize>(), 1);
    }

    #[test]
    fn hrd_shove_jump_edges_on_the_real_integrator() {
        // Carl at d 128, jump on attack tick 37 / 38 / 55 / 56: hit / clear /
        // clear / hit.
        for (press, hit) in [(37, true), (38, false), (55, false), (56, true)] {
            let (landed, _, _) = jump_run(128.0, Some(press), false, 130);
            assert_eq!(landed, hit, "jump on attack tick {press}");
        }
    }

    /// The three hero columns of the section 4 reach tables, on the real
    /// hero integrator: Carl (jump by the press edge, 185 px/s run), the base
    /// Donut (launch 500 px/s) and the boosted Donut (547 px/s), both running
    /// at 207 px/s with the forgiving 20 x 24 hurtbox.
    #[derive(Clone, Copy, Debug)]
    enum Kind {
        Carl,
        DonutBase,
        DonutBoosted,
    }

    #[derive(Clone, Copy, Debug)]
    enum Act {
        Jump,
        /// Run away from her (toward -x here) from this tick on.
        RunAway,
    }

    struct ReachSim {
        hero: Hero,
        kind: Kind,
        blocks: Vec<crate::prototype::Block>,
        running: bool,
    }

    impl ReachSim {
        fn new(kind: Kind, x: f32) -> Self {
            let sim = CarlSim::new(x);
            Self {
                hero: sim.hero,
                kind,
                blocks: sim.blocks,
                running: false,
            }
        }

        fn step(&mut self, act: Option<Act>) {
            let mut input = InputFrame::default();
            let mut drive = BodyDrive::default();
            match (act, self.kind) {
                (Some(Act::Jump), Kind::Carl) => input.jump_pressed = true,
                (Some(Act::Jump), Kind::DonutBase) => drive.launch = Some((0.0, 500.0)),
                (Some(Act::Jump), Kind::DonutBoosted) => drive.launch = Some((0.0, 547.0)),
                (Some(Act::RunAway), _) => self.running = true,
                (None, _) => {}
            }
            if self.running {
                match self.kind {
                    Kind::Carl => input.held.move_x = -1.0,
                    _ => drive.vx = Some(-207.0),
                }
            }
            crate::prototype::step_hero_driven(
                &mut self.hero,
                &input,
                &self.blocks,
                4000.0,
                &drive,
            );
        }

        fn view(&self) -> HeroView {
            let foot = self.hero.y - HERO_HALF_HEIGHT;
            match self.kind {
                Kind::Carl => view(HeroId::Carl, self.hero.x, foot, 12.0, 52.0),
                _ => view(HeroId::Donut, self.hero.x, foot, 10.0, 24.0),
            }
        }
    }

    /// One natural attack from scene tick 1 at pivot distance `d`; the action
    /// is taken on the hero step of attack tick `at`. Whether any blow landed.
    fn reach_run(kind: Kind, d: f32, at: usize, act: Act, ticks: usize) -> bool {
        let mut boss = awake(E);
        let mut sim = ReachSim::new(kind, E - d);
        let mut landed = false;
        for s in 1..=ticks {
            sim.step((s - 1 == at).then_some(act));
            let view = sim.view();
            let heroes = match kind {
                Kind::Carl => [view, absent(HeroId::Donut)],
                _ => [absent(HeroId::Carl), view],
            };
            let out = boss.step(&ctx(heroes, &[], s as u64));
            landed |= !out.blows.is_empty();
            for (_, x) in out.shifts {
                sim.hero.x = x;
            }
        }
        landed
    }

    /// Assert the edge pairs of one table row: the action is clear on
    /// `first..=last` and hits one tick outside on each side.
    fn assert_window(kind: Kind, d: f32, act: Act, first: usize, last: usize, ticks: usize) {
        for at in [first.saturating_sub(1), first, last, last + 1] {
            let clear = at >= first && at <= last;
            assert_eq!(
                !reach_run(kind, d, at, act, ticks),
                clear,
                "{kind:?} d {d} {act:?} on attack tick {at}: window {first}..={last}"
            );
        }
        if first > 0 {
            assert!(
                reach_run(kind, d, first - 1, act, ticks),
                "{kind:?} d {d}: one tick before {first} hits"
            );
        }
    }

    #[test]
    fn the_swipe_reach_table_jump_windows_for_all_three_columns() {
        // Jump in place at d 44 and d 60: Carl 1..30, Donut base 0..31,
        // boosted 0..32. 100 ticks: her second swipe (after the Cooldown)
        // would otherwise hit the landed hero and mask the window.
        for d in [44.0, 60.0] {
            assert_window(Kind::Carl, d, Act::Jump, 1, 30, 100);
            assert_window(Kind::DonutBase, d, Act::Jump, 0, 31, 100);
            assert_window(Kind::DonutBoosted, d, Act::Jump, 0, 32, 100);
        }
    }

    #[test]
    fn the_swipe_reach_table_step_back_rows() {
        // Running away from d 44 must start by attack tick 22 (Carl, 185 px/s)
        // or 24 (Donut, 207 px/s); one tick later the slam still lands.
        for (kind, last) in [
            (Kind::Carl, 22),
            (Kind::DonutBase, 24),
            (Kind::DonutBoosted, 24),
        ] {
            assert!(
                !reach_run(kind, 44.0, last, Act::RunAway, 100),
                "{kind:?} clears on tick {last}"
            );
            assert!(
                reach_run(kind, 44.0, last + 1, Act::RunAway, 100),
                "{kind:?} is hit on tick {}",
                last + 1
            );
        }
    }

    #[test]
    fn the_shove_reach_table_jump_windows_for_all_three_columns() {
        // (d, Carl, Donut base, Donut boosted) first..=last clearing press.
        type Window = (usize, usize);
        let rows: [(f32, Window, Window, Window); 4] = [
            (96.0, (28, 45), (19, 46), (13, 47)),
            (128.0, (38, 55), (30, 57), (24, 58)),
            (160.0, (47, 66), (39, 68), (33, 69)),
            (176.0, (47, 71), (39, 73), (33, 74)),
        ];
        for (d, carl, base, boosted) in rows {
            assert_window(Kind::Carl, d, Act::Jump, carl.0, carl.1, 130);
            assert_window(Kind::DonutBase, d, Act::Jump, base.0, base.1, 130);
            assert_window(Kind::DonutBoosted, d, Act::Jump, boosted.0, boosted.1, 130);
        }
    }

    #[test]
    fn hrd_shove_jump_lands_behind_her_and_she_turns() {
        let (landed, x, boss) = jump_run(128.0, Some(45), false, 130);
        assert!(!landed);
        // She ended at E - 144 and Carl came down behind her (+x side): the
        // push-out moved him to her back edge.
        assert_eq!(boss.x(), E - 144.0);
        assert!(x > boss.x(), "behind her: {x} vs {}", boss.x());
        assert!(x >= boss.x() + 28.0, "pushed out of her body: {x}");
        // After Recover and Cooldown she turns before her next tell.
        let mut boss = boss;
        let heroes = heroes_carl(x);
        let mut first = None;
        for s in 0..200u64 {
            boss.step(&ctx(heroes, &[], 200 + s));
            if matches!(
                boss.mode(),
                PileMode::Turn | PileMode::SwipeTell | PileMode::ShoveTell
            ) {
                first = Some(boss.mode());
                break;
            }
        }
        assert_eq!(first, Some(PileMode::Turn));
    }

    #[test]
    fn hrd_shove_2p_both_hit_once_one_shove() {
        let mut boss = awake(E);
        let heroes = [carl(E - 112.0), donut(E - 160.0)];
        let outs = run(&mut boss, heroes, &[], 140);
        let blows: Vec<&PileBlow> = outs.iter().flat_map(|o| &o.blows).collect();
        assert_eq!(blows.len(), 2);
        assert!(blows.iter().all(|b| b.attack == PileAttack::Shove));
        let tells = outs
            .iter()
            .flat_map(|o| &o.pile_events)
            .filter(|(_, e)| {
                matches!(
                    e,
                    PileEvent::Tell {
                        attack: PileAttack::Shove,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(tells, 1, "one shove");
    }

    #[test]
    fn the_shove_tell_lunge_and_recover_edges() {
        let mut boss = awake(E);
        let heroes = heroes_carl(E - 128.0);
        let modes: Vec<PileMode> = (1..=300)
            .map(|t| {
                boss.step(&ctx(heroes, &[], t));
                boss.mode()
            })
            .collect();
        // Tell 0..39, lunge from 40, 48 ticks, then Recover 48.
        assert_eq!(modes[0], PileMode::ShoveTell);
        assert_eq!(modes[39], PileMode::ShoveTell);
        assert_eq!(modes[40], PileMode::Lunge);
        assert_eq!(modes[86], PileMode::Lunge);
        assert_eq!(modes[87], PileMode::ShoveRecover);
        assert_eq!(modes[87 + 47], PileMode::ShoveRecover);
        assert_eq!(modes[87 + 48], PileMode::Cooldown);
    }

    // ------------------------------------------------- jam and obstacles ---

    #[test]
    fn hrd_shove_jams_on_bank_a_with_no_motion_backward() {
        // She at 470, Carl behind the bank at x 330 (d 140): the box front
        // would pass the face 416 on lunge tick 3.
        let bank = [bank_a()];
        let mut boss = awake(470.0);
        let heroes = heroes_carl(330.0);
        let mut jam_tick = None;
        let mut lunge_start = None;
        let mut xs = Vec::new();
        for s in 1..=200u64 {
            let out = boss.step(&ctx(heroes, &bank, s));
            if boss.mode() == PileMode::Lunge && lunge_start.is_none() {
                lunge_start = Some(s);
            }
            xs.push(boss.x());
            if !out.jams.is_empty() {
                jam_tick = Some(s);
                assert_eq!(out.jams[0].obstacle, ObstacleKind::BankA);
                assert_eq!(out.jams[0].credit, Some(0), "credited to the bait, Carl");
                assert!(out.blows.is_empty(), "the box stops on the bank");
                break;
            }
        }
        let jam = jam_tick.expect("jammed");
        assert_eq!(jam - lunge_start.unwrap(), 2, "lunge tick 3");
        assert_eq!(boss.x(), 464.0, "box front rests on the face 416");
        assert_eq!(boss.mode(), PileMode::Jammed);
        assert!(xs.windows(2).all(|w| w[1] <= w[0]), "never backward");
        // Jammed lasts 72 ticks, then Cooldown.
        let mut ticks_jammed = 1;
        for s in 1..=100u64 {
            boss.step(&ctx(heroes, &bank, 1000 + s));
            if boss.mode() != PileMode::Jammed {
                break;
            }
            ticks_jammed += 1;
        }
        assert_eq!(ticks_jammed, 72);
        assert_eq!(boss.mode(), PileMode::Cooldown);
        // Armed on the jam tick (181), 72 Jammed ticks and the first Cooldown tick later.
        assert_eq!(boss.shove_cooldown(), 181 - 72);
    }

    #[test]
    fn a_shove_at_a_bot_hero_jams_with_credit_none() {
        let bank = [bank_a()];
        let mut boss = awake(470.0);
        let mut bot = carl(330.0);
        bot.bot = true;
        let mut credit = Some(7);
        for s in 1..=200u64 {
            let out = boss.step(&PileCtx {
                solo: true,
                ..ctx([bot, absent(HeroId::Donut)], &bank, s)
            });
            if let Some(j) = out.jams.first() {
                credit = j.credit;
                break;
            }
        }
        assert_eq!(credit, None);
    }

    #[test]
    fn the_post_is_a_plain_obstacle_for_the_shove_and_no_solid_for_walkers() {
        let post = [post()];
        // She at 880 facing -x with the post at 808..824: the shove jams on its
        // right face; the box front rests at 824.
        let mut boss = awake(880.0);
        let heroes = heroes_carl(700.0);
        let mut jam = None;
        for s in 1..=300u64 {
            let out = boss.step(&ctx(heroes, &post, s));
            if let Some(j) = out.jams.first() {
                jam = Some(j.obstacle);
                break;
            }
        }
        assert_eq!(jam, Some(ObstacleKind::Post));
        assert_eq!(boss.x(), 824.0 + 48.0);
        // A walking hero is not blocked by it, and she walks through it.
        assert!(!post[0].blocks_heroes());
        assert!(!post[0].blocks_her_body());
        let mut walker = awake(840.0);
        for s in 1..=60u64 {
            walker.step(&ctx(heroes_carl(400.0), &post, s));
        }
        assert!(walker.x() < 840.0);
    }

    #[test]
    fn a_remnant_never_stops_her_walk_or_her_shove() {
        let remnant = [Obstacle {
            kind: ObstacleKind::Remnant,
            rect: rect(296.0, FLOOR, 80.0, 16.0),
        }];
        assert!(!remnant[0].stops_shove() && !remnant[0].blocks_her_body());
        assert!(remnant[0].blocks_heroes(), "solid under a hero's feet");
        let mut boss = awake(470.0);
        let heroes = heroes_carl(330.0);
        let mut jams = 0;
        for s in 1..=200u64 {
            jams += boss.step(&ctx(heroes, &remnant, s)).jams.len();
        }
        assert_eq!(jams, 0);
        assert!(
            boss.x() < 400.0,
            "she crosses a 16 px remnant: {}",
            boss.x()
        );
    }

    // ----------------------------------------------- armour, poise, hits ---

    #[test]
    fn armour_per_mode() {
        use HitReaction::*;
        let table: Vec<(PileMode, HitReaction)> =
            PileMode::ALL.iter().map(|m| (*m, m.reaction())).collect();
        for (mode, want) in [
            (PileMode::Dormant, None),
            (PileMode::Reveal, None),
            (PileMode::Defeated, None),
            (PileMode::SwipeTell, Armour),
            (PileMode::SwipeSmear, Armour),
            (PileMode::SwipeImpact, Armour),
            (PileMode::ShoveTell, Armour),
            (PileMode::Lunge, Armour),
            (PileMode::SwipeRecover, Free),
            (PileMode::ShoveRecover, Free),
            (PileMode::Jammed, Free),
            (PileMode::Flinch, Free),
            (PileMode::Exposed, Free),
            (PileMode::GetUp, Free),
            (PileMode::Neutral, Poise),
            (PileMode::Walk, Poise),
            (PileMode::Turn, Poise),
            (PileMode::Cooldown, Poise),
        ] {
            assert!(table.contains(&(mode, want)), "{mode:?}");
        }
        assert_eq!(table.len(), 18);
    }

    #[test]
    fn hrd_armour_tell_damage_lands_and_the_swipe_continues() {
        let mut boss = awake(E);
        let heroes = heroes_carl(E - 44.0);
        for s in 1..=6u64 {
            boss.step(&ctx(heroes, &[], s));
        }
        assert_eq!(boss.mode(), PileMode::SwipeTell);
        let (r, out) = jab(&mut boss, 6);
        assert_eq!(r.unwrap().hp_left, 29);
        assert!(!r.unwrap().flinched);
        assert_eq!(boss.mode(), PileMode::SwipeTell);
        assert!(matches!(
            out.pile_events[0].1,
            PileEvent::Hurt {
                damage: 1,
                hp_left: 29,
                ..
            }
        ));
        // Four hits in the tell never interrupt it.
        for t in 7..=9 {
            jab(&mut boss, t);
        }
        assert_eq!(boss.mode(), PileMode::SwipeTell);
        assert_eq!(boss.hp(), 26);
    }

    #[test]
    fn hrd_swipe_punish_free_damage_has_no_poise() {
        let mut boss = awake(E);
        let heroes = heroes_carl(E - 44.0);
        for s in 1..=60u64 {
            boss.step(&ctx(heroes, &[], s));
        }
        assert_eq!(boss.mode(), PileMode::SwipeRecover);
        for t in 0..6 {
            let (r, _) = jab(&mut boss, 60 + t);
            assert!(!r.unwrap().flinched);
        }
        assert_eq!(boss.mode(), PileMode::SwipeRecover, "never flinches");
        assert_eq!(boss.hp(), 24);
    }

    #[test]
    fn hrd_poise_flinch_on_the_fourth_and_none_for_sixty_ticks() {
        let mut boss = awake(E).with_shove_cooldown(180);
        let heroes = heroes_donut(E - 150.0);
        for s in 1..=4u64 {
            boss.step(&ctx(heroes, &[], s));
            let mut out = PileOut::default();
            let r = boss
                .apply_hit(hit(0, 1), None, s, &mut out)
                .expect("hurtbox");
            assert_eq!(r.flinched, s == 4, "hit {s}");
        }
        assert_eq!(boss.mode(), PileMode::Flinch);
        // A 5th (and more) within 60 ticks adds no poise.
        for t in 5..=40u64 {
            boss.step(&ctx(heroes, &[], t));
            let mut out = PileOut::default();
            let r = boss.apply_hit(hit(0, 1), None, t, &mut out).unwrap();
            assert!(!r.flinched, "tick {t}");
        }
        assert_ne!(boss.mode(), PileMode::Flinch, "Flinch is 12 ticks");
    }

    #[test]
    fn a_swat_adds_poise_without_damage() {
        let mut boss = awake(E);
        let heroes = heroes_donut(E - 300.0);
        boss.step(&ctx(heroes, &[], 1));
        for t in 0..4u64 {
            let mut out = PileOut::default();
            let r = boss
                .apply_hit(hit(0, 1), None, 2 + t, &mut out)
                .expect("hurtbox");
            assert_eq!(r.hp_left, 30);
            assert!(
                out.pile_events
                    .iter()
                    .all(|(_, e)| !matches!(e, PileEvent::Hurt { .. }))
            );
            assert_eq!(r.flinched, t == 3);
        }
    }

    #[test]
    fn flinch_lasts_twelve_ticks_then_cooldown() {
        let mut boss = awake(E);
        let heroes = heroes_donut(E - 300.0);
        boss.step(&ctx(heroes, &[], 1));
        // Scene order of one tick T: the hits land, then her step runs.
        let flinch_tick = 2u64;
        let mut out = PileOut::default();
        for _ in 0..4 {
            boss.apply_hit(hit(1, 1), None, flinch_tick, &mut out);
        }
        assert!(out.pile_events.iter().any(|(_, e)| *e == PileEvent::Flinch));
        let mut modes = Vec::new();
        for t in flinch_tick..flinch_tick + 14 {
            boss.step(&ctx(heroes, &[], t));
            modes.push(boss.mode());
        }
        // Flinch on exactly T..T+11 (12 ticks), Cooldown from T+12.
        assert!(
            modes[..12].iter().all(|m| *m == PileMode::Flinch),
            "{modes:?}"
        );
        assert_eq!(modes[12], PileMode::Cooldown);
        assert_eq!(modes[13], PileMode::Cooldown);
    }

    #[test]
    fn no_hurtbox_while_dormant_in_reveal_or_defeated() {
        let mut boss = PileBoss::new(tun(), setup());
        assert!(boss.hurt_rects().is_empty());
        let mut out = PileOut::default();
        assert!(boss.apply_hit(hit(1, 1), None, 1, &mut out).is_none());
        assert_eq!(boss.hp(), 30);
    }

    // -------------------------------------------------- body and push-out ---

    #[test]
    fn the_body_box_per_mode() {
        let t = tun();
        let mut boss = awake(E);
        let standing = rect(E - 28.0, FLOOR, 56.0, 104.0);
        for mode in [
            PileMode::Neutral,
            PileMode::Walk,
            PileMode::Turn,
            PileMode::SwipeTell,
            PileMode::SwipeSmear,
            PileMode::SwipeImpact,
            PileMode::SwipeRecover,
            PileMode::ShoveTell,
            PileMode::ShoveRecover,
            PileMode::Jammed,
            PileMode::Flinch,
            PileMode::Cooldown,
        ] {
            boss.mode = mode;
            assert_eq!(boss.body_rect(), Some(standing), "{mode:?}");
            let hurt = boss.hurt_rects();
            assert_eq!(hurt.len(), 2, "{mode:?}: body plus head");
            assert_eq!(hurt[0], standing);
            // Head [1, 21) x [88, 110), forward is -x here.
            assert_eq!(hurt[1], rect(E - 21.0, FLOOR + 88.0, 20.0, 22.0));
        }
        // The lunge: not solid, pushes nobody; the hurtbox stays.
        boss.mode = PileMode::Lunge;
        assert_eq!(boss.body_rect(), None);
        assert_eq!(boss.hurt_rects().len(), 2);
        // Exposed and Get-up: the prone box [-48, 72) x [0, 48), forward -x.
        for mode in [PileMode::Exposed, PileMode::GetUp] {
            boss.mode = mode;
            let prone = rect(E - 72.0, FLOOR, 120.0, 48.0);
            assert_eq!(boss.body_rect(), Some(prone));
            assert_eq!(boss.hurt_rects(), vec![prone]);
        }
        // Dormant and Defeated: none. Reveal: none until its last tick.
        for mode in [PileMode::Dormant, PileMode::Defeated] {
            boss.mode = mode;
            assert_eq!(boss.body_rect(), None);
            assert!(boss.hurt_rects().is_empty());
        }
        boss.mode = PileMode::Reveal;
        boss.mode_tick = u32::from(t.hrdq05_reveal_ticks) - 1;
        assert_eq!(boss.body_rect(), None);
        boss.mode_tick = u32::from(t.hrdq05_reveal_ticks);
        assert_eq!(boss.body_rect(), Some(standing));
        assert!(boss.hurt_rects().is_empty(), "no hurtbox in Reveal");
        // Mirrored by the facing: facing +x puts the prone box's long side +x.
        let mut east = PileBoss::awake_at(tun(), setup(), E, 1);
        east.mode = PileMode::Exposed;
        assert_eq!(east.body_rect(), Some(rect(E - 48.0, FLOOR, 120.0, 48.0)));
    }

    fn push_once(
        boss: &PileBoss,
        heroes: [HeroView; 2],
        obstacles: &[Obstacle],
    ) -> Vec<(HeroId, f32)> {
        let mut out = PileOut::default();
        let mut b = boss.clone();
        b.suppress_push = false;
        b.push_heroes(&ctx(heroes, obstacles, 1), &mut out);
        out.shifts
    }

    #[test]
    fn push_out_goes_to_the_nearer_edge_at_six_px_per_tick() {
        let boss = awake(E);
        // Carl 4 px left of her pivot: nearer edge is the left one (E-28-13).
        let shifts = push_once(&boss, heroes_carl(E - 4.0), &[]);
        assert_eq!(shifts, [(HeroId::Carl, E - 10.0)]);
        let shifts = push_once(&boss, heroes_carl(E + 4.0), &[]);
        assert_eq!(shifts, [(HeroId::Carl, E + 10.0)]);
        // A hero above her head (feet at the body top) is not pushed.
        let high = view(HeroId::Carl, E, FLOOR + 104.0, 12.0, 52.0);
        assert!(push_once(&boss, [high, absent(HeroId::Donut)], &[]).is_empty());
        // Outside the body: nothing.
        assert!(push_once(&boss, heroes_carl(E - 60.0), &[]).is_empty());
    }

    #[test]
    fn push_out_takes_the_other_side_when_the_near_side_is_cramped() {
        // A full-height wall 20 px left of her body: fewer than 28 free, so a
        // hero on the near (left) side goes right.
        let wall = [Obstacle {
            kind: ObstacleKind::Solid,
            rect: rect(E - 28.0 - 20.0 - 10.0, FLOOR, 10.0, 228.0),
        }];
        let boss = awake(E);
        let shifts = push_once(&boss, heroes_carl(E - 10.0), &wall);
        assert_eq!(shifts.len(), 1);
        assert!(shifts[0].1 > E - 10.0, "went right: {}", shifts[0].1);
        // Without the wall he goes left.
        let shifts = push_once(&boss, heroes_carl(E - 10.0), &[]);
        assert!(shifts[0].1 < E - 10.0);
    }

    #[test]
    fn push_out_never_goes_into_a_solid() {
        // Both sides closed in by walls: nobody is moved into them.
        let walls = [
            Obstacle {
                kind: ObstacleKind::Solid,
                rect: rect(E - 28.0 - 14.0 - 10.0, FLOOR, 10.0, 228.0),
            },
            Obstacle {
                kind: ObstacleKind::Solid,
                rect: rect(E + 28.0 + 14.0, FLOOR, 10.0, 228.0),
            },
        ];
        let boss = awake(E);
        assert!(push_once(&boss, heroes_carl(E - 30.0), &walls).is_empty());
    }

    #[test]
    fn push_out_on_the_bank_top_does_not_count_the_bank_as_a_solid() {
        // N3: Carl at x 427 on bank A's top (feet 48) when she arrives at 444:
        // the bank under his feet is not a solid of his height band, so the
        // nearer (left) side is free and he ends at x 404 or less, still on
        // the top.
        let bank = [bank_a()];
        let boss = awake(444.0);
        let mut carl_view = view(HeroId::Carl, 427.0, FLOOR + 48.0, 12.0, 52.0);
        let mut x = 427.0;
        for _ in 0..8 {
            carl_view.x = x;
            carl_view.hurt = rect(x - 12.0, FLOOR + 48.0, 24.0, 52.0);
            let shifts = push_once(&boss, [carl_view, absent(HeroId::Donut)], &bank);
            match shifts.first() {
                Some((_, to)) => x = *to,
                None => break,
            }
        }
        assert!(x <= 404.0, "ended at {x}");
        assert!(x > 352.0, "still over the bank");
        assert!(
            x + 12.0 <= 416.0 + 12.0 && x - 12.0 >= 352.0 - 12.0,
            "on the top"
        );
    }

    #[test]
    fn the_lunge_pushes_nobody_but_the_first_tick_after_it_does() {
        let (landed, x, boss) = jump_run(128.0, Some(45), false, 88);
        assert!(!landed);
        // On the last lunge tick (87th step) she is already in Recover but the
        // push-out waits for the next tick.
        assert_eq!(boss.mode(), PileMode::ShoveRecover);
        assert!(
            x > boss.x(),
            "Carl is under/behind her, not yet pushed: {x}"
        );
        assert!(x < boss.x() + 28.0);
    }

    // ---------------------------------------------------------- exposure ---

    #[test]
    fn the_exposure_floors_hp_and_is_never_lethal() {
        let mut boss = awake(E);
        boss.hp = 4;
        let mut out = PileOut::default();
        assert!(boss.expose(Some(0), 10, &mut out));
        assert_eq!(boss.hp(), 1, "6 damage floors at 1");
        assert_eq!(boss.mode(), PileMode::Exposed);
        assert!(
            out.pile_events
                .iter()
                .any(|(_, e)| matches!(e, PileEvent::Exposed { credit: Some(0) }))
        );
        assert!(
            out.events.is_empty() && out.story_trigger.is_none(),
            "never a defeat"
        );
        // Again: HP stays 1, no hurt event for 0 damage.
        let mut out = PileOut::default();
        boss.expose(None, 11, &mut out);
        assert_eq!(boss.hp(), 1);
        assert!(
            !out.pile_events
                .iter()
                .any(|(_, e)| matches!(e, PileEvent::Hurt { .. }))
        );
        // A full-health landing does 6.
        let mut boss = awake(E);
        boss.expose(None, 1, &mut PileOut::default());
        assert_eq!(boss.hp(), 24);
    }

    #[test]
    fn exposed_then_get_up_then_cooldown_and_the_finishing_blow_is_a_heros() {
        let mut boss = awake(E);
        boss.expose(Some(0), 1, &mut PileOut::default());
        let heroes = heroes_donut(E - 300.0);
        let mut seq = vec![(boss.mode(), 1u32)];
        for s in 1..=400u64 {
            boss.step(&ctx(heroes, &[], s));
            match seq.last_mut() {
                Some((m, n)) if *m == boss.mode() => *n += 1,
                _ => seq.push((boss.mode(), 1)),
            }
            if boss.mode() == PileMode::Neutral {
                break;
            }
        }
        assert_eq!(seq[0], (PileMode::Exposed, 150));
        assert_eq!(seq[1], (PileMode::GetUp, 24));
        assert_eq!(seq[2], (PileMode::Cooldown, 30));
        // Free damage while Exposed, and a hero's blow can finish her there.
        let mut boss = awake(E);
        boss.hp = 3;
        boss.expose(None, 1, &mut PileOut::default());
        assert_eq!(boss.hp(), 1);
        let mut out = PileOut::default();
        let r = boss.apply_hit(hit(1, 1), None, 2, &mut out).unwrap();
        assert!(r.defeated);
    }

    #[test]
    fn the_exposure_overrides_the_lunge_and_needs_a_body_to_land_on() {
        let mut boss = awake(E);
        let heroes = heroes_carl(E - 128.0);
        for s in 1..=50u64 {
            boss.step(&ctx(heroes, &[], s));
        }
        assert_eq!(boss.mode(), PileMode::Lunge);
        assert!(boss.expose(None, 51, &mut PileOut::default()));
        assert_eq!(boss.mode(), PileMode::Exposed);
        let mut dormant = PileBoss::new(tun(), setup());
        assert!(!dormant.expose(None, 1, &mut PileOut::default()));
        assert_eq!(dormant.hp(), 30);
    }

    // ------------------------------------------------------------ defeat ---

    #[test]
    fn defeat_emits_the_existing_event_and_the_story_trigger() {
        let mut boss = awake(E);
        boss.hp = 1;
        let mut out = PileOut::default();
        let r = boss
            .apply_hit(
                PileHit {
                    damage: 1,
                    poise: 1,
                    player: 1,
                    hit_stop: 4,
                    cause: DefeatCause::Melee,
                },
                None,
                42,
                &mut out,
            )
            .unwrap();
        assert!(r.defeated);
        assert_eq!(boss.mode(), PileMode::Defeated);
        assert!(boss.door_open());
        assert!(!boss.alive());
        assert_eq!(boss.gate(), Gate::Released);
        assert_eq!(out.story_trigger, Some("pile_boss_f01_defeated"));
        assert_eq!(out.events.len(), 1, "one defeat event");
        match out.events[0] {
            CombatEvent::EnemyDefeated {
                tick,
                target,
                cause,
                force,
                instigator,
            } => {
                assert_eq!((tick, target), (42, PILE_BOSS_ID));
                assert_eq!(cause, DefeatCause::Melee);
                assert_eq!(force, cause.default_force());
                assert_eq!(instigator, Some(1));
            }
            ref other => panic!("not the defeat event: {other:?}"),
        }
        // Defeated: no body, no hurtbox, no further events, a second hit is
        // refused (so the trigger fires once).
        assert_eq!(boss.body_rect(), None);
        let mut again = PileOut::default();
        assert!(boss.apply_hit(hit(1, 1), None, 43, &mut again).is_none());
        assert!(again.events.is_empty() && again.story_trigger.is_none());
        let after = boss.step(&ctx(heroes_carl(E - 40.0), &[], 44));
        assert_eq!(after, PileOut::default());
    }

    #[test]
    fn a_defeat_during_the_lunge_stops_the_blows() {
        let mut boss = awake(E);
        let heroes = heroes_carl(E - 128.0);
        for s in 1..=45u64 {
            boss.step(&ctx(heroes, &[], s));
        }
        assert_eq!(boss.mode(), PileMode::Lunge);
        boss.hp = 1;
        let mut out = PileOut::default();
        boss.apply_hit(hit(1, 1), None, 46, &mut out);
        let after = run(&mut boss, heroes, &[], 80);
        assert!(after.iter().all(|o| o.blows.is_empty()));
    }

    // ------------------------------------------------------------- gate ---

    fn gate_boss() -> PileBoss {
        PileBoss::new(tun(), setup())
    }

    fn gate_ctx(heroes: [HeroView; 2], solo: bool, tick: u64) -> PileCtx<'static> {
        PileCtx {
            heroes,
            obstacles: &[],
            solo,
            tick,
            bank_a_disturbed: false,
        }
    }

    #[test]
    fn the_gate_closes_eight_ticks_after_the_engage_and_the_reveal_follows() {
        let mut boss = gate_boss();
        assert!(boss.gate_solid().is_none());
        let mut engaged = None;
        let mut reveal_ticks = 0;
        for t in 1..=200u64 {
            // Carl reaches x 432 on tick 5 and stands past 236 inside.
            let x = if t < 5 { 428.0 } else { 440.0 };
            let out = boss.step(&gate_ctx(heroes_carl(x), false, t));
            if out
                .pile_events
                .iter()
                .any(|(_, e)| *e == PileEvent::Engaged)
            {
                engaged = Some(t);
            }
            if boss.mode() == PileMode::Reveal {
                reveal_ticks += 1;
            }
            if engaged.is_some() && boss.mode() != PileMode::Reveal {
                break;
            }
        }
        assert_eq!(engaged, Some(5 + 8));
        assert_eq!(reveal_ticks, 90, "no hurtbox, no tell for 90 ticks");
        assert_ne!(boss.mode(), PileMode::Reveal, "then ordinary choices");
        assert!(boss.gate_solid().is_some());
        let g = boss.gate_solid().unwrap();
        assert_eq!((g.left, g.width), (208.0, 16.0));
        assert_eq!(boss.gate(), Gate::Closed);
    }

    #[test]
    fn the_reveal_has_no_hurtbox_and_no_attack_and_becomes_solid_on_its_last_tick() {
        let mut boss = gate_boss();
        let mut tells = 0;
        for t in 1..=40u64 {
            let out = boss.step(&gate_ctx(heroes_carl(440.0), false, t));
            tells += out
                .pile_events
                .iter()
                .filter(|(_, e)| matches!(e, PileEvent::Tell { .. }))
                .count();
            assert!(out.blows.is_empty());
            if boss.mode() == PileMode::Reveal {
                assert!(boss.hurt_rects().is_empty());
                let mut o = PileOut::default();
                assert!(boss.apply_hit(hit(1, 1), None, t, &mut o).is_none());
            }
        }
        assert_eq!(tells, 0);
        // A hero standing inside her at the last reveal tick is pushed out.
        let mut boss = gate_boss();
        let inside = heroes_carl(608.0);
        let mut shifted_on = None;
        for t in 1..=200u64 {
            let c = if t < 3 {
                gate_ctx(heroes_carl(440.0), false, t)
            } else {
                gate_ctx(inside, false, t)
            };
            let out = boss.step(&c);
            if !out.shifts.is_empty() && shifted_on.is_none() {
                shifted_on = Some((t, boss.mode(), boss.mode_tick()));
            }
        }
        let (_, mode, tick) = shifted_on.expect("pushed out");
        assert_eq!(mode, PileMode::Reveal);
        assert_eq!(tick, 90, "on the reveal's last tick");
    }

    #[test]
    fn hrd_gate_2p_waits_for_the_far_hero() {
        let mut boss = gate_boss();
        let mut engaged = None;
        for t in 1..=300u64 {
            let donut_x = if t < 250 { 150.0 } else { 236.0 };
            let out = boss.step(&gate_ctx([carl(440.0), donut(donut_x)], false, t));
            assert!(out.regroup.is_none(), "two players wait indefinitely");
            if out
                .pile_events
                .iter()
                .any(|(_, e)| *e == PileEvent::Engaged)
            {
                engaged = Some(t);
                break;
            }
            assert_eq!(boss.mode(), PileMode::Dormant, "tick {t}");
            assert!(matches!(boss.gate(), Gate::Open { .. }));
        }
        assert_eq!(engaged, Some(250), "the tick Donut reaches 236");
    }

    #[test]
    fn hrd_gate_solo_regroup_places_the_bot_after_120_ticks() {
        let mut boss = gate_boss();
        let mut bot = donut(150.0);
        bot.bot = true;
        let mut first = None;
        let mut closed = None;
        for t in 1..=400u64 {
            // Carl crosses 432 on tick 1.
            let out = boss.step(&gate_ctx([carl(440.0), bot], true, t));
            if let Some(id) = out.regroup
                && first.is_none()
            {
                assert_eq!(id, HeroId::Donut);
                first = Some(t);
                // The scene places the bot at the regroup x.
                bot = donut(f32::from(boss.tuning().hrdq41_regroup_x));
                bot.bot = true;
            }
            if out
                .pile_events
                .iter()
                .any(|(_, e)| *e == PileEvent::Engaged)
            {
                closed = Some(t);
                break;
            }
        }
        assert_eq!(first, Some(1 + 120));
        assert_eq!(closed, Some(1 + 120 + 1), "the gate closes the next tick");
        // A bot that already stands inside never needs a regroup.
        let mut boss = gate_boss();
        let mut inside = donut(250.0);
        inside.bot = true;
        let mut any_regroup = false;
        for t in 1..=40u64 {
            any_regroup |= boss
                .step(&gate_ctx([carl(440.0), inside], true, t))
                .regroup
                .is_some();
        }
        assert!(!any_regroup);
    }

    #[test]
    fn the_gate_lever_off_still_engages_but_places_no_solid() {
        let mut off = tun().clone();
        off.hrdq41_gate_enabled = false;
        let off: &'static PileBossTuning = Box::leak(Box::new(off));
        let mut boss = PileBoss::new(off, setup());
        let mut engaged = false;
        for t in 1..=40u64 {
            let out = boss.step(&gate_ctx(heroes_carl(440.0), false, t));
            engaged |= out
                .pile_events
                .iter()
                .any(|(_, e)| *e == PileEvent::Engaged);
        }
        assert!(engaged);
        assert_eq!(boss.gate(), Gate::Closed);
        assert!(boss.gate_solid().is_none());
    }

    #[test]
    fn the_gate_opens_on_defeat_and_in_a_fresh_attempt_with_the_short_reveal() {
        let mut boss = gate_boss();
        for t in 1..=120u64 {
            boss.step(&gate_ctx(heroes_carl(440.0), false, t));
        }
        assert!(boss.gate_solid().is_some());
        // Defeat opens it.
        let mut dead = boss.clone();
        dead.hp = 1;
        dead.apply_hit(hit(1, 1), None, 121, &mut PileOut::default());
        assert!(dead.gate_solid().is_none());
        assert_eq!(dead.gate(), Gate::Released);
        // A restart: open again, Dormant, and the reveal is 30 ticks.
        boss.restart();
        assert_eq!(boss.gate(), Gate::Open { crossed: None });
        assert!(boss.gate_solid().is_none());
        assert_eq!(boss.mode(), PileMode::Dormant);
        let mut reveal = 0;
        for t in 1..=200u64 {
            boss.step(&gate_ctx(heroes_carl(440.0), false, t));
            if boss.mode() == PileMode::Reveal {
                reveal += 1;
            }
        }
        assert_eq!(reveal, 30);
        // Restart mid-reveal equals a fresh retry.
        let mut a = gate_boss();
        a.restart();
        let mut b = gate_boss();
        for t in 1..=40u64 {
            b.step(&gate_ctx(heroes_carl(440.0), false, t));
        }
        b.restart();
        assert_eq!(state_hash(&a), state_hash(&b));
    }

    // -------------------------------------------------------- targeting ---

    #[test]
    fn the_target_is_locked_at_the_tell_and_picked_by_hysteresis_and_bias() {
        // Nearest wins; a switch needs 24 px.
        let mut boss = awake(E);
        let heroes = [carl(E - 300.0), donut(E - 360.0)];
        boss.step(&ctx(heroes, &[], 1));
        assert_eq!(boss.target(), Some(HeroId::Carl));
        let heroes = [carl(E - 330.0), donut(E - 310.0)];
        boss.step(&ctx(heroes, &[], 2));
        assert_eq!(boss.target(), Some(HeroId::Carl), "inside the hysteresis");
        let heroes = [carl(E - 360.0), donut(E - 310.0)];
        boss.step(&ctx(heroes, &[], 3));
        assert_eq!(boss.target(), Some(HeroId::Donut));
        // The solo bias: the bot counts 40 px farther.
        let mut boss = awake(E);
        let mut bot = donut(E - 100.0);
        bot.bot = true;
        let human = carl(E - 130.0);
        boss.step(&PileCtx {
            solo: true,
            ..ctx([human, bot], &[], 1)
        });
        assert_eq!(boss.target(), Some(HeroId::Carl), "100 + 40 > 130");
        // Locked at the tell start: heroes moving during the tell change nothing.
        let mut boss = awake(E);
        let heroes = [carl(E - 70.0), donut(E - 400.0)];
        boss.step(&ctx(heroes, &[], 1));
        assert_eq!(boss.mode(), PileMode::SwipeTell);
        assert_eq!(boss.target(), Some(HeroId::Carl));
        let swapped = [carl(E - 500.0), donut(E - 70.0)];
        for t in 2..=30 {
            boss.step(&ctx(swapped, &[], t));
            assert_eq!(boss.target(), Some(HeroId::Carl), "locked at tick {t}");
        }
    }

    #[test]
    fn a_perched_or_airborne_hero_is_not_a_swipe_or_shove_target() {
        // Feet above 44 px: she walks toward but never swipes or shoves.
        let mut boss = awake(E);
        let high = view(HeroId::Carl, E - 70.0, FLOOR + 50.0, 12.0, 52.0);
        let mut tells = 0;
        for t in 1..=100u64 {
            let out = boss.step(&ctx([high, absent(HeroId::Donut)], &[], t));
            tells += out
                .pile_events
                .iter()
                .filter(|(_, e)| matches!(e, PileEvent::Tell { .. }))
                .count();
        }
        assert_eq!(tells, 0);
        assert_eq!(boss.target(), Some(HeroId::Carl), "still walks toward");
        // At 44 px exactly she does swipe.
        let mut boss = awake(E);
        let low = view(HeroId::Carl, E - 70.0, FLOOR + 44.0, 12.0, 52.0);
        let out = boss.step(&ctx([low, absent(HeroId::Donut)], &[], 1));
        assert!(out.pile_events.iter().any(|(_, e)| matches!(
            e,
            PileEvent::Tell {
                attack: PileAttack::Swipe,
                ..
            }
        )));
    }

    #[test]
    fn hrd_turn_flank_turns_in_twenty_ticks_then_swipes_at_the_flank() {
        let mut boss = awake(E);
        let heroes = heroes_carl(E + 40.0);
        let mut turn_start = None;
        let mut tell = None;
        for t in 1..=60u64 {
            let out = boss.step(&ctx(heroes, &[], t));
            if boss.mode() == PileMode::Turn && turn_start.is_none() {
                turn_start = Some(t);
            }
            if out.pile_events.iter().any(|(_, e)| {
                matches!(
                    e,
                    PileEvent::Tell {
                        attack: PileAttack::Swipe,
                        ..
                    }
                )
            }) {
                tell = Some(t);
                break;
            }
        }
        assert_eq!(turn_start, Some(1));
        assert_eq!(tell, Some(21), "Turn 20 ticks, the tell on the next");
        assert_eq!(boss.facing(), 1, "facing him");
    }

    // ------------------------------------------------------ the nest rule ---

    fn carl_on_top(x: f32) -> HeroView {
        view(HeroId::Carl, x, FLOOR + 48.0, 12.0, 52.0)
    }

    #[test]
    fn the_nest_walk_goal_is_the_face_for_targets_on_the_top_or_left_of_the_bank() {
        let bank = [bank_a()];
        for x in [340.0, 352.0, 380.0, 410.0, 427.0] {
            let mut boss = awake(608.0);
            let hero = if x < 352.0 { carl(x) } else { carl_on_top(x) };
            let c = ctx([hero, absent(HeroId::Donut)], &bank, 1);
            boss.step(&c);
            assert_eq!(boss.nest_walk_goal(&c), Some(444.0), "target at {x}");
        }
        // A target past the bank's far edge is not a nest target.
        let mut boss = awake(608.0);
        let c = ctx(heroes_carl(430.0 + 12.0), &bank, 1);
        boss.step(&c);
        assert_eq!(boss.nest_walk_goal(&c), None);
        // Without the bank, or with the rule off, there is no goal.
        let c = ctx(heroes_carl(380.0), &[], 1);
        assert_eq!(boss.nest_walk_goal(&c), None);
        let mut off = tun().clone();
        off.hrdq24_nest_ticks = 0;
        let off: &'static PileBossTuning = Box::leak(Box::new(off));
        let b = PileBoss::awake_at(off, setup(), 608.0, -1);
        let c = ctx([carl_on_top(400.0), absent(HeroId::Donut)], &bank, 1);
        let mut b = b;
        b.step(&c);
        assert_eq!(b.nest_walk_goal(&c), None);
    }

    #[test]
    fn she_walks_to_the_face_with_no_standoff_and_the_bank_stops_her() {
        let bank = [bank_a()];
        let mut boss = awake(608.0);
        let heroes = [carl_on_top(400.0), absent(HeroId::Donut)];
        for t in 1..=400u64 {
            boss.step(&ctx(heroes, &bank, t));
            if t == 1 {
                assert_eq!(boss.mode(), PileMode::Walk);
            }
            assert!(boss.x() >= 444.0, "the bank blocks her body: {}", boss.x());
        }
        assert_eq!(boss.x(), 444.0);
    }

    #[test]
    fn hrd_nest_push_after_sixty_ticks_at_the_face() {
        let bank = [bank_a()];
        for (hero, name) in [
            (carl_on_top(410.0), "Carl 410"),
            (
                view(HeroId::Donut, 425.0, FLOOR + 48.0, 10.0, 24.0),
                "Donut 425",
            ),
        ] {
            let heroes = if hero.id == HeroId::Carl {
                [hero, absent(HeroId::Donut)]
            } else {
                [absent(HeroId::Carl), hero]
            };
            let mut boss = awake(444.0);
            let mut push = None;
            let mut jam = None;
            for t in 1..=300u64 {
                let out = boss.step(&ctx(heroes, &bank, t));
                if out
                    .pile_events
                    .iter()
                    .any(|(_, e)| *e == PileEvent::NestPush)
                    && push.is_none()
                {
                    push = Some(t);
                }
                if let Some(j) = out.jams.first() {
                    jam = Some((t, j.obstacle, j.credit, boss.x(), out.blows.len()));
                    break;
                }
            }
            // The timer counts from the first tick at the face: the push
            // begins on tick 61, the shove tell (40) then the immediate jam.
            assert_eq!(push, Some(61), "{name}");
            let (t, kind, credit, x, blows) = jam.expect(name);
            assert_eq!(t, 61 + 40, "{name}: the jam is lunge tick 1");
            assert_eq!(kind, ObstacleKind::BankA);
            assert_eq!(credit, Some(hero.id.index() as u32));
            assert_eq!(x, 444.0, "no motion");
            assert_eq!(blows, 0, "a percher on the top (feet 48) is not hit");
            assert_eq!(boss.facing(), -1);
        }
    }

    /// HRD-D (review of #260): the push needs bank A `stable`. While the bank is `disturbed` the
    /// timer still counts (it is keyed on her pivot and the heroes' x only), but she does not push
    /// and does not turn for it.
    #[test]
    fn the_nest_push_waits_for_a_stable_bank() {
        let bank = [bank_a()];
        let heroes = [carl_on_top(410.0), absent(HeroId::Donut)];
        let mut boss = awake(444.0);
        for t in 1..=120u64 {
            let out = boss.step(&PileCtx {
                bank_a_disturbed: true,
                ..ctx(heroes, &bank, t)
            });
            assert!(
                !out.pile_events
                    .iter()
                    .any(|(_, e)| *e == PileEvent::NestPush),
                "no push at a disturbed bank (tick {t})"
            );
            assert_eq!(boss.mode(), PileMode::Neutral, "tick {t}");
        }
        assert_eq!(boss.nest_ticks(), 120, "the timer still counts");
        let out = boss.step(&ctx(heroes, &bank, 121));
        assert!(
            out.pile_events
                .iter()
                .any(|(_, e)| *e == PileEvent::NestPush),
            "a stable bank takes the push at once"
        );
    }

    #[test]
    fn the_nest_push_turns_first_when_she_faces_away() {
        let bank = [bank_a()];
        let heroes = [carl_on_top(410.0), absent(HeroId::Donut)];
        let mut boss = PileBoss::awake_at(tun(), setup(), 444.0, 1);
        let mut order = Vec::new();
        for t in 1..=200u64 {
            let out = boss.step(&ctx(heroes, &bank, t));
            if matches!(boss.mode(), PileMode::Turn) && order.last() != Some(&"turn") {
                order.push("turn");
            }
            if out
                .pile_events
                .iter()
                .any(|(_, e)| *e == PileEvent::NestPush)
            {
                order.push("push");
                break;
            }
        }
        // She faces +x (away from the bank): the first tick that she may push
        // is the turn, then the push.
        assert_eq!(order, ["turn", "push"]);
        assert_eq!(boss.facing(), -1, "the push always faces the bank");
    }

    #[test]
    fn the_nest_timer_is_not_reset_by_a_hop_a_hit_a_flinch_or_a_switch() {
        let bank = [bank_a()];
        let mut boss = awake(444.0);
        let on_top = [carl_on_top(400.0), absent(HeroId::Donut)];
        for t in 1..=10u64 {
            boss.step(&ctx(on_top, &bank, t));
        }
        assert_eq!(boss.nest_ticks(), 10);
        // A hop: the percher is in the air (still left of 416, any height).
        let hop = view(HeroId::Carl, 400.0, FLOOR + 90.0, 12.0, 52.0);
        for t in 11..=15u64 {
            boss.step(&ctx([hop, absent(HeroId::Donut)], &bank, t));
        }
        assert_eq!(boss.nest_ticks(), 15);
        // A hit on her.
        let (r, _) = jab(&mut boss, 16);
        assert!(r.is_some());
        boss.step(&ctx(on_top, &bank, 16));
        assert_eq!(boss.nest_ticks(), 16);
        // A Flinch (four poise).
        let mut out = PileOut::default();
        for _ in 0..4 {
            boss.apply_hit(hit(0, 1), None, 17, &mut out);
        }
        assert_eq!(boss.mode(), PileMode::Flinch);
        for t in 17..=20u64 {
            boss.step(&ctx(on_top, &bank, t));
        }
        assert_eq!(boss.nest_ticks(), 20);
        // A switch of target between two qualifying heroes.
        let two = [
            carl_on_top(380.0),
            view(HeroId::Donut, 405.0, FLOOR + 48.0, 10.0, 24.0),
        ];
        for t in 21..=25u64 {
            boss.step(&ctx(two, &bank, t));
        }
        let two_swapped = [
            carl_on_top(405.0),
            view(HeroId::Donut, 380.0, FLOOR + 48.0, 10.0, 24.0),
        ];
        for t in 26..=30u64 {
            boss.step(&ctx(two_swapped, &bank, t));
        }
        assert_eq!(boss.nest_ticks(), 30);
        // It resets only when no hero qualifies...
        boss.step(&ctx(heroes_carl(500.0), &bank, 31));
        assert_eq!(boss.nest_ticks(), 0);
    }

    #[test]
    fn the_nest_timer_counts_in_every_mode_and_resets_when_she_leaves_the_face() {
        let bank = [bank_a()];
        let mut boss = awake(444.0);
        boss.hp = 30;
        // The percher is on the top; she is in a Recover (not choosing).
        boss.mode = PileMode::SwipeRecover;
        boss.mode_tick = 1;
        let top = [carl_on_top(400.0), absent(HeroId::Donut)];
        for t in 1..=20u64 {
            boss.step(&ctx(top, &bank, t));
        }
        assert_eq!(boss.nest_ticks(), 20, "counts during a recovery");
        // Her pivot leaves the face: it resets.
        boss.x = 470.0;
        boss.step(&ctx(top, &bank, 21));
        assert_eq!(boss.nest_ticks(), 0);
    }

    // -------------------------------------------- leash, disengage, home ---

    #[test]
    fn she_stops_at_the_leash_edge_and_disengages_after_ninety_unreachable_ticks() {
        // hrd_leash_return: no bank, Carl at x 100.
        let mut boss = awake(400.0);
        let heroes = heroes_carl(100.0);
        let mut disengage = None;
        for t in 1..=900u64 {
            let out = boss.step(&ctx(heroes, &[], t));
            assert!(boss.x() >= 290.0, "never past the leash: {}", boss.x());
            if out
                .pile_events
                .iter()
                .any(|(_, e)| *e == PileEvent::Disengage)
            {
                disengage = Some(t);
                break;
            }
        }
        let t = disengage.expect("a disengage");
        assert!(
            (290.0..=291.0).contains(&boss.x()),
            "she held the edge, the first step home is under a pixel: {}",
            boss.x()
        );
        assert_eq!(boss.hp(), 30, "HP unchanged");
        assert!(boss.walking_home());
        // 90 consecutive unreachable ticks counted from her arrival at the
        // edge: the walk starts on the tick after the counter reaches 90.
        let arrive = (400.0 - 290.0) / 0.75;
        assert!(
            (t as f32 - (arrive + 90.0)).abs() <= 3.0,
            "t {t} arrive {arrive}"
        );
        // She walks to 608 and holds there, HP unchanged.
        for s in 1..=600u64 {
            boss.step(&ctx(heroes, &[], 1000 + s));
        }
        assert_eq!(boss.x(), 608.0);
        assert!(boss.home_hold());
        assert!(!boss.walking_home());
        assert_eq!(boss.hp(), 30);
    }

    #[test]
    fn hrd_disengage_trigger_band_uses_the_swipe_trigger_not_the_box() {
        // Donut at pivot 200, she at 290: distance 90 exceeds the 88 px
        // trigger though her hurtbox still touches the swipe box.
        let mut boss = awake(290.0);
        let heroes = heroes_donut(200.0);
        let mut at = None;
        for t in 1..=200u64 {
            let out = boss.step(&ctx(heroes, &[], t));
            if out
                .pile_events
                .iter()
                .any(|(_, e)| *e == PileEvent::Disengage)
            {
                at = Some(t);
                break;
            }
            // She does not swipe an 90 px target (eligible, in front, > 88).
            assert!(out.blows.is_empty());
        }
        assert!(at.is_some());
        // At 88 she swipes instead and never disengages.
        let mut boss = awake(290.0);
        let heroes = heroes_donut(290.0 - 88.0);
        let mut disengaged = false;
        let mut swiped = false;
        for t in 1..=400u64 {
            let out = boss.step(&ctx(heroes, &[], t));
            disengaged |= out
                .pile_events
                .iter()
                .any(|(_, e)| *e == PileEvent::Disengage);
            swiped |= out.pile_events.iter().any(|(_, e)| {
                matches!(
                    e,
                    PileEvent::Tell {
                        attack: PileAttack::Swipe,
                        ..
                    }
                )
            });
        }
        assert!(swiped && !disengaged);
    }

    #[test]
    fn hits_alone_never_end_the_disengage_walk_and_never_cancel_it() {
        // hrd_no_safe_snipe: Donut at x 150 shoots missiles; they deal damage
        // and poise, never cancel the walk.
        let mut boss = awake(290.0);
        let heroes = heroes_donut(150.0);
        let mut started = false;
        let mut flinches = 0;
        let mut hp_lost = 0;
        for t in 1..=1500u64 {
            let out = boss.step(&ctx(heroes, &[], t));
            started |= out
                .pile_events
                .iter()
                .any(|(_, e)| *e == PileEvent::Disengage);
            if started && t % 7 == 0 && hp_lost < 20 && boss.hurtbox_exists() {
                let mut o = PileOut::default();
                if let Some(r) = boss.apply_hit(hit(1, 1), None, t, &mut o) {
                    hp_lost += 1;
                    flinches += usize::from(r.flinched);
                }
            }
            if boss.home_hold() {
                break;
            }
        }
        assert!(started);
        assert!(boss.home_hold(), "she still got home");
        assert_eq!(boss.x(), 608.0);
        assert!(hp_lost > 0 && boss.hp() == 30 - hp_lost as u16);
        assert!(flinches >= 1, "a Flinch plays, then the walk resumes");
    }

    impl PileBoss {
        fn hurtbox_exists(&self) -> bool {
            !self.hurt_rects().is_empty()
        }
    }

    #[test]
    fn hrd_disengage_follow_punished_no_free_jabs_on_the_walk_home() {
        // N1: Carl follows her home to 44 px behind her and jabs. She answers
        // within one tick (Turn) and a swipe tell follows within 21 ticks; he
        // lands at most two jabs first.
        let mut boss = awake(290.0);
        let mut carl_x: f32 = 150.0;
        let mut disengaged = false;
        let mut entered = None;
        let mut turn = None;
        let mut tell = None;
        let mut jabs = 0;
        for t in 1..=1500u64 {
            // Carl waits at 150 until she is walking home, then follows at her
            // speed (she walks 0.75 px/tick; he is faster).
            let walking = boss.walking_home();
            if walking {
                let want = (boss.x() - 44.0).max(150.0);
                carl_x = (carl_x + 3.0).min(want);
            }
            let heroes = heroes_carl(carl_x);
            let out = boss.step(&ctx(heroes, &[], t));
            disengaged |= out
                .pile_events
                .iter()
                .any(|(_, e)| *e == PileEvent::Disengage);
            let gap = (boss.x() - carl_x).abs();
            if disengaged && entered.is_none() && gap <= 88.0 {
                entered = Some(t);
            }
            if let Some(e) = entered {
                if boss.mode() == PileMode::Turn && turn.is_none() {
                    turn = Some(t);
                }
                if out.pile_events.iter().any(|(_, ev)| {
                    matches!(
                        ev,
                        PileEvent::Tell {
                            attack: PileAttack::Swipe,
                            ..
                        }
                    )
                }) && tell.is_none()
                {
                    tell = Some(t);
                }
                // Carl jabs every 14 ticks while he is within reach and she has
                // not started her tell yet.
                if tell.is_none() && (t - e) % 14 == 0 {
                    let mut o = PileOut::default();
                    if boss.apply_hit(hit(1, 1), None, t, &mut o).is_some() {
                        jabs += 1;
                    }
                }
            }
            if tell.is_some() {
                break;
            }
        }
        let e = entered.expect("Carl entered 88 px");
        let turn = turn.expect("she turned");
        let tell = tell.expect("she swiped");
        assert!(turn - e <= 1, "Turn within 1 tick: {} vs {}", turn, e);
        assert!(tell - e <= 21, "tell within 21 ticks: {} vs {}", tell, e);
        assert!(jabs <= 2, "at most two jabs before the tell: {jabs}");
        assert_eq!(
            boss.facing(),
            -1,
            "she turned to face Carl, who follows behind"
        );
    }

    #[test]
    fn the_walk_home_ends_for_a_hero_in_the_shove_range_that_is_not_beyond_the_edge() {
        let mut boss = awake(290.0);
        let far = heroes_carl(100.0);
        let mut t = 0;
        while !boss.walking_home() {
            t += 1;
            boss.step(&ctx(far, &[], t));
            assert!(t < 500);
        }
        // A hero beyond the edge in the shove band (290 - 120 = 170) does not
        // end it; one on the arena side does.
        for _ in 0..10 {
            t += 1;
            boss.step(&ctx(heroes_carl(boss.x() - 120.0), &[], t));
        }
        assert!(boss.walking_home(), "beyond the edge: the walk goes on");
        let inside = boss.x() + 120.0;
        t += 1;
        boss.step(&ctx(heroes_carl(inside), &[], t));
        assert!(!boss.walking_home(), "inside the arena: ordinary choices");
    }

    #[test]
    fn the_home_hold_waits_for_a_hero_within_320_and_still_answers_close_ones() {
        let mut boss = awake(290.0);
        let far = heroes_carl(100.0);
        for t in 1..=1500u64 {
            boss.step(&ctx(far, &[], t));
            if boss.home_hold() {
                break;
            }
        }
        assert!(boss.home_hold());
        // At 318 px she stays (the 320 px draw has not happened): wait, within
        // 320 ends the hold.
        for t in 1..=60u64 {
            boss.step(&ctx(heroes_carl(608.0 - 330.0), &[], 2000 + t));
        }
        assert!(boss.home_hold());
        assert_eq!(boss.x(), 608.0, "no walk choice while holding");
        boss.step(&ctx(heroes_carl(608.0 - 320.0), &[], 3000));
        assert!(!boss.home_hold(), "a hero within 320 px draws her out");
        // The swipe and the shove choices still apply in the hold.
        let mut held = awake(608.0);
        held.home = Home::Hold;
        let out = held.step(&ctx(heroes_carl(608.0 - 60.0), &[], 1));
        // A hero within 320 ends the hold at once, but the choice is the swipe.
        assert!(out.pile_events.iter().any(|(_, e)| matches!(
            e,
            PileEvent::Tell {
                attack: PileAttack::Swipe,
                ..
            }
        )));
    }

    #[test]
    fn she_returns_home_after_240_ticks_with_no_hero_within_320() {
        // No hero within 320 px: nobody is present, so nobody is near.
        let mut boss = awake(800.0);
        let nobody = [absent(HeroId::Carl), absent(HeroId::Donut)];
        let mut walked_home_at = None;
        for t in 1..=1200u64 {
            boss.step(&ctx(nobody, &[], t));
            if boss.walking_home() && walked_home_at.is_none() {
                walked_home_at = Some(t);
            }
        }
        let at = walked_home_at.expect("she walked home");
        assert!((240..=243).contains(&at), "after 240 ticks: {at}");
        assert_eq!(boss.x(), 608.0);
        assert!(boss.home_hold());
        assert_eq!(boss.hp(), 30, "no regeneration, ever");
    }

    // ----------------------------------------------------- the shove guard ---

    #[test]
    fn no_shove_when_the_lunge_would_end_at_the_leash_clamp_short_of_the_target() {
        // She at 290 (the minimum) facing -x: the edge clamp puts the box front
        // at 242. A target whose hurtbox lies wholly beyond it is not shoved.
        let heroes = heroes_carl(290.0 - 110.0);
        let mut boss = awake(290.0);
        let mut shoves = 0;
        for t in 1..=60u64 {
            let out = boss.step(&ctx(heroes, &[], t));
            shoves += out
                .pile_events
                .iter()
                .filter(|(_, e)| {
                    matches!(
                        e,
                        PileEvent::Tell {
                            attack: PileAttack::Shove,
                            ..
                        }
                    )
                })
                .count();
        }
        assert_eq!(shoves, 0, "she walks (or holds) instead");
        // A target whose hurtbox reaches the clamped front is shoved: hurtbox
        // right edge 243 > 242.
        let mut boss = awake(290.0);
        let heroes = heroes_carl(243.0 - 12.0);
        let out = boss.step(&ctx(heroes, &[], 1));
        assert!(
            out.pile_events.iter().any(|(_, e)| matches!(
                e,
                PileEvent::Tell {
                    attack: PileAttack::Shove,
                    ..
                }
            )) || boss.mode() == PileMode::SwipeTell,
            "an in-reach target is answered"
        );
        // Mirror at the maximum: the clamp front is 928.
        let mut boss = PileBoss::awake_at(tun(), setup(), 880.0, 1);
        let heroes = heroes_carl(880.0 + 110.0);
        let mut shoves = 0;
        for t in 1..=60u64 {
            let out = boss.step(&ctx(heroes, &[], t));
            shoves += out
                .pile_events
                .iter()
                .filter(|(_, e)| {
                    matches!(
                        e,
                        PileEvent::Tell {
                            attack: PileAttack::Shove,
                            ..
                        }
                    )
                })
                .count();
        }
        assert_eq!(shoves, 0);
    }

    #[test]
    fn the_shove_has_its_own_cooldown() {
        let mut boss = awake(E).with_shove_cooldown(180);
        let heroes = heroes_carl(E - 128.0);
        let mut tells = Vec::new();
        for t in 1..=40u64 {
            let out = boss.step(&ctx(heroes, &[], t));
            if out.pile_events.iter().any(|(_, e)| {
                matches!(
                    e,
                    PileEvent::Tell {
                        attack: PileAttack::Shove,
                        ..
                    }
                )
            }) {
                tells.push(t);
            }
        }
        assert!(tells.is_empty(), "the cooldown holds the shove back");
        assert_eq!(boss.shove_cooldown(), 140);
    }

    #[test]
    fn the_shove_cycle_is_268_ticks_counted_from_the_end_of_the_lunge() {
        // HRD-Q14: 180 ticks after the shove ends (the lunge), so a whiff
        // cycle is tell 40 + lunge 48 + 180 = 268 ticks. The target stays 128 px
        // ahead of her so she answers with a shove every time it is allowed.
        let mut boss = awake(E);
        let mut tells = Vec::new();
        let mut cd_at_lunge_end = None;
        let mut prev = PileMode::Neutral;
        for t in 1..=600u64 {
            let x = boss.x() - 128.0;
            let heroes = [
                HeroView {
                    vulnerable: false,
                    ..carl(x)
                },
                absent(HeroId::Donut),
            ];
            let out = boss.step(&ctx(heroes, &[], t));
            if out.pile_events.iter().any(|(_, e)| {
                matches!(
                    e,
                    PileEvent::Tell {
                        attack: PileAttack::Shove,
                        ..
                    }
                )
            }) {
                tells.push(t);
            }
            if prev == PileMode::Lunge && boss.mode() == PileMode::ShoveRecover {
                cd_at_lunge_end.get_or_insert((t, boss.shove_cooldown()));
            }
            if boss.mode() == PileMode::ShoveRecover && cd_at_lunge_end.is_none() {
                cd_at_lunge_end = Some((t, boss.shove_cooldown()));
            }
            prev = boss.mode();
        }
        assert_eq!(&tells[..2], [1, 1 + 268], "tells: {tells:?}");
        // The cooldown was armed on the lunge's last tick (scene tick 88).
        assert_eq!(cd_at_lunge_end, Some((88, 181)));
        // After a jam the same count runs from the jam tick.
        let bank = [bank_a()];
        let mut boss = awake(470.0);
        let heroes = heroes_carl(330.0);
        let mut jam = None;
        let mut second_tell = None;
        for t in 1..=700u64 {
            let out = boss.step(&ctx(heroes, &bank, t));
            if jam.is_none() && !out.jams.is_empty() {
                jam = Some(t);
            }
            if jam.is_some_and(|j| t > j)
                && out.pile_events.iter().any(|(_, e)| {
                    matches!(
                        e,
                        PileEvent::Tell {
                            attack: PileAttack::Shove,
                            ..
                        }
                    )
                })
            {
                second_tell = Some(t);
                break;
            }
        }
        assert_eq!(second_tell, jam.map(|j| j + 181), "jam {jam:?}");
    }

    // ------------------------------------------------------ the AI feed ---

    #[test]
    fn the_pound_dodge_anchor_is_thirteen_px_ahead_with_no_target() {
        let mut boss = awake(E);
        boss.step(&ctx(heroes_carl(E - 44.0), &[], 1));
        match boss.threat() {
            Some(BossThreat::Pound { x, facing, target }) => {
                assert_eq!(x, E - 13.0, "pivot + facing * 13");
                assert_eq!(facing, -1);
                assert_eq!(target, None, "her swipe hits every hero in the box");
            }
            other => panic!("{other:?}"),
        }
        // Held through the smear and the impact, gone in the recover.
        for t in 2..=43u64 {
            boss.step(&ctx(heroes_carl(E - 44.0), &[], t));
            assert!(
                matches!(boss.threat(), Some(BossThreat::Pound { .. })),
                "tick {t}"
            );
        }
        boss.step(&ctx(heroes_carl(E - 44.0), &[], 44));
        assert_eq!(boss.mode(), PileMode::SwipeRecover);
        assert!(boss.threat().is_none());
        // The follower's 84 px step-out plus its 3.45 px follow pull-back (207 px/s
        // at 60 Hz) must stay clear of her 92 px reach: 13 >= 92 - 84 + 1 + 3.45
        // (reviewer ruling on #215, HRD-Q33; the sheet's 9 ignored the pull-back).
        let anchor = f32::from(tun().hrdq33_pound_anchor_px);
        assert_eq!(anchor, 13.0);
        assert!(anchor >= 92.0 - 84.0 + 1.0 + 3.45);
        // Facing +x mirrors it.
        let mut east = PileBoss::awake_at(tun(), setup(), E, 1);
        east.step(&ctx(heroes_carl(E + 44.0), &[], 1));
        match east.threat() {
            Some(BossThreat::Pound { x, facing, .. }) => {
                assert_eq!((x, facing), (E + 13.0, 1));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_charge_feed_runs_through_the_tell_and_the_lunge() {
        let mut boss = awake(E);
        let heroes = heroes_carl(E - 128.0);
        boss.step(&ctx(heroes, &[], 1));
        assert_eq!(boss.mode(), PileMode::ShoveTell);
        match boss.threat() {
            Some(BossThreat::Charge {
                tell_left,
                x,
                dir,
                running,
                front,
                depth,
                speed,
                lead_ticks,
                started,
            }) => {
                assert_eq!(tell_left, 39);
                assert_eq!(started, 1, "the clock of the tell's first tick");
                assert_eq!((x, dir, running), (E, -1, false));
                assert_eq!((front, depth, speed, lead_ticks), (48.0, 28.0, 3.0, 18));
            }
            other => panic!("{other:?}"),
        }
        for t in 2..=41u64 {
            boss.step(&ctx(heroes, &[], t));
        }
        match boss.threat() {
            Some(BossThreat::Charge {
                tell_left,
                running,
                x,
                ..
            }) => {
                assert_eq!(tell_left, 0);
                assert!(running);
                assert_eq!(x, boss.x(), "her real pivot");
            }
            other => panic!("{other:?}"),
        }
        // No threat in Neutral.
        let neutral = awake(E);
        assert!(neutral.threat().is_none());
    }

    // ------------------------------------------------------------ misc ---

    #[test]
    fn the_kill_switch_makes_her_inert() {
        let mut off = tun().clone();
        off.hrdq37_enabled = false;
        let off: &'static PileBossTuning = Box::leak(Box::new(off));
        let mut boss = PileBoss::awake_at(off, setup(), E, -1);
        let out = boss.step(&ctx(heroes_carl(E - 60.0), &[], 1));
        assert_eq!(out, PileOut::default());
        assert_eq!(boss.mode(), PileMode::Neutral);
    }

    #[test]
    fn determinism_the_same_script_gives_the_same_state_hashes() {
        fn script() -> Vec<u64> {
            let bank = [bank_a(), post()];
            let mut boss = PileBoss::new(tun(), setup());
            let mut hashes = Vec::new();
            for t in 1..=900u64 {
                let carl_x = 300.0 + 150.0 * ((t as f32) * 0.01).sin() + 200.0;
                let heroes = [carl(carl_x), donut(240.0)];
                let out = boss.step(&ctx(heroes, &bank, t));
                if t % 37 == 0 {
                    boss.apply_hit(hit(1, 1), None, t, &mut PileOut::default());
                }
                if t == 500 {
                    boss.expose(Some(0), t, &mut PileOut::default());
                }
                hashes.push(state_hash(&boss) ^ out.blows.len() as u64);
            }
            hashes
        }
        let a = script();
        let b = script();
        assert_eq!(a, b);
        assert!(a.windows(2).any(|w| w[0] != w[1]), "the state moves");
        // A different input changes the trace.
        let mut boss = PileBoss::new(tun(), setup());
        let mut other = Vec::new();
        for t in 1..=900u64 {
            boss.step(&ctx([carl(500.0), donut(240.0)], &[], t));
            other.push(state_hash(&boss));
        }
        assert_ne!(a[..100], other[..100]);
    }

    #[test]
    fn a_clone_forks_a_run_identically() {
        let mut boss = awake(E);
        let heroes = heroes_carl(E - 100.0);
        for t in 1..=30u64 {
            boss.step(&ctx(heroes, &[], t));
        }
        let mut fork = boss.clone();
        for t in 31..=120u64 {
            let a = boss.step(&ctx(heroes, &[], t));
            let b = fork.step(&ctx(heroes, &[], t));
            assert_eq!(a, b);
            assert_eq!(state_hash(&boss), state_hash(&fork));
        }
    }

    #[test]
    fn hit_stop_freezes_her_clock_but_not_the_counters() {
        let bank = [bank_a()];
        let mut boss = awake(444.0);
        let top = [carl_on_top(400.0), absent(HeroId::Donut)];
        boss.step(&ctx(top, &bank, 1));
        let mut out = PileOut::default();
        boss.apply_hit(
            PileHit {
                damage: 1,
                poise: 1,
                player: 0,
                hit_stop: 5,
                cause: DefeatCause::Melee,
            },
            None,
            1,
            &mut out,
        );
        let tick_before = boss.mode_tick();
        for t in 2..=6u64 {
            boss.step(&ctx(top, &bank, t));
        }
        assert_eq!(boss.mode_tick(), tick_before, "frozen for five ticks");
        assert_eq!(boss.nest_ticks(), 6, "the nest timer still counts");
    }
}
