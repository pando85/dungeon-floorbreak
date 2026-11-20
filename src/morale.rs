//! CF02-G goblin morale core (Q57..Q68).
//!
//! Pure, Bevy-free and deterministic: integers and fixed orders only, no wall
//! clock and no RNG stream. Every function reads [`Cf02Tuning`]; **nothing is
//! wired here** (CF02-G wires it into the goblins). Morale exists only under
//! `cf02_rules`, so no function checks the master switch except where a switch
//! knob is read through [`Cf02Tuning::on`].
//!
//! Per fixed tick, for each goblin in index order, after the tick's combat
//! events, the caller (1) builds [`event_delta`] from the tick's events and
//! (2) calls [`Morale::step`]. Taunt and back-off are started separately
//! ([`Morale::try_taunt`], [`Morale::try_backoff`]) because their triggers are
//! world conditions, not nerve events.
//!
//! **Call order inside one tick** (the Q64 and Q65 tests count the start tick as
//! tick 1 under exactly this order): `step`, then `try_backoff` / `try_taunt`,
//! then apply [`Morale::backoff_step_px`] and the movement. Calling `try_*`
//! before `step` shifts the count by one tick (the hop covers 27 px over 9
//! ticks and the taunt lasts 35 ticks).

use crate::cf02::{Cf02Tuning, Force, SpectacularCause, Temperament};

// ---- Q58: the stateless seeded draw -------------------------------------

/// One SplitMix64 output for state `x` (the generator's first output when
/// seeded with `x`). `sm(0) == 0xe220_a839_7b1d_cdaf`.
pub const fn sm(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Q58: `sm(sm(sm(sm(seed) ^ restarts) ^ goblin) ^ draw)`. Chaining the mix
/// after every field keeps close seeds, restarts, goblins and draws from
/// aliasing. `seed` is the effective seed.
pub const fn mix(seed: u64, restarts: u32, goblin: u32, draw: u32) -> u64 {
    sm(sm(sm(sm(seed) ^ restarts as u64) ^ goblin as u64) ^ draw as u64)
}

/// Q58 draw 0: Fisher-Yates shuffle of the temperament bag. For slot `i` from
/// `n - 1` down to 1, swap slot `i` with `j = mix(seed, restarts, i, 0) % (i + 1)`.
pub fn shuffled_bag(t: &Cf02Tuning, seed: u64, restarts: u32) -> Vec<Temperament> {
    let mut bag = t.q58_bag.clone();
    for i in (1..bag.len()).rev() {
        let j = (mix(seed, restarts, i as u32, 0) % (i as u64 + 1)) as usize;
        bag.swap(i, j);
    }
    bag
}

/// A value in `lo..=hi` from a hash.
fn ranged(lo: i32, hi: i32, h: u64) -> i32 {
    let span = i64::from(hi) - i64::from(lo) + 1;
    (i64::from(lo) + (h % span as u64) as i64) as i32
}

/// Q59: the baseline nerve of a temperament.
pub fn baseline(t: &Cf02Tuning, temperament: Temperament) -> u8 {
    match temperament {
        Temperament::Brawler => t.q59_baseline_brawler,
        Temperament::ShowOff => t.q59_baseline_show_off,
        Temperament::Coward => t.q59_baseline_coward,
    }
}

/// The four Q58 draws of one goblin for one attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Draw {
    /// Draw 0: the bag slot (`goblin % bag length`) of the shuffled bag.
    pub temperament: Temperament,
    /// Draw 1: nerve offset.
    pub nerve_offset: i8,
    /// Draw 2: panic length in ticks.
    pub panic_len: u8,
    /// Draw 3: first-approach delay in ticks.
    pub approach_delay: u8,
}

/// All draws of goblin `goblin` for `(seed, restarts)`; rooms with more goblins
/// than the bag repeat the bag.
pub fn draw_goblin(t: &Cf02Tuning, seed: u64, restarts: u32, goblin: u32) -> Draw {
    let bag = shuffled_bag(t, seed, restarts);
    let temperament = bag[goblin as usize % bag.len()];
    let pick = |draw: u32, lo: i32, hi: i32| ranged(lo, hi, mix(seed, restarts, goblin, draw));
    Draw {
        temperament,
        nerve_offset: pick(
            1,
            i32::from(t.q58_nerve_offset_min),
            i32::from(t.q58_nerve_offset_max),
        ) as i8,
        panic_len: pick(
            2,
            i32::from(t.q58_panic_len_min),
            i32::from(t.q58_panic_len_max),
        ) as u8,
        approach_delay: pick(
            3,
            i32::from(t.q58_approach_delay_min),
            i32::from(t.q58_approach_delay_max),
        ) as u8,
    }
}

// ---- Q60: witnessed events ------------------------------------------------

/// A world position in px.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

/// An event that moves a goblin's nerve (Q60). Goblins are addressed by their
/// index in the room.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoraleEvent {
    /// A goblin was defeated (the defeated goblin does not witness itself).
    GoblinDefeated {
        goblin: usize,
        pos: Pos,
        force: Force,
        cause: Option<SpectacularCause>,
    },
    /// A `WallSlam`, `BodyContact` or shore landing without a defeat.
    Spectacle { pos: Pos },
    /// This goblin was knocked down.
    OwnKnockdown { goblin: usize },
    /// A hero was hurt.
    HeroHurt { pos: Pos },
    /// This goblin hit a hero (on top of the witnessed [`Self::HeroHurt`]).
    OwnHitOnHero { goblin: usize },
}

fn force_rank(force: Force) -> u8 {
    match force {
        Force::Routine => 0,
        Force::Heavy => 1,
        Force::Catastrophic => 2,
    }
}

/// Q60: a defeat is spectacular at force Heavy or more, or for a spectacular
/// cause (shore, explosion, decap) at any force.
pub fn is_spectacular(t: &Cf02Tuning, force: Force, cause: Option<SpectacularCause>) -> bool {
    force_rank(force) >= force_rank(t.q60_spectacular_min_force)
        || cause.is_some_and(|c| t.q60_spectacular_causes.contains(&c))
}

/// Q60: whether a goblin at `goblin` witnesses an event at `event` (no line of
/// sight): within the horizontal and vertical radii, both inclusive.
pub fn witnesses(t: &Cf02Tuning, goblin: Pos, event: Pos) -> bool {
    (goblin.x - event.x).abs() <= f32::from(t.q60_witness_dx_px)
        && (goblin.y - event.y).abs() <= f32::from(t.q60_witness_dy_px)
}

/// The net nerve change a goblin gets from one tick's events.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventDelta {
    /// Sum of the witnessed deltas of this tick (several events add up).
    pub delta: i16,
    /// The x of the first witnessed event that lowered nerve, in the order the
    /// events were given: where a panic flees from.
    pub anchor_x: Option<f32>,
}

