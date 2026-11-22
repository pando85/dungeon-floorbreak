//! CORRAL-B core: the llama rush, a Bevy-free pure machine
//! (`docs/01-design/enemy-gameplay-llama-rush.md`, CRL-Q01..Q43).
//!
//! The Infernal Corral's llamas gain one committed floor charge next to the
//! legacy spit. This module owns everything that does not need the scene and
//! nothing that touches [`crate::llama::Llama`] or the combat scene (those are
//! the CORRAL-B wire step):
//!
//! * [`RushProfile`]: every `crlq##` knob of
//!   `assets/data/gameplay/enemy_llama_rush_v0.json`, loaded, validated and
//!   FNV-1a pinned like `enemy_engineer_v0.json`;
//! * the attack machine ([`RushPhase`], [`RushState`]): tell, committed rush,
//!   recover, bonk, with no tracking after the tell starts and one blow per
//!   instance ([`RushState::step`]);
//! * the geometry of the box, the motion and the lane ([`rush_box`],
//!   [`rush_motion`], [`rush_lane_clear`], [`glob_lane_clear`]);
//! * the corral brain as pure functions ([`decide`]: wake, classify, target,
//!   rush, spit, walk), the F10 hit table ([`hit_reaction`]) and the CRL-Q43
//!   leash clamp ([`corral_slide`], [`clamp_step`]);
//! * the read-only [`RushLane`] accessor the AI partner reads later (KC7).
//!
//! Attack tick 0 is the tick the tell starts: [`RushState::start`] builds the
//! state on that tick and every later [`RushState::step`] raises the attack
//! tick first (the existing `Llama::attack_tick` convention: the spit releases
//! on the step that makes it 36). Heroes step before llamas in a tick, so the
//! hero boxes the caller passes are already this tick's.
//!
//! The machine is inert until CORRAL-B wire hooks it into `llama.rs`: nothing
//! in the simulation constructs a [`RushState`], so every existing replay and
//! golden is unchanged.

use crate::cf02::BoxPx;
use crate::donut_mana::content_hash;
use crate::donut_moves::TargetState;
use crate::llama::{FireView, LLAMA_HALF_HEIGHT};
use crate::party::HeroId;
use crate::prototype::{Block, Rect, TICK_SECONDS};
use serde::{Deserialize, Serialize};

pub mod geometry;

#[cfg(test)]
mod geometry_tests;
#[cfg(test)]
mod tests;

pub use geometry::{
    Leash, RushMotion, bonk_tick, clamp_step, contact_tick, corral_slide, glob_band_overlaps,
    glob_lane_clear, llama_body, llama_solid, rush_box, rush_lane_clear, rush_motion, walk_step,
};

/// The tuning file, embedded so the binary and the tests read the same bytes.
pub const TUNING_JSON: &str = include_str!("../assets/data/gameplay/enemy_llama_rush_v0.json");

/// The committed FNV-1a 64 content hash of [`TUNING_JSON`]. Editing the file
/// without updating this value fails [`RushProfile::pinned`] and the pin test.
pub const TUNING_HASH: u64 = 0x74b6_95e4_796b_6fc1;

/// The manifest key a recording stamps the hash under (`playtrace::live`).
pub const PIN_KEY: &str = "llama_rush_tuning";

/// CRL-Q21: the `HeroBlow` source of a rush contact ("LR"). It collides with
/// no other hero-blow source (`0x4255`, `0x5241`, `0x5348`, `0x504C` and the
/// index-based goblin, Engineer and glob sources); the wire step creates the
/// distinct-source-id test that covers them all. The constant does not say
/// which llama hit: the diagnostic takes the llama from `LlamaRushEnd`.
pub const LLAMA_RUSH_SOURCE_ID: u32 = 0x4C52;

/// CRL-Q14: the llama's own hit-stop on a rush contact (the `hurt_hero` path
/// gives the hero 4; the wire step uses `HERO_DEFEAT_HIT_STOP` when the blow
/// defeats the hero).
pub const RUSH_HIT_STOP: u8 = 4;

/// CRL-Q32: a solid this tall or lower is a prop remnant, not a solid for a
/// llama (the slot-enemy rule). Heroes keep the remnant-lift rule.
pub const REMNANT_MAX_PX: f32 = 16.0;

/// The hero hurtbox half-widths of the fairness proof (section 6), read from
/// their sources: Carl's hurtbox is his 24 x 52 collider (12), Donut's is the
/// shipped `PROVISIONAL_TUNING` hurtbox (20 x 24, so 10). The narrowest one
/// bounds the standing reach (CRL-Q05).
pub const CARL_HALF_WIDTH: f32 = crate::prototype::HERO_HALF_WIDTH;
pub const DONUT_HALF_WIDTH: f32 = crate::party::PROVISIONAL_TUNING.donut_hurtbox.size().0 / 2.0;

/// The llama's body height, the ceiling of the rush box (`y1 <= 39`).
pub const LLAMA_BODY_HEIGHT: f32 = LLAMA_HALF_HEIGHT * 2.0;

