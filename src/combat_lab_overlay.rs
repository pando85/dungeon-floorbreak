//! COMBAT-LAB-01 LAB-00-wire: the lab's own on-screen overlay: the control
//! strip, the REC badge, the R00 Setup Cage menu and the end panel.
//!
//! The overlay is **presentation only**. It never touches the simulation and
//! never reads it: `floor01_scene` feeds a [`LabOverlay`] resource once per
//! fixed tick from the finished state and executes the [`LabRequest`] the
//! overlay raises (a menu confirmation, an exit). The rules (the menu lists,
//! the exit rule, the room order) live in [`crate::combat_lab::flow`].
//!
//! It draws on the HUD's render layer through its own pooled `Text2d` lines
//! and rects and **never touches `src/hud.rs` or `src/hud_draw.rs`**: the
//! strip sits in the bottom 26 px of the view (masonry), the menu panel in the
//! middle, both clear of the HUD's top 60 px.
//!
//! F01-RUN reuses it for the floor run of `--scene f01_late --f01-run`
//! ([`LabOverlay::floor_run`]): no strip, only the room-clear and floor-end
//! panels, whose confirmation raises [`LabRequest::Continue`] (the rules are in
//! [`crate::f01_run`]).
//!
//! The REC badge shows the recorder state (PTI-09): `REC` while recording,
//! `REC ERROR: <reason>` when the recorder failed or is losing writes, and
//! `REC off (<reason>)` when it is off (`--no-record`, the environment, a
//! scripted run).

use bevy::{
    camera::visibility::RenderLayers,
    prelude::*,
    sprite::Anchor,
    text::{FontHinting, FontSmoothing, LineHeight},
};

use crate::combat_lab::flow::{LabSelection, MenuKey, MenuOutcome, SetupMenu, back_to_cage};
use crate::hud::VIEW_WIDTH;
use crate::hud_adapter::LAYER_HUD;

/// Silkscreen is the HUD's small-text font; the binary reads its bytes.
pub const FONT_PATH: &str = "assets/fonts/silkscreen/Silkscreen-Regular.ttf";

const FONT_PX: f32 = 8.0;
const LINE_PX: f32 = 10.0;
const POOL_TEXTS: usize = 30;
const POOL_RECTS: usize = 4;
const STRIP_H: i32 = 26;
/// The HUD owns the top of the view; the menu panel stays below it.
const HUD_BAND_PX: i32 = 60;
const Z_RECT: f32 = 50.0;
const Z_TEXT: f32 = 51.0;

/// The recorder state the badge shows (PTI-09).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecStatus {
    /// Not recording, with the reason ("--no-record", "scripted run").
    Off(String),
    /// A session is being recorded.
    Recording,
    /// The recorder failed: the reason is shown.
    Error(String),
}

impl RecStatus {
    /// The state before the first tick reported one.
    pub fn starting() -> Self {
        Self::Off("starting".into())
    }

    pub fn badge(&self) -> (String, Tone) {
        match self {
            Self::Off(why) => (format!("REC off ({why})"), Tone::Dim),
            Self::Recording => ("REC".to_owned(), Tone::Red),
            Self::Error(why) => (format!("REC ERROR: {why}"), Tone::Amber),
        }
    }
}

/// A transition the binary executes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabRequest {
    /// Start the scene again with this selection (one room per launch).
    Relaunch(LabSelection),
    /// F01-RUN: the end panel was confirmed; the binary goes on (the next room
    /// of the floor run, or the end of it).
    Continue,
}

/// What confirming the end panel does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndAction {
    /// The lab: back to the Setup Cage.
    BackToCage,
    /// The floor run: [`LabRequest::Continue`].
    Continue,
}

/// What the end panel (R02 cleared and exited) shows: facts only (LAB-Q10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndPanel {
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Normal,
    Dim,
    Amber,
    Red,
    Green,
}

impl Tone {
    fn color(self) -> Color {
        match self {
            Self::Normal => Color::srgb_u8(230, 226, 214),
            Self::Dim => Color::srgb_u8(138, 134, 126),
            Self::Amber => Color::srgb_u8(240, 182, 64),
            Self::Red => Color::srgb_u8(224, 72, 64),
            Self::Green => Color::srgb_u8(120, 200, 110),
        }
    }
}

/// One text line to draw, in view pixels (top-left origin).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawLine {
    pub x: i32,
    pub y: i32,
    pub text: String,
    pub tone: Tone,
}

/// One filled rectangle to draw, in view pixels (top-left origin).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub alpha: u8,
}

