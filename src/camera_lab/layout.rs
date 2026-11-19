//! Compose the render layout of a lab room from the **existing approved Floor 01
//! env-kit pieces** (no new art): an `env-kit-layout-v2` document that the
//! existing [`EnvKit`] loader and the A1 lighting contract consume unchanged.
//!
//! Composition (all of it presentation; collision is the room's own data):
//!
//! * a ceiling and cornice at the very top, wall columns at both ends and a deep
//!   floor on the room's walking line, with platforms and the authored access
//!   steps drawn from the deck / face / fill tiles;
//! * the whole wall between ceiling and floor is masonry backwall, taller bands
//!   of `brick_upper` / `brick_mid` for tall rooms;
//! * stacked arch bays (pier, arch ring, pier) tier on tier from the floor up:
//!   the ground tier opens onto the parallax gallery, the upper tiers are blind
//!   arcades, so the architecture above the action is genuinely drawn, with
//!   torches and pools of light at every tier, chains, pilasters and pipes;
//! * per setting the same kit pieces are re-arranged (pipes and valves for the
//!   workshop, rubble for the hoarder lair, a denser arcade for the gym and the
//!   arena). Workshop, gym, hoarder lair and swine vault have **no dedicated
//!   approved art**; they are greybox compositions of Floor 01 pieces.

use std::path::Path;

use serde_json::{Value, json};

use super::room::{Room, Setting, TILE, WALL_PX};
use crate::env_kit::{Contract, EnvError, EnvKit, Layout};

pub const CATACOMB_CONTRACT: &str = "assets/source/terrain/floor01/floor01_catacomb_kit.json";
pub const OPENING_CONTRACT: &str = "assets/source/terrain/floor01/floor01_opening_kit.json";
pub const BASE_LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";
/// Height of one arch bay tier (pier / ring) in px.
pub const BAY_H: i32 = 272;
/// Width of one arch bay, pier to pier, and the gap between bays.
pub const BAY_W: i32 = 304;
const BAY_PITCH: i32 = 368;

fn read_json(root: &Path, rel: &str) -> Result<Value, EnvError> {
    let path = root.join(rel);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| EnvError::Io(format!("{}: {e}", path.display())))?;
    serde_json::from_str(&text).map_err(|e| EnvError::Parse(format!("{}: {e}", path.display())))
}

/// The catacomb contract merged with the opening-kit extension (the loader in
/// `env_kit` reads one flat contract; `scripts/env_kit.py` does the same merge).
pub fn merged_contract(root: &Path) -> Result<Contract, EnvError> {
    let mut base = read_json(root, CATACOMB_CONTRACT)?;
    let ext = read_json(root, OPENING_CONTRACT)?;
    for key in ["atlases"] {
        if let (Some(b), Some(e)) = (base[key].as_object_mut(), ext[key].as_object()) {
            for (k, v) in e {
                b.insert(k.clone(), v.clone());
            }
        }
    }
    for key in ["tiles", "pieces"] {
        if let (Some(b), Some(e)) = (base[key].as_array_mut(), ext[key].as_array()) {
            b.extend(e.iter().cloned());
        }
    }
    serde_json::from_value(base).map_err(|e| EnvError::Parse(e.to_string()))
}

fn hash(a: i32, b: i32, c: i32) -> u32 {
    let mut h = 0x811c_9dc5_u32;
    for v in [a, b, c] {
        h ^= v as u32;
        h = h.wrapping_mul(0x0100_0193);
        h ^= h >> 15;
    }
    h
}

fn letter(a: i32, b: i32, c: i32, n: u32) -> char {
    (b'a' + (hash(a, b, c) % n) as u8) as char
}

/// Rows and columns of a layout.
struct Grid {
    cols: usize,
    rows: usize,
    cells: Vec<Vec<Option<String>>>,
}

