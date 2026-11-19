//! DEATH-C: the ENV-02 rotten shore and its falling block, drawn in the combat
//! fixtures.
//!
//! Pure, Bevy-free and presentation only: it maps the **existing simulated**
//! [`Shore`] state (hits, block state, block height) to which approved ENV-02
//! piece is drawn where, using the piece rects and anchors of the opening kit
//! contract (`floor01_opening_kit.json`). Nothing here is read by the
//! simulation, and the pieces stay non-collidable exactly as in the contract.
//!
//! Placement follows the opening route layout, relative to the shore's strike
//! column: the post foot 7 px left of it and 10 px behind the walking line, the
//! block centred 65 px right of it (the middle of the 80 px hazard footprint)
//! with its bottom 140 px above the walking line while seated, the wall
//! cavity behind it, the impact dust on the landing.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::combat::CombatEvent;
use crate::shore::{BlockState, Shore, ShoreState};

pub const KIT_PATH: &str = "assets/source/terrain/floor01/floor01_opening_kit.json";

/// Post foot, left of the strike column (opening layout: foot x 2968, strike 2975).
const POST_FOOT_DX: f32 = -7.0;
/// The post foot stands this far above the walking line (deck back).
const POST_FOOT_DY: f32 = 10.0;
/// Block centre, right of the strike column.
const BLOCK_DX: f32 = 65.0;
/// Top-left of the seated block relative to its bottom-centre anchor (px).
const SEATED_ANCHOR: (f32, f32) = (32.0, 48.0);
/// Ticks one impact-dust frame is shown (83 ms at 60 Hz).
pub const DUST_TICKS_PER_FRAME: u64 = 5;
/// Ticks one trickle frame is shown (117 ms).
pub const TRICKLE_TICKS_PER_FRAME: u64 = 7;

#[derive(Deserialize)]
struct Kit {
    atlases: BTreeMap<String, KitAtlas>,
    pieces: Vec<KitPiece>,
}

#[derive(Deserialize)]
struct KitAtlas {
    image: String,
}

#[derive(Deserialize)]
struct KitPiece {
    id: String,
    atlas: String,
    rect: [i32; 4],
    anchor: [i32; 2],
    #[serde(default)]
    animation: Option<KitAnimation>,
}

#[derive(Deserialize)]
struct KitAnimation {
    frames: Vec<KitFrame>,
}

#[derive(Deserialize)]
struct KitFrame {
    frame: KitRect,
}

#[derive(Deserialize)]
struct KitRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

/// One sprite slot of the shore art (each is one pre-spawned sprite).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShoreSlot {
    /// The socket in the back wall (`backwall_decal` layer).
    Cavity,
    Post,
    Block,
    Trickle,
    Impact,
}

impl ShoreSlot {
    pub const ALL: [Self; 5] = [
        Self::Cavity,
        Self::Post,
        Self::Block,
        Self::Trickle,
        Self::Impact,
    ];

    /// Draw order inside the `prop` layer (added to its z): the dust over the
    /// landed block, the falling block over the post.
    pub const fn z_bias(self) -> f32 {
        match self {
            Self::Cavity | Self::Post => 0.10,
            Self::Block => 0.20,
            Self::Trickle => 0.25,
            Self::Impact => 0.30,
        }
    }
}

/// One sprite to show: the image rect and the world position of its centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShoreDraw {
    pub slot: ShoreSlot,
    /// Index into [`ShoreArt::images`].
    pub image: usize,
    /// Pixel rect in the image: x, y, w, h.
    pub rect: [i32; 4],
    /// Centre of the sprite, y-up world px (whole pixels).
    pub centre: (f32, f32),
}

#[derive(Debug, Clone)]
struct Piece {
    image: usize,
    rect: [i32; 4],
    anchor: (f32, f32),
    frames: Vec<[i32; 4]>,
}

/// The approved ENV-02 pieces the shore uses, read from the opening kit contract.
#[derive(Debug, Clone)]
pub struct ShoreArt {
    /// Asset names (below `generated/art/`) of the images used.
    pub images: Vec<String>,
    pieces: BTreeMap<String, Piece>,
}

const USED: [&str; 11] = [
    "shore_post_intact",
    "shore_post_damaged_1",
    "shore_post_damaged_2",
    "shore_post_broken",
    "loose_block_seated",
    "loose_block_settling",
    "loose_block_slipped",
    "loose_block_falling",
    "loose_block_landed",
    "wall_cavity",
    "shore_dust_trickle",
];

