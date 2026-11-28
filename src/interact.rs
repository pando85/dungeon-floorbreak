//! GUILD-VERB-01: the Interact verb (R1/RB, keyboard E) as a pure, Bevy-free fixed-tick core.
//!
//! A press edge of `RawInput::interact` starts a hero's verb on a valid target of the room's
//! [`InteractSite`]. Rooms without a site (every room but `act1_guild`) never build the runtime,
//! so Interact does nothing there and nothing of it reaches a hash. The verb tables follow
//! `docs/02-art/interact-anim-01-spec.md` section 3 (frame windows, contact ticks) and are pinned
//! to the export manifests by a test.
//!
//! * The verb is chosen by the target's **height class** and the hero ([`verb_for`]): Carl
//!   `Hand`, `Floor` or `Push`, Donut `Paw` or `Nudge` (she has no verb above her low class).
//! * Tick 0 is the tick the press is accepted. The effect lands on the **contact tick**, never on
//!   the press. A hit, a defeat or a hurt state cancels the verb, and a cancel before the contact
//!   tick means the effect never happens. A hit-stop holds the clock.
//! * During a verb the hero's input is stripped (velocity zero, no jump, no attack): the feet stay
//!   planted, as the art assumes. The hero is snapped to the verb's stand-off distance on the
//!   first tick (GUILDV-AD3), at most [`REACH_AHEAD`] px.
//! * The companion AI never interacts: a hero driven by a bot has its Interact press ignored
//!   (GUILDV-AD4).
//!
//! Hash policy: [`Interact::mix_into`] mixes nothing until the first accepted press, so every
//! run that never presses Interact on a target keeps its hash (the goldens).

use crate::input::RawInput;
use crate::party::HeroId;

/// A verb of the interact set: the five tags of the two interact atlases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    /// Carl, hand-height prop (switch, lever, handle, a strongbox latch).
    Hand,
    /// Carl, floor pickup or low prop.
    Floor,
    /// Carl, waist-height shove.
    Push,
    /// Donut, a paw pat on a low switch or small item.
    Paw,
    /// Donut, a muzzle nudge on a small item or floor plate.
    Nudge,
}

/// Where a prop's contact band sits (spec section 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeightClass {
    Floor,
    Low,
    Waist,
    Hand,
}

/// The verb `hero` uses on a prop of `class`, `None` when the hero has none (a cat cannot use a
/// hand-height or waist-height prop, spec sections 1 and 4).
pub fn verb_for(hero: HeroId, class: HeightClass) -> Option<Verb> {
    match (hero, class) {
        (HeroId::Carl, HeightClass::Floor | HeightClass::Low) => Some(Verb::Floor),
        (HeroId::Carl, HeightClass::Waist) => Some(Verb::Push),
        (HeroId::Carl, HeightClass::Hand) => Some(Verb::Hand),
        (HeroId::Donut, HeightClass::Floor) => Some(Verb::Nudge),
        (HeroId::Donut, HeightClass::Low) => Some(Verb::Paw),
        (HeroId::Donut, HeightClass::Waist | HeightClass::Hand) => None,
    }
}

impl Verb {
    pub const ALL: [Verb; 5] = [Verb::Hand, Verb::Floor, Verb::Push, Verb::Paw, Verb::Nudge];

    /// The atlas tag and the manifest key.
    pub const fn tag(self) -> &'static str {
        match self {
            Verb::Hand => "interact_hand",
            Verb::Floor => "interact_floor",
            Verb::Push => "interact_push",
            Verb::Paw => "interact_paw",
            Verb::Nudge => "interact_nudge",
        }
    }

    /// The first tick of each of the verb's four frames (spec section 3).
    const fn starts(self) -> [u32; 4] {
        match self {
            Verb::Hand => [0, 5, 13, 23],
            Verb::Floor => [0, 8, 18, 28],
            Verb::Push => [0, 6, 14, 26],
            Verb::Paw => [0, 6, 12, 20],
            Verb::Nudge => [0, 8, 16, 26],
        }
    }

    /// Ticks of the whole verb.
    pub const fn total_ticks(self) -> u32 {
        match self {
            Verb::Hand => 31,
            Verb::Floor => 36,
            Verb::Push => 34,
            Verb::Paw => 26,
            Verb::Nudge => 34,
        }
    }

    /// The tick the effect lands (the first tick of the contact frame).
    pub const fn contact_tick(self) -> u32 {
        self.starts()[1]
    }

    /// Pivot to the prop's near face, px (spec section 4).
    pub const fn stand_off(self) -> i32 {
        match self {
            Verb::Hand => 17,
            Verb::Floor => 15,
            Verb::Push => 20,
            Verb::Paw => 18,
            Verb::Nudge => 21,
        }
    }

    /// The frame index inside the verb's tag at verb clock `clock`, `None` past the verb.
    pub fn frame_at(self, clock: u32) -> Option<usize> {
        if clock >= self.total_ticks() {
            return None;
        }
        self.starts().iter().rposition(|start| clock >= *start)
    }
}

