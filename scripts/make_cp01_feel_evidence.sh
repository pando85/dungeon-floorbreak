#!/usr/bin/env bash
# Build the CP-01 evidence (GIFs, stills, per-tick strip, timing diagrams, logs,
# SHA256SUMS) in docs/evidence/cp01-feel from the real Bevy capture that
#   xvfb-run -a -s '-screen 0 1920x1080x24' bash scripts/ci_floor01_combat.sh render-sequences
# writes to artifacts/cp01-feel. Needs the debug binary (cargo build --locked
# --bin floor01_scene), Pillow and matplotlib. The README is hand-written.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
python3 scripts/make_cp01_feel_evidence.py
