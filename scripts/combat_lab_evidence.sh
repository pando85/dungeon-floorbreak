#!/usr/bin/env bash
# COMBAT-LAB-01 LAB-01/LAB-00-wire evidence: scripted captures of R00 to R02 in
# solo Carl, solo Donut (spawn only) and co-op at 640x360. Run inside xvfb
# from the repository root after `cargo build --bin floor01_scene`:
#   xvfb-run -a -s '-screen 0 1280x800x24' bash scripts/combat_lab_evidence.sh
# The binary renders at --scale 2 (a scale-1 window renders black under xvfb
# on this setup), then each frame is reduced 2:1 with a point filter, which is
# exactly the 640x360 virtual view.
set -euo pipefail
out="${1:-docs/evidence/combat-lab-01}"
raw="$(mktemp -d)"
trap 'rm -rf "$raw"' EXIT
mkdir -p "$out/frames" "$out/sheets"
bin=./target/debug/floor01_scene

# name | extra args | first:step:count (ticks of the scripted autopilot run)
runs=(
  "r00_solo_carl|--lab-room R00 --lab-confirmed --lab-autopilot|5:40:5"
  "r00_coop|--lab-room R00 --lab-confirmed --lab-mode coop_local --lab-autopilot|5:40:5"
  "r00_menu|--lab-room R00|5:1:1"
  "r01_solo_carl|--lab-room R01 --lab-autopilot|5:55:6"
  "r01_coop|--lab-room R01 --lab-mode coop_local --lab-autopilot|5:55:6"
  "r01_solo_donut|--lab-room R01 --lab-mode solo_donut|5:1:1"
  "r02_solo_carl|--lab-room R02 --lab-autopilot|5:140:9"
  "r02_coop|--lab-room R02 --lab-mode coop_local --lab-autopilot|5:140:9"
  "r02_solo_donut|--lab-room R02 --lab-mode solo_donut|5:1:1"
)
for entry in "${runs[@]}"; do
  IFS='|' read -r name args seq <<<"$entry"
  echo "capture $name"
  # shellcheck disable=SC2086
  "$bin" --scene combat_lab $args --scale 2 --no-audio \
    --capture-sequence "$seq" --capture-dir "$raw/$name" >"$raw/$name.log" 2>&1
  i=0
  for f in "$raw/$name"/frame_*.png; do
    tick="$(sed -n 's/.*_tick_0*\([0-9]*\)\.png/\1/p' <<<"$f")"
    convert "$f" -filter point -resize 50% "$out/frames/${name}_t$(printf '%05d' "$tick").png"
    i=$((i + 1))
  done
  grep -E '^LAB_ENTRY|^COMBAT_STATE' "$raw/$name.log" | tail -n 2 >"$out/frames/${name}.state.txt"
  # One contact sheet per run (frames left to right, 2 columns).
  montage "$out/frames/${name}_t"*.png -tile 2x -geometry +4+4 -background '#101018' "$out/sheets/${name}.png"
done
echo "done: $out"
