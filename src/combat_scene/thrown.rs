//! CARL-PROG-01 CP-B1: the combat state's hooks into the thrown charge.
//!
//! The pure state machine (press gate, throw clock, stick flight, recharge) is
//! [`crate::thrown_charge`]; the blast data and line of sight are in
//! [`crate::blast`]. This file feeds them: Carl's press, the solids, the hurt
//! events; it turns their outcomes into events and the one reserved
//! [`BlastCharge`] slot, and resolves a non-QA blast against the goblins, the
//! llamas, the boss and the heroes (CP-Q20..CP-Q26, K3).
//!
//! A run whose hero never holds the capability has an idle [`Thrower`]: every
//! function below returns at its first line, nothing is evented and nothing is
//! hashed, so every golden is byte-identical (spec section 8).
//!
//! Tick order (appended to the existing order): the melee request is gated and
//! the special press resolves right after Carl's strike (`step_special`), the
//! scene physics run, then the stick step (`step_stick`: flight, landing, the
//! release on t41), then goblins, llamas, contacts, shore, boss, and the
//! charges in index order (fixture charges first, then the one reserved slot).

use super::CombatState;
use crate::blast::{
    BlastBreak, BlastCharge, BlastReach, BlastTarget, ChainRule, blast_break, closest_point,
    line_of_sight,
};
use crate::boss::{BOSS_ENEMY_ID, Boss, BossHit, HeroBlow};
use crate::carl_prog::CarlProgTuning;
use crate::combat::{CombatEvent, DefeatCause};
use crate::corridor::SceneSim;
use crate::goblin::BlastOuter;
use crate::llama::LlamaBlast;
use crate::party::HeroId;
use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Rect};
use crate::shore::PLAYER_ONE;
use crate::story_unlock::SPECIAL_THROWN_CHARGE;
use crate::thrown_charge::{HurtOutcome, PressView, Thrower, blast_profile};

/// CP-Q34: the chain rule's knobs from `carl_prog_v0.json`.
pub fn chain_rule(t: &CarlProgTuning) -> ChainRule {
    ChainRule {
        delay: u64::from(t.cpq34_chain_delay_ticks),
        max_depth: t.cpq34_chain_max_depth,
        max_per_tick: usize::from(t.cpq34_chain_max_per_tick),
    }
}

/// CP-Q35: the boss rack strut as a [`BlastTarget`] for one blast: the drop is
/// credited to the blast's owner and its event is collected for the caller.
struct RackStrut<'a> {
    boss: &'a mut Boss,
    tick: u64,
    events: Vec<CombatEvent>,
}

impl BlastTarget for RackStrut<'_> {
    fn blast_footprint(&self) -> Option<Rect> {
        self.boss.strut_blast_footprint()
    }

    fn break_by_blast(&mut self, blast: BlastBreak) -> bool {
        let player = blast.owner.unwrap_or(PLAYER_ONE);
        self.boss
            .blast_drop_strut(player, self.tick, &mut self.events)
    }
}

/// The hero index of Carl in events (`CapabilityGranted { hero }` convention).
const CARL: u8 = 0;

/// The launch id of an outer-ring launch (CP-Q21: "a new launch instance"):
/// above the move instances and below the ram stamps (`0x8000_0000 | seq`),
/// unique per charge slot and tick.
fn blast_launch_id(charge_index: usize, tick: u64) -> u32 {
    0x4000_0000 | (((charge_index as u32) & 0xFF) << 22) | ((tick as u32) & 0x3F_FFFF)
}

/// +1 / -1: the way a body is thrown from the blast column (`facing` breaks a
/// tie, as the DEATH-D rule does).
fn away(body_x: f32, charge_x: f32, facing: i8) -> i8 {
    let dx = body_x - charge_x;
    if dx > 0.0 {
        1
    } else if dx < 0.0 {
        -1
    } else {
        -facing
    }
}

impl CombatState {
    /// The index of the one reserved dynamite charge slot: **after every
    /// authored charge** (the fixture's, then any sticker charges appended with
    /// [`Self::push_authored_charge`]), CP-Q31. The only place the slot index is
    /// computed: arming, clearing and the detonation test all use it.
    pub fn dynamite_slot(&self) -> usize {
        self.authored_charges
    }

    /// Append an authored charge (a sticker's blast, a trap) after the fixture
    /// charges, before any throw. The reserved thrown-charge slot stays after it,
    /// so a throw never overwrites it, its blast is never taken for the stick's
    /// and a respawn never deletes it. Called when an attempt is built; HAZ-A2 is
    /// the first user and the one allowed editor of this function besides CP-B1.
    ///
    /// # Panics
    ///
    /// In debug builds when the thrown-charge slot already exists (an authored
    /// charge must come first).
    pub fn push_authored_charge(&mut self, charge: BlastCharge) {
        debug_assert_eq!(
            self.charges.len(),
            self.authored_charges,
            "an authored charge is appended before the thrown-charge slot exists"
        );
        self.charges.push(charge);
        self.authored_charges = self.charges.len();
    }

    /// The thrower (idle unless Carl pressed the special).
    pub fn thrower(&self) -> &Thrower {
        &self.thrower
    }

    /// A melee request on this tick, gated by a running throw: nothing cancels
    /// the throw before t50 and a request before that is lost; from t50 a melee
    /// move ends the recovery (CP-Q14). Identity while no throw runs.
    pub(super) fn special_gate_request<T>(&mut self, tick: u64, request: Option<T>) -> Option<T> {
        if self.thrower.is_idle() {
            return request;
        }
        if request.is_some() {
            self.thrower.cancel_recovery(tick);
        }
        if self.thrower.blocks_melee(tick) {
            None
        } else {
            request
        }
    }

    /// Whether a running throw ignores Carl's jump press on this tick (CP-Q15).
    pub(super) fn special_strips_jump(&self, tick: u64) -> bool {
        self.thrower.strips_jump(tick)
    }

    /// Carl's special step (CP-Q12, CP-Q13): the recharge's ready tick, the end
    /// of the throw clock, a jump that cancels the recovery, and the Circle/B
    /// press edge. Runs after Carl's strike, before the scene physics, and only
    /// while the capability is held (a press without it is inert and silent).
    /// The AI never starts a throw (CP-Q29): a bot-driven Carl presses nothing.
    pub(super) fn step_special(&mut self, scene: &SceneSim, tick: u64, hero_frozen: bool) {
        if !self.story_holds(SPECIAL_THROWN_CHARGE) {
            return;
        }
        if self.thrower.take_ready(tick) {
            self.log(CombatEvent::SpecialReady { tick, hero: CARL });
        }
        self.thrower.end_clock(tick);
        if self.input.jump_pressed {
            self.thrower.cancel_recovery(tick);
        }
        if !self.input.ability_pressed || scene.drive().is_bot(HeroId::Carl) {
            return;
        }
        let view = PressView {
            grounded: scene.world.hero.grounded,
            hurt: self.hurt_ticks > 0,
            defeated: self.hero_defeated,
            frozen: hero_frozen,
            melee_busy: scene.world.hero.melee.busy(),
        };
        match self.thrower.press(tick, &view) {
            Ok(()) => self.log(CombatEvent::SpecialStarted {
                tick,
                hero: CARL,
                capability: SPECIAL_THROWN_CHARGE.as_str(),
            }),
            Err(reason) => self.log(CombatEvent::SpecialRejected {
                tick,
                hero: CARL,
                reason,
            }),
        }
    }

    /// The stick step, after the scene physics and before the goblins: the
    /// flight and landing of a live stick, then the release of a throw on t41
    /// (from Carl's pivot after this tick's move, facing and speed). A stick
    /// released this tick does not fly until the next one.
    pub(super) fn step_stick(&mut self, scene: &SceneSim, tick: u64) {
        if self.thrower.is_idle() {
            return;
        }
        let slot = self.dynamite_slot();
        let x_range = (HERO_HALF_WIDTH, self.x_max - HERO_HALF_WIDTH);
        let step = self.thrower.step_stick(&scene.world.solids, x_range);
        if let Some(stick) = self.thrower.stick().copied() {
            // The charge follows the stick (its detonation reads this column).
            if let Some(charge) = self.charges.get_mut(slot) {
                charge.x = stick.x;
                charge.foot_y = stick.y;
            }
            if step.landed {
                self.log(CombatEvent::ChargeLanded {
                    tick,
                    charge: slot as u32,
                    x: stick.x.round() as i32,
                    y: stick.y.round() as i32,
                });
            }
        }
        if self.thrower.releases_now(tick) {
            let hero = &scene.world.hero;
            self.thrower.release(
                tick,
                hero.x,
                hero.y - HERO_HALF_HEIGHT,
                hero.facing,
                hero.vx,
            );
            self.arm_slot(tick);
        }
    }

    /// Put the live stick into the one reserved charge slot (created on the
    /// first throw, reused after: the charge vector never grows past it) and
    /// log the throw. The charge is credited to Carl's player (CP-Q30: the
    /// player of Carl's body is slot 0 in solo and in 2P).
    fn arm_slot(&mut self, tick: u64) {
        let Some(stick) = self.thrower.stick().copied() else {
            return;
        };
        let slot = self.dynamite_slot();
        let charge = BlastCharge::new(stick.x, stick.y, stick.detonate_tick, Some(PLAYER_ONE))
            .with_profile(blast_profile(CarlProgTuning::active()));
        if self.charges.len() > slot {
            self.charges[slot] = charge;
        } else {
            self.charges.push(charge);
        }
        self.log(CombatEvent::ChargeThrown {
            tick,
            charge: slot as u32,
            x: stick.x.round() as i32,
            y: stick.y.round() as i32,
        });
    }

