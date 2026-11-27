#!/usr/bin/env bash
# PLAYTEST-READY-01: a headless smoke of the whole floor run. One `--headless-floor01-smoke --scene f01_late
# --f01-room ID` launch per manifest row, in manifest (run) order, each of which must print the room's entry line,
# `FLOOR1_HEADLESS_OK` and its `*_SMOKE_OK` marker (the Juicer and the Ball of Swine print `FLOOR1_HEADLESS_OK` only). The order and the
# row count come from `assets/levels/f01_late/manifest.json`, the same data `f01_run::next_room` walks. This proves
# that every listed room builds, runs 600 ticks and reaches its entry state on this head; it is NOT one carried
# `--f01-run` (a floor run relaunches the binary per room, and a piloted run cannot cross a boundary: ROOM-PILOT-RUN).
# The carried hand-off between rooms is covered by the `f01_run` tests (`cargo test f01_run`) and the ignored helper
# `print_the_carried_run_chain`.
# The script fails (exit 1) when the manifest cannot be read, yields no rooms or a count other than the expected one,
# when a launch fails, when the entry line does not name the requested room, or when the room's own marker
# (`F01_<ROOM>_SMOKE_OK`, ROOM upper case) is missing. The boss rooms (the Juicer, `boss_f01`, and the Ball of Swine, `bos`, BOS-A) are the
# documented exceptions: they have no `*_SMOKE_OK` marker, so they need `FLOOR1_HEADLESS_OK` and their entry line only.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)   MANIFEST=<manifest.json> (default the committed one)
#   F01_RUN_ROOMS=<count> (default 18, the rooms of the floor run)   Output: stdout (one RUN_SMOKE line per room).
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
manifest="${MANIFEST:-assets/levels/f01_late/manifest.json}"
expected="${F01_RUN_ROOMS:-18}"
list="$(python3 - "$manifest" <<'PY'
import json, sys
for row in json.load(open(sys.argv[1]))["rooms"]:
    print(row["id"])
PY
)" || { echo "RUN_SMOKE cannot read the room list from $manifest" >&2; exit 1; }
mapfile -t rooms <<< "$list"
n=${#rooms[@]}
[[ -n "$list" ]] && (( n > 0 )) || { echo "RUN_SMOKE no rooms read from $manifest" >&2; exit 1; }
(( n == expected )) || { echo "RUN_SMOKE expected $expected rooms, $manifest lists $n" >&2; exit 1; }
fail=0
for i in "${!rooms[@]}"; do
  room="${rooms[$i]}"
  next="end"
  if (( i + 1 < n )); then next="${rooms[$((i + 1))]}"; fi
  log="$(mktemp -t f01_run_smoke.XXXXXX)"
  timeout 300 "$bin" --headless-floor01-smoke --scene f01_late --f01-room "$room" > "$log" 2>&1
  code=$?
  entry="$(grep -m1 "^F01_LATE_ENTRY room=$room " "$log" | cut -c1-200)"
  ok="$(grep -c '^FLOOR1_HEADLESS_OK' "$log")"
  want="F01_${room^^}_SMOKE_OK"
  marker="$(grep -m1 "^$want" "$log" | awk '{print $1}')"
  hash="$(grep '^COMBAT_STATE' "$log" | tail -n 1 | grep -o 'hash=[0-9a-f]*' | head -n 1)"
  status=ok
  if [[ "$code" != 0 || "$ok" == 0 || -z "$entry" ]]; then status=FAILED; fail=1; fi
  if [[ -z "$marker" && "$room" != juicer && "$room" != bos ]]; then status=FAILED; fail=1; fi
  echo "RUN_SMOKE $((i + 1))/$n room=$room next=$next status=$status exit=$code marker=${marker:--} ${hash:-hash=-}"
  echo "  $entry"
  rm -f "$log"
done
exit "$fail"
