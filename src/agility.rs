//! DONUT-MOBILITY-01 knob schema (DM-A): Donut's agile motion.
//!
//! Bevy-free data contract for `docs/03-tech/donut-mobility-01.md`.
//! [`AgilityTuning`] mirrors `assets/data/gameplay/hero_agility_v0.json` field
//! for field. The file is embedded, `deny_unknown_fields`, validated and FNV-1a
//! pinned like `cf02_combat_v0.json` ([`crate::cf02`]): editing it without
//! updating [`TUNING_HASH`] fails [`AgilityTuning::pinned`] and the pin test, so
//! a tuning edit never changes a recorded outcome silently.
//!
//! DM-A reads the motion knobs (DM-Q01, Q02, Q08). DM-B reads the evasion
//! knobs (DM-Q11, Q12) through [`ReflexProfile`] and runs the reflex itself as
//! the pure state machine [`Reflex`] (DM-Q13, Q19, Q20, Q22): ticks only, no
//! RNG, no Bevy. Knob names carry the spec's question ID (`dm_q01_run_speed`).
//! Questions without a key are listed in [`NO_KNOB`] with the reason.

use crate::donut_mana::content_hash;
use crate::prototype::{GRAVITY, HeroMotion, TICK_SECONDS};
use serde::{Deserialize, Serialize};

/// The tuning file, embedded so the binary and the tests read the same bytes.
pub const TUNING_JSON: &str = include_str!("../assets/data/gameplay/hero_agility_v0.json");

/// The committed FNV-1a 64 content hash of [`TUNING_JSON`].
pub const TUNING_HASH: u64 = 0xad4b_7f26_bea9_8c8f;

/// The reflex rearm range of DM-Q12, in ticks (inclusive).
pub const REARM_RANGE: core::ops::RangeInclusive<u32> = 600..=900;

/// The run speed must stay below the scroll camera's 4 px/tick pan cap
/// (240 px/s), so a controlled Donut never outruns the camera (DM-Q01).
pub const RUN_SPEED_CEILING: f32 = 240.0;

/// Question IDs of the spec (1..=27) that have no key in the file, and why.
/// The completeness test requires every other ID to own a `dm_qNN_` key.
pub const NO_KNOB: [(u8, &str); 22] = [
    (3, "shared GRAVITY, unchanged"),
    (4, "derived: air ticks and flat air distance"),
    (5, "no acceleration or deceleration, as Carl"),
    (6, "coyote 6 and jump buffer 7, unchanged"),
    (7, "not scaled: attacks, hops, knockback, collider, hurtbox"),
    (9, "rule: the 2P wall uses each hero's own run speed"),
    (10, "no motion gate: PartyTuning::donut_motion is Agile"),
    (
        13,
        "rule: the reflex reset rule, Reflex::reset and the scene hooks",
    ),
    (14, "retired: roll mode"),
    (15, "retired: roll mode"),
    (16, "retired: roll mode"),
    (17, "allow-list in code: the EvadeSource variants (DM-Q17)"),
    (
        18,
        "default never: a source with no EvadeSource variant is never evaded",
    ),
    (
        19,
        "rule: the evade outcome, scoped to the attack instance (EvadeKey)",
    ),
    (
        20,
        "rule: the check order, evade_check after the existing guards",
    ),
    (21, "DM-C: presentation stand-ins"),
    (
        22,
        "rule: HeroEvaded and EvadeArmed, hashed from the first arm",
    ),
    (23, "retired: the evasion CLI override"),
    (24, "no share target"),
    (25, "Q4 hurtbox, unchanged"),
    (26, "rule: HUD clearance on the drawn top"),
    (27, "room scenario field ceiling_px, not a hero knob"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgilityTuningError {
    /// The JSON did not parse or has an unknown or missing field.
    Parse(String),
    /// A field failed validation; `field` names it.
    Field { field: &'static str, reason: String },
    /// The file's content hash is not the pinned one.
    HashMismatch { expected: u64, actual: u64 },
}

impl core::fmt::Display for AgilityTuningError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "agility tuning: {msg}"),
            Self::Field { field, reason } => {
                write!(f, "agility tuning: field `{field}`: {reason}")
            }
            Self::HashMismatch { expected, actual } => write!(
                f,
                "agility tuning hash {actual:#018x} is not the pinned {expected:#018x}"
            ),
        }
    }
}

