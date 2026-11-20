//! CF02-I acting overlay selector (Q69).
//!
//! Presentation only: pure, Bevy-free and deterministic, and **not wired**
//! (CF02-I wires it into the clip selection). Nothing here reads or returns
//! anything the simulation could depend on: it takes a snapshot of what an actor
//! is doing this tick and names the layer to show, so an overlay can never delay
//! a move, jump or swap.
//!
//! Priority (Q69): death > hurt > move > morale reaction > acting overlay >
//! locomotion. An overlay shows only in idle or run and ends on any press on the
//! same tick. This module also ends an overlay whenever any other layer wins (a
//! higher layer interrupts it, it does not pause it), so an overlay never
//! reappears after a move.

use crate::cf02::Cf02Tuning;

/// The layer of the animation to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    Death,
    Hurt,
    Move,
    /// A goblin morale reaction (hesitate, panic, taunt, ...).
    MoraleReaction,
    Overlay,
    Locomotion,
}

/// What an actor's locomotion is on this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locomotion {
    Idle,
    Run,
    /// Airborne, landing, crouching, or anything else: no overlay.
    Other,
}

/// A snapshot of one actor on one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActingInput {
    pub dead: bool,
    pub hurt: bool,
    /// A move (or its recovery) is running.
    pub in_move: bool,
    /// A goblin morale reaction is running (always false for heroes).
    pub morale_reaction: bool,
    pub locomotion: Locomotion,
    /// Any press edge on this tick (attack, jump, swap, ...), whether or not it
    /// started anything.
    pub press_this_tick: bool,
}

/// `q69_acting_overlays`, effective only under `cf02_rules`.
pub fn overlays_enabled(t: &Cf02Tuning) -> bool {
    t.on(t.q69_acting_overlays)
}

/// The layer that wins when no overlay is running: the priority list without
/// the overlay rung.
pub fn base_layer(input: &ActingInput) -> Layer {
    if input.dead {
        Layer::Death
    } else if input.hurt {
        Layer::Hurt
    } else if input.in_move {
        Layer::Move
    } else if input.morale_reaction {
        Layer::MoraleReaction
    } else {
        Layer::Locomotion
    }
}

/// One actor's running overlay of kind `K` (the caller's own enum: flinch,
/// frustration, startle, smug, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Acting<K: Copy> {
    running: Option<(K, u8)>,
}

impl<K: Copy> Default for Acting<K> {
    fn default() -> Self {
        Self { running: None }
    }
}

/// What to show on one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection<K: Copy> {
    pub layer: Layer,
    /// The overlay to show; `Some` only when `layer` is [`Layer::Overlay`].
    pub overlay: Option<K>,
}

impl<K: Copy> Acting<K> {
    /// Start an overlay for `ticks` ticks. Refused (`false`) when overlays are
    /// off, `ticks` is 0, or one is already running.
    pub fn start(&mut self, t: &Cf02Tuning, kind: K, ticks: u8) -> bool {
        if !overlays_enabled(t) || ticks == 0 || self.running.is_some() {
            return false;
        }
        self.running = Some((kind, ticks));
        true
    }

    /// The running overlay's kind and the ticks left, if any.
    pub fn running(&self) -> Option<(K, u8)> {
        self.running
    }

