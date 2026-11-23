//! PR-D (HAZ-FU): the T11 follow-up items of the hazard room, one test each, and the
//! knob-off proofs (`hazfu01..04` in `enemy_slime_v0.json`: `false` is the behaviour and
//! the state hashes of the build before PR-D).

use super::*;
use crate::combat::{CombatEvent, DefeatCause};
use crate::combat_scene::EnvSolidKind;
use crate::combat_scene::slime::SlimeEnemy;
use crate::env_prop::{Ignored, Offer, Stimulus};
use crate::morale::Reaction;
use crate::rot_sticker::StickerState;

mod scn;

const POOL: (i32, i32) = (700, 764);

fn coward_seed() -> u64 {
    (1..500u64)
        .find(|seed| {
            crate::morale::draw_goblin(crate::cf02::active(), *seed, 0, 0).temperament
                == crate::cf02::Temperament::Coward
        })
        .expect("some seed draws a Coward first")
}

fn goblin_x(rig: &Rig) -> f32 {
    rig.combat().enemies[0].x
}

// ------------------------------------------------------------------ D8

/// D8: the morale-forcing hook lets a scene fixture show what only unit tests showed: a
/// panicking Coward (Q61) that flees toward a pool is stopped at its edge, never inside it
/// (HAZ-Q48), and a Coward's hop back (Q64) toward a pool is cut short at the edge.
#[test]
fn haz_acid_coward_flee_and_hop_stop_at_the_pool_edge() {
    let rig_at = |x: i32| {
        Layout {
            acid: vec![(POOL.0, POOL.1, None)],
            goblins: vec![x],
            carl_x: POOL.0 - 200,
            seed: Some(coward_seed()),
            ..Layout::default()
        }
        .sim(None)
    };
    let inside = |rig: &Rig| {
        let x = goblin_x(rig);
        x + 12.0 > POOL.0 as f32 + 0.01 && x - 12.0 < POOL.1 as f32 - 0.01
    };
    // Flee: Panic toward +x (away from Carl, toward the pool), from 60 px short of it.
    let mut rig = rig_at(POOL.0 - 72);
    rig.combat_mut().force_reaction_for_test(
        0,
        Reaction::Panic {
            left: 90,
            flee_dir: 1,
            blocked: 0,
        },
        0,
    );
    assert!(matches!(
        rig.combat().goblin_reaction(0),
        Reaction::Panic { .. }
    ));
    let start = goblin_x(&rig);
    let mut nearest = f32::MIN;
    for tick in 0..120u64 {
        rig.scene_mut().world.hero.invulnerable_ticks = 255;
        rig.carl(RawInput::NONE);
        assert!(!inside(&rig), "tick {tick}: the flee entered the pool");
        nearest = nearest.max(goblin_x(&rig) + 12.0);
    }
    assert!(
        nearest > start + 12.0 + 40.0,
        "it did flee toward the pool: {start} -> {nearest}"
    );
    assert!(
        (nearest - POOL.0 as f32).abs() < 1.5,
        "and was stopped at the edge: {nearest}"
    );
    // Hop: the back-off (28 px over 10 ticks) from 10 px short of the pool is cut at the edge.
    let mut rig = rig_at(POOL.0 - 22);
    rig.combat_mut()
        .force_reaction_for_test(0, Reaction::BackOff { elapsed: 0 }, 1);
    for tick in 0..14u64 {
        rig.scene_mut().world.hero.invulnerable_ticks = 255;
        rig.carl(RawInput::NONE);
        assert!(!inside(&rig), "tick {tick}: the hop entered the pool");
    }
    let edge = POOL.0 as f32 - 12.0;
    assert!(
        (goblin_x(&rig) - edge).abs() < 0.5 || goblin_x(&rig) < edge,
        "the hop ended at or before the edge: {}",
        goblin_x(&rig)
    );
}

// ------------------------------------------------------------ scenarios

/// Carl's push kick (`physical`).
const KICK: RawInput = RawInput {
    physical: true,
    ..RawInput::NONE
};