impl std::error::Error for AgilityTuningError {}

/// DM-Q11: the evasion mode. `roll` is retired and not a valid value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvasionMode {
    Reflex,
    Off,
}

/// The tuning file, field for field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgilityTuning {
    pub schema_version: u32,
    /// DM-Q01: Donut's run speed, px/s.
    pub dm_q01_run_speed: f32,
    /// DM-Q02: Donut's jump launch speed, px/s.
    pub dm_q02_jump_speed: f32,
    /// DM-Q08: the follower bot's reach margin for Donut, px.
    pub dm_q08_ai_jump_margin_px: f32,
    /// DM-Q11 (DM-B).
    pub dm_q11_evasion_mode: EvasionMode,
    /// DM-Q12 (DM-B): ticks without a hurt before the reflex arms.
    pub dm_q12_reflex_rearm_ticks: u32,
}

impl AgilityTuning {
    /// Parse and validate.
    pub fn from_json(text: &str) -> Result<Self, AgilityTuningError> {
        let tuning: Self =
            serde_json::from_str(text).map_err(|err| AgilityTuningError::Parse(err.to_string()))?;
        tuning.validate()?;
        Ok(tuning)
    }

    /// Parse the embedded file and require the pinned content hash.
    pub fn pinned() -> Result<Self, AgilityTuningError> {
        Self::from_json_pinned(TUNING_JSON, TUNING_HASH)
    }

    /// Parse `text` and require its content hash to equal `expected`.
    pub fn from_json_pinned(text: &str, expected: u64) -> Result<Self, AgilityTuningError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(AgilityTuningError::HashMismatch { expected, actual });
        }
        Self::from_json(text)
    }

    pub fn validate(&self) -> Result<(), AgilityTuningError> {
        fn bad(field: &'static str, reason: &str) -> Result<(), AgilityTuningError> {
            Err(AgilityTuningError::Field {
                field,
                reason: reason.to_string(),
            })
        }
        if self.schema_version != 1 {
            return bad("schema_version", "must be 1");
        }
        let run = self.dm_q01_run_speed;
        if !run.is_finite() || run <= 0.0 || run >= RUN_SPEED_CEILING {
            return bad(
                "dm_q01_run_speed",
                "must be positive and below 240 px/s (the scroll camera's pan cap)",
            );
        }
        let jump = self.dm_q02_jump_speed;
        if !jump.is_finite() || jump <= 0.0 {
            return bad("dm_q02_jump_speed", "must be positive");
        }
        let margin = self.dm_q08_ai_jump_margin_px;
        if !margin.is_finite() || margin < 0.0 {
            return bad("dm_q08_ai_jump_margin_px", "must not be negative");
        }
        if !REARM_RANGE.contains(&self.dm_q12_reflex_rearm_ticks) {
            return bad(
                "dm_q12_reflex_rearm_ticks",
                "must be within 600..=900 ticks",
            );
        }
        Ok(())
    }

    /// Donut's agile [`HeroMotion`].
    pub fn motion(&self) -> HeroMotion {
        HeroMotion {
            run_speed: self.dm_q01_run_speed,
            jump_speed: self.dm_q02_jump_speed,
        }
    }

    /// DM-Q08: the follower bot's reach for Donut: her discrete apex less the
    /// margin (110.92 px at the adopted values).
    pub fn ai_jump_reach_px(&self) -> f32 {
        discrete_apex_px(self.dm_q02_jump_speed) - self.dm_q08_ai_jump_margin_px
    }
}

