//! HAZ-A1: the Slime's knobs and Bevy-free domain, and the acid patch
//! (`docs/01-design/enemy-gameplay-slime-rot.md`, HAZ-Q01..HAZ-Q50).
//!
//! The Slime is a slot enemy (ENEMY-SLOT wires it in HAZ-A2): a slow, heavy
//! zone guard with a corrosive body, a low lunge and a gel body that never
//! flinches. This module owns everything that needs no scene:
//!
//! * [`SlimeTuning`]: every `hazq##` knob of the slime and acid half of
//!   `assets/data/gameplay/enemy_slime_v0.json`, loaded, validated and FNV-1a
//!   pinned like the Engineer's file (the Rot Sticker half is
//!   `enemy_rot_sticker_v0.json`, [`crate::rot_sticker`]);
//! * the attack timeline ([`SlimePhase`]) and the per-state boxes
//!   ([`Slime::hurtbox`], [`Slime::contact_box`], [`Slime::lunge_box`]);
//! * the brain (wake, eligibility, creep, standoff, leash, token candidacy)
//!   and the impact tick (lunge candidates, the reflex callback of HAZ-B,
//!   corrosive body contact) in [`Slime::step`];
//! * gel ([`Slime::hit`]) and the push-kick slide ([`Slime::push_kick`]);
//! * the acid patch ([`AcidPatch`]): an always-on persistent volume of class
//!   `Acid` on [`crate::env_hazard`]'s volume, the per-entry token, the pop,
//!   the 60-tick drain and the per-launch token.
//!
//! Frames: the simulation world is y up (`Rect`, feet at `foot_y`); the acid
//! patch uses the layout convention of hazard volumes (y down, `IRect`).
//! Authoring facing is +x; every box mirrors about the pivot column.
//!
//! Everything is deterministic: no RNG, no wall-clock time. The slime has no
//! persistent state beyond what [`Slime::mix_into`] hashes.
//!
//! The Slime exists only when `cf02_rules` is on and the hazard scene lists it
//! (HAZ-Q02); every switch is read through [`SlimeTuning::on`], never raw.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::boss::{HeroBlow, HeroView};
use crate::camera_lab::room::IRect;
use crate::cf02::BoxPx;
use crate::combat::solid_at;
use crate::donut_mana::content_hash;
use crate::env_hazard::{
    EntryTokens, HazardClass, HazardSpec, HazardVolume, HazardWindow, LaunchTokens, NavRect, Victim,
};
use crate::hero_damage::HeroDamagePolicy;
use crate::party::HeroId;
use crate::prototype::{Block, Rect};

/// The tuning file, embedded so the binary and the tests read the same bytes.
pub const TUNING_JSON: &str = include_str!("../assets/data/gameplay/enemy_slime_v0.json");

/// The committed FNV-1a 64 content hash of [`TUNING_JSON`]. Editing the file
/// without updating this value fails [`SlimeTuning::pinned`] and the pin test.
pub const TUNING_HASH: u64 = 0xa68d_8a48_11e1_6901;

/// The key of the pin in a session manifest's `content_hashes`.
pub const PIN_KEY: &str = "slime_tuning";

/// The `HeroBlow::source` base of the slime's lunge and corrosive body (the
/// ASCII "SL"). The instance index goes in the high half ([`slime_source`]).
pub const SLIME_SOURCE_ID: u32 = 0x534c;
/// The `HeroBlow::source` base of an acid patch (the ASCII "AC"); the patch id
/// goes in the high half ([`acid_source`]).
pub const ACID_SOURCE_ID: u32 = 0x4143;

/// The source of slime instance `index`.
pub const fn slime_source(index: u8) -> u32 {
    SLIME_SOURCE_ID | ((index as u32) << 16)
}

/// The source of the acid patch `patch_id`.
pub const fn acid_source(patch_id: u16) -> u32 {
    ACID_SOURCE_ID | ((patch_id as u32) << 16)
}

/// HAZ questions that have no key in **either** data file, and why. The
/// completeness test (in `rot_sticker.rs`, which reads both files) requires
/// every question of 1..=50 to own at least one `hazqNN_` key in one of the two
/// files, or to be listed here (and then own none).
pub const NO_KNOB: [(u8, &str); 23] = [
    (1, "design: the roles of the two kinds"),
    (3, "validator: rule 5 of the encounter validator"),
    (
        14,
        "code: the tokens are the goblins' MAX_ATTACKERS and tell spacing",
    ),
    (
        16,
        "data: the slime species in the death presentation (HAZ-B)",
    ),
    (17, "design: no splitting, no trail, no slime-made hazard"),
    (22, "design: acid is always active and always visible"),
    (23, "validator: rule 3 and the sidecar track"),
    (32, "rules: the standard explosion pipeline"),
    (33, "validator: rule 4 (separation) and CP-Q34"),
    (34, "rules: the initiator rule of BlastCharge.owner"),
    (36, "design: a Rot Sticker is not an enemy"),
    (
        37,
        "presentation: the frame table of rot_sticker.rs (HAZ-B)",
    ),
    (39, "sidecar: the defeat_roster goal"),
    (40, "sidecar: pocket A layout and validator rules 1, 2, 6"),
    (41, "sidecar: pocket B layout"),
    (42, "sidecar: pocket C layout and validator"),
    (43, "validator: rule 3 (the camera band)"),
    (44, "design: the companion row is ENEMY-SLOT's (HAZ-B)"),
    (45, "report: the AI-hero hazard-hurt threshold"),
    (46, "design: no randomness in either kind"),
    (48, "rules: one walk barrier for every walker"),
    (49, "rules: restart and respawn (section 15)"),
    (50, "sidecar: the R2 start point"),
];

/// A knob file problem. A load error names the field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlimeTuningError {
    Parse(String),
    Field { field: &'static str, reason: String },
    HashMismatch { expected: u64, actual: u64 },
}

impl core::fmt::Display for SlimeTuningError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "slime tuning: {msg}"),
            Self::Field { field, reason } => write!(f, "slime tuning: field `{field}`: {reason}"),
            Self::HashMismatch { expected, actual } => write!(
                f,
                "slime tuning hash {actual:#018x} is not the pinned {expected:#018x}"
            ),
        }
    }
}

impl std::error::Error for SlimeTuningError {}

