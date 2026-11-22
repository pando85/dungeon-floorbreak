//! The perception contract (`CompanionView`, reconciliation section c).
//!
//! Plain owned data: `Clone`, no references into `CombatState` or `SceneSim`,
//! no Bevy. 01A-wire builds one per tick from the pre-tick state, only while a
//! hero is companion-driven; it is derived from hashed state, never written
//! back and never hashed itself. `threats` and `targets` are ordered by
//! [`StableEntityId`] (goblin, llama, boss, then index, then spawn serial).

use crate::boss::{BossFacts, BossMode, BossThreat};
use crate::combat::MoveId;
use crate::companion::profile::ProfileSetId;
use crate::party::HeroId;
use crate::prototype::Rect;

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

/// FA-Q02: what the boss is doing, in the five classes the goal layer reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossModeClass {
    /// A tell is up (pound, second pound, charge): the hit is coming.
    Tell,
    /// A strike, the charge, the double step or the rebound: a hitbox is, or is
    /// about to be, live.
    Committed,
    /// The boss takes free damage: Stun, Pinned, the pound recoveries, Flinch.
    Vulnerable,
    /// The phase roar.
    Roar,
    /// Everything else (dormant, neutral walk, turn, backstep, defeated).
    Neutral,
}

impl BossModeClass {
    /// The class of a boss mode (FA-Q02).
    pub const fn of(mode: BossMode) -> Self {
        match mode {
            BossMode::PoundTell | BossMode::SecondTell | BossMode::ChargeTell => Self::Tell,
            BossMode::PoundStrike
            | BossMode::SecondStrike
            | BossMode::Charge
            | BossMode::DoubleStep
            | BossMode::Rebound => Self::Committed,
            BossMode::Stun
            | BossMode::Pinned
            | BossMode::PoundRecover
            | BossMode::SecondRecover
            | BossMode::Flinch => Self::Vulnerable,
            BossMode::Roar => Self::Roar,
            _ => Self::Neutral,
        }
    }
}

/// FA-Q02: the boss as the goal layer sees it, present whenever a boss is alive
/// in the room (not only during a tell, unlike [`CompanionView::boss`]). Plain
/// owned data derived from the boss' hashed state, never hashed itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BossView {
    pub x: f32,
    pub foot_y: f32,
    pub facing: i8,
    pub hp: u16,
    pub phase: u8,
    /// The body box by mode (`None` while dormant).
    pub hurtbox: Option<Rect>,
    pub mode_class: BossModeClass,
    /// Ticks the vulnerable mode still has (`0` outside `Vulnerable`).
    pub vulnerable_ticks_left: u16,
    /// Who the boss is after.
    pub target: Option<HeroId>,
    /// A charge is up (its tell or the run): the ram is coming along the lane, and
    /// the answer is the dodge, not a walk.
    pub charging: bool,
    /// The mode is a pound recovery (the first or the second pound): the
    /// shockwave ripples of that pound are still on their way.
    pub pound_recovery: bool,
    /// The boss mode is Stun, Pinned, Defeated or Dormant: no standing threat.
    pub inert: bool,
    /// The charge lane (FA-Q03): the ram box (x interval) projected ahead of the
    /// pivot in the direction the boss faces (the run direction while it charges),
    /// the box x 36..82 where the ram starts.
    pub ram_box: (f32, f32),
    /// The x interval of the damaging box a Committed mode has live or about to
    /// be: the pound's hit box ahead of the pivot, or the live ram box while it
    /// runs (FA-Q03: "the retreat path crosses a Committed hitbox"). `None`
    /// outside `Committed` and for the rebound.
    pub committed_box: Option<(f32, f32)>,
    /// How far from the pivot a shockwave ripple reaches (`0.0` below phase 2).
    pub ripple_reach_px: f32,
}

/// The id of the boss in a view (kind `Boss`, index 0, serial 0).
pub const BOSS_ID: StableEntityId = StableEntityId {
    kind: EntityKind::Boss,
    index: 0,
    spawn_serial: 0,
};

