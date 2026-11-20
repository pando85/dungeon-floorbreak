//! Stage A part 2: observation, diagnostics, reward and the `ap_*` scenarios.

use std::path::PathBuf;

use super::observation::{
    ENEMY_DAMAGED, ENEMY_DX, ENEMY_FEATURES, HAZARD_FEATURES, MAX_ENEMIES, MAX_HAZARDS, SELF_LEN,
    SELF_VY, SELF_X_IN_VIEW, observe,
};
use super::scenario::{
    AP_SINGLE_GOBLIN_FAR, AP_TWO_GOBLINS_WIDE, Split, collides_with_game_fixtures,
    layout_fingerprint, lookup, roster, validate_fixture,
};
use super::*;
use crate::combat::{CombatEvent, DefeatCause, ForceTier, MoveId};
use crate::combat_scene::{
    CP01_GOBLIN_ATTACKS, CP01_SINGLE_GOBLIN, CP02_TWO_GOBLINS, FIXTURES, Fixture, FixtureSim,
    QA_BLAST_BREAKUP,
};
use crate::demo;
use crate::party::HeroId;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn env_for(id: &str) -> AutoplayEnv {
    AutoplayEnv::new(AutoplayConfig::new(id), &root()).unwrap()
}

fn plain_sim(fixture: &'static Fixture) -> FixtureSim {
    let kit =
        crate::env_kit::EnvKit::load(&root(), "assets/levels/floor01/corridor_abc_sample.json")
            .unwrap();
    FixtureSim::new(&kit, 360, fixture).unwrap()
}

const ENEMY_MASK_ON: f32 = 1.0;

/// A deterministic observation-only bot: walk right while nothing is in view,
/// otherwise close in on the nearest enemy and jab with fresh presses.
fn bot_action(obs: &ObservationV1, tick: u64) -> ActionV1 {
    if obs.enemy_mask[0] < ENEMY_MASK_ON {
        return ActionV1::new([2, 0, 0, 0, 0, 0]).unwrap();
    }
    let dx = obs.enemies[0][ENEMY_DX] * 640.0;
    let horizontal = if dx.abs() > 44.0 {
        if dx > 0.0 { 2 } else { 0 }
    } else if dx > 0.0 && obs.own[super::observation::SELF_FACING] < 0.0 {
        2
    } else if dx < 0.0 && obs.own[super::observation::SELF_FACING] > 0.0 {
        0
    } else {
        1
    };
    let attack = u8::from(dx.abs() <= 44.0 && tick.is_multiple_of(2));
    ActionV1::new([horizontal, 0, attack, 0, 0, 0]).unwrap()
}

/// Run the bot to the end of the episode. Returns the last step and the return.
fn run_bot(env: &mut AutoplayEnv) -> (StepOutcome, f64) {
    env.reset(0).unwrap();
    let mut total = 0.0;
    for tick in 0..2000u64 {
        let action = bot_action(&env.observe(), tick);
        let step = env.step(action).unwrap();
        total += step.reward.total();
        if step.outcome.is_some() {
            return (step, total);
        }
    }
    panic!("the bot never ended the episode");
}

// ---------------------------------------------------------------------------
// Observation: schema, purity, bounds
// ---------------------------------------------------------------------------

#[test]
fn schema_pins_length_order_and_bounds() {
    assert_eq!(OBSERVATION_VERSION, 1);
    assert_eq!(ObservationV1::VERSION, 1);
    assert_eq!(OBS_V1.version, 1);
    assert_eq!((SELF_LEN, MAX_ENEMIES, ENEMY_FEATURES), (14, 16, 14));
    assert_eq!((MAX_HAZARDS, HAZARD_FEATURES), (8, 6));
    assert_eq!(
        OBS_V1.self_names,
        [
            "vx",
            "vy",
            "facing",
            "grounded",
            "health",
            "hurt",
            "invulnerable",
            "busy",
            "move_jab",
            "move_haymaker",
            "move_push_kick",
            "hit_stop",
            "x_in_view",
            "y_in_view"
        ]
    );
    assert_eq!(OBS_V1.self_names[SELF_VY], "vy");
    assert_eq!(OBS_V1.self_names[SELF_X_IN_VIEW], "x_in_view");
    assert_eq!(
        OBS_V1.enemy_names,
        [
            "dx",
            "dy",
            "facing",
            "kind_goblin",
            "kind_llama",
            "vx",
            "vy",
            "grounded",
            "tell",
            "swing",
            "recover",
            "staggered",
            "launched",
            "damaged"
        ]
    );
    assert_eq!(OBS_V1.enemy_names[ENEMY_DAMAGED], "damaged");
    assert_eq!(
        OBS_V1.hazard_names,
        [
            "dx",
            "dy",
            "kind_block",
            "kind_charge",
            "kind_glob",
            "active"
        ]
    );
    assert_eq!(OBS_V1.available_names, ["special", "solo_swap"]);
    // The signed features are exactly the velocities, offsets and facings.
    let signed_self: Vec<&str> = OBS_V1
        .self_names
        .iter()
        .zip(OBS_V1.self_bounds)
        .filter(|(_, b)| *b == (-1.0, 1.0))
        .map(|(n, _)| *n)
        .collect();
    assert_eq!(signed_self, ["vx", "vy", "facing"]);
    let signed_enemy: Vec<&str> = OBS_V1
        .enemy_names
        .iter()
        .zip(OBS_V1.enemy_bounds)
        .filter(|(_, b)| *b == (-1.0, 1.0))
        .map(|(n, _)| *n)
        .collect();
    assert_eq!(signed_enemy, ["dx", "dy", "facing", "vx", "vy"]);
}

