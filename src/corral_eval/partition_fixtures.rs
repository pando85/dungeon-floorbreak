//! CORRAL-D: the seven partition fixtures of the sheet (section 15,
//! `enemy-gameplay-llama-rush.md`), run on the real party simulation with the
//! room's real sidecar prop (the evaluator's rig: [`build`] with the `intact`,
//! `broken` or `absent` [`PartitionVariant`]).
//!
//! Both heroes are driven (two-player), so Carl's and Donut's raw inputs are
//! exact. A fixture may write its **setup** once, before the first step: the
//! hero columns and a llama's `(cooldown, rush_cooldown)` (to hold B back from
//! a rush that would break the screen when the fixture is about something
//! else), or B's leash (the near miss). After that the only mutation is
//! [`PartyFixtureSim::step_slots`].
//!
//! | fixture | what it pins |
//! | --- | --- |
//! | `corral_partition_rush_break` | B rushes a Donut at 1376: on the contact tick the rush ends in `Partition`, the prop logs `Shove` and `Broken`, B bonks 60 ticks, nobody is hurt, the lane opens and a bolt then reaches B |
//! | `corral_partition_near_miss` | B's leash stops it 4 px short of the face: a miss, the screen stands |
//! | `corral_partition_carl_breaks` | three distinct Carl hits (one per move instance, a long active window counts once) break it on the third; Donut's swat counts the same way |
//! | `corral_partition_missile_blocked` | eight Magic Missile bolts leave it standing and B unhurt |
//! | `corral_partition_glob_blocked` | B's spit at a shelf Donut dies on the screen (the `absent` control hits her) |
//! | `corral_partition_remnant_bolt_passes` | the pre-broken remnant (16 px) lets a floor bolt through to B |
//! | `corral_partition_retry_restores` | a restart or a checkpoint respawn after the break restores the standing screen and its solid |

use std::path::PathBuf;
use std::sync::OnceLock;

use super::policy::PolicyId;
use super::{
    Evasion, HORIZON_TICKS, Kits, PartitionVariant, RunSpec, Seat, Stage, build, load_kits, room,
};
use crate::combat::{CombatEvent, MoveId, PropPhase};
use crate::env_prop::{PartitionState, StimulusKind};
use crate::input::RawInput;
use crate::llama_rush::{RushEnd, RushPhase};
use crate::party::HeroId;
use crate::party_scene::PartyFixtureSim;
use crate::prototype::HERO_HALF_HEIGHT;

/// The sheet's seven fixtures.
pub const FIXTURES: [&str; 7] = [
    "corral_partition_rush_break",
    "corral_partition_near_miss",
    "corral_partition_carl_breaks",
    "corral_partition_missile_blocked",
    "corral_partition_glob_blocked",
    "corral_partition_remnant_bolt_passes",
    "corral_partition_retry_restores",
];

/// The partition's faces (layout x).
const FACE_LEFT: f32 = 1392.0;
/// Llama B (index 0 in the `corral_m2` rig) and the prop (index 0 in the sidecar).
const B: usize = 0;
const PROP: u16 = 0;

fn kits() -> &'static Kits {
    static KITS: OnceLock<Kits> = OnceLock::new();
    KITS.get_or_init(|| {
        load_kits(&PathBuf::from(env!("CARGO_MANIFEST_DIR"))).expect("the corral kits load")
    })
}

/// One fixture's rig: Carl at `carl_x`, Donut at `donut_x` on `rise`, both
/// controlled, B and A with attack phase `(0, 0)`.
fn rig(variant: PartitionVariant, carl_x: f32, donut_x: i32, rise: i32) -> PartyFixtureSim {
    rig_seat(Seat::TwoPlayer, variant, carl_x, donut_x, rise)
}

fn rig_seat(
    seat: Seat,
    variant: PartitionVariant,
    carl_x: f32,
    donut_x: i32,
    rise: i32,
) -> PartyFixtureSim {
    let spec = RunSpec {
        stage: Stage::CorralM2,
        seat,
        policy: PolicyId::NoInteraction,
        evasion: Evasion::Off,
        donut_x,
        donut_rise: rise,
        facing: 1,
        phase: [(0, 0), (0, 0)],
        jump_offset: 0,
        horizon: HORIZON_TICKS,
        partition: variant,
    };
    let mut sim = build(kits(), &spec).expect("the fixture start is legal");
    // Setup (tick 0): Carl's column (a parked Carl stays where `build` put him).
    if seat == Seat::TwoPlayer {
        sim.scene.world.hero.x = carl_x;
    }
    sim
}

