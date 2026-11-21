//! Party contract: local two-player ownership, the solo swap, the disconnect
//! pause and the party-defeat rule (WORK-005, WORK-006, HUD-DEBT-04/01).
//!
//! Bevy-free and deterministic. This module only decides *who controls which
//! hero*; it never moves a hero. The adapter feeds it pad events and per-slot
//! raw inputs, then hands the resulting per-hero inputs to
//! [`crate::corridor::SceneSim::step_party`].
//!
//! Owner decisions (2026-11-19), fixed:
//! * D1: either hero at 0 HP is a party defeat ([`is_party_defeat`]).
//! * D5: the keyboard is P1; a second pad joins as P2 by pressing Start
//!   (Menu / Options) ([`PartyState::pad_start_pressed`]).
//! * D6: any owning pad disconnect pauses the game until a pad reclaims the
//!   slot; there is no AI handoff ([`PartyState::pad_disconnected`]).
//!
//! Everything under [`PartyTuning`] is a data knob labelled with its owner
//! question (`Qn`), never a hardcoded design decision. Status of every value:
//! adopted (orchestrator, owner directive 2026-11-19), game-feel pending. The
//! analysis and its recommendations are in
//! `docs/01-design/party-2p-open-questions.md`.

use crate::combat::{HERO_HEALTH, HERO_INVULNERABLE_TICKS};
use crate::companion::profile::CompanionProfileSet;
use crate::feel::HERO_HURT_TICKS;
use crate::input::{RawInput, SlotInputs};
use crate::prototype::HeroMotion;

/// An opaque pad identity supplied by the adapter (for example the Bevy
/// gamepad entity bits). Never interpreted here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PadId(pub u64);

/// A local player slot. P1 also owns the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerSlot {
    P1,
    P2,
}

impl PlayerSlot {
    pub const ALL: [PlayerSlot; 2] = [PlayerSlot::P1, PlayerSlot::P2];

    pub const fn index(self) -> usize {
        match self {
            PlayerSlot::P1 => 0,
            PlayerSlot::P2 => 1,
        }
    }
}

/// A party hero. The index is the order of the per-hero arrays
/// (`step_party` inputs, health pairs): Carl 0, Donut 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeroId {
    Carl,
    Donut,
}

impl HeroId {
    pub const ALL: [HeroId; 2] = [HeroId::Carl, HeroId::Donut];

    pub const fn index(self) -> usize {
        match self {
            HeroId::Carl => 0,
            HeroId::Donut => 1,
        }
    }

    pub const fn other(self) -> HeroId {
        match self {
            HeroId::Carl => HeroId::Donut,
            HeroId::Donut => HeroId::Carl,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartyMode {
    /// One human: directly controls one hero and swaps with L1/LB.
    Solo,
    /// Two humans: P1 is Carl, P2 is Donut. Swap is disabled.
    TwoPlayer,
}

/// Who drives a hero this tick (the HUD reads this, never fakes it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeroControl {
    Player(PlayerSlot),
    /// Not human-controlled: follows the trail ([`UncontrolledHero`]).
    Uncontrolled,
}

/// Q3 (adopted, game-feel pending). Which hero a goblin chases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyTargetRule {
    /// Nearest living hero (ties to Carl). Recomputed every tick.
    NearestLivingTieSlot0,
    /// Nearest, but a goblin keeps its target through an attack and switches
    /// only when the other hero is `target_switch_px` closer. In solo the
    /// uncontrolled hero counts `target_ai_bias_px` farther away.
    HysteresisControlledBias,
}

/// Q4 (adopted, game-feel pending). Donut's damage hurtbox, measured from her
/// feet and centred on her pivot. Separate from her terrain collider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DonutHurtbox {
    /// Carl's 24x52 box.
    SameAsCarl,
    /// 32x24: fitted to the visible body.
    FitBody,
    /// 20x24: smaller than the silhouette, a swing that only clips her tail misses.
    Forgiving,
}

impl DonutHurtbox {
    /// `(width, height)` in pixels.
    pub const fn size(self) -> (f32, f32) {
        match self {
            DonutHurtbox::SameAsCarl => (24.0, 52.0),
            DonutHurtbox::FitBody => (32.0, 24.0),
            DonutHurtbox::Forgiving => (20.0, 24.0),
        }
    }
}

/// Q7 (adopted, game-feel pending). What the scene camera follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraFollow {
    /// The controlled hero in solo, the midpoint of both in two-player, with a
    /// separation wall. No zoom.
    ControlledSoloMidpointTwoPlayer,
}

/// Q9 (adopted, game-feel pending). The uncontrolled hero's behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UncontrolledHero {
    /// Legacy trail replay (never attacks). Only the scripted story and
    /// fixture paths use it; a live party never does.
    TrailFollowNeverAttacks,
    /// Follow-and-jump bot: steers behind the leader with its own run and
    /// jump, dodges a goblin tell with a jump, never attacks.
    FollowAndJumpBot,
    /// COMPANION-AI-01 (CA-21, 01A-wire): the shared companion director
    /// drives the uncontrolled hero with one profile set. Its movement is the
    /// follow-and-jump bot's; on top of it the director may start one legal
    /// support move. `FollowAndJumpBot` stays the default and the rollback
    /// (CA-22): nothing of the companion runs, hashes or is read without this
    /// variant.
    Companion(CompanionProfileSet),
}

