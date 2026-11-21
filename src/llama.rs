//! DEMO-01 PR B: the Floor 1 Bad Llama (LLAMA-01 art, ranged spitter).
//! Bevy-free and deterministic at the 60 Hz fixed tick, no randomness.
//!
//! One llama is a small state machine ([`LlamaMode`]): it walks toward Carl
//! until it is [`LLAMA_STAND_OFF`] away (never retreating, never farther than
//! [`LLAMA_LEASH`] from its home column), starts a tell when the **fair-fire**
//! rule allows it (owner option D: its whole body is inside the simulation's
//! scroll-camera view and Carl is within [`LLAMA_FIRE_RANGE`] in the direction
//! it faces; both are simulation state, so the rule is the same on every
//! output), releases a spit glob on attack tick [`LLAMA_TELL_TICKS`] and
//! recovers. The glob is a [`Glob`] the scene steps; it hurts Carl once through
//! the scene's ordinary hurt path. Hits arrive through
//! [`crate::combat::StrikeTarget`], like a goblin's.
//!
//! Hits: a jab or haymaker staggers it ([`LLAMA_HURT_TICKS`], the tell is
//! cancelled); the push kick staggers it and pushes it back about 30 px. The
//! llama is never launched and never touches a goblin. A lethal hit puts it in
//! `Down` where it stands (no knockback): which death art plays is the
//! presenter's choice from the defeat event (the slam rule, `death.rs`), never
//! the simulation's. Every number is a playtest proposal in [`crate::feel`].
//!
//! Attack timeline (attack tick 0 = the tick the tell starts): tell 0..36,
//! **spit released on tick 36** (the first tick of the art's `attack_swing`
//! frame), recover to 60.

use crate::cf02::Cf02Tuning;
use crate::combat::{EnemyAi, HitOutcome, MoveSpec, StrikeTarget, solid_at};
use crate::donut_moves::{HitStopSplit, LightGuard, TargetState, hash_guard};
use crate::feel::{
    GROUND_FRICTION, LETHAL_HIT_STOP_BONUS, LLAMA_ATTACK_TICKS, LLAMA_COOLDOWN,
    LLAMA_DEATH_SLIDE_VX, LLAMA_DOWN_TICKS, LLAMA_FIRE_RANGE, LLAMA_GLOB_HALF_H, LLAMA_GLOB_HALF_W,
    LLAMA_GLOB_MOUTH_DX, LLAMA_GLOB_MOUTH_DY, LLAMA_GLOB_RANGE, LLAMA_GLOB_SPEED, LLAMA_HEALTH,
    LLAMA_HURT_COOLDOWN, LLAMA_HURT_TICKS, LLAMA_KICK_PUSH_VX, LLAMA_LEASH, LLAMA_STAND_OFF,
    LLAMA_TELL_TICKS, LLAMA_WALK_SPEED,
};
use crate::prototype::{Block, Rect, TICK_SECONDS};

/// The llama's body box: the LLAMA-01 standing box (46 x 39, barrel and legs;
/// neck and head excluded so the llama does not body-block with its face).
/// The art's box (frame x 18 to 63) sits 3 px behind the pivot column, so the
/// box is centred 3 px behind `x` (away from the way it faces).
pub const LLAMA_HALF_WIDTH: f32 = 23.0;
pub const LLAMA_HALF_HEIGHT: f32 = 19.5;
pub const LLAMA_BODY_BACK_SHIFT: f32 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlamaMode {
    /// Standing or walking.
    Neutral,
    /// Tell, release and recover: see [`Llama::attack_tick`].
    Attack,
    /// Staggered by a hit (no attack, no walking).
    Hurt,
    /// Defeated and lying down, then it becomes a corpse.
    Down,
    /// Defeated for good: no hurtbox, no spit, no collision.
    Corpse,
}

/// What one llama tick did, for the event log and the glob.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LlamaStep {
    pub started_tell: bool,
    /// The glob released this tick (attack tick 36), if any.
    pub spit: Option<Glob>,
}

