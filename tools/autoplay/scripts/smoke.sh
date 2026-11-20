#!/usr/bin/env bash
# AUTOPLAY-01 local setup and smoke (CPU only, no GPU, no sudo).
#
#   tools/autoplay/scripts/smoke.sh            # full: build, tests, baseline, replay verify, PPO
#   tools/autoplay/scripts/smoke.sh --no-train # skip the torch/SB3 part
#
# Sequence: build the Rust binding and the native CLI -> check_env -> evaluate
# a baseline (native Rust bot) -> verify its native replay -> train a small PPO
# -> reload and evaluate -> verify the PPO replay.
#
# Needs: uv (https://docs.astral.sh/uv/), a Rust toolchain, the Linux build
# dependencies of the game (pkg-config, libasound2-dev, libudev-dev, the X11,
# xkbcommon, wayland and vulkan dev packages).
#
# The venv lives outside the repository (scripts/check_docs.py walks every
# *.md and *.json below the repo root): AUTOPLAY_VENV overrides the default.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
tool="$root/tools/autoplay"
out="${AUTOPLAY_OUT:-$root/artifacts/autoplay-01}"
venv="${AUTOPLAY_VENV:-${XDG_CACHE_HOME:-$HOME/.cache}/dungeon-floorbreak-autoplay/venv}"
train=1
[ "${1:-}" = "--no-train" ] && train=0

total_started=$SECONDS
step() {
  echo "[$((SECONDS - total_started))s] $*"
}

cd "$tool"
rm -rf "$venv" dist "$out/eval" "$out/ppo-smoke"
uv venv --python 3.12 "$venv"
# shellcheck disable=SC1091
source "$venv/bin/activate"

# 1. Build the Rust binding into a wheel and install it (non-editable), then
#    build the native CLI that verifies replays. Both share one target dir so
#    Bevy compiles once.
step "1/8 build the extension wheel and the native autoplay CLI"
uv pip install "maturin==1.16.0"
maturin build --release --locked --manifest-path native/Cargo.toml --out dist
uv pip install --no-deps dist/*.whl
uv pip install "gymnasium==1.4.0" "numpy==2.5.4" "pytest==9.2.0"
CARGO_TARGET_DIR="$tool/native/target" cargo build --release --locked --bin autoplay \
  --manifest-path "$root/Cargo.toml"
cli="$tool/native/target/release/autoplay"

# 2. Tests run from another directory against the installed wheel.
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
step "2/8 Python tests (installed package, native CLI replay verify)"
(cd "$scratch" && AUTOPLAY_TEST_INSTALLED=1 AUTOPLAY_REQUIRE_CLI=1 AUTOPLAY_CLI="$cli" \
  python -m pytest "$tool/tests" -q)

# 3. Gymnasium check_env.
step "3/8 check_env"
(cd "$scratch" && python -m autoplay check-env)

# 4. Seeded baseline: the native Rust bot, driven through the binding.
step "4/8 evaluate the reactive_survivor baseline (native bot)"
(cd "$scratch" && python -m autoplay evaluate --scenario cp01_single_goblin \
  --policy reactive_survivor --episodes 50 --seed 42 --out "$out/eval")

# 5. Verify the Python-written episode replay in a fresh native process.
step "5/8 verify the baseline replay with the Rust CLI"
for replay in "$out"/eval/replays/*/*/episode_*.jsonl; do
  "$cli" replay --episode "$replay" --verify --assets-root "$root"
done

# 6. Bounded CPU PPO: train, save, reload, evaluate.
if [ "$train" = 1 ]; then
  step "6/8 install stable-baselines3 and CPU torch"
  uv pip install "stable-baselines3==2.9.2" "torch==2.15.0" \
    --index-url https://download.pytorch.org/whl/cpu \
    --extra-index-url https://pypi.org/simple --index-strategy unsafe-best-match
  step "7/8 train a 10k-step PPO, save, reload and evaluate"
  (cd "$scratch" && python -m autoplay train --algorithm ppo --scenario cp01_single_goblin \
    --steps 10000 --seed 42 --out "$out/ppo-smoke")
  (cd "$scratch" && python -m autoplay evaluate --checkpoint "$out/ppo-smoke" \
    --scenario cp01_single_goblin --scenario cp02_two_goblins --episodes 20 --seed 42 \
    --out "$out/ppo-smoke/eval_reload")
  step "8/8 verify the PPO checkpoint replays with the Rust CLI"
  for replay in "$out"/ppo-smoke/eval_ppo/replays/*/ppo/episode_*.jsonl \
    "$out"/ppo-smoke/eval_reload/replays/*/ppo/episode_*.jsonl; do
    "$cli" replay --episode "$replay" --verify --assets-root "$root"
  done
fi
step "autoplay smoke ok: $out"