/// Q11 (adopted, game-feel pending). Which glyph set a HUD plate shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlateGlyphs {
    /// Each plate shows the glyphs of the pad that owns its hero.
    OwnPad,
}

/// DONUT-MOBILITY-01 (DM-Q10): which ordinary motion Donut's body steps with.
/// There is no CLI flag or data switch: [`PROVISIONAL_TUNING`] ships the agile
/// motion, so every tuning built from it is agile. [`MotionProfile::Legacy`] is
/// constructed only by tests and by the playtrace rebuild of a recording made
/// before DM-A (the absent descriptor field means legacy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionProfile {
    /// Carl's motion (`RUN_SPEED`, `JUMP_SPEED`): every body before DM-A.
    Legacy,
    /// Donut's agile motion from `hero_agility_v0.json` (DM-Q01, DM-Q02).
    Agile,
}

impl MotionProfile {
    /// The motion this profile steps Donut's body with.
    pub fn motion(self) -> HeroMotion {
        match self {
            Self::Legacy => HeroMotion::CARL,
            Self::Agile => crate::agility::agile_motion(),
        }
    }

    /// The name written to the playtrace (`feature_flags["donut_motion"]`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Legacy => "legacy",
            Self::Agile => "agile",
        }
    }
}

/// The party knobs. Every field names the owner question (`Qn`) it answers;
/// change the value here, not at a call site. Status of every value: adopted
/// (orchestrator, owner directive 2026-11-19), game-feel pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartyTuning {
    /// Q2. Health Donut loses per hit (Carl's rule: 1).
    pub donut_damage_per_hit: u8,
    /// Q2. Donut's invulnerability ticks after a hit.
    pub donut_invulnerable_ticks: u8,
    /// Q2. Donut's hit-reaction (flinch) ticks.
    pub donut_hurt_ticks: u8,
    /// Q3.
    pub enemy_target: EnemyTargetRule,
    /// Q3. A goblin switches target when the other hero is this much closer.
    pub target_switch_px: u16,
    /// Q3. In solo the uncontrolled hero counts this much farther away.
    pub target_ai_bias_px: u16,
    /// Q4.
    pub donut_hurtbox: DonutHurtbox,
    /// Q7.
    pub camera_follow: CameraFollow,
    /// Q7. Gap between each hero and the frame edge at maximum separation; the
    /// maximum separation is `view_w - 2 * this`.
    pub camera_edge_margin_px: u16,
    /// Q7. The edge arrow appears from this separation (HUD hint).
    pub camera_arrow_px: u16,
    /// Q7. Temporary camera speed cap after a solo swap (normal cap: 4).
    pub swap_pan_px_per_tick: u8,
    /// CF02-Q20. How the solo-swap pan moves: the legacy constant cap, or eased
    /// in and out ([`SwapPanMode::Eased`], `--cf02` only; the cap above is then
    /// not read).
    pub swap_pan_mode: SwapPanMode,
    /// Q8. The solo swap is refused while the controlled hero is hurt.
    pub swap_blocked_while_hurt: bool,
    /// Q8. Ticks after a swap before the next one.
    pub swap_cooldown_ticks: u8,
    /// Q8. A swap press refused by hurt or cooldown stays buffered this long.
    pub swap_buffer_ticks: u8,
    /// Q9.
    pub uncontrolled_hero: UncontrolledHero,
    /// Q9. The bot waits this far behind the leader.
    pub follow_distance_px: u16,
    /// Q9. The bot jumps when a goblin tell starts this close.
    pub dodge_radius_px: u16,
    /// Q9. Ticks off-screen before the bot is placed back behind the leader.
    pub regroup_ticks: u16,
    /// D1 safeguard M1 (solo only). An uncontrolled hero takes hits (flinch,
    /// HUD loss) but cannot drop below 1 HP.
    pub solo_last_hp_guard: bool,
    /// D1 safeguard M4 (solo only). A hero this far outside the view cannot
    /// be hurt.
    pub solo_offscreen_immunity: bool,
    /// D1 safeguard M4. Margin outside the view, in pixels.
    pub offscreen_margin_px: u16,
    /// Q10-01 (WORK-023). Donut has her swat and pounce (and, after the
    /// biscuit, her Magic Missile). `false` is the attack-less Donut of the
    /// party PR (#114): no move starts, no press is read and no chip shows.
    /// Her ability chips follow it ([`Self::donut_ability_chips`]).
    pub donut_moves: bool,
    /// Q11.
    pub plate_glyphs: PlateGlyphs,
    /// DONUT-MOBILITY-01 (DM-Q10). The motion Donut's body steps with, applied
    /// every tick (never stored on the body). Carl is never affected.
    pub donut_motion: MotionProfile,
}

/// CF02-Q20: how the solo-swap camera pan moves toward the new hero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapPanMode {
    /// The legacy pan: a constant `swap_pan_px_per_tick` (8) until it catches up.
    Constant,
    /// CF02-Q20: eased in and out. Each tick `v = min(clamp(min(v_prev + 2,
    /// ceil(remaining / 4)), 2, 12), remaining)` px toward the new hero, from
    /// `v_prev = 0` ([`eased_pan_step`]).
    Eased,
}

/// CF02-Q19: the solo-swap cooldown under `--cf02` (legacy 20).
pub const CF02_SWAP_COOLDOWN_TICKS: u8 = 12;