impl BossView {
    /// Build the view from the boss' facts (FA-Q02), `foot_y` being the boss'
    /// floor.
    pub fn from_facts(f: &BossFacts, foot_y: f32) -> Self {
        let class = BossModeClass::of(f.mode);
        // The ram starts in the direction the boss faces (`run_dir` while it runs).
        let dir = f32::from(f.run_dir);
        let (a, b) = (f.x + dir * f.ram_x[0], f.x + dir * f.ram_x[1]);
        let facing = f32::from(f.facing);
        let (pa, pb) = (f.x + facing * f.pound_x[0], f.x + facing * f.pound_x[1]);
        let committed_box = match f.mode {
            BossMode::Charge => Some((a.min(b), a.max(b))),
            BossMode::PoundStrike | BossMode::SecondStrike | BossMode::DoubleStep => {
                Some((pa.min(pb), pa.max(pb)))
            }
            _ => None,
        };
        Self {
            x: f.x,
            foot_y,
            facing: f.facing,
            hp: f.hp,
            phase: f.phase,
            hurtbox: f.hurtbox,
            mode_class: class,
            vulnerable_ticks_left: if class == BossModeClass::Vulnerable {
                f.vulnerable_ticks_left.unwrap_or(0)
            } else {
                0
            },
            target: f.target,
            charging: matches!(f.mode, BossMode::ChargeTell | BossMode::Charge),
            pound_recovery: matches!(f.mode, BossMode::PoundRecover | BossMode::SecondRecover),
            inert: matches!(
                f.mode,
                BossMode::Stun | BossMode::Pinned | BossMode::Defeated | BossMode::Dormant
            ),
            ram_box: (a.min(b), a.max(b)),
            committed_box,
            ripple_reach_px: f.ripple_reach_px,
        }
    }

    /// The boss as a target for the director's support plan (FA-Q04): standing,
    /// unguarded, `hp` as the boss' own.
    pub fn as_target(&self, in_view: bool) -> TargetView {
        TargetView {
            id: BOSS_ID,
            x: self.x,
            foot_y: self.foot_y,
            hp: self.hp.min(u16::from(u8::MAX)) as u8,
            state: TargetState::Standing,
            guarded: false,
            in_view,
            carl_hit: false,
            carl_hit_age: None,
        }
    }
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
    /// The boss tell, when one is up. Ignored by 01A-core; boss play is
    /// partially enabled by FA-Q02..Q07 (see [`Self::boss_view`]); the rest of
    /// ADR-0009's 01E stays with 01E.
    pub boss: Option<BossThreat>,
    /// FA-Q02: the boss as the goal layer sees it, present whenever a boss is
    /// alive in the room. Only the goal layer's boss play reads it (knob
    /// `fa_q02_boss_play`).
    pub boss_view: Option<BossView>,
    /// CF02-DUO-A15: the x interval around Donut that is flat standing ground
    /// at her own foot level (no ledge edge, no wall, no hole), already inset by
    /// her body half width; `None` when she is airborne, stands on nothing
    /// flat, or the nav is driving her this tick. Built only where the flank
    /// is enabled; the director never leaves it.
    pub flank_span: Option<(f32, f32)>,
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

