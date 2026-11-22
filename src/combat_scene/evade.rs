//! DONUT-MOBILITY-01 DM-B: the combat state's hooks into Donut's cat reflex.
//!
//! The model is [`crate::agility::Reflex`] (pure, ticks only, no RNG, no Bevy);
//! this file only feeds it: the per-tick arm, the reset events (the unlock, every
//! hurt she takes), and the check the three evadable sources run after their
//! guards (spec section 3, DM-Q12 to DM-Q22). A run before the biscuit never
//! arms, so none of this costs a hash word or an event.

use super::CombatState;
use crate::agility::{EvadeGate, EvadeKey, EvadeSource, Reflex, Verdict};
use crate::combat::CombatEvent;
use crate::corridor::SceneSim;
use crate::party::HeroId;

impl CombatState {
    /// The reflex state, for tests and the presentation (DM-C reads it
    /// read-only: the tail glint shows while [`Reflex::armed`]).
    pub fn reflex(&self) -> &Reflex {
        &self.reflex
    }

    /// Tests only: the reflex is armed at once (no event), as if its wait had
    /// elapsed, so a scenario can meet an armed Donut without 720 idle ticks.
    #[doc(hidden)]
    pub fn arm_reflex_for_test(&mut self) {
        self.reflex.reset(0);
        let _ = self.reflex.step(Some(0), true, 0);
    }

    /// DM-Q11, DM-Q12: the rearm of the live reflex in this run's tuning, or
    /// `None` when evasion is off or Donut has no body (no live party).
    fn reflex_rearm(scene: &SceneSim) -> Option<u32> {
        if scene.party_live() {
            scene.drive().tuning.donut_reflex.rearm()
        } else {
            None
        }
    }

    /// One fixed tick of the wait (DM-Q12): after the biscuit, the reflex arms
    /// on the first tick where `tick - last_reset_tick >= rearm`, and logs
    /// `EvadeArmed` once. Before the unlock nothing advances.
    pub(super) fn step_reflex(&mut self, scene: &SceneSim, tick: u64) {
        let unlocked = self.donut_kit.mana.unlocked();
        if self.reflex.step(Self::reflex_rearm(scene), unlocked, tick) {
            self.log(CombatEvent::EvadeArmed {
                tick,
                hero: HeroId::Donut,
            });
        }
    }

    /// DM-Q13: the events that reset the wait. The unlock tick starts it (a
    /// reflex is never ready at the moment of the unlock) and **every**
    /// `HeroHurt` she takes, from any source, evadable or not, restarts it
    /// (she is rattled). An evade and a respawn reset it where they happen; a
    /// restart in `reset_attempt`. A solo swap resets nothing: it changes who
    /// drives her body, not the body.
    pub(super) fn observe_reflex(&mut self, event: &CombatEvent) {
        match *event {
            CombatEvent::HeroHurt {
                tick,
                hero: HeroId::Donut,
                ..
            } => self.reflex.reset(tick),
            CombatEvent::Mana(crate::donut_mana::ManaEvent::UnlockGranted { tick, .. }) => {
                self.reflex.reset(tick);
            }
            _ => {}
        }
    }

    /// DM-Q19, DM-Q20: an evadable instance on Donut that already passed
    /// every existing guard (defeat, i-frames, the off-screen immunity, the
    /// box overlap). `(source, enemy, stamp)` is its [`EvadeKey`] identity and
    /// `hurt_source` the attacker's target id for the event. `Evaded` consumed
    /// the arm and logged `HeroEvaded`; `Passed` is the same instance seen
    /// again (no event, no new arm); `Hit` resolves as before.
    ///
    /// This is the only door to the reflex: a source with no [`EvadeSource`]
    /// variant never reaches it (DM-Q18, default never).
    pub(super) fn evade_check(
        &mut self,
        scene: &SceneSim,
        (source, enemy, stamp): (EvadeSource, u32, u64),
        hurt_source: u32,
        tick: u64,
    ) -> Verdict {
        let mut gate = self.evade_gate(scene);
        let verdict = gate.check(
            EvadeKey {
                source,
                enemy,
                stamp,
            },
            hurt_source,
            tick,
        );
        self.settle_evade_gate(gate);
        verdict
    }

    /// HAZ-B: the reflex lent to a slot enemy's step (the slime's lunge). Same
    /// condition as [`Self::evade_check`]: evasion on, a live Donut, the biscuit.
    pub(super) fn evade_gate(&self, scene: &SceneSim) -> EvadeGate {
        EvadeGate::new(
            Self::reflex_rearm(scene).is_some() && self.donut_kit.mana.unlocked(),
            self.reflex,
        )
    }

    /// HAZ-B: take the reflex back from a gate and log its evades, in order.
    pub(super) fn settle_evade_gate(&mut self, gate: EvadeGate) {
        let (reflex, evaded) = gate.into_parts();
        self.reflex = reflex;
        for (tick, source, kind) in evaded {
            self.log(CombatEvent::HeroEvaded {
                tick,
                hero: HeroId::Donut,
                source,
                kind,
            });
        }
    }
}

#[cfg(test)]
mod tests;
