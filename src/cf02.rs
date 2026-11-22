//! COMBAT-FUN-02 knob schema (CF02-00).
//!
//! Bevy-free data contract for `docs/01-design/combat-fun-02-post-demo01-proposals.md`
//! section 7. [`Cf02Tuning`] mirrors `assets/data/gameplay/cf02_combat_v0.json`
//! field for field: every adopted value of CF02-Q01..CF02-Q75 that is neither a
//! party knob (Q19, Q20: `PartyTuning`, CF02-C2) nor a companion value (Q13..Q17,
//! Q39, Q40: `companion_ai_v0.json`, COMPANION-AI-01). Knob names equal the
//! question IDs (`q02_stomp_startup`). **CF02-00 only declares the schema**: the
//! master switch `cf02_rules` is `false`, nothing reads a value yet, and later
//! PRs read (and, holding the data token, edit) values without changing the
//! shape. The file is loaded, validated and FNV-1a pinned like
//! `companion_magic_v0.json` ([`crate::donut_mana::content_hash`]).
//!
//! **Every boolean switch is read through [`Cf02Tuning::on`]** (never raw), so a
//! rule cannot leak into `--demo`. A load error names the field. Questions without a knob are listed in
//! [`NO_KNOB`] (with the reason) so a test can prove the schema is complete.

use crate::donut_mana::content_hash;
use serde::{Deserialize, Serialize};

/// The tuning file, embedded so the binary and the tests read the same bytes.
pub const TUNING_JSON: &str = include_str!("../assets/data/gameplay/cf02_combat_v0.json");

/// The committed FNV-1a 64 content hash of [`TUNING_JSON`]. Editing the file
/// without updating this value fails [`Cf02Tuning::pinned`] and the pin test, so
/// a tuning edit never changes a recorded outcome silently.
pub const TUNING_HASH: u64 = 0x2da4_8577_fa64_315e;

/// Question IDs that have no key in the file, and why. The completeness test
/// requires every other ID in 1..=75 to own at least one `qNN_` key.
pub const NO_KNOB: [(u8, &str); 22] = [
    (7, "retired: superseded by Q50..Q53"),
    (
        13,
        "COMPANION-AI-01: UncontrolledHero::Companion(Cf02Assist)",
    ),
    (
        14,
        "COMPANION-AI-01: profile cf02_assist in companion_ai_v0.json",
    ),
    (15, "COMPANION-AI-01: profile cf02_assist"),
    (16, "COMPANION-AI-01: profile cf02_assist"),
    (17, "COMPANION-AI-01: profile cf02_assist"),
    (19, "party knob: PartyTuning (CF02-C2)"),
    (20, "party knob: PartyTuning (CF02-C2)"),
    (23, "process: the CF02-00 defeat audit, docs/evidence/cf02"),
    (25, "CINE-01 unchanged"),
    (26, "decision: options A + B, no evade command"),
    (33, "knob: cf02_rules (the --cf02 flag sets it)"),
    (35, "code: input priority"),
    (37, "the switch is q18_llama_targets"),
    (39, "COMPANION-AI-01: profile cf02_assist"),
    (
        40,
        "COMPANION-AI-01: the shared CompanionView perception rule",
    ),
    (41, "code: the ShoreTripped event and the CINE rule"),
    (44, "fixture cf02_donut_solo_llama only"),
    (67, "no llama morale"),
    (68, "code: reactions and tokens"),
    (74, "process: tone limits of the acting spec"),
    (75, "process: the art route"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cf02TuningError {
    /// The JSON did not parse or has an unknown or missing field.
    Parse(String),
    /// A field failed validation; `field` names it.
    Field { field: &'static str, reason: String },
    /// The file's content hash is not the pinned one.
    HashMismatch { expected: u64, actual: u64 },
}

impl core::fmt::Display for Cf02TuningError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "cf02 tuning: {msg}"),
            Self::Field { field, reason } => {
                write!(f, "cf02 tuning: field `{field}`: {reason}")
            }
            Self::HashMismatch { expected, actual } => write!(
                f,
                "cf02 tuning hash {actual:#018x} is not the pinned {expected:#018x}"
            ),
        }
    }
}

impl std::error::Error for Cf02TuningError {}

/// A pivot-relative half-open box: x forward, y up from the feet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoxPx {
    pub x0: i16,
    pub x1: i16,
    pub y0: i16,
    pub y1: i16,
}

/// Q01: how the stomp is bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StompBinding {
    DownPhysical,
    Off,
}

/// Q09: what a body contact does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactMode {
    Knockdown,
    Legacy,
}

/// A force tier by name (`ForceTier` in `combat.rs` keeps the simulation's own).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Force {
    Routine,
    Heavy,
    Catastrophic,
}

/// Q10: a move whose lethal body bowls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BowlMove {
    PushKick,
    FollowKick,
    AirKick,
}

/// Q31: a shore state (`intact -> damaged_1 -> damaged_2 -> broken`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShoreState {
    #[serde(rename = "damaged_1")]
    Damaged1,
    #[serde(rename = "damaged_2")]
    Damaged2,
}

/// Q58, Q59: a goblin temperament.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Temperament {
    Brawler,
    Coward,
    ShowOff,
}

/// Q60: a defeat cause that is spectacular at any force.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpectacularCause {
    Shore,
    Explosion,
    Decap,
}