/// The overlay state. The binary writes the `strip_*` fields and `rec` every
/// tick; the menu and the end panel are driven by the overlay's own input.
#[derive(Resource, Debug, Clone)]
pub struct LabOverlay {
    pub view_h: i32,
    /// What this launch is (`LAB R01 v1 | solo_carl | legacy | seed 1`).
    pub title: String,
    /// The exit and party state (`EXIT closed: 3 left | P2: Start on pad 2`).
    pub status: String,
    pub rec: RecStatus,
    pub menu: Option<SetupMenu>,
    pub end: Option<EndPanel>,
    pub selection: LabSelection,
    /// Raised by the overlay's input; the binary takes it.
    pub request: Option<LabRequest>,
    /// Whether the bottom strip (title, status, REC badge) is drawn: the lab's,
    /// not the floor run's (its HUD owns the screen).
    pub strip: bool,
    /// The end panel's heading and confirm hint.
    pub end_heading: String,
    pub end_hint: String,
    pub end_action: EndAction,
    /// F01-RUN: frames left during which the end panel ignores Confirm (the
    /// floor run arms it when the panel opens, so a jump pressed through the exit
    /// does not skip the facts). The lab leaves it at 0.
    pub end_lock_frames: u16,
}

impl LabOverlay {
    pub fn new(view_h: i32, selection: LabSelection, menu_open: bool) -> Self {
        let menu = menu_open.then(|| SetupMenu::new(&selection));
        Self {
            view_h,
            title: String::new(),
            status: String::new(),
            rec: RecStatus::starting(),
            menu,
            end: None,
            selection,
            request: None,
            strip: true,
            end_heading: "LAB-01 R00-R02 SET FINISHED".to_owned(),
            end_hint: "Enter, A or Start: back to the Setup Cage".to_owned(),
            end_action: EndAction::BackToCage,
            end_lock_frames: 0,
        }
    }

    /// F01-RUN: the floor run's overlay: no strip, no menu, the end panel only
    /// once a room is cleared (its heading and hint are set with it).
    pub fn floor_run(view_h: i32) -> Self {
        Self {
            strip: false,
            end_heading: String::new(),
            end_hint: String::new(),
            end_action: EndAction::Continue,
            ..Self::new(view_h, LabSelection::default(), false)
        }
    }

    /// Gameplay input is ignored while a panel is up (spec 3.6).
    pub fn blocks_gameplay(&self) -> bool {
        self.menu.is_some() || self.end.is_some()
    }

    /// One key of the menu or the end panel.
    pub fn press(&mut self, key: MenuKey) {
        if let Some(menu) = self.menu.as_mut() {
            if menu.press(key) == MenuOutcome::Confirmed {
                let picked = menu.selection();
                self.request = Some(LabRequest::Relaunch(picked));
            }
        } else if self.end.is_some() && key == MenuKey::Confirm && self.end_lock_frames == 0 {
            self.request = Some(match self.end_action {
                EndAction::BackToCage => LabRequest::Relaunch(back_to_cage(&self.selection)),
                EndAction::Continue => LabRequest::Continue,
            });
        }
    }

    /// The P1 pad's Start button. While a panel is open it is Confirm and
    /// returns `true` (the caller must not quit); otherwise it returns `false`
    /// and the normal Start behaviour applies.
    pub fn pad_start(&mut self) -> bool {
        if !self.blocks_gameplay() {
            return false;
        }
        self.press(MenuKey::Confirm);
        true
    }

