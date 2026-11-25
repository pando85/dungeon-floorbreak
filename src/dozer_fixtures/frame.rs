//! DR-B fixtures: the stop frame (`docs/01-design/enemy-gameplay-dozer.md`,
//! sections 6 and 10, DOZR-02 to 04; DOZR-01 is [`super::K5Report::check`]).
//!
//! The release tick R is counted as [`crate::dozer::release_window`] counts it,
//! from the landing: a frame released on R lands (warning 30, fall 8) on world
//! tick `chase0` + R + 37, `chase0` being the machine's first committed-chase
//! tick. A hero's release (a jab, a missile, a blast) logged on world tick `t`
//! is R = `t` − `chase0`; a fixture's direct stimulus applied between two
//! ticks counts from the next one (it lands 38 ticks after the tick it is
//! logged on).

use super::*;
use crate::combat::PropPhase;
use crate::dozer::{ReleaseWindow, StopFrame};
use crate::env_hazard::HazardClass;
use crate::env_prop::{MassState, Stimulus, StimulusKind};

/// The stop frame's prop index in room 3's sidecar.
pub const FRAME: usize = f01_dozer::STOP_FRAME_PROP;

/// The ticks from a release to its landing in room 3 (30 + 8 − 1).
pub const LAND_DELAY: u64 = 37;

/// Carl's dynamite: the ticks from the button press to the blast (the throw's
/// wind-up, the flight and the fuse).
pub const DYNAMITE_PRESS_TO_BLAST: u64 = 191;

/// A direct stimulus lands this many ticks after the tick it is logged on.
const DIRECT_LAND_DELAY: u64 = LAND_DELAY + 1;

/// The spot of a dynamite or missile user: back from the brace so the stick
/// lands under the target (Carl's throw carries about 95 px).
pub const THROW_X: f32 = 900.0;

/// The ticks between Carl's two jab presses at the brace.
pub const JAB_PERIOD: u64 = 6;
/// The ticks from the second jab's press to the release it causes.
pub const JAB_TO_RELEASE: u64 = 9;

/// The ticks from Donut's second missile press to the release (the cast and
/// the bolt's flight from x 900 to the brace).
pub const MISSILE_PRESS_TO_RELEASE: u64 = 13;
/// The ticks between her two presses (one cast ends before the next starts).
pub const MISSILE_PERIOD: u64 = 49;

/// Donut's player id under two pads (pad 2).
pub const DONUT_PLAYER: u32 = 1;

/// The frame's state.
pub fn frame_state(sim: &PartyFixtureSim) -> MassState {
    sim.combat
        .env()
        .and_then(|e| e.mass_state(FRAME))
        .expect("room 3 has its stop frame")
}

/// A direct release: a blast of weight 2 credited to `owner`.
pub fn blast_release(st: &mut Stage, owner: u32) {
    let tick = st.tick();
    st.sim.combat.env_stimulate(
        FRAME,
        Stimulus::Blast {
            blast_id: 9000,
            owner: Some(owner),
            depth: 0,
        },
        tick,
    );
}

/// Room 3's stop frame as [`crate::dozer::release_window`] reads it, from the
/// sidecar (the footprint, the warning) and the machine's site.
pub fn stop_frame(st: &Stage) -> Result<StopFrame, String> {
    let fall = st
        .run
        .loaded
        .encounter
        .hazards
        .iter()
        .find(|h| h.owner == Some(FRAME) && h.spec.class == HazardClass::FallingMass)
        .ok_or("room 3's frame has its falling mass")?;
    let f = fall.spec.rect;
    let land =
        u32::from(fall.spec.telegraph_ticks()) + u32::from(crate::env_prop::DEFAULT_FALL_TICKS) - 1;
    ensure!(u64::from(land) == LAND_DELAY, "the landing delay is {land}");
    let frame = StopFrame {
        footprint_x0: f.x as f32,
        footprint_x1: f.right() as f32,
        land_delay_ticks: land,
        emerge_front_x: st.dozer().site.emerge_front_x as f32,
    };
    // The live reading (the `dozer:brace` room pilot's) is the sidecar's.
    ensure!(
        f01_dozer::stop_frame_of(&st.sim.combat) == Some(frame),
        "the live stop frame {:?} differs from the sidecar's {frame:?}",
        f01_dozer::stop_frame_of(&st.sim.combat)
    );
    Ok(frame)
}

