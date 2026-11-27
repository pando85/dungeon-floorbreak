//! TROG-A: the troglodytes as ENEMY-SLOT enemies
//! (`docs/01-design/floor-01-troglodyte-integration.md` sections 5 and 12.1). The domain is
//! [`crate::trog`]; this module is the adapter: one [`SlotEnemy`], three registry rows
//! ([`build_pygmy`], [`build_basher`], [`build_virtuoso`], registered in
//! [`crate::scene_enemy::SLOT_KINDS`]) and nothing in `combat_scene.rs` or `contacts_cf02.rs`.
//!
//! Adopted defaults of this step are listed in the plan (`TROG-AD*`); the ones that live here:
//!
//! * a troglodyte's target id is [`TROG_ID_BASE`] plus its **roster index**, and its
//!   `HeroBlow` source is [`crate::trog::trog_source`] of the roster index, so two of one room
//!   never share a source;
//! * it never supplies the `boss` feed: the companion sees it through
//!   [`SlotEnemy::companion_target`] and [`SlotEnemy::companion_threat`] (TROG-AD16);
//! * a hit, blast or contact goes through the domain's reaction table (`HitSource`, never poise,
//!   TROG-AD10); a launched body deals it no damage and flinches the Pygmy only;
//! * Donut's reflex may evade any blow (TROG-AD9): the candidate loop asks
//!   [`SlotStepCtx::evade`] for her, with the key `(TrogBlow, slot target id, tell start tick)`.

use crate::agility::{EvadeKey, EvadeSource, Verdict};
use crate::combat::CombatEvent;
use crate::companion::view::TargetState;
use crate::party::HeroId;
use crate::prototype::Rect;
use crate::scene_enemy::{
    CompanionTarget, CompanionThreat, HitSource, SLOT_ID_MIN, SlotBlast, SlotBuild, SlotContact,
    SlotDeathBody, SlotEnemy, SlotError, SlotHit, SlotHitResult, SlotOut, SlotStepCtx,
};
use crate::trog::{HitKind, Spawn, Species, State, Trog, TrogCtx, TrogStep, TrogTuning};

/// The first troglodyte target id: above every other slot id of a trog room.
pub const TROG_ID_BASE: u32 = SLOT_ID_MIN + 0x300;

/// The slot target id of the troglodyte at roster index `index`.
pub const fn target_id(index: usize) -> u32 {
    TROG_ID_BASE + index as u32
}

/// One troglodyte in the slot.
#[derive(Debug, Clone)]
pub struct TrogEnemy {
    trog: Trog,
    id: u32,
}

fn build(b: &SlotBuild<'_>, species: Species) -> Result<Box<dyn SlotEnemy>, SlotError> {
    let kind = species.kind_name();
    let enc = b.encounter.ok_or_else(|| SlotError::Build {
        kind,
        reason: "a troglodyte builds from a sidecar roster".into(),
    })?;
    let entry = enc
        .roster
        .get(b.entry.index)
        .ok_or_else(|| SlotError::Build {
            kind,
            reason: format!("roster index {} is out of range", b.entry.index),
        })?;
    let index = u8::try_from(b.entry.index).map_err(|_| SlotError::Build {
        kind,
        reason: "more than 255 roster entries".into(),
    })?;
    let trigger = entry.trigger_x.ok_or_else(|| SlotError::Build {
        kind,
        reason: format!("roster {}: a troglodyte needs a trigger_x", entry.id),
    })?;
    let p = enc.protected_spawn;
    let trog = Trog::new(
        TrogTuning::active(),
        &Spawn {
            species,
            index,
            x: b.entry.x,
            floor_y: b.room.floor_y,
            trigger_x: Some(trigger as f32),
            delay_ticks: b.entry.delay_ticks,
            protected: (p.x as f32, p.right() as f32),
        },
    );
    Ok(Box::new(TrogEnemy {
        trog,
        id: target_id(b.entry.index),
    }))
}

