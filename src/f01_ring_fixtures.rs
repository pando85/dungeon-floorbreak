//! RING-A wire and RING-B wire: scripted traversal and fight fixtures of the two
//! Broken Ring rooms (`ring_court`, `ring_split`; plan section 15.3). Named
//! `ring_*` (KR23).
//!
//! Bevy-free and library-only (no committed replay file; the scene's headless
//! smoke and the PLAYTRACE tests cover the launched rooms). Each run is one
//! launch of the room built the way the scene builds it
//! ([`crate::f01_late::ring_start`], the story loadout: the post-Hoarder pin),
//! driven by a **move-right, jump, attack** pilot (RING-14): the shared room
//! pilot of the late slice ([`crate::f01_mastery_fixtures::pilot_with`], base
//! kit only, no dynamite, no ability) plus one route rule per room:
//!
//! * **court** (static closed shutter, KR3): a hero on the floor left of the
//!   shutter whose business lies right of it walks to the 64 px step, jumps onto
//!   it, hops the 16 px onto the walkway and drops off its far end. It never
//!   presses against the shutter and never uses it (RING-01, RING-14);
//! * **split**: the ground route by default; the `high` variant jumps onto the
//!   64 px overhead route first (RING-08: both routes rejoin, no dead end);
//! * with the roster clear, the pilot walks to the exit.
//!
//! | who | what plays |
//! | --- | --- |
//! | [`Who::CarlSolo`] | solo Carl (the live party's `cf02_assist` partner follows) |
//! | [`Who::DonutSolo`] | the swap on [`SWAP_TICK`], then Donut (solo) plays the room |
//! | [`Who::Companion`] | Carl is the pilot's, Donut the autonomous partner (`cf02_free`, KR19) |
//!
//! A hero is **stuck** when it is told to walk and its column does not change;
//! the longest such run must stay at most [`STUCK_LIMIT`] ticks (plan section
//! 10, item 5). The companion's stuck run is the ticks it is **left behind**
//! (standing still more than [`APART_PX`] from Carl while he walks on).

use std::path::Path;

use crate::boss_scene::Pilot;
use crate::combat::CombatEvent;
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::env_kit::to_world_y;
use crate::f01_late::{Loadout, ring_start};
pub use crate::f01_mastery_fixtures::STUCK_LIMIT;
use crate::f01_mastery_fixtures::{APART_PX, pilot_with, tuning as mastery_tuning};
use crate::f01_ring::{RingRoom, RingRun};
use crate::input::RawInput;
use crate::party::{HeroId, PartyTuning};
use crate::party_scene::{PartyFixture, PartyFixtureSim};
use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Rect};

/// The seeds every fixture covers (`--cf02-seed`).
pub const SEEDS: std::ops::Range<u64> = 0..9;
/// The run length (100 s of game time).
pub const RUN_TICKS: u64 = 6000;
/// The tick on which [`Who::DonutSolo`] swaps to Donut.
pub const SWAP_TICK: u64 = 30;
/// The takeoff window before a platform's left edge: a grounded hero inside it
/// that walks right jumps (both motions land on a 64 px surface from it).
const TAKEOFF_PX: f32 = 36.0;
/// A goblin whose column is this near the hero's overlaps it (review F4).
const OVERLAP_PX: f32 = 8.0;

/// Who plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Who {
    CarlSolo,
    DonutSolo,
    Companion,
}

impl Who {
    pub const ALL: [Who; 3] = [Who::CarlSolo, Who::DonutSolo, Who::Companion];

    pub const fn name(self) -> &'static str {
        match self {
            Self::CarlSolo => "carl_solo",
            Self::DonutSolo => "donut_solo",
            Self::Companion => "companion_cf02_free",
        }
    }

    fn tuning(self) -> PartyTuning {
        mastery_tuning(match self {
            Self::Companion => crate::f01_mastery_fixtures::Who::Companion,
            _ => crate::f01_mastery_fixtures::Who::CarlSolo,
        })
    }
}

/// The route of one run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Route {
    /// The floor's feet line (world y, y up).
    floor_feet: f32,
    /// The closed shutter's x span, if the room has one.
    shutter: Option<(f32, f32)>,
    /// The platform a crossing climbs first: its x span (the court's step,
    /// the split's overhead route for the `high` variant).
    climb: Option<(f32, f32)>,
    /// The exit's centre column.
    exit_x: f32,
}

impl Route {
    pub fn of(run: &RingRun, high: bool) -> Self {
        let room = &run.loaded.room;
        let layout_h = run.kit.height;
        let exit = run.loaded.encounter.exit;
        let shutter = run.shutter().map(|s| (s.x as f32, s.right() as f32));
        let climb = match run.room {
            // The court's step is the first platform (RING-A core).
            RingRoom::Court => room.platforms.first(),
            RingRoom::Split if high => room.platforms.first(),
            RingRoom::Split => None,
        }
        .map(|p| (p.rect.x as f32, p.rect.right() as f32));
        Self {
            floor_feet: to_world_y(layout_h, room.floor_y),
            shutter,
            climb,
            exit_x: (exit.x + exit.w / 2) as f32,
        }
    }