impl Grid {
    fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            cells: vec![vec![None; cols]; rows],
        }
    }
    fn set(&mut self, col: i32, row: i32, id: String) {
        if col >= 0 && row >= 0 && (col as usize) < self.cols && (row as usize) < self.rows {
            self.cells[row as usize][col as usize] = Some(id);
        }
    }
    fn clear(&mut self, col: i32, row: i32) {
        if col >= 0 && row >= 0 && (col as usize) < self.cols && (row as usize) < self.rows {
            self.cells[row as usize][col as usize] = None;
        }
    }
    fn to_value(&self) -> Value {
        json!(self.cells)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bay {
    pub x0: i32,
    pub tier: i32,
    pub y0: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct Torch {
    pub x: i32,
    pub y: i32,
}

/// What the composition decided, for tests and the binary.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub bays: Vec<Bay>,
    pub torches: Vec<Torch>,
    pub tiers: i32,
    /// Tier-0 arch openings (x, y, w, h): backwall is empty there and the
    /// parallax gallery shows.
    pub openings: Vec<(i32, i32, i32, i32)>,
}

/// Opening mask of the ground arch relative to the bay: (rel row, first rel col,
/// last rel col), from the approved sample corridor.
const OPENING: [(i32, i32, i32); 14] = [
    (2, 5, 11),
    (3, 4, 12),
    (4, 3, 13),
    (5, 3, 13),
    (6, 2, 14),
    (7, 2, 14),
    (8, 2, 14),
    (9, 2, 14),
    (10, 2, 14),
    (11, 2, 14),
    (12, 2, 14),
    (13, 2, 14),
    (14, 2, 14),
    (15, 2, 14),
];

pub fn compose(room: &Room, base_layout: &Value) -> (Value, Plan) {
    let (w, h) = room.render;
    let cols = (w / TILE) as usize;
    let rows = (h / TILE) as usize;
    let fr = (room.floor_y - 4) / TILE; // deck_front row
    let mut plan = Plan::default();

    // ------------------------------------------------------------ platforms
    // Columns / rows occupied by platforms and steps (the ground bays avoid them).
    let mut blocked_x: Vec<(i32, i32)> = Vec::new();
    for p in &room.platforms {
        blocked_x.push((p.rect.x, p.rect.right()));
    }
    for s in &room.steps {
        blocked_x.push((s.x, s.right()));
    }
    for p in &room.props {
        blocked_x.push((p.rect.x, p.rect.right()));
    }

    // ------------------------------------------------------------ arch bays
    let tiers = ((fr * TILE - 2 * TILE) / BAY_H).max(1);
    plan.tiers = tiers;
    // Ground bays sit in the gaps between platforms and steps (props are drawn
    // in front of the structure layer, so they do not block a bay).
    let mut solid_x: Vec<(i32, i32)> = room
        .platforms
        .iter()
        .map(|p| (p.rect.x, p.rect.right()))
        .chain(room.steps.iter().map(|s| (s.x, s.right())))
        .chain(room.solid_backwall.iter().copied())
        .collect();
    solid_x.sort_unstable();
    let mut gaps: Vec<(i32, i32)> = Vec::new();
    let mut at = WALL_PX;
    for (a, b) in solid_x {
        if a > at {
            gaps.push((at, a));
        }
        at = at.max(b);
    }
    gaps.push((at, w - WALL_PX));
    let mut ground_bays: Vec<i32> = Vec::new();
    for (a, b) in gaps {
        // Centre as many bays as fit in the gap.
        let n = (b - a + 32) / (BAY_W + 32);
        if n < 1 {
            continue;
        }
        let used = n * BAY_W + (n - 1) * 32;
        let first = a + (b - a - used) / 2;
        for k in 0..n {
            ground_bays.push((first + k * (BAY_W + 32)) / TILE * TILE);
        }
    }
    // Upper tiers continue over the whole width on the same 368 px pitch plus the
    // ground columns, so the arcade reads as one structure.
    let mut upper_x: Vec<i32> = ground_bays.clone();
    let mut ux = 48;
    while ux + BAY_W <= w - 48 {
        if !upper_x.iter().any(|g| (g - ux).abs() < BAY_W + 16) {
            upper_x.push(ux);
        }
        ux += BAY_PITCH;
    }
    upper_x.sort_unstable();
    for &bx in &upper_x {
        for tier in 0..tiers {
            if tier == 0 && !ground_bays.contains(&bx) {
                continue;
            }
            plan.bays.push(Bay {
                x0: bx,
                tier,
                y0: fr * TILE - BAY_H * (tier + 1),
            });
        }
    }

    // ------------------------------------------------------------- backwall
    let mut backwall = Grid::new(cols, rows);
    let extra = (fr - 19).max(0);
    let upper_rows = 4 + (extra * 6 + 5) / 10;
    let upper_end = 4 + upper_rows; // exclusive
    for row in 2..(fr - 1) {
        let (band, hi) = if row < 4 {
            ("brick_top", 4)
        } else if row < upper_end {
            ("brick_upper", 4)
        } else if row < fr - 6 {
            ("brick_mid", 4)
        } else if row < fr - 4 {
            ("brick_lower", 4)
        } else if row < fr - 2 {
            ("brick_low", 4)
        } else {
            ("base_course", 2)
        };
        for col in 1..(cols as i32 - 1) {
            let run = (col + 3) / 4;
            let k = ((col + 3) % 4) + 1;
            let l = if band == "base_course" {
                letter(run, row, 1, hi)
            } else {
                letter(run, row, 2, hi)
            };
            backwall.set(col, row, format!("{band}_{l}_{k}"));
        }
    }
    // Ground arch openings show the parallax gallery.
    for &bx in &ground_bays {
        let c0 = bx / TILE;
        let b_row = fr - 16; // bay top row (sample: bay row 2, deck row 19 -> 19 - 17)
        let b_row = b_row + 1;
        for &(rel_row, a, z) in &OPENING {
            for rel_col in a..=z {
                backwall.clear(c0 + rel_col, b_row + rel_row - 1);
            }
        }
        plan.openings
            .push((bx + 2 * TILE, (b_row + 1) * TILE, 13 * TILE, 14 * TILE));
    }

    // ------------------------------------------------------------ collision
    let mut col_layer = Grid::new(cols, rows);
    let last = cols as i32 - 1;
    for r in 0..rows as i32 {
        let odd = r % 2 == 1;
        col_layer.set(0, r, if odd { "wall_end_l_b" } else { "wall_end_l" }.into());
        col_layer.set(
            last,
            r,
            if odd { "wall_end_r_b" } else { "wall_end_r" }.into(),
        );
    }
    for col in 1..last {
        let k = ((col + 2) % 4) + 1;
        col_layer.set(col, 0, format!("ceiling_fill_a_{k}"));
        let cl = if (col / 4) % 2 == 0 { 'a' } else { 'b' };
        col_layer.set(col, 1, format!("cornice_{cl}_{k}"));
    }
    col_layer.set(1, 0, "ceiling_fill_s".into());
    col_layer.set(last - 1, 0, "ceiling_fill_s".into());
    col_layer.set(1, 1, "ceiling_corner_in_l".into());
    col_layer.set(last - 1, 1, "ceiling_corner_in_r".into());

    // Torches: on the ground piers of the bays and between them, every tier.
    // (Light budget: the ground tier carries two torches per bay, every upper
    // tier one on every other bay. Each lit torch is two emitters, the A1 pass
    // holds 64 per view.)
    for (index, bay) in plan.bays.iter().enumerate() {
        let bottom = bay.y0 + BAY_H;
        let xs: &[i32] = if bay.tier == 0 {
            &[24, 264]
        } else if (bay.x0 / BAY_PITCH) % 2 == 0 && index % 2 == 0 {
            &[24]
        } else {
            &[]
        };
        for dx in xs {
            plan.torches.push(Torch {
                x: bay.x0 + dx,
                y: bottom - 128,
            });
        }
    }
    // Free torches on the ground tier midway between neighbouring bays and at the
    // room ends.
    let mut anchors: Vec<i32> = vec![WALL_PX + 40];
    for pair in ground_bays.windows(2) {
        anchors.push((pair[0] + BAY_W + pair[1]) / 2);
    }
    anchors.push(w - WALL_PX - 40);
    for tx in anchors {
        if blocked_x
            .iter()
            .any(|&(a, b)| a < tx + 24 && b > tx - 24 && a != b)
        {
            continue;
        }
        plan.torches.push(Torch {
            x: tx,
            y: fr * TILE - 144,
        });
    }
    plan.torches.sort_by_key(|t| (t.x, t.y));
    plan.torches.dedup_by_key(|t| (t.x, t.y));

    // Floor lighting variant by distance to the nearest ground torch.
    let ground_torch_xs: Vec<i32> = plan
        .torches
        .iter()
        .filter(|t| t.y >= fr * TILE - 200)
        .map(|t| t.x)
        .collect();
    let light_of = |x: i32| -> &'static str {
        let d = ground_torch_xs
            .iter()
            .map(|tx| (tx - x).abs())
            .min()
            .unwrap_or(9999);
        if d <= 96 {
            "lit"
        } else if d <= 300 {
            "mid"
        } else {
            "dim"
        }
    };
    for col in 2..last {
        let k = ((col + 2) % 4) + 1;
        let x = col * TILE + 8;
        let lit = light_of(x);
        let run = (col + 2) / 4;
        let ab = letter(run, 1, 3, 2);
        col_layer.set(col, fr - 1, format!("deck_back_{lit}_{ab}_{k}"));
        col_layer.set(col, fr, format!("deck_front_{lit}_{ab}_{k}"));
        col_layer.set(col, fr + 1, format!("face_a_{lit}_a_{k}"));
        col_layer.set(
            col,
            fr + 2,
            format!("face_b_{}_{k}", if (run % 2) == 0 { "a" } else { "b" }),
        );
        for row in (fr + 3)..rows as i32 {
            let id = match row - fr {
                3 => "fill_1_j0_a",
                r if r % 2 == 0 => "fill_2_j0_a",
                _ => "fill_2_j16_a",
            };
            col_layer.set(col, row, format!("{id}_{k}"));
        }
    }
    col_layer.set(1, fr - 1, "deck_back_wall_l".into());
    col_layer.set(1, fr, "deck_front_wall_l".into());
    col_layer.set(1, fr + 1, "face_a_wall_l".into());
    col_layer.set(1, fr + 2, "face_b_wall_l".into());
    col_layer.set(1, fr + 3, "fill_wall_l_1".into());
    for row in (fr + 4)..rows as i32 {
        col_layer.set(1, row, "fill_wall_l_2".into());
    }

    // Platforms and steps as deck / face / fill stacks.
    let stack = |col_layer: &mut Grid, col: i32, top_row: i32, k: i32, end: Option<char>| {
        let kk = ((col + 2) % 4) + 1;
        let _ = k;
        let deck = match end {
            Some('l') => "deck_front_end_l".to_string(),
            Some('r') => "deck_front_end_r".to_string(),
            _ => format!("deck_front_mid_a_{kk}"),
        };
        col_layer.set(col, top_row, deck);
        let face = match end {
            Some('l') => "face_a_end_l".to_string(),
            Some('r') => "face_a_end_r".to_string(),
            _ => format!("face_a_mid_a_{kk}"),
        };
        col_layer.set(col, top_row + 1, face);
    };
    for p in &room.platforms {
        let top_row = (p.rect.y - 4) / TILE;
        let c0 = p.rect.x / TILE;
        let c1 = p.rect.right() / TILE;
        for col in c0..c1 {
            let end = if col == c0 {
                Some('l')
            } else if col == c1 - 1 {
                Some('r')
            } else {
                None
            };
            stack(&mut col_layer, col, top_row, 0, end);
        }
    }
    for s in &room.steps {
        let top_row = (s.y - 4) / TILE;
        let c0 = s.x / TILE;
        let c1 = s.right() / TILE;
        for col in c0..c1 {
            let kk = ((col + 2) % 4) + 1;
            col_layer.set(col, top_row, format!("deck_front_mid_a_{kk}"));
            for (n, row) in ((top_row + 1)..(fr - 1)).enumerate() {
                let id = if n % 2 == 0 {
                    format!("face_a_mid_a_{kk}")
                } else {
                    format!("face_b_b_{kk}")
                };
                col_layer.set(col, row, id);
            }
            col_layer.clear(col, fr - 1);
            col_layer.set(col, fr - 1, format!("face_b_b_{kk}"));
        }
    }

    // --------------------------------------------------------------- pieces
    let mut structure: Vec<Value> = Vec::new();
    let mut decal: Vec<Value> = Vec::new();
    let mut prop: Vec<Value> = Vec::new();
    let mut foreground: Vec<Value> = Vec::new();
    let place = |list: &mut Vec<Value>, piece: &str, x: i32, y: i32| {
        list.push(json!({"piece": piece, "x": x, "y": y}));
    };
    for bay in &plan.bays {
        place(&mut structure, "pier_l", bay.x0, bay.y0);
        place(
            &mut structure,
            if (bay.x0 / BAY_PITCH + bay.tier) % 2 == 0 {
                "arch_ring_a"
            } else {
                "arch_ring_b"
            },
            bay.x0 + 16,
            bay.y0,
        );
        place(&mut structure, "pier_r", bay.x0 + 224, bay.y0);
        place(&mut structure, "arch_threshold", bay.x0 + 64, bay.y0 + 240);
        place(&mut structure, "arch_chain", bay.x0 + 80, bay.y0 + 80);
    }
    // Pilasters under the cornice in the wall left over above the top tier.
    let top_free = fr * TILE - BAY_H * tiers - 2 * TILE;
    if top_free >= 96 {
        let mut px = 64 + BAY_PITCH / 2;
        while px + 32 <= w - 64 {
            let h = top_free.min(256);
            if h >= 96 {
                place(&mut structure, "pilaster_plain", px, 2 * TILE);
            }
            px += BAY_PITCH;
        }
    }
    for t in &plan.torches {
        let idx = prop.len() as i32;
        prop.push(
            json!({"piece": "torch_wall", "x": t.x - 8, "y": t.y, "frame_offset": (idx % 4)}),
        );
        let pool = match hash(t.x, t.y, 9) % 4 {
            0 => "torch_pool_a",
            1 => "torch_pool_b",
            2 => "torch_pool_a_m",
            _ => "torch_pool_b_m",
        };
        place(&mut decal, pool, t.x - 8 - 112 + 8, t.y - 112 + 28);
    }
    place(&mut decal, "grime_a", 600, fr * TILE - 200);
    place(&mut decal, "pier_spill_l", 48 + 8, fr * TILE - 280);
    // Exit: the guild portal as the protected exit marker (bottom on the line).
    place(
        &mut prop,
        "guild_portal_closed",
        room.exit.x - 24,
        room.floor_y - 136,
    );

    // Heat vents of the room data are drawn with the kit's hazard grit (the only
    // approved floor hazard piece) so the damage zone is never invisible.
    for hz in &room.hazards {
        place(
            &mut prop,
            "hazard_grit",
            hz.rect.x + hz.rect.w / 2 - 48,
            room.floor_y - 16,
        );
    }

    // Setting flavour: only existing pieces, re-arranged.
    match room.setting {
        Setting::Workshop => {
            let run_y = 3 * TILE;
            let mut x = 560;
            while x < 1500.min(w - 64) {
                place(
                    &mut prop,
                    if (x / TILE) % 7 == 0 {
                        "pipe_h_seam"
                    } else {
                        "pipe_h"
                    },
                    x,
                    run_y,
                );
                x += TILE;
            }
            place(&mut prop, "pipe_elbow_ld", 528, run_y - 16);
            for vx in [700, 1100, 1450] {
                let mut y = run_y + 16;
                while y < fr * TILE - 80 {
                    place(&mut prop, "pipe_v", vx, y);
                    y += TILE;
                }
                place(&mut prop, "valve_wheel", vx - 8, fr * TILE - 176);
                place(&mut prop, "pipe_collar_floor", vx - 8, fr * TILE - 70);
            }
            place(&mut prop, "maint_hatch", 900, fr * TILE - 100);
        }
        Setting::Hoarder => {
            let mut x = 420;
            while x < w - 300 {
                let piece = if (x / 64) % 2 == 0 {
                    "rubble_a"
                } else {
                    "rubble_b"
                };
                place(
                    &mut foreground,
                    piece,
                    x,
                    room.floor_y - if piece == "rubble_a" { 28 } else { 20 },
                );
                x += 150;
            }
            place(&mut decal, "wall_cavity", 900, fr * TILE - 330);
            place(&mut decal, "alcove_niche", 1300, fr * TILE - 260);
        }
        Setting::HeatCatacomb => {
            for gx in [300, 760, 1200] {
                place(&mut decal, "grime_b", gx, fr * TILE - 120);
            }
            place(&mut decal, "pier_spill_r", 1040, fr * TILE - 280);
        }
        Setting::Gym => {
            for px in [200, 900, 1500] {
                place(&mut prop, "door_timber", px, room.floor_y - 92);
            }
            place(&mut prop, "maint_hatch", 700, fr * TILE - 120);
        }
        Setting::Arena => {
            place(&mut decal, "alcove_niche", 400, fr * TILE - 260);
            place(&mut decal, "wall_cavity", 1600, fr * TILE - 330);
            place(&mut decal, "pier_spill_r", w - 200, fr * TILE - 280);
        }
        Setting::DryCatacomb => {
            place(&mut prop, "door_timber", 216, room.floor_y - 92);
        }
    }

    let mut layout = base_layout.clone();
    layout["layout_id"] = json!(format!("camera_lab_{}", room.id.to_lowercase()));
    layout["kit_contract"] = json!(OPENING_CONTRACT);
    layout["status"] = json!(
        "camera lab greybox composition from approved Floor 01 pieces; not authored level art"
    );
    layout["grid_size"] = json!([cols, rows]);
    layout["pixel_size"] = json!([w, h]);
    layout["base_walk_line_y"] = json!(room.floor_y);
    layout["segments"] = json!([]);
    layout["markers"] = json!([
        {"id": "player_spawn", "kind": "spawn", "x": room.spawn_carl, "y": room.floor_y - 1},
        {"id": "exit", "kind": "exit", "x": room.goal_x, "y": room.floor_y - 1,
         "rect": [room.exit.x, room.exit.y, room.exit.w, room.exit.h]},
    ]);
    layout["views"] = json!({"lab": [0, 0, 640, 360]});
    let gy = plan.openings.first().map_or(60, |o| o.1 - 4);
    layout["parallax_layers"] = json!({
        "parallax_far": {"piece": "gallery_far", "x_origin": 115, "y": gy},
        "parallax_mid": {"piece": "gallery_mid", "x_origin": 39, "y": gy},
    });
    layout["tile_layers"] = json!({
        "backwall": backwall.to_value(),
        "collision": col_layer.to_value(),
    });
    layout["piece_layers"] = json!({
        "backwall_decal": decal,
        "structure": structure,
        "prop": prop,
        "foreground": foreground,
    });
    // Floor fog of the base profile is in layout y of the 400 px corridor:
    // move it to this room's floor.
    if let Some(groups) = layout["lighting"]["groups"].as_object_mut() {
        for (_, g) in groups.iter_mut() {
            if let Some(fog) = g
                .get_mut("haze")
                .and_then(|h| h.get_mut("floor_fog"))
                .and_then(Value::as_object_mut)
            {
                let from = room.floor_y - 62;
                let to = room.floor_y - 20;
                fog.insert("from_y".into(), json!(from));
                fog.insert("to_y".into(), json!(to));
            }
        }
    }
    (layout, plan)
}