fn slime_hp(rig: &Rig, i: usize) -> u8 {
    rig.combat().slot_enemies()[i]
        .as_any()
        .and_then(|a| a.downcast_ref::<SlimeEnemy>())
        .map(|s| s.slime().health())
        .expect("a slime")
}

/// D6: Carl kicks a goblin into a slime that sleeps beyond Carl's sight. Returns the rig
/// after 100 ticks and the goblin's greatest x.
fn flier_into_slime() -> (Rig, f32) {
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
    let mut max_x = f32::MIN;
    for _ in 0..100 {
        rig.scene_mut().world.hero.invulnerable_ticks = 255;
        rig.carl(RawInput::NONE);
        max_x = max_x.max(goblin_x(&rig));
    }
    (rig, max_x)
}

/// D6 (sheet sections 6 and 19): a launched goblin that meets a slime stops against it, as
/// against a llama: it takes the normal CF02 contact result of a flier and stops dead along
/// x, and the slime (never bowled, never launched, gel) takes the contact damage of Q12
/// (0 in the shipped data), is not moved by it and keeps its three hit points. Knob off
/// (`hazfu01_flier_contact` false): the flier passes into the slime's body and lands in it.
#[test]
fn haz_slime_stops_a_launched_flier_and_is_not_bowled() {
    let (rig, max_x) = flier_into_slime();
    assert!(max_x < 670.0, "it stopped at the slime's edge: {max_x}");
    assert_eq!(slime_hp(&rig, 0), 3, "Q12 contact damage is 0");
    assert!(
        rig.combat().enemies[0].alive(),
        "the flier is not hurt by the stop"
    );
    // One contact per (launch, target): the slot contact does not event.
    let (off, off_max) =
        crate::slime::with_knobs_for_test(|t| t.hazfu01_flier_contact = false, flier_into_slime);
    assert!(
        off_max > 690.0,
        "knob off: the flier lands inside the slime's body (max {off_max})"
    );
    assert_ne!(rig.hash(), off.hash());
    // The edge of the contact: a slime 40 px farther is not touched by the same kick.
    let far = {
        let mut rig = Layout {
            slimes: vec![SlimeAt {
                x: 760,
                facing: -1,
                home: 760,
            }],
            goblins: vec![632],
            carl_x: 600,
            ..Layout::default()
        }
        .sim(None);
        rig.carl(KICK);
        let mut max_x = f32::MIN;
        for _ in 0..100 {
            rig.scene_mut().world.hero.invulnerable_ticks = 255;
            rig.carl(RawInput::NONE);
            max_x = max_x.max(goblin_x(&rig));
        }
        max_x
    };
    assert!(
        far > 690.0,
        "out of reach, the flier flies its full range: {far}"
    );
}

// ------------------------------------------------------- prop sentinel

/// The `prop: 0` sentinel is gone: a soffit, the exit gate and a slot enemy's gate carry no
/// prop index (`None`), a remnant and a drum carry theirs and a lock its lock index.
#[test]
fn a_solid_that_belongs_to_no_prop_has_no_prop_index() {
    let run = f01_hazards::launch(&root()).unwrap();
    let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
    let combat = f01_hazards::combat_of(&run, &mut scene).unwrap();
    let solids = combat.env().unwrap().solids();
    let kinds: Vec<_> = solids.iter().map(|s| (s.kind, s.prop)).collect();
    assert!(
        kinds.iter().any(|(k, _)| matches!(k, EnvSolidKind::Soffit))
            && kinds
                .iter()
                .any(|(k, _)| matches!(k, EnvSolidKind::ExitGate)),
        "{kinds:?}"
    );
    for (kind, prop) in &kinds {
        match kind {
            EnvSolidKind::Soffit | EnvSolidKind::ExitGate => {
                assert_eq!(*prop, None, "{kind:?} belongs to no prop")
            }
            other => panic!("unexpected solid {other:?} in the hazard room"),
        }
    }
    // A slot enemy's gate: `with_slot_gate` adds one after the others.
    let gated = crate::combat_scene::EnvRuntime::new(&run.loaded.encounter)
        .unwrap()
        .in_room(run.loaded.room.floor_y, &[], None)
        .with_slot_gate(9, IRect::new(0, 0, 8, 8));
    let gate = gated.solids().last().unwrap();
    assert!(matches!(gate.kind, EnvSolidKind::Gate { target: 9 }));
    assert_eq!(gate.prop, None);
    // The Boiler's remnant and drum carry their prop indices.
    let boiler = crate::f01_boiler::launch(&root()).unwrap();
    let env = crate::f01_boiler::env_runtime(&boiler).unwrap();
    let with_prop: Vec<_> = env
        .solids()
        .iter()
        .filter(|s| matches!(s.kind, EnvSolidKind::Remnant | EnvSolidKind::Drum))
        .collect();
    assert!(!with_prop.is_empty());
    assert!(with_prop.iter().all(|s| s.prop.is_some()), "{with_prop:?}");
}

