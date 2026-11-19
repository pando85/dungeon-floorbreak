//! AUDIENCE-01 part B: the Bevy-free live-viewer domain.
//!
//! Every hero owns a viewer count (Carl and Donut separately, keyed by the
//! **hero index**, never by a controller slot or P1/P2). The model is an
//! **observer**, exactly like CINE: it reads plain-data inputs mapped from
//! combat events and never feeds anything back, and nothing here is mixed
//! into `COMBAT_STATE` or the `COMBAT_EVENT` logs. While the feature is
//! locked (the v1 default; there is no in-game unlock yet) no step runs and no
//! state exists to digest, so the legacy replay goldens are untouched.
//!
//! Time is a domain-owned step counter: the caller invokes [`AudienceState::step`]
//! once per fixed tick with that tick's inputs. Event ticks are attempt-local
//! (a `Restart` resets them) and are never used here.
//!
//! Every tuning number lives in [`TUNING`]. They are **hypotheses** from the
//! AUDIENCE-01 review (`docs/02-art/individual-viewers-hud-v1.md`), not
//! balance; the owner reviews game feel after merge.

use std::collections::{BTreeMap, BTreeSet};

use crate::combat::{CombatEvent, DefeatCause, ForceTier};
use crate::party::HeroId;

// ------------------------------------------------------------------ tuning --

/// All audience tuning in one table. **Every value is a hypothesis** (AUD-Q2:
/// final award and decay balance waits for a playtest).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudienceTuning {
    /// The count both heroes start at on unlock, and the decay floor.
    pub baseline: u64,
    /// Routine defeat award (tiers replace, never stack).
    pub award_routine: u64,
    /// Heavy defeat award.
    pub award_heavy: u64,
    /// Catastrophic defeat award.
    pub award_catastrophic: u64,
    /// Environmental creativity bonus: an environmental defeat whose
    /// instigator is the credited hero themselves.
    pub award_environmental: u64,
    /// Assist share, in percent of the actor's award (integer truncation).
    pub assist_percent: u64,
    /// Chain award is `min(chain_cap, chain_unit * (score - chain_score_floor))`.
    pub chain_unit: u64,
    /// Chain feat score that earns nothing; each point above earns one unit.
    pub chain_score_floor: u8,
    /// Chain award cap.
    pub chain_cap: u64,
    /// Smallest authored anticipation award.
    pub anticipation_min: u64,
    /// Largest authored anticipation award.
    pub anticipation_max: u64,
    /// Inactive steps before decay starts (8 s at 60 Hz).
    pub decay_delay_steps: u32,
    /// Decay happens once per this many steps after the delay (1 s).
    pub decay_period_steps: u32,
    /// Decay of the excess over the baseline, in permille per period (2.3%).
    pub decay_permille: u64,
    /// Spike: a single-step award `delta >= max(spike_floor, before * spike_percent / 100)`.
    pub spike_floor: u64,
    /// Spike relative threshold, percent of the count before the award.
    pub spike_percent: u64,
    /// Fall: `reference - shown >= max(fall_floor, reference * fall_percent / 100)`.
    pub fall_floor: u64,
    /// Fall relative threshold, percent of the reference.
    pub fall_percent: u64,
    /// Steps a trend arrow stays up after a spike or a fall cue.
    pub trend_steps: u8,
    /// Steps before another fall cue may fire.
    pub fall_cooldown_steps: u16,
    /// Displayed count refreshes at most this often (a spike refreshes at once).
    pub refresh_steps: u8,
    /// A displayed count-up takes at most this many steps.
    pub count_up_steps: u8,
}

/// The single tuning table. Hypotheses, not balance.
pub const TUNING: AudienceTuning = AudienceTuning {
    baseline: 250,
    award_routine: 120,
    award_heavy: 500,
    award_catastrophic: 1_800,
    award_environmental: 900,
    assist_percent: 30,
    chain_unit: 250,
    chain_score_floor: 4,
    chain_cap: 1_000,
    anticipation_min: 4_000,
    anticipation_max: 15_000,
    decay_delay_steps: 480,
    decay_period_steps: 60,
    decay_permille: 23,
    spike_floor: 300,
    spike_percent: 15,
    fall_floor: 250,
    fall_percent: 10,
    trend_steps: 48,
    fall_cooldown_steps: 600,
    refresh_steps: 12,
    count_up_steps: 30,
};

// ----------------------------------------------------------------- credit --

/// The one credit mapping: a hero **index** (Carl 0, Donut 1) to a [`HeroId`].
/// Anything else earns no award. `Control` and `PadSlots` never appear here.
pub const fn hero_from_index(index: u32) -> Option<HeroId> {
    match index {
        0 => Some(HeroId::Carl),
        1 => Some(HeroId::Donut),
        _ => None,
    }
}

/// The hero an `EnemyDefeated` credits (`DefeatCause`'s `player` fields and
/// `instigator` hold **hero indices**, see the CINE-01 review F1).
///
/// The match is exhaustive on purpose: a new cause must decide its credit.
pub const fn credited_hero(cause: DefeatCause, instigator: Option<u32>) -> Option<HeroId> {
    let index = match cause {
        DefeatCause::Melee => instigator,
        DefeatCause::ShoreBlock { player } | DefeatCause::ShoreBlockEdge { player } => Some(player),
        DefeatCause::Explosion { player, .. } => player,
    };
    match index {
        Some(i) => hero_from_index(i),
        None => None,
    }
}

// ------------------------------------------------------------------ input --

/// A CINE-01 feat as plain data. Map a `cine::CinematicMoment` into it:
/// `credited` (a hero index), `target` and `score`. Audience never detects
/// chains itself and CINE never reads audience data (one-way dependency).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainFeat {
    /// The credited hero index (Carl 0, Donut 1).
    pub credited: u32,
    /// The target id the moment is anchored on (the dedupe key).
    pub target: u32,
    /// The CINE feat score.
    pub score: u8,
}

/// What can make an authored anticipation cue fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnticipationTrigger {
    /// Any hero's reported x position reaches `x` (scene world units).
    HeroCrossesX { x: i32 },
}

/// An authored anticipation cue (encounter data, validated on load). It names
/// the heroes it targets and never discloses the threat in the UI. Floor 01
/// authors none; only the QA fixture does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnticipationCue {
    pub id: u32,
    pub trigger: AnticipationTrigger,
    /// The targeted heroes (one or both).
    pub heroes: Vec<HeroId>,
    /// The award each targeted hero gets, inside the tuning range.
    pub award: u64,
}

/// Why a cue table was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CueError {
    DuplicateId(u32),
    NoHeroes(u32),
    DuplicateHero(u32),
    AwardOutOfRange(u32),
}

