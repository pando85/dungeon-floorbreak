#!/usr/bin/env bash
# PT-GATE-B: the mastery arena's frame time (gate item 7) into docs/evidence/f01-playtest/perf_mastery.txt:
# three runs at 640x360 and three at 640x400 (--deck), gore preset 4, the real Bevy window under xvfb.
# FRAMES is the room pilot's exit tick plus one: the pilot clears the tuned arena and reaches the exit
# on tick 1190 (the README), after being defeated once on tick 908, so 1191 frames end on the exit and no frame measures a finished, static room.
# Run it on a quiet machine: the interval is wall-clock and every ROOM_PERF line carries the load average.
# See scripts/room_perf.sh for what is measured and what it is not (xvfb, not real hardware).
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
out=docs/evidence/f01-playtest
frames=1191
mkdir -p "$out"
: > "$out/perf_mastery.txt"
for view in 640x360 640x400; do
  extra=()
  [[ "$view" == 640x400 ]] && extra=(--deck)
  for run in 1 2 3; do
    echo "# mastery arena, $view, run $run" >> "$out/perf_mastery.txt"
    bash scripts/room_perf.sh mastery mastery "$frames" "${extra[@]}" | grep -v '^log=' >> "$out/perf_mastery.txt"
  done
done
(cd "$out" && sha256sum ./*.gif ./*.png ./*.txt ./*.md | grep -v ' ./README.md$' | sed 's# \./# #' > SHA256SUMS)