/// CF02-Q20: the eased pan's velocity ramp, clamp and arrival divisor.
pub const EASED_PAN_RAMP_PX: i32 = 2;
pub const EASED_PAN_MIN_PX: i32 = 2;
pub const EASED_PAN_MAX_PX: i32 = 12;
pub const EASED_PAN_ARRIVAL_DIV: i32 = 4;

/// CF02-Q20: the pan velocity of this tick, in px, given the previous tick's
/// velocity and the pixels still to cover (`remaining > 0`). It ramps 2, 4, 6 ...
/// up to 12, slows on arrival (`ceil(remaining / 4)`) and never overshoots.
pub fn eased_pan_step(v_prev: i32, remaining: i32) -> i32 {
    if remaining <= 0 {
        return 0;
    }
    let arrival = (remaining + EASED_PAN_ARRIVAL_DIV - 1) / EASED_PAN_ARRIVAL_DIV;
    let wanted = (v_prev + EASED_PAN_RAMP_PX).min(arrival);
    wanted
        .clamp(EASED_PAN_MIN_PX, EASED_PAN_MAX_PX)
        .min(remaining)
}

impl PartyTuning {
    /// CF02-Q19 and Q20: this tuning with the `--cf02` swap feel (a 12-tick
    /// cooldown and the eased pan). The buffer, the hurt block, 2P ownership and
    /// the party-defeat rules are unchanged.
    #[must_use]
    pub const fn with_cf02_swap(self) -> Self {
        Self {
            swap_cooldown_ticks: CF02_SWAP_COOLDOWN_TICKS,
            swap_pan_mode: SwapPanMode::Eased,
            ..self
        }
    }

    /// Whether a human-driven Donut plate shows ability chips: derived from
    /// [`Self::donut_moves`] (Q10 section 4), never a second knob.
    pub const fn donut_ability_chips(&self) -> bool {
        self.donut_moves
    }

    /// The largest horizontal distance between the heroes in two-player.
    pub const fn max_separation_px(&self, view_w: i32) -> i32 {
        view_w - 2 * self.camera_edge_margin_px as i32
    }
}

/// The tuning table the party ships with: the recommendations of
/// `docs/01-design/party-2p-open-questions.md` adopted by the orchestrator on
/// the owner's directive (2026-11-19). Game feel is pending the owner's play.
pub const PROVISIONAL_TUNING: PartyTuning = PartyTuning {
    donut_damage_per_hit: 1,
    donut_invulnerable_ticks: HERO_INVULNERABLE_TICKS,
    donut_hurt_ticks: HERO_HURT_TICKS,
    enemy_target: EnemyTargetRule::HysteresisControlledBias,
    target_switch_px: 24,
    target_ai_bias_px: 40,
    donut_hurtbox: DonutHurtbox::Forgiving,
    camera_follow: CameraFollow::ControlledSoloMidpointTwoPlayer,
    camera_edge_margin_px: 96,
    camera_arrow_px: 384,
    swap_pan_px_per_tick: 8,
    swap_pan_mode: SwapPanMode::Constant,
    swap_blocked_while_hurt: true,
    swap_cooldown_ticks: 20,
    swap_buffer_ticks: 6,
    uncontrolled_hero: UncontrolledHero::FollowAndJumpBot,
    follow_distance_px: 40,
    dodge_radius_px: 40,
    regroup_ticks: 120,
    solo_last_hp_guard: true,
    solo_offscreen_immunity: true,
    offscreen_margin_px: 16,
    donut_moves: true,
    plate_glyphs: PlateGlyphs::OwnPad,
    donut_motion: MotionProfile::Agile,
};

impl Default for PartyTuning {
    fn default() -> Self {
        PROVISIONAL_TUNING
    }
}

/// What the scene needs to know about the party each tick: whether the party
/// simulation is live at all, who is human-driven, and the tuning. Built by
/// [`PartyState::drive`]. [`PartyDrive::LEGACY`] (the default) keeps the
/// single-hero behaviour byte-for-byte: Donut replays Carl's trail, goblins
/// chase Carl and nothing new is hashed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartyDrive {
    /// Donut is a real hero (own physics, hurtbox and health rules).
    pub live: bool,
    pub two_player: bool,
    /// The hero P1 controls in solo (Carl in two-player).
    pub controlled: HeroId,
    /// Heroes driven by the follower bot, indexed by [`HeroId::index`].
    pub bots: [bool; 2],
    pub tuning: PartyTuning,
}

impl PartyDrive {
    /// The scripted story and fixture paths: no live party.
    pub const LEGACY: PartyDrive = PartyDrive {
        live: false,
        two_player: false,
        controlled: HeroId::Carl,
        bots: [false, false],
        tuning: PROVISIONAL_TUNING,
    };

    /// True while `hero` is driven by the follower bot.
    pub fn is_bot(&self, hero: HeroId) -> bool {
        self.live && self.bots[hero.index()]
    }

    /// The companion profile set that drives `hero`, while it is a bot and the
    /// tuning names [`UncontrolledHero::Companion`] (CA-21). `None` for the
    /// rollback bot, for a human-driven hero and in two-player.
    pub fn companion_set(&self, hero: HeroId) -> Option<CompanionProfileSet> {
        match self.tuning.uncontrolled_hero {
            UncontrolledHero::Companion(set) if self.is_bot(hero) => Some(set),
            _ => None,
        }
    }

