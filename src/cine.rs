//! CINE-01 phase 1: the skill-gated cinematic finisher, Bevy-free.
//!
//! Spec: `docs/03-tech/cinematic-finishers-cine-01.md`. This module is a pure
//! **observer**. It reads authoritative combat facts as plain data
//! ([`CineEvent`]), decides fail-closed whether a defeat was an exceptional,
//! player-authored feat ([`CinematicEvaluator`]), and hands a
//! presentation-only slow-motion request to an adapter ([`SlowMoRequest`],
//! [`SlowMoPlanner`]). Nothing here writes a combat event, a hash input, an RNG
//! draw or a collision: the evaluator returns values and the simulation never
//! reads them back. Replays with the same events give the same moments whether
//! or not presentation is switched on.
//!
//! Every number lives in [`CineTuning`] (one table, one place to adjust).
//!
//! Layout:
//! 1. [`CineTuning`]: the single tuning table.
//! 2. [`CineEvent`] / [`CineCause`]: the input mirror of the combat events
//!    ([`CineEvent::from_combat`] is the whole adapter mapping).
//! 3. [`CinematicEvaluator`]: bounded rolling history, scoring, tie-breaks,
//!    cooldown, upgrade-of-a-pending-moment.
//! 4. [`SlowMoPlanner`]: `Idle -> Pending -> Active -> Idle` presentation
//!    clock (real-time driven), settings, pause and lock cancellation.
//! 5. [`EdgeLatch`]: the press-edge buffer that keeps every press consumed
//!    exactly once while fewer fixed ticks run per real second.

use std::collections::VecDeque;

use crate::boss::BOSS_ENEMY_ID;
use crate::combat::{CombatEvent, DefeatCause};
use crate::hud_settings::parse_switch;

// ------------------------------------------------------------------- tuning --

/// The one tuning table of CINE-01 (spec section 3 plus the feat scores of
/// section 2). Initial playtest defaults, **not** owner-approved game feel and
/// never part of authoritative or replay state. Fields marked *(assumption)*
/// are not in the spec and exist so the code has no hidden magic numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CineTuning {
    /// Rolling chain window, in simulation ticks (1.5 s at 60 Hz).
    pub history_window_ticks: u64,
    /// No hurt to the credited hero in this many ticks before the defeat,
    /// including the defeat tick.
    pub clean_window_ticks: u64,
    /// Simulation ticks after an emitted moment before another may trigger.
    pub cooldown_ticks: u64,
    /// Lowest eligible primary-feat score.
    pub ordinary_threshold: u8,
    /// Score of the rarer tier.
    pub legendary_threshold: u8,
    /// Score of `CINE-PRECISION-EDGE`.
    pub precision_edge_score: u8,
    /// Score of `CINE-MIXED-CHAIN`.
    pub mixed_chain_score: u8,
    /// Bonus of `CINE-CONTACT-SETUP`.
    pub contact_setup_bonus: u8,
    /// Review F4: `CINE-CONTACT-SETUP` is the only road to the legendary tier
    /// (5 + the capped 2 extra kills = 7 < 8). The linkage is the spec's: the
    /// contact's `source` or `target` is defeated by the credited hero within
    /// `history_window_ticks` of the contact, with the launch proved to be
    /// that hero's by a `LaunchCredited` fact (a `PushKick` `Hit` whose
    /// `instance` equals `GoblinContact.launch`). Without that proof the bonus
    /// is never awarded; set `false` to switch the bonus off entirely.
    pub contact_setup_enabled: bool,
    /// Bonus per extra credited distinct defeat (`CINE-EXTRA-KILL`).
    pub extra_kill_bonus: u8,
    /// Cap of the extra-kill bonuses together.
    pub extra_kill_bonus_cap: u8,
    /// World time multiplier of the ordinary tier, in thousandths (0.35x).
    pub ordinary_scale_permille: u16,
    /// World time multiplier of the legendary tier, in thousandths (0.25x).
    pub legendary_scale_permille: u16,
    /// Real-time length of the ordinary moment, blends included.
    pub ordinary_duration_ms: u32,
    /// Real-time length of the legendary moment, blends included.
    pub legendary_duration_ms: u32,
    /// Ramp from 1.0x to the tier scale.
    pub blend_in_ms: u32,
    /// Ramp from the tier scale back to 1.0x.
    pub blend_out_ms: u32,
    /// Maximum optional camera emphasis, in virtual pixels.
    pub camera_emphasis_px: u8,
    /// *(assumption)* Hard cap of the evaluator's rolling entries, so memory
    /// stays bounded even under a stress of simultaneous defeats (CINE-T13).
    pub max_history_entries: usize,
    /// *(assumption)* After a moment is emitted, a strictly higher-tier
    /// candidate within this many ticks upgrades it ("one candidate can
    /// upgrade a pending moment before it starts"). Defaults to the longest
    /// current hit-stop (the push kick, 8 ticks) because the spec gives no
    /// number.
    pub upgrade_window_ticks: u64,
    /// *(assumption)* Pending press edges kept per action. 1 is the current
    /// latch (`Controls`, `bool` taken once per fixed tick); 2 would keep a
    /// second press made inside one slowed tick for the next tick.
    pub edge_buffer_cap: u8,
}

impl CineTuning {
    pub const DEFAULT: CineTuning = CineTuning {
        history_window_ticks: 90,
        clean_window_ticks: 90,
        cooldown_ticks: 900,
        ordinary_threshold: 5,
        legendary_threshold: 8,
        precision_edge_score: 5,
        mixed_chain_score: 5,
        contact_setup_bonus: 2,
        contact_setup_enabled: true,
        extra_kill_bonus: 1,
        extra_kill_bonus_cap: 2,
        ordinary_scale_permille: 350,
        legendary_scale_permille: 250,
        ordinary_duration_ms: 480,
        legendary_duration_ms: 650,
        blend_in_ms: 35,
        blend_out_ms: 90,
        camera_emphasis_px: 2,
        max_history_entries: 64,
        upgrade_window_ticks: 8,
        edge_buffer_cap: 1,
    };

    /// The presentation request a moment turns into: tier scale, duration,
    /// blends, cooldown. Presentation data only.
    pub fn request_for(&self, moment: &CinematicMoment) -> SlowMoRequest {
        let (scale_permille, duration_ms) = match moment.tier {
            CineTier::Ordinary => (self.ordinary_scale_permille, self.ordinary_duration_ms),
            CineTier::Legendary => (self.legendary_scale_permille, self.legendary_duration_ms),
        };
        SlowMoRequest {
            tier: moment.tier,
            credited: moment.credited,
            tick: moment.tick,
            scale_permille,
            duration_ms,
            blend_in_ms: self.blend_in_ms,
            blend_out_ms: self.blend_out_ms,
            camera_emphasis_px: self.camera_emphasis_px,
            cooldown_ticks: self.cooldown_ticks,
        }
    }
}

impl Default for CineTuning {
    fn default() -> Self {
        Self::DEFAULT
    }
}

// -------------------------------------------------------------------- input --

/// How a defeat happened, as far as the evaluator cares. Force, gore and
/// finish selection are deliberately absent: they are outputs of the death
/// pipeline, never inputs here (spec section 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CineCause {
    /// A direct melee defeat, credited to the event's `instigator`.
    Melee,
    /// The falling CP-03 block, credited to `player` (not to `instigator`).
    ShoreBlock { player: u32 },
    /// DEATH-D's upright neck-edge shear of the block (`CINE-PRECISION-EDGE`).
    ShoreBlockEdge { player: u32 },
    /// A player-dropped plate rack (`DefeatCause::DropHazard`, the boss
    /// arena). The cause names the player who dropped it, which is the proof
    /// of ownership; it counts as a falling-block kill for the mixed chain.
    /// The boss itself is never mapped here (boss defeats never trigger CINE
    /// in v0), so in practice no v0 chain reaches it.
    DropHazard { player: u32 },
    /// Any cause this evaluator does not know. Never qualifies, never credited.
    Other,
}

/// The facts the evaluator consumes. Ticks are attempt-local, like
/// [`CombatEvent`]. The adapter maps combat events into this and nothing else.
///
/// **Credit identity (review F1).** Every `player`/`instigator` number in
/// these events is a **hero index** (Carl 0, Donut 1), as on `main` and in the
/// Q10 pounce credit. "Same player" in a chain means the same hero: a Donut
/// kill and a block broken as Carl are two credits, even if one human drove
/// both through a solo swap. The *owning human slot* at the event tick travels
/// next to it (`slot`, supplied by the adapter) and is used only to name the
/// credited slot of a moment; mapping slots to heroes beyond that is left to
/// the future `CINE-DUO`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CineEvent {
    /// `CombatEvent::EnemyDefeated` (once per alive -> dead transition).
    Defeated {
        tick: u64,
        target: u32,
        cause: CineCause,
        instigator: Option<u32>,
        /// Human slot owning the credited hero at this tick, if the adapter
        /// knows it (`None` from [`CineEvent::from_combat`]).
        slot: Option<u32>,
    },
    /// `CombatEvent::HeroHurt`. `player` is the hurt **hero index**.
    HeroHurt { tick: u64, player: u32 },
    /// `CombatEvent::HeroDefeated`: a party defeat; locks the evaluator until
    /// a restart or respawn.
    HeroDefeated { tick: u64 },
    /// `CombatEvent::ShoreBroken` (the verified break of the credited player).
    ShoreBroken { tick: u64, player: u32 },
    /// **Bridge, not in `CombatEvent` today:** `player`'s hit launched the
    /// living `goblin`. `CombatEvent::Hit` has no attacker field, so the
    /// contact setup is never awarded until the adapter phase can prove this
    /// association (spec: "never inferred from screen proximity").
    LaunchCredited { tick: u64, goblin: u32, player: u32 },
    /// `CombatEvent::GoblinContact` (`launch` is not needed here).
    GoblinContact { tick: u64, source: u32, target: u32 },
    /// `CombatEvent::Restart`: a new attempt, ticks start again at 1.
    Restart { tick: u64 },
    /// `CombatEvent::Respawn` (demo): history is dropped, the cooldown stays.
    Respawn { tick: u64 },
}

impl CineEvent {
    pub const fn tick(&self) -> u64 {
        match *self {
            Self::Defeated { tick, .. }
            | Self::HeroHurt { tick, .. }
            | Self::HeroDefeated { tick }
            | Self::ShoreBroken { tick, .. }
            | Self::LaunchCredited { tick, .. }
            | Self::GoblinContact { tick, .. }
            | Self::Restart { tick }
            | Self::Respawn { tick } => tick,
        }
    }

    /// The whole adapter mapping for events `CombatEvent` already carries.
    /// `HeroHurt` maps a hero by its index (Carl 0, Donut 1), the same space
    /// as a melee `instigator` and `ShoreBlock.player`. The owning slot is
    /// left `None`: the adapter adds it with [`CineEvent::with_slot`].
    /// Unknown causes map to [`CineCause::Other`]; unrelated events to `None`.
    pub fn from_combat(event: &CombatEvent) -> Option<Self> {
        Some(match *event {
            CombatEvent::EnemyDefeated {
                tick,
                target,
                cause,
                instigator,
                ..
            } => Self::Defeated {
                tick,
                target,
                // Review F8: a boss defeat never triggers CINE in v0.
                cause: if target == BOSS_ENEMY_ID {
                    CineCause::Other
                } else {
                    cine_cause(cause)
                },
                instigator,
                slot: None,
            },
            CombatEvent::HeroHurt { tick, hero, .. } => Self::HeroHurt {
                tick,
                player: hero.index() as u32,
            },
            CombatEvent::HeroDefeated { tick, .. } => Self::HeroDefeated { tick },
            CombatEvent::ShoreBroken { tick, player } => Self::ShoreBroken { tick, player },
            CombatEvent::GoblinContact {
                tick,
                source,
                target,
                ..
            } => Self::GoblinContact {
                tick,
                source,
                target,
            },
            CombatEvent::Restart { tick, .. } => Self::Restart { tick },
            CombatEvent::Respawn { tick, .. } => Self::Respawn { tick },
            _ => return None,
        })
    }

