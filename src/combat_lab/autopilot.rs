//! The lab's stand-in for a player: a deterministic policy that walks, jumps
//! faces and fights, built from the finished simulation state only. It is the
//! "legacy autopilot" of the clear budget (spec 9.1, assert 13) and drives the
//! scripted evidence captures (`floor01_scene --lab-autopilot`). It is not an
//! AI, it has no memory and no RNG, and it is never used by a live human run.

use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::input::RawInput;
use crate::party::HeroId;
use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Rect};

/// Carl's reach, in px of centre distance, for a jab and for the push kick on
/// the llama (its 46 px body reaches 42 px from its column).
const JAB_REACH: f32 = 30.0;
const LLAMA_KICK_REACH: f32 = 36.0;
/// Ticks between two presses (a human mashes slower).
const PRESS_CADENCE: u64 = 4;

/// Walk `hero` towards `goal_x`, jumping a face that blocks the way.
pub fn walk(scene: &SceneSim, hero: HeroId, goal_x: f32) -> RawInput {
    let body = scene.world.body(hero);
    let dir = (goal_x - body.x).signum();
    let mut raw = RawInput {
        move_x: if (goal_x - body.x).abs() < 4.0 {
            0.0
        } else {
            dir
        },
        ..RawInput::NONE
    };
    let probe = Rect {
        left: body.x - HERO_HALF_WIDTH + dir * 14.0,
        bottom: body.y - HERO_HALF_HEIGHT + 4.0,
        width: HERO_HALF_WIDTH * 2.0,
        height: 8.0,
    };
    if body.grounded
        && raw.move_x != 0.0
        && scene
            .world
            .solids
            .iter()
            .any(|b| b.intact && probe.overlaps(b.rect))
    {
        raw.jump = true;
    }
    raw
}

/// Close on the nearest living goblin and jab it, then the llama and kick it,
/// jumping faces on the way; with nothing left, walk to `exit_x`.
pub fn clear_and_exit(combat: &CombatState, scene: &SceneSim, exit_x: f32) -> RawInput {
    let hero = &scene.world.hero;
    let tick = scene.world.tick + 1;
    let press = tick.is_multiple_of(PRESS_CADENCE);
    let goblin = combat
        .enemies
        .iter()
        .filter(|g| g.alive())
        .min_by(|a, b| (a.x - hero.x).abs().total_cmp(&(b.x - hero.x).abs()));
    let (target, reach, kick) = match (goblin, combat.llamas.iter().find(|l| l.alive())) {
        (Some(g), _) => (g.x, JAB_REACH, false),
        (None, Some(l)) => (l.x, LLAMA_KICK_REACH, true),
        (None, None) => return walk(scene, HeroId::Carl, exit_x),
    };
    let mut raw = walk(scene, HeroId::Carl, target);
    if (target - hero.x).abs() <= reach {
        raw.move_x = (target - hero.x).signum();
        if press {
            raw.attack = !kick;
            raw.physical = kick;
        }
    }
    raw
}

/// The second human of a scripted co-op run: Donut keeps 48 px behind Carl,
/// so the party wall never holds the pair (evidence captures only).
pub fn follow(scene: &SceneSim) -> RawInput {
    let carl = &scene.world.hero;
    let goal = carl.x - 48.0 * f32::from(carl.facing);
    walk(scene, HeroId::Donut, goal)
}