/// Hold B back: a rush or spit cooldown too long to run out inside a fixture.
fn hold_llama(sim: &mut PartyFixtureSim, index: usize) {
    let l = &mut sim.combat.llamas[index];
    l.cooldown = 5000;
    if let Some(c) = l.corral.as_mut() {
        c.rush_cooldown = 5000;
    }
}

fn step(sim: &mut PartyFixtureSim, carl: RawInput, donut: RawInput) {
    sim.step_slots([carl, donut]);
}

fn idle(sim: &mut PartyFixtureSim) {
    step(sim, RawInput::NONE, RawInput::NONE);
}

fn press_attack() -> RawInput {
    RawInput {
        attack: true,
        ..RawInput::NONE
    }
}

fn partition_state(sim: &PartyFixtureSim) -> Option<PartitionState> {
    sim.combat
        .env()
        .and_then(|e| e.partition(usize::from(PROP)))
        .map(|p| p.state())
}

fn partition_hits(sim: &PartyFixtureSim) -> u8 {
    sim.combat
        .env()
        .and_then(|e| e.partition(usize::from(PROP)))
        .map_or(0, |p| p.hits())
}

/// The wall and the remnant blocks of the synced env range, as `(wall intact,
/// remnant intact)`.
fn solids_of(sim: &PartyFixtureSim) -> (bool, bool) {
    let env = sim.combat.env().expect("the env runtime");
    let start = env.solid_range().start;
    let mut wall = false;
    let mut remnant = false;
    for (i, s) in env.solids().iter().enumerate() {
        let intact = sim.scene.world.solids[start + i].intact;
        match s.kind {
            crate::combat_scene::EnvSolidKind::PartitionWall => wall = intact,
            crate::combat_scene::EnvSolidKind::Remnant => remnant = intact,
            _ => {}
        }
    }
    (wall, remnant)
}

fn events(sim: &PartyFixtureSim) -> &[CombatEvent] {
    &sim.combat.events
}

fn rush_end(sim: &PartyFixtureSim) -> Option<(u64, RushEnd)> {
    events(sim).iter().find_map(|e| match *e {
        CombatEvent::LlamaRushEnd {
            tick,
            llama: 0,
            end,
        } => Some((tick, end)),
        _ => None,
    })
}

fn prop_hits(sim: &PartyFixtureSim) -> Vec<(u64, StimulusKind, u8)> {
    events(sim)
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::PropHit {
                tick,
                prop: PROP,
                kind,
                progress,
                ..
            } => Some((tick, kind, progress)),
            _ => None,
        })
        .collect()
}

fn broken_ticks(sim: &PartyFixtureSim) -> Vec<u64> {
    events(sim)
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::PropPhase {
                tick,
                prop: PROP,
                phase: PropPhase::Broken,
                ..
            } => Some(tick),
            _ => None,
        })
        .collect()
}

fn hero_hurts(sim: &PartyFixtureSim) -> usize {
    events(sim)
        .iter()
        .filter(|e| matches!(e, CombatEvent::HeroHurt { .. }))
        .count()
}

fn bolt_hits_on_b(sim: &PartyFixtureSim) -> Vec<u64> {
    events(sim)
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::Hit {
                tick,
                move_id: MoveId::MagicMissile,
                target,
                ..
            } if target == (sim.combat.enemies.len() + B) as u32 => Some(tick),
            _ => None,
        })
        .collect()
}