    /// The same event with the owning slot of the credited hero attached
    /// (only a defeat carries one).
    pub const fn with_slot(self, slot: Option<u32>) -> Self {
        match self {
            Self::Defeated {
                tick,
                target,
                cause,
                instigator,
                ..
            } => Self::Defeated {
                tick,
                target,
                cause,
                instigator,
                slot,
            },
            other => other,
        }
    }

    /// Deterministic in-tick processing order, independent of the order the
    /// caller supplied the events in. Resets come first, defeats last so every
    /// supporting fact of the tick is recorded before the defeat is scored.
    fn sort_key(&self) -> (u8, u32, u32, u32) {
        match *self {
            Self::Restart { .. } => (0, 0, 0, 0),
            Self::Respawn { .. } => (1, 0, 0, 0),
            Self::ShoreBroken { player, .. } => (2, player, 0, 0),
            Self::HeroHurt { player, .. } => (3, player, 0, 0),
            Self::HeroDefeated { .. } => (4, 0, 0, 0),
            Self::LaunchCredited { goblin, player, .. } => (5, goblin, player, 0),
            Self::GoblinContact { source, target, .. } => (6, source, target, 0),
            Self::Defeated {
                target,
                cause,
                instigator,
                ..
            } => (
                7,
                target,
                match cause {
                    CineCause::Melee => 0,
                    CineCause::ShoreBlock { player } => 1 + player * 4,
                    CineCause::ShoreBlockEdge { player } => 2 + player * 4,
                    CineCause::DropHazard { player } => 3 + player * 4,
                    CineCause::Other => 8,
                },
                instigator.map_or(0, |p| p + 1),
            ),
        }
    }
}

/// The one place a combat `DefeatCause` becomes a [`CineCause`] (review F7).
/// Every player-activated drop hazard is recognised through
/// [`DefeatCause::drop_credit`] (`ShoreBlock`, `ShoreBlockEdge` and the boss
/// arena's `DropHazard`), never by naming one variant. The match is exhaustive
/// on purpose so a new cause cannot silently be credited or silently dropped.
fn cine_cause(cause: DefeatCause) -> CineCause {
    match cause {
        DefeatCause::Melee => CineCause::Melee,
        DefeatCause::ShoreBlockEdge { .. } => cause
            .drop_credit()
            .map_or(CineCause::Other, |player| CineCause::ShoreBlockEdge {
                player,
            }),
        DefeatCause::ShoreBlock { .. } => cause
            .drop_credit()
            .map_or(CineCause::Other, |player| CineCause::ShoreBlock { player }),
        DefeatCause::DropHazard { .. } => cause
            .drop_credit()
            .map_or(CineCause::Other, |player| CineCause::DropHazard { player }),
        // A blast is never a primary feat (spec T02).
        DefeatCause::Explosion { .. } => CineCause::Other,
    }
}

// ------------------------------------------------------------------ outputs --

/// The two presentation tiers. Legendary is a stronger variant of the one
/// selected moment, never a second execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CineTier {
    Ordinary,
    Legendary,
}

/// Which feats built a moment's score (for logs and evidence).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FeatSet {
    pub precision_edge: bool,
    pub mixed_chain: bool,
    pub contact_setup: bool,
    /// Counted `CINE-EXTRA-KILL` defeats (before the cap).
    pub extra_kills: u8,
}

/// An eligible exceptional defeat: the evaluator's only output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CinematicMoment {
    /// The tick of the batch that completed the feat.
    pub tick: u64,
    pub tier: CineTier,
    pub score: u8,
    /// The credited **hero index** (Carl 0, Donut 1).
    pub credited: u32,
    /// The human slot that owned the credited hero at the completing defeat,
    /// when the adapter supplied it. Presentation naming only.
    pub credited_slot: Option<u32>,
    /// The lowest target id this player defeated in the completing batch.
    pub target: u32,
    pub feats: FeatSet,
    /// `true` when this replaces the tier of the moment emitted a few ticks
    /// earlier (the planner applies it only while that moment is still
    /// pending).
    pub upgrade: bool,
}

// --------------------------------------------------------------- evaluator --

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DefeatKind {
    Melee,
    Shore,
    Edge,
}

#[derive(Debug, Clone, Copy)]
struct DefeatEntry {
    tick: u64,
    target: u32,
    /// `None`: seen (for deduplication) but not proven player-authored.
    credit: Option<u32>,
    slot: Option<u32>,
    kind: DefeatKind,
}

#[derive(Debug, Clone, Copy)]
struct Launch {
    tick: u64,
    goblin: u32,
    player: u32,
}

#[derive(Debug, Clone, Copy)]
struct Contact {
    tick: u64,
    source: u32,
    target: u32,
}

#[derive(Debug, Clone, Copy)]
struct Emitted {
    /// The monotonic clock (`epoch + attempt-local tick`) the moment was
    /// emitted at, so the cooldown survives `Restart` and `Respawn`.
    clock: u64,
    tier: CineTier,
}

/// Why the last batch produced no moment. Diagnostics and tests only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    /// Nothing credited was defeated in the batch.
    NoCreditedDefeat,
    /// A credited defeat without a primary feat, or below the threshold.
    NoQualifyingFeat,
    /// The credited hero was hurt inside the clean window.
    NotClean,
    /// Cooldown after an earlier moment.
    Cooldown,
    /// Locked: party defeat, restart wait or an adapter-reported lock.
    Locked,
    /// The batch arrived with a tick before the last evaluated one.
    OutOfOrder,
}

/// The pure, deterministic feat evaluator. Feed it one batch per resolved
/// tick, in tick order, with [`CinematicEvaluator::observe_tick`].
#[derive(Debug, Clone)]
pub struct CinematicEvaluator {
    tuning: CineTuning,
    defeats: VecDeque<DefeatEntry>,
    hurts: VecDeque<(u64, u32)>,
    launches: VecDeque<Launch>,
    contacts: VecDeque<Contact>,
    shore_breakers: Vec<u32>,
    last_tick: Option<u64>,
    /// Ticks of the attempts finished before this one (a `Restart` carries the
    /// tick the finished attempt ended on). `epoch + tick` is the monotonic
    /// clock the cooldown is measured on (spec section 2, attempt boundaries).
    epoch: u64,
    emitted: Option<Emitted>,
    /// Defeats at or before this tick belong to a rewarded chain.
    rewarded_through: Option<u64>,
    defeated_latch: bool,
    last_rejection: Option<Rejection>,
}

impl CinematicEvaluator {
    pub fn new(tuning: CineTuning) -> Self {
        Self {
            tuning,
            defeats: VecDeque::new(),
            hurts: VecDeque::new(),
            launches: VecDeque::new(),
            contacts: VecDeque::new(),
            shore_breakers: Vec::new(),
            last_tick: None,
            epoch: 0,
            emitted: None,
            rewarded_through: None,
            defeated_latch: false,
            last_rejection: None,
        }
    }

    pub fn tuning(&self) -> &CineTuning {
        &self.tuning
    }

    pub fn last_rejection(&self) -> Option<Rejection> {
        self.last_rejection
    }

    /// Rolling entries held (defeats, hurts, launches, contacts): the bound
    /// CINE-T13 checks.
    pub fn held_entries(&self) -> usize {
        self.defeats.len() + self.hurts.len() + self.launches.len() + self.contacts.len()
    }

    /// Back to a completely fresh evaluator (cooldown included).
    pub fn reset(&mut self) {
        *self = Self::new(self.tuning);
    }

    /// A new attempt after a `Restart` that ended the previous one on
    /// `ended_at`: history, deduplication and the shore breakers are dropped
    /// (the attempt-local tick restarts at 1 and target ids are reused), but
    /// the cooldown **survives** on the monotonic clock.
    fn restart_attempt(&mut self, ended_at: u64) {
        let (epoch, emitted) = (self.epoch + ended_at, self.emitted);
        self.reset();
        self.epoch = epoch;
        self.emitted = emitted;
    }

    /// Observe everything that resolved on `tick` and decide whether it
    /// completes an exceptional feat. `locked` is the adapter's flag for
    /// externally locked time flow (app pause, scripted capture): the batch is
    /// still recorded, but no moment is emitted and no cooldown starts.
    ///
    /// The slice order does not matter: events are processed in a fixed order,
    /// and events whose own tick differs from `tick` are ignored. A batch
    /// older than the last one (without a restart) is ignored entirely.
    pub fn observe_tick(
        &mut self,
        tick: u64,
        events: &[CineEvent],
        locked: bool,
    ) -> Option<CinematicMoment> {
        self.last_rejection = None;
        let mut batch: Vec<CineEvent> = events
            .iter()
            .copied()
            .filter(|e| e.tick() == tick)
            .collect();
        batch.sort_by_key(CineEvent::sort_key);
        batch.dedup();

        if batch.iter().any(|e| matches!(e, CineEvent::Restart { .. })) {
            self.restart_attempt(tick);
            return None;
        }
        if self.last_tick.is_some_and(|last| tick < last) {
            self.last_rejection = Some(Rejection::OutOfOrder);
            return None;
        }
        self.last_tick = Some(tick);

        if batch.iter().any(|e| matches!(e, CineEvent::Respawn { .. })) {
            self.defeats.clear();
            self.hurts.clear();
            self.launches.clear();
            self.contacts.clear();
            self.shore_breakers.clear();
            self.defeated_latch = false;
        }

        // Entries older than the window are gone before this batch is
        // recorded, so a target id reused after a long quiet spell is not
        // mistaken for a duplicate of its stale namesake.
        self.prune(tick);
        let mut batch_credited: Vec<(u32, u32)> = Vec::new();
        for event in &batch {
            match *event {
                CineEvent::ShoreBroken { player, .. } => {
                    if !self.shore_breakers.contains(&player) {
                        self.shore_breakers.push(player);
                    }
                }
                CineEvent::HeroHurt { tick, player } => self.hurts.push_back((tick, player)),
                CineEvent::HeroDefeated { .. } => self.defeated_latch = true,
                CineEvent::LaunchCredited {
                    tick,
                    goblin,
                    player,
                } => self.launches.push_back(Launch {
                    tick,
                    goblin,
                    player,
                }),
                CineEvent::GoblinContact {
                    tick,
                    source,
                    target,
                } => self.contacts.push_back(Contact {
                    tick,
                    source,
                    target,
                }),
                CineEvent::Defeated {
                    tick,
                    target,
                    cause,
                    instigator,
                    slot,
                } => {
                    if self.defeats.iter().any(|d| d.target == target) {
                        continue;
                    }
                    let (credit, kind) = self.credit_of(cause, instigator);
                    self.defeats.push_back(DefeatEntry {
                        tick,
                        target,
                        credit,
                        slot,
                        kind,
                    });
                    if let Some(player) = credit {
                        batch_credited.push((player, target));
                    }
                }
                CineEvent::Restart { .. } | CineEvent::Respawn { .. } => {}
            }
        }
        self.prune(tick);

        if batch_credited.is_empty() {
            self.last_rejection = Some(Rejection::NoCreditedDefeat);
            return None;
        }
        if locked || self.defeated_latch {
            self.last_rejection = Some(Rejection::Locked);
            return None;
        }

        // Inside the upgrade window the earlier (rewarded) kills are the same
        // chain still growing; after it they are spent.
        let now = self.epoch + tick;
        let growing = self
            .emitted
            .is_some_and(|e| now.saturating_sub(e.clock) <= self.tuning.upgrade_window_ticks);
        let mut best: Option<CinematicMoment> = None;
        let mut saw_unclean = false;
        let mut players: Vec<u32> = batch_credited.iter().map(|&(p, _)| p).collect();
        players.sort_unstable();
        players.dedup();
        for player in players {
            if self.hurts.iter().any(|&(t, p)| {
                p == player && tick.saturating_sub(t) <= self.tuning.clean_window_ticks
            }) {
                saw_unclean = true;
                continue;
            }
            let target = batch_credited
                .iter()
                .filter(|&&(p, _)| p == player)
                .map(|&(_, t)| t)
                .min()
                .unwrap_or(0);
            let Some(mut candidate) = self.score_player(tick, player, target, growing) else {
                continue;
            };
            // Deterministic pick: highest score, then lowest player, then
            // lowest target (players are visited in ascending order, so `>`
            // keeps the lower id on a score tie).
            if best.is_none_or(|b| candidate.score > b.score) {
                candidate.upgrade = false;
                best = Some(candidate);
            }
        }
        let Some(mut moment) = best else {
            self.last_rejection = Some(if saw_unclean {
                Rejection::NotClean
            } else {
                Rejection::NoQualifyingFeat
            });
            return None;
        };

        if let Some(prev) = self.emitted {
            let age = now.saturating_sub(prev.clock);
            let upgrade_ok = moment.tier > prev.tier && age <= self.tuning.upgrade_window_ticks;
            if age < self.tuning.cooldown_ticks && !upgrade_ok {
                self.last_rejection = Some(Rejection::Cooldown);
                return None;
            }
            if upgrade_ok {
                // Re-score with the earlier chain still counted: it is the
                // same chain growing, not a new one.
                moment.upgrade = true;
                self.emitted = Some(Emitted {
                    clock: prev.clock,
                    tier: moment.tier,
                });
                return Some(moment);
            }
        }
        self.emitted = Some(Emitted {
            clock: now,
            tier: moment.tier,
        });
        self.rewarded_through = Some(tick);
        Some(moment)
    }

