//! Donut's biscuit unlock and mana pool (HUD-DEBT-02, WORK-023).
//!
//! Bevy-free and deterministic at the 60 Hz fixed tick. The rules are the
//! adopted ones of `docs/01-design/donut-mana-v0-proposal.md`: 8 charges, a
//! Magic Missile that costs 1, a regen timer that restarts only on a **release
//! tick** (30 ticks of delay, then one charge per 40 ticks), an unlock that is
//! run progression (it survives a retry and a checkpoint restart) and a
//! hash-pinned tuning file.
//!
//! This module owns the resource only. The cast timeline (gather, release on
//! cast tick 4) and the swat and pounce live elsewhere
//! ([`crate::donut_moves`]); the adapter calls [`DonutMana::step`] on every
//! tick, [`DonutMana::release`] on the release tick and [`DonutMana::reject_dry`]
//! on a dry press, and turns the returned [`ManaEvent`]s into log entries.
//!
//! The state hash ([`DonutMana::hash_into`]) touches nothing while the pool is
//! locked, so a scene that never grants the unlock hashes exactly as before.

use serde::Deserialize;

/// The hero index the mana events carry: Donut is the second hero.
pub const DONUT_HERO: u8 = 1;

/// The tuning file, embedded so the binary and the tests read the same bytes.
pub const TUNING_JSON: &str = include_str!("../assets/data/gameplay/companion_magic_v0.json");

/// The committed content hash of [`TUNING_JSON`] (FNV-1a 64, the hash the
/// repository already uses for its event hashes). Editing the file without
/// updating this value fails [`ManaTuning::pinned`] and the pin test, so a
/// tuning edit never changes a recorded outcome silently (proposal section 5).
pub const TUNING_HASH: u64 = 0x7bbd_4601_b07e_eee6;

/// FNV-1a 64 over raw bytes.
pub const fn content_hash(text: &str) -> u64 {
    let bytes = text.as_bytes();
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        i += 1;
    }
    hash
}

/// What a Primary press does at 0 mana after the biscuit (Q10-16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmptyPress {
    /// Final rule: no cast, no swat, no lock, no buffer.
    Dry,
    /// Playtest variant, valid only while `donut_moves` is on.
    Swat,
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
            Self::Parse(msg) => write!(f, "companion magic tuning: {msg}"),
            Self::Field { field, reason } => {
                write!(f, "companion magic tuning: field `{field}`: {reason}")
            }
            Self::HashMismatch { expected, actual } => write!(
                f,
                "companion magic tuning hash {actual:#018x} is not the pinned {expected:#018x}"
            ),
        }
    }
}

impl std::error::Error for TuningError {}

/// `assets/data/gameplay/companion_magic_v0.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManaTuning {
    pub schema_version: u32,
    pub mana_max: u8,
    pub missile_cost: u8,
    pub missile_damage: u8,
    pub regen_delay_ticks: u16,
    pub regen_period_ticks: u16,
    pub ai_mana_reserve: u8,
    pub empty_press: EmptyPress,
    pub projectile_speed_px_per_tick: u8,
    pub projectile_range_px: u16,
    pub refire_ticks: u16,
    /// FA-Q14 (the fix-all plan, T21): on its first tick the bolt's hit test
    /// also sweeps from Donut's front edge to the spawn point, so an enemy
    /// closer than the 27 px spawn offset is hit instead of passed. Default on;
    /// off reproduces the earlier hashes (a one-line revert).
    pub missile_spawn_sweep: bool,
}

/// Segments of the HUD-A mana track (the validated `mana_max`).
pub const HUD_MANA_SEGMENTS: u8 = 8;

impl ManaTuning {
    /// Parse and validate. `donut_moves` is Q10-01 (`PartyTuning`): the `swat`
    /// empty press is rejected while it is off.
    pub fn from_json(text: &str, donut_moves: bool) -> Result<Self, TuningError> {
        let tuning: Self =
            serde_json::from_str(text).map_err(|err| TuningError::Parse(err.to_string()))?;
        tuning.validate(donut_moves)?;
        Ok(tuning)
    }

    /// Parse the embedded file and require the pinned content hash.
    pub fn pinned(donut_moves: bool) -> Result<Self, TuningError> {
        Self::from_json_pinned(TUNING_JSON, TUNING_HASH, donut_moves)
    }