/// `corral_partition_rush_break`: B rushes a Donut at 1376 and bonks the screen.
fn rush_break() -> PartyFixtureSim {
    let mut sim = rig(PartitionVariant::Intact, 1040.0, 1376, 0);
    assert_eq!(partition_state(&sim), Some(PartitionState::Intact));
    assert_eq!(
        solids_of(&sim),
        (true, false),
        "the screen stands at tick 0"
    );
    for _ in 0..900 {
        idle(&mut sim);
        if rush_end(&sim).is_some() {
            break;
        }
    }
    let (tick, end) = rush_end(&sim).expect("B rushes into the screen");
    assert_eq!(end, RushEnd::Partition);
    // The same tick: the shove, the break, and the remnant standing.
    assert_eq!(sim.scene.world.tick, tick, "checked on the contact tick");
    assert_eq!(prop_hits(&sim), vec![(tick, StimulusKind::Shove, 1)]);
    assert_eq!(broken_ticks(&sim), vec![tick]);
    assert_eq!(partition_state(&sim), Some(PartitionState::Broken));
    assert_eq!(
        solids_of(&sim),
        (false, true),
        "the screen's solid toggles off at the end of the tick, the remnant stands"
    );
    assert_eq!(hero_hurts(&sim), 0, "nobody is hurt by a bonk");
    let l = &sim.combat.llamas[B];
    assert_eq!(
        l.rush_state().map(|r| r.phase),
        Some(RushPhase::Bonk),
        "B stands dazed at the face"
    );
    // B stops on the face: its 28 px box front rests on 1408, the screen's right face.
    assert_eq!(l.x, 1436.0, "B's pivot at the face");
    assert!(l.x > FACE_LEFT + 16.0);
    let stopped_at = l.x;
    // The lane is open: a bolt now reaches B.
    let mut bonk_left = l.rush_state().map(|r| r.left);
    let before = sim.combat.llamas[B].health;
    for t in 0..120 {
        step(
            &mut sim,
            RawInput::NONE,
            if t % 20 == 0 {
                press_attack()
            } else {
                RawInput::NONE
            },
        );
        bonk_left = sim.combat.llamas[B].rush_state().map(|r| r.left);
        if !bolt_hits_on_b(&sim).is_empty() {
            break;
        }
    }
    assert!(bonk_left.is_some(), "B is still bonked when the bolt lands");
    assert_eq!(bolt_hits_on_b(&sim).len(), 1, "a bolt reaches B");
    assert!(sim.combat.llamas[B].health < before, "and hurts it");
    assert_eq!(sim.combat.llamas[B].x, stopped_at, "B did not move");
    sim
}

/// `corral_partition_near_miss`: B's leash stops the rush 4 px short of the face.
fn near_miss() -> PartyFixtureSim {
    let mut sim = rig(PartitionVariant::Intact, 1040.0, 1376, 0);
    // The rush's box front is 28 px ahead of the pivot, so a rush that meets the
    // screen's right face (1408) bonks with the pivot at 1436. Setup (tick 0): B's
    // leash minimum 4 px short of that, so the leash stops the rush first.
    let stop = 1436.0;
    let c = sim.combat.llamas[B].corral.as_mut().unwrap();
    c.leash.min = stop + 4.0;
    for _ in 0..900 {
        idle(&mut sim);
        if rush_end(&sim).is_some() {
            break;
        }
    }
    let (_, end) = rush_end(&sim).expect("B rushes");
    assert_eq!(
        end,
        RushEnd::Miss,
        "stopped by the leash, never met the face"
    );
    assert_eq!(sim.combat.llamas[B].x, stop + 4.0, "4 px short of the face");
    assert_eq!(partition_state(&sim), Some(PartitionState::Intact));
    assert_eq!(solids_of(&sim), (true, false));
    assert!(prop_hits(&sim).is_empty(), "nothing touched the prop");
    assert!(broken_ticks(&sim).is_empty());
    // Nothing breaks it on a timer: 600 ticks on, it stands.
    for _ in 0..600 {
        idle(&mut sim);
    }
    assert_eq!(partition_state(&sim), Some(PartitionState::Intact));
    sim
}