    /// Credit of a defeat, fail-closed: a melee defeat needs an `instigator`;
    /// a block defeat needs a prior `ShoreBroken` by the very player the cause
    /// names (a different or missing breaker means no credit at all).
    fn credit_of(&self, cause: CineCause, instigator: Option<u32>) -> (Option<u32>, DefeatKind) {
        match cause {
            CineCause::Melee => (instigator, DefeatKind::Melee),
            CineCause::ShoreBlock { player } => (
                self.shore_breakers.contains(&player).then_some(player),
                DefeatKind::Shore,
            ),
            CineCause::ShoreBlockEdge { player } => (
                self.shore_breakers.contains(&player).then_some(player),
                DefeatKind::Edge,
            ),
            // The cause names the player who dropped it: no `ShoreBroken`.
            CineCause::DropHazard { player } => (Some(player), DefeatKind::Shore),
            CineCause::Other => (None, DefeatKind::Melee),
        }
    }

    fn prune(&mut self, now: u64) {
        let window = self.tuning.history_window_ticks;
        let keep = |tick: u64| now.saturating_sub(tick) <= window;
        self.defeats.retain(|d| keep(d.tick));
        self.launches.retain(|l| keep(l.tick));
        self.contacts.retain(|c| keep(c.tick));
        let clean = self.tuning.clean_window_ticks;
        self.hurts.retain(|&(t, _)| now.saturating_sub(t) <= clean);
        let cap = self.tuning.max_history_entries;
        for len in [
            &mut self.defeats as &mut dyn Trim,
            &mut self.hurts,
            &mut self.launches,
            &mut self.contacts,
        ] {
            len.trim_to(cap);
        }
    }

    /// Score `player`'s chain ending at `tick`. `None` without a primary feat
    /// or below the ordinary threshold. Bonuses never qualify on their own.
    fn score_player(
        &self,
        tick: u64,
        player: u32,
        target: u32,
        include_rewarded: bool,
    ) -> Option<CinematicMoment> {
        let t = &self.tuning;
        let mine: Vec<&DefeatEntry> = self
            .defeats
            .iter()
            .filter(|d| d.credit == Some(player))
            .filter(|d| include_rewarded || self.rewarded_through.is_none_or(|r| d.tick > r))
            .collect();
        let has = |kind: DefeatKind| mine.iter().any(|d| d.kind == kind);
        let precision_edge = has(DefeatKind::Edge);
        let mixed_chain = has(DefeatKind::Melee) && (has(DefeatKind::Shore) || precision_edge);
        let primary = match (precision_edge, mixed_chain) {
            (false, false) => return None,
            (true, false) => t.precision_edge_score,
            (false, true) => t.mixed_chain_score,
            (true, true) => t.precision_edge_score.max(t.mixed_chain_score),
        };
        let base_targets = if mixed_chain { 2 } else { 1 };
        let extra_kills = u8::try_from(mine.len().saturating_sub(base_targets)).unwrap_or(u8::MAX);
        let extra_bonus = extra_kills
            .saturating_mul(t.extra_kill_bonus)
            .min(t.extra_kill_bonus_cap);
        let contact_setup =
            t.contact_setup_enabled && mine.iter().any(|d| self.contact_links(player, d));
        let score = primary
            .saturating_add(extra_bonus)
            .saturating_add(if contact_setup {
                t.contact_setup_bonus
            } else {
                0
            });
        let tier = if score >= t.legendary_threshold {
            CineTier::Legendary
        } else if score >= t.ordinary_threshold {
            CineTier::Ordinary
        } else {
            return None;
        };
        let credited_slot = mine
            .iter()
            .filter(|d| d.tick == tick)
            .min_by_key(|d| d.target)
            .and_then(|d| d.slot);
        Some(CinematicMoment {
            tick,
            tier,
            score,
            credited: player,
            credited_slot,
            target,
            feats: FeatSet {
                precision_edge,
                mixed_chain,
                contact_setup,
                extra_kills,
            },
            upgrade: false,
        })
    }

    /// Review F4 proposal: `defeat` is of the `source` or the `target` of a
    /// recorded contact, no earlier than the contact and within the history
    /// window of it, and the contact's `source` was launched by `player` (the
    /// launch no later than the contact). The proof of the launch is the
    /// adapter's `LaunchCredited` bridge (`Hit { PushKick, instance ==
    /// launch, target == source }` on `main`).
    fn contact_links(&self, player: u32, defeat: &DefeatEntry) -> bool {
        self.contacts.iter().any(|c| {
            (c.target == defeat.target || c.source == defeat.target)
                && c.tick <= defeat.tick
                && defeat.tick - c.tick <= self.tuning.history_window_ticks
                && self
                    .launches
                    .iter()
                    .any(|l| l.goblin == c.source && l.player == player && l.tick <= c.tick)
        })
    }
}

impl Default for CinematicEvaluator {
    fn default() -> Self {
        Self::new(CineTuning::DEFAULT)
    }
}

/// Tiny helper so every rolling queue is trimmed by one code path.
trait Trim {
    fn trim_to(&mut self, cap: usize);
}

impl<T> Trim for VecDeque<T> {
    fn trim_to(&mut self, cap: usize) {
        while self.len() > cap {
            self.pop_front();
        }
    }
}

// ------------------------------------------------------------- slow motion --

/// A presentation-only slow-motion request (tier, scale, duration, blends,
/// cooldown). It never feeds back into the simulation. **Non-stacking:** a
/// request that arrives while one is pending or active never extends or resets
/// it (a higher tier may upgrade a still-pending one); the cooldown
/// (`cooldown_ticks`, simulation ticks) is enforced by the evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlowMoRequest {
    pub tier: CineTier,
    pub credited: u32,
    /// Simulation tick of the lethal batch.
    pub tick: u64,
    pub scale_permille: u16,
    pub duration_ms: u32,
    pub blend_in_ms: u32,
    pub blend_out_ms: u32,
    pub camera_emphasis_px: u8,
    pub cooldown_ticks: u64,
}

/// The comfort setting that gates the presentation (never the evaluator).
///
/// Review F5: `cinematic_finishes: on/off` has its own flag and environment
/// variable. It does **not** reuse the HUD-only [`crate::hud_settings`]
/// reduced-motion flag (HUD scope; world motion is deferred to HUD-DEBT-08,
/// which must also disable CINE once it exists).
///
/// Environment: `FLOORBREAK_CINEMATIC_FINISHES` (`1`/`true`/`on`/`yes`,
/// `0`/`false`/`off`/`no`). Flag: `--cinematic-finishes[=on|off]` (the bare
/// form means `on`). Layering: default, then environment, then flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentationSettings {
    /// `cinematic_finishes: on/off`. Off removes time dilation and camera
    /// emphasis only: hit confirmation, death VFX and outcomes are unchanged.
    pub cinematic_finishes: bool,
}

pub const ENV_CINEMATIC_FINISHES: &str = "FLOORBREAK_CINEMATIC_FINISHES";
/// The command line flag of [`PresentationSettings`].
pub const FLAG_CINEMATIC_FINISHES: &str = "--cinematic-finishes";

impl PresentationSettings {
    /// What headless fixtures use unless they run a CINE-01 visual probe.
    pub const OFF: PresentationSettings = PresentationSettings {
        cinematic_finishes: false,
    };

    /// Interactive default: on.
    pub const DEFAULT: PresentationSettings = PresentationSettings {
        cinematic_finishes: true,
    };

    pub const fn allows_slow_motion(&self) -> bool {
        self.cinematic_finishes
    }

    /// Defaults overridden by whatever `get` returns (an env lookup in the
    /// game, a map in tests). Unparsable values are ignored.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Self {
        let mut s = Self::DEFAULT;
        if let Some(v) = get(ENV_CINEMATIC_FINISHES).and_then(|v| parse_switch(&v)) {
            s.cinematic_finishes = v;
        }
        s
    }

    pub fn from_env() -> Self {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Apply one CLI argument. `Ok(true)` when it was ours, `Ok(false)` when
    /// not, `Err` on a bad `=value`.
    pub fn apply_flag(&mut self, arg: &str) -> Result<bool, String> {
        if arg == FLAG_CINEMATIC_FINISHES {
            self.cinematic_finishes = true;
            return Ok(true);
        }
        if let Some(value) = arg
            .strip_prefix(FLAG_CINEMATIC_FINISHES)
            .and_then(|rest| rest.strip_prefix('='))
        {
            self.cinematic_finishes = parse_switch(value).ok_or_else(|| {
                format!("bad value {value:?} for {FLAG_CINEMATIC_FINISHES} (expected on or off)")
            })?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl Default for PresentationSettings {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// What the planner did with a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestOutcome {
    /// Queued until hit-stop completes.
    Queued,
    /// A higher tier replaced the still-pending one.
    UpgradedPending,
    /// A request is already pending (same or lower tier) or active:
    /// non-stacking, no extension, no reset.
    IgnoredBusy,
    /// Disabled: nothing will slow down.
    Suppressed,
}

/// The planner's phase. The spec's `Cooldown` lives in the evaluator, in
/// simulation ticks (see [`Rejection::Cooldown`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlannerState {
    Idle,
    Pending,
    Active,
}

/// Why a moment left the planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanEnd {
    /// Ran its full duration.
    Completed,
    /// Pause: cancelled, back to 1.0x before the pause menu (never resumed).
    CancelledPause,
    /// Restart, defeat, respawn, capture or scene exit.
    CancelledLocked,
    /// The setting was switched off meanwhile.
    CancelledSettings,
    /// No defeat sequence was visible when it would have started: skipped
    /// rather than slowing empty scenery.
    SkippedNoFinish,
}

/// One frame of context for [`SlowMoPlanner::step`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameContext {
    /// Unscaled real seconds of the frame that just elapsed.
    pub real_dt: f64,
    /// Ticks of hit-stop still pending for the lethal contact.
    pub hit_stop_left: u8,
    /// The death presenter still has an active finish.
    pub finish_active: bool,
    /// The D6 disconnect pause (`pads.is_paused()`; the game has no pause
    /// menu yet, review F9): the fixed step waits while the virtual clock
    /// keeps running, so the moment must be cancelled, not left to expire.
    pub paused: bool,
    /// Defeat, restart, respawn, `Frozen` capture, scene exit: any externally
    /// locked time flow.
    pub locked: bool,
}

