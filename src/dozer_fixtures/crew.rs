//! DR-C fixtures: the crew (`docs/01-design/enemy-gameplay-dozer.md`, sections
//! 8 and 10, DOZR-06 and the crew half of DOZR-07).
//!
//! The riders are room 3's enemies 0 (`rider_a`) and 1 (`rider_b`). They ride
//! the machine until it is disabled and then land at the wheel front + 32 and
//! + 72 (nudged right off a hero), lying for 45 ticks.

use super::frame::{blast_release, brace_pilot, brace_release_r, exit_x, frame_state, to_chase};
use super::*;
use crate::combat::ENEMY_HALF_WIDTH;
use crate::combat_scene::CombatState;
use crate::combat_scene::crew::landing_x;
use crate::env_prop::MassState;
use crate::goblin::GoblinMode;

/// The riders' enemy indices.
pub const RIDERS: [usize; 2] = [0, 1];

/// The rider spots' offsets from the wheel front (DZ-Q20).
pub const LAND_DX: [f32; 2] = [32.0, 72.0];

/// The ticks a rider lies after its landing (DZ-Q20).
pub const DOWN_TICKS: u64 = 45;

/// DR-C: both riders are aboard.
pub fn all_carried(c: &CombatState) -> bool {
    RIDERS.iter().all(|&i| c.is_carried(i))
}

/// The living enemies of the room: the goblins and the slot enemies.
fn living(sim: &PartyFixtureSim) -> usize {
    sim.combat.enemies.iter().filter(|g| g.alive()).count()
        + sim
            .combat
            .slot_enemies()
            .iter()
            .filter(|e| e.alive())
            .count()
}

/// The sidecar's `max_living` (DZ-Q21).
fn max_living(st: &Stage) -> usize {
    usize::from(st.run.loaded.encounter.max_living)
}

/// An intact solid lies on the walking line in the remnant's span (640..720).
fn remnant_down(st: &Stage) -> bool {
    let floor = st.floor();
    st.sim.scene.world.solids.iter().any(|b| {
        b.intact
            && b.rect.left < 720.0
            && b.rect.right() > 640.0
            && b.rect.bottom <= floor + 0.5
            && b.rect.top() > floor + 0.5
    })
}

/// The machine has stopped (stunned or disabled).
fn stopped(st: &Stage) -> bool {
    matches!(
        st.dozer().state,
        DozerState::ImpactStunned { .. } | DozerState::Disabled
    )
}

/// Step with nothing pressed until the next tick disables the machine.
fn to_eve_of_disable(st: &mut Stage) -> Result<(), String> {
    for _ in 0..600 {
        let mut probe = st.clone();
        probe.step_slots([RawInput::NONE; 2]);
        if probe.dozer().state == DozerState::Disabled {
            return Ok(());
        }
        ensure!(
            all_carried(&st.sim.combat),
            "a rider left before the disable"
        );
        st.step_slots([RawInput::NONE; 2]);
    }
    Err("the machine was never disabled".into())
}

/// Check the landing of every rider on the disable tick against the spots
/// [`landing_x`] gives for the heroes where they stand; returns the spots.
fn check_landing(st: &Stage) -> Result<[f32; 2], String> {
    let c = &st.sim.combat;
    let d = st.dozer();
    ensure!(
        d.state == DozerState::Disabled,
        "the machine is {:?}",
        d.state
    );
    let front = d.wheel_front();
    let stop_x = d.site.stop_x as f32;
    let heroes = [st.sim.scene.world.hero.x, st.sim.scene.world.donut_body.x];
    let (view_l, view_r) = st.sim.scene.view_x();
    let mut spots = [0.0; 2];
    for (k, &i) in RIDERS.iter().enumerate() {
        let mount = c.crew_of(i).ok_or(format!("enemy {i} is no rider"))?;
        ensure!(
            mount.land_dx as f32 == LAND_DX[k] && u64::from(mount.down_ticks) == DOWN_TICKS,
            "rider {k}: the sidecar's {mount:?}"
        );
        let g = &c.enemies[i];
        let want = landing_x(
            front,
            &mount,
            &heroes,
            view_l + ENEMY_HALF_WIDTH,
            view_r.min(stop_x) - ENEMY_HALF_WIDTH,
        );
        ensure!(
            !c.is_carried(i) && g.alive() && g.mode == GoblinMode::Downed,
            "rider {k}: carried {} mode {:?} health {}",
            c.is_carried(i),
            g.mode,
            g.health
        );
        ensure!(g.x == want, "rider {k} at {}, the spot is {want}", g.x);
        ensure!(
            g.x - ENEMY_HALF_WIDTH >= view_l
                && g.x + ENEMY_HALF_WIDTH <= view_r
                && g.x + ENEMY_HALF_WIDTH <= stop_x
                && g.x > front,
            "rider {k} at {} off the lane (view {view_l}..{view_r}, front {front}, stop {stop_x})",
            g.x
        );
        let nearest = heroes
            .iter()
            .min_by(|a, b| (*a - g.x).abs().total_cmp(&(*b - g.x).abs()))
            .copied()
            .unwrap_or(g.x);
        let toward: i8 = if nearest >= g.x { 1 } else { -1 };
        ensure!(g.facing == toward, "rider {k} faces {}", g.facing);
        spots[k] = g.x;
    }
    Ok(spots)
}

