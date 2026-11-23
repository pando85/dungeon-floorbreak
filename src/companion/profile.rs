//! `assets/data/gameplay/companion_ai_v0.json`: the companion profiles.
//!
//! Three sets (CA-21): `cf02_assist` is exactly the CF02-Q13..Q17 and Q39 assistant
//! (AI Donut active, AI Carl passive: follow layer only), `companion_v0`
//! makes both roles active, and `cf02_free` (COMPANION-AI-02, CA2-Q01) is
//! `cf02_assist` with the `autonomy` block enabled and the A15 flank off. Knob names that carry a CF02 value are named by
//! their question ID (`cf02_q14_swat_range_px`); `cf02_q16_action_cadence_ticks` is
//! CF02-Q16. The file is validated and FNV-1a pinned like
//! `companion_magic_v0.json` ([`crate::donut_mana::content_hash`]); a load error
//! names the field. Canonical mana values (the AI reserve) are not repeated
//! here: the view carries `DonutMana::ai_can_cast`.

use serde::{Deserialize, Serialize};

use crate::donut_mana::content_hash;

/// The tuning file, embedded so the binary and the tests read the same bytes.
pub const PROFILE_JSON: &str = include_str!("../../assets/data/gameplay/companion_ai_v0.json");

/// The committed FNV-1a 64 content hash of [`PROFILE_JSON`]. Editing the file
/// without updating this value fails [`ProfileFile::pinned`] and the pin test.
pub const PROFILE_HASH: u64 = 0x21ec_f2a4_b285_9f85;

/// The profile set id under the spec's name (CA-21, section 17): the runtime's
/// `UncontrolledHero::Companion(CompanionProfileSet)` names it.
pub type CompanionProfileSet = ProfileSetId;

/// What an AI-started hit that would defeat does (CA-19, CA-20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KillPolicy {
    /// The default through 01D: an AI-started hit with damage >= target HP
    /// deals 0 (see [`crate::companion::rules::glance`]).
    #[default]
    Never,
    /// The hit is unchanged; credit stays factual either way. Supported as
    /// data now, tested as the candidate default in 01E (addendum A2).
    Factual,
}

/// Which profile set drives the companion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileSetId {
    Cf02Assist,
    CompanionV0,
    /// COMPANION-AI-02 (CA2-Q01): `cf02_assist` plus the autonomy layer.
    Cf02Free,
}

impl ProfileSetId {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cf02Assist => "cf02_assist",
            Self::CompanionV0 => "companion_v0",
            Self::Cf02Free => "cf02_free",
        }
    }
}

/// COMPANION-AI-02 (CA2-Q02, spec 5.3): which role goal table a hero uses when
/// it is the companion-driven partner. Role names are IP-neutral; the fan
/// concept names live only in the profile data and the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyRole {
    /// No autonomy table: the hero keeps today's follow layer.
    None,
    /// A small, proud, mobile hero who works the enemy the leader is not
    /// working: hit and run, spacing, perches (v1's only table).
    Skirmisher,
}

/// One goal of a role table: enable flag, base utility (CA2-Q18) and whether
/// the goal is allowed while a boss is up (CA2-Q35: only Retreat, Escort and
/// Idle, until COMPANION-AI-01E; the interrupt is the director's).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalRow {
    pub enabled: bool,
    pub utility: u16,
    pub under_boss: bool,
}

/// The goals the skirmisher table lists (CA2-Q18). Goal kinds are code; which
/// goals a role has and their utilities are data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleGoals {
    pub retreat: GoalRow,
    /// FA-Q04: punish a vulnerable boss (utility 1500, between Support-leader
    /// and Retreat). Read only while `fix_all.fa_q02_boss_play` is on.
    pub boss_punish: GoalRow,
    pub support_leader: GoalRow,
    pub hunt: GoalRow,
    pub escort: GoalRow,
    pub show_off: GoalRow,
    pub perch: GoalRow,
    pub idle: GoalRow,
}

impl RoleGoals {
    fn rows(&self) -> [(&'static str, &GoalRow); 8] {
        [
            ("retreat", &self.retreat),
            ("boss_punish", &self.boss_punish),
            ("support_leader", &self.support_leader),
            ("hunt", &self.hunt),
            ("escort", &self.escort),
            ("show_off", &self.show_off),
            ("perch", &self.perch),
            ("idle", &self.idle),
        ]
    }
}

/// COMPANION-AI-02: the `skirmisher` role goal table (spec 5.3 and section 6:
/// CA2-Q11, Q14, Q15, Q18 to Q24, Q31). Read by the goal selector (CA2-B1) and
/// later stages; CA2-A only loads and validates it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkirmisherTable {
    pub goals: RoleGoals,
    /// CA2-Q11: an enemy in the leader's claim set is never a Hunt candidate
    /// (a rule, not a penalty; review B-2).
    pub ca2_q11_exclude_claimed_from_hunt: bool,
    /// Hunt score base by enemy state (spec 4.2 table, CA2-Q14 family).
    pub ca2_q14_base_lying: u16,
    pub ca2_q14_base_standing: u16,
    pub ca2_q14_base_llama: u16,
    pub ca2_q14_base_engineer: u16,
    /// CA2-Q14: isolation term: `min(distance to the leader, cap) / divisor`.
    pub ca2_q14_isolation_cap_px: u16,
    pub ca2_q14_isolation_divisor: u16,
    /// Hunt travel term: `- distance from her / divisor`.
    pub ca2_q14_travel_divisor: u16,
    /// CA2-Q15: an enemy coming at the leader's back.
    pub ca2_q15_back_threat_bonus: u16,
    /// CA2-Q19: hit and run.
    pub ca2_q19_pounce_prefer_px: u16,
    pub ca2_q19_recoil_px: u16,
    pub ca2_q19_recoil_ticks: u16,
    /// CA2-Q20: casting spacing after the unlock.
    pub ca2_q20_cast_spacing_px: u16,
    pub ca2_q20_backoff_px: u16,
    /// CA2-Q21: Retreat (a hurt is the only trigger).
    pub ca2_q21_hurt_window_ticks: u16,
    pub ca2_q21_hurt_enemy_px: u16,
    pub ca2_q21_low_hp: u8,
    pub ca2_q21_low_enemy_px: u16,
    pub ca2_q21_exit_ticks: u16,
    pub ca2_q21_exit_clear_px: u16,
    pub ca2_q21_cooldown_ticks: u16,
    /// CA2-Q22: Retreat spot terms.
    pub ca2_q22_between_bonus: u16,
    pub ca2_q22_height_bonus_per_16px: u16,
    /// CA2-Q23: Perch.
    pub ca2_q23_quiet_ticks: u16,
    pub ca2_q23_perch_range_px: u16,
    pub ca2_q23_max_ticks: u16,
    pub ca2_q23_cooldown_ticks: u16,
    /// CA2-Q24: Show-off.
    pub ca2_q24_hold_ticks: u16,
    pub ca2_q24_safe_px: u16,
    pub ca2_q24_cooldown_ticks: u16,
    /// CA2-Q31: a llama is a Hunt target only after the unlock (she has no
    /// damaging move against it before).
    pub ca2_q31_llama_hunt_needs_unlock: bool,
    /// DONUT-PROG-01 DP-Q41 (DP-E): the AI short blink. A **recorded schema
    /// extension** of CA2-A's file: optional fields, absent = off. Only
    /// `cf02_free` sets them. `blink_enabled` lets the goal layer press her
    /// special as an escape (DP-Q39); `blink_threat_px` is how near a committed
    /// attack (or, inside Retreat, a standing enemy) must be; `blink_min_gain_px`
    /// the distance the destination must add from it.
    #[serde(default)]
    pub blink_enabled: bool,
    #[serde(default)]
    pub blink_threat_px: u16,
    #[serde(default)]
    pub blink_min_gain_px: u16,
}

/// The role tables of a set (v1: `skirmisher` only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutonomyRoles {
    pub skirmisher: SkirmisherTable,
}

