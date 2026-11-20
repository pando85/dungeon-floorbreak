//! Pure mapping from Donut's mana and moves to the hybrid-HUD views
//! (HUD-DEBT-02, WORK-023).
//!
//! Bevy-free and stateless except for [`DepletedFlash`], which is presentation
//! only. It builds the existing [`crate::hud`] types ([`ManaView`],
//! [`AbilityRow`], [`DonutMagic`]) without owning any of the HUD: the real HUD
//! plate calls [`donut_ability_row`] for Donut's chips and
//! [`magic_of`] / [`donut_magic`] for her mana, from the real pool
//! (`HudRuntime::set_donut_mana`). The private `donut_abilities` row of
//! `hud.rs` is only the `--hud-preview` fixture row.
//!
//! Rules (`docs/02-art/donut-moveset-q10.md` section 4 and
//! `docs/01-design/donut-mana-v0-proposal.md` section 4):
//!
//! - The fill is whole segments: `(mana * 94 + max / 2) / max` in integers.
//! - Chips appear only when `donut_moves` is on **and** `melee_live` is true.
//! - Pre-biscuit: `SWAT` and `POUNCE`. After it: `MISSILE` and `POUNCE`, with
//!   `MISSILE` muted at 0 mana. With the playtest `empty_press: "swat"` the
//!   0-mana chip is an unmuted `SWAT`. `POUNCE` is never muted.
//! - An AI-controlled plate shows `COMPANION`.

use crate::donut_mana::{DonutMana, EmptyPress, ManaTuning};
use crate::hud::{
    AbilityRow, Chip, Control, DonutMagic, LabelId, LogicalAction, MANA_BAR_INNER_W,
    MANA_FLASH_TICKS, ManaView, SpecialSlot,
};

/// `round(mana / mana_max * 94)` in integers: no float ties at 2 and 6.
/// 0, 12, 24, 35, 47, 59, 71, 82, 94 for `mana_max = 8`.
pub fn mana_fill_px(mana: u8, mana_max: u8) -> u8 {
    if mana_max == 0 {
        return 0;
    }
    let mana = u32::from(mana.min(mana_max));
    let max = u32::from(mana_max);
    let fill = (mana * MANA_BAR_INNER_W as u32 + max / 2) / max;
    fill as u8
}

/// The one-time depleted outline flash age: `Some(0)` on the tick the pool
/// reaches 0, then counting up while it stays at 0, `None` otherwise. It never
/// reaches the simulation or the hash.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DepletedFlash {
    age: Option<u8>,
}

impl DepletedFlash {
    /// Advance one tick with the pool's current mana.
    pub fn step(&mut self, mana: u8) -> Option<u8> {
        self.age = if mana == 0 {
            match self.age {
                // Saturate just past the flash so the age never wraps.
                Some(age) => Some(age.saturating_add(1).min(MANA_FLASH_TICKS)),
                None => Some(0),
            }
        } else {
            None
        };
        self.age
    }

    pub const fn age(&self) -> Option<u8> {
        self.age
    }
}

/// `RealHudInputs::donut_magic`: `PreBiscuit` while locked, the whole-segment
/// view once unlocked.
pub fn donut_magic(mana: &DonutMana, tuning: &ManaTuning, depleted_age: Option<u8>) -> DonutMagic {
    magic_of(
        mana.unlocked().then_some((mana.mana(), tuning.mana_max)),
        depleted_age,
    )
}

/// [`donut_magic`] from the plain numbers the HUD runtime holds: `(mana,
/// mana_max)` while unlocked, `None` while locked.
pub fn magic_of(mana: Option<(u8, u8)>, depleted_age: Option<u8>) -> DonutMagic {
    let Some((mana, mana_max)) = mana else {
        return DonutMagic::PreBiscuit;
    };
    let fill_px = mana_fill_px(mana, mana_max);
    DonutMagic::Unlocked(ManaView {
        fill_px,
        depleted_age: if fill_px == 0 { depleted_age } else { None },
    })
}

