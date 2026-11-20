//! Donut's swat, pounce, Magic Missile and mana inside the combat scene
//! (WORK-023, HUD-DEBT-02): the thin adapter between the Bevy-free domains
//! ([`crate::donut_moves`], [`crate::donut_mana`]) and [`CombatState`].
//!
//! A child module of `combat_scene`, so it reads and writes the combat state
//! directly while living in its own file. Nothing here runs unless the party is
//! live and `PartyTuning::donut_moves` is on; every piece of state it adds is
//! mixed into the hash only while it is non-default, so every legacy golden
//! (fixtures, party, boss, story) stays byte-identical whenever Donut's
//! abilities are locked or unused.
//!
//! Step order inside a tick (the same every tick): Carl, **Donut**, the scene's
//! physics, the goblins. Donut's step runs before her motion is integrated, as
//! the domain's tick contract asks: the pounce's launch impulse of tick 10 is
//! applied by the same tick's integration.

use super::companion::{Glance, MarkKind};
use super::{
    BOSS_ENEMY_ID, CombatEvent, CombatState, DefeatCause, Goblin, GoblinMode, HeroId, InputFrame,
    RawInput, Rect, SceneSim, TICK_SECONDS, solid_at,
};
use crate::boss::BossHit;
use crate::combat::{DONUT_POUNCE, DONUT_SWAT, MAGIC_MISSILE, MoveId, MoveSpec};
use crate::corridor::PartyStepCtx;
use crate::donut_mana::{
    DonutMana, DonutSetup, EmptyPress, ManaTuning, RestartKind, StartState, UnlockTrigger,
};
use crate::donut_moves::{
    DONUT_HERO, DonutMoveTuning, DonutMoves, HitStopSplit, LightGuard, MoveEvent, MoveKind,
    MoveView, StepInput, TargetKind, TargetState, TargetView, route_primary,
};
use crate::llama::{LlamaMode, LlamaTargets};
use crate::prototype::{BodyDrive, HERO_HALF_HEIGHT, HERO_HALF_WIDTH};

/// The Magic Missile cast: the bolt spawns on cast tick 4 (the release), the
/// cast ends on tick 16, which is also the re-fire tick (ANIM-DONUT-03).
pub const CAST_RELEASE_TICK: u16 = 4;
pub const CAST_TOTAL_TICKS: u16 = 16;
/// A Primary press made during a cast is remembered this long (Carl's value).
const CAST_BUFFER_TICKS: u64 = 6;
/// Where the bolt appears relative to Donut's pivot: 27 px ahead, 19 px up
/// (ANIM-DONUT-03 section 3, on the idle base).
const BOLT_SPAWN_FORWARD: f32 = 27.0;
const BOLT_SPAWN_UP: f32 = 19.0;
/// The bolt's hit box is 8 x 6, centred 4 px ahead of its pivot.
const BOLT_BOX_W: f32 = 8.0;
const BOLT_BOX_H: f32 = 6.0;
const BOLT_BOX_AHEAD: f32 = 4.0;

/// A cast in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cast {
    pub instance: u32,
    /// Ticks since the cast started (0 on its first tick).
    pub age: u16,
}

/// A Magic Missile bolt in flight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bolt {
    pub instance: u32,
    pub x: f32,
    pub y: f32,
    pub dir: i8,
    pub travelled: f32,
}

impl Bolt {
    /// The hit box: 8 x 6 at +4 px ahead of the bolt's pivot.
    pub fn rect(&self) -> Rect {
        let cx = self.x + f32::from(self.dir) * BOLT_BOX_AHEAD;
        Rect {
            left: cx - BOLT_BOX_W * 0.5,
            bottom: self.y - BOLT_BOX_H * 0.5,
            width: BOLT_BOX_W,
            height: BOLT_BOX_H,
        }
    }
}

/// Everything of Donut's abilities that the combat state owns.
#[derive(Debug, Clone)]
pub struct DonutKit {
    /// CF02-C1: llamas are hero-hit targets (configuration, never hashed and
    /// kept across restarts).
    llama_targets: Option<LlamaTargets>,
    pub tuning: ManaTuning,
    pub moves: DonutMoves,
    pub mana: DonutMana,
    trigger: Option<UnlockTrigger>,
    /// The state the fixture declares (a fixture restart restores it).
    start: StartState,
    /// Donut's own press edges (the scene keeps Carl's in `CombatState::input`).
    frame: InputFrame,
    /// The horizontal speed the pounce holds in flight.
    pounce_vx: Option<f32>,
    cast: Option<Cast>,
    /// A Primary press made during a cast fires when the cast ends, until this
    /// tick.
    cast_buffer_until: Option<u64>,
    next_cast: u32,
    bolts: Vec<Bolt>,
    /// The Donut control state of the previous tick (a swap to the AI clears
    /// the press buffer).
    was_bot: bool,
    /// Presentation caches: never hashed, never read by the simulation.
    hitbox: Option<Rect>,
    dry_tick: Option<u64>,
}

impl DonutKit {
    pub fn new(setup: DonutSetup) -> Self {
        let tuning = ManaTuning::pinned(true).expect("the committed mana tuning is valid");
        let mana = DonutMana::from_start(&tuning, setup.start);
        let trigger = setup
            .guild_zone
            .map(|(min_x, max_x)| UnlockTrigger::new(min_x, max_x, f32::MIN, f32::MAX));
        let mut trigger = trigger;
        if let Some(trigger) = trigger.as_mut() {
            trigger.restart_fixture(setup.start);
        }
        Self {
            llama_targets: None,
            tuning,
            moves: DonutMoves::new(DonutMoveTuning::SPEC),
            mana,
            trigger,
            start: setup.start,
            frame: InputFrame::default(),
            pounce_vx: None,
            cast: None,
            cast_buffer_until: None,
            next_cast: 0,
            bolts: Vec::new(),
            was_bot: true,
            hitbox: None,
            dry_tick: None,
        }
    }

    /// A restart: a defeat retry and a checkpoint respawn keep the unlock and
    /// refill the pool; a fixture restart restores the declared state.
    pub fn restart(&mut self, kind: RestartKind) {
        self.mana.restart(&self.tuning, kind);
        if let (RestartKind::Fixture(start), Some(trigger)) = (kind, self.trigger.as_mut()) {
            trigger.restart_fixture(start);
        }
        self.moves = DonutMoves::new(*self.moves.tuning());
        self.next_cast = 0;
        self.frame = InputFrame::default();
        self.pounce_vx = None;
        self.cast = None;
        self.cast_buffer_until = None;
        self.bolts.clear();
        self.was_bot = true;
        self.hitbox = None;
        self.dry_tick = None;
    }

    /// Fold into the state hash: **nothing** while Donut never used an ability
    /// (no move, no cast, no bolt, no held button, still locked).
    pub fn hash_into(&self, mix: &mut impl FnMut(u64)) {
        self.moves.hash_into(mix);
        self.mana.hash_into(mix);
        let held = &self.frame.held;
        if held.attack || held.physical {
            mix(0x444f_4e42_5554_0001); // "DONBUT"
            mix(u64::from(held.attack));
            mix(u64::from(held.physical));
        }
        if self.cast.is_some() || self.cast_buffer_until.is_some() || !self.bolts.is_empty() {
            mix(0x444f_4e43_4153_0001); // "DONCAS"
            match self.cast {
                Some(cast) => {
                    mix(1 + u64::from(cast.instance));
                    mix(u64::from(cast.age));
                }
                None => mix(0),
            }
            mix(self.cast_buffer_until.map_or(0, |t| t + 1));
            for bolt in &self.bolts {
                mix(u64::from(bolt.instance));
                mix(u64::from(bolt.x.to_bits()));
                mix(u64::from(bolt.y.to_bits()));
                mix(bolt.dir as u64);
                mix(u64::from(bolt.travelled.to_bits()));
            }
        }
    }
}

