//! ENEMY-GAMEPLAY-ENGINEER: the Goblin Engineer's knobs and Bevy-free domain
//! (`docs/01-design/enemy-gameplay-engineer.md`, ENG-G-Q01..Q27).
//!
//! The Engineer is a goblin of [`crate::goblin::GoblinKind::Engineer`]: it lives in
//! the scene's goblin list, shares the goblins' reactions to hits and the attack
//! tokens, and differs in its brain and its attack. This module owns everything
//! that does not need the scene:
//!
//! * [`EngineerTuning`]: every `engq##` knob of
//!   `assets/data/gameplay/enemy_engineer_v0.json`, loaded, validated and FNV-1a
//!   pinned like `cf02_combat_v0.json`;
//! * the attack timeline ([`EngineerPhase`], [`EngineerState`]): rear, point,
//!   launch, ram, recover, bonk;
//! * the geometry of the ram ([`ram_box`], [`ram_motion`]) and of the lane check
//!   ([`lane_clear`]);
//! * [`EngineerSense`], the only thing the Engineer's step reads of the scene.
//!
//! The Engineer exists only when `cf02_rules` is on, the `--cf02-engineer` flag is
//! given (a fixture with engineers) **and** the data switch `engq25_enabled` is on.
//! Every switch is read through [`EngineerTuning::on`], never raw.

use crate::cf02::BoxPx;
use crate::combat::ENEMY_HALF_WIDTH;
use crate::donut_mana::content_hash;
use crate::goblin::AttackPhase;
use crate::party::HeroId;
use crate::prototype::{Block, Rect};
use serde::{Deserialize, Serialize};

/// The tuning file, embedded so the binary and the tests read the same bytes.
pub const TUNING_JSON: &str = include_str!("../assets/data/gameplay/enemy_engineer_v0.json");

/// The committed FNV-1a 64 content hash of [`TUNING_JSON`]. Editing the file
/// without updating this value fails [`EngineerTuning::pinned`] and the pin test.
pub const TUNING_HASH: u64 = 0x5b8d_49f4_e8de_fa5f;

/// ENG-G questions that have no key in the file, and why. The completeness test
/// requires every other ID in 1..=27 to own at least one `engqNN_` key.
pub const NO_KNOB: [(u8, &str); 9] = [
    (1, "design: the role of the sheet"),
    (
        16,
        "code: the tokens are the goblins' MAX_ATTACKERS and tell spacing",
    ),
    (17, "reuses the CF02 q31_* knobs"),
    (20, "reuses the goblin's hit reactions"),
    (
        21,
        "data: the goblin_engineer species in species_death_profiles_cf02.json",
    ),
    (
        22,
        "code: the kind-aware phase mapping of the companion view",
    ),
    (23, "reuses the goblin tell cue"),
    (
        24,
        "process: ENG-02 is ordered after the owner's game-feel review",
    ),
    (26, "reuses the CF02-Q58 bag; the fifth slot is unused"),
];

/// A knob file problem. A load error names the field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineerTuningError {
    Parse(String),
    Field { field: &'static str, reason: String },
    HashMismatch { expected: u64, actual: u64 },
}

impl core::fmt::Display for EngineerTuningError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "engineer tuning: {msg}"),
            Self::Field { field, reason } => {
                write!(f, "engineer tuning: field `{field}`: {reason}")
            }
            Self::HashMismatch { expected, actual } => write!(
                f,
                "engineer tuning hash {actual:#018x} is not the pinned {expected:#018x}"
            ),
        }
    }
}

impl std::error::Error for EngineerTuningError {}