/// Validate a cue table: unique ids, at least one distinct hero each, and an
/// award inside `[anticipation_min, anticipation_max]`.
pub fn validate_cues(cues: &[AnticipationCue]) -> Result<(), CueError> {
    let mut ids = BTreeSet::new();
    for cue in cues {
        if !ids.insert(cue.id) {
            return Err(CueError::DuplicateId(cue.id));
        }
        if cue.heroes.is_empty() {
            return Err(CueError::NoHeroes(cue.id));
        }
        let distinct: BTreeSet<usize> = cue.heroes.iter().map(|h| h.index()).collect();
        if distinct.len() != cue.heroes.len() {
            return Err(CueError::DuplicateHero(cue.id));
        }
        if cue.award < TUNING.anticipation_min || cue.award > TUNING.anticipation_max {
            return Err(CueError::AwardOutOfRange(cue.id));
        }
    }
    Ok(())
}

/// One plain-data input of a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudienceInput {
    /// Map of `CombatEvent::EnemyDefeated`.
    Defeat {
        target: u32,
        cause: DefeatCause,
        force: ForceTier,
        instigator: Option<u32>,
    },
    /// Map of a CINE-01 `CinematicMoment`.
    Chain(ChainFeat),
    /// A read-only hero position, for authored anticipation triggers.
    HeroX { hero: HeroId, x: i32 },
    /// Map of `CombatEvent::Restart`: restore the attempt-start snapshot.
    Restart,
    /// Map of `CombatEvent::Respawn`: the demo respawn keeps counts and the
    /// dedupe set, and never costs viewers.
    Respawn,
}

impl AudienceInput {
    /// Map a combat event. Every other event (`HeroHurt`, `HeroDefeated`,
    /// tells, `GoblinContact`, ...) awards nothing: a hurt hero would create a
    /// farming incentive and a tell is on-screen danger, not anticipation.
    pub fn from_combat(event: &CombatEvent) -> Option<Self> {
        match *event {
            CombatEvent::EnemyDefeated {
                target,
                cause,
                force,
                instigator,
                ..
            } => Some(Self::Defeat {
                target,
                cause,
                force,
                instigator,
            }),
            CombatEvent::Restart { .. } => Some(Self::Restart),
            CombatEvent::Respawn { .. } => Some(Self::Respawn),
            _ => None,
        }
    }
}

// ------------------------------------------------------------- formatting --

/// Longest displayed count text (`999M+`, `99.9K`).
pub const MAX_TEXT_LEN: usize = 5;

/// Format a count for display (the model stays an untruncated `u64`):
/// `0..999` full, `1.0K..99.9K` one decimal (truncated), `100K..999K`,
/// `1.0M..9.9M`, `10M..999M`, and `999M+` from a billion. ASCII, no locale.
pub fn format_count(n: u64) -> String {
    match n {
        0..=999 => n.to_string(),
        1_000..=99_999 => format!("{}.{}K", n / 1_000, (n % 1_000) / 100),
        100_000..=999_999 => format!("{}K", n / 1_000),
        1_000_000..=9_999_999 => format!("{}.{}M", n / 1_000_000, (n % 1_000_000) / 100_000),
        10_000_000..=999_999_999 => format!("{}M", n / 1_000_000),
        _ => "999M+".to_string(),
    }
}

/// Ink width in pixels of a formatted count in the original monospace 5x7 set
/// (advance 6 per glyph, `.` advance 2, last glyph without its gap). Unknown
/// glyphs count as 6.
pub fn text_ink_width(text: &str) -> u32 {
    let advance: u32 = text.chars().map(|c| if c == '.' { 2 } else { 6 }).sum();
    advance.saturating_sub(1)
}

/// Where the hero name ink starts inside the plate, x.
pub const NAME_ORIGIN_X: u32 = 40;
/// The last plate-relative x the name ink may reach (F2).
pub const NAME_INK_LAST_X: u32 = 88;
/// The widest name ink that still ends at or before [`NAME_INK_LAST_X`].
pub const NAME_MAX_INK_WIDTH: u32 = NAME_INK_LAST_X - NAME_ORIGIN_X + 1;

/// The label a hero plate falls back to when a replaceable fan-layer name
/// would run into the eye icon.
pub const fn fallback_name(hero: HeroId) -> &'static str {
    match hero {
        HeroId::Carl => "HERO",
        HeroId::Donut => "ALLY",
    }
}

/// `name` when its ink (`ink_width` px from the name origin) respects the
/// bound, otherwise the hero's fallback label.
pub fn bounded_name(name: &str, ink_width: u32, hero: HeroId) -> &str {
    if ink_width <= NAME_MAX_INK_WIDTH {
        name
    } else {
        fallback_name(hero)
    }
}

// ------------------------------------------------------------------- view --

/// The trend glyph state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    Flat,
    Up,
    Down,
}

/// What the HUD reads for one hero (part C maps it into `PlateView`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudienceView {
    /// The count at the last display refresh (what reduced motion draws).
    pub shown: u64,
    /// The count-up interpolation of the displayed number.
    pub animated: u64,
    /// The highest count so far this attempt lineage.
    pub peak: u64,
    pub trend: Trend,
    /// Steps the trend arrow still has.
    pub trend_steps_left: u8,
}

impl AudienceView {
    /// The number to draw: the snapped count under reduced motion, the
    /// count-up value otherwise.
    pub const fn display_count(&self, reduced_motion: bool) -> u64 {
        if reduced_motion {
            self.shown
        } else {
            self.animated
        }
    }
}

// ------------------------------------------------------------------ state --

#[derive(Debug, Clone, PartialEq, Eq)]
struct HeroAudience {
    current: u64,
    peak: u64,
    /// Decay milli-remainder (`acc = excess * permille + rem`).
    decay_rem: u64,
    inactive_steps: u32,
    // Presentation, derived from `current`, never read by scoring.
    shown: u64,
    anim_from: u64,
    anim_elapsed: u8,
    since_refresh: u8,
    trend: Trend,
    trend_left: u8,
    fall_ref: u64,
    fall_cooldown: u16,
}

impl HeroAudience {
    fn at(count: u64, peak: u64, decay_rem: u64) -> Self {
        Self {
            current: count,
            peak,
            decay_rem,
            inactive_steps: 0,
            shown: count,
            anim_from: count,
            anim_elapsed: TUNING.count_up_steps,
            since_refresh: 0,
            trend: Trend::Flat,
            trend_left: 0,
            fall_ref: count,
            fall_cooldown: 0,
        }
    }

    fn animated(&self) -> u64 {
        let total = u64::from(TUNING.count_up_steps);
        let elapsed = u64::from(self.anim_elapsed);
        if elapsed >= total {
            return self.shown;
        }
        let from = i128::from(self.anim_from);
        let to = i128::from(self.shown);
        (from + (to - from) * i128::from(elapsed) / i128::from(total)) as u64
    }

