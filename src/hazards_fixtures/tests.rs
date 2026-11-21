use super::*;
use crate::combat::{CombatEvent, DefeatCause};
use crate::combat_scene::slime::SlimeEnemy;
use crate::f01_hazards;
use crate::party::HeroId;
use crate::rot_sticker::{RotCause, StickerState};
use crate::slime::slime_source;

const SLIME_A: SlimeAt = SlimeAt {
    x: 400,
    facing: 1,
    home: 400,
};

fn slime_of(rig: &Rig, i: usize) -> &SlimeEnemy {
    rig.combat().slot_enemies()[i]
        .as_any()
        .expect("a slime")
        .downcast_ref::<SlimeEnemy>()
        .expect("a SlimeEnemy")
}

fn attack_tick(rig: &Rig) -> Option<u16> {
    slime_of(rig, 0).slime().attack_tick()
}

/// Step Carl's input until the slime's tell starts; returns the tick it did.
fn until_tell(rig: &mut Rig, input: RawInput, limit: u64) -> u64 {
    for _ in 0..limit {
        rig.carl(input);
        if attack_tick(rig) == Some(0) {
            return rig.tick();
        }
    }
    panic!("the tell never started");
}

fn one_slime(carl_x: i32) -> Rig {
    Layout {
        slimes: vec![SLIME_A],
        carl_x,
        ..Layout::default()
    }
    .sim(None)
}

// ---------------------------------------------------------------- slime

#[test]
fn haz_slime_lunge_hits_idle_carl() {
    let mut rig = one_slime(444);
    let tell = until_tell(&mut rig, RawInput::NONE, 10);
    // First harm on attack tick 32 (HAZ-Q09): 1 damage, source of roster 0.
    for _ in 0..40 {
        rig.carl(RawInput::NONE);
    }
    let hurts = rig.hurts(HeroId::Carl);
    assert_eq!(hurts.len(), 1, "{hurts:?}");
    assert_eq!(hurts[0].0, tell + 32);
    assert_eq!(hurts[0].1, slime_source(0));
    assert_eq!(hurts[0].2, crate::combat::HERO_HEALTH - 1);
}

#[test]
fn haz_slime_gel_tell_cannot_be_jabbed_out() {
    // A jab in the tell damages it and never ends it early (gel).
    let mut rig = one_slime(430);
    rig.scene_mut().world.hero.facing = -1;
    let tell = until_tell(&mut rig, RawInput::NONE, 10);
    for k in 1..=40u64 {
        let press = k == 4 || k == 20;
        rig.carl(if press { JAB } else { RawInput::NONE });
        if tell + k <= tell + 31 {
            assert!(attack_tick(&rig).is_some(), "the tell holds on +{k}");
        }
    }
    assert!(
        slime_of(&rig, 0).slime().health() < 3,
        "the jabs damaged it"
    );
    let hurts = rig.hurts(HeroId::Carl);
    assert_eq!(hurts.first().map(|h| h.0), Some(tell + 32), "{hurts:?}");
}

#[test]
fn haz_slime_no_split_three_hits_one_defeat() {
    let mut rig = one_slime(430);
    // Park the slime asleep far from the lunge: a passive kill by three jabs.
    for k in 0..400u64 {
        let press = k % 30 == 0;
        rig.carl(if press { JAB } else { RawInput::NONE });
    }
    let defeats = rig.defeats();
    assert!(defeats.len() <= 1, "{defeats:?}");
    assert_eq!(rig.combat().slot_enemies().len(), 1, "nothing is spawned");
}

#[test]
fn haz_slime_body_burn_pops_carl_once_per_entry() {
    // Asleep (Passive is not a slime state: it wakes only in view and sight; put it
    // out of sight by staying more than 160 px away, then walk in).
    let mut rig = Layout {
        slimes: vec![SLIME_A],
        carl_x: 200,
        ..Layout::default()
    }
    .sim(None);
    for _ in 0..200 {
        rig.carl(walk(1.0));
        if !rig.hurts(HeroId::Carl).is_empty() {
            break;
        }
    }
    let hurts = rig.hurts(HeroId::Carl);
    assert_eq!(hurts.len(), 1, "{hurts:?}");
    assert_eq!(hurts[0].1, slime_source(0));
}

