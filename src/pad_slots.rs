//! Pad-to-slot input mapping for local two-player (HUD-DEBT-04, WORK-005).
//!
//! Bevy-free adapter logic: the scene binary turns Bevy gamepad state into
//! [`PadEvent`]s and [`PadFrame`]s (opaque ids, vendor id, held and
//! just-pressed buttons); this module routes them through
//! [`crate::party::PartyState`] and hands back one [`RawInput`] per player slot
//! ([`SlotInputs`]) plus the per-slot HUD glyph sets. No Bevy type appears
//! here, so the routing is unit-tested with fake pad ids.
//!
//! Rules (owner decisions 2026-11-19, see `party`):
//! * the keyboard is always P1 and merges with P1's pad;
//! * the first pad owns P1 unless the keyboard already drove P1 (S2); a second
//!   pad joins as P2 by pressing Start, and a Start on a pad that owns no slot
//!   only ever joins: only the device that owns P1 quits on Start;
//! * a third pad, or a pad that owns no slot, is ignored (except Start);
//! * an owning pad that disconnects pauses the game ([`PadSlots::is_paused`])
//!   until a pad reclaims its slot;
//! * each slot's glyph set follows its own pad's vendor id (provisional Q11),
//!   and is kept while the slot waits for a reconnect.

use crate::hud::{Control, GlyphSet, PartyView};
use crate::input::{RawInput, STICK_DEADZONE, SlotInputs};
use crate::party::{HeroControl, HeroId, PadId, PartyState, PartyTuning, PlayerSlot, StartOutcome};

/// A pad connected or disconnected (from the Bevy connection messages).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadEvent {
    Connected { pad: PadId, vendor_id: Option<u16> },
    Disconnected { pad: PadId },
}

/// The buttons the scene reads, by logical action. The adapter owns the
/// physical mapping (South, West, North, LeftTrigger, Select, Start).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PadButtons {
    pub jump: bool,
    /// PrimaryAttack (Square/X).
    pub attack: bool,
    /// PhysicalAttack (Triangle/Y).
    pub physical: bool,
    /// SwapCharacter (L1/LB).
    pub swap: bool,
    /// Restart (Select).
    pub restart: bool,
    /// Start / Menu / Options.
    pub start: bool,
}

impl PadButtons {
    fn or(self, other: PadButtons) -> PadButtons {
        PadButtons {
            jump: self.jump || other.jump,
            attack: self.attack || other.attack,
            physical: self.physical || other.physical,
            swap: self.swap || other.swap,
            restart: self.restart || other.restart,
            start: self.start || other.start,
        }
    }
}

/// One rendered frame of one pad. `move_x` is the horizontal axis with the
/// stick or the d-pad already resolved by the adapter. `pressed` is the
/// just-pressed edge of this frame (the fixed step may run less often than the
/// frame loop, so edges are latched until [`PadSlots::take_inputs`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PadFrame {
    pub pad: PadId,
    pub vendor_id: Option<u16>,
    pub move_x: f32,
    pub held: PadButtons,
    pub pressed: PadButtons,
}

/// What a frame's pad input asks of the application (not of the simulation).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PadCommands {
    /// P1's pad pressed Start: quit (the legacy Start behaviour).
    pub exit: bool,
    /// An owning pad pressed Select: restart.
    pub restart: bool,
    /// A pad joined or reclaimed a slot with Start this frame.
    pub joined: Option<PlayerSlot>,
}

#[derive(Debug, Default, Clone, Copy)]
struct SlotState {
    move_x: f32,
    held: PadButtons,
    /// Edges latched since the last `take_inputs`.
    taps: PadButtons,
}

/// Slot ownership plus the per-slot pad input state.
#[derive(Debug, Clone)]
pub struct PadSlots {
    party: PartyState,
    /// Every pad seen and not yet disconnected, with its vendor id.
    known: Vec<(PadId, Option<u16>)>,
    slots: [SlotState; 2],
    glyphs: [GlyphSet; 2],
}

