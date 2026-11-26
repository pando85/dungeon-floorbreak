#!/usr/bin/env bash
# AREA-FASTFOOD-01: the real Bevy evidence of the fast-food refuge (Floor 01 room 1, `rest_refuge`).
#
# Each still is the REST-WIRE capture command (`--scene f01_late --f01-room rest_refuge --rest-pilot P --companion off
# --capture-sequence TICK:1:1`) rendered in a real window under xvfb, at 640x360 (`--scale 1`), 1920x1080 (`--scale 3`)
# and the 640x400 deck view (`--deck --scale 1`). Every still is captured TWICE (two full passes) and the two PNGs must
# be byte-identical; each capture also checks that the window's final COMBAT_STATE hash equals the headless run's of
# the same tick (the art is presentation only), and, where REST-WIRE captured the same pilot, tick and size
# (docs/evidence/rest-wire/capture_lines.txt), that it equals the REST-WIRE hash too. The camera eases per frame, so a
# still is defined by its pilot and tick; the drawn view (left, top) is measured from landmarks afterwards
# (scripts/ff01_view.py), never taken from the logged `cam`.
#
# Output: the PNGs `scene_<name>_<size>.png`, capture_lines.txt (one FF_STILL line per still and size), the offline
# sheets and measurements of scripts/make_ff01_sheets.py (when FF_LAYOUT_DIR holds the layouts written by the ignored
# test `fastfood_kit::tests::dump_ff_layouts`), the regenerated gate sheets, and SHA256SUMS.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases (bash scripts/export_assets.sh),
# xvfb-run, flock, ImageMagick and Pillow.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/area-fastfood-01)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
#   FF_LAYOUT_DIR=<dir written by the ignored test fastfood_kit::tests::dump_ff_layouts> (optional)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/area-fastfood-01}"
caps="${CAPTURES:-artifacts/ff01-evidence}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/floor01_ff_door.png ]] || { echo "the floor01_ff_* atlases are missing: run scripts/export_assets.sh" >&2; exit 2; }
# one window at a time (the room_capture.sh lock)
lock="${ROOM_CAPTURE_LOCK:-${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}/dungeon-floorbreak-room-capture-$(id -u).lock}"
if [[ "$lock" != off ]]; then
  exec 9>"$lock"
  flock -x 9
fi
rm -rf "$caps"
mkdir -p "$caps" "$out"
# name:pilot:tick (the REST-WIRE pilots: donut, carl_door, base_jump)
stills=(
  threshold:donut:20
  play_area:donut:120
  window:donut:270
  box_full:donut:312
  box_grant:donut:332
  door_closed:carl_door:560
  door_rising_371:donut:371
  door_rising_373:donut:373
  door_rising_376:donut:376
  door_open:donut:392
  exit:donut:408
)
# res:scale:deck-flag
sizes=(640x360:1: 1920x1080:3: 640x400:1:--deck)
state() { grep "^COMBAT_STATE " "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
rest_wire_hash() { # pilot tick size -> the REST-WIRE capture's window hash, or empty
  grep -E "^REST_STILL .* pilot=$1 tick=$2 size=$3 " docs/evidence/rest-wire/capture_lines.txt | head -n 1 \
    | sed -E 's/.* window_hash=([0-9a-f]+) .*/\1/' || true
}
# capture NAME PILOT TICK RES SCALE DECK DIR: one window frame at TICK (the last frame of the sequence TICK:1:1)
capture() {
  local pilot="$1" tick="$2" res="$3" scale="$4" deck="$5" dir="$6"
  for attempt in 1 2 3; do
    rm -rf "$dir"; mkdir -p "$dir"
    xvfb-run -a -s '-screen 0 1920x1080x24' "$bin" --no-audio --no-record --scale "$scale" ${deck:+$deck} \
      --scene f01_late --f01-room rest_refuge --rest-pilot "$pilot" --companion off \
      --capture-sequence "$tick:1:1" --capture-dir "$dir" > "$dir.log" 2>&1 || true
    grep -q '^FLOOR1_CAPTURE_SEQUENCE_DONE' "$dir.log" && break
    [[ "$attempt" -lt 3 ]] || { echo "capture $pilot $tick at $res failed 3 times: see $dir.log" >&2; exit 1; }
  done
  "$bin" --headless-floor01-smoke ${deck:+$deck} --scene f01_late --f01-room rest_refuge --rest-pilot "$pilot" --companion off \
    --capture-tick "$tick" > "$dir.headless.log" 2>&1
  local frame
  frame="$(find "$dir" -name 'frame_*.png' | sort | tail -n 1)"
  [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size for $res: $frame" >&2; exit 1; }
  echo "$frame"
}
for pass in 1 2; do
  for size in "${sizes[@]}"; do
    IFS=: read -r res scale deck <<<"$size"
    for still in "${stills[@]}"; do
      IFS=: read -r name pilot tick <<<"$still"
      frame="$(capture "$pilot" "$tick" "$res" "$scale" "$deck" "$caps/pass$pass-$res-$name")"
      cp "$frame" "$caps/pass$pass-${name}_$res.png"
    done
  done
done
rm -f "$out"/scene_*.png "$out"/capture_lines.txt
fail=0
for size in "${sizes[@]}"; do
  IFS=: read -r res scale deck <<<"$size"
  for still in "${stills[@]}"; do
    IFS=: read -r name pilot tick <<<"$still"
    a="$caps/pass1-${name}_$res.png" b="$caps/pass2-${name}_$res.png"
    same=no; cmp -s "$a" "$b" && same=yes
    window="$(state "$caps/pass1-$res-$name.log")"
    headless="$(state "$caps/pass1-$res-$name.headless.log")"
    match=no; [[ -n "$window" && "$window" == "$headless" ]] && match=yes
    rw="$(rest_wire_hash "$pilot" "$tick" "$res")"
    rwmatch=n/a
    if [[ -n "$rw" ]]; then rwmatch=no; [[ "$rw" == "$window" ]] && rwmatch=yes; fi
    [[ "$same" == yes && "$match" == yes && "$rwmatch" != no ]] || fail=1
    cp "$a" "$out/scene_${name}_$res.png"
    echo "FF_STILL name=$name pilot=$pilot tick=$tick size=$res window_hash=$window headless_hash=$headless hash_match=$match rest_wire_hash=${rw:-none} rest_wire_match=$rwmatch passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
  done
done
python3 scripts/author_ff01.py gate "$out/gate" > /dev/null 2>&1
python3 scripts/make_ff01_sheets.py "$out" "${FF_LAYOUT_DIR:-}"
(cd "$out" && find . -type f \( -name '*.png' -o -name '*.txt' \) | sort | sed 's#^\./##' | xargs sha256sum > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
