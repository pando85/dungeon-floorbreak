#!/usr/bin/env bash
# EVID-ROOM bonus: a NON-FINAL first look at the hazard room (`--scene f01_late
# --f01-room hazards`) at gameplay scale (640 x 360), played by the `direct` room
# pilot, into docs/evidence/f01-hazards/first-look. It is the first use of
# scripts/room_capture.sh for HAZ-C and approves nothing: the pilot walks in and
# fights a goblin, it does not dodge the hazards (it is defeated). Needs a debug
# binary, the exported atlases, xvfb-run and Pillow. The README is hand-written.
#   BIN=<path to floor01_scene> (default ./target/debug/floor01_scene)
#   CAPTURES=<scratch dir> (default artifacts/f01-hazards-first-look)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
caps="${CAPTURES:-artifacts/f01-hazards-first-look}"
out=docs/evidence/f01-hazards/first-look
rm -rf "$caps"
mkdir -p "$caps" "$out"
bash scripts/room_capture.sh hazards direct 0:6:70 "$caps/direct" | tee "$out/capture_lines.txt"
# Stills (frame index: tick = index * 6): slime and goblin, the sticker's reach
# ring (tick 216), the sticker's blast (tick 240).
python3 scripts/room_gif.py "$caps/direct" "$out/hazards-first-look.gif" --step-ticks 6 \
  --still 8 "$out/still-slime-goblin-tick048.png" \
  --still 36 "$out/still-sticker-ring-tick216.png" \
  --still 40 "$out/still-sticker-blast-tick240.png"
(cd "$out" && sha256sum ./*.gif ./*.png ./*.txt | sed 's# \./# #' > SHA256SUMS)
