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

use crate::cf02::Cf02Tuning;
use crate::combat::{
    ENEMY_HALF_HEIGHT, ENEMY_HALF_WIDTH, ENEMY_HEALTH, EnemyAi, HitOutcome, MoveId, MoveSpec,
    StrikeTarget, body_rect, solid_at,
};
use crate::donut_moves::{HitStopSplit, LightGuard, TargetState, hash_guard};
use crate::feel::{
    ATTACK_COOLDOWN, ATTACK_TICKS, BLAST_KNOCK_VX, BLAST_KNOCK_VX_POINT_BLANK, BLAST_KNOCK_VY,
    BLAST_KNOCK_VY_POINT_BLANK, CONTACT_HIT_STOP, CONTACT_POSE_TICKS, CONTACT_PUSH_VX,
    CONTACT_SOURCE_KEEP, CONTACT_STAGGER, DOWN_TICKS, DOWNED_TICKS, GETUP_TICKS, GOBLIN_SIGHT,
    GOBLIN_STANDOFF, GOBLIN_TRIGGER, GOBLIN_WALK_SPEED, GROUND_FRICTION, HURT_COOLDOWN, IMPACT_END,
    KNOCK_GRAVITY, KNOCK_VX_MAX, LETHAL_HIT_STOP_BONUS, LETHAL_KNOCK_VX, LETHAL_KNOCK_VY,
    SMEAR_END, SWING_X_MAX, SWING_X_MIN, SWING_Y_MAX, SWING_Y_MIN, TELL_TICKS,
};
use crate::prototype::{Block, Rect, TICK_SECONDS};

/// DEATH-D: how far ahead of the pivot column (the neck, on GOB-01 frame 0) the
/// head probe sits, in the facing direction (the head leads).
pub const NECK_PROBE_FORWARD: f32 = 4.0;
/// How far behind the pivot column the torso probe sits.
pub const NECK_PROBE_BACK: f32 = 4.0;

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
    /// CF02-Q63: a panicking goblin could not move because a solid or the layout
    /// edge was in the way this tick (always false in the legacy game).
    pub blocked: bool,
}

/// CF02-G: what the morale layer ([`crate::morale`]) asks of one goblin for one
/// tick. [`Self::LEGACY`] is the unchanged goblin; the scene builds the others only
/// for a run with morale. The goblin's own state machine, tell and swing timings
/// never change: a drive only picks the walk speed, whether the goblin stands
/// still, whether it flees and the cooldown it sets after an attack (Q61).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindDrive {
    /// Walk speed in px/s (legacy 62; enraged 81).
    pub walk_speed: f32,
    /// Stand still: no walking and no new tell (hesitate, taunt, back-off, the
    /// first-approach delay).
    pub hold: bool,
    /// Panic: flee in this direction (+1 right, -1 left) at this speed in px/s;
    /// a fleeing goblin starts no attack.
    pub flee: Option<(i8, f32)>,
    /// Cooldown after an attack (legacy 40; enraged 24).
    pub attack_cooldown: u8,
}