/// CRL questions (1..=43) that have no key in the file, and why. The
/// completeness test requires every other ID to own a `crlqNN_` key.
pub const NO_KNOB: [(u8, &str); 26] = [
    (1, "design: the role of the sheet"),
    (2, "instance data: Llama::rush is set per llama"),
    (3, "builder: CombatState::with_llama_party_target"),
    (17, "rule: the fair-rush view rule"),
    (19, "rule: the rush lane, the partition is passable"),
    (20, "rule: the committed rule, damage and hit-stop only"),
    (21, "code: LLAMA_RUSH_SOURCE_ID"),
    (22, "code: LlamaRushTell, LlamaRushCommit, LlamaRushEnd"),
    (23, "code: the per-CombatState rush_seq token"),
    (25, "room data: the corral layout"),
    (27, "instance data: the M1 llama has no profile"),
    (28, "instance data: homes and leashes"),
    (29, "room data: the shelves"),
    (30, "sidecar prop data: the partition"),
    (31, "room data: ceiling_px"),
    (
        32,
        "rule: remnants of 16 px or less are not solids (REMNANT_MAX_PX)",
    ),
    (33, "story pin: room 0 grants nothing"),
    (34, "evaluator constant: the gated policies"),
    (35, "evaluator assertion: the gate invariant"),
    (36, "evaluator data: the phase variants"),
    (37, "rule: the rush does not bowl in v1"),
    (38, "presentation: the code-drawn stand-in"),
    (39, "process: LLAMA-03 is the rush art"),
    (41, "room data: the dock bumper"),
    (42, "evaluator rule: jumping in place counts as stationary"),
    (43, "rule: the leash clamp of every slide and rush step"),
];

/// A knob file problem. A load error names the field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RushProfileError {
    Parse(String),
    Field { field: &'static str, reason: String },
    HashMismatch { expected: u64, actual: u64 },
}

impl core::fmt::Display for RushProfileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "llama rush tuning: {msg}"),
            Self::Field { field, reason } => {
                write!(f, "llama rush tuning: field `{field}`: {reason}")
            }
            Self::HashMismatch { expected, actual } => write!(
                f,
                "llama rush tuning hash {actual:#018x} is not the pinned {expected:#018x}"
            ),
        }
    }
}

impl std::error::Error for RushProfileError {}

/// Every CRL knob (`docs/01-design/enemy-gameplay-llama-rush.md`, section 13).
/// A llama with a profile is a corral llama; one without keeps every legacy
/// rule (CRL-Q02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RushProfile {
    pub schema_version: u32,
    /// Q04: walk speed in px/s, the distance it walks to toward a rushable
    /// target and the legacy stand-off it keeps from a spit-only target.
    pub crlq04_walk_px_s: u16,
    pub crlq04_rush_approach_px: u16,
    pub crlq04_spit_standoff_px: u16,
    /// Q05: a rushable target within this pivot distance, no minimum.
    pub crlq05_trigger_px: u16,
    /// Q06: the tell, interruptible.
    pub crlq06_tell_ticks: u16,
    /// Q07: the committed rush's speed (a multiple of 60 px/s, at most 240) and
    /// its active ticks.
    pub crlq07_speed_px_s: u16,
    pub crlq07_active_ticks: u16,
    /// Q08: the rush box, pivot-relative, x forward, y up from the feet. Its
    /// front (`x1`) is what meets a solid.
    pub crlq08_box: BoxPx,
    /// Q09: recover ticks after a whiff, a hero hit or a leash stop.
    pub crlq09_recover_ticks: u16,
    /// Q10: bonk ticks and self-damage (0 = none).
    pub crlq10_bonk_ticks: u16,
    pub crlq10_self_damage: u8,
    /// Q11: the corral spit rule (spit only when the glob can hit at the
    /// target's support height and the lane is clear).
    pub crlq11_spit_height_check: bool,
    /// Q12: rush cooldown after Recover or Bonk ends, and the spit cooldown
    /// floor set at the same time.
    pub crlq12_rush_cooldown: u16,
    pub crlq12_spit_gap: u16,
    /// Q13: `rush_cooldown` floor after an interrupted tell.
    pub crlq13_interrupt_cooldown: u16,
    /// Q14: the contact's damage and knock.
    pub crlq14_damage: u8,
    pub crlq14_knock_x: u16,
    pub crlq14_knock_y: u16,
    /// Q15: only heroes within this pivot distance are candidates.
    pub crlq15_target_range_px: u16,
    /// Q16: a hero is rushable while her support height is this close to the
    /// llama's floor line.
    pub crlq16_floor_tolerance_px: u8,
    /// Q18: the rush is not reflex-evadable (default `false`).
    pub crlq18_rush_evadable: bool,
    /// Q24: a dormant llama wakes when a living hero is this close.
    pub crlq24_wake_px: u16,
    /// Q26: the M3 gate stays closed until both M2 llamas are defeated.
    pub crlq26_exit_gate: bool,
    /// Q40: the data kill switch: `false` makes every corral llama run without
    /// its profile.
    pub crlq40_enabled: bool,
}

impl RushProfile {
    /// Parse and validate.
    pub fn from_json(text: &str) -> Result<Self, RushProfileError> {
        let profile: Self =
            serde_json::from_str(text).map_err(|err| RushProfileError::Parse(err.to_string()))?;
        profile.validate()?;
        Ok(profile)
    }

    /// Parse the embedded file and require the pinned content hash.
    pub fn pinned() -> Result<Self, RushProfileError> {
        Self::from_json_pinned(TUNING_JSON, TUNING_HASH)
    }

