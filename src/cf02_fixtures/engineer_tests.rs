//! ENEMY-GAMEPLAY-ENGINEER fixture tests: every fixture of
//! `docs/01-design/enemy-gameplay-engineer.md` section 14 replays identically and
//! reproduces the outcome of its row (ticks, damage, events), its golden is pinned,
//! and the gate tests hold.

use super::*;
use crate::combat::{CombatEvent, MoveId};
use crate::engineer::EngineerPhase;
use crate::env_kit::EnvKit;
use crate::goblin::GoblinMode;
use crate::party::HeroId;
use crate::party_scene::PartyFixtureSim;

fn kit() -> &'static EnvKit {
    static KIT: std::sync::OnceLock<EnvKit> = std::sync::OnceLock::new();
    KIT.get_or_init(|| {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        crate::cf02::arena::load_arena(&root).unwrap()
    })
}

fn sim_of(name: &str) -> PartyFixtureSim {
    let fixture = find_party_fixture(name).unwrap();
    PartyFixtureSim::new(kit(), 360, fixture).unwrap()
}

fn replay(name: &str) -> PartyFixtureSim {
    let mut sim = sim_of(name);
    let ticks = find_party_fixture(name).unwrap().base.ticks;
    sim.run_scripted(ticks);
    sim
}

fn events(sim: &PartyFixtureSim) -> &[CombatEvent] {
    &sim.combat.events
}

fn tells(sim: &PartyFixtureSim, goblin: u32) -> Vec<u64> {
    events(sim)
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::GoblinTell { tick, goblin: g } if g == goblin => Some(tick),
            _ => None,
        })
        .collect()
}

/// `(tick, hero)` of every `HeroHurt` by the goblin `source`.
fn hurts(sim: &PartyFixtureSim, source: u32) -> Vec<(u64, HeroId)> {
    events(sim)
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::HeroHurt {
                tick,
                source: s,
                hero,
                ..
            } if s == source => Some((tick, hero)),
            _ => None,
        })
        .collect()
}

fn bonks(sim: &PartyFixtureSim) -> Vec<(u64, u32, Option<u32>)> {
    events(sim)
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::RamBonk {
                tick,
                goblin,
                instigator,
            } => Some((tick, goblin, instigator)),
            _ => None,
        })
        .collect()
}

fn hits(sim: &PartyFixtureSim) -> Vec<(u64, MoveId, u32)> {
    events(sim)
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::Hit {
                tick,
                move_id,
                target,
                ..
            } => Some((tick, move_id, target)),
            _ => None,
        })
        .collect()
}

/// Run `ticks` and collect `f(sim)` after every tick.
fn trace<T>(name: &str, ticks: u64, f: impl Fn(&PartyFixtureSim) -> T) -> Vec<T> {
    let mut sim = sim_of(name);
    (0..ticks)
        .map(|_| {
            sim.step_scripted();
            f(&sim)
        })
        .collect()
}

/// The golden of every Engineer fixture (the committed value of its final state).
const ENGINEER_GOLDENS: [(&str, u64); 29] = [
    ("eng_ram_hits_idle_carl", 0xb793d5b325ae049f),
    ("eng_jump_over_ram", 0x4ae489a7b31f448e),
    ("eng_jump_window_edges_7", 0x494f2ce14e01f3ec),
    ("eng_jump_window_edges_8", 0x0766a73b1f24c040),
    ("eng_jump_window_edges_39", 0x9bce359c6267e443),
    ("eng_jump_window_edges_40", 0x69944234cdeb41f6),
    ("eng_bailout_does_not_clear", 0xa195acb7054d40c9),
    ("eng_bailout_escapes_at_range", 0xaa8f5a2b5fc3a0e9),
    ("eng_run_out_of_range", 0x6c8c976b2f4c0aa3),
    ("eng_snap_rush", 0xc243b052854f723d),
    ("eng_point_interrupt_jab", 0x7bf2fb7b156c1388),
    ("eng_point_interrupt_swat", 0x0cf9c19231fd3cab),
    ("eng_parapet_bonk", 0x7fd7a7e7f1972f21),
    ("eng_parapet_gap_whiff", 0xb15bb77399979a27),
    ("eng_end_wall_bonk_kill", 0x1217716574c175ca),
    ("eng_lane_blocked_goblin", 0x7ae1ce127a3eeb43),
    ("eng_lane_blocked_parapet", 0xc203254efc7948b3),
    ("eng_perch_not_targeted", 0x8e71c6152918001e),
    ("eng_perch_dive", 0x3cb42d7088a2e8ca),
    ("eng_bowls_goblin", 0xb5558b83c58d82e6),
    ("eng_shore_trip", 0x7fc32e9648588641),
    ("eng_shore_intact", 0xa4223215f7c15a7f),
    ("eng_defeat_scares_crew", 0x40b477aeff19fd09),
    ("eng_kick_launch_contact", 0xefbbcf0840e6ffdc),
    ("eng_2p_targets_nearest", 0xbd0214303e42939e),
    ("eng_2p_partner_in_lane", 0x475de5f61a0c5d12),
    ("eng_solo_assist", 0xf28556f86e16a187),
    ("cf02_arena_eng", 0x396b00cb46cfbc3b),
    ("cf02_arena_eng_cp2", 0x9a127517724b02af),
];