/// The simulation's scroll-camera view in layout pixels, for the fair-fire
/// rule. Simulation state (`SceneSim::camera`), never the presentation view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FireView {
    pub left: f32,
    pub right: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Llama {
    pub x: f32,
    /// Centre height of the llama standing on the floor.
    pub y: f32,
    pub floor_y: f32,
    pub home_x: f32,
    pub health: u8,
    pub ai: EnemyAi,
    /// +1 / -1: the way it faces (always toward Carl while it lives and acts).
    pub facing: i8,
    pub mode: LlamaMode,
    /// Ticks since the tell started (only meaningful in `Attack`).
    pub attack_tick: u16,
    pub stagger_left: u8,
    /// Ticks lying down since the killing hit landed (only in `Down`).
    pub down_ticks: u16,
    pub vx: f32,
    /// Ticks of hit-stop left: nothing of this llama advances.
    pub hit_stop: u8,
    /// Ticks until the next tell may start.
    pub cooldown: u16,
    /// The player whose move last hit it (read for the defeat only; slot 0 is
    /// not mixed into the hash, like a goblin's).
    pub hit_by: Option<u32>,
    /// CF02-C1 (Q18): the light guard a Donut swat that interrupted a tell
    /// sets (Q10-06), so Donut alone cannot lock a llama. It clears after
    /// `LIGHT_GUARD_MIN_TICKS` **and** the llama's next spit release, or on
    /// defeat. Mixed into the hash only while set, so no earlier replay (where
    /// nothing sets it) changes.
    pub guard: Option<LightGuard>,
}

impl Llama {
    pub fn new(x: f32, floor_y: f32, ai: EnemyAi) -> Self {
        Self {
            x,
            y: floor_y,
            floor_y,
            home_x: x,
            health: LLAMA_HEALTH,
            ai,
            facing: -1,
            mode: LlamaMode::Neutral,
            attack_tick: 0,
            stagger_left: 0,
            down_ticks: 0,
            vx: 0.0,
            hit_stop: 0,
            cooldown: 0,
            hit_by: None,
            guard: None,
        }
    }

    /// What the Donut move domain sees of this llama (CF02-C1, Q18): the tell
    /// is attack ticks `0..LLAMA_TELL_TICKS`, the spit release tick is the
    /// `Impact` (the guard's clear condition, latched), the rest of the attack
    /// is `Recovery`; a defeated llama is no target.
    pub fn target_state(&self) -> TargetState {
        match self.mode {
            LlamaMode::Neutral => TargetState::Neutral,
            LlamaMode::Attack => match self.attack_tick.cmp(&LLAMA_TELL_TICKS) {
                core::cmp::Ordering::Less => TargetState::Tell,
                core::cmp::Ordering::Equal => TargetState::Impact,
                core::cmp::Ordering::Greater => TargetState::Recovery,
            },
            LlamaMode::Hurt => TargetState::Staggered,
            LlamaMode::Down | LlamaMode::Corpse => TargetState::Defeated,
        }
    }

    /// One tick of the light guard (nothing while unset): dropped on release.
    fn observe_guard(&mut self) {
        let state = self.target_state();
        if let Some(guard) = self.guard.as_mut()
            && guard.observe(state)
        {
            self.guard = None;
        }
    }

    pub fn alive(&self) -> bool {
        self.health > 0
    }

    /// The 46 x 39 standing box.
    pub fn body(&self) -> Rect {
        llama_rect(self.x, self.y, self.facing)
    }

    /// In the tell: a hit cancels the spit.
    pub fn telling(&self) -> bool {
        self.mode == LlamaMode::Attack && self.attack_tick < LLAMA_TELL_TICKS
    }

    /// The fair-fire rule (owner option D): the whole body is inside the
    /// simulation's scroll-camera view and Carl is within
    /// [`LLAMA_FIRE_RANGE`] in the direction the llama faces.
    pub fn may_fire(&self, hero_x: f32, view: FireView) -> bool {
        let body = self.body();
        let dx = hero_x - self.x;
        body.left >= view.left
            && body.right() <= view.right
            && dx.abs() <= LLAMA_FIRE_RANGE
            && dx * f32::from(self.facing) > 0.0
    }

    /// One fixed tick. `view` is the simulation's scroll-camera view;
    /// `hero_alive` false keeps the llama idle.
    pub fn step(
        &mut self,
        hero_x: f32,
        hero_alive: bool,
        solids: &[Block],
        max_x: f32,
        view: FireView,
    ) -> LlamaStep {
        let out = self.step_mode(hero_x, hero_alive, solids, max_x, view);
        self.observe_guard();
        out
    }

