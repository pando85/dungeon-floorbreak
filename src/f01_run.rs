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
use crate::f01_late::{Loadout, Manifest, RoomRow};
use crate::story_unlock::LedgerSnapshot;

/// The flag that makes `--scene f01_late` a floor run.
pub const FLAG_RUN: &str = "--f01-run";
/// The ledger snapshot of the previous room (JSON, written by a relaunch).
pub const FLAG_CARRY: &str = "--f01-carry";
/// The run's tally so far (`rooms:ticks:deaths:restarts`, written by a relaunch).
pub const FLAG_TALLY: &str = "--f01-run-tally";

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
            rooms: self.rooms + 1,
            ticks: self.ticks + room.ticks,
            deaths: self.deaths + room.deaths,
            restarts: self.restarts + room.restarts,
        }
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
/// run's own per-room flags (`--f01-room`, `--f01-carry`, `--f01-run-tally`)
/// replaced. Everything else (`--f01-run`, the loadout, the scale, the deck,
/// `--companion`, the recording flags) is kept.
pub fn relaunch_argv(
    argv: &[String],
    next: &str,
    carry: Option<&LedgerSnapshot>,
    tally: RunTally,
) -> Vec<String> {
    let mut out = Vec::with_capacity(argv.len() + 6);
    let mut iter = argv.iter();
    if let Some(program) = iter.next() {
        out.push(program.clone());
    }
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--f01-room" | FLAG_CARRY | FLAG_TALLY => {
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
    out
}

/// The ledger a relaunch carries: the room's snapshot under the story loadout,
/// nothing under the TEST LOADOUT (every room holds everything; adopted
/// F01RUN-AD2).
pub fn carry_for(loadout: Loadout, combat: &CombatState) -> Option<LedgerSnapshot> {
    match loadout {
        Loadout::Story => combat.story_snapshot(),
        Loadout::All => None,
    }
}