#[test]
#[ignore = "prints the Engineer goldens"]
fn print_engineer_goldens() {
    for fixture in ENGINEER_FIXTURES {
        println!(
            "    (\"{}\", {:#018x}),",
            fixture.name(),
            replay(fixture.name()).state_hash()
        );
    }
}

#[test]
fn every_engineer_fixture_replays_identically_and_matches_its_golden() {
    assert_eq!(ENGINEER_GOLDENS.len(), ENGINEER_FIXTURES.len());
    for (fixture, (name, golden)) in ENGINEER_FIXTURES.iter().zip(ENGINEER_GOLDENS) {
        assert_eq!(fixture.name(), name);
        let (a, b) = (replay(name), replay(name));
        assert_eq!(a.combat.events, b.combat.events, "{name} events replay");
        assert_eq!(a.state_hash(), b.state_hash(), "{name} replays");
        assert_eq!(a.state_hash(), golden, "{name}: {}", a.state_line());
    }
}

#[test]
fn the_engineer_fixtures_are_listed_findable_and_run_under_cf02_rules() {
    let listed = list_fixtures("eng_");
    let own: Vec<_> = ENGINEER_FIXTURES
        .iter()
        .map(|f| f.name())
        .filter(|n| n.starts_with("eng_"))
        .collect();
    assert_eq!(listed, own);
    assert_eq!(list_fixtures("cf02_arena_eng").len(), 2);
    for fixture in ENGINEER_FIXTURES {
        let name = fixture.name();
        assert!(find_party_fixture(name).is_some());
        let sim = replay_ticks(name, 1);
        assert!(sim.combat.cf02().is_some_and(|t| t.cf02_rules), "{name}");
        assert!(
            sim.combat.enemies.iter().any(|g| g.is_engineer()),
            "{name} has an Engineer"
        );
    }
}

fn replay_ticks(name: &str, ticks: u64) -> PartyFixtureSim {
    let mut sim = sim_of(name);
    sim.run_scripted(ticks);
    sim
}

#[test]
fn the_ram_hits_an_idle_carl_on_active_tick_10_and_the_recover_and_cooldown_follow() {
    let sim = replay("eng_ram_hits_idle_carl");
    assert_eq!(tells(&sim, 0)[0], 1, "attack tick 0 is scene tick 1");
    // Active tick 10 is attack tick 44 (scene tick 45); 1 damage.
    assert_eq!(hurts(&sim, 0)[0], (45, HeroId::Carl));
    assert!(matches!(
        events(&sim)
            .iter()
            .find(|e| matches!(e, CombatEvent::HeroHurt { .. })),
        Some(CombatEvent::HeroHurt { health_left: 3, .. })
    ));
    // The pivot stops where the hit ended the ram: 200 + 10 * 4.
    let phases = trace("eng_ram_hits_idle_carl", 100, |s| {
        (s.combat.enemies[0].x, s.combat.enemies[0].engineer_phase())
    });
    assert_eq!(phases[44].0, 240.0);
    assert_eq!(phases[45].1, Some(EngineerPhase::Recover));
    // Recover 40 (attack ticks 45..84), then the 60-tick cooldown: the second tell
    // cannot start before attack tick 145 (scene tick 146).
    assert!(tells(&sim, 0)[1] >= 146, "{:?}", tells(&sim, 0));
}