    /// The column a hero must pass on the floor to use the climb: right of the
    /// shutter for the court, right of the overhead route for the split.
    fn climb_until(&self) -> Option<f32> {
        match (self.shutter, self.climb) {
            (Some((_, right)), Some(_)) => Some(right),
            (None, Some((_, right))) => Some(right),
            _ => None,
        }
    }
}

fn on_floor(route: &Route, scene: &SceneSim, hero: HeroId) -> bool {
    let body = scene.world.body(hero);
    body.y - HERO_HALF_HEIGHT < route.floor_feet + 2.0
}

/// Whether a solid stands within 14 px ahead of a body at ledge height.
fn blocked_ahead(scene: &SceneSim, x: f32, y: f32, dir: f32) -> bool {
    let probe = Rect {
        left: x - HERO_HALF_WIDTH + dir * 14.0,
        bottom: y - HERO_HALF_HEIGHT + 4.0,
        width: HERO_HALF_WIDTH * 2.0,
        height: 8.0,
    };
    crate::combat::solid_at(&scene.world.solids, probe)
}

/// Whether any roster enemy lives.
fn enemies_left(combat: &CombatState) -> bool {
    combat.enemies.iter().any(|g| g.alive())
}

/// The pilot of hero `hero`: the shared late-slice pilot (base kit, no
/// ability), the walk to the exit once the roster is clear, and the room's
/// route rule (module docs). A pure function of the state; it never reads the
/// camera and never presses `ability`.
pub fn pilot(route: &Route, combat: &CombatState, scene: &SceneSim, hero: HeroId) -> RawInput {
    let body = scene.world.body(hero);
    let mut raw = if enemies_left(combat) {
        pilot_with(combat, scene, hero, false)
    } else {
        let mut raw = RawInput::default();
        let dx = route.exit_x - body.x;
        if dx.abs() > 4.0 {
            raw.move_x = dx.signum();
            if body.grounded && blocked_ahead(scene, body.x, body.y, raw.move_x) {
                raw.jump = true;
            }
        }
        raw
    };
    raw.ability = false;
    // A hero standing on a raised surface (the step, the walkway, the overhead
    // route, the view ledge) walks on to its far end and drops back to the lane:
    // every goal of the room lies right (RING-14), Donut's level missile flies
    // over a goblin below and a hop-and-dive from 64 px stalls the shared pilot.
    if body.grounded && !on_floor(route, scene, hero) {
        raw = RawInput {
            move_x: 1.0,
            jump: blocked_ahead(scene, body.x, body.y, 1.0),
            ..RawInput::default()
        };
    }
    // Review F4 of #271: the shared pilot steps toward a goblin it overlaps
    // (|dx| under its turn dead band), so its jab never lands and the hero is
    // worn down. A grounded hero on the floor that overlaps a living goblin
    // steps away from it first, then the shared pilot fights from range. Carl
    // only: Donut's own rules answer point-blank with the pounce.
    if hero == HeroId::Carl
        && body.grounded
        && on_floor(route, scene, hero)
        && let Some(g) = combat
            .enemies
            .iter()
            .filter(|g| g.alive())
            .find(|g| (g.x - body.x).abs() < OVERLAP_PX)
    {
        let away = if g.x > body.x { -1.0 } else { 1.0 };
        raw = RawInput {
            move_x: away,
            ..RawInput::default()
        };
    }
    // The route: a grounded hero on the floor, left of the crossing, whose
    // business lies right of it.
    if let (Some((climb_x, _)), Some(until)) = (route.climb, route.climb_until())
        && on_floor(route, scene, hero)
        && body.x < until
    {
        let goal_right = combat
            .enemies
            .iter()
            .filter(|g| g.alive())
            .any(|g| g.x > until)
            || !enemies_left(combat);
        let wants = goal_right && (raw.move_x > 0.0 || route.shutter.is_some());
        // Under the climb (or against the shutter): no takeoff there, back out.
        let blocked_under = body.x > climb_x - 4.0;
        if wants && route.shutter.is_some() && blocked_under {
            raw = RawInput {
                move_x: -1.0,
                ..RawInput::default()
            };
        } else if wants && !blocked_under {
            raw = RawInput {
                move_x: 1.0,
                jump: body.grounded && body.x >= climb_x - TAKEOFF_PX,
                ..RawInput::default()
            };
        }
    }
    raw
}

