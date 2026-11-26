//! F01-RUN: the in-session floor run of `--scene f01_late --f01-run`.
//!
//! Bevy-free, so every rule here is unit-tested headlessly; `floor01_scene`
//! only feeds it the finished state of a room and draws what it returns
//! through the developer lab's overlay (`LabOverlay::floor_run`). The leak guard
//! of LAB-Q01 keeps this module free of the lab's profile and modules.
//!
//! **One room per launch** (K17, the combat lab's pattern in
//! its flow module): the scene binary builds its kit, camera and art
//! once, so a room change starts the binary again with the next room's
//! arguments ([`relaunch_argv`]). The run:
//!
//! - starts at the **first listed room** of `assets/levels/f01_late/manifest.json`
//!   ([`first_room`]) unless `--f01-room` names another;
//! - when the room's exit is reached, shows a facts-only room-clear panel
//!   ([`room_clear_panel`]: ticks, deaths, restarts, grants) and, on confirm,
//!   relaunches with the **next listed room** ([`next_room`]), carrying the
//!   story ledger ([`crate::combat_scene::CombatState::story_snapshot`], applied
//!   by `carry_story`) and the run's tally on the argument list;
//! - after the last listed room shows a facts-only floor-end panel
//!   ([`floor_end_panel`]).
//!
//! The room order is the manifest's row order and nothing else: a room that a
//! later step lists (the corral, the Ring rooms, the Dozer) joins the run with
//! no change here.

use crate::combat_scene::CombatState;
use crate::f01_late::{Loadout, Manifest, RoomRow, RoomScene};
use crate::story_unlock::LedgerSnapshot;

/// The flag that makes `--scene f01_late` a floor run.
pub const FLAG_RUN: &str = "--f01-run";
/// The ledger snapshot of the previous room (JSON, written by a relaunch).
pub const FLAG_CARRY: &str = "--f01-carry";
/// The run's tally so far (`rooms:ticks:deaths:restarts`, written by a relaunch).
pub const FLAG_TALLY: &str = "--f01-run-tally";

/// AUDIENCE-RUN: the viewer counts of the previous room
/// (`current_carl:peak_carl:current_donut:peak_donut`, written by a relaunch).
pub const FLAG_AUDIENCE: &str = "--f01-audience";
/// AUDIENCE-RUN (AR-AD1): the broadcast goes live in every run room listed
/// **after** this one (the Infernal Corral).
pub const AUDIENCE_UNLOCK_AFTER: &str = "corral";
/// The room-clear panel line announcing the broadcast (AR-AD2). A flat fact:
/// no viewer number, no quote.
pub const BROADCAST_LINE: &str = "broadcast: live from the next room (viewers per hero)";

/// Fixed ticks per second (the tally's clock).
const TICKS_PER_SECOND: u64 = 60;

/// What the rooms cleared so far in this run add up to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunTally {
    pub rooms: u32,
    pub ticks: u64,
    pub deaths: u32,
    pub restarts: u32,
}

impl RunTally {
    /// The `--f01-run-tally` value.
    pub fn to_arg(self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.rooms, self.ticks, self.deaths, self.restarts
        )
    }

    /// Parse a `--f01-run-tally` value.
    pub fn parse(text: &str) -> Result<Self, String> {
        let bad = || format!("bad {FLAG_TALLY} {text:?} (expected rooms:ticks:deaths:restarts)");
        let parts: Vec<&str> = text.split(':').collect();
        let [rooms, ticks, deaths, restarts] = parts.as_slice() else {
            return Err(bad());
        };
        Ok(Self {
            rooms: rooms.parse().map_err(|_| bad())?,
            ticks: ticks.parse().map_err(|_| bad())?,
            deaths: deaths.parse().map_err(|_| bad())?,
            restarts: restarts.parse().map_err(|_| bad())?,
        })
    }

    /// The tally with `room` cleared.
    #[must_use]
    pub fn with(self, room: &RoomFacts) -> Self {
        Self {
            rooms: self.rooms.saturating_add(1),
            ticks: self.ticks.saturating_add(room.ticks),
            deaths: self.deaths.saturating_add(room.deaths),
            restarts: self.restarts.saturating_add(room.restarts),
        }
    }
}

/// The viewer counts a run room hands to the next one (AR-AD3): current and
/// peak per hero, the decay remainder dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudienceCarry {
    /// Carl: `(current, peak)`.
    pub carl: (u64, u64),
    /// Donut: `(current, peak)`.
    pub donut: (u64, u64),
}

impl AudienceCarry {
    /// The `--f01-audience` value (`C:P:D:Q`).
    pub fn to_arg(self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.carl.0, self.carl.1, self.donut.0, self.donut.1
        )
    }

    /// Parse a `--f01-audience` value. Four decimal numbers, each hero's
    /// `peak >= current >= baseline`.
    pub fn parse(text: &str) -> Result<Self, String> {
        let baseline = crate::audience::TUNING.baseline;
        let bad = || {
            format!(
                "bad {FLAG_AUDIENCE} {text:?} (expected carl:carl_peak:donut:donut_peak, each peak >= current >= {baseline})"
            )
        };
        let parts: Vec<&str> = text.split(':').collect();
        let [c, p, d, q] = parts.as_slice() else {
            return Err(bad());
        };
        let num = |v: &str| {
            (!v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
                .then(|| v.parse::<u64>().ok())
                .flatten()
                .ok_or_else(bad)
        };
        let (c, p, d, q) = (num(c)?, num(p)?, num(d)?, num(q)?);
        if c < baseline || p < c || d < baseline || q < d {
            return Err(bad());
        }
        Ok(Self {
            carl: (c, p),
            donut: (d, q),
        })
    }

    /// The `(current, peak)` pairs, Carl first, as the audience resumes them.
    pub const fn counts(self) -> [(u64, u64); 2] {
        [self.carl, self.donut]
    }

    /// The carry of the audience's exit counts.
    pub const fn from_counts(counts: [(u64, u64); 2]) -> Self {
        Self {
            carl: counts[0],
            donut: counts[1],
        }
    }
}

/// AUDIENCE-RUN (AR-AD1): whether the broadcast is live in `room`: the
/// [`AUDIENCE_UNLOCK_AFTER`] row exists and `room`'s row comes after it.
/// Positional, so it does not depend on how the room was reached; fails closed
/// when the unlock row is not listed.
pub fn audience_live_in(manifest: &Manifest, room: &str) -> bool {
    let index_of = |id: &str| manifest.rooms.iter().position(|r| r.id == id);
    match (index_of(AUDIENCE_UNLOCK_AFTER), index_of(room)) {
        (Some(unlock), Some(at)) => at > unlock,
        _ => false,
    }
}

/// The `--f01-carry` value of a snapshot.
pub fn encode_carry(snapshot: &LedgerSnapshot) -> String {
    serde_json::to_string(snapshot).expect("a ledger snapshot serializes")
}

/// Parse a `--f01-carry` value (the version and the ids are checked when the
/// room applies it, `carry_story`).
pub fn decode_carry(text: &str) -> Result<LedgerSnapshot, String> {
    serde_json::from_str(text).map_err(|e| format!("bad {FLAG_CARRY}: {e}"))
}

/// The room a run starts in: the first listed room.
pub fn first_room(manifest: &Manifest) -> &RoomRow {
    &manifest.rooms[0]
}

/// Where a room's exit leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NextStep {
    /// Relaunch into this listed room.
    Room(RoomRow),
    /// `current` was the last listed room: the floor-end panel.
    FloorEnd,
}

/// The listed room after `current`, in manifest order.
pub fn next_room(manifest: &Manifest, current: &str) -> Result<NextStep, String> {
    let at = manifest
        .rooms
        .iter()
        .position(|r| r.id == current)
        .ok_or_else(|| format!("the floor run is in an unlisted room {current:?}"))?;
    Ok(manifest
        .rooms
        .get(at + 1)
        .map_or(NextStep::FloorEnd, |row| NextStep::Room(row.clone())))
}

/// The 1-based position of `room` among the listed rooms, and their count.
pub fn room_index(manifest: &Manifest, room: &str) -> Option<(usize, usize)> {
    manifest
        .rooms
        .iter()
        .position(|r| r.id == room)
        .map(|i| (i + 1, manifest.rooms.len()))
}

/// The argument list of the next launch: `argv` (program name first) with the
/// run's own per-room flags (`--f01-room`, `--f01-carry`, `--f01-run-tally`,
/// `--f01-audience`) replaced. Everything else (`--f01-run`, the loadout, the scale, the deck,
/// `--companion`, the recording flags) is kept.
pub fn relaunch_argv(
    argv: &[String],
    next: &str,
    carry: Option<&LedgerSnapshot>,
    tally: RunTally,
    audience: Option<AudienceCarry>,
) -> Vec<String> {
    let mut out = Vec::with_capacity(argv.len() + 6);
    let mut iter = argv.iter();
    if let Some(program) = iter.next() {
        out.push(program.clone());
    }
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--f01-room" | FLAG_CARRY | FLAG_TALLY | FLAG_AUDIENCE => {
                iter.next();
            }
            _ => out.push(arg.clone()),
        }
    }
    out.extend(["--f01-room".to_owned(), next.to_owned()]);
    if let Some(snapshot) = carry {
        out.extend([FLAG_CARRY.to_owned(), encode_carry(snapshot)]);
    }
    out.extend([FLAG_TALLY.to_owned(), tally.to_arg()]);
    if let Some(carry) = audience {
        out.extend([FLAG_AUDIENCE.to_owned(), carry.to_arg()]);
    }
    out
}

