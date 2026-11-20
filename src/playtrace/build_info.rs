//! Build identity of this executable (PT-C robustness follow-up 1).
//!
//! The commit, the dirty flag, the toolchain and the source fingerprint are
//! embedded at **build time** by `build.rs`; they never change after the
//! binary is built (the old run-time `git` call recorded whatever the checkout
//! was when the game started). The executable fingerprint is computed from the
//! running binary's own bytes.

use std::sync::OnceLock;

/// The commit of the checkout this binary was built from (`unknown` when git
/// could not tell at build time).
pub const BUILD_COMMIT: &str = env!("DF_BUILD_COMMIT");
/// `true`, `false` or `unknown` (tracked files changed at build time).
pub const BUILD_DIRTY: &str = env!("DF_BUILD_DIRTY");
/// The `rustc -V` line of the build.
pub const BUILD_TOOLCHAIN: &str = env!("DF_BUILD_TOOLCHAIN");
/// FNV-1a-64 (hex) over the toolchain line, `Cargo.toml`, `Cargo.lock` and
/// every file below `src/` at build time. Equal for two binaries of one source
/// tree and toolchain, dirty or not: this is the identity a replay needs.
pub const SOURCE_FINGERPRINT: &str = env!("DF_SOURCE_FINGERPRINT");

/// What a manifest records about the build that produced the session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildIdentity {
    pub commit: String,
    /// `None` when git could not tell.
    pub dirty: Option<bool>,
    pub toolchain: String,
    pub source_fingerprint: String,
    /// `exe-w64/1:<length>:<hash>` of the running binary, or `unknown`.
    pub binary_fingerprint: String,
}

impl BuildIdentity {
    /// True when the commit alone does not identify the source (dirty or no
    /// git): the source fingerprint is then the only identity.
    pub fn commit_is_not_enough(&self) -> bool {
        self.commit == "unknown" || self.dirty != Some(false)
    }
}

/// The identity of the running binary. The executable fingerprint is computed
/// once per process (it reads the whole file).
pub fn build_identity() -> BuildIdentity {
    static EXE: OnceLock<String> = OnceLock::new();
    BuildIdentity {
        commit: BUILD_COMMIT.to_owned(),
        dirty: match BUILD_DIRTY {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        },
        toolchain: BUILD_TOOLCHAIN.to_owned(),
        source_fingerprint: SOURCE_FINGERPRINT.to_owned(),
        binary_fingerprint: EXE.get_or_init(executable_fingerprint).clone(),
    }
}

const WORD_PRIME: u64 = 0x0000_0100_0000_01b3;

/// A word-wise FNV-style hash of `bytes` folded into `hash` (8 bytes per
/// multiply; the tail is zero-padded and the length is mixed by the caller).
pub fn fold_words(mut hash: u64, bytes: &[u8]) -> u64 {
    let (words, tail) = bytes.as_chunks::<8>();
    for word in words {
        hash = (hash ^ u64::from_le_bytes(*word)).wrapping_mul(WORD_PRIME);
        hash ^= hash >> 29;
    }
    if !tail.is_empty() {
        let mut array = [0_u8; 8];
        array[..tail.len()].copy_from_slice(tail);
        hash = (hash ^ u64::from_le_bytes(array)).wrapping_mul(WORD_PRIME);
        hash ^= hash >> 29;
    }
    hash
}

/// `exe-w64/1:<length>:<hash>` of the running executable's bytes, `unknown`
/// when it cannot be read. Identifies this exact binary file (not the source:
/// see [`SOURCE_FINGERPRINT`]); the path is never stored.
pub fn executable_fingerprint() -> String {
    use std::io::Read;
    let Ok(path) = std::env::current_exe() else {
        return "unknown".to_owned();
    };
    let Ok(mut file) = std::fs::File::open(path) else {
        return "unknown".to_owned();
    };
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut total = 0_u64;
    // A multiple of 8 so every block but the last is whole words.
    let mut buffer = vec![0_u8; 1 << 20];
    loop {
        let mut filled = 0;
        while filled < buffer.len() {
            match file.read(&mut buffer[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(_) => return "unknown".to_owned(),
            }
        }
        if filled == 0 {
            break;
        }
        hash = fold_words(hash, &buffer[..filled]);
        total += filled as u64;
        if filled < buffer.len() {
            break;
        }
    }
    format!("exe-w64/1:{total}:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_identity_is_embedded_at_build_time() {
        let id = build_identity();
        assert!(
            id.commit == "unknown"
                || (id.commit.len() == 40 && id.commit.bytes().all(|b| b.is_ascii_hexdigit())),
            "commit {:?}",
            id.commit
        );
        assert!(id.toolchain.starts_with("rustc ") || id.toolchain == "unknown");
        assert_eq!(id.source_fingerprint.len(), 16);
        assert!(u64::from_str_radix(&id.source_fingerprint, 16).is_ok());
        // Git missing at build time is `unknown`, never a failed build and
        // never a guess.
        assert_eq!(id.dirty.is_none(), BUILD_DIRTY == "unknown");
        assert_eq!(id.commit == "unknown", BUILD_DIRTY == "unknown");
    }

    #[test]
    fn the_source_fingerprint_matches_a_recomputation_of_the_tree() {
        // The same algorithm as `build.rs`: toolchain line, `Cargo.toml`,
        // `Cargo.lock`, then every file below `src/` in path order.
        fn fnv(mut hash: u64, bytes: &[u8]) -> u64 {
            for byte in bytes {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
            hash
        }
        fn below(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            let mut paths: Vec<_> = std::fs::read_dir(dir)
                .unwrap()
                .flatten()
                .map(|e| e.path())
                .collect();
            paths.sort();
            for path in paths {
                if path.is_dir() {
                    below(root, &path, out);
                } else {
                    out.push(path.strip_prefix(root).unwrap().to_path_buf());
                }
            }
        }
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut files = vec!["Cargo.toml".into(), "Cargo.lock".into()];
        below(&root, &root.join("src"), &mut files);
        let mut hash = fnv(0xcbf2_9ce4_8422_2325, BUILD_TOOLCHAIN.as_bytes());
        for relative in &files {
            hash = fnv(hash, relative.to_string_lossy().as_bytes());
            hash = fnv(hash, &[0]);
            let bytes = std::fs::read(root.join(relative)).unwrap();
            hash = fnv(hash, &(bytes.len() as u64).to_le_bytes());
            hash = fnv(hash, &bytes);
        }
        assert_eq!(format!("{hash:016x}"), SOURCE_FINGERPRINT);
    }

    #[test]
    fn the_executable_fingerprint_names_the_running_binary() {
        let a = executable_fingerprint();
        let b = build_identity().binary_fingerprint;
        assert_eq!(a, b, "stable for one binary");
        let parts: Vec<&str> = a.split(':').collect();
        assert_eq!(parts.len(), 3, "{a}");
        assert_eq!(parts[0], "exe-w64/1");
        assert!(parts[1].parse::<u64>().unwrap() > 0);
        assert_eq!(parts[2].len(), 16);
    }

    #[test]
    fn the_word_hash_depends_on_every_byte_and_the_length() {
        let base = fold_words(1, b"0123456789abcdef!");
        assert_eq!(base, fold_words(1, b"0123456789abcdef!"));
        assert_ne!(base, fold_words(1, b"0123456789abcdeF!"));
        assert_ne!(base, fold_words(1, b"0123456789abcdef"));
    }
}