    /// A hit that landed on Carl (called from the two hurt paths): cancel the
    /// throw, drop the lit stick at his feet or only end the recovery (CP-Q27).
    pub(super) fn thrower_hurt(&mut self, scene: &SceneSim, tick: u64) {
        if self.thrower.is_idle() {
            return;
        }
        let hero = &scene.world.hero;
        let outcome = self
            .thrower
            .on_hurt(tick, hero.x, hero.y - HERO_HALF_HEIGHT);
        if outcome == HurtOutcome::Drop {
            self.arm_slot(tick);
        }
    }

    /// The restart rule (CP-Q28): the stick is gone without a blast and the
    /// special is ready; the capability stays (the ledger decides that).
    pub(super) fn clear_thrower(&mut self) {
        self.thrower = Thrower::default();
        self.charges.truncate(self.dynamite_slot());
    }

    /// The thrown charge blew: its recharge starts on this tick.
    pub(super) fn thrower_detonated(&mut self, charge_index: usize, tick: u64) {
        if charge_index == self.dynamite_slot() {
            self.thrower.detonated(tick);
        }
    }

    /// Whether this charge's blast chains onward (CP-Q34): the dynamite's own
    /// blast (the reserved slot) and any blast a chain has reached
    /// (`chain_depth > 0`). A QA or world charge that no dynamite has reached
    /// keeps its own rule and chains nothing. BOIL-D's drum is reached by a chain
    /// (depth >= 1) like any other charge, so it needs no flag of its own.
    fn chains_from(&self, index: usize) -> bool {
        self.charges[index].chain_depth > 0 || index == self.dynamite_slot()
    }

    /// CP-Q34: the chain reaction of the blast of charge `source` on `tick`: every
    /// **authored** charge (fixture and sticker charges; never the thrown stick
    /// itself) whose centre lies within this blast's outer radius, with line of
    /// sight, is lit, pulled to or left at `tick + 8` ([`BlastCharge::chain_set`]).
    /// The depth is `source depth + 1`; a blast at the deepest link chains nothing
    /// (CP-Q34: depth 4 at most). Targets are visited in charge-index order, and
    /// the initiator's credit is inherited only by a charge with no owner.
    /// Evaluated before the blast changes anything this tick (CP-Q22).
    pub(super) fn chain_from(&mut self, scene: &SceneSim, source: usize, tick: u64) {
        if !self.chains_from(source) {
            return;
        }
        let rule = chain_rule(CarlProgTuning::active());
        let origin = self.charges[source];
        if origin.chain_depth >= rule.max_depth {
            return;
        }
        let depth = origin.chain_depth + 1;
        let (cx, cy) = origin.centre();
        let reach = origin.profile.outer_radius;
        for target in 0..self.authored_charges {
            if target == source || self.charges[target].detonated {
                continue;
            }
            let (tx, ty) = self.charges[target].centre();
            let (dx, dy) = (tx - cx, ty - cy);
            if dx * dx + dy * dy > reach * reach
                || !line_of_sight(&scene.world.solids, (cx, cy), (tx, ty))
            {
                continue;
            }
            if let Some(due) = self.charges[target].chain_set(tick, rule.delay, origin.owner, depth)
            {
                self.log(CombatEvent::ChargeChained {
                    tick,
                    charge: target as u32,
                    source: source as u32,
                    due,
                    depth,
                });
            }
        }
    }

    /// CP-Q35: a blast with `breaks_structures` breaks the CP-03 shore strike
    /// zone and the boss rack strut it reaches (outer circle, line of sight),
    /// outright, credited to the blast's owner (player one for an owner-less
    /// blast). Each is a [`BlastTarget`]; the env-solid index range is never
    /// targeted (KC17: a prop is its own target, BOIL-D).
    pub(super) fn blast_structures(
        &mut self,
        scene: &SceneSim,
        index: usize,
        charge: &BlastCharge,
        tick: u64,
    ) {
        if !charge.profile.breaks_structures {
            return;
        }
        let id = blast_launch_id(index, tick);
        let solids = &scene.world.solids;
        let broke = self
            .shore
            .as_mut()
            .is_some_and(|shore| blast_break(charge, id, solids, shore));
        if broke {
            self.log(CombatEvent::ShoreBroken {
                tick,
                player: charge.owner.unwrap_or(PLAYER_ONE),
            });
        }
        if let Some(mut boss) = self.boss.take() {
            let mut strut = RackStrut {
                boss: &mut boss,
                tick,
                events: Vec::new(),
            };
            blast_break(charge, id, solids, &mut strut);
            let events = strut.events;
            self.boss = Some(boss);
            for event in events {
                self.log(event);
            }
        }
    }

    /// One blast with a non-QA profile (CP-Q20..CP-Q26): the goblins (and
    /// Engineers) in index order, the llamas, the boss, then the heroes in
    /// hero order. Squared compares only; line of sight is evaluated for every
    /// target before this blast changes anything (CP-Q22). The QA profile never
    /// comes here.
    pub(super) fn resolve_profile_blast(
        &mut self,
        scene: &mut SceneSim,
        charge_index: usize,
        charge: BlastCharge,
        tick: u64,
    ) {
        let profile = charge.profile;
        let (cx, cy) = charge.centre();
        let owner = charge.owner;
        let blocked = |solids: &[crate::prototype::Block], body: Rect| {
            profile.los && !line_of_sight(solids, (cx, cy), closest_point(body, (cx, cy)))
        };
        let explosion = |point_blank: bool| DefeatCause::Explosion {
            charge: charge_index as u32,
            player: owner,
            point_blank,
        };

        // Goblins and Engineers.
        for index in 0..self.enemies.len() {
            if !self.enemies[index].alive() {
                continue;
            }
            let body = self.enemies[index].body();
            let Some(reach) = charge.reach(body) else {
                continue;
            };
            if blocked(&scene.world.solids, body) {
                continue;
            }
            let was_attacking = self.enemies[index].token_held();
            let way = away(self.enemies[index].x, charge.x, self.enemies[index].facing);
            let defeated_as = match reach {
                BlastReach::PointBlank => self.enemies[index].blast(way, true).then_some(true),
                BlastReach::Ring => self.enemies[index].blast(way, false).then_some(false),
                BlastReach::Outer => {
                    let launch = blast_launch_id(charge_index, tick);
                    match self.enemies[index].blast_outer(
                        way,
                        profile.outer_damage,
                        profile.launch,
                        launch,
                    ) {
                        Some(BlastOuter::Defeated) => Some(false),
                        Some(BlastOuter::Launched { .. }) => {
                            // The thrower is credited with the launch, so a wall
                            // slam or a later block kill is the thrower's.
                            self.enemies[index].hit_by = owner.or(self.enemies[index].hit_by);
                            None
                        }
                        None => continue,
                    }
                }
            };
            if let Some(point_blank) = defeated_as {
                let instigator = owner.or(self.enemies[index].hit_by);
                self.log(CombatEvent::enemy_defeated(
                    tick,
                    index as u32,
                    explosion(point_blank),
                    instigator,
                    None,
                ));
            }
            if was_attacking && self.enemies[index].mode != crate::goblin::GoblinMode::Attack {
                self.log(CombatEvent::GoblinInterrupted {
                    tick,
                    goblin: index as u32,
                });
            }
            if let Some(slot) = self.flash_tick.get_mut(index) {
                *slot = Some(tick);
            }
        }

        // Llamas (CP-Q26): the core kills, the outer ring hurts with the kick
        // reaction.
        let first_llama = self.enemies.len();
        for index in 0..self.llamas.len() {
            if !self.llamas[index].alive() {
                continue;
            }
            let body = self.llamas[index].body();
            let Some(reach) = charge.reach(body) else {
                continue;
            };
            if blocked(&scene.world.solids, body) {
                continue;
            }
            let was_telling = self.llamas[index].telling();
            let way = away(self.llamas[index].x, charge.x, self.llamas[index].facing);
            let defeated_as = match reach {
                BlastReach::PointBlank => self.llamas[index].blast(way).then_some(true),
                BlastReach::Ring => self.llamas[index].blast(way).then_some(false),
                BlastReach::Outer => {
                    match self.llamas[index].blast_outer(way, profile.llama_outer_damage) {
                        Some(LlamaBlast::Defeated) => Some(false),
                        Some(LlamaBlast::Hurt { .. }) => None,
                        None => continue,
                    }
                }
            };
            self.llamas[index].hit_by = owner.or(self.llamas[index].hit_by);
            if let Some(point_blank) = defeated_as {
                self.log(CombatEvent::enemy_defeated(
                    tick,
                    (first_llama + index) as u32,
                    explosion(point_blank),
                    owner.or(self.llamas[index].hit_by),
                    None,
                ));
            }
            if was_telling && self.llamas[index].mode != crate::llama::LlamaMode::Attack {
                self.log(CombatEvent::LlamaInterrupted {
                    tick,
                    llama: index as u32,
                });
            }
            if let Some(slot) = self.flash_tick.get_mut(first_llama + index) {
                *slot = Some(tick);
            }
        }

        // The boss (CP-Q25): damage anywhere in reach, counted toward poise as
        // damage, super armour holds (it never interrupts a tell or a strike),
        // no launch. Never a point-blank burst: a boss is not exploded.
        if profile.boss_damage > 0
            && let Some(mut boss) = self.boss.take()
        {
            if let Some(hurtbox) = boss.hurtbox()
                && charge.reach(hurtbox).is_some()
                && !blocked(&scene.world.solids, hurtbox)
            {
                let player = owner.unwrap_or(PLAYER_ONE);
                let mut events = Vec::new();
                let landed = boss.apply_hit(
                    BossHit {
                        damage: u16::from(profile.boss_damage),
                        poise: u16::from(profile.boss_damage),
                        player,
                        hit_stop: 0,
                    },
                    &mut events,
                    tick,
                );
                if landed.is_some_and(|result| result.defeated) {
                    events.push(CombatEvent::enemy_defeated(
                        tick,
                        BOSS_ENEMY_ID,
                        explosion(false),
                        Some(player),
                        None,
                    ));
                }
                for event in events {
                    self.log(event);
                }
            }
            self.boss = Some(boss);
        }

        // ENEMY-SLOT: the slot enemies' hurtboxes, after the boss.
        self.blast_slot_enemies(scene, charge_index, charge, tick);

        self.blast_heroes(scene, charge, tick);
    }

