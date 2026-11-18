//! Bevy-free art bookkeeping of the lab: which approved atlas and which frame a
//! simulation state draws. Frame choice is a pure function of the simulation
//! (so it is deterministic and identical at every coverage); nothing here feeds
//! back into it.
//!
//! Atlases are the approved production sprites exported by
//! `scripts/export_assets.sh` (plus the SWINE-01 atlas exported by
//! `scripts/camera_lab_assets.sh`): frame rectangles, durations and tags come
//! from the exported atlas JSON, the frame size and pivot from the manifest.

use std::collections::BTreeMap;
use std::path::Path;

use super::room::EnemyKind;
use super::sim::{CARL, DONUT, Foe, FoeMode, LabSim};
use crate::combat::MoveId;
use crate::goblin::{Goblin, GoblinMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AtlasId {
    CarlLoco,
    CarlMelee,
    DonutLoco,
    DonutMelee,
    GoblinA,
    GoblinB,
    Llama,
    Engineer,
    Scatterer,
    Hoarder,
    Trog,
    Juicer,
    Swine,
    HitSpark,
}

pub const ALL_ATLASES: [AtlasId; 14] = [
    AtlasId::CarlLoco,
    AtlasId::CarlMelee,
    AtlasId::DonutLoco,
    AtlasId::DonutMelee,
    AtlasId::GoblinA,
    AtlasId::GoblinB,
    AtlasId::Llama,
    AtlasId::Engineer,
    AtlasId::Scatterer,
    AtlasId::Hoarder,
    AtlasId::Trog,
    AtlasId::Juicer,
    AtlasId::Swine,
    AtlasId::HitSpark,
];

impl AtlasId {
    /// Manifest file under `assets/manifests` (None: SWINE-01, which has no
    /// manifest yet).
    pub fn manifest(self) -> Option<&'static str> {
        Some(match self {
            Self::CarlLoco => "carl_book1_h52_locomotion.json",
            Self::CarlMelee => "carl_book1_h52_melee.json",
            Self::DonutLoco => "donut_h40_locomotion.json",
            Self::DonutMelee => "donut_h40_melee.json",
            Self::GoblinA => "goblin_a_h36.json",
            Self::GoblinB => "goblin_b_h36.json",
            Self::Llama => "llama-01.json",
            Self::Engineer => "eng_01.json",
            Self::Scatterer => "scat-01.json",
            Self::Hoarder => "hoard-01.json",
            Self::Trog => "trog-02.json",
            Self::Juicer => "juice-01.json",
            Self::HitSpark => "vfx_hit_spark_metal.json",
            Self::Swine => return None,
        })
    }
    /// Atlas image name below `generated/art/` (also the Bevy asset path).
    pub fn image(self) -> &'static str {
        match self {
            Self::CarlLoco => "carl_book1_h52_locomotion.png",
            Self::CarlMelee => "carl_book1_h52_melee.png",
            Self::DonutLoco => "donut_h40_locomotion.png",
            Self::DonutMelee => "donut_h40_melee.png",
            Self::GoblinA => "goblin_a_h36.png",
            Self::GoblinB => "goblin_b_h36.png",
            Self::Llama => "llama-01.png",
            Self::Engineer => "eng_01.png",
            Self::Scatterer => "scat-01.png",
            Self::Hoarder => "hoard-01.png",
            Self::Trog => "trog_02.png",
            Self::Juicer => "juice_01.png",
            Self::Swine => "swine_01.png",
            Self::HitSpark => "vfx_hit_spark_metal.png",
        }
    }
    pub fn json(self) -> String {
        format!("generated/art/{}", self.image().replace(".png", ".json"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone)]
pub struct Clips {
    pub id: AtlasId,
    pub frame_w: i32,
    pub frame_h: i32,
    pub pivot: (i32, i32),
    pub frames: Vec<(FrameRect, u32)>,
    pub tags: BTreeMap<String, (usize, usize)>,
}

fn json_of(root: &Path, rel: &str) -> Result<serde_json::Value, String> {
    let text = std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{rel}: {e}"))
}

impl Clips {
    pub fn load(root: &Path, id: AtlasId) -> Result<Self, String> {
        let atlas = json_of(root, &id.json())?;
        let (frame, pivot) = match id.manifest() {
            Some(file) => {
                let m = json_of(root, &format!("assets/manifests/{file}"))?;
                let pair = |key: &str| -> Result<(i32, i32), String> {
                    let a = m[key].as_array().ok_or_else(|| format!("{file}: {key}"))?;
                    Ok((
                        a[0].as_i64().unwrap_or(0) as i32,
                        a[1].as_i64().unwrap_or(0) as i32,
                    ))
                };
                (pair("frame_size")?, pair("pivot")?)
            }
            None => {
                let m = json_of(
                    root,
                    "assets/source/characters/swine/swine-01.pixelart.json",
                )?;
                let pair = |key: &str| -> (i32, i32) {
                    (
                        m[key][0].as_i64().unwrap_or(0) as i32,
                        m[key][1].as_i64().unwrap_or(0) as i32,
                    )
                };
                (pair("frame_size"), pair("pivot"))
            }
        };
        let mut frames = Vec::new();
        for f in atlas["frames"].as_array().ok_or("frames")? {
            let r = &f["frame"];
            let ms = f["duration"].as_u64().unwrap_or(100) as f32;
            frames.push((
                FrameRect {
                    x: r["x"].as_i64().unwrap_or(0) as i32,
                    y: r["y"].as_i64().unwrap_or(0) as i32,
                    w: r["w"].as_i64().unwrap_or(0) as i32,
                    h: r["h"].as_i64().unwrap_or(0) as i32,
                },
                ((ms * 0.06).round() as u32).max(1),
            ));
        }
        let mut tags = BTreeMap::new();
        for t in atlas["meta"]["frameTags"].as_array().ok_or("frameTags")? {
            tags.insert(
                t["name"].as_str().unwrap_or("").to_owned(),
                (
                    t["from"].as_u64().unwrap_or(0) as usize,
                    t["to"].as_u64().unwrap_or(0) as usize,
                ),
            );
        }
        Ok(Self {
            id,
            frame_w: frame.0,
            frame_h: frame.1,
            pivot,
            frames,
            tags,
        })
    }

    pub fn has(&self, tag: &str) -> bool {
        self.tags.contains_key(tag)
    }

    fn range(&self, tag: &str) -> (usize, usize) {
        *self
            .tags
            .get(tag)
            .or_else(|| self.tags.get("idle"))
            .unwrap_or(&(0, 0))
    }

    /// Frame of `tag` shown `age` ticks after it started; the last frame holds.
    pub fn frame_at(&self, tag: &str, age: u32) -> usize {
        let (a, b) = self.range(tag);
        let mut left = age;
        for (i, (_, ticks)) in self.frames.iter().enumerate().take(b + 1).skip(a) {
            let ticks = *ticks;
            if left < ticks {
                return i;
            }
            left -= ticks;
        }
        b
    }

    /// Frame of a looping tag at `tick`.
    pub fn loop_frame(&self, tag: &str, tick: u64) -> usize {
        let (a, b) = self.range(tag);
        let total: u32 = (a..=b).map(|i| self.frames[i].1).sum();
        self.frame_at(tag, (tick % u64::from(total.max(1))) as u32)
    }

    /// Frame of `tag` after `done` of `total` ticks of an action stretched over
    /// the tag's frames (an attack whose tick count differs from the art's).
    pub fn progress_frame(&self, tag: &str, done: u32, total: u32) -> usize {
        let (a, b) = self.range(tag);
        let n = (b - a + 1) as u32;
        a + ((done * n) / total.max(1)).min(n - 1) as usize
    }
}

/// What to draw for one actor or enemy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub atlas: AtlasId,
    pub frame: usize,
    pub flip: bool,
    /// Pivot (feet) position, y up.
    pub foot_x: f32,
    pub foot_y: f32,
    /// Impact flash (presentation).
    pub flash: bool,
}

