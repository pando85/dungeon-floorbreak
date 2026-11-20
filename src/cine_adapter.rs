//! CINE-01 phase 2: the adapter between the combat event log and
//! [`crate::cine`], still Bevy-free so it can be tested headlessly.
//!
//! The adapter is a pure **observer**. [`CineAdapter::push`] takes the
//! combat events of one `print_combat_events` call (the single fan-out of the
//! spec, section 2 "Event handling"), [`CineAdapter::flush`] hands them to the
//! evaluator one resolved tick at a time and queues the resulting moment on
//! the presentation planner, and [`CineAdapter::frame`] is the per-frame step
//! the Bevy system feeds with real time. Nothing here writes a combat event,
//! a hash input, an RNG draw or a collision: the simulation never reads the
//! adapter back, so a run is identical with CINE on or off (CINE-T09).
//!
//! What the adapter adds on top of `CineEvent::from_combat`:
//!
//! * the **launch proof** (`CINE-CONTACT-SETUP`): a `PushKick` `Hit` remembers
//!   `(instance, target)`; a later `GoblinContact` whose `launch` equals that
//!   instance and whose `source` is that target emits
//!   [`CineEvent::LaunchCredited`]. Never inferred from screen proximity;
//! * the **batching**: events are grouped into one batch per resolved tick, in
//!   log order, with a `Restart` always alone in its own batch (the attempt
//!   after it counts its ticks from 1 again);
//! * the **cancel flag**: a `Restart`, `Respawn` or `HeroDefeated` cancels a
//!   pending or active moment on the next frame (spec section 3).

use std::collections::VecDeque;

use crate::cine::{
    CineCause, CineEvent, CineTuning, CinematicEvaluator, CinematicMoment, FrameContext, FramePlan,
    PlannerState, PresentationSettings, RequestOutcome, SlowMoPlanner,
};
use crate::combat::{CombatEvent, MoveId};

/// How many launch kicks are remembered for the launch proof. A contact must
/// follow its kick within the flight of one goblin, so a handful is plenty.
const KICK_MEMORY: usize = 16;

/// Whether a defeat is credited to a hero the companion was driving (CA-29):
/// the melee `instigator`, or the `player` the cause names. Any other event is
/// not a credit.
fn is_companion_credited(event: &CineEvent, companion_driven: [bool; 2]) -> bool {
    let CineEvent::Defeated {
        cause, instigator, ..
    } = *event
    else {
        return false;
    };
    let driven = |hero: u32| {
        companion_driven
            .get(hero as usize)
            .copied()
            .unwrap_or(false)
    };
    let named = match cause {
        CineCause::ShoreBlock { player }
        | CineCause::ShoreBlockEdge { player }
        | CineCause::DropHazard { player } => Some(player),
        CineCause::Melee | CineCause::Other => None,
    };
    instigator.is_some_and(driven) || named.is_some_and(driven)
}

/// A moment the evaluator emitted and what the planner did with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Emission {
    pub moment: CinematicMoment,
    pub outcome: RequestOutcome,
}

/// See the module documentation.
#[derive(Debug, Clone)]
pub struct CineAdapter {
    evaluator: CinematicEvaluator,
    planner: SlowMoPlanner,
    pending: Vec<CineEvent>,
    /// `(attacker hero, kick instance, kicked goblin)`.
    kicks: VecDeque<(u32, u32, u32)>,
    cancel: bool,
    emitted: u32,
    target: Option<u32>,
}

impl CineAdapter {
    pub fn new(settings: PresentationSettings) -> Self {
        Self::with_tuning(settings, CineTuning::DEFAULT)
    }

    pub fn with_tuning(settings: PresentationSettings, tuning: CineTuning) -> Self {
        Self {
            evaluator: CinematicEvaluator::new(tuning),
            planner: SlowMoPlanner::new(settings),
            pending: Vec::new(),
            kicks: VecDeque::new(),
            cancel: false,
            emitted: 0,
            target: None,
        }
    }

    pub fn settings(&self) -> PresentationSettings {
        self.planner.settings()
    }

    pub fn state(&self) -> PlannerState {
        self.planner.state()
    }

    pub fn evaluator(&self) -> &CinematicEvaluator {
        &self.evaluator
    }

    /// Moments emitted so far (whether or not presentation was on).
    pub const fn emitted(&self) -> u32 {
        self.emitted
    }

    /// The target whose defeat sequence the queued or running moment waits
    /// for (the adapter skips the moment when that finish is no longer
    /// playing). `None` before the first moment.
    pub const fn moment_target(&self) -> Option<u32> {
        self.target
    }