impl Default for PadSlots {
    fn default() -> Self {
        Self::new(PartyState::default())
    }
}

impl PadSlots {
    pub fn new(party: PartyState) -> Self {
        Self {
            party,
            known: Vec::new(),
            slots: [SlotState::default(); 2],
            glyphs: [GlyphSet::default(); 2],
        }
    }

    pub fn with_tuning(tuning: PartyTuning) -> Self {
        Self::new(PartyState::new(tuning))
    }

    pub fn party(&self) -> &PartyState {
        &self.party
    }

    /// The party for the solo swap and `hero_inputs` (the caller owns when to
    /// swap, since the hurt state lives in the simulation).
    pub fn party_mut(&mut self) -> &mut PartyState {
        &mut self.party
    }

    /// D6: an owning pad is gone; the simulation must not tick.
    pub fn is_paused(&self) -> bool {
        self.party.is_paused()
    }

    /// The first slot still waiting for a pad (what the HUD asks to reconnect).
    pub fn waiting_slot(&self) -> Option<PlayerSlot> {
        self.party.vacated_slots().first().copied()
    }

    /// The glyph set of `slot`'s own pad (Xbox until it has one). Kept while
    /// the slot waits for a reconnect, so the plate does not flicker.
    pub fn glyphs(&self, slot: PlayerSlot) -> GlyphSet {
        self.glyphs[slot.index()]
    }

    /// What the HUD draws: real ownership and the glyph family of each
    /// hero's own pad (an AI-driven hero shows P1's set, which it never draws),
    /// and the controller a disconnect pause waits for.
    pub fn hud_view(&self) -> PartyView {
        let control = |hero: HeroId| match self.party.hero_control(hero) {
            HeroControl::Player(PlayerSlot::P1) => (Control::Player(1), PlayerSlot::P1),
            HeroControl::Player(PlayerSlot::P2) => (Control::Player(2), PlayerSlot::P2),
            HeroControl::Uncontrolled => (Control::Ai, PlayerSlot::P1),
        };
        let (carl, carl_slot) = control(HeroId::Carl);
        let (donut, donut_slot) = control(HeroId::Donut);
        PartyView {
            carl,
            donut,
            carl_glyphs: self.glyphs(carl_slot),
            donut_glyphs: self.glyphs(donut_slot),
            reconnect: self.waiting_slot().map(|slot| slot.index() as u8 + 1),
            donut_chips: self.party.tuning().donut_ability_chips(),
        }
    }

    fn vendor_of(&self, pad: PadId) -> Option<u16> {
        self.known
            .iter()
            .find(|(id, _)| *id == pad)
            .and_then(|(_, vendor)| *vendor)
    }

    fn refresh_glyphs(&mut self) {
        for slot in PlayerSlot::ALL {
            if let Some(pad) = self.party.pad_of(slot) {
                self.glyphs[slot.index()] = GlyphSet::for_vendor_id(self.vendor_of(pad));
            }
        }
    }

    fn remember(&mut self, pad: PadId, vendor_id: Option<u16>) {
        match self.known.iter_mut().find(|(id, _)| *id == pad) {
            Some(entry) => entry.1 = vendor_id,
            None => self.known.push((pad, vendor_id)),
        }
    }

    /// Apply a connection event.
    pub fn apply(&mut self, event: PadEvent) {
        match event {
            PadEvent::Connected { pad, vendor_id } => {
                self.remember(pad, vendor_id);
                self.party.pad_connected(pad);
            }
            PadEvent::Disconnected { pad } => {
                let slot = self.party.slot_of(pad);
                self.known.retain(|(id, _)| *id != pad);
                if self.party.pad_disconnected(pad)
                    && let Some(slot) = slot
                {
                    // Nothing sticks: the vacated slot reads as idle.
                    self.slots[slot.index()] = SlotState::default();
                }
            }
        }
        self.refresh_glyphs();
    }

