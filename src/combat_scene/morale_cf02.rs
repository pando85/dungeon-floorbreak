//! CF02-G: goblin morale wiring (Q57..Q68).
//!
//! The pure rules live in [`crate::morale`] (nerve, reactions, back-off, taunt,
//! the Q58 draw, the Q66 pincer rule). This module only feeds them the tick's
//! authoritative events and applies what they decide to the goblins: the
//! [`MindDrive`] of the next tick, a panic that aborts a tell, the Coward's hop.
//!
//! **Order inside one tick** (the same every tick): Carl's strike, Donut, the
//! scene step, [`CombatState::step_goblins`] (which reads each goblin's drive and
//! ranks the attack tokens), llamas, globs, contacts, shore, boss, charges, and
//! last [`CombatState::step_morale`], which judges the tick's events (Q60) in
//! goblin index order. A reaction therefore starts on the tick of its cause and
//! changes the goblin's behaviour from the next tick on. Nothing here changes a
//! tell or a swing: the 24-tick tell is never shortened by a nerve reaction.
//!
//! Morale runs only for a run with `cf02_rules` and the morale switch (the
//! `--cf02` scenario, every G fixture); a `Passive` goblin (a QA dummy) has none.
//! Its state enters the state hash only then.

use super::CombatState;
use crate::cf02::{Force, SpectacularCause};
use crate::combat::{
    CombatEvent, DefeatCause, EnemyAi, ForceTier, Phase, ReactionKind, carl_move, donut_move,
};
use crate::corridor::SceneSim;
use crate::feel::{GOBLIN_TRIGGER, MAX_ATTACKERS, TELL_TICKS};
use crate::goblin::{GoblinMode, MindDrive};
use crate::morale::{
    self, Draw, Morale, MoraleEvent, Pos, Reaction, Started, StepInput, TokenCandidate,
};
use crate::party::HeroId;

/// One run's morale state (`None` on the [`CombatState`] without morale).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MoraleRuntime {
    /// The effective Q58 seed.
    pub seed: u64,
    /// Per goblin; a `Passive` goblin keeps its spawn state and is never stepped.
    pub morale: Vec<Morale>,
    /// Q58 draw 3: ticks left before the goblin first approaches.
    delay_left: Vec<u8>,
    /// Q64: the direction of the running hop back (+1 / -1).
    back_dir: Vec<i8>,
    /// Q63: the last goblin step could not flee (a solid or the edge).
    blocked: Vec<bool>,
    /// Which goblins were knocked down (`Launched` / `Downed`) on the last tick.
    knocked: Vec<bool>,
    /// Health at the end of the last tick (Q65: a hit during a taunt).
    health: Vec<u8>,
    /// Carl's move instance that has been active without hitting anything.
    carl_active: Option<u32>,
}

impl MoraleRuntime {
    /// The morale of goblin `index` (for the presentation and the tests).
    pub fn morale_of(&self, index: usize) -> Option<&Morale> {
        self.morale.get(index)
    }

    /// Fold the whole runtime into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(0x4d4f_5241_4c45);
        mix(self.seed);
        for (index, morale) in self.morale.iter().enumerate() {
            morale.mix_into(mix);
            mix(u64::from(self.delay_left[index]));
            mix(self.back_dir[index] as i64 as u64);
        }
    }
}

fn force_of(force: ForceTier) -> Force {
    match force {
        ForceTier::Routine => Force::Routine,
        ForceTier::Heavy => Force::Heavy,
        ForceTier::Catastrophic => Force::Catastrophic,
    }
}

fn spectacular_cause(cause: DefeatCause) -> Option<SpectacularCause> {
    match cause {
        DefeatCause::ShoreBlock { .. } | DefeatCause::ShoreBlockEdge { .. } => {
            Some(SpectacularCause::Shore)
        }
        DefeatCause::Explosion { .. } => Some(SpectacularCause::Explosion),
        DefeatCause::Melee | DefeatCause::DropHazard { .. } => None,
    }
}

fn hero_of(index: u32) -> HeroId {
    if index == 0 {
        HeroId::Carl
    } else {
        HeroId::Donut
    }
}

fn side(of: f32, from: f32) -> i8 {
    if of >= from { 1 } else { -1 }
}

impl CombatState {
    /// Whether this run has morale: the master switch and the morale switch.
    pub(super) fn morale_wanted(&self) -> bool {
        self.cf02.is_some_and(|t| t.cf02_rules) && self.morale_on
    }

