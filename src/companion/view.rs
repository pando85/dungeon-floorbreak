//! The perception contract (`CompanionView`, reconciliation section c).
//!
//! Plain owned data: `Clone`, no references into `CombatState` or `SceneSim`,
//! no Bevy. 01A-wire builds one per tick from the pre-tick state, only while a
//! hero is companion-driven; it is derived from hashed state, never written
//! back and never hashed itself. `threats` and `targets` are ordered by
//! [`StableEntityId`] (goblin, llama, boss, then index, then spawn serial).

use crate::boss::BossThreat;
use crate::combat::MoveId;
use crate::companion::profile::ProfileSetId;
use crate::party::HeroId;

/// What kind of enemy an id names. The derived order is the view order:
/// goblins, then llamas, then the boss.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntityKind {
    Goblin,
    Llama,
    Boss,
}

/// A stable enemy id (CA-28): the slot index alone aliases after a respawn, so
/// the per-attempt `spawn_serial` is part of the identity. 01A fixtures may use
/// serial 0; 01D requires it. The derived `Ord` (kind, index, serial) is the
/// tie-break order of every selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableEntityId {
    pub kind: EntityKind,
    pub index: u32,
    pub spawn_serial: u32,
}

impl StableEntityId {
    pub const fn goblin(index: u32) -> Self {
        Self {
            kind: EntityKind::Goblin,
            index,
            spawn_serial: 0,
        }
    }

    pub fn hash_into(&self, mix: &mut impl FnMut(u64)) {
        mix(self.kind as u64);
        mix(u64::from(self.index));
        mix(u64::from(self.spawn_serial));
    }
}

/// Who drives a hero this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// A human on this slot (0 = P1, 1 = P2).
    Human(u8),
    /// Driven by the companion director.
    Companion,
    /// Driven by today's follow-and-jump bot (`FollowAndJumpBot`).
    Bot,
}

/// Where a hero's own move is in its timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovePhase {
    Startup,
    Active,
    Recovery,
}

/// A hero's running move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveMove {
    pub id: MoveId,
    pub phase: MovePhase,
    /// `spec.total() - running.tick`, read pre-tick: the move clock has not yet
    /// advanced for the tick the view is built for (it advances before a press
    /// is read, see `director`).
    pub ticks_left: u16,
    /// The move was started by an injected press (the flag survives a swap,
    /// CF02-Q16, and drives the kill policy's `glance`).
    pub ai_started: bool,
}

/// Donut's pool as the AI may use it. `ai_can_cast` is
/// `DonutMana::ai_can_cast` (the reserve is not duplicated here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManaView {
    pub mana: u8,
    pub max: u8,
    pub unlocked: bool,
    pub ai_can_cast: bool,
    /// Bolt speed in px per tick; the builder fills it from
    /// `ManaTuning::projectile_speed_px_per_tick` so the value is not
    /// duplicated (Q17 arrival-time gate).
    pub projectile_speed_px_per_tick: u8,
    /// How far ahead of Donut's pivot the bolt's hit-box front sits on its
    /// spawn tick (spawn offset plus the box's lead and half width). 01A-wire
    /// fills it from the real constants in `combat_scene/donut.rs`
    /// (`BOLT_SPAWN_FORWARD + BOLT_BOX_AHEAD + BOLT_BOX_W / 2`, 35).
    pub bolt_lead_px: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeroView {
    pub id: HeroId,
    pub x: f32,
    pub foot_y: f32,
    pub vx: f32,
    pub vy: f32,
    /// +1 / -1.
    pub facing: i8,
    pub grounded: bool,
    pub health: u8,
    pub hurt_ticks: u8,
    pub invulnerable_ticks: u8,
    pub defeated: bool,
    pub control: Control,
    pub active_move: Option<ActiveMove>,
    /// The hero could accept a move press this tick (not hurt, defeated,
    /// frozen or in a move).
    pub can_start_move: bool,
    /// Donut only.
    pub mana: Option<ManaView>,
}