#[test]
fn every_value_stays_finite_and_inside_its_bounds_on_every_roster_scenario() {
    for entry in roster() {
        let mut env = env_for(entry.id);
        env.reset(0).unwrap();
        for (i, action) in super::tests::action_stream(7, 600).into_iter().enumerate() {
            check_bounds(&env.observe(), entry.id, i);
            if env.step(action).unwrap().outcome.is_some() {
                break;
            }
        }
        check_bounds(&env.observe(), entry.id, 9999);
    }
}

fn check_bounds(obs: &ObservationV1, id: &str, tick: usize) {
    for (i, v) in obs.own.iter().enumerate() {
        let (lo, hi) = OBS_V1.self_bounds[i];
        assert!(
            v.is_finite() && (lo..=hi).contains(v),
            "{id}@{tick} self[{i}]={v}"
        );
    }
    for (slot, row) in obs.enemies.iter().enumerate() {
        for (i, v) in row.iter().enumerate() {
            let (lo, hi) = OBS_V1.enemy_bounds[i];
            assert!(
                v.is_finite() && (lo..=hi).contains(v),
                "{id}@{tick} enemy[{slot}][{i}]={v}"
            );
            if obs.enemy_mask[slot] == 0.0 {
                assert_eq!(*v, 0.0, "padding must be zero");
            }
        }
    }
    assert!(obs.enemy_mask.iter().all(|m| *m == 0.0 || *m == 1.0));
    assert!(obs.hazards.iter().flatten().all(|v| *v == 0.0));
    assert!(obs.hazard_mask.iter().all(|v| *v == 0.0));
    assert_eq!(obs.available, [0.0, 0.0]);
}

#[test]
fn observation_is_pure_same_state_same_obs_and_no_mutation() {
    let mut env = env_for("cp02_two_goblins");
    env.reset(0).unwrap();
    for action in super::tests::action_stream(3, 150) {
        env.step(action).unwrap();
    }
    let hash = env.state_hash();
    let tick = env.tick_count();
    let events = env.episode_events().len();
    let diag_before = env.diagnostics();
    let first = env.observe();
    for _ in 0..5 {
        assert_eq!(env.observe(), first);
    }
    assert_eq!(first.to_bits(), env.observe().to_bits());
    assert_eq!(env.state_hash(), hash);
    assert_eq!(env.tick_count(), tick);
    assert_eq!(env.episode_events().len(), events);
    assert_eq!(env.diagnostics(), diag_before, "observe changes no counter");
    // A clone of the env at the same state observes identically.
    assert_eq!(env.clone().observe(), first);
    // And the free function agrees with the env accessor.
    assert_eq!(observe(env.sim()).0, first);
}

#[test]
fn values_beyond_a_bound_are_clamped_and_counted() {
    let mut sim = plain_sim(&CP01_SINGLE_GOBLIN);
    sim.scene.world.hero.vy = -1.0e6;
    sim.scene.world.hero.vx = 1.0e6;
    let (obs, stats) = observe(&sim);
    assert_eq!(obs.own[SELF_VY], -1.0);
    assert_eq!(obs.own[0], 1.0);
    assert_eq!(stats.clipped, 2);
    let (_, calm) = observe(&plain_sim(&CP01_SINGLE_GOBLIN));
    assert_eq!(calm, ObsStats::default());
}

#[test]
fn carl_state_features_follow_the_game() {
    let mut env = env_for("cp01_single_goblin");
    env.reset(0).unwrap();
    let start = env.observe();
    assert_eq!(start.own[4], 1.0, "full health");
    assert_eq!(start.own[3], 1.0, "grounded");
    assert_eq!(start.own[2], 1.0, "facing right");
    // The camera leads Carl, so the view is not centred on him at the start.
    let left = env.sim().scene.camera.left() as f32;
    assert!((start.own[SELF_X_IN_VIEW] - (400.0 - left) / 640.0).abs() < 1e-6);
    let y = start.own[13];
    assert!(
        (0.0..=1.0).contains(&y) && y > 0.5,
        "feet low in the view: {y}"
    );
    // A jab shows busy and exactly one move flag.
    let jab = ActionV1::new([1, 0, 1, 0, 0, 0]).unwrap();
    let before = env.episode_events().len();
    env.step(jab).unwrap();
    assert!(env.episode_events()[before..].iter().any(|e| matches!(
        e,
        CombatEvent::MoveStarted {
            move_id: MoveId::Jab,
            ..
        }
    )));
    let o = env.observe();
    assert_eq!(
        (o.own[7], o.own[8], o.own[9], o.own[10]),
        (1.0, 1.0, 0.0, 0.0)
    );
    // A jump leaves the ground and gives a positive (up) vertical speed.
    let mut env = env_for("cp01_single_goblin");
    env.reset(0).unwrap();
    env.step(ActionV1::new([1, 1, 0, 0, 0, 0]).unwrap())
        .unwrap();
    env.step(ActionV1::new([1, 1, 0, 0, 0, 0]).unwrap())
        .unwrap();
    let o = env.observe();
    assert_eq!(o.own[3], 0.0);
    assert!(o.own[SELF_VY] > 0.0);
}