/// `corral_partition_carl_breaks`: three distinct Carl hits, then Donut's swat.
fn carl_breaks() -> PartyFixtureSim {
    // Carl at the face; B held back (an unrelated rush would break the screen).
    let mut sim = rig(PartitionVariant::Intact, 1378.0, 1100, 0);
    sim.scene.world.hero.facing = 1;
    hold_llama(&mut sim, B);
    hold_llama(&mut sim, 1);
    // One jab: its whole active window is one move instance, so exactly one hit.
    step(&mut sim, press_attack(), RawInput::NONE);
    for _ in 0..40 {
        idle(&mut sim);
    }
    assert_eq!(
        prop_hits(&sim).iter().map(|h| h.2).collect::<Vec<_>>(),
        vec![1],
        "one move instance counts once"
    );
    assert_eq!(partition_state(&sim), Some(PartitionState::Weakened));
    assert_eq!(partition_hits(&sim), 1);
    assert_eq!(solids_of(&sim), (true, false), "weakened still stands");
    // A held attack button is one instance per press, not per tick; two more.
    for _ in 0..2 {
        step(&mut sim, press_attack(), RawInput::NONE);
        for _ in 0..40 {
            idle(&mut sim);
        }
    }
    let hits = prop_hits(&sim);
    assert_eq!(
        hits.iter().map(|h| (h.1, h.2)).collect::<Vec<_>>(),
        vec![
            (StimulusKind::MeleeHit, 1),
            (StimulusKind::MeleeHit, 2),
            (StimulusKind::MeleeHit, 3)
        ]
    );
    assert_eq!(broken_ticks(&sim).len(), 1, "it broke on the third hit");
    assert_eq!(partition_state(&sim), Some(PartitionState::Broken));
    assert_eq!(solids_of(&sim), (false, true));
    // Further hits on the remnant count nothing.
    step(&mut sim, press_attack(), RawInput::NONE);
    for _ in 0..40 {
        idle(&mut sim);
    }
    assert_eq!(prop_hits(&sim).len(), 3);
    sim
}

/// Donut's swat counts like Carl's jab (the pounce and the swat are physical hits).
fn donut_swat_breaks() -> PartyFixtureSim {
    let mut sim = rig(PartitionVariant::Intact, 1100.0, 1380, 0);
    hold_llama(&mut sim, B);
    hold_llama(&mut sim, 1);
    let swat = RawInput {
        physical: true,
        ..RawInput::NONE
    };
    for _ in 0..3 {
        step(&mut sim, RawInput::NONE, swat);
        for _ in 0..50 {
            idle(&mut sim);
        }
    }
    assert_eq!(
        prop_hits(&sim).iter().map(|h| h.2).collect::<Vec<_>>(),
        vec![1, 2, 3],
        "three swats"
    );
    assert_eq!(partition_state(&sim), Some(PartitionState::Broken));
    sim
}

/// `corral_partition_missile_blocked`: eight bolts at B across the screen.
fn missile_blocked() -> PartyFixtureSim {
    let mut sim = rig(PartitionVariant::Intact, 1040.0, 1376, 0);
    hold_llama(&mut sim, B);
    hold_llama(&mut sim, 1);
    let mana0 = sim.combat.donut_mana_view().map_or(0, |(m, _)| m);
    let mut bolts = 0;
    for t in 0..400 {
        step(
            &mut sim,
            RawInput::NONE,
            if t % 24 == 0 && bolts < 8 {
                bolts += 1;
                press_attack()
            } else {
                RawInput::NONE
            },
        );
    }
    let spent = events(&sim)
        .iter()
        .filter(|e| {
            matches!(
                e,
                CombatEvent::Mana(crate::donut_mana::ManaEvent::ManaSpent { .. })
            )
        })
        .count();
    assert_eq!(mana0, 8);
    assert_eq!(spent, 8, "eight bolts were released");
    assert_eq!(partition_state(&sim), Some(PartitionState::Intact));
    assert_eq!(partition_hits(&sim), 0);
    assert!(prop_hits(&sim).is_empty(), "no stimulus reached the prop");
    assert!(bolt_hits_on_b(&sim).is_empty(), "no bolt reached B");
    assert_eq!(sim.combat.llamas[B].health, 3, "B is unhurt");
    assert_eq!(solids_of(&sim), (true, false));
    sim
}

