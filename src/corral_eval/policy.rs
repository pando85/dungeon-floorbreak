//! CORRAL-AUTO-01 action policies (versioned, [`super::POLICY_VERSION`]).
//!
//! A policy is a pure function of the **observable** state (the previous
//! tick's finished state: positions, health, mana, the cast in progress, the
//! llamas' modes) plus its own press bookkeeping. It returns one raw input per
//! **hero** with true press edges (a press tick, then a release tick before
//! the next legal cast), and the runner routes them to the party's slots. A
//! policy never reads the future, never switches to a stronger policy to turn a
//! loss into a win, and never calls anything that writes game state.

use crate::input::RawInput;
use crate::llama::LlamaMode;

/// Which policy drives the tested hero(es).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PolicyId {
    /// Neutral inputs: what the llamas do to a hero who plays nothing.
    NoInteraction,
    /// `attack` held for the whole run: one press, not many spells. The "press
    /// continuously" no-op of #181, run and labelled trivial.
    HoldAttackTrivial,
    /// Donut stands still (no move, no jump) and presses a new Magic Missile
    /// whenever a legal cast can begin and mana permits, through the regen.
    StationaryCast,
    /// Same stand, but spends the pool in a closely spaced burst and waits for
    /// a full pool before the next one.
    StationaryCastBurst,
    /// Donut moves to keep a firing band, faces the nearest llama and jumps a
    /// telling llama's glob lane. Reads only the current state.
    MobileCast,
    /// Carl closes with the nearest llama and jabs it (solo as Carl).
    CarlMelee,
    /// P1 Carl melee and P2 Donut stationary cast on the same ticks.
    CoopProbe,
}

impl PolicyId {
    pub const ALL: [PolicyId; 7] = [
        PolicyId::NoInteraction,
        PolicyId::HoldAttackTrivial,
        PolicyId::StationaryCast,
        PolicyId::StationaryCastBurst,
        PolicyId::MobileCast,
        PolicyId::CarlMelee,
        PolicyId::CoopProbe,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            PolicyId::NoInteraction => "no_interaction",
            PolicyId::HoldAttackTrivial => "hold_attack_trivial",
            PolicyId::StationaryCast => "stationary_cast",
            PolicyId::StationaryCastBurst => "stationary_cast_burst",
            PolicyId::MobileCast => "mobile_cast",
            PolicyId::CarlMelee => "carl_melee",
            PolicyId::CoopProbe => "coop_probe",
        }
    }

    /// The two stationary policies the KC19 gate is defined on.
    pub const fn is_stationary_gate(self) -> bool {
        matches!(
            self,
            PolicyId::StationaryCast | PolicyId::StationaryCastBurst
        )
    }
}

/// One hero as the policy sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeroObs {
    pub x: f32,
    pub facing: i8,
    pub health: u8,
    pub grounded: bool,
}

/// One llama as the policy sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LlamaObs {
    pub x: f32,
    pub facing: i8,
    pub health: u8,
    pub mode: LlamaMode,
    pub attack_tick: u16,
}

impl LlamaObs {
    pub fn alive(&self) -> bool {
        self.health > 0 && !matches!(self.mode, LlamaMode::Down | LlamaMode::Corpse)
    }
    pub fn telling(&self) -> bool {
        self.mode == LlamaMode::Attack
    }
}

/// What a policy may read: the finished state of the previous tick.
#[derive(Debug, Clone)]
pub struct Obs {
    /// The tick about to be stepped (1-based, like the scene's `tick`).
    pub tick: u64,
    pub carl: HeroObs,
    pub donut: HeroObs,
    /// `(mana, mana_max)` once the biscuit is unlocked.
    pub mana: Option<(u8, u8)>,
    /// A Magic Missile cast is in progress (its press has been taken).
    pub cast_active: bool,
    /// The hero is in a hurt flinch: it cannot start a cast.
    pub donut_hurt: bool,
    pub llamas: Vec<LlamaObs>,
}