/// Which evasion rule a tuning runs (DM-Q11). [`Self::Shipped`] reads the
/// embedded file (`reflex`, 720 ticks) and is what every play path uses. The
/// other two exist for tests and the share measurement (spec section 3.3): a
/// test-only mode switch and rearm override inside the loader's range. No CLI
/// flag or data switch constructs them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflexProfile {
    /// The embedded `hero_agility_v0.json` values (DM-Q11, DM-Q12).
    Shipped,
    /// `dm_q11_evasion_mode: off`: no evasion state advances.
    Off,
    /// The reflex with this rearm, in ticks (the loader's range is 600..=900).
    Rearm(u32),
}

impl ReflexProfile {
    /// The rearm of the live reflex, or `None` when evasion is off.
    pub fn rearm(self) -> Option<u32> {
        match self {
            Self::Shipped => match tuning().dm_q11_evasion_mode {
                EvasionMode::Reflex => Some(tuning().dm_q12_reflex_rearm_ticks),
                EvasionMode::Off => None,
            },
            Self::Off => None,
            Self::Rearm(ticks) => Some(ticks),
        }
    }
}

/// DM-Q17: the evadable sources. This enum **is** the allow-list: the reflex
/// is only ever asked about a source that has a variant here, so a new source
/// (a hazard, a blast, a boss blow, a body contact, a shore block, any future
/// enemy) is **never** evaded until its own design opts it in by adding a
/// variant and a call site (DM-Q18, owner decision 2026-11-21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvadeSource {
    /// A common goblin's swing, every temperament.
    GoblinSwing,
    /// The Engineer's ram.
    EngineerRam,
    /// The llama's spit glob.
    LlamaGlob,
    /// HAZ-B (DM-Q17 list, HAZ-Q15): the slime's lunge. Its body contact, the
    /// acid patch and the Rot blast stay unlisted and never evaded (DM-Q18).
    SlimeLunge,
}

impl EvadeSource {
    /// Every evadable source, in tag order.
    pub const ALL: [Self; 4] = [
        Self::GoblinSwing,
        Self::EngineerRam,
        Self::LlamaGlob,
        Self::SlimeLunge,
    ];

    /// The stable name written to logs.
    pub const fn name(self) -> &'static str {
        match self {
            Self::GoblinSwing => "goblin_swing",
            Self::EngineerRam => "engineer_ram",
            Self::LlamaGlob => "llama_glob",
            Self::SlimeLunge => "slime_lunge",
        }
    }

    const fn tag(self) -> u64 {
        match self {
            Self::GoblinSwing => 1,
            Self::EngineerRam => 2,
            Self::LlamaGlob => 3,
            Self::SlimeLunge => 4,
        }
    }
}

/// HAZ-B: Donut's reflex as a slot enemy's step sees it. The combat state
/// copies its [`Reflex`] and its `live` condition (an evasion tuning on, a live
/// Donut, the biscuit unlocked) into a gate, lends it to the step, and settles
/// it afterwards ([`Self::into_parts`]): the reflex goes back and each evade
/// becomes one `HeroEvaded`. [`Self::check`] is the same decision as
/// `CombatState::evade_check`, which is built on it.
#[derive(Debug, Clone)]
pub struct EvadeGate {
    live: bool,
    reflex: Reflex,
    /// `(tick, hurt source, evadable source)` of each evade this gate made.
    evaded: Vec<(u64, u32, EvadeSource)>,
}

impl EvadeGate {
    pub fn new(live: bool, reflex: Reflex) -> Self {
        Self {
            live,
            reflex,
            evaded: Vec::new(),
        }
    }

    /// DM-Q20: an instance that already passed every guard and overlaps her.
    /// `hurt_source` is the attacker's target id for the event.
    pub fn check(&mut self, key: EvadeKey, hurt_source: u32, tick: u64) -> Verdict {
        if !self.live {
            return Verdict::Hit;
        }
        let verdict = self.reflex.check(key, tick);
        if verdict == Verdict::Evaded {
            self.evaded.push((tick, hurt_source, key.source));
        }
        verdict
    }

    /// The reflex to put back and the evades to log, in order.
    pub fn into_parts(self) -> (Reflex, Vec<(u64, u32, EvadeSource)>) {
        (self.reflex, self.evaded)
    }
}

