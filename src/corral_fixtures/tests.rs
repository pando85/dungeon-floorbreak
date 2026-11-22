use super::*;
use crate::env_kit::EnvKit;
use crate::party_scene::PartyFixtureSim;
use std::path::PathBuf;

fn kit() -> EnvKit {
    EnvKit::load(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        crate::corral_eval::LAYOUT,
    )
    .unwrap()
}

fn fixture(name: &str) -> &'static PartyFixture {
    REGISTRY
        .iter()
        .find(|e| e.fixture.name() == name)
        .unwrap()
        .fixture
}

fn run(name: &str, ticks: u64) -> PartyFixtureSim {
    let mut sim = PartyFixtureSim::new(&kit(), 360, fixture(name)).unwrap();
    sim.run_scripted(ticks);
    sim
}

use crate::combat::CombatEvent;
use crate::llama::LlamaMode;
use crate::llama_rush::{RushEnd, RushPhase};
use crate::party::HeroId;

/// The tick of the first event picked by `pick`.
fn tick_of(sim: &PartyFixtureSim, pick: impl Fn(&CombatEvent) -> Option<u64>) -> Option<u64> {
    sim.combat.events.iter().find_map(pick)
}

fn all<T>(sim: &PartyFixtureSim, pick: impl Fn(&CombatEvent) -> Option<T>) -> Vec<T> {
    sim.combat.events.iter().filter_map(pick).collect()
}

fn tell(sim: &PartyFixtureSim) -> Option<u64> {
    tick_of(sim, |e| match *e {
        CombatEvent::LlamaRushTell { tick, .. } => Some(tick),
        _ => None,
    })
}

fn commit(sim: &PartyFixtureSim) -> Option<u64> {
    tick_of(sim, |e| match *e {
        CombatEvent::LlamaRushCommit { tick, .. } => Some(tick),
        _ => None,
    })
}

fn end(sim: &PartyFixtureSim) -> Option<(u64, RushEnd)> {
    sim.combat.events.iter().find_map(|e| match *e {
        CombatEvent::LlamaRushEnd { tick, end, .. } => Some((tick, end)),
        _ => None,
    })
}

fn hurts(sim: &PartyFixtureSim) -> Vec<(u64, HeroId, u32)> {
    all(sim, |e| match *e {
        CombatEvent::HeroHurt {
            tick, hero, source, ..
        } => Some((tick, hero, source)),
        _ => None,
    })
}

const RUSH_SOURCE: u32 = crate::llama_rush::LLAMA_RUSH_SOURCE_ID;

/// Attack tick `n` is scene tick `T0 + n`.
fn attack(n: u64) -> u64 {
    T0 + n
}

#[test]
fn the_llama_tells_on_scene_tick_one_so_attack_tick_n_is_t0_plus_n() {
    for name in ["corral_rush_hits_idle_donut", "corral_rush_hits_idle_carl"] {
        let sim = run(name, 20);
        assert_eq!(tell(&sim), Some(T0), "{name}");
    }
}

#[test]
fn rush_hits_idle_donut_with_the_full_timeline() {
    let sim = run("corral_rush_hits_idle_donut", 200);
    assert_eq!(tell(&sim), Some(attack(0)));
    assert_eq!(commit(&sim), Some(attack(32)));
    // Contact on active tick 15 = attack tick 46.
    assert_eq!(
        end(&sim),
        Some((attack(46), RushEnd::Contact(HeroId::Donut)))
    );
    assert_eq!(hurts(&sim), vec![(attack(46), HeroId::Donut, RUSH_SOURCE)]);
    let llama = &sim.combat.llamas[0];
    // The llama stopped at 540 (600 - 15 x 4), Recover then the cooldowns.
    assert_eq!(llama.x, 540.0);
    assert_eq!(sim.scene.world.donut.health, 3, "Donut lost one health");
}

#[test]
fn the_recover_runs_40_ticks_then_the_cooldowns_are_set() {
    let mut sim =
        PartyFixtureSim::new(&kit(), 360, fixture("corral_rush_hits_idle_donut")).unwrap();
    let mut recover_ticks = 0;
    let mut finished = None;
    for t in 1..=200u64 {
        sim.step_scripted();
        let l = &sim.combat.llamas[0];
        if l.rush_state()
            .is_some_and(|r| r.phase == RushPhase::Recover)
        {
            recover_ticks += 1;
        }
        if t > attack(46) && finished.is_none() && l.mode == LlamaMode::Neutral {
            finished = Some((t, l.cooldown, l.corral.as_ref().unwrap().rush_cooldown));
        }
    }
    // The contact starts the Recover; the llama's hit-stop (4 ticks) freezes
    // it first, then 40 ticks run: Neutral again 44 ticks after the contact.
    assert_eq!(recover_ticks, 44, "40 recover ticks plus 4 of hit-stop");
    let (t, spit_cd, rush_cd) = finished.unwrap();
    assert_eq!(
        t,
        attack(46) + 44,
        "Neutral again after hit-stop and Recover"
    );
    // Set that tick: rush cooldown 150, spit cooldown at least 30 (CRL-Q12).
    assert_eq!((rush_cd, spit_cd), (150, 30));
}