/// All atlases of the lab.
pub struct ArtSet {
    pub clips: BTreeMap<AtlasId, Clips>,
}

impl ArtSet {
    pub fn load(root: &Path) -> Result<Self, String> {
        let mut clips = BTreeMap::new();
        for id in ALL_ATLASES {
            clips.insert(id, Clips::load(root, id)?);
        }
        Ok(Self { clips })
    }
    pub fn get(&self, id: AtlasId) -> &Clips {
        &self.clips[&id]
    }

    /// Carl or Donut as drawn.
    pub fn actor_pose(&self, sim: &LabSim, i: usize) -> Pose {
        let a = &sim.actors[i];
        let (loco, melee) = if i == CARL {
            (AtlasId::CarlLoco, AtlasId::CarlMelee)
        } else {
            (AtlasId::DonutLoco, AtlasId::DonutMelee)
        };
        let base = Pose {
            atlas: loco,
            frame: 0,
            flip: a.body.facing < 0,
            foot_x: a.body.x,
            foot_y: a.body.feet_y(),
            flash: false,
        };
        let m = self.get(melee);
        let l = self.get(loco);
        if a.down() {
            let tag = "knockdown";
            return Pose {
                atlas: melee,
                frame: m.frame_at(tag, a.down_ticks as u32),
                ..base
            };
        }
        if a.hurt_ticks > 0 {
            let age = u32::from(crate::feel::HERO_HURT_TICKS.saturating_sub(a.hurt_ticks));
            return Pose {
                atlas: melee,
                frame: m.frame_at("hurt", age),
                ..base
            };
        }
        if let Some(mt) = a.current_move() {
            let tag = match (i, mt.spec.id) {
                (CARL, MoveId::Jab) => "punch_a",
                (CARL, MoveId::Haymaker) => "punch_b",
                (CARL, _) => "kick",
                (_, MoveId::Jab) => "pounce",
                _ => "swat",
            };
            return Pose {
                atlas: melee,
                frame: m.progress_frame(tag, u32::from(mt.tick), u32::from(mt.spec.total())),
                ..base
            };
        }
        let tag = if !a.body.grounded {
            if a.body.vy > 60.0 {
                "jump_rise"
            } else {
                "jump_fall"
            }
        } else if a.body.vx.abs() > 0.1 {
            "run"
        } else {
            "idle"
        };
        Pose {
            frame: l.loop_frame(tag, sim.tick),
            ..base
        }
    }

