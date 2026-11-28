//! GUILD-VERB-01: the combat state's hooks of the Interact verb (`crate::interact`).
//!
//! A room with an interact site (`act1_guild`) installs the runtime with
//! [`CombatState::with_interact`]; every other room leaves it `None`, so a press does nothing and
//! no hash, event or input changes there.

use super::*;
use crate::interact::{HeroCtx, Interact, InteractSite, Verb};

impl CombatState {
    /// Install the room's interact site (setup only).
    #[must_use]
    pub fn with_interact(mut self, site: InteractSite) -> Self {
        self.interact = Some(Interact::new(site));
        self
    }

    /// The strongbox was opened by a hero's verb (presentation reads it).
    pub fn interact_box_open(&self) -> bool {
        self.interact.as_ref().is_some_and(Interact::box_open)
    }

    /// The hero's verb in progress and its frame index inside the verb's tag.
    pub fn interact_pose(&self, hero: HeroId) -> Option<(Verb, usize)> {
        self.interact.as_ref().and_then(|i| i.pose(hero))
    }

    /// The guide's `talk` key is held by an Interact press at sim tick `tick`.
    pub fn interact_guide_talking(&self, tick: u64) -> bool {
        self.interact
            .as_ref()
            .is_some_and(|i| i.guide_talking(tick))
    }

    /// One tick of the verb: the heroes' inputs back, a hero in a verb with none.
    pub(super) fn step_interact(
        &mut self,
        scene: &mut SceneSim,
        mut inputs: [RawInput; 2],
        live: bool,
        tick: u64,
    ) -> [RawInput; 2] {
        let Some(mut interact) = self.interact.take() else {
            return inputs;
        };
        let drive = *scene.drive();
        let world = &scene.world;
        let carl = HeroCtx {
            x: world.hero.x,
            facing: world.hero.facing,
            grounded: world.hero.grounded,
            can_start: !self.hero_defeated
                && self.hurt_ticks == 0
                && self.hero_hit_stop == 0
                && !world.hero.melee.busy(),
            interrupted: self.hero_defeated || self.hurt_ticks > 0,
            frozen: self.hero_hit_stop > 0,
            bot: drive.is_bot(HeroId::Carl),
            present: true,
        };
        let donut = HeroCtx {
            x: world.donut_body.x,
            facing: world.donut_body.facing,
            grounded: world.donut_body.grounded,
            can_start: !self.hero_defeated
                && self.donut.hurt_ticks == 0
                && self.donut.hit_stop == 0,
            interrupted: self.hero_defeated || self.donut.hurt_ticks > 0,
            frozen: self.donut.hit_stop > 0,
            bot: drive.is_bot(HeroId::Donut),
            present: live,
        };
        let out = interact.step(tick, [carl, donut], inputs);
        for hero in HeroId::ALL {
            let h = hero.index();
            if out.locked[h] {
                inputs[h] = RawInput::NONE;
            }
            let body = scene.world.body_mut(hero);
            if let Some(x) = out.snap_x[h] {
                body.x = x;
                body.vx = 0.0;
            }
            if let Some(facing) = out.face[h] {
                body.facing = facing;
            }
        }
        self.interact = Some(interact);
        inputs
    }
}