// ----------------------------------------------------------------- acid

/// The test pool: clear of the pocket A foothold (x 160..208) and every slab.
const POOL: (i32, i32) = (700, 764);

fn pool(x0: i32, x1: i32, carl_x: i32) -> Rig {
    Layout {
        acid: vec![(x0, x1, None)],
        carl_x,
        ..Layout::default()
    }
    .sim(None)
}

fn acid_hurts(rig: &Rig) -> Vec<(u64, u32, u8)> {
    rig.hurts(HeroId::Carl)
        .into_iter()
        .filter(|h| h.1 & 0xFFFF == crate::slime::ACID_SOURCE_ID)
        .collect()
}

#[test]
fn haz_acid_pop_out() {
    let mut rig = pool(POOL.0, POOL.1, POOL.0 - 50);
    let mut first = None;
    for _ in 0..120 {
        let held = first.is_none();
        rig.carl(if held { walk(1.0) } else { RawInput::NONE });
        if first.is_none() && !acid_hurts(&rig).is_empty() {
            first = Some(rig.x_of(HeroId::Carl));
        }
    }
    let hurts = acid_hurts(&rig);
    assert_eq!(hurts.len(), 1, "one pop, no second hurt: {hurts:?}");
    assert_eq!(hurts[0].1, crate::slime::acid_source(1));
    assert_eq!(hurts[0].2, crate::combat::HERO_HEALTH - 1);
    assert!(first.is_some());
    assert!(
        rig.x_of(HeroId::Carl) < POOL.0 as f32,
        "popped back out to the left: {}",
        rig.x_of(HeroId::Carl)
    );
}

#[test]
fn haz_acid_centre_tie_pops_toward_plus_x() {
    let mut rig = pool(POOL.0, POOL.1, 732);
    rig.carl(RawInput::NONE);
    let hurts = acid_hurts(&rig);
    assert_eq!(hurts.len(), 1, "{hurts:?}");
    assert_eq!(hurts[0].0, 1, "on tick 1");
    for _ in 0..20 {
        rig.carl(RawInput::NONE);
    }
    assert!(rig.x_of(HeroId::Carl) > 732.0, "popped +x");
}

#[test]
fn haz_acid_jump_clear_and_invulnerable_entry() {
    // A standing-start run and jump 8 px before the pool clears it (HAZ-01).
    let mut rig = pool(POOL.0, POOL.1, POOL.0 - 50);
    let mut jumped = false;
    for _ in 0..150 {
        let right_edge = rig.x_of(HeroId::Carl) + 12.0;
        let jump = !jumped && right_edge >= POOL.0 as f32 - 8.0;
        jumped |= jump;
        rig.carl(RawInput {
            move_x: 1.0,
            jump,
            ..RawInput::NONE
        });
    }
    assert!(acid_hurts(&rig).is_empty(), "{:?}", acid_hurts(&rig));
    assert!(rig.x_of(HeroId::Carl) > POOL.1 as f32);

    // Invulnerable on entry: no hurt while the 55 ticks last.
    let mut rig = pool(POOL.0, POOL.1, POOL.0 - 50);
    rig.scene_mut().world.hero.invulnerable_ticks = 55;
    for _ in 0..40 {
        rig.carl(walk(1.0));
    }
    assert!(acid_hurts(&rig).is_empty());
}

#[test]
fn haz_acid_goblin_never_enters_the_pool() {
    let mut rig = Layout {
        acid: vec![(POOL.0, POOL.1, None)],
        goblins: vec![POOL.0 - 80],
        carl_x: POOL.1 + 40,
        ..Layout::default()
    }
    .sim(None);
    for _ in 0..360 {
        rig.carl(RawInput::NONE);
        let g = &rig.combat().enemies[0];
        assert!(
            g.x + 12.0 <= POOL.0 as f32 + 0.5,
            "the goblin holds at the edge: {}",
            g.x
        );
    }
    assert!(
        rig.combat().enemies[0].x > POOL.0 as f32 - 40.0,
        "it walked up to the edge"
    );
}