/// Donut's kit as the COMPANION view reads it (see
/// [`CombatState::donut_companion_state`]).
pub(super) struct DonutCompanionState {
    pub abilities_live: bool,
    pub running: Option<crate::companion::view::ActiveMove>,
    pub mana: crate::companion::view::ManaView,
}

/// What Donut's step decided for the scene's physics this tick.
pub struct DonutTick {
    /// Donut's input after the move locks (walking and jump only).
    pub raw: RawInput,
    pub can_move: bool,
    pub frozen: bool,
    pub drive: BodyDrive,
}

impl CombatState {
    /// Declare Donut's biscuit state and unlock zone (a fixture's `donut`
    /// setup, or the interactive scene's placeholder Guild zone), then restart
    /// the attempt from it.
    #[must_use]
    pub fn with_donut(mut self, setup: DonutSetup) -> Self {
        self.donut_kit = DonutKit::new(setup);
        // Q37: the llama switch is derived from the one CF02 switch
        // (`CombatState::cf02()`): a `cf02_*` fixture or `--cf02` makes the
        // llamas targets; every other fixture, the story route and `--demo` not.
        if self.cf02.is_some() {
            self.enable_llama_targets();
        }
        self
    }

    /// CF02-C1 (Q18, Q37): the llamas become Donut's targets (the one place that
    /// sets the switch; callers derive it from [`CombatState::cf02`]).
    pub(super) fn enable_llama_targets(&mut self) {
        self.donut_kit.llama_targets = LlamaTargets::for_rules(true);
    }

    /// Whether the llamas are targets of Donut's moves (Q18).
    pub fn llama_targets_live(&self) -> Option<LlamaTargets> {
        self.donut_kit.llama_targets
    }

    /// Donut's mana for the HUD: `(mana, mana_max)` once the biscuit is
    /// unlocked, `None` before it. Read-only.
    pub fn donut_mana_view(&self) -> Option<(u8, u8)> {
        let kit = &self.donut_kit;
        kit.mana
            .unlocked()
            .then_some((kit.mana.mana(), kit.tuning.mana_max))
    }

    /// The running swat or pounce, for the presentation and the tests.
    pub fn donut_move_view(&self) -> Option<MoveView> {
        self.donut_kit.moves.view()
    }

    /// The cast in progress, if any.
    pub fn donut_cast(&self) -> Option<Cast> {
        self.donut_kit.cast
    }

    /// The Magic Missile bolts in flight.
    pub fn donut_bolts(&self) -> &[Bolt] {
        &self.donut_kit.bolts
    }

    /// The active hit box of Donut's move this tick (`--debug-combat`).
    pub fn donut_hitbox(&self) -> Option<Rect> {
        self.donut_kit.hitbox
    }

    /// The tick of the last dry press (presentation only: the dry cue).
    pub fn donut_dry_tick(&self) -> Option<u64> {
        self.donut_kit.dry_tick
    }

    /// Donut's mana tuning (the pinned file).
    pub fn donut_mana_tuning(&self) -> &ManaTuning {
        &self.donut_kit.tuning
    }

    /// Restart kinds of the attempt reset (see [`DonutKit::restart`]).
    pub(super) fn restart_donut(&mut self, kind: RestartKind) {
        self.donut_kit.restart(kind);
    }

    /// The start state the fixture declared.
    pub(super) fn donut_start(&self) -> StartState {
        self.donut_kit.start
    }

    /// Donut's abilities are live: the party is, and the knob is on.
    fn donut_abilities_live(scene: &SceneSim) -> bool {
        scene.party_live() && scene.drive().tuning.donut_moves
    }