/// The shelf Donut script of the evaluator's glob test: step down into the pen
/// (so B wakes), hop back onto the shelf and wait. Returns the sim after `ticks`.
fn shelf_donut(variant: PartitionVariant, ticks: u64) -> PartyFixtureSim {
    // Solo Donut with a parked Carl, the evaluator's seat: the camera follows her.
    let mut sim = rig_seat(Seat::SoloDonutParked, variant, 0.0, 1204, 32);
    let mut back_on_shelf = false;
    for t in 1..=ticks {
        let d = &sim.scene.world.donut_body;
        let feet = d.y - HERO_HALF_HEIGHT;
        let on_shelf = (feet - (room::FLOOR_WORLD_Y + 32.0)).abs() < 0.5 && d.grounded;
        let mut raw = RawInput::NONE;
        // The swap press on tick 1 makes Donut the controlled hero (the real swap path).
        raw.swap = t == 1;
        if t < 40 {
            raw.move_x = 1.0;
        } else if !on_shelf && !back_on_shelf {
            raw.move_x = if d.x > 1180.0 { -1.0 } else { 0.0 };
            raw.jump = d.grounded && d.x < 1240.0;
        } else {
            back_on_shelf = true;
        }
        step(&mut sim, raw, RawInput::NONE);
    }
    assert!(back_on_shelf, "she is back on the shelf");
    sim
}

/// `corral_partition_glob_blocked`: B's glob never crosses the screen.
///
/// Two halves. (1) Decision: B does not spit through a standing screen at all
/// (CRL-Q11, the glob path must be clear), so the shelf Donut, hit by every
/// glob with no screen, is never hit. (2) Physics: a glob released at B's mouth
/// with the screen standing dies on its face, takes no hero with it and is no
/// stimulus (the prop's state and hit count do not move).
fn glob_blocked() -> PartyFixtureSim {
    let spits = |sim: &PartyFixtureSim| {
        events(sim)
            .iter()
            .filter(|e| matches!(e, CombatEvent::LlamaSpit { llama: 0, .. }))
            .count()
    };
    let glob_hurts = |sim: &PartyFixtureSim| {
        events(sim)
            .iter()
            .filter(|e| {
                matches!(e, CombatEvent::HeroHurt { hero: HeroId::Donut, source, .. }
                    if *source != crate::llama_rush::LLAMA_RUSH_SOURCE_ID)
            })
            .count()
    };
    // (1) The control: with no screen B spits at the shelf Donut and hits her.
    let control = shelf_donut(PartitionVariant::Absent, 900);
    assert!(spits(&control) > 0, "control: B spits at the shelf Donut");
    assert!(glob_hurts(&control) > 0, "control: and the glob hits her");
    // With the screen B holds its spit and she is never hit.
    let blocked = shelf_donut(PartitionVariant::Intact, 900);
    assert_eq!(spits(&blocked), 0, "B does not spit through the screen");
    assert_eq!(glob_hurts(&blocked), 0);
    assert_eq!(partition_state(&blocked), Some(PartitionState::Intact));
    // (2) A glob that is released anyway: B's mouth, 46 px ahead and 45 up.
    let glob = || crate::llama::Glob {
        x: 1488.0 - 46.0,
        y: room::FLOOR_WORLD_Y + 45.0,
        dir: -1,
        travelled: 0.0,
        owner: 0,
        spawn_tick: 0,
    };
    let fly = |variant| {
        let mut sim = rig(variant, 1040.0, 1204, 32);
        hold_llama(&mut sim, B);
        hold_llama(&mut sim, 1);
        sim.combat.globs.push(glob());
        let mut min_x = f32::MAX;
        for _ in 0..240 {
            idle(&mut sim);
            for g in &sim.combat.globs {
                min_x = min_x.min(g.x);
            }
        }
        (sim, min_x)
    };
    let (open, _) = fly(PartitionVariant::Absent);
    assert!(
        events(&open).iter().any(|e| matches!(
            e,
            CombatEvent::HeroHurt {
                hero: HeroId::Donut,
                ..
            }
        )),
        "control: the glob reaches the shelf Donut"
    );
    let (sim, min_x) = fly(PartitionVariant::Intact);
    assert!(sim.combat.globs.is_empty(), "the glob is spent");
    assert!(min_x > 1408.0, "it never passed the face: {min_x}");
    assert_eq!(hero_hurts(&sim), 0, "and hurt nobody");
    assert_eq!(partition_state(&sim), Some(PartitionState::Intact));
    assert_eq!(partition_hits(&sim), 0);
    assert!(prop_hits(&sim).is_empty(), "a glob is no stimulus");
    assert_eq!(solids_of(&sim), (true, false));
    sim
}