// ---------------------------------------------------------------------------
// Perception mask
// ---------------------------------------------------------------------------

#[test]
fn initial_masks_match_the_roster_and_spawns_keep_16_px_off_the_view_edge() {
    let expected = [
        ("cp01_single_goblin", 1),
        ("ap_two_goblins_wide", 2),
        ("cp02_two_goblins", 2),
        ("ap_single_goblin_far", 0),
    ];
    for (id, visible) in expected {
        let mut env = env_for(id);
        env.reset(0).unwrap();
        let obs = env.observe();
        assert_eq!(obs.visible_enemies(), visible, "{id}");
        // Populated entries come first, padding after.
        assert_eq!(obs.enemy_mask.len(), MAX_ENEMIES);
        for (slot, mask) in obs.enemy_mask.iter().enumerate() {
            assert_eq!(*mask > 0.0, slot < visible, "{id} slot {slot}");
        }
        // The real initial edge (the camera leads Carl by 24 px: 744) must be
        // at least 16 px away from every spawn.
        let view_right = (env.sim().scene.camera.left() + 640) as f32;
        assert_eq!(view_right, 744.0);
        for spawn in lookup(id).unwrap().fixture.enemies {
            assert!(
                (spawn.x as f32 - view_right).abs() >= 16.0,
                "{id}: spawn {} within 16 px of the view edge {view_right}",
                spawn.x
            );
        }
    }
    let mut env = env_for("ap_single_goblin_far");
    env.reset(0).unwrap();
    let obs = env.observe();
    assert!(
        obs.enemies.iter().flatten().all(|v| *v == 0.0),
        "empty observation"
    );
}

#[test]
fn mask_equals_the_games_own_view_for_every_enemy_every_tick() {
    for id in ["ap_single_goblin_far", "ap_two_goblins_wide"] {
        let mut env = env_for(id);
        env.reset(0).unwrap();
        let mut seen_hidden_then_visible = false;
        let mut was_hidden = false;
        for tick in 0..400u64 {
            let obs = env.observe();
            let sim = env.sim();
            let expected = sim
                .combat
                .enemies
                .iter()
                .filter(|g| g.alive() && !sim.scene.is_offscreen(g.x, 0.0))
                .count();
            assert_eq!(obs.visible_enemies(), expected, "{id} tick {tick}");
            let hidden = sim
                .combat
                .enemies
                .iter()
                .any(|g| g.alive() && sim.scene.is_offscreen(g.x, 0.0));
            if was_hidden && !hidden {
                seen_hidden_then_visible = true;
            }
            was_hidden |= hidden;
            if env
                .step(ActionV1::new([2, 0, 0, 0, 0, 0]).unwrap())
                .unwrap()
                .outcome
                .is_some()
            {
                break;
            }
        }
        if id == "ap_single_goblin_far" {
            assert!(
                was_hidden && seen_hidden_then_visible,
                "the far goblin comes into view"
            );
        }
    }
}

#[test]
fn enemies_are_ordered_by_distance_then_id_and_defeated_ones_are_omitted() {
    let mut env = env_for("ap_two_goblins_wide");
    env.reset(0).unwrap();
    let obs = env.observe();
    assert!(obs.enemies[0][ENEMY_DX].abs() < obs.enemies[1][ENEMY_DX].abs());

    // Ties break on the enemy id: two goblins at the same distance, only the
    // second one damaged, so the damaged flag reveals the slot order.
    let mut sim = plain_sim(&CP02_TWO_GOBLINS);
    sim.combat.enemies[0].x = 560.0;
    sim.combat.enemies[1].x = 560.0;
    sim.combat.enemies[1].health = 1;
    let (obs, _) = observe(&sim);
    assert_eq!(obs.enemies[0][ENEMY_DAMAGED], 0.0);
    assert_eq!(obs.enemies[1][ENEMY_DAMAGED], 1.0);
    sim.combat.enemies[0].health = 1;
    sim.combat.enemies[1].health = 2;
    let (obs, _) = observe(&sim);
    assert_eq!(obs.enemies[0][ENEMY_DAMAGED], 1.0, "id 0 stays first");

    // A defeated goblin leaves the observation.
    let mut env = env_for("cp01_single_goblin");
    let (last, _) = run_bot(&mut env);
    assert_eq!(last.outcome, Some(Outcome::Success));
    assert_eq!(env.observe().visible_enemies(), 0);
}

// ---------------------------------------------------------------------------
// Leakage differential tests (AP-Q08)
// ---------------------------------------------------------------------------

fn bits(sim: &FixtureSim) -> Vec<u32> {
    observe(sim).0.to_bits()
}

/// A sim a few ticks into the fight (the goblin is in view and chasing).
fn engaged(fixture: &'static Fixture, ticks: u64) -> FixtureSim {
    let mut sim = plain_sim(fixture);
    for _ in 0..ticks {
        sim.step(crate::input::RawInput::default());
    }
    sim
}

