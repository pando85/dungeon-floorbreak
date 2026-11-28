#!/usr/bin/env bash
# TROGB-AD1 (TROG-Q14): the real Bevy window evidence of the bench smash in the Test of Strength.
#
# The pilot is `--trog-pilot bait` (Carl defeats the Virtuoso, stands in the bait zone at x 616 and
# lets the Basher smash the bench, then fights it) under `--companion off`, rendered in a real window
# under xvfb at 640x360 (`--scale 1`). Every still is captured TWICE (two full passes) and the two PNGs
# must be byte-identical; each capture also checks that the window's final COMBAT_STATE hash equals the
# headless run's of the same tick (the art is presentation only).
#
# Output: scene_<name>_640x360.png, capture_lines.txt (one TROG_BENCH_STILL line per still) and SHA256SUMS.
#
# Needs the debug binary, the exported atlases (bash scripts/export_assets.sh), xvfb-run, flock and ImageMagick.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/trog-bench-smash)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/trog-bench-smash}"
caps="${CAPTURES:-artifacts/trog-bench-evidence}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/floor01_trog_bench.png ]] || { echo "the floor01_trog_bench atlas is missing: run scripts/export_assets.sh" >&2; exit 2; }
lock="${ROOM_CAPTURE_LOCK:-${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}/dungeon-floorbreak-room-capture-$(id -u).lock}"
if [[ "$lock" != off ]]; then
  exec 9>"$lock"
  flock -x 9
fi
rm -rf "$caps"
mkdir -p "$caps" "$out"
# name:tick. The pilot's log (headless): the Virtuoso falls at 168, the Basher wakes at 238, tells at the
# bench at 391, launches at 427 and the smash breaks the bench at 430.
#   bench_intact   the bench stack standing, Carl walking to the bait column
#   bench_bait     Carl at the bait zone, the Basher in its tell at the bench
#   bench_burst    two ticks after the break: the 6 frame debris burst mid-flight
#   bench_broken   the remnant and the settled litter, the Basher in its recovery
stills=(
  bench_intact:200
  bench_bait:400
  bench_burst:432
  bench_broken:480
)
res=640x360
state() { grep "^COMBAT_STATE " "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
capture() { # tick dir -> the window frame at tick
  local tick="$1" dir="$2"
  for attempt in 1 2 3; do
    rm -rf "$dir"; mkdir -p "$dir"
    xvfb-run -a -s '-screen 0 1920x1080x24' "$bin" --no-audio --no-record --scale 1 \
      --scene f01_late --f01-room trog_strength --trog-pilot bait --companion off \
      --capture-sequence "$tick:1:1" --capture-dir "$dir" > "$dir.log" 2>&1 || true
    grep -q '^FLOOR1_CAPTURE_SEQUENCE_DONE' "$dir.log" && break
    [[ "$attempt" -lt 3 ]] || { echo "capture $tick failed 3 times: see $dir.log" >&2; exit 1; }
  done
  "$bin" --headless-floor01-smoke --scene f01_late --f01-room trog_strength --trog-pilot bait --companion off \
    --capture-tick "$tick" > "$dir.headless.log" 2>&1
  local frame
  frame="$(find "$dir" -name 'frame_*.png' | sort | tail -n 1)"
  [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size: $frame" >&2; exit 1; }
  echo "$frame"
}
for pass in 1 2; do
  for still in "${stills[@]}"; do
    IFS=: read -r name tick <<<"$still"
    frame="$(capture "$tick" "$caps/pass$pass-$name")"
    cp "$frame" "$caps/pass$pass-$name.png"
  done
done
rm -f "$out"/scene_*.png "$out"/capture_lines.txt
fail=0
for still in "${stills[@]}"; do
  IFS=: read -r name tick <<<"$still"
  a="$caps/pass1-$name.png" b="$caps/pass2-$name.png"
  same=no; cmp -s "$a" "$b" && same=yes
  window="$(state "$caps/pass1-$name.log")"
  headless="$(state "$caps/pass1-$name.headless.log")"
  match=no; [[ -n "$window" && "$window" == "$headless" ]] && match=yes
  bench="$(grep '^COMBAT_STATE' "$caps/pass1-$name.headless.log" | tail -n 1 | sed -E 's/.* env=([^ ]+).*/\1/')"
  [[ "$same" == yes && "$match" == yes ]] || fail=1
  cp "$a" "$out/scene_${name}_$res.png"
  echo "TROG_BENCH_STILL name=$name room=trog_strength pilot=bait tick=$tick size=$res bench=$bench window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
done
(cd "$out" && find . -type f \( -name '*.png' -o -name '*.txt' \) -print | sort | sed 's#^\./##' | grep -v '^SHA256SUMS$' | xargs sha256sum > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