    /// One tick of Donut's abilities. Runs inside the live-party block of
    /// `step_party`, after Carl's strike and before the physics. `ungated` is
    /// Donut's raw input of the slot routing (idle for the AI); `can_act` is
    /// the legacy gate (not hurt, not defeated, not frozen by a blow).
    pub(super) fn step_donut(
        &mut self,
        scene: &SceneSim,
        ungated: RawInput,
        can_act: bool,
        tick: u64,
    ) -> DonutTick {
        let gated_legacy = if can_act {
            RawInput {
                move_x: ungated.move_x,
                jump: ungated.jump,
                ..RawInput::NONE
            }
        } else {
            RawInput::default()
        };
        let mut result = DonutTick {
            raw: gated_legacy,
            can_move: can_act,
            frozen: false,
            drive: BodyDrive::default(),
        };
        if !Self::donut_abilities_live(scene) {
            return result;
        }
        let drive = *scene.drive();
        let bot = drive.is_bot(HeroId::Donut);
        // COMPANION-AI-01 (CA-24): the companion director drives her through
        // the same injected press; every move it starts is AI-started (CA-20).
        let companion_driven = drive.companion_set(HeroId::Donut).is_some();

        // Mana: the regen runs on every tick, whoever controls Donut, and the
        // Guild trigger is evaluated in the fixed tick.
        let defeated = self.hero_defeated;
        let (pivot_x, foot_y) = {
            let body = scene.world.body(HeroId::Donut);
            (body.x, body.y - HERO_HALF_HEIGHT)
        };
        let mut mana_events = Vec::new();
        {
            let kit = &mut self.donut_kit;
            mana_events.extend(kit.mana.step(&kit.tuning, tick));
            if let Some(trigger) = kit.trigger.as_mut() {
                mana_events.extend(trigger.step(
                    &mut kit.mana,
                    &kit.tuning,
                    tick,
                    (pivot_x, foot_y),
                    defeated,
                ));
            }
        }
        for event in mana_events {
            self.log(CombatEvent::Mana(event));
        }

        // Presses: Donut's own edges. A swap to the AI forgets a buffered press.
        self.donut_kit.frame.advance(ungated);
        let frame = self.donut_kit.frame;
        if bot && !self.donut_kit.was_bot {
            self.donut_kit.moves.clear_buffer();
            self.donut_kit.cast_buffer_until = None;
        }
        self.donut_kit.was_bot = bot;

        let (facing, grounded) = {
            let body = scene.world.body(HeroId::Donut);
            (body.facing, body.grounded)
        };
        let hurt = self.donut.hurt_ticks > 0 || self.hero_defeated;
        let route = route_primary(
            true,
            self.donut_kit.mana.unlocked(),
            self.donut_kit.mana.can_start_cast(&self.donut_kit.tuning),
            self.donut_kit.tuning.empty_press == EmptyPress::Swat,
        );
        let targets = self.donut_targets();
        let wall_blocked = self.donut_wall_ahead(scene, facing);
        let input = StepInput {
            tick,
            primary_pressed: frame.attack_pressed,
            primary_route: route,
            physical_pressed: frame.physical_pressed,
            jump_pressed: frame.jump_pressed,
            move_pressed: frame.held.move_x != 0.0,
            grounded,
            pivot_x,
            pivot_y: foot_y,
            facing,
            wall_blocked,
            hurt,
            may_start: (!bot || companion_driven) && can_act,
            targets: &targets,
        };

        // The cast clock first: a cast ends on tick 16, so a press on that tick
        // re-fires.
        self.step_cast_clock(hurt);
        let out = self.donut_kit.moves.step(&input);
        self.donut_kit.hitbox = out.hitbox;

        let mut started_move = false;
        let instance_now = self.donut_kit.moves.view().map(|v| v.instance);
        for event in &out.events {
            match *event {
                MoveEvent::Started { mv, instance } => {
                    started_move = true;
                    if companion_driven {
                        self.companion
                            .mark_ai_started(MarkKind::DonutMove, instance);
                    }
                    self.log(CombatEvent::MoveStarted {
                        tick,
                        move_id: match mv {
                            MoveKind::Swat => MoveId::Swat,
                            MoveKind::Pounce => MoveId::Pounce,
                        },
                        instance,
                    });
                }
                // The launch speed is held in flight (below); the other moves
                // of the state machine need nothing from the scene.
                MoveEvent::Launched { .. }
                | MoveEvent::Touchdown { .. }
                | MoveEvent::Lapsed { .. }
                | MoveEvent::Cancelled { .. }
                | MoveEvent::Ended { .. } => {}
                MoveEvent::TellInterrupted { target, effect }
                | MoveEvent::Staggered { target, effect } => {
                    self.donut_swat_hit(target, effect, facing, instance_now.unwrap_or(0), tick);
                }
                MoveEvent::Glance {
                    target,
                    reason,
                    poise,
                } => {
                    self.log(CombatEvent::DonutGlance {
                        tick,
                        instance: instance_now.unwrap_or(0),
                        target,
                        reason,
                    });
                    if poise > 0 && target == BOSS_ENEMY_ID {
                        self.donut_boss_hit(
                            &DONUT_SWAT,
                            0,
                            u16::from(poise),
                            0,
                            tick,
                            Glance::default(),
                        );
                    }
                }
                MoveEvent::PounceHit { target, effect } => {
                    self.donut_pounce_hit(target, effect, facing, instance_now.unwrap_or(0), tick);
                }
                MoveEvent::DryPress => {
                    self.donut_dry(tick);
                }
            }
        }
        if started_move {
            // A new swat or pounce cancels a cast that has not released: it
            // costs nothing.
            self.cancel_unfired_cast();
        }
        if out.cast_requested || self.cast_buffer_fires(tick) {
            let before = self.donut_kit.next_cast;
            self.request_cast(tick);
            if companion_driven && self.donut_kit.next_cast != before {
                self.companion
                    .mark_ai_started(MarkKind::DonutCast, self.donut_kit.next_cast);
            }
        }
        if let Some((vx, _)) = out.launch {
            self.donut_kit.pounce_vx = Some(vx);
        }
        if let Some(vx) = out.set_vx {
            self.donut_kit.pounce_vx = Some(vx);
        }
        // The pounce holds its speed from the launch to the touchdown only.
        if !self
            .donut_kit
            .moves
            .view()
            .is_some_and(|v| v.mv == MoveKind::Pounce && v.launched && v.touchdown.is_none())
        {
            self.donut_kit.pounce_vx = None;
        }
        self.step_cast_release(scene, facing, pivot_x, foot_y, tick);
        self.step_bolts(scene, tick);

        // The locks of the running move on top of the legacy gate.
        let mut raw = gated_legacy;
        if out.input_locked {
            raw.move_x = 0.0;
        }
        if out.jump_blocked {
            raw.jump = false;
        }
        result.raw = raw;
        result.can_move = can_act && !out.input_locked;
        result.frozen = out.frozen;
        result.drive = BodyDrive {
            launch: out.launch,
            vx: self.donut_kit.pounce_vx,
            vx_cap: out.vx_cap,
            facing_locked: out.facing_locked,
            ..BodyDrive::default()
        };
        result
    }

    /// What the COMPANION view builder reads of Donut's kit (01A-wire):
    /// whether her abilities run at all, her running swat, pounce or cast, and
    /// her pool as the AI may use it. Read-only.
    pub(super) fn donut_companion_state(&self, scene: &SceneSim) -> DonutCompanionState {
        use crate::companion::view::{ActiveMove, ManaView, MovePhase as ViewPhase};
        use crate::donut_moves::{MovePhase, POUNCE_TOTAL_NOMINAL, SWAT_TOTAL};
        let kit = &self.donut_kit;
        let running = if let Some(view) = kit.moves.view() {
            let (id, total) = match view.mv {
                MoveKind::Swat => (MoveId::Swat, SWAT_TOTAL),
                MoveKind::Pounce => (MoveId::Pounce, POUNCE_TOTAL_NOMINAL),
            };
            Some(ActiveMove {
                id,
                phase: match view.phase {
                    MovePhase::Startup | MovePhase::Crouch => ViewPhase::Startup,
                    MovePhase::Active | MovePhase::Flight => ViewPhase::Active,
                    MovePhase::Recovery | MovePhase::Pin => ViewPhase::Recovery,
                },
                ticks_left: total.saturating_sub(view.tick),
                ai_started: self.companion.marked(MarkKind::DonutMove, view.instance),
            })
        } else {
            kit.cast.map(|cast| ActiveMove {
                id: MoveId::MagicMissile,
                phase: match cast.age.cmp(&CAST_RELEASE_TICK) {
                    core::cmp::Ordering::Less => ViewPhase::Startup,
                    core::cmp::Ordering::Equal => ViewPhase::Active,
                    core::cmp::Ordering::Greater => ViewPhase::Recovery,
                },
                ticks_left: CAST_TOTAL_TICKS.saturating_sub(cast.age),
                ai_started: self.companion.marked(MarkKind::DonutCast, cast.instance),
            })
        };
        DonutCompanionState {
            abilities_live: Self::donut_abilities_live(scene),
            running,
            mana: ManaView {
                mana: kit.mana.mana(),
                max: kit.tuning.mana_max,
                unlocked: kit.mana.unlocked(),
                ai_can_cast: kit.mana.ai_can_cast(&kit.tuning),
                projectile_speed_px_per_tick: kit.tuning.projectile_speed_px_per_tick,
                bolt_lead_px: (BOLT_SPAWN_FORWARD + BOLT_BOX_AHEAD + BOLT_BOX_W / 2.0) as u8,
            },
        }
    }

    /// Fill the party context's Donut slot from the tick of [`Self::step_donut`].
    pub(super) fn apply_donut_tick(ctx: &mut PartyStepCtx<'_>, tick: &DonutTick) {
        ctx.can_move[1] = tick.can_move;
        ctx.frozen[1] |= tick.frozen;
        ctx.drives[1] = tick.drive;
    }

    /// The targets of Donut's moves: every goblin, then (Q18, `cf02_rules`
    /// only) every llama with id `goblins + index`, then the boss while it has
    /// a hurtbox. Indices ascend (the lowest-first rule). Without Q18 llamas
    /// are no target.
    fn donut_targets(&self) -> Vec<TargetView> {
        let mut targets: Vec<TargetView> = self
            .enemies
            .iter()
            .enumerate()
            .map(|(index, goblin)| TargetView {
                index: index as u32,
                kind: TargetKind::Goblin,
                body: goblin.body(),
                state: goblin.target_state(),
                guarded: goblin.guard.is_some(),
            })
            .collect();
        if self.donut_kit.llama_targets.is_some() {
            let first = self.enemies.len() as u32;
            targets.extend(
                self.llamas
                    .iter()
                    .enumerate()
                    .map(|(index, llama)| TargetView {
                        index: first + index as u32,
                        kind: TargetKind::Llama,
                        body: llama.body(),
                        state: llama.target_state(),
                        guarded: llama.guard.is_some(),
                    }),
            );
        }
        if let Some(boss) = self.boss.as_ref()
            && let Some(body) = boss.hurtbox()
        {
            let neutral = matches!(
                boss.mode,
                crate::boss::BossMode::Neutral | crate::boss::BossMode::Turn
            );
            targets.push(TargetView {
                index: BOSS_ENEMY_ID,
                kind: TargetKind::Boss,
                body,
                state: if neutral {
                    TargetState::Neutral
                } else {
                    TargetState::Recovery
                },
                guarded: false,
            });
        }
        targets
    }

