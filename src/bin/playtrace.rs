//! PLAYTRACE-01 session tool: `list`, `stats`, `inspect`, `verify`, `recover`,
//! `delete` and `synth-fixture` (a synthetic fixture session for the exporter's
//! tests).
//!
//! `verify` replays a session from its reset descriptor in this (fresh)
//! process and compares hashes, events, effective inputs and observations. It
//! is a simulation replay: it does not reproduce rendering. Recordings stay
//! local: this tool never uploads or exports anything.

use std::path::PathBuf;
use std::process::ExitCode;

use dungeon_floorbreak::playtrace::{
    self, ControlEvent, SessionStats, SessionStatus, dir_size_bytes, list_sessions, read_session,
    recover_session, session_stats, synthetic,
};

const USAGE: &str =
    "usage: playtrace <list|stats [ID]|inspect ID|verify ID|recover ID|delete ID --yes> [--dir DIR] [--root REPO]
       playtrace synth-fixture --dir DIR [--fixture NAME] [--ticks N] [--root REPO]
  list              every recorded session (id, status, ticks, day, commit)
  stats [ID]        read-only aggregate counts: ticks, episodes, per-hero human and AI action histograms
  synth-fixture     write a SYNTHETIC fixture-scripted session (test data, never a human demonstration)
  inspect ID        manifest summary, episodes, control events and integrity of one session
  recover ID        seal a crashed session to its valid complete prefix
  delete ID --yes   remove one session directory (local deletion; nothing is uploaded)
  verify ID         fresh-process replay from the reset descriptor (exit 0 verified, 1 rejected)
ID is a session id or a unique prefix of one. --dir overrides the base directory
(default: $XDG_DATA_HOME/dungeon-floorbreak/playtrace). --root is the repository
checkout whose assets the replay loads (default: the current directory).";