/// `dz_crew_bounded` (DOZR-06). The brace pilot disables the machine and Carl
/// runs to the exit: until the disable both riders ride, on the disable tick
/// they land at the wheel front + 32 and + 72, inside the view and the lane,
/// facing the nearest hero, lying for 45 ticks; at most `max_living` (3) enemies
/// live on any tick and nobody is hurt. A second run puts the heroes on the
/// spots on the eve of the disable: rider A is nudged twice (+ 96, the last
/// spot taken anyway is free here) and rider B once (+ 104).
pub fn dz_crew_bounded(root: &Path) -> Result<(u64, [f32; 4]), String> {
    let mid = brace_release_r(root)?;
    // The plain landing.
    let (mut st, _) = brace_pilot(root, mid)?;
    let exit_x = exit_x(&st);
    let cap = max_living(&st);
    ensure!(cap == 3, "max_living {cap}");
    let mut disabled_on = None;
    let mut spots = [0.0; 2];
    for _ in 0..900 {
        if st.sim.scene.world.complete {
            break;
        }
        let carl = st.sim.scene.world.hero.x;
        let was_disabled = st.dozer().state == DozerState::Disabled;
        st.step(RawInput {
            move_x: if stopped(&st) && carl < exit_x {
                1.0
            } else {
                0.0
            },
            ..RawInput::default()
        });
        let n = living(&st.sim);
        ensure!(n <= cap, "{n} living enemies on {}", st.tick());
        let disabled = st.dozer().state == DozerState::Disabled;
        if !disabled {
            ensure!(
                all_carried(&st.sim.combat),
                "a rider left before the disable on {}",
                st.tick()
            );
        } else if !was_disabled {
            spots = check_landing(&st)?;
            let front = st.dozer().wheel_front();
            ensure!(
                spots == [front + LAND_DX[0], front + LAND_DX[1]],
                "the riders landed at {spots:?}, the front is {front}"
            );
            disabled_on = Some(st.tick());
        }
        if let Some(t) = disabled_on {
            let lying = st.tick() < t + DOWN_TICKS;
            for (k, &i) in RIDERS.iter().enumerate() {
                let g = &st.sim.combat.enemies[i];
                ensure!(
                    (g.mode == GoblinMode::Downed) == lying,
                    "rider {k} is {:?} {} ticks after the landing",
                    g.mode,
                    st.tick() - t
                );
            }
        }
    }
    ensure!(disabled_on.is_some(), "the machine was never disabled");
    ensure!(st.sim.scene.world.complete, "the room never completed");
    ensure!(st.hurts() == 0, "{} hurts", st.hurts());
    let plain = st.hash();
    // The nudges: Carl on rider A's spot, Donut on its first nudge.
    let (mut st, _) = brace_pilot(root, mid)?;
    to_eve_of_disable(&mut st)?;
    let front = st.dozer().wheel_front();
    st.put(HeroId::Carl, front + LAND_DX[0]);
    st.put(HeroId::Donut, front + LAND_DX[0] + 32.0);
    st.step_slots([RawInput::NONE; 2]);
    let heroes = [st.sim.scene.world.hero.x, st.sim.scene.world.donut_body.x];
    ensure!(
        heroes == [front + 32.0, front + 64.0],
        "the heroes moved to {heroes:?} (front {front})"
    );
    let nudged = check_landing(&st)?;
    ensure!(
        nudged == [front + 96.0, front + 104.0],
        "the nudged riders landed at {nudged:?}, the front is {front}"
    );
    ensure!(living(&st.sim) <= cap, "too many living enemies");
    Ok((
        fold([plain, st.hash()]),
        [spots[0], spots[1], nudged[0], nudged[1]],
    ))
}