    /// A wall, or the layout edge, in the way of the pounce's next step.
    fn donut_wall_ahead(&self, scene: &SceneSim, facing: i8) -> bool {
        let body = scene.world.body(HeroId::Donut);
        let step =
            f32::from(facing) * self.donut_kit.moves.tuning().pounce_launch_vx * TICK_SECONDS;
        let next = body.x + step;
        if next < HERO_HALF_WIDTH || next > self.x_max - HERO_HALF_WIDTH {
            return true;
        }
        solid_at(
            &scene.world.solids,
            Rect {
                left: next - HERO_HALF_WIDTH,
                bottom: body.y - HERO_HALF_HEIGHT,
                width: HERO_HALF_WIDTH * 2.0,
                height: HERO_HALF_HEIGHT * 2.0,
            },
        )
    }

    // ------------------------------------------------------------ hits --

    /// A swat that staggered (or interrupted) a goblin: the same `take_hit` as
    /// Carl's jab with damage 0, the hit-stop split (Donut freezes, the goblin
    /// does not), and the light guard.
    fn donut_swat_hit(
        &mut self,
        target: u32,
        effect: crate::donut_moves::TargetEffect,
        facing: i8,
        instance: u32,
        tick: u64,
    ) {
        if let Some(index) = self.llama_target(target) {
            self.donut_swat_hits_llama(index, effect, facing, instance, tick);
            return;
        }
        let Some(goblin) = self.enemies.get_mut(target as usize) else {
            return;
        };
        let was_attacking = goblin.token_held();
        let (_, split) = goblin.take_hit_split(&DONUT_SWAT, facing, |_| effect.hit_stop);
        let health_left = goblin.health;
        goblin.hit_by = Some(DONUT_HERO);
        if effect.sets_guard {
            goblin.guard = Some(LightGuard::new());
        }
        let interrupted = was_attacking && goblin.mode != GoblinMode::Attack;
        self.log(CombatEvent::Hit {
            tick,
            move_id: MoveId::Swat,
            instance,
            target,
            damage: DONUT_SWAT.damage,
            health_left,
            hit_stop: split.attacker,
        });
        if interrupted {
            self.log(CombatEvent::GoblinInterrupted {
                tick,
                goblin: target,
            });
        }
        self.flash_hit(target as usize, tick);
    }

    /// The pounce on a goblin (`take_hit` like Carl's jab, the legacy hit-stop,
    /// a kill credited to Donut) or on the boss (its hero-hit rule, Q10-A03).
    fn donut_pounce_hit(
        &mut self,
        target: u32,
        effect: crate::donut_moves::TargetEffect,
        facing: i8,
        instance: u32,
        tick: u64,
    ) {
        if target == BOSS_ENEMY_ID {
            // Super armour: damage and Donut's hit-stop, never an interrupt and
            // never the 14-tick stagger (the boss ignores `stagger_ticks`).
            let glance = self.companion.glance_for(MarkKind::DonutMove, instance);
            self.donut_boss_hit(
                &DONUT_POUNCE,
                u16::from(effect.damage),
                u16::from(effect.damage),
                effect.hit_stop.attacker,
                tick,
                glance,
            );
            return;
        }
        if let Some(index) = self.llama_target(target) {
            self.donut_pounce_hits_llama(index, facing, instance, tick);
            return;
        }
        // CA-20 / CF02-Q38: an AI-started pounce re-checks lethality on its hit
        // tick, on the HP the goblin has now (a human may have lowered it
        // mid-move); a lethal one lands as a glance (damage 0, same reaction).
        let glance = self.companion.glance_for(MarkKind::DonutMove, instance);
        let Some(goblin) = self.enemies.get_mut(target as usize) else {
            return;
        };
        let landed = glance.apply(&DONUT_POUNCE, goblin.health);
        let was_attacking = goblin.token_held();
        let (outcome, split) = goblin.take_hit_split(&landed, facing, HitStopSplit::legacy);
        let health_left = goblin.health;
        goblin.hit_by = Some(DONUT_HERO);
        let interrupted = was_attacking && goblin.mode != GoblinMode::Attack;
        let defeated = matches!(outcome, crate::combat::HitOutcome::Defeated);
        // A lethal pounce adds the lethal hit-stop the goblin also took.
        self.donut_kit.moves.extend_hit_stop(split.attacker);
        self.log(CombatEvent::Hit {
            tick,
            move_id: MoveId::Pounce,
            instance,
            target,
            damage: landed.damage,
            health_left,
            hit_stop: split.attacker,
        });
        if defeated {
            self.log_donut_defeat(target, &DONUT_POUNCE, tick);
        }
        if interrupted {
            self.log(CombatEvent::GoblinInterrupted {
                tick,
                goblin: target,
            });
        }
        self.flash_hit(target as usize, tick);
    }

    // ------------------------------------------------------ llamas (Q18) --

    /// The llama index of a move target id (`goblins + index`), while the
    /// llamas are targets at all.
    fn llama_target(&self, target: u32) -> Option<usize> {
        if self.donut_kit.llama_targets.is_none() || target == BOSS_ENEMY_ID {
            return None;
        }
        let index = (target as usize).checked_sub(self.enemies.len())?;
        (index < self.llamas.len()).then_some(index)
    }

    fn llama_knobs(&self) -> LlamaTargets {
        self.donut_kit
            .llama_targets
            .expect("llama_target only returns an index while the llamas are targets")
    }

    /// A swat that interrupted a llama's tell (Q18): 0 damage, the llama's own
    /// 18-tick stagger and 30-tick cooldown, the Q10 light guard, Donut alone
    /// frozen. The only swat outcome a llama gets besides a glance.
    fn donut_swat_hits_llama(
        &mut self,
        index: usize,
        effect: crate::donut_moves::TargetEffect,
        facing: i8,
        instance: u32,
        tick: u64,
    ) {
        let spec = MoveSpec {
            damage: self.llama_knobs().swat_damage,
            ..DONUT_SWAT
        };
        let glance = self.companion.glance_for(MarkKind::DonutMove, instance);
        let id = (self.enemies.len() + index) as u32;
        let llama = &mut self.llamas[index];
        let landed = glance.apply(&spec, llama.health);
        let was_telling = llama.telling();
        let (outcome, split) = llama.take_hit_split(&landed, facing, |_| effect.hit_stop);
        let health_left = llama.health;
        llama.hit_by = Some(DONUT_HERO);
        let defeated = matches!(outcome, crate::combat::HitOutcome::Defeated);
        if effect.sets_guard && !defeated {
            llama.guard = Some(LightGuard::new());
        }
        let interrupted = was_telling && llama.mode != LlamaMode::Attack;
        self.log(CombatEvent::Hit {
            tick,
            move_id: MoveId::Swat,
            instance,
            target: id,
            damage: landed.damage,
            health_left,
            hit_stop: split.attacker,
        });
        if defeated {
            self.log_donut_defeat(id, &spec, tick);
        }
        if interrupted {
            self.log(CombatEvent::LlamaInterrupted {
                tick,
                llama: index as u32,
            });
        }
        self.flash_hit(id as usize, tick);
    }