/// The report of one run.
#[derive(Debug, Clone)]
pub struct Report {
    pub room: RingRoom,
    pub who: Who,
    pub seed: u64,
    pub high: bool,
    /// The tick the last roster enemy was defeated on.
    pub cleared_tick: Option<u64>,
    /// The tick a hero first reached the exit's column after the clear.
    pub exit_tick: Option<u64>,
    /// The longest stuck run per hero (module docs).
    pub max_stuck: [u32; 2],
    /// RING-14: px the controlled hero walked back to the left, in total.
    pub backtrack_px: f32,
    /// RING-14: ticks the controlled hero was told to walk and did not move.
    pub forced_wait_ticks: u32,
    pub deaths: u32,
    pub hero_hurts: usize,
    /// Whether the hero crossed the shutter's column standing on the walkway
    /// (court only), per hero: the crossing over the closed shutter (KR3).
    pub walkway_crossing: [bool; 2],
    /// Whether the hero ever stood on the crossing platform (split `high`).
    pub climbed: [bool; 2],
    /// Whether a living hero ever stood right of the shutter on the floor
    /// before crossing over it (a crossing under the closed arch).
    pub under_the_shutter: bool,
    /// Whether `ability` was ever pressed (never: RING-06, no dynamite).
    pub ability_pressed: bool,
    pub hash: u64,
    pub ticks: u64,
    pub events: Vec<CombatEvent>,
}

/// Start one run of `room` for `who` at seed `seed`.
pub fn start(
    root: &Path,
    room: RingRoom,
    who: Who,
    seed: u64,
) -> Result<(PartyFixtureSim, RingRun), String> {
    let (carl, donut) = Loadout::Story.flags();
    let (run, scene, combat) = ring_start(root, room, 360, carl, donut, Some(seed))?;
    let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
        base: run.fixture,
        two_player: false,
        tuning: who.tuning(),
        script: &[],
        pilot: Pilot::ArenaDirect,
        donut: DonutSetup::unlocked(8),
    }));
    Ok((PartyFixtureSim::from_parts(scene, combat, fixture), run))
}

/// Run one pilot for [`RUN_TICKS`] (or until the exit is reached).
pub fn run(root: &Path, room: RingRoom, who: Who, seed: u64, high: bool) -> Result<Report, String> {
    let (mut sim, ring) = start(root, room, who, seed)?;
    let route = Route::of(&ring, high);
    let walkway = (room == RingRoom::Court).then(|| {
        let p = ring.loaded.room.platforms[1].rect;
        (
            p.x as f32,
            p.right() as f32,
            to_world_y(ring.kit.height, p.y),
        )
    });
    let climb_feet = route.climb.map(|_| {
        let p = ring.loaded.room.platforms[0].rect;
        to_world_y(ring.kit.height, p.y)
    });
    let mut last_x = [sim.scene.world.hero.x, sim.scene.world.donut_body.x];
    let mut stuck = [0u32; 2];
    let mut max_stuck = [0u32; 2];
    let mut backtrack_px = 0.0f32;
    let mut forced_wait_ticks = 0u32;
    let mut cleared_tick = None;
    let mut exit_tick = None;
    // Review F1 of #271: a hero defeat respawns the party at the checkpoint
    // without a restart, so the count is the combat state's own `deaths`, never
    // the restart counter (which stays 0 here).
    let deaths_at_start = sim.combat.deaths;
    let mut swap_pressed = false;
    let mut walkway_crossing = [false; 2];
    let mut climbed = [false; 2];
    let mut under_the_shutter = false;
    let mut ability_pressed = false;
    for _ in 0..RUN_TICKS {
        let tick = sim.scene.world.tick + 1;
        let controlled = sim.party.controlled_hero();
        let mut input = pilot(&route, &sim.combat, &sim.scene, controlled);
        if who == Who::DonutSolo && tick >= SWAP_TICK && !swap_pressed && controlled == HeroId::Carl
        {
            input.swap = true;
            swap_pressed = true;
        }
        ability_pressed |= input.ability;
        let walking = input.move_x != 0.0;
        sim.step_slots([input, RawInput::NONE]);
        let carl_x = sim.scene.world.hero.x;
        let carl_moved = (carl_x - last_x[0]).abs() >= 0.1;
        for (i, id) in HeroId::ALL.into_iter().enumerate() {
            let body = sim.scene.world.body(id);
            let x = body.x;
            let feet = body.y - HERO_HALF_HEIGHT;
            let still = (x - last_x[i]).abs() < 0.1;
            let counts = if id == controlled {
                walking && still
            } else {
                // The partner: left behind while the steered hero walks on.
                still && (x - sim.scene.world.body(controlled).x).abs() > APART_PX && carl_moved
            };
            if id == controlled {
                if walking && still {
                    forced_wait_ticks += 1;
                }
                if x < last_x[i] {
                    backtrack_px += last_x[i] - x;
                }
            }
            stuck[i] = if counts { stuck[i] + 1 } else { 0 };
            max_stuck[i] = max_stuck[i].max(stuck[i]);
            if let (Some((s0, s1)), Some((w0, w1, top))) = (route.shutter, walkway) {
                let mid = (s0 + s1) * 0.5;
                if (feet - top).abs() < 1.0 && x >= w0 && x <= w1 && last_x[i] < mid && x >= mid {
                    walkway_crossing[i] = true;
                }
                // On the floor right of the shutter's face before any crossing.
                if !walkway_crossing[i] && feet < route.floor_feet + 1.0 && x > s1 && x < s1 + 8.0 {
                    under_the_shutter = true;
                }
            }
            if let (Some((c0, c1)), Some(top)) = (route.climb, climb_feet)
                && (feet - top).abs() < 1.0
                && x >= c0
                && x <= c1
            {
                climbed[i] = true;
            }
            last_x[i] = x;
        }
        if cleared_tick.is_none() && !enemies_left(&sim.combat) {
            cleared_tick = Some(sim.scene.world.tick);
        }
        if cleared_tick.is_some()
            && exit_tick.is_none()
            && (sim.scene.world.complete
                || HeroId::ALL
                    .into_iter()
                    .any(|id| sim.scene.world.body(id).x >= route.exit_x))
        {
            exit_tick = Some(sim.scene.world.tick);
        }
        if exit_tick.is_some() {
            break;
        }
    }
    let hero_hurts = sim
        .combat
        .events
        .iter()
        .filter(|e| matches!(e, CombatEvent::HeroHurt { .. }))
        .count();
    let deaths = sim.combat.deaths - deaths_at_start;
    // The event log says the same: one `HeroDefeated` per defeat.
    debug_assert_eq!(
        deaths as usize,
        sim.combat
            .events
            .iter()
            .filter(|e| matches!(e, CombatEvent::HeroDefeated { .. }))
            .count()
    );
    Ok(Report {
        room,
        who,
        seed,
        high,
        cleared_tick,
        exit_tick,
        max_stuck,
        backtrack_px,
        forced_wait_ticks,
        deaths,
        hero_hurts,
        walkway_crossing,
        climbed,
        under_the_shutter,
        ability_pressed,
        hash: sim.state_hash(),
        ticks: sim.scene.world.tick,
        events: sim.combat.events.clone(),
    })
}