/// `dz_respawn_after_disable` (DOZR-07): after the disable and the landing,
/// rider B is defeated (the fixture's direct hook) and the checkpoint respawn
/// runs: the machine is dormant with its pristine state, the frame hangs again
/// with no remnant, rider A rides again and rider B stays dead. Nothing changes
/// over the next 200 idle ticks.
pub fn dz_respawn_after_disable(root: &Path) -> Result<u64, String> {
    let (sim, _) = start(root, Mode::CarlSolo, 0)?;
    let pristine = dozer_hash(dozer_of(&sim));
    let mid = brace_release_r(root)?;
    let (mut st, _) = brace_pilot(root, mid)?;
    to_eve_of_disable(&mut st)?;
    st.step_slots([RawInput::NONE; 2]);
    check_landing(&st)?;
    ensure!(remnant_down(&st), "no remnant after the landing");
    let b = &mut st.sim.combat.enemies[RIDERS[1]];
    b.health = 0;
    b.mode = GoblinMode::Corpse;
    let b_x = b.x;
    st.sim.combat.respawn(&mut st.sim.scene);
    for k in 0..=200 {
        if k > 0 {
            st.step_slots([RawInput::NONE; 2]);
        }
        let c = &st.sim.combat;
        let b = &c.enemies[RIDERS[1]];
        ensure!(
            c.is_carried(RIDERS[0]),
            "rider A is not aboard {k} ticks after the respawn"
        );
        ensure!(
            !c.is_carried(RIDERS[1]) && !b.alive() && b.x == b_x,
            "rider B came back {k} ticks after the respawn"
        );
        ensure!(
            matches!(st.dozer().state, DozerState::Dormant { .. }),
            "the machine is {:?}",
            st.dozer().state
        );
        ensure!(
            frame_state(&st.sim) == MassState::Supported && !remnant_down(&st),
            "the frame is {:?}, remnant {}",
            frame_state(&st.sim),
            remnant_down(&st)
        );
        if k == 0 {
            ensure!(
                dozer_hash(st.dozer()) == pristine,
                "the respawn left the machine {:?} at {}",
                st.dozer().state,
                st.dozer().o
            );
        }
    }
    Ok(st.hash())
}

/// The machine's hash, the heroes' columns and the riders' carried flags and
/// states of each tick of a `run` plan.
type CrewTick = (u64, u32, u32, [bool; 2], u64);

fn crew_trace(st: &mut Stage, ticks: u32, exit_x: f32) -> Vec<CrewTick> {
    let mut plan = PlanState::default();
    let lead = st.controlled();
    let mut out = Vec::new();
    for _ in 0..ticks {
        let raw = plan_input(Plan::Run, Vary::BASE, &st.sim, lead, &mut plan, exit_x);
        st.step(raw);
        let c = &st.sim.combat;
        let mut riders = Vec::new();
        for &i in &RIDERS {
            c.enemies[i].mix_into(&mut |v| riders.push(v));
        }
        out.push((
            dozer_hash(st.dozer()),
            st.sim.scene.world.hero.x.to_bits(),
            st.sim.scene.world.donut_body.x.to_bits(),
            RIDERS.map(|i| c.is_carried(i)),
            fold(riders),
        ));
    }
    out
}

