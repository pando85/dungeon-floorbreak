//! BOSB: the Tuskling machine, the small tusked rusher the Ball of Swine sheds
//! (`docs/01-design/floor-01-bos-b-tusklings.md`, BOSB-AD1..AD9, AD14). Pure,
//! Bevy-free and deterministic at the 60 Hz tick: no random draw, no wall clock.
//!
//! Every number is data ([`TusklingData`], in `assets/data/bosses/ball_boss_f01.json`,
//! each with its BOSB-AD id). The adapter that makes it an ENEMY-SLOT enemy is
//! [`crate::combat_scene::tuskling`]; the spawn rule (qualifying impacts, caps, budget,
//! legal spawn) is there too, because it reads the boss and the heroes.
//!
//! A Tuskling never respawns and never reassembles (BOSB-AD6). It is a plain
//! mover: it chases the nearest hero, tells for 28 ticks (8 onset + 20 hold, the art's
//! frames 7 and 8), lunges for 3 and strikes for 6 (frames 9 and 10), recovers, and
//! cools down. A new one scrambles first (BOSB-AD5): it moves with no attack.
//! When the Ball is defeated it cowers (BOSB-AD9): harmless and non-blocking.

use crate::boss::{HeroBlow, HeroView, TusklingData};
use crate::combat::solid_at;
use crate::feel::{GOBLIN_WALK_SPEED, SWING_Y_MAX};
use crate::party::HeroId;
use crate::prototype::{Block, Rect};
use crate::scene_enemy::SLOT_ID_MIN;

const TICK: f32 = 1.0 / 60.0;

/// The `HeroBlow::source` base of a Tuskling (the ASCII "TU"). The spawn index goes in the
/// high half ([`tusk_source`]).
pub const TUSK_SOURCE_ID: u32 = 0x5455;

/// The first Tuskling target id: above every other slot id of the Ball's room.
pub const TUSK_ID_BASE: u32 = SLOT_ID_MIN + 0x500;

/// The source of the Tuskling with spawn index `index`.
pub const fn tusk_source(index: u8) -> u32 {
    TUSK_SOURCE_ID | ((index as u32) << 16)
}

/// The slot target id of the Tuskling with spawn index `index`.
pub const fn target_id(index: u8) -> u32 {
    TUSK_ID_BASE + index as u32
}

/// The attack damage of BOSB-AD6: half the goblin jab damage, rounded up, at least 1. The
/// goblin's jab deals 1 to Carl (`hurt_hero_by`), so the rule gives 1 today.
pub fn attack_damage(d: &TusklingData) -> u8 {
    const GOBLIN_JAB_DAMAGE: u8 = 1;
    GOBLIN_JAB_DAMAGE
        .div_ceil(d.damage_goblin_jab_divisor.max(1))
        .max(1)
}

/// The hit points of BOSB-AD6: `hp_punches` of Carl's jab damage.
pub fn max_hp(d: &TusklingData) -> u16 {
    u16::from(d.hp_punches) * u16::from(crate::combat::CARL_JAB.damage)
}

/// What a Tuskling does, as a combat-log sub-event (`CombatEvent::Tuskling`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuskAct {
    /// Detached from the Ball at pivot `x` (px, rounded).
    Spawned {
        x: i16,
    },
    /// A detach that was skipped: `0` the live cap, `1` the budget, `2` no legal spot.
    Skipped {
        reason: u8,
    },
    TellStart,
    Strike,
    Hurt,
    /// Launched by Carl's kick; the launch is the kick's move instance.
    Kicked {
        launch: u32,
    },
    /// A kicked Tuskling reached the Ball; `staggered` says whether the Ball staggered.
    KickContact {
        launch: u32,
        staggered: bool,
    },
    Cower,
}