/// One raw input per hero. The runner routes it to the party's slots.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HeroInputs {
    pub carl: RawInput,
    pub donut: RawInput,
}

/// Magic Missile reach (pivot to pivot, px) the mobile policy aims inside:
/// the pinned 240 px range with a margin for the body half-width.
const SHOOT_BAND: (f32, f32) = (130.0, 225.0);
/// Carl's jab reach the melee policy closes to.
const JAB_REACH: f32 = 52.0;

fn toward(from: f32, to: f32) -> f32 {
    if to >= from { 1.0 } else { -1.0 }
}

fn nearest(from: f32, llamas: &[LlamaObs]) -> Option<&LlamaObs> {
    llamas
        .iter()
        .filter(|l| l.alive())
        .min_by(|a, b| (a.x - from).abs().total_cmp(&(b.x - from).abs()))
}

/// A policy instance: [`PolicyId`] plus its press bookkeeping.
#[derive(Debug, Clone)]
pub struct Policy {
    id: PolicyId,
    /// The previous tick carried a press: this tick is the release.
    released_due: bool,
    /// Burst phase: refilling the pool before the next burst.
    refilling: bool,
    /// Presses issued (press edges, never held ticks).
    pub presses: u32,
    /// The jab of the melee policy alternates press and release.
    jab_pressed: bool,
}

impl Policy {
    pub fn new(id: PolicyId) -> Self {
        Self {
            id,
            released_due: false,
            refilling: false,
            presses: 0,
            jab_pressed: false,
        }
    }

    pub fn id(&self) -> PolicyId {
        self.id
    }

    pub fn act(&mut self, obs: &Obs) -> HeroInputs {
        match self.id {
            PolicyId::NoInteraction => HeroInputs::default(),
            PolicyId::HoldAttackTrivial => {
                // Counted as one press: the edge is the first tick only.
                if self.presses == 0 {
                    self.presses = 1;
                }
                HeroInputs {
                    donut: RawInput {
                        attack: true,
                        ..RawInput::NONE
                    },
                    ..HeroInputs::default()
                }
            }
            PolicyId::StationaryCast => HeroInputs {
                donut: self.stationary(obs, false),
                ..HeroInputs::default()
            },
            PolicyId::StationaryCastBurst => HeroInputs {
                donut: self.stationary(obs, true),
                ..HeroInputs::default()
            },
            PolicyId::MobileCast => HeroInputs {
                donut: self.mobile(obs),
                ..HeroInputs::default()
            },
            PolicyId::CarlMelee => HeroInputs {
                carl: self.melee(obs),
                ..HeroInputs::default()
            },
            PolicyId::CoopProbe => HeroInputs {
                carl: self.melee(obs),
                donut: self.stationary(obs, false),
            },
        }
    }

    /// A press edge on the next legal cast: press for one tick, release the
    /// next, then wait for the cast to end (16-tick refire) before pressing
    /// again. Never moves, never jumps.
    fn stationary(&mut self, obs: &Obs, burst: bool) -> RawInput {
        if self.released_due {
            self.released_due = false;
            return RawInput::NONE;
        }
        let Some((mana, max)) = obs.mana else {
            return RawInput::NONE;
        };
        if burst {
            if self.refilling && mana == max {
                self.refilling = false;
            }
            if mana == 0 {
                self.refilling = true;
            }
            if self.refilling {
                return RawInput::NONE;
            }
        } else if mana == 0 {
            return RawInput::NONE;
        }
        if obs.cast_active || obs.donut_hurt {
            return RawInput::NONE;
        }
        self.released_due = true;
        self.presses += 1;
        RawInput {
            attack: true,
            ..RawInput::NONE
        }
    }