#[test]
fn a_jump_on_attack_tick_20_clears_the_ram_and_the_jab_hits_in_the_recover() {
    let sim = replay("eng_jump_over_ram");
    assert!(
        hurts(&sim, 0).iter().all(|h| h.0 > 100),
        "{:?}",
        hurts(&sim, 0)
    );
    // The whiff ends on active tick 20 (attack tick 54) at pivot 280, 8 px past Carl.
    let rows = trace("eng_jump_over_ram", 70, |s| {
        (s.combat.enemies[0].x, s.combat.enemies[0].engineer_phase())
    });
    assert_eq!(rows[54].0, 280.0);
    assert_eq!(rows[55].1, Some(EngineerPhase::Recover));
    // Carl is back on the ground by attack tick 69 (scene tick 70).
    let carl = trace("eng_jump_over_ram", 71, |s| s.scene.world.hero.grounded);
    assert!(!carl[60] && carl[69], "lands on attack tick 69");
    // The jab (pressed on attack tick 70) hits inside the recover (55..94) and staggers.
    let sim = replay_ticks("eng_jump_over_ram", 80);
    assert_eq!(hits(&sim), vec![(74, MoveId::Jab, 0)]);
    assert_eq!(sim.combat.enemies[0].mode, GoblinMode::Hurt);
}

#[test]
fn the_jump_window_edges_at_the_standoff() {
    let first_hurt = |name: &str| {
        let sim = replay_ticks(name, 150);
        hurts(&sim, 0).first().map(|h| h.0)
    };
    // Attack tick 7: a hit on active tick 17 (attack tick 51, scene tick 52).
    assert_eq!(first_hurt("eng_jump_window_edges_7"), Some(52));
    assert_eq!(first_hurt("eng_jump_window_edges_8"), None);
    assert_eq!(first_hurt("eng_jump_window_edges_39"), None);
    // Attack tick 40: a hit on active tick 10 (scene tick 45).
    assert_eq!(first_hurt("eng_jump_window_edges_40"), Some(45));
}

/// The highest feet height Carl reaches in the first `ticks` ticks.
fn peak_feet(name: &str, ticks: u64) -> f32 {
    let floor = sim_of(name).scene.world.hero.y;
    trace(name, ticks, |s| s.scene.world.hero.y - floor)
        .into_iter()
        .fold(0.0, f32::max)
}

#[test]
fn the_bail_out_hop_never_clears_the_ram_and_only_escapes_by_distance() {
    // 40 px: the hop (apex 30 px) starts and Carl takes the ram.
    let peak = peak_feet("eng_bailout_does_not_clear", 60);
    assert!((20.0..33.0).contains(&peak), "bail-out apex {peak}");
    let sim = replay_ticks("eng_bailout_does_not_clear", 150);
    assert_eq!(hurts(&sim, 0).len(), 1, "{:?}", hurts(&sim, 0));
    // 72 px: the same hop escapes by distance.
    let peak = peak_feet("eng_bailout_escapes_at_range", 60);
    assert!((20.0..33.0).contains(&peak), "bail-out apex {peak}");
    let sim = replay_ticks("eng_bailout_escapes_at_range", 150);
    assert!(hurts(&sim, 0).is_empty());
}

#[test]
fn running_out_of_range_whiffs() {
    let sim = replay("eng_run_out_of_range");
    assert!(hurts(&sim, 0).is_empty());
    assert_eq!(tells(&sim, 0).len(), 1);
}

