//! The generic story-unlock ledger (CARL-PROG-01 CP-A, spec section 3).
//!
//! Bevy-free, deterministic and IP-neutral: a capability is a data id with a
//! `hero`, a `slot` and an optional `requires` list (CP-Q44); a story trigger
//! is a once-only zone or event match that carries a `grants` list, so one
//! story event can grant several capabilities, possibly to different heroes,
//! in list order on one tick. Nothing here names a franchise item or
//! character: the names live in the `display` and notice fields of
//! `assets/data/gameplay/story_unlocks_v0.json` (the fan layer).
//!
//! The ledger is run progression (CP-Q07): a grant survives a defeat retry, a
//! checkpoint respawn, a solo swap and a 2P join or drop; a fixture restart
//! restores the declared start state (the pins). It mixes **nothing** into the
//! state hash while empty, so a scene without a grant hashes as before, and a
//! read-through capability (the biscuit, CP-Q45) is never stored, hashed,
//! receipted or evented by it: [`UnlockLedger::holds`] asks the view the caller
//! passes in, and nothing else.
//!
//! This module owns the model only. The combat state owns one
//! [`StoryRuntime`], feeds it the tick's events and hero pivots and turns the
//! returned [`Receipt`]s into `CombatEvent::CapabilityGranted`.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::donut_mana::content_hash;

/// The data file, embedded so the binary and the tests read the same bytes.
pub const TABLE_JSON: &str = include_str!("../assets/data/gameplay/story_unlocks_v0.json");

/// The committed FNV-1a 64 content hash of [`TABLE_JSON`]. Editing the file
/// without updating this value fails [`StoryTable::pinned`] and the pin test.
pub const TABLE_HASH: u64 = 0x28d0_302f_b11b_7778;

/// The schema version [`StoryTable`] accepts.
pub const SCHEMA_VERSION: u32 = 1;

/// The version of [`LedgerSnapshot`].
pub const SNAPSHOT_VERSION: u32 = 1;

/// The longest grant notice, in characters (CP-Q11).
pub const NOTICE_MAX_CHARS: usize = 13;

/// The highest hero index a row may name (Carl 0, Donut 1).
pub const MAX_HERO: u8 = 1;

/// The `primary_ranged` capability of the biscuit: a read-through (CP-Q06).
pub const PRIMARY_RANGED: CapabilityId = CapabilityId("primary_ranged");

/// The thrown-charge special row id of Carl.
pub const SPECIAL_THROWN_CHARGE: CapabilityId = CapabilityId("special_thrown_charge");

/// The reinforced-fist row id of Carl.
pub const FIST_REINFORCED: CapabilityId = CapabilityId("fist_reinforced");

/// A capability id: a data id, compared and ordered by its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityId(&'static str);

impl CapabilityId {
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }

    /// FNV-1a 64 of the id text (what the state and event hashes mix).
    pub const fn hash(self) -> u64 {
        content_hash(self.0)
    }
}

/// Which button or system a capability extends (CP-Q44).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Slot {
    /// The Circle/B equipped special.
    Special,
    /// No button: a standing property.
    Passive,
    /// A change to the primary button.
    PrimaryMod,
}

/// Where a capability is stored (CP-Q45).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Storage {
    /// Held by the [`UnlockLedger`].
    Ledger,
    /// Held elsewhere and only read: the ledger never stores, hashes,
    /// receipts or emits it.
    ReadThrough,
}

/// The scene a pin or a trigger belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneKind {
    /// The opening route corridor.
    Corridor,
    /// The DEMO-01 hall.
    Demo,
    /// The CF02 microarena.
    CfArena,
    /// The Floor 01 boss arena.
    Boss,
}

/// How a data event id is matched against the combat events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventMatch {
    /// `CombatEvent::EnemyDefeated` of the boss enemy id.
    EnemyDefeatedBoss,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityRow {
    pub id: String,
    pub hero: u8,
    pub slot: Slot,
    pub storage: Storage,
    #[serde(default)]
    pub requires: Vec<String>,
    pub display: String,
    pub cp_q: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventRow {
    pub id: String,
    #[serde(rename = "match")]
    pub matcher: EventMatch,
    pub cp_q: String,
}

/// A half-open zone over one hero's pivot (minimum bounds inclusive, maximum
/// bounds exclusive), in layout pixels.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneRow {
    pub hero: u8,
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
}

impl ZoneRow {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.min_x && x < self.max_x && y >= self.min_y && y < self.max_y
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerKind {
    Zone,
    Event,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriggerRow {
    pub id: String,
    pub kind: TriggerKind,
    #[serde(default)]
    pub zone: Option<ZoneRow>,
    #[serde(default)]
    pub event: Option<String>,
    pub grants: Vec<String>,
    /// The story flag that arms it (`carl_prog`, later `donut_prog`).
    pub flag: String,
    /// The scenes it is armed in under its flag. Empty: a stub that only a
    /// fixture arms by id (the Hoarder does not exist yet, K1).
    #[serde(default)]
    pub armed_in: Vec<SceneKind>,
    pub cp_q: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinRow {
    pub id: String,
    pub scene: SceneKind,
    pub flag: String,
    pub holds: Vec<String>,
    pub cp_q: String,
}

/// A story flag and the heroes whose rows it may grant. A debug or story flag
/// applies only its own heroes' rows: `carl_prog` alone never grants Donut's
/// rows of a shared trigger, and a later `donut_prog` alone never grants Carl's.
/// A trigger armed without a flag (a fixture) grants every row of its list.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoryFlagRow {
    pub flag: String,
    pub heroes: Vec<u8>,
    pub cp_q: String,
}

/// What a debug flag grants at start (`--carl-dynamite`, `--carl-gauntlet`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartFlagRow {
    pub flag: String,
    pub holds: Vec<String>,
    pub cp_q: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoticeRow {
    pub capability: String,
    pub text: String,
    pub cp_q: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoryError {
    /// The JSON did not parse or has an unknown or missing field.
    Parse(String),
    /// A field failed validation; `field` names it.
    Field { field: String, reason: String },
    /// The file's content hash is not the pinned one.
    HashMismatch { expected: u64, actual: u64 },
}

impl core::fmt::Display for StoryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "story unlocks: {msg}"),
            Self::Field { field, reason } => {
                write!(f, "story unlocks: field `{field}`: {reason}")
            }
            Self::HashMismatch { expected, actual } => write!(
                f,
                "story unlocks hash {actual:#018x} is not the pinned {expected:#018x}"
            ),
        }
    }
}

impl std::error::Error for StoryError {}

fn bad(field: impl Into<String>, reason: impl Into<String>) -> StoryError {
    StoryError::Field {
        field: field.into(),
        reason: reason.into(),
    }
}