/// `corral_partition_remnant_bolt_passes`: the pre-broken remnant (16 px) lets a
/// floor bolt (box bottom exactly 16) through.
fn remnant_bolt_passes() -> PartyFixtureSim {
    let mut sim = rig(PartitionVariant::Broken, 1040.0, 1376, 0);
    hold_llama(&mut sim, B);
    hold_llama(&mut sim, 1);
    assert_eq!(partition_state(&sim), Some(PartitionState::Broken));
    assert_eq!(solids_of(&sim), (false, true), "authored pre-broken");
    // The remnant is the prop core's 16 px: a floor bolt's box bottom is 16.
    let env = sim.combat.env().unwrap();
    let remnant = env
        .solids()
        .iter()
        .find(|s| s.kind == crate::combat_scene::EnvSolidKind::Remnant)
        .unwrap()
        .rect;
    assert_eq!((remnant.x, remnant.w, remnant.h), (1392, 16, 16));
    for t in 0..80 {
        step(
            &mut sim,
            RawInput::NONE,
            if t == 0 {
                press_attack()
            } else {
                RawInput::NONE
            },
        );
    }
    assert_eq!(
        bolt_hits_on_b(&sim).len(),
        1,
        "the bolt passed over the remnant"
    );
    assert!(sim.combat.llamas[B].health < 3);
    assert!(prop_hits(&sim).is_empty(), "a bolt is no stimulus either");
    assert_eq!(partition_state(&sim), Some(PartitionState::Broken));
    sim
}

/// `corral_partition_retry_restores`: after the break a restart (the evaluator's
/// rig) and a checkpoint respawn (the launched room, which has the demo rules)
/// each bring the standing screen back.
fn retry_restores() -> PartyFixtureSim {
    let fresh_env = env_hash(&rig(PartitionVariant::Intact, 1378.0, 1100, 0));
    let mut sim = rig(PartitionVariant::Intact, 1378.0, 1100, 0);
    sim.scene.world.hero.facing = 1;
    hold_llama(&mut sim, B);
    hold_llama(&mut sim, 1);
    for _ in 0..3 {
        step(&mut sim, press_attack(), RawInput::NONE);
        for _ in 0..40 {
            idle(&mut sim);
        }
    }
    assert_eq!(partition_state(&sim), Some(PartitionState::Broken));
    assert_eq!(solids_of(&sim), (false, true));
    sim.combat.restart(&mut sim.scene);
    assert_eq!(partition_state(&sim), Some(PartitionState::Intact));
    assert_eq!(partition_hits(&sim), 0);
    assert_eq!(
        solids_of(&sim),
        (true, false),
        "the wall solid stands again at once"
    );
    assert_eq!(env_hash(&sim), fresh_env, "bit for bit");
    // And it breaks again from a clean count (Carl walked back to the face).
    sim.scene.world.hero.x = 1378.0;
    sim.scene.world.hero.facing = 1;
    hold_llama(&mut sim, B);
    hold_llama(&mut sim, 1);
    step(&mut sim, press_attack(), RawInput::NONE);
    for _ in 0..40 {
        idle(&mut sim);
    }
    assert_eq!(partition_hits(&sim), 1, "a clean count");

    // The checkpoint respawn of the launched room (design 6: props restore).
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let (carl, donut) = crate::f01_late::Loadout::Story.flags();
    let (_run, mut scene, mut combat) =
        crate::f01_late::corral_start(&root, 360, carl, donut, Some(0)).unwrap();
    let hits = |combat: &crate::combat_scene::CombatState| {
        combat
            .env()
            .and_then(|e| e.partition(usize::from(PROP)))
            .map(|p| (p.state(), p.hits()))
    };
    assert_eq!(hits(&combat), Some((PartitionState::Intact, 0)));
    for token in 1..=3 {
        combat.env_stimulate(
            usize::from(PROP),
            crate::env_prop::Stimulus::MeleeHit {
                token,
                player: 0,
                weight: 1,
            },
            1,
        );
    }
    combat.step(&mut scene, RawInput::NONE);
    assert_eq!(hits(&combat), Some((PartitionState::Broken, 3)));
    combat.respawn(&mut scene);
    assert_eq!(hits(&combat), Some((PartitionState::Intact, 0)), "respawn");
    let env = combat.env().unwrap();
    let wall = env
        .solids()
        .iter()
        .position(|s| s.kind == crate::combat_scene::EnvSolidKind::PartitionWall)
        .unwrap();
    assert!(
        scene.world.solids[env.solid_range().start + wall].intact,
        "the wall solid stands again at once on the respawn"
    );
    sim
}