/// Every slime and acid knob of the HAZ-0 sheet, section 16.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlimeTuning {
    pub schema_version: u32,
    /// Q02: the data kill switch of the slime.
    pub hazq02_slime_enabled: bool,
    /// Q04: health.
    pub hazq04_slime_health: u8,
    /// Q05: the boxes, pivot-relative, facing +x, y up from the feet.
    pub hazq05_hurtbox_idle: BoxPx,
    pub hazq05_hurtbox_tell: BoxPx,
    pub hazq05_contact_standing: BoxPx,
    pub hazq05_contact_gathered: BoxPx,
    pub hazq05_lunge_box: BoxPx,
    /// Q06: wake distance, creep speed (px/s) and leash.
    pub hazq06_sight_px: u16,
    pub hazq06_creep_px_s: u16,
    pub hazq06_leash_px: u16,
    /// Q07: a hero is eligible while its feet are this close to the slime's floor.
    pub hazq07_floor_tolerance_px: u8,
    /// Q08: the tell starts at this pivot distance or less, and the standoff.
    pub hazq08_trigger_px: u16,
    pub hazq08_standoff_px: u16,
    /// Q09: the timeline.
    pub hazq09_tell_rise_ticks: u16,
    pub hazq09_tell_hold_ticks: u16,
    pub hazq09_launch_ticks: u16,
    pub hazq09_impact_ticks: u16,
    pub hazq09_recover_ticks: u16,
    pub hazq09_cooldown_ticks: u16,
    /// Q10: the lunge's damage and knock (the caller signs `knock_x`).
    pub hazq10_damage: u8,
    pub hazq10_knock_x: u16,
    pub hazq10_knock_y: u16,
    /// Q11: gel (a hit never flinches it).
    pub hazq11_gel: bool,
    /// Q12: the push kick slide.
    pub hazq12_kick_slide_px: u16,
    pub hazq12_kick_slide_ticks: u16,
    /// Q13: the corrosive body.
    pub hazq13_body_contact: bool,
    /// Q15: Donut's reflex may evade the lunge.
    pub hazq15_lunge_evadable: bool,
    /// Q18: the acid volume's height and the allowed patch widths.
    pub hazq18_volume_height_px: u8,
    pub hazq18_width_min_px: u16,
    pub hazq18_width_max_px: u16,
    /// Q19: acid contact damage and pop (the caller signs `pop_x`).
    pub hazq19_damage: u8,
    pub hazq19_pop_x: u16,
    pub hazq19_pop_y: u16,
    /// Q20: launched goblins landing in acid are hurt once per launch.
    pub hazq20_acid_hurts_launched: bool,
    /// Q21: the drain of a linked pool, in ticks.
    pub hazq21_drain_ticks: u16,
    /// HAZ-FU (PR-D) knobs: each default is the new behaviour, and `false` is the
    /// behaviour (and the state hashes) of the build before it.
    ///
    /// D6: a launched body that meets a slime stops against it (sheet section 6).
    pub hazfu01_flier_contact: bool,
    /// A goblin that landed inside a pool walks out by the nearer edge.
    pub hazfu02_acid_exit: bool,
    /// A non-lethal acid landing on a goblin logs `CombatEvent::EnvironmentHurt`.
    pub hazfu03_acid_hurt_event: bool,
    /// A pool drains on the up -> down edge of its slime only (not every tick the
    /// slime is down), and a slime that was never alive (the kill switch, AD6)
    /// has its pool removed at once.
    pub hazfu04_pool_edge: bool,
    /// FA-Q21 (fix-all ruling, T28): the reach in px of **Carl's dynamite** onto a Rot
    /// Sticker's charge (CP-Q34's chain, centre to centre, line of sight unchanged). A floor
    /// stick is at least 70.007 px from the dome of R2 under the 84 px soffit, over the
    /// dynamite's outer radius of 64, so pattern 3 (sheet section 15) was unreachable. 76 gives
    /// a horizontal window of about +-29 px. `0` is off: the blast's `outer_radius`. Applies
    /// only dynamite -> sticker; a drum, a sticker's own blast and the blast's damage radius
    /// keep `outer_radius`.
    pub hazfu05_sticker_chain_reach_px: u16,
}

fn bad(field: &'static str, reason: &str) -> Result<(), SlimeTuningError> {
    Err(SlimeTuningError::Field {
        field,
        reason: reason.to_string(),
    })
}

fn positive(field: &'static str, value: u32) -> Result<(), SlimeTuningError> {
    if value == 0 {
        return bad(field, "must be positive");
    }
    Ok(())
}

fn valid_box(field: &'static str, b: &BoxPx) -> Result<(), SlimeTuningError> {
    if b.x0 >= b.x1 || b.y0 >= b.y1 {
        return bad(field, "needs x0 < x1 and y0 < y1");
    }
    Ok(())
}

impl SlimeTuning {
    /// Parse and validate.
    pub fn from_json(text: &str) -> Result<Self, SlimeTuningError> {
        let tuning: Self =
            serde_json::from_str(text).map_err(|err| SlimeTuningError::Parse(err.to_string()))?;
        tuning.validate()?;
        Ok(tuning)
    }

    /// Parse the embedded file and require the pinned content hash.
    pub fn pinned() -> Result<Self, SlimeTuningError> {
        Self::from_json_pinned(TUNING_JSON, TUNING_HASH)
    }