    /// D1 safeguards M1 and M4 apply to a hero that a human does not drive
    /// while the party is solo.
    pub fn is_guarded(&self, hero: HeroId) -> bool {
        self.live && !self.two_player && self.bots[hero.index()]
    }
}

impl Default for PartyDrive {
    fn default() -> Self {
        Self::LEGACY
    }
}

/// D1: either hero at 0 HP is a party defeat. `health` is indexed by
/// [`HeroId::index`].
pub const fn is_party_defeat(health: [u8; 2]) -> bool {
    health[0] == 0 || health[1] == 0
}

/// What a pad's Start press did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartOutcome {
    /// The pad already owns a slot: Start is the normal pause/menu press.
    OwnerPause(PlayerSlot),
    /// The pad claimed or reclaimed a slot (a join when it is P2 in Solo).
    Joined(PlayerSlot),
    /// Nothing changed (a third pad, or P2 is taken).
    Ignored,
}

/// Slot ownership, mode, solo swap and disconnect pause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyState {
    tuning: PartyTuning,
    mode: PartyMode,
    /// The solo controlled hero (meaningless in two-player).
    controlled: HeroId,
    /// The pad owning each slot (`None`: keyboard only, or a vacated slot).
    pads: [Option<PadId>; 2],
    /// A slot whose owning pad disconnected and has not been reclaimed.
    vacated: [bool; 2],
    /// The pad that last owned each slot, kept across a disconnect so a
    /// reconnecting pad returns to its own slot (S1).
    last_pads: [Option<PadId>; 2],
    /// The keyboard has driven P1 (S2): a pad that connects afterwards does
    /// not take P1 implicitly, so its Start joins instead of quitting.
    keyboard_p1: bool,
    /// Q8: ticks until the next swap is allowed.
    swap_cooldown: u8,
    /// Q8: ticks a refused swap press stays buffered.
    swap_buffer: u8,
}

impl Default for PartyState {
    fn default() -> Self {
        Self::new(PROVISIONAL_TUNING)
    }
}

impl PartyState {
    /// A fresh solo party: Carl controlled, no pad owned, nothing paused.
    pub fn new(tuning: PartyTuning) -> Self {
        Self {
            tuning,
            mode: PartyMode::Solo,
            controlled: HeroId::Carl,
            pads: [None, None],
            vacated: [false, false],
            last_pads: [None, None],
            keyboard_p1: false,
            swap_cooldown: 0,
            swap_buffer: 0,
        }
    }

    pub fn tuning(&self) -> &PartyTuning {
        &self.tuning
    }

    pub fn mode(&self) -> PartyMode {
        self.mode
    }

    /// The hero P1 controls in solo (Carl in two-player).
    pub fn controlled_hero(&self) -> HeroId {
        match self.mode {
            PartyMode::Solo => self.controlled,
            PartyMode::TwoPlayer => HeroId::Carl,
        }
    }

    /// Who drives `hero` right now.
    pub fn hero_control(&self, hero: HeroId) -> HeroControl {
        match self.mode {
            PartyMode::Solo if hero == self.controlled => HeroControl::Player(PlayerSlot::P1),
            PartyMode::Solo => HeroControl::Uncontrolled,
            PartyMode::TwoPlayer => HeroControl::Player(match hero {
                HeroId::Carl => PlayerSlot::P1,
                HeroId::Donut => PlayerSlot::P2,
            }),
        }
    }

    /// The pad that owns `slot`, if any.
    pub fn pad_of(&self, slot: PlayerSlot) -> Option<PadId> {
        self.pads[slot.index()]
    }

    /// The slot a pad owns. Input from a pad with no slot is dropped.
    pub fn slot_of(&self, pad: PadId) -> Option<PlayerSlot> {
        PlayerSlot::ALL
            .into_iter()
            .find(|slot| self.pads[slot.index()] == Some(pad))
    }

    /// D6: true while any owning pad is disconnected. The sim must not tick.
    pub fn is_paused(&self) -> bool {
        self.vacated.iter().any(|v| *v)
    }

    /// The slots waiting for a pad to come back.
    pub fn vacated_slots(&self) -> Vec<PlayerSlot> {
        PlayerSlot::ALL
            .into_iter()
            .filter(|slot| self.vacated[slot.index()])
            .collect()
    }

    /// Claim a vacated slot for `pad`: the slot this same pad owned before
    /// when it is waiting (S1), otherwise the lowest vacated slot.
    fn reclaim(&mut self, pad: PadId) -> Option<PlayerSlot> {
        let waiting = self.vacated_slots();
        let slot = waiting
            .iter()
            .copied()
            .find(|slot| self.last_pads[slot.index()] == Some(pad))
            .or_else(|| waiting.first().copied())?;
        self.pads[slot.index()] = Some(pad);
        self.last_pads[slot.index()] = Some(pad);
        self.vacated[slot.index()] = false;
        Some(slot)
    }

    /// A pad connected. A pad that returns while a slot waits reclaims it (a
    /// reconnect may bring a new id). Otherwise the first pad becomes P1's
    /// pad; any further pad stays unowned until it presses Start.
    pub fn pad_connected(&mut self, pad: PadId) -> Option<PlayerSlot> {
        if let Some(slot) = self.slot_of(pad) {
            return Some(slot);
        }
        if let Some(slot) = self.reclaim(pad) {
            return Some(slot);
        }
        if self.pads[0].is_none() && !self.keyboard_p1 {
            self.pads[0] = Some(pad);
            self.last_pads[0] = Some(pad);
            return Some(PlayerSlot::P1);
        }
        None
    }