/// COMPANION-AI-02 (spec 5.3, section 6): the autonomy layer's shared knobs
/// (CA2-Q03 to Q10, Q12, Q13, Q16, Q25, Q27, Q29, Q30, Q34) and its role tables
/// (Q35 is the per-goal `under_boss` flag). Q17 reuses
/// `ca10_decision_cadence_ticks`, Q26 is the existing off-screen regroup and Q28
/// is `cf02_duo_a15_flank`, so none of the three has a knob here. The
/// block is present in every set; with `enabled` false (`companion_v0`,
/// `cf02_assist`) nothing in it is read, hashed or applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Autonomy {
    pub enabled: bool,
    /// CA2-Q03: the zone stays this far inside the view edge.
    pub ca2_q03_edge_margin_px: u16,
    /// CA2-Q04: the roam cap around the leader (owner decision 2026-11-21).
    pub ca2_q04_leash_px: u16,
    /// CA2-Q06: free inside the comfort radius, a soft pull past it.
    pub ca2_q06_comfort_px: u16,
    pub ca2_q06_pull_per_px: u16,
    /// CA2-Q07: the leader "travels" after this many ticks of one direction;
    /// the escort bonus and spacing band.
    pub ca2_q07_travel_ticks: u16,
    pub ca2_q07_escort_bonus: u16,
    pub ca2_q07_escort_min_px: u16,
    pub ca2_q07_escort_max_px: u16,
    /// CA2-Q08: vertical band margins and the rise limit above the leader's
    /// feet.
    pub ca2_q08_view_top_margin_px: u16,
    pub ca2_q08_view_bottom_margin_px: u16,
    pub ca2_q08_max_rise_px: u16,
    /// CA2-Q09: the drift check's slack.
    pub ca2_q09_drift_slack_px: u16,
    /// CA2-Q10: the front claim radius of the leader's claim set.
    pub ca2_q10_claim_radius_px: u16,
    /// CA2-Q12: the leader is "low" at this health or less.
    pub ca2_q12_leader_low_hp: u8,
    /// CA2-Q13: the leader is "outnumbered" with this many enemies within the
    /// radius.
    pub ca2_q13_outnumbered_count: u8,
    pub ca2_q13_outnumbered_radius_px: u16,
    /// CA2-Q16: goal stickiness and minimum goal time.
    pub ca2_q16_stickiness: u16,
    pub ca2_q16_min_goal_ticks: u16,
    /// CA2-Q25: the automatic camera's bound pad for a bound-only subject.
    pub ca2_q25_bound_pad_px: u16,
    /// CA2-Q27: the follow bot's ledge hop is not used under autonomy (she does
    /// not copy the leader's jumps; real routes come from the nav graph).
    pub ca2_q27_partner_ledge_hop: bool,
    /// CA2-Q29: spot safety.
    pub ca2_q29_spot_max_risk: u16,
    pub ca2_q29_ledge_inset_px: u16,
    pub ca2_q29_lane_penalty: u16,
    /// CA2-Q30: re-pick a spot only when the leader moved this far.
    pub ca2_q30_respot_px: u16,
    /// CA2-Q34: the M5 involuntary-cause window.
    pub ca2_q34_cause_window_ticks: u16,
    /// CA2-Q36: the drift check prices a transit from her position (the walk to
    /// the launch point, the simulated air time, the walk from the landing to
    /// the spot), not centre to centre; this many ticks are added per jump or
    /// drop edge for the approach and landing settle the simulation does not
    /// price. A zero is valid (the exact simulated price).
    pub ca2_q36_transit_slack_ticks: u16,
    pub roles: AutonomyRoles,
}

impl Autonomy {
    /// The role table for `role`; `None` for [`AutonomyRole::None`].
    pub const fn table(&self, role: AutonomyRole) -> Option<&SkirmisherTable> {
        match role {
            AutonomyRole::None => None,
            AutonomyRole::Skirmisher => Some(&self.roles.skirmisher),
        }
    }
}

/// AI Carl's knobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarlRole {
    /// COMPANION-AI-02 (CA2-Q02): the autonomy role table this hero uses while
    /// it is the companion-driven partner. `none` for AI Carl until his own
    /// work order.
    pub autonomy_role: AutonomyRole,
    /// `false`: AI Carl only follows (the CF02 Q13 passive Carl).
    pub ca21_active: bool,
    /// Centre distance to a target at which the jab can connect (CA-41, a
    /// planning window inside the move boxes widened by `ENEMY_HALF_WIDTH`).
    pub ca41_strike_min_px: u16,
    pub ca41_strike_max_px: u16,
    /// CA-40: a target farther than this (CF02-Q14's 96 px) is not engaged.
    pub ca40_engage_range_px: u16,
    /// A tell aimed at the human hero is interrupted within this distance.
    pub ca40_interrupt_range_px: u16,
}

/// AI Donut's knobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DonutRole {
    /// COMPANION-AI-02 (CA2-Q02): `skirmisher` for Donut (inert while the set's
    /// `autonomy.enabled` is false).
    pub autonomy_role: AutonomyRole,
    pub ca21_active: bool,
    /// Q14: a tell aimed at the controlled hero is answered within this range.
    pub cf02_q14_swat_range_px: u16,
    /// Q14 (Q39): the earliest tell tick an assist swat may start from.
    pub cf02_q14_swat_tell_tick: u16,
    /// Centre distance at which the swat box connects (CA-41, same rule).
    pub ca41_swat_hit_min_px: u16,
    pub ca41_swat_hit_max_px: u16,
    /// Q15: pounce range ahead of Donut, on a lying or staggered goblin.
    pub cf02_q15_pounce_min_px: u16,
    pub cf02_q15_pounce_max_px: u16,
    /// CF02-Q38 (hold): a pounce on a lying goblin starts only while the goblin
    /// still lies for at least this many ticks, about the pounce's startup plus
    /// flight, so it lands before the get-up (a late one would stand the goblin
    /// up, the outcome Q34 and Q38 prevent).
    pub cf02_q38_pounce_lying_min_ticks_left: u16,
    /// Q17: Magic Missile range, after the unlock only.
    pub cf02_q17_missile_min_px: u16,
    pub cf02_q17_missile_max_px: u16,
}

/// CF02-DUO (owner playtest feedback 2026-11-21): AI Donut as a fighting
/// partner. With `cf02_duo_a04_offense` off (the `companion_v0` set) none of the
/// offensive candidates exists, so the director is exactly the earlier one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DuoOffense {
    /// CF02-DUO-A04: AI Donut also starts a pounce or a Magic Missile on her
    /// own at a standing or staggered enemy, not only as an interrupt or on a
    /// lying goblin. (The offensive swat of the first cut was removed by
    /// CF02-DUO-A07: it dealt no damage and set the guard that blocks her own
    /// interrupt.)
    pub cf02_duo_a04_offense: bool,
    /// CF02-DUO-A05: the band the offensive Magic Missile starts from (after
    /// the unlock, above the AI mana reserve, never a free cast).
    pub cf02_duo_a05_missile_min_px: u16,
    pub cf02_duo_a05_missile_max_px: u16,
    /// CF02-DUO-A06: the band the offensive pounce starts from on a standing
    /// goblin (the lying/staggered pounce keeps the Q15 band).
    pub cf02_duo_a06_pounce_min_px: u16,
    pub cf02_duo_a06_pounce_max_px: u16,
    /// CF02-DUO-A08 (spec section 8, `HumanFinishWindow`): the AI Donut never
    /// starts a damaging move that would take the last hit on an enemy the
    /// human-controlled Carl is engaging. Needs `ca19_kill_policy = factual`
    /// to matter (under `never` no AI hit kills).
    pub cf02_duo_a08_respect_finisher: bool,
    /// CF02-DUO-A08: Carl engages an enemy within this many px of him while he
    /// has a move running or the enemy is down from his hit: the jab strike
    /// band (30), the knockback drift a jab leaves (6, measured 430 to 436 in
    /// `cf02_assist_pounce_staggered`) and the haymaker's 4 px step, so the
    /// window covers jab to haymaker.
    pub cf02_duo_a08_finisher_reach_px: u16,
    /// CF02-DUO-A08: hit points kept for Carl's pending hit: an engaged enemy
    /// whose HP is at most the AI hit's damage plus this is not hit by her.
    pub cf02_duo_a08_finisher_reserve_hp: u8,
    /// CF02-DUO-A09: the offensive pounce and missile start only at an enemy
    /// whose feet are within this many px of her own (no swat in the air over
    /// a goblin on another level).
    pub cf02_duo_a09_vertical_reach_px: u16,
    /// CF02-DUO-A11: Carl engages only the enemies on the side he faces; an
    /// enemy behind him is fair game for AI Donut (she guards his back).
    pub cf02_duo_a11_engage_facing_only: bool,
    /// CF02-DUO-A12: an enemy Carl hit within this many ticks stays his (the
    /// walk-up between two jabs), on the side he faces, whatever its distance
    /// from him. 0: only a running move or a stagger counts.
    pub cf02_duo_a12_recent_hit_ticks: u16,
    /// CF02-DUO-A13: the offensive pounce and Magic Missile also choose an
    /// enemy she is not facing; the director turns her to it (the existing
    /// turn step) and presses on the next tick. Without it she only attacks
    /// what is in front of her, which a trailing partner who walks toward
    /// Carl almost never has (measured: 1 offensive start in the arena run).
    pub cf02_duo_a13_turn_to_target: bool,
    /// CF02-DUO-A15 (adopted default): while a standing goblin approaches on
    /// the side the human Carl faces, AI Donut idles on a flank spot ahead of
    /// him instead of trailing, so the goblin reaches her pounce band before
    /// it reaches him. Anti-steal (A08, A11, A12) and the dodge are unchanged.
    pub cf02_duo_a15_flank: bool,
    /// CF02-DUO-A15: the flank spot is this many px ahead of Carl, on the
    /// side he faces (50 to 60 px: inside her 30..80 px pounce band of a goblin
    /// that is still outside his 40 px reach).
    pub cf02_duo_a15_flank_ahead_px: u16,
    /// CF02-DUO-A15: a goblin triggers the flank while it is at most this far
    /// ahead of Carl, and at least `flank_ahead + a06_pounce_min` (she never
    /// walks through it to reach the spot).
    pub cf02_duo_a15_flank_goblin_range_px: u16,
    /// CF02-DUO-A15: the spot stays at least this many px inside the
    /// simulation camera (never pushed to the edge or off-screen).
    pub cf02_duo_a15_flank_view_margin_px: u16,
    /// CF02-DUO-A15: once the goblin is closer to Carl than the flank spot
    /// plus her pounce minimum, it would walk through her and leave her too
    /// close to pounce it again; she falls back to this many px behind Carl
    /// (the follow bot's own distance), where it arrives inside her band.
    pub cf02_duo_a15_fallback_behind_px: u16,
}