/// `assets/data/gameplay/story_unlocks_v0.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoryTable {
    pub schema_version: u32,
    pub provenance: String,
    pub capabilities: Vec<CapabilityRow>,
    #[serde(default)]
    pub story_flags: Vec<StoryFlagRow>,
    #[serde(default)]
    pub events: Vec<EventRow>,
    #[serde(default)]
    pub triggers: Vec<TriggerRow>,
    #[serde(default)]
    pub pins: Vec<PinRow>,
    #[serde(default)]
    pub start_flags: Vec<StartFlagRow>,
    #[serde(default)]
    pub notices: Vec<NoticeRow>,
}

impl StoryTable {
    /// Parse and validate.
    pub fn from_json(text: &str) -> Result<Self, StoryError> {
        let table: Self =
            serde_json::from_str(text).map_err(|err| StoryError::Parse(err.to_string()))?;
        table.validate()?;
        Ok(table)
    }

    /// Parse the embedded file and require the pinned content hash.
    pub fn pinned() -> Result<Self, StoryError> {
        Self::from_json_pinned(TABLE_JSON, TABLE_HASH)
    }

    /// Parse `text` and require its content hash to equal `expected`.
    pub fn from_json_pinned(text: &str, expected: u64) -> Result<Self, StoryError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(StoryError::HashMismatch { expected, actual });
        }
        Self::from_json(text)
    }

    /// The pinned table every simulation reads: one shared static.
    pub fn active() -> &'static Self {
        static ACTIVE: OnceLock<StoryTable> = OnceLock::new();
        ACTIVE.get_or_init(|| Self::pinned().expect("the embedded story unlock data is pinned"))
    }

    /// Give a validated table the `'static` lifetime (tests build stub tables
    /// this way; the shipped one is [`Self::active`]).
    pub fn leak(self) -> &'static Self {
        Box::leak(Box::new(self))
    }

    pub fn capability(&self, id: &str) -> Option<&CapabilityRow> {
        self.capabilities.iter().find(|row| row.id == id)
    }

    pub fn trigger(&self, id: &str) -> Option<&TriggerRow> {
        self.triggers.iter().find(|row| row.id == id)
    }

    pub fn notice(&self, capability: &str) -> Option<&NoticeRow> {
        self.notices.iter().find(|row| row.capability == capability)
    }

    pub fn validate(&self) -> Result<(), StoryError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(bad("schema_version", format!("expected {SCHEMA_VERSION}")));
        }
        if self.capabilities.is_empty() {
            return Err(bad("capabilities", "at least one capability"));
        }
        for (i, row) in self.capabilities.iter().enumerate() {
            let at = |name: &str| format!("capabilities[{i}].{name}");
            if !valid_id(&row.id) {
                return Err(bad(at("id"), "lower snake_case ascii"));
            }
            if self.capabilities[..i].iter().any(|prev| prev.id == row.id) {
                return Err(bad(at("id"), format!("duplicate id {}", row.id)));
            }
            if row.hero > MAX_HERO {
                return Err(bad(at("hero"), format!("hero index is 0..={MAX_HERO}")));
            }
            if row.display.trim().is_empty() {
                return Err(bad(at("display"), "empty"));
            }
            if row.cp_q.is_empty() {
                return Err(bad(at("cp_q"), "empty"));
            }
            for required in &row.requires {
                if *required == row.id {
                    return Err(bad(at("requires"), "a capability cannot require itself"));
                }
                if self.capability(required).is_none() {
                    return Err(bad(
                        at("requires"),
                        format!("unknown capability {required}"),
                    ));
                }
            }
        }
        self.check_requires_acyclic()?;
        for (i, row) in self.story_flags.iter().enumerate() {
            let at = |name: &str| format!("story_flags[{i}].{name}");
            if !valid_id(&row.flag) {
                return Err(bad(at("flag"), "lower snake_case ascii"));
            }
            if self.story_flags[..i]
                .iter()
                .any(|prev| prev.flag == row.flag)
            {
                return Err(bad(at("flag"), "duplicate flag"));
            }
            if row.heroes.is_empty() || row.heroes.iter().any(|h| *h > MAX_HERO) {
                return Err(bad(at("heroes"), format!("hero indices in 0..={MAX_HERO}")));
            }
        }
        for (i, row) in self.events.iter().enumerate() {
            if !valid_id(&row.id) {
                return Err(bad(format!("events[{i}].id"), "lower snake_case ascii"));
            }
            if self.events[..i].iter().any(|prev| prev.id == row.id) {
                return Err(bad(format!("events[{i}].id"), "duplicate id"));
            }
        }
        for (i, row) in self.triggers.iter().enumerate() {
            let at = |name: &str| format!("triggers[{i}].{name}");
            if !valid_id(&row.id) {
                return Err(bad(at("id"), "lower snake_case ascii"));
            }
            if self.triggers[..i].iter().any(|prev| prev.id == row.id) {
                return Err(bad(at("id"), "duplicate id"));
            }
            if !valid_id(&row.flag) {
                return Err(bad(at("flag"), "lower snake_case ascii"));
            }
            if !self.story_flags.iter().any(|f| f.flag == row.flag) {
                return Err(bad(at("flag"), "undeclared story flag"));
            }
            match (row.kind, &row.zone, &row.event) {
                (TriggerKind::Zone, Some(zone), None) => {
                    if zone.hero > MAX_HERO {
                        return Err(bad(
                            at("zone.hero"),
                            format!("hero index is 0..={MAX_HERO}"),
                        ));
                    }
                    let finite = [zone.min_x, zone.max_x, zone.min_y, zone.max_y]
                        .iter()
                        .all(|v| v.is_finite());
                    if !finite || zone.min_x >= zone.max_x || zone.min_y >= zone.max_y {
                        return Err(bad(at("zone"), "needs finite min < max on both axes"));
                    }
                }
                (TriggerKind::Event, None, Some(event)) => {
                    if !self.events.iter().any(|row| row.id == *event) {
                        return Err(bad(at("event"), format!("unknown event {event}")));
                    }
                }
                (TriggerKind::Zone, ..) => {
                    return Err(bad(at("zone"), "a zone trigger has a zone and no event"));
                }
                (TriggerKind::Event, ..) => {
                    return Err(bad(
                        at("event"),
                        "an event trigger has an event and no zone",
                    ));
                }
            }
            self.check_grant_list(&at("grants"), &row.grants)?;
            if row.cp_q.is_empty() {
                return Err(bad(at("cp_q"), "empty"));
            }
        }
        for (i, row) in self.pins.iter().enumerate() {
            let at = |name: &str| format!("pins[{i}].{name}");
            if !valid_id(&row.id) {
                return Err(bad(at("id"), "lower snake_case ascii"));
            }
            if self.pins[..i].iter().any(|prev| prev.id == row.id) {
                return Err(bad(at("id"), "duplicate id"));
            }
            if !valid_id(&row.flag) {
                return Err(bad(at("flag"), "lower snake_case ascii"));
            }
            if !self.story_flags.iter().any(|f| f.flag == row.flag) {
                return Err(bad(at("flag"), "undeclared story flag"));
            }
            self.check_grant_list(&at("holds"), &row.holds)?;
        }
        for (i, row) in self.start_flags.iter().enumerate() {
            let at = |name: &str| format!("start_flags[{i}].{name}");
            if !valid_id(&row.flag) {
                return Err(bad(at("flag"), "lower snake_case ascii"));
            }
            if self.start_flags[..i]
                .iter()
                .any(|prev| prev.flag == row.flag)
            {
                return Err(bad(at("flag"), "duplicate flag"));
            }
            self.check_grant_list(&at("holds"), &row.holds)?;
        }
        for (i, row) in self.notices.iter().enumerate() {
            let at = |name: &str| format!("notices[{i}].{name}");
            match self.capability(&row.capability) {
                None => return Err(bad(at("capability"), "unknown capability")),
                Some(cap) if cap.storage == Storage::ReadThrough => {
                    return Err(bad(
                        at("capability"),
                        "a read-through capability has no notice",
                    ));
                }
                Some(_) => {}
            }
            if self.notices[..i]
                .iter()
                .any(|prev| prev.capability == row.capability)
            {
                return Err(bad(at("capability"), "duplicate notice"));
            }
            let chars = row.text.chars().count();
            if chars == 0 || chars > NOTICE_MAX_CHARS || !row.text.is_ascii() {
                return Err(bad(
                    at("text"),
                    format!("1..={NOTICE_MAX_CHARS} ascii characters (CP-Q11)"),
                ));
            }
        }
        Ok(())
    }

    /// A non-empty list of known, stored (not read-through), distinct ids.
    fn check_grant_list(&self, field: &str, ids: &[String]) -> Result<(), StoryError> {
        if ids.is_empty() {
            return Err(bad(field, "needs at least one capability"));
        }
        for (i, id) in ids.iter().enumerate() {
            match self.capability(id) {
                None => return Err(bad(field, format!("unknown capability {id}"))),
                Some(cap) if cap.storage == Storage::ReadThrough => {
                    return Err(bad(
                        field,
                        format!("{id} is a read-through capability: it is never granted"),
                    ));
                }
                Some(_) => {}
            }
            if ids[..i].contains(id) {
                return Err(bad(field, format!("duplicate capability {id}")));
            }
        }
        Ok(())
    }

    fn check_requires_acyclic(&self) -> Result<(), StoryError> {
        // A capability can reach at most `len` others without repeating, so a
        // walk longer than that is a cycle.
        for (i, row) in self.capabilities.iter().enumerate() {
            let mut frontier: Vec<&str> = row.requires.iter().map(String::as_str).collect();
            let mut steps = 0;
            while let Some(id) = frontier.pop() {
                steps += 1;
                if id == row.id || steps > self.capabilities.len() * self.capabilities.len() {
                    return Err(bad(
                        format!("capabilities[{i}].requires"),
                        "a requires cycle",
                    ));
                }
                if let Some(next) = self.capability(id) {
                    frontier.extend(next.requires.iter().map(String::as_str));
                }
            }
        }
        Ok(())
    }
}