    /// Parse `text` and require its content hash to equal `expected`.
    pub fn from_json_pinned(text: &str, expected: u64) -> Result<Self, SlimeTuningError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(SlimeTuningError::HashMismatch { expected, actual });
        }
        Self::from_json(text)
    }

    /// The one way to read a switch of the file: effective only while the data
    /// kill switch `hazq02_slime_enabled` is on.
    pub fn on(&self, switch: bool) -> bool {
        self.hazq02_slime_enabled && switch
    }

    /// Whether the slime exists at all (the kill switch).
    pub fn enabled(&self) -> bool {
        self.hazq02_slime_enabled
    }

    /// Creep in px per tick.
    pub fn creep_px_per_tick(&self) -> f32 {
        f32::from(self.hazq06_creep_px_s) / 60.0
    }

    /// Kick slide in px per tick.
    pub fn kick_px_per_tick(&self) -> f32 {
        f32::from(self.hazq12_kick_slide_px) / f32::from(self.hazq12_kick_slide_ticks)
    }

    /// The tell is rise + hold ticks.
    pub fn tell_len(&self) -> u16 {
        self.hazq09_tell_rise_ticks + self.hazq09_tell_hold_ticks
    }

    /// Attack tick on which the launch begins (the tell length).
    pub fn launch_start(&self) -> u16 {
        self.tell_len()
    }

    /// Attack tick on which the impact begins (the first dangerous tick).
    pub fn impact_start(&self) -> u16 {
        self.tell_len() + self.hazq09_launch_ticks
    }

    /// Attack tick on which the recover begins (the impact is over).
    pub fn recover_start(&self) -> u16 {
        self.impact_start() + self.hazq09_impact_ticks
    }

    /// Attack tick on which the cooldown begins (the slime turns here).
    pub fn cooldown_start(&self) -> u16 {
        self.recover_start() + self.hazq09_recover_ticks
    }

    /// Attack ticks in all: the attack ends and the slime is idle again.
    pub fn total_ticks(&self) -> u16 {
        self.cooldown_start() + self.hazq09_cooldown_ticks
    }

    /// Recover plus cooldown: the punish window.
    pub fn punish_window(&self) -> u16 {
        self.hazq09_recover_ticks + self.hazq09_cooldown_ticks
    }

    pub fn validate(&self) -> Result<(), SlimeTuningError> {
        positive("hazq04_slime_health", u32::from(self.hazq04_slime_health))?;
        valid_box("hazq05_hurtbox_idle", &self.hazq05_hurtbox_idle)?;
        valid_box("hazq05_hurtbox_tell", &self.hazq05_hurtbox_tell)?;
        valid_box("hazq05_contact_standing", &self.hazq05_contact_standing)?;
        valid_box("hazq05_contact_gathered", &self.hazq05_contact_gathered)?;
        valid_box("hazq05_lunge_box", &self.hazq05_lunge_box)?;
        positive("hazq06_sight_px", u32::from(self.hazq06_sight_px))?;
        positive("hazq06_creep_px_s", u32::from(self.hazq06_creep_px_s))?;
        if self.hazq06_creep_px_s > 120 {
            return bad(
                "hazq06_creep_px_s",
                "a zone guard creeps at 120 px/s or less",
            );
        }
        positive("hazq06_leash_px", u32::from(self.hazq06_leash_px))?;
        positive("hazq08_trigger_px", u32::from(self.hazq08_trigger_px))?;
        if self.hazq08_standoff_px > self.hazq08_trigger_px {
            return bad("hazq08_standoff_px", "must not exceed hazq08_trigger_px");
        }
        positive(
            "hazq09_tell_rise_ticks",
            u32::from(self.hazq09_tell_rise_ticks),
        )?;
        positive(
            "hazq09_tell_hold_ticks",
            u32::from(self.hazq09_tell_hold_ticks),
        )?;
        positive("hazq09_launch_ticks", u32::from(self.hazq09_launch_ticks))?;
        positive("hazq09_impact_ticks", u32::from(self.hazq09_impact_ticks))?;
        positive("hazq09_recover_ticks", u32::from(self.hazq09_recover_ticks))?;
        positive(
            "hazq09_cooldown_ticks",
            u32::from(self.hazq09_cooldown_ticks),
        )?;
        // Owner (#178): clear anticipation of 24..36 ticks before the first harm.
        if !(24..=36).contains(&self.impact_start()) {
            return bad(
                "hazq09_tell_rise_ticks",
                "the first dangerous tick must come 24..=36 ticks after the tell starts",
            );
        }
        positive("hazq10_damage", u32::from(self.hazq10_damage))?;
        positive("hazq12_kick_slide_px", u32::from(self.hazq12_kick_slide_px))?;
        positive(
            "hazq12_kick_slide_ticks",
            u32::from(self.hazq12_kick_slide_ticks),
        )?;
        positive(
            "hazq18_volume_height_px",
            u32::from(self.hazq18_volume_height_px),
        )?;
        if self.hazq18_width_min_px == 0 || self.hazq18_width_min_px > self.hazq18_width_max_px {
            return bad("hazq18_width_min_px", "needs 0 < min <= max");
        }
        positive("hazq19_damage", u32::from(self.hazq19_damage))?;
        positive("hazq19_pop_x", u32::from(self.hazq19_pop_x))?;
        positive("hazq19_pop_y", u32::from(self.hazq19_pop_y))?;
        // The pop must clear the widest patch inside the hurt lock (section 8):
        // the lock carries the hero `pop_x * lock / 60` px, and leaving a patch
        // from its centre takes half the width plus a hero's half-width.
        let carried = u32::from(self.hazq19_pop_x) * u32::from(crate::feel::HERO_HURT_TICKS) / 60;
        let needed = u32::from(self.hazq18_width_max_px) / 2 + 12;
        if carried < needed {
            return bad(
                "hazq19_pop_x",
                "the pop must clear the widest patch (hazq18_width_max_px) inside the hurt lock",
            );
        }
        positive("hazq21_drain_ticks", u32::from(self.hazq21_drain_ticks))?;
        Ok(())
    }
}

/// The pinned tuning: what a simulation with a slime reads. One shared static.
pub fn active() -> &'static SlimeTuning {
    #[cfg(test)]
    if let Some(t) = TEST_TUNING.with(std::cell::Cell::get) {
        return t;
    }
    static ACTIVE: std::sync::OnceLock<SlimeTuning> = std::sync::OnceLock::new();
    ACTIVE
        .get_or_init(|| SlimeTuning::pinned().expect("the embedded slime data is pinned and valid"))
}

#[cfg(test)]
thread_local! {
    /// The tuning one test thread runs under (`with_knobs_for_test`).
    static TEST_TUNING: std::cell::Cell<Option<&'static SlimeTuning>> =
        const { std::cell::Cell::new(None) };
}

/// Tests only (HAZ-FU): run `f` on this thread under the pinned tuning with `edit` applied,
/// so a knob-off run builds and steps the whole room under the old behaviour. The previous
/// tuning is restored afterwards, also when `f` panics.
#[cfg(test)]
pub(crate) fn with_knobs_for_test<R>(
    edit: impl FnOnce(&mut SlimeTuning),
    f: impl FnOnce() -> R,
) -> R {
    struct Restore(Option<&'static SlimeTuning>);
    impl Drop for Restore {
        fn drop(&mut self) {
            TEST_TUNING.with(|c| c.set(self.0));
        }
    }
    let mut t = active().clone();
    edit(&mut t);
    let _restore = Restore(TEST_TUNING.with(|c| c.replace(Some(Box::leak(Box::new(t))))));
    f()
}

/// Tests only (HAZ-FU): every follow-up knob off, the build before PR-D.
#[cfg(test)]
pub(crate) fn hazfu_off(t: &mut SlimeTuning) {
    t.hazfu01_flier_contact = false;
    t.hazfu02_acid_exit = false;
    t.hazfu03_acid_hurt_event = false;
    t.hazfu04_pool_edge = false;
    t.hazfu05_sticker_chain_reach_px = 0;
}

// ------------------------------------------------------------------ phases

/// The phase of the slime's attack timeline (HAZ-Q09).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlimePhase {
    /// No attack: idle and creep.
    Idle,
    /// SLIME-01 frame 7: the half-rise.
    TellRise,
    /// Frame 8: reared up and back.
    TellHold,
    /// Frame 9: committed, no motion.
    Launch,
    /// Frame 10: the lunge box is live.
    Impact,
    /// Frame 11: bunched and spent; facing still locked.
    Recover,
    /// Idle or creep again, turned to the target, never attacking.
    Cooldown,
}