    fn step_mode(
        &mut self,
        hero_x: f32,
        hero_alive: bool,
        solids: &[Block],
        max_x: f32,
        view: FireView,
    ) -> LlamaStep {
        let mut out = LlamaStep::default();
        if self.hit_stop > 0 {
            self.hit_stop -= 1;
            return out;
        }
        match self.mode {
            LlamaMode::Corpse => {}
            LlamaMode::Down => {
                self.slide(solids, max_x);
                self.down_ticks += 1;
                if self.down_ticks >= LLAMA_DOWN_TICKS {
                    self.mode = LlamaMode::Corpse;
                    self.vx = 0.0;
                }
            }
            LlamaMode::Hurt => {
                self.slide(solids, max_x);
                self.stagger_left = self.stagger_left.saturating_sub(1);
                if self.stagger_left == 0 {
                    self.mode = LlamaMode::Neutral;
                }
            }
            LlamaMode::Attack => {
                self.attack_tick += 1;
                if self.attack_tick == LLAMA_TELL_TICKS {
                    out.spit = Some(Glob::from_mouth(self));
                }
                if self.attack_tick >= LLAMA_ATTACK_TICKS {
                    self.mode = LlamaMode::Neutral;
                    self.attack_tick = 0;
                    self.cooldown = LLAMA_COOLDOWN;
                }
            }
            LlamaMode::Neutral => {
                self.cooldown = self.cooldown.saturating_sub(1);
                self.slide(solids, max_x);
                out.started_tell = self.neutral(hero_x, hero_alive, solids, view);
            }
        }
        out
    }

    /// Standing or walking: start a tell when allowed, else close in toward
    /// Carl (never away from him, never beyond the leash).
    fn neutral(&mut self, hero_x: f32, hero_alive: bool, solids: &[Block], view: FireView) -> bool {
        if self.ai == EnemyAi::Passive || !hero_alive {
            return false;
        }
        let dx = hero_x - self.x;
        let toward: i8 = if dx > 0.0 { 1 } else { -1 };
        self.facing = toward;
        if self.cooldown == 0 && self.vx == 0.0 && self.may_fire(hero_x, view) {
            self.mode = LlamaMode::Attack;
            self.attack_tick = 0;
            return true;
        }
        if dx.abs() > LLAMA_STAND_OFF {
            let candidate = self.x + f32::from(toward) * LLAMA_WALK_SPEED * TICK_SECONDS;
            if (candidate - self.home_x).abs() <= LLAMA_LEASH
                && !solid_at(solids, llama_rect(candidate, self.y, self.facing))
            {
                self.x = candidate;
            }
        }
        false
    }

    /// Slide by the knockback velocity: stopped by walls and the layout edge,
    /// slowed by ground friction.
    fn slide(&mut self, solids: &[Block], max_x: f32) {
        if self.vx == 0.0 {
            return;
        }
        let wanted = self.x + self.vx * TICK_SECONDS;
        let candidate = wanted.clamp(LLAMA_HALF_WIDTH, max_x);
        let blocked = solid_at(solids, llama_rect(candidate, self.y, self.facing));
        if !blocked {
            self.x = candidate;
        }
        if blocked || candidate != wanted {
            self.vx = 0.0;
            return;
        }
        let drag = GROUND_FRICTION * TICK_SECONDS;
        self.vx = if self.vx.abs() <= drag {
            0.0
        } else {
            self.vx - drag * self.vx.signum()
        };
    }

    /// CP-03 shore: the falling block lands on this llama. It is defeated on
    /// the spot. Returns false for a llama that is not alive.
    pub fn crush(&mut self) -> bool {
        if !self.alive() {
            return false;
        }
        self.health = 0;
        self.mode = LlamaMode::Down;
        self.down_ticks = 0;
        self.attack_tick = 0;
        self.stagger_left = 0;
        self.vx = 0.0;
        self.hit_stop = 0;
        true
    }

    /// Fold the full state into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(u64::from(self.x.to_bits()));
        mix(u64::from(self.y.to_bits()));
        mix(u64::from(self.home_x.to_bits()));
        mix(u64::from(self.health));
        mix(self.ai as u64);
        mix(self.facing as u64);
        mix(self.mode as u64);
        mix(u64::from(self.attack_tick));
        mix(u64::from(self.stagger_left));
        mix(u64::from(self.down_ticks));
        mix(u64::from(self.vx.to_bits()));
        mix(u64::from(self.hit_stop));
        mix(u64::from(self.cooldown));
        // Slot 0 is implied by a hashed `Hit`; any other player is mixed.
        if let Some(player) = self.hit_by
            && player != 0
        {
            mix(0x4849_5442);
            mix(u64::from(player));
        }
        // CF02-C1: the light guard, only while set.
        hash_guard(&self.guard, mix);
    }
}