#[test]
fn rush_hits_idle_carl() {
    let sim = run("corral_rush_hits_idle_carl", 120);
    assert_eq!(
        end(&sim),
        Some((attack(46), RushEnd::Contact(HeroId::Carl)))
    );
    assert_eq!(hurts(&sim), vec![(attack(46), HeroId::Carl, RUSH_SOURCE)]);
}

#[test]
fn point_blank_contact_is_active_tick_one() {
    let sim = run("corral_rush_point_blank", 120);
    assert_eq!(
        end(&sim),
        Some((attack(32), RushEnd::Contact(HeroId::Donut)))
    );
}

#[test]
fn the_trigger_edge_tells_at_128_and_walks_first_at_132() {
    let sim = run("corral_rush_trigger_edge", 120);
    assert_eq!(tell(&sim), Some(attack(0)));
    // Active tick 23 = attack tick 54.
    assert_eq!(
        end(&sim),
        Some((attack(54), RushEnd::Contact(HeroId::Donut)))
    );
    // 132: setup-only move of Donut, no tell on tick 1, the llama walks to 112.
    let mut sim = PartyFixtureSim::new(&kit(), 360, fixture("corral_rush_trigger_edge")).unwrap();
    sim.scene.world.donut_body.x = (E - 132) as f32;
    sim.scene.world.sync_donut_for_tests();
    sim.step_scripted();
    assert_eq!(tell(&sim), None, "132 px is outside the trigger");
    sim.run_scripted(60);
    let x = sim.combat.llamas[0].x;
    assert!(x < E as f32, "the llama walked toward her");
    assert!(tell(&sim).is_some(), "and tells once inside the trigger");
}

/// Donut at 96: jump on attack tick 4 and 43 clear; 3 and 44 are hit (spec 6).
#[test]
fn a_jump_clears_donut_on_attack_ticks_4_to_43_and_not_3_or_44() {
    for (n, hit) in [(3u64, true), (4, false), (43, false), (44, true)] {
        let mut sim =
            PartyFixtureSim::new(&kit(), 360, fixture("corral_rush_hits_idle_donut")).unwrap();
        for t in 1..=160u64 {
            let mut slots = fixture("corral_rush_hits_idle_donut").slot_inputs(t);
            slots[1].jump = t == attack(n);
            sim.step_slots(slots);
        }
        let hit_now = hurts(&sim).iter().any(|h| h.1 == HeroId::Donut);
        assert_eq!(hit_now, hit, "Donut jump on attack tick {n}");
        if !hit {
            assert_eq!(end(&sim).map(|e| e.1), Some(RushEnd::Miss), "jump {n}");
            assert_eq!(sim.combat.llamas[0].x, 504.0, "the rush ran 24 ticks");
        }
    }
}

/// Carl at 96: 12 and 42 clear; 11 and 43 are hit (spec 6).
#[test]
fn a_jump_clears_carl_on_attack_ticks_12_to_42_and_not_11_or_43() {
    for (n, hit) in [(11u64, true), (12, false), (42, false), (43, true)] {
        let mut sim =
            PartyFixtureSim::new(&kit(), 360, fixture("corral_rush_hits_idle_carl")).unwrap();
        for t in 1..=160u64 {
            let mut slots = fixture("corral_rush_hits_idle_carl").slot_inputs(t);
            slots[0].jump = t == attack(n);
            sim.step_slots(slots);
        }
        let hit_now = hurts(&sim).iter().any(|h| h.1 == HeroId::Carl);
        assert_eq!(hit_now, hit, "Carl jump on attack tick {n}");
    }
}

/// Run `name` for `ticks`, with `tweak(t, slots)` applied to each tick's inputs
/// (the fixture's own script first).
fn run_with(
    name: &str,
    ticks: u64,
    tweak: impl Fn(u64, &mut crate::input::SlotInputs),
) -> PartyFixtureSim {
    let f = fixture(name);
    let mut sim = PartyFixtureSim::new(&kit(), 360, f).unwrap();
    for t in 1..=ticks {
        let mut slots = f.slot_inputs(t);
        tweak(t, &mut slots);
        sim.step_slots(slots);
    }
    sim
}

fn donut_hurt(sim: &PartyFixtureSim) -> bool {
    hurts(sim).iter().any(|h| h.1 == HeroId::Donut)
}