/// The registry builder of `trog_pygmy`.
pub fn build_pygmy(b: &SlotBuild<'_>) -> Result<Box<dyn SlotEnemy>, SlotError> {
    build(b, Species::Pygmy)
}

/// The registry builder of `trog_basher`.
pub fn build_basher(b: &SlotBuild<'_>) -> Result<Box<dyn SlotEnemy>, SlotError> {
    build(b, Species::Basher)
}

/// The registry builder of `trog_virtuoso`.
pub fn build_virtuoso(b: &SlotBuild<'_>) -> Result<Box<dyn SlotEnemy>, SlotError> {
    build(b, Species::Virtuoso)
}

impl TrogEnemy {
    /// The domain machine (fixtures and the presenter read it).
    pub fn trog(&self) -> &Trog {
        &self.trog
    }

    /// Fixture hook: the machine with its domain state replaced.
    #[must_use]
    pub fn with_trog(mut self, trog: Trog) -> Self {
        self.trog = trog;
        self
    }

    fn emit(&self, out: &mut SlotOut, tick: u64, acts: &[crate::trog::TrogAct]) {
        out.events.extend(acts.iter().map(|&act| CombatEvent::Trog {
            tick,
            enemy: self.id,
            act,
        }));
    }

    fn react(
        &mut self,
        kind: HitKind,
        damage: u16,
        tick: u64,
        cause: impl FnOnce() -> CombatEvent,
        out: &mut SlotOut,
    ) -> Option<SlotHitResult> {
        let mut step = TrogStep::default();
        let outcome = self
            .trog
            .hit(TrogTuning::active(), tick, kind, damage, &mut step)?;
        self.emit(out, tick, &step.acts);
        if outcome.defeated {
            out.events.push(cause());
        }
        Some(SlotHitResult {
            hp_left: outcome.hp_left,
            defeated: outcome.defeated,
        })
    }
}

