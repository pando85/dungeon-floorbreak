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
    #[derive(Debug, Clone, Default)]
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
        /// FA-Q13: only the room 6 pilots measure the stall; the rest print NOT RUN.
        max_stall: Option<u32>,
        /// FA-Q17: the run's outcome (clear tick, hurts, kill order).
        outcome: Option<mastery::Outcome>,
    }

    fn count(events: &[CombatEvent]) -> Row {
        let mut row = Row::default();
        let mut order = Vec::new();
        for e in events {
            match e {
                CombatEvent::EnemyDefeated { target, .. } => order.push(*target),
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
        row.outcome = Some((None, row.hurts as usize, order));
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
        // FA-Q13: the stall, measured by the room 6 pilots only.
        let stall = if rows.iter().all(|r| r.max_stall.is_some()) {
            let v: Vec<u64> = rows
                .iter()
                .filter_map(|r| r.max_stall)
                .map(u64::from)
                .collect();
            format!(
                "{} / {}",
                median(v.clone()).unwrap_or(0),
                v.iter().copied().max().unwrap_or(0)
            )
        } else {
            "NOT RUN".to_owned()
        };
        // FA-Q17: the distinct (clear tick, hurts, kill order) tuples over the seeds.
        let mut seen: Vec<(Option<u64>, u32, Vec<u32>)> = Vec::new();
        for r in rows {
            let order = r.outcome.as_ref().map(|o| o.2.clone()).unwrap_or_default();
            let tuple = (r.cleared, r.hurts, order);
            if !seen.contains(&tuple) {
                seen.push(tuple);
            }
        }
        let distinct = seen.len();
        let _ = writeln!(
            out,
            "| {label} | {clears} of {n} | {} | {} / {worst_time} | {} / {} | {} / {} | {} / {} | {} / {} | {} / {} | NOT RUN | {} / {} | {} / {} | {} / {} | {stall} | {distinct} of {n} |",
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

    const HEAD: &str = "| Pilot | Clears | Roster or boss cleared tick (median) | Exit tick (median / worst) | Deaths (med / worst) | Restart events | Hurts | Throws | Chains | Pierces | Blinks | Evades | Max stuck ticks | Max stall ticks (med / worst) | Distinct outcomes |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n";

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

    fn boiler_live_rows(who: boiler::Who, plan: boiler::Plan) -> Vec<Row> {
        SEEDS
            .map(|seed| {
                let r = boiler::run_live(&root(), who, plan, seed).unwrap();
                Row {
                    exit: r.exit_tick,
                    max_stuck: r.max_stuck.into_iter().max().unwrap_or(0),
                    ..count(&r.events)
                }
            })
            .collect()
    }

    /// Room 1 (FA-Q24, PR-E): the Hoarder's room pilot, `hoarder:carl_solo:SEED`, seeds 0..8: solo
    /// Carl with the room's `cf02_assist` partner, from the spawn to the exit. The room was
    /// excluded from earlier scorecards (PTG-AD1, no defeating pilot); it is listed again.
    fn hoarder_table(out: &mut String) {
        use crate::hoarder_fixtures::{ROOM_SEEDS, room_run};
        let _ = writeln!(
            out,
            "\n## Room 1, the Hoarder (`hoarder_fixtures::room_run`, the aligned room pilot `hoarder:carl_solo:SEED`, seeds 0..8; hashes pinned in `hoarder_fixtures`; listed again by FA-Q24, it reverses PTG-AD1)\n"
        );
        out.push_str(HEAD);
        let reports: Vec<_> = ROOM_SEEDS
            .map(|seed| room_run(&root(), seed).unwrap())
            .collect();
        let rows: Vec<Row> = reports
            .iter()
            .map(|r| Row {
                cleared: Some(r.defeat_tick),
                exit: Some(r.ticks),
                defeats: r.deaths,
                hurts: r.hurts,
                max_stuck: u32::try_from(r.stuck).unwrap_or(u32::MAX),
                ..Row::default()
            })
            .collect();
        summary(
            out,
            "hoarder:carl_solo (the story loadout, cf02_assist partner)",
            &rows,
        );
        let grants = reports.iter().filter(|r| r.grants == 1).count();
        let _ = writeln!(
            out,
            "\nThe boss is the Hoarder: the \"cleared tick\" is her defeat. Exactly one `CapabilityGranted` (`special_thrown_charge`, hero 0, on her defeat tick) on {grants} of {} seeds. Seeds 1, 6 and 8 play the identical fight. Hurts count both heroes; the pilot jumps the swipe and the shove and punishes the free windows. Not run: Donut who fights her alone as a gate mode (`hrd_direct_clear_donut` exists, library only).",
            reports.len()
        );
    }

    /// Room 4 (PT-GATE-C2, #240): the HAZ-06 direct route (with the Rot kind present and
    /// removed) and the HAZ-09 modes, seeds 0..8, with the AI hero's hurts by source.
    fn hazards_tables(out: &mut String) {
        use crate::hazards_fixtures::pilots::{self as haz, HurtSource};
        let _ = writeln!(
            out,
            "\n## Room 4, the hazards room (`hazards_fixtures::pilots`, seeds 0..8; hashes pinned by #240)\n"
        );
        out.push_str(HEAD);
        let mut detail = Vec::new();
        for (who, rot, label) in [
            (
                haz::Who::CarlSolo,
                true,
                "haz_room_direct_route / solo_carl (HAZ-06, HAZ-09)",
            ),
            (
                haz::Who::CarlSolo,
                false,
                "haz_room_direct_route, Rot kind removed (HAZ-06)",
            ),
            (haz::Who::DonutBySwap, true, "haz_room_solo_donut (HAZ-09)"),
            (
                haz::Who::Companion,
                true,
                "haz_room_companion (HAZ-09, cf02_free)",
            ),
        ] {
            let reports: Vec<haz::Report> = SEEDS
                .map(|seed| haz::run(&root(), who, seed, rot).unwrap())
                .collect();
            let rows: Vec<Row> = reports
                .iter()
                .map(|r| Row {
                    exit: r.exit_tick,
                    cleared: r.cleared_tick,
                    max_stuck: r.max_stuck.into_iter().max().unwrap_or(0),
                    ..count(&r.events)
                })
                .collect();
            summary(out, label, &rows);
            let by = |source| reports.iter().map(|r| r.ai_hurts_by(source)).sum::<usize>();
            detail.push(format!(
                "| {label} | {} | {} | {} | {} | {} | {} |",
                by(HurtSource::Acid),
                by(HurtSource::SlimeLunge),
                by(HurtSource::SlimeBody),
                by(HurtSource::Rot),
                by(HurtSource::Goblin),
                reports.iter().map(|r| r.ai_provokes()).sum::<usize>(),
            ));
        }
        let _ = writeln!(
            out,
            "\nThe AI hero's hurts by source and its Rot provokes, summed over the nine seeds (HAZ-Q45, report-only):\n\n| Run | Acid | Slime lunge | Slime body | Rot | Goblin | Rot provokes |\n| --- | --- | --- | --- | --- | --- | --- |"
        );
        for line in detail {
            let _ = writeln!(out, "{line}");
        }
        let _ = writeln!(
            out,
            "\nSolo Donut who fights the whole room (`haz_room_donut_fights`) does not clear it: ignored fixture, NOT RUN here (known gap)."
        );
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
                max_stall: Some(r.max_stall_ticks),
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
            "# F01 playtest scripted scorecard (PT-GATE-B, live rows PT-GATE-D)\n\nGenerated by `scripts/make_f01_gate_b_evidence.sh` from the committed scripted pilots and fixtures (`f01_gate_scorecard`). Scripted pilots, not a human: game feel is pending and every metric a pilot cannot produce is NOT RUN. Deaths are hero defeats; restart events are Select restarts (the pilots never press Select); the time is a tick (60 a second); throws, chains, blinks and evades are 0 for a base-kit pilot by construction.\n\n",
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

        // PT-GATE-D: the live room's pilots (PT-GATE-C2, #240; pinned there). The partner of
        // the solo runs is the `cf02_assist` partner, the companion run's is `cf02_free`;
        // its hurts include the partner's (the live partner hits the valve in every run).
        let _ = writeln!(
            out,
            "\n## Room 2, the LIVE room (`Roster::Live`: goblins and the Engineer; `boiler_fixtures::scene_pilots::run_live`, seeds 0..8; hashes pinned by #240)\n"
        );
        out.push_str(HEAD);
        for (plan, name) in [
            (boiler::Plan::Direct, "direct route (BOIL-01)"),
            (boiler::Plan::Both, "valve then brace (BOIL-06/07)"),
        ] {
            for (who, who_name) in [
                (boiler::Who::CarlSolo, "CarlSolo"),
                (boiler::Who::DonutBySwap, "DonutBySwap"),
                (boiler::Who::Companion, "Companion (cf02_free)"),
            ] {
                summary(
                    &mut out,
                    &format!("live roster: {who_name}, {name}"),
                    &boiler_live_rows(who, plan),
                );
            }
        }
        let _ = writeln!(
            out,
            "\nThe live `Direct` run is \"the pilot touches no prop\": the partner's own hits on the valve are reported by the fixture (a follow-up, not a gate failure). Hurts count both heroes."
        );
        hazards_tables(&mut out);
        hoarder_table(&mut out);

        let _ = writeln!(
            out,
            "\n## Room 5, the Juicer (`f01_late_fixtures` pilots, seeds 0..8; the seed shifts the pilot's jab phase only)\n"
        );
        out.push_str(HEAD);
        let mut juicer_coverage = Vec::new();
        for who in JuicerWho::ALL {
            let rows = juicer_rows(who);
            summary(&mut out, who.name(), &rows);
            juicer_coverage.push(format!(
                "{}: {} distinct (defeat, exit) tick pairs over {} seeds",
                who.name(),
                distinct(&rows.iter().map(|r| (r.cleared, r.exit)).collect::<Vec<_>>()),
                JUICER_SEEDS.end
            ));
        }
        // PT-FU-B7 (#234): the room 5 seed coverage, listed in the owner packet.
        let _ = writeln!(
            out,
            "\nSeed coverage (a known gap): the room reads no seed (no roster, and the boss draws nothing), so the seed shifts the base-kit pilot's jab phase only. {}.",
            juicer_coverage.join("; ")
        );
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
                "{}: {} distinct final hashes, {} distinct outcomes (clear tick, hurts, kill order; FA-Q17), {} distinct (clear, exit) tick pairs over {} seeds",
                who.name(),
                distinct(&reports.iter().map(|r| r.hash).collect::<Vec<_>>()),
                mastery::distinct_outcomes(&reports),
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
            "\nD3 bot median clear: {} ticks (main 517; target at least 647, every seed at most 4800; margin over 647: {} ticks, FA-Q18 asks for at least 30; the human band 900..3000 is the owner's, OQ4). Clear ticks: min {}, max {}.",
            fmt_opt(median(clears.clone())),
            median(clears.clone()).map_or("-".to_owned(), |m| (m as i64 - 647).to_string()),
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