/// Q60: the deltas goblin `me` (at `at`) gets from this tick's events. The
/// sum is clamped once, by [`Morale::step`], after all events.
pub fn event_delta(t: &Cf02Tuning, me: usize, at: Pos, events: &[MoraleEvent]) -> EventDelta {
    let mut delta = 0i16;
    let mut anchor_x = None;
    for event in events {
        let (d, x) = match *event {
            MoraleEvent::GoblinDefeated {
                goblin,
                pos,
                force,
                cause,
            } if goblin != me && witnesses(t, at, pos) => {
                let d = if is_spectacular(t, force, cause) {
                    t.q60_delta_spectacular_defeat
                } else {
                    t.q60_delta_routine_defeat
                };
                (d, Some(pos.x))
            }
            MoraleEvent::Spectacle { pos } if witnesses(t, at, pos) => {
                (t.q60_delta_spectacle_no_defeat, Some(pos.x))
            }
            MoraleEvent::OwnKnockdown { goblin } if goblin == me => {
                (t.q60_delta_own_knockdown, Some(at.x))
            }
            MoraleEvent::HeroHurt { pos } if witnesses(t, at, pos) => {
                (t.q60_delta_hero_hurt, Some(pos.x))
            }
            MoraleEvent::OwnHitOnHero { goblin } if goblin == me => {
                (t.q60_delta_own_hit_on_hero, None)
            }
            _ => continue,
        };
        let d = i16::from(d);
        delta += d;
        if d < 0 && anchor_x.is_none() {
            anchor_x = x;
        }
    }
    EventDelta { delta, anchor_x }
}

// ---- Q57, Q61..Q65, Q68: nerve and reactions --------------------------------

/// The reaction a goblin is in. Reactions start only from [`Reaction::Neutral`]
/// (Q61). `left` counts the observed ticks still to go, including the current
/// one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reaction {
    Neutral,
    /// Q61: stand still.
    Hesitate {
        left: u8,
    },
    /// Q61: flee at `q61_panic_speed_px_s` in `flee_dir`, no attacks.
    Panic {
        left: u8,
        flee_dir: i8,
        blocked: u8,
    },
    /// Q61: faster walk and attack cooldown; the tell is unchanged.
    Enraged {
        left: u8,
    },
    /// Q65: a Show-off's taunt.
    Taunt {
        left: u8,
    },
    /// Q64: a Coward's hop back; `elapsed` observed ticks so far.
    BackOff {
        elapsed: u8,
    },
}

/// A nerve reaction that [`Morale::step`] started on this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Started {
    Hesitate,
    Panic,
    Enraged,
}

impl Started {
    /// Q61: a panic also aborts a tell (not a swing already past its tell, Q68).
    pub fn aborts_tell(self) -> bool {
        self == Self::Panic
    }

    /// Q68: hesitate and panic return the attack token on their first tick.
    pub fn returns_token(self) -> bool {
        matches!(self, Self::Hesitate | Self::Panic)
    }
}

impl Reaction {
    /// Q68: hesitate, panic, taunt (and the back-off) return the token on their
    /// first tick; an enraged goblin keeps fighting.
    pub fn returns_token_on_start(&self) -> bool {
        matches!(
            self,
            Self::Hesitate { .. } | Self::Panic { .. } | Self::Taunt { .. }
        )
    }

    /// Q68: a goblin in a non-fighting reaction cannot take a token. Enraged
    /// goblins attack faster (Q61), so they can.
    pub fn can_take_token(&self) -> bool {
        matches!(self, Self::Neutral | Self::Enraged { .. })
    }
}

/// Everything [`Morale::step`] needs besides the nerve itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepInput {
    /// This tick's [`event_delta`].
    pub events: EventDelta,
    pub goblin_x: f32,
    /// The nearest hero's x (tie-break of the flee direction).
    pub nearest_hero_x: Option<f32>,
    /// Facing sign, -1 or 1.
    pub facing: i8,
    /// The last living goblin in the room (Q62).
    pub last_goblin: bool,
    /// A reaction may start now. False while the goblin is in a swing past its
    /// tell (Q68): the nerve still changes, but no reaction starts or is
    /// consumed.
    pub can_react: bool,
    /// A panicking goblin is blocked by a solid or the layout edge (Q63).
    pub blocked: bool,
}

/// What a step did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StepOutcome {
    pub started: Option<Started>,
    /// Q63: a cornered panic ended; nerve is now `q63_cornered_nerve` and the
    /// goblin fights.
    pub cornered: bool,
}

/// Q61: the flee direction (+1 right, -1 left): away from the event's x; on a
/// tie (or no event x) away from the nearest hero; on a further tie opposite to
/// the facing.
pub fn flee_direction(
    event_x: Option<f32>,
    goblin_x: f32,
    nearest_hero_x: Option<f32>,
    facing: i8,
) -> i8 {
    for from in [event_x, nearest_hero_x].into_iter().flatten() {
        let dx = goblin_x - from;
        if dx > 0.0 {
            return 1;
        }
        if dx < 0.0 {
            return -1;
        }
    }
    if facing >= 0 { -1 } else { 1 }
}

/// One goblin's morale: nerve, drift, reactions and cooldowns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Morale {
    temperament: Temperament,
    baseline: u8,
    nerve: u8,
    panic_len: u8,
    panic_used: bool,
    reaction: Reaction,
    drift_timer: u8,
    taunt_cooldown: u16,
    backoff_cooldown: u16,
}

impl Morale {
    /// Q57, Q58: the state a goblin gets when it (re)spawns for an attempt:
    /// baseline plus the draw's offset clamped to the nerve range, panic not yet
    /// used, no reaction, no cooldown.
    pub fn spawn(t: &Cf02Tuning, draw: &Draw) -> Self {
        let base = baseline(t, draw.temperament);
        Self {
            temperament: draw.temperament,
            baseline: base,
            nerve: clamp_nerve(t, i32::from(base) + i32::from(draw.nerve_offset)),
            panic_len: draw.panic_len,
            panic_used: false,
            reaction: Reaction::Neutral,
            drift_timer: 0,
            taunt_cooldown: 0,
            backoff_cooldown: 0,
        }
    }

    pub fn temperament(&self) -> Temperament {
        self.temperament
    }

    pub fn baseline(&self) -> u8 {
        self.baseline
    }

    pub fn nerve(&self) -> u8 {
        self.nerve
    }

    pub fn reaction(&self) -> Reaction {
        self.reaction
    }

    /// Q61: the goblin has already panicked in this attempt.
    pub fn panic_used(&self) -> bool {
        self.panic_used
    }

    /// Advance one tick, in this order: (a) the running reaction's timer (a
    /// cornered panic ends here), (b) the tick's events change the nerve (the sum
    /// is clamped once) and a reaction may start from the value after the events,
    /// (c) cooldowns and the drift step (drift never starts a reaction).
    pub fn step(&mut self, t: &Cf02Tuning, input: &StepInput) -> StepOutcome {
        let mut out = StepOutcome::default();
        self.advance_reaction(t, input.blocked, &mut out);

        let net = input.events.delta;
        if net != 0 {
            let before = self.nerve;
            self.nerve = clamp_nerve(t, i32::from(before) + i32::from(net));
            if input.can_react && matches!(self.reaction, Reaction::Neutral) {
                out.started = self.evaluate(t, before, net, input);
            }
        }

        self.taunt_cooldown = self.taunt_cooldown.saturating_sub(1);
        self.backoff_cooldown = self.backoff_cooldown.saturating_sub(1);
        self.drift(t);
        out
    }