impl SlimePhase {
    /// Mixed into the state hash.
    pub const fn code(self) -> u64 {
        self as u64
    }

    /// The tell, launch and impact: the rearing pose (the `boss` row of the
    /// companion view, HAZ-B).
    pub const fn is_rearing(self) -> bool {
        matches!(
            self,
            Self::TellRise | Self::TellHold | Self::Launch | Self::Impact
        )
    }

    /// The tell and launch use the pulled-back reared hurtbox.
    pub const fn reared_hurtbox(self) -> bool {
        matches!(self, Self::TellRise | Self::TellHold | Self::Launch)
    }

    /// The tell, launch and impact use the gathered contact box.
    pub const fn gathered_contact(self) -> bool {
        matches!(
            self,
            Self::TellRise | Self::TellHold | Self::Launch | Self::Impact
        )
    }
}

/// The phase on attack tick `tick`, `None` past the cooldown.
pub fn phase_at(t: &SlimeTuning, tick: u16) -> Option<SlimePhase> {
    Some(if tick < t.hazq09_tell_rise_ticks {
        SlimePhase::TellRise
    } else if tick < t.launch_start() {
        SlimePhase::TellHold
    } else if tick < t.impact_start() {
        SlimePhase::Launch
    } else if tick < t.recover_start() {
        SlimePhase::Impact
    } else if tick < t.cooldown_start() {
        SlimePhase::Recover
    } else if tick < t.total_ticks() {
        SlimePhase::Cooldown
    } else {
        return None;
    })
}

// ------------------------------------------------------------------- boxes

/// A pivot-relative box in the world: `facing` is +1 or -1, `feet` the y of the
/// slime's feet.
pub fn world_box(b: &BoxPx, x: f32, facing: i8, feet: f32) -> Rect {
    let (x0, x1) = (f32::from(b.x0), f32::from(b.x1));
    let left = if facing >= 0 { x + x0 } else { x - x1 };
    Rect {
        left,
        bottom: feet + f32::from(b.y0),
        width: x1 - x0,
        height: f32::from(b.y1 - b.y0),
    }
}

// ------------------------------------------------------------------- slime

/// What one hurt of the slime was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlimeHurt {
    Lunge,
    Body,
}

/// A hero hurt by the slime on this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlimeHit {
    pub hero: HeroId,
    pub kind: SlimeHurt,
    pub blow: HeroBlow,
}

/// What one slime step asks of the scene.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SlimeStep {
    /// The tell began on this tick (the scene plays the goblin tell cue and
    /// stamps the attack token).
    pub tell_started: bool,
    /// Heroes hurt on this tick, in resolution order (lunge first).
    pub hits: Vec<SlimeHit>,
    /// Heroes whose reflex evaded the lunge on this tick.
    pub evaded: Vec<HeroId>,
}

/// What the scene tells the slime each tick.
#[derive(Debug, Clone, Copy)]
pub struct SlimeCtx<'a> {
    pub heroes: &'a [HeroView],
    /// The slime's whole body is inside the simulation camera view.
    pub in_view: bool,
    /// The hero the scene's `update_targets` picked among
    /// [`Slime::eligible`] heroes (hysteresis, solo bias); `None` when there is
    /// no eligible hero.
    pub target: Option<HeroId>,
    pub solids: &'a [Block],
}

/// The outcome of a hit on the slime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitOutcome {
    /// The hit was lethal.
    pub defeated: bool,
    /// The 2-tick white flash (presentation).
    pub flash: bool,
    /// The SLIME-01 hurt frame (12 ticks), only when the hit lands in idle.
    pub hurt_frame: bool,
    /// Only with `hazq11_gel` off: the hit ended the attack (a goblin-style
    /// reaction, not used in v1).
    pub interrupted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Slide {
    left: u16,
    dir: f32,
}

/// One slime. The scene owns the instance list; ids index it.
#[derive(Debug, Clone, PartialEq)]
pub struct Slime {
    /// Pivot x.
    pub x: f32,
    /// World y of the floor line the slime stands on.
    pub foot_y: f32,
    pub facing: i8,
    /// The column the leash is measured from (its pool's centre or its spawn).
    pub home_x: f32,
    /// The instance index (roster order); the high half of the blow source.
    /// A provisional scheme: HAZ-A2 replaces it with ENEMY-SLOT's slot target id.
    pub index: u8,
    health: u8,
    defeated: bool,
    /// Attack tick, `None` while idle. Attack tick 0 = the tell starts.
    attack: Option<u16>,
    /// The hero locked at the tell start until the end of the recover.
    locked: Option<HeroId>,
    /// The lunge hurt a hero (one hero per lunge).
    lunge_done: bool,
    /// Heroes whose reflex evaded this lunge instance.
    evaded: [bool; 2],
    slide: Option<Slide>,
    contact: EntryTokens,
    flash_left: u8,
    hurt_frame_left: u8,
}

impl Slime {
    pub fn new(t: &SlimeTuning, x: f32, foot_y: f32, facing: i8, home_x: f32) -> Self {
        Self {
            x,
            foot_y,
            facing: if facing >= 0 { 1 } else { -1 },
            home_x,
            index: 0,
            health: t.hazq04_slime_health,
            defeated: false,
            attack: None,
            locked: None,
            lunge_done: false,
            evaded: [false; 2],
            slide: None,
            contact: EntryTokens::new(),
            flash_left: 0,
            hurt_frame_left: 0,
        }
    }

    /// The same slime with instance index `index`.
    pub fn with_index(mut self, index: u8) -> Self {
        self.index = index;
        self
    }

    pub fn health(&self) -> u8 {
        self.health
    }

    pub fn is_defeated(&self) -> bool {
        self.defeated
    }

    /// The attack tick (0 = the tell started), `None` while idle.
    pub fn attack_tick(&self) -> Option<u16> {
        self.attack
    }

    pub fn locked_target(&self) -> Option<HeroId> {
        self.locked
    }

    pub fn flash_left(&self) -> u8 {
        self.flash_left
    }

    pub fn hurt_frame_left(&self) -> u8 {
        self.hurt_frame_left
    }

    pub fn sliding(&self) -> bool {
        self.slide.is_some()
    }

