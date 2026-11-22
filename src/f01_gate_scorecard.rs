//! PT-GATE-B: the scripted scorecard of the playtest gate (plan section 10, item 8).
//!
//! Test-only and report-only: it runs the committed scripted pilots (room 2's scene
//! pilots, room 5's `JuicerWho` pilots, room 6's `Who` pilots) at seeds 0..8 and
//! counts what they did from the authoritative combat log. It reads simulation
//! state and changes none, pins nothing and moves no golden. The generator is the
//! ignored test [`prints_the_gate_scorecard`]; `scripts/make_f01_gate_b_scorecard.sh`
//! runs it and writes `docs/evidence/f01-playtest/scorecard.md`.
//!
//! A metric a pilot cannot produce is printed `NOT RUN`, never estimated. The
//! pierce count is `NOT RUN` everywhere: no scripted pilot holds and swings the
//! gauntlet, and the combat log has no pierce event.

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;
    use std::path::PathBuf;

    use crate::boiler_fixtures::scene_pilots as boiler;
    use crate::combat::CombatEvent;
    use crate::f01_late_fixtures::{JUICER_SEEDS, JuicerWho, juicer_run};
    use crate::f01_mastery_fixtures::{self as mastery, SEEDS, Who};
    use crate::party::HeroId;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// What one scripted run did, counted from the combat log.
    #[derive(Debug, Clone, Copy, Default)]
    struct Row {
        cleared: Option<u64>,
        exit: Option<u64>,
        defeats: u32,
        restarts: u32,
        hurts: u32,
        throws: u32,
        chains: u32,
        blinks: u32,
        evades: u32,
        max_stuck: u32,
    }

    fn count(events: &[CombatEvent]) -> Row {
        let mut row = Row::default();
        for e in events {
            match e {
                CombatEvent::HeroDefeated { .. } => row.defeats += 1,
                CombatEvent::Restart { .. } => row.restarts += 1,
                CombatEvent::HeroHurt { .. } => row.hurts += 1,
                CombatEvent::ChargeThrown { .. } => row.throws += 1,
                CombatEvent::ChargeChained { .. } => row.chains += 1,
                CombatEvent::BlinkResolved { .. } => row.blinks += 1,
                CombatEvent::HeroEvaded { .. } => row.evades += 1,
                _ => {}
            }
        }
        row
    }

    fn median(mut v: Vec<u64>) -> Option<u64> {
        v.sort_unstable();
        v.get(v.len() / 2).copied()
    }

    fn fmt_opt(v: Option<u64>) -> String {
        v.map_or("-".into(), |v| v.to_string())
    }

    /// One table row: the median and the worst over the seeds of every column.
    fn summary(out: &mut String, label: &str, rows: &[Row]) {
        let n = rows.len();
        let clears = rows.iter().filter(|r| r.exit.is_some()).count();
        let times: Vec<u64> = rows.iter().filter_map(|r| r.exit).collect();
        let cleared = median(rows.iter().filter_map(|r| r.cleared).collect());
        let worst_time = if clears == n {
            fmt_opt(times.iter().copied().max())
        } else {
            "no clear".into()
        };
        let col = |f: fn(&Row) -> u32| -> (u64, u64) {
            let v: Vec<u64> = rows.iter().map(|r| u64::from(f(r))).collect();
            (
                median(v.clone()).unwrap_or(0),
                v.iter().copied().max().unwrap_or(0),
            )
        };
        let d = col(|r| r.defeats);
        let rs = col(|r| r.restarts);
        let h = col(|r| r.hurts);
        let t = col(|r| r.throws);
        let c = col(|r| r.chains);
        let b = col(|r| r.blinks);
        let e = col(|r| r.evades);
        let s = col(|r| r.max_stuck);
        let _ = writeln!(
            out,
            "| {label} | {clears} of {n} | {} | {} / {worst_time} | {} / {} | {} / {} | {} / {} | {} / {} | {} / {} | NOT RUN | {} / {} | {} / {} | {} / {} |",
            fmt_opt(cleared),
            fmt_opt(median(times)),
            d.0,
            d.1,
            rs.0,
            rs.1,
            h.0,
            h.1,
            t.0,
            t.1,
            c.0,
            c.1,
            b.0,
            b.1,
            e.0,
            e.1,
            s.0,
            s.1,
        );
    }

    const HEAD: &str = "| Pilot | Clears | Roster or boss cleared tick (median) | Exit tick (median / worst) | Deaths (med / worst) | Restart events | Hurts | Throws | Chains | Pierces | Blinks | Evades | Max stuck ticks |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n";

    fn boiler_rows(who: boiler::Who, plan: boiler::Plan) -> Vec<Row> {
        SEEDS
            .map(|seed| {
                let r = boiler::run(&root(), who, plan, seed).unwrap();
                Row {
                    exit: r.exit_tick,
                    max_stuck: r.max_stuck.into_iter().max().unwrap_or(0),
                    ..count(&r.events)
                }
            })
            .collect()
    }

    fn juicer_rows(who: JuicerWho) -> Vec<Row> {
        JUICER_SEEDS
            .map(|seed| {
                let (r, sim) = juicer_run(&root(), who, seed).unwrap();
                Row {
                    exit: r.exit_tick,
                    cleared: r.defeated_tick,
                    max_stuck: r.max_stuck.into_iter().max().unwrap_or(0),
                    ..count(&sim.combat.events)
                }
            })
            .collect()
    }

    fn mastery_rows(who: Who) -> (Vec<Row>, Vec<mastery::Report>) {
        let reports: Vec<mastery::Report> = SEEDS
            .map(|seed| mastery::run(&root(), who, seed).unwrap())
            .collect();
        let rows = reports
            .iter()
            .map(|r| Row {
                exit: r.exit_tick,
                cleared: r.cleared_tick,
                max_stuck: r.max_stuck.into_iter().max().unwrap_or(0),
                ..count(&r.events)
            })
            .collect();
        (rows, reports)
    }

    fn distinct<T: Ord + Clone>(v: &[T]) -> usize {
        let mut s = v.to_vec();
        s.sort();
        s.dedup();
        s.len()
    }

    fn pass(ok: bool) -> &'static str {
        if ok { "pass" } else { "MISS" }
    }

    /// The CA2 metrics M1 to M16 of the closing arena, per seed: measured, the spec
    /// target and the bound the committed test holds, and pass or miss against the
    /// spec target (`docs/03-tech/companion-ai-02-donut-autonomy.md` section 7.1).
    fn metrics_table(out: &mut String) {
        let _ = writeln!(
            out,
            "| Seed | M1 spread / median px | M2 shadow | M3 distinct / same-target ticks | M4 steals+start | M5 self-caused | M6 outside / ahead | M10 retreats, distances px, hits | M12 teleports | M13 hits (share) vs assist | M14 twice-identical | M15 retreat share | M16 skipped / targeted her / drift-rejected |"
        );
        let _ = writeln!(
            out,
            "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
        );
        let mut miss = [0u32; 16];
        for seed in SEEDS {
            let (m, assist) = mastery::metrics(&root(), seed).unwrap();
            let (again, _) = mastery::metrics(&root(), seed).unwrap();
            let m1 = m.spread_percent() >= 40 && m.median_distance_px >= 96.0;
            let m2 = m.shadow_percent() <= 20;
            let m3 =
                m.distinct_percent() >= 70 && m.same_target_ticks * 100 <= 15 * m.counted_ticks;
            let m4 = m.steals + m.start_steals == 0;
            let m5 = m.episodes_self_caused == 0;
            let m6 = m.outside_ticks == 0 && m.ahead_exits == 0;
            let dists: Vec<String> = m
                .retreat_distances
                .iter()
                .map(|d| d.map_or("none".into(), |d| format!("{:.0}", d)))
                .collect();
            let m10 =
                m.hits_in_retreat <= 1 && m.retreat_distances.iter().flatten().all(|d| *d >= 96.0);
            let m12 = m.regroup_teleports == 0;
            let m13 = m.her_hits >= assist && m.hit_share_percent() <= 50;
            let m14 = m == again;
            let m15 = m.retreat_share_percent() <= 25;
            for (i, ok) in [
                (1, m1),
                (2, m2),
                (3, m3),
                (4, m4),
                (5, m5),
                (6, m6),
                (10, m10),
                (12, m12),
                (13, m13),
                (14, m14),
                (15, m15),
            ] {
                if !ok {
                    miss[i - 1] += 1;
                }
            }
            let _ = writeln!(
                out,
                "| {seed} | {}% / {:.0} ({}) | {}% ({}) | {}% / {} ({}) | {}+{} ({}) | {} ({}) | {} / {} ({}) | {}, [{}], {} ({}) | {} ({}) | {} ({}%) vs {assist} ({}) | {} | {}% ({}) | {} / {} / {} |",
                m.spread_percent(),
                m.median_distance_px,
                pass(m1),
                m.shadow_percent(),
                pass(m2),
                m.distinct_percent(),
                m.same_target_ticks,
                pass(m3),
                m.steals,
                m.start_steals,
                pass(m4),
                m.episodes_self_caused,
                pass(m5),
                m.outside_ticks,
                m.ahead_exits,
                pass(m6),
                m.retreats,
                dists.join(", "),
                m.hits_in_retreat,
                pass(m10),
                m.regroup_teleports,
                pass(m12),
                m.her_hits,
                m.hit_share_percent(),
                pass(m13),
                pass(m14),
                m.retreat_share_percent(),
                pass(m15),
                m.interrupt_drift_skip_ticks,
                m.goblins_targeted_her,
                m.hunt_drift_rejected,
            );
        }
        let _ = writeln!(
            out,
            "\nSeeds that miss the spec target, per metric (of {}): {}.",
            SEEDS.end,
            [1usize, 2, 3, 4, 5, 6, 10, 12, 13, 14, 15]
                .iter()
                .map(|i| format!("M{i} {}", miss[i - 1]))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    /// The scorecard, to stdout (the script captures it).
    #[test]
    #[ignore = "prints docs/evidence/f01-playtest/scorecard.md; run through scripts/make_f01_gate_b_scorecard.sh"]
    fn prints_the_gate_scorecard() {
        let mut out = String::from(
            "# F01 playtest scripted scorecard (PT-GATE-B)\n\nGenerated by `scripts/make_f01_gate_b_evidence.sh` from the committed scripted pilots and fixtures (`f01_gate_scorecard`). Scripted pilots, not a human: game feel is pending and every metric a pilot cannot produce is NOT RUN. Deaths are hero defeats; restart events are Select restarts (the pilots never press Select); the time is a tick (60 a second); throws, chains, blinks and evades are 0 for a base-kit pilot by construction.\n\n",
        );
        let _ = writeln!(
            out,
            "## Room 2, the Goblin Boiler: the EMPTY GREYBOX, not the live room (scene pilots, `boiler_fixtures::scene_pilots`, CF02 seeds 0..8; `f01_boiler::launch` is `Roster::Inert`: no goblins and no Engineer, so 0 hurts and 0 deaths say nothing about the live room)\n"
        );
        out.push_str(HEAD);
        for (plan, name) in [
            (boiler::Plan::Direct, "direct route (BOIL-01)"),
            (boiler::Plan::Both, "valve then brace (BOIL-06/07)"),
        ] {
            for (who, who_name) in [
                (boiler::Who::CarlSolo, "CarlSolo"),
                (boiler::Who::DonutBySwap, "DonutBySwap"),
            ] {
                summary(
                    &mut out,
                    &format!("greybox, no roster: {who_name}, {name}"),
                    &boiler_rows(who, plan),
                );
            }
        }
        let _ = writeln!(
            out,
            "| Companion (cf02_free), and every pilot on the live `Roster::Live` room at seeds 0..8 | NOT RUN (no such pilot exists) | | | | | | | | | | | |"
        );

        let _ = writeln!(
            out,
            "\n## Room 5, the Juicer (`f01_late_fixtures` pilots, seeds 0..8; the seed shifts the pilot's jab phase only)\n"
        );
        out.push_str(HEAD);
        for who in JuicerWho::ALL {
            let rows = juicer_rows(who);
            summary(&mut out, who.name(), &rows);
        }
        let _ = writeln!(
            out,
            "\nThe Juicer's `Exit tick` is the tick the scene's exit was reached; the boss falls earlier (the cleared column)."
        );

        let _ = writeln!(
            out,
            "\n## Room 6, the mastery arena (`f01_mastery_fixtures` pilots, seeds 0..8)\n"
        );
        out.push_str(HEAD);
        let mut carl_reports = Vec::new();
        let mut distinct_lines = Vec::new();
        for who in Who::ALL.into_iter().chain(Who::KITS) {
            let (rows, reports) = mastery_rows(who);
            summary(&mut out, who.name(), &rows);
            distinct_lines.push(format!(
                "{}: {} distinct final hashes, {} distinct (clear, exit) tick pairs over {} seeds",
                who.name(),
                distinct(&reports.iter().map(|r| r.hash).collect::<Vec<_>>()),
                distinct(
                    &reports
                        .iter()
                        .map(|r| (r.cleared_tick, r.exit_tick))
                        .collect::<Vec<_>>()
                ),
                SEEDS.end
            ));
            if who == Who::CarlSolo {
                carl_reports = reports;
            }
        }
        let _ = writeln!(
            out,
            "\n`carl_kit` presses the held dynamite and `donut_kit` the blink (the D7 report-only kit pilots); every other pilot is base-kit, so its throws and blinks are 0 by construction, not a measurement of play.\n"
        );

        let _ = writeln!(
            out,
            "### PT-TUNE D1 to D6, `carl_solo` per seed (`docs/03-tech/pt-tune-design.md` section 3)\n"
        );
        let _ = writeln!(
            out,
            "| Seed | D1 overlap W1 / W2 | D2 peak kinds W1 (kinds seen) / W2 | D3 clear tick | D4 hurts (W1/W2), deaths | D5 peak engaged | stuck |"
        );
        let _ = writeln!(out, "| --- | --- | --- | --- | --- | --- | --- |");
        for r in &carl_reports {
            let d = &r.difficulty;
            let _ = writeln!(
                out,
                "| {} | {}% / {}% | {} ({}) / {} | {} | {} ({}/{}), {} | {} | {:?} |",
                r.seed,
                d.waves[0].overlap_percent(),
                d.waves[1].overlap_percent(),
                d.waves[0].peak_kinds,
                d.waves[0].kinds_seen(),
                d.waves[1].peak_kinds,
                fmt_opt(r.cleared_tick),
                r.hero_hurts,
                d.hurts[0],
                d.hurts[1],
                r.deaths,
                d.peak_engaged,
                r.max_stuck,
            );
        }
        let clears: Vec<u64> = carl_reports.iter().filter_map(|r| r.cleared_tick).collect();
        let _ = writeln!(
            out,
            "\nD3 bot median clear: {} ticks (main 517; target at least 647, every seed at most 4800; the human band 900..3000 is the owner's, OQ4). Clear ticks: min {}, max {}.",
            fmt_opt(median(clears.clone())),
            clears.iter().min().unwrap_or(&0),
            clears.iter().max().unwrap_or(&0)
        );
        let _ = writeln!(
            out,
            "\nSeed coverage (a known gap): {}.",
            distinct_lines.join("; ")
        );

        let _ = writeln!(
            out,
            "\n### CA2 metrics M1 to M16, the autonomous partner (`cf02_free`) in the arena, Carl on the room pilot, {} ticks, seeds 0..8\n",
            mastery::METRICS_TICKS
        );
        metrics_table(&mut out);

        // The partner's hurts in the Companion pilot run (M9 is not separable by cause).
        let mut hurts = Vec::new();
        for seed in SEEDS {
            let r = mastery::run(&root(), Who::Companion, seed).unwrap();
            hurts.push(
                r.events
                    .iter()
                    .filter(|e| {
                        matches!(
                            e,
                            CombatEvent::HeroHurt {
                                hero: HeroId::Donut,
                                ..
                            }
                        )
                    })
                    .count(),
            );
        }
        let _ = writeln!(
            out,
            "\nM9 (hazard hurts to her): `HeroHurt` carries a source id, not a hazard class, so the per-cause count is NOT RUN. All hurts to Donut in the `companion_cf02_free` pilot run to the exit, per seed 0..8: {hurts:?}."
        );

        println!("\nSCORECARD_BEGIN\n{out}\nSCORECARD_END");
    }

    /// A cheap guard on the generator's counting: Carl's room-6 clear at seed 0
    /// is the pinned pilot's, and every counted death equals the pilot's own.
    #[test]
    fn the_counting_agrees_with_the_pilot_reports() {
        let r = mastery::run(&root(), Who::CarlSolo, 0).unwrap();
        let row = count(&r.events);
        assert_eq!(row.restarts, r.deaths);
        assert_eq!(row.hurts as usize, r.hero_hurts);
        assert!(r.cleared_tick.is_some());
    }
}

/// Why the room pilot (`--room-pilot mastery`) is defeated once and the PT-TUNE fixture pilot
/// (`Who::CarlSolo`) never is: the same Carl pilot plays two different fights. A scripted window
/// run drives the other hero with the follower bot (`companion_inactive`); the fixture run's
/// partner is the active `cf02_assist` companion, which attacks (it takes the slime on tick 101
/// where the follower bot's fight takes it on tick 180). Report-only: no pilot or rule changes.
#[cfg(test)]
mod room_pilot_discrepancy {
    use crate::boss_scene::Pilot;
    use crate::combat::CombatEvent;
    use crate::donut_mana::DonutSetup;
    use crate::f01_late::{Loadout, mastery_start};
    use crate::f01_mastery_fixtures as mastery;
    use crate::input::RawInput;
    use crate::party::{HeroId, PROVISIONAL_TUNING, PartyTuning};
    use crate::party_scene::{PartyFixture, PartyFixtureSim};

    /// Carl on the mastery pilot for 1500 ticks at the room's own seed under `tuning`:
    /// the ticks of the enemy defeats and of the hero defeats.
    fn fight(tuning: PartyTuning) -> (Vec<u64>, Vec<u64>) {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (carl, donut) = Loadout::Story.flags();
        let (run, scene, combat) = mastery_start(&root, 360, carl, donut, None).unwrap();
        let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            base: run.fixture,
            two_player: false,
            tuning,
            script: &[],
            pilot: Pilot::ArenaDirect,
            donut: DonutSetup::unlocked(8),
        }));
        let mut sim = PartyFixtureSim::from_parts(scene, combat, fixture);
        for _ in 0..1500 {
            let input = mastery::pilot(&sim.combat, &sim.scene, HeroId::Carl);
            sim.step_slots([input, RawInput::NONE]);
        }
        let mut kills = Vec::new();
        let mut hero = Vec::new();
        for e in &sim.combat.events {
            match e {
                CombatEvent::EnemyDefeated { tick, .. } => kills.push(*tick),
                CombatEvent::HeroDefeated { tick, .. } => hero.push(*tick),
                _ => {}
            }
        }
        (kills, hero)
    }

    #[test]
    fn the_room_pilot_fight_is_the_follower_bot_fight_and_the_fixture_fight_is_the_assist_partners()
    {
        // The follower bot (the room pilot's partner): the first seven defeats are the live
        // window run's (ticks 180, 239, 311, 529, 683, 683, 786 in the committed capture), and
        // Carl is defeated once (the live run: tick 908; here 914, see the evidence README).
        let (kills, hero) = fight(PROVISIONAL_TUNING.with_cf02_swap());
        assert_eq!(kills[..7], [180, 239, 311, 529, 683, 683, 786]);
        assert_eq!(hero.len(), 1, "{hero:?}");
        // The assist partner (the fixture pilot's): the slime falls on tick 101 and Carl is
        // never defeated.
        let assist = mastery::tuning(mastery::Who::CarlSolo);
        let (kills, hero) = fight(assist);
        assert_eq!(kills[0], 101);
        assert!(hero.is_empty(), "{hero:?}");
    }
}
