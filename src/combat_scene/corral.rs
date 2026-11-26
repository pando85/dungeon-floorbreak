//! CORRAL-B wire: the scene half of the llama rush and of llama party
//! targeting (plan section 14, KC1, KC5, R1, R2).
//!
//! [`crate::llama_rush`] is the Bevy-free machine and [`crate::llama`] holds
//! the per-llama state. This file wires them to the combat scene: the per-llama
//! opt-ins and their load check, the hero facts the brain reads, the rush
//! contact (the one hero the box meets goes through `hurt_hero`) and the
//! rush events. Everything is opt-in: a legacy run never reaches any of it.
//!
//! Adopted defaults (question ids in the PR): WIRE-D1 a staggered Recover
//! counts as the rush having ended (the rush cooldown is set as at the end of a
//! Recover); WIRE-D2 a pierced commit raises the rush cooldown to the CRL-Q13
//! floor like an interrupted tell; WIRE-D3 the wire refuses the two knobs it
//! does not implement (`crlq18_rush_evadable` and `crlq10_self_damage`) so a
//! data edit fails loudly instead of being ignored; WIRE-D4 the rush cooldown
//! counts down on `Neutral` ticks only (review N-c).

use super::{CombatState, SceneSim};
use crate::boss::HeroBlow;
use crate::combat::CombatEvent;
use crate::llama::{
    Corral, CorralEnv, FireView, LLAMA_HALF_HEIGHT, Llama, LlamaOptIn, LlamaStep, RushOptIn,
};
use crate::llama_rush::{
    Activation, HeroFacts, REMNANT_MAX_PX, RUSH_HIT_STOP, RushHero, RushProfile,
    geometry::llama_body, pick_two,
};
use crate::party::{EnemyTargetRule, HeroId};
use crate::prototype::{Block, HERO_HALF_HEIGHT, feet_height_above_solid};

/// What a `corral_*` fixture replayed by name carries (KC15): party targeting
/// and the per-llama opt-ins.
#[derive(Debug, Clone)]
pub struct LlamaOptInSet {
    pub party_target: bool,
    pub opt_ins: Vec<LlamaOptIn>,
    /// Authored solids of the fixture (a wall face), appended once to the
    /// scene's solids on the first step.
    pub solids: Vec<Block>,
}

impl CombatState {
    /// KC1: every llama of this run picks its hero with the party's target
    /// rule (the goblins' hysteresis and the solo AI bias) instead of aiming at
    /// Carl. Room-wide and opt-in: no legacy run calls it, so every legacy hash
    /// is unchanged.
    #[must_use]
    pub fn with_llama_party_target(mut self) -> Self {
        self.llama_party_target = true;
        self.reset_attempt();
        self
    }

    /// R1, R2: opt the fixture's llamas in (spawn surface, facing, activation,
    /// rush profile and leash), validated against the scene's solids. The
    /// opt-ins are stored here and reapplied by every attempt reset.
    pub fn with_llama_opt_ins(
        mut self,
        scene: &SceneSim,
        opt_ins: Vec<LlamaOptIn>,
    ) -> Result<Self, String> {
        self.validate_llama_opt_ins(&scene.world.solids, &opt_ins)?;
        self.llama_opt_ins = opt_ins;
        self.reset_attempt();
        Ok(self)
    }

    /// Append the fixture's authored solids to the scene once (the boss arena's
    /// lazy push precedent). Nothing for a fixture without any.
    pub(super) fn sync_corral_solids(&mut self, scene: &mut SceneSim) {
        if self.corral_solids_synced {
            return;
        }
        self.corral_solids_synced = true;
        if let Some(set) = crate::corral_fixtures::llama_opt_ins(self.fixture.name) {
            scene.world.solids.extend(set.solids);
        }
    }

    /// The attack instances handed out so far (0 before the first rush tell).
    pub fn rush_seq(&self) -> u32 {
        self.rush_seq
    }

    /// Tests only: the opt-in load check against `solids`.
    #[doc(hidden)]
    pub fn validate_llama_opt_ins_for_test(
        &self,
        solids: &[Block],
        opt_ins: &[LlamaOptIn],
    ) -> Result<(), String> {
        self.validate_llama_opt_ins(solids, opt_ins)
    }