/// The env runtime's own hash block.
fn env_hash(sim: &PartyFixtureSim) -> u64 {
    let mut h = 0u64;
    sim.combat
        .env()
        .unwrap()
        .mix_into(&mut |v| h = h.rotate_left(5) ^ v.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    h
}

/// Run one fixture by name (the final sim, for the evidence writer).
pub fn run(name: &str) -> PartyFixtureSim {
    match name {
        "corral_partition_rush_break" => rush_break(),
        "corral_partition_near_miss" => near_miss(),
        "corral_partition_carl_breaks" => carl_breaks(),
        "corral_partition_missile_blocked" => missile_blocked(),
        "corral_partition_glob_blocked" => glob_blocked(),
        "corral_partition_remnant_bolt_passes" => remnant_bolt_passes(),
        "corral_partition_retry_restores" => retry_restores(),
        other => panic!("no partition fixture {other}"),
    }
}

#[test]
fn corral_partition_rush_break_on_the_contact_tick() {
    rush_break();
}

#[test]
fn corral_partition_near_miss_leaves_it_standing() {
    near_miss();
}

#[test]
fn corral_partition_carl_breaks_on_the_third_distinct_hit() {
    carl_breaks();
}

#[test]
fn corral_partition_donut_swat_counts_the_same_way() {
    donut_swat_breaks();
}

#[test]
fn corral_partition_missile_blocked_eight_bolts() {
    missile_blocked();
}

#[test]
fn corral_partition_glob_blocked_on_the_screen() {
    glob_blocked();
}

#[test]
fn corral_partition_remnant_bolt_passes_over_it() {
    remnant_bolt_passes();
}

#[test]
fn corral_partition_retry_restores_the_screen() {
    retry_restores();
}

/// The height of a hero's feet above the walking line, and whether it stands.
fn feet_above_floor(sim: &PartyFixtureSim, hero: HeroId) -> (f32, bool) {
    let body = sim.scene.world.body(hero);
    (
        body.y - HERO_HALF_HEIGHT - room::FLOOR_WORLD_Y,
        body.grounded,
    )
}

/// DV-10: the screen is jumpable and standable by both heroes, so it never
/// blocks progress. Each hero runs at it with the jump held on the way and ends
/// beyond it; each stands on its 48 px top (the M2-06 start) without sliding off.
#[test]
fn corral_partition_is_jumpable_and_standable_by_both_heroes() {
    for hero in [HeroId::Carl, HeroId::Donut] {
        let mut sim = rig(PartitionVariant::Intact, 1040.0, 1040, 0);
        hold_llama(&mut sim, B);
        hold_llama(&mut sim, 1);
        // Setup (tick 0): the runner's column, 40 px short of the face.
        match hero {
            HeroId::Carl => sim.scene.world.hero.x = 1340.0,
            HeroId::Donut => {
                sim.scene.world.donut_body.x = 1340.0;
                sim.scene.world.sync_donut_for_tests();
            }
        }
        let mut peak: f32 = 0.0;
        for t in 0..160 {
            let run = RawInput {
                move_x: 1.0,
                // Jump 20 px before the face and keep trying: a hop that lands on the
                // top walks on over it.
                jump: t > 10,
                ..RawInput::NONE
            };
            if hero == HeroId::Carl {
                step(&mut sim, run, RawInput::NONE);
            } else {
                step(&mut sim, RawInput::NONE, run);
            }
            peak = peak.max(feet_above_floor(&sim, hero).0);
        }
        let body = sim.scene.world.body(hero);
        assert!(
            body.x > 1420.0,
            "{hero:?} crossed the screen: x {} (peak {peak})",
            body.x
        );
        assert!(peak >= 48.0, "{hero:?} cleared its 48 px top: peak {peak}");
        assert_eq!(partition_state(&sim), Some(PartitionState::Intact));

        // Standable: the M2-06 start, her pivot on the top's column.
        let mut sim = rig(PartitionVariant::Intact, 1040.0, 1400, 48);
        hold_llama(&mut sim, B);
        hold_llama(&mut sim, 1);
        if hero == HeroId::Carl {
            sim.scene.world.hero.x = 1400.0;
            sim.scene.world.hero.y = room::FLOOR_WORLD_Y + 48.0 + HERO_HALF_HEIGHT;
        }
        for _ in 0..90 {
            idle(&mut sim);
        }
        let (feet, grounded) = feet_above_floor(&sim, hero);
        assert!(grounded, "{hero:?} stands on the top");
        assert!(
            (feet - 48.0).abs() < 0.5,
            "{hero:?} on the 48 px top: {feet}"
        );
    }
}

/// A hero on top of the screen when it breaks drops onto the remnant (16 px) and
/// stands there: nobody is left inside a solid or falling through.
#[test]
fn corral_partition_a_hero_on_top_drops_onto_the_remnant_when_it_breaks() {
    let mut sim = rig(PartitionVariant::Intact, 1378.0, 1400, 48);
    sim.scene.world.hero.facing = 1;
    hold_llama(&mut sim, B);
    hold_llama(&mut sim, 1);
    assert!((feet_above_floor(&sim, HeroId::Donut).0 - 48.0).abs() < 0.5);
    for _ in 0..3 {
        step(&mut sim, press_attack(), RawInput::NONE);
        for _ in 0..40 {
            idle(&mut sim);
        }
    }
    assert_eq!(partition_state(&sim), Some(PartitionState::Broken));
    let (feet, grounded) = feet_above_floor(&sim, HeroId::Donut);
    assert!(grounded, "she landed");
    assert!((feet - 16.0).abs() < 0.5, "on the remnant's top: {feet}");
}

/// The remnant-lift rule on the break tick: a hero whose body overlaps the
/// remnant when the screen breaks is lifted straight up onto its top (the rule
/// the weight's landing uses). The screen's own footprint keeps bodies out, so
/// this only guards a body the solid no longer holds.
#[test]
fn corral_partition_break_lifts_a_hero_that_overlaps_the_remnant() {
    use crate::combat_scene::EnvRuntime;
    use crate::env_hazard::Victim;
    use crate::env_prop::Stimulus;
    let mut env = EnvRuntime::new(&kits().encounter).unwrap();
    let hero = crate::camera_lab::room::IRect::new(1388, 540, 20, 40);
    let _ = env.offer(
        usize::from(PROP),
        Stimulus::Shove {
            shove_id: 1,
            impulse_px_s: 240,
            source: 0,
            credit: None,
        },
        1,
    );
    let out = env.step(1, &[(Victim::hero(0), hero)]);
    assert_eq!(out.lifts.len(), 1);
    assert_eq!((out.lifts[0].hero, out.lifts[0].up_px), (0, 16));
    // Once: the next step lifts nobody.
    assert!(env.step(2, &[(Victim::hero(0), hero)]).lifts.is_empty());
}

/// Every fixture runs twice with an equal event log and state hash.
#[test]
fn corral_partition_fixtures_are_deterministic() {
    for name in FIXTURES {
        let a = run(name);
        let b = run(name);
        assert_eq!(a.state_hash(), b.state_hash(), "{name}");
        let lines = |s: &PartyFixtureSim| {
            s.combat
                .events
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        };
        assert_eq!(lines(&a), lines(&b), "{name}");
    }
}

/// Evidence: with `CORRAL_D_EVIDENCE_DIR` set, write each fixture's COMBAT_EVENT
/// lines and final state hash there (`docs/evidence/floor01-corral/corral-d/`).
#[test]
fn corral_partition_fixtures_write_their_replays_when_asked() {
    let Ok(dir) = std::env::var("CORRAL_D_EVIDENCE_DIR") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut index = String::new();
    for name in FIXTURES {
        let sim = run(name);
        let mut text = String::new();
        for e in &sim.combat.events {
            text.push_str(&e.to_string());
            text.push('\n');
        }
        text.push_str(&format!(
            "FIXTURE_END name={name} tick={} state_hash={:#018x} env_hash={:#018x}\n",
            sim.scene.world.tick,
            sim.state_hash(),
            env_hash(&sim)
        ));
        std::fs::write(dir.join(format!("{name}.log")), &text).unwrap();
        index.push_str(&format!(
            "{name} ticks={} state_hash={:#018x}\n",
            sim.scene.world.tick,
            sim.state_hash()
        ));
    }
    std::fs::write(dir.join("fixtures.txt"), index).unwrap();
}