/// The helper's window and per-R hits for a hero waiting at `hero_x`.
pub fn helper(st: &Stage, hero_x: f32) -> Result<(ReleaseWindow, Vec<bool>), String> {
    let frame = stop_frame(st)?;
    let d = st.dozer();
    let w = crate::dozer::release_window(
        &d.profile,
        &frame,
        hero_x,
        u32::from(d.tuning.dzq23_melee_recovery_ticks),
    );
    Ok((w, crate::dozer::release_hits(&d.profile, &frame, hero_x)))
}

/// The middle of the helper's useful window.
fn mid_window(w: &ReleaseWindow) -> Result<u64, String> {
    let (a, b) = (
        w.first_tick.ok_or("no window")?,
        w.last_tick.ok_or("no window")?,
    );
    Ok(u64::from(a + b) / 2)
}

/// Step `st` with nothing pressed until the machine commits to the chase;
/// returns the first chase tick.
fn to_chase(st: &mut Stage) -> Result<u64, String> {
    for _ in 0..600 {
        st.step_slots([RawInput::NONE; 2]);
        if st.dozer().state == DozerState::CommittedChase {
            return Ok(st.tick());
        }
    }
    Err("the machine never committed".into())
}

/// The world tick of the frame's first `phase` event and the player credited.
fn phase_tick(st: &Stage, phase: PropPhase) -> Option<(u64, Option<u32>)> {
    st.sim.combat.events.iter().find_map(|e| match *e {
        CombatEvent::PropPhase {
            tick,
            prop,
            phase: p,
            player,
        } if usize::from(prop) == FRAME && p == phase => Some((tick, player)),
        _ => None,
    })
}

/// The frame's hits by kind and player.
fn prop_hits(st: &Stage) -> Vec<(StimulusKind, Option<u32>)> {
    st.sim
        .combat
        .events
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::PropHit {
                prop, kind, player, ..
            } if usize::from(prop) == FRAME => Some((kind, player)),
            _ => None,
        })
        .collect()
}

/// The stuns so far: `(tick, front, by)`.
fn stuns(st: &Stage) -> Vec<(u64, i32, Option<u32>)> {
    st.events
        .iter()
        .filter_map(|(t, e)| match *e {
            DozerEvent::Stunned { front_x, by } => Some((*t, front_x, by)),
            _ => None,
        })
        .collect()
}

/// The glances so far: `(tick, front)`.
fn glances(st: &Stage) -> Vec<(u64, i32)> {
    st.events
        .iter()
        .filter_map(|(t, e)| match *e {
            DozerEvent::Glance { front_x } => Some((*t, front_x)),
            _ => None,
        })
        .collect()
}

/// The exit's centre (the pilots run to it).
fn exit_x(st: &Stage) -> f32 {
    let exit = st.run.loaded.encounter.exit;
    (exit.x + exit.w / 2) as f32
}

/// After a release: the `runner` holds until the machine is stunned, then runs
/// to the exit. Checks the reversal: one stun on the landing tick credited to
/// `by`, the disable exactly the stunned ticks later, no glance, no hurt, the
/// room complete. Returns the release's R.
fn check_disable(st: &mut Stage, chase0: u64, by: u32, runner: HeroId) -> Result<u64, String> {
    let stunned_ticks = u64::from(st.dozer().tuning.dzq15_stunned_ticks);
    let exit_x = exit_x(st);
    for _ in 0..900 {
        if st.sim.scene.world.complete {
            break;
        }
        let stopped = matches!(
            st.dozer().state,
            DozerState::ImpactStunned { .. } | DozerState::Disabled
        );
        let run = RawInput {
            move_x: if stopped && st.sim.scene.world.body(runner).x < exit_x {
                1.0
            } else {
                0.0
            },
            ..RawInput::default()
        };
        st.step_slots(match runner {
            HeroId::Carl => [run, RawInput::NONE],
            HeroId::Donut => [RawInput::NONE, run],
        });
    }
    let (released, credit) =
        phase_tick(st, PropPhase::Released).ok_or("the frame never released")?;
    ensure!(credit == Some(by), "the release is credited to {credit:?}");
    let r = released
        .checked_sub(chase0)
        .ok_or("released before the chase")?;
    let (landed, _) = phase_tick(st, PropPhase::Landed).ok_or("the frame never landed")?;
    ensure!(
        landed == released + LAND_DELAY,
        "released {released}, landed {landed}"
    );
    let s = stuns(st);
    ensure!(s.len() == 1, "stuns {s:?}");
    let (stun_tick, _, stun_by) = s[0];
    ensure!(
        stun_tick == landed,
        "the stun on {stun_tick}, the landing on {landed}"
    );
    ensure!(stun_by == Some(by), "the stun is credited to {stun_by:?}");
    let disabled: Vec<u64> = st
        .events
        .iter()
        .filter(|(_, e)| matches!(e, DozerEvent::Disabled))
        .map(|(t, _)| *t)
        .collect();
    ensure!(
        disabled == [stun_tick + stunned_ticks],
        "disabled on {disabled:?}, the stun on {stun_tick}"
    );
    ensure!(
        st.dozer().state == DozerState::Disabled,
        "the machine is {:?}",
        st.dozer().state
    );
    ensure!(glances(st).is_empty(), "a glance on a hit");
    ensure!(st.hurts() == 0, "{} hurts", st.hurts());
    ensure!(st.sim.scene.world.complete, "the room never completed");
    Ok(r)
}