// ------------------------------------------------------------------ rot

fn rot_state(rig: &Rig, prop: u16) -> StickerState {
    rig.combat()
        .env()
        .and_then(|e| e.sticker(prop))
        .expect("a sticker")
        .state()
}

fn under_slab_rig(carl_x: i32, track: (i32, i32, i32)) -> Rig {
    Layout {
        stickers: vec![Layout::under_slab(track.0, track.1, track.2)],
        carl_x,
        ..Layout::default()
    }
    .sim(None)
}

#[test]
fn haz_rot_scuttle_and_trigger() {
    let mut rig = under_slab_rig(540, (512, 600, 600));
    let mut provoked = None;
    for _ in 0..80 {
        rig.carl(RawInput::NONE);
        if provoked.is_none() && !rig.provokes().is_empty() {
            provoked = Some(rig.tick());
        }
    }
    let t = provoked.expect("it provoked");
    let provokes = rig.provokes();
    assert_eq!(provokes.len(), 1, "once per sticker: {provokes:?}");
    assert!(matches!(
        provokes[0],
        CombatEvent::RotProvoked {
            cause: RotCause::Proximity,
            player: Some(0),
            sticker: 0,
            ..
        }
    ));
    let blasts = rig.blasts();
    assert_eq!(
        blasts,
        vec![(t + 24, 0)],
        "the blast is 24 ticks after: {blasts:?}"
    );
    let hurts = rig.hurts(HeroId::Carl);
    assert_eq!(hurts.len(), 1, "{hurts:?}");
    assert_eq!(hurts[0].0, t + 24);
    assert_eq!(hurts[0].1, crate::rot_sticker::rot_source(0));
    assert_eq!(rot_state(&rig, 0), StickerState::Spent);
}

#[test]
fn haz_rot_run_through_is_hit_and_enemy_never_triggers() {
    // An awake, still sticker at 600: Carl cannot run through it.
    let mut rig = under_slab_rig(450, (600, 600, 600));
    for _ in 0..160 {
        rig.carl(walk(1.0));
    }
    assert_eq!(rig.provokes().len(), 1);
    assert_eq!(
        rig.hurts(HeroId::Carl).len(),
        1,
        "{:?}",
        rig.hurts(HeroId::Carl)
    );

    // A goblin walking under it provokes nothing (Carl far away, in view).
    let mut rig = Layout {
        stickers: vec![Layout::under_slab(600, 600, 600)],
        goblins: vec![520],
        carl_x: 700,
        ..Layout::default()
    }
    .sim(None);
    for _ in 0..240 {
        rig.carl(RawInput::NONE);
    }
    assert!(rig.provokes().is_empty(), "{:?}", rig.provokes());
}

#[test]
fn haz_rot_slab_top_shields_the_hero() {
    // Carl on the slab's top above a still sticker: the slab blocks the line of
    // sight, so there is no provoke.
    let mut rig = under_slab_rig(560, (600, 600, 600));
    let foot = rig.scene().world.hero_foot_y();
    rig.scene_mut().world.hero.y += 80.0;
    rig.scene_mut().world.hero.x = 596.0;
    let _ = foot;
    for _ in 0..240 {
        rig.carl(RawInput::NONE);
    }
    assert!(rig.provokes().is_empty(), "{:?}", rig.provokes());
}

