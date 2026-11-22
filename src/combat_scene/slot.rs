//! ENEMY-SLOT: the combat state's hooks into the per-enemy update slot.
//!
//! The contract, the registry and the roster spawn are [`crate::scene_enemy`];
//! this file feeds the slot: the heroes' views, the solids, Carl's and Donut's
//! hits, the missiles; it turns an enemy's [`SlotOut`] into events, shifts and
//! hero blows through the existing `hurt_hero` path. The blast victims are in
//! `thrown.rs` (CP-B1's victim loop), the contact target in `contacts_cf02.rs`,
//! and the env prop and hazard victims in [`CombatState::slot_env_hits`].
//!
//! A state with no slot enemy returns at the first line of every function
//! below: nothing is stepped, evented or hashed, so every golden is
//! byte-identical (the empty slot is the legacy game).
//!
//! Tick order. Carl's strike (`strike_slot_enemies`) and Donut's swat, pounce
//! and missile (the hooks in `donut.rs`) land **before** the scene step, like
//! the boss' hits; the slot's own step runs **after** the goblins, llamas and
//! globs and **before** the contacts, the shore, the boss and the charges
//! (`step_slot_enemies`). A hit's follow-up outputs that need the scene (a
//! blow, a shift) wait in `deferred` and apply at the start of the slot step.
//! Carl's and Donut's hits land before that step, so theirs apply on the same
//! tick. A contact reaction (`slot_contact_one`) resolves **after** the slot
//! step (`resolve_contacts`), so its follow-ups apply on the **next** tick's
//! slot step; the state hash folds the pending queue, so it covers them.
//!
//! **Env hook for BOIL-B wire and BOIL-C wire.** `combat_scene/env.rs` (the env
//! step, after the charges and before the morale) does not exist on this base.
//! When it lands it (1) calls [`CombatState::set_slot_env`] with the runtime env
//! solids' index range and the props' hit targets, (2) builds the victim bodies
//! from [`CombatState::slot_victim_bodies`] (world rectangles, y up: convert to
//! the layout the hazard set uses), (3) resolves its volumes and footprints, and
//! (4) calls [`CombatState::slot_env_hits`] with the enemy-team hits
//! ([`SlotEnvHit::from_hazard`]). Whichever of ENEMY-SLOT and BOIL-B wire merges
//! second adds nothing else.

use super::companion::{Glance, MarkKind};
use super::{
    Attacker, CombatEvent, CombatState, Confirm, HeroId, LETHAL_CAMERA_KICK_BONUS, MoveTick, Phase,
    SceneSim, hit_box,
};
use crate::boss::{BossThreat, HeroView};
use crate::combat::{DONUT_POUNCE, DONUT_SWAT, MAGIC_MISSILE, MoveSpec};
use crate::donut_moves::{DONUT_HERO, TargetEffect, TargetKind, TargetView};
use crate::prototype::{HERO_HALF_HEIGHT, Rect};
use crate::scene_enemy::{
    EnvView, HitSource, SLOT_SOURCE_MIN, SlotContact, SlotDeathBody, SlotEnemy, SlotEnvHit,
    SlotError, SlotHit, SlotOut, SlotRoom, SlotSet, SlotStepCtx, check_suppliers,
};

/// The slot enemies of a state, with the pristine copies a restart returns to.
#[derive(Debug, Clone, Default)]
pub(super) struct SlotRuntime {
    pub(super) enemies: Vec<Box<dyn SlotEnemy>>,
    pristine: Vec<Box<dyn SlotEnemy>>,
    /// Read access to the env props, filled by the env step (empty until then).
    env: EnvView,
    /// The outputs of hits that had no scene to apply them to, in order.
    deferred: Vec<SlotOut>,
}

impl SlotRuntime {
    pub(super) fn is_empty(&self) -> bool {
        self.enemies.is_empty()
    }

    fn install(&mut self, enemies: Vec<Box<dyn SlotEnemy>>) {
        self.pristine = enemies.clone();
        self.enemies = enemies;
        self.env = EnvView::default();
        self.deferred.clear();
    }

    /// A new attempt: every enemy back to its spawn state.
    pub(super) fn reset(&mut self) {
        if self.pristine.is_empty() {
            return;
        }
        self.enemies = self.pristine.clone();
        for enemy in &mut self.enemies {
            enemy.on_retry();
        }
        self.env = EnvView::default();
        self.deferred.clear();
    }

    pub(super) fn index_of(&self, id: u32) -> Option<usize> {
        self.enemies.iter().position(|e| e.target_id() == id)
    }

    /// The first living supplier's threat, in slot order.
    fn threat(&self) -> Option<BossThreat> {
        self.enemies
            .iter()
            .filter(|e| e.alive() && e.supplies_boss())
            .find_map(|e| e.threat())
    }

    /// Fold every enemy into the state hash, behind a marker word; nothing
    /// while the slot is empty.
    pub(super) fn mix_into(&self, mix: &mut dyn FnMut(u64)) {
        if self.enemies.is_empty() {
            return;
        }
        mix(0x534c_4f54_0001); // "SLOT"
        mix(self.enemies.len() as u64);
        for e in &self.enemies {
            mix(u64::from(e.target_id()));
            e.mix_into(mix);
        }
        // The follow-ups still waiting for the next slot step (a contact
        // reaction resolves after the step), so the hash covers them. Folded
        // only while something is pending: a quiet tick hashes as before.
        if !self.deferred.is_empty() {
            mix(0x534c_4f54_0002); // "SLOT" pending
            mix(self.deferred.len() as u64);
            for out in &self.deferred {
                mix(out.blows.len() as u64);
                for b in &out.blows {
                    mix(b.hero as u64);
                    mix(u64::from(b.source));
                    mix(u64::from(b.damage));
                    mix(u64::from(b.from_x.to_bits()));
                    mix(u64::from(b.knock_vx.to_bits()));
                    mix(u64::from(b.knock_vy.to_bits()));
                }
                mix(out.shifts.len() as u64);
                for (hero, x) in &out.shifts {
                    mix(*hero as u64);
                    mix(u64::from(x.to_bits()));
                }
                mix(out.regroup.len() as u64);
                for (hero, x) in &out.regroup {
                    mix(*hero as u64);
                    mix(u64::from(x.to_bits()));
                }
            }
        }
    }
}

