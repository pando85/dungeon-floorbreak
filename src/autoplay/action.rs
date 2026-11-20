//! ActionV1 and its mapping to the game's own [`RawInput`] (AP-Q03, AP-Q04).
//!
//! The adapter holds buttons as the controller would: it never creates press
//! edges itself, so the simulation's own edge detectors and input priority
//! apply unchanged.

use crate::input::RawInput;

use super::error::AutoplayError;

pub const ACTION_V1_LEN: usize = 6;

/// Cardinality of each ActionV1 component, in index order:
/// horizontal (0 left, 1 neutral, 2 right), jump, primary, physical,
/// special (reserved), solo swap (reserved).
const ACTION_V1_SIZES: [i64; ACTION_V1_LEN] = [3, 2, 2, 2, 2, 2];
const COMPONENT_NAMES: [&str; ACTION_V1_LEN] = [
    "horizontal",
    "jump",
    "primary",
    "physical",
    "special",
    "swap",
];

/// A validated ActionV1 value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionV1 {
    values: [u8; ACTION_V1_LEN],
}

impl ActionV1 {
    /// Neutral stick, nothing held.
    pub const NOOP: Self = Self {
        values: [1, 0, 0, 0, 0, 0],
    };

    /// Validate every component; nothing is clamped.
    pub fn new(values: [u8; ACTION_V1_LEN]) -> Result<Self, AutoplayError> {
        for (index, &value) in values.iter().enumerate() {
            check_component(index, i64::from(value))?;
        }
        Ok(Self { values })
    }

    /// Validate a dynamically sized action (for example from a Python binding).
    pub fn from_slice(values: &[i64]) -> Result<Self, AutoplayError> {
        if values.len() != ACTION_V1_LEN {
            return Err(AutoplayError::InvalidAction(format!(
                "expected {ACTION_V1_LEN} components, got {}",
                values.len()
            )));
        }
        let mut out = [0u8; ACTION_V1_LEN];
        for (index, &value) in values.iter().enumerate() {
            check_component(index, value)?;
            out[index] = value as u8;
        }
        Ok(Self { values: out })
    }

    pub fn as_array(&self) -> [u8; ACTION_V1_LEN] {
        self.values
    }

    pub fn horizontal(&self) -> u8 {
        self.values[0]
    }

    pub fn jump(&self) -> bool {
        self.values[1] == 1
    }

    pub fn primary(&self) -> bool {
        self.values[2] == 1
    }

    pub fn physical(&self) -> bool {
        self.values[3] == 1
    }

    /// Reserved: `ability` held. No Carl behaviour on this build.
    pub fn special(&self) -> bool {
        self.values[4] == 1
    }

    /// Reserved: `swap` held. Ignored by the single-hero fixture sim.
    pub fn swap(&self) -> bool {
        self.values[5] == 1
    }

    /// The game's own input for this action. Held buttons pass through so the
    /// simulation's press-edge logic and input priority decide what happens;
    /// `pause` is never a policy action.
    pub fn to_raw_input(&self) -> RawInput {
        // `pause` (and any field a later ruleset adds, such as Down) comes
        // from the neutral base: it is never a policy action (AP-Q04).
        RawInput {
            move_x: match self.values[0] {
                0 => -1.0,
                2 => 1.0,
                _ => 0.0,
            },
            jump: self.jump(),
            attack: self.primary(),
            physical: self.physical(),
            ability: self.special(),
            swap: self.swap(),
            ..RawInput::NONE
        }
    }
}

fn check_component(index: usize, value: i64) -> Result<(), AutoplayError> {
    let size = ACTION_V1_SIZES[index];
    if (0..size).contains(&value) {
        Ok(())
    } else {
        Err(AutoplayError::InvalidAction(format!(
            "component {index} ({}) is {value}, expected 0..{size}",
            COMPONENT_NAMES[index]
        )))
    }
}

/// Attempts at the reserved, unavailable actions (AP-Q03). Counted per
/// executed tick: `*_held_ticks` while the component is 1, `*_presses` on each
/// 0 to 1 transition. They never feed a reward.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UnavailableCounters {
    pub special_held_ticks: u64,
    pub special_presses: u64,
    pub swap_held_ticks: u64,
    pub swap_presses: u64,
}

impl UnavailableCounters {
    /// Account one executed tick; `previous` is the action of the tick before
    /// (`None` on the first tick of an episode).
    pub(crate) fn record(&mut self, previous: Option<ActionV1>, action: ActionV1) {
        let was_special = previous.is_some_and(|a| a.special());
        let was_swap = previous.is_some_and(|a| a.swap());
        if action.special() {
            self.special_held_ticks += 1;
            if !was_special {
                self.special_presses += 1;
            }
        }
        if action.swap() {
            self.swap_held_ticks += 1;
            if !was_swap {
                self.swap_presses += 1;
            }
        }
    }
}