    /// Donut keeps the nearest llama inside the shooting band, faces it, jumps
    /// the glob lane of a telling llama that faces her and casts when legal.
    fn mobile(&mut self, obs: &Obs) -> RawInput {
        let me = obs.donut;
        let mut raw = RawInput::NONE;
        let Some(target) = nearest(me.x, &obs.llamas) else {
            return raw;
        };
        let dist = (target.x - me.x).abs();
        let dir = toward(me.x, target.x);
        if target.telling() && target.facing as f32 == -dir && dist < 260.0 && me.grounded {
            raw.jump = true;
        }
        if dist > SHOOT_BAND.1 {
            raw.move_x = dir;
        } else if dist < SHOOT_BAND.0 {
            raw.move_x = -dir;
        } else if me.facing as f32 != dir {
            // A one-tick nudge turns her toward the llama (facing follows move).
            raw.move_x = dir;
        }
        let can_cast = obs.mana.is_some_and(|(m, _)| m > 0) && !obs.cast_active && !obs.donut_hurt;
        if self.released_due {
            self.released_due = false;
        } else if can_cast && me.facing as f32 == dir && dist <= SHOOT_BAND.1 {
            self.released_due = true;
            self.presses += 1;
            raw.attack = true;
        }
        raw
    }

    /// Carl walks to the nearest llama and jabs it, one press then one release.
    fn melee(&mut self, obs: &Obs) -> RawInput {
        let me = obs.carl;
        let mut raw = RawInput::NONE;
        let Some(target) = nearest(me.x, &obs.llamas) else {
            return raw;
        };
        let dist = (target.x - me.x).abs();
        let dir = toward(me.x, target.x);
        // Close in, and turn toward the llama when already in reach.
        if dist > JAB_REACH || me.facing as f32 != dir {
            raw.move_x = dir;
        }
        if self.jab_pressed {
            self.jab_pressed = false;
        } else if dist <= JAB_REACH + 8.0 && me.facing as f32 == dir {
            self.jab_pressed = true;
            raw.attack = true;
        }
        raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(mana: u8, cast_active: bool) -> Obs {
        let hero = HeroObs {
            x: 300.0,
            facing: 1,
            health: 4,
            grounded: true,
        };
        Obs {
            tick: 1,
            carl: hero,
            donut: hero,
            mana: Some((mana, 8)),
            cast_active,
            donut_hurt: false,
            llamas: vec![],
        }
    }

    #[test]
    fn a_stationary_press_is_one_tick_and_releases_before_the_next() {
        let mut p = Policy::new(PolicyId::StationaryCast);
        let a = p.act(&obs(8, false));
        assert!(a.donut.attack);
        // The release tick carries no attack even though a cast could begin.
        let b = p.act(&obs(8, false));
        assert!(!b.donut.attack);
        let c = p.act(&obs(8, false));
        assert!(c.donut.attack, "the next legal cast is a new press edge");
        assert_eq!(p.presses, 2);
        assert_eq!(a.donut.move_x, 0.0);
        assert!(!a.donut.jump);
    }

    #[test]
    fn no_press_while_a_cast_runs_or_the_pool_is_dry() {
        let mut p = Policy::new(PolicyId::StationaryCast);
        assert!(!p.act(&obs(8, true)).donut.attack);
        assert!(!p.act(&obs(0, false)).donut.attack);
        assert_eq!(p.presses, 0);
    }

    #[test]
    fn the_burst_policy_spends_the_pool_then_waits_for_a_full_one() {
        let mut p = Policy::new(PolicyId::StationaryCastBurst);
        assert!(p.act(&obs(8, false)).donut.attack);
        let _ = p.act(&obs(7, false)); // release tick
        assert!(p.act(&obs(1, false)).donut.attack);
        let _ = p.act(&obs(0, false)); // release
        // Empty: refilling. It does not cast on a partial pool.
        for mana in 0..8 {
            assert!(!p.act(&obs(mana, false)).donut.attack, "mana {mana}");
        }
        assert!(p.act(&obs(8, false)).donut.attack, "full pool: next burst");
    }

    #[test]
    fn the_trivial_hold_counts_one_press() {
        let mut p = Policy::new(PolicyId::HoldAttackTrivial);
        for _ in 0..120 {
            assert!(p.act(&obs(8, false)).donut.attack);
        }
        assert_eq!(p.presses, 1);
    }
}