#[test]
fn running_away_clears_from_attack_tick_44_and_is_hit_from_45() {
    // The fixture's own script runs from 44.
    let sim = run("corral_rush_run_away", 160);
    assert!(!donut_hurt(&sim), "{:?}", hurts(&sim));
    assert_eq!(end(&sim).map(|e| e.1), Some(RushEnd::Miss));
    let sim = run_with("corral_rush_run_away", 160, |t, slots| {
        slots[1].move_x = if t >= attack(45) && t < attack(85) {
            -1.0
        } else {
            0.0
        };
    });
    assert!(donut_hurt(&sim), "running from 45 is hit");
}

fn interrupted(sim: &PartyFixture_) -> Vec<u64> {
    all(sim, |e| match *e {
        CombatEvent::LlamaInterrupted { tick, .. } => Some(tick),
        _ => None,
    })
}
type PartyFixture_ = PartyFixtureSim;

#[test]
fn a_bolt_pressed_on_20_or_21_interrupts_the_tell_and_22_lands_in_the_commit() {
    let sim = run("corral_rush_missile_interrupt", 160);
    assert_eq!(interrupted(&sim).len(), 1, "{:?}", sim.combat.events);
    assert!(
        commit(&sim).is_none_or(|t| t > 60),
        "no commit from the interrupted tell"
    );
    // The bolt flies 11 ticks: a press on 21 lands on attack tick 32, which
    // still meets the Tell (strikes resolve before the llama's step); a press
    // on 22 lands in the commit. The sheet says 21 (design erratum).
    let press = |n: u64| {
        move |t: u64, slots: &mut crate::input::SlotInputs| {
            slots[1].attack = t == attack(n);
        }
    };
    let sim = run_with("corral_rush_missile_interrupt", 160, press(21));
    assert_eq!(interrupted(&sim).len(), 1, "a press on 21 still interrupts");
    let sim = run_with("corral_rush_missile_interrupt", 160, press(22));
    assert!(interrupted(&sim).is_empty(), "{:?}", interrupted(&sim));
    assert!(commit(&sim).is_some());
    let hits: Vec<_> = all(&sim, |e| match *e {
        CombatEvent::Hit {
            tick,
            damage,
            health_left,
            ..
        } => Some((tick, damage, health_left)),
        _ => None,
    });
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].1, 1);
    assert!(
        hits[0].0 > attack(32),
        "the bolt landed in the commit: {hits:?}"
    );
    // The rush went on and ended in contact: damage only, no stagger.
    assert_eq!(
        end(&sim).map(|e| e.1),
        Some(RushEnd::Contact(HeroId::Donut))
    );
}

#[test]
fn an_interrupted_tell_raises_both_cooldowns_to_30() {
    let mut sim =
        PartyFixtureSim::new(&kit(), 360, fixture("corral_rush_missile_interrupt")).unwrap();
    let f = fixture("corral_rush_missile_interrupt");
    let mut checked = false;
    for t in 1..=80u64 {
        sim.step_slots(f.slot_inputs(t));
        let l = &sim.combat.llamas[0];
        if !checked && l.mode == LlamaMode::Hurt {
            let c = l.corral.as_ref().unwrap();
            assert!(c.rush_cooldown >= 30, "{}", c.rush_cooldown);
            assert!(l.cooldown >= 30);
            assert!(c.state.is_none(), "the rush attack is gone");
            checked = true;
        }
    }
    assert!(checked, "the bolt staggered the llama");
}

#[test]
fn a_lethal_hit_in_the_commit_downs_the_llama_in_place_with_no_contact() {
    let sim = run("corral_rush_commit_lethal", 160);
    assert!(commit(&sim).is_some());
    assert_eq!(end(&sim), None, "no LlamaRushEnd on a lethal hit");
    assert!(!donut_hurt(&sim));
    let l = &sim.combat.llamas[0];
    assert!(matches!(l.mode, LlamaMode::Down | LlamaMode::Corpse));
    assert!(l.rush_state().is_none());
    // The legacy death slide (about 30 px, away from the attacker) is clamped
    // to the leash; the llama stays within a slide of where the bolt found it.
    assert!(
        (560.0..=640.0).contains(&l.x),
        "went down in place: {}",
        l.x
    );
}

#[test]
fn a_rush_into_a_wall_face_bonks_on_active_tick_15() {
    let sim = run("corral_rush_bonk_wall", 200);
    // Face 84 ahead: the box front rests on it on active tick 15 (attack 46).
    assert_eq!(end(&sim), Some((attack(46), RushEnd::Blocked)));
    assert_eq!(sim.combat.llamas[0].x, E as f32 - 56.0, "pivot at 56");
    assert!(
        !hurts(&sim).iter().any(|h| h.2 == RUSH_SOURCE),
        "{:?}",
        hurts(&sim)
    );
    // Bonk 60 ticks (plus the contact-free tick), no self-damage.
    assert_eq!(sim.combat.llamas[0].health, 3);
    let mut sim = PartyFixtureSim::new(&kit(), 360, fixture("corral_rush_bonk_wall")).unwrap();
    let mut bonk = 0;
    for t in 1..=200u64 {
        sim.step_scripted();
        if sim.combat.llamas[0]
            .rush_state()
            .is_some_and(|r| r.phase == RushPhase::Bonk)
        {
            bonk += 1;
        }
        let _ = t;
    }
    assert_eq!(bonk, 60);
}