    /// Feed one pad's state for this rendered frame. A pad not seen before is
    /// registered first (a pad present at start-up may not announce itself).
    /// Input from a pad that owns no slot is dropped, except its Start press.
    pub fn sample(&mut self, frame: PadFrame) -> PadCommands {
        if !self.known.iter().any(|(id, _)| *id == frame.pad) {
            self.apply(PadEvent::Connected {
                pad: frame.pad,
                vendor_id: frame.vendor_id,
            });
        }
        let mut commands = PadCommands::default();
        if frame.pressed.start {
            match self.party.pad_start_pressed(frame.pad) {
                StartOutcome::OwnerPause(PlayerSlot::P1) => commands.exit = true,
                StartOutcome::OwnerPause(PlayerSlot::P2) | StartOutcome::Ignored => {}
                StartOutcome::Joined(slot) => commands.joined = Some(slot),
            }
            self.refresh_glyphs();
        }
        let Some(slot) = self.party.slot_of(frame.pad) else {
            return commands;
        };
        let state = &mut self.slots[slot.index()];
        state.move_x = if frame.move_x.is_finite() {
            frame.move_x
        } else {
            0.0
        };
        state.held = frame.held;
        state.taps = state.taps.or(frame.pressed);
        commands.restart = frame.pressed.restart;
        commands
    }

    /// One slot's raw input for a fixed tick, and the latched edges consumed.
    /// `swap` is the **press edge** (what `PartyState::request_swap` takes),
    /// not the held state.
    fn slot_raw(state: &SlotState) -> RawInput {
        RawInput {
            move_x: if state.move_x.abs() > STICK_DEADZONE {
                state.move_x
            } else {
                0.0
            },
            jump: state.held.jump || state.taps.jump,
            attack: state.held.attack || state.taps.attack,
            physical: state.held.physical || state.taps.physical,
            swap: state.taps.swap,
            ..RawInput::NONE
        }
    }

