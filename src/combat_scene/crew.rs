//! DR-C (DZ-Q20, DZ-Q21): the Dozer's crew.
//!
//! A roster goblin with a `mount` rides the room's machine. While carried it is
//! parked like a goblin retired behind the start (health 0, a corpse, passive,
//! at [`RETIRED_X`]): no body, never a target, no tells and never drawn by the
//! goblin presenter. Its slot in the state hash is a single CARRIED word.
//!
//! On the tick the machine turns `disabled` (48 ticks after the frame landed)
//! every carried rider lands [`Mount::land_dx`] px right of the wheel front,
//! nudged right while it would overlap a hero, clamped inside the lane before
//! `stop_x` and the view, facing the nearest hero, and starts in the CF02
//! knockdown for [`Mount::down_ticks`]. A machine that is never disabled never
//! sheds a rider. A defeat retry carries both riders again; a checkpoint
//! respawn carries the living ones again and leaves the dead ones dead.

use super::{CombatState, RETIRED_X};
use crate::combat::{ENEMY_HALF_WIDTH, EnemyAi};
use crate::corridor::SceneSim;
use crate::encounter::Mount;
use crate::goblin::GoblinMode;
use crate::party::HeroId;
use crate::prototype::HERO_HALF_WIDTH;

/// The hash word of a carried rider ("CARRIED").
const CARRIED: u64 = 0x43_4152_5249_4544;

/// DZ-Q20: the landing column of a rider. It starts at `front + land_dx`, moves
/// `nudge_px` right while a hero body overlaps it (at most `max_nudges` times;
/// after the last nudge it lands there anyway) and is clamped into `[lo, hi]`
/// (the view and the lane before `stop_x`, already inset by the half width).
pub fn landing_x(front: f32, mount: &Mount, heroes: &[f32], lo: f32, hi: f32) -> f32 {
    let overlaps = |x: f32| {
        heroes
            .iter()
            .any(|h| (h - x).abs() < ENEMY_HALF_WIDTH + HERO_HALF_WIDTH)
    };
    let mut x = front + mount.land_dx as f32;
    for _ in 0..mount.max_nudges {
        if !overlaps(x) {
            break;
        }
        x += mount.nudge_px as f32;
    }
    if lo > hi { hi } else { x.clamp(lo, hi) }
}

impl CombatState {
    /// DR-C: the mounts of the run's enemies (index order: goblins, then
    /// Engineers), from the room's roster. Run configuration, never hashed. Every
    /// mounted rider is carried at once.
    #[must_use]
    pub fn with_crew(mut self, crew: Vec<Option<Mount>>) -> Self {
        self.crew = crew;
        self.carry_crew();
        self
    }

    /// DR-C: the mount of enemy `index` (`None`: not a rider).
    pub fn crew_of(&self, index: usize) -> Option<Mount> {
        self.crew.get(index).copied().flatten()
    }

    /// DR-C: enemy `index` rides the machine (not drawn, not a target).
    pub fn is_carried(&self, index: usize) -> bool {
        self.carried.get(index).copied().unwrap_or(false)
    }

    /// DR-C: whether the DOZ-01 rig draws a rider on `seat`: a run without a
    /// crew keeps both seats drawn (the DR-A/DR-B rig); with a crew, a seat is
    /// drawn while its rider is carried (not after the dismount, nor for a
    /// rider that stayed dead through a respawn).
    pub fn seat_taken(&self, seat: crate::encounter::Seat) -> bool {
        !self.has_crew()
            || (0..self.crew.len())
                .any(|i| self.is_carried(i) && self.crew_of(i).is_some_and(|m| m.seat == seat))
    }

    /// Whether this run has a crew at all.
    fn has_crew(&self) -> bool {
        self.crew.iter().any(Option::is_some)
    }

    /// Park rider `index` aboard the machine, the retired-goblin way.
    fn carry(&mut self, index: usize) {
        let Some(mut goblin) = self.fresh_enemy(index) else {
            return;
        };
        goblin.health = 0;
        goblin.mode = GoblinMode::Corpse;
        goblin.ai = EnemyAi::Passive;
        goblin.x = RETIRED_X;
        self.enemies[index] = goblin;
        if self.carried.len() < self.enemies.len() {
            self.carried.resize(self.enemies.len(), false);
        }
        self.carried[index] = true;
        self.clear_slots_of(index);
        // Its morale bookkeeping follows the parked body (health 0), so a later
        // tick never reads a drop as a hit.
        self.respawn_morale_of(index);
    }

    /// The per-enemy bookkeeping of `index` goes back to its spawn value.
    fn clear_slots_of(&mut self, index: usize) {
        if let Some(t) = self.targets.get_mut(index) {
            *t = None;
        }
        if let Some(t) = self.flash_tick.get_mut(index) {
            *t = None;
        }
        if let Some(t) = self.carl_hit_tick.get_mut(index) {
            *t = None;
        }
        if let Some(t) = self.attack_started.get_mut(index) {
            *t = 0;
        }
    }

