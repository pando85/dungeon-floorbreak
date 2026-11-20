//! The live flow of the lab (LAB-00-wire): the R00 Setup Cage menu, the exit
//! rule and the room order. Bevy-free, so every rule here is unit-tested
//! headlessly; [`crate::combat_lab_overlay`] only draws it and
//! `floor01_scene` only feeds it the finished simulation state.
//!
//! **One room per launch.** The scene binary builds its kit, camera and art
//! once, so a room change (the Setup Cage confirming, an exit, the end panel)
//! starts the binary again with the next room's arguments
//! ([`relaunch_argv`]). An in-room restart (Select hold, `R`) stays in place
//! and replays the room from its descriptor. The next room is a function of
//! this module ([`next_room`]), never of the adapter.
//!
//! The exit (spec section 6, LAB-R-Q08) opens when the room is clear and fires
//! when the controlled hero (solo) or both heroes (co-op) are inside the exit
//! rect. R00 is "clear" once the menu is confirmed.

use super::descriptor::GoalKind;
use super::runner::PlayerMode;
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::env_kit::IRect;
use crate::party::{HeroId, PartyMode, PartyState};
use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH};

/// The PLAYTRACE session source of a lab run (PLAYTRACE PT-Q12): a lab session
/// is recorded as `combat_lab`, never as `story`, `demo` or `fixture`
/// (`recording_manifest` in `floor01_scene` takes this value, and the overlay's
/// REC badge follows the recorder health).
pub const SESSION_SOURCE: &str = "combat_lab";

/// The rooms the Setup Cage offers as a start room (later rooms are greyed
/// out until authored, spec 3.6).
pub const START_ROOMS: [&str; 2] = ["R01", "R02"];
/// The variants the menu offers (the held-out geometry is CLI only).
pub const MENU_VARIANTS: [&str; 3] = ["v1", "v2", "v3"];
/// The rules the menu lists; `cf02_rules` is greyed out until CF02-F.
pub const RULES_LEGACY: &str = "legacy";
pub const RULES_CF02: &str = "cf02_rules";

/// Everything a relaunch carries: the next room and the choice made in the
/// Setup Cage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabSelection {
    pub room: String,
    pub mode: PlayerMode,
    /// The variant **this** room runs (R00 and R02 have one layout, `v1`).
    pub variant: String,
    /// The room the Setup Cage hands over to when its exit fires.
    pub next: String,
    /// The variant chosen in the Setup Cage for [`Self::next`]; it travels
    /// through the whole room chain and never reaches a room that does not
    /// have it.
    pub next_variant: String,
    /// The menu was confirmed (R00 only): the warm-up and its exit are open.
    pub confirmed: bool,
}

impl Default for LabSelection {
    fn default() -> Self {
        Self {
            room: "R00".into(),
            mode: PlayerMode::SoloCarl,
            variant: "v1".into(),
            next: START_ROOMS[0].into(),
            next_variant: "v1".into(),
            confirmed: false,
        }
    }
}

/// The lab flags of `floor01_scene` (`--lab-room`, `--lab-variant`,
/// `--lab-mode`, `--lab-next`, `--lab-next-variant`, `--lab-confirmed`). Returns `Ok(true)` when
/// `flag` was one of them; `value` is called for the flags that take one.
pub fn apply_flag(
    selection: &mut LabSelection,
    flag: &str,
    mut value: impl FnMut() -> Result<String, String>,
) -> Result<bool, String> {
    match flag {
        "--lab-room" => selection.room = value()?,
        "--lab-variant" => selection.variant = value()?,
        "--lab-mode" => {
            let v = value()?;
            selection.mode = PlayerMode::parse(&v).ok_or_else(|| {
                format!("unknown lab mode {v:?} (expected solo_carl, solo_donut, coop_local)")
            })?;
        }
        "--lab-next" => selection.next = value()?,
        "--lab-next-variant" => selection.next_variant = value()?,
        "--lab-confirmed" => selection.confirmed = true,
        _ => return Ok(false),
    }
    Ok(true)
}

/// Whether `flag` is a lab flag (so the parser can reject it outside the lab).
pub fn is_lab_flag(flag: &str) -> bool {
    matches!(
        flag,
        "--lab-room"
            | "--lab-variant"
            | "--lab-mode"
            | "--lab-next"
            | "--lab-next-variant"
            | "--lab-confirmed"
    )
}