fn llama_rect(x: f32, y: f32, facing: i8) -> Rect {
    Rect {
        left: x - LLAMA_HALF_WIDTH - LLAMA_BODY_BACK_SHIFT * f32::from(facing),
        bottom: y - LLAMA_HALF_HEIGHT,
        width: LLAMA_HALF_WIDTH * 2.0,
        height: LLAMA_HALF_HEIGHT * 2.0,
    }
}

impl StrikeTarget for Llama {
    fn hurtbox_active(&self) -> bool {
        self.alive()
    }

    fn can_be_hit(&self) -> bool {
        self.alive() && self.hit_stop == 0
    }

    fn body(&self) -> Rect {
        Llama::body(self)
    }

    fn centre(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    fn health_for_glance(&self) -> u8 {
        self.health
    }

    /// Damage, then the reaction: the killing hit slides the body briefly
    /// away from the attacker ([`LLAMA_DEATH_SLIDE_VX`], about 30 px, so the
    /// head of the death pose does not lie under Carl's feet; the death art is
    /// the presenter's business); any other hit staggers it and cancels a tell,
    /// and the push kick also pushes it back.
    fn take_hit(&mut self, spec: &MoveSpec, attacker_facing: i8) -> (HitOutcome, u8) {
        let (outcome, split) = self.take_hit_split(spec, attacker_facing, HitStopSplit::legacy);
        (outcome, split.attacker)
    }
}

impl Llama {
    /// [`StrikeTarget::take_hit`] with the hit-stop split chosen by the caller
    /// (CF02-C1: Donut's moves freeze the llama differently from Carl's). The
    /// scene steps llamas after the hero strikes on the same tick, hence the
    /// legacy split's `stop + 1` for the target. Behaviour is identical to the
    /// former `take_hit` for [`HitStopSplit::legacy`].
    pub fn take_hit_split(
        &mut self,
        spec: &MoveSpec,
        attacker_facing: i8,
        split: impl FnOnce(u8) -> HitStopSplit,
    ) -> (HitOutcome, HitStopSplit) {
        let reaction = spec.reaction;
        let direction = f32::from(attacker_facing);
        self.health = self.health.saturating_sub(spec.damage);
        let mut stop = reaction.hit_stop;
        let outcome = if self.health == 0 {
            self.mode = LlamaMode::Down;
            self.attack_tick = 0;
            self.stagger_left = 0;
            self.down_ticks = 0;
            self.vx = direction * LLAMA_DEATH_SLIDE_VX;
            stop += LETHAL_HIT_STOP_BONUS;
            HitOutcome::Defeated
        } else {
            self.mode = LlamaMode::Hurt;
            self.attack_tick = 0;
            self.stagger_left = LLAMA_HURT_TICKS;
            self.cooldown = self.cooldown.max(LLAMA_HURT_COOLDOWN);
            self.vx = if reaction.launch {
                direction * LLAMA_KICK_PUSH_VX
            } else {
                direction * reaction.knock_vx
            };
            HitOutcome::Hurt {
                health_left: self.health,
            }
        };
        let split = split(stop);
        self.hit_stop = split.target;
        (outcome, split)
    }
}

/// CF02-C1 (Q18, Q37): the damage Donut's moves deal to a llama, read from the
/// CF02 knob file. `Some` in the kit only while `q18_llama_targets` is
/// effective (`cf02_rules` on); `None` everywhere else, so no llama is a target
/// and every earlier replay is byte-identical. The interrupt stagger (18), the
/// hurt cooldown (30) and the guard minimum (60) are the llama's own constants
/// (`LLAMA_HURT_TICKS`, `LLAMA_HURT_COOLDOWN`, `LIGHT_GUARD_MIN_TICKS`), pinned
/// equal to their knobs by a test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LlamaTargets {
    pub swat_damage: u8,
    pub pounce_damage: u8,
    pub missile_damage: u8,
}

impl LlamaTargets {
    /// The knobs of `tuning`, or `None` while the Q18 switch is off (Q37).
    pub fn from_tuning(tuning: &Cf02Tuning) -> Option<Self> {
        tuning.llama_targets().then_some(Self {
            swat_damage: tuning.q18_swat_damage,
            pounce_damage: tuning.q18_pounce_damage,
            missile_damage: tuning.q18_missile_damage,
        })
    }

    /// The committed knobs with `cf02_rules` forced to `rules`: the `--cf02`
    /// run (`true`) or the legacy scene (`false`, giving `None`).
    pub fn for_rules(rules: bool) -> Option<Self> {
        let mut tuning = Cf02Tuning::pinned().expect("the committed CF02 data loads");
        tuning.cf02_rules = rules;
        Self::from_tuning(&tuning)
    }
}

