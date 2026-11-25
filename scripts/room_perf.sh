#!/usr/bin/env bash
# PT-GATE-A: the frame-time probe of an f01_late room played by its room pilot
# (gate item 7: "frame time with gore preset 4 ... --perf-frames").
#
#   scripts/room_perf.sh ROOM PLAN FRAMES [more floor01_scene flags]
#
#   ROOM    hoarder | boiler | dozer | hazards | mastery   (--scene f01_late --f01-room ROOM)
#   PLAN    direct | valve | chain | mastery | dozer:run | dozer:close_call | dozer:wait   (--room-pilot, src/room_pilots.rs; solo only)
#   FRAMES  measured frames = pilot ticks (one sim tick per frame, from tick 0, after a
#           120-frame warm-up that does not step the room)
#
# Examples (the committed numbers are in docs/evidence/f01-playtest/):
#   bash scripts/room_perf.sh boiler chain 950      # the Boiler's full chain
#   bash scripts/room_perf.sh mastery mastery 738   # the mastery arena: FRAMES = the pilot's exit tick (737) + 1
#
# The run is a real Bevy window under xvfb-run at 640x360, 1x, gore preset 4, lighting on,
# AutoNoVsync, no audio. Under xvfb the swapchain is a virtual X display (the frame is
# copied by the X server): the interval is NOT a real-display or handheld frame time. That
# the X copy paces it near 16.6 ms is a hypothesis (a median of 16.2 to 16.6 ms under
# AutoNoVsync), not measured. `gpu_ms_*` (summed timed render passes) and
# `tick_us_*` (the room pilot's simulation tick on the CPU) are reported next to it; together they
# are a LOWER BOUND of the frame cost (main-thread presentation, extract and prepare are not
# measured). Size FRAMES to the pilot's exit tick + 1 so no frame measures a finished, static room. The interval
# is wall-clock: `loadavg_after` is printed so a run on a busy machine is not mistaken for a clean one.
#
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene), the exported atlases
#   (bash scripts/export_assets.sh) and xvfb-run. Prints one ROOM_PERF line (and writes the
#   full log to OUT, default a temporary file whose path is printed).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
room="${1:?room}"
plan="${2:?plan}"
frames="${3:?frames}"
shift 3
bin="${BIN:-./target/debug/floor01_scene}"
log="${OUT:-$(mktemp -t room_perf.XXXXXX.log)}"
xvfb-run -a -s '-screen 0 1280x720x24' "$bin" --scale 1 --no-audio \
  --scene f01_late --f01-room "$room" --room-pilot "$plan" --gore-preset 4 \
  --perf-frames "$frames" --perf-from-tick 0 "$@" > "$log" 2>&1
perf="$(grep '^FLOOR1_PERF ' "$log" | tail -n 1)"
cpu_gpu="$(grep '^FLOOR1_PERF_CPU_GPU ' "$log" | tail -n 1)"
[[ -n "$perf" ]] || { tail -n 20 "$log"; echo "no FLOOR1_PERF line" >&2; exit 1; }
adapter="$(grep -o 'AdapterInfo { name: "[^"]*"[^}]*device_type: [A-Za-z]*' "$log" | head -n 1 \
  | sed -E 's/AdapterInfo \{ name: "([^"]*)".*device_type: ([A-Za-z]*)/\1 (\2)/')"
cpu="$(grep -m1 'model name' /proc/cpuinfo | sed -E 's/.*: //')"
last_tick="$(grep '^FLOOR1_PERF_TICK ' "$log" | tail -n 1 | sed -E 's/.*tick=([0-9]+).*/\1/')"
load="$(cut -d' ' -f1-3 /proc/loadavg)"
echo "ROOM_PERF room=$room plan=$plan last_tick=$last_tick cpu=\"$cpu\" adapter=\"${adapter:-unknown}\" display=xvfb profile=debug loadavg_after=\"$load\""
echo "$perf"
echo "$cpu_gpu"
echo "$(grep '^FLOOR1_PERF_MEM ' "$log" | tail -n 1)"
echo "log=$log"