    /// Parse `text` and require its content hash to equal `expected`.
    pub fn from_json_pinned(text: &str, expected: u64) -> Result<Self, RushProfileError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(RushProfileError::HashMismatch { expected, actual });
        }
        Self::from_json(text)
    }

    /// The one way to read a switch of the file: effective only while the data
    /// kill switch `crlq40_enabled` is on.
    pub fn on(&self, switch: bool) -> bool {
        self.crlq40_enabled && switch
    }

    /// Whether corral llamas run their profile at all (the kill switch).
    pub fn enabled(&self) -> bool {
        self.crlq40_enabled
    }

    /// Rush speed in whole px per tick.
    pub fn px_per_tick(&self) -> f32 {
        f32::from(self.crlq07_speed_px_s / 60)
    }

    /// The distance of a whole rush: active ticks times the step.
    pub fn rush_distance(&self) -> f32 {
        f32::from(self.crlq07_active_ticks) * self.px_per_tick()
    }

    /// The pivot distance below which a standing hero of half-width `half_w`
    /// is hit by a full rush (134 for Donut, 136 for Carl).
    pub fn standing_reach(&self, half_w: f32) -> f32 {
        self.rush_distance() + f32::from(self.crlq08_box.x1) + half_w
    }

    /// The legacy-or-profile walk speed in px per tick.
    pub fn walk_px_per_tick(&self) -> f32 {
        f32::from(self.crlq04_walk_px_s) * TICK_SECONDS
    }

    pub fn validate(&self) -> Result<(), RushProfileError> {
        fn bad(field: &'static str, reason: &str) -> Result<(), RushProfileError> {
            Err(RushProfileError::Field {
                field,
                reason: reason.to_string(),
            })
        }
        fn positive(field: &'static str, value: u32) -> Result<(), RushProfileError> {
            if value == 0 {
                return bad(field, "must be positive");
            }
            Ok(())
        }
        if self.schema_version != 1 {
            return bad("schema_version", "must be 1");
        }
        positive("crlq04_walk_px_s", u32::from(self.crlq04_walk_px_s))?;
        positive(
            "crlq04_rush_approach_px",
            u32::from(self.crlq04_rush_approach_px),
        )?;
        positive(
            "crlq04_spit_standoff_px",
            u32::from(self.crlq04_spit_standoff_px),
        )?;
        positive("crlq05_trigger_px", u32::from(self.crlq05_trigger_px))?;
        if self.crlq04_rush_approach_px > self.crlq05_trigger_px {
            return bad(
                "crlq04_rush_approach_px",
                "must not exceed crlq05_trigger_px",
            );
        }
        positive("crlq06_tell_ticks", u32::from(self.crlq06_tell_ticks))?;
        if self.crlq07_speed_px_s == 0 || !self.crlq07_speed_px_s.is_multiple_of(60) {
            return bad(
                "crlq07_speed_px_s",
                "must be a positive multiple of 60 (whole px per tick)",
            );
        }
        if self.crlq07_speed_px_s > 240 {
            return bad(
                "crlq07_speed_px_s",
                "must be at most 240 px/s (the scroll camera's 4 px/tick cap, KC18)",
            );
        }
        positive("crlq07_active_ticks", u32::from(self.crlq07_active_ticks))?;
        let b = &self.crlq08_box;
        if b.x0 >= b.x1 || b.y0 >= b.y1 {
            return bad("crlq08_box", "needs x0 < x1 and y0 < y1");
        }
        if b.y0 < 0 || f32::from(b.y1) > LLAMA_BODY_HEIGHT {
            return bad(
                "crlq08_box",
                "must lie inside the 39 px body (0 <= y0, y1 <= 39)",
            );
        }
        // The trigger stays inside the narrowest hero's standing reach, so every
        // rush that starts can hit a hero who does nothing (section 6).
        if f32::from(self.crlq05_trigger_px) >= self.standing_reach(DONUT_HALF_WIDTH) {
            return bad(
                "crlq05_trigger_px",
                "must be below the standing reach (active ticks x speed + box front + 10)",
            );
        }
        positive("crlq09_recover_ticks", u32::from(self.crlq09_recover_ticks))?;
        positive("crlq10_bonk_ticks", u32::from(self.crlq10_bonk_ticks))?;
        positive("crlq14_damage", u32::from(self.crlq14_damage))?;
        positive(
            "crlq15_target_range_px",
            u32::from(self.crlq15_target_range_px),
        )?;
        positive("crlq24_wake_px", u32::from(self.crlq24_wake_px))?;
        Ok(())
    }
}

/// The pinned profile: what a simulation with a corral llama reads. One shared
/// static so llamas can hold a `&'static` reference.
pub fn active() -> &'static RushProfile {
    static ACTIVE: std::sync::OnceLock<RushProfile> = std::sync::OnceLock::new();
    ACTIVE.get_or_init(|| {
        RushProfile::pinned().expect("the embedded llama rush data is pinned and valid")
    })
}

// ------------------------------------------------------------------ machine --

/// The phase of the rush (`RushTell`, `CommittedRush`, `Recover`, `Bonk`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RushPhase {
    /// Attack ticks `0..tell`: facing, direction and target locked; the llama
    /// is fully interruptible like the spit tell.
    Tell,
    /// The committed rush: 4 px forward per active tick, no tracking, no turn.
    Committed,
    /// After a whiff, a hero hit or a leash stop.
    Recover,
    /// The box met a solid or the partition.
    Bonk,
}

impl RushPhase {
    /// The one kind-aware mapping every consumer reads (section 5): the tell is
    /// the tell, the committed rush the impact (so a swat guard clears on it),
    /// recover and bonk the recovery.
    pub fn target_state(self) -> TargetState {
        match self {
            Self::Tell => TargetState::Tell,
            Self::Committed => TargetState::Impact,
            Self::Recover | Self::Bonk => TargetState::Recovery,
        }
    }