    fn refresh(&mut self) {
        self.anim_from = self.animated();
        self.anim_elapsed = 0;
        self.shown = self.current;
        self.since_refresh = 0;
    }
}

/// The attempt-start snapshot a `Restart` restores.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    heroes: [(u64, u64, u64); 2],
    defeats: BTreeSet<u32>,
    chains: BTreeMap<(usize, u32), u64>,
    fired_cues: BTreeSet<u32>,
}

/// The per-hero audience model. Locked until [`Self::unlock`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudienceState {
    unlocked: bool,
    /// Monotonic domain step counter; never restored, never an event tick.
    step: u64,
    heroes: [HeroAudience; 2],
    /// Defeated targets already awarded this attempt (tiers never stack).
    defeats: BTreeSet<u32>,
    /// Chain bonus already paid per `(hero, target)`, so a CINE upgrade pays
    /// only the difference.
    chains: BTreeMap<(usize, u32), u64>,
    cues: Vec<AnticipationCue>,
    fired_cues: BTreeSet<u32>,
    snapshot: Snapshot,
}

impl Default for AudienceState {
    fn default() -> Self {
        Self::locked()
    }
}

impl AudienceState {
    /// The locked state: no counts, no view, steps and inputs are ignored.
    pub fn locked() -> Self {
        let base = HeroAudience::at(0, 0, 0);
        Self {
            unlocked: false,
            step: 0,
            heroes: [base.clone(), base],
            defeats: BTreeSet::new(),
            chains: BTreeMap::new(),
            cues: Vec::new(),
            fired_cues: BTreeSet::new(),
            snapshot: Snapshot {
                heroes: [(0, 0, 0); 2],
                defeats: BTreeSet::new(),
                chains: BTreeMap::new(),
                fired_cues: BTreeSet::new(),
            },
        }
    }

    pub const fn is_unlocked(&self) -> bool {
        self.unlocked
    }

    /// The one explicit `locked -> unlocked` story flag (`broadcast_live`):
    /// both heroes at the baseline and the attempt-start snapshot taken. A
    /// second call is a no-op. v1 has no in-game trigger; preview and QA only.
    pub fn unlock(&mut self) {
        if self.unlocked {
            return;
        }
        self.unlocked = true;
        let base = TUNING.baseline;
        self.heroes = [
            HeroAudience::at(base, base, 0),
            HeroAudience::at(base, base, 0),
        ];
        self.begin_attempt();
    }

    /// Install the authored cue table (validated; replaces any earlier one).
    pub fn set_cues(&mut self, cues: Vec<AnticipationCue>) -> Result<(), CueError> {
        validate_cues(&cues)?;
        self.cues = cues;
        Ok(())
    }

    /// Take a fresh attempt-start snapshot of the current state (a new attempt
    /// that is not a `Restart`, e.g. a later floor). No-op while locked.
    pub fn begin_attempt(&mut self) {
        if !self.unlocked {
            return;
        }
        self.snapshot = Snapshot {
            heroes: [0, 1].map(|i| {
                let h = &self.heroes[i];
                (h.current, h.peak, h.decay_rem)
            }),
            defeats: self.defeats.clone(),
            chains: self.chains.clone(),
            fired_cues: self.fired_cues.clone(),
        };
    }

    /// The count of one hero, `None` while locked.
    pub fn count(&self, hero: HeroId) -> Option<u64> {
        self.unlocked.then(|| self.heroes[hero.index()].current)
    }

    /// The HUD view of one hero, `None` while locked.
    pub fn view(&self, hero: HeroId) -> Option<AudienceView> {
        if !self.unlocked {
            return None;
        }
        let h = &self.heroes[hero.index()];
        Some(AudienceView {
            shown: h.shown,
            animated: h.animated(),
            peak: h.peak,
            trend: h.trend,
            trend_steps_left: h.trend_left,
        })
    }

    /// The steps run since unlock.
    pub const fn steps(&self) -> u64 {
        self.step
    }

    /// Advance one fixed tick with that tick's inputs. A locked state ignores
    /// everything and does not count the step.
    pub fn step(&mut self, inputs: &[AudienceInput]) {
        if !self.unlocked {
            return;
        }
        self.step += 1;
        let mut before = [self.heroes[0].current, self.heroes[1].current];
        let mut delta = [0u64; 2];
        for input in inputs {
            match *input {
                AudienceInput::Restart => {
                    self.restore();
                    before = [self.heroes[0].current, self.heroes[1].current];
                    delta = [0; 2];
                }
                AudienceInput::Respawn => {}
                AudienceInput::Defeat {
                    target,
                    cause,
                    force,
                    instigator,
                } => self.defeat(target, cause, force, instigator, &mut delta),
                AudienceInput::Chain(feat) => self.chain(feat, &mut delta),
                AudienceInput::HeroX { x, .. } => self.cues_at(x, &mut delta),
            }
        }
        for i in 0..2 {
            self.settle(i, before[i], delta[i]);
        }
    }

    fn award(&mut self, hero: HeroId, amount: u64, delta: &mut [u64; 2]) {
        if amount == 0 {
            return;
        }
        let i = hero.index();
        delta[i] = delta[i].saturating_add(amount);
    }

    fn defeat(
        &mut self,
        target: u32,
        cause: DefeatCause,
        force: ForceTier,
        instigator: Option<u32>,
        delta: &mut [u64; 2],
    ) {
        let Some(actor) = credited_hero(cause, instigator) else {
            return;
        };
        if !self.defeats.insert(target) {
            return;
        }
        let mut actor_award = match force {
            ForceTier::Routine => TUNING.award_routine,
            ForceTier::Heavy => TUNING.award_heavy,
            ForceTier::Catastrophic => TUNING.award_catastrophic,
        };
        let environmental = !matches!(cause, DefeatCause::Melee);
        let helper = instigator.and_then(hero_from_index);
        if environmental && helper == Some(actor) {
            actor_award = actor_award.saturating_add(TUNING.award_environmental);
        }
        self.award(actor, actor_award, delta);
        if environmental && let Some(h) = helper.filter(|h| *h != actor) {
            self.award(
                h,
                actor_award.saturating_mul(TUNING.assist_percent) / 100,
                delta,
            );
        }
    }

    fn chain(&mut self, feat: ChainFeat, delta: &mut [u64; 2]) {
        let Some(hero) = hero_from_index(feat.credited) else {
            return;
        };
        let units = u64::from(feat.score.saturating_sub(TUNING.chain_score_floor));
        let bonus = (units * TUNING.chain_unit).min(TUNING.chain_cap);
        let paid = self.chains.entry((hero.index(), feat.target)).or_insert(0);
        let owed = bonus.saturating_sub(*paid);
        *paid = (*paid).max(bonus);
        self.award(hero, owed, delta);
    }

