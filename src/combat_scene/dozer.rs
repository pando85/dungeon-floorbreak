//! DOZER-CORE: the revenge machine of room 3 as a slot enemy
//! (`docs/01-design/enemy-gameplay-dozer.md`, sections 4 and 7).
//!
//! The adapter behind ENEMY-SLOT's hooks: it wraps the pure
//! [`crate::dozer::Dozer`] in the [`SlotEnemy`] contract and adds no hook
//! anywhere else.
//!
//! * The hull is a hurtbox: a hit or a blast **clangs** (a `Clang` sub-event, the
//!   move's own hit-stop through `Some`), never lowers its fixed `hp()`, and the
//!   machine never logs `EnemyDefeated` (it is never killed, DZ-Q04).
//! * Its tick: the heroes' views, the solids, solo and the sim view in; the
//!   wheel's blows, the push-out shifts, the solo rescue (`SlotOut::place`) and
//!   its sub-events (`CombatEvent::Dozer`) out.
//! * Its nav rows ([`SlotEnemy::nav_rects`]) join the hazard feed after the
//!   slime and sticker rows.
//! * A falling mass (the stop frame of DR-B) on its front collider during the
//!   chase stuns it once ([`SlotEnemy::apply_env`]).
//! * It never supplies the `boss` feed and is never a companion target.
//!
//! The registry row is `dozer` ([`KIND`], the roster kind string).

use crate::combat::CombatEvent;
use crate::dozer::{DOZER_HP, DOZER_ID, Dozer, DozerCtx, DozerEvent, DozerOut};
use crate::env_hazard::{HazardClass, NavRect};
use crate::prototype::Rect;
use crate::scene_enemy::{
    SlotBlast, SlotBuild, SlotEnemy, SlotEnvHit, SlotError, SlotHit, SlotHitResult, SlotOut,
    SlotStepCtx,
};

/// The registry name of the kind: the roster kind string.
pub const KIND: &str = "dozer";

/// The machine in the slot.
#[derive(Debug, Clone)]
pub struct DozerEnemy {
    dozer: Dozer,
}

/// The registry builder: the sidecar's `dozer` block and exit.
pub fn build(b: &SlotBuild<'_>) -> Result<Box<dyn SlotEnemy>, SlotError> {
    let enc = b.encounter.ok_or_else(|| SlotError::Build {
        kind: KIND,
        reason: "a dozer builds from a sidecar roster".into(),
    })?;
    let site = enc.dozer.ok_or_else(|| SlotError::Build {
        kind: KIND,
        reason: "a dozer entry needs the sidecar's dozer block".into(),
    })?;
    Ok(Box::new(DozerEnemy {
        dozer: Dozer::new(site, enc.exit.x as f32, b.room.floor_y),
    }))
}

impl DozerEnemy {
    /// The domain machine (fixtures and the presenter read it).
    pub fn dozer(&self) -> &Dozer {
        &self.dozer
    }

    /// Fixture hook: the machine with its domain state replaced.
    #[must_use]
    pub fn with_dozer(mut self, dozer: Dozer) -> Self {
        self.dozer = dozer;
        self
    }

    fn untouched() -> SlotHitResult {
        SlotHitResult {
            hp_left: DOZER_HP,
            defeated: false,
        }
    }

    fn emit(out: &mut SlotOut, tick: u64, events: impl IntoIterator<Item = DozerEvent>) {
        out.events.extend(
            events
                .into_iter()
                .map(|event| CombatEvent::Dozer { tick, event }),
        );
    }
}

impl SlotEnemy for DozerEnemy {
    fn kind(&self) -> &'static str {
        KIND
    }

    fn target_id(&self) -> u32 {
        DOZER_ID
    }

    fn box_clone(&self) -> Box<dyn SlotEnemy> {
        Box::new(self.clone())
    }

    /// Never killed (DZ-Q04): stepped, hit and drawn for the whole attempt.
    fn alive(&self) -> bool {
        true
    }

    fn hp(&self) -> u16 {
        DOZER_HP
    }

    /// The hull (the push box) and the front collider once it has emerged;
    /// nothing while it is parked off-room. Donut's swat and pounce aim only
    /// at the first rect (`slot_donut_targets`), so her hits on the wheel or
    /// the front collider do not clang yet (DZ-IMPL-AD12; cosmetic in room 3).
    fn hurt_rects(&self) -> Vec<Rect> {
        if self.dozer.state.emerged() {
            vec![self.dozer.push_box(), self.dozer.impact_collider()]
        } else {
            Vec::new()
        }
    }

    /// The front impact collider: what a stop frame's landing tests.
    fn victim_rect(&self) -> Option<Rect> {
        self.dozer
            .state
            .emerged()
            .then(|| self.dozer.impact_collider())
    }

    fn step(&mut self, ctx: &SlotStepCtx<'_>, out: &mut SlotOut) {
        let dctx = DozerCtx {
            tick: ctx.tick,
            heroes: ctx.heroes,
            solids: ctx.solids,
            solo: ctx.solo,
            view_x: ctx.view_x,
        };
        let mut dout = DozerOut::default();
        self.dozer.step(&dctx, &mut dout);
        Self::emit(out, ctx.tick, dout.events);
        out.blows.extend(dout.blows);
        out.shifts.extend(dout.shifts);
        out.place.extend(dout.place);
    }

    /// DZ-Q05: a hit on the hull clangs with the move's own hit-stop.
    fn apply_hit(&mut self, hit: &SlotHit, tick: u64, out: &mut SlotOut) -> Option<SlotHitResult> {
        if !self.dozer.state.emerged() {
            return None;
        }
        Self::emit(
            out,
            tick,
            [DozerEvent::Clang {
                player: hit.player,
                blast: false,
            }],
        );
        Some(Self::untouched())
    }

    /// A blast on the hull clangs: no state change.
    fn apply_blast(
        &mut self,
        blast: &SlotBlast,
        tick: u64,
        out: &mut SlotOut,
    ) -> Option<SlotHitResult> {
        if !self.dozer.state.emerged() {
            return None;
        }
        Self::emit(
            out,
            tick,
            [DozerEvent::Clang {
                player: blast.charge.owner.unwrap_or(crate::shore::PLAYER_ONE),
                blast: true,
            }],
        );
        Some(Self::untouched())
    }

    /// DZ-Q15: a falling mass on the front collider during the chase stuns it
    /// once; anything else leaves it untouched.
    fn apply_env(
        &mut self,
        hit: &SlotEnvHit,
        tick: u64,
        out: &mut SlotOut,
    ) -> Option<SlotHitResult> {
        if hit.class != HazardClass::FallingMass {
            return None;
        }
        let mut dout = DozerOut::default();
        if !self.dozer.apply_falling_mass(&mut dout) {
            return None;
        }
        Self::emit(out, tick, dout.events);
        Some(Self::untouched())
    }

    fn nav_rects(&self, layout_h: i32) -> Vec<NavRect> {
        self.dozer.nav_rows(layout_h)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn mix_into(&self, mix: &mut dyn FnMut(u64)) {
        self.dozer.mix_into(&mut |v| mix(v));
    }
}