/// The argument list of the next launch: `argv` (program name first) with
/// every lab flag replaced by `selection`'s. Every other argument (the camera,
/// the scale, `--deck`) is kept. `--cf02-seed` is **dropped**: the seed
/// override belongs to the first launch, and the room chain owns the variant
/// from then on (on R01 the seed is the variant, LAB-R-Q06), so a carried
/// seed could contradict the menu's choice.
pub fn relaunch_argv(argv: &[String], selection: &LabSelection) -> Vec<String> {
    let mut out = Vec::with_capacity(argv.len() + 8);
    let mut iter = argv.iter();
    if let Some(program) = iter.next() {
        out.push(program.clone());
    }
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--lab-room" | "--lab-variant" | "--lab-mode" | "--lab-next" | "--lab-next-variant"
            | "--cf02-seed" => {
                iter.next();
            }
            "--lab-confirmed" => {}
            _ => out.push(arg.clone()),
        }
    }
    // A room with one layout takes no variant: only a non-default one is
    // written (it is always the chosen variant of a room that has it).
    let own_variant = (selection.variant != "v1").then_some(selection.variant.as_str());
    out.extend(
        [
            Some(("--lab-room", selection.room.as_str())),
            own_variant.map(|v| ("--lab-variant", v)),
            Some(("--lab-mode", selection.mode.id())),
            Some(("--lab-next", selection.next.as_str())),
            Some(("--lab-next-variant", selection.next_variant.as_str())),
        ]
        .into_iter()
        .flatten()
        .flat_map(|(flag, value)| [flag.to_owned(), value.to_owned()]),
    );
    if selection.confirmed {
        out.push("--lab-confirmed".into());
    }
    out
}

// ------------------------------------------------------------------- the menu

/// A row of the Setup Cage menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuRow {
    Profile,
    Mode,
    Rules,
    Variant,
    StartRoom,
    Start,
}

impl MenuRow {
    pub const ALL: [MenuRow; 6] = [
        Self::Profile,
        Self::Mode,
        Self::Rules,
        Self::Variant,
        Self::StartRoom,
        Self::Start,
    ];

    const fn index(self) -> usize {
        match self {
            Self::Profile => 0,
            Self::Mode => 1,
            Self::Rules => 2,
            Self::Variant => 3,
            Self::StartRoom => 4,
            Self::Start => 5,
        }
    }
}

/// A menu key (the adapter maps the keyboard and the d-pad onto these).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKey {
    Up,
    Down,
    Left,
    Right,
    Confirm,
}

/// What a key press did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuOutcome {
    /// Cursor or value moved (or the key did nothing).
    None,
    /// A greyed-out value was asked for and refused.
    Refused(&'static str),
    /// The Start row was confirmed: the selection is final.
    Confirmed,
}

/// One drawn row: label, value, whether the value can be changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuLine {
    pub label: &'static str,
    pub value: String,
    pub selected: bool,
    /// Read-only (the profile) or greyed out (`cf02_rules`).
    pub dimmed: bool,
}

/// The R00 selection screen: the fixed lists of LAB-R-Q11 and nothing else
/// (no difficulty, no per-ability toggle, no item).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupMenu {
    mode: usize,
    variant: usize,
    start_room: usize,
    row: usize,
    /// The message of the last refused press, shown under the list.
    pub note: Option<&'static str>,
}

impl SetupMenu {
    /// A menu holding the selection of the launch (the cursor on Start).
    pub fn new(selection: &LabSelection) -> Self {
        let mode = PlayerMode::ALL
            .iter()
            .position(|m| *m == selection.mode)
            .unwrap_or(0);
        let variant = MENU_VARIANTS
            .iter()
            .position(|v| *v == selection.next_variant)
            .unwrap_or(0);
        let start_room = START_ROOMS
            .iter()
            .position(|r| *r == selection.next)
            .unwrap_or(0);
        Self {
            mode,
            variant,
            start_room,
            row: MenuRow::Start.index(),
            note: None,
        }
    }

    pub fn row(&self) -> MenuRow {
        MenuRow::ALL[self.row]
    }

    pub fn mode(&self) -> PlayerMode {
        PlayerMode::ALL[self.mode]
    }