    fn cues_at(&mut self, x: i32, delta: &mut [u64; 2]) {
        let mut due: Vec<(u32, Vec<HeroId>, u64)> = Vec::new();
        for cue in &self.cues {
            let AnticipationTrigger::HeroCrossesX { x: at } = cue.trigger;
            if x >= at && !self.fired_cues.contains(&cue.id) {
                due.push((cue.id, cue.heroes.clone(), cue.award));
            }
        }
        for (id, heroes, amount) in due {
            self.fired_cues.insert(id);
            for hero in heroes {
                self.award(hero, amount, delta);
            }
        }
    }

    /// Restore the attempt-start snapshot (counts, peak, decay remainder,
    /// dedupe sets, fired cues). Inactivity timers and trend cues restart.
    fn restore(&mut self) {
        let snap = self.snapshot.clone();
        for (i, &(count, peak, rem)) in snap.heroes.iter().enumerate() {
            self.heroes[i] = HeroAudience::at(count, peak, rem);
        }
        self.defeats = snap.defeats;
        self.chains = snap.chains;
        self.fired_cues = snap.fired_cues;
    }

    /// Per-hero end of step: apply the awards, decay, then presentation.
    fn settle(&mut self, i: usize, before: u64, delta: u64) {
        let h = &mut self.heroes[i];
        let mut spike = false;
        if delta > 0 {
            h.current = h.current.saturating_add(delta);
            h.peak = h.peak.max(h.current);
            h.inactive_steps = 0;
            spike = delta
                >= TUNING
                    .spike_floor
                    .max(before.saturating_mul(TUNING.spike_percent) / 100);
        } else {
            h.inactive_steps = h.inactive_steps.saturating_add(1);
            let over = h.inactive_steps.saturating_sub(TUNING.decay_delay_steps);
            if h.inactive_steps >= TUNING.decay_delay_steps
                && over.is_multiple_of(TUNING.decay_period_steps)
            {
                let excess = h.current.saturating_sub(TUNING.baseline);
                let acc = excess
                    .saturating_mul(TUNING.decay_permille)
                    .saturating_add(h.decay_rem);
                h.decay_rem = acc % 1_000;
                h.current = h.current.saturating_sub(acc / 1_000).max(TUNING.baseline);
            }
        }

        h.anim_elapsed = h.anim_elapsed.saturating_add(1).min(TUNING.count_up_steps);
        h.fall_cooldown = h.fall_cooldown.saturating_sub(1);
        h.trend_left = h.trend_left.saturating_sub(1);
        if h.trend_left == 0 {
            h.trend = Trend::Flat;
        }
        h.since_refresh = h.since_refresh.saturating_add(1);

        if spike {
            h.refresh();
            h.trend = Trend::Up;
            h.trend_left = TUNING.trend_steps;
            h.fall_ref = h.shown;
        } else if h.since_refresh >= TUNING.refresh_steps {
            if h.shown != h.current {
                h.refresh();
            }
            h.since_refresh = 0;
            h.fall_ref = h.fall_ref.max(h.shown);
            let threshold = TUNING
                .fall_floor
                .max(h.fall_ref.saturating_mul(TUNING.fall_percent) / 100);
            if h.fall_cooldown == 0 && h.trend != Trend::Up && h.fall_ref - h.shown >= threshold {
                h.trend = Trend::Down;
                h.trend_left = TUNING.trend_steps;
                h.fall_ref = h.shown;
                h.fall_cooldown = TUNING.fall_cooldown_steps;
            }
        }
    }