    /// The effective Q58 seed: the `--cf02-seed` override or the fixture seed.
    fn morale_seed(&self) -> u64 {
        self.cf02_seed.unwrap_or(self.fixture.seed)
    }

    /// Q57, Q58: (re)build every goblin's morale for the attempt `self.restarts`.
    pub(super) fn spawn_morale(&mut self) {
        self.morale_feed.clear();
        let Some(t) = self.cf02.filter(|_| self.morale_wanted()) else {
            self.morale = None;
            return;
        };
        let seed = self.morale_seed();
        let draws: Vec<Draw> = (0..self.enemies.len())
            .map(|i| morale::draw_goblin(t, seed, self.restarts, i as u32))
            .collect();
        self.morale = Some(MoraleRuntime {
            seed,
            morale: draws.iter().map(|d| Morale::spawn(t, d)).collect(),
            delay_left: draws.iter().map(|d| d.approach_delay).collect(),
            back_dir: vec![0; draws.len()],
            blocked: vec![false; draws.len()],
            knocked: vec![false; draws.len()],
            health: self.enemies.iter().map(|g| g.health).collect(),
            carl_active: None,
        });
    }

    /// A checkpoint respawn rebuilt goblin `index`: it starts the attempt's draw
    /// again (Q57: a goblin that is not respawned keeps its morale).
    pub(super) fn respawn_morale_of(&mut self, index: usize) {
        let Some(t) = self.cf02.filter(|_| self.morale.is_some()) else {
            return;
        };
        let draw = morale::draw_goblin(t, self.morale_seed(), self.restarts, index as u32);
        let goblin_health = self.enemies[index].health;
        if let Some(m) = self.morale.as_mut() {
            m.morale[index] = Morale::spawn(t, &draw);
            m.delay_left[index] = draw.approach_delay;
            m.back_dir[index] = 0;
            m.blocked[index] = false;
            m.knocked[index] = false;
            m.health[index] = goblin_health;
        }
    }

    /// The morale of goblin `index` (`None` without morale or out of range).
    pub fn goblin_morale(&self, index: usize) -> Option<&Morale> {
        self.morale.as_ref()?.morale_of(index)
    }

    /// The reaction goblin `index` is in (`Neutral` without morale): what the
    /// CF02-I poses read.
    pub fn goblin_reaction(&self, index: usize) -> Reaction {
        self.goblin_morale(index)
            .map_or(Reaction::Neutral, Morale::reaction)
    }

    /// What morale asks of goblin `index` for this tick's step.
    pub(super) fn mind_drive(&self, index: usize) -> MindDrive {
        let (Some(t), Some(m)) = (self.cf02, self.morale.as_ref()) else {
            return MindDrive::LEGACY;
        };
        if self.enemies[index].ai == EnemyAi::Passive {
            return MindDrive::LEGACY;
        }
        let morale = &m.morale[index];
        let legacy = MindDrive::LEGACY;
        let flee = match morale.reaction() {
            Reaction::Panic { flee_dir, .. } => {
                Some((flee_dir, morale.walk_speed_px_s(t, legacy.walk_speed)))
            }
            _ => None,
        };
        MindDrive {
            walk_speed: morale.walk_speed_px_s(t, legacy.walk_speed),
            hold: m.delay_left[index] > 0
                || matches!(
                    morale.reaction(),
                    Reaction::Hesitate { .. } | Reaction::Taunt { .. } | Reaction::BackOff { .. }
                ),
            flee,
            attack_cooldown: morale.attack_cooldown(t, legacy.attack_cooldown),
        }
    }

