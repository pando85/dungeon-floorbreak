//! CARL-PROG-01 CP-A: the combat state's hooks into the story-unlock ledger.
//!
//! The model lives in [`crate::story_unlock`] (Bevy-free, no combat types);
//! this file only feeds it: the defeat of the boss as a data event, the heroes'
//! pivots as zone probes, the restart kinds, and turns each grant receipt into
//! one [`CombatEvent::CapabilityGranted`]. A run without a story flag has no
//! runtime (`story` is `None`), so none of this costs a tick or a hash word.

use super::CombatState;
use crate::boss::BOSS_ENEMY_ID;
use crate::combat::CombatEvent;
use crate::corridor::SceneSim;
use crate::donut_mana::RestartKind;
use crate::party::HeroId;
use crate::prototype::HERO_HALF_HEIGHT;
use crate::story_unlock::{
    CapabilityId, EventMatch, ReadThrough, StoryRuntime, StorySetup, UnlockLedger,
    holds_without_story,
};

impl CombatState {
    /// Declare the story the scene runs under: the capabilities it starts with
    /// and the triggers armed in it (see [`StorySetup`]). An inert setup keeps
    /// the default: no runtime, nothing hashed, nothing evented.
    ///
    /// # Panics
    ///
    /// When a pin, a start flag or a hold needs a read-through capability the
    /// scene does not have (a Donut capability in a scene where the biscuit is
    /// locked): the story order is broken by the scene's data, and the message
    /// names the pin (DP-Q06). Call it after `with_donut`.
    #[must_use]
    pub fn with_story(mut self, setup: StorySetup) -> Self {
        if let Err(error) = setup.check_read_through(&self.read_through()) {
            panic!("{error}");
        }
        self.story = StoryRuntime::new(&setup);
        self
    }

    /// The values of the capabilities that live elsewhere, read now.
    fn read_through(&self) -> ReadThrough {
        ReadThrough {
            primary_ranged: self.donut_kit.mana.unlocked(),
        }
    }

    /// Whether the capability is held right now. A stored capability asks the
    /// ledger; the biscuit (`primary_ranged`) is a read-through of
    /// `DonutMana::unlocked()` and nothing else (CP-Q45).
    pub fn story_holds(&self, id: CapabilityId) -> bool {
        let view = self.read_through();
        match &self.story {
            Some(story) => story.holds(id, &view),
            None => holds_without_story(id, &view),
        }
    }

    /// The ledger of a run under the story flags (`None` without them).
    pub fn story_ledger(&self) -> Option<&UnlockLedger> {
        self.story.as_ref().map(StoryRuntime::ledger)
    }

    /// Map a logged combat event to the data event ids the armed triggers wait
    /// for (`brute_boss_f01_defeated` is the boss's `EnemyDefeated`).
    pub(super) fn observe_story(&mut self, event: &CombatEvent) {
        let Some(story) = self.story.as_mut() else {
            return;
        };
        if let CombatEvent::EnemyDefeated { target, .. } = event
            && *target == BOSS_ENEMY_ID
        {
            for row in story.event_rows() {
                if row.matcher == EventMatch::EnemyDefeatedBoss {
                    story.observe(row.id.as_str());
                }
            }
        }
    }

    /// One tick of the triggers, after everything else has moved: a zone reads
    /// its hero's pivot (the same convention as the Guild zone: x and the foot
    /// line, whoever controls the hero), an event trigger reads what was logged
    /// this tick. Each grant is one event, in grant order, on this tick.
    pub(super) fn step_story(&mut self, scene: &SceneSim, tick: u64) {
        let view = self.read_through();
        let Some(story) = self.story.as_mut() else {
            return;
        };
        let pivot = |hero: HeroId| {
            let body = scene.world.body(hero);
            (body.x, body.y - HERO_HALF_HEIGHT)
        };
        let pivots = [pivot(HeroId::Carl), pivot(HeroId::Donut)];
        let receipts = story.step(tick, pivots, self.hero_defeated, &view);
        for receipt in receipts {
            self.log(CombatEvent::CapabilityGranted {
                tick,
                hero: receipt.hero,
                capability: receipt.capability.as_str(),
            });
        }
    }

    /// A retry or a checkpoint respawn keeps the grants; a fixture restart
    /// restores the declared start state (CP-Q07).
    pub(super) fn restart_story(&mut self, kind: RestartKind) {
        if let Some(story) = self.story.as_mut() {
            story.restart(kind.into());
        }
    }
}
