//! Device-independent gameplay commands used by agents, replay tests and UI.
//!
//! The logical actions follow `docs/01-design/mvp-controller-contract.md`:
//! `attack` is PrimaryAttack (Square/X), `physical` is PhysicalAttack
//! (Triangle/Y: Carl's kick) and `ability` is EquippedSpecial (Circle/B, only
//! after its story unlock, so it does nothing in the Floor 1 combat). Physical
//! button codes live in the device adapters, never here.

pub const STICK_DEADZONE: f32 = 0.18;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct RawInput {
    pub move_x: f32,
    pub jump: bool,
    pub attack: bool,
    /// PhysicalAttack (Triangle/Y): Carl's kick.
    pub physical: bool,
    /// EquippedSpecial (Circle/B): unlock-gated, no effect in Floor 1 combat.
    pub ability: bool,
    pub pause: bool,
    /// SwapCharacter (L1/LB): solo only. The party layer (`crate::party`) consumes
    /// it and strips it before the per-hero inputs reach the simulation.
    pub swap: bool,
    /// COMBAT-FUN-02 (CF02-Q01): Down is held (D-pad down, or the left stick down
    /// past the threshold with the vertical axis dominant, resolved by the device
    /// adapter). It is a direction, not a button: only Carl's Physical press under
    /// `cf02_rules` reads it (Down + Physical is the stomp, or the dive in the
    /// air); every other consumer ignores it.
    pub down: bool,
}

impl RawInput {
    /// No input: every field zero (what `Default` returns, usable in `const fn`).
    pub const NONE: RawInput = RawInput {
        move_x: 0.0,
        jump: false,
        attack: false,
        physical: false,
        ability: false,
        pause: false,
        swap: false,
        down: false,
    };
}

/// One raw input per local player slot: index 0 is P1 (keyboard plus the first
/// pad), index 1 is P2 (the second pad, joined with Start). See `crate::party`.
pub type SlotInputs = [RawInput; 2];

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Actions {
    pub move_x: f32,
    pub jump: bool,
    pub attack: bool,
    /// PhysicalAttack (Triangle/Y): Carl's kick.
    pub physical: bool,
    /// EquippedSpecial (Circle/B): unlock-gated, no effect in Floor 1 combat.
    pub ability: bool,
    pub pause: bool,
    pub swap: bool,
    /// Down is held (CF02-Q01); a direction with no press edge.
    pub down: bool,
}

impl From<RawInput> for Actions {
    fn from(raw: RawInput) -> Self {
        let x = if raw.move_x.is_finite() {
            raw.move_x.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        Self {
            move_x: if x.abs() <= STICK_DEADZONE { 0.0 } else { x },
            jump: raw.jump,
            attack: raw.attack,
            physical: raw.physical,
            ability: raw.ability,
            pause: raw.pause,
            swap: raw.swap,
            down: raw.down,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct InputFrame {
    pub held: Actions,
    pub jump_pressed: bool,
    pub attack_pressed: bool,
    pub physical_pressed: bool,
    pub ability_pressed: bool,
    pub pause_pressed: bool,
    pub swap_pressed: bool,
}

impl InputFrame {
    pub fn advance(&mut self, raw: RawInput) {
        let next = Actions::from(raw);
        self.jump_pressed = next.jump && !self.held.jump;
        self.attack_pressed = next.attack && !self.held.attack;
        self.physical_pressed = next.physical && !self.held.physical;
        self.ability_pressed = next.ability && !self.held.ability;
        self.pause_pressed = next.pause && !self.held.pause;
        self.swap_pressed = next.swap && !self.held.swap;
        self.held = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadzone_and_clamping_are_deterministic() {
        fn normalized_axis(value: f32) -> f32 {
            Actions::from(RawInput {
                move_x: value,
                ..RawInput::default()
            })
            .move_x
        }
        assert_eq!(normalized_axis(0.10), 0.0);
        assert_eq!(normalized_axis(1.7), 1.0);
        assert_eq!(normalized_axis(-1.8), -1.0);
        assert_eq!(normalized_axis(f32::NAN), 0.0);
    }

    #[test]
    fn held_buttons_do_not_repeat_press_edges() {
        let mut frame = InputFrame::default();
        let pressed = RawInput {
            jump: true,
            attack: true,
            ..RawInput::default()
        };
        frame.advance(pressed);
        assert!(frame.jump_pressed && frame.attack_pressed);
        frame.advance(pressed);
        assert!(!frame.jump_pressed && !frame.attack_pressed);
        frame.advance(RawInput::default());
        assert!(!frame.held.jump);
        frame.advance(pressed);
        assert!(frame.jump_pressed && frame.attack_pressed);
    }

    #[test]
    fn physical_attack_and_equipped_special_are_separate_actions() {
        let mut frame = InputFrame::default();
        frame.advance(RawInput {
            physical: true,
            ..RawInput::default()
        });
        assert!(frame.physical_pressed && !frame.ability_pressed && !frame.attack_pressed);
        frame.advance(RawInput {
            physical: true,
            ..RawInput::default()
        });
        assert!(!frame.physical_pressed, "a held button never repeats");
        frame.advance(RawInput {
            ability: true,
            ..RawInput::default()
        });
        assert!(frame.ability_pressed && !frame.physical_pressed && !frame.held.physical);
        let actions = Actions::from(RawInput {
            physical: true,
            ..RawInput::default()
        });
        assert!(actions.physical && !actions.ability && !actions.attack);
    }

    #[test]
    fn swap_is_its_own_action_with_a_press_edge() {
        let mut frame = InputFrame::default();
        let swap = RawInput {
            swap: true,
            ..RawInput::default()
        };
        frame.advance(swap);
        assert!(frame.swap_pressed && !frame.attack_pressed && !frame.jump_pressed);
        frame.advance(swap);
        assert!(!frame.swap_pressed, "a held button never repeats");
        assert!(Actions::from(swap).swap);
    }

    #[test]
    fn simultaneous_actions_have_independent_edges() {
        let mut frame = InputFrame::default();
        frame.advance(RawInput {
            jump: true,
            ..RawInput::default()
        });
        frame.advance(RawInput {
            jump: true,
            ability: true,
            ..RawInput::default()
        });
        assert!(!frame.jump_pressed);
        assert!(frame.ability_pressed);
    }

    #[test]
    fn none_is_the_default_input() {
        assert_eq!(RawInput::NONE, RawInput::default());
    }

    #[test]
    fn down_is_a_held_direction_without_a_press_edge() {
        let mut frame = InputFrame::default();
        let down = RawInput {
            down: true,
            physical: true,
            ..RawInput::NONE
        };
        frame.advance(down);
        assert!(frame.held.down && frame.physical_pressed);
        frame.advance(down);
        assert!(frame.held.down && !frame.physical_pressed);
        frame.advance(RawInput::NONE);
        assert!(!frame.held.down);
        assert!(Actions::from(down).down);
    }
}