    /// Select the layer for this tick and advance the overlay. A press on this
    /// tick ends the overlay at once: the same tick already shows the move or
    /// the locomotion. Any layer above the overlay, or a locomotion other than
    /// idle or run, ends it too.
    pub fn select(&mut self, t: &Cf02Tuning, input: &ActingInput) -> Selection<K> {
        let base = base_layer(input);
        let can_show = overlays_enabled(t)
            && base == Layer::Locomotion
            && !input.press_this_tick
            && matches!(input.locomotion, Locomotion::Idle | Locomotion::Run);
        match self.running {
            Some((kind, left)) if can_show => {
                self.running = (left > 1).then_some((kind, left - 1));
                Selection {
                    layer: Layer::Overlay,
                    overlay: Some(kind),
                }
            }
            _ => {
                self.running = None;
                Selection {
                    layer: base,
                    overlay: None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Kind {
        Flinch,
        Startle,
    }

    fn on() -> Cf02Tuning {
        let mut t = Cf02Tuning::pinned().expect("pinned tuning");
        t.cf02_rules = true;
        t
    }

    fn calm(locomotion: Locomotion) -> ActingInput {
        ActingInput {
            dead: false,
            hurt: false,
            in_move: false,
            morale_reaction: false,
            locomotion,
            press_this_tick: false,
        }
    }

    /// Every combination of the five flags and the three locomotions.
    fn all_inputs() -> Vec<ActingInput> {
        let mut v = Vec::new();
        for bits in 0..32u8 {
            for locomotion in [Locomotion::Idle, Locomotion::Run, Locomotion::Other] {
                v.push(ActingInput {
                    dead: bits & 1 != 0,
                    hurt: bits & 2 != 0,
                    in_move: bits & 4 != 0,
                    morale_reaction: bits & 8 != 0,
                    locomotion,
                    press_this_tick: bits & 16 != 0,
                });
            }
        }
        v
    }

    #[test]
    fn q69_priority_death_hurt_move_morale_overlay_locomotion() {
        let t = on();
        let mut running: Acting<Kind> = Acting::default();
        let mut input = calm(Locomotion::Idle);
        let expect = |input: &ActingInput, layer: Layer| {
            let mut a = Acting::default();
            assert!(a.start(&t, Kind::Flinch, 8));
            assert_eq!(a.select(&t, input).layer, layer, "{input:?}");
        };
        expect(&input, Layer::Overlay);
        input.morale_reaction = true;
        expect(&input, Layer::MoraleReaction);
        input.in_move = true;
        expect(&input, Layer::Move);
        input.hurt = true;
        expect(&input, Layer::Hurt);
        input.dead = true;
        expect(&input, Layer::Death);
        // Locomotion is the floor.
        assert_eq!(
            running.select(&t, &calm(Locomotion::Run)).layer,
            Layer::Locomotion
        );
        // The ordering of the enum is the priority.
        assert!(Layer::Death < Layer::Hurt);
        assert!(Layer::Hurt < Layer::Move);
        assert!(Layer::Move < Layer::MoraleReaction);
        assert!(Layer::MoraleReaction < Layer::Overlay);
        assert!(Layer::Overlay < Layer::Locomotion);
    }

    #[test]
    fn q69_every_combination_follows_the_priority_list() {
        let t = on();
        for input in all_inputs() {
            let mut a = Acting::default();
            assert!(a.start(&t, Kind::Startle, 12));
            let got = a.select(&t, &input).layer;
            let want = if input.dead {
                Layer::Death
            } else if input.hurt {
                Layer::Hurt
            } else if input.in_move {
                Layer::Move
            } else if input.morale_reaction {
                Layer::MoraleReaction
            } else if !input.press_this_tick && input.locomotion != Locomotion::Other {
                Layer::Overlay
            } else {
                Layer::Locomotion
            };
            assert_eq!(got, want, "{input:?}");
        }
    }

    #[test]
    fn q69_an_overlay_shows_only_in_idle_or_run() {
        let t = on();
        for (locomotion, shown) in [
            (Locomotion::Idle, true),
            (Locomotion::Run, true),
            (Locomotion::Other, false),
        ] {
            let mut a = Acting::default();
            a.start(&t, Kind::Flinch, 8);
            let s = a.select(&t, &calm(locomotion));
            assert_eq!(s.layer == Layer::Overlay, shown, "{locomotion:?}");
            assert_eq!(s.overlay, shown.then_some(Kind::Flinch));
        }
    }

    #[test]
    fn q69_a_press_always_ends_an_overlay_on_the_same_tick() {
        let t = on();
        for locomotion in [Locomotion::Idle, Locomotion::Run, Locomotion::Other] {
            let mut a = Acting::default();
            assert!(a.start(&t, Kind::Flinch, 8));
            // Two quiet ticks show it...
            if locomotion != Locomotion::Other {
                assert_eq!(a.select(&t, &calm(locomotion)).layer, Layer::Overlay);
                assert_eq!(a.select(&t, &calm(locomotion)).layer, Layer::Overlay);
            }
            // ...a press, even one that starts no move, ends it on that tick.
            let mut pressed = calm(locomotion);
            pressed.press_this_tick = true;
            let s = a.select(&t, &pressed);
            assert_eq!(s.layer, Layer::Locomotion);
            assert_eq!(s.overlay, None);
            assert_eq!(a.running(), None);
            // And it does not come back on the next quiet tick.
            assert_eq!(a.select(&t, &calm(locomotion)).layer, Layer::Locomotion);
        }
    }

    #[test]
    fn q69_a_press_that_starts_a_move_shows_the_move_not_the_overlay() {
        let t = on();
        let mut a = Acting::default();
        a.start(&t, Kind::Startle, 12);
        let mut started = calm(Locomotion::Idle);
        started.press_this_tick = true;
        started.in_move = true;
        assert_eq!(a.select(&t, &started).layer, Layer::Move);
        assert_eq!(a.running(), None, "the overlay is over, not paused");
    }

    #[test]
    fn q69_a_higher_layer_ends_the_overlay_it_does_not_pause_it() {
        let t = on();
        let mut a = Acting::default();
        a.start(&t, Kind::Flinch, 8);
        let mut hurt = calm(Locomotion::Idle);
        hurt.hurt = true;
        assert_eq!(a.select(&t, &hurt).layer, Layer::Hurt);
        assert_eq!(
            a.select(&t, &calm(Locomotion::Idle)).layer,
            Layer::Locomotion
        );
    }

    #[test]
    fn q69_an_overlay_lasts_exactly_its_ticks() {
        let t = on();
        for ticks in [1u8, 8, 12, 24] {
            let mut a = Acting::default();
            assert!(a.start(&t, Kind::Flinch, ticks));
            let shown = (0..40)
                .filter(|_| a.select(&t, &calm(Locomotion::Idle)).layer == Layer::Overlay)
                .count();
            assert_eq!(shown, usize::from(ticks));
            assert_eq!(a.running(), None);
        }
    }

    #[test]
    fn q69_start_is_refused_when_off_empty_or_busy() {
        let mut off = on();
        off.cf02_rules = false;
        let mut a = Acting::default();
        assert!(!a.start(&off, Kind::Flinch, 8), "master switch off");
        let mut sw = on();
        sw.q69_acting_overlays = false;
        assert!(!a.start(&sw, Kind::Flinch, 8), "acting_overlays off");
        assert!(!a.start(&on(), Kind::Flinch, 0), "no ticks");
        assert!(a.start(&on(), Kind::Flinch, 8));
        assert!(!a.start(&on(), Kind::Startle, 12), "one at a time");
        assert_eq!(a.running(), Some((Kind::Flinch, 8)));
    }

    #[test]
    fn q69_with_overlays_off_the_selection_is_the_base_layer_everywhere() {
        let mut off = on();
        off.q69_acting_overlays = false;
        for input in all_inputs() {
            let mut a: Acting<Kind> = Acting::default();
            let s = a.select(&off, &input);
            assert_eq!(s.layer, base_layer(&input));
            assert_eq!(s.overlay, None);
        }
    }

    #[test]
    fn q69_an_overlay_never_changes_anything_but_overlay_versus_locomotion() {
        // The presentation-only guarantee at the selector: for every snapshot,
        // running an overlay changes the layer only from Locomotion to Overlay.
        // It never outranks a move, a hurt, a death or a reaction, and a press
        // always shows what it would have shown without the overlay (so no
        // input is delayed by it).
        let t = on();
        for input in all_inputs() {
            let mut with = Acting::default();
            with.start(&t, Kind::Flinch, 8);
            let a = with.select(&t, &input).layer;
            let b = base_layer(&input);
            assert!(
                a == b || (a == Layer::Overlay && b == Layer::Locomotion),
                "{input:?}"
            );
            if input.press_this_tick {
                assert_eq!(a, b, "{input:?}");
            }
        }
    }

    #[test]
    fn q69_selection_is_deterministic() {
        let t = on();
        let run = || {
            let mut a = Acting::default();
            a.start(&t, Kind::Startle, 12);
            all_inputs()
                .iter()
                .map(|i| a.select(&t, i).layer)
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }
}