    /// A stable digest of the whole audience state, `None` while locked. Only
    /// audience fixtures print it (`AUDIENCE_STATE`); it is never mixed into
    /// the combat replay hash.
    pub fn digest(&self) -> Option<u64> {
        if !self.unlocked {
            return None;
        }
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        let mut mix = |v: u64| {
            for b in v.to_le_bytes() {
                hash ^= u64::from(b);
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
        };
        mix(self.step);
        for h in &self.heroes {
            mix(h.current);
            mix(h.peak);
            mix(h.decay_rem);
            mix(u64::from(h.inactive_steps));
            mix(h.shown);
            mix(h.animated());
            mix(h.trend as u64);
            mix(u64::from(h.trend_left));
            mix(h.fall_ref);
            mix(u64::from(h.fall_cooldown));
        }
        mix(self.defeats.len() as u64);
        for t in &self.defeats {
            mix(u64::from(*t));
        }
        for (&(hero, target), &paid) in &self.chains {
            mix(hero as u64);
            mix(u64::from(target));
            mix(paid);
        }
        for id in &self.fired_cues {
            mix(u64::from(*id));
        }
        Some(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unlocked() -> AudienceState {
        let mut s = AudienceState::locked();
        s.unlock();
        s
    }

    fn melee(target: u32, hero: u32, force: ForceTier) -> AudienceInput {
        AudienceInput::Defeat {
            target,
            cause: DefeatCause::Melee,
            force,
            instigator: Some(hero),
        }
    }

    fn idle(s: &mut AudienceState, steps: u32) {
        for _ in 0..steps {
            s.step(&[]);
        }
    }

    fn carl(s: &AudienceState) -> u64 {
        s.count(HeroId::Carl).unwrap()
    }

    fn donut(s: &AudienceState) -> u64 {
        s.count(HeroId::Donut).unwrap()
    }

    #[test]
    fn locked_has_no_state_and_ignores_everything() {
        let mut s = AudienceState::locked();
        s.step(&[melee(1, 0, ForceTier::Catastrophic)]);
        s.step(&[AudienceInput::Restart]);
        assert!(!s.is_unlocked());
        assert_eq!(s.count(HeroId::Carl), None);
        assert!(s.view(HeroId::Donut).is_none());
        assert_eq!(s.digest(), None);
        assert_eq!(s.steps(), 0);
        assert_eq!(s, AudienceState::locked());
    }

    #[test]
    fn unlock_starts_both_heroes_at_the_baseline_once() {
        let mut s = unlocked();
        assert_eq!((carl(&s), donut(&s)), (250, 250));
        s.step(&[melee(1, 0, ForceTier::Routine)]);
        s.unlock();
        assert_eq!(carl(&s), 370, "a second unlock is a no-op");
    }

    #[test]
    fn force_tiers_replace_and_do_not_stack() {
        let mut s = unlocked();
        s.step(&[melee(1, 0, ForceTier::Routine)]);
        assert_eq!(carl(&s), 250 + 120);
        s.step(&[melee(2, 0, ForceTier::Heavy)]);
        assert_eq!(carl(&s), 250 + 120 + 500);
        s.step(&[melee(3, 0, ForceTier::Catastrophic)]);
        assert_eq!(carl(&s), 250 + 120 + 500 + 1_800);
        assert_eq!(donut(&s), 250);
    }

    #[test]
    fn a_target_pays_only_once() {
        let mut s = unlocked();
        s.step(&[melee(7, 0, ForceTier::Heavy), melee(7, 0, ForceTier::Heavy)]);
        s.step(&[melee(7, 1, ForceTier::Heavy)]);
        assert_eq!(carl(&s), 750);
        assert_eq!(donut(&s), 250);
    }

    #[test]
    fn credit_follows_the_hero_index_for_every_cause() {
        let cases = [
            (DefeatCause::Melee, Some(1), Some(HeroId::Donut)),
            (DefeatCause::Melee, Some(0), Some(HeroId::Carl)),
            (DefeatCause::Melee, None, None),
            (DefeatCause::Melee, Some(2), None),
            (
                DefeatCause::ShoreBlock { player: 1 },
                None,
                Some(HeroId::Donut),
            ),
            (
                DefeatCause::ShoreBlockEdge { player: 0 },
                Some(1),
                Some(HeroId::Carl),
            ),
            (DefeatCause::ShoreBlock { player: 9 }, None, None),
            (
                DefeatCause::Explosion {
                    charge: 0,
                    player: Some(1),
                    point_blank: true,
                },
                None,
                Some(HeroId::Donut),
            ),
            (
                DefeatCause::Explosion {
                    charge: 0,
                    player: None,
                    point_blank: false,
                },
                Some(0),
                None,
            ),
        ];
        for (cause, instigator, want) in cases {
            assert_eq!(credited_hero(cause, instigator), want, "{cause:?}");
        }
    }

    #[test]
    fn environmental_creativity_needs_the_actor_to_be_the_instigator() {
        let mut s = unlocked();
        // Carl kicked the goblin under the block he then broke himself.
        s.step(&[AudienceInput::Defeat {
            target: 1,
            cause: DefeatCause::ShoreBlock { player: 0 },
            force: ForceTier::Catastrophic,
            instigator: Some(0),
        }]);
        assert_eq!(carl(&s), 250 + 1_800 + 900);
        // No instigator: the block alone, no bonus.
        s.step(&[AudienceInput::Defeat {
            target: 2,
            cause: DefeatCause::ShoreBlockEdge { player: 0 },
            force: ForceTier::Catastrophic,
            instigator: None,
        }]);
        assert_eq!(carl(&s), 250 + 1_800 + 900 + 1_800);
        // A melee defeat never earns the environmental bonus.
        let before = carl(&s);
        s.step(&[melee(3, 0, ForceTier::Heavy)]);
        assert_eq!(carl(&s), before + 500);
    }

    #[test]
    fn assist_pays_thirty_percent_truncated_to_the_instigator() {
        let mut s = unlocked();
        // Carl broke the shore, Donut's hit put the goblin under it.
        s.step(&[AudienceInput::Defeat {
            target: 1,
            cause: DefeatCause::ShoreBlock { player: 0 },
            force: ForceTier::Catastrophic,
            instigator: Some(1),
        }]);
        assert_eq!(carl(&s), 250 + 1_800);
        assert_eq!(donut(&s), 250 + 540);
        // Truncation: 500 * 30 / 100 = 150; 120 * 30 / 100 = 36; 7 -> 2.
        let mut s = unlocked();
        s.step(&[AudienceInput::Defeat {
            target: 1,
            cause: DefeatCause::Explosion {
                charge: 3,
                player: Some(1),
                point_blank: false,
            },
            force: ForceTier::Heavy,
            instigator: Some(0),
        }]);
        assert_eq!(donut(&s), 250 + 500);
        assert_eq!(carl(&s), 250 + 150);
        // An unknown instigator index earns no assist.
        let mut s = unlocked();
        s.step(&[AudienceInput::Defeat {
            target: 1,
            cause: DefeatCause::ShoreBlock { player: 0 },
            force: ForceTier::Catastrophic,
            instigator: Some(5),
        }]);
        assert_eq!(donut(&s), 250);
    }

    #[test]
    fn melee_has_no_assist_and_hero_hurt_never_awards() {
        let mut s = unlocked();
        s.step(&[melee(1, 1, ForceTier::Routine)]);
        assert_eq!(carl(&s), 250);
        assert_eq!(
            AudienceInput::from_combat(&CombatEvent::HeroHurt {
                tick: 1,
                source: 0,
                health_left: 1,
                hero: HeroId::Carl,
            }),
            None
        );
        assert_eq!(
            AudienceInput::from_combat(&CombatEvent::HeroDefeated {
                tick: 1,
                hero: HeroId::Donut,
            }),
            None
        );
        assert_eq!(
            AudienceInput::from_combat(&CombatEvent::GoblinTell { tick: 1, goblin: 0 }),
            None
        );
    }

    #[test]
    fn from_combat_maps_defeat_restart_and_respawn() {
        let defeat = CombatEvent::enemy_defeated(
            9,
            4,
            DefeatCause::ShoreBlockEdge { player: 1 },
            Some(0),
            None,
        );
        assert_eq!(
            AudienceInput::from_combat(&defeat),
            Some(AudienceInput::Defeat {
                target: 4,
                cause: DefeatCause::ShoreBlockEdge { player: 1 },
                force: ForceTier::Catastrophic,
                instigator: Some(0),
            })
        );
        assert_eq!(
            AudienceInput::from_combat(&CombatEvent::Restart { tick: 1, deaths: 2 }),
            Some(AudienceInput::Restart)
        );
        assert_eq!(
            AudienceInput::from_combat(&CombatEvent::Respawn {
                tick: 1,
                checkpoint: 0
            }),
            Some(AudienceInput::Respawn)
        );
    }

    #[test]
    fn chain_award_scales_caps_and_pays_an_upgrade_only_the_difference() {
        let chain = |score| {
            AudienceInput::Chain(ChainFeat {
                credited: 1,
                target: 3,
                score,
            })
        };
        for (score, want) in [
            (0, 0),
            (4, 0),
            (5, 250),
            (6, 500),
            (8, 1_000),
            (9, 1_000),
            (255, 1_000),
        ] {
            let mut s = unlocked();
            s.step(&[chain(score)]);
            assert_eq!(donut(&s), 250 + want, "score {score}");
            assert_eq!(carl(&s), 250);
        }
        let mut s = unlocked();
        s.step(&[chain(5)]);
        s.step(&[chain(5)]);
        assert_eq!(donut(&s), 500, "same feat twice pays once");
        s.step(&[chain(8)]);
        assert_eq!(donut(&s), 250 + 1_000, "an upgrade pays the difference");
        s.step(&[AudienceInput::Chain(ChainFeat {
            credited: 7,
            target: 1,
            score: 8,
        })]);
        assert_eq!(donut(&s), 1_250, "unknown hero index awards nothing");
    }

    fn cue(id: u32, x: i32, heroes: &[HeroId], award: u64) -> AnticipationCue {
        AnticipationCue {
            id,
            trigger: AnticipationTrigger::HeroCrossesX { x },
            heroes: heroes.to_vec(),
            award,
        }
    }

    #[test]
    fn cue_validation_rejects_bad_tables() {
        use HeroId::{Carl, Donut};
        assert_eq!(
            validate_cues(&[cue(1, 0, &[Carl], 4_000), cue(2, 0, &[Carl, Donut], 15_000)]),
            Ok(())
        );
        assert_eq!(
            validate_cues(&[cue(1, 0, &[Carl], 4_000), cue(1, 5, &[Donut], 4_000)]),
            Err(CueError::DuplicateId(1))
        );
        assert_eq!(
            validate_cues(&[cue(2, 0, &[], 4_000)]),
            Err(CueError::NoHeroes(2))
        );
        assert_eq!(
            validate_cues(&[cue(3, 0, &[Carl, Carl], 4_000)]),
            Err(CueError::DuplicateHero(3))
        );
        assert_eq!(
            validate_cues(&[cue(4, 0, &[Carl], 3_999)]),
            Err(CueError::AwardOutOfRange(4))
        );
        assert_eq!(
            validate_cues(&[cue(5, 0, &[Carl], 15_001)]),
            Err(CueError::AwardOutOfRange(5))
        );
        let mut s = unlocked();
        assert!(s.set_cues(vec![cue(1, 0, &[], 4_000)]).is_err());
    }

    #[test]
    fn anticipation_fires_once_for_the_targeted_heroes_only() {
        let mut s = unlocked();
        s.set_cues(vec![cue(7, 300, &[HeroId::Donut], 9_000)])
            .unwrap();
        s.step(&[AudienceInput::HeroX {
            hero: HeroId::Carl,
            x: 299,
        }]);
        assert_eq!(donut(&s), 250, "not crossed yet");
        s.step(&[AudienceInput::HeroX {
            hero: HeroId::Carl,
            x: 300,
        }]);
        assert_eq!((carl(&s), donut(&s)), (250, 9_250));
        s.step(&[AudienceInput::HeroX {
            hero: HeroId::Donut,
            x: 500,
        }]);
        assert_eq!(donut(&s), 9_250, "fires once per attempt");
        assert_eq!(s.view(HeroId::Donut).unwrap().trend, Trend::Up);
        assert_eq!(s.view(HeroId::Carl).unwrap().trend, Trend::Flat);
    }

    #[test]
    fn decay_waits_then_cuts_the_excess_with_a_milli_remainder() {
        let mut s = unlocked();
        s.step(&[AudienceInput::Chain(ChainFeat {
            credited: 0,
            target: 1,
            score: 8,
        })]);
        assert_eq!(carl(&s), 1_250);
        // The award step reset inactivity; 479 idle steps change nothing.
        idle(&mut s, 479);
        assert_eq!(carl(&s), 1_250);
        // The 480th idle step (8 s) is the first decay: excess 1000 * 23 = 23_000 -> 23, rem 0.
        idle(&mut s, 1);
        assert_eq!(carl(&s), 1_227);
        idle(&mut s, 59);
        assert_eq!(carl(&s), 1_227);
        idle(&mut s, 1);
        // excess 977 * 23 = 22_471 -> 22, rem 471.
        assert_eq!(carl(&s), 1_205);
        assert_eq!(donut(&s), 250, "the other hero was at the floor");
    }

    #[test]
    fn small_excess_still_decays_through_the_remainder() {
        let mut s = unlocked();
        s.step(&[melee(1, 0, ForceTier::Routine)]);
        assert_eq!(carl(&s), 370);
        // excess 120: 120 * 23 = 2760 -> 2, rem 760.
        idle(&mut s, 480);
        assert_eq!(carl(&s), 368);
        // excess 43: 43 * 23 = 989 would truncate to 0 forever without the
        // remainder; with it the second period pays 1.
        let mut s = unlocked();
        s.heroes[0] = HeroAudience::at(293, 293, 0);
        idle(&mut s, 480);
        assert_eq!(carl(&s), 293, "989 milli: nothing yet");
        idle(&mut s, 60);
        assert_eq!(carl(&s), 292, "1978 milli: one viewer");
    }

    #[test]
    fn decay_never_goes_below_the_baseline_and_resets_per_hero() {
        let mut s = unlocked();
        s.step(&[melee(1, 0, ForceTier::Heavy), melee(2, 1, ForceTier::Heavy)]);
        idle(&mut s, 400);
        // Only Carl earns something now; Donut's timer keeps running.
        s.step(&[melee(3, 0, ForceTier::Routine)]);
        idle(&mut s, 100);
        assert!(donut(&s) < 750, "Donut started decaying");
        assert_eq!(carl(&s), 250 + 500 + 120, "Carl's timer was reset");
        idle(&mut s, 60 * 600);
        assert_eq!((carl(&s), donut(&s)), (250, 250));
    }

    #[test]
    fn no_underflow_or_overflow_at_the_extremes() {
        let mut s = unlocked();
        s.heroes[0].current = u64::MAX - 100;
        s.heroes[0].peak = u64::MAX - 100;
        s.heroes[0].shown = u64::MAX - 100;
        s.heroes[0].fall_ref = u64::MAX - 100;
        s.step(&[AudienceInput::Chain(ChainFeat {
            credited: 0,
            target: 1,
            score: 255,
        })]);
        assert_eq!(carl(&s), u64::MAX, "saturates");
        idle(&mut s, 481);
        assert!(carl(&s) < u64::MAX);
        idle(&mut s, 3_000);
        assert!(carl(&s) >= TUNING.baseline);
    }

    #[test]
    fn spike_threshold_and_up_arrow_for_48_steps() {
        let mut s = unlocked();
        // The spike floor is 300 (15% of 250 is only 37).
        s.step(&[melee(1, 0, ForceTier::Routine)]);
        assert_eq!(
            s.view(HeroId::Carl).unwrap().trend,
            Trend::Flat,
            "120 is small"
        );
        s.step(&[melee(2, 0, ForceTier::Heavy)]);
        let v = s.view(HeroId::Carl).unwrap();
        assert_eq!(v.trend, Trend::Up, "500 >= 300 is a spike");
        assert_eq!(v.trend_steps_left, 48);
        assert_eq!(v.shown, 870, "a spike refreshes the shown count at once");
        idle(&mut s, 47);
        assert_eq!(s.view(HeroId::Carl).unwrap().trend, Trend::Up);
        assert_eq!(s.view(HeroId::Carl).unwrap().trend_steps_left, 1);
        idle(&mut s, 1);
        assert_eq!(s.view(HeroId::Carl).unwrap().trend, Trend::Flat);
    }

    #[test]
    fn relative_spike_threshold_scales_with_the_count() {
        let mut s = unlocked();
        s.heroes[0] = HeroAudience::at(100_000, 100_000, 0);
        // 15% of 100,000 is 15,000: a 1,800 award is not a spike.
        s.step(&[melee(1, 0, ForceTier::Catastrophic)]);
        assert_eq!(s.view(HeroId::Carl).unwrap().trend, Trend::Flat);
        s.step(&[AudienceInput::Chain(ChainFeat {
            credited: 0,
            target: 2,
            score: 8,
        })]);
        assert_eq!(s.view(HeroId::Carl).unwrap().trend, Trend::Flat);
    }

    #[test]
    fn a_new_spike_restarts_the_arrow_without_a_queue() {
        let mut s = unlocked();
        s.step(&[melee(1, 0, ForceTier::Heavy)]);
        idle(&mut s, 30);
        assert_eq!(s.view(HeroId::Carl).unwrap().trend_steps_left, 18);
        s.step(&[melee(2, 0, ForceTier::Heavy)]);
        assert_eq!(s.view(HeroId::Carl).unwrap().trend_steps_left, 48);
    }

    #[test]
    fn display_refreshes_every_12_steps_and_counts_up_over_30() {
        let mut s = unlocked();
        s.step(&[melee(1, 0, ForceTier::Routine)]);
        assert_eq!(s.view(HeroId::Carl).unwrap().shown, 250, "small gain waits");
        idle(&mut s, 10);
        assert_eq!(s.view(HeroId::Carl).unwrap().shown, 250);
        idle(&mut s, 1);
        let v = s.view(HeroId::Carl).unwrap();
        assert_eq!(v.shown, 370, "refreshed on the 12th step");
        assert_eq!(v.animated, 250);
        idle(&mut s, 15);
        let mid = s.view(HeroId::Carl).unwrap();
        assert!(mid.animated > 250 && mid.animated < 370);
        assert_eq!(mid.display_count(true), 370, "reduced motion snaps");
        assert_eq!(mid.display_count(false), mid.animated);
        idle(&mut s, 15);
        assert_eq!(s.view(HeroId::Carl).unwrap().animated, 370);
    }

    #[test]
    fn fall_cue_needs_the_threshold_and_has_a_cooldown() {
        let mut s = unlocked();
        s.heroes[0] = HeroAudience::at(20_000, 20_000, 0);
        // Idle: decay starts after 480 steps; 2% steps of the excess.
        let mut first_down = None;
        let mut downs = 0;
        let mut was_down = false;
        for n in 1..=4_000u32 {
            s.step(&[]);
            let down = s.view(HeroId::Carl).unwrap().trend == Trend::Down;
            if down && !was_down {
                downs += 1;
                first_down.get_or_insert(n);
            }
            was_down = down;
        }
        let first = first_down.expect("a fall cue fired");
        assert!(first > 480, "never before decay starts: {first}");
        assert!(downs >= 2, "the cue repeats after the cooldown");
        assert!(downs <= 4_000 / 600 + 1);
        assert_eq!(
            s.view(HeroId::Donut).unwrap().trend,
            Trend::Flat,
            "Donut is at the floor"
        );
    }

    #[test]
    fn a_spike_replaces_a_down_arrow() {
        let mut s = unlocked();
        s.heroes[0] = HeroAudience::at(20_000, 20_000, 0);
        s.set_cues(vec![cue(1, 0, &[HeroId::Carl], 15_000)])
            .unwrap();
        let mut guard = 0;
        while s.view(HeroId::Carl).unwrap().trend != Trend::Down {
            s.step(&[]);
            guard += 1;
            assert!(guard < 5_000);
        }
        s.step(&[AudienceInput::HeroX {
            hero: HeroId::Carl,
            x: 0,
        }]);
        let v = s.view(HeroId::Carl).unwrap();
        assert_eq!((v.trend, v.trend_steps_left), (Trend::Up, 48));
    }

    #[test]
    fn restart_restores_the_attempt_start_counts_and_dedupe() {
        let mut s = unlocked();
        s.step(&[melee(1, 0, ForceTier::Heavy)]);
        s.begin_attempt();
        let start = (carl(&s), donut(&s));
        s.set_cues(vec![cue(1, 10, &[HeroId::Carl], 5_000)])
            .unwrap();
        s.step(&[
            melee(2, 0, ForceTier::Heavy),
            melee(3, 1, ForceTier::Catastrophic),
        ]);
        s.step(&[AudienceInput::HeroX {
            hero: HeroId::Carl,
            x: 10,
        }]);
        assert_ne!((carl(&s), donut(&s)), start);
        s.step(&[AudienceInput::Restart]);
        assert_eq!((carl(&s), donut(&s)), start);
        let v = s.view(HeroId::Carl).unwrap();
        assert_eq!(
            (v.shown, v.animated, v.trend),
            (start.0, start.0, Trend::Flat)
        );
        // The same target ids and the same cue pay again, not twice.
        s.step(&[melee(2, 0, ForceTier::Heavy)]);
        s.step(&[AudienceInput::HeroX {
            hero: HeroId::Carl,
            x: 10,
        }]);
        assert_eq!(carl(&s), start.0 + 500 + 5_000);
        // Target 1 was defeated before the snapshot: no farming after a restart.
        s.step(&[AudienceInput::Restart]);
        s.step(&[melee(1, 0, ForceTier::Heavy)]);
        assert_eq!(carl(&s), start.0);
    }

    #[test]
    fn restart_never_farms_through_repeated_deaths() {
        let mut s = unlocked();
        for _ in 0..50 {
            s.step(&[melee(1, 0, ForceTier::Catastrophic), AudienceInput::Restart]);
            s.step(&[melee(1, 0, ForceTier::Catastrophic)]);
            s.step(&[AudienceInput::Restart]);
        }
        assert_eq!(carl(&s), 250);
    }

    #[test]
    fn restart_keeps_the_monotonic_step_counter() {
        let mut s = unlocked();
        idle(&mut s, 10);
        s.step(&[AudienceInput::Restart]);
        assert_eq!(s.steps(), 11);
    }

    #[test]
    fn respawn_keeps_counts_and_the_dedupe_set() {
        let mut s = unlocked();
        s.step(&[melee(1, 0, ForceTier::Heavy)]);
        let c = carl(&s);
        s.step(&[AudienceInput::Respawn]);
        assert_eq!(carl(&s), c, "a death never costs viewers");
        s.step(&[melee(1, 0, ForceTier::Heavy)]);
        assert_eq!(carl(&s), c, "defeated enemies stay defeated");
    }

    #[test]
    fn restart_in_the_same_step_discards_earlier_awards_only() {
        let mut s = unlocked();
        s.step(&[
            melee(1, 0, ForceTier::Heavy),
            AudienceInput::Restart,
            melee(2, 0, ForceTier::Routine),
        ]);
        assert_eq!(carl(&s), 250 + 120);
    }

    #[test]
    fn ownership_is_independent_of_controllers_solo_ai_and_two_player() {
        // The domain only sees hero indices. A solo run where the player swapped
        // to Donut, a solo run where Donut is an AI follower and a 2P run where
        // P1 drives Donut all feed the same index-1 input and credit Donut.
        for _ in 0..3 {
            let mut s = unlocked();
            s.step(&[melee(1, 1, ForceTier::Heavy)]);
            assert_eq!((carl(&s), donut(&s)), (250, 750));
        }
        // Both heroes score in the same step without merging totals.
        let mut s = unlocked();
        s.step(&[
            melee(1, 0, ForceTier::Routine),
            melee(2, 1, ForceTier::Heavy),
        ]);
        assert_eq!((carl(&s), donut(&s)), (370, 750));
        // A swap never clones the scoreboard: only the credit index matters.
        s.step(&[melee(3, 0, ForceTier::Routine)]);
        assert_eq!((carl(&s), donut(&s)), (490, 750));
    }

    #[test]
    fn peak_tracks_the_highest_count_per_hero() {
        let mut s = unlocked();
        s.step(&[melee(1, 0, ForceTier::Catastrophic)]);
        let peak = s.view(HeroId::Carl).unwrap().peak;
        assert_eq!(peak, 2_050);
        idle(&mut s, 5_000);
        assert_eq!(s.view(HeroId::Carl).unwrap().peak, peak);
        assert_eq!(s.view(HeroId::Donut).unwrap().peak, 250);
    }

    #[test]
    fn the_run_is_deterministic_and_the_digest_is_sensitive() {
        let script = |s: &mut AudienceState| {
            s.set_cues(vec![cue(1, 50, &[HeroId::Carl, HeroId::Donut], 4_000)])
                .unwrap();
            for n in 0..2_000u32 {
                let mut inputs = Vec::new();
                if n % 97 == 0 {
                    inputs.push(melee(n, (n / 97) % 2, ForceTier::Heavy));
                }
                if n == 300 {
                    inputs.push(AudienceInput::HeroX {
                        hero: HeroId::Carl,
                        x: 60,
                    });
                }
                if n == 1_500 {
                    inputs.push(AudienceInput::Restart);
                }
                s.step(&inputs);
            }
        };
        let (mut a, mut b) = (unlocked(), unlocked());
        script(&mut a);
        script(&mut b);
        assert_eq!(a.digest(), b.digest());
        assert_eq!(a, b);
        let mut c = unlocked();
        script(&mut c);
        c.step(&[melee(99_999, 0, ForceTier::Routine)]);
        assert_ne!(a.digest(), c.digest());
    }

    #[test]
    fn audience_is_a_pure_observer_of_combat_events() {
        // `from_combat` borrows the event immutably, so it cannot alter a replay;
        // mapping the same event twice yields the same input.
        let ev = CombatEvent::enemy_defeated(5, 2, DefeatCause::Melee, Some(1), None);
        assert_eq!(
            AudienceInput::from_combat(&ev),
            AudienceInput::from_combat(&ev)
        );
        // Locked: events change nothing at all, not even the (absent) digest.
        let mut locked = AudienceState::locked();
        locked.step(&[AudienceInput::from_combat(&ev).unwrap()]);
        assert_eq!(locked, AudienceState::locked());
    }

    #[test]
    fn formatting_test_vectors() {
        let cases: [(u64, &str); 24] = [
            (0, "0"),
            (7, "7"),
            (999, "999"),
            (1_000, "1.0K"),
            (1_099, "1.0K"),
            (1_100, "1.1K"),
            (12_499, "12.4K"),
            (99_999, "99.9K"),
            (100_000, "100K"),
            (159_999, "159K"),
            (999_999, "999K"),
            (1_000_000, "1.0M"),
            (1_299_999, "1.2M"),
            (9_999_999, "9.9M"),
            (10_000_000, "10M"),
            (12_345_678, "12M"),
            (999_999_999, "999M"),
            (1_000_000_000, "999M+"),
            (u64::MAX, "999M+"),
            (1, "1"),
            (10, "10"),
            (100, "100"),
            (9_999, "9.9K"),
            (10_000, "10.0K"),
        ];
        for (n, want) in cases {
            assert_eq!(format_count(n), want, "{n}");
        }
    }

    #[test]
    fn formatted_text_stays_inside_five_glyphs_and_the_reserved_field() {
        let mut n = 0u64;
        while n < 2_000_000_000 {
            let text = format_count(n);
            assert!(text.len() <= MAX_TEXT_LEN, "{n} -> {text}");
            assert!(text.is_ascii());
            assert!(text_ink_width(&text) <= 29, "{n} -> {text}");
            n = n * 11 / 10 + 1;
        }
        for (text, width) in [
            ("999M+", 29),
            ("99.9K", 25),
            ("999M", 23),
            ("159K", 23),
            ("999", 17),
        ] {
            assert_eq!(text_ink_width(text), width, "{text}");
        }
        // Reserved field +104..+150 is 47 px wide, and the glyph slot starts at +153.
        assert!(104 + text_ink_width("999M+") < 150);
    }

    #[test]
    fn name_bound_falls_back_when_the_ink_runs_into_the_eye() {
        assert_eq!(NAME_MAX_INK_WIDTH, 49);
        assert_eq!(bounded_name("DONUT", 40, HeroId::Donut), "DONUT");
        assert_eq!(bounded_name("CARL", 30, HeroId::Carl), "CARL");
        assert_eq!(bounded_name("X", 49, HeroId::Carl), "X");
        assert_eq!(bounded_name("PRINCESS DONUT", 50, HeroId::Donut), "ALLY");
        assert_eq!(bounded_name("LONG NAME", 90, HeroId::Carl), "HERO");
    }

    #[test]
    fn tuning_table_is_internally_consistent() {
        let t = TUNING;
        assert!(t.award_routine < t.award_heavy && t.award_heavy < t.award_catastrophic);
        assert!(t.anticipation_min <= t.anticipation_max);
        assert!(t.decay_permille < 1_000, "decay never exceeds the excess");
        assert!(t.count_up_steps <= 30);
        assert!(t.trend_steps >= 1 && t.refresh_steps >= 1);
    }
}