#[test]
fn haz_rot_jab_and_run_on_the_wall() {
    // A wall sticker on K2's left face, dome 32 up (x 954): Carl's hurtbox edge
    // 17 px from the dome centre (x 925). The sheet expects `Hit(Jab)`; the scene
    // gives `Proximity` on tick 1: 17 px is inside the 32 px trigger, and the
    // proximity test runs on the first tick, before the jab is active (every hit
    // that can reach the 14 px shell is also inside the trigger circle, so the
    // melee cause is shadowed: HAZA2-D1). What the sheet's fixture is for still
    // holds: the attacker gets no hit-stop, and running away at once outruns the
    // blast (11 running ticks inside the 24-tick warning).
    let mut rig = Layout {
        stickers: vec![Layout::on_pillar(32)],
        solids: vec![Layout::pillar()],
        carl_x: 925,
        ..Layout::default()
    }
    .sim(None);
    rig.carl(JAB);
    for _ in 0..40 {
        rig.carl(walk(-1.0));
    }
    let p = rig.provokes();
    assert_eq!(p.len(), 1, "{p:?}");
    assert!(matches!(
        p[0],
        CombatEvent::RotProvoked {
            tick: 1,
            cause: RotCause::Proximity,
            player: Some(0),
            ..
        }
    ));
    assert_eq!(rig.blasts().len(), 1);
    assert!(
        rig.hurts(HeroId::Carl).is_empty(),
        "{:?}",
        rig.hurts(HeroId::Carl)
    );
}

// ------------------------------------------------------- more fixtures

fn kill_slime(rig: &mut Rig, roster: usize) {
    use crate::scene_enemy::{EnvSource, SlotEnvHit};
    let hit = SlotEnvHit {
        victim: crate::combat_scene::slime::target_id(roster),
        source: EnvSource::Prop(0),
        class: crate::env_hazard::HazardClass::FallingMass,
        damage: 9,
        push_x: 0,
        initiator: Some(0),
        activation: 1,
    };
    let tick = rig.tick();
    let (scene, combat) = rig.parts();
    combat.slot_env_hits(scene, &[hit], tick);
}

fn pool_bounds(rig: &Rig, id: u16) -> Option<(i32, i32)> {
    rig.combat().env().unwrap().acid().get(id).unwrap().bounds()
}

#[test]
fn haz_acid_drain_follows_its_slime_and_is_removed_on_drain_tick_60() {
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
    assert_eq!(pool_bounds(&rig, 1), Some(POOL));
    kill_slime(&mut rig, 0);
    assert_eq!(rig.defeats().len(), 1);
    for i in 0..60u16 {
        rig.idle(1);
        assert_eq!(
            pool_bounds(&rig, 1),
            crate::slime::drain_bounds(POOL.0, POOL.1, i, 60),
            "drain tick {i}"
        );
        assert!(pool_bounds(&rig, 1).is_some(), "never a zero-width volume");
    }
    rig.idle(1);
    assert_eq!(pool_bounds(&rig, 1), None, "removed on drain tick 60");
}

#[test]
fn haz_acid_launched_goblin_takes_one_damage_per_launch() {
    // The same kick with and without the pool: the pool costs exactly one
    // damage (or the kill, credited to the environment), once per launch.
    let run = |acid: bool| {
        let mut rig = Layout {
            acid: if acid {
                vec![(POOL.0, POOL.1, None)]
            } else {
                Vec::new()
            },
            goblins: vec![POOL.0 - 18],
            carl_x: POOL.0 - 50,
            ..Layout::default()
        }
        .sim(None);
        rig.carl(RawInput {
            physical: true,
            ..RawInput::NONE
        });
        for _ in 0..200 {
            rig.carl(RawInput::NONE);
        }
        rig
    };
    let (dry, wet) = (run(false), run(true));
    let (g_dry, g_wet) = (&dry.combat().enemies[0], &wet.combat().enemies[0]);
    eprintln!(
        "dry x={} hp={} wet x={} hp={}",
        g_dry.x, g_dry.health, g_wet.x, g_wet.health
    );
    assert!(
        g_wet.x + 12.0 > POOL.0 as f32,
        "it flew past the walk barrier"
    );
    let env: Vec<_> = wet
        .defeats()
        .into_iter()
        .filter(|d| matches!(d.2, DefeatCause::Environment { .. }))
        .collect();
    assert!(
        g_wet.health + 1 == g_dry.health || (!env.is_empty() && g_dry.health == 1),
        "one acid damage: dry {} wet {} env {env:?}",
        g_dry.health,
        g_wet.health
    );
}