    /// Everything to draw this frame.
    pub fn layout(&self) -> (Vec<DrawRect>, Vec<DrawLine>) {
        let mut rects = Vec::new();
        let mut lines = Vec::new();
        let top = self.view_h - STRIP_H;
        if self.strip {
            rects.push(DrawRect {
                x: 0,
                y: top,
                w: VIEW_WIDTH,
                h: STRIP_H,
                alpha: 200,
            });
        }
        let mut line = |x: i32, y: i32, text: &str, tone: Tone| {
            lines.push(DrawLine {
                x,
                y,
                text: text.to_owned(),
                tone,
            });
        };
        if self.strip {
            line(6, top + 3, &self.title, Tone::Normal);
            line(6, top + 14, &self.status, Tone::Normal);
            let (badge, tone) = self.rec.badge();
            // Right-aligned by the 5 px advance of the 8 px font, clear of the
            // HUD's corner emblem (the last 36 px).
            let x = (VIEW_WIDTH - 36 - 5 * badge.chars().count() as i32).max(0);
            line(x, top + 3, &badge, tone);
        }
        if let Some(menu) = &self.menu {
            panel(&mut rects, 96, HUD_BAND_PX + 8, 448, 150);
            let mut y = HUD_BAND_PX + 16;
            line(
                112,
                y,
                "THE SETUP CAGE - COMBAT LAB (dev only)",
                Tone::Amber,
            );
            y += 16;
            for row in menu.lines() {
                let tone = if row.selected {
                    Tone::Amber
                } else if row.dimmed {
                    Tone::Dim
                } else {
                    Tone::Normal
                };
                let marker = if row.selected { ">" } else { " " };
                // Proportional glyphs: the label and the value get their own
                // columns instead of padding with spaces.
                line(112, y, marker, tone);
                if row.label.is_empty() {
                    line(128, y, &row.value, tone);
                } else {
                    line(128, y, row.label, tone);
                    line(208, y, &row.value, tone);
                }
                y += 12;
            }
            if let Some(note) = menu.note {
                line(112, y + 2, note, Tone::Red);
            }
            line(
                112,
                HUD_BAND_PX + 8 + 150 - 14,
                "W/S or d-pad: row   A/D or d-pad: change   Enter, A or Start: confirm",
                Tone::Dim,
            );
        }
        if let Some(end) = &self.end {
            let h = 40 + 12 * end.lines.len() as i32;
            panel(&mut rects, 96, HUD_BAND_PX + 8, 448, h);
            line(112, HUD_BAND_PX + 16, &self.end_heading, Tone::Green);
            for (i, text) in end.lines.iter().enumerate() {
                line(112, HUD_BAND_PX + 32 + 12 * i as i32, text, Tone::Normal);
            }
            line(112, HUD_BAND_PX + 8 + h - 14, &self.end_hint, Tone::Dim);
        }
        (rects, lines)
    }
}

fn panel(rects: &mut Vec<DrawRect>, x: i32, y: i32, w: i32, h: i32) {
    rects.push(DrawRect {
        x,
        y,
        w,
        h,
        alpha: 225,
    });
}

// -------------------------------------------------------------------- Bevy

/// The plugin: the font bytes are read by the binary (like the HUD's).
pub struct LabOverlayPlugin {
    pub font: Vec<u8>,
    pub overlay: LabOverlay,
}

#[derive(Resource)]
struct OverlayFont(Option<Vec<u8>>);

#[derive(Resource, Default)]
struct OverlayPool {
    texts: Vec<Entity>,
    rects: Vec<Entity>,
}

#[derive(Component)]
struct LabText;

#[derive(Component)]
struct LabRect;

impl Plugin for LabOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.overlay.clone())
            .insert_resource(OverlayFont(Some(self.font.clone())))
            .init_resource::<OverlayPool>()
            .add_systems(Startup, spawn_overlay)
            .add_systems(Update, (overlay_input, draw_overlay).chain());
    }
}

fn spawn_overlay(
    mut commands: Commands,
    mut fonts: ResMut<Assets<Font>>,
    mut font_bytes: ResMut<OverlayFont>,
    mut pool: ResMut<OverlayPool>,
) {
    let layers = RenderLayers::layer(LAYER_HUD);
    let bytes = font_bytes.0.take().unwrap_or_default();
    let handle = fonts.add(Font::from_bytes(bytes));
    for _ in 0..POOL_RECTS {
        pool.rects.push(
            commands
                .spawn((
                    LabRect,
                    Sprite::default(),
                    Anchor::TOP_LEFT,
                    Transform::default(),
                    Visibility::Hidden,
                    layers.clone(),
                ))
                .id(),
        );
    }
    for _ in 0..POOL_TEXTS {
        pool.texts.push(
            commands
                .spawn((
                    LabText,
                    Text2d::new(""),
                    TextFont {
                        font: handle.clone().into(),
                        font_size: FontSize::Px(FONT_PX),
                        font_smoothing: FontSmoothing::None,
                        ..default()
                    },
                    LineHeight::Px(LINE_PX),
                    FontHinting::Enabled,
                    TextColor(Color::WHITE),
                    Anchor::TOP_LEFT,
                    Transform::default(),
                    Visibility::Hidden,
                    layers.clone(),
                ))
                .id(),
        );
    }
}