    /// The per-slot raw inputs for one fixed tick. `keyboard` is merged into
    /// P1 (the keyboard's move wins when pressed; buttons are ORed). The
    /// latched edges are consumed; a paused party reads as idle on both slots
    /// (nothing sticks across the pause).
    // The merge below ends in `..RawInput::NONE` on purpose: CF02-A adds a field
    // (`down`) that must default to off here until the adapter fills it.
    #[allow(clippy::needless_update)]
    pub fn take_inputs(&mut self, keyboard: RawInput) -> SlotInputs {
        let paused = self.party.is_paused();
        if keyboard.move_x != 0.0
            || keyboard.jump
            || keyboard.attack
            || keyboard.physical
            || keyboard.swap
        {
            self.party.note_keyboard_p1();
        }
        let mut out = [RawInput::default(); 2];
        for slot in PlayerSlot::ALL {
            let i = slot.index();
            let pad = Self::slot_raw(&self.slots[i]);
            self.slots[i].taps = PadButtons::default();
            out[i] = if slot == PlayerSlot::P1 {
                RawInput {
                    move_x: if keyboard.move_x != 0.0 {
                        keyboard.move_x
                    } else {
                        pad.move_x
                    },
                    jump: keyboard.jump || pad.jump,
                    attack: keyboard.attack || pad.attack,
                    physical: keyboard.physical || pad.physical,
                    ability: keyboard.ability || pad.ability,
                    pause: keyboard.pause || pad.pause,
                    swap: keyboard.swap || pad.swap,
                    ..RawInput::NONE
                }
            } else {
                pad
            };
        }
        if paused {
            [RawInput::default(); 2]
        } else {
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::party::{HeroControl, HeroId, PartyMode};

    const A: PadId = PadId(10);
    const B: PadId = PadId(11);
    const C: PadId = PadId(12);
    const SONY: Option<u16> = Some(crate::hud::SONY_VENDOR_ID);
    const MS: Option<u16> = Some(0x045e);

    fn connect(slots: &mut PadSlots, pad: PadId, vendor_id: Option<u16>) {
        slots.apply(PadEvent::Connected { pad, vendor_id });
    }

    fn frame(pad: PadId) -> PadFrame {
        PadFrame {
            pad,
            vendor_id: None,
            move_x: 0.0,
            held: PadButtons::default(),
            pressed: PadButtons::default(),
        }
    }

    fn press_start(slots: &mut PadSlots, pad: PadId) -> PadCommands {
        let mut f = frame(pad);
        f.pressed.start = true;
        f.held.start = true;
        slots.sample(f)
    }

    fn two_player() -> PadSlots {
        let mut slots = PadSlots::default();
        connect(&mut slots, A, MS);
        connect(&mut slots, B, SONY);
        let joined = press_start(&mut slots, B);
        assert_eq!(joined.joined, Some(PlayerSlot::P2));
        slots
    }

    #[test]
    fn the_first_pad_is_p1_and_a_second_pad_is_not_yet_a_player() {
        let mut slots = PadSlots::default();
        connect(&mut slots, A, MS);
        connect(&mut slots, B, SONY);
        assert_eq!(slots.party().slot_of(A), Some(PlayerSlot::P1));
        assert_eq!(slots.party().slot_of(B), None);
        assert_eq!(slots.party().mode(), PartyMode::Solo);
        // An unowned pad's moves and buttons are dropped.
        let mut f = frame(B);
        f.move_x = 1.0;
        f.held.attack = true;
        f.pressed.attack = true;
        slots.sample(f);
        let inputs = slots.take_inputs(RawInput::default());
        assert_eq!(inputs, [RawInput::default(); 2]);
    }

    #[test]
    fn start_on_a_second_pad_joins_p2() {
        let slots = two_player();
        assert_eq!(slots.party().mode(), PartyMode::TwoPlayer);
        assert_eq!(slots.party().slot_of(B), Some(PlayerSlot::P2));
        assert_eq!(
            slots.party().hero_control(HeroId::Donut),
            HeroControl::Player(PlayerSlot::P2)
        );
    }

    #[test]
    fn two_pads_do_not_cross_talk() {
        let mut slots = two_player();
        let mut fa = frame(A);
        fa.move_x = -1.0;
        fa.held.jump = true;
        fa.pressed.jump = true;
        let mut fb = frame(B);
        fb.move_x = 0.9;
        fb.held.attack = true;
        fb.pressed.attack = true;
        slots.sample(fa);
        slots.sample(fb);
        let [p1, p2] = slots.take_inputs(RawInput::default());
        assert_eq!(p1.move_x, -1.0);
        assert!(p1.jump && !p1.attack);
        assert_eq!(p2.move_x, 0.9);
        assert!(p2.attack && !p2.jump);
    }

    #[test]
    fn a_third_pad_is_ignored_even_on_start() {
        let mut slots = two_player();
        connect(&mut slots, C, MS);
        let cmd = press_start(&mut slots, C);
        assert_eq!(cmd, PadCommands::default());
        assert_eq!(slots.party().slot_of(C), None);
        let mut f = frame(C);
        f.move_x = 1.0;
        f.held.jump = true;
        f.pressed.jump = true;
        slots.sample(f);
        assert_eq!(
            slots.take_inputs(RawInput::default()),
            [RawInput::default(); 2]
        );
    }

    #[test]
    fn the_keyboard_is_p1_only() {
        let mut slots = two_player();
        let keys = RawInput {
            move_x: 1.0,
            jump: true,
            ..RawInput::NONE
        };
        let [p1, p2] = slots.take_inputs(keys);
        assert!(p1.jump && p1.move_x == 1.0);
        assert_eq!(p2, RawInput::default());
    }

    #[test]
    fn pad_buttons_merge_with_the_keyboard_on_p1() {
        let mut slots = PadSlots::default();
        connect(&mut slots, A, MS);
        let mut f = frame(A);
        f.move_x = -0.8;
        f.held.attack = true;
        slots.sample(f);
        let keys = RawInput {
            jump: true,
            ..RawInput::NONE
        };
        let [p1, _] = slots.take_inputs(keys);
        assert!(p1.attack && p1.jump);
        assert_eq!(p1.move_x, -0.8, "no key pressed: the stick moves");
        let keys = RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        };
        let [p1, _] = slots.take_inputs(keys);
        assert_eq!(p1.move_x, 1.0, "a pressed key wins over the stick");
    }

    #[test]
    fn a_tap_between_ticks_is_not_lost_and_fires_once() {
        let mut slots = PadSlots::default();
        connect(&mut slots, A, MS);
        let mut f = frame(A);
        f.pressed.jump = true;
        slots.sample(f);
        // Released before the fixed step runs.
        slots.sample(frame(A));
        let [p1, _] = slots.take_inputs(RawInput::default());
        assert!(p1.jump, "the latched tap reaches the tick");
        let [p1, _] = slots.take_inputs(RawInput::default());
        assert!(!p1.jump, "and is consumed");
    }

    #[test]
    fn swap_is_the_press_edge_and_only_the_owner_pad_sends_it() {
        let mut slots = PadSlots::default();
        connect(&mut slots, A, MS);
        let mut f = frame(A);
        f.held.swap = true;
        f.pressed.swap = true;
        slots.sample(f);
        let [p1, _] = slots.take_inputs(RawInput::default());
        assert!(p1.swap);
        // Still held next tick: no new edge.
        let mut held = frame(A);
        held.held.swap = true;
        slots.sample(held);
        let [p1, _] = slots.take_inputs(RawInput::default());
        assert!(!p1.swap);
    }

    #[test]
    fn start_on_the_owner_pad_exits_only_for_p1() {
        let mut slots = two_player();
        assert!(press_start(&mut slots, A).exit);
        assert!(!press_start(&mut slots, B).exit, "P2's Start never quits");
    }

    #[test]
    fn start_on_a_pad_that_owns_nothing_never_quits() {
        // S2: a keyboard P1 and a friend's pad.
        let mut slots = PadSlots::default();
        let keys = RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        };
        slots.take_inputs(keys);
        connect(&mut slots, B, SONY);
        assert_eq!(slots.party().slot_of(B), None, "no implicit P1");
        let cmd = press_start(&mut slots, B);
        assert!(!cmd.exit, "joining is not quitting");
        assert_eq!(cmd.joined, Some(PlayerSlot::P2));
        assert_eq!(slots.party().mode(), PartyMode::TwoPlayer);
        // Both devices now play: the keyboard is P1, the pad is P2, and the
        // pad's Start still never quits.
        assert!(!press_start(&mut slots, B).exit);
        let mut f = frame(B);
        f.move_x = -1.0;
        slots.sample(f);
        let [p1, p2] = slots.take_inputs(keys);
        assert_eq!((p1.move_x, p2.move_x), (1.0, -1.0));

        // With a stranger pad beside the owner of P1, Start joins, not quits.
        let mut slots = PadSlots::default();
        connect(&mut slots, A, MS);
        connect(&mut slots, B, SONY);
        assert!(!press_start(&mut slots, B).exit);
        assert!(press_start(&mut slots, A).exit, "P1's own pad quits");
    }