/// DM-Q19: the identity of one attack instance. A goblin swing or an Engineer
/// ram is `(enemy index, attack start tick)` (the tick its tell started, which
/// no hit-stop moves); a glob is `(owner, spawn tick)`; a slime lunge is
/// `(slot target id, tell start tick)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvadeKey {
    pub source: EvadeSource,
    pub enemy: u32,
    pub stamp: u64,
}

/// What the reflex says about an instance that reached the check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// No evasion: the hit resolves as before.
    Hit,
    /// The armed reflex evaded this instance now (log `HeroEvaded`).
    Evaded,
    /// This same instance was evaded earlier: it never hits her again (no
    /// event, no new arm consumed).
    Passed,
}

/// The reflex state (DM-Q12, DM-Q13, DM-Q19). Armed iff `tick - last_reset_tick
/// >= rearm`; there is no per-tick countdown. Attempt-local: a fixture or
/// defeat restart calls [`Self::restart`], a checkpoint respawn [`Self::reset`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Reflex {
    last_reset_tick: u64,
    armed: bool,
    ever_armed: bool,
    evaded: Option<EvadeKey>,
    /// Tests only (DM-C F5): the evadable instances that reached [`Self::check`],
    /// the spec's definition of an attempt (section 3.3), summed over restarts.
    /// Never read, hashed or compared by play code.
    #[cfg(test)]
    attempts: u32,
}

impl Reflex {
    /// DM-Q13: a reset of the wait at `tick` (the unlock tick, every hurt she
    /// takes, every evade, a checkpoint respawn). The reflex is no longer armed.
    pub fn reset(&mut self, tick: u64) {
        self.last_reset_tick = tick;
        self.armed = false;
    }

    /// DM-Q13: a new attempt starts: the wait restarts at 0 and nothing of the
    /// last attempt stays, hashed state included.
    pub fn restart(&mut self) {
        #[cfg(test)]
        let attempts = self.attempts;
        *self = Self::default();
        #[cfg(test)]
        {
            self.attempts = attempts;
        }
    }

    /// Tests only: the attempts counted at the check (spec section 3.3).
    #[cfg(test)]
    pub const fn attempts_for_test(&self) -> u32 {
        self.attempts
    }

    /// Whether the reflex is ready to evade the next evadable instance.
    pub const fn armed(&self) -> bool {
        self.armed
    }

    /// Whether the reflex armed at least once this attempt (the state is hashed
    /// from then on, spec section 3.5).
    pub const fn ever_armed(&self) -> bool {
        self.ever_armed
    }

    /// The tick of the last reset (the unlock, a hurt, an evade, a respawn).
    pub const fn last_reset_tick(&self) -> u64 {
        self.last_reset_tick
    }

    /// The instance the last evade covered.
    pub const fn evaded(&self) -> Option<EvadeKey> {
        self.evaded
    }

    /// One fixed tick. `rearm` is `None` when evasion is off, `unlocked` is the
    /// biscuit state. Returns true on the tick the reflex arms (log
    /// `EvadeArmed`): the first tick where the armed condition holds.
    pub fn step(&mut self, rearm: Option<u32>, unlocked: bool, tick: u64) -> bool {
        let Some(rearm) = rearm else {
            return false;
        };
        if !unlocked || self.armed || tick.saturating_sub(self.last_reset_tick) < u64::from(rearm) {
            return false;
        }
        self.armed = true;
        self.ever_armed = true;
        true
    }

    /// DM-Q19, DM-Q20: the check for an instance that already passed every
    /// guard and overlaps her. The same instance never hits her again; an
    /// armed reflex evades a new one and starts the wait again.
    pub fn check(&mut self, key: EvadeKey, tick: u64) -> Verdict {
        if self.evaded == Some(key) {
            return Verdict::Passed;
        }
        #[cfg(test)]
        {
            self.attempts += 1;
        }
        if !self.armed {
            return Verdict::Hit;
        }
        self.evaded = Some(key);
        self.reset(tick);
        Verdict::Evaded
    }