#[test]
fn hidden_enemy_state_never_changes_the_observation() {
    // An enemy outside the view: position, health, mode and cooldowns differ.
    let base = plain_sim(&AP_SINGLE_GOBLIN_FAR);
    let reference = bits(&base);
    for (x, health, cooldown, hurt) in [
        (900.0, 2u8, 0u8, 0u8),
        (760.0, 1, 40, 12),
        (1500.0, 2, 255, 255),
    ] {
        let mut other = plain_sim(&AP_SINGLE_GOBLIN_FAR);
        other.combat.enemies[0].x = x;
        other.combat.enemies[0].health = health;
        other.combat.enemies[0].attack_cooldown = cooldown;
        other.combat.enemies[0].hurt_cooldown = hurt;
        other.combat.enemies[0].attack_tick = 17;
        assert!(other.scene.is_offscreen(other.combat.enemies[0].x, 0.0));
        assert_eq!(bits(&other), reference, "off-screen enemy leaked at x={x}");
    }

    // A visible goblin: private cooldowns and a health pair on the same side of
    // the maximum must not show.
    let base = engaged(&CP01_SINGLE_GOBLIN, 30);
    let reference = bits(&base);
    for (cooldown, hurt) in [(0u8, 0u8), (25, 7), (200, 99)] {
        let mut other = engaged(&CP01_SINGLE_GOBLIN, 30);
        other.combat.enemies[0].attack_cooldown = cooldown;
        other.combat.enemies[0].hurt_cooldown = hurt;
        assert_eq!(
            bits(&other),
            reference,
            "cooldown leaked ({cooldown}, {hurt})"
        );
    }
    let mut a = engaged(&CP01_SINGLE_GOBLIN, 30);
    let mut b = engaged(&CP01_SINGLE_GOBLIN, 30);
    a.combat.enemies[0].health = 2;
    b.combat.enemies[0].health = 5; // both at or above max: not damaged
    assert_eq!(bits(&a), bits(&b));

    // Carl's hurt cooldown timers are rule knowledge (a flag), not a count.
    let mut a = engaged(&CP01_SINGLE_GOBLIN, 30);
    let mut b = engaged(&CP01_SINGLE_GOBLIN, 30);
    a.combat.hurt_ticks = 5;
    b.combat.hurt_ticks = 40;
    a.scene.world.hero.invulnerable_ticks = 5;
    b.scene.world.hero.invulnerable_ticks = 50;
    assert_eq!(
        bits(&a),
        bits(&b),
        "only the flags show, never the remaining count"
    );
}

#[test]
fn damaged_is_the_one_intentional_health_bit_and_only_it() {
    let mut full = engaged(&CP01_SINGLE_GOBLIN, 30);
    let mut hurt = engaged(&CP01_SINGLE_GOBLIN, 30);
    full.combat.enemies[0].health = 2;
    hurt.combat.enemies[0].health = 1;
    let (a, b) = (full.clone(), hurt.clone());
    let (oa, ob) = (observe(&a).0, observe(&b).0);
    assert_ne!(oa.to_bits(), ob.to_bits());
    let mut differing = Vec::new();
    for (slot, (ra, rb)) in oa.enemies.iter().zip(&ob.enemies).enumerate() {
        for (i, (x, y)) in ra.iter().zip(rb).enumerate() {
            if x.to_bits() != y.to_bits() {
                differing.push((slot, i));
            }
        }
    }
    assert_eq!(differing, [(0, ENEMY_DAMAGED)]);
    assert_eq!(oa.own, ob.own);
    assert_eq!(oa.enemy_mask, ob.enemy_mask);
}

#[test]
fn the_fixture_script_is_invisible_to_the_observation() {
    static SILENT: Fixture = Fixture {
        script: &[],
        ..CP01_SINGLE_GOBLIN
    };
    let mut scripted =
        AutoplayEnv::new(AutoplayConfig::new("cp01_single_goblin"), &root()).unwrap();
    let mut silent =
        AutoplayEnv::new_unchecked(AutoplayConfig::new("cp01_single_goblin"), &root(), &SILENT)
            .unwrap();
    scripted.reset(0).unwrap();
    silent.reset(0).unwrap();
    for (tick, action) in super::tests::action_stream(11, 120).into_iter().enumerate() {
        assert_eq!(
            scripted.observe().to_bits(),
            silent.observe().to_bits(),
            "tick {tick}"
        );
        scripted.step(action).unwrap();
        silent.step(action).unwrap();
    }
}