/// The planner's output for the **next** frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FramePlan {
    /// The relative speed to set on the virtual clock (1.0 when not slowed).
    pub relative_speed: f32,
    /// Optional camera emphasis in virtual pixels (0 unless active).
    pub camera_emphasis_px: u8,
    pub state: PlannerState,
    /// Set on the frame a moment ended, with the reason.
    pub ended: Option<PlanEnd>,
}

/// Presentation clock of a cinematic moment: `Idle -> Pending -> Active ->
/// Idle`, driven by **unscaled real** time. It owns no simulation state and
/// the adapter owns the actual Bevy clock. Call [`SlowMoPlanner::step`] once
/// per frame with the real delta of the frame that just elapsed and apply the
/// returned speed to the next one.
#[derive(Debug, Clone)]
pub struct SlowMoPlanner {
    settings: PresentationSettings,
    state: Slot,
}

#[derive(Debug, Clone, Copy)]
enum Slot {
    Idle,
    Pending(SlowMoRequest),
    Active { req: SlowMoRequest, elapsed: f64 },
}

impl SlowMoPlanner {
    pub const fn new(settings: PresentationSettings) -> Self {
        Self {
            settings,
            state: Slot::Idle,
        }
    }

    pub fn settings(&self) -> PresentationSettings {
        self.settings
    }

    pub fn set_settings(&mut self, settings: PresentationSettings) {
        self.settings = settings;
    }

    pub fn state(&self) -> PlannerState {
        match self.state {
            Slot::Idle => PlannerState::Idle,
            Slot::Pending(_) => PlannerState::Pending,
            Slot::Active { .. } => PlannerState::Active,
        }
    }

    pub fn request(&mut self, req: SlowMoRequest) -> RequestOutcome {
        if !self.settings.allows_slow_motion() {
            return RequestOutcome::Suppressed;
        }
        match self.state {
            Slot::Idle => {
                self.state = Slot::Pending(req);
                RequestOutcome::Queued
            }
            Slot::Pending(old) if req.tier > old.tier => {
                self.state = Slot::Pending(req);
                RequestOutcome::UpgradedPending
            }
            Slot::Pending(_) | Slot::Active { .. } => RequestOutcome::IgnoredBusy,
        }
    }

    pub fn step(&mut self, ctx: FrameContext) -> FramePlan {
        let cancel = if ctx.paused {
            Some(PlanEnd::CancelledPause)
        } else if ctx.locked {
            Some(PlanEnd::CancelledLocked)
        } else if !self.settings.allows_slow_motion() {
            Some(PlanEnd::CancelledSettings)
        } else {
            None
        };
        if let Some(reason) = cancel {
            let was_busy = !matches!(self.state, Slot::Idle);
            self.state = Slot::Idle;
            return plan(1.0, 0, PlannerState::Idle, was_busy.then_some(reason));
        }
        match self.state {
            Slot::Idle => plan(1.0, 0, PlannerState::Idle, None),
            Slot::Pending(req) => {
                if ctx.hit_stop_left > 0 {
                    return plan(1.0, 0, PlannerState::Pending, None);
                }
                if !ctx.finish_active {
                    self.state = Slot::Idle;
                    return plan(1.0, 0, PlannerState::Idle, Some(PlanEnd::SkippedNoFinish));
                }
                self.state = Slot::Active { req, elapsed: 0.0 };
                plan(
                    envelope(&req, 0.0),
                    req.camera_emphasis_px,
                    PlannerState::Active,
                    None,
                )
            }
            Slot::Active { req, elapsed } => {
                let elapsed = elapsed + ctx.real_dt.max(0.0);
                if elapsed * 1000.0 >= f64::from(req.duration_ms) {
                    self.state = Slot::Idle;
                    return plan(1.0, 0, PlannerState::Idle, Some(PlanEnd::Completed));
                }
                self.state = Slot::Active { req, elapsed };
                plan(
                    envelope(&req, elapsed),
                    req.camera_emphasis_px,
                    PlannerState::Active,
                    None,
                )
            }
        }
    }
}

fn plan(speed: f64, px: u8, state: PlannerState, ended: Option<PlanEnd>) -> FramePlan {
    FramePlan {
        relative_speed: speed as f32,
        camera_emphasis_px: px,
        state,
        ended,
    }
}

/// The speed at `elapsed` real seconds into an active moment: linear ramp
/// 1.0 -> scale over `blend_in_ms`, hold, linear ramp back over `blend_out_ms`
/// at the end. The whole curve fits `duration_ms` (the blends are included in
/// it and shrink if the duration is shorter than both).
pub fn envelope_speed(req: &SlowMoRequest, elapsed_s: f64) -> f32 {
    envelope(req, elapsed_s) as f32
}

fn envelope(req: &SlowMoRequest, elapsed_s: f64) -> f64 {
    let scale = f64::from(req.scale_permille) / 1000.0;
    let total = f64::from(req.duration_ms);
    let t = (elapsed_s * 1000.0).clamp(0.0, total);
    let blend_in = f64::from(req.blend_in_ms).min(total);
    let blend_out = f64::from(req.blend_out_ms).min(total - blend_in);
    if blend_in > 0.0 && t < blend_in {
        1.0 + (scale - 1.0) * (t / blend_in)
    } else if blend_out > 0.0 && t > total - blend_out {
        scale + (1.0 - scale) * ((t - (total - blend_out)) / blend_out)
    } else {
        scale
    }
}

// --------------------------------------------------------------- edge latch --

/// A press-edge buffer for one action of one player, matching the current
/// `Controls` latch: `Update` calls [`EdgeLatch::press`] for every frame edge,
/// the fixed step calls [`EdgeLatch::take`] once per tick. When slow motion
/// runs zero fixed ticks in a frame the edge simply stays latched until the
/// next tick consumes it, so it is never dropped and never repeated.
///
/// With `cap == 1` (the legacy latch) two presses inside one tick interval
/// collapse into one; the collapsed press is counted in
/// [`EdgeLatch::coalesced`] rather than lost silently. A larger cap keeps a
/// second press for the next tick.
#[derive(Debug, Clone)]
pub struct EdgeLatch {
    cap: u8,
    pending: u8,
    coalesced: u32,
}

impl EdgeLatch {
    pub const fn new(cap: u8) -> Self {
        Self {
            cap: if cap == 0 { 1 } else { cap },
            pending: 0,
            coalesced: 0,
        }
    }

    pub fn press(&mut self) {
        if self.pending >= self.cap {
            self.coalesced = self.coalesced.saturating_add(1);
        } else {
            self.pending += 1;
        }
    }

    /// Consume one edge for this fixed tick.
    pub fn take(&mut self) -> bool {
        if self.pending == 0 {
            return false;
        }
        self.pending -= 1;
        true
    }

    pub const fn pending(&self) -> u8 {
        self.pending
    }

    pub const fn coalesced(&self) -> u32 {
        self.coalesced
    }

    /// Drop everything pending (a pause or restart consumes edges, the same
    /// as the scene's `fixed_step` does today).
    pub fn clear(&mut self) {
        self.pending = 0;
    }
}