#[test]
fn a_rush_stops_at_the_leash_bound_with_a_miss() {
    let sim = run("corral_rush_leash_stop", 200);
    assert_eq!(end(&sim).map(|e| e.1), Some(RushEnd::Miss));
    assert_eq!(
        sim.combat.llamas[0].x,
        E as f32 - 40.0,
        "stopped at the bound"
    );
    assert!(!donut_hurt(&sim));
}

#[test]
fn the_committed_rush_never_turns_toward_a_hero_who_lands_behind() {
    let mut sim = PartyFixtureSim::new(&kit(), 360, fixture("corral_rush_no_tracking")).unwrap();
    for _ in 1..=attack(80) {
        sim.step_scripted();
        let l = &sim.combat.llamas[0];
        if let Some(r) = l.rush_state() {
            assert_eq!(r.dir, -1);
            assert_eq!(l.facing, -1, "no turn during the rush");
        }
    }
    assert_eq!(end(&sim).map(|e| e.1), Some(RushEnd::Miss));
}

#[test]
fn two_heroes_in_the_lane_only_the_nearer_is_hit_once() {
    let sim = run("corral_rush_2p_one_hit", 160);
    let blows: Vec<_> = hurts(&sim)
        .into_iter()
        .filter(|h| h.2 == RUSH_SOURCE)
        .collect();
    assert_eq!(blows.len(), 1, "{blows:?}");
    assert_eq!(blows[0].1, HeroId::Carl);
    assert!(!donut_hurt(&sim));
}

#[test]
fn an_invulnerable_hero_is_passed_and_the_next_hero_is_hit() {
    let mut sim =
        PartyFixtureSim::new(&kit(), 360, fixture("corral_rush_invulnerable_pass")).unwrap();
    sim.scene.world.hero.invulnerable_ticks = 250;
    sim.run_scripted(160);
    let blows: Vec<_> = hurts(&sim)
        .into_iter()
        .filter(|h| h.2 == RUSH_SOURCE)
        .collect();
    assert_eq!(blows.len(), 1, "{blows:?}");
    assert_eq!(blows[0].1, HeroId::Donut, "Carl was passed");
}

#[test]
fn a_lane_end_outside_the_view_starts_no_tell() {
    // The llama starts out of the view: no tell on tick 1. It walks in, and
    // tells only once the whole lane is inside the view (CRL-Q17).
    let sim = run("corral_rush_offscreen_no_tell", 200);
    assert_ne!(
        tell(&sim),
        Some(T0),
        "no tell while the lane end is outside the view"
    );
    let first = tell(&sim);
    assert!(first.is_none() || sim.combat.llamas[0].x < E as f32 + 100.0);
}

#[test]
fn a_rush_never_spits_and_telling_is_false_from_attack_tick_32() {
    let mut sim =
        PartyFixtureSim::new(&kit(), 360, fixture("corral_rush_hits_idle_donut")).unwrap();
    for t in 1..=attack(60) {
        sim.step_scripted();
        let l = &sim.combat.llamas[0];
        if t >= attack(32) {
            assert!(!l.telling(), "tick {t}");
        }
        assert_eq!(
            l.attack_kind() == crate::llama::AttackKind::Rush,
            l.rush_state().is_some()
        );
    }
    assert!(sim.combat.globs.is_empty());
    assert!(
        all(&sim, |e| matches!(e, CombatEvent::LlamaSpit { .. })
            .then_some(()))
        .is_empty()
    );
}

#[test]
fn a_floor_donut_is_never_spat_at_the_llama_walks_into_the_128_trigger_and_rushes() {
    let sim = run("corral_spit_height_rule", 200);
    assert!(
        all(&sim, |e| matches!(e, CombatEvent::LlamaSpit { .. })
            .then_some(()))
        .is_empty()
    );
    assert!(
        all(&sim, |e| matches!(e, CombatEvent::LlamaTell { .. })
            .then_some(()))
        .is_empty()
    );
    let t = tell(&sim).expect("a rush tell");
    // 160 -> 128 at 40 px/s (2/3 px per tick): the tell starts as soon as she is
    // inside the 128 px trigger (the walk stops at 112 only if no rush started).
    assert!((46..=50).contains(&t), "tell on {t}");
    let llama = &sim.combat.llamas[0];
    assert!(llama.x < E as f32 - 28.0, "it walked toward her");
}

