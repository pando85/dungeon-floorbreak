#!/usr/bin/env bash
# BOIL-F evidence: real windowed captures of the Goblin Boiler (`--scene f01_late
# --f01-room boiler`) played by the room pilots (src/room_pilots.rs), solo and with
# two local players, assembled into GIFs under docs/evidence/f01-boiler/, plus the
# committed pilot hashes. Needs a debug binary (cargo build --locked --bin
# floor01_scene), the exported atlases (bash scripts/export_assets.sh), xvfb-run and
# Pillow. The README is hand-written.
#   BIN=<path to floor01_scene> (default ./target/debug/floor01_scene)
#   CAPTURES=<scratch dir> (default artifacts/f01-boiler-captures)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
caps="${CAPTURES:-artifacts/f01-boiler-captures}"
out=docs/evidence/f01-boiler
rm -rf "$caps"
mkdir -p "$caps" "$out"
# name|plan|first|step|count|extra flags  (step 6 ticks = 100 ms: the GIF plays at game speed)
clips=(
  "direct-solo|direct|180|6|60"
  "direct-2p|direct:2p|180|6|50"
  "valve-solo|valve|160|6|45"
  "valve-2p|valve:2p|160|6|45"
  "chain-solo|chain|440|6|60"
  "chain-2p|chain:2p|440|6|60"
  "chain-solo-640x400|chain|440|6|60|--deck"
  "b3-wake-chain-solo|chain|690|3|16"
)
: > "$out/capture_lines.txt"
for spec in "${clips[@]}"; do
  IFS='|' read -r name plan first step count flags <<<"$spec"
  # shellcheck disable=SC2086
  bash scripts/room_capture.sh boiler "$plan" "$first:$step:$count" "$caps/$name" $flags
  echo "$(tail -n 1 "$caps/$name/capture.txt") clip=$name" >> "$out/capture_lines.txt"
  python3 scripts/room_gif.py "$caps/$name" "$out/boiler-$name.gif" --step-ticks "$step"
done
# The committed pilot hashes: the final COMBAT_STATE of every plan, headless.
python3 scripts/room_pilot_hashes.py boiler > "$out/pilot_hashes.txt"
(cd "$out" && sha256sum ./*.gif ./*.txt | sed 's# \./# #' > SHA256SUMS)