    /// Mixed into the state hash.
    pub fn code(self) -> u64 {
        match self {
            Self::Tell => 0,
            Self::Committed => 1,
            Self::Recover => 2,
            Self::Bonk => 3,
        }
    }

    /// The box exists only while the rush is committed.
    pub fn has_box(self) -> bool {
        self == Self::Committed
    }
}

/// How a rush ended (`LlamaRushEnd { end }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RushEnd {
    /// The box met a hero.
    Contact(HeroId),
    /// A whole rush or a leash stop with nobody hit.
    Miss,
    /// The box met an intact solid.
    Blocked,
    /// The box met the breakable partition (it also gets the stimulus).
    Partition,
}

impl RushEnd {
    /// Mixed into the event hash: the end kind, and the hero for a contact.
    pub fn code(self) -> u64 {
        match self {
            Self::Contact(hero) => 0x10 | hero.index() as u64,
            Self::Miss => 1,
            Self::Blocked => 2,
            Self::Partition => 3,
        }
    }

    /// The log word of the event line.
    pub fn name(self) -> &'static str {
        match self {
            Self::Contact(crate::party::HeroId::Carl) => "contact_carl",
            Self::Contact(crate::party::HeroId::Donut) => "contact_donut",
            Self::Miss => "miss",
            Self::Blocked => "blocked",
            Self::Partition => "partition",
        }
    }
}

/// A hero the rush may hit, as the caller sees her after this tick's hero step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RushHero {
    pub id: HeroId,
    pub hurtbox: Rect,
    /// An invulnerable hero, or a guarded off-screen hero (D1 M4), is not hit
    /// and does not stop the rush.
    pub hittable: bool,
}

/// What a rush step reads of the scene.
#[derive(Debug, Clone, Copy)]
pub struct RushWorld<'a> {
    /// The llama's floor line.
    pub feet: f32,
    pub solids: &'a [Block],
    /// The index in `solids` of the breakable partition, if any.
    pub partition: Option<usize>,
    pub leash: Leash,
    /// The layout edge for the pivot.
    pub max_x: f32,
    pub heroes: &'a [RushHero],
}

/// The blow a contact hands to `hurt_hero` (CRL-Q14, CRL-Q21).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RushBlow {
    pub hero: HeroId,
    pub source: u32,
    pub damage: u8,
    pub from_x: f32,
    pub knock_vx: f32,
    pub knock_vy: f32,
    pub instance: u32,
}

/// What one rush step did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RushOut {
    /// The llama's new pivot x.
    pub x: f32,
    /// The commit began this tick (`LlamaRushCommit`): attack tick `tell`.
    pub commit: bool,
    /// The rush ended this tick (`LlamaRushEnd`).
    pub end: Option<RushEnd>,
    /// A contact: the blow for `hurt_hero`.
    pub blow: Option<RushBlow>,
    /// The box met the partition this tick: send it the rush stimulus.
    pub partition_hit: bool,
    /// Self-damage on a bonk (0 = none; the Q10 knob).
    pub self_damage: u8,
    /// The whole attack is over: the llama goes back to `Neutral` and takes the
    /// [`cooldowns_after_rush`].
    pub finished: bool,
}

/// The rush attack's state, meaningful while the llama is in `Attack` with the
/// rush kind. `None` outside it, so a llama's rush costs nothing while idle. A
/// live `RushState` always means the llama is busy (one `Attack` at a time, so
/// no spit can start); the exclusion itself is wire work.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RushState {
    pub phase: RushPhase,
    /// Attack tick: steps since the tell started (0 on the start tick).
    pub tick: u16,
    /// Recover or bonk ticks still to run.
    pub left: u16,
    /// Active ticks done.
    pub active: u16,
    /// +1 or -1, locked at the tell start.
    pub dir: i8,
    /// The hero the tell points at, locked at the tell start.
    pub target: HeroId,
    /// The pivot at the tell start.
    pub start_x: f32,
    /// The attack-instance token (`rush_seq` of the combat state).
    pub seq: u32,
}

impl RushState {
    /// A fresh attack: attack tick 0, the tell starts. `seq` is the combat
    /// state's `rush_seq` counter after its increment.
    pub fn start(x: f32, dir: i8, target: HeroId, seq: u32) -> Self {
        Self {
            phase: RushPhase::Tell,
            tick: 0,
            left: 0,
            active: 0,
            dir: if dir >= 0 { 1 } else { -1 },
            target,
            start_x: x,
            seq,
        }
    }

    /// In the tell: a hit cancels the rush (the legacy `Hurt` path).
    pub fn telling(&self) -> bool {
        self.phase == RushPhase::Tell
    }