    pub fn start_room(&self) -> &'static str {
        START_ROOMS[self.start_room]
    }

    /// The variant that applies to the chosen start room: R01 has three
    /// placements (the effective seed 1 to 3, LAB-R-Q06), R02 one layout.
    pub fn variant(&self) -> &'static str {
        if self.start_room() == "R01" {
            MENU_VARIANTS[self.variant]
        } else {
            "v1"
        }
    }

    /// The selection a confirmation hands to the next launch.
    pub fn selection(&self) -> LabSelection {
        LabSelection {
            room: "R00".into(),
            mode: self.mode(),
            variant: "v1".into(),
            next: self.start_room().into(),
            next_variant: self.variant().into(),
            confirmed: true,
        }
    }

    pub fn press(&mut self, key: MenuKey) -> MenuOutcome {
        self.note = None;
        let rows = MenuRow::ALL.len();
        match key {
            MenuKey::Up => self.row = (self.row + rows - 1) % rows,
            MenuKey::Down => self.row = (self.row + 1) % rows,
            MenuKey::Left | MenuKey::Right => {
                let forward = key == MenuKey::Right;
                let step = |i: usize, n: usize| {
                    if forward {
                        (i + 1) % n
                    } else {
                        (i + n - 1) % n
                    }
                };
                match self.row() {
                    MenuRow::Mode => self.mode = step(self.mode, PlayerMode::ALL.len()),
                    MenuRow::Variant => self.variant = step(self.variant, MENU_VARIANTS.len()),
                    MenuRow::StartRoom => {
                        self.start_room = step(self.start_room, START_ROOMS.len());
                    }
                    MenuRow::Rules => {
                        let note = "cf02_rules is greyed out until CF02-F";
                        self.note = Some(note);
                        return MenuOutcome::Refused(note);
                    }
                    MenuRow::Profile | MenuRow::Start => {}
                }
            }
            MenuKey::Confirm => {
                if self.row() == MenuRow::Start {
                    return MenuOutcome::Confirmed;
                }
                // Confirm on any other row moves to the Start row.
                self.row = MenuRow::Start.index();
            }
        }
        MenuOutcome::None
    }

    /// The rows as the overlay draws them.
    pub fn lines(&self) -> Vec<MenuLine> {
        let variant = if self.start_room() == "R01" {
            format!("{} (seed {})", self.variant(), self.variant + 1)
        } else {
            "v1 (one layout)".to_owned()
        };
        MenuRow::ALL
            .iter()
            .map(|row| {
                let (label, value, dimmed) = match row {
                    MenuRow::Profile => ("Profile", super::PROFILE_ID.to_owned(), true),
                    MenuRow::Mode => ("Mode", self.mode().id().to_owned(), false),
                    MenuRow::Rules => {
                        ("Rules", format!("{RULES_LEGACY}  ({RULES_CF02} off)"), true)
                    }
                    MenuRow::Variant => ("Variant", variant.clone(), false),
                    MenuRow::StartRoom => ("Start room", self.start_room().to_owned(), false),
                    MenuRow::Start => ("", "START".to_owned(), false),
                };
                MenuLine {
                    label,
                    value,
                    selected: *row == self.row(),
                    dimmed,
                }
            })
            .collect()
    }
}

// ------------------------------------------------------------------ the exit

/// Whether the room's exit is open, and why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitState {
    /// The menu is open (R00) or enemies remain.
    Closed,
    Open,
}

/// The exit is open when the goal is met: R00 once its menu is confirmed, a
/// `clear_enemies` room once every goblin and llama is defeated, a
/// `reach_exit` room always.
pub fn exit_state(goal: GoalKind, menu_confirmed: bool, combat: &CombatState) -> ExitState {
    let open = match goal {
        GoalKind::None => menu_confirmed,
        GoalKind::ClearEnemies => crate::demo::complete(combat),
        GoalKind::ReachExit => true,
        GoalKind::Link => false,
    };
    if open {
        ExitState::Open
    } else {
        ExitState::Closed
    }
}

/// Enemies still alive (the strip shows it while the exit is closed).
pub fn enemies_left(combat: &CombatState) -> usize {
    combat.enemies.iter().filter(|g| g.alive()).count()
        + combat.llamas.iter().filter(|l| l.alive()).count()
}

/// Whether the hero's body overlaps `exit` (layout pixels, y down).
fn hero_in_exit(scene: &SceneSim, hero: HeroId, exit: IRect) -> bool {
    let body = scene.world.body(hero);
    let height = scene.world.layout_height() as f32;
    // The body is y-up with its centre at (x, y); the exit is y-down.
    let left = body.x - HERO_HALF_WIDTH;
    let right = body.x + HERO_HALF_WIDTH;
    let top = height - (body.y + HERO_HALF_HEIGHT);
    let bottom = height - (body.y - HERO_HALF_HEIGHT);
    left < exit.right() as f32
        && right > exit.x as f32
        && top < exit.bottom() as f32
        && bottom > exit.y as f32
}