/// A defeat retry from the brace pilot, `disabled` ticks after the disable
/// (`None`: on the stun tick): the machine is pristine, the frame hangs, no
/// remnant, both riders aboard, and the next attempt replays a fresh one tick
/// for tick (the machine, the heroes and the riders).
fn retry_from(root: &Path, disabled: Option<u32>) -> Result<u64, String> {
    let (sim, _) = start(root, Mode::CarlSolo, 0)?;
    let pristine = dozer_hash(dozer_of(&sim));
    let mid = brace_release_r(root)?;
    let (mut st, _) = brace_pilot(root, mid)?;
    match disabled {
        None => {
            for _ in 0..300 {
                if matches!(st.dozer().state, DozerState::ImpactStunned { .. }) {
                    break;
                }
                st.step_slots([RawInput::NONE; 2]);
            }
            ensure!(
                matches!(st.dozer().state, DozerState::ImpactStunned { .. }),
                "never stunned"
            );
            ensure!(all_carried(&st.sim.combat), "a rider left on the stun");
        }
        Some(after) => {
            to_eve_of_disable(&mut st)?;
            for _ in 0..=after {
                st.step_slots([RawInput::NONE; 2]);
            }
            ensure!(
                RIDERS.iter().all(|&i| !st.sim.combat.is_carried(i)),
                "the riders never landed"
            );
        }
    }
    ensure!(remnant_down(&st), "no remnant before the retry");
    st.sim.combat.restart(&mut st.sim.scene);
    st.sim.party.restart();
    ensure!(
        dozer_hash(st.dozer()) == pristine,
        "the retry left the machine {:?} at {}",
        st.dozer().state,
        st.dozer().o
    );
    ensure!(
        frame_state(&st.sim) == MassState::Supported,
        "the frame is {:?}",
        frame_state(&st.sim)
    );
    ensure!(!remnant_down(&st), "the remnant survived the retry");
    ensure!(all_carried(&st.sim.combat), "a rider is not aboard");
    let mut fresh = Stage::new(
        root,
        Mode::TwoPlayer,
        Loadout::Story.flags(),
        |_| {},
        192.0,
        160.0,
    )?;
    let exit_x = exit_x(&fresh);
    let ticks = 400;
    let a = crew_trace(&mut fresh, ticks, exit_x);
    let b = crew_trace(&mut st, ticks, exit_x);
    if let Some(i) = a.iter().zip(&b).position(|(x, y)| x != y) {
        return Err(format!(
            "the retry differs from a fresh attempt at tick {}: {:?} vs {:?}",
            i + 1,
            a[i],
            b[i]
        ));
    }
    Ok(fold([fresh.hash(), st.hash()]))
}

/// `dz_retry_stunned` (DOZR-07, #287 review F3): a defeat retry on the stun.
pub fn dz_retry_stunned(root: &Path) -> Result<u64, String> {
    retry_from(root, None)
}

/// `dz_retry_disabled` (DOZR-07, #287 review F3): a defeat retry 10 ticks
/// after the disable, the riders on the floor.
pub fn dz_retry_disabled(root: &Path) -> Result<u64, String> {
    retry_from(root, Some(10))
}

/// The ticks a lingering leader waits after the disable before it runs.
pub const LINGER_TICKS: u64 = 240;

/// The report of one companion crew pilot.
#[derive(Debug, Clone, PartialEq)]
pub struct CrewReport {
    pub mode: Mode,
    pub seed: u64,
    pub release_r: u64,
    pub disabled_on: Option<u64>,
    pub complete_tick: Option<u64>,
    pub max_stuck: [u32; 2],
    pub max_living: usize,
    pub hurts: usize,
    pub riders_defeated: usize,
    pub hash: u64,
}

impl CrewReport {
    pub fn name(&self) -> String {
        format!("dz_crew_{}_s{}", self.mode.name(), self.seed)
    }

    pub fn check(&self) -> Result<(), String> {
        let fails = [
            (self.disabled_on.is_none(), "the machine was never disabled"),
            (self.complete_tick.is_none(), "the room never completed"),
            (
                self.max_stuck.iter().any(|s| *s > STUCK_LIMIT),
                "a hero was stuck",
            ),
            (self.max_living > 3, "more than 3 living enemies"),
        ];
        let failed: Vec<&str> = fails.iter().filter(|(f, _)| *f).map(|(_, m)| *m).collect();
        if failed.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{}: {}: {}",
                self.name(),
                failed.join(", "),
                self.line()
            ))
        }
    }

    pub fn line(&self) -> String {
        format!(
            "DZ_CREW name={} release_r={} disabled={:?} complete={:?} max_stuck={:?} \
             max_living={} hurts={} riders_defeated={} hash={:016x}",
            self.name(),
            self.release_r,
            self.disabled_on,
            self.complete_tick,
            self.max_stuck,
            self.max_living,
            self.hurts,
            self.riders_defeated,
            self.hash
        )
    }
}