    /// Whether party targeting is on (the AI partner and the tests read it).
    pub fn llama_party_target_on(&self) -> bool {
        self.llama_party_target
    }

    /// The load check of the opt-ins (R1, N-h): an index of a fixture llama, a
    /// facing of -1, 0 or +1, an authored surface that is the floor line or the
    /// top of intact solids under the whole body, a spawn column inside its own
    /// leash, and a profile whose knobs the wire implements.
    pub(super) fn validate_llama_opt_ins(
        &self,
        solids: &[Block],
        opt_ins: &[LlamaOptIn],
    ) -> Result<(), String> {
        let name = self.fixture.name;
        for (n, opt) in opt_ins.iter().enumerate() {
            let Some(spawn) = self.fixture.llamas.get(opt.index) else {
                return Err(format!(
                    "{name}: opt-in for llama {} (no such llama)",
                    opt.index
                ));
            };
            if opt_ins[..n].iter().any(|o| o.index == opt.index) {
                return Err(format!("{name}: llama {} is opted in twice", opt.index));
            }
            if !(-1..=1).contains(&opt.facing) {
                return Err(format!(
                    "{name}: llama {} facing must be -1, 0 or 1",
                    opt.index
                ));
            }
            let x = spawn.x as f32;
            let feet = opt.feet.unwrap_or(self.enemy_foot_y);
            if feet != self.enemy_foot_y && !surface_under_body(solids, x, feet, opt.facing) {
                return Err(format!(
                    "{name}: llama {} surface {feet} is neither the floor line nor the top of a solid under its whole body",
                    opt.index
                ));
            }
            if opt.dormant && opt.rush.is_none() {
                return Err(format!(
                    "{name}: llama {} is dormant without a rush profile",
                    opt.index
                ));
            }
            if let Some(rush) = opt.rush {
                if rush.leash.min > rush.leash.max || !rush.leash.contains(x) {
                    return Err(format!(
                        "{name}: llama {} spawns at {x}, outside its leash {}..{}",
                        opt.index, rush.leash.min, rush.leash.max
                    ));
                }
                check_profile(rush.profile).map_err(|e| format!("{name}: {e}"))?;
                // N-e (#201 review): a corral llama ignores solids of 16 px or
                // less (CRL-Q32: prop remnants), so no static solid that low may
                // lie in its leash band, or it would walk through it.
                let (lo, hi) = (rush.leash.min - 26.0, rush.leash.max + 26.0);
                if let Some(low) = solids.iter().find(|b| {
                    b.intact
                        && b.rect.height <= REMNANT_MAX_PX
                        && b.rect.right() > lo
                        && b.rect.left < hi
                        && b.rect.top() > feet
                        && b.rect.bottom < feet + LLAMA_HALF_HEIGHT * 2.0
                }) {
                    return Err(format!(
                        "{name}: llama {} has a static solid of {} px in its leash band (a corral llama ignores solids of {REMNANT_MAX_PX} px or less)",
                        opt.index, low.rect.height
                    ));
                }
            }
        }
        Ok(())
    }

    /// A llama of this run at its spawn, with its opt-in applied.
    pub(super) fn new_llama(&self, spawn: &super::LlamaSpawn, opt: Option<&LlamaOptIn>) -> Llama {
        let feet = opt.and_then(|o| o.feet).unwrap_or(self.enemy_foot_y);
        let mut llama = Llama::new(spawn.x as f32, feet + LLAMA_HALF_HEIGHT, spawn.ai);
        if let Some(opt) = opt {
            if opt.facing != 0 {
                llama.facing = opt.facing;
            }
            if let Some(health) = opt.health {
                llama.health = health;
            }
            // Q40: the data kill switch runs every corral llama without its profile.
            if let Some(RushOptIn { profile, leash }) = opt.rush
                && profile.enabled()
            {
                llama.corral = Some(Corral {
                    profile,
                    leash,
                    activation: if opt.dormant {
                        Activation::Dormant
                    } else {
                        Activation::Awake
                    },
                    rush_cooldown: 0,
                    state: None,
                });
            }
        }
        llama
    }