/// Every ENG-G knob (`docs/01-design/enemy-gameplay-engineer.md`, section 12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineerTuning {
    pub schema_version: u32,
    /// Q02: health.
    pub engq02_health: u8,
    /// Q03: the ram box, pivot-relative, x forward, y up from the feet. Its front
    /// (`x1`) is what meets a solid.
    pub engq03_ram_box: BoxPx,
    /// Q04: walk speed in px/s and sight in px.
    pub engq04_walk_speed: u16,
    pub engq04_sight: u16,
    /// Q05: tell at this pivot distance or less, and the standoff he walks to.
    pub engq05_trigger_px: u16,
    pub engq05_standoff_px: u16,
    /// Q06: a hero is eligible while its feet are this close to his floor line.
    pub engq06_floor_tolerance_px: u8,
    /// Q07: the lane check switch.
    pub engq07_lane_check: bool,
    /// Q08: the tell is `rear + point` ticks.
    pub engq08_rear_ticks: u16,
    pub engq08_point_ticks: u16,
    /// Q09: committed launch ticks.
    pub engq09_launch_ticks: u16,
    /// Q10: ram active ticks at most, and its speed (a multiple of 60 px/s).
    pub engq10_active_ticks: u16,
    pub engq10_speed_px_s: u16,
    /// Q11: the ram's damage and knock on a hero.
    pub engq11_damage: u8,
    pub engq11_knock_x: u16,
    pub engq11_knock_y: u16,
    /// Q12: recover ticks.
    pub engq12_recover_ticks: u16,
    /// Q13: the bonk is `hurt + daze` ticks; self-damage 0 turns the damage off.
    pub engq13_hurt_ticks: u16,
    pub engq13_daze_ticks: u16,
    pub engq13_self_damage: u8,
    /// Q14: the crew bowl switch.
    pub engq14_bowl: bool,
    /// Q15: cooldown after a recover or a bonk.
    pub engq15_cooldown_ticks: u8,
    /// Q18: nerve and reactions (always off in v1).
    pub engq18_morale: bool,
    /// Q19: his defeat is a spectacular witness event.
    pub engq19_defeat_spectacular: bool,
    /// Q25: the data kill switch.
    pub engq25_enabled: bool,
    /// Q27: the snap distance (0 = off).
    pub engq27_snap_px: u16,
}

impl EngineerTuning {
    /// Parse and validate.
    pub fn from_json(text: &str) -> Result<Self, EngineerTuningError> {
        let tuning: Self = serde_json::from_str(text)
            .map_err(|err| EngineerTuningError::Parse(err.to_string()))?;
        tuning.validate()?;
        Ok(tuning)
    }

    /// Parse the embedded file and require the pinned content hash.
    pub fn pinned() -> Result<Self, EngineerTuningError> {
        Self::from_json_pinned(TUNING_JSON, TUNING_HASH)
    }