    /// [`Self::target`], or the boss as a target when `id` is [`BOSS_ID`] and
    /// the view has a boss (FA-Q04: the boss is never in `targets`).
    pub fn find_target(&self, id: StableEntityId) -> Option<TargetView> {
        if id == BOSS_ID {
            return self.boss_view.map(|b| b.as_target(self.view.in_view(b.x)));
        }
        self.target(id).copied()
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
            boss_view: None,
            flank_span: None,
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

    // ------------------------------------------------------- FA-Q02: BossView ----

    fn facts(mode: BossMode, x: f32, facing: i8) -> BossFacts {
        BossFacts {
            mode,
            phase: 2,
            hp: 30,
            x,
            facing,
            target: Some(HeroId::Carl),
            hurtbox: Some(Rect {
                left: x - 24.0,
                bottom: 100.0,
                width: 68.0,
                height: 100.0,
            }),
            vulnerable_ticks_left: Some(40),
            run_dir: facing,
            ram_x: [36.0, 82.0],
            pound_x: [32.0, 67.0],
            ripple_reach_px: 176.0,
            left_wall_x: 32.0,
            right_wall_x: 608.0,
        }
    }

    #[test]
    fn every_boss_mode_has_the_class_the_plan_names() {
        use BossMode::*;
        for (mode, class) in [
            (PoundTell, BossModeClass::Tell),
            (SecondTell, BossModeClass::Tell),
            (ChargeTell, BossModeClass::Tell),
            (PoundStrike, BossModeClass::Committed),
            (SecondStrike, BossModeClass::Committed),
            (Charge, BossModeClass::Committed),
            (DoubleStep, BossModeClass::Committed),
            (Rebound, BossModeClass::Committed),
            (Stun, BossModeClass::Vulnerable),
            (Pinned, BossModeClass::Vulnerable),
            (PoundRecover, BossModeClass::Vulnerable),
            (SecondRecover, BossModeClass::Vulnerable),
            (Flinch, BossModeClass::Vulnerable),
            (Roar, BossModeClass::Roar),
            (Dormant, BossModeClass::Neutral),
            (Neutral, BossModeClass::Neutral),
            (Turn, BossModeClass::Neutral),
            (Backstep, BossModeClass::Neutral),
            (Defeated, BossModeClass::Neutral),
        ] {
            assert_eq!(BossModeClass::of(mode), class, "{mode:?}");
        }
    }

    #[test]
    fn the_view_carries_the_geometry_the_goal_layer_reads() {
        // A boss at 300 facing left: the ram box and the pound box lie ahead of
        // the pivot, mirrored.
        let v = BossView::from_facts(&facts(BossMode::PoundStrike, 300.0, -1), 100.0);
        assert_eq!(v.ram_box, (218.0, 264.0));
        assert_eq!(v.committed_box, Some((233.0, 268.0)));
        assert_eq!(v.mode_class, BossModeClass::Committed);
        assert!(!v.inert && !v.charging && !v.pound_recovery);
        assert_eq!(v.ripple_reach_px, 176.0);
        // The vulnerable ticks are read only in a vulnerable class.
        assert_eq!(v.vulnerable_ticks_left, 0);
        let v = BossView::from_facts(&facts(BossMode::Stun, 300.0, -1), 100.0);
        assert_eq!(v.vulnerable_ticks_left, 40);
        assert!(v.inert && v.committed_box.is_none());
        let v = BossView::from_facts(&facts(BossMode::PoundRecover, 300.0, 1), 100.0);
        assert!(v.pound_recovery && !v.inert);
        // A running charge: the live ram box in the run direction.
        let v = BossView::from_facts(&facts(BossMode::Charge, 300.0, 1), 100.0);
        assert!(v.charging);
        assert_eq!(v.committed_box, Some((336.0, 382.0)));
        let v = BossView::from_facts(&facts(BossMode::ChargeTell, 300.0, 1), 100.0);
        assert!(v.charging && v.committed_box.is_none());
        // Dormant and defeated are inert; a dormant boss has no body.
        let mut f = facts(BossMode::Dormant, 480.0, -1);
        f.hurtbox = None;
        let v = BossView::from_facts(&f, 100.0);
        assert!(v.inert && v.hurtbox.is_none());
    }

    #[test]
    fn the_boss_is_a_target_only_through_find_target() {
        let mut v = view(
            hero(HeroId::Carl, 0.0, Control::Human(0)),
            hero(HeroId::Donut, 0.0, Control::Companion),
        );
        assert!(v.find_target(BOSS_ID).is_none(), "no boss, no target");
        v.boss_view = Some(BossView::from_facts(
            &facts(BossMode::Stun, 300.0, -1),
            100.0,
        ));
        let t = v.find_target(BOSS_ID).expect("the boss target");
        assert_eq!((t.id, t.x, t.foot_y), (BOSS_ID, 300.0, 100.0));
        assert!(t.in_view && t.state == TargetState::Standing && !t.guarded);
        // It is never in `targets`, so validate and the id order stay as before.
        assert!(v.target(BOSS_ID).is_none());
        assert_eq!(v.validate(), Ok(()));
        // The boss sorts after every goblin and llama.
        assert!(BOSS_ID > StableEntityId::goblin(999));
    }
}
