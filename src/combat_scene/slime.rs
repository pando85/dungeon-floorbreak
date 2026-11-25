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
//! * AD7 (replaced by HAZ-B, HAZ-Q15) Donut's reflex may evade the lunge: the
//!   candidate loop asks [`SlotStepCtx::evade`] for her, with the key
//!   `(SlimeLunge, slot target id, tell start tick)`; Carl is never asked, and
//!   the body contact, the acid patch and the Rot blast never reach the reflex
//!   (DM-Q18).

use crate::agility::{EvadeKey, EvadeSource, Verdict};
use crate::blast::BlastReach;
use crate::boss::{BossThreat, HeroView};
use crate::camera_lab::room::IRect;
use crate::combat::{CombatEvent, DefeatCause, MoveId};
use crate::env_hazard::{HazardClass, NavPhase, NavRect};
use crate::party::HeroId;
use crate::prototype::Rect;
use crate::rot_sticker::StickerState;
use crate::scene_enemy::{
    EnvSource, SLOT_ID_MIN, SlotBlast, SlotBuild, SlotContact, SlotDeathBody, SlotEnemy,
    SlotEnvHit, SlotError, SlotHit, SlotHitResult, SlotOut, SlotStepCtx,
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

    /// HAZ-FU (D6): a launched body meets the slime's hurtbox (the gel mass).
    fn contact_rect(&self) -> Option<Rect> {
        self.tuning
            .on(self.tuning.hazfu01_flier_contact)
            .then(|| self.slime.hurtbox(self.tuning))
            .flatten()
    }

    /// HAZ-FU (D6): never bowled, launched or staggered (gel): it flashes and takes
    /// the flier's contact damage (CF02-Q12, 0 in the shipped data); the scene
    /// stops the flier.
    fn apply_contact(
        &mut self,
        contact: &SlotContact,
        tick: u64,
        out: &mut SlotOut,
    ) -> Option<SlotHitResult> {
        if !self.alive() {
            return None;
        }
        let outcome = self.slime.hit(self.tuning, contact.damage);
        if outcome.defeated {
            out.events.push(CombatEvent::enemy_defeated(
                tick,
                self.id,
                DefeatCause::Melee,
                contact.instigator,
                None,
            ));
        }
        Some(self.defeat_result())
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
        // HAZ-B: the instance key's stamp is the tick the tell started. The
        // attack clock advances before the impact tick inside the step, so on an
        // impact tick it reads one more than it did here (a tell cannot begin on
        // an impact tick).
        let stamp = ctx
            .tick
            .saturating_sub(self.slime.attack_tick().map_or(0, |a| u64::from(a) + 1));
        let id = self.id;
        let evade = |hero: HeroId| {
            hero == HeroId::Donut
                && ctx.evade.is_some_and(|gate| {
                    gate.borrow_mut().check(
                        EvadeKey {
                            source: EvadeSource::SlimeLunge,
                            enemy: id,
                            stamp,
                        },
                        id,
                        ctx.tick,
                    ) != Verdict::Hit
                })
        };
        let step = self.slime.step(t, &sctx, ctx.token_free, evade);
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

/// HAZ-B: the hazard-feed ids of the slime and sticker rows. The hazard core's
/// own ids (a Boiler volume, an acid patch) are small, so these bases keep the
/// three families apart in the one sorted feed.
pub const FEED_ID_SLIME: u16 = 0x0200;
pub const FEED_ID_STICKER: u16 = 0x0300;
/// HAZ-FU: each family owns this many feed ids (`0x100`): an index at or over it
/// would spill into the next family's range and mislabel a row.
pub const FEED_ID_SPAN: u16 = 0x0100;

/// HAZ-FU: the feed id of entry `index` of the family that starts at `base`. An
/// index at or over [`FEED_ID_SPAN`] would land in the next family's range (a
/// sticker row labelled as another prop), so it fails loudly in a debug build.
pub fn feed_id(base: u16, index: u32) -> u16 {
    debug_assert!(
        index < u32::from(FEED_ID_SPAN),
        "feed index {index} overflows the {FEED_ID_SPAN}-id range of family {base:#06x}"
    );
    base.wrapping_add(index as u16)
}

/// A world rectangle (y up) as the feed's layout rectangle (y down), rounded
/// outward to whole pixels.
fn layout_rect(r: Rect, layout_h: i32) -> IRect {
    let (x0, x1) = (r.left.floor() as i32, r.right().ceil() as i32);
    let (y0, y1) = (r.bottom.floor() as i32, (r.bottom + r.height).ceil() as i32);
    IRect {
        x: x0,
        y: layout_h - y1,
        w: x1 - x0,
        h: y1 - y0,
    }
}

impl crate::combat_scene::CombatState {
    /// HAZ-B (`docs/01-design/enemy-gameplay-slime-rot.md` section 13, item 4):
    /// every rectangle that can hurt a hero this tick, for the partner's nav
    /// graph and the blink's landing check, in a fixed order: the hazard core's
    /// and the acid patches' rows ([`crate::combat_scene::EnvRuntime::nav_feed`]),
    /// then each living slime's contact box (**active**, class `Acid`), then each
    /// sticker in prop order: an awake (scuttling) sticker's trigger box, a
    /// warning sticker's blast box with its ticks left (**telegraphed**, class
    /// `Blast`). A latched or spent sticker has no row. **Feed only**: no sticker
    /// has a damage-dealing hazard volume, its damage is its `BlastCharge`.
    /// Then each slot enemy's own rows ([`crate::scene_enemy::SlotEnemy::nav_rects`],
    /// the Dozer's, DOZER-CORE).
    pub fn hazard_feed(&self, layout_h: i32, tick: u64) -> Vec<NavRect> {
        let mut feed = self.env.as_ref().map(|e| e.nav_feed()).unwrap_or_default();
        for e in self.slot.enemies.iter() {
            let Some(slime) = e
                .as_any()
                .and_then(|a| a.downcast_ref::<SlimeEnemy>())
                .filter(|s| s.alive())
            else {
                continue;
            };
            let Some(rect) = slime.slime().contact_box(slime.tuning) else {
                continue;
            };
            feed.push(NavRect {
                hazard: feed_id(FEED_ID_SLIME, slime.id - SLIME_ID_BASE),
                class: HazardClass::Acid,
                rect: layout_rect(rect, layout_h),
                phase: NavPhase::Active,
                ticks_left: None,
                hurts_heroes: true,
                hurts_enemies: false,
            });
        }
        if let Some(env) = self.env.as_ref() {
            let t = crate::rot_sticker::active();
            for (prop, sticker, _) in env.stickers() {
                let (rect, ticks_left) = match sticker.state() {
                    StickerState::Scuttling => (sticker.trigger_bounds(t), None),
                    StickerState::Warning => (
                        sticker.blast_bounds(t),
                        sticker
                            .remaining(tick)
                            .map(|r| u16::try_from(r).unwrap_or(u16::MAX)),
                    ),
                    StickerState::Latched | StickerState::Spent => continue,
                };
                feed.push(NavRect {
                    hazard: feed_id(FEED_ID_STICKER, u32::from(prop)),
                    class: HazardClass::Blast,
                    rect: layout_rect(rect, layout_h),
                    phase: NavPhase::Telegraph,
                    ticks_left,
                    hurts_heroes: true,
                    hurts_enemies: true,
                });
            }
        }
        // DOZER-CORE: the slot enemies' own rows (the Dozer's machine and
        // telegraph rows), after the stickers; none for any other kind.
        for e in self.slot.enemies.iter() {
            e.nav_rects(layout_h, &mut feed);
        }
        feed
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