    #[test]
    fn a_pad_that_connected_before_the_keyboard_keeps_p1() {
        let mut slots = PadSlots::default();
        connect(&mut slots, A, MS);
        slots.take_inputs(RawInput {
            jump: true,
            ..RawInput::NONE
        });
        assert_eq!(slots.party().slot_of(A), Some(PlayerSlot::P1));
        assert!(press_start(&mut slots, A).exit);
    }

    #[test]
    fn a_reconnecting_pad_returns_to_its_own_slot() {
        // S1 through the adapter: both pads drop, P2's pad comes back first.
        let mut slots = two_player();
        slots.apply(PadEvent::Disconnected { pad: A });
        slots.apply(PadEvent::Disconnected { pad: B });
        assert_eq!(slots.waiting_slot(), Some(PlayerSlot::P1));
        connect(&mut slots, B, SONY);
        assert_eq!(slots.party().slot_of(B), Some(PlayerSlot::P2));
        assert_eq!(slots.waiting_slot(), Some(PlayerSlot::P1));
        assert!(slots.is_paused());
        connect(&mut slots, A, MS);
        assert_eq!(slots.party().slot_of(A), Some(PlayerSlot::P1));
        assert!(!slots.is_paused());
        // P2's pad still drives Donut and A drives Carl, with the right glyphs.
        let mut fb = frame(B);
        fb.move_x = 0.9;
        slots.sample(fb);
        let [p1, p2] = slots.take_inputs(RawInput::default());
        assert_eq!((p1.move_x, p2.move_x), (0.0, 0.9));
        assert_eq!(slots.glyphs(PlayerSlot::P1), GlyphSet::Xbox);
        assert_eq!(slots.glyphs(PlayerSlot::P2), GlyphSet::DualSense);
    }