#[test]
fn carl_on_the_floor_is_spat_at_and_not_rushed_beyond_the_trigger() {
    let sim = run("corral_spit_carl_floor", 120);
    let first = sim.combat.events.iter().find_map(|e| match e {
        CombatEvent::LlamaTell { .. } => Some("spit"),
        CombatEvent::LlamaRushTell { .. } => Some("rush"),
        _ => None,
    });
    assert_eq!(first, Some("spit"));
}

#[test]
fn a_hopping_floor_donut_is_never_spat_at_and_a_tell_starts_only_on_a_grounded_tick() {
    let mut sim =
        PartyFixtureSim::new(&kit(), 360, fixture("corral_spit_hop_classified_floor")).unwrap();
    let mut tell_ticks = Vec::new();
    for t in 1..=300u64 {
        let before = sim.combat.events.len();
        sim.step_scripted();
        for e in &sim.combat.events[before..] {
            if matches!(e, CombatEvent::LlamaRushTell { .. }) {
                tell_ticks.push((t, sim.scene.world.donut_body.grounded));
            }
        }
    }
    assert!(
        !tell_ticks.is_empty(),
        "she stays a rush target while hopping"
    );
    assert!(tell_ticks.iter().all(|t| t.1), "{tell_ticks:?}");
    assert!(
        all(&sim, |e| matches!(e, CombatEvent::LlamaSpit { .. })
            .then_some(()))
        .is_empty()
    );
}

#[test]
fn the_m1_llama_has_party_targeting_but_never_rushes() {
    let sim = run("corral_m1_llama_never_rushes", 600);
    assert!(sim.combat.llama_party_target_on());
    assert!(sim.combat.llamas[0].corral.is_none());
    assert_eq!(tell(&sim), None);
    assert!(
        !all(&sim, |e| matches!(e, CombatEvent::LlamaTell { .. })
            .then_some(()))
        .is_empty(),
        "the legacy spit still runs"
    );
}

/// KC1, CRL-Q15: solo, Donut controlled: Carl counts `target_ai_bias_px` (40)
/// farther. 32 px nearer is not enough for him; exactly 40 px nearer is the tie,
/// which goes to Carl (the shared rule's tie-break).
#[test]
fn the_solo_bias_makes_the_llama_target_the_controlled_donut() {
    let sim = run("corral_party_target_solo_donut", 3);
    let l = &sim.combat.llamas[0];
    assert_eq!(l.held, Some(HeroId::Donut));
    assert_eq!(l.rush_state().map(|r| r.target), Some(HeroId::Donut));
    // Control: Carl 40 px nearer is the tie, and the tie goes to Carl.
    let mut sim =
        PartyFixtureSim::new(&kit(), 360, fixture("corral_party_target_solo_donut")).unwrap();
    sim.scene.world.hero.x = (E - 56) as f32;
    sim.step_scripted();
    assert_eq!(sim.combat.llamas[0].held, Some(HeroId::Carl));
}

#[test]
fn a_kicked_and_killed_llama_at_its_leash_minimum_never_leaves_the_leash_or_its_surface() {
    let mut sim = PartyFixtureSim::new(&kit(), 360, fixture("corral_kick_at_leash_edge")).unwrap();
    let y = sim.combat.llamas[0].y;
    let mut pushed = false;
    for t in 1..=200u64 {
        sim.step_scripted();
        let l = &sim.combat.llamas[0];
        assert!(
            l.x >= E as f32 && l.x <= E as f32 + 200.0,
            "tick {t}: x {}",
            l.x
        );
        assert_eq!(l.y, y, "never leaves its surface");
        pushed |= l.vx < 0.0;
        // Once the clamp stops it, vx is 0 on the same slide.
        if l.vx < 0.0 {
            assert_eq!(
                l.x, E as f32,
                "a push toward the bound happens at the bound"
            );
        }
    }
    assert!(
        pushed,
        "the kick and the lethal hit pushed it toward the bound"
    );
    let l = &sim.combat.llamas[0];
    assert!(matches!(l.mode, LlamaMode::Down | LlamaMode::Corpse));
    assert_eq!(l.x, E as f32, "the death slide is clamped too");
}