/// How far behind the stand-off distance a press is still accepted (spec section 7: the
/// tolerance is 3 px for Carl).
pub const REACH_BEHIND: i32 = 3;
/// How far in front of the stand-off distance a press is still accepted; the hero is snapped to
/// the exact stand-off (GUILDV-AD3), so the window is wider than the art's tolerance.
pub const REACH_AHEAD: i32 = 15;
/// A hero's pivot within this many px of the guide's column can talk to him (GUILDV-AD5).
pub const GUIDE_REACH: i32 = 64;
/// Ticks the guide's `talk` key shows after an accepted press (GUILDV-AD5).
pub const TALK_TICKS: u64 = 150;

/// The interactable things of one room (layout px, x only: the rooms are single-floor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteractSite {
    /// The grant strongbox's left and right faces, `None` without one.
    pub box_faces: Option<(i32, i32)>,
    /// The guide's column, `None` without one.
    pub guide_x: Option<i32>,
}

/// What a press targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    GrantBox,
    Guide,
}

/// What [`Interact::pick`] found.
enum Pick {
    GrantBox { verb: Verb, snap_x: f32 },
    Guide,
}

/// A verb in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Active {
    pub verb: Verb,
    pub target: Target,
    /// The verb clock: 0 on the press tick.
    pub clock: u32,
}

/// What the combat state tells the core about one hero this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeroCtx {
    pub x: f32,
    pub facing: i8,
    pub grounded: bool,
    /// The hero can start something (alive, not hurt, not in hit-stop, no melee running).
    pub can_start: bool,
    /// Hurt or defeated: an active verb is cancelled.
    pub interrupted: bool,
    /// In hit-stop: the verb clock holds.
    pub frozen: bool,
    /// The hero is driven by a bot (the companion AI never interacts).
    pub bot: bool,
    /// The hero's party slot exists (Donut only while the party is live).
    pub present: bool,
}

/// The result of one tick for the combat state to apply.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Outcome {
    /// The input each hero is allowed to use this tick (a hero in a verb gets none).
    pub locked: [bool; 2],
    /// Move a hero's pivot to this x (the stand-off snap).
    pub snap_x: [Option<f32>; 2],
    /// Turn a hero to this facing.
    pub face: [Option<i8>; 2],
}

/// The runtime of a room with an interact site.
#[derive(Debug, Clone, PartialEq)]
pub struct Interact {
    site: InteractSite,
    /// The previous tick's raw Interact button per hero (the press edge).
    prev: [bool; 2],
    active: [Option<Active>; 2],
    box_open: bool,
    /// The guide talks while the sim tick is below this (0: never).
    talk_until: u64,
    /// Accepted presses so far; mixed into the hash once non-zero.
    presses: u32,
}

impl Interact {
    pub fn new(site: InteractSite) -> Self {
        Self {
            site,
            prev: [false; 2],
            active: [None; 2],
            box_open: false,
            talk_until: 0,
            presses: 0,
        }
    }

    pub fn site(&self) -> &InteractSite {
        &self.site
    }

    /// The strongbox was opened by a hero's verb.
    pub fn box_open(&self) -> bool {
        self.box_open
    }

    /// The hero's verb in progress.
    pub fn active(&self, hero: HeroId) -> Option<Active> {
        self.active[hero.index()]
    }

    /// The hero's verb and its frame index inside the verb's tag, for the presenter.
    pub fn pose(&self, hero: HeroId) -> Option<(Verb, usize)> {
        let active = self.active(hero)?;
        active.verb.frame_at(active.clock).map(|f| (active.verb, f))
    }

    /// The guide's `talk` key is held at sim tick `tick`.
    pub fn guide_talking(&self, tick: u64) -> bool {
        tick < self.talk_until
    }