impl MindDrive {
    /// The unchanged goblin.
    pub const LEGACY: Self = Self {
        walk_speed: GOBLIN_WALK_SPEED,
        hold: false,
        flee: None,
        attack_cooldown: ATTACK_COOLDOWN,
    };
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
    /// CF02-Q31: this swing already counted as a hit on the rotten shore (one per
    /// swing, however many impact ticks overlap the strike zone). Set by the scene,
    /// cleared when the goblin leaves the impact phase; mixed into the hash only
    /// while set, so no legacy hash sees it.
    pub(crate) shore_counted: bool,
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
    /// WORK-023 (Q10-06): the light guard a swat stagger sets. A guarded goblin
    /// glances further swats; Carl's hits and the pounce still interrupt it.
    /// `None` for every goblin Donut never swatted, so nothing of it is mixed
    /// into a hash then.
    pub guard: Option<LightGuard>,
    /// COMBAT-FUN-02: the pinned tuning with the master switch on, `Some` only for
    /// a goblin of a run under `cf02_rules` (`None` everywhere else, so every
    /// legacy rule and hash is untouched). Not mixed into the hash: it is run
    /// configuration, constant for the whole run.
    pub(crate) cf02: Option<&'static Cf02Tuning>,
    /// CF02-Q38: ticks the down timer is still paused by a pounce hold.
    pub(crate) hold_left: u8,
    /// CF02-Q38: this down instance already used its hold (a kick relaunch starts
    /// a new down instance).
    pub(crate) hold_used: bool,
    /// CF02-Q51: an air kick already relaunched this flight (once per original
    /// launch instance).
    pub(crate) relaunched: bool,
    /// CF02-Q10: the kick instance a body killed by a lethal kick keeps, so it can
    /// knock a living goblin down (`Airborne` only; cleared when it lands). The
    /// scene stamps it from the lethal `Hit`.
    pub(crate) bowl: Option<u32>,
    /// CF02-Q11: set by [`Self::integrate`] on the tick a solid (not the layout
    /// edge) stopped this goblin: the abs(vx) it had before the stop. Cleared at
    /// the start of every step and read by the scene on the same tick, so it is
    /// transient and never hashed.
    pub(crate) wall_stop: Option<f32>,
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
            shore_counted: false,
            launch: None,
            flight_ticks: 0,
            contact_ticks: 0,
            hit_by: None,
            guard: None,
            cf02: None,
            hold_left: 0,
            hold_used: false,
            relaunched: false,
            bowl: None,
            wall_stop: None,
        }
    }

    /// Run this goblin under `cf02_rules` (the scene sets it when it builds the
    /// goblin; `None` is the legacy goblin).
    #[must_use]
    pub(crate) fn with_cf02(mut self, cf02: Option<&'static Cf02Tuning>) -> Self {
        self.cf02 = cf02;
        self
    }

    /// Ticks a living goblin lies after a launch: [`DOWNED_TICKS`], or the CF02
    /// value (Q04).
    fn downed_ticks(&self) -> u16 {
        self.cf02.map_or(DOWNED_TICKS, |t| t.q04_downed_ticks)
    }

    /// CF02-Q03: lying on the floor (`Downed`, before the get-up). Under
    /// `cf02_rules` only; it narrows the hurtbox and pauses under a Q38 hold.
    /// Read by the COMPANION view builder (Q15: assist pounce on a lying goblin).
    pub(crate) fn lying(&self) -> bool {
        self.cf02.is_some()
            && self.mode == GoblinMode::Downed
            && self.down_ticks < self.downed_ticks()
    }

    /// CF02-Q04, Q38: ticks the goblin still lies before it starts to get up,
    /// `Some` only while [`Self::lying`]. It does **not** include a running hold:
    /// the down timer is paused for `hold_left` more ticks first, so the time
    /// until the get-up begins is `down_ticks_left() + hold_left` (and a hold not
    /// yet used can add `q38_hold_ticks` once). Read by the COMPANION view
    /// builder (`Lying { down_ticks_left, hold_used }`).
    pub(crate) fn down_ticks_left(&self) -> Option<u16> {
        self.lying()
            .then(|| self.downed_ticks().saturating_sub(self.down_ticks))
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

    /// What the Donut move domain sees of this goblin (Q10 section 1): the
    /// attack phase from the real `attack_tick` (the Impact is latched by the
    /// guard, never re-derived), and a defeated goblin, lying or a corpse, is
    /// no target.
    pub fn target_state(&self) -> TargetState {
        match self.mode {
            GoblinMode::Neutral => TargetState::Neutral,
            GoblinMode::Attack => match attack_phase(self.attack_tick) {
                AttackPhase::Tell => TargetState::Tell,
                AttackPhase::Smear => TargetState::Smear,
                AttackPhase::Impact => TargetState::Impact,
                AttackPhase::Recover => TargetState::Recovery,
            },
            GoblinMode::Hurt => TargetState::Staggered,
            GoblinMode::Launched => TargetState::Launched,
            GoblinMode::Downed => TargetState::Downed,
            GoblinMode::Airborne => TargetState::Airborne,
            GoblinMode::Down | GoblinMode::Corpse => TargetState::Defeated,
        }
    }

    /// One tick of the light guard (call once per goblin tick, after
    /// [`Self::step`]): it is dropped once it releases.
    pub fn observe_guard(&mut self) {
        let state = self.target_state();
        if let Some(guard) = self.guard.as_mut()
            && guard.observe(state)
        {
            self.guard = None;
        }
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
        self.step_with(
            hero_x,
            hero_alive,
            solids,
            max_x,
            approved,
            may_start,
            &MindDrive::LEGACY,
        )
    }

    /// [`Self::step`] under a morale drive (CF02-G). With [`MindDrive::LEGACY`]
    /// this is exactly [`Self::step`].
    #[allow(clippy::too_many_arguments)]
    pub fn step_with(
        &mut self,
        hero_x: f32,
        hero_alive: bool,
        solids: &[Block],
        max_x: f32,
        approved: bool,
        may_start: bool,
        drive: &MindDrive,
    ) -> GoblinStep {
        let mut out = GoblinStep::default();
        self.wall_stop = None;
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
                    self.bowl = None;
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
                    // A new down instance: its one hold is available again.
                    self.hold_left = 0;
                    self.hold_used = false;
                    self.relaunched = false;
                }
            }
            GoblinMode::Downed => {
                self.integrate(solids, max_x);
                if self.hold_left > 0 {
                    // CF02-Q38: a pounce holds the lying goblin down; the timer waits.
                    self.hold_left -= 1;
                } else {
                    self.down_ticks += 1;
                }
                if self.down_ticks >= self.downed_ticks() + GETUP_TICKS {
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
                    self.attack_cooldown = drive.attack_cooldown;
                }
            }
            GoblinMode::Neutral => {
                self.attack_cooldown = self.attack_cooldown.saturating_sub(1);
                self.hurt_cooldown = self.hurt_cooldown.saturating_sub(1);
                self.integrate(solids, max_x);
                if let Some((dir, speed)) = drive.flee {
                    out.blocked = !self.flee(dir, speed, solids, max_x);
                } else if !drive.hold {
                    out.started_tell = self.neutral(
                        hero_x,
                        hero_alive,
                        solids,
                        approved,
                        may_start,
                        drive.walk_speed,
                    );
                }
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
        walk_speed: f32,
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
            let candidate = self.x + f32::from(toward) * walk_speed * TICK_SECONDS;
            if !solid_at(solids, body_rect(candidate, self.y)) {
                self.x = candidate;
            }
        }
        false
    }

    /// CF02-Q61, Q63: one tick of fleeing in `dir` at `speed` px/s. The goblin
    /// faces where it runs. Returns false when a solid or the layout edge stopped
    /// it (it did not move).
    fn flee(&mut self, dir: i8, speed: f32, solids: &[Block], max_x: f32) -> bool {
        self.facing = dir;
        let wanted = self.x + f32::from(dir) * speed * TICK_SECONDS;
        let candidate = wanted.clamp(ENEMY_HALF_WIDTH, max_x);
        if candidate == self.x || solid_at(solids, body_rect(candidate, self.y)) {
            return false;
        }
        self.x = candidate;
        candidate == wanted
    }

    /// CF02-Q64: a hop of `px` px in `dir` (the Coward's back-off), stopped by a
    /// solid or the layout edge. The goblin keeps facing the hero.
    pub(crate) fn hop(&mut self, dir: i8, px: f32, solids: &[Block], max_x: f32) {
        let candidate = (self.x + f32::from(dir) * px).clamp(ENEMY_HALF_WIDTH, max_x);
        if !solid_at(solids, body_rect(candidate, self.y)) {
            self.x = candidate;
        }
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
            } else {
                // CF02-Q11: a solid, not the layout edge, stopped the flight.
                self.wall_stop = Some(self.vx.abs());
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
        if self.shore_counted {
            mix(0x5348_5357); // "SHSW"
        }
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
        // WORK-023: the light guard, mixed only while it is set.
        hash_guard(&self.guard, mix);
        // CF02-Q38: the hold, mixed only while it exists.
        if self.hold_left > 0 || self.hold_used {
            mix(0x484f_4c44);
            mix(u64::from(self.hold_left));
            mix(u64::from(self.hold_used));
        }
        // CF02-Q51: the relaunch, mixed only while it is spent.
        if self.relaunched {
            mix(0x5245_4c4e);
        }
        // CF02-Q10: the bowling body's kick instance, mixed only while it exists.
        if let Some(launch) = self.bowl {
            mix(0x424f_574c);
            mix(u64::from(launch));
        }
    }

    /// CP-02, the flier's side of a contact: `knockback_contact` for a few
    /// ticks, most of its speed gone and a short freeze. No damage.
    pub fn contact_as_flier(&mut self) {
        self.contact_ticks = CONTACT_POSE_TICKS;
        self.vx *= CONTACT_SOURCE_KEEP;
        self.hit_stop = self.hit_stop.max(CONTACT_HIT_STOP);
    }

    /// CF02-Q09, the flier's side under `cf02_rules`: `knockback_contact` for a few
    /// ticks, a share of its speed kept and a short freeze. No damage (Q12).
    pub(crate) fn contact_as_flier_cf02(&mut self, t: &Cf02Tuning) {
        self.contact_ticks = CONTACT_POSE_TICKS;
        self.vx *= f32::from(t.q09_flier_keep_vx_pct) / 100.0;
        self.hit_stop = self.hit_stop.max(t.q09_flier_hit_stop);
    }

    /// CF02-Q10, the side of a killed body that bowls: the same slowdown and freeze
    /// as a living flier, but no contact pose (the body keeps its death art).
    pub(crate) fn contact_as_body_cf02(&mut self, t: &Cf02Tuning) {
        self.vx *= f32::from(t.q09_flier_keep_vx_pct) / 100.0;
        self.hit_stop = self.hit_stop.max(t.q09_flier_hit_stop);
    }

    /// CF02-Q09, the second goblin's side when the flier is fast enough: its attack
    /// ends (the token goes back), and it hops and falls as a living `Launched`
    /// goblin **without a launch instance**, so it can never chain into a third
    /// goblin; it then lies for the CF02 lying time. No damage (Q12).
    pub(crate) fn contact_knockdown(&mut self, push: i8, t: &Cf02Tuning) {
        self.mode = GoblinMode::Launched;
        self.attack_tick = 0;
        self.swing_hit = false;
        self.stagger_left = 0;
        self.launch = None;
        self.flight_ticks = 0;
        self.contact_ticks = 0;
        self.hold_left = 0;
        self.hold_used = false;
        self.relaunched = false;
        self.vx = f32::from(push) * f32::from(t.q09_contact_push_x);
        self.vy = f32::from(t.q09_contact_hop_y);
        self.facing = -push;
        self.hit_stop = self.hit_stop.max(t.q09_flier_hit_stop);
    }

    /// CF02-Q11: a kicked living flier was stopped by a solid. It takes the wall
    /// damage and keeps no flight: with health left it drops (a `Launched` goblin
    /// with no launch instance and no sideways speed, so it falls straight down,
    /// lands `Downed` and cannot slam or contact again); at 0 health it is
    /// defeated in place and crumples. Returns true when it was defeated.
    pub(crate) fn wall_slam(&mut self, t: &Cf02Tuning) -> bool {
        self.health = self.health.saturating_sub(t.q11_wall_slam_damage);
        self.launch = None;
        self.flight_ticks = 0;
        self.contact_ticks = 0;
        self.relaunched = false;
        self.vx = 0.0;
        self.vy = self.vy.min(0.0);
        self.hit_stop = self.hit_stop.max(t.q09_flier_hit_stop);
        if self.health > 0 {
            if self.y <= self.floor_y {
                // Already on the floor: nothing to fall, so it lies at once.
                self.mode = GoblinMode::Downed;
                self.down_ticks = 0;
            }
            return false;
        }
        self.attack_tick = 0;
        self.stagger_left = 0;
        self.swing_hit = false;
        if self.y <= self.floor_y {
            self.y = self.floor_y;
            self.mode = GoblinMode::Down;
            self.down_ticks = 0;
        } else {
            self.mode = GoblinMode::Airborne;
        }
        true
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
        self.contact_stagger(push, CONTACT_STAGGER, CONTACT_PUSH_VX, CONTACT_HIT_STOP);
    }

    /// The legacy second-goblin reaction with its three numbers as parameters
    /// (CF02-Q09 below the knockdown threshold passes the tuning's).
    pub(crate) fn contact_stagger(&mut self, push: i8, stagger: u8, push_vx: f32, hit_stop: u8) {
        self.mode = GoblinMode::Hurt;
        self.attack_tick = 0;
        self.swing_hit = false;
        self.stagger_left = stagger;
        self.vx = f32::from(push) * push_vx;
        self.vy = 0.0;
        self.facing = -push;
        self.hit_stop = self.hit_stop.max(hit_stop);
    }

    /// DEATH-D: the x of the head and of the torso of an **upright** goblin
    /// (`Neutral`, `Attack` or `Hurt`): `(head, torso)`, 4 px ahead of and
    /// behind the pivot column in the facing direction. `None` for a goblin
    /// that is flying, lying or defeated (a lying goblin under an edge is a
    /// plain crush).
    pub fn neck_probes(&self) -> Option<(f32, f32)> {
        if !self.alive()
            || !matches!(
                self.mode,
                GoblinMode::Neutral | GoblinMode::Attack | GoblinMode::Hurt
            )
        {
            return None;
        }
        let f = f32::from(self.facing);
        Some((
            self.x + f * NECK_PROBE_FORWARD,
            self.x - f * NECK_PROBE_BACK,
        ))
    }

    /// DEATH-D: a blast charge killed this goblin. It is defeated on the spot
    /// and thrown `away` (+1 / -1) from the charge, as `Airborne` like the
    /// lethal branch of `take_hit` (down, then corpse). Returns false, and
    /// changes nothing, for a goblin that is not alive.
    pub fn blast(&mut self, away: i8, point_blank: bool) -> bool {
        if !self.alive() {
            return false;
        }
        let (vx, vy) = if point_blank {
            (BLAST_KNOCK_VX_POINT_BLANK, BLAST_KNOCK_VY_POINT_BLANK)
        } else {
            (BLAST_KNOCK_VX, BLAST_KNOCK_VY)
        };
        self.health = 0;
        self.mode = GoblinMode::Airborne;
        self.attack_tick = 0;
        self.stagger_left = 0;
        self.swing_hit = false;
        self.launch = None;
        self.contact_ticks = 0;
        self.hit_stop = 0;
        self.vx = (f32::from(away) * vx).clamp(-KNOCK_VX_MAX, KNOCK_VX_MAX);
        self.vy = vy;
        true
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

    /// The hurtbox: the 24 x 38 standing body, or (CF02-Q03) the low box of a goblin
    /// lying on the floor, so a jab and a haymaker pass over it while a kick, a
    /// stomp and a pounce reach it.
    fn body(&self) -> Rect {
        if self.lying()
            && let Some(t) = self.cf02
        {
            let lying = t.q03_lying_box;
            let feet = self.y - ENEMY_HALF_HEIGHT;
            return Rect {
                left: self.x + f32::from(lying.x0),
                bottom: feet + f32::from(lying.y0),
                width: f32::from(lying.x1 - lying.x0),
                height: f32::from(lying.y1 - lying.y0),
            };
        }
        Goblin::body(self)
    }

    fn centre(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    fn health_for_glance(&self) -> u8 {
        self.health
    }

    /// Damage, then the reaction: the killing hit launches the goblin (down,
    /// then corpse); a hit during the tell or the recovery staggers it (the
    /// attack is cancelled); a hit during the swing only damages it. Returns the
    /// hit-stop the attacker freezes for.
    fn take_hit(&mut self, spec: &MoveSpec, attacker_facing: i8) -> (HitOutcome, u8) {
        let (outcome, split) = self.take_hit_split(spec, attacker_facing, HitStopSplit::legacy);
        (outcome, split.attacker)
    }
}

impl Goblin {
    /// The knock `(vx, vy)` of a defeating hit by move `id`, or `None` for a body
    /// that stays where it stands (the stomp, Q02). Legacy without CF02; with it a
    /// kick that kills throws the body harder (Q06).
    fn cf02_lethal_knock(&self, id: MoveId) -> Option<(f32, f32)> {
        let Some(t) = self.cf02 else {
            return Some((LETHAL_KNOCK_VX, LETHAL_KNOCK_VY));
        };
        match id {
            MoveId::AirKick => Some((
                f32::from(t.q51_lethal_launch_x),
                f32::from(t.q51_lethal_launch_y),
            )),
            MoveId::Stomp | MoveId::DiveStomp => Some((
                f32::from(t.q02_stomp_lethal_knock_x),
                f32::from(t.q02_stomp_lethal_knock_y),
            ))
            .filter(|&(vx, vy)| vx != 0.0 || vy != 0.0),
            MoveId::PushKick | MoveId::FollowKick => Some((
                f32::from(t.q06_lethal_kick_knock_x),
                f32::from(t.q06_lethal_kick_knock_y),
            )),
            _ => Some((
                f32::from(t.q06_lethal_melee_knock_x),
                f32::from(t.q06_lethal_melee_knock_y),
            )),
        }
    }

    /// CF02-Q34: a non-lethal hit by move `id` on a lying goblin leaves it down
    /// (every hit but the two kicks, which relaunch it).
    fn keeps_down(&self, id: MoveId) -> bool {
        self.cf02.is_some_and(|t| t.on(t.q34_keep_down))
            && !matches!(id, MoveId::PushKick | MoveId::FollowKick)
    }

    /// [`StrikeTarget::take_hit`] with the freeze of the two fighters as two
    /// numbers (Q10 hit-stop split). `split` maps the move's hit-stop (plus the
    /// lethal bonus) to the freezes: the legacy rule is
    /// [`HitStopSplit::legacy`] (the attacker `stop`, the goblin `stop + 1`
    /// because it is stepped after the strike on the same tick); the swat
    /// freezes only Donut (`target: 0`), so a Carl jab on the next tick still
    /// connects. The goblin stores `target` directly.
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
        let was_lying = self.lying();
        let outcome = if self.health == 0 {
            self.attack_tick = 0;
            self.stagger_left = 0;
            self.swing_hit = false;
            self.launch = None;
            self.contact_ticks = 0;
            self.hold_left = 0;
            stop += LETHAL_HIT_STOP_BONUS;
            match self.cf02_lethal_knock(spec.id) {
                // CF02-Q02: a stomp finish leaves the body where it stands.
                None => {
                    self.mode = GoblinMode::Down;
                    self.down_ticks = 0;
                    self.vx = 0.0;
                    self.vy = 0.0;
                    self.y = self.floor_y;
                }
                Some((vx, vy)) => {
                    self.mode = GoblinMode::Airborne;
                    self.vx = direction * vx;
                    self.vy = vy;
                }
            }
            HitOutcome::Defeated
        } else {
            let committed = self.mode == GoblinMode::Attack
                && matches!(
                    attack_phase(self.attack_tick),
                    AttackPhase::Smear | AttackPhase::Impact
                );
            if was_lying && self.keeps_down(spec.id) {
                // CF02-Q34: a non-lethal hit other than a kick deals its damage and
                // leaves the goblin lying, its timer unchanged. Q38: the one
                // exception is Donut's pounce, which holds the timer.
                if spec.id == MoveId::Pounce
                    && let Some(t) = self.cf02
                    && t.q38_hold_ticks > 0
                    && !(t.q38_hold_once_per_down && self.hold_used)
                {
                    self.hold_left = t.q38_hold_ticks;
                    self.hold_used = true;
                }
            } else if !committed
                && let Some(t) = self.cf02
                && spec.id == MoveId::AirKick
                && self.mode == GoblinMode::Launched
            {
                // CF02-Q51: an air kick on a flying goblin relaunches it as a new
                // launch instance (the scene stamps it), once per original launch;
                // a second one only deals its damage.
                if !self.relaunched {
                    self.relaunched = true;
                    self.launch = None;
                    self.flight_ticks = 0;
                    self.contact_ticks = 0;
                    self.vx = (direction * f32::from(t.q51_relaunch_x))
                        .clamp(-KNOCK_VX_MAX, KNOCK_VX_MAX);
                    self.vy = f32::from(t.q51_relaunch_y);
                }
            } else if !committed && self.cf02.is_some() && spec.id == MoveId::DiveStomp {
                // CF02-Q52: the dive knocks a goblin straight down (no hop).
                self.mode = GoblinMode::Downed;
                self.down_ticks = 0;
                self.attack_tick = 0;
                self.stagger_left = 0;
                self.swing_hit = false;
                self.launch = None;
                self.flight_ticks = 0;
                self.contact_ticks = 0;
                self.hold_left = 0;
                self.hold_used = false;
                self.relaunched = false;
                self.vx = 0.0;
                self.vy = 0.0;
                self.y = self.floor_y;
            } else if !committed && reaction.launch {
                // The kick: a living launch (D-02), no stagger.
                self.relaunched = false;
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
        let split = split(stop);
        self.hit_stop = split.target;
        (outcome, split)
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

    // ---- DEATH-D: the neck probes and the blast throw ----

    #[test]
    fn the_neck_probes_exist_only_for_an_upright_living_goblin() {
        for (mode, health, upright) in [
            (GoblinMode::Neutral, 2, true),
            (GoblinMode::Attack, 2, true),
            (GoblinMode::Hurt, 2, true),
            (GoblinMode::Launched, 2, false),
            (GoblinMode::Downed, 2, false),
            (GoblinMode::Airborne, 0, false),
            (GoblinMode::Down, 0, false),
            (GoblinMode::Corpse, 0, false),
            // A defeated goblin is never upright, whatever its mode says.
            (GoblinMode::Neutral, 0, false),
        ] {
            let mut g = goblin_at(500.0);
            g.mode = mode;
            g.health = health;
            assert_eq!(g.neck_probes().is_some(), upright, "{mode:?}");
        }
    }

    #[test]
    fn the_head_leads_in_the_facing_direction() {
        let mut g = goblin_at(500.0);
        g.facing = -1;
        assert_eq!(g.neck_probes(), Some((496.0, 504.0)));
        g.facing = 1;
        assert_eq!(g.neck_probes(), Some((504.0, 496.0)));
        assert_eq!((NECK_PROBE_FORWARD, NECK_PROBE_BACK), (4.0, 4.0));
    }

    #[test]
    fn a_blast_throws_the_goblin_dead_and_changes_only_the_spec_fields() {
        let mut g = goblin_at(500.0);
        g.mode = GoblinMode::Attack;
        g.attack_tick = 10;
        g.stagger_left = 3;
        g.swing_hit = true;
        g.launch = Some(4);
        g.contact_ticks = 2;
        g.hit_stop = 5;
        g.flight_ticks = 7;
        g.down_ticks = 3;
        g.attack_cooldown = 9;
        g.hurt_cooldown = 4;
        g.hit_by = Some(0);
        let before = g.clone();
        assert!(g.blast(1, true));
        assert_eq!(g.health, 0);
        assert_eq!(g.mode, GoblinMode::Airborne);
        assert_eq!(
            (g.attack_tick, g.stagger_left, g.contact_ticks, g.hit_stop),
            (0, 0, 0, 0)
        );
        assert!(!g.swing_hit);
        assert_eq!(g.launch, None);
        assert_eq!((g.vx, g.vy), (220.0, 320.0), "point blank, away +1");
        // Everything else stays, as in the lethal branch of take_hit.
        assert_eq!(
            (
                g.x,
                g.y,
                g.facing,
                g.flight_ticks,
                g.down_ticks,
                g.attack_cooldown,
                g.hurt_cooldown,
                g.hit_by
            ),
            (
                before.x,
                before.y,
                before.facing,
                before.flight_ticks,
                before.down_ticks,
                before.attack_cooldown,
                before.hurt_cooldown,
                before.hit_by
            )
        );
        // The ring throw: slower and lower, away -1.
        let mut ring = goblin_at(500.0);
        assert!(ring.blast(-1, false));
        assert_eq!((ring.vx, ring.vy), (-180.0, 260.0));
        // A goblin that is not alive is left alone.
        let dead = g.clone();
        assert!(!g.blast(-1, false));
        assert_eq!(g, dead);
    }

    #[test]
    fn about_eight_percent_of_block_kills_are_edge_shears() {
        use crate::combat::ENEMY_HALF_HEIGHT;
        use crate::shore::Shore;
        // A passive goblin facing -1 at every x from 420 to 560 in 0.5 px steps
        // under the landing block (footprint [444, 524)).
        let shore = Shore::new(419.0, 100.0);
        let footprint = shore.footprint();
        let (mut kills, mut shears) = (0, 0);
        let mut x = 420.0_f32;
        while x <= 560.0 {
            let g = Goblin::new(x, 100.0 + ENEMY_HALF_HEIGHT, EnemyAi::Passive);
            assert_eq!(g.facing, -1);
            if g.body().overlaps(footprint) {
                kills += 1;
                if g.neck_probes().is_some_and(|p| shore.edge_shears(p)) {
                    shears += 1;
                }
            }
            x += 0.5;
        }
        let share = f64::from(shears) / f64::from(kills);
        assert!(
            (0.05..=0.12).contains(&share),
            "{shears} of {kills}: {share}"
        );
        // 16 shear samples (8 px) of the 104 px kill band (208 samples).
        assert_eq!((shears, kills), (16, 207));
    }

    // ---------------------------------------------- WORK-023 (Q10 guard) --

    fn hash_of(g: &Goblin) -> Vec<u64> {
        let mut values = Vec::new();
        g.mix_into(&mut |v| values.push(v));
        values
    }

    #[test]
    fn take_hit_is_take_hit_split_with_the_legacy_split() {
        use crate::combat::StrikeTarget;
        use crate::donut_moves::HitStopSplit;
        for spec in [&CARL_JAB, &CARL_HAYMAKER] {
            let (mut a, mut b) = (goblin_at(500.0), goblin_at(500.0));
            let (outcome_a, stop) = a.take_hit(spec, 1);
            let (outcome_b, split) = b.take_hit_split(spec, 1, HitStopSplit::legacy);
            assert_eq!(outcome_a, outcome_b);
            assert_eq!(stop, split.attacker);
            assert_eq!(a, b, "the same goblin state after the same hit");
            assert_eq!(a.hit_stop, stop + 1, "the legacy target freeze is stop + 1");
        }
    }

    #[test]
    fn a_swat_split_leaves_the_goblin_unfrozen_and_staggers_a_tell() {
        use crate::combat::DONUT_SWAT;
        use crate::donut_moves::SWAT_HIT_STOP;
        let (mut g, _) = telling();
        for _ in 0..4 {
            tick(&mut g, 0.0, true);
        }
        assert!(g.token_held());
        let (_, split) = g.take_hit_split(&DONUT_SWAT, 1, |_| SWAT_HIT_STOP);
        assert_eq!((split.attacker, g.hit_stop), (2, 0));
        assert_eq!(g.mode, GoblinMode::Hurt, "the tell is cancelled");
        assert_eq!(g.stagger_left, 10);
        assert_eq!(g.health, ENEMY_HEALTH, "damage 0");
        assert!(!g.token_held(), "the token is back");
    }

    #[test]
    fn the_target_state_follows_the_mode_and_the_real_attack_phase() {
        let mut g = goblin_at(500.0);
        assert_eq!(g.target_state(), TargetState::Neutral);
        g.mode = GoblinMode::Attack;
        for (attack_tick, want) in [
            (0, TargetState::Tell),
            (23, TargetState::Tell),
            (24, TargetState::Smear),
            (26, TargetState::Impact),
            (29, TargetState::Impact),
            (30, TargetState::Recovery),
        ] {
            g.attack_tick = attack_tick;
            assert_eq!(g.target_state(), want, "attack tick {attack_tick}");
        }
        for (mode, want) in [
            (GoblinMode::Hurt, TargetState::Staggered),
            (GoblinMode::Launched, TargetState::Launched),
            (GoblinMode::Downed, TargetState::Downed),
            (GoblinMode::Airborne, TargetState::Airborne),
            (GoblinMode::Down, TargetState::Defeated),
            (GoblinMode::Corpse, TargetState::Defeated),
        ] {
            g.mode = mode;
            assert_eq!(g.target_state(), want, "{mode:?}");
        }
    }

    #[test]
    fn the_guard_is_mixed_into_the_hash_only_while_it_is_set() {
        let mut g = goblin_at(500.0);
        let unset = hash_of(&g);
        g.guard = Some(LightGuard::new());
        let set = hash_of(&g);
        assert!(set.len() > unset.len() && set[..unset.len()] == unset[..]);
        g.guard = None;
        assert_eq!(hash_of(&g), unset, "an unset guard hashes nothing");
    }

    #[test]
    fn the_guard_clears_after_60_ticks_and_the_next_impact_or_when_the_goblin_goes_down() {
        let mut g = goblin_at(500.0);
        g.guard = Some(LightGuard::new());
        // 70 neutral ticks: old enough, but no swing has reached Impact.
        for _ in 0..70 {
            g.observe_guard();
        }
        assert!(g.guard.is_some(), "no Impact yet: it stays");
        g.mode = GoblinMode::Attack;
        g.attack_tick = 27;
        g.observe_guard();
        assert!(g.guard.is_none(), "60 ticks and an Impact seen: it clears");
        // A launch (or a down) clears it at once.
        let mut h = goblin_at(500.0);
        h.guard = Some(LightGuard::new());
        h.mode = GoblinMode::Launched;
        h.observe_guard();
        assert!(h.guard.is_none());
    }

    // ---- CF02-B (Q09..Q11) -------------------------------------------------

    fn cf02() -> &'static Cf02Tuning {
        crate::cf02::active()
    }

    #[test]
    fn q11_a_wall_slam_of_damage_0_is_visual_only_and_the_goblin_just_drops() {
        let mut t = cf02().clone();
        t.q11_wall_slam_damage = 0;
        let mut g = goblin_at(500.0);
        g.mode = GoblinMode::Launched;
        g.launch = Some(3);
        g.y = FLOOR + 20.0;
        g.vy = 40.0;
        assert!(!g.wall_slam(&t));
        assert_eq!(g.health, ENEMY_HEALTH);
        // It falls straight down: no launch, no sideways speed.
        assert_eq!((g.launch, g.vx, g.mode), (None, 0.0, GoblinMode::Launched));
        assert!(g.vy <= 0.0);
    }

    #[test]
    fn q11_a_slam_with_health_left_drops_the_goblin_downed_and_at_0_defeats_it_in_place() {
        let t = cf02();
        // Health 2: 1 damage, it falls and lands lying, never launched again.
        let mut g = goblin_at(500.0);
        g.mode = GoblinMode::Launched;
        g.launch = Some(3);
        g.y = FLOOR + 10.0;
        g.vy = 80.0;
        g.vx = 170.0;
        assert!(!g.wall_slam(t));
        assert_eq!(g.health, ENEMY_HEALTH - 1);
        let mut landed = false;
        for _ in 0..30 {
            g.step(0.0, true, &[], EDGE, false, false);
            landed |= g.mode == GoblinMode::Downed;
        }
        assert!(landed, "mode {:?}", g.mode);
        // Health 1: it is defeated in place (airborne, falling), then lies.
        let mut g = goblin_at(500.0);
        g.health = 1;
        g.mode = GoblinMode::Launched;
        g.launch = Some(3);
        g.y = FLOOR + 10.0;
        g.vy = 80.0;
        g.vx = 170.0;
        assert!(g.wall_slam(t));
        assert!(!g.alive());
        assert_eq!((g.mode, g.vx), (GoblinMode::Airborne, 0.0));
        assert!(g.vy <= 0.0, "it crumples down the wall");
        for _ in 0..30 {
            g.step(0.0, true, &[], EDGE, false, false);
        }
        assert_eq!(g.mode, GoblinMode::Down);
        // Already on the floor: it is down at once (never stuck airborne).
        let mut g = goblin_at(500.0);
        g.health = 1;
        g.mode = GoblinMode::Launched;
        g.launch = Some(3);
        assert!(g.wall_slam(t));
        assert_eq!(g.mode, GoblinMode::Down);
    }

    #[test]
    fn q09_the_knockdown_has_no_launch_and_the_legacy_stagger_keeps_its_numbers() {
        let t = cf02();
        let mut g = goblin_at(500.0);
        g.mode = GoblinMode::Attack;
        g.attack_tick = 10;
        g.contact_knockdown(1, t);
        assert_eq!(g.mode, GoblinMode::Launched);
        assert_eq!((g.launch, g.attack_tick, g.vx, g.vy), (None, 0, 90.0, 90.0));
        assert_eq!(g.facing, -1);
        assert!(!g.token_held());
        let mut h = goblin_at(500.0);
        h.contact_as_target(1);
        let mut i = goblin_at(500.0);
        i.contact_stagger(1, t.q09_legacy_stagger, f32::from(t.q09_legacy_push), 4);
        assert_eq!(h, i, "the cf02 legacy numbers equal the original constants");
    }

    #[test]
    fn q11_a_solid_sets_the_wall_stop_and_the_layout_edge_does_not() {
        let mut g = goblin_at(100.0);
        g.mode = GoblinMode::Launched;
        g.launch = Some(1);
        g.vx = 170.0;
        g.vy = 150.0;
        g.y = FLOOR + 1.0;
        let wall = wall(g.x + 14.0);
        for _ in 0..10 {
            g.step(0.0, true, std::slice::from_ref(&wall), EDGE, false, false);
            if g.wall_stop.is_some() {
                break;
            }
        }
        assert_eq!(g.wall_stop, Some(170.0));
        // The next step clears it.
        g.step(0.0, true, std::slice::from_ref(&wall), EDGE, false, false);
        assert_eq!(g.wall_stop, None);
        // The layout edge: no solid, so no stop is reported.
        let mut e = goblin_at(60.0);
        e.mode = GoblinMode::Launched;
        e.launch = Some(1);
        e.vx = 170.0;
        e.vy = 150.0;
        e.y = FLOOR + 1.0;
        for _ in 0..40 {
            e.step(0.0, true, &[], 70.0, false, false);
            assert_eq!(e.wall_stop, None);
        }
        assert!(e.x <= 70.0);
    }
}