    /// The phase now. A defeated slime reports `Idle` (it is inert).
    pub fn phase(&self, t: &SlimeTuning) -> SlimePhase {
        self.attack
            .and_then(|a| phase_at(t, a))
            .unwrap_or(SlimePhase::Idle)
    }

    /// The hurtbox for heroes' melee, missiles and blasts; `None` once defeated.
    pub fn hurtbox(&self, t: &SlimeTuning) -> Option<Rect> {
        if self.defeated {
            return None;
        }
        let b = if self.phase(t).reared_hurtbox() {
            &t.hazq05_hurtbox_tell
        } else {
            &t.hazq05_hurtbox_idle
        };
        Some(world_box(b, self.x, self.facing, self.foot_y))
    }

    /// The corrosive body; `None` once defeated or with the switch off.
    pub fn contact_box(&self, t: &SlimeTuning) -> Option<Rect> {
        if self.defeated || !t.on(t.hazq13_body_contact) {
            return None;
        }
        let b = if self.phase(t).gathered_contact() {
            &t.hazq05_contact_gathered
        } else {
            &t.hazq05_contact_standing
        };
        Some(world_box(b, self.x, self.facing, self.foot_y))
    }

    /// The lunge box, live on the impact ticks only.
    pub fn lunge_box(&self, t: &SlimeTuning) -> Option<Rect> {
        if self.defeated || self.phase(t) != SlimePhase::Impact {
            return None;
        }
        Some(world_box(
            &t.hazq05_lunge_box,
            self.x,
            self.facing,
            self.foot_y,
        ))
    }

    /// HAZ-Q07: living, present, and its feet within the tolerance of the
    /// slime's floor line. A jumping hero or a hero on a ledge is not a target.
    pub fn eligible(&self, t: &SlimeTuning, hero: &HeroView) -> bool {
        hero.present && (hero.foot_y - self.foot_y).abs() <= f32::from(t.hazq07_floor_tolerance_px)
    }

    /// HAZ-Q06: awake only with its whole body in view and a hero's hurtbox
    /// within the sight distance (horizontally, from the pivot).
    pub fn awake(&self, t: &SlimeTuning, ctx: &SlimeCtx<'_>) -> bool {
        ctx.in_view
            && !self.defeated
            && ctx
                .heroes
                .iter()
                .any(|h| h.present && x_gap(self.x, h.hurt) <= f32::from(t.hazq06_sight_px))
    }

    fn hero_by_id(heroes: &[HeroView], id: HeroId) -> Option<&HeroView> {
        heroes.iter().find(|h| h.id == id)
    }

    /// HAZ-Q08/Q14: whether the slime wants an attack token this tick: awake,
    /// not in an attack (never during the cooldown), the target eligible and
    /// within the trigger distance. The scene grants it (the goblins'
    /// `MAX_ATTACKERS` and tell spacing) and passes the answer to [`Self::step`].
    pub fn wants_token(&self, t: &SlimeTuning, ctx: &SlimeCtx<'_>) -> bool {
        if self.defeated || self.attack.is_some() || !self.awake(t, ctx) {
            return false;
        }
        let Some(hero) = ctx.target.and_then(|id| Self::hero_by_id(ctx.heroes, id)) else {
            return false;
        };
        self.eligible(t, hero) && (hero.x - self.x).abs() <= f32::from(t.hazq08_trigger_px)
    }

    /// One fixed tick of the slot step: attack timer, the turn at the cooldown
    /// start, the push-kick slide and the creep, then the impact tick (lunge
    /// and corrosive body). `token_granted` answers [`Self::wants_token`].
    /// `evade` is the reflex callback of HAZ-B (DM-B's candidate-loop pattern):
    /// it is called for a hero the lunge overlaps and returns `true` when that
    /// hero's reflex evades this lunge instance; HAZ-A1's tests pass `|_| false`.
    pub fn step(
        &mut self,
        t: &SlimeTuning,
        ctx: &SlimeCtx<'_>,
        token_granted: bool,
        mut evade: impl FnMut(HeroId) -> bool,
    ) -> SlimeStep {
        let mut out = SlimeStep::default();
        if self.defeated {
            return out;
        }
        self.flash_left = self.flash_left.saturating_sub(1);
        self.hurt_frame_left = self.hurt_frame_left.saturating_sub(1);

        // The attack clock: advance an attack in progress before a new one may
        // begin, so a tell that starts on this tick is on attack tick 0 now.
        if let Some(a) = self.attack {
            let next = a + 1;
            self.attack = (next < t.total_ticks()).then_some(next);
            if self.attack.is_none() {
                self.locked = None;
            }
        }
        if self.attack.is_none() && token_granted && self.wants_token(t, ctx) {
            self.begin(ctx);
            out.tell_started = true;
        }

        // The slime turns to its target on the first cooldown tick (the back is
        // open only during the recover); idle faces its target every tick.
        let turning = match self.attack {
            None => true,
            Some(a) => a >= t.cooldown_start(),
        };
        let target = ctx
            .target
            .and_then(|id| Self::hero_by_id(ctx.heroes, id))
            .filter(|h| self.eligible(t, h));
        if turning {
            let face_to = target.map(|h| h.x).or_else(|| {
                // No eligible hero: hold facing the nearest present hero.
                ctx.heroes
                    .iter()
                    .filter(|h| h.present)
                    .min_by(|a, b| {
                        (a.x - self.x)
                            .abs()
                            .total_cmp(&(b.x - self.x).abs())
                            .then(a.id.index().cmp(&b.id.index()))
                    })
                    .map(|h| h.x)
            });
            if let Some(tx) = face_to
                && (tx - self.x).abs() > f32::EPSILON
            {
                self.facing = if tx > self.x { 1 } else { -1 };
            }
        }

        self.step_slide(t, ctx.solids);
        if turning
            && self.slide.is_none()
            && self.awake(t, ctx)
            && let Some(h) = target
        {
            self.creep(t, h.x, ctx.solids);
        }

        self.impact_tick(t, ctx, &mut evade, &mut out);
        out
    }

    fn begin(&mut self, ctx: &SlimeCtx<'_>) {
        self.attack = Some(0);
        self.locked = ctx.target;
        self.lunge_done = false;
        self.evaded = [false; 2];
        if let Some(h) = ctx.target.and_then(|id| Self::hero_by_id(ctx.heroes, id))
            && (h.x - self.x).abs() > f32::EPSILON
        {
            self.facing = if h.x > self.x { 1 } else { -1 };
        }
    }

