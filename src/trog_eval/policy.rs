//! The movement policies of `trog_eval` (plan 7.3 row 4): closed loops on the observable state,
//! built from the TROG-A fixtures' brawler and caster ([`crate::trog_fixtures::carl_vs`],
//! [`crate::trog_fixtures::donut_vs`], [`crate::trog_fixtures::with_hop`]). A policy reads only
//! the finished state of the previous tick, never the future, and never writes game state.

use crate::encounter::StaticKind;
use crate::input::RawInput;
use crate::party::HeroId;
use crate::trog_fixtures::{TrogSim, carl_vs, donut_vs, moving, with_hop};

/// The press bookkeeping of the movement policies.
#[derive(Debug, Clone, Default)]
pub struct Brain {
    last_carl: u64,
    last_donut: u64,
}

fn exit_x(sim: &TrogSim) -> f32 {
    sim.run.loaded.encounter.exit.x as f32 + 32.0
}

impl Brain {
    /// Carl fights the nearest living troglodyte with the brawler policy (back out of a
    /// committed tell, punish the recovery, step into the Virtuoso's dead zone), hops the lane
    /// steps (the shelf and the bench) and walks to the exit once the room is clear.
    pub fn carl_melee(&mut self, sim: &TrogSim) -> RawInput {
        let probe = sim.probe();
        let raw = match sim.living().first().copied() {
            Some(i) => {
                let t = sim.trog(i).clone();
                carl_vs(&probe, &t, &mut self.last_carl)
            }
            None => moving(if sim.x_of(HeroId::Carl) < exit_x(sim) {
                1.0
            } else {
                0.0
            }),
        };
        with_hop(&probe, HeroId::Carl, raw)
    }

    /// Donut moves: she walks toward the nearest living troglodyte, crosses any lane step
    /// between them (a hop onto it, then off the far side), then keeps a firing band and casts
    /// on the cooldown (`donut_vs`). Carl is parked or follows; she never reads the future.
    pub fn mobile_cast(&mut self, sim: &TrogSim) -> RawInput {
        let probe = sim.probe();
        let donut = sim.x_of(HeroId::Donut);
        let raw = match sim.living().first().copied() {
            Some(i) => {
                let t = sim.trog(i).clone();
                let (lo, hi) = (donut.min(t.x), donut.max(t.x));
                let step_between = probe.statics.iter().any(|st| {
                    st.kind == StaticKind::Solid
                        && st.rect.h > 16
                        && (st.rect.x as f32) > lo
                        && (st.rect.right() as f32) < hi
                });
                if step_between && sim.foot_of(HeroId::Donut) <= sim.foot_of_floor() + 20.0 {
                    // Cross it: walk on toward the troglodyte (the hop is `with_hop`'s).
                    moving(if t.x >= donut { 1.0 } else { -1.0 })
                } else {
                    donut_vs(&probe, &t, &mut self.last_donut)
                }
            }
            None => moving(if donut < exit_x(sim) { 1.0 } else { 0.0 }),
        };
        with_hop(&probe, HeroId::Donut, raw)
    }
}
