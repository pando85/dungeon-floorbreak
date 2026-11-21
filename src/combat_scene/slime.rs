//! HAZ-A2: the slime as an ENEMY-SLOT enemy (`docs/01-design/enemy-gameplay-slime-rot.md`
//! sections 4, 8 and 19). The domain is [`crate::slime`]; this module is the
//! adapter: one [`SlotEnemy`], one registry row ([`build`], registered in
//! [`crate::scene_enemy::SLOT_KINDS`]) and nothing in `combat_scene.rs`,
//! `companion/**` or `contacts_cf02.rs`.
//!
//! Adopted defaults of this step (`HAZA2-AD*`, listed in the PR):
//!
//! * AD1 the slime's target id is `SLIME_ID_BASE + roster index`, and its
//!   `HeroBlow` source is [`crate::slime::slime_source`] of the **roster
//!   index** (the instance in the high half), so two slimes of one room
//!   never share a source;
//! * AD2 the slime picks its own target: the nearest eligible hero, kept until
//!   the other is [`TARGET_SWITCH_PX`] nearer (the goblins' hysteresis, without
//!   the solo AI bias);
//! * AD3 (replaced by the HAZ-Q14 rule, PR #214 F2) it starts a tell only with a
//!   free goblin token and the tell spacing ([`SlotStepCtx::token_free`]) and
//!   holds a token through its tell, launch and impact; **the goblins count
//!   that holder in their own approval** (`approved_goblins`), so goblins and
//!   slimes share one cap of two attacks in flight;
//! * AD4 a blast: the core and ring bands are lethal, the outer band deals the
//!   profile's outer damage with no state change and no launch (gel);
//! * AD5 its `boss` threat row is `Pound { x, facing, target: None }` while it
//!   rears (tell, launch, impact), the HAZ-Q44 value;
//! * AD6 with `hazq02_slime_enabled` false the slime is built defeated: no
//!   hurtbox, no step, no event, counted as cleared by the room goal;
//! * AD7 the lunge is never evaded here (the reflex callback is HAZ-B).

use crate::blast::BlastReach;
use crate::boss::{BossThreat, HeroView};
use crate::combat::{CombatEvent, MoveId};
use crate::party::HeroId;
use crate::prototype::Rect;
use crate::scene_enemy::{
    EnvSource, SLOT_ID_MIN, SlotBlast, SlotBuild, SlotDeathBody, SlotEnemy, SlotEnvHit, SlotError,
    SlotHit, SlotHitResult, SlotOut, SlotStepCtx,
};
use crate::slime::{Slime, SlimeCtx, SlimeTuning};

/// The registry key and roster kind name.
pub const KIND: &str = "slime";

/// The first slime target id: above every other slot id the room can hold (the
/// Hoarder's id is its own constant).
pub const SLIME_ID_BASE: u32 = SLOT_ID_MIN + 0x100;

/// HAZA2-AD2: the other hero must be this much nearer to take the target.
pub const TARGET_SWITCH_PX: f32 = 24.0;

/// The slot target id of the slime at roster index `index` (HAZA2-AD1).
pub const fn target_id(index: usize) -> u32 {
    SLIME_ID_BASE + index as u32
}

/// One slime in the slot.
#[derive(Debug, Clone)]
pub struct SlimeEnemy {
    slime: Slime,
    tuning: &'static SlimeTuning,
    id: u32,
    spawn_x: f32,
    spawn_facing: i8,
    target: Option<HeroId>,
    enabled: bool,
}

/// The registry builder: the roster entry's `x`, `facing` and `home`.
pub fn build(b: &SlotBuild<'_>) -> Result<Box<dyn SlotEnemy>, SlotError> {
    let tuning = crate::slime::active();
    let enc = b.encounter.ok_or_else(|| SlotError::Build {
        kind: KIND,
        reason: "a slime builds from a sidecar roster".into(),
    })?;
    let entry = enc
        .roster
        .get(b.entry.index)
        .ok_or_else(|| SlotError::Build {
            kind: KIND,
            reason: format!("roster index {} is out of range", b.entry.index),
        })?;
    let index = u8::try_from(b.entry.index).map_err(|_| SlotError::Build {
        kind: KIND,
        reason: "more than 255 roster entries".into(),
    })?;
    let mut slime = Slime::new(
        tuning,
        b.entry.x,
        b.room.floor_y,
        entry.facing,
        entry.home as f32,
    )
    .with_index(index);
    let enabled = tuning.enabled();
    if !enabled {
        // HAZA2-AD6: built defeated, inert.
        slime.hit(tuning, u8::MAX);
    }
    Ok(Box::new(SlimeEnemy {
        slime,
        tuning,
        id: target_id(b.entry.index),
        spawn_x: b.entry.x,
        spawn_facing: entry.facing,
        target: None,
        enabled,
    }))
}

impl SlimeEnemy {
    /// The domain slime (fixtures and the presenter read it).
    pub fn slime(&self) -> &Slime {
        &self.slime
    }

    fn pick_target(&mut self, heroes: &[HeroView; 2]) {
        let t = self.tuning;
        let eligible = |h: &HeroView| self.slime.eligible(t, h);
        let dist = |h: &HeroView| (h.x - self.slime.x).abs();
        let current = self
            .target
            .and_then(|id| heroes.iter().find(|h| h.id == id))
            .filter(|h| eligible(h));
        let best = heroes.iter().filter(|h| eligible(h)).min_by(|a, b| {
            dist(a)
                .total_cmp(&dist(b))
                .then(a.id.index().cmp(&b.id.index()))
        });
        self.target = match (current, best) {
            (Some(cur), Some(b)) if b.id != cur.id && dist(b) + TARGET_SWITCH_PX <= dist(cur) => {
                Some(b.id)
            }
            (Some(cur), _) => Some(cur.id),
            (None, b) => b.map(|h| h.id),
        };
    }