    pub fn goblin_pose(&self, sim: &LabSim, index: usize, g: &Goblin) -> Pose {
        let id = if index.is_multiple_of(2) {
            AtlasId::GoblinA
        } else {
            AtlasId::GoblinB
        };
        let c = self.get(id);
        let tick = sim.tick;
        let frame = match g.mode {
            GoblinMode::Neutral => {
                if g.vx.abs() > 0.1 || sim_goblin_walking(g) {
                    c.loop_frame("walk", tick + index as u64 * 7)
                } else {
                    c.loop_frame("idle", tick + index as u64 * 13)
                }
            }
            GoblinMode::Attack => match crate::goblin::attack_phase(g.attack_tick) {
                crate::goblin::AttackPhase::Tell => {
                    c.progress_frame("attack_tell", u32::from(g.attack_tick), 24)
                }
                crate::goblin::AttackPhase::Smear | crate::goblin::AttackPhase::Impact => {
                    c.progress_frame("attack_swing", u32::from(g.attack_tick) - 24, 6)
                }
                crate::goblin::AttackPhase::Recover => {
                    c.progress_frame("attack_recover", u32::from(g.attack_tick) - 30, 20)
                }
            },
            GoblinMode::Hurt => c.frame_at("hurt", 0),
            GoblinMode::Airborne => c.frame_at("knockback", g.down_ticks as u32),
            GoblinMode::Down => c.frame_at("down", u32::from(g.down_ticks)),
            GoblinMode::Corpse => c.frame_at("corpse", 0),
        };
        Pose {
            atlas: id,
            frame,
            flip: g.facing < 0,
            foot_x: g.x,
            foot_y: g.y - crate::combat::ENEMY_HALF_HEIGHT,
            flash: false,
        }
    }