    /// Fold into the state hash: nothing until the first arm.
    pub fn hash_into(&self, mix: &mut impl FnMut(u64)) {
        if !self.ever_armed {
            return;
        }
        mix(0x444d_4252_4546_4c58); // "DMBREFLX"
        mix(self.last_reset_tick);
        mix(u64::from(self.armed));
        match self.evaded {
            None => mix(0),
            Some(key) => {
                mix(key.source.tag());
                mix(u64::from(key.enemy));
                mix(key.stamp);
            }
        }
    }
}

/// The embedded, pinned tuning, parsed once. Panics on a drifted or invalid
/// embedded file: that is a build defect the pin test also catches.
pub fn tuning() -> &'static AgilityTuning {
    static TUNING: std::sync::OnceLock<AgilityTuning> = std::sync::OnceLock::new();
    TUNING.get_or_init(|| AgilityTuning::pinned().expect("hero_agility_v0.json is pinned"))
}

/// Donut's agile motion from the embedded file.
pub fn agile_motion() -> HeroMotion {
    tuning().motion()
}

/// The highest point (px above the takeoff) of a standing jump launched at
/// `jump_speed`, from the code's discrete step order in `f32` (`vy = J`, then
/// `vy -= g * dt`, then `y += vy * dt`), not from the continuous `J^2 / 2g`.
pub fn discrete_apex_px(jump_speed: f32) -> f32 {
    discrete_arc(jump_speed).0
}

/// The number of ticks a standing jump launched at `jump_speed` spends in the
/// air on flat ground (the tick of the launch counts).
pub fn air_ticks(jump_speed: f32) -> u32 {
    discrete_arc(jump_speed).1
}