// ---------------------------------------------------- env_stimulate

/// `env_stimulate` on a Rot Sticker says what happened: a stimulus the sticker does not count
/// (a blast, a prop contact, a shove, a hit of weight 0) is `NotAccepted`, a counted one
/// (melee, projectile, launched body) is `Progress`, and once it is provoked (warning) or
/// spent every stimulus is `AlreadyActivated`. Before PR-D every one reported `Progress`.
#[test]
fn env_stimulate_on_a_sticker_reports_only_what_it_counted() {
    let rig_of = || {
        Layout {
            stickers: vec![Layout::under_slab(560, 640, 600)],
            carl_x: 112,
            ..Layout::default()
        }
        .sim(None)
    };
    let melee = |weight| Stimulus::MeleeHit {
        token: 1,
        player: 0,
        weight,
    };
    let ignored = [
        Stimulus::Blast {
            blast_id: 1,
            owner: None,
            depth: 0,
        },
        Stimulus::Shove {
            shove_id: 1,
            impulse_px_s: 400,
            source: 0,
            credit: None,
        },
        Stimulus::PropContact {
            source: 0,
            initiator: None,
            depth: 1,
        },
        melee(0),
        Stimulus::Projectile {
            projectile_id: 1,
            owner: Some(1),
            weight: 0,
        },
    ];
    for stimulus in ignored {
        let mut rig = rig_of();
        assert_eq!(
            rig.combat_mut().env_stimulate(0, stimulus, 1),
            Some(Offer::Ignored(Ignored::NotAccepted)),
            "{stimulus:?}"
        );
    }
    let counted = [
        melee(1),
        Stimulus::Projectile {
            projectile_id: 2,
            owner: Some(1),
            weight: 1,
        },
        Stimulus::LaunchedBody {
            launch_id: 3,
            impulse_px_s: 200,
            initiator: Some(0),
        },
    ];
    for stimulus in counted {
        let mut rig = rig_of();
        assert_eq!(
            rig.combat_mut().env_stimulate(0, stimulus, 1),
            Some(Offer::Progress { progress: 1 }),
            "{stimulus:?}"
        );
    }
    // A counted stimulus still provokes it (the sticker's own intake is unchanged).
    let mut rig = rig_of();
    rig.combat_mut().env_stimulate(0, counted[0], 1);
    rig.idle(1);
    assert_eq!(rig.provokes().len(), 1);
    // Warning: one activation only.
    assert_eq!(
        rig.combat_mut().env_stimulate(0, counted[1], 2),
        Some(Offer::Ignored(Ignored::AlreadyActivated))
    );
    // Spent: the blast has gone off.
    rig.idle(30);
    assert!(matches!(
        rig.combat().env().unwrap().sticker(0).unwrap().state(),
        StickerState::Spent
    ));
    assert_eq!(
        rig.combat_mut().env_stimulate(0, counted[2], 40),
        Some(Offer::Ignored(Ignored::AlreadyActivated))
    );
    // No such prop: nothing.
    assert_eq!(rig.combat_mut().env_stimulate(9, counted[0], 40), None);
}

// ---------------------------------------------------- goblin in acid

