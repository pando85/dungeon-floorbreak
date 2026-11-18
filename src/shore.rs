//! CP-03: the ENV-02 rotten shore and its falling block, as authored,
//! localized, deterministic gameplay (`docs/02-art/env-02-opening-spec.md`,
//! "CP-03 breakable"). Bevy-free; it is **not** a general terrain collapse
//! solver.
//!
//! Rules (all from the frozen ENV-02 spec and `opening_route_proposal.json`):
//!
//! * Prop states `intact -> damaged_1 -> damaged_2 -> broken`: one step per
//!   **melee move instance** whose hit box overlaps the strike zone (held
//!   inputs and hit-stop never repeat it); [`SHORE_HITS_TO_BREAK`] = 3.
//! * Block states `seated -> settling -> slipped -> falling -> landed`: the
//!   first two hits swap the block to `settling` and `slipped`, the break
//!   starts a [`BLOCK_FALL_DELAY_TICKS`] delay and then a straight fall of
//!   [`BLOCK_FALL_HEIGHT`] px under [`BLOCK_GRAVITY`], integrated per tick.
//! * One landing, one evaluation: every living goblin whose body overlaps the
//!   hazard footprint on the landing tick is defeated, credited to the player
//!   who broke the shore. Outside the footprint, or with the shore unbroken,
//!   nothing is defeated.
//! * The landed block is **non-collidable** (spec): this module never touches
//!   the solids, so the route stays walkable in every event order.
//!
//! Positions are relative to the strike column, so the same rules run on the
//! opening route (strike x 2975) and on the fixtures' clean ground. A unit test
//! pins the constants to the marker rects of the route layout.

use crate::prototype::Rect;

/// Melee hits that break the shore.
pub const SHORE_HITS_TO_BREAK: u8 = 3;
/// Ticks between the break and the first tick of the fall.
pub const BLOCK_FALL_DELAY_TICKS: u8 = 6;
/// The fall: bottom of the block from 140 px above the walking line to it.
pub const BLOCK_FALL_HEIGHT: f32 = 140.0;
pub const BLOCK_GRAVITY: f32 = 1050.0;
/// Strike zone half width around the strike column and its height band above
/// the feet (layout y 271..290 over the walking line 308 = 18..37 px).
pub const STRIKE_HALF_WIDTH: f32 = 8.0;
pub const STRIKE_Y_MIN: f32 = 18.0;
pub const STRIKE_Y_MAX: f32 = 37.0;
/// Hazard footprint relative to the strike column (layout 3000..3080 around
/// strike 2975), full fall height.
pub const FOOTPRINT_X_MIN: f32 = 25.0;
pub const FOOTPRINT_WIDTH: f32 = 80.0;
/// The player slot credited with the break (single-player slot 0 for now; the
/// 2P ownership of the hit is **not proven** here).
pub const PLAYER_ONE: u32 = 0;

const TICK_SECONDS: f32 = 1.0 / 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShoreState {
    Intact,
    Damaged1,
    Damaged2,
    Broken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockState {
    Seated,
    Settling,
    Slipped,
    Falling,
    Landed,
}

/// What one tick of the shore did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ShoreStep {
    /// The block touched down this tick (the one impact evaluation).
    pub landed: bool,
}

/// What a strike did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShoreStrike {
    pub hits: u8,
    pub broke: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Shore {
    /// The strike column (centre x) and the feet height of the walking line.
    pub strike_x: f32,
    pub foot_y: f32,
    pub hits: u8,
    last_instance: Option<u32>,
    /// Ticks of the pre-fall delay left (counts down from the break).
    delay: u8,
    /// Height of the block's bottom above the walking line while it falls.
    pub block_bottom: f32,
    pub block_vy: f32,
    pub landed: bool,
    /// The player who landed the breaking hit.
    pub breaker: Option<u32>,
}

impl Shore {
    pub fn new(strike_x: f32, foot_y: f32) -> Self {
        Self {
            strike_x,
            foot_y,
            hits: 0,
            last_instance: None,
            delay: 0,
            block_bottom: BLOCK_FALL_HEIGHT,
            block_vy: 0.0,
            landed: false,
            breaker: None,
        }
    }

