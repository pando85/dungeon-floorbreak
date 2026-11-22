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
//!
//! DONUT-PROG-01 DP-D adds her special socket (DP-Q12, DP-Q33): [`blink_socket`]
//! maps the blink state to the padlock, the equipped chip, the cooling chip with
//! its cooldown underline, or the ready chip. Presentation only; it reads state.

use crate::donut_mana::{DonutMana, EmptyPress, ManaTuning};
use crate::hud::{
    AbilityRow, Chip, Control, DonutMagic, LabelId, LogicalAction, MANA_BAR_INNER_W,
    MANA_FLASH_TICKS, ManaView, SOCKET_W, SpecialSlot,
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
    special: SpecialSlot,
) -> AbilityRow {
    match control {
        Control::Ai => AbilityRow::AiCompanion,
        Control::Player(_) if !chips_live(donut_moves, melee_live) => {
            AbilityRow::NoActions { special }
        }
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
                special,
            }
        }
    }
}

/// DP-Q33: the width of the cooldown underline, `ceil(remaining / cooldown *
/// socket_w)` in integers, `0` when nothing remains (the special is ready).
pub fn cooldown_bar_px(remaining: u64, cooldown: u16) -> u8 {
    if remaining == 0 || cooldown == 0 {
        return 0;
    }
    let w = SOCKET_W as u64;
    let px = (remaining * w).div_ceil(u64::from(cooldown));
    px.min(w) as u8
}

/// The blink state a socket is drawn from (DP-Q33), plain data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlinkSocketInputs {
    /// She holds `special_short_blink`.
    pub holds: bool,
    /// The windup is running (the press to the arrival).
    pub winding_up: bool,
    /// Ticks of cooldown left after this tick (`ready_tick - tick`), 0 when none.
    pub cooling_left: u64,
    /// Her mana and the blink's cost, in charges.
    pub charges: u8,
    pub cost: u8,
    /// The cooldown's full length (`dpq24_cooldown_ticks`).
    pub cooldown_ticks: u16,
}

/// DP-Q33: the socket of Donut's plate. A padlock until she holds the blink; then
/// the Circle chip with the data label, muted while the windup runs, while it
/// cools down (with the underline) or while she holds fewer charges than the cost
/// (the zig-zag), and live otherwise.
pub fn blink_socket(i: &BlinkSocketInputs, label: LabelId) -> SpecialSlot {
    if !i.holds {
        return SpecialSlot::Locked;
    }
    let chip = |muted| Chip {
        action: LogicalAction::Special,
        label,
        muted,
    };
    let bar_px = cooldown_bar_px(i.cooling_left, i.cooldown_ticks);
    if bar_px > 0 {
        SpecialSlot::Cooling {
            chip: chip(true),
            bar_px,
        }
    } else {
        SpecialSlot::Equipped(chip(i.winding_up || i.charges < i.cost))
    }
}

