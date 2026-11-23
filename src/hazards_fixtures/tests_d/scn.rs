//! PR-D: the scenarios whose final state hash is pinned twice: with the HAZ-FU knobs on
//! (the new behaviour, `GOLDEN_ON`) and with them off (`GOLDEN_OFF`, the hashes the build
//! before PR-D produced for the same scripts). Written against the fixture kit only, so the
//! same file runs unchanged on the commit before PR-D.

use crate::combat_scene::slime::SlimeEnemy;
use crate::hazards_fixtures::{Layout, Rig, SlimeAt};
use crate::input::RawInput;

const POOL: (i32, i32) = (700, 764);
const KICK: RawInput = RawInput {
    physical: true,
    ..RawInput::NONE
};

fn hold(rig: &mut Rig, ticks: u64) {
    for _ in 0..ticks {
        rig.scene_mut().world.hero.invulnerable_ticks = 255;
        rig.carl(RawInput::NONE);
    }
}

/// D6: a goblin kicked into a slime.
pub fn flier_into_slime() -> u64 {
    let mut rig = Layout {
        slimes: vec![SlimeAt {
            x: 716,
            facing: -1,
            home: 716,
        }],
        goblins: vec![632],
        carl_x: 600,
        ..Layout::default()
    }
    .sim(None);
    rig.carl(KICK);
    hold(&mut rig, 100);
    rig.hash()
}

/// A goblin standing inside the pool, Carl 70 px beyond its far edge.
pub fn goblin_in_pool() -> u64 {
    let mut rig = Layout {
        acid: vec![(POOL.0, POOL.1, None)],
        goblins: vec![POOL.0 + 32],
        carl_x: POOL.1 + 70,
        ..Layout::default()
    }
    .sim(None);
    rig.combat_mut().enemies[0].x = POOL.0 as f32 + 14.0;
    rig.combat_mut().set_delay_for_test(0, 0);
    hold(&mut rig, 120);
    rig.hash()
}

/// A goblin with 9 health kicked into the pool: the non-lethal landing.
pub fn kicked_into_the_pool() -> u64 {
    let mut rig = Layout {
        acid: vec![(POOL.0, POOL.1, None)],
        goblins: vec![POOL.0 - 18],
        carl_x: POOL.0 - 50,
        ..Layout::default()
    }
    .sim(None);
    rig.combat_mut().enemies[0].health = 9;
    rig.carl(KICK);
    rig.idle(200);
    rig.hash()
}

/// AD6: the linked slime is down before the first env step.
pub fn pool_of_a_slime_never_alive() -> u64 {
    let mut rig = Layout {
        acid: vec![(POOL.0, POOL.1, Some(0))],
        slimes: vec![SlimeAt {
            x: POOL.0 + 32,
            facing: -1,
            home: POOL.0 + 32,
        }],
        carl_x: 300,
        ..Layout::default()
    }
    .sim(None);
    rig.kill_slime(0);
    rig.idle(30);
    rig.hash()
}

/// The slime of pocket A killed after three ticks: the edge case the knobs must not move.
pub fn pool_drain_after_the_slime() -> u64 {
    let mut rig = Layout {
        acid: vec![(POOL.0, POOL.1, Some(0))],
        slimes: vec![SlimeAt {
            x: POOL.0 + 32,
            facing: -1,
            home: POOL.0 + 32,
        }],
        carl_x: 300,
        ..Layout::default()
    }
    .sim(None);
    rig.idle(3);
    rig.kill_slime(0);
    rig.idle(90);
    rig.hash()
}

/// HAZ-Q12: the push kick slides a slime that is in its tell (HAZA2 fixture, hash pinned).
pub fn kick_slide() -> u64 {
    let mut rig = Layout {
        slimes: vec![SlimeAt {
            x: 400,
            facing: 1,
            home: 400,
        }],
        carl_x: 430,
        ..Layout::default()
    }
    .sim(None);
    rig.scene_mut().world.hero.facing = -1;
    for _ in 0..10 {
        rig.carl(RawInput::NONE);
        let tell = rig.combat().slot_enemies()[0]
            .as_any()
            .and_then(|a| a.downcast_ref::<SlimeEnemy>())
            .is_some_and(|s| s.slime().attack_tick() == Some(0));
        if tell {
            break;
        }
    }
    for k in 1..=120u64 {
        rig.carl(if k == 3 { KICK } else { RawInput::NONE });
    }
    rig.hash()
}

/// CP-Q34: a dynamite thrown from column 500 at a latched sticker of pocket B.
pub fn dynamite_chain() -> u64 {
    let mut rig = Layout {
        stickers: vec![Layout::under_slab(600, 600, 600)],
        dynamite: true,
        carl_x: 500,
        ..Layout::default()
    }
    .sim(None);
    rig.carl(RawInput {
        ability: true,
        ..RawInput::NONE
    });
    rig.idle(260);
    rig.hash()
}
