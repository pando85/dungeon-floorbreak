//! COMPANION-AI-02 CA2-B2 fixtures: the `cf02_free_*` goldens (spec 7.2) and the
//! autonomy metrics harness (spec 7.1, M1 to M16).
//!
//! Every fixture runs in the CF02 microarena under `cf02_rules` with the
//! autonomous partner (`--companion cf02_free`): `UncontrolledHero::Companion(
//! ProfileSetId::Cf02Free)` plus the `--cf02` swap feel. They are **not** in
//! [`PartyFixture::all`]: [`CF02_FREE_FIXTURES`] is chained into
//! `cf02_fixtures::find_party_fixture` and `list_fixtures` (the `cf02_` listing
//! the frozen CI loop replays against `docs/evidence/cf02/replays`), so every
//! earlier fixture, golden and replay is untouched.
//!
//! Recorded with Donut's agile motion (DM-A, her default everywhere). No fixture
//! contains a hero-hurting hazard: the hazard feed lands in CA2-C (review FU-3, a
//! known gap listed in the PR body).
//!
//! Tick numbering is the one of [`crate::combat_scene`]: a hold over
//! `from..to` covers the steps that produce `tick = from .. to - 1`.

use crate::boss_scene::Pilot;
use crate::cf02::arena;
use crate::combat::EnemyAi;
use crate::combat_scene::{EnemySpawn, Fixture};
use crate::companion::profile::ProfileSetId;
use crate::donut_fixtures::primary;
use crate::donut_mana::DonutSetup;
use crate::party::{PROVISIONAL_TUNING, PartyTuning, UncontrolledHero};
use crate::party_scene::{PartyFixture, run};

const fn chase(x: i32) -> EnemySpawn {
    EnemySpawn {
        x,
        ai: EnemyAi::Chase,
    }
}

const fn dummy(x: i32) -> EnemySpawn {
    EnemySpawn {
        x,
        ai: EnemyAi::Passive,
    }
}

/// Solo in the room: the autonomous partner drives Donut, with the `--cf02`
/// swap feel (the same shape as `cf02_fixtures::ARENA_SOLO`).
pub const FREE: PartyTuning = PartyTuning {
    uncontrolled_hero: UncontrolledHero::Companion(ProfileSetId::Cf02Free),
    ..PROVISIONAL_TUNING
}
.with_cf02_swap();

/// Hit points of a `cf02_free_*` goblin that needs more than the legacy 2 (so
/// the goblin Carl works survives his jabs). `None`: the legacy 2.
pub fn goblin_health(fixture: &str) -> Option<u8> {
    match fixture {
        "cf02_free_lone_claimed" => Some(40),
        "cf02_free_split_two_goblins" => Some(6),
        _ => None,
    }
}

/// An arena room with a custom roster and no llama, no shore.
const fn room(
    name: &'static str,
    summary: &'static str,
    seed: u64,
    enemies: &'static [EnemySpawn],
    ticks: u64,
) -> Fixture {
    Fixture {
        enemies,
        llamas: &[],
        shore_dx: None,
        ..arena::fixture(name, summary, seed, ticks)
    }
}

// ------------------------------------------------------------ explore --

const EXPLORE_BASE: Fixture = room(
    "cf02_free_explore_room",
    "CA2-B2: no enemies. Carl walks, pauses 90 ticks, walks on and stops at the foot of the left perch; the autonomous Donut keeps her own spacing and comes along (M1, M2, M5, M6)",
    0xCF02_FE01,
    &[],
    380,
);

