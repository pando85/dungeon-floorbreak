//! CORRAL-EVAL core: run the STATIONARY-DONUT sweep on the real party sim and
//! write `summary.json`, `runs.csv` and `report.md` (CORRAL-AUTO-01).
//!
//! ```text
//! cargo run --release --locked --bin corral_eval -- --out target/corral-eval
//!     [--assets-root DIR] [--quick] [--horizon TICKS]
//! ```
//!
//! No Python, GPU or trained policy is involved. The M2 gate in the report is
//! report-only until CORRAL-C; the exit code is 0 unless the sweep itself
//! cannot run.

use std::path::PathBuf;
use std::process::ExitCode;

use dungeon_floorbreak::corral_eval::{self, SweepConfig};

fn main() -> ExitCode {
    let mut out = PathBuf::from("target/corral-eval");
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut quick = false;
    let mut horizon = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => out = args.next().map(PathBuf::from).unwrap_or(out),
            "--assets-root" => root = args.next().map(PathBuf::from).unwrap_or(root),
            "--quick" => quick = true,
            // The KC19 clear limit stays 1,800 ticks: a longer horizon never
            // widens what counts as a clear (see `is_low_risk_stationary_clear`).
            "--horizon" => horizon = args.next().and_then(|v| v.parse::<u64>().ok()),
            other => {
                eprintln!(
                    "unknown argument {other}; usage: corral_eval --out DIR [--assets-root DIR] [--quick] [--horizon TICKS]"
                );
                return ExitCode::from(2);
            }
        }
    }
    let mut config = if quick {
        SweepConfig::quick()
    } else {
        SweepConfig::full()
    };
    if let Some(h) = horizon {
        config.horizon = h;
    }
    match corral_eval::evaluate_to(&root, &out, &config) {
        Ok(summary) => {
            println!("{summary}");
            println!("wrote {}", out.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("corral_eval: {e}");
            ExitCode::FAILURE
        }
    }
}