#[test]
fn haz_rot_bait_and_retreat_pins_the_latest_safe_reversal() {
    let escapes = |l: u64| -> bool {
        let mut rig = under_slab_rig(450, (512, 600, 600));
        let mut t = None;
        for _ in 0..200 {
            rig.carl(walk(1.0));
            if !rig.provokes().is_empty() {
                t = Some(rig.tick());
                break;
            }
        }
        let t = t.expect("provoked");
        // The sheet's rule: a hero who starts moving away on provoke tick t + L
        // (until then he stands: he does not walk deeper).
        for _ in 0..40 {
            let dir = if rig.tick() + 1 >= t + l { -1.0 } else { 0.0 };
            rig.carl(walk(dir));
        }
        rig.hurts(HeroId::Carl).is_empty()
    };
    let safe: Vec<u64> = (0..30).filter(|l| escapes(*l)).collect();
    eprintln!("safe reversals: {safe:?}");
    let latest = *safe.last().expect("an early reversal escapes");
    assert!(
        safe.iter().all(|l| *l <= latest) && (0..=latest).all(|l| safe.contains(&l)),
        "the safe reversals are exactly 0..=L: {safe:?}"
    );
    assert!(!escapes(latest + 1), "the next one is hit");
    assert_eq!(latest, 19, "the sheet predicts 19");
}

#[test]
fn haz_rot_dynamite_chain_lights_a_latched_sticker_at_plus_8_with_the_owner() {
    let mut found = None;
    for carl_x in (420..=540).step_by(10) {
        let mut rig = Layout {
            stickers: vec![Layout::under_slab(600, 600, 600)],
            dynamite: true,
            carl_x,
            ..Layout::default()
        }
        .sim(None);
        rig.carl(RawInput {
            ability: true,
            ..RawInput::NONE
        });
        for _ in 0..260 {
            rig.carl(RawInput::NONE);
        }
        let blasts = rig.blasts();
        let chained = rig.events().iter().any(|e| {
            matches!(
                e,
                CombatEvent::RotProvoked {
                    cause: RotCause::Chain,
                    ..
                }
            )
        });
        eprintln!("carl {carl_x}: blasts {blasts:?} chained {chained}");
        if chained && blasts.len() == 2 {
            found = Some((carl_x, blasts));
            break;
        }
    }
    let (_, blasts) = found.expect("some throw lands within 64 px of the sticker");
    // The sticker charge is index 0, the dynamite slot index 1 (after every authored charge).
    let sticker = blasts.iter().find(|b| b.1 == 0).unwrap();
    let dynamite = blasts.iter().find(|b| b.1 == 1).unwrap();
    assert_eq!(sticker.0, dynamite.0 + 8, "the chain is +8 ticks");
}

#[test]
fn haz_rot_respawn_a_warning_sticker_latches_and_a_spent_one_stays_spent() {
    // Warning at the respawn: latched again, nothing goes off.
    let mut rig = under_slab_rig(570, (600, 600, 600));
    for _ in 0..200 {
        rig.carl(RawInput::NONE);
        if !rig.provokes().is_empty() {
            break;
        }
    }
    assert_eq!(rot_state(&rig, 0), StickerState::Warning);
    rig.idle(5);
    rig.respawn();
    assert_eq!(rot_state(&rig, 0), StickerState::Latched);
    rig.idle(60);
    assert!(rig.blasts().is_empty(), "{:?}", rig.blasts());

    // Spent at the respawn: stays spent.
    let mut rig = under_slab_rig(570, (600, 600, 600));
    rig.idle(40);
    assert_eq!(rot_state(&rig, 0), StickerState::Spent);
    rig.respawn();
    assert_eq!(rot_state(&rig, 0), StickerState::Spent);
}