    /// Parse `text` and require its content hash to equal `expected`. A replay
    /// header or a fixture records the hash it ran with and calls this.
    pub fn from_json_pinned(
        text: &str,
        expected: u64,
        donut_moves: bool,
    ) -> Result<Self, TuningError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(TuningError::HashMismatch { expected, actual });
        }
        Self::from_json(text, donut_moves)
    }

    pub fn validate(&self, donut_moves: bool) -> Result<(), TuningError> {
        let bad = |field: &'static str, reason: &str| {
            Err(TuningError::Field {
                field,
                reason: reason.to_string(),
            })
        };
        if self.schema_version != 1 {
            return bad("schema_version", "must be 1");
        }
        if self.mana_max != HUD_MANA_SEGMENTS {
            return bad(
                "mana_max",
                "must be 8 while the HUD-A mana track has 8 segments",
            );
        }
        if self.missile_cost < 1 || self.missile_cost > self.mana_max {
            return bad("missile_cost", "must be in 1..=mana_max");
        }
        if self.regen_delay_ticks == 0 {
            return bad("regen_delay_ticks", "must be positive");
        }
        if self.regen_period_ticks == 0 {
            return bad("regen_period_ticks", "must be positive");
        }
        if self.ai_mana_reserve > self.mana_max {
            return bad("ai_mana_reserve", "must be at most mana_max");
        }
        if self.projectile_speed_px_per_tick == 0 {
            return bad("projectile_speed_px_per_tick", "must be positive");
        }
        if self.projectile_range_px == 0 {
            return bad("projectile_range_px", "must be positive");
        }
        if self.refire_ticks == 0 {
            return bad("refire_ticks", "must be positive");
        }
        // A full refill must fit the u16 regen timer.
        let full = u32::from(self.regen_delay_ticks)
            + u32::from(self.regen_period_ticks) * u32::from(self.mana_max);
        if full > u32::from(u16::MAX) {
            return bad("regen_period_ticks", "a full refill must fit 65535 ticks");
        }
        if self.empty_press == EmptyPress::Swat && !donut_moves {
            return bad(
                "empty_press",
                "`swat` is valid only while donut_moves is on",
            );
        }
        Ok(())
    }

    /// Ticks from a release to a full pool when the pool was emptied.
    pub fn full_refill_ticks(&self) -> u32 {
        u32::from(self.regen_delay_ticks)
            + u32::from(self.regen_period_ticks) * u32::from(self.mana_max)
    }
}

/// The only unlock the engine knows (the fan layer calls it "biscuit").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unlock {
    PrimaryRanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    Dry,
}

/// Capped-log events of the proposal (section 5). `tick` is the caller's
/// fixed tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaEvent {
    UnlockGranted {
        tick: u64,
        hero: u8,
        unlock: Unlock,
    },
    ManaSpent {
        tick: u64,
        hero: u8,
        left: u8,
    },
    ManaRegained {
        tick: u64,
        hero: u8,
        now: u8,
    },
    CastRejected {
        tick: u64,
        hero: u8,
        reason: RejectReason,
    },
}

/// The state a fixture declares (`unlocks: []` or `["primary_ranged"]`, and
/// the mana it starts with).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartState {
    pub unlocked: bool,
    pub mana: u8,
}

impl StartState {
    /// A scene that never grants the unlock: the default.
    pub const LOCKED: Self = Self {
        unlocked: false,
        mana: 0,
    };
}

/// What a scene declares about Donut's biscuit: the state it starts in and the
/// authored unlock zone (a placeholder x range until the Guild room exists,
/// proposal section 1). `NONE` is a scene that never grants the unlock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DonutSetup {
    pub start: StartState,
    /// The Tutorial Guild entry as a half-open `min_x..max_x` range in layout
    /// pixels (Donut's pivot), or `None` for no trigger.
    pub guild_zone: Option<(f32, f32)>,
}

impl DonutSetup {
    pub const NONE: Self = Self {
        start: StartState::LOCKED,
        guild_zone: None,
    };

    /// A fixture that starts with the biscuit and `mana` charges.
    pub const fn unlocked(mana: u8) -> Self {
        Self {
            start: StartState {
                unlocked: true,
                mana,
            },
            guild_zone: None,
        }
    }
}

/// Which restart is happening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartKind {
    /// Defeat retry: restores HP and full mana, the unlock stays.
    DefeatRetry,
    /// Checkpoint restart: same as a retry.
    CheckpointRestart,
    /// Fixture restart (R / Select): restores the fixture's declared state.
    Fixture(StartState),
}

/// Donut's unlock flag and charge pool. Owned by Donut, not by a player slot:
/// a solo swap neither refills nor resets it and the regen runs whoever
/// controls her.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DonutMana {
    unlocked: bool,
    mana: u8,
    /// Ticks since the last release (or since the pool was declared below
    /// full). Idle (0) while the pool is full.
    since_release: u16,
    /// Charges regained since the last release.
    regained: u8,
}