/// The fix-all knobs (PR-A of the Floor 1 fix-all plan). Every behaviour change
/// ships behind a flag whose shipped value is the new behaviour; a set without
/// the `fix_all` block (or with a flag false) keeps the pre-fix behaviour
/// byte for byte (the knob-off hash identity). Every number cites its FA-Q id.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FixAll {
    /// FA-Q02..Q06: the boss as a target and a threat. Feeds `BossView`,
    /// counts the boss in `standing_within`, gives Retreat and Escort their
    /// boss variants, adds the BossPunish goal and makes a lethal AI hit on the
    /// boss a glance (the kill is Carl's, FA-Q06).
    pub fa_q02_boss_play: bool,
    /// FA-Q03: a retreat spot under a boss is at least this far from the boss.
    pub fa_q03_retreat_boss_clear_px: u16,
    /// FA-Q03 (FA-Q07 tuning iteration 1, recorded in the PR): Retreat also
    /// triggers on a boss tell or committed move within the hurt radius, not only
    /// on a hurt.
    pub fa_q03_retreat_on_boss_tell: bool,
    /// FA-Q04: BossPunish needs this many vulnerable ticks left to start.
    pub fa_q04_punish_min_ticks: u16,
    /// FA-Q05: the boss escort (Standoff) distance band from the boss.
    pub fa_q05_escort_boss_min_px: u16,
    pub fa_q05_escort_boss_max_px: u16,
    /// FA-Q05 deviation (recorded in the PR): from phase 2 the standoff also
    /// clears the shockwave ripples' reach by this margin, because the ripples
    /// hurt a grounded hero out to 176 px either side of the boss.
    pub fa_q05_ripple_margin_px: u16,
    /// FA-Q05 deviation (recorded in the PR): a grounded partner hops a shockwave
    /// ripple that is this many px from her and coming at her (the follow layer
    /// has no ripple dodge, and in a corner the standoff cannot clear the reach).
    pub fa_q05_ripple_hop_px: u16,
    /// FA-Q09: the boss-charge blink window opens at the follow layer's jump lead
    /// (`ai_charge_jump_lead`, 18) plus `fa_q09_window_extra_ticks`, and a blink
    /// press for a charge silences the follow layer's dodge jump for that charge.
    pub fa_q09_blink_charge_window: bool,
    pub fa_q09_window_extra_ticks: u16,
    /// FA-Q10: a goblin's swing triggers the blink only if her hurtbox overlaps
    /// its hitbox widened by `fa_q10_reach_margin_px`.
    pub fa_q10_blink_reach_gate: bool,
    pub fa_q10_reach_margin_px: u16,
    /// FA-Q15: the follow layer of a driven partner without the goal layer
    /// (`cf02_assist`) reads the hazard feed and its ground steps are fenced
    /// from hazard rectangles, the same feed and fence `cf02_free` uses.
    pub fa_q15_assist_rot_avoidance: bool,
    /// FA-Q16: the AI never releases a missile, swat or pounce whose hitbox path
    /// crosses a live setpiece prop's hit target (valve, brace): no AI profile has
    /// a prop as its goal.
    pub fa_q16_prop_safe_fire: bool,
    /// FA-Q25 (M2 shadowing): when both heroes travel the same way within
    /// `fa_q25_shadow_px` for more than `fa_q25_shadow_ticks` ticks, she re-picks
    /// her escort spot at least `fa_q25_respot_px` from the leader.
    pub fa_q25_shadow_respot: bool,
    pub fa_q25_shadow_px: u16,
    pub fa_q25_shadow_ticks: u16,
    pub fa_q25_respot_px: u16,
    /// FA-Q26 (M4 steals): a lethal AI hit on an enemy the leader hit within the
    /// A12 window (`cf02_duo_a12_recent_hit_ticks`), or that is within the A08
    /// finisher reach of the leader, is a glance.
    pub fa_q26_steal_glance: bool,
    /// FA-Q27 (M10 retreat distance): the retreat spot is at least this far from
    /// the nearest standing enemy; with none, the highest anchor.
    pub fa_q27_retreat_clear: bool,
    pub fa_q27_retreat_clear_px: u16,
    /// FA-Q28 (M13 her hits): the Escort travel bonus drops to
    /// `fa_q28_escort_bonus` while an unclaimed Hunt candidate is within
    /// `fa_q28_hunt_near_px` of her.
    pub fa_q28_hunt_escort_bonus: bool,
    pub fa_q28_escort_bonus: u16,
    pub fa_q28_hunt_near_px: u16,
    /// FA-Q29 (CF02-FU-B): the AI pounces only if its predicted landing comes
    /// `fa_q29_slack_ticks` or more before the target's next impact tick.
    pub fa_q29_pounce_timing: bool,
    pub fa_q29_slack_ticks: u16,
}

/// One profile set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileSet {
    /// No serde default: a missing field is a parse error that names it.
    pub ca19_kill_policy: KillPolicy,
    /// CA-10: the utility planning cadence.
    pub ca10_decision_cadence_ticks: u16,
    /// CF02-Q16: at most one AI-started move per this many ticks.
    pub cf02_q16_action_cadence_ticks: u16,
    /// CF02-Q55 / CA-26: the AI never bails out (v0 requires `false`).
    pub ca26_ai_bailout: bool,
    /// Q39 acceptance floor, read by the fixtures: assist starts.
    pub cf02_q39_min_assist_starts: u8,
    /// Q39 acceptance ceiling: the AI's share of the hits, in percent.
    pub cf02_q39_max_hit_share_percent: u8,
    pub carl: CarlRole,
    pub donut: DonutRole,
    /// CF02-DUO: AI Donut's offense (off in `companion_v0`).
    pub cf02_duo: DuoOffense,
    /// COMPANION-AI-02: the autonomy layer (`enabled` only in `cf02_free`).
    pub autonomy: Autonomy,
    /// The fix-all knobs (absent: every flag off).
    #[serde(default)]
    pub fix_all: FixAll,
}

/// The whole file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileFile {
    pub schema_version: u32,
    pub cf02_assist: ProfileSet,
    pub companion_v0: ProfileSet,
    pub cf02_free: ProfileSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    /// The JSON did not parse or has an unknown or missing field (serde names
    /// it in the message).
    Parse(String),
    /// A field failed validation; `field` is its dotted path.
    Field { field: String, reason: String },
    /// The file's content hash is not the pinned one.
    HashMismatch { expected: u64, actual: u64 },
}

impl core::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "companion ai profile: {msg}"),
            Self::Field { field, reason } => {
                write!(f, "companion ai profile: field `{field}`: {reason}")
            }
            Self::HashMismatch { expected, actual } => write!(
                f,
                "companion ai profile hash {actual:#018x} is not the pinned {expected:#018x}"
            ),
        }
    }
}

impl std::error::Error for ProfileError {}

fn bad(field: &str, reason: &str) -> Result<(), ProfileError> {
    Err(ProfileError::Field {
        field: field.to_string(),
        reason: reason.to_string(),
    })
}

