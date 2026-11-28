//! HUD-ART-02: the Bevy-free keyboard binding table (spec section 3).
//!
//! One `const` table is the single source for two readers: the `floor01_scene`
//! keyboard adapter maps each [`KeyName`] to a Bevy `KeyCode` and reads its
//! bindings from here, and the HUD shows the matching [`KeyCap`] when the
//! keyboard glyph set is active ([`crate::hud::GlyphSet::Keyboard`]). A rebinding
//! therefore changes the glyph with it. Presentation and input mapping only: no
//! gameplay type reads this table, and the simulation sees the same `RawInput`
//! as before.

use crate::hud::{Glyph, LogicalAction};

/// The keyboard actions that have a key cap. Restart (`R`) and quit (`Esc`) are
/// not HUD actions and get no cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Primary,
    Physical,
    Special,
    Swap,
    Jump,
    MoveLeft,
    MoveRight,
    Down,
}

/// A physical keyboard key the table names (the adapter maps it to its engine key code).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyName {
    KeyJ,
    KeyK,
    KeyL,
    KeyQ,
    Space,
    ArrowLeft,
    ArrowRight,
    ArrowDown,
    KeyA,
    KeyD,
    KeyS,
    KeyE,
}

/// GUILD-VERB-01 (GUILDV-AD2): the keyboard's Interact key. The contract fixes only R1/RB; `E` is
/// free (J, K, L, Q, Space, the arrows, A, D and S are bound, R and Esc are the scene's). It has
/// no row in [`KEY_BINDINGS`] because a row needs a HUD key cap and no prompt is drawn yet
/// (GUILDV-Q2); the adapter reads this constant directly.
pub const INTERACT_KEY: KeyName = KeyName::KeyE;

/// The cap the HUD draws for an action (one of the `Glyph::Key*` glyphs).
pub type KeyCap = Glyph;

/// One row: the action, the cap the HUD shows, and the keys that trigger it.
/// The **first** key is the primary (the one the cap shows); the others are
/// alternates without a glyph.
pub type KeyBinding = (KeyAction, KeyCap, &'static [KeyName]);

/// The binding table (spec section 3).
pub const KEY_BINDINGS: [KeyBinding; 8] = [
    (KeyAction::Primary, Glyph::KeyJ, &[KeyName::KeyJ]),
    (KeyAction::Physical, Glyph::KeyK, &[KeyName::KeyK]),
    (KeyAction::Special, Glyph::KeyL, &[KeyName::KeyL]),
    (KeyAction::Swap, Glyph::KeyQ, &[KeyName::KeyQ]),
    (KeyAction::Jump, Glyph::KeySpace, &[KeyName::Space]),
    (
        KeyAction::MoveLeft,
        Glyph::KeyLeft,
        &[KeyName::ArrowLeft, KeyName::KeyA],
    ),
    (
        KeyAction::MoveRight,
        Glyph::KeyRight,
        &[KeyName::ArrowRight, KeyName::KeyD],
    ),
    (
        KeyAction::Down,
        Glyph::KeyDown,
        &[KeyName::ArrowDown, KeyName::KeyS],
    ),
];

/// The keys bound to `action` (primary first).
pub fn keys_of(action: KeyAction) -> &'static [KeyName] {
    KEY_BINDINGS
        .iter()
        .find(|(a, _, _)| *a == action)
        .map_or(&[], |(_, _, keys)| *keys)
}

/// The cap the HUD shows for `action`.
pub fn cap_of(action: KeyAction) -> KeyCap {
    KEY_BINDINGS
        .iter()
        .find(|(a, _, _)| *a == action)
        .map(|(_, cap, _)| *cap)
        .expect("every key action has a row")
}

/// The key action behind a HUD chip action (Primary, Physical, Special).
pub const fn key_action_of(action: LogicalAction) -> KeyAction {
    match action {
        LogicalAction::Primary => KeyAction::Primary,
        LogicalAction::Physical => KeyAction::Physical,
        LogicalAction::Special => KeyAction::Special,
    }
}

/// The cap the keyboard glyph set shows for a HUD chip action, read from the table.
pub fn cap_for_chip(action: LogicalAction) -> KeyCap {
    cap_of(key_action_of(action))
}

/// Every action of the table, in table order.
pub fn all_actions() -> [KeyAction; 8] {
    KEY_BINDINGS.map(|(a, _, _)| a)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::GlyphSet;

    #[test]
    fn every_action_has_exactly_one_row_and_one_distinct_cap() {
        let actions = all_actions();
        for (i, a) in actions.iter().enumerate() {
            assert_eq!(actions.iter().filter(|b| *b == a).count(), 1, "{i}");
        }
        let caps: Vec<_> = KEY_BINDINGS.iter().map(|(_, c, _)| *c).collect();
        for (i, c) in caps.iter().enumerate() {
            assert!(!caps[i + 1..].contains(c), "{c:?} on two rows");
        }
        assert!(KEY_BINDINGS.iter().all(|(_, _, keys)| !keys.is_empty()));
    }

    #[test]
    fn the_interact_key_is_free_of_every_table_row() {
        assert_eq!(INTERACT_KEY, KeyName::KeyE);
        assert!(
            KEY_BINDINGS
                .iter()
                .all(|(_, _, keys)| !keys.contains(&INTERACT_KEY))
        );
    }

    #[test]
    fn no_key_is_bound_to_two_actions() {
        let all: Vec<KeyName> = KEY_BINDINGS
            .iter()
            .flat_map(|(_, _, keys)| keys.iter().copied())
            .collect();
        for (i, k) in all.iter().enumerate() {
            assert!(!all[i + 1..].contains(k), "{k:?} bound twice");
        }
    }

    #[test]
    fn every_hud_action_has_one_cap_and_the_keyboard_set_reads_it_from_the_table() {
        for action in [
            LogicalAction::Primary,
            LogicalAction::Physical,
            LogicalAction::Special,
        ] {
            assert_eq!(GlyphSet::Keyboard.glyph(action), cap_for_chip(action));
        }
        assert_eq!(cap_of(KeyAction::Primary), Glyph::KeyJ);
        assert_eq!(cap_of(KeyAction::Physical), Glyph::KeyK);
        assert_eq!(cap_of(KeyAction::Special), Glyph::KeyL);
        assert_eq!(cap_of(KeyAction::Swap), Glyph::KeyQ);
        assert_eq!(cap_of(KeyAction::Jump), Glyph::KeySpace);
        assert_eq!(keys_of(KeyAction::MoveLeft)[0], KeyName::ArrowLeft);
        assert_eq!(keys_of(KeyAction::MoveRight)[1], KeyName::KeyD);
    }

    #[test]
    fn the_primary_key_is_the_one_the_cap_names() {
        let letter = |k: KeyName| match k {
            KeyName::KeyJ => Some('J'),
            KeyName::KeyK => Some('K'),
            KeyName::KeyL => Some('L'),
            KeyName::KeyQ => Some('Q'),
            _ => None,
        };
        for (action, cap, keys) in KEY_BINDINGS {
            let want = match cap {
                Glyph::KeyJ => Some('J'),
                Glyph::KeyK => Some('K'),
                Glyph::KeyL => Some('L'),
                Glyph::KeyQ => Some('Q'),
                _ => None,
            };
            assert_eq!(letter(keys[0]), want, "{action:?}");
        }
    }
}