/// A goblin placed inside the pool (it landed there) with Carl on the far side.
fn goblin_in_pool(x: f32, carl_x: i32) -> Rig {
    let mut rig = Layout {
        acid: vec![(POOL.0, POOL.1, None)],
        goblins: vec![POOL.0 + 32],
        carl_x,
        ..Layout::default()
    }
    .sim(None);
    rig.combat_mut().enemies[0].x = x;
    // The first-approach delay (Q58) is over: it is a goblin that has been fighting.
    rig.combat_mut().set_delay_for_test(0, 0);
    rig
}

fn overlaps_pool(rig: &Rig) -> bool {
    let x = goblin_x(rig);
    x + 12.0 > POOL.0 as f32 && x - 12.0 < POOL.1 as f32
}

/// A goblin that is inside the pool walks out by the **nearer** edge, whichever side Carl
/// is on, and starts no attack while it is inside; it never walks deeper. Before PR-D it
/// stood in the pool (a step toward the centre is refused, and so is every other).
#[test]
fn a_goblin_inside_the_pool_is_driven_out_by_the_nearer_edge() {
    // Left half, Carl far right: it leaves left (toward the nearer edge, away from Carl).
    let mut left = goblin_in_pool(POOL.0 as f32 + 14.0, POOL.1 + 70);
    let mut deepest = goblin_x(&left);
    for tick in 0..120u64 {
        left.scene_mut().world.hero.invulnerable_ticks = 255;
        left.carl(RawInput::NONE);
        deepest = deepest.max(goblin_x(&left));
        assert!(
            deepest < POOL.0 as f32 + 14.2,
            "tick {tick}: it went deeper: {deepest}"
        );
        if !overlaps_pool(&left) {
            break;
        }
    }
    assert!(
        !overlaps_pool(&left),
        "out of the pool: {}",
        goblin_x(&left)
    );
    assert!(goblin_x(&left) + 12.0 <= POOL.0 as f32 + 0.01);
    // Right half, Carl on the left: it leaves right, still the nearer edge.
    let mut right = goblin_in_pool(POOL.1 as f32 - 14.0, POOL.0 - 70);
    for _ in 0..120u64 {
        right.scene_mut().world.hero.invulnerable_ticks = 255;
        right.carl(RawInput::NONE);
        if !overlaps_pool(&right) {
            break;
        }
    }
    assert!(
        !overlaps_pool(&right),
        "out of the pool: {}",
        goblin_x(&right)
    );
    assert!(goblin_x(&right) - 12.0 >= POOL.1 as f32 - 0.01);
    // Knob off: it stands in the pool (the old behaviour).
    let stuck = crate::slime::with_knobs_for_test(
        |t| t.hazfu02_acid_exit = false,
        || {
            let mut rig = goblin_in_pool(POOL.0 as f32 + 14.0, POOL.1 + 70);
            for _ in 0..120u64 {
                rig.scene_mut().world.hero.invulnerable_ticks = 255;
                rig.carl(RawInput::NONE);
            }
            rig
        },
    );
    assert!(
        overlaps_pool(&stuck),
        "knob off: still inside: {}",
        goblin_x(&stuck)
    );
}

// ----------------------------------------------------- acid hurt event

/// Carl kicks the goblin beside the pool into it; 200 ticks.
fn kicked_into_the_pool(health: u8) -> Rig {
    let mut rig = Layout {
        acid: vec![(POOL.0, POOL.1, None)],
        goblins: vec![POOL.0 - 18],
        carl_x: POOL.0 - 50,
        ..Layout::default()
    }
    .sim(None);
    rig.combat_mut().enemies[0].health = health;
    rig.carl(KICK);
    for _ in 0..200 {
        rig.carl(RawInput::NONE);
    }
    rig
}

fn env_hurts(rig: &Rig) -> Vec<CombatEvent> {
    rig.events()
        .into_iter()
        .filter(|e| matches!(e, CombatEvent::EnvironmentHurt { .. }))
        .collect()
}