    /// S2: the keyboard drove P1. From now on a pad that connects does not
    /// take P1 implicitly (it joins with Start, as P2 while the keyboard
    /// plays P1), so a friend's Start never quits the keyboard player's run.
    pub fn note_keyboard_p1(&mut self) {
        self.keyboard_p1 = true;
    }

    /// A pad disconnected. D6: an owning pad pauses the game and its slot waits
    /// for a reconnect. Returns true when the game is now paused because of it.
    pub fn pad_disconnected(&mut self, pad: PadId) -> bool {
        match self.slot_of(pad) {
            Some(slot) => {
                self.pads[slot.index()] = None;
                self.vacated[slot.index()] = true;
                true
            }
            None => false,
        }
    }

    /// D5: a pad pressed Start (Menu / Options). An owner keeps its normal
    /// pause press; an unowned pad reclaims a vacated slot, or claims an
    /// empty P1, or joins as P2 (switching to two-player) when P2 is free.
    pub fn pad_start_pressed(&mut self, pad: PadId) -> StartOutcome {
        if let Some(slot) = self.slot_of(pad) {
            return StartOutcome::OwnerPause(slot);
        }
        if let Some(slot) = self.reclaim(pad) {
            return StartOutcome::Joined(slot);
        }
        if self.pads[0].is_none() && !self.keyboard_p1 {
            self.pads[0] = Some(pad);
            self.last_pads[0] = Some(pad);
            return StartOutcome::Joined(PlayerSlot::P1);
        }
        if self.mode == PartyMode::Solo && self.pads[1].is_none() {
            self.pads[1] = Some(pad);
            self.last_pads[1] = Some(pad);
            self.mode = PartyMode::TwoPlayer;
            return StartOutcome::Joined(PlayerSlot::P2);
        }
        StartOutcome::Ignored
    }

    /// The solo swap (L1/LB), called once per simulated tick (it advances the
    /// Q8 cooldown and buffer). `swap_pressed` is the press edge and
    /// `controlled_hurt` says the controlled hero is in hit reaction. Never
    /// swaps in two-player (L1 must not steal the other human's hero) or while
    /// paused. Q8: a press refused by hurt or by the cooldown stays buffered for
    /// `swap_buffer_ticks` and fires the first tick it is allowed; a swap never
    /// cancels the outgoing hero's move (the party only changes who drives).
    /// Returns true on a swap.
    pub fn request_swap(&mut self, swap_pressed: bool, controlled_hurt: bool) -> bool {
        if self.mode != PartyMode::Solo || self.is_paused() {
            self.swap_buffer = 0;
            return false;
        }
        self.swap_cooldown = self.swap_cooldown.saturating_sub(1);
        self.swap_buffer = self.swap_buffer.saturating_sub(1);
        if swap_pressed {
            self.swap_buffer = self.tuning.swap_buffer_ticks.max(1);
        }
        if self.swap_buffer == 0 {
            return false;
        }
        let hurt_blocks = controlled_hurt && self.tuning.swap_blocked_while_hurt;
        if hurt_blocks || self.swap_cooldown > 0 {
            return false;
        }
        self.swap_buffer = 0;
        self.swap_cooldown = self.tuning.swap_cooldown_ticks;
        self.controlled = self.controlled.other();
        true
    }

    /// The Q8 swap timers `(cooldown, buffer)`, for hashes and traces.
    pub fn swap_timers(&self) -> (u8, u8) {
        (self.swap_cooldown, self.swap_buffer)
    }

    /// The scene's view of the party this tick (`live`: Donut is a real hero).
    /// Scripted scenario and fixture runs pass `live = false` and keep the
    /// legacy single-hero behaviour.
    pub fn drive(&self, live: bool) -> PartyDrive {
        PartyDrive {
            live,
            two_player: self.mode == PartyMode::TwoPlayer,
            controlled: self.controlled_hero(),
            bots: [
                self.hero_control(HeroId::Carl) == HeroControl::Uncontrolled,
                self.hero_control(HeroId::Donut) == HeroControl::Uncontrolled,
            ],
            tuning: self.tuning,
        }
    }

    /// Map the per-slot raw inputs to the per-hero inputs for
    /// `SceneSim::step_party`. Swap bits are stripped (the party consumed
    /// them), the uncontrolled hero gets an idle input, and nothing moves while
    /// paused.
    pub fn hero_inputs(&self, slots: SlotInputs) -> [RawInput; 2] {
        let strip = |raw: RawInput| RawInput { swap: false, ..raw };
        let mut out = [RawInput::default(); 2];
        if self.is_paused() {
            return out;
        }
        match self.mode {
            PartyMode::Solo => out[self.controlled.index()] = strip(slots[0]),
            PartyMode::TwoPlayer => {
                out[HeroId::Carl.index()] = strip(slots[0]);
                out[HeroId::Donut.index()] = strip(slots[1]);
            }
        }
        out
    }

    /// D1 with this party's health pair; see [`is_party_defeat`].
    pub fn defeated(&self, health: [u8; 2]) -> bool {
        is_party_defeat(health)
    }

    /// The party defeat restart: Carl is controlled again, mode and pad
    /// ownership stay (same restart flow as Carl's defeat).
    pub fn restart(&mut self) {
        self.controlled = HeroId::Carl;
        self.swap_cooldown = 0;
        self.swap_buffer = 0;
    }
}

