//! BOS-A: run the STATIONARY gate of the Ball of Swine on the real party simulation and write
//! `summary.json` and `report.txt` (plan 9.3).
//!
//! ```text
//! cargo run --release --locked --bin bos_eval -- --out target/bos-eval
//!     [--assets-root DIR] [--q11]
//! ```
//!
//! `--q11` switches BOS-Q11 on (the siege counter ignores the follower bot in solo while the
//! player's hero is in an alcove); the committed default is off unless the solo rows fail
//! without it. The exit code is 1 when the gate fails or cannot run.

use std::path::PathBuf;
use std::process::ExitCode;

use dungeon_floorbreak::bos_eval;

fn main() -> ExitCode {
    let mut out = PathBuf::from("target/bos-eval");
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut q11 = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => out = args.next().map(PathBuf::from).unwrap_or(out),
            "--assets-root" => root = args.next().map(PathBuf::from).unwrap_or(root),
            "--q11" => q11 = true,
            other => {
                eprintln!(
                    "unknown argument {other}; usage: bos_eval --out DIR [--assets-root DIR] [--q11]"
                );
                return ExitCode::from(2);
            }
        }
    }
    match bos_eval::evaluate(&root, q11) {
        Ok(evaluation) => {
            let passes = evaluation.gate_passes();
            if let Err(e) = bos_eval::write_to(&evaluation, &out) {
                eprintln!("bos_eval: {e}");
                return ExitCode::FAILURE;
            }
            print!("{}", evaluation.text());
            println!("wrote {}", out.display());
            if passes {
                ExitCode::SUCCESS
            } else {
                eprintln!("bos_eval: the gate fails (see report.txt)");
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("bos_eval: {e}");
            ExitCode::FAILURE
        }
    }
}