#[test]
fn hidden_blast_charge_timing_is_absent_from_the_observation() {
    let base = plain_sim(&QA_BLAST_BREAKUP);
    assert!(!base.combat.charges.is_empty());
    let reference = bits(&base);
    let mut other = plain_sim(&QA_BLAST_BREAKUP);
    for charge in &mut other.combat.charges {
        charge.detonate_tick += 500;
        charge.armed_tick = Some(3);
    }
    assert_eq!(bits(&other), reference);
    let (obs, _) = observe(&base);
    assert!(obs.hazards.iter().flatten().all(|v| *v == 0.0));
    assert!(obs.hazard_mask.iter().all(|v| *v == 0.0));
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

#[test]
fn diagnostics_are_a_separate_accessor_with_the_privileged_state() {
    let mut env = env_for("cp01_single_goblin");
    env.reset(0).unwrap();
    let (last, _) = {
        let mut total = 0.0;
        let mut out = None;
        for tick in 0..2000 {
            let step = env.step(bot_action(&env.observe(), tick)).unwrap();
            total += step.reward.total();
            if step.outcome.is_some() {
                out = Some((step, total));
                break;
            }
        }
        out.unwrap()
    };
    assert_eq!(last.outcome, Some(Outcome::Success));
    let d = env.diagnostics();
    assert_eq!(d.state_hash, env.state_hash());
    assert_eq!(d.events, env.episode_events());
    assert_eq!(d.event_counts.enemies_defeated, 1);
    assert!(d.event_counts.hits_carl >= 2);
    assert_eq!(d.event_counts.hits_donut + d.event_counts.hits_unknown, 0);
    assert_eq!(d.defeated, [0]);
    assert_eq!(d.credited, [0]);
    assert_eq!(d.enemies.len(), 1);
    assert_eq!(d.enemies[0].health, 0);
    assert_eq!(d.carl.health, env.sim().scene.world.hero.health);
    assert_eq!(d.obs.overflow, 0);
    assert_eq!(d.obs.unknown_modes, 0);
    assert!(d.reward_total != 0.0);
}

#[test]
fn diagnostics_expose_hidden_state_that_observations_do_not() {
    let mut a = engaged(&CP01_SINGLE_GOBLIN, 30);
    let mut b = engaged(&CP01_SINGLE_GOBLIN, 30);
    a.combat.enemies[0].attack_cooldown = 1;
    b.combat.enemies[0].attack_cooldown = 90;
    assert_eq!(bits(&a), bits(&b));
    let da = super::diagnostics::enemy_diagnostics(&a);
    let db = super::diagnostics::enemy_diagnostics(&b);
    assert_ne!(da[0].attack_cooldown, db[0].attack_cooldown);
}

#[test]
fn event_counts_attribute_hits_and_derive_whiffs() {
    let hit = |move_id, instance| CombatEvent::Hit {
        tick: 1,
        move_id,
        instance,
        target: 0,
        damage: 1,
        health_left: 1,
        hit_stop: 0,
    };
    let started = |move_id, instance| CombatEvent::MoveStarted {
        tick: 1,
        move_id,
        instance,
    };
    let events = [
        started(MoveId::Jab, 1),
        hit(MoveId::Jab, 1),
        started(MoveId::Haymaker, 2),
        started(MoveId::Swat, 3),
        hit(MoveId::Swat, 3),
        hit(MoveId::GreyboxStrike, 4),
        CombatEvent::GoblinTell { tick: 2, goblin: 0 },
    ];
    let c = EventCounts::from_events(&events);
    assert_eq!(c.moves_started, 3);
    assert_eq!((c.hits_carl, c.hits_donut, c.hits_unknown), (1, 1, 1));
    assert_eq!(c.derived_whiffs, 1, "the haymaker never hit");
    assert_eq!(c.other_events, 1);
}

// ---------------------------------------------------------------------------
// Reward
// ---------------------------------------------------------------------------

fn defeated(target: u32, instigator: Option<u32>) -> CombatEvent {
    CombatEvent::EnemyDefeated {
        tick: 5,
        target,
        cause: DefeatCause::Melee,
        force: ForceTier::Routine,
        instigator,
    }
}

fn tracker(id: &str, required: &[u32]) -> RewardTracker {
    RewardTracker::new(RewardProfile::by_id(id).unwrap(), required)
}

#[test]
fn reward_profiles_carry_the_spec_numbers_as_data() {
    let sparse = RewardProfile::by_id("sparse-v1").unwrap();
    assert_eq!(
        (
            sparse.success,
            sparse.defeat,
            sparse.abandoned,
            sparse.timeout
        ),
        (1.0, -1.0, -1.0, 0.0)
    );
    assert_eq!(
        (sparse.enemy_defeated, sparse.carl_hurt, sparse.tick),
        (0.0, 0.0, 0.0)
    );
    let combat = RewardProfile::by_id("combat-v1").unwrap();
    assert_eq!(
        (
            combat.success,
            combat.defeat,
            combat.abandoned,
            combat.timeout
        ),
        (10.0, -10.0, -10.0, 0.0)
    );
    assert_eq!(
        (combat.enemy_defeated, combat.carl_hurt, combat.tick),
        (1.0, -1.0, -0.001)
    );
    assert_eq!(
        AutoplayConfig::new("cp01_single_goblin").reward_id,
        "combat-v1"
    );
    assert_eq!(RewardProfile::all().len(), 2);
    assert!(matches!(
        RewardProfile::by_id("fun-v9"),
        Err(AutoplayError::InvalidConfig(_))
    ));
    let mut config = AutoplayConfig::new("cp01_single_goblin");
    config.reward_id = "fun-v9".into();
    assert!(matches!(
        AutoplayEnv::new(config, &root()),
        Err(AutoplayError::InvalidConfig(_))
    ));
}

#[test]
fn one_defeated_foe_is_credited_once_across_repeated_events_and_steps() {
    let mut t = tracker("combat-v1", &[0, 1]);
    let first = t.step(&[defeated(0, Some(0))], 1, None);
    assert_eq!(first.enemy_defeated, 1.0);
    assert_eq!(
        first.defeats,
        [DefeatCredit {
            target: 0,
            attribution: Attribution::Carl
        }]
    );
    // The same enemy again: in the next step, twice in one step, in a bundle.
    for _ in 0..5 {
        let again = t.step(&[defeated(0, Some(0)), defeated(0, Some(1))], 4, None);
        assert_eq!(again.enemy_defeated, 0.0);
        assert!(again.defeats.is_empty());
    }
    assert_eq!(t.credited().len(), 1);
    // The second enemy is credited once, with an unknown instigator.
    let second = t.step(&[defeated(1, None)], 1, None);
    assert_eq!(
        second.defeats,
        [DefeatCredit {
            target: 1,
            attribution: Attribution::Unknown
        }]
    );
    // A foe outside the required roster pays nothing.
    let other = t.step(&[defeated(77, Some(0))], 1, None);
    assert_eq!(other.enemy_defeated, 0.0);
    assert!(other.defeats.is_empty());
    assert_eq!(Attribution::from_instigator(Some(1)), Attribution::Donut);
    assert_eq!(Attribution::from_instigator(Some(9)), Attribution::Unknown);
}

#[test]
fn cosmetic_and_unrelated_events_give_nothing() {
    let mut t = tracker("combat-v1", &[0]);
    let events = [
        CombatEvent::DonutGlance {
            tick: 1,
            instance: 1,
            target: 0,
            reason: crate::donut_moves::GlanceReason::Swing,
        },
        CombatEvent::GoblinContact {
            tick: 1,
            launch: 1,
            source: 0,
            target: 1,
        },
        CombatEvent::Hit {
            tick: 1,
            move_id: MoveId::Jab,
            instance: 1,
            target: 0,
            damage: 1,
            health_left: 1,
            hit_stop: 3,
        },
        CombatEvent::GoblinTell { tick: 1, goblin: 0 },
        CombatEvent::HeroHurt {
            tick: 1,
            source: 0,
            health_left: 3,
            hero: HeroId::Donut,
        },
    ];
    let r = t.step(&events, 0, None);
    assert_eq!(r, RewardBreakdown::default());
    assert_eq!(r.total(), 0.0);
    assert_eq!(t.total(), 0.0);
}

#[test]
fn reward_breakdown_matches_the_spec_formula_for_each_outcome() {
    let mut t = tracker("combat-v1", &[0]);
    let hurt = CombatEvent::HeroHurt {
        tick: 1,
        source: 0,
        health_left: 3,
        hero: HeroId::Carl,
    };
    let r = t.step(&[hurt, defeated(0, Some(0))], 4, Some(Outcome::Success));
    assert_eq!(r.terminal, 10.0);
    assert_eq!(r.enemy_defeated, 1.0);
    assert_eq!(r.carl_hurt, -1.0);
    assert!((r.tick + 0.004).abs() < 1e-12);
    assert!((r.total() - 9.996).abs() < 1e-12);
    // Terminal values of the other outcomes.
    for (outcome, want) in [
        (Outcome::Defeat, -10.0),
        (Outcome::Abandoned, -10.0),
        (Outcome::Timeout, 0.0),
    ] {
        let mut t = tracker("combat-v1", &[0]);
        assert_eq!(t.step(&[], 0, Some(outcome)).terminal, want, "{outcome:?}");
    }
    let mut s = tracker("sparse-v1", &[0]);
    let r = s.step(&[hurt, defeated(0, Some(0))], 100, Some(Outcome::Success));
    assert_eq!(r.total(), 1.0, "sparse pays only the outcome");
}

#[test]
fn success_episode_reward_is_credited_once_per_foe_through_the_env() {
    for (id, foes) in [("cp01_single_goblin", 1usize), ("ap_two_goblins_wide", 2)] {
        let mut env = env_for(id);
        env.reset(0).unwrap();
        let mut credits = Vec::new();
        let mut total = 0.0;
        let mut hurt = 0u32;
        let mut ticks = 0u64;
        let mut last = None;
        for tick in 0..2000u64 {
            let step = env.step(bot_action(&env.observe(), tick)).unwrap();
            credits.extend(step.reward.defeats.iter().map(|d| d.target));
            hurt += step.reward.hurt_events;
            ticks += u64::from(step.ticks_executed);
            total += step.reward.total();
            if step.outcome.is_some() {
                last = Some(step);
                break;
            }
        }
        let last = last.expect("episode ended");
        assert_eq!(last.outcome, Some(Outcome::Success), "{id}");
        assert_eq!(credits.len(), foes, "{id}: one credit per foe");
        let mut unique = credits.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), foes, "{id}: no id twice");
        assert!(credits.iter().all(|t| *t < foes as u32));
        let want = 10.0 + foes as f64 - f64::from(hurt) - 0.001 * ticks as f64;
        assert!((total - want).abs() < 1e-9, "{id}: {total} vs {want}");
        assert!((env.episode_return() - total).abs() < 1e-12);
        assert_eq!(
            env.episode_events()
                .iter()
                .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
                .count(),
            foes
        );
        // The attribution comes from the event's own instigator: Carl.
        assert!(
            last.reward
                .defeats
                .iter()
                .all(|d| d.attribution == Attribution::Carl)
        );
    }
}