#[test]
fn the_snap_fires_on_attack_tick_15_and_the_ram_hits_on_active_tick_1() {
    let rows = trace("eng_snap_rush", 21, |s| {
        let e = s.combat.enemies[0].engineer_state();
        (e.snapped, e.tell_len, s.combat.enemies[0].engineer_phase())
    });
    // Scene tick 16 is attack tick 15: the snap fires and the point ends.
    assert!(!rows[14].0 && rows[15].0, "{rows:?}");
    assert_eq!(rows[15].1, 16);
    // Launch 16..18, the ram's first tick is attack tick 19 (scene tick 20).
    assert_eq!(rows[16].2, Some(EngineerPhase::Launch));
    assert_eq!(rows[18].2, Some(EngineerPhase::Launch));
    assert_eq!(rows[19].2, Some(EngineerPhase::Recover), "the hit ended it");
    let sim = replay("eng_snap_rush");
    assert_eq!(hurts(&sim, 0)[0], (20, HeroId::Carl));
}

#[test]
fn a_jab_in_the_point_staggers_him_with_no_launch_and_the_hurt_cooldown() {
    let sim = replay("eng_point_interrupt_jab");
    assert_eq!(hits(&sim), vec![(15, MoveId::Jab, 0)]);
    assert!(events(&sim).iter().any(|e| matches!(
        e,
        CombatEvent::GoblinInterrupted {
            tick: 15,
            goblin: 0
        }
    )));
    let rows = trace("eng_point_interrupt_jab", 60, |s| {
        let g = &s.combat.enemies[0];
        (g.mode, g.token_held(), g.engineer_state().snapped)
    });
    assert_eq!(rows[14].0, GoblinMode::Hurt);
    assert!(!rows[14].1, "the token went back");
    assert!(
        rows.iter().all(|r| !r.2),
        "inside the snap line: never armed"
    );
    // The hurt path: the next tell waits for the stagger and HURT_COOLDOWN (20).
    let second = tells(&sim, 0)[1];
    assert!(second >= 15 + 20, "second tell {second}");
    assert!(hurts(&sim, 0).iter().all(|h| h.0 > second));
}

#[test]
fn donuts_swat_interrupts_the_point_without_a_snap_and_sets_the_light_guard() {
    let sim = replay_ticks("eng_point_interrupt_swat", 30);
    assert!(hits(&sim).contains(&(25, MoveId::Swat, 0)));
    assert!(events(&sim).iter().any(|e| matches!(
        e,
        CombatEvent::GoblinInterrupted {
            tick: 25,
            goblin: 0
        }
    )));
    assert!(sim.combat.enemies[0].guard.is_some(), "light guard set");
    let snapped = trace("eng_point_interrupt_swat", 25, |s| {
        s.combat.enemies[0].engineer_state().snapped
    });
    assert!(snapped.iter().all(|s| !s), "Donut is not the target");
}

#[test]
fn the_pot_meets_the_parapet_face_and_the_bonk_costs_him_one_health() {
    let sim = replay("eng_parapet_bonk");
    // Active tick 16 is attack tick 50 (scene tick 51); credited to Carl.
    assert_eq!(bonks(&sim), vec![(51, 0, Some(0))]);
    assert!(hurts(&sim, 0).is_empty());
    let rows = trace("eng_parapet_bonk", 130, |s| {
        let g = &s.combat.enemies[0];
        (g.x, g.health, g.engineer_phase())
    });
    assert_eq!(rows[51].0, 792.0);
    assert_eq!(rows[51].1, 2);
    assert_eq!(rows[51].2, Some(EngineerPhase::Bonk));
    // Bonk 60 ticks (scene ticks 52..111), the cooldown 60: the next tell is 171+.
    assert!(rows[110].2 == Some(EngineerPhase::Bonk) && rows[111].2.is_none());
    let sim = replay_ticks("eng_parapet_bonk", 160);
    assert_eq!(tells(&sim, 0).len(), 1);
}

#[test]
fn a_24_px_gap_is_a_whiff_at_pivot_796() {
    let sim = replay("eng_parapet_gap_whiff");
    assert!(bonks(&sim).is_empty());
    assert!(hurts(&sim, 0).is_empty());
    let x = trace("eng_parapet_gap_whiff", 70, |s| s.combat.enemies[0].x);
    assert_eq!(x[60], 796.0);
}