/// A launched goblin that lands in the pool and survives took 1 damage once, silently
/// before PR-D: now one `EnvironmentHurt` (the patch id as `prop`, class acid, damage 1, the
/// health it has left, credited to the kicker). A lethal landing is `EnemyDefeated` plus
/// `EnvironmentKill` and no `EnvironmentHurt`. Knob off (`hazfu03_acid_hurt_event`
/// false): no event, the same damage and the same outcome.
#[test]
fn a_non_lethal_acid_landing_emits_an_environment_hurt_event() {
    let dry_hp = {
        let mut rig = Layout {
            goblins: vec![POOL.0 - 18],
            carl_x: POOL.0 - 50,
            ..Layout::default()
        }
        .sim(None);
        rig.combat_mut().enemies[0].health = 9;
        rig.carl(KICK);
        rig.idle(200);
        rig.combat().enemies[0].health
    };
    let wet = kicked_into_the_pool(9);
    let hurts = env_hurts(&wet);
    assert_eq!(hurts.len(), 1, "{hurts:?}");
    assert_eq!(
        hurts[0],
        CombatEvent::EnvironmentHurt {
            tick: match hurts[0] {
                CombatEvent::EnvironmentHurt { tick, .. } => tick,
                _ => unreachable!(),
            },
            target: 0,
            prop: 1,
            class: crate::env_hazard::HazardClass::Acid,
            damage: 1,
            health_left: dry_hp - 1,
            player: Some(0),
        }
    );
    assert_eq!(wet.combat().enemies[0].health, dry_hp - 1);
    assert!(
        hurts[0]
            .to_string()
            .contains("kind=environment_hurt target=0 prop=1 class=acid damage=1")
            && hurts[0].to_string().ends_with("credit_player=0"),
        "{}",
        hurts[0]
    );
    // Lethal: the default goblin the kick leaves on 1 hp dies of the acid; the kill is evented as before.
    let lethal = kicked_into_the_pool(crate::combat::ENEMY_HEALTH);
    assert!(env_hurts(&lethal).is_empty());
    assert!(
        lethal
            .defeats()
            .iter()
            .any(|d| matches!(d.2, DefeatCause::Environment { .. }))
    );
    // Knob off: nothing is evented and the goblin is just as hurt.
    let off = crate::slime::with_knobs_for_test(
        |t| t.hazfu03_acid_hurt_event = false,
        || kicked_into_the_pool(9),
    );
    assert!(env_hurts(&off).is_empty());
    assert_eq!(
        off.combat().enemies[0].health,
        wet.combat().enemies[0].health
    );
    assert_ne!(off.hash(), wet.hash(), "the event is hashed");
}

// ---------------------------------------- edge-triggered drain and AD6

/// A pool linked to slime 0; the slime sits away from Carl (asleep).
fn linked_pool() -> Rig {
    Layout {
        acid: vec![(POOL.0, POOL.1, Some(0))],
        slimes: vec![SlimeAt {
            x: POOL.0 + 32,
            facing: -1,
            home: POOL.0 + 32,
        }],
        carl_x: 300,
        ..Layout::default()
    }
    .sim(None)
}

fn bounds(rig: &Rig) -> Option<(i32, i32)> {
    rig.combat().env().unwrap().acid().get(1).unwrap().bounds()
}

/// The drain starts on the tick the slime goes from up to down and not on every tick it
/// is down: a pool restored while its slime stays down (what a respawn does for a living
/// slime) stays full. The old level test drained it again at once (knob off).
#[test]
fn a_pool_drains_on_the_slime_defeated_edge_only() {
    let run = || {
        let mut rig = linked_pool();
        rig.idle(3);
        rig.kill_slime(0);
        rig.idle(70);
        assert_eq!(bounds(&rig), None, "drained and removed");
        rig.combat_mut().restore_pool_for_test(0);
        assert_eq!(bounds(&rig), Some(POOL), "restored in full");
        rig.idle(5);
        rig
    };
    let edge = run();
    assert_eq!(bounds(&edge), Some(POOL), "no new edge: it stays full");
    let level = crate::slime::with_knobs_for_test(|t| t.hazfu04_pool_edge = false, run);
    assert_ne!(
        bounds(&level),
        Some(POOL),
        "the level test drains it again: {:?}",
        bounds(&level)
    );
}