impl Report {
    /// One line for the evidence.
    pub fn line(&self) -> String {
        format!(
            "RING_RUN room={} who={} seed={} high={} cleared={:?} exit={:?} deaths={} hurts={} \
             max_stuck={:?} backtrack_px={:.0} forced_wait={} walkway={:?} climbed={:?} \
             under_shutter={} hash={:016x}",
            self.room.id(),
            self.who.name(),
            self.seed,
            self.high,
            self.cleared_tick,
            self.exit_tick,
            self.deaths,
            self.hero_hurts,
            self.max_stuck,
            self.backtrack_px,
            self.forced_wait_ticks,
            self.walkway_crossing,
            self.climbed,
            self.under_the_shutter,
            self.hash
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// RING-06, RING-14 and the solo traversal (RING-A wire): solo Carl and
    /// solo Donut (the swap) clear both rooms and reach the exit at every seed,
    /// with the base kit only (no dynamite, no ability), no death, no hero stuck
    /// over the limit, a bounded backtrack, and in the court a crossing of the
    /// closed shutter over the walkway (never under the arch).
    #[test]
    fn ring_solo_carl_and_solo_donut_clear_both_rooms_at_every_seed() {
        for room in RingRoom::ALL {
            for who in [Who::CarlSolo, Who::DonutSolo] {
                let hero = match who {
                    Who::DonutSolo => 1,
                    _ => 0,
                };
                let mut outcomes = Vec::new();
                for seed in SEEDS {
                    let r = run(&root(), room, who, seed, false).unwrap();
                    let line = r.line();
                    assert!(r.cleared_tick.is_some() && r.exit_tick.is_some(), "{line}");
                    assert_eq!(r.deaths, 0, "{line}");
                    assert!(!r.ability_pressed, "{line}");
                    assert!(r.max_stuck[hero] <= STUCK_LIMIT, "{line}");
                    assert!(r.backtrack_px <= 256.0, "RING-14 backtrack: {line}");
                    assert!(r.exit_tick.unwrap() < 1500, "{line}");
                    assert!(!r.under_the_shutter, "{line}");
                    if room == RingRoom::Court {
                        assert!(r.walkway_crossing[hero], "KR3 walkway: {line}");
                    }
                    outcomes.push(r.hash);
                }
                // The seeds are part of the run (the goblins' temperaments).
                outcomes.sort_unstable();
                outcomes.dedup();
                assert!(
                    outcomes.len() > 1,
                    "{room:?} {who:?}: one outcome for nine seeds"
                );
            }
        }
    }

    /// RING-08: the split's overhead route is a real choice: solo Carl takes it
    /// (he stands on it), it rejoins the lane and he clears the room and exits.
    #[test]
    fn ring_split_overhead_route_rejoins_and_exits() {
        for seed in [0, 4, 8] {
            let r = run(&root(), RingRoom::Split, Who::CarlSolo, seed, true).unwrap();
            let line = r.line();
            assert!(r.climbed[0], "{line}");
            assert!(r.exit_tick.is_some() && r.deaths == 0, "{line}");
            assert!(r.max_stuck[0] <= STUCK_LIMIT, "{line}");
        }
    }

    /// The runs replay: the same room, pilot and seed give the same hash.
    #[test]
    fn ring_runs_are_deterministic() {
        let once = || {
            run(&root(), RingRoom::Court, Who::DonutSolo, 3, false)
                .unwrap()
                .hash
        };
        assert_eq!(once(), once());
    }

    /// KR19 with the autonomous partner (`cf02_free`): both rooms clear at every
    /// seed with no defeat. In the court Carl crosses over the walkway and nobody
    /// walks under the closed arch; that the AI Donut follows him over the
    /// walkway during the fight is [`ring_court_companion_crosses_the_walkway`]
    /// and [`ring_court_companion_near_follow_crosses_the_shutter`] (RINGW-F1).
    #[test]
    fn ring_companion_runs() {
        for seed in SEEDS {
            let r = run(&root(), RingRoom::Split, Who::Companion, seed, false).unwrap();
            assert!(r.exit_tick.is_some() && r.deaths == 0, "{}", r.line());
            let r = run(&root(), RingRoom::Court, Who::Companion, seed, false).unwrap();
            assert!(r.walkway_crossing[0], "{}", r.line());
            assert!(!r.under_the_shutter, "{}", r.line());
            // Since the F4 pilot fix Carl clears the court even when the partner
            // stays behind (RINGW-F1 is about following, not progression).
            assert!(r.exit_tick.is_some() && r.deaths == 0, "{}", r.line());
        }
    }

    /// RINGW-F1 (KR19): the autonomous partner crosses the closed shutter over
    /// the walkway while Carl fights **near** it on the far side, before he
    /// reaches the exit. Before the fix she approached him directly and waited at
    /// the shutter's left face (x ~1172) while he fought g1 about 200 px away:
    /// the drift check priced the whole crossing as one commitment and refused
    /// every spot across (RWF1-AD1, RWF1-AD2).
    #[test]
    fn ring_court_companion_crosses_the_walkway() {
        for seed in SEEDS {
            let r = run(&root(), RingRoom::Court, Who::Companion, seed, false).unwrap();
            assert!(r.walkway_crossing[1], "{}", r.line());
            assert!(!r.under_the_shutter, "{}", r.line());
            assert!(r.exit_tick.is_some(), "{}", r.line());
        }
    }

    /// RINGW-F1 knob off (`rwf1_cut_off_escort`): the court's companion run is
    /// the pre-fix run byte for byte (the hashes of `ring_runs.txt` before the
    /// fix, every seed), and the partner does not cross over the walkway.
    #[test]
    fn ring_court_companion_with_the_rwf1_knob_off_keeps_its_old_hash() {
        const PRE_FIX: [u64; 9] = [
            0x28e0_19ad_b12a_867d,
            0xdd50_0119_e37c_db95,
            0xcf06_6cac_2402_4fc9,
            0x43d8_38df_73bc_176c,
            0x8962_63db_1ca7_c240,
            0x4cc9_cc15_f097_f013,
            0x7ad6_3789_4c2e_c497,
            0xe604_2116_18d1_bc51,
            0xdd2c_2ef6_c46c_a188,
        ];
        for seed in SEEDS {
            let r = crate::combat_scene::with_profile_override(
                |f| f.cf02_free.fix_all.rwf1_cut_off_escort = false,
                || run(&root(), RingRoom::Court, Who::Companion, seed, false).unwrap(),
            );
            assert_eq!(r.hash, PRE_FIX[seed as usize], "{}", r.line());
            assert_eq!(r.walkway_crossing, [true, false], "{}", r.line());
        }
    }

    /// RINGW-F1, the near follow: Carl crosses over the walkway and then stands
    /// (no input) on the floor just right of the closed shutter, about 120 px
    /// from its face, while the goblins come to him. The autonomous partner, cut
    /// off on the left floor, takes the nav route over the step and the walkway
    /// by herself (no regroup placement) and joins him on his floor within
    /// [`NEAR_FOLLOW_TICKS`] of his landing; she never waits under the shutter.
    #[test]
    fn ring_court_companion_near_follow_crosses_the_shutter() {
        use crate::companion::events::CompanionEvent;
        /// 6 s: the crossing (two jumps and a drop) takes about 4 s.
        const NEAR_FOLLOW_TICKS: u64 = 360;
        for seed in SEEDS {
            let (mut sim, ring) = start(&root(), RingRoom::Court, Who::Companion, seed).unwrap();
            let route = Route::of(&ring, false);
            let (s0, s1) = route.shutter.unwrap();
            let mid = (s0 + s1) * 0.5;
            let walk = ring.loaded.room.platforms[1].rect;
            let walk_top = to_world_y(ring.kit.height, walk.y);
            let (w0, w1) = (walk.x as f32, walk.right() as f32);
            let mut landed: Option<u64> = None;
            let (mut over, mut under, mut regroup) = (false, false, false);
            let mut joined: Option<u64> = None;
            let mut last_x = sim.scene.world.body(HeroId::Donut).x;
            for _ in 0..RUN_TICKS {
                let input = if landed.is_some() {
                    RawInput::NONE
                } else {
                    pilot(&route, &sim.combat, &sim.scene, HeroId::Carl)
                };
                sim.step_slots([input, RawInput::NONE]);
                let t = sim.scene.world.tick;
                regroup |= sim
                    .combat
                    .take_companion_events()
                    .iter()
                    .any(|e| matches!(e, CompanionEvent::Regroup { .. }));
                let carl = sim.scene.world.body(HeroId::Carl).x;
                let (dx, dy) = {
                    let d = sim.scene.world.body(HeroId::Donut);
                    (d.x, d.y)
                };
                if landed.is_none()
                    && on_floor(&route, &sim.scene, HeroId::Carl)
                    && carl > s1 + HERO_HALF_WIDTH
                {
                    landed = Some(t);
                }
                let feet = dy - HERO_HALF_HEIGHT;
                if (feet - walk_top).abs() < 1.0
                    && (w0..=w1).contains(&dx)
                    && last_x < mid
                    && dx >= mid
                {
                    over = true;
                }
                if !over && on_floor(&route, &sim.scene, HeroId::Donut) && dx > s1 {
                    under = true;
                }
                last_x = dx;
                if joined.is_none()
                    && over
                    && on_floor(&route, &sim.scene, HeroId::Donut)
                    && dx > s1
                    && (dx - carl).abs() <= 120.0
                {
                    joined = Some(t);
                }
                if let Some(t0) = landed
                    && (joined.is_some() || t >= t0 + NEAR_FOLLOW_TICKS)
                {
                    break;
                }
            }
            let line = format!(
                "seed {seed}: landed {landed:?} joined {joined:?} over {over} under {under} \
                 regroup {regroup}"
            );
            let t0 = landed.unwrap_or_else(|| panic!("Carl never landed right: {line}"));
            assert!(
                joined.is_some_and(|t| t <= t0 + NEAR_FOLLOW_TICKS),
                "{line}"
            );
            assert!(over && !under && !regroup, "{line}");
        }
    }

    /// The nav of the companion sees the closed shutter as slices with no open
    /// anchor and plans the crossing over the step and the walkway (KR6, KR19).
    #[test]
    fn ring_court_nav_plans_the_walkway_crossing() {
        use crate::companion::nav::{NavConfig, NavGraph, NavLevel, Unlocks};
        let (sim, ring) = start(&root(), RingRoom::Court, Who::Companion, 0).unwrap();
        let layout_h = ring.kit.height;
        let feet = to_world_y(layout_h, ring.loaded.room.floor_y);
        let step = ring.loaded.room.platforms[0].rect;
        let walk = ring.loaded.room.platforms[1].rect;
        for cfg in [NavConfig::agile(), NavConfig::integrated()] {
            let g = NavGraph::build(
                &NavLevel::from_blocks(&sim.scene.world.solids, Vec::new()),
                cfg,
            )
            .unwrap();
            let from = g.anchor_at(900.0, feet).unwrap();
            let to = g.anchor_at(1400.0, feet).unwrap();
            for hero in HeroId::ALL {
                let path = g.path_or_reason(hero, from, to, Unlocks::NONE).unwrap();
                let tops: Vec<f32> = path
                    .anchors
                    .iter()
                    .map(|a| g.anchor_pos(*a).unwrap().1)
                    .collect();
                assert!(
                    tops.contains(&to_world_y(layout_h, step.y)),
                    "{hero:?} {tops:?}"
                );
                assert!(
                    tops.contains(&to_world_y(layout_h, walk.y)),
                    "{hero:?} {tops:?}"
                );
            }
            // No slice top is an open anchor (KR3 clause 1).
            let shutter = ring.shutter().unwrap();
            for a in g.anchors() {
                let (x, y) = g.anchor_pos(a).unwrap();
                let on_slice = x > shutter.x as f32
                    && x < shutter.right() as f32
                    && y < to_world_y(layout_h, walk.y)
                    && y > feet;
                assert!(!(on_slice && g.anchor_open(a)), "{a:?} at ({x}, {y})");
            }
        }
    }

    /// The LoS check (RING-B wire): Donut's Magic Missile stops on the closed
    /// shutter (no hit on a goblin behind it), and the same cast from the
    /// goblins' side hits. The CF02-B slam: Carl's kick that launches a goblin
    /// into the shutter's face slams it (`WallSlam`, KR4: the existing effect).
    #[test]
    fn ring_court_shutter_stops_the_missile_and_slams_a_launched_goblin() {
        let cast = |donut_x: f32| {
            let (mut sim, ring) = start(&root(), RingRoom::Court, Who::DonutSolo, 0).unwrap();
            // The swap to Donut, then place her and the first goblin.
            sim.step_slots([
                RawInput {
                    swap: true,
                    ..RawInput::NONE
                },
                RawInput::NONE,
            ]);
            for _ in 0..20 {
                sim.step_slots([RawInput::NONE, RawInput::NONE]);
            }
            assert_eq!(sim.party.controlled_hero(), HeroId::Donut);
            sim.scene.world.donut_body.x = donut_x;
            sim.scene.world.hero.x = 400.0;
            // The same distance both ways, inside the bolt's 240 px flight.
            sim.combat.enemies[0].x = donut_x + 182.0;
            let _ = &ring;
            let before = sim.combat.events.len();
            for t in 0..240u64 {
                let input = RawInput {
                    move_x: if t < 2 { 1.0 } else { 0.0 },
                    attack: t % 40 == 10,
                    ..RawInput::NONE
                };
                sim.step_slots([input, RawInput::NONE]);
            }
            sim.combat.events[before..]
                .iter()
                .any(|e| matches!(e, CombatEvent::Hit { target, .. } if *target < 2))
        };
        // Left of the shutter (1200..1232) with the goblin behind it, then the
        // same cast on the goblins' side of it.
        assert!(!cast(1140.0), "the bolt stops on the closed shutter");
        assert!(
            cast(1250.0),
            "the control: the same cast at the same distance hits"
        );

        // The slam: Carl right of the shutter, a goblin between him and its face.
        let (mut sim, ring) = start(&root(), RingRoom::Court, Who::CarlSolo, 0).unwrap();
        let s = ring.shutter().unwrap();
        sim.scene.world.hero.x = s.right() as f32 + 40.0;
        sim.scene.world.donut_body.x = 300.0;
        sim.combat.enemies[0].x = s.right() as f32 + 14.0;
        let mut slammed = false;
        for t in 0..240u64 {
            let input = RawInput {
                move_x: if t < 3 { -1.0 } else { 0.0 },
                physical: t % 30 == 5,
                ..RawInput::NONE
            };
            sim.step_slots([input, RawInput::NONE]);
            slammed |= sim
                .combat
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::WallSlam { goblin: 0, .. }));
        }
        assert!(slammed, "a launched goblin slams on the shutter's face");
    }