/// `(apex, air ticks)` of a flat-ground jump.
fn discrete_arc(jump_speed: f32) -> (f32, u32) {
    let (mut y, mut vy, mut apex, mut ticks) = (0.0_f32, jump_speed, 0.0_f32, 0_u32);
    loop {
        vy -= GRAVITY * TICK_SECONDS;
        y += vy * TICK_SECONDS;
        ticks += 1;
        if y <= 0.0 {
            return (apex, ticks);
        }
        apex = apex.max(y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prototype::JUMP_SPEED;

    fn tuning() -> AgilityTuning {
        AgilityTuning::pinned().expect("the embedded file is pinned")
    }

    #[test]
    fn tuning_hash_is_pinned() {
        assert_eq!(content_hash(TUNING_JSON), TUNING_HASH);
        assert!(AgilityTuning::pinned().is_ok());
    }

    #[test]
    fn a_drifted_file_is_a_clear_hash_failure() {
        let drifted = TUNING_JSON.replace("207.0", "208.0");
        assert_ne!(drifted, TUNING_JSON);
        assert!(matches!(
            AgilityTuning::from_json_pinned(&drifted, TUNING_HASH),
            Err(AgilityTuningError::HashMismatch { .. })
        ));
        assert!(AgilityTuning::from_json(&drifted).is_ok());
    }

    #[test]
    fn the_adopted_values_are_the_spec_values() {
        let t = tuning();
        assert_eq!(t.dm_q01_run_speed, 207.0);
        assert_eq!(t.dm_q02_jump_speed, 500.0);
        assert_eq!(t.dm_q08_ai_jump_margin_px, 4.0);
        assert_eq!(t.dm_q11_evasion_mode, EvasionMode::Reflex);
        assert_eq!(t.dm_q12_reflex_rearm_ticks, 720);
    }

    #[test]
    fn the_discrete_arc_matches_the_spec_numbers() {
        // Spec section 2: apex 114.92 px, 57 air ticks (Carl: 88.54, 50).
        let (apex, ticks) = discrete_arc(500.0);
        assert!((apex - 114.92).abs() < 0.05, "{apex}");
        assert_eq!(ticks, 57);
        let (apex, ticks) = discrete_arc(JUMP_SPEED);
        assert!((apex - 88.54).abs() < 0.05, "{apex}");
        assert_eq!(ticks, 50);
        // DM-Q08: the bot's reach is 110.92, not the continuous 115.05.
        assert!((tuning().ai_jump_reach_px() - 110.92).abs() < 0.05);
    }

    #[test]
    fn the_schema_round_trips_without_losing_or_inventing_a_field() {
        let t = tuning();
        let written = serde_json::to_string_pretty(&t).unwrap();
        assert_eq!(AgilityTuning::from_json(&written).unwrap(), t);
        let mut value: serde_json::Value = serde_json::from_str(TUNING_JSON).unwrap();
        value["extra"] = serde_json::json!(1);
        assert!(AgilityTuning::from_json(&value.to_string()).is_err());
    }

    fn field_of(text: &str) -> &'static str {
        match AgilityTuning::from_json(text) {
            Err(AgilityTuningError::Field { field, .. }) => field,
            other => panic!("expected a field error, got {other:?}"),
        }
    }

    #[test]
    fn validation_names_the_field() {
        let with = |from: &str, to: &str| TUNING_JSON.replace(from, to);
        assert_eq!(
            field_of(&with(
                "\"dm_q01_run_speed\": 207.0",
                "\"dm_q01_run_speed\": 240.0"
            )),
            "dm_q01_run_speed"
        );
        assert_eq!(
            field_of(&with(
                "\"dm_q01_run_speed\": 207.0",
                "\"dm_q01_run_speed\": 0.0"
            )),
            "dm_q01_run_speed"
        );
        assert_eq!(
            field_of(&with(
                "\"dm_q02_jump_speed\": 500.0",
                "\"dm_q02_jump_speed\": -1.0"
            )),
            "dm_q02_jump_speed"
        );
        assert_eq!(
            field_of(&with(
                "\"dm_q08_ai_jump_margin_px\": 4.0",
                "\"dm_q08_ai_jump_margin_px\": -1.0"
            )),
            "dm_q08_ai_jump_margin_px"
        );
        for bad in ["599", "901"] {
            assert_eq!(
                field_of(&with(
                    "\"dm_q12_reflex_rearm_ticks\": 720",
                    &format!("\"dm_q12_reflex_rearm_ticks\": {bad}")
                )),
                "dm_q12_reflex_rearm_ticks"
            );
        }
        for ok in ["600", "900"] {
            assert!(
                AgilityTuning::from_json(&with(
                    "\"dm_q12_reflex_rearm_ticks\": 720",
                    &format!("\"dm_q12_reflex_rearm_ticks\": {ok}")
                ))
                .is_ok()
            );
        }
        // `roll` is retired and not a valid mode.
        assert!(matches!(
            AgilityTuning::from_json(&with("\"reflex\"", "\"roll\"")),
            Err(AgilityTuningError::Parse(_))
        ));
        assert_eq!(
            field_of(&with("\"schema_version\": 1", "\"schema_version\": 2")),
            "schema_version"
        );
    }

    #[test]
    fn every_question_has_a_key_or_a_reason() {
        let keys: Vec<String> = serde_json::from_str::<serde_json::Value>(TUNING_JSON)
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        for id in 1..=27_u8 {
            let owned = keys.iter().any(|k| k.starts_with(&format!("dm_q{id:02}_")));
            let listed = NO_KNOB.iter().any(|(n, _)| *n == id);
            assert!(owned ^ listed, "DM-Q{id:02}: key={owned} no-knob={listed}");
        }
    }

    #[test]
    fn the_run_speed_stays_below_the_camera_cap() {
        let t = tuning();
        assert!(t.dm_q01_run_speed < RUN_SPEED_CEILING);
        assert!(t.dm_q01_run_speed * TICK_SECONDS < 4.0);
    }

    fn key(stamp: u64) -> EvadeKey {
        EvadeKey {
            source: EvadeSource::GoblinSwing,
            enemy: 0,
            stamp,
        }
    }

    /// Step the reflex with the shipped rearm, unlocked.
    fn arm_at(reflex: &mut Reflex, tick: u64) -> bool {
        reflex.step(ReflexProfile::Shipped.rearm(), true, tick)
    }

    #[test]
    fn the_profiles_read_the_data_file_or_a_test_override() {
        assert_eq!(ReflexProfile::Shipped.rearm(), Some(720));
        assert_eq!(ReflexProfile::Off.rearm(), None);
        assert_eq!(ReflexProfile::Rearm(600).rearm(), Some(600));
        assert_eq!(ReflexProfile::Rearm(900).rearm(), Some(900));
    }

    #[test]
    fn it_arms_exactly_the_rearm_after_the_last_reset_and_announces_once() {
        let mut reflex = Reflex::default();
        reflex.reset(100);
        assert!(!arm_at(&mut reflex, 100 + 719));
        assert!(!reflex.armed() && !reflex.ever_armed());
        assert!(
            arm_at(&mut reflex, 100 + 720),
            "first tick of the condition"
        );
        assert!(reflex.armed() && reflex.ever_armed());
        assert!(!arm_at(&mut reflex, 100 + 721), "announced once");
    }

    #[test]
    fn a_locked_or_off_reflex_never_arms_and_hashes_nothing() {
        let mut reflex = Reflex::default();
        assert!(!reflex.step(Some(720), false, 5000));
        assert!(!reflex.step(None, true, 5000));
        assert!(!reflex.armed());
        let mut words = Vec::new();
        reflex.hash_into(&mut |w| words.push(w));
        assert!(words.is_empty(), "nothing is hashed before the first arm");
        assert!(arm_at(&mut reflex, 720));
        reflex.hash_into(&mut |w| words.push(w));
        assert!(!words.is_empty());
    }

    #[test]
    fn an_armed_reflex_evades_one_instance_then_waits_again() {
        let mut reflex = Reflex::default();
        assert!(arm_at(&mut reflex, 720));
        assert_eq!(reflex.check(key(7), 730), Verdict::Evaded);
        assert!(!reflex.armed());
        assert_eq!(reflex.last_reset_tick(), 730, "an evade resets the wait");
        // The same instance never hits her again, and consumes nothing.
        assert_eq!(reflex.check(key(7), 731), Verdict::Passed);
        // Any other instance can hit her on the next tick.
        assert_eq!(reflex.check(key(8), 731), Verdict::Hit);
        assert!(!arm_at(&mut reflex, 730 + 719));
        assert!(arm_at(&mut reflex, 730 + 720));
    }

    #[test]
    fn an_unarmed_reflex_lets_the_hit_through_and_remembers_nothing() {
        let mut reflex = Reflex::default();
        assert_eq!(reflex.check(key(1), 10), Verdict::Hit);
        assert_eq!(reflex.evaded(), None);
    }

    #[test]
    fn a_reset_disarms_and_a_restart_forgets_the_attempt() {
        let mut reflex = Reflex::default();
        assert!(arm_at(&mut reflex, 720));
        reflex.reset(800);
        assert!(!reflex.armed(), "a hurt rattles her");
        assert!(reflex.ever_armed(), "a reset keeps the hash gate");
        assert!(arm_at(&mut reflex, 800 + 720));
        assert_eq!(reflex.check(key(3), 1530), Verdict::Evaded);
        reflex.restart();
        // Everything but the test-only attempt counter (DM-C F5, kept across
        // restarts on purpose) is back to the default.
        assert!(!reflex.armed() && !reflex.ever_armed() && reflex.evaded().is_none());
        assert_eq!(reflex.last_reset_tick(), 0);
        assert_eq!(
            reflex.attempts_for_test(),
            1,
            "the one check survives a restart"
        );
        let fresh = Reflex {
            attempts: reflex.attempts,
            ..Reflex::default()
        };
        assert_eq!(reflex, fresh);
    }

    #[test]
    fn the_allow_list_is_exactly_four_sources_with_distinct_names() {
        assert_eq!(EvadeSource::ALL.len(), 4);
        let names: Vec<_> = EvadeSource::ALL.iter().map(|s| s.name()).collect();
        assert_eq!(
            names,
            ["goblin_swing", "engineer_ram", "llama_glob", "slime_lunge"]
        );
        let tags: Vec<_> = EvadeSource::ALL.iter().map(|s| s.tag()).collect();
        assert_eq!(tags, [1, 2, 3, 4]);
    }
}