    /// The pounce on a llama (Q18): the pounce's own damage (1), one target,
    /// the legacy hit-stop; a kill is credited to Donut and is a plain `down`
    /// (a routine force, never the slam: only Carl's lethal kick slams).
    fn donut_pounce_hits_llama(&mut self, index: usize, facing: i8, instance: u32, tick: u64) {
        let spec = MoveSpec {
            damage: self.llama_knobs().pounce_damage,
            ..DONUT_POUNCE
        };
        let glance = self.companion.glance_for(MarkKind::DonutMove, instance);
        let id = (self.enemies.len() + index) as u32;
        let llama = &mut self.llamas[index];
        let landed = glance.apply(&spec, llama.health);
        let was_telling = llama.telling();
        let (outcome, split) = llama.take_hit_split(&landed, facing, HitStopSplit::legacy);
        let health_left = llama.health;
        llama.hit_by = Some(DONUT_HERO);
        let interrupted = was_telling && llama.mode != LlamaMode::Attack;
        let defeated = matches!(outcome, crate::combat::HitOutcome::Defeated);
        self.donut_kit.moves.extend_hit_stop(split.attacker);
        self.log(CombatEvent::Hit {
            tick,
            move_id: MoveId::Pounce,
            instance,
            target: id,
            damage: landed.damage,
            health_left,
            hit_stop: split.attacker,
        });
        if defeated {
            self.log_donut_defeat(id, &spec, tick);
        }
        if interrupted {
            self.log(CombatEvent::LlamaInterrupted {
                tick,
                llama: index as u32,
            });
        }
        self.flash_hit(id as usize, tick);
    }

    /// A Magic Missile bolt that ends on a llama (Q18): 1 damage, only the
    /// llama freezes, the bolt is consumed by the caller.
    fn missile_hits_llama(&mut self, index: usize, bolt: Bolt, tick: u64) {
        let spec = MoveSpec {
            damage: self.llama_knobs().missile_damage,
            ..MAGIC_MISSILE
        };
        let glance = self
            .companion
            .glance_for(MarkKind::DonutCast, bolt.instance);
        let id = (self.enemies.len() + index) as u32;
        let llama = &mut self.llamas[index];
        let landed = glance.apply(&spec, llama.health);
        let was_telling = llama.telling();
        let (outcome, split) = llama.take_hit_split(&landed, bolt.dir, |stop| HitStopSplit {
            attacker: 0,
            target: stop + 1,
        });
        let health_left = llama.health;
        llama.hit_by = Some(DONUT_HERO);
        let interrupted = was_telling && llama.mode != LlamaMode::Attack;
        let defeated = matches!(outcome, crate::combat::HitOutcome::Defeated);
        self.log(CombatEvent::Hit {
            tick,
            move_id: MoveId::MagicMissile,
            instance: bolt.instance,
            target: id,
            damage: landed.damage,
            health_left,
            hit_stop: split.target.saturating_sub(1),
        });
        if defeated {
            self.log_donut_defeat(id, &spec, tick);
        }
        if interrupted {
            self.log(CombatEvent::LlamaInterrupted {
                tick,
                llama: index as u32,
            });
        }
        self.flash_hit(id as usize, tick);
    }

    /// A Donut hit on the boss: the boss' `apply_hit` (the rule every hero hit
    /// follows), then the defeat event if it fell.
    fn donut_boss_hit(
        &mut self,
        spec: &MoveSpec,
        damage: u16,
        poise: u16,
        hit_stop: u8,
        tick: u64,
        glance: Glance,
    ) {
        let Some(mut boss) = self.boss.take() else {
            return;
        };
        // CA-20: the kill policy reads the boss' HP on the hit tick (clamped,
        // so a boss with more than 255 HP never glances).
        let damage = glance.lands(damage, boss.hp);
        let mut events = Vec::new();
        if let Some(result) = boss.apply_hit(
            BossHit {
                damage,
                poise,
                player: DONUT_HERO,
                hit_stop,
            },
            &mut events,
            tick,
        ) && result.defeated
        {
            events.push(CombatEvent::enemy_defeated(
                tick,
                BOSS_ENEMY_ID,
                DefeatCause::Melee,
                Some(DONUT_HERO),
                Some(spec),
            ));
        }
        for event in events {
            self.log(event);
        }
        self.boss = Some(boss);
    }

    fn log_donut_defeat(&mut self, target: u32, spec: &MoveSpec, tick: u64) {
        self.log(CombatEvent::enemy_defeated(
            tick,
            target,
            DefeatCause::Melee,
            Some(DONUT_HERO),
            Some(spec),
        ));
    }

    fn flash_hit(&mut self, target: usize, tick: u64) {
        if let Some(slot) = self.flash_tick.get_mut(target) {
            *slot = Some(tick);
        }
    }

    // ------------------------------------------------------------ mana --

    fn donut_dry(&mut self, tick: u64) {
        if let Some(event) = self.donut_kit.mana.reject_dry(tick) {
            self.donut_kit.dry_tick = Some(tick);
            self.log(CombatEvent::Mana(event));
        }
    }

    // ------------------------------------------------------- the cast --

    /// Advance the cast clock one tick: it ends on cast tick 16, and a hurt
    /// cancels it (an unreleased cast costs nothing).
    fn step_cast_clock(&mut self, hurt: bool) {
        let kit = &mut self.donut_kit;
        if let Some(cast) = kit.cast.as_mut() {
            cast.age += 1;
            if hurt || cast.age >= CAST_TOTAL_TICKS {
                kit.cast = None;
            }
        }
        if hurt {
            kit.cast_buffer_until = None;
        }
    }

    fn cancel_unfired_cast(&mut self) {
        if let Some(cast) = self.donut_kit.cast
            && cast.age < CAST_RELEASE_TICK
        {
            self.donut_kit.cast = None;
        }
    }

    /// A buffered Primary press fires when the cast that held it ended.
    fn cast_buffer_fires(&mut self, tick: u64) -> bool {
        let kit = &mut self.donut_kit;
        match kit.cast_buffer_until {
            Some(until) if kit.cast.is_none() => {
                kit.cast_buffer_until = None;
                tick <= until
            }
            Some(until) if tick > until => {
                kit.cast_buffer_until = None;
                false
            }
            _ => false,
        }
    }

    /// A Primary press routed to the cast (fresh or released from a buffer): it
    /// starts a cast, or is remembered while one runs. The mana check runs on
    /// the tick a cast **starts**, also for a buffered press.
    fn request_cast(&mut self, tick: u64) {
        if self.donut_kit.cast.is_some() {
            self.donut_kit.cast_buffer_until = Some(tick + CAST_BUFFER_TICKS);
            return;
        }
        if !self.donut_kit.mana.can_start_cast(&self.donut_kit.tuning) {
            self.donut_dry(tick);
            return;
        }
        self.donut_kit.next_cast += 1;
        let instance = self.donut_kit.next_cast;
        self.donut_kit.cast = Some(Cast { instance, age: 0 });
        self.log(CombatEvent::MoveStarted {
            tick,
            move_id: MoveId::MagicMissile,
            instance,
        });
    }