    /// Review F5 of #271 (RING-B wire, the blast LoS): Carl's dynamite has line
    /// of sight on (CP-Q22), and the closed shutter's slices in the world block a
    /// blast segment from its left side to a goblin behind it inside the 64 px
    /// ring; the same distance on the goblins' side is clear.
    #[test]
    fn ring_court_shutter_blocks_the_dynamite_blast_line() {
        use crate::blast::{closest_point, line_of_sight};
        let profile =
            crate::thrown_charge::blast_profile(crate::carl_prog::CarlProgTuning::active());
        assert!(profile.los, "the thrown charge's blast respects solids");
        let (sim, ring) = start(&root(), RingRoom::Court, Who::CarlSolo, 0).unwrap();
        let s = ring.shutter().unwrap();
        let feet = to_world_y(ring.kit.height, ring.loaded.room.floor_y);
        let goblin = |x: f32| Rect {
            left: x - 10.0,
            bottom: feet,
            width: 20.0,
            height: 38.0,
        };
        let solids = &sim.scene.world.solids;
        // Centre 20 px left of the face, the goblin's near edge 10 px right of it.
        let centre = (s.x as f32 - 20.0, feet + 8.0);
        let behind = goblin(s.right() as f32 + 20.0);
        let near = closest_point(behind, centre);
        let dist = ((near.0 - centre.0).powi(2) + (near.1 - centre.1).powi(2)).sqrt();
        assert!(dist < profile.outer_radius, "{dist} inside the ring");
        assert!(
            !line_of_sight(solids, centre, near),
            "the shutter blocks it"
        );
        // The control: the same offset on the goblins' side.
        let centre = (s.right() as f32 + 40.0, feet + 8.0);
        let open = goblin(s.right() as f32 + 80.0);
        assert!(line_of_sight(solids, centre, closest_point(open, centre)));
    }