#[test]
fn defeat_scores_ten_below_zero_plus_the_lethal_hit_under_combat_v1() {
    let mut env = env_for("cp01_single_goblin");
    env.reset(0).unwrap();
    let mut total = 0.0;
    let mut ticks = 0u64;
    let mut hurt = 0u32;
    let last = loop {
        let step = env.step(ActionV1::NOOP).unwrap();
        total += step.reward.total();
        ticks += u64::from(step.ticks_executed);
        hurt += step.reward.hurt_events;
        if step.outcome.is_some() {
            break step;
        }
    };
    assert_eq!(last.outcome, Some(Outcome::Defeat));
    assert_eq!(last.reward.terminal, -10.0);
    assert!(
        last.reward.hurt_events >= 1,
        "the lethal blow is also a HeroHurt"
    );
    assert!(hurt >= 1);
    let want = -10.0 - f64::from(hurt) - 0.001 * ticks as f64;
    assert!((total - want).abs() < 1e-9);
    assert_eq!(last.reward.enemy_defeated, 0.0);
}

#[test]
fn timeout_has_its_own_zero_outcome_and_sparse_pays_nothing_for_ticks() {
    let mut config = AutoplayConfig::new("ap_single_goblin_far");
    config.max_ticks = 90;
    config.reward_id = "sparse-v1".into();
    let mut env = AutoplayEnv::new(config, &root()).unwrap();
    env.reset(0).unwrap();
    let mut total = 0.0;
    let last = loop {
        let step = env.step(ActionV1::NOOP).unwrap();
        total += step.reward.total();
        if step.outcome.is_some() {
            break step;
        }
    };
    assert_eq!(last.outcome, Some(Outcome::Timeout));
    assert_eq!(total, 0.0);
    assert!(last.truncated());
}

