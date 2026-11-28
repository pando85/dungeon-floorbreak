//! AREA-REFUGE-01: Mordecai's Tutorial Guild room (`--scene refuge_guild`).
//!
//! The room is its own scene. It is not in the `f01_late` playtest manifest, it
//! is not recorded and it adds no gameplay: the layout
//! (`assets/levels/floor01/refuge_guild.json`, `env-kit-layout-v2`) draws it and
//! the room file (`refuge_guild.scene.json`) names the cast marks, the door and
//! its exit trigger. Pure and headless: no Bevy types.
//!
//! Mordecai's mark is data with an off switch: `enabled: false` hides him. The
//! loader refuses a mark that is enabled without a sprite, so there is never a
//! silent placeholder. His MORDECAI-01 sprite (`mordecai_h44`) is on.

use std::path::Path;

use serde::Deserialize;

use crate::env_kit::{EnvError, EnvKit};

pub const SCENE_NAME: &str = "refuge_guild";
pub const ROOM_PATH: &str = "assets/levels/floor01/refuge_guild.scene.json";
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
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let room: Room =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        room.check_shape()?;
        Ok(room)
    }

    /// Checks that need no kit.
    fn check_shape(&self) -> Result<(), String> {
        if self.schema != ROOM_SCHEMA {
            return Err(format!(
                "room schema {:?}, expected {ROOM_SCHEMA}",
                self.schema
            ));
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
                return Err(format!(
                    "mark {name} at x {} is not on the walk line",
                    mark.x
                ));
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
            return Err(format!(
                "door state piece {shown} is not placed in the layout"
            ));
        }
        Ok(())
    }
}

/// Mordecai as the scene draws him: his MORDECAI-01 atlas (`mordecai_h44`) at his mark. The
/// review room (`refuge_guild`, presentation tick 0) shows the first idle frame; a live room
/// (SMALL-02) loops the `idle` tag over its frame durations and shows the `talk` key while Carl
/// is within [`TALK_RADIUS`] ([`Mordecai::frame_at`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Mordecai {
    /// Atlas image path relative to the Bevy asset root (`generated/art/`).
    pub atlas_asset: String,
    pub frame_w: i32,
    pub frame_h: i32,
    pub pivot_x: i32,
    pub pivot_y: i32,
    /// Frames in the atlas (one row of equally sized frames).
    pub frame_count: usize,
    /// First frame of the `idle` tag.
    pub idle_first: usize,
    /// Ticks (60 Hz, at least 1) each frame of the `idle` tag is held, read from the atlas
    /// frame durations (40, 22, 14 and 22 for MORDECAI-01).
    pub idle_ticks: Vec<u32>,
    /// The `talk` key frame, when the atlas has one.
    pub talk_frame: Option<usize>,
    pub x: i32,
    pub facing: Facing,
}

/// SMALL02-AD1: Mordecai shows his `talk` key while Carl stands within this many pixels of his
/// column (the default of the work order; the simpler of "a text beat is on screen" and "Carl is
/// near", because the guild's lines are HUD messages the scene does not read back).
pub const TALK_RADIUS: i32 = 48;

impl Mordecai {
    /// The atlas frame at presentation tick `tick`: the `talk` key while `talking`, else the
    /// `idle` loop (0, 1, 2, 3, 0, ... at the atlas durations). Pure and deterministic.
    pub fn frame_at(&self, tick: u64, talking: bool) -> usize {
        if let (true, Some(talk)) = (talking, self.talk_frame) {
            return talk;
        }
        let cycle: u64 = self.idle_ticks.iter().map(|&t| u64::from(t)).sum();
        if cycle == 0 {
            return self.idle_first;
        }
        let mut at = tick % cycle;
        for (i, &t) in self.idle_ticks.iter().enumerate() {
            if at < u64::from(t) {
                return self.idle_first + i;
            }
            at -= u64::from(t);
        }
        self.idle_first
    }

    /// Whether Carl at `carl_x` is close enough for the `talk` key.
    pub fn talks_to(&self, carl_x: f32) -> bool {
        (carl_x - self.x as f32).abs() <= TALK_RADIUS as f32
    }
}

