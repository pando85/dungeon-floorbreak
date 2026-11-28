#!/usr/bin/env bash
# RING-C wire: the real Bevy evidence of the moving shutter of room 4a ring_court.
#
# `--scene f01_late --f01-run --f01-room ring_court --ring-pilot` plays the court's library pilot (the step, the
# walkway, the fight, the exit) and renders the real window under xvfb at 640x360 (`--scale 1`) at the ticks below:
# the warning, the closing edge, the closed shutter, the opening edge and the open lane, plus the RING-B static gate
# (`--ring-static-gate`) at the closed tick for comparison. Every capture runs TWICE (two full passes) and the PNGs of
# the two passes must be byte-identical; each capture also checks that the window's final COMBAT_STATE hash equals the
# headless run's (the presenter is presentation only). Output: PNGs, capture_lines.txt (one RINGC_STILL line per still)
# and SHA256SUMS. The pin moment has no window still: no scripted window run places a goblin in the arch; its
# deterministic trace is `pin_moment.txt` (`cargo test ring_c_pin_trace -- --ignored --nocapture`).
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases (bash scripts/export_assets.sh),
# xvfb-run, flock and ImageMagick.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/ring-c-wire)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/ring-c-wire}"
caps="${CAPTURES:-artifacts/ring-c-wire-evidence}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/floor01_ring_wall.png ]] || { echo "the floor01_ring_* atlases are missing: run scripts/export_assets.sh" >&2; exit 2; }
lock="${ROOM_CAPTURE_LOCK:-${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}/dungeon-floorbreak-room-capture-$(id -u).lock}"
if [[ "$lock" != off ]]; then
  exec 9>"$lock"
  flock -x 9
fi
rm -rf "$caps"
mkdir -p "$caps" "$out"
xvfb() { xvfb-run -a -s '-screen 0 1920x1080x24' "$@" || true; }
# name:tick:extra-flag. The pilot arms the warning at tick ~291 (Carl crosses arm_x 1008), the edge moves from ~332 to
# ~365 (34 ticks), the shutter stays closed until ~515, opens by ~548 and stays open (cycles 1).
stills=(warning:310: closing:348: closed:420: opening:532: open:600: static_gate_closed:420:--ring-static-gate)
state() { grep "^COMBAT_STATE " "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
for pass in 1 2; do
  for still in "${stills[@]}"; do
    IFS=: read -r name tick extra <<<"$still"
    dir="$caps/pass$pass-$name"
    for attempt in 1 2 3; do
      rm -rf "$dir"; mkdir -p "$dir"
      xvfb "$bin" --no-audio --no-record --scale 1 --scene f01_late --f01-run --f01-room ring_court --ring-pilot \
        ${extra:+$extra} --capture-sequence "$tick:1:1" --capture-dir "$dir" > "$dir.log" 2>&1
      grep -q '^FLOOR1_CAPTURE_SEQUENCE_DONE' "$dir.log" && break
      [[ "$attempt" -lt 3 ]] || { echo "capture $name failed 3 times: see $dir.log" >&2; exit 1; }
    done
    frame="$(find "$dir" -name 'frame_*.png')"
    [[ "$(identify -format '%wx%h' "$frame")" == "640x360" ]] || { echo "bad size: $frame" >&2; exit 1; }
    cp "$frame" "$caps/pass$pass-$name.png"
  done
done
rm -f "$out"/scene_*.png "$out"/capture_lines.txt "$out"/SHA256SUMS
fail=0
for still in "${stills[@]}"; do
  IFS=: read -r name tick extra <<<"$still"
  a="$caps/pass1-$name.png" b="$caps/pass2-$name.png"
  same=no
  cmp -s "$a" "$b" && same=yes
  [[ "$same" == yes ]] || fail=1
  window="$(state "$caps/pass1-$name.log")"
  "$bin" --headless-floor01-smoke --scene f01_late --f01-run --f01-room ring_court --ring-pilot ${extra:+$extra} \
    --capture-tick "$tick" > "$caps/headless-$name.log" 2>&1
  headless="$(state "$caps/headless-$name.log")"
  env_token="$(grep '^COMBAT_STATE ' "$caps/headless-$name.log" | tail -n 1 | sed -E 's/.* (env=[^ ]*).*/\1/')"
  match=no
  [[ -n "$window" && "$window" == "$headless" ]] && match=yes
  [[ "$match" == yes ]] || fail=1
  cp "$a" "$out/scene_${name}_640x360.png"
  echo "RINGC_STILL name=$name tick=$tick size=640x360 $env_token window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
done
(cd "$out" && find . -type f \( -name '*.png' -o -name '*.txt' \) | sort | sed 's#^\./##' | xargs sha256sum > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