/// `assets/data/gameplay/cf02_combat_v0.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cf02Tuning {
    /// Schema version; must be 1.
    pub schema_version: u32,
    /// Master switch: every CF02 rule reads it. `false` everywhere except `--cf02`.
    pub cf02_rules: bool,
    /// Q01: `down_physical` (Down + Physical) or `off`.
    pub q01_stomp_binding: StompBinding,
    /// Q01: left stick y <= -threshold/1000 with abs(y) >= abs(x) counts as Down.
    pub q01_down_stick_threshold_milli: u16,
    /// Q02: stomp startup ticks.
    pub q02_stomp_startup: u8,
    /// Q02: stomp active ticks.
    pub q02_stomp_active: u8,
    /// Q02: stomp recovery ticks.
    pub q02_stomp_recovery: u8,
    /// Q02: root step in px over move ticks 1..=q02_stomp_step_ticks.
    pub q02_stomp_step_px: u8,
    /// Q02: last move tick of the root step.
    pub q02_stomp_step_ticks: u8,
    /// Q02: stomp hit box, pivot relative, half open.
    pub q02_stomp_box: BoxPx,
    /// Q02: stomp damage.
    pub q02_stomp_damage: u8,
    /// Q02: hit-stop ticks.
    pub q02_stomp_hit_stop: u8,
    /// Q02: extra hit-stop on a lethal stomp.
    pub q02_stomp_hit_stop_lethal_bonus: u8,
    /// Q02: camera kick px.
    pub q02_stomp_camera_kick_px: u8,
    /// Q02: stagger of a standing target.
    pub q02_stomp_standing_stagger: u8,
    /// Q02: knock of a standing target (px/s).
    pub q02_stomp_standing_knock_x: i16,
    /// Q02: lethal knock x (the body stays in place).
    pub q02_stomp_lethal_knock_x: i16,
    /// Q02: lethal knock y.
    pub q02_stomp_lethal_knock_y: i16,
    /// Q02: force tier of a lethal stomp.
    pub q02_stomp_lethal_force: Force,
    /// Q03: hurtbox of a goblin lying in `Downed`.
    pub q03_lying_box: BoxPx,
    /// Q04: how long a kicked goblin lies (legacy 18).
    pub q04_downed_ticks: u16,
    /// Q04: get-up ticks (unchanged).
    pub q04_getup_ticks: u16,
    /// Q05: first jab tick that accepts the follow-through press.
    pub q05_follow_window_start: u8,
    /// Q05: last jab tick that accepts the follow-through press.
    pub q05_follow_window_end: u8,
    /// Q05: follow-through kick startup (push kick legacy 13).
    pub q05_follow_startup: u8,
    /// Q05: follow-through active ticks.
    pub q05_follow_active: u8,
    /// Q05: follow-through recovery ticks.
    pub q05_follow_recovery: u8,
    /// Q05: the follow-through starts on this jab tick or the press tick, whichever is later.
    pub q05_follow_start_jab_tick: u8,
    /// Q06: lethal kick launch x (px/s).
    pub q06_lethal_kick_knock_x: i16,
    /// Q06: lethal kick launch y (px/s).
    pub q06_lethal_kick_knock_y: i16,
    /// Q06: jab and haymaker kill launch x (unchanged).
    pub q06_lethal_melee_knock_x: i16,
    /// Q06: jab and haymaker kill launch y (unchanged).
    pub q06_lethal_melee_knock_y: i16,
    /// Q08: stagger of an ordinary hero hit on a llama.
    pub q08_llama_ordinary_stagger: u8,
    /// Q34: a non-lethal non-kick hit keeps a lying goblin down.
    pub q34_keep_down: bool,
    /// Q09: `knockdown` or `legacy`.
    pub q09_contact_mode: ContactMode,
    /// Q09/Q11: minimum abs(vx) of a flier for a knockdown or a wall slam (px/s).
    pub q09_contact_min_vx: u16,
    /// Q09: target push x (px/s).
    pub q09_contact_push_x: i16,
    /// Q09: target hop y (px/s).
    pub q09_contact_hop_y: i16,
    /// Q09: legacy stagger below the threshold.
    pub q09_legacy_stagger: u8,
    /// Q09: legacy push below the threshold (px/s).
    pub q09_legacy_push: i16,
    /// Q09: share of its vx the flier keeps (percent).
    pub q09_flier_keep_vx_pct: u8,
    /// Q09: flier hit-stop ticks.
    pub q09_flier_hit_stop: u8,
    /// Q10: killed bodies bowl.
    pub q10_bowling: bool,
    /// Q10: lethal moves whose body bowls.
    pub q10_bowl_moves: Vec<BowlMove>,
    /// Q11: wall slam enabled.
    pub q11_wall_slam: bool,
    /// Q11: wall slam damage (0 = visual only).
    pub q11_wall_slam_damage: u8,
    /// Q12: body contact damage.
    pub q12_contact_damage: u8,
    /// Q18/Q37: llamas are hero-hit targets. Effective only when `cf02_rules` is on (see `Cf02Tuning::llama_targets`).
    pub q18_llama_targets: bool,
    /// Q18: swat damage on a llama.
    pub q18_swat_damage: u8,
    /// Q18: llama stagger of an interrupting swat.
    pub q18_swat_interrupt_stagger: u8,
    /// Q18: `LLAMA_HURT_COOLDOWN` (unchanged).
    pub q18_llama_hurt_cooldown: u8,
    /// Q18: light guard minimum ticks (also until the next spit release).
    pub q18_guard_min_ticks: u16,
    /// Q18: pounce damage on a llama.
    pub q18_pounce_damage: u8,
    /// Q18: Magic Missile damage on a llama.
    pub q18_missile_damage: u8,
    /// Q38: down timer pause of a non-lethal pounce on a lying goblin (0 = no hold).
    pub q38_hold_ticks: u8,
    /// Q38: once per down instance.
    pub q38_hold_once_per_down: bool,
    /// Q55: Donut's cat hop vx (px/s).
    pub q55_hop_vx: i16,
    /// Q55: Donut's cat hop vy (px/s).
    pub q55_hop_vy: i16,
    /// Q55: landing recovery ticks.
    pub q55_hop_landing_recovery: u8,
    /// Q55: C6 enabled.
    pub q55_bailout_enabled: bool,
    /// Q21: each slammed pose holds its impact frame at least this long.
    pub q21_slammed_impact_hold_ticks: u8,
    /// Q22: gib quarters of heavy and catastrophic finishes at preset 4.
    pub q22_preset4_gib_quarters: u8,
    /// Q22: extra separated parts at preset 4.
    pub q22_preset4_extra_separated_parts: u8,
    /// Q24: the cf02 cue overlay (read only under `cf02_rules`).
    pub q24_cue_overlay: bool,
    /// Q24: pack rule, major transients that may overlap.
    pub q24_max_overlapping_transients: u8,
    /// Q36: the cf02 death profile overlay (read only under `cf02_rules`).
    pub q36_death_overlay: bool,
    /// Q36: the cf02 gore budget (read only under `cf02_rules`).
    pub q36_gore_budget: bool,
    /// Q36: the cf02 cue overlay (read only under `cf02_rules`).
    pub q36_cue_overlay: bool,
    /// Q43: stomped, gib quarters at preset 4.
    pub q43_stomped_gib_quarters: u8,
    /// Q43: stomped, separated parts.
    pub q43_stomped_separated_parts: u8,
    /// Q43: wall crumple, gib quarters.
    pub q43_wall_crumple_gib_quarters: u8,
    /// Q43: wall crumple, separated parts (an arm).
    pub q43_wall_crumple_separated_parts: u8,
    /// Q43: flung, gib quarters.
    pub q43_flung_gib_quarters: u8,
    /// Q43: flung, separated parts.
    pub q43_flung_separated_parts: u8,
    /// Q43: transient sprays added to a routine blunt kill at preset 4.
    pub q43_routine_spray_count: u8,
    /// Q43: lifetime of the transient spray.
    pub q43_routine_spray_ticks: u8,
    /// Q27: llama tell ticks (unchanged; `feel::LLAMA_TELL_TICKS` stays the source until a PR moves it).
    pub q27_llama_tell_ticks: u16,
    /// Q27: llama attack ticks, tell start to recovered (unchanged).
    pub q27_llama_attack_ticks: u16,
    /// Q27: llama cooldown ticks (unchanged).
    pub q27_llama_cooldown_ticks: u16,
    /// Q27: glob speed (px/s, unchanged).
    pub q27_llama_glob_speed_px_s: u16,
    /// Q27: glob height above the llama's feet, lower bound (px).
    pub q27_llama_glob_y_min_px: u8,
    /// Q27: glob height above the llama's feet, upper bound (px).
    pub q27_llama_glob_y_max_px: u8,
    /// Q27: llama fair-fire range (px, unchanged).
    pub q27_llama_fire_range_px: u16,
    /// Q28: cover at least this tall stops a glob.
    pub q28_cover_min_height_px: u8,
    /// Q29: microarena width.
    pub q29_room_width_px: u16,
    /// Q29: microarena height.
    pub q29_room_height_px: u16,
    /// Q29: floor y.
    pub q29_floor_y: u16,
    /// Q29: width of each end wall.
    pub q29_end_wall_px: u8,
    /// Q29: Carl's spawn column and checkpoint 1.
    pub q29_carl_spawn_x: u16,
    /// Q29: Donut's spawn column.
    pub q29_donut_spawn_x: u16,
    /// Q29: checkpoint columns.
    pub q29_checkpoints_x: Vec<u16>,
    /// Q29: shore strike column.
    pub q29_shore_strike_x: u16,
    /// Q29: left perch slab, x range.
    pub q29_left_perch_x: Vec<u16>,
    /// Q29: right perch slab, x range.
    pub q29_right_perch_x: Vec<u16>,
    /// Q29: parapet, x range.
    pub q29_parapet_x: Vec<u16>,
    /// Q29: parapet height.
    pub q29_parapet_height_px: u8,
    /// Q29: first-wave goblin columns.
    pub q29_first_wave_goblins_x: Vec<u16>,
    /// Q29: second-wave goblin columns.
    pub q29_second_wave_goblins_x: Vec<u16>,
    /// Q29: llama home column.
    pub q29_llama_home_x: u16,
    /// Q29: llama leash.
    pub q29_llama_leash_px: u16,
    /// Q30: the falling block hurts heroes in its footprint.
    pub q30_block_hurts_heroes: bool,
    /// Q31: a goblin swing can trip a damaged shore.
    pub q31_goblin_self_own: bool,
    /// Q31: a goblin swing counts as a shore hit from this state on.
    pub q31_min_shore_state: ShoreState,
    /// Q32: goblin sight (unchanged).
    pub q32_goblin_sight_px: u16,
    /// Q32: Select hold that restarts the room (unchanged).
    pub q32_restart_hold_ticks: u8,
    /// Q32: a restart draws a new temperament permutation.
    pub q32_new_draw_on_restart: bool,
    /// Q42: block damage to a hero (0 = off).
    pub q42_block_damage: u8,
    /// Q42: hurt flinch ticks (`HERO_HURT_TICKS`).
    pub q42_block_hurt_ticks: u8,
    /// Q42: knock x away from the block centre (px/s).
    pub q42_block_knock_x: i16,
    /// Q42: knock y (px/s).
    pub q42_block_knock_y: i16,
    /// Q42: invulnerability after the hit.
    pub q42_block_invulnerable_ticks: u8,
    /// Q45: perch slab top above the floor.
    pub q45_perch_top_px: u8,
    /// Q45: perch slab underside above the floor (ruling: 68, the approved deck
    /// tile's 12 px collider under a top at 80).
    pub q45_perch_underside_px: u8,
    /// Q45: perch slab thickness as drawn (one 16 px tile; its collider is 12 px,
    /// so the underside is `q45_perch_top_px - 12` = 68).
    pub q45_perch_slab_px: u8,
    /// Q46: buffer window for Carl's Primary and Physical presses (legacy 6).
    pub q46_move_buffer_ticks: u8,
    /// Q46: the runner's own buffer under `cf02_rules`.
    pub q46_runner_buffer_ticks: u8,
    /// Q47: Jump wins over an attack on the same tick; the attack fires in the air.
    pub q47_jump_wins_same_tick: bool,
    /// Q48: an air attack within this height of a solid top waits for the landing.
    pub q48_landing_grace_px: u8,
    /// Q49: C1 jab -> haymaker.
    pub q49_c1_enabled: bool,
    /// Q49: C1 first jab tick (legacy).
    pub q49_c1_window_start: u8,
    /// Q49: C1 last jab tick (legacy).
    pub q49_c1_window_end: u8,
    /// Q49: C2 jab hit -> follow-through (window in Q05).
    pub q49_c2_enabled: bool,
    /// Q49: C3 grounded recovery -> bail-out.
    pub q49_c3_enabled: bool,
    /// Q49: C4 air kick that hit -> dive.
    pub q49_c4_enabled: bool,
    /// Q49: C5 dive bounce -> air kick.
    pub q49_c5_enabled: bool,
    /// Q49: C6 Donut recovery -> cat hop (see Q55).
    pub q49_c6_enabled: bool,
    /// Q50: air kick startup.
    pub q50_air_kick_startup: u8,
    /// Q50: air kick active.
    pub q50_air_kick_active: u8,
    /// Q50: air kick recovery.
    pub q50_air_kick_recovery: u8,
    /// Q50: air kick hit box.
    pub q50_air_kick_box: BoxPx,
    /// Q50: air kick damage.
    pub q50_air_kick_damage: u8,
    /// Q50: air kick hit-stop.
    pub q50_air_kick_hit_stop: u8,
    /// Q50: standing goblin stagger.
    pub q50_air_kick_standing_stagger: u8,
    /// Q50: standing goblin knock x (px/s).
    pub q50_air_kick_knock_x: i16,
    /// Q50: standing goblin knock y (px/s).
    pub q50_air_kick_knock_y: i16,
    /// Q50: air kicks per airborne period.
    pub q50_air_kicks_per_period: u8,
    /// Q51: relaunch of a living flier, x (px/s).
    pub q51_relaunch_x: i16,
    /// Q51: relaunch y (px/s).
    pub q51_relaunch_y: i16,
    /// Q51: relaunches per original launch instance.
    pub q51_relaunches_per_launch: u8,
    /// Q51: lethal air kick launch x (px/s).
    pub q51_lethal_launch_x: i16,
    /// Q51: lethal air kick launch y (px/s).
    pub q51_lethal_launch_y: i16,
    /// Q52: tuck ticks (vy 0).
    pub q52_dive_tuck_ticks: u8,
    /// Q52: share of vx kept during the tuck (percent).
    pub q52_dive_tuck_vx_pct: u8,
    /// Q52: plunge vy (px/s).
    pub q52_dive_plunge_vy: i16,
    /// Q52: plunge vx forward (px/s).
    pub q52_dive_plunge_vx: i16,
    /// Q52: plunge active ticks at most.
    pub q52_dive_max_active_ticks: u8,
    /// Q52/Q48: the dive needs more than this height above the ground.
    pub q52_dive_min_height_px: u8,
    /// Q52: dive hit box.
    pub q52_dive_box: BoxPx,
    /// Q52: dive damage.
    pub q52_dive_damage: u8,
    /// Q52: dive hit-stop.
    pub q52_dive_hit_stop: u8,
    /// Q52: extra hit-stop on a lethal dive.
    pub q52_dive_hit_stop_lethal_bonus: u8,
    /// Q52: camera kick px.
    pub q52_dive_camera_kick_px: u8,
    /// Q52: dives per airborne period.
    pub q52_dives_per_period: u8,
    /// Q53: dive bounce vy (px/s; 0 = no bounce).
    pub q53_bounce_vy: i16,
    /// Q53: damage cap per airborne period.
    pub q53_max_damage_per_period: u8,
    /// Q54: the bail-out hop.
    pub q54_bailout_enabled: bool,
    /// Q54: bail-out vx away from the facing (px/s).
    pub q54_bailout_vx: i16,
    /// Q54: bail-out vy (px/s).
    pub q54_bailout_vy: i16,
    /// Q54: abs(move_x) at which the held direction wins.
    pub q54_bailout_direction_threshold_milli: u16,
    /// Q54: invulnerability ticks (0 by design).
    pub q54_bailout_iframes: u8,
    /// Q56: air kick landing recovery.
    pub q56_landing_air_kick: u8,
    /// Q56: dive whiff landing recovery.
    pub q56_landing_dive_whiff: u8,
    /// Q56: dive hit landing recovery (bounce).
    pub q56_landing_dive_hit: u8,
    /// Q56: bail-out landing recovery.
    pub q56_landing_bailout: u8,
    /// Q56: ordinary jump landing recovery (legacy).
    pub q56_landing_jump: u8,
    /// Q57: nerve range is 0..=this.
    pub q57_nerve_max: u8,
    /// Q57: ticks per drift step.
    pub q57_drift_period_ticks: u8,
    /// Q57: points of drift toward the baseline per step.
    pub q57_drift_step: u8,
    /// Q58: temperament bag, shuffled by draw 0.
    pub q58_bag: Vec<Temperament>,
    /// Q58: draw 1, nerve offset minimum.
    pub q58_nerve_offset_min: i8,
    /// Q58: draw 1, nerve offset maximum.
    pub q58_nerve_offset_max: i8,
    /// Q58: draw 2, panic length minimum (ticks).
    pub q58_panic_len_min: u8,
    /// Q58: draw 2, panic length maximum (ticks).
    pub q58_panic_len_max: u8,
    /// Q58: draw 3, first-approach delay minimum (ticks).
    pub q58_approach_delay_min: u8,
    /// Q58: draw 3, first-approach delay maximum (ticks).
    pub q58_approach_delay_max: u8,
    /// Q59: Brawler baseline nerve.
    pub q59_baseline_brawler: u8,
    /// Q59: Show-off baseline nerve.
    pub q59_baseline_show_off: u8,
    /// Q59: Coward baseline nerve.
    pub q59_baseline_coward: u8,
    /// Q59: temperaments that can become enraged.
    pub q59_enrage_temperaments: Vec<Temperament>,
    /// Q60: witness range, horizontal.
    pub q60_witness_dx_px: u16,
    /// Q60: witness range, vertical.
    pub q60_witness_dy_px: u16,
    /// Q60: routine defeat of a goblin.
    pub q60_delta_routine_defeat: i8,
    /// Q60: spectacular defeat.
    pub q60_delta_spectacular_defeat: i8,
    /// Q60: `WallSlam`, `BodyContact` or shore landing without a defeat.
    pub q60_delta_spectacle_no_defeat: i8,
    /// Q60: its own knockdown.
    pub q60_delta_own_knockdown: i8,
    /// Q60: a hero is hurt.
    pub q60_delta_hero_hurt: i8,
    /// Q60: its own hit on a hero.
    pub q60_delta_own_hit_on_hero: i8,
    /// Q60: a defeat of this force or more is spectacular.
    pub q60_spectacular_min_force: Force,
    /// Q60: causes that are spectacular at any force.
    pub q60_spectacular_causes: Vec<SpectacularCause>,
    /// Q61: panic when nerve falls below this.
    pub q61_panic_threshold: u8,
    /// Q61: hesitate when nerve falls below this.
    pub q61_hesitate_threshold: u8,
    /// Q61: hesitate length.
    pub q61_hesitate_ticks: u8,
    /// Q61: panic flee speed (1.25x walk).
    pub q61_panic_speed_px_s: u8,
    /// Q61: enrage when events raise nerve to this or more.
    pub q61_enrage_threshold: u8,
    /// Q61: enrage length.
    pub q61_enrage_ticks: u8,
    /// Q61: enraged walk speed.
    pub q61_enrage_walk_speed_px_s: u8,
    /// Q61: enraged `ATTACK_COOLDOWN` (legacy 40).
    pub q61_enrage_attack_cooldown: u8,
    /// Q62: the last goblin panics below this.
    pub q62_last_goblin_panic_threshold: u8,
    /// Q63: blocked this long ends a panic.
    pub q63_cornered_ticks: u8,
    /// Q63: nerve after a cornered panic.
    pub q63_cornered_nerve: u8,
    /// Q64: coward back-off trigger distance.
    pub q64_backoff_trigger_px: u8,
    /// Q64: a hero move with at least this startup triggers it.
    pub q64_backoff_min_startup: u8,
    /// Q64: hop-back distance.
    pub q64_backoff_distance_px: u8,
    /// Q64: hop-back duration.
    pub q64_backoff_ticks: u8,
    /// Q64: back-off cooldown.
    pub q64_backoff_cooldown: u8,
    /// Q65: taunt length.
    pub q65_taunt_ticks: u8,
    /// Q65: a hero hurt within this triggers a taunt.
    pub q65_taunt_hero_hurt_px: u16,
    /// Q65: a hero whiff within this triggers a taunt.
    pub q65_taunt_whiff_px: u8,
    /// Q65: taunt cooldown.
    pub q65_taunt_cooldown: u16,
    /// Q65: stagger of a non-launching hit during the taunt.
    pub q65_taunt_punish_stagger: u8,
    /// Q66: the token prefers the other side of the target.
    pub q66_pincer_token: bool,
    /// Q66: added to the trigger distance.
    pub q66_pincer_extra_px: u8,
    /// Q69: acting overlays (read only under `cf02_rules`).
    pub q69_acting_overlays: bool,
    /// Q70: near-miss distance for Carl's flinch.
    pub q70_flinch_px: u8,
    /// Q70: flinch ticks.
    pub q70_flinch_ticks: u8,
    /// Q70: moves without a hit that frustrate Carl.
    pub q70_frustration_moves: u8,
    /// Q70: window of those moves.
    pub q70_frustration_window_ticks: u16,
    /// Q70: frustration ticks.
    pub q70_frustration_ticks: u8,
    /// Q70: frustration cooldown.
    pub q70_frustration_cooldown: u16,
    /// Q71: Donut's startle distance.
    pub q71_startle_px: u16,
    /// Q71: startle ticks.
    pub q71_startle_ticks: u8,
    /// Q71: startle cooldown.
    pub q71_startle_cooldown: u16,
    /// Q71: smug ticks.
    pub q71_smug_ticks: u8,
    /// Q71: smug cooldown.
    pub q71_smug_cooldown: u16,
    /// Q72: extra presentation ticks a taunt pose holds when hit.
    pub q72_taunt_hold_ticks: u8,
    /// Q73: new Carl frames at most.
    pub q73_carl_frame_budget: u8,
    /// Q73: new Donut frames at most.
    pub q73_donut_frame_budget: u8,
    /// Q73: new goblin frames per sheet at most.
    pub q73_goblin_frame_budget_per_sheet: u8,
    /// Q73: new goblin frames in total at most.
    pub q73_goblin_frame_budget_total: u8,
}