impl TuskAct {
    pub fn name(self) -> &'static str {
        match self {
            Self::Spawned { .. } => "spawned",
            Self::Skipped { .. } => "skipped",
            Self::TellStart => "tell",
            Self::Strike => "strike",
            Self::Hurt => "hurt",
            Self::Kicked { .. } => "kicked",
            Self::KickContact { .. } => "kick_contact",
            Self::Cower => "cower",
        }
    }

    /// The `key=value` fields of the log line (empty for none).
    pub fn fields(self) -> String {
        match self {
            Self::Spawned { x } => format!("x={x}"),
            Self::Skipped { reason } => format!("reason={reason}"),
            Self::Kicked { launch } => format!("launch={launch}"),
            Self::KickContact { launch, staggered } => {
                format!("launch={launch} staggered={}", u8::from(staggered))
            }
            _ => String::new(),
        }
    }

    pub fn mix_into(self, mix: &mut impl FnMut(u64)) {
        let (tag, a, b) = match self {
            Self::Spawned { x } => (1, x as i64 as u64, 0),
            Self::Skipped { reason } => (2, u64::from(reason), 0),
            Self::TellStart => (3, 0, 0),
            Self::Strike => (4, 0, 0),
            Self::Hurt => (5, 0, 0),
            Self::Kicked { launch } => (6, u64::from(launch), 0),
            Self::KickContact { launch, staggered } => (7, u64::from(launch), u64::from(staggered)),
            Self::Cower => (8, 0, 0),
        };
        mix(tag);
        mix(a);
        mix(b);
    }
}

/// The machine's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Chasing the nearest hero (and, while it scrambles, with no attack).
    Chase,
    /// The attack tell (art frames 7, 8).
    Tell,
    /// The 3-tick lunge (frame 9).
    Lunge,
    /// The strike, hit box live (frame 10).
    Strike,
    /// The strike's recovery (frame 11).
    Recover,
    /// Flinching (frame 12).
    Hurt,
    /// Flying from Carl's kick (frame 12).
    Kicked,
    /// Defeated: lies on its back (frames 13..15), harmless and not hurt again.
    Down,
    /// The Ball is down: it cowers (BOSB-AD9).
    Cower,
}

/// A kick in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Kick {
    launch: u32,
    ticks_left: u16,
    dir: i8,
    px_per_tick_bits: u32,
}

/// What a hit on a Tuskling does to it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TuskHit {
    pub damage: u16,
    /// The way the blow travels (+1 / -1).
    pub dir: i8,
    /// A kick that launches (`spec.reaction.launch`): the move instance and its speed (px/s).
    pub launch: Option<(u32, f32)>,
}

/// The result of a hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TuskOutcome {
    pub hp_left: u16,
    pub defeated: bool,
}

/// What the scene tells a Tuskling each tick.
#[derive(Debug, Clone, Copy)]
pub struct TuskCtx<'a> {
    pub tick: u64,
    pub heroes: &'a [HeroView; 2],
    pub solids: &'a [Block],
    /// The room's walkable x range.
    pub walls: (f32, f32),
    /// The scene's last tell start (any enemy).
    pub last_tell_start: Option<u64>,
}

/// What one step asks the scene to do.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TuskStep {
    pub acts: Vec<TuskAct>,
    pub blows: Vec<HeroBlow>,
    pub tell_started: bool,
}

/// One Tuskling.
#[derive(Debug, Clone)]
pub struct Tuskling {
    data: &'static TusklingData,
    /// The spawn index (0..budget): the costume (even tux, odd gown) and the id.
    pub index: u8,
    pub x: f32,
    pub foot_y: f32,
    pub facing: i8,
    pub hp: u16,
    pub state: State,
    state_tick: u16,
    scramble_left: u16,
    cooldown: u16,
    target: Option<HeroId>,
    strike_hit: [bool; 2],
    kick: Option<Kick>,
    /// Ticks since the spawn (the presenter's walk cycle and the idle loop).
    pub age: u64,
    /// The tick the last step moved it (presentation read, hashed with the position).
    moved: bool,
}

