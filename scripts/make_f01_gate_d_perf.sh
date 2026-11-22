#!/usr/bin/env bash
# PT-GATE-D (plan section 10, item 7): the Boiler's full chain at 640x400 (`--deck`), three runs, gore
# preset 4, appended to docs/evidence/f01-playtest/perf.txt after the three 640x360 runs of PT-GATE-A
# (scripts/make_f01_gate_a_perf.sh; any earlier 640x400 block is replaced). Run it on a quiet machine:
# the interval is wall-clock and every ROOM_PERF line carries the load average. See scripts/room_perf.sh.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
out=docs/evidence/f01-playtest
mkdir -p "$out"
touch "$out/perf.txt"
# Keep the 640x360 runs: every block that is not a "640x400" one.
awk '/^# /{skip = ($0 ~ /640x400/)} !skip' "$out/perf.txt" > "$out/perf.txt.tmp"
for run in 1 2 3; do
  echo "# boiler chain, 640x400, run $run" >> "$out/perf.txt.tmp"
  bash scripts/room_perf.sh boiler chain 950 --deck | grep -v '^log=' >> "$out/perf.txt.tmp"
done
mv "$out/perf.txt.tmp" "$out/perf.txt"
(cd "$out" && sha256sum ./*.gif ./*.png ./*.txt ./*.md | grep -v ' ./README.md$' | sed 's# \./# #' > SHA256SUMS)