    /// Parse `text` and require its content hash to equal `expected`.
    pub fn from_json_pinned(text: &str, expected: u64) -> Result<Self, EngineerTuningError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(EngineerTuningError::HashMismatch { expected, actual });
        }
        Self::from_json(text)
    }

    /// The one way to read a switch of the file: effective only while the data
    /// kill switch `engq25_enabled` is on.
    pub fn on(&self, switch: bool) -> bool {
        self.engq25_enabled && switch
    }

    /// Whether the Engineer exists at all (the kill switch).
    pub fn enabled(&self) -> bool {
        self.engq25_enabled
    }

    /// Ram speed in whole px per tick.
    pub fn ram_px_per_tick(&self) -> f32 {
        f32::from(self.engq10_speed_px_s / 60)
    }

    /// The tell length without a snap: the attack tick on which the launch begins.
    pub fn tell_len(&self) -> u16 {
        self.engq08_rear_ticks + self.engq08_point_ticks
    }

    /// Bonk ticks in all.
    pub fn bonk_ticks(&self) -> u16 {
        self.engq13_hurt_ticks + self.engq13_daze_ticks
    }

    pub fn validate(&self) -> Result<(), EngineerTuningError> {
        fn bad(field: &'static str, reason: &str) -> Result<(), EngineerTuningError> {
            Err(EngineerTuningError::Field {
                field,
                reason: reason.to_string(),
            })
        }
        fn positive(field: &'static str, value: u32) -> Result<(), EngineerTuningError> {
            if value == 0 {
                return bad(field, "must be positive");
            }
            Ok(())
        }
        positive("engq02_health", u32::from(self.engq02_health))?;
        let b = &self.engq03_ram_box;
        if b.x0 >= b.x1 || b.y0 >= b.y1 {
            return bad("engq03_ram_box", "needs x0 < x1 and y0 < y1");
        }
        positive("engq04_walk_speed", u32::from(self.engq04_walk_speed))?;
        positive("engq04_sight", u32::from(self.engq04_sight))?;
        positive("engq05_trigger_px", u32::from(self.engq05_trigger_px))?;
        if self.engq05_standoff_px > self.engq05_trigger_px {
            return bad("engq05_standoff_px", "must not exceed engq05_trigger_px");
        }
        positive("engq08_rear_ticks", u32::from(self.engq08_rear_ticks))?;
        positive("engq08_point_ticks", u32::from(self.engq08_point_ticks))?;
        positive("engq09_launch_ticks", u32::from(self.engq09_launch_ticks))?;
        positive("engq10_active_ticks", u32::from(self.engq10_active_ticks))?;
        if self.engq10_speed_px_s == 0 || !self.engq10_speed_px_s.is_multiple_of(60) {
            return bad(
                "engq10_speed_px_s",
                "must be a positive multiple of 60 (whole px per tick)",
            );
        }
        positive("engq12_recover_ticks", u32::from(self.engq12_recover_ticks))?;
        positive("engq13_hurt_ticks", u32::from(self.engq13_hurt_ticks))?;
        positive("engq13_daze_ticks", u32::from(self.engq13_daze_ticks))?;
        if self.engq27_snap_px >= self.engq05_trigger_px {
            return bad(
                "engq27_snap_px",
                "must be below engq05_trigger_px (0 = off)",
            );
        }
        Ok(())
    }
}

/// The pinned tuning: what a simulation with an Engineer reads. One shared static
/// so goblins can hold a `&'static` reference.
pub fn active() -> &'static EngineerTuning {
    static ACTIVE: std::sync::OnceLock<EngineerTuning> = std::sync::OnceLock::new();
    ACTIVE.get_or_init(|| {
        EngineerTuning::pinned().expect("the embedded engineer data is pinned and valid")
    })
}

/// The phase of the Engineer's attack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineerPhase {
    /// Frame 9: the rear (tell).
    Rear,
    /// Frame 10: the arm points at the target (tell).
    Point,
    /// Frame 11: committed, no motion.
    Launch,
    /// Frame 12: the ram runs.
    Ram,
    /// Frame 13: after a whiff, a hero hit or a bowl.
    Recover,
    /// The pot met a solid: frame 14 for the hurt ticks, then frame 13.
    Bonk,
}

impl EngineerPhase {
    /// The one kind-aware mapping every consumer reads (design section 14): rear
    /// and point are the tell, the launch the smear, the ram the impact, recover
    /// and bonk the recovery.
    pub fn attack_phase(self) -> AttackPhase {
        match self {
            Self::Rear | Self::Point => AttackPhase::Tell,
            Self::Launch => AttackPhase::Smear,
            Self::Ram => AttackPhase::Impact,
            Self::Recover | Self::Bonk => AttackPhase::Recover,
        }
    }

    /// Mixed into the state hash.
    pub fn code(self) -> u64 {
        match self {
            Self::Rear => 0,
            Self::Point => 1,
            Self::Launch => 2,
            Self::Ram => 3,
            Self::Recover => 4,
            Self::Bonk => 5,
        }
    }
}

