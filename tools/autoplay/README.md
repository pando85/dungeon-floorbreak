# AUTOPLAY-01C: Python bridge, Gymnasium env and CPU PPO smoke

Optional tooling over the native Rust `AutoplayEnv` (stage A, `src/autoplay/`).
The game never depends on any of this: the shipped binary and the root
`cargo test --locked --all-targets` do not compile PyO3 or NumPy (AP-Q02).
Design authority: `docs/03-tech/autoplay-01-automated-gameplay-evaluation.md`
(section 5 and AP-Q20).

```text
native/                 standalone PyO3 crate (own [workspace], own Cargo.lock,
                        path dependency on the game lib)
python/autoplay/        Gymnasium env, baselines, evaluate, train, CLI
tests/                  pytest suite, run against the installed wheel
scripts/smoke.sh        one-command local setup and smoke
pyproject.toml          maturin build backend, pinned dependencies
uv.lock                 locked, CPU-only Python resolution
```

## One command

```bash
tools/autoplay/scripts/smoke.sh            # build, tests, check_env, baseline, 10k-step PPO
tools/autoplay/scripts/smoke.sh --no-train # everything except torch / SB3
```

Needs `uv`, a Rust toolchain and the game's Linux build dependencies
(`pkg-config libasound2-dev libudev-dev`). No GPU, no sudo, no network service.
The first build compiles Bevy in release mode (about 10 minutes on a busy
machine); later builds are incremental.

## Step by step

```bash
cd tools/autoplay
uv venv --python 3.12 .venv && source .venv/bin/activate
uv pip install maturin==1.16.0
maturin build --release --manifest-path native/Cargo.toml --out dist
uv pip install --no-deps dist/*.whl
uv pip install gymnasium==1.4.0 numpy==2.5.4 pytest==9.2.0

# tests from another directory, against the installed wheel
cd "$(mktemp -d)" && AUTOPLAY_TEST_INSTALLED=1 python -m pytest <repo>/tools/autoplay/tests -q

python -m autoplay check-env
python -m autoplay evaluate --scenario cp01_single_goblin --policy random_legal \
  --episodes 50 --seed 42 --out artifacts/autoplay-01/eval
python -m autoplay benchmark --scenario cp01_single_goblin --steps 20000

# optional extra: CPU PPO (torch comes from the PyTorch CPU index)
uv pip install stable-baselines3==2.9.2 torch==2.15.0 \
  --index-url https://download.pytorch.org/whl/cpu \
  --extra-index-url https://pypi.org/simple --index-strategy unsafe-best-match
python -m autoplay train --algorithm ppo --scenario cp01_single_goblin --steps 10000 \
  --seed 42 --out artifacts/autoplay-01/ppo-smoke
python -m autoplay evaluate --checkpoint artifacts/autoplay-01/ppo-smoke \
  --scenario cp01_single_goblin --scenario cp02_two_goblins --episodes 20 --out artifacts/autoplay-01/ppo-smoke/eval_reload
```

The wheel finds the game assets through, in order: the `assets_root` argument,
the `AUTOPLAY_ASSETS_ROOT` environment variable, then the checkout it was built
from. Set `AUTOPLAY_ASSETS_ROOT` when you move the wheel to another machine.

`uv sync --extra test --extra train` also works from `tools/autoplay` against
the committed `uv.lock` (editable install, release profile).

Native package checks (a standalone package, so root fmt and clippy do not
cover it):

```bash
cd tools/autoplay/native
cargo fmt --check && cargo clippy --release --locked --all-targets -- -D warnings
cargo test --release --locked
```

## Contract

- `ActionV1`: `MultiDiscrete([3, 2, 2, 2, 2, 2])` (horizontal, jump, primary,
  physical, reserved special, reserved swap). Wrong length or out-of-range
  values raise `InvalidActionError`; nothing is clamped.
- `ObservationV1`: `spaces.Dict` of `float32` Boxes (`self` 14, `enemies`
  16x14, `enemy_mask` 16, `hazards` 8x6, `hazard_mask` 8, `available` 2),
  built from the native schema, so `MultiInputPolicy` applies.
- `reset(seed, options) -> (obs, info)`, `step(action) -> (obs, reward,
  terminated, truncated, info)`. `step` before `reset` or after the episode
  ended raises `EpisodeOverError`; nothing resets silently. Timeout is
  `truncated`; success, defeat and abandoned are `terminated`.
- `info` carries bookkeeping only (AP-Q08). State hashes and diagnostics are
  behind `env.state_hash()` and `env.diagnostics()`, for tests and QA.
- The seed seeds policy-side randomness only: the simulation has no gameplay
  RNG on this build, so `info["seed_affects_sim"]` is `False`.
- Baselines read the observation only. Stage C ships `noop` and `random_legal`;
  the scripted Rust bots and native replay verification arrive with stage B.

## Checkpoints

`train` writes `model.zip` and `metadata.json` (package versions, algorithm
and hyperparameters, seed, observation/action/reward versions and schema
hash, commit, asset content hash, train/development/hold-out scenario lists,
model sha256). Reload refuses a mismatched schema or a changed model file.
Checkpoints, venvs and build products are git-ignored. A 10k-step PPO run
demonstrates the pipeline; it is not evidence of a strong policy.

Note: `scripts/check_docs.py` globs every `*.md` and `*.json` under the
checkout, so it also walks `tools/autoplay/.venv`. Run it before creating the
venv, or from a clean checkout (CI does).