    pub fn state(&self) -> ShoreState {
        match self.hits {
            0 => ShoreState::Intact,
            1 => ShoreState::Damaged1,
            2 => ShoreState::Damaged2,
            _ => ShoreState::Broken,
        }
    }

    pub fn block_state(&self) -> BlockState {
        if self.landed {
            BlockState::Landed
        } else if self.hits >= SHORE_HITS_TO_BREAK {
            if self.delay > 0 {
                BlockState::Slipped
            } else {
                BlockState::Falling
            }
        } else {
            match self.hits {
                0 => BlockState::Seated,
                1 => BlockState::Settling,
                _ => BlockState::Slipped,
            }
        }
    }

    /// Where a melee box hits the shore.
    pub fn strike_zone(&self) -> Rect {
        Rect {
            left: self.strike_x - STRIKE_HALF_WIDTH,
            bottom: self.foot_y + STRIKE_Y_MIN,
            width: STRIKE_HALF_WIDTH * 2.0,
            height: STRIKE_Y_MAX - STRIKE_Y_MIN,
        }
    }

    /// The hazard footprint: evaluated once, on the landing tick.
    pub fn footprint(&self) -> Rect {
        Rect {
            left: self.strike_x + FOOTPRINT_X_MIN,
            bottom: self.foot_y,
            width: FOOTPRINT_WIDTH,
            height: BLOCK_FALL_HEIGHT,
        }
    }

    /// Offer an active melee hit box of move `instance` by `player`. Counts at
    /// most once per instance, never after the break. `None` when nothing
    /// changed.
    pub fn strike(&mut self, instance: u32, hit_box: Rect, player: u32) -> Option<ShoreStrike> {
        if self.hits >= SHORE_HITS_TO_BREAK
            || self.last_instance == Some(instance)
            || !hit_box.overlaps(self.strike_zone())
        {
            return None;
        }
        self.last_instance = Some(instance);
        self.hits += 1;
        let broke = self.hits >= SHORE_HITS_TO_BREAK;
        if broke {
            self.delay = BLOCK_FALL_DELAY_TICKS;
            self.breaker = Some(player);
        }
        Some(ShoreStrike {
            hits: self.hits,
            broke,
        })
    }

    /// One fixed tick of the block: the pre-fall delay, then the fall. Reports
    /// the landing exactly once.
    pub fn step(&mut self) -> ShoreStep {
        let mut out = ShoreStep::default();
        if self.hits < SHORE_HITS_TO_BREAK || self.landed {
            return out;
        }
        if self.delay > 0 {
            self.delay -= 1;
            return out;
        }
        self.block_vy += BLOCK_GRAVITY * TICK_SECONDS;
        self.block_bottom -= self.block_vy * TICK_SECONDS;
        if self.block_bottom <= 0.0 {
            self.block_bottom = 0.0;
            self.landed = true;
            out.landed = true;
        }
        out
    }