/// Lower snake_case ascii: a letter first, then letters, digits or `_`.
fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// The values of the capabilities that live elsewhere (CP-Q45): today only the
/// biscuit. The caller reads them from their owner every time; the ledger
/// keeps none of it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReadThrough {
    /// `DonutMana::unlocked()`.
    pub primary_ranged: bool,
}

impl ReadThrough {
    fn read(&self, id: CapabilityId) -> bool {
        id == PRIMARY_RANGED && self.primary_ranged
    }
}

/// Which restart is happening (the shape of `donut_mana::RestartKind`, without
/// Donut's start state).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryRestart {
    /// Defeat retry: the grants stay.
    DefeatRetry,
    /// Checkpoint respawn: the grants stay.
    CheckpointRestart,
    /// Fixture restart (R / Select): the declared start state comes back.
    Fixture,
}

impl From<crate::donut_mana::RestartKind> for StoryRestart {
    fn from(kind: crate::donut_mana::RestartKind) -> Self {
        use crate::donut_mana::RestartKind;
        match kind {
            RestartKind::DefeatRetry => Self::DefeatRetry,
            RestartKind::CheckpointRestart => Self::CheckpointRestart,
            RestartKind::Fixture(_) => Self::Fixture,
        }
    }
}

/// One grant: who got what, when and from which trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub hero: u8,
    pub capability: CapabilityId,
    pub tick: u64,
    pub source: String,
}

/// The held capabilities per hero and their grant receipts.
#[derive(Debug, Clone)]
pub struct UnlockLedger {
    table: &'static StoryTable,
    /// The declared start state (the pins), sorted.
    start: Vec<(u8, CapabilityId)>,
    /// Held now, sorted by `(hero, id)`.
    held: Vec<(u8, CapabilityId)>,
    /// Receipt ticks are attempt-local and keep the previous attempt's tick
    /// after a defeat retry (spec section 3): deterministic, stated so nobody
    /// "fixes" it. A pin has no receipt.
    receipts: Vec<Receipt>,
}

impl UnlockLedger {
    /// A ledger that starts holding `holds` (read-through ids are skipped).
    pub fn new(table: &'static StoryTable, holds: &[CapabilityId]) -> Self {
        let mut start: Vec<(u8, CapabilityId)> = holds
            .iter()
            .filter_map(|id| {
                let row = table.capability(id.as_str())?;
                (row.storage == Storage::Ledger).then_some((row.hero, *id))
            })
            .collect();
        start.sort_unstable();
        start.dedup();
        Self {
            table,
            held: start.clone(),
            start,
            receipts: Vec::new(),
        }
    }

    /// Nothing held: the ledger mixes nothing into the state hash.
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// The held `(hero, id)` pairs, sorted.
    pub fn held(&self) -> &[(u8, CapabilityId)] {
        &self.held
    }

    pub fn receipts(&self) -> &[Receipt] {
        &self.receipts
    }

    /// Whether a stored capability is held (a read-through id never is).
    fn holds_stored(&self, id: &str) -> bool {
        self.table.capability(id).is_some_and(|row| {
            row.storage == Storage::Ledger
                && self
                    .held
                    .binary_search(&(row.hero, CapabilityId(static_id(self.table, id))))
                    .is_ok()
        })
    }