/// What the Engineer's step reads of the scene (computed by `combat_scene` before
/// his step, so the Bevy-free goblin never reads the scene).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineerSense {
    /// The hero he points at (locked at the tell start by the scene).
    pub target: HeroId,
    pub target_x: f32,
    /// The target's feet are within the floor tolerance of his floor line.
    pub eligible: bool,
    /// Nothing solid and no contactable goblin between him and the target.
    pub lane_clear: bool,
    /// Eligible and within the trigger distance.
    pub in_trigger: bool,
}

/// The Engineer's attack state, meaningful while his goblin is in `Attack`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineerState {
    pub phase: EngineerPhase,
    /// Ticks the timed phase (launch, recover, bonk) still has.
    pub left: u16,
    /// Ram active ticks so far.
    pub ram_ticks: u16,
    /// The attack tick on which the launch begins: the tell length, or the snap
    /// tick plus one once the snap fired.
    pub tell_len: u16,
    /// Armed at the tell start when the target was farther than the snap
    /// distance.
    pub snap_armed: bool,
    pub snapped: bool,
    /// The ram stamp of this attack (`0x8000_0000 | ram_seq`, set by the scene on
    /// the launch's first tick; 0 until then).
    pub ram_id: u32,
    // Transient, set by the step and read by the scene on the same tick; never
    // hashed and cleared at the start of every step.
    /// The ram moved this tick (an active tick).
    pub ram_stepped: bool,
    /// The pot front met a solid this tick.
    pub met_solid: bool,
    /// The launch began this tick (the scene stamps `ram_id`).
    pub launch_began: bool,
}

impl EngineerState {
    pub const IDLE: Self = Self {
        phase: EngineerPhase::Rear,
        left: 0,
        ram_ticks: 0,
        tell_len: 0,
        snap_armed: false,
        snapped: false,
        ram_id: 0,
        ram_stepped: false,
        met_solid: false,
        launch_began: false,
    };

    /// A fresh attack: the tell starts.
    pub fn start(tuning: &EngineerTuning, snap_armed: bool) -> Self {
        Self {
            phase: EngineerPhase::Rear,
            tell_len: tuning.tell_len(),
            snap_armed,
            ..Self::IDLE
        }
    }

    /// Mix the persistent part into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.phase.code());
        mix(u64::from(self.left));
        mix(u64::from(self.ram_ticks));
        mix(u64::from(self.tell_len));
        mix(u64::from(self.snap_armed));
        mix(u64::from(self.snapped));
        if self.ram_id != 0 {
            mix(0x5241_4d49); // "RAMI"
            mix(u64::from(self.ram_id));
        }
    }
}

/// The ram box in the world: `dir` is +1 or -1, `feet` the height of his feet.
pub fn ram_box(t: &EngineerTuning, x: f32, dir: i8, feet: f32) -> Rect {
    let b = &t.engq03_ram_box;
    let (x0, x1) = (f32::from(b.x0), f32::from(b.x1));
    let (left, right) = if dir >= 0 {
        (x + x0, x + x1)
    } else {
        (x - x1, x - x0)
    };
    Rect {
        left,
        bottom: feet + f32::from(b.y0),
        width: right - left,
        height: f32::from(b.y1 - b.y0),
    }
}

/// One active ram tick of motion: he moves a whole step forward. When the pot's
/// front would pass the face of an intact solid he is clamped so the front rests
/// on the face (never backward) and the tick is marked `met_solid`; the body
/// meeting a low solid also stops him. Returns `(new x, met_solid)`.
pub fn ram_motion(
    t: &EngineerTuning,
    x: f32,
    dir: i8,
    feet: f32,
    solids: &[Block],
    max_x: f32,
) -> (f32, bool) {
    let step = f32::from(dir) * t.ram_px_per_tick();
    let candidate = x + step;
    let boxed = ram_box(t, candidate, dir, feet);
    let mut face: Option<f32> = None;
    for block in solids.iter().filter(|b| b.intact && boxed.overlaps(b.rect)) {
        let f = if dir >= 0 {
            block.rect.left
        } else {
            block.rect.right()
        };
        face = Some(match face {
            None => f,
            Some(g) if dir >= 0 => g.min(f),
            Some(g) => g.max(f),
        });
    }
    if let Some(face) = face {
        let front = f32::from(t.engq03_ram_box.x1);
        let stop = if dir >= 0 { face - front } else { face + front };
        let (lo, hi) = if dir >= 0 {
            (x, candidate)
        } else {
            (candidate, x)
        };
        return (stop.clamp(lo, hi), true);
    }
    let body = crate::combat::body_rect(candidate, feet + crate::combat::ENEMY_HALF_HEIGHT);
    if solids.iter().any(|b| b.intact && body.overlaps(b.rect)) {
        return (x, true);
    }
    let clamped = candidate.clamp(ENEMY_HALF_WIDTH, max_x);
    (clamped, false)
}