impl Tuskling {
    pub fn new(data: &'static TusklingData, index: u8, x: f32, foot_y: f32, facing: i8) -> Self {
        Self {
            data,
            index,
            x,
            foot_y,
            facing,
            hp: max_hp(data),
            state: State::Chase,
            state_tick: 0,
            scramble_left: data.scramble_ticks,
            cooldown: 0,
            target: None,
            strike_hit: [false; 2],
            kick: None,
            age: 0,
            moved: false,
        }
    }

    pub fn data(&self) -> &'static TusklingData {
        self.data
    }

    /// Alive: stepped, hit and drawn as a living body (a cowering one is alive).
    pub fn alive(&self) -> bool {
        self.state != State::Down
    }

    /// It is in the first `scramble_ticks` of its life: it moves and cannot attack.
    pub fn scrambling(&self) -> bool {
        self.scramble_left > 0
    }

    /// Ticks the current state has run.
    pub fn state_tick(&self) -> u16 {
        self.state_tick
    }

    /// It moved on the last step (the presenter picks the walk frames from it).
    pub fn moving(&self) -> bool {
        self.moved
    }

    /// The tell's total ticks (onset + hold).
    pub fn tell_ticks(&self) -> u16 {
        self.data.tell_onset_ticks + self.data.tell_hold_ticks
    }

    /// The ticks from the first tell tick to the first strike tick (the art's 31).
    pub fn first_contact_ticks(&self) -> u16 {
        self.tell_ticks() + self.data.lunge_ticks
    }

    fn walk_px_per_tick(&self) -> f32 {
        GOBLIN_WALK_SPEED * self.data.walk_goblin_mult * TICK
    }

    /// Pivot-relative forward box mirrored by the facing, y up from the feet.
    fn forward_rect(&self, x: [f32; 2], y: [f32; 2]) -> Rect {
        let left = if self.facing >= 0 {
            self.x + x[0]
        } else {
            self.x - x[1]
        };
        Rect {
            left,
            bottom: self.foot_y + y[0],
            width: x[1] - x[0],
            height: y[1] - y[0],
        }
    }

    /// The hurtbox: none while down (the `down` frames hold no hurtbox).
    pub fn hurtbox(&self) -> Option<Rect> {
        (self.state != State::Down)
            .then(|| self.forward_rect(self.data.hurt_box.x, self.data.hurt_box.y))
    }

    /// The collision body (a mover is blocked by solids with this box).
    pub fn body(&self) -> Rect {
        let half = self.data.body_half_width;
        Rect {
            left: self.x - half,
            bottom: self.foot_y + 0.5,
            width: half * 2.0,
            height: self.data.hurt_box.y[1] - 1.0,
        }
    }

    /// The strike box while the strike is live: from the body's front, `reach_px` long.
    pub fn strike_box(&self) -> Option<Rect> {
        (self.state == State::Strike).then(|| {
            let near = self.data.body_half_width;
            self.forward_rect([near, near + self.data.reach_px], self.data.strike_y)
        })
    }

    /// The reach of a tell that would start now: the far edge of the strike box after the
    /// lunge, measured from the pivot.
    pub fn strike_reach(&self) -> f32 {
        self.data.body_half_width + self.data.lunge_px + self.data.reach_px
    }

    /// The kick in flight: its launch and the flying body (`None` unless kicked).
    pub fn kick_flight(&self) -> Option<(u32, Rect)> {
        (self.state == State::Kicked)
            .then(|| self.kick.map(|k| (k.launch, self.body())))
            .flatten()
    }

    fn blocked(&self, solids: &[Block], x: f32) -> bool {
        let mut body = self.body();
        body.left = x - self.data.body_half_width;
        solid_at(solids, body)
    }

    /// Step toward `dx` px (walls and solids stop it). Returns whether it moved.
    fn shift(&mut self, ctx: &TuskCtx<'_>, dx: f32) -> bool {
        let half = self.data.body_half_width;
        let to = (self.x + dx).clamp(ctx.walls.0 + half, ctx.walls.1 - half);
        if (to - self.x).abs() < f32::EPSILON || self.blocked(ctx.solids, to) {
            return false;
        }
        self.x = to;
        true
    }

    fn enter(&mut self, state: State) {
        self.state = state;
        self.state_tick = 0;
    }

    /// The nearest present hero (ties go to Carl), keeping the held one unless the other is
    /// 24 px nearer.
    fn pick_target(&mut self, heroes: &[HeroView; 2]) {
        let dist = |h: &HeroView| (h.x - self.x).abs();
        let nearest = heroes.iter().filter(|h| h.present).min_by(|a, b| {
            dist(a)
                .total_cmp(&dist(b))
                .then(a.id.index().cmp(&b.id.index()))
        });
        let held = self
            .target
            .and_then(|id| heroes.iter().find(|h| h.id == id && h.present));
        self.target = match (held, nearest) {
            (Some(kept), Some(best)) if best.id != kept.id && dist(best) + 24.0 <= dist(kept) => {
                Some(best.id)
            }
            (Some(kept), _) => Some(kept.id),
            (None, best) => best.map(|h| h.id),
        };
    }

    /// One tick.
    pub fn step(&mut self, ctx: &TuskCtx<'_>, out: &mut TuskStep) {
        self.moved = false;
        if self.state == State::Down {
            return;
        }
        self.age += 1;
        self.state_tick = self.state_tick.saturating_add(1);
        let d = self.data;
        match self.state {
            State::Down => {}
            State::Cower => {}
            State::Chase => self.chase(ctx, out),
            State::Tell => {
                if self.state_tick >= self.tell_ticks() {
                    self.enter(State::Lunge);
                }
            }
            State::Lunge => {
                let per_tick = d.lunge_px / f32::from(d.lunge_ticks.max(1));
                let dx = f32::from(self.facing) * per_tick;
                self.moved = self.shift(ctx, dx);
                if self.state_tick >= d.lunge_ticks {
                    self.strike_hit = [false; 2];
                    self.enter(State::Strike);
                    out.acts.push(TuskAct::Strike);
                    // The box is live on the tick it opens: the first contact is the 31st tick
                    // of the tell (the art's 8 + 20 + 3).
                    self.strike(ctx, out);
                }
            }
            State::Strike => {
                // Tick 0 of the strike ran on the lunge's last tick: `active_ticks` in all.
                if self.state_tick >= d.active_ticks {
                    self.enter(State::Recover);
                    self.cooldown = d.cooldown_ticks;
                } else {
                    self.strike(ctx, out);
                }
            }
            State::Recover => {
                if self.state_tick >= d.recover_ticks {
                    self.enter(State::Chase);
                }
            }
            State::Hurt => {
                if self.state_tick >= d.hurt_ticks {
                    self.enter(State::Chase);
                }
            }
            State::Kicked => self.fly(ctx),
        }
        self.cooldown = self.cooldown.saturating_sub(1);
        if self.scramble_left > 0 && self.state != State::Cower {
            self.scramble_left -= 1;
        }
    }

    fn chase(&mut self, ctx: &TuskCtx<'_>, out: &mut TuskStep) {
        self.pick_target(ctx.heroes);
        let Some(hero) = self
            .target
            .map(|id| &ctx.heroes[id.index()])
            .filter(|h| h.present)
        else {
            return;
        };
        let gap = hero.x - self.x;
        if gap.abs() > 4.0 {
            self.facing = if gap >= 0.0 { 1 } else { -1 };
        }
        let trigger = self.strike_reach() - self.data.trigger_margin_px;
        let ready = self.scramble_left == 0
            && self.cooldown == 0
            && gap.abs() <= trigger
            && ctx.last_tell_start.is_none_or(|start| {
                ctx.tick.saturating_sub(start) >= u64::from(self.data.tell_spacing_ticks)
            });
        if ready {
            self.enter(State::Tell);
            out.acts.push(TuskAct::TellStart);
            out.tell_started = true;
            return;
        }
        if gap.abs() > trigger * 0.5 {
            let dx = f32::from(self.facing) * self.walk_px_per_tick();
            self.moved = self.shift(ctx, dx);
        }
    }

    fn strike(&mut self, ctx: &TuskCtx<'_>, out: &mut TuskStep) {
        let Some(area) = self.strike_box() else {
            return;
        };
        let damage = attack_damage(self.data);
        let source = tusk_source(self.index);
        for hero in ctx.heroes.iter().filter(|h| h.present && h.vulnerable) {
            let i = hero.id.index();
            if !self.strike_hit[i] && hero.hurt.overlaps(area) {
                self.strike_hit[i] = true;
                out.blows.push(HeroBlow {
                    hero: hero.id,
                    source,
                    damage,
                    from_x: self.x,
                    knock_vx: 0.0,
                    knock_vy: 0.0,
                });
            }
        }
    }

    fn fly(&mut self, ctx: &TuskCtx<'_>) {
        let Some(kick) = self.kick else {
            self.enter(State::Hurt);
            return;
        };
        let dx = f32::from(kick.dir) * f32::from_bits(kick.px_per_tick_bits);
        let moved = self.shift(ctx, dx);
        self.moved = moved;
        let left = kick.ticks_left.saturating_sub(1);
        if left == 0 || !moved {
            self.kick = None;
            self.enter(State::Hurt);
        } else {
            self.kick = Some(Kick {
                ticks_left: left,
                ..kick
            });
        }
    }

    /// A landed hit. `None`: it takes none now (down). A non-lethal hit flinches it (it
    /// cancels a tell, a lunge and a strike), a launching kick flies it; a lethal one lays
    /// it down. A hit with no damage (Donut's swat) flinches it.
    pub fn hit(&mut self, hit: &TuskHit, out: &mut TuskStep) -> Option<TuskOutcome> {
        if self.state == State::Down {
            return None;
        }
        self.hp = self.hp.saturating_sub(hit.damage);
        self.kick = None;
        self.strike_hit = [false; 2];
        if self.hp == 0 {
            self.enter(State::Down);
            return Some(TuskOutcome {
                hp_left: 0,
                defeated: true,
            });
        }
        if let Some((launch, px_s)) = hit.launch {
            self.kick = Some(Kick {
                launch,
                ticks_left: self.data.kick_flight_ticks.max(1),
                dir: hit.dir,
                px_per_tick_bits: (px_s.abs() * TICK).to_bits(),
            });
            self.enter(State::Kicked);
            out.acts.push(TuskAct::Kicked { launch });
        } else {
            self.enter(State::Hurt);
            out.acts.push(TuskAct::Hurt);
        }
        // A flinch pushes the cooldown, so a punched Tuskling does not strike at once.
        self.cooldown = self.cooldown.max(self.data.hurt_ticks);
        Some(TuskOutcome {
            hp_left: self.hp,
            defeated: false,
        })
    }

    /// The Ball is down: it cowers (BOSB-AD9). A defeated Tuskling stays down.
    pub fn cower(&mut self, out: &mut TuskStep) {
        if matches!(self.state, State::Down | State::Cower) {
            return;
        }
        self.kick = None;
        self.enter(State::Cower);
        out.acts.push(TuskAct::Cower);
    }

    /// Fold the whole state into a state hash.
    pub fn mix_into(&self, mix: &mut dyn FnMut(u64)) {
        mix(u64::from(self.index));
        mix(u64::from(self.x.to_bits()));
        mix(u64::from(self.foot_y.to_bits()));
        mix(self.facing as i64 as u64);
        mix(u64::from(self.hp));
        mix(self.state as u64);
        mix(u64::from(self.state_tick));
        mix(u64::from(self.scramble_left));
        mix(u64::from(self.cooldown));
        mix(self.target.map_or(0, |t| t.index() as u64 + 1));
        mix(u64::from(self.strike_hit[0]) | u64::from(self.strike_hit[1]) << 1);
        match self.kick {
            Some(k) => {
                mix(1);
                mix(u64::from(k.launch));
                mix(u64::from(k.ticks_left));
                mix(k.dir as i64 as u64);
                mix(u64::from(k.px_per_tick_bits));
            }
            None => mix(0),
        }
        mix(self.age);
        mix(u64::from(self.moved));
    }
}