#[test]
fn a_retry_during_a_commit_restores_the_rush_state_the_seq_and_the_cooldowns() {
    let mut sim = PartyFixtureSim::new(&kit(), 360, fixture("corral_rush_retry_reset")).unwrap();
    let fresh_hash = sim.state_hash();
    sim.run_scripted(attack(40));
    let l = &sim.combat.llamas[0];
    assert!(l.rush_committed(), "mid-commit");
    assert_eq!(sim.combat.rush_seq(), 1);
    let mut sim2 = PartyFixtureSim::new(&kit(), 360, fixture("corral_rush_retry_reset")).unwrap();
    sim2.run_scripted(attack(40));
    assert_eq!(sim.state_hash(), sim2.state_hash(), "deterministic");
    sim.combat.restart(&mut sim.scene);
    let l = &sim.combat.llamas[0];
    assert!(l.rush_state().is_none());
    assert_eq!(l.x, E as f32);
    let c = l.corral.as_ref().unwrap();
    assert_eq!((c.rush_cooldown, l.cooldown), (0, 0));
    assert_eq!(c.activation, crate::llama_rush::Activation::Awake);
    assert_eq!(sim.combat.rush_seq(), 0);
    assert!(
        sim.combat.llama_party_target_on(),
        "party targeting survives the retry"
    );
    // The llama block of the restarted attempt is the fresh one, bit for bit.
    let fresh = PartyFixtureSim::new(&kit(), 360, fixture("corral_rush_retry_reset")).unwrap();
    let llama_hash = |sim: &PartyFixtureSim| {
        let mut h = 0u64;
        sim.combat.llamas[0]
            .mix_into(&mut |v| h = h.rotate_left(5) ^ v.wrapping_mul(0x9e37_79b9_7f4a_7c15));
        h
    };
    assert_eq!(llama_hash(&sim), llama_hash(&fresh));
    let _ = fresh_hash;
}

#[test]
fn a_dormant_llama_holds_until_a_living_hero_is_within_320_then_wakes_and_rushes() {
    let mut sim = PartyFixtureSim::new(&kit(), 360, fixture("corral_wake_dormant")).unwrap();
    let mut woke = None;
    for t in 1..=200u64 {
        sim.step_scripted();
        let l = &sim.combat.llamas[0];
        let c = l.corral.as_ref().unwrap();
        if woke.is_none() && c.activation.awake() {
            woke = Some(t);
            // Within 320 px of her pivot at that moment.
            let d = (l.x - sim.scene.world.donut_body.x).abs();
            assert!(d <= 320.0, "woke at {d}");
        }
        if woke.is_none() {
            assert_eq!(l.x, E as f32, "a dormant llama holds its position");
            assert_eq!(l.mode, LlamaMode::Neutral);
        }
    }
    let woke = woke.expect("it woke");
    assert!(woke > 5, "not on the first tick: {woke}");
    assert!(
        tell(&sim).is_some_and(|t| t >= woke),
        "tells only after waking"
    );
}

#[test]
fn the_opt_in_load_check_refuses_a_bad_surface_leash_or_index() {
    use crate::combat_scene::CombatState;
    let kit = kit();
    let f = fixture("corral_rush_hits_idle_donut");
    let scene = crate::corridor::SceneSim::new_at(&kit, 360, f.base.carl_x).unwrap();
    let combat = CombatState::new(f.base, &kit, &scene).unwrap();
    let rush = Some(RushOptIn {
        profile: llama_rush::active(),
        leash: Leash {
            min: 400.0,
            max: 800.0,
        },
    });
    let ok = LlamaOptIn {
        index: 0,
        feet: None,
        facing: -1,
        dormant: false,
        health: None,
        rush,
    };
    let solids = &scene.world.solids;
    assert!(
        combat
            .validate_llama_opt_ins_for_test(solids, &[ok])
            .is_ok()
    );
    let err = |opt: LlamaOptIn| {
        combat
            .validate_llama_opt_ins_for_test(solids, &[opt])
            .unwrap_err()
    };
    // A surface that is neither the floor line nor the top of a solid.
    assert!(
        err(LlamaOptIn {
            feet: Some(32.0),
            ..ok
        })
        .contains("surface")
    );
    // A spawn column outside its own leash (N-h).
    let bad = Some(RushOptIn {
        profile: llama_rush::active(),
        leash: Leash {
            min: 100.0,
            max: 500.0,
        },
    });
    assert!(err(LlamaOptIn { rush: bad, ..ok }).contains("leash"));
    // An inverted leash.
    let inv = Some(RushOptIn {
        profile: llama_rush::active(),
        leash: Leash {
            min: 800.0,
            max: 400.0,
        },
    });
    assert!(err(LlamaOptIn { rush: inv, ..ok }).contains("leash"));
    // A llama that does not exist, a bad facing, a dormant llama with no rush.
    assert!(err(LlamaOptIn { index: 3, ..ok }).contains("no such llama"));
    assert!(err(LlamaOptIn { facing: 2, ..ok }).contains("facing"));
    assert!(
        err(LlamaOptIn {
            rush: None,
            dormant: true,
            ..ok
        })
        .contains("dormant")
    );
    // The same llama twice.
    assert!(
        combat
            .validate_llama_opt_ins_for_test(solids, &[ok, ok])
            .unwrap_err()
            .contains("twice")
    );
}