/// Full health for both heroes (restart value): `[Carl, Donut]`.
pub const FULL_HEALTH: [u8; 2] = [HERO_HEALTH, HERO_HEALTH];

#[cfg(test)]
mod tests {
    use super::*;

    const A: PadId = PadId(1);
    const B: PadId = PadId(2);
    const C: PadId = PadId(3);

    fn raw(move_x: f32) -> RawInput {
        RawInput {
            move_x,
            attack: true,
            swap: true,
            ..RawInput::NONE
        }
    }

    #[test]
    fn solo_starts_with_carl_and_no_pad() {
        let party = PartyState::default();
        assert_eq!(party.mode(), PartyMode::Solo);
        assert_eq!(party.controlled_hero(), HeroId::Carl);
        assert_eq!(
            party.hero_control(HeroId::Carl),
            HeroControl::Player(PlayerSlot::P1)
        );
        assert_eq!(party.hero_control(HeroId::Donut), HeroControl::Uncontrolled);
        assert!(!party.is_paused());
    }

    #[test]
    fn first_pad_is_p1_and_a_second_pad_waits_for_start() {
        let mut party = PartyState::default();
        assert_eq!(party.pad_connected(A), Some(PlayerSlot::P1));
        assert_eq!(party.pad_connected(B), None, "connecting is not joining");
        assert_eq!(party.mode(), PartyMode::Solo);
        assert_eq!(party.slot_of(B), None);
        assert_eq!(
            party.pad_start_pressed(B),
            StartOutcome::Joined(PlayerSlot::P2)
        );
        assert_eq!(party.mode(), PartyMode::TwoPlayer);
        assert_eq!(party.slot_of(B), Some(PlayerSlot::P2));
    }

    #[test]
    fn owner_start_is_pause_not_a_join() {
        let mut party = PartyState::default();
        party.pad_connected(A);
        assert_eq!(
            party.pad_start_pressed(A),
            StartOutcome::OwnerPause(PlayerSlot::P1)
        );
        assert_eq!(party.mode(), PartyMode::Solo);
    }

    #[test]
    fn start_from_an_unowned_pad_claims_an_empty_p1_first() {
        let mut party = PartyState::default();
        assert_eq!(
            party.pad_start_pressed(A),
            StartOutcome::Joined(PlayerSlot::P1)
        );
        assert_eq!(
            party.mode(),
            PartyMode::Solo,
            "the keyboard player's pad, not a join"
        );
    }

    #[test]
    fn a_third_pad_is_ignored() {
        let mut party = PartyState::default();
        party.pad_connected(A);
        party.pad_start_pressed(B);
        assert_eq!(party.pad_start_pressed(C), StartOutcome::Ignored);
        assert_eq!(party.slot_of(C), None);
    }

    #[test]
    fn solo_swap_toggles_the_controlled_hero_and_routes_input() {
        let mut party = PartyState::default();
        assert_eq!(
            party.hero_inputs([raw(1.0), RawInput::default()])[0].move_x,
            1.0
        );
        assert!(party.request_swap(true, false));
        assert_eq!(party.controlled_hero(), HeroId::Donut);
        assert_eq!(party.hero_control(HeroId::Carl), HeroControl::Uncontrolled);
        let inputs = party.hero_inputs([raw(-1.0), RawInput::default()]);
        assert_eq!(inputs[HeroId::Carl.index()], RawInput::default());
        assert_eq!(inputs[HeroId::Donut.index()].move_x, -1.0);
        assert!(inputs[HeroId::Donut.index()].attack);
        assert!(
            !inputs[HeroId::Donut.index()].swap,
            "the party consumes swap"
        );
        for _ in 0..PROVISIONAL_TUNING.swap_cooldown_ticks {
            assert!(!party.request_swap(false, false));
        }
        assert!(party.request_swap(true, false));
        assert_eq!(party.controlled_hero(), HeroId::Carl);
    }

    #[test]
    fn q8_cooldown_refuses_a_second_swap_and_a_press_in_it_is_buffered() {
        let mut party = PartyState::default();
        assert!(party.request_swap(true, false));
        // Tick 1..19 after the swap: a press on tick 15 is buffered for 6
        // ticks and fires the tick the cooldown ends (tick 20).
        for _ in 0..14 {
            assert!(!party.request_swap(false, false));
        }
        assert!(!party.request_swap(true, false), "inside the cooldown");
        let mut fired_at = None;
        for tick in 16..=30 {
            if party.request_swap(false, false) {
                fired_at = Some(tick);
                break;
            }
        }
        assert_eq!(fired_at, Some(20), "buffered press fires when allowed");
        assert_eq!(party.controlled_hero(), HeroId::Carl);
    }

    #[test]
    fn q8_a_press_older_than_the_buffer_is_forgotten() {
        let mut party = PartyState::default();
        assert!(party.request_swap(true, false));
        assert!(!party.request_swap(true, false), "press on cooldown tick 1");
        for _ in 0..40 {
            assert!(!party.request_swap(false, false), "expired, never fires");
        }
        assert_eq!(party.controlled_hero(), HeroId::Donut);
    }