#[test]
fn three_bonks_at_the_end_wall_defeat_him_in_place_credited_to_carl() {
    let sim = replay("eng_end_wall_bonk_kill");
    let b = bonks(&sim);
    assert_eq!(b.len(), 3, "{b:?}");
    assert!(b.iter().all(|x| x.1 == 0 && x.2 == Some(0)));
    // The first bonk is on active tick 16 and stops him at 40; the next two are
    // point-blank on active tick 1 (attack tick 35 of their tells).
    assert_eq!(b[0].0, 51);
    let tell = tells(&sim, 0);
    assert_eq!(b[1].0, tell[1] + 35);
    assert_eq!(b[2].0, tell[2] + 35);
    let x = trace("eng_end_wall_bonk_kill", 52, |s| s.combat.enemies[0].x);
    assert_eq!(x[51], 40.0);
    let e = &sim.combat.enemies[0];
    assert_eq!(e.health, 0);
    assert!(matches!(e.mode, GoblinMode::Down | GoblinMode::Corpse));
    assert!(hurts(&sim, 0).is_empty());
    let defeat: Vec<_> = events(&sim)
        .iter()
        .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
        .collect();
    assert_eq!(defeat.len(), 1);
    assert!(matches!(
        defeat[0],
        CombatEvent::EnemyDefeated {
            tick: 365,
            target: 0,
            instigator: Some(0),
            ..
        }
    ));
    // No CINE trigger events (shore, contact) at all.
    assert!(!events(&sim).iter().any(|e| matches!(
        e,
        CombatEvent::ShoreBroken { .. } | CombatEvent::GoblinContact { .. }
    )));
}

#[test]
fn a_goblin_or_the_parapet_in_the_lane_blocks_the_tell_and_the_token() {
    let mut sim = sim_of("eng_lane_blocked_goblin");
    for _ in 0..240 {
        sim.step_scripted();
        assert!(!sim.combat.enemies[1].token_held());
    }
    assert!(tells(&sim, 1).is_empty());
    let sim = replay("eng_lane_blocked_parapet");
    assert!(tells(&sim, 0).is_empty());
}

#[test]
fn a_hero_on_the_perch_is_not_a_target_and_he_holds_outside_the_slab() {
    let sim = replay("eng_perch_not_targeted");
    assert!(tells(&sim, 0).is_empty());
    let x = sim.combat.enemies[0].x;
    assert!((x - 1028.0).abs() < 1.5, "he holds near 1028: {x}");
    // Carl stands on the perch top (80 px up).
    let carl = &sim.scene.world.hero;
    assert!((955.0..957.0).contains(&carl.x), "{}", carl.x);
}

#[test]
fn a_dive_from_the_perch_knocks_the_waiting_engineer_down() {
    let sim = replay_ticks("eng_perch_dive", 120);
    assert!(
        hits(&sim)
            .iter()
            .any(|h| h.1 == MoveId::DiveStomp && h.2 == 0)
    );
    assert_eq!(sim.combat.enemies[0].mode, GoblinMode::Downed);
    assert_eq!(sim.combat.enemies[0].health, 2);
}

#[test]
fn the_ram_bowls_the_goblin_beyond_carl_with_a_ram_stamp() {
    let sim = replay("eng_bowls_goblin");
    let bowl: Vec<_> = events(&sim)
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::BodyContact {
                tick,
                launch,
                source,
                target,
                instigator,
            } => Some((tick, launch, source, target, instigator)),
            _ => None,
        })
        .collect();
    // Active tick 14 is attack tick 48 (scene tick 49); the stamp is 0x8000_0001.
    assert_eq!(bowl, vec![(49, 0x8000_0001, 1, 0, Some(0))]);
    let rows = trace("eng_bowls_goblin", 52, |s| {
        (
            s.combat.enemies[0].mode,
            s.combat.enemies[1].engineer_phase(),
        )
    });
    assert!(matches!(
        rows[49].0,
        GoblinMode::Launched | GoblinMode::Downed
    ));
    assert_eq!(rows[49].1, Some(EngineerPhase::Recover));
    let sim = replay_ticks("eng_bowls_goblin", 150);
    assert!(hurts(&sim, 1).is_empty());
}

