#!/usr/bin/env bash
# AREA-HEAT-01: the real Bevy evidence of the scorched corral (Floor 01 room 0, `--scene f01_late --f01-run`).
#
# `scripts/room_capture.sh corral corral FIRST:STEP:COUNT` plays the corral's library pilot and renders the real
# window under xvfb; the LAST frame of each sequence is the still. It runs at 640x360 (`--scale 1`), 1920x1080
# (`--scale 3`) and the 640x400 deck view (`--deck --scale 1`). The camera eases per frame, so a still is
# defined by its whole sequence (first, step, count), not by its tick alone. Every capture runs TWICE (two
# full passes) and the PNGs of the two passes must be byte-identical; each capture also checks that the window's
# final COMBAT_STATE hash equals the headless run's (the dressing is presentation only). A bare
# `--scene f01_late --f01-run` (no `--f01-room`, no pilot: Carl walks right) is captured too, as the proof that
# the dressing is visible by default. With BASE_BIN (a floor01_scene built from origin/main, the undressed room)
# the pen entry and the dock are captured again for the before/after pairs.
#
# Output: PNGs, capture_lines.txt (one HEAT_STILL line per still and size), the offline sheets of
# scripts/make_heat01_sheets.py, the measurements of scripts/measure_heat01.py (if HEAT_LAYOUT_DIR holds the layouts)
# and SHA256SUMS. The partition cannot appear in any runtime still before CORRAL-D (HEAT-AD1): its strips are
# gate composites, labelled so.
#
# Needs the debug binary (cargo build --locked --bin floor01_scene), the exported atlases
# (bash scripts/export_assets.sh), xvfb-run, flock, ImageMagick and Pillow.
#   BIN=<floor01_scene> (default ./target/debug/floor01_scene)
#   BASE_BIN=<floor01_scene built from origin/main> (optional: the before captures)
#   OUT=<evidence dir> (default docs/evidence/area-heat-01)
#   CAPTURES=<scratch dir, under artifacts/> (deleted first)
#   HEAT_LAYOUT_DIR=<dir written by `heat_kit::tests::dump_heat_layouts`> (optional)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
bin="${BIN:-./target/debug/floor01_scene}"
base_bin="${BASE_BIN:-}"
out="${OUT:-docs/evidence/area-heat-01}"
caps="${CAPTURES:-artifacts/heat01-evidence}"
case "$caps" in
  artifacts/?*) ;;
  *) echo "refusing scratch dir '$caps': it must be a path under artifacts/" >&2; exit 2 ;;
esac
[[ -f generated/art/floor01_heat_wall.png ]] || { echo "the floor01_heat_* atlases are missing: run scripts/export_assets.sh" >&2; exit 2; }
rm -rf "$caps"
mkdir -p "$caps" "$out"
# name:first:step:count. The still is the last frame (tick = first + step * (count - 1)); the drawn view of each
# (measured from landmarks by scripts/heat01_view.py): m0 x 0, m1 x 478, pen_entry x 1038, pen_rails x 1190,
# dock x 1610, m3_exit x 1680.
stills=(m0_goblins:30:30:3 m1_throat:330:30:2 pen_entry:420:30:4 pen_rails:600:30:3 dock:780:30:3 m3_exit:930:30:2)
befores=(pen_entry:420:30:4 dock:780:30:3)
# res:scale:deck-flag
sizes=(640x360:1: 1920x1080:3: 640x400:1:--deck)
state() { grep "^COMBAT_STATE " "$1" | tail -n 1 | sed -E 's/.* hash=([0-9a-f]+).*/\1/'; }
# capture BINARY NAME FIRST STEP COUNT RES SCALE DECK DIR: the last frame of a room_capture.sh run
capture() {
  local binary="$1" name="$2" seq="$3" res="$4" scale="$5" deck="$6" dir="$7"
  for attempt in 1 2 3; do
    rm -rf "$dir"
    if BIN="$binary" scripts/room_capture.sh corral corral "$seq" "$dir" --scale "$scale" ${deck:+$deck} > "$dir.log" 2>&1; then
      break
    fi
    [[ "$attempt" -lt 3 ]] || { echo "capture $name at $res failed 3 times: see $dir.log" >&2; exit 1; }
  done
  # room_capture.sh passes its own --scale 1 first; the later flag wins
  local frame
  frame="$(find "$dir" -name 'frame_*.png' | sort | tail -n 1)"
  [[ "$(identify -format '%wx%h' "$frame")" == "$res" ]] || { echo "bad size for $res: $frame" >&2; exit 1; }
  echo "$frame"
}
for pass in 1 2; do
  for size in "${sizes[@]}"; do
    IFS=: read -r res scale deck <<<"$size"
    for still in "${stills[@]}"; do
      IFS=: read -r name first step count <<<"$still"
      frame="$(capture "$bin" "$name" "$first:$step:$count" "$res" "$scale" "$deck" "$caps/pass$pass-$res-$name")"
      cp "$frame" "$caps/pass$pass-${name}_$res.png"
    done
    if [[ -n "$base_bin" && "$deck" == "" ]]; then
      for still in "${befores[@]}"; do
        IFS=: read -r name first step count <<<"$still"
        frame="$(capture "$base_bin" "before-$name" "$first:$step:$count" "$res" "$scale" "$deck" "$caps/pass$pass-$res-before-$name")"
        cp "$frame" "$caps/pass$pass-before_${name}_$res.png"
      done
    fi
  done
  # the bare run: no room, no pilot; the dressing is on by default
  for attempt in 1 2 3; do
    d="$caps/pass$pass-bare"
    rm -rf "$d"; mkdir -p "$d"
    xvfb-run -a -s '-screen 0 1920x1080x24' "$bin" --no-audio --no-record --scale 1 --scene f01_late --f01-run \
      --capture-sequence 60:30:2 --capture-dir "$d" > "$d.log" 2>&1 || true
    grep -q '^FLOOR1_CAPTURE_SEQUENCE_DONE' "$d.log" && break
    [[ "$attempt" -lt 3 ]] || { echo "the bare run failed 3 times: see $d.log" >&2; exit 1; }
  done
  cp "$(find "$d" -name 'frame_*.png' | sort | tail -n 1)" "$caps/pass$pass-bare_run_640x360.png"
