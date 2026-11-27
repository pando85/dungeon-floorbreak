//! TROG-B: run the STATIONARY-DONUT sweep of the Test of Strength on the real party simulation
//! and write `summary.json` and `report.txt` (plan 7.2 and 7.3).
//!
//! ```text
//! cargo run --release --locked --bin trog_eval -- --out target/trog-eval
//!     [--assets-root DIR] [--quick]
//! ```
//!
//! No Python, GPU or trained policy is involved. The full sweep covers the gated Test of
//! Strength (every gated stationary policy from the natural starts, the swept jump-cast, the
//! movement rows), the run's pockets T1..T3 and the 16 px diagnostic grid (both reported);
//! `--quick` runs the gate alone. The exit code is 1 when the sweep cannot run or when the gate
//! fails. `TROG_EVAL_THREADS` sets the worker threads.

use std::path::PathBuf;
use std::process::ExitCode;

use dungeon_floorbreak::trog_eval::{self, SweepConfig};

fn main() -> ExitCode {
    let mut out = PathBuf::from("target/trog-eval");
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut quick = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => out = args.next().map(PathBuf::from).unwrap_or(out),
            "--assets-root" => root = args.next().map(PathBuf::from).unwrap_or(root),
            "--quick" => quick = true,
            other => {
                eprintln!(
                    "unknown argument {other}; usage: trog_eval --out DIR [--assets-root DIR] [--quick]"
                );
                return ExitCode::from(2);
            }
        }
    }
    let config = if quick {
        SweepConfig::quick()
    } else {
        SweepConfig::full()
    };
    let evaluation = trog_eval::evaluate(&root, &config);
    let passes = evaluation.gate.passes();
    match trog_eval::write_to(&evaluation, &out) {
        Ok(()) => {
            print!("{}", evaluation.text());
            println!("wrote {}", out.display());
            if passes {
                ExitCode::SUCCESS
            } else {
                eprintln!("trog_eval: the Test of Strength gate fails (see report.txt)");
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("trog_eval: {e}");
            ExitCode::FAILURE
        }
    }
}
