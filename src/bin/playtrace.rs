//! PLAYTRACE-01 session tool: `list`, `inspect`, `recover` and `delete`.
//!
//! `verify` (fresh-process replay) is PT-C and is not implemented here.
//! Recordings stay local: this tool never uploads or exports anything.

use std::path::PathBuf;
use std::process::ExitCode;

use dungeon_floorbreak::playtrace::{
    self, ControlEvent, SessionStatus, dir_size_bytes, list_sessions, read_session, recover_session,
};

const USAGE: &str =
    "usage: playtrace <list|inspect ID|recover ID|delete ID --yes|verify> [--dir DIR]
  list              every recorded session (id, status, ticks, day, commit)
  inspect ID        manifest summary, episodes, control events and integrity of one session
  recover ID        seal a crashed session to its valid complete prefix
  delete ID --yes   remove one session directory (local deletion; nothing is uploaded)
  verify            not implemented yet (PLAYTRACE-01 PT-C)
ID is a session id or a unique prefix of one. --dir overrides the base directory
(default: $XDG_DATA_HOME/dungeon-floorbreak/playtrace).";

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
            for (_, m) in sessions {
                let status = if m.status == SessionStatus::Recording {
                    "Recording(unfinished: running or crashed; inspect or recover)".to_owned()
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
            if loaded.recovered {
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
        "delete" => {
            let base = base_dir(args)?;
            let path = resolve(&base, id.ok_or("delete needs a session id")?)?;
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
        "verify" => Err("verify is not implemented yet (PLAYTRACE-01 PT-C)".to_owned()),
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