/// The ledger a relaunch carries: the room's snapshot under the story loadout,
/// nothing under the TEST LOADOUT (every room holds everything; adopted
/// F01RUN-AD2). Only a **story-less** scene ([`RoomScene::story_less`]: the corral, the
/// one story room with no story runtime (its pin holds nothing and arms nothing, KC11),
/// and the Act I rooms, ACT1-AD1) carries the empty ledger when it has no snapshot, so
/// the next room starts from its own pin and the panel states the story chain (CORRAL-F,
/// adopted CF-AD3; ACT1-A). Any other room without a snapshot carries nothing, so a
/// future story room never claims a carried ledger by accident.
pub fn carry_for(
    loadout: Loadout,
    scene: RoomScene,
    combat: &CombatState,
) -> Option<LedgerSnapshot> {
    match loadout {
        Loadout::Story => combat.story_snapshot().or_else(|| {
            scene.story_less().then(|| LedgerSnapshot {
                version: crate::story_unlock::SNAPSHOT_VERSION,
                held: Vec::new(),
                receipts: Vec::new(),
            })
        }),
        Loadout::All => None,
    }
}

/// Whether the HUD shows the TEST LOADOUT tag (PT-SCENE U5): exactly under
/// `--f01-loadout all` (every capability held, the story triggers off). A story
/// room started from its pin is the canonical story state at that point (the
/// KR12 pins, its triggers armed), so it is not tagged; whether a room started
/// from a pin or from a carried ledger is in the descriptor (`f01_carry`).
pub fn test_loadout_tag(loadout: Option<Loadout>) -> bool {
    loadout == Some(Loadout::All)
}

/// What the run does once the room-clear or floor-end panel is confirmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Relaunch the binary with these arguments into this listed room.
    Relaunch { room: String, argv: Vec<String> },
    /// The floor end: quit.
    Quit,
}

/// Everything the binary knows when a floor run room's exit is reached.
pub struct ExitInput<'a> {
    pub manifest: &'a Manifest,
    /// The room being left.
    pub room: &'a str,
    pub loadout: Loadout,
    pub combat: &'a CombatState,
    /// The ledger the run carried into this room (`--f01-carry`).
    pub carried: Option<&'a LedgerSnapshot>,
    /// The run's tally before this room (`--f01-run-tally`; none in the first room).
    pub tally: RunTally,
    /// This process's arguments (program name first).
    pub argv: &'a [String],
    /// The REC badge text.
    pub recording: &'a str,
    /// AUDIENCE-RUN: the viewer counts at the exit tick, `Some` only while the
    /// audience is live in this room.
    pub audience: Option<AudienceCarry>,
    /// AUDIENCE-RUN: the broadcast is enabled for this run (`--audience`
    /// resolved); the corral's panel announces it only then.
    pub audience_enabled: bool,
}

/// F01-RUN: the decision of an exit: the room's facts, the panel to show and
/// what its confirmation does (the next listed room with the carried ledger and
/// the tally, or the floor end). The binary's run step only calls this.
pub fn on_exit(input: &ExitInput<'_>) -> Result<(RoomFacts, PanelText, Decision), String> {
    let row = input.manifest.room(input.room)?;
    let facts = RoomFacts::of(row, input.combat, input.carried);
    let tally = input.tally.with(&facts);
    match next_room(input.manifest, input.room)? {
        NextStep::Room(next) => {
            let carry = carry_for(input.loadout, row.scene, input.combat);
            let argv = relaunch_argv(input.argv, &next.id, carry.as_ref(), tally, input.audience);
            let panel = room_clear_panel(
                input.manifest,
                &facts,
                tally,
                &next,
                carry.is_some(),
                input.recording,
                input.audience_enabled,
            );
            Ok((
                facts,
                panel,
                Decision::Relaunch {
                    room: next.id,
                    argv,
                },
            ))
        }
        NextStep::FloorEnd => {
            let panel = floor_end_panel(
                input.manifest,
                &facts,
                tally,
                &held_of(input.combat),
                input.recording,
            );
            Ok((facts, panel, Decision::Quit))
        }
    }
}

/// The facts of one cleared room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomFacts {
    pub id: String,
    pub title: String,
    /// Ticks over every attempt ([`CombatState::elapsed`]).
    pub ticks: u64,
    pub deaths: u32,
    pub restarts: u32,
    /// The capabilities granted **in this room**: `(hero, capability)`, the
    /// receipts of the room's ledger that the carried ledger did not have.
    pub grants: Vec<(u8, String)>,
}

/// The ledger receipts of `now` that `before` did not hold, in ledger order.
fn new_grants(now: Option<&LedgerSnapshot>, before: Option<&LedgerSnapshot>) -> Vec<(u8, String)> {
    let Some(now) = now else {
        return Vec::new();
    };
    now.receipts
        .iter()
        .filter(|r| {
            !before.is_some_and(|b| {
                b.receipts
                    .iter()
                    .any(|old| old.hero == r.hero && old.capability == r.capability)
            })
        })
        .map(|r| (r.hero, r.capability.clone()))
        .collect()
}

/// The capability Donut's biscuit grants: a **read-through** (it is read from her mana, never
/// stored in the ledger, so it has no receipt, ACT1-AD12).
pub const BISCUIT_CAPABILITY: &str = "primary_ranged";

/// The grants of a finished room: the ledger receipts the carried ledger did not have, plus
/// the biscuit (`(1, primary_ranged)`) when the room's combat log holds the
/// `ManaEvent::UnlockGranted` of its guild zone (ACT1-AD12).
fn room_grants(combat: &CombatState, carried: Option<&LedgerSnapshot>) -> Vec<(u8, String)> {
    let mut grants = new_grants(combat.story_snapshot().as_ref(), carried);
    let biscuit = combat.events.iter().any(|e| {
        matches!(
            e,
            crate::combat::CombatEvent::Mana(crate::donut_mana::ManaEvent::UnlockGranted { .. })
        )
    });
    if biscuit {
        grants.push((1, BISCUIT_CAPABILITY.to_owned()));
    }
    grants
}

impl RoomFacts {
    /// Read the facts of `row`'s room from its finished combat state and the
    /// ledger the run carried into it.
    pub fn of(row: &RoomRow, combat: &CombatState, carried: Option<&LedgerSnapshot>) -> Self {
        Self {
            id: row.id.clone(),
            title: row.title.clone(),
            ticks: combat.elapsed,
            deaths: combat.deaths,
            restarts: combat.restarts,
            grants: room_grants(combat, carried),
        }
    }
}

/// The plain name of a hero index of the ledger.
pub fn hero_name(hero: u8) -> &'static str {
    match hero {
        0 => "Carl",
        _ => "Donut",
    }
}