    /// Feed one combat event, in log order. The companion is off for this
    /// call: [`Self::push_driven`] with no hero companion-driven.
    pub fn push(&mut self, event: &CombatEvent) {
        self.push_driven(event, [false; 2]);
    }

    /// Feed one combat event with the heroes the companion drove at its tick
    /// (`CombatState::companion_driven`, Carl 0, Donut 1).
    ///
    /// **CA-29, fail closed.** A defeat credited to a companion-driven hero is
    /// not forwarded to the evaluator, so it can neither trigger a moment nor
    /// join another's chain. The credited hero is the `instigator` of a melee
    /// defeat and the cause's `player` for `ShoreBlock`, `ShoreBlockEdge` and
    /// `DropHazard`; `slot` alone never gates a moment. A defeat that names no
    /// hero is not credited to anyone and passes through. Presentation only:
    /// no combat event, hash or RNG changes, and AUDIENCE credit stays factual.
    pub fn push_driven(&mut self, event: &CombatEvent, companion_driven: [bool; 2]) {
        match *event {
            CombatEvent::Hit {
                move_id: move_id @ MoveId::PushKick,
                instance,
                target,
                ..
            } => {
                // The attacker is the owner of the move (`MoveId::hero`), never
                // a default: `Hit` carries no attacker field.
                self.kicks.push_back((move_id.hero(), instance, target));
                while self.kicks.len() > KICK_MEMORY {
                    self.kicks.pop_front();
                }
            }
            CombatEvent::GoblinContact {
                tick,
                launch,
                source,
                ..
            } => {
                if let Some(&(hero, ..)) = self
                    .kicks
                    .iter()
                    .rev()
                    .find(|&&(_, instance, goblin)| instance == launch && goblin == source)
                {
                    self.pending.push(CineEvent::LaunchCredited {
                        tick,
                        goblin: source,
                        player: hero,
                    });
                }
            }
            CombatEvent::Restart { .. } => self.kicks.clear(),
            _ => {}
        }
        if matches!(
            event,
            CombatEvent::Restart { .. }
                | CombatEvent::Respawn { .. }
                | CombatEvent::HeroDefeated { .. }
        ) {
            self.cancel = true;
        }
        if let Some(mapped) = CineEvent::from_combat(event) {
            if is_companion_credited(&mapped, companion_driven) {
                return;
            }
            self.pending.push(mapped);
        }
    }

    /// Evaluate what was pushed since the last call: one `observe_tick` per
    /// resolved tick (consecutive events with the same tick, a `Restart`
    /// always alone). The evaluator runs whatever the presentation setting is,
    /// so the moments are replay-identical with CINE off; only the planner
    /// request is suppressed then.
    pub fn flush(&mut self) -> Vec<Emission> {
        let events = std::mem::take(&mut self.pending);
        let mut out = Vec::new();
        let mut start = 0;
        while start < events.len() {
            let tick = events[start].tick();
            let mut end = start + 1;
            if !matches!(events[start], CineEvent::Restart { .. }) {
                while end < events.len()
                    && events[end].tick() == tick
                    && !matches!(events[end], CineEvent::Restart { .. })
                {
                    end += 1;
                }
            }
            if let Some(moment) = self
                .evaluator
                .observe_tick(tick, &events[start..end], false)
            {
                self.emitted += 1;
                let outcome = self
                    .planner
                    .request(self.evaluator.tuning().request_for(&moment));
                if matches!(
                    outcome,
                    RequestOutcome::Queued | RequestOutcome::UpgradedPending
                ) {
                    self.target = Some(moment.target);
                }
                out.push(Emission { moment, outcome });
            }
            start = end;
        }
        out
    }

    /// One frame of the presentation clock. A restart, respawn or party
    /// defeat seen since the last frame cancels the moment (as does
    /// `ctx.locked`).
    pub fn frame(&mut self, mut ctx: FrameContext) -> FramePlan {
        ctx.locked |= std::mem::take(&mut self.cancel);
        self.planner.step(ctx)
    }