impl DonutMana {
    /// Locked: the default of every scene.
    pub const fn locked() -> Self {
        Self {
            unlocked: false,
            mana: 0,
            since_release: 0,
            regained: 0,
        }
    }

    /// The pool a fixture declares. A pool declared below full starts its regen
    /// timer at 0, as if released on tick 0.
    pub fn from_start(tuning: &ManaTuning, start: StartState) -> Self {
        if !start.unlocked {
            return Self::locked();
        }
        Self {
            unlocked: true,
            mana: start.mana.min(tuning.mana_max),
            since_release: 0,
            regained: 0,
        }
    }

    pub const fn unlocked(&self) -> bool {
        self.unlocked
    }

    pub const fn mana(&self) -> u8 {
        self.mana
    }

    /// Ticks since the last release (0 while idle at full).
    pub const fn since_release(&self) -> u16 {
        self.since_release
    }

    /// The unlock fires once; the pool starts full. `None` when already
    /// unlocked (idempotent).
    pub fn grant_unlock(&mut self, tuning: &ManaTuning, tick: u64, hero: u8) -> Option<ManaEvent> {
        if self.unlocked {
            return None;
        }
        self.unlocked = true;
        self.mana = tuning.mana_max;
        self.since_release = 0;
        self.regained = 0;
        Some(ManaEvent::UnlockGranted {
            tick,
            hero,
            unlock: Unlock::PrimaryRanged,
        })
    }

    /// The mana check of a cast start (also for a buffered press, judged on
    /// its own cast start tick).
    pub fn can_start_cast(&self, tuning: &ManaTuning) -> bool {
        self.unlocked && self.mana >= tuning.missile_cost
    }

    /// The companion AI pays the same cost and never spends below
    /// `ai_mana_reserve` charges: after the cast at least the reserve is left.
    pub fn ai_can_cast(&self, tuning: &ManaTuning) -> bool {
        self.can_start_cast(tuning) && self.mana - tuning.missile_cost >= tuning.ai_mana_reserve
    }

    /// Advance one fixed tick. Runs on every tick, also on the gather and cast
    /// ticks 0-3 of a cast that has not released. Call it **before**
    /// [`Self::release`] on the same tick, so a charge due on a release tick is
    /// credited first.
    pub fn step(&mut self, tuning: &ManaTuning, tick: u64) -> Option<ManaEvent> {
        if !self.unlocked || self.mana >= tuning.mana_max {
            self.since_release = 0;
            self.regained = 0;
            return None;
        }
        self.since_release = self.since_release.saturating_add(1);
        let due = u32::from(tuning.regen_delay_ticks)
            + u32::from(tuning.regen_period_ticks) * (u32::from(self.regained) + 1);
        if u32::from(self.since_release) != due {
            return None;
        }
        self.mana += 1;
        self.regained += 1;
        let event = ManaEvent::ManaRegained {
            tick,
            hero: DONUT_HERO,
            now: self.mana,
        };
        if self.mana >= tuning.mana_max {
            self.since_release = 0;
            self.regained = 0;
        }
        Some(event)
    }

    /// The release tick (cast tick 4, the bolt spawns): spend the cost and
    /// restart the regen timer, discarding partial progress. `None` and no
    /// change when the pool cannot pay.
    pub fn release(&mut self, tuning: &ManaTuning, tick: u64) -> Option<ManaEvent> {
        self.spend(tuning.missile_cost, tick)
    }

    /// DONUT-PROG-01 DP-Q23, appended: pay `cost` charges and restart the regen
    /// timer exactly as a missile release does (30 ticks of delay, then one per
    /// 40), discarding partial progress. `None` and no change while locked or
    /// when the pool cannot pay. [`Self::release`] is this with the missile's
    /// cost, so mana keeps one rule.
    pub fn spend(&mut self, cost: u8, tick: u64) -> Option<ManaEvent> {
        if !self.unlocked || self.mana < cost {
            return None;
        }
        self.mana -= cost;
        self.since_release = 0;
        self.regained = 0;
        Some(ManaEvent::ManaSpent {
            tick,
            hero: DONUT_HERO,
            left: self.mana,
        })
    }

    /// A press at 0 mana after the biscuit: only the event. It never touches
    /// the regen timer, so mashing cannot block a regen. `None` while locked.
    pub fn reject_dry(&self, tick: u64) -> Option<ManaEvent> {
        self.unlocked.then_some(ManaEvent::CastRejected {
            tick,
            hero: DONUT_HERO,
            reason: RejectReason::Dry,
        })
    }