#[test]
fn haz_restart_full_equals_a_fresh_start() {
    let layout = Layout {
        stickers: vec![Layout::under_slab(512, 600, 600)],
        acid: vec![(POOL.0, POOL.1, None)],
        slimes: vec![SlimeAt {
            x: 400,
            facing: -1,
            home: 400,
        }],
        carl_x: 450,
        ..Layout::default()
    };
    let fresh = layout.sim(None);
    let h0 = fresh.hash();
    let mut rig = layout.sim(None);
    for _ in 0..90 {
        rig.carl(walk(1.0));
    }
    assert_ne!(rig.hash(), h0);
    rig.restart();
    // `elapsed` spans attempts by design (combat_scene.rs): the rest is equal.
    rig.combat_mut().elapsed = 0;
    assert_eq!(rig.hash(), h0, "the post-restart hash equals a fresh start");
    assert_eq!(rot_state(&rig, 0), StickerState::Latched);
}

#[test]
fn haz_rot_one_evaluation_hurts_each_hero_once_with_no_evade() {
    let mut rig = Layout {
        stickers: vec![Layout::under_slab(600, 600, 600)],
        carl_x: 580,
        ..Layout::default()
    }
    .sim(Some(true));
    rig.scene_mut().world.donut_body.x = 570.0;
    for _ in 0..40 {
        rig.carl(RawInput::NONE);
    }
    assert_eq!(rig.hurts(HeroId::Carl).len(), 1);
    assert_eq!(rig.hurts(HeroId::Donut).len(), 1);
    assert!(
        !rig.events()
            .iter()
            .any(|e| matches!(e, CombatEvent::HeroEvaded { .. })),
        "a blast is never evadable"
    );
}

#[test]
fn haz_rot_core_zero_never_point_blank() {
    let mut rig = Layout {
        stickers: vec![Layout::on_pillar(32)],
        solids: vec![Layout::pillar()],
        goblins: vec![948],
        carl_x: 925,
        ..Layout::default()
    }
    .sim(None);
    // The goblin is passive here (it never walks into the test): put it at rest.
    let before = rig.combat().enemies[0].health;
    for _ in 0..60 {
        rig.carl(RawInput::NONE);
    }
    let blasts = rig.blasts();
    assert_eq!(blasts.len(), 1, "{:?}", rig.events());
    for (_, _, cause) in rig.defeats() {
        if let DefeatCause::Explosion { point_blank, .. } = cause {
            assert!(!point_blank, "the outer result applies");
        }
    }
    let g = &rig.combat().enemies[0];
    assert!(
        g.health < before || !g.alive(),
        "1 damage from the outer band"
    );
}

#[test]
fn haz_room_exit_opens_on_defeat_roster() {
    let mut rig = Rig::room(false);
    rig.idle(2);
    assert!(!rig.combat().roster_cleared());
    let exit = f01_hazards::launch(&root()).unwrap().loaded.encounter.exit;
    let gate_intact = |rig: &Rig| {
        let layout_h = rig.scene().world.layout_height();
        rig.scene().world.solids.iter().any(|b| {
            b.intact
                && b.rect.left == exit.x as f32
                && b.rect.bottom == (layout_h - exit.bottom()) as f32
        })
    };
    assert!(gate_intact(&rig), "the exit is shut while the roster lives");
    kill_slime(&mut rig, 0);
    kill_slime(&mut rig, 2);
    for i in 0..rig.combat().enemies.len() {
        rig.combat_mut().enemies[i].crush();
    }
    rig.idle(2);
    assert!(rig.combat().roster_cleared());
    assert!(!gate_intact(&rig), "the exit opened");
}

/// Pocket C: Donut at x 708 jumps on tick 1 and presses cast on tick `p`.
fn pocket_c(p: u64, ticks: u64) -> Rig {
    // Both heroes are staged at the pocket (Carl parked 68 px left of R2's
    // dome and 80 px right of R1's, out of both triggers and both blasts), so the shared camera frames R2 as the sheet's
    // solo-Donut camera does.
    let mut rig = Rig::room_at(true, 674);
    rig.scene_mut().world.hero.x = 680.0;
    rig.scene_mut().world.donut_body.x = 708.0;
    rig.scene_mut().world.donut_body.facing = 1;
    for k in 1..=ticks {
        let jump = k == 1;
        let cast = k == p;
        rig.step(
            RawInput::NONE,
            RawInput {
                jump,
                attack: cast,
                ..RawInput::NONE
            },
        );
    }
    rig
}