    /// KC1: the hero the llama at `index` points at under party targeting: the
    /// goblins' rule over both heroes (hysteresis, solo AI bias, ties to Carl).
    pub(super) fn llama_hero(&mut self, scene: &SceneSim, index: usize) -> HeroId {
        let drive = *scene.drive();
        let x = self.llamas[index].x;
        let effective = |hero: HeroId| {
            let bias = if drive.is_guarded(hero) {
                f32::from(drive.tuning.target_ai_bias_px)
            } else {
                0.0
            };
            (x - scene.world.body(hero).x).abs() + bias
        };
        let chosen = pick_two(
            effective(HeroId::Carl),
            effective(HeroId::Donut),
            self.llamas[index].held,
            switch_px(scene),
        );
        self.llamas[index].held = Some(chosen);
        chosen
    }

    /// One tick of a corral llama: the hero facts and the rush world from the
    /// finished hero step, then [`Llama::step_corral`].
    pub(super) fn step_corral_llama(
        &mut self,
        scene: &SceneSim,
        index: usize,
        view: FireView,
        x_max: f32,
    ) -> LlamaStep {
        let drive = *scene.drive();
        let heroes: &[HeroId] = if scene.party_live() {
            &[HeroId::Carl, HeroId::Donut]
        } else {
            &[HeroId::Carl]
        };
        let mut facts = Vec::with_capacity(2);
        let mut rush_heroes = Vec::with_capacity(2);
        for &id in heroes {
            let hero = scene.world.body(id);
            let hurtbox = Self::hurtbox(scene, id);
            let feet = hero.y - HERO_HALF_HEIGHT;
            let support = feet - feet_height_above_solid(hero, &scene.world.solids).unwrap_or(0.0);
            let bias = if drive.is_guarded(id) {
                f32::from(drive.tuning.target_ai_bias_px)
            } else {
                0.0
            };
            facts.push(HeroFacts {
                id,
                x: hero.x,
                hurtbox,
                support,
                grounded: hero.grounded,
                alive: !self.hero_defeated,
                bias_px: bias,
            });
            // D1 M4: a guarded hero outside the view cannot be hurt.
            let unseen = drive.is_guarded(id)
                && drive.tuning.solo_offscreen_immunity
                && scene.is_offscreen(hero.x, f32::from(drive.tuning.offscreen_margin_px));
            rush_heroes.push(RushHero {
                id,
                hurtbox,
                hittable: !self.hero_defeated && hero.invulnerable_ticks == 0 && !unseen,
            });
        }
        // CORRAL-D: the standing partition's wall solid in the scene's solids (the
        // synced env range); a rush that meets it breaks it (`apply_llama_step`).
        let partition = self
            .env
            .as_deref()
            .and_then(super::EnvRuntime::partition_world_index);
        let env = CorralEnv {
            solids: &scene.world.solids,
            partition,
            max_x: x_max,
            view,
            heroes: &facts,
            rush_heroes: &rush_heroes,
            switch_px: switch_px(scene),
            next_seq: self.rush_seq + 1,
        };
        self.llamas[index].step_corral(&env)
    }

    /// The scene half of a rush tick: the events, the contact through
    /// `hurt_hero` and the llama's hit-stop (spec section 5, steps 2 and 3).
    pub(super) fn apply_llama_step(
        &mut self,
        scene: &mut SceneSim,
        index: usize,
        step: &LlamaStep,
        tick: u64,
    ) {
        let llama = index as u32;
        if step.rush_tell {
            self.rush_seq += 1;
            self.log(CombatEvent::LlamaRushTell { tick, llama });
        }
        let Some(out) = step.rush else {
            return;
        };
        if out.commit {
            self.log(CombatEvent::LlamaRushCommit { tick, llama });
        }
        if let Some(blow) = out.blow {
            let landed = self.hurt_hero(
                scene,
                HeroBlow {
                    hero: blow.hero,
                    source: blow.source,
                    damage: blow.damage,
                    from_x: blow.from_x,
                    knock_vx: blow.knock_vx,
                    knock_vy: blow.knock_vy,
                },
                tick,
            );
            if landed {
                let defeated = self.hero_defeated;
                let l = &mut self.llamas[index];
                l.hit_stop = l.hit_stop.max(RUSH_HIT_STOP);
                if defeated {
                    l.hit_stop = l.hit_stop.max(super::HERO_DEFEAT_HIT_STOP);
                }
            }
        }
        if let Some(end) = out.end {
            self.log(CombatEvent::LlamaRushEnd { tick, llama, end });
        }
        // CORRAL-D: the box met the standing partition: the rush stimulus breaks it
        // on this same tick (the env step right after makes its remnant solid and
        // lifts nobody the screen no longer holds). The shove is the attack
        // instance, at the rush's own speed; nobody is credited.
        if out.partition_hit
            && let Some(speed) = self.llamas[index]
                .corral
                .as_ref()
                .map(|c| c.profile.crlq07_speed_px_s)
            && let Some(prop) = self
                .env
                .as_deref()
                .and_then(super::EnvRuntime::partition_prop)
        {
            self.env_stimulate(
                usize::from(prop),
                crate::env_prop::Stimulus::Shove {
                    shove_id: self.rush_seq,
                    impulse_px_s: speed,
                    source: llama,
                    credit: None,
                },
                tick,
            );
        }
    }
}

