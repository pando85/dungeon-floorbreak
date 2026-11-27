#!/usr/bin/env bash
# AREA-POLISH-01: the real Bevy evidence of the mastery arena, the Dozer's room shell, the corral's
# grate and partition burst and the Juicer's exit lintel (`--scene f01_late --f01-room ROOM
# --room-pilot PLAN`, the PT-GATE and EVID-ROOM pilots).
#
# `scripts/room_capture.sh ROOM PLAN FIRST:STEP:COUNT` renders the real window under xvfb at 640x360 and
# the LAST frame of each sequence is the still (the camera eases per frame, so a still is defined by its
# whole sequence). Every still is captured TWICE (two full passes, the PNGs must be byte-identical) and
# each capture checks that the window's final COMBAT_STATE hash equals the headless run's: the dressing,
# the grate art and the burst are presentation only. Output: PNGs, capture_lines.txt (one POLISH_STILL
# line per still) and SHA256SUMS.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh), xvfb-run, flock and ImageMagick.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)   OUT=<evidence dir>
#   CAPTURES=<scratch dir, under artifacts/>
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/area-polish-01}"
caps="${CAPTURES:-artifacts/polish01-evidence}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/f01-dfx-gate.png && -f generated/art/floor01_splinter_burst.png ]] \
  || { echo "the atlases are missing: run scripts/export_assets.sh" >&2; exit 2; }
rm -rf "$caps"
mkdir -p "$caps" "$out"
# name:room:plan:first:step:count
stills=(
  mastery_spawn:mastery:mastery:60:30:3
  mastery_arcade:mastery:mastery:330:30:3
  mastery_exit:mastery:mastery:900:30:3
  dozer_bricked_bay:dozer:dozer:run:60:30:3
  dozer_chase:dozer:dozer:run:480:30:3
  dozer_tunnel:dozer:dozer:run:780:30:3
  corral_grate_down:corral:corral:930:30:2
  corral_grate_open:corral:corral:1000:20:2
  corral_partition_standing:corral:corral_partition:600:20:2
  corral_partition_burst_a:corral:corral_partition:660:4:6
  corral_partition_burst_b:corral:corral_partition:660:4:7
  juicer_exit_closed:juicer:juicer:60:1:1
  juicer_exit_open:juicer:juicer:3230:1:1
)
state() { grep "^COMBAT_STATE " "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
capture() {
  local room="$1" plan="$2" seq="$3" dir="$4"
  for attempt in 1 2 3; do
    rm -rf "$dir"
    if BIN="$bin" scripts/room_capture.sh "$room" "$plan" "$seq" "$dir" > "$dir.log" 2>&1; then
      break
    fi
    [[ "$attempt" -lt 3 ]] || { echo "capture $room $plan $seq failed 3 times: see $dir.log" >&2; exit 1; }
  done
  find "$dir" -name 'frame_*.png' | sort | tail -n 1
}
split_still() { # name:room:plan[:run]:first:step:count -> sets name room plan first step count
  local IFS=:
  read -ra f <<<"$1"
  name="${f[0]}"; room="${f[1]}"
  if [[ "${#f[@]}" == 7 ]]; then plan="${f[2]}:${f[3]}"; first="${f[4]}"; step="${f[5]}"; count="${f[6]}"
  else plan="${f[2]}"; first="${f[3]}"; step="${f[4]}"; count="${f[5]}"; fi
}
for pass in 1 2; do
  for still in "${stills[@]}"; do
    split_still "$still"
    frame="$(capture "$room" "$plan" "$first:$step:$count" "$caps/pass$pass-$name")"
    [[ "$(identify -format '%wx%h' "$frame")" == 640x360 ]] || { echo "bad size: $frame" >&2; exit 1; }
    cp "$frame" "$caps/pass$pass-$name.png"
  done
done
rm -f "$out"/scene_*.png "$out"/capture_lines.txt "$out"/SHA256SUMS
fail=0
for still in "${stills[@]}"; do
  split_still "$still"
  tick=$((first + step * (count - 1)))
  a="$caps/pass1-$name.png" b="$caps/pass2-$name.png"
  same=no; cmp -s "$a" "$b" && same=yes
  window="$(state "$caps/pass1-$name/run.log")"
  headless="$(state "$caps/pass1-$name/headless.log")"
  match=no; [[ -n "$window" && "$window" == "$headless" ]] && match=yes
  [[ "$same" == yes && "$match" == yes ]] || fail=1
  cp "$a" "$out/scene_${name}_640x360.png"
  echo "POLISH_STILL name=$name room=$room plan=$plan sequence=$first:$step:$count tick=$tick size=640x360 window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
done
(cd "$out" && find . -type f \( -name '*.png' -o -name '*.txt' \) | sort | sed 's#^\./##' | xargs sha256sum > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