/// The sweep's report.
#[derive(Debug, Clone, PartialEq)]
pub struct SweepReport {
    /// The release ticks evaluated (the helper's domain).
    pub evaluated: u32,
    /// The release ticks whose landing stunned the machine: first, last, count.
    pub sim_hits: (Option<u32>, Option<u32>, u32),
    pub window: ReleaseWindow,
    /// The rear-hull landings (glances) in the domain.
    pub glances: u32,
}

/// `dz_frame_window_sweep` (DOZR-03): the real chase of Carl waiting at the
/// brace (x 832, facing the frame; Donut in the tunnel, two pads). For every
/// release tick R of the helper's domain, a copy of the run released on R (a
/// weight-2 stimulus credited to Carl) lands on R + 37 and stuns the machine
/// on its landing tick exactly when [`crate::dozer::release_hits`] says so:
/// the sweep's hit window equals the helper's exactly.
pub fn dz_frame_window_sweep(root: &Path) -> Result<(u64, SweepReport), String> {
    let mut st = Stage::new(
        root,
        Mode::TwoPlayer,
        Loadout::Story.flags(),
        |_| {},
        WAIT_X,
        1150.0,
    )?;
    st.sim.scene.world.body_mut(HeroId::Carl).facing = -1;
    let (window, hits) = helper(&st, WAIT_X)?;
    // Walk to the pre-state of R = 0 (one tick before the first chase tick).
    let chase0 = to_chase(&mut st.clone())?;
    while st.tick() + 1 < chase0 {
        st.step(RawInput::NONE);
    }
    let mut hashes = Vec::new();
    let mut sim_hits = (None, None, 0u32);
    let mut glance_count = 0;
    for (r, &hit) in hits.iter().enumerate() {
        let r = r as u32;
        ensure!(
            frame_state(&st.sim) == MassState::Supported,
            "R {r}: the baseline touched the frame"
        );
        let mut c = st.clone();
        blast_release(&mut c, crate::shore::PLAYER_ONE);
        let release = c.tick();
        for _ in 0..DIRECT_LAND_DELAY {
            c.step(RawInput::NONE);
        }
        let (landed, _) = phase_tick(&c, PropPhase::Landed).ok_or(format!("R {r}: no landing"))?;
        ensure!(
            landed == release + DIRECT_LAND_DELAY && landed == chase0 + u64::from(r) + LAND_DELAY,
            "R {r}: released {release}, landed {landed}"
        );
        let s = stuns(&c);
        let stunned = !s.is_empty();
        ensure!(
            stunned == hit,
            "R {r}: the helper says hit={hit}, the sim stunned={stunned} (front {:.2})",
            c.dozer().wheel_front()
        );
        if stunned {
            ensure!(
                s.len() == 1 && s[0].0 == landed && s[0].2 == Some(crate::shore::PLAYER_ONE),
                "R {r}: stuns {s:?}, landed {landed}"
            );
            sim_hits.0.get_or_insert(r);
            sim_hits.1 = Some(r);
            sim_hits.2 += 1;
        }
        let g = glances(&c);
        ensure!(g.len() <= 1, "R {r}: glances {g:?}");
        ensure!(!(stunned && !g.is_empty()), "R {r}: a glance on a hit");
        glance_count += g.len() as u32;
        hashes.push(c.hash());
        st.step(RawInput::NONE);
    }
    ensure!(window.evaluated as usize == hits.len(), "the domain");
    ensure!(
        sim_hits
            == (
                window.hit_first_tick,
                window.hit_last_tick,
                window.hit_ticks
            ),
        "sim {sim_hits:?}, helper {window:?}"
    );
    ensure!(st.hurts() == 0, "the baseline was hurt");
    hashes.push(st.hash());
    Ok((
        fold(hashes),
        SweepReport {
            evaluated: window.evaluated,
            sim_hits,
            window,
            glances: glance_count,
        },
    ))
}