#[test]
fn the_ram_trips_a_damaged_shore_once_and_an_intact_shore_not_at_all() {
    let trips = |name: &str| {
        events(&replay(name))
            .iter()
            .filter(|e| matches!(e, CombatEvent::ShoreTripped { .. }))
            .count()
    };
    assert_eq!(trips("eng_shore_trip"), 1);
    assert_eq!(trips("eng_shore_intact"), 0);
}

#[test]
fn his_defeat_is_a_spectacular_minus_35_for_the_goblin_that_witnesses_it() {
    let mut sim = sim_of("eng_defeat_scares_crew");
    let nerve = |s: &PartyFixtureSim| s.combat.goblin_morale(0).unwrap().nerve();
    let mut before = nerve(&sim);
    let mut defeated_on = None;
    for _ in 0..80 {
        before = nerve(&sim);
        sim.step_scripted();
        if defeated_on.is_none()
            && events(&sim)
                .iter()
                .any(|e| matches!(e, CombatEvent::EnemyDefeated { target: 1, .. }))
        {
            defeated_on = Some(sim.scene.world.tick);
            break;
        }
    }
    assert_eq!(defeated_on, Some(47));
    assert_eq!(
        i32::from(before) - i32::from(nerve(&sim)),
        35,
        "exactly -35"
    );
}

#[test]
fn a_push_kick_launches_the_pointing_engineer_into_the_goblin_behind_him() {
    let sim = replay_ticks("eng_kick_launch_contact", 40);
    // The kick pressed on attack tick 0 lands during the point (tick 14).
    assert!(hits(&sim).contains(&(14, MoveId::PushKick, 1)));
    let contact: Vec<_> = events(&sim)
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::GoblinContact { source, target, .. } => Some((source, target)),
            _ => None,
        })
        .collect();
    assert_eq!(contact, vec![(1, 0)]);
    assert!(matches!(
        sim.combat.enemies[0].mode,
        GoblinMode::Launched | GoblinMode::Downed
    ));
}

#[test]
fn in_two_player_he_points_at_the_nearer_hero_and_the_first_hero_overlapped_is_hit() {
    let sim = replay("eng_2p_targets_nearest");
    assert_eq!(hurts(&sim, 0)[0], (42, HeroId::Donut));
    assert!(hurts(&sim, 0).iter().take(1).all(|h| h.1 == HeroId::Donut));
    assert_eq!(sim.scene.world.hero.health, 4, "Carl untouched");
    // Donut walks into the lane of a ram aimed at Carl: she is hit, no snap.
    let sim = replay("eng_2p_partner_in_lane");
    assert_eq!(hurts(&sim, 0)[0], (38, HeroId::Donut));
    assert_eq!(sim.scene.world.hero.health, 4);
    let snapped = trace("eng_2p_partner_in_lane", 60, |s| {
        s.combat.enemies[0].engineer_state().snapped
    });
    assert!(snapped.iter().all(|s| !s));
}

/// What the assistant's Donut did to the Engineer in a run: `(swat cancels,
/// ram hits)`. A swat cancel is a Swat hit on him that interrupted his attack on
/// the same tick; a ram hit is a `HeroHurt` of Donut with him as the source.
fn ai_counts(sim: &PartyFixtureSim, engineer: u32) -> (usize, usize) {
    let swat_cancels = hits(sim)
        .into_iter()
        .filter(|&(tick, move_id, target)| {
            move_id == MoveId::Swat
                && target == engineer
                && events(sim).iter().any(|e| {
                    matches!(*e, CombatEvent::GoblinInterrupted { tick: t, goblin: g }
                        if t == tick && g == engineer)
                })
        })
        .count();
    let ram_hits = hurts(sim, engineer)
        .into_iter()
        .filter(|h| h.1 == HeroId::Donut)
        .count();
    (swat_cancels, ram_hits)
}

