//! Build identity for PLAYTRACE-01 (std only, no new crate).
//!
//! Emits four compile-time values the session recorder and the verifier read
//! (`src/playtrace/build_info.rs`):
//!
//! - `DF_BUILD_COMMIT`: the commit of the checkout (`unknown` without git);
//! - `DF_BUILD_DIRTY`: `true` / `false` (`unknown` without git);
//! - `DF_BUILD_TOOLCHAIN`: the `rustc -V` line;
//! - `DF_SOURCE_FINGERPRINT`: FNV-1a-64 over the toolchain line, `Cargo.toml`,
//!   `Cargo.lock`, every file below `src/` and every file below `assets/data/`
//!   (each sorted by relative path; `assets/data/` holds the sim tuning that
//!   `include_str!` compiles in, for example the boss and companion JSON), so
//!   two binaries of the same source tree and toolchain carry the same value
//!   even when the tree is dirty.
//!
//! Git being unavailable never fails the build: the values become `unknown`.
//! No absolute path is emitted. Rebuild triggers are the sources, `assets/data`, the manifest
//! files and the git `HEAD`, the current ref, `packed-refs` and `index`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Every file below `dir`, relative to `root`, in a stable order.
fn files_below(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            files_below(root, &path, out);
        } else if let Ok(relative) = path.strip_prefix(root) {
            out.push(relative.to_path_buf());
        }
    }
}

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));

    let commit = git(&root, &["rev-parse", "HEAD"])
        .filter(|c| c.len() >= 7)
        .unwrap_or_else(|| "unknown".to_owned());
    let dirty = if commit == "unknown" {
        "unknown"
    } else {
        match git(&root, &["status", "--porcelain", "--untracked-files=no"]) {
            Some(text) if text.is_empty() => "false",
            Some(_) => "true",
            None => "unknown",
        }
    };

    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let toolchain = Command::new(rustc)
        .arg("-V")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned());

    let mut files = vec![PathBuf::from("Cargo.toml"), PathBuf::from("Cargo.lock")];
    files_below(&root, &root.join("src"), &mut files);
    files_below(&root, &root.join("assets").join("data"), &mut files);
    let mut hash = fnv(FNV_OFFSET, toolchain.as_bytes());
    for relative in &files {
        hash = fnv(
            hash,
            relative.to_string_lossy().replace('\\', "/").as_bytes(),
        );
        hash = fnv(hash, &[0]);
        let bytes = fs::read(root.join(relative)).unwrap_or_default();
        hash = fnv(hash, &(bytes.len() as u64).to_le_bytes());
        hash = fnv(hash, &bytes);
    }

    println!("cargo:rustc-env=DF_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=DF_BUILD_DIRTY={dirty}");
    println!("cargo:rustc-env=DF_BUILD_TOOLCHAIN={toolchain}");
    println!("cargo:rustc-env=DF_SOURCE_FINGERPRINT={hash:016x}");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=assets/data");
    // Git state: a worktree keeps `HEAD` and `index` in its own git dir and
    // the refs in the common dir. Any of them missing is fine (no git).
    if let Some(git_dir) = git(&root, &["rev-parse", "--absolute-git-dir"]) {
        println!("cargo:rerun-if-changed={git_dir}/HEAD");
        println!("cargo:rerun-if-changed={git_dir}/index");
    }
    if let Some(common) = git(&root, &["rev-parse", "--git-common-dir"]) {
        let common = if Path::new(&common).is_absolute() {
            PathBuf::from(common)
        } else {
            root.join(common)
        };
        println!(
            "cargo:rerun-if-changed={}",
            common.join("packed-refs").display()
        );
        if let Some(reference) = git(&root, &["symbolic-ref", "-q", "HEAD"]) {
            println!(
                "cargo:rerun-if-changed={}",
                common.join(reference).display()
            );
        }
    }
}