// -------------------------------------------------------------------- tests --

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::ForceTier;
    use crate::party::HeroId;

    const T: CineTuning = CineTuning::DEFAULT;

    fn melee(tick: u64, target: u32, by: Option<u32>) -> CineEvent {
        CineEvent::Defeated {
            tick,
            target,
            cause: CineCause::Melee,
            instigator: by,
            slot: None,
        }
    }

    fn block(tick: u64, target: u32, player: u32) -> CineEvent {
        CineEvent::Defeated {
            tick,
            target,
            cause: CineCause::ShoreBlock { player },
            instigator: None,
            slot: None,
        }
    }

    fn edge(tick: u64, target: u32, player: u32) -> CineEvent {
        CineEvent::Defeated {
            tick,
            target,
            cause: CineCause::ShoreBlockEdge { player },
            instigator: None,
            slot: None,
        }
    }

    fn broken(tick: u64, player: u32) -> CineEvent {
        CineEvent::ShoreBroken { tick, player }
    }

    fn hurt(tick: u64, player: u32) -> CineEvent {
        CineEvent::HeroHurt { tick, player }
    }

    /// Feed one batch per tick; return the moments with their tick.
    fn run(eval: &mut CinematicEvaluator, events: &[CineEvent]) -> Vec<CinematicMoment> {
        let mut ticks: Vec<u64> = events.iter().map(CineEvent::tick).collect();
        ticks.sort_unstable();
        ticks.dedup();
        ticks
            .into_iter()
            .filter_map(|t| {
                let batch: Vec<CineEvent> =
                    events.iter().copied().filter(|e| e.tick() == t).collect();
                eval.observe_tick(t, &batch, false)
            })
            .collect()
    }

    fn fresh() -> CinematicEvaluator {
        CinematicEvaluator::new(T)
    }

    // ---- CINE-T01 / T02: no candidate without a primary feat ------------

    #[test]
    fn t01_an_ordinary_melee_kill_never_qualifies_whatever_the_force() {
        let mut ev = fresh();
        assert!(run(&mut ev, &[melee(10, 1, Some(0))]).is_empty());
        assert_eq!(ev.last_rejection(), Some(Rejection::NoQualifyingFeat));
        // Force and gore never reach the evaluator: every force tier of the
        // same defeat maps to the very same input.
        let inputs: Vec<_> = [
            ForceTier::Routine,
            ForceTier::Heavy,
            ForceTier::Catastrophic,
        ]
        .map(|force| {
            CineEvent::from_combat(&CombatEvent::EnemyDefeated {
                tick: 10,
                target: 1,
                cause: DefeatCause::Melee,
                force,
                instigator: Some(0),
            })
        })
        .into();
        assert!(inputs.windows(2).all(|w| w[0] == w[1]));
        assert_eq!(inputs[0], Some(melee(10, 1, Some(0))));
    }

    #[test]
    fn t02_corpses_and_catastrophe_without_a_primary_feat_never_qualify() {
        let mut ev = fresh();
        // Three routine melee kills on one tick, all credited: no primary.
        let kills = [
            melee(5, 1, Some(0)),
            melee(5, 2, Some(0)),
            melee(5, 3, Some(0)),
        ];
        assert!(ev.observe_tick(5, &kills, false).is_none());
        // Unattributed block defeats (no ShoreBroken): not even credited.
        let mut ev = fresh();
        let blocks = [block(7, 1, 0), block(7, 2, 0), block(7, 3, 0)];
        assert!(ev.observe_tick(7, &blocks, false).is_none());
        assert_eq!(ev.last_rejection(), Some(Rejection::NoCreditedDefeat));
        // Several block kills by the verified breaker, still no melee half
        // and no edge: bonuses never qualify on their own.
        let mut ev = fresh();
        let mut batch = vec![broken(7, 0)];
        batch.extend([block(7, 1, 0), block(7, 2, 0), block(7, 3, 0)]);
        assert!(ev.observe_tick(7, &batch, false).is_none());
        assert_eq!(ev.last_rejection(), Some(Rejection::NoQualifyingFeat));
        // Other / unknown cause is never credited.
        let mut ev = fresh();
        let other = CineEvent::Defeated {
            tick: 3,
            target: 1,
            cause: CineCause::Other,
            instigator: Some(0),
            slot: None,
        };
        assert!(ev.observe_tick(3, &[other], false).is_none());
        assert_eq!(ev.last_rejection(), Some(Rejection::NoCreditedDefeat));
    }

    // ---- CINE-T03 / T04: the mixed chain and its windows ---------------

    #[test]
    fn t03_a_clean_melee_plus_block_chain_is_one_ordinary_moment_of_score_5() {
        let mut ev = fresh();
        let moments = run(
            &mut ev,
            &[melee(100, 1, Some(0)), broken(120, 0), block(150, 2, 0)],
        );
        assert_eq!(moments.len(), 1, "one moment, not one per kill");
        let m = moments[0];
        assert_eq!(
            (m.tier, m.score, m.credited, m.target, m.tick),
            (CineTier::Ordinary, 5, 0, 2, 150)
        );
        assert!(m.feats.mixed_chain && !m.feats.precision_edge && !m.feats.contact_setup);
        assert!(!m.upgrade);
    }

    #[test]
    fn t03_the_melee_kill_may_follow_the_block_kill() {
        let mut ev = fresh();
        let moments = run(
            &mut ev,
            &[broken(10, 0), block(20, 2, 0), melee(70, 1, Some(0))],
        );
        assert_eq!(moments.len(), 1);
        assert_eq!(moments[0].tick, 70);
    }

    #[test]
    fn t04_the_chain_outside_the_window_or_after_a_hurt_does_not_qualify() {
        // Window: the melee kill is 91 ticks before the block kill.
        let mut ev = fresh();
        let late = run(
            &mut ev,
            &[melee(100, 1, Some(0)), broken(150, 0), block(191, 2, 0)],
        );
        assert!(late.is_empty());
        // Boundary: exactly 90 ticks apart still counts.
        let mut ev = fresh();
        let edge_of_window = run(
            &mut ev,
            &[melee(100, 1, Some(0)), broken(150, 0), block(190, 2, 0)],
        );
        assert_eq!(edge_of_window.len(), 1);
        // The credited hero hurt inside the clean window (even on the tick).
        let mut ev = fresh();
        let hurt_mid = run(
            &mut ev,
            &[
                melee(100, 1, Some(0)),
                hurt(130, 0),
                broken(140, 0),
                block(150, 2, 0),
            ],
        );
        assert!(hurt_mid.is_empty());
        assert_eq!(ev.last_rejection(), Some(Rejection::NotClean));
        let mut ev = fresh();
        assert!(
            ev.observe_tick(
                150,
                &[
                    hurt(150, 0),
                    block(150, 2, 0),
                    broken(150, 0),
                    melee(150, 1, Some(0))
                ],
                false
            )
            .is_none()
        );
        // A hurt older than the clean window does not veto.
        let mut ev = fresh();
        let old_hurt = run(
            &mut ev,
            &[
                hurt(50, 0),
                melee(150, 1, Some(0)),
                broken(150, 0),
                block(150, 2, 0),
            ],
        );
        assert_eq!(old_hurt.len(), 1);
    }

    // ---- CINE-T05: DEATH-D edge shear (evaluator side) ------------------

    #[test]
    fn t05_a_credited_edge_shear_with_a_verified_break_is_ordinary() {
        let mut ev = fresh();
        let moments = run(&mut ev, &[broken(10, 0), edge(40, 3, 0)]);
        assert_eq!(moments.len(), 1);
        let m = moments[0];
        assert_eq!((m.tier, m.score), (CineTier::Ordinary, 5));
        assert!(m.feats.precision_edge && !m.feats.mixed_chain);
        // Without the verified break the shear is not credited.
        let mut ev = fresh();
        assert!(run(&mut ev, &[edge(40, 3, 0)]).is_empty());
        // A break by another player does not verify it.
        let mut ev = fresh();
        assert!(run(&mut ev, &[broken(10, 1), edge(40, 3, 0)]).is_empty());
        // A plain crush by the same breaker is not a shear.
        let mut ev = fresh();
        assert!(run(&mut ev, &[broken(10, 0), block(40, 3, 0)]).is_empty());
    }

    #[test]
    fn r3_primaries_do_not_stack_one_primary_plus_bonuses() {
        // A melee kill, then a landing that shears one goblin and crushes
        // another: it satisfies both primaries, and still scores one primary
        // (5) plus the extra-kill bonuses, never 10 and never legendary.
        let mut ev = fresh();
        let m = run(
            &mut ev,
            &[
                melee(10, 1, Some(0)),
                broken(20, 0),
                edge(40, 2, 0),
                block(40, 3, 0),
            ],
        )
        .pop()
        .unwrap();
        assert!(m.feats.precision_edge && m.feats.mixed_chain);
        assert_eq!((m.tier, m.score), (CineTier::Ordinary, 6));
        // Even with every extra-kill slot used it stays under legendary.
        let mut ev = fresh();
        let m = run(
            &mut ev,
            &[
                melee(10, 1, Some(0)),
                broken(20, 0),
                edge(40, 2, 0),
                block(40, 3, 0),
                block(40, 4, 0),
                block(40, 5, 0),
            ],
        )
        .pop()
        .unwrap();
        assert_eq!(m.score, 5 + T.extra_kill_bonus_cap);
        assert!(m.score < T.legendary_threshold);
    }

    #[test]
    fn r6_the_cooldown_starts_at_emission_whatever_presentation_does() {
        // The evaluator emits and starts the cooldown; nothing the planner does
        // (queue, skip, cancel, suppress) is an input.
        let mut ev = fresh();
        let first = run(
            &mut ev,
            &[melee(1, 1, Some(0)), broken(2, 0), block(3, 2, 0)],
        );
        assert_eq!(first.len(), 1);
        let again = run(
            &mut ev,
            &[melee(300, 3, Some(0)), broken(301, 0), block(302, 4, 0)],
        );
        assert!(again.is_empty());
        assert_eq!(ev.last_rejection(), Some(Rejection::Cooldown));
    }

    // ---- CINE-T06: legendary -------------------------------------------

    fn legendary_chain() -> Vec<CineEvent> {
        vec![
            CineEvent::LaunchCredited {
                tick: 100,
                goblin: 9,
                player: 0,
            },
            CineEvent::GoblinContact {
                tick: 110,
                source: 9,
                target: 2,
            },
            melee(120, 1, Some(0)),
            broken(125, 0),
            block(150, 2, 0),
            melee(150, 3, Some(0)),
        ]
    }

    /// The contact bonus switched off (the pre-launch-proof behaviour).
    const UNLINKED: CineTuning = CineTuning {
        contact_setup_enabled: false,
        ..CineTuning::DEFAULT
    };

    fn linked() -> CinematicEvaluator {
        CinematicEvaluator::new(CineTuning::DEFAULT)
    }

    #[test]
    fn f4_legendary_is_unreachable_without_the_contact_setup_bonus() {
        // Bonus off: even a fully proven launch + contact + kills chain
        // scores at most 5 + 2 extra-kill bonus = 7: ordinary.
        let mut ev = CinematicEvaluator::new(UNLINKED);
        let mut chain = legendary_chain();
        chain.extend([
            melee(150, 4, Some(0)),
            melee(150, 5, Some(0)),
            melee(150, 6, Some(0)),
        ]);
        let m = run(&mut ev, &chain).pop().unwrap();
        assert_eq!((m.tier, m.score), (CineTier::Ordinary, 7));
        assert!(!m.feats.contact_setup);
        // No evaluator input can exceed 7 without the contact bonus.
        let max_without = T.mixed_chain_score + T.extra_kill_bonus_cap;
        assert!(max_without < T.legendary_threshold);
    }

    #[test]
    fn t06_mixed_chain_plus_proven_contact_plus_a_third_kill_is_legendary_8() {
        let mut ev = linked();
        let moments = run(&mut ev, &legendary_chain());
        let m = moments.last().copied().unwrap();
        assert_eq!((m.tier, m.score), (CineTier::Legendary, 8));
        assert_eq!(
            m.feats,
            FeatSet {
                precision_edge: false,
                mixed_chain: true,
                contact_setup: true,
                extra_kills: 1
            }
        );
    }

    #[test]
    fn t06_the_contact_bonus_needs_a_proven_launch_by_the_same_player() {
        // No LaunchCredited event (the launch is not proven).
        let mut chain = legendary_chain();
        chain.remove(0);
        let mut ev = linked();
        let m = run(&mut ev, &chain).pop().unwrap();
        assert_eq!((m.tier, m.score), (CineTier::Ordinary, 6));
        // The launch is credited to another player.
        let mut chain = legendary_chain();
        chain[0] = CineEvent::LaunchCredited {
            tick: 100,
            goblin: 9,
            player: 1,
        };
        let mut ev = linked();
        assert_eq!(run(&mut ev, &chain).pop().unwrap().score, 6);
        // The contact hit another goblin than the one defeated.
        let mut chain = legendary_chain();
        chain[1] = CineEvent::GoblinContact {
            tick: 110,
            source: 9,
            target: 77,
        };
        let mut ev = linked();
        assert_eq!(run(&mut ev, &chain).pop().unwrap().score, 6);
        // The reviewer's linkage also accepts the launched goblin itself
        // being defeated, but only within the window of the contact.
        let src = [
            CineEvent::LaunchCredited {
                tick: 100,
                goblin: 9,
                player: 0,
            },
            CineEvent::GoblinContact {
                tick: 110,
                source: 9,
                target: 77,
            },
            melee(120, 9, Some(0)),
            broken(125, 0),
            block(150, 2, 0),
        ];
        let mut ev = linked();
        assert!(run(&mut ev, &src).pop().unwrap().feats.contact_setup);
        let mut stale = src;
        stale[2] = melee(205, 9, Some(0));
        stale[4] = block(206, 2, 0);
        stale[3] = broken(204, 0);
        let mut ev = linked();
        let m = run(&mut ev, &stale).pop();
        assert!(m.is_none() || !m.unwrap().feats.contact_setup);
    }

    #[test]
    fn extra_kill_bonus_is_capped_and_contact_bonus_alone_never_qualifies() {
        let mut chain = legendary_chain();
        chain.extend([
            melee(150, 4, Some(0)),
            melee(150, 5, Some(0)),
            melee(150, 6, Some(0)),
        ]);
        let mut ev = linked();
        let m = run(&mut ev, &chain).pop().unwrap();
        // 5 + 2 (contact) + min(4 extras, cap 2) = 9.
        assert_eq!(
            (m.tier, m.score, m.feats.extra_kills),
            (CineTier::Legendary, 9, 4)
        );
        // Contact + extras without a primary feat: no moment.
        let mut ev = linked();
        let no_primary = [
            CineEvent::LaunchCredited {
                tick: 1,
                goblin: 9,
                player: 0,
            },
            CineEvent::GoblinContact {
                tick: 2,
                source: 9,
                target: 2,
            },
            melee(5, 1, Some(0)),
            melee(5, 2, Some(0)),
            melee(5, 3, Some(0)),
            melee(5, 4, Some(0)),
        ];
        assert!(run(&mut ev, &no_primary).is_empty());
    }

    // ---- attribution ----------------------------------------------------

    #[test]
    fn attribution_fails_closed_and_credits_the_right_slot() {
        // Melee without an instigator: no credit, so no chain.
        let mut ev = fresh();
        assert!(
            run(
                &mut ev,
                &[melee(10, 1, None), broken(20, 0), block(30, 2, 0)]
            )
            .is_empty()
        );
        // The block's credit is the cause's player, not an older instigator.
        let mut ev = fresh();
        let m = run(
            &mut ev,
            &[melee(10, 1, Some(0)), broken(20, 1), block(30, 2, 1)],
        );
        assert!(
            m.is_empty(),
            "P0 melee + P1 block are two players: no chain"
        );
        // Mismatched breaker and cause player: the block is not credited.
        let mut ev = fresh();
        assert!(
            run(
                &mut ev,
                &[melee(10, 1, Some(0)), broken(20, 0), block(30, 2, 1)]
            )
            .is_empty()
        );
        // A Donut (player 1) chain credits player 1.
        let mut ev = fresh();
        let m = run(
            &mut ev,
            &[melee(10, 1, Some(1)), broken(20, 1), block(30, 2, 1)],
        );
        assert_eq!(m[0].credited, 1);
    }

    #[test]
    fn from_combat_maps_the_events_the_evaluator_needs_and_ignores_the_rest() {
        use CombatEvent as C;
        assert_eq!(
            CineEvent::from_combat(&C::EnemyDefeated {
                tick: 4,
                target: 8,
                cause: DefeatCause::ShoreBlock { player: 1 },
                force: ForceTier::Catastrophic,
                instigator: Some(0),
            }),
            Some(CineEvent::Defeated {
                tick: 4,
                target: 8,
                cause: CineCause::ShoreBlock { player: 1 },
                instigator: Some(0),
                slot: None
            })
        );
        assert_eq!(
            CineEvent::from_combat(&C::HeroHurt {
                tick: 2,
                source: 1,
                health_left: 1,
                hero: HeroId::Donut
            }),
            Some(CineEvent::HeroHurt { tick: 2, player: 1 })
        );
        assert_eq!(
            CineEvent::from_combat(&C::ShoreBroken { tick: 3, player: 0 }),
            Some(broken(3, 0))
        );
        assert_eq!(
            CineEvent::from_combat(&C::GoblinContact {
                tick: 5,
                launch: 1,
                source: 2,
                target: 3
            }),
            Some(CineEvent::GoblinContact {
                tick: 5,
                source: 2,
                target: 3
            })
        );
        assert_eq!(CineEvent::from_combat(&C::ShoreLanded { tick: 9 }), None);
        assert_eq!(
            CineEvent::from_combat(&C::GoblinSwing { tick: 9, goblin: 1 }),
            None
        );
    }

    // ---- CINE-T07: cooldown, rewarded chains ---------------------------

    #[test]
    fn t07_a_repeat_inside_the_cooldown_is_rejected_and_one_after_it_is_not() {
        let mut ev = fresh();
        let mut evs = vec![melee(10, 1, Some(0)), broken(20, 0), block(30, 2, 0)];
        // The same chain again 500 ticks later, inside the 900-tick cooldown.
        evs.extend([melee(530, 11, Some(0)), broken(540, 0), block(550, 12, 0)]);
        let first = run(&mut ev, &evs);
        assert_eq!(first.len(), 1);
        assert_eq!(ev.last_rejection(), Some(Rejection::Cooldown));
        // After the cooldown (30 + 900) the next chain is eligible again.
        let again = run(
            &mut ev,
            &[melee(940, 21, Some(0)), broken(950, 0), block(960, 22, 0)],
        );
        assert_eq!(again.len(), 1);
        assert_eq!(again[0].tick, 960);
        // Exactly one tick short of the cooldown is still rejected.
        let mut ev = fresh();
        let evs = [
            melee(10, 1, Some(0)),
            broken(20, 0),
            block(30, 2, 0),
            melee(900, 11, Some(0)),
            broken(905, 0),
            block(929, 12, 0),
        ];
        assert_eq!(run(&mut ev, &evs).len(), 1);
    }

    #[test]
    fn a_rewarded_chain_is_not_reused_once_the_cooldown_is_short() {
        let tuning = CineTuning {
            cooldown_ticks: 10,
            ..T
        };
        let mut ev = CinematicEvaluator::new(tuning);
        let first = run(
            &mut ev,
            &[melee(10, 1, Some(0)), broken(20, 0), block(30, 2, 0)],
        );
        assert_eq!(first.len(), 1);
        // A new lone block kill at tick 60 would pair with the old melee kill
        // (tick 10, inside 90), but that kill was already rewarded.
        let again = run(&mut ev, &[block(60, 3, 0)]);
        assert!(again.is_empty());
    }

    // ---- CINE-T08: determinism ------------------------------------------

    #[test]
    fn t08_same_tick_choice_is_deterministic_and_credit_is_not_repeated() {
        let batch = vec![
            melee(150, 7, Some(1)),
            block(150, 8, 1),
            broken(150, 1),
            melee(150, 3, Some(0)),
            block(150, 4, 0),
            broken(150, 0),
        ];
        let mut a = fresh();
        let expect = a.observe_tick(150, &batch, false).unwrap();
        // Both players tie at score 5: the lowest player wins, with its
        // lowest target in the batch.
        assert_eq!((expect.credited, expect.target, expect.score), (0, 3, 5));
        // Every rotation and the reverse give the identical moment.
        for shift in 0..batch.len() {
            let mut rotated = batch.clone();
            rotated.rotate_left(shift);
            for view in [rotated.clone(), rotated.iter().rev().copied().collect()] {
                let mut e = fresh();
                assert_eq!(e.observe_tick(150, &view, false), Some(expect));
            }
        }
        // A repeated event is not credited twice (still one chain, 5).
        let mut dup = batch.clone();
        dup.extend(batch.clone());
        let mut e = fresh();
        assert_eq!(e.observe_tick(150, &dup, false), Some(expect));
        // A higher score beats a lower player id.
        let mut e = fresh();
        let mut richer = batch.clone();
        richer.push(melee(150, 9, Some(1)));
        let m = e.observe_tick(150, &richer, false).unwrap();
        assert_eq!((m.credited, m.score), (1, 6));
    }

    #[test]
    fn a_repeated_defeat_event_in_a_later_tick_gives_no_new_credit() {
        let mut ev = fresh();
        let evs = [
            melee(10, 1, Some(0)),
            melee(20, 1, Some(0)),
            broken(25, 0),
            block(30, 2, 0),
        ];
        let m = run(&mut ev, &evs).pop().unwrap();
        assert_eq!(m.feats.extra_kills, 0);
    }

    #[test]
    fn events_of_another_tick_are_ignored_and_old_batches_are_out_of_order() {
        let mut ev = fresh();
        assert!(
            ev.observe_tick(10, &[melee(9, 1, Some(0))], false)
                .is_none()
        );
        assert_eq!(ev.last_rejection(), Some(Rejection::NoCreditedDefeat));
        assert!(ev.observe_tick(5, &[melee(5, 1, Some(0))], false).is_none());
        assert_eq!(ev.last_rejection(), Some(Rejection::OutOfOrder));
    }

    // ---- CINE-T10: shared 2P -------------------------------------------

    #[test]
    fn t10_p2_hurt_does_not_veto_p1_and_p1_hurt_does_not_veto_p2() {
        let mut ev = fresh();
        let m = run(
            &mut ev,
            &[
                hurt(100, 1),
                melee(110, 1, Some(0)),
                broken(120, 0),
                block(130, 2, 0),
            ],
        );
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].credited, 0);
        let mut ev = fresh();
        let m = run(
            &mut ev,
            &[
                hurt(100, 0),
                melee(110, 1, Some(1)),
                broken(120, 1),
                block(130, 2, 1),
            ],
        );
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].credited, 1);
        // P2 attacking meanwhile does not steal or dilute P1's credit.
        let mut ev = fresh();
        let m = run(
            &mut ev,
            &[
                melee(110, 1, Some(0)),
                melee(115, 9, Some(1)),
                broken(120, 0),
                block(130, 2, 0),
            ],
        );
        assert_eq!((m[0].credited, m[0].feats.extra_kills), (0, 0));
    }

    #[test]
    fn two_players_in_one_chain_do_not_combine_credit() {
        let mut ev = fresh();
        let m = run(
            &mut ev,
            &[melee(10, 1, Some(0)), broken(20, 1), block(30, 2, 1)],
        );
        assert!(m.is_empty());
    }

    // ---- locks, pause, restart, respawn ---------------------------------

    #[test]
    fn a_locked_batch_records_but_emits_nothing_and_starts_no_cooldown() {
        let mut ev = fresh();
        let batch = [melee(30, 1, Some(0)), broken(30, 0), block(30, 2, 0)];
        assert!(ev.observe_tick(30, &batch, true).is_none());
        assert_eq!(ev.last_rejection(), Some(Rejection::Locked));
        // The same chain completed on the next unlocked tick still qualifies
        // (no cooldown was consumed).
        let m = ev.observe_tick(31, &[melee(31, 3, Some(0))], false);
        assert!(m.is_some());
    }

    #[test]
    fn a_party_defeat_locks_until_restart_and_restart_resets_everything() {
        let mut ev = fresh();
        let lock = [CineEvent::HeroDefeated { tick: 40 }, melee(40, 1, Some(0))];
        assert!(ev.observe_tick(40, &lock, false).is_none());
        assert!(
            ev.observe_tick(41, &[broken(41, 0), block(41, 2, 0)], false)
                .is_none()
        );
        assert_eq!(ev.last_rejection(), Some(Rejection::Locked));
        // Restart: ticks begin again at 1, state is fresh, cooldown cleared.
        assert!(
            ev.observe_tick(41, &[CineEvent::Restart { tick: 41 }], false)
                .is_none()
        );
        assert_eq!(ev.held_entries(), 0);
        let m = run(
            &mut ev,
            &[melee(1, 1, Some(0)), broken(2, 0), block(3, 2, 0)],
        );
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn restart_and_respawn_both_keep_the_cooldown_on_the_monotonic_clock() {
        let mut ev = fresh();
        let first = run(
            &mut ev,
            &[melee(1, 1, Some(0)), broken(2, 0), block(3, 2, 0)],
        );
        assert_eq!(first.len(), 1);
        // Respawn drops history and the lock, not the cooldown.
        assert!(
            ev.observe_tick(5, &[CineEvent::Respawn { tick: 5 }], false)
                .is_none()
        );
        assert_eq!(ev.held_entries(), 0);
        let blocked = run(
            &mut ev,
            &[melee(6, 5, Some(0)), broken(7, 0), block(8, 6, 0)],
        );
        assert!(blocked.is_empty());
        assert_eq!(ev.last_rejection(), Some(Rejection::Cooldown));
        // Restart clears the history but the cooldown survives: the finished
        // attempt ended on tick 9, so the new attempt's tick 1 is clock 10.
        assert!(
            ev.observe_tick(9, &[CineEvent::Restart { tick: 9 }], false)
                .is_none()
        );
        let again = run(
            &mut ev,
            &[melee(1, 1, Some(0)), broken(2, 0), block(3, 2, 0)],
        );
        assert!(again.is_empty());
        assert_eq!(ev.last_rejection(), Some(Rejection::Cooldown));
        // Emitted at clock 3; eligible again at clock 903 = epoch 9 + tick 894.
        let early = run(
            &mut ev,
            &[melee(893, 1, Some(0)), broken(893, 0), block(893, 2, 0)],
        );
        assert!(early.is_empty(), "clock 902 is still inside the cooldown");
        let late = run(
            &mut ev,
            &[melee(894, 3, Some(0)), broken(894, 0), block(894, 4, 0)],
        );
        assert_eq!(late.len(), 1, "clock 903: the 900 ticks have passed");
    }

    // ---- tier thresholds and upgrade -----------------------------------

    #[test]
    fn a_higher_tier_may_upgrade_a_moment_only_inside_the_upgrade_window() {
        let mut ev = linked();
        let base = [
            CineEvent::LaunchCredited {
                tick: 1,
                goblin: 9,
                player: 0,
            },
            CineEvent::GoblinContact {
                tick: 2,
                source: 9,
                target: 2,
            },
            melee(10, 1, Some(0)),
            broken(11, 0),
            block(12, 2, 0),
        ];
        let first = run(&mut ev, &base);
        assert_eq!(first[0].tier, CineTier::Ordinary);
        assert_eq!(first[0].score, 7);
        // One more kill 4 ticks later makes it 8: upgrades the same moment.
        let up = ev
            .observe_tick(16, &[melee(16, 3, Some(0))], false)
            .unwrap();
        assert!(up.upgrade);
        assert_eq!((up.tier, up.score), (CineTier::Legendary, 8));
        // Cooldown still runs from the first emission (12 + 900).
        let mut ev = linked();
        run(&mut ev, &base);
        assert!(
            ev.observe_tick(22, &[melee(22, 3, Some(0))], false)
                .is_none()
        );
        // Past the upgrade window the rewarded chain is spent (and the
        // cooldown would reject any new one anyway).
        assert_eq!(ev.last_rejection(), Some(Rejection::NoQualifyingFeat));
        // Same tier never re-emits (no extra-kill bonus: still ordinary).
        let mut ev = CinematicEvaluator::new(CineTuning {
            extra_kill_bonus: 0,
            ..CineTuning::DEFAULT
        });
        run(&mut ev, &base);
        assert!(
            ev.observe_tick(14, &[melee(14, 4, Some(0))], false)
                .is_none()
        );
    }

    #[test]
    fn the_tuning_table_moves_the_thresholds() {
        let strict = CineTuning {
            ordinary_threshold: 6,
            ..T
        };
        let mut ev = CinematicEvaluator::new(strict);
        assert!(
            run(
                &mut ev,
                &[melee(1, 1, Some(0)), broken(2, 0), block(3, 2, 0)]
            )
            .is_empty()
        );
    }

    // ---- CINE-T13: bounded memory --------------------------------------

    #[test]
    fn t13_the_history_is_bounded_under_a_stress_of_simultaneous_defeats() {
        let mut ev = fresh();
        for tick in 1..=2000u64 {
            let batch: Vec<CineEvent> = (0..40u32)
                .map(|i| melee(tick, tick as u32 * 100 + i, Some(0)))
                .chain([hurt(tick, 1)])
                .collect();
            let _ = ev.observe_tick(tick, &batch, false);
            assert!(ev.held_entries() <= 4 * T.max_history_entries);
        }
        // No ordinary-kill stress ever qualified.
        assert!(ev.held_entries() > 0);
    }

    // ---- request mapping, planner, envelope -----------------------------

    fn moment(tier: CineTier) -> CinematicMoment {
        CinematicMoment {
            tick: 150,
            tier,
            score: if tier == CineTier::Legendary { 8 } else { 5 },
            credited: 0,
            credited_slot: None,
            target: 2,
            feats: FeatSet::default(),
            upgrade: false,
        }
    }

    fn ctx(real_dt: f64) -> FrameContext {
        FrameContext {
            real_dt,
            hit_stop_left: 0,
            finish_active: true,
            paused: false,
            locked: false,
        }
    }

    #[test]
    fn the_request_carries_tier_duration_cooldown_from_the_one_table() {
        let o = T.request_for(&moment(CineTier::Ordinary));
        assert_eq!(
            (
                o.scale_permille,
                o.duration_ms,
                o.blend_in_ms,
                o.blend_out_ms,
                o.cooldown_ticks
            ),
            (350, 480, 35, 90, 900)
        );
        let l = T.request_for(&moment(CineTier::Legendary));
        assert_eq!((l.scale_permille, l.duration_ms), (250, 650));
        assert_eq!(l.camera_emphasis_px, 2);
        let tuned = CineTuning {
            ordinary_duration_ms: 300,
            cooldown_ticks: 60,
            ..T
        };
        let r = tuned.request_for(&moment(CineTier::Ordinary));
        assert_eq!((r.duration_ms, r.cooldown_ticks), (300, 60));
    }

    #[test]
    fn the_envelope_ramps_in_holds_and_ramps_out_inside_the_duration() {
        let r = T.request_for(&moment(CineTier::Ordinary));
        assert_eq!(envelope_speed(&r, 0.0), 1.0);
        // Halfway through the 35 ms entrance: (1 + 0.35) / 2.
        assert!((envelope_speed(&r, 0.0175) - 0.675).abs() < 1e-6);
        assert!((envelope_speed(&r, 0.035) - 0.35).abs() < 1e-6);
        assert!((envelope_speed(&r, 0.2) - 0.35).abs() < 1e-6);
        // Exit starts at 480 - 90 = 390 ms and returns to 1.0 at 480 ms.
        assert!((envelope_speed(&r, 0.390) - 0.35).abs() < 1e-6);
        assert!((envelope_speed(&r, 0.435) - 0.675).abs() < 1e-6);
        assert!((envelope_speed(&r, 0.480) - 1.0).abs() < 1e-6);
        // Never discontinuous and never outside [scale, 1.0].
        let mut prev = envelope_speed(&r, 0.0);
        for i in 1..=480 {
            let s = envelope_speed(&r, f64::from(i) / 1000.0);
            assert!((0.35 - 1e-6..=1.0 + 1e-6).contains(&s));
            assert!((s - prev).abs() <= 0.65 / 35.0 + 1e-6);
            prev = s;
        }
    }

    #[test]
    fn the_planner_waits_for_hit_stop_then_runs_the_duration_in_real_time() {
        let mut p = SlowMoPlanner::new(PresentationSettings::DEFAULT);
        let req = T.request_for(&moment(CineTier::Ordinary));
        assert_eq!(p.request(req), RequestOutcome::Queued);
        // Hit-stop still running: pending, normal speed, no double freeze.
        let mut waiting = ctx(1.0 / 60.0);
        waiting.hit_stop_left = 3;
        for _ in 0..3 {
            let f = p.step(waiting);
            assert_eq!((f.state, f.relative_speed), (PlannerState::Pending, 1.0));
        }
        // Hit-stop done: starts, runs 480 ms of *real* time and ends.
        let mut frames = 0;
        let ended = loop {
            let f = p.step(ctx(1.0 / 60.0));
            frames += 1;
            if let Some(e) = f.ended {
                break Some((e, f.relative_speed, f.camera_emphasis_px));
            }
            assert_eq!(f.state, PlannerState::Active);
            assert!(frames < 100);
        };
        assert_eq!(ended, Some((PlanEnd::Completed, 1.0, 0)));
        // 1 start frame + 29 frames of 16.67 ms reach 480 ms (30 steps).
        assert_eq!(frames, 30);
        assert_eq!(p.state(), PlannerState::Idle);
    }

    #[test]
    fn the_planner_is_non_stacking_and_upgrades_only_while_pending() {
        let mut p = SlowMoPlanner::new(PresentationSettings::DEFAULT);
        let ord = T.request_for(&moment(CineTier::Ordinary));
        let leg = T.request_for(&moment(CineTier::Legendary));
        assert_eq!(p.request(ord), RequestOutcome::Queued);
        assert_eq!(p.request(ord), RequestOutcome::IgnoredBusy);
        assert_eq!(p.request(leg), RequestOutcome::UpgradedPending);
        assert_eq!(p.request(leg), RequestOutcome::IgnoredBusy);
        p.step(ctx(0.0));
        assert_eq!(p.state(), PlannerState::Active);
        // Active: nothing extends, resets or upgrades it.
        assert_eq!(p.request(leg), RequestOutcome::IgnoredBusy);
        let f = p.step(ctx(0.1));
        assert!(
            (f.relative_speed - 0.25).abs() < 1e-6,
            "legendary scale kept"
        );
        // It still ends on its own original clock (650 ms): 100 ms elapsed.
        let f = p.step(ctx(0.6));
        assert_eq!(f.ended, Some(PlanEnd::Completed));
    }

    #[test]
    fn a_pending_moment_with_no_visible_finish_is_skipped() {
        let mut p = SlowMoPlanner::new(PresentationSettings::DEFAULT);
        p.request(T.request_for(&moment(CineTier::Ordinary)));
        let mut c = ctx(1.0 / 60.0);
        c.finish_active = false;
        let f = p.step(c);
        assert_eq!(f.ended, Some(PlanEnd::SkippedNoFinish));
        assert_eq!((f.state, f.relative_speed), (PlannerState::Idle, 1.0));
    }

    // ---- reduced motion / settings / pause ------------------------------

    #[test]
    fn t11_off_suppresses_every_request_and_camera_emphasis() {
        for settings in [PresentationSettings::OFF] {
            let mut p = SlowMoPlanner::new(settings);
            let req = T.request_for(&moment(CineTier::Legendary));
            assert_eq!(p.request(req), RequestOutcome::Suppressed);
            for _ in 0..60 {
                let f = p.step(ctx(1.0 / 60.0));
                assert_eq!((f.relative_speed, f.camera_emphasis_px), (1.0, 0));
                assert_eq!(f.state, PlannerState::Idle);
            }
        }
        // The evaluator is unaffected by presentation settings: moments are
        // still produced (and replay-identical) with presentation off.
        let mut ev = fresh();
        assert_eq!(
            run(
                &mut ev,
                &[melee(1, 1, Some(0)), broken(2, 0), block(3, 2, 0)]
            )
            .len(),
            1
        );
    }

    #[test]
    fn t11_switching_a_setting_mid_moment_restores_normal_speed_at_once() {
        let mut p = SlowMoPlanner::new(PresentationSettings::DEFAULT);
        p.request(T.request_for(&moment(CineTier::Ordinary)));
        p.step(ctx(0.0));
        let f = p.step(ctx(0.1));
        assert!(f.relative_speed < 1.0 && f.camera_emphasis_px == 2);
        p.set_settings(PresentationSettings::OFF);
        let f = p.step(ctx(0.016));
        assert_eq!(
            (f.relative_speed, f.camera_emphasis_px, f.ended),
            (1.0, 0, Some(PlanEnd::CancelledSettings))
        );
        assert_eq!(p.state(), PlannerState::Idle);
    }

    #[test]
    fn t11_pause_cancels_and_never_resumes_the_moment() {
        let mut p = SlowMoPlanner::new(PresentationSettings::DEFAULT);
        p.request(T.request_for(&moment(CineTier::Ordinary)));
        p.step(ctx(0.0));
        p.step(ctx(0.1));
        let mut paused = ctx(0.5);
        paused.paused = true;
        let f = p.step(paused);
        assert_eq!(
            (f.relative_speed, f.state, f.ended),
            (1.0, PlannerState::Idle, Some(PlanEnd::CancelledPause))
        );
        // Resume: nothing restarts, and real time spent paused did not tick
        // an elapsed counter that survived.
        for _ in 0..40 {
            let f = p.step(ctx(1.0 / 60.0));
            assert_eq!((f.relative_speed, f.state), (1.0, PlannerState::Idle));
            assert_eq!(f.ended, None);
        }
        // A request made while paused (a defeat on the pause frame) is
        // dropped by the pause too.
        p.request(T.request_for(&moment(CineTier::Ordinary)));
        let f = p.step(paused);
        assert_eq!(f.ended, Some(PlanEnd::CancelledPause));
    }

    #[test]
    fn t11_restart_respawn_and_exit_locks_cancel_and_idle_cancels_report_nothing() {
        let mut p = SlowMoPlanner::new(PresentationSettings::DEFAULT);
        let mut locked = ctx(0.016);
        locked.locked = true;
        assert_eq!(p.step(locked).ended, None);
        p.request(T.request_for(&moment(CineTier::Ordinary)));
        let f = p.step(locked);
        assert_eq!(
            (f.relative_speed, f.ended),
            (1.0, Some(PlanEnd::CancelledLocked))
        );
    }

    #[test]
    fn t11_a_hit_stop_overlap_never_adds_a_second_freeze() {
        // Hit-stop is simulation-side; the planner only waits and never
        // outputs a speed below 1.0 until it has finished.
        let mut p = SlowMoPlanner::new(PresentationSettings::DEFAULT);
        p.request(T.request_for(&moment(CineTier::Ordinary)));
        for left in (1..=8u8).rev() {
            let mut c = ctx(0.016);
            c.hit_stop_left = left;
            assert_eq!(p.step(c).relative_speed, 1.0);
        }
    }

    // ---- review fixes F1, F3, F5, F6, F10 --------------------------------

    #[test]
    fn f1_credit_is_the_hero_index_and_a_swap_does_not_chain_across_heroes() {
        // Solo P1 pounces a kill as Donut (hero 1), then the block they broke
        // as Carl (hero 0) lands: two heroes, no chain.
        let mut ev = fresh();
        let events = [melee(10, 1, Some(1)), broken(20, 0), block(30, 2, 0)];
        assert!(run(&mut ev, &events).is_empty());
        // The slot owning the credited hero travels to the moment.
        let mut ev = fresh();
        let owned = [
            melee(10, 1, Some(1)).with_slot(Some(0)),
            broken(20, 1),
            block(30, 2, 1).with_slot(Some(0)),
        ];
        let m = run(&mut ev, &owned).pop().unwrap();
        assert_eq!((m.credited, m.credited_slot), (1, Some(0)));
        // Hurt of Carl does not veto Donut's chain, a Donut hurt does.
        let mut ev = fresh();
        let m = run(
            &mut ev,
            &[
                hurt(5, 0),
                melee(10, 1, Some(1)),
                broken(20, 1),
                block(30, 2, 1),
            ],
        );
        assert_eq!(m.len(), 1);
        let mut ev = fresh();
        let m = run(
            &mut ev,
            &[
                hurt(5, 1),
                melee(10, 1, Some(1)),
                broken(20, 1),
                block(30, 2, 1),
            ],
        );
        assert!(m.is_empty());
        // `with_slot` only touches defeats.
        assert_eq!(hurt(1, 0).with_slot(Some(1)), hurt(1, 0));
    }

    #[test]
    fn f2_a_block_nobody_broke_never_credits_the_default_player() {
        // `step_shore` credits P1 when no breaker is recorded; the evaluator
        // cannot see that, so it needs the ShoreBroken of the same attempt.
        let mut ev = fresh();
        assert!(run(&mut ev, &[melee(10, 1, Some(0)), block(30, 2, 0)]).is_empty());
        assert_eq!(ev.last_rejection(), Some(Rejection::NoCreditedDefeat));
        // The break may be older than the 90-tick window (the fall is slow).
        let mut ev = fresh();
        let m = run(
            &mut ev,
            &[broken(1, 0), melee(400, 1, Some(0)), block(410, 2, 0)],
        );
        assert_eq!(m.len(), 1);
        // A break in a previous attempt does not carry over a restart.
        let mut ev = fresh();
        run(&mut ev, &[broken(1, 0)]);
        ev.observe_tick(50, &[CineEvent::Restart { tick: 50 }], false);
        assert!(run(&mut ev, &[melee(2, 1, Some(0)), block(3, 2, 0)]).is_empty());
    }

    #[test]
    fn f3_restart_resets_history_and_dedup_so_reused_ids_count_again() {
        let mut ev = fresh();
        let first = run(
            &mut ev,
            &[melee(10, 1, Some(0)), broken(20, 0), block(30, 2, 0)],
        );
        assert_eq!(first.len(), 1);
        ev.observe_tick(31, &[CineEvent::Restart { tick: 31 }], false);
        // Ticks begin at 1 again and target ids 1 and 2 are reused: the same
        // chain at the same attempt-local ticks is a fresh one (it qualifies,
        // so it is rejected only by the surviving cooldown, never as a
        // duplicate or as a stale-history underflow).
        let second = run(
            &mut ev,
            &[melee(1, 1, Some(0)), broken(2, 0), block(3, 2, 0)],
        );
        assert!(second.is_empty());
        assert_eq!(ev.last_rejection(), Some(Rejection::Cooldown));
        // Once the cooldown has passed the reused ids score again.
        let third = run(
            &mut ev,
            &[melee(900, 1, Some(0)), broken(901, 0), block(902, 2, 0)],
        );
        assert_eq!(third.len(), 1, "{:?}", ev.last_rejection());
        assert_eq!(third[0].tick, 902);
    }

    #[test]
    fn f5_the_cinematic_settings_are_separate_from_the_hud_ones() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |key: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| (*v).to_string())
            }
        };
        // The HUD flag has no effect on the cinematic setting.
        let hud_only = PresentationSettings::from_lookup(env(&[(
            crate::hud_settings::ENV_REDUCED_MOTION,
            "1",
        )]));
        assert_eq!(hud_only, PresentationSettings::DEFAULT);
        let mut s = PresentationSettings::from_lookup(env(&[(ENV_CINEMATIC_FINISHES, "off")]));
        assert_eq!(s, PresentationSettings::OFF);
        // An unparsable value keeps the default.
        let kept = PresentationSettings::from_lookup(env(&[(ENV_CINEMATIC_FINISHES, "maybe")]));
        assert_eq!(kept, PresentationSettings::DEFAULT);
        // Flags layer over the environment.
        assert_eq!(s.apply_flag("--cinematic-finishes"), Ok(true));
        assert_eq!(s, PresentationSettings::DEFAULT);
        assert_eq!(s.apply_flag("--cinematic-finishes=off"), Ok(true));
        assert!(!s.allows_slow_motion());
        // There is no cinematic reduced-motion flag (review F5): HUD-DEBT-08.
        assert_eq!(s.apply_flag("--cinematic-reduced-motion=on"), Ok(false));
        assert_eq!(s.apply_flag("--hud-reduced-motion"), Ok(false));
        assert!(s.apply_flag("--cinematic-finishes=bogus").is_err());
    }

    #[test]
    fn f10_hit_stop_starting_during_the_moment_is_accepted_and_does_not_cancel_it() {
        let mut p = SlowMoPlanner::new(PresentationSettings::DEFAULT);
        p.request(T.request_for(&moment(CineTier::Ordinary)));
        p.step(ctx(0.0));
        let mut c = ctx(0.05);
        c.hit_stop_left = 8;
        let f = p.step(c);
        assert_eq!(f.state, PlannerState::Active);
        assert!((f.relative_speed - 0.35).abs() < 1e-6);
    }

    // ---- time-scale + input buffering under a simulated frame loop ------

    /// Mimics Bevy's fixed step: virtual time = real * speed, a fixed 60 Hz
    /// accumulator, taps latched in `Update` and taken once per tick.
    struct Loop {
        planner: SlowMoPlanner,
        speed: f32,
        overstep: f64,
        ticks: u64,
        max_ticks_in_frame: u32,
    }

    impl Loop {
        const STEP: f64 = 1.0 / 60.0;

        fn new(planner: SlowMoPlanner) -> Self {
            Self {
                planner,
                speed: 1.0,
                overstep: 0.0,
                ticks: 0,
                max_ticks_in_frame: 0,
            }
        }

        /// One frame: latch presses, advance fixed ticks, plan the next
        /// frame. Returns the number of fixed ticks and the consumed edges.
        fn frame(&mut self, real_dt: f64, latch: &mut EdgeLatch, presses: u32) -> (u32, u32) {
            for _ in 0..presses {
                latch.press();
            }
            self.overstep += real_dt * f64::from(self.speed);
            let mut ticks = 0;
            let mut taken = 0;
            while self.overstep >= Self::STEP {
                self.overstep -= Self::STEP;
                ticks += 1;
                self.ticks += 1;
                if latch.take() {
                    taken += 1;
                }
            }
            self.max_ticks_in_frame = self.max_ticks_in_frame.max(ticks);
            self.speed = self.planner.step(ctx(real_dt)).relative_speed;
            (ticks, taken)
        }
    }

    #[test]
    fn slow_motion_runs_fewer_ticks_but_never_more_than_one_per_normal_frame() {
        let mut lp = Loop::new(SlowMoPlanner::new(PresentationSettings::DEFAULT));
        let mut latch = EdgeLatch::new(1);
        // 1 s of normal play at 60 fps.
        for _ in 0..60 {
            lp.frame(1.0 / 60.0, &mut latch, 0);
        }
        assert!((59..=61).contains(&lp.ticks));
        let base = lp.ticks;
        lp.planner
            .request(T.request_for(&moment(CineTier::Ordinary)));
        // 480 ms moment + margin = 60 frames.
        for _ in 0..60 {
            lp.frame(1.0 / 60.0, &mut latch, 0);
        }
        let during = lp.ticks - base;
        // 480 ms at ~0.35x plus the rest at 1.0x is far fewer than 60.
        assert!(during < 50 && during > 25, "ticks {during}");
        assert!(lp.max_ticks_in_frame <= 1, "no catch-up burst");
        assert_eq!(lp.planner.state(), PlannerState::Idle);
        assert_eq!(lp.speed, 1.0);
    }

    #[test]
    fn every_press_is_consumed_exactly_once_at_slow_speed() {
        let mut lp = Loop::new(SlowMoPlanner::new(PresentationSettings::DEFAULT));
        let mut latch = EdgeLatch::new(T.edge_buffer_cap);
        lp.planner
            .request(T.request_for(&moment(CineTier::Legendary)));
        let (mut pressed, mut consumed) = (0u32, 0u32);
        // A press every 5 frames (83 ms, slower than a 0.25x tick of 66 ms
        // would swallow): over the whole moment and after.
        for frame in 0..120 {
            let press = u32::from(frame % 5 == 0);
            pressed += press;
            let (_, taken) = lp.frame(1.0 / 60.0, &mut latch, press);
            consumed += taken;
        }
        // Drain whatever is still latched at the end.
        while latch.pending() > 0 {
            consumed += u32::from(latch.take());
        }
        assert_eq!(consumed, pressed, "no press dropped or duplicated");
        assert_eq!(latch.coalesced(), 0);
    }

    #[test]
    fn a_press_in_a_frame_with_zero_ticks_stays_latched_for_the_next_tick() {
        let mut lp = Loop::new(SlowMoPlanner::new(PresentationSettings::DEFAULT));
        lp.speed = 0.25; // 4 frames per tick
        let mut latch = EdgeLatch::new(1);
        let (t0, c0) = lp.frame(1.0 / 60.0, &mut latch, 1);
        assert_eq!((t0, c0), (0, 0), "no tick this frame, press kept");
        assert_eq!(latch.pending(), 1);
        let mut taken = 0;
        for _ in 0..8 {
            lp.speed = 0.25;
            taken += lp.frame(1.0 / 60.0, &mut latch, 0).1;
        }
        assert_eq!(taken, 1, "consumed once, not repeated");
    }

    #[test]
    fn the_latch_counts_a_collapsed_double_press_and_a_cap_of_two_keeps_it() {
        let mut one = EdgeLatch::new(1);
        one.press();
        one.press();
        assert_eq!((one.pending(), one.coalesced()), (1, 1));
        assert!(one.take());
        assert!(!one.take());
        let mut two = EdgeLatch::new(2);
        two.press();
        two.press();
        assert_eq!((two.pending(), two.coalesced()), (2, 0));
        assert!(two.take() && two.take() && !two.take());
        // Cap 0 is treated as the legacy latch, and clear drops the edge.
        let mut zero = EdgeLatch::new(0);
        zero.press();
        zero.clear();
        assert!(!zero.take());
    }

    #[test]
    fn p1_and_p2_latches_are_independent_while_the_world_is_shared() {
        let mut lp = Loop::new(SlowMoPlanner::new(PresentationSettings::DEFAULT));
        let mut p1 = EdgeLatch::new(1);
        let mut p2 = EdgeLatch::new(1);
        lp.planner
            .request(T.request_for(&moment(CineTier::Ordinary)));
        let (mut sent, mut got) = ([0u32; 2], [0u32; 2]);
        for frame in 0..90 {
            let a = u32::from(frame % 7 == 0);
            let b = u32::from(frame % 11 == 0);
            sent[0] += a;
            sent[1] += b;
            // One shared virtual clock drives one tick for both latches.
            for _ in 0..a {
                p1.press();
            }
            for _ in 0..b {
                p2.press();
            }
            lp.overstep += (1.0 / 60.0) * f64::from(lp.speed);
            while lp.overstep >= Loop::STEP {
                lp.overstep -= Loop::STEP;
                got[0] += u32::from(p1.take());
                got[1] += u32::from(p2.take());
            }
            lp.speed = lp.planner.step(ctx(1.0 / 60.0)).relative_speed;
        }
        got[0] += u32::from(p1.take());
        got[1] += u32::from(p2.take());
        assert_eq!(got, sent);
    }
}