impl Room {
    /// The enabled Mordecai mark with his sprite read from the manifest and the exported atlas
    /// metadata (`bash scripts/export_assets.sh`), or `None` while his mark is off.
    pub fn mordecai(&self, root: &Path) -> Result<Option<Mordecai>, String> {
        let mark = &self.marks.mordecai;
        if !mark.enabled {
            return Ok(None);
        }
        let manifest_path = mark
            .sprite
            .as_deref()
            .ok_or("the mordecai mark is enabled but names no sprite")?;
        let read = |rel: &str| -> Result<serde_json::Value, String> {
            let text =
                std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
            serde_json::from_str(&text).map_err(|e| format!("{rel}: {e}"))
        };
        let manifest = read(manifest_path)?;
        let pair = |key: &str| -> Result<(i32, i32), String> {
            let v = &manifest[key];
            match (v[0].as_i64(), v[1].as_i64()) {
                (Some(a), Some(b)) => Ok((a as i32, b as i32)),
                _ => Err(format!("{manifest_path}: missing {key}")),
            }
        };
        let (frame_w, frame_h) = pair("frame_size")?;
        let (pivot_x, pivot_y) = pair("pivot")?;
        let atlas = manifest["output_atlas"]
            .as_str()
            .and_then(|a| a.strip_prefix(crate::env_kit::ATLAS_ASSET_PREFIX))
            .ok_or_else(|| format!("{manifest_path}: output_atlas is not below generated/art/"))?
            .to_owned();
        let meta_path = manifest["output_metadata"]
            .as_str()
            .ok_or_else(|| format!("{manifest_path}: missing output_metadata"))?;
        let meta = read(meta_path)?;
        let frame_count = meta["frames"].as_array().map_or(0, Vec::len);
        let tags = meta["meta"]["frameTags"].as_array();
        let tag = |name: &str| tags.and_then(|tags| tags.iter().find(|t| t["name"] == name));
        let idle = tag("idle").ok_or_else(|| format!("{meta_path}: no idle tag"))?;
        let idle_first = idle["from"]
            .as_u64()
            .ok_or_else(|| format!("{meta_path}: no idle tag"))? as usize;
        let idle_last = idle["to"].as_u64().map_or(idle_first, |to| to as usize);
        let talk_frame = tag("talk")
            .and_then(|t| t["from"].as_u64())
            .map(|f| f as usize)
            .filter(|f| *f < frame_count);
        let idle_ticks: Vec<u32> = (idle_first..=idle_last.min(frame_count.saturating_sub(1)))
            .map(|i| {
                let ms = meta["frames"][i]["duration"].as_f64().unwrap_or(0.0);
                ((ms * 60.0 / 1000.0).round() as u32).max(1)
            })
            .collect();
        if frame_count == 0 || idle_first >= frame_count {
            return Err(format!(
                "{meta_path}: idle frame {idle_first} of {frame_count}"
            ));
        }
        Ok(Some(Mordecai {
            atlas_asset: atlas,
            frame_w,
            frame_h,
            pivot_x,
            pivot_y,
            frame_count,
            idle_first,
            idle_ticks,
            talk_frame,
            x: mark.x,
            facing: mark.facing,
        }))
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
    fn marks_are_the_spec_marks_and_mordecai_is_on_his_sprite() {
        let (_, room) = room_and_kit();
        assert_eq!(
            (room.marks.donut.x, room.marks.carl.x, room.marks.mordecai.x),
            (226, 258, 380)
        );
        assert_eq!(room.marks.donut.facing, Facing::Right);
        assert_eq!(room.marks.carl.facing, Facing::Right);
        assert_eq!(room.marks.mordecai.facing, Facing::Left);
        assert!(room.marks.mordecai.enabled);
        assert_eq!(
            room.marks.mordecai.sprite.as_deref(),
            Some("assets/manifests/mordecai_h44.json")
        );
        let placed: Vec<_> = room.placed().iter().map(|(n, _)| *n).collect();
        assert_eq!(placed, ["donut", "carl", "mordecai"]);
        assert_eq!(
            room.cast_x(),
            CastX {
                carl: 258.0,
                donut: 226.0
            }
        );
    }

    #[test]
    fn an_enabled_mordecai_needs_a_sprite() {
        let (_, mut room) = room_and_kit();
        room.marks.mordecai.sprite = None;
        assert!(room.check_shape().unwrap_err().contains("names no sprite"));
        assert!(room.mordecai(root()).is_err());
        // Switched off, he is not placed and not loaded (the placeholder state).
        room.marks.mordecai.enabled = false;
        room.check_shape().unwrap();
        assert_eq!(room.placed().len(), 2);
        assert_eq!(room.mordecai(root()), Ok(None));
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
        assert_eq!(
            kit.exit.map(|e| (e.x, e.y, e.w, e.h)),
            Some((40, 248, 64, 60))
        );
    }

    #[test]
    fn props_stand_behind_the_walk_line_and_the_clear_span_is_free_of_cast_overlaps() {
        let (kit, room) = room_and_kit();
        for p in kit
            .pieces
            .iter()
            .filter(|p| p.layer == "prop" && p.piece.starts_with("refuge_"))
        {
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
        // The cast boxes (Mordecai included, he is on): no overlap with the hearth opening or the desk.
        room.check_kit(&kit).unwrap();
        let m = Room::body_span(&room.marks.mordecai);
        assert!(
            m[0] > 359 && m[1] < 424,
            "Mordecai {m:?} stands between the hearth (ends 359) and the desk (starts 424)"
        );
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
        for (x, ok) in [
            (a - BODY_HALF, true),
            (a - BODY_HALF + 1, false),
            (b + BODY_HALF, true),
            (b + BODY_HALF - 1, false),
        ] {
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
            Some(IRect {
                x,
                y: y + VIEW_TOP,
                w,
                h
            }),
            "the layout's exit marker is the room's trigger"
        );
        assert_eq!(room.door.default, "closed");
        assert_eq!(t.opens, "open");
        assert_eq!(t.returns_to, "exit_guild");
        assert!(
            kit.pieces
                .iter()
                .any(|p| p.piece == "refuge_door_inside_closed")
        );
    }

    /// His sprite loads from the exported MORDECAI-01 atlas (skipped before the export).
    #[test]
    fn mordecai_loads_from_the_exported_atlas() {
        let (_, room) = room_and_kit();
        if !root().join("generated/art/mordecai_h44.json").exists() {
            return;
        }
        let m = room.mordecai(root()).unwrap().expect("enabled");
        assert_eq!(
            (m.frame_w, m.frame_h, m.pivot_x, m.pivot_y),
            (52, 48, 24, 47)
        );
        assert_eq!(
            (m.x, m.facing, m.idle_first, m.atlas_asset.as_str()),
            (380, Facing::Left, 0, "mordecai_h44.png")
        );
        assert!(m.frame_count >= 5);
        // SMALL-02: the loop and the talk key come from the atlas tags and durations.
        assert_eq!(m.idle_ticks, vec![40, 22, 14, 22]);
        assert_eq!(m.talk_frame, Some(4));
    }

    fn sample() -> Mordecai {
        Mordecai {
            atlas_asset: "mordecai_h44.png".into(),
            frame_w: 52,
            frame_h: 48,
            pivot_x: 24,
            pivot_y: 47,
            frame_count: 5,
            idle_first: 0,
            idle_ticks: vec![40, 22, 14, 22],
            talk_frame: Some(4),
            x: 380,
            facing: Facing::Left,
        }
    }

    /// SMALL02-AD1: the idle loop holds 40, 22, 14 and 22 ticks (98 in all), tick 0 is the first
    /// idle frame (the review pose), and the talk key replaces the loop while Carl is near.
    #[test]
    fn mordecai_frame_follows_the_loop_and_the_talk_radius() {
        let m = sample();
        let at = |t| m.frame_at(t, false);
        assert_eq!((at(0), at(39), at(40), at(61), at(62)), (0, 0, 1, 1, 2));
        assert_eq!(
            (at(75), at(76), at(97), at(98), at(98 + 40)),
            (2, 3, 3, 0, 1)
        );
        assert_eq!(m.frame_at(0, true), 4);
        assert_eq!(m.frame_at(500, true), 4);
        assert!(m.talks_to(380.0 + TALK_RADIUS as f32) && m.talks_to(380.0 - TALK_RADIUS as f32));
        assert!(!m.talks_to(380.0 + TALK_RADIUS as f32 + 0.5));
        let no_talk = Mordecai {
            talk_frame: None,
            ..sample()
        };
        assert_eq!(no_talk.frame_at(41, true), 1);
    }

    #[test]
    fn the_scene_name_is_not_an_f01_late_room() {
        // PT-SCENE: the f01_late manifest lists its own rooms; the refuge is not one.
        let manifest = crate::f01_late::Manifest::load(root()).expect("manifest");
        assert!(manifest.room(SCENE_NAME).is_err());
    }
}