impl ProfileFile {
    /// Parse and validate.
    pub fn from_json(text: &str) -> Result<Self, ProfileError> {
        let file: Self =
            serde_json::from_str(text).map_err(|err| ProfileError::Parse(err.to_string()))?;
        file.validate()?;
        Ok(file)
    }

    /// Parse the embedded file and require the pinned content hash.
    pub fn pinned() -> Result<Self, ProfileError> {
        Self::from_json_pinned(PROFILE_JSON, PROFILE_HASH)
    }

    /// Parse `text` and require its content hash to equal `expected` (a replay
    /// header or fixture records the hash it ran with).
    pub fn from_json_pinned(text: &str, expected: u64) -> Result<Self, ProfileError> {
        let actual = content_hash(text);
        if actual != expected {
            return Err(ProfileError::HashMismatch { expected, actual });
        }
        Self::from_json(text)
    }

    pub fn set(&self, id: ProfileSetId) -> &ProfileSet {
        match id {
            ProfileSetId::Cf02Assist => &self.cf02_assist,
            ProfileSetId::CompanionV0 => &self.companion_v0,
            ProfileSetId::Cf02Free => &self.cf02_free,
        }
    }

    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.schema_version != 1 {
            return bad("schema_version", "must be 1");
        }
        self.cf02_assist.validate("cf02_assist")?;
        self.companion_v0.validate("companion_v0")?;
        self.cf02_free.validate("cf02_free")?;
        // CF02-Q38 (the AI never defeats) is superseded by CF02-DUO-A01 (owner
        // playtest feedback 2026-11-21): both policies are valid data for
        // every set.
        // CA-21: cf02_assist is Q13's assistant (AI Carl passive); v0 is both.
        if self.cf02_assist.carl.ca21_active {
            return bad(
                "cf02_assist.carl.ca21_active",
                "CF02-Q13: AI Carl stays passive",
            );
        }
        if !self.cf02_assist.donut.ca21_active {
            return bad(
                "cf02_assist.donut.ca21_active",
                "CF02-Q13: AI Donut is the assistant",
            );
        }
        // CA2-Q01: cf02_free is the assistant set (AI Carl passive, AI Donut
        // active) with the autonomy layer on.
        if self.cf02_free.carl.ca21_active {
            return bad(
                "cf02_free.carl.ca21_active",
                "CF02-Q13: AI Carl stays passive",
            );
        }
        if !self.cf02_free.donut.ca21_active {
            return bad(
                "cf02_free.donut.ca21_active",
                "CF02-Q13: AI Donut is the assistant",
            );
        }
        if !self.companion_v0.carl.ca21_active {
            return bad(
                "companion_v0.carl.ca21_active",
                "companion_v0 makes both roles active",
            );
        }
        if !self.companion_v0.donut.ca21_active {
            return bad(
                "companion_v0.donut.ca21_active",
                "companion_v0 makes both roles active",
            );
        }
        Ok(())
    }
}

impl ProfileSet {
    fn validate(&self, set: &str) -> Result<(), ProfileError> {
        let f = |name: &str| format!("{set}.{name}");
        if self.ca10_decision_cadence_ticks == 0 {
            return bad(&f("ca10_decision_cadence_ticks"), "must be positive");
        }
        if self.cf02_q16_action_cadence_ticks == 0 {
            return bad(&f("cf02_q16_action_cadence_ticks"), "must be positive");
        }
        if self.ca26_ai_bailout {
            return bad(
                &f("ca26_ai_bailout"),
                "CF02-Q55: the AI never bails out in v0",
            );
        }
        if self.cf02_q39_min_assist_starts == 0 {
            return bad(&f("cf02_q39_min_assist_starts"), "must be positive");
        }
        if self.cf02_q39_max_hit_share_percent == 0 || self.cf02_q39_max_hit_share_percent > 100 {
            return bad(&f("cf02_q39_max_hit_share_percent"), "must be in 1..=100");
        }
        let carl = &self.carl;
        if carl.ca41_strike_min_px >= carl.ca41_strike_max_px {
            return bad(
                &f("carl.ca41_strike_min_px"),
                "must be below ca41_strike_max_px",
            );
        }
        if carl.ca40_engage_range_px == 0 {
            return bad(&f("carl.ca40_engage_range_px"), "must be positive");
        }
        if carl.ca40_interrupt_range_px == 0 {
            return bad(&f("carl.ca40_interrupt_range_px"), "must be positive");
        }
        let duo = &self.cf02_duo;
        if duo.cf02_duo_a05_missile_min_px >= duo.cf02_duo_a05_missile_max_px {
            return bad(
                &f("cf02_duo.cf02_duo_a05_missile_min_px"),
                "must be below cf02_duo_a05_missile_max_px",
            );
        }
        if duo.cf02_duo_a08_respect_finisher && duo.cf02_duo_a08_finisher_reach_px == 0 {
            return bad(
                &f("cf02_duo.cf02_duo_a08_finisher_reach_px"),
                "must be positive while cf02_duo_a08_respect_finisher is on",
            );
        }
        if duo.cf02_duo_a09_vertical_reach_px == 0 {
            return bad(
                &f("cf02_duo.cf02_duo_a09_vertical_reach_px"),
                "must be positive",
            );
        }
        if duo.cf02_duo_a06_pounce_min_px >= duo.cf02_duo_a06_pounce_max_px {
            return bad(
                &f("cf02_duo.cf02_duo_a06_pounce_min_px"),
                "must be below cf02_duo_a06_pounce_max_px",
            );
        }
        if duo.cf02_duo_a15_flank
            && (duo.cf02_duo_a15_flank_ahead_px == 0
                || duo.cf02_duo_a15_flank_goblin_range_px
                    <= duo.cf02_duo_a15_flank_ahead_px + duo.cf02_duo_a06_pounce_min_px)
        {
            return bad(
                &f("cf02_duo.cf02_duo_a15_flank_goblin_range_px"),
                "must exceed flank_ahead_px + a06_pounce_min_px while cf02_duo_a15_flank is on",
            );
        }
        let d = &self.donut;
        if d.cf02_q14_swat_range_px == 0 {
            return bad(&f("donut.cf02_q14_swat_range_px"), "must be positive");
        }
        if d.ca41_swat_hit_min_px >= d.ca41_swat_hit_max_px {
            return bad(
                &f("donut.ca41_swat_hit_min_px"),
                "must be below ca41_swat_hit_max_px",
            );
        }
        if d.cf02_q15_pounce_min_px >= d.cf02_q15_pounce_max_px {
            return bad(
                &f("donut.cf02_q15_pounce_min_px"),
                "must be below cf02_q15_pounce_max_px",
            );
        }
        if d.cf02_q17_missile_min_px >= d.cf02_q17_missile_max_px {
            return bad(
                &f("donut.cf02_q17_missile_min_px"),
                "must be below cf02_q17_missile_max_px",
            );
        }
        self.validate_autonomy(set)
    }

