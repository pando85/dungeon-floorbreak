#!/usr/bin/env bash
# AREA-CAMP-01: the real Bevy evidence of the survivors' camp (`rest_camp`) and the quiet aftermath (`rest_aftermath`).
#
# Each still is the REST-WIRE capture command (`--scene f01_late --f01-room ROOM --rest-pilot PILOT --companion off
# --capture-sequence TICK:1:1`) rendered in a real window under xvfb at 640x360 (`--scale 1`) and 1920x1080
# (`--scale 3`). Every still is captured TWICE (two full passes) and the two PNGs must be byte-identical; each capture also
# checks that the window's final COMBAT_STATE hash equals the headless run's of the same tick (the art is presentation
# only) and, where REST-WIRE captured the same room, pilot, tick and size (docs/evidence/rest-wire/capture_lines_bc.txt),
# that it equals the REST-WIRE hash too. The camera eases per frame, so a still is defined by its room, pilot and tick;
# the drawn view (left) is measured from landmarks afterwards (scripts/make_camp01_sheets.py), never taken from the
# logged `cam`.
#
# Output: the PNGs `scene_<name>_<size>.png`, capture_lines.txt (one CAMP_STILL line per still and size), the offline
# sheets and measurements of scripts/make_camp01_sheets.py (when CAMP_LAYOUT_DIR holds the layouts written by the
# ignored test `camp_kit::tests::dump_camp_layouts`), and SHA256SUMS.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases (bash scripts/export_assets.sh),
# xvfb-run, flock, ImageMagick and Pillow.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   OUT=<evidence dir> (default docs/evidence/area-camp-01)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
#   CAMP_LAYOUT_DIR=<dir written by the ignored test camp_kit::tests::dump_camp_layouts> (optional)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
out="${OUT:-docs/evidence/area-camp-01}"
caps="${CAPTURES:-artifacts/camp01-evidence}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/floor01_camp_hearth.png ]] || { echo "the floor01_camp_* atlases are missing: run scripts/export_assets.sh" >&2; exit 2; }
lock="${ROOM_CAPTURE_LOCK:-${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}/dungeon-floorbreak-room-capture-$(id -u).lock}"
if [[ "$lock" != off ]]; then
  exec 9>"$lock"
  flock -x 9