    fn body_for_motion(&self, t: &SlimeTuning, x: f32) -> Rect {
        world_box(&t.hazq05_contact_standing, x, self.facing, self.foot_y)
    }

    /// Creep toward the standoff point on its current side of the target,
    /// never backward, never farther than the leash from home.
    fn creep(&mut self, t: &SlimeTuning, target_x: f32, solids: &[Block]) {
        let side = if self.x >= target_x { 1.0 } else { -1.0 };
        let goal = target_x + side * f32::from(t.hazq08_standoff_px);
        let step = t.creep_px_per_tick();
        let next = if side > 0.0 {
            if self.x <= goal {
                return;
            }
            (self.x - step).max(goal)
        } else {
            if self.x >= goal {
                return;
            }
            (self.x + step).min(goal)
        };
        if (next - self.home_x).abs() > f32::from(t.hazq06_leash_px) {
            return;
        }
        if solid_at(solids, self.body_for_motion(t, next)) {
            return;
        }
        self.x = next;
    }

    /// The push-kick slide: `px / ticks` per tick away from the kicker, in any
    /// state, stopped by a solid and by the leash.
    fn step_slide(&mut self, t: &SlimeTuning, solids: &[Block]) {
        let Some(mut slide) = self.slide else { return };
        let next = self.x + slide.dir * t.kick_px_per_tick();
        let leashed = (next - self.home_x).abs() > f32::from(t.hazq06_leash_px);
        if leashed || solid_at(solids, self.body_for_motion(t, next)) {
            self.slide = None;
            return;
        }
        self.x = next;
        slide.left -= 1;
        self.slide = (slide.left > 0).then_some(slide);
    }

    /// A push kick that landed: slide away from the kicker (HAZ-Q12).
    pub fn push_kick(&mut self, t: &SlimeTuning, kicker_x: f32) {
        if self.defeated {
            return;
        }
        self.slide = Some(Slide {
            left: t.hazq12_kick_slide_ticks,
            dir: if self.x >= kicker_x { 1.0 } else { -1.0 },
        });
    }

    /// A hit of any kind (HAZ-Q11, gel): the damage lands, the attack and its
    /// timer never change. A lethal hit defeats it in place.
    pub fn hit(&mut self, t: &SlimeTuning, damage: u8) -> HitOutcome {
        if self.defeated {
            return HitOutcome {
                defeated: false,
                flash: false,
                hurt_frame: false,
                interrupted: false,
            };
        }
        let idle = self.attack.is_none();
        self.health = self.health.saturating_sub(damage);
        self.flash_left = 2;
        if self.health == 0 {
            self.defeated = true;
            self.attack = None;
            self.locked = None;
            self.slide = None;
            self.hurt_frame_left = 0;
            return HitOutcome {
                defeated: true,
                flash: true,
                hurt_frame: false,
                interrupted: false,
            };
        }
        let gel = t.on(t.hazq11_gel);
        let interrupted = !gel && self.attack.is_some();
        if interrupted {
            self.attack = None;
            self.locked = None;
        }
        let hurt_frame = idle || !gel;
        if hurt_frame {
            self.hurt_frame_left = 12;
        }
        HitOutcome {
            defeated: false,
            flash: true,
            hurt_frame,
            interrupted,
        }
    }

    fn impact_tick(
        &mut self,
        t: &SlimeTuning,
        ctx: &SlimeCtx<'_>,
        evade: &mut impl FnMut(HeroId) -> bool,
        out: &mut SlimeStep,
    ) {
        let phase = self.phase(t);
        // 1. The lunge (heroes stepped earlier this tick).
        if phase == SlimePhase::Impact
            && !self.lunge_done
            && let Some(lunge) = self.lunge_box(t)
        {
            let mut candidates: Vec<&HeroView> = ctx
                .heroes
                .iter()
                .filter(|h| {
                    h.present
                        && h.vulnerable
                        && !self.evaded[h.id.index()]
                        && lunge.overlaps(h.hurt)
                })
                .collect();
            // The one whose near edge is closer to the slime's front first;
            // ties go to Carl.
            candidates.sort_by(|a, b| {
                self.near_edge(a.hurt)
                    .total_cmp(&self.near_edge(b.hurt))
                    .then(a.id.index().cmp(&b.id.index()))
            });
            for h in candidates {
                if t.on(t.hazq15_lunge_evadable) && evade(h.id) {
                    self.evaded[h.id.index()] = true;
                    out.evaded.push(h.id);
                    continue;
                }
                self.lunge_done = true;
                out.hits.push(SlimeHit {
                    hero: h.id,
                    kind: SlimeHurt::Lunge,
                    blow: self.lunge_blow(t, h),
                });
                break;
            }
        }
        // 2. Body contact, every living tick.
        let Some(body) = self.contact_box(t) else {
            return;
        };
        for h in ctx.heroes.iter().filter(|h| h.present) {
            // A hero who evaded this lunge is exempt on its impact ticks.
            if phase == SlimePhase::Impact && self.evaded[h.id.index()] {
                continue;
            }
            let inside = body.overlaps(h.hurt);
            if self
                .contact
                .offer(Victim::hero(h.id.index() as u32), inside, h.vulnerable)
            {
                out.hits.push(SlimeHit {
                    hero: h.id,
                    kind: SlimeHurt::Body,
                    blow: self.body_blow(t, h),
                });
            }
        }
    }

    /// Distance from the pivot to the near edge of a hurtbox, along the facing.
    fn near_edge(&self, hurt: Rect) -> f32 {
        if self.facing >= 0 {
            hurt.left - self.x
        } else {
            self.x - hurt.right()
        }
    }

    /// The lunge's blow: signed away from the slime by the caller's rule.
    pub fn lunge_blow(&self, t: &SlimeTuning, hero: &HeroView) -> HeroBlow {
        HeroBlow {
            hero: hero.id,
            source: slime_source(self.index),
            damage: t.hazq10_damage,
            from_x: self.x,
            knock_vx: signed_away(hero.x, self.x, f32::from(t.hazq10_knock_x)),
            knock_vy: f32::from(t.hazq10_knock_y),
        }
    }

    /// The corrosive body's blow: the pop (HAZ-Q19), `+` at `hero.x >= x`.
    pub fn body_blow(&self, t: &SlimeTuning, hero: &HeroView) -> HeroBlow {
        HeroBlow {
            hero: hero.id,
            source: slime_source(self.index),
            damage: t.hazq19_damage,
            from_x: self.x,
            knock_vx: signed_away(hero.x, self.x, f32::from(t.hazq19_pop_x)),
            knock_vy: f32::from(t.hazq19_pop_y),
        }
    }

