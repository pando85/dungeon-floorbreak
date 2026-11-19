//! The Floor 1 scene goblin (CP-01, GOB-01 spec section 4): approach, a
//! telegraphed tell and swing in place of contact damage, stagger, knockback,
//! defeat and corpse. Bevy-free and deterministic at the 60 Hz fixed tick.
//!
//! One goblin is a small state machine ([`GoblinMode`]); the group rules (the
//! attack token, tell spacing) are decided by [`crate::combat_scene`], which
//! tells each goblin whether it may approach and whether it may start a tell
//! this tick. Hits arrive through [`crate::combat::StrikeTarget`], the same
//! [`crate::combat::resolve_strike`] the greybox enemy uses. Every number is in
//! [`crate::feel`].
//!
//! Attack timeline (attack tick 0 = the tick the tell starts): tell 0..24,
//! smear 24..26, **impact 26..30 (the only ticks that can hurt Carl)**, recover
//! 30..50. A hit during the tell or the recovery interrupts the attack (stagger,
//! token back). The swing itself (smear and impact) cannot be interrupted, as
//! in the GOB-01 state diagram: a hit then deals damage and hit-stop but no
//! stagger and no knockback, unless it defeats the goblin.

use crate::combat::{
    ENEMY_HALF_HEIGHT, ENEMY_HALF_WIDTH, ENEMY_HEALTH, EnemyAi, HitOutcome, MoveSpec, StrikeTarget,
    body_rect, solid_at,
};
use crate::feel::{
    ATTACK_COOLDOWN, ATTACK_TICKS, CONTACT_HIT_STOP, CONTACT_POSE_TICKS, CONTACT_PUSH_VX,
    CONTACT_SOURCE_KEEP, CONTACT_STAGGER, DOWN_TICKS, DOWNED_TICKS, GETUP_TICKS, GOBLIN_SIGHT,
    GOBLIN_STANDOFF, GOBLIN_TRIGGER, GOBLIN_WALK_SPEED, GROUND_FRICTION, HURT_COOLDOWN, IMPACT_END,
    KNOCK_GRAVITY, KNOCK_VX_MAX, LETHAL_HIT_STOP_BONUS, LETHAL_KNOCK_VX, LETHAL_KNOCK_VY,
    SMEAR_END, SWING_X_MAX, SWING_X_MIN, SWING_Y_MAX, SWING_Y_MIN, TELL_TICKS,
};
use crate::prototype::{Block, Rect, TICK_SECONDS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoblinMode {
    /// Standing or walking.
    Neutral,
    /// Tell, swing or recovery: see [`Goblin::attack_phase`].
    Attack,
    /// Staggered by a hit (no attack, no walking).
    Hurt,
    /// Launched by the killing hit.
    Airborne,
    /// Defeated and lying down, then it becomes a corpse.
    Down,
    /// Defeated for good: no hurtbox, no swing, no collision, never hurts.
    Corpse,
    // Appended after `Corpse` so the state hashes of the older modes stay put.
    /// A **living** goblin launched by the push kick (D-02); it can contact a
    /// second goblin once per launch (D-03).
    Launched,
    /// A living goblin that landed from a launch: lies down, then gets up.
    Downed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackPhase {
    Tell,
    Smear,
    Impact,
    Recover,
}

/// The phase of an attack at `attack_tick` (tick 0 = the tell starts).
pub fn attack_phase(attack_tick: u16) -> AttackPhase {
    if attack_tick < TELL_TICKS {
        AttackPhase::Tell
    } else if attack_tick < SMEAR_END {
        AttackPhase::Smear
    } else if attack_tick < IMPACT_END {
        AttackPhase::Impact
    } else {
        AttackPhase::Recover
    }
}

/// What one goblin tick did, for the event log and the hit test on Carl.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GoblinStep {
    pub started_tell: bool,
    /// The swing became active this tick (attack tick 26).
    pub swing_began: bool,
    /// The swing is active this tick (attack ticks 26..=29).
    pub impact: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Goblin {
    pub x: f32,
    pub y: f32,
    /// Centre height of the goblin standing on the floor.
    pub floor_y: f32,
    pub health: u8,
    pub ai: EnemyAi,
    /// +1 / -1.
    pub facing: i8,
    pub mode: GoblinMode,
    /// Ticks since the tell started (only meaningful in `Attack`).
    pub attack_tick: u16,
    pub stagger_left: u8,
    /// Ticks lying down since the killing hit landed (only in `Down`).
    pub down_ticks: u16,
    pub vx: f32,
    pub vy: f32,
    /// Ticks of hit-stop left: nothing of this goblin advances.
    pub hit_stop: u8,
    pub attack_cooldown: u8,
    pub hurt_cooldown: u8,
    /// The running attack already hurt Carl (one hit per attack).
    pub swing_hit: bool,
    /// The move instance whose kick carries this goblin (`Launched` only).
    pub launch: Option<u32>,
    /// Ticks since the launch was released (the animation's flight age).
    pub flight_ticks: u16,
    /// Ticks the `knockback_contact` pose is still held after a contact.
    pub contact_ticks: u8,
    /// The player whose move last hit this goblin while it lived (a kick that
    /// launched it, a jab). It credits a later block as the instigator of the
    /// defeat; it is read for the defeat event only. Player 0 (the only one in
    /// play) is not mixed into the state hash, because with one player
    /// `Some(0)` is already implied by a hashed `Hit`; any other player is
    /// mixed, so a second player's hit never hashes like Carl's.
    pub hit_by: Option<u32>,
}

impl Goblin {
    pub fn new(x: f32, floor_y: f32, ai: EnemyAi) -> Self {
        Self {
            x,
            y: floor_y,
            floor_y,
            health: ENEMY_HEALTH,
            ai,
            facing: -1,
            mode: GoblinMode::Neutral,
            attack_tick: 0,
            stagger_left: 0,
            down_ticks: 0,
            vx: 0.0,
            vy: 0.0,
            hit_stop: 0,
            attack_cooldown: 0,
            hurt_cooldown: 0,
            swing_hit: false,
            launch: None,
            flight_ticks: 0,
            contact_ticks: 0,
            hit_by: None,
        }
    }

    pub fn alive(&self) -> bool {
        self.health > 0
    }

    pub fn staggered(&self) -> bool {
        self.mode == GoblinMode::Hurt
    }

    /// Standing on the floor (not launched or hopping).
    pub fn grounded(&self) -> bool {
        self.y <= self.floor_y && self.vy <= 0.0
    }

    /// In a tell or a swing: holds one of the attack tokens. The token goes
    /// back when the swing ends (recovery) or the attack is cancelled.
    pub fn token_held(&self) -> bool {
        self.mode == GoblinMode::Attack && self.attack_tick < IMPACT_END
    }

    pub fn attack_phase(&self) -> Option<AttackPhase> {
        (self.mode == GoblinMode::Attack).then(|| attack_phase(self.attack_tick))
    }

    /// Could take an attack token this tick (decided before the tick steps).
    pub fn wants_attack(&self) -> bool {
        self.alive()
            && self.mode == GoblinMode::Neutral
            && self.ai == EnemyAi::Chase
            && self.attack_cooldown == 0
            && self.hurt_cooldown == 0
            && self.hit_stop == 0
            && self.grounded()
    }

    /// The 24 x 38 standing body, also while hopping or flying.
    pub fn body(&self) -> Rect {
        body_rect(self.x, self.y)
    }

    /// The swing's hit box: 8..30 px ahead of the feet, 0..30 px up (half-open
    /// in x like the Carl boxes).
    pub fn swing_box(&self) -> Rect {
        let feet = self.y - ENEMY_HALF_HEIGHT;
        let (left, right) = if self.facing >= 0 {
            (self.x + SWING_X_MIN, self.x + SWING_X_MAX)
        } else {
            (self.x - SWING_X_MAX, self.x - SWING_X_MIN)
        };
        Rect {
            left,
            bottom: feet + SWING_Y_MIN,
            width: right - left,
            height: SWING_Y_MAX - SWING_Y_MIN,
        }
    }

    /// One fixed tick. `approved` (this goblin holds a free token slot and may
    /// close in) and `may_start` (the tell spacing allows a start now) come
    /// from the group rule. `hero_alive` false keeps goblins idle.
    pub fn step(
        &mut self,
        hero_x: f32,
        hero_alive: bool,
        solids: &[Block],
        max_x: f32,
        approved: bool,
        may_start: bool,
    ) -> GoblinStep {
        let mut out = GoblinStep::default();
        if self.hit_stop > 0 {
            // Frozen: no timer, no motion, no swing progress, no damage.
            self.hit_stop -= 1;
            return out;
        }
        match self.mode {
            GoblinMode::Corpse => {}
            GoblinMode::Down => {
                self.integrate(solids, max_x);
                self.down_ticks += 1;
                if self.down_ticks >= DOWN_TICKS {
                    self.mode = GoblinMode::Corpse;
                    self.vx = 0.0;
                }
            }
            GoblinMode::Airborne => {
                if self.integrate(solids, max_x) {
                    self.mode = GoblinMode::Down;
                    self.down_ticks = 0;
                }
            }
            GoblinMode::Launched => {
                self.flight_ticks = self.flight_ticks.saturating_add(1);
                self.contact_ticks = self.contact_ticks.saturating_sub(1);
                if self.integrate(solids, max_x) {
                    self.mode = GoblinMode::Downed;
                    self.down_ticks = 0;
                    self.launch = None;
                    self.contact_ticks = 0;
                }
            }
            GoblinMode::Downed => {
                self.integrate(solids, max_x);
                self.down_ticks += 1;
                if self.down_ticks >= DOWNED_TICKS + GETUP_TICKS {
                    self.mode = GoblinMode::Neutral;
                    self.down_ticks = 0;
                    self.vx = 0.0;
                    self.hurt_cooldown = HURT_COOLDOWN;
                }
            }
            GoblinMode::Hurt => {
                self.integrate(solids, max_x);
                self.stagger_left = self.stagger_left.saturating_sub(1);
                if self.stagger_left == 0 {
                    self.mode = GoblinMode::Neutral;
                    self.hurt_cooldown = HURT_COOLDOWN;
                }
            }
            GoblinMode::Attack => {
                self.integrate(solids, max_x);
                self.attack_tick += 1;
                out.swing_began = self.attack_tick == TELL_TICKS + 2;
                out.impact = (SMEAR_END..IMPACT_END).contains(&self.attack_tick);
                if self.attack_tick >= ATTACK_TICKS {
                    self.mode = GoblinMode::Neutral;
                    self.attack_tick = 0;
                    self.swing_hit = false;
                    self.attack_cooldown = ATTACK_COOLDOWN;
                }
            }
            GoblinMode::Neutral => {
                self.attack_cooldown = self.attack_cooldown.saturating_sub(1);
                self.hurt_cooldown = self.hurt_cooldown.saturating_sub(1);
                self.integrate(solids, max_x);
                out.started_tell = self.neutral(hero_x, hero_alive, solids, approved, may_start);
            }
        }
        out
    }

    /// Standing or walking: maybe start a tell, otherwise close in (to the
    /// trigger distance with a token, to the stand-off distance without one).
    fn neutral(
        &mut self,
        hero_x: f32,
        hero_alive: bool,
        solids: &[Block],
        approved: bool,
        may_start: bool,
    ) -> bool {
        let dx = hero_x - self.x;
        let distance = dx.abs();
        if self.ai == EnemyAi::Passive || !hero_alive || distance >= GOBLIN_SIGHT {
            return false;
        }
        let toward: i8 = if dx > 0.0 { 1 } else { -1 };
        if approved && may_start && distance <= GOBLIN_TRIGGER && self.wants_attack() {
            self.facing = toward;
            self.mode = GoblinMode::Attack;
            self.attack_tick = 0;
            self.swing_hit = false;
            return true;
        }
        let stop_at = if approved {
            GOBLIN_TRIGGER
        } else {
            GOBLIN_STANDOFF
        };
        if distance > stop_at {
            self.facing = toward;
            let candidate = self.x + f32::from(toward) * GOBLIN_WALK_SPEED * TICK_SECONDS;
            if !solid_at(solids, body_rect(candidate, self.y)) {
                self.x = candidate;
            }
        }
        false
    }

    /// Move by the knockback velocity: stopped by walls and the layout edge,
    /// gravity in the air, friction on the floor. Returns true on the tick a
    /// launched goblin touches down.
    fn integrate(&mut self, solids: &[Block], max_x: f32) -> bool {
        if self.vx != 0.0 {
            let wanted = self.x + self.vx * TICK_SECONDS;
            let candidate = wanted.clamp(ENEMY_HALF_WIDTH, max_x);
            let blocked = solid_at(solids, body_rect(candidate, self.y));
            if !blocked {
                self.x = candidate;
            }
            if blocked || candidate != wanted {
                // A wall or the edge of the layout: the goblin stops dead.
                self.vx = 0.0;
            }
        }
        if self.y > self.floor_y || self.vy > 0.0 {
            self.vy -= KNOCK_GRAVITY * TICK_SECONDS;
            self.y += self.vy * TICK_SECONDS;
            if self.y <= self.floor_y && self.vy <= 0.0 {
                self.y = self.floor_y;
                self.vy = 0.0;
                return true;
            }
        } else if self.vx != 0.0 {
            let drag = GROUND_FRICTION * TICK_SECONDS;
            self.vx = if self.vx.abs() <= drag {
                0.0
            } else {
                self.vx - drag * self.vx.signum()
            };
        }
        false
    }

    /// Fold the full state into a state hash.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(u64::from(self.x.to_bits()));
        mix(u64::from(self.y.to_bits()));
        mix(u64::from(self.health));
        mix(self.ai as u64);
        mix(self.facing as u64);
        mix(self.mode as u64);
        mix(u64::from(self.attack_tick));
        mix(u64::from(self.stagger_left));
        mix(u64::from(self.down_ticks));
        mix(u64::from(self.vx.to_bits()));
        mix(u64::from(self.vy.to_bits()));
        mix(u64::from(self.hit_stop));
        mix(u64::from(self.attack_cooldown));
        mix(u64::from(self.hurt_cooldown));
        mix(u64::from(self.swing_hit));
        // CP-02 state: mixed only while it exists, so goblins that were never
        // launched keep the hashes they had before the kick launch.
        if self.launch.is_some() || self.flight_ticks != 0 || self.contact_ticks != 0 {
            mix(self.launch.map_or(0, |id| u64::from(id) + 1));
            mix(u64::from(self.flight_ticks));
            mix(u64::from(self.contact_ticks));
        }
        // DEATH-B: a hit by a player other than slot 0 (2P). Absent for every
        // single-player replay, so their hashes stay put.
        if let Some(player) = self.hit_by
            && player != 0
        {
            mix(0x4849_5442);
            mix(u64::from(player));
        }
    }

    /// CP-02, the flier's side of a contact: `knockback_contact` for a few
    /// ticks, most of its speed gone and a short freeze. No damage.
    pub fn contact_as_flier(&mut self) {
        self.contact_ticks = CONTACT_POSE_TICKS;
        self.vx *= CONTACT_SOURCE_KEEP;
        self.hit_stop = self.hit_stop.max(CONTACT_HIT_STOP);
    }

    /// Can a launched goblin interrupt this one: a living goblin on its feet
    /// or already staggered (not flying, lying down or defeated).
    pub fn can_be_contacted(&self) -> bool {
        self.alive()
            && matches!(
                self.mode,
                GoblinMode::Neutral | GoblinMode::Attack | GoblinMode::Hurt
            )
    }

    /// CP-02, the second goblin's side: interrupted (the attack is cancelled,
    /// the token goes back), staggered and pushed `push` (+1 / -1). No damage.
    pub fn contact_as_target(&mut self, push: i8) {
        self.mode = GoblinMode::Hurt;
        self.attack_tick = 0;
        self.swing_hit = false;
        self.stagger_left = CONTACT_STAGGER;
        self.vx = f32::from(push) * CONTACT_PUSH_VX;
        self.vy = 0.0;
        self.facing = -push;
        self.hit_stop = self.hit_stop.max(CONTACT_HIT_STOP);
    }

    /// CP-03: a falling block lands on this goblin. It is defeated on the spot
    /// and lies as a `Down` / `Corpse` body (temporary stand-in art until
    /// GOB-02-CRUSH). Returns false for a goblin that is not alive.
    pub fn crush(&mut self) -> bool {
        if !self.alive() {
            return false;
        }
        self.health = 0;
        self.mode = GoblinMode::Down;
        self.down_ticks = 0;
        self.attack_tick = 0;
        self.stagger_left = 0;
        self.swing_hit = false;
        self.launch = None;
        self.contact_ticks = 0;
        self.vx = 0.0;
        self.vy = 0.0;
        self.y = self.floor_y;
        self.hit_stop = 0;
        true
    }
}

impl StrikeTarget for Goblin {
    fn hurtbox_active(&self) -> bool {
        self.alive()
    }

    fn can_be_hit(&self) -> bool {
        self.alive() && self.hit_stop == 0
    }

    fn body(&self) -> Rect {
        Goblin::body(self)
    }

    fn centre(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    /// Damage, then the reaction: the killing hit launches the goblin (down,
    /// then corpse); a hit during the tell or the recovery staggers it (the
    /// attack is cancelled); a hit during the swing only damages it. Returns the
    /// hit-stop the attacker freezes for.
    fn take_hit(&mut self, spec: &MoveSpec, attacker_facing: i8) -> (HitOutcome, u8) {
        let reaction = spec.reaction;
        let direction = f32::from(attacker_facing);
        self.health = self.health.saturating_sub(spec.damage);
        let mut stop = reaction.hit_stop;
        let outcome = if self.health == 0 {
            self.mode = GoblinMode::Airborne;
            self.attack_tick = 0;
            self.stagger_left = 0;
            self.swing_hit = false;
            self.vx = direction * LETHAL_KNOCK_VX;
            self.vy = LETHAL_KNOCK_VY;
            self.launch = None;
            self.contact_ticks = 0;
            stop += LETHAL_HIT_STOP_BONUS;
            HitOutcome::Defeated
        } else {
            let committed = self.mode == GoblinMode::Attack
                && matches!(
                    attack_phase(self.attack_tick),
                    AttackPhase::Smear | AttackPhase::Impact
                );
            if !committed && reaction.launch {
                // The kick: a living launch (D-02), no stagger.
                self.mode = GoblinMode::Launched;
                self.attack_tick = 0;
                self.stagger_left = 0;
                self.swing_hit = false;
                self.launch = None; // the scene stamps the move instance
                self.flight_ticks = 0;
                self.contact_ticks = 0;
                self.vx = (direction * reaction.knock_vx).clamp(-KNOCK_VX_MAX, KNOCK_VX_MAX);
                self.vy = reaction.knock_vy;
            } else if !committed {
                // A jab on a goblin in the air or lying: no launch state left.
                self.launch = None;
                self.flight_ticks = 0;
                self.contact_ticks = 0;
                self.mode = GoblinMode::Hurt;
                self.attack_tick = 0;
                self.stagger_left = reaction.stagger_ticks;
                self.swing_hit = false;
                self.vx = (direction * reaction.knock_vx).clamp(-KNOCK_VX_MAX, KNOCK_VX_MAX);
                self.vy = reaction.knock_vy;
            }
            HitOutcome::Hurt {
                health_left: self.health,
            }
        };
        // The scene steps goblins after Carl's strike on the same tick, so that
        // step is frozen too: the goblin loses the hit tick plus `stop` more
        // ticks, the attacker's clock `stop` ticks after the hit tick.
        self.hit_stop = stop + 1;
        (outcome, stop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{CARL_HAYMAKER, CARL_JAB};
    use crate::feel::{HAYMAKER_FEEL, JAB_FEEL};
    use crate::prototype::Material;

    const FLOOR: f32 = 119.0;
    const EDGE: f32 = 2000.0;

    fn goblin_at(x: f32) -> Goblin {
        Goblin::new(x, FLOOR, EnemyAi::Chase)
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

    fn tick(g: &mut Goblin, hero_x: f32, approved: bool) -> GoblinStep {
        g.step(hero_x, true, &[], EDGE, approved, true)
    }

    /// A goblin that has just started its tell (attack tick 0), hero at 28 px.
    fn telling() -> (Goblin, f32) {
        let hero_x = 400.0;
        let mut g = goblin_at(hero_x + 28.0);
        let step = tick(&mut g, hero_x, true);
        assert!(step.started_tell);
        assert_eq!((g.mode, g.attack_tick), (GoblinMode::Attack, 0));
        (g, hero_x)
    }

    #[test]
    fn the_attack_timeline_is_6_plus_18_tell_hit_active_26_to_29_recover_20() {
        let (mut g, hero_x) = telling();
        let mut impact = Vec::new();
        let mut began = Vec::new();
        for _ in 0..60 {
            let step = tick(&mut g, hero_x, false);
            if step.impact {
                impact.push(g.attack_tick);
            }
            if step.swing_began {
                began.push(g.attack_tick);
            }
            if g.mode != GoblinMode::Attack {
                break;
            }
        }
        assert_eq!(impact, vec![26, 27, 28, 29]);
        assert_eq!(began, vec![26]);
        // Recover 30..50, then back to neutral with the 40-tick cooldown.
        assert_eq!(g.mode, GoblinMode::Neutral);
        assert_eq!(g.attack_cooldown, ATTACK_COOLDOWN);
        assert_eq!(attack_phase(0), AttackPhase::Tell);
        assert_eq!(attack_phase(23), AttackPhase::Tell);
        assert_eq!(attack_phase(24), AttackPhase::Smear);
        assert_eq!(attack_phase(26), AttackPhase::Impact);
        assert_eq!(attack_phase(30), AttackPhase::Recover);
        assert_eq!(attack_phase(49), AttackPhase::Recover);
    }

    #[test]
    fn the_token_is_held_through_tell_and_swing_and_returned_in_recovery() {
        let (mut g, hero_x) = telling();
        for _ in 0..29 {
            assert!(g.token_held());
            tick(&mut g, hero_x, false);
        }
        assert_eq!(g.attack_tick, 29);
        assert!(g.token_held());
        tick(&mut g, hero_x, false);
        assert_eq!(g.attack_tick, 30);
        assert!(!g.token_held(), "returned when the swing ends");
    }

    #[test]
    fn the_swing_box_is_8_to_30_px_in_front_and_mirrors_with_the_facing() {
        let mut g = goblin_at(100.0);
        g.facing = 1;
        let right = g.swing_box();
        assert_eq!((right.left, right.right()), (108.0, 130.0));
        assert_eq!((right.bottom, right.top()), (100.0, 130.0));
        g.facing = -1;
        let left = g.swing_box();
        assert_eq!((left.left, left.right()), (70.0, 92.0));
    }

    #[test]
    fn a_hit_in_the_tell_cancels_the_attack_and_returns_the_token() {
        let (mut g, hero_x) = telling();
        for _ in 0..10 {
            tick(&mut g, hero_x, false);
        }
        assert!(g.token_held());
        let (outcome, stop) = g.take_hit(&CARL_JAB, 1);
        assert_eq!(outcome, HitOutcome::Hurt { health_left: 1 });
        assert_eq!(stop, JAB_FEEL.hit_stop);
        assert_eq!(g.mode, GoblinMode::Hurt);
        assert!(!g.token_held());
        assert_eq!(g.stagger_left, JAB_FEEL.stagger);
        // After the freeze and the stagger the swing never happens.
        let mut swung = false;
        for _ in 0..80 {
            let step = tick(&mut g, hero_x, false);
            swung |= step.impact;
            if g.mode == GoblinMode::Attack {
                break;
            }
        }
        assert!(!swung);
    }

    #[test]
    fn after_a_stagger_there_are_20_ticks_before_any_new_tell() {
        let hero_x = 400.0;
        let mut g = goblin_at(hero_x + 28.0);
        g.take_hit(&CARL_JAB, 1);
        let mut stagger_over_on = None;
        let mut first_tell = None;
        for n in 1..200u32 {
            // The group rule approves this goblin whenever it can attack.
            let approved = g.wants_attack();
            let step = tick(&mut g, hero_x, approved);
            if g.mode != GoblinMode::Hurt && g.hit_stop == 0 && stagger_over_on.is_none() {
                stagger_over_on = Some(n);
            }
            if step.started_tell {
                first_tell = Some(n);
                break;
            }
        }
        let (over, tell) = (stagger_over_on.unwrap(), first_tell.expect("attacks again"));
        assert!(
            tell - over >= u32::from(HURT_COOLDOWN),
            "stagger over on {over}, tell on {tell}"
        );
        // And while the cooldown runs it does not want an attack.
        let mut g = goblin_at(hero_x + 28.0);
        g.hurt_cooldown = 5;
        assert!(!g.wants_attack());
    }

    #[test]
    fn a_hit_during_the_swing_is_armoured_damage_and_hit_stop_but_no_stagger() {
        let (mut g, hero_x) = telling();
        for _ in 0..27 {
            tick(&mut g, hero_x, false);
        }
        assert_eq!(g.attack_phase(), Some(AttackPhase::Impact));
        let before_tick = g.attack_tick;
        let (outcome, stop) = g.take_hit(&CARL_JAB, 1);
        assert_eq!(outcome, HitOutcome::Hurt { health_left: 1 });
        assert_eq!(stop, JAB_FEEL.hit_stop);
        assert_eq!(g.mode, GoblinMode::Attack, "the swing is committed");
        assert_eq!(g.attack_tick, before_tick);
        assert_eq!(g.vx, 0.0, "no knockback during the swing");
        // A killing hit still defeats it and cancels everything.
        g.hit_stop = 0;
        let (outcome, _) = g.take_hit(&CARL_JAB, 1);
        assert_eq!(outcome, HitOutcome::Defeated);
        assert_eq!(g.mode, GoblinMode::Airborne);
        assert!(!g.token_held());
    }

    /// Hit-stop: no timer, no motion and no swing progress of the frozen
    /// goblin advances; it resumes exactly where it stopped.
    #[test]
    fn hit_stop_freezes_every_timer_and_motion_then_resumes_where_it_stopped() {
        let hero_x = 400.0;
        let mut g = goblin_at(hero_x + 28.0);
        g.attack_cooldown = 7;
        g.hurt_cooldown = 3;
        let (outcome, stop) = g.take_hit(&CARL_HAYMAKER, 1);
        assert_eq!(outcome, HitOutcome::Hurt { health_left: 1 });
        assert_eq!(stop, HAYMAKER_FEEL.hit_stop);
        // The confirming tick itself steps the goblin once (it is stepped
        // after the strike): that is the first frozen tick.
        tick(&mut g, hero_x, false);
        let frozen = g.clone();
        for _ in 0..stop {
            tick(&mut g, hero_x, false);
            assert_eq!(g.x, frozen.x);
            assert_eq!(g.y, frozen.y);
            assert_eq!(g.stagger_left, HAYMAKER_FEEL.stagger);
            assert_eq!((g.attack_cooldown, g.hurt_cooldown), (7, 3));
            assert_eq!(g.vx, frozen.vx, "the impulse waits too");
        }
        assert_eq!(g.hit_stop, 0);
        // The first tick after the freeze moves and counts down.
        tick(&mut g, hero_x, false);
        assert_eq!(g.stagger_left, HAYMAKER_FEEL.stagger - 1);
        assert!(g.x > frozen.x, "the knockback starts after the freeze");
    }

    #[test]
    fn a_swing_frozen_by_hit_stop_deals_no_damage_and_resumes_the_same_ticks_later() {
        // Reference: the swing's first impact tick without any freeze.
        let (mut free, hero_x) = telling();
        let mut free_first = None;
        for n in 1..40 {
            if tick(&mut free, hero_x, false).impact && free_first.is_none() {
                free_first = Some(n);
            }
        }
        // The same swing hit (armoured) during its smear freezes for `stop`
        // ticks and its first impact tick comes exactly that late.
        let (mut hit, hero_x) = telling();
        for _ in 0..25 {
            tick(&mut hit, hero_x, false);
        }
        let (_, stop) = hit.take_hit(&CARL_JAB, 1);
        // The target's own step on the hit tick is skipped as well (the scene
        // steps goblins after Carl's strike): it loses `stop + 1` ticks.
        tick(&mut hit, hero_x, false);
        let mut first = None;
        let mut impacts_while_frozen = 0;
        for n in 27..80 {
            let was_frozen = hit.hit_stop > 0;
            let step = tick(&mut hit, hero_x, false);
            if step.impact {
                impacts_while_frozen += u32::from(was_frozen);
                first.get_or_insert(n);
            }
        }
        assert_eq!(impacts_while_frozen, 0, "no damage window opens frozen");
        assert_eq!(first.unwrap() - free_first.unwrap(), u32::from(stop) + 1);
    }

    #[test]
    fn a_goblin_in_hit_stop_cannot_be_hit_again() {
        let mut g = goblin_at(430.0);
        assert!(g.can_be_hit());
        g.take_hit(&CARL_JAB, 1);
        assert!(!g.can_be_hit(), "frozen by the hit it just took");
        for _ in 0..=JAB_FEEL.hit_stop {
            tick(&mut g, 400.0, false);
        }
        assert!(g.can_be_hit());
    }

    #[test]
    fn the_killing_hit_launches_it_down_then_a_corpse_with_no_hurtbox_or_threat() {
        let hero_x = 400.0;
        let mut g = goblin_at(hero_x + 28.0);
        g.take_hit(&CARL_JAB, 1);
        g.hit_stop = 0;
        let (outcome, stop) = g.take_hit(&CARL_HAYMAKER, 1);
        assert_eq!(outcome, HitOutcome::Defeated);
        assert_eq!(stop, HAYMAKER_FEEL.hit_stop + LETHAL_HIT_STOP_BONUS);
        assert_eq!(g.mode, GoblinMode::Airborne);
        assert_eq!(g.vx, LETHAL_KNOCK_VX);
        assert!(!g.alive() && !g.hurtbox_active() && !g.can_be_hit());
        assert!(!g.wants_attack() && !g.token_held());
        let mut modes = vec![g.mode];
        let start_x = g.x;
        for _ in 0..200 {
            let step = tick(&mut g, hero_x, true);
            assert!(
                !step.impact && !step.started_tell,
                "a defeated goblin never attacks"
            );
            if modes.last() != Some(&g.mode) {
                modes.push(g.mode);
            }
        }
        assert_eq!(
            modes,
            vec![GoblinMode::Airborne, GoblinMode::Down, GoblinMode::Corpse]
        );
        assert_eq!(g.y, g.floor_y, "landed");
        assert!(g.x > start_x + 20.0, "launched away from Carl");
        // A corpse stays put and takes no further hits.
        let rest = g.x;
        for _ in 0..30 {
            tick(&mut g, hero_x, true);
        }
        assert_eq!(g.x, rest);
        assert!(!g.can_be_hit());
    }

    #[test]
    fn knockback_is_bounded_and_stopped_by_walls_and_the_layout_edge() {
        // Open floor: the lethal launch is bounded (about 75 px, never more
        // than 95).
        let mut g = goblin_at(430.0);
        g.take_hit(&CARL_JAB, 1);
        g.hit_stop = 0;
        g.take_hit(&CARL_JAB, 1);
        for _ in 0..200 {
            tick(&mut g, 400.0, false);
        }
        let travelled = g.x - 430.0;
        assert!((60.0..95.0).contains(&travelled), "travelled {travelled}");
        // A wall 30 px in front: the goblin never enters it and stops dead.
        let blocks = [wall(460.0)];
        let mut g = goblin_at(430.0);
        g.take_hit(&CARL_JAB, 1);
        g.hit_stop = 0;
        g.take_hit(&CARL_JAB, 1);
        for _ in 0..200 {
            g.step(400.0, true, &blocks, EDGE, false, true);
            assert!(
                g.x + ENEMY_HALF_WIDTH <= 460.0 + 0.001,
                "goblin inside the wall at {}",
                g.x
            );
        }
        assert!(g.x > 440.0, "it travelled up to the wall");
        // The layout edge stops it as well.
        let mut g = goblin_at(430.0);
        g.take_hit(&CARL_JAB, 1);
        g.hit_stop = 0;
        g.take_hit(&CARL_JAB, 1);
        for _ in 0..200 {
            g.step(400.0, true, &[], 445.0, false, true);
        }
        assert!(g.x <= 445.0);
        // A huge impulse is clamped (bounded knockback).
        let mut spec = CARL_HAYMAKER;
        spec.reaction.knock_vx = 5000.0;
        let mut g = goblin_at(430.0);
        g.take_hit(&spec, 1);
        assert_eq!(g.vx, KNOCK_VX_MAX);
        // Knocked away from the attacker: a hit from the right pushes left.
        let mut g = goblin_at(430.0);
        g.take_hit(&CARL_HAYMAKER, -1);
        assert!(g.vx < 0.0);
    }

    #[test]
    fn the_haymaker_hop_is_small_and_lands_about_12_px_away() {
        let mut g = goblin_at(430.0);
        g.take_hit(&CARL_HAYMAKER, 1);
        let mut peak = 0.0f32;
        for _ in 0..80 {
            tick(&mut g, 400.0, false);
            peak = peak.max(g.y - g.floor_y);
        }
        assert!(peak > 0.5 && peak < 4.0, "small hop, peak {peak}");
        let pushed = g.x - 430.0;
        assert!((9.0..16.0).contains(&pushed), "pushed {pushed}");
    }

    #[test]
    fn a_goblin_without_a_token_stops_at_the_stand_off_and_one_with_it_closes_in() {
        let hero_x = 400.0;
        let mut waiting = goblin_at(hero_x + 150.0);
        let mut approved = goblin_at(hero_x + 150.0);
        for _ in 0..300 {
            waiting.step(hero_x, true, &[], EDGE, false, true);
            approved.step(hero_x, true, &[], EDGE, true, false);
        }
        let stand_off = waiting.x - hero_x;
        assert!((55.0..=60.5).contains(&stand_off), "stand-off {stand_off}");
        let close = approved.x - hero_x;
        assert!((29.0..=30.5).contains(&close), "closed in to {close}");
        assert_eq!(
            approved.mode,
            GoblinMode::Neutral,
            "spacing denied the start"
        );
    }

    #[test]
    fn replays_of_the_same_hits_hash_the_same() {
        let run = || {
            let mut g = goblin_at(428.0);
            let mut hash = 0u64;
            for n in 0..120u32 {
                if n == 10 {
                    g.take_hit(&CARL_JAB, 1);
                }
                if n == 40 {
                    g.take_hit(&CARL_HAYMAKER, 1);
                }
                let approved = g.wants_attack();
                tick(&mut g, 400.0, approved);
                g.mix_into(&mut |v| hash = hash.rotate_left(5) ^ v);
            }
            hash
        };
        assert_eq!(run(), run());
    }
}