/// [`blink_socket`] read from the combat state after the tick `tick` was stepped.
pub fn blink_socket_of(combat: &crate::combat_scene::CombatState, tick: u64) -> SpecialSlot {
    let tuning = crate::donut_prog::DonutProgTuning::active();
    let blink = combat.donut_blink();
    let inputs = BlinkSocketInputs {
        holds: combat.story_holds(crate::donut_prog::SPECIAL_SHORT_BLINK),
        winding_up: blink.windup().is_some(),
        cooling_left: blink
            .ready_tick()
            .map_or(0, |ready| ready.saturating_sub(tick)),
        charges: combat.donut_mana_view().map_or(0, |(mana, _)| mana),
        cost: tuning.dpq23_cost_charges,
        cooldown_ticks: tuning.dpq24_cooldown_ticks,
    };
    blink_socket(
        &inputs,
        LabelId::for_socket_text(&tuning.dpq33_socket_label).unwrap_or(LabelId::Blink),
    )
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
            SpecialSlot::Locked,
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
            SpecialSlot::Locked,
        ));
        assert_eq!((a.label, a.muted), (LabelId::Missile, false));
        assert_eq!((b.label, b.muted), (LabelId::Pounce, false));

        let (a, b) = chips(donut_ability_row(
            p2,
            mana_view(0),
            EmptyPress::Dry,
            true,
            true,
            SpecialSlot::Locked,
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
            SpecialSlot::Locked,
        ));
        assert_eq!((a.label, a.muted), (LabelId::Swat, false));
        // With mana left it is still the missile.
        let (a, _) = chips(donut_ability_row(
            Control::Player(1),
            mana_view(12),
            EmptyPress::Swat,
            true,
            true,
            SpecialSlot::Locked,
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
                donut_ability_row(
                    Control::Player(2),
                    magic,
                    EmptyPress::Dry,
                    false,
                    true,
                    SpecialSlot::Locked
                ),
                no_actions,
                "donut_moves off (no POUNCE chip while the move is not live)"
            );
            assert_eq!(
                donut_ability_row(
                    Control::Player(2),
                    magic,
                    EmptyPress::Dry,
                    true,
                    false,
                    SpecialSlot::Locked
                ),
                no_actions,
                "the story corridor ignores melee"
            );
            assert_eq!(
                donut_ability_row(
                    Control::Ai,
                    magic,
                    EmptyPress::Dry,
                    true,
                    true,
                    SpecialSlot::Locked
                ),
                AbilityRow::AiCompanion
            );
            assert_eq!(
                donut_ability_row(
                    Control::Ai,
                    magic,
                    EmptyPress::Dry,
                    false,
                    false,
                    SpecialSlot::Locked
                ),
                AbilityRow::AiCompanion
            );
        }
        assert!(chips_live(true, true));
        assert!(!chips_live(true, false) && !chips_live(false, true) && !chips_live(false, false));
    }

    fn socket(winding_up: bool, cooling_left: u64, charges: u8) -> SpecialSlot {
        blink_socket(
            &BlinkSocketInputs {
                holds: true,
                winding_up,
                cooling_left,
                charges,
                cost: 2,
                cooldown_ticks: 540,
            },
            LabelId::Blink,
        )
    }

    fn chip(muted: bool) -> Chip {
        Chip {
            action: LogicalAction::Special,
            label: LabelId::Blink,
            muted,
        }
    }

    /// DP-Q33: the underline is `ceil(remaining / 540 * 11)`, exact on both edges.
    #[test]
    fn the_cooldown_underline_widths_are_pinned() {
        let table: Vec<(u64, u8)> = [0u64, 1, 49, 50, 270, 490, 491, 539, 540]
            .into_iter()
            .map(|left| (left, cooldown_bar_px(left, 540)))
            .collect();
        assert_eq!(
            table,
            [
                (0, 0),
                (1, 1),
                (49, 1),
                (50, 2),
                (270, 6),
                (490, 10),
                (491, 11),
                (539, 11),
                (540, 11)
            ]
        );
        // Never wider than the socket, and a zero cooldown draws nothing.
        assert_eq!(cooldown_bar_px(5000, 540), 11);
        assert_eq!(cooldown_bar_px(3, 0), 0);
    }

    /// DP-Q33: locked, equipped, cooling at 0, 270 and 539 ticks left, and ready.
    #[test]
    fn the_blink_socket_is_locked_equipped_cooling_or_ready() {
        let locked = BlinkSocketInputs {
            holds: false,
            winding_up: false,
            cooling_left: 0,
            charges: 8,
            cost: 2,
            cooldown_ticks: 540,
        };
        assert_eq!(blink_socket(&locked, LabelId::Blink), SpecialSlot::Locked);
        // Ready: equipped and live, 2 charges are enough (the cost).
        assert_eq!(socket(false, 0, 8), SpecialSlot::Equipped(chip(false)));
        assert_eq!(socket(false, 0, 2), SpecialSlot::Equipped(chip(false)));
        // Below the cost: muted, no underline (the zig-zag draws).
        assert_eq!(socket(false, 0, 1), SpecialSlot::Equipped(chip(true)));
        // The windup: muted, no underline yet (the cooldown starts at the arrival).
        assert_eq!(socket(true, 0, 8), SpecialSlot::Equipped(chip(true)));
        // Cooling at 539, 270 and 1 ticks left, then ready at 0.
        for (left, bar_px) in [(539, 11), (270, 6), (1, 1)] {
            assert_eq!(
                socket(false, left, 8),
                SpecialSlot::Cooling {
                    chip: chip(true),
                    bar_px
                },
                "{left} ticks left"
            );
        }
        assert_eq!(socket(false, 0, 8), SpecialSlot::Equipped(chip(false)));
    }

    /// The socket rides on the chips and on the no-actions row, never on the AI
    /// plate.
    #[test]
    fn the_special_rides_on_both_player_rows() {
        let magic = mana_view(47);
        let slot = socket(false, 270, 8);
        match donut_ability_row(Control::Player(2), magic, EmptyPress::Dry, true, true, slot) {
            AbilityRow::Chips { special, .. } => assert_eq!(special, slot),
            other => panic!("{other:?}"),
        }
        assert_eq!(
            donut_ability_row(
                Control::Player(2),
                magic,
                EmptyPress::Dry,
                true,
                false,
                slot
            ),
            AbilityRow::NoActions { special: slot }
        );
        assert_eq!(
            donut_ability_row(Control::Ai, magic, EmptyPress::Dry, true, true, slot),
            AbilityRow::AiCompanion
        );
    }
}
