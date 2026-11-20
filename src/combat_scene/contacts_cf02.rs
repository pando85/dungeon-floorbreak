//! CF02-B: body contacts and wall slams under `cf02_rules` (Q09..Q12).
//!
//! A kicked flier (a living `Launched` goblin with a launch instance) or the body
//! of a goblin killed by a lethal kick (it kept the kick's launch instance, Q10)
//! that overlaps a living goblin knocks it down, and one that is stopped by a
//! solid at 120 px/s or more slams into the wall (Q11). The legacy
//! [`CombatState::resolve_contacts`] stays the code of every run without
//! `cf02_rules`; this module replaces it only when the tuning is on.
//!
//! Contacts and slams are decided on the tick the flight happens, after the
//! goblins' own step (the same slot as the legacy contacts) and in index order.
//! There is **one effect per (launch, target)**, and a knocked-down goblin carries
//! no launch instance, so it can never chain into a third goblin. Nothing here
//! deals contact damage (Q12); the only damage is the wall's (Q11).

use super::CombatState;
use crate::cf02::{Cf02Tuning, ContactMode};
use crate::combat::{CombatEvent, DefeatCause, MoveId, carl_move};
use crate::goblin::GoblinMode;

/// What kind of flier a goblin is this tick.
struct Flier {
    launch: u32,
    /// A killed body that bowls (Q10), not a living flier.
    body: bool,
}

impl CombatState {
    /// The flier `goblin` is, if any: a living `Launched` goblin with a launch
    /// instance, or the `Airborne` body of a lethal kick that kept one.
    fn flier_of(&self, goblin: usize) -> Option<Flier> {
        let g = &self.enemies[goblin];
        match (g.mode, g.launch, g.bowl) {
            (GoblinMode::Launched, Some(launch), _) if g.alive() => Some(Flier {
                launch,
                body: false,
            }),
            (GoblinMode::Airborne, _, Some(launch)) if !g.alive() => {
                Some(Flier { launch, body: true })
            }
            _ => None,
        }
    }

    /// CF02-Q09..Q12: the contacts and wall slams of this tick.
    pub(super) fn resolve_contacts_cf02(&mut self, t: &'static Cf02Tuning, tick: u64) {
        let min_vx = f32::from(t.q09_contact_min_vx);
        for source in 0..self.enemies.len() {
            let Some(flier) = self.flier_of(source) else {
                continue;
            };
            if t.on(t.q11_wall_slam)
                && self.enemies[source]
                    .wall_stop
                    .is_some_and(|speed| speed >= min_vx)
            {
                self.wall_slam(t, tick, source, &flier);
                continue;
            }
            // The speed the flier had when this tick began its contacts: every
            // goblin it overlaps now is judged against the same speed.
            let speed = self.enemies[source].vx.abs();
            for target in 0..self.enemies.len() {
                if target == source
                    || self.contacts.contains(&(flier.launch, target as u32))
                    || !self.enemies[target].can_be_contacted()
                    || !self.enemies[source]
                        .body()
                        .overlaps(self.enemies[target].body())
                {
                    continue;
                }
                self.contacts.push((flier.launch, target as u32));
                let push: i8 = if self.enemies[target].x >= self.enemies[source].x {
                    1
                } else {
                    -1
                };
                let was_attacking = self.enemies[target].token_held();
                let instigator = self.enemies[source].hit_by;
                if flier.body {
                    self.enemies[source].contact_as_body_cf02(t);
                } else {
                    self.enemies[source].contact_as_flier_cf02(t);
                }
                if t.q09_contact_mode == ContactMode::Knockdown && speed >= min_vx {
                    self.enemies[target].contact_knockdown(push, t);
                } else {
                    self.enemies[target].contact_stagger(
                        push,
                        t.q09_legacy_stagger,
                        f32::from(t.q09_legacy_push),
                        t.q09_flier_hit_stop,
                    );
                }
                if flier.body {
                    self.log(CombatEvent::BodyContact {
                        tick,
                        launch: flier.launch,
                        source: source as u32,
                        target: target as u32,
                        instigator,
                    });
                } else {
                    self.log(CombatEvent::GoblinContact {
                        tick,
                        launch: flier.launch,
                        source: source as u32,
                        target: target as u32,
                    });
                }
                if was_attacking {
                    self.log(CombatEvent::GoblinInterrupted {
                        tick,
                        goblin: target as u32,
                    });
                }
                if let Some(slot) = self.flash_tick.get_mut(target) {
                    *slot = Some(tick);
                }
            }
        }
    }

    /// CF02-Q11: the flier `source` was stopped by a solid at speed. A living
    /// goblin takes the wall damage (defeated in place at 0 health, credited to the
    /// hero whose kick made the launch: `Melee`, force Heavy); a dead body only
    /// crumples. A slam ends the flight, so it happens once per launch.
    fn wall_slam(&mut self, t: &'static Cf02Tuning, tick: u64, source: usize, flier: &Flier) {
        let instigator = self.enemies[source].hit_by;
        self.log(CombatEvent::WallSlam {
            tick,
            launch: flier.launch,
            goblin: source as u32,
            instigator,
        });
        if let Some(slot) = self.flash_tick.get_mut(source) {
            *slot = Some(tick);
        }
        if flier.body {
            // Dead: the body crumples where it stopped and bowls nothing more.
            self.enemies[source].bowl = None;
            return;
        }
        if self.enemies[source].wall_slam(t) {
            // A kick-launched goblin that dies against the wall is a kick kill.
            let spec = carl_move(MoveId::PushKick);
            self.log(CombatEvent::enemy_defeated(
                tick,
                source as u32,
                DefeatCause::Melee,
                instigator,
                spec,
            ));
        }
    }

    /// CF02-Q10: stamp the bowling instance on a goblin a lethal kick just defeated
    /// (the kick's move instance), when the move is on the tuning's list.
    pub(super) fn stamp_bowl(&mut self, goblin: usize, move_id: MoveId, instance: u32) {
        let Some(t) = self.cf02 else {
            return;
        };
        let listed = match move_id {
            MoveId::PushKick => crate::cf02::BowlMove::PushKick,
            MoveId::FollowKick => crate::cf02::BowlMove::FollowKick,
            MoveId::AirKick => crate::cf02::BowlMove::AirKick,
            _ => return,
        };
        let body = &mut self.enemies[goblin];
        if t.on(t.q10_bowling)
            && t.q10_bowl_moves.contains(&listed)
            && body.mode == GoblinMode::Airborne
            && !body.alive()
        {
            body.bowl = Some(instance);
        }
    }
}

#[cfg(test)]
impl CombatState {
    /// Test access to [`Self::stamp_bowl`] (the private hit loop calls it).
    pub(crate) fn stamp_bowl_for_test(&mut self, goblin: usize, move_id: MoveId, instance: u32) {
        self.stamp_bowl(goblin, move_id, instance);
    }
}