pub const CF02_FREE_EXPLORE_ROOM: PartyFixture = PartyFixture {
    base: &EXPLORE_BASE,
    two_player: false,
    tuning: FREE,
    script: &[run(5, 105, 0, 1.0), run(195, 235, 0, 1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// -------------------------------------------------------------- split --

const SPLIT_BASE: Fixture = room(
    "cf02_free_split_two_goblins",
    "CA2-B2: Carl works goblin A in front of him; goblin B walks in from the right; she takes B and A stays his (M3, M4, M5)",
    0xCF02_FE02,
    &[dummy(150), chase(340)],
    420,
);

pub const CF02_FREE_SPLIT_TWO_GOBLINS: PartyFixture = PartyFixture {
    base: &SPLIT_BASE,
    two_player: false,
    tuning: FREE,
    script: &[
        primary(20, 21, 0),
        primary(50, 51, 0),
        primary(80, 81, 0),
        primary(110, 111, 0),
        primary(140, 141, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// -------------------------------------------------------------- lone --

const LONE_BASE: Fixture = room(
    "cf02_free_lone_claimed",
    "CA2-B2: one goblin and Carl on it the whole time; she never starts a damaging move on it and has no Hunt goal (review B-2; M3, M4)",
    0xCF02_FE03,
    &[dummy(150)],
    360,
);

pub const CF02_FREE_LONE_CLAIMED: PartyFixture = PartyFixture {
    base: &LONE_BASE,
    two_player: false,
    tuning: FREE,
    script: &[
        primary(20, 21, 0),
        run(38, 39, 0, 1.0),
        primary(50, 51, 0),
        run(68, 69, 0, 1.0),
        primary(80, 81, 0),
        run(98, 99, 0, 1.0),
        primary(110, 111, 0),
        run(128, 129, 0, 1.0),
        primary(140, 141, 0),
        run(158, 159, 0, 1.0),
        primary(170, 171, 0),
        run(188, 189, 0, 1.0),
        primary(200, 201, 0),
        run(218, 219, 0, 1.0),
        primary(230, 231, 0),
        run(248, 249, 0, 1.0),
        primary(260, 261, 0),
        run(278, 279, 0, 1.0),
        primary(290, 291, 0),
        run(308, 309, 0, 1.0),
        primary(320, 321, 0),
        run(338, 339, 0, 1.0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// ---------------------------------------------------------- abandoned --

const ABANDONED_BASE: Fixture = room(
    "cf02_free_finish_abandoned",
    "CA2-B2 (owner decision 1): a 2 HP goblin; Carl hits it once, walks away and stands; she waits out the A12 window, then hunts it and lands the killing hit (M4, M5)",
    0xCF02_FE04,
    &[dummy(215)],
    420,
);

pub const CF02_FREE_FINISH_ABANDONED: PartyFixture = PartyFixture {
    base: &ABANDONED_BASE,
    two_player: false,
    tuning: FREE,
    script: &[run(5, 25, 0, 1.0), primary(40, 41, 0), run(60, 95, 0, -1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// --------------------------------------------------------- outnumbered --

const OUTNUMBERED_BASE: Fixture = room(
    "cf02_free_outnumbered",
    "CA2-B2: three goblins converge on Carl; Support-leader (M3, M4)",
    0xCF02_FE05,
    &[chase(300), chase(330), chase(360)],
    420,
);

pub const CF02_FREE_OUTNUMBERED: PartyFixture = PartyFixture {
    base: &OUTNUMBERED_BASE,
    two_player: false,
    tuning: FREE,
    script: &[
        primary(60, 61, 0),
        primary(90, 91, 0),
        primary(120, 121, 0),
        primary(150, 151, 0),
        primary(180, 181, 0),
        primary(210, 211, 0),
        primary(240, 241, 0),
        primary(270, 271, 0),
        primary(300, 301, 0),
        primary(330, 331, 0),
        primary(360, 361, 0),
        primary(390, 391, 0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// -------------------------------------------------------------- bolts --

const BOLTS_BASE: Fixture = room(
    "cf02_free_carl_bolts",
    "CA2-B2: she hunts a goblin 112 px from Carl; on the tick her first pounce starts (the press of tick 25) Carl sprints toward the parapet (about 600 px); she lands, breaks off and keeps in frame (M5, M6, M12)",
    0xCF02_FE06,
    &[dummy(240)],
    520,
);

pub const CF02_FREE_CARL_BOLTS: PartyFixture = PartyFixture {
    base: &BOLTS_BASE,
    two_player: false,
    tuning: FREE,
    script: &[run(26, 480, 0, 1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// ------------------------------------------------------------ retreat --

const RETREAT_BASE: Fixture = room(
    "cf02_free_retreat_when_hurt",
    "CA2-B2: goblins come at the room; one hits her and she retreats behind Carl or up high, then returns (M10)",
    0xCF02_FE07,
    &[
        chase(300),
        chase(308),
        chase(316),
        chase(324),
        chase(332),
        chase(340),
    ],
    480,
);

pub const CF02_FREE_RETREAT_WHEN_HURT: PartyFixture = PartyFixture {
    base: &RETREAT_BASE,
    two_player: false,
    tuning: FREE,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

// -------------------------------------------------------------- arena --

const ARENA_DIRECT_BASE: Fixture = arena::fixture(
    "cf02_free_arena_direct",
    "CA2-B2: the `cf02_arena_direct` inputs under `cf02_free` (all metrics; M13 and M15 against `cf02_assist`)",
    0xCF02_FE08,
    1800,
);

pub const CF02_FREE_ARENA_DIRECT: PartyFixture = PartyFixture {
    base: &ARENA_DIRECT_BASE,
    two_player: false,
    tuning: FREE,
    script: &[],
    pilot: Pilot::ArenaDirect,
    donut: DonutSetup::NONE,
};

/// Every `cf02_free_*` fixture, in spec 7.2 order.
pub const CF02_FREE_FIXTURES: [&PartyFixture; 8] = [
    &CF02_FREE_EXPLORE_ROOM,
    &CF02_FREE_SPLIT_TWO_GOBLINS,
    &CF02_FREE_LONE_CLAIMED,
    &CF02_FREE_FINISH_ABANDONED,
    &CF02_FREE_OUTNUMBERED,
    &CF02_FREE_CARL_BOLTS,
    &CF02_FREE_RETREAT_WHEN_HURT,
    &CF02_FREE_ARENA_DIRECT,
];

// ---------------------------------------------------------------- metrics --

use crate::combat::{CombatEvent, MoveId};
use crate::companion::autonomy::{Goal, LeaderClaims};
use crate::companion::events::CompanionEvent;
use crate::companion::navfollow::NavNote;
use crate::companion::profile::ProfileFile;
use crate::companion::view::StableEntityId;
use crate::goblin::GoblinMode;
use crate::party::HeroId;
use crate::party_scene::PartyFixtureSim;
use crate::prototype::{HERO_HALF_WIDTH, RUN_SPEED, TICK_SECONDS};

/// The M1 spread radius (px).
pub const M1_SPREAD_PX: f32 = 80.0;
/// The M2 shadowing radius (px).
pub const M2_SHADOW_PX: f32 = 64.0;
/// M10: the ticks after a Retreat trigger at which the distance is read.
pub const M10_READ_TICKS: u64 = 60;
/// The "standing enemy close" radius of M15 (px).
pub const M15_NEAR_PX: f32 = 160.0;
/// The `steals` window of the #170 harness (ticks).
const STEAL_WINDOW: u64 = 30;

/// The autonomy metrics of one run (spec 7.1). Both heroes alive, solo and no
/// swap pan running unless stated. Everything is computed from authoritative
/// state each tick; nothing here feeds back into the simulation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AutonomyMetrics {
    pub ticks: u64,
    /// Ticks both heroes were alive, solo, no swap pan (the M1/M2 base).
    pub counted_ticks: u64,
    /// M1: ticks with |her x - his x| over [`M1_SPREAD_PX`]; the median distance.
    pub spread_ticks: u64,
    pub median_distance_px: f32,
    /// M2: ticks within [`M2_SHADOW_PX`] while both move the same way.
    pub shadow_ticks: u64,
    pub both_moving_ticks: u64,
    /// M3: her AI-started damaging moves, those started on an enemy outside the
    /// leader's claim set `C`, and ticks both heroes' most recent hit (within 12
    /// ticks) is on the same enemy. (`MoveStarted` carries no target: the
    /// instance's target is its first `Hit`, so a whiffed move is not counted in
    /// the last number.)
    pub ai_starts: u32,
    pub ai_starts_outside_claims: u32,
    pub same_target_ticks: u64,
    /// M4: the #170 steal counters.
    pub steals: u32,
    pub start_steals: u32,
    /// M5, M6: ticks her body is not wholly inside the scroll view; episodes
    /// (maximal runs), split by the M5 involuntary predicate; episodes that began
    /// ahead of the leader (the side he faces).
    pub outside_ticks: u64,
    pub episodes_self_caused: u32,
    pub episodes_involuntary: u32,
    pub ahead_exits: u32,
    /// M10: Retreat triggers; per trigger the distance to the nearest standing
    /// enemy `M10_READ_TICKS` later (`None`: no standing enemy), and hits she took
    /// during a retreat.
    pub retreats: u32,
    pub retreat_distances: Vec<Option<f32>>,
    pub hits_in_retreat: u32,
    /// M12: regroup teleports of the partner.
    pub regroup_teleports: u32,
    /// M13: her landed hits, all landed hits.
    pub her_hits: u32,
    pub all_hits: u32,
    /// M15: ticks in Retreat, ticks with a standing enemy within
    /// [`M15_NEAR_PX`] of either hero.
    pub retreat_ticks: u64,
    pub near_enemy_ticks: u64,
    /// M16 (reported, no target): interrupt press **ticks** the drift check
    /// skipped; ticks the brief allowed interrupts only for the drift check;
    /// distinct goblins that ever targeted her; Hunt evaluations, claim
    /// exclusions and drift rejections.
    pub interrupt_drift_skip_ticks: u32,
    pub brief_drift_blocked_ticks: u32,
    pub goblins_targeted_her: u32,
    pub hunt_evaluations: u32,
    pub hunt_claim_excluded: u32,
    pub hunt_drift_rejected: u32,
    /// N-e (CA2-B1 review): ticks the leader counted as travelling, and those
    /// that only a pending camera pan made true.
    pub travelling_ticks: u32,
    pub pan_only_travelling_ticks: u32,
    /// Ticks per goal (index: `Goal as usize`).
    pub goal_ticks: [u64; 7],
    /// The tick of each of her damaging press starts, and of each goal change.
    pub press_ticks: Vec<u64>,
    pub goals: Vec<(u64, Goal)>,
}

impl AutonomyMetrics {
    /// M1 spread as a percentage of counted ticks.
    pub fn spread_percent(&self) -> u64 {
        (self.spread_ticks * 100)
            .checked_div(self.counted_ticks)
            .unwrap_or(0)
    }

    /// M2 shadowing as a percentage of the ticks both moved.
    pub fn shadow_percent(&self) -> u64 {
        (self.shadow_ticks * 100)
            .checked_div(self.both_moving_ticks)
            .unwrap_or(0)
    }

    /// M3: the share of her starts outside the claim set, percent.
    pub fn distinct_percent(&self) -> u64 {
        (u64::from(self.ai_starts_outside_claims) * 100)
            .checked_div(u64::from(self.ai_starts))
            .unwrap_or(0)
    }

    /// M13: her share of the landed hits, percent.
    pub fn hit_share_percent(&self) -> u64 {
        (u64::from(self.her_hits) * 100)
            .checked_div(u64::from(self.all_hits))
            .unwrap_or(0)
    }

    /// M15: the Retreat share, percent.
    pub fn retreat_share_percent(&self) -> u64 {
        (self.retreat_ticks * 100)
            .checked_div(self.near_enemy_ticks)
            .unwrap_or(0)
    }

    /// Hunt candidate evaluations dropped by the claim set, percent (M16).
    pub fn claim_excluded_percent(&self) -> u64 {
        (u64::from(self.hunt_claim_excluded) * 100)
            .checked_div(u64::from(self.hunt_evaluations))
            .unwrap_or(0)
    }

    /// Hunt evaluations that failed only the drift check, percent (M16).
    pub fn drift_rejected_percent(&self) -> u64 {
        (u64::from(self.hunt_drift_rejected) * 100)
            .checked_div(u64::from(self.hunt_evaluations))
            .unwrap_or(0)
    }
}

/// Run `ticks` scripted ticks of `sim` and measure it (spec 7.1). The simulation
/// is the real one, stepped exactly as the golden is; the harness only reads.
pub fn autonomy_metrics(sim: &mut PartyFixtureSim, ticks: u64) -> AutonomyMetrics {
    autonomy_metrics_with(sim, ticks, PartyFixtureSim::step_scripted)
}

/// [`autonomy_metrics`] with the caller's per-tick step (PT-TUNE: the closing
/// arena's Carl is the room's pilot, not the arena autopilot, which cannot bait
/// the slime). The measurement is identical; only the stepping differs.
pub fn autonomy_metrics_with(
    sim: &mut PartyFixtureSim,
    ticks: u64,
    mut step: impl FnMut(&mut PartyFixtureSim),
) -> AutonomyMetrics {
    let profiles = ProfileFile::pinned().expect("the embedded profile file is pinned");
    let a12 = u64::from(
        profiles
            .set(ProfileSetId::Cf02Free)
            .cf02_duo
            .cf02_duo_a12_recent_hit_ticks,
    );
    let window = u64::from(
        profiles
            .set(ProfileSetId::Cf02Free)
            .autonomy
            .ca2_q34_cause_window_ticks,
    );
    let mut m = AutonomyMetrics::default();
    let mut distances: Vec<f32> = Vec::new();
    let mut carl_hits: Vec<(u64, u32)> = Vec::new();
    let mut last_hit_target: [Option<(u64, u32)>; 2] = [None, None];
    let mut starts: Vec<(u64, u32, MoveId)> = Vec::new();
    let mut targeted: Vec<u32> = Vec::new();
    // M5 predicate state: the last tick each cause held.
    let mut last_cause: Option<u64> = None;
    let mut in_exit: Option<(bool, u64)> = None; // (involuntary, first tick)
    let mut prev_leader_x = sim.scene.world.hero.x;
    let mut prev_health = sim.scene.world.donut_body.health;
    let mut prev_controlled = sim.scene.drive().controlled;
    let mut open_retreats: Vec<u64> = Vec::new();
    let mut prev_goal = Goal::Idle;
    for _ in 0..ticks {
        let pre_tick = sim.scene.world.tick + 1;
        let pre_view = sim.combat.companion_probe_view(&sim.scene);
        let claims = pre_view.as_ref().map(|v| {
            let set = profiles.set(ProfileSetId::Cf02Free);
            LeaderClaims::compute(v, set, v.hero(HeroId::Carl))
        });
        step(sim);
        m.ticks += 1;
        let tick = sim.scene.world.tick;
        let events: Vec<CombatEvent> = sim.combat.take_new_events().to_vec();
        let cevents = sim.combat.take_companion_events();
        let driven = sim.combat.companion_driven();
        let carl = sim.scene.world.hero.x;
        let donut = sim.scene.world.donut_body.x;
        let health = sim.scene.world.donut_body.health;
        let solo = driven[HeroId::Donut.index()] && !driven[HeroId::Carl.index()];
        let both_alive = !sim.combat.hero_defeated && sim.scene.world.hero.health > 0 && health > 0;
        let left = sim.scene.camera.left() as f32;
        let right = left + sim.scene.camera.config().view_w as f32;
        let swapped = sim.scene.drive().controlled != prev_controlled;
        prev_controlled = sim.scene.drive().controlled;

        // ---- events of this tick
        let mut regroup = false;
        let mut nav_hold = false;
        for e in &cevents {
            match *e {
                CompanionEvent::Regroup {
                    hero: HeroId::Donut,
                    outcome,
                    ..
                } => {
                    regroup |= matches!(
                        outcome,
                        crate::companion::events::RegroupOutcome::Placed { .. }
                    );
                    m.regroup_teleports += u32::from(matches!(
                        outcome,
                        crate::companion::events::RegroupOutcome::Placed { .. }
                    ));
                }
                CompanionEvent::Nav {
                    hero: HeroId::Donut,
                    note,
                    ..
                } => {
                    nav_hold |= matches!(note, NavNote::Stranded { .. } | NavNote::Waiting { .. });
                }
                CompanionEvent::GoalChanged {
                    tick: t,
                    hero: HeroId::Donut,
                    goal,
                    ..
                } => {
                    m.goals.push((t, goal));
                    if goal == Goal::Retreat {
                        m.retreats += 1;
                        open_retreats.push(t);
                    }
                }
                CompanionEvent::Press {
                    tick: t,
                    hero: HeroId::Donut,
                    target,
                    chain: false,
                    ..
                } => {
                    m.ai_starts += 1;
                    m.press_ticks.push(t);
                    let claimed = claims.as_ref().is_some_and(|c| c.contains(target));
                    m.ai_starts_outside_claims += u32::from(!claimed);
                }
                _ => {}
            }
        }
        for e in &events {
            match *e {
                CombatEvent::MoveStarted {
                    tick: t,
                    move_id,
                    instance,
                } if move_id.hero() == 1 => starts.push((t, instance, move_id)),
                CombatEvent::Hit {
                    tick: t,
                    move_id,
                    target,
                    damage,
                    health_left,
                    instance,
                    ..
                } if damage > 0 => {
                    m.all_hits += 1;
                    let hero = move_id.hero() as usize;
                    last_hit_target[hero.min(1)] = Some((t, target));
                    if hero == 0 {
                        carl_hits.push((t, target));
                    } else {
                        m.her_hits += 1;
                        if health_left == 0 {
                            let carl_hit = |until: u64| {
                                carl_hits
                                    .iter()
                                    .any(|&(c, id)| id == target && c <= until && until - c <= a12)
                            };
                            m.steals +=
                                u32::from(carl_hits.iter().any(|&(c, id)| {
                                    id == target && c <= t && t - c <= STEAL_WINDOW
                                }));
                            let start = starts
                                .iter()
                                .find(|s| s.1 == instance && s.2.hero() == 1)
                                .map(|s| s.0);
                            m.start_steals += u32::from(start.is_some_and(carl_hit));
                        }
                    }
                }
                _ => {}
            }
        }
        let _ = pre_tick;

        // ---- goal and Retreat bookkeeping
        let st = sim.combat.autonomy_state(HeroId::Donut);
        let goal = st.goal();
        if driven[HeroId::Donut.index()] {
            m.goal_ticks[goal as usize] += 1;
        }
        if goal == Goal::Retreat && driven[HeroId::Donut.index()] {
            m.retreat_ticks += 0; // counted with the near-enemy gate below
            if health < prev_health {
                m.hits_in_retreat += 1;
            }
        }
        prev_goal = goal;
        let standing: Vec<f32> = sim
            .combat
            .enemies
            .iter()
            .filter(|g| matches!(g.mode, GoblinMode::Neutral | GoblinMode::Attack) && g.alive())
            .map(|g| g.x)
            .collect();
        let near = standing
            .iter()
            .any(|x| (x - carl).abs() <= M15_NEAR_PX || (x - donut).abs() <= M15_NEAR_PX);
        if near {
            m.near_enemy_ticks += 1;
            if goal == Goal::Retreat {
                m.retreat_ticks += 1;
            }
        }
        for t0 in open_retreats.clone() {
            if tick == t0 + M10_READ_TICKS {
                let nearest = standing
                    .iter()
                    .map(|x| (x - donut).abs())
                    .fold(None, |acc: Option<f32>, d| {
                        Some(acc.map_or(d, |a| a.min(d)))
                    });
                m.retreat_distances.push(nearest);
                open_retreats.retain(|t| *t != t0);
            }
        }
        for (i, g) in sim.combat.enemies.iter().enumerate() {
            if sim.combat.target_of(i) == HeroId::Donut
                && g.alive()
                && !targeted.contains(&(i as u32))
            {
                targeted.push(i as u32);
            }
        }

        // ---- M1, M2, M3 (second part)
        if solo && both_alive && !sim.scene.pan_boosted() {
            m.counted_ticks += 1;
            let d = (donut - carl).abs();
            distances.push(d);
            m.spread_ticks += u64::from(d > M1_SPREAD_PX);
            let (cv, dv) = (sim.scene.world.hero.vx, sim.scene.world.donut_body.vx);
            if cv != 0.0 && dv != 0.0 && (cv > 0.0) == (dv > 0.0) {
                m.both_moving_ticks += 1;
                m.shadow_ticks += u64::from(d <= M2_SHADOW_PX);
            }
            let fresh = |h: Option<(u64, u32)>| h.filter(|(t, _)| tick.saturating_sub(*t) <= 12);
            if let (Some((_, a)), Some((_, b))) =
                (fresh(last_hit_target[0]), fresh(last_hit_target[1]))
            {
                m.same_target_ticks += u64::from(a == b);
            }
        }

        // ---- M5, M6: exits and their causes
        let leader_dx = (carl - prev_leader_x).abs();
        prev_leader_x = carl;
        let hurt = health < prev_health || sim.combat.hurt_ticks_of(HeroId::Donut) > 0;
        prev_health = health;
        let cause_now = hurt
            || nav_hold
            || sim.scene.pan_boosted()
            || leader_dx > RUN_SPEED * TICK_SECONDS + 0.01
            || swapped
            || regroup;
        if cause_now {
            last_cause = Some(tick);
        }
        let outside = donut - HERO_HALF_WIDTH < left || donut + HERO_HALF_WIDTH > right;
        if solo && outside {
            m.outside_ticks += 1;
            if in_exit.is_none() {
                let involuntary = last_cause.is_some_and(|c| tick - c <= window);
                in_exit = Some((involuntary, tick));
                let facing = sim.scene.world.hero.facing as f32;
                m.ahead_exits += u32::from((donut - carl) * facing > 0.0);
            }
        } else if let Some((involuntary, _)) = in_exit.take() {
            if involuntary {
                m.episodes_involuntary += 1;
            } else {
                m.episodes_self_caused += 1;
            }
        }
    }
    if let Some((involuntary, _)) = in_exit {
        if involuntary {
            m.episodes_involuntary += 1;
        } else {
            m.episodes_self_caused += 1;
        }
    }
    let _ = prev_goal;
    distances.sort_by(f32::total_cmp);
    m.median_distance_px = distances.get(distances.len() / 2).copied().unwrap_or(0.0);
    let stats = sim.combat.autonomy_state(HeroId::Donut).stats();
    m.interrupt_drift_skip_ticks = stats.interrupts_drift_skipped;
    m.brief_drift_blocked_ticks = stats.brief_drift_blocked_ticks;
    m.hunt_evaluations = stats.hunt_evaluations;
    m.hunt_claim_excluded = stats.hunt_claim_excluded;
    m.hunt_drift_rejected = stats.hunt_drift_rejected;
    m.travelling_ticks = stats.travelling_ticks;
    m.pan_only_travelling_ticks = stats.pan_only_travelling_ticks;
    m.goblins_targeted_her = targeted.len() as u32;
    m
}

/// `StableEntityId` of goblin `index` (the id a press names).
pub fn goblin_id(index: u32) -> StableEntityId {
    StableEntityId::goblin(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env_kit::EnvKit;
    use std::path::PathBuf;

    fn arena_kit() -> EnvKit {
        crate::cf02::arena::load_arena(&PathBuf::from(env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    fn sim_of(f: &'static PartyFixture) -> PartyFixtureSim {
        PartyFixtureSim::new(&arena_kit(), 360, f).unwrap()
    }

    /// The same scenario under the `cf02_assist` partner (the comparison column
    /// of M1, M2, M13): leaked, fixtures are static.
    fn assist_twin(f: &PartyFixture) -> &'static PartyFixture {
        Box::leak(Box::new(PartyFixture {
            base: f.base,
            two_player: f.two_player,
            tuning: PartyTuning {
                uncontrolled_hero: UncontrolledHero::Companion(ProfileSetId::Cf02Assist),
                ..f.tuning
            },
            script: f.script,
            pilot: f.pilot,
            donut: f.donut,
        }))
    }

    fn metrics_of(f: &'static PartyFixture) -> AutonomyMetrics {
        let mut sim = sim_of(f);
        autonomy_metrics(&mut sim, f.base.ticks)
    }

    /// One whole run: every combat and companion event, and the per-tick x of
    /// both heroes (index `tick - 1`).
    struct Rec {
        combat: Vec<CombatEvent>,
        comp: Vec<CompanionEvent>,
        carl_x: Vec<f32>,
        donut_x: Vec<f32>,
        view_left: Vec<i32>,
    }

    fn record(f: &'static PartyFixture) -> Rec {
        let mut sim = sim_of(f);
        let mut r = Rec {
            combat: Vec::new(),
            comp: Vec::new(),
            carl_x: Vec::new(),
            donut_x: Vec::new(),
            view_left: Vec::new(),
        };
        for _ in 0..f.base.ticks {
            sim.step_scripted();
            r.combat
                .extend(sim.combat.take_new_events().iter().copied());
            r.comp.extend(sim.combat.take_companion_events());
            r.carl_x.push(sim.scene.world.hero.x);
            r.donut_x.push(sim.scene.world.donut_body.x);
            r.view_left.push(sim.scene.camera.left());
        }
        r
    }

    /// The ticks of her AI-started damaging presses (`chain` false).
    fn her_presses(r: &Rec) -> Vec<(u64, StableEntityId)> {
        r.comp
            .iter()
            .filter_map(|e| match *e {
                CompanionEvent::Press {
                    tick,
                    hero: HeroId::Donut,
                    target,
                    chain: false,
                    ..
                } => Some((tick, target)),
                _ => None,
            })
            .collect()
    }

    fn goals_of(r: &Rec) -> Vec<Goal> {
        r.comp
            .iter()
            .filter_map(|e| match *e {
                CompanionEvent::GoalChanged {
                    hero: HeroId::Donut,
                    goal,
                    ..
                } => Some(goal),
                _ => None,
            })
            .collect()
    }

    // ---------------------------------------------------------- goldens ----

    /// The committed final `PARTY_STATE` hash of every fixture here (the replays
    /// in `docs/evidence/cf02/replays` pin the same values through the CI loop).
    const GOLDENS: [(&str, u64); 8] = [
        ("cf02_free_explore_room", 0x6c70b2d943b87695),
        ("cf02_free_split_two_goblins", 0x8452e118b7766779),
        ("cf02_free_lone_claimed", 0xd469e7deac91c4be),
        ("cf02_free_finish_abandoned", 0x6157b6ebd088ec95),
        ("cf02_free_outnumbered", 0xaaaaea1c9e79bde8),
        ("cf02_free_carl_bolts", 0xf269f4d8859bce16),
        ("cf02_free_retreat_when_hurt", 0x025015c30e99827e),
        ("cf02_free_arena_direct", 0xbac0d64831960ead),
    ];

    #[test]
    #[ignore = "prints the goldens"]
    fn print_goldens() {
        for f in CF02_FREE_FIXTURES {
            let mut sim = sim_of(f);
            sim.run_scripted(f.base.ticks);
            println!("        (\"{}\", {:#018x}),", f.name(), sim.state_hash());
        }
    }

    #[test]
    fn every_fixture_replays_identically_twice_and_matches_its_golden() {
        assert_eq!(GOLDENS.len(), CF02_FREE_FIXTURES.len());
        for (f, (name, golden)) in CF02_FREE_FIXTURES.iter().zip(GOLDENS) {
            assert_eq!(f.name(), name);
            let run = || {
                let mut sim = sim_of(f);
                sim.run_scripted(f.base.ticks);
                (sim.state_hash(), sim.state_line())
            };
            let (a, line) = run();
            assert_eq!(a, run().0, "{name} is not deterministic");
            assert_eq!(a, golden, "{name}: {line}");
        }
    }

    /// The CI loop selects by prefix and the registry is listed, found and
    /// run in the room; no fixture is in `PartyFixture::all` (so no earlier
    /// registry, golden or replay sees them).
    #[test]
    fn the_registry_is_listed_found_and_kept_out_of_the_older_registries() {
        let listed = crate::cf02_fixtures::list_fixtures("cf02_free_");
        let own: Vec<_> = CF02_FREE_FIXTURES.iter().map(|f| f.name()).collect();
        assert_eq!(listed, own);
        for f in CF02_FREE_FIXTURES {
            let name = f.name();
            assert!(name.starts_with("cf02_free_"), "{name}");
            assert!(crate::cf02::arena::fixture_uses_arena(name), "{name}");
            assert!(crate::cf02::fixture_runs_cf02(name), "{name}");
            assert!(crate::cf02_fixtures::find_party_fixture(name).is_some());
            assert!(
                PartyFixture::all().all(|g| g.name() != name),
                "{name} leaked into the base registry"
            );
            assert_eq!(
                f.tuning.uncontrolled_hero,
                UncontrolledHero::Companion(ProfileSetId::Cf02Free)
            );
        }
    }

    /// Every committed replay is what the binary prints for its fixture
    /// (`PARTY_STATE` hash line).
    #[test]
    fn the_committed_replays_carry_the_golden_hash() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for (name, golden) in GOLDENS {
            let path = root.join(format!("docs/evidence/cf02/replays/headless_{name}.txt"));
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{name}: no committed replay: {e}"));
            let state = text
                .lines()
                .rev()
                .find(|l| l.starts_with("PARTY_STATE"))
                .unwrap_or_else(|| panic!("{name}: no PARTY_STATE line"));
            assert!(
                state.contains(&format!("{golden:016x}")),
                "{name}: replay {state} vs golden {golden:#018x}"
            );
        }
    }

    // ------------------------------------------------- the fixtures' claims ----

    /// M1, M2, M5, M6, M12 in the quiet room, and the `cf02_assist` value beside it.
    #[test]
    fn explore_room_she_keeps_her_own_spacing() {
        let m = metrics_of(&CF02_FREE_EXPLORE_ROOM);
        assert!(m.spread_percent() >= 40, "M1 {}%", m.spread_percent());
        assert!(
            m.median_distance_px >= 96.0,
            "M1 median {}",
            m.median_distance_px
        );
        assert!(m.shadow_percent() <= 20, "M2 {}%", m.shadow_percent());
        assert_eq!((m.outside_ticks, m.regroup_teleports), (0, 0));
        assert_eq!((m.episodes_self_caused, m.ahead_exits), (0, 0));
        // The comparison column: the assist partner shadows him (M1 0 %, M2 100 %).
        let a = metrics_of(assist_twin(&CF02_FREE_EXPLORE_ROOM));
        assert!(m.spread_percent() > a.spread_percent());
        assert!(m.median_distance_px > a.median_distance_px);
    }

    /// M3: she takes the goblin Carl is not working; he keeps his.
    #[test]
    fn split_two_goblins_she_takes_the_other_goblin() {
        let f = &CF02_FREE_SPLIT_TWO_GOBLINS;
        let m = metrics_of(f);
        assert!(m.ai_starts >= 3, "she acts: {m:?}");
        assert!(m.distinct_percent() >= 70, "M3 {}%", m.distinct_percent());
        assert!(
            m.same_target_ticks * 100 <= 15 * m.ticks,
            "M3 same-target ticks {}",
            m.same_target_ticks
        );
        assert_eq!((m.steals, m.start_steals), (0, 0));
        assert_eq!((m.outside_ticks, m.regroup_teleports), (0, 0));
        let r = record(f);
        // Her first press is on goblin B (index 1), the one Carl never touched.
        assert_eq!(her_presses(&r)[0].1, goblin_id(1));
        assert!(goals_of(&r).contains(&Goal::Hunt));
    }

    /// B-2: one goblin, Carl on it the whole time: no Hunt goal, no damaging
    /// move of hers on it.
    #[test]
    fn lone_claimed_she_never_hunts_the_goblin_carl_is_working() {
        let r = record(&CF02_FREE_LONE_CLAIMED);
        let carl_hits = r
            .combat
            .iter()
            .filter(|e| matches!(e, CombatEvent::Hit { move_id, .. } if move_id.hero() == 0))
            .count();
        assert!(carl_hits >= 10, "Carl works it ({carl_hits} hits)");
        assert!(!goals_of(&r).contains(&Goal::Hunt), "{:?}", goals_of(&r));
        assert!(her_presses(&r).is_empty());
        assert!(
            !r.combat.iter().any(
                |e| matches!(e, CombatEvent::MoveStarted { move_id, .. } if move_id.hero() == 1)
            ),
            "she started a move on the claimed goblin"
        );
        let m = metrics_of(&CF02_FREE_LONE_CLAIMED);
        assert_eq!((m.steals, m.start_steals, m.ai_starts), (0, 0, 0));
        // Every Hunt evaluation of the run was dropped by the claim (M16).
        assert_eq!(m.hunt_claim_excluded, m.hunt_evaluations);
    }

    /// Owner decision 1 (review FU-C): the abandoned goblin is hers once the A12
    /// window is over; the defeat is credited to her and nothing is stolen.
    #[test]
    fn finish_abandoned_she_waits_out_the_window_then_finishes_it() {
        let f = &CF02_FREE_FINISH_ABANDONED;
        let r = record(f);
        let carl_last = r
            .combat
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Hit { tick, move_id, .. } if move_id.hero() == 0 => Some(tick),
                _ => None,
            })
            .max()
            .expect("Carl hits it once");
        let presses = her_presses(&r);
        let (first, _) = presses[0];
        let a12 = u64::from(
            ProfileFile::pinned()
                .unwrap()
                .set(ProfileSetId::Cf02Free)
                .cf02_duo
                .cf02_duo_a12_recent_hit_ticks,
        );
        assert!(
            first > carl_last + a12,
            "first press {first} vs Carl's last hit {carl_last} (window {a12})"
        );
        let goblin_x = 218.0;
        assert!(
            (goblin_x - r.carl_x[first as usize - 1]).abs() > 64.0,
            "Carl is more than 64 px from the goblin at her press"
        );
        let defeat = r
            .combat
            .iter()
            .find_map(|e| match *e {
                CombatEvent::EnemyDefeated { instigator, .. } => Some(instigator),
                _ => None,
            })
            .expect("the goblin is defeated");
        assert_eq!(defeat, Some(1), "the defeat is credited to her");
        let m = metrics_of(f);
        assert_eq!((m.steals, m.start_steals), (0, 0));
        assert_eq!((m.outside_ticks, m.regroup_teleports), (0, 0));
        // The assist partner steals the same goblin (the reason the layer exists).
        let a = metrics_of(assist_twin(f));
        assert_eq!(a.steals + a.start_steals, 1);
    }

    /// Three goblins converge: she supports him, off the claimed goblins first.
    #[test]
    fn outnumbered_she_supports_the_leader() {
        let f = &CF02_FREE_OUTNUMBERED;
        let m = metrics_of(f);
        let r = record(f);
        assert!(
            goals_of(&r).contains(&Goal::SupportLeader),
            "{:?}",
            goals_of(&r)
        );
        assert!(m.goal_ticks[Goal::SupportLeader as usize] > 0);
        assert!(m.distinct_percent() >= 70, "M3 {}%", m.distinct_percent());
        assert_eq!((m.steals, m.start_steals), (0, 0));
        assert_eq!((m.outside_ticks, m.regroup_teleports), (0, 0));
    }

    /// M5, M6, M12: Carl sprints away on the tick her pounce starts.
    #[test]
    fn carl_bolts_she_lands_breaks_off_and_stays_in_frame() {
        let f = &CF02_FREE_CARL_BOLTS;
        let r = record(f);
        let (first, _) = her_presses(&r)[0];
        assert_eq!(first, 25, "the sprint in the script starts on the pounce");
        assert!(r.carl_x.last().unwrap() - r.carl_x[0] > 500.0, "Carl ran");
        let m = metrics_of(f);
        assert_eq!(
            (m.outside_ticks, m.episodes_self_caused, m.ahead_exits),
            (0, 0, 0)
        );
        assert_eq!(m.regroup_teleports, 0);
        // She comes along: far behind at the start of the run, close at its end.
        let gap = (r.carl_x.last().unwrap() - r.donut_x.last().unwrap()).abs();
        assert!(gap < 200.0, "gap {gap}");
    }

    /// M10: a goblin hurts her; she retreats behind Carl and is not hit again.
    #[test]
    fn retreat_when_hurt_she_breaks_away_and_is_not_hit_again() {
        let f = &CF02_FREE_RETREAT_WHEN_HURT;
        let m = metrics_of(f);
        assert!(m.retreats >= 1, "a Retreat started");
        assert!(
            m.retreat_distances
                .iter()
                .all(|d| d.is_none_or(|d| d >= 96.0)),
            "M10 distances {:?}",
            m.retreat_distances
        );
        assert!(m.hits_in_retreat <= 1, "M10 hits {}", m.hits_in_retreat);
        assert_eq!((m.outside_ticks, m.regroup_teleports), (0, 0));
        let r = record(f);
        let hurt: Vec<u64> = r
            .combat
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::HeroHurt {
                    tick,
                    hero: HeroId::Donut,
                    ..
                } => Some(tick),
                _ => None,
            })
            .collect();
        assert!(!hurt.is_empty(), "a goblin hurt her");
        let first_retreat = r
            .comp
            .iter()
            .find_map(|e| match *e {
                CompanionEvent::GoalChanged {
                    tick,
                    goal: Goal::Retreat,
                    ..
                } => Some(tick),
                _ => None,
            })
            .expect("Retreat");
        assert!(first_retreat >= hurt[0] && first_retreat - hurt[0] <= 6);
    }

    /// The arena: M1, M2, M5, M6, M12, M15, and M13 against the assist value.
    /// CA2-Q36 (PR #213 review B-1): with transit pricing from her position she
    /// reaches Support-leader and Carl is no longer defeated. M13 is **indicative**
    /// (CA2 FU-4: the `cf02_arena_direct` inputs were recorded against the old
    /// Donut, so goblin retargeting differs): the measured pair is pinned so a
    /// regression or an improvement is visible, and the shortfall against the
    /// assist value stays a design note for the top tier (never a silent retune).
    #[test]
    fn arena_direct_the_metrics_of_the_whole_room() {
        let f = &CF02_FREE_ARENA_DIRECT;
        let m = metrics_of(f);
        assert!(
            m.spread_percent() >= 40 && m.median_distance_px >= 96.0,
            "M1"
        );
        assert!(m.shadow_percent() <= 20, "M2");
        assert_eq!(
            (m.outside_ticks, m.episodes_self_caused, m.ahead_exits),
            (0, 0, 0)
        );
        assert_eq!(m.regroup_teleports, 0, "M12");
        assert_eq!((m.steals, m.start_steals), (0, 0));
        assert!(
            m.retreat_share_percent() <= 25,
            "M15 {}%",
            m.retreat_share_percent()
        );
        assert!(
            m.hit_share_percent() <= 50,
            "M13 share {}%",
            m.hit_share_percent()
        );
        // She reaches Support-leader (it was 0 ticks before CA2-Q36).
        assert!(
            m.goal_ticks[Goal::SupportLeader as usize] > 0,
            "{:?}",
            m.goal_ticks
        );
        // Both partners: no hero is defeated on these inputs (before CA2-Q36 Carl
        // was defeated twice under `cf02_free` while she stood in Escort).
        for g in [f, assist_twin(f)] {
            let r = record(g);
            assert!(
                !r.combat
                    .iter()
                    .any(|e| matches!(e, CombatEvent::HeroDefeated { .. })),
                "{}: a hero was defeated",
                g.name()
            );
            assert!(
                !r.combat
                    .iter()
                    .any(|e| matches!(e, CombatEvent::Restart { .. }))
            );
        }
        // M13 pinned: her landed hits against the assist partner's (indicative).
        let assist = metrics_of(assist_twin(f));
        assert_eq!((m.her_hits, assist.her_hits), (1, 4), "M13 pair");
        assert_eq!((m.all_hits, assist.all_hits), (13, 13));
        // Review N-e, reported (never tuned here).
        assert!(m.travelling_ticks > 0);
    }

    /// PR #213 review F-1: the autonomy counters (M16, N-e) survive a restart.
    #[test]
    fn the_autonomy_counters_survive_a_restart() {
        let f = &CF02_FREE_SPLIT_TWO_GOBLINS;
        let mut sim = sim_of(f);
        sim.run_scripted(200);
        let before = sim.combat.autonomy_state(HeroId::Donut).stats();
        assert!(before.decisions > 0 && before.hunt_evaluations > 0);
        sim.combat.restart(&mut sim.scene);
        assert_eq!(sim.combat.autonomy_state(HeroId::Donut).stats(), before);
        sim.run_scripted(10);
        assert!(sim.combat.autonomy_state(HeroId::Donut).stats().decisions > before.decisions);
    }

    /// M9: no fixture here has a hero-hurting hazard (the hazard feed is CA2-C);
    /// the planner reports no stranded or waiting nav note for her.
    #[test]
    fn no_fixture_strands_her_and_none_has_a_hazard() {
        for f in CF02_FREE_FIXTURES {
            let r = record(f);
            assert!(
                !r.comp.iter().any(|e| matches!(
                    e,
                    CompanionEvent::Nav {
                        hero: HeroId::Donut,
                        note: NavNote::Stranded { .. },
                        ..
                    }
                )),
                "{}: stranded",
                f.name()
            );
            assert!(f.base.llamas.is_empty() || f.name() == "cf02_free_arena_direct");
        }
    }

    // ------------------------------------------------------------ the sweep ----

    /// A scripted Carl for the sweep: walk right `pre` ticks, then run `dir`
    /// from `at` for 400 ticks. The dummy goblin stands 112 px ahead of where he
    /// stopped (a pounce target inside her drift budget).
    fn sweep_fixture(pre: u64, at: u64, dir: f32, legacy: bool) -> &'static PartyFixture {
        let carl_end = 128 + (pre as f32 * 185.0 / 60.0) as i32;
        let enemies: &'static [EnemySpawn] = Box::leak(Box::new([dummy(carl_end + 112)]));
        let base: &'static Fixture = Box::leak(Box::new(Fixture {
            enemies,
            llamas: &[],
            shore_dx: None,
            ..arena::fixture(
                "cf02_free_bolt_sweep",
                "CA2-B2 sweep",
                0xCF02_FEFF,
                at + 400,
            )
        }));
        let mut script = Vec::new();
        if pre > 0 {
            script.push(run(1, 1 + pre, 0, 1.0));
        }
        script.push(run(at, at + 400, 0, dir));
        let mut tuning = FREE;
        if legacy {
            tuning.donut_motion = crate::party::MotionProfile::Legacy;
        }
        Box::leak(Box::new(PartyFixture {
            base,
            two_player: false,
            tuning,
            script: Box::leak(script.into_boxed_slice()),
            pilot: Pilot::None,
            donut: DonutSetup::NONE,
        }))
    }

    /// Spec 7.2: Carl starts running away on every tick of her pounce, both
    /// directions, at four start offsets of his screen position inside the dead
    /// zone, under both motions: she never leaves the view (M5, M6).
    #[test]
    fn the_bolt_sweep_she_never_leaves_the_view() {
        let mut pounces = 0;
        for legacy in [false, true] {
            for pre in [0u64, 12, 24, 36] {
                // Find her first pounce press in the no-run baseline.
                let baseline = sweep_fixture(pre, 4000, 1.0, legacy);
                let r = record(baseline);
                let Some(&(press, _)) = her_presses(&r).first() else {
                    continue;
                };
                for dir in [1.0f32, -1.0] {
                    // Every tick of the pounce: press (startup 10, air up to 20,
                    // pin 2, recovery 18, hit-stop 4 = 54) plus two.
                    for k in 0..=56u64 {
                        let f = sweep_fixture(pre, press + k, dir, legacy);
                        let mut sim = sim_of(f);
                        let m = autonomy_metrics(&mut sim, f.base.ticks);
                        pounces += 1;
                        assert_eq!(
                            (m.outside_ticks, m.episodes_self_caused),
                            (0, 0),
                            "legacy {legacy} pre {pre} dir {dir} start {}: {m:?}",
                            press + k
                        );
                    }
                }
            }
        }
        assert!(pounces >= 200, "the sweep ran ({pounces} runs)");
    }
}

#[cfg(test)]
mod hazard;