// -------------------------------------------------------------------- glob --

/// What a glob did this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobFate {
    Flying,
    /// Stopped by a solid.
    Blocked,
    /// Flew its full range.
    Spent,
}

/// The spit glob: a separate entity (LLAMA-01 spawn contract) that flies
/// straight at mouth height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glob {
    pub x: f32,
    /// Centre height (world y-up).
    pub y: f32,
    pub dir: i8,
    pub travelled: f32,
    /// Index among the llamas of the one that spat it.
    pub owner: u32,
    /// DONUT-MOBILITY-01 DM-B (F8): the combat tick the glob spawned on. With
    /// `owner` it is the glob's instance identity for the cat reflex (DM-Q19).
    /// Set by the scene at spawn, **not hashed** (derivable from the spit
    /// events), so adding it moves no golden.
    pub spawn_tick: u32,
}

impl Glob {
    /// Spawned at the mouth of `llama` (46 px ahead, 45 px above its feet).
    fn from_mouth(llama: &Llama) -> Self {
        let feet = llama.y - LLAMA_HALF_HEIGHT;
        Self {
            x: llama.x + f32::from(llama.facing) * LLAMA_GLOB_MOUTH_DX,
            y: feet + LLAMA_GLOB_MOUTH_DY,
            dir: llama.facing,
            travelled: 0.0,
            owner: 0,
            spawn_tick: 0,
        }
    }

    /// The 8 x 6 hit box.
    pub fn rect(&self) -> Rect {
        Rect {
            left: self.x - LLAMA_GLOB_HALF_W,
            bottom: self.y - LLAMA_GLOB_HALF_H,
            width: LLAMA_GLOB_HALF_W * 2.0,
            height: LLAMA_GLOB_HALF_H * 2.0,
        }
    }

    /// One fixed tick of flight: straight on, stopped by a solid, spent after
    /// [`LLAMA_GLOB_RANGE`].
    pub fn step(&mut self, solids: &[Block]) -> GlobFate {
        let dx = f32::from(self.dir) * LLAMA_GLOB_SPEED * TICK_SECONDS;
        let moved = Glob {
            x: self.x + dx,
            ..*self
        };
        if solid_at(solids, moved.rect()) {
            return GlobFate::Blocked;
        }
        self.x += dx;
        self.travelled += dx.abs();
        if self.travelled >= LLAMA_GLOB_RANGE {
            GlobFate::Spent
        } else {
            GlobFate::Flying
        }
    }

    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(u64::from(self.x.to_bits()));
        mix(u64::from(self.y.to_bits()));
        mix(self.dir as u64);
        mix(u64::from(self.travelled.to_bits()));
        mix(u64::from(self.owner));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{CARL_HAYMAKER, CARL_JAB, CARL_PUSH_KICK};
    use crate::prototype::Material;

    const FLOOR: f32 = 119.0;
    const EDGE: f32 = 2000.0;
    const VIEW: FireView = FireView {
        left: 0.0,
        right: 640.0,
    };

    fn llama_at(x: f32) -> Llama {
        Llama::new(x, FLOOR, EnemyAi::Chase)
    }

    fn wall(left: f32) -> Block {
        Block {
            rect: Rect {
                left,
                bottom: 0.0,
                width: 20.0,
                height: 400.0,
            },
            material: Material::Stone,
            intact: true,
        }
    }

    fn tick(l: &mut Llama, hero_x: f32) -> LlamaStep {
        l.step(hero_x, true, &[], EDGE, VIEW)
    }

    #[test]
    fn the_spit_timeline_is_tell_release_recover_cooldown() {
        let mut l = llama_at(300.0);
        let mut spit_tick = None;
        let mut tell_tick = None;
        let mut idle_again = None;
        for t in 1..=400u32 {
            let step = tick(&mut l, 100.0);
            if step.started_tell && tell_tick.is_none() {
                tell_tick = Some(t);
            }
            if step.spit.is_some() && spit_tick.is_none() {
                spit_tick = Some(t);
            }
            if tell_tick.is_some() && l.mode == LlamaMode::Neutral && idle_again.is_none() {
                idle_again = Some(t);
            }
        }
        // A tell starts on the first tick (nothing blocks it), the glob leaves
        // 36 ticks later, the attack ends on tick 60 (tell tick + 59 steps).
        assert_eq!(tell_tick, Some(1));
        assert_eq!(spit_tick, Some(1 + u32::from(LLAMA_TELL_TICKS)));
        assert_eq!(idle_again, Some(1 + u32::from(LLAMA_ATTACK_TICKS)));
    }