impl Cf02Tuning {
    /// Parse and validate.
    pub fn from_json(text: &str) -> Result<Self, Cf02TuningError> {
        let tuning: Self =
            serde_json::from_str(text).map_err(|err| Cf02TuningError::Parse(err.to_string()))?;
        tuning.validate()?;
        Ok(tuning)
    }

    /// Parse the embedded file and require the pinned content hash.
    pub fn pinned() -> Result<Self, Cf02TuningError> {
        Self::from_json_pinned(TUNING_JSON, TUNING_HASH)
    }

    /// Parse `text` and require its content hash to equal `expected`.
    pub fn from_json_pinned(text: &str, expected: u64) -> Result<Self, Cf02TuningError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(Cf02TuningError::HashMismatch { expected, actual });
        }
        Self::from_json(text)
    }

    /// The one way to read a boolean switch of the file: it is effective only
    /// under `cf02_rules` (`q10_bowling`, `q11_wall_slam`, `q34_keep_down`,
    /// `q36_*`, `q47_*`, `q49_c*`, `q54/q55_*_enabled`, `q66_pincer_token`,
    /// `q69_acting_overlays`, ...). Reading a raw switch would leak a rule into
    /// `--demo`.
    pub fn on(&self, switch: bool) -> bool {
        self.cf02_rules && switch
    }

    /// Q18/Q37: llamas are hero-hit targets only under `cf02_rules`.
    pub fn llama_targets(&self) -> bool {
        self.on(self.q18_llama_targets)
    }

    pub fn validate(&self) -> Result<(), Cf02TuningError> {
        fn bad(field: &'static str, reason: &str) -> Result<(), Cf02TuningError> {
            Err(Cf02TuningError::Field {
                field,
                reason: reason.to_string(),
            })
        }
        fn positive(field: &'static str, value: u32) -> Result<(), Cf02TuningError> {
            if value == 0 {
                return bad(field, "must be positive");
            }
            Ok(())
        }
        fn ordered(field: &'static str, lo: i32, hi: i32) -> Result<(), Cf02TuningError> {
            if lo > hi {
                return bad(field, "the minimum must not exceed the maximum");
            }
            Ok(())
        }
        fn boxed(field: &'static str, b: &BoxPx) -> Result<(), Cf02TuningError> {
            if b.x0 >= b.x1 || b.y0 >= b.y1 {
                return bad(field, "needs x0 < x1 and y0 < y1");
            }
            Ok(())
        }
        fn pct(field: &'static str, value: u8) -> Result<(), Cf02TuningError> {
            if value > 100 {
                return bad(field, "must be at most 100");
            }
            Ok(())
        }
        fn span(field: &'static str, v: &[u16], width: u16) -> Result<(), Cf02TuningError> {
            match v {
                [a, b] if a < b && *b <= width => Ok(()),
                _ => bad(
                    field,
                    "needs exactly [from, to] with from < to <= room width",
                ),
            }
        }
        if self.schema_version != 1 {
            return bad("schema_version", "must be 1");
        }
        // Frame data and timers that must be positive.
        positive("q02_stomp_startup", u32::from(self.q02_stomp_startup))?;
        positive("q02_stomp_active", u32::from(self.q02_stomp_active))?;
        positive("q02_stomp_recovery", u32::from(self.q02_stomp_recovery))?;
        positive("q04_downed_ticks", u32::from(self.q04_downed_ticks))?;
        positive("q04_getup_ticks", u32::from(self.q04_getup_ticks))?;
        positive("q05_follow_startup", u32::from(self.q05_follow_startup))?;
        positive("q05_follow_active", u32::from(self.q05_follow_active))?;
        positive("q05_follow_recovery", u32::from(self.q05_follow_recovery))?;
        positive(
            "q46_move_buffer_ticks",
            u32::from(self.q46_move_buffer_ticks),
        )?;
        positive("q50_air_kick_startup", u32::from(self.q50_air_kick_startup))?;
        positive("q50_air_kick_active", u32::from(self.q50_air_kick_active))?;
        positive(
            "q50_air_kick_recovery",
            u32::from(self.q50_air_kick_recovery),
        )?;
        positive("q52_dive_tuck_ticks", u32::from(self.q52_dive_tuck_ticks))?;
        positive(
            "q52_dive_max_active_ticks",
            u32::from(self.q52_dive_max_active_ticks),
        )?;
        positive(
            "q57_drift_period_ticks",
            u32::from(self.q57_drift_period_ticks),
        )?;
        positive("q58_panic_len_min", u32::from(self.q58_panic_len_min))?;
        positive("q61_hesitate_ticks", u32::from(self.q61_hesitate_ticks))?;
        positive("q61_enrage_ticks", u32::from(self.q61_enrage_ticks))?;
        positive("q63_cornered_ticks", u32::from(self.q63_cornered_ticks))?;
        positive("q64_backoff_ticks", u32::from(self.q64_backoff_ticks))?;
        positive("q65_taunt_ticks", u32::from(self.q65_taunt_ticks))?;
        positive(
            "q32_restart_hold_ticks",
            u32::from(self.q32_restart_hold_ticks),
        )?;
        positive("q27_llama_tell_ticks", u32::from(self.q27_llama_tell_ticks))?;
        positive(
            "q27_llama_cooldown_ticks",
            u32::from(self.q27_llama_cooldown_ticks),
        )?;
        positive(
            "q27_llama_glob_speed_px_s",
            u32::from(self.q27_llama_glob_speed_px_s),
        )?;
        positive(
            "q27_llama_fire_range_px",
            u32::from(self.q27_llama_fire_range_px),
        )?;
        if self.q27_llama_attack_ticks < self.q27_llama_tell_ticks {
            return bad("q27_llama_attack_ticks", "must cover the tell");
        }
        if self.q27_llama_glob_y_min_px > self.q27_llama_glob_y_max_px {
            return bad("q27_llama_glob_y_max_px", "must not be below the minimum");
        }
        // Boxes.
        boxed("q02_stomp_box", &self.q02_stomp_box)?;
        boxed("q03_lying_box", &self.q03_lying_box)?;
        boxed("q50_air_kick_box", &self.q50_air_kick_box)?;
        boxed("q52_dive_box", &self.q52_dive_box)?;
        // Windows and ranges.
        ordered(
            "q05_follow_window_end",
            i32::from(self.q05_follow_window_start),
            i32::from(self.q05_follow_window_end),
        )?;
        ordered(
            "q49_c1_window_end",
            i32::from(self.q49_c1_window_start),
            i32::from(self.q49_c1_window_end),
        )?;
        ordered(
            "q58_nerve_offset_max",
            i32::from(self.q58_nerve_offset_min),
            i32::from(self.q58_nerve_offset_max),
        )?;
        ordered(
            "q58_panic_len_max",
            i32::from(self.q58_panic_len_min),
            i32::from(self.q58_panic_len_max),
        )?;
        ordered(
            "q58_approach_delay_max",
            i32::from(self.q58_approach_delay_min),
            i32::from(self.q58_approach_delay_max),
        )?;
        pct("q09_flier_keep_vx_pct", self.q09_flier_keep_vx_pct)?;
        pct("q52_dive_tuck_vx_pct", self.q52_dive_tuck_vx_pct)?;
        if !(1..=1000).contains(&self.q01_down_stick_threshold_milli) {
            return bad("q01_down_stick_threshold_milli", "must be in 1..=1000");
        }
        if !(1..=1000).contains(&self.q54_bailout_direction_threshold_milli) {
            return bad(
                "q54_bailout_direction_threshold_milli",
                "must be in 1..=1000",
            );
        }
        if self.q52_dive_plunge_vy >= 0 {
            return bad("q52_dive_plunge_vy", "must be negative (downward)");
        }
        if self.q53_bounce_vy < 0 {
            return bad("q53_bounce_vy", "must not be negative (0 = no bounce)");
        }
        if self.q53_max_damage_per_period == 0 {
            return bad("q53_max_damage_per_period", "must be positive");
        }
        if self.q45_perch_top_px <= self.q45_perch_underside_px {
            return bad("q45_perch_top_px", "must be above the underside");
        }
        // Morale: a small ladder of thresholds inside 0..=nerve_max.
        if self.q57_nerve_max == 0 || self.q57_nerve_max > 100 {
            return bad("q57_nerve_max", "must be in 1..=100");
        }
        let max = self.q57_nerve_max;
        if self.q58_bag.is_empty() {
            return bad("q58_bag", "must not be empty");
        }
        for (field, value) in [
            ("q59_baseline_brawler", self.q59_baseline_brawler),
            ("q59_baseline_show_off", self.q59_baseline_show_off),
            ("q59_baseline_coward", self.q59_baseline_coward),
            ("q61_hesitate_threshold", self.q61_hesitate_threshold),
            ("q61_enrage_threshold", self.q61_enrage_threshold),
            (
                "q62_last_goblin_panic_threshold",
                self.q62_last_goblin_panic_threshold,
            ),
            ("q63_cornered_nerve", self.q63_cornered_nerve),
        ] {
            if value > max {
                return bad(field, "must be at most q57_nerve_max");
            }
        }
        if self.q61_panic_threshold >= self.q61_hesitate_threshold {
            return bad(
                "q61_panic_threshold",
                "must be below the hesitate threshold",
            );
        }
        if self.q61_enrage_threshold <= self.q61_hesitate_threshold {
            return bad(
                "q61_enrage_threshold",
                "must be above the hesitate threshold",
            );
        }
        if self.q62_last_goblin_panic_threshold < self.q61_panic_threshold {
            return bad("q62_last_goblin_panic_threshold", "must not be below Q61's");
        }
        for (field, value) in [
            ("q60_delta_routine_defeat", self.q60_delta_routine_defeat),
            (
                "q60_delta_spectacular_defeat",
                self.q60_delta_spectacular_defeat,
            ),
            (
                "q60_delta_spectacle_no_defeat",
                self.q60_delta_spectacle_no_defeat,
            ),
            ("q60_delta_own_knockdown", self.q60_delta_own_knockdown),
        ] {
            if value > 0 {
                return bad(field, "a fright delta must not be positive");
            }
        }
        for (field, value) in [
            ("q60_delta_hero_hurt", self.q60_delta_hero_hurt),
            ("q60_delta_own_hit_on_hero", self.q60_delta_own_hit_on_hero),
        ] {
            if value < 0 {
                return bad(field, "a courage delta must not be negative");
            }
        }
        // Frame budget.
        if self.q73_goblin_frame_budget_total < self.q73_goblin_frame_budget_per_sheet {
            return bad(
                "q73_goblin_frame_budget_total",
                "must be at least the per-sheet budget",
            );
        }
        // The arena (Q29).
        positive("q29_room_width_px", u32::from(self.q29_room_width_px))?;
        positive("q29_room_height_px", u32::from(self.q29_room_height_px))?;
        if self.q29_floor_y > self.q29_room_height_px {
            return bad("q29_floor_y", "must be inside the room height");
        }
        let width = self.q29_room_width_px;
        span("q29_left_perch_x", &self.q29_left_perch_x, width)?;
        span("q29_right_perch_x", &self.q29_right_perch_x, width)?;
        span("q29_parapet_x", &self.q29_parapet_x, width)?;
        for (field, list) in [
            ("q29_checkpoints_x", &self.q29_checkpoints_x),
            ("q29_first_wave_goblins_x", &self.q29_first_wave_goblins_x),
            ("q29_second_wave_goblins_x", &self.q29_second_wave_goblins_x),
        ] {
            if list.is_empty() || list.iter().any(|x| *x >= width) {
                return bad(field, "needs at least one column inside the room");
            }
        }
        for (field, x) in [
            ("q29_carl_spawn_x", self.q29_carl_spawn_x),
            ("q29_donut_spawn_x", self.q29_donut_spawn_x),
            ("q29_shore_strike_x", self.q29_shore_strike_x),
            ("q29_llama_home_x", self.q29_llama_home_x),
        ] {
            if x >= width {
                return bad(field, "must be inside the room width");
            }
        }
        Ok(())
    }
}