    /// Restarts: a retry or a checkpoint restart keeps the unlock and refills
    /// the pool; a fixture restart restores the declared state.
    pub fn restart(&mut self, tuning: &ManaTuning, kind: RestartKind) {
        match kind {
            RestartKind::DefeatRetry | RestartKind::CheckpointRestart => {
                if self.unlocked {
                    self.mana = tuning.mana_max;
                }
                self.since_release = 0;
                self.regained = 0;
            }
            RestartKind::Fixture(start) => *self = Self::from_start(tuning, start),
        }
    }

    /// Fold into the state hash. **Nothing is mixed while locked**, so every
    /// scene that never grants the unlock keeps its golden hashes.
    pub fn hash_into(&self, mix: &mut impl FnMut(u64)) {
        if !self.unlocked {
            return;
        }
        mix(0x646f_6e75_745f_6d61); // "donut_ma" marker
        mix(u64::from(self.mana));
        mix(u64::from(self.since_release));
        mix(u64::from(self.regained));
    }
}

/// The authored once-only unlock zone at the Tutorial Guild beat: a half-open
/// AABB (minimum bounds inclusive, maximum bounds exclusive). It fires on the
/// first tick Donut's pivot is inside, whoever controls her, and never while
/// she is defeated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnlockTrigger {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    fired: bool,
}

impl UnlockTrigger {
    pub const fn new(min_x: f32, max_x: f32, min_y: f32, max_y: f32) -> Self {
        Self {
            min_x,
            max_x,
            min_y,
            max_y,
            fired: false,
        }
    }

    pub const fn fired(&self) -> bool {
        self.fired
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.min_x && x < self.max_x && y >= self.min_y && y < self.max_y
    }

    /// Evaluate on one fixed tick with Donut's pivot. Returns the grant event
    /// the first time it fires. A trigger whose unlock is already held (a
    /// fixture started unlocked) marks itself fired without an event.
    pub fn step(
        &mut self,
        mana: &mut DonutMana,
        tuning: &ManaTuning,
        tick: u64,
        pivot: (f32, f32),
        donut_defeated: bool,
    ) -> Option<ManaEvent> {
        if self.fired {
            return None;
        }
        if mana.unlocked() {
            self.fired = true;
            return None;
        }
        if donut_defeated || !self.contains(pivot.0, pivot.1) {
            return None;
        }
        self.fired = true;
        mana.grant_unlock(tuning, tick, DONUT_HERO)
    }