/// Q07: the lane is clear when no intact solid and no goblin body (the caller
/// passes only the contactable common goblins) lies inside the open interval
/// between his front edge and the target's column, at heights 0..38.
pub fn lane_clear(
    x: f32,
    feet: f32,
    target_x: f32,
    solids: &[Block],
    goblin_bodies: impl IntoIterator<Item = Rect>,
) -> bool {
    let toward = if target_x >= x { 1.0 } else { -1.0 };
    let front = x + toward * ENEMY_HALF_WIDTH;
    let (lo, hi) = if toward > 0.0 {
        (front, target_x)
    } else {
        (target_x, front)
    };
    if hi <= lo {
        return true;
    }
    let lane = Rect {
        left: lo,
        bottom: feet,
        width: hi - lo,
        height: crate::combat::ENEMY_HALF_HEIGHT * 2.0,
    };
    !solids.iter().any(|b| b.intact && lane.overlaps(b.rect))
        && !goblin_bodies.into_iter().any(|r| lane.overlaps(r))
}

/// Reach maths of the design (section 4), for tests and tools: the first ram
/// active tick on which the box overlaps a standing hero of half-width `h`
/// at pivot distance `d`, or `None` when the ram never reaches it.
pub fn contact_tick(t: &EngineerTuning, d: f32, h: f32) -> Option<u16> {
    let step = t.ram_px_per_tick();
    let front = f32::from(t.engq03_ram_box.x1);
    (1..=t.engq10_active_ticks).find(|&k| step * f32::from(k) + front > d - h)
}