/// The exit rule (LAB-R-Q08): the controlled hero in solo, both heroes in
/// two-player, standing in `exit`.
pub fn heroes_in_exit(scene: &SceneSim, party: &PartyState, exit: IRect) -> bool {
    match party.mode() {
        PartyMode::Solo => hero_in_exit(scene, party.controlled_hero(), exit),
        PartyMode::TwoPlayer => HeroId::ALL
            .into_iter()
            .all(|hero| hero_in_exit(scene, hero, exit)),
    }
}

// ------------------------------------------------------------- the room order

/// Where an exit leads (spec section 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NextRoom {
    /// Relaunch into this selection.
    Room(LabSelection),
    /// The end of the LAB-01 set: the factual panel, then the Setup Cage.
    End,
}

/// The room after `current`: R00 hands over to the chosen start room, R01 to
/// R02, R02 ends the set (LAB-Q10). Later rooms are not part of the live set
/// yet and also end it.
pub fn next_room(current: &LabSelection) -> NextRoom {
    let go = |room: &str, variant: &str| {
        NextRoom::Room(LabSelection {
            room: room.into(),
            variant: variant.into(),
            confirmed: false,
            ..current.clone()
        })
    };
    match current.room.as_str() {
        "R00" => go(&current.next, &current.next_variant),
        "R01" => go("R02", "v1"),
        _ => NextRoom::End,
    }
}