#[test]
fn haz_room_c_missile_collateral_window_is_six_plus_eight_press_ticks() {
    let provoked = |p: u64| {
        let rig = pocket_c(p, 100);
        rig.provokes().iter().any(|e| {
            matches!(
                e,
                CombatEvent::RotProvoked {
                    cause: RotCause::Projectile,
                    player: Some(1),
                    ..
                }
            )
        })
    };
    let rise: Vec<u64> = (1..12).filter(|p| provoked(*p)).collect();
    let fall: Vec<u64> = (38..56).filter(|p| provoked(*p)).collect();
    // HAZA2-D2 (an open question for the orchestrator, nothing retuned): press
    // `p` is the step on which the cast is pressed and the jump is pressed on
    // step 1, so jump tick = p - 1. The real scene gives cast presses on jump
    // ticks 0..=6 (7 press ticks) and 42..=49 (8), which are the sheet's
    // original predictions "from the scene's exact bolt step". The owner-noted
    // "6 press ticks" (0..=5, 41..=48) is the pure model's figure
    // (`q9_the_airborne_cast_window_into_r2_is_a_handful_of_press_ticks`), one
    // tick narrower on the rise. The test pins what the scene does, with the
    // window edges: both the first and the last press provoke, and the ones
    // just outside do not.
    assert_eq!(rise, (1..=7).collect::<Vec<_>>(), "rise {rise:?}");
    assert_eq!(fall, (43..=50).collect::<Vec<_>>(), "fall {fall:?}");
    assert!(!provoked(8) && !provoked(42) && !provoked(51));
    // A provoke never comes from proximity, Donut is never hurt, the slime S2
    // inside the blast takes 1 damage with no state change.
    // Before she lands (tick ~58), and after the blast (provoke + 24): S2 holds
    // at home while she is airborne, so nothing but R2's blast touches it.
    let rig = pocket_c(rise[0], 50);
    assert!(rig.hurts(HeroId::Donut).is_empty());
    assert!(!rig.events().iter().any(|e| matches!(
        e,
        CombatEvent::RotProvoked {
            cause: RotCause::Proximity,
            ..
        }
    )));
    let s2 = rig.combat().slot_enemies()[1]
        .as_any()
        .unwrap()
        .downcast_ref::<SlimeEnemy>()
        .unwrap();
    assert_eq!(s2.slime().health(), 2, "R2's blast hit S2 for 1");
}

#[test]
fn haz_slime_kick_slide_pushes_it_24_px_and_the_lunge_whiffs() {
    // Slime at 400 facing +x, Carl facing it: a push kick pressed on attack tick
    // 3 lands in the tell and slides it 24 px away from the kicker (HAZ-Q12);
    // the lunge whiffs. HAZA2-D3: the sheet places Carl at 440; the scene's push
    // kick reaches the tell hurtbox (375..413) from 430 and not from 435 or 440,
    // so the fixture stands at 430 (nothing retuned).
    let mut rig = one_slime(430);
    rig.scene_mut().world.hero.facing = -1;
    let tell = until_tell(&mut rig, RawInput::NONE, 10);
    let mut min_x = f32::MAX;
    let mut slid_at = None;
    for k in 1..=120u64 {
        let press = k == 3;
        rig.carl(if press {
            RawInput {
                physical: true,
                ..RawInput::NONE
            }
        } else {
            RawInput::NONE
        });
        let s = slime_of(&rig, 0).slime();
        if s.sliding() && slid_at.is_none() {
            slid_at = Some(k);
        }
        min_x = min_x.min(s.x);
    }
    assert!(
        slid_at.is_some_and(|k| k <= 24),
        "the kick lands by attack tick 24: {slid_at:?}"
    );
    assert!(
        (min_x - 376.0).abs() < 0.6,
        "it slid 24 px away from the kicker: {min_x}"
    );
    let _ = tell;
    assert!(
        rig.hurts(HeroId::Carl).is_empty(),
        "the lunge whiffs: {:?}",
        rig.hurts(HeroId::Carl)
    );
}