    /// Whether the capability is held. A stored capability asks the ledger; a
    /// read-through one asks `view` and nothing else (CP-Q45).
    pub fn holds(&self, id: CapabilityId, view: &ReadThrough) -> bool {
        match self.table.capability(id.as_str()) {
            Some(row) if row.storage == Storage::ReadThrough => view.read(id),
            Some(row) => self.held.binary_search(&(row.hero, id)).is_ok(),
            None => false,
        }
    }

    /// Grant a stored capability. `None` when it is already held, unknown or a
    /// read-through (the ledger never stores one): nothing is logged then.
    pub fn grant(&mut self, id: CapabilityId, tick: u64, source: &str) -> Option<Receipt> {
        let row = self.table.capability(id.as_str())?;
        if row.storage != Storage::Ledger {
            return None;
        }
        let key = (row.hero, id);
        let at = self.held.binary_search(&key).err()?;
        self.held.insert(at, key);
        let receipt = Receipt {
            hero: row.hero,
            capability: id,
            tick,
            source: source.to_owned(),
        };
        self.receipts.push(receipt.clone());
        Some(receipt)
    }

    /// A retry or a respawn keeps everything; a fixture restart restores the
    /// declared start state and forgets the receipts.
    pub fn restart(&mut self, kind: StoryRestart) {
        if kind == StoryRestart::Fixture {
            self.held = self.start.clone();
            self.receipts.clear();
        }
    }

    /// Fold into the state hash. **Nothing is mixed while empty.** Otherwise a
    /// tag, the sorted `(hero, id)` pairs and the receipts' ticks.
    pub fn hash_into(&self, mix: &mut impl FnMut(u64)) {
        if self.held.is_empty() {
            return;
        }
        mix(0x5354_4f52_595f_554c); // "STORY_UL"
        mix(self.held.len() as u64);
        for (hero, id) in &self.held {
            mix(u64::from(*hero));
            mix(id.hash());
        }
        mix(self.receipts.len() as u64);
        for receipt in &self.receipts {
            mix(u64::from(receipt.hero));
            mix(receipt.capability.hash());
            mix(receipt.tick);
        }
    }

    /// The sorted, versioned snapshot a future campaign save stores (CP-Q07).
    pub fn snapshot(&self) -> LedgerSnapshot {
        LedgerSnapshot {
            version: SNAPSHOT_VERSION,
            held: self
                .held
                .iter()
                .map(|(hero, id)| SnapshotHeld {
                    hero: *hero,
                    capability: id.as_str().to_owned(),
                })
                .collect(),
            receipts: self
                .receipts
                .iter()
                .map(|r| SnapshotReceipt {
                    hero: r.hero,
                    capability: r.capability.as_str().to_owned(),
                    tick: r.tick,
                    source: r.source.clone(),
                })
                .collect(),
        }
    }

    /// Replace the held set and the receipts with a snapshot's. The start
    /// state (the pins) is the scene's and is not part of a snapshot.
    pub fn restore(&mut self, snapshot: &LedgerSnapshot) -> Result<(), StoryError> {
        if snapshot.version != SNAPSHOT_VERSION {
            return Err(bad(
                "snapshot.version",
                format!("expected {SNAPSHOT_VERSION}"),
            ));
        }
        let mut held = Vec::new();
        for (i, entry) in snapshot.held.iter().enumerate() {
            let row = self.stored_row(&format!("snapshot.held[{i}]"), &entry.capability)?;
            if row.hero != entry.hero {
                return Err(bad(
                    format!("snapshot.held[{i}].hero"),
                    format!("{} belongs to hero {}", entry.capability, row.hero),
                ));
            }
            held.push((row.hero, CapabilityId(static_id(self.table, &row.id))));
        }
        held.sort_unstable();
        held.dedup();
        let mut receipts = Vec::new();
        for (i, entry) in snapshot.receipts.iter().enumerate() {
            let row = self.stored_row(&format!("snapshot.receipts[{i}]"), &entry.capability)?;
            if row.hero != entry.hero {
                return Err(bad(
                    format!("snapshot.receipts[{i}].hero"),
                    format!("{} belongs to hero {}", entry.capability, row.hero),
                ));
            }
            receipts.push(Receipt {
                hero: row.hero,
                capability: CapabilityId(static_id(self.table, &row.id)),
                tick: entry.tick,
                source: entry.source.clone(),
            });
        }
        self.held = held;
        self.receipts = receipts;
        Ok(())
    }

    fn stored_row(&self, field: &str, id: &str) -> Result<&'static CapabilityRow, StoryError> {
        match self.table.capability(id) {
            None => Err(bad(field, format!("unknown capability {id}"))),
            Some(row) if row.storage != Storage::Ledger => Err(bad(
                field,
                format!("{id} is a read-through capability: a snapshot never holds it"),
            )),
            Some(row) => Ok(row),
        }
    }
}