/// The one definition of "this fixture runs under `cf02_rules`" (Q37): every
/// `cf02_*` fixture does; every other fixture, the story route and `--demo` do not
/// (`--cf02` turns the rules on for a run through `CombatState::with_cf02`).
pub fn fixture_runs_cf02(name: &str) -> bool {
    // ENEMY-GAMEPLAY-ENGINEER: the `eng_*` fixtures are CF02 room fixtures too.
    // CARL-PROG-01 CP-C: the two dynamite fixtures that prove the CF02-B launch rules.
    name.starts_with("cf02_")
        || name.starts_with("eng_")
        || crate::carl_prog_fixtures::runs_cf02(name)
        // DONUT-PROG-01 DP-F: the AI Donut's escape capture (the autonomy layer and
        // the 40 HP goblin are `cf02_rules` features).
        || name == "donut_prog_arena_blink_cornered"
}

/// The pinned tuning with the master switch on: what a simulation running under
/// `cf02_rules` reads (`--cf02` and every `cf02_*` fixture). One shared static, so
/// goblins and the move tables can hold a `&'static` reference to it. The committed
/// file keeps `cf02_rules: false`, so a run that never asks for this reads nothing.
pub fn active() -> &'static Cf02Tuning {
    static ACTIVE: std::sync::OnceLock<Cf02Tuning> = std::sync::OnceLock::new();
    ACTIVE.get_or_init(|| {
        let mut tuning = Cf02Tuning::pinned().expect("the embedded CF02 data is pinned and valid");
        tuning.cf02_rules = true;
        tuning
    })
}

/// The CF02 run configuration (`--cf02`, `--cf02-seed`). CF02-00 only carries
/// it: no simulation reads it yet, so a `--cf02` run replays the DEMO hall
/// exactly as `--demo` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cf02Config {
    /// `cf02_rules` for this run: `true` only under `--cf02`.
    pub rules: bool,
    /// Q58: the effective seed override (`--cf02-seed`); `None` = the fixture seed.
    pub seed: Option<u64>,
}

impl Cf02Config {
    /// Every CF02 rule off (every run except `--cf02`).
    pub const OFF: Self = Self {
        rules: false,
        seed: None,
    };

    /// The one-line description a run logs.
    pub fn log_line(&self) -> String {
        match self.seed {
            Some(seed) => format!("CF02_CONFIG: rules={} seed={seed}", self.rules),
            None => format!("CF02_CONFIG: rules={} seed=fixture", self.rules),
        }
    }
}