    pub fn foe_pose(&self, sim: &LabSim, index: usize, f: &Foe) -> Pose {
        let (id, attack_tag, recover_tag) = match f.spec.kind {
            EnemyKind::Llama => (AtlasId::Llama, "attack_swing", "attack_swing"),
            EnemyKind::Engineer => (AtlasId::Engineer, "attack_ram", "attack_recover"),
            EnemyKind::Scatterer => (AtlasId::Scatterer, "attack_swing", "attack_swing"),
            EnemyKind::Hoarder => (AtlasId::Hoarder, "attack_swing", "attack_swing"),
            EnemyKind::Trog => (AtlasId::Trog, "attack_smash", "attack_recover"),
            EnemyKind::Juicer => (AtlasId::Juicer, "attack_pound", "attack_recover"),
            EnemyKind::Ball | EnemyKind::Goblin => {
                (AtlasId::Swine, "attack_charge", "attack_charge")
            }
        };
        let c = self.get(id);
        let tick = sim.tick;
        let ranged = f.spec.ranged;
        let frame = match f.mode {
            FoeMode::Neutral => {
                let moving = f.vx.abs() > 1.0;
                let tag = if moving {
                    if id == AtlasId::Swine {
                        "roll"
                    } else if c.has("walk") {
                        "walk"
                    } else {
                        "creep"
                    }
                } else {
                    "idle"
                };
                c.loop_frame(tag, tick + index as u64 * 11)
            }
            FoeMode::Tell => c.progress_frame(
                "attack_tell",
                u32::from(f.mode_tick),
                u32::from(f.spec.tell),
            ),
            FoeMode::Active => {
                let tag = if ranged && c.has("spit") {
                    "spit"
                } else {
                    attack_tag
                };
                c.progress_frame(tag, u32::from(f.mode_tick), u32::from(f.spec.active))
            }
            FoeMode::Recover => {
                if id == AtlasId::Swine {
                    c.frame_at(if f.stun > 0 { "hurt" } else { "idle" }, 0)
                } else {
                    c.progress_frame(
                        recover_tag,
                        u32::from(f.mode_tick),
                        u32::from(f.spec.recover).max(1),
                    )
                }
            }
            FoeMode::Hurt => c.frame_at("hurt", 0),
            FoeMode::Down | FoeMode::Corpse => c.frame_at("down", u32::from(f.mode_tick) * 3),
        };
        // Foes face the way their sprite faces: art faces right.
        Pose {
            atlas: id,
            frame,
            flip: f.facing < 0,
            foot_x: f.x,
            foot_y: f.y - f.spec.hh,
            flash: false,
        }
    }
}

fn sim_goblin_walking(_g: &Goblin) -> bool {
    false
}

/// Every atlas the lab reads, for the load check of the binary and the tests.
pub fn required_assets() -> Vec<String> {
    ALL_ATLASES
        .iter()
        .map(|a| format!("generated/art/{}", a.image()))
        .collect()
}