    /// The release (cast tick 4): the charge is spent and the bolt appears.
    fn step_cast_release(
        &mut self,
        scene: &SceneSim,
        facing: i8,
        pivot_x: f32,
        foot_y: f32,
        tick: u64,
    ) {
        let Some(cast) = self.donut_kit.cast else {
            return;
        };
        if cast.age != CAST_RELEASE_TICK {
            return;
        }
        let tuning = self.donut_kit.tuning.clone();
        let Some(event) = self.donut_kit.mana.release(&tuning, tick) else {
            return;
        };
        self.log(CombatEvent::Mana(event));
        let bolt = Bolt {
            instance: cast.instance,
            x: pivot_x + f32::from(facing) * BOLT_SPAWN_FORWARD,
            y: foot_y + BOLT_SPAWN_UP,
            dir: facing,
            travelled: 0.0,
        };
        self.donut_kit.bolts.push(bolt);
        // The new bolt is tested on its spawn tick (before it moves).
        self.test_bolt(self.donut_kit.bolts.len() - 1, scene, tick);
    }

    /// Move every older bolt and test it; a bolt that hit, ran into a wall or
    /// reached its range is gone.
    fn step_bolts(&mut self, scene: &SceneSim, tick: u64) {
        let speed = f32::from(self.donut_kit.tuning.projectile_speed_px_per_tick);
        let range = f32::from(self.donut_kit.tuning.projectile_range_px);
        let spawned_now = self
            .donut_kit
            .cast
            .filter(|c| c.age == CAST_RELEASE_TICK)
            .map(|c| c.instance);
        let mut index = 0;
        while index < self.donut_kit.bolts.len() {
            let bolt = &mut self.donut_kit.bolts[index];
            if Some(bolt.instance) == spawned_now {
                // Spawned this tick and already tested.
                index += 1;
                continue;
            }
            bolt.x += f32::from(bolt.dir) * speed;
            bolt.travelled += speed;
            let gone = bolt.travelled >= range;
            if gone {
                self.donut_kit.bolts.remove(index);
                continue;
            }
            if self.test_bolt(index, scene, tick) {
                continue;
            }
            index += 1;
        }
    }

    /// Test bolt `index` against the walls, the goblins and the boss. Returns
    /// true when the bolt was consumed (and removed).
    fn test_bolt(&mut self, index: usize, scene: &SceneSim, tick: u64) -> bool {
        let bolt = self.donut_kit.bolts[index];
        let rect = bolt.rect();
        if solid_at(&scene.world.solids, rect) {
            self.donut_kit.bolts.remove(index);
            return true;
        }
        // The first living goblin it overlaps, lowest index first.
        let goblin_hit = self
            .enemies
            .iter()
            .position(|g| g.alive() && g.body().overlaps(rect));
        // Q18: the llamas come after the goblins (ids ascend), and the bolt
        // ends on the first living one it overlaps instead of passing through.
        let llama_hit = self
            .donut_kit
            .llama_targets
            .and_then(|_| {
                self.llamas
                    .iter()
                    .position(|l| l.alive() && l.body().overlaps(rect))
            })
            .filter(|_| goblin_hit.is_none());
        let boss_hit = self
            .boss
            .as_ref()
            .and_then(|b| b.hurtbox())
            .is_some_and(|b| b.overlaps(rect));
        if let Some(target) = goblin_hit {
            self.missile_hits_goblin(target, bolt, tick);
            self.donut_kit.bolts.remove(index);
            return true;
        }
        if let Some(llama) = llama_hit {
            self.missile_hits_llama(llama, bolt, tick);
            self.donut_kit.bolts.remove(index);
            return true;
        }
        if boss_hit {
            let glance = self
                .companion
                .glance_for(MarkKind::DonutCast, bolt.instance);
            self.donut_boss_hit(
                &MAGIC_MISSILE,
                u16::from(MAGIC_MISSILE.damage),
                u16::from(MAGIC_MISSILE.damage),
                MAGIC_MISSILE.reaction.hit_stop,
                tick,
                glance,
            );
            self.donut_kit.bolts.remove(index);
            return true;
        }
        false
    }

    fn missile_hits_goblin(&mut self, target: usize, bolt: Bolt, tick: u64) {
        // CA-20 / CF02-Q38: the AI-started bolt re-checks lethality on its hit
        // tick, like the pounce.
        let glance = self
            .companion
            .glance_for(MarkKind::DonutCast, bolt.instance);
        let goblin: &mut Goblin = &mut self.enemies[target];
        let landed = glance.apply(&MAGIC_MISSILE, goblin.health);
        let was_attacking = goblin.token_held();
        // The bolt is at range: only the target freezes (it is stepped after
        // this strike on the same tick, hence `stop + 1`).
        let (outcome, split) = goblin.take_hit_split(&landed, bolt.dir, |stop| HitStopSplit {
            attacker: 0,
            target: stop + 1,
        });
        let health_left = goblin.health;
        goblin.hit_by = Some(DONUT_HERO);
        let interrupted = was_attacking && goblin.mode != GoblinMode::Attack;
        let defeated = matches!(outcome, crate::combat::HitOutcome::Defeated);
        self.log(CombatEvent::Hit {
            tick,
            move_id: MoveId::MagicMissile,
            instance: bolt.instance,
            target: target as u32,
            damage: landed.damage,
            health_left,
            hit_stop: split.target.saturating_sub(1),
        });
        if defeated {
            self.log_donut_defeat(target as u32, &MAGIC_MISSILE, tick);
        }
        if interrupted {
            self.log(CombatEvent::GoblinInterrupted {
                tick,
                goblin: target as u32,
            });
        }
        self.flash_hit(target, tick);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss_scene::Pilot;
    use crate::combat::{CombatEvent, DONUT_MOVES, MAGIC_MISSILE, MoveId, MoveSpec, carl_move};
    use crate::combat::{EnemyAi, HitShape};
    use crate::combat_scene::EnemySpawn;
    use crate::combat_scene::Fixture;
    use crate::donut_fixtures::{physical, primary};
    use crate::donut_mana::DonutSetup;
    use crate::donut_moves::{
        LIGHT_GUARD_MIN_TICKS, POUNCE_BOX, POUNCE_DAMAGE, POUNCE_HIT_STOP, POUNCE_RECOVERY,
        POUNCE_STAGGER, POUNCE_STARTUP, SWAT_ACTIVE, SWAT_BOX, SWAT_DAMAGE, SWAT_HIT_STOP,
        SWAT_PUSH_PX_S, SWAT_RECOVERY, SWAT_STAGGER, SWAT_STARTUP,
    };
    use crate::env_kit::EnvKit;
    use crate::feel::{POUNCE_FEEL, SWAT_FEEL};
    use crate::party::{PROVISIONAL_TUNING, PartyTuning};
    use crate::party_scene::{PartyFixture, PartyFixtureSim, SlotHold, base};
    use std::path::PathBuf;

    fn kit() -> EnvKit {
        EnvKit::load(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            "assets/levels/floor01/corridor_abc_sample.json",
        )
        .unwrap()
    }

    fn fixture(
        name: &'static str,
        enemies: &'static [EnemySpawn],
        script: &'static [SlotHold],
        donut: DonutSetup,
        two_player: bool,
        ticks: u64,
    ) -> &'static PartyFixture {
        let base: &'static Fixture = Box::leak(Box::new(base(name, "test", 1, enemies, ticks)));
        Box::leak(Box::new(PartyFixture {
            base,
            two_player,
            tuning: PROVISIONAL_TUNING,
            script,
            pilot: Pilot::None,
            donut,
        }))
    }