/// Build the env kit of a room (contract merge, composition, loader).
pub fn build_kit(root: &Path, room: &Room) -> Result<(EnvKit, Contract, Plan), EnvError> {
    let contract = merged_contract(root)?;
    let base = read_json(root, BASE_LAYOUT)?;
    let (value, plan) = compose(room, &base);
    let layout: Layout =
        serde_json::from_value(value).map_err(|e| EnvError::Parse(e.to_string()))?;
    let kit = EnvKit::build(&contract, &layout)?;
    Ok((kit, contract, plan))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_lab::room::load_rooms;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn every_room_composes_and_loads_through_the_existing_env_kit() {
        for room in load_rooms(root()).unwrap() {
            let (kit, _, plan) =
                build_kit(root(), &room).unwrap_or_else(|e| panic!("{}: {e}", room.id));
            assert_eq!((kit.width, kit.height), room.render);
            assert!(plan.tiers >= 2, "{}: tiers {}", room.id, plan.tiers);
            assert!(kit.lighting.is_some(), "{}: lighting profile", room.id);
            assert!(!plan.torches.is_empty());
        }
    }

    /// The headline requirement: no camera position at any coverage, 16:9 or
    /// 16:10, shows an undrawn cell. Every cell between the ceiling row and the
    /// floor bottom holds a backwall tile, a collision tile or the parallax
    /// gallery behind a ground arch opening; the gallery strips span the openings.
    #[test]
    fn the_whole_render_bounds_is_drawn_so_no_view_shows_void() {
        for room in load_rooms(root()).unwrap() {
            let (kit, _contract, plan) = build_kit(root(), &room).unwrap();
            let cols = (kit.width / TILE) as usize;
            let rows = (kit.height / TILE) as usize;
            let mut drawn = vec![vec![false; cols]; rows];
            for t in &kit.tiles {
                drawn[t.row as usize][t.col as usize] = true;
            }
            let in_opening = |c: usize, r: usize| {
                let (x, y) = (c as i32 * TILE + 8, r as i32 * TILE + 8);
                plan.openings
                    .iter()
                    .any(|&(ox, oy, ow, oh)| x >= ox && x < ox + ow && y >= oy && y < oy + oh)
            };
            let mut holes = Vec::new();
            for (r, row) in drawn.iter().enumerate() {
                for (c, &ok) in row.iter().enumerate() {
                    if !ok && !in_opening(c, r) {
                        holes.push((c, r));
                    }
                }
            }
            assert!(
                holes.is_empty(),
                "{}: {} undrawn cells, first {:?}",
                room.id,
                holes.len(),
                &holes[..holes.len().min(5)]
            );
            // The gallery strips cover the openings vertically (y 60.. of a 232 px strip).
            for &(_, oy, _, oh) in &plan.openings {
                for s in &kit.strips {
                    assert!(
                        s.y <= oy && s.y + s.rect.h >= oy + oh - 8,
                        "{}: strip {} does not cover an opening",
                        room.id,
                        s.piece
                    );
                }
            }
            // Architecture above the action: tiers of bays reach the cornice.
            let top_bay = plan.bays.iter().map(|b| b.y0).min().unwrap();
            assert!(
                top_bay - 2 * TILE < BAY_H,
                "{}: top bay at {}",
                room.id,
                top_bay
            );
        }
    }

    #[test]
    fn composition_is_deterministic() {
        let rooms = load_rooms(root()).unwrap();
        let base = read_json(root(), BASE_LAYOUT).unwrap();
        for room in &rooms {
            let a = compose(room, &base).0;
            let b = compose(room, &base).0;
            assert_eq!(a, b);
        }
    }

    #[test]
    fn torch_budget_stays_inside_the_light_pass_in_the_widest_view() {
        use crate::lighting::{MAX_LIGHTS, ViewRect};
        for room in load_rooms(root()).unwrap() {
            let (kit, _, _) = build_kit(root(), &room).unwrap();
            let scene = kit.lighting.as_ref().unwrap();
            let mut worst = 0;
            let mut left = 0;
            while left + 1120 <= kit.width {
                let mut top = 0;
                while top + 700 <= kit.height {
                    let (g, dropped) = scene.gather(
                        &kit,
                        ViewRect {
                            left,
                            top,
                            w: 1120,
                            h: 700,
                        },
                        0,
                    );
                    assert_eq!(dropped, 0, "{}: lights dropped at {left},{top}", room.id);
                    worst = worst.max(g.lights.len());
                    top += 160;
                }
                left += 160;
            }
            assert!(worst <= MAX_LIGHTS, "{}: {worst} lights", room.id);
        }
    }
}