    /// BLAST-SEAM (RING-FU-A): the court's world with its one-block shutter
    /// (RINGW-AD8) replaced by KR6's 17 Timber slices of 4 px, as RING-C wire's
    /// moving shutter will append them. The floor charge's blast line runs
    /// exactly along a slice seam (centre 8 px up), and since the seam fix
    /// the sliced stack blocks it as the one block does; so does every other
    /// slice seam, and the seam between the stack's top and the walkway's
    /// underside (flush at 68 px). Beside the stack the walkway's underside is
    /// a plain outer edge and still passes.
    #[test]
    fn ring_court_a_sliced_shutter_and_its_walkway_seam_block_the_blast_line() {
        use crate::blast::line_of_sight;
        use crate::prototype::Block;
        let (sim, ring) = start(&root(), RingRoom::Court, Who::CarlSolo, 0).unwrap();
        let s = ring.shutter().unwrap();
        let feet = to_world_y(ring.kit.height, ring.loaded.room.floor_y);
        let (left, right) = (s.x as f32, s.right() as f32);
        let mut solids = sim.scene.world.solids.clone();
        let at = solids
            .iter()
            .position(|b| b.rect.left == left && b.rect.width == s.w as f32)
            .expect("the shutter's block");
        let stack = solids.remove(at);
        assert!(stack.intact);
        assert_eq!((stack.rect.bottom, stack.rect.height), (feet, 68.0));
        let slice_px = f32::from(
            ring.loaded.encounter.props[0]
                .shutter
                .expect("the shutter's config")
                .slice_px,
        );
        let count = (stack.rect.height / slice_px) as usize;
        assert_eq!(count, 17, "KR6");
        solids.extend((0..count).map(|k| Block {
            rect: Rect {
                bottom: feet + slice_px * k as f32,
                height: slice_px,
                ..stack.rect
            },
            ..stack.clone()
        }));
        // The floor charge 20 px left of the face, a goblin's near edge 10 px
        // right of it: the segment is horizontal on the seam at 8 px.
        let (centre, near) = ((left - 20.0, feet + 8.0), (right + 10.0, feet + 8.0));
        assert_eq!(slice_px * 2.0, 8.0, "the line lies on a seam");
        assert!(!line_of_sight(&solids, centre, near), "the slices block it");
        for k in 1..count {
            let y = feet + slice_px * k as f32;
            assert!(
                !line_of_sight(&solids, (left - 20.0, y), (right + 10.0, y)),
                "seam {k}"
            );
        }
        // The stack's top against the walkway's underside: a blast exactly at
        // that height, from under the walkway left of the stack to the right.
        let top = feet + 68.0;
        assert!(
            solids.iter().any(|b| b.intact
                && b.rect.bottom == top
                && b.rect.left < left
                && b.rect.right() > right),
            "the walkway's underside is flush with the stack's top"
        );
        for solids in [&solids, &sim.scene.world.solids] {
            assert!(!line_of_sight(
                solids,
                (left - 20.0, top),
                (right + 10.0, top)
            ));
            assert!(
                line_of_sight(solids, (right + 10.0, top), (right + 60.0, top)),
                "beside the stack the underside is an outer edge"
            );
        }
    }