    /// Q68, Q66: the goblins that may close in and start a tell this tick. A goblin
    /// in a reaction other than enraged takes no token; the second token prefers
    /// the other side of the first holder's target (a pincer); otherwise the
    /// nearest first, ties to the lower index.
    pub(super) fn approved_goblins(&self, target_x: &[f32]) -> Vec<usize> {
        let active: Vec<usize> = (0..self.enemies.len())
            .filter(|&i| self.enemies[i].token_held())
            .collect();
        let slots = MAX_ATTACKERS.saturating_sub(active.len());
        let mut ranked: Vec<(f32, usize)> = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(i, g)| g.wants_attack() && self.reaction_takes_token(*i))
            .map(|(index, g)| ((g.x - target_x[index]).abs(), index))
            .collect();
        ranked.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let order: Vec<usize> = ranked.iter().map(|r| r.1).collect();
        let (Some(t), true) = (self.cf02, self.morale.is_some()) else {
            return order.into_iter().take(slots).collect();
        };
        let mut approved: Vec<usize> = Vec::new();
        let mut left = slots;
        let mut holder = active.first().copied();
        if holder.is_none()
            && left > 0
            && let Some(&first) = order.first()
        {
            approved.push(first);
            holder = Some(first);
            left -= 1;
        }
        if left > 0
            && let Some(holder) = holder
        {
            let anchor = target_x[holder];
            let candidates: Vec<TokenCandidate> = order
                .iter()
                .filter(|i| !approved.contains(i) && **i != holder)
                .map(|&i| TokenCandidate {
                    index: i,
                    side: side(self.enemies[i].x, anchor),
                    distance_px: (self.enemies[i].x - anchor).abs(),
                })
                .collect();
            let holder_side = side(self.enemies[holder].x, anchor);
            if let Some(pick) =
                morale::pincer_preferred(t, holder_side, GOBLIN_TRIGGER, &candidates)
            {
                approved.push(pick);
                left -= 1;
            }
        }
        for &i in &order {
            if left == 0 {
                break;
            }
            if !approved.contains(&i) {
                approved.push(i);
                left -= 1;
            }
        }
        approved
    }

    /// Q68: a goblin in a non-fighting reaction cannot take a token; neither can one
    /// still in its first-approach delay (Q58 draw 3), which would hold the slot
    /// without being able to tell.
    fn reaction_takes_token(&self, index: usize) -> bool {
        self.morale.as_ref().is_none_or(|m| {
            self.enemies[index].ai == EnemyAi::Passive
                || (m.morale[index].reaction().can_take_token() && m.delay_left[index] == 0)
        })
    }

    /// Remember whether goblin `index`'s last step was blocked while fleeing (Q63).
    pub(super) fn note_flee_blocked(&mut self, index: usize, blocked: bool) {
        if let Some(m) = self.morale.as_mut() {
            m.blocked[index] = blocked;
        }
    }

    /// Q57..Q65, Q68: judge the tick's events and run every goblin's morale.
    pub(super) fn step_morale(&mut self, scene: &SceneSim, tick: u64, max_x: f32) {
        let Some(t) = self.cf02 else {
            return;
        };
        if self.morale.is_none() {
            return;
        }
        let feed = std::mem::take(&mut self.morale_feed);
        let n = self.enemies.len();
        let pos_of = |c: &Self, i: usize| Pos {
            x: c.enemies[i].x,
            y: c.enemies[i].y,
        };
        let hero_pos = |id: HeroId| {
            let b = scene.world.body(id);
            Pos { x: b.x, y: b.y }
        };

        // Q60: the tick's authoritative events, in log order.
        let mut events: Vec<MoraleEvent> = Vec::new();
        let defeated_now = |goblin: u32| {
            feed.iter().any(
                |e| matches!(*e, CombatEvent::EnemyDefeated { target, .. } if target == goblin),
            )
        };
        for event in &feed {
            match *event {
                CombatEvent::EnemyDefeated {
                    target,
                    cause,
                    force,
                    ..
                } if (target as usize) < n => events.push(MoraleEvent::GoblinDefeated {
                    goblin: target as usize,
                    pos: pos_of(self, target as usize),
                    force: force_of(force),
                    cause: spectacular_cause(cause),
                }),
                CombatEvent::GoblinContact { target, .. }
                | CombatEvent::BodyContact { target, .. }
                    if (target as usize) < n =>
                {
                    events.push(MoraleEvent::Spectacle {
                        pos: pos_of(self, target as usize),
                    });
                }
                CombatEvent::WallSlam { goblin, .. }
                    if (goblin as usize) < n && !defeated_now(goblin) =>
                {
                    events.push(MoraleEvent::Spectacle {
                        pos: pos_of(self, goblin as usize),
                    });
                }
                CombatEvent::ShoreLanded { .. } => {
                    let shore_defeat = feed.iter().any(|e| {
                        matches!(
                            *e,
                            CombatEvent::EnemyDefeated {
                                cause: DefeatCause::ShoreBlock { .. }
                                    | DefeatCause::ShoreBlockEdge { .. },
                                ..
                            }
                        )
                    });
                    if !shore_defeat && let Some(shore) = self.shore.as_ref() {
                        let r = shore.footprint();
                        events.push(MoraleEvent::Spectacle {
                            pos: Pos {
                                x: r.left + r.width * 0.5,
                                y: r.bottom + r.height * 0.5,
                            },
                        });
                    }
                }
                CombatEvent::HeroHurt { source, hero, .. } => {
                    events.push(MoraleEvent::HeroHurt {
                        pos: hero_pos(hero),
                    });
                    if (source as usize) < n {
                        events.push(MoraleEvent::OwnHitOnHero {
                            goblin: source as usize,
                        });
                    }
                }
                _ => {}
            }
        }
        // Q60: a goblin's own knockdown, on the tick it goes down.
        let knocked_now: Vec<bool> = self
            .enemies
            .iter()
            .map(|g| g.alive() && matches!(g.mode, GoblinMode::Launched | GoblinMode::Downed))
            .collect();
        {
            let m = self.morale.as_ref().expect("checked above");
            for i in 0..n {
                if knocked_now[i] && !m.knocked[i] {
                    events.push(MoraleEvent::OwnKnockdown { goblin: i });
                }
            }
        }

        // The hero conditions of Q64 and Q65.
        let hero_ids: Vec<HeroId> = if scene.party_live() {
            vec![HeroId::Carl, HeroId::Donut]
        } else {
            vec![HeroId::Carl]
        };
        let slow_starts: Vec<(HeroId, u16)> = feed
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::MoveStarted { move_id, .. } => {
                    let spec = carl_move(move_id).or_else(|| donut_move(move_id))?;
                    Some((hero_of(move_id.hero()), spec.startup))
                }
                _ => None,
            })
            .collect();
        let hurt_heroes: Vec<Pos> = feed
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::HeroHurt { hero, .. } => Some(hero_pos(hero)),
                _ => None,
            })
            .collect();
        let whiff = self.carl_whiffed(scene);
        let alive = self.enemies.iter().filter(|g| g.alive()).count();
        // Goblins whose tell a panic ended this tick (logged once the loop is done).
        let mut aborted: Vec<u32> = Vec::new();

        for i in 0..n {
            let goblin = &self.enemies[i];
            if !goblin.alive() || goblin.ai == EnemyAi::Passive {
                continue;
            }
            let at = pos_of(self, i);
            let nearest_hero_x = hero_ids
                .iter()
                .map(|&id| hero_pos(id).x)
                .min_by(|a, b| (a - at.x).abs().total_cmp(&(b - at.x).abs()));
            let in_swing = goblin.mode == GoblinMode::Attack && goblin.attack_tick >= TELL_TICKS;
            let can_react = !in_swing
                && matches!(
                    goblin.mode,
                    GoblinMode::Neutral | GoblinMode::Hurt | GoblinMode::Attack
                )
                && !(goblin.mode == GoblinMode::Attack
                    && goblin.attack_tick >= crate::feel::IMPACT_END);
            let facing = goblin.facing;
            let attacking = goblin.token_held();
            let neutral_mode = goblin.mode == GoblinMode::Neutral;
            let hurt_now = goblin.health < self.morale.as_ref().expect("checked").health[i];
            let (launched_or_down, dead) = (
                matches!(
                    goblin.mode,
                    GoblinMode::Launched | GoblinMode::Downed | GoblinMode::Airborne
                ),
                !goblin.alive(),
            );
            let blocked = self.morale.as_ref().expect("checked").blocked[i];
            let delta = morale::event_delta(t, i, at, &events);
            let m = self.morale.as_mut().expect("checked above");

            // Q65: a hero hit during a taunt (any hero, any non-launching hit).
            if hurt_now && let Some(stagger) = m.morale[i].on_hero_hit(t, launched_or_down || dead)
            {
                // The legacy hit-site rule: a goblin frozen by the hit's hit-stop
                // did not count a stagger tick on the hit tick, any other did.
                let g = &mut self.enemies[i];
                if g.alive() && g.mode == GoblinMode::Hurt {
                    g.stagger_left = if g.hit_stop > 0 {
                        stagger
                    } else {
                        stagger.saturating_sub(1)
                    };
                }
            }

            let out = m.morale[i].step(
                t,
                &StepInput {
                    events: delta,
                    goblin_x: at.x,
                    nearest_hero_x,
                    facing,
                    last_goblin: alive == 1,
                    can_react,
                    blocked,
                },
            );
            let nerve = m.morale[i].nerve();
            let mut began: Vec<ReactionKind> = Vec::new();
            if let Some(started) = out.started {
                began.push(match started {
                    Started::Hesitate => ReactionKind::Hesitate,
                    Started::Panic => ReactionKind::Panic,
                    Started::Enraged => ReactionKind::Enraged,
                });
                if started.aborts_tell() && attacking && !in_swing {
                    // Q61: a panic ends a tell (never a swing already past it).
                    let g = &mut self.enemies[i];
                    g.mode = GoblinMode::Neutral;
                    g.attack_tick = 0;
                    g.swing_hit = false;
                    aborted.push(i as u32);
                }
            }
            if out.cornered {
                began.push(ReactionKind::Cornered);
            }
            let m = self.morale.as_mut().expect("checked above");

            // Q64: a Coward hops back out of a slow hero move that starts in front
            // of it. Q65: a Show-off taunts when a hero is hurt or whiffs nearby.
            if neutral_mode {
                for &(id, startup) in &slow_starts {
                    let hero = hero_pos(id);
                    let facing = scene.world.body(id).facing;
                    let dx = at.x - hero.x;
                    let in_front = (dx * f32::from(facing) > 0.0).then_some(dx.abs());
                    if m.morale[i].try_backoff(t, false, in_front, startup.min(255) as u8) {
                        m.back_dir[i] = side(at.x, hero.x);
                        began.push(ReactionKind::BackOff);
                        break;
                    }
                }
                let hurt_px = hurt_heroes
                    .iter()
                    .map(|p| (p.x - at.x).abs())
                    .min_by(f32::total_cmp);
                let whiff_px = whiff.map(|x| (x - at.x).abs());
                let triggered = Morale::taunt_triggered(t, hurt_px, whiff_px);
                if m.morale[i].try_taunt(t, false, triggered) {
                    began.push(ReactionKind::Taunt);
                }
            }
            // Q64: the hop covers `q64_backoff_distance_px` over the reaction.
            let step_px = m.morale[i].backoff_step_px(t);
            let back_dir = m.back_dir[i];
            m.knocked[i] = knocked_now[i];
            m.health[i] = self.enemies[i].health;
            if m.delay_left[i] > 0 {
                m.delay_left[i] -= 1;
            }
            if step_px > 0 && neutral_mode {
                self.enemies[i].hop(back_dir, f32::from(step_px), &scene.world.solids, max_x);
            }
            for reaction in began {
                self.log(CombatEvent::GoblinReaction {
                    tick,
                    goblin: i as u32,
                    reaction,
                    nerve,
                });
            }
        }
        for goblin in aborted {
            self.log(CombatEvent::GoblinInterrupted { tick, goblin });
        }
        // Dead goblins keep the bookkeeping current (a respawn resets them).
        if let Some(m) = self.morale.as_mut() {
            for i in 0..n {
                m.knocked[i] = knocked_now[i];
                m.health[i] = self.enemies[i].health;
            }
        }
        // Whatever was logged above belongs to the next tick's feed only if it is
        // a morale event; the reactions themselves are not nerve events.
        self.morale_feed.clear();
    }

    /// Q65: Carl's move whiffed on this tick (it left its active phase without a
    /// hit). Returns Carl's x. Updates the active-move bookkeeping.
    fn carl_whiffed(&mut self, scene: &SceneSim) -> Option<f32> {
        let melee = &scene.world.hero.melee;
        let current = melee.current();
        let hit = melee.current_hit();
        let m = self.morale.as_mut()?;
        let whiff = match current {
            Some(mv) if mv.phase == Phase::Active => {
                m.carl_active = if hit { None } else { Some(mv.instance) };
                false
            }
            Some(mv) if mv.phase == Phase::Recovery && m.carl_active == Some(mv.instance) => {
                m.carl_active = None;
                true
            }
            None => m.carl_active.take().is_some(),
            _ => false,
        };
        whiff.then_some(scene.world.hero.x)
    }
}

#[cfg(test)]
impl CombatState {
    /// Test access: switch the morale on or off and rebuild the attempt.
    pub(crate) fn set_morale_for_test(&mut self, on: bool) {
        self.morale_on = on;
        self.reset_attempt();
    }
}

#[cfg(test)]
impl CombatState {
    /// Test access: set goblin `index`'s remaining first-approach delay.
    pub(crate) fn set_delay_for_test(&mut self, index: usize, ticks: u8) {
        if let Some(m) = self.morale.as_mut() {
            m.delay_left[index] = ticks;
        }
    }
}
