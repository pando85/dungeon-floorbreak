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
//! Everything under [`PartyTuning`] is a provisional data knob, labelled with
//! its open owner question (`Qn`), never a hardcoded design decision.

use crate::combat::{HERO_HEALTH, HERO_INVULNERABLE_TICKS};
use crate::input::{RawInput, SlotInputs};

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

/// provisional: Q3. Which hero a goblin chases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyTargetRule {
    /// Nearest living hero; a tie goes to slot 0 (Carl).
    NearestLivingTieSlot0,
}

/// provisional: Q4. Donut's damage hurtbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DonutHurtbox {
    SameAsCarl,
}

/// provisional: Q7. What the scene camera follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraFollow {
    /// The controlled hero in solo, the midpoint of both in two-player.
    ControlledSoloMidpointTwoPlayer,
}

/// provisional: Q9. The uncontrolled hero's behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UncontrolledHero {
    /// Today's trail-follow; never attacks.
    TrailFollowNeverAttacks,
}

/// provisional: Q11. Which glyph set a HUD plate shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlateGlyphs {
    /// Each plate shows the glyphs of the pad that owns its hero.
    OwnPad,
}

/// The provisional party knobs. Every field names the unresolved owner
/// question it stands in for; change the value here, not at a call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartyTuning {
    /// provisional: Q2. Health Donut loses per hit (Carl's rule: 1).
    pub donut_damage_per_hit: u8,
    /// provisional: Q2. Donut's invulnerability ticks after a hit (Carl's).
    pub donut_invulnerable_ticks: u8,
    /// provisional: Q3.
    pub enemy_target: EnemyTargetRule,
    /// provisional: Q4.
    pub donut_hurtbox: DonutHurtbox,
    /// provisional: Q7.
    pub camera_follow: CameraFollow,
    /// provisional: Q8. When true the solo swap is refused while the
    /// controlled hero is hurt (in hit reaction); otherwise it is always allowed.
    pub swap_blocked_while_hurt: bool,
    /// provisional: Q9.
    pub uncontrolled_hero: UncontrolledHero,
    /// provisional: Q10. Show ability chips for Donut (none for now).
    pub donut_ability_chips: bool,
    /// provisional: Q11.
    pub plate_glyphs: PlateGlyphs,
}

/// The tuning table used until the owner answers Q2, Q3, Q4, Q7-Q11.
pub const PROVISIONAL_TUNING: PartyTuning = PartyTuning {
    donut_damage_per_hit: 1,
    donut_invulnerable_ticks: HERO_INVULNERABLE_TICKS,
    enemy_target: EnemyTargetRule::NearestLivingTieSlot0,
    donut_hurtbox: DonutHurtbox::SameAsCarl,
    camera_follow: CameraFollow::ControlledSoloMidpointTwoPlayer,
    swap_blocked_while_hurt: true,
    uncontrolled_hero: UncontrolledHero::TrailFollowNeverAttacks,
    donut_ability_chips: false,
    plate_glyphs: PlateGlyphs::OwnPad,
};

impl Default for PartyTuning {
    fn default() -> Self {
        PROVISIONAL_TUNING
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

    /// Claim the lowest vacated slot for `pad`, if there is one.
    fn reclaim(&mut self, pad: PadId) -> Option<PlayerSlot> {
        let slot = self.vacated_slots().first().copied()?;
        self.pads[slot.index()] = Some(pad);
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
        if self.pads[0].is_none() {
            self.pads[0] = Some(pad);
            return Some(PlayerSlot::P1);
        }
        None
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
        if self.pads[0].is_none() {
            self.pads[0] = Some(pad);
            return StartOutcome::Joined(PlayerSlot::P1);
        }
        if self.mode == PartyMode::Solo && self.pads[1].is_none() {
            self.pads[1] = Some(pad);
            self.mode = PartyMode::TwoPlayer;
            return StartOutcome::Joined(PlayerSlot::P2);
        }
        StartOutcome::Ignored
    }

    /// The solo swap (L1/LB). `swap_pressed` is the press edge and
    /// `controlled_hurt` says the controlled hero is in hit reaction. Never
    /// swaps in two-player (L1 must not steal the other human's hero), while
    /// paused, or while hurt when the Q8 knob says so. Returns true on a swap.
    pub fn request_swap(&mut self, swap_pressed: bool, controlled_hurt: bool) -> bool {
        if !swap_pressed || self.mode != PartyMode::Solo || self.is_paused() {
            return false;
        }
        if controlled_hurt && self.tuning.swap_blocked_while_hurt {
            return false;
        }
        self.controlled = self.controlled.other();
        true
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
            ..RawInput::default()
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
        assert!(party.request_swap(true, false));
        assert_eq!(party.controlled_hero(), HeroId::Carl);
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
            ..RawInput::default()
        };
        let p2 = RawInput {
            attack: true,
            swap: true,
            ..RawInput::default()
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
    fn provisional_tuning_matches_the_owner_provisional_values() {
        let t = PROVISIONAL_TUNING;
        assert_eq!(t.donut_damage_per_hit, 1);
        assert_eq!(t.donut_invulnerable_ticks, HERO_INVULNERABLE_TICKS);
        assert_eq!(t.enemy_target, EnemyTargetRule::NearestLivingTieSlot0);
        assert_eq!(t.donut_hurtbox, DonutHurtbox::SameAsCarl);
        assert_eq!(
            t.camera_follow,
            CameraFollow::ControlledSoloMidpointTwoPlayer
        );
        assert!(t.swap_blocked_while_hurt);
        assert_eq!(
            t.uncontrolled_hero,
            UncontrolledHero::TrailFollowNeverAttacks
        );
        assert!(!t.donut_ability_chips);
        assert_eq!(t.plate_glyphs, PlateGlyphs::OwnPad);
    }
}