/// Donut and Carl indices for pose lookups.
pub const ACTORS: [usize; 2] = [CARL, DONUT];

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    /// Skipped (with a note) when the atlases are not exported: CI exports
    /// them before the tests (`scripts/export_assets.sh`).
    #[test]
    fn every_lab_atlas_loads_with_the_tags_the_poses_use() {
        if !root().join("generated/art/goblin_a_h36.json").exists()
            || !root().join("generated/art/swine_01.json").exists()
        {
            eprintln!(
                "atlases not exported; run scripts/export_assets.sh and scripts/camera_lab_assets.sh"
            );
            return;
        }
        let art = ArtSet::load(root()).unwrap();
        for (id, tags) in [
            (
                AtlasId::CarlLoco,
                &["idle", "run", "jump_rise", "jump_fall"][..],
            ),
            (
                AtlasId::CarlMelee,
                &["punch_a", "punch_b", "kick", "hurt", "knockdown"][..],
            ),
            (
                AtlasId::DonutLoco,
                &["idle", "run", "jump_rise", "jump_fall"][..],
            ),
            (
                AtlasId::DonutMelee,
                &["swat", "pounce", "hurt", "knockdown"][..],
            ),
            (
                AtlasId::GoblinA,
                &[
                    "idle",
                    "walk",
                    "attack_tell",
                    "attack_swing",
                    "attack_recover",
                    "hurt",
                    "knockback",
                    "down",
                    "corpse",
                ][..],
            ),
            (
                AtlasId::Llama,
                &["idle", "walk", "attack_tell", "spit", "hurt", "down"][..],
            ),
            (
                AtlasId::Engineer,
                &[
                    "idle",
                    "walk",
                    "attack_tell",
                    "attack_ram",
                    "attack_recover",
                    "hurt",
                    "down",
                ][..],
            ),
            (
                AtlasId::Scatterer,
                &[
                    "idle",
                    "walk",
                    "attack_tell",
                    "attack_swing",
                    "hurt",
                    "down",
                ][..],
            ),
            (
                AtlasId::Hoarder,
                &[
                    "idle",
                    "walk",
                    "attack_tell",
                    "attack_swing",
                    "hurt",
                    "down",
                ][..],
            ),
            (
                AtlasId::Trog,
                &[
                    "idle",
                    "walk",
                    "attack_tell",
                    "attack_smash",
                    "attack_recover",
                    "hurt",
                    "down",
                ][..],
            ),
            (
                AtlasId::Juicer,
                &[
                    "idle",
                    "walk",
                    "attack_tell",
                    "attack_pound",
                    "attack_recover",
                    "hurt",
                    "down",
                ][..],
            ),
            (
                AtlasId::Swine,
                &[
                    "idle",
                    "roll",
                    "attack_tell",
                    "attack_charge",
                    "hurt",
                    "down",
                ][..],
            ),
        ] {
            let c = art.get(id);
            for t in tags {
                assert!(c.has(t), "{id:?} lacks tag {t}");
            }
        }
        // Production sizes: Carl 48x56 frame with the 52 px figure, Donut 72x44.
        assert_eq!(
            (
                art.get(AtlasId::CarlLoco).frame_w,
                art.get(AtlasId::CarlLoco).frame_h
            ),
            (48, 56)
        );
        assert_eq!(
            (
                art.get(AtlasId::DonutLoco).frame_w,
                art.get(AtlasId::DonutLoco).frame_h
            ),
            (72, 44)
        );
        assert_eq!(art.get(AtlasId::Swine).frame_w, 176);
    }

    #[test]
    fn frame_selection_is_pure_and_holds_the_last_frame() {
        let mut tags = BTreeMap::new();
        tags.insert("a".to_owned(), (0usize, 2usize));
        let c = Clips {
            id: AtlasId::CarlLoco,
            frame_w: 1,
            frame_h: 1,
            pivot: (0, 0),
            frames: (0..3)
                .map(|i| {
                    (
                        FrameRect {
                            x: i,
                            y: 0,
                            w: 1,
                            h: 1,
                        },
                        4,
                    )
                })
                .collect(),
            tags,
        };
        assert_eq!(c.frame_at("a", 0), 0);
        assert_eq!(c.frame_at("a", 4), 1);
        assert_eq!(c.frame_at("a", 11), 2);
        assert_eq!(c.frame_at("a", 500), 2);
        assert_eq!(c.loop_frame("a", 12), 0);
        assert_eq!(c.progress_frame("a", 5, 10), 1);
    }
}