    fn advance_reaction(&mut self, t: &Cf02Tuning, blocked: bool, out: &mut StepOutcome) {
        self.reaction = match self.reaction {
            Reaction::Neutral => Reaction::Neutral,
            Reaction::Hesitate { left } => countdown(left, |left| Reaction::Hesitate { left }),
            Reaction::Enraged { left } => countdown(left, |left| Reaction::Enraged { left }),
            Reaction::Taunt { left } => countdown(left, |left| Reaction::Taunt { left }),
            Reaction::BackOff { elapsed } => {
                if elapsed + 1 >= t.q64_backoff_ticks {
                    Reaction::Neutral
                } else {
                    Reaction::BackOff {
                        elapsed: elapsed + 1,
                    }
                }
            }
            Reaction::Panic {
                left,
                flee_dir,
                blocked: run,
            } => {
                let run = if blocked { run.saturating_add(1) } else { 0 };
                if run >= t.q63_cornered_ticks {
                    // Q63: the cornered rat bites.
                    self.nerve = clamp_nerve(t, i32::from(t.q63_cornered_nerve));
                    out.cornered = true;
                    Reaction::Neutral
                } else if left <= 1 {
                    Reaction::Neutral
                } else {
                    Reaction::Panic {
                        left: left - 1,
                        flee_dir,
                        blocked: run,
                    }
                }
            }
        };
    }

    /// Q61: only on a tick where events changed the nerve, against the value
    /// after them.
    fn evaluate(
        &mut self,
        t: &Cf02Tuning,
        before: u8,
        net: i16,
        input: &StepInput,
    ) -> Option<Started> {
        let after = self.nerve;
        if net < 0 {
            let panic_below = if input.last_goblin {
                t.q62_last_goblin_panic_threshold
            } else {
                t.q61_panic_threshold
            };
            if after < panic_below && !self.panic_used {
                self.panic_used = true;
                self.reaction = Reaction::Panic {
                    left: self.panic_len,
                    flee_dir: flee_direction(
                        input.events.anchor_x,
                        input.goblin_x,
                        input.nearest_hero_x,
                        input.facing,
                    ),
                    blocked: 0,
                };
                return Some(Started::Panic);
            }
            if after < t.q61_hesitate_threshold {
                self.reaction = Reaction::Hesitate {
                    left: t.q61_hesitate_ticks,
                };
                return Some(Started::Hesitate);
            }
        } else if before < t.q61_enrage_threshold
            && after >= t.q61_enrage_threshold
            && t.q59_enrage_temperaments.contains(&self.temperament)
        {
            self.reaction = Reaction::Enraged {
                left: t.q61_enrage_ticks,
            };
            return Some(Started::Enraged);
        }
        None
    }

    /// Q57: one point toward the baseline every `q57_drift_period_ticks`.
    fn drift(&mut self, t: &Cf02Tuning) {
        self.drift_timer += 1;
        if self.drift_timer < t.q57_drift_period_ticks {
            return;
        }
        self.drift_timer = 0;
        let step = t.q57_drift_step;
        self.nerve = if self.nerve < self.baseline {
            self.nerve.saturating_add(step).min(self.baseline)
        } else {
            self.nerve.saturating_sub(step).max(self.baseline)
        };
    }

    /// Q61: the enraged `ATTACK_COOLDOWN` (the tell is untouched), else `legacy`.
    pub fn attack_cooldown(&self, t: &Cf02Tuning, legacy: u8) -> u8 {
        if matches!(self.reaction, Reaction::Enraged { .. }) {
            t.q61_enrage_attack_cooldown
        } else {
            legacy
        }
    }

    /// Q61: the walk speed in px/s: the flee speed in a panic, the enraged speed,
    /// 0 while standing still (hesitate, taunt, back-off hops are moved by
    /// [`Self::backoff_step_px`]), else `legacy`.
    pub fn walk_speed_px_s(&self, t: &Cf02Tuning, legacy: f32) -> f32 {
        match self.reaction {
            Reaction::Panic { .. } => f32::from(t.q61_panic_speed_px_s),
            Reaction::Enraged { .. } => f32::from(t.q61_enrage_walk_speed_px_s),
            Reaction::Hesitate { .. } | Reaction::Taunt { .. } | Reaction::BackOff { .. } => 0.0,
            Reaction::Neutral => legacy,
        }
    }

    /// Q64: start the Coward's hop back. Needs a Coward in Neutral without a
    /// token, off cooldown, `in_front_px` (its distance in front of a hero, `None`
    /// when it is not in front) within the trigger distance, and a hero move
    /// whose startup is at least `q64_backoff_min_startup`. The cooldown counts
    /// from the start.
    pub fn try_backoff(
        &mut self,
        t: &Cf02Tuning,
        has_token: bool,
        in_front_px: Option<f32>,
        hero_move_startup: u8,
    ) -> bool {
        let near =
            in_front_px.is_some_and(|d| (0.0..=f32::from(t.q64_backoff_trigger_px)).contains(&d));
        if self.temperament != Temperament::Coward
            || !matches!(self.reaction, Reaction::Neutral)
            || has_token
            || self.backoff_cooldown > 0
            || !near
            || hero_move_startup < t.q64_backoff_min_startup
        {
            return false;
        }
        self.reaction = Reaction::BackOff { elapsed: 0 };
        self.backoff_cooldown = u16::from(t.q64_backoff_cooldown);
        true
    }

    /// Q64: the hop-back distance to apply on the current observed tick:
    /// `q64_backoff_distance_px` spread over `q64_backoff_ticks` in integers, so
    /// the steps add up to exactly the distance. 0 outside a back-off.
    pub fn backoff_step_px(&self, t: &Cf02Tuning) -> u8 {
        let Reaction::BackOff { elapsed } = self.reaction else {
            return 0;
        };
        let dist = u32::from(t.q64_backoff_distance_px);
        let ticks = u32::from(t.q64_backoff_ticks);
        let done = |k: u32| dist * k / ticks;
        let k = u32::from(elapsed);
        (done(k + 1) - done(k)) as u8
    }

    /// Q65: whether a Show-off's taunt trigger holds: a hero within
    /// `q65_taunt_hero_hurt_px` was hurt, or a hero's move whiffed within
    /// `q65_taunt_whiff_px` of the goblin (distances in px, `None` = no such
    /// event this tick).
    pub fn taunt_triggered(
        t: &Cf02Tuning,
        hero_hurt_px: Option<f32>,
        whiff_px: Option<f32>,
    ) -> bool {
        hero_hurt_px.is_some_and(|d| d <= f32::from(t.q65_taunt_hero_hurt_px))
            || whiff_px.is_some_and(|d| d <= f32::from(t.q65_taunt_whiff_px))
    }

    /// Q65: start the Show-off's taunt. Needs a Show-off in Neutral without a
    /// token, off cooldown and `triggered` (see [`Self::taunt_triggered`]). The
    /// taunt does not change the nerve. The cooldown counts from the start.
    pub fn try_taunt(&mut self, t: &Cf02Tuning, has_token: bool, triggered: bool) -> bool {
        if self.temperament != Temperament::ShowOff
            || !matches!(self.reaction, Reaction::Neutral)
            || has_token
            || self.taunt_cooldown > 0
            || !triggered
        {
            return false;
        }
        self.reaction = Reaction::Taunt {
            left: t.q65_taunt_ticks,
        };
        self.taunt_cooldown = t.q65_taunt_cooldown;
        true
    }

    /// Q65: a hero hit lands. During a taunt a non-launching hit ends the taunt
    /// and staggers the goblin for the returned ticks (instead of the move's
    /// stagger); a launch or knockdown ends it and keeps its own effect
    /// (`None`). Outside a taunt: `None`, nothing changes.
    pub fn on_hero_hit(&mut self, t: &Cf02Tuning, launches_or_knocks_down: bool) -> Option<u8> {
        if !matches!(self.reaction, Reaction::Taunt { .. }) {
            return None;
        }
        self.reaction = Reaction::Neutral;
        (!launches_or_knocks_down).then_some(t.q65_taunt_punish_stagger)
    }
}