    /// Mix the state into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.phase.code());
        mix(u64::from(self.tick));
        mix(u64::from(self.left));
        mix(u64::from(self.active));
        mix(self.dir as u64);
        mix(self.target.index() as u64);
        mix(u64::from(self.start_x.to_bits()));
        mix(u64::from(self.seq));
    }

    /// One fixed tick (the llama is not in hit-stop, not hurt). `x` is the
    /// llama's pivot; the output carries the new one.
    ///
    /// An active tick runs in this order (step first, then test; the Engineer
    /// order): the step, then the hero, then the solid, then the leash, then
    /// the distance.
    pub fn step(&mut self, p: &RushProfile, x: f32, world: &RushWorld) -> RushOut {
        let mut out = RushOut {
            x,
            commit: false,
            end: None,
            blow: None,
            partition_hit: false,
            self_damage: 0,
            finished: false,
        };
        self.tick = self.tick.saturating_add(1);
        match self.phase {
            RushPhase::Tell => {
                if self.tick < p.crlq06_tell_ticks {
                    return out;
                }
                self.phase = RushPhase::Committed;
                out.commit = true;
                self.active_tick(p, world, &mut out);
            }
            RushPhase::Committed => self.active_tick(p, world, &mut out),
            RushPhase::Recover | RushPhase::Bonk => {
                self.left = self.left.saturating_sub(1);
                if self.left == 0 {
                    out.finished = true;
                }
            }
        }
        out
    }

    fn active_tick(&mut self, p: &RushProfile, world: &RushWorld, out: &mut RushOut) {
        self.active += 1;
        // 1. Step.
        let motion = rush_motion(
            p,
            out.x,
            self.dir,
            world.feet,
            world.solids,
            world.leash,
            world.max_x,
        );
        out.x = motion.x;
        // 2. Hero.
        let boxed = rush_box(p, out.x, self.dir, world.feet);
        if let Some(hero) = first_hero_in_box(boxed, self.dir, world.heroes) {
            out.blow = Some(RushBlow {
                hero,
                source: LLAMA_RUSH_SOURCE_ID,
                damage: p.crlq14_damage,
                from_x: out.x,
                knock_vx: f32::from(self.dir) * f32::from(p.crlq14_knock_x),
                knock_vy: f32::from(p.crlq14_knock_y),
                instance: self.seq,
            });
            out.end = Some(RushEnd::Contact(hero));
            self.phase = RushPhase::Recover;
            self.left = p.crlq09_recover_ticks;
            return;
        }
        // 3. Solid.
        if motion.met {
            let partition = motion.met_block.is_some() && motion.met_block == world.partition;
            out.end = Some(if partition {
                RushEnd::Partition
            } else {
                RushEnd::Blocked
            });
            out.partition_hit = partition;
            out.self_damage = p.crlq10_self_damage;
            self.phase = RushPhase::Bonk;
            self.left = p.crlq10_bonk_ticks;
            return;
        }
        // 4. Leash, 5. Distance.
        if motion.leash_stop || self.active >= p.crlq07_active_ticks {
            out.end = Some(RushEnd::Miss);
            self.phase = RushPhase::Recover;
            self.left = p.crlq09_recover_ticks;
        }
    }
}

/// The first hero whose hurtbox the box overlaps. When both overlap on one
/// tick, the hero the box front passed first takes the blow: the one nearest
/// the llama, that is the largest penetration `dir * (front - near edge)`.
/// Exact ties go to Carl (the lower index). CRL-Q14 ruling.
fn first_hero_in_box(boxed: Rect, dir: i8, heroes: &[RushHero]) -> Option<HeroId> {
    let front = if dir >= 0 { boxed.right() } else { boxed.left };
    let mut best: Option<(f32, HeroId)> = None;
    for hero in heroes.iter().filter(|h| h.hittable) {
        if !boxed.overlaps(hero.hurtbox) {
            continue;
        }
        let near = if dir >= 0 {
            hero.hurtbox.left
        } else {
            hero.hurtbox.right()
        };
        let depth = f32::from(dir) * (front - near);
        let better = match best {
            None => true,
            Some((d, id)) => depth > d || (depth == d && hero.id.index() < id.index()),
        };
        if better {
            best = Some((depth, hero.id));
        }
    }
    best.map(|(_, id)| id)
}

/// The cooldowns a finished rush (the end of Recover or Bonk) sets: `rush`
/// to the data value and the spit cooldown raised to at least the gap
/// (CRL-Q12). Returns `(rush_cooldown, cooldown)`.
pub fn cooldowns_after_rush(p: &RushProfile, cooldown: u16) -> (u16, u16) {
    (p.crlq12_rush_cooldown, cooldown.max(p.crlq12_spit_gap))
}

/// CRL-Q13: an interrupted tell raises `rush_cooldown` to at least the data
/// floor (the legacy `Hurt` path raises the spit cooldown on its own).
pub fn rush_cooldown_after_interrupt(p: &RushProfile, rush_cooldown: u16) -> u16 {
    rush_cooldown.max(p.crlq13_interrupt_cooldown)
}

// ---------------------------------------------------------------- hit table --

/// What a hero hit does to a corral llama (the F10 table, section 5). The hit
/// path branches on this before it touches the llama.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitReaction {
    /// The legacy path: `Hurt` 18, the slide, the hurt cooldown; lethal is the
    /// legacy `Down`.
    Legacy,
    /// The legacy path, and an interrupted tell: `rush_cooldown >= 30` and
    /// `LlamaInterrupted` is logged.
    Interrupt,
    /// Damage and hit-stop by the hit's split only: no mode change, no `vx`,
    /// `attack_tick` and the phase untouched, so the rush continues on the next
    /// unfrozen tick.
    DamageOnly,
    /// A lethal hit while committed or bonking: `Down` on the spot with the
    /// legacy death slide (clamped by CRL-Q43). The rush ends with no end
    /// event and the box is gone that tick.
    DownInPlace,
}

/// The F10 table with the gauntlet's pierce (CP-Q40, KC12): a non-lethal hit by
/// a move that `pierces_commit` takes a rush out of its committed phase like an
/// interrupted tell (`Interrupt`: the stagger, the rush cooldown floor and
/// `LlamaInterrupted`). A bonk is not a committed phase (the Engineer's bonk
/// rule), and every other state is the plain table.
pub fn hit_reaction_pierced(
    rush: Option<&RushState>,
    lethal: bool,
    pierces_commit: bool,
) -> HitReaction {
    if pierces_commit && !lethal && rush.is_some_and(|r| r.phase == RushPhase::Committed) {
        return HitReaction::Interrupt;
    }
    hit_reaction(rush, lethal)
}