    /// Back to the authored start of an attempt (a full restart).
    pub fn reset(&mut self, t: &SlimeTuning, x: f32, facing: i8) {
        *self = Slime::new(t, x, self.foot_y, facing, self.home_x).with_index(self.index);
    }

    /// Fold the full state into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(u64::from(self.x.to_bits()));
        mix(u64::from(self.foot_y.to_bits()));
        mix(self.facing as u8 as u64);
        mix(u64::from(self.home_x.to_bits()));
        mix(u64::from(self.index));
        mix(u64::from(self.health));
        mix(u64::from(self.defeated));
        mix(self.attack.map_or(0, |a| u64::from(a) + 1));
        mix(self.locked.map_or(0, |h| h.index() as u64 + 1));
        mix(u64::from(self.lunge_done));
        mix(u64::from(self.evaded[0]) | (u64::from(self.evaded[1]) << 1));
        match self.slide {
            None => mix(0),
            Some(s) => {
                mix(1 + u64::from(s.left));
                mix(u64::from(s.dir.to_bits()));
            }
        }
        self.contact.mix_into(mix);
        mix(u64::from(self.flash_left));
        mix(u64::from(self.hurt_frame_left));
    }
}

/// Horizontal distance from a column to a hurtbox (0 when inside).
fn x_gap(x: f32, hurt: Rect) -> f32 {
    (hurt.left - x).max(0.0).max(x - hurt.right())
}

/// `magnitude` signed away from `from`: `+` when `at >= from` (a hero exactly
/// at the centre pops toward +x, HAZ-Q19).
pub fn signed_away(at: f32, from: f32, magnitude: f32) -> f32 {
    if at >= from { magnitude } else { -magnitude }
}

// -------------------------------------------------------------- acid patch

/// The authored acid patch (the sidecar's `acid_patches` entry), layout px.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcidPatchSpec {
    pub id: u16,
    pub x0: i32,
    pub x1: i32,
    /// The walking surface the patch lies on (layout y, down).
    pub floor_y: i32,
    /// The slime whose death drains it (a roster index), if any.
    pub linked_slime: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcidError {
    Width {
        id: u16,
        width: i32,
        min: u16,
        max: u16,
    },
    Spec(String),
}

impl core::fmt::Display for AcidError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Width {
                id,
                width,
                min,
                max,
            } => write!(f, "acid patch {id}: width {width} is not in {min}..={max}"),
            Self::Spec(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for AcidError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pool {
    Full,
    /// Drain tick `k` is the next one to apply (0 on the defeat tick).
    Draining(u16),
    Removed,
}

/// The state of a patch's pool, for the stain the presenter draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolState {
    Full,
    Draining,
    Removed,
}

/// The bounds of a draining patch on drain tick `k` (HAZ-Q21): both edges move
/// inward by `floor(w / 2 * k / drain)` whole pixels; from tick `drain` on the
/// pool is gone (`None`). A drained patch is never a zero-width volume.
pub fn drain_bounds(x0: i32, x1: i32, k: u16, drain: u16) -> Option<(i32, i32)> {
    if k >= drain {
        return None;
    }
    let w = i64::from(x1 - x0);
    let inset = (w * i64::from(k) / (2 * i64::from(drain))) as i32;
    let (a, b) = (x0 + inset, x1 - inset);
    (b > a).then_some((a, b))
}

/// One acid patch: an always-on persistent volume of class `Acid`.
#[derive(Debug, Clone, PartialEq)]
pub struct AcidPatch {
    spec: AcidPatchSpec,
    height: i32,
    drain_ticks: u16,
    launches_hurt: bool,
    volume: Option<HazardVolume>,
    pool: Pool,
    tokens: EntryTokens,
    launches: LaunchTokens,
    pop_x: f32,
    pop_y: f32,
    damage: u8,
}

impl AcidPatch {
    pub fn new(t: &SlimeTuning, spec: AcidPatchSpec) -> Result<Self, AcidError> {
        let width = spec.x1 - spec.x0;
        if width < i32::from(t.hazq18_width_min_px) || width > i32::from(t.hazq18_width_max_px) {
            return Err(AcidError::Width {
                id: spec.id,
                width,
                min: t.hazq18_width_min_px,
                max: t.hazq18_width_max_px,
            });
        }
        let height = i32::from(t.hazq18_volume_height_px);
        let volume = HazardVolume::new(Self::volume_spec(&spec, t, height))
            .map_err(|e| AcidError::Spec(format!("acid patch {}: {e}", spec.id)))?;
        Ok(Self {
            height,
            drain_ticks: t.hazq21_drain_ticks,
            launches_hurt: t.hazq20_acid_hurts_launched,
            volume: Some(volume),
            pool: Pool::Full,
            tokens: EntryTokens::new(),
            launches: LaunchTokens::new(),
            pop_x: f32::from(t.hazq19_pop_x),
            pop_y: f32::from(t.hazq19_pop_y),
            damage: t.hazq19_damage,
            spec,
        })
    }

    fn volume_spec(spec: &AcidPatchSpec, t: &SlimeTuning, height: i32) -> HazardSpec {
        let rect = IRect::new(spec.x0, spec.floor_y - height, spec.x1 - spec.x0, height);
        HazardSpec {
            id: spec.id,
            class: HazardClass::Acid,
            rect,
            telegraph_rect: rect,
            window: HazardWindow::Persistent,
            damage: t.hazq19_damage,
            // The caller signs the pop: the volume's own push is unused.
            push_x: 0,
            // HAZ-Q20: heroes and landing fliers are passed by HAZ's acid
            // step; the volume's data is AllHeroes + hurts_enemies.
            hero_damage: HeroDamagePolicy::AllHeroes,
            hurts_enemies: true,
        }
    }

    pub fn spec(&self) -> &AcidPatchSpec {
        &self.spec
    }

    pub fn id(&self) -> u16 {
        self.spec.id
    }

    /// The patch centre (the `from_x` of the pop); it never moves (the drain
    /// is symmetric).
    pub fn centre_x(&self) -> f32 {
        (self.spec.x0 + self.spec.x1) as f32 / 2.0
    }

    /// The contact rectangle now, `None` once the pool is removed.
    pub fn rect(&self) -> Option<IRect> {
        self.volume.as_ref().map(HazardVolume::rect)
    }

    /// The current x bounds, `None` once removed.
    pub fn bounds(&self) -> Option<(i32, i32)> {
        self.rect().map(|r| (r.x, r.right()))
    }

    pub fn pool_state(&self) -> PoolState {
        match self.pool {
            Pool::Full => PoolState::Full,
            Pool::Draining(_) => PoolState::Draining,
            Pool::Removed => PoolState::Removed,
        }
    }