/// Whether the HUD shows the TEST LOADOUT tag (PT-SCENE U5): always under
/// `--f01-loadout all`; under the story loadout when the room started from its
/// static pin instead of the ledger the run carried into it. The first room of
/// a floor run has nothing to carry: its pin is the run's own start state.
pub fn test_loadout_tag(loadout: Option<Loadout>, floor_run_start: bool, carried: bool) -> bool {
    match loadout {
        Some(Loadout::All) => true,
        _ => !(floor_run_start || carried),
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
            grants: new_grants(combat.story_snapshot().as_ref(), carried),
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
) -> PanelText {
    let index =
        room_index(manifest, &facts.id).map_or_else(String::new, |(i, n)| format!("ROOM {i}/{n} "));
    PanelText {
        heading: format!("{index}CLEARED: {}", facts.title.to_uppercase()),
        lines: vec![
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
        ],
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
            "{}: {} ticks ({}), deaths {}, restarts {}, granted {}",
            facts.title,
            facts.ticks,
            clock(facts.ticks),
            facts.deaths,
            facts.restarts,
            grant_list(&facts.grants)
        ),
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
    use crate::f01_late::RoomScene;
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
        assert!(next_room(&manifest, "corral").is_err(), "unlisted");
    }

    #[test]
    fn a_room_listed_later_joins_the_run_with_no_code_change() {
        // The committed manifest with every room of its story order listed (as
        // CORRAL-WIRE, RING-A wire and the Dozer steps will list them): the
        // chain follows the rows, whatever they are.
        let text = std::fs::read_to_string(root().join(crate::f01_late::MANIFEST_PATH)).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let rooms = value["rooms"].as_array_mut().unwrap();
        let corral = serde_json::json!({
            "id": "corral", "title": "The Infernal Corral", "scene": "f01_boiler",
            "rules": "cf02_rules", "milestone": "greybox"
        });
        let dozer = serde_json::json!({
            "id": "dozer", "title": "Dozer Revenge", "scene": "f01_boiler",
            "rules": "cf02_rules", "milestone": "greybox"
        });
        rooms.insert(0, corral);
        rooms.insert(3, dozer);
        let manifest = Manifest::from_json(&value.to_string(), StoryTable::active()).unwrap();
        assert_eq!(first_room(&manifest).id, "corral");
        let mut seen = vec!["corral".to_owned()];
        while let NextStep::Room(row) = next_room(&manifest, seen.last().unwrap()).unwrap() {
            seen.push(row.id);
        }
        assert_eq!(
            seen,
            [
                "corral", "hoarder", "boiler", "dozer", "hazards", "juicer", "mastery"
            ]
        );
        assert_eq!(room_index(&manifest, "dozer"), Some((4, 7)));
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
        let second = relaunch_argv(&first, "boiler", Some(&snapshot), tally);
        assert_eq!(&second[..8], &first[..]);
        assert_eq!(&second[8..10], ["--f01-room", "boiler"]);
        assert_eq!(second[10], FLAG_CARRY);
        assert_eq!(decode_carry(&second[11]).unwrap(), snapshot);
        assert_eq!(&second[12..], [FLAG_TALLY, "1:2400:1:0"]);
        // A second relaunch replaces, never duplicates.
        let third = relaunch_argv(&second, "hazards", None, tally);
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
    fn the_test_loadout_tag_marks_a_pinned_start_and_the_all_loadout() {
        // A bare room launch starts from its pin: tagged.
        assert!(test_loadout_tag(Some(Loadout::Story), false, false));
        // The first room of a floor run and a carried room are the story.
        assert!(!test_loadout_tag(Some(Loadout::Story), true, false));
        assert!(!test_loadout_tag(Some(Loadout::Story), false, true));
        // `all` is always the TEST LOADOUT.
        assert!(test_loadout_tag(Some(Loadout::All), true, false));
        assert!(test_loadout_tag(Some(Loadout::All), false, true));
    }

    /// One listed room's tick-0 state under the story loadout, built through the
    /// constructor the binary and the verifier share, then carried. The boss
    /// room is the PT-SCENE fixture (the Juicer nearly dead, defeated by the
    /// scripted pilot), the Hoarder her defeat run.
    fn room_run(
        row: &RoomRow,
        carry: Option<&LedgerSnapshot>,
    ) -> (CombatState, Option<LedgerSnapshot>) {
        let (carl, donut) = Loadout::Story.flags();
        let combat = match row.scene {
            RoomScene::Hoarder => {
                assert!(carry.is_none(), "the Hoarder is the run's first room");
                hoarder_defeat(&root()).unwrap().combat
            }
            RoomScene::Boss => {
                juicer_relaunch(&root(), Loadout::Story, carry)
                    .unwrap()
                    .combat
            }
            RoomScene::Boiler | RoomScene::Hazards | RoomScene::Mastery => {
                let (_, mut combat) = match row.scene {
                    RoomScene::Boiler => {
                        crate::f01_late::boiler_start(&root(), 360, carl, donut, None)
                            .map(|(_, s, c)| (s, c))
                    }
                    RoomScene::Hazards => {
                        crate::f01_late::hazards_start(&root(), 360, carl, donut, None)
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
        let snapshot = carry_for(Loadout::Story, &combat);
        (combat, snapshot)
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
            if row.id != "hoarder" {
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
            let snapshot = snapshot.expect("a story room has a ledger");
            let text = encode_carry(&snapshot);
            carry = Some(decode_carry(&text).unwrap());
        }
        // Each grant is reported once, in the room that made it.
        let expect: Vec<(String, Vec<(u8, String)>)> = manifest
            .rooms
            .iter()
            .map(|row| {
                let g: Vec<(u8, String)> = match row.id.as_str() {
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
        let panel = room_clear_panel(&manifest, &facts, tally, next, true, "REC");
        assert!(panel.heading.starts_with("ROOM 1/5 CLEARED: THE HOARDER"));
        assert!(panel.lines[1].contains("special_thrown_charge (Carl)"));
        assert!(panel.lines[3].contains("Goblin Boiler (story ledger carried)"));
        assert!(panel.lines.last().unwrap().starts_with("Facts only"));
        let end = floor_end_panel(&manifest, &facts, tally, &held_of(&hoarder.combat), "REC");
        assert_eq!(end.heading, "FLOOR 1 SLICE FINISHED");
        assert!(end.lines[1].contains("1 of 5 listed rooms"));
        assert!(end.lines[2].starts_with("Carl holds: special_thrown_charge"));
        for text in panel.lines.iter().chain(&end.lines) {
            // The overlay draws from x 112 in 6 px cells (its own on-screen test).
            assert!(112 + 6 * text.chars().count() <= 640, "{text}");
            for word in ["score", "rating", "grade", "rank"] {
                assert!(
                    !text.to_lowercase().contains(word) || text.starts_with("Facts only"),
                    "{text}"
                );
            }
        }
    }
}