/// The F10 table. `rush` is the llama's rush state if it is in a rush attack
/// (`None` in `Neutral`, a spit `Attack`, `Hurt`: the legacy path).
pub fn hit_reaction(rush: Option<&RushState>, lethal: bool) -> HitReaction {
    match rush.map(|r| r.phase) {
        Some(RushPhase::Tell) if !lethal => HitReaction::Interrupt,
        Some(RushPhase::Committed | RushPhase::Bonk) => {
            if lethal {
                HitReaction::DownInPlace
            } else {
                HitReaction::DamageOnly
            }
        }
        _ => HitReaction::Legacy,
    }
}

// ---------------------------------------------------------------- activation --

/// Plan R1, CRL-Q24: the per-llama activation state. A dormant llama idles and
/// holds its position; once awake it stays awake for the attempt. Restored on
/// retry; mixed into the hash only while the profile is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    Dormant,
    Awake,
}

impl Activation {
    pub fn code(self) -> u64 {
        match self {
            Self::Dormant => 0,
            Self::Awake => 1,
        }
    }

    pub fn awake(self) -> bool {
        self == Self::Awake
    }

    /// One tick: a dormant llama wakes when its whole body is inside the
    /// simulation view and a living hero is within the wake distance (pivot to
    /// pivot). Returns true on the tick it wakes. An awake llama stays awake.
    pub fn step(
        &mut self,
        p: &RushProfile,
        body: Rect,
        llama_x: f32,
        view: FireView,
        living_hero_xs: impl IntoIterator<Item = f32>,
    ) -> bool {
        if *self == Self::Awake {
            return false;
        }
        let in_view = body.left >= view.left && body.right() <= view.right;
        let near = living_hero_xs
            .into_iter()
            .any(|hx| (hx - llama_x).abs() <= f32::from(p.crlq24_wake_px));
        if in_view && near {
            *self = Self::Awake;
            return true;
        }
        false
    }
}

// ---------------------------------------------------------------- the brain --

/// What the brain reads of one hero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeroFacts {
    pub id: HeroId,
    pub x: f32,
    pub hurtbox: Rect,
    /// Her support height: her feet minus `feet_height_above_solid`, the top
    /// of the highest intact solid under her collider. A hero mid-hop over the
    /// pen floor is still a floor hero (F6).
    pub support: f32,
    pub grounded: bool,
    pub alive: bool,
    /// Added to her effective distance (the solo AI bias on the hero the
    /// player does not control).
    pub bias_px: f32,
}

/// What the brain reads of the llama.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LlamaFacts {
    pub x: f32,
    pub feet: f32,
    pub vx_zero: bool,
    /// The legacy spit cooldown.
    pub cooldown: u16,
    pub rush_cooldown: u16,
    pub activation: Activation,
}

/// What the brain reads of the scene.
#[derive(Debug, Clone, Copy)]
pub struct DecideEnv<'a> {
    pub solids: &'a [Block],
    pub partition: Option<usize>,
    pub view: FireView,
    /// The hero the llama pointed at last (hysteresis); `None` at first sight.
    pub held: Option<HeroId>,
    /// The party's `target_switch_px`.
    pub switch_px: f32,
}

/// How a hero reads to a llama.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeroClass {
    /// Her support height is within the floor tolerance of the llama's floor
    /// line, airborne or not.
    pub rushable: bool,
    /// The glob band overlaps her hurtbox as if she stood at her support
    /// height, and nothing solid lies on the glob's path (CRL-Q11).
    pub spittable: bool,
}

/// What the brain chose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    /// Start the rush tell at the target (locks facing, direction, target).
    Rush,
    /// Start the legacy spit tell.
    Spit,
    /// Walk toward the target at the profile's speed (the caller checks the
    /// leash and solids with [`walk_step`]).
    Walk {
        dir: i8,
    },
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decision {
    pub target: Option<HeroId>,
    /// The way it faces: toward the target; `None` keeps the current facing.
    pub facing: Option<i8>,
    pub action: Action,
}

/// The hero's class for `llama` (section 5, step 2). `facing` is the way the
/// llama would face her.
pub fn classify(
    p: &RushProfile,
    llama: &LlamaFacts,
    facing: i8,
    hero: &HeroFacts,
    solids: &[Block],
) -> HeroClass {
    let rushable = (hero.support - llama.feet).abs() <= f32::from(p.crlq16_floor_tolerance_px);
    let spittable = !p.crlq11_spit_height_check || {
        // The glob band [feet + 42, feet + 48) against the hurtbox as if she
        // stood at her support height, and a clear glob path.
        let at_support = Rect {
            bottom: hero.support,
            ..hero.hurtbox
        };
        glob_band_overlaps(llama.feet, at_support)
            && glob_lane_clear(llama.x, facing, llama.feet, hero.x, solids)
    };
    HeroClass {
        rushable,
        spittable,
    }
}