/// The keyboard and the d-pad drive the menu and the end panel. Edges only.
fn overlay_input(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut overlay: ResMut<LabOverlay>,
) {
    if !overlay.blocks_gameplay() {
        return;
    }
    if overlay.end_lock_frames > 0 {
        overlay.end_lock_frames -= 1;
    }
    let mut pressed = |key: MenuKey| overlay.press(key);
    let map = [
        (
            MenuKey::Up,
            KeyCode::KeyW,
            KeyCode::ArrowUp,
            GamepadButton::DPadUp,
        ),
        (
            MenuKey::Down,
            KeyCode::KeyS,
            KeyCode::ArrowDown,
            GamepadButton::DPadDown,
        ),
        (
            MenuKey::Left,
            KeyCode::KeyA,
            KeyCode::ArrowLeft,
            GamepadButton::DPadLeft,
        ),
        (
            MenuKey::Right,
            KeyCode::KeyD,
            KeyCode::ArrowRight,
            GamepadButton::DPadRight,
        ),
    ];
    for (key, k1, k2, button) in map {
        if keys.just_pressed(k1)
            || keys.just_pressed(k2)
            || gamepads.iter().any(|g| g.just_pressed(button))
        {
            pressed(key);
        }
    }
    if keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::NumpadEnter)
        || gamepads
            .iter()
            .any(|g| g.just_pressed(GamepadButton::South))
    {
        pressed(MenuKey::Confirm);
    }
}

type TextQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Text2d,
        &'static mut TextColor,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (With<LabText>, Without<LabRect>),
>;
type RectQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Sprite,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (With<LabRect>, Without<LabText>),
>;