/// The `'static` text of a row's id (the table is `'static`, so this is a
/// borrow of it, not an allocation).
fn static_id(table: &'static StoryTable, id: &str) -> &'static str {
    table
        .capabilities
        .iter()
        .find(|row| row.id == id)
        .map(|row| row.id.as_str())
        .expect("the id was just looked up in this table")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotHeld {
    pub hero: u8,
    pub capability: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotReceipt {
    pub hero: u8,
    pub capability: String,
    pub tick: u64,
    pub source: String,
}

/// A serializable ledger state: sorted and versioned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerSnapshot {
    pub version: u32,
    pub held: Vec<SnapshotHeld>,
    pub receipts: Vec<SnapshotReceipt>,
}

/// What a scene declares about the story: the capabilities it starts with and
/// the triggers that are armed in it. [`Self::NONE`] is every scene without
/// the story flags and every scripted fixture that declares no pin.
#[derive(Debug, Clone, Default)]
pub struct StorySetup {
    table: Option<&'static StoryTable>,
    holds: Vec<CapabilityId>,
    triggers: Vec<&'static TriggerRow>,
    /// The heroes whose rows a firing trigger may grant: the union of the
    /// enabled story flags' heroes. `None` (a fixture arming a trigger by id)
    /// grants every row of the list.
    heroes: Option<Vec<u8>>,
}

impl StorySetup {
    pub const NONE: Self = Self {
        table: None,
        holds: Vec::new(),
        triggers: Vec::new(),
        heroes: None,
    };

    /// The setup the enabled story `flags` (`carl_prog`) and debug `starts`
    /// (`carl_dynamite`, `carl_gauntlet`) give `scene`: the pins of the scene,
    /// the capabilities the debug flags grant at start and the triggers armed
    /// in the scene. With no flag it is [`Self::NONE`].
    pub fn for_scene(
        table: &'static StoryTable,
        flags: &[&str],
        starts: &[&str],
        scene: SceneKind,
    ) -> Self {
        let mut heroes: Vec<u8> = Vec::new();
        for row in &table.story_flags {
            if flags.contains(&row.flag.as_str()) {
                heroes.extend(&row.heroes);
            }
        }
        heroes.sort_unstable();
        heroes.dedup();
        let mut setup = Self {
            table: Some(table),
            holds: Vec::new(),
            triggers: Vec::new(),
            heroes: Some(heroes),
        };
        for pin in &table.pins {
            if pin.scene == scene && flags.contains(&pin.flag.as_str()) {
                setup.hold_all(table, &pin.holds);
            }
        }
        for start in &table.start_flags {
            if starts.contains(&start.flag.as_str()) {
                setup.hold_all(table, &start.holds);
            }
        }
        for trigger in &table.triggers {
            if trigger.armed_in.contains(&scene) && flags.contains(&trigger.flag.as_str()) {
                setup.triggers.push(trigger);
            }
        }
        setup
    }

    fn hold_all(&mut self, table: &'static StoryTable, ids: &[String]) {
        for id in ids {
            let id = CapabilityId(static_id(table, id));
            if !self.holds.contains(&id) {
                self.holds.push(id);
            }
        }
    }

    /// A fixture declares a capability held from tick 0.
    #[must_use]
    pub fn with_hold(mut self, id: &str) -> Self {
        let table = self.table.unwrap_or_else(StoryTable::active);
        self.table = Some(table);
        self.hold_all(table, &[id.to_owned()]);
        self
    }

    /// A fixture arms a trigger by id (the placeholder Hoarder-defeat zone has no scene).
    /// Armed without a flag, it grants every row of its list (story progression). A
    /// setup built by `for_scene` from debug start flags only has an empty hero
    /// filter, so a trigger added to it may grant no row and is spent at start.
    #[must_use]
    pub fn with_trigger(mut self, id: &str) -> Self {
        let table = self.table.unwrap_or_else(StoryTable::active);
        self.table = Some(table);
        let row = table
            .triggers
            .iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("no story trigger {id}"));
        if !self.triggers.iter().any(|t| t.id == row.id) {
            self.triggers.push(row);
        }
        self
    }

    /// Nothing held and nothing armed.
    pub fn is_inert(&self) -> bool {
        self.holds.is_empty() && self.triggers.is_empty()
    }

    pub fn holds(&self) -> &[CapabilityId] {
        &self.holds
    }

    /// The heroes a firing trigger may grant (`None`: every row).
    pub fn grant_heroes(&self) -> Option<&[u8]> {
        self.heroes.as_deref()
    }

    pub fn trigger_ids(&self) -> Vec<&'static str> {
        self.triggers.iter().map(|t| t.id.as_str()).collect()
    }
}

#[derive(Debug, Clone)]
struct TriggerState {
    row: &'static TriggerRow,
    fired: bool,
}

/// The ledger with the armed triggers of one run.
#[derive(Debug, Clone)]
pub struct StoryRuntime {
    ledger: UnlockLedger,
    triggers: Vec<TriggerState>,
    /// The data event ids seen since the last [`Self::step`].
    pending: Vec<&'static str>,
    /// See [`StorySetup`]: the heroes whose rows a trigger may grant.
    heroes: Option<Vec<u8>>,
}

impl StoryRuntime {
    /// `None` for an inert setup: nothing is held and nothing is armed, so
    /// there is nothing to step or hash.
    pub fn new(setup: &StorySetup) -> Option<Self> {
        let table = setup.table?;
        if setup.is_inert() {
            return None;
        }
        let ledger = UnlockLedger::new(table, &setup.holds);
        let mut runtime = Self {
            ledger,
            triggers: setup
                .triggers
                .iter()
                .map(|row| TriggerState { row, fired: false })
                .collect(),
            pending: Vec::new(),
            heroes: setup.heroes.clone(),
        };
        runtime.rearm();
        Some(runtime)
    }

    pub fn ledger(&self) -> &UnlockLedger {
        &self.ledger
    }

    /// A trigger is spent when every capability it grants is already held
    /// (a fixture that starts with them): it fires without an event.
    fn rearm(&mut self) {
        let ledger = &self.ledger;
        let heroes = &self.heroes;
        for state in &mut self.triggers {
            state.fired = state
                .row
                .grants
                .iter()
                .filter(|id| grants_row(ledger.table, heroes.as_deref(), id))
                .all(|id| ledger.holds_stored(id));
        }
        self.pending.clear();
    }

    /// The data event rows (for the caller that maps combat events to ids).
    pub fn event_rows(&self) -> &'static [EventRow] {
        &self.ledger.table.events
    }

    /// Record that the data event `id` happened this tick. Only an event an
    /// armed trigger waits for is kept.
    pub fn observe(&mut self, id: &'static str) {
        let waited = self.triggers.iter().any(|state| {
            !state.fired
                && state.row.kind == TriggerKind::Event
                && state.row.event.as_deref() == Some(id)
        });
        if waited && !self.pending.contains(&id) {
            self.pending.push(id);
        }
    }

    /// One fixed tick. `pivots` are the heroes' pivots (Carl, Donut), `defeated`
    /// whether the party is defeated (a zone never fires then). Triggers fire in
    /// table order, each at most once, each granting its list in order; the
    /// returned receipts are in grant order. A grant of something already held
    /// returns no receipt.
    pub fn step(&mut self, tick: u64, pivots: [(f32, f32); 2], defeated: bool) -> Vec<Receipt> {
        let mut out = Vec::new();
        for state in &mut self.triggers {
            if state.fired {
                continue;
            }
            let hit = match (state.row.kind, &state.row.zone, &state.row.event) {
                (TriggerKind::Zone, Some(zone), _) => {
                    let (x, y) = pivots[usize::from(zone.hero.min(MAX_HERO))];
                    !defeated && zone.contains(x, y)
                }
                (TriggerKind::Event, _, Some(event)) => self.pending.contains(&event.as_str()),
                _ => false,
            };
            if !hit {
                continue;
            }
            state.fired = true;
            for id in &state.row.grants {
                if !grants_row(self.ledger.table, self.heroes.as_deref(), id) {
                    continue;
                }
                let id = CapabilityId(static_id(self.ledger.table, id));
                if let Some(receipt) = self.ledger.grant(id, tick, &state.row.id) {
                    out.push(receipt);
                }
            }
        }
        self.pending.clear();
        out
    }

    /// A retry or respawn keeps the grants and the spent triggers; a fixture
    /// restart restores the start state and re-arms the triggers. Events seen
    /// before the restart are forgotten.
    pub fn restart(&mut self, kind: StoryRestart) {
        self.ledger.restart(kind);
        if kind == StoryRestart::Fixture {
            self.rearm();
        } else {
            self.pending.clear();
        }
    }

    /// Whether a capability is held (see [`UnlockLedger::holds`]).
    pub fn holds(&self, id: CapabilityId, view: &ReadThrough) -> bool {
        self.ledger.holds(id, view)
    }

    pub fn hash_into(&self, mix: &mut impl FnMut(u64)) {
        self.ledger.hash_into(mix);
    }
}