/// A companion crew pilot (DR-C acceptance): a solo mode under CF02 seed
/// `seed`. The leader waits at the brace (x 832, facing the frame), the
/// companion starts in the tunnel (x 1150). The frame is released directly on
/// R = the middle of the window + (`seed` − 4), credited to the leader. Once
/// the machine stops, an even seed runs to the exit at once and an odd one
/// lingers [`LINGER_TICKS`] after the disable first (the riders reach it). The
/// run must disable the machine, complete, keep at most 3 living enemies and
/// leave no hero stuck more than [`STUCK_LIMIT`] ticks (the K5 stuck rule).
pub fn dz_crew_companion(root: &Path, mode: Mode, seed: u64) -> Result<CrewReport, String> {
    ensure!(mode != Mode::TwoPlayer, "a solo mode");
    let mid = brace_release_r(root)?;
    let release_r = (mid + seed).checked_sub(4).ok_or("R below 0")?;
    let (carl_x, donut_x) = match mode {
        Mode::DonutSolo => (1150.0, WAIT_X),
        _ => (WAIT_X, 1150.0),
    };
    let mut st = Stage::seeded(
        root,
        mode,
        (Loadout::Story.flags(), DonutSetup::unlocked(8)),
        |_| {},
        (carl_x, donut_x),
        seed,
    )?;
    let lead = st.controlled();
    st.sim.scene.world.body_mut(lead).facing = -1;
    let exit_x = exit_x(&st);
    let chase0 = to_chase(&mut st)?;
    while st.tick() + 1 < chase0 + release_r {
        st.step(RawInput::NONE);
    }
    blast_release(&mut st, crate::shore::PLAYER_ONE);
    let mut last_x = [st.sim.scene.world.hero.x, st.sim.scene.world.donut_body.x];
    let (mut stuck, mut max_stuck) = ([0u32; 2], [0u32; 2]);
    let (mut disabled_on, mut complete_tick, mut running) = (None, None, false);
    let mut max_seen = 0usize;
    for _ in 0..RUN_TICKS {
        if disabled_on.is_none() && st.dozer().state == DozerState::Disabled {
            disabled_on = Some(st.tick());
        }
        let linger_done =
            seed.is_multiple_of(2) || disabled_on.is_some_and(|t| st.tick() >= t + LINGER_TICKS);
        running |= stopped(&st) && linger_done;
        let x = st.sim.scene.world.body(lead).x;
        let walk = running && x < exit_x;
        st.step(RawInput {
            move_x: if walk { 1.0 } else { 0.0 },
            ..RawInput::default()
        });
        max_seen = max_seen.max(living(&st.sim));
        if disabled_on.is_none() && st.dozer().state != DozerState::Disabled {
            ensure!(
                all_carried(&st.sim.combat),
                "{} s{seed}: a rider left before the disable",
                mode.name()
            );
        }
        let leader = st.controlled();
        for id in HeroId::ALL {
            let i = id.index();
            let x = st.sim.scene.world.body(id).x;
            let still = (x - last_x[i]).abs() < 0.1;
            let counts = if id == leader {
                walk && still
            } else {
                let lx = st.sim.scene.world.body(leader).x;
                let leader_moved = (lx - last_x[leader.index()]).abs() >= 0.1;
                still && (x - lx).abs() > APART_PX && leader_moved
            };
            stuck[i] = if counts { stuck[i] + 1 } else { 0 };
            max_stuck[i] = max_stuck[i].max(stuck[i]);
        }
        for id in HeroId::ALL {
            last_x[id.index()] = st.sim.scene.world.body(id).x;
        }
        if st.sim.scene.world.complete {
            complete_tick = Some(st.tick());
            break;
        }
    }
    let riders_defeated = st
        .sim
        .combat
        .events
        .iter()
        .filter(|e| {
            matches!(e, CombatEvent::EnemyDefeated { target, .. } if RIDERS.contains(&(*target as usize)))
        })
        .count();
    Ok(CrewReport {
        mode,
        seed,
        release_r,
        disabled_on,
        complete_tick,
        max_stuck,
        max_living: max_seen,
        hurts: st.hurts(),
        riders_defeated,
        hash: st.hash(),
    })
}