fn clamp_nerve(t: &Cf02Tuning, value: i32) -> u8 {
    value.clamp(0, i32::from(t.q57_nerve_max)) as u8
}

fn countdown(left: u8, make: impl Fn(u8) -> Reaction) -> Reaction {
    if left <= 1 {
        Reaction::Neutral
    } else {
        make(left - 1)
    }
}

// ---- Q66: pincer token ---------------------------------------------------------

/// A goblin that could take an attack token.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenCandidate {
    pub index: usize,
    /// Which side of the target hero it stands on: -1 or 1.
    pub side: i8,
    /// Its distance to the target hero in px.
    pub distance_px: f32,
}

/// Q66: the goblin the second token should go to preferentially: the nearest
/// eligible candidate on the **other** side of the target from the first
/// holder, within `trigger_px + q66_pincer_extra_px`. `None` means the legacy
/// nearest-first rule applies (the switch is off, or nobody qualifies). Ties on
/// distance go to the lower index. `eligible` candidates are the ones that may
/// take a token at all ([`Reaction::can_take_token`]).
pub fn pincer_preferred(
    t: &Cf02Tuning,
    holder_side: i8,
    trigger_px: f32,
    eligible: &[TokenCandidate],
) -> Option<usize> {
    if !t.on(t.q66_pincer_token) || holder_side == 0 {
        return None;
    }
    let reach = trigger_px + f32::from(t.q66_pincer_extra_px);
    eligible
        .iter()
        .filter(|c| c.side == -holder_side.signum() && c.distance_px <= reach)
        .min_by(|a, b| {
            a.distance_px
                .total_cmp(&b.distance_px)
                .then(a.index.cmp(&b.index))
        })
        .map(|c| c.index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tuning() -> Cf02Tuning {
        let mut t = Cf02Tuning::pinned().expect("pinned tuning");
        t.cf02_rules = true;
        t
    }

    fn draw(temperament: Temperament, offset: i8) -> Draw {
        Draw {
            temperament,
            nerve_offset: offset,
            panic_len: 75,
            approach_delay: 10,
        }
    }

    fn input(delta: i16) -> StepInput {
        StepInput {
            events: EventDelta {
                delta,
                anchor_x: (delta < 0).then_some(100.0),
            },
            goblin_x: 200.0,
            nearest_hero_x: Some(50.0),
            facing: -1,
            last_goblin: false,
            can_react: true,
            blocked: false,
        }
    }

    fn quiet() -> StepInput {
        input(0)
    }

    fn at(x: f32) -> Pos {
        Pos { x, y: 0.0 }
    }

    // ---- Q58 ----------------------------------------------------------

    #[test]
    fn q58_splitmix_matches_the_reference_vector() {
        // The reference outputs of SplitMix64 seeded with 0 (the autoplay bots
        // pin the same vector).
        assert_eq!(sm(0), 0xe220_a839_7b1d_cdaf);
        assert_eq!(sm(0x9e37_79b9_7f4a_7c15), 0x6e78_9e6a_a1b9_65f4);
        assert_eq!(
            sm(0x9e37_79b9_7f4a_7c15u64.wrapping_mul(2)),
            0x06c4_5d18_8009_454f
        );
    }

    #[test]
    fn q58_mix_is_the_chained_formula_and_stateless() {
        let (s, r, g, d) = (0xdead_beef_u64, 3u32, 2u32, 1u32);
        assert_eq!(
            mix(s, r, g, d),
            sm(sm(sm(sm(s) ^ u64::from(r)) ^ u64::from(g)) ^ u64::from(d))
        );
        assert_eq!(mix(s, r, g, d), mix(s, r, g, d));
        // Every field matters.
        let base = mix(s, r, g, d);
        assert_ne!(base, mix(s + 1, r, g, d));
        assert_ne!(base, mix(s, r + 1, g, d));
        assert_ne!(base, mix(s, r, g + 1, d));
        assert_ne!(base, mix(s, r, g, d + 1));
    }

    #[test]
    fn q58_pinned_vectors_match_an_independent_implementation() {
        // Computed with a separate Python implementation of the Q58 formula.
        assert_eq!(mix(1, 2, 3, 4), 0xd55c_cd4a_eb3c_cafb);
        assert_eq!(mix(0, 0, 0, 0), 0x2130_748a_aac8_0268);
        use Temperament::{Brawler as B, Coward as C, ShowOff as S};
        let t = tuning();
        assert_eq!(shuffled_bag(&t, 0, 0), vec![B, C, B, S, C]);
        assert_eq!(shuffled_bag(&t, 0, 1), vec![S, B, B, C, C]);
        assert_eq!(shuffled_bag(&t, 7, 2), vec![B, S, C, C, B]);
    }

    #[test]
    fn q58_close_seeds_and_restarts_are_not_aliased() {
        let mut seen = std::collections::BTreeSet::new();
        for seed in 0..64u64 {
            for restarts in 0..16u32 {
                assert!(
                    seen.insert(mix(seed, restarts, 0, 0)),
                    "seed {seed} restarts {restarts} collides"
                );
            }
        }
        // Over many consecutive seeds the shuffle reaches every one of the 30
        // distinct permutations of [B, B, C, C, S]: nothing pins it to a few.
        let t = tuning();
        let perms: std::collections::BTreeSet<_> = (0..2048u64)
            .map(|s| format!("{:?}", shuffled_bag(&t, s, 0)))
            .collect();
        assert_eq!(perms.len(), 30);
        // Seeds s and s + 1 rarely agree (independent draws, 1/30 each).
        let same = (0..2048u64)
            .filter(|s| shuffled_bag(&t, *s, 0) == shuffled_bag(&t, s + 1, 0))
            .count();
        assert!(same < 2048 / 10, "{same} adjacent seeds share a bag");
    }

    #[test]
    fn q58_bag_shuffle_is_stable_for_one_seed_and_restart() {
        let t = tuning();
        let a = shuffled_bag(&t, 42, 0);
        assert_eq!(a, shuffled_bag(&t, 42, 0));
        let mut sorted = a.clone();
        sorted.sort_by_key(|x| format!("{x:?}"));
        let mut bag = t.q58_bag.clone();
        bag.sort_by_key(|x| format!("{x:?}"));
        assert_eq!(sorted, bag, "a shuffle keeps the bag's contents");
        // Draw by goblin is stable too.
        for g in 0..5 {
            assert_eq!(draw_goblin(&t, 42, 0, g), draw_goblin(&t, 42, 0, g));
        }
    }

    #[test]
    fn q58_a_restart_reshuffles() {
        let t = tuning();
        for seed in [0u64, 1, 7, 42, 0xfeed_f00d] {
            let bags: std::collections::BTreeSet<_> = (0..6u32)
                .map(|r| format!("{:?}", shuffled_bag(&t, seed, r)))
                .collect();
            assert!(
                bags.len() >= 3,
                "seed {seed}: {} distinct over 6 restarts",
                bags.len()
            );
        }
        // Restarts 0 and 1 differ for the default seed 0 (pinned, so the CF02-G
        // fixture `cf02_restart_reshuffles` can rely on it).
        assert_ne!(shuffled_bag(&t, 0, 0), shuffled_bag(&t, 0, 1));
        // The panic length, offset and delay change with restarts, too.
        let draws: std::collections::BTreeSet<_> = (0..8u32)
            .map(|r| {
                let d = draw_goblin(&t, 0, r, 0);
                (d.nerve_offset, d.panic_len, d.approach_delay)
            })
            .collect();
        assert!(draws.len() >= 4);
    }

    #[test]
    fn q58_the_fisher_yates_swap_order_is_exact() {
        // Re-derive draw 0 by hand for [B, B, C, C, S] and seed 7, restart 2.
        let t = tuning();
        let mut want = t.q58_bag.clone();
        for i in (1..5usize).rev() {
            let j = (mix(7, 2, i as u32, 0) % (i as u64 + 1)) as usize;
            want.swap(i, j);
        }
        assert_eq!(shuffled_bag(&t, 7, 2), want);
        // Goblin k takes slot k; a sixth goblin repeats the bag.
        for k in 0..5u32 {
            assert_eq!(draw_goblin(&t, 7, 2, k).temperament, want[k as usize]);
        }
        assert_eq!(draw_goblin(&t, 7, 2, 5).temperament, want[0]);
        assert_eq!(draw_goblin(&t, 7, 2, 7).temperament, want[2]);
    }

    #[test]
    fn q58_draws_stay_in_their_ranges_and_cover_them() {
        let t = tuning();
        let mut off = (i8::MAX, i8::MIN);
        let mut len = (u8::MAX, u8::MIN);
        let mut delay = (u8::MAX, u8::MIN);
        for seed in 0..400u64 {
            for g in 0..5 {
                let d = draw_goblin(&t, seed, 0, g);
                assert!((-5..=5).contains(&d.nerve_offset));
                assert!((60..=90).contains(&d.panic_len));
                assert!((0..=20).contains(&d.approach_delay));
                off = (off.0.min(d.nerve_offset), off.1.max(d.nerve_offset));
                len = (len.0.min(d.panic_len), len.1.max(d.panic_len));
                delay = (delay.0.min(d.approach_delay), delay.1.max(d.approach_delay));
            }
        }
        assert_eq!(off, (-5, 5));
        assert_eq!(len, (60, 90));
        assert_eq!(delay, (0, 20));
    }

    #[test]
    fn q57_q59_spawn_is_baseline_plus_offset_clamped_to_the_nerve_range() {
        let t = tuning();
        assert_eq!(
            Morale::spawn(&t, &draw(Temperament::Brawler, 0)).nerve(),
            70
        );
        assert_eq!(
            Morale::spawn(&t, &draw(Temperament::ShowOff, 5)).nerve(),
            65
        );
        assert_eq!(
            Morale::spawn(&t, &draw(Temperament::Coward, -5)).nerve(),
            40
        );
        assert_eq!(
            Morale::spawn(&t, &draw(Temperament::Coward, 0)).baseline(),
            45
        );
        let mut high = t.clone();
        high.q59_baseline_brawler = 100;
        assert_eq!(
            Morale::spawn(&high, &draw(Temperament::Brawler, 5)).nerve(),
            100
        );
        let mut low = t.clone();
        low.q59_baseline_coward = 2;
        assert_eq!(
            Morale::spawn(&low, &draw(Temperament::Coward, -5)).nerve(),
            0
        );
        let m = Morale::spawn(&t, &draw(Temperament::Brawler, 0));
        assert!(!m.panic_used());
        assert_eq!(m.reaction(), Reaction::Neutral);
    }

    // ---- Q60 ----------------------------------------------------------

    fn defeat(goblin: usize, x: f32, force: Force, cause: Option<SpectacularCause>) -> MoraleEvent {
        MoraleEvent::GoblinDefeated {
            goblin,
            pos: at(x),
            force,
            cause,
        }
    }

    #[test]
    fn q60_each_delta() {
        let t = tuning();
        let me = Pos { x: 200.0, y: 0.0 };
        let d = |events: &[MoraleEvent]| event_delta(&t, 0, me, events).delta;
        assert_eq!(d(&[defeat(1, 150.0, Force::Routine, None)]), -20);
        assert_eq!(d(&[defeat(1, 150.0, Force::Heavy, None)]), -35);
        assert_eq!(d(&[defeat(1, 150.0, Force::Catastrophic, None)]), -35);
        for cause in [
            SpectacularCause::Shore,
            SpectacularCause::Explosion,
            SpectacularCause::Decap,
        ] {
            assert_eq!(d(&[defeat(1, 150.0, Force::Routine, Some(cause))]), -35);
        }
        assert_eq!(d(&[MoraleEvent::Spectacle { pos: at(150.0) }]), -15);
        assert_eq!(d(&[MoraleEvent::OwnKnockdown { goblin: 0 }]), -10);
        assert_eq!(d(&[MoraleEvent::HeroHurt { pos: at(150.0) }]), 15);
        assert_eq!(d(&[MoraleEvent::OwnHitOnHero { goblin: 0 }]), 10);
    }

    #[test]
    fn q60_own_events_belong_to_one_goblin_and_a_goblin_does_not_witness_itself_die() {
        let t = tuning();
        let me = at(200.0);
        let d = |who: usize, events: &[MoraleEvent]| event_delta(&t, who, me, events).delta;
        assert_eq!(d(1, &[MoraleEvent::OwnKnockdown { goblin: 0 }]), 0);
        assert_eq!(d(1, &[MoraleEvent::OwnHitOnHero { goblin: 0 }]), 0);
        assert_eq!(d(0, &[defeat(0, 200.0, Force::Heavy, None)]), 0);
    }

    #[test]
    fn q60_the_witness_range_is_160_by_64_inclusive() {
        let t = tuning();
        let me = Pos { x: 300.0, y: 10.0 };
        let e = |x: f32, y: f32| MoraleEvent::HeroHurt { pos: Pos { x, y } };
        let d = |ev: MoraleEvent| event_delta(&t, 0, me, &[ev]).delta;
        assert_eq!(d(e(140.0, 10.0)), 15, "dx 160 is in");
        assert_eq!(d(e(139.0, 10.0)), 0, "dx 161 is out");
        assert_eq!(d(e(460.0, 10.0)), 15);
        assert_eq!(d(e(461.0, 10.0)), 0);
        assert_eq!(d(e(300.0, 74.0)), 15, "dy 64 is in");
        assert_eq!(d(e(300.0, 75.0)), 0, "dy 65 is out");
        assert_eq!(d(e(300.0, -54.0)), 15);
        assert_eq!(d(e(300.0, -55.0)), 0);
        // An unwitnessed defeat changes nothing.
        assert_eq!(
            event_delta(&t, 0, me, &[defeat(1, 900.0, Force::Heavy, None)]).delta,
            0
        );
    }

    #[test]
    fn q60_several_events_on_one_tick_add_up_and_the_anchor_is_the_first_scare() {
        let t = tuning();
        let me = at(200.0);
        let r = event_delta(
            &t,
            0,
            me,
            &[
                MoraleEvent::HeroHurt { pos: at(260.0) },
                MoraleEvent::OwnHitOnHero { goblin: 0 },
                defeat(1, 120.0, Force::Routine, None),
                MoraleEvent::Spectacle { pos: at(80.0) },
            ],
        );
        assert_eq!(r.delta, 15 + 10 - 20 - 15);
        assert_eq!(r.anchor_x, Some(120.0));
        let calm = event_delta(&t, 0, me, &[MoraleEvent::HeroHurt { pos: at(260.0) }]);
        assert_eq!(calm.anchor_x, None, "a hero hurt is not a scare");
        let own = event_delta(&t, 0, me, &[MoraleEvent::OwnKnockdown { goblin: 0 }]);
        assert_eq!(own.anchor_x, Some(200.0));
    }

    #[test]
    fn q60_spectacular_is_force_or_cause() {
        let t = tuning();
        assert!(!is_spectacular(&t, Force::Routine, None));
        assert!(is_spectacular(&t, Force::Heavy, None));
        assert!(is_spectacular(&t, Force::Catastrophic, None));
        assert!(is_spectacular(
            &t,
            Force::Routine,
            Some(SpectacularCause::Decap)
        ));
        let mut none = t.clone();
        none.q60_spectacular_causes.clear();
        assert!(!is_spectacular(
            &none,
            Force::Routine,
            Some(SpectacularCause::Decap)
        ));
    }

    // ---- Q57 / Q61 --------------------------------------------------------

    #[test]
    fn q57_drift_moves_one_point_toward_the_baseline_every_20_ticks() {
        let t = tuning();
        let mut up = Morale::spawn(&t, &draw(Temperament::Brawler, -5)); // 65 -> 70
        for tick in 1..=19 {
            up.step(&t, &quiet());
            assert_eq!(up.nerve(), 65, "tick {tick}");
        }
        up.step(&t, &quiet());
        assert_eq!(up.nerve(), 66);
        for _ in 0..(20 * 10) {
            up.step(&t, &quiet());
        }
        assert_eq!(up.nerve(), 70, "stops at the baseline");

        let mut down = Morale::spawn(&t, &draw(Temperament::Coward, 5)); // 50 -> 45
        for _ in 0..(20 * 3) {
            down.step(&t, &quiet());
        }
        assert_eq!(down.nerve(), 47);
    }

    #[test]
    fn q57_nerve_is_clamped_to_0_and_100() {
        let t = tuning();
        let mut m = Morale::spawn(&t, &draw(Temperament::Brawler, 5));
        m.step(&t, &input(90));
        assert_eq!(m.nerve(), 100);
        let mut m = Morale::spawn(&t, &draw(Temperament::Coward, -5));
        m.step(&t, &input(-90));
        assert_eq!(m.nerve(), 0);
        // One clamp after the whole sum, not after each event.
        let mut m = Morale::spawn(&t, &draw(Temperament::Brawler, 0));
        let sum = input(30 - 35 + 15);
        m.step(&t, &sum);
        assert_eq!(m.nerve(), 80);
    }

    #[test]
    fn q61_a_coward_at_baseline_never_hesitates_from_drift() {
        let t = tuning();
        for offset in [-5i8, 0, 5] {
            let mut m = Morale::spawn(&t, &draw(Temperament::Coward, offset));
            for tick in 0..5000 {
                let out = m.step(&t, &quiet());
                assert_eq!(out.started, None, "offset {offset} tick {tick}");
                assert_eq!(m.reaction(), Reaction::Neutral);
            }
            assert_eq!(m.nerve(), 45);
        }
    }

    #[test]
    fn q61_a_coward_starts_below_the_hesitate_threshold_and_still_has_to_be_scared() {
        // Baseline 45 < 50: the first witnessed event that changes the nerve
        // (even a hero hurt that raises it a little) is judged. A raise never
        // hesitates; a drop of any size below 50 does.
        let t = tuning();
        let mut m = Morale::spawn(&t, &draw(Temperament::Coward, 0));
        let out = m.step(&t, &input(15)); // 60
        assert_eq!(out.started, None);
        let out = m.step(&t, &input(-15)); // 45 < 50
        assert_eq!(out.started, Some(Started::Hesitate));
    }

    #[test]
    fn q61_hesitate_stands_still_for_20_ticks() {
        let t = tuning();
        let mut m = Morale::spawn(&t, &draw(Temperament::Brawler, 0)); // 70
        let out = m.step(&t, &input(-35)); // 35
        assert_eq!(out.started, Some(Started::Hesitate));
        assert!(out.started.unwrap().returns_token());
        assert!(!out.started.unwrap().aborts_tell());
        let mut observed = 1; // the starting tick
        assert_eq!(m.reaction(), Reaction::Hesitate { left: 20 });
        assert_eq!(m.walk_speed_px_s(&t, 62.0), 0.0);
        loop {
            m.step(&t, &quiet());
            if m.reaction() == Reaction::Neutral {
                break;
            }
            observed += 1;
            assert!(observed <= 20);
        }
        assert_eq!(observed, 20);
        assert_eq!(m.walk_speed_px_s(&t, 62.0), 62.0);
    }

    #[test]
    fn q61_panic_below_25_once_per_attempt_then_hesitate() {
        let t = tuning();
        let mut m = Morale::spawn(&t, &draw(Temperament::Coward, 0)); // 45
        let out = m.step(&t, &input(-25)); // 20 < 25
        assert_eq!(out.started, Some(Started::Panic));
        assert!(out.started.unwrap().aborts_tell());
        assert!(m.panic_used());
        let Reaction::Panic { left, flee_dir, .. } = m.reaction() else {
            panic!("expected a panic");
        };
        assert_eq!(left, 75);
        assert_eq!(flee_dir, 1, "away from the event at x 100, goblin at 200");
        assert_eq!(m.walk_speed_px_s(&t, 62.0), 78.0);
        // Let it run out.
        for _ in 0..75 {
            m.step(&t, &quiet());
        }
        assert_eq!(m.reaction(), Reaction::Neutral);
        // A second scare under 25 does not panic again: it hesitates (< 50).
        let out = m.step(&t, &input(-5));
        assert_eq!(out.started, Some(Started::Hesitate));
        assert!(m.panic_used());
    }

    #[test]
    fn q57_panic_is_available_again_after_a_restart() {
        let t = tuning();
        let (seed, goblin) = (7u64, 2u32);
        let mut first = Morale::spawn(&t, &draw_goblin(&t, seed, 0, goblin));
        assert!(!first.panic_used());
        assert_eq!(first.step(&t, &input(-100)).started, Some(Started::Panic));
        assert!(first.panic_used());
        // The same attempt cannot panic a second time.
        let mut again = first;
        assert_ne!(again.step(&t, &input(-100)).started, Some(Started::Panic));
        // A restart re-spawns from the next attempt's draws: a fresh state, and
        // the panic fires again.
        let mut second = Morale::spawn(&t, &draw_goblin(&t, seed, 1, goblin));
        assert!(!second.panic_used(), "restart resets panic_used");
        assert_eq!(second.step(&t, &input(-100)).started, Some(Started::Panic));
        assert!(second.panic_used());
    }

    #[test]
    fn q61_panic_lasts_exactly_the_drawn_length() {
        let t = tuning();
        for len in [60u8, 75, 90] {
            let mut d = draw(Temperament::Coward, 0);
            d.panic_len = len;
            let mut m = Morale::spawn(&t, &d);
            assert_eq!(m.step(&t, &input(-30)).started, Some(Started::Panic));
            let mut observed = 1;
            loop {
                m.step(&t, &quiet());
                if m.reaction() == Reaction::Neutral {
                    break;
                }
                observed += 1;
            }
            assert_eq!(observed, u32::from(len));
        }
    }

    #[test]
    fn q61_reactions_start_only_on_an_event_tick_from_neutral() {
        let t = tuning();
        // Not from an unchanged nerve, however low.
        let mut m = Morale::spawn(&t, &draw(Temperament::Coward, -5)); // 40
        assert_eq!(m.step(&t, &quiet()).started, None);
        // An event that nets zero does not evaluate.
        assert_eq!(m.step(&t, &input(0)).started, None);
        // Not while another reaction runs.
        let mut b = Morale::spawn(&t, &draw(Temperament::Brawler, 0));
        assert_eq!(b.step(&t, &input(-35)).started, Some(Started::Hesitate));
        assert_eq!(b.step(&t, &input(-30)).started, None, "already hesitating");
        assert_eq!(b.nerve(), 5, "the nerve still changes");
        assert!(!b.panic_used(), "and no panic was consumed");
        // Not while it cannot react (a swing past its tell): the nerve changes,
        // nothing starts or is spent.
        let mut s = Morale::spawn(&t, &draw(Temperament::Coward, 0));
        let mut swing = input(-30);
        swing.can_react = false;
        assert_eq!(s.step(&t, &swing).started, None);
        assert_eq!(s.nerve(), 15);
        assert!(!s.panic_used());
    }

    #[test]
    fn q61_a_raise_never_panics_or_hesitates() {
        let t = tuning();
        let mut m = Morale::spawn(&t, &draw(Temperament::Coward, -5));
        assert_eq!(m.step(&t, &input(10)).started, None);
        assert_eq!(m.reaction(), Reaction::Neutral);
    }

    #[test]
    fn q61_a_brawler_reaching_85_is_enraged_and_a_show_off_is_not() {
        let t = tuning();
        // Brawler 70 + 15 = 85.
        let mut b = Morale::spawn(&t, &draw(Temperament::Brawler, 0));
        let out = b.step(&t, &input(15));
        assert_eq!(out.started, Some(Started::Enraged));
        assert!(!out.started.unwrap().returns_token());
        assert_eq!(b.reaction(), Reaction::Enraged { left: 120 });
        assert_eq!(b.walk_speed_px_s(&t, 62.0), 81.0);
        assert_eq!(b.attack_cooldown(&t, 40), 24);
        // Show-off 60 + 15 + 10 = 85: no enrage.
        let mut s = Morale::spawn(&t, &draw(Temperament::ShowOff, 0));
        let out = s.step(&t, &input(25));
        assert_eq!(s.nerve(), 85);
        assert_eq!(out.started, None);
        assert_eq!(s.reaction(), Reaction::Neutral);
        assert_eq!(s.attack_cooldown(&t, 40), 40);
        // A coward either.
        let mut c = Morale::spawn(&t, &draw(Temperament::Coward, 5));
        assert_eq!(c.step(&t, &input(40)).started, None);
    }

    #[test]
    fn q61_enrage_lasts_120_ticks_and_needs_a_rise_across_85() {
        let t = tuning();
        let mut m = Morale::spawn(&t, &draw(Temperament::Brawler, 5)); // 75
        assert_eq!(m.step(&t, &input(15)).started, Some(Started::Enraged)); // 90
        let mut observed = 1;
        loop {
            m.step(&t, &quiet());
            if m.reaction() == Reaction::Neutral {
                break;
            }
            observed += 1;
        }
        assert_eq!(observed, 120);
        assert_eq!(m.attack_cooldown(&t, 40), 40);
        // A raise that stays at or above 85 is not a crossing: no new enrage.
        let mut high = Morale::spawn(&t, &draw(Temperament::Brawler, 5));
        high.step(&t, &input(20)); // 95, enraged
        for _ in 0..121 {
            high.step(&t, &quiet());
        }
        assert_eq!(high.reaction(), Reaction::Neutral);
        assert!(high.nerve() >= 85, "drift is slow: still {}", high.nerve());
        assert_eq!(high.step(&t, &input(2)).started, None);
        // Drift never starts an enrage, even for a brawler whose baseline is 85
        // or more.
        let mut hi = t.clone();
        hi.q59_baseline_brawler = 90;
        let mut m = Morale::spawn(&hi, &draw(Temperament::Brawler, 0));
        for _ in 0..2000 {
            assert_eq!(m.step(&hi, &quiet()).started, None);
        }
    }

    #[test]
    fn q62_the_last_goblin_panics_below_50() {
        let t = tuning();
        let mut last = input(-15);
        last.last_goblin = true;
        // 60 - 15 = 45: not below 25, but the last goblin panics below 50.
        let mut s = Morale::spawn(&t, &draw(Temperament::ShowOff, 0));
        assert_eq!(s.step(&t, &last).started, Some(Started::Panic));
        // The same drop for a goblin that is not the last one hesitates.
        let mut o = Morale::spawn(&t, &draw(Temperament::ShowOff, 0));
        assert_eq!(o.step(&t, &input(-15)).started, Some(Started::Hesitate));
        // Still once per attempt: after a panic the last goblin hesitates.
        for _ in 0..75 {
            s.step(&t, &quiet());
        }
        assert_eq!(s.step(&t, &last).started, Some(Started::Hesitate));
    }

    #[test]
    fn q61_the_flee_direction_ties() {
        // Away from the event.
        assert_eq!(flee_direction(Some(100.0), 200.0, Some(300.0), 1), 1);
        assert_eq!(flee_direction(Some(300.0), 200.0, Some(100.0), 1), -1);
        // Event on the goblin: away from the nearest hero.
        assert_eq!(flee_direction(Some(200.0), 200.0, Some(100.0), 1), 1);
        assert_eq!(flee_direction(None, 200.0, Some(300.0), 1), -1);
        // Everything ties: opposite to its facing.
        assert_eq!(flee_direction(Some(200.0), 200.0, Some(200.0), 1), -1);
        assert_eq!(flee_direction(None, 200.0, None, -1), 1);
    }

    #[test]
    fn q63_a_panic_blocked_for_20_ticks_ends_with_nerve_40() {
        let t = tuning();
        let mut m = Morale::spawn(&t, &draw(Temperament::Coward, 0));
        assert_eq!(m.step(&t, &input(-30)).started, Some(Started::Panic));
        let mut blocked = quiet();
        blocked.blocked = true;
        // 19 blocked ticks: still panicking.
        for _ in 0..19 {
            let out = m.step(&t, &blocked);
            assert!(!out.cornered);
            assert!(matches!(m.reaction(), Reaction::Panic { .. }));
        }
        let out = m.step(&t, &blocked);
        assert!(out.cornered);
        assert_eq!(m.reaction(), Reaction::Neutral);
        assert_eq!(m.nerve(), 40);
        // The panic stays used.
        assert!(m.panic_used());
    }

    #[test]
    fn q63_a_blocked_run_must_be_consecutive() {
        let t = tuning();
        let mut m = Morale::spawn(&t, &draw(Temperament::Coward, 0));
        m.step(&t, &input(-30));
        let mut blocked = quiet();
        blocked.blocked = true;
        for round in 0..3 {
            for _ in 0..19 {
                assert!(!m.step(&t, &blocked).cornered, "round {round}");
            }
            m.step(&t, &quiet()); // one free tick resets the run
        }
        assert!(matches!(m.reaction(), Reaction::Panic { .. }));
    }

    // ---- Q64, Q65, Q66, Q68 ---------------------------------------------------

    #[test]
    fn q64_only_a_coward_without_a_token_backs_off_from_a_slow_move_in_front() {
        let t = tuning();
        let go = |temp: Temperament, token: bool, front: Option<f32>, startup: u8| {
            let mut m = Morale::spawn(&t, &draw(temp, 0));
            (m.try_backoff(&t, token, front, startup), m)
        };
        assert!(go(Temperament::Coward, false, Some(48.0), 9).0);
        assert!(go(Temperament::Coward, false, Some(0.0), 13).0);
        assert!(!go(Temperament::Coward, false, Some(49.0), 9).0, "too far");
        assert!(
            !go(Temperament::Coward, false, Some(30.0), 3).0,
            "the jab (startup 3) still hits"
        );
        assert!(!go(Temperament::Coward, false, Some(30.0), 8).0);
        assert!(
            !go(Temperament::Coward, true, Some(30.0), 9).0,
            "holds a token"
        );
        assert!(!go(Temperament::Coward, false, None, 9).0, "not in front");
        assert!(!go(Temperament::Brawler, false, Some(30.0), 9).0);
        assert!(!go(Temperament::ShowOff, false, Some(30.0), 9).0);
    }

    #[test]
    fn q64_the_hop_is_28_px_over_10_ticks_with_a_120_tick_cooldown() {
        let t = tuning();
        let mut m = Morale::spawn(&t, &draw(Temperament::Coward, 0));
        assert_eq!(m.backoff_step_px(&t), 0);
        assert!(m.try_backoff(&t, false, Some(20.0), 9));
        let mut total = 0u32;
        let mut ticks = 0;
        while matches!(m.reaction(), Reaction::BackOff { .. }) {
            total += u32::from(m.backoff_step_px(&t));
            ticks += 1;
            m.step(&t, &quiet());
        }
        assert_eq!((ticks, total), (10, 28));
        // Cooldown counts from the start: the hop ended after 10 ticks, so 110
        // more ticks remain.
        for tick in 0..109 {
            m.step(&t, &quiet());
            assert!(!m.try_backoff(&t, false, Some(20.0), 9), "tick {tick}");
        }
        m.step(&t, &quiet());
        assert!(m.try_backoff(&t, false, Some(20.0), 9));
    }

    #[test]
    fn q65_a_show_off_taunts_on_a_hurt_hero_or_a_whiff_and_the_taunt_can_be_punished() {
        let t = tuning();
        assert!(Morale::taunt_triggered(&t, Some(160.0), None));
        assert!(!Morale::taunt_triggered(&t, Some(161.0), None));
        assert!(Morale::taunt_triggered(&t, None, Some(64.0)));
        assert!(!Morale::taunt_triggered(&t, None, Some(65.0)));
        assert!(!Morale::taunt_triggered(&t, None, None));

        let mut s = Morale::spawn(&t, &draw(Temperament::ShowOff, 0));
        assert!(
            !s.try_taunt(&t, true, true),
            "a token holder does not taunt"
        );
        assert!(!s.try_taunt(&t, false, false));
        assert!(s.try_taunt(&t, false, true));
        assert_eq!(s.reaction(), Reaction::Taunt { left: 36 });
        assert_eq!(s.nerve(), 60, "the taunt does not change nerve");
        // A non-launching hit staggers it 20 ticks and ends the taunt.
        assert_eq!(s.on_hero_hit(&t, false), Some(20));
        assert_eq!(s.reaction(), Reaction::Neutral);
        // Cooldown 240 from the start.
        for _ in 0..239 {
            s.step(&t, &quiet());
            assert!(!s.try_taunt(&t, false, true));
        }
        s.step(&t, &quiet());
        assert!(s.try_taunt(&t, false, true));
        // A launch ends it and keeps its own effect.
        assert_eq!(s.on_hero_hit(&t, true), None);
        assert_eq!(s.reaction(), Reaction::Neutral);
        // Outside a taunt nothing happens.
        assert_eq!(s.on_hero_hit(&t, false), None);
        // Other temperaments never taunt.
        for temp in [Temperament::Brawler, Temperament::Coward] {
            let mut m = Morale::spawn(&t, &draw(temp, 0));
            assert!(!m.try_taunt(&t, false, true));
        }
    }

    #[test]
    fn q65_a_taunt_runs_36_ticks() {
        let t = tuning();
        let mut s = Morale::spawn(&t, &draw(Temperament::ShowOff, 0));
        assert!(s.try_taunt(&t, false, true));
        let mut observed = 1;
        loop {
            s.step(&t, &quiet());
            if s.reaction() == Reaction::Neutral {
                break;
            }
            observed += 1;
        }
        assert_eq!(observed, 36);
    }

    #[test]
    fn q68_reactions_return_the_token_and_block_taking_one() {
        for r in [
            Reaction::Hesitate { left: 20 },
            Reaction::Panic {
                left: 70,
                flee_dir: 1,
                blocked: 0,
            },
            Reaction::Taunt { left: 36 },
        ] {
            assert!(r.returns_token_on_start(), "{r:?}");
            assert!(!r.can_take_token(), "{r:?}");
        }
        assert!(!Reaction::BackOff { elapsed: 0 }.can_take_token());
        assert!(Reaction::Neutral.can_take_token());
        assert!(!Reaction::Neutral.returns_token_on_start());
        // An enraged goblin keeps fighting (it attacks faster, Q61).
        assert!(Reaction::Enraged { left: 100 }.can_take_token());
        assert!(!Reaction::Enraged { left: 100 }.returns_token_on_start());
    }

    #[test]
    fn q66_the_second_token_prefers_the_other_side_within_trigger_plus_30() {
        let t = tuning();
        let c = |index: usize, side: i8, distance_px: f32| TokenCandidate {
            index,
            side,
            distance_px,
        };
        // Holder on the right (+1): prefer the nearest on the left.
        let cs = [c(0, 1, 20.0), c(1, -1, 70.0), c(2, -1, 55.0)];
        assert_eq!(pincer_preferred(&t, 1, 48.0, &cs), Some(2));
        // The reach is trigger + 30.
        let far = [c(0, -1, 78.0)];
        assert_eq!(pincer_preferred(&t, 1, 48.0, &far), Some(0));
        let too_far = [c(0, -1, 78.5)];
        assert_eq!(pincer_preferred(&t, 1, 48.0, &too_far), None);
        // Nobody on the other side: the legacy rule decides.
        assert_eq!(pincer_preferred(&t, 1, 48.0, &[c(0, 1, 10.0)]), None);
        // Distance ties go to the lower index.
        assert_eq!(
            pincer_preferred(&t, -1, 48.0, &[c(3, 1, 40.0), c(1, 1, 40.0)]),
            Some(1)
        );
        // Switch off or master switch off: no preference.
        let mut s = t.clone();
        s.q66_pincer_token = false;
        assert_eq!(pincer_preferred(&s, 1, 48.0, &cs), None);
        let mut m = t.clone();
        m.cf02_rules = false;
        assert_eq!(pincer_preferred(&m, 1, 48.0, &cs), None);
        assert_eq!(pincer_preferred(&t, 0, 48.0, &cs), None);
    }

    #[test]
    fn q61_a_whole_attempt_replays_identically() {
        // The same seed, restart and events give the same states, tick by tick.
        let t = tuning();
        let run = || {
            let mut trace = Vec::new();
            for g in 0..3u32 {
                let d = draw_goblin(&t, 9, 1, g);
                let mut m = Morale::spawn(&t, &d);
                for tick in 0..400u32 {
                    let delta = match tick {
                        30 => -35,
                        150 => 25,
                        250 => -20,
                        _ => 0,
                    };
                    m.step(&t, &input(delta));
                    trace.push((m.nerve(), m.reaction()));
                }
            }
            trace
        };
        assert_eq!(run(), run());
    }
}