fn draw_overlay(
    overlay: Res<LabOverlay>,
    pool: Res<OverlayPool>,
    mut texts: TextQuery,
    mut rects: RectQuery,
) {
    if !overlay.is_changed() {
        return;
    }
    let (draw_rects, draw_lines) = overlay.layout();
    let to_world = |x: i32, y: i32, z: f32| Vec3::new(x as f32, (overlay.view_h - y) as f32, z);
    for (i, &entity) in pool.rects.iter().enumerate() {
        let Ok((mut sprite, mut tr, mut vis)) = rects.get_mut(entity) else {
            continue;
        };
        match draw_rects.get(i) {
            Some(r) => {
                sprite.color = Color::srgba_u8(8, 7, 14, r.alpha);
                sprite.custom_size = Some(Vec2::new(r.w as f32, r.h as f32));
                tr.translation = to_world(r.x, r.y, Z_RECT);
                vis.set_if_neq(Visibility::Inherited);
            }
            None => {
                vis.set_if_neq(Visibility::Hidden);
            }
        }
    }
    for (i, &entity) in pool.texts.iter().enumerate() {
        let Ok((mut text, mut color, mut tr, mut vis)) = texts.get_mut(entity) else {
            continue;
        };
        match draw_lines.get(i) {
            Some(l) => {
                if text.0 != l.text {
                    text.0.clone_from(&l.text);
                }
                color.set_if_neq(TextColor(l.tone.color()));
                tr.translation = to_world(l.x, l.y, Z_TEXT);
                vis.set_if_neq(Visibility::Inherited);
            }
            None => {
                vis.set_if_neq(Visibility::Hidden);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_lab::PlayerMode;

    fn overlay(menu: bool) -> LabOverlay {
        let mut o = LabOverlay::new(360, LabSelection::default(), menu);
        o.title = "LAB R00 v1 | solo_carl | legacy | seed 1".into();
        o.status = "EXIT closed: confirm the menu".into();
        o
    }

    #[test]
    fn the_badge_reads_pending_recording_and_error() {
        assert_eq!(RecStatus::starting().badge().0, "REC off (starting)");
        assert_eq!(
            RecStatus::Off("--no-record".into()).badge().0,
            "REC off (--no-record)"
        );
        assert_eq!(RecStatus::Recording.badge().0, "REC");
        let (text, tone) = RecStatus::Error("disk full".into()).badge();
        assert_eq!((text.as_str(), tone), ("REC ERROR: disk full", Tone::Amber));
    }

    #[test]
    fn everything_stays_on_screen_and_the_panels_clear_the_hud_band() {
        for view_h in [360, 400] {
            let mut o = overlay(true);
            o.view_h = view_h;
            o.end = Some(EndPanel {
                lines: vec!["R02: 1175 ticks, 1 deaths".into(); 4],
            });
            let (rects, lines) = o.layout();
            for r in &rects {
                assert!(r.x >= 0 && r.x + r.w <= VIEW_WIDTH && r.y + r.h <= view_h);
            }
            for l in &lines {
                let width = 6 * l.text.chars().count() as i32;
                assert!(
                    l.x >= 0 && l.x + width <= VIEW_WIDTH && l.y + 8 <= view_h,
                    "{l:?}"
                );
                // Everything but the strip sits below the HUD's top band.
                if l.y < view_h - STRIP_H {
                    assert!(l.y >= HUD_BAND_PX, "{l:?}");
                }
            }
            assert!(lines.len() <= POOL_TEXTS && rects.len() <= POOL_RECTS);
        }
    }

    #[test]
    fn the_menu_blocks_gameplay_until_it_confirms_and_raises_a_relaunch() {
        let mut o = overlay(true);
        assert!(o.blocks_gameplay());
        o.press(MenuKey::Up);
        o.press(MenuKey::Up);
        o.press(MenuKey::Up);
        o.press(MenuKey::Up);
        o.press(MenuKey::Right);
        o.press(MenuKey::Confirm);
        assert!(o.request.is_none(), "a value row only moves to Start");
        o.press(MenuKey::Confirm);
        let Some(LabRequest::Relaunch(sel)) = o.request.take() else {
            panic!("a confirmation raises a relaunch");
        };
        assert_eq!(sel.mode, PlayerMode::SoloDonut);
        assert!(sel.confirmed && sel.room == "R00");
    }

    #[test]
    fn a_running_room_has_no_menu_and_the_end_panel_returns_to_the_cage() {
        let mut o = overlay(false);
        assert!(!o.blocks_gameplay());
        o.end = Some(EndPanel {
            lines: vec!["R02 cleared".into()],
        });
        assert!(o.blocks_gameplay());
        o.press(MenuKey::Down);
        assert!(o.request.is_none());
        o.press(MenuKey::Confirm);
        let Some(LabRequest::Relaunch(sel)) = o.request else {
            panic!("the end panel relaunches");
        };
        assert_eq!((sel.room.as_str(), sel.confirmed), ("R00", false));
    }

    #[test]
    fn pad_start_confirms_in_a_panel_and_never_quits_there() {
        // Menu open: Start on the Start row confirms (a relaunch request).
        let mut o = overlay(true);
        assert!(o.pad_start());
        assert!(matches!(o.request.take(), Some(LabRequest::Relaunch(_))));
        // End panel open: Start goes back to the Setup Cage.
        let mut o = overlay(false);
        o.end = Some(EndPanel { lines: vec![] });
        assert!(o.pad_start());
        assert!(matches!(o.request.take(), Some(LabRequest::Relaunch(_))));
        // No panel: Start keeps its normal meaning (the caller quits).
        let mut o = overlay(false);
        assert!(!o.pad_start());
        assert!(o.request.is_none());
    }

    #[test]
    fn the_floor_run_overlay_draws_only_its_panel_and_confirms_with_continue() {
        let mut o = LabOverlay::floor_run(360);
        assert!(!o.blocks_gameplay());
        let (rects, lines) = o.layout();
        assert!(rects.is_empty() && lines.is_empty(), "no strip");
        o.end_heading = "ROOM 1/5 CLEARED: THE HOARDER".into();
        o.end_hint = "Enter, A or Start: next room (Goblin Boiler)".into();
        o.end = Some(EndPanel {
            lines: vec!["2400 ticks (0:40), deaths 0, restarts 0".into()],
        });
        assert!(o.blocks_gameplay());
        let (rects, lines) = o.layout();
        assert_eq!(rects.len(), 1, "the panel only");
        assert_eq!(lines[0].text, "ROOM 1/5 CLEARED: THE HOARDER");
        assert!(
            lines
                .last()
                .unwrap()
                .text
                .starts_with("Enter, A or Start: next room")
        );
        o.press(MenuKey::Down);
        assert!(o.request.is_none());
        // Armed: a confirm while the lock runs is swallowed (Start still never quits).
        o.end_lock_frames = 30;
        assert!(o.pad_start());
        o.press(MenuKey::Confirm);
        assert!(o.request.is_none(), "locked");
        o.end_lock_frames = 0;
        assert!(o.pad_start());
        assert_eq!(o.request.take(), Some(LabRequest::Continue));
        // The lab keeps its own heading and hint.
        let lab = overlay(false);
        assert_eq!(lab.end_heading, "LAB-01 R00-R02 SET FINISHED");
        assert_eq!(lab.end_action, EndAction::BackToCage);
    }

    #[test]
    fn the_strip_shows_the_rec_state_and_controls_text() {
        let mut o = overlay(false);
        o.rec = RecStatus::Recording;
        let (_, lines) = o.layout();
        assert!(lines.iter().any(|l| l.text == "REC" && l.tone == Tone::Red));
        assert!(lines.iter().any(|l| l.text.starts_with("LAB R00")));
    }
}