/// Whether chips can show at all: Q10-01 and the scene's real melee gate.
/// `PartyTuning::donut_ability_chips` is derived from `donut_moves`; a test
/// pins the two equal at integration.
pub const fn chips_live(donut_moves: bool, melee_live: bool) -> bool {
    donut_moves && melee_live
}

/// Donut's ability row (section 4 of the Q10 spec).
pub fn donut_ability_row(
    control: Control,
    magic: DonutMagic,
    empty_press: EmptyPress,
    donut_moves: bool,
    melee_live: bool,
) -> AbilityRow {
    match control {
        Control::Ai => AbilityRow::AiCompanion,
        Control::Player(_) if !chips_live(donut_moves, melee_live) => AbilityRow::NoActions {
            special: SpecialSlot::Locked,
        },
        Control::Player(_) => {
            let (label, muted) = match magic {
                DonutMagic::PreBiscuit => (LabelId::Swat, false),
                DonutMagic::Unlocked(mana) if mana.fill_px > 0 => (LabelId::Missile, false),
                DonutMagic::Unlocked(_) => match empty_press {
                    EmptyPress::Dry => (LabelId::Missile, true),
                    EmptyPress::Swat => (LabelId::Swat, false),
                },
            };
            AbilityRow::Chips {
                a: Chip {
                    action: LogicalAction::Primary,
                    label,
                    muted,
                },
                b: Chip {
                    action: LogicalAction::Physical,
                    label: LabelId::Pounce,
                    muted: false,
                },
                special: SpecialSlot::Locked,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::donut_mana::DONUT_HERO;

    fn tuning() -> ManaTuning {
        ManaTuning::pinned(true).unwrap()
    }

    #[test]
    fn the_fill_table_for_0_to_8_is_pinned() {
        let table: Vec<u8> = (0..=8).map(|m| mana_fill_px(m, 8)).collect();
        assert_eq!(table, [0, 12, 24, 35, 47, 59, 71, 82, 94]);
        // Whole segments: strictly increasing, one step per charge, full is 94.
        assert!(table.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(mana_fill_px(8, 8), MANA_BAR_INNER_W as u8);
        // Above max clamps, a zero max is empty.
        assert_eq!(mana_fill_px(20, 8), 94);
        assert_eq!(mana_fill_px(3, 0), 0);
    }

    #[test]
    fn the_integer_rule_matches_the_hud_float_rule_including_the_ties() {
        // 2/8 and 6/8 of 94 are exactly 23.5 and 70.5: both rules round up.
        for m in 0..=8u32 {
            let float = (m as f32 / 8.0 * 94.0).round() as u32;
            assert_eq!(u32::from(mana_fill_px(m as u8, 8)), float, "m = {m}");
        }
    }

    #[test]
    fn locked_pool_is_pre_biscuit_and_unlocked_pool_is_a_whole_segment_view() {
        let t = tuning();
        let locked = DonutMana::locked();
        assert_eq!(donut_magic(&locked, &t, None), DonutMagic::PreBiscuit);
        let mut m = DonutMana::locked();
        m.grant_unlock(&t, 0, DONUT_HERO);
        assert_eq!(
            donut_magic(&m, &t, None),
            DonutMagic::Unlocked(ManaView {
                fill_px: 94,
                depleted_age: None
            })
        );
        for tick in 1..=5 {
            m.release(&t, tick);
        }
        assert_eq!(
            donut_magic(&m, &t, Some(3)),
            DonutMagic::Unlocked(ManaView {
                fill_px: 35,
                depleted_age: None
            }),
            "the flash age only shows at 0"
        );
        for tick in 6..=8 {
            m.release(&t, tick);
        }
        assert_eq!(
            donut_magic(&m, &t, Some(0)),
            DonutMagic::Unlocked(ManaView {
                fill_px: 0,
                depleted_age: Some(0)
            })
        );
    }

    #[test]
    fn the_depleted_flash_starts_at_zero_counts_and_clears_on_a_charge() {
        let mut f = DepletedFlash::default();
        assert_eq!(f.step(3), None);
        assert_eq!(f.step(0), Some(0));
        assert_eq!(f.step(0), Some(1));
        for _ in 0..20 {
            f.step(0);
        }
        assert_eq!(f.age(), Some(MANA_FLASH_TICKS), "saturates, never wraps");
        assert_eq!(f.step(1), None);
        assert_eq!(f.step(0), Some(0), "a new depletion flashes again");
    }

    fn chips(row: AbilityRow) -> (Chip, Chip) {
        let AbilityRow::Chips { a, b, special } = row else {
            panic!("expected chips, got {row:?}");
        };
        assert_eq!(special, SpecialSlot::Locked);
        (a, b)
    }

    fn mana_view(fill_px: u8) -> DonutMagic {
        DonutMagic::Unlocked(ManaView {
            fill_px,
            depleted_age: None,
        })
    }

    #[test]
    fn chips_follow_the_biscuit_and_mana_and_pounce_is_never_muted() {
        let p2 = Control::Player(2);
        let (a, b) = chips(donut_ability_row(
            p2,
            DonutMagic::PreBiscuit,
            EmptyPress::Dry,
            true,
            true,
        ));
        assert_eq!((a.label, a.muted), (LabelId::Swat, false));
        assert_eq!(
            (a.action, b.action),
            (LogicalAction::Primary, LogicalAction::Physical)
        );
        assert_eq!((b.label, b.muted), (LabelId::Pounce, false));

        let (a, b) = chips(donut_ability_row(
            p2,
            mana_view(47),
            EmptyPress::Dry,
            true,
            true,
        ));
        assert_eq!((a.label, a.muted), (LabelId::Missile, false));
        assert_eq!((b.label, b.muted), (LabelId::Pounce, false));

        let (a, b) = chips(donut_ability_row(
            p2,
            mana_view(0),
            EmptyPress::Dry,
            true,
            true,
        ));
        assert_eq!(
            (a.label, a.muted),
            (LabelId::Missile, true),
            "muted at 0 mana"
        );
        assert_eq!((b.label, b.muted), (LabelId::Pounce, false));
    }

    #[test]
    fn the_swat_empty_press_variant_shows_an_unmuted_swat_at_zero() {
        let (a, _) = chips(donut_ability_row(
            Control::Player(1),
            mana_view(0),
            EmptyPress::Swat,
            true,
            true,
        ));
        assert_eq!((a.label, a.muted), (LabelId::Swat, false));
        // With mana left it is still the missile.
        let (a, _) = chips(donut_ability_row(
            Control::Player(1),
            mana_view(12),
            EmptyPress::Swat,
            true,
            true,
        ));
        assert_eq!(a.label, LabelId::Missile);
    }

    #[test]
    fn no_chips_unless_donut_moves_and_melee_are_live_and_the_ai_plate_says_companion() {
        let no_actions = AbilityRow::NoActions {
            special: SpecialSlot::Locked,
        };
        for magic in [DonutMagic::PreBiscuit, mana_view(0), mana_view(94)] {
            assert_eq!(
                donut_ability_row(Control::Player(2), magic, EmptyPress::Dry, false, true),
                no_actions,
                "donut_moves off (no POUNCE chip while the move is not live)"
            );
            assert_eq!(
                donut_ability_row(Control::Player(2), magic, EmptyPress::Dry, true, false),
                no_actions,
                "the story corridor ignores melee"
            );
            assert_eq!(
                donut_ability_row(Control::Ai, magic, EmptyPress::Dry, true, true),
                AbilityRow::AiCompanion
            );
            assert_eq!(
                donut_ability_row(Control::Ai, magic, EmptyPress::Dry, false, false),
                AbilityRow::AiCompanion
            );
        }
        assert!(chips_live(true, true));
        assert!(!chips_live(true, false) && !chips_live(false, true) && !chips_live(false, false));
    }
}