/// Whether a hero could be hit by a Tuskling's strike box at the head of the strike (a
/// helper for the pilots): `true` when the hero stands within the box's reach band.
pub fn in_strike_band(t: &Tuskling, hero_x: f32, hero_foot_y: f32) -> bool {
    let gap = (hero_x - t.x) * f32::from(t.facing);
    gap > 0.0 && gap <= t.strike_reach() && hero_foot_y < t.foot_y + SWING_Y_MAX
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{Attacker, CARL_JAB, CARL_PUSH_KICK, hit_box};
    use crate::feel::GOBLIN_WALK_SPEED;

    fn data() -> &'static TusklingData {
        crate::boss::ball()
            .tusklings
            .as_ref()
            .expect("the Ball sheds Tusklings")
    }

    fn view(id: HeroId, x: f32) -> HeroView {
        HeroView {
            id,
            present: true,
            x,
            foot_y: 0.0,
            grounded: true,
            hurt: Rect {
                left: x - 6.0,
                bottom: 0.0,
                width: 12.0,
                height: 50.0,
            },
            vulnerable: true,
            bot: id == HeroId::Donut,
        }
    }

    fn heroes(carl_x: f32) -> [HeroView; 2] {
        let mut gone = view(HeroId::Donut, 5000.0);
        gone.present = false;
        [view(HeroId::Carl, carl_x), gone]
    }

    fn ctx<'a>(tick: u64, heroes: &'a [HeroView; 2]) -> TuskCtx<'a> {
        TuskCtx {
            tick,
            heroes,
            solids: &[],
            walls: (0.0, 640.0),
            last_tell_start: None,
        }
    }

    /// A Tuskling at x = 200 facing right, with Carl `gap` px ahead.
    fn duel(gap: f32) -> (Tuskling, [HeroView; 2]) {
        (Tuskling::new(data(), 0, 200.0, 0.0, 1), heroes(200.0 + gap))
    }

    fn run(t: &mut Tuskling, h: &[HeroView; 2], from: u64, ticks: u64) -> Vec<(u64, TuskStep)> {
        (from..from + ticks)
            .map(|tick| {
                let mut step = TuskStep::default();
                t.step(&ctx(tick, h), &mut step);
                (tick, step)
            })
            .collect()
    }

    #[test]
    fn the_numbers_are_the_adopted_ones_and_all_data() {
        let d = data();
        assert_eq!((d.first_detach, d.later_detach), (2, 1), "BOSB-AD3");
        assert_eq!((d.max_live, d.budget), (3, 5), "BOSB-AD4");
        assert_eq!((d.clearance_px, d.scramble_ticks), (24.0, 20), "BOSB-AD5");
        assert_eq!(d.hp_punches, 2, "BOSB-AD6");
        assert_eq!(
            (
                d.tell_onset_ticks,
                d.tell_hold_ticks,
                d.lunge_ticks,
                d.cooldown_ticks
            ),
            (8, 20, 3, 60),
            "BOSB-AD7"
        );
        assert_eq!(d.reach_px, 20.0, "BOSB-AD7");
        assert_eq!(d.kick_stagger_ticks, 20, "BOSB-AD8");
        assert_eq!(crate::boss::ball().crash.self_damage, 3, "BOSB-AD12");
        // BOSB-AD6: HP is two punch_a hits, the damage half the goblin jab, rounded up, at least 1.
        assert_eq!(max_hp(d), 2 * u16::from(CARL_JAB.damage));
        assert_eq!(attack_damage(d), 1);
        // BOSB-AD7: 1.2 x the goblin's walk speed.
        let t = Tuskling::new(d, 0, 0.0, 0.0, 1);
        assert!((t.walk_px_per_tick() * 60.0 - 1.2 * GOBLIN_WALK_SPEED).abs() < 1e-3);
        assert_eq!(
            t.first_contact_ticks(),
            31,
            "BSW-11: the first contact at 31 ticks"
        );
        // The Juicer sheds none.
        assert!(crate::boss::builtin().tusklings.is_none());
        assert_eq!(crate::boss::builtin().crash.self_damage, 0);
    }

    #[test]
    fn a_new_tuskling_scrambles_for_twenty_ticks_with_no_attack() {
        // Carl stands inside its reach from the first tick.
        let (mut t, h) = duel(20.0);
        let log = run(&mut t, &h, 1, 20);
        assert!(
            log.iter()
                .all(|(_, s)| s.acts.is_empty() && s.blows.is_empty()),
            "no tell and no blow while it scrambles"
        );
        assert!(!t.scrambling(), "the scramble is over after 20 ticks");
        let log = run(&mut t, &h, 21, 1);
        assert_eq!(
            log[0].1.acts,
            [TuskAct::TellStart],
            "it tells at once afterwards"
        );
    }

    #[test]
    fn the_scramble_moves_it_toward_the_hero() {
        let (mut t, h) = duel(300.0);
        let x0 = t.x;
        run(&mut t, &h, 1, 20);
        assert!(t.x > x0 + 20.0, "it closed in: {} from {x0}", t.x);
    }

    #[test]
    fn the_first_contact_comes_thirty_one_ticks_after_the_tell_starts() {
        let (mut t, h) = duel(20.0);
        let log = run(&mut t, &h, 1, 80);
        let tell = log
            .iter()
            .find(|(_, s)| s.acts.contains(&TuskAct::TellStart))
            .unwrap()
            .0;
        let first = log.iter().find(|(_, s)| !s.blows.is_empty()).unwrap().0;
        assert_eq!(first - tell, 31, "tell {tell}, first blow {first}");
        let blow = log
            .iter()
            .find(|(_, s)| !s.blows.is_empty())
            .unwrap()
            .1
            .blows[0];
        assert_eq!(blow.damage, attack_damage(data()));
        assert_eq!(blow.source, tusk_source(0));
        // One blow per hero and strike, not one per active tick.
        let all: usize = log.iter().map(|(_, s)| s.blows.len()).sum();
        assert_eq!(all, 1);
    }

    #[test]
    fn a_jump_clears_the_strike_band() {
        let (mut t, mut h) = duel(20.0);
        // Feet above the band top: the hero's box (50 tall) is lifted clear of 12..24 above the
        // tuskling's feet only when its bottom passes 24.
        h[0].foot_y = 30.0;
        h[0].hurt.bottom = 30.0;
        let log = run(&mut t, &h, 1, 80);
        assert!(
            log.iter().all(|(_, s)| s.blows.is_empty()),
            "the strike passes under"
        );
    }

    #[test]
    fn two_punch_a_hits_lay_one_down_and_the_second_kick_plus_punch_too() {
        let (mut t, _) = duel(20.0);
        let mut step = TuskStep::default();
        let punch = TuskHit {
            damage: u16::from(CARL_JAB.damage),
            dir: 1,
            launch: None,
        };
        let first = t.hit(&punch, &mut step).unwrap();
        assert_eq!((first.hp_left, first.defeated), (max_hp(data()) - 1, false));
        assert_eq!(t.state, State::Hurt);
        let second = t.hit(&punch, &mut step).unwrap();
        assert!(second.defeated);
        assert!(!t.alive());
        assert!(
            t.hit(&punch, &mut step).is_none(),
            "a body on the floor takes no hit"
        );
        // A kick (damage 1) plus a punch also kills.
        let (mut u, _) = duel(20.0);
        let kick = TuskHit {
            damage: u16::from(CARL_PUSH_KICK.damage),
            dir: 1,
            launch: Some((7, 200.0)),
        };
        assert!(!u.hit(&kick, &mut step).unwrap().defeated);
        assert!(u.hit(&punch, &mut step).unwrap().defeated);
    }

    #[test]
    fn carls_jab_reaches_its_hurtbox_at_a_lunges_distance_and_the_art_box_would_miss() {
        // The jab's box is 29..35 above the feet: the hurtbox must reach it (the art's 27 would not).
        // Carl at 170 faces right at a Tuskling at 200 that faces him: the 30 px lunge distance.
        let t = Tuskling::new(data(), 0, 200.0, 0.0, -1);
        let attacker = Attacker {
            x: 170.0,
            y: 26.0,
            half_height: 26.0,
            facing: 1,
            player: 0,
        };
        let jab = hit_box(&CARL_JAB.shape, &attacker).unwrap();
        assert!(
            t.hurtbox().unwrap().overlaps(jab),
            "{jab:?} vs {:?}",
            t.hurtbox()
        );
        assert!(t.hurtbox().unwrap().top() > 29.0 && 27.0 <= 29.0);
    }

    #[test]
    fn a_hit_on_the_tell_cancels_it_and_it_does_not_strike() {
        let (mut t, h) = duel(20.0);
        run(&mut t, &h, 1, 30);
        assert_eq!(t.state, State::Tell);
        let mut step = TuskStep::default();
        t.hit(
            &TuskHit {
                damage: 1,
                dir: 1,
                launch: None,
            },
            &mut step,
        );
        assert_eq!(t.state, State::Hurt);
        let log = run(&mut t, &h, 31, 14);
        assert!(log.iter().all(|(_, s)| s.blows.is_empty()));
    }

    #[test]
    fn a_kick_flies_it_once_for_the_data_ticks_then_it_flinches() {
        let (mut t, _) = duel(20.0);
        let mut step = TuskStep::default();
        t.hit(
            &TuskHit {
                damage: 1,
                dir: -1,
                launch: Some((9, 240.0)),
            },
            &mut step,
        );
        assert_eq!(step.acts, [TuskAct::Kicked { launch: 9 }]);
        assert_eq!(t.kick_flight().map(|(l, _)| l), Some(9));
        let x0 = t.x;
        let far = heroes(600.0);
        run(&mut t, &far, 1, u64::from(data().kick_flight_ticks));
        assert!(t.x < x0 - 30.0, "it flew left: {} from {x0}", t.x);
        assert_eq!(t.state, State::Hurt);
        assert!(t.kick_flight().is_none());
    }

    #[test]
    fn a_cowering_tuskling_is_harmless_and_stays_put() {
        let (mut t, h) = duel(20.0);
        run(&mut t, &h, 1, 25);
        let mut step = TuskStep::default();
        t.cower(&mut step);
        assert_eq!(step.acts, [TuskAct::Cower]);
        let x = t.x;
        let log = run(&mut t, &h, 26, 120);
        assert!(
            log.iter()
                .all(|(_, s)| s.blows.is_empty() && s.acts.is_empty())
        );
        assert_eq!(t.x, x);
        assert!(t.alive(), "cowering is alive; a defeated one stays down");
        // A down body does not cower.
        let (mut d, _) = duel(20.0);
        d.hit(
            &TuskHit {
                damage: 9,
                dir: 1,
                launch: None,
            },
            &mut step,
        );
        d.cower(&mut TuskStep::default());
        assert_eq!(d.state, State::Down);
    }

    #[test]
    fn the_step_is_deterministic() {
        let hash = |t: &Tuskling| {
            let mut acc = 0u64;
            t.mix_into(&mut |v| acc = acc.wrapping_mul(0x100_0000_01b3) ^ v);
            acc
        };
        let (mut a, h) = duel(120.0);
        let (mut b, _) = duel(120.0);
        run(&mut a, &h, 1, 200);
        run(&mut b, &h, 1, 200);
        assert_eq!(hash(&a), hash(&b));
    }
}