    /// The slot enemies in slot order (ENEMY-SLOT, the victim loop of a non-QA
    /// blast): the strongest band any hurtbox the line of sight reaches is in
    /// (CP-Q22: squared compares, solids block when the profile says so). The
    /// enemy decides what the band does ([`crate::scene_enemy::SlotEnemy::apply_blast`]);
    /// a defeat is its own `EnemyDefeated`, the cause `Explosion` through
    /// [`crate::scene_enemy::SlotBlast::cause`]. The body is thrown away from the
    /// charge, a column tie leftward. The follow-ups apply right away, before the
    /// heroes take the same blast.
    fn blast_slot_enemies(
        &mut self,
        scene: &mut SceneSim,
        charge_index: usize,
        charge: BlastCharge,
        tick: u64,
    ) {
        use crate::scene_enemy::{SlotBlast, SlotOut, best_reach};
        if self.slot.is_empty() {
            return;
        }
        let (cx, cy) = charge.centre();
        for index in 0..self.slot.enemies.len() {
            let enemy = &self.slot.enemies[index];
            if !enemy.alive() {
                continue;
            }
            let mut rects = enemy.hurt_rects();
            rects.retain(|r| {
                !charge.profile.los
                    || line_of_sight(&scene.world.solids, (cx, cy), closest_point(*r, (cx, cy)))
            });
            let Some(reach) = best_reach(&charge, &rects) else {
                continue;
            };
            let blast = SlotBlast {
                charge_index: charge_index as u32,
                charge,
                reach,
                away: away(rects[0].center().0, charge.x, 1),
                launch: blast_launch_id(charge_index, tick),
            };
            let enemy = &mut self.slot.enemies[index];
            let (id, was_alive) = (enemy.target_id(), true);
            let mut out = SlotOut::default();
            enemy.apply_blast(&blast, tick, &mut out);
            Self::check_defeat_logged(id, was_alive, self.slot.enemies[index].alive(), &out);
            self.apply_slot_out(scene, out, tick);
        }
    }