fn base_dir(args: &[String]) -> Result<PathBuf, String> {
    if let Some(at) = args.iter().position(|a| a == "--dir") {
        return args
            .get(at + 1)
            .map(PathBuf::from)
            .ok_or_else(|| "--dir needs a directory".to_owned());
    }
    playtrace::default_record_dir(
        std::env::var("XDG_DATA_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )
    .map_err(|e| e.to_string())
}

fn resolve(base: &std::path::Path, id: &str) -> Result<PathBuf, String> {
    let matches: Vec<PathBuf> = list_sessions(base)
        .into_iter()
        .filter(|(_, m)| m.session_id.starts_with(id))
        .map(|(path, _)| path)
        .collect();
    match matches.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(format!("no session matches {id:?}")),
        _ => Err(format!(
            "{id:?} matches {} sessions: use more digits",
            matches.len()
        )),
    }
}

fn status_name(status: SessionStatus) -> String {
    format!("{status:?}")
}

fn run(args: &[String]) -> Result<(), String> {
    let command = args.first().ok_or(USAGE)?.as_str();
    let id = args.get(1).filter(|a| !a.starts_with("--"));
    match command {
        "list" => {
            let base = base_dir(args)?;
            let sessions = list_sessions(&base);
            println!(
                "PLAYTRACE_LIST sessions={} bytes={}",
                sessions.len(),
                dir_size_bytes(&base)
            );
            for (path, m) in sessions {
                let status = if m.status == SessionStatus::Recording {
                    if playtrace::session_is_live(&path) {
                        "Recording(live)".to_owned()
                    } else {
                        "Recording(unfinished: crashed; recover it)".to_owned()
                    }
                } else {
                    status_name(m.status)
                };
                println!(
                    "{} status={} ticks={} day={} commit={} complete={}",
                    m.session_id,
                    status,
                    m.tick_count,
                    m.started_on_utc_day.as_deref().unwrap_or("-"),
                    m.build_commit.get(..12).unwrap_or(&m.build_commit),
                    m.capture_complete,
                );
            }
            Ok(())
        }
        "inspect" => {
            let base = base_dir(args)?;
            let path = resolve(&base, id.ok_or("inspect needs a session id")?)?;
            let loaded = read_session(&path, true).map_err(|e| format!("INVALID: {e}"))?;
            let m = &loaded.manifest;
            println!("session {}", m.session_id);
            println!(
                "  status {} (capture_complete={})",
                status_name(m.status),
                m.capture_complete
            );
            println!(
                "  schema {} participants {:?} run_kind {:?} mode {}",
                m.schema, m.participants, m.run_kind, m.mode
            );
            println!(
                "  build {} dirty={} profile {}",
                m.build_commit, m.build_dirty, m.build_profile
            );
            println!(
                "  source {} binary {} toolchain {}",
                if m.source_fingerprint.is_empty() {
                    "-"
                } else {
                    &m.source_fingerprint
                },
                if m.binary_fingerprint.is_empty() {
                    "-"
                } else {
                    &m.binary_fingerprint
                },
                if m.toolchain.is_empty() {
                    "-"
                } else {
                    &m.toolchain
                },
            );
            println!(
                "  streams control {} diagnostics {} (dropped control {} diagnostics {})",
                m.control_integrity
                    .map_or("-".to_owned(), |i| format!("{} lines", i.lines)),
                m.diagnostics_integrity
                    .map_or("-".to_owned(), |i| format!("{} lines", i.lines)),
                m.dropped_control,
                m.dropped_diagnostics,
            );
            println!(
                "  reset {}:{} demo_beat={} cf02={}",
                m.reset.source, m.reset.name, m.reset.demo_beat, m.reset.cf02_rules
            );
            println!(
                "  ticks {} chunks {} (closed {}) gaps {} dropped_ticks {} write_errors {}",
                loaded.ticks.len(),
                loaded.chunks.len(),
                loaded.chunks.iter().filter(|c| c.closed).count(),
                loaded.gaps.len(),
                m.dropped_ticks,
                m.write_errors,
            );
            let episodes = loaded.ticks.last().map_or(0, |t| t.ep + 1);
            let events: usize = loaded.ticks.iter().map(|t| t.ev.len()).sum();
            println!(
                "  episodes {episodes} combat_events {events} control_events {}",
                loaded.control.len()
            );
            for record in &loaded.control {
                if let ControlEvent::EpisodeBegin {
                    episode,
                    reason,
                    stage,
                } = &record.ev
                {
                    println!(
                        "  episode {episode} begins at t={} ({reason:?}, stage {stage})",
                        record.at_t
                    );
                }
            }
            println!("  flags {:?}", m.quality_flags);
            println!(
                "  replay_verified={} training_eligible={} observation_is_partial={}",
                m.replay_verified, m.training_eligible, m.observation_is_partial
            );
            if loaded.live {
                println!("  live: a writer holds this session (it is being recorded)");
            } else if loaded.recovered {
                println!(
                    "  recovered: a crashed session (partial lines dropped {})",
                    loaded.partial_lines_dropped
                );
            }
            Ok(())
        }
        "recover" => {
            let base = base_dir(args)?;
            let path = resolve(&base, id.ok_or("recover needs a session id")?)?;
            let m = recover_session(&path).map_err(|e| e.to_string())?;
            println!(
                "PLAYTRACE_RECOVERED session={} status={} ticks={}",
                m.session_id,
                status_name(m.status),
                m.tick_count
            );
            Ok(())
        }
        "stats" => {
            let base = base_dir(args)?;
            let mut paths: Vec<PathBuf> = match id {
                Some(id) => vec![resolve(&base, id)?],
                None => list_sessions(&base).into_iter().map(|(p, _)| p).collect(),
            };
            paths.sort();
            let mut total = SessionStats::default();
            let mut sessions = 0_u64;
            let mut lines = Vec::new();
            for path in &paths {
                let name = path
                    .file_name()
                    .map_or(String::new(), |n| n.to_string_lossy().into_owned());
                match read_session(path, true) {
                    Ok(loaded) => {
                        let stats = session_stats(&loaded);
                        sessions += 1;
                        total.ticks += stats.ticks;
                        total.episodes += stats.episodes;
                        total.combat_events += stats.combat_events;
                        lines.push(format!(
                            "PLAYTRACE_STATS_SESSION session={name} status={} ticks={} episodes={} combat_events={}",
                            status_name(loaded.manifest.status),
                            stats.ticks,
                            stats.episodes,
                            stats.combat_events
                        ));
                        for (hero, name_of) in ["carl", "donut"].iter().enumerate() {
                            let h = &stats.heroes[hero];
                            for (class, hist) in [("human", &h.human), ("ai", &h.ai)] {
                                lines.push(format!(
                                    "  hero={name_of} class={class} ticks={} left={} neutral={} right={} jump={} attack={} physical={} ability={} down={} swap={}",
                                    hist.ticks,
                                    hist.left,
                                    hist.neutral,
                                    hist.right,
                                    hist.jump,
                                    hist.attack,
                                    hist.physical,
                                    hist.ability,
                                    hist.down,
                                    hist.swap
                                ));
                            }
                            lines.push(format!(
                                "  hero={name_of} undriven_ticks={} human_intent_differs_effective={}",
                                h.undriven_ticks, h.human_intent_differs_effective
                            ));
                        }
                    }
                    Err(e) => lines.push(format!(
                        "PLAYTRACE_STATS_SESSION session={name} INVALID: {e}"
                    )),
                }
            }
            println!(
                "PLAYTRACE_STATS sessions={sessions} ticks={} episodes={} combat_events={}",
                total.ticks, total.episodes, total.combat_events
            );
            for line in lines {
                println!("{line}");
            }
            Ok(())
        }
        "synth-fixture" => {
            // A synthetic, fixture-scripted session for the exporter's tests. It
            // never goes into the default recording store: --dir is required.
            let Some(at) = args.iter().position(|a| a == "--dir") else {
                return Err("synth-fixture needs an explicit --dir (it never writes to the default recording store)".to_owned());
            };
            let base = PathBuf::from(args.get(at + 1).ok_or("--dir needs a directory")?);
            let value = |flag: &str| {
                args.iter()
                    .position(|a| a == flag)
                    .and_then(|at| args.get(at + 1))
            };
            let fixture = value("--fixture").map_or(synthetic::DEFAULT_FIXTURE, String::as_str);
            let ticks: u64 = match value("--ticks") {
                Some(text) => text.parse().map_err(|_| "--ticks needs a number")?,
                None => 400,
            };
            let root = match value("--root") {
                Some(root) => PathBuf::from(root),
                None => std::env::current_dir()
                    .map_err(|e| format!("no working directory: {:?}", e.kind()))?,
            };
            let manifest = synthetic::record_synthetic(&base, &root, fixture, ticks)?;
            println!(
                "PLAYTRACE_SYNTHETIC session={} fixture={fixture} ticks={} (synthetic: not a human demonstration)",
                manifest.session_id, manifest.tick_count
            );
            Ok(())
        }
        "delete" => {
            let base = base_dir(args)?;
            let path = resolve(&base, id.ok_or("delete needs a session id")?)?;
            if playtrace::session_is_live(&path) {
                return Err(
                    "the session is still being recorded (locked): refusing to delete it"
                        .to_owned(),
                );
            }
            if !args.iter().any(|a| a == "--yes") {
                return Err("delete removes the session directory: add --yes to confirm".to_owned());
            }
            std::fs::remove_dir_all(&path).map_err(|e| format!("cannot delete: {:?}", e.kind()))?;
            println!(
                "PLAYTRACE_DELETED session={}",
                path.file_name()
                    .map_or(String::new(), |n| n.to_string_lossy().into_owned())
            );
            Ok(())
        }
        "verify" => {
            let base = base_dir(args)?;
            let path = resolve(&base, id.ok_or("verify needs a session id")?)?;
            let root = match args.iter().position(|a| a == "--root") {
                Some(at) => PathBuf::from(args.get(at + 1).ok_or("--root needs a directory")?),
                None => std::env::current_dir()
                    .map_err(|e| format!("no working directory: {:?}", e.kind()))?,
            };
            if !root
                .join("assets/levels/floor01/corridor_abc_sample.json")
                .exists()
            {
                return Err(
                    "run verify from the repository root, or pass --root REPO (the replay loads its assets)"
                        .to_owned(),
                );
            }
            let session = path
                .file_name()
                .map_or(String::new(), |n| n.to_string_lossy().into_owned());
            match playtrace::verify_dir(&path, &root) {
                Ok(report) => {
                    println!(
                        "PLAYTRACE_VERIFY session={} result=PASS scope={} ticks={} episodes={} combat_events={} control_events={} build={}",
                        report.session_id,
                        report.scope.name(),
                        report.ticks,
                        report.episodes,
                        report.combat_events,
                        report.control_events,
                        report.build,
                    );
                    println!(
                        "PLAYTRACE_VERIFY_CHECKS state_hashes={} event_hashes={} effective_inputs={} observations={} final_observations={} diagnostics={} slot_routing={} labels={}",
                        report.state_hashes,
                        report.event_hashes,
                        report.effective_inputs,
                        report.observations,
                        report.final_observations,
                        report.diagnostics,
                        report.routing,
                        report.labels,
                    );
                    for note in &report.notes {
                        println!("PLAYTRACE_VERIFY_NOTE {note}");
                    }
                    Ok(())
                }
                Err(failure) => {
                    println!(
                        "PLAYTRACE_VERIFY session={session} result=FAIL code={} tick={} reason={:?}",
                        failure.code,
                        failure.tick.map_or("-".to_owned(), |t| t.to_string()),
                        failure.message,
                    );
                    // A rejected session is a verdict (exit 1), not a usage error.
                    std::process::exit(1);
                }
            }
        }
        _ => Err(USAGE.to_owned()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("playtrace: {message}");
            ExitCode::from(2)
        }
    }
}