/// The selection that returns to the Setup Cage (menu open) after the end
/// panel, keeping the last choice as the menu's starting values.
pub fn back_to_cage(current: &LabSelection) -> LabSelection {
    LabSelection {
        room: "R00".into(),
        variant: "v1".into(),
        confirmed: false,
        ..current.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(room: &str) -> LabSelection {
        LabSelection {
            room: room.into(),
            ..LabSelection::default()
        }
    }

    #[test]
    fn the_menu_cycles_fixed_lists_and_refuses_cf02_rules() {
        let mut menu = SetupMenu::new(&LabSelection::default());
        assert_eq!(menu.row(), MenuRow::Start);
        // Up to the Start room row and change it.
        assert_eq!(menu.press(MenuKey::Up), MenuOutcome::None);
        assert_eq!(menu.row(), MenuRow::StartRoom);
        menu.press(MenuKey::Right);
        assert_eq!(menu.start_room(), "R02");
        // R02 has one layout: the variant is v1 whatever was picked.
        assert_eq!(menu.variant(), "v1");
        menu.press(MenuKey::Left);
        assert_eq!(menu.start_room(), "R01");
        // Variant row cycles v1 -> v2 -> v3 -> v1.
        menu.press(MenuKey::Up);
        assert_eq!(menu.row(), MenuRow::Variant);
        menu.press(MenuKey::Right);
        assert_eq!(menu.variant(), "v2");
        menu.press(MenuKey::Right);
        menu.press(MenuKey::Right);
        assert_eq!(menu.variant(), "v1");
        // Rules row: cf02_rules is refused.
        menu.press(MenuKey::Up);
        assert_eq!(menu.row(), MenuRow::Rules);
        assert!(matches!(
            menu.press(MenuKey::Right),
            MenuOutcome::Refused(_)
        ));
        assert!(menu.note.is_some());
        // Mode row: all three modes, wrapping.
        menu.press(MenuKey::Up);
        assert_eq!(menu.row(), MenuRow::Mode);
        for expected in ["solo_donut", "coop_local", "solo_carl"] {
            menu.press(MenuKey::Right);
            assert_eq!(menu.mode().id(), expected);
        }
        // The profile row is read-only.
        menu.press(MenuKey::Up);
        assert_eq!(menu.row(), MenuRow::Profile);
        menu.press(MenuKey::Right);
        assert_eq!(menu.mode(), PlayerMode::SoloCarl);
    }

    #[test]
    fn confirm_walks_to_the_start_row_then_confirms_the_selection() {
        let mut menu = SetupMenu::new(&LabSelection::default());
        menu.press(MenuKey::Up);
        menu.press(MenuKey::Up);
        menu.press(MenuKey::Up);
        assert_eq!(menu.row(), MenuRow::Rules);
        // Confirm on a value row only moves to Start.
        assert_eq!(menu.press(MenuKey::Confirm), MenuOutcome::None);
        assert_eq!(menu.row(), MenuRow::Start);
        menu.press(MenuKey::Up);
        menu.press(MenuKey::Up);
        menu.press(MenuKey::Up);
        menu.press(MenuKey::Up);
        assert_eq!(menu.row(), MenuRow::Mode);
        menu.press(MenuKey::Right);
        menu.press(MenuKey::Right);
        menu.press(MenuKey::Confirm);
        assert_eq!(menu.press(MenuKey::Confirm), MenuOutcome::Confirmed);
        let picked = menu.selection();
        assert_eq!(picked.mode, PlayerMode::CoopLocal);
        assert_eq!(
            (picked.room.as_str(), picked.next.as_str(), picked.confirmed),
            ("R00", "R01", true)
        );
    }

    #[test]
    fn the_menu_has_no_difficulty_or_toggle_rows() {
        let labels: Vec<&str> = SetupMenu::new(&LabSelection::default())
            .lines()
            .iter()
            .map(|l| l.label)
            .collect();
        assert_eq!(
            labels,
            ["Profile", "Mode", "Rules", "Variant", "Start room", ""]
        );
        let lines = SetupMenu::new(&LabSelection::default()).lines();
        assert!(lines[0].dimmed && lines[2].dimmed);
        assert!(lines[0].value.contains("postbiscuit"));
        assert!(lines[2].value.contains("cf02_rules off"));
    }

    #[test]
    fn the_room_order_is_cage_start_room_r02_end() {
        let mut cage = s("R00");
        cage.next = "R02".into();
        cage.mode = PlayerMode::CoopLocal;
        cage.confirmed = true;
        let NextRoom::Room(r02) = next_room(&cage) else {
            panic!("R00 hands over to R02");
        };
        assert_eq!(
            (r02.room.as_str(), r02.mode),
            ("R02", PlayerMode::CoopLocal)
        );
        assert!(!r02.confirmed);
        assert_eq!(next_room(&r02), NextRoom::End);
        let NextRoom::Room(r02b) = next_room(&s("R01")) else {
            panic!("R01 hands over to R02");
        };
        assert_eq!((r02b.room.as_str(), r02b.variant.as_str()), ("R02", "v1"));
        let again = back_to_cage(&r02);
        assert_eq!((again.room.as_str(), again.confirmed), ("R00", false));
    }

    #[test]
    fn relaunch_argv_replaces_lab_flags_and_keeps_the_rest() {
        let argv: Vec<String> = [
            "floor01_scene",
            "--scene",
            "combat_lab",
            "--lab-room",
            "R00",
            "--lab-mode",
            "solo_carl",
            "--deck",
            "--lab-confirmed",
            "--cf02-seed",
            "3",
            "--scale",
            "3",
        ]
        .iter()
        .map(|a| (*a).to_owned())
        .collect();
        let selection = LabSelection {
            room: "R01".into(),
            mode: PlayerMode::SoloDonut,
            variant: "v3".into(),
            next: "R02".into(),
            next_variant: "v2".into(),
            confirmed: false,
        };
        let out = relaunch_argv(&argv, &selection);
        assert_eq!(
            out,
            [
                "floor01_scene",
                "--scene",
                "combat_lab",
                "--deck",
                "--scale",
                "3",
                "--lab-room",
                "R01",
                "--lab-variant",
                "v3",
                "--lab-mode",
                "solo_donut",
                "--lab-next",
                "R02",
                "--lab-next-variant",
                "v2",
            ]
        );
        // Round trip through the flag parser.
        let mut parsed = LabSelection::default();
        let mut iter = out.iter().skip(1);
        while let Some(arg) = iter.next() {
            let _ = apply_flag(&mut parsed, arg, || {
                iter.next().cloned().ok_or_else(|| "missing".to_owned())
            })
            .unwrap();
        }
        assert_eq!(parsed, selection);
    }

    #[test]
    fn a_lab_session_is_recorded_as_combat_lab() {
        assert_eq!(
            SESSION_SOURCE,
            super::super::entry::EntryPoint::CombatLab.name()
        );
    }

    #[test]
    fn an_unknown_lab_mode_is_rejected() {
        let mut sel = LabSelection::default();
        let e = apply_flag(&mut sel, "--lab-mode", || Ok("duo".into())).unwrap_err();
        assert!(e.contains("solo_carl"));
        assert!(!apply_flag(&mut sel, "--demo", || Ok(String::new())).unwrap());
    }
}