    #[test]
    fn the_next_tell_waits_for_the_cooldown() {
        let mut l = llama_at(300.0);
        let mut tells = Vec::new();
        for t in 1..=400u32 {
            if tick(&mut l, 100.0).started_tell {
                tells.push(t);
            }
        }
        // tell start to tell start: the 60 attack ticks, then the 90 tick
        // cooldown (the next tell starts on the tick the cooldown reaches 0).
        let gap = u32::from(LLAMA_ATTACK_TICKS) + u32::from(LLAMA_COOLDOWN);
        assert_eq!(tells[1] - tells[0], gap);
        assert_eq!(tells[2] - tells[1], gap);
    }

    #[test]
    fn fair_fire_needs_the_body_inside_the_view_and_carl_in_range_ahead() {
        let mut l = llama_at(300.0);
        l.facing = -1;
        // Inside the view, Carl 200 px ahead: may fire.
        assert!(l.may_fire(100.0, VIEW));
        // Carl farther than 320 px: no.
        assert!(!l.may_fire(-30.0, VIEW));
        // Carl behind it (it turns first, so only the facing rule is tested).
        assert!(!l.may_fire(500.0, VIEW));
        // The body partly outside the view: no.
        let edge = FireView {
            left: 290.0,
            right: 930.0,
        };
        assert!(!l.may_fire(100.0, edge));
        // Fully outside: no.
        let away = FireView {
            left: 1000.0,
            right: 1640.0,
        };
        assert!(!l.may_fire(100.0, away));
    }

    #[test]
    fn a_llama_outside_the_view_never_starts_a_tell() {
        let mut l = llama_at(300.0);
        let away = FireView {
            left: 1000.0,
            right: 1640.0,
        };
        for _ in 0..300 {
            assert!(!l.step(100.0, true, &[], EDGE, away).started_tell);
        }
    }

    #[test]
    fn it_walks_toward_carl_to_the_stand_off_and_never_retreats() {
        let mut l = llama_at(900.0);
        // Home 900, leash 160: it may reach x 740 at most (Carl is at 100).
        for _ in 0..1200 {
            tick(&mut l, 100.0);
            assert!(l.x <= 900.0, "never retreats");
        }
        assert!((l.x - (900.0 - LLAMA_LEASH)).abs() < 1.0, "x {}", l.x);
        // Carl close: it stands still.
        let mut near = llama_at(300.0);
        for _ in 0..100 {
            tick(&mut near, 200.0);
        }
        assert_eq!(near.x, 300.0);
    }

    #[test]
    fn the_leash_keeps_it_within_160_px_of_home() {
        let mut l = llama_at(1000.0);
        for _ in 0..2000 {
            // Carl far away out of the view: it walks but cannot fire.
            l.step(
                0.0,
                true,
                &[],
                EDGE,
                FireView {
                    left: 0.0,
                    right: 100.0,
                },
            );
        }
        assert!((l.x - (1000.0 - LLAMA_LEASH)).abs() < 1.0, "x {}", l.x);
    }

    #[test]
    fn a_wall_stops_the_walk() {
        let mut l = llama_at(640.0);
        let solids = [wall(560.0)];
        for _ in 0..300 {
            l.step(
                100.0,
                true,
                &solids,
                EDGE,
                FireView {
                    left: 0.0,
                    right: 100.0,
                },
            );
        }
        // The wall's right face is at 580: the body (23 px half width) stops
        // just right of it and never enters it.
        // (Facing left, the box is centred 3 px right of x.)
        assert!(
            l.x >= 580.0 + LLAMA_HALF_WIDTH - LLAMA_BODY_BACK_SHIFT,
            "x {}",
            l.x
        );
        assert!(l.x < 640.0, "it walked toward Carl until the wall");
    }