fn clock(ticks: u64) -> String {
    let seconds = ticks / TICKS_PER_SECOND;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn grant_list(grants: &[(u8, String)]) -> String {
    if grants.is_empty() {
        return "nothing".to_owned();
    }
    grants
        .iter()
        .map(|(hero, cap)| format!("{cap} ({})", hero_name(*hero)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A panel's text: the heading, the fact lines and the confirm hint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelText {
    pub heading: String,
    pub lines: Vec<String>,
    pub hint: String,
}

/// The room-clear panel (facts only, the lab's `EndPanel` copy rules).
pub fn room_clear_panel(
    manifest: &Manifest,
    facts: &RoomFacts,
    tally_after: RunTally,
    next: &RoomRow,
    carried_on: bool,
    recording: &str,
    announce_broadcast: bool,
) -> PanelText {
    let index =
        room_index(manifest, &facts.id).map_or_else(String::new, |(i, n)| format!("ROOM {i}/{n} "));
    // AR-AD2: the one reveal, only on the unlock room's panel and only when
    // the broadcast is on; no viewer number (AR-AD20).
    let mut lines = vec![
        format!(
            "{} ticks ({}), deaths {}, restarts {}",
            facts.ticks,
            clock(facts.ticks),
            facts.deaths,
            facts.restarts
        ),
        format!("granted here: {}", grant_list(&facts.grants)),
        format!(
            "run so far: {} room{}, {} ticks ({}), deaths {}, restarts {}",
            tally_after.rooms,
            if tally_after.rooms == 1 { "" } else { "s" },
            tally_after.ticks,
            clock(tally_after.ticks),
            tally_after.deaths,
            tally_after.restarts
        ),
        format!(
            "next: {} ({})",
            next.title,
            if carried_on {
                "story ledger carried"
            } else {
                "TEST LOADOUT, nothing carried"
            }
        ),
        format!("recording: {recording}"),
        "Facts only: no score, no rating.".to_owned(),
    ];
    if announce_broadcast && facts.id == AUDIENCE_UNLOCK_AFTER {
        let at = lines
            .iter()
            .position(|l| l.starts_with("next: "))
            .expect("the panel has a next line");
        lines.insert(at, BROADCAST_LINE.to_owned());
    }
    PanelText {
        heading: format!("{index}CLEARED: {}", facts.title.to_uppercase()),
        lines,
        hint: format!("Enter, A or Start: next room ({})", next.title),
    }
}

/// The floor-end panel after the last listed room.
pub fn floor_end_panel(
    manifest: &Manifest,
    facts: &RoomFacts,
    tally_after: RunTally,
    held: &[(u8, String)],
    recording: &str,
) -> PanelText {
    let mut lines = vec![
        format!(
            "{}: {} ticks ({}), deaths {}, restarts {}",
            facts.title,
            facts.ticks,
            clock(facts.ticks),
            facts.deaths,
            facts.restarts
        ),
        format!("granted here: {}", grant_list(&facts.grants)),
        format!(
            "run total: {} of {} listed rooms, {} ticks ({}), deaths {}, restarts {}",
            tally_after.rooms,
            manifest.rooms.len(),
            tally_after.ticks,
            clock(tally_after.ticks),
            tally_after.deaths,
            tally_after.restarts
        ),
    ];
    for hero in [0u8, 1] {
        let caps: Vec<&str> = held
            .iter()
            .filter(|(h, _)| *h == hero)
            .map(|(_, c)| c.as_str())
            .collect();
        lines.push(format!(
            "{} holds: {}",
            hero_name(hero),
            if caps.is_empty() {
                "nothing stored".to_owned()
            } else {
                caps.join(", ")
            }
        ));
    }
    lines.push(format!("recording: {recording}"));
    lines.push("Facts only: no score, no rating.".to_owned());
    PanelText {
        heading: "FLOOR 1 SLICE FINISHED".to_owned(),
        lines,
        hint: "Enter, A or Start: quit".to_owned(),
    }
}

/// The stored capabilities a finished room holds, `(hero, capability)`.
pub fn held_of(combat: &CombatState) -> Vec<(u8, String)> {
    combat.story_ledger().map_or_else(Vec::new, |ledger| {
        ledger
            .held()
            .iter()
            .map(|(hero, id)| (*hero, id.as_str().to_owned()))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::f01_late_fixtures::{hoarder_defeat, juicer_relaunch};
    use crate::story_unlock::{FIST_REINFORCED, SPECIAL_THROWN_CHARGE, StoryTable};
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn manifest() -> Manifest {
        Manifest::load(&root()).expect("the committed manifest loads")
    }

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|a| (*a).to_owned()).collect()
    }

    #[test]
    fn the_run_walks_the_listed_rooms_in_manifest_order_then_ends() {
        let manifest = manifest();
        let mut seen = vec![first_room(&manifest).id.clone()];
        while let NextStep::Room(row) = next_room(&manifest, seen.last().unwrap()).unwrap() {
            seen.push(row.id);
        }
        assert_eq!(seen, manifest.listed());
        // ACT1-F (CORRAL-F): the run starts in the first Act I room (the chase, ACT1-B), walks
        // the shore, the guild and the corral; REST-WIRE: it walks on from the corral to the
        // refuge, and the refuge to the Hoarder.
        assert_eq!(first_room(&manifest).id, "act1_chase");
        assert_eq!(
            next_room(&manifest, "act1_chase").unwrap(),
            NextStep::Room(manifest.room("act1_shore").unwrap().clone())
        );
        assert_eq!(
            next_room(&manifest, "act1_shore").unwrap(),
            NextStep::Room(manifest.room("act1_guild").unwrap().clone())
        );
        assert_eq!(
            next_room(&manifest, "act1_guild").unwrap(),
            NextStep::Room(manifest.room("corral").unwrap().clone())
        );
        assert_eq!(
            next_room(&manifest, "corral").unwrap(),
            NextStep::Room(manifest.room("rest_refuge").unwrap().clone())
        );
        assert_eq!(
            next_room(&manifest, "rest_refuge").unwrap(),
            NextStep::Room(manifest.room("hoarder").unwrap().clone())
        );
        // The listed rooms in story order, ACT1-A, ACT1-B and REST-WIRE (REST-A, REST-B,
        // REST-C) and TROG-A: sixteen rooms, the chase, the shore and the guild first, the camp after the
        // Ring, the troglodyte run after the camp, the aftermath after the Juicer, mastery ends the run.
        assert_eq!(
            seen,
            [
                "act1_chase",
                "act1_shore",
                "act1_guild",
                "corral",
                "rest_refuge",
                "hoarder",
                "boiler",
                "dozer",
                "hazards",
                "ring_court",
                "ring_split",
                "rest_camp",
                "trog_run",
                "juicer",
                "rest_aftermath",
                "mastery"
            ]
        );
        assert_eq!(
            next_room(&manifest, "ring_split").unwrap(),
            NextStep::Room(manifest.room("rest_camp").unwrap().clone())
        );
        assert_eq!(
            next_room(&manifest, "rest_camp").unwrap(),
            NextStep::Room(manifest.room("trog_run").unwrap().clone())
        );
        assert_eq!(
            next_room(&manifest, "trog_run").unwrap(),
            NextStep::Room(manifest.room("juicer").unwrap().clone())
        );
        assert_eq!(
            next_room(&manifest, "juicer").unwrap(),
            NextStep::Room(manifest.room("rest_aftermath").unwrap().clone())
        );
        assert_eq!(
            next_room(&manifest, "rest_aftermath").unwrap(),
            NextStep::Room(manifest.room("mastery").unwrap().clone())
        );
        assert_eq!(next_room(&manifest, "mastery").unwrap(), NextStep::FloorEnd);
        assert!(next_room(&manifest, "cellar").is_err(), "unknown");
    }

    #[test]
    fn a_room_listed_later_joins_the_run_with_no_code_change() {
        // The committed manifest (CORRAL-F listed the corral, DR-A the dozer: every
        // room of its story order is listed) plus a made-up room listed after the
        // dozer: the chain follows the rows, whatever they are.
        let text = std::fs::read_to_string(root().join(crate::f01_late::MANIFEST_PATH)).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let rooms = value["rooms"].as_array_mut().unwrap();
        let made_up = serde_json::json!({
            "id": "cellar", "title": "A Made-Up Room", "scene": "f01_boiler",
            "rules": "cf02_rules", "milestone": "greybox"
        });
        rooms.insert(8, made_up);
        let order = value["story_order"].as_array_mut().unwrap();
        let at = order.iter().position(|o| o["id"] == "dozer").unwrap();
        let mut row = order[at].clone();
        row["id"] = "cellar".into();
        order.insert(at + 1, row);
        let manifest = Manifest::from_json(&value.to_string(), StoryTable::active()).unwrap();
        assert_eq!(first_room(&manifest).id, "act1_chase");
        let mut seen = vec!["act1_chase".to_owned()];
        while let NextStep::Room(row) = next_room(&manifest, seen.last().unwrap()).unwrap() {
            seen.push(row.id);
        }
        assert_eq!(
            seen,
            [
                "act1_chase",
                "act1_shore",
                "act1_guild",
                "corral",
                "rest_refuge",
                "hoarder",
                "boiler",
                "dozer",
                "cellar",
                "hazards",
                "ring_court",
                "ring_split",
                "rest_camp",
                "trog_run",
                "juicer",
                "rest_aftermath",
                "mastery"
            ]
        );
        assert_eq!(room_index(&manifest, "dozer"), Some((8, 17)));
        assert_eq!(room_index(&manifest, "cellar"), Some((9, 17)));
    }

    #[test]
    fn relaunch_argv_replaces_the_room_flags_and_keeps_the_rest() {
        let snapshot = hoarder_defeat(&root()).unwrap().snapshot().unwrap();
        let first = argv(&[
            "floor01_scene",
            "--scene",
            "f01_late",
            "--f01-run",
            "--scale",
            "3",
            "--companion",
            "cf02_free",
        ]);
        let tally = RunTally {
            rooms: 1,
            ticks: 2400,
            deaths: 1,
            restarts: 0,
        };
        let second = relaunch_argv(&first, "boiler", Some(&snapshot), tally, None);
        assert_eq!(&second[..8], &first[..]);
        assert_eq!(&second[8..10], ["--f01-room", "boiler"]);
        assert_eq!(second[10], FLAG_CARRY);
        assert_eq!(decode_carry(&second[11]).unwrap(), snapshot);
        assert_eq!(&second[12..], [FLAG_TALLY, "1:2400:1:0"]);
        // A second relaunch replaces, never duplicates.
        let third = relaunch_argv(&second, "hazards", None, tally, None);
        assert_eq!(third.iter().filter(|a| *a == "--f01-room").count(), 1);
        assert!(!third.iter().any(|a| a == FLAG_CARRY));
        assert_eq!(third.iter().filter(|a| *a == FLAG_TALLY).count(), 1);
        assert_eq!(third.iter().filter(|a| *a == FLAG_RUN).count(), 1);
        assert_eq!(&third[8..10], ["--f01-room", "hazards"]);
    }

    #[test]
    fn the_tally_round_trips_and_rejects_bad_text() {
        let t = RunTally {
            rooms: 3,
            ticks: 12_345,
            deaths: 2,
            restarts: 7,
        };
        assert_eq!(RunTally::parse(&t.to_arg()).unwrap(), t);
        for bad in ["", "1:2:3", "1:2:3:4:5", "a:2:3:4", "-1:0:0:0"] {
            assert!(RunTally::parse(bad).is_err(), "{bad:?}");
        }
        assert!(decode_carry("{").is_err());
    }

    #[test]
    fn audience_carry_round_trips_and_is_replaced_on_relaunch() {
        let carry = AudienceCarry {
            carl: (1_400, 1_900),
            donut: (600, 800),
        };
        assert_eq!(carry.to_arg(), "1400:1900:600:800");
        assert_eq!(AudienceCarry::parse(&carry.to_arg()).unwrap(), carry);
        assert_eq!(AudienceCarry::from_counts(carry.counts()), carry);
        // The boundary values parse; everything else is an error naming the flag.
        assert!(AudienceCarry::parse("250:250:250:250").is_ok());
        for bad in [
            "",
            "1:2:3",
            "1:2:3:4:5",
            "250:250:250",
            "249:300:300:300",
            "300:299:300:300",
            "300:300:249:300",
            "300:300:400:399",
            "a:300:300:300",
            "-300:300:300:300",
            "+300:300:300:300",
            "300::300:300",
            "300:300:300:99999999999999999999999",
        ] {
            let error = AudienceCarry::parse(bad).unwrap_err();
            assert!(error.contains(FLAG_AUDIENCE), "{bad:?}: {error}");
        }
        // relaunch_argv writes it once and drops an older one.
        let first = argv(&["floor01_scene", "--scene", "f01_late", "--f01-run"]);
        let tally = RunTally::default();
        let second = relaunch_argv(&first, "boiler", None, tally, Some(carry));
        assert_eq!(second.iter().filter(|a| *a == FLAG_AUDIENCE).count(), 1);
        let at = second.iter().position(|a| a == FLAG_AUDIENCE).unwrap();
        assert_eq!(second[at + 1], "1400:1900:600:800");
        let newer = AudienceCarry {
            carl: (2_000, 2_000),
            donut: (700, 900),
        };
        let third = relaunch_argv(&second, "hazards", None, tally, Some(newer));
        assert_eq!(third.iter().filter(|a| *a == FLAG_AUDIENCE).count(), 1);
        let at = third.iter().position(|a| a == FLAG_AUDIENCE).unwrap();
        assert_eq!(third[at + 1], "2000:2000:700:900");
        // No carry (the audience was locked at the exit): the flag is dropped.
        let fourth = relaunch_argv(&third, "juicer", None, tally, None);
        assert!(!fourth.iter().any(|a| a == FLAG_AUDIENCE));
        // An opt-out flag the run does not own survives every relaunch.
        let off = argv(&["floor01_scene", "--f01-run", "--audience=off"]);
        assert!(
            relaunch_argv(&off, "boiler", None, tally, None).contains(&"--audience=off".to_owned())
        );
    }

    #[test]
    fn the_audience_is_live_only_after_the_corral() {
        let manifest = manifest();
        let listed = manifest.listed();
        // ACT1-A: the Act I rooms come first, so the broadcast is off in them as in the corral.
        let corral = listed.iter().position(|r| *r == "corral").unwrap();
        assert_eq!(listed[..corral], ["act1_chase", "act1_shore", "act1_guild"]);
        for room in &listed[..=corral] {
            assert!(!audience_live_in(&manifest, room), "{room}");
        }
        for room in &listed[corral + 1..] {
            assert!(audience_live_in(&manifest, room), "{room}");
        }
        assert!(!audience_live_in(&manifest, "cellar"), "unlisted room");
        // A manifest without the corral row: nothing unlocks (fail closed).
        // Built in memory: `Manifest::validate` refuses such a manifest on load
        // (CORRAL-F), so the rule is pinned on the struct itself, and the load
        // guard is pinned too.
        let mut headless = self::manifest();
        headless.rooms.retain(|r| r.id != "corral");
        assert!(!headless.rooms.is_empty());
        for row in &headless.rooms {
            assert!(!audience_live_in(&headless, &row.id), "{}", row.id);
        }
        let text = std::fs::read_to_string(root().join(crate::f01_late::MANIFEST_PATH)).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        value["rooms"]
            .as_array_mut()
            .unwrap()
            .retain(|r| r["id"] != "corral");
        value["story_order"]
            .as_array_mut()
            .unwrap()
            .retain(|o| o["id"] != "corral");
        assert!(Manifest::from_json(&value.to_string(), StoryTable::active()).is_err());
    }

    #[test]
    fn only_the_unlock_room_panel_announces_the_broadcast() {
        let manifest = manifest();
        let tally = RunTally::default();
        for (i, row) in manifest.rooms.iter().enumerate() {
            let Some(next) = manifest.rooms.get(i + 1) else {
                continue;
            };
            let facts = RoomFacts {
                id: row.id.clone(),
                title: row.title.clone(),
                ticks: 600,
                deaths: 0,
                restarts: 0,
                grants: vec![],
            };
            let off = room_clear_panel(&manifest, &facts, tally, next, true, "REC", false);
            let on = room_clear_panel(&manifest, &facts, tally, next, true, "REC", true);
            if row.id == AUDIENCE_UNLOCK_AFTER {
                // Exactly one extra line, before `next:`, and nothing else moves.
                assert_eq!(on.lines.len(), off.lines.len() + 1);
                assert_eq!(on.lines[3], BROADCAST_LINE);
                assert!(on.lines[4].starts_with("next: "));
                let mut without = on.lines.clone();
                without.remove(3);
                assert_eq!(without, off.lines);
                assert_eq!((&on.heading, &on.hint), (&off.heading, &off.hint));
            } else {
                assert_eq!(on, off, "{}", row.id);
            }
            // Never a viewer number or any count on a panel.
            for line in on.lines.iter().chain(&off.lines) {
                assert!(
                    !line.contains("viewers:") && !line.contains("250"),
                    "{line}"
                );
            }
        }
        // The same through the real exit decision.
        let (carl, donut) = Loadout::Story.flags();
        let (_, _, corral) =
            crate::f01_late::corral_start(&root(), 360, carl, donut, None).unwrap();
        let argv = argv(&["floor01_scene", "--scene", "f01_late", "--f01-run"]);
        let input = ExitInput {
            manifest: &manifest,
            room: "corral",
            loadout: Loadout::Story,
            combat: &corral,
            carried: None,
            tally: RunTally::default(),
            argv: &argv,
            recording: "REC",
            audience: None,
            audience_enabled: true,
        };
        let (_, panel, _) = on_exit(&input).unwrap();
        assert!(panel.lines.contains(&BROADCAST_LINE.to_owned()));
        let (_, panel, _) = on_exit(&ExitInput {
            audience_enabled: false,
            ..input
        })
        .unwrap();
        assert!(!panel.lines.contains(&BROADCAST_LINE.to_owned()));
    }

    #[test]
    fn the_test_loadout_tag_marks_the_all_loadout_only() {
        assert!(test_loadout_tag(Some(Loadout::All)));
        assert!(!test_loadout_tag(Some(Loadout::Story)));
        assert!(!test_loadout_tag(None));
    }

    #[test]
    fn an_exit_relaunches_into_the_next_room_or_ends_the_floor() {
        let manifest = manifest();
        let hoarder = hoarder_defeat(&root()).unwrap();
        let argv = argv(&["floor01_scene", "--scene", "f01_late", "--f01-run"]);
        let input = ExitInput {
            manifest: &manifest,
            room: "hoarder",
            loadout: Loadout::Story,
            combat: &hoarder.combat,
            carried: None,
            tally: RunTally::default(),
            argv: &argv,
            recording: "REC",
            audience: None,
            audience_enabled: false,
        };
        let (facts, panel, decision) = on_exit(&input).unwrap();
        assert_eq!(facts.grants, [(0, "special_thrown_charge".to_owned())]);
        let (at, count) = room_index(&manifest, "hoarder").unwrap();
        assert!(
            panel
                .heading
                .starts_with(&format!("ROOM {at}/{count} CLEARED")),
            "{}",
            panel.heading
        );
        let Decision::Relaunch { room, argv: next } = decision else {
            panic!("the Hoarder leads on");
        };
        assert_eq!(room, "boiler");
        let carry = next.iter().position(|a| a == FLAG_CARRY).unwrap();
        assert_eq!(
            decode_carry(&next[carry + 1]).unwrap(),
            hoarder.snapshot().unwrap()
        );
        let tally = next.iter().position(|a| a == FLAG_TALLY).unwrap();
        assert_eq!(
            RunTally::parse(&next[tally + 1]).unwrap(),
            RunTally::default().with(&facts)
        );
        // The TEST LOADOUT chains without a carry.
        let all = ExitInput {
            loadout: Loadout::All,
            ..input
        };
        let (_, panel, Decision::Relaunch { argv: next, .. }) = on_exit(&all).unwrap() else {
            panic!("relaunch");
        };
        assert!(!next.iter().any(|a| a == FLAG_CARRY));
        assert!(panel.lines[3].contains("TEST LOADOUT, nothing carried"));
        // The last listed room ends the floor; an unknown room is an error.
        let last = &manifest.rooms.last().unwrap().id;
        let end = ExitInput {
            room: last,
            tally: RunTally {
                rooms: u32::try_from(manifest.rooms.len() - 1).unwrap(),
                ..RunTally::default()
            },
            ..input
        };
        let (_, panel, decision) = on_exit(&end).unwrap();
        assert_eq!(decision, Decision::Quit);
        assert_eq!(panel.heading, "FLOOR 1 SLICE FINISHED");
        let total = manifest.rooms.len();
        assert!(
            panel.lines[2].contains(&format!("{total} of {total} listed rooms")),
            "{}",
            panel.lines[2]
        );
        assert!(
            on_exit(&ExitInput {
                room: "cellar",
                ..input
            })
            .is_err()
        );
    }

    /// CORRAL-F, REST-WIRE: the corral's exit relaunches the run into the refuge (room 1)
    /// with the corral's (empty) ledger carried, and the refuge accepts it: it arms the
    /// reward trigger under the Story flags and holds nothing.
    #[test]
    fn the_corral_exit_relaunches_into_the_refuge() {
        let manifest = manifest();
        let (carl, donut) = Loadout::Story.flags();
        let (_, _, corral) =
            crate::f01_late::corral_start(&root(), 360, carl, donut, None).unwrap();
        let argv = argv(&["floor01_scene", "--scene", "f01_late", "--f01-run"]);
        let input = ExitInput {
            manifest: &manifest,
            room: "corral",
            loadout: Loadout::Story,
            combat: &corral,
            carried: None,
            tally: RunTally::default(),
            argv: &argv,
            recording: "REC",
            audience: None,
            audience_enabled: false,
        };
        let (facts, panel, decision) = on_exit(&input).unwrap();
        assert!(facts.grants.is_empty(), "the corral grants nothing (KC11)");
        let (at, count) = room_index(&manifest, "corral").unwrap();
        assert!(
            panel
                .heading
                .starts_with(&format!("ROOM {at}/{count} CLEARED: THE INFERNAL CORRAL")),
            "{}",
            panel.heading
        );
        assert!(
            panel.lines[3].contains("The Wrong Restaurant (story ledger carried)"),
            "{:?}",
            panel.lines
        );
        let Decision::Relaunch { room, argv: next } = decision else {
            panic!("the corral leads on");
        };
        assert_eq!(room, "rest_refuge");
        let at = next.iter().position(|a| a == "--f01-room").unwrap();
        assert_eq!(next[at + 1], "rest_refuge");
        let carry = next.iter().position(|a| a == FLAG_CARRY).unwrap();
        let snapshot = decode_carry(&next[carry + 1]).unwrap();
        // The corral runs no story runtime (its pin is empty, KC11): the run
        // carries the empty ledger (CF-AD3).
        assert!(corral.story_snapshot().is_none());
        assert!(snapshot.held.is_empty() && snapshot.receipts.is_empty());
        assert_eq!(
            Some(snapshot.clone()),
            carry_for(Loadout::Story, RoomScene::Corral, &corral)
        );
        // The fallback is the corral's only: a story-less state in any other
        // room carries nothing (review finding F4 of #283).
        assert_eq!(None, carry_for(Loadout::Story, RoomScene::Boiler, &corral));
        assert_eq!(None, carry_for(Loadout::All, RoomScene::Corral, &corral));
        // The refuge accepts the empty ledger and keeps its reward trigger armed.
        let (_, _, mut refuge) = crate::f01_late::rest_start(
            &root(),
            crate::f01_rest::RestRoom::Refuge,
            360,
            carl,
            donut,
            None,
        )
        .unwrap();
        refuge.carry_story(&snapshot).unwrap();
        assert!(refuge.story_zone_armed(1), "the box can still grant");
        let tally = next.iter().position(|a| a == FLAG_TALLY).unwrap();
        assert_eq!(RunTally::parse(&next[tally + 1]).unwrap().rooms, 1);
    }

    /// ACT1-T02: each Act I exit carries the empty ledger under Story (a story-less scene), the
    /// next room accepts it (the corral too: `carry_story` takes an empty snapshot where no
    /// story runtime runs), and under the All loadout nothing is carried.
    #[test]
    fn act1_carry_is_empty_and_accepted() {
        let (carl, donut) = Loadout::Story.flags();
        for scene in [
            RoomScene::Act1Shore,
            RoomScene::Act1Guild,
            RoomScene::Corral,
        ] {
            assert!(scene.story_less());
            let row = manifest()
                .rooms
                .iter()
                .find(|r| r.scene == scene)
                .unwrap()
                .clone();
            let (combat, snapshot) = room_run(&row, None);
            let snapshot = snapshot.expect("a story-less room carries the empty ledger");
            assert!(
                snapshot.held.is_empty() && snapshot.receipts.is_empty(),
                "{}",
                row.id
            );
            assert_eq!(carry_for(Loadout::All, scene, &combat), None, "{}", row.id);
            // The next room accepts it.
            let (_, _, mut corral) =
                crate::f01_late::corral_start(&root(), 360, carl, donut, None).unwrap();
            corral.carry_story(&snapshot).unwrap();
            let (_, _, mut shore) = crate::f01_late::act1_start(
                &root(),
                crate::f01_act1::Act1Room::Shore,
                360,
                carl,
                donut,
                None,
            )
            .unwrap();
            shore.carry_story(&snapshot).unwrap();
        }
        // A story room that is not story-less never claims a carried ledger by accident.
        assert!(!RoomScene::Boiler.story_less());
    }

    /// ACT1-B: the chase is the run's first room: its exit relaunches into the shore carrying the
    /// empty ledger under the Story loadout (a story-less scene), and nothing under All; the panel
    /// reports no grant and the broadcast is off (the audience goes live after the corral).
    #[test]
    fn the_chase_exit_relaunches_into_the_shore() {
        let manifest = manifest();
        let argv = argv(&["floor01_scene", "--scene", "f01_late", "--f01-run"]);
        let chase_row = manifest.room("act1_chase").unwrap().clone();
        let (chase, snapshot) = room_run(&chase_row, None);
        assert_eq!(first_room(&manifest).id, "act1_chase");
        let snapshot = snapshot.expect("a story-less room carries the empty ledger");
        assert!(snapshot.held.is_empty() && snapshot.receipts.is_empty());
        let input = ExitInput {
            manifest: &manifest,
            room: "act1_chase",
            loadout: Loadout::Story,
            combat: &chase,
            carried: None,
            tally: RunTally::default(),
            argv: &argv,
            recording: "REC",
            audience: None,
            audience_enabled: true,
        };
        let (facts, panel, decision) = on_exit(&input).unwrap();
        assert!(facts.grants.is_empty(), "the chase grants nothing");
        assert_eq!(room_index(&manifest, "act1_chase"), Some((1, 16)));
        assert!(
            panel
                .heading
                .starts_with("ROOM 1/16 CLEARED: THE FIRST CHASE"),
            "{}",
            panel.heading
        );
        assert!(
            panel.lines[3].contains("The Rotten Shore (story ledger carried)"),
            "{:?}",
            panel.lines
        );
        assert!(!panel.lines.contains(&BROADCAST_LINE.to_owned()));
        let Decision::Relaunch { room, argv: next } = decision else {
            panic!("the chase leads on");
        };
        assert_eq!(room, "act1_shore");
        let carry = next.iter().position(|a| a == FLAG_CARRY).unwrap();
        let carried = decode_carry(&next[carry + 1]).unwrap();
        assert!(carried.held.is_empty() && carried.receipts.is_empty());
        let (carl, donut) = Loadout::Story.flags();
        let (_, _, mut shore) = crate::f01_late::act1_start(
            &root(),
            crate::f01_act1::Act1Room::Shore,
            360,
            carl,
            donut,
            None,
        )
        .unwrap();
        shore.carry_story(&carried).unwrap();
        // Under the All loadout nothing is carried.
        let all = ExitInput {
            loadout: Loadout::All,
            ..input
        };
        let (_, panel, Decision::Relaunch { argv: next, .. }) = on_exit(&all).unwrap() else {
            panic!("relaunch");
        };
        assert!(!next.iter().any(|a| a == FLAG_CARRY));
        assert!(panel.lines[3].contains("TEST LOADOUT, nothing carried"));
    }

    /// ACT1-A: the run walks the Act I rooms: the shore's exit relaunches into the guild, the
    /// guild's (with the biscuit on its panel) into the corral, each carrying the empty ledger
    /// (ACT1-T12, the walk of the first rooms).
    #[test]
    fn the_guild_exit_relaunches_into_the_corral() {
        let manifest = manifest();
        let argv = argv(&["floor01_scene", "--scene", "f01_late", "--f01-run"]);
        // The shore leads to the guild.
        let shore_row = manifest.room("act1_shore").unwrap().clone();
        let (shore, snapshot) = room_run(&shore_row, None);
        let shore_input = ExitInput {
            manifest: &manifest,
            room: "act1_shore",
            loadout: Loadout::Story,
            combat: &shore,
            carried: None,
            tally: RunTally::default(),
            argv: &argv,
            recording: "REC",
            audience: None,
            audience_enabled: true,
        };
        let (facts, panel, decision) = on_exit(&shore_input).unwrap();
        assert!(facts.grants.is_empty(), "the shore grants nothing");
        let (at, count) = room_index(&manifest, "act1_shore").unwrap();
        assert_eq!((at, count), (2, 16));
        assert!(
            panel
                .heading
                .starts_with("ROOM 2/16 CLEARED: THE ROTTEN SHORE"),
            "{}",
            panel.heading
        );
        assert!(
            panel.lines[3].contains("The Guild Door (story ledger carried)"),
            "{:?}",
            panel.lines
        );
        let Decision::Relaunch { room, argv: next } = decision else {
            panic!("the shore leads on");
        };
        assert_eq!(room, "act1_guild");
        assert!(next.iter().any(|a| a == FLAG_CARRY));
        assert!(
            !panel.lines.contains(&BROADCAST_LINE.to_owned()),
            "no broadcast before the corral"
        );
        // The guild leads to the corral; its panel names the biscuit (a read-through grant).
        let guild_row = manifest.room("act1_guild").unwrap().clone();
        let (guild, _) = room_run(&guild_row, snapshot.as_ref());
        let guild_input = ExitInput {
            room: "act1_guild",
            combat: &guild,
            carried: snapshot.as_ref(),
            ..shore_input
        };
        let (facts, panel, decision) = on_exit(&guild_input).unwrap();
        assert_eq!(facts.grants, [(1, "primary_ranged".to_owned())]);
        assert!(
            panel.lines[1].contains("primary_ranged (Donut)"),
            "{:?}",
            panel.lines
        );
        assert!(
            panel
                .heading
                .starts_with("ROOM 3/16 CLEARED: THE GUILD DOOR"),
            "{}",
            panel.heading
        );
        assert!(
            panel.lines[3].contains("The Infernal Corral (story ledger carried)"),
            "{:?}",
            panel.lines
        );
        let Decision::Relaunch { room, argv: next } = decision else {
            panic!("the guild leads on");
        };
        assert_eq!(room, "corral");
        let carry = next.iter().position(|a| a == FLAG_CARRY).unwrap();
        let carried = decode_carry(&next[carry + 1]).unwrap();
        assert!(
            carried.held.is_empty() && carried.receipts.is_empty(),
            "the ledger never holds the biscuit"
        );
        let (carl, donut) = Loadout::Story.flags();
        let (_, _, mut corral) =
            crate::f01_late::corral_start(&root(), 360, carl, donut, None).unwrap();
        corral.carry_story(&carried).unwrap();
        // Under the All loadout the guild's panel states the TEST LOADOUT and carries nothing.
        let all = ExitInput {
            loadout: Loadout::All,
            ..guild_input
        };
        let (_, panel, Decision::Relaunch { argv: next, .. }) = on_exit(&all).unwrap() else {
            panic!("relaunch");
        };
        assert!(!next.iter().any(|a| a == FLAG_CARRY));
        assert!(panel.lines[3].contains("TEST LOADOUT, nothing carried"));
    }

    /// REST-WIRE (REST-A): a refuge crossing's exit snapshot holds Donut's boost with
    /// exactly one receipt (the grant of the box), the run carries it on the relaunch,
    /// and the Hoarder starts from it (her pin holds the boost too: the carry unions in).
    #[test]
    fn the_refuge_exit_carries_light_on_your_feet_into_the_hoarder() {
        use crate::rest_fixtures::Case;
        let manifest = manifest();
        let (carl, donut) = Loadout::Story.flags();
        let crossing = crate::rest_fixtures::run_sim(&root(), Case::SoloDonut).unwrap();
        let combat: &CombatState = &crossing.combat;
        let snapshot = combat
            .story_snapshot()
            .expect("a refuge runs a story runtime");
        assert_eq!(snapshot.held.len(), 1, "{:?}", snapshot.held);
        assert_eq!(snapshot.receipts.len(), 1, "exactly one receipt");
        assert_eq!(snapshot.receipts[0].capability, "jump_reach_boost");
        assert_eq!(snapshot.receipts[0].source, "f01_safe_room_reward_found");
        let argv = argv(&["floor01_scene", "--scene", "f01_late", "--f01-run"]);
        let input = ExitInput {
            manifest: &manifest,
            room: "rest_refuge",
            loadout: Loadout::Story,
            combat,
            carried: Some(&LedgerSnapshot {
                version: snapshot.version,
                held: Vec::new(),
                receipts: Vec::new(),
            }),
            tally: RunTally::default(),
            argv: &argv,
            recording: "REC",
            audience: None,
            audience_enabled: false,
        };
        let (facts, panel, decision) = on_exit(&input).unwrap();
        assert_eq!(
            facts.grants,
            [(1, "jump_reach_boost".to_owned())],
            "the grant is the refuge's"
        );
        assert!(
            panel.heading.starts_with(&format!(
                "ROOM {}/{} CLEARED: THE WRONG RESTAURANT",
                room_index(&manifest, "rest_refuge").unwrap().0,
                manifest.rooms.len()
            )),
            "{}",
            panel.heading
        );
        let Decision::Relaunch { room, argv: next } = decision else {
            panic!("the refuge leads on");
        };
        assert_eq!(room, "hoarder");
        let carry = next.iter().position(|a| a == FLAG_CARRY).unwrap();
        assert_eq!(decode_carry(&next[carry + 1]).unwrap(), snapshot);
        // The Hoarder accepts the carried ledger: one boost, one receipt, nothing doubled.
        let (_, _, mut hoarder) =
            crate::f01_late::hoarder_start(&root(), 360, carl, donut).unwrap();
        hoarder.carry_story(&snapshot).unwrap();
        let carried = hoarder.story_snapshot().unwrap();
        assert!(
            carried
                .receipts
                .iter()
                .filter(|r| r.capability == "jump_reach_boost")
                .count()
                == 1,
            "{carried:?}"
        );
        assert!(hoarder.story_holds(crate::donut_prog::JUMP_REACH_BOOST));
        let tally = next.iter().position(|a| a == FLAG_TALLY).unwrap();
        assert_eq!(RunTally::parse(&next[tally + 1]).unwrap().rooms, 1);
    }

    /// One listed room's tick-0 state under the story loadout, built through the
    /// constructor the binary and the verifier share, then carried. The boss
    /// room is the PT-SCENE fixture (the Juicer nearly dead, defeated by the
    /// scripted pilot), the Hoarder her defeat run, the corral (room 0, CORRAL-F)
    /// its story start.
    fn room_run(
        row: &RoomRow,
        carry: Option<&LedgerSnapshot>,
    ) -> (CombatState, Option<LedgerSnapshot>) {
        let (carl, donut) = Loadout::Story.flags();
        let combat = match row.scene {
            RoomScene::Hoarder => {
                // CORRAL-F: the Hoarder follows the corral, whose ledger holds
                // nothing the Hoarder's pin does not (KC11), so her defeat run
                // (built from the pin) is the carried room's: the carry changes
                // nothing at tick 0.
                let (_, _, mut carried) =
                    crate::f01_late::hoarder_start(&root(), 360, carl, donut).unwrap();
                let (_, _, plain) =
                    crate::f01_late::hoarder_start(&root(), 360, carl, donut).unwrap();
                if let Some(snapshot) = carry {
                    carried.carry_story(snapshot).unwrap();
                }
                assert_eq!(carried.story_snapshot(), plain.story_snapshot());
                hoarder_defeat(&root()).unwrap().combat
            }
            RoomScene::Boss => {
                juicer_relaunch(&root(), Loadout::Story, carry)
                    .unwrap()
                    .combat
            }
            // ACT1: the Act I rooms open the run. The chase opens it (nothing carried), the
            // shore follows it with an empty carry; the guild is read after its crossing, so its
            // panel fact (the biscuit) shows.
            RoomScene::Act1Chase => {
                assert!(carry.is_none(), "the chase is the run's first room");
                crate::f01_late::act1_start(
                    &root(),
                    crate::f01_act1::Act1Room::Chase,
                    360,
                    carl,
                    donut,
                    None,
                )
                .unwrap()
                .2
            }
            RoomScene::Act1Shore => {
                assert!(carry.is_none_or(|c| c.held.is_empty() && c.receipts.is_empty()));
                crate::f01_late::act1_start(
                    &root(),
                    crate::f01_act1::Act1Room::Shore,
                    360,
                    carl,
                    donut,
                    None,
                )
                .unwrap()
                .2
            }
            RoomScene::Act1Guild => {
                assert!(carry.is_none_or(|c| c.held.is_empty() && c.receipts.is_empty()));
                crate::act1_fixtures::run_sim(&root(), crate::act1_fixtures::Case::GuildSoloDonut)
                    .unwrap()
                    .combat
            }
            // The corral follows the Act I rooms: an empty carry (ACT1-A), accepted.
            RoomScene::Corral => {
                assert!(carry.is_none_or(|c| c.held.is_empty() && c.receipts.is_empty()));
                let (_, _, mut corral) =
                    crate::f01_late::corral_start(&root(), 360, carl, donut, None).unwrap();
                if let Some(snapshot) = carry {
                    corral.carry_story(snapshot).unwrap();
                }
                corral
            }
            RoomScene::Boiler
            | RoomScene::Dozer
            | RoomScene::Hazards
            | RoomScene::Mastery
            | RoomScene::RestRefuge
            | RoomScene::RestCamp
            | RoomScene::RestAftermath
            | RoomScene::RingCourt
            | RoomScene::RingSplit
            | RoomScene::TrogRun => {
                let (_, mut combat) = match row.scene {
                    // TROG-A: the troglodyte run, after the camp.
                    RoomScene::TrogRun => crate::f01_late::trog_start(
                        &root(),
                        row.scene.trog().expect("a troglodyte scene"),
                        360,
                        carl,
                        donut,
                        None,
                    )
                    .map(|(_, s, c)| (s, c)),
                    // REST-WIRE: the quiet rooms (the refuge arms its trigger under the Story
                    // flags; the camp and the aftermath run under their room pins).
                    RoomScene::RestRefuge | RoomScene::RestCamp | RoomScene::RestAftermath => {
                        crate::f01_late::rest_start(
                            &root(),
                            row.scene.rest().expect("a quiet-room scene"),
                            360,
                            carl,
                            donut,
                            None,
                        )
                        .map(|(_, s, c)| (s, c))
                    }
                    // RING-A/B wire: the two Ring rooms of the run.
                    RoomScene::RingCourt | RoomScene::RingSplit => crate::f01_late::ring_start(
                        &root(),
                        row.scene.ring().expect("a Ring scene"),
                        360,
                        carl,
                        donut,
                        None,
                    )
                    .map(|(_, s, c)| (s, c)),
                    RoomScene::Boiler => {
                        crate::f01_late::boiler_start(&root(), 360, carl, donut, None)
                            .map(|(_, s, c)| (s, c))
                    }
                    RoomScene::Hazards => {
                        crate::f01_late::hazards_start(&root(), 360, carl, donut, None)
                            .map(|(_, s, c)| (s, c))
                    }
                    // DR-A: room 3, after the Boiler.
                    RoomScene::Dozer => {
                        crate::f01_late::dozer_start(&root(), 360, carl, donut, None)
                            .map(|(_, s, c)| (s, c))
                    }
                    _ => crate::f01_late::mastery_start(&root(), 360, carl, donut, None)
                        .map(|(_, s, c)| (s, c)),
                }
                .unwrap();
                if let Some(snapshot) = carry {
                    combat.carry_story(snapshot).unwrap();
                }
                combat
            }
        };
        let snapshot = carry_for(Loadout::Story, row.scene, &combat);
        (combat, snapshot)
    }

    /// The carry of the run up to and including `last`, built room by room through the same
    /// constructors the relaunch uses (each room's snapshot carried into the next).
    fn carry_up_to(manifest: &Manifest, last: &str) -> LedgerSnapshot {
        let mut carry: Option<LedgerSnapshot> = None;
        for row in &manifest.rooms {
            carry = room_run(row, carry.as_ref()).1;
            if row.id == last {
                break;
            }
        }
        carry.expect("the run carries a ledger")
    }

    /// REST-C-03: the ledger passes through the camp and the aftermath untouched. A camp
    /// crossing entered with `ring_split`'s ledger leaves with exactly that ledger (nothing
    /// granted, no receipt added or dropped), the exit's relaunch argv carries it into the
    /// Juicer, and the Juicer launches from it with the Hoarder's dynamite still one receipt.
    #[test]
    fn the_camp_and_aftermath_carry_their_ledger_through() {
        use crate::f01_rest::RestRoom;
        use crate::rest_fixtures::{BOT, Party, RestSim, walk_to_the_exit};
        let manifest = manifest();
        let (carl, donut) = Loadout::Story.flags();
        let argv = argv(&["floor01_scene", "--scene", "f01_late", "--f01-run"]);
        // ring_split exit -> camp.
        let carry = carry_up_to(&manifest, "ring_split");
        assert_eq!(
            carry_for(
                Loadout::Story,
                RoomScene::RingSplit,
                &room_run(manifest.room("ring_split").unwrap(), Some(&carry)).0
            ),
            Some(carry.clone()),
            "the Ring's exit hands the ledger on unchanged"
        );
        let mut camp = RestSim::new_in(
            &root(),
            RestRoom::Camp,
            360,
            carl,
            donut,
            Party::Solo { uncontrolled: BOT },
            Some(&carry),
        )
        .unwrap();
        walk_to_the_exit(&mut camp, 1200);
        assert!(camp.complete(), "the camp is crossed");
        assert_eq!(camp.combat.story_snapshot(), Some(carry.clone()));
        let input = ExitInput {
            manifest: &manifest,
            room: "rest_camp",
            loadout: Loadout::Story,
            combat: &camp.combat,
            carried: Some(&carry),
            tally: RunTally::default(),
            argv: &argv,
            recording: "REC",
            audience: None,
            audience_enabled: false,
        };
        let (facts, _, decision) = on_exit(&input).unwrap();
        assert!(facts.grants.is_empty(), "the camp grants nothing");
        let Decision::Relaunch { room, argv: next } = decision else {
            panic!("the camp leads on");
        };
        // TROG-A: the camp leads into the troglodyte run.
        assert_eq!(room, "trog_run");
        let at = next.iter().position(|a| a == FLAG_CARRY).unwrap();
        assert_eq!(
            decode_carry(&next[at + 1]).unwrap(),
            carry,
            "carried unchanged"
        );
        // TROG-T04: the troglodyte run hands the camp's ledger on unchanged (it grants nothing),
        // and its exit relaunches into the Juicer.
        let (_, _, mut trog) = crate::f01_late::trog_start(
            &root(),
            crate::f01_trog::TrogRoom::Run,
            360,
            carl,
            donut,
            None,
        )
        .unwrap();
        trog.carry_story(&carry).unwrap();
        assert_eq!(
            carry_for(Loadout::Story, RoomScene::TrogRun, &trog),
            Some(carry.clone()),
            "the troglodyte run's exit hands the ledger on unchanged"
        );
        let trog_input = ExitInput {
            manifest: &manifest,
            room: "trog_run",
            loadout: Loadout::Story,
            combat: &trog,
            carried: Some(&carry),
            tally: RunTally::default(),
            argv: &argv,
            recording: "REC",
            audience: None,
            audience_enabled: false,
        };
        let (facts, _, decision) = on_exit(&trog_input).unwrap();
        assert!(facts.grants.is_empty(), "the troglodyte run grants nothing");
        let Decision::Relaunch { room, argv: next } = decision else {
            panic!("the troglodyte run leads on");
        };
        assert_eq!(room, "juicer");
        let at = next.iter().position(|a| a == FLAG_CARRY).unwrap();
        assert_eq!(decode_carry(&next[at + 1]).unwrap(), carry);
        // The Juicer launches from the camp's ledger and still holds one dynamite receipt.
        let juicer = juicer_relaunch(&root(), Loadout::Story, Some(&carry)).unwrap();
        let after = juicer.combat.story_snapshot().unwrap();
        for receipt in &carry.receipts {
            assert!(after.receipts.contains(receipt), "{receipt:?} kept");
        }
        assert_eq!(
            after
                .receipts
                .iter()
                .filter(|r| r.capability == SPECIAL_THROWN_CHARGE.as_str())
                .count(),
            1
        );
    }

    /// REST-C-03: juicer exit -> aftermath -> mastery. The aftermath, entered with the Juicer's
    /// ledger (every capability with its receipt), grants nothing and leaves it unchanged; its
    /// exit relaunches into the mastery arena carrying it, and the arena accepts it with no
    /// extra receipt.
    #[test]
    fn the_aftermath_exit_relaunches_into_mastery() {
        use crate::f01_rest::RestRoom;
        use crate::rest_fixtures::{BOT, Party, RestSim, walk_to_the_exit};
        let manifest = manifest();
        let (carl, donut) = Loadout::Story.flags();
        let argv = argv(&["floor01_scene", "--scene", "f01_late", "--f01-run"]);
        let camp_carry = carry_up_to(&manifest, "rest_camp");
        let juicer = juicer_relaunch(&root(), Loadout::Story, Some(&camp_carry)).unwrap();
        let carry = carry_for(Loadout::Story, RoomScene::Boss, &juicer.combat)
            .expect("the Juicer carries a ledger");
        assert!(carry.receipts.len() >= 3, "{:?}", carry.receipts);
        let mut aftermath = RestSim::new_in(
            &root(),
            RestRoom::Aftermath,
            360,
            carl,
            donut,
            Party::Solo { uncontrolled: BOT },
            Some(&carry),
        )
        .unwrap();
        walk_to_the_exit(&mut aftermath, 1200);
        assert!(aftermath.complete(), "the aftermath is crossed");
        assert_eq!(aftermath.combat.story_snapshot(), Some(carry.clone()));
        let input = ExitInput {
            manifest: &manifest,
            room: "rest_aftermath",
            loadout: Loadout::Story,
            combat: &aftermath.combat,
            carried: Some(&carry),
            tally: RunTally::default(),
            argv: &argv,
            recording: "REC",
            audience: None,
            audience_enabled: false,
        };
        let (facts, _, decision) = on_exit(&input).unwrap();
        assert!(facts.grants.is_empty(), "nothing is granted twice");
        let Decision::Relaunch { room, argv: next } = decision else {
            panic!("the aftermath leads on");
        };
        assert_eq!(room, "mastery");
        let at = next.iter().position(|a| a == FLAG_CARRY).unwrap();
        let carried = decode_carry(&next[at + 1]).unwrap();
        assert_eq!(carried, carry);
        // The arena accepts it: no receipt added, none dropped.
        let (_, _, mut mastery) =
            crate::f01_late::mastery_start(&root(), 360, carl, donut, None).unwrap();
        mastery.carry_story(&carried).unwrap();
        assert_eq!(mastery.story_snapshot().unwrap().receipts, carry.receipts);
    }

    /// Evidence helper (ignored): the carried chain of the whole run, one line per room: the
    /// ledger carried in, the ledger handed on through the relaunch argument and whether the
    /// room changed it. The quiet rooms are walked to their exit first.
    #[test]
    #[ignore = "evidence helper: prints the carried chain of the floor run"]
    fn print_the_carried_run_chain() {
        use crate::f01_rest::RestRoom;
        use crate::rest_fixtures::{BOT, Party, RestSim, walk_to_the_exit};
        let manifest = manifest();
        let (carl, donut) = Loadout::Story.flags();
        let n = manifest.rooms.len();
        let mut carry: Option<LedgerSnapshot> = None;
        let size = |c: &Option<LedgerSnapshot>| {
            c.as_ref()
                .map_or((0, 0), |c| (c.held.len(), c.receipts.len()))
        };
        for (i, row) in manifest.rooms.iter().enumerate() {
            let carried_in = size(&carry);
            let handed = match row.scene.rest() {
                // The camp and the aftermath are walked to their exit; the other rooms are read
                // at their start through the shared constructors (the refuge's own crossing is
                // covered by its fixtures; its boost is the Hoarder's pin here).
                Some(room) if room != RestRoom::Refuge => {
                    let mut sim = RestSim::new_in(
                        &root(),
                        room,
                        360,
                        carl,
                        donut,
                        Party::Solo { uncontrolled: BOT },
                        carry.as_ref(),
                    )
                    .unwrap();
                    walk_to_the_exit(&mut sim, 1200);
                    assert!(sim.complete(), "{} is crossed", row.id);
                    carry_for(Loadout::Story, row.scene, &sim.combat)
                }
                _ => room_run(row, carry.as_ref()).1,
            };
            let (held, receipts) = size(&handed);
            println!(
                "RUN_CHAIN {}/{n} room={} carried_in=held:{},receipts:{} handed_on=held:{},receipts:{} unchanged={}",
                i + 1,
                row.id,
                carried_in.0,
                carried_in.1,
                held,
                receipts,
                handed == carry
            );
            carry = handed;
        }
    }

    #[test]
    fn the_hoarders_dynamite_survives_every_listed_room_of_the_run() {
        let manifest = manifest();
        let mut carry: Option<LedgerSnapshot> = None;
        let mut grants = Vec::new();
        for row in &manifest.rooms {
            let (combat, snapshot) = room_run(row, carry.as_ref());
            let facts = RoomFacts::of(row, &combat, carry.as_ref());
            grants.push((row.id.clone(), facts.grants.clone()));
            if ![
                "act1_chase",
                "act1_shore",
                "act1_guild",
                "hoarder",
                "corral",
                "rest_refuge",
            ]
            .contains(&row.id.as_str())
            {
                assert!(
                    combat.story_holds(SPECIAL_THROWN_CHARGE),
                    "dynamite carried into {}",
                    row.id
                );
                let receipts = &snapshot.as_ref().unwrap().receipts;
                assert_eq!(
                    receipts
                        .iter()
                        .filter(|r| r.capability == SPECIAL_THROWN_CHARGE.as_str())
                        .count(),
                    1,
                    "one receipt, never a second grant, in {}",
                    row.id
                );
            }
            // The relaunch writes it on the argument list and reads it back.
            // Every room of the run carries a ledger: its story snapshot, or
            // the empty one for the corral only (CF-AD3), so a story-less room
            // anywhere else fails here.
            let snapshot = snapshot.unwrap_or_else(|| panic!("{} carries no ledger", row.id));
            let text = encode_carry(&snapshot);
            carry = Some(decode_carry(&text).unwrap());
        }
        // Each grant is reported once, in the room that made it.
        let expect: Vec<(String, Vec<(u8, String)>)> = manifest
            .rooms
            .iter()
            .map(|row| {
                let g: Vec<(u8, String)> = match row.id.as_str() {
                    "act1_guild" => vec![(1, BISCUIT_CAPABILITY.to_owned())],
                    "hoarder" => vec![(0, SPECIAL_THROWN_CHARGE.as_str().to_owned())],
                    "juicer" => vec![
                        (0, FIST_REINFORCED.as_str().to_owned()),
                        (
                            1,
                            crate::donut_prog::SPECIAL_SHORT_BLINK.as_str().to_owned(),
                        ),
                    ],
                    _ => vec![],
                };
                (row.id.clone(), g)
            })
            .collect();
        assert_eq!(grants, expect);
        // The last room ends the floor holding the Hoarder's and the Juicer's grants.
        let last = manifest.rooms.last().unwrap();
        let (combat, _) = room_run(last, carry.as_ref());
        let held = held_of(&combat);
        for cap in [SPECIAL_THROWN_CHARGE, FIST_REINFORCED] {
            assert!(held.iter().any(|(h, c)| *h == 0 && c == cap.as_str()));
        }
    }

    #[test]
    fn the_panels_state_facts_only_and_fit_the_overlay() {
        let manifest = manifest();
        let hoarder = hoarder_defeat(&root()).unwrap();
        let row = manifest.room("hoarder").unwrap();
        let facts = RoomFacts::of(row, &hoarder.combat, None);
        assert_eq!(facts.grants, [(0, "special_thrown_charge".to_owned())]);
        let tally = RunTally::default().with(&facts);
        assert_eq!(tally.rooms, 1);
        let next = manifest.room("boiler").unwrap();
        let panel = room_clear_panel(&manifest, &facts, tally, next, true, "REC", false);
        let (at, count) = room_index(&manifest, "hoarder").unwrap();
        assert!(
            panel
                .heading
                .starts_with(&format!("ROOM {at}/{count} CLEARED: THE HOARDER")),
            "{}",
            panel.heading
        );
        assert!(panel.lines[1].contains("special_thrown_charge (Carl)"));
        assert!(panel.lines[3].contains("Goblin Boiler (story ledger carried)"));
        assert!(panel.lines.last().unwrap().starts_with("Facts only"));
        let end = floor_end_panel(&manifest, &facts, tally, &held_of(&hoarder.combat), "REC");
        assert_eq!(end.heading, "FLOOR 1 SLICE FINISHED");
        assert!(
            end.lines[2].contains(&format!("1 of {} listed rooms", manifest.rooms.len())),
            "{}",
            end.lines[2]
        );
        assert!(end.lines[3].starts_with("Carl holds: special_thrown_charge"));
        // The real floor end: the last room's facts and the held list after the
        // Juicer's grants (the longest lines the panel draws today).
        let last = manifest.rooms.last().unwrap();
        let mut carry = hoarder.snapshot();
        // The rooms after the Hoarder (ACT1-F: the Act I rooms and the corral first,
        // REST-WIRE: the refuge before her).
        let after_hoarder = room_index(&manifest, "hoarder").unwrap().0;
        for row in &manifest.rooms[after_hoarder..manifest.rooms.len() - 1] {
            carry = room_run(row, carry.as_ref()).1;
        }
        let (mastery, _) = room_run(last, carry.as_ref());
        let last_facts = RoomFacts::of(last, &mastery, carry.as_ref());
        let real_end = floor_end_panel(&manifest, &last_facts, tally, &held_of(&mastery), "REC");
        for text in panel
            .lines
            .iter()
            .chain(&end.lines)
            .chain(&real_end.lines)
            .chain([&panel.heading, &panel.hint])
        {
            // The overlay draws from x 112 in the 8 px font's 5 px advance; the
            // panel box ends at x 544 (96 + 448).
            assert!(112 + 5 * text.chars().count() <= 544, "{text}");
            for word in ["score", "rating", "grade", "rank"] {
                assert!(
                    !text.to_lowercase().contains(word) || text.starts_with("Facts only"),
                    "{text}"
                );
            }
        }
    }
}