/// Whether a firing trigger may grant the row `id` under the heroes filter.
fn grants_row(table: &StoryTable, heroes: Option<&[u8]>, id: &str) -> bool {
    match (heroes, table.capability(id)) {
        (None, _) => true,
        (Some(heroes), Some(row)) => heroes.contains(&row.hero),
        (Some(_), None) => false,
    }
}

/// [`StoryRuntime::holds`] for a run without a runtime: only a read-through
/// capability can be held then.
pub fn holds_without_story(id: CapabilityId, view: &ReadThrough) -> bool {
    StoryTable::active()
        .capability(id.as_str())
        .is_some_and(|row| row.storage == Storage::ReadThrough && view.read(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> &'static StoryTable {
        StoryTable::active()
    }

    #[test]
    fn the_data_file_is_pinned_and_valid() {
        // Fails when story_unlocks_v0.json changes without updating TABLE_HASH.
        assert_eq!(content_hash(TABLE_JSON), TABLE_HASH);
        let loaded = StoryTable::pinned().expect("pinned table loads");
        assert_eq!(loaded.schema_version, SCHEMA_VERSION);
        let drifted = TABLE_JSON.replace("\"GOBLIN BOOM\"", "\"GOBLIN BOOM!\"");
        assert_ne!(drifted, TABLE_JSON);
        match StoryTable::from_json_pinned(&drifted, TABLE_HASH) {
            Err(StoryError::HashMismatch { expected, .. }) => assert_eq!(expected, TABLE_HASH),
            other => panic!("a drifted file must fail the pin, got {other:?}"),
        }
    }

    #[test]
    fn the_shipped_rows_are_the_adopted_ones() {
        let t = table();
        let cap = |id: &str| t.capability(id).unwrap_or_else(|| panic!("row {id}"));
        assert_eq!(
            (
                cap("special_thrown_charge").hero,
                cap("special_thrown_charge").slot
            ),
            (0, Slot::Special)
        );
        assert_eq!(
            (cap("fist_reinforced").hero, cap("fist_reinforced").slot),
            (0, Slot::PrimaryMod)
        );
        assert_eq!(cap("primary_ranged").storage, Storage::ReadThrough);
        assert_eq!(cap("primary_ranged").hero, 1);
        // The Juicer trigger is armed only in the boss arena, the Hoarder-defeat zone stays a placeholder.
        assert_eq!(
            t.trigger("brute_boss_f01_defeated").unwrap().armed_in,
            [SceneKind::Boss]
        );
        assert!(
            t.trigger("pile_boss_f01_defeated")
                .unwrap()
                .armed_in
                .is_empty()
        );
        assert_eq!(t.pins.len(), 1);
        assert_eq!(t.pins[0].scene, SceneKind::Boss);
    }

    #[test]
    fn a_load_error_names_the_field() {
        let broken = |from: &str, to: &str| {
            let text = TABLE_JSON.replacen(from, to, 1);
            assert_ne!(text, TABLE_JSON, "{from}");
            StoryTable::from_json(&text).expect_err(to)
        };
        let field = |error: StoryError| match error {
            StoryError::Field { field, .. } => field,
            other => panic!("expected a field error, got {other:?}"),
        };
        assert_eq!(
            field(broken(
                "\"hero\": 0,\n      \"slot\": \"special\"",
                "\"hero\": 9,\n      \"slot\": \"special\""
            )),
            "capabilities[0].hero"
        );
        assert_eq!(
            field(broken(
                "\"text\": \"HEAVY FIST\"",
                "\"text\": \"FAR TOO LONG A NOTICE\""
            )),
            "notices[1].text"
        );
        assert_eq!(
            field(broken(
                "\"grants\": [\"fist_reinforced\"]",
                "\"grants\": [\"nothing_here\"]"
            )),
            "triggers[1].grants"
        );
        assert_eq!(
            field(broken(
                "\"grants\": [\"fist_reinforced\"]",
                "\"grants\": [\"primary_ranged\"]"
            )),
            "triggers[1].grants"
        );
        assert_eq!(
            field(broken(
                "\"event\": \"brute_boss_f01_defeated\",",
                "\"event\": \"never_seen\","
            )),
            "triggers[1].event"
        );
        assert_eq!(
            field(broken("\"min_x\": 600.0", "\"min_x\": 700.0")),
            "triggers[0].zone"
        );
        assert!(matches!(
            StoryTable::from_json("{\"schema_version\": 1, \"bogus\": 1}"),
            Err(StoryError::Parse(_))
        ));
    }

    /// A stub table with two heroes: one trigger grants both of them something.
    fn two_hero_table() -> &'static StoryTable {
        let json = r#"{
          "schema_version": 1,
          "provenance": "test stub",
          "story_flags": [
            {"flag": "stub_a", "heroes": [0], "cp_q": "T"},
            {"flag": "stub_b", "heroes": [1], "cp_q": "T"}
          ],
          "capabilities": [
            {"id": "alpha_special", "hero": 0, "slot": "special", "storage": "ledger", "display": "A", "cp_q": "T"},
            {"id": "beta_passive", "hero": 1, "slot": "passive", "storage": "ledger", "requires": ["alpha_special"], "display": "B", "cp_q": "T"},
            {"id": "gamma_primary", "hero": 1, "slot": "primary_mod", "storage": "read_through", "display": "C", "cp_q": "T"}
          ],
          "events": [{"id": "boss_down", "match": "enemy_defeated_boss", "cp_q": "T"}],
          "triggers": [
            {"id": "both_on_boss", "kind": "event", "event": "boss_down", "grants": ["alpha_special", "beta_passive"], "flag": "stub_a", "armed_in": ["boss"], "cp_q": "T"},
            {"id": "alpha_zone", "kind": "zone", "zone": {"hero": 0, "min_x": 10.0, "max_x": 20.0, "min_y": -50.0, "max_y": 50.0}, "grants": ["alpha_special"], "flag": "stub_a", "armed_in": ["demo"], "cp_q": "T"}
          ],
          "pins": [{"id": "alpha_held", "scene": "boss", "flag": "stub_a", "holds": ["alpha_special"], "cp_q": "T"}]
        }"#;
        StoryTable::from_json(json)
            .expect("the stub is valid")
            .leak()
    }

    #[test]
    fn a_two_capability_two_hero_trigger_grants_in_list_order_on_one_tick() {
        let t = two_hero_table();
        let setup = StorySetup::for_scene(t, &["stub_a", "stub_b"], &[], SceneKind::Boss);
        // The pin holds alpha, so only beta is new on the event.
        assert_eq!(setup.holds(), [CapabilityId("alpha_special")]);
        let mut runtime = StoryRuntime::new(&setup).expect("not inert");
        runtime.observe("boss_down");
        let receipts = runtime.step(77, [(0.0, 0.0); 2], false);
        assert_eq!(receipts.len(), 1);
        assert_eq!(
            (
                receipts[0].hero,
                receipts[0].capability.as_str(),
                receipts[0].tick
            ),
            (1, "beta_passive", 77)
        );
        // A fresh run without the pin grants both, alpha first, on one tick.
        let bare = StorySetup::for_scene(t, &["stub_a", "stub_b"], &[], SceneKind::Boss);
        let mut bare_table = bare.clone();
        bare_table.holds.clear();
        let mut runtime =
            StoryRuntime::new(&bare_table.with_trigger("both_on_boss")).expect("armed");
        runtime.observe("boss_down");
        let receipts = runtime.step(90, [(0.0, 0.0); 2], false);
        let granted: Vec<_> = receipts
            .iter()
            .map(|r| (r.hero, r.capability.as_str(), r.tick))
            .collect();
        assert_eq!(granted, [(0, "alpha_special", 90), (1, "beta_passive", 90)]);
        assert_eq!(receipts[0].source, "both_on_boss");
        // Once only: the same event again grants and logs nothing.
        runtime.observe("boss_down");
        assert!(runtime.step(91, [(0.0, 0.0); 2], false).is_empty());
    }

    /// A flag applies only its own heroes' rows of a shared trigger; a trigger
    /// armed without a flag (story progression, a fixture) grants every row.
    #[test]
    fn a_flag_grants_only_its_own_heroes_rows_of_a_shared_trigger() {
        let t = two_hero_table();
        let fire = |setup: &StorySetup| {
            let mut setup = setup.clone();
            setup.holds.clear();
            let mut runtime = StoryRuntime::new(&setup).expect("armed");
            runtime.observe("boss_down");
            runtime
                .step(5, [(0.0, 0.0); 2], false)
                .iter()
                .map(|r| (r.hero, r.capability.as_str()))
                .collect::<Vec<_>>()
        };
        let carl_only = StorySetup::for_scene(t, &["stub_a"], &[], SceneKind::Boss);
        assert_eq!(carl_only.grant_heroes(), Some(&[0u8][..]));
        assert_eq!(
            fire(&carl_only),
            [(0, "alpha_special")],
            "never the other hero's row"
        );
        let both = StorySetup::for_scene(t, &["stub_a", "stub_b"], &[], SceneKind::Boss);
        assert_eq!(both.grant_heroes(), Some(&[0u8, 1][..]));
        assert_eq!(fire(&both), [(0, "alpha_special"), (1, "beta_passive")]);
        // No flag: a fixture arms the trigger by id and every row is granted.
        let fixture = StorySetup {
            table: Some(t),
            holds: Vec::new(),
            triggers: vec![t.trigger("both_on_boss").unwrap()],
            heroes: None,
        };
        assert_eq!(fire(&fixture), [(0, "alpha_special"), (1, "beta_passive")]);
        // A trigger whose only remaining rows belong to other heroes is spent
        // once its own heroes' rows are held.
        let mut runtime = StoryRuntime::new(&carl_only).unwrap();
        runtime.observe("boss_down");
        let first = runtime.step(9, [(0.0, 0.0); 2], false);
        assert!(
            first.is_empty(),
            "alpha is pinned, beta is not this flag's: nothing to grant"
        );
        assert!(runtime.ledger().receipts().is_empty());
    }

    #[test]
    fn a_grant_is_idempotent_and_logs_once() {
        let t = table();
        let mut ledger = UnlockLedger::new(t, &[]);
        assert!(ledger.is_empty());
        let first = ledger
            .grant(SPECIAL_THROWN_CHARGE, 12, "test")
            .expect("first grant");
        assert_eq!((first.hero, first.tick), (0, 12));
        assert_eq!(ledger.grant(SPECIAL_THROWN_CHARGE, 40, "test"), None);
        assert_eq!(ledger.receipts().len(), 1);
        assert_eq!(ledger.receipts()[0].tick, 12, "the first receipt stands");
        assert!(ledger.holds(SPECIAL_THROWN_CHARGE, &ReadThrough::default()));
        assert!(!ledger.holds(FIST_REINFORCED, &ReadThrough::default()));
    }

    #[test]
    fn restart_kinds_keep_the_grants_and_a_fixture_restart_restores_the_pin() {
        let t = table();
        let mut ledger = UnlockLedger::new(t, &[FIST_REINFORCED]);
        ledger.grant(SPECIAL_THROWN_CHARGE, 30, "test").unwrap();
        for kind in [StoryRestart::DefeatRetry, StoryRestart::CheckpointRestart] {
            ledger.restart(kind);
            assert_eq!(ledger.held().len(), 2, "{kind:?} keeps the grants");
            assert_eq!(ledger.receipts().len(), 1, "{kind:?} keeps the receipts");
        }
        ledger.restart(StoryRestart::Fixture);
        assert_eq!(ledger.held(), [(0, FIST_REINFORCED)], "the pin comes back");
        assert!(ledger.receipts().is_empty());
        // The same restart through the donut restart kinds.
        use crate::donut_mana::{RestartKind, StartState};
        assert_eq!(
            StoryRestart::from(RestartKind::DefeatRetry),
            StoryRestart::DefeatRetry
        );
        assert_eq!(
            StoryRestart::from(RestartKind::CheckpointRestart),
            StoryRestart::CheckpointRestart
        );
        assert_eq!(
            StoryRestart::from(RestartKind::Fixture(StartState::LOCKED)),
            StoryRestart::Fixture
        );
    }

    fn mixed(ledger: &UnlockLedger) -> Vec<u64> {
        let mut out = Vec::new();
        ledger.hash_into(&mut |v| out.push(v));
        out
    }

    #[test]
    fn an_empty_ledger_mixes_nothing_and_a_held_one_mixes_pairs_and_receipt_ticks() {
        let t = table();
        let mut ledger = UnlockLedger::new(t, &[]);
        assert!(mixed(&ledger).is_empty());
        ledger.grant(SPECIAL_THROWN_CHARGE, 5, "test").unwrap();
        let a = mixed(&ledger);
        assert!(!a.is_empty());
        let mut later = UnlockLedger::new(t, &[]);
        later.grant(SPECIAL_THROWN_CHARGE, 6, "test").unwrap();
        assert_ne!(a, mixed(&later), "the receipt tick is part of the hash");
        // A fixture restart of an empty pin returns to nothing mixed.
        ledger.restart(StoryRestart::Fixture);
        assert!(mixed(&ledger).is_empty());
    }

    #[test]
    fn a_read_through_capability_is_never_stored_hashed_receipted_or_granted() {
        let t = table();
        let mut ledger = UnlockLedger::new(t, &[PRIMARY_RANGED]);
        assert!(ledger.is_empty(), "a pin never stores a read-through");
        assert_eq!(ledger.grant(PRIMARY_RANGED, 3, "test"), None);
        assert!(ledger.receipts().is_empty());
        assert!(mixed(&ledger).is_empty());
        assert!(!ledger.holds(PRIMARY_RANGED, &ReadThrough::default()));
        assert!(ledger.holds(
            PRIMARY_RANGED,
            &ReadThrough {
                primary_ranged: true
            }
        ));
        // The view alone decides, and nothing changed in the ledger.
        assert!(ledger.is_empty() && mixed(&ledger).is_empty());
        assert!(!holds_without_story(
            PRIMARY_RANGED,
            &ReadThrough::default()
        ));
        assert!(holds_without_story(
            PRIMARY_RANGED,
            &ReadThrough {
                primary_ranged: true
            }
        ));
        assert!(!holds_without_story(
            SPECIAL_THROWN_CHARGE,
            &ReadThrough {
                primary_ranged: true
            }
        ));
    }

    #[test]
    fn the_snapshot_is_sorted_versioned_and_round_trips() {
        let t = table();
        let mut ledger = UnlockLedger::new(t, &[]);
        ledger.grant(SPECIAL_THROWN_CHARGE, 8, "find").unwrap();
        ledger.grant(FIST_REINFORCED, 20, "boss").unwrap();
        let snapshot = ledger.snapshot();
        assert_eq!(snapshot.version, SNAPSHOT_VERSION);
        let ids: Vec<_> = snapshot
            .held
            .iter()
            .map(|h| h.capability.as_str())
            .collect();
        assert_eq!(
            ids,
            ["fist_reinforced", "special_thrown_charge"],
            "sorted by (hero, id)"
        );
        let text = serde_json::to_string(&snapshot).unwrap();
        let back: LedgerSnapshot = serde_json::from_str(&text).unwrap();
        assert_eq!(back, snapshot);
        let mut fresh = UnlockLedger::new(t, &[]);
        fresh.restore(&back).expect("restores");
        assert_eq!(fresh.held(), ledger.held());
        assert_eq!(fresh.receipts(), ledger.receipts());
        assert_eq!(mixed(&fresh), mixed(&ledger));
        // A snapshot never holds a read-through, an unknown id or another hero's row.
        let mut bad_snapshot = snapshot.clone();
        bad_snapshot.held[0].capability = "primary_ranged".to_owned();
        assert!(fresh.restore(&bad_snapshot).is_err());
        bad_snapshot.held[0].capability = "nope".to_owned();
        assert!(fresh.restore(&bad_snapshot).is_err());
        let mut wrong_hero = snapshot.clone();
        wrong_hero.held[0].hero = 1;
        assert!(fresh.restore(&wrong_hero).is_err());
        let mut old = snapshot;
        old.version = 0;
        assert!(fresh.restore(&old).is_err());
    }

    #[test]
    fn a_zone_trigger_fires_once_on_the_first_pivot_inside_and_never_while_defeated() {
        let t = two_hero_table();
        let stub = StorySetup {
            table: Some(t),
            holds: Vec::new(),
            triggers: vec![t.trigger("alpha_zone").unwrap()],
            heroes: None,
        };
        let mut runtime = StoryRuntime::new(&stub).unwrap();
        let outside = [(5.0, 0.0), (15.0, 0.0)];
        let inside = [(15.0, 0.0), (5.0, 0.0)];
        assert!(
            runtime.step(1, outside, false).is_empty(),
            "Donut inside does not fire a Carl zone"
        );
        assert!(
            runtime.step(2, inside, true).is_empty(),
            "never while defeated"
        );
        let receipts = runtime.step(3, inside, false);
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].tick, 3);
        assert!(runtime.step(4, inside, false).is_empty(), "once only");
        // A retry keeps it spent; a fixture restart re-arms it.
        runtime.restart(StoryRestart::DefeatRetry);
        assert!(runtime.step(5, inside, false).is_empty());
        runtime.restart(StoryRestart::Fixture);
        assert!(runtime.ledger().is_empty());
        assert_eq!(runtime.step(6, inside, false).len(), 1);
    }

    #[test]
    fn a_trigger_whose_grants_are_all_held_at_start_is_spent_without_an_event() {
        let setup = StorySetup::NONE
            .with_hold("special_thrown_charge")
            .with_trigger("pile_boss_f01_defeated");
        let mut runtime = StoryRuntime::new(&setup).unwrap();
        let inside = [(610.0, 0.0), (0.0, 0.0)];
        assert!(runtime.step(1, inside, false).is_empty());
        assert!(runtime.ledger().receipts().is_empty());
    }

    #[test]
    fn an_inert_setup_has_no_runtime_and_the_flags_map_to_scenes() {
        let t = table();
        assert!(StoryRuntime::new(&StorySetup::NONE).is_none());
        for scene in [SceneKind::Corridor, SceneKind::Demo, SceneKind::CfArena] {
            let setup = StorySetup::for_scene(t, &["carl_prog"], &[], scene);
            assert!(
                setup.is_inert(),
                "{scene:?} holds and arms nothing under the story flag"
            );
            assert!(StoryRuntime::new(&setup).is_none());
        }
        let boss = StorySetup::for_scene(t, &["carl_prog"], &[], SceneKind::Boss);
        assert_eq!(boss.holds(), [SPECIAL_THROWN_CHARGE]);
        assert_eq!(boss.trigger_ids(), ["brute_boss_f01_defeated"]);
        let none = StorySetup::for_scene(t, &[], &[], SceneKind::Boss);
        assert!(none.is_inert());
        let both =
            StorySetup::for_scene(t, &[], &["carl_dynamite", "carl_gauntlet"], SceneKind::Demo);
        assert_eq!(both.holds(), [SPECIAL_THROWN_CHARGE, FIST_REINFORCED]);
        assert!(both.trigger_ids().is_empty());
    }

    #[test]
    fn no_capability_id_or_data_key_names_a_franchise_item_or_character() {
        let banned = [
            "dynamite", "gauntlet", "carl", "hoarder", "juicer", "donut", "biscuit", "puddle",
        ];
        let t = table();
        for row in &t.capabilities {
            for word in banned {
                assert!(
                    !row.id.contains(word),
                    "capability id {} says {word}",
                    row.id
                );
            }
        }
        // The data keys (not the fan-layer values) are neutral too.
        let value: serde_json::Value = serde_json::from_str(TABLE_JSON).unwrap();
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
        for key in all {
            for word in banned {
                assert!(!key.contains(word), "data key {key} says {word}");
            }
        }
    }
}