    /// The heroes in hero order, by the profile's policy (K3): `OwnerReduced`
    /// hurts only the owner hero (reduced) and spares everyone else (CP-Q24, no
    /// friendly fire: nothing damaged, launched or logged), `AllHeroes` hurts
    /// every hero present, `NoHeroes` none. Through the **existing hero-hurt
    /// path** (`hurt_hero`), so invulnerability, the solo last-HP guard and the
    /// off-screen immunity of a guarded hero apply unchanged (CP-Q23).
    fn blast_heroes(&mut self, scene: &mut SceneSim, charge: BlastCharge, tick: u64) {
        use crate::hero_damage::HeroDamagePolicy;
        let profile = charge.profile;
        if !profile.hero_damage.hurts_any_hero() || profile.hero_hp == 0 {
            return;
        }
        let (cx, cy) = charge.centre();
        let owner_hero = charge
            .owner
            .and_then(|player| HeroId::ALL.get(player as usize).copied());
        let live = scene.party_live();
        for id in HeroId::ALL {
            let hurts = match profile.hero_damage {
                HeroDamagePolicy::NoHeroes => false,
                HeroDamagePolicy::OwnerReduced => owner_hero == Some(id),
                HeroDamagePolicy::AllHeroes => true,
            };
            if !hurts || (id == HeroId::Donut && !live) {
                continue;
            }
            let body = Self::hurtbox(scene, id);
            let reach = body.squared_distance_to(cx, cy);
            if reach > profile.outer_radius * profile.outer_radius {
                continue;
            }
            if profile.los
                && !line_of_sight(&scene.world.solids, (cx, cy), closest_point(body, (cx, cy)))
            {
                continue;
            }
            // D1 M4 (solo only): a guarded hero outside the view cannot be hurt.
            let drive = *scene.drive();
            let x = scene.world.body(id).x;
            if drive.is_guarded(id)
                && drive.tuning.solo_offscreen_immunity
                && scene.is_offscreen(x, f32::from(drive.tuning.offscreen_margin_px))
            {
                continue;
            }
            let (knock_vx, knock_vy) = profile.hero_knock;
            self.hurt_hero(
                scene,
                HeroBlow {
                    hero: id,
                    source: profile.hero_source,
                    damage: profile.hero_hp,
                    from_x: charge.x,
                    knock_vx: if x >= charge.x { knock_vx } else { -knock_vx },
                    knock_vy,
                },
                tick,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss_scene::Pilot;
    use crate::combat::EnemyAi;
    use crate::donut_mana::DonutSetup;
    use crate::env_kit::EnvKit;
    use crate::goblin::GoblinMode;
    use crate::input::RawInput;
    use crate::llama::LlamaMode;
    use crate::party::PROVISIONAL_TUNING;
    use crate::party_scene::{PartyFixture, PartyFixtureSim, base};
    use crate::story_unlock::StorySetup;
    use crate::thrown_charge::THROWN_CHARGE_SOURCE;
    use std::path::PathBuf;

    use super::super::{EnemySpawn, Fixture, LlamaSpawn};
    use crate::combat::HERO_HEALTH;

    const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

    fn kit() -> EnvKit {
        EnvKit::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")), LAYOUT).unwrap()
    }

    const fn passive(x: i32) -> EnemySpawn {
        EnemySpawn {
            x,
            ai: EnemyAi::Passive,
        }
    }

    const fn party(base: &'static Fixture, two_player: bool) -> PartyFixture {
        PartyFixture {
            base,
            two_player,
            tuning: PROVISIONAL_TUNING,
            script: &[],
            pilot: Pilot::None,
            donut: DonutSetup::NONE,
        }
    }

    static EMPTY_BASE: Fixture = base("test_dyn_empty", "no enemies", 7, &[], 900);
    static EMPTY_SOLO: PartyFixture = party(&EMPTY_BASE, false);
    static EMPTY_DUO: PartyFixture = party(&EMPTY_BASE, true);

    const PRESS: RawInput = RawInput {
        ability: true,
        ..RawInput::NONE
    };

    /// A party sim of `fixture` whose Carl holds the thrown charge from tick 0
    /// (`held`) or has a ledger that holds nothing.
    fn sim(fixture: &'static PartyFixture, held: bool) -> PartyFixtureSim {
        let kit = kit();
        let scene = crate::corridor::SceneSim::new_at(&kit, 360, fixture.base.carl_x).unwrap();
        let story = if held {
            StorySetup::NONE.with_hold("special_thrown_charge")
        } else {
            StorySetup::NONE.with_trigger("pile_boss_f01_defeated")
        };
        let combat = CombatState::new(fixture.base, &kit, &scene)
            .unwrap()
            .with_donut(fixture.donut)
            .with_story(story);
        PartyFixtureSim::from_parts(scene, combat, fixture)
    }

    fn idle(sim: &mut PartyFixtureSim, ticks: u64) {
        for _ in 0..ticks {
            sim.step_slots([RawInput::default(); 2]);
        }
    }

    /// One tick of Circle/B on slot 0 (a press edge when the previous tick was idle).
    fn press(sim: &mut PartyFixtureSim) {
        sim.step_slots([PRESS, RawInput::default()]);
    }

    fn tick_of(sim: &PartyFixtureSim) -> u64 {
        sim.scene.world.tick
    }

    fn lines_of(sim: &PartyFixtureSim, kind: &str) -> Vec<String> {
        sim.combat
            .events
            .iter()
            .map(ToString::to_string)
            .filter(|l| l.contains(&format!("kind={kind}")))
            .collect()
    }

    /// The press tick of every test: three idle ticks, then Circle.
    const P: u64 = 4;
    /// The release (t41), the landing of a standing throw (38 ticks later on
    /// flat ground) and the detonation (150 ticks after the release).
    const RELEASE: u64 = P + 41;
    const LANDING: u64 = RELEASE + 38;
    const BLAST: u64 = RELEASE + 150;

    /// Press at tick `P` on a fresh sim and return it right after.
    fn thrown(fixture: &'static PartyFixture) -> PartyFixtureSim {
        let mut s = sim(fixture, true);
        idle(&mut s, 3);
        press(&mut s);
        assert_eq!(tick_of(&s), P);
        s
    }

    fn run_to(sim: &mut PartyFixtureSim, tick: u64) {
        assert!(tick_of(sim) <= tick, "already past {tick}");
        while tick_of(sim) < tick {
            idle(sim, 1);
        }
    }

    fn blasts(sim: &PartyFixtureSim) -> Vec<u64> {
        sim.combat
            .events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::Blast { tick, .. } => Some(*tick),
                _ => None,
            })
            .collect()
    }

    fn hero_hurts(sim: &PartyFixtureSim) -> Vec<(u64, u32, u8, HeroId)> {
        sim.combat
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

    fn stick_x(sim: &PartyFixtureSim) -> f32 {
        sim.combat.thrower().stick().unwrap().x
    }

    /// A hit on Carl after the step of the tick just run, as a goblin's swing
    /// would land it in the goblin phase of that tick.
    fn hit_carl(sim: &mut PartyFixtureSim, damage: u8) -> bool {
        let tick = tick_of(sim);
        sim.combat.hurt_hero(
            &mut sim.scene,
            HeroBlow {
                hero: HeroId::Carl,
                source: 0,
                damage,
                from_x: 450.0,
                knock_vx: -100.0,
                knock_vy: 0.0,
            },
            tick,
        )
    }

    // ------------------------------------------------------------ the gate --

    /// 80 ticks of Circle/B before any grant: no event, and the run hashes like
    /// the same run on a state that never had a ledger.
    #[test]
    fn carl_prog_special_locked() {
        let mut locked = sim(&EMPTY_SOLO, false);
        let mut legacy = {
            let kit = kit();
            let scene = crate::corridor::SceneSim::new_at(&kit, 360, 400).unwrap();
            let combat = CombatState::new(&EMPTY_BASE, &kit, &scene).unwrap();
            PartyFixtureSim::from_parts(scene, combat, &EMPTY_SOLO)
        };
        for tick in 1..=80u64 {
            let raw = if tick % 2 == 0 {
                PRESS
            } else {
                RawInput::default()
            };
            locked.step_slots([raw, RawInput::default()]);
            legacy.step_slots([raw, RawInput::default()]);
        }
        assert!(
            locked.combat.events.is_empty(),
            "{:?}",
            locked.combat.events
        );
        assert!(locked.combat.thrower().is_idle());
        assert!(locked.combat.charges.is_empty());
        assert_eq!(locked.state_hash(), legacy.state_hash());
    }

    // ----------------------------------------------------------- the throw --

    #[test]
    fn carl_prog_dyn_arc_standing() {
        let mut s = thrown(&EMPTY_SOLO);
        idle(&mut s, 700);
        let started = lines_of(&s, "special_started");
        assert_eq!(
            started,
            [format!(
                "COMBAT_EVENT tick={P} kind=special_started hero=0 capability=special_thrown_charge"
            )]
        );
        // Spawn on t41 from pivot + (16, 46 up); lands 38 ticks later about
        // 111 px ahead; detonates exactly 150 ticks after the release.
        assert_eq!(
            lines_of(&s, "charge_thrown"),
            [format!(
                "COMBAT_EVENT tick={RELEASE} kind=charge_thrown charge=0 x=416 y=138"
            )]
        );
        assert_eq!(
            lines_of(&s, "charge_landed"),
            [format!(
                "COMBAT_EVENT tick={LANDING} kind=charge_landed charge=0 x=511 y=92"
            )]
        );
        assert_eq!(blasts(&s), [BLAST]);
        assert_eq!(s.combat.charges.len(), 1, "the one reserved slot");
        assert_eq!(s.combat.dynamite_slot(), 0);
        // The recharge ends exactly 420 ticks after the detonation.
        assert_eq!(
            lines_of(&s, "special_ready"),
            [format!(
                "COMBAT_EVENT tick={} kind=special_ready hero=0",
                BLAST + 420
            )]
        );
        assert!(s.combat.thrower().is_idle());
    }

    #[test]
    fn carl_prog_dyn_arc_moving() {
        // The same run as the registered fixture: Carl runs right from tick 1 and
        // presses on tick 5. The stick spawns on t41 (tick 46) at the pivot + 16,
        // carries half his speed, lands on tick 84 at +169 px from the pivot it
        // was thrown from, and blows exactly 150 ticks after the release.
        let mut s = sim(&EMPTY_SOLO, true);
        let run = RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        };
        for tick in 1..=196u64 {
            let raw = if tick == 5 {
                RawInput {
                    ability: true,
                    ..run
                }
            } else {
                run
            };
            s.step_slots([raw, RawInput::default()]);
            if tick == 46 {
                // Walking continued during the throw: the pivot moved on.
                assert!(s.scene.world.hero.x > 400.0 + 100.0);
            }
        }
        assert_eq!(
            lines_of(&s, "charge_thrown"),
            ["COMBAT_EVENT tick=46 kind=charge_thrown charge=0 x=558 y=138"]
        );
        assert_eq!(
            lines_of(&s, "charge_landed"),
            ["COMBAT_EVENT tick=84 kind=charge_landed charge=0 x=711 y=92"]
        );
        assert_eq!(blasts(&s), [46 + 150]);
        // 711 - (558 - 16) = 169: about 170 px ahead, well past the standing 111.
    }

    #[test]
    fn carl_prog_dyn_one_active() {
        let mut s = thrown(&EMPTY_SOLO);
        // Presses during the throw clock, the flight and the fuse are Busy.
        for tick in [P + 10, P + 45, LANDING + 20, BLAST - 1] {
            run_to(&mut s, tick - 1);
            press(&mut s);
        }
        // After the blast: Cooldown, not buffered.
        run_to(&mut s, BLAST);
        assert_eq!(blasts(&s), [BLAST]);
        for tick in [BLAST + 1, BLAST + 200, BLAST + 418] {
            run_to(&mut s, tick - 1);
            press(&mut s);
        }
        let got: Vec<(u64, String)> = lines_of(&s, "special_rejected")
            .iter()
            .map(|l| {
                let t = l.split("tick=").nth(1).unwrap().split(' ').next().unwrap();
                (
                    t.parse().unwrap(),
                    l.split("reason=").nth(1).unwrap().to_owned(),
                )
            })
            .collect();
        let want: Vec<(u64, String)> = [
            (P + 10, "busy"),
            (P + 45, "busy"),
            (LANDING + 20, "busy"),
            (BLAST - 1, "busy"),
            (BLAST + 1, "cooldown"),
            (BLAST + 200, "cooldown"),
            (BLAST + 418, "cooldown"),
        ]
        .map(|(t, r)| (t, r.to_owned()))
        .to_vec();
        assert_eq!(got, want);
        assert_eq!(
            lines_of(&s, "special_started").len(),
            1,
            "nothing was buffered"
        );
        // The edge: a press on detonation + 419 is rejected, on + 420 accepted
        // (the ready event comes first, on the same tick).
        let at = |press_tick: u64| {
            let mut s = thrown(&EMPTY_SOLO);
            run_to(&mut s, press_tick - 1);
            press(&mut s);
            s
        };
        let early = at(BLAST + 419);
        assert_eq!(lines_of(&early, "special_started").len(), 1);
        assert!(
            lines_of(&early, "special_rejected")
                .last()
                .unwrap()
                .contains(&format!(
                    "tick={} kind=special_rejected hero=0 reason=cooldown",
                    BLAST + 419
                ))
        );
        assert!(lines_of(&early, "special_ready").is_empty());
        let on_time = at(BLAST + 420);
        let started = lines_of(&on_time, "special_started");
        assert_eq!(started.len(), 2);
        assert!(started[1].contains(&format!("tick={} ", BLAST + 420)));
        let ready = lines_of(&on_time, "special_ready");
        assert_eq!(
            ready,
            [format!(
                "COMBAT_EVENT tick={} kind=special_ready hero=0",
                BLAST + 420
            )]
        );
    }

    #[test]
    fn carl_prog_dyn_slot_reuse() {
        let mut s = thrown(&EMPTY_SOLO);
        run_to(&mut s, BLAST + 419);
        press(&mut s);
        let second = tick_of(&s);
        assert_eq!(second, BLAST + 420);
        idle(&mut s, 400);
        let blast_lines = lines_of(&s, "blast");
        assert_eq!(blast_lines.len(), 2);
        assert!(
            blast_lines.iter().all(|l| l.contains("charge=0")),
            "{blast_lines:?}"
        );
        let thrown = lines_of(&s, "charge_thrown");
        assert!(thrown.iter().all(|l| l.contains("charge=0")));
        assert_eq!(s.combat.charges.len(), 1, "the charge vector never grows");
        // A restart removes the slot: nothing of the thrower or the charge is
        // mixed any more (the hash returns to the unthrown value).
        s.combat.restart(&mut s.scene);
        assert!(s.combat.charges.is_empty());
        let mut mixed = false;
        s.combat.thrower().mix_into(&mut |_| mixed = true);
        assert!(!mixed);
    }

    #[test]
    fn carl_prog_dyn_airborne_rejected() {
        let mut s = sim(&EMPTY_SOLO, true);
        let jump = RawInput {
            jump: true,
            ..RawInput::NONE
        };
        s.step_slots([jump, RawInput::default()]);
        idle(&mut s, 4);
        assert!(!s.scene.world.hero.grounded, "Carl is in the air");
        let before = tick_of(&s) + 1;
        press(&mut s);
        assert_eq!(
            lines_of(&s, "special_rejected"),
            [format!(
                "COMBAT_EVENT tick={before} kind=special_rejected hero=0 reason=airborne"
            )]
        );
        assert!(
            s.combat.thrower().is_idle(),
            "nothing was spent or buffered"
        );
        idle(&mut s, 120);
        assert!(lines_of(&s, "special_started").is_empty(), "not buffered");
    }

    /// Remove every floor (the walking line and the bedrock under it) right of
    /// column `edge` under Carl: a ledge with a long drop.
    fn make_ledge(s: &mut PartyFixtureSim, edge: f32) {
        let foot = s.scene.world.hero_foot_y();
        let mut add = Vec::new();
        for block in &mut s.scene.world.solids {
            if block.intact && block.rect.top() <= foot && block.rect.right() > edge {
                block.intact = false;
                if block.rect.left < edge {
                    let mut kept = block.clone();
                    kept.intact = true;
                    kept.rect.width = edge - block.rect.left;
                    add.push(kept);
                }
            }
        }
        s.scene.world.solids.extend(add);
    }

    #[test]
    fn carl_prog_dyn_airborne_mid_throw() {
        // A moving throw walks off a ledge: grounded is checked at the press
        // only, so the throw continues and the stick still spawns on t41, from
        // Carl's pivot wherever he is.
        let mut s = sim(&EMPTY_SOLO, true);
        make_ledge(&mut s, 430.0);
        idle(&mut s, 3);
        let run = RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        };
        s.step_slots([
            RawInput {
                ability: true,
                ..run
            },
            RawInput::default(),
        ]);
        let press_tick = tick_of(&s);
        while tick_of(&s) < press_tick + 41 {
            s.step_slots([run, RawInput::default()]);
        }
        assert!(!s.scene.world.hero.grounded, "Carl walked off the ledge");
        let thrown = lines_of(&s, "charge_thrown");
        assert_eq!(thrown.len(), 1, "{thrown:?}");
        assert!(thrown[0].contains(&format!("tick={} ", press_tick + 41)));
        // With nothing below the stick keeps falling and still blows on its tick.
        assert!(lines_of(&s, "charge_landed").is_empty());
        for _ in 0..160 {
            s.step_slots([run, RawInput::default()]);
        }
        assert_eq!(blasts(&s), [press_tick + 41 + 150]);
        assert!(
            lines_of(&s, "charge_landed").is_empty(),
            "no surface, no landing event"
        );
    }