    /// A defeat retry (and the run's start): both riders are carried again.
    pub(super) fn carry_crew(&mut self) {
        self.carried = vec![false; self.enemies.len()];
        if !self.has_crew() {
            self.carried.clear();
            return;
        }
        for index in 0..self.enemies.len() {
            if self.crew_of(index).is_some() {
                self.carry(index);
            }
        }
    }

    /// A checkpoint respawn: a living rider is carried again, a carried one
    /// stays carried and a dead one stays dead (the CF02 rule).
    pub(super) fn crew_respawn(&mut self) {
        if !self.has_crew() {
            return;
        }
        for index in 0..self.enemies.len() {
            if self.crew_of(index).is_some()
                && !self.is_carried(index)
                && self.enemies[index].health > 0
            {
                self.carry(index);
            }
        }
    }

    /// DZ-Q20: on the tick the machine is disabled every carried rider lands.
    pub(super) fn step_crew(&mut self, scene: &SceneSim) {
        if !self.carried.iter().any(|&c| c) {
            return;
        }
        let Some((front, stop_x)) = self.slot.enemies.iter().find_map(|e| {
            let dozer = e
                .as_any()?
                .downcast_ref::<super::dozer::DozerEnemy>()?
                .dozer();
            (dozer.state == crate::dozer::DozerState::Disabled)
                .then(|| (dozer.wheel_front(), dozer.site.stop_x as f32))
        }) else {
            return;
        };
        let heroes: Vec<f32> = if scene.party_live() {
            vec![
                scene.world.body(HeroId::Carl).x,
                scene.world.body(HeroId::Donut).x,
            ]
        } else {
            vec![scene.world.body(HeroId::Carl).x]
        };
        let (view_l, view_r) = scene.view_x();
        let lo = view_l + ENEMY_HALF_WIDTH;
        let hi = view_r.min(stop_x) - ENEMY_HALF_WIDTH;
        for index in 0..self.enemies.len() {
            let Some(mount) = self.crew_of(index).filter(|_| self.is_carried(index)) else {
                continue;
            };
            let Some(mut goblin) = self.fresh_enemy(index) else {
                continue;
            };
            let x = landing_x(front, &mount, &heroes, lo, hi);
            goblin.x = x;
            goblin.facing = heroes
                .iter()
                .min_by(|a, b| (*a - x).abs().total_cmp(&(*b - x).abs()))
                .map_or(-1, |h| if *h >= x { 1 } else { -1 });
            goblin.knock_down_for(mount.down_ticks);
            self.enemies[index] = goblin;
            self.carried[index] = false;
            self.clear_slots_of(index);
            self.respawn_morale_of(index);
        }
    }

    /// The state hash of enemy `index`: a carried rider is one CARRIED word.
    pub(super) fn mix_enemy(&self, index: usize, put: &mut impl FnMut(u64)) {
        if self.is_carried(index) {
            put(CARRIED);
        } else {
            self.enemies[index].mix_into(put);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cf02::Temperament;
    use crate::encounter::Seat;

    fn mount(dx: i32) -> Mount {
        Mount {
            seat: Seat::RiderA,
            land_dx: dx,
            down_ticks: 45,
            nudge_px: 32,
            max_nudges: 2,
            temperament: Temperament::Brawler,
        }
    }

    #[test]
    fn a_free_spot_is_the_wheel_front_plus_the_offset() {
        assert_eq!(landing_x(500.0, &mount(32), &[300.0], 0.0, 2000.0), 532.0);
        assert_eq!(landing_x(500.0, &mount(72), &[300.0], 0.0, 2000.0), 572.0);
    }

    #[test]
    fn a_hero_on_the_spot_nudges_it_right_at_most_twice() {
        // One hero on the spot: one nudge.
        assert_eq!(landing_x(500.0, &mount(32), &[532.0], 0.0, 2000.0), 564.0);
        // Heroes on the spot and on the first nudge: two nudges.
        assert_eq!(
            landing_x(500.0, &mount(32), &[532.0, 564.0], 0.0, 2000.0),
            596.0
        );
        // A hero just clear of the spot (24 px apart) does not nudge it.
        assert_eq!(landing_x(500.0, &mount(32), &[556.0], 0.0, 2000.0), 532.0);
        // After two nudges the rider lands anyway, even on a hero.
        assert_eq!(
            landing_x(500.0, &mount(32), &[532.0, 564.0, 596.0], 0.0, 2000.0),
            596.0
        );
    }

    #[test]
    fn the_spot_is_clamped_into_the_lane_and_the_view() {
        assert_eq!(landing_x(500.0, &mount(72), &[], 0.0, 560.0), 560.0);
        assert_eq!(landing_x(500.0, &mount(32), &[], 600.0, 900.0), 600.0);
        // An empty window takes its right bound.
        assert_eq!(landing_x(500.0, &mount(32), &[], 700.0, 650.0), 650.0);
    }
}