    fn defeat_result(&self) -> SlotHitResult {
        SlotHitResult {
            hp_left: u16::from(self.slime.health()),
            defeated: self.slime.is_defeated(),
        }
    }
}

impl SlotEnemy for SlimeEnemy {
    fn kind(&self) -> &'static str {
        KIND
    }

    fn target_id(&self) -> u32 {
        self.id
    }

    fn box_clone(&self) -> Box<dyn SlotEnemy> {
        Box::new(self.clone())
    }

    fn alive(&self) -> bool {
        !self.slime.is_defeated()
    }

    fn hp(&self) -> u16 {
        u16::from(self.slime.health())
    }

    fn hurt_rects(&self) -> Vec<Rect> {
        self.slime.hurtbox(self.tuning).into_iter().collect()
    }

    fn supplies_boss(&self) -> bool {
        true
    }

    /// HAZA2-AD5.
    fn threat(&self) -> Option<BossThreat> {
        (self.alive() && self.slime.phase(self.tuning).is_rearing()).then_some(BossThreat::Pound {
            x: self.slime.x,
            facing: self.slime.facing,
            target: None,
        })
    }

    fn holds_token(&self) -> bool {
        self.alive() && self.slime.phase(self.tuning).is_rearing()
    }

    fn death_body(&self) -> Option<SlotDeathBody> {
        Some(SlotDeathBody {
            x: self.slime.x,
            ground_y: self.slime.foot_y,
            facing: self.slime.facing,
            species: "slime",
        })
    }

    fn step(&mut self, ctx: &SlotStepCtx<'_>, out: &mut SlotOut) {
        if !self.alive() {
            return;
        }
        let t = self.tuning;
        self.pick_target(&ctx.heroes);
        // HAZ-Q06: awake only with its whole body in the camera view.
        let body = self.slime.hurtbox(t).unwrap_or(Rect {
            left: self.slime.x,
            bottom: 0.0,
            width: 0.0,
            height: 0.0,
        });
        let in_view = body.left >= ctx.view_x.0 && body.right() <= ctx.view_x.1;
        let sctx = SlimeCtx {
            heroes: &ctx.heroes,
            in_view,
            target: self.target,
            solids: ctx.solids,
        };
        let step = self.slime.step(t, &sctx, ctx.token_free, |_| false);
        out.tell_started |= step.tell_started;
        for hit in step.hits {
            out.blows.push(hit.blow);
        }
    }

    fn apply_hit(&mut self, hit: &SlotHit, tick: u64, out: &mut SlotOut) -> Option<SlotHitResult> {
        if !self.alive() {
            return None;
        }
        // Donut's swat is a glance (0 damage): gel, nothing happens.
        if hit.damage == 0 {
            return Some(self.defeat_result());
        }
        if hit.source == crate::scene_enemy::HitSource::CarlMelee(MoveId::PushKick) {
            // HAZ-Q12: the slide goes the way the kick travels.
            self.slime
                .push_kick(self.tuning, self.slime.x - f32::from(hit.facing));
        }
        let outcome = self
            .slime
            .hit(self.tuning, u8::try_from(hit.damage).unwrap_or(u8::MAX));
        if outcome.defeated {
            out.events.push(hit.defeat_event(tick, self.id));
        }
        Some(self.defeat_result())
    }

    /// HAZA2-AD4.
    fn apply_blast(
        &mut self,
        blast: &SlotBlast,
        tick: u64,
        out: &mut SlotOut,
    ) -> Option<SlotHitResult> {
        if !self.alive() {
            return None;
        }
        let damage = match blast.reach {
            BlastReach::PointBlank | BlastReach::Ring => u8::MAX,
            BlastReach::Outer => blast.charge.profile.outer_damage.max(1),
        };
        let outcome = self.slime.hit(self.tuning, damage);
        if outcome.defeated {
            out.events.push(CombatEvent::enemy_defeated(
                tick,
                self.id,
                blast.cause(),
                blast.charge.owner,
                None,
            ));
        }
        Some(self.defeat_result())
    }

    /// An env prop footprint: ordinary damage (acid never reaches a slime: the
    /// acid step passes only heroes and landing goblins).
    fn apply_env(
        &mut self,
        hit: &SlotEnvHit,
        tick: u64,
        out: &mut SlotOut,
    ) -> Option<SlotHitResult> {
        if !self.alive()
            || matches!(hit.source, EnvSource::Hazard(_))
                && hit.class == crate::env_hazard::HazardClass::Acid
        {
            return None;
        }
        let outcome = self.slime.hit(self.tuning, hit.damage);
        if outcome.defeated {
            out.events.push(CombatEvent::enemy_defeated(
                tick,
                self.id,
                hit.cause(),
                hit.initiator,
                None,
            ));
        }
        Some(self.defeat_result())
    }

    fn mix_into(&self, mix: &mut dyn FnMut(u64)) {
        mix(0x534c_4d45_0001); // "SLME"
        mix(u64::from(self.id));
        self.slime.mix_into(&mut |v| mix(v));
        mix(self.target.map_or(0, |h| h.index() as u64 + 1));
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

/// The spawn state facts (fixtures and tests).
impl SlimeEnemy {
    pub fn spawn(&self) -> (f32, i8) {
        (self.spawn_x, self.spawn_facing)
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }
}