/// `dz_frame_early_miss` (DOZR-02): the frame released with the wheel front at
/// 400 lands ahead of the machine: no stun, no glance; the remnant lies in the
/// lane and the chasing machine rolls over it (tread overrun) on its walking
/// line; Carl waits at the brace, runs at the close call and the room
/// completes untouched.
pub fn dz_frame_early_miss(root: &Path) -> Result<u64, String> {
    let mut st = Stage::new(
        root,
        Mode::TwoPlayer,
        Loadout::Story.flags(),
        chase_at(400.0),
        WAIT_X,
        1150.0,
    )?;
    st.sim.scene.world.body_mut(HeroId::Carl).facing = -1;
    blast_release(&mut st, crate::shore::PLAYER_ONE);
    let release = st.tick();
    let floor = st.dozer().floor_y;
    let exit_x = exit_x(&st);
    let mut overran = false;
    let mut running = false;
    for _ in 0..900 {
        if st.sim.scene.world.complete {
            break;
        }
        let d = st.dozer();
        let carl = st.sim.scene.world.body(HeroId::Carl).x;
        running |= carl - HERO_HALF_WIDTH - d.wheel_front() <= CLOSE_GAP_PX;
        st.step(RawInput {
            move_x: if running && carl < exit_x { 1.0 } else { 0.0 },
            ..RawInput::default()
        });
        let d = st.dozer();
        ensure!(d.floor_y == floor, "the machine left its walking line");
        if d.state == DozerState::CommittedChase && frame_state(&st.sim) == MassState::Landed {
            // The hull over the remnant's x span (640..720).
            let hull = d.push_box();
            overran |= hull.left < 720.0 && hull.right() > 640.0;
        }
    }
    let (landed, _) = phase_tick(&st, PropPhase::Landed).ok_or("no landing")?;
    ensure!(
        landed == release + DIRECT_LAND_DELAY,
        "released {release}, landed {landed}"
    );
    ensure!(stuns(&st).is_empty(), "stuns {:?}", stuns(&st));
    ensure!(glances(&st).is_empty(), "glances {:?}", glances(&st));
    ensure!(overran, "the machine never rolled over the remnant");
    // Section 7.3's remnant case: a rescue `place` over the landed remnant
    // stands on its top, not inside it.
    let remnant = st
        .sim
        .scene
        .world
        .solids
        .iter()
        .filter(|b| b.intact && b.rect.left < 720.0 && b.rect.right() > 640.0)
        .filter(|b| b.rect.bottom <= floor + 0.5)
        .map(|b| b.rect.top())
        .fold(None, |a: Option<f32>, t| Some(a.map_or(t, |a| a.max(t))))
        .ok_or("no remnant solid in the lane")?;
    let foot = crate::dozer::standing_foot_y(&st.sim.scene.world.solids, 680.0, floor);
    ensure!(
        remnant > floor && foot == remnant,
        "a place on the remnant stands at {foot}, its top is {remnant}, the floor {floor}"
    );
    ensure!(st.hurts() == 0, "{} hurts", st.hurts());
    ensure!(st.sim.scene.world.complete, "the room never completed");
    Ok(st.hash())
}

/// The brace pilot (section 10): Carl waits at the brace (x 832, facing the
/// frame) and jabs twice so the release is on R = `release_r`. Donut waits
/// in the tunnel (two pads). Returns the stage on the release tick and the
/// first chase tick.
pub fn brace_pilot(root: &Path, release_r: u64) -> Result<(Stage, u64), String> {
    let mut st = Stage::new(
        root,
        Mode::TwoPlayer,
        Loadout::Story.flags(),
        |_| {},
        WAIT_X,
        1150.0,
    )?;
    st.sim.scene.world.body_mut(HeroId::Carl).facing = -1;
    let chase0 = to_chase(&mut st)?;
    let second = (chase0 + release_r)
        .checked_sub(JAB_TO_RELEASE)
        .ok_or("too early")?;
    let first = second - JAB_PERIOD;
    ensure!(first > st.tick(), "the jabs are before the chase");
    while st.tick() < second + JAB_TO_RELEASE {
        let k = st.tick() + 1;
        st.step(RawInput {
            attack: k == first || k == second,
            ..RawInput::default()
        });
    }
    Ok((st, chase0))
}