    #[test]
    fn a_hit_in_the_tell_cancels_the_spit() {
        let mut l = llama_at(300.0);
        for _ in 0..10 {
            tick(&mut l, 100.0);
        }
        assert!(l.telling());
        let (outcome, stop) = l.take_hit(&CARL_JAB, 1);
        assert_eq!(outcome, HitOutcome::Hurt { health_left: 2 });
        assert_eq!(stop, CARL_JAB.reaction.hit_stop);
        assert_eq!(l.mode, LlamaMode::Hurt);
        // The cancelled tell never spits: for the stagger, the pause and the
        // next full tell there is no glob.
        let mut first_spit = None;
        for t in 1..=200u32 {
            if tick(&mut l, 100.0).spit.is_some() && first_spit.is_none() {
                first_spit = Some(t);
            }
        }
        let earliest = u32::from(LLAMA_HURT_TICKS)
            + u32::from(LLAMA_HURT_COOLDOWN)
            + u32::from(LLAMA_TELL_TICKS);
        assert!(
            first_spit.is_none_or(|t| t >= earliest),
            "first glob at {first_spit:?}, not before {earliest}"
        );
        assert!(first_spit.is_some(), "it goes around again");
    }

    #[test]
    fn the_kick_pushes_it_back_about_thirty_pixels_and_never_launches() {
        let mut l = llama_at(300.0);
        let (outcome, stop) = l.take_hit(&CARL_PUSH_KICK, 1);
        assert_eq!(outcome, HitOutcome::Hurt { health_left: 2 });
        assert_eq!(stop, CARL_PUSH_KICK.reaction.hit_stop);
        // Carl stands close (260), so the llama stays where the push leaves it.
        for _ in 0..80 {
            tick(&mut l, 260.0);
        }
        let pushed = l.x - 300.0;
        assert!((26.0..=36.0).contains(&pushed), "pushed {pushed} px");
        assert_eq!(l.y, FLOOR, "no launch");
    }

    #[test]
    fn three_hits_defeat_it_where_it_stands_then_it_is_a_corpse() {
        for spec in [&CARL_JAB, &CARL_HAYMAKER, &CARL_PUSH_KICK] {
            let mut l = llama_at(300.0);
            let mut last = None;
            for _ in 0..3 {
                l.hit_stop = 0;
                last = Some(l.take_hit(spec, 1));
            }
            let (outcome, stop) = last.unwrap();
            assert_eq!(outcome, HitOutcome::Defeated);
            assert_eq!(stop, spec.reaction.hit_stop + LETHAL_HIT_STOP_BONUS);
            assert_eq!((l.mode, l.x), (LlamaMode::Down, 300.0));
            assert!(!l.alive() && !l.can_be_hit() && !l.hurtbox_active());
            // The body slides about 30 px away from the attacker (facing +1
            // pushes it right) and then lies; a corpse stays put.
            for _ in 0..(u32::from(LLAMA_DOWN_TICKS) + 20) {
                tick(&mut l, 100.0);
            }
            assert_eq!(l.mode, LlamaMode::Corpse);
            assert!((326.0..=336.0).contains(&l.x), "slid to {}", l.x);
            assert_eq!(l.vx, 0.0);
        }
    }

    #[test]
    fn the_glob_spawns_at_the_mouth_flies_straight_and_ends_at_its_range() {
        let mut l = llama_at(300.0);
        let mut glob = None;
        for _ in 0..40 {
            if let Some(g) = tick(&mut l, 100.0).spit {
                glob = Some(g);
            }
        }
        let mut glob = glob.expect("the glob");
        assert_eq!(glob.x, 300.0 - LLAMA_GLOB_MOUTH_DX);
        assert_eq!(glob.y, FLOOR - LLAMA_HALF_HEIGHT + LLAMA_GLOB_MOUTH_DY);
        assert_eq!(glob.dir, -1);
        let (x0, y0) = (glob.x, glob.y);
        let mut steps = 0;
        while glob.step(&[]) == GlobFate::Flying {
            steps += 1;
            assert_eq!(glob.y, y0, "straight at mouth height");
        }
        assert_eq!(glob.step(&[]), GlobFate::Spent);
        assert!((x0 - glob.x - LLAMA_GLOB_RANGE).abs() < LLAMA_GLOB_SPEED * TICK_SECONDS * 2.0);
        // 320 px at 2.5 px per tick: 128 ticks.
        assert!((127..=129).contains(&(steps + 1)), "{steps}");
    }

    #[test]
    fn the_glob_stops_on_a_solid() {
        let mut glob = Glob {
            x: 300.0,
            y: 140.0,
            dir: -1,
            travelled: 0.0,
            owner: 0,
            spawn_tick: 0,
        };
        let solids = [wall(230.0)];
        let mut fate = GlobFate::Flying;
        for _ in 0..200 {
            fate = glob.step(&solids);
            if fate != GlobFate::Flying {
                break;
            }
        }
        assert_eq!(fate, GlobFate::Blocked);
        assert!(glob.x >= 250.0 - LLAMA_GLOB_HALF_W - 3.0);
        // A glob above the wall is not stopped.
        let mut high = Glob { y: 500.0, ..glob };
        assert_eq!(high.step(&solids), GlobFate::Flying);
    }

