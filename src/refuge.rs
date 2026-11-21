//! AREA-REFUGE-01: Mordecai's Tutorial Guild room (`--scene refuge_guild`).
//!
//! The room is its own scene. It is not in the `f01_late` playtest manifest, it
//! is not recorded and it adds no gameplay: the layout
//! (`assets/levels/floor01/refuge_guild.json`, `env-kit-layout-v2`) draws it and
//! the room file (`refuge_guild.room.json`) names the cast marks, the door and
//! its exit trigger. Pure and headless: no Bevy types.
//!
//! Mordecai's mark is data with an off switch: `enabled: false` and no `sprite`
//! until MORDECAI-01's idle key exists. The loader refuses a mark that is
//! enabled without a sprite, so the day his key lands is a data edit plus the
//! sprite hook, never a silent placeholder.

use std::path::Path;

use serde::Deserialize;

use crate::env_kit::{EnvError, EnvKit};

pub const SCENE_NAME: &str = "refuge_guild";
pub const ROOM_PATH: &str = "assets/levels/floor01/refuge_guild.room.json";
pub const ROOM_SCHEMA: &str = "floor01-refuge-room-v1";

/// A cast mark: the pivot column on the walking line and the facing.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Mark {
    pub x: i32,
    pub facing: Facing,
    /// Only Mordecai carries the switch; Carl and Donut are always placed.
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    /// The manifest of the sprite to draw; required when the mark is enabled.
    #[serde(default)]
    pub sprite: Option<String>,
    #[serde(default)]
    pub height_px: Option<i32>,
}