/// What the follow layer (today's `follower_input_boss`) would do this tick.
/// Filled by the builder with `follower_input_boss` on the same arguments
/// `step_live` uses (pre-tick), `FollowInput::default()` when `!ctx.can_move[i]`;
/// the director starts from it so movement is today's bot.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FollowInput {
    pub move_x: f32,
    pub jump: bool,
}

/// An enemy attack in its tell, as the AI sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThreatView {
    pub id: StableEntityId,
    pub x: f32,
    pub foot_y: f32,
    pub facing: i8,
    /// Ticks since the tell started.
    pub tell_tick: u16,
    /// First non-tell attack tick (goblin `TELL_TICKS` 24, llama
    /// `LLAMA_TELL_TICKS` 36); an interrupt must hit while `attack tick <
    /// tell_len`; `tell_tick` is the pre-tick attack tick.
    pub tell_len: u16,
    /// Attack tick at which the swing becomes active (goblin 26).
    pub impact_tick: u16,
    pub aimed_at: Option<HeroId>,
    /// The Q10 light guard: a further swat would glance.
    pub guarded: bool,
    pub in_view: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetState {
    Standing,
    Staggered {
        ticks: u8,
    },
    Lying {
        down_ticks_left: u16,
        hold_used: bool,
    },
    Airborne,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetView {
    pub id: StableEntityId,
    pub x: f32,
    pub foot_y: f32,
    pub hp: u8,
    pub state: TargetState,
    pub guarded: bool,
    pub in_view: bool,
    /// Carl's move was the last to hit it (`hit_by`, CF02-DUO-A08: the
    /// `HumanFinishWindow` reads it with the target's state).
    pub carl_hit: bool,
    /// Ticks since Carl's last hit on it (`None`: no hit of his this life),
    /// CF02-DUO-A12: the recently-hit window of the `HumanFinishWindow`.
    pub carl_hit_age: Option<u16>,
}

/// The simulation scroll-camera view (CF02-Q40 = AP-Q07): `in_view(x)` is
/// `!scene.is_offscreen(x, 0.0)`, never the presentation camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewBounds {
    pub left: f32,
    pub right: f32,
}