    #[test]
    fn carl_prog_dyn_out_of_world() {
        // Thrown at the world edge: the stick clamps to the column range like a
        // hero and, on a floor, stops there.
        let mut edge = sim(&EMPTY_SOLO, true);
        edge.scene.world.hero.x = 2040.0 - 40.0;
        edge.scene.world.hero.facing = 1;
        idle(&mut edge, 3);
        press(&mut edge);
        idle(&mut edge, 80);
        let max = edge.combat.x_max - crate::prototype::HERO_HALF_WIDTH;
        let stick = edge.combat.thrower().stick().unwrap();
        assert!(stick.x <= max, "{} <= {max}", stick.x);
        assert!(stick.landed, "on a floor it stops at the edge");

        // With no floor it keeps falling, never lands, and the recharge still
        // starts on its fuse tick. The floors go before the release (t41), so the
        // stick really falls.
        let mut pit = sim(&EMPTY_SOLO, true);
        pit.scene.world.hero.x = 2040.0 - 40.0;
        idle(&mut pit, 3);
        press(&mut pit);
        let press_tick = tick_of(&pit);
        run_to(&mut pit, press_tick + 20);
        for block in &mut pit.scene.world.solids {
            block.intact = false;
        }
        run_to(&mut pit, press_tick + 41 + 100);
        let falling = pit.combat.thrower().stick().unwrap();
        assert!(
            !falling.landed && falling.y < 0.0,
            "still falling: {falling:?}"
        );
        assert!(falling.x <= max);
        run_to(&mut pit, press_tick + 41 + 150);
        assert_eq!(blasts(&pit), [press_tick + 41 + 150]);
        assert!(lines_of(&pit, "charge_landed").is_empty());
        assert!(pit.combat.thrower().recharging());
        run_to(&mut pit, press_tick + 41 + 150 + 420);
        assert_eq!(lines_of(&pit, "special_ready").len(), 1);
    }

    // ------------------------------------------------------- blast reach --

    static CORE_BASE: Fixture = base(
        "test_dyn_core",
        "a goblin on the landing",
        7,
        &[passive(511)],
        600,
    );
    static CORE: PartyFixture = party(&CORE_BASE, false);
    static OUTER_BASE: Fixture = base(
        "test_dyn_outer",
        "a goblin 40 px out",
        7,
        &[passive(563)],
        600,
    );
    static OUTER: PartyFixture = party(&OUTER_BASE, false);

    fn explosion_lines(sim: &PartyFixtureSim) -> Vec<String> {
        lines_of(sim, "enemy_defeated")
    }