fn enabled_by_default() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Facing {
    Left,
    Right,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Marks {
    pub donut: Mark,
    pub carl: Mark,
    pub mordecai: Mark,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExitTrigger {
    pub id: String,
    /// `[x, y, w, h]` in view pixels (layout y = view y + 20).
    pub rect_view: [i32; 4],
    pub opens: String,
    pub returns_to: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Door {
    pub states: std::collections::BTreeMap<String, String>,
    pub default: String,
    pub exit_trigger: ExitTrigger,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Room {
    pub schema: String,
    pub id: String,
    pub layout: String,
    pub walk_line_view_y: i32,
    pub marks: Marks,
    pub clearances_view_x: Clearances,
    pub door: Door,
}

/// Horizontal ranges (view x) the cast must keep clear of, and the floor span
/// that stays free of props on the walk line.
#[derive(Debug, Clone, Deserialize)]
pub struct Clearances {
    pub hearth_opening: [i32; 2],
    pub desk: [i32; 2],
    pub free_floor: [i32; 2],
}

/// Layout y of the top of the 640 x 360 view (layout y = view y + `VIEW_TOP`).
pub const VIEW_TOP: i32 = 20;
/// Half the width of a hero's body box (`prototype::HERO_HALF_WIDTH`); Mordecai
/// is drawn with the same body width until his own box exists.
const BODY_HALF: i32 = crate::prototype::HERO_HALF_WIDTH as i32;

/// Where Carl and Donut stand in the room, for the scene's reference pose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CastX {
    pub carl: f32,
    pub donut: f32,
}

impl Room {
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join(ROOM_PATH);
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let room: Room =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        room.check_shape()?;
        Ok(room)
    }

    /// Checks that need no kit.
    fn check_shape(&self) -> Result<(), String> {
        if self.schema != ROOM_SCHEMA {
            return Err(format!("room schema {:?}, expected {ROOM_SCHEMA}", self.schema));
        }
        let m = &self.marks;
        if m.mordecai.enabled && m.mordecai.sprite.is_none() {
            return Err(
                "the mordecai mark is enabled but names no sprite: set sprite (his MORDECAI-01 manifest) first"
                    .into(),
            );
        }
        if !self.door.states.contains_key(&self.door.default)
            || !self.door.states.contains_key(&self.door.exit_trigger.opens)
        {
            return Err("the door default and the exit trigger must name door states".into());
        }
        Ok(())
    }

    /// Carl's and Donut's pivot columns (the scene's reference pose).
    pub fn cast_x(&self) -> CastX {
        CastX {
            carl: self.marks.carl.x as f32,
            donut: self.marks.donut.x as f32,
        }
    }

    /// The cast that is placed: Carl, Donut and, only when enabled, Mordecai.
    pub fn placed(&self) -> Vec<(&'static str, &Mark)> {
        let mut out = vec![("donut", &self.marks.donut), ("carl", &self.marks.carl)];
        if self.marks.mordecai.enabled {
            out.push(("mordecai", &self.marks.mordecai));
        }
        out
    }

    /// The body box of a mark, `[left, right)` in view x.
    pub fn body_span(mark: &Mark) -> [i32; 2] {
        [mark.x - BODY_HALF, mark.x + BODY_HALF]
    }

    /// Checks against the loaded kit: every mark is a column of the one walk
    /// line, the cast stays off the hearth opening and the desk, and the door
    /// pieces exist in the layout.
    pub fn check_kit(&self, kit: &EnvKit) -> Result<(), String> {
        let walk = self.walk_line_view_y + VIEW_TOP;
        if kit.base_walk_line_y != walk {
            return Err(format!(
                "walk line: the layout says {} and the room says {walk}",
                kit.base_walk_line_y
            ));
        }
        for (name, mark) in [
            ("donut", &self.marks.donut),
            ("carl", &self.marks.carl),
            ("mordecai", &self.marks.mordecai),
        ] {
            if kit.walking_line_at(mark.x) != Some(walk) {
                return Err(format!("mark {name} at x {} is not on the walk line", mark.x));
            }
            let [left, right] = Self::body_span(mark);
            if mark.enabled || name != "mordecai" {
                for (what, [a, b]) in [
                    ("the hearth opening", self.clearances_view_x.hearth_opening),
                    ("the desk", self.clearances_view_x.desk),
                ] {
                    if left < b && right > a {
                        return Err(format!("mark {name} (x {left}..{right}) overlaps {what}"));
                    }
                }
            }
        }
        // The layout shows the default door state; the other state is swapped in by the exit trigger.
        let shown = &self.door.states[&self.door.default];
        if !kit.pieces.iter().any(|p| &p.piece == shown) {
            return Err(format!("door state piece {shown} is not placed in the layout"));
        }
        Ok(())
    }
}

/// The room and its kit, checked against each other.
pub fn load(root: &Path) -> Result<(EnvKit, Room), String> {
    let room = Room::load(root)?;
    let kit = EnvKit::load(root, &room.layout).map_err(|e: EnvError| e.to_string())?;
    room.check_kit(&kit)?;
    Ok((kit, room))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env_kit::IRect;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn room_and_kit() -> (EnvKit, Room) {
        load(root()).expect("the refuge room loads and checks")
    }

    #[test]
    fn marks_are_the_spec_marks_and_mordecai_is_off() {
        let (_, room) = room_and_kit();
        assert_eq!(
            (room.marks.donut.x, room.marks.carl.x, room.marks.mordecai.x),
            (226, 258, 380)
        );
        assert_eq!(room.marks.donut.facing, Facing::Right);
        assert_eq!(room.marks.carl.facing, Facing::Right);
        assert_eq!(room.marks.mordecai.facing, Facing::Left);
        assert!(!room.marks.mordecai.enabled && room.marks.mordecai.sprite.is_none());
        let placed: Vec<_> = room.placed().iter().map(|(n, _)| *n).collect();
        assert_eq!(placed, ["donut", "carl"]);
        assert_eq!(room.cast_x(), CastX { carl: 258.0, donut: 226.0 });
    }

    #[test]
    fn an_enabled_mordecai_needs_a_sprite() {
        let (_, mut room) = room_and_kit();
        room.marks.mordecai.enabled = true;
        assert!(room.check_shape().unwrap_err().contains("names no sprite"));
        room.marks.mordecai.sprite = Some("assets/manifests/mordecai.json".into());
        room.check_shape().unwrap();
        assert_eq!(room.placed().len(), 3);
    }

    #[test]
    fn walk_line_is_continuous_and_the_only_solids_are_floor_piers_and_ceiling() {
        let (kit, _) = room_and_kit();
        assert_eq!((kit.width, kit.height), (640, 400));
        for x in 16..624 {
            assert_eq!(kit.walking_line_at(x), Some(308), "walk line at x {x}");
        }
        let ceiling_bottom = 32;
        for r in &kit.solid_rects {
            let pier = r.x == 0 && r.w == 16 || r.right() == 640 && r.w == 16;
            let floor = r.y >= 308;
            let ceiling = r.bottom() <= ceiling_bottom;
            assert!(pier || floor || ceiling, "unexpected solid {r:?}");
        }
        assert_eq!(kit.exit.map(|e| (e.x, e.y, e.w, e.h)), Some((40, 248, 64, 60)));
    }

    #[test]
    fn props_stand_behind_the_walk_line_and_the_clear_span_is_free_of_cast_overlaps() {
        let (kit, room) = room_and_kit();
        for p in kit.pieces.iter().filter(|p| p.layer == "prop" && p.piece.starts_with("refuge_")) {
            let bottom = p.y + p.rect.h - 1;
            if p.piece == "refuge_candle_flame" {
                continue; // sits on the desk top
            }
            assert!(
                bottom <= 287,
                "{} ends on layout row {bottom}, behind the walk line means <= 287",
                p.piece
            );
        }
        // The cast boxes: no overlap with the hearth opening or the desk, and
        // with Mordecai switched on his box stays clear of both as well.
        let mut on = room.clone();
        on.marks.mordecai.enabled = true;
        on.marks.mordecai.sprite = Some("x".into());
        on.check_kit(&kit).unwrap();
        // Negative: a hero standing in the fire opening is refused, as is one on the desk.
        for (x, what) in [(320, "the hearth opening"), (452, "the desk")] {
            let mut bad = room.clone();
            bad.marks.carl.x = x;
            let err = bad.check_kit(&kit).unwrap_err();
            assert!(err.contains(what) || err.contains("walk line"), "{err}");
        }
    }

    #[test]
    fn edge_of_each_clearance_is_exact() {
        let (kit, room) = room_and_kit();
        let [a, b] = room.clearances_view_x.hearth_opening;
        // Last x clear on the left (box right edge == a) and first x that overlaps.
        for (x, ok) in [(a - BODY_HALF, true), (a - BODY_HALF + 1, false), (b + BODY_HALF, true), (b + BODY_HALF - 1, false)] {
            let mut r = room.clone();
            r.marks.carl.x = x;
            assert_eq!(r.check_kit(&kit).is_ok(), ok, "carl at {x}");
        }
    }

    #[test]
    fn the_door_trigger_matches_the_exit_marker_and_names_its_states() {
        let (kit, room) = room_and_kit();
        let t = &room.door.exit_trigger;
        let [x, y, w, h] = t.rect_view;
        assert_eq!(
            kit.exit,
            Some(IRect { x, y: y + VIEW_TOP, w, h }),
            "the layout's exit marker is the room's trigger"
        );
        assert_eq!(room.door.default, "closed");
        assert_eq!(t.opens, "open");
        assert_eq!(t.returns_to, "exit_guild");
        assert!(kit.pieces.iter().any(|p| p.piece == "refuge_door_inside_closed"));
    }

    #[test]
    fn the_scene_name_is_not_an_f01_late_room() {
        // PT-SCENE: the f01_late manifest lists its own rooms; the refuge is not one.
        let manifest = crate::f01_late::Manifest::load(root()).expect("manifest");
        assert!(manifest.room(SCENE_NAME).is_err());
    }
}