/// The R of the brace pilot: the middle of the useful window of a hero
/// waiting at the brace.
pub fn brace_release_r(root: &Path) -> Result<u64, String> {
    let st = Stage::new(
        root,
        Mode::TwoPlayer,
        Loadout::Story.flags(),
        |_| {},
        WAIT_X,
        1150.0,
    )?;
    mid_window(&helper(&st, WAIT_X)?.0)
}

/// `dz_frame_disable_once` (DOZR-03, the brace pilot): Carl's two jabs at the
/// brace release the frame in the middle of the useful window; it lands on the
/// front collider: one stun on the landing tick credited to Carl, the disable
/// 48 ticks later, and Carl runs to the exit untouched.
pub fn dz_frame_disable_once(root: &Path) -> Result<(u64, u64), String> {
    let mid = brace_release_r(root)?;
    let (mut st, chase0) = brace_pilot(root, mid)?;
    let r = check_disable(&mut st, chase0, crate::shore::PLAYER_ONE, HeroId::Carl)?;
    let hits = prop_hits(&st);
    ensure!(
        hits.len() == 2
            && hits
                .iter()
                .all(|&(k, p)| k == StimulusKind::MeleeHit && p == Some(crate::shore::PLAYER_ONE)),
        "the frame's hits {hits:?}"
    );
    ensure!(r == mid, "released on R {r}, aimed at {mid}");
    Ok((st.hash(), r))
}

/// `dz_frame_dynamite` (DOZR-04): Carl, back from the brace, throws one stick:
/// the blast releases the frame at once (weight 2), credited to him, and the
/// landing disables the machine. Without the dynamite in the pin the same
/// press throws nothing and the frame stays up.
pub fn dz_frame_dynamite(root: &Path) -> Result<(u64, u64), String> {
    let mut hashes = Vec::new();
    let mut r_out = 0;
    for armed in [true, false] {
        let (carl, donut) = Loadout::Story.flags();
        // Without the capability: Carl's program off, so the room's pin
        // holds no dynamite for him.
        let flags = (if armed { carl } else { CarlProgFlags::OFF }, donut);
        let mut st = Stage::new(root, Mode::CarlSolo, flags, |_| {}, THROW_X, 1150.0)?;
        st.sim.park_uncontrolled();
        st.sim.scene.world.body_mut(HeroId::Carl).facing = -1;
        let mid = mid_window(&helper(&st, THROW_X)?.0)?;
        let chase0 = to_chase(&mut st.clone())?;
        let press = (chase0 + mid)
            .checked_sub(DYNAMITE_PRESS_TO_BLAST)
            .ok_or("too late")?;
        ensure!(press > st.tick(), "the press is before tick 1");
        while st.tick() < press + DYNAMITE_PRESS_TO_BLAST {
            let k = st.tick() + 1;
            st.step(RawInput {
                ability: k == press,
                ..RawInput::default()
            });
        }
        if armed {
            let hits = prop_hits(&st);
            ensure!(
                hits == [(StimulusKind::Blast, Some(crate::shore::PLAYER_ONE))],
                "the frame's hits {hits:?}"
            );
            r_out = check_disable(&mut st, chase0, crate::shore::PLAYER_ONE, HeroId::Carl)?;
            ensure!(r_out == mid, "released on R {r_out}, aimed at {mid}");
        } else {
            for _ in 0..LAND_DELAY + 4 {
                st.step(RawInput::NONE);
            }
            let thrown = st
                .sim
                .combat
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::ChargeThrown { .. }));
            ensure!(!thrown, "a stick without the dynamite");
            ensure!(
                frame_state(&st.sim) == MassState::Supported,
                "the frame is {:?}",
                frame_state(&st.sim)
            );
            ensure!(
                prop_hits(&st).is_empty() && stuns(&st).is_empty(),
                "the frame was touched"
            );
        }
        hashes.push(st.hash());
    }
    Ok((fold(hashes), r_out))
}