    #[test]
    fn q8_a_press_during_hurt_is_buffered_and_fires_when_the_hurt_ends() {
        let mut party = PartyState::default();
        // Hurt ticks 1..=18; the press lands on tick 14 (buffer 6 -> alive to 19).
        for _ in 0..13 {
            assert!(!party.request_swap(false, true));
        }
        assert!(!party.request_swap(true, true));
        let mut fired_at = None;
        for tick in 15..=25 {
            let still_hurt = tick <= 18;
            if party.request_swap(false, still_hurt) {
                fired_at = Some(tick);
                break;
            }
        }
        assert_eq!(fired_at, Some(19), "fires on the first tick after the hurt");
    }

    #[test]
    fn swap_needs_a_press_edge() {
        let mut party = PartyState::default();
        assert!(!party.request_swap(false, false));
        assert_eq!(party.controlled_hero(), HeroId::Carl);
    }

    #[test]
    fn swap_is_refused_while_hurt_and_follows_the_q8_knob() {
        let mut party = PartyState::default();
        assert!(!party.request_swap(true, true));
        assert_eq!(party.controlled_hero(), HeroId::Carl);
        let open = PartyTuning {
            swap_blocked_while_hurt: false,
            ..PROVISIONAL_TUNING
        };
        let mut party = PartyState::new(open);
        assert!(party.request_swap(true, true));
    }

    #[test]
    fn two_player_swap_never_steals_a_hero() {
        let mut party = PartyState::default();
        party.pad_connected(A);
        party.pad_start_pressed(B);
        assert!(!party.request_swap(true, false));
        assert_eq!(
            party.hero_control(HeroId::Carl),
            HeroControl::Player(PlayerSlot::P1)
        );
        assert_eq!(
            party.hero_control(HeroId::Donut),
            HeroControl::Player(PlayerSlot::P2)
        );
    }

