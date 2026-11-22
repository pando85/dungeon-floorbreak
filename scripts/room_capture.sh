#!/usr/bin/env bash
# EVID-ROOM: the deterministic scripted capture of an f01_late room scene.
# Reusable by every room evidence stage (BOIL-F, HAZ-C, HRD-D, ...).
#
#   scripts/room_capture.sh ROOM PLAN FIRST:STEP:COUNT OUTDIR [more floor01_scene flags]
#
#   ROOM   boiler | hazards | mastery | juicer   (--scene f01_late --f01-room ROOM)
#   PLAN   direct | valve | chain | mastery | juicer, optionally ":2p"   (--room-pilot, src/room_pilots.rs;
#          mastery and juicer are valid only in their own room)
#
# The pilot plays the room from the finished simulation state alone (no
# synthetic input into any window, no clock, no RNG), so the run is a pure
# function of the flags. The script runs it twice: once headless to FIRST+STEP*(COUNT-1)
# ticks (the state hash), once in a real window under xvfb (the Bevy presentation,
# one PNG per STEP ticks). The two final COMBAT_STATE hashes must be equal: that is
# the evidence that the presentation is read-only. For `:2p` plans a third, timed
# window run (`--exit-after-ticks`) must end on that hash too. Output: frame_NNNN_tick_TTTTT.png,
# run.log (the window run) and capture.txt (one ROOM_CAPTURE line).
#
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene), needs the exported
#   atlases (bash scripts/export_assets.sh) and xvfb-run.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
room="${1:?room}"
plan="${2:?plan}"
seq="${3:?first:step:count}"
out="${4:?out dir}"
shift 4
bin="${BIN:-./target/debug/floor01_scene}"
IFS=: read -r first step count <<<"$seq"
last=$((first + step * (count - 1)))
rm -rf "$out"
mkdir -p "$out"
common=(--scene f01_late --f01-room "$room" --room-pilot "$plan" "$@")
# The window run (Bevy presentation, frames).
xvfb-run -a -s '-screen 0 1280x720x24' "$bin" --scale 1 "${common[@]}" \
  --capture-sequence "$seq" --capture-dir "$out" > "$out/run.log" 2>&1
# The headless run to the same tick (no presentation).
"$bin" --headless-floor01-smoke "${common[@]}" --capture-tick "$last" > "$out/headless.log" 2>&1
# The last COMBAT_STATE of each log: the state after the final captured tick (a
# room that was completed stops advancing its scene tick, `elapsed` goes on).
state() { grep "^COMBAT_STATE " "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
window_hash="$(state "$out/run.log")"
headless_hash="$(state "$out/headless.log")"
# A timed window run (`--exit-after-ticks`, the live loop) takes the same input as the
# capture sequence: checked for every `:2p` plan (where Donut's slot matters) and
# whenever CHECK_TIMED=1. It must end on the same hash as the headless replay.
timed_hash=skipped
if [[ "$plan" == *:2p || "${CHECK_TIMED:-0}" == 1 ]]; then
  xvfb-run -a -s '-screen 0 1280x720x24' "$bin" --scale 1 "${common[@]}" \
    --exit-after-ticks "$last" > "$out/timed.log" 2>&1
  timed_hash="$(state "$out/timed.log")"
fi
frames="$(find "$out" -name 'frame_*.png' | wc -l)"
match=no
[[ -n "$window_hash" && "$window_hash" == "$headless_hash" ]] && match=yes
if [[ "$timed_hash" != skipped && "$timed_hash" != "$headless_hash" ]]; then match=no; fi
echo "ROOM_CAPTURE room=$room plan=$plan frames=$frames first_tick=$first step=$step last_tick=$last window_hash=${window_hash:-none} headless_hash=${headless_hash:-none} timed_hash=$timed_hash match=$match" | tee "$out/capture.txt"
[[ "$match" == yes && "$frames" == "$count" ]]