    /// Change the presentation setting (the planner restores 1.0x on the next
    /// frame if a moment was running).
    pub fn set_settings(&mut self, settings: PresentationSettings) {
        self.planner.set_settings(settings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss::BOSS_ENEMY_ID;
    use crate::cine::{CineTier, PlanEnd};
    use crate::combat::{DefeatCause, ForceTier};

    fn defeat(tick: u64, target: u32, cause: DefeatCause, instigator: Option<u32>) -> CombatEvent {
        CombatEvent::enemy_defeated(tick, target, cause, instigator, None)
    }

    fn feed(a: &mut CineAdapter, events: &[CombatEvent]) -> Vec<Emission> {
        for e in events {
            a.push(e);
        }
        a.flush()
    }

    fn ctx() -> FrameContext {
        FrameContext {
            real_dt: 1.0 / 60.0,
            hit_stop_left: 0,
            finish_active: true,
            paused: false,
            locked: false,
        }
    }

    const MELEE: DefeatCause = DefeatCause::Melee;

    #[test]
    fn the_launch_proof_needs_the_kick_instance_and_the_kicked_goblin() {
        let kick = CombatEvent::Hit {
            tick: 10,
            move_id: MoveId::PushKick,
            instance: 7,
            target: 3,
            damage: 1,
            health_left: 2,
            hit_stop: 8,
        };
        let contact = |launch, source| CombatEvent::GoblinContact {
            tick: 20,
            launch,
            source,
            target: 4,
        };
        let launched = |a: &mut CineAdapter, c: CombatEvent| {
            for e in [kick, c] {
                a.push(&e);
            }
            a.pending.iter().any(|e| {
                matches!(
                    e,
                    CineEvent::LaunchCredited {
                        goblin: 3,
                        player: 0,
                        ..
                    }
                )
            })
        };
        assert!(launched(
            &mut CineAdapter::new(PresentationSettings::OFF),
            contact(7, 3)
        ));
        // A different kick instance, or a source that was not the kicked one.
        assert!(!launched(
            &mut CineAdapter::new(PresentationSettings::OFF),
            contact(8, 3)
        ));
        assert!(!launched(
            &mut CineAdapter::new(PresentationSettings::OFF),
            contact(7, 5)
        ));
        // A jab is not a launch.
        let mut a = CineAdapter::new(PresentationSettings::OFF);
        a.push(&CombatEvent::Hit {
            tick: 10,
            move_id: MoveId::Jab,
            instance: 7,
            target: 3,
            damage: 1,
            health_left: 2,
            hit_stop: 3,
        });
        a.push(&contact(7, 3));
        assert!(
            !a.pending
                .iter()
                .any(|e| matches!(e, CineEvent::LaunchCredited { .. }))
        );
    }

    #[test]
    fn every_move_names_its_hero_and_donut_moves_are_never_carls() {
        for mv in [MoveId::Swat, MoveId::Pounce, MoveId::MagicMissile] {
            assert_eq!(mv.hero(), 1, "{mv:?}");
        }
        for mv in [
            MoveId::GreyboxStrike,
            MoveId::Jab,
            MoveId::Haymaker,
            MoveId::PushKick,
        ] {
            assert_eq!(mv.hero(), 0, "{mv:?}");
        }
    }

    /// Guard: a launch proven for Donut (hero 1) and her kills make hero 1's
    /// legendary chain, with the contact bonus credited to hero 1 only; the
    /// same chain with Carl's kills does not borrow Donut's launch.
    #[test]
    fn a_donut_launch_and_her_kills_credit_hero_one_never_hero_zero() {
        let chain = |launcher: u32, killer: u32| {
            let mut a = CineAdapter::new(PresentationSettings::DEFAULT);
            // The launch proof as the `Hit` handler records it: the attacker
            // is the owner of the launching move.
            a.kicks.push_back((launcher, 5, 9));
            feed(
                &mut a,
                &[
                    CombatEvent::GoblinContact {
                        tick: 110,
                        launch: 5,
                        source: 9,
                        target: 2,
                    },
                    defeat(120, 1, MELEE, Some(killer)),
                    CombatEvent::ShoreBroken {
                        tick: 125,
                        player: killer,
                    },
                    defeat(
                        150,
                        2,
                        DefeatCause::ShoreBlock { player: killer },
                        Some(killer),
                    ),
                    defeat(150, 3, MELEE, Some(killer)),
                ],
            )
        };
        let donut = chain(1, 1);
        assert_eq!(donut.len(), 1, "{donut:?}");
        let m = donut[0].moment;
        assert_eq!((m.credited, m.tier, m.score), (1, CineTier::Legendary, 8));
        assert!(m.feats.contact_setup);
        // Carl's kills with a launch that was Donut's: ordinary at best, and
        // never the contact setup.
        let carl = chain(1, 0);
        assert_eq!(carl.len(), 1, "{carl:?}");
        assert!(!carl[0].moment.feats.contact_setup);
        assert_eq!(carl[0].moment.credited, 0);
    }

    #[test]
    fn t06_a_proven_launch_a_contact_and_a_third_credited_kill_is_legendary() {
        let mut a = CineAdapter::new(PresentationSettings::DEFAULT);
        let events = [
            CombatEvent::Hit {
                tick: 100,
                move_id: MoveId::PushKick,
                instance: 5,
                target: 9,
                damage: 1,
                health_left: 2,
                hit_stop: 8,
            },
            CombatEvent::GoblinContact {
                tick: 110,
                launch: 5,
                source: 9,
                target: 2,
            },
            defeat(120, 1, MELEE, Some(0)),
            CombatEvent::ShoreBroken {
                tick: 125,
                player: 0,
            },
            defeat(150, 2, DefeatCause::ShoreBlock { player: 0 }, Some(0)),
            defeat(150, 3, MELEE, Some(0)),
        ];
        let out = feed(&mut a, &events);
        assert_eq!(out.len(), 1, "one moment, not one per kill: {out:?}");
        let m = out[0].moment;
        assert_eq!((m.tier, m.score), (CineTier::Legendary, 8));
        assert_eq!(out[0].outcome, RequestOutcome::Queued);
    }

    #[test]
    fn f8_a_boss_defeat_never_triggers_cine() {
        let mut a = CineAdapter::new(PresentationSettings::DEFAULT);
        let out = feed(
            &mut a,
            &[
                defeat(2, 4, MELEE, Some(0)),
                defeat(
                    3,
                    BOSS_ENEMY_ID,
                    DefeatCause::DropHazard { player: 0, rack: 1 },
                    Some(0),
                ),
            ],
        );
        assert!(out.is_empty(), "melee + a rack kill on the boss: {out:?}");
    }

    #[test]
    fn f7_the_drop_credit_helper_covers_the_block_the_edge_and_the_rack() {
        for (cause, player) in [
            (DefeatCause::ShoreBlock { player: 1 }, Some(1)),
            (DefeatCause::ShoreBlockEdge { player: 0 }, Some(0)),
            (DefeatCause::DropHazard { player: 1, rack: 2 }, Some(1)),
            (DefeatCause::Melee, None),
            (
                DefeatCause::Explosion {
                    charge: 0,
                    player: Some(0),
                    point_blank: true,
                },
                None,
            ),
        ] {
            assert_eq!(cause.drop_credit(), player, "{cause:?}");
        }
        // A melee kill plus a player-dropped rack kill on another target is a
        // mixed chain through the same helper (no boss involved).
        let mut a = CineAdapter::new(PresentationSettings::OFF);
        let out = feed(
            &mut a,
            &[
                defeat(10, 1, MELEE, Some(0)),
                defeat(
                    40,
                    2,
                    DefeatCause::DropHazard { player: 0, rack: 0 },
                    Some(0),
                ),
            ],
        );
        assert_eq!(out.len(), 1);
        assert!(out[0].moment.feats.mixed_chain);
        assert_eq!(out[0].outcome, RequestOutcome::Suppressed);
    }

    #[test]
    fn a_restart_is_its_own_batch_and_ticks_that_restart_at_one_are_not_merged() {
        let mut a = CineAdapter::new(PresentationSettings::DEFAULT);
        // A finished attempt ending on tick 1 followed by a new attempt whose
        // first tick is also 1: the restart must not swallow the new events.
        let out = feed(
            &mut a,
            &[
                CombatEvent::Restart { tick: 1, deaths: 1 },
                CombatEvent::ShoreBroken { tick: 1, player: 0 },
                defeat(1, 2, DefeatCause::ShoreBlockEdge { player: 0 }, Some(0)),
            ],
        );
        assert_eq!(out.len(), 1, "the edge shear after the restart counts");
        assert!(out[0].moment.feats.precision_edge);
    }

    #[test]
    fn restart_respawn_and_party_defeat_cancel_a_running_moment_on_the_next_frame() {
        for cancel in [
            CombatEvent::Restart { tick: 9, deaths: 1 },
            CombatEvent::Respawn {
                tick: 9,
                checkpoint: 1,
            },
            CombatEvent::HeroDefeated {
                tick: 9,
                hero: crate::party::HeroId::Carl,
            },
        ] {
            let mut a = CineAdapter::new(PresentationSettings::DEFAULT);
            let out = feed(
                &mut a,
                &[
                    CombatEvent::ShoreBroken { tick: 1, player: 0 },
                    defeat(5, 2, DefeatCause::ShoreBlockEdge { player: 0 }, Some(0)),
                ],
            );
            assert_eq!(out.len(), 1);
            assert_eq!(a.frame(ctx()).state, PlannerState::Active);
            a.push(&cancel);
            let f = a.frame(ctx());
            assert_eq!(
                (f.relative_speed, f.state, f.ended),
                (1.0, PlannerState::Idle, Some(PlanEnd::CancelledLocked)),
                "{cancel:?}"
            );
        }
    }

    #[test]
    fn the_evaluator_runs_with_cine_off_and_only_the_request_is_suppressed() {
        let events = [
            CombatEvent::ShoreBroken { tick: 1, player: 0 },
            defeat(5, 2, DefeatCause::ShoreBlockEdge { player: 0 }, Some(0)),
        ];
        let mut on = CineAdapter::new(PresentationSettings::DEFAULT);
        let mut off = CineAdapter::new(PresentationSettings::OFF);
        let (a, b) = (feed(&mut on, &events), feed(&mut off, &events));
        assert_eq!(a.len(), 1);
        assert_eq!(
            a.iter().map(|e| e.moment).collect::<Vec<_>>(),
            b.iter().map(|e| e.moment).collect::<Vec<_>>()
        );
        assert_eq!(off.state(), PlannerState::Idle);
        let f = off.frame(ctx());
        assert_eq!((f.relative_speed, f.camera_emphasis_px), (1.0, 0));
        assert_eq!(off.emitted(), 1);
    }

    #[test]
    fn a_force_tier_never_decides_anything() {
        // The same lone melee defeat at every force tier: no moment.
        for force in [
            ForceTier::Routine,
            ForceTier::Heavy,
            ForceTier::Catastrophic,
        ] {
            let mut a = CineAdapter::new(PresentationSettings::DEFAULT);
            let event = CombatEvent::EnemyDefeated {
                tick: 3,
                target: 1,
                cause: MELEE,
                force,
                instigator: Some(0),
            };
            assert!(feed(&mut a, &[event]).is_empty(), "{force:?}");
        }
    }

    /// CA-29: a defeat credited to a companion-driven hero never reaches the
    /// evaluator (fail closed), whichever field names the credit; a defeat
    /// credited to the human-driven hero, or to nobody, still does.
    #[test]
    fn a_companion_credited_defeat_never_triggers_a_moment() {
        let edge = |player| {
            [
                CombatEvent::ShoreBroken { tick: 1, player },
                defeat(5, 2, DefeatCause::ShoreBlockEdge { player }, Some(player)),
            ]
        };
        let run = |events: &[CombatEvent], driven: [bool; 2]| {
            let mut a = CineAdapter::new(PresentationSettings::OFF);
            for e in events {
                a.push_driven(e, driven);
            }
            (a.flush().len(), a.emitted())
        };
        // Carl credited: a moment while he is human-driven, none while he is the companion.
        assert_eq!(
            run(&edge(0), [false, true]).0,
            1,
            "the human hero's credit counts"
        );
        assert_eq!(
            run(&edge(0), [true, false]).0,
            0,
            "the companion's credit does not"
        );
        assert_eq!(
            run(&edge(0), [false, false]).0,
            1,
            "companion off: as before"
        );
        // The same for Donut (hero index 1).
        assert_eq!(run(&edge(1), [true, false]).0, 1);
        assert_eq!(run(&edge(1), [false, true]).0, 0);
        // The cause's player gates even when the instigator names the human.
        let mixed = [
            CombatEvent::ShoreBroken { tick: 1, player: 1 },
            defeat(5, 2, DefeatCause::ShoreBlockEdge { player: 1 }, Some(0)),
        ];
        assert_eq!(run(&mixed, [false, true]).0, 0);
        // `push` is the companion-off feed.
        let mut a = CineAdapter::new(PresentationSettings::OFF);
        for e in &edge(0) {
            a.push(e);
        }
        assert_eq!(a.flush().len(), 1);
        // A defeat that names no hero is credited to nobody and is not gated.
        assert!(!is_companion_credited(
            &CineEvent::from_combat(&defeat(5, 2, MELEE, None)).unwrap(),
            [true, true]
        ));
        // Only defeats are gated.
        assert!(!is_companion_credited(
            &CineEvent::HeroHurt { tick: 1, player: 0 },
            [true, true]
        ));
    }
}