    #[test]
    fn two_player_routes_each_slot_to_its_own_hero() {
        let mut party = PartyState::default();
        party.pad_connected(A);
        party.pad_start_pressed(B);
        let p1 = RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        };
        let p2 = RawInput {
            attack: true,
            swap: true,
            ..RawInput::NONE
        };
        let out = party.hero_inputs([p1, p2]);
        assert_eq!(out[0], p1, "no cross-talk: P1 drives Carl only");
        assert!(out[1].attack && !out[1].swap && out[1].move_x == 0.0);
    }

    #[test]
    fn an_owning_pad_disconnect_pauses_and_a_reconnect_resumes() {
        let mut party = PartyState::default();
        party.pad_connected(A);
        party.pad_start_pressed(B);
        assert!(party.pad_disconnected(B));
        assert!(party.is_paused());
        assert_eq!(party.vacated_slots(), vec![PlayerSlot::P2]);
        assert_eq!(
            party.hero_inputs([raw(1.0), raw(1.0)]),
            [RawInput::default(); 2]
        );
        assert!(!party.request_swap(true, false));
        // The reconnecting pad may arrive under a new id.
        assert_eq!(party.pad_connected(C), Some(PlayerSlot::P2));
        assert!(!party.is_paused());
        assert_eq!(party.slot_of(C), Some(PlayerSlot::P2));
        assert_eq!(party.slot_of(B), None);
        assert_eq!(party.mode(), PartyMode::TwoPlayer);
    }

    #[test]
    fn a_p1_pad_disconnect_pauses_in_solo_too_and_start_reclaims() {
        let mut party = PartyState::default();
        party.pad_connected(A);
        assert!(party.pad_disconnected(A));
        assert!(party.is_paused());
        assert_eq!(
            party.pad_start_pressed(B),
            StartOutcome::Joined(PlayerSlot::P1)
        );
        assert!(!party.is_paused());
        assert_eq!(party.mode(), PartyMode::Solo);
    }

    #[test]
    fn an_unowned_pad_disconnect_does_not_pause() {
        let mut party = PartyState::default();
        party.pad_connected(A);
        party.pad_connected(B);
        assert!(!party.pad_disconnected(B));
        assert!(!party.is_paused());
    }

    #[test]
    fn both_slots_can_wait_and_resume_only_when_both_return() {
        let mut party = PartyState::default();
        party.pad_connected(A);
        party.pad_start_pressed(B);
        party.pad_disconnected(A);
        party.pad_disconnected(B);
        assert_eq!(party.vacated_slots(), vec![PlayerSlot::P1, PlayerSlot::P2]);
        party.pad_connected(C);
        assert!(party.is_paused());
        party.pad_connected(PadId(4));
        assert!(!party.is_paused());
    }

    #[test]
    fn a_reconnecting_pad_returns_to_its_own_slot_whatever_the_order() {
        // S1: both pads drop, P2's pad comes back first and keeps P2.
        let mut party = PartyState::default();
        party.pad_connected(A);
        party.pad_start_pressed(B);
        party.pad_disconnected(A);
        party.pad_disconnected(B);
        assert_eq!(party.pad_connected(B), Some(PlayerSlot::P2));
        assert_eq!(party.slot_of(B), Some(PlayerSlot::P2));
        assert!(party.is_paused(), "P1 still waits");
        assert_eq!(party.pad_connected(A), Some(PlayerSlot::P1));
        assert_eq!(party.slot_of(A), Some(PlayerSlot::P1));
        assert!(!party.is_paused());
        assert_eq!(
            party.hero_control(HeroId::Carl),
            HeroControl::Player(PlayerSlot::P1)
        );

        // The same holds for a Start press, and for A coming back first.
        let mut party = PartyState::default();
        party.pad_connected(A);
        party.pad_start_pressed(B);
        party.pad_disconnected(A);
        party.pad_disconnected(B);
        assert_eq!(
            party.pad_start_pressed(B),
            StartOutcome::Joined(PlayerSlot::P2)
        );
        assert_eq!(party.slot_of(A), None);
        assert_eq!(party.pad_connected(A), Some(PlayerSlot::P1));
        assert_eq!(party.slot_of(B), Some(PlayerSlot::P2));

        // A stranger (never owned a slot) takes the lowest waiting slot.
        let mut party = PartyState::default();
        party.pad_connected(A);
        party.pad_start_pressed(B);
        party.pad_disconnected(A);
        party.pad_disconnected(B);
        assert_eq!(party.pad_connected(C), Some(PlayerSlot::P1));
    }

    #[test]
    fn a_pad_connecting_after_keyboard_play_does_not_take_p1() {
        // S2: the keyboard drives P1, a friend's pad connects.
        let mut party = PartyState::default();
        party.note_keyboard_p1();
        assert_eq!(party.pad_connected(A), None);
        assert_eq!(party.slot_of(A), None);
        // Its Start joins as P2 and never reads as the P1 owner's press.
        assert_eq!(
            party.pad_start_pressed(A),
            StartOutcome::Joined(PlayerSlot::P2)
        );
        assert_eq!(party.mode(), PartyMode::TwoPlayer);
        // Now it owns P2: its Start is P2's (not a quit).
        assert_eq!(
            party.pad_start_pressed(A),
            StartOutcome::OwnerPause(PlayerSlot::P2)
        );
    }

    #[test]
    fn either_hero_at_zero_is_party_defeat() {
        assert!(!is_party_defeat(FULL_HEALTH));
        assert!(!is_party_defeat([1, 1]));
        assert!(is_party_defeat([0, HERO_HEALTH]), "Carl down");
        assert!(
            is_party_defeat([HERO_HEALTH, 0]),
            "D1: Donut down is the same defeat"
        );
        assert!(PartyState::default().defeated([2, 0]));
    }

    #[test]
    fn restart_returns_control_to_carl_and_keeps_ownership() {
        let mut party = PartyState::default();
        party.pad_connected(A);
        party.request_swap(true, false);
        party.restart();
        assert_eq!(party.controlled_hero(), HeroId::Carl);
        assert_eq!(party.slot_of(A), Some(PlayerSlot::P1));
    }

    #[test]
    fn adopted_tuning_matches_the_recommendations_of_the_analysis() {
        let t = PROVISIONAL_TUNING;
        // Q2
        assert_eq!(t.donut_damage_per_hit, 1);
        assert_eq!(t.donut_invulnerable_ticks, HERO_INVULNERABLE_TICKS);
        assert_eq!(t.donut_hurt_ticks, HERO_HURT_TICKS);
        // Q3
        assert_eq!(t.enemy_target, EnemyTargetRule::HysteresisControlledBias);
        assert_eq!((t.target_switch_px, t.target_ai_bias_px), (24, 40));
        // Q4
        assert_eq!(t.donut_hurtbox, DonutHurtbox::Forgiving);
        assert_eq!(t.donut_hurtbox.size(), (20.0, 24.0));
        // Q7
        assert_eq!(
            t.camera_follow,
            CameraFollow::ControlledSoloMidpointTwoPlayer
        );
        assert_eq!(t.max_separation_px(640), 448);
        assert_eq!((t.camera_arrow_px, t.swap_pan_px_per_tick), (384, 8));
        // Q8
        assert!(t.swap_blocked_while_hurt);
        assert_eq!((t.swap_cooldown_ticks, t.swap_buffer_ticks), (20, 6));
        // Q9
        assert_eq!(t.uncontrolled_hero, UncontrolledHero::FollowAndJumpBot);
        assert_eq!(
            (t.follow_distance_px, t.dodge_radius_px, t.regroup_ticks),
            (40, 40, 120)
        );
        // D1 safeguards
        assert!(t.solo_last_hp_guard && t.solo_offscreen_immunity);
        assert_eq!(t.offscreen_margin_px, 16);
        // Q10-01: Donut has her swat and pounce, and her chips follow it.
        assert!(t.donut_moves);
        assert_eq!(t.donut_ability_chips(), t.donut_moves);
        // Q11
        assert_eq!(t.plate_glyphs, PlateGlyphs::OwnPad);
    }

    #[test]
    fn the_drive_reports_who_is_a_bot_and_when_the_guards_apply() {
        let mut party = PartyState::default();
        assert!(!party.drive(false).live, "scripted runs stay legacy");
        let solo = party.drive(true);
        assert!(solo.live && !solo.is_bot(HeroId::Carl) && solo.is_bot(HeroId::Donut));
        assert!(solo.is_guarded(HeroId::Donut) && !solo.is_guarded(HeroId::Carl));
        party.request_swap(true, false);
        let swapped = party.drive(true);
        assert!(swapped.is_guarded(HeroId::Carl), "the guard follows the AI");
        assert_eq!(swapped.controlled, HeroId::Donut);
        party.pad_connected(A);
        party.pad_start_pressed(B);
        let duo = party.drive(true);
        assert!(duo.two_player && duo.bots == [false, false]);
        assert!(!duo.is_guarded(HeroId::Donut), "2P applies no safeguard");
    }
}