/// CF02-F: the microarena (`floor01_scene --cf02`), CF02-Q26..Q33, Q41, Q42, Q45.
///
/// The room is data (`assets/levels/floor01/cf02_microarena.room.json`) composed
/// from approved Floor 01 kit pieces by `camera_lab::layout`; the spawns, the
/// two waves, the checkpoints and the completion rule are the constants below,
/// which the tests pin to the adopted `q29_*` values of `cf02_combat_v0.json`.
/// Everything here is behind `cf02_rules`: `--demo` never loads it.
pub mod arena {
    use std::path::Path;

    use serde_json::Value;

    use crate::camera_lab::layout::{BASE_LAYOUT, compose, merged_contract};
    use crate::camera_lab::room::{IRect, TILE, parse_rooms};
    use crate::combat::EnemyAi;
    use crate::combat_scene::{CombatState, DemoRules, EnemySpawn, Fixture, LlamaSpawn};
    use crate::corridor::SceneSim;
    use crate::env_kit::{EnvKit, Layout, ViewDef};
    use crate::input::RawInput;

    /// The room geometry (camera-lab room schema, one room).
    pub const ROOM_PATH: &str = "assets/levels/floor01/cf02_microarena.room.json";
    pub const LAYOUT_ID: &str = "floor01_cf02_microarena";
    /// Q29: width, floor (the walking line) and the room height of the DEMO hall.
    pub const WIDTH: i32 = 1152;
    pub const FLOOR_Y: i32 = 580;
    /// Q29: the left and right end walls are the composer's drawn wall tile: one
    /// 16 px column (0..16 and 1136..1152), full height.
    pub const END_WALL_PX: i32 = 16;
    /// Q29: the rotten shore's strike column; its footprint is 409..489.
    pub const SHORE_STRIKE_X: i32 = 384;
    /// Q29: Carl's and Donut's spawn columns, and the two checkpoints.
    pub const CARL_SPAWN_X: i32 = 128;
    pub const DONUT_SPAWN_X: i32 = 96;
    pub const CHECKPOINTS: [i32; 2] = [128, 704];
    /// Q29: the parapet (solid, 48 px tall) and the two perches (top 80 px up).
    pub const PARAPET_X: (i32, i32) = (736, 768);
    pub const PARAPET_HEIGHT: i32 = 48;
    pub const LEFT_PERCH_X: (i32, i32) = (544, 672);
    pub const RIGHT_PERCH_X: (i32, i32) = (896, 992);
    pub const PERCH_TOP: i32 = 80;
    /// Q29: the llama's home column and leash.
    pub const LLAMA_HOME_X: i32 = 1000;

    const fn goblin(x: i32) -> EnemySpawn {
        EnemySpawn {
            x,
            ai: EnemyAi::Chase,
        }
    }

    /// Q29, Q32: the five goblins in two waves (a chaser notices a hero within the
    /// goblin sight of 220 px, so the waves wake by position: no scripted trigger).
    /// Index order: 0..=2 the brawl lane, 3 and 4 the llama yard.
    pub static GOBLINS: [EnemySpawn; 5] = [
        goblin(520),
        goblin(600),
        goblin(680),
        goblin(880),
        goblin(940),
    ];

    /// Q29: the Bad Llama of the llama yard (index 5 of the six enemies).
    pub static LLAMAS: [LlamaSpawn; 1] = [LlamaSpawn {
        x: LLAMA_HOME_X,
        ai: EnemyAi::Chase,
    }];

    /// Q29, Q32: the checkpoint rules of the room (the demo's `Respawn` machinery).
    pub static RULES: DemoRules = DemoRules {
        checkpoints: &CHECKPOINTS,
    };

    /// ENG-G-Q25: the four goblins of the Engineer variant (the yard goblin at
    /// 940 is replaced one for one by the Engineer, so the room keeps six enemies
    /// and the LAB-Q04 cap of four active). Q26: they take slots 0 to 3 of the same
    /// Q58 bag, so the fifth slot is unused.
    pub static GOBLINS_ENG: [EnemySpawn; 4] = [goblin(520), goblin(600), goblin(680), goblin(880)];

    /// ENG-G-Q25: the Engineer of the llama yard (enemy index 4, after the goblins).
    pub static ENGINEERS: [EnemySpawn; 1] = [goblin(940)];

    /// The live room: Carl at the spawn, five goblins, one llama, the shore.
    pub const fn fixture(
        name: &'static str,
        summary: &'static str,
        seed: u64,
        ticks: u64,
    ) -> Fixture {
        Fixture {
            name,
            summary,
            seed,
            carl_x: CARL_SPAWN_X,
            enemies: &GOBLINS,
            shore_dx: Some(SHORE_STRIKE_X - CARL_SPAWN_X),
            boss: None,
            engineers: &[],
            llamas: &LLAMAS,
            charges: &[],
            script: &[],
            ticks,
        }
    }

    /// ENG-G-Q25: the room of `floor01_scene --cf02 --cf02-engineer`: the same
    /// room with the yard goblin at 940 replaced by the Goblin Engineer.
    pub const fn fixture_eng(
        name: &'static str,
        summary: &'static str,
        seed: u64,
        ticks: u64,
    ) -> Fixture {
        Fixture {
            enemies: &GOBLINS_ENG,
            engineers: &ENGINEERS,
            ..fixture(name, summary, seed, ticks)
        }
    }

    /// The fixture `floor01_scene --cf02 --cf02-engineer` runs.
    pub static LIVE_ENG: Fixture = fixture_eng(
        "cf02_arena_eng",
        "ENEMY-GAMEPLAY-ENGINEER: the microarena with the Goblin Engineer in the llama yard, live (floor01_scene --cf02 --cf02-engineer)",
        0xCF02_F200,
        3600,
    );

    /// The Engineer room from the second checkpoint.
    pub static LIVE_ENG_CP2: Fixture = Fixture {
        name: "cf02_arena_eng_cp2",
        summary: "ENEMY-GAMEPLAY-ENGINEER: the Engineer room from the second checkpoint (floorbreak --cf02 --cf02-engineer --demo-beat 1)",
        seed: 0xCF02_F201,
        carl_x: CHECKPOINTS[1],
        shore_dx: Some(SHORE_STRIKE_X - CHECKPOINTS[1]),
        ..fixture_eng("", "", 0, 3600)
    };