impl CombatState {
    /// Install the slot enemies of this run. The boss-supplier rule is checked
    /// again against this state's fixture boss (the set may have been built
    /// without it): a supplier kind beside a fixture boss is refused.
    pub fn with_slot_enemies(mut self, set: SlotSet) -> Result<Self, SlotError> {
        let enemies = set.into_enemies();
        let kinds: Vec<(&'static str, bool)> = enemies
            .iter()
            .map(|e| (e.kind(), e.supplies_boss()))
            .collect();
        check_suppliers(&kinds, self.fixture.boss.is_some())?;
        self.slot.install(enemies);
        Ok(self)
    }

    /// The slot enemies, in roster order.
    pub fn slot_enemies(&self) -> &[Box<dyn SlotEnemy>] {
        &self.slot.enemies
    }

    /// The env step fills the slot's read access to the env props and hit
    /// targets (the runtime env solids' index range and the hit targets).
    pub fn set_slot_env(&mut self, view: EnvView) {
        self.slot.env = view;
    }

    /// The env read access the slot enemies see this tick.
    pub fn slot_env(&self) -> &EnvView {
        &self.slot.env
    }

    /// The threat the follow layer reads this tick: the fixture boss' own, else
    /// the single slot supplier's. **The one source of both supply sites**: the
    /// plain bot's `PartyStepCtx.boss` (`step_party`) and the companion's
    /// `CompanionView.boss` plus its follow layer (`companion_view`) call it, so
    /// the two never disagree. Setup guarantees at most one supplier kind per
    /// room, and a fixture boss has none beside it.
    pub fn boss_threat(&self) -> Option<BossThreat> {
        self.boss
            .as_ref()
            .and_then(crate::boss::Boss::threat)
            .or_else(|| self.slot.threat())
    }

    /// Whether `id` is a slot enemy's target id.
    pub(super) fn slot_owns(&self, id: u32) -> bool {
        !self.slot.is_empty() && self.slot.index_of(id).is_some()
    }

    /// Log an output's events now. What needs the scene (blows, shifts, regroups)
    /// is returned for the caller to apply, or to defer.
    fn log_slot_events(&mut self, out: &mut SlotOut) {
        for event in std::mem::take(&mut out.events) {
            self.log(event);
        }
    }

    /// Apply an output: events, shifts, regroups, then the blows through the
    /// one `hurt_hero` path (invulnerability, the solo last-HP guard).
    pub(super) fn apply_slot_out(&mut self, scene: &mut SceneSim, mut out: SlotOut, tick: u64) {
        self.log_slot_events(&mut out);
        for (id, x) in out.shifts {
            scene.world.body_mut(id).x = x;
        }
        for (id, x) in out.regroup {
            let leader_y = scene.world.body(id.other()).y;
            let body = scene.world.body_mut(id);
            body.x = x;
            body.y = leader_y;
            body.vx = 0.0;
            body.vy = 0.0;
            body.grounded = false;
        }
        for blow in out.blows {
            debug_assert!(
                blow.source >= SLOT_SOURCE_MIN,
                "a slot enemy's blow source {:#x} is in the goblin index range",
                blow.source
            );
            self.hurt_hero(scene, blow, tick);
        }
    }

    /// A slot enemy that fell must have logged the existing `EnemyDefeated`
    /// (this step adds no defeat variant, and a silent death would leave a story
    /// trigger or a lock waiting forever).
    pub(super) fn check_defeat_logged(id: u32, was_alive: bool, alive: bool, out: &SlotOut) {
        if was_alive && !alive {
            debug_assert!(
                out.events.iter().any(|e| matches!(
                    e,
                    CombatEvent::EnemyDefeated { target, .. } if *target == id
                )),
                "slot enemy {id:#x} fell without logging EnemyDefeated"
            );
        }
    }

    /// Run one enemy's hit and apply what it asks, and log the defeat check.
    fn slot_hit_one(
        &mut self,
        index: usize,
        hit: &SlotHit,
        tick: u64,
    ) -> (Option<crate::scene_enemy::SlotHitResult>, SlotOut) {
        let enemy = &mut self.slot.enemies[index];
        let (id, was_alive) = (enemy.target_id(), enemy.alive());
        let mut out = SlotOut::default();
        let result = enemy.apply_hit(hit, tick, &mut out);
        Self::check_defeat_logged(id, was_alive, self.slot.enemies[index].alive(), &out);
        (result, out)
    }

    fn confirm_slot_hit(&mut self, tick: u64, spec: &MoveSpec, defeated: bool, facing: i8) {
        let bonus = if defeated {
            LETHAL_CAMERA_KICK_BONUS
        } else {
            0
        };
        self.confirm = Some(Confirm {
            tick,
            kick_px: spec.reaction.camera_kick_px + bonus,
            direction: facing,
        });
    }

    /// Carl's melee against the slot enemies: the active box on every hurtbox,
    /// one hit per move instance and enemy. Runs after the boss strike, before
    /// the scene step; Carl wins a same-tick trade.
    pub(super) fn strike_slot_enemies(
        &mut self,
        scene: &mut SceneSim,
        move_tick: &MoveTick,
        attacker: Attacker,
        tick: u64,
        glance: &dyn Fn(&MoveSpec, u8) -> MoveSpec,
    ) {
        if self.slot.is_empty() || move_tick.phase != Phase::Active || move_tick.frozen {
            return;
        }
        let Some(boxed) = hit_box(&move_tick.spec.shape, &attacker) else {
            return;
        };
        for index in 0..self.slot.enemies.len() {
            let enemy = &self.slot.enemies[index];
            if !enemy.alive() || !enemy.hurt_rects().iter().any(|r| r.overlaps(boxed)) {
                continue;
            }
            if !scene.world.hero.melee.record_hit(enemy.target_id()) {
                continue;
            }
            let landed = glance(move_tick.spec, u8::try_from(enemy.hp()).unwrap_or(u8::MAX));
            let hit = SlotHit {
                source: HitSource::CarlMelee(move_tick.spec.id),
                instance: move_tick.instance,
                damage: u16::from(landed.damage),
                poise: u16::from(move_tick.spec.damage),
                player: attacker.player,
                hit_stop: move_tick.spec.reaction.hit_stop,
                facing: attacker.facing,
                spec: move_tick.spec,
            };
            let (result, out) = self.slot_hit_one(index, &hit, tick);
            if let Some(result) = result {
                scene
                    .world
                    .hero
                    .melee
                    .begin_hit_stop(move_tick.spec.reaction.hit_stop);
                self.confirm_slot_hit(tick, move_tick.spec, result.defeated, attacker.facing);
            }
            self.apply_slot_out(scene, out, tick);
        }
    }

    /// The hero views a slot enemy reads (the boss' view of the heroes).
    pub(super) fn slot_hero_views(&self, scene: &SceneSim) -> [HeroView; 2] {
        let drive = *scene.drive();
        let live = scene.party_live();
        HeroId::ALL.map(|id| {
            let body = scene.world.body(id);
            let present = !self.hero_defeated && (live || id == HeroId::Carl);
            HeroView {
                id,
                present,
                x: body.x,
                foot_y: body.y - HERO_HALF_HEIGHT,
                grounded: body.grounded,
                hurt: Self::hurtbox(scene, id),
                vulnerable: present && body.invulnerable_ticks == 0,
                bot: drive.is_bot(id),
            }
        })
    }

    /// One tick of every living slot enemy, in slot order, after the goblins,
    /// the llamas and the globs. The deferred outputs of this tick's hits apply
    /// first; each enemy's step output then applies before the next enemy steps
    /// (a shift of one enemy is seen by the next).
    pub(super) fn step_slot_enemies(&mut self, scene: &mut SceneSim, tick: u64) {
        if self.slot.is_empty() {
            return;
        }
        for out in std::mem::take(&mut self.slot.deferred) {
            self.apply_slot_out(scene, out, tick);
        }
        let drive = *scene.drive();
        let solo = scene.party_live() && !drive.two_player;
        let room = SlotRoom {
            floor_y: self.enemy_foot_y,
            walls: (0.0, self.x_max),
        };
        let view_x = scene.view_x();
        for index in 0..self.slot.enemies.len() {
            if !self.slot.enemies[index].alive() {
                continue;
            }
            // HAZ-A2: the goblins' token pool, with the slot enemies' holders.
            let holders = self.enemies.iter().filter(|g| g.token_held()).count()
                + self.slot.enemies.iter().filter(|e| e.holds_token()).count();
            let token_free = holders < crate::feel::MAX_ATTACKERS
                && self
                    .last_tell_start
                    .is_none_or(|start| tick - start >= crate::feel::TELL_SPACING);
            let heroes = self.slot_hero_views(scene);
            let mut out = SlotOut::default();
            // HAZ-B: the reflex is lent for the step and settled after it; with no
            // evasion in the run the gate is not live and changes nothing.
            let gate = std::cell::RefCell::new(self.evade_gate(scene));
            let ctx = SlotStepCtx {
                tick,
                heroes,
                solids: &scene.world.solids,
                env: &self.slot.env,
                solo,
                room,
                view_x,
                token_free,
                evade: Some(&gate),
            };
            let enemy = &mut self.slot.enemies[index];
            let (id, was_alive) = (enemy.target_id(), enemy.alive());
            enemy.step(&ctx, &mut out);
            self.settle_evade_gate(gate.into_inner());
            Self::check_defeat_logged(id, was_alive, self.slot.enemies[index].alive(), &out);
            if out.tell_started {
                self.last_tell_start = Some(tick);
            }
            self.apply_slot_out(scene, out, tick);
        }
    }

    /// A demo checkpoint respawn: a living slot enemy goes back to its spawn
    /// state, a defeated one stays defeated (the goblins' rule).
    pub(super) fn slot_respawn(&mut self) {
        let SlotRuntime {
            enemies, pristine, ..
        } = &mut self.slot;
        for (live, fresh) in enemies.iter_mut().zip(pristine.iter()) {
            if live.alive() {
                *live = fresh.clone();
                live.on_retry();
            }
        }
        self.slot.deferred.clear();
    }

    /// A launched body touched slot enemy `index`: its reaction, with the
    /// scene-bound follow-ups deferred to the **next** tick's slot step (the
    /// contacts resolve after this tick's slot step and without the scene).
    pub(super) fn slot_contact_one(&mut self, index: usize, contact: &SlotContact, tick: u64) {
        let enemy = &mut self.slot.enemies[index];
        let (id, was_alive) = (enemy.target_id(), enemy.alive());
        let mut out = SlotOut::default();
        enemy.apply_contact(contact, tick, &mut out);
        Self::check_defeat_logged(id, was_alive, self.slot.enemies[index].alive(), &out);
        self.log_slot_events(&mut out);
        if !out.is_empty() {
            self.slot.deferred.push(out);
        }
    }

    // ---------------------------------------------------------- Donut --

    /// Donut's swat and pounce see a slot enemy as a boss-class target (the
    /// swat glances with poise while it stands neutral, the pounce lands its
    /// damage), at its first hurtbox.
    pub(super) fn slot_donut_targets(&self, targets: &mut Vec<TargetView>) {
        for enemy in self.slot.enemies.iter().filter(|e| e.alive()) {
            if let Some(body) = enemy.hurt_rects().first().copied() {
                targets.push(TargetView {
                    index: enemy.target_id(),
                    kind: TargetKind::Boss,
                    body,
                    state: enemy.donut_target_state(),
                    guarded: false,
                });
            }
        }
    }

    /// A Donut hit on slot enemy `target`. Its follow-up outputs wait for the
    /// slot step of the same tick. Returns whether `target` is a slot enemy.
    fn slot_donut_hit(
        &mut self,
        target: u32,
        spec: &'static MoveSpec,
        hit: (HitSource, u32, i8),
        amounts: (u16, u16, u8),
        glance: Glance,
        tick: u64,
    ) -> bool {
        let Some(index) = self.slot.index_of(target) else {
            return false;
        };
        let enemy = &self.slot.enemies[index];
        if !enemy.alive() {
            return true;
        }
        let (source, instance, facing) = hit;
        let (damage, poise, hit_stop) = amounts;
        let landed = SlotHit {
            source,
            instance,
            damage: glance.lands(damage, enemy.hp()),
            poise,
            player: DONUT_HERO,
            hit_stop,
            facing,
            spec,
        };
        let (_, mut out) = self.slot_hit_one(index, &landed, tick);
        self.log_slot_events(&mut out);
        if !out.is_empty() {
            self.slot.deferred.push(out);
        }
        true
    }

    /// A swat's glance (poise only, damage 0) on a slot enemy.
    pub(super) fn slot_swat_glance(&mut self, target: u32, poise: u8, instance: u32, tick: u64) {
        if poise > 0 && self.slot_owns(target) {
            self.slot_donut_hit(
                target,
                &DONUT_SWAT,
                (HitSource::DonutSwat, instance, 0),
                (0, u16::from(poise), 0),
                Glance::default(),
                tick,
            );
        }
    }

    /// A pounce on a slot enemy (its hero-hit rule, like the boss'). Returns
    /// whether `target` was a slot enemy.
    pub(super) fn slot_pounce_hit(
        &mut self,
        target: u32,
        effect: TargetEffect,
        facing: i8,
        instance: u32,
        tick: u64,
    ) -> bool {
        if !self.slot_owns(target) {
            return false;
        }
        let glance = self.companion.glance_for(MarkKind::DonutMove, instance);
        let damage = u16::from(effect.damage);
        self.slot_donut_hit(
            target,
            &DONUT_POUNCE,
            (HitSource::DonutPounce, instance, facing),
            (damage, damage, effect.hit_stop.attacker),
            glance,
            tick,
        )
    }

    /// A Magic Missile bolt that overlaps a slot hurtbox ends there (the first
    /// enemy in slot order), after the goblins, llamas and boss of `test_bolt`.
    /// Returns whether the bolt was consumed.
    pub(super) fn slot_missile_hit(&mut self, bolt: &super::Bolt, tick: u64) -> bool {
        if self.slot.is_empty() {
            return false;
        }
        let rect = bolt.rect();
        let Some(target) = self
            .slot
            .enemies
            .iter()
            .find(|e| e.alive() && e.hurt_rects().iter().any(|r| r.overlaps(rect)))
            .map(|e| e.target_id())
        else {
            return false;
        };
        let glance = self
            .companion
            .glance_for(MarkKind::DonutCast, bolt.instance);
        let damage = u16::from(MAGIC_MISSILE.damage);
        self.slot_donut_hit(
            target,
            &MAGIC_MISSILE,
            (HitSource::MagicMissile, bolt.instance, bolt.dir),
            (damage, damage, MAGIC_MISSILE.reaction.hit_stop),
            glance,
            tick,
        )
    }

    // ------------------------------------------------------------ env --

    /// The death-species hook: where and what the cosmetic presenter draws for
    /// slot enemy `target` (alive or fallen). `None` for an unknown id or an
    /// enemy with no death art.
    pub fn slot_death_body(&self, target: u32) -> Option<SlotDeathBody> {
        let index = self.slot.index_of(target)?;
        self.slot.enemies[index].death_body()
    }

    /// The bodies an env step tests against its volumes and footprints: every
    /// living slot enemy's victim rectangle (world, y up), by target id.
    pub fn slot_victim_bodies(&self) -> Vec<(u32, Rect)> {
        self.slot
            .enemies
            .iter()
            .filter(|e| e.alive())
            .filter_map(|e| e.victim_rect().map(|r| (e.target_id(), r)))
            .collect()
    }

    /// Route the env prop and hazard hits an env step resolved against the
    /// slot enemies' bodies. A hit on an unknown or defeated victim is dropped;
    /// the dedupe per victim and activation is the env step's (the hazard set
    /// already applies it).
    pub fn slot_env_hits(&mut self, scene: &mut SceneSim, hits: &[SlotEnvHit], tick: u64) {
        for hit in hits {
            let Some(index) = self.slot.index_of(hit.victim) else {
                continue;
            };
            let enemy = &mut self.slot.enemies[index];
            if !enemy.alive() {
                continue;
            }
            let (id, was_alive) = (enemy.target_id(), true);
            let mut out = SlotOut::default();
            enemy.apply_env(hit, tick, &mut out);
            Self::check_defeat_logged(id, was_alive, self.slot.enemies[index].alive(), &out);
            self.apply_slot_out(scene, out, tick);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{ChargeSpawn, DemoRules, EnemySpawn, Fixture};
    use super::*;
    use crate::blast::{Arm, BlastCharge, BlastProfile, BlastReach};
    use crate::boss::HeroBlow;
    use crate::boss_scene::Pilot;
    use crate::combat::{DefeatCause, EnemyAi, HERO_HEALTH, MoveId};
    use crate::donut_mana::DonutSetup;
    use crate::env_hazard::HazardClass;
    use crate::env_kit::EnvKit;
    use crate::goblin::GoblinMode;
    use crate::hero_damage::HeroDamagePolicy;
    use crate::input::RawInput;
    use crate::party::{PROVISIONAL_TUNING, PartyTuning, UncontrolledHero};
    use crate::party_scene::{PartyFixture, PartyFixtureSim, base};
    use crate::prototype::{Block, Material};
    use crate::scene_enemy::testing::{DUMMY_SOURCE, Dummy, jab_reach};
    use crate::scene_enemy::{EnvSource, EnvTarget, SLOT_ID_MIN};
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use crate::scene_enemy::testing::DummyLog;

    const ID: u32 = SLOT_ID_MIN;
    const CARL_X: f32 = 400.0;

    fn kit() -> EnvKit {
        EnvKit::load(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            "assets/levels/floor01/corridor_abc_sample.json",
        )
        .unwrap()
    }

    const fn party(base: &'static Fixture, two_player: bool, donut: DonutSetup) -> PartyFixture {
        PartyFixture {
            base,
            two_player,
            tuning: PROVISIONAL_TUNING,
            script: &[],
            pilot: Pilot::None,
            donut,
        }
    }

    static EMPTY_BASE: Fixture = base("test_slot_empty", "no goblins", 7, &[], 900);
    static SOLO: PartyFixture = party(&EMPTY_BASE, false, DonutSetup::NONE);
    static DUO: PartyFixture = party(&EMPTY_BASE, true, DonutSetup::NONE);
    static DUO_ARMED: PartyFixture = party(&EMPTY_BASE, true, DonutSetup::unlocked(8));

    static GOBLIN_BASE: Fixture = base(
        "test_slot_goblin",
        "one passive goblin",
        7,
        &[EnemySpawn {
            x: 520,
            ai: EnemyAi::Passive,
        }],
        900,
    );
    static GOBLIN_SOLO: PartyFixture = party(&GOBLIN_BASE, false, DonutSetup::NONE);

    static QA_BASE: Fixture = Fixture {
        charges: &[ChargeSpawn {
            dx: 30,
            tick: 3,
            owner: None,
            arm: Arm::Tick,
        }],
        ..base("test_slot_qa", "a QA charge", 7, &[], 900)
    };
    static QA_SOLO: PartyFixture = party(&QA_BASE, false, DonutSetup::NONE);

    static BOSS_BASE: Fixture = crate::boss_scene::boss_base(
        "test_slot_boss",
        "the arena",
        7,
        380,
        crate::boss_scene::hp(10),
        900,
    );
    static BOSS_SOLO: PartyFixture = party(&BOSS_BASE, false, DonutSetup::NONE);

    static COMPANION_BASE: Fixture = base("test_slot_companion", "a companion Donut", 7, &[], 900);
    static COMPANION: PartyFixture = PartyFixture {
        tuning: PartyTuning {
            uncontrolled_hero: UncontrolledHero::Companion(
                crate::companion::profile::ProfileSetId::Cf02Assist,
            ),
            ..PROVISIONAL_TUNING
        },
        ..party(&COMPANION_BASE, false, DonutSetup::NONE)
    };

    fn foot() -> f32 {
        crate::corridor::SceneSim::new_at(&kit(), 360, 400)
            .unwrap()
            .world
            .hero_foot_y()
    }

    /// A dummy `dx` px right of Carl's start column, with its log.
    fn dummy_at(dx: f32) -> (Dummy, Arc<Mutex<DummyLog>>) {
        let d = Dummy::new(ID, CARL_X + dx, foot());
        let log = d.log.clone();
        (d, log)
    }

    fn sim(fixture: &'static PartyFixture, dummies: Vec<Dummy>) -> PartyFixtureSim {
        let kit = if fixture.base.boss.is_some() {
            crate::boss_scene::load_arena(&PathBuf::from(env!("CARGO_MANIFEST_DIR"))).unwrap()
        } else {
            kit()
        };
        let carl_x = fixture.base.carl_x;
        let scene = crate::corridor::SceneSim::new_at(&kit, 360, carl_x).unwrap();
        let set = SlotSet::new(
            dummies.into_iter().map(Dummy::boxed).collect(),
            fixture.base.boss.is_some(),
        )
        .unwrap();
        let combat = CombatState::new(fixture.base, &kit, &scene)
            .unwrap()
            .with_donut(fixture.donut)
            .with_slot_enemies(set)
            .unwrap();
        PartyFixtureSim::from_parts(scene, combat, fixture)
    }

    fn idle(s: &mut PartyFixtureSim, ticks: u64) {
        for _ in 0..ticks {
            s.step_slots([RawInput::default(); 2]);
        }
    }

    fn step(s: &mut PartyFixtureSim, carl: RawInput, donut: RawInput) {
        s.step_slots([carl, donut]);
    }

    const ATTACK: RawInput = RawInput {
        attack: true,
        ..RawInput::NONE
    };
    const ATTACK_LEFT: RawInput = RawInput {
        attack: true,
        move_x: -1.0,
        ..RawInput::NONE
    };
    const PHYSICAL: RawInput = RawInput {
        physical: true,
        ..RawInput::NONE
    };

    fn tick(s: &PartyFixtureSim) -> u64 {
        s.scene.world.tick
    }

    fn defeats(s: &PartyFixtureSim) -> Vec<CombatEvent> {
        s.combat
            .events
            .iter()
            .copied()
            .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
            .collect()
    }

    fn hurts(s: &PartyFixtureSim) -> Vec<(u64, u32, u8, HeroId)> {
        s.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::HeroHurt {
                    tick,
                    source,
                    health_left,
                    hero,
                } => Some((tick, source, health_left, hero)),
                _ => None,
            })
            .collect()
    }

    // ------------------------------------------------------ the empty slot --

    #[test]
    fn an_empty_slot_is_the_legacy_game_byte_for_byte() {
        let kit = kit();
        let run = |slot: bool| {
            let scene = crate::corridor::SceneSim::new_at(&kit, 360, 400).unwrap();
            let mut combat = CombatState::new(GOBLIN_SOLO.base, &kit, &scene).unwrap();
            if slot {
                combat = combat
                    .with_slot_enemies(SlotSet::new(Vec::new(), false).unwrap())
                    .unwrap();
            }
            let mut s = PartyFixtureSim::from_parts(scene, combat, &GOBLIN_SOLO);
            for i in 0..120 {
                step(
                    &mut s,
                    if i % 20 == 3 {
                        ATTACK
                    } else {
                        RawInput::default()
                    },
                    RawInput::default(),
                );
            }
            (s.combat.state_hash(&s.scene), s.combat.events.len())
        };
        assert_eq!(run(true), run(false));
        // A slot enemy does change the hash, and its hp changes it again.
        let (d, _) = dummy_at(300.0);
        let mut s = sim(&SOLO, vec![d]);
        let legacy = sim(&SOLO, Vec::new());
        assert_ne!(
            s.combat.state_hash(&s.scene),
            legacy.combat.state_hash(&legacy.scene)
        );
        let before = s.combat.state_hash(&s.scene);
        s.combat.slot.enemies[0] = {
            let mut d = Dummy::new(ID, CARL_X + 300.0, foot());
            d.hp = 2;
            d.boxed()
        };
        assert_ne!(s.combat.state_hash(&s.scene), before);
    }

    // ------------------------------------------------------------- melee --

    /// Carl's jab box at his start column, for edge geometry.
    fn jab_box(s: &PartyFixtureSim, facing: i8) -> Rect {
        let hero = &s.scene.world.hero;
        jab_reach(hero.x, hero.y, facing).unwrap()
    }

    #[test]
    fn carls_jab_hits_a_slot_enemy_once_per_move_and_the_edges_hold_on_both_facings() {
        for (facing, input) in [(1i8, ATTACK), (-1, ATTACK_LEFT)] {
            // The box edge nearest the dummy decides: touching is a miss, a
            // quarter pixel inside is a hit.
            let probe = sim(&SOLO, Vec::new());
            let boxed = jab_box(&probe, facing);
            for (inside, expect) in [(false, 0usize), (true, 1)] {
                let slack = if inside { 0.25 } else { 0.0 };
                let x = if facing > 0 {
                    boxed.right() + 12.0 - slack
                } else {
                    boxed.left - 12.0 + slack
                };
                let mut d = Dummy::new(ID, x, foot());
                d.hp_max = 4;
                let log = d.log.clone();
                let mut s = sim(&SOLO, vec![d]);
                step(&mut s, input, RawInput::default());
                idle(&mut s, 12);
                let hits = log.lock().unwrap().hits.clone();
                assert_eq!(hits.len(), expect, "facing {facing} inside {inside}");
                if inside {
                    let (t, source, damage, poise) = hits[0];
                    assert_eq!(source, HitSource::CarlMelee(MoveId::Jab));
                    assert_eq!((damage, poise), (1, 1), "the jab: 1 damage, 1 poise");
                    // Press on tick 1, startup 3: the first active tick is 4.
                    assert_eq!(t, 4, "the first active tick of the jab");
                    assert_eq!(s.combat.slot_enemies()[0].hp(), 3);
                }
            }
        }
    }

    #[test]
    fn a_slot_enemy_takes_one_hit_per_move_instance_and_the_next_move_hits_again() {
        let (d, log) = dummy_at(30.0);
        let mut s = sim(&SOLO, vec![d]);
        step(&mut s, ATTACK, RawInput::default());
        idle(&mut s, 30);
        assert_eq!(
            log.lock().unwrap().hits.len(),
            1,
            "two active ticks, one hit"
        );
        step(&mut s, ATTACK, RawInput::default());
        idle(&mut s, 30);
        let hits = log.lock().unwrap().hits.clone();
        assert_eq!(hits.len(), 2, "a new instance hits again");
        assert_eq!(s.combat.slot_enemies()[0].hp(), 2);
    }

    #[test]
    fn a_melee_hit_freezes_carl_and_kicks_the_camera_and_a_lethal_one_kicks_harder() {
        let (mut d, _) = dummy_at(30.0);
        d.hp = 1;
        let mut s = sim(&SOLO, vec![d]);
        step(&mut s, ATTACK, RawInput::default());
        idle(&mut s, 3);
        assert!(s.scene.world.hero.melee.hit_stop_left() > 0, "Carl froze");
        let lethal = s.combat.camera_kick(tick(&s));
        let (d, _) = dummy_at(30.0);
        let mut other = sim(&SOLO, vec![d]);
        step(&mut other, ATTACK, RawInput::default());
        idle(&mut other, 3);
        let plain = other.combat.camera_kick(tick(&other));
        // At the hit tick the kick is the move's px, plus the lethal bonus; it
        // is opposite the hit direction (Carl faces right).
        assert_eq!(
            lethal.abs() - plain.abs(),
            i32::from(LETHAL_CAMERA_KICK_BONUS),
            "lethal {lethal} vs plain {plain}"
        );
        assert!(lethal < 0, "against the hit direction");
    }

    #[test]
    fn a_lethal_melee_hit_logs_the_existing_enemy_defeated_and_the_corpse_takes_no_hit() {
        let (mut d, log) = dummy_at(30.0);
        d.hp = 1;
        let mut s = sim(&SOLO, vec![d]);
        step(&mut s, ATTACK, RawInput::default());
        idle(&mut s, 20);
        let fell = defeats(&s);
        assert_eq!(fell.len(), 1, "{fell:?}");
        let CombatEvent::EnemyDefeated {
            tick: t,
            target,
            cause,
            instigator,
            ..
        } = fell[0]
        else {
            unreachable!()
        };
        assert_eq!(
            (t, target, cause, instigator),
            (4, ID, DefeatCause::Melee, Some(0))
        );
        assert!(!s.combat.slot_enemies()[0].alive());
        assert!(s.combat.slot_enemies()[0].hurt_rects().is_empty());
        // A second jab passes through the corpse.
        step(&mut s, ATTACK, RawInput::default());
        idle(&mut s, 20);
        assert_eq!(log.lock().unwrap().hits.len(), 1);
        assert_eq!(defeats(&s).len(), 1);
    }

    #[test]
    fn an_enemy_without_a_hurtbox_lets_the_hit_pass() {
        let (mut d, log) = dummy_at(30.0);
        d.hidden = true;
        let mut s = sim(&SOLO, vec![d]);
        step(&mut s, ATTACK, RawInput::default());
        idle(&mut s, 12);
        assert!(log.lock().unwrap().hits.is_empty());
        assert_eq!(s.combat.slot_enemies()[0].hp(), 4);
    }

    #[test]
    #[should_panic(expected = "fell without logging EnemyDefeated")]
    fn a_slot_enemy_that_falls_silently_is_caught() {
        let (mut d, _) = dummy_at(30.0);
        d.hp = 1;
        d.silent_death = true;
        let mut s = sim(&SOLO, vec![d]);
        step(&mut s, ATTACK, RawInput::default());
        idle(&mut s, 6);
    }

    // ------------------------------------------------------------ Donut --

    fn donut_ahead(s: &PartyFixtureSim) -> (f32, i8) {
        let d = &s.scene.world.donut_body;
        (d.x, d.facing)
    }

    #[test]
    fn donuts_swat_glances_a_neutral_slot_enemy_with_poise_and_none_in_another_state() {
        for (state, poise) in [
            (crate::donut_moves::TargetState::Neutral, 1u16),
            (crate::donut_moves::TargetState::Recovery, 0),
        ] {
            let probe = sim(&DUO, Vec::new());
            let (dx, facing) = donut_ahead(&probe);
            let (mut d, log) = dummy_at(0.0);
            d.x = dx + f32::from(facing) * 30.0;
            d.target_state = state;
            let mut s = sim(&DUO, vec![d]);
            step(&mut s, RawInput::default(), ATTACK);
            idle(&mut s, 30);
            let hits = log.lock().unwrap().hits.clone();
            if poise == 0 {
                assert!(hits.is_empty(), "no poise in another state: {hits:?}");
            } else {
                assert_eq!(hits.len(), 1, "{hits:?}");
                let (_, source, damage, p) = hits[0];
                assert_eq!((source, damage, p), (HitSource::DonutSwat, 0, 1));
                assert_eq!(s.combat.slot_enemies()[0].hp(), 4, "a swat never damages");
            }
        }
    }

    #[test]
    fn donuts_pounce_lands_its_damage_and_a_lethal_one_is_credited_to_her() {
        let probe = sim(&DUO, Vec::new());
        let (dx, facing) = donut_ahead(&probe);
        let (mut d, log) = dummy_at(0.0);
        d.x = dx + f32::from(facing) * 60.0;
        d.hp = 1;
        let mut s = sim(&DUO, vec![d]);
        step(&mut s, RawInput::default(), PHYSICAL);
        idle(&mut s, 45);
        let hits = log.lock().unwrap().hits.clone();
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].1, HitSource::DonutPounce);
        assert!(hits[0].2 >= 1, "the pounce's damage");
        let fell = defeats(&s);
        assert_eq!(fell.len(), 1, "{fell:?}");
        assert!(matches!(
            fell[0],
            CombatEvent::EnemyDefeated {
                target: ID,
                cause: DefeatCause::Melee,
                instigator: Some(1),
                ..
            }
        ));
    }

    #[test]
    fn a_magic_missile_ends_on_a_slot_hurtbox_and_passes_through_a_hidden_one() {
        let probe = sim(&DUO_ARMED, Vec::new());
        let (dx, facing) = donut_ahead(&probe);
        let place = |hidden: bool| {
            let (mut d, log) = dummy_at(0.0);
            d.x = dx + f32::from(facing) * 90.0;
            d.hidden = hidden;
            let mut s = sim(&DUO_ARMED, vec![d]);
            step(&mut s, RawInput::default(), ATTACK);
            idle(&mut s, 24);
            (s, log)
        };
        let (s, log) = place(false);
        let hits = log.lock().unwrap().hits.clone();
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(
            (hits[0].1, hits[0].2, hits[0].3),
            (HitSource::MagicMissile, 1, 1)
        );
        assert!(s.combat.donut_bolts().is_empty(), "the bolt was consumed");
        // A hurtbox-less enemy is passed through.
        let (s, log) = place(true);
        assert!(log.lock().unwrap().hits.is_empty());
        assert!(!s.combat.donut_bolts().is_empty(), "the bolt flew on");
    }

    // ------------------------------------------------------------ blasts --

    fn profile(core: f32, outer: f32, los: bool) -> BlastProfile {
        BlastProfile {
            core_radius: core,
            outer_radius: outer,
            los,
            ..BlastProfile::QA
        }
    }

    /// Blow a charge `gap` px right of the dummy's right edge (centre y inside
    /// the body) and return the dummy's blast log.
    fn blast_at_gap(
        gap: f32,
        profile: BlastProfile,
        owner: Option<u32>,
        wall: bool,
    ) -> (PartyFixtureSim, Vec<(u64, BlastReach, i8)>) {
        let (d, log) = dummy_at(100.0);
        let mut s = sim(&SOLO, vec![d]);
        let edge = CARL_X + 100.0 + 12.0;
        if wall {
            s.scene.world.solids.push(Block {
                rect: Rect {
                    left: edge + 8.0,
                    bottom: foot() - 4.0,
                    width: 4.0,
                    height: 90.0,
                },
                material: Material::Stone,
                intact: true,
            });
        }
        let charge = BlastCharge::new(edge + gap, foot(), 1, owner).with_profile(profile);
        s.combat.resolve_profile_blast(&mut s.scene, 0, charge, 3);
        let blasts = log.lock().unwrap().blasts.clone();
        (s, blasts)
    }

    #[test]
    fn a_blast_reaches_a_slot_enemy_in_each_band_and_the_radii_hold_on_both_sides() {
        let p = profile(20.0, 60.0, true);
        let bands = [
            // Inside the body: point blank. Core edge: ring. Past it: outer.
            (-6.0, Some(BlastReach::PointBlank)),
            (20.0, Some(BlastReach::Ring)),
            (20.5, Some(BlastReach::Outer)),
            (60.0, Some(BlastReach::Outer)),
            (60.5, None),
        ];
        for (gap, expect) in bands {
            let (s, blasts) = blast_at_gap(gap, p, None, false);
            assert_eq!(blasts.first().map(|b| b.1), expect, "gap {gap}");
            assert_eq!(blasts.len(), usize::from(expect.is_some()), "gap {gap}");
            if let Some(reach) = expect {
                // The blast throws the body away from the charge: the charge is
                // to its right.
                assert_eq!(blasts[0].2, -1);
                assert_eq!(blasts[0].0, 3);
                let hp = s.combat.slot_enemies()[0].hp();
                assert_eq!(hp, if reach == BlastReach::Outer { 3 } else { 0 });
            }
        }
        // A charge to the left of the body throws it right.
        let (d, log) = dummy_at(100.0);
        let mut s = sim(&SOLO, vec![d]);
        let charge =
            BlastCharge::new(CARL_X + 100.0 - 12.0 - 10.0, foot(), 1, None).with_profile(p);
        s.combat.resolve_profile_blast(&mut s.scene, 0, charge, 3);
        assert_eq!(log.lock().unwrap().blasts[0].2, 1);
    }

    #[test]
    fn a_solid_blocks_a_blast_unless_the_profile_ignores_line_of_sight() {
        let (_, blocked) = blast_at_gap(30.0, profile(20.0, 60.0, true), None, true);
        assert!(blocked.is_empty(), "a wall between the charge and the body");
        let (_, open) = blast_at_gap(30.0, profile(20.0, 60.0, false), None, true);
        assert_eq!(open.len(), 1);
        // The same charge without the wall reaches it.
        let (_, clear) = blast_at_gap(30.0, profile(20.0, 60.0, true), None, false);
        assert_eq!(clear.len(), 1);
    }

    #[test]
    fn a_lethal_blast_credits_the_owner_and_logs_an_explosion_defeat() {
        for (gap, point_blank) in [(-6.0, true), (10.0, false)] {
            let (s, _) = blast_at_gap(gap, profile(20.0, 60.0, true), Some(1), false);
            let fell = defeats(&s);
            assert_eq!(fell.len(), 1, "{fell:?}");
            assert!(
                matches!(
                    fell[0],
                    CombatEvent::EnemyDefeated {
                        target: ID,
                        cause: DefeatCause::Explosion {
                            charge: 0,
                            player: Some(1),
                            point_blank: pb
                        },
                        instigator: Some(1),
                        ..
                    } if pb == point_blank
                ),
                "{:?}",
                fell[0]
            );
        }
    }

    #[test]
    fn a_blast_takes_the_goblins_then_the_slot_then_the_heroes_in_that_order() {
        let (d, _) = Dummy::new(ID, 520.0, foot()).boxed_pair();
        let kit = kit();
        let scene = crate::corridor::SceneSim::new_at(&kit, 360, 400).unwrap();
        let combat = CombatState::new(GOBLIN_SOLO.base, &kit, &scene)
            .unwrap()
            .with_slot_enemies(SlotSet::new(vec![d], false).unwrap())
            .unwrap();
        let mut s = PartyFixtureSim::from_parts(scene, combat, &GOBLIN_SOLO);
        // Everyone inside a wide profile that hurts every hero.
        let mut p = profile(120.0, 200.0, false);
        p.hero_damage = HeroDamagePolicy::AllHeroes;
        p.hero_hp = 1;
        p.hero_source = 0x7777;
        let charge = BlastCharge::new(500.0, foot(), 1, None).with_profile(p);
        s.combat.resolve_profile_blast(&mut s.scene, 0, charge, 3);
        let kinds: Vec<&str> = s
            .combat
            .events
            .iter()
            .map(|e| match e {
                CombatEvent::EnemyDefeated { target: 0, .. } => "goblin",
                CombatEvent::EnemyDefeated { target: ID, .. } => "slot",
                CombatEvent::HeroHurt { .. } => "hero",
                _ => "other",
            })
            .collect();
        let first_hero = kinds.iter().position(|k| *k == "hero").unwrap();
        let goblin = kinds.iter().position(|k| *k == "goblin").unwrap();
        let slot = kinds.iter().position(|k| *k == "slot").unwrap();
        assert!(goblin < slot && slot < first_hero, "{kinds:?}");
    }

    #[test]
    fn a_qa_charge_never_reaches_a_slot_enemy() {
        let (d, log) = dummy_at(30.0);
        let mut s = sim(&QA_SOLO, vec![d]);
        idle(&mut s, 6);
        assert!(
            s.combat
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::Blast { .. })),
            "the charge went off"
        );
        assert!(log.lock().unwrap().blasts.is_empty());
        assert_eq!(s.combat.slot_enemies()[0].hp(), 4);
    }

    // ---------------------------------------------------------- contacts --

    fn launched_goblin(s: &mut PartyFixtureSim, launch: u32) {
        let g = &mut s.combat.enemies[0];
        g.mode = GoblinMode::Launched;
        g.launch = Some(launch);
        g.vx = 0.0;
        g.vy = 0.0;
    }

    #[test]
    fn a_launched_goblin_contacts_a_slot_enemy_once_per_launch() {
        // The push goes from the goblin through the enemy: right of the goblin's
        // centre pushes right, left of it pushes left.
        for (x, push) in [(530.0, 1i8), (510.0, -1)] {
            let (mut d, log) = dummy_at(0.0);
            d.x = x;
            let mut s = sim(&GOBLIN_SOLO, vec![d]);
            s.combat = s.combat.clone().with_cf02();
            launched_goblin(&mut s, 9);
            idle(&mut s, 1);
            let contacts = log.lock().unwrap().contacts.clone();
            assert_eq!(contacts.len(), 1, "{contacts:?}");
            assert_eq!(contacts[0].1.push, push, "dummy at {x}");
        }
        let (mut d, log) = dummy_at(0.0);
        d.x = 520.0;
        let mut s = sim(&GOBLIN_SOLO, vec![d]);
        s.combat = s.combat.clone().with_cf02();
        launched_goblin(&mut s, 9);
        idle(&mut s, 1);
        let contacts = log.lock().unwrap().contacts.clone();
        assert_eq!(contacts.len(), 1, "{contacts:?}");
        let (t, c) = contacts[0];
        assert_eq!((t, c.launch, c.source, c.body), (1, 9, 0, false));
        // The same launch does not contact again, a new launch does.
        idle(&mut s, 1);
        assert_eq!(log.lock().unwrap().contacts.len(), 1);
        launched_goblin(&mut s, 10);
        idle(&mut s, 1);
        let contacts = log.lock().unwrap().contacts.clone();
        assert_eq!(contacts.len(), 2);
        assert_eq!(contacts[1].1.launch, 10);
    }

    #[test]
    fn a_contact_follow_up_applies_next_tick_and_is_hashed_while_pending() {
        let (mut d, log) = dummy_at(0.0);
        d.x = 520.0;
        d.contact_blow = true;
        let mut s = sim(&GOBLIN_SOLO, vec![d]);
        s.combat = s.combat.clone().with_cf02();
        launched_goblin(&mut s, 9);
        idle(&mut s, 1);
        // The contact resolved after this tick's slot step: the blow waits.
        assert_eq!(log.lock().unwrap().contacts.len(), 1);
        assert!(hurts(&s).is_empty(), "{:?}", hurts(&s));
        assert_eq!(s.combat.slot.deferred.len(), 1);
        // The pending queue is part of the hash.
        let pending = s.combat.state_hash(&s.scene);
        let mut cleared = s.combat.clone();
        cleared.slot.deferred.clear();
        assert_ne!(cleared.state_hash(&s.scene), pending);
        // Next tick's slot step applies it.
        idle(&mut s, 1);
        let hurt = hurts(&s);
        assert_eq!(hurt.len(), 1, "{hurt:?}");
        assert_eq!((hurt[0].1, hurt[0].3), (DUMMY_SOURCE, HeroId::Carl));
        assert!(s.combat.slot.deferred.is_empty());
    }

    #[test]
    fn contacts_need_overlap_a_contact_box_and_cf02_rules() {
        // Not overlapping: no contact.
        let (mut far, log) = dummy_at(0.0);
        far.x = 900.0;
        let mut s = sim(&GOBLIN_SOLO, vec![far]);
        s.combat = s.combat.clone().with_cf02();
        launched_goblin(&mut s, 9);
        idle(&mut s, 3);
        assert!(log.lock().unwrap().contacts.is_empty());
        // Overlapping but immune (`victim: false` gives no contact box).
        let (mut immune, log) = dummy_at(0.0);
        immune.x = 520.0;
        immune.victim = false;
        let mut s = sim(&GOBLIN_SOLO, vec![immune]);
        s.combat = s.combat.clone().with_cf02();
        launched_goblin(&mut s, 9);
        idle(&mut s, 1);
        assert!(log.lock().unwrap().contacts.is_empty());
        // Overlapping, but the legacy contact path is untouched.
        let (mut d, log) = dummy_at(0.0);
        d.x = 520.0;
        let mut s = sim(&GOBLIN_SOLO, vec![d]);
        launched_goblin(&mut s, 9);
        idle(&mut s, 1);
        assert!(log.lock().unwrap().contacts.is_empty());
    }

    #[test]
    fn a_bowling_body_contacts_a_slot_enemy_as_a_body() {
        let (mut d, log) = dummy_at(0.0);
        d.x = 520.0;
        let mut s = sim(&GOBLIN_SOLO, vec![d]);
        s.combat = s.combat.clone().with_cf02();
        let g = &mut s.combat.enemies[0];
        g.health = 0;
        g.mode = GoblinMode::Airborne;
        g.bowl = Some(5);
        g.hit_by = Some(1);
        idle(&mut s, 1);
        let contacts = log.lock().unwrap().contacts.clone();
        assert_eq!(contacts.len(), 1, "{contacts:?}");
        let c = contacts[0].1;
        assert!(c.body);
        assert_eq!((c.launch, c.instigator), (5, Some(1)));
    }

    // -------------------------------------------------------- hero hurt --

    #[test]
    fn a_slot_enemys_blow_goes_through_the_one_hurt_path_for_either_hero() {
        // Carl (solo).
        let (mut d, log) = dummy_at(20.0);
        d.reach = 40.0;
        d.cooldown = 1;
        let mut s = sim(&SOLO, vec![d]);
        idle(&mut s, 12);
        let hurt = hurts(&s);
        assert_eq!(hurt.len(), 1, "invulnerability holds the rest: {hurt:?}");
        assert_eq!(hurt[0], (1, DUMMY_SOURCE, HERO_HEALTH - 1, HeroId::Carl));
        assert!(log.lock().unwrap().swings.len() > 3, "it kept swinging");
        // Donut.
        let probe = sim(&DUO, Vec::new());
        let (dx, _) = donut_ahead(&probe);
        let (mut d, _) = dummy_at(0.0);
        d.x = dx - 4.0;
        d.reach = 6.0;
        let mut s = sim(&DUO, vec![d]);
        idle(&mut s, 2);
        assert!(
            hurts(&s)
                .iter()
                .any(|h| h.3 == HeroId::Donut && h.1 == DUMMY_SOURCE)
        );
    }

    #[test]
    fn a_lethal_blow_defeats_the_hero_through_the_same_path() {
        let (mut d, _) = dummy_at(20.0);
        d.reach = 40.0;
        let mut s = sim(&SOLO, vec![d]);
        s.scene.world.hero.health = 1;
        idle(&mut s, 2);
        assert!(s.combat.hero_defeated);
        assert!(s.combat.events.iter().any(|e| matches!(
            e,
            CombatEvent::HeroDefeated {
                hero: HeroId::Carl,
                ..
            }
        )));
    }

    #[test]
    fn the_blow_source_floor_keeps_goblin_indices_apart() {
        const { assert!(SLOT_SOURCE_MIN > 0xFF) };
        const { assert!(DUMMY_SOURCE >= SLOT_SOURCE_MIN) };
        // A source in the goblin index range is a defect the scene refuses.
        let result = std::panic::catch_unwind(|| {
            let mut s = sim(&SOLO, Vec::new());
            let out = SlotOut {
                blows: vec![HeroBlow {
                    hero: HeroId::Carl,
                    source: 3,
                    damage: 1,
                    from_x: 0.0,
                    knock_vx: 0.0,
                    knock_vy: 0.0,
                }],
                ..SlotOut::default()
            };
            s.combat.apply_slot_out(&mut s.scene, out, 1);
        });
        assert!(result.is_err());
    }

    // --------------------------------------------------------------- env --

    #[test]
    fn the_slot_step_reads_the_env_solids_and_hit_targets_and_the_view_is_empty_by_default() {
        let (d, log) = dummy_at(200.0);
        let mut s = sim(&SOLO, vec![d]);
        idle(&mut s, 1);
        let before = s.scene.world.solids.len();
        assert_eq!(log.lock().unwrap().seen[0], (1, before, 0, 0, 2));
        // The env step appends two runtime solids and sets the view.
        for left in [10.0, 30.0] {
            s.scene.world.solids.push(Block {
                rect: Rect {
                    left,
                    bottom: 0.0,
                    width: 8.0,
                    height: 8.0,
                },
                material: Material::Timber,
                intact: true,
            });
        }
        s.combat.set_slot_env(EnvView {
            solid_range: before..before + 2,
            targets: vec![EnvTarget {
                prop: 4,
                rect: Rect {
                    left: 10.0,
                    bottom: 0.0,
                    width: 8.0,
                    height: 8.0,
                },
                solid: true,
            }],
        });
        idle(&mut s, 1);
        let seen = *log.lock().unwrap().seen.last().unwrap();
        assert_eq!(
            seen,
            (2, before + 2, 2, 1, 2),
            "(tick, solids, env solids, targets, heroes)"
        );
        assert_eq!(s.combat.slot_env().targets[0].prop, 4);
        // A restart clears the view with the attempt.
        s.combat.restart(&mut s.scene);
        assert!(s.combat.slot_env().is_empty());
    }

    fn env_hit(victim: u32, activation: u8, damage: u8) -> SlotEnvHit {
        SlotEnvHit {
            victim,
            source: EnvSource::Hazard(2),
            class: HazardClass::Acid,
            damage,
            push_x: -30,
            initiator: Some(1),
            activation,
        }
    }

    #[test]
    fn env_hits_route_to_a_living_slot_victim_and_a_lethal_one_logs_a_defeat() {
        let (d, log) = dummy_at(200.0);
        let mut s = sim(&SOLO, vec![d]);
        let bodies = s.combat.slot_victim_bodies();
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0].0, ID);
        assert_eq!(bodies[0].1.bottom, foot());
        let t = tick(&s) + 1;
        // An unknown victim is dropped.
        let mut scene = s.scene.clone();
        s.combat
            .slot_env_hits(&mut scene, &[env_hit(ID + 7, 1, 1)], t);
        assert!(log.lock().unwrap().env.is_empty());
        s.combat.slot_env_hits(&mut scene, &[env_hit(ID, 1, 3)], t);
        assert_eq!(s.combat.slot_enemies()[0].hp(), 1);
        assert_eq!(log.lock().unwrap().env[0].1.source, EnvSource::Hazard(2));
        assert!(defeats(&s).is_empty());
        s.combat.slot_env_hits(&mut scene, &[env_hit(ID, 2, 1)], t);
        assert_eq!(defeats(&s).len(), 1, "the existing EnemyDefeated");
        // A defeated victim has no body and takes nothing more.
        assert!(s.combat.slot_victim_bodies().is_empty());
        s.combat.slot_env_hits(&mut scene, &[env_hit(ID, 3, 1)], t);
        assert_eq!(log.lock().unwrap().env.len(), 2);
    }

    #[test]
    fn a_slot_enemys_fall_goes_through_the_death_presenter_by_species() {
        use crate::death_present::DeathPresenter;
        use crate::gore::{DeathArt, GorePreset};
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let art = Arc::new(DeathArt::load(&root).unwrap());
        let variant_b = DeathArt::load_variant(&root, 1).unwrap();
        let drawn = |species: Option<&'static str>| {
            let mut d = Dummy::new(ID, CARL_X + 200.0, foot());
            d.death_species = species;
            let mut s = sim(&SOLO, vec![d]);
            let mut p =
                DeathPresenter::with_variants(art.clone(), &variant_b, GorePreset::Full).unwrap();
            let mut scene = s.scene.clone();
            let t = tick(&s) + 1;
            s.combat.slot_env_hits(&mut scene, &[env_hit(ID, 1, 9)], t);
            assert_eq!(defeats(&s).len(), 1);
            p.observe(&s.combat, &s.scene);
            assert_eq!(
                s.combat.slot_death_body(ID).map(|b| b.species),
                species,
                "a fallen enemy still reports its body"
            );
            (p.stats().parts_spawned, p.live_parts())
        };
        // No species: nothing is drawn (today's behaviour for an unknown id).
        assert_eq!(drawn(None), (0, 0));
        // A species with drawn art plays its finish; a species without a profile
        // (or whose finish has no art yet) draws nothing; the simulation is
        // untouched either way.
        for species in ["goblin", "brute_boss"] {
            let (parts, live) = drawn(Some(species));
            assert!(
                live > 0 && parts > 0,
                "{species}: {parts} parts, {live} live"
            );
        }
        assert_eq!(drawn(Some("no_such_species")), (0, 0));
        assert_eq!(drawn(Some("skeleton_mock")), (0, 0));
        // An unknown id has no body.
        let (d, _) = dummy_at(200.0);
        assert!(sim(&SOLO, vec![d]).combat.slot_death_body(ID + 1).is_none());
    }

    #[test]
    fn an_immune_enemy_takes_no_env_hit() {
        let (mut d, log) = dummy_at(200.0);
        d.victim = false;
        let mut s = sim(&SOLO, vec![d]);
        let mut scene = s.scene.clone();
        s.combat.slot_env_hits(&mut scene, &[env_hit(ID, 1, 3)], 1);
        assert!(log.lock().unwrap().env.is_empty());
        assert_eq!(s.combat.slot_enemies()[0].hp(), 4);
    }

    // ------------------------------------------------------- threat feed --

    #[test]
    fn the_threat_is_the_single_suppliers_and_a_quiet_enemy_or_a_corpse_feeds_nothing() {
        let pound = BossThreat::Pound {
            x: 300.0,
            facing: -1,
            target: None,
        };
        let supplier = |supplies: bool, hp: u16| {
            let (mut d, _) = dummy_at(100.0);
            d.threat = Some(pound);
            d.supplies = supplies;
            d.hp = hp;
            d
        };
        assert_eq!(
            sim(&SOLO, vec![supplier(true, 4)]).combat.boss_threat(),
            Some(pound)
        );
        assert_eq!(
            sim(&SOLO, vec![supplier(false, 4)]).combat.boss_threat(),
            None
        );
        assert_eq!(
            sim(&SOLO, vec![supplier(true, 0)]).combat.boss_threat(),
            None
        );
        assert_eq!(sim(&SOLO, Vec::new()).combat.boss_threat(), None);
        // Two of one kind: the first living supplier in slot order.
        let mut a = supplier(true, 4);
        a.hp = 0;
        let mut b = supplier(true, 4);
        b.id = ID + 1;
        let alone = sim(&SOLO, vec![a, b]);
        assert_eq!(alone.combat.boss_threat(), Some(pound));
    }

    #[test]
    fn a_supplier_beside_a_fixture_boss_is_refused_and_a_quiet_slot_enemy_is_not() {
        let kit =
            crate::boss_scene::load_arena(&PathBuf::from(env!("CARGO_MANIFEST_DIR"))).unwrap();
        let scene = crate::corridor::SceneSim::new_at(&kit, 360, 380).unwrap();
        let build = |supplies: bool| {
            let (mut d, _) = dummy_at(0.0);
            d.supplies = supplies;
            // `SlotSet::new` is told there is no boss: the state checks again.
            let set = SlotSet::new(vec![d.boxed()], false).unwrap();
            CombatState::new(BOSS_SOLO.base, &kit, &scene)
                .unwrap()
                .with_slot_enemies(set)
        };
        assert_eq!(
            build(true).unwrap_err(),
            SlotError::FixtureBossConflict { kind: "test_dummy" }
        );
        assert!(build(false).is_ok());
    }

    /// Donut's x after `ticks` of a run whose slot enemy feeds a pound tell just
    /// to her right, facing her, 10 px away: the bot steps out leftward at once.
    fn donut_x_after(fixture: &'static PartyFixture, supplies: bool, ticks: u64) -> f32 {
        let probe = sim(fixture, Vec::new());
        let dx = probe.scene.world.donut_body.x;
        let (mut d, _) = dummy_at(0.0);
        d.x = 900.0;
        d.supplies = supplies;
        d.threat = Some(BossThreat::Pound {
            x: dx + 10.0,
            facing: -1,
            target: None,
        });
        let mut s = sim(fixture, vec![d]);
        idle(&mut s, ticks);
        s.scene.world.donut_body.x
    }

    #[test]
    fn site_one_the_plain_bot_dodges_a_slot_enemys_pound_like_the_bosss() {
        let fed = donut_x_after(&SOLO, true, 6);
        let ignored = donut_x_after(&SOLO, false, 6);
        assert!(
            fed < ignored - 1.0,
            "the bot stepped out leftward: fed {fed}, ignored {ignored}"
        );
    }

    #[test]
    fn site_two_the_companion_view_and_follow_layer_read_the_same_threat() {
        let fed = donut_x_after(&COMPANION, true, 6);
        let ignored = donut_x_after(&COMPANION, false, 6);
        assert!(
            fed < ignored - 1.0,
            "the companion's follow layer stepped out: fed {fed}, ignored {ignored}"
        );
    }

    // ----------------------------------------------- restart and respawn --

    #[test]
    fn a_restart_returns_every_slot_enemy_to_its_spawn_state() {
        let (mut d, _) = dummy_at(30.0);
        d.hp = 1;
        let mut s = sim(&SOLO, vec![d]);
        let fresh = s.combat.clone();
        step(&mut s, ATTACK, RawInput::default());
        idle(&mut s, 8);
        assert!(!s.combat.slot_enemies()[0].alive());
        s.combat.restart(&mut s.scene);
        assert!(s.combat.slot_enemies()[0].alive());
        assert_eq!(s.combat.slot_enemies()[0].hp(), 1);
        assert_eq!(s.combat.slot_enemies().len(), 1);
        // The slot's own state hashes like a fresh one.
        let slot_hash = |c: &CombatState| {
            let mut words = Vec::new();
            c.slot.mix_into(&mut |w| words.push(w));
            words
        };
        assert_eq!(slot_hash(&s.combat), slot_hash(&fresh));
    }

    static DEMO: DemoRules = DemoRules {
        checkpoints: &[400],
    };

    #[test]
    fn a_demo_respawn_restores_a_living_slot_enemy_and_leaves_a_defeated_one_down() {
        let (mut living, _) = dummy_at(300.0);
        living.hp = 4;
        let mut dead = Dummy::new(ID + 1, CARL_X + 340.0, foot());
        dead.hp = 4;
        let kit = kit();
        let scene = crate::corridor::SceneSim::new_at(&kit, 360, 400).unwrap();
        let set = SlotSet::new(vec![living.boxed(), dead.boxed()], false).unwrap();
        let combat = CombatState::new(SOLO.base, &kit, &scene)
            .unwrap()
            .with_demo(&DEMO)
            .with_slot_enemies(set)
            .unwrap();
        let mut s = PartyFixtureSim::from_parts(scene, combat, &SOLO);
        // Hurt the first, kill the second, through the env path.
        let mut scene = s.scene.clone();
        s.combat.slot_env_hits(&mut scene, &[env_hit(ID, 1, 2)], 1);
        s.combat
            .slot_env_hits(&mut scene, &[env_hit(ID + 1, 1, 9)], 1);
        assert_eq!(s.combat.slot_enemies()[0].hp(), 2);
        assert!(!s.combat.slot_enemies()[1].alive());
        s.combat.respawn(&mut s.scene);
        assert_eq!(
            s.combat.slot_enemies()[0].hp(),
            4,
            "back at its spawn state"
        );
        assert!(
            !s.combat.slot_enemies()[1].alive(),
            "a defeated one stays down"
        );
    }

    #[test]
    fn a_slot_defeat_beside_the_morale_and_the_goblins_breaks_nothing() {
        // Under cf02 the goblin morale judges every tick's events: a slot id is
        // above every goblin and llama index, so it is invisible to it.
        let (mut d, _) = dummy_at(0.0);
        d.x = 560.0;
        let mut s = sim(&GOBLIN_SOLO, vec![d]);
        s.combat = s.combat.clone().with_cf02();
        let mut scene = s.scene.clone();
        s.combat.slot_env_hits(&mut scene, &[env_hit(ID, 1, 9)], 1);
        idle(&mut s, 40);
        assert_eq!(defeats(&s).len(), 1);
        assert!(s.combat.enemies[0].alive(), "the goblin is untouched");
    }
}