/// KC12, CP-Q40: the reinforced haymaker pierces the llama's committed rush
/// like a goblin's committed swing. The fist is the only difference between the
/// fixture and its control.
#[test]
fn the_reinforced_haymaker_pierces_a_committed_rush_and_the_control_does_not() {
    let sim = run("corral_rush_pierce_haymaker", attack(60));
    let hay = all(&sim, |e| match *e {
        CombatEvent::Hit { tick, move_id, .. } if format!("{move_id:?}") == "Haymaker" => {
            Some(tick)
        }
        _ => None,
    });
    assert_eq!(hay, vec![attack(45)], "the haymaker lands in the commit");
    assert!(
        commit(&sim).is_some_and(|c| c < attack(45)),
        "the rush was committed"
    );
    let interrupted = all(&sim, |e| match *e {
        CombatEvent::LlamaInterrupted { tick, .. } => Some(tick),
        _ => None,
    });
    assert_eq!(
        interrupted,
        vec![attack(45)],
        "pierced: the rush is cancelled"
    );
    assert_eq!(end(&sim), None, "no LlamaRushEnd, the box is gone");
    assert!(
        hurts(&sim).is_empty(),
        "Carl is never hurt: {:?}",
        hurts(&sim)
    );
    assert_eq!(sim.combat.pierce_count(), 1);
    // The llama took the pierce's stagger and the interrupt cooldown floors.
    let l = &sim.combat.llamas[0];
    assert!(l.rush_state().is_none());
    assert!(l.corral.as_ref().unwrap().rush_cooldown >= 30 || l.mode != LlamaMode::Neutral);

    let control = run("corral_rush_pierce_control", attack(60));
    assert!(
        interrupted_ticks(&control).is_empty(),
        "no pierce without the fist"
    );
    assert_eq!(control.combat.pierce_count(), 0);
    let hay = all(&control, |e| match *e {
        CombatEvent::Hit { tick, move_id, .. } if format!("{move_id:?}") == "Haymaker" => {
            Some(tick)
        }
        _ => None,
    });
    assert_eq!(hay, vec![attack(45)], "the same string lands the same hit");
    assert!(
        matches!(end(&control), Some((_, RushEnd::Contact(HeroId::Carl)))),
        "the committed rush went on and hit Carl: {:?}",
        end(&control)
    );
    assert_eq!(hurts(&control).len(), 1);
}

fn interrupted_ticks(sim: &PartyFixtureSim) -> Vec<u64> {
    all(sim, |e| match *e {
        CombatEvent::LlamaInterrupted { tick, .. } => Some(tick),
        _ => None,
    })
}

/// N-e (#201 review): a static solid of 16 px or less in a corral llama's leash
/// band is refused at load (the llama would walk through it, CRL-Q32), and a
/// tall one is not.
#[test]
fn a_low_static_solid_in_the_leash_band_is_refused() {
    use crate::prototype::{Block, Material, Rect};
    let kit = kit();
    let f = fixture("corral_rush_hits_idle_donut");
    let scene = crate::corridor::SceneSim::new_at(&kit, 360, f.base.carl_x).unwrap();
    let combat = crate::combat_scene::CombatState::new(f.base, &kit, &scene).unwrap();
    let rush = Some(RushOptIn {
        profile: llama_rush::active(),
        leash: Leash {
            min: 400.0,
            max: 800.0,
        },
    });
    let opt = LlamaOptIn {
        index: 0,
        feet: None,
        facing: -1,
        dormant: false,
        health: None,
        rush,
    };
    let floor = scene.world.hero_foot_y();
    let block = |height: f32, left: f32| Block {
        rect: Rect {
            left,
            bottom: floor,
            width: 20.0,
            height,
        },
        material: Material::Stone,
        intact: true,
    };
    let err = combat
        .validate_llama_opt_ins_for_test(&[block(10.0, 500.0)], &[opt])
        .unwrap_err();
    assert!(err.contains("16 px or less"), "{err}");
    assert!(
        combat
            .validate_llama_opt_ins_for_test(&[block(10.0, 1200.0)], &[opt])
            .is_ok(),
        "outside the band"
    );
    assert!(
        combat
            .validate_llama_opt_ins_for_test(&[block(32.0, 500.0)], &[opt])
            .is_ok(),
        "a tall block is a solid"
    );
    let mut broken = block(10.0, 500.0);
    broken.intact = false;
    assert!(
        combat
            .validate_llama_opt_ins_for_test(&[broken], &[opt])
            .is_ok(),
        "a broken block is no solid"
    );
}