    /// The room fixture of a run: the plain microarena, or (ENG-G-Q25) the Engineer
    /// variant when the flag is given **and** the data kill switch `tuning` allows
    /// it; with the switch off the flag runs the plain room (`cf02_arena`).
    pub fn fixture_for_run(
        beat: usize,
        engineer_flag: bool,
        tuning: &crate::engineer::EngineerTuning,
    ) -> &'static Fixture {
        if engineer_flag && tuning.enabled() {
            if beat == 0 { &LIVE_ENG } else { &LIVE_ENG_CP2 }
        } else {
            fixture_for_beat(beat)
        }
    }

    /// The room fixture a recorded `cf02_rules` demo session names: the Engineer
    /// room by its fixture name, every earlier session by its beat. The headless
    /// replay and the Bevy replay viewer share this match.
    pub fn fixture_for_recorded(name: &str, beat: usize) -> &'static Fixture {
        match name {
            "cf02_arena_eng" => &LIVE_ENG,
            "cf02_arena_eng_cp2" => &LIVE_ENG_CP2,
            _ => fixture_for_beat(beat),
        }
    }

    /// The fixture `floor01_scene --cf02` runs (nothing scripted: the player's).
    pub static LIVE: Fixture = fixture(
        "cf02_arena",
        "CF02-F: the microarena, live (floor01_scene --cf02)",
        0xCF02_F000,
        3600,
    );

    /// The fixture of `--demo-beat N` under `--cf02`: beat 0 starts at the spawn,
    /// any later beat at the second checkpoint (the first wave is retired).
    pub fn fixture_for_beat(beat: usize) -> &'static Fixture {
        if beat == 0 { &LIVE } else { &LIVE_CP2 }
    }

    /// The run that starts at the second checkpoint.
    pub static LIVE_CP2: Fixture = Fixture {
        name: "cf02_arena_cp2",
        summary: "CF02-F: the microarena from the second checkpoint (floor01_scene --cf02 --demo-beat 1)",
        seed: 0xCF02_F001,
        carl_x: CHECKPOINTS[1],
        shore_dx: Some(SHORE_STRIKE_X - CHECKPOINTS[1]),
        ..fixture("", "", 0, 3600)
    };

    /// Fixtures that run on the arena kit and geometry rather than the sample
    /// corridor: the room fixtures of CF02-F.
    pub fn fixture_uses_arena(name: &str) -> bool {
        name == "cf02_arena"
            || name.starts_with("eng_")
            || name.starts_with("cf02_arena_")
            || name.starts_with("companion_arena_")
            || name.starts_with("cf02_block_")
            // COMPANION-AI-02 CA2-B2: the `cf02_free_*` fixtures run in the room.
            || name.starts_with("cf02_free_")
            // DONUT-PROG-01 DP-D: the blink evidence fixtures that run in the room.
            || name.starts_with("donut_prog_arena_")
            || matches!(
                name,
                "cf02_goblin_self_own"
                    | "cf02_intact_shore_ignores_goblins"
                    | "cf02_parapet_cover"
                    | "cf02_perch_slab_climb"
                    | "cf02_perch_above_spit"
                    | "cf02_perch_dive"
            )
    }

    /// The rotten shore's source id in `CombatEvent::HeroHurt` (the falling block).
    pub const SHORE_BLOCK_SOURCE: u32 = 0x5348;

    /// Load the room: the geometry composed by the camera-lab composer, with the
    /// authored access steps replaced by the parapet, the two perches cut to one
    /// tile (Q45: a 16 px slab, top 80 px up, underside 68 px up (the deck tile's solid is 12 px), so Carl walks
    /// under it), the scene's two scroll-camera views, and no exit trigger (the
    /// room ends when its six enemies are defeated, see [`complete`]).
    pub fn load_arena(root: &Path) -> Result<EnvKit, String> {
        load_arena_with(root, &[])
    }

    /// DP-F: the 128 px ledge of the `donut_prog_arena_ledge_*` capture fixtures
    /// (`x`, `w`, height above the walking line), a masonry step like the
    /// parapet. The shipped room is untouched: only these fixture names add it.
    pub const LEDGE_128: (i32, i32, i32) = (136, 128, 128);

    /// [`load_arena`] for the room of fixture `name`: the shipped room, plus the
    /// [`LEDGE_128`] step for a `donut_prog_arena_ledge_*` capture fixture.
    pub fn load_arena_for(root: &Path, name: &str) -> Result<EnvKit, String> {
        if name.starts_with("donut_prog_arena_ledge_") {
            let (x, w, h) = LEDGE_128;
            load_arena_with(root, &[IRect::new(x, FLOOR_Y - h, w, h)])
        } else {
            load_arena(root)
        }
    }

    fn load_arena_with(root: &Path, extra_steps: &[IRect]) -> Result<EnvKit, String> {
        let path = root.join(ROOM_PATH);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut room = parse_rooms(&text)?
            .into_iter()
            .next()
            .ok_or("the arena file holds no room")?;
        let file: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        // No authored stairs: the perches are climbed by a jump (Q45), the
        // parapet is a masonry step the heroes jump.
        room.steps.clear();
        for p in file["scenarios"][0]["parapet"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let x = p["x"].as_i64().ok_or("parapet x")? as i32;
            let w = p["w"].as_i64().ok_or("parapet w")? as i32;
            let h = p["h"].as_i64().ok_or("parapet h")? as i32;
            room.steps.push(IRect::new(x, room.floor_y - h, w, h));
        }
        room.steps.extend_from_slice(extra_steps);
        let base_path = root.join(BASE_LAYOUT);
        let base: Value = serde_json::from_str(
            &std::fs::read_to_string(&base_path)
                .map_err(|e| format!("{}: {e}", base_path.display()))?,
        )
        .map_err(|e| e.to_string())?;
        let (mut layout, _plan) = compose(&room, &base);
        // One tile thick: drop the face row under each perch's deck row.
        for p in &room.platforms {
            let face_row = ((p.rect.y - 4) / TILE + 1) as usize;
            for col in (p.rect.x / TILE)..(p.rect.right() / TILE) {
                layout["tile_layers"]["collision"][face_row][col as usize] = Value::Null;
            }
        }
        let layout: Layout = serde_json::from_value(layout).map_err(|e| e.to_string())?;
        let contract = merged_contract(root).map_err(|e| e.to_string())?;
        let mut kit = EnvKit::build(&contract, &layout).map_err(|e| e.to_string())?;
        // The scene's camera takes its vertical position from a named view: the
        // walking line 288 px below the top of the 640x360 view (308 px below the
        // top of the 640x400 view), like the DEMO hall.
        kit.views.insert(
            "abc_640x360".to_owned(),
            ViewDef {
                x: 0,
                y: FLOOR_Y - 288,
                w: 640,
                h: 360,
            },
        );
        kit.views.insert(
            "abc_640x400".to_owned(),
            ViewDef {
                x: 0,
                y: FLOOR_Y - 308,
                w: 640,
                h: 400,
            },
        );
        // DM-Q27: the optional lane ceiling, the boss room's pattern: a solid
        // slab whose underside is `ceiling_px` above the walking line, so a
        // jump from a perch stays below the HUD band. Absent = none.
        if let Some(ceiling) = room.ceiling_px {
            kit.solid_rects.push(crate::env_kit::IRect {
                x: 0,
                y: FLOOR_Y - ceiling - 16,
                w: kit.width,
                h: 16,
            });
        }
        kit.blink_barriers = room.kit_blink_barriers();
        kit.layout_id = LAYOUT_ID.to_owned();
        kit.exit = None;
        Ok(kit)
    }

    /// Q32: every enemy of the room is defeated (five goblins and the llama): the
    /// exit opens (the console line `CF02_ARENA_COMPLETE`).
    pub fn complete(combat: &CombatState) -> bool {
        !combat.enemies.is_empty()
            && combat.enemies.iter().all(|g| !g.alive())
            && combat.llamas.iter().all(|l| !l.alive())
    }

    /// The enemies of the room still alive (goblins, then the llama).
    pub fn enemies_left(combat: &CombatState) -> usize {
        combat.enemies.iter().filter(|g| g.alive()).count()
            + combat.llamas.iter().filter(|l| l.alive()).count()
    }

    // -------------------------------------------------------------- autopilot --

    /// Carl's reach, in px of centre distance, for a jab and a kick on a goblin,
    /// and for a kick on the llama (its 46 px body reaches 42 px from its column).
    const JAB_REACH: f32 = 30.0;
    const LLAMA_KICK_REACH: f32 = 36.0;
    /// Ticks between two presses of the autopilot (a human mashes slower).
    const PRESS_CADENCE: u64 = 4;
    /// The autopilot stays this far left of the shore column's footprint while
    /// goblins are near it (the footprint starts at 409), so it never walks the
    /// careless line that the block punishes.
    const SHORE_KEEP_OUT: f32 = 40.0;

    /// How the autopilot treats the rotten shore.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Plan {
        /// Fight straight through both waves; never hits the shore on purpose.
        Direct,
        /// Wait left of the column, break the shore (three jabs from the left)
        /// and let the goblins that chase him walk under the block.
        Environment,
    }

    /// A deterministic stand-in for the player, from the finished simulation state
    /// only (it never reads the camera). It walks to the nearest living enemy
    /// ahead, fights the nearest awake goblin with jabs and haymakers, kicks the
    /// llama, and jumps the parapet. Used by the headless fixtures and the
    /// evidence captures; the live room is driven by the keyboard or pad.
    pub fn autopilot(combat: &CombatState, scene: &SceneSim, plan: Plan) -> RawInput {
        let hero = &scene.world.hero;
        let tick = scene.world.tick + 1;
        let press = tick.is_multiple_of(PRESS_CADENCE);
        let towards = |x: f32| {
            let dx = x - hero.x;
            if dx.abs() < 4.0 { 0.0 } else { dx.signum() }
        };
        let mut raw = RawInput::default();
        if combat.hero_defeated {
            return raw;
        }
        let shore_open = combat.shore.as_ref().is_some_and(|s| !s.landed);
        // Environment plan: stand left of the column and break the shore first.
        if plan == Plan::Environment
            && shore_open
            && combat.shore.as_ref().is_some_and(|s| s.hits < 3)
        {
            let stand = SHORE_STRIKE_X as f32 - 20.0;
            let near = (hero.x - stand).abs() < 6.0;
            raw.move_x = if near { 1.0 } else { towards(stand) };
            if near && tick.is_multiple_of(40) {
                raw.attack = true;
            }
            return raw;
        }
        let threat = combat
            .enemies
            .iter()
            .filter(|g| g.alive() && g.ai == EnemyAi::Chase)
            .min_by(|a, b| (a.x - hero.x).abs().total_cmp(&(b.x - hero.x).abs()));
        if let Some(g) = threat
            && (g.x - hero.x).abs() < 220.0
        {
            let dx = g.x - hero.x;
            let dir = dx.signum();
            if dx.abs() > JAB_REACH {
                raw.move_x = dir;
                // Direct: do not walk into the footprint while the block can still fall.
                if plan == Plan::Direct
                    && shore_open
                    && combat.shore.as_ref().is_some_and(|s| s.hits > 0)
                    && hero.x > SHORE_STRIKE_X as f32 - SHORE_KEEP_OUT
                    && hero.x < SHORE_STRIKE_X as f32 + 110.0
                    && dir > 0.0
                {
                    raw.move_x = 0.0;
                }
            } else {
                raw.move_x = dir;
                if press {
                    raw.attack = true;
                }
            }
            if hero.grounded && raw.move_x != 0.0 && blocked_ahead(scene, raw.move_x) {
                raw.jump = true;
            }
            return raw;
        }
        if let Some(l) = combat.llamas.iter().find(|l| l.alive()) {
            let dx = l.x - hero.x;
            if dx.abs() < 320.0 {
                raw.move_x = dx.signum();
                if dx.abs() <= LLAMA_KICK_REACH && press {
                    raw.physical = true;
                }
                if hero.grounded && blocked_ahead(scene, raw.move_x) {
                    raw.jump = true;
                }
                return raw;
            }
        }
        // Nothing awake: walk on to the nearest enemy that is still alive.
        let goal = combat
            .enemies
            .iter()
            .filter(|g| g.alive())
            .map(|g| g.x)
            .chain(combat.llamas.iter().filter(|l| l.alive()).map(|l| l.x))
            .min_by(f32::total_cmp)
            .map_or(WIDTH as f32 - 80.0, |x| x - 150.0);
        raw.move_x = towards(goal);
        if hero.grounded && raw.move_x != 0.0 && blocked_ahead(scene, raw.move_x) {
            raw.jump = true;
        }
        raw
    }

    /// P2 Donut as a human would play the room: stay a step behind Carl, swat a
    /// goblin's tell within reach (the swat interrupts it), pounce a goblin that
    /// lies (it holds it for Carl; the pounce never kills a 2 HP goblin it has not
    /// wounded) and otherwise follow. A pure function of the finished state.
    pub fn donut_pilot(combat: &CombatState, scene: &SceneSim) -> RawInput {
        donut_pilot_at(combat, scene, -40.0)
    }

    /// [`donut_pilot`] with the follow goal `goal_dx` px from Carl (negative:
    /// behind him, the shipped pilot's -40).
    fn donut_pilot_at(combat: &CombatState, scene: &SceneSim, goal_dx: f32) -> RawInput {
        use crate::goblin::{AttackPhase, GoblinMode};
        let body = &scene.world.donut_body;
        let carl = &scene.world.hero;
        let tick = scene.world.tick + 1;
        let mut raw = RawInput::default();
        // A goblin in its tell within a swat's reach in front of Donut.
        let telling = combat.enemies.iter().find(|g| {
            g.alive()
                && g.mode == GoblinMode::Attack
                && g.attack_phase() == Some(AttackPhase::Tell)
                && (g.x - body.x).abs() < 100.0
        });
        if let Some(g) = telling {
            let dx = g.x - body.x;
            raw.move_x = dx.signum();
            if dx.abs() <= 30.0 && tick.is_multiple_of(PRESS_CADENCE) {
                raw.attack = true;
            }
            return raw;
        }
        let lying = combat.enemies.iter().find(|g| {
            g.alive()
                && matches!(g.mode, GoblinMode::Downed)
                && (g.x - body.x).abs() < 90.0
                && (g.x - body.x).abs() > 30.0
        });
        if let Some(g) = lying {
            raw.move_x = (g.x - body.x).signum();
            if tick.is_multiple_of(PRESS_CADENCE * 4) && body.grounded {
                raw.physical = true;
            }
            return raw;
        }
        let goal = carl.x + goal_dx;
        let dx = goal - body.x;
        raw.move_x = if dx.abs() < 14.0 { 0.0 } else { dx.signum() };
        if body.grounded && raw.move_x != 0.0 && blocked_ahead_of(scene, body.x, body.y, raw.move_x)
        {
            raw.jump = true;
        }
        raw
    }

    /// DP-F (DP-Q47): [`donut_pilot`] with one scripted rule, the human's blink. A
    /// goblin's swing aimed at her within [`BLINK_THREAT_PX`] whose impact is
    /// due 7 to 12 ticks from now (more than the windup, at most the windup plus
    /// 6: the DP-Q39 (b) window, read from [`crate::goblin::Goblin::tell_view`])
    /// makes her press Circle/B with the stick pushed away from it (toward Carl
    /// when level), while the blink is ready and paid for. Nothing else is
    /// pressed on the press tick or during the windup (a primary, physical or
    /// jump press on t0..t5 cancels the blink, DP-Q25), and while the blink is
    /// ready and a swing aimed at her is up, her swat, pounce and jump are
    /// withheld (a busy hero cannot start it). The script presses the
    /// blink; the AI's own blink is measured by the `cf02_free_blink_*` fixtures.
    /// A pure function of the finished state.
    pub fn donut_blink_pilot(combat: &CombatState, scene: &SceneSim) -> RawInput {
        let mut raw = donut_pilot_at(combat, scene, BLINK_PILOT_GOAL_DX);
        let blink = combat.donut_blink();
        let tick = scene.world.tick + 1;
        if blink.windup().is_some() {
            raw.attack = false;
            raw.physical = false;
            raw.jump = false;
            return raw;
        }
        let t = crate::donut_prog::DonutProgTuning::active();
        let charges = combat.donut_mana_view().map_or(0, |m| m.0);
        if blink.recovering(tick) || blink.cooling(tick) || charges < t.dpq23_cost_charges {
            return raw;
        }
        let body = &scene.world.donut_body;
        let windup = u16::from(t.dpq25_windup_ticks);
        // The swings aimed at her within reach, as (x, ticks to impact).
        let mut aimed = combat.enemies.iter().enumerate().filter_map(|(i, g)| {
            let (tell, _, impact) = g.tell_view()?;
            (combat.target_of(i) == crate::party::HeroId::Donut
                && (g.x - body.x).abs() <= BLINK_THREAT_PX)
                .then(|| (g.x, impact.saturating_sub(tell)))
        });
        // A blink that is ready frees her hands for it: no swat, pounce or jump
        // while a swing aimed at her is up (a busy hero cannot start the blink).
        let Some(first) = aimed.next() else {
            return raw;
        };
        raw.attack = false;
        raw.physical = false;
        raw.jump = false;
        let threat = std::iter::once(first)
            .chain(aimed)
            .find_map(|(x, left)| (left > windup && left <= windup + 6).then_some(x));
        let Some(threat_x) = threat else {
            return raw;
        };
        let carl = scene.world.hero.x;
        let away = if (body.x - threat_x).abs() < 0.5 {
            carl - body.x
        } else {
            body.x - threat_x
        };
        RawInput {
            move_x: if away < 0.0 { -1.0 } else { 1.0 },
            ability: true,
            ..RawInput::default()
        }
    }

    /// The reach of the scripted blink rule (DP-Q39: `dpq39_ai_threat_px`).
    const BLINK_THREAT_PX: f32 = 48.0;

    /// Where the blink pilot stands, px from Carl: **ahead** of him (the shipped
    /// pilot trails him at -40). In the shipped room the goblins pick the nearer
    /// hero, and a P2 who trails Carl is never the target (0 swings aimed at her
    /// in 16 seeded runs), so the rule would be dead code; a Donut a step ahead
    /// draws the swings the blink is for. 32 is the one value of a 16..48 sweep
    /// where every seed 0..8 presses the blink and 7 of 8 clear the room.
    const BLINK_PILOT_GOAL_DX: f32 = 32.0;

    /// A solid within 14 px ahead of Carl's body at ledge height.
    fn blocked_ahead(scene: &SceneSim, dir: f32) -> bool {
        blocked_ahead_of(scene, scene.world.hero.x, scene.world.hero.y, dir)
    }

    fn blocked_ahead_of(scene: &SceneSim, x: f32, y: f32, dir: f32) -> bool {
        use crate::combat::solid_at;
        use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Rect};
        let probe = Rect {
            left: x - HERO_HALF_WIDTH + dir * 14.0,
            bottom: y - HERO_HALF_HEIGHT + 4.0,
            width: HERO_HALF_WIDTH * 2.0,
            height: 8.0,
        };
        solid_at(&scene.world.solids, probe)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::camera_lab::layout::build_kit;
        use crate::cf02::Cf02Tuning;

        fn root() -> &'static Path {
            Path::new(env!("CARGO_MANIFEST_DIR"))
        }

        /// The constants of this module are the adopted `q29_*` values of the data
        /// file, so the room, the spawns and the file cannot drift apart.
        #[test]
        fn the_room_constants_are_the_adopted_q29_values() {
            let t = Cf02Tuning::pinned().unwrap();
            assert_eq!(
                (t.q29_room_width_px, t.q29_floor_y, t.q29_room_height_px),
                (WIDTH as u16, FLOOR_Y as u16, 660)
            );
            assert_eq!(i32::from(t.q29_end_wall_px), END_WALL_PX);
            assert_eq!(i32::from(t.q29_carl_spawn_x), CARL_SPAWN_X);
            assert_eq!(i32::from(t.q29_donut_spawn_x), DONUT_SPAWN_X);
            assert_eq!(i32::from(t.q29_shore_strike_x), SHORE_STRIKE_X);
            assert_eq!(i32::from(t.q29_llama_home_x), LLAMA_HOME_X);
            let pair = |v: &[u16]| (i32::from(v[0]), i32::from(v[1]));
            assert_eq!(pair(&t.q29_left_perch_x), LEFT_PERCH_X);
            assert_eq!(pair(&t.q29_right_perch_x), RIGHT_PERCH_X);
            assert_eq!(pair(&t.q29_parapet_x), PARAPET_X);
            assert_eq!(i32::from(t.q29_parapet_height_px), PARAPET_HEIGHT);
            assert_eq!(i32::from(t.q45_perch_top_px), PERCH_TOP);
            let columns = |v: &[u16]| v.iter().map(|x| i32::from(*x)).collect::<Vec<_>>();
            assert_eq!(columns(&t.q29_checkpoints_x), CHECKPOINTS);
            let spawns: Vec<i32> = GOBLINS.iter().map(|g| g.x).collect();
            let mut expected = columns(&t.q29_first_wave_goblins_x);
            expected.extend(columns(&t.q29_second_wave_goblins_x));
            assert_eq!(spawns, expected);
            assert_eq!(GOBLINS.len() + LLAMAS.len(), 6, "five goblins and a llama");
            assert!(GOBLINS.iter().all(|g| g.ai == EnemyAi::Chase));
            assert_eq!(LIVE.shore_dx, Some(256));
        }

        /// Knobs the code does not read as a number are pinned to the thing they
        /// describe, so editing one of them cannot silently do nothing.
        #[test]
        fn the_unread_knobs_equal_what_they_describe() {
            let t = Cf02Tuning::pinned().unwrap();
            assert_eq!(t.q42_block_hurt_ticks, crate::feel::HERO_HURT_TICKS);
            assert_eq!(
                t.q42_block_invulnerable_ticks,
                crate::combat::HERO_INVULNERABLE_TICKS
            );
            assert_eq!(f32::from(t.q29_llama_leash_px), crate::feel::LLAMA_LEASH);
            // The drawn slab is one tile; its collider is 12 px (underside = top - 12).
            assert_eq!(i32::from(t.q45_perch_slab_px), TILE);
            assert_eq!(t.q45_perch_top_px - t.q45_perch_underside_px, 12);
        }

        #[test]
        fn the_room_file_is_the_q29_layout() {
            let kit = load_arena(root()).unwrap();
            assert_eq!(
                (kit.width, kit.base_walk_line_y),
                (WIDTH, FLOOR_Y),
                "1152 wide, walking line 580"
            );
            // 660 high, rounded up to whole tiles for the 640x400 view (272 + 400).
            assert_eq!(kit.height, 672);
            let solid = |x: i32, y: i32, w: i32, h: i32| {
                kit.solid_rects
                    .iter()
                    .any(|r| r.x <= x && r.y <= y && r.x + r.w >= x + w && r.y + r.h >= y + h)
            };
            // End walls, full height; the floor between them.
            for x in [0, WIDTH - END_WALL_PX] {
                assert!(solid(x, 32, END_WALL_PX, 540), "end wall tile at {x}");
            }
            assert!(
                solid(END_WALL_PX, FLOOR_Y, WIDTH - 2 * END_WALL_PX, 8),
                "floor"
            );
            // Parapet: 736..768, 48 px tall.
            assert!(solid(
                PARAPET_X.0,
                FLOOR_Y - PARAPET_HEIGHT,
                32,
                PARAPET_HEIGHT - 4
            ));
            assert!(!solid(PARAPET_X.0, FLOOR_Y - PARAPET_HEIGHT - 4, 32, 4));
            // Perches: a one-tile slab, top 80 px up; Carl (52 px) walks under it.
            for (x0, x1) in [LEFT_PERCH_X, RIGHT_PERCH_X] {
                let top = FLOOR_Y - PERCH_TOP;
                assert!(solid(x0, top, x1 - x0, 12), "slab {x0}..{x1}");
                assert!(!solid(x0, top + 12, x1 - x0, 8), "no face under the slab");
                let under = kit
                    .solid_rects
                    .iter()
                    .filter(|r| r.x < x1 && r.x + r.w > x0 && r.y > 32 && r.y < FLOOR_Y)
                    .map(|r| r.y + r.h)
                    .max()
                    .unwrap();
                assert!(FLOOR_Y - under >= 52 + 12, "headroom {}", FLOOR_Y - under);
                // The built underside is the knob (the deck tile's 12 px collider).
                let t = Cf02Tuning::pinned().unwrap();
                assert_eq!(FLOOR_Y - under, i32::from(t.q45_perch_underside_px));
            }
            // No stairs: nothing else stands in the lane.
            let lane: Vec<_> = kit
                .solid_rects
                .iter()
                .filter(|r| {
                    r.y > 32
                        && r.y < FLOOR_Y
                        && r.x >= END_WALL_PX
                        && r.x + r.w <= WIDTH - END_WALL_PX
                })
                .collect();
            assert_eq!(lane.len(), 3, "two perches and the parapet: {lane:?}");
        }

        /// The headline: no camera position, at any coverage, shows an undrawn cell
        /// (the whole room, at the widest view and at 640x400).
        #[test]
        fn the_whole_render_bounds_is_drawn_so_no_view_shows_void() {
            let text = std::fs::read_to_string(root().join(ROOM_PATH)).unwrap();
            let room = parse_rooms(&text).unwrap().remove(0);
            let (kit, _, plan) = build_kit(root(), &room).unwrap();
            let kit2 = load_arena(root()).unwrap();
            assert_eq!((kit.width, kit.height), (kit2.width, kit2.height));
            let cols = (kit2.width / TILE) as usize;
            let rows = (kit2.height / TILE) as usize;
            let mut drawn = vec![vec![false; cols]; rows];
            for t in &kit2.tiles {
                drawn[t.row as usize][t.col as usize] = true;
            }
            let in_opening = |c: usize, r: usize| {
                let (x, y) = (c as i32 * TILE + 8, r as i32 * TILE + 8);
                plan.openings
                    .iter()
                    .any(|&(ox, oy, ow, oh)| x >= ox && x < ox + ow && y >= oy && y < oy + oh)
            };
            let holes: Vec<_> = (0..rows)
                .flat_map(|r| (0..cols).map(move |c| (c, r)))
                .filter(|&(c, r)| !drawn[r][c] && !in_opening(c, r))
                .collect();
            // Cells with no tile and no opening: only the strip above the ceiling row.
            assert!(
                holes.iter().all(|&(_, r)| r < 2) || holes.is_empty(),
                "undrawn cells: {:?}",
                &holes[..holes.len().min(8)]
            );
            // Both scroll views and the widest auto view (1.5x of 640x360, 960x540)
            // sit inside the room.
            for (w, h, top) in [(640, 360, FLOOR_Y - 288), (640, 400, FLOOR_Y - 308)] {
                assert!(top >= 0 && top + h <= kit2.height, "{w}x{h}");
            }
            const { assert!(WIDTH >= 960) };
            // The rig clamps a taller view into the room, so it fits (672 >= 540).
            assert!(kit2.height >= 540);
        }

        /// DM-Q27: `ceiling_px` is an optional room field. Absent is none; a
        /// positive integer is the underside of a lane ceiling; anything else is
        /// a load error that names the room.
        #[test]
        fn the_ceiling_field_is_optional_and_validated() {
            let text = std::fs::read_to_string(root().join(ROOM_PATH)).unwrap();
            let mut value: Value = serde_json::from_str(&text).unwrap();
            let with = |value: &mut Value, ceiling: Value| {
                value["scenarios"][0]["ceiling_px"] = ceiling;
                parse_rooms(&value.to_string())
            };
            value["scenarios"][0]
                .as_object_mut()
                .unwrap()
                .remove("ceiling_px");
            assert_eq!(parse_rooms(&value.to_string()).unwrap()[0].ceiling_px, None);
            let rooms = with(&mut value, serde_json::json!(228)).unwrap();
            assert_eq!(rooms[0].ceiling_px, Some(228));
            for bad in [
                serde_json::json!(0),
                serde_json::json!(-4),
                serde_json::json!("x"),
            ] {
                let err = with(&mut value, bad).unwrap_err();
                assert!(
                    err.contains("CF02-ARENA") && err.contains("ceiling_px"),
                    "{err}"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn tuning() -> Cf02Tuning {
        Cf02Tuning::pinned().expect("the embedded file is pinned")
    }

    fn field_of(text: &str) -> &'static str {
        match Cf02Tuning::from_json(text) {
            Err(Cf02TuningError::Field { field, .. }) => field,
            other => panic!("expected a field error, got {other:?}"),
        }
    }

    #[test]
    fn tuning_hash_is_pinned() {
        // Fails when cf02_combat_v0.json changes without updating TUNING_HASH.
        assert_eq!(content_hash(TUNING_JSON), TUNING_HASH);
        assert!(Cf02Tuning::pinned().is_ok());
    }

    #[test]
    fn a_drifted_file_is_a_clear_hash_failure() {
        let drifted = TUNING_JSON.replace("\"q02_stomp_startup\": 9", "\"q02_stomp_startup\": 10");
        assert_ne!(drifted, TUNING_JSON);
        match Cf02Tuning::from_json_pinned(&drifted, TUNING_HASH) {
            Err(Cf02TuningError::HashMismatch { expected, actual }) => {
                assert_eq!(expected, TUNING_HASH);
                assert_ne!(actual, expected);
            }
            other => panic!("expected a hash mismatch, got {other:?}"),
        }
        // The same edit parses fine unpinned: the pin is what catches it.
        assert!(Cf02Tuning::from_json(&drifted).is_ok());
    }

    #[test]
    fn the_schema_round_trips_without_losing_or_inventing_a_field() {
        let t = tuning();
        let written = serde_json::to_string_pretty(&t).unwrap();
        assert_eq!(Cf02Tuning::from_json(&written).unwrap(), t);
        let original: Value = serde_json::from_str(TUNING_JSON).unwrap();
        assert_eq!(serde_json::to_value(&t).unwrap(), original);
    }

    #[test]
    fn every_non_party_adopted_question_has_a_knob() {
        let value: Value = serde_json::from_str(TUNING_JSON).unwrap();
        let keys: Vec<&String> = value.as_object().unwrap().keys().collect();
        let exempt: Vec<u8> = NO_KNOB.iter().map(|(q, _)| *q).collect();
        for q in 1..=75u8 {
            let prefix = format!("q{q:02}_");
            let has_key = keys.iter().any(|k| k.starts_with(&prefix));
            if exempt.contains(&q) {
                assert!(!has_key, "Q{q:02} is listed as knob-less but has a key");
            } else {
                assert!(has_key, "Q{q:02} has no knob and is not in NO_KNOB");
            }
        }
        // Every key names a question (or is one of the two header keys).
        for key in keys {
            let named = key.starts_with('q')
                && key[1..3].parse::<u8>().is_ok_and(|q| (1..=75).contains(&q))
                && key.as_bytes()[3] == b'_';
            assert!(
                named || key == "schema_version" || key == "cf02_rules",
                "{key} is not named after a question id"
            );
        }
    }

    #[test]
    fn q27_knobs_equal_the_unchanged_llama_constants() {
        use crate::feel::{LLAMA_COOLDOWN, LLAMA_FIRE_RANGE, LLAMA_GLOB_SPEED, LLAMA_TELL_TICKS};
        let t = tuning();
        assert_eq!(t.q27_llama_tell_ticks, LLAMA_TELL_TICKS);
        assert_eq!(t.q27_llama_cooldown_ticks, LLAMA_COOLDOWN);
        assert_eq!(f32::from(t.q27_llama_glob_speed_px_s), LLAMA_GLOB_SPEED);
        assert_eq!(f32::from(t.q27_llama_fire_range_px), LLAMA_FIRE_RANGE);
    }

    #[test]
    fn the_master_switch_is_off_and_gates_the_llama_switch() {
        let t = tuning();
        assert!(!t.cf02_rules, "every legacy golden needs the switch off");
        assert!(
            t.q18_llama_targets,
            "the data value is on, the gate is the master switch"
        );
        assert!(!t.llama_targets());
        assert!(
            !t.on(true),
            "no switch is effective with the master switch off"
        );
        let on = Cf02Tuning {
            cf02_rules: true,
            ..t
        };
        assert!(on.llama_targets());
        assert_eq!(Cf02Config::OFF, Cf02Config::default());
    }

    #[test]
    fn spot_values_match_the_adopted_proposal() {
        let t = tuning();
        assert_eq!(
            (
                t.q02_stomp_startup,
                t.q02_stomp_active,
                t.q02_stomp_recovery
            ),
            (9, 3, 14)
        );
        assert_eq!(
            t.q02_stomp_box,
            BoxPx {
                x0: 6,
                x1: 26,
                y0: 0,
                y1: 10
            }
        );
        assert_eq!((t.q04_downed_ticks, t.q04_getup_ticks), (40, 22));
        assert_eq!(t.q46_move_buffer_ticks, 8);
        assert_eq!(t.q52_dive_plunge_vy, -480);
        assert_eq!(t.q53_max_damage_per_period, 2);
        assert_eq!(t.q54_bailout_iframes, 0);
        assert_eq!(
            t.q58_bag,
            [
                Temperament::Brawler,
                Temperament::Brawler,
                Temperament::Coward,
                Temperament::Coward,
                Temperament::ShowOff
            ]
        );
        assert_eq!(
            (
                t.q59_baseline_brawler,
                t.q59_baseline_show_off,
                t.q59_baseline_coward
            ),
            (70, 60, 45)
        );
        assert_eq!((t.q61_panic_threshold, t.q61_hesitate_threshold), (25, 50));
        assert_eq!(t.q42_block_knock_x, 120);
        assert_eq!(t.q29_room_width_px, 1152);
        assert_eq!(t.q73_carl_frame_budget, 13);
    }

    #[test]
    fn bad_configs_are_rejected_and_name_the_field() {
        let edit = |from: &str, to: &str| {
            assert!(TUNING_JSON.contains(from), "{from}");
            TUNING_JSON.replacen(from, to, 1)
        };
        assert_eq!(
            field_of(&edit("\"schema_version\": 1", "\"schema_version\": 2")),
            "schema_version"
        );
        assert_eq!(
            field_of(&edit(
                "\"q02_stomp_startup\": 9",
                "\"q02_stomp_startup\": 0"
            )),
            "q02_stomp_startup"
        );
        assert_eq!(
            field_of(&edit(
                "\"x0\": 6,\n    \"x1\": 26",
                "\"x0\": 30,\n    \"x1\": 26"
            )),
            "q02_stomp_box"
        );
        assert_eq!(
            field_of(&edit(
                "\"q52_dive_plunge_vy\": -480",
                "\"q52_dive_plunge_vy\": 480"
            )),
            "q52_dive_plunge_vy"
        );
        assert_eq!(
            field_of(&edit(
                "\"q61_panic_threshold\": 25",
                "\"q61_panic_threshold\": 60"
            )),
            "q61_panic_threshold"
        );
        assert_eq!(
            field_of(&edit(
                "\"q58_nerve_offset_min\": -5",
                "\"q58_nerve_offset_min\": 6"
            )),
            "q58_nerve_offset_max"
        );
        assert_eq!(
            field_of(&edit(
                "\"q60_delta_hero_hurt\": 15",
                "\"q60_delta_hero_hurt\": -15"
            )),
            "q60_delta_hero_hurt"
        );
        // An unknown or a missing field is a parse error that names it.
        let unknown = edit(
            "\"cf02_rules\": false,",
            "\"cf02_rules\": false,\n  \"q99_extra\": 1,",
        );
        assert!(matches!(
            Cf02Tuning::from_json(&unknown),
            Err(Cf02TuningError::Parse(msg)) if msg.contains("q99_extra")
        ));
        let missing = edit("  \"q02_stomp_damage\": 1,\n", "");
        assert!(matches!(
            Cf02Tuning::from_json(&missing),
            Err(Cf02TuningError::Parse(msg)) if msg.contains("q02_stomp_damage")
        ));
    }

    #[test]
    fn config_logs_the_effective_seed() {
        assert_eq!(
            Cf02Config::OFF.log_line(),
            "CF02_CONFIG: rules=false seed=fixture"
        );
        let c = Cf02Config {
            rules: true,
            seed: Some(7),
        };
        assert_eq!(c.log_line(), "CF02_CONFIG: rules=true seed=7");
    }
}