    /// Review F5 of #271 (RING-B wire, "the walkway lane stays open"): the
    /// band a bolt flies in when Donut casts on the walkway (16..22 px above its
    /// top, the box `bolt_blocked` probes) crosses the closed shutter's column
    /// with no intact solid in it, while the same band on the floor is blocked
    /// by the shutter. (No goblin stands on the walkway: goblins walk the
    /// floor line, so a hit there cannot be staged without moving one by hand.)
    #[test]
    fn ring_court_walkway_lane_stays_open_for_the_missile() {
        let (sim, ring) = start(&root(), RingRoom::Court, Who::CarlSolo, 0).unwrap();
        let walk = ring.loaded.room.platforms[1].rect;
        let s = ring.shutter().unwrap();
        let band = |feet: f32| Rect {
            left: walk.x as f32 + 8.0,
            bottom: feet + 16.0,
            width: (s.right() + 60 - walk.x) as f32,
            height: 6.0,
        };
        let solids = &sim.scene.world.solids;
        let floor = to_world_y(ring.kit.height, ring.loaded.room.floor_y);
        let top = to_world_y(ring.kit.height, walk.y);
        assert!(
            !crate::combat::solid_at(solids, band(top)),
            "the walkway lane is open"
        );
        assert!(
            crate::combat::solid_at(solids, band(floor)),
            "the floor lane is cut"
        );
    }