    /// Cancel every verb (a respawn). The opened box and the talk stay.
    pub fn cancel(&mut self) {
        self.active = [None; 2];
        self.prev = [false; 2];
    }

    /// Back to the room's first state (a fixture restart).
    pub fn reset(&mut self) {
        *self = Self::new(self.site);
    }

    /// One fixed tick. `raw` are the heroes' inputs after the party and the companion layers;
    /// `tick` is the sim tick being run.
    pub fn step(&mut self, tick: u64, ctx: [HeroCtx; 2], raw: [RawInput; 2]) -> Outcome {
        let mut out = Outcome::default();
        for hero in HeroId::ALL {
            let h = hero.index();
            let c = ctx[h];
            let pressed = raw[h].interact && !self.prev[h];
            self.prev[h] = raw[h].interact;
            if !c.present {
                self.active[h] = None;
                continue;
            }
            // A verb in progress: cancel, hold or advance. After tick N + k of a press on
            // tick N the clock reads k, so the effect lands on the tick the clock reaches the
            // contact tick, the one the presenter shows the contact frame from.
            if let Some(mut a) = self.active[h] {
                if c.interrupted {
                    self.active[h] = None;
                } else {
                    if !c.frozen {
                        a.clock += 1;
                        if a.clock == a.verb.contact_tick() {
                            self.land(a.target);
                        }
                    }
                    self.active[h] = (a.clock < a.verb.total_ticks()).then_some(a);
                    out.locked[h] = self.active[h].is_some();
                }
                continue;
            }
            if !pressed || c.bot || !c.can_start || !c.grounded {
                continue;
            }
            let facing = if raw[h].move_x > 0.0 {
                1
            } else if raw[h].move_x < 0.0 {
                -1
            } else {
                c.facing
            };
            match self.pick(hero, c.x, facing) {
                Some(Pick::GrantBox { verb, snap_x }) => {
                    self.presses += 1;
                    self.active[h] = Some(Active {
                        verb,
                        target: Target::GrantBox,
                        clock: 0,
                    });
                    out.locked[h] = true;
                    out.snap_x[h] = Some(snap_x);
                    out.face[h] = Some(facing);
                }
                Some(Pick::Guide) => {
                    self.presses += 1;
                    self.talk_until = tick + TALK_TICKS;
                    if let Some(g) = self.site.guide_x {
                        out.face[h] = Some(if (g as f32) >= c.x { 1 } else { -1 });
                    }
                }
                None => {}
            }
        }
        out
    }

    /// The target of a press by `hero` at `x` facing `facing`: the strongbox first (while it is
    /// closed, free and the hero has a verb for it, in its reach window), then the guide.
    fn pick(&self, hero: HeroId, x: f32, facing: i8) -> Option<Pick> {
        if let Some((left, right)) = self.site.box_faces
            && !self.box_open
            && !self
                .active
                .iter()
                .flatten()
                .any(|a| a.target == Target::GrantBox)
            && let Some(verb) = verb_for(hero, BOX_CLASS)
        {
            let so = verb.stand_off();
            let (near, dist) = if facing > 0 {
                (left, left as f32 - x)
            } else {
                (right, x - right as f32)
            };
            if dist >= (so - REACH_BEHIND) as f32 && dist <= (so + REACH_AHEAD) as f32 {
                let snap_x = if facing > 0 { near - so } else { near + so } as f32;
                return Some(Pick::GrantBox { verb, snap_x });
            }
        }
        let guide = self.site.guide_x?;
        ((x - guide as f32).abs() <= GUIDE_REACH as f32).then_some(Pick::Guide)
    }

    fn land(&mut self, target: Target) {
        if target == Target::GrantBox {
            self.box_open = true;
        }
    }

    /// Mix the state into a hash, nothing until the first accepted press.
    pub fn mix_into(&self, put: &mut impl FnMut(u64)) {
        if self.presses == 0 {
            return;
        }
        put(0x494e_5441_4354);
        put(u64::from(self.presses));
        put(u64::from(self.box_open));
        put(self.talk_until);
        for a in self.active {
            put(a.map_or(0, |a| {
                1 + a.verb as u64 * 1000 + u64::from(a.clock) * 10 + a.target as u64
            }));
        }
    }
}

/// The class of the grant strongbox (24 x 28 px, its latch at 24 to 28 px: the hand band).
pub const BOX_CLASS: HeightClass = HeightClass::Hand;

#[cfg(test)]
mod tests;