// ---------------------------------------------------------------------------
// The ap_* scenarios and the roster rules (AP-Q17)
// ---------------------------------------------------------------------------

#[test]
fn ap_scenarios_load_run_and_terminate_under_a_scripted_policy() {
    for id in ["ap_two_goblins_wide", "ap_single_goblin_far"] {
        let mut env = env_for(id);
        let info = env.reset(5).unwrap();
        assert_eq!(info.scenario_id, id);
        assert!(!info.seed_affects_sim);
        assert_eq!(info.reward_id, "combat-v1");
        assert_eq!(info.observation_version, 1);
        let (last, _) = run_bot(&mut env);
        assert_eq!(last.outcome, Some(Outcome::Success), "{id}");
        assert!(last.terminated());
        assert!(demo::complete(&env.sim().combat), "{id}");
        assert_eq!(env.diagnostics().defeated.len(), info.required_enemies);
        // Replaying the same policy gives the same hash, tick and events.
        let (hash, tick, events) = (
            env.state_hash(),
            env.tick_count(),
            env.episode_events().to_vec(),
        );
        let (_, _) = run_bot(&mut env);
        assert_eq!((env.state_hash(), env.tick_count()), (hash, tick), "{id}");
        assert_eq!(env.episode_events(), events.as_slice(), "{id}");
        assert_eq!(
            env.step(ActionV1::NOOP),
            Err(AutoplayError::EpisodeOver(Outcome::Success))
        );
    }
}

#[test]
fn ap_scenarios_end_in_defeat_timeout_and_abandoned_when_the_policy_says_so() {
    // Idle in the near-goblin layout: Carl is defeated, never restarted.
    let mut env = env_for("ap_two_goblins_wide");
    env.reset(0).unwrap();
    let last = loop {
        let step = env.step(ActionV1::NOOP).unwrap();
        if step.outcome.is_some() {
            break step;
        }
    };
    assert_eq!(last.outcome, Some(Outcome::Defeat));
    assert_eq!(env.sim().combat.restarts, 0);

    // The far goblin never engages an idle Carl: timeout, truncated.
    let mut config = AutoplayConfig::new("ap_single_goblin_far");
    config.max_ticks = 120;
    let mut env = AutoplayEnv::new(config, &root()).unwrap();
    env.reset(0).unwrap();
    let last = loop {
        let step = env.step(ActionV1::NOOP).unwrap();
        if step.outcome.is_some() {
            break step;
        }
    };
    assert_eq!((last.outcome, last.tick), (Some(Outcome::Timeout), 120));

    // Running past it to the exit abandons the fight.
    let mut env = env_for("ap_single_goblin_far");
    env.reset(0).unwrap();
    let run_jump = ActionV1::new([2, 1, 0, 0, 0, 0]).unwrap();
    let run = ActionV1::new([2, 0, 0, 0, 0, 0]).unwrap();
    let last = (0..2000)
        .find_map(|t| {
            let step = env.step(if t % 40 < 5 { run_jump } else { run }).unwrap();
            step.outcome.map(|_| step)
        })
        .expect("running right ends the episode");
    assert_eq!(last.outcome, Some(Outcome::Abandoned));
}