done
rm -f "$out"/scene_*.png "$out"/capture_lines.txt "$out"/SHA256SUMS
fail=0
for size in "${sizes[@]}"; do
  IFS=: read -r res scale deck <<<"$size"
  for still in "${stills[@]}"; do
    IFS=: read -r name first step count <<<"$still"
    tick=$((first + step * (count - 1)))
    a="$caps/pass1-${name}_$res.png" b="$caps/pass2-${name}_$res.png"
    same=no; cmp -s "$a" "$b" && same=yes
    window="$(state "$caps/pass1-$res-$name/run.log")"
    headless="$(state "$caps/pass1-$res-$name/headless.log")"
    match=no; [[ -n "$window" && "$window" == "$headless" ]] && match=yes
    [[ "$same" == yes && "$match" == yes ]] || fail=1
    cp "$a" "$out/scene_${name}_$res.png"
    echo "HEAT_STILL name=$name sequence=$first:$step:$count tick=$tick size=$res room_dressing=on window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
  done
  if [[ -n "$base_bin" && "$deck" == "" ]]; then
    for still in "${befores[@]}"; do
      IFS=: read -r name first step count <<<"$still"
      tick=$((first + step * (count - 1)))
      a="$caps/pass1-before_${name}_$res.png" b="$caps/pass2-before_${name}_$res.png"
      same=no; cmp -s "$a" "$b" && same=yes
      window="$(state "$caps/pass1-$res-before-$name/run.log")"
      headless="$(state "$caps/pass1-$res-before-$name/headless.log")"
      match=no; [[ -n "$window" && "$window" == "$headless" ]] && match=yes
      [[ "$same" == yes && "$match" == yes ]] || fail=1
      cp "$a" "$out/scene_before_${name}_$res.png"
      echo "HEAT_STILL name=before_$name sequence=$first:$step:$count tick=$tick size=$res room_dressing=off(origin/main) window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
    done
  fi
done
# the bare run
a="$caps/pass1-bare_run_640x360.png" b="$caps/pass2-bare_run_640x360.png"
same=no; cmp -s "$a" "$b" && same=yes
window="$(state "$caps/pass1-bare.log")"
"$bin" --headless-floor01-smoke --scene f01_late --f01-run --capture-tick 90 > "$caps/headless-bare.log" 2>&1
headless="$(state "$caps/headless-bare.log")"
match=no; [[ -n "$window" && "$window" == "$headless" ]] && match=yes
[[ "$same" == yes && "$match" == yes ]] || fail=1
cp "$a" "$out/scene_bare_run_640x360.png"
echo "HEAT_STILL name=bare_run sequence=60:30:2 tick=90 size=640x360 command=--scene_f01_late_--f01-run room_dressing=on window_hash=$window headless_hash=$headless hash_match=$match passes_byte_identical=$same sha256=$(sha256sum "$a" | cut -d' ' -f1)" >> "$out/capture_lines.txt"
python3 scripts/author_heat01.py gate "$out/gate" > /dev/null 2>&1
python3 scripts/make_heat01_sheets.py "$out" "${HEAT_LAYOUT_DIR:-}"
if [[ -n "${HEAT_LAYOUT_DIR:-}" ]]; then
  python3 scripts/measure_heat01.py "$HEAT_LAYOUT_DIR" "$out/measurements.txt" "$out" > /dev/null
fi
(cd "$out" && find . -type f \( -name '*.png' -o -name '*.txt' \) | sort | sed 's#^\./##' | xargs sha256sum > SHA256SUMS)
cat "$out/capture_lines.txt"
exit "$fail"