impl ShoreArt {
    pub fn load(root: &Path) -> Result<Self, String> {
        let text =
            std::fs::read_to_string(root.join(KIT_PATH)).map_err(|e| format!("{KIT_PATH}: {e}"))?;
        let kit: Kit = serde_json::from_str(&text).map_err(|e| format!("{KIT_PATH}: {e}"))?;
        let mut images: Vec<String> = Vec::new();
        let mut pieces = BTreeMap::new();
        for id in USED.iter().copied().chain(["block_impact_dust"]) {
            let p = kit
                .pieces
                .iter()
                .find(|p| p.id == id)
                .ok_or_else(|| format!("{KIT_PATH}: no piece {id}"))?;
            let image = kit
                .atlases
                .get(&p.atlas)
                .ok_or_else(|| format!("{KIT_PATH}: no atlas {}", p.atlas))?
                .image
                .strip_prefix(crate::env_kit::ATLAS_ASSET_PREFIX)
                .ok_or_else(|| {
                    format!("{KIT_PATH}: atlas {} is not below the asset root", p.atlas)
                })?
                .to_owned();
            let index = images.iter().position(|i| *i == image).unwrap_or_else(|| {
                images.push(image);
                images.len() - 1
            });
            let frames = p
                .animation
                .as_ref()
                .map(|a| {
                    a.frames
                        .iter()
                        .map(|f| [f.frame.x, f.frame.y, f.frame.w, f.frame.h])
                        .collect()
                })
                .unwrap_or_default();
            pieces.insert(
                id.to_owned(),
                Piece {
                    image: index,
                    rect: p.rect,
                    anchor: (p.anchor[0] as f32, p.anchor[1] as f32),
                    frames,
                },
            );
        }
        Ok(Self { images, pieces })
    }

    fn draw(&self, slot: ShoreSlot, id: &str, frame: usize, anchor_world: (f32, f32)) -> ShoreDraw {
        let p = &self.pieces[id];
        let rect = if p.frames.is_empty() {
            p.rect
        } else {
            p.frames[frame % p.frames.len()]
        };
        // The anchor is a point of the rect (y down); the centre of the sprite
        // is (w/2 - ax, ay - h/2) from it in y-up world px.
        let centre = (
            (anchor_world.0 + rect[2] as f32 / 2.0 - p.anchor.0).round(),
            (anchor_world.1 + p.anchor.1 - rect[3] as f32 / 2.0).round(),
        );
        ShoreDraw {
            slot,
            image: p.image,
            rect,
            centre,
        }
    }

    /// The sprites to draw for `shore`. `landed_age` is the number of ticks
    /// since the block landed (`None` before). Pure: a function of the
    /// simulated shore state.
    pub fn draw_list(&self, shore: &Shore, landed_age: Option<u64>, clock: u64) -> Vec<ShoreDraw> {
        let centre_x = shore.strike_x + BLOCK_DX;
        let foot = shore.foot_y;
        let mut out = Vec::new();
        let post = match shore.state() {
            ShoreState::Intact => "shore_post_intact",
            ShoreState::Damaged1 => "shore_post_damaged_1",
            ShoreState::Damaged2 => "shore_post_damaged_2",
            ShoreState::Broken => "shore_post_broken",
        };
        out.push(self.draw(
            ShoreSlot::Post,
            post,
            0,
            (shore.strike_x + POST_FOOT_DX, foot + POST_FOOT_DY),
        ));
        let seated_bottom = foot + crate::shore::BLOCK_FALL_HEIGHT;
        let (block, anchor_y) = match shore.block_state() {
            BlockState::Seated => ("loose_block_seated", seated_bottom),
            BlockState::Settling => ("loose_block_settling", seated_bottom),
            BlockState::Slipped => ("loose_block_slipped", seated_bottom),
            BlockState::Falling => ("loose_block_falling", foot + shore.block_bottom),
            BlockState::Landed => ("loose_block_landed", foot),
        };
        out.push(self.draw(ShoreSlot::Block, block, 0, (centre_x, anchor_y)));
        // The socket: its anchor (8, 8) sits on the seated block's top-left corner.
        let corner = (centre_x - SEATED_ANCHOR.0, seated_bottom + SEATED_ANCHOR.1);
        out.push(self.draw(ShoreSlot::Cavity, "wall_cavity", 0, corner));
        // Crumbs under the block's split from the first hit until it falls.
        if shore.hits >= 1
            && !matches!(
                shore.block_state(),
                BlockState::Falling | BlockState::Landed
            )
        {
            let at = (centre_x - SEATED_ANCHOR.0 + 41.0, seated_bottom - 2.0);
            out.push(self.draw(
                ShoreSlot::Trickle,
                "shore_dust_trickle",
                (clock / TRICKLE_TICKS_PER_FRAME) as usize,
                at,
            ));
        }
        // The impact dust: four frames from the landing tick.
        if let Some(age) = landed_age {
            let frames = self.pieces["block_impact_dust"].frames.len() as u64;
            if age / DUST_TICKS_PER_FRAME < frames {
                out.push(self.draw(
                    ShoreSlot::Impact,
                    "block_impact_dust",
                    (age / DUST_TICKS_PER_FRAME) as usize,
                    (centre_x, foot),
                ));
            }
        }
        out
    }
}

