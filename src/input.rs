//! Device-independent gameplay commands used by agents, replay tests and UI.

pub const STICK_DEADZONE: f32 = 0.18;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct RawInput {
    pub move_x: f32,
    pub jump: bool,
    pub attack: bool,
    pub ability: bool,
    pub pause: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Actions {
    pub move_x: f32,
    pub jump: bool,
    pub attack: bool,
    pub ability: bool,
    pub pause: bool,
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
            ability: raw.ability,
            pause: raw.pause,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct InputFrame {
    pub held: Actions,
    pub jump_pressed: bool,
    pub attack_pressed: bool,
    pub ability_pressed: bool,
    pub pause_pressed: bool,
}

impl InputFrame {
    pub fn advance(&mut self, raw: RawInput) {
        let next = Actions::from(raw);
        self.jump_pressed = next.jump && !self.held.jump;
        self.attack_pressed = next.attack && !self.held.attack;
        self.ability_pressed = next.ability && !self.held.ability;
        self.pause_pressed = next.pause && !self.held.pause;
        self.held = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadzone_and_clamping_are_deterministic() {
        assert_eq!(Actions::from(RawInput { move_x: 0.10, ..RawInput::default() }).move_x, 0.0);
        assert_eq!(Actions::from(RawInput { move_x: 1.7, ..RawInput::default() }).move_x, 1.0);
        assert_eq!(Actions::from(RawInput { move_x: -1.8, ..RawInput::default() }).move_x, -1.0);
        assert_eq!(Actions::from(RawInput { move_x: f32::NAN, ..RawInput::default() }).move_x, 0.0);
    }

    #[test]
    fn held_buttons_do_not_repeat_press_edges() {
        let mut frame = InputFrame::default();
        let pressed = RawInput { jump: true, attack: true, ..RawInput::default() };
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
    fn simultaneous_actions_have_independent_edges() {
        let mut frame = InputFrame::default();
        frame.advance(RawInput { jump: true, ..RawInput::default() });
        frame.advance(RawInput { jump: true, ability: true, ..RawInput::default() });
        assert!(!frame.jump_pressed);
        assert!(frame.ability_pressed);
    }
}