fi
rm -rf "$caps"
mkdir -p "$caps" "$out"
# name:room:pilot:tick. `walk` is Carl walking right to the exit (REST-WIRE); `punch` stands Carl at the heavy bag and
# punches it every 40 ticks (the bag's hit sway, the first jab is active on ticks 164 and 165).
stills=(
  camp_entrance:rest_camp:walk:41
  camp_sentry:rest_camp:walk:161
  camp_figures:rest_camp:walk:241
  camp_hearth:rest_camp:walk:281
  camp_exit:rest_camp:walk:361
  aft_entry_swing:rest_aftermath:walk:21
  aft_egress:rest_aftermath:walk:41
  aft_wreck:rest_aftermath:walk:161
  aft_lane:rest_aftermath:walk:281
  aft_grooves:rest_aftermath:walk:349
  aft_punch_rest:rest_aftermath:punch:163
  aft_punch_r1:rest_aftermath:punch:166
  aft_punch_r2:rest_aftermath:punch:170
  aft_punch_r1b:rest_aftermath:punch:176
  aft_punch_l1:rest_aftermath:punch:184
  aft_punch_late:rest_aftermath:punch:194
)
# res:scale. The punch strip is captured at 1x only (it is cropped to the bag).
sizes=(640x360:1 1920x1080:3)
state() { grep "^COMBAT_STATE " "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
rest_wire_hash() { # room pilot tick size -> the REST-WIRE capture's window hash, or empty
  grep -hE "^REST_STILL .* room=$1 pilot=$2 tick=$3 size=$4 " docs/evidence/rest-wire/capture_lines_bc.txt docs/evidence/rest-wire/capture_lines.txt 2>/dev/null \
    | head -n 1 | sed -E 's/.* window_hash=([0-9a-f]+) .*/\1/' || true
}
# capture ROOM PILOT TICK RES SCALE DIR: one window frame at TICK (the last frame of the sequence TICK:1:1)
capture() {
  local room="$1" pilot="$2" tick="$3" res="$4" scale="$5" dir="$6"
  for attempt in 1 2 3; do
    rm -rf "$dir"; mkdir -p "$dir"
    xvfb-run -a -s '-screen 0 1920x1080x24' "$bin" --no-audio --no-record --scale "$scale" \
      --scene f01_late --f01-room "$room" --rest-pilot "$pilot" --companion off \
      --capture-sequence "$tick:1:1" --capture-dir "$dir" > "$dir.log" 2>&1 || true
    grep -q '^FLOOR1_CAPTURE_SEQUENCE_DONE' "$dir.log" && break
    [[ "$attempt" -lt 3 ]] || { echo "capture $room $pilot $tick at $res failed 3 times: see $dir.log" >&2; exit 1; }
  done
  "$bin" --headless-floor01-smoke --scene f01_late --f01-room "$room" --rest-pilot "$pilot" --companion off \
    --capture-tick "$tick" > "$dir.headless.log" 2>&1
  local frame
  frame="$(find "$dir" -name 'frame_*.png' | sort | tail -n 1)"
  [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size for $res: $frame" >&2; exit 1; }
  echo "$frame"
}
wanted() { # still name, size -> whether the still is captured at this size (the punch strip is 1x only)
  [[ "$2" == 640x360 || "$1" != aft_punch_* || "$1" == aft_punch_r2 ]]
}
for pass in 1 2; do
  for size in "${sizes[@]}"; do
    IFS=: read -r res scale <<<"$size"
    for still in "${stills[@]}"; do
      IFS=: read -r name room pilot tick <<<"$still"
      wanted "$name" "$res" || continue
      frame="$(capture "$room" "$pilot" "$tick" "$res" "$scale" "$caps/pass$pass-$res-$name")"
      cp "$frame" "$caps/pass$pass-${name}_$res.png"
    done
  done
done
rm -f "$out"/scene_*.png "$out"/capture_lines.txt
fail=0
for size in "${sizes[@]}"; do
  IFS=: read -r res scale <<<"$size"
  for still in "${stills[@]}"; do
    IFS=: read -r name room pilot tick <<<"$still"
    wanted "$name" "$res" || continue
    a="$caps/pass1-${name}_$res.png" b="$caps/pass2-${name}_$res.png"
    same=no; cmp -s "$a" "$b" && same=yes
    window="$(state "$caps/pass1-$res-$name.log")"
    headless="$(state "$caps/pass1-$res-$name.headless.log")"
    match=no; [[ -n "$window" && "$window" == "$headless" ]] && match=yes
    rw="$(rest_wire_hash "$room" "$pilot" "$tick" "$res")"
    rwmatch=n/a
    if [[ -n "$rw" ]]; then rwmatch=no; [[ "$rw" == "$window" ]] && rwmatch=yes; fi
    [[ "$same" == yes && "$match" == yes && "$rwmatch" != no ]] || fail=1
    carl="$(grep '^FLOOR1_SCENE_STATE' "$caps/pass1-$res-$name.log" | tail -n 1 | sed -E 's/.* hero=([0-9.-]+),.*/\1/')"
    cp "$a" "$out/scene_${name}_$res.png"
    echo "CAMP_STILL name=$name room=$room pilot=$pilot tick=$tick size=$res carl_x=$carl window_hash=$window headless_hash=$headless hash_match=$match rest_wire_hash=${rw:-none} rest_wire_match=$rwmatch passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
  done
done
python3 scripts/make_camp01_sheets.py "$out" "${CAMP_LAYOUT_DIR:-}"
(cd "$out" && find . -type f \( -name '*.png' -o -name '*.txt' \) | sort | sed 's#^\./##' | grep -v '^SHA256SUMS$' | xargs sha256sum > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