/// AD6: a slime that is already down on the first env step was never alive (the kill
/// switch builds it defeated): its pool is removed at once, with no 60-tick drain in which a
/// hero could still be hurt by a volume of a slime the room does not have. Knob off: the old
/// drain.
#[test]
fn a_pool_of_a_slime_that_was_never_alive_is_removed_at_once() {
    let run = || {
        let mut rig = linked_pool();
        rig.kill_slime(0);
        rig.idle(1);
        rig
    };
    let now = run();
    assert_eq!(bounds(&now), None, "removed on the first step");
    let drained = crate::slime::with_knobs_for_test(|t| t.hazfu04_pool_edge = false, run);
    assert!(bounds(&drained).is_some(), "the old drain has only begun");
    // A slime that dies after at least one step still drains over 60 ticks.
    let mut rig = linked_pool();
    rig.idle(1);
    rig.kill_slime(0);
    rig.idle(1);
    assert!(bounds(&rig).is_some());
}

// ------------------------------------------ no damage volume on a sticker

/// A Rot Sticker's damage is its blast charge only: a sidecar that gives a sticker a hazard
/// volume (a second blast) is refused when the env runtime is built, with or without the
/// kill switch, and the shipped room (stickers, no owned volume) builds.
#[test]
fn a_sticker_that_owns_a_hazard_volume_is_refused() {
    let room = f01_hazards::launch(&root()).unwrap();
    assert!(
        room.loaded
            .encounter
            .hazards
            .iter()
            .all(|h| h.owner.is_none())
    );
    assert!(crate::combat_scene::EnvRuntime::new(&room.loaded.encounter).is_ok());
    let boiler = crate::f01_boiler::launch(&root()).unwrap();
    let volume = boiler.loaded.encounter.hazards[0].clone();
    let sticker = room
        .loaded
        .encounter
        .props
        .iter()
        .position(|p| p.kind == PropKind::RotSticker)
        .unwrap();
    let mut enc = room.loaded.encounter.clone();
    enc.hazards.push(crate::encounter::Hazard {
        owner: Some(sticker),
        ..volume
    });
    for rot in [true, false] {
        let error = crate::combat_scene::EnvRuntime::new_with_rot(&enc, rot)
            .expect_err("a sticker owns no volume");
        assert!(error.contains("owns no hazard volume"), "{error}");
    }
}

// ------------------------------------------------------------ feed ids

/// The feed id of an entry past its family's 0x100 range fails loudly in a debug build
/// (it would be another family's id); inside the range it is `base + index`.
#[test]
fn a_feed_id_past_its_family_range_fails_in_a_debug_build() {
    use crate::combat_scene::slime::{FEED_ID_SLIME, FEED_ID_SPAN, FEED_ID_STICKER, feed_id};
    assert_eq!(feed_id(FEED_ID_STICKER, 0), 0x300);
    assert_eq!(feed_id(FEED_ID_STICKER, 7), 0x307);
    assert_eq!(feed_id(FEED_ID_SLIME, u32::from(FEED_ID_SPAN) - 1), 0x2ff);
    if cfg!(debug_assertions) {
        let over = std::panic::catch_unwind(|| feed_id(FEED_ID_STICKER, u32::from(FEED_ID_SPAN)));
        assert!(over.is_err(), "0x100 overflows into the next family");
    }
}

// ------------------------------------------------------- knob-off proofs