    #[test]
    fn carl_prog_dyn_core_kill() {
        let mut s = thrown(&CORE);
        run_to(&mut s, BLAST);
        let lines = explosion_lines(&s);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains(&format!("tick={BLAST} ")));
        assert!(lines[0].contains("cause=explosion charge=0"));
        assert!(lines[0].contains("point_blank"), "the burst: {}", lines[0]);
        assert!(lines[0].contains("credit_player=0"), "{}", lines[0]);
        assert!(!s.combat.enemies[0].alive());
        assert_eq!(s.combat.enemies[0].mode, GoblinMode::Airborne);
    }

    #[test]
    fn carl_prog_dyn_outer_launch() {
        let mut s = thrown(&OUTER);
        run_to(&mut s, BLAST);
        // A 2 HP goblin ~40 px from the blast is launched alive with 1 HP, away
        // from the centre, as a new launch credited to the thrower.
        let g = &s.combat.enemies[0];
        assert!(g.alive());
        assert_eq!(g.health, 1);
        assert_eq!(g.mode, GoblinMode::Launched);
        assert_eq!(g.vx, 200.0);
        assert_eq!(g.vy, 220.0);
        assert_eq!(g.hit_by, Some(0));
        assert!(g.launch.is_some_and(|l| l & 0x4000_0000 != 0));
        assert!(explosion_lines(&s).is_empty(), "alive: no defeat event");
        // It flies, lands, and is not killed by the blast again.
        idle(&mut s, 120);
        assert!(s.combat.enemies[0].alive());
        assert_eq!(s.combat.enemies[0].health, 1);
        assert_eq!(blasts(&s), [BLAST]);
        // A second ring hit would be lethal (1 HP): a lethal outer hit is a
        // heavy ring kill, not point blank.
        let mut g = s.combat.enemies[0].clone();
        assert_eq!(
            g.blast_outer(1, 1, (200.0, 220.0), 1),
            Some(crate::goblin::BlastOuter::Defeated)
        );
    }

    static EDGE_BASE: Fixture = base(
        "test_dyn_edge",
        "goblins exactly 64 px and just beyond, both sides",
        7,
        &[passive(476), passive(477), passive(324), passive(323)],
        600,
    );
    static EDGE: PartyFixture = party(&EDGE_BASE, false);

    #[test]
    fn carl_prog_dyn_edge_64() {
        // A hit on t10 drops the stick at Carl's feet (x 400, an integer), so the
        // reach is exact: a body edge 64 px away is reached, 65 px is not.
        let mut s = thrown(&EDGE);
        run_to(&mut s, P + 10);
        assert!(hit_carl(&mut s, 1));
        let drop = tick_of(&s);
        run_to(&mut s, drop + 150);
        let health: Vec<u8> = s.combat.enemies.iter().map(|g| g.health).collect();
        assert_eq!(health, [1, 2, 1, 2], "64 px reached, 65 px not: {health:?}");
        assert_eq!(s.combat.enemies[0].mode, GoblinMode::Launched);
        assert_eq!(s.combat.enemies[2].vx, -200.0, "thrown the other way");
    }

    #[test]
    fn carl_prog_dyn_hit_drops_stick() {
        // t10: the lit stick drops at Carl's feet, its fuse restarts at 150 from
        // the drop tick and it is the live stick.
        let mut s = thrown(&EMPTY_SOLO);
        run_to(&mut s, P + 10);
        assert!(hit_carl(&mut s, 1));
        let drop = tick_of(&s);
        assert_eq!(
            lines_of(&s, "charge_thrown"),
            [format!(
                "COMBAT_EVENT tick={drop} kind=charge_thrown charge=0 x=400 y=92"
            )]
        );
        assert!(
            s.combat.thrower().throw_clock(drop).is_none(),
            "the throw ended"
        );
        run_to(&mut s, drop + 149);
        assert!(blasts(&s).is_empty());
        idle(&mut s, 1);
        assert_eq!(blasts(&s), [drop + 150]);
        // t3 cancels: nothing spent, no stick, no recharge.
        let mut c = thrown(&EMPTY_SOLO);
        run_to(&mut c, P + 3);
        assert!(hit_carl(&mut c, 1));
        assert!(c.combat.thrower().is_idle());
        assert!(c.combat.charges.is_empty());
        idle(&mut c, 300);
        assert!(blasts(&c).is_empty() && lines_of(&c, "charge_thrown").is_empty());
        // Ready at once (after the flinch): a new press is accepted.
        press(&mut c);
        assert_eq!(lines_of(&c, "special_started").len(), 2);
    }

    #[test]
    fn carl_prog_dyn_hit_on_t41() {
        // The stick has already spawned (the special step runs before the
        // goblins): the hit only ends the recovery.
        let mut s = thrown(&EMPTY_SOLO);
        run_to(&mut s, RELEASE);
        let stick = *s.combat.thrower().stick().unwrap();
        assert!(hit_carl(&mut s, 1));
        assert_eq!(*s.combat.thrower().stick().unwrap(), stick, "left alone");
        assert!(s.combat.thrower().throw_clock(RELEASE).is_none());
        assert_eq!(lines_of(&s, "charge_thrown").len(), 1, "no second stick");
        idle(&mut s, 300);
        assert_eq!(blasts(&s), [BLAST]);
        // A hit on t42..t69 does the same.
        let mut late = thrown(&EMPTY_SOLO);
        run_to(&mut late, P + 55);
        assert!(hit_carl(&mut late, 1));
        idle(&mut late, 300);
        assert_eq!(blasts(&late), [BLAST]);
    }

    // ----------------------------------------------------- line of sight --

    #[test]
    fn carl_prog_dyn_los_wall() {
        // A wall between the centre and the goblin shields it; without the wall
        // the same blast launches it.
        let walled = |wall: bool| {
            let mut s = thrown(&OUTER);
            if wall {
                s.scene.world.solids.push(crate::prototype::Block {
                    rect: crate::prototype::Rect {
                        left: 530.0,
                        bottom: 92.0,
                        width: 10.0,
                        height: 100.0,
                    },
                    material: crate::prototype::Material::Stone,
                    intact: true,
                });
            }
            run_to(&mut s, BLAST);
            s
        };
        let open = walled(false);
        assert_eq!(open.combat.enemies[0].health, 1);
        let shielded = walled(true);
        assert_eq!(shielded.combat.enemies[0].health, 2, "blocked: untouched");
        assert_eq!(shielded.combat.enemies[0].mode, GoblinMode::Neutral);
        assert_eq!(blasts(&shielded), [BLAST], "the blast still goes off");
    }

    // ------------------------------------------------------ heroes (K3) --

    fn place_carl_for_blast(s: &mut PartyFixtureSim, dx: f32) {
        let x = s.combat.thrower().stick().unwrap().x + dx;
        s.scene.world.hero.x = x;
    }

    #[test]
    fn carl_prog_dyn_self_damage() {
        let at = |dx: f32| {
            let mut s = thrown(&EMPTY_SOLO);
            run_to(&mut s, LANDING + 5);
            place_carl_for_blast(&mut s, dx);
            run_to(&mut s, BLAST);
            s
        };
        // Inside 64 px with line of sight: 1 damage, the knock, the hurt state
        // and 55 invulnerable ticks, through the existing hero-hurt path.
        let s = at(-30.0);
        assert_eq!(
            hero_hurts(&s),
            [(BLAST, THROWN_CHARGE_SOURCE, HERO_HEALTH - 1, HeroId::Carl)]
        );
        assert_eq!(s.scene.world.hero.invulnerable_ticks, 55);
        assert!(s.combat.hurt_ticks > 0);
        assert_eq!(s.combat.hero_vx, -120.0, "knocked away from the centre");
        assert_eq!(s.scene.world.hero.vy, 160.0);
        // The same blast from the other side knocks the other way.
        assert_eq!(at(30.0).combat.hero_vx, 120.0);
        // Just outside the reach: nothing.
        let far = at(-100.0);
        assert!(hero_hurts(&far).is_empty());
        assert_eq!(far.scene.world.hero.health, HERO_HEALTH);
        // Invulnerable Carl takes nothing (the existing path's rule).
        let mut inv = thrown(&EMPTY_SOLO);
        run_to(&mut inv, LANDING + 5);
        place_carl_for_blast(&mut inv, -30.0);
        inv.scene.world.hero.invulnerable_ticks = 200;
        run_to(&mut inv, BLAST);
        assert!(hero_hurts(&inv).is_empty());
    }

    #[test]
    fn carl_prog_dyn_self_damage_guard() {
        // A human-controlled Carl at 1 HP: the blast is lethal, the party falls.
        let mut s = thrown(&EMPTY_SOLO);
        run_to(&mut s, LANDING + 5);
        place_carl_for_blast(&mut s, -30.0);
        s.scene.world.hero.health = 1;
        run_to(&mut s, BLAST);
        assert!(s.combat.hero_defeated);
        assert_eq!(s.scene.world.hero.health, 0);
        // The AI-driven Carl (a solo swap to Donut) is floored at 1 HP.
        let mut g = thrown(&EMPTY_SOLO);
        run_to(&mut g, P + 20);
        g.step_slots([
            RawInput {
                swap: true,
                ..RawInput::NONE
            },
            RawInput::default(),
        ]);
        assert_eq!(g.party.controlled_hero(), HeroId::Donut);
        run_to(&mut g, BLAST - 1);
        place_carl_for_blast(&mut g, -30.0);
        g.scene.world.hero.health = 1;
        run_to(&mut g, BLAST);
        assert!(!g.combat.hero_defeated, "the guard floors the AI Carl");
        assert_eq!(g.scene.world.hero.health, 1);
        assert_eq!(hero_hurts(&g).len(), 1, "he was hurt, not killed");
    }

    #[test]
    fn carl_prog_dyn_no_friendly_fire() {
        // Donut standing on the stick, solo and 2P: nothing damaged, launched or
        // logged. (Carl stands clear.)
        for fixture in [&EMPTY_SOLO, &EMPTY_DUO] {
            let mut s = thrown(fixture);
            run_to(&mut s, LANDING + 5);
            let x = stick_x(&s);
            s.scene.world.donut_body.x = x;
            s.scene.world.sync_donut_for_tests();
            let health = s.scene.world.donut_body.health;
            run_to(&mut s, BLAST - 1);
            s.scene.world.donut_body.x = x;
            s.scene.world.sync_donut_for_tests();
            let vy_before = s.scene.world.donut_body.vy;
            idle(&mut s, 1);
            assert_eq!(blasts(&s), [BLAST]);
            assert_eq!(
                s.scene.world.donut_body.health, health,
                "two_player={}",
                fixture.two_player
            );
            assert!(
                hero_hurts(&s).iter().all(|h| h.3 != HeroId::Donut),
                "no Donut hurt event"
            );
            assert_eq!(s.combat.donut.hurt_ticks, 0);
            assert!(
                s.scene.world.donut_body.vy <= vy_before.max(0.0),
                "not launched"
            );
        }
    }

    // ---------------------------------------------------- ownership, swaps --

    #[test]
    fn carl_prog_dyn_swap_keeps_stick() {
        // A solo swap mid-fuse changes nothing: the stick, its fuse and the
        // credit belong to Carl's body.
        let run_with = |swap_at: Option<u64>| {
            let mut s = thrown(&CORE);
            while tick_of(&s) < BLAST {
                let swap = swap_at == Some(tick_of(&s) + 1);
                s.step_slots([
                    RawInput {
                        swap,
                        ..RawInput::NONE
                    },
                    RawInput::default(),
                ]);
            }
            s
        };
        let plain = run_with(None);
        let swapped = run_with(Some(RELEASE + 30));
        assert_eq!(swapped.party.controlled_hero(), HeroId::Donut);
        assert_eq!(blasts(&swapped), [BLAST]);
        assert_eq!(explosion_lines(&swapped), explosion_lines(&plain));
        assert!(explosion_lines(&swapped)[0].contains("credit_player=0"));
    }

    #[test]
    fn carl_prog_dyn_swap_mid_throw() {
        // A swap on t20: the throw finishes under AI control, the stick spawns on
        // t41 and the credit goes to the solo player's slot.
        let mut s = thrown(&CORE);
        run_to(&mut s, P + 19);
        s.step_slots([
            RawInput {
                swap: true,
                ..RawInput::NONE
            },
            RawInput::default(),
        ]);
        assert_eq!(tick_of(&s), P + 20);
        assert_eq!(s.party.controlled_hero(), HeroId::Donut);
        run_to(&mut s, BLAST);
        assert_eq!(
            lines_of(&s, "charge_thrown").len(),
            1,
            "{:?}",
            lines_of(&s, "charge_thrown")
        );
        assert!(lines_of(&s, "charge_thrown")[0].contains(&format!("tick={RELEASE} ")));
        assert_eq!(s.combat.charges[0].owner, Some(0));
        assert!(explosion_lines(&s)[0].contains("credit_player=0"));
    }

    #[test]
    fn carl_prog_dyn_swap_press_same_tick() {
        // The live case: L1 swaps on the press edge and the slot is routed to the
        // newly controlled hero, so a Circle on that tick goes to Donut (inert).
        let mut s = sim(&EMPTY_SOLO, true);
        idle(&mut s, 3);
        s.step_slots([
            RawInput {
                swap: true,
                ability: true,
                ..RawInput::NONE
            },
            RawInput::default(),
        ]);
        assert_eq!(s.party.controlled_hero(), HeroId::Donut);
        idle(&mut s, 60);
        assert!(s.combat.events.is_empty(), "{:?}", s.combat.events);
        assert!(s.combat.thrower().is_idle());
        // And back: the swap to Carl with Circle on the same tick starts his throw.
        idle(&mut s, 60);
        s.step_slots([
            RawInput {
                swap: true,
                ability: true,
                ..RawInput::NONE
            },
            RawInput::default(),
        ]);
        assert_eq!(s.party.controlled_hero(), HeroId::Carl);
        assert_eq!(lines_of(&s, "special_started").len(), 1);
    }

    // ------------------------------------------------------------ restarts --

    static DEFEAT_BASE: Fixture = base("test_dyn_defeat", "retry", 7, &[], 900);
    static DEFEAT: PartyFixture = party(&DEFEAT_BASE, false);

    #[test]
    fn carl_prog_dyn_retry_clears() {
        let mut s = thrown(&DEFEAT);
        run_to(&mut s, RELEASE + 5);
        assert!(s.combat.thrower().stick().is_some());
        assert!(hit_carl(&mut s, 4), "the blow that defeats Carl");
        assert!(s.combat.hero_defeated);
        let restarts = s.combat.restarts;
        while s.combat.restarts == restarts {
            idle(&mut s, 1);
        }
        // The stick is gone without a blast, the special is ready, the
        // capability stays.
        assert!(s.combat.charges.is_empty());
        assert!(s.combat.thrower().is_idle());
        assert!(s.combat.story_holds(SPECIAL_THROWN_CHARGE));
        idle(&mut s, 400);
        assert!(blasts(&s).is_empty(), "no blast from the cleared stick");
        press(&mut s);
        let started: Vec<String> = lines_of(&s, "special_started");
        assert_eq!(started.len(), 2, "a new throw is accepted at once");
    }

    #[test]
    fn a_checkpoint_respawn_clears_the_stick_without_a_blast() {
        use crate::demo::{DEMO_RULES, fixture_for_beat, load_hall};
        let kit = load_hall(&PathBuf::from(env!("CARGO_MANIFEST_DIR"))).unwrap();
        let fixture = fixture_for_beat(1);
        let mut scene = crate::corridor::SceneSim::new_at(&kit, 360, fixture.carl_x).unwrap();
        let mut combat = CombatState::new(fixture, &kit, &scene)
            .unwrap()
            .with_demo(&DEMO_RULES)
            .with_story(StorySetup::NONE.with_hold("special_thrown_charge"));
        let own = combat.fixture.charges.len();
        combat.step(&mut scene, RawInput::default());
        combat.step(&mut scene, PRESS);
        for _ in 0..60 {
            combat.step(&mut scene, RawInput::default());
        }
        assert!(combat.thrower().stick().is_some());
        assert_eq!(combat.charges.len(), own + 1);
        combat.respawn(&mut scene);
        assert!(combat.thrower().is_idle());
        assert_eq!(
            combat.charges.len(),
            own,
            "the slot is gone, the authored charges stay"
        );
        assert!(combat.story_holds(SPECIAL_THROWN_CHARGE));
    }

    // ------------------------------------------------- llamas and the boss --

    static LLAMA_CORE_BASE: Fixture = Fixture {
        llamas: &[LlamaSpawn {
            x: 511,
            ai: EnemyAi::Passive,
        }],
        ..base("test_dyn_llama_core", "a llama on the landing", 7, &[], 600)
    };
    static LLAMA_CORE: PartyFixture = party(&LLAMA_CORE_BASE, false);
    static LLAMA_OUTER_BASE: Fixture = Fixture {
        llamas: &[LlamaSpawn {
            x: 580,
            ai: EnemyAi::Passive,
        }],
        ..base("test_dyn_llama_outer", "a llama in the ring", 7, &[], 600)
    };
    static LLAMA_OUTER: PartyFixture = party(&LLAMA_OUTER_BASE, false);

    #[test]
    fn carl_prog_dyn_llama_reactions() {
        // CP-Q26: the core kills as for goblins.
        let mut core = thrown(&LLAMA_CORE);
        run_to(&mut core, BLAST);
        assert!(!core.combat.llamas[0].alive());
        assert_eq!(core.combat.llamas[0].mode, LlamaMode::Down);
        let lines = explosion_lines(&core);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("target=0 ") && lines[0].contains("cause=explosion"));
        // The outer ring deals 1 damage with the kick reaction.
        let mut ring = thrown(&LLAMA_OUTER);
        run_to(&mut ring, BLAST);
        let llama = &ring.combat.llamas[0];
        assert_eq!(llama.health, crate::feel::LLAMA_HEALTH - 1);
        assert_eq!(llama.mode, LlamaMode::Hurt);
        assert_eq!(llama.stagger_left, crate::feel::LLAMA_HURT_TICKS);
        assert_eq!(llama.vx, crate::feel::LLAMA_KICK_PUSH_VX, "pushed away");
        assert!(explosion_lines(&ring).is_empty());
        assert_eq!(llama.hit_by, Some(0));
    }

    static BOSS_BASE: Fixture = crate::boss_scene::boss_base(
        "test_dyn_boss",
        "the arena",
        7,
        380,
        crate::boss_scene::hp(10),
        900,
    );
    static BOSS: PartyFixture = party(&BOSS_BASE, false);

    fn boss_sim(hp: u16) -> PartyFixtureSim {
        // One fixture per hp is needed (statics): the 10 HP boss below, and a
        // 4 HP one.
        let fixture: &'static PartyFixture = if hp == 4 { &BOSS_LOW } else { &BOSS };
        let kit =
            crate::boss_scene::load_arena(&PathBuf::from(env!("CARGO_MANIFEST_DIR"))).unwrap();
        let scene = crate::corridor::SceneSim::new_at(&kit, 360, 380).unwrap();
        let combat = CombatState::new(fixture.base, &kit, &scene)
            .unwrap()
            .with_donut(fixture.donut)
            .with_story(StorySetup::NONE.with_hold("special_thrown_charge"));
        let mut s = PartyFixtureSim::from_parts(scene, combat, fixture);
        idle(&mut s, 3);
        press(&mut s);
        s
    }

    static BOSS_LOW_BASE: Fixture = crate::boss_scene::boss_base(
        "test_dyn_boss_low",
        "the arena, a boss at 4 HP",
        7,
        380,
        crate::boss_scene::hp(4),
        900,
    );
    static BOSS_LOW: PartyFixture = party(&BOSS_LOW_BASE, false);

    #[test]
    fn carl_prog_dyn_boss() {
        // CP-Q25: 4 damage, counted toward poise as damage, super armour holds
        // (the pound in progress is not interrupted), no launch.
        let mut s = boss_sim(10);
        run_to(&mut s, BLAST - 1);
        let before = s.combat.boss.as_ref().unwrap().x;
        let mode = s.combat.boss.as_ref().unwrap().state().mode;
        idle(&mut s, 1);
        let hurt = lines_of(&s, "boss_hurt");
        assert_eq!(
            hurt,
            [format!(
                "COMBAT_EVENT tick={BLAST} kind=boss_hurt source=0 damage=4 hp_left=6"
            )]
        );
        let boss = s.combat.boss.as_ref().unwrap();
        assert_eq!(boss.hp, 6);
        assert_eq!(boss.x, before, "no launch");
        assert_eq!(boss.state().mode, mode, "the strike in progress holds");
        assert_ne!(boss.state().mode, crate::boss::BossMode::Flinch);
        // A boss at 4 HP falls to one stick, credited to Carl's player.
        let mut low = boss_sim(4);
        run_to(&mut low, BLAST);
        let lines = explosion_lines(&low);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains(&format!("tick={BLAST} ")));
        assert!(lines[0].contains("cause=explosion charge=0") && !lines[0].contains("point_blank"));
        assert!(lines[0].contains("credit_player=0"), "{}", lines[0]);
        assert_eq!(low.combat.boss.as_ref().unwrap().hp, 0);
    }

    #[test]
    fn carl_prog_dyn_clock_gates_melee_and_jump() {
        // Nothing cancels the throw before t50: a melee press and a jump press
        // are lost; from t50 a melee move ends the recovery. Walking continues.
        let attack = RawInput {
            attack: true,
            ..RawInput::NONE
        };
        let jump = RawInput {
            jump: true,
            ..RawInput::NONE
        };
        let mut s = thrown(&EMPTY_SOLO);
        run_to(&mut s, P + 9);
        s.step_slots([attack, RawInput::default()]);
        idle(&mut s, 3);
        s.step_slots([jump, RawInput::default()]);
        idle(&mut s, 3);
        assert!(s.scene.world.hero.grounded, "the jump press was ignored");
        assert!(
            lines_of(&s, "move_started").is_empty(),
            "the melee press was lost"
        );
        assert!(s.combat.thrower().throw_clock(tick_of(&s)).is_some());
        // Walking continues during the throw.
        let x = s.scene.world.hero.x;
        let walk = RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        };
        s.step_slots([walk, RawInput::default()]);
        assert!(s.scene.world.hero.x > x);
        // t50: a melee move starts and ends the recovery.
        run_to(&mut s, P + 49);
        s.step_slots([attack, RawInput::default()]);
        assert_eq!(tick_of(&s), P + 50);
        assert_eq!(lines_of(&s, "move_started").len(), 1);
        assert!(s.combat.thrower().throw_clock(tick_of(&s)).is_none());
        // The stick left on t41 and is untouched by the cancel.
        assert!(s.combat.thrower().stick().is_some());
    }

    #[test]
    fn carl_prog_dyn_jump_at_t50_cancels_the_recovery() {
        let jump = RawInput {
            jump: true,
            ..RawInput::NONE
        };
        let mut s = thrown(&EMPTY_SOLO);
        run_to(&mut s, P + 49);
        s.step_slots([jump, RawInput::default()]);
        assert!(s.combat.thrower().throw_clock(tick_of(&s)).is_none());
        idle(&mut s, 2);
        assert!(!s.scene.world.hero.grounded, "the jump went off");
    }

    /// A stand-in hero-hurt source id for a world blast (the Rot sticker's base).
    const WORLD_SOURCE: u32 = 0x5254;

    #[test]
    fn carl_prog_dyn_hero_damage_policies_share_one_vocabulary() {
        use crate::hero_damage::HeroDamagePolicy;
        // Donut next to a world blast: AllHeroes hurts her in full, OwnerReduced
        // (the thrown profile, owner Carl) and NoHeroes spare her.
        let outcome = |policy: HeroDamagePolicy, owner: Option<u32>| {
            let mut s = sim(&EMPTY_DUO, true);
            idle(&mut s, 2);
            let x = s.scene.world.donut_body.x;
            let mut profile = blast_profile(CarlProgTuning::active());
            profile.hero_damage = policy;
            profile.hero_hp = 2;
            // A world blast routed through the profile logs the id its profile
            // carries (the Rot sticker's, say), never the thrown charge's.
            profile.hero_source = WORLD_SOURCE;
            let charge =
                BlastCharge::new(x, s.scene.world.hero_foot_y(), 1, owner).with_profile(profile);
            s.combat.resolve_profile_blast(&mut s.scene, 0, charge, 3);
            hero_hurts(&s)
        };
        let donut = |hurts: &[(u64, u32, u8, HeroId)]| hurts.iter().any(|h| h.3 == HeroId::Donut);
        assert!(
            donut(&outcome(HeroDamagePolicy::AllHeroes, None)),
            "a world blast hurts every hero"
        );
        assert!(
            donut(&outcome(HeroDamagePolicy::AllHeroes, Some(0))),
            "whoever triggered it"
        );
        assert!(!donut(&outcome(HeroDamagePolicy::OwnerReduced, Some(0))));
        assert!(!donut(&outcome(HeroDamagePolicy::NoHeroes, None)));
        // The owner (hero 1) is the one reduced under OwnerReduced.
        let own = outcome(HeroDamagePolicy::OwnerReduced, Some(1));
        assert!(donut(&own));
        assert!(own.iter().all(|h| h.1 == WORLD_SOURCE));
        assert!(
            outcome(HeroDamagePolicy::AllHeroes, None)
                .iter()
                .all(|h| h.1 == WORLD_SOURCE && h.1 != THROWN_CHARGE_SOURCE)
        );
    }

    static ARMED_BASE: Fixture = base("test_dyn_armed", "an armed Donut", 7, &[], 2000);
    static ARMED_DUO: PartyFixture = PartyFixture {
        donut: DonutSetup::unlocked(8),
        ..party(&ARMED_BASE, true)
    };

    /// DM-B review item F2: a blast is never evadable. The allow-list is the
    /// `EvadeSource` enum (a goblin swing, the Engineer's ram, a llama glob) and
    /// the blast path calls `hurt_hero` directly, never `evade_check`, so an
    /// armed Donut caught in a non-QA blast that hurts her is hurt, logs no
    /// `HeroEvaded` and is rattled (her wait restarts).
    #[test]
    fn carl_prog_dyn_an_armed_donut_never_evades_a_blast() {
        use crate::agility::EvadeSource;
        use crate::hero_damage::HeroDamagePolicy;
        // The allow-list has no blast or charge variant.
        for source in EvadeSource::ALL {
            let name = source.name();
            assert!(
                !name.contains("blast") && !name.contains("charge"),
                "{name}"
            );
        }
        assert_eq!(
            EvadeSource::ALL.len(),
            3,
            "a new variant needs its own design"
        );
        let caught = |policy: HeroDamagePolicy| {
            let kit = kit();
            let scene = crate::corridor::SceneSim::new_at(&kit, 360, 400).unwrap();
            let combat = CombatState::new(ARMED_DUO.base, &kit, &scene)
                .unwrap()
                .with_donut(ARMED_DUO.donut)
                .with_story(StorySetup::NONE.with_hold("special_thrown_charge"));
            let mut s = PartyFixtureSim::from_parts(scene, combat, &ARMED_DUO);
            idle(&mut s, 800);
            assert!(s.combat.reflex().armed(), "Donut's reflex is armed");
            let x = s.scene.world.donut_body.x;
            let mut profile = blast_profile(CarlProgTuning::active());
            profile.hero_damage = policy;
            profile.hero_source = WORLD_SOURCE;
            let charge =
                BlastCharge::new(x, s.scene.world.hero_foot_y(), 1, None).with_profile(profile);
            let tick = tick_of(&s) + 1;
            s.combat
                .resolve_profile_blast(&mut s.scene, 0, charge, tick);
            s
        };
        // A world blast that hurts every hero (AllHeroes, no owner): she is hurt.
        let hurt = caught(HeroDamagePolicy::AllHeroes);
        assert!(
            hero_hurts(&hurt)
                .iter()
                .any(|h| h.3 == HeroId::Donut && h.1 == WORLD_SOURCE),
            "an armed Donut is hurt by the blast, logged with the profile's source"
        );
        assert!(
            !hurt
                .combat
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::HeroEvaded { .. })),
            "never evaded"
        );
        assert!(!hurt.combat.reflex().armed(), "the hurt rattles her");
        // The thrown profile (OwnerReduced, owner Carl) spares her: no hurt, and
        // still no evade event (nothing reached the reflex).
        let spared = caught(HeroDamagePolicy::OwnerReduced);
        assert!(hero_hurts(&spared).iter().all(|h| h.3 != HeroId::Donut));
        assert!(
            !spared
                .combat
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::HeroEvaded { .. }))
        );
        assert!(spared.combat.reflex().armed(), "untouched, still armed");
    }

    /// A state of `CORE`'s shape with one extra authored (sticker-like) charge
    /// far from everything, appended the way HAZ-A2 will.
    fn with_sticker(held: bool) -> (PartyFixtureSim, BlastCharge) {
        let mut s = sim(&EMPTY_SOLO, held);
        let sticker = BlastCharge::new(1500.0, s.scene.world.hero_foot_y(), 30_000, None);
        s.combat.push_authored_charge(sticker);
        (s, sticker)
    }

    /// CP-Q31 / plan CP-B1 addition: the slot is after every authored charge, so
    /// a throw never overwrites a sticker, its blast is never taken for the
    /// stick's, and a respawn never deletes it.
    #[test]
    fn carl_prog_dyn_the_slot_is_after_every_authored_charge() {
        let (mut s, sticker) = with_sticker(true);
        assert_eq!(s.combat.dynamite_slot(), 1, "after the sticker");
        idle(&mut s, 3);
        press(&mut s);
        run_to(&mut s, BLAST);
        // The throw used index 1 and left the sticker alone.
        assert_eq!(s.combat.charges.len(), 2);
        assert_eq!(s.combat.charges[0], sticker);
        let thrown = lines_of(&s, "charge_thrown");
        assert!(thrown[0].contains("charge=1"), "{thrown:?}");
        // The stick's blast is charge 1 and starts the recharge; the sticker did
        // not blow and was not mistaken for the stick.
        assert_eq!(
            lines_of(&s, "blast"),
            [format!("COMBAT_EVENT tick={BLAST} kind=blast charge=1")]
        );
        assert!(s.combat.thrower().recharging());
        assert!(!s.combat.charges[0].detonated);
        // A second throw reuses index 1 and the sticker is still there.
        run_to(&mut s, BLAST + 419);
        press(&mut s);
        idle(&mut s, 120);
        assert_eq!(s.combat.charges.len(), 2);
        assert_eq!(s.combat.charges[0], sticker);
        assert!(
            lines_of(&s, "charge_thrown")
                .iter()
                .all(|l| l.contains("charge=1"))
        );
        // A restart rebuilds the attempt from the fixture: the thrower is clear
        // and the slot is gone (the sticker is the next attempt's to re-add).
        s.combat.restart(&mut s.scene);
        assert!(s.combat.thrower().is_idle());
        assert!(s.combat.charges.is_empty());
        assert_eq!(s.combat.dynamite_slot(), 0);
    }

    #[test]
    fn a_checkpoint_respawn_keeps_the_authored_charges_and_clears_only_the_stick() {
        use crate::demo::{DEMO_RULES, fixture_for_beat, load_hall};
        let kit = load_hall(&PathBuf::from(env!("CARGO_MANIFEST_DIR"))).unwrap();
        let fixture = fixture_for_beat(1);
        let mut scene = crate::corridor::SceneSim::new_at(&kit, 360, fixture.carl_x).unwrap();
        let mut combat = CombatState::new(fixture, &kit, &scene)
            .unwrap()
            .with_demo(&DEMO_RULES)
            .with_story(StorySetup::NONE.with_hold("special_thrown_charge"));
        let own = combat.fixture.charges.len();
        let sticker = BlastCharge::new(1500.0, scene.world.hero_foot_y(), 30_000, None);
        combat.push_authored_charge(sticker);
        assert_eq!(combat.dynamite_slot(), own + 1);
        combat.step(&mut scene, RawInput::default());
        combat.step(&mut scene, PRESS);
        for _ in 0..60 {
            combat.step(&mut scene, RawInput::default());
        }
        assert!(combat.thrower().stick().is_some());
        assert_eq!(combat.charges.len(), own + 2);
        combat.respawn(&mut scene);
        assert!(combat.thrower().is_idle());
        assert_eq!(
            combat.charges.len(),
            own + 1,
            "the sticker survives the respawn"
        );
        assert_eq!(combat.charges[own], sticker);
    }
}
