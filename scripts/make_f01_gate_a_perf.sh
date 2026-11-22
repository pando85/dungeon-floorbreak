#!/usr/bin/env bash
# PT-GATE-A: the frame-time numbers of gate item 7 into docs/evidence/f01-playtest/perf.txt:
# the Boiler's full chain (three runs, gore preset 4). The mastery arena (room 6) is PT-GATE-B's:
# its command is `bash scripts/room_perf.sh mastery mastery N` with N = the tuned pilot's exit tick + 1 (1191 on the current data: exit at tick 1190), NOT RUN here. Run on a quiet machine: the
# interval is wall-clock, and every ROOM_PERF line carries the load average after the run.
# See scripts/room_perf.sh for what is measured and what it is not (xvfb, not real hardware).
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
out=docs/evidence/f01-playtest
mkdir -p "$out"
: > "$out/perf.txt"
for run in 1 2 3; do
  echo "# boiler chain, run $run" >> "$out/perf.txt"
  bash scripts/room_perf.sh boiler chain 950 | grep -v '^log=' >> "$out/perf.txt"
done

(cd "$out" && sha256sum ./*.gif ./*.png ./*.txt ./*.md | grep -v ' ./README.md$' | sed 's# \./# #' > SHA256SUMS)