/// `(scenario, hash with the knobs on, hash with every knob off)`. The off column is what the
/// build before PR-D (main `274b9151`) produced for the same script (measured there with
/// `scn.rs` copied unchanged); the on column is the first pin of the new behaviour. A
/// scenario a knob does not touch has the same hash in both columns.
const SCENARIOS: [(&str, u64, u64); 8] = [
    // D6 (`hazfu01`): the flier stops against the slime.
    (
        "flier_into_slime",
        0x42ca_bb55_4e7d_7460,
        0x6abc_b441_e923_c99b,
    ),
    // Goblin in the pool (`hazfu02`): it walks out.
    (
        "goblin_in_pool",
        0x8e00_820d_3a80_2800,
        0x30a9_f19e_b33d_c425,
    ),
    // Silent acid damage (`hazfu03`): the event is hashed.
    (
        "kicked_into_the_pool",
        0xe4d0_20cf_08c1_01ab,
        0xc31c_db48_e9b5_2f05,
    ),
    // AD6 (`hazfu04`): removed at once, not drained.
    (
        "pool_of_a_slime_never_alive",
        0x238a_9ed0_8d9b_b307,
        0xea26_6da2_cc6d_e371,
    ),
    // Untouched by any knob.
    (
        "pool_drain_after_the_slime",
        0x4be2_9e07_fd2d_bfd0,
        0x4be2_9e07_fd2d_bfd0,
    ),
    ("kick_slide", 0x09ef_ca95_6ebf_62e6, 0x09ef_ca95_6ebf_62e6),
    (
        "dynamite_chain",
        0x55d1_b0f7_7956_fbe0,
        0x55d1_b0f7_7956_fbe0,
    ),
    // FA-Q21 (`hazfu05`): the stick chains R2's stand-in under the soffit.
    (
        "dynamite_at_a_soffit_sticker",
        0xd291_2648_dea8_9d56,
        0xf7c1_5b98_516f_fe1e,
    ),
];

fn scenario(name: &str) -> u64 {
    match name {
        "flier_into_slime" => scn::flier_into_slime(),
        "goblin_in_pool" => scn::goblin_in_pool(),
        "kicked_into_the_pool" => scn::kicked_into_the_pool(),
        "pool_of_a_slime_never_alive" => scn::pool_of_a_slime_never_alive(),
        "pool_drain_after_the_slime" => scn::pool_drain_after_the_slime(),
        "kick_slide" => scn::kick_slide(),
        "dynamite_chain" => scn::dynamite_chain(),
        "dynamite_at_a_soffit_sticker" => scn::dynamite_at_a_soffit_sticker(),
        other => panic!("{other}"),
    }
}

