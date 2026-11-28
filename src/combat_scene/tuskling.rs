//! BOSB: the Tusklings of the Ball of Swine as ENEMY-SLOT enemies
//! (`docs/01-design/floor-01-bos-b-tusklings.md`). The domain is [`crate::tuskling`]; this
//! module is the adapter (one [`SlotEnemy`]) and the **detach rule** that spawns them into the
//! slot: the qualifying impacts, the caps, the budget, the legal spawn, the kick-stagger and the
//! cower after the Ball's defeat.
//!
//! Adopted defaults of this step (`BOSB-AD*`, the numbers are data in `ball_boss_f01.json`):
//!
//! * AD2: a **qualifying impact** is a `BossCrash` event, or a `RackLanded` that crushed the Ball
//!   and left it alive (the pin). Nothing else detaches;
//! * AD3, AD4: 2 detach on the first, 1 on each later one; at most 3 live and 5 per attempt. A
//!   detach that would break either cap is **skipped and spends no budget**; the budget and the
//!   count of impacts reset on a restart (the whole yard is cleared with the slot);
//! * AD5: it spawns on the Ball's flank on the side away from the nearest hero, with the data's
//!   clearance from every hero, solid and live Tuskling; no legal spot skips the spawn;
//! * AD8: a kicked Tuskling in flight that overlaps the Ball staggers it **once per launch**; the
//!   link is the launch itself (the kick's move instance, carried in the Tuskling's `Kicked`
//!   state and listed in [`TuskYard`]). A slot enemy has no goblin `LaunchedBody`, so the goblin
//!   contact path of RINGC-AD7 is not used. The contact deals 0 HP;
//! * AD9: on the Ball's defeat every live Tuskling cowers; they are never solids;
//! * AD11: the defeat writes the data's fact once per attempt.
//!
//! A state with no Tuskling yard (every room but the Ball's) never runs any of it.

use super::CombatState;
use crate::boss::{Boss, TusklingData};
use crate::combat::CombatEvent;
use crate::corridor::SceneSim;
use crate::prototype::Rect;
use crate::scene_enemy::{
    HitSource, SlotBlast, SlotContact, SlotDeathBody, SlotEnemy, SlotHit, SlotHitResult, SlotOut,
    SlotStepCtx,
};
use crate::tuskling::{State, TuskAct, TuskCtx, TuskHit, TuskStep, Tuskling, target_id};

/// The registry-style kind name (the slot enemy's `kind()`; no roster kind builds it).
pub const KIND: &str = "tuskling";

/// The species key of the death profile (BOSB-AD14).
pub const SPECIES: &str = "tuskling";

/// One Tuskling in the slot.
#[derive(Debug, Clone)]
pub struct TuskEnemy {
    tusk: Tuskling,
    id: u32,
}

impl TuskEnemy {
    pub fn new(tusk: Tuskling) -> Self {
        let id = target_id(tusk.index);
        Self { tusk, id }
    }

    /// The domain machine (the presenter and the fixtures read it).
    pub fn tusk(&self) -> &Tuskling {
        &self.tusk
    }

    fn emit(&self, out: &mut SlotOut, tick: u64, acts: &[TuskAct]) {
        out.events
            .extend(acts.iter().map(|&act| CombatEvent::Tuskling {
                tick,
                enemy: self.id,
                act,
            }));
    }
}