/// The corral brain for one `Neutral` tick (section 5, selection). Pure: the
/// caller applies the action.
///
/// 1. Dormant: hold.
/// 2. Candidates: living heroes within the target range; classify each.
/// 3. Target: the party rule over the rushable ones, else the spittable ones,
///    else all candidates (for facing only); with none, hold.
/// 4. Rush when allowed (cooldowns, still, rushable and grounded target,
///    inside the trigger, the fair-rush view rule, a clear lane).
/// 5. Else spit when the target is spittable and the legacy fair-fire holds.
/// 6. Else walk: toward a rushable target until the approach distance, toward
///    a spittable-only target until the stand-off, never backward.
pub fn decide(
    p: &RushProfile,
    llama: &LlamaFacts,
    heroes: &[HeroFacts],
    env: &DecideEnv,
) -> Decision {
    let hold = Decision {
        target: None,
        facing: None,
        action: Action::Hold,
    };
    if !llama.activation.awake() {
        return hold;
    }
    let range = f32::from(p.crlq15_target_range_px);
    let mut candidates: Vec<(&HeroFacts, HeroClass)> = Vec::with_capacity(2);
    for hero in heroes
        .iter()
        .filter(|h| h.alive && (h.x - llama.x).abs() <= range)
    {
        let toward: i8 = if hero.x - llama.x > 0.0 { 1 } else { -1 };
        candidates.push((hero, classify(p, llama, toward, hero, env.solids)));
    }
    let rushable: Vec<usize> = (0..candidates.len())
        .filter(|&i| candidates[i].1.rushable)
        .collect();
    let spittable: Vec<usize> = (0..candidates.len())
        .filter(|&i| candidates[i].1.spittable)
        .collect();
    let pool: Vec<usize> = if !rushable.is_empty() {
        rushable
    } else if !spittable.is_empty() {
        spittable
    } else {
        (0..candidates.len()).collect()
    };
    let Some(chosen) = pick_target(llama.x, &candidates, &pool, env.held, env.switch_px) else {
        return hold;
    };
    let (hero, class) = candidates[chosen];
    let dx = hero.x - llama.x;
    let dir: i8 = if dx > 0.0 { 1 } else { -1 };
    let decision = |action| Decision {
        target: Some(hero.id),
        facing: Some(dir),
        action,
    };
    // Rush.
    if llama.rush_cooldown == 0
        && llama.cooldown == 0
        && llama.vx_zero
        && class.rushable
        && hero.grounded
        && dx.abs() <= f32::from(p.crlq05_trigger_px)
        && fair_rush_view(p, llama.x, dir, hero.hurtbox, env.view)
        && rush_lane_clear(
            p,
            llama.x,
            llama.feet,
            dir,
            hero.x,
            env.solids,
            env.partition,
        )
    {
        return decision(Action::Rush);
    }
    // Spit.
    if llama.cooldown == 0
        && llama.vx_zero
        && class.spittable
        && legacy_may_fire(llama.x, dir, dx, env.view)
    {
        return decision(Action::Spit);
    }
    // Walk.
    let stop = if class.rushable {
        f32::from(p.crlq04_rush_approach_px)
    } else if class.spittable {
        f32::from(p.crlq04_spit_standoff_px)
    } else {
        return decision(Action::Hold);
    };
    if dx.abs() > stop {
        decision(Action::Walk { dir })
    } else {
        decision(Action::Hold)
    }
}

/// The hysteresis rule over `pool` (indices into `candidates`): the nearest by
/// effective distance (pivot distance plus the AI bias), ties to Carl; a held
/// hero is kept unless the other is `switch_px` closer.
fn pick_target(
    llama_x: f32,
    candidates: &[(&HeroFacts, HeroClass)],
    pool: &[usize],
    held: Option<HeroId>,
    switch_px: f32,
) -> Option<usize> {
    let effective = |i: usize| (candidates[i].0.x - llama_x).abs() + candidates[i].0.bias_px;
    let nearest = pool.iter().copied().reduce(|a, b| {
        let (ea, eb) = (effective(a), effective(b));
        if eb < ea || (eb == ea && candidates[b].0.id.index() < candidates[a].0.id.index()) {
            b
        } else {
            a
        }
    })?;
    let Some(held) = held else {
        return Some(nearest);
    };
    let Some(kept) = pool.iter().copied().find(|&i| candidates[i].0.id == held) else {
        return Some(nearest);
    };
    let other = pool.iter().copied().find(|&i| i != kept);
    match other {
        Some(o) if effective(o) + switch_px <= effective(kept) => Some(o),
        _ => Some(kept),
    }
}

/// The party's enemy-target rule over the two heroes (KC1, N-a): the nearest by
/// effective distance (pivot distance plus the solo AI bias), ties to Carl; with
/// `switch_px` (the hysteresis rule) a held hero is kept unless the other is
/// `switch_px` closer. The goblins' `update_targets` and the llamas share this
/// one function, and a test pins [`pick_target`] to it.
pub fn pick_two(
    carl_effective: f32,
    donut_effective: f32,
    held: Option<HeroId>,
    switch_px: Option<f32>,
) -> HeroId {
    let nearest = if donut_effective < carl_effective {
        HeroId::Donut
    } else {
        HeroId::Carl
    };
    let effective = |hero: HeroId| match hero {
        HeroId::Carl => carl_effective,
        HeroId::Donut => donut_effective,
    };
    match (switch_px, held) {
        (Some(switch), Some(held)) => {
            let (kept, other) = (effective(held), effective(held.other()));
            if other + switch <= kept {
                held.other()
            } else {
                held
            }
        }
        _ => nearest,
    }
}