    fn sim(fixture: &'static PartyFixture) -> PartyFixtureSim {
        PartyFixtureSim::new(&kit(), 360, fixture).unwrap()
    }

    // ------------------------------------------------------ data pins --

    fn box_of(spec: &MoveSpec) -> (f32, f32, f32, f32) {
        match spec.shape {
            HitShape::Box {
                x_min,
                x_max,
                y_min,
                y_max,
            } => (x_min, x_max, y_min, y_max),
            HitShape::GreyboxReach { .. } => panic!("not a box"),
        }
    }

    #[test]
    fn the_move_specs_carry_the_q10_numbers_the_domain_owns() {
        use crate::combat::{DONUT_POUNCE, DONUT_SWAT};
        // Swat (Q10-02..05, 07).
        assert_eq!(
            (DONUT_SWAT.startup, DONUT_SWAT.active, DONUT_SWAT.recovery),
            (SWAT_STARTUP, SWAT_ACTIVE, SWAT_RECOVERY)
        );
        let swat = SWAT_BOX;
        assert_eq!(
            box_of(&DONUT_SWAT),
            (swat.x0, swat.x1, swat.y0, swat.y1),
            "the same box"
        );
        assert_eq!(DONUT_SWAT.damage, SWAT_DAMAGE);
        assert_eq!(DONUT_SWAT.reaction.stagger_ticks, SWAT_STAGGER);
        assert_eq!(DONUT_SWAT.reaction.knock_vx, SWAT_PUSH_PX_S);
        assert_eq!(SWAT_FEEL.hit_stop, SWAT_HIT_STOP.attacker);
        assert_eq!(SWAT_HIT_STOP.target, 0, "the target is never frozen");
        let chain = DONUT_SWAT.chain.expect("the swat chains");
        assert_eq!(
            (chain.press_from, chain.press_to, chain.earliest),
            (4, 13, 10)
        );
        // Pounce (Q10-10, 11, 13, 14): the flat-floor nominal of the spec.
        assert_eq!(DONUT_POUNCE.startup, POUNCE_STARTUP);
        assert_eq!(DONUT_POUNCE.recovery, POUNCE_RECOVERY);
        assert_eq!(
            DONUT_POUNCE.total(),
            crate::donut_moves::POUNCE_TOTAL_NOMINAL
        );
        let pounce = POUNCE_BOX;
        assert_eq!(
            box_of(&DONUT_POUNCE),
            (pounce.x0, pounce.x1, pounce.y0, pounce.y1)
        );
        assert_eq!(DONUT_POUNCE.damage, POUNCE_DAMAGE);
        assert_eq!(DONUT_POUNCE.reaction.stagger_ticks, POUNCE_STAGGER);
        assert_eq!(POUNCE_FEEL.hit_stop, POUNCE_HIT_STOP.attacker);
        assert_eq!(POUNCE_HIT_STOP.target, POUNCE_FEEL.hit_stop + 1, "legacy");
        // Magic Missile: release on cast tick 4, 16 ticks per cast, 1 damage.
        assert_eq!(MAGIC_MISSILE.startup, CAST_RELEASE_TICK);
        assert_eq!(MAGIC_MISSILE.total(), CAST_TOTAL_TICKS);
        assert_eq!(
            MAGIC_MISSILE.damage,
            ManaTuning::pinned(true).unwrap().missile_damage
        );
        // Donut's table never holds a Carl move and Carl's never a Donut move.
        assert!(DONUT_MOVES.iter().all(|spec| carl_move(spec.id).is_none()));
        assert_eq!(LIGHT_GUARD_MIN_TICKS, 60);
    }

    // ---------------------------------------------------- hit-stop split --

    #[test]
    fn a_swat_freezes_donut_for_two_ticks_and_the_goblin_for_none() {
        static SCRIPT: [SlotHold; 2] =
            [crate::party_scene::run(70, 80, 1, 1.0), primary(95, 96, 1)];
        let f = fixture(
            "t_swat_split",
            &[EnemySpawn {
                x: 520,
                ai: EnemyAi::Chase,
            }],
            &SCRIPT,
            DonutSetup::NONE,
            true,
            99,
        );
        let mut s = sim(f);
        s.run_scripted(99);
        let hit = s
            .combat
            .events
            .iter()
            .position(|e| {
                matches!(
                    e,
                    CombatEvent::Hit {
                        move_id: MoveId::Swat,
                        ..
                    }
                )
            })
            .expect("the swat hit on tick 99");
        let _ = hit;
        assert_eq!(s.combat.enemies[0].hit_stop, 0, "the goblin keeps moving");
        assert_eq!(s.combat.donut_kit.moves.hit_stop_left(), 2, "Donut holds 2");
        assert!(
            s.combat.enemies[0].guard.is_some(),
            "the swat set the guard"
        );
        assert_eq!(s.combat.enemies[0].health, 2);
    }

    // ------------------------------------------------------ hash gating --

    #[test]
    fn nothing_of_donuts_kit_is_mixed_into_the_hash_until_she_uses_an_ability() {
        let kit_hash = |kit: &DonutKit| {
            let mut v = Vec::new();
            kit.hash_into(&mut |x| v.push(x));
            v
        };
        let mut kit = DonutKit::new(DonutSetup::NONE);
        assert!(kit_hash(&kit).is_empty(), "locked and idle: nothing");
        kit.frame.advance(RawInput {
            attack: true,
            ..RawInput::NONE
        });
        assert!(!kit_hash(&kit).is_empty(), "a held button is hashed");
        kit.frame.advance(RawInput::default());
        assert!(kit_hash(&kit).is_empty());
        kit.cast = Some(Cast {
            instance: 1,
            age: 2,
        });
        assert!(!kit_hash(&kit).is_empty(), "a cast is hashed");
        kit.cast = None;
        kit.bolts.push(Bolt {
            instance: 1,
            x: 10.0,
            y: 20.0,
            dir: 1,
            travelled: 0.0,
        });
        assert!(!kit_hash(&kit).is_empty(), "a bolt is hashed");
        kit.bolts.clear();
        let unlocked = DonutKit::new(DonutSetup::unlocked(8));
        assert!(
            !kit_hash(&unlocked).is_empty(),
            "an unlocked pool is hashed"
        );
    }

    // --------------------------------------------------------- the bolt --

    #[test]
    fn the_bolt_spawns_ahead_of_donut_flies_6_px_a_tick_and_fizzles_at_240_px() {
        static SCRIPT: [SlotHold; 1] = [primary(10, 11, 1)];
        let f = fixture("t_bolt", &[], &SCRIPT, DonutSetup::unlocked(8), true, 80);
        let mut s = sim(f);
        s.run_scripted(13);
        assert!(s.combat.donut_bolts().is_empty(), "nothing before tick 14");
        s.run_scripted(1);
        let bolt = s.combat.donut_bolts()[0];
        // Release on cast tick 4: 27 px ahead of her pivot, 19 px above the floor.
        let donut = s.scene.world.donut_body.x;
        assert_eq!(bolt.x, donut + 27.0);
        assert_eq!(bolt.y, s.scene.world.foot_y_of(HeroId::Donut) + 19.0);
        s.run_scripted(1);
        assert_eq!(s.combat.donut_bolts()[0].x, bolt.x + 6.0);
        // 240 px range: 40 ticks of flight, gone on the 40th step.
        s.run_scripted(38);
        assert_eq!(s.combat.donut_bolts().len(), 1);
        s.run_scripted(1);
        assert!(s.combat.donut_bolts().is_empty(), "it fizzled at range");
        assert_eq!(s.combat.donut_mana_view(), Some((7, 8)), "one charge spent");
    }