/// `dz_frame_missile` (DOZR-04): Donut, steered by pad 2 back from the brace,
/// casts two Magic Missiles at it: each counts 1, the second releases the
/// frame, and the release and the disable credit her. Without the biscuit
/// (the missile's capability) the same presses fire nothing at the frame and
/// it stays up.
pub fn dz_frame_missile(root: &Path) -> Result<(u64, u64), String> {
    let mut hashes = Vec::new();
    let mut r_out = 0;
    for armed in [true, false] {
        let (carl, donut) = Loadout::Story.flags();
        let kit = if armed {
            ((carl, donut), DonutSetup::unlocked(8))
        } else {
            ((carl, DonutProgFlags::default()), DonutSetup::NONE)
        };
        let mut st = Stage::with_kit(root, Mode::TwoPlayer, kit, |_| {}, 1150.0, THROW_X)?;
        st.sim.scene.world.body_mut(HeroId::Donut).facing = -1;
        let mid = mid_window(&helper(&st, THROW_X)?.0)?;
        let chase0 = to_chase(&mut st.clone())?;
        let second = (chase0 + mid)
            .checked_sub(MISSILE_PRESS_TO_RELEASE)
            .ok_or("too late")?;
        let first = second - MISSILE_PERIOD;
        ensure!(first > st.tick(), "the presses are before tick 1");
        while st.tick() < second + MISSILE_PRESS_TO_RELEASE {
            let k = st.tick() + 1;
            let p2 = RawInput {
                attack: k == first || k == second,
                ..RawInput::default()
            };
            st.step_slots([RawInput::NONE, p2]);
        }
        if armed {
            let hits = prop_hits(&st);
            ensure!(
                hits.len() == 2
                    && hits
                        .iter()
                        .all(|&(k, p)| k == StimulusKind::Projectile && p == Some(DONUT_PLAYER)),
                "the frame's hits {hits:?}"
            );
            r_out = check_disable(&mut st, chase0, DONUT_PLAYER, HeroId::Donut)?;
            ensure!(r_out == mid, "released on R {r_out}, aimed at {mid}");
        } else {
            for _ in 0..LAND_DELAY + 4 {
                st.step_slots([RawInput::NONE; 2]);
            }
            ensure!(
                frame_state(&st.sim) == MassState::Supported,
                "the frame is {:?}",
                frame_state(&st.sim)
            );
            ensure!(
                prop_hits(&st).is_empty() && stuns(&st).is_empty(),
                "the frame was touched"
            );
        }
        hashes.push(st.hash());
    }
    Ok((fold(hashes), r_out))
}

/// `dz_frame_rear_glance` (DOZR-03): the heroes wait in the tunnel and the
/// frame drops on the chasing machine at front 800: it lands on the rear hull
/// (the wheel front in the doc's [869, 972)), a miss: no stun, exactly one
/// glance on the landing tick, and the machine goes on to its brake and rest.
/// Returns the hash and the glance's front.
pub fn dz_frame_rear_glance(root: &Path) -> Result<(u64, i32), String> {
    let mut st = Stage::new(
        root,
        Mode::TwoPlayer,
        Loadout::Story.flags(),
        chase_at(800.0),
        1100.0,
        1150.0,
    )?;
    blast_release(&mut st, crate::shore::PLAYER_ONE);
    let release = st.tick();
    let mut landing_front = None;
    for _ in 0..240 {
        st.step(RawInput::NONE);
        if landing_front.is_none() && frame_state(&st.sim) == MassState::Landed {
            landing_front = Some(st.dozer().wheel_front());
        }
    }
    let (landed, _) = phase_tick(&st, PropPhase::Landed).ok_or("no landing")?;
    ensure!(
        landed == release + DIRECT_LAND_DELAY,
        "released {release}, landed {landed}"
    );
    let front = landing_front.ok_or("no landing")?;
    ensure!(
        (869.0..972.0).contains(&front),
        "the landing front {front:.2}"
    );
    ensure!(stuns(&st).is_empty(), "stuns {:?}", stuns(&st));
    let g = glances(&st);
    ensure!(
        g.len() == 1 && g[0].0 == landed,
        "glances {g:?}, landed {landed}"
    );
    ensure!(
        escaped_at_rest(st.dozer()),
        "the machine is {:?}",
        st.dozer().state
    );
    ensure!(st.hurts() == 0, "{} hurts", st.hurts());
    Ok((st.hash(), g[0].1))
}