/// CRL-Q17: the llama body, the whole swept lane `[start body, start pivot +
/// 96 + 28]` and the target are inside the simulation view.
pub fn fair_rush_view(p: &RushProfile, x: f32, dir: i8, target: Rect, view: FireView) -> bool {
    let body = llama_body(x, 0.0, dir);
    let reach = x + f32::from(dir) * (p.rush_distance() + f32::from(p.crlq08_box.x1));
    let lo = body.left.min(reach);
    let hi = body.right().max(reach);
    lo >= view.left && hi <= view.right && target.left >= view.left && target.right() <= view.right
}

/// The legacy fair-fire rule's view and range part (`Llama::may_fire`): the
/// whole body inside the view, the target within the fire range in the way the
/// llama faces (the brain faces the target).
fn legacy_may_fire(x: f32, dir: i8, dx: f32, view: FireView) -> bool {
    let body = llama_body(x, 0.0, dir);
    body.left >= view.left
        && body.right() <= view.right
        && dx.abs() <= crate::feel::LLAMA_FIRE_RANGE
        && dx * f32::from(dir) > 0.0
}

// ----------------------------------------------------------------- AI lane --

/// The read-only accessor of the AI partner (KC7): where the rush box will be.
/// Nothing in `companion/**` reads it yet; the CA2 step appends the dodge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RushLane {
    pub phase: RushPhase,
    pub dir: i8,
    pub feet: f32,
    /// The swept extent of the box from now to its end, leash and solids
    /// applied.
    pub x_from: f32,
    pub x_to: f32,
    /// `feet + y0 .. feet + y1`.
    pub y_from: f32,
    pub y_to: f32,
    /// Ticks from now until the first motion step (0 = the next step; 0 while
    /// already committed).
    pub ticks_to_motion: u16,
    /// Motion steps still to run from the first one, leash and solids applied.
    pub active_left: u16,
    /// The box's back and front edge at the first motion step, the step and the
    /// distance the box travels after the first step.
    box_back: f32,
    box_front: f32,
    step: f32,
    travel: f32,
}

/// The lane of a rushing llama at pivot `x`, or `None` outside the tell and the
/// committed phase (recover and bonk have no box). It replays the machine's own
/// motion, so the lane is exactly what the next steps do against `solids`.
pub fn rush_lane(
    p: &RushProfile,
    state: &RushState,
    x: f32,
    feet: f32,
    solids: &[Block],
    leash: Leash,
    max_x: f32,
) -> Option<RushLane> {
    let ticks_to_motion = match state.phase {
        RushPhase::Tell => p.crlq06_tell_ticks.saturating_sub(state.tick + 1),
        RushPhase::Committed => 0,
        RushPhase::Recover | RushPhase::Bonk => return None,
    };
    let remaining = p.crlq07_active_ticks.saturating_sub(state.active);
    let step = p.px_per_tick();
    let mut pivot = x;
    let (mut first_back, mut first_front) = (0.0, 0.0);
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    let mut steps = 0u16;
    let mut travel = 0.0;
    for i in 0..remaining {
        let m = rush_motion(p, pivot, state.dir, feet, solids, leash, max_x);
        let boxed = rush_box(p, m.x, state.dir, feet);
        if i == 0 {
            first_back = boxed.left;
            first_front = boxed.right();
        } else {
            travel += (m.x - pivot).abs();
        }
        lo = lo.min(boxed.left);
        hi = hi.max(boxed.right());
        steps += 1;
        pivot = m.x;
        if m.met || m.leash_stop {
            break;
        }
    }
    if steps == 0 {
        return None;
    }
    Some(RushLane {
        phase: state.phase,
        dir: state.dir,
        feet,
        x_from: lo,
        x_to: hi,
        y_from: feet + f32::from(p.crlq08_box.y0),
        y_to: feet + f32::from(p.crlq08_box.y1),
        ticks_to_motion,
        active_left: steps,
        box_back: first_back,
        box_front: first_front,
        step,
        travel,
    })
}

impl RushLane {
    /// The box's x span after motion step `i` (0-based from the first step):
    /// the box moves `step` per tick and stops at the end of the lane.
    fn span(&self, i: u16) -> (f32, f32) {
        let offset = (f32::from(i) * self.step).min(self.travel) * f32::from(self.dir);
        (self.box_back + offset, self.box_front + offset)
    }

    fn overlaps_column(&self, i: u16, hero_x: f32, half_w: f32) -> bool {
        let (a, b) = self.span(i);
        let (lo, hi) = (a.min(b), a.max(b));
        lo < hero_x + half_w && hi > hero_x - half_w
    }
}

/// Ticks from now (0 = the next step) until the box first overlaps a hero
/// column `hero_x ± half_w`, or `None` if it never does.
pub fn ticks_until_box_reaches(lane: &RushLane, hero_x: f32, half_w: f32) -> Option<u16> {
    (0..lane.active_left)
        .find(|&i| lane.overlaps_column(i, hero_x, half_w))
        .map(|i| lane.ticks_to_motion + i)
}

/// Ticks from now (0 = the next step) until the box has left a hero column:
/// the first tick after its last overlap, or `None` if it never overlaps.
/// A box still overlapping when the rush ends vanishes the tick after its
/// last step.
pub fn ticks_until_box_clears(lane: &RushLane, hero_x: f32, half_w: f32) -> Option<u16> {
    let first = (0..lane.active_left).find(|&i| lane.overlaps_column(i, hero_x, half_w))?;
    let last = (first..lane.active_left)
        .take_while(|&i| lane.overlaps_column(i, hero_x, half_w))
        .last()
        .unwrap_or(first);
    Some(lane.ticks_to_motion + last + 1)
}