    /// Fold the full state into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(u64::from(self.strike_x.to_bits()));
        mix(u64::from(self.hits));
        mix(self.last_instance.map_or(0, |i| u64::from(i) + 1));
        mix(u64::from(self.delay));
        mix(u64::from(self.block_bottom.to_bits()));
        mix(u64::from(self.block_vy.to_bits()));
        mix(u64::from(self.landed));
        mix(self.breaker.map_or(0, |p| u64::from(p) + 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jab_box(shore: &Shore) -> Rect {
        Rect {
            left: shore.strike_x - 3.0,
            bottom: shore.foot_y + 29.0,
            width: 6.0,
            height: 6.0,
        }
    }

    #[test]
    fn constants_match_the_route_layout_markers() {
        let text = std::fs::read_to_string("assets/levels/floor01/opening_route_proposal.json")
            .expect("route layout");
        let layout: serde_json::Value = serde_json::from_str(&text).expect("json");
        let marker = |id: &str| {
            layout["markers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| m["id"] == id)
                .unwrap_or_else(|| panic!("marker {id}"))
                .clone()
        };
        let num = |v: &serde_json::Value| v.as_f64().unwrap() as f32;
        let shore = marker("breakable_shore");
        let hazard = marker("hazard_fall_region");
        let line = num(&layout["base_walk_line_y"]);
        let strike_x = num(&shore["x"]);
        let rect = hazard["rect"].as_array().unwrap();
        assert_eq!(num(&rect[0]) - strike_x, FOOTPRINT_X_MIN);
        assert_eq!(num(&rect[2]), FOOTPRINT_WIDTH);
        assert_eq!(num(&rect[3]), BLOCK_FALL_HEIGHT);
        assert_eq!(shore["hits_to_break"], SHORE_HITS_TO_BREAK);
        let band = shore["strike_band_y"].as_array().unwrap();
        assert_eq!(line - num(&band[1]), STRIKE_Y_MIN);
        assert_eq!(line - num(&band[0]), STRIKE_Y_MAX);
        // Land y of the falling block is the walking line.
        assert_eq!(num(&marker("falling_block")["land_y"]), line);
    }

    #[test]
    fn three_distinct_instances_break_it_and_a_repeat_never_counts() {
        let mut shore = Shore::new(100.0, 50.0);
        let hit = jab_box(&shore);
        assert_eq!(shore.state(), ShoreState::Intact);
        assert_eq!(shore.block_state(), BlockState::Seated);
        // The same move instance over many active ticks counts once.
        assert_eq!(shore.strike(1, hit, 0).map(|s| s.hits), Some(1));
        assert_eq!(shore.strike(1, hit, 0), None);
        assert_eq!(shore.state(), ShoreState::Damaged1);
        assert_eq!(shore.block_state(), BlockState::Settling);
        assert_eq!(shore.strike(2, hit, 0).map(|s| s.hits), Some(2));
        assert_eq!(shore.block_state(), BlockState::Slipped);
        let last = shore.strike(3, hit, 0).unwrap();
        assert!(last.broke);
        assert_eq!(shore.state(), ShoreState::Broken);
        assert_eq!(shore.breaker, Some(0));
        // Broken: nothing advances it further.
        assert_eq!(shore.strike(4, hit, 0), None);
    }

    #[test]
    fn a_box_outside_the_strike_zone_does_nothing() {
        let mut shore = Shore::new(100.0, 50.0);
        let mut miss = jab_box(&shore);
        miss.left += 30.0;
        assert_eq!(shore.strike(1, miss, 0), None);
        let mut low = jab_box(&shore);
        low.bottom = 50.0;
        low.height = 10.0;
        assert_eq!(shore.strike(2, low, 0), None);
        assert_eq!(shore.hits, 0);
    }

    #[test]
    fn the_block_falls_after_the_delay_and_lands_exactly_once() {
        let mut shore = Shore::new(100.0, 50.0);
        let hit = jab_box(&shore);
        for instance in 1..=3 {
            shore.strike(instance, hit, 0);
        }
        let mut landed = Vec::new();
        for tick in 1..=80 {
            if shore.step().landed {
                landed.push(tick);
            }
        }
        assert_eq!(landed.len(), 1, "one impact");
        // 6 ticks of delay, then about 0.52 s (32 ticks) of fall.
        assert!(
            (36..=42).contains(&landed[0]),
            "landed on tick {}",
            landed[0]
        );
        assert_eq!(shore.block_state(), BlockState::Landed);
        assert_eq!(shore.block_bottom, 0.0);
    }

    #[test]
    fn an_unbroken_shore_never_drops_the_block() {
        let mut shore = Shore::new(100.0, 50.0);
        let hit = jab_box(&shore);
        shore.strike(1, hit, 0);
        shore.strike(2, hit, 0);
        for _ in 0..200 {
            assert!(!shore.step().landed);
        }
        assert_eq!(shore.block_state(), BlockState::Slipped);
    }
}
