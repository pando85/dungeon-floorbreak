//! Deterministic, engine-neutral action snapshots for the first technical spike.
//! Rendering and physical input devices must not be authoritative gameplay state.

/// Raw frontend input (keyboard, gamepad or deterministic replay fixture).
#[derive(Clone, Copy, Debug, Default)]
pub struct RawInput {
    pub move_x: f32,
    pub move_y: f32,
    pub aim_x: f32,
    pub aim_y: f32,
    pub jump: bool,
    pub attack: bool,
    pub special: bool,
    pub interact: bool,
    pub swap: bool,
}

/// One action snapshot per fixed simulation tick, independent of Bevy types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerAction {
    pub tick: u64,
    pub player_slot: u8,
    pub move_axis: (i16, i16),
    pub aim_axis: (i16, i16),
    pub jump_pressed: bool,
    pub attack_pressed: bool,
    pub special_pressed: bool,
    pub interact_pressed: bool,
    pub swap_pressed: bool,
}

impl PlayerAction {
    pub fn from_raw(tick: u64, player_slot: u8, raw: RawInput) -> Self {
        Self {
            tick,
            player_slot,
            move_axis: (quantize(raw.move_x), quantize(raw.move_y)),
            aim_axis: (quantize(raw.aim_x), quantize(raw.aim_y)),
            jump_pressed: raw.jump,
            attack_pressed: raw.attack,
            special_pressed: raw.special,
            interact_pressed: raw.interact,
            swap_pressed: raw.swap,
        }
    }
}

/// Deadzone and integer quantization: finite, stable, replay-friendly input values.
fn quantize(value: f32) -> i16 {
    if !value.is_finite() || value.abs() < 0.2 {
        return 0;
    }
    (value.clamp(-1.0, 1.0) * 1000.0).round() as i16
}

/// Stable FNV-1a digest for a sequence of action snapshots.
/// This is a diagnostic replay checksum, not a security hash.
pub fn replay_digest(frames: impl IntoIterator<Item = RawInput>) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for (tick, raw) in frames.into_iter().enumerate() {
        let a = PlayerAction::from_raw(tick as u64, 0, raw);
        let values = [
            a.tick as i64,
            i64::from(a.player_slot),
            i64::from(a.move_axis.0),
            i64::from(a.move_axis.1),
            i64::from(a.aim_axis.0),
            i64::from(a.aim_axis.1),
            i64::from(a.jump_pressed),
            i64::from(a.attack_pressed),
            i64::from(a.special_pressed),
            i64::from(a.interact_pressed),
            i64::from(a.swap_pressed),
        ];
        for value in values {
            for b in value.to_le_bytes() {
                hash ^= u64::from(b);
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadzone_and_nonfinite_values_are_zero() {
        let a = PlayerAction::from_raw(
            3,
            0,
            RawInput { move_x: 0.19, move_y: f32::NAN, aim_x: f32::INFINITY, ..RawInput::default() },
        );
        assert_eq!(a.move_axis, (0, 0));
        assert_eq!(a.aim_axis, (0, 0));
    }

    #[test]
    fn axes_clamp_and_quantize() {
        let a = PlayerAction::from_raw(
            8,
            1,
            RawInput { move_x: -4.0, move_y: 0.73, aim_x: 2.0, ..RawInput::default() },
        );
        assert_eq!(a.move_axis, (-1000, 730));
        assert_eq!(a.aim_axis, (1000, 0));
        assert_eq!((a.tick, a.player_slot), (8, 1));
    }

    #[test]
    fn button_edges_are_preserved_without_bevy() {
        let a = PlayerAction::from_raw(
            4,
            0,
            RawInput { jump: true, attack: true, special: true, interact: false, swap: true, ..RawInput::default() },
        );
        assert!(a.jump_pressed && a.attack_pressed && a.special_pressed && a.swap_pressed);
        assert!(!a.interact_pressed);
    }

    #[test]
    fn synthetic_action_replay_is_deterministic() {
        let fixture = [
            RawInput { move_x: 1.0, ..RawInput::default() },
            RawInput { move_x: 1.0, jump: true, ..RawInput::default() },
            RawInput { move_x: 0.0, attack: true, ..RawInput::default() },
        ];
        assert_eq!(replay_digest(fixture), replay_digest(fixture));
        assert_ne!(replay_digest(fixture), replay_digest([RawInput::default(); 3]));
    }
}
