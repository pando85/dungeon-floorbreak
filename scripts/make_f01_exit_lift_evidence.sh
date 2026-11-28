#!/usr/bin/env bash
# EXIT-LIFT-01: the in-scene evidence of the room exit grate's lift (spec: docs/02-art/floor01-room-exit-spec.md
# section 8). The aligned room pilots play room 6 (`mastery:carl_solo:0`, the last roster enemy falls on tick 672, the
# grate lifts over ticks 672..688) and room 0, the corral (`corral:carl_solo:0`, the second M2 llama falls on tick 1006,
# the M3 gate lifts over ticks 1006..1022). Each still is the lift's mid-point (age 8, a 40 px rise of 80) of the real
# Bevy window under xvfb at 640x360 (`--scale 1`), captured TWICE (two full passes) and byte-identical, with the
# window's final COMBAT_STATE hash equal to the headless replay's (the presentation is read-only).
# Output in $OUT: the PNGs, capture_lines.txt and SHA256SUMS.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh: generated/art/f01-dfx-exit.png), xvfb-run, flock and ImageMagick.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/exit-lift-01)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export BIN="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/exit-lift-01}"
caps="${CAPTURES:-artifacts/exit-lift}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/f01-dfx-exit.png ]] || { echo "generated/art/f01-dfx-exit.png is missing: run scripts/export_assets.sh" >&2; exit 2; }
rm -rf "$caps"
mkdir -p "$caps" "$out"
# room:plan:name:tick:opening_tick
stills=(
  mastery:mastery:carl_solo:0:room6_lift_mid:680:672
  corral:corral:carl_solo:0:room0_corral_lift_mid:1014:1006
)
for pass in 1 2; do
  for still in "${stills[@]}"; do
    IFS=: read -r room p1 p2 p3 name tick open <<<"$still"
    plan="$p1:$p2:$p3"
    dir="$caps/pass$pass-$name"
    for attempt in 1 2 3; do
      bash scripts/room_capture.sh "$room" "$plan" "$tick:1:1" "$dir" > "$dir.txt" 2> "$dir.err" && break
      [[ "$attempt" -lt 3 ]] || { echo "capture $dir failed 3 times: see $dir/run.log" >&2; exit 1; }
    done
    frame="$(find "$dir" -name 'frame_*.png')"
    [[ "$(identify -format '%wx%h' "$frame")" == "640x360" ]] || { echo "bad size: $frame" >&2; exit 1; }
    grep -q '^ROOM_EXIT drawn' "$dir/run.log" || { echo "the exit door was not drawn: $dir/run.log" >&2; exit 1; }
    cp "$frame" "$caps/pass$pass-$name.png"
  done
done
rm -f "$out"/*.png "$out"/capture_lines.txt "$out"/SHA256SUMS
fail=0
for still in "${stills[@]}"; do
  IFS=: read -r room p1 p2 p3 name tick open <<<"$still"
  plan="$p1:$p2:$p3"
  a="$caps/pass1-$name.png" b="$caps/pass2-$name.png"
  same=no
  cmp -s "$a" "$b" && same=yes
  window="$(sed -E 's/.* window_hash=([0-9a-f]+).*/\1/' "$caps/pass1-$name.txt")"
  headless="$(sed -E 's/.* headless_hash=([0-9a-f]+).*/\1/' "$caps/pass1-$name.txt")"
  match=no
  [[ -n "$window" && "$window" == "$headless" ]] && match=yes
  [[ "$same" == yes && "$match" == yes ]] || fail=1
  cp "$a" "$out/${name}_640x360.png"
  echo "EXIT_LIFT_STILL room=$room plan=$plan name=$name tick=$tick opening_tick=$open age=$((tick - open)) size=640x360 window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
done
(cd "$out" && sha256sum ./*.png ./capture_lines.txt | sed 's# \./# #' > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