#[test]
fn ap_scenario_hashes_are_pinned_and_a_fresh_sim_matches() {
    for entry in roster() {
        let mut env = env_for(entry.id);
        let info = env.reset(0).unwrap();
        assert_eq!(info.initial_hash, entry.initial_hash, "{}", entry.id);
        assert_ne!(entry.initial_hash, 0);
        assert_eq!(plain_sim(entry.fixture).state_hash(), entry.initial_hash);
    }
}

#[test]
fn roster_layouts_are_unique_and_the_ap_fixtures_are_explicitly_owned() {
    let entries = roster();
    let mut prints: Vec<String> = entries
        .iter()
        .map(|e| layout_fingerprint(e.fixture))
        .collect();
    prints.sort();
    prints.dedup();
    assert_eq!(
        prints.len(),
        entries.len(),
        "layout fingerprints must be unique"
    );

    for fixture in [&AP_TWO_GOBLINS_WIDE, &AP_SINGLE_GOBLIN_FAR] {
        assert!(fixture.name.starts_with("ap_"));
        assert!(
            !collides_with_game_fixtures(fixture.name),
            "{}",
            fixture.name
        );
        assert!(Fixture::by_name(fixture.name).is_none());
        assert!(FIXTURES.iter().all(|f| f.name != fixture.name));
        assert!(fixture.script.is_empty(), "no inherited script");
        assert!(
            FIXTURES.iter().all(|f| f.seed != fixture.seed),
            "distinct seed"
        );
        assert!(!fixture.summary.is_empty());
        assert!(
            fixture.summary.starts_with("autoplay:"),
            "no inherited summary"
        );
        // Layout-distinct from every fixture of the game.
        let print = layout_fingerprint(fixture);
        assert!(
            FIXTURES.iter().all(|f| layout_fingerprint(f) != print),
            "{} duplicates a game fixture layout",
            fixture.name
        );
    }
    assert_ne!(AP_TWO_GOBLINS_WIDE.seed, AP_SINGLE_GOBLIN_FAR.seed);

    // A script-only variant is a duplicate layout and is not on the roster.
    assert_eq!(
        layout_fingerprint(&CP01_GOBLIN_ATTACKS),
        layout_fingerprint(&CP01_SINGLE_GOBLIN)
    );
    assert!(matches!(
        AutoplayEnv::new(AutoplayConfig::new("cp01_goblin_attacks"), &root()),
        Err(AutoplayError::UnsupportedScenario(_))
    ));
    assert!(matches!(
        AutoplayEnv::new(AutoplayConfig::new("ap_unknown"), &root()),
        Err(AutoplayError::UnsupportedScenario(_))
    ));
}

#[test]
fn roster_splits_and_validation_follow_the_spec() {
    let entries = roster();
    let dev: Vec<&str> = entries
        .iter()
        .filter(|e| e.split == Split::Development)
        .map(|e| e.id)
        .collect();
    let hold: Vec<&str> = entries
        .iter()
        .filter(|e| e.split == Split::Holdout)
        .map(|e| e.id)
        .collect();
    assert_eq!(dev, ["cp01_single_goblin", "ap_two_goblins_wide"]);
    assert_eq!(hold, ["cp02_two_goblins", "ap_single_goblin_far"]);
    for entry in &entries {
        validate_fixture(entry.fixture).unwrap();
        assert_eq!(entry.fixture.name, entry.id);
    }
    // Hazard fixtures and bosses are rejected, never simplified.
    assert!(matches!(
        validate_fixture(&QA_BLAST_BREAKUP),
        Err(AutoplayError::UnsupportedScenario(_))
    ));
    // More enemies than the observation holds is rejected, not truncated.
    static CROWD: Fixture = Fixture {
        enemies: &[crate::combat_scene::EnemySpawn {
            x: 520,
            ai: crate::combat::EnemyAi::Chase,
        }; 17],
        ..CP02_TWO_GOBLINS
    };
    assert!(matches!(
        validate_fixture(&CROWD),
        Err(AutoplayError::UnsupportedScenario(_))
    ));
}

#[test]
fn envs_on_threads_match_the_single_thread_run_with_observations_and_rewards() {
    fn run(id: &'static str) -> (u64, Vec<u32>, f64) {
        let mut env = AutoplayEnv::new(
            AutoplayConfig::new(id),
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        )
        .unwrap();
        env.reset(0).unwrap();
        let mut total = 0.0;
        let mut last_bits = Vec::new();
        for (tick, action) in super::tests::action_stream(21, 300).into_iter().enumerate() {
            let step = env.step(action).unwrap();
            total += step.reward.total();
            last_bits = env.observe().to_bits();
            if step.outcome.is_some() {
                break;
            }
            let _ = tick;
        }
        (env.state_hash(), last_bits, total)
    }
    let ids = [
        "cp01_single_goblin",
        "ap_two_goblins_wide",
        "cp02_two_goblins",
        "ap_single_goblin_far",
    ];
    let sequential: Vec<_> = ids.iter().map(|id| run(id)).collect();
    let threaded: Vec<_> = std::thread::scope(|scope| {
        let handles: Vec<_> = ids.iter().map(|id| scope.spawn(move || run(id))).collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert_eq!(sequential, threaded);
}