impl ViewBounds {
    pub fn in_view(&self, x: f32) -> bool {
        !(x < self.left || x > self.right)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompanionView {
    /// The tick about to run.
    pub tick: u64,
    pub solo: bool,
    pub cf02_rules: bool,
    pub profile_set: ProfileSetId,
    pub view: ViewBounds,
    /// Carl 0, Donut 1.
    pub heroes: [HeroView; 2],
    /// The follow layer's input per hero.
    pub follow: [FollowInput; 2],
    /// Goblin, llama, boss order (ordered by id).
    pub threats: Vec<ThreatView>,
    /// Ordered by id.
    pub targets: Vec<TargetView>,
    /// The boss tell, when one is up. Ignored by 01A-core (boss play is 01E).
    pub boss: Option<BossThreat>,
}

/// A violated view guarantee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewError {
    HeroOrder,
    NotFinite(&'static str),
    UnorderedThreats,
    UnorderedTargets,
}

impl CompanionView {
    pub fn hero(&self, id: HeroId) -> &HeroView {
        &self.heroes[id.index()]
    }

    pub fn target(&self, id: StableEntityId) -> Option<&TargetView> {
        self.targets
            .binary_search_by(|t| t.id.cmp(&id))
            .ok()
            .map(|i| &self.targets[i])
    }

    /// The guarantees a builder owes (ordering, finite numbers).
    pub fn validate(&self) -> Result<(), ViewError> {
        for (i, hero) in self.heroes.iter().enumerate() {
            if hero.id.index() != i {
                return Err(ViewError::HeroOrder);
            }
            if !(hero.x.is_finite() && hero.foot_y.is_finite()) {
                return Err(ViewError::NotFinite("hero"));
            }
        }
        if !(self.view.left.is_finite() && self.view.right.is_finite()) {
            return Err(ViewError::NotFinite("view"));
        }
        if self.threats.iter().any(|t| !t.x.is_finite()) {
            return Err(ViewError::NotFinite("threat"));
        }
        if self.targets.iter().any(|t| !t.x.is_finite()) {
            return Err(ViewError::NotFinite("target"));
        }
        if self.threats.windows(2).any(|w| w[0].id >= w[1].id) {
            return Err(ViewError::UnorderedThreats);
        }
        if self.targets.windows(2).any(|w| w[0].id >= w[1].id) {
            return Err(ViewError::UnorderedTargets);
        }
        Ok(())
    }
}

/// Test constructors shared with the director's tests.
#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;

    pub(crate) fn hero(id: HeroId, x: f32, control: Control) -> HeroView {
        HeroView {
            id,
            x,
            foot_y: 0.0,
            vx: 0.0,
            vy: 0.0,
            facing: 1,
            grounded: true,
            health: 3,
            hurt_ticks: 0,
            invulnerable_ticks: 0,
            defeated: false,
            control,
            active_move: None,
            can_start_move: true,
            mana: (id == HeroId::Donut).then_some(ManaView {
                mana: 0,
                max: 8,
                unlocked: false,
                ai_can_cast: false,
                projectile_speed_px_per_tick: 6,
                bolt_lead_px: 35,
            }),
        }
    }

    pub(crate) fn view(carl: HeroView, donut: HeroView) -> CompanionView {
        CompanionView {
            tick: 100,
            solo: true,
            cf02_rules: true,
            profile_set: ProfileSetId::Cf02Assist,
            view: ViewBounds {
                left: 0.0,
                right: 640.0,
            },
            heroes: [carl, donut],
            follow: [FollowInput::default(); 2],
            threats: Vec::new(),
            targets: Vec::new(),
            boss: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{hero, view};
    use super::*;

    #[test]
    fn ids_order_by_kind_then_index_then_serial() {
        let a = StableEntityId::goblin(5);
        let b = StableEntityId {
            kind: EntityKind::Llama,
            index: 0,
            spawn_serial: 0,
        };
        let c = StableEntityId {
            kind: EntityKind::Goblin,
            index: 5,
            spawn_serial: 1,
        };
        assert!(a < c && c < b);
        assert_eq!(a, StableEntityId::goblin(5));
        // A respawn into the same slot is a different id (CA-28).
        assert_ne!(a, c);
    }

    #[test]
    fn the_fixture_projectile_speed_is_the_pinned_mana_tuning_value() {
        // The Q17 arithmetic in the director tests uses this fixture value.
        let tuning = crate::donut_mana::ManaTuning::pinned(true).unwrap();
        let mana = hero(HeroId::Donut, 0.0, Control::Companion).mana.unwrap();
        assert_eq!(
            mana.projectile_speed_px_per_tick,
            tuning.projectile_speed_px_per_tick
        );
    }

    #[test]
    fn view_bounds_match_the_scene_offscreen_rule() {
        let v = ViewBounds {
            left: 100.0,
            right: 740.0,
        };
        assert!(v.in_view(100.0) && v.in_view(740.0) && v.in_view(300.0));
        assert!(!v.in_view(99.9) && !v.in_view(740.1));
    }

    #[test]
    fn validate_checks_order_and_finiteness() {
        let mut v = view(
            hero(HeroId::Carl, 0.0, Control::Human(0)),
            hero(HeroId::Donut, 0.0, Control::Companion),
        );
        assert_eq!(v.validate(), Ok(()));
        let target = |i| TargetView {
            id: StableEntityId::goblin(i),
            x: 10.0,
            foot_y: 0.0,
            hp: 2,
            state: TargetState::Standing,
            guarded: false,
            in_view: true,
            carl_hit: false,
            carl_hit_age: None,
        };
        v.targets = vec![target(2), target(1)];
        assert_eq!(v.validate(), Err(ViewError::UnorderedTargets));
        v.targets = vec![target(1), target(2)];
        assert_eq!(v.validate(), Ok(()));
        assert!(v.target(StableEntityId::goblin(2)).is_some());
        assert!(v.target(StableEntityId::goblin(3)).is_none());
        v.heroes[0].x = f32::NAN;
        assert_eq!(v.validate(), Err(ViewError::NotFinite("hero")));
        v.heroes[0].x = 0.0;
        v.heroes.swap(0, 1);
        assert_eq!(v.validate(), Err(ViewError::HeroOrder));
    }
}