/// Ticks since the block of the current attempt landed, from the event log
/// (`None` before the landing; a `Restart` starts a new attempt).
pub fn landed_age(events: &[CombatEvent], tick: u64) -> Option<u64> {
    for event in events.iter().rev() {
        match *event {
            CombatEvent::Restart { .. } => return None,
            CombatEvent::ShoreLanded { tick: landed } => return Some(tick.saturating_sub(landed)),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn art() -> ShoreArt {
        ShoreArt::load(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    fn shore(hits: u8) -> Shore {
        let mut s = Shore::new(419.0, 100.0);
        s.hits = hits;
        s
    }

    fn find(list: &[ShoreDraw], slot: ShoreSlot) -> Option<ShoreDraw> {
        list.iter().copied().find(|d| d.slot == slot)
    }

    #[test]
    fn the_landing_age_counts_from_the_event_of_this_attempt() {
        let landed = CombatEvent::ShoreLanded { tick: 139 };
        assert_eq!(landed_age(&[], 150), None);
        assert_eq!(landed_age(&[landed], 139), Some(0));
        assert_eq!(landed_age(&[landed], 150), Some(11));
        let restart = CombatEvent::Restart {
            tick: 300,
            deaths: 1,
        };
        assert_eq!(landed_age(&[landed, restart], 10), None);
    }

    #[test]
    fn the_art_comes_from_the_opening_kit_contract() {
        let art = art();
        assert_eq!(
            art.images,
            [
                "floor01_opening_props.png",
                "floor01_opening_decal.png",
                "floor01_shore_dust.png",
                "floor01_block_impact.png"
            ]
        );
        assert_eq!(art.pieces["loose_block_landed"].rect, [400, 144, 96, 32]);
        assert_eq!(art.pieces["block_impact_dust"].frames.len(), 4);
    }

    #[test]
    fn each_simulated_state_draws_its_approved_piece_where_the_opening_layout_has_it() {
        let art = art();
        // Seated: the opening layout puts the block bottom-centre 140 px above
        // the walking line and 65 px right of the strike column.
        let list = art.draw_list(&shore(0), None, 0);
        let block = find(&list, ShoreSlot::Block).unwrap();
        assert_eq!(block.rect, [136, 144, 64, 56]);
        // bottom edge of the anchor point: centre y = 240 + 48 - 28.
        assert_eq!(block.centre, (484.0, 260.0));
        assert!(
            find(&list, ShoreSlot::Trickle).is_none(),
            "no crumbs before the first hit"
        );
        assert!(find(&list, ShoreSlot::Impact).is_none());
        assert_eq!(find(&list, ShoreSlot::Post).unwrap().rect, [0, 0, 136, 144]);
        // Each hit swaps the post and the block (the tell).
        for (hits, post, block) in [
            (1, [136, 0, 136, 144], [200, 144, 64, 56]),
            (2, [272, 0, 136, 144], [264, 144, 64, 56]),
            (3, [0, 144, 136, 144], [328, 144, 72, 64]),
        ] {
            let list = art.draw_list(&shore(hits), None, 3);
            assert_eq!(
                find(&list, ShoreSlot::Post).unwrap().rect,
                post,
                "hits {hits}"
            );
            assert_eq!(
                find(&list, ShoreSlot::Block).unwrap().rect,
                block,
                "hits {hits}"
            );
            assert_eq!(
                find(&list, ShoreSlot::Trickle).is_some(),
                hits < 3,
                "crumbs from the first hit until it falls, hits {hits}"
            );
        }
    }

    #[test]
    fn the_block_falls_with_the_simulated_height_and_lands_on_the_floor_line() {
        let art = art();
        let mut s = shore(3);
        s.block_bottom = 70.0;
        assert_eq!(s.block_state(), BlockState::Falling);
        let falling = find(&art.draw_list(&s, None, 0), ShoreSlot::Block).unwrap();
        assert_eq!(falling.rect, [328, 144, 72, 64]);
        // y of the anchor (the tipped block's bottom centre) follows block_bottom.
        let lower = {
            let mut t = s.clone();
            t.block_bottom = 30.0;
            find(&art.draw_list(&t, None, 0), ShoreSlot::Block).unwrap()
        };
        assert_eq!(falling.centre.1 - lower.centre.1, 40.0);
        assert!(find(&art.draw_list(&s, None, 0), ShoreSlot::Trickle).is_none());
        // Landed: the rubble sits on the walking line, with four dust frames.
        s.landed = true;
        s.block_bottom = 0.0;
        for (age, frame) in [(0, 0), (4, 0), (5, 1), (19, 3)] {
            let list = art.draw_list(&s, Some(age), 0);
            assert_eq!(
                find(&list, ShoreSlot::Block).unwrap().rect,
                [400, 144, 96, 32]
            );
            assert_eq!(
                find(&list, ShoreSlot::Impact).unwrap().rect[0],
                112 * frame,
                "age {age}"
            );
        }
        assert!(
            find(&art.draw_list(&s, Some(20), 0), ShoreSlot::Impact).is_none(),
            "the dust is over"
        );
    }
}
