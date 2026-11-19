//! HUD-DEBT-06: the HUD accessibility settings, Bevy-free.
//!
//! Presentation only: nothing here reaches the simulation, the combat state or
//! a replay. Settings come from defaults, then the environment, then CLI
//! flags (the same layering as `AudioSettings`).
//!
//! Environment (`1`/`true`/`on`/`yes` enable, `0`/`false`/`off`/`no` disable,
//! anything else is ignored):
//! * `FLOORBREAK_HUD_REDUCED_MOTION`
//! * `FLOORBREAK_HUD_HIGH_CONTRAST`
//!
//! Flags: `--hud-reduced-motion[=on|off]`, `--hud-high-contrast[=on|off]`
//! (the bare form means `on`).
//!
//! Not available, on purpose (see `docs/03-tech/floor01-scene.md`, HUD
//! section): a larger HUD text size (the 640x360 spec rects have no room for
//! it, see `larger_text_does_not_fit_the_plate`), persistence (the repository has
//! no settings file yet) and a controller-reachable settings screen (the game
//! has no menu yet).

/// HUD options a player may need. Both default to off.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HudSettings {
    /// No notice or boss slides, blinks or flashes (spec section 5).
    pub reduced_motion: bool,
    /// All HUD text reaches 7:1, empty-state outlines 13:1, fills and track
    /// outlines at least 4.4:1 (floor01-scene.md, HUD accessibility settings).
    pub high_contrast: bool,
}

pub const ENV_REDUCED_MOTION: &str = "FLOORBREAK_HUD_REDUCED_MOTION";
pub const ENV_HIGH_CONTRAST: &str = "FLOORBREAK_HUD_HIGH_CONTRAST";

/// `on`/`off` style booleans; `None` for anything else.
pub fn parse_switch(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
    }
}

impl HudSettings {
    /// Defaults overridden by whatever `get` returns (an env lookup in the
    /// game, a map in tests). Unparsable values are ignored.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Self {
        let mut s = Self::default();
        if let Some(v) = get(ENV_REDUCED_MOTION).and_then(|v| parse_switch(&v)) {
            s.reduced_motion = v;
        }
        if let Some(v) = get(ENV_HIGH_CONTRAST).and_then(|v| parse_switch(&v)) {
            s.high_contrast = v;
        }
        s
    }

    pub fn from_env() -> Self {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Apply one CLI argument. `Ok(true)` when it was a HUD setting flag,
    /// `Ok(false)` when it is not one of ours, `Err` on a bad `=value`.
    pub fn apply_flag(&mut self, arg: &str) -> Result<bool, String> {
        for (name, slot) in [
            ("--hud-reduced-motion", &mut self.reduced_motion),
            ("--hud-high-contrast", &mut self.high_contrast),
        ] {
            if arg == name {
                *slot = true;
                return Ok(true);
            }
            if let Some(value) = arg
                .strip_prefix(name)
                .and_then(|rest| rest.strip_prefix('='))
            {
                *slot = parse_switch(value).ok_or_else(|| {
                    format!("bad value {value:?} for {name} (expected on or off)")
                })?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::HudFont;
    use crate::hud_draw::FrameId;
    use std::collections::HashMap;

    fn lookup(pairs: &[(&str, &str)]) -> HudSettings {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        HudSettings::from_lookup(|key| map.get(key).cloned())
    }

    #[test]
    fn defaults_are_off() {
        assert_eq!(lookup(&[]), HudSettings::default());
        assert!(!HudSettings::default().reduced_motion);
        assert!(!HudSettings::default().high_contrast);
    }

    #[test]
    fn the_environment_sets_each_switch_and_ignores_junk() {
        let s = lookup(&[(ENV_REDUCED_MOTION, "ON"), (ENV_HIGH_CONTRAST, " true ")]);
        assert!(s.reduced_motion && s.high_contrast);
        let s = lookup(&[(ENV_REDUCED_MOTION, "1")]);
        assert!(s.reduced_motion && !s.high_contrast);
        let s = lookup(&[(ENV_REDUCED_MOTION, "maybe"), (ENV_HIGH_CONTRAST, "")]);
        assert_eq!(s, HudSettings::default());
        let s = lookup(&[(ENV_HIGH_CONTRAST, "off")]);
        assert!(!s.high_contrast);
    }

    #[test]
    fn flags_override_the_environment_in_both_directions() {
        let mut s = lookup(&[(ENV_REDUCED_MOTION, "1"), (ENV_HIGH_CONTRAST, "0")]);
        assert_eq!(s.apply_flag("--hud-reduced-motion=off"), Ok(true));
        assert_eq!(s.apply_flag("--hud-high-contrast"), Ok(true));
        assert!(!s.reduced_motion && s.high_contrast);
        assert_eq!(s.apply_flag("--hud-high-contrast=no"), Ok(true));
        assert_eq!(s.apply_flag("--hud-reduced-motion=on"), Ok(true));
        assert!(s.reduced_motion && !s.high_contrast);
    }

    #[test]
    fn unknown_flags_pass_through_and_bad_values_fail() {
        let mut s = HudSettings::default();
        assert_eq!(s.apply_flag("--glyphs=xbox"), Ok(false));
        assert_eq!(s.apply_flag("--hud-text-size=2"), Ok(false));
        assert_eq!(s.apply_flag("--hud-high-contrast-x"), Ok(false));
        assert!(s.apply_flag("--hud-high-contrast=maybe").is_err());
        assert!(s.apply_flag("--hud-reduced-motion=").is_err());
        assert_eq!(s, HudSettings::default());
    }

    /// Why there is no text-size setting: at the next integer step (2x) the
    /// plate text no longer fits its spec rows. The numbers are the spec's
    /// section 2.1 offsets and section 4 cap heights.
    #[test]
    fn larger_text_does_not_fit_the_plate() {
        let (_, plate_h) = FrameId::PlateFrame.size();
        let inner_bottom = plate_h - 3; // 3 px ring
        // Jersey 10 name: caps top +4, the HP icon starts at +16.
        let (name_top, hp_row_top, jersey_caps) = (4, 16, 10);
        assert!(name_top + jersey_caps <= hp_row_top, "1x name fits");
        assert!(name_top + 2 * jersey_caps > hp_row_top, "2x name collides");
        // Silkscreen 5 px labels: caps top +37, the ring starts at inner_bottom.
        let (label_top, silk_caps) = (37, 5);
        assert!(label_top + silk_caps <= inner_bottom, "1x label fits");
        assert!(
            label_top + 2 * silk_caps > inner_bottom,
            "2x label hits the ring"
        );
        // Two chip columns start 53 px apart; "KICK" plus its glyph (14 px)
        // is 35 px at 1x and 56 px at 2x, so the chips would also collide.
        let kick = crate::hud::text_width_px(HudFont::SilkRegular, "KICK");
        assert!(14 + kick <= 53, "1x chip fits its column");
        assert!(14 + 2 * kick > 53, "2x chip overruns its column");
    }
}