    /// The nav-feed row of this tick: active, class `Acid`, persistent.
    pub fn nav_rect(&self) -> Option<NavRect> {
        self.volume.as_ref().and_then(HazardVolume::nav_rect)
    }

    /// The linked slime was defeated on this tick: the drain starts (drain tick
    /// 0 is this tick). Only a linked, full pool drains.
    pub fn on_slime_defeated(&mut self) {
        if self.spec.linked_slime.is_some() && self.pool == Pool::Full {
            self.pool = Pool::Draining(0);
        }
    }

    /// HAZ-FU (HAZA2-AD6): the linked slime was never alive in this attempt (the
    /// kill switch `hazq02_slime_enabled` built it defeated): the pool is removed
    /// at once, with no drain and no 60-tick window of a volume that belongs to a
    /// slime the room does not have. Only a linked, full pool is removed.
    pub fn remove_for_disabled_slime(&mut self) {
        if self.spec.linked_slime.is_some() && self.pool == Pool::Full {
            self.pool = Pool::Removed;
            self.volume = None;
        }
    }

    /// The linked slime respawned: the pool is restored at full width and the
    /// drain is cancelled.
    pub fn on_slime_respawn(&mut self, t: &SlimeTuning) {
        if self.spec.linked_slime.is_none() {
            return;
        }
        self.restore(t);
    }

    /// A full restart: the authored start.
    pub fn reset(&mut self, t: &SlimeTuning) {
        self.restore(t);
        self.tokens.clear();
        self.launches.clear();
    }

    fn restore(&mut self, t: &SlimeTuning) {
        self.pool = Pool::Full;
        self.volume = HazardVolume::new(Self::volume_spec(&self.spec, t, self.height)).ok();
    }

    /// The env step's drain: apply drain tick `k` and advance. The volume and
    /// the stain share the bounds this returns. On tick `drain` the volume is
    /// removed and the stain with it.
    pub fn step(&mut self) {
        let Pool::Draining(k) = self.pool else { return };
        match drain_bounds(self.spec.x0, self.spec.x1, k, self.drain_ticks) {
            Some((a, b)) => {
                if let Some(v) = self.volume.as_mut() {
                    let rect = IRect::new(a, self.spec.floor_y - self.height, b - a, self.height);
                    v.set_rect(rect);
                }
                self.pool = Pool::Draining(k + 1);
            }
            None => {
                self.volume = None;
                self.pool = Pool::Removed;
            }
        }
    }

    /// A hero's body against the patch (HAZ-Q19): `true` on the first tick of
    /// an entry on which the hero can be hurt. Slimes and walking goblins are
    /// never passed.
    pub fn contact(&mut self, hero: HeroId, body: IRect, vulnerable: bool) -> bool {
        let inside = self.rect().is_some_and(|r| r.overlaps(body));
        self.tokens
            .offer(Victim::hero(hero.index() as u32), inside, vulnerable)
    }

    /// The pop of a hurt on `hero` at `hero_x`: 1 damage, `+` at or right of
    /// the centre, signed by this caller (`hurt_hero` copies it as given).
    pub fn pop_blow(&self, hero: HeroId, hero_x: f32) -> HeroBlow {
        HeroBlow {
            hero,
            source: acid_source(self.spec.id),
            damage: self.damage,
            from_x: self.centre_x(),
            knock_vx: signed_away(hero_x, self.centre_x(), self.pop_x),
            knock_vy: self.pop_y,
        }
    }

    /// A launched body landed (HAZ-Q20): hurt once per launch instance while it
    /// overlaps the patch; lying in acid does nothing further.
    pub fn launched_landing(&mut self, launch_id: u32, body: IRect) -> bool {
        if !self.launches_hurt || !self.rect().is_some_and(|r| r.overlaps(body)) {
            return false;
        }
        self.launches.offer(launch_id)
    }

    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(u64::from(self.spec.id));
        match self.pool {
            Pool::Full => mix(0),
            Pool::Draining(k) => mix(1 + u64::from(k)),
            Pool::Removed => mix(u64::MAX),
        }
        if let Some(v) = &self.volume {
            v.mix_into(mix);
        }
        self.tokens.mix_into(mix);
        self.launches.mix_into(mix);
    }
}

/// All acid patches of a room, by id; the slime-death hooks of HAZ-A2 call
/// [`AcidSet::slime_defeated`] and [`AcidSet::slime_respawned`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AcidSet {
    patches: BTreeMap<u16, AcidPatch>,
}

impl AcidSet {
    pub fn new(t: &SlimeTuning, specs: Vec<AcidPatchSpec>) -> Result<Self, AcidError> {
        let mut patches = BTreeMap::new();
        for spec in specs {
            let id = spec.id;
            let patch = AcidPatch::new(t, spec)?;
            if patches.insert(id, patch).is_some() {
                return Err(AcidError::Spec(format!("duplicate acid patch id {id}")));
            }
        }
        Ok(Self { patches })
    }

    pub fn patches(&self) -> impl Iterator<Item = &AcidPatch> {
        self.patches.values()
    }

    pub fn get(&self, id: u16) -> Option<&AcidPatch> {
        self.patches.get(&id)
    }

    pub fn get_mut(&mut self, id: u16) -> Option<&mut AcidPatch> {
        self.patches.get_mut(&id)
    }

    /// The linked slime (roster index) was defeated: its pools start draining.
    pub fn slime_defeated(&mut self, roster_index: u8) {
        for p in self.patches.values_mut() {
            if p.spec.linked_slime == Some(roster_index) {
                p.on_slime_defeated();
            }
        }
    }

    /// The linked slime respawned: its pools are restored.
    pub fn slime_respawned(&mut self, t: &SlimeTuning, roster_index: u8) {
        for p in self.patches.values_mut() {
            if p.spec.linked_slime == Some(roster_index) {
                p.on_slime_respawn(t);
            }
        }
    }

    /// One drain tick of every patch, in id order.
    pub fn step(&mut self) {
        for p in self.patches.values_mut() {
            p.step();
        }
    }

    pub fn reset(&mut self, t: &SlimeTuning) {
        for p in self.patches.values_mut() {
            p.reset(t);
        }
    }

    /// The hazard feed rows of this tick, in id order.
    pub fn nav_feed(&self) -> Vec<NavRect> {
        self.patches
            .values()
            .filter_map(AcidPatch::nav_rect)
            .collect()
    }

    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.patches.len() as u64);
        for p in self.patches.values() {
            p.mix_into(mix);
        }
    }
}

#[cfg(test)]
mod tests;