/// The corral hooks match exact names only: every other fixture, the
/// evaluator's `corral_eval_*` stages included, gets no opt-in, no CF02 arm and
/// no start column.
#[test]
fn the_hooks_are_exact_names_and_leave_every_other_fixture_alone() {
    for name in [
        "corral_eval_m1",
        "corral_eval_m2",
        "corral_nope",
        "cp05_llama_spit",
        "eng_ram",
    ] {
        assert!(llama_opt_ins(name).is_none(), "{name}");
        assert!(!runs_cf02(name), "{name}");
        assert!(donut_start_x(name).is_none(), "{name}");
    }
    for e in &REGISTRY {
        let name = e.fixture.name();
        assert!(
            name.starts_with("corral_") && !name.starts_with("corral_eval"),
            "{name}"
        );
        assert!(
            runs_cf02(name) && crate::cf02::fixture_runs_cf02(name),
            "{name}"
        );
        assert!(llama_opt_ins(name).is_some());
        assert_eq!(
            crate::cf02_fixtures::donut_start_x(name),
            donut_start_x(name),
            "{name}"
        );
        // Every name is unique and the listing is the replayable ones.
        assert_eq!(
            REGISTRY.iter().filter(|o| o.fixture.name() == name).count(),
            1
        );
        assert_eq!(
            crate::party_scene::PartyFixture::by_name(name).is_some(),
            e.replay,
            "{name}"
        );
    }
}

/// DM-Q18, CRL-Q18: the rush has no `EvadeSource`, so an armed reflex never
/// evades it: the rush hits Donut, no `HeroEvaded` is logged, and the hit resets
/// the reflex's wait like every `HeroHurt` she takes (DM-Q13).
#[test]
fn an_armed_reflex_never_evades_the_rush_and_the_hit_resets_it() {
    let mut sim = PartyFixtureSim::new(&kit(), 360, fixture("corral_rush_not_evadable")).unwrap();
    sim.combat.arm_reflex_for_test();
    assert!(sim.combat.reflex().armed());
    sim.run_scripted(attack(50));
    assert!(donut_hurt(&sim), "the rush hit her: {:?}", hurts(&sim));
    assert!(
        all(&sim, |e| matches!(e, CombatEvent::HeroEvaded { .. })
            .then_some(()))
        .is_empty(),
        "no evade"
    );
    assert!(!sim.combat.reflex().armed(), "the hurt disarmed the reflex");
}

/// R1: a llama spawns at an authored surface and height (`y = feet + 19.5`),
/// the load check widened to authored surfaces: the top of intact solids under
/// the **whole** body. A legacy spawn keeps `enemy_foot_y`.
#[test]
fn an_authored_surface_is_the_top_of_solids_under_the_whole_body() {
    use crate::prototype::{Block, Material, Rect};
    let kit = kit();
    let base = fixture("corral_rush_hits_idle_donut").base;
    let probe: &'static Fixture = Box::leak(Box::new(Fixture {
        name: "surface_probe",
        summary: base.summary,
        seed: base.seed,
        carl_x: base.carl_x,
        enemies: base.enemies,
        shore_dx: base.shore_dx,
        boss: base.boss,
        engineers: base.engineers,
        llamas: base.llamas,
        charges: base.charges,
        script: base.script,
        ticks: base.ticks,
    }));
    let mut scene = crate::corridor::SceneSim::new_at(&kit, 360, probe.carl_x).unwrap();
    let floor = scene.world.hero_foot_y();
    let dock = |left: f32, width: f32| Block {
        rect: Rect {
            left,
            bottom: floor,
            width,
            height: 32.0,
        },
        material: Material::Stone,
        intact: true,
    };
    // Two adjoining blocks cover the whole 46 px body (x 574..626): one block
    // alone covers only a part of it.
    scene.world.solids.push(dock(560.0, 40.0));
    let rush = Some(RushOptIn {
        profile: llama_rush::active(),
        leash: Leash {
            min: 560.0,
            max: 640.0,
        },
    });
    let opt = LlamaOptIn {
        index: 0,
        feet: Some(floor + 32.0),
        facing: -1,
        dormant: false,
        health: None,
        rush,
    };
    let half = crate::combat_scene::CombatState::new(probe, &kit, &scene).unwrap();
    let err = half
        .validate_llama_opt_ins_for_test(&scene.world.solids, &[opt])
        .unwrap_err();
    assert!(err.contains("surface"), "{err}");
    scene.world.solids.push(dock(600.0, 40.0));
    let combat = crate::combat_scene::CombatState::new(probe, &kit, &scene)
        .unwrap()
        .with_llama_opt_ins(&scene, vec![opt])
        .unwrap();
    let l = &combat.llamas[0];
    assert_eq!(l.feet(), floor + 32.0);
    assert_eq!(l.y, floor + 32.0 + crate::llama::LLAMA_HALF_HEIGHT);
    assert!(l.corral.is_some());
    // A legacy spawn is unchanged.
    let legacy = crate::combat_scene::CombatState::new(probe, &kit, &scene).unwrap();
    assert_eq!(legacy.llamas[0].feet(), floor);
    // A broken support is no surface.
    let n = scene.world.solids.len();
    scene.world.solids[n - 2].intact = false;
    scene.world.solids[n - 1].intact = false;
    assert!(
        legacy
            .validate_llama_opt_ins_for_test(&scene.world.solids, &[opt])
            .is_err()
    );
}