#[test]
fn the_solo_assist_and_live_rooms_make_progress_and_log_the_ai_counts() {
    // (fixture, AI swat cancels, AI Donut ram hits): the counts the fixture rows
    // promise to log, pinned. The post-#165 director cancels at most one point and
    // is rammed at most once per clear (the design's follow-up threshold is more
    // than one). Every run must also finish the room, not just produce events.
    let rows = [
        // PR #170 A15 flank: her position moved, so the Engineer does not ram
        // her in the first two runs (was 1,1 / 0,1; cp2 was 0,1). DM-A: her
        // 207 px/s run puts her elsewhere again, and in cp2 she now cancels one
        // point and is not rammed (was 0,1).
        ("eng_solo_assist", 0, 0),
        ("cf02_arena_eng", 0, 0),
        ("cf02_arena_eng_cp2", 1, 0),
    ];
    for (name, swat_cancels, ram_hits) in rows {
        let sim = replay(name);
        let index = sim
            .combat
            .enemies
            .iter()
            .position(|g| g.is_engineer())
            .unwrap() as u32;
        assert_eq!(ai_counts(&sim, index), (swat_cancels, ram_hits), "{name}");
        // The Engineer starts at least one ram (a tell that ends in a hit) ...
        assert!(!tells(&sim, index).is_empty(), "{name}: he never points");
        if name == "cf02_arena_eng_cp2" {
            // DM-A: in cp2 her AI swat cancels the one point before the ram
            // starts, so the ram that opens the room is the cancelled one: a
            // tell, then her swat cancel on him, and no ram strike on anybody.
            let tell = tells(&sim, index)[0];
            let cancel = hits(&sim)
                .into_iter()
                .find(|&(tick, move_id, target)| {
                    move_id == MoveId::Swat
                        && target == index
                        && events(&sim).iter().any(|e| {
                            matches!(*e, CombatEvent::GoblinInterrupted { tick: t, goblin: g }
                                if t == tick && g == index)
                        })
                })
                .map(|h| h.0)
                .expect("her swat cancels his point");
            assert!(cancel >= tell, "{name}: the cancel follows the tell");
            assert!(hurts(&sim, index).is_empty(), "{name}: no ram hit");
            assert!(bonks(&sim).is_empty(), "{name}: no ram bonk");
        } else {
            assert!(
                !hurts(&sim, index).is_empty() || !bonks(&sim).is_empty(),
                "{name}: he never rams"
            );
        }
        // ... and the room is cleared: he and every goblin are dead and Carl lives.
        assert!(
            sim.combat.enemies.iter().all(|g| !g.alive()),
            "{name}: the room is not cleared: {:?}",
            sim.combat
                .enemies
                .iter()
                .map(|g| (g.x as i32, g.health))
                .collect::<Vec<_>>()
        );
        assert!(!sim.combat.hero_defeated, "{name}: Carl is down");
        assert!(
            sim.scene.world.hero.x > 1000.0,
            "{name}: Carl is past the yard"
        );
    }
}

#[test]
fn a_recorded_demo_session_names_its_room_by_fixture_name_then_beat() {
    use crate::cf02::arena::{LIVE, LIVE_CP2, LIVE_ENG, LIVE_ENG_CP2, fixture_for_recorded};
    assert!(std::ptr::eq(
        fixture_for_recorded("cf02_arena_eng", 0),
        &LIVE_ENG
    ));
    assert!(std::ptr::eq(
        fixture_for_recorded("cf02_arena_eng_cp2", 1),
        &LIVE_ENG_CP2
    ));
    // Every earlier session names the plain room and picks by beat.
    assert!(std::ptr::eq(fixture_for_recorded("cf02_arena", 0), &LIVE));
    assert!(std::ptr::eq(
        fixture_for_recorded("cf02_arena_cp2", 1),
        &LIVE_CP2
    ));
}

#[test]
fn a_restart_reshuffles_the_engineer_room_temperaments() {
    // The Q58 bag: the four goblins draw slots 0..3 and the Engineer has no nerve,
    // so the missing temperament varies per attempt.
    let mut sim = sim_of("cf02_arena_eng");
    let draw = |s: &PartyFixtureSim| {
        (0..4)
            .map(|i| format!("{:?}", s.combat.goblin_morale(i).unwrap().temperament()))
            .collect::<Vec<_>>()
    };
    let first = draw(&sim);
    let mut varied = false;
    for _ in 0..6 {
        sim.combat.restart(&mut sim.scene);
        varied |= draw(&sim) != first;
    }
    assert!(varied, "a restart reshuffles the temperaments");
    let e = &sim.combat.enemies[4];
    assert!(e.is_engineer());
    assert_eq!(e.health, 3);
}