    #[test]
    fn select_on_an_owner_pad_restarts_but_not_on_a_stranger() {
        let mut slots = two_player();
        connect(&mut slots, C, MS);
        let mut f = frame(C);
        f.pressed.restart = true;
        assert!(!slots.sample(f).restart);
        let mut f = frame(B);
        f.pressed.restart = true;
        assert!(slots.sample(f).restart);
    }

    #[test]
    fn disconnecting_an_owner_pauses_and_clears_its_input() {
        let mut slots = two_player();
        let mut f = frame(B);
        f.move_x = 1.0;
        f.held.attack = true;
        f.pressed.attack = true;
        slots.sample(f);
        slots.apply(PadEvent::Disconnected { pad: B });
        assert!(slots.is_paused());
        assert_eq!(slots.waiting_slot(), Some(PlayerSlot::P2));
        assert_eq!(
            slots.take_inputs(RawInput::default()),
            [RawInput::default(); 2]
        );
        // Still paused and idle even if the keyboard is held.
        let keys = RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        };
        assert_eq!(slots.take_inputs(keys), [RawInput::default(); 2]);
    }

    #[test]
    fn a_stranger_disconnecting_does_not_pause() {
        let mut slots = PadSlots::default();
        connect(&mut slots, A, MS);
        connect(&mut slots, B, MS);
        slots.apply(PadEvent::Disconnected { pad: B });
        assert!(!slots.is_paused());
        assert_eq!(slots.waiting_slot(), None);
    }

    #[test]
    fn the_same_pad_reconnecting_resumes() {
        let mut slots = two_player();
        slots.apply(PadEvent::Disconnected { pad: A });
        assert!(slots.is_paused());
        assert_eq!(slots.waiting_slot(), Some(PlayerSlot::P1));
        connect(&mut slots, A, MS);
        assert!(!slots.is_paused());
        assert_eq!(slots.party().slot_of(A), Some(PlayerSlot::P1));
        assert_eq!(slots.party().slot_of(B), Some(PlayerSlot::P2));
        let mut f = frame(A);
        f.held.jump = true;
        slots.sample(f);
        assert!(slots.take_inputs(RawInput::default())[0].jump);
    }

    #[test]
    fn a_new_pad_reclaims_the_vacated_slot() {
        let mut slots = two_player();
        slots.apply(PadEvent::Disconnected { pad: B });
        // A pad that returns under a new id takes the waiting slot as soon as
        // it connects (a reconnect may bring a new id).
        connect(&mut slots, C, MS);
        assert!(!slots.is_paused());
        // Start from it afterwards is just the owner's normal press.
        let cmd = press_start(&mut slots, C);
        assert!(!cmd.exit, "P2's Start never quits");
        assert!(!slots.is_paused());
        assert_eq!(slots.party().slot_of(C), Some(PlayerSlot::P2));
    }

    #[test]
    fn a_pad_seen_only_in_a_frame_is_registered() {
        let mut slots = PadSlots::default();
        let mut f = frame(A);
        f.vendor_id = SONY;
        f.held.jump = true;
        slots.sample(f);
        assert_eq!(slots.party().slot_of(A), Some(PlayerSlot::P1));
        assert_eq!(slots.glyphs(PlayerSlot::P1), GlyphSet::DualSense);
        assert!(slots.take_inputs(RawInput::default())[0].jump);
    }

    #[test]
    fn glyphs_follow_only_the_owner_pad_of_each_slot() {
        let mut slots = PadSlots::default();
        connect(&mut slots, A, MS);
        assert_eq!(slots.glyphs(PlayerSlot::P1), GlyphSet::Xbox);
        // A stranger Sony pad changes nothing, not even on activity.
        connect(&mut slots, B, SONY);
        let mut f = frame(B);
        f.held.attack = true;
        f.pressed.attack = true;
        slots.sample(f);
        assert_eq!(slots.glyphs(PlayerSlot::P1), GlyphSet::Xbox);
        assert_eq!(slots.glyphs(PlayerSlot::P2), GlyphSet::Xbox);
        // Joining makes P2 follow B (Sony), P1 stays on A (Xbox).
        press_start(&mut slots, B);
        assert_eq!(slots.glyphs(PlayerSlot::P1), GlyphSet::Xbox);
        assert_eq!(slots.glyphs(PlayerSlot::P2), GlyphSet::DualSense);
    }

    #[test]
    fn a_vacated_slot_keeps_its_glyph_set_until_reclaimed() {
        let mut slots = two_player();
        slots.apply(PadEvent::Disconnected { pad: B });
        assert_eq!(slots.glyphs(PlayerSlot::P2), GlyphSet::DualSense);
        connect(&mut slots, C, MS);
        press_start(&mut slots, C);
        assert_eq!(slots.glyphs(PlayerSlot::P2), GlyphSet::Xbox);
    }

    #[test]
    fn the_hud_view_is_real_ownership_per_plate() {
        let mut slots = PadSlots::default();
        connect(&mut slots, A, SONY);
        let solo = slots.hud_view();
        assert_eq!((solo.carl, solo.donut), (Control::Player(1), Control::Ai));
        assert_eq!(solo.carl_glyphs, GlyphSet::DualSense);
        assert_eq!(solo.reconnect, None);
        assert!(solo.donut_chips, "Q10-01: a human-driven Donut shows chips");
        // Solo swap moves the P1 tag, and P1's glyphs, to Donut.
        assert!(slots.party_mut().request_swap(true, false));
        let swapped = slots.hud_view();
        assert_eq!(
            (swapped.carl, swapped.donut),
            (Control::Ai, Control::Player(1))
        );
        assert_eq!(swapped.donut_glyphs, GlyphSet::DualSense);
        // Two players: each plate shows its own pad's family.
        connect(&mut slots, B, MS);
        press_start(&mut slots, B);
        let duo = slots.hud_view();
        assert_eq!(
            (duo.carl, duo.donut),
            (Control::Player(1), Control::Player(2))
        );
        assert_eq!(duo.carl_glyphs, GlyphSet::DualSense);
        assert_eq!(duo.donut_glyphs, GlyphSet::Xbox);
        // A disconnect asks for that controller, lowest slot first.
        slots.apply(PadEvent::Disconnected { pad: B });
        assert_eq!(slots.hud_view().reconnect, Some(2));
        slots.apply(PadEvent::Disconnected { pad: A });
        assert_eq!(slots.hud_view().reconnect, Some(1));
        connect(&mut slots, A, SONY);
        assert_eq!(slots.hud_view().reconnect, Some(2));
        connect(&mut slots, B, MS);
        assert_eq!(slots.hud_view().reconnect, None);
    }

    #[test]
    fn solo_swap_goes_through_the_party() {
        let mut slots = PadSlots::default();
        connect(&mut slots, A, MS);
        let mut f = frame(A);
        f.pressed.swap = true;
        slots.sample(f);
        let inputs = slots.take_inputs(RawInput::default());
        assert!(slots.party_mut().request_swap(inputs[0].swap, false));
        assert_eq!(slots.party().controlled_hero(), HeroId::Donut);
        let hero = slots.party().hero_inputs(inputs);
        assert_eq!(hero[0], RawInput::default());
        assert!(!hero[1].swap, "the party strips the swap bit");
    }
}