    /// The trace probe: `RING_TRACE=room:who:high` prints every 20 ticks.
    #[test]
    #[ignore = "probe: traces one run"]
    fn ring_trace() {
        let spec = std::env::var("RING_TRACE").unwrap_or_else(|_| "ring_court:donut_solo:0".into());
        let parts: Vec<&str> = spec.split(':').collect();
        let room = RingRoom::from_id(parts[0]).unwrap();
        let who = Who::ALL
            .into_iter()
            .find(|w| w.name().starts_with(parts[1]))
            .unwrap();
        let high = parts.get(2) == Some(&"1");
        let (mut sim, ring) = start(&root(), room, who, 0).unwrap();
        let route = Route::of(&ring, high);
        let mut swapped = false;
        for _ in 0..2400 {
            let tick = sim.scene.world.tick + 1;
            let c = sim.party.controlled_hero();
            let mut input = pilot(&route, &sim.combat, &sim.scene, c);
            if who == Who::DonutSolo && tick >= SWAP_TICK && !swapped && c == HeroId::Carl {
                input.swap = true;
                swapped = true;
            }
            sim.step_slots([input, RawInput::NONE]);
            if tick % 20 == 0 {
                let w = &sim.scene.world;
                let g: Vec<String> = sim
                    .combat
                    .enemies
                    .iter()
                    .map(|g| format!("{:.0}/{:.0}:{}", g.x, g.y, g.alive()))
                    .collect();
                println!(
                    "t={tick} ctl={c:?} in=({},{},{},{}) carl=({:.0},{:.0}) donut=({:.0},{:.0}) gob={g:?}",
                    input.move_x,
                    input.jump,
                    input.attack,
                    input.physical,
                    w.hero.x,
                    w.hero.y,
                    w.donut_body.x,
                    w.donut_body.y
                );
            }
        }
    }

    /// The probe: every room, pilot and seed, one line each.
    #[test]
    #[ignore = "probe: prints every run"]
    fn ring_probe() {
        for room in RingRoom::ALL {
            for who in Who::ALL {
                for seed in SEEDS {
                    let r = run(&root(), room, who, seed, false).unwrap();
                    println!("{}", r.line());
                }
            }
        }
        for seed in SEEDS {
            let r = run(&root(), RingRoom::Split, Who::CarlSolo, seed, true).unwrap();
            println!("{}", r.line());
        }
    }
}