impl SlotEnemy for TuskEnemy {
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
        self.tusk.alive()
    }

    fn hp(&self) -> u16 {
        self.tusk.hp
    }

    fn hurt_rects(&self) -> Vec<Rect> {
        self.tusk.hurtbox().into_iter().collect()
    }

    /// The `down` frames hold as the body; the species profile (BOSB-AD14) has no art of its own.
    fn death_body(&self) -> Option<SlotDeathBody> {
        Some(SlotDeathBody {
            x: self.tusk.x,
            ground_y: self.tusk.foot_y,
            facing: self.tusk.facing,
            species: SPECIES,
        })
    }

    fn step(&mut self, ctx: &SlotStepCtx<'_>, out: &mut SlotOut) {
        if !self.alive() {
            return;
        }
        let tctx = TuskCtx {
            tick: ctx.tick,
            heroes: &ctx.heroes,
            solids: ctx.solids,
            walls: ctx.room.walls,
            last_tell_start: ctx.last_tell_start,
        };
        let mut step = TuskStep::default();
        self.tusk.step(&tctx, &mut step);
        self.emit(out, ctx.tick, &step.acts);
        out.tell_started |= step.tell_started;
        out.blows.extend(step.blows);
    }

    fn apply_hit(&mut self, hit: &SlotHit, tick: u64, out: &mut SlotOut) -> Option<SlotHitResult> {
        let launch = match hit.source {
            HitSource::CarlMelee(_) if hit.spec.reaction.launch => {
                Some((hit.instance, hit.spec.reaction.knock_vx))
            }
            _ => None,
        };
        let th = TuskHit {
            damage: hit.damage,
            dir: hit.facing,
            launch,
        };
        let mut step = TuskStep::default();
        let outcome = self.tusk.hit(&th, &mut step)?;
        self.emit(out, tick, &step.acts);
        if outcome.defeated {
            out.events.push(hit.defeat_event(tick, self.id));
        }
        Some(SlotHitResult {
            hp_left: outcome.hp_left,
            defeated: outcome.defeated,
        })
    }

    /// A blast deals the profile's boss damage, like the troglodytes (TROG-AD15).
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
        let th = TuskHit {
            damage,
            dir: blast.away,
            launch: None,
        };
        let mut step = TuskStep::default();
        let outcome = self.tusk.hit(&th, &mut step)?;
        self.emit(out, tick, &step.acts);
        if outcome.defeated {
            out.events.push(CombatEvent::enemy_defeated(
                tick,
                self.id,
                blast.cause(),
                blast.charge.owner,
                None,
            ));
        }
        Some(SlotHitResult {
            hp_left: outcome.hp_left,
            defeated: outcome.defeated,
        })
    }

    /// The goblin flier contact is not a Tuskling reaction (BOSB-AD8 uses the kick itself).
    fn apply_contact(
        &mut self,
        _contact: &SlotContact,
        _tick: u64,
        _out: &mut SlotOut,
    ) -> Option<SlotHitResult> {
        None
    }

    fn on_boss_defeated(&mut self, tick: u64, out: &mut SlotOut) {
        let mut step = TuskStep::default();
        self.tusk.cower(&mut step);
        self.emit(out, tick, &step.acts);
    }

    fn mix_into(&self, mix: &mut dyn FnMut(u64)) {
        mix(0x5455_534b_0002);
        mix(u64::from(self.id));
        self.tusk.mix_into(mix);
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

/// The detach bookkeeping of one attempt. Empty (and unhashed) until the first qualifying
/// impact, so a Ball fight that has not crashed hashes as it did.
#[derive(Debug, Clone, Default)]
pub(super) struct TuskYard {
    /// Qualifying impacts so far.
    impacts: u8,
    /// Tusklings spawned so far (the budget spent, and the next spawn index).
    spent: u8,
    /// Launches that already staggered (or touched) the Ball: once per launch.
    kick_done: Vec<u32>,
    /// The boss' defeat was seen (the cower and the fact fired).
    defeat_seen: bool,
}

impl TuskYard {
    pub(super) fn mix_into(&self, mix: &mut dyn FnMut(u64)) {
        if self.impacts == 0 && self.kick_done.is_empty() && !self.defeat_seen {
            return;
        }
        mix(0x5455_534b_0001);
        mix(u64::from(self.impacts));
        mix(u64::from(self.spent));
        mix(self.kick_done.len() as u64);
        for launch in &self.kick_done {
            mix(u64::from(*launch));
        }
        mix(u64::from(self.defeat_seen));
    }
}

/// The reason codes of [`TuskAct::Skipped`].
pub const SKIP_LIVE_CAP: u8 = 0;
pub const SKIP_BUDGET: u8 = 1;
pub const SKIP_NO_SPOT: u8 = 2;

fn grown(rect: Rect, by: f32) -> Rect {
    Rect {
        left: rect.left - by,
        bottom: rect.bottom,
        width: rect.width + 2.0 * by,
        height: rect.height,
    }
}

impl CombatState {
    /// The living Tusklings in the slot.
    pub fn tusklings_live(&self) -> usize {
        self.slot
            .enemies
            .iter()
            .filter(|e| e.kind() == KIND && e.alive())
            .count()
    }

    /// The Tusklings spawned this attempt (the budget spent).
    pub fn tusklings_spent(&self) -> u8 {
        self.tusk.spent
    }

    /// Qualifying impacts so far this attempt.
    pub fn tusk_impacts(&self) -> u8 {
        self.tusk.impacts
    }

    /// The boss events of one tick, in order: every qualifying impact detaches (BOSB-AD2, AD3).
    /// `events` are already logged, so the spawns follow the crash in the log.
    pub(super) fn tusk_on_boss_events(
        &mut self,
        scene: &SceneSim,
        boss: &Boss,
        events: &[CombatEvent],
        tick: u64,
    ) {
        let Some(data) = boss.data().tusklings.as_ref() else {
            return;
        };
        for event in events {
            let qualifies = match event {
                CombatEvent::BossCrash { .. } => true,
                CombatEvent::RackLanded { crushed, .. } => *crushed && boss.alive(),
                _ => false,
            };
            if !qualifies {
                continue;
            }
            self.tusk.impacts = self.tusk.impacts.saturating_add(1);
            let count = if self.tusk.impacts == 1 {
                data.first_detach
            } else {
                data.later_detach
            };
            for _ in 0..count {
                self.tusk_spawn_one(scene, boss, data, tick);
            }
        }
    }

    fn tusk_skip(&mut self, reason: u8, tick: u64) {
        self.log(CombatEvent::Tuskling {
            tick,
            enemy: 0,
            act: TuskAct::Skipped { reason },
        });
    }

    /// One detach (BOSB-AD4, AD5): the live cap, then the budget, then a legal spot. A skip
    /// spends nothing.
    fn tusk_spawn_one(
        &mut self,
        scene: &SceneSim,
        boss: &Boss,
        data: &'static TusklingData,
        tick: u64,
    ) {
        if self.tusklings_live() >= usize::from(data.max_live) {
            self.tusk_skip(SKIP_LIVE_CAP, tick);
            return;
        }
        if self.tusk.spent >= data.budget {
            self.tusk_skip(SKIP_BUDGET, tick);
            return;
        }
        let heroes = self.slot_hero_views(scene);
        let nearest = heroes
            .iter()
            .filter(|h| h.present)
            .min_by(|a, b| {
                (a.x - boss.x)
                    .abs()
                    .total_cmp(&(b.x - boss.x).abs())
                    .then(a.id.index().cmp(&b.id.index()))
            })
            .copied();
        // The flank away from the nearest hero; with nobody in play, away from the arch the
        // ball just left (toward the centre).
        let side: f32 = match nearest {
            Some(h) if h.x >= boss.x => -1.0,
            Some(_) => 1.0,
            None if boss.x >= boss.data().arena.centre_x => -1.0,
            None => 1.0,
        };
        let half = data.body_half_width;
        let ball_half = boss.data().body.standing.x[1];
        let others: Vec<Rect> = self
            .slot
            .enemies
            .iter()
            .filter(|e| e.kind() == KIND && e.alive())
            .filter_map(|e| e.hurt_rects().first().copied())
            .collect();
        let spot = (0..data.spawn_tries).find_map(|k| {
            let x = boss.x
                + side * (ball_half + half + data.flank_gap_px + f32::from(k) * data.spawn_step_px);
            let body = Rect {
                left: x - half,
                bottom: boss.floor_y() + 0.5,
                width: half * 2.0,
                height: data.hurt_box.y[1] - 1.0,
            };
            let room = (0.0, self.x_max);
            let clear = grown(body, data.clearance_px);
            let legal = x - half >= room.0
                && x + half <= room.1
                && !crate::combat::solid_at(&scene.world.solids, clear)
                && heroes
                    .iter()
                    .filter(|h| h.present)
                    .all(|h| !clear.overlaps(h.hurt))
                && others.iter().all(|r| !clear.overlaps(*r));
            legal.then_some(x)
        });
        let Some(x) = spot else {
            self.tusk_skip(SKIP_NO_SPOT, tick);
            return;
        };
        let index = self.tusk.spent;
        let facing = match nearest {
            Some(h) if h.x >= x => 1,
            Some(_) => -1,
            None => -(side as i8),
        };
        let tusk = Tuskling::new(data, index, x, boss.floor_y(), facing);
        self.slot.enemies.push(Box::new(TuskEnemy::new(tusk)));
        self.tusk.spent += 1;
        self.log(CombatEvent::Tuskling {
            tick,
            enemy: target_id(index),
            act: TuskAct::Spawned {
                x: x.round() as i16,
            },
        });
    }

    /// BOSB-AD8: a kicked Tuskling in flight that overlaps the Ball staggers it, **once per
    /// launch** and for 0 HP. Run before the boss' own step, so the stagger is read this tick.
    pub(super) fn tusk_kick_contacts(&mut self, boss: &mut Boss, tick: u64) {
        let Some(data) = boss.data().tusklings.as_ref() else {
            return;
        };
        let Some(body) = boss.hurtbox() else {
            return;
        };
        let flights: Vec<(u32, u32)> = self
            .slot
            .enemies
            .iter()
            .filter_map(|e| {
                let tusk = e.as_any()?.downcast_ref::<TuskEnemy>()?;
                let (launch, rect) = tusk.tusk().kick_flight()?;
                (rect.overlaps(body) && !self.tusk.kick_done.contains(&launch))
                    .then_some((launch, tusk.id))
            })
            .collect();
        for (launch, id) in flights {
            if self.tusk.kick_done.contains(&launch) {
                continue;
            }
            self.tusk.kick_done.push(launch);
            let staggered = boss.kick_stagger(data.kick_stagger_ticks);
            self.log(CombatEvent::Tuskling {
                tick,
                enemy: id,
                act: TuskAct::KickContact { launch, staggered },
            });
        }
    }

    /// BOSB-AD9, AD11: the boss' defeat, seen once. Every live Tuskling cowers, and the
    /// boss' fact is written. The exit opens as it always did (there is no forced cleanup).
    pub(super) fn tusk_after_boss_step(&mut self, scene: &mut SceneSim, boss: &Boss, tick: u64) {
        let data = boss.data();
        if (data.tusklings.is_none() && data.ledger_fact.is_empty())
            || boss.alive()
            || self.tusk.defeat_seen
        {
            return;
        }
        self.tusk.defeat_seen = true;
        for index in 0..self.slot.enemies.len() {
            let mut out = SlotOut::default();
            self.slot.enemies[index].on_boss_defeated(tick, &mut out);
            self.apply_slot_out(scene, out, tick);
        }
        if !data.ledger_fact.is_empty() {
            self.log(CombatEvent::LedgerFact {
                tick,
                fact: data.ledger_fact.as_str(),
            });
        }
    }

    /// Whether `state` is a Tuskling state a fixture can wait for.
    pub fn tusk_in(&self, index: u8, state: State) -> bool {
        self.slot.enemies.iter().any(|e| {
            e.as_any()
                .and_then(|a| a.downcast_ref::<TuskEnemy>())
                .is_some_and(|t| t.tusk().index == index && t.tusk().state == state)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ball_fixtures::{BALL01_DUO, BALL01_FULL_CLEAR_SOLO, BALL01_SOLO, Rig};
    use crate::boss::{BossHit, BossMode};
    use crate::input::RawInput;
    use crate::party::HeroId;
    use std::path::Path;

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    const IDLE: RawInput = RawInput::NONE;
    const CRASH: CombatEvent = CombatEvent::BossCrash { tick: 1, wall: -1 };

    /// A two-seat room with the gate closed, the heroes at `carl` and `donut` and the ball at the
    /// left crash pivot (216, room on its right flank for the spawns).
    fn room(carl: f32, donut: f32) -> Rig {
        let mut rig = Rig::new(root(), &BALL01_DUO).unwrap();
        for hero in [HeroId::Carl, HeroId::Donut] {
            rig.place(hero, 150.0);
        }
        assert!(rig.walk_in(200), "the gate closes");
        rig.place(HeroId::Carl, carl);
        rig.place(HeroId::Donut, donut);
        let boss = rig.boss_mut();
        boss.x = 216.0;
        boss.facing = -1;
        rig
    }

    /// Offer `events` to the detach rule as if the boss had logged them.
    fn offer(rig: &mut Rig, events: &[CombatEvent]) {
        let boss = rig.sim.combat.boss.take().unwrap();
        let tick = rig.sim.scene.world.tick;
        rig.sim
            .combat
            .tusk_on_boss_events(&rig.sim.scene, &boss, events, tick);
        rig.sim.combat.boss = Some(boss);
    }

    fn acts(rig: &Rig) -> Vec<TuskAct> {
        rig.sim
            .combat
            .events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::Tuskling { act, .. } => Some(*act),
                _ => None,
            })
            .collect()
    }

    fn spawned(rig: &Rig) -> usize {
        acts(rig)
            .iter()
            .filter(|a| matches!(a, TuskAct::Spawned { .. }))
            .count()
    }

    fn lay_down(rig: &mut Rig, index: u8) {
        for e in &mut rig.sim.combat.slot.enemies {
            if let Some(t) = e.as_any().and_then(|a| a.downcast_ref::<TuskEnemy>())
                && t.tusk().index == index
            {
                let mut down = t.tusk().clone();
                down.hit(
                    &TuskHit {
                        damage: 9,
                        dir: 1,
                        launch: None,
                    },
                    &mut TuskStep::default(),
                );
                *e = Box::new(TuskEnemy::new(down));
            }
        }
    }

    #[test]
    fn the_first_qualifying_impact_detaches_two_and_each_later_one_detaches_one() {
        let mut rig = room(60.0, 80.0);
        offer(&mut rig, &[CRASH]);
        assert_eq!((spawned(&rig), rig.sim.combat.tusklings_live()), (2, 2));
        lay_down(&mut rig, 0);
        offer(&mut rig, &[CRASH]);
        assert_eq!(spawned(&rig), 3, "one more on the second impact");
        offer(
            &mut rig,
            &[CombatEvent::RackLanded {
                tick: 2,
                rack: 0,
                crushed: true,
            }],
        );
        assert_eq!(spawned(&rig), 4, "the pin is an impact too: one more");
        assert_eq!(rig.sim.combat.tusk_impacts(), 3);
    }

    #[test]
    fn only_a_crash_or_a_pin_detaches_a_bump_a_miss_or_a_defeat_does_not() {
        let mut rig = room(60.0, 80.0);
        let rack = |crushed| CombatEvent::RackLanded {
            tick: 2,
            rack: 0,
            crushed,
        };
        offer(&mut rig, &[rack(false)]);
        offer(
            &mut rig,
            &[CombatEvent::BossStrike {
                tick: 3,
                mv: crate::boss::BossMove::Pound,
            }],
        );
        offer(
            &mut rig,
            &[CombatEvent::BossHurt {
                tick: 3,
                source: 0,
                damage: 1,
                hp_left: 20,
            }],
        );
        assert_eq!((spawned(&rig), rig.sim.combat.tusk_impacts()), (0, 0));
        offer(&mut rig, &[rack(true)]);
        assert_eq!(spawned(&rig), 2, "the pin counts as the first impact");
    }

    #[test]
    fn at_most_three_live_and_five_per_attempt_and_a_skipped_detach_spends_no_budget() {
        let mut rig = room(60.0, 80.0);
        offer(&mut rig, &[CRASH]); // 2 live, spent 2
        offer(&mut rig, &[CRASH]); // 3 live, spent 3
        assert_eq!(
            (
                rig.sim.combat.tusklings_live(),
                rig.sim.combat.tusklings_spent()
            ),
            (3, 3)
        );
        offer(&mut rig, &[CRASH]); // the live cap
        assert_eq!(
            (
                rig.sim.combat.tusklings_live(),
                rig.sim.combat.tusklings_spent()
            ),
            (3, 3),
            "the skipped detach spent nothing"
        );
        assert_eq!(
            acts(&rig).last(),
            Some(&TuskAct::Skipped {
                reason: SKIP_LIVE_CAP
            })
        );
        lay_down(&mut rig, 0);
        lay_down(&mut rig, 1);
        offer(&mut rig, &[CRASH]); // 2 live -> 3, spent 4
        offer(&mut rig, &[CRASH]); // 2 live -> 3, spent 5
        assert_eq!(rig.sim.combat.tusklings_spent(), 5);
        lay_down(&mut rig, 2);
        lay_down(&mut rig, 3);
        offer(&mut rig, &[CRASH]); // the budget, though only 1 is live
        assert_eq!(rig.sim.combat.tusklings_spent(), 5);
        assert_eq!(rig.sim.combat.tusklings_live(), 1);
        assert_eq!(
            acts(&rig).last(),
            Some(&TuskAct::Skipped {
                reason: SKIP_BUDGET
            })
        );
    }

    #[test]
    fn a_spawn_keeps_the_clearance_and_no_legal_spot_skips_it_without_spending() {
        // Both heroes left of the ball, so its flank away from them is the right one.
        let mut rig = room(100.0, 60.0);
        offer(&mut rig, &[CRASH]);
        let live: Vec<_> = rig
            .sim
            .combat
            .slot
            .enemies
            .iter()
            .filter_map(|e| e.hurt_rects().first().copied())
            .collect();
        assert_eq!(live.len(), 2);
        let data = rig.boss().data().tusklings.as_ref().unwrap();
        let carl = rig.sim.combat.slot_hero_views(&rig.sim.scene)[0].hurt;
        for r in &live {
            assert!(r.left >= rig.boss().x + 60.0, "on the far flank: {r:?}");
            let gap = (r.left - carl.right()).max(carl.left - r.right());
            assert!(gap >= data.clearance_px, "clearance {gap}");
        }
        let gap = (live[1].left - live[0].right()).max(live[0].left - live[1].right());
        assert!(gap >= data.clearance_px, "between two Tusklings {gap}");
        // At the right wall (the ball at 448) with Donut on the flank, no candidate is legal: the
        // detach is skipped and nothing is spent.
        let mut rig = room(380.0, 530.0);
        rig.boss_mut().x = 448.0;
        offer(&mut rig, &[CRASH]);
        assert_eq!(rig.sim.combat.tusklings_spent(), 0);
        assert!(acts(&rig).iter().all(|a| *a
            == TuskAct::Skipped {
                reason: SKIP_NO_SPOT
            }));
        assert_eq!(acts(&rig).len(), 2, "both detaches skipped");
    }

    #[test]
    fn a_restart_clears_the_impacts_the_budget_and_the_tusklings() {
        let mut rig = room(60.0, 80.0);
        offer(&mut rig, &[CRASH]);
        assert!(rig.sim.combat.tusklings_spent() > 0);
        let scene = &mut rig.sim.scene;
        rig.sim
            .combat
            .restart_with(scene, super::super::RestartKind::DefeatRetry);
        assert_eq!(
            (
                rig.sim.combat.tusklings_live(),
                rig.sim.combat.tusklings_spent(),
                rig.sim.combat.tusk_impacts()
            ),
            (0, 0, 0)
        );
        assert!(rig.sim.combat.slot_enemies().is_empty());
        // And the first impact of the new attempt detaches two again (the restart put the ball
        // back at its spawn: stand it at the left crash pivot again).
        rig.boss_mut().x = 216.0;
        offer(&mut rig, &[CRASH]);
        assert_eq!(rig.sim.combat.tusklings_live(), 2);
    }

    #[test]
    fn the_first_real_crash_detaches_two_costs_the_ball_three_and_writes_one_event() {
        let mut rig = Rig::new(root(), &BALL01_FULL_CLEAR_SOLO).unwrap();
        for _ in 0..400 {
            rig.step_scripted();
            if !rig.tally.crashes.is_empty() {
                break;
            }
        }
        assert_eq!(rig.tally.crashes.len(), 1);
        let events = &rig.sim.combat.events;
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(
                    e,
                    CombatEvent::BossSelfHurt {
                        damage: 3,
                        hp_left: 21,
                        ..
                    }
                ))
                .count(),
            1,
            "BOSB-AD12: the crash costs 3"
        );
        assert_eq!(rig.boss().hp, 21);
        assert_eq!(
            spawned(&rig),
            2,
            "the signature image: two at the first crash"
        );
        // BossHurt stays the heroes' and the weight's damage.
        assert_eq!(rig.tally.boss_damage, 0);
    }

    #[test]
    fn a_crash_never_takes_the_last_hit_point() {
        let mut rig = Rig::new(root(), &BALL01_FULL_CLEAR_SOLO).unwrap();
        rig.boss_mut().hp = 2;
        for _ in 0..400 {
            rig.step_scripted();
            if !rig.tally.crashes.is_empty() {
                break;
            }
        }
        assert_eq!(rig.tally.crashes.len(), 1);
        assert_eq!(rig.boss().hp, 1, "3 is clamped to leave 1");
        assert!(rig.boss().alive());
    }

    /// A kicked Tuskling beside the ball's right flank, flying into it.
    fn kicked_into_the_ball(rig: &mut Rig, launch: u32) {
        let (bx, floor) = (rig.boss().x, rig.boss().floor_y());
        let data = rig.boss().data().tusklings.as_ref().unwrap();
        let mut tusk = Tuskling::new(data, 0, bx + 60.0 + 12.0 + 14.0, floor, -1);
        tusk.hit(
            &TuskHit {
                damage: 1,
                dir: -1,
                launch: Some((launch, 240.0)),
            },
            &mut TuskStep::default(),
        );
        rig.sim
            .combat
            .slot
            .enemies
            .push(Box::new(TuskEnemy::new(tusk)));
    }

    #[test]
    fn a_kicked_tuskling_staggers_the_ball_once_per_launch_for_zero_hp() {
        let mut rig = room(60.0, 80.0);
        let hp = rig.boss().hp;
        assert_eq!(rig.boss().mode, BossMode::Neutral);
        kicked_into_the_ball(&mut rig, 41);
        let mut contacts = 0;
        for _ in 0..30 {
            rig.step([IDLE, IDLE]);
            contacts = acts(&rig)
                .iter()
                .filter(|a| matches!(a, TuskAct::KickContact { .. }))
                .count();
        }
        assert_eq!(contacts, 1, "once per launch");
        assert!(acts(&rig).contains(&TuskAct::KickContact {
            launch: 41,
            staggered: true
        }));
        assert_eq!(rig.boss().hp, hp, "enemy to enemy damage stays 0");
        assert_eq!(rig.tally.boss_damage, 0);
        // The stagger held the ball for the data's ticks, then it acted again.
        let held = rig.boss().stagger_left();
        assert!(held < 20, "the stagger runs down: {held}");
        for _ in 0..40 {
            rig.step([IDLE, IDLE]);
        }
        assert_eq!(rig.boss().stagger_left(), 0);
        // The same launch never staggers again; a new launch may.
        rig.sim.combat.tusk.kick_done.retain(|l| *l == 41);
        kicked_into_the_ball(&mut rig, 41);
        for _ in 0..30 {
            rig.step([IDLE, IDLE]);
        }
        let n = acts(&rig)
            .iter()
            .filter(|a| matches!(a, TuskAct::KickContact { .. }))
            .count();
        assert_eq!(n, 1, "launch 41 is spent");
    }

    #[test]
    fn a_staggered_ball_stands_still_for_twenty_ticks_and_nothing_is_cancelled() {
        let mut rig = room(60.0, 80.0);
        let x0 = rig.boss().x;
        assert!(rig.boss_mut().kick_stagger(20));
        assert!(!rig.boss_mut().kick_stagger(20), "no stacking");
        for _ in 0..20 {
            rig.step([IDLE, IDLE]);
            assert_eq!(rig.boss().x, x0, "it stands");
            assert_eq!(rig.boss().mode, BossMode::Neutral);
        }
        rig.step([IDLE, IDLE]);
        rig.step([IDLE, IDLE]);
        assert_ne!(rig.boss().x, x0, "then it walks again");
        // A roll tell is delayed, not cancelled: its clock stands still for the 20 ticks.
        let mut busy = room(320.0, 80.0);
        for _ in 0..200 {
            busy.step([IDLE, IDLE]);
            if busy.boss().mode == BossMode::ChargeTell {
                let at = busy.boss().mode_tick;
                assert!(busy.boss_mut().kick_stagger(20));
                for _ in 0..20 {
                    busy.step([IDLE, IDLE]);
                    assert_eq!(
                        (busy.boss().mode, busy.boss().mode_tick),
                        (BossMode::ChargeTell, at)
                    );
                }
                busy.step([IDLE, IDLE]);
                assert_eq!(busy.boss().mode_tick, at + 1, "then it runs on");
                return;
            }
        }
        panic!("the ball never told a roll");
    }

    #[test]
    fn the_defeat_makes_the_tusklings_cower_writes_the_fact_once_and_leaves_the_exit_open() {
        let mut rig = room(60.0, 80.0);
        // Two live Tusklings, one of them in the heroes' way to the exit.
        offer(&mut rig, &[CRASH]);
        let floor = rig.boss().floor_y();
        let data = rig.boss().data().tusklings.as_ref().unwrap();
        rig.sim
            .combat
            .slot
            .enemies
            .push(Box::new(TuskEnemy::new(Tuskling::new(
                data, 4, 580.0, floor, -1,
            ))));
        // The hit that defeats the ball (the path of a hero's blow).
        let mut events = Vec::new();
        let tick = rig.sim.scene.world.tick;
        let hit = BossHit {
            damage: 99,
            poise: 0,
            player: 0,
            hit_stop: 0,
        };
        assert!(
            rig.boss_mut()
                .apply_hit(hit, &mut events, tick)
                .unwrap()
                .defeated
        );
        for _ in 0..4 {
            rig.step([IDLE, IDLE]);
        }
        let live_states: Vec<_> = rig
            .sim
            .combat
            .slot
            .enemies
            .iter()
            .filter_map(|e| e.as_any()?.downcast_ref::<TuskEnemy>())
            .map(|t| t.tusk().state)
            .collect();
        assert_eq!(live_states.len(), 3);
        assert!(
            live_states.iter().all(|s| *s == State::Cower),
            "{live_states:?}"
        );
        // The fact: once, whatever the ticks after.
        for _ in 0..300 {
            rig.step([IDLE, IDLE]);
        }
        let facts = rig
            .sim
            .combat
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    CombatEvent::LedgerFact {
                        fact: "f01_ball_defeated",
                        ..
                    }
                )
            })
            .count();
        assert_eq!(facts, 1, "BOSB-AD11: written once");
        assert!(rig.boss().released(), "the exit opens as it always did");
        // Harmless and non-blocking: no hurt from them, and Carl walks through to the exit.
        let hp_before = rig.health_of(HeroId::Carl);
        let go = RawInput {
            move_x: 1.0,
            ..RawInput::NONE
        };
        for _ in 0..400 {
            rig.step([go, IDLE]);
        }
        assert!(
            rig.x_of(HeroId::Carl) >= 590.0,
            "reached the exit: {}",
            rig.x_of(HeroId::Carl)
        );
        assert_eq!(rig.health_of(HeroId::Carl), hp_before);
        assert_eq!(
            rig.sim
                .combat
                .events
                .iter()
                .filter(|e| matches!(e, CombatEvent::LedgerFact { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn the_ledger_fact_is_per_attempt_and_the_juicer_writes_none() {
        assert_eq!(crate::boss::ball().ledger_fact, "f01_ball_defeated");
        assert!(crate::boss::builtin().ledger_fact.is_empty());
    }

    #[test]
    fn the_follower_bot_flees_to_the_nearest_alcove_when_the_roll_is_told_and_is_untouched() {
        // Solo: the player is Carl, parked in the left alcove; the bot Donut stands in the lane.
        let mut rig = Rig::new(root(), &BALL01_SOLO).unwrap();
        for hero in [HeroId::Carl, HeroId::Donut] {
            rig.place(hero, 150.0);
        }
        assert!(rig.walk_in(200));
        rig.place(HeroId::Carl, 80.0);
        rig.place(HeroId::Donut, 300.0);
        {
            let boss = rig.boss_mut();
            boss.x = 448.0;
            boss.facing = -1;
        }
        let mut crashed = false;
        let mut bot_in_alcove_at_crash = false;
        for _ in 0..400 {
            rig.step([IDLE, IDLE]);
            if !rig.tally.crashes.is_empty() {
                crashed = true;
                bot_in_alcove_at_crash = rig.x_of(HeroId::Donut) <= 110.0;
                break;
            }
        }
        assert!(crashed, "the roll came");
        assert!(
            bot_in_alcove_at_crash,
            "the bot was in the alcove: {}",
            rig.x_of(HeroId::Donut)
        );
        assert_eq!(rig.tally.hero_hits, 0, "the roll hit nobody");
    }

    #[test]
    fn the_species_has_a_death_profile_row_and_the_audience_pays_a_tuskling_defeat() {
        use crate::audience::AudienceInput;
        use crate::combat::DefeatCause;
        use crate::combat::ForceTier;
        use crate::death::DeathProfiles;
        let own = DeathProfiles::builtin().select(SPECIES, DefeatCause::Melee, ForceTier::Routine);
        assert_eq!(
            own.finish, "tuskling_down",
            "no new art keys: the species' own NoArt finish"
        );
        let event = CombatEvent::enemy_defeated(5, target_id(0), DefeatCause::Melee, Some(0), None);
        assert!(
            matches!(
                AudienceInput::from_combat(&event),
                Some(AudienceInput::Defeat { target, .. }) if target == target_id(0)
            ),
            "a Tuskling defeat is an ordinary enemy defeat to the audience"
        );
    }
}