    #[test]
    fn a_passive_llama_never_tells_and_a_dead_hero_keeps_it_idle() {
        let mut passive = Llama::new(300.0, FLOOR, EnemyAi::Passive);
        let mut chase = llama_at(300.0);
        for _ in 0..200 {
            assert!(!passive.step(100.0, true, &[], EDGE, VIEW).started_tell);
            assert!(!chase.step(100.0, false, &[], EDGE, VIEW).started_tell);
        }
    }

    #[test]
    fn the_body_box_sits_three_pixels_behind_the_pivot() {
        let mut l = llama_at(300.0);
        l.facing = 1;
        assert_eq!(l.body().left, 300.0 - 23.0 - 3.0);
        l.facing = -1;
        assert_eq!(l.body().left, 300.0 - 23.0 + 3.0);
        assert_eq!(l.body().width, 46.0);
    }

    #[test]
    fn crush_defeats_once() {
        let mut l = llama_at(300.0);
        assert!(l.crush());
        assert!(!l.crush());
        assert_eq!((l.mode, l.health), (LlamaMode::Down, 0));
    }
    // ------------------------------------------- CF02-C1: Donut's hits --

    #[test]
    fn the_legacy_take_hit_is_the_legacy_split_of_take_hit_split() {
        for spec in [&CARL_JAB, &CARL_HAYMAKER, &CARL_PUSH_KICK] {
            let mut a = llama_at(440.0);
            let mut b = llama_at(440.0);
            let (out_a, stop) = a.take_hit(spec, 1);
            let (out_b, split) = b.take_hit_split(spec, 1, HitStopSplit::legacy);
            assert_eq!((out_a, stop), (out_b, split.attacker));
            assert_eq!(a, b);
            assert_eq!(a.hit_stop, stop + 1);
        }
    }

    #[test]
    fn the_target_state_follows_the_mode_and_the_attack_tick() {
        let mut l = llama_at(440.0);
        assert_eq!(l.target_state(), TargetState::Neutral);
        l.mode = LlamaMode::Attack;
        for (attack_tick, want) in [
            (0, TargetState::Tell),
            (LLAMA_TELL_TICKS - 1, TargetState::Tell),
            (LLAMA_TELL_TICKS, TargetState::Impact),
            (LLAMA_TELL_TICKS + 1, TargetState::Recovery),
        ] {
            l.attack_tick = attack_tick;
            assert_eq!(l.target_state(), want, "attack tick {attack_tick}");
        }
        l.mode = LlamaMode::Hurt;
        assert_eq!(l.target_state(), TargetState::Staggered);
        for mode in [LlamaMode::Down, LlamaMode::Corpse] {
            l.mode = mode;
            assert_eq!(l.target_state(), TargetState::Defeated);
        }
    }

    fn hash_of(l: &Llama) -> Vec<u64> {
        let mut out = Vec::new();
        l.mix_into(&mut |v| out.push(v));
        out
    }

    #[test]
    fn the_guard_is_mixed_only_while_set_and_clears_after_60_ticks_and_the_spit() {
        let mut l = llama_at(560.0);
        let unset = hash_of(&l);
        l.guard = Some(LightGuard::new());
        assert_ne!(hash_of(&l), unset);
        l.guard = None;
        assert_eq!(hash_of(&l), unset, "an unset guard hashes nothing");
        // Carl idles at 400: the llama tells at tick 1, spits on attack tick
        // 36 (the Impact) and the guard (set at the start) clears once 60
        // ticks have passed and that spit was seen.
        l.guard = Some(LightGuard::new());
        let mut cleared_at = None;
        for t in 1..=120u32 {
            tick(&mut l, 400.0);
            if l.guard.is_none() {
                cleared_at = Some(t);
                break;
            }
        }
        // Tell on tick 1 (attack tick 0), spit on tick 37, but the guard needs
        // 60 ticks: it clears on tick 60.
        assert_eq!(cleared_at, Some(60));
        // A guard set on a defeated llama is dropped at once.
        let mut dead = llama_at(560.0);
        dead.guard = Some(LightGuard::new());
        dead.take_hit(&CARL_PUSH_KICK, 1);
        dead.health = 0;
        dead.mode = LlamaMode::Down;
        tick(&mut dead, 400.0);
        assert!(dead.guard.is_none());
    }
}