/// The party's `target_switch_px` when the rule is the hysteresis one, `None`
/// for the plain nearest-hero rule.
fn switch_px(scene: &SceneSim) -> Option<f32> {
    let tuning = scene.drive().tuning;
    (tuning.enemy_target == EnemyTargetRule::HysteresisControlledBias)
        .then(|| f32::from(tuning.target_switch_px))
}

/// WIRE-D3: the knobs the wire does not implement are refused, not ignored.
fn check_profile(profile: &RushProfile) -> Result<(), String> {
    if profile.crlq18_rush_evadable {
        return Err("crlq18_rush_evadable is not wired (the rush is never evadable)".into());
    }
    if profile.crlq10_self_damage != 0 {
        return Err("crlq10_self_damage is not wired (no self-damage on a bonk)".into());
    }
    Ok(())
}

/// R1: the intact solids whose top is exactly `feet` cover the whole body.
fn surface_under_body(solids: &[Block], x: f32, feet: f32, facing: i8) -> bool {
    let body = llama_body(x, feet, if facing == 0 { -1 } else { facing });
    let mut spans: Vec<(f32, f32)> = solids
        .iter()
        .filter(|b| b.intact && b.rect.top() == feet)
        .map(|b| (b.rect.left, b.rect.right()))
        .collect();
    spans.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut covered = body.left;
    for (left, right) in spans {
        if left <= covered {
            covered = covered.max(right);
        }
    }
    covered >= body.right()
}

#[cfg(test)]
mod tests {
    /// Spec section 15: main had no distinct-source-id test, so the rush wire
    /// creates it. Every hero-blow source constant is pairwise distinct, none sits
    /// in the index-based range (goblin and Engineer indices, the glob's goblin
    /// count plus llama index, below `SLOT_SOURCE_MIN`) and the rush id is "LR".
    #[test]
    fn every_hero_blow_source_id_is_distinct_and_outside_the_index_range() {
        let sources: [(&str, u32); 11] = [
            ("juicer", crate::boss::BOSS_SOURCE_ID),
            ("rack", crate::boss::RACK_SOURCE_ID),
            ("shore block", crate::cf02::arena::SHORE_BLOCK_SOURCE),
            ("pile boss", crate::pile_boss::PILE_BOSS_SOURCE_ID),
            ("llama rush", crate::llama_rush::LLAMA_RUSH_SOURCE_ID),
            ("slime", crate::slime::SLIME_SOURCE_ID),
            ("acid", crate::slime::ACID_SOURCE_ID),
            ("rot sticker", crate::rot_sticker::ROT_SOURCE_ID),
            ("thrown charge", crate::thrown_charge::THROWN_CHARGE_SOURCE),
            ("env", crate::combat_scene::env::ENV_SOURCE_ID),
            ("blink", 0xb11),
        ];
        for (i, (a, x)) in sources.iter().enumerate() {
            assert!(
                *x >= crate::scene_enemy::SLOT_SOURCE_MIN,
                "{a} is in the index range"
            );
            for (b, y) in &sources[i + 1..] {
                assert_ne!(x, y, "{a} and {b} share a source id");
            }
        }
        assert_eq!(crate::llama_rush::LLAMA_RUSH_SOURCE_ID, 0x4C52);
    }
}