    /// A fixture restart re-arms the trigger from the declared start state.
    pub fn restart_fixture(&mut self, start: StartState) {
        self.fired = start.unlocked;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tuning() -> ManaTuning {
        ManaTuning::pinned(true).expect("pinned tuning loads")
    }

    fn unlocked_full() -> (DonutMana, ManaTuning) {
        let t = tuning();
        let mut m = DonutMana::locked();
        m.grant_unlock(&t, 0, DONUT_HERO).unwrap();
        (m, t)
    }

    /// Drive `ticks` ticks from `from` (exclusive), returning the events.
    fn run(m: &mut DonutMana, t: &ManaTuning, from: u64, ticks: u64) -> Vec<ManaEvent> {
        (from + 1..=from + ticks)
            .filter_map(|tick| m.step(t, tick))
            .collect()
    }

    #[test]
    fn tuning_hash_is_pinned() {
        // Fails when companion_magic_v0.json changes without updating
        // TUNING_HASH (and every replay header that recorded it).
        assert_eq!(content_hash(TUNING_JSON), TUNING_HASH);
        assert!(ManaTuning::pinned(false).is_ok());
    }

    #[test]
    fn a_drifted_file_is_a_clear_hash_failure() {
        let drifted =
            TUNING_JSON.replace("\"regen_period_ticks\": 40", "\"regen_period_ticks\": 41");
        match ManaTuning::from_json_pinned(&drifted, TUNING_HASH, true) {
            Err(TuningError::HashMismatch { expected, actual }) => {
                assert_eq!(expected, TUNING_HASH);
                assert_ne!(actual, expected);
            }
            other => panic!("expected a hash mismatch, got {other:?}"),
        }
        // The same edit parses fine unpinned: the pin is what catches it.
        assert!(ManaTuning::from_json(&drifted, true).is_ok());
    }

    #[test]
    fn pinned_values_match_the_adopted_proposal() {
        let t = tuning();
        assert_eq!((t.mana_max, t.missile_cost, t.missile_damage), (8, 1, 1));
        assert_eq!((t.regen_delay_ticks, t.regen_period_ticks), (30, 40));
        assert_eq!(t.ai_mana_reserve, 2);
        assert_eq!(t.empty_press, EmptyPress::Dry);
        assert_eq!(
            (
                t.projectile_speed_px_per_tick,
                t.projectile_range_px,
                t.refire_ticks
            ),
            (6, 240, 16)
        );
        assert_eq!(t.full_refill_ticks(), 350);
    }

    #[test]
    fn bad_configs_are_rejected_and_name_the_field() {
        let field = |text: &str, donut_moves: bool| match ManaTuning::from_json(text, donut_moves) {
            Err(TuningError::Field { field, .. }) => field,
            other => panic!("expected a field error, got {other:?}"),
        };
        let ok = TUNING_JSON;
        assert_eq!(
            field(&ok.replace("\"mana_max\": 8", "\"mana_max\": 6"), true),
            "mana_max"
        );
        assert_eq!(
            field(
                &ok.replace("\"missile_cost\": 1", "\"missile_cost\": 0"),
                true
            ),
            "missile_cost"
        );
        assert_eq!(
            field(
                &ok.replace("\"missile_cost\": 1", "\"missile_cost\": 9"),
                true
            ),
            "missile_cost"
        );
        assert_eq!(
            field(
                &ok.replace("\"regen_delay_ticks\": 30", "\"regen_delay_ticks\": 0"),
                true
            ),
            "regen_delay_ticks"
        );
        assert_eq!(
            field(
                &ok.replace("\"regen_period_ticks\": 40", "\"regen_period_ticks\": 0"),
                true
            ),
            "regen_period_ticks"
        );
        assert_eq!(
            field(
                &ok.replace("\"schema_version\": 1", "\"schema_version\": 2"),
                true
            ),
            "schema_version"
        );
        // `swat` is valid only while donut_moves is on.
        let swat = ok.replace("\"empty_press\": \"dry\"", "\"empty_press\": \"swat\"");
        assert!(ManaTuning::from_json(&swat, true).is_ok());
        assert_eq!(field(&swat, false), "empty_press");
        // Unknown and missing fields are parse errors.
        let extra = ok.replace("\"mana_max\": 8,", "\"mana_max\": 8, \"gore\": 1,");
        assert!(matches!(
            ManaTuning::from_json(&extra, true),
            Err(TuningError::Parse(_))
        ));
        assert!(matches!(
            ManaTuning::from_json("{}", true),
            Err(TuningError::Parse(_))
        ));
        assert!(matches!(
            ManaTuning::from_json("not json", true),
            Err(TuningError::Parse(_))
        ));
    }

    #[test]
    fn the_flag_is_false_by_default_and_a_locked_pool_never_casts_or_regens() {
        let t = tuning();
        let mut m = DonutMana::default();
        assert!(!m.unlocked());
        assert_eq!(m, DonutMana::locked());
        assert!(!m.can_start_cast(&t));
        assert!(m.release(&t, 1).is_none());
        assert!(m.reject_dry(1).is_none());
        assert!(run(&mut m, &t, 0, 400).is_empty());
        assert_eq!(m, DonutMana::locked());
    }

    #[test]
    fn locked_state_hashes_nothing_and_unlocked_state_hashes_mana() {
        let t = tuning();
        let mut locked = Vec::new();
        DonutMana::locked().hash_into(&mut |v| locked.push(v));
        assert!(locked.is_empty(), "a locked pool must not touch the hash");
        let mut default = Vec::new();
        DonutMana::default().hash_into(&mut |v| default.push(v));
        assert!(default.is_empty());

        let (mut m, _) = unlocked_full();
        let mut full = Vec::new();
        m.hash_into(&mut |v| full.push(v));
        assert!(!full.is_empty());
        m.release(&t, 5);
        let mut spent = Vec::new();
        m.hash_into(&mut |v| spent.push(v));
        assert_ne!(full, spent, "the state hash includes mana");
        // Same timer, different regen progress: also distinct.
        m.step(&t, 6);
        let mut later = Vec::new();
        m.hash_into(&mut |v| later.push(v));
        assert_ne!(spent, later, "the state hash includes the regen timer");
    }

    #[test]
    fn unlock_starts_full_fires_once_and_is_idempotent() {
        let t = tuning();
        let mut m = DonutMana::locked();
        let first = m.grant_unlock(&t, 12, 1);
        assert_eq!(
            first,
            Some(ManaEvent::UnlockGranted {
                tick: 12,
                hero: 1,
                unlock: Unlock::PrimaryRanged
            })
        );
        assert_eq!((m.unlocked(), m.mana()), (true, 8));
        m.release(&t, 13);
        assert_eq!(m.grant_unlock(&t, 14, 1), None);
        assert_eq!(m.mana(), 7, "a second grant never refills");
    }

    #[test]
    fn the_charge_is_spent_on_the_release_and_a_cancelled_gather_spends_nothing() {
        let (mut m, t) = unlocked_full();
        // Cast ticks 0-3: the gather. A hurt cancels it: the adapter never
        // calls release, and step() alone changes nothing at full.
        for tick in 1..=3 {
            assert!(m.step(&t, tick).is_none());
            assert_eq!(m.mana(), 8);
        }
        // The release (cast tick 4).
        let ev = m.release(&t, 4);
        assert_eq!(
            ev,
            Some(ManaEvent::ManaSpent {
                tick: 4,
                hero: 1,
                left: 7
            })
        );
        assert_eq!(m.mana(), 7);
    }

    #[test]
    fn dry_press_spawns_and_spends_nothing_and_never_resets_the_regen() {
        let (mut m, t) = unlocked_full();
        for tick in 1..=8 {
            m.release(&t, tick);
        }
        assert_eq!(m.mana(), 0);
        assert!(!m.can_start_cast(&t));
        assert!(m.release(&t, 9).is_none(), "no spend at 0");
        let before = m;
        let ev = m.reject_dry(9);
        assert_eq!(
            ev,
            Some(ManaEvent::CastRejected {
                tick: 9,
                hero: 1,
                reason: RejectReason::Dry
            })
        );
        assert_eq!(m, before, "a dry press changes no state");
        // Mashing during the delay cannot block the regen: charge 1 at
        // release(8) + 70.
        let mut regained_at = None;
        for tick in 9..=200 {
            if tick % 3 == 0 {
                m.reject_dry(tick);
            }
            if let Some(ManaEvent::ManaRegained { tick, .. }) = m.step(&t, tick) {
                regained_at = Some(tick);
                break;
            }
        }
        assert_eq!(regained_at, Some(8 + 70));
    }

    #[test]
    fn charge_k_returns_exactly_at_release_plus_30_plus_40k_and_a_refill_takes_350() {
        let (mut m, t) = unlocked_full();
        let release = 100u64;
        for _ in 0..8 {
            m.release(&t, release); // eight bolts on one tick would be a test shortcut
        }
        assert_eq!(m.mana(), 0);
        let events = run(&mut m, &t, release, 400);
        assert_eq!(events.len(), 8);
        for (i, ev) in events.iter().enumerate() {
            let k = i as u64 + 1;
            assert_eq!(
                *ev,
                ManaEvent::ManaRegained {
                    tick: release + 30 + 40 * k,
                    hero: 1,
                    now: k as u8
                },
                "charge {k}"
            );
        }
        assert_eq!(
            events.last(),
            Some(&ManaEvent::ManaRegained {
                tick: release + 350,
                hero: 1,
                now: 8
            })
        );
        assert_eq!(m.mana(), 8, "never above max");
        assert_eq!(m.since_release(), 0, "idle at full");
        assert!(run(&mut m, &t, release + 400, 500).is_empty());
        assert_eq!(m.mana(), 8);
    }

    #[test]
    fn a_release_mid_period_discards_partial_progress() {
        let (mut m, t) = unlocked_full();
        m.release(&t, 1); // 7, timer 0
        // Run to just before charge 1 (due at tick 71), then release again.
        let ev = run(&mut m, &t, 1, 69); // ticks 2..=70
        assert!(ev.is_empty());
        m.release(&t, 70); // 6, partial progress discarded
        assert_eq!(m.mana(), 6);
        // Charge 1 now returns at 70 + 70, not at 71.
        let ev = run(&mut m, &t, 70, 69);
        assert!(ev.is_empty(), "no charge at the old due tick");
        let ev = m.step(&t, 140);
        assert_eq!(
            ev,
            Some(ManaEvent::ManaRegained {
                tick: 140,
                hero: 1,
                now: 7
            })
        );
    }

    #[test]
    fn the_delay_runs_during_the_cast_ticks_and_a_burst_costs_what_the_proposal_says() {
        // Burst of n bolts, 16 ticks apart: the last charge is back at
        // last_release + 30 + 40 * k for the k charges still to return.
        let (mut m, t) = unlocked_full();
        let mut tick = 0u64;
        let n = 4u64;
        let mut last_release = 0;
        for i in 0..n {
            tick = 1 + 16 * i;
            // Regen runs on every tick, including cast ticks 0-3 of a cast
            // that has not released (no events inside a burst: delay > gap).
            for t_i in (tick.saturating_sub(15).max(1))..tick {
                assert!(m.step(&t, t_i).is_none());
            }
            m.step(&t, tick);
            m.release(&t, tick);
            last_release = tick;
        }
        assert_eq!(m.mana(), 8 - n as u8);
        let events = run(&mut m, &t, tick, 30 + 40 * n + 40);
        assert_eq!(
            events.first(),
            Some(&ManaEvent::ManaRegained {
                tick: last_release + 70,
                hero: 1,
                now: 8 - n as u8 + 1
            })
        );
        assert_eq!(m.mana(), 8);
        // Burst cost formula: 16(n-1) + 30 + 40n from the first release to full.
        let full_at = events.last().map(|e| match e {
            ManaEvent::ManaRegained { tick, .. } => *tick,
            _ => unreachable!(),
        });
        assert_eq!(full_at, Some(1 + 16 * (n - 1) + 30 + 40 * n));
    }

    #[test]
    fn a_charge_due_on_the_release_tick_is_credited_before_the_spend() {
        let (mut m, t) = unlocked_full();
        m.release(&t, 0); // 7
        // Charge 1 is due on tick 70; step runs first, then the release.
        run(&mut m, &t, 0, 69);
        assert_eq!(
            m.step(&t, 70),
            Some(ManaEvent::ManaRegained {
                tick: 70,
                hero: 1,
                now: 8
            })
        );
        assert_eq!(
            m.release(&t, 70),
            Some(ManaEvent::ManaSpent {
                tick: 70,
                hero: 1,
                left: 7
            })
        );
    }

    #[test]
    fn spend_check_uses_the_cost_and_a_buffered_press_is_checked_at_its_own_start() {
        let (mut m, t) = unlocked_full();
        for tick in 1..=7 {
            m.release(&t, tick);
        }
        assert_eq!(m.mana(), 1);
        assert!(m.can_start_cast(&t), "the last charge can start a cast");
        // A buffered press is judged when its cast starts: another cast spent
        // the last charge in between.
        m.release(&t, 8);
        assert!(!m.can_start_cast(&t));
    }

    #[test]
    fn the_ai_never_spends_below_its_reserve() {
        let (mut m, t) = unlocked_full();
        let mut casts = 0;
        for tick in 1..=8 {
            if m.ai_can_cast(&t) {
                m.release(&t, tick);
                casts += 1;
            }
        }
        assert_eq!(casts, 6, "8 charges minus the reserve of 2");
        assert_eq!(m.mana(), 2);
        assert!(!m.ai_can_cast(&t));
        assert!(m.can_start_cast(&t), "a player can still drain her");
    }

    #[test]
    fn a_swap_keeps_the_timer_and_mana() {
        // The pool has no player slot: nothing in the API takes one, so a swap
        // is a no-op by construction. Pin that a swap mid-regen changes nothing
        // by comparing against an undisturbed twin.
        let (mut a, t) = unlocked_full();
        let mut b = a;
        a.release(&t, 1);
        b.release(&t, 1);
        run(&mut a, &t, 1, 20);
        run(&mut b, &t, 1, 20);
        // (swap happens here: no call)
        run(&mut a, &t, 21, 60);
        run(&mut b, &t, 21, 60);
        assert_eq!(a, b);
        assert_eq!(a.mana(), 8, "charge 1 returned at 1 + 70 = 71");
    }

    #[test]
    fn restarts_keep_the_unlock_and_refill_and_a_fixture_restart_restores_its_declaration() {
        let (mut m, t) = unlocked_full();
        for tick in 1..=5 {
            m.release(&t, tick);
        }
        assert_eq!(m.mana(), 3);
        m.restart(&t, RestartKind::DefeatRetry);
        assert_eq!((m.unlocked(), m.mana(), m.since_release()), (true, 8, 0));
        m.release(&t, 9);
        m.restart(&t, RestartKind::CheckpointRestart);
        assert_eq!((m.unlocked(), m.mana()), (true, 8));
        // A retry before the unlock leaves it locked and hashes nothing.
        let mut locked = DonutMana::locked();
        locked.restart(&t, RestartKind::DefeatRetry);
        assert_eq!(locked, DonutMana::locked());
        // A fixture restart restores the declared state, whatever happened.
        m.restart(
            &t,
            RestartKind::Fixture(StartState {
                unlocked: true,
                mana: 3,
            }),
        );
        assert_eq!((m.unlocked(), m.mana()), (true, 3));
        m.restart(&t, RestartKind::Fixture(StartState::LOCKED));
        assert_eq!(m, DonutMana::locked());
    }

    #[test]
    fn a_fixture_declared_below_full_regens_from_tick_zero() {
        let t = tuning();
        let mut m = DonutMana::from_start(
            &t,
            StartState {
                unlocked: true,
                mana: 5,
            },
        );
        let ev = run(&mut m, &t, 0, 70);
        assert_eq!(
            ev,
            vec![ManaEvent::ManaRegained {
                tick: 70,
                hero: 1,
                now: 6
            }]
        );
        // Declared mana above max is clamped.
        let m = DonutMana::from_start(
            &t,
            StartState {
                unlocked: true,
                mana: 99,
            },
        );
        assert_eq!(m.mana(), 8);
    }

    #[test]
    fn the_trigger_is_a_half_open_box_that_fires_once_for_whoever_controls_donut() {
        let t = tuning();
        let mut mana = DonutMana::locked();
        let mut trig = UnlockTrigger::new(100.0, 140.0, 0.0, 50.0);
        assert!(trig.contains(100.0, 0.0), "minimum bounds inclusive");
        assert!(!trig.contains(140.0, 10.0), "maximum bounds exclusive");
        assert!(!trig.contains(99.9, 10.0));
        assert!(trig.step(&mut mana, &t, 1, (99.0, 10.0), false).is_none());
        assert!(!mana.unlocked());
        let ev = trig.step(&mut mana, &t, 2, (100.0, 10.0), false);
        assert_eq!(
            ev,
            Some(ManaEvent::UnlockGranted {
                tick: 2,
                hero: 1,
                unlock: Unlock::PrimaryRanged
            })
        );
        assert!(trig.fired() && mana.unlocked());
        // Idempotent: walking back and forth grants nothing more and never
        // refills a spent pool.
        mana.release(&t, 3);
        assert!(trig.step(&mut mana, &t, 4, (110.0, 10.0), false).is_none());
        assert_eq!(mana.mana(), 7);
    }

    #[test]
    fn the_trigger_waits_for_a_defeated_donut_to_be_restored() {
        let t = tuning();
        let mut mana = DonutMana::locked();
        let mut trig = UnlockTrigger::new(0.0, 100.0, 0.0, 100.0);
        // Inside the zone while defeated (the 50-tick party-defeat delay).
        for tick in 1..=50 {
            assert!(trig.step(&mut mana, &t, tick, (10.0, 10.0), true).is_none());
        }
        assert!(!mana.unlocked() && !trig.fired());
        // The first tick after the retry restores her.
        let ev = trig.step(&mut mana, &t, 51, (10.0, 10.0), false);
        assert!(matches!(
            ev,
            Some(ManaEvent::UnlockGranted { tick: 51, .. })
        ));
    }

    #[test]
    fn the_unlock_survives_retry_and_a_started_unlocked_fixture_does_not_replay_the_guild() {
        let t = tuning();
        let mut mana = DonutMana::locked();
        let mut trig = UnlockTrigger::new(0.0, 100.0, 0.0, 100.0);
        trig.step(&mut mana, &t, 1, (10.0, 10.0), false);
        mana.restart(&t, RestartKind::DefeatRetry);
        assert!(mana.unlocked(), "run progression survives a retry");
        assert!(trig.step(&mut mana, &t, 9, (10.0, 10.0), false).is_none());
        // A fixture that declares the unlock: the trigger never fires.
        let start = StartState {
            unlocked: true,
            mana: 8,
        };
        let mut declared = DonutMana::from_start(&t, start);
        let mut trig = UnlockTrigger::new(0.0, 100.0, 0.0, 100.0);
        assert!(
            trig.step(&mut declared, &t, 1, (10.0, 10.0), false)
                .is_none()
        );
        assert!(trig.fired());
        // A fixture restart to the locked state re-arms it.
        trig.restart_fixture(StartState::LOCKED);
        assert!(!trig.fired());
    }

    #[test]
    fn replays_are_deterministic() {
        let play = || {
            let (mut m, t) = unlocked_full();
            let mut log = Vec::new();
            let mut hash = 0xcbf2_9ce4_8422_2325_u64;
            for tick in 1..=600u64 {
                if let Some(ev) = m.step(&t, tick) {
                    log.push(ev);
                }
                if tick % 23 == 0 {
                    match m.release(&t, tick) {
                        Some(ev) => log.push(ev),
                        None => log.extend(m.reject_dry(tick)),
                    }
                }
                m.hash_into(&mut |v| {
                    hash ^= v;
                    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
                });
            }
            (log, hash)
        };
        assert_eq!(play(), play());
    }
}