/// The bonk tick for a solid face `face` px ahead of his start pivot, or `None`
/// when the ram stops first.
pub fn bonk_tick(t: &EngineerTuning, face: f32) -> Option<u16> {
    let step = t.ram_px_per_tick();
    let front = f32::from(t.engq03_ram_box.x1);
    (1..=t.engq10_active_ticks).find(|&k| step * f32::from(k) + front > face)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prototype::Material;
    use serde_json::Value;

    fn tuning() -> EngineerTuning {
        EngineerTuning::pinned().unwrap()
    }

    #[test]
    fn the_file_is_pinned_and_round_trips() {
        assert_eq!(content_hash(TUNING_JSON), TUNING_HASH);
        let t = tuning();
        let original: Value = serde_json::from_str(TUNING_JSON).unwrap();
        assert_eq!(serde_json::to_value(&t).unwrap(), original);
        let drifted = TUNING_JSON.replace("\"engq02_health\": 3", "\"engq02_health\": 4");
        match EngineerTuning::from_json_pinned(&drifted, TUNING_HASH) {
            Err(EngineerTuningError::HashMismatch { expected, .. }) => {
                assert_eq!(expected, TUNING_HASH)
            }
            other => panic!("a drifted file must fail the pin: {other:?}"),
        }
    }

    #[test]
    fn every_question_has_a_knob_or_a_recorded_reason() {
        let value: Value = serde_json::from_str(TUNING_JSON).unwrap();
        let keys: Vec<&String> = value.as_object().unwrap().keys().collect();
        let exempt: Vec<u8> = NO_KNOB.iter().map(|(q, _)| *q).collect();
        for q in 1..=27u8 {
            let prefix = format!("engq{q:02}_");
            let has_key = keys.iter().any(|k| k.starts_with(&prefix));
            if exempt.contains(&q) {
                assert!(
                    !has_key,
                    "ENG-G-Q{q:02} is listed as knob-less but has a key"
                );
            } else {
                assert!(has_key, "ENG-G-Q{q:02} has no knob and is not in NO_KNOB");
            }
        }
        for key in keys {
            let named = key.starts_with("engq")
                && key[4..6].parse::<u8>().is_ok_and(|q| (1..=27).contains(&q))
                && key.as_bytes()[6] == b'_';
            assert!(
                named || key == "schema_version",
                "{key} is not named after an ENG-G question id"
            );
        }
    }

    #[test]
    fn a_bad_value_names_its_field() {
        let cases = [
            (
                "\"engq10_speed_px_s\": 240",
                "\"engq10_speed_px_s\": 250",
                "engq10_speed_px_s",
            ),
            (
                "\"engq10_speed_px_s\": 240",
                "\"engq10_speed_px_s\": 0",
                "engq10_speed_px_s",
            ),
            (
                "\"engq05_standoff_px\": 72",
                "\"engq05_standoff_px\": 80",
                "engq05_standoff_px",
            ),
            (
                "\"engq27_snap_px\": 48",
                "\"engq27_snap_px\": 72",
                "engq27_snap_px",
            ),
            (
                "\"engq08_rear_ticks\": 8",
                "\"engq08_rear_ticks\": 0",
                "engq08_rear_ticks",
            ),
            (
                "\"engq12_recover_ticks\": 40",
                "\"engq12_recover_ticks\": 0",
                "engq12_recover_ticks",
            ),
            ("\"x1\": 24", "\"x1\": 14", "engq03_ram_box"),
        ];
        for (from, to, field) in cases {
            let text = TUNING_JSON.replace(from, to);
            match EngineerTuning::from_json(&text) {
                Err(EngineerTuningError::Field { field: f, .. }) => assert_eq!(f, field),
                other => panic!("{field}: {other:?}"),
            }
        }
        // The snap may be off.
        let off = TUNING_JSON.replace("\"engq27_snap_px\": 48", "\"engq27_snap_px\": 0");
        assert!(EngineerTuning::from_json(&off).is_ok());
    }

    #[test]
    fn the_kill_switch_turns_every_switch_off() {
        let mut t = tuning();
        assert!(t.on(t.engq14_bowl));
        t.engq25_enabled = false;
        assert!(!t.on(true));
    }

    /// Hero hurtbox half-widths of the design: Carl 12, Donut 10.
    const CARL_H: f32 = 12.0;
    const DONUT_H: f32 = 10.0;

    #[test]
    fn the_reach_table_contact_ticks() {
        let t = tuning();
        // (start d, active tick of a hit on a standing Carl)
        for (d, tick) in [
            (12.0, Some(1)),
            (36.0, Some(1)),
            (40.0, Some(2)),
            (48.0, Some(4)),
            (60.0, Some(7)),
            (72.0, Some(10)),
            (88.0, Some(14)),
            (100.0, Some(17)),
            (112.0, Some(20)),
            (116.0, None),
        ] {
            assert_eq!(contact_tick(&t, d, CARL_H), tick, "Carl at {d}");
            // Donut has the same contact ticks at every listed distance.
            assert_eq!(contact_tick(&t, d, DONUT_H), tick, "Donut at {d}");
        }
        // Carl is hit when d < 116, Donut when d < 114.
        assert_eq!(contact_tick(&t, 115.0, CARL_H), Some(20));
        assert_eq!(contact_tick(&t, 114.0, CARL_H), Some(20));
        assert_eq!(contact_tick(&t, 114.0, DONUT_H), None);
        assert_eq!(contact_tick(&t, 113.0, DONUT_H), Some(20));
    }

    /// Feet height after jump tick `n` (tick 1 is the press tick): the scene's
    /// jump (440 px/s, gravity 1050, velocity set, then gravity, then move).
    fn jump_feet(n: u32) -> f32 {
        if n == 0 {
            return 0.0;
        }
        let n = n as f32;
        ((440.0 * n - 8.75 * n * (n + 1.0)) / 60.0).max(0.0)
    }

    /// Simulate one ram at pivot distance `d` against a hero at the pivot column
    /// `d`, pressing the jump on attack tick `press` (the ram's active tick 1 is
    /// attack tick 35). `Some(active tick)` of the hit.
    fn ram_hit_tick(t: &EngineerTuning, d: f32, h: f32, press: Option<u32>) -> Option<u16> {
        let launch_end = t.tell_len() + t.engq09_launch_ticks; // 35
        for k in 1..=t.engq10_active_ticks {
            let attack_tick = u32::from(launch_end + k - 1);
            let feet = press.map_or(0.0, |p| {
                if attack_tick >= p {
                    jump_feet(attack_tick - p + 1)
                } else {
                    0.0
                }
            });
            // The box after active tick k: the pivot has moved k steps.
            let pot = ram_box(t, f32::from(k) * t.ram_px_per_tick(), 1, 0.0);
            let hero = Rect {
                left: d - h,
                bottom: feet,
                width: h * 2.0,
                height: 52.0,
            };
            if pot.overlaps(hero) {
                return Some(k);
            }
        }
        None
    }

    #[test]
    fn the_jump_window_edges_at_the_standoff() {
        let t = tuning();
        // `eng_jump_window_edges`: a jump pressed on attack tick 7 / 8 / 39 / 40.
        assert_eq!(ram_hit_tick(&t, 72.0, CARL_H, Some(7)), Some(17));
        assert_eq!(ram_hit_tick(&t, 72.0, CARL_H, Some(8)), None);
        assert_eq!(ram_hit_tick(&t, 72.0, CARL_H, Some(39)), None);
        assert_eq!(ram_hit_tick(&t, 72.0, CARL_H, Some(40)), Some(10));
        // The table's window at the standoff is 8..39, a standing hero is hit on 10.
        assert_eq!(ram_hit_tick(&t, 72.0, CARL_H, None), Some(10));
        // Point-blank: a jump by attack tick 30 clears, 31 does not.
        assert_eq!(ram_hit_tick(&t, 12.0, CARL_H, Some(30)), None);
        assert_eq!(ram_hit_tick(&t, 12.0, CARL_H, Some(31)), Some(1));
        // d 40: 0..31.
        assert_eq!(ram_hit_tick(&t, 40.0, CARL_H, Some(0)), None);
        assert_eq!(ram_hit_tick(&t, 40.0, CARL_H, Some(31)), None);
        assert_eq!(ram_hit_tick(&t, 40.0, CARL_H, Some(32)), Some(2));
    }

    #[test]
    fn the_matador_table_bonk_ticks_and_stop_columns() {
        let t = tuning();
        for (face, tick) in [
            (24.0, Some(1)),
            (56.0, Some(9)),
            (84.0, Some(16)),
            (92.0, Some(18)),
            (103.0, Some(20)),
            (104.0, None),
        ] {
            assert_eq!(bonk_tick(&t, face), tick, "face {face}");
        }
        // The real motion agrees: a solid face `face` ahead of pivot 200.
        const ORIGIN: f32 = 200.0;
        for (face, tick, stops) in [
            (24.0_f32, 1u16, 0.0_f32),
            (56.0, 9, 32.0),
            (84.0, 16, 60.0),
            (92.0, 18, 68.0),
            (103.0, 20, 79.0),
        ] {
            let wall = [Block {
                rect: Rect {
                    left: ORIGIN + face,
                    bottom: 0.0,
                    width: 16.0,
                    height: 200.0,
                },
                material: Material::Stone,
                intact: true,
            }];
            let mut x = ORIGIN;
            let mut met_on = None;
            for k in 1..=t.engq10_active_ticks {
                let (nx, met) = ram_motion(&t, x, 1, 0.0, &wall, 10_000.0);
                x = nx;
                if met {
                    met_on = Some(k);
                    break;
                }
            }
            assert_eq!(met_on, Some(tick), "face {face}");
            assert_eq!(x - ORIGIN, stops, "stop column for face {face}");
        }
        // No bonk from 104 on: the ram runs its 80 px.
        let wall = [Block {
            rect: Rect {
                left: ORIGIN + 104.0,
                bottom: 0.0,
                width: 16.0,
                height: 200.0,
            },
            material: Material::Stone,
            intact: true,
        }];
        let mut x = ORIGIN;
        for _ in 1..=t.engq10_active_ticks {
            let (nx, met) = ram_motion(&t, x, 1, 0.0, &wall, 10_000.0);
            assert!(!met);
            x = nx;
        }
        assert_eq!(x - ORIGIN, 80.0);
    }

    #[test]
    fn the_clamp_never_moves_him_backward() {
        let t = tuning();
        // The pot front is already beyond the face (a face closer than 24): no motion.
        let wall = [Block {
            rect: Rect {
                left: 210.0,
                bottom: 0.0,
                width: 16.0,
                height: 200.0,
            },
            material: Material::Stone,
            intact: true,
        }];
        assert_eq!(ram_motion(&t, 200.0, 1, 0.0, &wall, 1000.0), (200.0, true));
        // Facing left, mirrored.
        let wall = [Block {
            rect: Rect {
                left: 60.0,
                bottom: 0.0,
                width: 16.0,
                height: 200.0,
            },
            material: Material::Stone,
            intact: true,
        }];
        // Facing left with the pot front exactly on the face (76): one more step
        // would pass it, so he stays (the box meets the face on this very tick).
        assert_eq!(ram_motion(&t, 100.0, -1, 0.0, &wall, 1000.0), (100.0, true));
    }

    #[test]
    fn the_lane_check_sees_solids_and_goblins_between_him_and_the_target() {
        let wall = [Block {
            rect: Rect {
                left: 230.0,
                bottom: 0.0,
                width: 16.0,
                height: 48.0,
            },
            material: Material::Stone,
            intact: true,
        }];
        let body = |x: f32| crate::combat::body_rect(x, 19.0);
        // Engineer 200, target 272: front 212, lane (212, 272).
        assert!(lane_clear(200.0, 0.0, 272.0, &[], []));
        assert!(!lane_clear(200.0, 0.0, 272.0, &wall, []));
        assert!(!lane_clear(200.0, 0.0, 272.0, &[], [body(236.0)]));
        // A goblin beyond the target is outside the lane.
        assert!(lane_clear(200.0, 0.0, 272.0, &[], [body(290.0)]));
        // Facing left works the same.
        assert!(!lane_clear(272.0, 0.0, 200.0, &wall, []));
        // A target inside his front edge: empty lane.
        assert!(lane_clear(200.0, 0.0, 205.0, &wall, [body(204.0)]));
    }

    #[test]
    fn the_phase_mapping_is_the_one_kind_aware_function() {
        use AttackPhase::{Impact, Smear, Tell};
        use EngineerPhase::*;
        assert_eq!(Rear.attack_phase(), Tell);
        assert_eq!(Point.attack_phase(), Tell);
        assert_eq!(Launch.attack_phase(), Smear);
        assert_eq!(Ram.attack_phase(), Impact);
        assert_eq!(EngineerPhase::Recover.attack_phase(), AttackPhase::Recover);
        assert_eq!(Bonk.attack_phase(), AttackPhase::Recover);
    }
}
