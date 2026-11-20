//! Python-free helpers of the bridge: asset root resolution and the native
//! reference run used by the parity test. Nothing here touches PyO3, so it is
//! covered by a plain `cargo test` inside this package.

use std::path::PathBuf;

use dungeon_floorbreak::autoplay::{ActionV1, AutoplayConfig, AutoplayEnv, AutoplayError};

/// Environment variable that overrides the asset root at run time.
pub const ASSETS_ROOT_ENV: &str = "AUTOPLAY_ASSETS_ROOT";

/// Repository root recorded at build time: the default asset root when the
/// extension was built from a checkout (an editable or in-tree build).
pub fn default_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// Asset root precedence: explicit argument, then `AUTOPLAY_ASSETS_ROOT`, then
/// the build-time checkout.
pub fn resolve_assets_root(explicit: Option<String>) -> PathBuf {
    explicit
        .map(PathBuf::from)
        .or_else(|| std::env::var_os(ASSETS_ROOT_ENV).map(PathBuf::from))
        .unwrap_or_else(default_assets_root)
}

/// Step a fresh native env through `actions` and return the authoritative
/// `state_hash` after every executed `step`. Stops at the end of the episode,
/// exactly like a Gymnasium loop that honours `terminated` / `truncated`.
pub fn reference_hashes(
    config: AutoplayConfig,
    assets_root: &std::path::Path,
    seed: u64,
    actions: &[[u8; 6]],
) -> Result<Vec<u64>, AutoplayError> {
    let mut env = AutoplayEnv::new(config, assets_root)?;
    env.reset(seed)?;
    let mut hashes = Vec::with_capacity(actions.len());
    for values in actions {
        let outcome = env.step(ActionV1::new(*values)?)?;
        hashes.push(env.state_hash());
        if outcome.outcome.is_some() {
            break;
        }
    }
    Ok(hashes)
}

/// Pure-Rust throughput loop for the benchmark: steps `steps` times cycling
/// through `actions`, reading the observation after every step (the work a
/// Python caller also asks for), resetting at the end of an episode. Returns
/// `(steps, elapsed seconds)`.
pub fn bench_native(
    config: AutoplayConfig,
    assets_root: &std::path::Path,
    actions: &[[u8; 6]],
    steps: usize,
) -> Result<(usize, f64), AutoplayError> {
    let mut env = AutoplayEnv::new(config, assets_root)?;
    env.reset(0)?;
    let parsed: Vec<ActionV1> = actions
        .iter()
        .map(|a| ActionV1::new(*a))
        .collect::<Result<_, _>>()?;
    let started = std::time::Instant::now();
    let mut sink = 0.0_f32;
    for i in 0..steps {
        let outcome = env.step(parsed[i % parsed.len()])?;
        sink += env.observe().own[0];
        if outcome.outcome.is_some() {
            env.reset(0)?;
        }
    }
    let elapsed = started.elapsed().as_secs_f64();
    std::hint::black_box(sink);
    Ok((steps, elapsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixed parity sequence shared with `tools/autoplay/tests/test_parity.py`:
    /// walk right for 90 ticks, then jab (primary held) for 90 ticks.
    pub(crate) fn parity_actions() -> Vec<[u8; 6]> {
        let mut actions = vec![[2, 0, 0, 0, 0, 0]; 90];
        actions.extend(vec![[1, 0, 1, 0, 0, 0]; 90]);
        actions
    }

    /// Literal golden of the parity sequence on `cp01_single_goblin`, as
    /// `(step count, state_hash)`. The same literals are pinned in the Python
    /// parity test; a change to either side means the sim or the binding moved.
    const GOLDEN: [(usize, u64); 3] = [
        (1, 7_889_728_490_312_218_380),
        (90, 2_903_576_537_444_934_922),
        (180, 5_063_528_022_019_914_666),
    ];

    #[test]
    fn reference_hashes_are_pinned_and_deterministic() {
        let root = default_assets_root();
        let run = || {
            reference_hashes(
                AutoplayConfig::new("cp01_single_goblin"),
                &root,
                7,
                &parity_actions(),
            )
            .expect("reference run")
        };
        let first = run();
        assert_eq!(first, run(), "same actions, same hashes");
        assert_eq!(first.len(), 180);
        let seeded = reference_hashes(
            AutoplayConfig::new("cp01_single_goblin"),
            &root,
            12345,
            &parity_actions(),
        )
        .expect("reference run");
        assert_eq!(first, seeded, "the reset seed never reaches the sim");
        for (count, hash) in GOLDEN {
            assert_eq!(first[count - 1], hash, "hash after step {count}");
        }
    }

    #[test]
    fn explicit_root_beats_the_default() {
        assert_eq!(
            resolve_assets_root(Some("some/root".into())),
            PathBuf::from("some/root")
        );
    }
}