impl SlotEnemy for TrogEnemy {
    fn kind(&self) -> &'static str {
        self.trog.species.kind_name()
    }

    fn target_id(&self) -> u32 {
        self.id
    }

    fn box_clone(&self) -> Box<dyn SlotEnemy> {
        Box::new(self.clone())
    }

    fn alive(&self) -> bool {
        self.trog.alive()
    }

    fn hp(&self) -> u16 {
        self.trog.hp
    }

    fn hurt_rects(&self) -> Vec<Rect> {
        self.trog
            .hurtbox(TrogTuning::active())
            .into_iter()
            .collect()
    }

    fn holds_token(&self) -> bool {
        self.trog.holds_token()
    }

    /// The `down` frame holds as the body. The presenter adds the species' death profile
    /// (`trog_pygmy`, `trog_basher`, `trog_virtuoso`, TROG-FX-01: blood on a jab, the kind's own
    /// parts on a point blank blast) when the run has the troglodyte death art, and nothing
    /// otherwise (TROG-AD19).
    fn death_body(&self) -> Option<SlotDeathBody> {
        Some(SlotDeathBody {
            x: self.trog.x,
            ground_y: self.trog.foot_y,
            facing: self.trog.facing,
            species: self.trog.species.kind_name(),
        })
    }

    fn step(&mut self, ctx: &SlotStepCtx<'_>, out: &mut SlotOut) {
        if !self.alive() {
            return;
        }
        let t = TrogTuning::active();
        let tctx = TrogCtx {
            tick: ctx.tick,
            heroes: &ctx.heroes,
            solids: ctx.solids,
            solo: ctx.solo,
            view_x: ctx.view_x,
            token_free: ctx.token_free,
            last_tell_start: ctx.last_tell_start,
        };
        // The instance key's stamp is the tick the tell started (a strike never shares the
        // tick a tell starts on, so the stamp read here is the committed one).
        let id = self.id;
        let stamp = self.trog.tell_start_tick();
        let mut evade = |hero: HeroId| {
            hero == HeroId::Donut
                && ctx.evade.is_some_and(|gate| {
                    gate.borrow_mut().check(
                        EvadeKey {
                            source: EvadeSource::TrogBlow,
                            enemy: id,
                            stamp,
                        },
                        id,
                        ctx.tick,
                    ) != Verdict::Hit
                })
        };
        let mut step = TrogStep::default();
        self.trog.step(t, &tctx, &mut evade, &mut step);
        self.emit(out, ctx.tick, &step.acts);
        out.tell_started |= step.tell_started;
        out.blows.extend(step.blows);
    }

    fn apply_hit(&mut self, hit: &SlotHit, tick: u64, out: &mut SlotOut) -> Option<SlotHitResult> {
        let kind = match hit.source {
            HitSource::CarlMelee(_) => HitKind::Melee,
            HitSource::DonutSwat => HitKind::Swat,
            HitSource::DonutPounce => HitKind::Pounce,
            HitSource::MagicMissile => HitKind::Missile,
        };
        let (id, h) = (self.id, *hit);
        self.react(kind, hit.damage, tick, || h.defeat_event(tick, id), out)
    }

    /// TROG-AD15: dynamite deals the blast's boss damage and interrupts every species.
    fn apply_blast(
        &mut self,
        blast: &SlotBlast,
        tick: u64,
        out: &mut SlotOut,
    ) -> Option<SlotHitResult> {
        let damage = u16::from(blast.charge.profile.boss_damage);
        if damage == 0 {
            return None;
        }
        let (id, b) = (self.id, *blast);
        self.react(
            HitKind::Blast,
            damage,
            tick,
            || CombatEvent::enemy_defeated(tick, id, b.cause(), b.charge.owner, None),
            out,
        )
    }

    /// A launched body deals no damage and flinches the Pygmy only.
    fn apply_contact(
        &mut self,
        contact: &SlotContact,
        tick: u64,
        out: &mut SlotOut,
    ) -> Option<SlotHitResult> {
        if self.trog.species != Species::Pygmy || !self.alive() {
            return None;
        }
        let (id, instigator) = (self.id, contact.instigator);
        self.react(
            HitKind::Contact,
            0,
            tick,
            || {
                CombatEvent::enemy_defeated(
                    tick,
                    id,
                    crate::combat::DefeatCause::Melee,
                    instigator,
                    None,
                )
            },
            out,
        )
    }

    fn companion_target(&self) -> Option<CompanionTarget> {
        let t = &self.trog;
        if !t.alive() {
            return None;
        }
        let state = match t.state {
            State::Hurt => TargetState::Staggered {
                ticks: u8::try_from(t.hurt_left()).unwrap_or(u8::MAX),
            },
            State::Active if t.lift() > 0.0 => TargetState::Airborne,
            _ => TargetState::Standing,
        };
        Some(CompanionTarget {
            x: t.x,
            foot_y: t.foot_y + t.lift(),
            hp: u8::try_from(t.hp).unwrap_or(u8::MAX),
            state,
        })
    }

    /// The committed hitbox of a tell or an active window; the solids it is clipped by are the
    /// scene's, so the companion's row uses the unclipped band (a conservative threat).
    fn companion_threat(&self) -> Option<CompanionThreat> {
        let t = &self.trog;
        let tuning = TrogTuning::active();
        let sp = tuning.species(t.species);
        let (rect, _) = t.threat(tuning, &[])?;
        Some(CompanionThreat {
            x: t.x,
            foot_y: t.foot_y,
            facing: t.facing,
            tell_tick: t.attack_tick(),
            tell_len: sp.tell_ticks,
            impact_tick: sp.strike_start,
            aimed_at: t.target(),
            swing_x: Some((rect.left, rect.right())),
        })
    }

    fn mix_into(&self, mix: &mut dyn FnMut(u64)) {
        mix(0x5452_4f47_0002);
        mix(u64::from(self.id));
        self.trog.mix_into(mix);
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

/// Whether the machine is in the state `state` (fixtures).
impl TrogEnemy {
    pub fn is_in(&self, state: State) -> bool {
        self.trog.state == state
    }
}