/// Each HAZ-FU knob, alone off, reproduces the old hash of its own scenario and leaves the
/// other scenarios of the table on their new pins; every knob off reproduces every old hash
/// of the table (seven scenarios, three of which no knob touches). Each scenario replays to
/// the same hash twice.
#[test]
fn every_knob_off_reproduces_the_old_hashes() {
    // A scenario and the knobs that touch it. A goblin that lands in the pool is also driven
    // out of it (`hazfu02`), so the kick scenario is touched by `hazfu02` and `hazfu03`.
    type Edit = fn(&mut crate::slime::SlimeTuning);
    let own: [(&str, Edit); 5] = [
        ("flier_into_slime", |t| t.hazfu01_flier_contact = false),
        ("goblin_in_pool", |t| t.hazfu02_acid_exit = false),
        ("kicked_into_the_pool", |t| {
            t.hazfu02_acid_exit = false;
            t.hazfu03_acid_hurt_event = false;
        }),
        ("pool_of_a_slime_never_alive", |t| {
            t.hazfu04_pool_edge = false
        }),
        ("dynamite_at_a_soffit_sticker", |t| {
            t.hazfu05_sticker_chain_reach_px = 0
        }),
    ];
    let mut wrong = Vec::new();
    for (name, on, off) in SCENARIOS {
        let (a, b) = (scenario(name), scenario(name));
        if a != b || a != on {
            wrong.push(format!(
                "{name}: on {a:#018x} (pinned {on:#018x}), replay {b:#018x}"
            ));
        }
        let all = crate::slime::with_knobs_for_test(crate::slime::hazfu_off, || scenario(name));
        if all != off {
            wrong.push(format!("{name}: all off {all:#018x} (old {off:#018x})"));
        }
    }
    for (name, knob) in own {
        let (_, _, off) = SCENARIOS.iter().find(|s| s.0 == name).copied().unwrap();
        let alone = crate::slime::with_knobs_for_test(knob, || scenario(name));
        if alone != off {
            wrong.push(format!(
                "{name}: its knobs off {alone:#018x} (old {off:#018x})"
            ));
        }
    }
    // The flier knob and the AD6 knob touch only their own scenarios.
    let flier_off = |t: &mut crate::slime::SlimeTuning| t.hazfu01_flier_contact = false;
    let pool_off = |t: &mut crate::slime::SlimeTuning| t.hazfu04_pool_edge = false;
    for (name, on, _) in SCENARIOS {
        for (knob_of, edit) in [
            (
                "flier_into_slime",
                &flier_off as &dyn Fn(&mut crate::slime::SlimeTuning),
            ),
            ("pool_of_a_slime_never_alive", &pool_off),
        ] {
            if name == knob_of {
                continue;
            }
            let h = crate::slime::with_knobs_for_test(|t| edit(t), || scenario(name));
            if h != on {
                wrong.push(format!(
                    "{name}: the knob of {knob_of} moved it to {h:#018x}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The nine pinned fixtures of `tests_b` and the 27 aligned hashes of the room routes are
/// untouched by the knobs: the nine golden cases equal their pins with every knob off too.
#[test]
fn the_pinned_golden_cases_are_the_same_with_every_knob_off() {
    crate::slime::with_knobs_for_test(crate::slime::hazfu_off, || {
        for ((name, run), golden) in super::tests_b::GOLDEN_CASES {
            assert_eq!(run(), golden, "{name} with every knob off");
        }
    });
}

// ------------------------------------------------ hazfu05: chain reach

/// A dynamite thrown at a latched sticker under the pocket B slab, then moved by hand to
/// `d` px to the left of the sticker's dome, at its height (centre to centre) and chained once: whether the
/// sticker's charge was lit. `block` places a wall across the segment first.
fn chain_at(d: f32, block: bool, knob: u16) -> bool {
    crate::slime::with_knobs_for_test(
        |t| t.hazfu05_sticker_chain_reach_px = knob,
        || {
            let mut rig = Layout {
                stickers: vec![Layout::under_slab(600, 600, 600)],
                dynamite: true,
                carl_x: 420,
                ..Layout::default()
            }
            .sim(None);
            rig.carl(RawInput {
                ability: true,
                ..RawInput::NONE
            });
            rig.idle(60);
            let tick = rig.tick();
            let (dome_x, dome_y) = rig.combat().charges[0].centre();
            let slot = rig.combat().dynamite_slot();
            let stick = &mut rig.combat_mut().charges[slot];
            stick.x = dome_x - d;
            stick.foot_y = dome_y - crate::blast::BLAST_CENTRE_DY;
            stick.owner = Some(0);
            assert_eq!(
                rig.combat().charges[0].detonate_tick,
                0,
                "R1 is latched and unlit"
            );
            let (scene, combat) = rig.parts();
            if block {
                let mid = dome_x - d / 2.0;
                scene.world.solids.push(crate::prototype::Block {
                    rect: crate::prototype::Rect {
                        left: mid - 1.0,
                        bottom: dome_y - 20.0,
                        width: 2.0,
                        height: 40.0,
                    },
                    material: crate::prototype::Material::Stone,
                    intact: true,
                });
            }
            combat.chain_from_for_test(scene, slot, tick);
            combat.charges[0].detonate_tick == tick + 8
        },
    )
}

/// FA-Q21 boundary: with the knob on (76 px), a dynamite stick chains a sticker 75.9 px away
/// (centre to centre) and not 76.1 px away; with line of sight blocked it does not chain at
/// 70 px; at 63.9 px it chains with the knob on or off (the old reach), and at 64.1 it chains
/// only with the knob on. Knob off, 70 px does not chain (the old rule).
#[test]
fn a_dynamite_chains_a_sticker_out_to_76_px_and_not_beyond() {
    assert!(chain_at(75.9, false, 76), "75.9 chains");
    assert!(!chain_at(76.1, false, 76), "76.1 does not");
    assert!(
        !chain_at(70.0, true, 76),
        "70 with the line of sight blocked does not"
    );
    assert!(chain_at(70.0, false, 76), "70 chains on");
    assert!(
        !chain_at(70.0, false, 0),
        "70 does not chain with the knob off"
    );
    assert!(chain_at(63.9, false, 0) && chain_at(63.9, false, 76));
    assert!(!chain_at(64.1, false, 0) && chain_at(64.1, false, 76));
}