    #[test]
    fn a_missile_that_kills_credits_donut_and_is_a_routine_melee_defeat() {
        static SCRIPT: [SlotHold; 2] = [primary(10, 11, 1), primary(26, 27, 1)];
        let f = fixture(
            "t_bolt_kill",
            &[EnemySpawn {
                x: 600,
                ai: EnemyAi::Passive,
            }],
            &SCRIPT,
            DonutSetup::unlocked(8),
            true,
            90,
        );
        let mut s = sim(f);
        s.run_scripted(90);
        let defeats: Vec<_> = s
            .combat
            .events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::EnemyDefeated {
                    instigator, force, ..
                } => Some((*instigator, *force)),
                _ => None,
            })
            .collect();
        assert_eq!(
            defeats,
            vec![(Some(1), crate::combat::ForceTier::Routine)],
            "two bolts: two damage"
        );
        assert_eq!(s.combat.donut_mana_view(), Some((6, 8)));
    }

    /// AUDIENCE-01: the scene's own Magic Missile kill emits the hero index of
    /// Donut, whoever holds the pad. Both runs go through the real emit site
    /// and `AudienceInput::from_combat`: P2 driving Donut in 2P, and P1
    /// driving Donut after a solo swap (the controller slot is 0 there).
    #[test]
    fn a_donut_kill_credits_donut_whoever_drives_her() {
        use crate::audience::{AudienceInput, AudienceState, TUNING};
        use crate::party::HeroId;
        use crate::party_scene::swap;
        static TWO_PLAYER: [SlotHold; 2] = [primary(10, 11, 1), primary(26, 27, 1)];
        static SOLO_SWAP: [SlotHold; 3] = [swap(5, 6), primary(10, 11, 0), primary(26, 27, 0)];
        static ENEMY: [EnemySpawn; 1] = [EnemySpawn {
            x: 600,
            ai: EnemyAi::Passive,
        }];
        for (name, script, two_player) in [
            ("t_audience_2p", &TWO_PLAYER[..], true),
            ("t_audience_swap", &SOLO_SWAP[..], false),
        ] {
            let script: &'static [SlotHold] = script;
            let f = fixture(
                name,
                &ENEMY,
                script,
                DonutSetup::unlocked(8),
                two_player,
                120,
            );
            let mut s = sim(f);
            s.run_scripted(120);
            let defeats: Vec<_> = s
                .combat
                .events
                .iter()
                .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
                .collect();
            assert_eq!(defeats.len(), 1, "{name}: the bolts kill the goblin");
            let mut audience = AudienceState::locked();
            audience.unlock();
            let inputs: Vec<_> = s
                .combat
                .events
                .iter()
                .filter_map(AudienceInput::from_combat)
                .collect();
            audience.step(&inputs);
            let (carl, donut) = (
                audience.count(HeroId::Carl).unwrap(),
                audience.count(HeroId::Donut).unwrap(),
            );
            assert_eq!(carl, TUNING.baseline, "{name}: Carl is never credited");
            assert!(donut > TUNING.baseline, "{name}: Donut is credited");
        }
    }

    #[test]
    fn a_hurt_during_the_gather_cancels_the_cast_at_no_cost() {
        static SCRIPT: [SlotHold; 1] = [primary(10, 11, 1)];
        let f = fixture(
            "t_cast_hurt",
            &[],
            &SCRIPT,
            DonutSetup::unlocked(8),
            true,
            30,
        );
        let mut s = sim(f);
        s.run_scripted(11);
        assert!(s.combat.donut_cast().is_some());
        // A goblin's blow lands on Donut during the gather (cast tick 1).
        s.combat.donut.hurt_ticks = 18;
        s.run_scripted(20);
        assert!(s.combat.donut_cast().is_none());
        assert_eq!(s.combat.donut_mana_view(), Some((8, 8)), "nothing spent");
    }

    #[test]
    fn a_press_during_the_follow_through_fires_when_the_cast_ends() {
        // Pressed on cast tick 10 of a cast started on tick 10: it fires on 26.
        static SCRIPT: [SlotHold; 2] = [primary(10, 11, 1), primary(20, 21, 1)];
        let f = fixture(
            "t_cast_buffer",
            &[],
            &SCRIPT,
            DonutSetup::unlocked(8),
            true,
            40,
        );
        let mut s = sim(f);
        s.run_scripted(40);
        let starts: Vec<u64> = s
            .combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::MoveStarted {
                    tick,
                    move_id: MoveId::MagicMissile,
                    ..
                } => Some(tick),
                _ => None,
            })
            .collect();
        assert_eq!(starts, vec![10, 26], "re-fire on cast tick 16");
    }

    // -------------------------------------------------------------- moves --

    #[test]
    fn a_pounce_cancels_an_unreleased_cast() {
        static SCRIPT: [SlotHold; 2] = [primary(10, 11, 1), physical(12, 13, 1)];
        let f = fixture(
            "t_cast_pounce",
            &[],
            &SCRIPT,
            DonutSetup::unlocked(8),
            true,
            40,
        );
        let mut s = sim(f);
        s.run_scripted(40);
        assert_eq!(
            s.combat.donut_mana_view(),
            Some((8, 8)),
            "the gather was cancelled before the release"
        );
    }

    #[test]
    fn with_donut_moves_off_no_ability_runs_and_the_kit_never_hashes() {
        static SCRIPT: [SlotHold; 2] = [primary(10, 11, 1), physical(30, 31, 1)];
        let f: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            tuning: PartyTuning {
                donut_moves: false,
                ..PROVISIONAL_TUNING
            },
            ..PartyFixture {
                base: fixture("t_off", &[], &SCRIPT, DonutSetup::unlocked(8), true, 60).base,
                two_player: true,
                tuning: PROVISIONAL_TUNING,
                script: &SCRIPT,
                pilot: Pilot::None,
                donut: DonutSetup::unlocked(8),
            }
        }));
        let mut s = sim(f);
        s.run_scripted(60);
        assert!(s.combat.events.is_empty());
        assert!(s.combat.donut_move_view().is_none() && s.combat.donut_cast().is_none());
    }

    #[test]
    fn a_missile_on_the_boss_is_a_one_damage_hero_hit_from_hero_one() {
        use crate::boss_scene::{FULL, boss_base, load_arena};
        static BASE: Fixture = boss_base("t_bolt_boss", "test", 1, 380, FULL, 40);
        static SCRIPT: [SlotHold; 1] = [primary(10, 11, 1)];
        static F: PartyFixture = PartyFixture {
            base: &BASE,
            two_player: true,
            tuning: PROVISIONAL_TUNING,
            script: &SCRIPT,
            pilot: Pilot::None,
            donut: DonutSetup::unlocked(8),
        };
        let kit = load_arena(&PathBuf::from(env!("CARGO_MANIFEST_DIR"))).unwrap();
        let mut s = PartyFixtureSim::new(&kit, 360, &F).unwrap();
        s.run_scripted(40);
        let hurt: Vec<_> = s
            .combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::BossHurt {
                    source,
                    damage,
                    hp_left,
                    ..
                } => Some((source, damage, hp_left)),
                _ => None,
            })
            .collect();
        assert_eq!(hurt, vec![(1, 1, 35)]);
        assert!(s.combat.donut_bolts().is_empty(), "the bolt was consumed");
    }
}