    /// COMPANION-AI-02: the shared autonomy knobs and the role table. The
    /// ranges are the ones the zone, drift and goal code divide or compare by;
    /// the adopted values are pinned by the tests, not here.
    fn validate_autonomy(&self, set: &str) -> Result<(), ProfileError> {
        // A goal utility stays below the director's interrupt (2000, CA2-Q18).
        const INTERRUPT_UTILITY: u16 = 2000;
        // 304 (the settled leading edge) - 32 (CA2-Q03) - 12 (her half width).
        const MAX_LEASH_PX: u16 = 260;
        let a = &self.autonomy;
        let f = |name: &str| format!("{set}.autonomy.{name}");
        let positive = |name: &str, value: u16| {
            if value == 0 {
                bad(&f(name), "must be positive")
            } else {
                Ok(())
            }
        };
        positive("ca2_q04_leash_px", a.ca2_q04_leash_px)?;
        // On the leading side the settled view edge is 304 px ahead of the
        // leader; a leash above 260 would eat the 32 px edge margin there.
        if a.ca2_q04_leash_px > MAX_LEASH_PX {
            return bad(
                &f("ca2_q04_leash_px"),
                "must not exceed 260 px (the leading-side camera bound)",
            );
        }
        positive("ca2_q06_pull_per_px", a.ca2_q06_pull_per_px)?;
        positive("ca2_q07_travel_ticks", a.ca2_q07_travel_ticks)?;
        positive("ca2_q08_max_rise_px", a.ca2_q08_max_rise_px)?;
        positive("ca2_q10_claim_radius_px", a.ca2_q10_claim_radius_px)?;
        positive(
            "ca2_q13_outnumbered_radius_px",
            a.ca2_q13_outnumbered_radius_px,
        )?;
        positive("ca2_q16_min_goal_ticks", a.ca2_q16_min_goal_ticks)?;
        positive("ca2_q25_bound_pad_px", a.ca2_q25_bound_pad_px)?;
        positive("ca2_q30_respot_px", a.ca2_q30_respot_px)?;
        positive("ca2_q34_cause_window_ticks", a.ca2_q34_cause_window_ticks)?;
        if a.ca2_q13_outnumbered_count == 0 {
            return bad(&f("ca2_q13_outnumbered_count"), "must be positive");
        }
        if a.ca2_q06_comfort_px > a.ca2_q04_leash_px {
            return bad(&f("ca2_q06_comfort_px"), "must not exceed ca2_q04_leash_px");
        }
        if a.ca2_q07_escort_min_px >= a.ca2_q07_escort_max_px {
            return bad(
                &f("ca2_q07_escort_min_px"),
                "must be below ca2_q07_escort_max_px",
            );
        }
        if self.fix_all.fa_q02_boss_play {
            let fa = &self.fix_all;
            for (name, value) in [
                (
                    "fa_q03_retreat_boss_clear_px",
                    fa.fa_q03_retreat_boss_clear_px,
                ),
                ("fa_q04_punish_min_ticks", fa.fa_q04_punish_min_ticks),
                ("fa_q05_escort_boss_min_px", fa.fa_q05_escort_boss_min_px),
                ("fa_q05_escort_boss_max_px", fa.fa_q05_escort_boss_max_px),
                ("fa_q05_ripple_hop_px", fa.fa_q05_ripple_hop_px),
            ] {
                if value == 0 {
                    return bad(
                        &format!("{set}.fix_all.{name}"),
                        "must be positive while fa_q02_boss_play",
                    );
                }
            }
            if fa.fa_q05_escort_boss_min_px >= fa.fa_q05_escort_boss_max_px {
                return bad(
                    &format!("{set}.fix_all.fa_q05_escort_boss_min_px"),
                    "must be below fa_q05_escort_boss_max_px",
                );
            }
        }
        for (on, name, value) in [
            (
                self.fix_all.fa_q25_shadow_respot,
                "fa_q25_shadow_px",
                self.fix_all.fa_q25_shadow_px,
            ),
            (
                self.fix_all.fa_q25_shadow_respot,
                "fa_q25_shadow_ticks",
                self.fix_all.fa_q25_shadow_ticks,
            ),
            (
                self.fix_all.fa_q25_shadow_respot,
                "fa_q25_respot_px",
                self.fix_all.fa_q25_respot_px,
            ),
            (
                self.fix_all.fa_q27_retreat_clear,
                "fa_q27_retreat_clear_px",
                self.fix_all.fa_q27_retreat_clear_px,
            ),
            (
                self.fix_all.fa_q28_hunt_escort_bonus,
                "fa_q28_hunt_near_px",
                self.fix_all.fa_q28_hunt_near_px,
            ),
            (
                self.fix_all.fa_q29_pounce_timing,
                "fa_q29_slack_ticks",
                self.fix_all.fa_q29_slack_ticks,
            ),
            (
                self.fix_all.fa_q09_blink_charge_window,
                "fa_q09_window_extra_ticks",
                self.fix_all.fa_q09_window_extra_ticks,
            ),
            (
                self.fix_all.fa_q10_blink_reach_gate,
                "fa_q10_reach_margin_px",
                self.fix_all.fa_q10_reach_margin_px,
            ),
        ] {
            if on && value == 0 {
                return bad(
                    &format!("{set}.fix_all.{name}"),
                    "must be positive while its flag is on",
                );
            }
        }
        let sk = &a.roles.skirmisher;
        let g = |name: &str| format!("{set}.autonomy.roles.skirmisher.{name}");
        for (name, row) in sk.goals.rows() {
            if row.utility >= INTERRUPT_UTILITY {
                return bad(
                    &g(&format!("goals.{name}.utility")),
                    "must stay below the interrupt utility (2000)",
                );
            }
        }
        for (name, value) in [
            ("ca2_q14_isolation_divisor", sk.ca2_q14_isolation_divisor),
            ("ca2_q14_travel_divisor", sk.ca2_q14_travel_divisor),
            ("ca2_q19_recoil_ticks", sk.ca2_q19_recoil_ticks),
            ("ca2_q21_hurt_window_ticks", sk.ca2_q21_hurt_window_ticks),
            ("ca2_q21_exit_ticks", sk.ca2_q21_exit_ticks),
            ("ca2_q23_max_ticks", sk.ca2_q23_max_ticks),
            ("ca2_q24_hold_ticks", sk.ca2_q24_hold_ticks),
        ] {
            if value == 0 {
                return bad(&g(name), "must be positive");
            }
        }
        if sk.blink_enabled {
            // DP-Q41: an enabled blink needs its two knobs, and only the goal layer
            // presses it (DP-Q40: never for a set without autonomy).
            if !a.enabled {
                return bad(
                    &g("blink_enabled"),
                    "the AI blink needs an enabled autonomy layer (DP-Q40)",
                );
            }
            for (name, value) in [
                ("blink_threat_px", sk.blink_threat_px),
                ("blink_min_gain_px", sk.blink_min_gain_px),
            ] {
                if value == 0 {
                    return bad(&g(name), "must be positive while blink_enabled");
                }
            }
        }
        if sk.ca2_q20_backoff_px >= sk.ca2_q20_cast_spacing_px {
            return bad(
                &g("ca2_q20_backoff_px"),
                "must be below ca2_q20_cast_spacing_px",
            );
        }
        if !a.enabled {
            return Ok(());
        }
        // An enabled layer needs what its zone and claim rules read: the
        // partner has a role table, the leader (Carl in v1) has none, because
        // the claim set reads `carl_hit` facts that only describe Carl's hits
        // (an AI Carl work order adds per-hero facts first), and the kill-steal
        // veto rules (A08, A11, A12) the claim set is built from, under the
        // `factual` policy.
        if self.donut.autonomy_role == AutonomyRole::None {
            return bad(
                &format!("{set}.donut.autonomy_role"),
                "an enabled autonomy layer needs a role table for the partner",
            );
        }
        if self.carl.autonomy_role != AutonomyRole::None {
            return bad(
                &format!("{set}.carl.autonomy_role"),
                "v1 ships the partner's table only: AI Carl's role must be none",
            );
        }
        let duo = &self.cf02_duo;
        if self.ca19_kill_policy != KillPolicy::Factual {
            return bad(
                &format!("{set}.ca19_kill_policy"),
                "an enabled autonomy layer needs the factual kill policy",
            );
        }
        if !duo.cf02_duo_a08_respect_finisher {
            return bad(
                &format!("{set}.cf02_duo.cf02_duo_a08_respect_finisher"),
                "an enabled autonomy layer needs A08 on (the claim set)",
            );
        }
        if !duo.cf02_duo_a11_engage_facing_only {
            return bad(
                &format!("{set}.cf02_duo.cf02_duo_a11_engage_facing_only"),
                "an enabled autonomy layer needs A11 on (the claim set)",
            );
        }
        if duo.cf02_duo_a12_recent_hit_ticks == 0 {
            return bad(
                &format!("{set}.cf02_duo.cf02_duo_a12_recent_hit_ticks"),
                "an enabled autonomy layer needs a positive A12 window (the claim set)",
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> ProfileFile {
        ProfileFile::pinned().expect("the embedded profile file is valid and pinned")
    }

    #[test]
    fn profile_hash_is_pinned() {
        // Fails when companion_ai_v0.json changes without updating PROFILE_HASH.
        assert_eq!(content_hash(PROFILE_JSON), PROFILE_HASH);
        assert!(ProfileFile::pinned().is_ok());
    }

    #[test]
    fn a_drifted_file_is_a_clear_hash_failure() {
        let drifted = PROFILE_JSON.replacen(
            "\"cf02_q16_action_cadence_ticks\": 60",
            "\"cf02_q16_action_cadence_ticks\": 61",
            1,
        );
        assert_ne!(drifted, PROFILE_JSON);
        match ProfileFile::from_json_pinned(&drifted, PROFILE_HASH) {
            Err(ProfileError::HashMismatch { expected, actual }) => {
                assert_eq!(expected, PROFILE_HASH);
                assert_ne!(actual, expected);
            }
            other => panic!("expected a hash mismatch, got {other:?}"),
        }
        // Unpinned it still parses: the pin is what catches the edit.
        assert!(ProfileFile::from_json(&drifted).is_ok());
    }

    #[test]
    fn the_file_round_trips() {
        let a = file();
        let text = serde_json::to_string(&a).unwrap();
        let b = ProfileFile::from_json(&text).unwrap();
        assert_eq!(a, b);
        assert_eq!(serde_json::to_string(&b).unwrap(), text);
    }

    #[test]
    fn cf02_assist_carries_the_adopted_q13_to_q17_and_q39_values() {
        let f = file();
        let s = &f.cf02_assist;
        // CF02-DUO-A01 (was never), A02 (was 60), A03 (was 25).
        assert_eq!(s.ca19_kill_policy, KillPolicy::Factual);
        assert_eq!(
            (
                s.ca10_decision_cadence_ticks,
                s.cf02_q16_action_cadence_ticks
            ),
            (6, 20)
        );
        // CF02-DUO-A04..A06, A08, A09: AI Donut's offense.
        assert!(s.cf02_duo.cf02_duo_a04_offense);
        assert!(s.cf02_duo.cf02_duo_a08_respect_finisher);
        assert_eq!(
            (
                s.cf02_duo.cf02_duo_a08_finisher_reach_px,
                s.cf02_duo.cf02_duo_a08_finisher_reserve_hp,
                s.cf02_duo.cf02_duo_a09_vertical_reach_px
            ),
            (40, 1, 24)
        );
        // CF02-DUO-A11..A13: facing-scoped engagement, the recently-hit window
        // (31 ticks: hit at 49 to Carl's press at 80 in the walk-up probe), the
        // turn to an enemy behind her. All off in `companion_v0`.
        assert!(s.cf02_duo.cf02_duo_a11_engage_facing_only);
        assert_eq!(s.cf02_duo.cf02_duo_a12_recent_hit_ticks, 31);
        assert!(s.cf02_duo.cf02_duo_a13_turn_to_target);
        let v0 = &f.companion_v0.cf02_duo;
        assert!(!v0.cf02_duo_a11_engage_facing_only && !v0.cf02_duo_a13_turn_to_target);
        assert_eq!(v0.cf02_duo_a12_recent_hit_ticks, 0);
        assert_eq!(
            (
                s.cf02_duo.cf02_duo_a05_missile_min_px,
                s.cf02_duo.cf02_duo_a05_missile_max_px
            ),
            (64, 160)
        );
        assert_eq!(
            (
                s.cf02_duo.cf02_duo_a06_pounce_min_px,
                s.cf02_duo.cf02_duo_a06_pounce_max_px
            ),
            (30, 80)
        );
        assert!(!s.ca26_ai_bailout);
        assert!(!s.carl.ca21_active, "Q13: AI Carl is passive");
        assert_eq!(
            (s.carl.ca40_engage_range_px, s.carl.ca40_interrupt_range_px),
            (96, 96)
        );
        let d = &s.donut;
        assert!(d.ca21_active);
        assert_eq!(d.cf02_q14_swat_range_px, 96);
        assert_eq!(d.cf02_q14_swat_tell_tick, 4);
        assert_eq!(
            (d.cf02_q15_pounce_min_px, d.cf02_q15_pounce_max_px),
            (30, 80)
        );
        assert_eq!(
            (d.cf02_q17_missile_min_px, d.cf02_q17_missile_max_px),
            (97, 160)
        );
        assert_eq!(
            (
                s.cf02_q39_min_assist_starts,
                s.cf02_q39_max_hit_share_percent
            ),
            (2, 50)
        );
    }

    #[test]
    fn companion_v0_makes_both_roles_active_with_the_same_cadences() {
        let f = file();
        let v0 = f.set(ProfileSetId::CompanionV0);
        assert!(v0.carl.ca21_active && v0.donut.ca21_active);
        assert_eq!(v0.ca19_kill_policy, KillPolicy::Never);
        assert_eq!(
            (
                v0.ca10_decision_cadence_ticks,
                v0.cf02_q16_action_cadence_ticks
            ),
            (6, 60)
        );
        assert!(!v0.ca26_ai_bailout);
        assert!(
            !v0.cf02_duo.cf02_duo_a04_offense,
            "v0 has no CF02-DUO offense"
        );
        assert_eq!(v0.donut, f.cf02_assist.donut, "Donut's numbers are shared");
        assert_eq!(ProfileSetId::Cf02Assist.name(), "cf02_assist");
        assert_eq!(ProfileSetId::CompanionV0.name(), "companion_v0");
    }

    #[test]
    fn kill_policy_is_required_and_either_policy_is_data_for_every_set() {
        assert_eq!(KillPolicy::default(), KillPolicy::Never);
        // CF02-DUO-A01: cf02_assist is `factual`, companion_v0 stays `never`.
        let f = file();
        assert_eq!(f.cf02_assist.ca19_kill_policy, KillPolicy::Factual);
        assert_eq!(f.companion_v0.ca19_kill_policy, KillPolicy::Never);
        // Either set accepts either policy (CF02-Q38's never is superseded).
        let never = PROFILE_JSON.replacen("\"factual\"", "\"never\"", 1);
        let f = ProfileFile::from_json(&never).unwrap();
        assert_eq!(f.cf02_assist.ca19_kill_policy, KillPolicy::Never);
        let factual = PROFILE_JSON.replacen("\"never\"", "\"factual\"", 1);
        let f = ProfileFile::from_json(&factual).unwrap();
        assert_eq!(f.companion_v0.ca19_kill_policy, KillPolicy::Factual);
        // A missing field is a parse error that names it.
        let missing = PROFILE_JSON.replacen("\"ca19_kill_policy\": \"factual\",", "", 1);
        match ProfileFile::from_json(&missing) {
            Err(ProfileError::Parse(msg)) => assert!(msg.contains("ca19_kill_policy"), "{msg}"),
            other => panic!("expected a parse error, got {other:?}"),
        }
    }

    #[test]
    fn the_planning_hit_windows_stay_inside_the_move_boxes() {
        use crate::combat::{CARL_HAYMAKER, CARL_JAB, ENEMY_HALF_WIDTH, HitShape};
        use crate::donut_moves::SWAT_BOX;
        let f = file();
        let inside = |lo: u16, hi: u16, box_min: f32, box_max: f32, what: &str| {
            assert!(
                f32::from(lo) >= box_min - ENEMY_HALF_WIDTH
                    && f32::from(hi) <= box_max + ENEMY_HALF_WIDTH,
                "{what}: {lo}..{hi} is outside [{}, {}]",
                box_min - ENEMY_HALF_WIDTH,
                box_max + ENEMY_HALF_WIDTH
            );
        };
        for set in [&f.cf02_assist, &f.companion_v0] {
            for spec in [CARL_JAB, CARL_HAYMAKER] {
                let HitShape::Box { x_min, x_max, .. } = spec.shape else {
                    panic!("{:?} is a box move", spec.id);
                };
                inside(
                    set.carl.ca41_strike_min_px,
                    set.carl.ca41_strike_max_px,
                    x_min,
                    x_max,
                    "carl strike",
                );
            }
            inside(
                set.donut.ca41_swat_hit_min_px,
                set.donut.ca41_swat_hit_max_px,
                SWAT_BOX.x0,
                SWAT_BOX.x1,
                "donut swat",
            );
        }
    }

    fn field_of(text: &str) -> String {
        match ProfileFile::from_json(text) {
            Err(ProfileError::Field { field, .. }) => field,
            other => panic!("expected a field error, got {other:?}"),
        }
    }

    #[test]
    fn a_load_error_names_the_field() {
        let swap = |from: &str, to: &str| PROFILE_JSON.replacen(from, to, 1);
        assert_eq!(
            field_of(&swap(
                "\"ca10_decision_cadence_ticks\": 6",
                "\"ca10_decision_cadence_ticks\": 0"
            )),
            "cf02_assist.ca10_decision_cadence_ticks"
        );
        assert_eq!(
            field_of(&swap(
                "\"ca26_ai_bailout\": false",
                "\"ca26_ai_bailout\": true"
            )),
            "cf02_assist.ca26_ai_bailout"
        );
        assert_eq!(
            field_of(&swap(
                "\"cf02_q15_pounce_min_px\": 30",
                "\"cf02_q15_pounce_min_px\": 80"
            )),
            "cf02_assist.donut.cf02_q15_pounce_min_px"
        );
        assert_eq!(
            field_of(&swap("\"ca21_active\": false", "\"ca21_active\": true")),
            "cf02_assist.carl.ca21_active"
        );
        assert_eq!(
            field_of(&swap("\"schema_version\": 1", "\"schema_version\": 2")),
            "schema_version"
        );
        // A missing or unknown field is named by the parser.
        let missing = PROFILE_JSON.replacen("\"cf02_q39_min_assist_starts\": 3,", "", 1);
        match ProfileFile::from_json(&missing) {
            Err(ProfileError::Parse(msg)) => {
                assert!(msg.contains("cf02_q39_min_assist_starts"), "{msg}")
            }
            other => panic!("expected a parse error, got {other:?}"),
        }
        let unknown = PROFILE_JSON.replacen("\"ca26_ai_bailout\"", "\"ai_bailout_typo\"", 1);
        assert!(matches!(
            ProfileFile::from_json(&unknown),
            Err(ProfileError::Parse(_))
        ));
        assert!(
            ProfileError::Field {
                field: "x.y".into(),
                reason: "r".into()
            }
            .to_string()
            .contains("`x.y`")
        );
    }
    // ------------------------------------------- COMPANION-AI-02 (CA2-A) ----

    #[test]
    fn cf02_free_is_cf02_assist_with_autonomy_on_and_the_a15_flank_off() {
        let f = file();
        assert_eq!(ProfileSetId::Cf02Free.name(), "cf02_free");
        assert_eq!(f.set(ProfileSetId::Cf02Free), &f.cf02_free);
        // CA2-Q01 and CA2-Q28: the only differences from the assistant set.
        let mut expected = f.cf02_assist.clone();
        expected.autonomy.enabled = true;
        expected.cf02_duo.cf02_duo_a15_flank = false;
        // FA-Q02..Q05 (PR-A): and the fix-all knobs, `cf02_free` only (the
        // layer they steer is off in the other sets).
        expected.fix_all = f.cf02_free.fix_all.clone();
        // DP-Q41 (DP-E): and the AI blink knobs, `cf02_free` only.
        let free_sk = &f.cf02_free.autonomy.roles.skirmisher;
        expected.autonomy.roles.skirmisher.blink_enabled = free_sk.blink_enabled;
        expected.autonomy.roles.skirmisher.blink_threat_px = free_sk.blink_threat_px;
        expected.autonomy.roles.skirmisher.blink_min_gain_px = free_sk.blink_min_gain_px;
        assert_eq!(f.cf02_free, expected);
        assert!(f.cf02_assist.cf02_duo.cf02_duo_a15_flank);
        assert!(f.cf02_free.autonomy.enabled);
        // Every other set ships with the layer off, so nothing reads it.
        assert!(!f.cf02_assist.autonomy.enabled);
        assert!(!f.companion_v0.autonomy.enabled);
        // The kill-steal veto rules it builds the claim set from stay on.
        let d = &f.cf02_free.cf02_duo;
        assert!(d.cf02_duo_a08_respect_finisher && d.cf02_duo_a11_engage_facing_only);
        assert_eq!(d.cf02_duo_a12_recent_hit_ticks, 31);
        assert_eq!(f.cf02_free.ca19_kill_policy, KillPolicy::Factual);
    }

    #[test]
    fn the_role_assignment_is_donut_skirmisher_and_carl_none_in_every_set() {
        let f = file();
        for set in [&f.cf02_assist, &f.companion_v0, &f.cf02_free] {
            assert_eq!(set.donut.autonomy_role, AutonomyRole::Skirmisher);
            assert_eq!(set.carl.autonomy_role, AutonomyRole::None);
            assert!(set.autonomy.table(AutonomyRole::None).is_none());
            assert!(set.autonomy.table(AutonomyRole::Skirmisher).is_some());
        }
        // The goal tables of the three sets are identical data (the DP-E blink
        // knobs, `cf02_free` only, aside).
        let mut free = f.cf02_free.autonomy.roles.clone();
        free.skirmisher.blink_enabled = false;
        free.skirmisher.blink_threat_px = 0;
        free.skirmisher.blink_min_gain_px = 0;
        assert_eq!(f.cf02_assist.autonomy.roles, free);
        assert_eq!(f.companion_v0.autonomy.roles, free);
    }

    #[test]
    fn the_autonomy_block_carries_the_adopted_ca2_defaults() {
        let a = &file().cf02_free.autonomy;
        assert_eq!(
            (
                a.ca2_q03_edge_margin_px,
                a.ca2_q04_leash_px,
                a.ca2_q06_comfort_px,
                a.ca2_q06_pull_per_px
            ),
            (32, 240, 160, 2)
        );
        assert_eq!(
            (
                a.ca2_q07_travel_ticks,
                a.ca2_q07_escort_bonus,
                a.ca2_q07_escort_min_px,
                a.ca2_q07_escort_max_px
            ),
            (12, 400, 72, 120)
        );
        assert_eq!(
            (
                a.ca2_q08_view_top_margin_px,
                a.ca2_q08_view_bottom_margin_px,
                a.ca2_q08_max_rise_px
            ),
            (48, 8, 176)
        );
        assert_eq!(
            (a.ca2_q09_drift_slack_px, a.ca2_q10_claim_radius_px),
            (8, 64)
        );
        assert_eq!(
            (
                a.ca2_q12_leader_low_hp,
                a.ca2_q13_outnumbered_count,
                a.ca2_q13_outnumbered_radius_px
            ),
            (2, 2, 96)
        );
        assert_eq!((a.ca2_q16_stickiness, a.ca2_q16_min_goal_ticks), (150, 24));
        assert_eq!(a.ca2_q25_bound_pad_px, 40);
        assert_eq!(
            (
                a.ca2_q29_spot_max_risk,
                a.ca2_q29_ledge_inset_px,
                a.ca2_q29_lane_penalty,
                a.ca2_q30_respot_px,
                a.ca2_q34_cause_window_ticks,
                a.ca2_q36_transit_slack_ticks
            ),
            (250, 16, 300, 48, 60, 4)
        );
        let sk = a.table(AutonomyRole::Skirmisher).unwrap();
        // CA2-Q18.
        let g = &sk.goals;
        assert_eq!(
            [
                g.retreat.utility,
                g.boss_punish.utility,
                g.support_leader.utility,
                g.hunt.utility,
                g.escort.utility,
                g.show_off.utility,
                g.perch.utility,
                g.idle.utility
            ],
            [1800, 1500, 1400, 1200, 800, 700, 600, 100]
        );
        assert!(g.rows().iter().all(|(_, row)| row.enabled));
        // CA2-Q35 and FA-Q04: under a boss only Retreat, BossPunish, Escort and
        // Idle (the interrupt is the director's).
        assert_eq!(
            g.rows()
                .iter()
                .filter(|(_, row)| row.under_boss)
                .map(|(name, _)| *name)
                .collect::<Vec<_>>(),
            ["retreat", "boss_punish", "escort", "idle"]
        );
        // CA2-Q27: no follow-bot ledge hop under autonomy.
        assert!(!a.ca2_q27_partner_ledge_hop);
        assert!(sk.ca2_q11_exclude_claimed_from_hunt && sk.ca2_q31_llama_hunt_needs_unlock);
        assert_eq!(
            (
                sk.ca2_q14_base_lying,
                sk.ca2_q14_base_standing,
                sk.ca2_q14_base_llama,
                sk.ca2_q14_base_engineer,
                sk.ca2_q14_isolation_cap_px,
                sk.ca2_q14_isolation_divisor,
                sk.ca2_q14_travel_divisor,
                sk.ca2_q15_back_threat_bonus
            ),
            (300, 200, 260, 260, 192, 2, 2, 300)
        );
        assert_eq!(
            (
                sk.ca2_q19_pounce_prefer_px,
                sk.ca2_q19_recoil_px,
                sk.ca2_q19_recoil_ticks,
                sk.ca2_q20_cast_spacing_px,
                sk.ca2_q20_backoff_px
            ),
            (56, 96, 30, 112, 72)
        );
        assert_eq!(
            (
                sk.ca2_q21_hurt_window_ticks,
                sk.ca2_q21_hurt_enemy_px,
                sk.ca2_q21_low_hp,
                sk.ca2_q21_low_enemy_px,
                sk.ca2_q21_exit_ticks,
                sk.ca2_q21_exit_clear_px,
                sk.ca2_q21_cooldown_ticks
            ),
            (90, 96, 2, 128, 120, 160, 180)
        );
        assert_eq!(
            (sk.ca2_q22_between_bonus, sk.ca2_q22_height_bonus_per_16px),
            (64, 4)
        );
        assert_eq!(
            (
                sk.ca2_q23_quiet_ticks,
                sk.ca2_q23_perch_range_px,
                sk.ca2_q23_max_ticks,
                sk.ca2_q23_cooldown_ticks,
                sk.ca2_q24_hold_ticks,
                sk.ca2_q24_safe_px,
                sk.ca2_q24_cooldown_ticks
            ),
            (45, 200, 600, 240, 36, 128, 360)
        );
    }

    #[test]
    fn an_autonomy_load_error_names_the_field() {
        // The first occurrence of each value is in `cf02_assist` (autonomy off),
        // so the zero or inverted value is rejected there and names that set.
        let swap = |from: &str, to: &str| PROFILE_JSON.replacen(from, to, 1);
        assert_eq!(
            field_of(&swap(
                "\"ca2_q04_leash_px\": 240",
                "\"ca2_q04_leash_px\": 0"
            )),
            "cf02_assist.autonomy.ca2_q04_leash_px"
        );
        assert_eq!(
            field_of(&swap(
                "\"ca2_q06_comfort_px\": 160",
                "\"ca2_q06_comfort_px\": 241"
            )),
            "cf02_assist.autonomy.ca2_q06_comfort_px"
        );
        assert_eq!(
            field_of(&swap(
                "\"ca2_q07_escort_min_px\": 72",
                "\"ca2_q07_escort_min_px\": 120"
            )),
            "cf02_assist.autonomy.ca2_q07_escort_min_px"
        );
        assert_eq!(
            field_of(&swap("\"utility\": 1800", "\"utility\": 2000")),
            "cf02_assist.autonomy.roles.skirmisher.goals.retreat.utility"
        );
        assert_eq!(
            field_of(&swap(
                "\"ca2_q19_recoil_ticks\": 30",
                "\"ca2_q19_recoil_ticks\": 0"
            )),
            "cf02_assist.autonomy.roles.skirmisher.ca2_q19_recoil_ticks"
        );
        assert_eq!(
            field_of(&swap(
                "\"ca2_q20_backoff_px\": 72",
                "\"ca2_q20_backoff_px\": 112"
            )),
            "cf02_assist.autonomy.roles.skirmisher.ca2_q20_backoff_px"
        );
        // Unknown and missing fields are parse errors that name them.
        let unknown = PROFILE_JSON.replacen("\"ca2_q03_edge_margin_px\"", "\"ca2_typo\"", 1);
        assert!(matches!(
            ProfileFile::from_json(&unknown),
            Err(ProfileError::Parse(_))
        ));
        let bad_role = PROFILE_JSON.replacen("\"skirmisher\"", "\"brawler\"", 1);
        assert!(matches!(
            ProfileFile::from_json(&bad_role),
            Err(ProfileError::Parse(_))
        ));
    }

    #[test]
    fn an_enabled_autonomy_layer_needs_a_role_the_factual_policy_and_the_claim_rules() {
        let mut f = file();
        f.cf02_free.donut.autonomy_role = AutonomyRole::None;
        match f.validate() {
            Err(ProfileError::Field { field, .. }) => {
                assert_eq!(field, "cf02_free.donut.autonomy_role")
            }
            other => panic!("expected a field error, got {other:?}"),
        }
        let mut f = file();
        f.cf02_free.ca19_kill_policy = KillPolicy::Never;
        assert!(matches!(
            f.validate(),
            Err(ProfileError::Field { ref field, .. }) if field == "cf02_free.ca19_kill_policy"
        ));
        for edit in 0..3 {
            let mut f = file();
            let named = match edit {
                0 => {
                    f.cf02_free.cf02_duo.cf02_duo_a08_respect_finisher = false;
                    "cf02_duo_a08_respect_finisher"
                }
                1 => {
                    f.cf02_free.cf02_duo.cf02_duo_a11_engage_facing_only = false;
                    "cf02_duo_a11_engage_facing_only"
                }
                _ => {
                    f.cf02_free.cf02_duo.cf02_duo_a12_recent_hit_ticks = 0;
                    "cf02_duo_a12_recent_hit_ticks"
                }
            };
            assert!(
                matches!(
                    f.validate(),
                    Err(ProfileError::Field { ref field, .. })
                        if *field == format!("cf02_free.cf02_duo.{named}")
                ),
                "edit {edit}"
            );
        }
        // v1 ships the partner's table only (review N-2): an enabled layer
        // refuses a role on AI Carl, whose hits the claim set does not read.
        let mut f = file();
        f.cf02_free.carl.autonomy_role = AutonomyRole::Skirmisher;
        assert!(matches!(
            f.validate(),
            Err(ProfileError::Field { ref field, .. }) if field == "cf02_free.carl.autonomy_role"
        ));
        // The same assignment is inert data while the layer is off (the AI blink
        // is not: DP-Q40 refuses it without the layer).
        f.cf02_free.autonomy.roles.skirmisher.blink_enabled = false;
        f.cf02_free.autonomy.enabled = false;
        assert!(f.validate().is_ok());
        // The leash is capped at the leading-side camera bound (review N-4).
        let mut f = file();
        f.cf02_free.autonomy.ca2_q04_leash_px = 260;
        assert!(f.validate().is_ok());
        f.cf02_free.autonomy.ca2_q04_leash_px = 261;
        assert!(matches!(
            f.validate(),
            Err(ProfileError::Field { ref field, .. })
                if field == "cf02_free.autonomy.ca2_q04_leash_px"
        ));
        // The same edits are fine while the layer is off (companion_v0 has
        // them off today).
        let mut f = file();
        f.cf02_free.autonomy.roles.skirmisher.blink_enabled = false;
        f.cf02_free.autonomy.enabled = false;
        f.cf02_free.cf02_duo.cf02_duo_a12_recent_hit_ticks = 0;
        assert!(f.validate().is_ok());
        // cf02_free is the assistant set: AI Carl stays passive.
        let mut f = file();
        f.cf02_free.carl.ca21_active = true;
        assert!(matches!(
            f.validate(),
            Err(ProfileError::Field { ref field, .. }) if field == "cf02_free.carl.ca21_active"
        ));
    }

    /// DP-Q41 (DP-E): the blink knobs are a recorded schema extension: optional,
    /// absent = off, `cf02_free` only, 48 and 64 px (DP-Q39).
    #[test]
    fn the_ai_blink_knobs_are_on_in_cf02_free_only() {
        let file = file();
        for id in [
            ProfileSetId::Cf02Assist,
            ProfileSetId::CompanionV0,
            ProfileSetId::Cf02Free,
        ] {
            let sk = &file.set(id).autonomy.roles.skirmisher;
            if id == ProfileSetId::Cf02Free {
                assert!(sk.blink_enabled, "{id:?}");
                assert_eq!((sk.blink_threat_px, sk.blink_min_gain_px), (48, 64));
            } else {
                assert!(!sk.blink_enabled, "{id:?}: absent means off");
                assert_eq!((sk.blink_threat_px, sk.blink_min_gain_px), (0, 0));
            }
        }
        // Absent fields load as off: the file without any of the three parses.
        let stripped = PROFILE_JSON
            .replace(",\n          \"blink_enabled\": true", "")
            .replace(",\n          \"blink_threat_px\": 48", "")
            .replace(",\n          \"blink_min_gain_px\": 64", "");
        assert_ne!(stripped, PROFILE_JSON);
        let loaded = ProfileFile::from_json(&stripped).expect("optional fields");
        assert!(
            !loaded
                .set(ProfileSetId::Cf02Free)
                .autonomy
                .roles
                .skirmisher
                .blink_enabled
        );
    }

    #[test]
    fn an_enabled_ai_blink_needs_its_knobs_and_an_enabled_autonomy_layer() {
        let swap = |from: &str, to: &str| PROFILE_JSON.replacen(from, to, 1);
        assert_eq!(
            field_of(&swap("\"blink_threat_px\": 48", "\"blink_threat_px\": 0")),
            "cf02_free.autonomy.roles.skirmisher.blink_threat_px"
        );
        assert_eq!(
            field_of(&swap(
                "\"blink_min_gain_px\": 64",
                "\"blink_min_gain_px\": 0"
            )),
            "cf02_free.autonomy.roles.skirmisher.blink_min_gain_px"
        );
        // DP-Q40: never for a set without an enabled autonomy layer (the first
        // `"ca2_q31_llama_hunt_needs_unlock": true` belongs to `cf02_assist`).
        assert_eq!(
            field_of(&swap(
                "\"ca2_q31_llama_hunt_needs_unlock\": true",
                "\"ca2_q31_llama_hunt_needs_unlock\": true, \"blink_enabled\": true, \"blink_threat_px\": 48, \"blink_min_gain_px\": 64"
            )),
            "cf02_assist.autonomy.roles.skirmisher.blink_enabled"
        );
    }
}