// ----------------------------------------------------------------- gates --

#[test]
fn the_engineer_exists_only_with_cf02_rules_the_fixture_flag_and_the_data_switch() {
    use crate::cf02::arena::{LIVE, LIVE_CP2, LIVE_ENG, LIVE_ENG_CP2, fixture_for_run};
    let on = crate::engineer::active();
    // Plain `--cf02` is the regression room.
    assert!(std::ptr::eq(fixture_for_run(0, false, on), &LIVE));
    assert!(std::ptr::eq(fixture_for_run(1, false, on), &LIVE_CP2));
    assert!(std::ptr::eq(fixture_for_run(0, true, on), &LIVE_ENG));
    assert!(std::ptr::eq(fixture_for_run(1, true, on), &LIVE_ENG_CP2));
    // The kill switch: the flag runs plain `cf02_arena`.
    let mut off = crate::engineer::EngineerTuning::pinned().unwrap();
    off.engq25_enabled = false;
    assert!(std::ptr::eq(fixture_for_run(0, true, &off), &LIVE));
    assert!(std::ptr::eq(fixture_for_run(1, true, &off), &LIVE_CP2));
    // The Engineer fixture builds him under cf02_rules ...
    let scene = crate::corridor::SceneSim::new_at(kit(), 360, LIVE_ENG.carl_x).unwrap();
    let combat = crate::combat_scene::CombatState::new(&LIVE_ENG, kit(), &scene)
        .unwrap()
        .with_cf02();
    assert_eq!(combat.enemies.len(), 5);
    assert!(combat.enemies[4].is_engineer());
    // ... and not with the switch off, nor without cf02_rules, nor in the plain room.
    let off: &'static crate::engineer::EngineerTuning = Box::leak(Box::new(off));
    let combat = crate::combat_scene::CombatState::new(&LIVE_ENG, kit(), &scene)
        .unwrap()
        .with_cf02()
        .with_engineer_tuning_for_test(off);
    assert_eq!(combat.enemies.len(), 4);
    assert!(combat.enemies.iter().all(|g| !g.is_engineer()));
    let plain = crate::combat_scene::CombatState::new(&LIVE, kit(), &scene)
        .unwrap()
        .with_cf02();
    assert_eq!(plain.enemies.len(), 5);
    assert!(plain.enemies.iter().all(|g| !g.is_engineer()));
}

#[test]
fn the_engineer_room_keeps_six_enemies_and_the_yard_goblin_is_replaced_one_for_one() {
    use crate::cf02::arena::{ENGINEERS, GOBLINS, GOBLINS_ENG, LLAMAS};
    assert_eq!(GOBLINS_ENG.len() + ENGINEERS.len() + LLAMAS.len(), 6);
    let xs: Vec<i32> = GOBLINS_ENG.iter().chain(&ENGINEERS).map(|s| s.x).collect();
    let plain: Vec<i32> = GOBLINS.iter().map(|s| s.x).collect();
    assert_eq!(xs, plain, "the Engineer stands where goblin 4 stood");
}

#[test]
#[ignore = "prints the simulation cost of the Engineer room against the plain room"]
fn engineer_room_sim_cost() {
    let cost = |name: &str| {
        let mut best = f64::MAX;
        for _ in 0..5 {
            let mut sim = sim_of(name);
            let start = std::time::Instant::now();
            sim.run_scripted(1800);
            best = best.min(start.elapsed().as_secs_f64() * 1e6 / 1800.0);
        }
        best
    };
    println!(
        "SIM_COST_US_PER_TICK cf02_arena_direct {:.2}",
        cost("cf02_arena_direct")
    );
    println!(
        "SIM_COST_US_PER_TICK cf02_arena_eng {:.2}",
        cost("cf02_arena_eng")
    );
}
