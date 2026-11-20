#!/usr/bin/env bash
# AUTOPLAY-01C single local setup and smoke command (CPU only, no GPU, no sudo).
#
#   tools/autoplay/scripts/smoke.sh            # full: build, tests, baseline, PPO
#   tools/autoplay/scripts/smoke.sh --no-train # skip the torch/SB3 part
#
# Needs: uv (https://docs.astral.sh/uv/), a Rust toolchain, the Linux build
# dependencies of the game (pkg-config, libasound2-dev, libudev-dev).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
tool="$root/tools/autoplay"
out="${AUTOPLAY_OUT:-$root/artifacts/autoplay-01}"
train=1
[ "${1:-}" = "--no-train" ] && train=0

cd "$tool"
rm -rf .venv dist
uv venv --python 3.12 .venv
# shellcheck disable=SC1091
source .venv/bin/activate

# 1. Build the native extension into a wheel and install it (non-editable).
uv pip install "maturin==1.16.0"
maturin build --release --manifest-path native/Cargo.toml --out dist
uv pip install --no-deps dist/*.whl
uv pip install "gymnasium==1.4.0" "numpy==2.5.4" "pytest==9.2.0"

# 2. Tests run from another directory against the installed wheel.
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
(cd "$scratch" && AUTOPLAY_TEST_INSTALLED=1 python -m pytest "$tool/tests" -q)

# 3. Gymnasium check_env, then a seeded baseline evaluation.
(cd "$scratch" && python -m autoplay check-env)
(cd "$scratch" && python -m autoplay evaluate --scenario cp01_single_goblin \
  --policy random_legal --episodes 50 --seed 42 --out "$out/eval")

# 4. Bounded CPU PPO: train, save, reload, evaluate.
if [ "$train" = 1 ]; then
  uv pip install "stable-baselines3==2.9.2" "torch==2.15.0" \
    --index-url https://download.pytorch.org/whl/cpu \
    --extra-index-url https://pypi.org/simple --index-strategy unsafe-best-match
  (cd "$scratch" && python -m autoplay train --algorithm ppo --scenario cp01_single_goblin \
    --steps 10000 --seed 42 --out "$out/ppo-smoke")
  (cd "$scratch" && python -m autoplay evaluate --checkpoint "$out/ppo-smoke" \
    --scenario cp01_single_goblin --scenario cp02_two_goblins --episodes 20 --seed 42 \
    --out "$out/ppo-smoke/eval_reload")
fi
echo "autoplay smoke ok: $out"
